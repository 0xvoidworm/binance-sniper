//! No credentials/network: production supervisor against a deterministic exchange.
use super::{
    Ready, ledger, margin,
    protocol::*,
    session_store::{Config, Store, digest},
    supervisor::{Engine, Frame},
    transport::{self, Exchange},
};
use crate::{
    account,
    paper::strategy::{self, Bar, INTERVAL},
};
use binance_engine::Side;
use rust_decimal::Decimal as D;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

fn d(s: &str) -> D {
    s.parse().unwrap()
}

#[tokio::test]
async fn review12_consistent_clock_refinement_must_not_stop_flat_session() {
    let mut r = Rig::new(false);
    let t = r.now;
    let at = Instant::now();
    let previous_at = at - Duration::from_millis(8000);
    let previous = account::ClockSample::new(
        t - 8000,
        previous_at - Duration::from_millis(4500),
        previous_at,
    )
    .unwrap();
    let raw_first = account::ClockSample::new(t, at - Duration::from_millis(4500), at).unwrap();
    let first_raw = transport::ClockWindow::from_sample(raw_first, at).unwrap();
    let first_clock = transport::ClockWindow::from_sample(previous, at)
        .unwrap()
        .review12_intersection(first_raw, at);
    let first_depth = json!({
        "lastUpdateId": 7001, "E": t, "T": t,
        "bids": [["100", "1"]], "asks": [["101", "1"]]
    });
    super::quotes::Book::parse_with_clock(&first_depth, first_clock, None).unwrap();
    let mut first = r.broker.frame(&r.bars, t);
    first.ready.clock = first_clock;
    first.ready.context.now = first_clock.latest();
    first.account_as_of = first_clock.latest();
    first.ready.context.quote.observed_at_ms = t - 1;
    first.fresh().unwrap();
    r.engine.tick(&mut r.broker, &first).await.unwrap();
    let old_upper = r.engine.store.state.session.as_ref().unwrap().updated;

    tokio::time::sleep(Duration::from_millis(2500)).await;
    let next_at = Instant::now();
    let elapsed = next_at.duration_since(at).as_millis() as u64;
    let actual = t + elapsed;
    let next_sample =
        account::ClockSample::new(actual, next_at - Duration::from_millis(100), next_at).unwrap();
    let next_clock = first_raw.review12_intersection(
        transport::ClockWindow::from_sample(next_sample, next_at).unwrap(),
        next_at,
    );
    let next_depth = json!({
        "lastUpdateId": 7002, "E": actual, "T": actual,
        "bids": [["100", "1"]], "asks": [["101", "1"]]
    });
    super::quotes::Book::parse_with_clock(&next_depth, next_clock, None).unwrap();
    assert!(next_clock.earliest() > first_clock.earliest());
    assert!(next_clock.latest() < old_upper);
    let mut next = r.broker.frame(&r.bars, actual);
    next.ready.clock = next_clock;
    next.ready.context.now = next_clock.latest();
    next.account_as_of = next_clock.latest();
    next.ready.context.quote.observed_at_ms = actual - 1;
    next.fresh().unwrap();
    let outcome = r.engine.tick(&mut r.broker, &next).await;
    eprintln!(
        "REVIEW12_SHRINK; elapsed_ms={elapsed}; previous_upper={old_upper}; next_lower={}; next_upper={}; outcome={outcome:?}",
        next_clock.earliest(),
        next_clock.latest()
    );
    if let Err(error) = outcome {
        r.engine.check_failed(&error).unwrap();
    }
    assert!(r.broker.commands.is_empty());
    assert!(!r.engine.stopping());
}

#[test]
fn review12_upper_bound_must_enclose_fractional_duration_carry() {
    let t = 1_000_000;
    let started = Instant::now();
    let completed = started + Duration::from_micros(1800);
    let observed = completed + Duration::from_micros(1800);
    let sample = account::ClockSample::new(t, started, completed).unwrap();
    let clock = transport::ClockWindow::from_sample(sample, observed).unwrap();
    let possible_true_ms = t + observed.duration_since(started).as_millis() as u64;
    let result = clock.validate_age("quote", possible_true_ms, 5000);
    eprintln!(
        "REVIEW12_ROUNDING; lower={}; upper={}; possible_true={possible_true_ms}; classification={result:?}",
        clock.earliest(),
        clock.latest()
    );
    assert!(clock.latest() >= possible_true_ms);
}

fn r13_frame_with_clock(r: &mut Rig, clock: transport::ClockWindow) -> Frame {
    let lower = clock.earliest();
    let upper = clock.latest();
    let mut frame = r.broker.frame(&r.bars, lower);
    frame.ready.clock = clock;
    frame.ready.context.now = upper;
    frame.account_as_of = upper;
    frame.ready.context.quote.observed_at_ms = lower;
    frame
}

#[tokio::test]
async fn review13_activation_first_frame_must_not_be_false_clock_rollback() {
    let mut r = Rig::new(false);
    let root = r._dir.path().join("activation");
    let store = Store::open(&root, &digest(b"activation fixture")).unwrap();
    let mut engine = Engine::new(store);
    let acquired = Instant::now();
    let sample =
        account::ClockSample::new(r.now, acquired - Duration::from_millis(100), acquired).unwrap();
    let clock = transport::ClockWindow::from_sample(sample, acquired).unwrap();
    let first = r13_frame_with_clock(&mut r, clock);

    // Production fetches this frame, awaits the extra FeeBurn GET, then begins
    // and consumes the same frame on its first tick.
    tokio::time::sleep(Duration::from_millis(300)).await;
    first.fresh().unwrap();
    engine.begin(config(), &first).unwrap();
    let session = engine.store.state.session.as_ref().unwrap();
    let floor = session.clock_floor_ms.unwrap();
    assert_eq!(floor, first.ready.clock.earliest());
    assert!(session.started > first.ready.clock.latest());
    assert_eq!(
        session.deadline - session.started,
        u64::from(session.config.minutes) * 60_000
    );
    let outcome = engine.tick(&mut r.broker, &first).await;
    eprintln!(
        "REVIEW13_ACTIVATION; acquired_lower={}; acquired_upper={}; begin_floor={floor}; projected_lower={}; outcome={outcome:?}",
        clock.earliest(),
        clock.latest(),
        first.earliest_now()
    );
    outcome.unwrap();
    assert!(r.broker.commands.is_empty());
    assert!(
        !engine.stopping(),
        "a valid first snapshot must not terminally stop a new session"
    );
}

#[tokio::test]
async fn r14_activation_delay_relative_to_uncertainty_keeps_guards_and_deadline() {
    for (delay_ms, expected_fresh) in [(50, true), (300, true), (4500, true), (5101, false)] {
        let mut r = Rig::new(false);
        let root = r._dir.path().join(format!("activation-{delay_ms}"));
        let store =
            Store::open(&root, &digest(format!("activation {delay_ms}").as_bytes())).unwrap();
        let mut engine = Engine::new(store);
        let acquired = Instant::now() - Duration::from_millis(delay_ms);
        let clock = transport::ClockWindow::new(r.now, r.now + 100, acquired).unwrap();
        let first = r13_frame_with_clock(&mut r, clock);
        let result = engine.begin(config(), &first);
        assert_eq!(result.is_ok(), expected_fresh, "delay_ms={delay_ms}");
        if expected_fresh {
            let session = engine.store.state.session.as_ref().unwrap();
            assert_eq!(session.clock_floor_ms, Some(clock.earliest()));
            assert_eq!(
                session.deadline - session.started,
                u64::from(session.config.minutes) * 60_000
            );
            engine.tick(&mut r.broker, &first).await.unwrap();
            assert!(!engine.stopping(), "delay_ms={delay_ms}");
        } else {
            assert!(engine.store.state.session.is_none());
        }
        assert!(r.broker.commands.is_empty());
    }
}

#[tokio::test(start_paused = true)]
async fn review13_recovery_supervisor_handles_fresh_processing_jitter() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r10_flat_recovery(&mut r).await;
    let commands = r.broker.commands.len();
    let base = r.now;
    for i in 0..=40_u64 {
        let processing_ms = if i % 2 == 1 { 500 } else { 0 };
        if i > 0 {
            let observation_gap = if i % 2 == 1 { 2500 } else { 1500 };
            tokio::time::advance(Duration::from_millis(observation_gap)).await;
        }
        let acquired = Instant::now() - Duration::from_millis(processing_ms);
        let acquired_local = tokio::time::Instant::now() - Duration::from_millis(processing_ms);
        let clock = transport::ClockWindow::new_anchored(
            base + i * 2000,
            base + i * 2000 + 100,
            acquired,
            acquired_local,
        )
        .unwrap();
        let frame = r13_frame_with_clock(&mut r, clock);
        frame.fresh().unwrap();
        assert_eq!(
            frame.bars.last().unwrap().close_ms,
            (frame.earliest_now() / INTERVAL * INTERVAL) - 1
        );
        r.engine.tick(&mut r.broker, &frame).await.unwrap();
        assert!(!r.engine.stopping());
        assert_eq!(r.broker.commands.len(), commands);
    }
    eprintln!(
        "REVIEW13_HEALTH_SUPERVISOR; frames=41; elapsed_ms=80000; all_fresh=true; recovering={}; note={}",
        r.engine.recovering_transport(),
        r.engine.note
    );
    assert!(
        !r.engine.recovering_transport(),
        "consistent fresh frames must complete health after 80 seconds, including processing jitter"
    );
}

#[tokio::test]
async fn r13_clock_refinement_with_intent_never_duplicates_entry_or_clears_stop() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 1);

    let at = Instant::now();
    let wide_sample =
        account::ClockSample::new(r.now, at - Duration::from_millis(4500), at).unwrap();
    let wide = transport::ClockWindow::from_sample(wide_sample, at).unwrap();
    let wide_depth = json!({
        "lastUpdateId": 7101, "E": r.now, "T": r.now,
        "bids": [["100", "1"]], "asks": [["101", "1"]]
    });
    super::quotes::Book::parse_with_clock(&wide_depth, wide, None).unwrap();
    let wide_frame = r13_frame_with_clock(&mut r, wide);
    r.engine.tick(&mut r.broker, &wide_frame).await.unwrap();

    let next_at = Instant::now();
    let elapsed = next_at.duration_since(at).as_millis() as u64;
    let narrow_sample = account::ClockSample::new(
        r.now + elapsed,
        next_at - Duration::from_millis(100),
        next_at,
    )
    .unwrap();
    let narrow = wide.review12_intersection(
        transport::ClockWindow::from_sample(narrow_sample, next_at).unwrap(),
        next_at,
    );
    let narrow_depth = json!({
        "lastUpdateId": 7102, "E": r.now + elapsed, "T": r.now + elapsed,
        "bids": [["100", "1"]], "asks": [["101", "1"]]
    });
    super::quotes::Book::parse_with_clock(&narrow_depth, narrow, None).unwrap();
    assert!(narrow.latest() < wide.latest() + 2500);
    let narrow_frame = r13_frame_with_clock(&mut r, narrow);
    r.engine.tick(&mut r.broker, &narrow_frame).await.unwrap();
    assert_eq!(r.broker.entries(), 1);
    assert!(!r.engine.stopping());

    r.engine.stop("fixture terminal stop").unwrap();
    let later = transport::ClockWindow::new(r.now + 7000, r.now + 7100, Instant::now()).unwrap();
    let later_frame = r13_frame_with_clock(&mut r, later);
    r.engine.tick(&mut r.broker, &later_frame).await.unwrap();
    assert!(r.engine.stopping(), "a later fresh range cannot clear Stop");
    assert_eq!(r.broker.entries(), 1);
}

#[tokio::test]
async fn r13_narrow_to_wide_raw_samples_remain_safe_through_parser_and_supervisor() {
    let mut r = Rig::new(false);
    let t = r.now;
    let at = Instant::now();
    let narrow_raw = transport::ClockWindow::from_sample(
        account::ClockSample::new(t, at - Duration::from_millis(100), at).unwrap(),
        at,
    )
    .unwrap();
    let wide_raw = transport::ClockWindow::from_sample(
        account::ClockSample::new(t, at - Duration::from_millis(4500), at).unwrap(),
        at,
    )
    .unwrap();

    // Connector keeps raw samples: narrow∩wide is narrow for one frame, then
    // wide∩wide may legitimately widen the accepted frame on the next cycle.
    let refined = narrow_raw.review12_intersection(wide_raw, at);
    let widened = wide_raw.review12_intersection(wide_raw, at);
    assert!(widened.latest() > refined.latest());
    for (id, clock) in [(7201, refined), (7202, widened)] {
        let depth = json!({
            "lastUpdateId": id, "E": t, "T": t,
            "bids": [["100", "1"]], "asks": [["101", "1"]]
        });
        super::quotes::Book::parse_with_clock(&depth, clock, None).unwrap();
        let frame = r13_frame_with_clock(&mut r, clock);
        r.engine.tick(&mut r.broker, &frame).await.unwrap();
    }
    assert!(!r.engine.stopping());
    assert!(r.broker.commands.is_empty());
}

#[tokio::test]
async fn r13_durable_lower_bound_detects_real_rollback_after_restart() {
    let mut r = Rig::new(false);
    r.now += 2000;
    r.tick().await.unwrap();
    let floor = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .clock_floor_ms
        .unwrap();
    r.restart();
    let old = floor - 1;
    let frame = r.broker.frame(&r.bars, old);
    let error = r.engine.tick(&mut r.broker, &frame).await.unwrap_err();
    assert!(error.downcast_ref::<transport::ClockRollback>().is_some());
    r.engine.check_failed(&error).unwrap();
    assert!(r.engine.stopping());
    assert!(r.broker.commands.is_empty());
}

#[tokio::test]
async fn r13_accounting_wait_uses_current_lower_against_saved_upper() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.engine.stop("fixture accounting wait").unwrap();
    r.bars.clear();
    for _ in 0..12 {
        r.tick().await.unwrap();
        if r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .as_ref()
            .and_then(|trade| trade.flat_ms)
            .is_some()
        {
            break;
        }
    }
    let flat = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .current
        .as_ref()
        .unwrap()
        .flat_ms
        .unwrap();

    // Latest appears over eight seconds later, but the lower bound proves only
    // four seconds. Accounting must not query/accept a settlement yet.
    let ambiguous = transport::ClockWindow::new(flat + 4000, flat + 8000, Instant::now()).unwrap();
    let ambiguous_frame = r13_frame_with_clock(&mut r, ambiguous);
    r.engine
        .tick(&mut r.broker, &ambiguous_frame)
        .await
        .unwrap();
    let session = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(session.completed, 0);
    assert!(session.current.as_ref().unwrap().settlement.is_none());

    let proven = transport::ClockWindow::new(flat + 5000, flat + 9000, Instant::now()).unwrap();
    let proven_frame = r13_frame_with_clock(&mut r, proven);
    r.engine.tick(&mut r.broker, &proven_frame).await.unwrap();
    assert!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .settlement
            .is_some()
    );
    assert_eq!(r.broker.entries(), 1);
}

