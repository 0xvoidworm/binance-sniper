//! Public data for display and virtual paper simulation. No credentials, signing or order endpoints.
use anyhow::{Context, Result, bail, ensure};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::Value;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;

pub const SYMBOLS: [&str; 2] = ["BTCUSDT", "ETHUSDT"];
pub const HISTORY: usize = 240;
pub const STALE_AFTER: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, clap::ValueEnum)]
pub enum Market {
    Spot,
    #[default]
    Futures,
}
impl Market {
    pub fn label(self) -> &'static str {
        match self {
            Self::Spot => "Spot",
            Self::Futures => "USD-M Futures",
        }
    }
    fn host(self) -> &'static str {
        match self {
            Self::Spot => "api.binance.com",
            Self::Futures => "fapi.binance.com",
        }
    }
    fn prefix(self) -> &'static str {
        match self {
            Self::Spot => "/api/v3",
            Self::Futures => "/fapi/v1",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, clap::ValueEnum)]
pub enum Timeframe {
    #[default]
    #[value(name = "1m")]
    M1,
    #[value(name = "5m")]
    M5,
    #[value(name = "15m")]
    M15,
    #[value(name = "1h")]
    H1,
}
impl Timeframe {
    pub const ALL: [Self; 4] = [Self::M1, Self::M5, Self::M15, Self::H1];
    pub fn label(self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
        }
    }
    pub fn millis(self) -> i64 {
        match self {
            Self::M1 => 60_000,
            Self::M5 => 300_000,
            Self::M15 => 900_000,
            Self::H1 => 3_600_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Request {
    pub market: Market,
    pub timeframe: Timeframe,
    pub paused: bool,
    pub generation: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Candle {
    pub open_ms: i64,
    pub close_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

pub fn parse_candles(value: Value, timeframe: Timeframe) -> Result<Vec<Candle>> {
    let rows = value.as_array().context("Format candle tidak valid")?;
    ensure!(
        !rows.is_empty() && rows.len() <= HISTORY,
        "Jumlah candle tidak valid"
    );
    let mut candles = Vec::<Candle>::with_capacity(rows.len());
    for row in rows {
        let r = row.as_array().context("Format bar tidak valid")?;
        ensure!(r.len() >= 7, "Bar tidak lengkap");
        let number = |i: usize| -> Result<f64> {
            let n: f64 = r[i]
                .as_str()
                .context("Harga/volume harus string")?
                .parse()?;
            ensure!(n.is_finite(), "Harga/volume bukan angka finite");
            Ok(n)
        };
        let c = Candle {
            open_ms: r[0].as_i64().context("Waktu buka tidak valid")?,
            close_ms: r[6].as_i64().context("Waktu tutup tidak valid")?,
            open: number(1)?,
            high: number(2)?,
            low: number(3)?,
            close: number(4)?,
            volume: number(5)?,
        };
        ensure!(
            c.open_ms >= 0
                && c.open_ms % timeframe.millis() == 0
                && c.open_ms.checked_add(timeframe.millis() - 1) == Some(c.close_ms),
            "Batas waktu candle tidak valid"
        );
        ensure!(
            c.low > 0.0
                && c.high >= c.open
                && c.high >= c.close
                && c.low <= c.open
                && c.low <= c.close
                && c.volume >= 0.0,
            "OHLCV tidak valid"
        );
        if let Some(previous) = candles.last() {
            ensure!(
                previous.open_ms.checked_add(timeframe.millis()) == Some(c.open_ms),
                "Candle duplikat, tidak berurutan, atau berlubang"
            );
        }
        candles.push(c);
    }
    Ok(candles)
}

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub request: Request,
    pub candles: [Vec<Candle>; 2],
    pub server_ms: i64,
    pub fetched_ms: u128,
    pub updates: u64,
    pub recoveries: u64,
    pub failures: u32,
    pub error: Option<String>,
    pub route: String,
    #[serde(skip)]
    pub received: Option<Instant>,
}
impl Snapshot {
    pub fn empty(request: Request) -> Self {
        Self {
            request,
            candles: [vec![], vec![]],
            server_ms: 0,
            fetched_ms: 0,
            updates: 0,
            recoveries: 0,
            failures: 0,
            error: None,
            route: "Menunggu koneksi".into(),
            received: None,
        }
    }
    pub fn age(&self) -> Option<Duration> {
        self.received.map(|t| t.elapsed())
    }
    pub fn status(&self) -> &'static str {
        if self.request.paused {
            "DIJEDA"
        } else if self.error.is_some() {
            "MENGHUBUNGKAN ULANG"
        } else if self.age().is_none() {
            "MENYAMBUNG"
        } else if self.age().is_some_and(|a| a > STALE_AFTER) {
            "DATA TERLAMBAT"
        } else {
            "AKTIF · REST 2 dtk"
        }
    }
    pub fn fresh(&self) -> bool {
        !self.request.paused && self.error.is_none() && self.age().is_some_and(|a| a <= STALE_AFTER)
    }
    fn accept(&mut self, batch: Batch) {
        if self.failures > 0 {
            self.recoveries += 1;
        }
        self.candles = batch.candles;
        self.server_ms = batch.server_ms;
        self.fetched_ms = batch.elapsed.as_millis();
        self.route = batch.route;
        self.received = Some(Instant::now());
        self.error = None;
        self.failures = 0;
        self.updates += 1;
    }
}

struct Batch {
    candles: [Vec<Candle>; 2],
    server_ms: i64,
    elapsed: Duration,
    route: String,
}

pub struct Feed {
    command: watch::Sender<Request>,
    output: watch::Receiver<Snapshot>,
}
impl Feed {
    pub fn start(request: Request) -> Result<Self> {
        let (command, input) = watch::channel(request);
        let (output, receiver) = watch::channel(Snapshot::empty(request));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("binance-public-feed".into())
            .spawn(move || {
                runtime.block_on(async {
                    match Transport::new() {
                        Ok(mut source) => run_worker(input, output, &mut source).await,
                        Err(_) => {
                            output.send_modify(|s| {
                                s.error = Some("Inisialisasi koneksi gagal".into())
                            });
                        }
                    }
                });
            })?;
        Ok(Self {
            command,
            output: receiver,
        })
    }
    pub fn request(&self, request: Request) {
        self.command.send_replace(request);
    }
    pub fn latest(&self) -> Snapshot {
        self.output.borrow().clone()
    }
}

trait Source {
    async fn fetch(&mut self, request: Request) -> Result<Batch>;
    fn cooldown(&self) -> Duration {
        Duration::ZERO
    }
}

fn retry_delay(failures: u32) -> Duration {
    Duration::from_secs(2_u64.saturating_pow(failures.min(5)).min(30))
}

async fn run_worker<S: Source>(
    mut input: watch::Receiver<Request>,
    output: watch::Sender<Snapshot>,
    source: &mut S,
) {
    let mut state = Snapshot::empty(*input.borrow_and_update());
    loop {
        let mut request = *input.borrow_and_update();
        if request != state.request {
            if request.market == state.request.market
                && request.timeframe == state.request.timeframe
            {
                if state.request.paused && !request.paused {
                    state.received = None;
                }
                state.request = request;
            } else {
                state = Snapshot::empty(request);
            }
            output.send_replace(state.clone());
        }
        if request.paused {
            if input.changed().await.is_err() {
                return;
            }
            continue;
        }
        let start = Instant::now();
        let result = tokio::select! {
            biased;
            changed = input.changed() => { if changed.is_err() { return; } continue; }
            result = source.fetch(request) => result,
        };
        // A selection changed at the same instant as a response: never publish it
        // under the new selection, even when switching A -> B -> A quickly.
        request = *input.borrow();
        if request != state.request {
            continue;
        }
        let delay = match result {
            Ok(batch) => {
                state.accept(batch);
                Duration::from_secs(2).saturating_sub(start.elapsed())
            }
            Err(error) => {
                state.failures = state.failures.saturating_add(1);
                state.error = Some(error.to_string());
                retry_delay(state.failures)
            }
        }
        .max(source.cooldown());
        output.send_replace(state.clone());
        tokio::select! {
            changed = input.changed() => { if changed.is_err() { return; } }
            () = tokio::time::sleep(delay) => {}
        }
    }
}

/// Credential-free failure classification for public GETs, not order dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadFailure {
    Network,
    RateLimit,
    Invalid,
}
impl std::fmt::Display for ReadFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Network => "Koneksi publik Binance gagal",
            Self::RateLimit => "Batas laju Binance; tunggu cooldown",
            Self::Invalid => "Permintaan/respons publik tidak valid",
        })
    }
}
impl std::error::Error for ReadFailure {}
pub(crate) fn public_read_error(stage: &'static str, error: &reqwest::Error) -> ReadFailure {
    eprintln!(
        "PUBLIC_ROUTE_HEALTH; stage={stage}; timeout={}; connect={}; body={}",
        error.is_timeout(),
        error.is_connect(),
        error.is_body()
    );
    if error.is_timeout() || error.is_connect() {
        ReadFailure::Network
    } else {
        ReadFailure::Invalid
    }
}

