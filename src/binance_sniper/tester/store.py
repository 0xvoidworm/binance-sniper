"""Append-only local evidence store; reader never creates a DB or starts MT5."""
from __future__ import annotations

from datetime import datetime, timezone
from hashlib import sha256
from pathlib import Path
import json
import re
import sqlite3

from .report import canonical, digest, parse_report

DEFAULT_DB = Path(__file__).resolve().parents[3] / "state" / "tester-evidence.sqlite3"
CANDIDATE = {"id": "jimmy-range-122237", "name": "Range Breakout EA with Range Filters",
             "author": "Jimmy Peter Eriksson", "reviewed_version": "5.20", "reviewed_at": "2026-09-14",
             "url": "https://www.mql5.com/en/market/product/122237",
             "reference_broker": "IC Markets", "local_terminal_broker_observed": "FBS",
             "scope": "ORIGINAL_MT5_EA_TESTER_ONLY", "profitability": "NOT_ESTABLISHED"}
CONTEXT_FIELDS = {"engine_id", "expected_expert", "version", "purpose", "broker_timezone", "tester_model",
                  "execution_delay", "cost_model", "dataset_id", "randomizer", "run_label"}
REPEAT_FIELDS = {"version", "broker_timezone", "tester_model", "execution_delay", "cost_model", "dataset_id", "randomizer"}
ARTIFACT_TYPES = {"binary", "set", "tester_config", "tester_log"}


def context_checked(value: dict) -> dict:
    if not isinstance(value, dict) or set(value) - CONTEXT_FIELDS:
        raise ValueError("Unsupported context fields; do not include account credentials")
    result = {}
    for key in CONTEXT_FIELDS:
        item = value.get(key)
        if item is not None and (not isinstance(item, str) or not item.strip() or len(item) > 240 or any(ord(c) < 32 for c in item)):
            raise ValueError("Invalid tester context field")
        result[key] = item
    if result["engine_id"] not in {CANDIDATE["id"], "legacy-sniperfx"}:
        raise ValueError("Unknown engine identity")
    if not result["expected_expert"] or result["purpose"] not in {"COMPATIBILITY", "REPLICATION_ATTEMPT", "LEGACY_PARSER_CHECK"}:
        raise ValueError("Explicit expert identity and test purpose required")
    if result["engine_id"] == CANDIDATE["id"]:
        if result["expected_expert"] != CANDIDATE["name"] or result["purpose"] == "LEGACY_PARSER_CHECK":
            raise ValueError("Candidate identity conflicts with declared purpose/expert")
    elif result["expected_expert"] != "SniperFX" or result["purpose"] != "LEGACY_PARSER_CHECK":
        raise ValueError("Legacy evidence must remain separate from candidate")
    return result


def read_artifact(path: Path, limit: int) -> bytes:
    if not path.is_file() or path.stat().st_size > limit:
        raise ValueError("Missing or oversized local artifact")
    with path.open("rb") as file:
        raw = file.read(limit + 1)
    if len(raw) > limit:
        raise ValueError("Artifact grew during read")
    return raw


