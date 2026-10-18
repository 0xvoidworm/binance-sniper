//! Live activation is reachable only from the explicit native UI action.
use super::{
    automatic::Decision,
    exit::Reason as ExitReason,
    planning, readiness,
    session_input::{self, History, Mode, Sampler},
    session_store::{Config, Phase, Store, digest},
    supervisor::{Engine, Frame, Reconciled},
    transport::{Access, ClockWindow, Exchange},
};
use crate::vault::{self, Credentials};
use anyhow::{Context, Result, ensure};
use rust_decimal::Decimal as D;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct Snapshot {
    pub phase: Phase,
    pub busy: bool,
    pub active: bool,
    pub unfinished: bool,
    pub reason: String,
    pub error: Option<String>,
    pub decision: Option<Decision>,
    pub position: Option<D>,
    pub stop: Option<D>,
    pub target: Option<D>,
    pub net: D,
    pub fees: D,
    pub funding: D,
    pub completed: u32,
    pub updated: u64,
    pub received: Option<Instant>,
    pub server_ms: u64,
    pub last_bar: Option<u64>,
    pub started: Option<u64>,
    pub deadline: Option<u64>,
    pub setup: Option<SetupCheck>,
    pub diagnostics: Option<super::diagnostics::Diagnostics>,
    pub recovery: Option<super::recovery::State>,
}
#[derive(Clone)]
pub struct SetupCheck {
    pub config: Config,
    pub report: planning::Assessment,
    pub received: Instant,
    pub market_summary: String,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self{phase:Phase::Ready,busy:false,active:false,unfinished:false,reason:"Belum ada sesi live. Strategi memilih arah dan SL/TP otomatis setelah sesi diaktifkan.".into(),error:None,decision:None,position:None,stop:None,target:None,net:D::ZERO,fees:D::ZERO,funding:D::ZERO,completed:0,updated:0,received:None,server_ms:0,last_bar:None,started:None,deadline:None,setup:None,diagnostics:None,recovery:None}
    }
}
impl Snapshot {
    fn inspect_decision(&mut self, decision: Decision, now: u64, received: Instant) {
        if !self.unfinished && self.phase != Phase::Blocked {
            self.reason = format!(
                "Analisis arah: {}. Sesi live belum diaktifkan",
                decision.reason
            );
        }
        self.decision = Some(decision);
        // Read-only inspection cannot extend a historical session's duration.
        if self.started.is_none() {
            self.updated = now;
        }
        self.received = Some(received);
    }
    pub fn ended_early(&self) -> bool {
        self.phase == Phase::Finished
            && self.started.is_some()
            && self
                .deadline
                .is_some_and(|deadline| self.updated < deadline)
    }
    pub fn timing_summary(&self) -> Option<String> {
        let started = self.started?;
        let deadline = self.deadline?;
        let now = if self.active {
            self.received.map_or(self.updated, |at| {
                self.server_ms
                    .saturating_add(at.elapsed().as_millis() as u64)
            })
        } else {
            self.updated
        };
        let seconds = now.saturating_sub(started) / 1000;
        Some(format!(
            "Durasi tercatat: {} menit {} detik / target {} menit{}",
            seconds / 60,
            seconds % 60,
            deadline.saturating_sub(started) / 60_000,
            if self.ended_early() {
                " · BERAKHIR LEBIH AWAL"
            } else {
                ""
            }
        ))
    }
    pub fn stale(&self) -> bool {
        self.active
            && self
                .received
                .is_none_or(|at| at.elapsed() > Duration::from_secs(8))
    }
    pub fn next_candle_seconds(&self) -> Option<u64> {
        if !self.active || self.phase != Phase::Watching || self.stale() {
            return None;
        }
        let now = self
            .server_ms
            .checked_add(self.received?.elapsed().as_millis() as u64)?;
        let next = self
            .last_bar?
            .checked_add(crate::paper::strategy::INTERVAL + 1)?;
        Some(next.saturating_sub(now).div_ceil(1000))
    }
    pub fn status_label(&self) -> &'static str {
        if self.busy {
            "Sesi sedang diperiksa"
        } else if self.active && self.recovery.as_ref().is_some_and(|r| r.active) {
            "Pemulihan transport; tutup posisi / periksa data, entry dijeda"
        } else if self.stale() && matches!(self.phase, Phase::Watching | Phase::Trading) {
            "Data sesi terlambat; menunggu pembacaan baru"
        } else {
            match self.phase {
                Phase::Recovering | Phase::Blocked => {
                    "Sesi tertahan / rekonsiliasi; entry dihentikan"
                }
                Phase::Stopping | Phase::Accounting => "Sesi sedang dihentikan / diperiksa",
                Phase::Finished if self.ended_early() => {
                    "Sesi berakhir lebih awal; periksa penyebab penghentian"
                }
                Phase::Finished => "Sesi selesai; belum ada sesi baru",
                Phase::Watching if self.active => "Sesi live menunggu sinyal",
                Phase::Trading if self.active => "Sesi live mengelola posisi",
                _ => "Sesi live tidak berjalan di jendela ini",
            }
        }
    }
    pub fn saved(root: &Path) -> Self {
        match super::session_store::read_saved(root) {
            Ok(Some(state)) => match state.session {
                Some(s) => Self {
                    phase: if s.finished {
                        Phase::Finished
                    } else {
                        Phase::Recovering
                    },
                    unfinished: !s.finished,
                    reason: if s.finished {
                        format!("Catatan sesi terakhir: {}. Belum ada sesi baru", s.reason)
                    } else {
                        format!(
                            "Catatan sesi belum selesai: {}. Periksa jendela lain atau pilih Pulihkan & tutup sesi lama; ini belum pembacaan Binance terkini",
                            s.reason
                        )
                    },
                    decision: s.decision,
                    net: s.net,
                    fees: s.fees,
                    funding: s.funding,
                    completed: s.completed,
                    updated: s.updated,
                    started: Some(s.started),
                    deadline: Some(s.deadline),
                    diagnostics: s.diagnostics,
                    recovery: s.recovery,
                    ..Default::default()
                },
                None => Self::default(),
            },
            Ok(None) => Self::default(),
            Err(e) => Self {
                phase: Phase::Blocked,
                error: Some(e.to_string()),
                reason: "Catatan sesi tidak terbaca; periksa pemulihan sebelum trading".into(),
                ..Default::default()
            },
        }
    }
}
pub enum Control {
    Credentials(Option<Credentials>),
    Inspect,
    Check(Config),
    Start(Config),
    Resume,
    Stop,
}
pub struct Feed {
    sender: mpsc::Sender<Control>,
    output: watch::Receiver<Snapshot>,
    activation: Arc<AtomicBool>,
}
impl Feed {
    #[cfg(test)]
    pub(crate) fn fixture(
        snapshot: Snapshot,
    ) -> (Self, mpsc::Receiver<Control>, watch::Sender<Snapshot>) {
        let (sender, input) = mpsc::channel(8);
        let (output, read) = watch::channel(snapshot);
        (
            Self {
                sender,
                output: read,
                activation: Arc::new(AtomicBool::new(false)),
            },
            input,
            output,
        )
    }
    pub fn start(root: PathBuf) -> Result<Self> {
        let (sender, input) = mpsc::channel(8);
        let (output, read) = watch::channel(Snapshot::saved(&root));
        let activation = Arc::new(AtomicBool::new(false));
        let worker_activation = activation.clone();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("binance-live-session".into())
            .spawn(move || runtime.block_on(worker(root, input, output, worker_activation)))?;
        Ok(Self {
            sender,
            output: read,
            activation,
        })
    }
    pub fn send(&self, command: Control) -> bool {
        let activating = matches!(command, Control::Start(_) | Control::Resume);
        if activating
            && (self.output.borrow().busy
                || self
                    .activation
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err())
        {
            return false;
        }
        let accepted = self.sender.try_send(command).is_ok();
        if activating && !accepted {
            self.activation.store(false, Ordering::Release);
        }
        accepted
    }
    pub fn latest(&self) -> Snapshot {
        let mut value = self.output.borrow().clone();
        value.busy |= self.activation.load(Ordering::Acquire) && !value.active;
        value
    }
    pub fn session_pending(&self) -> bool {
        self.activation.load(Ordering::Acquire)
    }
}
struct Source {
    inner: readiness::Source,
    history: History,
    sampler: Option<Sampler>,
    clock: Option<ClockWindow>,
}
impl Source {
    fn new(credentials: Credentials, access: Access) -> Result<Self> {
        Ok(Self {
            inner: readiness::Source::with_access(credentials, access)?,
            history: History::default(),
            sampler: None,
            clock: None,
        })
    }
    async fn fetch(&mut self, mode: Mode) -> Result<Frame> {
        let frame = session_input::poll(&mut self.inner, &mut self.history, mode).await?;
        self.clock = Some(frame.ready.clock);
        Ok(frame)
    }
    async fn session_frame(
        &mut self,
        engine: Option<&Engine>,
        mode: Mode,
    ) -> Result<(Frame, Reconciled)> {
        ensure!(
            engine.is_some() || !self.inner.private.live_enabled(),
            "Observer snapshot memerlukan transport read-only"
        );
        static CYCLE: AtomicU64 = AtomicU64::new(1);
        let cycle = CYCLE.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        let mut stages = [0u128; 4];
        // Also clears a lease abandoned by Stop/deadline cancellation.
        self.inner.private.end_read_window();
        let result = async {
            if mode == Mode::Recovery {
                self.sampler = None; // Abort public-only background work on Stop.
            } else {
                if self.sampler.is_none() {
                    self.sampler = Some(Sampler::new()?);
                }
                let clock = self.clock.context("Clock history belum tersedia")?;
                self.sampler
                    .as_mut()
                    .unwrap()
                    .poll(&mut self.history, clock.current_earliest())?;
            }
            self.inner.prepare_reference(Duration::from_secs(5)).await?;
            stages[0] = started.elapsed().as_millis();
            self.inner.private.begin_read_window().await?;
            stages[1] = started.elapsed().as_millis();
            let mut reconciled = match engine {
                Some(engine) => engine.reconcile_orders(&mut self.inner.private).await?,
                None => Reconciled::observation(),
            };
            reconciled.cycle = cycle;
            stages[2] = started.elapsed().as_millis();
            // No kline request or second private DNS/clock preparation here.
            let mut frame = session_input::prepared_account(&mut self.inner).await?;
            self.clock = Some(frame.ready.clock);
            if mode == Mode::Signals {
                self.history.attach(&mut frame);
            }
            frame.fresh()?;
            stages[3] = started.elapsed().as_millis();
            Ok((frame, reconciled))
        }
        .await;
        self.inner.private.end_read_window();
        eprintln!(
            "LIVE_SNAPSHOT_CYCLE; cycle={cycle}; ok={}; elapsed_ms={}; reference_done_ms={}; clock_done_ms={}; orders_done_ms={}; frame_done_ms={}; history_pending={}",
            result.is_ok(),
            started.elapsed().as_millis(),
            stages[0],
            stages[1],
            stages[2],
            stages[3],
            self.sampler.as_ref().is_some_and(Sampler::pending)
        );
        result
    }
    fn cooldown(&self) -> Duration {
        self.inner
            .private
            .cooldown()
            .max(self.inner.public.public_cooldown())
    }
}
fn binding(credentials: &Credentials) -> String {
    // One-way identifier only; neither key nor secret is written into session files.
    let mut bytes = zeroize::Zeroizing::new(b"binance-desk-session-binding-v1\0".to_vec());
    bytes.extend_from_slice(credentials.key.as_bytes());
    digest(&bytes)
}
fn snapshot(engine: &Engine, frame: Option<&Frame>) -> Snapshot {
    let mut out = Snapshot {
        phase: engine.phase,
        active: !engine.finished(),
        unfinished: !engine.finished(),
        reason: engine.note.clone(),
        ..Default::default()
    };
    if let Some(s) = &engine.store.state.session {
        out.reason = s.reason.clone();
        out.decision = s.decision.clone();
        out.net = s.net;
        out.fees = s.fees;
        out.funding = s.funding;
        out.completed = s.completed;
        out.updated = s.updated;
        out.last_bar = Some(s.last_bar);
        out.started = Some(s.started);
        out.deadline = Some(s.deadline);
        out.diagnostics = s.diagnostics.clone();
        out.recovery = s.recovery.clone();
        if let Some(t) = &s.current {
            out.stop = Some(t.intent.plan.stop);
            out.target = Some(t.intent.plan.take_profit);
        }
    }
    if let Some(f) = frame {
        out.position = Some(f.position());
        out.received = Some(f.received);
        out.server_ms = f.ready.context.now;
    }
    out
}
async fn worker(
    root: PathBuf,
    mut input: mpsc::Receiver<Control>,
    output: watch::Sender<Snapshot>,
    activation: Arc<AtomicBool>,
) {
    let mut credentials = None;
    while let Some(command) = input.recv().await {
        if let Control::Credentials(value) = command {
            credentials = value;
            output.send_modify(|s| s.setup = None);
            continue;
        }
        if matches!(command, Control::Stop) {
            continue;
        }
        output.send_modify(|s| {
            s.busy = true;
            s.error = None;
        });
        let activating = matches!(command, Control::Start(_) | Control::Resume);
        let loaded = credentials.clone().map(Ok).unwrap_or_else(vault::load);
        let result = match loaded {
            Err(e) => Err(anyhow::anyhow!(e.message())),
            Ok(credentials) => match command {
                Control::Inspect => inspect(&root, credentials, &output).await,
                Control::Check(config) => match check_setup(&root, credentials, config).await {
                    Ok(check) => {
                        let mut state = Snapshot::saved(&root);
                        state.setup = Some(check);
                        output.send_replace(state);
                        Ok(())
                    }
                    Err(error) => Err(error),
                },
                Control::Start(config) => {
                    run(&root, credentials, Some(config), &mut input, &output).await
                }
                Control::Resume => run(&root, credentials, None, &mut input, &output).await,
                _ => Ok(()),
            },
        };
        if let Err(error) = result {
            eprintln!("LIVE_SESSION_ERROR: {error}");
            let mut state = Snapshot::saved(&root);
            state.error = Some(error.to_string());
            output.send_replace(state);
        } else {
            output.send_modify(|s| s.busy = false);
        }
        if activating {
            activation.store(false, Ordering::Release);
        }
    }
}
async fn inspect(
    root: &Path,
    credentials: Credentials,
    output: &watch::Sender<Snapshot>,
) -> Result<()> {
    let mut source = Source::new(credentials, Access::ReadOnly)?;
    let frame = tokio::time::timeout(Duration::from_secs(40), source.fetch(Mode::Signals))
        .await
        .context("Analisis arah timeout")??;
    let decision = frame.decision()?;
    let mut state = Snapshot::saved(root);
    state.inspect_decision(decision, frame.now(), frame.received);
    output.send_replace(state);
    Ok(())
}

