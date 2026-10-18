//! Durable dispatch records: reserve before send; never repeat a reserved mutation.
use super::{
    protocol::{Command, Receipt},
    transport::{Error, Exchange},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Uncertain,
    Acknowledged,
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub command: Command,
    pub status: Status,
    pub receipt: Option<Receipt>,
    pub error_code: Option<i64>,
    pub reconciled: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Store {
    schema: u32,
    records: Vec<Record>,
    hash: String,
}
pub struct Outbox {
    path: PathBuf,
    lock: File,
    records: Vec<Record>,
    poisoned: bool,
}
fn hash(records: &[Record]) -> Result<String> {
    Ok(Sha256::digest(serde_json::to_vec(records)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
impl Drop for Outbox {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}
impl Outbox {
    pub fn open(path: &Path) -> Result<Self> {
        let parent = path.parent().context("Folder outbox hilang")?;
        fs::create_dir_all(parent)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path.with_extension("lock"))?;
        lock.try_lock()
            .context("Outbox sedang dipakai proses lain")?;
        let records = if path.exists() {
            let metadata = fs::symlink_metadata(path)?;
            ensure!(
                metadata.is_file()
                    && metadata.len() <= 4_194_304
                    && metadata.permissions().mode() & 0o077 == 0,
                "File outbox tidak valid/terlalu besar/izin terbuka"
            );
            let store: Store = serde_json::from_slice(&fs::read(path)?).context("Outbox rusak")?;
            ensure!(
                store.schema == 1
                    && store.records.len() <= 1024
                    && store.hash == hash(&store.records)?,
                "Integritas outbox gagal"
            );
            let mut ids = std::collections::BTreeSet::new();
            for r in &store.records {
                ensure!(
                    ids.insert(r.id.clone()) && valid_id(&r.id),
                    "ID outbox duplikat/invalid"
                );
                r.command.spec()?;
                ensure!(
                    match r.status {
                        Status::Uncertain => r.receipt.is_none() && r.error_code.is_none(),
                        Status::Acknowledged => r.receipt.is_some() && r.error_code.is_none(),
                        Status::Rejected =>
                            r.receipt.is_none() && r.error_code.is_some_and(|c| c < 0),
                    },
                    "Status outbox tidak konsisten"
                );
            }
            store.records
        } else {
            vec![]
        };
        Ok(Self {
            path: path.to_owned(),
            lock,
            records,
            poisoned: false,
        })
    }
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    pub fn record(&self, id: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.id == id)
    }
    fn commit(&mut self, records: Vec<Record>) -> Result<()> {
        ensure!(!self.poisoned, "Outbox perlu dibuka ulang setelah IO gagal");
        let result = (|| -> Result<()> {
            let store = Store {
                schema: 1,
                hash: hash(&records)?,
                records,
            };
            let bytes = serde_json::to_vec(&store)?;
            ensure!(bytes.len() <= 4_194_304, "Batas outbox tercapai");
            let temporary = self.path.with_extension("tmp");
            // The separate writer lock covers replacement of the state file.
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &self.path)?;
            File::open(self.path.parent().context("Parent outbox hilang")?)?.sync_all()?;
            self.records = store.records;
            Ok(())
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    pub fn reserve(&mut self, id: &str, command: Command) -> Result<bool> {
        ensure!(valid_id(id), "ID dispatch invalid");
        command.spec()?;
        if let Some(existing) = self.record(id) {
            ensure!(
                existing.command == command,
                "ID dispatch sama dengan isi berbeda"
            );
            return Ok(false);
        }
        if let Some(client_id) = creation_id(&command) {
            ensure!(
                !self
                    .records
                    .iter()
                    .any(|r| creation_id(&r.command) == Some(client_id)),
                "Client ID order/algo sudah dipakai oleh dispatch lain"
            );
        }
        ensure!(
            self.records.len() < 1024,
            "Outbox penuh; arsip/reconcile diperlukan"
        );
        let mut records = self.records.clone();
        records.push(Record {
            id: id.into(),
            command,
            status: Status::Uncertain,
            receipt: None,
            error_code: None,
            reconciled: false,
        });
        self.commit(records)?;
        Ok(true)
    }
    fn finish(
        &mut self,
        id: &str,
        receipt: std::result::Result<Receipt, i64>,
        reconciled: bool,
    ) -> Result<Record> {
        if let Ok(next) = &receipt {
            // Query/cancel responses can arrive out of order. Compare all
            // records for the same exchange object, including cancel records.
            for previous in self.records.iter().filter_map(|r| r.receipt.as_ref()) {
                validate_progress(previous, next)?;
            }
        }
        let mut records = self.records.clone();
        let row = records
            .iter_mut()
            .find(|r| r.id == id)
            .context("Dispatch belum dicatat")?;
        match receipt {
            Ok(receipt) => {
                row.status = Status::Acknowledged;
                row.receipt = Some(receipt);
                row.error_code = None;
            }
            Err(code) => {
                ensure!(code < 0, "Kode reject invalid");
                row.status = Status::Rejected;
                row.receipt = None;
                row.error_code = Some(code);
            }
        }
        row.reconciled = reconciled;
        let result = row.clone();
        self.commit(records)?;
        Ok(result)
    }
}
fn validate_progress(previous: &Receipt, next: &Receipt) -> Result<()> {
    match (previous, next) {
        (Receipt::Order(old), Receipt::Order(new)) if old.client_id == new.client_id => {
            ensure!(
                old.exchange_id == new.exchange_id,
                "Identitas exchange order berubah"
            );
            ensure!(
                new.executed_qty >= old.executed_qty && new.quote_qty >= old.quote_qty,
                "Laporan fill mundur; rekonsiliasi diperlukan"
            );
            ensure!(
                !old.status.terminal() || old == new,
                "Laporan order terminal berubah; rekonsiliasi diperlukan"
            );
        }
        (Receipt::Algo(old), Receipt::Algo(new)) if old.client_id == new.client_id => {
            ensure!(
                old.exchange_id == new.exchange_id,
                "Identitas exchange algo berubah"
            );
            ensure!(
                old.actual_order_id.is_none() || old.actual_order_id == new.actual_order_id,
                "Identitas child order berubah/hilang"
            );
            ensure!(
                !old.terminal() || old.status == new.status,
                "Laporan algo terminal berubah; rekonsiliasi diperlukan"
            );
            let rank = |status: &str| match status {
                "NEW" => 0,
                "TRIGGERING" => 1,
                "TRIGGERED" => 2,
                _ => 3,
            };
            ensure!(
                rank(&new.status) >= rank(&old.status),
                "Laporan algo mundur"
            );
        }
        _ => {}
    }
    Ok(())
}
fn creation_id(command: &Command) -> Option<&str> {
    match command {
        Command::Market { client_id, .. } | Command::Protect { client_id, .. } => Some(client_id),
        _ => None,
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dispatch {
    Existing(Record),
    Recorded(Record),
    Uncertain(Error),
}
pub async fn dispatch<E: Exchange>(
    exchange: &mut E,
    outbox: &mut Outbox,
    id: &str,
    command: Command,
) -> Result<Dispatch> {
    ensure!(
        exchange.live_enabled(),
        "Transport sesi baca saja; mutation tidak dikirim"
    );
    if !outbox.reserve(id, command.clone())? {
        return Ok(Dispatch::Existing(
            outbox.record(id).context("Record hilang")?.clone(),
        ));
    }
    let started = std::time::Instant::now();
    let outcome = match exchange.write_once(&command).await {
        Ok(value) => match command.receipt(&value, false) {
            Ok(receipt) => Ok(Dispatch::Recorded(outbox.finish(id, Ok(receipt), false)?)),
            Err(_) => {
                eprintln!(
                    "LIVE_ORDER_DISPATCH; command={}; outcome=uncertain; stage=receipt_schema; elapsed_ms={}",
                    command.label(),
                    started.elapsed().as_millis()
                );
                // New Order's documented ACK/RESULT shape need not include
                // cumQuote. Obtain the full receipt with ONE GET immediately;
                // never invent quote/fill values and never resend the mutation.
                if matches!(command, Command::Market { .. }) && value.get("cumQuote").is_none() {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        reconcile(exchange, outbox, id),
                    )
                    .await;
                    return match result {
                        Ok(result) => result,
                        Err(_) => {
                            eprintln!(
                                "LIVE_ORDER_QUERY_ERROR; command={}; stage=receipt_fallback; category=Timeout",
                                command.label()
                            );
                            Ok(Dispatch::Uncertain(Error::Unknown))
                        }
                    };
                }
                return Ok(Dispatch::Uncertain(Error::Unknown));
            }
        },
        Err(Error::Rejected(code)) => {
            Ok(Dispatch::Recorded(outbox.finish(id, Err(code), false)?))
        }
        Err(error) => Ok(Dispatch::Uncertain(error)),
    };
    let label = match &outcome {
        Ok(Dispatch::Uncertain(error)) => format!("uncertain; category={error:?}"),
        Ok(Dispatch::Recorded(row)) if row.status == Status::Rejected => {
            format!("rejected; code={}", row.error_code.unwrap_or(0))
        }
        Ok(_) => "acknowledged".into(),
        Err(_) => "persistence_error".into(),
    };
    eprintln!(
        "LIVE_ORDER_DISPATCH; command={}; outcome={label}; elapsed_ms={}",
        command.label(),
        started.elapsed().as_millis()
    );
    outcome
}
/// Reconciliation issues GET only. NotFound never licenses another POST.
pub async fn reconcile<E: Exchange>(
    exchange: &mut E,
    outbox: &mut Outbox,
    id: &str,
) -> Result<Dispatch> {
    let command = outbox
        .record(id)
        .context("Dispatch belum dicatat")?
        .command
        .clone();
    let started = std::time::Instant::now();
    let outcome = match exchange.read(command.reconciliation()).await {
        Ok(value) => match command.receipt(&value, true) {
            Ok(receipt) => Ok(Dispatch::Recorded(outbox.finish(id, Ok(receipt), true)?)),
            Err(_) => Ok(Dispatch::Uncertain(Error::Schema)),
        },
        Err(error) => Ok(Dispatch::Uncertain(error)),
    };
    let category = match &outcome {
        Ok(Dispatch::Uncertain(error)) => format!("uncertain; category={error:?}"),
        Ok(_) => "reconciled".into(),
        Err(_) => "persistence_error".into(),
    };
    eprintln!(
        "LIVE_ORDER_QUERY; command={}; outcome={category}; elapsed_ms={}",
        command.label(),
        started.elapsed().as_millis()
    );
    outcome
}

/// Applies a bounded batch of GET results without retrying any mutation.
/// A partial failure preserves every last known receipt and requests close-only.
pub(crate) async fn reconcile_batch<E: Exchange>(
    exchange: &mut E,
    outbox: &mut Outbox,
    keys: &[String],
) -> Result<(bool, std::time::Instant)> {
    ensure!(
        keys.len() <= 16,
        "Terlalu banyak order aktif untuk batch rekonsiliasi"
    );
    if keys.is_empty() {
        return Ok((false, std::time::Instant::now()));
    }
    let commands = keys
        .iter()
        .map(|key| {
            outbox
                .record(key)
                .map(|r| r.command.clone())
                .context("Record rekonsiliasi hilang")
        })
        .collect::<Result<Vec<_>>>()?;
    let reads = commands
        .iter()
        .map(Command::reconciliation)
        .collect::<Vec<_>>();
    let batch = exchange.read_orders(&reads).await;
    let results = batch.results;
    ensure!(
        results.len() == keys.len(),
        "Jumlah respons rekonsiliasi tidak cocok"
    );
    let mut failed = false;
    for ((key, command), result) in keys.iter().zip(commands).zip(results) {
        match result {
            Ok(value) => match command.receipt(&value, true) {
                Ok(receipt) => {
                    outbox.finish(key, Ok(receipt), true)?;
                }
                Err(_) => {
                    failed = true;
                    eprintln!(
                        "LIVE_ORDER_QUERY_ERROR; stage=receipt_schema; command={}",
                        command.label()
                    );
                }
            },
            Err(error) => {
                failed = true;
                eprintln!(
                    "LIVE_ORDER_QUERY_ERROR; stage=transport; command={}; category={error:?}",
                    command.label()
                );
            }
        }
    }
    Ok((failed, batch.started))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futures::protocol::Algo;

    #[test]
    fn terminal_algo_cannot_regress_or_lose_its_executed_child() {
        let old = Algo {
            exchange_id: 42,
            client_id: "bd-test-sl".into(),
            status: "FINISHED".into(),
            actual_order_id: Some(99),
        };
        for case in [
            "status",
            "missing-child",
            "different-child",
            "different-algo",
        ] {
            let mut new = old.clone();
            match case {
                "status" => new.status = "NEW".into(),
                "missing-child" => new.actual_order_id = None,
                "different-child" => new.actual_order_id = Some(100),
                "different-algo" => new.exchange_id = 43,
                _ => unreachable!(),
            }
            assert!(validate_progress(&Receipt::Algo(old.clone()), &Receipt::Algo(new)).is_err());
        }
        assert!(validate_progress(&Receipt::Algo(old.clone()), &Receipt::Algo(old)).is_ok());
    }
}