class EvidenceStore:
    def __init__(self, path: Path = DEFAULT_DB):
        self.path = Path(path)

    def import_report(self, raw: bytes, context: dict, artifacts: dict[str, bytes] | None = None) -> dict:
        context = context_checked(context)
        report = parse_report(raw)
        if report["expert"] != context["expected_expert"]:
            raise ValueError("Report expert does not match declared engine; import rejected")
        artifacts = artifacts or {}
        if set(artifacts) - ARTIFACT_TYPES or any(not isinstance(v, bytes) or not v or len(v) > 64 * 1024 * 1024 for v in artifacts.values()):
            raise ValueError("Invalid evidence artifacts")
        hashes = {key: sha256(value).hexdigest() for key, value in artifacts.items()}
        fingerprint = {key: report[key] for key in ("expert", "symbol", "timeframe", "start", "end", "currency", "leverage", "server_build", "inputs_sha256")}
        fingerprint.update(initial_deposit=report["metrics"]["initial_deposit"],
                           context={key: value for key, value in context.items() if key != "run_label"},
                           artifacts={key: value for key, value in hashes.items() if key != "tester_log"})
        missing = sorted(key for key in REPEAT_FIELDS if not context[key] or context[key].upper() in {"UNKNOWN", "UNVERIFIED"})
        missing += [key for key in ("binary", "set", "tester_config") if key not in hashes]
        missing += [key for key in ("inputs_sha256", "server_build") if not report[key]]
        identity = digest({"source": report["source_sha256"], "context": context, "artifacts": hashes})
        run = {"id": identity, "imported_at": datetime.now(timezone.utc).isoformat(), "context": context,
               "artifacts": hashes, "comparison_fingerprint": digest(fingerprint), "repeat_missing": missing,
               "outcome_sha256": digest({"metrics": report["metrics"], "deals": report["deals"]}), "report": report,
               "provenance": "LOCAL_IMPORT_WITH_OPERATOR_DECLARED_CONTEXT_NOT_ATTESTED"}
        payload = canonical(run)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with sqlite3.connect(self.path, timeout=10) as con:
            con.execute("PRAGMA journal_mode=WAL")
            con.execute("PRAGMA synchronous=FULL")
            con.execute("CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, payload TEXT NOT NULL, payload_sha256 TEXT NOT NULL, raw_report BLOB NOT NULL)")
            con.execute("CREATE TRIGGER IF NOT EXISTS immutable_update BEFORE UPDATE ON runs BEGIN SELECT RAISE(ABORT, 'immutable evidence'); END")
            con.execute("CREATE TRIGGER IF NOT EXISTS immutable_delete BEFORE DELETE ON runs BEGIN SELECT RAISE(ABORT, 'immutable evidence'); END")
            # The report and sanitized parsed data commit together. Reimport is idempotent.
            con.execute("INSERT OR IGNORE INTO runs VALUES (?, ?, ?, ?)", (identity, payload, sha256(payload.encode()).hexdigest(), raw))
        return self.get(identity)

    def _read(self, run_id: str | None = None) -> list[dict]:
        if not self.path.exists():
            return []
        with sqlite3.connect(self.path.resolve().as_uri() + "?mode=ro", uri=True, timeout=3) as con:
            query = "SELECT id, payload, payload_sha256 FROM runs"
            rows = con.execute(query + (" WHERE id=?" if run_id else " ORDER BY rowid DESC"), (run_id,) if run_id else ()).fetchall()
        result = []
        for row in rows:
            if sha256(row[1].encode()).hexdigest() != row[2]:
                raise ValueError("Evidence integrity check failed")
            item = json.loads(row[1])
            if item["id"] != row[0]:
                raise ValueError("Evidence identity mismatch")
            result.append(item)
        return result

    def get(self, run_id: str) -> dict | None:
        if not re.fullmatch(r"[a-f0-9]{64}", run_id):
            return None
        values = self._read(run_id)
        return values[0] if values else None

    def snapshot(self) -> dict:
        runs = self._read()
        summaries = []
        for run in runs:
            report = run["report"]
            summaries.append({key: run[key] for key in ("id", "imported_at", "context", "comparison_fingerprint", "repeat_missing")} | {
                "expert": report["expert"], "symbol": report["symbol"], "start": report["start"], "end": report["end"],
                "validation": report["validation"], "metrics": report["metrics"], "currency": report["currency"]})
        candidates = [r for r in summaries if r["context"]["engine_id"] == CANDIDATE["id"]]
        return {"schema_version": 1, "candidate": CANDIDATE, "status": "REPORTS_AVAILABLE" if candidates else "AWAITING_CANDIDATE_REPORT",
                "candidate_runs": len(candidates), "legacy_runs": len(summaries) - len(candidates), "runs": summaries,
                "execution": "NO_TRADING_CONTROL", "sources_separate": ["MT5_TESTER_REPORT", "BINANCE_REFERENCE_MODEL", "BINANCE_ACCOUNT_READ_ONLY"]}

    def compare(self, left_id: str, right_id: str) -> dict | None:
        left, right = self.get(left_id), self.get(right_id)
        if not left or not right:
            return None
        reasons = []
        if left_id == right_id:
            reasons.append("SAME_IMPORT_NOT_INDEPENDENT_RUNS")
        if left["comparison_fingerprint"] != right["comparison_fingerprint"]:
            reasons.append("DIFFERENT_TEST_CONTEXT_OR_ARTIFACTS")
        if left["repeat_missing"] or right["repeat_missing"]:
            reasons.append("INCOMPLETE_REPEAT_PROVENANCE")
        if any(r["report"]["errors"] for r in (left, right)):
            reasons.append("REPORT_RECONCILIATION_FAILED")
        if left["report"]["source_sha256"] == right["report"]["source_sha256"] and (not left["artifacts"].get("tester_log") or left["artifacts"].get("tester_log") == right["artifacts"].get("tester_log")):
            reasons.append("IDENTICAL_REPORT_WITHOUT_DISTINCT_RUN_LOGS")
        deltas = {key: round(right["report"]["metrics"][key] - left["report"]["metrics"][key], 8)
                  for key in ("net_profit", "total_trades", "total_deals", "initial_deposit")}
        same = left["outcome_sha256"] == right["outcome_sha256"]
        return {"status": "NOT_COMPARABLE" if reasons else ("MATCH_REPORTED_OUTPUT" if same else "DIFFERENT_REPORTED_OUTPUT"),
                "reasons": reasons, "delta_right_minus_left": deltas, "left": left_id, "right": right_id,
                "limitation": "Matching supplied artifacts/context does not attest broker history, independent execution, or future profitability."}
