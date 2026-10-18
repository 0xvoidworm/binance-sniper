//! A mutation is attempted once. DNS failover happens on public time before signing.
use super::protocol::{Command, Read, Spec};
use crate::{account, market, vault::Credentials};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_SERVER_MS: u64 = 4_102_444_800_000;
const NANOS_PER_MS: u128 = 1_000_000;
const MAX_CLOCK_WIDTH_NS: u128 = 5_000 * NANOS_PER_MS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    LiveSession,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Network,
    Unknown,
    Rejected(i64),
    NotFound,
    RateLimit,
    Schema,
    Clock,
    ClockUncertain,
    ReadWindowExpired,
    Preparation(account::Error),
}
impl Error {
    pub fn message(self) -> String {
        match self {
            Self::Disabled => "Pengiriman order belum diaktifkan dalam sesi ini".into(),
            Self::Network => "Koneksi Futures gagal sebelum pengiriman".into(),
            Self::Unknown => {
                "Hasil pengiriman belum pasti; query diperlukan, jangan kirim ulang".into()
            }
            Self::Rejected(code) => {
                format!("Binance menolak permintaan (kode {code}); isi respons disamarkan")
            }
            Self::NotFound => {
                "Order belum ditemukan; ini belum membuktikan pengiriman gagal".into()
            }
            Self::RateLimit => "Batas laju Binance; tunggu cooldown".into(),
            Self::Schema => "Respons Futures tidak cocok dengan kontrak".into(),
            Self::Clock => "Waktu Futures belum valid".into(),
            Self::ClockUncertain => {
                "Ketidakpastian sinkronisasi waktu melampaui jendela aman".into()
            }
            Self::ReadWindowExpired => {
                "Jendela baca 5 detik habis; snapshot dibuang, perlu siklus baru".into()
            }
            Self::Preparation(error) => format!("Persiapan Futures ditolak: {}", error.message()),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) struct ClockUncertain {
    source: &'static str,
    timestamp_ms: u64,
    earliest_ms: u64,
    latest_ms: u64,
}
impl ClockUncertain {
    pub(crate) fn new(
        source: &'static str,
        timestamp_ms: u64,
        earliest_ms: u64,
        latest_ms: u64,
    ) -> Self {
        Self {
            source,
            timestamp_ms,
            earliest_ms,
            latest_ms,
        }
    }
}
impl std::fmt::Display for ClockUncertain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sinkronisasi waktu {} belum pasti; timestamp_ms={}; clock_earliest_ms={}; clock_latest_ms={}",
            self.source, self.timestamp_ms, self.earliest_ms, self.latest_ms
        )
    }
}
impl std::error::Error for ClockUncertain {}

#[derive(Debug)]
pub(crate) struct InvalidTimestamp {
    source: &'static str,
    timestamp_ms: u64,
    earliest_ms: u64,
    latest_ms: u64,
}
impl InvalidTimestamp {
    pub(crate) fn new(
        source: &'static str,
        timestamp_ms: u64,
        earliest_ms: u64,
        latest_ms: u64,
    ) -> Self {
        Self {
            source,
            timestamp_ms,
            earliest_ms,
            latest_ms,
        }
    }
}
impl std::fmt::Display for InvalidTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Timestamp {} di luar rentang clock; timestamp_ms={}; age_ms={}; clock_earliest_ms={}; clock_latest_ms={}",
            self.source,
            self.timestamp_ms,
            i128::from(self.latest_ms) - i128::from(self.timestamp_ms),
            self.earliest_ms,
            self.latest_ms
        )
    }
}
impl std::error::Error for InvalidTimestamp {}

#[derive(Debug)]
pub(crate) struct StaleTimestamp {
    source: &'static str,
    age_ms: u64,
    limit_ms: u64,
}
impl StaleTimestamp {
    fn new(source: &'static str, age_ms: u64, limit_ms: u64) -> Self {
        Self {
            source,
            age_ms,
            limit_ms,
        }
    }
}
impl std::fmt::Display for StaleTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Data {} usang; age_ms={}; limit_ms={}",
            self.source, self.age_ms, self.limit_ms
        )
    }
}
impl std::error::Error for StaleTimestamp {}

#[derive(Debug)]
pub(crate) struct ClockRollback {
    previous_earliest_ms: u64,
    current_latest_ms: u64,
}
impl ClockRollback {
    pub(crate) fn new(previous_earliest_ms: u64, current_latest_ms: u64) -> Self {
        Self {
            previous_earliest_ms,
            current_latest_ms,
        }
    }
}
impl std::fmt::Display for ClockRollback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Waktu sesi bergerak mundur; previous_earliest_ms={}; current_latest_ms={}",
            self.previous_earliest_ms, self.current_latest_ms
        )
    }
}
impl std::error::Error for ClockRollback {}

pub struct OrderBatch {
    /// Conservative age of the first GET, excluding DNS/clock preparation.
    pub(crate) started: Instant,
    pub(crate) results: Vec<Result<Value>>,
}
impl OrderBatch {
    fn immediate(results: Vec<Result<Value>>) -> Self {
        Self {
            started: Instant::now(),
            results,
        }
    }
}

