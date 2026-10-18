use binance_engine::{engine::remaining, *};
use rust_decimal::Decimal as D;
use std::str::FromStr;

fn d(value: &str) -> D {
    D::from_str(value).unwrap()
}
fn config() -> Config {
    serde_json::from_str(include_str!("../fixtures/paper.json")).unwrap()
}
fn fill(qty: D, price: D) -> Fill {
    Fill {
        qty,
        notional: qty * price,
        fee: qty * price * d("0.0005"),
    }
}
fn enter(side: Side) -> Event {
    Event::Enter {
        intent_id: "trade".into(),
        side,
        stop: if side == Side::Long {
            d("98")
        } else {
            d("102")
        },
        take_profit: if side == Side::Long {
            d("104")
        } else {
            d("96")
        },
    }
}
fn entry(qty: D, price: D, status: OrderStatus, reconciled: bool) -> Event {
    Event::EntryReport {
        order_id: "trade-entry".into(),
        status,
        fill: fill(qty, price),
        reconciled,
    }
}
fn exit(qty: D, price: D, status: OrderStatus, reconciled: bool) -> Event {
    Event::ExitReport {
        order_id: "trade-exit".into(),
        status,
        fill: fill(qty, price),
        reconciled,
    }
}

struct Rig {
    engine: Engine,
    seq: usize,
    now: u64,
}
impl Rig {
    fn new() -> Self {
        Self::with(config())
    }
    fn with(config: Config) -> Self {
        let mut rig = Self {
            engine: Engine::new(config).unwrap(),
            seq: 0,
            now: 1_000_000,
        };
        rig.quote("100", "100.1");
        rig
    }
    fn apply(&mut self, event: Event) -> Outcome {
        self.seq += 1;
        self.now += 1;
        self.engine.apply(&Envelope {
            id: format!("e{}", self.seq),
            at_ms: self.now,
            event,
        })
    }
    fn ok(&mut self, event: Event) -> Vec<Effect> {
        let result = self.apply(event);
        assert!(result.accepted, "{:?}", result.error);
        result.effects
    }
    fn bad(&mut self, event: Event) {
        let state = self.engine.state().clone();
        let result = self.apply(event);
        assert!(!result.accepted);
        assert_eq!(result.effects, vec![]);
        assert_eq!(self.engine.state(), &state);
    }
    fn quote(&mut self, bid: &str, ask: &str) -> Vec<Effect> {
        self.ok(Event::Quote {
            quote: Quote {
                bid: d(bid),
                ask: d(ask),
                observed_at_ms: self.now + 1,
            },
        })
    }
    fn start(&mut self, side: Side) -> D {
        self.ok(enter(side));
        self.engine.state().active.as_ref().unwrap().plan.qty
    }
    fn protections(&mut self) {
        for suffix in ["sl", "tp"] {
            self.ok(Event::ProtectionReport {
                order_id: format!("trade-{suffix}"),
                accepted: true,
            });
        }
    }
    fn open(&mut self, side: Side) -> D {
        let qty = self.start(side);
        self.ok(entry(
            qty,
            if side == Side::Long {
                d("100.1")
            } else {
                d("100")
            },
            OrderStatus::Filled,
            false,
        ));
        self.protections();
        qty
    }
}

fn has_exit(effects: &[Effect], reason: ExitReason, qty: D) -> bool {
    effects.iter().any(|e| matches!(e, Effect::PlaceExit { qty: q, reason: r, reduce_only: true, .. } if *q == qty && *r == reason))
}