fn r12_clock_depth_error(
    now: u64,
    uncertainty_ms: u64,
    event_offset_ms: u64,
    transaction_offset_ms: u64,
) -> anyhow::Error {
    let completed = Instant::now();
    let sample = account::ClockSample::new(
        now,
        completed - Duration::from_millis(uncertainty_ms),
        completed,
    )
    .unwrap();
    let clock = transport::ClockWindow::from_sample(sample, completed).unwrap();
    let value = json!({
        "lastUpdateId":12345,
        "E":now + event_offset_ms,
        "T":now + transaction_offset_ms,
        "bids":[["100","1"]],
        "asks":[["101","1"]]
    });
    super::quotes::Book::parse_with_clock(&value, clock, None).unwrap_err()
}

#[tokio::test]
async fn r12_request_body_clock_range_depth_and_supervisor_revalidate_without_order() {
    let mut r = Rig::new(false);
    r.signal(false);
    let deadline = r.engine.store.state.session.as_ref().unwrap().deadline;

    // Exact production incident offsets. The 120ms request/body duration is a
    // deterministic fault injection, not a measured production one-way delay.
    let error = r12_clock_depth_error(r.now, 120, 17, 14);
    assert!(error.downcast_ref::<transport::ClockUncertain>().is_some());
    assert!(super::recovery::transient(&error));
    r.engine.read_failed(&error).unwrap();
    assert!(!r.engine.stopping());
    r.tick().await.unwrap();
    assert!(!r.engine.finished());
    assert!(r.broker.commands.is_empty());
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().deadline,
        deadline
    );

    // A healthy replacement frame consumes the recovery candle. Only a later
    // candle may become a new signal.
    for _ in 0..3 {
        r.tick().await.unwrap();
    }
    assert!(r.broker.commands.is_empty());
    r.bars = history(101, false);
    r.bars.last_mut().unwrap().volume = D::from(2);
    r.broker.price = r.bars.last().unwrap().close;
    r.now = r.bars.last().unwrap().close_ms + 1;
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 1);
}

#[tokio::test]
async fn r12_clock_uncertainty_with_intent_enters_bounded_close_only_recovery() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    assert!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .is_some()
    );
    let error = r12_clock_depth_error(r.now, 400, 350, 347);
    r.engine.read_failed(&error).unwrap();
    let session = r.engine.store.state.session.as_ref().unwrap();
    assert!(r.engine.close_only() && !session.stop);
    assert_eq!(session.recovery.as_ref().unwrap().episodes, 1);
    r.engine.read_failed(&error).unwrap();
    assert_eq!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .recovery
            .as_ref()
            .unwrap()
            .episodes,
        1,
        "repeated frames in one disturbance cannot spend another episode"
    );
}

#[tokio::test]
async fn r12_invalid_future_and_clock_jump_are_terminal_and_stop_never_revives() {
    let mut future = Rig::new(false);
    let error = r12_clock_depth_error(future.now, 120, 121, 118);
    assert!(
        error
            .downcast_ref::<transport::InvalidTimestamp>()
            .is_some()
    );
    assert!(!super::recovery::transient(&error));
    future.engine.read_failed(&error).unwrap();
    future.tick().await.unwrap();
    assert!(future.engine.finished());
    assert!(future.broker.commands.is_empty());
    let uncertain = r12_clock_depth_error(future.now, 120, 17, 14);
    future.engine.read_failed(&uncertain).unwrap();
    future.tick().await.unwrap();
    assert!(
        future.engine.finished(),
        "fresh frames cannot clear terminal Stop"
    );

    let mut jumped = Rig::new(false);
    let updated = jumped.engine.store.state.session.as_ref().unwrap().updated;
    let old_now = updated.saturating_sub(1000);
    let mut frame = jumped.broker.frame(&jumped.bars, old_now);
    frame.account_as_of = old_now;
    frame.ready.context.quote.observed_at_ms = old_now;
    let error = jumped
        .engine
        .tick(&mut jumped.broker, &frame)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("bergerak mundur"));
    jumped.engine.check_failed(&error).unwrap();
    assert!(jumped.engine.stopping());
}

#[test]
fn r12_freshness_boundaries_distinguish_stale_uncertain_and_invalid() {
    let at = Instant::now();
    let exact = transport::ClockWindow::new(10_000, 10_000, at).unwrap();
    assert!(exact.validate_age("depth", 5_000, 5_000).is_ok());
    let stale = exact.validate_age("depth", 4_999, 5_000).unwrap_err();
    assert!(stale.downcast_ref::<transport::StaleTimestamp>().is_some());
    let ranged = transport::ClockWindow::new(10_000, 10_120, at).unwrap();
    let uncertain = ranged.validate_age("depth", 5_001, 5_000).unwrap_err();
    assert!(
        uncertain
            .downcast_ref::<transport::ClockUncertain>()
            .is_some()
    );
    let future = ranged.validate_age("depth", 10_121, 5_000).unwrap_err();
    assert!(
        future
            .downcast_ref::<transport::InvalidTimestamp>()
            .is_some()
    );
}

#[tokio::test]
async fn r12_utc_boundary_revalidates_and_deadline_uses_conservative_latest_bound() {
    let mut utc = Rig::new(false);
    let boundary = 86_400_000;
    let lower = boundary - 50;
    let mut frame = utc.broker.frame(&utc.bars, lower);
    frame.ready.clock = transport::ClockWindow::new(lower, boundary + 50, Instant::now()).unwrap();
    frame.ready.context.now = boundary + 50;
    frame.ready.context.quote.observed_at_ms = lower;
    frame.account_as_of = lower;
    let error = frame.fresh().unwrap_err();
    assert!(error.downcast_ref::<transport::ClockUncertain>().is_some());
    utc.engine.check_failed(&error).unwrap();
    assert!(!utc.engine.stopping());

    let mut deadline = Rig::new(false);
    let lower = deadline.now;
    let upper = lower + 50;
    deadline
        .engine
        .store
        .state
        .session
        .as_mut()
        .unwrap()
        .deadline = upper;
    let mut frame = deadline.broker.frame(&deadline.bars, lower);
    frame.ready.clock = transport::ClockWindow::new(lower, upper, Instant::now()).unwrap();
    frame.ready.context.now = upper;
    frame.ready.context.quote.observed_at_ms = lower;
    frame.account_as_of = lower;
    deadline
        .engine
        .tick(&mut deadline.broker, &frame)
        .await
        .unwrap();
    assert!(deadline.engine.finished());
    assert_eq!(
        deadline
            .engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .stop_code,
        Some(super::exit::Reason::SessionDeadline)
    );
    assert!(deadline.broker.commands.is_empty());
}

fn r11_stale_depth(now: u64) -> anyhow::Error {
    let value = json!({"lastUpdateId":12345,"E":now-7265,"T":now-7267,
        "bids":[["100","1"]],"asks":[["101","1"]]});
    super::quotes::Book::parse(&value, now, None).unwrap_err()
}

#[tokio::test]
async fn r11_actual_stale_depth_flat_pauses_consumes_recovery_bar_and_waits_new_signal() {
    let mut r = Rig::new(false);
    r.signal(false);
    let deadline = r.engine.store.state.session.as_ref().unwrap().deadline;
    r.engine.read_failed(&r11_stale_depth(r.now)).unwrap();
    assert!(
        !r.engine.stopping(),
        "stale depth while flat must pause, not latch Stop"
    );
    r.tick().await.unwrap();
    for _ in 0..4 {
        r.tick().await.unwrap();
    }
    assert!(!r.engine.finished());
    assert!(
        r.broker.commands.is_empty(),
        "recovered signal must not be chased"
    );
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(s.deadline, deadline);
    assert_eq!(s.recovery.as_ref().unwrap().episodes, 0);
    r.bars = history(101, false);
    r.bars.last_mut().unwrap().volume = D::from(2);
    r.broker.price = r.bars.last().unwrap().close;
    r.now = r.bars.last().unwrap().close_ms + 1;
    r.tick().await.unwrap();
    assert_eq!(
        r.broker.entries(),
        1,
        "only a genuinely new valid signal may enter"
    );
}

#[tokio::test(start_paused = true)]
async fn r11_stale_exposure_closes_accounts_waits_and_later_flat_staleness_is_not_terminal() {
    for uncertain in [false, true] {
        let mut r = Rig::new(false);
        r.broker.lose_entry = uncertain;
        r.broker.residual = uncertain;
        r.signal(false);
        r.tick().await.unwrap();
        r.engine.read_failed(&r11_stale_depth(r.now)).unwrap();
        assert!(r.engine.close_only() && !r.engine.stopping());
        for _ in 0..60 {
            r.tick().await.unwrap();
            if r.engine
                .store
                .state
                .session
                .as_ref()
                .unwrap()
                .current
                .is_none()
            {
                break;
            }
        }
        let s = r.engine.store.state.session.as_ref().unwrap();
        assert!(s.current.is_none() && !s.finished && !s.stop);
        assert_eq!(s.completed, 1);
        assert_eq!(s.net, r.broker.wallet - d("1000"));
        r.tick().await.unwrap();
        for _ in 0..29 {
            r10_healthy_step(&mut r).await;
        }
        assert!(r.engine.recovering_transport());
        // A stale account during cooldown resets health, not Stop or episode count.
        let mut frame = r.broker.frame(&r.bars, r.now);
        frame.account_as_of = r.now - 5001;
        let error = r.engine.tick(&mut r.broker, &frame).await.unwrap_err();
        r.engine.check_failed(&error).unwrap();
        assert!(!r.engine.stopping());
        assert_eq!(
            r.engine
                .store
                .state
                .session
                .as_ref()
                .unwrap()
                .recovery
                .as_ref()
                .unwrap()
                .episodes,
            1
        );
        r.tick().await.unwrap();
        for _ in 0..29 {
            r10_healthy_step(&mut r).await;
        }
        assert!(r.engine.recovering_transport());
        r10_healthy_step(&mut r).await;
        assert!(!r.engine.recovering_transport());
        // Production failure: recovery completed, then a later stale depth while flat.
        r.engine.read_failed(&r11_stale_depth(r.now)).unwrap();
        r.tick().await.unwrap();
        assert!(!r.engine.stopping() && !r.engine.finished());
        assert_eq!(r.broker.entries(), 1);
        assert_eq!(
            r.engine
                .store
                .state
                .session
                .as_ref()
                .unwrap()
                .recovery
                .as_ref()
                .unwrap()
                .episodes,
            1
        );
    }
}

#[tokio::test]
async fn r11_repeated_flat_stale_reads_never_extend_deadline_or_override_stop() {
    let mut r = Rig::new(false);
    let deadline = r.engine.store.state.session.as_ref().unwrap().deadline;
    for _ in 0..200 {
        r.engine.read_failed(&r11_stale_depth(r.now)).unwrap();
    }
    assert!(!r.engine.stopping());
    assert_eq!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .recovery
            .as_ref()
            .unwrap()
            .episodes,
        0
    );
    r.now = deadline;
    r.tick().await.unwrap();
    assert!(r.engine.finished());
    r.engine.read_failed(&r11_stale_depth(r.now)).unwrap();
    assert!(r.engine.stopping() && r.engine.finished());
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().stop_code,
        Some(super::exit::Reason::SessionDeadline)
    );
    assert!(r.broker.commands.is_empty());
}

#[tokio::test]
async fn r11_expired_clock_and_stale_frame_pause_flat_but_future_clock_stops() {
    for mode in ["clock-lease", "account", "quote", "future"] {
        let mut r = Rig::new(false);
        r.signal(false);
        let commands = r.broker.commands.len();
        let error = if mode == "clock-lease" {
            transport::Error::ReadWindowExpired.into()
        } else {
            let mut frame = r.broker.frame(&r.bars, r.now);
            match mode {
                "account" => frame.account_as_of = r.now - 5001,
                "quote" => frame.ready.context.quote.observed_at_ms = r.now - 5001,
                "future" => frame.account_as_of = r.now + 5000,
                _ => unreachable!(),
            }
            r.engine.tick(&mut r.broker, &frame).await.unwrap_err()
        };
        r.engine.check_failed(&error).unwrap();
        assert_eq!(r.engine.stopping(), mode == "future", "{mode}: {error}");
        r.tick().await.unwrap();
        assert_eq!(r.engine.finished(), mode == "future");
        assert_eq!(r.broker.commands.len(), commands);
    }
}

async fn r10_flat_recovery(r: &mut Rig) {
    r.engine
        .read_failed(&transport::Error::Network.into())
        .unwrap();
    assert!(r.engine.close_only());
    for _ in 0..60 {
        r.tick().await.unwrap();
        if r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .is_none()
        {
            break;
        }
    }
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert!(
        s.current.is_none() && !s.stop && !s.finished,
        "{}",
        s.reason
    );
    assert!(r.engine.recovering_transport());
    assert!(
        !r.engine.close_only(),
        "flat recovery needs signal/history reads, never entry"
    );
    assert_eq!(s.net, r.broker.wallet - d("1000"));
}