fn valid_order_batch(reads: &[Read]) -> bool {
    reads.len() <= 16
        && reads
            .iter()
            .all(|r| matches!(r, Read::Order { .. } | Read::Algo { .. }) && r.spec().is_ok())
}

pub trait Exchange {
    fn live_enabled(&self) -> bool;
    fn read(&mut self, read: Read) -> impl std::future::Future<Output = Result<Value>>;
    /// Bounded GET-only reconciliation. Individual failures never erase receipts.
    fn read_orders(&mut self, reads: &[Read]) -> impl std::future::Future<Output = OrderBatch> {
        async move {
            if !valid_order_batch(reads) {
                return OrderBatch::immediate(vec![Err(Error::Schema); reads.len().min(16)]);
            }
            let started = Instant::now();
            let mut results = Vec::with_capacity(reads.len());
            for read in reads {
                results.push(self.read(read.clone()).await);
            }
            OrderBatch { started, results }
        }
    }
    fn write_once(&mut self, command: &Command)
    -> impl std::future::Future<Output = Result<Value>>;
}
pub struct Connector {
    route: account::Transport,
    credentials: Credentials,
    access: Access,
    limited_until: Mutex<Option<Instant>>,
    read_window: Option<ReadWindow>,
    last_clock: Option<ClockWindow>,
}
// A clock/route lease for one GET-only decision cycle, not a data snapshot.
// Account/order/quote age is still measured independently from original reads.
struct ReadWindow {
    client: Client,
    sample: account::ClockSample,
    clock: ClockWindow,
}
impl ReadWindow {
    fn clock(&self) -> Result<ClockWindow> {
        if self.sample.completed().elapsed() > Duration::from_secs(5) {
            return Err(Error::ReadWindowExpired);
        }
        self.clock.now()
    }
    fn signing_now(&self) -> Result<u64> {
        if self.sample.completed().elapsed() > Duration::from_secs(5) {
            return Err(Error::ReadWindowExpired);
        }
        if self.sample.uncertainty() > Duration::from_secs(5) {
            return Err(Error::ClockUncertain);
        }
        self.sample
            .signing_at(Instant::now())
            .map_err(preparation_error)
    }
}

