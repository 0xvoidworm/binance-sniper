"""Immutable offline trials, fixed selection and optional single final evaluation."""
from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import asdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import random
import shutil

from binance_sniper.domain import Candle
from binance_sniper.research.store import digest, encoded
from binance_sniper.round01.data import DAY, H4, SYMBOLS, sha, utc, write_json
from binance_sniper.round01.engine import CONFIGS, COSTS, FUNDING_POLICY_VERSION, ChannelEngine, project_frame

EXPECTED_PROTOCOL = "b140599978af02c40cc9b74600f7672ca1b335c82fd4dd634b6435f897b96d70"
EXPECTED_DATA = "13582bb5aa781e94117b28ee1bc943a595fc7cc1ee1d7393ec56cf4710b57676"
PARTITIONS = {"DEVELOPMENT": ("2025-06-20", "2025-12-20"), "SELECTION": ("2025-12-20", "2026-06-15"),
              "FINAL_EXPLORATORY": ("2026-06-15", "2026-09-13")}


def millis(date):
    return int(datetime.fromisoformat(date).replace(tzinfo=timezone.utc).timestamp() * 1000)


def percentile(values, q):
    values = sorted(values)
    at = (len(values) - 1) * q
    lo, hi = int(at), min(int(at) + 1, len(values) - 1)
    return values[lo] + (values[hi] - values[lo]) * (at - lo)


def bootstrap_lower(daily):
    blocks = {}
    for d in daily:
        week = (d["day"] + 3 * DAY) // (7 * DAY)  # Monday-aligned calendar weeks
        blocks.setdefault(week, []).append(d["net_change"])
    values = list(blocks.values())
    if not values:
        return None
    rng = random.Random(14092026)
    means = []
    for _ in range(1000):
        sample = []
        while len(sample) < len(daily):
            sample.extend(rng.choice(values))
        means.append(sum(sample[:len(daily)]) / len(daily))
    return percentile(means, .05)


def trade_metrics(trades):
    profits = sorted((t["net"] for t in trades if t["net"] > 0), reverse=True)
    losses = -sum(t["net"] for t in trades if t["net"] < 0)
    return {"trade_count": len(trades), **{k: sum(t[k] for t in trades) for k in ("gross", "slippage_cost", "fee_cost", "funding_cost", "net")},
            "gross_profit_net_trades": sum(profits), "gross_loss_net_trades": losses,
            "pf": sum(profits) / losses if losses else None,
            "top3_positive_share": sum(profits[:3]) / sum(profits) if profits else None,
            "mean_net_r": sum(t["net_r"] for t in trades) / len(trades) if trades else None,
            "win_rate": len(profits) / len(trades) if trades else None,
            "partition_end_exits": sum(t["exit_reason"] == "PARTITION_END" for t in trades),
            "entry_bar_stops": sum(t["entry_bar_stop"] for t in trades),
            "gap_stops": sum(t["exit_reason"] == "GAP_STOP" for t in trades)}