async fn r10_healthy_step(r: &mut Rig) {
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    r.now += 1000; // tick advances the other second.
    r.tick().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn r10_recovery_waits_sixty_seconds_resets_on_error_and_reenters_only_new_bar() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    let deadline = r.engine.store.state.session.as_ref().unwrap().deadline;
    r10_flat_recovery(&mut r).await;
    r.tick().await.unwrap(); // First healthy flat snapshot starts proof.
    for _ in 0..29 {
        r10_healthy_step(&mut r).await;
    }
    assert!(r.engine.recovering_transport(), "58 seconds cannot resume");
    r.engine
        .read_failed(&transport::Error::Network.into())
        .unwrap();
    assert_eq!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .recovery
            .as_ref()
            .unwrap()
            .episodes,
        1
    );
    r.tick().await.unwrap();
    for _ in 0..29 {
        r10_healthy_step(&mut r).await;
    }
    assert!(
        r.engine.recovering_transport(),
        "new failure resets the full proof"
    );
    assert_eq!(r.broker.entries(), 1);
    r10_healthy_step(&mut r).await;
    assert!(!r.engine.recovering_transport());
    for _ in 0..4 {
        r.tick().await.unwrap();
    }
    assert_eq!(r.broker.entries(), 1, "recovered candle must not be chased");
    r.bars = history(101, false);
    r.bars.last_mut().unwrap().volume = D::from(2);
    r.broker.price = r.bars.last().unwrap().close;
    r.now = r.bars.last().unwrap().close_ms + 1;
    r.tick().await.unwrap();
    assert_eq!(
        r.broker.entries(),
        2,
        "a genuinely new valid candle can enter"
    );
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().deadline,
        deadline
    );
}

#[tokio::test(start_paused = true)]
async fn r10_third_exposure_episode_stops_but_repeated_errors_do_not_spend_extra_episodes() {
    let mut r = Rig::new(false);
    for episode in 1..=3 {
        r.bars = history(99 + episode, false);
        r.bars.last_mut().unwrap().volume = D::from(2);
        r.broker.price = r.bars.last().unwrap().close;
        r.now = r.bars.last().unwrap().close_ms + 1;
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        assert_eq!(r.broker.entries(), episode);
        if episode == 3 {
            r.engine
                .read_failed(&transport::Error::Network.into())
                .unwrap();
            assert!(r.engine.stopping());
            for _ in 0..60 {
                r.tick().await.unwrap();
                if r.engine.finished() {
                    break;
                }
            }
            assert!(r.engine.finished());
        } else {
            r10_flat_recovery(&mut r).await;
            for _ in 0..5 {
                r.engine
                    .read_failed(&transport::Error::Network.into())
                    .unwrap();
            }
            assert_eq!(
                usize::from(
                    r.engine
                        .store
                        .state
                        .session
                        .as_ref()
                        .unwrap()
                        .recovery
                        .as_ref()
                        .unwrap()
                        .episodes
                ),
                episode
            );
            r.tick().await.unwrap();
            for _ in 0..30 {
                r10_healthy_step(&mut r).await;
            }
            assert!(!r.engine.recovering_transport());
        }
    }
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(s.completed, 3);
    assert_eq!(s.recovery.as_ref().unwrap().episodes, 2);
    assert_eq!(s.stop_code, Some(super::exit::Reason::DataFailure));
}

#[tokio::test(start_paused = true)]
async fn r10_terminal_reasons_never_resume_during_close_or_healthy_wait() {
    use super::exit::Reason;
    for flat in [false, true] {
        for reason in [
            Reason::UserStop,
            Reason::WindowClosed,
            Reason::SessionDeadline,
            Reason::SessionLoss,
            Reason::DailyLoss,
            Reason::TradeLimit,
            Reason::SequenceLimit,
            Reason::ProtectionFailure,
            Reason::AccountChanged,
        ] {
            let mut r = Rig::new(false);
            r.signal(false);
            r.tick().await.unwrap();
            r.tick().await.unwrap();
            if flat {
                r10_flat_recovery(&mut r).await;
            } else {
                r.engine
                    .read_failed(&transport::Error::Network.into())
                    .unwrap();
            }
            r.engine.stop_with(reason, "terminal fixture").unwrap();
            r.engine.read_failed(&r11_stale_depth(r.now)).unwrap();
            r.engine
                .read_failed(&transport::Error::Network.into())
                .unwrap();
            for _ in 0..60 {
                r.tick().await.unwrap();
                if r.engine.finished() {
                    break;
                }
            }
            let s = r.engine.store.state.session.as_ref().unwrap();
            assert!(
                s.finished && s.stop && !s.recovery.as_ref().unwrap().active,
                "{reason:?}, flat={flat}"
            );
            assert_eq!(s.stop_code, Some(reason));
            assert_eq!(r.broker.entries(), 1);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn r10_restart_and_legacy_state_have_no_automatic_reentry_authority() {
    for flat in [false, true] {
        let mut r = Rig::new(false);
        r.signal(false);
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        if flat {
            r10_flat_recovery(&mut r).await;
        } else {
            r.engine
                .read_failed(&transport::Error::Network.into())
                .unwrap();
        }
        r.restart();
        assert!(r.engine.stopping() && !r.engine.recovering_transport());
        assert_eq!(
            r.engine
                .store
                .state
                .session
                .as_ref()
                .unwrap()
                .recovery
                .as_ref()
                .unwrap()
                .episodes,
            1
        );
        for _ in 0..60 {
            r.tick().await.unwrap();
            if r.engine.finished() {
                break;
            }
        }
        assert!(r.engine.finished());
        assert_eq!(r.broker.entries(), 1);
    }
    let mut r = Rig::new(false);
    r.engine.store.state.session.as_mut().unwrap().recovery = None;
    r.signal(false);
    r.tick().await.unwrap();
    r.engine
        .read_failed(&transport::Error::Network.into())
        .unwrap();
    assert!(r.engine.stopping() && !r.engine.recovering_transport());
}

#[tokio::test]
async fn r10_unclassified_auth_schema_rate_and_uncertain_failures_stop_even_flat() {
    for entered in [false, true] {
        for error in [
            transport::Error::Unknown,
            transport::Error::Schema,
            transport::Error::Clock,
            transport::Error::RateLimit,
            transport::Error::Preparation(account::Error::Auth),
        ] {
            let mut r = Rig::new(false);
            if entered {
                r.signal(false);
                r.tick().await.unwrap();
            }
            r.engine.read_failed(&error.into()).unwrap();
            assert!(
                r.engine.stopping() && !r.engine.recovering_transport(),
                "{error:?}"
            );
        }
        let mut r = Rig::new(false);
        if entered {
            r.signal(false);
            r.tick().await.unwrap();
        }
        r.engine
            .check_failed(&anyhow::anyhow!("synthetic integrity error"))
            .unwrap();
        assert!(r.engine.stopping());
    }
}

#[tokio::test(start_paused = true)]
async fn r10_recovery_partial_residual_and_uncertain_receipts_never_duplicate_entry() {
    for mode in ["entry-response", "stop-response", "partial", "residual"] {
        let mut r = Rig::new(false);
        r.broker.lose_entry = mode == "entry-response";
        r.broker.lose_stop = mode == "stop-response";
        r.broker.partial = mode == "partial";
        r.broker.residual = mode == "residual";
        r.signal(false);
        r.tick().await.unwrap();
        r10_flat_recovery(&mut r).await;
        assert_eq!(r.broker.entries(), 1, "{mode}");
        assert!(r.broker.qty.is_zero());
        assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
        r.tick().await.unwrap();
        for _ in 0..30 {
            r10_healthy_step(&mut r).await;
        }
        assert!(!r.engine.recovering_transport());
        assert_eq!(r.broker.entries(), 1, "{mode}");
    }
}

#[tokio::test(start_paused = true)]
async fn r10_recovery_rechecks_actual_deadline_risk_foreign_orders_wallet_and_freshness() {
    for mode in [
        "deadline",
        "session-loss",
        "daily-loss",
        "trade-limit",
        "foreign-order",
        "wallet",
        "future",
        "leverage",
        "margin",
    ] {
        let mut r = Rig::new(false);
        r.signal(false);
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        r10_flat_recovery(&mut r).await;
        r.tick().await.unwrap();
        let s = r.engine.store.state.session.as_mut().unwrap();
        match mode {
            "deadline" => r.now = s.deadline,
            "session-loss" => s.config.session_loss = d("0.01"),
            "daily-loss" => s.config.daily_loss = d("0.01"),
            "trade-limit" => s.config.max_trades = 1,
            _ => (),
        }
        let mut f = r.broker.frame(&r.bars, r.now);
        match mode {
            "foreign-order" => f.regular.push(("foreign".into(), 777)),
            "wallet" => f.ready.account.balances.total_wallet_balance += D::ONE,
            "future" => f.account_as_of = r.now + 6000,
            "leverage" => f.ready.current_leverage += 1,
            "margin" => f.ready.isolated = false,
            _ => (),
        }
        if let Err(error) = r.engine.tick(&mut r.broker, &f).await {
            r.engine.check_failed(&error).unwrap();
        }
        assert!(
            r.engine.stopping() && !r.engine.recovering_transport(),
            "{mode}: {}",
            r.engine.note
        );
        assert_eq!(r.broker.entries(), 1);
    }
}

#[tokio::test(start_paused = true)]
async fn r10_transient_transport_closes_but_preserves_session() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 1);
    tokio::time::advance(std::time::Duration::from_millis(15_099)).await;
    r.now += 15_099;
    r.engine
        .read_failed(&transport::Error::Network.into())
        .unwrap();
    assert!(
        !r.engine.stopping(),
        "A classified transient read must close exposure without latching the session Stop"
    );
    assert!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .close
    );
    for _ in 0..40 {
        r.tick().await.unwrap();
        if r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .is_none()
        {
            break;
        }
    }
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(s.completed, 1);
    assert!(s.current.is_none() && !s.finished && !s.stop);
    assert_eq!(r.broker.entries(), 1);
}

#[tokio::test(start_paused = true)]
async fn r10_ledger_read_outage_cannot_skip_accounting_or_duplicate_costs() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.engine
        .read_failed(&transport::Error::Network.into())
        .unwrap();
    r.broker.ledger_network = true;
    let mut failed = 0;
    for _ in 0..35 {
        if let Err(e) = r.tick().await {
            assert!(super::recovery::transient(&e));
            r.engine.check_failed(&e).unwrap();
            failed += 1;
        }
    }
    assert!(failed > 0);
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(s.completed, 0);
    assert!(s.current.is_some() && !s.stop && r.engine.close_only());
    assert_eq!(s.recovery.as_ref().unwrap().episodes, 1);
    assert_eq!(r.broker.entries(), 1);
    r.broker.ledger_network = false;
    for _ in 0..25 {
        r.tick().await.unwrap();
    }
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert!(s.current.is_none() && !s.stop && !s.finished);
    assert_eq!(s.completed, 1);
    assert_eq!(s.net, r.broker.wallet - d("1000"));
    assert_eq!(s.fees, -s.net);
    assert_eq!(r.broker.entries(), 1);
    assert!(r.engine.recovering_transport());
}

#[tokio::test(start_paused = true)]
async fn r10_uncertain_protection_during_recovery_is_terminal_and_never_resends_entry() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.engine
        .read_failed(&transport::Error::Network.into())
        .unwrap();
    r.broker.fail_reads = true;
    if let Err(e) = r.tick().await {
        r.engine.check_failed(&e).unwrap();
    }
    assert!(r.engine.stopping() && !r.engine.recovering_transport());
    r.broker.fail_reads = false;
    for _ in 0..60 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().stop_code,
        Some(super::exit::Reason::ProtectionFailure)
    );
}

#[tokio::test(start_paused = true)]
async fn r10_history_revalidation_restarts_health_and_does_not_consume_an_episode() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r10_flat_recovery(&mut r).await;
    r.tick().await.unwrap();
    for _ in 0..29 {
        r10_healthy_step(&mut r).await;
    }
    let mut f = r.broker.frame(&r.bars, r.now);
    f.skip_signal = Some("history revision".into());
    r.engine.tick(&mut r.broker, &f).await.unwrap();
    r.tick().await.unwrap();
    for _ in 0..29 {
        r10_healthy_step(&mut r).await;
    }
    assert!(r.engine.recovering_transport());
    r10_healthy_step(&mut r).await;
    assert!(!r.engine.recovering_transport());
    assert_eq!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .recovery
            .as_ref()
            .unwrap()
            .episodes,
        1
    );
    assert_eq!(r.broker.entries(), 1);
}

#[test]
fn r10_recovery_metadata_rejects_invalid_state_and_legacy_bytes_roundtrip() {
    let r = Rig::new(false);
    let mut state = r.engine.store.state.clone();
    for (episodes, active, stop, finished) in [
        (3, false, false, false),
        (0, true, false, false),
        (1, true, true, false),
        (1, true, false, true),
    ] {
        let s = state.session.as_mut().unwrap();
        s.recovery = Some(super::recovery::State { episodes, active });
        s.stop = stop;
        s.finished = finished;
        let hash = digest(&serde_json::to_vec(&state).unwrap());
        let encoded = serde_json::to_vec(&json!({"schema":1,"state":state,"hash":hash})).unwrap();
        assert!(super::session_store::decode_saved(&encoded).is_err());
    }
    let s = state.session.as_mut().unwrap();
    s.recovery = None;
    s.stop = false;
    s.finished = false;
    let bytes = serde_json::to_vec(&state).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("\"recovery\""));
    let hash = digest(&bytes);
    let encoded = serde_json::to_vec(&json!({"schema":1,"state":state,"hash":hash})).unwrap();
    let decoded = super::session_store::decode_saved(&encoded).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
}

#[tokio::test(start_paused = true)]
async fn r10_actual_history_revision_pauses_flat_but_cannot_resume_exposure_or_stop() {
    for mode in ["flat", "cooldown", "exposure", "stopped"] {
        let mut r = Rig::new(false);
        if mode != "flat" {
            r.signal(false);
            r.tick().await.unwrap();
            r.tick().await.unwrap();
        }
        if mode == "cooldown" {
            r10_flat_recovery(&mut r).await;
        }
        if mode == "stopped" {
            r.engine.stop("explicit stop").unwrap();
        }
        let mut h = super::session_input::History::default();
        h.accept(r.bars.clone(), r.now).unwrap();
        let mut revised = r.bars.clone();
        revised.last_mut().unwrap().close -= d("0.01");
        let e = h.accept(revised, r.now + 2000).unwrap_err();
        assert!(
            e.downcast_ref::<super::session_input::HistoryRevalidation>()
                .is_some()
        );
        assert!(!super::recovery::transient(&e));
        r.engine.read_failed(&e).unwrap();
        assert_eq!(
            r.engine.stopping(),
            matches!(mode, "exposure" | "stopped"),
            "{mode}"
        );
        if mode == "cooldown" {
            assert!(r.engine.recovering_transport());
        }
    }
}

