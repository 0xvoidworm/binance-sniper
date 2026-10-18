//! Read-only snapshot under the EXISTING writer lock. No Store::open/commit.
use super::super::{
    automatic, exit,
    session_store::{self, TradeArchive},
};
use super::{package, schema::*};
use anyhow::{Context, Result, ensure};
use rust_decimal::Decimal as D;
use std::{collections::BTreeSet, fs, path::Path};

const LEGACY_STALE: &str =
    "Pemeriksaan sesi gagal; entry dihentikan: Bukti rekonsiliasi usang; order_batch_age_ms=";
fn stop(s: &session_store::Session) -> Stop {
    if let Some(code) = s.stop_code {
        return Stop {
            code: Some(code),
            evidence: StopEvidence::StructuredState,
            legacy_detail: None,
            order_batch_age_ms: None,
            limit_ms: None,
        };
    }
    // Exact entire template only. No arbitrary source text reaches an output/error.
    if let Some(age) = s
        .stop_reason
        .as_deref()
        .and_then(|v| v.strip_prefix(LEGACY_STALE))
        .filter(|v| v.len() <= 13 && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| (5001..=MAX_TIME).contains(v))
    {
        return Stop {
            code: Some(exit::Reason::SafetyCheck),
            evidence: StopEvidence::LegacyExactTemplate,
            legacy_detail: Some(LegacyStop::ReconciliationStale),
            order_batch_age_ms: Some(age),
            limit_ms: Some(5000),
        };
    }
    Stop {
        code: None,
        evidence: StopEvidence::Missing,
        legacy_detail: None,
        order_batch_age_ms: None,
        limit_ms: None,
    }
}
pub fn snapshot(root: &Path) -> Result<Packet> {
    package::private_directory(root)?;
    let lock = package::open_regular(&root.join("session.lock"), 0)?;
    lock.try_lock_shared()
        .context("Evaluation refused: live writer owns the session")?;
    let bytes = package::read_private(&root.join("session.json"), MAX_FILE)?;
    let state = session_store::decode_saved(&bytes)?;
    let s = state
        .session
        .as_ref()
        .context("Evaluation has no saved session")?;
    ensure!(
        s.finished && s.current.is_none(),
        "Evaluation refuses unfinished/active accounting"
    );
    ensure!(
        s.id.len() <= 19
            && !s.id.is_empty()
            && s.id.bytes().all(|c| c.is_ascii_alphanumeric())
            && s.sequence <= 200
            && s.completed <= 100
            && s.completed <= s.sequence,
        "Evaluation source identity/count invalid"
    );
    ensure!(
        s.strategy_version == automatic::VERSION,
        "Evaluation source strategy unsupported"
    );
    s.config.validate()?;
    let mut trades = Vec::new();
    let mut fill_ids = BTreeSet::new();
    let mut funding_ids = BTreeSet::new();
    let mut archives_seen = BTreeSet::new();
    // Enumerate bounded entries and require exact numeric sequence. Other sessions
    // are not exported. Unexpected ledgers for THIS session are never ignored.
    for entry in fs::read_dir(root)?.take(4097) {
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .context("Evaluation source filename invalid")?;
        if let Some(sequence) = name
            .strip_prefix(&format!("{}-", s.id))
            .and_then(|v| v.strip_suffix("-ledger.json"))
        {
            let n = sequence
                .parse::<u32>()
                .ok()
                .filter(|n| *n > 0 && *n <= s.sequence)
                .context("Evaluation source archive sequence invalid")?;
            ensure!(
                sequence == n.to_string() && archives_seen.insert(n),
                "Evaluation duplicate archive"
            );
        }
    }
    ensure!(
        fs::read_dir(root)?.take(4097).count() <= 4096,
        "Evaluation source directory oversized"
    );
    ensure!(
        archives_seen.len() == s.completed as usize,
        "Evaluation source archive count mismatch"
    );
    let mut gross = D::ZERO;
    for sequence in archives_seen {
        let prefix = format!("{}-{sequence}", s.id);
        let bytes = package::read_private(&root.join(format!("{prefix}-ledger.json")), MAX_FILE)?;
        let a: TradeArchive = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("Evaluation source archive malformed"))?;
        ensure!(
            a.schema == 1
                && a.session_id == s.id
                && a.strategy_version == s.strategy_version
                && a.config == s.config
                && a.intent.prefix == prefix
                && a.intent.symbol == super::super::protocol::Symbol::BTCUSDT,
            "Evaluation source archive foreign/inconsistent"
        );
        let r = &a.result;
        ensure!(
            r.fill_ids.len() <= 100_000
                && r.funding_ids.len() <= 20_000
                && r.fill_ids.iter().all(|id| *id > 0 && fill_ids.insert(*id))
                && r.funding_ids
                    .iter()
                    .all(|id| *id > 0 && funding_ids.insert(*id)),
            "Evaluation source duplicate/invalid fill or funding ID"
        );
        ensure!(
            a.decision.side == Some(a.intent.plan.side)
                && a.decision
                    .votes
                    .iter()
                    .all(|v| v.side == a.intent.plan.side),
            "Evaluation source decision inconsistent"
        );
        let mut patterns = Vec::new();
        for v in &a.decision.votes {
            patterns.push(match v.strategy.as_str() {
                "Breakout 20 candle" => Pattern::Breakout20,
                "Pullback tren" => Pattern::TrendPullback,
                "Pantulan range" => Pattern::RangeBounce,
                _ => anyhow::bail!("Evaluation source pattern unsupported"),
            });
        }
        patterns.sort_unstable();
        // No raw reason, wallet, binding, order ID, fill ID or source path in DTOs.
        trades.push(Trade {
            trade_id: format!("trade-{:03}", trades.len() + 1),
            session_id: "session-001".into(),
            intent_at_ms: a.started,
            signal_bar_close_ms: a.decision.bar_ms,
            last_fill_at_ms: r.last_fill_ms,
            side: a.intent.plan.side,
            patterns,
            planned_qty: Amount(a.intent.plan.qty),
            planned_worst_entry: Amount(a.intent.plan.worst_entry),
            planned_stop: Amount(a.intent.plan.stop),
            planned_target: Amount(a.intent.plan.take_profit),
            entry_filled_qty: Amount(r.entry_qty),
            exit_filled_qty: Amount(r.exit_qty),
            gross: Amount(r.gross),
            fees: Amount(r.fees),
            funding_actual: Amount(r.funding),
            net: Amount(r.net),
            fill_count: r.fill_ids.len() as u32,
            funding_record_count: r.funding_ids.len() as u32,
            exit_decision: a.exit_decision,
            exit_outcome: a.exit_outcome,
        });
        // Validate each archive's bounded amounts BEFORE aggregating untrusted D.
        let encoded = serde_json::to_vec(trades.last().unwrap())?;
        let _: Trade = serde_json::from_slice(&encoded)
            .map_err(|_| anyhow::anyhow!("Evaluation source decimal invalid"))?;
        gross = gross
            .checked_add(r.gross)
            .context("Evaluation total overflow")?;
    }
    let summary = Summary {
        session_id: "session-001".into(),
        strategy: Strategy::Ensemble2CostAware,
        requested_minutes: s.config.minutes,
        started_at_ms: s.started,
        deadline_at_ms: s.deadline,
        finished_at_ms: s.updated,
        actual_duration_ms: s
            .updated
            .checked_sub(s.started)
            .context("Evaluation source time invalid")?,
        ended_early: s.updated < s.deadline,
        completed_trades: s.completed,
        gross: Amount(gross),
        fees: Amount(s.fees),
        funding_actual: Amount(s.funding),
        net: Amount(s.net),
        stop: stop(s),
        economics: Economics::from_config(&s.config),
        diagnostics: s.diagnostics.clone(),
        recovery: s.recovery.clone(),
    };
    let encoded = serde_json::to_vec(&summary)?;
    let _: Summary = serde_json::from_slice(&encoded)
        .map_err(|_| anyhow::anyhow!("Evaluation source totals invalid"))?;
    validate_content(&summary, &trades)?;
    // Lock released here, before serialization/publication/evaluator work.
    Ok(Packet { summary, trades })
}
