//! USD-M account observation. GET-only, fixed host/routes, no raw error logging.
use crate::{
    market,
    vault::{self, Credentials, VaultError},
};
use hmac::{Hmac, KeyInit, Mac};
use reqwest::{
    Client, Method, Request, StatusCode,
    header::{HeaderName, HeaderValue},
};
use rust_decimal::Decimal as D;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use std::{
    collections::BTreeSet,
    fmt::Write,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, watch};
use zeroize::Zeroizing;

const HOST: &str = "fapi.binance.com";
const BASE: &str = "https://fapi.binance.com";
pub const STALE_AFTER: Duration = Duration::from_secs(30);
const MAX_SERVER_MS: u64 = 4_102_444_800_000;
const NANOS_PER_MS: u128 = 1_000_000;
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Network,
    Payload,
    // Both strings are compile-time labels, never Binance values/errors/URLs.
    Schema(&'static str, &'static str),
    Auth,
    Clock,
    RateLimit,
    Restricted,
    Unavailable,
    Vault(VaultError),
}

/// Evidence from one successful `/fapi/v1/time` request. Binance's timestamp
/// was produced no earlier than request start and no later than completion of
/// the bounded response body, so the server clock at `completed` is in
/// `[server_ms, server_ms + request_elapsed]`. No symmetric-route assumption is
/// needed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClockSample {
    server_ms: u64,
    started: Instant,
    completed: Instant,
}
impl ClockSample {
    pub(crate) fn new(server_ms: u64, started: Instant, completed: Instant) -> Result<Self> {
        if server_ms == 0
            || server_ms > MAX_SERVER_MS
            || completed.checked_duration_since(started).is_none()
        {
            return Err(Error::Clock);
        }
        let sample = Self {
            server_ms,
            started,
            completed,
        };
        sample.upper_at(completed)?;
        Ok(sample)
    }
    pub(crate) fn completed(self) -> Instant {
        self.completed
    }
    pub(crate) fn uncertainty(self) -> Duration {
        self.completed.duration_since(self.started)
    }
    pub(crate) fn bounds_at_nanos(self, at: Instant) -> Result<(u128, u128)> {
        let elapsed = at
            .checked_duration_since(self.completed)
            .ok_or(Error::Clock)?;
        let lower = u128::from(self.server_ms)
            .checked_mul(NANOS_PER_MS)
            .and_then(|value| value.checked_add(elapsed.as_nanos()))
            .ok_or(Error::Clock)?;
        let upper = lower
            .checked_add(self.uncertainty().as_nanos())
            .ok_or(Error::Clock)?;
        if upper > u128::from(MAX_SERVER_MS) * NANOS_PER_MS {
            return Err(Error::Clock);
        }
        Ok((lower, upper))
    }
    pub(crate) fn lower_at(self, at: Instant) -> Result<u64> {
        let (lower, _) = self.bounds_at_nanos(at)?;
        (lower / NANOS_PER_MS).try_into().map_err(|_| Error::Clock)
    }
    pub(crate) fn upper_at(self, at: Instant) -> Result<u64> {
        let (_, upper) = self.bounds_at_nanos(at)?;
        upper
            .checked_add(NANOS_PER_MS - 1)
            .map(|value| value / NANOS_PER_MS)
            .and_then(|value| value.try_into().ok())
            .ok_or(Error::Clock)
    }
    /// The lower bound cannot be in the server's future. It remains acceptable
    /// for a signed request only while the measured uncertainty fits Binance's
    /// explicit 5 second receive window.
    pub(crate) fn signing_at(self, at: Instant) -> Result<u64> {
        if self.uncertainty() > Duration::from_secs(5) {
            return Err(Error::Clock);
        }
        self.lower_at(at)
    }
}
impl Error {
    pub fn message(self) -> String {
        match self {
            Self::Network => "Koneksi akun terputus; mencoba lagi otomatis.",
            Self::Payload => "Data akun Binance belum lengkap atau formatnya berubah.",
            Self::Schema(endpoint, check) => return format!(
                "Data Binance belum bisa dibaca. Diagnosis: {endpoint} · {check}."
            ),
            Self::Auth => {
                "Autentikasi ditolak. Periksa key/secret, izin baca Futures dan batas IP di Binance."
            }
            Self::Clock => "Waktu permintaan ditolak; menyinkronkan ulang dengan Binance.",
            Self::RateLimit => "Batas permintaan Binance tercapai; menunggu waktu retry server.",
            Self::Restricted => "Layanan akun ditolak oleh Binance pada koneksi ini.",
            Self::Unavailable => "Layanan akun Binance sedang tidak tersedia.",
            Self::Vault(error) => error.message(),
        }
        .into()
    }
    fn at(self, endpoint: &'static str) -> Self {
        if self == Self::Payload {
            Self::Schema(endpoint, "JSON_RESPONSE")
        } else {
            self
        }
    }
    fn terminal(self) -> bool {
        matches!(self, Self::Auth | Self::Restricted | Self::Vault(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Endpoint {
    Account,
    Positions,
    Config,
    Symbols,
}
impl Endpoint {
    const ALL: [Self; 4] = [Self::Account, Self::Positions, Self::Config, Self::Symbols];
    fn path(self) -> &'static str {
        match self {
            Self::Account => "/fapi/v3/account",
            Self::Positions => "/fapi/v3/positionRisk",
            Self::Config => "/fapi/v1/accountConfig",
            Self::Symbols => "/fapi/v1/symbolConfig",
        }
    }
}

fn signature(secret: &[u8], query: &[u8]) -> Result<Zeroizing<String>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| Error::Auth)?;
    mac.update(query);
    let mut hex = Zeroizing::new(String::with_capacity(64));
    for b in mac.finalize().into_bytes() {
        write!(hex, "{b:02x}").map_err(|_| Error::Auth)?;
    }
    Ok(hex)
}
fn signed_request(
    client: &Client,
    credentials: &Credentials,
    endpoint: Endpoint,
    timestamp: u64,
) -> Result<Request> {
    let query = format!("timestamp={timestamp}&recvWindow=5000");
    let signature = signature(credentials.secret.as_bytes(), query.as_bytes())?;
    let mut key = HeaderValue::from_str(&credentials.key).map_err(|_| Error::Auth)?;
    key.set_sensitive(true);
    let request = client
        .get(format!(
            "{BASE}{}?{query}&signature={}",
            endpoint.path(),
            *signature
        ))
        .header(HeaderName::from_static("x-mbx-apikey"), key)
        .build()
        .map_err(|_| Error::Auth)?;
    guard(&request)?;
    Ok(request)
}
fn guard(request: &Request) -> Result<()> {
    let url = request.url();
    if request.method() != Method::GET
        || url.scheme() != "https"
        || url.host_str() != Some(HOST)
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !Endpoint::ALL.iter().any(|e| e.path() == url.path())
        || request.headers().contains_key("authorization")
        || request.headers().contains_key("cookie")
    {
        return Err(Error::Restricted);
    }
    let keys: BTreeSet<_> = url.query_pairs().map(|(k, _)| k.into_owned()).collect();
    if keys
        != ["timestamp", "recvWindow", "signature"]
            .into_iter()
            .map(str::to_owned)
            .collect()
        || url.query_pairs().count() != 3
    {
        return Err(Error::Restricted);
    }
    Ok(())
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balances {
    pub total_wallet_balance: D,
    pub total_margin_balance: D,
    pub available_balance: D,
    pub total_initial_margin: D,
    pub total_maint_margin: D,
    pub total_unrealized_profit: D,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub asset: String,
    pub wallet_balance: D,
    pub available_balance: D,
    pub margin_balance: D,
    pub unrealized_profit: D,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub can_trade: bool,
    pub dual_side_position: bool,
    pub multi_assets_margin: bool,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolConfig {
    pub symbol: String,
    pub margin_type: String,
    pub leverage: u32,
    pub is_auto_add_margin: bool,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub symbol: String,
    pub position_side: String,
    pub position_amt: D,
    pub entry_price: D,
    pub mark_price: D,
    #[serde(rename = "unRealizedProfit")]
    pub unrealized_profit: D,
    pub liquidation_price: D,
    pub margin_asset: String,
    pub initial_margin: D,
    pub maint_margin: D,
}
#[derive(Clone)]
pub struct Data {
    pub balances: Balances,
    pub config: Config,
    pub symbols: Vec<SymbolConfig>,
    pub positions: Vec<Position>,
    pub server_ms: u64,
}
impl Data {
    pub fn unit(&self) -> &'static str {
        if self.config.multi_assets_margin {
            "USD setara"
        } else {
            "USDT"
        }
    }
    pub fn symbol(&self, symbol: &str) -> Option<&SymbolConfig> {
        self.symbols.iter().find(|s| s.symbol == symbol)
    }
    pub(crate) fn parse(
        account: Value,
        positions: Value,
        config: Value,
        symbols: Value,
        server_ms: u64,
    ) -> Result<Self> {
        let account_error = |field| Error::Schema(Endpoint::Account.path(), field);
        let position_error = |field| Error::Schema(Endpoint::Positions.path(), field);
        let symbol_error = |field| Error::Schema(Endpoint::Symbols.path(), field);
        let balances: Balances =
            serde_json::from_value(account).map_err(|_| account_error("FIELD_TYPES"))?;
        let config: Config = serde_json::from_value(config)
            .map_err(|_| Error::Schema(Endpoint::Config.path(), "FIELD_TYPES"))?;
        let mut positions: Vec<Position> =
            serde_json::from_value(positions).map_err(|_| position_error("FIELD_TYPES"))?;
        let symbols: Vec<SymbolConfig> =
            serde_json::from_value(symbols).map_err(|_| symbol_error("FIELD_TYPES"))?;
        let mut unique_assets = BTreeSet::new();
        for asset in &balances.assets {
            if !name_ok(&asset.asset) {
                return Err(account_error("asset"));
            }
            if !unique_assets.insert(asset.asset.clone()) {
                return Err(account_error("DUPLICATE_ASSET"));
            }
        }
        let mut unique_symbols = BTreeSet::new();
        for symbol in &symbols {
            if !name_ok(&symbol.symbol) {
                return Err(symbol_error("symbol"));
            }
            if !unique_symbols.insert(symbol.symbol.clone()) {
                return Err(symbol_error("DUPLICATE_SYMBOL"));
            }
            if !matches!(symbol.margin_type.as_str(), "CROSSED" | "ISOLATED") {
                return Err(symbol_error("marginType"));
            }
            if !(1..=1000).contains(&symbol.leverage) {
                return Err(symbol_error("leverage"));
            }
        }
        let mut unique_positions = BTreeSet::new();
        for position in &positions {
            let mode_ok = if config.dual_side_position {
                (position.position_side == "LONG" && position.position_amt >= D::ZERO)
                    || (position.position_side == "SHORT" && position.position_amt <= D::ZERO)
            } else {
                position.position_side == "BOTH"
            };
            if !name_ok(&position.symbol) || !name_ok(&position.margin_asset) {
                return Err(position_error("symbol_or_marginAsset"));
            }
            if !mode_ok {
                return Err(position_error("POSITION_MODE"));
            }
            if !unique_positions.insert((position.symbol.clone(), position.position_side.clone())) {
                return Err(position_error("DUPLICATE_POSITION"));
            }
            if position.entry_price < D::ZERO
                || position.mark_price < D::ZERO
                || position.liquidation_price < D::ZERO
                || position.initial_margin < D::ZERO
                || position.maint_margin < D::ZERO
            {
                return Err(position_error("PRICE_OR_MARGIN"));
            }
        }
        positions.retain(|p| p.position_amt != D::ZERO);
        positions.sort_by(|a, b| (&a.symbol, &a.position_side).cmp(&(&b.symbol, &b.position_side)));
        Ok(Self {
            balances,
            config,
            symbols,
            positions,
            server_ms,
        })
    }
}

fn name_ok(name: &str) -> bool {
    // Binance symbols can contain Unicode letters (e.g. 币安人生USDT).
    // Preserve the exchange identifier exactly; do not normalize/transliterate.
    !name.is_empty()
        && name.chars().count() <= 48
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Unconfigured,
    Loading,
    Connecting,
    Connected,
    Failed,
    Disconnected,
}
#[derive(Clone)]
pub struct Snapshot {
    pub generation: u64,
    pub status: Status,
    pub error: Option<Error>,
    pub note: Option<&'static str>,
    pub data: Option<Data>,
    pub received: Option<Instant>,
    pub updates: u64,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            status: Status::Unconfigured,
            error: None,
            note: None,
            data: None,
            received: None,
            updates: 0,
        }
    }
}
impl Snapshot {
    fn loaded(
        &mut self,
        result: std::result::Result<Credentials, VaultError>,
    ) -> Option<Credentials> {
        match result {
            Ok(value) => {
                self.status = Status::Connecting;
                Some(value)
            }
            Err(VaultError::Missing) => {
                self.status = Status::Unconfigured;
                self.note = Some(VaultError::Missing.message());
                None
            }
            Err(error) => {
                self.failed(Error::Vault(error));
                None
            }
        }
    }
    pub fn fresh(&self) -> bool {
        self.status == Status::Connected
            && self.error.is_none()
            && self
                .received
                .is_some_and(|time| time.elapsed() <= STALE_AFTER)
    }
    pub fn label(&self) -> &'static str {
        match self.status {
            Status::Unconfigured => "BELUM DIKONFIGURASI",
            Status::Loading => "MEMBUKA KWALLET",
            Status::Connecting => "MENYAMBUNG AKUN",
            Status::Connected if !self.fresh() => "DATA AKUN TERLAMBAT",
            Status::Connected => "TERHUBUNG · BACA SAJA",
            Status::Failed => "AKUN BELUM TERHUBUNG",
            Status::Disconnected => "AKUN DIPUTUS",
        }
    }
    fn failed(&mut self, error: Error) {
        self.status = Status::Failed;
        self.error = Some(error);
        if error.terminal() {
            self.data = None;
            self.received = None;
        }
    }
    fn connected(&mut self, data: Data) {
        self.status = Status::Connected;
        self.error = None;
        self.data = Some(data);
        self.received = Some(Instant::now());
        self.updates += 1;
    }
}