async fn check_setup(root: &Path, credentials: Credentials, config: Config) -> Result<SetupCheck> {
    config.validate()?;
    let saved = super::session_store::read_saved(root)?;
    if let Some(saved) = &saved {
        ensure!(
            saved.binding.is_empty() || saved.binding == binding(&credentials),
            "Catatan sesi terikat akun lain; periksa pemulihan"
        );
        ensure!(
            saved
                .session
                .as_ref()
                .is_none_or(|s| s.finished && s.current.is_none()),
            "Sesi lama belum selesai; periksa pemulihan dahulu"
        );
    }
    let mut source = Source::new(credentials, Access::ReadOnly)?;
    let frame = tokio::time::timeout(Duration::from_secs(40), source.fetch(Mode::Signals))
        .await
        .context("Pemeriksaan setup timeout")??;
    let fee = source
        .inner
        .private
        .read(super::protocol::Read::FeeBurn)
        .await?;
    ensure!(
        fee["feeBurn"].as_bool() == Some(false),
        "Fee BNB belum didukung; setup belum siap"
    );
    let daily = saved
        .as_ref()
        .and_then(|s| s.daily.get(&(frame.now() / 86_400_000)))
        .copied()
        .unwrap_or(D::ZERO);
    let report = frame.setup(&config, daily)?;
    let decision = frame.decision()?;
    let atr_percent =
        decision.atr / frame.bars.last().context("Candle hilang")?.close * D::from(100);
    let mut context = frame.ready.context.clone();
    context.now = frame.now();
    let actual = [binance_engine::Side::Long, binance_engine::Side::Short]
        .map(|side| {
            let review = planning::review(&context, &report.effective_budget, side, decision.atr);
            review.log();
            review.summary()
        })
        .join(". ");
    let atr_in_range = (D::from(planning::MIN_ATR_BPS)..=D::from(planning::MAX_ATR_BPS))
        .contains(&(atr_percent * D::from(100)));
    let market_summary = format!(
        "Kondisi sekarang: ATR {}% (filter ATR {}). {}. Hitungan ATR aktual: {actual}. Fee taker {} bps; minimum {} BTC / {} USDT. Ini rencana hipotetis pada snapshot, bukan sinyal/profit. Entry tetap perlu candle baru terkonfirmasi, pola, funding dan data lolos.",
        atr_percent.round_dp(4),
        if atr_in_range {
            "lolos"
        } else {
            "tertahan: perlu 0.10–3.00%"
        },
        decision.reason,
        frame.ready.context.taker_rate * D::from(10_000),
        frame.ready.context.filters.min_qty,
        frame.ready.context.filters.min_notional
    );
    Ok(SetupCheck {
        config,
        report,
        received: Instant::now(),
        market_summary,
    })
}

