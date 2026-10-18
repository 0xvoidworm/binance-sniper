//! Automatic bracket lifecycle decisions. A runtime must keep observations and
//! receipts fresh, dispatch through the durable outbox, and settle fills/fees
//! before proposing another trade. This module never sends HTTP by itself.
use super::{
    margin::Plan,
    outbox::{Outbox, Record, Status},
    protocol::{Algo, Command, Order, ProtectionKind, Receipt, Symbol, opposite},
};
use anyhow::{Context, Result, ensure};
use binance_engine::{Quote, Side};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub prefix: String,
    pub symbol: Symbol,
    pub plan: Plan,
    pub entry_deadline_ms: u64,
}
pub struct Observation {
    pub now_ms: u64,
    pub as_of_ms: u64,
    pub quote: Quote,
    pub mark: D,
    pub position_qty: D,
    pub isolated: bool,
    pub leverage: u32,
    pub compatible_account: bool,
    pub unowned_orders: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    Send {
        key: String,
        command: Command,
    },
    Reconcile {
        key: String,
    },
    Observe(&'static str),
    Watch {
        keys: Vec<String>,
    },
    NeedsAccounting {
        entry_order_id: u64,
        exit_order_id: Option<u64>,
        child_order_ids: Vec<u64>,
    },
    Halt(&'static str),
}
impl Intent {
    fn key(&self, suffix: &str) -> String {
        format!("{}-{suffix}", self.prefix)
    }
    fn entry(&self) -> Command {
        Command::Market {
            symbol: self.symbol,
            client_id: self.key("entry"),
            side: self.plan.side,
            qty: self.plan.qty,
            reduce_only: false,
        }
    }
    fn protection(&self, kind: ProtectionKind) -> Command {
        Command::Protect {
            symbol: self.symbol,
            client_id: self.key(if kind == ProtectionKind::Stop {
                "sl"
            } else {
                "tp"
            }),
            position_side: self.plan.side,
            kind,
            trigger: if kind == ProtectionKind::Stop {
                self.plan.stop
            } else {
                self.plan.take_profit
            },
        }
    }
    fn query(&self, suffix: &str) -> Next {
        Next::Reconcile {
            key: self.key(suffix),
        }
    }
    fn send(&self, suffix: &str, command: Command) -> Next {
        Next::Send {
            key: self.key(suffix),
            command,
        }
    }
    fn row<'a>(&self, box_: &'a Outbox, suffix: &str) -> Option<&'a Record> {
        box_.record(&self.key(suffix))
    }

