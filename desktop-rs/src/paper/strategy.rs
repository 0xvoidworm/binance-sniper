use anyhow::{Context, Result, ensure};
use binance_engine::Side;
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const INTERVAL: u64 = 300_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bar {
    pub open_ms: u64,
    pub close_ms: u64,
    pub open: D,
    pub high: D,
    pub low: D,
    pub close: D,
    pub volume: D,
}
pub fn decimal(value: &Value) -> Result<D> {
    let n: D = value
        .as_str()
        .context("Desimal publik harus string")?
        .parse()?;
    ensure!(
        n.abs() <= D::from(1_000_000_000_000_u64) && n.scale() <= 12,
        "Desimal publik di luar batas"
    );
    Ok(n)
}
pub fn bars(value: &Value, now: u64) -> Result<Vec<Bar>> {
    let rows = value.as_array().context("Candle bukan array")?;
    ensure!(rows.len() <= 240, "Terlalu banyak candle");
    let mut result: Vec<Bar> = Vec::new();
    for row in rows {
        let fields = row.as_array().context("Candle bukan array")?;
        ensure!(fields.len() >= 7, "Candle tidak lengkap");
        let bar = Bar {
            open_ms: fields[0].as_u64().context("Waktu candle invalid")?,
            close_ms: fields[6].as_u64().context("Waktu candle invalid")?,
            open: decimal(&fields[1])?,
            high: decimal(&fields[2])?,
            low: decimal(&fields[3])?,
            close: decimal(&fields[4])?,
            volume: decimal(&fields[5])?,
        };
        ensure!(
            bar.open_ms.is_multiple_of(INTERVAL)
                && bar.open_ms.checked_add(INTERVAL - 1) == Some(bar.close_ms),
            "Batas candle invalid"
        );
        ensure!(
            bar.low > D::ZERO
                && bar.low <= bar.open
                && bar.low <= bar.close
                && bar.high >= bar.open
                && bar.high >= bar.close
                && bar.volume >= D::ZERO,
            "OHLCV invalid"
        );
        if let Some(previous) = result.last() {
            ensure!(
                previous.open_ms + INTERVAL == bar.open_ms,
                "Candle gap/duplikat"
            );
        }
        if bar.close_ms >= now {
            break;
        }
        result.push(bar);
    }
    ensure!(result.len() >= 60, "Warmup kurang dari 60 bar selesai");
    Ok(result)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub bar_ms: u64,
    pub fast: D,
    pub slow: D,
    pub atr: D,
    pub side: Option<Side>,
}
fn ema(bars: &[Bar], period: usize) -> D {
    let mut value = bars[..period].iter().map(|b| b.close).sum::<D>() / D::from(period);
    let alpha = D::from(2) / D::from(period + 1);
    for bar in &bars[period..] {
        value += alpha * (bar.close - value);
    }
    value
}
pub fn signal(bars: &[Bar]) -> Result<Signal> {
    ensure!(bars.len() >= 60, "Warmup belum cukup");
    let fast = ema(bars, 20);
    let slow = ema(bars, 50);
    let ranges: Vec<D> = bars
        .windows(2)
        .map(|p| {
            (p[1].high - p[1].low)
                .max((p[1].high - p[0].close).abs())
                .max((p[1].low - p[0].close).abs())
        })
        .collect();
    let mut atr = ranges[..14].iter().copied().sum::<D>() / D::from(14);
    for range in &ranges[14..] {
        atr = (atr * D::from(13) + range) / D::from(14);
    }
    let last = &bars[bars.len() - 1];
    let previous = &bars[bars.len() - 2];
    let side = if fast > slow && last.close > fast && last.close > previous.high {
        Some(Side::Long)
    } else if fast < slow && last.close < fast && last.close < previous.low {
        Some(Side::Short)
    } else {
        None
    };
    Ok(Signal {
        bar_ms: last.close_ms,
        fast,
        slow,
        atr,
        side,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    pub fn history(count: usize, descending: bool) -> Vec<Bar> {
        (0..count)
            .map(|i| {
                let price = if descending {
                    D::from(1000 - i)
                } else {
                    D::from(100 + i)
                };
                Bar {
                    open_ms: i as u64 * INTERVAL,
                    close_ms: (i as u64 + 1) * INTERVAL - 1,
                    open: price,
                    high: price + D::new(5, 1),
                    low: price - D::new(5, 1),
                    close: price,
                    volume: D::ONE,
                }
            })
            .collect()
    }
    #[test]
    fn ema_atr_long_short_and_flat_are_causal() {
        for (descending, side) in [(false, Side::Long), (true, Side::Short)] {
            let bars = history(100, descending);
            let s = signal(&bars).unwrap();
            assert_eq!(s.side, Some(side));
            assert_eq!(s.atr, D::new(15, 1));
            let before = signal(&bars[..90]).unwrap();
            let mut future = bars.clone();
            future[99].close = D::from(1_000_000);
            assert_eq!(signal(&future[..90]).unwrap().fast, before.fast);
        }
        let mut flat = history(100, false);
        for b in &mut flat {
            b.open = D::from(100);
            b.close = b.open;
            b.high = b.open + D::ONE;
            b.low = b.open - D::ONE;
        }
        assert!(signal(&flat).unwrap().side.is_none());
    }
    #[test]
    fn parser_excludes_forming_bar_and_rejects_gaps() {
        let input = history(70, false);
        let mut rows: Vec<Value> = input
            .iter()
            .map(|b| {
                serde_json::json!([
                    b.open_ms,
                    b.open.to_string(),
                    b.high.to_string(),
                    b.low.to_string(),
                    b.close.to_string(),
                    b.volume.to_string(),
                    b.close_ms
                ])
            })
            .collect();
        let parsed = bars(&serde_json::json!(rows), input[69].open_ms + 1000).unwrap();
        assert_eq!(parsed.len(), 69);
        rows.remove(5);
        assert!(bars(&serde_json::json!(rows), u64::MAX).is_err());
    }
}
