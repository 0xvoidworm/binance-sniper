use binance_engine::*;
use rust_decimal::Decimal as D;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    process::Command,
};
use tempfile::tempdir;

fn config() -> Config {
    serde_json::from_str(include_str!("../fixtures/paper.json")).unwrap()
}
fn event(id: &str, at_ms: u64, event: Event) -> Envelope {
    Envelope {
        id: id.into(),
        at_ms,
        event,
    }
}
fn quote() -> Envelope {
    event(
        "quote",
        100,
        Event::Quote {
            quote: Quote {
                bid: D::from(100),
                ask: D::new(10010, 2),
                observed_at_ms: 100,
            },
        },
    )
}
fn enter() -> Envelope {
    event(
        "enter",
        101,
        Event::Enter {
            intent_id: "persist".into(),
            side: Side::Long,
            stop: D::from(98),
            take_profit: D::from(104),
        },
    )
}

#[test]
fn replay_preserves_unknown_entry_and_never_reissues_effect() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let mut journal = Journal::create(&path, config()).unwrap();
    journal.apply(quote()).unwrap();
    let original = journal.apply(enter()).unwrap();
    assert!(
        original
            .outcome
            .effects
            .iter()
            .any(|e| matches!(e, Effect::PlaceEntry { .. }))
    );
    journal
        .apply(event(
            "unknown",
            102,
            Event::EntryUncertain {
                order_id: "persist-entry".into(),
            },
        ))
        .unwrap();
    let state = journal.state().clone();
    let head = journal.head_hash().to_owned();
    drop(journal);
    let mut journal = Journal::open(&path).unwrap();
    assert_eq!(journal.state(), &state);
    assert_eq!(journal.head_hash(), head);
    let duplicate = journal.apply(enter()).unwrap();
    assert!(duplicate.duplicate && duplicate.outcome.effects.is_empty());
    assert_eq!(journal.records(), 4);
    assert!(journal.state().active.as_ref().unwrap().entry.uncertain);
    let qty = journal.state().active.as_ref().unwrap().plan.qty;
    let result = journal
        .apply(event(
            "found",
            103,
            Event::EntryReport {
                order_id: "persist-entry".into(),
                status: OrderStatus::Filled,
                fill: Fill {
                    qty,
                    notional: qty * D::new(1001, 1),
                    fee: D::ZERO,
                },
                reconciled: true,
            },
        ))
        .unwrap();
    assert!(result.outcome.accepted);
    assert_eq!(
        result
            .outcome
            .effects
            .iter()
            .filter(|e| matches!(e, Effect::PlaceProtection { .. }))
            .count(),
        2
    );
}

#[test]
fn identical_retry_is_noop_but_id_collision_is_an_error() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let mut journal = Journal::create(&path, config()).unwrap();
    journal.apply(quote()).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(journal.apply(quote()).unwrap().duplicate);
    let mut collision = quote();
    collision.at_ms += 1;
    assert!(journal.apply(collision).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn rejected_events_are_durable_and_replay_without_state_change() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let mut journal = Journal::create(&path, config()).unwrap();
    let before = journal.state().clone();
    let outcome = journal.apply(enter()).unwrap();
    assert!(!outcome.outcome.accepted);
    assert_eq!(journal.state(), &before);
    drop(journal);
    let mut journal = Journal::open(&path).unwrap();
    assert_eq!(journal.state(), &before);
    assert_eq!(journal.records(), 2);
    let retry = journal.apply(enter()).unwrap();
    assert!(retry.duplicate && !retry.outcome.accepted);
}

#[test]
fn single_writer_lock_is_enforced_in_same_and_other_process() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let journal = Journal::create(&path, config()).unwrap();
    assert!(Journal::open(&path).is_err());
    let output = Command::new(env!("CARGO_BIN_EXE_binance-engine"))
        .arg("status")
        .arg("--journal")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("locked"));
    drop(journal);
    assert!(Journal::open(&path).is_ok());
}