#[tokio::test]
async fn p3_actual_decision_counters_survive_restart_and_do_not_count_polling() {
    use super::diagnostics::Gate;
    let mut r = Rig::new(false);
    for _ in 0..8 {
        r.tick().await.unwrap();
    }
    assert_eq!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .diagnostics
            .as_ref()
            .unwrap()
            .evaluated_candles(),
        0
    );
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    let d = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .diagnostics
        .clone()
        .unwrap();
    assert_eq!(d.evaluated_candles(), 1);
    assert_eq!(d.counts[Gate::Signal as usize], 1);
    assert_eq!(d.counts[Gate::Intent as usize], 1);
    r.engine
        .data_failed("Snapshot account/posisi usang; account_age_ms=5407; quote_age_ms=201")
        .unwrap();
    assert!(r.engine.stopping());
    r.restart();
    for _ in 0..40 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert!(s.finished && s.current.is_none());
    assert_eq!(
        s.diagnostics.as_ref().unwrap().counts[Gate::DataFailure as usize],
        1
    );
    assert_eq!(s.diagnostics.as_ref().unwrap().evaluated_candles(), 1);
    assert_eq!(r.broker.entries(), 1);
    assert_eq!(s.completed, 1);
    let saved = super::live::Snapshot::saved(&r.engine.store.root);
    assert_eq!(saved.diagnostics, s.diagnostics);
}

#[tokio::test]
async fn p3_recovery_skip_count_is_not_multiplied_by_repeated_polls() {
    use super::diagnostics::Gate;
    let mut r = Rig::new(false);
    r.signal(false);
    for _ in 0..5 {
        let mut frame = r.broker.frame(&r.bars, r.now);
        frame.skip_signal = Some("fixture revalidation".into());
        r.engine.tick(&mut r.broker, &frame).await.unwrap();
        r.now += 1000;
    }
    let d = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .diagnostics
        .as_ref()
        .unwrap();
    assert_eq!(d.counts[Gate::RecoverySkip as usize], 1);
    assert_eq!(d.evaluated_candles(), 0);
    assert!(r.broker.commands.is_empty());
}
fn history(count: usize, short: bool) -> Vec<Bar> {
    let mut bars = strategy::tests::history(count, short);
    if short {
        for bar in &mut bars {
            bar.open -= D::from(700);
            bar.close -= D::from(700);
            bar.high -= D::from(700);
            bar.low -= D::from(700);
        }
    }
    bars
}
fn config() -> Config {
    Config {
        budget: margin::Budget {
            margin: d("50"),
            risk: d("3"),
            max_notional: d("300"),
            leverage: 10,
            slippage_bps: D::ONE,
            liquidation_buffer_bps: d("10"),
            funding_reserve: D::ZERO,
        },
        daily_loss: d("10"),
        session_loss: d("10"),
        minutes: 1440,
        max_trades: 10,
    }
}
struct Broker {
    orders: BTreeMap<String, Value>,
    algos: BTreeMap<String, Value>,
    fills: BTreeMap<u64, Vec<Value>>,
    funding: Vec<Value>,
    commands: Vec<Command>,
    next_id: u64,
    now: u64,
    price: D,
    qty: D,
    entry: D,
    wallet: D,
    isolated: bool,
    leverage: u32,
    lose_entry: bool,
    lose_stop: bool,
    reject_stop: bool,
    partial: bool,
    residual: bool,
    fail_reads: bool,
    fee_burn: bool,
    missing_fills: bool,
    ledger_network: bool,
    mutation_without_quote: bool,
    reconcile_delay: std::time::Duration,
    snapshot_lag_ms: u64,
}
impl Default for Broker {
    fn default() -> Self {
        Self {
            orders: BTreeMap::new(),
            algos: BTreeMap::new(),
            fills: BTreeMap::new(),
            funding: vec![],
            commands: vec![],
            next_id: 100,
            now: 0,
            price: d("100"),
            qty: D::ZERO,
            entry: D::ZERO,
            wallet: d("1000"),
            isolated: true,
            leverage: 10,
            lose_entry: false,
            lose_stop: false,
            reject_stop: false,
            partial: false,
            residual: false,
            fail_reads: false,
            fee_burn: false,
            missing_fills: false,
            ledger_network: false,
            mutation_without_quote: false,
            reconcile_delay: std::time::Duration::ZERO,
            snapshot_lag_ms: 0,
        }
    }
}
impl Broker {
    fn frame(&mut self, bars: &[Bar], now: u64) -> Frame {
        self.now = now;
        let mut ctx = super::tests::context();
        ctx.now = now;
        ctx.quote.observed_at_ms = now;
        ctx.quote.bid = self.price;
        ctx.quote.ask = self.price;
        ctx.mark = self.price;
        ctx.available = self.wallet;
        let mut account = account::tests::example();
        account.server_ms = now;
        account.balances.total_wallet_balance = self.wallet;
        account.balances.available_balance = self.wallet;
        account.config.can_trade = true;
        account.config.dual_side_position = false;
        account.config.multi_assets_margin = false;
        account.symbols = vec![account::SymbolConfig {
            symbol: "BTCUSDT".into(),
            margin_type: if self.isolated { "ISOLATED" } else { "CROSSED" }.into(),
            leverage: self.leverage,
            is_auto_add_margin: false,
        }];
        account.positions = if self.qty.is_zero() {
            vec![]
        } else {
            vec![account::Position {
                symbol: "BTCUSDT".into(),
                position_side: "BOTH".into(),
                position_amt: self.qty,
                entry_price: self.entry,
                mark_price: self.price,
                unrealized_profit: (self.price - self.entry) * self.qty,
                liquidation_price: D::ZERO,
                margin_asset: "USDT".into(),
                initial_margin: D::ONE,
                maint_margin: D::ZERO,
            }]
        };
        Frame {
            ready: Ready {
                account,
                context: ctx,
                current_leverage: self.leverage,
                isolated: self.isolated,
                blockers: vec![],
                next_funding: 86_400_000,
                clock: transport::ClockWindow::new(now, now, Instant::now()).unwrap(),
            },
            bars: bars.to_vec(),
            skip_signal: None,
            regular: self
                .orders
                .iter()
                .filter(|(_, o)| o["status"] == "NEW" || o["status"] == "PARTIALLY_FILLED")
                .map(|(id, o)| (id.clone(), o["orderId"].as_u64().unwrap()))
                .collect(),
            algos: self
                .algos
                .iter()
                .filter(|(_, a)| a["algoStatus"] == "NEW")
                .map(|(id, _)| id.clone())
                .collect(),
            max_notional: d("10000"),
            account_as_of: now.saturating_sub(self.snapshot_lag_ms),
            received: Instant::now(),
        }
    }
    fn fill(&mut self, client: &str, side: Side, qty: D, reduce: bool, partial: bool) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let amount = if partial { qty / D::from(2) } else { qty };
        let signed = if side == Side::Long { amount } else { -amount };
        let gross = if reduce {
            (self.price - self.entry) * if self.qty > D::ZERO { amount } else { -amount }
        } else {
            D::ZERO
        };
        if reduce {
            assert_eq!(
                side,
                if self.qty > D::ZERO {
                    Side::Short
                } else {
                    Side::Long
                }
            );
            assert!(amount <= self.qty.abs());
        } else {
            assert!(self.qty.is_zero());
            self.entry = self.price;
        }
        self.qty += signed;
        let quote = amount * self.price;
        let fee = quote * d("0.0006");
        self.wallet += gross - fee;
        let order = json!({"symbol":"BTCUSDT","clientOrderId":client,"orderId":id,"status":if partial{if reduce{"EXPIRED"}else{"PARTIALLY_FILLED"}}else{"FILLED"},"executedQty":amount.to_string(),"cumQuote":quote.to_string(),"origQty":qty.to_string(),"side":side_name(side),"positionSide":"BOTH","type":"MARKET","reduceOnly":reduce});
        self.orders.insert(client.into(), order.clone());
        self.fills.insert(id,vec![json!({"id":id+10000,"orderId":id,"symbol":"BTCUSDT","side":side_name(side),"positionSide":"BOTH","qty":amount.to_string(),"quoteQty":quote.to_string(),"commission":fee.to_string(),"commissionAsset":"USDT","realizedPnl":gross.to_string(),"time":self.now})]);
        order
    }
    fn target(&mut self) {
        self.trigger(ProtectionKind::Target);
    }
    fn trigger(&mut self, kind: ProtectionKind) {
        let (client, algo) = self
            .algos
            .iter()
            .find(|(_, a)| {
                a["orderType"]
                    == (if kind == ProtectionKind::Target {
                        "TAKE_PROFIT_MARKET"
                    } else {
                        "STOP_MARKET"
                    })
                    && a["algoStatus"] == "NEW"
            })
            .map(|(id, a)| (id.clone(), a.clone()))
            .unwrap();
        self.price = algo["triggerPrice"].as_str().unwrap().parse().unwrap();
        self.now += 1000;
        let side = if self.qty > D::ZERO {
            Side::Short
        } else {
            Side::Long
        };
        let qty = self.qty.abs();
        let order = self.fill(&format!("child-{}", self.next_id), side, qty, true, false);
        let a = self.algos.get_mut(&client).unwrap();
        a["algoStatus"] = json!("FINISHED");
        a["actualOrderId"] = json!(order["orderId"].as_u64().unwrap().to_string());
    }
    fn entries(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    Command::Market {
                        reduce_only: false,
                        ..
                    }
                )
            })
            .count()
    }
}
impl Exchange for Broker {
    fn live_enabled(&self) -> bool {
        true
    }
    async fn read(&mut self, read: Read) -> transport::Result<Value> {
        if matches!(read, Read::Algo { .. }) && !self.reconcile_delay.is_zero() {
            tokio::time::sleep(self.reconcile_delay).await;
        }
        if self.fail_reads {
            return Err(transport::Error::Network);
        }
        match read {
            Read::Order { client_id, .. } => self
                .orders
                .get(&client_id)
                .cloned()
                .ok_or(transport::Error::NotFound),
            Read::Algo { client_id } => self
                .algos
                .get(&client_id)
                .cloned()
                .ok_or(transport::Error::NotFound),
            Read::Symbols => Ok(
                json!([{"symbol":"BTCUSDT","marginType":if self.isolated{"ISOLATED"}else{"CROSSED"},"leverage":self.leverage}]),
            ),
            Read::FeeBurn => Ok(json!({"feeBurn":self.fee_burn})),
            Read::OrderId { order_id, .. } => self
                .orders
                .values()
                .find(|o| o["orderId"].as_u64() == Some(order_id))
                .cloned()
                .ok_or(transport::Error::NotFound),
            Read::Trades {
                order_id, from_id, ..
            } => {
                if self.ledger_network {
                    Err(transport::Error::Network)
                } else {
                    Ok(if self.missing_fills {
                        json!([])
                    } else {
                        Value::Array(
                            self.fills
                                .get(&order_id)
                                .into_iter()
                                .flatten()
                                .filter(|f| from_id.is_none_or(|n| f["id"].as_u64().unwrap() >= n))
                                .take(1000)
                                .cloned()
                                .collect(),
                        )
                    })
                }
            }
            Read::Funding { page, .. } => Ok(Value::Array(
                self.funding
                    .iter()
                    .skip((page as usize - 1) * 1000)
                    .take(1000)
                    .cloned()
                    .collect(),
            )),
            _ => panic!("Unexpected read in fake exchange"),
        }
    }
    async fn write_once(&mut self, command: &Command) -> transport::Result<Value> {
        self.commands.push(command.clone());
        match command {
            Command::Isolated { .. } => {
                self.isolated = true;
                Ok(json!({"code":200}))
            }
            Command::Leverage { leverage, .. } => {
                self.leverage = *leverage;
                Ok(json!({"symbol":"BTCUSDT","leverage":leverage,"maxNotionalValue":"10000"}))
            }
            Command::Market {
                client_id,
                side,
                qty,
                reduce_only,
                ..
            } => {
                let partial = if *reduce_only {
                    std::mem::take(&mut self.residual)
                } else {
                    self.partial
                };
                let mut result = self.fill(client_id, *side, *qty, *reduce_only, partial);
                if self.mutation_without_quote {
                    // Current New Order response schema omits cumQuote; GET has it.
                    result.as_object_mut().unwrap().remove("cumQuote");
                }
                if !reduce_only && std::mem::take(&mut self.lose_entry) {
                    Err(transport::Error::Unknown)
                } else {
                    Ok(result)
                }
            }
            Command::Protect {
                client_id,
                position_side,
                kind,
                trigger,
                ..
            } => {
                if self.reject_stop && *kind == ProtectionKind::Stop {
                    return Err(transport::Error::Rejected(-2021));
                }
                self.next_id += 1;
                let result = json!({"symbol":"BTCUSDT","clientAlgoId":client_id,"algoId":self.next_id,"algoType":"CONDITIONAL","orderType":if *kind==ProtectionKind::Stop{"STOP_MARKET"}else{"TAKE_PROFIT_MARKET"},"algoStatus":"NEW","actualOrderId":"","side":side_name(opposite(*position_side)),"positionSide":"BOTH","workingType":"MARK_PRICE","triggerPrice":trigger.to_string(),"closePosition":true,"priceProtect":false});
                self.algos.insert(client_id.clone(), result.clone());
                if *kind == ProtectionKind::Stop && std::mem::take(&mut self.lose_stop) {
                    Err(transport::Error::Unknown)
                } else {
                    Ok(result)
                }
            }
            Command::CancelOrder { client_id, .. } => {
                let row = self.orders.get_mut(client_id).unwrap();
                row["status"] = json!("CANCELED");
                Ok(row.clone())
            }
            Command::CancelAlgo { client_id } => {
                let row = self.algos.get_mut(client_id).unwrap();
                row["algoStatus"] = json!("CANCELED");
                Ok(json!({"code":"200","clientAlgoId":client_id,"algoId":row["algoId"]}))
            }
        }
    }
}
struct Rig {
    engine: Engine,
    broker: Broker,
    bars: Vec<Bar>,
    now: u64,
    _dir: tempfile::TempDir,
}

#[test]
fn p1_frame_rejects_stale_quote_before_any_prepared_decision() {
    let mut r = Rig::new(false);
    let mut frame = r.broker.frame(&r.bars, r.now);
    frame.ready.context.quote.observed_at_ms = frame.ready.context.now - 5069;
    assert!(
        frame.fresh().is_err(),
        "A fresh account cannot freshen an old quote"
    );
}

