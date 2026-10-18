"""Durable, read-only closed-bar observation; NOT an intrabar execution adapter.

The frozen research engine is unchanged. Funding is missing, so this adapter
never publishes its partial result as complete net P&L or account return.
"""
from __future__ import annotations

import copy
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
import sqlite3

from binance_sniper.domain import Candle
from binance_sniper.round01.data import H4
from binance_sniper.round01.engine import ChannelEngine, CONFIGS, COSTS

SYMBOLS = ("BTCUSDT", "ETHUSDT")
WARMUP = 220
MODE = "LIVE_CLOSED_BAR_OBSERVATION"


def fingerprint():
    raw = Path(__file__).with_name("engine.py").read_bytes()
    spec = {"adapter": 1, "configs": [asdict(c) for c in CONFIGS],
            "costs": asdict(COSTS[0]), "warmup": WARMUP, "funding": "MISSING_NOT_ZERO"}
    return hashlib.sha256(raw + Path(__file__).read_bytes() + json.dumps(spec, sort_keys=True).encode()).hexdigest()


class ChannelObserver:
    def __init__(self, path: Path, now: int):
        path.parent.mkdir(parents=True, exist_ok=True)
        self.db = sqlite3.connect(path)
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.executescript("""
          CREATE TABLE IF NOT EXISTS observer_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS observer_inputs (
            symbol TEXT NOT NULL, opened INTEGER NOT NULL, candle TEXT NOT NULL,
            observed_at INTEGER NOT NULL, trading INTEGER NOT NULL,
            PRIMARY KEY(symbol, opened));
        """)
        self.identity = fingerprint()
        meta = dict(self.db.execute("SELECT key,value FROM observer_meta"))
        if meta and meta.get("identity") != self.identity:
            self.db.close()
            raise ValueError("OBSERVER_CODE_IDENTITY_CHANGED_REQUIRES_EXPLICIT_MIGRATION")
        if not meta:
            with self.db:
                self.db.executemany("INSERT INTO observer_meta VALUES (?,?)",
                                    [("identity", self.identity), ("started_at", str(now))])
        self.started_at = int(meta.get("started_at", now))
        self.engines = {s: [ChannelEngine(s, c, COSTS[0]) for c in CONFIGS] for s in SYMBOLS}
        self.inputs: dict[str, dict[int, dict]] = {s: {} for s in SYMBOLS}
        self.status = {s: "STARTING" for s in SYMBOLS}
        self.errors: dict[str, str] = {}
        self.updated_at = 0
        try:
            for s, opened, payload, observed_at, trading in self.db.execute(
                    "SELECT symbol,opened,candle,observed_at,trading FROM observer_inputs ORDER BY symbol,opened"):
                bar = Candle(**json.loads(payload))
                if s not in self.engines or bar.symbol != s or opened != bar.open_time:
                    raise ValueError("OBSERVER_PERSISTED_INPUT_INVALID")
                for engine in self.engines[s]:
                    engine.step(bar, available_at=observed_at, trading=bool(trading))
                self.inputs[s][opened] = {"bar": bar, "observed_at": observed_at, "trading": bool(trading)}
        except Exception:
            self.db.close()
            raise

    def close(self):
        self.db.close()

    def _append(self, symbol, bars, now, trading):
        # Validate all configurations before committing any input. Disk precedes
        # in-memory replacement, so a crash can always rebuild deterministically.
        engines = copy.deepcopy(self.engines[symbol])
        for bar in bars:
            for engine in engines:
                engine.step(bar, available_at=now, trading=trading)
        with self.db:
            self.db.executemany("INSERT INTO observer_inputs VALUES (?,?,?,?,?)",
                [(symbol, b.open_time, json.dumps(asdict(b), sort_keys=True), now, int(trading)) for b in bars])
        self.engines[symbol] = engines
        for bar in bars:
            self.inputs[symbol][bar.open_time] = {"bar": bar, "observed_at": now, "trading": trading}

    def capture(self, cache, now: int, stale_after_ms=90_000):
        for symbol in SYMBOLS:
            feed = cache.caches.get((symbol, "4h"))
            if feed is None:
                self.status[symbol] = "NO_FEED"
                continue
            bars = feed.values()
            q = feed.audit(now)
            if q["success_age_ms"] is None or q["success_age_ms"] > stale_after_ms or q.get("last_error_class"):
                self.status[symbol] = "PAUSED_STALE_DATA"
                continue
            if not bars or bars[-1].open_time != now // H4 * H4 - H4:
                self.status[symbol] = "WAIT_LATEST_H4_CLOSE"
                continue
            # Revisions and missing history never silently overwrite a decision.
            prior = self.inputs[symbol]
            if any(b.open_time in prior and prior[b.open_time]["bar"] != b for b in bars):
                self.errors[symbol] = "REVISION_OF_CONSUMED_BAR"
            if symbol in self.errors:
                self.status[symbol] = "PAUSED_INTEGRITY_ERROR"
                continue
            try:
                if not prior:
                    if len(bars) < WARMUP:
                        self.status[symbol] = "WAIT_220_WARMUP_BARS"
                        continue
                    self._append(symbol, bars[-WARMUP:], now, False)
                else:
                    new = [b for b in bars if b.open_time > max(prior)]
                    if new and new[0].open_time != max(prior) + H4:
                        self.status[symbol] = "PAUSED_MISSING_H4_HISTORY"
                        continue
                    if new:
                        self._append(symbol, new, now, True)
                self.status[symbol] = "OBSERVING"
            except (ValueError, sqlite3.Error) as exc:
                self.errors[symbol] = str(exc) if isinstance(exc, ValueError) else "PERSISTENCE_WRITE_FAILED"
                self.status[symbol] = "PAUSED_INTEGRITY_ERROR"
        self.updated_at = now

    def snapshot(self, cache, now: int, stale_after_ms=90_000):
        rows = []
        for symbol in SYMBOLS:
            feed = cache.caches.get((symbol, "4h"))
            price_feed = cache.caches.get((symbol, "5m"))
            quality = feed.audit(now) if feed else {}
            price_quality = price_feed.audit(now) if price_feed else {}
            preview = price_feed.preview_value() if price_feed else None
            price_fresh = bool(preview and price_quality.get("preview_age_ms") is not None
                               and price_quality["preview_age_ms"] <= 9_000
                               and not price_quality.get("last_error_class")
                               and preview.open_time <= now <= preview.close_time)
            state = self.status[symbol]
            if state == "OBSERVING" and (quality.get("success_age_ms") is None
                    or quality["success_age_ms"] > stale_after_ms or quality.get("last_error_class")):
                state = "PAUSED_STALE_DATA"
            inputs = self.inputs[symbol]
            forward = [r for r in inputs.values() if r["trading"]]
            for engine in self.engines[symbol]:
                frame = engine.last_frame or {}
                channels = engine._channels()
                model_events = []
                # Recompute only for persistence restore, not during UI polling.
                # Trades already produced by the exact engine are projected here.
                for trade in engine.trades[-30:]:
                    receipt = inputs[trade["exit_bar_open"]]
                    model_events.append({
                        "id": trade["trade_id"], "direction": trade["direction"],
                        "entry_model_at": trade["entry_at"], "exit_bar_open": trade["exit_bar_open"],
                        "recorded_at": receipt["observed_at"], "exit_reason": trade["exit_reason"],
                        "entry": trade["entry_fill"], "exit": trade["exit_fill"],
                        "index_after_fee_slippage_before_funding": trade["net"], "net_complete": None,
                        "origin": "RECONSTRUCTED_AFTER_DELAY" if receipt["observed_at"] - trade["exit_recorded_at"] > stale_after_ms else "CLOSED_BAR_MODEL",
                    })
                bars = engine.history[-80:]
                h4_preview = feed.preview_value() if feed else None
                if h4_preview and (not bars or h4_preview.open_time == bars[-1].open_time + H4):
                    chart = [asdict(b) for b in bars] + [asdict(h4_preview)]
                else:
                    chart = [asdict(b) for b in bars]
                rows.append({
                    "id": f"{symbol}:{engine.config.name}", "symbol": symbol,
                    "config": asdict(engine.config), "status": state, "error": self.errors.get(symbol),
                    "price": preview.close if preview else None, "price_fresh": price_fresh,
                    "price_age_ms": price_quality.get("preview_age_ms"),
                    "h4_success_age_ms": quality.get("success_age_ms"),
                    "h4_transport": quality.get("transport"), "channels": channels,
                    "atr": engine.atr, "reason": frame.get("reason", "STARTING"),
                    "position_model_at_last_close": asdict(engine.position) if engine.position else None,
                    "pending_model": copy.deepcopy(engine.pending),
                    "stop_for_next_bar": frame.get("stop_for_next_bar"),
                    "last_closed_at": frame.get("decision_at"),
                    "last_received_at": max((r["observed_at"] for r in inputs.values()), default=None),
                    "next_close_at": (now // H4 + 1) * H4,
                    "warmup_bars": sum(not r["trading"] for r in inputs.values()),
                    "observed_closed_bars": len(forward),
                    "delayed_closed_bars": sum(r["observed_at"] - (r["bar"].close_time+1) > stale_after_ms for r in forward),
                    "trade_count": len(engine.trades), "trades": model_events,
                    "realized_index_before_funding": engine.realized,
                    "mtm_at_last_close_before_funding": frame.get("index_pnl_mtm"),
                    "net_complete": None, "cost_complete_for_model": False,
                    "account_model_complete": False, "chart": chart,
                })
        healthy = all(r["status"] == "OBSERVING" and r["price_fresh"] for r in rows)
        return {"status": "OBSERVING_LIVE_MARKET" if healthy else "DEGRADED",
                "mode": MODE, "execution": "DISABLED_NO_ORDER_ROUTER", "strategy_verdict": "ROUND01_REJECTED_EXPERIMENTAL",
                "started_at": self.started_at, "updated_at": self.updated_at, "server_now": now,
                "code_identity": self.identity, "funding_status": "MISSING_NOT_ZERO",
                "fill_model": "NEXT_OPEN_AND_STOPS_RECONSTRUCTED_AT_H4_CLOSE_NOT_LIVE_FILLS",
                "costs": asdict(COSTS[0]), "rows": rows}
