use super::super::{exit, session_store};
use anyhow::{Result, ensure};
use binance_engine::Side;
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

pub const FILES: [&str; 4] = [
    "summary.json",
    "trades.jsonl",
    "decisions.jsonl",
    "health.jsonl",
];
pub const MAX_FILE: u64 = 2_097_152;
pub const MAX_MANIFEST: u64 = 16_384;
pub const MAX_TRADES: usize = 100;
pub const MAX_TIME: u64 = 4_102_444_800_000;

/// Canonical decimal string, never a JSON float or scientific notation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Amount(pub D);
impl TryFrom<String> for Amount {
    type Error = &'static str;
    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        if s.len() > 28 || s.is_empty() {
            return Err("Decimal evaluation invalid");
        }
        let n: D = s.parse().map_err(|_| "Decimal evaluation invalid")?;
        if n.scale() > 12
            || n.abs() > D::from(1_000_000_000_000_u64)
            || n.normalize().to_string() != s
        {
            return Err("Decimal evaluation noncanonical/out of bounds");
        }
        Ok(Self(n))
    }
}
impl From<Amount> for String {
    fn from(a: Amount) -> Self {
        a.0.normalize().to_string()
    }
}
impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.normalize())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Economics {
    pub margin_cap: Amount,
    pub risk_per_trade: Amount,
    pub notional_cap: Amount,
    pub leverage: u32,
    pub funding_reserve: Amount,
    pub slippage_bps: Amount,
    pub liquidation_buffer_bps: Amount,
    pub session_loss_cap: Amount,
    pub daily_loss_cap: Amount,
    pub max_trades: u32,
}
impl Economics {
    pub fn from_config(c: &session_store::Config) -> Self {
        Self {
            margin_cap: Amount(c.budget.margin),
            risk_per_trade: Amount(c.budget.risk),
            notional_cap: Amount(c.budget.max_notional),
            leverage: c.budget.leverage,
            funding_reserve: Amount(c.budget.funding_reserve),
            slippage_bps: Amount(c.budget.slippage_bps),
            liquidation_buffer_bps: Amount(c.budget.liquidation_buffer_bps),
            session_loss_cap: Amount(c.session_loss),
            daily_loss_cap: Amount(c.daily_loss),
            max_trades: c.max_trades,
        }
    }
    pub fn validate(&self, minutes: u32) -> Result<()> {
        session_store::Config {
            budget: super::super::margin::Budget {
                margin: self.margin_cap.0,
                risk: self.risk_per_trade.0,
                max_notional: self.notional_cap.0,
                leverage: self.leverage,
                funding_reserve: self.funding_reserve.0,
                slippage_bps: self.slippage_bps.0,
                liquidation_buffer_bps: self.liquidation_buffer_bps.0,
            },
            daily_loss: self.daily_loss_cap.0,
            session_loss: self.session_loss_cap.0,
            minutes,
            max_trades: self.max_trades,
        }
        .validate()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    Ensemble2CostAware,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    LocalFinalSessionAndTradeArchivesNoExchangeRequery,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Missing {
    SourceBuildVersion,
    FullDecisionHistory,
    HealthTimeSeries,
    IntratradePricePath,
    ActualFillPrices,
    RawFillDetails,
    ExitDecision,
    ExitOutcome,
    StructuredSessionStop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopEvidence {
    StructuredState,
    LegacyExactTemplate,
    Missing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyStop {
    ReconciliationStale,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub code: Option<exit::Reason>,
    pub evidence: StopEvidence,
    pub legacy_detail: Option<LegacyStop>,
    pub order_batch_age_ms: Option<u64>,
    pub limit_ms: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub session_id: String,
    pub strategy: Strategy,
    pub requested_minutes: u32,
    pub started_at_ms: u64,
    pub deadline_at_ms: u64,
    /// Final local state update, not the last fill timestamp.
    pub finished_at_ms: u64,
    pub actual_duration_ms: u64,
    pub ended_early: bool,
    pub completed_trades: u32,
    pub gross: Amount,
    pub fees: Amount,
    pub funding_actual: Amount,
    pub net: Amount,
    pub stop: Stop,
    pub economics: Economics,
    // Optional extension: absence preserves canonical bytes of 0.6.8 packets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<super::super::diagnostics::Diagnostics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<super::super::recovery::State>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Breakout20,
    TrendPullback,
    RangeBounce,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trade {
    pub trade_id: String,
    pub session_id: String,
    /// Local intent time; NOT exchange entry fill time.
    pub intent_at_ms: u64,
    pub signal_bar_close_ms: u64,
    pub last_fill_at_ms: u64,
    pub side: Side,
    pub patterns: Vec<Pattern>,
    pub planned_qty: Amount,
    pub planned_worst_entry: Amount,
    pub planned_stop: Amount,
    pub planned_target: Amount,
    pub entry_filled_qty: Amount,
    pub exit_filled_qty: Amount,
    pub gross: Amount,
    pub fees: Amount,
    pub funding_actual: Amount,
    pub net: Amount,
    pub fill_count: u32,
    pub funding_record_count: u32,
    pub exit_decision: Option<exit::Decision>,
    pub exit_outcome: Option<exit::Outcome>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    pub bytes: u64,
    pub sha256: String,
    pub records: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub export_id: String,
    pub session_id: String,
    pub exporter_version: String,
    pub exporter_source_sha256: String,
    // Original binary not recorded by legacy state; never substitute exporter version.
    pub source_build_version: Option<String>,
    pub strategy: Strategy,
    pub config_sha256: String,
    pub exported_at_ms: u64,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub final_accounting: bool,
    pub provenance: Provenance,
    pub missing_data: Vec<Missing>,
    pub files: BTreeMap<String, FileEntry>,
}
pub struct Packet {
    pub summary: Summary,
    pub trades: Vec<Trade>,
}
pub struct Validated {
    pub manifest: Manifest,
    pub manifest_digest: String,
    pub summary: Summary,
    pub trades: Vec<Trade>,
}
pub fn hash_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn missing(trades: &[Trade], summary: &Summary) -> Vec<Missing> {
    let mut out = vec![
        Missing::FullDecisionHistory,
        Missing::HealthTimeSeries,
        Missing::IntratradePricePath,
        Missing::ActualFillPrices,
        Missing::RawFillDetails,
    ];
    if summary.diagnostics.is_none() {
        out.push(Missing::SourceBuildVersion);
    }
    if trades.iter().any(|t| t.exit_decision.is_none()) {
        out.push(Missing::ExitDecision);
    }
    if trades.iter().any(|t| t.exit_outcome.is_none()) {
        out.push(Missing::ExitOutcome);
    }
    if summary.stop.evidence != StopEvidence::StructuredState {
        out.push(Missing::StructuredSessionStop);
    }
    out.sort_unstable();
    out
}
pub fn validate_content(s: &Summary, trades: &[Trade]) -> Result<()> {
    if let Some(r) = &s.recovery {
        ensure!(
            r.valid()
                && !r.active
                && s.diagnostics.as_ref().is_some_and(|d| matches!(
                    d.build,
                    super::super::diagnostics::Build::V0610
                        | super::super::diagnostics::Build::V0611
                        | super::super::diagnostics::Build::V0612
                        | super::super::diagnostics::Build::V0613
                        | super::super::diagnostics::Build::V0614
                )),
            "Evaluation recovery metadata invalid"
        );
    }
    if let Some(d) = &s.diagnostics {
        use super::super::diagnostics::Gate;
        let n = |gate: Gate| u64::from(d.counts[gate as usize]);
        ensure!(
            n(Gate::Funding) + n(Gate::Economics) + n(Gate::Intent) <= n(Gate::Signal)
                && n(Gate::Intent) >= u64::from(s.completed_trades)
                && d.last_gate
                    .map_or_else(|| d.counts == [0; 11], |g| n(g) > 0),
            "Evaluation decision counters inconsistent"
        );
    }
    ensure!(
        s.session_id == "session-001"
            && s.started_at_ms > 0
            && s.started_at_ms <= s.finished_at_ms
            && s.finished_at_ms <= MAX_TIME
            && s.deadline_at_ms <= MAX_TIME,
        "Evaluation session/time invalid"
    );
    s.economics.validate(s.requested_minutes)?;
    ensure!(
        s.deadline_at_ms.checked_sub(s.started_at_ms)
            == Some(u64::from(s.requested_minutes) * 60_000)
            && s.actual_duration_ms == s.finished_at_ms - s.started_at_ms
            && s.ended_early == (s.finished_at_ms < s.deadline_at_ms),
        "Evaluation duration mismatch"
    );
    ensure!(
        trades.len() <= MAX_TRADES
            && s.completed_trades as usize == trades.len()
            && s.completed_trades <= s.economics.max_trades,
        "Evaluation trade count invalid"
    );
    let stop = &s.stop;
    ensure!(
        match stop.evidence {
            StopEvidence::StructuredState =>
                stop.code.is_some()
                    && stop.legacy_detail.is_none()
                    && stop.order_batch_age_ms.is_none()
                    && stop.limit_ms.is_none(),
            StopEvidence::LegacyExactTemplate =>
                stop.code == Some(exit::Reason::SafetyCheck)
                    && stop.legacy_detail == Some(LegacyStop::ReconciliationStale)
                    && stop
                        .order_batch_age_ms
                        .is_some_and(|v| (5001..=MAX_TIME).contains(&v))
                    && stop.limit_ms == Some(5000),
            StopEvidence::Missing =>
                stop.code.is_none()
                    && stop.legacy_detail.is_none()
                    && stop.order_batch_age_ms.is_none()
                    && stop.limit_ms.is_none(),
        },
        "Evaluation stop evidence invalid"
    );
    let (mut gross, mut fees, mut funding, mut net) = (D::ZERO, D::ZERO, D::ZERO, D::ZERO);
    let mut last = s.started_at_ms;
    for (i, t) in trades.iter().enumerate() {
        ensure!(
            t.session_id == s.session_id && t.trade_id == format!("trade-{:03}", i + 1),
            "Evaluation trade identity/order invalid"
        );
        ensure!(
            t.intent_at_ms >= last
                && t.intent_at_ms <= t.last_fill_at_ms
                && t.last_fill_at_ms <= s.finished_at_ms
                && t.signal_bar_close_ms < t.intent_at_ms
                && t.intent_at_ms - t.signal_bar_close_ms <= 15_000,
            "Evaluation trade timestamps invalid"
        );
        last = t.last_fill_at_ms;
        ensure!(
            !t.patterns.is_empty()
                && t.patterns.len() <= 3
                && t.patterns.windows(2).all(|p| p[0] < p[1]),
            "Evaluation patterns invalid"
        );
        ensure!(
            t.planned_qty.0 > D::ZERO
                && t.entry_filled_qty.0 > D::ZERO
                && t.entry_filled_qty == t.exit_filled_qty
                && t.entry_filled_qty.0 <= t.planned_qty.0
                && t.planned_worst_entry.0 > D::ZERO
                && t.planned_stop.0 > D::ZERO
                && t.planned_target.0 > D::ZERO
                && match t.side {
                    Side::Long =>
                        t.planned_stop.0 < t.planned_worst_entry.0
                            && t.planned_worst_entry.0 < t.planned_target.0,
                    Side::Short =>
                        t.planned_target.0 < t.planned_worst_entry.0
                            && t.planned_worst_entry.0 < t.planned_stop.0,
                },
            "Evaluation qty/plan invalid"
        );
        ensure!(
            t.fill_count >= 2
                && t.fill_count <= 100_000
                && t.funding_record_count <= 20_000
                && (t.funding_record_count != 0 || t.funding_actual.0.is_zero())
                && t.net.0 == t.gross.0 - t.fees.0 + t.funding_actual.0,
            "Evaluation ledger mismatch"
        );
        if let Some(e) = &t.exit_decision {
            ensure!(
                e.rule_version == "exit-evidence-1"
                    && !e.causes.is_empty()
                    && e.causes.len() <= 20
                    && e.causes.contains(&e.primary)
                    && e.causes.windows(2).all(|p| p[0] < p[1])
                    && e.decision_at_ms >= t.intent_at_ms
                    && e.decision_at_ms <= s.finished_at_ms,
                "Evaluation exit evidence invalid"
            );
        }
        gross += t.gross.0;
        fees += t.fees.0;
        funding += t.funding_actual.0;
        net += t.net.0;
    }
    ensure!(
        (gross, fees, funding, net) == (s.gross.0, s.fees.0, s.funding_actual.0, s.net.0)
            && net == gross - fees + funding,
        "Evaluation totals mismatch"
    );
    Ok(())
}
