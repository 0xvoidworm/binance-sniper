from __future__ import annotations

import time
from dataclasses import asdict
from pathlib import Path

from binance_sniper.analysis.impulse import detect as detect_impulse
from binance_sniper.analysis.lifecycle import FourBarTracker, ZoneLedger, pullback_assessment, zone_key
from binance_sniper.analysis.regime import regime
from binance_sniper.analysis.triggers import detect as detect_trigger
from binance_sniper.analysis.zones import build as build_zone
from binance_sniper.domain import Candidate, Zone, ZoneState
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.presentation import decision_view


TERMINAL_ZONE_STATES = {ZoneState.COMPLETED, ZoneState.INVALIDATED, ZoneState.EXPIRED}


class Scanner:
    """Closed-bar-only H4 -> H1 -> M15 -> M5 causal scanner."""

    def __init__(self, cache: MtfCache, settings, state_dir: Path | None = None):
        self.cache, self.settings = cache, settings
        self.latest: dict[str, dict] = {
            symbol: {"symbol": symbol, "bias": "NEUTRAL", "zone": None, "trigger": None, "status": "WAITING"}
            for symbol in settings.symbols
        }
        self.reconnects = 0
        root = state_dir or Path("state")
        self.ledger = ZoneLedger(root / "zones.json")
        self.ledger.load()
        self.horizon = FourBarTracker(root / "four-bar.json")
        self.horizon.load()

    @staticmethod
    def _touches(zone: Zone, candle) -> bool:
        return candle.low <= zone.upper and candle.high >= zone.lower

    @staticmethod
    def _invalidated(zone: Zone, candle) -> bool:
        return candle.low <= zone.invalidation if zone.direction == "BULLISH" else candle.high >= zone.invalidation

    def _create_candidate(self, symbol: str, bias: str, efficiency: float, impulse: dict, zone: Zone, trigger: dict) -> dict:
        entry = float(trigger["price"])
        stop = float(zone.invalidation)
        risk = abs(entry - stop)
        target = entry + (2 * risk if bias == "BULLISH" else -2 * risk)
        directional_room = float(impulse["end"]) - entry if bias == "BULLISH" else entry - float(impulse["end"])
        net_room = max(0.0, directional_room / risk) if risk > 0 else 0.0
        components = {
            "regime": max(0.0, min(1.0, float(efficiency))),
            "impulse": max(0.0, min(1.0, float(impulse["quality"]))),
            "pullback": max(0.0, min(1.0, float(zone.quality.get("pullback_quality", 0.0)))),
            "trigger": max(0.0, min(1.0, float(trigger.get("body_fraction", 0.0)))),
            "room": max(0.0, min(1.0, net_room / 2.0)),
        }
        score = (
            0.25 * components["regime"] + 0.25 * components["impulse"]
            + 0.20 * components["pullback"] + 0.20 * components["trigger"]
            + 0.10 * components["room"]
        )
        candidate_id = f"{zone.zone_id}:{trigger['time']}"
        candidate = Candidate(
            symbol=symbol, direction=bias, decision_time=int(trigger["time"]),
            score=max(0.0, min(1.0, score)), net_room_r=net_room,
            trend_efficiency=float(efficiency), impulse_quality=float(impulse["quality"]),
            zone_id=zone.zone_id, reason_code=str(trigger["reason_code"]),
            entry_reference=entry, stop_reference=stop, target_reference=target,
            candidate_id=candidate_id, score_components=components,
        ).json()
        if self.ledger.register_candidate(candidate_id, candidate):
            self.horizon.start(candidate_id, entry, stop, target, int(trigger["time"]), symbol, zone.zone_id)
        return self.ledger.candidates[candidate_id]

    def _observe_outcomes(self, symbol: str, m5: list) -> None:
        completed_zone_ids: list[tuple[str, int]] = []
        for candidate_id, item in list(self.horizon.active.items()):
            if item.get("symbol") != symbol:
                continue
            for candle in m5:
                if candle.close_time <= int(item.get("last_bar_close", item["started_at"])):
                    continue
                result = self.horizon.observe(candidate_id, candle, self.settings.four_bar_horizon)
                if result and result["status"] == "COMPLETED":
                    completed_zone_ids.append((str(result.get("zone_id", "")), candle.close_time))
                    break
        for item in self.horizon.completed.values():
            key = str(item.get("zone_id", ""))
            zone = self.ledger.zones.get(key)
            if item.get("symbol") == symbol and zone is not None and zone.state == ZoneState.TRIGGERED:
                completed_zone_ids.append((key, int(item.get("last_bar_close", item["started_at"]))))
        for key, completed_at in completed_zone_ids:
            if key in self.ledger.zones and self.ledger.transition(key, ZoneState.COMPLETED, "FOUR_BAR_COMPLETED", completed_at):
                outcome = next((value for value in self.horizon.completed.values() if value.get("zone_id") == key), None)
                self.ledger.record_event_once({
                    "event_id": f"{key}|OUTCOME|{completed_at}", "zone_id": key,
                    "symbol": self.ledger.zones[key].symbol, "from": "TRACKING", "to": "OUTCOME",
                    "reason_code": "FOUR_BAR_OUTCOME", "time": completed_at, "outcome": outcome,
                })
                self.ledger.save()

    def _advance_zone(self, symbol: str, bias: str, efficiency: float, impulse: dict, zone: Zone, m5: list) -> dict | None:
        trigger_payload: dict | None = None
        changed = False
        if zone.state == ZoneState.DETECTED:
            changed = self.ledger.transition(zone.zone_id, ZoneState.ARMED, "PULLBACK_VALID", zone.created_at) or changed

        for index, candle in enumerate(m5):
            if candle.close_time <= max(zone.created_at, zone.last_evaluated_m5):
                continue
            if zone.state in TERMINAL_ZONE_STATES or zone.state == ZoneState.TRIGGERED:
                break
            zone.last_evaluated_m5 = candle.close_time
            zone.updated_at = candle.close_time
            changed = True
            if candle.close_time > zone.expires_at:
                changed = self.ledger.transition(zone.zone_id, ZoneState.EXPIRED, "ZONE_EXPIRED", candle.close_time) or changed
                break
            if self._invalidated(zone, candle):
                changed = self.ledger.transition(zone.zone_id, ZoneState.INVALIDATED, "ZONE_INVALIDATED", candle.close_time) or changed
                break
            if zone.state == ZoneState.ARMED and self._touches(zone, candle):
                changed = self.ledger.transition(zone.zone_id, ZoneState.TOUCHED, "M5_ZONE_TOUCH", candle.close_time) or changed
            if zone.state == ZoneState.TOUCHED:
                trigger = detect_trigger(m5[: index + 1], zone, bias, self.settings.trigger_min_body_fraction)
                if trigger and self.ledger.transition(zone.zone_id, ZoneState.TRIGGERED, "M5_RECLAIM_MICRO_BREAK", trigger["time"]):
                    trigger_payload = trigger
                    self._create_candidate(symbol, bias, efficiency, impulse, zone, trigger)
                    changed = True
                    break
        if changed:
            self.ledger.save()
        return trigger_payload

    def evaluate(self, symbol: str) -> None:
        h4, h1, m15, m5 = [self.cache.candles(symbol, tf) for tf in ("4h", "1h", "15m", "5m")]
        self._observe_outcomes(symbol, m5)
        bias, efficiency, pivots = regime(h4, self.settings.pivot_left_right)
        impulse = detect_impulse(h1, bias, self.settings.pivot_left_right)
        zone: Zone | None = None
        trigger: dict | None = None

        ledger_changed = False
        if bias != "NEUTRAL" and h4:
            decision_time = h4[-1].close_time
            ledger_changed = self.ledger.record_event_once({
                "event_id": f"{symbol}|REGIME|{bias}|{decision_time}", "symbol": symbol,
                "from": None, "to": "REGIME", "reason_code": f"H4_{bias}",
                "time": decision_time, "efficiency": efficiency,
            }) or ledger_changed
        if impulse:
            ledger_changed = self.ledger.record_event_once({
                "event_id": f"{impulse['id']}|IMPULSE", "symbol": symbol,
                "from": "REGIME", "to": "IMPULSE", "reason_code": f"H1_{bias}_IMPULSE",
                "time": impulse["end_time"], "impulse": impulse,
            }) or ledger_changed
            key = zone_key(impulse)
            zone = self.ledger.zones.get(key)
            if zone is None:
                pullback = pullback_assessment(
                    m15, impulse, bias, self.settings.pullback_min_depth,
                    self.settings.pullback_max_depth, self.settings.pullback_max_bars,
                )
                if pullback:
                    zone = build_zone(
                        impulse, bias, m15, self.settings.zone_atr_width,
                        created_at=int(pullback["detected_at"]), pullback=pullback,
                    )
                    if zone:
                        zone.quality.update({"impulse_end": float(impulse["end"]), "regime_efficiency": float(efficiency)})
                        zone = self.ledger.upsert(zone, key)
                        self.ledger.save()
        tracked_zones = [item for item in self.ledger.zones.values() if item.symbol == symbol and item.state not in TERMINAL_ZONE_STATES]
        for tracked in tracked_zones:
            context = impulse if impulse and tracked.source_impulse == impulse["id"] else {
                "id": tracked.source_impulse,
                "end": tracked.quality.get("impulse_end", tracked.upper if tracked.direction == "BULLISH" else tracked.lower),
                "quality": tracked.quality.get("impulse_quality", 0.0),
            }
            tracked_trigger = self._advance_zone(
                symbol, tracked.direction, float(tracked.quality.get("regime_efficiency", efficiency)),
                context, tracked, m5,
            )
            if zone is not None and tracked.zone_id == zone.zone_id and tracked_trigger:
                trigger = tracked_trigger
        self._observe_outcomes(symbol, m5)

        if zone is None:
            visible = [item for item in self.ledger.zones.values() if item.symbol == symbol]
            zone = max(visible, key=lambda item: (item.state not in TERMINAL_ZONE_STATES, item.created_at)) if visible else None
        if ledger_changed:
            self.ledger.save()

        if zone and trigger is None and zone.state in {ZoneState.TRIGGERED, ZoneState.COMPLETED}:
            candidate = next((item for item in self.ledger.candidates.values() if item.get("zone_id") == zone.zone_id), None)
            if candidate:
                trigger = {
                    "time": candidate["decision_time"], "price": candidate["entry_reference"],
                    "body_fraction": candidate.get("score_components", {}).get("trigger", 0.0),
                    "reason_code": candidate["reason_code"],
                }

        status = zone.state.value if zone else ("NO_IMPULSE" if impulse is None else "NO_ZONE")
        self.latest[symbol] = {
            "symbol": symbol, "bias": bias, "efficiency": efficiency,
            "pivots": pivots, "impulse": impulse, "zone": asdict(zone) if zone else None,
            "trigger": trigger, "status": status, "last_update": int(time.time() * 1000),
            "decision": decision_view(bias, efficiency, impulse, zone, m5, self.settings, len(h4)),
        }

    def evaluate_all(self) -> None:
        for symbol in self.settings.symbols:
            self.evaluate(symbol)

    def snapshot(self) -> dict:
        """Pure projection. Calling this method must never mutate analysis state."""
        candidates = sorted(
            self.ledger.candidates.values(),
            key=lambda item: (-float(item["score"]), -float(item["net_room_r"]), item["symbol"], int(item["decision_time"])),
        )
        charts = {}
        for symbol in self.settings.symbols:
            for timeframe in self.settings.timeframes:
                cache = self.cache.caches[(symbol, timeframe)]
                preview = cache.preview_value()
                charts[f"{symbol}:{timeframe}"] = {
                    "closed": [asdict(candle) for candle in cache.values()[-80:]],
                    "preview": asdict(preview) if preview else None,
                }
        return {
            "symbols": self.latest, "charts": charts, "candidates": candidates,
            "outcomes": {**self.horizon.active, **self.horizon.completed},
            "events": list(reversed(self.ledger.event_ledger[-200:])),
            "zones": {key: asdict(zone) for key, zone in self.ledger.zones.items()},
            "feed_quality": self.cache.quality(), "reconnects": self.reconnects,
        }