#[tokio::test]
async fn p1_stopped_settlement_never_returns_to_watching() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.engine.stop("fixture stop before settlement").unwrap();
    r.bars.clear();
    for _ in 0..40 {
        r.tick().await.unwrap();
        assert_ne!(r.engine.phase, super::session_store::Phase::Watching);
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
}

#[tokio::test(start_paused = true)]
async fn p1_pending_history_cannot_delay_protection_and_failure_keeps_close_only() {
    use super::session_input::{History, Sampler};
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap(); // Entry and immediate SL.
    let mut history = History::default();
    let mut sampler =
        Sampler::spawn(|_| async { std::future::pending::<anyhow::Result<Vec<Bar>>>().await });
    sampler.poll(&mut history, r.now).unwrap();
    tokio::task::yield_now().await;
    let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
    let mut frame = r.broker.frame(&r.bars, r.now);
    history.attach(&mut frame);
    assert!(sampler.pending());
    r.engine
        .tick_prepared(&mut r.broker, &frame, evidence)
        .await
        .unwrap();
    assert!(matches!(
        r.broker.commands.last(),
        Some(Command::Protect {
            kind: ProtectionKind::Target,
            ..
        })
    ));
    assert!(!r.engine.stopping());
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    tokio::task::yield_now().await;
    let error = sampler.poll(&mut history, r.now + 11000).unwrap_err();
    r.engine.data_failed(&error.to_string()).unwrap();
    assert!(r.engine.stopping());
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().stop_code,
        Some(super::exit::Reason::DataFailure)
    );
    drop(sampler);
    r.bars.clear();
    for _ in 0..40 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    assert!(r.broker.qty.is_zero());
}

#[tokio::test]
async fn p1_exit_evidence_distinguishes_hold_user_stop_and_exchange_target() {
    for case in ["holding", "user", "target", "stop_loss", "funding"] {
        let mut r = Rig::new(false);
        r.signal(false);
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        let prefix = r
            .engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .current
            .as_ref()
            .unwrap()
            .intent
            .prefix
            .clone();
        match case {
            "holding" => {
                r.now += 30 * 60000;
            }
            "user" => r
                .engine
                .stop_with(super::exit::Reason::UserStop, "fixture user stop")
                .unwrap(),
            "target" => {
                r.broker.target();
                r.now = r.broker.now + 1000;
            }
            "stop_loss" => {
                r.broker.trigger(ProtectionKind::Stop);
                r.now = r.broker.now + 1000;
            }
            "funding" => {
                let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
                let mut frame = r.broker.frame(&r.bars, r.now);
                frame.ready.next_funding = r.now + 60000;
                r.engine
                    .tick_prepared(&mut r.broker, &frame, evidence)
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        for _ in 0..40 {
            r.tick().await.unwrap();
            if r.engine.store.state.session.as_ref().unwrap().completed == 1 {
                break;
            }
        }
        let bytes =
            std::fs::read(r.engine.store.root.join(format!("{prefix}-ledger.json"))).unwrap();
        let archive: Value = serde_json::from_slice(&bytes).unwrap();
        if matches!(case, "target" | "stop_loss") {
            assert_eq!(
                archive["exit_outcome"],
                if case == "target" {
                    "take_profit"
                } else {
                    "stop_loss"
                }
            );
            assert!(
                archive.get("exit_decision").is_none(),
                "No software exit request should be invented for TP fill"
            );
        } else {
            assert_eq!(archive["exit_outcome"], "reduce_only");
            assert_eq!(
                archive["exit_decision"]["primary"],
                match case {
                    "holding" => "hold_timeout",
                    "funding" => "funding_window",
                    _ => "user_stop",
                }
            );
            assert_eq!(archive["exit_decision"]["rule_version"], "exit-evidence-1");
        }
        assert_eq!(r.broker.entries(), 1);
    }
}

#[tokio::test]
async fn p1_legacy_state_and_archive_keep_hashes_and_crash_settlement_compatible() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    let state_file = r.engine.store.root.join("session.json");
    let before = std::fs::read(&state_file).unwrap();
    let envelope: Value = serde_json::from_slice(&before).unwrap();
    assert!(envelope["state"]["session"].get("stop_code").is_none());
    assert!(
        envelope["state"]["session"]["current"]
            .get("exit_decision")
            .is_none()
    );
    assert!(
        envelope["state"]["session"]["current"]
            .get("exit_outcome")
            .is_none()
    );
    let saved = super::session_store::read_saved(&r.engine.store.root)
        .unwrap()
        .unwrap();
    assert_eq!(
        digest(&serde_json::to_vec(&saved).unwrap()),
        envelope["hash"].as_str().unwrap()
    );
    let prefix = saved.session.unwrap().current.unwrap().intent.prefix;
    let result = ledger::Settlement {
        entry_qty: D::ONE,
        exit_qty: D::ONE,
        gross: D::ZERO,
        fees: D::ONE,
        funding: D::ZERO,
        net: -D::ONE,
        fill_ids: vec![1, 2],
        funding_ids: vec![],
        last_fill_ms: r.now,
    };
    r.engine.store.settle(result.clone(), r.now).unwrap();
    let archive_file = r.engine.store.root.join(format!("{prefix}-ledger.json"));
    let old_archive = std::fs::read(&archive_file).unwrap();
    std::fs::write(&state_file, before).unwrap(); // Fixture crash: old state, final legacy archive.
    r.restart();
    let trade = r
        .engine
        .store
        .state
        .session
        .as_mut()
        .unwrap()
        .current
        .as_mut()
        .unwrap();
    super::exit::record(
        &mut trade.exit_decision,
        vec![super::exit::Reason::Recovery],
        r.now,
    );
    trade.exit_outcome = Some(super::exit::Outcome::ReduceOnly);
    let mut wrong = result.clone();
    wrong.net -= D::ONE;
    assert!(r.engine.store.settle(wrong, r.now).is_err());
    r.engine.store.settle(result, r.now).unwrap();
    assert_eq!(
        std::fs::read(&archive_file).unwrap(),
        old_archive,
        "Do not backfill reasons into a legacy ledger"
    );
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().net, -D::ONE);
}

#[tokio::test(start_paused = true)]
async fn p1_async_confirmed_signal_can_enter_but_late_stop_and_deadline_cannot() {
    use super::session_input::{History, Sampler};
    for case in ["fresh", "late", "stop", "deadline"] {
        let mut r = Rig::new(false);
        let mut history = History::default();
        history.accept(r.bars.clone(), r.now).unwrap();
        r.signal(false);
        let bar_ms = r.bars.last().unwrap().close_ms;
        r.now = bar_ms + 3000;
        let next = r.bars.clone();
        let mut sampler = Sampler::spawn(move |_| {
            let bars = next.clone();
            async move { Ok(bars) }
        });
        for step in 0..3 {
            if step == 2 {
                match case {
                    "late" => r.now = bar_ms + 16000,
                    "stop" => r
                        .engine
                        .stop_with(
                            super::exit::Reason::UserStop,
                            "fixture before confirmed input",
                        )
                        .unwrap(),
                    "deadline" => r.engine.store.state.session.as_mut().unwrap().deadline = r.now,
                    _ => {}
                }
            }
            sampler.poll(&mut history, r.now).unwrap();
            let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
            let mut frame = r.broker.frame(&r.bars, r.now);
            history.attach(&mut frame);
            r.engine
                .tick_prepared(&mut r.broker, &frame, evidence)
                .await
                .unwrap();
            if step < 2 {
                assert_eq!(r.broker.entries(), 0);
            }
            tokio::task::yield_now().await;
            tokio::time::advance(std::time::Duration::from_secs(2)).await;
            r.now += 2000;
        }
        assert_eq!(history.confirmations, 1, "{case}");
        assert_eq!(r.broker.entries(), usize::from(case == "fresh"), "{case}");
        if case == "fresh" {
            assert!(matches!(
                r.broker.commands.last(),
                Some(Command::Protect {
                    kind: ProtectionKind::Stop,
                    ..
                })
            ));
        }
    }
}

#[tokio::test]
async fn p1_twenty_four_hour_virtual_deadline_crosses_utc_without_extending_session() {
    let mut r = Rig::new(false);
    let started = r.engine.store.state.session.as_ref().unwrap().started;
    let deadline = r.engine.store.state.session.as_ref().unwrap().deadline;
    assert_eq!(deadline - started, 24 * 60 * 60000);
    for elapsed_hours in 0..=24 {
        r.now = started + elapsed_hours * 60 * 60000;
        let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
        let mut frame = r.broker.frame(&r.bars, r.now);
        frame.ready.next_funding = r.now + 8 * 60 * 60000;
        r.engine
            .tick_prepared(&mut r.broker, &frame, evidence)
            .await
            .unwrap();
        assert_eq!(r.engine.finished(), elapsed_hours == 24);
        assert_eq!(
            r.engine.store.state.session.as_ref().unwrap().deadline,
            deadline
        );
    }
    assert!(deadline / 86400000 > started / 86400000);
    assert!(r.broker.commands.is_empty());
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().stop_code,
        Some(super::exit::Reason::SessionDeadline)
    );
}

#[tokio::test]
async fn order_query_latency_must_not_age_the_next_position_frame() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    assert_eq!(r.engine.phase, super::session_store::Phase::Trading);
    assert_eq!(r.broker.entries(), 1);
    let commands = r.broker.commands.len();
    r.broker.snapshot_lag_ms = 1000;
    r.broker.reconcile_delay = std::time::Duration::from_millis(2200);
    let result = r.tick().await;
    assert!(
        result.is_ok(),
        "Order reads aged the decision frame: {result:?}"
    );
    assert!(!r.engine.stopping());
    assert_eq!(r.engine.phase, super::session_store::Phase::Trading);
    assert_eq!(r.broker.commands.len(), commands);
    assert_eq!(r.broker.entries(), 1);
}

#[tokio::test]
async fn fresh_account_never_overrides_stale_order_evidence() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5100)).await;
    let f = r.broker.frame(&r.bars, r.now);
    let count = r.broker.commands.len();
    let error = r
        .engine
        .tick_prepared(&mut r.broker, &f, evidence)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("order_batch_age_ms="));
    assert_eq!(r.broker.commands.len(), count);
}

#[tokio::test]
async fn fresh_order_evidence_never_overrides_stale_or_future_account_and_quote() {
    for field in ["account", "quote", "future-account", "future-quote"] {
        let mut r = Rig::new(false);
        r.signal(false);
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
        let mut f = r.broker.frame(&r.bars, r.now);
        match field {
            "account" => f.account_as_of -= 5001,
            "quote" => f.ready.context.quote.observed_at_ms -= 5001,
            "future-account" => f.account_as_of += 60_000,
            "future-quote" => f.ready.context.quote.observed_at_ms += 60_000,
            _ => unreachable!(),
        }
        let count = r.broker.commands.len();
        let error = r
            .engine
            .tick_prepared(&mut r.broker, &f, evidence)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("age_ms="), "{field}: {error}");
        assert_eq!(
            super::recovery::transient(&error),
            !field.starts_with("future")
        );
        assert_eq!(r.broker.commands.len(), count);
    }
}

#[tokio::test]
async fn evidence_is_bound_to_intent_and_missing_protection_cannot_report_watch() {
    let mut r = Rig::new(false);
    let flat_evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    let f = r.broker.frame(&r.bars, r.now);
    let count = r.broker.commands.len();
    assert!(
        r.engine
            .tick_prepared(&mut r.broker, &f, flat_evidence)
            .await
            .unwrap_err()
            .to_string()
            .contains("intent saat ini")
    );
    assert_eq!(r.broker.commands.len(), count);
    let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
    let mut f = r.broker.frame(&r.bars, r.now);
    f.algos.clear(); // Disappeared AFTER individual GET, BEFORE account/open-order GET.
    r.engine
        .tick_prepared(&mut r.broker, &f, evidence)
        .await
        .unwrap();
    assert!(r.engine.stopping());
    assert_ne!(r.engine.phase, super::session_store::Phase::Trading);
    assert!(r.broker.qty.is_zero());
    assert_eq!(r.broker.entries(), 1);
}

#[tokio::test]
async fn interrupted_reconciliation_cannot_override_stop_or_duplicate_entry_after_restart() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.broker.reconcile_delay = std::time::Duration::from_secs(1);
    let count = r.broker.commands.len();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(10),
            r.engine.reconcile_orders(&mut r.broker)
        )
        .await
        .is_err()
    );
    assert_eq!(r.broker.commands.len(), count);
    r.engine.stop("Stop during order GET").unwrap();
    r.restart();
    r.broker.reconcile_delay = std::time::Duration::ZERO;
    r.bars.clear();
    for _ in 0..40 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    assert!(r.broker.qty.is_zero());
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
}

#[tokio::test]
async fn stop_or_deadline_after_reconciliation_wins_over_a_fresh_signal() {
    for stop in [true, false] {
        let mut r = Rig::new(false);
        r.signal(false);
        let evidence = r.engine.reconcile_orders(&mut r.broker).await.unwrap();
        if stop {
            r.engine.stop("queued user stop").unwrap();
        } else {
            r.now = r.engine.store.state.session.as_ref().unwrap().deadline;
        }
        let f = r.broker.frame(&r.bars, r.now);
        r.engine
            .tick_prepared(&mut r.broker, &f, evidence)
            .await
            .unwrap();
        assert!(r.engine.finished());
        assert!(r.broker.commands.is_empty());
    }
}

#[tokio::test]
async fn market_response_without_quote_is_queried_once_before_immediate_stop_protection() {
    let mut r = Rig::new(false);
    r.broker.mutation_without_quote = true;
    r.signal(false);
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 1);
    assert!(matches!(
        r.broker.commands.last(),
        Some(Command::Protect {
            kind: ProtectionKind::Stop,
            ..
        })
    ));
    let t = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .current
        .as_ref()
        .unwrap();
    let out = super::outbox::Outbox::open(&r.engine.store.outbox_path(&t.intent.prefix).unwrap())
        .unwrap();
    let entry = out.record(&format!("{}-entry", t.intent.prefix)).unwrap();
    assert!(entry.reconciled);
    assert!(
        matches!(&entry.receipt,Some(Receipt::Order(o)) if o.status==OrderStatus::Filled && o.quote_qty > D::ZERO)
    );
    drop(out);
    r.engine.stop("test exit receipt too").unwrap();
    r.bars.clear();
    for _ in 0..40 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
    assert_eq!(r.broker.entries(), 1);
}

