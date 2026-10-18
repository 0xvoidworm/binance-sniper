//! One bounded economic model shared by setup checks and actual entry sizing.
//! Feasibility is not a trading signal or a prediction of profit.
use super::margin::{self, Budget, ContextData, Plan};
use anyhow::{Result, ensure};
use binance_engine::{Filters, Quote, Side};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};

pub const MIN_ATR_BPS: u32 = 10;
pub const MAX_ATR_BPS: u32 = 300;
pub const BASE_TARGET_ATR: u32 = 3;
pub const MAX_TARGET_ATR: u32 = 6;

pub fn levels(ctx: &ContextData, side: Side, atr: D, target_atr: u32) -> Result<(D, D)> {
    // ATR is an intermediate average, not an exchange input: its exact decimal
    // precision can exceed the 12-place limit used for API payload numbers.
    ensure!(
        atr > D::ZERO && atr <= D::from(1_000_000_000_000_u64),
        "ATR invalid"
    );
    ensure!(
        ctx.filters.tick_size > D::ZERO
            && ctx.quote.bid > D::ZERO
            && ctx.quote.ask >= ctx.quote.bid
            && ctx.mark > D::ZERO
            && (BASE_TARGET_ATR..=MAX_TARGET_ATR).contains(&target_atr),
        "Quote/filter strategi invalid"
    );
    let tick = ctx.filters.tick_size;
    let floor = |v: D| (v / tick).floor() * tick;
    let ceil = |v: D| (v / tick).ceil() * tick;
    let stop_distance = atr * D::from(2);
    let reward_distance = atr * D::from(target_atr);
    let (stop, target) = match side {
        Side::Long => (
            floor(ctx.quote.bid.min(ctx.mark) - stop_distance),
            ceil(ctx.quote.ask.max(ctx.mark) + reward_distance),
        ),
        Side::Short => (
            ceil(ctx.quote.ask.max(ctx.mark) + stop_distance),
            floor(ctx.quote.bid.min(ctx.mark) - reward_distance),
        ),
    };
    ensure!(stop > D::ZERO && target > D::ZERO, "SL/TP otomatis invalid");
    Ok((stop.normalize(), target.normalize()))
}

pub fn net_reward(plan: &Plan, slippage_bps: D) -> D {
    let slip = slippage_bps / D::from(10_000);
    let gross = match plan.side {
        Side::Long => plan.take_profit * (D::ONE - slip) - plan.worst_entry,
        Side::Short => plan.worst_entry - plan.take_profit * (D::ONE + slip),
    };
    plan.qty * gross - plan.fee_reserve - plan.funding_reserve
}

pub fn reward_passes(plan: &Plan, slippage_bps: D) -> bool {
    net_reward(plan, slippage_bps) >= plan.budgeted_loss * D::new(11, 1)
}

pub fn entry_plan(ctx: &ContextData, budget: &Budget, side: Side, atr: D) -> Result<Plan> {
    entry_plan_inner(ctx, budget, side, atr, |_, _| {})
}

