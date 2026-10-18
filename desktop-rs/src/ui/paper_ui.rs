use super::*;
use crate::paper::Phase;

impl Desk {
    pub fn start_paper(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.network_disabled && !self.paper_report,
            "Sesi ini hanya offline/laporan"
        );
        anyhow::ensure!(
            self.paper_feed.is_none() || self.paper.phase.finished(),
            "Sesi paper masih aktif"
        );
        let feed = crate::paper::Feed::start(self.export_path.clone())?;
        self.paper = feed.latest();
        self.paper_feed = Some(feed);
        Ok(())
    }
    pub(super) fn paper_page(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Mesin paper",
            "BTCUSDT Futures · harga Binance nyata, posisi dan modal virtual.",
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            badge(
                ui,
                if self.paper_report {
                    "LAPORAN TERSIMPAN"
                } else {
                    self.paper.phase.label()
                },
                match self.paper.phase {
                    Phase::Running => GREEN,
                    Phase::Failed => RED,
                    Phase::Recovering | Phase::Stopping => GOLD,
                    _ => MUTED,
                },
            );
            badge(ui, "TANPA ORDER UANG NYATA", GOLD);
            if self.paper.phase == Phase::Running && !self.paper.fresh() && !self.paper_report {
                badge(ui, "DATA TERLAMBAT", GOLD);
            }
        });
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let enabled = !self.network_disabled
                && !self.paper_report
                && (self.paper_feed.is_none() || self.paper.phase.finished());
            let response = ui.add_enabled(
                enabled,
                egui::Button::new(RichText::new("Mulai sesi 60 menit").color(BG).strong())
                    .fill(GREEN),
            );
            self.widgets.insert("start-paper", response.rect);
            if response.clicked()
                && let Err(error) = self.start_paper()
            {
                self.notice = Some(error.to_string());
            }
            let response = ui.add_enabled(
                self.paper_feed.is_some() && !self.paper.phase.finished(),
                egui::Button::new("Hentikan & tutup virtual"),
            );
            self.widgets.insert("stop-paper", response.rect);
            if response.clicked() && self.paper_feed.as_ref().is_some_and(|f| !f.stop()) {
                self.notice = Some("Worker paper sudah berhenti".into());
            }
        });
        ui.add_space(12.0);
        card().show(ui,|ui| {
            ui.label(RichText::new(&self.paper.reason).strong().size(16.0));
            if let Some(error)=&self.paper.error {ui.colored_label(GOLD,error);}
            if self.paper.started_ms>0 {
                let end=chrono::DateTime::from_timestamp_millis(self.paper.deadline_ms as i64).map_or_else(||"—".into(),|t|t.format("%H:%M UTC").to_string());
                ui.label(format!("Berakhir otomatis {end} · {} pembaruan · {} record jurnal",self.paper.updates,self.paper.records));
            }
            ui.label(RichText::new("EMA20/50 + penembusan bar 5m · stop 2×ATR14 · target 3×ATR14 · satu posisi.").color(MUTED));
            ui.label(RichText::new("Aturan ini belum terbukti menghasilkan profit. Warmup tidak membuat transaksi historis.").size(12.0).color(MUTED));
        });
        ui.add_space(12.0);
        if let Some(state) = &self.paper.state {
            ui.columns(3, |col| {
                metric(
                    &mut col[0],
                    "EKUITAS VIRTUAL",
                    &money(state.equity),
                    "Modal awal virtual 1000 USDT",
                    TEXT,
                );
                metric(
                    &mut col[1],
                    "PNL VIRTUAL",
                    &signed(state.equity - state.config.capital),
                    "Termasuk fee model, posisi ditandai bid/ask",
                    if state.equity >= state.config.capital {
                        GREEN
                    } else {
                        RED
                    },
                );
                metric(
                    &mut col[2],
                    "TRADE VIRTUAL SELESAI",
                    &state.completed_trades.to_string(),
                    "Bukan transaksi akun Binance",
                    TEXT,
                );
            });
            if !self.paper.complete_costs {
                ui.colored_label(GOLD,"Hasil tidak lengkap: ada gap, sesi gagal, atau kemungkinan funding yang belum dimodelkan.");
            }
            ui.add_space(12.0);
            card().show(ui, |ui| {
                ui.strong("Posisi & proteksi virtual");
                if let Some(trade) = &state.active {
                    ui.horizontal_wrapped(|ui| {
                        key_value(
                            ui,
                            "ARAH",
                            if trade.side == binance_engine::Side::Long {
                                "LONG"
                            } else {
                                "SHORT"
                            },
                        );
                        key_value(ui, "JUMLAH", &trade.entry.fill.qty.normalize().to_string());
                        key_value(ui, "STOP", &trade.stop.normalize().to_string());
                        key_value(ui, "TARGET", &trade.take_profit.normalize().to_string());
                        key_value(ui, "SL", protection(trade.stop_status));
                        key_value(ui, "TP", protection(trade.take_profit_status));
                    });
                } else {
                    ui.label("Tidak ada posisi virtual terbuka.");
                }
                if let Some(q) = &state.quote {
                    ui.label(format!(
                        "Bid {} · Ask {}",
                        q.bid.normalize(),
                        q.ask.normalize()
                    ));
                }
            });
        }
        ui.add_space(12.0);
        card().show(ui,|ui| {
            ui.strong("Batas simulasi");
            ui.label("Risiko/entry 2.5 USDT · notional maks. 250 · batas rugi harian 10 / total 30 · tanpa leverage simulasi.");
            ui.label("Asumsi fee 6 bps/sisi, adverse slippage hingga 1 bp. Fill virtual langsung, bukan bukti likuiditas bursa.");
            ui.label("Entry ditahan 30 menit sebelum funding; posisi virtual ditutup sebelum 5 menit. Funding/likuidasi belum dimodelkan penuh.");
            if let Some(signal)=&self.paper.signal {
                ui.label(format!("EMA20 {:.2} · EMA50 {:.2} · ATR14 {:.2}",signal.fast,signal.slow,signal.atr));
            }
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.strong("Keputusan terakhir");
            for message in self.paper.history.iter().rev().take(10) {
                ui.label(message);
            }
            if let Some(path) = &self.paper.path {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Jurnal: {}", path.display()))
                        .size(11.0)
                        .color(MUTED),
                );
                if ui.button("Buka folder sesi").clicked() {
                    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                }
            }
        });
    }
}
