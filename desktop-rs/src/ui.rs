use crate::market::{Feed, Request as MarketRequest, Snapshot as MarketSnapshot};
use crate::model::{Page, Preferences, Replay, effect_label};
mod account_ui;
mod futures_ui;
mod live_ui;
mod market_ui;
mod paper_ui;
use binance_engine::{OrderStatus, Protection, engine::remaining};
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Stroke,
    pos2, vec2,
};
use rust_decimal::Decimal as D;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};

pub const BG: Color32 = Color32::from_rgb(12, 17, 25);
const PANEL: Color32 = Color32::from_rgb(18, 25, 35);
const LINE: Color32 = Color32::from_rgb(38, 49, 64);
const TEXT: Color32 = Color32::from_rgb(225, 232, 242);
const MUTED: Color32 = Color32::from_rgb(132, 149, 170);
const GREEN: Color32 = Color32::from_rgb(70, 220, 174);
const GOLD: Color32 = Color32::from_rgb(239, 193, 95);
const RED: Color32 = Color32::from_rgb(243, 113, 131);

pub struct Desk {
    pub live: crate::futures::live::Snapshot,
    live_feed: Option<crate::futures::live::Feed>,
    live_form: live_ui::LiveForm,
    pub live_root: PathBuf,
    close_after_live: bool,
    pub futures: crate::futures::Snapshot,
    futures_feed: Option<crate::futures::Feed>,
    futures_form: futures_ui::Form,
    pub paper: crate::paper::Snapshot,
    paper_feed: Option<crate::paper::Feed>,
    pub paper_report: bool,
    pub account: crate::account::Snapshot,
    account_feed: Option<crate::account::Feed>,
    account_form: account_ui::AccountForm,
    pub page: Page,
    pub replay: Replay,
    pub prefs: Preferences,
    pub prefs_path: PathBuf,
    pub export_path: PathBuf,
    pub notice: Option<String>,
    pub widgets: BTreeMap<&'static str, Rect>,
    pub last_frame: Instant,
    pub capture: Option<PathBuf>,
    pub capture_requested: bool,
    pub frames: usize,
    pub capture_error: Option<String>,
    pub capture_after: Instant,
    pub market_request: MarketRequest,
    pub market: MarketSnapshot,
    pub network_disabled: bool,
    feed: Option<Feed>,
    chart_view: market_ui::ChartView,
    pub last_export: Option<PathBuf>,
    instrument: usize,
    detail_tab: usize,
    log_filter: String,
}

impl Desk {
    pub fn new(prefs_path: PathBuf, export_path: PathBuf) -> anyhow::Result<Self> {
        let (prefs, notice) = if prefs_path.exists() {
            match Preferences::load(&prefs_path) {
                Ok(p) => (p, None),
                Err(_) => (
                    Preferences::default(),
                    Some("Pengaturan lama tidak terbaca; nilai bawaan digunakan.".into()),
                ),
            }
        } else {
            (Preferences::default(), None)
        };
        Ok(Self {
            live: crate::futures::live::Snapshot::default(),
            live_feed: None,
            live_form: live_ui::LiveForm::default(),
            live_root: prefs_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("live"),
            close_after_live: false,
            futures: crate::futures::Snapshot::default(),
            futures_feed: None,
            futures_form: futures_ui::Form::default(),
            paper: crate::paper::Snapshot::default(),
            paper_feed: None,
            paper_report: false,
            account: crate::account::Snapshot::default(),
            account_feed: None,
            account_form: account_ui::AccountForm::default(),
            page: Page::Workspace,
            replay: Replay::new()?,
            prefs,
            prefs_path,
            export_path,
            notice,
            widgets: BTreeMap::new(),
            last_frame: Instant::now(),
            capture: None,
            capture_requested: false,
            frames: 0,
            capture_error: None,
            capture_after: Instant::now(),
            market_request: MarketRequest::default(),
            market: MarketSnapshot::empty(MarketRequest::default()),
            network_disabled: false,
            feed: None,
            chart_view: market_ui::ChartView::default(),
            last_export: None,
            instrument: 0,
            detail_tab: 0,
            log_filter: String::new(),
        })
    }