/// A server-clock range anchored to a local monotonic instant. `earliest` is
/// safe for proving an event has happened; `latest` is safe for freshness,
/// deadlines and avoiding late signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClockWindow {
    // Exact sub-millisecond bounds are retained across projection and
    // intersection. Integer accessors round outward only at the decision edge.
    earliest_ns: u128,
    latest_ns: u128,
    at: Instant,
    local_at: tokio::time::Instant,
}
impl ClockWindow {
    #[cfg(test)]
    pub(crate) fn review12_intersection(self, other: Self, at: Instant) -> Self {
        self.intersect_at(other, at)
            .unwrap()
            .expect("consistent ranges")
    }
    pub(crate) fn from_sample(sample: account::ClockSample, at: Instant) -> Result<Self> {
        let (earliest_ns, latest_ns) = sample.bounds_at_nanos(at).map_err(preparation_error)?;
        Self::from_nanos(earliest_ns, latest_ns, at)
    }
    #[cfg(test)]
    pub(crate) fn new(earliest_ms: u64, latest_ms: u64, at: Instant) -> Result<Self> {
        let earliest_ns = u128::from(earliest_ms) * NANOS_PER_MS;
        let latest_ns = u128::from(latest_ms) * NANOS_PER_MS;
        Self::from_anchors(earliest_ns, latest_ns, at, tokio::time::Instant::now())
    }
    #[cfg(test)]
    pub(crate) fn new_anchored(
        earliest_ms: u64,
        latest_ms: u64,
        at: Instant,
        local_at: tokio::time::Instant,
    ) -> Result<Self> {
        let earliest_ns = u128::from(earliest_ms) * NANOS_PER_MS;
        let latest_ns = u128::from(latest_ms) * NANOS_PER_MS;
        Self::from_anchors(earliest_ns, latest_ns, at, local_at)
    }
    fn from_nanos(earliest_ns: u128, latest_ns: u128, at: Instant) -> Result<Self> {
        let observed = Instant::now();
        let local_observed = tokio::time::Instant::now();
        let local_at = if let Some(age) = observed.checked_duration_since(at) {
            local_observed.checked_sub(age).ok_or(Error::Clock)?
        } else {
            local_observed
                .checked_add(at.duration_since(observed))
                .ok_or(Error::Clock)?
        };
        Self::from_anchors(earliest_ns, latest_ns, at, local_at)
    }
    fn from_anchors(
        earliest_ns: u128,
        latest_ns: u128,
        at: Instant,
        local_at: tokio::time::Instant,
    ) -> Result<Self> {
        if earliest_ns == 0 || earliest_ns > latest_ns {
            return Err(Error::Clock);
        }
        if latest_ns - earliest_ns > MAX_CLOCK_WIDTH_NS {
            return Err(Error::ClockUncertain);
        }
        let latest_ms = Self::ceil_ms(latest_ns).ok_or(Error::Clock)?;
        if latest_ms > MAX_SERVER_MS {
            return Err(Error::Clock);
        }
        Ok(Self {
            earliest_ns,
            latest_ns,
            at,
            local_at,
        })
    }
    fn floor_ms(value: u128) -> Option<u64> {
        (value / NANOS_PER_MS).try_into().ok()
    }
    fn ceil_ms(value: u128) -> Option<u64> {
        value
            .checked_add(NANOS_PER_MS - 1)
            .map(|rounded| rounded / NANOS_PER_MS)
            .and_then(|rounded| rounded.try_into().ok())
    }
    pub(crate) fn advanced(self, at: Instant) -> Result<Self> {
        let elapsed = at.checked_duration_since(self.at).ok_or(Error::Clock)?;
        Self::from_anchors(
            self.earliest_ns
                .checked_add(elapsed.as_nanos())
                .ok_or(Error::Clock)?,
            self.latest_ns
                .checked_add(elapsed.as_nanos())
                .ok_or(Error::Clock)?,
            at,
            self.local_at.checked_add(elapsed).ok_or(Error::Clock)?,
        )
    }
    pub(crate) fn now(self) -> Result<Self> {
        self.advanced(Instant::now())
    }
    pub(crate) fn local_anchor(self) -> tokio::time::Instant {
        self.local_at
    }
    fn intersect_at(self, other: Self, at: Instant) -> Result<Option<Self>> {
        let left = self.advanced(at)?;
        let right = other.advanced(at)?;
        let earliest = left.earliest_ns.max(right.earliest_ns);
        let latest = left.latest_ns.min(right.latest_ns);
        if earliest > latest {
            return Ok(None);
        }
        Self::from_anchors(earliest, latest, at, left.local_at).map(Some)
    }
    pub(crate) fn earliest(self) -> u64 {
        Self::floor_ms(self.earliest_ns).unwrap_or(u64::MAX)
    }
    pub(crate) fn latest(self) -> u64 {
        Self::ceil_ms(self.latest_ns).unwrap_or(u64::MAX)
    }
    #[cfg(test)]
    pub(crate) fn uncertainty_ms(self) -> u64 {
        Self::ceil_ms(self.latest_ns.saturating_sub(self.earliest_ns)).unwrap_or(u64::MAX)
    }
    pub(crate) fn current_earliest(self) -> u64 {
        self.now().map_or(u64::MAX, Self::earliest)
    }
    pub(crate) fn current_latest(self) -> u64 {
        self.now().map_or(u64::MAX, Self::latest)
    }
    pub(crate) fn validate_age(
        self,
        source: &'static str,
        timestamp_ms: u64,
        limit_ms: u64,
    ) -> anyhow::Result<u64> {
        let earliest_ms = self.earliest();
        let latest_ms = self.latest();
        if timestamp_ms > latest_ms {
            return Err(InvalidTimestamp::new(source, timestamp_ms, earliest_ms, latest_ms).into());
        }
        if timestamp_ms > earliest_ms {
            return Err(ClockUncertain::new(source, timestamp_ms, earliest_ms, latest_ms).into());
        }
        let youngest = earliest_ms - timestamp_ms;
        let oldest = latest_ms - timestamp_ms;
        if youngest > limit_ms {
            return Err(StaleTimestamp::new(source, youngest, limit_ms).into());
        }
        if oldest > limit_ms {
            return Err(ClockUncertain::new(source, timestamp_ms, earliest_ms, latest_ms).into());
        }
        Ok(oldest)
    }
    pub(crate) fn prove_future(
        self,
        source: &'static str,
        timestamp_ms: u64,
    ) -> anyhow::Result<()> {
        let earliest_ms = self.earliest();
        let latest_ms = self.latest();
        if timestamp_ms > latest_ms {
            return Ok(());
        }
        if timestamp_ms > earliest_ms {
            return Err(ClockUncertain::new(source, timestamp_ms, earliest_ms, latest_ms).into());
        }
        Err(InvalidTimestamp::new(source, timestamp_ms, earliest_ms, latest_ms).into())
    }
}

#[test]
fn p1_read_window_boundary_and_clock_overflow_fail_closed() {
    let at = Instant::now();
    assert!(ClockWindow::new(10_000, 15_000, at).is_ok());
    assert_eq!(
        ClockWindow::new(10_000, 15_001, at),
        Err(Error::ClockUncertain)
    );
    assert_eq!(ClockWindow::new(u64::MAX, u64::MAX, at), Err(Error::Clock));
}

