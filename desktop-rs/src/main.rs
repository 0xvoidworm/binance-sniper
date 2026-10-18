mod account;
pub mod futures;
mod market;
mod model;
mod paper;
mod ui;
mod vault;

use anyhow::{Context, Result, ensure};
use clap::{Parser, ValueEnum};
use eframe::egui;
use std::{
    fs::{self, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    version,
    args_conflicts_with_subcommands = true,
    about = "Binance Desk — pasar, akun, margin Futures dan mesin, desktop Linux native"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long, value_enum, default_value = "workspace")]
    page: StartPage,
    #[arg(long, default_value_t = 0)]
    step: usize,
    /// Capture the native application framebuffer to a new PNG, then exit.
    #[arg(long)]
    capture: Option<PathBuf>,
    #[arg(long, default_value_t = 1400)]
    width: u32,
    #[arg(long, default_value_t = 900)]
    height: u32,
    /// Disable market and account networking (local simulation remains available).
    #[arg(long, conflicts_with_all = ["probe_account", "probe_market", "probe_paper", "paper_run", "probe_futures", "probe_auto"])]
    offline: bool,
    #[arg(long, value_enum, default_value = "futures")]
    market: market::Market,
    #[arg(long, value_enum, default_value = "1m")]
    timeframe: market::Timeframe,
    /// Delay framebuffer capture to allow real public data to arrive (max 60s).
    #[arg(long, default_value_t = 0)]
    capture_wait: u64,
    /// Verify public feeds and pause/resume without opening a window.
    #[arg(long)]
    probe_market: Option<PathBuf>,
    /// Check this application's KWallet entry without printing credentials.
    #[arg(long)]
    check_vault: bool,
    /// Test a disposable KWallet entry and delete it; no Binance requests.
    #[arg(long)]
    test_vault: bool,
    /// Read the saved Futures account twice, printing only connection status.
    #[arg(long)]
    probe_account: bool,
    /// Verify Futures brackets, fees and account preflight using signed GET only.
    #[arg(long)]
    probe_futures: bool,
    /// Verify autonomous signal inputs with signed GET only; never activates orders.
    #[arg(long)]
    probe_auto: bool,
    /// Observe repeated GET-only snapshots for this many seconds (maximum 3600).
    #[arg(long, requires = "probe_auto", value_parser = clap::value_parser!(u64).range(1..=3600))]
    probe_auto_seconds: Option<u64>,
    /// Check saved or supplied session bounds with GET only; never activate trading.
    #[arg(long, conflicts_with_all=["offline","capture","finish_empty_session","probe_auto","probe_futures","probe_account","probe_market","probe_paper","paper_run","paper_report","check_vault","test_vault"])]
    check_setup: bool,
    /// Optional Config JSON for --check-setup; does not change the saved session.
    #[arg(long, requires = "check_setup")]
    setup_config: Option<PathBuf>,
    /// Finish an empty stopped/expired local session after signed GET verification; never trades.
    #[arg(long, conflicts_with_all = ["offline", "capture", "probe_auto", "probe_futures", "probe_account", "probe_market", "probe_paper", "paper_run", "paper_report", "check_vault", "test_vault"])]
    finish_empty_session: bool,
    /// Start a 60-minute public-market simulation; no exchange orders.
    #[arg(long, conflicts_with_all = ["paper_report", "probe_paper"])]
    paper_run: bool,
    /// Verify public paper feed and stop flat; outputs virtual data only.
    #[arg(long)]
    probe_paper: Option<PathBuf>,
    /// Display a saved virtual session snapshot (read-only report).
    #[arg(long)]
    paper_report: Option<PathBuf>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Offline evaluation export/validation; no GUI, wallet or network startup.
    Evaluation(futures::evaluation::Cli),
}