pub(crate) fn client() -> reqwest::ClientBuilder {
    Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(7))
        .user_agent(concat!(
            "BinanceDesk/",
            env!("CARGO_PKG_VERSION"),
            " native-client"
        ))
}

pub(crate) struct Transport {
    normal: Client,
    fallback: Option<(Market, Instant, Client)>,
    limited_until: Option<Instant>,
    route: String,
}
impl Transport {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            normal: client().build()?,
            fallback: None,
            limited_until: None,
            route: "DNS sistem · TLS".into(),
        })
    }
    pub(crate) fn public_cooldown(&self) -> Duration {
        self.limited_until.map_or(Duration::ZERO, |t| {
            t.saturating_duration_since(Instant::now())
        })
    }
    pub(crate) async fn json(
        &mut self,
        market: Market,
        endpoint: &str,
        query: &[(&str, String)],
    ) -> Result<Value> {
        let started = Instant::now();
        let result = self.json_inner(market, endpoint, query).await;
        if started.elapsed() >= Duration::from_secs(1) || result.is_err() {
            // Static labels only; never echo arbitrary caller URLs/query/error bodies.
            let label = match endpoint {
                "time" | "klines" | "exchangeInfo" | "ticker/bookTicker" | "premiumIndex"
                | "depth" => endpoint,
                _ => "denied",
            };
            eprintln!(
                "PUBLIC_READ_HEALTH; endpoint={label}; elapsed_ms={}; ok={}; route={}",
                started.elapsed().as_millis(),
                result.is_ok(),
                self.route
            );
        }
        result
    }
    async fn json_inner(
        &mut self,
        market: Market,
        endpoint: &str,
        query: &[(&str, String)],
    ) -> Result<Value> {
        ensure!(
            matches!(
                endpoint,
                "time" | "klines" | "exchangeInfo" | "ticker/bookTicker" | "premiumIndex" | "depth"
            ),
            "Endpoint publik ditolak"
        );
        if endpoint == "depth" {
            ensure!(
                market == Market::Futures
                    && query.len() == 2
                    && query.iter().any(|(k, v)| *k == "symbol" && v == "BTCUSDT")
                    && query.iter().any(|(k, v)| *k == "limit" && v == "5"),
                "Depth publik dibatasi BTCUSDT Futures limit5"
            );
        }
        ensure!(self.cooldown().is_zero(), "Menunggu batas laju Binance");
        let url = format!("https://{}{}/{}", market.host(), market.prefix(), endpoint);
        let fallback = self
            .fallback
            .as_ref()
            .filter(|(m, expires, _)| *m == market && *expires > Instant::now());
        let active = fallback.map_or(&self.normal, |(_, _, c)| c);
        self.route = if fallback.is_some() {
            "DNS publik · TLS Binance"
        } else {
            "DNS sistem · TLS Binance"
        }
        .into();
        let response = match active.get(&url).query(query).send().await {
            Ok(response) => response,
            Err(e) if e.is_connect() || e.is_timeout() => {
                public_read_error("primary_get", &e);
                let (addresses, ttl) = self.resolve(market).await?;
                self.route = "DNS publik · TLS Binance".into();
                let expires = Instant::now() + ttl;
                let mut connected = None;
                for address in addresses.iter().take(4) {
                    let fallback = client().resolve(market.host(), *address).build()?;
                    match fallback.get(&url).query(query).send().await {
                        Ok(response) => {
                            self.fallback = Some((market, expires, fallback));
                            connected = Some(response);
                            break;
                        }
                        Err(error) => {
                            let failure = public_read_error("fallback_get", &error);
                            if failure != ReadFailure::Network {
                                return Err(failure.into());
                            }
                        }
                    }
                }
                connected.ok_or(ReadFailure::Network)?
            }
            Err(_) => bail!("Permintaan publik Binance gagal"),
        };
        if matches!(
            response.status(),
            StatusCode::TOO_MANY_REQUESTS | StatusCode::IM_A_TEAPOT
        ) {
            let default = if response.status() == StatusCode::IM_A_TEAPOT {
                120
            } else {
                60
            };
            let seconds = retry_after_seconds(
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok()),
                default,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            // Never retry earlier than the server's Retry-After, including after UI switches.
            self.limited_until = Instant::now().checked_add(Duration::from_secs(seconds));
            if self.limited_until.is_none() {
                self.limited_until = Instant::now().checked_add(Duration::from_secs(86_400 * 365));
            }
            return Err(anyhow::Error::new(ReadFailure::RateLimit)
                .context(format!("Batas laju Binance; tunggu {seconds} detik")));
        }
        ensure!(
            response.status().is_success(),
            "Binance HTTP {}",
            response.status().as_u16()
        );
        if endpoint == "exchangeInfo" {
            bounded_json_limit(response, 4_194_304).await
        } else {
            bounded_json(response).await
        }
    }
    async fn resolve(&self, market: Market) -> Result<(Vec<SocketAddr>, Duration)> {
        let response = self
            .normal
            .get("https://dns.google/resolve")
            .query(&[("name", market.host()), ("type", "A")])
            .send()
            .await
            .map_err(|e| {
                anyhow::Error::new(public_read_error("dns_resolver", &e))
                    .context("Resolusi DNS publik gagal")
            })?;
        ensure!(response.status().is_success(), "DNS publik tidak tersedia");
        let value = bounded_json(response).await?;
        parse_dns(value)
    }
}