fn entry_plan_inner(
    ctx: &ContextData,
    budget: &Budget,
    side: Side,
    atr: D,
    mut observe: impl FnMut(D, &Result<Plan>),
) -> Result<Plan> {
    let (stop, mut target) = levels(ctx, side, atr, BASE_TARGET_ATR)?;
    let (_, cap) = levels(ctx, side, atr, MAX_TARGET_ATR)?;
    let tick = ctx.filters.tick_size;
    for _ in 0..8 {
        // Re-size on every target change: a farther TP can increase fee reserve
        // and reduce the permitted lot. Neither stop nor risk budget is relaxed.
        let sized = margin::plan(ctx, budget, side, stop, target);
        observe(target, &sized);
        let plan = sized?;
        if reward_passes(&plan, budget.slippage_bps) {
            return Ok(plan);
        }
        let ratio = D::new(11, 1);
        let slip = budget.slippage_bps / D::from(10_000);
        let slope = match side {
            Side::Long => D::ONE - slip - (D::ONE + ratio) * ctx.taker_rate,
            Side::Short => D::ONE + slip,
        };
        ensure!(slope > D::ZERO, "Biaya tidak memungkinkan target bersih");
        let delta = (plan.budgeted_loss * ratio - net_reward(&plan, budget.slippage_bps))
            / (plan.qty * slope);
        target = match side {
            Side::Long => (((target + delta) / tick).ceil() * tick).max(target + tick),
            Side::Short => (((target - delta) / tick).floor() * tick).min(target - tick),
        }
        .normalize();
        ensure!(
            target > D::ZERO
                && match side {
                    Side::Long => target <= cap,
                    Side::Short => target >= cap,
                },
            "Biaya/lot/risiko memerlukan TP melebihi batas 6 ATR; perlu harga TP {target}, batas {cap}, ATR {}; tidak ada entry",
            atr.round_dp(8)
        );
    }
    anyhow::bail!("Sizing biaya belum konvergen dalam batas pemeriksaan; tidak ada entry")
}

/// Whitelisted economic input, not an account dump. Available balance affects
/// sizing only through `budget.margin <= available`; retain that boolean only.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewInput {
    pub schema: u32,
    pub side: Side,
    pub atr: D,
    pub now: u64,
    pub quote: Quote,
    pub mark: D,
    pub filters: Filters,
    pub taker_rate: D,
    pub brackets: margin::Brackets,
    pub budget: Budget,
    pub margin_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attempt {
    pub target: D,
    pub plan: Option<Plan>,
    pub rejection: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryReview {
    pub input: ReviewInput,
    pub attempts: Vec<Attempt>,
    pub result: std::result::Result<Plan, String>,
}
impl EntryReview {
    pub fn summary(&self) -> String {
        match &self.result {
            Ok(plan) => format!(
                "{:?}: hitungan lolos; qty {} BTC, SL {}, TP {}, risiko {} USDT, hasil target bersih {} USDT, rasio {}",
                self.input.side,
                plan.qty,
                plan.stop,
                plan.take_profit,
                plan.budgeted_loss.round_dp(4),
                net_reward(plan, self.input.budget.slippage_bps).round_dp(4),
                (net_reward(plan, self.input.budget.slippage_bps) / plan.budgeted_loss).round_dp(3)
            ),
            Err(error) => format!("{:?}: tertahan — {error}", self.input.side),
        }
    }
    pub fn log(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            eprintln!("LIVE_ENTRY_REVIEW; {json}");
        }
    }
    pub fn into_plan(self) -> Result<Plan> {
        self.result.map_err(anyhow::Error::msg)
    }
}

/// Shares the exact sizing path with execution and the hypothetical ATR grid.
/// At most eight sizing attempts; no credentials, wallet or private responses.
pub fn review(ctx: &ContextData, budget: &Budget, side: Side, atr: D) -> EntryReview {
    let mut attempts = Vec::new();
    let result = entry_plan_inner(ctx, budget, side, atr, |target, sized| {
        attempts.push(Attempt {
            target,
            plan: sized.as_ref().ok().cloned(),
            rejection: sized.as_ref().err().map(ToString::to_string),
        });
    })
    .map_err(|error| error.to_string());
    EntryReview {
        input: ReviewInput {
            schema: 1,
            side,
            atr,
            now: ctx.now,
            quote: ctx.quote.clone(),
            mark: ctx.mark,
            filters: ctx.filters.clone(),
            taker_rate: ctx.taker_rate,
            brackets: ctx.brackets.clone(),
            budget: budget.clone(),
            margin_available: budget.margin <= ctx.available,
        },
        attempts,
        result,
    }
}

#[derive(Clone, Debug)]
pub struct Example {
    pub atr_percent: D,
    pub plan: Plan,
    pub reward: D,
}