#[test]
fn create_never_overwrites_an_existing_session() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    drop(Journal::create(&path, config()).unwrap());
    let bytes = fs::read(&path).unwrap();
    assert!(Journal::create(&path, config()).is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn torn_append_is_rejected_and_bytes_are_retained_for_explicit_recovery() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    drop(Journal::create(&path, config()).unwrap());
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"{\"body\":")
        .unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(Journal::open(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn checksum_modification_and_missing_middle_record_are_rejected() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let mut journal = Journal::create(&path, config()).unwrap();
    journal.apply(quote()).unwrap();
    journal.apply(enter()).unwrap();
    drop(journal);
    let original = fs::read_to_string(&path).unwrap();
    fs::write(&path, original.replace("SYNTHUSDT", "OTHERUSDT")).unwrap();
    assert!(Journal::open(&path).is_err());
    let lines: Vec<_> = original.lines().collect();
    fs::write(&path, format!("{}\n{}\n", lines[0], lines[2])).unwrap();
    assert!(Journal::open(&path).is_err());
}

#[test]
fn valid_checksum_with_changed_replay_outcome_is_rejected() {
    use sha2::{Digest, Sha256};
    // Checksums alone are insufficient: the replayed outcome/state must agree.
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let mut journal = Journal::create(&path, config()).unwrap();
    journal.apply(quote()).unwrap();
    drop(journal);
    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = text.lines().collect();
    let body = lines[1]
        .strip_prefix("{\"body\":")
        .unwrap()
        .rsplit_once(",\"hash\":")
        .unwrap()
        .0;
    let changed = body.replace("\"accepted\":true", "\"accepted\":false");
    assert_ne!(body, changed);
    let digest: String = Sha256::digest(changed.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    fs::write(
        &path,
        format!(
            "{}\n{{\"body\":{},\"hash\":\"{}\"}}\n",
            lines[0], changed, digest
        ),
    )
    .unwrap();
    let error = Journal::open(&path).err().unwrap().to_string();
    assert!(error.contains("outcome diverged"), "{error}");
}

#[test]
fn oversized_journal_is_rejected_before_reading_contents() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("journal.jsonl");
    let file = fs::File::create(&path).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    assert!(Journal::open(&path).is_err());
}

#[test]
fn cli_demo_reproduces_byte_for_byte_and_all_prefixes_restart() {
    let temp = tempdir().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    for dir in [&first, &second] {
        let output = Command::new(env!("CARGO_BIN_EXE_binance-engine"))
            .arg("demo")
            .arg("--output")
            .arg(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for file in [
        "journal.jsonl",
        "events.jsonl",
        "effects.json",
        "report.json",
        "report.md",
    ] {
        assert_eq!(
            fs::read(first.join(file)).unwrap(),
            fs::read(second.join(file)).unwrap()
        );
    }
    let events: Vec<Envelope> = fs::read_to_string(first.join("events.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let path = temp.path().join("restarted.jsonl");
    let mut journal = Journal::create(&path, config()).unwrap();
    for event in events {
        assert!(journal.apply(event.clone()).unwrap().outcome.accepted);
        let state = journal.state().clone();
        let head = journal.head_hash().to_owned();
        drop(journal);
        journal = Journal::open(&path).unwrap();
        assert_eq!(journal.state(), &state);
        assert_eq!(journal.head_hash(), head);
        assert!(journal.apply(event).unwrap().outcome.effects.is_empty());
    }
    assert_eq!(
        fs::read(&path).unwrap(),
        fs::read(first.join("journal.jsonl")).unwrap()
    );
    let status = Command::new(env!("CARGO_BIN_EXE_binance-engine"))
        .arg("status")
        .arg("--journal")
        .arg(&path)
        .output()
        .unwrap();
    drop(journal);
    assert!(!status.status.success()); // A second process cannot inspect a live writer.
    let status = Command::new(env!("CARGO_BIN_EXE_binance-engine"))
        .arg("status")
        .arg("--journal")
        .arg(&path)
        .output()
        .unwrap();
    assert!(status.status.success());
    let report: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(report["state"]["completed_trades"], 1);
    assert!(report["state"]["active"].is_null());
}

#[test]
fn cli_init_apply_duplicate_and_validate_config() {
    let temp = tempdir().unwrap();
    let conf = temp.path().join("config.json");
    let path = temp.path().join("journal.jsonl");
    let events = temp.path().join("events.jsonl");
    fs::write(&conf, include_bytes!("../fixtures/paper.json")).unwrap();
    let bin = env!("CARGO_BIN_EXE_binance-engine");
    assert!(
        Command::new(bin)
            .arg("validate-config")
            .arg("--config")
            .arg(&conf)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        Command::new(bin)
            .arg("init")
            .arg("--config")
            .arg(&conf)
            .arg("--journal")
            .arg(&path)
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(
        &events,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&quote()).unwrap(),
            serde_json::to_string(&enter()).unwrap()
        ),
    )
    .unwrap();
    for _ in 0..2 {
        assert!(
            Command::new(bin)
                .arg("apply")
                .arg("--journal")
                .arg(&path)
                .arg("--events")
                .arg(&events)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    let journal = Journal::open(&path).unwrap();
    assert_eq!(journal.records(), 3);
}
