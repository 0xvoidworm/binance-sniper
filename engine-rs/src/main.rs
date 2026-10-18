use anyhow::{Context, Result, ensure};
use binance_engine::{journal::hash, *};
use clap::{Parser, Subcommand};
use rust_decimal::Decimal as D;
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    version,
    about = "Offline paper execution core. No credentials, network or live orders."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate explicit paper configuration.
    ValidateConfig {
        #[arg(long)]
        config: PathBuf,
    },
    /// Create a new durable paper session; never overwrite a journal.
    Init {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        journal: PathBuf,
    },
    /// Apply normalized JSONL events; halt the batch on first rejection.
    Apply {
        #[arg(long)]
        journal: PathBuf,
        #[arg(long)]
        events: PathBuf,
    },
    /// Verify and replay the journal, then show state.
    Status {
        #[arg(long)]
        journal: PathBuf,
    },
    /// Run a fixed, entirely synthetic long-entry/TP scenario and verify restart.
    Demo {
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ValidateConfig { config } => {
            let config: Config = serde_json::from_slice(&read_bounded(&config)?)?;
            config.validate()?;
            println!(
                "{}",
                json!({"valid": true, "mode": "paper", "config_hash": hash(&config)?})
            );
        }
        Command::Init { config, journal } => {
            let config = serde_json::from_slice(&read_bounded(&config)?)?;
            let journal = Journal::create(&journal, config)?;
            println!("{}", serde_json::to_string_pretty(&report(&journal))?);
        }
        Command::Apply { journal, events } => {
            let bytes = read_bounded(&events)?;
            let text = std::str::from_utf8(&bytes)?;
            let events: Vec<Envelope> = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()?;
            ensure!(events.len() <= 10_000, "event batch exceeds limit");
            let mut journal = Journal::open(&journal)?;
            for event in events {
                let result = journal.apply(event)?;
                println!("{}", serde_json::to_string(&result)?);
                ensure!(
                    result.outcome.accepted,
                    "event rejected; batch stopped (rejection is journaled)"
                );
            }
        }
        Command::Status { journal } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report(&Journal::open(&journal)?))?
            );
        }
        Command::Demo { output } => demo(&output)?,
    }
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    File::open(path)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 2 * 1024 * 1024, "input file exceeds 2 MiB");
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn report(journal: &Journal) -> serde_json::Value {
    json!({
        "schema": SCHEMA,
        "purpose": "synthetic software verification; not strategy performance",
        "network_transport_available": false,
        "records": journal.records(),
        "head_hash": journal.head_hash(),
        "state": journal.state(),
    })
}

