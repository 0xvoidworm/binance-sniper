//! Diagnostic evidence only: these fields never authorize orders or alter risk.
use super::{
    outbox::Outbox,
    protocol::{Command, ProtectionKind, Receipt},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Order is the deterministic priority for causes observed on the SAME decision.
/// Later observations never rewrite the original primary cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    ProtectionFailure,
    DataFailure,
    SafetyCheck,
    SessionLoss,
    DailyLoss,
    LiquidationBuffer,
    AccountChanged,
    UserStop,
    WindowClosed,
    SessionDeadline,
    TradeLimit,
    SequenceLimit,
    Recovery,
    FundingWindow,
    HoldTimeout,
    OppositeSignal,
    LifecycleClose,
    OtherStop,
    LegacyUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// Exchange-derived time of the first supervisor close decision with data.
    /// Not the UI click time, exchange fill time or an inferred legacy timestamp.
    pub decision_at_ms: u64,
    pub rule_version: String,
    pub primary: Reason,
    pub causes: Vec<Reason>,
}
pub fn record(trace: &mut Option<Decision>, mut causes: Vec<Reason>, now: u64) {
    causes.sort_unstable();
    causes.dedup();
    let Some(primary) = causes.first().copied() else {
        return;
    };
    if let Some(trace) = trace {
        trace.causes.extend(causes);
        trace.causes.sort_unstable();
        trace.causes.dedup();
    } else {
        *trace = Some(Decision {
            decision_at_ms: now,
            rule_version: "exit-evidence-1".into(),
            primary,
            causes,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    StopLoss,
    TakeProfit,
    ReduceOnly,
    Mixed,
    Unknown,
}

/// IDs here have already passed the ledger's nonzero fill/order/quantity checks.
/// A triggered/expired Algo without verified fills does not establish an outcome.
pub fn observed(box_: &Outbox, filled_exit_orders: &[u64]) -> Outcome {
    let mut kinds = BTreeSet::new();
    for id in filled_exit_orders {
        let mut matched = BTreeSet::new();
        for row in box_.records() {
            match (&row.command, &row.receipt) {
                (
                    Command::Market {
                        reduce_only: true, ..
                    },
                    Some(Receipt::Order(o)),
                ) if o.exchange_id == *id => {
                    matched.insert(Outcome::ReduceOnly);
                }
                (Command::Protect { kind, .. }, Some(Receipt::Algo(a)))
                    if a.actual_order_id == Some(*id) =>
                {
                    matched.insert(match kind {
                        ProtectionKind::Stop => Outcome::StopLoss,
                        ProtectionKind::Target => Outcome::TakeProfit,
                    });
                }
                (Command::CancelAlgo { client_id }, Some(Receipt::Algo(a)))
                    if a.actual_order_id == Some(*id) =>
                {
                    for original in box_.records() {
                        if let Command::Protect {
                            client_id: original_id,
                            kind,
                            ..
                        } = &original.command
                            && original_id == client_id
                        {
                            matched.insert(match kind {
                                ProtectionKind::Stop => Outcome::StopLoss,
                                ProtectionKind::Target => Outcome::TakeProfit,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        if matched.len() != 1 {
            return Outcome::Unknown;
        }
        kinds.extend(matched);
    }
    match kinds.len() {
        0 => Outcome::Unknown,
        1 => *kinds.first().unwrap(),
        _ => Outcome::Mixed,
    }
}

#[test]
fn p1_exit_priority_is_deterministic_but_later_causes_do_not_rewrite_history() {
    let mut trace = None;
    record(
        &mut trace,
        vec![
            Reason::HoldTimeout,
            Reason::FundingWindow,
            Reason::HoldTimeout,
        ],
        100,
    );
    let first = trace.as_ref().unwrap().clone();
    assert_eq!(first.primary, Reason::FundingWindow);
    assert_eq!(first.causes.len(), 2);
    record(&mut trace, vec![Reason::ProtectionFailure], 200);
    let trace = trace.unwrap();
    assert_eq!(trace.primary, first.primary);
    assert_eq!(trace.decision_at_ms, 100);
    assert_eq!(trace.causes.len(), 3);
}