#[derive(Clone, Copy, ValueEnum)]
enum StartPage {
    Workspace,
    Account,
    Futures,
    Paper,
    Simulation,
    Journal,
    Settings,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if let Some(Command::Evaluation(cli)) = args.command {
        return cli.run();
    }
    if args.check_vault {
        println!("{}", vault::check());
        return Ok(());
    }
    if args.test_vault {
        return vault::self_test();
    }
    if args.probe_account {
        return account::probe();
    }
    if args.probe_futures {
        return futures::probe();
    }
    if args.probe_auto {
        return futures::live::probe(args.probe_auto_seconds);
    }
    if let Some(path) = args.probe_market {
        return market::probe(&path);
    }
    if let Some(path) = args.probe_paper {
        return paper::probe(path);
    }
    ensure!(args.capture_wait <= 60, "Capture wait maksimum 60 detik");
    ensure!(
        (1024..=3840).contains(&args.width) && (720..=2160).contains(&args.height),
        "Ukuran jendela di luar rentang 1024×720 hingga 3840×2160"
    );
    if let Some(path) = &args.capture {
        ensure!(
            !path.exists(),
            "Screenshot tidak akan menimpa file yang sudah ada"
        );
    }
    let user_home = PathBuf::from(std::env::var_os("HOME").context("HOME tidak tersedia")?);
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| user_home.join(".config"));
    let data_root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| user_home.join(".local/share"));
    if args.check_setup {
        return futures::live::probe_setup(
            &data_root.join("binance-desk/live"),
            args.setup_config.as_deref(),
        );
    }
    if args.finish_empty_session {
        return futures::live::finish_empty_session(&data_root.join("binance-desk/live"));
    }
    let mut desk = ui::Desk::new(
        config_root.join("binance-desk/settings.json"),
        user_home.join("Workspace/outputs"),
    )?;
    desk.live_root = data_root.join("binance-desk/live");
    desk.restore_live_session();
    desk.page = match args.page {
        StartPage::Workspace => model::Page::Workspace,
        StartPage::Account => model::Page::Account,
        StartPage::Futures => model::Page::Futures,
        StartPage::Paper => model::Page::Paper,
        StartPage::Simulation => model::Page::Simulation,
        StartPage::Journal => model::Page::Journal,
        StartPage::Settings => model::Page::Settings,
    };
    desk.replay.seek(args.step);
    desk.capture = args.capture.clone();
    desk.capture_after =
        std::time::Instant::now() + std::time::Duration::from_secs(args.capture_wait);
    desk.market_request.market = args.market;
    desk.market_request.timeframe = args.timeframe;
    desk.market_request.paused = args.offline;
    desk.network_disabled = args.offline;
    desk.market = market::Snapshot::empty(desk.market_request);
    if !args.offline {
        desk.start_feed()?;
        desk.start_account()?;
        if matches!(desk.page, model::Page::Futures) {
            desk.start_futures()?;
            desk.futures_command(futures::ReadinessCommand::Load);
        }
    }
    if args.paper_run {
        desk.start_paper()?;
        desk.page = model::Page::Paper;
    }
    if let Some(path) = args.paper_report {
        ensure!(
            fs::metadata(&path)?.len() <= 131_072,
            "Laporan paper terlalu besar"
        );
        desk.paper = serde_json::from_slice(&fs::read(path)?)?;
        desk.paper_report = true;
        desk.page = model::Page::Paper;
    }
    if desk.capture.is_some() {
        desk.prefs = model::Preferences::default();
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("io.yuri.BinanceDesk")
            .with_title(concat!("Binance Desk ", env!("CARGO_PKG_VERSION")))
            .with_inner_size([args.width as f32, args.height as f32])
            .with_min_inner_size([1024.0, 720.0])
            .with_icon(icon()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Binance Desk",
        options,
        Box::new(move |cc| {
            ui::Desk::configure(&cc.egui_ctx, desk.prefs.scale);
            Ok(Box::new(desk))
        }),
    )
    .map_err(|error| anyhow::anyhow!("Gagal membuka aplikasi native: {error}"))?;
    if let Some(path) = &args.capture {
        ensure!(
            fs::metadata(path).is_ok_and(|meta| meta.len() > 8),
            "Screenshot tidak dihasilkan"
        );
    }
    Ok(())
}

fn icon() -> egui::IconData {
    let mut rgba = vec![0; 64 * 64 * 4];
    for y in 0..64 {
        for x in 0..64 {
            let inset = !(7..57).contains(&x) && !(7..57).contains(&y);
            let mut color = if inset {
                [0, 0, 0, 0]
            } else {
                [70, 220, 174, 255]
            };
            for (left, top, bottom) in [(13, 31, 50), (29, 20, 42), (45, 10, 32)] {
                if (left..left + 7).contains(&x) && (top..bottom).contains(&y) {
                    color = [12, 17, 25, 255];
                }
            }
            rgba[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&color);
        }
    }
    egui::IconData {
        rgba,
        width: 64,
        height: 64,
    }
}

pub fn save_png(path: &Path, image: &egui::ColorImage) -> Result<()> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut encoder = png::Encoder::new(
        BufWriter::new(file),
        image.width() as u32,
        image.height() as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.to_srgba_unmultiplied())
        .collect();
    writer.write_image_data(&bytes)?;
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod argument_tests {
    use super::*;

    #[test]
    fn p2_evaluation_cli_cannot_mix_with_network_vault_or_session_actions() {
        assert!(
            Args::try_parse_from(["binance-desk", "evaluation", "validate", "/packet"]).is_ok()
        );
        assert!(
            Args::try_parse_from([
                "binance-desk",
                "evaluation",
                "export",
                "--source",
                "/source",
                "--output",
                "/packet",
            ])
            .is_ok()
        );
        for flag in [
            "--probe-account",
            "--check-vault",
            "--test-vault",
            "--check-setup",
            "--probe-auto",
            "--paper-run",
            "--finish-empty-session",
            "--offline",
        ] {
            assert!(
                Args::try_parse_from(["binance-desk", flag, "evaluation", "validate", "/packet"])
                    .is_err()
            );
        }
        assert!(Args::try_parse_from(["binance-desk"]).is_ok());
        assert!(Args::try_parse_from(["binance-desk", "--check-setup"]).is_ok());
    }
}