#[test]
fn long_partial_entry_partial_exit_net_and_cleanup() {
    let mut r = Rig::new();
    let qty = r.start(Side::Long);
    let part = d("1");
    let effects = r.ok(entry(part, d("100.1"), OrderStatus::PartiallyFilled, false));
    assert_eq!(
        effects
            .iter()
            .filter(|e| matches!(
                e,
                Effect::PlaceProtection {
                    close_position: true,
                    ..
                }
            ))
            .count(),
        2
    );
    r.protections();
    assert!(
        r.ok(entry(qty, d("100.1"), OrderStatus::Filled, false))
            .is_empty()
    );
    assert!(has_exit(
        &r.quote("104.1", "104.2"),
        ExitReason::TakeProfit,
        qty
    ));
    r.ok(exit(part, d("104.1"), OrderStatus::PartiallyFilled, false));
    assert_eq!(
        remaining(r.engine.state().active.as_ref().unwrap()).unwrap(),
        qty - part
    );
    assert_eq!(
        r.engine.state().realized_net,
        part * d("4") - qty * d("100.1") * d("0.0005") - part * d("104.1") * d("0.0005")
    );
    let effects = r.ok(exit(qty, d("104.1"), OrderStatus::Filled, false));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::CancelProtections { .. }))
    );
    r.bad(enter(Side::Long));
    r.ok(Event::CleanupConfirmed {
        intent_id: "trade".into(),
    });
    assert_eq!(
        r.engine.state().realized_net,
        qty * d("4") - qty * d("204.2") * d("0.0005")
    );
    assert_eq!(
        r.engine.state().equity,
        d("1000") + r.engine.state().realized_net
    );
    assert_eq!(r.engine.state().completed_trades, 1);
    assert!(r.engine.state().active.is_none());
    r.bad(enter(Side::Long)); // An intent is never recycled.
}

#[test]
fn short_uses_ask_to_trigger_and_correct_pnl_sign() {
    let mut r = Rig::new();
    let qty = r.open(Side::Short);
    assert!(r.quote("95.9", "96.1").is_empty());
    assert!(has_exit(
        &r.quote("95.9", "96"),
        ExitReason::TakeProfit,
        qty
    ));
    r.ok(exit(qty, d("96"), OrderStatus::Filled, false));
    assert_eq!(
        r.engine.state().realized_net,
        qty * d("4") - qty * d("196") * d("0.0005")
    );
}

#[test]
fn stops_trigger_both_sides_and_record_losses() {
    for side in [Side::Long, Side::Short] {
        let mut r = Rig::new();
        let qty = r.open(side);
        let (bid, ask, price) = if side == Side::Long {
            ("97.9", "98", "97.9")
        } else {
            ("102", "102.1", "102.1")
        };
        assert!(has_exit(&r.quote(bid, ask), ExitReason::StopLoss, qty));
        r.ok(exit(qty, d(price), OrderStatus::Filled, false));
        assert!(r.engine.state().realized_net < D::ZERO);
    }
}

#[test]
fn partial_entry_must_cancel_before_exit_and_late_fill_is_included() {
    let mut r = Rig::new();
    let qty = r.start(Side::Long);
    r.ok(entry(
        d("1"),
        d("100.1"),
        OrderStatus::PartiallyFilled,
        false,
    ));
    r.protections();
    let effects = r.ok(Event::Close {
        reason: ExitReason::Manual,
    });
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::CancelEntry { .. }))
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::PlaceExit { .. }))
    );
    assert!(
        r.ok(entry(
            d("2"),
            d("100.1"),
            OrderStatus::PartiallyFilled,
            false
        ))
        .is_empty()
    );
    assert!(has_exit(
        &r.ok(entry(d("2"), d("100.1"), OrderStatus::Canceled, false)),
        ExitReason::Manual,
        d("2")
    ));
    r.bad(entry(qty, d("100.1"), OrderStatus::Filled, false));
}

#[test]
fn uncertain_entry_never_resubmits_and_requires_reconciliation() {
    let mut r = Rig::new();
    let qty = r.start(Side::Long);
    let effects = r.ok(Event::EntryUncertain {
        order_id: "trade-entry".into(),
    });
    assert_eq!(
        effects,
        vec![Effect::QueryOrder {
            order_id: "trade-entry".into()
        }]
    );
    assert!(
        r.ok(Event::EntryUncertain {
            order_id: "trade-entry".into()
        })
        .is_empty()
    );
    r.bad(enter(Side::Long));
    r.bad(entry(qty, d("100.1"), OrderStatus::Filled, false));
    r.ok(entry(qty, d("100.1"), OrderStatus::Filled, true));
    assert!(!r.engine.state().active.as_ref().unwrap().entry.uncertain);
}