    pub fn configure(ctx: &egui::Context, scale: f32) {
        ctx.set_zoom_factor(scale);
        ctx.set_theme(egui::Theme::Dark);
        let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = Color32::from_rgb(34, 81, 73);
        style.visuals.selection.stroke = Stroke::new(1.0, GREEN);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(26, 37, 49);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(26, 37, 49);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(41, 56, 71);
        style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(35, 75, 65);
        style.visuals.widgets.inactive.corner_radius = 6.into();
        style.visuals.widgets.hovered.corner_radius = 6.into();
        style.spacing.item_spacing = vec2(12.0, 10.0);
        style.spacing.button_padding = vec2(14.0, 9.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
        ctx.set_style_of(egui::Theme::Dark, style);
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        if let Some(feed) = &self.live_feed {
            self.live = feed.latest();
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        if let Some(feed) = &self.futures_feed {
            self.futures = feed.latest();
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        if let Some(feed) = &self.paper_feed {
            self.paper = feed.latest();
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        if let Some(feed) = &self.account_feed {
            self.account = feed.latest();
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        if let Some(feed) = &self.feed {
            let latest = feed.latest();
            if latest.request == self.market_request {
                self.market = latest;
            }
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        self.widgets.clear();
        egui::Panel::top("header")
            .exact_size(67.0)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(22, 14)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(31.0, 31.0), Sense::hover());
                    draw_logo(ui.painter(), rect);
                    ui.label(RichText::new("BINANCE DESK").strong().size(18.0));
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Mesin trading pribadi")
                            .size(13.0)
                            .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        badge(
                            ui,
                            self.market.status(),
                            if self.market.fresh() { GREEN } else { GOLD },
                        );
                        badge(ui, "DESKTOP", GREEN);
                    });
                });
            });
        egui::Panel::bottom("footer")
            .exact_size(30.0)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(20, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(7.0, 7.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 3.0, GREEN);
                    ui.label(RichText::new("Aplikasi siap").size(11.0));
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            self.account.label(),
                            self.live.status_label()
                        ))
                        .size(11.0)
                        .color(MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(concat!("v", env!("CARGO_PKG_VERSION"), "  /  LINUX"))
                                .monospace()
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
            });
        egui::Panel::left("navigation")
            .exact_size(192.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(15, 21, 30))
                    .inner_margin(16),
            )
            .show(ui, |ui| {
                ui.add_space(12.0);
                eyebrow(ui, "WORKSPACE");
                ui.add_space(8.0);
                for (id, label, page) in [
                    ("nav-workspace", "01  Ruang kerja", Page::Workspace),
                    ("nav-account", "02  Akun Futures", Page::Account),
                    ("nav-futures", "03  Eksekusi Futures", Page::Futures),
                    ("nav-paper", "04  Mesin paper", Page::Paper),
                    ("nav-simulation", "05  Demo sintetis", Page::Simulation),
                    ("nav-journal", "06  Jurnal demo", Page::Journal),
                    ("nav-settings", "07  Pengaturan", Page::Settings),
                ] {
                    let selected = self.page == page;
                    let response = ui.add_sized(
                        [160.0, 31.0],
                        egui::Button::new(RichText::new(label).color(if selected {
                            GREEN
                        } else {
                            MUTED
                        }))
                        .fill(if selected {
                            Color32::from_rgb(24, 51, 47)
                        } else {
                            Color32::TRANSPARENT
                        })
                        .selected(selected),
                    );
                    self.widgets.insert(id, response.rect);
                    if response.clicked() {
                        self.page = page;
                    }
                }
                ui.add_space(18.0);
                ui.separator();
                ui.add_space(12.0);
                eyebrow(ui, "WATCHLIST");
                for (i, symbol) in ["BTC / USDT", "ETH / USDT"].iter().enumerate() {
                    let response = ui.add_sized(
                        [160.0, 34.0],
                        egui::Button::new(RichText::new(*symbol).monospace().size(12.0))
                            .selected(self.instrument == i && self.page == Page::Workspace),
                    );
                    self.widgets.insert(
                        if i == 0 { "symbol-btc" } else { "symbol-eth" },
                        response.rect,
                    );
                    if response.clicked() {
                        self.instrument = i;
                        self.page = Page::Workspace;
                    }
                    let price = self.market.candles[i].last().map_or_else(
                        || "—   Menunggu data".into(),
                        |c| format!("{:.2} USDT", c.close),
                    );
                    ui.label(
                        RichText::new(price)
                            .monospace()
                            .size(11.0)
                            .color(if self.market.fresh() { GREEN } else { MUTED }),
                    );
                    ui.add_space(6.0);
                }
                ui.add_space(12.0);
                if ui.available_height() >= 105.0 {
                    card().show(ui, |ui| {
                        ui.label(RichText::new("SESI FUTURES").size(10.0).color(GOLD));
                        ui.label(RichText::new("Mesin otomatis").strong());
                        ui.label(
                            RichText::new(if self.live.busy {
                                "MEMBACA / PREFLIGHT"
                            } else {
                                self.live.phase.label()
                            })
                            .size(11.0)
                            .color(MUTED),
                        );
                    });
                } else {
                    ui.label(
                        RichText::new("Pasar & Futures otomatis")
                            .size(11.0)
                            .color(MUTED),
                    );
                }
            });
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(24))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(message) = self.notice.clone() {
                            Frame::new()
                                .fill(Color32::from_rgb(38, 46, 44))
                                .inner_margin(10)
                                .corner_radius(6)
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(RichText::new(message).color(GOLD).size(12.0));
                                        if ui.small_button("Tutup").clicked() {
                                            self.notice = None;
                                        }
                                    });
                                });
                            ui.add_space(10.0);
                        }
                        match self.page {
                            Page::Workspace => self.market_workspace(ui),
                            Page::Account => self.account_page(ui),
                            Page::Futures => self.futures_page(ui),
                            Page::Paper => self.paper_page(ui),
                            Page::Simulation => self.simulation(ui),
                            Page::Journal => self.journal(ui),
                            Page::Settings => self.settings(ui),
                        }
                    });
            });
    }

    fn simulation(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Simulasi mesin",
            "Putar satu siklus order dari engine Rust yang sebenarnya.",
        );
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            badge(ui, "DATA SINTETIS", GOLD);
            ui.label(
                RichText::new(
                    "Harga, modal, dan fill contoh. Bukan transaksi atau profit Binance.",
                )
                .color(MUTED)
                .size(12.0),
            );
        });
        ui.add_space(15.0);
        let snapshot = self.replay.current().clone();
        let state = &snapshot.state;
        ui.columns(4, |columns| {
            metric(
                &mut columns[0],
                "EKUITAS DEMO",
                &money(state.equity),
                "USDT · modal contoh 1.000",
                TEXT,
            );
            metric(
                &mut columns[1],
                "P&L TEREALISASI",
                &signed(state.realized_net),
                "USDT · setelah fee contoh",
                if state.realized_net >= D::ZERO {
                    GREEN
                } else {
                    RED
                },
            );
            metric(
                &mut columns[2],
                "BATAS RUGI HARIAN",
                &money(state.config.max_daily_loss),
                "USDT · fixture pengujian",
                GOLD,
            );
            metric(
                &mut columns[3],
                "STATUS",
                stage(state),
                "Satu posisi · long",
                GREEN,
            );
        });
        ui.add_space(15.0);
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("SYNTH / USDT").size(19.0).strong());
                if let Some(q) = &state.quote {
                    ui.label(
                        RichText::new(money(q.bid))
                            .monospace()
                            .size(22.0)
                            .color(GREEN),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Jejak quote sintetis")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });
            let chart_height = (ui.ctx().viewport_rect().height() - 760.0).clamp(110.0, 230.0);
            let jump = self.chart(ui, chart_height);
            if let Some(step) = jump {
                self.replay.seek(step);
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let label = if self.replay.playing {
                    "Jeda"
                } else if self.replay.cursor == self.replay.last() {
                    "Putar ulang"
                } else {
                    "Jalankan demo"
                };
                let response = primary(ui, label);
                self.widgets.insert("play", response.rect);
                if response.clicked() {
                    self.replay.toggle();
                    self.last_frame = Instant::now();
                }
                for (id, label, action) in [
                    ("back", "Mundur", 0),
                    ("next", "Langkah berikut", 1),
                    ("reset", "Ulang dari awal", 2),
                ] {
                    let response = ui.button(label);
                    self.widgets.insert(id, response.rect);
                    if response.clicked() {
                        self.replay.seek(match action {
                            0 => self.replay.cursor.saturating_sub(1),
                            1 => self.replay.cursor + 1,
                            _ => 0,
                        });
                    }
                }
                let response = ui.button("Ekspor sesi");
                self.widgets.insert("export", response.rect);
                if response.clicked() {
                    self.export();
                }
            });
            ui.horizontal(|ui| {
                let mut cursor = self.replay.cursor;
                let response = ui.add(
                    egui::Slider::new(&mut cursor, 0..=self.replay.last())
                        .show_value(false)
                        .trailing_fill(true),
                );
                self.widgets.insert("timeline", response.rect);
                if response.changed() {
                    self.replay.seek(cursor);
                }
                ui.label(
                    RichText::new(format!(
                        "LANGKAH {:02} / {}",
                        self.replay.cursor,
                        self.replay.last()
                    ))
                    .monospace()
                    .color(MUTED)
                    .size(11.0),
                );
                ui.label(RichText::new(format!("{}×", self.prefs.speed)).color(GREEN));
            });
            ui.label(
                RichText::new(&self.replay.current().label)
                    .color(TEXT)
                    .size(13.0),
            );
        });
        ui.add_space(15.0);
        self.details(ui);
    }

    fn chart(&self, ui: &mut egui::Ui, height: f32) -> Option<usize> {
        let (outer, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
        let rect = Rect::from_min_max(outer.min + vec2(8.0, 15.0), outer.max - vec2(80.0, 25.0));
        let painter = ui.painter();
        grid(painter, rect, self.prefs.show_grid);
        let xy = |step: usize, value: f32| {
            pos2(
                rect.left() + step as f32 / self.replay.last() as f32 * rect.width(),
                rect.bottom() - (value - 97.5) / 7.5 * rect.height(),
            )
        };
        for value in [98.0, 100.0, 102.0, 104.0] {
            let y = xy(0, value).y;
            painter.text(
                pos2(rect.right() + 12.0, y),
                Align2::LEFT_CENTER,
                format!("{value:.2}"),
                FontId::monospace(11.0),
                MUTED,
            );
        }
        for (value, label, color) in [
            (98.0, "SL  98.00", RED),
            (100.1, "ENTRY  100.10", MUTED),
            (104.0, "TP  104.00", GREEN),
        ] {
            let y = xy(0, value).y;
            for part in 0..(rect.width() / 12.0) as usize {
                let x = rect.left() + part as f32 * 12.0;
                painter.line_segment(
                    [pos2(x, y), pos2((x + 5.0).min(rect.right()), y)],
                    Stroke::new(1.0, color.gamma_multiply(0.42)),
                );
            }
            painter.text(
                pos2(rect.left() + 6.0, y - 8.0),
                Align2::LEFT_BOTTOM,
                label,
                FontId::monospace(10.0),
                color,
            );
        }
        let points: Vec<_> = self
            .replay
            .prices
            .iter()
            .filter(|p| p.step <= self.replay.cursor)
            .map(|p| xy(p.step, p.price))
            .collect();
        if points.len() >= 2 {
            let mut mesh = egui::Mesh::default();
            for pair in points.windows(2) {
                let base = mesh.vertices.len() as u32;
                for point in [
                    pair[0],
                    pair[1],
                    pos2(pair[1].x, rect.bottom()),
                    pos2(pair[0].x, rect.bottom()),
                ] {
                    mesh.colored_vertex(point, Color32::from_rgba_unmultiplied(70, 220, 174, 14));
                }
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
            }
            painter.add(egui::Shape::mesh(mesh));
            painter.add(egui::Shape::line(points.clone(), Stroke::new(2.2, GREEN)));
        }
        if let Some(last) = points.last() {
            painter.circle_filled(*last, 4.0, GREEN);
        }
        if points.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Tekan Jalankan demo untuk memulai",
                FontId::proportional(16.0),
                MUTED,
            );
        }
        for step in [0, 10, 20, 30, 40, 50] {
            painter.text(
                pos2(xy(step, 97.5).x, rect.bottom() + 16.0),
                Align2::CENTER_CENTER,
                format!("{step:02}"),
                FontId::monospace(10.0),
                MUTED,
            );
        }
        if let Some(pointer) = response.hover_pos().filter(|p| rect.contains(*p)) {
            painter.line_segment(
                [pos2(pointer.x, rect.top()), pos2(pointer.x, rect.bottom())],
                Stroke::new(1.0, MUTED.gamma_multiply(0.6)),
            );
            if response.clicked() {
                return Some(
                    ((pointer.x - rect.left()) / rect.width() * self.replay.last() as f32).round()
                        as usize,
                );
            }
        }
        None
    }

    fn details(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, label) in ["Posisi", "Order & proteksi", "Risiko"].iter().enumerate() {
                    let response = ui.selectable_label(self.detail_tab == i, *label);
                    self.widgets
                        .insert(["tab-position", "tab-orders", "tab-risk"][i], response.rect);
                    if response.clicked() {
                        self.detail_tab = i;
                    }
                }
            });
            ui.separator();
            ui.add_space(6.0);
            let state = &self.replay.current().state;
            match self.detail_tab {
                0 => {
                    if let Some(trade) = &state.active {
                        let qty = remaining(trade).unwrap_or(D::ZERO);
                        egui::Grid::new("position-grid")
                            .num_columns(5)
                            .spacing([28.0, 12.0])
                            .show(ui, |ui| {
                                for h in [
                                    "INSTRUMEN",
                                    "ARAH",
                                    "QTY TERBUKA",
                                    "HARGA MASUK",
                                    "PROTEKSI",
                                ] {
                                    eyebrow(ui, h);
                                }
                                ui.end_row();
                                ui.monospace("SYNTHUSDT");
                                ui.colored_label(GREEN, "LONG");
                                ui.monospace(qty.normalize().to_string());
                                ui.monospace(if trade.entry.fill.qty > D::ZERO {
                                    money(trade.entry.fill.notional / trade.entry.fill.qty)
                                } else {
                                    "—".into()
                                });
                                ui.label(
                                    if trade.stop_status == Some(Protection::Active)
                                        && trade.take_profit_status == Some(Protection::Active)
                                    {
                                        "SL + TP aktif"
                                    } else {
                                        "Menunggu"
                                    },
                                );
                                ui.end_row();
                            });
                    } else {
                        ui.label(if state.completed_trades > 0 {
                            "Posisi sudah ditutup. Cleanup selesai."
                        } else {
                            "Belum ada posisi demo. Jalankan atau maju satu langkah."
                        });
                    }
                }
                1 => {
                    if let Some(t) = &state.active {
                        egui::Grid::new("orders-grid")
                            .num_columns(3)
                            .spacing([32.0, 8.0])
                            .show(ui, |ui| {
                                for (name, status, quantity) in [
                                    (
                                        "Entry",
                                        order_status(t.entry.status),
                                        t.entry.fill.qty.normalize().to_string(),
                                    ),
                                    ("Stop-loss", protection(t.stop_status), "98.00".into()),
                                    (
                                        "Take-profit",
                                        protection(t.take_profit_status),
                                        "104.00".into(),
                                    ),
                                    (
                                        "Exit",
                                        t.exit
                                            .as_ref()
                                            .map_or("Belum diminta", |o| order_status(o.status)),
                                        t.exit.as_ref().map_or_else(
                                            || "—".into(),
                                            |o| o.fill.qty.normalize().to_string(),
                                        ),
                                    ),
                                ] {
                                    ui.label(name);
                                    ui.colored_label(GREEN, status);
                                    ui.monospace(quantity);
                                    ui.end_row();
                                }
                            });
                    } else {
                        ui.label("Tidak ada order aktif dalam langkah demo ini.");
                    }
                }
                _ => {
                    ui.horizontal_wrapped(|ui| {
                        key_value(ui, "Risiko / transaksi", "10.00 USDT");
                        key_value(ui, "Notional maksimum", "500.00 USDT");
                        key_value(ui, "Loss harian / total", "25.00 / 50.00 USDT");
                    });
                    ui.label(
                        RichText::new("Nilai fixture; belum menjadi alokasi atau aturan akun lo.")
                            .size(12.0)
                            .color(GOLD),
                    );
                    if let Some(reason) = &state.halt_reason {
                        ui.colored_label(RED, reason);
                    }
                }
            }
        });
    }

    fn journal(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Jurnal",
            "Jejak event dan permintaan order pada langkah demo yang dipilih.",
        );
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            badge(ui, "DEMO SINTETIS", GOLD);
            ui.add(
                egui::TextEdit::singleline(&mut self.log_filter)
                    .hint_text("Cari event...")
                    .desired_width(240.0),
            );
            if ui.button("Ekspor sesi").clicked() {
                self.export();
            }
            if ui.button("Buka simulasi").clicked() {
                self.page = Page::Simulation;
            }
        });
        ui.add_space(14.0);
        if self.replay.cursor == 0 {
            card().show(ui, |ui| {
                ui.add_space(35.0);
                ui.label(RichText::new("Jurnal masih kosong").size(22.0));
                ui.label(
                    RichText::new(
                        "Jalankan simulasi untuk melihat setiap perubahan posisi dan order.",
                    )
                    .color(MUTED),
                );
                ui.add_space(35.0);
            });
            return;
        }
        let filter = self.log_filter.to_lowercase();
        card().show(ui, |ui| {
            egui::Grid::new("journal-table")
                .num_columns(3)
                .striped(true)
                .min_col_width(40.0)
                .spacing([20.0, 12.0])
                .show(ui, |ui| {
                    eyebrow(ui, "LANGKAH");
                    eyebrow(ui, "EVENT");
                    eyebrow(ui, "EFFECT MESIN");
                    ui.end_row();
                    for (index, snapshot) in self
                        .replay
                        .snapshots
                        .iter()
                        .enumerate()
                        .take(self.replay.cursor + 1)
                        .skip(1)
                        .rev()
                    {
                        let effects = snapshot
                            .effects
                            .iter()
                            .map(effect_label)
                            .collect::<Vec<_>>()
                            .join(", ");
                        if !format!("{} {effects}", snapshot.label)
                            .to_lowercase()
                            .contains(&filter)
                        {
                            continue;
                        }
                        ui.label(
                            RichText::new(format!("{index:02}"))
                                .monospace()
                                .color(MUTED),
                        );
                        ui.add_sized([380.0, 0.0], egui::Label::new(&snapshot.label).wrap());
                        ui.add(
                            egui::Label::new(
                                RichText::new(if effects.is_empty() { "—" } else { &effects })
                                    .color(GREEN)
                                    .size(12.0),
                            )
                            .wrap(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "Pengaturan",
            "Atur tampilan dan pengalaman demo di desktop lo.",
        );
        ui.add_space(20.0);
        card().show(ui, |ui| {
            ui.label(RichText::new("Tampilan").strong().size(18.0));
            ui.add_space(10.0);
            let old_scale = self.prefs.scale;
            ui.add(
                egui::Slider::new(&mut self.prefs.scale, 0.85..=1.35)
                    .text("Skala UI")
                    .step_by(0.05),
            );
            if old_scale != self.prefs.scale {
                ui.ctx().set_zoom_factor(self.prefs.scale);
            }
            ui.checkbox(&mut self.prefs.show_grid, "Tampilkan grid chart");
            ui.add_space(15.0);
            ui.label(RichText::new("Simulasi").strong().size(18.0));
            ui.add(
                egui::Slider::new(&mut self.prefs.speed, 0.5..=3.0)
                    .text("Kecepatan pemutaran")
                    .suffix("×")
                    .step_by(0.5),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let response = primary(ui, "Simpan pengaturan");
                self.widgets.insert("save-settings", response.rect);
                if response.clicked() {
                    self.notice = Some(match self.prefs.save(&self.prefs_path) {
                        Ok(()) => "Pengaturan tersimpan di perangkat ini.".into(),
                        Err(e) => format!("Gagal menyimpan pengaturan: {e}"),
                    });
                }
                if ui.button("Kembalikan bawaan").clicked() {
                    self.prefs = Preferences::default();
                    ui.ctx().set_zoom_factor(1.0);
                }
            });
        });
        ui.add_space(18.0);
        card().show(ui, |ui| {
            ui.label(RichText::new("Koneksi Binance").strong().size(18.0)); ui.add_space(8.0);
            badge(ui, self.market.status(), if self.market.fresh() { GREEN } else { GOLD });
            ui.label("Data pasar publik BTC/ETH diperbarui sekitar dua detik. Pilihan Spot/Futures di ruang kerja mengubah tampilan pasar.");
            ui.label("Koneksi akun tersedia pada halaman Akun Futures. Key/secret hanya dimasukkan di sana.");
        });
        ui.add_space(18.0);
        card().show(ui, |ui| {
            ui.label(RichText::new(concat!("Binance Desk ", env!("CARGO_PKG_VERSION"))).strong());
            ui.label(RichText::new("Aplikasi desktop pribadi untuk Linux. Rust + egui. Bukan aplikasi resmi Binance.").color(MUTED));
            ui.label(RichText::new("Ekspor demo tersimpan di Workspace/outputs.").size(12.0).color(MUTED));
        });
    }

    fn export(&mut self) {
        match self.replay.export(&self.export_path) {
            Ok(path) => {
                self.notice = Some(format!("Sesi diekspor: {}", path.display()));
                self.last_export = Some(path);
            }
            Err(e) => self.notice = Some(format!("Ekspor gagal: {e}")),
        }
    }
}

fn card() -> Frame {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(9)
        .inner_margin(16)
}
fn heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(29.0).strong());
    ui.label(RichText::new(subtitle).color(MUTED).size(14.0));
}
fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.0).color(MUTED).strong());
}
fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    Frame::new()
        .fill(color.gamma_multiply(0.10))
        .corner_radius(4)
        .inner_margin(Margin::symmetric(9, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.0).color(color).strong());
        });
}
fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).strong().color(BG)).fill(GREEN))
}
fn metric(ui: &mut egui::Ui, name: &str, value: &str, subtitle: &str, color: Color32) {
    card().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        eyebrow(ui, name);
        let size =
            (ui.available_width() / (value.chars().count().max(1) as f32 * 0.6)).clamp(15.0, 25.0);
        ui.add(egui::Label::new(RichText::new(value).size(size).color(color)).truncate())
            .on_hover_text(value);
        ui.label(RichText::new(subtitle).size(11.0).color(MUTED));
    });
}
fn key_value(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.vertical(|ui| {
        eyebrow(ui, name);
        ui.label(RichText::new(value).monospace());
    });
}
fn grid(painter: &egui::Painter, rect: Rect, enabled: bool) {
    if !enabled {
        return;
    }
    for i in 0..=6 {
        let y = rect.top() + rect.height() * i as f32 / 6.0;
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(1.0, LINE.gamma_multiply(0.55)),
        );
    }
    for i in 0..=10 {
        let x = rect.left() + rect.width() * i as f32 / 10.0;
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, LINE.gamma_multiply(0.45)),
        );
    }
}
fn draw_logo(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 7, GREEN);
    for (i, (top, bottom)) in [(0.52, 0.82), (0.29, 0.69), (0.16, 0.51)]
        .iter()
        .enumerate()
    {
        let x = rect.left() + 8.0 + i as f32 * 7.0;
        painter.line_segment(
            [
                pos2(x, rect.top() + rect.height() * (*top - 0.07)),
                pos2(x, rect.top() + rect.height() * (*bottom + 0.07)),
            ],
            Stroke::new(1.0, BG),
        );
        painter.rect_filled(
            Rect::from_min_max(
                pos2(x - 2.0, rect.top() + rect.height() * top),
                pos2(x + 2.0, rect.top() + rect.height() * bottom),
            ),
            1,
            BG,
        );
    }
}
fn money(value: D) -> String {
    format!("{:.2}", value.round_dp(2))
}
fn signed(value: D) -> String {
    format!(
        "{}{:.2}",
        if value > D::ZERO { "+" } else { "" },
        value.round_dp(2)
    )
}
fn stage(state: &binance_engine::State) -> &'static str {
    if state.halt_reason.is_some() {
        "Dihentikan"
    } else if let Some(t) = &state.active {
        if t.cleanup_pending {
            "Cleanup"
        } else if t.exit.is_some() {
            "Exit"
        } else if t.entry.uncertain {
            "Rekonsiliasi"
        } else if t.entry.fill.qty > D::ZERO {
            "Posisi terbuka"
        } else {
            "Entry"
        }
    } else if state.completed_trades > 0 {
        "Selesai"
    } else {
        "Siap"
    }
}
fn order_status(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Pending => "Menunggu",
        OrderStatus::New => "Diterima",
        OrderStatus::PartiallyFilled => "Terisi sebagian",
        OrderStatus::Filled => "Terisi penuh",
        OrderStatus::Canceled => "Dibatalkan",
        OrderStatus::Rejected => "Ditolak",
        OrderStatus::Expired => "Kedaluwarsa",
    }
}
fn protection(status: Option<Protection>) -> &'static str {
    match status {
        None => "Belum diminta",
        Some(Protection::Requested) => "Diminta",
        Some(Protection::Active) => "Aktif",
        Some(Protection::Failed) => "Gagal",
    }
}

