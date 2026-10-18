use anyhow::{Context, Result, ensure};
use binance_engine::{
    Config, Effect, Engine, Envelope, Event, ExitReason, Fill, Journal, OrderStatus, Quote, Side,
    State,
};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page {
    #[default]
    Workspace,
    Account,
    Futures,
    Paper,
    Simulation,
    Journal,
    Settings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub scale: f32,
    pub speed: f32,
    pub show_grid: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            scale: 1.0,
            speed: 1.0,
            show_grid: true,
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.scale.is_finite() && (0.85..=1.35).contains(&self.scale),
            "Skala tampilan tidak valid"
        );
        ensure!(
            self.speed.is_finite() && (0.5..=3.0).contains(&self.speed),
            "Kecepatan demo tidak valid"
        );
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        ensure!(
            fs::metadata(path)?.len() <= 16_384,
            "File pengaturan terlalu besar"
        );
        let prefs: Self = serde_json::from_slice(&fs::read(path)?)?;
        prefs.validate()?;
        Ok(prefs)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let parent = path.parent().context("Folder pengaturan tidak valid")?;
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(".settings-{}.tmp", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| -> Result<()> {
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

#[derive(Clone)]
pub struct Snapshot {
    pub state: State,
    pub effects: Vec<Effect>,
    pub label: String,
}

#[derive(Clone, Copy)]
pub struct PricePoint {
    pub step: usize,
    pub price: f32,
}

pub struct Replay {
    pub config: Config,
    pub events: Vec<Envelope>,
    pub snapshots: Vec<Snapshot>,
    pub prices: Vec<PricePoint>,
    pub cursor: usize,
    pub playing: bool,
}

impl Replay {
    pub fn new() -> Result<Self> {
        let config: Config =
            serde_json::from_str(include_str!("../../engine-rs/fixtures/paper.json"))?;
        let mut engine = Engine::new(config.clone())?;
        let mut snapshots = vec![Snapshot {
            state: engine.state().clone(),
            effects: vec![],
            label: "Sesi demo siap. Belum ada order.".into(),
        }];
        let mut events = vec![];
        let mut prices = vec![];
        let mut push = |event: Event, label: &str| -> Result<()> {
            let index = events.len();
            let envelope = Envelope {
                id: format!("ui-{:03}", index + 1),
                at_ms: 1_000_000 + index as u64 * 100,
                event,
            };
            let outcome = engine.apply(&envelope);
            ensure!(outcome.accepted, "Demo ditolak engine: {:?}", outcome.error);
            if let Event::Quote { quote } = &envelope.event {
                prices.push(PricePoint {
                    step: index + 1,
                    price: quote.bid.to_string().parse()?,
                });
            }
            snapshots.push(Snapshot {
                state: engine.state().clone(),
                effects: outcome.effects,
                label: label.into(),
            });
            events.push(envelope);
            Ok(())
        };
        let price = D::new(1001, 1);
        let quote = Quote {
            bid: D::from(100),
            ask: price,
            observed_at_ms: 1_000_000,
        };
        let qty = config
            .plan(&quote, 1_000_000, Side::Long, D::from(98), D::from(104))?
            .qty;
        let half =
            (qty / D::from(2)).round_dp_with_strategy(3, rust_decimal::RoundingStrategy::ToZero);
        let fill = |q: D, p: D| Fill {
            qty: q,
            notional: q * p,
            fee: q * p * D::new(5, 4),
        };
        push(Event::Quote { quote }, "Quote sintetis diterima")?;
        push(
            Event::Enter {
                intent_id: "desktop-demo".into(),
                side: Side::Long,
                stop: D::from(98),
                take_profit: D::from(104),
            },
            "Usulan long lolos sizing dan batas risiko",
        )?;
        push(
            Event::EntryUncertain {
                order_id: "desktop-demo-entry".into(),
            },
            "Status entry tidak pasti; query diperlukan",
        )?;
        push(
            Event::EntryReport {
                order_id: "desktop-demo-entry".into(),
                status: OrderStatus::PartiallyFilled,
                fill: fill(half, price),
                reconciled: true,
            },
            "Rekonsiliasi: entry terisi sebagian",
        )?;
        push(
            Event::ProtectionReport {
                order_id: "desktop-demo-sl".into(),
                accepted: true,
            },
            "Stop-loss diakui simulator",
        )?;
        push(
            Event::ProtectionReport {
                order_id: "desktop-demo-tp".into(),
                accepted: true,
            },
            "Take-profit diakui simulator",
        )?;
        push(
            Event::EntryReport {
                order_id: "desktop-demo-entry".into(),
                status: OrderStatus::Filled,
                fill: fill(qty, price),
                reconciled: false,
            },
            "Entry terisi penuh; posisi diproteksi",
        )?;
        // A frozen illustrative path, not sampled from Binance or a strategy.
        let path = [
            10005, 10018, 10031, 10022, 10007, 9991, 9975, 9958, 9940, 9955, 9980, 10010, 10036,
            10055, 10048, 10076, 10102, 10126, 10103, 10090, 10119, 10144, 10168, 10152, 10178,
            10203, 10225, 10204, 10240, 10262, 10283, 10265, 10302, 10327, 10308, 10339, 10363,
            10348, 10379, 10410,
        ];
        for (offset, cents) in path.iter().enumerate() {
            let bid = D::new(*cents, 2);
            push(
                Event::Quote {
                    quote: Quote {
                        bid,
                        ask: bid + D::new(1, 1),
                        observed_at_ms: 1_000_000 + (7 + offset as u64) * 100,
                    },
                },
                if *cents >= 10400 {
                    "Harga menyentuh TP; mesin meminta exit"
                } else {
                    "Quote sintetis; posisi masih terbuka"
                },
            )?;
        }
        push(
            Event::ExitReport {
                order_id: "desktop-demo-exit".into(),
                status: OrderStatus::PartiallyFilled,
                fill: fill(half, D::new(1041, 1)),
                reconciled: false,
            },
            "Exit terisi sebagian",
        )?;
        push(
            Event::ExitReport {
                order_id: "desktop-demo-exit".into(),
                status: OrderStatus::Filled,
                fill: fill(qty, D::new(1041, 1)),
                reconciled: false,
            },
            "Exit penuh; permintaan cleanup proteksi",
        )?;
        push(
            Event::CleanupConfirmed {
                intent_id: "desktop-demo".into(),
            },
            "Siklus selesai; posisi dan proteksi dibersihkan",
        )?;
        Ok(Self {
            config,
            events,
            snapshots,
            prices,
            cursor: 0,
            playing: false,
        })
    }
    pub fn current(&self) -> &Snapshot {
        &self.snapshots[self.cursor]
    }
    pub fn last(&self) -> usize {
        self.snapshots.len() - 1
    }
    pub fn seek(&mut self, step: usize) {
        self.cursor = step.min(self.last());
        self.playing = false;
    }
    pub fn advance(&mut self) {
        if self.cursor < self.last() {
            self.cursor += 1;
        }
        if self.cursor == self.last() {
            self.playing = false;
        }
    }
    pub fn toggle(&mut self) {
        if self.cursor == self.last() {
            self.cursor = 0;
        }
        self.playing = !self.playing;
    }
    pub fn export(&self, parent: &Path) -> Result<PathBuf> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        fs::create_dir_all(parent)?;
        let directory = parent.join(format!("binance-desk-demo-{stamp}"));
        fs::create_dir(&directory)?;
        let mut journal = Journal::create(&directory.join("journal.jsonl"), self.config.clone())?;
        for event in self.events.iter().take(self.cursor) {
            ensure!(
                journal.apply(event.clone())?.outcome.accepted,
                "Export replay rejected"
            );
        }
        ensure!(
            journal.state() == &self.current().state,
            "Export state mismatch"
        );
        let report = serde_json::json!({"source": "synthetic desktop demo", "live_trading": false, "step": self.cursor, "total_steps": self.last(), "head_hash": journal.head_hash(), "state": journal.state()});
        fs::write(
            directory.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        fs::write(
            directory.join("README.md"),
            "# Ekspor Binance Desk\n\nDemo sintetis untuk memeriksa UI dan engine. Bukan transaksi Binance atau hasil strategi. report.json dan journal.jsonl mencakup event sampai langkah yang dipilih.\n",
        )?;
        Ok(directory)
    }
}

pub fn effect_label(effect: &Effect) -> &'static str {
    match effect {
        Effect::PlaceEntry { .. } => "Permintaan entry",
        Effect::QueryOrder { .. } => "Query status order",
        Effect::CancelEntry { .. } => "Batalkan sisa entry",
        Effect::PlaceProtection {
            kind: ExitReason::StopLoss,
            ..
        } => "Pasang stop-loss",
        Effect::PlaceProtection { .. } => "Pasang take-profit",
        Effect::PlaceExit { .. } => "Permintaan exit",
        Effect::CancelProtections { .. } => "Bersihkan proteksi",
        Effect::Halt { .. } => "Hentikan entry baru",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_controls_track_engine_and_complete_cleanup() {
        let mut r = Replay::new().unwrap();
        assert_eq!(r.cursor, 0);
        r.toggle();
        assert!(r.playing);
        r.advance();
        assert_eq!(r.cursor, 1);
        r.seek(7);
        assert!(!r.playing);
        assert!(r.current().state.active.is_some());
        r.seek(usize::MAX);
        assert_eq!(r.cursor, r.last());
        assert_eq!(r.current().state.completed_trades, 1);
        assert!(r.current().state.active.is_none());
        assert_eq!(r.current().state.realized_net, D::new(162581409, 7));
        r.toggle();
        assert_eq!(r.cursor, 0);
        assert!(r.playing);
        while r.playing {
            r.advance();
        }
        assert_eq!(r.cursor, r.last());
    }
    #[test]
    fn export_matches_selected_partial_state_and_replays() {
        let mut r = Replay::new().unwrap();
        r.seek(4);
        let temp = tempfile::tempdir().unwrap();
        let path = r.export(temp.path()).unwrap();
        let journal = Journal::open(&path.join("journal.jsonl")).unwrap();
        assert_eq!(journal.state(), &r.current().state);
        assert_eq!(journal.records(), 5);
    }
    #[test]
    fn preferences_roundtrip_and_invalid_values_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("prefs.json");
        let prefs = Preferences {
            scale: 1.2,
            speed: 2.0,
            show_grid: false,
        };
        prefs.save(&path).unwrap();
        assert_eq!(Preferences::load(&path).unwrap().scale, 1.2);
        assert!(
            Preferences {
                scale: f32::NAN,
                ..prefs.clone()
            }
            .save(&path)
            .is_err()
        );
        fs::write(&path, "{\"scale\":10,\"speed\":1,\"show_grid\":true}").unwrap();
        assert!(Preferences::load(&path).is_err());
    }
}
