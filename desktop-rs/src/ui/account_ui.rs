use super::*;
use crate::{
    account::{Command, Status},
    vault::Credentials,
};
use zeroize::Zeroizing;

#[derive(Default)]
pub(super) struct AccountForm {
    key: Zeroizing<String>,
    secret: Zeroizing<String>,
    persist: bool,
    hide_values: bool,
    confirm_forget: bool,
    error: Option<&'static str>,
}

impl Desk {
    pub fn start_account(&mut self) -> anyhow::Result<()> {
        let feed = crate::account::Feed::start()?;
        feed.send(Command::Load);
        self.account_feed = Some(feed);
        Ok(())
    }
    fn account_command(&mut self, command: Command) {
        if self.live_locked() {
            self.account_form.error = Some("Hentikan sesi live sebelum mengganti koneksi akun");
            return;
        }
        let live_credentials = match &command {
            Command::Connect(value, _) => Some(Some(value.clone())),
            Command::Disconnect | Command::Forget | Command::Load => Some(None),
        };
        let futures_command = match &command {
            Command::Connect(value, _) => Some(crate::futures::ReadinessCommand::StoreSession(
                value.clone(),
            )),
            Command::Disconnect | Command::Forget => {
                Some(crate::futures::ReadinessCommand::Disconnect)
            }
            Command::Load if self.futures_feed.is_some() => {
                Some(crate::futures::ReadinessCommand::Load)
            }
            _ => None,
        };
        if self
            .account_feed
            .as_ref()
            .is_some_and(|feed| feed.send(command))
        {
            self.account = crate::account::Snapshot::default();
            self.account_form.error = None;
            if let Some(command) = futures_command {
                self.futures_command(command);
            }
            if let Some(credentials) = live_credentials {
                self.live_command(crate::futures::live::Control::Credentials(credentials));
            }
        } else {
            self.account_form.error = Some("Worker akun tidak tersedia; buka ulang aplikasi.");
        }
    }
    pub(super) fn private_value(&self, value: D) -> String {
        if self.account_form.hide_values || self.capture.is_some() {
            "••••".into()
        } else {
            let digits = value.normalize().scale().max(4) as usize;
            format!("{value:.digits$}")
        }
    }
    pub(super) fn account_page(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Akun Futures",
            "USD-M Binance · saldo, posisi dan konfigurasi akun.",
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            badge(
                ui,
                self.account.label(),
                if self.account.fresh() { GREEN } else { GOLD },
            );
            badge(ui, "BACA SAJA", MUTED);
            ui.checkbox(&mut self.account_form.hide_values, "Samarkan nilai akun");
        });
        if let Some(error) = self.account.error {
            ui.colored_label(GOLD, error.message());
        }
        if let Some(note) = self.account.note {
            ui.label(RichText::new(note).size(12.0).color(MUTED));
        }
        if let Some(message) = self.account_form.error {
            ui.colored_label(GOLD, message);
        }
        if let Some(time) = self.account.received {
            let at = self
                .account
                .data
                .as_ref()
                .and_then(|data| chrono::DateTime::from_timestamp_millis(data.server_ms as i64))
                .map_or_else(
                    || "—".into(),
                    |date| date.format("%H:%M:%S UTC").to_string(),
                );
            ui.label(
                RichText::new(format!(
                    "Pembaruan {at} · {:.0} detik lalu · polling 10 detik",
                    time.elapsed().as_secs_f32()
                ))
                .size(12.0)
                .color(MUTED),
            );
        }
        ui.add_space(12.0);
        let busy = self.account.status == Status::Loading;
        card().show(ui, |ui| {
            ui.label(RichText::new("Koneksi akun").strong().size(17.0));
            ui.label(RichText::new("Gunakan API HMAC Binance. Isi key dan secret langsung di perangkat ini.").color(MUTED).size(12.0));
            ui.add_enabled_ui(!busy && !self.network_disabled && !self.live_locked(), |ui| {
                ui.columns(2, |columns| {
                    for (index, id, title, buffer) in [(0, "account-key", "API key", &mut self.account_form.key),
                        (1, "account-secret", "API secret", &mut self.account_form.secret)] {
                        columns[index].label(title);
                        let mut edit = egui::TextEdit::singleline(&mut **buffer).password(true)
                            .id_salt(id).char_limit(512).desired_width(f32::INFINITY).show(&mut columns[index]);
                        // Password undo history must not retain the input after submission.
                        edit.state.clear_undoer(); edit.state.store(columns[index].ctx(), edit.response.id);
                        self.widgets.insert(id, edit.response.rect);
                    }
                });
                ui.checkbox(&mut self.account_form.persist, "Simpan di KWallet untuk pemakaian berikutnya");
                ui.horizontal_wrapped(|ui| {
                    let ready = !self.account_form.key.is_empty() && !self.account_form.secret.is_empty();
                    let r = ui.add_enabled(ready, egui::Button::new(RichText::new("Hubungkan akun").strong().color(BG)).fill(GREEN).stroke(Stroke::NONE));
                    self.widgets.insert("connect-account", r.rect);
                    if r.clicked() {
                        let key = std::mem::take(&mut *self.account_form.key);
                        let secret = std::mem::take(&mut *self.account_form.secret);
                        match Credentials::new(key, secret) {
                            Ok(value) => self.account_command(Command::Connect(value, self.account_form.persist)),
                            Err(error) => self.account_form.error = Some(error.message()),
                        }
                    }
                    let r = ui.button("Pakai dari KWallet"); self.widgets.insert("load-account", r.rect);
                    if r.clicked() { self.account_command(Command::Load); }
                    let r = ui.button("Putuskan"); self.widgets.insert("disconnect-account", r.rect);
                    if r.clicked() { self.account_command(Command::Disconnect); }
                    if ui.small_button("Hapus yang tersimpan").clicked() { self.account_form.confirm_forget = true; }
                });
            });
            if self.account_form.confirm_forget && !self.live_locked() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Hapus kredensial Binance Desk dari KWallet?");
                    if ui.button("Hapus").clicked() { self.account_command(Command::Forget); self.account_form.confirm_forget = false; }
                    if ui.button("Batal").clicked() { self.account_form.confirm_forget = false; }
                });
            }
            if self.network_disabled { ui.label("Sesi offline: koneksi akun dinonaktifkan."); }
            ui.label(RichText::new("Tanpa centang KWallet, key hanya dipakai selama sesi. Halaman ini membaca akun; aktivasi order dan leverage ada di Sesi otomatis pada Eksekusi Futures.").size(11.0).color(MUTED));
        });
        ui.add_space(14.0);
        if self.capture.is_some() && self.account.data.is_some() {
            card().show(ui, |ui| {
                ui.label("Data akun disamarkan pada tangkapan layar.");
            });
            return;
        }
        let Some(data) = self.account.data.clone() else {
            card().show(ui, |ui| {
                ui.label(
                    RichText::new("Data akun belum tersedia")
                        .strong()
                        .size(18.0),
                );
                ui.label("Saldo, posisi dan leverage akan tampil setelah pembacaan akun berhasil.");
                ui.label(
                    RichText::new("Data pasar publik tetap berjalan tanpa koneksi akun.")
                        .color(MUTED),
                );
            });
            return;
        };
        if !self.account.fresh() {
            ui.colored_label(
                GOLD,
                "Data terakhir tersimpan di memori; kondisi akun saat ini belum terverifikasi.",
            );
        }
        let color = if self.account.fresh() { TEXT } else { MUTED };
        ui.columns(3, |columns| {
            metric(
                &mut columns[0],
                "WALLET FUTURES",
                &self.private_value(data.balances.total_wallet_balance),
                data.unit(),
                color,
            );
            metric(
                &mut columns[1],
                "SALDO TERSEDIA",
                &self.private_value(data.balances.available_balance),
                data.unit(),
                color,
            );
            metric(
                &mut columns[2],
                "PNL MENGAMBANG",
                &self.private_value(data.balances.total_unrealized_profit),
                data.unit(),
                color,
            );
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                key_value(ui, "MODE POSISI", if data.config.dual_side_position { "Hedge" } else { "One-way" });
                key_value(ui, "ASET MARGIN", if data.config.multi_assets_margin { "Multi-assets" } else { "Single-asset" });
                key_value(ui, "CAN TRADE AKUN", if data.config.can_trade { "Ya (status Binance)" } else { "Tidak" });
            });
            ui.label(RichText::new("Status canTrade adalah milik akun; bukan bukti izin trading API atau aktivasi mesin.").color(MUTED).size(11.0));
            ui.horizontal_wrapped(|ui| {
                key_value(ui, "SALDO MARGIN", &format!("{} {}", self.private_value(data.balances.total_margin_balance), data.unit()));
                key_value(ui, "INITIAL MARGIN", &format!("{} {}", self.private_value(data.balances.total_initial_margin), data.unit()));
                key_value(ui, "MAINT. MARGIN", &format!("{} {}", self.private_value(data.balances.total_maint_margin), data.unit()));
            });
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.label(RichText::new("Saldo per aset").strong());
            egui::ScrollArea::horizontal()
                .id_salt("assets-scroll")
                .show(ui, |ui| {
                    egui::Grid::new("account-assets")
                        .striped(true)
                        .show(ui, |ui| {
                            for label in [
                                "Aset",
                                "Wallet",
                                "Tersedia",
                                "Saldo margin",
                                "PnL mengambang",
                            ] {
                                ui.strong(label);
                            }
                            ui.end_row();
                            for asset in &data.balances.assets {
                                ui.label(&asset.asset);
                                for value in [
                                    asset.wallet_balance,
                                    asset.available_balance,
                                    asset.margin_balance,
                                    asset.unrealized_profit,
                                ] {
                                    ui.monospace(self.private_value(value));
                                }
                                ui.end_row();
                            }
                        });
                });
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.label(RichText::new("Leverage & margin yang sedang berlaku").strong());
            let mut symbols: std::collections::BTreeSet<_> = ["BTCUSDT", "ETHUSDT"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            symbols.extend(data.positions.iter().map(|p| p.symbol.clone()));
            egui::Grid::new("account-symbols")
                .striped(true)
                .show(ui, |ui| {
                    for label in ["Simbol", "Leverage", "Margin", "Auto-add margin"] {
                        ui.strong(label);
                    }
                    ui.end_row();
                    for symbol in symbols {
                        ui.label(&symbol);
                        if let Some(config) = data.symbol(&symbol) {
                            ui.monospace(
                                if self.account_form.hide_values || self.capture.is_some() {
                                    "••••".into()
                                } else {
                                    format!("{}×", config.leverage)
                                },
                            );
                            ui.label(&config.margin_type);
                            ui.label(if config.is_auto_add_margin {
                                "Aktif"
                            } else {
                                "Nonaktif"
                            });
                        } else {
                            ui.label("—");
                            ui.label("Belum dilaporkan");
                            ui.label("—");
                        }
                        ui.end_row();
                    }
                });
        });
        ui.add_space(12.0);
        card().show(ui, |ui| {
            ui.label(RichText::new("Posisi terbuka").strong());
            if data.positions.is_empty() {
                ui.label(if self.account.fresh() {
                    "Binance melaporkan tidak ada posisi terbuka pada snapshot ini."
                } else {
                    "Snapshot terakhir tidak memuat posisi; keadaan terkini belum diketahui."
                });
            }
            for p in &data.positions {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.strong(&p.symbol);
                    ui.label(if p.position_amt < D::ZERO {
                        "Short"
                    } else {
                        "Long"
                    });
                    ui.label(format!(
                        "Mode {} · margin {}",
                        p.position_side, p.margin_asset
                    ));
                });
                ui.horizontal_wrapped(|ui| {
                    key_value(ui, "JUMLAH", &self.private_value(p.position_amt));
                    key_value(ui, "ENTRY", &self.private_value(p.entry_price));
                    key_value(ui, "MARK", &self.private_value(p.mark_price));
                    key_value(
                        ui,
                        "PNL MENGAMBANG",
                        &self.private_value(p.unrealized_profit),
                    );
                    key_value(
                        ui,
                        "LIKUIDASI",
                        &if p.liquidation_price == D::ZERO {
                            "Tidak tersedia".into()
                        } else {
                            self.private_value(p.liquidation_price)
                        },
                    );
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_fields_mask_input_and_submission_clears_buffers() {
        let temp = tempfile::tempdir().unwrap();
        let mut desk =
            Desk::new(temp.path().join("settings.json"), temp.path().join("out")).unwrap();
        desk.page = Page::Account;
        let ctx = egui::Context::default();
        Desk::configure(&ctx, 1.0);
        let frame = |desk: &mut Desk, events| {
            let result = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1400.0, 900.0))),
                    events,
                    ..Default::default()
                },
                |ui| desk.draw(ui),
            );
            let rendered = format!("{:?}", result.shapes);
            assert!(!rendered.contains("dummy_public_key_123"));
            assert!(!rendered.contains("dummy_secret_123456"));
            result.drop_without_applying_deltas();
        };
        let click = |desk: &mut Desk, id| {
            let pos = desk.widgets[id].center();
            for pressed in [true, false] {
                frame(
                    desk,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        };
        frame(&mut desk, vec![]);
        frame(&mut desk, vec![]);
        click(&mut desk, "account-key");
        frame(
            &mut desk,
            vec![egui::Event::Text("dummy_public_key_123".into())],
        );
        click(&mut desk, "account-secret");
        frame(
            &mut desk,
            vec![egui::Event::Text("dummy_secret_123456".into())],
        );
        assert_eq!(&*desk.account_form.key, "dummy_public_key_123");
        assert_eq!(&*desk.account_form.secret, "dummy_secret_123456");
        assert!(!desk.account_form.persist);
        click(&mut desk, "connect-account");
        assert!(desk.account_form.key.is_empty());
        assert!(desk.account_form.secret.is_empty());
        assert!(desk.account_form.error.is_some()); // No worker/network/keyring in this UI test.
        assert_eq!(desk.private_value(D::new(1, 8)), "0.00000001");
    }
}