#[test]
fn uncertain_entry_close_waits_for_authoritative_query() {
    let mut r = Rig::new();
    r.start(Side::Long);
    r.ok(Event::EntryUncertain {
        order_id: "trade-entry".into(),
    });
    assert!(
        r.ok(Event::Close {
            reason: ExitReason::Manual
        })
        .is_empty()
    );
    let effects = r.ok(entry(D::ZERO, D::ZERO, OrderStatus::New, true));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::CancelEntry { .. }))
    );
    r.ok(entry(D::ZERO, D::ZERO, OrderStatus::Canceled, false));
    assert!(r.engine.state().active.is_none());
}

#[test]
fn order_timeout_queries_once_and_does_not_claim_rejection() {
    let mut r = Rig::new();
    r.start(Side::Long);
    r.now += 2000;
    assert_eq!(
        r.ok(Event::Tick),
        vec![Effect::QueryOrder {
            order_id: "trade-entry".into()
        }]
    );
    assert!(r.ok(Event::Tick).is_empty());
    assert_eq!(
        r.engine.state().active.as_ref().unwrap().entry.status,
        OrderStatus::Pending
    );
}

#[test]
fn rejection_and_expiry_without_fill_release_entry() {
    for status in [
        OrderStatus::Rejected,
        OrderStatus::Expired,
        OrderStatus::Canceled,
    ] {
        let mut r = Rig::new();
        r.start(Side::Long);
        r.ok(entry(D::ZERO, D::ZERO, status, false));
        assert!(r.engine.state().active.is_none());
        assert_eq!(r.engine.state().completed_trades, 0);
    }
}

#[test]
fn protection_failure_halts_and_requests_reduce_only_exit() {
    let mut r = Rig::new();
    let qty = r.start(Side::Long);
    r.ok(entry(qty, d("100.1"), OrderStatus::Filled, false));
    let effects = r.ok(Event::ProtectionReport {
        order_id: "trade-sl".into(),
        accepted: false,
    });
    assert!(has_exit(&effects, ExitReason::ProtectionFailure, qty));
    assert!(r.engine.state().halt_reason.is_some());
    r.bad(Event::ProtectionReport {
        order_id: "trade-sl".into(),
        accepted: true,
    });
}

#[test]
fn protection_timeout_waits_for_partial_entry_cancel() {
    let mut r = Rig::new();
    r.start(Side::Long);
    r.ok(entry(
        d("1"),
        d("100.1"),
        OrderStatus::PartiallyFilled,
        false,
    ));
    r.now += 1000;
    let effects = r.ok(Event::Tick);
    assert!(
        r.engine
            .state()
            .halt_reason
            .as_ref()
            .unwrap()
            .contains("protection")
    );
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::CancelEntry { .. }))
    );
    assert!(has_exit(
        &r.ok(entry(d("1"), d("100.1"), OrderStatus::Canceled, false)),
        ExitReason::ProtectionFailure,
        d("1")
    ));
}

#[test]
fn stale_data_halts_and_requests_flatten_without_waiting_for_next_quote() {
    let mut r = Rig::new();
    let qty = r.open(Side::Long);
    r.now += 5001;
    assert!(has_exit(&r.ok(Event::Tick), ExitReason::StaleData, qty));
    assert!(
        r.engine
            .state()
            .halt_reason
            .as_ref()
            .unwrap()
            .contains("stale")
    );
}

#[test]
fn exit_timeout_and_rejection_preserve_residual_exposure() {
    let mut r = Rig::new();
    let qty = r.open(Side::Long);
    r.ok(Event::Close {
        reason: ExitReason::Manual,
    });
    r.now += 2000;
    assert_eq!(
        r.ok(Event::Tick),
        vec![Effect::QueryOrder {
            order_id: "trade-exit".into()
        }]
    );
    r.bad(exit(qty, d("100"), OrderStatus::Filled, false));
    r.ok(exit(D::ZERO, D::ZERO, OrderStatus::Rejected, true));
    assert!(
        r.engine
            .state()
            .halt_reason
            .as_ref()
            .unwrap()
            .contains("residual")
    );
    assert_eq!(
        remaining(r.engine.state().active.as_ref().unwrap()).unwrap(),
        qty
    );
    r.bad(Event::CleanupConfirmed {
        intent_id: "trade".into(),
    });
}