fn demo(output: &Path) -> Result<()> {
    fs::create_dir(output)
        .context("demo output must be a new directory with an existing parent")?;
    let config_bytes = include_bytes!("../fixtures/paper.json");
    let config: Config = serde_json::from_slice(config_bytes)?;
    write_new(&output.join("config.json"), config_bytes)?;
    let path = output.join("journal.jsonl");
    let mut journal = Journal::create(&path, config.clone())?;
    let mut events = vec![];
    let mut effects = vec![];
    let mut apply = |journal: &mut Journal, event: Event| -> Result<()> {
        let envelope = Envelope {
            id: format!("demo-{:02}", events.len() + 1),
            at_ms: 1_000_000 + events.len() as u64 * 50,
            event,
        };
        let result = journal.apply(envelope.clone())?;
        ensure!(
            result.outcome.accepted,
            "demo rejected: {:?}",
            result.outcome.error
        );
        effects.extend(result.outcome.effects);
        events.push(envelope);
        Ok(())
    };
    apply(
        &mut journal,
        Event::Quote {
            quote: Quote {
                bid: D::from(100),
                ask: D::new(10010, 2),
                observed_at_ms: 1_000_000,
            },
        },
    )?;
    apply(
        &mut journal,
        Event::Enter {
            intent_id: "demo-long".into(),
            side: Side::Long,
            stop: D::from(98),
            take_profit: D::from(104),
        },
    )?;
    let trade = journal
        .state()
        .active
        .as_ref()
        .context("demo entry missing")?;
    let qty = trade.plan.qty;
    let half = (qty / D::from(2)).round_dp_with_strategy(3, rust_decimal::RoundingStrategy::ToZero);
    let entry_price = D::new(10010, 2);
    let fill = |qty: D, price: D| Fill {
        qty,
        notional: qty * price,
        fee: qty * price * D::new(5, 4),
    };
    apply(
        &mut journal,
        Event::EntryUncertain {
            order_id: "demo-long-entry".into(),
        },
    )?;
    apply(
        &mut journal,
        Event::EntryReport {
            order_id: "demo-long-entry".into(),
            status: OrderStatus::PartiallyFilled,
            fill: fill(half, entry_price),
            reconciled: true,
        },
    )?;
    apply(
        &mut journal,
        Event::ProtectionReport {
            order_id: "demo-long-sl".into(),
            accepted: true,
        },
    )?;
    apply(
        &mut journal,
        Event::ProtectionReport {
            order_id: "demo-long-tp".into(),
            accepted: true,
        },
    )?;
    apply(
        &mut journal,
        Event::EntryReport {
            order_id: "demo-long-entry".into(),
            status: OrderStatus::Filled,
            fill: fill(qty, entry_price),
            reconciled: false,
        },
    )?;
    apply(
        &mut journal,
        Event::Quote {
            quote: Quote {
                bid: D::new(10410, 2),
                ask: D::new(10420, 2),
                observed_at_ms: 1_000_350,
            },
        },
    )?;
    apply(
        &mut journal,
        Event::ExitReport {
            order_id: "demo-long-exit".into(),
            status: OrderStatus::PartiallyFilled,
            fill: fill(half, D::new(10410, 2)),
            reconciled: false,
        },
    )?;
    apply(
        &mut journal,
        Event::ExitReport {
            order_id: "demo-long-exit".into(),
            status: OrderStatus::Filled,
            fill: fill(qty, D::new(10410, 2)),
            reconciled: false,
        },
    )?;
    apply(
        &mut journal,
        Event::CleanupConfirmed {
            intent_id: "demo-long".into(),
        },
    )?;
    let before = journal.state().clone();
    let head = journal.head_hash().to_owned();
    drop(journal);
    let mut journal = Journal::open(&path)?;
    ensure!(*journal.state() == before, "restart changed state");
    let duplicate = journal.apply(events.last().context("demo empty")?.clone())?;
    ensure!(
        duplicate.duplicate && duplicate.outcome.effects.is_empty() && journal.head_hash() == head,
        "duplicate event was not idempotent"
    );
    ensure!(
        journal.state().active.is_none() && journal.state().completed_trades == 1,
        "demo did not clean up"
    );
    let mut event_bytes = vec![];
    for event in &events {
        serde_json::to_writer(&mut event_bytes, event)?;
        event_bytes.push(b'\n');
    }
    write_new(&output.join("events.jsonl"), &event_bytes)?;
    write_new(
        &output.join("effects.json"),
        &serde_json::to_vec_pretty(&effects)?,
    )?;
    let mut report = report(&journal);
    report["restart_verified"] = json!(true);
    report["duplicate_verified"] = json!(true);
    write_new(
        &output.join("report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    let markdown = format!(
        "# Demo inti eksekusi Rust\n\n**SIMULASI SINTETIS — bukan hasil strategi atau akun Binance.**\n\n- Satu long: entry tak pasti → rekonsiliasi partial fill → TP/SL diakui → full fill → TP → partial/full exit → cleanup.\n- Qty: {qty}; net sintetis setelah fee: {} USDT. Harga/fill sengaja ditentukan fixture.\n- Restart: state identik. Pengulangan event: tidak menambah jurnal/effect.\n- Posisi akhir: kosong; transaksi selesai: 1.\n- Jurnal: {} record, SHA-256 `{}`.\n\nTidak ada koneksi jaringan atau order nyata. Efek order adalah instruksi simulasi; penerimaan/fill berasal dari fixture. Funding, margin/likuidasi dan likuiditas tidak dimodelkan.\n",
        journal.state().realized_net,
        journal.records(),
        journal.head_hash()
    );
    write_new(&output.join("report.md"), markdown.as_bytes())?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
