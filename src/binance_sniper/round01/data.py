"""Export only public candles/funding from the existing cache, read-only."""
from __future__ import annotations

import argparse
from dataclasses import asdict
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import sqlite3

from binance_sniper.domain import Candle
from binance_sniper.research.store import digest, encoded

H4 = 14_400_000
DAY = 86_400_000
SYMBOLS = ("BTCUSDT", "ETHUSDT")
DATASETS = ("binance-1750377600000-1781481600000-v1", "binance-1781481600000-1789257600000-v1")


def utc(ms):
    return datetime.fromtimestamp(ms / 1000, timezone.utc).isoformat()


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    Path(path).write_text(encoded(value) + "\n", encoding="utf-8")


def prepare(database: Path, output: Path):
    output.mkdir(parents=True, exist_ok=True)
    target = output / "public-data.json"
    if target.exists():
        raise FileExistsError("Public export is immutable; use a new directory")
    manifest = {"schema": "round01-data-v1", "exported_at": datetime.now(timezone.utc).isoformat(),
                "cache": str(database.resolve()), "exploratory_already_seen": True, "feeds": [], "funding": []}
    candles = {s: {} for s in SYMBOLS}
    funding = {s: {} for s in SYMBOLS}
    db = sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    try:
        for dataset in DATASETS:
            record = db.execute("SELECT * FROM datasets WHERE id=?", (dataset,)).fetchone()
            if not record or record["status"] != "COMPLETE":
                raise ValueError("DATASET_NOT_COMPLETE")
            original = json.loads(record["manifest"])
            for symbol in SYMBOLS:
                rows = list(db.execute("SELECT * FROM candles WHERE dataset=? AND symbol=? AND tf='4h' ORDER BY open_time", (dataset, symbol)))
                values, checksums = [], []
                for row in rows:
                    c = Candle(symbol, "4h", row["open_time"], row["close_time"], row["o"], row["h"], row["l"], row["c"], row["v"], True)
                    if (digest(asdict(c)) != row["checksum"] or c.open_time % H4 or c.close_time != c.open_time + H4 - 1
                            or not all(math.isfinite(x) for x in (c.open, c.high, c.low, c.close, c.volume))
                            or not 0 < c.low <= min(c.open, c.close) <= max(c.open, c.close) <= c.high or c.volume < 0):
                        raise ValueError("CANDLE_INVALID_OR_CHANGED")
                    checksums.append(row["checksum"])
                    values.append(asdict(c))
                declared = original["feeds"][symbol + ":4h"]
                actual_hash = hashlib.sha256("".join(checksums).encode()).hexdigest()
                if len(values) != declared["expected"] or actual_hash != declared["sha256"]:
                    raise ValueError("CACHE_MANIFEST_MISMATCH")
                if values[0]["open_time"] != record["start_ms"] - 220 * H4 or values[-1]["close_time"] + 1 != record["end_ms"]:
                    raise ValueError("COVERAGE_BOUNDARY_MISMATCH")
                if any(b["open_time"] - a["open_time"] != H4 for a, b in zip(values, values[1:])):
                    raise ValueError("CANDLE_GAP")
                overlap = 0
                for c in values:
                    prior = candles[symbol].get(c["open_time"])
                    if prior is not None:
                        overlap += 1
                        if prior != c:
                            raise ValueError("OVERLAPPING_CACHE_REVISION")
                    candles[symbol][c["open_time"]] = c
                manifest["feeds"].append({"dataset": dataset, "symbol": symbol, "interval": "4h", "source": original["source"],
                    "retrieved_at": utc(original["downloaded_at"]), "start_utc": utc(values[0]["open_time"]),
                    "end_exclusive_utc": utc(values[-1]["close_time"] + 1), "bars": len(values), "coverage": 1.0,
                    "gaps": 0, "duplicates_within_feed": 0, "identical_overlap_with_prior_dataset": overlap, "sha256": actual_hash})
                rates = [dict(r) for r in db.execute("SELECT at,rate,mark FROM funding WHERE dataset=? AND symbol=? ORDER BY at", (dataset, symbol))]
                declared_f = original["funding"][symbol]
                raw = [(dataset, symbol, r["at"], r["rate"], r["mark"]) for r in rates]
                funding_hash = digest(raw)
                if declared_f["status"] != "AVAILABLE" or len(rates) != declared_f["rows"]:
                    raise ValueError("FUNDING_NOT_AVAILABLE")
                if declared_f.get("sha256") and funding_hash != declared_f["sha256"]:
                    raise ValueError("FUNDING_CHECKSUM_MISMATCH")
                # Verify every scheduled 8-hour bucket, including the first/last;
                # timestamps may carry a few milliseconds of publication jitter.
                expected = list(range(record["start_ms"], record["end_ms"], 2 * H4))
                buckets = [r["at"] // (2 * H4) * (2 * H4) for r in rates]
                if buckets != expected or any(not 0 <= r["at"] % (2 * H4) < 1000 or not math.isfinite(r["rate"]) or not math.isfinite(r["mark"]) or r["mark"] <= 0 for r in rates):
                    raise ValueError("FUNDING_SCHEDULE_OR_VALUE_INVALID")
                for r in rates:
                    if r["at"] in funding[symbol]:
                        raise ValueError("DUPLICATE_FUNDING")
                    funding[symbol][r["at"]] = r
                manifest["funding"].append({"dataset": dataset, "symbol": symbol, "source": "https://fapi.binance.com/fapi/v1/fundingRate",
                    "retrieved_at": utc(original["downloaded_at"]), "rows": len(rates), "start_utc": utc(record["start_ms"]),
                    "end_exclusive_utc": utc(record["end_ms"]), "gaps": 0, "duplicates": 0, "observed_schedule_hours": 8,
                    "sha256": funding_hash, "original_hash_available": "sha256" in declared_f,
                    "limitation": "8h coverage verified from cache; original downloader not an independent archive attestation"})
        payload = {"candles": {s: list(sorted(candles[s].values(), key=lambda c: c["open_time"])) for s in SYMBOLS},
                   "funding": {s: list(sorted(funding[s].values(), key=lambda r: r["at"])) for s in SYMBOLS}}
        write_json(target, payload)
        manifest["public_data_sha256"] = sha(target)
        write_json(output / "data-manifest.json", manifest)
        return {"data_sha256": sha(target), "manifest_sha256": sha(output / "data-manifest.json"),
                "bars": {s: len(v) for s, v in payload["candles"].items()}, "funding": {s: len(v) for s, v in payload["funding"].items()}}
    finally:
        db.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(encoded(prepare(args.database, args.output)))