#[test]
fn activation_checks_economics_before_writing_session_and_respects_daily_remainder() {
    let mut broker = Broker::default();
    let bars = history(99, false);
    broker.price = bars.last().unwrap().close;
    let now = bars.last().unwrap().close_ms + 1;
    let frame = broker.frame(&bars, now);
    for case in ["notional", "daily", "balance", "orders", "stale"] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine =
            Engine::new(Store::open(dir.path(), &digest(b"setup regression")).unwrap());
        let mut config = config();
        let mut f = frame.clone();
        match case {
            "notional" => config.budget.max_notional = d("0.001"),
            "daily" => {
                engine.store.state.daily.insert(now / 86_400_000, -d("10"));
            }
            "balance" => f.ready.context.available = d("0.001"),
            "orders" => f.regular.push(("foreign".into(), 1)),
            "stale" => f.account_as_of = now - 6000,
            _ => unreachable!(),
        }
        assert!(engine.begin(config, &f).is_err(), "{case}");
        assert!(engine.store.state.session.is_none(), "{case}");
        assert!(!dir.path().join("session.json").exists(), "{case}");
    }
    assert!(broker.commands.is_empty());
}

#[test]
fn margin_is_a_cap_not_a_requirement_to_lock_the_entire_budget() {
    let dir = tempfile::tempdir().unwrap();
    let mut broker = Broker {
        wallet: d("20"),
        ..Default::default()
    };
    let bars = history(99, false);
    broker.price = bars.last().unwrap().close;
    let now = bars.last().unwrap().close_ms + 1;
    let f = broker.frame(&bars, now);
    let mut engine = Engine::new(Store::open(dir.path(), &digest(b"margin cap")).unwrap());
    let c = config();
    assert!(c.budget.margin > f.ready.context.available);
    let report = f.setup(&c, D::ZERO).unwrap();
    assert!(report.viable());
    assert_eq!(report.effective_budget.margin, d("20"));
    engine.begin(c.clone(), &f).unwrap();
    assert_eq!(engine.store.state.session.as_ref().unwrap().config, c);
    assert!(broker.commands.is_empty());
}

#[tokio::test]
async fn four_hour_deadline_survives_multiple_outages_without_automatic_restart() {
    let mut r = Rig::new(false);
    let started = r.engine.store.state.session.as_ref().unwrap().started;
    let deadline = started + 240 * 60_000;
    {
        let s = r.engine.store.state.session.as_mut().unwrap();
        s.config.minutes = 240;
        s.deadline = deadline;
    }
    for minute in 1..=240 {
        r.now = started + minute * 60_000;
        r.bars = history(99 + (minute / 5) as usize, false);
        for bar in &mut r.bars {
            bar.open = d("100");
            bar.high = d("100");
            bar.low = d("100");
            bar.close = d("100");
        }
        if (50..=61).contains(&minute) || (170..=181).contains(&minute) {
            r.engine
                .data_failed("synthetic multi-minute data outage")
                .unwrap();
        } else {
            let frame = r.broker.frame(&r.bars, r.now);
            r.engine.tick(&mut r.broker, &frame).await.unwrap();
        }
        assert_eq!(r.engine.finished(), minute == 240);
        assert_eq!(
            r.engine.store.state.session.as_ref().unwrap().deadline,
            deadline
        );
        assert!(r.broker.commands.is_empty());
    }
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert!(s.stop && s.finished);
    assert_eq!(s.completed, 0);
    assert!(s.current.is_none());
    assert_eq!(s.net, D::ZERO);
}
impl Rig {
    fn new(short: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), &digest(b"session fixture")).unwrap();
        let bars = history(99, short);
        let now = bars.last().unwrap().close_ms + 1;
        let mut broker = Broker {
            price: bars.last().unwrap().close,
            ..Default::default()
        };
        let f = broker.frame(&bars, now);
        let mut engine = Engine::new(store);
        engine.begin(config(), &f).unwrap();
        Self {
            engine,
            broker,
            bars,
            now,
            _dir: dir,
        }
    }
    fn signal(&mut self, short: bool) {
        self.bars = history(100, short);
        self.bars.last_mut().unwrap().volume = D::from(2);
        self.broker.price = self.bars.last().unwrap().close;
        self.now = self.bars.last().unwrap().close_ms + 1;
    }
    async fn tick(&mut self) -> anyhow::Result<()> {
        let reconciled = self.engine.reconcile_orders(&mut self.broker).await?;
        self.now = self.now.max(
            self.engine
                .store
                .state
                .session
                .as_ref()
                .unwrap()
                .updated
                .saturating_add(1),
        );
        let f = self.broker.frame(&self.bars, self.now);
        let r = self
            .engine
            .tick_prepared(&mut self.broker, &f, reconciled)
            .await;
        self.now += 1000;
        r
    }
    fn restart(&mut self) {
        let path = self.engine.store.root.clone();
        let replacement =
            Store::open(&self._dir.path().join("unused"), &digest(b"unused")).unwrap();
        let old = std::mem::replace(&mut self.engine.store, replacement);
        drop(old);
        self.engine = Engine::new(Store::open(&path, &digest(b"session fixture")).unwrap());
        self.engine.resume().unwrap();
    }
}

#[tokio::test]
async fn actual_supervisor_drives_long_short_protection_exit_costs_and_persistent_history() {
    for short in [false, true] {
        let mut r = Rig::new(short);
        r.tick().await.unwrap();
        assert!(r.broker.commands.is_empty());
        r.signal(short);
        r.tick().await.unwrap();
        assert_eq!(r.broker.entries(), 1);
        assert_eq!(
            r.broker.commands.len(),
            2,
            "entry immediately followed by stop"
        );
        assert!(matches!(
            r.broker.commands[1],
            Command::Protect {
                kind: ProtectionKind::Stop,
                ..
            }
        ));
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        r.broker.target();
        r.now = r.broker.now + 1000;
        for _ in 0..25 {
            r.tick().await.unwrap();
            if r.engine.store.state.session.as_ref().unwrap().completed == 1 {
                break;
            }
        }
        let s = r.engine.store.state.session.as_ref().unwrap();
        assert_eq!(s.completed, 1);
        assert!(s.net > D::ZERO);
        assert!(s.fees > D::ZERO);
        assert_eq!(r.broker.wallet - d("1000"), s.net);
        assert!(r.broker.qty.is_zero());
        assert!(s.current.is_none());
        assert_eq!(r.broker.entries(), 1);
        assert_eq!(
            r.engine.store.state.daily.values().copied().sum::<D>(),
            s.net
        );
        let archives = std::fs::read_dir(&r.engine.store.root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with("-ledger.json"))
            .count();
        assert_eq!(archives, 1);
        r.engine.stop("fixture stop").unwrap();
        r.tick().await.unwrap();
        assert!(r.engine.finished());
    }
}

#[tokio::test]
async fn cost_aware_user_bounds_drive_final_target_protection_and_accounting_in_both_directions() {
    for short in [false, true] {
        let scaled = |count| {
            let mut bars = history(count, short);
            for b in &mut bars {
                b.open = b.open * D::from(100) + D::from(60000);
                b.high = b.high * D::from(100) + D::from(60000);
                b.low = b.low * D::from(100) + D::from(60000);
                b.close = b.close * D::from(100) + D::from(60000);
            }
            bars
        };
        let dir = tempfile::tempdir().unwrap();
        let bars = scaled(99);
        let now = bars.last().unwrap().close_ms + 1;
        let mut broker = Broker {
            price: bars.last().unwrap().close,
            leverage: 5,
            ..Default::default()
        };
        let mut engine =
            Engine::new(Store::open(dir.path(), &digest(b"cost-aware lifecycle")).unwrap());
        let c = Config {
            budget: margin::Budget {
                margin: d("100"),
                risk: D::ONE,
                max_notional: d("200"),
                leverage: 5,
                slippage_bps: D::ONE,
                liquidation_buffer_bps: d("10"),
                funding_reserve: d("0.1"),
            },
            daily_loss: d("5"),
            session_loss: d("3"),
            minutes: 240,
            max_trades: 1,
        };
        engine.begin(c.clone(), &broker.frame(&bars, now)).unwrap();
        let mut r = Rig {
            engine,
            broker,
            bars,
            now,
            _dir: dir,
        };
        r.bars = scaled(100);
        r.bars.last_mut().unwrap().volume = d("2");
        r.broker.price = r.bars.last().unwrap().close;
        r.now = r.bars.last().unwrap().close_ms + 1;
        r.tick().await.unwrap();
        assert_eq!(r.broker.entries(), 1);
        let s = r.engine.store.state.session.as_ref().unwrap();
        let t = s.current.as_ref().unwrap();
        let p = t.intent.plan.clone();
        assert!(super::planning::reward_passes(&p, c.budget.slippage_bps));
        assert!(p.budgeted_loss <= D::ONE && p.notional <= d("200"));
        assert_eq!(s.decision.as_ref().unwrap().target, Some(p.take_profit));
        assert_eq!(t.decision.target, Some(p.take_profit));
        assert_eq!(s.strategy_version, super::automatic::VERSION);
        assert!(
            matches!(&r.broker.commands[1],Command::Protect {kind:ProtectionKind::Stop,trigger,..} if *trigger==p.stop)
        );
        r.tick().await.unwrap();
        r.tick().await.unwrap();
        assert!(r.broker.commands.iter().any(|cmd|matches!(cmd,Command::Protect{kind:ProtectionKind::Target,trigger,..} if *trigger==p.take_profit)));
        r.broker.target();
        r.now = r.broker.now + 1000;
        for _ in 0..35 {
            r.tick().await.unwrap();
            if r.engine.finished() {
                break;
            }
        }
        assert!(r.engine.finished());
        let s = r.engine.store.state.session.as_ref().unwrap();
        assert_eq!(s.completed, 1);
        assert!(s.current.is_none());
        assert_eq!(s.net, r.broker.wallet - d("1000"));
        assert!(s.fees > D::ZERO);
    }
}

#[tokio::test]
async fn uncertain_responses_restart_partial_rejection_and_residual_never_duplicate_entry() {
    for mode in [
        "entry-response",
        "stop-response",
        "partial",
        "stop-reject",
        "residual",
    ] {
        let mut r = Rig::new(false);
        r.broker.lose_entry = mode == "entry-response";
        r.broker.lose_stop = mode == "stop-response";
        r.broker.partial = mode == "partial";
        r.broker.reject_stop = mode == "stop-reject";
        r.broker.residual = mode == "residual";
        r.signal(false);
        r.tick().await.unwrap();
        if mode.ends_with("response") {
            r.restart();
        } else if mode == "residual" {
            r.engine.stop("fixture residual").unwrap();
        }
        for _ in 0..40 {
            r.tick().await.unwrap();
            if mode == "partial"
                && r.broker
                    .algos
                    .values()
                    .any(|a| a["orderType"] == "TAKE_PROFIT_MARKET" && a["algoStatus"] == "NEW")
            {
                r.engine.stop("fixture partial close").unwrap();
            }
            if r.engine.store.state.session.as_ref().unwrap().completed == 1 {
                break;
            }
        }
        let s = r.engine.store.state.session.as_ref().unwrap();
        assert_eq!(s.completed, 1, "{mode}: {}", s.reason);
        assert_eq!(r.broker.entries(), 1, "{mode}");
        assert!(r.broker.qty.is_zero());
        assert_eq!(r.broker.wallet - d("1000"), s.net);
        if mode == "residual" {
            assert_eq!(
                r.broker
                    .commands
                    .iter()
                    .filter(|c| matches!(
                        c,
                        Command::Market {
                            reduce_only: true,
                            ..
                        }
                    ))
                    .count(),
                2
            );
        }
    }
}

#[tokio::test]
async fn no_entry_for_stale_foreign_bnb_fee_budget_or_stop_and_unavailable_protection_cannot_claim_watch()
 {
    for mode in ["stale", "foreign", "fee-bnb", "budget", "stop"] {
        let mut r = Rig::new(false);
        r.signal(false);
        let mut f = r.broker.frame(&r.bars, r.now);
        match mode {
            "stale" => f.account_as_of -= 5001,
            "foreign" => f.regular.push(("external".into(), 99)),
            "fee-bnb" => r.broker.fee_burn = true,
            "budget" => r
                .engine
                .store
                .state
                .daily
                .insert(r.now / 86_400_000, -d("10"))
                .map(|_| ())
                .unwrap_or(()),
            "stop" => r.engine.stop("user stop before entry").unwrap(),
            _ => unreachable!(),
        }
        let _ = r.engine.tick(&mut r.broker, &f).await;
        assert_eq!(r.broker.entries(), 0, "{mode}");
    }
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.broker.fail_reads = true;
    r.tick().await.unwrap();
    assert!(r.engine.store.state.session.as_ref().unwrap().stop);
    assert!(r.broker.qty.is_zero());
    assert_eq!(r.broker.entries(), 1);
}

#[tokio::test]
async fn one_session_can_finish_long_then_select_short_without_per_order_input() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.broker.target();
    r.now = r.broker.now + 1000;
    for _ in 0..25 {
        r.tick().await.unwrap();
        if r.engine.store.state.session.as_ref().unwrap().completed == 1 {
            break;
        }
    }
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
    for i in 100..146 {
        let last = r.bars.last().unwrap();
        let price = last.close - D::ONE;
        let open_ms = last.open_ms + INTERVAL;
        r.bars.push(Bar {
            open_ms,
            close_ms: open_ms + INTERVAL - 1,
            open: price,
            close: price,
            high: price + d("0.5"),
            low: price - d("0.5"),
            volume: if i == 145 { D::from(2) } else { D::ONE },
        });
        r.now = open_ms + INTERVAL;
        r.broker.price = price;
        r.tick().await.unwrap();
        if r.broker.entries() == 2 {
            break;
        }
    }
    assert_eq!(r.broker.entries(), 2);
    assert!(r.broker.qty < D::ZERO);
    assert_eq!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .decision
            .as_ref()
            .unwrap()
            .side,
        Some(Side::Short)
    );
    r.tick().await.unwrap();
    r.broker.target();
    r.now = r.broker.now + 1000;
    for _ in 0..25 {
        r.tick().await.unwrap();
        if r.engine.store.state.session.as_ref().unwrap().completed == 2 {
            break;
        }
    }
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 2);
    assert!(r.broker.qty.is_zero());
}