pub enum Command {
    Load,
    Connect(Credentials, bool),
    Disconnect,
    Forget,
}
pub struct Feed {
    commands: mpsc::Sender<Command>,
    output: watch::Receiver<Snapshot>,
    expected: std::cell::Cell<u64>,
}
impl Feed {
    pub fn start() -> anyhow::Result<Self> {
        let (commands, input) = mpsc::channel(4);
        let (output, receiver) = watch::channel(Snapshot::default());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("binance-account-reader".into())
            .spawn(move || {
                runtime.block_on(async {
                    match Transport::new() {
                        Ok(mut source) => worker(input, output, &mut source).await,
                        Err(error) => {
                            output.send_modify(|s| s.failed(error));
                        }
                    }
                });
            })?;
        Ok(Self {
            commands,
            output: receiver,
            expected: std::cell::Cell::new(0),
        })
    }
    pub fn send(&self, command: Command) -> bool {
        if self.commands.try_send(command).is_err() {
            return false;
        }
        self.expected.set(self.expected.get() + 1);
        true
    }
    pub fn latest(&self) -> Snapshot {
        let snapshot = self.output.borrow().clone();
        if snapshot.generation == self.expected.get() {
            snapshot
        } else {
            Snapshot {
                generation: self.expected.get(),
                status: Status::Connecting,
                ..Snapshot::default()
            }
        }
    }
}

