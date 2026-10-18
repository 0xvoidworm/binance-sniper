//! The same autonomous session coordinator is used by production and local exchange tests.
use super::{
    Ready, automatic,
    exit::{self, Reason as ExitReason},
    flow::{Intent, Next, Observation},
    ledger, margin,
    outbox::{self, Outbox, Status},
    planning,
    protocol::{Command, Read, Receipt, Symbol},
    session_store::{Config, Phase, Store, Trade},
    transport::Exchange,
};
use crate::paper::strategy::{Bar, INTERVAL};
use anyhow::{Context, Result, ensure};
use binance_engine::Side;
use rust_decimal::Decimal as D;
use std::{collections::BTreeSet, time::Instant};

#[derive(Clone)]
pub struct Frame {
    pub ready: Ready,
    pub bars: Vec<Bar>,
    /// Revalidated history is usable only as a baseline, never as a fresh entry.
    pub skip_signal: Option<String>,
    pub regular: Vec<(String, u64)>,
    pub algos: Vec<String>,
    pub max_notional: D,
    pub account_as_of: u64,
    pub received: Instant,
}
impl Frame {
    fn budget(&self, config: &Config, net: D, daily: D) -> margin::Budget {
        let mut budget = config.budget.clone();
        budget.margin = budget.margin.min(self.ready.context.available);
        budget.max_notional = budget.max_notional.min(self.max_notional);
        budget.risk = budget
            .risk
            .min(config.session_loss + net)
            .min(config.daily_loss + daily);
        budget
    }
    pub(crate) fn setup(&self, config: &Config, daily: D) -> Result<planning::Assessment> {
        config.validate()?;
        self.fresh()?;
        ensure!(
            self.compatible()
                && !self.foreign_position()
                && self.position().is_zero()
                && self.regular.is_empty()
                && self.algos.is_empty(),
            "Akun harus kompatibel, tanpa posisi/order awal"
        );
        ensure!(
            self.ready.context.available > D::ZERO && self.max_notional > D::ZERO,
            "Saldo atau batas notional akun belum terbaca"
        );
        let budget = self.budget(config, D::ZERO, daily);
        ensure!(
            budget.risk > budget.funding_reserve,
            "Sisa batas rugi harian/sesi tidak mencukupi cadangan dan risiko order"
        );
        let mut context = self.ready.context.clone();
        context.now = self.now();
        let report = planning::assess(
            &context,
            &budget,
            self.bars
                .last()
                .context("Candle awal belum tersedia")?
                .close,
        )?;
        self.fresh()?;
        Ok(report)
    }
    pub(crate) fn decision(&self) -> Result<automatic::Decision> {
        let mut context = self.ready.context.clone();
        context.now = self.now();
        let mut decision = automatic::evaluate(&self.bars, &context)?;
        if let Some(reason) = &self.skip_signal {
            decision.side = None;
            decision.votes.clear();
            decision.stop = None;
            decision.target = None;
            decision.reason = reason.clone();
        }
        Ok(decision)
    }
    pub fn now(&self) -> u64 {
        self.ready.clock.current_latest()
    }
    pub(crate) fn earliest_now(&self) -> u64 {
        self.ready.clock.current_earliest()
    }
    fn health_observation(&self) -> super::recovery::Observation {
        let observed = tokio::time::Instant::now();
        super::recovery::Observation::new(
            self.ready.clock.earliest(),
            self.ready.clock.latest(),
            self.ready.clock.local_anchor(),
            observed,
        )
    }
    pub fn position(&self) -> D {
        self.ready
            .account
            .positions
            .iter()
            .filter(|p| p.symbol == "BTCUSDT")
            .map(|p| p.position_amt)
            .sum()
    }
    pub fn foreign_position(&self) -> bool {
        self.ready
            .account
            .positions
            .iter()
            .any(|p| p.symbol != "BTCUSDT" && !p.position_amt.is_zero())
    }
    pub fn wallet(&self) -> D {
        self.ready.account.balances.total_wallet_balance
    }
    pub fn compatible(&self) -> bool {
        let a = &self.ready.account;
        a.config.can_trade
            && !a.config.dual_side_position
            && !a.config.multi_assets_margin
            && a.symbol("BTCUSDT").is_some_and(|s| !s.is_auto_add_margin)
    }
    pub(crate) fn fresh(&self) -> Result<()> {
        let now = self.now();
        let earliest = self.earliest_now();
        let quote_at = self.ready.context.quote.observed_at_ms;
        if self.account_as_of > now {
            return Err(super::transport::InvalidTimestamp::new(
                "account",
                self.account_as_of,
                earliest,
                now,
            )
            .into());
        }
        super::recovery::StaleRead::check("account", now - self.account_as_of, 5000)?;
        self.ready
            .clock
            .now()?
            .validate_age("quote", quote_at, 5000)?;
        if earliest / 86_400_000 != now / 86_400_000 {
            let boundary = now / 86_400_000 * 86_400_000;
            return Err(super::transport::ClockUncertain::new(
                "batas_harian_utc",
                boundary,
                earliest,
                now,
            )
            .into());
        }
        Ok(())
    }
}
/// Single-use proof that order receipts were reconciled BEFORE the new frame.
/// Not serialized or cloneable; restart always obtains new evidence.
pub(crate) struct Reconciled {
    pub(crate) cycle: u64,
    prefix: Option<String>,
    started: Instant,
    completed: Instant,
    reads: usize,
    failed: bool,
}
impl Reconciled {
    /// Only for the read-only observer with no execution engine. This proof
    /// cannot match any active intent and must never be used to skip its GETs.
    pub(crate) fn observation() -> Self {
        let now = Instant::now();
        Self {
            cycle: 0,
            prefix: None,
            started: now,
            completed: now,
            reads: 0,
            failed: false,
        }
    }
}
pub struct Engine {
    pub store: Store,
    pub phase: Phase,
    pub note: String,
    data_paused: bool,
    recovery_health: Option<super::recovery::Healthy>,
}
impl Engine {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            phase: Phase::Ready,
            note: "Belum aktif".into(),
            data_paused: false,
            recovery_health: None,
        }
    }
    pub fn begin(&mut self, config: Config, f: &Frame) -> Result<()> {
        let daily = *self
            .store
            .state
            .daily
            .get(&(f.now() / 86_400_000))
            .unwrap_or(&D::ZERO);
        let report = f.setup(&config, daily)?;
        ensure!(report.viable(), "{}", report.summary());
        let bar = f
            .bars
            .last()
            .context("Candle awal belum tersedia")?
            .close_ms;
        // Session duration starts at activation, while rollback provenance
        // starts at acquisition of the frame that activation will consume.
        // Mixing those anchors made the first tick look older than begin().
        let activation = f.ready.clock.now()?;
        self.store.begin(
            config,
            activation.earliest(),
            f.ready.clock.earliest(),
            f.wallet(),
            bar,
        )?;
        self.phase = Phase::Watching;
        eprintln!(
            "LIVE_SETUP_CHECK; strategy={}; result={}",
            automatic::VERSION,
            report.summary()
        );
        Ok(())
    }
    pub fn resume(&mut self) -> Result<()> {
        let s = self
            .store
            .state
            .session
            .as_mut()
            .context("Tidak ada sesi untuk dipulihkan")?;
        ensure!(!s.finished, "Sesi sudah selesai");
        if s.stop_reason.is_none() && s.stop {
            s.stop_reason = Some(s.reason.clone());
        }
        if !s.stop {
            s.stop_code = Some(ExitReason::Recovery);
            s.stop_reason
                .get_or_insert_with(|| "Pemulihan sesi lama tanpa entry baru".into());
        }
        s.stop = true;
        if let Some(r) = &mut s.recovery {
            r.active = false;
        }
        self.recovery_health = None;
        if let Some(t) = &mut s.current {
            t.close = true;
        }
        s.reason = "Pemulihan: rekonsiliasi dan tutup sesi lama tanpa entry baru".into();
        self.store.commit()?;
        self.phase = Phase::Recovering;
        Ok(())
    }
    pub fn stop(&mut self, reason: &str) -> Result<()> {
        self.stop_with(ExitReason::OtherStop, reason)
    }
    pub(crate) fn stop_with(&mut self, code: ExitReason, reason: &str) -> Result<()> {
        self.recovery_health = None;
        if let Some(s) = &mut self.store.state.session {
            if let Some(r) = &mut s.recovery {
                r.active = false;
            }
            if s.stop_reason.is_none() {
                s.stop_reason = Some(reason.into());
                s.stop_code = Some(code);
            }
            s.stop = true;
            s.reason = reason.into();
            if let Some(t) = &mut s.current {
                t.close = true;
            }
            self.store.commit()?;
        }
        self.phase = Phase::Stopping;
        self.note = reason.into();
        Ok(())
    }
    pub fn finished(&self) -> bool {
        self.store
            .state
            .session
            .as_ref()
            .is_some_and(|s| s.finished)
    }
    pub fn stopping(&self) -> bool {
        self.store.state.session.as_ref().is_some_and(|s| s.stop)
    }
    pub(crate) fn data_failed(&mut self, error: &str) -> Result<()> {
        self.record_gate(super::diagnostics::Gate::DataFailure);
        // No arbitrary reconnect deadline while the bot has no intent. Entry
        // stays disabled, the original session deadline still applies, and the
        // first recovered candle is consumed without trading. Any possible bot
        // exposure (including an uncertain intent) keeps the close-only path.
        if self
            .store
            .state
            .session
            .as_ref()
            .is_some_and(|s| !s.stop && !s.finished && s.current.is_none())
        {
            self.data_paused = true;
            self.describe(
                Phase::Recovering,
                format!("Data gagal; entry dijeda sampai data pulih atau durasi sesi habis. Tidak ada intent bot: {error}"),
            )
        } else {
            self.stop_with(
                ExitReason::DataFailure,
                &format!("Koneksi/data gagal; entry dihentikan: {error}"),
            )?;
            self.phase = Phase::Recovering;
            Ok(())
        }
    }
    pub(crate) fn read_failed(&mut self, error: &anyhow::Error) -> Result<()> {
        self.recovery_health = None;
        if error
            .downcast_ref::<super::session_input::HistoryRevalidation>()
            .is_some()
            && self
                .store
                .state
                .session
                .as_ref()
                .is_some_and(|s| s.current.is_none() && !s.stop && !s.finished)
        {
            // Preserve the existing two-read history gate while flat. It cannot
            // grant recovery authority to an exposed or terminal session.
            return self.data_failed(&error.to_string());
        }
        if !super::recovery::transient(error) {
            self.record_gate(super::diagnostics::Gate::DataFailure);
            return self.stop_with(
                ExitReason::DataFailure,
                &format!(
                    "Kegagalan data non-transien/tidak terklasifikasi; sesi dihentikan: {error}"
                ),
            );
        }
        let s = self.store.state.session.as_ref().context("Sesi hilang")?;
        if s.stop
            || s.finished
            || s.recovery.is_none()
            || (s.current.is_none() && !self.recovering_transport())
        {
            // Existing flat pause and legacy close-only recovery remain conservative.
            return self.data_failed(&error.to_string());
        }
        if !self.recovering_transport()
            && s.recovery.as_ref().unwrap().episodes >= super::recovery::MAX_EPISODES
        {
            self.record_gate(super::diagnostics::Gate::DataFailure);
            return self.stop_with(
                ExitReason::DataFailure,
                "Gangguan koneksi/data usang berulang; batas dua pemulihan posisi tercapai, sesi dihentikan",
            );
        }
        self.record_gate(super::diagnostics::Gate::DataFailure);
        let s = self.store.state.session.as_mut().unwrap();
        let r = s.recovery.as_mut().unwrap();
        if !r.active {
            r.episodes += 1;
            r.active = true;
        }
        if let Some(t) = &mut s.current {
            t.close = true;
        }
        s.decision = None;
        let episodes = r.episodes;
        self.describe(Phase::Recovering, format!("Gangguan koneksi/data usang sementara; amankan/tutup posisi dan cocokkan biaya, lalu perlu 60 detik data sehat. Episode {}/{}; entry ditahan", episodes, super::recovery::MAX_EPISODES))
    }
    pub(crate) fn recovering_transport(&self) -> bool {
        self.store.state.session.as_ref().is_some_and(|s| {
            !s.stop && !s.finished && s.recovery.as_ref().is_some_and(|r| r.active)
        })
    }
    pub(crate) fn close_only(&self) -> bool {
        self.stopping()
            || (self.recovering_transport()
                && self
                    .store
                    .state
                    .session
                    .as_ref()
                    .is_some_and(|s| s.current.is_some()))
    }
    pub(crate) fn check_failed(&mut self, error: &anyhow::Error) -> Result<()> {
        if super::recovery::transient(error) {
            // Typed pre-send/read network or stale-data failure; never retry an
            // unknown mutation outcome through the recovery policy.
            self.read_failed(error)
        } else {
            self.record_gate(super::diagnostics::Gate::SafetyFailure);
            self.stop_with(
                ExitReason::SafetyCheck,
                &format!("Pemeriksaan sesi gagal; entry dihentikan: {error}"),
            )
        }
    }
    fn recovered_flat(&mut self, f: &Frame) -> Result<()> {
        let s = self.store.state.session.as_ref().context("Sesi hilang")?;
        ensure!(
            s.current.is_none() && !s.stop && !s.finished && self.recovering_transport(),
            "Pemulihan belum flat atau sesi sudah berhenti"
        );
        ensure!(
            f.ready.isolated && f.ready.current_leverage == s.config.budget.leverage,
            "Konfigurasi margin/leverage berubah selama pemulihan"
        );
        if f.skip_signal.is_some() {
            self.recovery_health = None;
            return self.describe(
                Phase::Recovering,
                "Pemulihan: candle sedang direvalidasi; penghitung sehat diulang",
            );
        }
        let observation = f.health_observation();
        let ready = match &mut self.recovery_health {
            Some(health) => health.observe(observation),
            None => {
                self.recovery_health = Some(super::recovery::Healthy::new(observation));
                false
            }
        };
        let closed = (f.earliest_now() / INTERVAL * INTERVAL).saturating_sub(1);
        if !ready || f.bars.last().is_none_or(|b| b.close_ms != closed) {
            return self.describe(Phase::Recovering, format!("Posisi/order kosong dan accounting selesai; pemulihan menunggu data sehat kontinu {}/60 detik serta candle terkonfirmasi; belum entry", self.recovery_health.as_ref().unwrap().seconds()));
        }
        self.record_gate(super::diagnostics::Gate::RecoverySkip);
        let s = self.store.state.session.as_mut().unwrap();
        s.recovery.as_mut().unwrap().active = false;
        s.last_bar = s.last_bar.max(closed);
        s.decision = None;
        self.data_paused = false;
        self.recovery_health = None;
        self.describe(Phase::Watching, "Pemulihan transport selesai; posisi/order kosong, accounting cocok dan data sehat 60 detik. Candle pemulihan dilewati; menunggu candle baru")
    }
    pub(crate) fn check_empty_recovery(&self, f: &Frame) -> Result<()> {
        f.fresh()?;
        let s = self
            .store
            .state
            .session
            .as_ref()
            .context("Tidak ada sesi untuk dipulihkan")?;
        ensure!(
            !s.finished
                && (s.stop || f.now() >= s.deadline)
                && f.now() >= s.clock_floor_ms.unwrap_or(s.started),
            "Sesi belum dihentikan/kedaluwarsa atau waktu tidak valid"
        );
        ensure!(
            s.current.is_none()
                && s.sequence == 0
                && s.completed == 0
                && s.net.is_zero()
                && s.fees.is_zero()
                && s.funding.is_zero(),
            "Sesi memiliki riwayat/intent transaksi; perlu rekonsiliasi penuh"
        );
        ensure!(
            f.compatible()
                && !f.foreign_position()
                && f.position().is_zero()
                && f.regular.is_empty()
                && f.algos.is_empty(),
            "Akun tidak kompatibel atau masih memiliki posisi/order"
        );
        ensure!(
            (f.wallet() - s.initial_wallet).abs() <= D::new(1, 6),
            "Wallet berubah; pemulihan kosong tidak dapat menyelesaikan sesi"
        );
        Ok(())
    }
    pub(crate) fn finish_empty_verified(&mut self, f: &Frame) -> Result<()> {
        self.check_empty_recovery(f)?;
        let s = self.store.state.session.as_mut().unwrap();
        s.updated = s.updated.max(f.now());
        self.terminal("Sesi kosong selesai; dua pembacaan akun cocok, tanpa transaksi")
    }
    fn describe(&mut self, phase: Phase, note: impl Into<String>) -> Result<()> {
        let note = note.into();
        if self.phase != phase || self.note != note {
            eprintln!("LIVE_SESSION_EVENT; phase={phase:?}; reason={note}");
        }
        self.phase = phase;
        self.note = note;
        if let Some(s) = &mut self.store.state.session {
            s.reason = self.note.clone();
        }
        self.store.commit()
    }
    pub(crate) fn record_gate(&mut self, gate: super::diagnostics::Gate) {
        if let Some(d) = self
            .store
            .state
            .session
            .as_mut()
            .and_then(|s| s.diagnostics.as_mut())
        {
            d.record(gate);
        }
    }
    fn terminal(&mut self, note: &str) -> Result<()> {
        let s = self.store.state.session.as_mut().context("Sesi hilang")?;
        ensure!(s.current.is_none(), "Masih ada intent yang belum selesai");
        if let Some(r) = &mut s.recovery {
            r.active = false;
        }
        self.recovery_health = None;
        s.finished = true;
        s.stop = true;
        let note = s.stop_reason.as_ref().map_or_else(
            || note.to_owned(),
            |cause| format!("{note}. Penyebab penghentian: {cause}"),
        );
        self.describe(Phase::Finished, note)
    }
    pub(crate) async fn reconcile_orders<E: Exchange>(
        &self,
        exchange: &mut E,
    ) -> Result<Reconciled> {
        let mut started = Instant::now();
        let prefix = self
            .store
            .state
            .session
            .as_ref()
            .and_then(|s| s.current.as_ref())
            .map(|t| t.intent.prefix.clone());
        let mut reads = 0;
        let mut failed = false;
        if let Some(prefix) = &prefix {
            let mut box_ = Outbox::open(&self.store.outbox_path(prefix)?)?;
            let keys: Vec<_> = box_
                .records()
                .iter()
                .filter(|r| match &r.receipt {
                    Some(Receipt::Order(o)) => !o.status.terminal(),
                    Some(Receipt::Algo(a)) => !a.terminal(),
                    Some(Receipt::CancelAccepted) => true,
                    Some(Receipt::Configured) => false,
                    None => r.status == Status::Uncertain,
                })
                .map(|r| r.id.clone())
                .collect();
            reads = keys.len();
            (failed, started) = outbox::reconcile_batch(exchange, &mut box_, &keys).await?;
        }
        Ok(Reconciled {
            cycle: 0,
            prefix,
            started,
            completed: Instant::now(),
            reads,
            failed,
        })
    }
    // Fixture convenience. Production uses reconcile -> fetch -> tick_prepared.
    #[cfg(test)]
    pub async fn tick<E: Exchange>(&mut self, exchange: &mut E, f: &Frame) -> Result<()> {
        let reconciled = self.reconcile_orders(exchange).await?;
        self.tick_prepared(exchange, f, reconciled).await
    }
    pub(crate) async fn tick_prepared<E: Exchange>(
        &mut self,
        exchange: &mut E,
        f: &Frame,
        reconciled: Reconciled,
    ) -> Result<()> {
        ensure!(
            exchange.live_enabled(),
            "Supervisor eksekusi memerlukan aktivasi sesi oleh pengguna"
        );
        f.fresh()?;
        let prefix = self
            .store
            .state
            .session
            .as_ref()
            .and_then(|s| s.current.as_ref())
            .map(|t| t.intent.prefix.as_str());
        ensure!(
            prefix == reconciled.prefix.as_deref(),
            "Rekonsiliasi bukan untuk intent saat ini"
        );
        // Both order evidence and the fresh account/quote remain bounded. Do not
        // manufacture a fresh timestamp for an old response or reuse old proof.
        if reconciled.reads > 0 {
            let age = reconciled.started.elapsed().as_millis();
            ensure!(
                age <= 5000,
                "Bukti rekonsiliasi usang; order_batch_age_ms={age}"
            );
            eprintln!(
                "LIVE_FRAME_HEALTH; cycle={}; order_reads={}; reconcile_ms={}; order_batch_age_ms={}; account_age_ms={}; quote_age_ms={}",
                reconciled.cycle,
                reconciled.reads,
                reconciled
                    .completed
                    .duration_since(reconciled.started)
                    .as_millis(),
                age,
                i128::from(f.now()) - i128::from(f.account_as_of),
                i128::from(f.now()) - i128::from(f.ready.context.quote.observed_at_ms)
            );
        }
        if reconciled.failed {
            self.stop_with(
                ExitReason::ProtectionFailure,
                "Status proteksi/order belum pasti; tutup eksposur yang sudah teridentifikasi",
            )?;
        }
        let now = f.now();
        let initial = self
            .store
            .state
            .session
            .clone()
            .context("Sesi belum dibuat")?;
        if initial.finished {
            return Ok(());
        }
        self.accept_clock_progress(f)?;
        let daily = *self
            .store
            .state
            .daily
            .get(&(now / 86_400_000))
            .unwrap_or(&D::ZERO);
        let floating = f
            .ready
            .account
            .positions
            .iter()
            .filter(|p| p.symbol == "BTCUSDT")
            .map(|p| p.unrealized_profit)
            .sum::<D>();
        let estimate_cost = initial.current.as_ref().map_or(D::ZERO, |t| {
            t.intent.plan.fee_reserve + t.intent.plan.funding_reserve
        });
        let account_delta = f.wallet() - initial.initial_wallet + floating;
        let risk_pnl = (initial.net + floating - estimate_cost).min(account_delta);
        let mut limit_causes = vec![];
        for (hit, reason) in [
            (
                risk_pnl <= -initial.config.session_loss,
                ExitReason::SessionLoss,
            ),
            (
                daily + (risk_pnl - initial.net) <= -initial.config.daily_loss,
                ExitReason::DailyLoss,
            ),
            (
                f.foreign_position() || !f.compatible(),
                ExitReason::AccountChanged,
            ),
            (now >= initial.deadline, ExitReason::SessionDeadline),
            (
                initial.completed >= initial.config.max_trades,
                ExitReason::TradeLimit,
            ),
            (initial.sequence >= 200, ExitReason::SequenceLimit),
        ] {
            if hit {
                limit_causes.push(reason);
            }
        }
        if let Some(reason) = limit_causes.first().copied() {
            self.stop_with(
                reason,
                "Batas sesi/risiko atau konfigurasi akun berubah; tidak ada entry baru",
            )?;
        }
        if initial.current.is_none() {
            if !f.position().is_zero() || !f.regular.is_empty() || !f.algos.is_empty() {
                if self.recovering_transport() {
                    self.stop_with(
                        ExitReason::AccountChanged,
                        "Posisi/order asing saat pemulihan; sesi tidak boleh melanjutkan entry",
                    )?;
                }
                return self.describe(
                    Phase::Blocked,
                    "Posisi/order asing ditemukan; tidak disentuh oleh sesi",
                );
            }
            if self.store.state.session.as_ref().unwrap().stop {
                return self.terminal("Sesi berhenti, posisi/order bot kosong");
            }
            ensure!(
                (f.wallet() - initial.initial_wallet - initial.net).abs() <= D::new(1, 6),
                "Wallet berubah di luar ledger sesi; entry ditahan"
            );
            if self.recovering_transport() {
                return self.recovered_flat(f);
            }
            if self.data_paused || f.skip_signal.is_some() {
                if self.data_paused || f.bars.last().is_some_and(|b| b.close_ms > initial.last_bar)
                {
                    self.record_gate(super::diagnostics::Gate::RecoverySkip);
                }
                // Recovery may precede finalization of the latest closed bar.
                // Consume its interval too: confirming it later is not a new signal.
                let recovered_through = if self.data_paused {
                    (f.earliest_now() / INTERVAL * INTERVAL).saturating_sub(1)
                } else {
                    0
                };
                self.data_paused = false;
                let s = self.store.state.session.as_mut().unwrap();
                s.last_bar = s
                    .last_bar
                    .max(f.bars.last().context("Candle revalidasi hilang")?.close_ms)
                    .max(recovered_through);
                s.decision = None;
                return self.describe(
                    Phase::Watching,
                    f.skip_signal.as_deref().unwrap_or(
                        "Data pulih; candle pemulihan dilewati tanpa entry, menunggu candle baru",
                    ),
                );
            }
            let mut setup =
                Outbox::open(&self.store.outbox_path(&format!("{}-setup", initial.id))?)?;
            for (key, command, matched) in [
                (
                    "margin",
                    Command::Isolated {
                        symbol: Symbol::BTCUSDT,
                    },
                    f.ready.isolated,
                ),
                (
                    "leverage",
                    Command::Leverage {
                        symbol: Symbol::BTCUSDT,
                        leverage: initial.config.budget.leverage,
                    },
                    f.ready.current_leverage == initial.config.budget.leverage,
                ),
            ] {
                if matched {
                    continue;
                }
                match setup.record(key) {
                    None => {
                        outbox::dispatch(exchange, &mut setup, key, command).await?;
                    }
                    Some(row) if row.status == Status::Rejected => {
                        self.stop_with(
                            ExitReason::AccountChanged,
                            "Perubahan margin/leverage ditolak Binance",
                        )?;
                        return self.terminal("Konfigurasi ditolak; tanpa entry");
                    }
                    Some(_) => {
                        outbox::reconcile(exchange, &mut setup, key).await?;
                    }
                }
                return self.describe(
                    Phase::Watching,
                    "Menunggu konfirmasi isolated/leverage pada akun",
                );
            }
            let current_bar = f.bars.last().context("Candle hilang")?.close_ms;
            if current_bar <= initial.last_bar {
                return self.describe(
                    Phase::Watching,
                    "Menunggu candle baru; tidak mengulang sinyal lama",
                );
            }
            ensure!(
                current_bar - initial.last_bar == INTERVAL,
                "Candle sesi terlewati; entry dihentikan"
            );
            self.store.state.session.as_mut().unwrap().last_bar = current_bar;
            if current_bar < initial.started {
                return self.describe(
                    Phase::Watching,
                    "Candle sebelum aktivasi dilewati; menunggu candle baru",
                );
            }
            let mut context = f.ready.context.clone();
            context.now = f.now();
            let (mut decision, gate) = automatic::evaluate_classified(&f.bars, &context)?;
            self.record_gate(gate);
            eprintln!(
                "LIVE_CANDLE_DECISION; bar_ms={}; side={:?}; reason={}",
                decision.bar_ms, decision.side, decision.reason
            );
            self.store.state.session.as_mut().unwrap().decision = Some(decision.clone());
            self.store.commit()?; // Consumes the bar before any potential dispatch.
            let Some(side) = decision.side else {
                return self.describe(Phase::Watching, decision.reason);
            };
            if f.ready.next_funding.saturating_sub(now) <= 600_000 {
                self.record_gate(super::diagnostics::Gate::Funding);
                return self.describe(Phase::Watching, "Entry ditahan 10 menit sebelum funding");
            }
            let budget = f.budget(&initial.config, initial.net, daily);
            let review = planning::review(&context, &budget, side, decision.atr);
            review.log();
            let plan = match review.into_plan() {
                Ok(p) => p,
                Err(error) => {
                    self.record_gate(super::diagnostics::Gate::Economics);
                    return self.describe(Phase::Watching, format!("Sinyal dilewati: {error}"));
                }
            };
            decision.stop = Some(plan.stop);
            decision.target = Some(plan.take_profit);
            eprintln!(
                "LIVE_ENTRY_PLAN; strategy={}; side={side:?}; qty={}; stop={}; target={}; budgeted_loss={}; reward={}; net_rr={}",
                automatic::VERSION,
                plan.qty,
                plan.stop,
                plan.take_profit,
                plan.budgeted_loss,
                planning::net_reward(&plan, budget.slippage_bps),
                (planning::net_reward(&plan, budget.slippage_bps) / plan.budgeted_loss).round_dp(4)
            );
            self.record_gate(super::diagnostics::Gate::Intent);
            let s = self.store.state.session.as_mut().unwrap();
            s.decision = Some(decision.clone());
            s.sequence += 1;
            s.current = Some(Trade {
                decision,
                intent: Intent {
                    prefix: format!("{}-{}", s.id, s.sequence),
                    symbol: Symbol::BTCUSDT,
                    plan,
                    // Entry expiry starts at the proven lower bound, so a wide
                    // range can only shorten the opportunity, never make an
                    // old signal eligible for longer.
                    entry_deadline_ms: f.ready.clock.earliest() + 15_000,
                },
                // Include a fill that can occur anywhere after acquisition.
                // A wide latest estimate is not proof that such a fill is old.
                started: f.ready.clock.earliest(),
                wallet_before: f.wallet(),
                close: false,
                flat_ms: None,
                next_funding: f.ready.next_funding,
                funding_due: vec![],
                settlement: None,
                exit_decision: None,
                exit_outcome: None,
            });
            self.store.commit()?;
        }
        if f.skip_signal.is_some() {
            self.stop_with(
                ExitReason::DataFailure,
                "Revisi candle saat ada intent; tutup dan rekonsiliasi tanpa entry baru",
            )?;
        }
        self.drive(exchange, f, reconciled.failed, limit_causes)
            .await
    }
    async fn drive<E: Exchange>(
        &mut self,
        exchange: &mut E,
        f: &Frame,
        refresh_failed: bool,
        mut exit_causes: Vec<ExitReason>,
    ) -> Result<()> {
        let current = self
            .store
            .state
            .session
            .as_ref()
            .and_then(|s| s.current.clone())
            .context("Intent hilang")?;
        let mut box_ = Outbox::open(&self.store.outbox_path(&current.intent.prefix)?)?;
        let now = f.now();
        let mut closing = self.store.state.session.as_ref().unwrap().stop || current.close;
        let session = self.store.state.session.as_ref().unwrap();
        let legacy_exit = current.close
            && current.exit_decision.is_none()
            && !self.recovering_transport()
            && (!session.stop || session.stop_code.is_none());
        if session.stop {
            exit_causes.push(session.stop_code.unwrap_or(ExitReason::LegacyUnknown));
        }
        if self.recovering_transport() {
            exit_causes.push(ExitReason::DataFailure);
        }
        if current.close
            && current.exit_decision.is_none()
            && !session.stop
            && !self.recovering_transport()
        {
            exit_causes.push(ExitReason::LegacyUnknown);
        }
        // An order can trigger/disappear after its individual GET and before
        // the new account/open-orders snapshot. Never call that state Watch.
        let missing_protection = !f.position().is_zero()
            && box_.records().iter().any(|row| {
                matches!((&row.command, &row.receipt),
                (Command::Protect {client_id,..}, Some(Receipt::Algo(a)))
                if a.status == "NEW" && !f.algos.contains(client_id))
            });
        if missing_protection {
            self.stop_with(
                ExitReason::ProtectionFailure,
                "Proteksi tidak terlihat pada snapshot order terbaru; tutup dan rekonsiliasi",
            )?;
            exit_causes.push(ExitReason::ProtectionFailure);
            closing = true;
        }
        if now.saturating_sub(current.started) >= 30 * 60_000 {
            closing = true;
            exit_causes.push(ExitReason::HoldTimeout);
        }
        if f.ready.next_funding.saturating_sub(now) <= 120_000 {
            closing = true;
            exit_causes.push(ExitReason::FundingWindow);
        }
        for p in f
            .ready
            .account
            .positions
            .iter()
            .filter(|p| p.symbol == "BTCUSDT" && !p.position_amt.is_zero())
        {
            let liquidation = match current.intent.plan.side {
                Side::Long => p.liquidation_price >= current.intent.plan.stop,
                Side::Short => {
                    p.liquidation_price > D::ZERO && p.liquidation_price <= current.intent.plan.stop
                }
            };
            if liquidation {
                closing = true;
                exit_causes.push(ExitReason::LiquidationBuffer);
            }
        }
        let new_bar = f.bars.last().map_or(0, |b| b.close_ms);
        let s = self.store.state.session.as_mut().unwrap();
        if f.skip_signal.is_none() && new_bar > s.last_bar {
            s.last_bar = new_bar;
            if let Ok(d) = automatic::evaluate(&f.bars, &f.ready.context) {
                if d.side.is_some_and(|side| side != current.intent.plan.side) {
                    closing = true;
                    exit_causes.push(ExitReason::OppositeSignal);
                }
                s.decision = Some(d);
            }
        }
        let t = s.current.as_mut().unwrap();
        if now >= t.next_funding && t.next_funding < f.ready.next_funding {
            t.funding_due.push(t.next_funding);
            t.next_funding = f.ready.next_funding;
        }
        t.close |= closing;
        if !f.position().is_zero() {
            if legacy_exit {
                exit::record(&mut t.exit_decision, vec![ExitReason::LegacyUnknown], now);
            }
            exit::record(&mut t.exit_decision, exit_causes, now);
        }
        self.store.commit()?;
        let (regular, algos, children) = owned(&box_);
        let unowned = f
            .regular
            .iter()
            .any(|(client, id)| !regular.contains(client) && !children.contains(id))
            || f.algos.iter().any(|id| !algos.contains(id));
        let obs = Observation {
            now_ms: f.now(),
            as_of_ms: f.account_as_of,
            quote: f.ready.context.quote.clone(),
            mark: f.ready.context.mark,
            position_qty: f.position(),
            isolated: f.ready.isolated,
            leverage: f.ready.current_leverage,
            compatible_account: f.compatible(),
            unowned_orders: unowned,
        };
        let next = current.intent.next(&box_, &obs, closing)?;
        match next {
            Next::Send { key, command } => {
                if matches!(
                    command,
                    Command::Market {
                        reduce_only: true,
                        ..
                    }
                ) {
                    let t = self
                        .store
                        .state
                        .session
                        .as_mut()
                        .unwrap()
                        .current
                        .as_mut()
                        .unwrap();
                    if t.exit_decision.is_none() {
                        exit::record(&mut t.exit_decision, vec![ExitReason::LifecycleClose], now);
                        self.store.commit()?;
                    }
                }
                if matches!(
                    command,
                    Command::Market {
                        reduce_only: false,
                        ..
                    }
                ) {
                    ensure!(
                        current.intent.plan.notional <= f.max_notional,
                        "Notional melebihi limit leverage akun"
                    );
                    ensure!(
                        current.intent.plan.required_balance <= f.ready.context.available,
                        "Saldo berubah sebelum entry"
                    );
                    let fee = exchange.read(Read::FeeBurn).await?;
                    ensure!(
                        fee["feeBurn"].as_bool() == Some(false),
                        "Fee BNB belum didukung; tidak mengubah konfigurasi akun"
                    );
                    f.fresh()?;
                }
                let is_entry = matches!(
                    command,
                    Command::Market {
                        reduce_only: false,
                        ..
                    }
                );
                let result = outbox::dispatch(exchange, &mut box_, &key, command).await?;
                // Protect an acknowledged partial/full fill immediately. The next
                // account poll is still required before TP/cleanup or another entry.
                if is_entry
                    && let Some(row) = box_.record(&key)
                    && matches!(&row.receipt,Some(Receipt::Order(o)) if o.executed_qty>D::ZERO)
                    && let Next::Send {
                        key: sl_key,
                        command: sl,
                    } = current.intent.next(&box_, &obs, false)?
                    && matches!(
                        sl,
                        Command::Protect {
                            kind: super::protocol::ProtectionKind::Stop,
                            ..
                        }
                    )
                {
                    outbox::dispatch(exchange, &mut box_, &sl_key, sl).await?;
                }
                self.describe(
                    if closing {
                        Phase::Stopping
                    } else {
                        Phase::Trading
                    },
                    match result {
                        outbox::Dispatch::Uncertain(_) => {
                            "Hasil pengiriman belum pasti; rekonsiliasi tanpa kirim ulang"
                        }
                        _ => "Mengelola entry, proteksi dan penutupan otomatis",
                    },
                )
            }
            Next::Reconcile { key } => {
                outbox::reconcile(exchange, &mut box_, &key).await?;
                self.describe(
                    Phase::Recovering,
                    "Memastikan status order/proteksi di Binance",
                )
            }
            Next::Watch { .. } => self.describe(
                Phase::Trading,
                "Posisi dipantau; SL/TP exchange terkonfirmasi pada pembacaan terakhir",
            ),
            Next::Observe(note) => self.describe(
                if closing {
                    Phase::Stopping
                } else {
                    Phase::Recovering
                },
                note,
            ),
            Next::Halt(note) => {
                let entry = box_.record(&format!("{}-entry", current.intent.prefix));
                let no_exposure=entry.is_none_or(|r|r.status==Status::Rejected||matches!(&r.receipt,Some(Receipt::Order(o)) if o.status.terminal()&&o.executed_qty.is_zero()));
                if f.position().is_zero()
                    && f.regular.is_empty()
                    && f.algos.is_empty()
                    && no_exposure
                {
                    self.store.state.session.as_mut().unwrap().current = None;
                    if self.stopping() {
                        return self.terminal("Sesi berhenti; intent tanpa exposure selesai");
                    }
                    return self.describe(
                        if self.recovering_transport() {
                            Phase::Recovering
                        } else {
                            Phase::Watching
                        },
                        note,
                    );
                }
                self.describe(Phase::Blocked, note)
            }
            Next::NeedsAccounting {
                entry_order_id,
                exit_order_id,
                mut child_order_ids,
            } => {
                ensure!(!refresh_failed, "Cleanup order belum dapat dikonfirmasi");
                if let Some(id) = exit_order_id {
                    child_order_ids.push(id);
                }
                child_order_ids.sort_unstable();
                child_order_ids.dedup();
                ensure!(
                    f.position().is_zero() && f.regular.is_empty() && f.algos.is_empty(),
                    "Cleanup exchange belum kosong"
                );
                let t = self
                    .store
                    .state
                    .session
                    .as_mut()
                    .unwrap()
                    .current
                    .as_mut()
                    .unwrap();
                // Store the upper bound at the first flat observation. A later
                // lower bound must pass it by five seconds before accounting;
                // two shrinking/expanding latest estimates cannot fake a wait.
                let flat = *t.flat_ms.get_or_insert(now);
                self.store.commit()?;
                if f.earliest_now().saturating_sub(flat) < 5000 {
                    return self.describe(Phase::Accounting, "Menunggu fill/biaya akhir tersedia");
                }
                let detailed = ledger::settle_detailed(
                    exchange,
                    ledger::Query {
                        symbol: Symbol::BTCUSDT,
                        side: current.intent.plan.side,
                        entry: entry_order_id,
                        exits: &child_order_ids,
                        start: current.started,
                        end: flat,
                        funding_due: &current.funding_due,
                    },
                )
                .await?;
                let result = detailed.result;
                let outcome = exit::observed(&box_, &detailed.filled_exit_orders);
                ensure!(
                    (f.wallet() - current.wallet_before - result.net).abs() <= D::new(1, 6),
                    "Fill/fee/funding belum cocok dengan perubahan wallet; entry ditahan"
                );
                let t = self
                    .store
                    .state
                    .session
                    .as_mut()
                    .unwrap()
                    .current
                    .as_mut()
                    .unwrap();
                t.exit_outcome = Some(outcome);
                if t.settlement.as_ref().is_some_and(|(old, at)| {
                    old == &result && f.earliest_now().saturating_sub(*at) >= 5000
                }) {
                    self.store.settle(result, now)?;
                    if self.stopping() {
                        return self.terminal(
                            "Sesi selesai; posisi/order kosong dan ledger cocok dua kali",
                        );
                    }
                    if self.recovering_transport() {
                        self.recovery_health = None;
                        return self.describe(Phase::Recovering, "Posisi ditutup dan ledger cocok dua kali; entry tetap ditahan untuk 60 detik data sehat");
                    }
                    self.describe(
                        Phase::Watching,
                        "Trade selesai; ledger dan wallet cocok dua kali, menunggu sinyal baru",
                    )
                } else {
                    if t.settlement.as_ref().is_none_or(|(old, _)| old != &result) {
                        t.settlement = Some((result, now));
                    }
                    self.describe(
                        Phase::Accounting,
                        "Memastikan fill/biaya stabil sebelum entry berikutnya",
                    )
                }
            }
        }
    }

    fn accept_clock_progress(&mut self, f: &Frame) -> Result<()> {
        // Persist evidence from acquisition, not a later UI/supervisor read.
        // Freshness still projects the same range to the decision instant.
        let current = f.ready.clock;
        let durable_floor = self
            .store
            .state
            .session
            .as_ref()
            .and_then(|session| session.clock_floor_ms);
        if let Some(floor) = durable_floor
            && current.latest() < floor
        {
            return Err(super::transport::ClockRollback::new(floor, current.latest()).into());
        }
        let session = self
            .store
            .state
            .session
            .as_mut()
            .context("Sesi belum dibuat")?;
        session.clock_floor_ms = Some(
            session
                .clock_floor_ms
                .unwrap_or(session.started)
                .max(current.earliest()),
        );
        // Monotonic audit/display high-water only. Safety decisions above use
        // interval evidence and the durable lower bound, never this metadata.
        session.updated = session.updated.max(current.latest());
        Ok(())
    }
}
fn owned(box_: &Outbox) -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<u64>) {
    let mut regular = BTreeSet::new();
    let mut algos = BTreeSet::new();
    let mut children = BTreeSet::new();
    for r in box_.records() {
        match &r.command {
            Command::Market { client_id, .. } => {
                regular.insert(client_id.clone());
            }
            Command::Protect { client_id, .. } => {
                algos.insert(client_id.clone());
            }
            _ => {}
        }
        if let Some(Receipt::Algo(a)) = &r.receipt
            && let Some(id) = a.actual_order_id
        {
            children.insert(id);
        }
    }
    (regular, algos, children)
}
