//! Recovery needs a fresh account/quote, never a valid strategy history.
use super::{readiness, supervisor::Frame};
use crate::{
    market,
    paper::strategy::{self, Bar, INTERVAL},
};
use anyhow::{Result, ensure};
use std::time::{Duration, Instant};

// REST close time is an interval boundary, not an explicit finalization flag.
// Exclude the boundary race; this does not extend the 15-second entry window.
const CLOSE_GRACE_MS: u64 = 2000;
const RECHECK_DELAY: Duration = Duration::from_secs(2);

/// A known revalidation gate, not an authentication/schema failure and not
/// transport recovery authority for an outstanding position.
#[derive(Debug)]
pub(crate) struct HistoryRevalidation;
impl std::fmt::Display for HistoryRevalidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Candle tertutup berubah; menunggu revalidasi")
    }
}
impl std::error::Error for HistoryRevalidation {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Signals,
    Recovery,
}

#[derive(Default)]
pub(crate) struct History {
    bars: Vec<Bar>,
    epoch: Option<u64>,
    candidate: Option<Candidate>,
    settling: Option<Candidate>,
    skip_through: Option<u64>,
    pub(crate) revisions: u32,
    pub(crate) rewarms: u32,
    pub(crate) confirmations: u32,
}

struct Candidate {
    bars: Vec<Bar>,
    at: tokio::time::Instant,
    server_ms: u64,
    detail: String,
}

fn revision(before: &[Bar], after: &[Bar]) -> Option<String> {
    before.iter().find_map(|old| {
        let new = after.iter().find(|bar| bar.open_ms == old.open_ms)?;
        if old == new {
            return None;
        }
        let changes = [
            ("open", old.open, new.open),
            ("high", old.high, new.high),
            ("low", old.low, new.low),
            ("close", old.close, new.close),
            ("volume", old.volume, new.volume),
        ]
        .into_iter()
        .filter(|(_, a, b)| a != b)
        .map(|(name, a, b)| format!("{name}:{a}->{b}"))
        .collect::<Vec<_>>()
        .join(",");
        Some(format!("open_ms={}; {changes}", old.open_ms))
    })
}