#[tokio::test]
async fn ledger_rejects_missing_duplicate_foreign_and_inconsistent_fills_and_waits_for_funding() {
    let mut b = Broker {
        now: 100_000,
        ..Default::default()
    };
    let entry = b.fill("entry", Side::Long, D::ONE, false, false)["orderId"]
        .as_u64()
        .unwrap();
    b.now = 200_000;
    b.price = d("110");
    let exit = b.fill("exit", Side::Short, D::ONE, true, false)["orderId"]
        .as_u64()
        .unwrap();
    let query = || ledger::Query {
        symbol: Symbol::BTCUSDT,
        side: Side::Long,
        entry,
        exits: std::slice::from_ref(&exit),
        start: 99_999,
        end: 210_000,
        funding_due: &[],
    };
    let clean = ledger::settle(&mut b, query()).await.unwrap();
    assert_eq!(clean.gross, d("10"));
    assert_eq!(clean.net, d("9.874"));
    let original = b.fills.clone();
    for mode in ["missing", "duplicate", "asset", "pnl", "identity"] {
        b.fills = original.clone();
        let rows = b.fills.get_mut(&exit).unwrap();
        match mode {
            "missing" => rows.clear(),
            "duplicate" => rows.push(rows[0].clone()),
            "asset" => rows[0]["commissionAsset"] = json!("BNB"),
            "pnl" => rows[0]["realizedPnl"] = json!("11"),
            "identity" => rows[0]["orderId"] = json!(123),
            _ => unreachable!(),
        };
        assert!(ledger::settle(&mut b, query()).await.is_err(), "{mode}");
    }
    b.fills = original;
    let due = [150_000];
    let mut q = query();
    q.funding_due = &due;
    assert!(ledger::settle(&mut b, q).await.is_err());
    b.funding = vec![
        json!({"tranId":1,"time":150_000,"symbol":"BTCUSDT","incomeType":"FUNDING_FEE","asset":"USDT","income":"-0.03"}),
    ];
    let mut q = query();
    q.funding_due = &due;
    let paid = ledger::settle(&mut b, q).await.unwrap();
    assert_eq!(paid.net, clean.net - d("0.03"));
    b.funding.clear();
    let outside = [205_000];
    let mut q = query();
    q.funding_due = &outside;
    assert!(ledger::settle(&mut b, q).await.is_ok());
}

#[tokio::test]
async fn ledger_lag_wallet_mismatch_and_hard_risk_limit_block_further_entries() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.broker.target();
    r.now = r.broker.now + 1000;
    r.broker.missing_fills = true;
    let mut failed = false;
    for _ in 0..20 {
        if r.tick().await.is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed);
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 0);
    assert_eq!(r.broker.entries(), 1);
    r.broker.missing_fills = false;
    r.broker.wallet += D::ONE;
    assert!(r.tick().await.is_err());
    r.broker.wallet -= D::ONE;
    for _ in 0..12 {
        r.tick().await.unwrap();
    }
    assert_eq!(r.engine.store.state.session.as_ref().unwrap().completed, 1);
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.broker.price -= d("20");
    for _ in 0..30 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    assert!(r.broker.qty.is_zero());
    assert!(r.engine.store.state.session.as_ref().unwrap().net <= -d("10"));
}

#[tokio::test]
async fn setup_changes_are_confirmed_before_entry_and_stop_does_not_configure_account() {
    let mut r = Rig::new(false);
    r.broker.isolated = false;
    r.broker.leverage = 1;
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    assert_eq!(r.broker.commands.len(), 2);
    assert_eq!(r.broker.entries(), 0);
    r.signal(false);
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 1);
    let mut r = Rig::new(false);
    r.broker.isolated = false;
    r.broker.leverage = 1;
    r.engine.stop("queued stop").unwrap();
    r.tick().await.unwrap();
    assert!(r.engine.finished());
    assert!(r.broker.commands.is_empty());
}

#[tokio::test]
async fn fill_pagination_after_one_thousand_rows_keeps_exact_totals() {
    let mut b = Broker {
        now: 100_000,
        ..Default::default()
    };
    let entry = b.fill("entry", Side::Long, D::from(1001), false, false)["orderId"]
        .as_u64()
        .unwrap();
    b.now = 200_000;
    b.price = d("101");
    let exit = b.fill("exit", Side::Short, D::from(1001), true, false)["orderId"]
        .as_u64()
        .unwrap();
    for (id, side, price, at, offset, pnl) in [
        (entry, "BUY", d("100"), 100_000, 0, D::ZERO),
        (exit, "SELL", d("101"), 200_000, 2000, D::ONE),
    ] {
        b.fills.insert(id,(1..=1001).map(|n|json!({"id":n+offset,"orderId":id,"symbol":"BTCUSDT","side":side,"positionSide":"BOTH","qty":"1","quoteQty":price.to_string(),"commission":(price*d("0.0006")).to_string(),"commissionAsset":"USDT","realizedPnl":pnl.to_string(),"time":at})).collect());
    }
    let result = ledger::settle(
        &mut b,
        ledger::Query {
            symbol: Symbol::BTCUSDT,
            side: Side::Long,
            entry,
            exits: &[exit],
            start: 99_999,
            end: 210_000,
            funding_due: &[],
        },
    )
    .await
    .unwrap();
    assert_eq!(result.fill_ids.len(), 2002);
    assert_eq!(result.entry_qty, D::from(1001));
    assert_eq!(result.net, b.wallet - d("1000"));
}

#[tokio::test]
async fn archive_then_crash_before_state_commit_recovers_exactly_once_and_daily_limit_persists() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.tick().await.unwrap();
    let root = r.engine.store.root.clone();
    let display = super::live::Snapshot::saved(&root);
    assert_eq!(display.phase, super::session_store::Phase::Recovering);
    assert!(!display.active && !display.busy && display.position.is_none());
    let before = std::fs::read(root.join("session.json")).unwrap();
    let prefix = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .current
        .as_ref()
        .unwrap()
        .intent
        .prefix
        .clone();
    std::fs::write(
        root.join(format!("{prefix}-ledger.tmp")),
        b"incomplete crash write",
    )
    .unwrap();
    let result = ledger::Settlement {
        entry_qty: D::ONE,
        exit_qty: D::ONE,
        gross: -d("9"),
        fees: D::ONE,
        funding: D::ZERO,
        net: -d("10"),
        fill_ids: vec![1, 2],
        funding_ids: vec![],
        last_fill_ms: r.now,
    };
    r.engine.store.settle(result.clone(), r.now).unwrap();
    let archive: Value =
        serde_json::from_slice(&std::fs::read(root.join(format!("{prefix}-ledger.json"))).unwrap())
            .unwrap();
    assert_eq!(archive["strategy_version"], super::automatic::VERSION);
    assert_eq!(archive["decision"]["side"], "long");
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(root.join(format!("{prefix}-ledger.json")))
            .unwrap()
            .permissions()
            .mode()
            & 0o077,
        0
    );
    // Model only the old state remaining durable while the ledger archive survived.
    std::fs::write(root.join("session.json"), before).unwrap();
    r.restart();
    r.engine.store.settle(result, r.now).unwrap();
    let s = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(s.net, -d("10"));
    assert_eq!(s.completed, 1);
    assert_eq!(
        *r.engine
            .store
            .state
            .daily
            .get(&(r.now / 86_400_000))
            .unwrap(),
        -d("10")
    );
    r.engine.store.state.session.as_mut().unwrap().finished = true;
    r.engine.store.commit().unwrap();
    assert!(
        r.engine
            .store
            .begin(
                config(),
                r.now + 1,
                r.now + 1,
                d("1000"),
                r.bars.last().unwrap().close_ms
            )
            .is_err()
    );
    let file = root.join("session.json");
    std::fs::write(file, b"corrupt").unwrap();
    let replacement = Store::open(&r._dir.path().join("unused2"), &digest(b"unused")).unwrap();
    drop(std::mem::replace(&mut r.engine.store, replacement));
    assert!(Store::open(&root, &digest(b"session fixture")).is_err());
}

#[tokio::test]
async fn cost_filter_trade_count_duration_and_new_day_are_enforced() {
    let mut r = Rig::new(true);
    r.signal(true);
    for bar in &mut r.bars {
        bar.open += D::from(700);
        bar.close += D::from(700);
        bar.high += D::from(700);
        bar.low += D::from(700);
    }
    r.broker.price = r.bars.last().unwrap().close;
    // This historical fixed-3-ATR rejection is now economically feasible with
    // a bounded cost-aware target; keep it as a positive regression.
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 1);
    let trade = r
        .engine
        .store
        .state
        .session
        .as_ref()
        .unwrap()
        .current
        .as_ref()
        .unwrap();
    assert!(super::planning::reward_passes(&trade.intent.plan, D::ONE));
    // A genuinely uneconomic current fee still blocks entry; no weakened guard.
    let mut r = Rig::new(true);
    r.signal(true);
    let mut frame = r.broker.frame(&r.bars, r.now);
    frame.ready.context.taker_rate = d("0.01");
    r.engine.tick(&mut r.broker, &frame).await.unwrap();
    assert_eq!(r.broker.entries(), 0);
    assert!(r.engine.note.contains("Sinyal dilewati"));
    let mut r = Rig::new(false);
    r.engine
        .store
        .state
        .session
        .as_mut()
        .unwrap()
        .config
        .max_trades = 1;
    r.signal(false);
    r.tick().await.unwrap();
    r.tick().await.unwrap();
    r.broker.target();
    r.now = r.broker.now + 1000;
    for _ in 0..30 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    let mut r = Rig::new(false);
    r.now = r.engine.store.state.session.as_ref().unwrap().deadline;
    r.tick().await.unwrap();
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 0);
    let mut r = Rig::new(false);
    r.engine.store.state.daily.insert(0, -d("10"));
    r.engine.store.state.session.as_mut().unwrap().finished = true;
    r.engine
        .store
        .begin(config(), 86_400_001, 86_400_001, d("1000"), 86_399_999)
        .unwrap();
    assert_eq!(*r.engine.store.state.daily.get(&0).unwrap(), -d("10"));
}

#[tokio::test]
async fn stopped_sessions_finish_without_signal_history_flat_or_after_managed_exit() {
    for entered in [false, true] {
        let mut r = Rig::new(false);
        if entered {
            r.signal(false);
            r.tick().await.unwrap();
            r.tick().await.unwrap();
            assert_eq!(r.broker.entries(), 1);
        }
        r.engine
            .stop("Candle history failed; entry stopped")
            .unwrap();
        r.bars.clear();
        for _ in 0..40 {
            r.tick().await.unwrap();
            if r.engine.finished() {
                break;
            }
        }
        assert!(
            r.engine.finished(),
            "recovery must not require signal history"
        );
        assert_eq!(r.broker.entries(), usize::from(entered));
        assert!(r.broker.qty.is_zero());
        assert!(
            r.engine
                .store
                .state
                .session
                .as_ref()
                .unwrap()
                .current
                .is_none()
        );
    }
}

#[test]
fn empty_session_repair_rejects_exposure_stale_data_changed_wallet_and_trade_history() {
    for case in [
        "running",
        "stale",
        "position",
        "order",
        "algo",
        "wallet",
        "sequence",
        "completed",
        "net",
        "fees",
        "funding",
    ] {
        let mut r = Rig::new(false);
        if case != "running" {
            r.engine.stop("fixture failure").unwrap();
        }
        let mut f = r.broker.frame(&r.bars, r.now);
        match case {
            "stale" => f.account_as_of -= 5001,
            "position" => {
                r.broker.qty = d("1");
                f = r.broker.frame(&r.bars, r.now);
            }
            "order" => f.regular.push(("foreign-order".into(), 1)),
            "algo" => f.algos.push("foreign-algo".into()),
            "wallet" => f.ready.account.balances.total_wallet_balance += D::ONE,
            "sequence" => r.engine.store.state.session.as_mut().unwrap().sequence = 1,
            "completed" => r.engine.store.state.session.as_mut().unwrap().completed = 1,
            "net" => r.engine.store.state.session.as_mut().unwrap().net = D::ONE,
            "fees" => r.engine.store.state.session.as_mut().unwrap().fees = D::ONE,
            "funding" => r.engine.store.state.session.as_mut().unwrap().funding = D::ONE,
            _ => {}
        }
        let before = std::fs::read(r.engine.store.root.join("session.json")).unwrap();
        assert!(
            r.engine.finish_empty_verified(&f).is_err(),
            "accepted invalid {case}"
        );
        assert!(!r.engine.finished());
        assert_eq!(
            before,
            std::fs::read(r.engine.store.root.join("session.json")).unwrap()
        );
        assert!(r.broker.commands.is_empty());
    }
    let mut r = Rig::new(false);
    r.now = r.engine.store.state.session.as_ref().unwrap().deadline;
    let f = r.broker.frame(&[], r.now);
    r.engine.finish_empty_verified(&f).unwrap();
    assert!(r.engine.finished());
    assert!(r.broker.commands.is_empty());
    assert!(
        super::session_store::read_saved(&r.engine.store.root)
            .unwrap()
            .unwrap()
            .session
            .unwrap()
            .finished
    );
}

#[tokio::test]
async fn recovery_consumes_the_closed_interval_even_when_its_candle_is_still_pending() {
    let mut r = Rig::new(false);
    let prior = r.bars.clone();
    r.signal(false);
    r.now += 2000;
    r.engine
        .data_failed("read failure while candle is settling")
        .unwrap();
    let pending = r.broker.frame(&prior, r.now);
    r.engine.tick(&mut r.broker, &pending).await.unwrap();
    r.now += 2000;
    r.tick().await.unwrap();
    assert_eq!(
        r.broker.entries(),
        0,
        "confirmation must not replay the recovery interval"
    );
    assert!(!r.engine.stopping());
}

