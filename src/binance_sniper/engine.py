from __future__ import annotations

import time
from collections import deque
from dataclasses import asdict

from binance_sniper.analysis.impulse import detect as detect_impulse
from binance_sniper.analysis.regime import regime
from binance_sniper.analysis.ranker import rank
from binance_sniper.analysis.triggers import detect as detect_trigger
from binance_sniper.analysis.zones import build as build_zone
from binance_sniper.domain import Candidate, Zone, ZoneState
from binance_sniper.market.mtf_cache import MtfCache


class Scanner:
    def __init__(self, cache: MtfCache, settings):
        self.cache, self.settings = cache, settings
        self.latest: dict[str, dict] = {symbol: {"symbol": symbol, "bias": "NEUTRAL", "zone": None, "trigger": None, "status": "WAITING"} for symbol in settings.symbols}
        self.events = deque(maxlen=200)
        self.reconnects = 0

    def evaluate(self, symbol: str) -> None:
        h4, h1, m15, m5 = [self.cache.candles(symbol, tf) for tf in ("4h", "1h", "15m", "5m")]
        bias, er, _ = regime(h4, self.settings.pivot_left_right)
        impulse = detect_impulse(h1, bias, self.settings.pivot_left_right)
        zone = build_zone(impulse, bias, m15, self.settings.zone_atr_width)
        trigger = detect_trigger(m5, zone, bias, self.settings.trigger_min_body_fraction)
        if zone and trigger:
            zone.state = ZoneState.TRIGGERED
        elif zone:
            zone.state = ZoneState.ARMED
        state = {"symbol": symbol, "bias": bias, "efficiency": er, "impulse": impulse, "zone": asdict(zone) if zone else None, "trigger": trigger, "status": "TRIGGERED" if trigger else ("ZONE_ARMED" if zone else "NO_TRIGGER"), "last_update": int(time.time() * 1000)}
        self.latest[symbol] = state
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
                candidates.append(Candidate(state["symbol"], state["bias"], trigger["time"], 0.0, 2.0, state["efficiency"], state["impulse"]["quality"], state["impulse"]["id"], "TRIGGER_CONFIRMED", entry, stop, target))
        ranked = rank(candidates)
        charts = {}
        for symbol in self.settings.symbols:
            for timeframe in self.settings.timeframes:
                charts[f"{symbol}:{timeframe}"] = [asdict(candle) for candle in self.cache.candles(symbol, timeframe)[-80:]]
        return {"symbols": self.latest, "charts": charts, "candidates": [c.json() for c in ranked], "events": list(self.events), "feed_quality": self.cache.quality(), "reconnects": self.reconnects}
