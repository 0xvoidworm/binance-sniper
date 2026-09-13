from __future__ import annotations

import time
from collections import deque
from dataclasses import asdict
from pathlib import Path

from binance_sniper.analysis.impulse import detect as detect_impulse
from binance_sniper.analysis.regime import regime
from binance_sniper.analysis.ranker import rank
from binance_sniper.analysis.triggers import detect as detect_trigger
from binance_sniper.analysis.zones import build as build_zone
from binance_sniper.domain import Candidate, Zone, ZoneState
from binance_sniper.analysis.lifecycle import FourBarTracker, ZoneLedger, pullback_valid, zone_key
from binance_sniper.market.mtf_cache import MtfCache


class Scanner:
    def __init__(self, cache: MtfCache, settings):
        self.cache, self.settings = cache, settings
        self.latest: dict[str, dict] = {symbol: {"symbol": symbol, "bias": "NEUTRAL", "zone": None, "trigger": None, "status": "WAITING"} for symbol in settings.symbols}
        self.events = deque(maxlen=200)
        self.reconnects = 0
        self.ledger = ZoneLedger(Path("state") / "zones.json")
        self.ledger.load()
        self.horizon = FourBarTracker()
        self.last_signature: dict[str, tuple] = {}

    def evaluate(self, symbol: str) -> None:
        h4, h1, m15, m5 = [self.cache.candles(symbol, tf) for tf in ("4h", "1h", "15m", "5m")]
        bias, er, _ = regime(h4, self.settings.pivot_left_right)
        impulse = detect_impulse(h1, bias, self.settings.pivot_left_right)
        zone = build_zone(impulse, bias, m15, self.settings.zone_atr_width) if pullback_valid(m15, impulse, bias, self.settings.pullback_min_depth, self.settings.pullback_max_depth, self.settings.pullback_max_bars) else None
        trigger = detect_trigger(m5, zone, bias, self.settings.trigger_min_body_fraction)
        if zone:
            key = zone_key(impulse, zone.created_at)
            zone = self.ledger.upsert(zone, key)
            if trigger:
                self.ledger.transition(key, ZoneState.TRIGGERED, "M5_RECLAIM_MICRO_BREAK", trigger["time"])
            else:
                self.ledger.transition(key, ZoneState.ARMED, "PULLBACK_VALID", zone.created_at)
            self.ledger.save()
        else:
            key = ""
        state = {"symbol": symbol, "bias": bias, "efficiency": er, "impulse": impulse, "zone": asdict(zone) if zone else None, "trigger": trigger, "status": "TRIGGERED" if trigger else ("ZONE_ARMED" if zone else "NO_TRIGGER"), "last_update": int(time.time() * 1000)}
        self.latest[symbol] = state
        signature = (state["bias"], state["status"], key, trigger["time"] if trigger else 0)
        if self.last_signature.get(symbol) != signature:
            self.last_signature[symbol] = signature
            self.events.appendleft({"time": state["last_update"], "symbol": symbol, "reason_code": "TRIGGER_CONFIRMED" if trigger else "NO_TRIGGER", "status": state["status"]})

    def evaluate_all(self) -> None:
        for symbol in self.settings.symbols:
            self.evaluate(symbol)

    def snapshot(self) -> dict:
        candidates = []
        for state in self.latest.values():
            if state.get("trigger") and state.get("zone"):
                trigger, zone = state["trigger"], state["zone"]
                entry = trigger["price"]
                stop = zone["invalidation"]
                risk = abs(entry - stop)
                target = entry + (2 * risk if state["bias"] == "BULLISH" else -2 * risk)
                net_room = 2.0
                pullback_quality = 1.0
                trigger_quality = min(1.0, trigger.get("body_fraction", 0.0))
                score = max(0.0, min(1.0, 0.30 * state["efficiency"] + 0.30 * state["impulse"]["quality"] + 0.20 * pullback_quality + 0.20 * trigger_quality))
                candidate_id = f"{state['symbol']}:{state['impulse']['id']}:{trigger['time']}"
                self.horizon.start(candidate_id, entry, stop, target, trigger["time"])
                candidates.append(Candidate(state["symbol"], state["bias"], trigger["time"], score, net_room, state["efficiency"], state["impulse"]["quality"], state["impulse"]["id"], "TRIGGER_CONFIRMED", entry, stop, target))
        ranked = rank(candidates)
        charts = {}
        for symbol in self.settings.symbols:
            for timeframe in self.settings.timeframes:
                cache = self.cache.caches[(symbol, timeframe)]
                charts[f"{symbol}:{timeframe}"] = {"closed": [asdict(candle) for candle in cache.values()[-80:]], "preview": asdict(cache.preview_value()) if cache.preview_value() else None}
        return {"symbols": self.latest, "charts": charts, "candidates": [c.json() for c in ranked], "outcomes": {**self.horizon.active, **self.horizon.completed}, "events": list(self.events), "feed_quality": self.cache.quality(), "reconnects": self.reconnects}