trait Source {
    async fn fetch(&mut self, credentials: &Credentials) -> Result<Data>;
    fn cooldown(&self) -> Duration {
        Duration::ZERO
    }
}

// Connection diagnostics contain only application status and static schema
// labels. Never include Snapshot::data, credentials, response bodies or URLs.
fn connection_notice(previous: &Snapshot, current: &Snapshot) -> Option<String> {
    if current.status == Status::Connected && current.updates >= 2 && previous.updates < 2 {
        return Some(
            "ACCOUNT_READ_ONLY_CONNECTED; snapshots=2; account_values=REDACTED; mutations=0".into(),
        );
    }
    if previous.status != current.status || previous.error != current.error {
        return Some(format!(
            "ACCOUNT_STATUS: {}{}",
            current.label(),
            current
                .error
                .map_or_else(String::new, |error| format!("; {}", error.message()))
        ));
    }
    None
}
fn publish(output: &watch::Sender<Snapshot>, current: &Snapshot) {
    let previous = output.send_replace(current.clone());
    if let Some(notice) = connection_notice(&previous, current) {
        eprintln!("{notice}");
    }
}
async fn worker<S: Source>(
    mut input: mpsc::Receiver<Command>,
    output: watch::Sender<Snapshot>,
    source: &mut S,
) {
    let mut credentials = None;
    let mut state = Snapshot::default();
    let mut next = Instant::now();
    let mut failures = 0_u32;
    loop {
        let delay = next
            .saturating_duration_since(Instant::now())
            .max(source.cooldown());
        let command = if let Some(ref current) = credentials {
            tokio::select! {
                biased;
                command = input.recv() => command,
                result = async { tokio::time::sleep(delay).await; source.fetch(current).await } => {
                    match result {
                        Ok(data) => { state.connected(data); failures = 0; next = Instant::now() + Duration::from_secs(10); }
                        Err(error) => {
                            state.failed(error); failures = failures.saturating_add(1);
                            if error.terminal() { credentials = None; }
                            next = Instant::now() + Duration::from_secs(2_u64.saturating_pow(failures.min(6)).min(60));
                        }
                    }
                    publish(&output, &state);
                    continue;
                }
            }
        } else {
            input.recv().await
        };
        let Some(command) = command else {
            return;
        };
        // Credential changes and disconnect clear private data immediately; old
        // in-flight HTTP futures are cancelled by the select above.
        credentials = None;
        state = Snapshot {
            generation: state.generation + 1,
            ..Snapshot::default()
        };
        match command {
            Command::Load => {
                state.status = Status::Loading;
                publish(&output, &state);
                credentials = state.loaded(
                    tokio::task::spawn_blocking(vault::load)
                        .await
                        .unwrap_or(Err(VaultError::Unavailable)),
                );
            }
            Command::Connect(value, persist) => {
                if persist {
                    state.status = Status::Loading;
                    publish(&output, &state);
                    match tokio::task::spawn_blocking(move || {
                        let result = vault::save(&value);
                        (value, result)
                    })
                    .await
                    {
                        Ok((value, Ok(()))) => {
                            credentials = Some(value);
                            state.note = Some("Kredensial tersimpan di KWallet.");
                        }
                        Ok((_, Err(error))) => state.failed(Error::Vault(error)),
                        Err(_) => state.failed(Error::Vault(VaultError::Unavailable)),
                    }
                } else {
                    credentials = Some(value);
                    state.note = Some("Kredensial hanya dipakai selama sesi ini.");
                }
                if credentials.is_some() {
                    state.status = Status::Connecting;
                }
            }
            Command::Disconnect => {
                state.status = Status::Disconnected;
            }
            Command::Forget => {
                state.status = Status::Loading;
                publish(&output, &state);
                match tokio::task::spawn_blocking(vault::delete).await {
                    Ok(Ok(())) => {
                        state.status = Status::Disconnected;
                        state.note = Some("Kredensial Binance Desk dihapus dari KWallet.");
                    }
                    Ok(Err(error)) => state.failed(Error::Vault(error)),
                    Err(_) => state.failed(Error::Vault(VaultError::Unavailable)),
                }
            }
        }
        next = Instant::now();
        failures = 0;
        publish(&output, &state);
    }
}

