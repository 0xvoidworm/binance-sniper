use super::super::{
    margin::Budget,
    session_store::{self, Config, Store, TradeArchive, digest},
};
use super::{package, proposal, schema::*, source};
use rust_decimal::Decimal as D;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
};

fn d(s: &str) -> D {
    s.parse().unwrap()
}

#[test]
fn r10_recovery_export_checks_final_metadata_and_old_versions_remain_readable() {
    use super::super::{
        diagnostics::{Build, Diagnostics, Gate},
        recovery,
    };
    for build in [
        Build::V069,
        Build::V0610,
        Build::V0611,
        Build::V0612,
        Build::V0613,
    ] {
        let (root, _) = fixture();
        save_state(root.path(), |state| {
            let s = state.session.as_mut().unwrap();
            let mut d = Diagnostics {
                build,
                ..Default::default()
            };
            d.record(Gate::Signal);
            d.record(Gate::Intent);
            s.diagnostics = Some(d);
            s.recovery = (build != Build::V069).then_some(recovery::State {
                episodes: 2,
                active: false,
            });
        });
        let out = output();
        let path = out.path().join("packet");
        package::publish(&path, &source::snapshot(root.path()).unwrap()).unwrap();
        let valid = package::validate(&path).unwrap();
        assert_eq!(
            valid.manifest.source_build_version.as_deref(),
            Some(build.label())
        );
        assert_eq!(valid.summary.recovery.is_some(), build != Build::V069);
        let mut manifest = valid.manifest;
        manifest.exporter_version = "0.6.10".into();
        write(
            &path.join("manifest.json"),
            &serde_json::to_vec(&manifest).unwrap(),
        );
        assert_eq!(
            package::validate(&path).is_ok(),
            matches!(build, Build::V069 | Build::V0610)
        );
        manifest.exporter_version = "0.6.9".into();
        write(
            &path.join("manifest.json"),
            &serde_json::to_vec(&manifest).unwrap(),
        );
        assert_eq!(package::validate(&path).is_ok(), build == Build::V069);
        if build != Build::V069 {
            for recovery in [
                recovery::State {
                    episodes: 3,
                    active: false,
                },
                recovery::State {
                    episodes: 1,
                    active: true,
                },
            ] {
                let mut summary = valid.summary.clone();
                summary.recovery = Some(recovery);
                assert!(validate_content(&summary, &valid.trades).is_err());
            }
        }
    }

    let (root, _) = fixture();
    save_state(root.path(), |state| {
        let mut diagnostics = Diagnostics::default();
        diagnostics.record(Gate::Signal);
        diagnostics.record(Gate::Intent);
        state.session.as_mut().unwrap().diagnostics = Some(diagnostics);
    });
    let out = output();
    let path = out.path().join("packet-current");
    package::publish(&path, &source::snapshot(root.path()).unwrap()).unwrap();
    let mut manifest = package::validate(&path).unwrap().manifest;
    manifest.exporter_version = "0.6.13".into();
    write(
        &path.join("manifest.json"),
        &serde_json::to_vec(&manifest).unwrap(),
    );
    assert!(
        package::validate(&path).is_err(),
        "an older exporter cannot claim 0.6.14 diagnostics"
    );
}

#[test]
fn p3_legacy_packet_version_remains_readable_without_fabricated_diagnostics() {
    let (_source, _out, path) = exported();
    let file = path.join("manifest.json");
    let mut m: Manifest = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    m.exporter_version = "0.6.8".into();
    write(&file, &serde_json::to_vec(&m).unwrap());
    let packet = package::validate(&path).unwrap();
    assert!(packet.summary.diagnostics.is_none());
    assert!(packet.manifest.source_build_version.is_none());
    assert!(
        packet
            .manifest
            .missing_data
            .contains(&Missing::SourceBuildVersion)
    );
    m.exporter_version = "99.0.0".into();
    write(&file, &serde_json::to_vec(&m).unwrap());
    assert!(package::validate(&path).is_err());
}

