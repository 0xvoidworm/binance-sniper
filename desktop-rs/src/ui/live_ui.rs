use super::*;
use crate::futures::{
    live::{Control, Feed},
    margin::Budget,
    session_store::Config,
};
use binance_engine::Side;

pub(super) struct LiveForm {
    margin: String,
    risk: String,
    notional: String,
    daily: String,
    session: String,
    funding: String,
    leverage: u32,
    minutes: u32,
    trades: u32,
    acknowledge: bool,
    error: Option<String>,
}
impl Default for LiveForm {
    fn default() -> Self {
        Self {
            margin: String::new(),
            risk: String::new(),
            notional: String::new(),
            daily: String::new(),
            session: String::new(),
            funding: "0".into(),
            leverage: 1,
            minutes: 60,
            trades: 10,
            acknowledge: false,
            error: None,
        }
    }
}
impl LiveForm {
    fn from_config(config: &Config) -> anyhow::Result<Self> {
        config.validate()?;
        Ok(Self {
            margin: config.budget.margin.to_string(),
            risk: config.budget.risk.to_string(),
            notional: config.budget.max_notional.to_string(),
            daily: config.daily_loss.to_string(),
            session: config.session_loss.to_string(),
            funding: config.budget.funding_reserve.to_string(),
            leverage: config.budget.leverage,
            minutes: config.minutes,
            trades: config.max_trades,
            ..Self::default()
        })
    }
    #[cfg(test)]
    pub(super) fn fixture() -> Self {
        Self {
            margin: "50".into(),
            risk: "2".into(),
            notional: "100".into(),
            daily: "10".into(),
            session: "6".into(),
            ..Default::default()
        }
    }
    fn parse(&self) -> anyhow::Result<Config> {
        let number = |value: &str| {
            value
                .trim()
                .parse::<D>()
                .map_err(|_| anyhow::anyhow!("Isi semua batas dengan angka desimal; gunakan titik"))
        };
        let config = Config {
            budget: Budget {
                margin: number(&self.margin)?,
                risk: number(&self.risk)?,
                max_notional: number(&self.notional)?,
                funding_reserve: number(&self.funding)?,
                leverage: self.leverage,
                slippage_bps: D::ONE,
                liquidation_buffer_bps: D::from(10),
            },
            daily_loss: number(&self.daily)?,
            session_loss: number(&self.session)?,
            minutes: self.minutes,
            max_trades: self.trades,
        };
        config.validate()?;
        Ok(config)
    }
}
impl Desk {
    pub fn restore_live_session(&mut self) {
        self.live = crate::futures::live::Snapshot::saved(&self.live_root);
        if let Ok(Some(saved)) = crate::futures::session_store::read_saved(&self.live_root)
            && let Some(session) = saved.session
            && let Ok(form) = LiveForm::from_config(&session.config)
        {
            self.live_form = form;
        }
    }
    pub(super) fn live_locked(&self) -> bool {
        self.live_feed
            .as_ref()
            .map_or(self.live.active || self.live.busy, |feed| {
                let s = feed.latest();
                s.active || s.busy
            })
    }
    pub(super) fn live_command(&mut self, command: Control) {
        if self.network_disabled || self.capture.is_some() {
            self.live_form.error =
                Some("Aktivasi/koneksi sesi dinonaktifkan pada mode offline atau capture".into());
            return;
        }
        if self.live_feed.is_none() {
            match Feed::start(self.live_root.clone()) {
                Ok(feed) => self.live_feed = Some(feed),
                Err(e) => {
                    self.live_form.error = Some(e.to_string());
                    return;
                }
            }
        }
        if self.live_feed.as_ref().is_some_and(|f| f.send(command)) {
            self.live_form.error = None;
            self.live = self.live_feed.as_ref().unwrap().latest();
        } else {
            self.live_form.error =
                Some("Perintah belum diterima; tunggu sesi aktif selesai".into());
        }
    }
    pub(super) fn automatic_session(&mut self, ui: &mut egui::Ui) {
        let locked = self.live_locked();
        let enabled = !self.network_disabled && self.capture.is_none();
        card().show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new("Sesi otomatis").strong().size(19.0));
                badge(ui,if self.live.busy{"MENYIAPKAN / MEMBACA"}else if self.live.stale(){"DATA SESI TERLAMBAT"}else if self.live.ended_early(){"BERAKHIR LEBIH AWAL"}else{self.live.phase.label()},if self.live.active && !self.live.stale(){GREEN}else{GOLD});
            });
            ui.label("Strategi memilih Long, Short atau menunggu. SL 2 ATR; TP mulai 3 ATR, disesuaikan biaya maksimal 6 ATR. Bukan pembelajaran otomatis.");
            ui.label(RichText::new("BTCUSDT · candle 5 menit · breakout / pullback tren / pantulan range · satu posisi · One-way, Single-asset USDT, Isolated").color(MUTED).size(12.0));
            if self.capture.is_some() && (self.live_form.funding!="0" || [&self.live_form.margin,&self.live_form.risk,&self.live_form.notional,&self.live_form.session,&self.live_form.daily].iter().any(|v|!v.is_empty())) {ui.label("Isian batas sesi disamarkan pada capture audit.");}else{ui.add_enabled_ui(!locked && enabled,|ui|{
                ui.columns(2,|columns|{
                    for (index,rows) in [
                        (0,vec![("Budget margin USDT",&mut self.live_form.margin),("Risiko per posisi USDT",&mut self.live_form.risk),("Notional maksimum USDT",&mut self.live_form.notional)]),
                        (1,vec![("Batas rugi sesi USDT",&mut self.live_form.session),("Batas rugi harian UTC USDT",&mut self.live_form.daily),("Cadangan funding USDT",&mut self.live_form.funding)])
                    ] {egui::Grid::new(("session-input",index)).num_columns(2).show(&mut columns[index],|ui|{for(label,value)in rows{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(90.0).char_limit(24));ui.end_row();}});}
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label("Leverage");ui.add(egui::DragValue::new(&mut self.live_form.leverage).range(1..=125).suffix("×"));
                    ui.label("Durasi");ui.add(egui::DragValue::new(&mut self.live_form.minutes).range(1..=1440).suffix(" menit"));
                    ui.label("Maks. transaksi/sesi");ui.add(egui::DragValue::new(&mut self.live_form.trades).range(1..=100));
                });
            });}
            ui.label(RichText::new("Batas rugi adalah pemicu penghentian; gap/slippage bisa melewatinya. Jangan jalankan bot/order lain pada akun ini selama sesi. Belum ada bukti profit strategi ini.").size(12.0).color(GOLD));
            ui.label(RichText::new("Data usang/koneksi putus saat kosong: entry dijeda, candle pemulihan dilewati. Saat ada intent: amankan/tutup posisi, maksimal 2 episode pemulihan/sesi; setelah accounting cocok, perlu 60 detik data sehat (gap ≤10 detik), lalu menunggu candle baru. Stop, deadline, batas risiko dan kegagalan keselamatan tetap mengakhiri sesi.").size(12.0).color(MUTED));
            ui.add_enabled_ui(!locked && enabled,|ui|{
                let response=ui.checkbox(&mut self.live_form.acknowledge,"Gunakan dana Futures nyata untuk sesi ini");self.widgets.insert("live-ack",response.rect);
                ui.horizontal_wrapped(|ui|{
                    let r=ui.button("Analisis arah sekarang");self.widgets.insert("live-inspect",r.rect);if r.clicked(){self.live_command(Control::Inspect);}
                    let r=ui.button("Periksa kelayakan setup");self.widgets.insert("live-check",r.rect);if r.clicked(){match self.live_form.parse(){Ok(config)=>self.live_command(Control::Check(config)),Err(e)=>self.live_form.error=Some(e.to_string())}}
                    let r=ui.add_enabled(self.live_form.acknowledge && !self.live.unfinished && self.live.phase!=crate::futures::session_store::Phase::Blocked,egui::Button::new(RichText::new("Aktifkan sesi live").strong().color(BG)).fill(GREEN));self.widgets.insert("live-start",r.rect);
                    if r.clicked(){match self.live_form.parse(){Ok(config)=>{self.live_command(Control::Start(config));self.live_form.acknowledge=false;},Err(e)=>self.live_form.error=Some(e.to_string())}}
                    let r=ui.add_enabled(self.live_form.acknowledge && self.live.unfinished,egui::Button::new("Pulihkan & tutup sesi lama"));self.widgets.insert("live-resume",r.rect);
                    if r.clicked(){self.live_command(Control::Resume);self.live_form.acknowledge=false;}
                });
            });
            let r=ui.add_enabled(locked && enabled,egui::Button::new("Hentikan & tutup posisi").fill(Color32::from_rgb(95,40,52)));self.widgets.insert("live-stop",r.rect);if r.clicked(){self.live_command(Control::Stop);}
            if let Some(error)=&self.live_form.error{ui.colored_label(GOLD,error);}
            if let Some(error)=&self.live.error{ui.colored_label(GOLD,error);}
            if let Some(check)=&self.live.setup {
                if self.capture.is_some() {ui.label("Hasil pemeriksaan setup disamarkan pada capture audit.");}
                else if self.live_form.parse().as_ref().ok()!=Some(&check.config) {ui.colored_label(GOLD,"Isian berubah; periksa setup lagi. Aktivasi selalu memeriksa ulang sebelum sesi dibuat.");}
                else {
                    ui.colored_label(if check.report.viable(){GREEN}else{GOLD},check.report.summary());
                    ui.label(&check.market_summary);
                    ui.label(format!("Pemeriksaan {:.0} detik lalu; berlaku untuk snapshot itu saja. Aktivasi memeriksa data dan sisa batas rugi ulang.",check.received.elapsed().as_secs_f32()));
                }
            }
            ui.separator();
            if let Some(timing)=self.live.timing_summary(){ui.label(RichText::new(timing).strong());}
            if self.capture.is_some() && (self.live.active || self.live.completed>0) {ui.label("Status akun dan sesi disamarkan pada capture audit.");return;}
            ui.label(&self.live.reason);
            ui.label(RichText::new(format!("Versi aplikasi berjalan: {} · Hermes/model AI tidak aktif",env!("CARGO_PKG_VERSION"))).size(12.0).color(MUTED));
            if let Some(r)=&self.live.recovery {
                ui.label(format!("Episode pemulihan data/koneksi: {}/{}{}",r.episodes,crate::futures::recovery::MAX_EPISODES,if r.active {" · entry masih dijeda"} else {""}));
            }
            if let Some(d)=&self.live.diagnostics {
                ui.collapsing("Jejak keputusan sesi — penghitung aktual",|ui|{
                    ui.label(format!("Sesi dibuat oleh versi {} · {} candle dievaluasi",d.build.label(),d.evaluated_candles()));
                    ui.label("Sinyal, intent, order fill, dan transaksi selesai adalah tahap berbeda. Angka ini bukan peluang profit.");
                    egui::Grid::new("live-decision-counts").num_columns(2).show(ui,|ui|{
                        for gate in crate::futures::diagnostics::Gate::ALL {
                            ui.label(gate.label());ui.label(d.counts[gate as usize].to_string());ui.end_row();
                        }
                    });
                    if let Some(gate)=d.last_gate {ui.label(format!("Cabang tercatat terakhir: {}",gate.label()));}
                    if d.saturated {ui.colored_label(GOLD,"Penghitung mencapai kapasitas; bukan riwayat lengkap.");}
                });
            } else if self.live.started.is_some() {
                ui.label("Sesi lama tidak merekam versi pembuat/penghitung keputusan; tidak direkonstruksi sebagai nol.");
            }
            if let Some(seconds)=self.live.next_candle_seconds(){
                ui.label(if seconds>0{format!("Mesin memantau akun. Evaluasi candle 5 menit berikutnya dalam {seconds} detik.")}else{"Candle sudah ditutup; menunggu data terbaru untuk evaluasi.".into()});
            }
            if let Some(decision)=&self.live.decision {
                let side=match decision.side{Some(Side::Long)=>"LONG",Some(Side::Short)=>"SHORT",None=>"MENUNGGU"};
                ui.label(RichText::new(format!("Arah: {side} · {}",decision.reason)).strong());
                if !decision.votes.is_empty(){ui.label(format!("Sinyal: {}",decision.votes.iter().map(|v|v.strategy.as_str()).collect::<Vec<_>>().join(", ")));}
            }
            if self.live.started.is_some() || self.live.active || self.live.completed>0 {
                ui.horizontal_wrapped(|ui|{
                    key_value(ui,"POSISI BTC",&self.live.position.map_or_else(||"Belum terbaca".into(),|v|self.private_value(v)));
                    key_value(ui,"HASIL BERSIH USDT",&self.private_value(self.live.net));
                    key_value(ui,"FEE / FUNDING",&format!("{} / {}",self.private_value(self.live.fees),self.private_value(self.live.funding)));
                    key_value(ui,"POSISI SELESAI",&self.live.completed.to_string());
                });
                if let (Some(sl),Some(tp))=(self.live.stop,self.live.target){ui.label(format!("SL {} · TP {}",self.private_value(sl),self.private_value(tp)));}
            }
            if let Some(at)=self.live.received{ui.label(RichText::new(format!("Snapshot terakhir {:.0} detik lalu · periksa rekonsiliasi jika koneksi terputus",at.elapsed().as_secs_f32())).size(11.0).color(MUTED));}
            if self.close_after_live {ui.colored_label(GOLD,"Menunggu posisi dan pembukuan selesai sebelum menutup jendela.");}
        });
        ui.add_space(14.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoring_last_bounds_preserves_limits_without_authorizing_another_session() {
        let config = LiveForm::fixture().parse().unwrap();
        let restored = LiveForm::from_config(&config).unwrap();
        assert_eq!(restored.parse().unwrap(), config);
        assert!(!restored.acknowledge);
        assert!(restored.error.is_none());
    }
    #[test]
    fn explicit_session_bounds_require_valid_loss_hierarchy() {
        let mut f = LiveForm::default();
        assert!(f.parse().is_err());
        f.margin = "50".into();
        f.risk = "2".into();
        f.notional = "100".into();
        f.session = "6".into();
        f.daily = "10".into();
        assert!(f.parse().is_ok());
        f.daily = "1".into();
        assert!(f.parse().is_err());
        f.daily = "10".into();
        f.funding = "2".into();
        assert!(f.parse().is_err());
    }
}
