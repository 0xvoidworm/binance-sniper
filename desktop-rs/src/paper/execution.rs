use super::{
    Phase, Snapshot,
    public::Batch,
    strategy::{self, Bar, INTERVAL},
};
use anyhow::{Context, Result, bail, ensure};
use binance_engine::{
    Config, Effect, Envelope, Event, ExitReason, Fill, Journal, OrderStatus, Side,
};
use rust_decimal::Decimal as D;
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    time::Instant,
};

pub const SESSION_MS: u64 = 3_600_000;
pub struct Session {
    journal: Journal,
    decisions: File,
    pub snapshot: Snapshot,
    history: Vec<Bar>,
    next_funding: u64,
    stopping: Option<String>,
    pub last_good: Instant,
}
#[derive(Serialize)]
struct Decision<'a> {
    at_ms: u64,
    message: &'a str,
    signal: &'a Option<strategy::Signal>,
}

pub fn config(batch: &Batch) -> Result<Config> {
    let mut config: Config =
        serde_json::from_str(include_str!("../../../engine-rs/fixtures/paper.json"))?;
    config.symbol = "BTCUSDT".into();
    config.capital = D::from(1000);
    config.risk_per_trade = D::new(25, 1);
    config.max_notional = D::from(250);
    config.max_daily_loss = D::from(10);
    config.max_total_loss = D::from(30);
    config.fee_bps = D::from(6);
    config.slippage_bps = D::ONE;
    config.max_spread_bps = D::from(5);
    config.max_quote_age_ms = 5000;
    config.filters = batch.filters.clone();
    config.validate()?;
    config.validate_quote(&batch.quote, batch.now)?;
    Ok(config)
}
impl Session {
    pub fn new(path: PathBuf, batch: &Batch) -> Result<Self> {
        let config = config(batch)?;
        fs::create_dir(&path).context("Folder sesi sudah ada atau tidak bisa dibuat")?;
        let journal = Journal::create(&path.join("journal.jsonl"), config)?;
        let decisions = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(path.join("decisions.jsonl"))?;
        let mut this = Self {
            snapshot: Snapshot {
                phase: Phase::Running,
                reason: "Warmup selesai; menunggu candle baru".into(),
                started_ms: batch.now,
                now: batch.now,
                deadline_ms: batch.now + SESSION_MS,
                path: Some(path),
                state: Some(journal.state().clone()),
                signal: Some(strategy::signal(&batch.bars)?),
                next_funding: batch.next_funding,
                ..Snapshot::default()
            },
            journal,
            decisions,
            history: batch.bars.clone(),
            next_funding: batch.next_funding,
            stopping: None,
            last_good: Instant::now(),
        };
        this.event(
            Event::Quote {
                quote: batch.quote.clone(),
            },
            batch.now,
        )?;
        this.note("Warmup selesai; data historis tidak menghasilkan entry".into())?;
        this.refresh(batch.now)?;
        Ok(this)
    }
    fn event(&mut self, event: Event, now: u64) -> Result<()> {
        let mut events = VecDeque::from([event]);
        let mut limit = 0;
        while let Some(event) = events.pop_front() {
            limit += 1;
            ensure!(limit <= 32, "Siklus effect simulator melebihi batas");
            let applied = self.journal.apply(Envelope {
                id: format!("p{:08}", self.journal.records()),
                at_ms: now,
                event,
            })?;
            ensure!(
                applied.outcome.accepted,
                "Event simulator ditolak: {}",
                applied.outcome.error.as_deref().unwrap_or("unknown")
            );
            for effect in applied.outcome.effects {
                match effect {
                    Effect::PlaceEntry {
                        order_id,
                        side,
                        qty,
                    } => {
                        let fill = self.fill(side, qty, now)?;
                        events.push_back(Event::EntryReport {
                            order_id,
                            status: OrderStatus::Filled,
                            fill,
                            reconciled: false,
                        });
                    }
                    Effect::PlaceExit {
                        order_id,
                        side,
                        qty,
                        ..
                    } => {
                        let fill = self.fill(side, qty, now)?;
                        events.push_back(Event::ExitReport {
                            order_id,
                            status: OrderStatus::Filled,
                            fill,
                            reconciled: false,
                        });
                    }
                    Effect::PlaceProtection { order_id, .. } => {
                        events.push_back(Event::ProtectionReport {
                            order_id,
                            accepted: true,
                        })
                    }
                    Effect::CancelProtections { intent_id, .. } => {
                        events.push_back(Event::CleanupConfirmed { intent_id })
                    }
                    Effect::CancelEntry { order_id } => {
                        let fill = self
                            .journal
                            .state()
                            .active
                            .as_ref()
                            .context("Entry tidak tersedia")?
                            .entry
                            .fill
                            .clone();
                        events.push_back(Event::EntryReport {
                            order_id,
                            status: OrderStatus::Canceled,
                            fill,
                            reconciled: false,
                        });
                    }
                    Effect::Halt { reason } => {
                        self.stopping = Some(format!("Risk halt: {reason}"));
                    }
                    Effect::QueryOrder { .. } => {
                        bail!("Simulator tidak dapat merekonsiliasi order ambigu")
                    }
                }
            }
        }
        Ok(())
    }
    fn fill(&self, side: Side, qty: D, now: u64) -> Result<Fill> {
        let state = self.journal.state();
        let quote = state.quote.as_ref().context("Quote fill tidak tersedia")?;
        state.config.validate_quote(quote, now)?;
        let rate = state.config.slippage_bps / D::from(10_000);
        let f = &state.config.filters;
        let price = match side {
            Side::Long => grid(quote.ask * (D::ONE + rate), f.tick_size, f.min_price, false),
            Side::Short => grid(quote.bid * (D::ONE - rate), f.tick_size, f.min_price, true),
        };
        let notional = (qty * price).round_dp(12).normalize();
        let fee = (notional * state.config.fee_bps / D::from(10_000))
            .round_dp(12)
            .normalize();
        Ok(Fill { qty, notional, fee })
    }
    fn note(&mut self, message: String) -> Result<()> {
        self.snapshot.reason = message.clone();
        self.snapshot
            .history
            .push(format!("{} · {}", self.snapshot.now, message));
        if self.snapshot.history.len() > 30 {
            self.snapshot.history.remove(0);
        }
        serde_json::to_writer(
            &mut self.decisions,
            &Decision {
                at_ms: self.snapshot.now,
                message: &message,
                signal: &self.snapshot.signal,
            },
        )?;
        self.decisions.write_all(b"\n")?;
        self.decisions.sync_data()?;
        Ok(())
    }
    pub fn request_stop(&mut self, reason: &str) -> Result<()> {
        if self.stopping.is_none() {
            self.stopping = Some(reason.into());
            self.snapshot.phase = Phase::Stopping;
            self.note(format!(
                "{reason}; entry berhenti, penutupan virtual menunggu quote segar"
            ))?;
        }
        if self.journal.state().active.is_none() {
            self.finish()?;
        }
        Ok(())
    }
    pub fn outage(&mut self, error: &str) -> Result<()> {
        if self.snapshot.phase.finished() {
            return Ok(());
        }
        self.snapshot.phase = Phase::Recovering;
        self.snapshot.error = Some(error.into());
        if self.last_good.elapsed().as_secs() >= 8 {
            self.request_stop("Data pasar terputus; sesi dihentikan")?;
        }
        self.persist()
    }
    pub fn fail(&mut self, error: &str) {
        self.snapshot.phase = Phase::Failed;
        self.snapshot.error = Some(error.into());
        self.snapshot.complete_costs = false;
        self.snapshot.state = Some(self.journal.state().clone());
        self.snapshot.records = self.journal.records();
        let _ = self
            .note("Sesi gagal; jangan menafsirkan posisi/hasil terbuka sebagai hasil akhir".into());
        let _ = self.persist();
    }
    fn finish(&mut self) -> Result<()> {
        self.snapshot.phase = Phase::Finished;
        self.snapshot.state = Some(self.journal.state().clone());
        self.note(
            self.stopping
                .clone()
                .unwrap_or_else(|| "Sesi selesai".into()),
        )?;
        self.persist()
    }
    fn refresh(&mut self, now: u64) -> Result<()> {
        self.snapshot.now = now;
        self.snapshot.state = Some(self.journal.state().clone());
        self.snapshot.records = self.journal.records();
        self.snapshot.updates += 1;
        self.snapshot.received = Some(Instant::now());
        self.snapshot.error = None;
        self.last_good = Instant::now();
        self.persist()
    }
    fn persist(&self) -> Result<()> {
        let path = self
            .snapshot
            .path
            .as_ref()
            .context("Path sesi tidak tersedia")?;
        let temporary = path.join("status.tmp");
        let mut file = File::create(&temporary)?;
        serde_json::to_writer_pretty(&mut file, &self.snapshot)?;
        file.sync_all()?;
        fs::rename(temporary, path.join("status.json"))?;
        Ok(())
    }
    pub fn update(&mut self, batch: &Batch) -> Result<()> {
        ensure!(!self.snapshot.phase.finished(), "Sesi sudah selesai");
        ensure!(batch.now >= self.snapshot.now, "Clock pasar mundur");
        ensure!(
            batch.filters == self.journal.state().config.filters,
            "Filter bursa berubah; sesi perlu dimulai ulang"
        );
        self.journal
            .state()
            .config
            .validate_quote(&batch.quote, batch.now)?;
        if self.journal.state().active.is_some() && batch.now >= self.next_funding {
            self.snapshot.complete_costs = false;
            self.stopping = Some(
                "Posisi virtual melewati funding saat data tidak tersedia; P&L tidak lengkap"
                    .into(),
            );
        }
        self.snapshot.now = batch.now;
        self.next_funding = batch.next_funding;
        self.snapshot.next_funding = batch.next_funding;
        if batch.now >= self.snapshot.deadline_ms {
            self.stopping = Some("Batas sesi 60 menit tercapai".into());
        }
        if self.last_good.elapsed().as_secs() >= 8 {
            self.stopping = Some("Data sempat terputus; sesi dihentikan".into());
        }
        let completed = self.journal.state().completed_trades;
        self.event(
            Event::Quote {
                quote: batch.quote.clone(),
            },
            batch.now,
        )?;
        if self.journal.state().completed_trades > completed {
            self.note("Posisi virtual ditutup oleh TP/SL atau batas risiko".into())?;
        }
        if self.stopping.is_some() {
            if self.journal.state().active.is_some() {
                self.event(
                    Event::Close {
                        reason: ExitReason::Manual,
                    },
                    batch.now,
                )?;
            }
            self.refresh(batch.now)?;
            return self.finish();
        }
        self.snapshot.phase = Phase::Running;
        if batch.next_funding.saturating_sub(batch.now) <= 300_000
            && self.journal.state().active.is_some()
        {
            self.event(
                Event::Close {
                    reason: ExitReason::Manual,
                },
                batch.now,
            )?;
            self.note("Posisi virtual ditutup sebelum funding".into())?;
        }
        for old in &self.history {
            if let Some(new) = batch.bars.iter().find(|b| b.open_ms == old.open_ms) {
                ensure!(new == old, "Candle selesai direvisi");
            }
        }
        let old = self.history.last().context("Warmup hilang")?;
        let latest = batch.bars.last().context("Bar selesai tidak tersedia")?;
        if latest.close_ms == old.close_ms {
            return self.refresh(batch.now);
        }
        ensure!(
            latest.close_ms == old.close_ms + INTERVAL,
            "Ada candle terlewat; tidak mengejar sinyal lampau"
        );
        self.history = batch.bars.clone();
        let signal = strategy::signal(&self.history)?;
        self.snapshot.signal = Some(signal.clone());
        let reason = if latest.close_ms <= self.snapshot.started_ms {
            "Candle selesai sebelum sesi dimulai; entry dilewati"
        } else if batch.now - latest.close_ms > 12_000 {
            "Candle baru terlambat; entry dilewati"
        } else if self.journal.state().active.is_some() {
            "Posisi virtual masih terbuka; tidak menambah posisi"
        } else if batch.next_funding - batch.now < 1_800_000 {
            "Menunggu: funding kurang dari 30 menit"
        } else if signal.atr <= D::ZERO {
            "ATR tidak positif; entry ditahan"
        } else if let Some(side) = signal.side {
            let cfg = &self.journal.state().config;
            let f = &cfg.filters;
            let (stop, tp) = match side {
                Side::Long => (
                    grid(
                        batch.quote.bid - D::from(2) * signal.atr,
                        f.tick_size,
                        f.min_price,
                        false,
                    ),
                    grid(
                        batch.quote.ask + D::from(3) * signal.atr,
                        f.tick_size,
                        f.min_price,
                        true,
                    ),
                ),
                Side::Short => (
                    grid(
                        batch.quote.ask + D::from(2) * signal.atr,
                        f.tick_size,
                        f.min_price,
                        true,
                    ),
                    grid(
                        batch.quote.bid - D::from(3) * signal.atr,
                        f.tick_size,
                        f.min_price,
                        false,
                    ),
                ),
            };
            if cfg.plan(&batch.quote, batch.now, side, stop, tp).is_ok() {
                self.event(
                    Event::Enter {
                        intent_id: format!("s{}", signal.bar_ms),
                        side,
                        stop,
                        take_profit: tp,
                    },
                    batch.now,
                )?;
                if side == Side::Long {
                    "Entry LONG virtual; stop/target virtual aktif"
                } else {
                    "Entry SHORT virtual; stop/target virtual aktif"
                }
            } else {
                "Entry ditahan: sizing/filter/spread/risiko tidak lolos"
            }
        } else {
            "Menunggu: tren EMA dan penembusan bar belum searah"
        };
        self.note(reason.into())?;
        self.refresh(batch.now)
    }
}
fn grid(price: D, tick: D, min: D, up: bool) -> D {
    let ticks = (price - min) / tick;
    (min + (if up { ticks.ceil() } else { ticks.floor() }) * tick).normalize()
}

#[cfg(test)]
mod tests;