#[derive(Clone, Debug)]
pub struct Assessment {
    pub long: Option<Example>,
    pub short: Option<Example>,
    pub checked: u32,
    pub last_rejection: String,
    pub effective_budget: Budget,
}
impl Assessment {
    pub fn viable(&self) -> bool {
        self.long.is_some() || self.short.is_some()
    }
    pub fn summary(&self) -> String {
        let side = |sample: &Option<Example>| {
            sample.as_ref().map_or("tidak ditemukan".into(), |e| {
                format!(
                    "contoh ATR {}%, qty {} BTC, rasio bersih {}",
                    e.atr_percent.normalize(),
                    e.plan.qty,
                    (e.reward / e.plan.budgeted_loss).round_dp(3)
                )
            })
        };
        if self.viable() {
            format!(
                "LAYAK BERSYARAT pada snapshot ini. Long: {}; Short: {}. Ini contoh hitungan, bukan sinyal/profit; aturan pola dan data tetap wajib lolos.",
                side(&self.long),
                side(&self.short)
            )
        } else {
            format!(
                "SETUP TERTAHAN: tidak ditemukan ukuran layak pada grid ATR 0.10–3.00% (langkah 0.0001%). {}. Bukan izin menaikkan risiko otomatis.",
                self.last_rejection
            )
        }
    }
}