/// Validates configured bounds twice, exclusively via GET. Never opens a writer
/// store or constructs an execution-enabled connector.
pub fn probe_setup(root: &Path, configuration: Option<&Path>) -> Result<()> {
    let config = if let Some(path) = configuration {
        ensure!(
            std::fs::metadata(path)?.len() <= 4096,
            "File konfigurasi terlalu besar"
        );
        serde_json::from_slice::<Config>(&std::fs::read(path)?)?
    } else {
        super::session_store::read_saved(root)?
            .and_then(|s| s.session)
            .context("Belum ada batas sesi tersimpan; gunakan --setup-config")?
            .config
    };
    config.validate()?;
    let credentials = vault::load().map_err(|e| anyhow::anyhow!(e.message()))?;
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        for attempt in 1..=2 {
            let check=check_setup(root,credentials.clone(),config.clone()).await?;
            println!("SETUP_READ_ONLY_CHECK; snapshot={attempt}; strategy={}; minutes={}; viable={}; checked={}; result={}; mutations=0",super::automatic::VERSION,config.minutes,check.report.viable(),check.report.checked,check.report.summary());
            println!("SETUP_EFFECTIVE_BOUNDS; margin={}; risk={}; notional={}; leverage={}; funding_reserve={}; session_loss={}; daily_loss={}; max_trades={}",check.report.effective_budget.margin,check.report.effective_budget.risk,check.report.effective_budget.max_notional,check.report.effective_budget.leverage,check.report.effective_budget.funding_reserve,config.session_loss,config.daily_loss,config.max_trades);
            println!("SETUP_CURRENT_MARKET; {}; mutations=0",check.market_summary);
            ensure!(check.report.viable(),"Setup tidak layak pada pemeriksaan ini; tidak ada aktivasi");
            if attempt==1 {tokio::time::sleep(Duration::from_secs(2)).await;}
        }
        println!("SETUP_READ_ONLY_PASS; snapshots=2; signal_guarantee=false; profit_proven=false; mutations=0");
        Ok(())
    })
}
fn deadline_at(deadline_ms: u64, now_ms: u64) -> Result<tokio::time::Instant> {
    tokio::time::Instant::now()
        .checked_add(Duration::from_millis(deadline_ms.saturating_sub(now_ms)))
        .context("Deadline sesi tidak valid")
}

