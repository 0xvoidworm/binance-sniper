use super::{
    margin::{self, Brackets, ContextData},
    protocol::{Read, Symbol},
    quotes::Book,
    transport::{Access, ClockWindow, Connector, Exchange},
};
use crate::{
    account, market,
    vault::{self, Credentials},
};
use anyhow::{Context, Result, ensure};
use rust_decimal::Decimal as D;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct Ready {
    pub account: account::Data,
    pub context: ContextData,
    pub current_leverage: u32,
    pub isolated: bool,
    pub blockers: Vec<&'static str>,
    pub next_funding: u64,
    pub(crate) clock: ClockWindow,
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub data: Option<Ready>,
    pub busy: bool,
    pub connected: bool,
    pub error: Option<String>,
    pub received: Option<Instant>,
    pub generation: u64,
    pub updates: u64,
}
impl Snapshot {
    pub fn fresh(&self) -> bool {
        self.error.is_none()
            && self
                .received
                .is_some_and(|t| t.elapsed() < Duration::from_secs(30))
    }
}
pub(crate) struct Source {
    pub(crate) private: Connector,
    pub(crate) public: market::Transport,
    pub(crate) regular: Vec<(String, u64)>,
    pub(crate) algos: Vec<String>,
    pub(crate) max_notional: D,
    pub(crate) account_started: Instant,
    reference: Option<(binance_engine::Filters, Brackets, D, Instant)>,
    last_book: Option<Book>,
}
impl Source {
    fn new(credentials: Credentials) -> Result<Self> {
        Self::with_access(credentials, Access::ReadOnly)
    }
    pub(crate) fn with_access(credentials: Credentials, access: Access) -> Result<Self> {
        Ok(Self {
            private: Connector::new(credentials, access)?,
            public: market::Transport::new()?,
            regular: vec![],
            algos: vec![],
            max_notional: D::ZERO,
            account_started: Instant::now(),
            reference: None,
            last_book: None,
        })
    }
    pub(crate) async fn prepare_reference(&mut self, headroom: Duration) -> Result<()> {
        // Warm DNS/static metadata before measuring the mutable account snapshot.
        // Refresh rules/fees every minute; no stale cache fallback on failure.
        if self
            .reference
            .as_ref()
            .is_none_or(|(_, _, _, at)| reference_due(at.elapsed(), headroom))
        {
            let metadata = self
                .public
                .json(market::Market::Futures, "exchangeInfo", &[])
                .await?;
            let filters = crate::paper::public::filters(&metadata)?;
            let brackets = Brackets::parse(
                &self.private.read(Read::Brackets(Symbol::BTCUSDT)).await?,
                "BTCUSDT",
            )?;
            let fees = self.private.read(Read::Commission(Symbol::BTCUSDT)).await?;
            ensure!(fees["symbol"] == "BTCUSDT", "Simbol fee berbeda");
            let rate = margin::decimal(&fees["takerCommissionRate"])?;
            ensure!(rate <= D::new(1, 2), "Fee akun di luar batas");
            self.reference = Some((filters, brackets, rate, Instant::now()));
        }
        Ok(())
    }
    pub(crate) async fn fetch(&mut self) -> Result<Ready> {
        self.private.end_read_window();
        self.prepare_reference(Duration::ZERO).await?;
        self.private.begin_read_window().await?;
        let result = self.fetch_prepared().await;
        self.private.end_read_window();
        result
    }
    pub(crate) async fn fetch_prepared(&mut self) -> Result<Ready> {
        // Never refresh slow reference data within an existing order proof.
        ensure!(
            self.reference
                .as_ref()
                .is_some_and(|(_, _, _, at)| !reference_due(at.elapsed(), Duration::ZERO)),
            "Metadata kedaluwarsa dalam jendela snapshot; perlu siklus baru"
        );
        let (filters, brackets, taker_rate, _) = self.reference.as_ref().unwrap().clone();
        let (batch, started) = self
            .private
            .read_batch(&[
                Read::Account,
                Read::Positions,
                Read::AccountConfig,
                Read::Symbols,
                Read::OpenOrders(Symbol::BTCUSDT),
                Read::OpenAlgos(Symbol::BTCUSDT),
            ])
            .await?;
        // DNS/clock preparation precedes the first mutable read. Its latency is
        // not account age; keep the five-second limit on the actual snapshot.
        self.account_started = started;
        let mut batch = batch.into_iter();
        let account = batch.next().context("Snapshot account hilang")?;
        let positions = batch.next().context("Snapshot posisi hilang")?;
        let configuration = batch.next().context("Snapshot konfigurasi hilang")?;
        let symbols = batch.next().context("Snapshot simbol hilang")?;
        self.max_notional = symbols
            .as_array()
            .and_then(|a| a.iter().find(|s| s["symbol"] == "BTCUSDT"))
            .and_then(|s| s.get("maxNotionalValue"))
            .map(margin::decimal)
            .transpose()?
            .unwrap_or(D::ZERO);
        let orders = batch.next().context("Snapshot order hilang")?;
        let orders = orders.as_array().context("Daftar order reguler invalid")?;
        let algos = batch.next().context("Snapshot algo hilang")?;
        let algos = algos.as_array().context("Daftar algo order invalid")?;
        ensure!(
            orders.len() <= 1000 && algos.len() <= 1000,
            "Terlalu banyak order untuk preflight ini"
        );
        self.regular = orders
            .iter()
            .map(|r| {
                Ok((
                    r["clientOrderId"]
                        .as_str()
                        .context("ID order preflight hilang")?
                        .to_owned(),
                    r["orderId"]
                        .as_u64()
                        .context("ID exchange preflight hilang")?,
                ))
            })
            .collect::<Result<_>>()?;
        self.algos = algos
            .iter()
            .map(|r| {
                r["clientAlgoId"]
                    .as_str()
                    .map(str::to_owned)
                    .context("ID algo preflight hilang")
            })
            .collect::<Result<_>>()?;
        let query = [("symbol", "BTCUSDT".into())];
        let book = self
            .public
            .json(
                market::Market::Futures,
                "depth",
                &[("symbol", "BTCUSDT".into()), ("limit", "5".into())],
            )
            .await?;
        let premium = self
            .public
            .json(market::Market::Futures, "premiumIndex", &query)
            .await?;
        // Never start another DNS/time GET after collecting mutable evidence.
        // A slow clock preparation occurs before that evidence, and an expired
        // lease discards the whole frame rather than stamping old data fresh.
        let clock = self.private.snapshot_clock()?;
        let now = clock.latest();
        let book = Book::parse_with_clock(&book, clock, self.last_book.as_ref())?;
        let premium_ms = premium["time"].as_u64().context("Timestamp mark hilang")?;
        ensure!(premium_ms > 0, "Timestamp mark hilang atau invalid");
        ensure!(premium["symbol"] == "BTCUSDT", "Simbol publik berbeda");
        let next_funding = premium["nextFundingTime"]
            .as_u64()
            .filter(|t| *t > 0)
            .context("Jadwal funding invalid")?;
        clock.prove_future("jadwal_funding", next_funding)?;
        let data = account::Data::parse(account, positions, configuration, symbols, now)
            .map_err(|e| anyhow::anyhow!(e.message()))?;
        let symbol = data
            .symbol("BTCUSDT")
            .context("Konfigurasi BTCUSDT hilang")?;
        let current_leverage = symbol.leverage;
        let isolated = symbol.margin_type == "ISOLATED";
        let mut blockers = Vec::new();
        if !data.config.can_trade {
            blockers.push("canTrade akun tidak aktif");
        }
        if data.config.dual_side_position {
            blockers.push("Mode Hedge belum didukung; diperlukan One-way");
        }
        if data.config.multi_assets_margin {
            blockers.push("Multi-assets belum didukung; diperlukan Single-asset USDT");
        }
        if data.positions.iter().any(|p| !p.position_amt.is_zero()) {
            blockers.push("Akun masih memiliki posisi; perlu rekonsiliasi kepemilikan");
        }
        if !orders.is_empty() || !algos.is_empty() {
            blockers.push("BTCUSDT masih memiliki order/algo aktif; perlu rekonsiliasi");
        }
        if symbol.is_auto_add_margin {
            blockers.push("Auto-add margin aktif; budget isolated belum dapat dibatasi");
        }
        let context = ContextData {
            filters,
            quote: book.quote(),
            mark: margin::decimal(&premium["markPrice"])?,
            now,
            available: data.balances.available_balance,
            taker_rate,
            brackets,
        };
        ensure!(
            context.quote.bid > D::ZERO
                && context.quote.ask >= context.quote.bid
                && context.mark > D::ZERO,
            "Harga preflight invalid"
        );
        let mark_age = clock.validate_age("mark", premium_ms, 10_000)?;
        eprintln!(
            "LIVE_QUOTE_HEALTH; source=depth5; last_update_id={}; event_age_ms={}; quote_age_ms={}; mark_age_ms={}",
            book.id,
            now - book.event_ms,
            now - book.transaction_ms,
            mark_age
        );
        self.last_book = Some(book);
        Ok(Ready {
            account: data,
            context,
            current_leverage,
            isolated,
            blockers,
            next_funding,
            clock,
        })
    }
}