pub(crate) async fn bounded_json(response: reqwest::Response) -> Result<Value> {
    bounded_json_limit(response, 1_048_576).await
}
async fn bounded_json_limit(mut response: reqwest::Response, limit: usize) -> Result<Value> {
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| {
        anyhow::Error::new(public_read_error("response_body", &e))
            .context("Pembacaan data publik gagal")
    })? {
        ensure!(
            data.len() + chunk.len() <= limit,
            "Respons publik terlalu besar"
        );
        data.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&data).context("Respons JSON publik tidak valid")
}

pub(crate) fn retry_after_seconds(header: Option<&str>, default: u64, now: u64) -> u64 {
    header
        .and_then(|value| {
            value.parse::<u64>().ok().or_else(|| {
                chrono::DateTime::parse_from_rfc2822(value)
                    .ok()
                    .and_then(|date| u64::try_from(date.timestamp()).ok())
                    .map(|timestamp| timestamp.saturating_sub(now))
            })
        })
        .unwrap_or(default)
        .max(1)
}

fn public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_documentation()
        && !ip.is_broadcast()
        && !ip.is_multicast()
        && a != 0
        && a < 240
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 198 && (b == 18 || b == 19))
        && !(a == 192 && b == 0 && c == 0)
}

pub(crate) fn parse_dns(value: Value) -> Result<(Vec<SocketAddr>, Duration)> {
    ensure!(value["Status"] == 0, "DNS publik tidak menemukan host");
    let answers = value["Answer"].as_array().context("Jawaban DNS kosong")?;
    let mut addresses = Vec::new();
    let mut ttl = 300;
    for row in answers {
        ttl = ttl.min(row["TTL"].as_u64().unwrap_or(1).clamp(1, 300));
        if row["type"] == 1 {
            let ip: Ipv4Addr = row["data"].as_str().context("Alamat DNS kosong")?.parse()?;
            ensure!(public_ipv4(ip), "Alamat DNS nonpublik ditolak");
            addresses.push(SocketAddr::new(IpAddr::V4(ip), 443));
        }
    }
    ensure!(
        !addresses.is_empty(),
        "DNS publik tidak memiliki alamat IPv4"
    );
    Ok((addresses, Duration::from_secs(ttl)))
}