pub(crate) struct Transport {
    normal: Client,
    selected: Option<(Client, Instant)>,
    limited_until: Option<Instant>,
}
impl Transport {
    pub(crate) fn cooldown(&self) -> Duration {
        <Self as Source>::cooldown(self)
    }
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            normal: market::client().build().map_err(|_| Error::Network)?,
            selected: None,
            limited_until: None,
        })
    }
    async fn response(&mut self, response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        if matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS | StatusCode::IM_A_TEAPOT
        ) {
            let default = if status == StatusCode::IM_A_TEAPOT {
                120
            } else {
                60
            };
            let wait = market::retry_after_seconds(
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()),
                default,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            self.limited_until = Instant::now()
                .checked_add(Duration::from_secs(wait))
                .or_else(|| Instant::now().checked_add(Duration::from_secs(365 * 86400)));
            return Err(Error::RateLimit);
        }
        if status == StatusCode::FORBIDDEN || status == StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS {
            return Err(Error::Restricted);
        }
        if status.is_server_error() {
            return Err(Error::Unavailable);
        }
        let value = market::bounded_json(response)
            .await
            .map_err(|_| Error::Payload)?;
        let code = value.get("code").and_then(Value::as_i64);
        if !status.is_success() || code.is_some_and(|code| code < 0) {
            if matches!(code, Some(-1003 | -1015)) {
                self.limited_until = Some(Instant::now() + Duration::from_secs(60));
            }
            return Err(error_from_code(code, status));
        }
        Ok(value)
    }
    pub(crate) async fn prepare(&mut self) -> Result<(Client, ClockSample)> {
        prepare_with_budget(self.prepare_inner()).await
    }
    async fn prepare_inner(&mut self) -> Result<(Client, ClockSample)> {
        if !self.cooldown().is_zero() {
            return Err(Error::RateLimit);
        }
        let active = self
            .selected
            .as_ref()
            .filter(|(_, expiry)| *expiry > Instant::now())
            .map_or_else(|| self.normal.clone(), |(client, _)| client.clone());
        let started = Instant::now();
        let (client, response, started) = match active
            .get(format!("{BASE}/fapi/v1/time"))
            .send()
            .await
        {
            Ok(response) => (active, response, started),
            Err(error) => {
                if market::public_read_error("account_time_primary", &error)
                    != market::ReadFailure::Network
                {
                    return Err(Error::Payload);
                }
                // Resolver is always a separate client without any credential/header.
                let response = self
                    .normal
                    .get("https://dns.google/resolve")
                    .query(&[("name", HOST), ("type", "A")])
                    .send()
                    .await
                    .map_err(|e| {
                        if market::public_read_error("account_dns_resolver", &e)
                            == market::ReadFailure::Network
                        {
                            Error::Network
                        } else {
                            Error::Payload
                        }
                    })?;
                if !response.status().is_success() {
                    return Err(
                        if response.status().is_server_error()
                            || response.status() == StatusCode::REQUEST_TIMEOUT
                        {
                            Error::Network
                        } else {
                            Error::Restricted
                        },
                    );
                }
                let dns = market::bounded_json(response).await.map_err(|e| {
                    if matches!(
                        e.downcast_ref::<market::ReadFailure>(),
                        Some(market::ReadFailure::Network)
                    ) {
                        Error::Network
                    } else {
                        Error::Payload
                    }
                })?;
                let (addresses, ttl) = market::parse_dns(dns).map_err(|_| Error::Payload)?;
                let expires = Instant::now() + ttl;
                let mut connected = None;
                for (attempt, address) in addresses.into_iter().take(4).enumerate() {
                    let client = market::client()
                        .resolve(HOST, address)
                        .build()
                        .map_err(|_| Error::Payload)?;
                    let started = Instant::now();
                    match client.get(format!("{BASE}/fapi/v1/time")).send().await {
                        Ok(response) => {
                            self.selected = Some((client.clone(), expires));
                            connected = Some((client, response, started));
                            break;
                        }
                        Err(error) => {
                            eprintln!(
                                "ACCOUNT_ROUTE_HEALTH; stage=time_fallback; attempt={}; elapsed_ms={}; timeout={}; connect={}",
                                attempt + 1,
                                started.elapsed().as_millis(),
                                error.is_timeout(),
                                error.is_connect()
                            );
                            if !error.is_timeout() && !error.is_connect() {
                                return Err(Error::Payload);
                            }
                        }
                    }
                }
                connected.ok_or(Error::Network)?
            }
        };
        let value = self
            .response(response)
            .await
            .map_err(|error| error.at("/fapi/v1/time"))?;
        let time = value["serverTime"]
            .as_u64()
            .filter(|t| *t > 0)
            .ok_or(Error::Schema("/fapi/v1/time", "serverTime"))?;
        let completed = Instant::now();
        Ok((client, ClockSample::new(time, started, completed)?))
    }
}
async fn prepare_with_budget<T>(future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    match tokio::time::timeout(Duration::from_secs(20), future).await {
        Ok(result) => result,
        Err(_) => {
            eprintln!("ACCOUNT_ROUTE_HEALTH; stage=total_budget; outcome=timeout; limit_ms=20000");
            Err(Error::Network)
        }
    }
}
fn error_from_code(code: Option<i64>, status: StatusCode) -> Error {
    match code {
        Some(-1021) => Error::Clock,
        Some(-1022 | -2014 | -2015) => Error::Auth,
        Some(-1003 | -1015) => Error::RateLimit,
        _ if status == StatusCode::UNAUTHORIZED => Error::Auth,
        _ => Error::Unavailable,
    }
}
impl Source for Transport {
    async fn fetch(&mut self, credentials: &Credentials) -> Result<Data> {
        let (client, clock) = self.prepare().await?;
        let mut responses = Vec::new();
        for endpoint in Endpoint::ALL {
            let timestamp = clock.signing_at(Instant::now())?;
            let request = signed_request(&client, credentials, endpoint, timestamp)?;
            let response = client.execute(request).await.map_err(|_| Error::Network)?;
            responses.push(
                self.response(response)
                    .await
                    .map_err(|error| error.at(endpoint.path()))?,
            );
        }
        let mut values = responses.into_iter();
        Data::parse(
            values.next().ok_or(Error::Payload)?,
            values.next().ok_or(Error::Payload)?,
            values.next().ok_or(Error::Payload)?,
            values.next().ok_or(Error::Payload)?,
            clock.lower_at(Instant::now())?,
        )
    }
    fn cooldown(&self) -> Duration {
        self.limited_until.map_or(Duration::ZERO, |t| {
            t.saturating_duration_since(Instant::now())
        })
    }
}