#[test]
fn r12_successive_clock_samples_intersect_or_force_one_revalidation_cycle() {
    let at = Instant::now();
    let previous = ClockWindow::new(10_000, 10_120, at).unwrap();
    let overlapping = ClockWindow::new(10_050, 10_170, at).unwrap();
    let narrowed = previous.intersect_at(overlapping, at).unwrap().unwrap();
    assert_eq!((narrowed.earliest(), narrowed.latest()), (10_050, 10_120));
    let jumped = ClockWindow::new(10_121, 10_241, at).unwrap();
    assert!(previous.intersect_at(jumped, at).unwrap().is_none());
}

#[test]
fn r13_fractional_projection_and_reanchor_keep_the_same_enclosing_bounds() {
    let started = Instant::now();
    let completed = started + Duration::from_nanos(400_001);
    let sample = account::ClockSample::new(1_000_000, started, completed).unwrap();
    let base_at = completed + Duration::from_nanos(400_001);
    let base = ClockWindow::from_sample(sample, base_at).unwrap();
    assert_eq!((base.earliest(), base.latest()), (1_000_000, 1_000_001));

    let middle_at = base_at + Duration::from_nanos(400_001);
    let final_at = middle_at + Duration::from_nanos(400_001);
    let repeated = base
        .advanced(middle_at)
        .unwrap()
        .advanced(final_at)
        .unwrap();
    let direct = base.advanced(final_at).unwrap();
    assert_eq!(
        repeated, direct,
        "reanchoring cannot accumulate rounding loss"
    );
    assert_eq!((direct.earliest(), direct.latest()), (1_000_001, 1_000_002));

    let intersected = base
        .intersect_at(
            ClockWindow::new(1_000_000, 1_000_002, base_at).unwrap(),
            base_at,
        )
        .unwrap()
        .unwrap();
    assert_eq!(intersected.earliest(), base.earliest());
    assert_eq!(intersected.latest(), base.latest());
}

#[test]
fn r11_snapshot_clock_is_local_monotonic_bounded_and_never_renews_lease() {
    let credentials =
        Credentials::new("fixture-only-key".into(), "fixture-only-secret".into()).unwrap();
    let mut c = Connector::new(credentials, Access::ReadOnly).unwrap();
    assert_eq!(
        c.snapshot_clock(),
        Err(Error::Clock),
        "no implicit preparation/network"
    );
    let completed = Instant::now() - Duration::from_millis(400);
    c.read_window = Some(ReadWindow {
        client: Client::new(),
        sample: account::ClockSample::new(10000, completed - Duration::from_millis(120), completed)
            .unwrap(),
        clock: ClockWindow::new(10400, 10520, Instant::now()).unwrap(),
    });
    let first = c.snapshot_clock().unwrap();
    let second = c.snapshot_clock().unwrap();
    assert!(first.earliest() >= 10400 && second.earliest() >= first.earliest());
    assert_eq!(first.uncertainty_ms(), 120);
    let expired = Instant::now() - Duration::from_millis(7157);
    c.read_window.as_mut().unwrap().sample =
        account::ClockSample::new(10000, expired - Duration::from_millis(120), expired).unwrap();
    assert_eq!(c.snapshot_clock(), Err(Error::ReadWindowExpired));
    assert!(super::recovery::transient(&Error::ReadWindowExpired.into()));
    c.limit_for(60).unwrap();
    assert_eq!(
        c.snapshot_clock(),
        Err(Error::RateLimit),
        "staleness cannot bypass cooldown"
    );
    c.end_read_window();
    assert!(c.read_window.is_none());
}