    /// One session activation can call this repeatedly; there is no human
    /// confirmation per order. Halt/NeedsAccounting do NOT permit another entry.
    pub fn next(&self, box_: &Outbox, obs: &Observation, stop: bool) -> Result<Next> {
        ensure!(
            !self.prefix.is_empty()
                && self.prefix.len() <= 24
                && self
                    .prefix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "Prefix intent invalid"
        );
        self.entry().spec()?;
        self.protection(ProtectionKind::Stop).spec()?;
        self.protection(ProtectionKind::Target).spec()?;
        ensure!(
            obs.as_of_ms <= obs.now_ms
                && obs.now_ms - obs.as_of_ms <= 5000
                && obs.quote.observed_at_ms <= obs.now_ms
                && obs.now_ms - obs.quote.observed_at_ms <= 5000,
            "Observasi posisi/harga usang; account_age_ms={}; quote_age_ms={}",
            i128::from(obs.now_ms) - i128::from(obs.as_of_ms),
            i128::from(obs.now_ms) - i128::from(obs.quote.observed_at_ms)
        );
        ensure!(
            obs.quote.bid > D::ZERO && obs.quote.ask >= obs.quote.bid && obs.mark > D::ZERO,
            "Observasi harga invalid"
        );
        if !obs.compatible_account || obs.unowned_orders {
            return Ok(Next::Halt("Mode akun/order asing perlu rekonsiliasi"));
        }
        let Some(entry_row) = self.row(box_, "entry") else {
            if stop || obs.now_ms >= self.entry_deadline_ms {
                return Ok(Next::Halt("Entry dihentikan atau deadline lewat"));
            }
            if !obs.position_qty.is_zero() {
                return Ok(Next::Halt("Posisi awal belum kosong"));
            }
            for (suffix, command, matched) in [
                (
                    "margin",
                    Command::Isolated {
                        symbol: self.symbol,
                    },
                    obs.isolated,
                ),
                (
                    "leverage",
                    Command::Leverage {
                        symbol: self.symbol,
                        leverage: self.plan.leverage,
                    },
                    obs.leverage == self.plan.leverage,
                ),
            ] {
                if let Some(row) = self.row(box_, suffix) {
                    ensure!(row.command == command, "Konfigurasi intent berubah");
                    if row.status == Status::Uncertain {
                        return Ok(self.query(suffix));
                    }
                    // A no-change error still needs GET confirmation of the desired state.
                    if !matched {
                        return Ok(self.query(suffix));
                    }
                } else if !matched {
                    return Ok(self.send(suffix, command));
                }
            }
            let valid = match self.plan.side {
                Side::Long => {
                    obs.quote.ask <= self.plan.worst_entry
                        && self.plan.stop < obs.quote.bid.min(obs.mark)
                        && self.plan.take_profit > obs.quote.ask.max(obs.mark)
                }
                Side::Short => {
                    obs.quote.bid >= self.plan.worst_entry
                        && self.plan.stop > obs.quote.ask.max(obs.mark)
                        && self.plan.take_profit < obs.quote.bid.min(obs.mark)
                }
            };
            if !valid {
                return Ok(Next::Halt(
                    "Harga berubah; sizing/stop harus dihitung ulang",
                ));
            }
            return Ok(self.send("entry", self.entry()));
        };
        ensure!(
            entry_row.command == self.entry(),
            "Intent entry berubah setelah reserve"
        );
        if entry_row.status == Status::Uncertain {
            return Ok(self.query("entry"));
        }
        if entry_row.status == Status::Rejected {
            return Ok(Next::Halt("Entry ditolak; sesi tidak mengulang order"));
        }
        let mut entry = order(entry_row)?.clone();
        if let Some(cancel) = self.row(box_, "cancel-entry")
            && let Some(Receipt::Order(report)) = &cancel.receipt
            && report.executed_qty >= entry.executed_qty
            && report.status.terminal()
        {
            entry = report.clone();
        }
        let exposed = entry.executed_qty > D::ZERO;
        if exposed && !obs.position_qty.is_zero() {
            if (obs.position_qty > D::ZERO) != (self.plan.side == Side::Long) {
                return Ok(Next::Halt("Arah posisi tidak sesuai order milik sesi"));
            }
            if obs.position_qty.abs() > entry.executed_qty {
                return Ok(self.query("entry"));
            }
        }
        let mut closing = stop;
        // Any reserved close keeps the exit intent latched after process restart.
        closing |= self.row(box_, "exit").is_some();
        for (suffix, kind) in [("sl", ProtectionKind::Stop), ("tp", ProtectionKind::Target)] {
            if let Some(row) = self.row(box_, suffix) {
                ensure!(
                    row.command == self.protection(kind),
                    "Intent proteksi berubah"
                );
                if row.status == Status::Rejected {
                    closing = true;
                }
                if let Some(Receipt::Algo(a)) = &row.receipt
                    && a.status != "NEW"
                {
                    closing = true;
                }
            }
        }
        if exposed && !closing {
            let protection = self.row(box_, "sl");
            match protection {
                None => return Ok(self.send("sl", self.protection(ProtectionKind::Stop))),
                Some(row) if row.status == Status::Uncertain => return Ok(self.query("sl")),
                Some(row) => {
                    algo(row)?;
                }
            }
        }
        // Protect a partial fill before canceling its remaining entry quantity.
        if !entry.status.terminal() && (exposed || closing || obs.now_ms >= self.entry_deadline_ms)
        {
            let command = Command::CancelOrder {
                symbol: self.symbol,
                client_id: self.key("entry"),
            };
            return Ok(match self.row(box_, "cancel-entry") {
                None => self.send("cancel-entry", command),
                Some(row) if row.status == Status::Uncertain => self.query("cancel-entry"),
                _ => self.query("entry"),
            });
        }
        if !entry.status.terminal() {
            return Ok(self.query("entry"));
        }
        if !exposed {
            return Ok(Next::Halt(
                "Entry terminal tanpa fill; tidak ada posisi baru",
            ));
        }
        // Fill price outside the prepared entry assumption closes the exposure.
        let actual_entry = entry
            .quote_qty
            .checked_div(entry.executed_qty)
            .context("Fill entry invalid")?;
        closing |= match self.plan.side {
            Side::Long => actual_entry > self.plan.worst_entry,
            Side::Short => actual_entry < self.plan.worst_entry,
        };
        if !closing {
            if obs.position_qty.is_zero() {
                return Ok(Next::Observe(
                    "Fill ada, posisi belum terlihat; baca posisi dan kedua proteksi lagi",
                ));
            }
            return Ok(match self.row(box_, "tp") {
                None => self.send("tp", self.protection(ProtectionKind::Target)),
                Some(row) if row.status == Status::Uncertain => self.query("tp"),
                Some(row) => {
                    algo(row)?;
                    Next::Watch {
                        keys: vec![self.key("entry"), self.key("sl"), self.key("tp")],
                    }
                }
            });
        }
        if !obs.position_qty.is_zero() {
            for attempt in 0..3 {
                let suffix = if attempt == 0 {
                    "exit".into()
                } else {
                    format!("exit{attempt}")
                };
                if let Some(row) = self.row(box_, &suffix) {
                    if row.status == Status::Uncertain {
                        return Ok(self.query(&suffix));
                    }
                    if row.status == Status::Rejected {
                        continue;
                    }
                    let exit = order(row)?;
                    if !exit.status.terminal() {
                        return Ok(self.query(&suffix));
                    }
                    continue;
                }
                return Ok(self.send(
                    &suffix,
                    Command::Market {
                        symbol: self.symbol,
                        client_id: self.key(&suffix),
                        side: opposite(self.plan.side),
                        qty: obs.position_qty.abs(),
                        reduce_only: true,
                    },
                ));
            }
            return Ok(Next::Observe(
                "Tiga exit sudah terminal/reject tetapi posisi tersisa; perlu pemulihan",
            ));
        }
        // Never remove server protection until fresh account data is flat and
        // the entry can no longer add exposure.
        let mut children = Vec::new();
        for suffix in ["sl", "tp"] {
            let Some(row) = self.row(box_, suffix) else {
                continue;
            };
            if row.status == Status::Uncertain {
                return Ok(self.query(suffix));
            }
            if row.status == Status::Rejected {
                continue;
            }
            let a = algo(row)?;
            if let Some(id) = a.actual_order_id {
                children.push(id);
            }
            if a.terminal() {
                continue;
            }
            let cancel_key = format!("cancel-{suffix}");
            if let Some(cancel) = self.row(box_, &cancel_key) {
                if let Some(Receipt::Algo(terminal)) = &cancel.receipt
                    && cancel.reconciled
                    && terminal.terminal()
                {
                    if let Some(id) = terminal.actual_order_id {
                        children.push(id);
                    }
                    continue;
                }
                return Ok(self.query(&cancel_key));
            }
            if a.status == "NEW" {
                return Ok(self.send(
                    &cancel_key,
                    Command::CancelAlgo {
                        client_id: self.key(suffix),
                    },
                ));
            }
            return Ok(self.query(suffix));
        }
        let mut exit_order_id = None;
        for suffix in ["exit", "exit1", "exit2"] {
            if let Some(row) = self.row(box_, suffix) {
                if row.status == Status::Uncertain {
                    return Ok(self.query(suffix));
                }
                if let Some(Receipt::Order(exit)) = &row.receipt {
                    if !exit.status.terminal() {
                        return Ok(self.query(suffix));
                    }
                    if exit_order_id.is_none() {
                        exit_order_id = Some(exit.exchange_id);
                    } else {
                        children.push(exit.exchange_id);
                    }
                }
            }
        }
        children.sort_unstable();
        children.dedup();
        Ok(Next::NeedsAccounting {
            entry_order_id: entry.exchange_id,
            exit_order_id,
            child_order_ids: children,
        })
    }
}
fn order(row: &Record) -> Result<&Order> {
    match &row.receipt {
        Some(Receipt::Order(o)) => Ok(o),
        _ => anyhow::bail!("Receipt order tidak tersedia"),
    }
}
fn algo(row: &Record) -> Result<&Algo> {
    match &row.receipt {
        Some(Receipt::Algo(a)) => Ok(a),
        _ => anyhow::bail!("Receipt proteksi tidak tersedia"),
    }
}
