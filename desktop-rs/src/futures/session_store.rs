//! A single application-wide writer and crash-consistent session intent.
use super::{
    automatic::Decision,
    flow::Intent,
    ledger::Settlement,
    margin::{self, Budget},
};
use anyhow::{Context, Result, ensure};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub budget: Budget,
    pub daily_loss: D,
    pub session_loss: D,
    pub minutes: u32,
    pub max_trades: u32,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        let b = &self.budget;
        for n in [
            b.margin,
            b.risk,
            b.max_notional,
            self.daily_loss,
            self.session_loss,
        ] {
            margin::number(n, false)?;
        }
        margin::number(b.funding_reserve, true)?;
        ensure!(
            b.risk <= self.session_loss
                && b.risk <= self.daily_loss
                && self.session_loss <= b.margin
                && self.daily_loss <= b.margin
                && b.funding_reserve < b.risk
                && (1..=125).contains(&b.leverage)
                && b.slippage_bps == D::ONE
                && b.liquidation_buffer_bps == D::from(10)
                && (1..=1440).contains(&self.minutes)
                && (1..=100).contains(&self.max_trades),
            "Batas sesi/risiko/leverage invalid"
        );
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Ready,
    Watching,
    Trading,
    Stopping,
    Recovering,
    Accounting,
    Finished,
    Blocked,
}
impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "BELUM AKTIF",
            Self::Watching => "LIVE · MENUNGGU SINYAL",
            Self::Trading => "LIVE · MENGELOLA POSISI",
            Self::Stopping => "MENGHENTIKAN & MENUTUP",
            Self::Recovering => "REKONSILIASI",
            Self::Accounting => "MEMERIKSA FILL & BIAYA",
            Self::Finished => "SESI SELESAI",
            Self::Blocked => "PERLU PEMULIHAN",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub intent: Intent,
    pub decision: Decision,
    pub started: u64,
    pub wallet_before: D,
    pub close: bool,
    pub flat_ms: Option<u64>,
    pub next_funding: u64,
    pub funding_due: Vec<u64>,
    pub settlement: Option<(Settlement, u64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_decision: Option<super::exit::Decision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_outcome: Option<super::exit::Outcome>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub strategy_version: String,
    pub config: Config,
    pub started: u64,
    pub deadline: u64,
    /// Greatest server-time lower bound actually proved by an accepted frame.
    /// `updated` remains monotonic display/archive metadata and is not clock
    /// rollback evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_floor_ms: Option<u64>,
    pub updated: u64,
    pub initial_wallet: D,
    pub net: D,
    pub fees: D,
    pub funding: D,
    pub completed: u32,
    pub last_bar: u64,
    pub sequence: u32,
    pub stop: bool,
    pub finished: bool,
    pub current: Option<Trade>,
    pub decision: Option<Decision>,
    pub reason: String,
    // Omit absent values so 0.6.1 state fingerprints remain valid on upgrade.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_code: Option<super::exit::Reason>,
    // Old sessions stay None; never fabricate pre-upgrade counters/build identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<super::diagnostics::Diagnostics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<super::recovery::State>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TradeArchive {
    pub(crate) schema: u32,
    pub(crate) session_id: String,
    pub(crate) strategy_version: String,
    pub(crate) config: Config,
    pub(crate) intent: Intent,
    pub(crate) decision: Decision,
    pub(crate) started: u64,
    pub(crate) result: Settlement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) exit_decision: Option<super::exit::Decision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) exit_outcome: Option<super::exit::Outcome>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub binding: String,
    pub session: Option<Session>,
    pub daily: BTreeMap<u64, D>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema: u32,
    state: State,
    hash: String,
}
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn fingerprint(state: &State) -> Result<String> {
    Ok(digest(&serde_json::to_vec(state)?))
}
/// Read-only startup display; does not claim ownership or activate recovery.
pub fn read_saved(root: &Path) -> Result<Option<State>> {
    let file = root.join("session.json");
    let m = match fs::symlink_metadata(&file) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    ensure!(
        m.is_file() && m.len() <= 2_097_152 && m.permissions().mode() & 0o077 == 0,
        "File sesi invalid/izin terbuka"
    );
    decode_saved(&fs::read(file)?).map(Some)
}
pub(crate) fn decode_saved(bytes: &[u8]) -> Result<State> {
    let e: Envelope =
        serde_json::from_slice(bytes).map_err(|_| anyhow::anyhow!("File sesi rusak"))?;
    ensure!(
        e.schema == 1 && e.hash == fingerprint(&e.state)?,
        "Integritas sesi gagal"
    );
    ensure!(
        e.state.session.as_ref().is_none_or(|s| s
            .recovery
            .as_ref()
            .is_none_or(|r| r.valid() && (!r.active || (!s.stop && !s.finished)))),
        "Metadata pemulihan sesi tidak valid"
    );
    Ok(e.state)
}
pub struct Store {
    pub root: PathBuf,
    lock: File,
    pub state: State,
    poisoned: bool,
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}
impl Store {
    pub fn open(root: &Path, binding: &str) -> Result<Self> {
        ensure!(
            binding.len() == 64 && binding.bytes().all(|c| c.is_ascii_hexdigit()),
            "Binding credential invalid"
        );
        fs::create_dir_all(root)?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(root.join("session.lock"))?;
        lock.try_lock()
            .context("Mesin live sudah dipakai jendela/proses lain")?;
        let mut state = read_saved(root)?.unwrap_or_default();
        if state.binding.is_empty() {
            state.binding = binding.into();
        }
        ensure!(
            state.binding == binding,
            "Sesi terikat API key lain; pemulihan akun diperlukan"
        );
        ensure!(state.daily.len() <= 366, "Riwayat hari melebihi batas");
        if let Some(s) = &state.session {
            s.config.validate()?;
            ensure!(
                valid_id(&s.id)
                    && s.sequence <= 200
                    && s.completed <= 100
                    && s.started <= s.updated
                    && s.clock_floor_ms.is_none_or(|floor| {
                        floor <= s.updated && (floor >= s.started || s.started - floor <= 5000)
                    })
                    && s.deadline > s.started
                    && s.recovery
                        .as_ref()
                        .is_none_or(|r| r.valid() && (!s.finished || !r.active)),
                "Sesi tersimpan invalid"
            );
            if let Some(t) = &s.current {
                ensure!(
                    valid_id(&t.intent.prefix)
                        && t.intent.prefix.starts_with(&s.id)
                        && t.funding_due.len() <= 200,
                    "Intent tersimpan invalid"
                );
            }
        }
        Ok(Self {
            root: root.into(),
            lock,
            state,
            poisoned: false,
        })
    }
    pub fn commit(&mut self) -> Result<()> {
        ensure!(!self.poisoned, "Sesi harus dipulihkan setelah kegagalan IO");
        let result = (|| -> Result<()> {
            let data = serde_json::to_vec(&Envelope {
                schema: 1,
                state: self.state.clone(),
                hash: fingerprint(&self.state)?,
            })?;
            ensure!(data.len() <= 2_097_152, "Sesi terlalu besar");
            let temp = self.root.join("session.tmp");
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp)?;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
            file.write_all(&data)?;
            file.sync_all()?;
            fs::rename(temp, self.root.join("session.json"))?;
            File::open(&self.root)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    pub fn begin(
        &mut self,
        config: Config,
        now: u64,
        clock_floor_ms: u64,
        wallet: D,
        last_bar: u64,
    ) -> Result<()> {
        config.validate()?;
        ensure!(
            clock_floor_ms > 0 && clock_floor_ms <= now && now - clock_floor_ms <= 5000,
            "Anchor clock aktivasi invalid"
        );
        ensure!(
            self.state
                .session
                .as_ref()
                .is_none_or(|s| s.finished && s.current.is_none()),
            "Sesi sebelumnya perlu dipulihkan dahulu"
        );
        let day = now / 86_400_000;
        let daily = *self.state.daily.get(&day).unwrap_or(&D::ZERO);
        ensure!(
            daily > -config.daily_loss,
            "Batas rugi harian sudah tercapai"
        );
        let id = format!("b{now:x}{:x}", std::process::id());
        ensure!(id.len() <= 19, "ID sesi terlalu panjang");
        let deadline = now
            .checked_add(u64::from(config.minutes) * 60_000)
            .context("Deadline overflow")?;
        self.state.session = Some(Session {
            id,
            strategy_version: super::automatic::VERSION.into(),
            config,
            started: now,
            deadline,
            clock_floor_ms: Some(clock_floor_ms),
            updated: now,
            initial_wallet: wallet,
            net: D::ZERO,
            fees: D::ZERO,
            funding: D::ZERO,
            completed: 0,
            last_bar,
            sequence: 0,
            stop: false,
            finished: false,
            current: None,
            decision: None,
            reason: "Warmup selesai; menunggu candle baru".into(),
            stop_reason: None,
            stop_code: None,
            diagnostics: Some(super::diagnostics::Diagnostics::default()),
            recovery: Some(super::recovery::State::default()),
        });
        self.state
            .daily
            .retain(|d, _| *d >= day.saturating_sub(365));
        self.commit()
    }
    pub fn settle(&mut self, result: Settlement, now: u64) -> Result<()> {
        let session = self.state.session.as_ref().context("Sesi hilang")?;
        let trade = session.current.as_ref().context("Trade ledger hilang")?;
        let archive = TradeArchive {
            schema: 1,
            session_id: session.id.clone(),
            strategy_version: session.strategy_version.clone(),
            config: session.config.clone(),
            intent: trade.intent.clone(),
            decision: trade.decision.clone(),
            started: trade.started,
            result: result.clone(),
            exit_decision: trade.exit_decision.clone(),
            exit_outcome: trade.exit_outcome,
        };
        let prefix = &self
            .state
            .session
            .as_ref()
            .and_then(|s| s.current.as_ref())
            .context("Trade ledger hilang")?
            .intent
            .prefix;
        let path = self.root.join(format!("{prefix}-ledger.json"));
        if path.exists() {
            let stored: TradeArchive = serde_json::from_slice(&fs::read(&path)?)
                .map_err(|_| anyhow::anyhow!("Arsip ledger rusak"))?;
            // A legacy archive can predate a crash/upgrade. Compare every old
            // financial/identity field; absent diagnostic metadata stays absent.
            let mut comparable = archive.clone();
            if stored.exit_decision.is_none() {
                comparable.exit_decision = None;
            }
            if stored.exit_outcome.is_none() {
                comparable.exit_outcome = None;
            }
            ensure!(
                stored == comparable,
                "Ledger trade sudah ada dengan hasil berbeda"
            );
        } else {
            let temp = path.with_extension("tmp");
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp)?;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
            file.write_all(&serde_json::to_vec(&archive)?)?;
            file.sync_all()?;
            fs::rename(temp, path)?;
            File::open(&self.root)?.sync_all()?;
        }
        let s = self.state.session.as_mut().context("Sesi hilang")?;
        ensure!(s.current.is_some(), "Tidak ada trade untuk pembukuan");
        s.net += result.net;
        s.fees += result.fees;
        s.funding += result.funding;
        s.completed += 1;
        s.updated = s.updated.max(now);
        *self
            .state
            .daily
            .entry(result.last_fill_ms / 86_400_000)
            .or_default() += result.net;
        s.current = None;
        s.reason = "Posisi flat; fill, biaya dan wallet sudah cocok".into();
        self.commit()
    }
    pub fn outbox_path(&self, id: &str) -> Result<PathBuf> {
        ensure!(valid_id(id), "ID outbox invalid");
        Ok(self.root.join(format!("{id}.json")))
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 30 && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn config() -> Config {
        Config {
            budget: Budget {
                margin: D::from(50),
                risk: D::from(2),
                max_notional: D::from(100),
                leverage: 3,
                slippage_bps: D::ONE,
                liquidation_buffer_bps: D::from(10),
                funding_reserve: D::new(1, 1),
            },
            daily_loss: D::from(10),
            session_loss: D::from(10),
            minutes: 60,
            max_trades: 10,
        }
    }
    #[test]
    fn global_lock_account_binding_and_unfinished_session_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let binding = digest(b"fixture only");
        let mut s = Store::open(dir.path(), &binding).unwrap();
        assert!(Store::open(dir.path(), &binding).is_err());
        s.begin(config(), 100000, 100000, D::from(100), 99999)
            .unwrap();
        drop(s);
        assert!(Store::open(dir.path(), &digest(b"different fixture")).is_err());
        let mut s = Store::open(dir.path(), &binding).unwrap();
        assert!(
            s.begin(config(), 100001, 100001, D::from(100), 99999)
                .is_err()
        );
        assert_eq!(
            fs::metadata(dir.path().join("session.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    #[test]
    fn r13_legacy_state_without_clock_floor_roundtrips_without_fabrication() {
        let dir = tempfile::tempdir().unwrap();
        let binding = digest(b"legacy clock fixture");
        let mut store = Store::open(dir.path(), &binding).unwrap();
        store
            .begin(config(), 100_000, 100_000, D::from(100), 99_999)
            .unwrap();
        let mut state_value = serde_json::to_value(&store.state).unwrap();
        state_value["session"]
            .as_object_mut()
            .unwrap()
            .remove("clock_floor_ms");
        let legacy: State = serde_json::from_value(state_value).unwrap();
        assert!(legacy.session.as_ref().unwrap().clock_floor_ms.is_none());
        let state_bytes = serde_json::to_vec(&legacy).unwrap();
        assert!(!String::from_utf8_lossy(&state_bytes).contains("clock_floor_ms"));
        let encoded = serde_json::to_vec(&serde_json::json!({
            "schema": 1,
            "state": legacy,
            "hash": digest(&state_bytes),
        }))
        .unwrap();
        let decoded = decode_saved(&encoded).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), state_bytes);
    }

    #[test]
    fn r14_activation_floor_may_precede_start_and_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let binding = digest(b"activation anchor fixture");
        let mut store = Store::open(dir.path(), &binding).unwrap();
        store
            .begin(config(), 100_300, 100_000, D::from(100), 99_999)
            .unwrap();
        let session = store.state.session.as_ref().unwrap();
        assert_eq!(session.started, 100_300);
        assert_eq!(session.clock_floor_ms, Some(100_000));
        assert_eq!(
            session.deadline - session.started,
            u64::from(session.config.minutes) * 60_000
        );
        drop(store);
        let reopened = Store::open(dir.path(), &binding).unwrap();
        assert_eq!(
            reopened.state.session.as_ref().unwrap().clock_floor_ms,
            Some(100_000)
        );
    }
}