#[tokio::test]
async fn p1_read_preparation_reuses_only_a_live_lease_and_preserves_cooldown() {
    let credentials =
        Credentials::new("fixture-only-key".into(), "fixture-only-secret".into()).unwrap();
    let mut c = Connector::new(credentials, Access::ReadOnly).unwrap();
    c.read_window = Some(ReadWindow {
        client: Client::new(),
        sample: account::ClockSample::new(10000, Instant::now(), Instant::now()).unwrap(),
        clock: ClockWindow::new(10000, 10000, Instant::now()).unwrap(),
    });
    let (_, first) = c.read_preparation().await.unwrap();
    let (_, second) = c.read_preparation().await.unwrap();
    assert!(second >= first && first >= 10000);
    let expired = Instant::now() - Duration::from_millis(5069);
    c.read_window.as_mut().unwrap().sample =
        account::ClockSample::new(10000, expired, expired).unwrap();
    assert!(matches!(
        c.read_preparation().await,
        Err(Error::ReadWindowExpired)
    ));
    c.limit_for(60).unwrap();
    assert!(matches!(c.read_preparation().await, Err(Error::RateLimit)));
    c.end_read_window();
    assert!(c.read_window.is_none());
    assert!(!c.live_enabled());
}
impl Connector {
    pub fn new(credentials: Credentials, access: Access) -> Result<Self> {
        Ok(Self {
            route: account::Transport::new().map_err(|_| Error::Network)?,
            credentials,
            access,
            limited_until: Mutex::new(None),
            read_window: None,
            last_clock: None,
        })
    }
    pub fn cooldown(&self) -> Duration {
        self.limited_until
            .lock()
            .map(|until| *until)
            .unwrap_or_else(|_| Instant::now().checked_add(Duration::from_secs(365 * 86400)))
            .map_or(Duration::ZERO, |t| {
                t.saturating_duration_since(Instant::now())
            })
            .max(self.route.cooldown())
    }
    fn limit_for(&self, seconds: u64) -> Result<()> {
        let until = Instant::now()
            .checked_add(Duration::from_secs(seconds))
            .or_else(|| Instant::now().checked_add(Duration::from_secs(365 * 86400)))
            .ok_or(Error::RateLimit)?;
        let mut stored = self.limited_until.lock().map_err(|_| Error::RateLimit)?;
        *stored = Some(stored.map_or(until, |previous| previous.max(until)));
        Ok(())
    }
    async fn prepare(&mut self) -> Result<(Client, account::ClockSample)> {
        if !self.cooldown().is_zero() {
            return Err(Error::RateLimit);
        }
        let started = Instant::now();
        let (client, sample) = self.route.prepare().await.map_err(|error| {
            eprintln!(
                "LIVE_TRANSPORT_HEALTH; stage=dns_clock; outcome=error; category={error:?}; elapsed_ms={}",
                started.elapsed().as_millis()
            );
            preparation_error(error)
        })?;
        if started.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "LIVE_TRANSPORT_HEALTH; stage=dns_clock; outcome=slow; elapsed_ms={}",
                started.elapsed().as_millis()
            );
        }
        Ok((client, sample))
    }
    async fn perform(&mut self, spec: Spec, mutation: bool) -> Result<Value> {
        if mutation && self.access != Access::LiveSession {
            return Err(Error::Disabled);
        }
        // All public DNS/time retries complete before a mutation can leave this process.
        let (client, now) = if mutation {
            // Mutations never inherit a GET lease, including after cancellation.
            self.end_read_window();
            let (client, sample) = self.prepare().await?;
            if sample.uncertainty() > Duration::from_secs(5) {
                return Err(Error::ClockUncertain);
            }
            let now = sample
                .signing_at(Instant::now())
                .map_err(preparation_error)?;
            (client, now)
        } else {
            self.read_preparation().await?
        };
        self.perform_prepared(spec, mutation, &client, now).await
    }
    pub(crate) async fn begin_read_window(&mut self) -> Result<()> {
        self.end_read_window();
        let (client, sample) = self.prepare().await?;
        // A sample wider than the signed receive window cannot safely support
        // either private GETs or an order decision.
        if sample.uncertainty() > Duration::from_secs(5) {
            return Err(Error::ClockUncertain);
        }
        sample
            .signing_at(Instant::now())
            .map_err(preparation_error)?;
        let at = Instant::now();
        let measured = ClockWindow::from_sample(sample, at)?;
        let clock = match self.last_clock {
            Some(previous) => match previous.intersect_at(measured, at)? {
                Some(intersection) => intersection,
                None => {
                    // Keep the newest evidence as the baseline so one discarded
                    // cycle can be followed by an independently consistent one.
                    self.last_clock = Some(measured);
                    return Err(Error::ClockUncertain);
                }
            },
            None => measured,
        };
        // Keep the raw measured range for the next comparison. Repeatedly
        // carrying intersections forward would manufacture false precision.
        self.last_clock = Some(measured);
        self.read_window = Some(ReadWindow {
            client,
            sample,
            clock,
        });
        Ok(())
    }
    pub(crate) fn end_read_window(&mut self) {
        self.read_window = None;
    }
    /// The same server clock prepared BEFORE mutable order/account/quote reads.
    /// No network request, lease extension or OS wall-clock substitution.
    pub(crate) fn snapshot_clock(&self) -> Result<ClockWindow> {
        if !self.cooldown().is_zero() {
            return Err(Error::RateLimit);
        }
        self.read_window.as_ref().ok_or(Error::Clock)?.clock()
    }
    async fn read_preparation(&mut self) -> Result<(Client, u64)> {
        if !self.cooldown().is_zero() {
            return Err(Error::RateLimit);
        }
        if let Some(window) = &self.read_window {
            // Do not perform a slow second DNS/clock round inside evidence age.
            // An expired lease fails closed; the next cycle prepares anew.
            return Ok((window.client.clone(), window.signing_now()?));
        }
        let (client, sample) = self.prepare().await?;
        if sample.uncertainty() > Duration::from_secs(5) {
            return Err(Error::ClockUncertain);
        }
        let now = sample
            .signing_at(Instant::now())
            .map_err(preparation_error)?;
        Ok((client, now))
    }
    /// Exactly the six independent account GETs, with one original start time.
    /// Never used for mutation, order reconciliation or arbitrary request batches.
    pub(crate) async fn read_batch(&mut self, reads: &[Read]) -> Result<(Vec<Value>, Instant)> {
        const READS: [Read; 6] = [
            Read::Account,
            Read::Positions,
            Read::AccountConfig,
            Read::Symbols,
            Read::OpenOrders(super::protocol::Symbol::BTCUSDT),
            Read::OpenAlgos(super::protocol::Symbol::BTCUSDT),
        ];
        if reads != READS {
            return Err(Error::Schema);
        }
        let (client, server_ms) = self.read_preparation().await?;
        let at = Instant::now();
        let times: [AtomicU64; 6] = std::array::from_fn(|_| AtomicU64::new(0));
        let connector: &Self = self;
        let requests = std::array::from_fn(|index| {
            let client = &client;
            let times = &times;
            async move {
                if !connector.cooldown().is_zero() {
                    return Err(Error::RateLimit);
                }
                let now = server_ms
                    .checked_add(at.elapsed().as_millis() as u64)
                    .ok_or(Error::Clock)?;
                let started = Instant::now();
                let result = connector
                    .perform_prepared(
                        READS[index].spec().map_err(|_| Error::Schema)?,
                        false,
                        client,
                        now,
                    )
                    .await;
                times[index].store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                result
            }
        });
        let result = collect_snapshot(requests).await;
        eprintln!(
            "LIVE_ACCOUNT_BATCH; ok={}; reads=6; elapsed_ms={}; account_ms={}; positions_ms={}; account_config_ms={}; symbol_config_ms={}; open_orders_ms={}; open_algos_ms={}",
            result.is_ok(),
            at.elapsed().as_millis(),
            times[0].load(Ordering::Relaxed),
            times[1].load(Ordering::Relaxed),
            times[2].load(Ordering::Relaxed),
            times[3].load(Ordering::Relaxed),
            times[4].load(Ordering::Relaxed),
            times[5].load(Ordering::Relaxed),
        );
        Ok((result?.into_iter().collect(), at))
    }
    async fn perform_prepared(
        &self,
        spec: Spec,
        mutation: bool,
        client: &Client,
        now: u64,
    ) -> Result<Value> {
        if mutation && self.access != Access::LiveSession {
            return Err(Error::Disabled);
        }
        let request = spec
            .signed(client, &self.credentials, now)
            .map_err(|_| Error::Schema)?;
        let started = Instant::now();
        let response = client.execute(request).await.map_err(|error| {
            eprintln!("LIVE_TRANSPORT_HEALTH; method={}; endpoint={}; stage=http; timeout={}; connect={}; elapsed_ms={}",spec.method,spec.path,error.is_timeout(),error.is_connect(),started.elapsed().as_millis());
            if mutation {
                Error::Unknown
            } else if error.is_timeout() || error.is_connect() {
                Error::Network
            } else {
                Error::Schema
            }
        })?;
        self.receive_prepared(spec, mutation, response, started)
            .await
    }
    async fn receive_prepared(
        &self,
        spec: Spec,
        mutation: bool,
        response: reqwest::Response,
        started: Instant,
    ) -> Result<Value> {
        let status = response.status();
        if matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS | StatusCode::IM_A_TEAPOT
        ) {
            let secs = market::retry_after_seconds(
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|s| s.to_str().ok()),
                if status == StatusCode::IM_A_TEAPOT {
                    120
                } else {
                    60
                },
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |t| t.as_secs()),
            );
            self.limit_for(secs)?;
        }
        let value = market::bounded_json(response).await.map_err(|_| {
            eprintln!("LIVE_TRANSPORT_HEALTH; method={}; endpoint={}; stage=json; http_status={}; elapsed_ms={}",spec.method,spec.path,status.as_u16(),started.elapsed().as_millis());
            if mutation {
                Error::Unknown
            } else {
                Error::Schema
            }
        })?;
        let code = super::protocol::code(&value);
        if matches!(code, Some(-1003 | -1015)) && self.cooldown().is_zero() {
            self.limit_for(60)?;
        }
        classify(status, code, mutation).inspect_err(|error| {
            eprintln!("LIVE_TRANSPORT_HEALTH; method={}; endpoint={}; stage=api; http_status={}; category={error:?}; elapsed_ms={}",spec.method,spec.path,status.as_u16(),started.elapsed().as_millis());
        })?;
        if started.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "LIVE_TRANSPORT_HEALTH; method={}; endpoint={}; stage=http; outcome=slow; elapsed_ms={}",
                spec.method,
                spec.path,
                started.elapsed().as_millis()
            );
        }
        Ok(value)
    }
}