impl History {
    pub(crate) fn due(&self, now: u64) -> bool {
        self.epoch != Some(now / INTERVAL) || self.candidate.is_some() || self.settling.is_some()
    }
    pub(crate) fn attach(&self, frame: &mut Frame) {
        frame.bars = self.bars.clone();
        frame.skip_signal = self.skip_signal();
    }
    pub(crate) fn accept(&mut self, next: Vec<Bar>, now: u64) -> Result<()> {
        ensure!(
            (80..=240).contains(&next.len()),
            "Warmup perlu 80–240 candle"
        );
        let changed = revision(&self.bars, &next);
        if changed.is_some() || self.candidate.is_some() {
            self.settling = None;
            let consistent = self.candidate.as_ref().is_some_and(|c| {
                c.bars == next
                    && c.at.elapsed() >= RECHECK_DELAY
                    && now.saturating_sub(c.server_ms) >= 2000
            });
            if !consistent {
                if self.candidate.as_ref().is_none_or(|c| c.bars != next) {
                    let detail =
                        changed.unwrap_or_else(|| "Riwayat kembali berubah saat revalidasi".into());
                    self.revisions = self.revisions.saturating_add(1);
                    self.candidate = Some(Candidate {
                        bars: next,
                        at: tokio::time::Instant::now(),
                        server_ms: now,
                        detail,
                    });
                }
                return Err(anyhow::Error::new(HistoryRevalidation).context(format!(
                    "Candle tertutup berubah; entry ditahan untuk dua pembacaan konsisten: {}",
                    self.candidate.as_ref().unwrap().detail
                )));
            }
            // New baseline is for FUTURE signals only. Never trade the revised
            // candle or any candle observed during revalidation, even on retry.
            self.skip_through = next.last().map(|bar| bar.close_ms);
            self.rewarms = self.rewarms.saturating_add(1);
            eprintln!(
                "CANDLE_HISTORY_REWARMED; skip_through_ms={}; {}",
                self.skip_through.unwrap_or(0),
                self.candidate.as_ref().unwrap().detail
            );
            self.candidate = None;
        } else if self
            .bars
            .last()
            .is_some_and(|old| next.last().is_some_and(|new| new.close_ms > old.close_ms))
        {
            // A newly closed REST bar can still be provisional. Keep the last
            // confirmed baseline while obtaining two equal reads >=2s apart.
            // This is NOT a data failure/recovery and does not consume the new
            // bar. Revisions of an already confirmed bar still use skip/recovery.
            let consistent = self.settling.as_ref().is_some_and(|c| {
                c.bars == next
                    && c.at.elapsed() >= RECHECK_DELAY
                    && now.saturating_sub(c.server_ms) >= 2000
            });
            if !consistent {
                if self.settling.as_ref().is_none_or(|c| c.bars != next) {
                    let bar_ms = next.last().unwrap().close_ms;
                    eprintln!(
                        "CANDLE_FINALIZATION_WAIT; bar_ms={bar_ms}; reason=two_equal_reads_required"
                    );
                    self.settling = Some(Candidate {
                        bars: next,
                        at: tokio::time::Instant::now(),
                        server_ms: now,
                        detail: String::new(),
                    });
                }
                return Ok(());
            }
            self.confirmations = self.confirmations.saturating_add(1);
            eprintln!(
                "CANDLE_FINALIZED; bar_ms={}; confirmed_age_ms={}",
                next.last().unwrap().close_ms,
                now.saturating_sub(next.last().unwrap().close_ms)
            );
            self.settling = None;
        }
        self.bars = next;
        let epoch = now / INTERVAL;
        self.epoch = self.bars.last().and_then(|bar| {
            (bar.close_ms.checked_add(1) == Some(epoch * INTERVAL)).then_some(epoch)
        });
        Ok(())
    }

    fn skip_signal(&self) -> Option<String> {
        self.skip_through
            .filter(|limit| self.bars.last().is_none_or(|bar| bar.close_ms <= *limit))
            .map(|bar| format!("Riwayat direvalidasi; candle hingga {bar} dilewati tanpa entry, menunggu candle baru"))
    }
}

pub(crate) trait Input {
    fn account_frame(&mut self) -> impl Future<Output = Result<Frame>>;
    fn closed_bars(&mut self, now: u64) -> impl Future<Output = Result<Vec<Bar>>>;
}

impl Input for readiness::Source {
    async fn account_frame(&mut self) -> Result<Frame> {
        let ready = self.fetch().await?;
        Ok(account_snapshot(self, ready))
    }

    async fn closed_bars(&mut self, now: u64) -> Result<Vec<Bar>> {
        public_bars(&mut self.public, now).await
    }
}

pub(crate) async fn prepared_account(source: &mut readiness::Source) -> Result<Frame> {
    let ready = source.fetch_prepared().await?;
    Ok(account_snapshot(source, ready))
}
fn account_snapshot(source: &readiness::Source, ready: readiness::Ready) -> Frame {
    let received = Instant::now();
    let account_as_of = ready
        .clock
        .current_latest()
        .saturating_sub(source.account_started.elapsed().as_millis() as u64);
    Frame {
        ready,
        bars: vec![],
        skip_signal: None,
        regular: source.regular.clone(),
        algos: source.algos.clone(),
        max_notional: source.max_notional,
        account_as_of,
        received,
    }
}

async fn public_bars(public: &mut market::Transport, now: u64) -> Result<Vec<Bar>> {
    let rows = public
        .json(
            market::Market::Futures,
            "klines",
            &[
                ("symbol", "BTCUSDT".into()),
                ("interval", "5m".into()),
                ("limit", "200".into()),
                (
                    "endTime",
                    (now / INTERVAL * INTERVAL).saturating_sub(1).to_string(),
                ),
            ],
        )
        .await?;
    strategy::bars(&rows, now)
}