#[test]
fn p3_counter_export_is_typed_and_linked_to_source_build_without_claiming_full_history() {
    use super::super::diagnostics::{Diagnostics, Gate};
    let (source, _) = fixture();
    save_state(source.path(), |state| {
        let mut d = Diagnostics::default();
        for gate in [
            Gate::Volatility,
            Gate::Signal,
            Gate::Intent,
            Gate::DataFailure,
        ] {
            d.record(gate);
        }
        state.session.as_mut().unwrap().diagnostics = Some(d);
    });
    let out = output();
    let path = out.path().join("packet");
    package::publish(&path, &source::snapshot(source.path()).unwrap()).unwrap();
    let valid = package::validate(&path).unwrap();
    assert_eq!(
        valid.manifest.source_build_version.as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert!(
        !valid
            .manifest
            .missing_data
            .contains(&Missing::SourceBuildVersion)
    );
    assert!(
        valid
            .manifest
            .missing_data
            .contains(&Missing::FullDecisionHistory)
    );
    assert!(
        valid
            .manifest
            .missing_data
            .contains(&Missing::HealthTimeSeries)
    );
    assert_eq!(
        valid
            .summary
            .diagnostics
            .as_ref()
            .unwrap()
            .evaluated_candles(),
        2
    );
    let mut summary = valid.summary;
    summary.diagnostics.as_mut().unwrap().counts[Gate::Intent as usize] = 2;
    repack(
        &path,
        "summary.json",
        &serde_json::to_vec(&summary).unwrap(),
    );
    assert!(package::validate(&path).is_err());
}
fn write(path: &Path, bytes: &[u8]) {
    let mut f = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
}
fn save_state(root: &Path, modify: impl FnOnce(&mut session_store::State)) {
    let mut state = session_store::read_saved(root).unwrap().unwrap();
    modify(&mut state);
    let hash = digest(&serde_json::to_vec(&state).unwrap());
    write(
        &root.join("session.json"),
        &serde_json::to_vec(&json!({"schema":1,"state":state,"hash":hash})).unwrap(),
    );
}
fn config() -> Config {
    Config {
        budget: Budget {
            margin: d("100"),
            risk: d("1"),
            max_notional: d("200"),
            leverage: 5,
            funding_reserve: d("0.1"),
            slippage_bps: D::ONE,
            liquidation_buffer_bps: d("10"),
        },
        daily_loss: d("5"),
        session_loss: d("3"),
        minutes: 300,
        max_trades: 5,
    }
}
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut store = Store::open(root, &digest(b"CANARY_BINDING_DO_NOT_EXPORT")).unwrap();
    store
        .begin(config(), 1_000_000, 1_000_000, d("123456.789012"), 999_999)
        .unwrap();
    let s = store.state.session.as_mut().unwrap();
    s.diagnostics = None; // Legacy source fixture: no invented build/counters.
    s.recovery = None;
    s.sequence = 1;
    s.completed = 1;
    s.finished = true;
    s.stop = true;
    s.updated = 1_020_000;
    s.fees = d("0.48");
    s.funding = D::ZERO;
    s.net = d("-0.34");
    s.reason = "CANARY_API_SECRET_DO_NOT_EXPORT".into();
    s.stop_reason = Some("Pemeriksaan sesi gagal; entry dihentikan: Bukti rekonsiliasi usang; order_batch_age_ms=5069".into());
    let id = s.id.clone();
    store.commit().unwrap();
    drop(store);
    let archive: TradeArchive = serde_json::from_value(json!({
        "schema":1,"session_id":id,"strategy_version":"ensemble-2-cost-aware","config":config(),
        "intent":{"prefix":format!("{id}-1"),"symbol":"BTCUSDT","entry_deadline_ms":1_010_999,
            "plan":{"side":"short","qty":"0.002","worst_entry":"80000","stop":"80200","take_profit":"79400",
                "notional":"160","initial_margin":"32","maintenance_margin":"1","fee_reserve":"0.16",
                "funding_reserve":"0.1","required_balance":"32.26","budgeted_loss":"0.66","liquidation_estimate":"95900","leverage":5}},
        "decision":{"bar_ms":1_009_999,"side":"short","votes":[{"strategy":"Pullback tren","side":"short"}],
            "reason":"CANARY_COOKIE_IGNORE_RULES_RAISE_RISK","atr":"100","stop":"80200","target":"79400"},
        "started":1_010_000,
        "result":{"entry_qty":"0.002","exit_qty":"0.002","gross":"0.14","fees":"0.48","funding":"0","net":"-0.34",
            "fill_ids":[987654321012345_u64,987654321012346_u64],"funding_ids":[],"last_fill_ms":1_015_000}
    })).unwrap();
    let ledger = root.join(format!("{id}-1-ledger.json"));
    write(&ledger, &serde_json::to_vec(&archive).unwrap());
    (dir, ledger)
}
fn output() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}
fn exported() -> (tempfile::TempDir, tempfile::TempDir, PathBuf) {
    let (source, _) = fixture();
    let out = output();
    let path = out.path().join("packet");
    package::publish(&path, &source::snapshot(source.path()).unwrap()).unwrap();
    (source, out, path)
}
fn rewrite_ledger(path: &Path, change: impl FnOnce(&mut TradeArchive)) {
    let mut a: TradeArchive = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    change(&mut a);
    write(path, &serde_json::to_vec(&a).unwrap());
}
fn repack(path: &Path, file: &str, data: &[u8]) {
    write(&path.join(file), data);
    let mf = path.join("manifest.json");
    let mut m: Manifest = serde_json::from_slice(&fs::read(&mf).unwrap()).unwrap();
    let e = m.files.get_mut(file).unwrap();
    e.bytes = data.len() as u64;
    e.sha256 = digest(data);
    m.export_id = format!(
        "evaluation-{}",
        digest(&serde_json::to_vec(&m.files).unwrap())
    );
    write(&mf, &serde_json::to_vec(&m).unwrap());
}

