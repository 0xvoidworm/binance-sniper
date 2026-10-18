mod execution;
pub(crate) mod public;
pub(crate) mod strategy;
use anyhow::{Context, Result};
use binance_engine::State;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    Idle,
    Connecting,
    Running,
    Recovering,
    Stopping,
    Finished,
    Failed,
}
impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "BELUM BERJALAN",
            Self::Connecting => "MENYIAPKAN PASAR",
            Self::Running => "PAPER BERJALAN",
            Self::Recovering => "DATA TERPUTUS",
            Self::Stopping => "MENUTUP VIRTUAL",
            Self::Finished => "SESI SELESAI",
            Self::Failed => "SESI GAGAL",
        }
    }
    pub fn finished(self) -> bool {
        matches!(self, Self::Finished | Self::Failed)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub phase: Phase,
    pub reason: String,
    pub error: Option<String>,
    pub state: Option<State>,
    pub signal: Option<strategy::Signal>,
    pub started_ms: u64,
    pub now: u64,
    pub deadline_ms: u64,
    pub next_funding: u64,
    pub updates: u64,
    pub records: u64,
    pub complete_costs: bool,
    pub path: Option<PathBuf>,
    pub history: Vec<String>,
    #[serde(skip)]
    pub received: Option<Instant>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            reason: "Mesin paper belum dimulai".into(),
            error: None,
            state: None,
            signal: None,
            started_ms: 0,
            now: 0,
            deadline_ms: 0,
            next_funding: 0,
            updates: 0,
            records: 0,
            complete_costs: true,
            path: None,
            history: vec![],
            received: None,
        }
    }
}
impl Snapshot {
    pub fn fresh(&self) -> bool {
        self.error.is_none()
            && self
                .received
                .is_some_and(|t| t.elapsed() < Duration::from_secs(8))
    }
}
pub struct Feed {
    commands: Option<mpsc::Sender<()>>,
    output: watch::Receiver<Snapshot>,
    task: Option<std::thread::JoinHandle<()>>,
}
impl Feed {
    pub fn start(root: PathBuf) -> Result<Self> {
        let (sender, receiver) = mpsc::channel(2);
        let (output, read) = watch::channel(Snapshot {
            phase: Phase::Connecting,
            ..Snapshot::default()
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let task = std::thread::Builder::new()
            .name("binance-forward-paper".into())
            .spawn(move || runtime.block_on(worker(root, receiver, output)))?;
        Ok(Self {
            commands: Some(sender),
            output: read,
            task: Some(task),
        })
    }
    pub fn latest(&self) -> Snapshot {
        self.output.borrow().clone()
    }
    pub fn stop(&self) -> bool {
        self.commands
            .as_ref()
            .is_some_and(|s| s.try_send(()).is_ok())
    }

    #[cfg(test)]
    pub fn fixture(snapshot: Snapshot) -> (Self, mpsc::Receiver<()>) {
        let (sender, receiver) = mpsc::channel(2);
        let (_, output) = watch::channel(snapshot);
        (
            Self {
                commands: Some(sender),
                output,
                task: None,
            },
            receiver,
        )
    }
}
impl Drop for Feed {
    fn drop(&mut self) {
        // Cancel the in-flight GET, record the interrupted virtual session,
        // and release the journal lock before the native process exits.
        self.commands.take();
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}
async fn worker(root: PathBuf, mut commands: mpsc::Receiver<()>, output: watch::Sender<Snapshot>) {
    let mut public = match public::Public::new() {
        Ok(p) => p,
        Err(e) => {
            output.send_modify(|s| {
                s.phase = Phase::Failed;
                s.error = Some(e.to_string());
            });
            return;
        }
    };
    let mut session: Option<execution::Session> = None;
    let mut first = true;
    let started = Instant::now();
    loop {
        let delay = if first {
            Duration::ZERO
        } else {
            Duration::from_secs(2)
        }
        .max(public.cooldown());
        first = false;
        let batch = tokio::select! {
            biased;
            command=commands.recv()=> {
                if let Some(ref mut session)=session {
                    if command.is_none() { session.fail("Jendela ditutup; sesi tidak dilanjutkan otomatis"); }
                    else if let Err(error)=session.request_stop("Dihentikan pengguna") {session.fail(&error.to_string());}
                    output.send_replace(session.snapshot.clone());
                    if command.is_none() || session.snapshot.phase.finished() {return;}
                } else { output.send_modify(|s|{s.phase=Phase::Finished;s.reason="Dihentikan sebelum sesi dibuat".into();});return; }
                continue;
            }
            batch=async {tokio::time::sleep(delay).await;tokio::time::timeout(Duration::from_secs(15),public.fetch()).await} => batch.map_err(|_|anyhow::anyhow!("Batch pasar timeout")).and_then(|b|b),
        };
        match batch {
            Ok(batch) => {
                if session.is_none() {
                    let result = (|| -> Result<execution::Session> {
                        std::fs::create_dir_all(&root)?;
                        let ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
                        let path = root.join(format!("binance-paper-{ms}-{}", std::process::id()));
                        execution::Session::new(path, &batch)
                    })();
                    match result {
                        Ok(s) => session = Some(s),
                        Err(e) => {
                            output.send_modify(|s| {
                                s.phase = Phase::Failed;
                                s.error = Some(e.to_string());
                            });
                            return;
                        }
                    }
                } else if let Some(ref mut session) = session
                    && let Err(e) = session.update(&batch)
                {
                    session.fail(&e.to_string());
                }
            }
            Err(e) => {
                if let Some(ref mut s) = session {
                    if let Err(error) = s.outage(&e.to_string()) {
                        s.fail(&error.to_string());
                    }
                } else {
                    output.send_modify(|s| {
                        s.error = Some(e.to_string());
                        if started.elapsed() > Duration::from_secs(60) {
                            s.phase = Phase::Failed;
                        }
                    });
                    if started.elapsed() > Duration::from_secs(60) {
                        return;
                    }
                }
            }
        }
        if let Some(ref session) = session {
            output.send_replace(session.snapshot.clone());
            if session.snapshot.phase.finished() {
                return;
            }
        }
    }
}
pub fn probe(root: PathBuf) -> Result<()> {
    let feed = Feed::start(root)?;
    let start = Instant::now();
    loop {
        let state = feed.latest();
        anyhow::ensure!(
            !state.phase.finished(),
            "Paper probe gagal: {}",
            state.error.as_deref().unwrap_or(&state.reason)
        );
        if state.updates >= 3 && state.fresh() {
            anyhow::ensure!(feed.stop(), "Worker paper berhenti");
            loop {
                let s = feed.latest();
                if s.phase.finished() {
                    anyhow::ensure!(
                        s.phase == Phase::Finished
                            && s.state.as_ref().is_some_and(|x| x.active.is_none()),
                        "Sesi probe tidak berakhir flat"
                    );
                    println!(
                        "PAPER_PUBLIC_FEED_PASS; snapshots>=3; stopped_flat=true; virtual_only=true; path={}",
                        s.path.context("Sesi tidak tersedia")?.display()
                    );
                    return Ok(());
                }
                anyhow::ensure!(
                    start.elapsed() < Duration::from_secs(75),
                    "Stop paper timeout"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        anyhow::ensure!(
            start.elapsed() < Duration::from_secs(70),
            "Paper probe timeout: {}",
            state.error.as_deref().unwrap_or(&state.reason)
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
