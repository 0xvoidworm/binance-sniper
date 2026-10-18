use super::*;
use binance_engine::{Protection, Quote};
use std::time::Duration;
use tempfile::tempdir;

fn batch(count: usize, descending: bool) -> Batch {
    let bars = strategy::tests::history(count, descending);
    let last = bars.last().unwrap();
    let now = last.close_ms + 1000;
    let base: Config =
        serde_json::from_str(include_str!("../../../../engine-rs/fixtures/paper.json")).unwrap();
    Batch {
        now,
        quote: Quote {
            bid: last.close,
            ask: last.close + D::new(1, 2),
            observed_at_ms: now,
        },
        next_funding: now + 8 * SESSION_MS,
        bars,
        filters: base.filters,
    }
}
fn tick(batch: &mut Batch, now: u64) {
    batch.now = now;
    batch.quote.observed_at_ms = now;
}

#[test]
fn warmup_new_bar_long_short_sizing_protections_and_dedup() {
    for descending in [false, true] {
        let dir = tempdir().unwrap();
        let mut s = Session::new(dir.path().join("session"), &batch(100, descending)).unwrap();
        assert!(s.journal.state().active.is_none());
        assert!(s.journal.state().used_intents.is_empty());
        let b = batch(101, descending);
        s.update(&b).unwrap();
        let state = s.journal.state();
        let trade = state
            .active
            .as_ref()
            .expect("new closed bar must enter virtually");
        assert_eq!(
            trade.side,
            if descending { Side::Short } else { Side::Long }
        );
        assert_eq!(trade.entry.status, OrderStatus::Filled);
        assert_eq!(trade.entry.fill.qty, trade.plan.qty);
        assert_eq!(trade.stop_status, Some(Protection::Active));
        assert_eq!(trade.take_profit_status, Some(Protection::Active));
        assert!(trade.plan.budgeted_loss <= D::new(25, 1));
        assert!(trade.entry.fill.notional <= D::from(250));
        assert!(trade.entry.fill.fee > D::ZERO);
        assert!(state.equity < state.config.capital);
        s.update(&b).unwrap();
        assert_eq!(s.journal.state().used_intents.len(), 1);
        assert_eq!(s.journal.state().completed_trades, 0);
    }
}

#[test]
fn tp_and_stop_fill_cleanup_fee_pnl_and_journal_replay() {
    for (descending, target) in [(false, true), (false, false), (true, true), (true, false)] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session");
        let mut s = Session::new(path.clone(), &batch(100, descending)).unwrap();
        let mut b = batch(101, descending);
        s.update(&b).unwrap();
        let trade = s.journal.state().active.as_ref().unwrap().clone();
        let price = if target {
            trade.take_profit
        } else {
            trade.stop
        };
        b.quote.bid = if descending {
            price - D::new(1, 2)
        } else {
            price
        };
        b.quote.ask = b.quote.bid + D::new(1, 2);
        let now = b.now + 2000;
        tick(&mut b, now);
        s.update(&b).unwrap();
        let state = s.journal.state().clone();
        assert!(state.active.is_none());
        assert_eq!(state.completed_trades, 1);
        assert_eq!(state.realized_net > D::ZERO, target);
        assert_eq!(state.equity, state.config.capital + state.realized_net);
        if !target {
            assert!(-state.realized_net <= state.config.risk_per_trade);
        }
        // The same closed bar cannot reopen after exit.
        s.update(&b).unwrap();
        assert_eq!(s.journal.state().used_intents.len(), 1);
        let hash = s.journal.head_hash().to_owned();
        s.request_stop("test complete").unwrap();
        assert_eq!(s.snapshot.phase, Phase::Finished);
        let saved: Snapshot =
            serde_json::from_slice(&fs::read(path.join("status.json")).unwrap()).unwrap();
        assert_eq!(saved.state.unwrap(), state);
        assert!(saved.received.is_none());
        drop(s);
        let replay = Journal::open(&path.join("journal.jsonl")).unwrap();
        assert_eq!(replay.state(), &state);
        assert_eq!(replay.head_hash(), hash);
    }
}

#[test]
fn manual_stop_waits_for_fresh_quote_and_deadline_closes_position() {
    for deadline in [false, true] {
        let dir = tempdir().unwrap();
        let mut s = Session::new(dir.path().join("session"), &batch(100, false)).unwrap();
        let mut b = batch(101, false);
        s.update(&b).unwrap();
        if deadline {
            tick(&mut b, s.snapshot.deadline_ms);
        } else {
            s.request_stop("manual").unwrap();
            assert_eq!(s.snapshot.phase, Phase::Stopping);
            assert!(s.journal.state().active.is_some());
            b.now += 6000;
            assert!(s.update(&b).is_err()); // No virtual fill on stale price.
            b.quote.observed_at_ms = b.now;
        }
        s.update(&b).unwrap();
        assert_eq!(s.snapshot.phase, Phase::Finished);
        assert!(s.journal.state().active.is_none());
        assert_eq!(s.journal.state().completed_trades, 1);
        assert_eq!(s.journal.state().used_intents.len(), 1);
    }
}

