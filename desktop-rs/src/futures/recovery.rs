//! Recovery never clears a terminal Stop and never authorizes an order itself.
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::Instant;

pub const MAX_EPISODES: u8 = 2;
pub const HEALTHY_SECONDS: u64 = 60;
const MAX_GAP: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub episodes: u8,
    pub active: bool,
}
impl State {
    pub fn valid(&self) -> bool {
        self.episodes <= MAX_EPISODES && (!self.active || self.episodes > 0)
    }
}

/// A snapshot is definitely older than its allowed freshness window.
#[derive(Debug)]
pub(crate) struct StaleRead {
    source: &'static str,
    age_ms: u64,
    limit_ms: u64,
}
impl StaleRead {
    pub(crate) fn check(source: &'static str, age_ms: u64, limit_ms: u64) -> anyhow::Result<()> {
        if age_ms > limit_ms {
            return Err(Self {
                source,
                age_ms,
                limit_ms,
            }
            .into());
        }
        Ok(())
    }
}
impl std::fmt::Display for StaleRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Data {} usang; age_ms={}; limit_ms={}",
            self.source, self.age_ms, self.limit_ms
        )
    }
}
impl std::error::Error for StaleRead {}

/// Explicitly typed failures only. Message text never grants recovery authority.
pub(crate) fn transient(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<super::transport::Error>(),
        Some(
            super::transport::Error::Network
                | super::transport::Error::ClockUncertain
                | super::transport::Error::ReadWindowExpired
        )
    ) || matches!(
        error.downcast_ref::<crate::market::ReadFailure>(),
        Some(crate::market::ReadFailure::Network)
    ) || error.downcast_ref::<StaleRead>().is_some()
        || error
            .downcast_ref::<super::transport::ClockUncertain>()
            .is_some()
        || error
            .downcast_ref::<super::transport::StaleTimestamp>()
            .is_some()
}

/// Ephemeral proof: no health/time proof survives process restart.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Observation {
    earliest: u64,
    latest: u64,
    acquired: Instant,
    observed: Instant,
}
impl Observation {
    pub(crate) fn new(earliest: u64, latest: u64, acquired: Instant, observed: Instant) -> Self {
        Self {
            earliest,
            latest,
            acquired,
            observed,
        }
    }
}