impl Source for Transport {
    async fn fetch(&mut self, request: Request) -> Result<Batch> {
        let start = Instant::now();
        let server_ms = self.json(request.market, "time", &[]).await?["serverTime"]
            .as_i64()
            .context("Waktu server tidak valid")?;
        let server_received = Instant::now();
        let mut candles = [vec![], vec![]];
        for (i, symbol) in SYMBOLS.iter().enumerate() {
            let value = self
                .json(
                    request.market,
                    "klines",
                    &[
                        ("symbol", (*symbol).into()),
                        ("interval", request.timeframe.label().into()),
                        ("limit", HISTORY.to_string()),
                    ],
                )
                .await?;
            candles[i] = parse_candles(value, request.timeframe)?;
        }
        let server_ms = server_ms + server_received.elapsed().as_millis() as i64;
        for series in &candles {
            let last = series.last().context("Data candle kosong")?;
            // A request can straddle a candle boundary. Allow one short update
            // window, not an entire candle interval of silently stale data.
            ensure!(
                last.open_ms <= server_ms && last.close_ms >= server_ms - 8_000,
                "Data Binance tertinggal dari waktu server"
            );
        }
        Ok(Batch {
            candles,
            server_ms,
            elapsed: start.elapsed(),
            route: self.route.clone(),
        })
    }
    fn cooldown(&self) -> Duration {
        self.limited_until.map_or(Duration::ZERO, |t| {
            t.saturating_duration_since(Instant::now())
        })
    }
}