#[test]
fn explicit_exit_uncertainty_resolves_without_duplicate_fee() {
    let mut r = Rig::new();
    let qty = r.open(Side::Long);
    r.ok(Event::Close {
        reason: ExitReason::Manual,
    });
    r.ok(Event::ExitUncertain {
        order_id: "trade-exit".into(),
    });
    r.ok(exit(qty, d("100"), OrderStatus::Filled, true));
    let net = r.engine.state().realized_net;
    assert!(
        r.ok(exit(qty, d("100"), OrderStatus::Filled, true))
            .is_empty()
    );
    assert_eq!(r.engine.state().realized_net, net);
}

#[test]
fn adverse_fill_is_recorded_then_halted_instead_of_discarding_exposure() {
    for side in [Side::Long, Side::Short] {
        let mut r = Rig::new();
        let qty = r.start(side);
        let price = if side == Side::Long {
            d("100.5")
        } else {
            d("99.5")
        };
        let effects = r.ok(entry(qty, price, OrderStatus::Filled, false));
        assert!(has_exit(&effects, ExitReason::RiskLimit, qty));
        assert_eq!(
            r.engine.state().active.as_ref().unwrap().entry.fill.qty,
            qty
        );
        assert!(r.engine.state().halt_reason.is_some());
    }
}

#[test]
fn stop_crossed_before_first_fill_is_still_honored() {
    let mut r = Rig::new();
    let qty = r.start(Side::Long);
    r.quote("97.9", "98");
    assert!(has_exit(
        &r.ok(entry(qty, d("100.1"), OrderStatus::Filled, false)),
        ExitReason::StopLoss,
        qty
    ));
}

#[test]
fn quote_gap_exceeds_loss_threshold_and_halt_persists_after_cleanup() {
    let mut r = Rig::new();
    let qty = r.open(Side::Long);
    let effects = r.quote("80", "80.1");
    assert!(has_exit(&effects, ExitReason::RiskLimit, qty));
    assert!(r.engine.state().equity < d("950")); // A threshold cannot guarantee a maximum gap loss.
    r.ok(exit(qty, d("80"), OrderStatus::Filled, false));
    r.ok(Event::CleanupConfirmed {
        intent_id: "trade".into(),
    });
    r.now += 86_400_000;
    r.quote("100", "100.1");
    r.bad(Event::Enter {
        intent_id: "new".into(),
        side: Side::Long,
        stop: d("98"),
        take_profit: d("104"),
    });
}

#[test]
fn daily_limit_trips_before_total_limit() {
    let mut r = Rig::new();
    let qty = r.open(Side::Long);
    assert!(has_exit(&r.quote("93", "93.1"), ExitReason::RiskLimit, qty));
    let loss = d("1000") - r.engine.state().equity;
    assert!(loss >= d("25") && loss < d("50"));
    assert!(r.engine.state().halt_reason.is_some());
}

#[test]
fn utc_day_change_baselines_previous_equity_and_counts_price_gap() {
    let mut r = Rig::new();
    r.now = 86_399_900;
    r.quote("100", "100.1");
    let qty = r.open(Side::Long);
    let previous = r.engine.state().equity;
    r.now = 86_400_001;
    assert!(has_exit(&r.quote("93", "93.1"), ExitReason::RiskLimit, qty));
    assert_eq!(r.engine.state().day_index, Some(1));
    assert_eq!(r.engine.state().day_start_equity, previous);
}

#[test]
fn conflicting_cumulative_reports_do_not_corrupt_state() {
    let mut r = Rig::new();
    let qty = r.start(Side::Long);
    r.ok(entry(
        d("1"),
        d("100.1"),
        OrderStatus::PartiallyFilled,
        false,
    ));
    r.bad(entry(
        d("0.5"),
        d("100.1"),
        OrderStatus::PartiallyFilled,
        false,
    ));
    r.bad(entry(
        d("1"),
        d("100.2"),
        OrderStatus::PartiallyFilled,
        false,
    ));
    r.bad(entry(qty + d("1"), d("100.1"), OrderStatus::Filled, false));
    r.bad(entry(d("1"), d("100.1"), OrderStatus::Filled, false));
    r.bad(Event::EntryReport {
        order_id: "other".into(),
        status: OrderStatus::Filled,
        fill: fill(qty, d("100.1")),
        reconciled: true,
    });
    r.bad(Event::CleanupConfirmed {
        intent_id: "trade".into(),
    });
}