#[test]
fn p2_exact_export_redacts_identifiers_free_text_wallet_and_leaves_source_unchanged() {
    let (root, ledger) = fixture();
    let state_before = fs::read(root.path().join("session.json")).unwrap();
    let ledger_before = fs::read(&ledger).unwrap();
    let packet = source::snapshot(root.path()).unwrap();
    assert_eq!(packet.summary.net.0, d("-0.34"));
    assert_eq!(packet.summary.funding_actual.0, D::ZERO);
    assert_eq!(packet.summary.economics.funding_reserve.0, d("0.1"));
    assert_eq!(packet.summary.stop.order_batch_age_ms, Some(5069));
    let out = output();
    let path = out.path().join("safe");
    package::publish(&path, &packet).unwrap();
    let validated = package::validate(&path).unwrap();
    assert_eq!(validated.trades[0].trade_id, "trade-001");
    assert!(
        validated
            .manifest
            .missing_data
            .contains(&Missing::IntratradePricePath)
    );
    assert!(validated.trades[0].exit_decision.is_none());
    assert!(validated.trades[0].exit_outcome.is_none());
    let id = session_store::read_saved(root.path())
        .unwrap()
        .unwrap()
        .session
        .unwrap()
        .id;
    for item in fs::read_dir(&path).unwrap() {
        let item = item.unwrap();
        let bytes = fs::read(item.path()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        for forbidden in [
            "CANARY_",
            "123456.789012",
            "987654321012345",
            "wallet",
            "binding",
            "client_id",
            "fill_ids",
            &id,
        ] {
            assert!(!text.contains(forbidden), "forbidden field escaped");
        }
        assert_eq!(item.metadata().unwrap().permissions().mode() & 0o077, 0);
    }
    assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o077, 0);
    assert_eq!(
        fs::read(root.path().join("session.json")).unwrap(),
        state_before
    );
    assert_eq!(fs::read(ledger).unwrap(), ledger_before);
}

#[test]
fn p2_source_lock_active_session_and_bad_state_hash_are_rejected_without_writes() {
    let (root, _) = fixture();
    let state_before = fs::read(root.path().join("session.json")).unwrap();
    let lock = fs::File::open(root.path().join("session.lock")).unwrap();
    lock.try_lock().unwrap();
    assert!(source::snapshot(root.path()).is_err());
    assert_eq!(
        fs::read(root.path().join("session.json")).unwrap(),
        state_before
    );
    drop(lock);
    save_state(root.path(), |state| {
        state.session.as_mut().unwrap().finished = false
    });
    assert!(source::snapshot(root.path()).is_err());
    save_state(root.path(), |state| {
        state.session.as_mut().unwrap().finished = true
    });
    let mut v: Value =
        serde_json::from_slice(&fs::read(root.path().join("session.json")).unwrap()).unwrap();
    v["hash"] = json!("0".repeat(64));
    write(
        &root.path().join("session.json"),
        &serde_json::to_vec(&v).unwrap(),
    );
    assert!(source::snapshot(root.path()).is_err());
}

