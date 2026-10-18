use super::*;
use crate::market::{Candle, Market, Timeframe};

#[derive(Debug)]
pub(super) struct ChartView {
    pub(super) visible: usize,
    pub(super) offset: usize,
    drag: f32,
}
impl Default for ChartView {
    fn default() -> Self {
        Self {
            visible: 80,
            offset: 0,
            drag: 0.0,
        }
    }
}
impl ChartView {
    fn zoom(&mut self, factor: f32) {
        self.visible = ((self.visible as f32 * factor).round() as usize).clamp(20, 240);
    }
    fn range(&mut self, len: usize) -> std::ops::Range<usize> {
        self.offset = self.offset.min(len.saturating_sub(self.visible));
        let end = len.saturating_sub(self.offset);
        end.saturating_sub(self.visible)..end
    }
}

impl Desk {
    pub fn start_feed(&mut self) -> anyhow::Result<()> {
        self.feed = Some(Feed::start(self.market_request)?);
        Ok(())
    }
    fn select_market(&mut self) {
        self.market_request.generation += 1;
        if self.market.request.market == self.market_request.market
            && self.market.request.timeframe == self.market_request.timeframe
        {
            if self.market.request.paused && !self.market_request.paused {
                self.market.received = None;
            }
            self.market.request = self.market_request;
        } else {
            self.market = MarketSnapshot::empty(self.market_request);
            self.chart_view = ChartView::default();
        }
        if let Some(feed) = &self.feed {
            feed.request(self.market_request);
        }
    }
    pub(super) fn market_workspace(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ruang kerja").size(29.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let r = primary(ui, "Buka simulasi mesin");
                self.widgets.insert("open-demo", r.rect);
                if r.clicked() {
                    self.page = Page::Simulation;
                }
            });
        });
        ui.label(
            RichText::new("Pasar Binance, posisi, dan kendali risiko.")
                .color(MUTED)
                .size(14.0),
        );
        ui.add_space(12.0);
        let wallet = self.account.data.as_ref().map_or_else(
            || "—".into(),
            |data| self.private_value(data.balances.total_wallet_balance),
        );
        let pnl = self.account.data.as_ref().map_or_else(
            || "—".into(),
            |data| self.private_value(data.balances.total_unrealized_profit),
        );
        let caption = self.account.data.as_ref().map_or_else(
            || "Belum dibaca dari Binance".into(),
            |data| {
                format!(
                    "{} · {}",
                    data.unit(),
                    if self.account.fresh() {
                        "Binance"
                    } else {
                        "Data terakhir"
                    }
                )
            },
        );
        ui.columns(3, |columns| {
            metric(&mut columns[0], "WALLET FUTURES", &wallet, &caption, TEXT);
            metric(&mut columns[1], "PNL MENGAMBANG", &pnl, &caption, TEXT);
            metric(
                &mut columns[2],
                "STATUS MESIN",
                "Observasi",
                "Eksekusi nyata belum aktif",
                GREEN,
            );
        });
        ui.add_space(14.0);
        card().show(ui, |ui| {
            let before = self.market_request;
            ui.horizontal_wrapped(|ui| {
                for market in [Market::Spot, Market::Futures] {
                    let r = ui.selectable_value(
                        &mut self.market_request.market,
                        market,
                        market.label(),
                    );
                    self.widgets.insert(
                        if market == Market::Spot {
                            "market-spot"
                        } else {
                            "market-futures"
                        },
                        r.rect,
                    );
                }
                ui.separator();
                for timeframe in Timeframe::ALL {
                    let r = ui.selectable_value(
                        &mut self.market_request.timeframe,
                        timeframe,
                        timeframe.label(),
                    );
                    self.widgets.insert(
                        match timeframe {
                            Timeframe::M1 => "tf-1m",
                            Timeframe::M5 => "tf-5m",
                            Timeframe::M15 => "tf-15m",
                            Timeframe::H1 => "tf-1h",
                        },
                        r.rect,
                    );
                }
                ui.separator();
                let r = ui.add_enabled(
                    !self.network_disabled,
                    egui::Button::new(if self.network_disabled {
                        "Feed offline"
                    } else if self.market_request.paused {
                        "Lanjut feed"
                    } else {
                        "Jeda feed"
                    }),
                );
                self.widgets.insert("pause-feed", r.rect);
                if r.clicked() {
                    self.market_request.paused = !self.market_request.paused;
                }
            });
            if self.market_request != before {
                self.select_market();
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(if self.instrument == 0 {
                        "BTC / USDT"
                    } else {
                        "ETH / USDT"
                    })
                    .strong()
                    .size(20.0),
                );
                if let Some(c) = self.market.candles[self.instrument].last() {
                    ui.label(
                        RichText::new(format!("{:.2}", c.close))
                            .monospace()
                            .size(22.0)
                            .color(if self.market.fresh() { GREEN } else { MUTED }),
                    );
                }
                badge(
                    ui,
                    self.market.status(),
                    if self.market.fresh() { GREEN } else { GOLD },
                );
            });
            let detail = self.market.age().map_or_else(
                || "Mengambil riwayat candle publik...".into(),
                |age| {
                    format!(
                        "Pembaruan {} UTC · {:.1} dtk lalu · {} candle · volume {}",
                        stamp(self.market.server_ms, "%H:%M:%S"),
                        age.as_secs_f32(),
                        self.market.candles[self.instrument].len(),
                        if self.instrument == 0 { "BTC" } else { "ETH" }
                    )
                },
            );
            ui.label(RichText::new(detail).color(MUTED).size(11.0));
            if let Some(error) = &self.market.error {
                ui.label(
                    RichText::new(format!("{error}. Mencoba tersambung kembali otomatis."))
                        .color(GOLD)
                        .size(12.0),
                );
            }
            ui.horizontal(|ui| {
                for (id, label, factor) in [("zoom-in", "+", 0.8), ("zoom-out", "−", 1.25)] {
                    let r = ui.small_button(label);
                    self.widgets.insert(id, r.rect);
                    if r.clicked() {
                        self.chart_view.zoom(factor);
                    }
                }
                let r = ui.small_button("Ke terbaru");
                self.widgets.insert("chart-reset", r.rect);
                if r.clicked() {
                    self.chart_view = ChartView::default();
                }
                ui.label(
                    RichText::new("Scroll: zoom · Geser: riwayat · Waktu UTC")
                        .size(11.0)
                        .color(MUTED),
                );
            });
            let height = (ui.ctx().content_rect().height() - 600.0).clamp(180.0, 340.0);
            let response = candle_chart(
                ui,
                &self.market.candles[self.instrument],
                &mut self.chart_view,
                height,
                self.prefs.show_grid,
                self.market.fresh(),
            );
            self.widgets.insert("market-chart", response.rect);
            ui.label(
                RichText::new(
                    "Candle terakhir masih dapat berubah. Harga berasal dari pasar publik Binance.",
                )
                .size(11.0)
                .color(MUTED),
            );
        });
    }
}