#[test]
fn funding_gates_entries_closes_ahead_and_marks_missed_costs() {
    let dir = tempdir().unwrap();
    let mut s = Session::new(dir.path().join("gate"), &batch(100, false)).unwrap();
    let mut b = batch(101, false);
    b.next_funding = b.now + 1_799_999;
    s.update(&b).unwrap();
    assert!(s.journal.state().active.is_none());
    assert!(s.snapshot.reason.contains("funding"));
    for missed in [false, true] {
        let mut s = Session::new(
            dir.path().join(if missed { "missed" } else { "exit" }),
            &batch(100, false),
        )
        .unwrap();
        let mut b = batch(101, false);
        b.next_funding = b.now + 1_900_000;
        s.update(&b).unwrap();
        assert!(s.journal.state().active.is_some());
        let now = if missed {
            b.next_funding + 1
        } else {
            b.next_funding - 300_000
        };
        tick(&mut b, now);
        if missed {
            b.next_funding += 8 * SESSION_MS;
        }
        s.update(&b).unwrap();
        assert!(s.journal.state().active.is_none());
        assert_eq!(s.snapshot.complete_costs, !missed);
        assert_eq!(
            s.snapshot.phase,
            if missed {
                Phase::Finished
            } else {
                Phase::Running
            }
        );
    }
}

#[test]
fn outage_cannot_reenter_and_failed_gap_revision_filter_are_explicit() {
    let dir = tempdir().unwrap();
    let mut s = Session::new(dir.path().join("outage"), &batch(100, false)).unwrap();
    let b = batch(101, false);
    s.update(&b).unwrap();
    s.last_good = Instant::now() - Duration::from_secs(9);
    s.outage("connection unavailable").unwrap();
    assert_eq!(s.snapshot.phase, Phase::Stopping);
    assert!(s.journal.state().active.is_some());
    s.update(&b).unwrap();
    assert_eq!(s.snapshot.phase, Phase::Finished);
    assert!(s.journal.state().active.is_none());
    for case in ["gap", "revision", "filter", "future", "stale"] {
        let mut s = Session::new(dir.path().join(case), &batch(100, false)).unwrap();
        let mut b = batch(101, false);
        match case {
            "gap" => b = batch(102, false),
            "revision" => b.bars[5].volume += D::ONE,
            "filter" => b.filters.step_size *= D::from(2),
            "future" => b.quote.observed_at_ms += 1,
            "stale" => b.quote.observed_at_ms -= 5001,
            _ => unreachable!(),
        }
        let error = s.update(&b).unwrap_err();
        s.fail(&error.to_string());
        assert_eq!(s.snapshot.phase, Phase::Failed);
        assert!(s.snapshot.error.is_some());
        assert!(!s.snapshot.complete_costs);
        assert!(s.journal.state().used_intents.is_empty());
    }
}

#[test]
fn late_bar_and_insufficient_notional_skip_without_inventing_trades() {
    for late in [false, true] {
        let dir = tempdir().unwrap();
        let mut initial = batch(100, false);
        if !late {
            initial.filters.min_notional = D::from(200);
        }
        let mut s = Session::new(dir.path().join("session"), &initial).unwrap();
        let mut b = batch(101, false);
        b.filters = initial.filters;
        if late {
            let now = b.now + 13_000;
            tick(&mut b, now);
        }
        s.update(&b).unwrap();
        assert_eq!(s.snapshot.phase, Phase::Running);
        assert!(s.journal.state().active.is_none());
        assert!(
            s.snapshot
                .reason
                .contains(if late { "terlambat" } else { "sizing" })
        );
    }
}

#[test]
fn startup_crossing_candle_boundary_does_not_enter_on_pre_session_close() {
    let dir = tempdir().unwrap();
    let mut initial = batch(100, false);
    let mut newer = batch(101, false);
    tick(&mut initial, newer.now);
    let mut s = Session::new(dir.path().join("session"), &initial).unwrap();
    let now = newer.now + 2000;
    tick(&mut newer, now);
    s.update(&newer).unwrap();
    assert!(s.journal.state().used_intents.is_empty());
    assert!(s.snapshot.reason.contains("sebelum sesi"));
}