/// Safe live verification: reports connectivity only, never private account data.
pub fn probe() -> anyhow::Result<()> {
    let feed = Feed::start()?;
    anyhow::ensure!(feed.send(Command::Load), "Worker akun tidak tersedia");
    let start = Instant::now();
    loop {
        let state = feed.latest();
        if state.fresh() && state.updates >= 2 {
            println!(
                "ACCOUNT_READ_ONLY_CONNECTED; snapshots=2; account_values=REDACTED; mutations=0"
            );
            return Ok(());
        }
        if state.error.is_some_and(Error::terminal) {
            anyhow::bail!(
                "{}",
                state
                    .error
                    .map_or_else(|| "Akun gagal".into(), Error::message)
            );
        }
        if start.elapsed() > Duration::from_secs(2) && state.status == Status::Unconfigured {
            anyhow::bail!("ACCOUNT_CREDENTIAL_MISSING: masukkan key/secret langsung di aplikasi");
        }
        anyhow::ensure!(
            start.elapsed() < Duration::from_secs(55),
            "ACCOUNT_PROBE_TIMEOUT: {}",
            state
                .error
                .map_or_else(|| state.label().into(), Error::message)
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r10_prepare_total_budget_cancels_pending_work_and_preserves_early_error() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct Pending(Arc<AtomicBool>);
        impl Drop for Pending {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let watch = dropped.clone();
        let start = tokio::time::Instant::now();
        let result = prepare_with_budget(async move {
            let _pending = Pending(watch);
            std::future::pending::<Result<()>>().await
        })
        .await;
        assert_eq!(result, Err(Error::Network));
        assert_eq!(start.elapsed(), Duration::from_secs(20));
        assert!(dropped.load(Ordering::Relaxed));
        assert_eq!(
            prepare_with_budget(async { Err::<(), _>(Error::Auth) }).await,
            Err(Error::Auth)
        );
    }
    use serde_json::json;
    fn payloads() -> [Value; 4] {
        [
            json!({"totalWalletBalance":"100.12345678","totalMarginBalance":"101.12345678", "availableBalance":"80.12345678",
            "totalInitialMargin":"20", "totalMaintMargin":"0.5", "totalUnrealizedProfit":"1", "assets":[{
                "asset":"USDT", "walletBalance":"100.12345678", "availableBalance":"80.12345678", "marginBalance":"101.12345678", "unrealizedProfit":"1"}]}),
            json!([{"symbol":"BTCUSDT","positionSide":"BOTH","positionAmt":"0.001", "entryPrice":"70000", "markPrice":"71000",
                "unRealizedProfit":"1","liquidationPrice":"0", "marginAsset":"USDT", "initialMargin":"14.2", "maintMargin":"0.284"}]),
            json!({"canTrade":true,"dualSidePosition":false,"multiAssetsMargin":false}),
            json!([{"symbol":"BTCUSDT","marginType":"ISOLATED","leverage":5,"isAutoAddMargin":false}]),
        ]
    }
    pub fn example() -> Data {
        let [a, p, c, s] = payloads();
        Data::parse(a, p, c, s, 1_789_550_000_000).unwrap()
    }
    #[test]
    fn accepts_all_official_public_symbol_names_including_unicode() {
        // Public exchangeInfo names, fetched 2026-09-16; balances/config below
        // are synthetic. No private account response is stored in fixtures.
        let names: Vec<String> = serde_json::from_str(include_str!(
            "../fixtures/public-futures-symbols-20260916.json"
        ))
        .unwrap();
        assert_eq!(names.len(), 897);
        assert!(names.iter().any(|s| !s.is_ascii()));
        let [a, p, c, _] = payloads();
        let symbols = Value::Array(names.iter().map(|symbol| json!({
            "symbol": symbol, "marginType":"CROSSED", "leverage":5, "isAutoAddMargin":false
        })).collect());
        assert!(Data::parse(a.clone(), p.clone(), c.clone(), symbols.clone(), 0).is_ok());
        for name in names.iter().filter(|s| !s.is_ascii()) {
            let mut position = p.clone();
            position[0]["symbol"] = json!(name);
            assert!(Data::parse(a.clone(), position, c.clone(), symbols.clone(), 0).is_ok());
        }
    }
    #[test]
    fn connection_diagnostics_report_two_snapshots_without_private_data() {
        let mut state = Snapshot::default();
        let mut data = example();
        data.positions[0].symbol = "private_value_sentinel".into();
        state.connected(data.clone());
        let notice = connection_notice(&Snapshot::default(), &state).unwrap();
        assert!(notice.contains("TERHUBUNG"));
        assert!(!notice.contains("private_value"));
        assert!(!notice.contains("100.12345678"));
        let previous = state.clone();
        state.connected(data.clone());
        assert_eq!(
            connection_notice(&previous, &state).unwrap(),
            "ACCOUNT_READ_ONLY_CONNECTED; snapshots=2; account_values=REDACTED; mutations=0"
        );
        let previous = state.clone();
        state.connected(data);
        assert!(connection_notice(&previous, &state).is_none());
        state.failed(Error::Schema(Endpoint::Symbols.path(), "symbol"));
        let notice = connection_notice(&previous, &state).unwrap();
        assert!(notice.contains("/fapi/v1/symbolConfig"));
        assert!(!notice.contains("private_value"));
        assert!(!notice.contains("100.12345678"));
    }
    #[test]
    fn unicode_names_keep_strict_control_character_and_duplicate_guards() {
        for name in [
            "",
            "BTC USDT",
            "BTC/USDT",
            "BTC\nUSDT",
            "BTC\0USDT",
            "\u{202e}BTCUSDT",
            "BTC\u{200d}USDT",
        ] {
            let [a, p, c, mut s] = payloads();
            s[0]["symbol"] = json!(name);
            assert_eq!(
                Data::parse(a, p, c, s, 0).err(),
                Some(Error::Schema(Endpoint::Symbols.path(), "symbol"))
            );
        }
        assert!(!name_ok(&"币".repeat(49)));
        let [a, p, c, mut s] = payloads();
        s[0]["symbol"] = json!("币安人生USDT");
        let duplicate = s[0].clone();
        s.as_array_mut().unwrap().push(duplicate);
        assert_eq!(
            Data::parse(a, p, c, s, 0).err(),
            Some(Error::Schema(Endpoint::Symbols.path(), "DUPLICATE_SYMBOL"))
        );
    }
    #[test]
    fn schema_diagnosis_identifies_endpoint_without_echoing_private_values() {
        for index in 0..4 {
            let mut values = payloads();
            values[index] = json!({"must_not_echo_private_value":"private_value_sentinel"});
            let [a, p, c, s] = values;
            let error = Data::parse(a, p, c, s, 0).err().unwrap();
            let message = error.message();
            assert!(message.contains(Endpoint::ALL[index].path()));
            assert!(message.contains("FIELD_TYPES"));
            assert!(!message.contains("private_value"));
            assert!(!format!("{error:?}").contains("private_value"));
        }
        let [a, p, c, mut s] = payloads();
        s[0]["marginType"] = json!("private_value_sentinel");
        let error = Data::parse(a, p, c, s, 0).err().unwrap();
        assert_eq!(error, Error::Schema(Endpoint::Symbols.path(), "marginType"));
        assert!(!error.message().contains("private_value"));
        assert_eq!(
            Error::Payload.at(Endpoint::Positions.path()),
            Error::Schema(Endpoint::Positions.path(), "JSON_RESPONSE")
        );
        assert_eq!(Error::Auth.at(Endpoint::Positions.path()), Error::Auth);
    }
    #[test]
    fn hedge_positions_are_not_netted_and_inconsistent_modes_are_rejected() {
        let [a, mut p, mut c, s] = payloads();
        p[0]["positionSide"] = json!("LONG");
        let mut short = p[0].clone();
        short["positionSide"] = json!("SHORT");
        short["positionAmt"] = json!("-0.001");
        p.as_array_mut().unwrap().push(short);
        c["dualSidePosition"] = json!(true);
        c["multiAssetsMargin"] = json!(true);
        let data = Data::parse(a.clone(), p.clone(), c.clone(), s.clone(), 0).unwrap();
        assert_eq!(data.positions.len(), 2);
        assert_eq!(data.unit(), "USD setara");
        c["dualSidePosition"] = json!(false);
        assert!(Data::parse(a, p, c, s, 0).is_err());
    }
    #[test]
    fn duplicate_positions_and_missing_or_invalid_financial_fields_are_rejected() {
        let [a, mut p, c, s] = payloads();
        let duplicate = p[0].clone();
        p.as_array_mut().unwrap().push(duplicate);
        assert!(Data::parse(a, p, c, s, 0).is_err());
        for bad in [Value::Null, json!("NaN"), json!("1e999"), json!({})] {
            let [mut a, p, c, s] = payloads();
            a["availableBalance"] = bad;
            assert!(Data::parse(a, p, c, s, 0).is_err());
        }
        let [a, _, c, s] = payloads();
        assert!(
            Data::parse(a, json!([]), c, s, 0)
                .unwrap()
                .positions
                .is_empty()
        );
    }
    #[test]
    fn hmac_matches_rfc4231_public_test_vector() {
        assert_eq!(
            &*signature(&[0x0b; 20], b"Hi There").unwrap(),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }
    #[test]
    fn signing_is_get_only_fixed_host_and_sensitive_header() {
        let client = market::client().build().unwrap();
        let credentials =
            Credentials::new("dummy_public_key_123".into(), "dummy_secret_123456".into()).unwrap();
        for endpoint in Endpoint::ALL {
            let r = signed_request(&client, &credentials, endpoint, 123456789).unwrap();
            assert!(r.headers()["x-mbx-apikey"].is_sensitive());
            assert!(guard(&r).is_ok());
            assert_eq!(
                r.url()
                    .query_pairs()
                    .find(|(k, _)| k == "recvWindow")
                    .unwrap()
                    .1,
                "5000"
            );
            assert!(!r.url().as_str().contains(&*credentials.key));
            assert!(!r.url().as_str().contains(&*credentials.secret));
        }
        for (method, url) in [
            (Method::POST, "https://fapi.binance.com/fapi/v3/account"),
            (Method::GET, "https://example.com/fapi/v3/account"),
            (Method::DELETE, "https://fapi.binance.com/fapi/v1/order"),
            (Method::GET, "https://fapi.binance.com/fapi/v1/order"),
            (Method::GET, "http://fapi.binance.com/fapi/v3/account"),
        ] {
            let r = Request::new(method, url.parse().unwrap());
            assert_eq!(guard(&r), Err(Error::Restricted));
        }
    }
    #[test]
    fn decimal_balances_and_zero_liquidation_are_preserved() {
        let mut data = example();
        assert_eq!(
            data.balances.total_wallet_balance.to_string(),
            "100.12345678"
        );
        assert_eq!(data.positions[0].liquidation_price, D::ZERO);
        assert_eq!(data.symbol("BTCUSDT").unwrap().leverage, 5);
        assert!(data.symbol("ETHUSDT").is_none());
        assert_eq!(data.unit(), "USDT");
        data.config.multi_assets_margin = true;
        assert_eq!(data.unit(), "USD setara");
    }
    #[test]
    fn missing_vault_finishes_loading_and_allows_manual_credentials() {
        let mut state = Snapshot {
            status: Status::Loading,
            ..Snapshot::default()
        };
        assert!(state.loaded(Err(VaultError::Missing)).is_none());
        assert_eq!(state.status, Status::Unconfigured);
        assert!(state.error.is_none());
        assert!(state.data.is_none());
        assert!(state.note.is_some());
        assert!(state.loaded(Err(VaultError::Unavailable)).is_none());
        assert_eq!(state.status, Status::Failed);
    }
    #[test]
    fn missing_account_fields_never_become_zero() {
        assert!(Data::parse(json!({}), json!([]), json!({}), json!([]), 0).is_err());
        let invalid: std::result::Result<Balances, _> =
            serde_json::from_value(json!({"totalWalletBalance":"NaN"}));
        assert!(invalid.is_err());
    }
    #[test]
    fn stale_and_error_snapshots_cannot_be_presented_as_connected() {
        let mut state = Snapshot::default();
        state.connected(example());
        assert!(state.fresh());
        state.received = Some(Instant::now() - Duration::from_secs(31));
        assert!(!state.fresh());
        state.failed(Error::Network);
        assert!(state.data.is_some());
        assert!(!state.fresh());
        state.connected(example());
        state.failed(Error::Auth);
        assert!(state.data.is_none());
        assert_eq!(
            error_from_code(Some(-1021), StatusCode::BAD_REQUEST),
            Error::Clock
        );
        assert_eq!(
            error_from_code(Some(-2015), StatusCode::UNAUTHORIZED),
            Error::Auth
        );
    }
    struct Fake {
        count: usize,
    }
    #[tokio::test]
    async fn server_rate_limits_and_raw_messages_are_sanitized() {
        let mut transport = Transport::new().unwrap();
        let response: reqwest::Response = http::Response::builder()
            .status(429)
            .header("Retry-After", "90")
            .body("must-not-log-signed-url-or-key")
            .unwrap()
            .into();
        assert_eq!(
            transport.response(response).await.err(),
            Some(Error::RateLimit)
        );
        assert!(transport.cooldown() > Duration::from_secs(89));
        let response: reqwest::Response = http::Response::builder()
            .status(401)
            .body(r#"{"code":-2015,"msg":"must-not-log-signed-url-or-key"}"#)
            .unwrap()
            .into();
        let error = transport.response(response).await.err().unwrap();
        assert_eq!(error, Error::Auth);
        assert!(!error.message().contains("must-not-log"));
    }
    #[test]
    fn r12_clock_sample_keeps_full_request_and_body_uncertainty_without_half_rtt_guess() {
        let completed = Instant::now();
        let started = completed - Duration::from_millis(120);
        let sample = ClockSample::new(1_000_000, started, completed).unwrap();
        assert_eq!(sample.lower_at(completed).unwrap(), 1_000_000);
        assert_eq!(sample.upper_at(completed).unwrap(), 1_000_120);
        // Signing deliberately uses the lower bound, which cannot be in the
        // server's future even for a fully asymmetric response path.
        assert_eq!(sample.signing_at(completed).unwrap(), 1_000_000);
        let later = completed + Duration::from_millis(300);
        assert_eq!(sample.lower_at(later).unwrap(), 1_000_300);
        assert_eq!(sample.upper_at(later).unwrap(), 1_000_420);

        // A delayed body is part of the measured uncertainty. Once it exceeds
        // the protocol's recvWindow, it cannot prepare a signed request.
        let slow = ClockSample::new(
            1_000_000,
            completed - Duration::from_millis(5001),
            completed,
        )
        .unwrap();
        assert_eq!(slow.signing_at(completed), Err(Error::Clock));
    }
    #[test]
    fn r13_clock_bounds_round_outward_once_across_fractional_request_and_projection() {
        let started = Instant::now();
        let completed = started + Duration::from_nanos(1_800_001);
        let sample = ClockSample::new(1_000_000, started, completed).unwrap();
        assert_eq!(sample.lower_at(completed).unwrap(), 1_000_000);
        assert_eq!(sample.upper_at(completed).unwrap(), 1_000_002);
        let observed = completed + Duration::from_nanos(1_800_001);
        assert_eq!(sample.lower_at(observed).unwrap(), 1_000_001);
        assert_eq!(sample.upper_at(observed).unwrap(), 1_000_004);
        assert_eq!(
            sample.signing_at(observed).unwrap(),
            1_000_001,
            "signing keeps the inward lower bound"
        );

        let almost_tick =
            ClockSample::new(2_000_000, started, started + Duration::from_nanos(999_999)).unwrap();
        assert_eq!(
            almost_tick.upper_at(almost_tick.completed()).unwrap(),
            2_000_001
        );
        assert!(ClockSample::new(MAX_SERVER_MS, started, completed).is_err());
    }
    #[tokio::test]
    #[ignore = "requires public Binance connectivity; no credential or signed request"]
    async fn unsigned_account_transport_verifies_official_time_endpoint() {
        let mut transport = Transport::new().unwrap();
        let (_, clock) = transport.prepare().await.unwrap();
        assert!(clock.lower_at(Instant::now()).unwrap() > 1_700_000_000_000);
    }

    struct Slow;
    impl Source for Slow {
        async fn fetch(&mut self, _: &Credentials) -> Result<Data> {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(example())
        }
    }
    #[tokio::test(start_paused = true)]
    async fn disconnect_cancels_pending_snapshot_without_republishing_account() {
        let (sender, input) = mpsc::channel(4);
        let (output, mut snapshots) = watch::channel(Snapshot::default());
        let task = tokio::spawn(async move {
            worker(input, output, &mut Slow).await;
        });
        sender
            .send(Command::Connect(
                Credentials::new("dummy_public_key_123".into(), "dummy_secret_123456".into())
                    .unwrap(),
                false,
            ))
            .await
            .unwrap();
        snapshots.changed().await.unwrap();
        sender.send(Command::Disconnect).await.unwrap();
        snapshots.changed().await.unwrap();
        assert_eq!(snapshots.borrow().generation, 2);
        tokio::time::advance(Duration::from_secs(120)).await;
        assert_eq!(snapshots.borrow().status, Status::Disconnected);
        assert!(snapshots.borrow().data.is_none());
        drop(sender);
        task.await.unwrap();
    }
    impl Source for Fake {
        async fn fetch(&mut self, _: &Credentials) -> Result<Data> {
            self.count += 1;
            if self.count == 2 {
                Err(Error::Network)
            } else {
                Ok(example())
            }
        }
    }
    #[tokio::test(start_paused = true)]
    async fn worker_recovers_then_disconnect_clears_all_private_values() {
        let (sender, input) = mpsc::channel(4);
        let (output, mut snapshots) = watch::channel(Snapshot::default());
        let task = tokio::spawn(async move {
            worker(input, output, &mut Fake { count: 0 }).await;
        });
        sender
            .send(Command::Connect(
                Credentials::new("dummy_public_key_123".into(), "dummy_secret_123456".into())
                    .unwrap(),
                false,
            ))
            .await
            .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow().fresh() {
                break;
            }
        }
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow().error == Some(Error::Network) {
                break;
            }
        }
        assert!(snapshots.borrow().data.is_some());
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow().fresh() {
                break;
            }
        }
        sender.send(Command::Disconnect).await.unwrap();
        snapshots.changed().await.unwrap();
        assert_eq!(snapshots.borrow().status, Status::Disconnected);
        assert!(snapshots.borrow().data.is_none());
        drop(sender);
        task.await.unwrap();
    }
}
