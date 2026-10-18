//! Closed-bar strategy ensemble. No credentials, HTTP, or exchange mutations.
use super::{diagnostics::Gate, margin, planning};
use crate::paper::strategy::{self, Bar, INTERVAL};
use anyhow::{Context, Result, ensure};
use binance_engine::Side;
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};

pub const VERSION: &str = "ensemble-2-cost-aware";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub strategy: String,
    pub side: Side,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub bar_ms: u64,
    pub side: Option<Side>,
    pub votes: Vec<Vote>,
    pub reason: String,
    pub atr: D,
    pub stop: Option<D>,
    pub target: Option<D>,
}
pub fn resolve(votes: &[Vote]) -> Option<Side> {
    let side = votes.first()?.side;
    votes.iter().all(|v| v.side == side).then_some(side)
}
pub fn evaluate(bars: &[Bar], ctx: &margin::ContextData) -> Result<Decision> {
    evaluate_classified(bars, ctx).map(|(decision, _)| decision)
}
pub(crate) fn evaluate_classified(
    bars: &[Bar],
    ctx: &margin::ContextData,
) -> Result<(Decision, Gate)> {
    ensure!(
        (80..=240).contains(&bars.len()),
        "Strategi perlu 80–240 candle selesai"
    );
    for (i, bar) in bars.iter().enumerate() {
        ensure!(
            bar.low > D::ZERO
                && bar.high >= bar.open.max(bar.close)
                && bar.low <= bar.open.min(bar.close)
                && bar.volume >= D::ZERO
                && bar.close_ms
                    == bar
                        .open_ms
                        .checked_add(INTERVAL - 1)
                        .context("Waktu candle invalid")?
                && bar.close_ms < ctx.now,
            "Candle strategi invalid/belum selesai"
        );
        ensure!(
            i == 0 || bars[i - 1].open_ms.checked_add(INTERVAL) == Some(bar.open_ms),
            "Candle strategi gap/revisi"
        );
        for value in [bar.open, bar.high, bar.low, bar.close, bar.volume] {
            margin::number(value, true)?;
        }
    }
    let last = bars.last().context("Candle hilang")?;
    let previous = &bars[bars.len() - 2];
    let indicators = strategy::signal(bars)?;
    let prior = strategy::signal(&bars[..bars.len() - 1])?;
    let mut out = Decision {
        bar_ms: last.close_ms,
        side: None,
        votes: vec![],
        reason: "Belum ada sinyal yang memenuhi aturan".into(),
        atr: indicators.atr,
        stop: None,
        target: None,
    };
    if ctx.now - last.close_ms > 15_000 {
        out.reason = "Menunggu candle baru; sinyal lama tidak dikejar".into();
        return Ok((out, Gate::LateSignal));
    }
    let atr_bps = indicators.atr / last.close * D::from(10_000);
    if !(D::from(planning::MIN_ATR_BPS)..=D::from(planning::MAX_ATR_BPS)).contains(&atr_bps) {
        out.reason = format!(
            "Volatilitas ATR {}% di luar rentang 0.10–3.00%; menunggu kondisi sesuai",
            (atr_bps / D::from(100)).round_dp(3)
        );
        return Ok((out, Gate::Volatility));
    }
    let window = &bars[bars.len() - 21..bars.len() - 1];
    let high = window
        .iter()
        .map(|b| b.high)
        .max()
        .context("Range kosong")?;
    let low = window.iter().map(|b| b.low).min().context("Range kosong")?;
    let volume = window.iter().map(|b| b.volume).sum::<D>() / D::from(20);
    let gap = indicators.fast - indicators.slow;
    let trend = gap.abs() >= indicators.atr * D::new(35, 2);
    let range = gap.abs() <= indicators.atr * D::new(20, 2);
    let mut vote = |name: &str, side| {
        out.votes.push(Vote {
            strategy: name.into(),
            side,
        })
    };
    if trend && last.volume >= volume * D::new(11, 1) {
        if gap > D::ZERO && last.close > high {
            vote("Breakout 20 candle", Side::Long);
        }
        if gap < D::ZERO && last.close < low {
            vote("Breakout 20 candle", Side::Short);
        }
    }
    if trend {
        if gap > D::ZERO
            && previous.low <= prior.fast
            && last.close > previous.high.max(indicators.fast)
        {
            vote("Pullback tren", Side::Long);
        }
        if gap < D::ZERO
            && previous.high >= prior.fast
            && last.close < previous.low.min(indicators.fast)
        {
            vote("Pullback tren", Side::Short);
        }
    }
    if range {
        if last.low < low && last.close > low && last.close > last.open {
            vote("Pantulan range", Side::Long);
        }
        if last.high > high && last.close < high && last.close < last.open {
            vote("Pantulan range", Side::Short);
        }
    }
    out.side = resolve(&out.votes);
    if let Some(side) = out.side {
        // Unbudgeted base levels only; actual entry persists the cost-aware plan.
        let (stop, target) =
            planning::levels(ctx, side, indicators.atr, planning::BASE_TARGET_ATR)?;
        out.stop = Some(stop.normalize());
        out.target = Some(target.normalize());
        out.reason = out
            .votes
            .iter()
            .map(|v| v.strategy.as_str())
            .collect::<Vec<_>>()
            .join(" + ");
    } else if !out.votes.is_empty() {
        out.reason = "Sinyal bertentangan; menunggu".into();
    }
    let gate = if out.side.is_some() {
        Gate::Signal
    } else if out.votes.is_empty() {
        Gate::NoPattern
    } else {
        Gate::ConflictingSignal
    };
    Ok((out, gate))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn p3_decision_classification_uses_the_same_gates_as_execution() {
        let bars = strategy::tests::history(100, false);
        let c = ctx(&bars);
        let (decision, gate) = evaluate_classified(&bars, &c).unwrap();
        assert_eq!(
            serde_json::to_value(&decision).unwrap(),
            serde_json::to_value(evaluate(&bars, &c).unwrap()).unwrap()
        );
        assert_eq!(
            gate,
            if decision.side.is_some() {
                Gate::Signal
            } else {
                Gate::NoPattern
            }
        );
        let mut late = c;
        late.now += 20_000;
        assert_eq!(
            evaluate_classified(&bars, &late).unwrap().1,
            Gate::LateSignal
        );
        let mut small = bars;
        for b in &mut small {
            b.open += D::from(1_000_000);
            b.close += D::from(1_000_000);
            b.high += D::from(1_000_000);
            b.low += D::from(1_000_000);
        }
        assert_eq!(
            evaluate_classified(&small, &ctx(&small)).unwrap().1,
            Gate::Volatility
        );
    }
    fn ctx(bars: &[Bar]) -> margin::ContextData {
        let mut c = crate::futures::tests::context();
        let b = bars.last().unwrap();
        c.now = b.close_ms + 1;
        c.quote.observed_at_ms = c.now;
        c.quote.bid = b.close;
        c.quote.ask = b.close + D::new(1, 2);
        c.mark = b.close;
        c
    }
    #[test]
    fn ensemble_selects_both_directions_from_closed_bars_and_refuses_late_or_conflicting_signals() {
        for (falling, side) in [(false, Side::Long), (true, Side::Short)] {
            let mut bars = strategy::tests::history(100, falling);
            bars.last_mut().unwrap().volume = D::from(2);
            let c = ctx(&bars);
            let result = evaluate(&bars, &c).unwrap();
            assert_eq!(result.side, Some(side));
            assert!(result.stop.is_some() && result.target.is_some());
            match side {
                Side::Long => {
                    assert!(result.stop.unwrap() < c.mark && result.target.unwrap() > c.mark)
                }
                Side::Short => {
                    assert!(result.stop.unwrap() > c.mark && result.target.unwrap() < c.mark)
                }
            }
            let mut late = c.clone();
            late.now += 20_000;
            assert!(evaluate(&bars, &late).unwrap().side.is_none());
            bars.last_mut().unwrap().close_ms = c.now;
            assert!(evaluate(&bars, &c).is_err());
        }
        assert_eq!(
            resolve(&[
                Vote {
                    strategy: "a".into(),
                    side: Side::Long
                },
                Vote {
                    strategy: "b".into(),
                    side: Side::Short
                }
            ]),
            None
        );
        assert_eq!(resolve(&[]), None);
    }
    #[test]
    fn pullback_and_range_reversals_support_both_directions_without_volume_breakout() {
        for side in [Side::Long, Side::Short] {
            let mut bars = strategy::tests::history(100, false);
            let fast = strategy::signal(&bars[..99]).unwrap().fast;
            bars[98].low = fast.round_dp(6) - D::ONE;
            if side == Side::Short {
                for b in &mut bars {
                    let (old_high, old_low) = (b.high, b.low);
                    b.open = D::from(400) - b.open;
                    b.close = D::from(400) - b.close;
                    b.high = D::from(400) - old_low;
                    b.low = D::from(400) - old_high;
                }
            }
            let out = evaluate(&bars, &ctx(&bars)).unwrap();
            assert_eq!(out.side, Some(side));
            assert_eq!(out.votes.len(), 1);
            assert_eq!(out.votes[0].strategy, "Pullback tren");
            let mut bars = strategy::tests::history(100, false);
            for b in &mut bars {
                b.open = D::from(100);
                b.close = D::from(100);
                b.high = D::from(101);
                b.low = D::from(99);
            }
            let flat = evaluate(&bars, &ctx(&bars)).unwrap();
            assert!(flat.side.is_none());
            let b = bars.last_mut().unwrap();
            match side {
                Side::Long => {
                    b.low = D::from(98);
                    b.open = D::new(995, 1);
                }
                Side::Short => {
                    b.high = D::from(102);
                    b.open = D::new(1005, 1);
                }
            }
            let out = evaluate(&bars, &ctx(&bars)).unwrap();
            assert_eq!(out.side, Some(side));
            assert_eq!(out.votes[0].strategy, "Pantulan range");
        }
    }
}