fn stamp(ms: i64, format: &str) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map_or_else(|| "—".into(), |t| t.format(format).to_string())
}

fn candle_chart(
    ui: &mut egui::Ui,
    candles: &[Candle],
    view: &mut ChartView,
    height: f32,
    show_grid: bool,
    fresh: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let area = Rect::from_min_max(rect.min + vec2(5.0, 9.0), rect.max - vec2(78.0, 24.0));
    let price_area = Rect::from_min_max(area.min, pos2(area.right(), area.bottom() - 62.0));
    let volume_area = Rect::from_min_max(pos2(area.left(), area.bottom() - 48.0), area.max);
    grid(&painter, price_area, show_grid);
    if candles.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Menunggu candle dari Binance...",
            FontId::proportional(18.0),
            MUTED,
        );
        return response;
    }
    if response.hovered() {
        let scroll = ui.input_mut(|i| {
            let delta = i.smooth_scroll_delta.y;
            i.smooth_scroll_delta.y = 0.0;
            delta
        });
        if scroll.abs() > 0.1 {
            view.zoom((-scroll * 0.004).exp());
        }
    }
    if response.dragged() {
        view.drag += response.drag_delta().x;
        let pixels_per_bar = area.width() / view.visible as f32;
        let bars = (view.drag / pixels_per_bar).trunc() as isize;
        if bars != 0 {
            view.offset = view.offset.saturating_add_signed(bars);
            view.drag -= bars as f32 * pixels_per_bar;
        }
    } else {
        view.drag = 0.0;
    }
    let range = view.range(candles.len());
    let series = &candles[range.clone()];
    let low = series.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let high = series
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let padding = ((high - low) * 0.08).max(high * 0.00001);
    let lower = low - padding;
    let upper = high + padding;
    let y = |price: f64| {
        price_area.bottom() - ((price - lower) / (upper - lower)) as f32 * price_area.height()
    };
    let width = area.width() / series.len() as f32;
    let max_volume = series
        .iter()
        .map(|c| c.volume)
        .fold(0.0, f64::max)
        .max(0.000001);
    for (i, c) in series.iter().enumerate() {
        let x = area.left() + (i as f32 + 0.5) * width;
        let color = if c.close >= c.open { GREEN } else { RED }.gamma_multiply(if fresh {
            1.0
        } else {
            0.55
        });
        painter.line_segment(
            [pos2(x, y(c.high)), pos2(x, y(c.low))],
            Stroke::new(1.0, color),
        );
        let body = Rect::from_min_max(
            pos2(x - width * 0.32, y(c.open.max(c.close))),
            pos2(
                x + width * 0.32,
                y(c.open.min(c.close)).max(y(c.open.max(c.close)) + 1.0),
            ),
        );
        painter.rect_filled(body, 0, color);
        let volume = Rect::from_min_max(
            pos2(
                x - width * 0.32,
                volume_area.bottom() - (c.volume / max_volume) as f32 * volume_area.height(),
            ),
            pos2(x + width * 0.32, volume_area.bottom()),
        );
        painter.rect_filled(volume, 0, color.gamma_multiply(0.45));
    }
    for i in 0..=4 {
        let value = lower + (upper - lower) * i as f64 / 4.0;
        painter.text(
            pos2(area.right() + 8.0, y(value)),
            Align2::LEFT_CENTER,
            format!("{value:.2}"),
            FontId::monospace(10.0),
            MUTED,
        );
    }
    for i in 0..=3 {
        let index = (series.len() - 1) * i / 3;
        painter.text(
            pos2(
                area.left() + (index as f32 + 0.5) * width,
                rect.bottom() - 8.0,
            ),
            if i == 0 {
                Align2::LEFT_CENTER
            } else if i == 3 {
                Align2::RIGHT_CENTER
            } else {
                Align2::CENTER_CENTER
            },
            stamp(series[index].open_ms, "%d/%m %H:%M"),
            FontId::monospace(10.0),
            MUTED,
        );
    }
    painter.text(
        volume_area.left_top(),
        Align2::LEFT_TOP,
        "VOL",
        FontId::monospace(9.0),
        MUTED,
    );
    if let Some(position) = response.hover_pos().filter(|p| area.contains(*p)) {
        let index = (((position.x - area.left()) / width) as usize).min(series.len() - 1);
        let c = &series[index];
        let x = area.left() + (index as f32 + 0.5) * width;
        painter.line_segment(
            [pos2(x, area.top()), pos2(x, area.bottom())],
            Stroke::new(1.0, MUTED),
        );
        return response.on_hover_ui(|ui| {
            ui.label(format!("{} UTC", stamp(c.open_ms, "%Y-%m-%d %H:%M")));
            ui.monospace(format!(
                "O {:.2}  H {:.2}\nL {:.2}  C {:.2}\nVolume {:.5}",
                c.open, c.high, c.low, c.close, c.volume
            ));
        });
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_and_pan_stay_within_history() {
        let mut view = ChartView::default();
        assert_eq!(view.range(240), 160..240);
        view.offset = 50;
        assert_eq!(view.range(240), 110..190);
        view.offset = usize::MAX;
        assert_eq!(view.range(240), 0..80);
        view.zoom(0.0001);
        assert_eq!(view.visible, 20);
        view.zoom(1000.0);
        assert_eq!(view.range(10), 0..10);
        assert_eq!(view.range(0), 0..0);
    }
}