pub(crate) struct Healthy {
    first_acquired: Instant,
    last_acquired: Instant,
    last_observed: Instant,
    first_latest: u64,
    last_earliest: u64,
    last_latest: u64,
    current_earliest: u64,
    local_elapsed: Duration,
    samples: u32,
}
impl Healthy {
    pub fn new(observation: Observation) -> Self {
        Self {
            first_acquired: observation.acquired,
            last_acquired: observation.acquired,
            last_observed: observation.observed,
            first_latest: observation.latest,
            last_earliest: observation.earliest,
            last_latest: observation.latest,
            current_earliest: observation.earliest,
            local_elapsed: Duration::ZERO,
            samples: 1,
        }
    }
    pub fn observe(&mut self, observation: Observation) -> bool {
        let Some(acquisition_elapsed) = observation
            .acquired
            .checked_duration_since(self.last_acquired)
        else {
            *self = Self::new(observation);
            return false;
        };
        let Some(observation_elapsed) = observation
            .observed
            .checked_duration_since(self.last_observed)
        else {
            *self = Self::new(observation);
            return false;
        };
        let Some(local_elapsed) = observation
            .acquired
            .checked_duration_since(self.first_acquired)
        else {
            *self = Self::new(observation);
            return false;
        };
        let elapsed_floor = acquisition_elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
        let elapsed_ceil = acquisition_elapsed
            .as_nanos()
            .div_ceil(1_000_000)
            .min(u128::from(u64::MAX)) as u64;
        let projected_earliest = self.last_earliest.saturating_add(elapsed_floor);
        let projected_latest = self.last_latest.saturating_add(elapsed_ceil);
        if acquisition_elapsed > MAX_GAP
            || observation_elapsed > MAX_GAP
            || observation.latest < projected_earliest
            || observation.earliest > projected_latest
            || observation.earliest <= self.last_earliest
        {
            *self = Self::new(observation);
            return false;
        }
        self.samples = self.samples.saturating_add(1);
        self.last_acquired = observation.acquired;
        self.last_observed = observation.observed;
        self.last_earliest = observation.earliest;
        self.last_latest = observation.latest;
        self.current_earliest = observation.earliest;
        self.local_elapsed = local_elapsed;
        self.samples >= 2
            && self.local_elapsed >= Duration::from_secs(HEALTHY_SECONDS)
            && observation.earliest.saturating_sub(self.first_latest) >= HEALTHY_SECONDS * 1000
    }
    pub fn seconds(&self) -> u64 {
        self.local_elapsed
            .as_secs()
            .min(self.current_earliest.saturating_sub(self.first_latest) / 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observed(earliest: u64, latest: u64) -> Observation {
        let now = Instant::now();
        Observation::new(earliest, latest, now, now)
    }

    #[tokio::test(start_paused = true)]
    async fn review13_healthy_acquisition_bounds_need_matching_local_anchor() {
        let base = 1_000_000;
        let first = Instant::now();
        let mut proof = Healthy::new(Observation::new(base, base + 100, first, first));
        let mut accepted = false;
        for i in 1..=40_u64 {
            let processing_ms = if i % 2 == 1 { 500 } else { 0 };
            let observation_gap = if i % 2 == 1 { 2500 } else { 1500 };
            tokio::time::advance(Duration::from_millis(observation_gap)).await;
            let observed = Instant::now();
            let acquired = observed - Duration::from_millis(processing_ms);
            let lower = base + i * 2000;
            accepted |= proof.observe(Observation::new(lower, lower + 100, acquired, observed));
        }
        eprintln!(
            "REVIEW13_HEALTH_ANCHOR; local_elapsed_ms=80000; acquisition_elapsed_ms=80000; max_observation_gap_ms=2500; max_processing_delay_ms=500; seconds={}; samples={}; accepted={accepted}",
            proof.seconds(),
            proof.samples
        );
        assert!(
            accepted,
            "80 seconds of consistent fresh acquisition evidence must complete 60-second health"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r10_health_requires_both_clocks_continuity_and_new_samples() {
        let mut proof = Healthy::new(observed(1_000_000, 1_000_000));
        for i in 1..=30 {
            tokio::time::advance(Duration::from_secs(2)).await;
            assert_eq!(
                proof.observe(observed(1_000_000 + i * 2000, 1_000_000 + i * 2000,)),
                i == 30
            );
        }
        assert!(
            !proof.observe(observed(1_060_000, 1_060_000)),
            "replayed server time resets proof"
        );
        tokio::time::advance(Duration::from_secs(11)).await;
        assert!(
            !proof.observe(observed(1_071_000, 1_071_000)),
            "a silent gap is not continuous health"
        );
        for i in 1..30 {
            tokio::time::advance(Duration::from_secs(2)).await;
            assert!(!proof.observe(observed(1_071_000 + i * 2000, 1_071_000 + i * 2000,)));
        }
        assert!(
            !proof.observe(observed(2_000_000, 2_000_000)),
            "a clock jump cannot complete cooldown"
        );
    }
    #[tokio::test(start_paused = true)]
    async fn r13_health_needs_sixty_proven_seconds_across_width_changes() {
        let base = 1_000_000;
        let mut proof = Healthy::new(observed(base, base + 4500));
        for seconds in (2..=60).step_by(2) {
            tokio::time::advance(Duration::from_secs(2)).await;
            let earliest = base + seconds * 1000;
            let latest = if seconds % 4 == 0 {
                earliest + 4500
            } else {
                earliest + 100
            };
            assert!(!proof.observe(observed(earliest, latest)));
        }
        assert_eq!(
            proof.seconds(),
            55,
            "initial upper bound limits proven duration"
        );
        for seconds in [62, 64, 66] {
            tokio::time::advance(Duration::from_secs(2)).await;
            let ready = proof.observe(observed(base + seconds * 1000, base + seconds * 1000 + 100));
            assert_eq!(ready, seconds >= 66);
        }
        assert!(proof.seconds() >= 60);
    }
    #[test]
    fn r10_only_typed_network_errors_allow_recovery() {
        use super::super::transport::Error;
        assert!(transient(
            &anyhow::Error::new(Error::Network).context("bounded read")
        ));
        assert!(transient(&Error::ClockUncertain.into()));
        assert!(transient(&crate::market::ReadFailure::Network.into()));
        assert!(transient(
            &super::super::transport::ClockUncertain::new("depth", 1001, 1000, 1100).into()
        ));
        assert!(!transient(
            &super::super::transport::InvalidTimestamp::new("depth", 1200, 1000, 1100).into()
        ));
        for e in [
            Error::Unknown,
            Error::Schema,
            Error::Clock,
            Error::RateLimit,
            Error::Disabled,
            Error::NotFound,
            Error::Rejected(-2015),
        ] {
            assert!(!transient(&e.into()));
        }
        assert!(!transient(&anyhow::anyhow!(
            "Koneksi Futures gagal sebelum pengiriman"
        )));
        assert!(!transient(&crate::market::ReadFailure::RateLimit.into()));
    }
}