def summarize(trades, equity, funding, start, end):
    result = trade_metrics(trades)
    result["per_symbol"] = {s: trade_metrics([t for t in trades if t["symbol"] == s]) for s in SYMBOLS}
    peak, dd = 1.0, 0.0
    daily_ends = {}
    for point in equity:
        peak = max(peak, point["equity_index"])
        dd = max(dd, (peak - point["equity_index"]) / peak)
        daily_ends[(point["at"] - 1) // DAY * DAY] = point["equity_index"]
    daily, previous = [], 1.0
    entries = Counter(t["entry_at"] // DAY * DAY for t in trades)
    for day in range(start, end, DAY):
        current = daily_ends[day]
        daily.append({"day": day, "utc": utc(day), "net_change": current - previous, "entries": entries[day]})
        previous = current
    block = len(daily) // 3
    blocks = [daily[:block], daily[block:2 * block], daily[2 * block:]]
    result.update({"max_mtm_drawdown_fraction": dd, "final_equity_index": equity[-1]["equity_index"],
        "calendar_days": len(daily), "trade_per_day": len(trades) / len(daily),
        "days_without_entry": sum(d["entries"] == 0 for d in daily),
        "subperiod_net": [sum(d["net_change"] for d in b) for b in blocks],
        "bootstrap_daily_mean_lower5": bootstrap_lower(daily), "bootstrap_seed": 14092026,
        "funding_event_count": len(funding), "ambiguous_funding_count": sum(f["ambiguous"] for f in funding),
        "cost_complete_for_model": True, "account_model_complete": False})
    # Reconciliation uses independently accumulated mark-to-market and fee events.
    if abs(result["gross"] - result["slippage_cost"] - result["fee_cost"] - result["funding_cost"] - result["net"]) > 1e-10:
        raise AssertionError("TRADE_COMPONENT_RECONCILIATION")
    if abs(sum(f["funding_cost"] for f in funding) - result["funding_cost"]) > 1e-10:
        raise AssertionError("FUNDING_RECONCILIATION")
    if abs(equity[-1]["equity_index"] - 1 - result["net"]) > 1e-10:
        raise AssertionError("EQUITY_LEDGER_RECONCILIATION")
    if abs(sum(d["net_change"] for d in daily) - result["net"]) > 1e-10:
        raise AssertionError("DAILY_RECONCILIATION")
    return result, daily


def verify_input(input_path):
    if sha(input_path / "PROTOCOL.md") != EXPECTED_PROTOCOL or sha(input_path / "PROTOCOL.v1.md") != EXPECTED_PROTOCOL:
        raise ValueError("FROZEN_PROTOCOL_CHANGED")
    if sha(input_path / "public-data.json") != EXPECTED_DATA:
        raise ValueError("FROZEN_DATA_CHANGED")
    if sha(input_path / "data-manifest.json") != "81fe614826dda86dcaf5d699e9b814d0f959b57ce8f5faa1db606eb4b1ab4f05":
        raise ValueError("FROZEN_MANIFEST_CHANGED")
    return json.loads((input_path / "public-data.json").read_text(encoding="utf-8"))


def run_partition(data, config, costs, partition):
    start, end = map(millis, PARTITIONS[partition])
    frames, trades, funding, by_symbol = [], [], [], {}
    for symbol in SYMBOLS:
        bars = [Candle(**c) for c in data["candles"][symbol] if start - 220 * H4 <= c["open_time"] < end]
        if len(bars) != (end - start) // H4 + 220 or bars[0].open_time != start - 220 * H4 or bars[-1].close_time + 1 != end:
            raise ValueError("PARTITION_COVERAGE")
        rate_rows = [r for r in data["funding"][symbol] if start <= r["at"] < end]
        expected = list(range(start, end, 2 * H4))
        if ([r["at"] // (2 * H4) * (2 * H4) for r in rate_rows] != expected
                or any(not 0 <= r["at"] % (2 * H4) < 1000 for r in rate_rows)):
            raise ValueError("PARTITION_FUNDING_COVERAGE")
        by_bar = {}
        for r in rate_rows:
            by_bar.setdefault(r["at"] // H4 * H4, []).append(r)
        engine = ChannelEngine(symbol, config, costs)
        series = []
        for bar in bars:
            warmup = bar.open_time < start
            frame = engine.step(bar, by_bar.get(bar.open_time, ()), trading=not warmup, force_close=bar.close_time + 1 == end)
            if not warmup:
                frames.append(project_frame(frame))
                series.append({"at": bar.close_time + 1, "pnl": frame["index_pnl_mtm"]})
        if engine.position or engine.pending:
            raise AssertionError("PARTITION_STATE_NOT_FLAT")
        trades.extend(engine.trades)
        funding.extend(engine.funding_ledger)
        by_symbol[symbol] = series
    equity = []
    for left, right in zip(by_symbol[SYMBOLS[0]], by_symbol[SYMBOLS[1]], strict=True):
        if left["at"] != right["at"]:
            raise AssertionError("SYMBOL_CLOCK_MISMATCH")
        equity.append({"at": left["at"], "equity_index": 1 + left["pnl"] + right["pnl"]})
    trades.sort(key=lambda t: (t["exit_recorded_at"], t["symbol"]))
    frames.sort(key=lambda f: (f["decision_at"], f["symbol"]))
    metrics, daily = summarize(trades, equity, funding, start, end)
    return {"partition": partition, "start_utc": utc(start), "end_exclusive_utc": utc(end),
            "funding_policy_version": FUNDING_POLICY_VERSION,
            "config": asdict(config), "costs": asdict(costs), "metrics": metrics,
            "trades": trades, "funding_ledger": funding, "equity": equity, "daily": daily, "frames": frames}


def economic_gates(base, stress, min_total, min_symbol):
    def pf_at_least(m, threshold):
        # JSON cannot contain Infinity. A positive numerator / zero losses is
        # represented by null PF plus explicit numerator/denominator fields.
        if m["pf"] is None:
            return m.get("gross_profit_net_trades", 0) > 0 and m.get("gross_loss_net_trades") == 0
        return m["pf"] >= threshold
    sample = {"total_sample": base["trade_count"] >= min_total,
              "symbol_sample": all(v["trade_count"] >= min_symbol for v in base["per_symbol"].values())}
    gates = {**sample, "base_net": base["net"] > 0, "base_pf": pf_at_least(base, 1.2),
        "stress_net": stress["net"] > 0, "stress_pf": pf_at_least(stress, 1),
        "both_symbols_positive": all(v["net"] > 0 for v in base["per_symbol"].values()),
        "two_positive_blocks": sum(n > 0 for n in base["subperiod_net"]) >= 2,
        "base_drawdown": base["max_mtm_drawdown_fraction"] <= .10,
        "stress_drawdown": stress["max_mtm_drawdown_fraction"] <= .12,
        "concentration": base["top3_positive_share"] is not None and base["top3_positive_share"] <= .5,
        "bootstrap_lower5_positive": base["bootstrap_daily_mean_lower5"] is not None and base["bootstrap_daily_mean_lower5"] > 0}
    return {"pass": all(gates.values()), "sample_sufficient": all(sample.values()), "gates": gates,
            "failed": [k for k, v in gates.items() if not v]}


def source_manifest():
    source_root = Path(__file__).resolve().parents[1]
    paths = sorted((source_root / "round01").glob("*.py")) + [source_root / "domain.py", source_root / "research/store.py"]
    return source_root, {p.relative_to(source_root).as_posix(): sha(p) for p in paths}


def run_study(input_path: Path, output: Path):
    # This CLI is frozen to the v1 protocol. Never label corrected accounting
    # with a historical protocol hash that specifies different funding rules.
    if FUNDING_POLICY_VERSION != "bar-boundary-v1":
        raise ValueError("FUNDING_POLICY_PROTOCOL_MISMATCH: v1 study requires frozen v1 source; no v2 study protocol is authorized")
    data = verify_input(input_path)
    if output.exists():
        raise FileExistsError("TRIAL_OUTPUT_EXISTS: preserve evidence; use a new directory for reproduction")
    output.mkdir(parents=True)
    root, sources = source_manifest()
    identity = {"protocol_sha256": EXPECTED_PROTOCOL, "data_sha256": EXPECTED_DATA, "source_sha256": digest(sources), "sources": sources}
    write_json(output / "source-manifest.json", identity)
    for file in sources:
        target = output / "source" / file
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(root / file, target)
    reports = {}
    trial_file = output / "trials.jsonl"

    def append_trial(value):
        with trial_file.open("a", encoding="utf-8") as f:
            f.write(encoded(value) + "\n")

    def execute(config, partition):
        pair = {}
        raw_decisions = None
        for costs in COSTS:
            run_id = "r01-" + digest({**identity, "config": asdict(config), "costs": asdict(costs), "partition": partition})[:24]
            entry = {"run_id": run_id, "config": asdict(config), "costs": asdict(costs), "partition": partition,
                     "protocol_sha256": EXPECTED_PROTOCOL, "data_sha256": EXPECTED_DATA, "source_sha256": identity["source_sha256"]}
            append_trial({**entry, "status": "RUNNING", "at": datetime.now(timezone.utc).isoformat()})
            try:
                result = run_partition(data, config, costs, partition)
                decisions = [(t["trade_id"], t["raw_entry"], t["raw_exit"], t["exit_reason"], t["qty_index"]) for t in result["trades"]]
                if raw_decisions is not None and decisions != raw_decisions:
                    raise AssertionError("COST_SCENARIO_DECISIONS_DIVERGED")
                raw_decisions = decisions
                result.update({"run_id": run_id, "identity": identity, "exploratory_already_seen": True})
                write_json(output / f"{run_id}.json", result)
                pair[costs.name] = {"run_id": run_id, "file": f"{run_id}.json", "sha256": sha(output / f"{run_id}.json"), **result["metrics"]}
                append_trial({**entry, "status": "COMPLETE", "at": datetime.now(timezone.utc).isoformat(), "metrics": result["metrics"]})
            except Exception as exc:
                append_trial({**entry, "status": "FAILED", "at": datetime.now(timezone.utc).isoformat(), "error": type(exc).__name__ + ": " + str(exc)})
                raise
        return pair

    for config in CONFIGS:
        reports[config.name] = {part: execute(config, part) for part in ("DEVELOPMENT", "SELECTION")}
    eligible, gates = [], {}
    for config in CONFIGS:
        r = reports[config.name]
        gate = economic_gates(r["SELECTION"]["BASE"], r["SELECTION"]["STRESS"], 30, 10)
        dev = {"development_sample": r["DEVELOPMENT"]["BASE"]["trade_count"] >= 20,
               "development_base": r["DEVELOPMENT"]["BASE"]["net"] > 0,
               "development_stress": r["DEVELOPMENT"]["STRESS"]["net"] >= 0}
        gate["gates"].update(dev)
        gate["sample_sufficient"] &= dev["development_sample"]
        gate["pass"] = all(gate["gates"].values())
        gate["failed"] = [k for k, v in gate["gates"].items() if not v]
        gates[config.name] = gate
        if gate["pass"]:
            eligible.append(config)
    eligible.sort(key=lambda c: (-min(s["net"] for s in reports[c.name]["SELECTION"]["BASE"]["per_symbol"].values()), c.name))
    finalist = eligible[0] if eligible else None
    freeze = {"frozen_at": datetime.now(timezone.utc).isoformat(), "identity": identity,
              "config": asdict(finalist) if finalist else None, "selection_gates": gates,
              "status": "FROZEN_ONE_CANDIDATE" if finalist else "NOT_RUN_NO_ELIGIBLE_CANDIDATE"}
    write_json(output / "FINAL_CANDIDATE.json", freeze)
    final_result = None
    if finalist:
        # Only this branch can execute final P&L; no loop over candidates.
        final_result = execute(finalist, "FINAL_EXPLORATORY")
        final_gate = economic_gates(final_result["BASE"], final_result["STRESS"], 15, 5)
        decision = "LANJUT KE AUDIT" if final_gate["pass"] else ("TOLAK" if final_gate["sample_sufficient"] else "BELUM DAPAT DIPUTUSKAN")
    else:
        final_gate = None
        decision = "TOLAK" if all(g["sample_sufficient"] for g in gates.values()) else "BELUM DAPAT DIPUTUSKAN"
    report = {"schema": "round01-study-v1", "decision": decision, "identity": identity, "reports": reports,
        "selection_gates": gates, "final": final_result, "final_gate": final_gate,
        "final_freeze_sha256": sha(output / "FINAL_CANDIDATE.json"), "final_status": freeze["status"],
        "exploratory_already_seen": True, "account_model_complete": False,
        "no_trade": {p: {"trade_count": 0, "gross": 0, "net": 0, "fee_cost": 0, "slippage_cost": 0,
                         "funding_cost": 0, "max_mtm_drawdown_fraction": 0, "pf": None} for p in PARTITIONS}}
    write_json(output / "SUMMARY.json", report)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = run_study(args.input, args.output)
    print(encoded({"decision": result["decision"], "selection_gates": result["selection_gates"], "final_status": result["final_status"],
                   "results": {c: {p: {k: {m: v[m] for m in ("trade_count", "net", "pf", "max_mtm_drawdown_fraction")} for k, v in costs.items()} for p, costs in parts.items()} for c, parts in result["reports"].items()}}))