/// Read-only integration probe uses the same worker as the native window.
pub fn probe(path: &Path) -> Result<()> {
    ensure!(!path.exists(), "Folder probe sudah ada");
    std::fs::create_dir_all(path)?;
    let mut request = Request::default();
    let feed = Feed::start(request)?;
    for market in [Market::Futures, Market::Spot] {
        for timeframe in Timeframe::ALL {
            request = Request {
                market,
                timeframe,
                paused: false,
                generation: request.generation + 1,
            };
            feed.request(request);
            let snapshot = wait_snapshot(&feed, request, 2)?;
            let name = format!("{:?}-{}.json", market, timeframe.label());
            std::fs::write(path.join(name), serde_json::to_vec_pretty(&snapshot)?)?;
            println!(
                "{:?} {}: BTC {} / ETH {} bars, {} updates, {}",
                market,
                timeframe.label(),
                snapshot.candles[0].len(),
                snapshot.candles[1].len(),
                snapshot.updates,
                snapshot.route
            );
        }
    }
    request.paused = true;
    request.generation += 1;
    feed.request(request);
    std::thread::sleep(Duration::from_millis(150));
    let paused = feed.latest();
    ensure!(
        paused.request == request && paused.status() == "DIJEDA",
        "Jeda worker gagal"
    );
    std::thread::sleep(Duration::from_secs(3));
    ensure!(
        feed.latest().updates == paused.updates,
        "Worker tidak berhenti saat dijeda"
    );
    request.paused = false;
    request.generation += 1;
    feed.request(request);
    let resumed = wait_snapshot(&feed, request, paused.updates + 2)?;
    std::fs::write(
        path.join("resumed.json"),
        serde_json::to_vec_pretty(&resumed)?,
    )?;
    println!(
        "Jeda/resume: PASS; feed kembali segar setelah {} updates",
        resumed.updates
    );
    // Cross the common 60-second DNS TTL and keep real, timestamped observations.
    let mut observations = Vec::new();
    let stable_start = Instant::now();
    while stable_start.elapsed() < Duration::from_secs(70) {
        std::thread::sleep(Duration::from_secs(5));
        let s = feed.latest();
        observations.push(serde_json::json!({
            "elapsed_seconds": stable_start.elapsed().as_secs(),
            "server_ms": s.server_ms, "updates": s.updates,
            "status": s.status(), "age_ms": s.age().map(|a| a.as_millis()),
            "error": s.error, "recoveries": s.recoveries,
            "prices": s.candles.iter().map(|rows| rows.last().map(|c| c.close)).collect::<Vec<_>>()
        }));
    }
    std::fs::write(
        path.join("stability.json"),
        serde_json::to_vec_pretty(&observations)?,
    )?;
    let end = feed.latest();
    ensure!(
        end.fresh() && end.updates >= resumed.updates + 10,
        "Feed belum stabil setelah pemantauan 70 detik"
    );
    println!(
        "Pemantauan 70 detik: {} batch tambahan, {} pemulihan, status {}",
        end.updates - resumed.updates,
        end.recoveries,
        end.status()
    );
    Ok(())
}