/// Searches only for a mathematical witness, never inventing a market signal.
/// Failure is conservative: the finite grid is not a proof about all future prices.
pub fn assess(ctx: &ContextData, budget: &Budget, anchor: D) -> Result<Assessment> {
    margin::number(anchor, false)?;
    let mut result = Assessment {
        long: None,
        short: None,
        checked: 0,
        last_rejection: String::new(),
        effective_budget: budget.clone(),
    };
    for side in [Side::Long, Side::Short] {
        for millionths in (MIN_ATR_BPS * 100)..=(MAX_ATR_BPS * 100) {
            result.checked += 1;
            let fraction = D::new(i64::from(millionths), 6);
            match entry_plan(ctx, budget, side, anchor * fraction) {
                Ok(plan) => {
                    let sample = Some(Example {
                        atr_percent: fraction * D::from(100),
                        reward: net_reward(&plan, budget.slippage_bps),
                        plan,
                    });
                    match side {
                        Side::Long => result.long = sample,
                        Side::Short => result.short = sample,
                    }
                    break;
                }
                Err(error) => result.last_rejection = error.to_string(),
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (ContextData, Budget) {
        let mut ctx = crate::futures::tests::context();
        ctx.quote.bid = D::from(81275);
        ctx.quote.ask = "81275.10".parse().unwrap();
        ctx.mark = "81275.04751124".parse().unwrap();
        ctx.taker_rate = D::new(5, 4);
        ctx.filters.min_notional = D::from(50);
        ctx.filters.tick_size = D::new(1, 1);
        (
            ctx,
            Budget {
                margin: D::from(100),
                risk: D::ONE,
                max_notional: D::from(200),
                leverage: 5,
                slippage_bps: D::ONE,
                liquidation_buffer_bps: D::from(10),
                funding_reserve: D::new(1, 1),
            },
        )
    }

    #[test]
    fn recorded_economics_replay_the_same_result_without_recording_wallet() {
        let (mut ctx, budget) = fixture();
        ctx.available = "9876.54321".parse().unwrap();
        for side in [Side::Long, Side::Short] {
            for atr in [D::from(10), D::from(90), D::from(150), D::from(300)] {
                let review = review(&ctx, &budget, side, atr);
                let json = serde_json::to_string(&review).unwrap();
                assert!(!json.contains("9876.54321"));
                for forbidden in ["wallet", "credential", "api_key", "secret", "signature"] {
                    assert!(!json.contains(forbidden));
                }
                assert!(review.attempts.len() <= 8);
                assert!(json.len() < 32_768);
                let recorded: EntryReview = serde_json::from_str(&json).unwrap();
                let i = recorded.input;
                let replay = entry_plan(
                    &ContextData {
                        filters: i.filters,
                        quote: i.quote,
                        mark: i.mark,
                        now: i.now,
                        available: if i.margin_available {
                            i.budget.margin
                        } else {
                            D::ZERO
                        },
                        taker_rate: i.taker_rate,
                        brackets: i.brackets,
                    },
                    &i.budget,
                    i.side,
                    i.atr,
                )
                .map_err(|e| e.to_string());
                assert_eq!(recorded.result, replay);
                assert_eq!(
                    review.result,
                    entry_plan(&ctx, &budget, side, atr).map_err(|e| e.to_string())
                );
            }
        }
    }

    #[test]
    fn user_bounds_can_cover_costs_without_increasing_risk_or_moving_stop() {
        let (ctx, budget) = fixture();
        let atr = ctx.mark / D::from(500);
        for side in [Side::Long, Side::Short] {
            let (stop, base) = levels(&ctx, side, atr, BASE_TARGET_ATR).unwrap();
            let (_, cap) = levels(&ctx, side, atr, MAX_TARGET_ATR).unwrap();
            let old = margin::plan(&ctx, &budget, side, stop, base).unwrap();
            assert!(!reward_passes(&old, budget.slippage_bps));
            let plan = entry_plan(&ctx, &budget, side, atr).unwrap();
            assert_eq!(plan.stop, stop);
            assert!(plan.budgeted_loss <= budget.risk);
            assert!(plan.required_balance <= budget.margin);
            assert!(plan.notional <= budget.max_notional);
            assert!(reward_passes(&plan, budget.slippage_bps));
            assert!(match side {
                Side::Long => plan.take_profit > base && plan.take_profit <= cap,
                Side::Short => plan.take_profit < base && plan.take_profit >= cap,
            });
        }
    }

    #[test]
    fn a_feasible_target_at_the_existing_cap_must_not_be_rejected() {
        let (base, original) = fixture();
        for side in [Side::Long, Side::Short] {
            for offset in [-200, -10, 0, 10, 200] {
                for atr in 1..=500 {
                    let mut ctx = base.clone();
                    ctx.mark += D::from(offset);
                    for notional in [100, 200, 400] {
                        let mut budget = original.clone();
                        budget.max_notional = D::from(notional);
                        let atr = D::from(atr);
                        let (stop, cap) = levels(&ctx, side, atr, MAX_TARGET_ATR).unwrap();
                        let Ok(at_cap) = margin::plan(&ctx, &budget, side, stop, cap) else {
                            continue;
                        };
                        if reward_passes(&at_cap, budget.slippage_bps) {
                            let found = entry_plan(&ctx, &budget, side, atr);
                            assert!(
                                found.is_ok(),
                                "side={side:?}; offset={offset}; atr={atr}; notional={notional}; cap={at_cap:?}; error={found:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn costs_and_lot_limits_are_never_removed_to_manufacture_feasibility() {
        let (mut ctx, budget) = fixture();
        let atr = ctx.mark / D::from(1000);
        ctx.taker_rate = D::new(1, 2);
        for side in [Side::Long, Side::Short] {
            assert!(entry_plan(&ctx, &budget, side, atr).is_err());
        }
        ctx.taker_rate = D::new(5, 4);
        let mut small = budget.clone();
        small.max_notional = D::from(50);
        let result = assess(&ctx, &small, ctx.mark).unwrap();
        assert!(!result.viable());
        assert_eq!(result.checked, 58002);
        assert_eq!(result.effective_budget, small);
        small = budget.clone();
        small.funding_reserve = small.risk;
        assert!(!assess(&ctx, &small, ctx.mark).unwrap().viable());
    }

    #[test]
    fn exit_slippage_is_included_and_no_cost_is_double_counted_in_net_reward() {
        let (ctx, budget) = fixture();
        for side in [Side::Long, Side::Short] {
            let p = entry_plan(&ctx, &budget, side, ctx.mark / D::from(500)).unwrap();
            let before_slippage =
                p.qty * (p.take_profit - p.worst_entry).abs() - p.fee_reserve - p.funding_reserve;
            assert_eq!(
                before_slippage - net_reward(&p, budget.slippage_bps),
                p.qty * p.take_profit * D::new(1, 4)
            );
            assert!(p.fee_reserve > D::ZERO && p.funding_reserve == budget.funding_reserve);
        }
    }

    #[test]
    fn feasibility_witness_matches_real_planner_for_both_directions() {
        let (ctx, budget) = fixture();
        let report = assess(&ctx, &budget, ctx.mark).unwrap();
        assert!(report.viable());
        for (side, sample) in [(Side::Long, report.long), (Side::Short, report.short)] {
            let sample = sample.unwrap();
            let p = entry_plan(
                &ctx,
                &budget,
                side,
                ctx.mark * sample.atr_percent / D::from(100),
            )
            .unwrap();
            assert_eq!(p, sample.plan);
            assert!(reward_passes(&p, budget.slippage_bps));
        }
    }

    #[test]
    fn target_cap_is_not_increased_to_make_boundary_case_pass() {
        let (ctx, budget) = fixture();
        let error = entry_plan(&ctx, &budget, Side::Long, ctx.mark / D::from(1000)).unwrap_err();
        assert!(error.to_string().contains("6 ATR"));
        assert_eq!(MAX_TARGET_ATR, 6);
    }

    #[test]
    fn intermediate_atr_precision_is_not_confused_with_exchange_price_precision() {
        let (ctx, budget) = fixture();
        let atr = ctx.mark / D::from(499);
        assert!(atr.scale() > 12);
        let p = entry_plan(&ctx, &budget, Side::Long, atr).unwrap();
        assert_eq!(p.stop % ctx.filters.tick_size, D::ZERO);
        assert_eq!(p.take_profit % ctx.filters.tick_size, D::ZERO);
        assert!(reward_passes(&p, budget.slippage_bps));
    }

    #[test]
    fn all_returned_plans_obey_same_stop_cost_cap_and_risk_invariants() {
        let (mut ctx, mut budget) = fixture();
        let mut accepted = 0;
        for fee in [
            D::ZERO,
            D::new(2, 4),
            D::new(5, 4),
            D::new(1, 3),
            D::new(1, 2),
        ] {
            ctx.taker_rate = fee;
            for risk in [D::new(5, 1), D::ONE, D::from(2)] {
                budget.risk = risk;
                for fraction in [
                    D::new(1, 3),
                    D::new(2, 3),
                    D::new(4, 3),
                    D::new(1, 2),
                    D::new(3, 2),
                ] {
                    for side in [Side::Long, Side::Short] {
                        let atr = ctx.mark * fraction;
                        if let Ok(p) = entry_plan(&ctx, &budget, side, atr) {
                            accepted += 1;
                            let (stop, base) = levels(&ctx, side, atr, BASE_TARGET_ATR).unwrap();
                            let (_, cap) = levels(&ctx, side, atr, MAX_TARGET_ATR).unwrap();
                            assert_eq!(p.stop, stop);
                            assert!(
                                p.budgeted_loss <= budget.risk
                                    && p.required_balance <= budget.margin
                                    && p.notional <= budget.max_notional
                            );
                            assert!(
                                p.qty >= ctx.filters.min_qty
                                    && p.qty % ctx.filters.step_size == D::ZERO
                            );
                            assert!(reward_passes(&p, budget.slippage_bps));
                            assert_eq!(
                                margin::plan(&ctx, &budget, side, stop, p.take_profit).unwrap(),
                                p
                            );
                            assert!(match side {
                                Side::Long => (base..=cap).contains(&p.take_profit),
                                Side::Short => (cap..=base).contains(&p.take_profit),
                            });
                        }
                    }
                }
            }
        }
        assert!(accepted > 10);
    }
}