#[test]
fn duplicate_broker_report_does_not_repeat_protection_or_charge_fees() {
    let mut r = Rig::new();
    r.start(Side::Long);
    let event = entry(d("1"), d("100.1"), OrderStatus::PartiallyFilled, false);
    r.ok(event.clone());
    let net = r.engine.state().realized_net;
    assert!(r.ok(event).is_empty());
    assert_eq!(r.engine.state().realized_net, net);
}

#[test]
fn invalid_times_quotes_and_empty_events_leave_state_unchanged() {
    let mut r = Rig::new();
    for quote in [
        Quote {
            bid: d("100"),
            ask: d("99"),
            observed_at_ms: r.now,
        },
        Quote {
            bid: d("100"),
            ask: d("101"),
            observed_at_ms: r.now,
        },
        Quote {
            bid: d("100"),
            ask: d("100.1"),
            observed_at_ms: r.now - 6000,
        },
        Quote {
            bid: d("100"),
            ask: d("100.1"),
            observed_at_ms: r.now + 1000,
        },
    ] {
        r.bad(Event::Quote { quote });
    }
    let state = r.engine.state().clone();
    assert!(
        !r.engine
            .apply(&Envelope {
                id: "old".into(),
                at_ms: 0,
                event: Event::Tick
            })
            .accepted
    );
    assert_eq!(*r.engine.state(), state);
    r.bad(Event::Close {
        reason: ExitReason::Manual,
    });
    r.bad(Event::EntryUncertain {
        order_id: "missing".into(),
    });
    r.now += 6000;
    r.bad(enter(Side::Long));
}

#[test]
fn decimal_sizing_obeys_grid_budget_and_notional_across_prices() {
    let c = config();
    for price in [10u64, 100, 1000, 10_000, 100_000] {
        for side in [Side::Long, Side::Short] {
            let p = D::from(price);
            let quote = Quote {
                bid: p,
                ask: p + d("0.01"),
                observed_at_ms: 100,
            };
            let (stop, tp) = if side == Side::Long {
                (p * d("0.98"), p * d("1.04"))
            } else {
                (p * d("1.02"), p * d("0.96"))
            };
            let plan = c.plan(&quote, 100, side, stop, tp).unwrap();
            assert!(plan.budgeted_loss <= c.risk_per_trade && plan.notional <= c.max_notional);
            assert_eq!(
                (plan.qty - c.filters.min_qty) % c.filters.step_size,
                D::ZERO
            );
            assert!(plan.qty >= c.filters.min_qty && plan.qty <= c.filters.max_qty);
        }
    }
}

#[test]
fn invalid_config_filters_and_costs_fail_closed() {
    let c = config();
    for invalid in [
        Config {
            capital: D::ZERO,
            ..c.clone()
        },
        Config {
            max_notional: d("1001"),
            ..c.clone()
        },
        Config {
            max_daily_loss: d("5"),
            ..c.clone()
        },
        Config {
            fee_bps: d("101"),
            ..c.clone()
        },
        Config {
            order_timeout_ms: 0,
            ..c.clone()
        },
    ] {
        assert!(Engine::new(invalid).is_err());
    }
    let quote = Quote {
        bid: d("100"),
        ask: d("100.1"),
        observed_at_ms: 100,
    };
    assert!(
        c.plan(&quote, 100, Side::Long, d("98.001"), d("104"))
            .is_err()
    );
    assert!(c.plan(&quote, 100, Side::Long, d("101"), d("104")).is_err());
    assert!(c.plan(&quote, 100, Side::Short, d("99"), d("96")).is_err());
    let tiny = Config {
        risk_per_trade: d("0.00001"),
        ..c
    };
    assert!(
        tiny.plan(&quote, 100, Side::Long, d("98"), d("104"))
            .is_err()
    );
}

#[test]
fn config_requires_decimal_strings_and_paper_only() {
    let config = include_str!("../fixtures/paper.json");
    for invalid in [
        config.replace("\"paper\"", "\"live\""),
        config.replace("\"1000\"", "1000"),
        config.replace("\"capital\":", "\"unknown\":"),
    ] {
        assert!(serde_json::from_str::<Config>(&invalid).is_err());
    }
}