async fn run(
    root: &Path,
    credentials: Credentials,
    config: Option<Config>,
    input: &mut mpsc::Receiver<Control>,
    output: &watch::Sender<Snapshot>,
) -> Result<()> {
    let recovering = config.is_none();
    let store = Store::open(root, &binding(&credentials))?;
    let mut engine = Engine::new(store);
    if config.is_none() {
        engine.resume()?;
    }
    // Construction alone never dispatches. begin()/resume() follow explicit UI activation.
    let mut source = Source::new(credentials, Access::LiveSession)?;
    let mut first = Some(
        tokio::time::timeout(
            Duration::from_secs(40),
            source.fetch(if config.is_some() {
                Mode::Signals
            } else {
                Mode::Recovery
            }),
        )
        .await
        .context("Preflight sesi timeout")??,
    );
    if let Some(config) = config {
        let fee = source
            .inner
            .private
            .read(super::protocol::Read::FeeBurn)
            .await?;
        ensure!(
            fee["feeBurn"].as_bool() == Some(false),
            "Fee BNB belum didukung; sesi belum mengubah margin/leverage atau membuat order"
        );
        engine.begin(config, first.as_ref().unwrap())?;
    }
    output.send_replace(snapshot(&engine, first.as_ref()));
    let mut channel_open = true;
    let session_deadline = deadline_at(
        engine
            .store
            .state
            .session
            .as_ref()
            .context("Sesi hilang")?
            .deadline,
        first.as_ref().context("Snapshot awal hilang")?.now(),
    )?;
    // Resume can have pending orders. Its first decision must use a NEW frame
    // after reconciliation, not the preflight frame captured above.
    if recovering {
        first = None;
    }
    let mut last_data_error = String::new();
    loop {
        // Stop queued during preflight takes effect before any configuration/order.
        while let Ok(command) = input.try_recv() {
            if matches!(command, Control::Stop) {
                engine.stop_with(
                    ExitReason::UserStop,
                    "Dihentikan pengguna; tutup posisi dan cocokkan biaya",
                )?;
            }
        }
        if input.is_closed() {
            channel_open = false;
            engine.stop_with(
                ExitReason::WindowClosed,
                "Jendela ditutup; selesaikan penutupan sesi",
            )?;
        }
        let mode = if engine.close_only() {
            Mode::Recovery
        } else {
            Mode::Signals
        };
        let frame = if let Some(frame) = first.take() {
            let reconciled = engine.reconcile_orders(&mut source.inner.private).await?;
            Ok((frame, reconciled))
        } else {
            tokio::select! {
                biased;
                command=input.recv(),if channel_open=>{
                    match command {
                        Some(Control::Stop)=>engine.stop_with(ExitReason::UserStop,"Dihentikan pengguna; tutup posisi dan cocokkan biaya")?,
                        None=>{channel_open=false;engine.stop_with(ExitReason::WindowClosed,"Jendela ditutup; selesaikan penutupan sesi")?;}
                        _=>{output.send_modify(|s|s.error=Some("Selesaikan sesi aktif sebelum mengganti pengaturan/kredensial".into()));}
                    }
                    output.send_replace(snapshot(&engine,None));continue;
                }
                _=tokio::time::sleep_until(session_deadline),if !engine.stopping()=>{
                    let reason=if last_data_error.is_empty(){"Durasi sesi habis; entry dihentikan".into()}else{format!("Durasi sesi habis; entry dihentikan. Gangguan data terakhir: {last_data_error}")};
                    engine.stop_with(ExitReason::SessionDeadline, &reason)?;
                    output.send_replace(snapshot(&engine,None));continue;
                }
                result=async{tokio::time::sleep(Duration::from_secs(2).max(source.cooldown())).await;tokio::time::timeout(Duration::from_secs(40),source.session_frame(Some(&engine), mode)).await}=>result.map_err(|_|anyhow::anyhow!("Snapshot/rekonsiliasi sesi timeout")).and_then(|r|r),
            }
        };
        match frame {
            Err(error) => {
                last_data_error = error.to_string();
                eprintln!("LIVE_SESSION_DATA_ERROR: {error}");
                engine.read_failed(&error)?;
                let mut state = snapshot(&engine, None);
                state.error = Some(error.to_string());
                output.send_replace(state);
            }
            Ok((frame, reconciled)) => {
                last_data_error.clear();
                let cycle = reconciled.cycle;
                let result = engine
                    .tick_prepared(&mut source.inner.private, &frame, reconciled)
                    .await;
                if let Err(error) = result {
                    eprintln!("LIVE_SESSION_CHECK_ERROR; cycle={cycle}: {error}");
                    engine.check_failed(&error)?;
                    engine.phase = Phase::Recovering;
                    let mut state = snapshot(&engine, Some(&frame));
                    state.error = Some(error.to_string());
                    output.send_replace(state);
                } else {
                    output.send_replace(snapshot(&engine, Some(&frame)));
                }
                if engine.finished() {
                    return Ok(());
                }
            }
        }
    }
}
/// Public signal plus authenticated GET preflight; never constructs a live transport.
pub fn probe(seconds: Option<u64>) -> Result<()> {
    let credentials = vault::load().map_err(|e| anyhow::anyhow!(e.message()))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all().build()?.block_on(async {
            let mut source = Source::new(credentials, Access::ReadOnly)?;
            let started = Instant::now();
            let mut snapshots = 0;
            let mut failures = 0;
            let mut max_age = 0;
            let mut last_bar = None;
            let mut rollovers = 0;
            let mut last_healthy: Option<Instant> = None;
            let mut max_gap_ms = 0;
            let mut skipped = 0;
            let mut signal_bars = std::collections::BTreeSet::new();
            let mut decision_bars = std::collections::BTreeSet::new();
            loop {
                let remaining = seconds.map(|s| Duration::from_secs(s).saturating_sub(started.elapsed()));
                if remaining.is_some_and(|time| time.is_zero()) { break; }
                let timeout = remaining.unwrap_or(Duration::from_secs(40)).min(Duration::from_secs(40));
                let result = tokio::time::timeout(timeout, async {
                    // Auxiliary permission check precedes the measured snapshot;
                    // its DNS/clock round must not age the account/quote afterward.
                    source.inner.private.end_read_window();
                    ensure!(source.inner.private.read(super::protocol::Read::FeeBurn).await?["feeBurn"].is_boolean(), "Status fee BNB tidak terbaca");
                    let frame = if source.clock.is_none() {
                        source.fetch(Mode::Signals).await?
                    } else {
                        source.session_frame(None, Mode::Signals).await?.0
                    };
                    let decision = frame.decision()?;
                    frame.fresh()?;
                    ensure!(frame.max_notional > D::ZERO, "Max notional akun belum terbaca");
                    Ok::<_, anyhow::Error>((frame, decision))
                }).await;
                if result.is_err() && remaining.is_some_and(|time| time <= Duration::from_secs(40)) { break; }
                let result = result.context("Probe arah timeout").and_then(|r| r);
                if let Some(healthy) = last_healthy {
                    max_gap_ms = max_gap_ms.max(healthy.elapsed().as_millis());
                }
                match result {
                    Ok((frame, decision)) => {
                        last_healthy = Some(Instant::now());
                        snapshots += 1;
                        max_age = max_age.max(frame.now().saturating_sub(frame.account_as_of));
                        if last_bar.is_some_and(|bar| decision.bar_ms > bar) { rollovers += 1; }
                        last_bar = Some(decision.bar_ms);
                        skipped += u64::from(frame.skip_signal.is_some());
                        if frame.now().saturating_sub(decision.bar_ms) <= 15_000 && decision_bars.insert(decision.bar_ms) {
                            println!("AUTO_CANDLE_DECISION; bar_ms={}; side={:?}; skipped={}; reason={}; mutations=0", decision.bar_ms, decision.side, frame.skip_signal.is_some(), decision.reason);
                        }
                        if decision.side.is_some() { signal_bars.insert(decision.bar_ms); }
                        if seconds.is_some() {
                            println!("AUTO_POLL; snapshots={snapshots}; elapsed_s={}; account_age_ms={}; closed_bar_ms={}; reason={}; mutations=0", started.elapsed().as_secs(), frame.now().saturating_sub(frame.account_as_of), decision.bar_ms, decision.reason);
                        }
                    }
                    Err(error) => {
                        if seconds.is_none() { return Err(error); }
                        failures += 1;
                        println!("AUTO_POLL_ERROR; elapsed_s={}; error={error}; mutations=0", started.elapsed().as_secs());
                    }
                }
                if seconds.map_or(snapshots >= 2, |s| started.elapsed() >= Duration::from_secs(s)) { break; }
                if let Some(seconds) = seconds {
                    let remaining = Duration::from_secs(seconds).saturating_sub(started.elapsed());
                    tokio::time::sleep(Duration::from_secs(2).max(source.cooldown()).min(remaining)).await;
                }
            }
            if let Some(healthy) = last_healthy { max_gap_ms = max_gap_ms.max(healthy.elapsed().as_millis()); }
            println!("AUTO_READ_ONLY_HEALTH; observed_s={}; requested_s={}; max_healthy_gap_ms={max_gap_ms}; within_30s_recovery_budget={}; revision_candidates={}; rewarms={}; confirmed_bars={}; held_snapshots={skipped}; fresh_decision_bars={}; signal_bars={}; account_values=REDACTED; mutations=0", started.elapsed().as_secs(), seconds.unwrap_or(0), snapshots>=2 && max_gap_ms<30_000, source.history.revisions, source.history.rewarms, source.history.confirmations, decision_bars.len(), signal_bars.len());
            println!("AUTO_STRATEGY_READ_ONLY_RESULT; snapshots={snapshots}; failures={failures}; rollovers={rollovers}; max_snapshot_age_ms={max_age}; account_values=REDACTED; mutations=0");
            ensure!(snapshots >= 2 && failures == 0, "Probe input sesi belum stabil; lihat status polling");
            Ok(())
        })
}