fn wait_snapshot(feed: &Feed, request: Request, updates: u64) -> Result<Snapshot> {
    let start = Instant::now();
    loop {
        let snapshot = feed.latest();
        if snapshot.request == request && snapshot.updates >= updates && snapshot.fresh() {
            return Ok(snapshot);
        }
        ensure!(
            start.elapsed() < Duration::from_secs(45),
            "Probe timeout: {}",
            snapshot
                .error
                .as_deref()
                .unwrap_or_else(|| snapshot.status())
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn depth_scope_is_rejected_before_any_network_access() {
        let mut transport = Transport::new().unwrap();
        for (market, query) in [
            (
                Market::Spot,
                vec![("symbol", "BTCUSDT".into()), ("limit", "5".into())],
            ),
            (
                Market::Futures,
                vec![("symbol", "ETHUSDT".into()), ("limit", "5".into())],
            ),
            (
                Market::Futures,
                vec![("symbol", "BTCUSDT".into()), ("limit", "1000".into())],
            ),
            (Market::Futures, vec![("symbol", "BTCUSDT".into())]),
        ] {
            let error = transport.json(market, "depth", &query).await.unwrap_err();
            assert!(error.to_string().contains("Depth publik dibatasi"));
        }
    }

    fn rows() -> Value {
        json!([
            [60000, "100", "103", "99", "102", "5", 119999],
            [120000, "102", "104", "100", "101", "7", 179999]
        ])
    }
    #[test]
    fn parser_rejects_gaps_duplicates_bad_prices_and_wrong_timeframe() {
        assert_eq!(parse_candles(rows(), Timeframe::M1).unwrap().len(), 2);
        for (row, col, value) in [
            (1, 0, json!(60000)),
            (1, 0, json!(180000)),
            (0, 1, json!("NaN")),
            (0, 2, json!("98")),
            (0, 5, json!("-1")),
            (0, 6, json!(120000)),
        ] {
            let mut input = rows();
            input[row][col] = value;
            assert!(parse_candles(input, Timeframe::M1).is_err());
        }
        assert!(parse_candles(rows(), Timeframe::M5).is_err());
    }
    #[test]
    fn stale_paused_and_error_never_report_fresh() {
        let mut s = Snapshot::empty(Request::default());
        assert!(!s.fresh());
        s.received = Some(Instant::now());
        assert!(s.fresh());
        s.received = Some(Instant::now() - Duration::from_secs(9));
        assert_eq!(s.status(), "DATA TERLAMBAT");
        assert!(!s.fresh());
        s.received = Some(Instant::now());
        s.error = Some("Offline".into());
        assert!(!s.fresh());
        s.error = None;
        s.request.paused = true;
        assert!(!s.fresh());
    }
    #[test]
    fn dns_fallback_rejects_nonpublic_ips_and_respects_cname_ttl() {
        for address in [
            "127.0.0.1",
            "10.0.0.2",
            "169.254.169.254",
            "100.64.0.1",
            "224.0.0.1",
            "192.0.2.2",
            "198.18.0.1",
        ] {
            assert!(
                parse_dns(json!({"Status":0,"Answer":[{"type":1,"TTL":60,"data":address}]}))
                    .is_err()
            );
        }
        let (_, ttl) = parse_dns(json!({"Status":0,"Answer":[{"type":5,"TTL":30}, {"type":1,"TTL":60,"data":"108.138.141.5"}]})).unwrap();
        assert_eq!(ttl, Duration::from_secs(30));
    }
    #[test]
    fn backoff_is_bounded() {
        assert_eq!(retry_delay(1), Duration::from_secs(2));
        assert_eq!(retry_delay(3), Duration::from_secs(8));
        assert_eq!(retry_delay(u32::MAX), Duration::from_secs(30));
    }

    #[test]
    fn rate_limits_honor_seconds_and_http_dates() {
        assert_eq!(retry_after_seconds(Some("3600"), 60, 0), 3600);
        assert_eq!(
            retry_after_seconds(Some("Thu, 01 Jan 1970 01:00:00 GMT"), 60, 1000),
            2600
        );
        assert_eq!(retry_after_seconds(Some("0"), 60, 0), 1);
        assert_eq!(retry_after_seconds(None, 120, 0), 120);
    }

    struct Scripted {
        calls: usize,
    }
    impl Source for Scripted {
        async fn fetch(&mut self, _request: Request) -> Result<Batch> {
            self.calls += 1;
            if self.calls == 2 {
                bail!("Koneksi uji terputus");
            }
            Ok(Batch {
                candles: [
                    parse_candles(rows(), Timeframe::M1)?,
                    parse_candles(rows(), Timeframe::M1)?,
                ],
                server_ms: 150_000,
                elapsed: Duration::from_millis(5),
                route: "TEST".into(),
            })
        }
    }
    #[tokio::test(start_paused = true)]
    async fn worker_retains_history_on_disconnect_and_recovers_without_duplicates() {
        let (commands, input) = watch::channel(Request::default());
        let (output, mut observations) = watch::channel(Snapshot::empty(Request::default()));
        let worker = tokio::spawn(async move {
            run_worker(input, output, &mut Scripted { calls: 0 }).await;
        });
        observations.changed().await.unwrap();
        assert!(observations.borrow().fresh());
        observations.changed().await.unwrap();
        assert!(observations.borrow().error.is_some());
        assert_eq!(observations.borrow().candles[0].len(), 2);
        observations.changed().await.unwrap();
        assert_eq!(observations.borrow().recoveries, 1);
        assert_eq!(observations.borrow().candles[0].len(), 2);
        assert!(observations.borrow().fresh());
        drop(commands);
        worker.await.unwrap();
    }

    struct Slow;
    impl Source for Slow {
        async fn fetch(&mut self, _: Request) -> Result<Batch> {
            tokio::time::sleep(Duration::from_secs(20)).await;
            bail!("Old response must be cancelled")
        }
    }
    #[tokio::test(start_paused = true)]
    async fn changing_selection_cancels_inflight_and_pause_stops_polling() {
        let (commands, input) = watch::channel(Request::default());
        let (output, mut observations) = watch::channel(Snapshot::empty(Request::default()));
        let worker = tokio::spawn(async move {
            run_worker(input, output, &mut Slow).await;
        });
        tokio::task::yield_now().await;
        let selected = Request {
            market: Market::Spot,
            timeframe: Timeframe::H1,
            paused: true,
            generation: 3,
        };
        commands.send_replace(selected);
        observations.changed().await.unwrap();
        tokio::time::advance(Duration::from_secs(30)).await;
        assert_eq!(observations.borrow().request, selected);
        assert!(observations.borrow().error.is_none());
        assert!(observations.borrow().candles[0].is_empty());
        drop(commands);
        worker.await.unwrap();
    }
}