#[test]
fn p2_missing_archive_cross_session_and_duplicate_fill_rejected() {
    let (root, ledger) = fixture();
    let bytes = fs::read(&ledger).unwrap();
    rewrite_ledger(&ledger, |a| a.result.fill_ids[1] = a.result.fill_ids[0]);
    assert!(source::snapshot(root.path()).is_err());
    write(&ledger, &bytes);
    rewrite_ledger(&ledger, |a| a.session_id = "foreign-session".into());
    assert!(source::snapshot(root.path()).is_err());
    write(&ledger, &bytes);
    fs::rename(&ledger, ledger.with_extension("held")).unwrap();
    assert!(source::snapshot(root.path()).is_err());
}

#[test]
fn p2_accounting_mismatch_and_unknown_strategy_fail_closed() {
    let (root, ledger) = fixture();
    rewrite_ledger(&ledger, |a| a.result.net += D::ONE);
    assert!(source::snapshot(root.path()).is_err());
    let (root, _) = fixture();
    save_state(root.path(), |state| {
        state.session.as_mut().unwrap().fees += D::ONE
    });
    assert!(source::snapshot(root.path()).is_err());
    let (root, ledger) = fixture();
    rewrite_ledger(&ledger, |a| {
        a.decision.votes[0].strategy = "CANARY_EXECUTE_SCRIPT".into()
    });
    let error = source::snapshot(root.path()).err().unwrap().to_string();
    assert!(!error.contains("CANARY"));
}