// Kept in one function so deterministic tests exercise the actual scheduling
// used by the account snapshot, not a separate latency calculator.
async fn collect_snapshot<T, F: Future<Output = Result<T>>>(requests: [F; 6]) -> Result<[T; 6]> {
    let [a, b, c, d, e, f] = requests;
    // No spawned/detached jobs. Error, Stop or deadline drops all pending GETs.
    // try_join preserves endpoint order, regardless of completion order.
    let (a, b, c, d, e, f) = tokio::try_join!(a, b, c, d, e, f)?;
    Ok([a, b, c, d, e, f])
}

fn preparation_error(error: account::Error) -> Error {
    match error {
        account::Error::Network => Error::Network,
        account::Error::RateLimit => Error::RateLimit,
        account::Error::Clock => Error::Clock,
        account::Error::Payload | account::Error::Schema(..) => Error::Schema,
        other => Error::Preparation(other),
    }
}

impl Exchange for Connector {
    fn live_enabled(&self) -> bool {
        self.access == Access::LiveSession
    }
    async fn read(&mut self, read: Read) -> Result<Value> {
        self.perform(read.spec().map_err(|_| Error::Schema)?, false)
            .await
    }
    async fn read_orders(&mut self, reads: &[Read]) -> OrderBatch {
        if reads.is_empty() {
            return OrderBatch::immediate(vec![]);
        }
        if !valid_order_batch(reads) {
            return OrderBatch::immediate(vec![Err(Error::Schema); reads.len().min(16)]);
        }
        let (client, server_ms) = match self.read_preparation().await {
            Ok(prepared) => prepared,
            Err(error) => return OrderBatch::immediate(vec![Err(error); reads.len()]),
        };
        let at = Instant::now();
        let mut results = Vec::with_capacity(reads.len());
        for read in reads {
            let result = match read.spec() {
                Err(_) => Err(Error::Schema),
                Ok(_) if !self.cooldown().is_zero() => Err(Error::RateLimit),
                Ok(spec) => match server_ms.checked_add(at.elapsed().as_millis() as u64) {
                    Some(now) => self.perform_prepared(spec, false, &client, now).await,
                    None => Err(Error::Clock),
                },
            };
            results.push(result);
        }
        OrderBatch {
            started: at,
            results,
        }
    }
    async fn write_once(&mut self, command: &Command) -> Result<Value> {
        if !self.live_enabled() {
            return Err(Error::Disabled);
        }
        self.perform(command.spec().map_err(|_| Error::Schema)?, true)
            .await
    }
}
pub fn classify(status: StatusCode, code: Option<i64>, mutation: bool) -> Result<()> {
    // 503 and 408 can mean the matching engine accepted an order. A malformed
    // or missing response never becomes "rejected" just because it is not 2xx.
    if status.is_server_error()
        || status == StatusCode::REQUEST_TIMEOUT
        || matches!(code, Some(-1006 | -1007))
    {
        return Err(if mutation {
            Error::Unknown
        } else {
            Error::Network
        });
    }
    if matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS | StatusCode::IM_A_TEAPOT
    ) || matches!(code, Some(-1003 | -1015))
    {
        return Err(Error::RateLimit);
    }
    if matches!(code, Some(-2013 | -2011)) && !mutation {
        return Err(Error::NotFound);
    }
    if let Some(code) = code.filter(|c| *c < 0) {
        return Err(Error::Rejected(code));
    }
    if !status.is_success() {
        return Err(if mutation {
            Error::Unknown
        } else if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            Error::Preparation(account::Error::Auth)
        } else {
            Error::Schema
        });
    }
    Ok(())
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicUsize};

    #[test]
    fn r10_http_auth_and_invalid_reads_cannot_grant_recovery() {
        for status in [
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
            StatusCode::NOT_FOUND,
        ] {
            let e = classify(status, None, false).unwrap_err();
            assert!(!super::super::recovery::transient(&e.into()), "{status}");
            assert_eq!(classify(status, None, true), Err(Error::Unknown));
        }
        for (status, code) in [
            (StatusCode::SERVICE_UNAVAILABLE, None),
            (StatusCode::REQUEST_TIMEOUT, None),
            (StatusCode::OK, Some(-1007)),
        ] {
            assert_eq!(classify(status, code, false), Err(Error::Network));
            assert_eq!(classify(status, code, true), Err(Error::Unknown));
        }
    }

    #[test]
    fn r10_preparation_preserves_auth_rate_clock_and_schema_errors() {
        for (input, expected) in [
            (account::Error::Network, Error::Network),
            (account::Error::RateLimit, Error::RateLimit),
            (account::Error::Clock, Error::Clock),
            (account::Error::Payload, Error::Schema),
            (account::Error::Schema("time", "clock"), Error::Schema),
            (
                account::Error::Auth,
                Error::Preparation(account::Error::Auth),
            ),
            (
                account::Error::Restricted,
                Error::Preparation(account::Error::Restricted),
            ),
            (
                account::Error::Unavailable,
                Error::Preparation(account::Error::Unavailable),
            ),
        ] {
            assert_eq!(preparation_error(input), expected);
            assert_eq!(
                super::super::recovery::transient(&expected.into()),
                input == account::Error::Network
            );
        }
    }

    struct Running(Arc<AtomicUsize>);
    impl Drop for Running {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn p3_snapshot_error_and_deadline_drop_all_pending_reads() {
        for fail in [false, true] {
            let active = Arc::new(AtomicUsize::new(0));
            let launched = Arc::new(AtomicUsize::new(0));
            let requests = std::array::from_fn(|index| {
                let active = active.clone();
                let launched = launched.clone();
                async move {
                    active.fetch_add(1, Ordering::Relaxed);
                    launched.fetch_add(1, Ordering::Relaxed);
                    let _running = Running(active);
                    tokio::time::sleep(Duration::from_millis(if index == 3 && fail {
                        10
                    } else {
                        600_000
                    }))
                    .await;
                    Err::<(), _>(Error::Network)
                }
            });
            let result =
                tokio::time::timeout(Duration::from_millis(20), collect_snapshot(requests)).await;
            assert_eq!(launched.load(Ordering::Relaxed), 6);
            assert_eq!(
                active.load(Ordering::Relaxed),
                0,
                "No read task may survive cancellation"
            );
            if fail {
                assert_eq!(result.unwrap(), Err(Error::Network));
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[tokio::test]
    async fn p3_account_batch_rejects_non_snapshot_reads_and_preserves_original_age() {
        let credentials =
            Credentials::new("fixture-only-key".into(), "fixture-only-secret".into()).unwrap();
        let mut c = Connector::new(credentials, Access::ReadOnly).unwrap();
        for reads in [
            vec![],
            vec![Read::Account; 6],
            vec![Read::FeeBurn; 6],
            vec![Read::Symbols; 17],
        ] {
            assert!(matches!(c.read_batch(&reads).await, Err(Error::Schema)));
        }
        assert!(!c.live_enabled());
        // The guard remains independent of whether the six GETs were parallel.
        let completed = Instant::now() - Duration::from_millis(5407);
        let window = ReadWindow {
            client: Client::new(),
            sample: account::ClockSample::new(10000, completed, completed).unwrap(),
            clock: ClockWindow::new(10000, 10000, completed).unwrap(),
        };
        assert_eq!(window.clock(), Err(Error::ReadWindowExpired));
    }

    fn response(status: u16, after: &str, body: &'static str) -> reqwest::Response {
        http::Response::builder()
            .status(status)
            .header("retry-after", after)
            .body(reqwest::Body::from(body))
            .unwrap()
            .into()
    }

    #[tokio::test]
    async fn p3_shared_rate_limit_survives_malformed_body_and_never_shortens() {
        let credentials =
            Credentials::new("fixture-only-key".into(), "fixture-only-secret".into()).unwrap();
        let mut c = Connector::new(credentials, Access::ReadOnly).unwrap();
        assert_eq!(
            c.receive_prepared(
                Read::Symbols.spec().unwrap(),
                false,
                response(429, "120", "invalid-json"),
                Instant::now()
            )
            .await,
            Err(Error::Schema)
        );
        assert!(c.cooldown() > Duration::from_secs(110));
        assert_eq!(
            c.receive_prepared(
                Read::Account.spec().unwrap(),
                false,
                response(429, "1", "{}"),
                Instant::now()
            )
            .await,
            Err(Error::RateLimit)
        );
        assert!(c.cooldown() > Duration::from_secs(110));
        assert!(matches!(c.read_preparation().await, Err(Error::RateLimit)));
        assert_eq!(
            c.receive_prepared(
                Read::Account.spec().unwrap(),
                false,
                response(418, "180", "{}"),
                Instant::now()
            )
            .await,
            Err(Error::RateLimit)
        );
        assert!(c.cooldown() > Duration::from_secs(170));
    }

    #[tokio::test]
    async fn p3_parallel_get_response_errors_never_become_valid_values() {
        let credentials =
            Credentials::new("fixture-only-key".into(), "fixture-only-secret".into()).unwrap();
        let c = Connector::new(credentials, Access::ReadOnly).unwrap();
        for (status, body, error) in [
            (200, "invalid-json", Error::Schema),
            (503, "{}", Error::Network),
            (
                200,
                "{\"code\":-2015,\"msg\":\"secret-canary\"}",
                Error::Rejected(-2015),
            ),
        ] {
            let result = c
                .receive_prepared(
                    Read::Symbols.spec().unwrap(),
                    false,
                    response(status, "0", body),
                    Instant::now(),
                )
                .await;
            assert_eq!(result, Err(error));
            assert!(!error.to_string().contains("secret-canary"));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn p3_slow_symbol_read_does_not_accumulate_other_account_latencies() {
        // Real incident: symbolConfig 4310ms, total account age 5407ms.
        // The remaining split below is synthetic, not claimed production timing.
        let started = tokio::time::Instant::now();
        let delays = [200, 200, 209, 4310, 100, 100];
        let requests = std::array::from_fn(|index| async move {
            tokio::time::sleep(Duration::from_millis(delays[index])).await;
            Ok(index)
        });
        let values = collect_snapshot(requests).await.unwrap();
        tokio::time::sleep(Duration::from_millis(288)).await;
        assert_eq!(values, [0, 1, 2, 3, 4, 5]);
        let age = started.elapsed().as_millis();
        assert!(
            age <= 5000,
            "Account evidence expired at {age}ms; serial topology recreates 5407ms failure"
        );
        assert_eq!(age, 4598);
    }
}
