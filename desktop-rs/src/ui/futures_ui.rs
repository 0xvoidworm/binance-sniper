use super::*;
use crate::futures::{
    ReadinessCommand,
    margin::{self, Budget, Plan},
};
use binance_engine::Side;

pub(super) struct Form {
    margin: String,
    risk: String,
    notional: String,
    stop: String,
    target: String,
    funding: String,
    leverage: u32,
    side: Side,
    plan: Option<Plan>,
    error: Option<String>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            margin: String::new(),
            risk: String::new(),
            notional: String::new(),
            stop: String::new(),
            target: String::new(),
            funding: "0".into(),
            leverage: 1,
            side: Side::Long,
            plan: None,
            error: None,
        }
    }
}
impl Form {
    fn parse(&self) -> anyhow::Result<(Budget, D, D)> {
        let parse = |s: &str| -> anyhow::Result<D> {
            s.trim()
                .parse::<D>()
                .map_err(|_| anyhow::anyhow!("Isi seluruh angka dengan desimal, gunakan titik"))
        };
        Ok((
            Budget {
                margin: parse(&self.margin)?,
                risk: parse(&self.risk)?,
                max_notional: parse(&self.notional)?,
                leverage: self.leverage,
                slippage_bps: D::ONE,
                liquidation_buffer_bps: D::from(10),
                funding_reserve: parse(&self.funding)?,
            },
            parse(&self.stop)?,
            parse(&self.target)?,
        ))
    }
}
impl Desk {
    pub fn start_futures(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.network_disabled, "Sesi offline");
        if self.futures_feed.is_none() {
            self.futures_feed = Some(crate::futures::Feed::start()?);
        }
        Ok(())
    }
    pub fn futures_command(&mut self, command: ReadinessCommand) {
        self.futures_form.plan = None;
        if let Err(e) = self.start_futures() {
            self.futures_form.error = Some(e.to_string());
            return;
        }
        if !self.futures_feed.as_ref().is_some_and(|f| f.send(command)) {
            self.futures_form.error = Some("Worker Futures tidak tersedia".into());
        } else {
            self.futures_form.error = None;
            self.futures = crate::futures::Snapshot::default();
        }
    }
    pub(super) fn futures_page(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Eksekusi Futures",
            "BTCUSDT USD-M · strategi, risiko dan pengelolaan order otomatis.",
        );
        ui.add_space(12.0);
        self.automatic_session(ui);
        ui.strong("Pemeriksaan akun — baca saja");
        ui.horizontal_wrapped(|ui| {
            badge(
                ui,
                if self.futures.busy {
                    "MEMBACA BINANCE"
                } else if self.futures.fresh() {
                    "PREFLIGHT TERBACA"
                } else {
                    "PERLU PEMBARUAN"
                },
                if self.futures.fresh() { GREEN } else { GOLD },
            );
            badge(ui, "GET-ONLY", MUTED);
        });
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let r = ui.add_enabled(
                !self.network_disabled && !self.futures.busy,
                egui::Button::new("Baca dari KWallet"),
            );
            self.widgets.insert("futures-load", r.rect);
            if r.clicked() {
                self.futures_command(ReadinessCommand::Load);
            }
            let r = ui.add_enabled(
                !self.network_disabled && self.futures.connected && !self.futures.busy,
                egui::Button::new("Perbarui data"),
            );
            self.widgets.insert("futures-refresh", r.rect);
            if r.clicked() {
                self.futures_command(ReadinessCommand::Refresh);
            }
            let r = ui.add_enabled(
                !self.network_disabled && self.futures.connected,
                egui::Button::new("Putuskan"),
            );
            self.widgets.insert("futures-disconnect", r.rect);
            if r.clicked() {
                self.futures_command(ReadinessCommand::Disconnect);
            }
        });
        ui.label(RichText::new("Pembacaan ini tidak membuat order atau mengubah akun. Kredensial sesi juga bisa dihubungkan lewat Akun Futures.").size(12.0).color(MUTED));
        if let Some(error) = &self.futures.error {
            ui.colored_label(GOLD, error);
        }
        if let Some(error) = &self.futures_form.error {
            ui.colored_label(GOLD, error);
        }
        ui.add_space(10.0);
        if self.capture.is_some() && self.futures.data.is_some() {
            card().show(ui,|ui|{ui.label("Data akun, bracket, biaya dan perhitungan order disamarkan pada capture audit.");});
            self.execution_boundary(ui);
            return;
        }
        let Some(data) = self.futures.data.clone() else {
            card().show(ui,|ui|{ui.strong("Data Futures belum tersedia");ui.label("Hubungkan untuk membaca leverage bracket akun, biaya taker, margin, serta order reguler dan TP/SL aktif.");});
            self.execution_boundary(ui);
            return;
        };
        card().show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                key_value(ui,"LEVERAGE AKUN",&format!("{}×",data.current_leverage));
                key_value(ui,"MARGIN AKUN",if data.isolated{"ISOLATED"}else{"CROSSED"});
                key_value(ui,"SALDO TERSEDIA",&self.private_value(data.context.available));
                key_value(ui,"TAKER FEE",&format!("{} bps",data.context.taker_rate*D::from(10000)));
            });
            ui.label(format!("Mark {} · Bid {} · Ask {} · {} tier leverage",data.context.mark,data.context.quote.bid,data.context.quote.ask,data.context.brackets.tiers.len()));
            if let Some(at)=self.futures.received{ui.label(RichText::new(format!("Pembacaan dimulai {:.0} detik lalu · GET berurutan, bukan snapshot atomik",at.elapsed().as_secs_f32())).size(11.0).color(MUTED));}
            if data.blockers.is_empty(){ui.label("Pemeriksaan awal mode/posisi/order lolos. Izin TRADE key belum diuji dengan transaksi.");}
            for blocker in &data.blockers{ui.colored_label(GOLD,*blocker);}
        });
        ui.add_space(12.0);
        ui.collapsing("Kalkulator contoh satu posisi (opsional)",|ui|{
        card().show(ui,|ui|{
            ui.strong("Kalkulator satu posisi — contoh manual");
            ui.label(RichText::new("Long/Short dan SL/TP di sini hanya untuk contoh hitungan. Pada mesin otomatis, strategi yang menentukan arah serta level SL/TP.").color(MUTED).size(12.0));
            ui.label(RichText::new("Angka di sini tidak mengubah leverage atau memakai saldo. Mode awal yang didukung: One-way, Single-asset USDT, Isolated.").color(MUTED).size(12.0));
            let mut changed=false;
            ui.horizontal_wrapped(|ui|{
                ui.label("Arah contoh");
                changed|=ui.selectable_value(&mut self.futures_form.side,Side::Long,"Long").changed();
                changed|=ui.selectable_value(&mut self.futures_form.side,Side::Short,"Short").changed();
                ui.label("Leverage target");changed|=ui.add(egui::DragValue::new(&mut self.futures_form.leverage).range(1..=125).suffix("×")).changed();
            });
            egui::Grid::new("margin-inputs").num_columns(2).spacing([18.0,8.0]).show(ui,|ui|{
                for (label,value) in [("Budget margin posisi USDT",&mut self.futures_form.margin),("Risiko posisi sampai stop USDT",&mut self.futures_form.risk),
                    ("Notional maksimum USDT",&mut self.futures_form.notional),("Harga stop (mark price)",&mut self.futures_form.stop),
                    ("Harga take-profit",&mut self.futures_form.target),("Cadangan funding USDT",&mut self.futures_form.funding)] {
                    ui.label(label);changed|=ui.add(egui::TextEdit::singleline(value).desired_width(180.0).char_limit(24)).changed();ui.end_row();
                }
            });
            if changed{self.futures_form.plan=None;}
            let r=ui.add_enabled(self.futures.fresh(),egui::Button::new("Hitung margin & risiko"));self.widgets.insert("futures-calculate",r.rect);
            if r.clicked(){
                let result=self.futures_form.parse().and_then(|(b,sl,tp)|margin::plan(&data.context,&b,self.futures_form.side,sl,tp));
                match result{Ok(plan)=>{self.futures_form.plan=Some(plan);self.futures_form.error=None;},Err(e)=>{self.futures_form.plan=None;self.futures_form.error=Some(e.to_string());}}
            }
            ui.label(RichText::new("Kalkulasi memakai snapshot di atas: slippage asumsi 1 bp dan jarak minimum stop dari estimasi likuidasi 10 bps. Perbarui sebelum menilai ulang.").size(11.0).color(MUTED));
        });
        if let Some(p) = &self.futures_form.plan {
            ui.add_space(12.0);
            card().show(ui,|ui|{
                ui.strong("Hasil perkiraan isolated");
                ui.label(format!("Jumlah BTC {} · Notional {:.4} · Initial margin {:.4} USDT",p.qty,p.notional,p.initial_margin));
                ui.label(format!("Maintenance margin {:.4} · Cadangan fee {:.4} · Funding {:.4}",p.maintenance_margin,p.fee_reserve,p.funding_reserve));
                ui.label(format!("Saldo dibutuhkan {:.4} · Kerugian model sampai stop {:.4} USDT",p.required_balance,p.budgeted_loss));
                ui.label(format!("Estimasi likuidasi {:.2} · Stop {} · TP {}",p.liquidation_estimate,p.stop,p.take_profit));
                ui.label(RichText::new("Likuidasi adalah estimasi: funding, fee, perubahan tier/margin dan gap dapat mengubah hasil. Batas rugi bukan jaminan harga eksekusi.").size(12.0).color(GOLD));
            });
        }
        });
        self.execution_boundary(ui);
    }
    fn execution_boundary(&self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        card().show(ui,|ui|{
            ui.strong("Cara sesi berjalan");
            ui.label("Setelah aktivasi, mesin menyesuaikan isolated/leverage, menunggu candle baru, memilih arah dan mengelola entry serta SL/TP di Binance. Hasil dihitung dari fill, komisi dan funding.");
            ui.label("Hentikan menutup posisi milik sesi lalu memeriksa hasilnya. Setelah crash, gunakan pemulihan; aplikasi tidak membuka posisi baru otomatis saat dijalankan ulang.");
            ui.label(RichText::new("Izin TRADE dan penerimaan order nyata belum terbukti melalui probe GET. Fitur awal hanya satu posisi BTCUSDT; fee BNB dan aktivitas akun lain menghentikan sesi.").color(MUTED));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn margin_form_requires_explicit_budgets_and_keeps_leverage_separate() {
        let mut form = Form::default();
        assert!(form.parse().is_err());
        form.margin = "50".into();
        form.risk = "10".into();
        form.notional = "1000".into();
        form.leverage = 10;
        form.stop = "98".into();
        form.target = "104".into();
        let (budget, stop, tp) = form.parse().unwrap();
        let plan = margin::plan(
            &crate::futures::tests::context(),
            &budget,
            Side::Long,
            stop,
            tp,
        )
        .unwrap();
        assert!(plan.notional > budget.margin && plan.required_balance <= budget.margin);
        form.risk = "nan".into();
        assert!(form.parse().is_err());
    }
}