#[tokio::test]
async fn transient_data_failure_pauses_flat_session_but_never_resumes_exposure_or_user_stop() {
    let mut r = Rig::new(false);
    r.engine.data_failed("temporary read failure").unwrap();
    assert!(!r.engine.stopping());
    assert_eq!(r.engine.phase, super::session_store::Phase::Recovering);
    assert!(r.broker.commands.is_empty());
    r.signal(false);
    r.tick().await.unwrap();
    assert_eq!(
        r.broker.entries(),
        0,
        "first recovered candle is not an entry"
    );
    r.bars = history(101, false);
    r.bars.last_mut().unwrap().volume = D::from(2);
    r.broker.price = r.bars.last().unwrap().close;
    r.now = r.bars.last().unwrap().close_ms + 1;
    r.tick().await.unwrap();
    assert_eq!(
        r.broker.entries(),
        1,
        "fresh input can resume the existing flat session"
    );
    r.engine.data_failed("read failure with exposure").unwrap();
    assert!(r.engine.stopping());
    r.bars.clear();
    for _ in 0..40 {
        r.tick().await.unwrap();
        if r.engine.finished() {
            break;
        }
    }
    assert!(r.engine.finished());
    assert_eq!(r.broker.entries(), 1);
    for user_stopped in [false, true] {
        let mut r = Rig::new(false);
        if user_stopped {
            r.engine.stop("user stop").unwrap();
        }
        r.engine.data_failed("read failure").unwrap();
        r.signal(false);
        if !user_stopped {
            r.now = r.engine.store.state.session.as_ref().unwrap().deadline;
        }
        r.tick().await.unwrap();
        assert_eq!(r.broker.entries(), 0);
        assert!(r.engine.finished());
    }
}

#[tokio::test]
async fn revalidated_history_skips_entry_and_configuration_until_a_new_candle() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.broker.isolated = false;
    r.broker.leverage = 1;
    let mut frame = r.broker.frame(&r.bars, r.now);
    frame.skip_signal = Some("Riwayat direvalidasi; candle dilewati".into());
    r.engine.tick(&mut r.broker, &frame).await.unwrap();
    assert!(
        r.broker.commands.is_empty(),
        "no leverage/margin/entry mutation on held history"
    );
    assert_eq!(
        r.engine.store.state.session.as_ref().unwrap().last_bar,
        frame.bars.last().unwrap().close_ms
    );
    assert!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .decision
            .is_none()
    );
    let saved = super::session_store::read_saved(&r.engine.store.root)
        .unwrap()
        .unwrap()
        .session
        .unwrap();
    assert_eq!(saved.last_bar, frame.bars.last().unwrap().close_ms);
    assert!(saved.current.is_none() && saved.sequence == 0);
    r.broker.isolated = true;
    r.broker.leverage = 10;
    frame = r.broker.frame(&r.bars, r.now);
    r.engine.tick(&mut r.broker, &frame).await.unwrap();
    assert!(
        r.broker.commands.is_empty(),
        "clearing a flag must not replay a consumed bar"
    );
    r.bars = history(101, false);
    r.bars.last_mut().unwrap().volume = D::from(2);
    r.broker.price = r.bars.last().unwrap().close;
    r.now = r.bars.last().unwrap().close_ms + 1;
    r.tick().await.unwrap();
    assert_eq!(
        r.broker.entries(),
        1,
        "a genuinely newer valid signal may trade"
    );
}

#[tokio::test(start_paused = true)]
async fn confirmed_new_signal_can_enter_but_late_stop_and_deadline_cannot() {
    use super::session_input::{self, History, Input, Mode};
    struct InputFixture(Frame);
    impl Input for InputFixture {
        async fn account_frame(&mut self) -> anyhow::Result<Frame> {
            let mut f = self.0.clone();
            f.received = Instant::now();
            Ok(f)
        }
        async fn closed_bars(&mut self, _: u64) -> anyhow::Result<Vec<Bar>> {
            Ok(self.0.bars.clone())
        }
    }
    for case in ["fresh", "late", "stop", "deadline", "recovery"] {
        let mut r = Rig::new(false);
        r.now += 2000;
        let mut history = History::default();
        let mut input = InputFixture(r.broker.frame(&r.bars, r.now));
        session_input::poll(&mut input, &mut history, Mode::Signals)
            .await
            .unwrap();
        r.signal(false);
        r.now += 2000;
        if case == "deadline" {
            r.engine.store.state.session.as_mut().unwrap().deadline = r.now + 1000;
        }
        if case == "recovery" {
            r.engine.data_failed("temporary read failure").unwrap();
        }
        input.0 = r.broker.frame(&r.bars, r.now);
        let pending = session_input::poll(&mut input, &mut history, Mode::Signals)
            .await
            .unwrap();
        r.engine.tick(&mut r.broker, &pending).await.unwrap();
        assert!(
            r.broker.commands.is_empty(),
            "first read cannot enter: {case}"
        );
        if case == "stop" {
            r.engine.stop("user stop while finalizing").unwrap();
        }
        let delay = if case == "late" { 16_000 } else { 2000 };
        tokio::time::advance(std::time::Duration::from_millis(delay)).await;
        r.now += delay;
        input.0 = r.broker.frame(&r.bars, r.now);
        let confirmed = session_input::poll(&mut input, &mut history, Mode::Signals)
            .await
            .unwrap();
        assert_eq!(confirmed.bars.last(), r.bars.last());
        assert!(confirmed.skip_signal.is_none());
        r.engine.tick(&mut r.broker, &confirmed).await.unwrap();
        assert_eq!(r.broker.entries(), usize::from(case == "fresh"), "{case}");
        assert_eq!(history.confirmations, 1);
        assert_eq!(history.rewarms, 0);
        if case == "stop" || case == "deadline" {
            assert!(r.engine.finished());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn real_input_pipeline_and_supervisor_recover_revision_without_replaying_signal() {
    use super::session_input::{self, History, Input, Mode};
    struct InputFixture(Frame);
    impl Input for InputFixture {
        async fn account_frame(&mut self) -> anyhow::Result<Frame> {
            let mut frame = self.0.clone();
            frame.received = Instant::now();
            Ok(frame)
        }
        async fn closed_bars(&mut self, now: u64) -> anyhow::Result<Vec<Bar>> {
            Ok(self
                .0
                .bars
                .iter()
                .filter(|bar| bar.close_ms < now)
                .cloned()
                .collect())
        }
    }
    let mut r = Rig::new(false);
    r.now += 2000;
    let mut history_cache = History::default();
    let mut input = InputFixture(r.broker.frame(&r.bars, r.now));
    let f = session_input::poll(&mut input, &mut history_cache, Mode::Signals)
        .await
        .unwrap();
    r.engine.tick(&mut r.broker, &f).await.unwrap();
    r.signal(false);
    r.now += 2000;
    r.bars[98].volume += D::ONE;
    input.0 = r.broker.frame(&r.bars, r.now);
    let error = session_input::poll(&mut input, &mut history_cache, Mode::Signals)
        .await
        .err()
        .unwrap();
    r.engine.data_failed(&error.to_string()).unwrap();
    assert!(!r.engine.stopping());
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    r.now += 2000;
    input.0 = r.broker.frame(&r.bars, r.now);
    let f = session_input::poll(&mut input, &mut history_cache, Mode::Signals)
        .await
        .unwrap();
    assert!(f.skip_signal.is_some());
    r.engine.tick(&mut r.broker, &f).await.unwrap();
    assert!(r.broker.commands.is_empty());
    assert!(!r.engine.stopping());
    let f = session_input::poll(&mut input, &mut history_cache, Mode::Signals)
        .await
        .unwrap();
    r.engine.tick(&mut r.broker, &f).await.unwrap();
    assert!(r.broker.commands.is_empty());
    r.bars = history(101, false);
    for i in [98, 99, 100] {
        r.bars[i].volume = D::from(2);
    }
    r.broker.price = r.bars.last().unwrap().close;
    r.now = r.bars.last().unwrap().close_ms + 2001;
    input.0 = r.broker.frame(&r.bars, r.now);
    let pending = session_input::poll(&mut input, &mut history_cache, Mode::Signals)
        .await
        .unwrap();
    r.engine.tick(&mut r.broker, &pending).await.unwrap();
    assert!(r.broker.commands.is_empty());
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    r.now += 2000;
    input.0 = r.broker.frame(&r.bars, r.now);
    let f = session_input::poll(&mut input, &mut history_cache, Mode::Signals)
        .await
        .unwrap();
    assert!(f.skip_signal.is_none());
    r.engine.tick(&mut r.broker, &f).await.unwrap();
    assert_eq!(r.broker.entries(), 1);
    assert!(matches!(
        r.broker.commands.last(),
        Some(Command::Protect {
            kind: ProtectionKind::Stop,
            ..
        })
    ));
}

#[tokio::test]
async fn revalidated_history_never_overrides_stop_deadline_or_existing_exposure() {
    for case in ["stop", "deadline", "exposure"] {
        let mut r = Rig::new(false);
        r.signal(false);
        if case == "exposure" {
            r.tick().await.unwrap();
            r.tick().await.unwrap();
        } else if case == "stop" {
            r.engine.stop("explicit user stop").unwrap();
        } else {
            r.now = r.engine.store.state.session.as_ref().unwrap().deadline;
        }
        let mut frame = r.broker.frame(&r.bars, r.now);
        frame.skip_signal = Some("Riwayat direvalidasi".into());
        r.engine.tick(&mut r.broker, &frame).await.unwrap();
        assert!(r.engine.stopping());
        r.bars.clear();
        for _ in 0..40 {
            r.tick().await.unwrap();
            if r.engine.finished() {
                break;
            }
        }
        assert!(r.engine.finished(), "{case} did not finish");
        assert_eq!(r.broker.entries(), usize::from(case == "exposure"));
        assert!(r.broker.qty.is_zero());
    }
}

#[tokio::test]
async fn candle_closed_before_activation_cannot_enter_after_finalization_delay() {
    let mut r = Rig::new(false);
    r.signal(false);
    r.engine.store.state.session.as_mut().unwrap().started = r.now;
    r.tick().await.unwrap();
    assert_eq!(r.broker.entries(), 0);
    assert!(
        r.engine
            .store
            .state
            .session
            .as_ref()
            .unwrap()
            .reason
            .contains("sebelum aktivasi")
    );
}

#[tokio::test]
async fn sixty_minute_session_survives_rewarm_and_finishes_at_original_deadline() {
    let mut r = Rig::new(false);
    let started = r.now;
    let deadline = started + 60 * 60_000;
    {
        let session = r.engine.store.state.session.as_mut().unwrap();
        session.config.minutes = 60;
        session.deadline = deadline;
    }
    for minute in 1..=60 {
        r.now = started + minute * 60_000;
        r.bars = history(99 + (minute / 5) as usize, false);
        for bar in &mut r.bars {
            bar.open = d("100");
            bar.high = d("100");
            bar.low = d("100");
            bar.close = d("100");
        }
        let mut frame = r.broker.frame(&r.bars, r.now);
        if minute == 20 {
            r.engine.data_failed("Candle tertutup berubah").unwrap();
            frame.skip_signal = Some("Revalidasi berhasil; candle terdampak dilewati".into());
        }
        r.engine.tick(&mut r.broker, &frame).await.unwrap();
        assert_eq!(r.engine.finished(), minute == 60);
        assert_eq!(
            r.engine.store.state.session.as_ref().unwrap().deadline,
            deadline
        );
        assert!(r.broker.commands.is_empty());
    }
    let session = r.engine.store.state.session.as_ref().unwrap();
    assert_eq!(session.net, D::ZERO);
    assert!(session.updated >= deadline);
}

#[tokio::test]
async fn original_stop_cause_survives_cleanup_and_restart_display() {
    for entered in [false, true] {
        let mut r = Rig::new(false);
        if entered {
            r.signal(false);
            r.tick().await.unwrap();
            r.tick().await.unwrap();
        }
        let cause = "Koneksi/data gagal: snapshot account/posisi usang";
        r.engine.stop(cause).unwrap();
        r.engine
            .stop("Pemulihan berikutnya masih menunggu akun")
            .unwrap();
        r.bars.clear();
        for _ in 0..40 {
            r.tick().await.unwrap();
            if r.engine.finished() {
                break;
            }
        }
        assert!(r.engine.finished());
        let saved = super::live::Snapshot::saved(&r.engine.store.root);
        assert!(
            saved.reason.contains(cause),
            "stop cause was lost: {}",
            saved.reason
        );
    }
}

#[tokio::test]
async fn flat_session_can_pause_beyond_thirty_seconds_without_expanding_deadline() {
    for seconds in [31, 310, 631] {
        let mut r = Rig::new(false);
        let deadline = r.engine.store.state.session.as_ref().unwrap().deadline;
        r.now += seconds * 1000;
        r.engine
            .data_failed("observed public connection outage")
            .unwrap();
        assert!(
            !r.engine.stopping(),
            "flat sessions pause, never extend duration or silently restart"
        );
        assert_eq!(
            r.engine.store.state.session.as_ref().unwrap().deadline,
            deadline
        );
        assert!(r.broker.commands.is_empty());
        let count = (r.now / INTERVAL) as usize;
        r.bars = history(count, false);
        r.bars.last_mut().unwrap().volume = D::from(2);
        r.broker.price = r.bars.last().unwrap().close;
        r.tick().await.unwrap();
        assert!(
            r.broker.commands.is_empty(),
            "no catch-up entry, even if multiple candles elapsed"
        );
        assert!(!r.engine.stopping());
        r.bars = history(count + 1, false);
        r.bars.last_mut().unwrap().volume = D::from(2);
        r.broker.price = r.bars.last().unwrap().close;
        r.now = r.bars.last().unwrap().close_ms + 2001;
        r.tick().await.unwrap();
        assert_eq!(r.broker.entries(), 1);
    }
}

#[tokio::test]
async fn resumed_data_cannot_bypass_account_wallet_order_or_freshness_checks() {
    for case in ["wallet", "position", "order", "stale", "mode"] {
        let mut r = Rig::new(false);
        r.engine.data_failed("connection lost").unwrap();
        r.signal(false);
        let mut f = r.broker.frame(&r.bars, r.now);
        match case {
            "wallet" => f.ready.account.balances.total_wallet_balance += D::ONE,
            "position" => {
                r.broker.qty = D::ONE;
                f = r.broker.frame(&r.bars, r.now);
            }
            "order" => f.regular.push(("foreign-order".into(), 999)),
            "stale" => f.account_as_of -= 5001,
            "mode" => f.ready.account.config.dual_side_position = true,
            _ => unreachable!(),
        }
        let outcome = r.engine.tick(&mut r.broker, &f).await;
        assert!(
            outcome.is_err()
                || r.engine.stopping()
                || r.engine.phase == super::session_store::Phase::Blocked,
            "unsafe recovery: {case}"
        );
        assert!(
            r.broker.commands.is_empty(),
            "recovery mutated an unsafe account: {case}"
        );
    }
}