/// Finish only an empty, never-traded, stopped/expired local record after two GET snapshots.
/// This path never constructs a live transport and cannot submit orders.
pub fn finish_empty_session(root: &Path) -> Result<()> {
    let credentials = vault::load().map_err(|e| anyhow::anyhow!(e.message()))?;
    let store = Store::open(root, &binding(&credentials))?;
    let mut engine = Engine::new(store);
    ensure!(!engine.finished(), "Sesi sudah selesai");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut source = Source::new(credentials, Access::ReadOnly)?;
            let first = tokio::time::timeout(Duration::from_secs(40), source.fetch(Mode::Recovery))
                .await
                .context("Pemeriksaan pemulihan pertama timeout")??;
            engine.check_empty_recovery(&first)?;
            tokio::time::sleep(Duration::from_secs(1)).await;
            let second =
                tokio::time::timeout(Duration::from_secs(40), source.fetch(Mode::Recovery))
                    .await
                    .context("Pemeriksaan pemulihan kedua timeout")??;
            ensure!(
                second.ready.context.now > first.ready.context.now,
                "Snapshot pemulihan tidak maju"
            );
            engine.finish_empty_verified(&second)?;
            println!(
                "EMPTY_SESSION_FINISHED; snapshots=2; exchange_mutations=0; session_started=false"
            );
            Ok(())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r10_ui_distinguishes_trade_recovery_from_terminal_session_stop() {
        let mut state = Snapshot {
            active: true,
            recovery: Some(super::super::recovery::State {
                episodes: 1,
                active: true,
            }),
            ..Default::default()
        };
        for phase in [Phase::Recovering, Phase::Stopping, Phase::Accounting] {
            state.phase = phase;
            assert!(state.status_label().contains("Pemulihan transport"));
        }
        state.recovery.as_mut().unwrap().active = false;
        assert!(state.status_label().contains("dihentikan"));
        state.active = false;
        state.recovery.as_mut().unwrap().active = true;
        state.phase = Phase::Recovering;
        assert!(
            !state.status_label().contains("Pemulihan transport"),
            "saved state must not imply an automatically resumed worker"
        );
    }
    #[test]
    fn inspecting_new_data_never_extends_a_finished_sessions_duration() {
        let mut state = Snapshot {
            phase: Phase::Finished,
            started: Some(1000),
            deadline: Some(3_601_000),
            updated: 3_601_500,
            ..Default::default()
        };
        let duration = state.timing_summary();
        state.inspect_decision(
            Decision {
                bar_ms: 8_000_000,
                side: None,
                votes: vec![],
                reason: "new market observation".into(),
                atr: D::ONE,
                stop: None,
                target: None,
            },
            9_000_000,
            Instant::now(),
        );
        assert_eq!(state.updated, 3_601_500);
        assert_eq!(state.timing_summary(), duration);
        assert!(!state.active && !state.unfinished);
    }

    #[tokio::test(start_paused = true)]
    async fn original_monotonic_deadline_interrupts_a_hanging_data_read() {
        let started = tokio::time::Instant::now();
        let deadline = deadline_at(3_601_000, 1000).unwrap();
        tokio::time::advance(Duration::from_secs(31)).await;
        assert_eq!(
            deadline.duration_since(tokio::time::Instant::now()),
            Duration::from_secs(3569)
        );
        tokio::select! {
            _ = std::future::pending::<()>() => panic!("a hanging GET must not finish"),
            _ = tokio::time::sleep_until(deadline) => {}
        }
        assert_eq!(
            tokio::time::Instant::now().duration_since(started),
            Duration::from_secs(3600)
        );
        assert_eq!(
            deadline_at(1000, 2000).unwrap(),
            tokio::time::Instant::now()
        );
    }

    #[test]
    fn finished_session_displays_actual_duration_and_distinguishes_early_stop() {
        let mut state = Snapshot {
            phase: Phase::Finished,
            started: Some(1_000_000),
            deadline: Some(4_600_000),
            updated: 2_209_104,
            ..Default::default()
        };
        assert!(state.ended_early());
        assert!(state.status_label().contains("lebih awal"));
        assert_eq!(
            state.timing_summary().unwrap(),
            "Durasi tercatat: 20 menit 9 detik / target 60 menit · BERAKHIR LEBIH AWAL"
        );
        state.updated = 4_600_000;
        assert!(!state.ended_early());
        assert_eq!(
            state.timing_summary().unwrap(),
            "Durasi tercatat: 60 menit 0 detik / target 60 menit"
        );
        assert!(Snapshot::default().timing_summary().is_none());
    }

    #[test]
    fn waiting_countdown_stale_feed_and_recovery_have_distinct_status() {
        let interval = crate::paper::strategy::INTERVAL;
        let mut state = Snapshot {
            phase: Phase::Watching,
            active: true,
            unfinished: true,
            server_ms: 100 * interval,
            last_bar: Some(100 * interval - 1),
            received: Some(Instant::now()),
            ..Default::default()
        };
        assert_eq!(state.next_candle_seconds(), Some(300));
        assert_eq!(state.status_label(), "Sesi live menunggu sinyal");
        state.received = Some(Instant::now() - Duration::from_secs(9));
        assert!(state.stale());
        assert!(state.next_candle_seconds().is_none());
        assert!(state.status_label().contains("terlambat"));
        state.phase = Phase::Recovering;
        assert!(state.status_label().contains("rekonsiliasi"));
        state.active = false;
        state.phase = Phase::Finished;
        assert!(state.next_candle_seconds().is_none());
        assert_eq!(state.status_label(), "Sesi selesai; belum ada sesi baru");
    }
}