impl eframe::App for Desk {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.replay.playing {
            if self.last_frame.elapsed() >= Duration::from_secs_f32(0.45 / self.prefs.speed) {
                self.replay.advance();
                self.last_frame = Instant::now();
            }
            ui.ctx().request_repaint_after(Duration::from_millis(40));
        }
        self.draw(ui);
        if ui.ctx().input(|i| i.viewport().close_requested())
            && (self.live.active || self.live_feed.as_ref().is_some_and(|f| f.session_pending()))
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.close_after_live {
                self.close_after_live = true;
                self.live_command(crate::futures::live::Control::Stop);
            }
        }
        if self.close_after_live && !self.live_locked() {
            self.close_after_live = false;
            if self.live.phase == crate::futures::session_store::Phase::Finished {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.frames += 1;
        if self.capture.is_some() {
            for event in ui.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(path) = &self.capture
                        && let Err(e) = crate::save_png(path, &image)
                    {
                        self.capture_error = Some(e.to_string());
                        eprintln!("capture error: {e}");
                    }
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            if self.frames >= 6 && Instant::now() >= self.capture_after && !self.capture_requested {
                self.capture_requested = true;
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Harness {
        desk: Desk,
        ctx: egui::Context,
        _temp: tempfile::TempDir,
    }
    impl Harness {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let desk = Desk::new(
                temp.path().join("settings.json"),
                temp.path().join("exports"),
            )
            .unwrap();
            let ctx = egui::Context::default();
            Desk::configure(&ctx, 1.0);
            let mut h = Self {
                desk,
                ctx,
                _temp: temp,
            };
            h.frame(vec![], vec2(1400.0, 900.0));
            h
        }
        fn frame(&mut self, events: Vec<egui::Event>, size: egui::Vec2) {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                events,
                ..Default::default()
            };
            let output = self.ctx.run_ui(input, |ui| self.desk.draw(ui));
            assert!(output.shapes.len() > 20);
            output.drop_without_applying_deltas();
        }
        fn click(&mut self, id: &str) {
            let pos = self
                .desk
                .widgets
                .get(id)
                .unwrap_or_else(|| panic!("widget missing: {id}"))
                .center();
            assert!(
                pos.x < 1400.0 && pos.y < 870.0,
                "Widget di luar viewport: {id} {pos:?}"
            );
            for pressed in [true, false] {
                self.frame(
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    vec2(1400.0, 900.0),
                );
            }
            self.frame(vec![], vec2(1400.0, 900.0));
        }
    }
    use egui::Pos2;

    #[test]
    fn pointer_navigation_and_replay_buttons_control_real_state() {
        let mut h = Harness::new();
        h.click("open-demo");
        assert_eq!(h.desk.page, Page::Simulation);
        h.click("play");
        assert!(h.desk.replay.playing);
        h.click("play");
        assert!(!h.desk.replay.playing);
        h.click("next");
        assert_eq!(h.desk.replay.cursor, 1);
        h.click("next");
        assert!(h.desk.replay.current().state.active.is_some());
        h.click("back");
        assert_eq!(h.desk.replay.cursor, 1);
        h.click("reset");
        assert_eq!(h.desk.replay.cursor, 0);
        h.click("nav-workspace");
        h.click("symbol-eth");
        assert_eq!(h.desk.instrument, 1);
    }

    #[test]
    fn pointer_export_and_settings_save_create_verified_files() {
        let mut h = Harness::new();
        h.click("nav-simulation");
        h.desk.replay.seek(7);
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("export");
        let path = h.desk.last_export.as_ref().unwrap().join("journal.jsonl");
        assert_eq!(
            binance_engine::Journal::open(&path).unwrap().state(),
            &h.desk.replay.current().state
        );
        h.desk.notice = None;
        h.click("nav-settings");
        h.click("save-settings");
        assert!(Preferences::load(&h.desk.prefs_path).is_ok());
    }

    #[test]
    fn market_controls_clear_previous_instrument_context_and_control_chart() {
        let mut h = Harness::new();
        let candles = crate::market::parse_candles(
            serde_json::json!([
                [60000, "100", "103", "99", "102", "5", 119999],
                [120000, "102", "104", "100", "101", "7", 179999]
            ]),
            crate::market::Timeframe::M1,
        )
        .unwrap();
        h.desk.market.candles = [candles.clone(), candles];
        h.desk.market.received = Some(Instant::now());
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("zoom-in");
        assert_eq!(h.desk.chart_view.visible, 64);
        h.click("zoom-out");
        assert_eq!(h.desk.chart_view.visible, 80);
        h.click("pause-feed");
        assert_eq!(h.desk.market.status(), "DIJEDA");
        assert_eq!(h.desk.market.candles[0].len(), 2);
        h.click("pause-feed");
        assert!(!h.desk.market.fresh());
        h.click("market-spot");
        assert_eq!(h.desk.market_request.market, crate::market::Market::Spot);
        assert!(h.desk.market.candles[0].is_empty());
        h.click("tf-15m");
        assert_eq!(
            h.desk.market_request.timeframe,
            crate::market::Timeframe::M15
        );
        h.click("chart-reset");
        assert_eq!(h.desk.chart_view.offset, 0);
    }

    #[test]
    fn account_page_renders_loaded_data_stale_state_and_redacts_capture() {
        let mut h = Harness::new();
        h.click("nav-account");
        h.desk.account.data = Some(crate::account::tests::example());
        h.desk.account.received = Some(Instant::now());
        h.desk.account.status = crate::account::Status::Connected;
        h.frame(vec![], vec2(1400.0, 900.0));
        assert_eq!(h.desk.private_value(D::from(123)), "123.0000");
        h.desk.capture = Some(h._temp.path().join("test.png"));
        assert_eq!(h.desk.private_value(D::from(123)), "••••");
        h.frame(vec![], vec2(1024.0, 720.0));
        h.desk.capture = None;
        h.desk.account.received = Some(Instant::now() - Duration::from_secs(31));
        h.frame(vec![], vec2(1024.0, 720.0));
        assert_eq!(h.desk.account.label(), "DATA AKUN TERLAMBAT");
        for id in [
            "account-key",
            "account-secret",
            "connect-account",
            "load-account",
            "disconnect-account",
        ] {
            assert!(
                h.desk.widgets[id].bottom() < 690.0,
                "account control outside minimum viewport: {id}"
            );
        }
    }

    #[test]
    fn dragging_market_chart_pans_history_and_reset_returns_to_latest() {
        let mut h = Harness::new();
        let series: Vec<_> = (0..240)
            .map(|i| crate::market::Candle {
                open_ms: i * 60000,
                close_ms: i * 60000 + 59999,
                open: 100.0,
                high: 103.0,
                low: 99.0,
                close: 102.0,
                volume: 5.0,
            })
            .collect();
        h.desk.market.candles = [series.clone(), series];
        h.desk.market.received = Some(Instant::now());
        h.frame(vec![], vec2(1400.0, 900.0));
        let start = h.desk.widgets["market-chart"].center();
        h.frame(
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            vec2(1400.0, 900.0),
        );
        let end = start + vec2(120.0, 0.0);
        h.frame(vec![egui::Event::PointerMoved(end)], vec2(1400.0, 900.0));
        h.frame(
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            vec2(1400.0, 900.0),
        );
        assert!(h.desk.chart_view.offset > 0);
        h.click("chart-reset");
        assert_eq!(h.desk.chart_view.offset, 0);
    }

    #[test]
    fn pages_render_at_minimum_window_without_horizontal_navigation_overflow() {
        let mut h = Harness::new();
        for page in [
            Page::Workspace,
            Page::Account,
            Page::Futures,
            Page::Paper,
            Page::Simulation,
            Page::Journal,
            Page::Settings,
        ] {
            h.desk.page = page;
            h.desk.replay.seek(35);
            h.frame(vec![], vec2(1024.0, 720.0));
            for id in [
                "nav-workspace",
                "nav-account",
                "nav-futures",
                "nav-paper",
                "nav-simulation",
                "nav-journal",
                "nav-settings",
            ] {
                let rect = h.desk.widgets[id];
                assert!(rect.left() >= 0.0 && rect.right() <= 1024.0 && rect.bottom() <= 720.0);
            }
            if page == Page::Simulation {
                for id in ["play", "next", "back", "reset", "export"] {
                    let rect = h.desk.widgets[id];
                    assert!(
                        rect.right() <= 1024.0 && rect.bottom() <= 690.0,
                        "control outside minimum viewport: {id} {rect:?}"
                    );
                }
            }
            if page == Page::Paper {
                for id in ["start-paper", "stop-paper"] {
                    let rect = h.desk.widgets[id];
                    assert!(rect.right() <= 1024.0 && rect.bottom() <= 690.0);
                }
            }
            if page == Page::Workspace {
                for id in [
                    "open-demo",
                    "market-spot",
                    "market-futures",
                    "tf-1m",
                    "tf-5m",
                    "tf-15m",
                    "tf-1h",
                    "pause-feed",
                    "zoom-in",
                    "zoom-out",
                    "chart-reset",
                ] {
                    let rect = h.desk.widgets[id];
                    assert!(
                        rect.right() <= 1024.0 && rect.bottom() <= 690.0,
                        "market control outside viewport: {id} {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn paper_stop_control_reaches_worker_and_offline_report_cannot_start() {
        let mut h = Harness::new();
        h.desk.network_disabled = true;
        h.click("nav-paper");
        h.click("start-paper");
        assert!(h.desk.paper_feed.is_none());
        assert!(h.desk.start_paper().is_err());
        h.desk.network_disabled = false;
        h.desk.paper_report = true;
        assert!(h.desk.start_paper().is_err());
        h.desk.paper_report = false;
        let snapshot = crate::paper::Snapshot {
            phase: crate::paper::Phase::Running,
            state: Some(h.desk.replay.current().state.clone()),
            ..Default::default()
        };
        let (feed, mut receiver) = crate::paper::Feed::fixture(snapshot);
        h.desk.paper_feed = Some(feed);
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("stop-paper");
        assert!(receiver.try_recv().is_ok());
        assert!(h.desk.start_paper().is_err());
    }

    #[test]
    fn futures_preflight_renders_modes_stale_data_and_masks_account_capture() {
        let mut h = Harness::new();
        h.desk.network_disabled = true;
        h.click("nav-futures");
        h.click("futures-load");
        assert!(h.desk.futures_feed.is_none());
        h.desk.futures = crate::futures::Snapshot {
            data: Some(crate::futures::Ready {
                account: crate::account::tests::example(),
                context: crate::futures::tests::context(),
                current_leverage: 10,
                isolated: true,
                blockers: vec![],
                next_funding: 1000000,
                clock: crate::futures::transport::ClockWindow::new(
                    1_000_000,
                    1_000_000,
                    Instant::now(),
                )
                .unwrap(),
            }),
            connected: true,
            received: Some(Instant::now()),
            ..Default::default()
        };
        h.frame(vec![], vec2(1400.0, 900.0));
        assert!(h.desk.widgets.contains_key("live-start"));
        assert!(!h.desk.widgets.contains_key("futures-calculate"));
        h.desk.capture = Some(h._temp.path().join("capture.png"));
        h.frame(vec![], vec2(1024.0, 720.0));
        assert!(!h.desk.widgets.contains_key("futures-calculate"));
        h.desk.capture = None;
        h.desk.futures.received = Some(Instant::now() - Duration::from_secs(31));
        assert!(!h.desk.futures.fresh());
        h.frame(vec![], vec2(1024.0, 720.0));
        for id in ["futures-load", "futures-refresh", "futures-disconnect"] {
            let rect = h.desk.widgets[id];
            assert!(rect.right() <= 1024.0);
        }
    }
    #[test]
    fn p3_live_diagnostics_render_legacy_unknown_and_hide_private_session_in_capture() {
        use crate::futures::diagnostics::{Diagnostics, Gate};
        fn text(shape: &egui::epaint::Shape, out: &mut String) {
            match shape {
                egui::epaint::Shape::Text(t) => out.push_str(t.galley.text()),
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        text(shape, out);
                    }
                }
                _ => {}
            }
        }
        let mut h = Harness::new();
        h.desk.network_disabled = true;
        h.desk.page = Page::Futures;
        h.desk.live.started = Some(1000);
        h.desk.live.completed = 1;
        h.desk.live.recovery = Some(crate::futures::recovery::State {
            episodes: 1,
            active: false,
        });
        let mut d = Diagnostics::default();
        d.record(Gate::Signal);
        d.record(Gate::Intent);
        for (diagnostics, capture) in [(Some(d), false), (None, false), (None, true)] {
            let legacy = diagnostics.is_none();
            h.desk.live.diagnostics = diagnostics;
            h.desk.capture = capture.then(|| h._temp.path().join("audit.png"));
            let mut rendered = String::new();
            let output = h.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 2000.0))),
                    ..Default::default()
                },
                |ui| h.desk.automatic_session(ui),
            );
            for shape in &output.shapes {
                text(&shape.shape, &mut rendered);
            }
            output.drop_without_applying_deltas();
            if capture {
                assert!(rendered.contains("Status akun dan sesi disamarkan"));
                assert!(!rendered.contains("Versi aplikasi berjalan:"));
                assert!(!rendered.contains("Jejak keputusan sesi"));
                assert!(!rendered.contains("Episode pemulihan data/koneksi:"));
            } else {
                assert!(rendered.contains(env!("CARGO_PKG_VERSION")));
                assert!(rendered.contains("Hermes/model AI tidak aktif"));
                assert!(rendered.contains("Episode pemulihan data/koneksi: 1/2"));
                assert_eq!(rendered.contains("Jejak keputusan sesi"), !legacy);
                assert_eq!(rendered.contains("Sesi lama tidak merekam"), legacy);
            }
            assert!(h.desk.live_feed.is_none());
        }
    }

    #[test]
    fn native_live_activation_reaches_only_fixture_once_and_stop_reaches_active_worker() {
        use crate::futures::{
            live::{Control, Feed, Snapshot},
            session_store::Phase,
        };
        let mut h = Harness::new();
        h.desk.page = Page::Futures;
        h.desk.live_form = live_ui::LiveForm::fixture();
        let (feed, mut commands, output) = Feed::fixture(Snapshot::default());
        h.desk.live_feed = Some(feed);
        h.frame(vec![], vec2(1400.0, 900.0));
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("live-start");
        assert!(commands.try_recv().is_err());
        h.click("live-ack");
        h.click("live-start");
        assert!(
            matches!(commands.try_recv().unwrap(),Control::Start(config) if config.budget.margin==D::from(50))
        );
        assert!(h.desk.live_locked());
        h.click("live-start");
        assert!(commands.try_recv().is_err());
        output.send_replace(Snapshot {
            phase: Phase::Trading,
            active: true,
            ..Default::default()
        });
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("live-stop");
        assert!(matches!(commands.try_recv().unwrap(), Control::Stop));
    }
    #[test]
    fn setup_check_requires_bounds_but_never_financial_ack_or_session_activation() {
        use crate::futures::live::{Control, Feed, Snapshot};
        let mut h = Harness::new();
        h.desk.page = Page::Futures;
        let (feed, mut commands, _output) = Feed::fixture(Snapshot::default());
        h.desk.live_feed = Some(feed);
        h.frame(vec![], vec2(1400.0, 900.0));
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("live-check");
        assert!(commands.try_recv().is_err());
        h.desk.live_form = live_ui::LiveForm::fixture();
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("live-check");
        assert!(
            matches!(commands.try_recv().unwrap(),Control::Check(c) if c.budget.risk==D::from(2))
        );
        assert!(!h.desk.live_feed.as_ref().unwrap().session_pending());
        h.click("live-start");
        assert!(commands.try_recv().is_err());
        h.desk.network_disabled = true;
        h.frame(vec![], vec2(1400.0, 900.0));
        h.click("live-check");
        assert!(commands.try_recv().is_err());
    }
    #[test]
    fn recovery_button_requires_an_unfinished_session_and_blocks_new_activation() {
        use crate::futures::{
            live::{Control, Feed, Snapshot},
            session_store::Phase,
        };
        for unfinished in [false, true] {
            let mut h = Harness::new();
            h.desk.page = Page::Futures;
            h.desk.live_form = live_ui::LiveForm::fixture();
            let (feed, mut commands, _output) = Feed::fixture(Snapshot {
                unfinished,
                phase: if unfinished {
                    Phase::Recovering
                } else {
                    Phase::Ready
                },
                ..Default::default()
            });
            h.desk.live_feed = Some(feed);
            h.frame(vec![], vec2(1400.0, 900.0));
            h.frame(vec![], vec2(1400.0, 900.0));
            h.click("live-ack");
            if unfinished {
                h.click("live-start");
                assert!(commands.try_recv().is_err());
                h.click("live-resume");
                assert!(matches!(commands.try_recv().unwrap(), Control::Resume));
            } else {
                h.click("live-resume");
                assert!(commands.try_recv().is_err());
                h.click("live-start");
                assert!(matches!(commands.try_recv().unwrap(), Control::Start(_)));
            }
        }
    }

    #[test]
    fn capture_and_offline_cannot_enqueue_live_activation() {
        use crate::futures::live::{Control, Feed, Snapshot};
        for capture in [false, true] {
            let mut h = Harness::new();
            h.desk.page = Page::Futures;
            h.desk.live_form = live_ui::LiveForm::fixture();
            let (feed, mut commands, _) = Feed::fixture(Snapshot::default());
            h.desk.live_feed = Some(feed);
            h.desk.network_disabled = !capture;
            h.desk.capture = capture.then(|| h._temp.path().join("audit.png"));
            h.frame(vec![], vec2(1400.0, 900.0));
            h.click("live-ack");
            h.click("live-start");
            h.desk.live_command(Control::Resume);
            assert!(commands.try_recv().is_err());
        }
    }
}