#[test]
fn p2_symlinks_hardlinks_permissions_and_traversal_refused() {
    let (root, ledger) = fixture();
    let out = output();
    let link = out.path().join("source");
    symlink(root.path(), &link).unwrap();
    assert!(source::snapshot(&link).is_err());
    let saved = ledger.with_extension("held");
    fs::rename(&ledger, &saved).unwrap();
    symlink(&saved, &ledger).unwrap();
    assert!(source::snapshot(root.path()).is_err());
    let (root, ledger) = fixture();
    let hard = out.path().join("hard");
    fs::hard_link(&ledger, hard).unwrap();
    assert!(source::snapshot(root.path()).is_err());
    let (_, _out, path) = exported();
    fs::set_permissions(path.join("summary.json"), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(package::validate(&path).is_err());
    assert!(package::validate(&path.join("..").join("packet")).is_err());
}

#[test]
fn p2_partial_package_extra_file_corruption_no_overwrite_and_limits() {
    let (_, _out, path) = exported();
    let before = fs::read(path.join("manifest.json")).unwrap();
    let (root, _) = fixture();
    assert!(package::publish(&path, &source::snapshot(root.path()).unwrap()).is_err());
    assert_eq!(fs::read(path.join("manifest.json")).unwrap(), before);
    write(&path.join("unexpected.env"), b"SYNTHETIC_CANARY");
    assert!(package::validate(&path).is_err());
    let (_, _out2, path2) = exported();
    fs::rename(path2.join("manifest.json"), path2.join("manifest.pending")).unwrap();
    assert!(package::validate(&path2).is_err());
    let (_, _out3, path3) = exported();
    write(&path3.join("summary.json"), b"{}");
    assert!(package::validate(&path3).is_err());
    let (_, _out4, path4) = exported();
    let f = fs::OpenOptions::new()
        .write(true)
        .open(path4.join("summary.json"))
        .unwrap();
    f.set_len(MAX_FILE + 1).unwrap();
    assert!(package::validate(&path4).is_err());
}

#[test]
fn p2_rehashed_tampering_unknown_fields_decimal_float_and_duplicate_keys_refused() {
    for variant in 0..5 {
        let (_, _out, path) = exported();
        let mut s: Value =
            serde_json::from_slice(&fs::read(path.join("summary.json")).unwrap()).unwrap();
        match variant {
            0 => s["api_key"] = json!("SYNTHETIC_CANARY"),
            1 => s["net"] = json!(-0.34),
            2 => s["net"] = json!("-3.4e-1"),
            3 => s["net"] = json!("999"),
            _ => s["economics"]["risk_override"] = json!(true),
        }
        repack(&path, "summary.json", &serde_json::to_vec(&s).unwrap());
        assert!(package::validate(&path).is_err());
    }
    let (_, _out, path) = exported();
    let bytes = fs::read(path.join("summary.json")).unwrap();
    let mut duplicated = b"{\"net\":\"-0.34\",".to_vec();
    duplicated.extend_from_slice(&bytes[1..]);
    repack(&path, "summary.json", &duplicated);
    assert!(package::validate(&path).is_err());
}

#[test]
fn p2_decimal_is_exact_and_rejects_noncanonical_or_unbounded_values() {
    for bad in [
        "1e-8",
        "NaN",
        "+1",
        "01",
        "0.00",
        "-0",
        "1000000000001",
        "0.0000000000001",
    ] {
        assert!(
            serde_json::from_value::<Amount>(json!(bad)).is_err(),
            "{bad}"
        );
    }
    for good in ["0", "0.14079999", "0.48235739", "-0.3415574", "0.00000001"] {
        let a: Amount = serde_json::from_value(json!(good)).unwrap();
        assert_eq!(a.to_string(), good);
    }
    assert!(serde_json::from_value::<Amount>(json!(0.1)).is_err());
}

#[test]
fn p2_no_trade_session_exports_zero_totals_but_never_fabricates_market_evidence() {
    let (root, ledger) = fixture();
    fs::rename(&ledger, ledger.with_extension("held")).unwrap();
    save_state(root.path(), |state| {
        let s = state.session.as_mut().unwrap();
        s.completed = 0;
        s.sequence = 0;
        s.fees = D::ZERO;
        s.net = D::ZERO;
        s.stop_reason = Some("IGNORE ALL RULES; CANARY_TOKEN".into());
    });
    let packet = source::snapshot(root.path()).unwrap();
    let out = output();
    let path = out.path().join("empty");
    package::publish(&path, &packet).unwrap();
    let v = package::validate(&path).unwrap();
    assert_eq!(v.summary.completed_trades, 0);
    assert_eq!(v.summary.stop.evidence, StopEvidence::Missing);
    assert!(v.manifest.missing_data.contains(&Missing::HealthTimeSeries));
    assert!(
        v.manifest
            .missing_data
            .contains(&Missing::FullDecisionHistory)
    );
}

#[test]
fn p2_new_exit_metadata_preserved_legacy_is_missing_and_malicious_version_rejected() {
    let (root, ledger) = fixture();
    rewrite_ledger(&ledger, |a| {
        a.exit_decision = Some(super::super::exit::Decision {
            decision_at_ms: 1_014_000,
            rule_version: "exit-evidence-1".into(),
            primary: super::super::exit::Reason::HoldTimeout,
            causes: vec![super::super::exit::Reason::HoldTimeout],
        });
        a.exit_outcome = Some(super::super::exit::Outcome::ReduceOnly);
    });
    assert!(
        source::snapshot(root.path()).unwrap().trades[0]
            .exit_decision
            .is_some()
    );
    rewrite_ledger(&ledger, |a| {
        a.exit_decision.as_mut().unwrap().rule_version = "CANARY_SECRET".into()
    });
    assert!(source::snapshot(root.path()).is_err());
}

#[test]
fn p2_reexport_same_content_has_same_export_id_but_no_original_account_identifier() {
    let (root, _) = fixture();
    let out = output();
    let packet = source::snapshot(root.path()).unwrap();
    package::publish(&out.path().join("a"), &packet).unwrap();
    package::publish(&out.path().join("b"), &packet).unwrap();
    let a = package::validate(&out.path().join("a")).unwrap();
    let b = package::validate(&out.path().join("b")).unwrap();
    assert_eq!(a.manifest.export_id, b.manifest.export_id);
    assert_eq!(a.summary, b.summary);
}

#[test]
fn p2_each_injected_write_failure_leaves_no_accepted_manifest_or_source_changes() {
    let (root, ledger) = fixture();
    let before = fs::read(&ledger).unwrap();
    let packet = source::snapshot(root.path()).unwrap();
    for fail_at in 1..=5 {
        let out = output();
        let path = out.path().join("interrupted");
        let mut count = 0;
        let result = package::publish_with(&path, &packet, |file, bytes| {
            count += 1;
            if count == fail_at {
                // Mid-file failure, e.g. ENOSPC; never publish its manifest.
                write(file, &bytes[..bytes.len() / 2]);
                anyhow::bail!("injected storage failure");
            }
            write(file, bytes);
            Ok(())
        });
        assert!(result.is_err());
        assert!(!path.join("manifest.json").exists());
        assert!(package::validate(&path).is_err());
        assert_eq!(fs::read(&ledger).unwrap(), before);
    }
}

#[test]
fn p2_rehashed_canonical_semantic_tampering_and_forged_completeness_rejected() {
    for variant in 0..4 {
        let (_, _out, path) = exported();
        let mut s: Summary =
            serde_json::from_slice(&fs::read(path.join("summary.json")).unwrap()).unwrap();
        match variant {
            0 => s.net.0 += D::ONE,
            1 => s.actual_duration_ms += 1,
            2 => s.completed_trades += 1,
            _ => s.stop.evidence = StopEvidence::StructuredState,
        }
        repack(&path, "summary.json", &serde_json::to_vec(&s).unwrap());
        assert!(package::validate(&path).is_err());
    }
    let (_, _out, path) = exported();
    let file = path.join("manifest.json");
    let mut m: Manifest = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    m.missing_data.clear();
    write(&file, &serde_json::to_vec(&m).unwrap());
    assert!(package::validate(&path).is_err());
    m.schema_version = 999;
    write(&file, &serde_json::to_vec(&m).unwrap());
    assert!(package::validate(&path).is_err());
}

#[test]
fn p2_manifest_path_and_package_symlink_and_duplicate_records_refused() {
    let (_, out, path) = exported();
    let file = path.join("manifest.json");
    let mut m: Manifest = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    let entry = m.files.remove("summary.json").unwrap();
    m.files.insert("../session.json".into(), entry);
    write(&file, &serde_json::to_vec(&m).unwrap());
    assert!(package::validate(&path).is_err());
    let (_, _out2, path2) = exported();
    let summary = path2.join("summary.json");
    let held = out.path().join("held.json");
    fs::rename(&summary, &held).unwrap();
    symlink(&held, &summary).unwrap();
    assert!(package::validate(&path2).is_err());
    let (_, _out3, path3) = exported();
    let rows = fs::read(path3.join("trades.jsonl")).unwrap();
    let twice = [rows.clone(), rows].concat();
    repack(&path3, "trades.jsonl", &twice);
    let mf = path3.join("manifest.json");
    let mut m: Manifest = serde_json::from_slice(&fs::read(&mf).unwrap()).unwrap();
    m.files.get_mut("trades.jsonl").unwrap().records = 2;
    m.export_id = format!(
        "evaluation-{}",
        digest(&serde_json::to_vec(&m.files).unwrap())
    );
    write(&mf, &serde_json::to_vec(&m).unwrap());
    assert!(package::validate(&path3).is_err());
}

fn valid_proposal(p: &Validated) -> Value {
    json!({"schema_version":1,"proposal_id":"fixture-001","export_digest":p.manifest_digest,
        "evaluator_version":"fixture-1","model":"offline-fixture","prompt_sha256":digest(b"fixture"),
        "question":"Is the evidence sufficient?","facts":[
            {"file":"summary","record":null,"field":"net","value":"-0.34"},
            {"file":"trades","record":1,"field":"trade_id","value":"trade-001"}],
        "inferences":["One trade cannot establish an edge."],"missing_data":p.manifest.missing_data,
        "hypothesis":"More observations are needed.","candidate_change":"collect_missing_observations",
        "invariants":["no_live_orders","no_risk_increase","no_auto_apply","no_profit_or24h_claim"],
        "test_protocol":"Freeze a chronological holdout before evaluation.","pass_fail_criteria":"Insufficient data unless coverage is verified.",
        "usage":{"calls":0,"input_tokens":0,"output_tokens":0},"status":"insufficient_data"})
}
#[test]
fn p2_proposal_reference_validation_rejects_fiction_risk_code_and_missing_flags() {
    let (_, out, path) = exported();
    let p = package::validate(&path).unwrap();
    let file = out.path().join("proposal.json");
    let valid = valid_proposal(&p);
    write(&file, &serde_json::to_vec_pretty(&valid).unwrap());
    proposal::validate(&p, &file).unwrap();
    for variant in 0..10 {
        let mut v = valid.clone();
        match variant {
            0 => v["facts"][0]["value"] = json!("999"),
            1 => v["facts"][1]["record"] = json!(2),
            2 => v["facts"][0]["field"] = json!("wallet"),
            3 => v["candidate_change"] = json!({"leverage":125}),
            4 => v["execute"] = json!("sh command"),
            5 => v["export_digest"] = json!("0".repeat(64)),
            6 => v["usage"]["calls"] = json!(7),
            7 => v["missing_data"] = json!([]),
            8 => v["status"] = json!("accepted"),
            _ => v["facts"][0]["file"] = json!("../../live/session.json"),
        }
        write(&file, &serde_json::to_vec(&v).unwrap());
        assert!(proposal::validate(&p, &file).is_err(), "{variant}");
    }
}