/// Public-only sampling on a separate task: pending candles/slow klines never
/// hold up the account/order/protection loop. One request and one result maximum.
pub(crate) struct Sampler {
    requests: tokio::sync::mpsc::Sender<u64>,
    results: tokio::sync::mpsc::Receiver<Result<(Vec<Bar>, u64)>>,
    task: tokio::task::JoinHandle<()>,
    busy: bool,
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Sampler {
    pub(crate) fn new() -> Result<Self> {
        let public = std::sync::Arc::new(tokio::sync::Mutex::new(market::Transport::new()?));
        Ok(Self::spawn(move |now| {
            let public = public.clone();
            async move { public_bars(&mut *public.lock().await, now).await }
        }))
    }
    pub(crate) fn spawn<F, Fut>(mut fetch: F) -> Self
    where
        F: FnMut(u64) -> Fut + Send + 'static,
        Fut: Future<Output = Result<Vec<Bar>>> + Send + 'static,
    {
        let (requests, mut input) = tokio::sync::mpsc::channel::<u64>(1);
        let (output, results) = tokio::sync::mpsc::channel(1);
        let task = tokio::spawn(async move {
            while let Some(now) = input.recv().await {
                let at = tokio::time::Instant::now();
                let result = tokio::time::timeout(
                    Duration::from_secs(10),
                    fetch(now.saturating_sub(CLOSE_GRACE_MS)),
                )
                .await
                .map_err(|_| {
                    anyhow::Error::new(crate::market::ReadFailure::Network)
                        .context("History publik timeout")
                })
                .and_then(|r| r)
                .map(|bars| (bars, now.saturating_add(at.elapsed().as_millis() as u64)));
                if output.send(result).await.is_err() {
                    break;
                }
            }
        });
        Self {
            requests,
            results,
            task,
            busy: false,
        }
    }
    pub(crate) fn poll(&mut self, history: &mut History, now: u64) -> Result<()> {
        match self.results.try_recv() {
            Ok(result) => {
                self.busy = false;
                let (bars, observed_ms) = result?;
                history.accept(bars, observed_ms)?;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                anyhow::bail!("Worker history berhenti")
            }
        }
        if !self.busy && history.due(now) {
            self.requests
                .try_send(now)
                .map_err(|_| anyhow::anyhow!("Antrean history tidak tersedia"))?;
            self.busy = true;
        }
        Ok(())
    }
    pub(crate) fn pending(&self) -> bool {
        self.busy
    }
}

pub(crate) async fn poll<I: Input>(
    input: &mut I,
    history: &mut History,
    mode: Mode,
) -> Result<Frame> {
    let mut frame = input.account_frame().await?;
    // Never feed cached signal history into a recovery snapshot.
    frame.bars.clear();
    frame.skip_signal = None;
    if mode == Mode::Signals {
        let now = frame.now();
        let closed_now = frame.earliest_now();
        let epoch = closed_now / INTERVAL;
        if history.epoch != Some(epoch) || history.candidate.is_some() {
            let next = input
                .closed_bars(closed_now.saturating_sub(CLOSE_GRACE_MS))
                .await?;
            frame.fresh()?;
            history.accept(next, now)?;
        }
        frame.bars = history.bars.clone();
        frame.skip_signal = history.skip_signal();
    }
    frame.fresh()?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal as D;

    #[tokio::test(start_paused = true)]
    async fn p1_slow_history_is_bounded_nonblocking_and_does_not_repeat_requests() {
        use std::sync::{
            Arc,
            atomic::{AtomicU32, Ordering},
        };
        let calls = Arc::new(AtomicU32::new(0));
        let seen = calls.clone();
        let mut sampler = Sampler::spawn(move |_| {
            seen.fetch_add(1, Ordering::Relaxed);
            async { std::future::pending::<Result<Vec<Bar>>>().await }
        });
        let mut history = History::default();
        let mut input = FakeInput::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        let baseline = history.bars.clone();
        input.now += INTERVAL;
        for _ in 0..5 {
            sampler.poll(&mut history, input.now).unwrap();
            tokio::task::yield_now().await;
            let mut frame = poll(&mut input, &mut history, Mode::Recovery)
                .await
                .unwrap();
            history.attach(&mut frame);
            frame.fresh().unwrap();
            assert_eq!(frame.bars, baseline);
            tokio::time::advance(Duration::from_secs(2)).await;
            input.now += 2000;
        }
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(
            sampler
                .poll(&mut history, input.now)
                .unwrap_err()
                .to_string()
                .contains("timeout")
        );
        assert_eq!(history.bars, baseline);
    }

    #[tokio::test(start_paused = true)]
    async fn p1_sampler_preserves_two_read_confirmation_and_revision_failure() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        let next = strategy::tests::history(101, false);
        let expected = next.clone();
        let mut sampler = Sampler::spawn(move |_| {
            let next = next.clone();
            async move { Ok(next) }
        });
        input.now += INTERVAL;
        sampler.poll(&mut history, input.now).unwrap();
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(2)).await;
        input.now += 2000;
        sampler.poll(&mut history, input.now).unwrap();
        assert_eq!(history.confirmations, 0);
        assert_eq!(history.bars.len(), 100);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(2)).await;
        input.now += 2000;
        sampler.poll(&mut history, input.now).unwrap();
        assert_eq!(history.confirmations, 1);
        assert_eq!(history.bars, expected);
        drop(sampler);
        let mut revised = expected;
        revised[99].volume += D::ONE;
        let mut sampler = Sampler::spawn(move |_| {
            let bars = revised.clone();
            async move { Ok(bars) }
        });
        input.now += INTERVAL;
        sampler.poll(&mut history, input.now).unwrap();
        tokio::task::yield_now().await;
        assert!(
            sampler
                .poll(&mut history, input.now)
                .unwrap_err()
                .to_string()
                .contains("Candle tertutup berubah")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn p1_dropping_sampler_aborts_work_and_discards_late_results() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let completed = Arc::new(AtomicBool::new(false));
        let seen = completed.clone();
        let mut sampler = Sampler::spawn(move |_| {
            let seen = seen.clone();
            async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                seen.store(true, Ordering::Relaxed);
                Ok(vec![])
            }
        });
        let mut history = History::default();
        sampler.poll(&mut history, INTERVAL).unwrap();
        tokio::task::yield_now().await;
        drop(sampler);
        tokio::time::advance(Duration::from_secs(6)).await;
        tokio::task::yield_now().await;
        assert!(!completed.load(Ordering::Relaxed));
        assert!(history.bars.is_empty());
    }

    struct FakeInput {
        now: u64,
        bars: Vec<Bar>,
        history_calls: usize,
        history_failed: bool,
        account_failed: bool,
        stale: bool,
        clock_uncertainty_ms: u64,
        history_now: Option<u64>,
    }
    impl Default for FakeInput {
        fn default() -> Self {
            Self {
                now: 100 * INTERVAL + CLOSE_GRACE_MS,
                bars: strategy::tests::history(100, false),
                history_calls: 0,
                history_failed: false,
                account_failed: false,
                stale: false,
                clock_uncertainty_ms: 0,
                history_now: None,
            }
        }
    }
    impl Input for FakeInput {
        async fn account_frame(&mut self) -> Result<Frame> {
            ensure!(!self.account_failed, "account unavailable");
            let mut context = super::super::tests::context();
            context.now = self.now + self.clock_uncertainty_ms;
            context.quote.observed_at_ms = self.now;
            Ok(Frame {
                ready: super::super::Ready {
                    account: crate::account::tests::example(),
                    context,
                    current_leverage: 5,
                    isolated: true,
                    blockers: vec![],
                    next_funding: self.now + self.clock_uncertainty_ms + INTERVAL,
                    clock: super::super::transport::ClockWindow::new(
                        self.now,
                        self.now + self.clock_uncertainty_ms,
                        Instant::now(),
                    )
                    .unwrap(),
                },
                bars: vec![],
                skip_signal: None,
                regular: vec![],
                algos: vec![],
                max_notional: D::from(1000),
                account_as_of: self.now - if self.stale { 5001 } else { 0 },
                received: Instant::now(),
            })
        }
        async fn closed_bars(&mut self, now: u64) -> Result<Vec<Bar>> {
            self.history_calls += 1;
            self.history_now = Some(now);
            ensure!(!self.history_failed, "history unavailable");
            Ok(self
                .bars
                .iter()
                .filter(|b| b.close_ms < now)
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn r12_candle_close_uses_earliest_clock_while_lateness_uses_latest() {
        let mut input = FakeInput {
            clock_uncertainty_ms: 400,
            ..Default::default()
        };
        let expected_earliest = input.now;
        let mut history = History::default();
        let frame = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(
            input.history_now,
            Some(expected_earliest.saturating_sub(CLOSE_GRACE_MS)),
            "candle closure must be proven by the lower clock bound"
        );
        assert!(frame.now() >= expected_earliest + 400);
        assert_eq!(frame.earliest_now(), expected_earliest);
    }

    #[tokio::test(start_paused = true)]
    async fn delayed_closed_bar_is_refetched_within_the_same_interval() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        input.now += INTERVAL;
        // The REST response still ends at the preceding candle just after rollover.
        let delayed = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(delayed.bars.last().unwrap().close_ms, 100 * INTERVAL - 1);
        input.bars = strategy::tests::history(101, false);
        input.now += 2000;
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        tokio::time::advance(RECHECK_DELAY).await;
        input.now += 2000;
        let fresh = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(fresh.bars.last().unwrap().close_ms, 101 * INTERVAL - 1);
        assert_eq!(input.history_calls, 4);
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(
            input.history_calls, 4,
            "a complete closed interval stays cached"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_bar_is_not_committed_before_two_consistent_closed_reads() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        input.now += INTERVAL;
        input.bars = strategy::tests::history(101, false);
        let pending = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(
            pending.bars.last().unwrap().close_ms,
            100 * INTERVAL - 1,
            "first REST read must not lock in a provisional new candle"
        );
        input.bars[100].close += D::new(1, 1);
        input.bars[100].volume += D::ONE;
        input.now += 2000;
        tokio::time::advance(Duration::from_secs(2)).await;
        let changed = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(changed.bars.last().unwrap().close_ms, 100 * INTERVAL - 1);
        input.now += 2000;
        tokio::time::advance(Duration::from_secs(2)).await;
        let confirmed = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(confirmed.bars.last().unwrap(), &input.bars[100]);
        assert!(confirmed.skip_signal.is_none());
        assert_eq!(history.rewarms, 0);
    }

    #[tokio::test]
    async fn revised_closed_candle_blocks_entry_but_cannot_poison_recovery() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        assert_eq!(
            poll(&mut input, &mut history, Mode::Signals)
                .await
                .unwrap()
                .bars
                .len(),
            100
        );
        input.now += INTERVAL;
        input.bars[99].volume += D::ONE;
        assert!(
            poll(&mut input, &mut history, Mode::Signals)
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("Candle tertutup berubah")
        );
        input.history_failed = true;
        let recovered = poll(&mut input, &mut history, Mode::Recovery)
            .await
            .unwrap();
        assert!(recovered.bars.is_empty());
        assert_eq!(input.history_calls, 2);
        // No automatic acceptance of revised history for new entries.
        input.history_failed = false;
        assert!(poll(&mut input, &mut history, Mode::Signals).await.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn persistent_revision_can_rewarm_from_two_consistent_reads() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        input.now += INTERVAL;
        input.bars = strategy::tests::history(101, false);
        input.bars[99].volume += D::ONE;
        assert!(poll(&mut input, &mut history, Mode::Signals).await.is_err());
        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        input.now += 2000;
        let frame = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(frame.bars[99].volume, D::from(2));
        assert!(frame.skip_signal.is_some());
        assert!(frame.decision().unwrap().side.is_none());
        assert_eq!(history.rewarms, 1);
        assert!(
            poll(&mut input, &mut history, Mode::Signals)
                .await
                .unwrap()
                .skip_signal
                .is_some()
        );
        input.now += INTERVAL;
        input.bars = strategy::tests::history(102, false);
        input.bars[99].volume += D::ONE;
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        tokio::time::advance(RECHECK_DELAY).await;
        input.now += 2000;
        let next = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert!(
            next.skip_signal.is_none(),
            "only a later unaffected candle restores signals"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn newly_closed_candle_waits_for_grace_without_pinning_previous_interval() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        input.now = 101 * INTERVAL + 1;
        input.bars = strategy::tests::history(101, false);
        let early = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(early.bars.last().unwrap().close_ms, 100 * INTERVAL - 1);
        input.now += CLOSE_GRACE_MS;
        let pending = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(pending.bars.last().unwrap().close_ms, 100 * INTERVAL - 1);
        tokio::time::advance(RECHECK_DELAY).await;
        input.now += 2000;
        let settled = poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        assert_eq!(settled.bars.last().unwrap().close_ms, 101 * INTERVAL - 1);
    }

    #[tokio::test(start_paused = true)]
    async fn flapping_history_and_fast_or_stale_rechecks_cannot_rewarm() {
        let mut input = FakeInput::default();
        let mut history = History::default();
        poll(&mut input, &mut history, Mode::Signals).await.unwrap();
        input.now += INTERVAL;
        input.bars = strategy::tests::history(101, false);
        input.bars[99].volume += D::ONE;
        let error = poll(&mut input, &mut history, Mode::Signals)
            .await
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("open_ms=29700000") && error.contains("volume:1->2"));
        assert!(poll(&mut input, &mut history, Mode::Signals).await.is_err());
        input.now += 2000;
        assert!(
            poll(&mut input, &mut history, Mode::Signals).await.is_err(),
            "server time alone is not a second independent reading"
        );
        tokio::time::advance(Duration::from_secs(2)).await;
        input.bars[99].volume += D::ONE;
        assert!(poll(&mut input, &mut history, Mode::Signals).await.is_err());
        assert_eq!(history.rewarms, 0);
        tokio::time::advance(Duration::from_secs(2)).await;
        input.now += 2000;
        input.stale = true;
        assert!(poll(&mut input, &mut history, Mode::Signals).await.is_err());
        assert_eq!(history.rewarms, 0);
        input.stale = false;
        assert!(
            poll(&mut input, &mut history, Mode::Signals)
                .await
                .unwrap()
                .skip_signal
                .is_some()
        );
        assert_eq!(history.rewarms, 1);
    }

    #[tokio::test]
    async fn recovery_without_history_still_requires_fresh_successful_account_reads() {
        let mut input = FakeInput {
            history_failed: true,
            ..Default::default()
        };
        let mut history = History::default();
        assert!(
            poll(&mut input, &mut history, Mode::Recovery)
                .await
                .unwrap()
                .bars
                .is_empty()
        );
        assert_eq!(input.history_calls, 0);
        input.stale = true;
        assert!(
            poll(&mut input, &mut history, Mode::Recovery)
                .await
                .is_err()
        );
        input.stale = false;
        input.account_failed = true;
        assert!(
            poll(&mut input, &mut history, Mode::Recovery)
                .await
                .is_err()
        );
    }
}