fn reference_due(age: Duration, headroom: Duration) -> bool {
    age.saturating_add(headroom) >= Duration::from_secs(60)
}

#[test]
fn metadata_refresh_can_precede_order_queries_without_extending_cache_lifetime() {
    assert!(!reference_due(
        Duration::from_secs(54),
        Duration::from_secs(5)
    ));
    assert!(reference_due(
        Duration::from_secs(55),
        Duration::from_secs(5)
    ));
    assert!(!reference_due(Duration::from_secs(59), Duration::ZERO));
    assert!(reference_due(Duration::from_secs(60), Duration::ZERO));
    assert!(reference_due(Duration::MAX, Duration::from_secs(5)));
}
pub enum Command {
    Load,
    Connect(Credentials),
    StoreSession(Credentials),
    Refresh,
    Disconnect,
}
pub struct Feed {
    sender: mpsc::Sender<Command>,
    output: watch::Receiver<Snapshot>,
    generation: Cell<u64>,
}
impl Feed {
    pub fn start() -> Result<Self> {
        let (sender, input) = mpsc::channel(4);
        let (output, read) = watch::channel(Snapshot::default());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("binance-futures-preflight".into())
            .spawn(move || runtime.block_on(worker(input, output)))?;
        Ok(Self {
            sender,
            output: read,
            generation: Cell::new(0),
        })
    }
    pub fn send(&self, command: Command) -> bool {
        if self.sender.try_send(command).is_err() {
            return false;
        }
        self.generation.set(self.generation.get() + 1);
        true
    }
    pub fn latest(&self) -> Snapshot {
        let state = self.output.borrow().clone();
        if state.generation == self.generation.get() {
            state
        } else {
            Snapshot {
                busy: true,
                generation: self.generation.get(),
                ..Default::default()
            }
        }
    }
}
async fn worker(mut input: mpsc::Receiver<Command>, output: watch::Sender<Snapshot>) {
    let mut source: Option<Source> = None;
    let mut state = Snapshot::default();
    let mut fetch = false;
    loop {
        let command = if fetch && let Some(ref mut source) = source {
            let started = Instant::now();
            tokio::select! {
                biased;
                command=input.recv()=>command,
                result=tokio::time::timeout(Duration::from_secs(40),source.fetch())=>{
                    state.busy=false;fetch=false;
                    match result {
                        Ok(Ok(data))=>{state.data=Some(data);state.error=None;state.received=Some(started);state.updates+=1;},
                        Ok(Err(error))=>state.error=Some(error.to_string()),
                        Err(_)=>state.error=Some("Pembacaan preflight timeout".into()),
                    }
                    output.send_replace(state.clone());continue;
                }
            }
        } else {
            input.recv().await
        };
        let Some(command) = command else {
            return;
        };
        let generation = state.generation + 1;
        match command {
            Command::Refresh => {
                state.generation = generation;
                state.busy = source.is_some();
                fetch = source.is_some();
                if source.is_none() {
                    state.error = Some("Hubungkan kredensial dahulu".into());
                }
            }
            Command::Disconnect => {
                source = None;
                fetch = false;
                state = Snapshot {
                    generation,
                    ..Default::default()
                };
            }
            Command::Load | Command::Connect(_) | Command::StoreSession(_) => {
                let read_now = !matches!(&command, Command::StoreSession(_));
                source = None;
                fetch = false;
                state = Snapshot {
                    busy: true,
                    generation,
                    ..Default::default()
                };
                output.send_replace(state.clone());
                let credentials = match command {
                    Command::Load => tokio::task::spawn_blocking(vault::load)
                        .await
                        .map_err(|_| "Pembacaan KWallet gagal".to_string())
                        .and_then(|r| r.map_err(|e| e.message().to_owned())),
                    Command::Connect(value) | Command::StoreSession(value) => Ok(value),
                    _ => unreachable!(),
                };
                match credentials.and_then(|c| Source::new(c).map_err(|e| e.to_string())) {
                    Ok(s) => {
                        source = Some(s);
                        fetch = read_now;
                        state.busy = read_now;
                        state.connected = true;
                    }
                    Err(error) => {
                        state.error = Some(error);
                        state.busy = false;
                    }
                }
            }
        }
        output.send_replace(state.clone());
    }
}
pub fn probe() -> Result<()> {
    let feed = Feed::start()?;
    ensure!(feed.send(Command::Load), "Worker preflight gagal");
    let start = Instant::now();
    let mut refreshed = false;
    loop {
        let state = feed.latest();
        if !state.busy
            && let Some(error) = state.error
        {
            anyhow::bail!("{error}");
        }
        if state.updates >= 2 && state.fresh() {
            println!(
                "FUTURES_PREFLIGHT_READ_ONLY_PASS; snapshots=2; bracket_fee_orders_validated=true; account_values=REDACTED; mutations=0"
            );
            return Ok(());
        }
        if state.updates == 1 && !refreshed {
            ensure!(feed.send(Command::Refresh), "Refresh gagal");
            refreshed = true;
        }
        ensure!(
            start.elapsed() < Duration::from_secs(90),
            "Probe preflight timeout"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn storing_session_credentials_does_not_fetch_and_disconnect_clears_state() {
        let (send, receive) = mpsc::channel(4);
        let (output, mut watch) = watch::channel(Snapshot::default());
        let task = tokio::spawn(worker(receive, output));
        send.send(Command::StoreSession(
            Credentials::new("dummy-session-key".into(), "dummy-session-secret".into()).unwrap(),
        ))
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                watch.changed().await.unwrap();
                if watch.borrow().connected {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(!watch.borrow().busy && watch.borrow().data.is_none());
        assert_eq!(watch.borrow().updates, 0);
        send.send(Command::Disconnect).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                watch.changed().await.unwrap();
                if watch.borrow().generation == 2 {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(!watch.borrow().connected && watch.borrow().data.is_none());
        drop(send);
        task.await.unwrap();
    }
}
