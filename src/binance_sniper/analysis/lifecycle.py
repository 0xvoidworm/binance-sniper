from __future__ import annotations

from dataclasses import asdict
import json
from pathlib import Path
from binance_sniper.domain import Candle, Zone, ZoneState


def zone_key(impulse: dict, created_at: int) -> str:
    return f"{impulse['id']}:{created_at}"


def pullback_valid(candles: list[Candle], impulse: dict | None, direction: str, min_depth: float, max_depth: float, max_bars: int) -> bool:
    if not impulse or len(candles) < 2 or direction == "NEUTRAL":
        return False
    span = abs(float(impulse["end"]) - float(impulse["origin"]))
    if span <= 0:
        return False
    recent = [candle for candle in candles if candle.open_time >= impulse["end_time"]][-max_bars:]
    if not recent:
        return False
    extreme = min(c.low for c in recent) if direction == "BULLISH" else max(c.high for c in recent)
    depth = ((float(impulse["end"]) - extreme) / span if direction == "BULLISH" else (extreme - float(impulse["end"])) / span)
    return min_depth <= depth <= max_depth and (recent[-1].close > float(impulse["origin"]) if direction == "BULLISH" else recent[-1].close < float(impulse["origin"]))


class ZoneLedger:
    def __init__(self, path: Path):
        self.path = path
        self.zones: dict[str, Zone] = {}
        self.last_events: set[tuple] = set()
        self.event_ledger: list[dict] = []

    def load(self) -> None:
        if not self.path.exists():
            return
        try:
            payload = json.loads(self.path.read_text(encoding="utf-8"))
            for key, value in payload.get("zones", {}).items():
                value["state"] = ZoneState(value["state"])
                self.zones[key] = Zone(**value)
            self.event_ledger = payload.get("events", [])
        except (OSError, ValueError, TypeError, KeyError):
            self.zones = {}

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temp = self.path.with_suffix(".tmp")
        temp.write_text(json.dumps({"zones": {k: {**asdict(v), "state": v.state.value} for k, v in self.zones.items()}, "events": self.event_ledger}, sort_keys=True), encoding="utf-8")
        temp.replace(self.path)

    def upsert(self, zone: Zone, key: str) -> Zone:
        if key not in self.zones:
            self.zones[key] = zone
        return self.zones[key]

    def transition(self, key: str, state: ZoneState, reason: str, now: int) -> bool:
        zone = self.zones[key]
        legal = {ZoneState.DETECTED: {ZoneState.ARMED, ZoneState.INVALIDATED, ZoneState.EXPIRED}, ZoneState.ARMED: {ZoneState.TOUCHED, ZoneState.TRIGGERED, ZoneState.INVALIDATED, ZoneState.EXPIRED}, ZoneState.TOUCHED: {ZoneState.TRIGGERED, ZoneState.INVALIDATED, ZoneState.EXPIRED}, ZoneState.TRIGGERED: {ZoneState.COMPLETED}, ZoneState.COMPLETED: set(), ZoneState.INVALIDATED: set(), ZoneState.EXPIRED: set()}
        if state not in legal[zone.state]:
            return False
        token = (key, zone.state.value, state.value, reason)
        if zone.state == state or token in self.last_events:
            return False
        zone.state = state
        zone.touch_count += 1 if state == ZoneState.TOUCHED else 0
        self.event_ledger.append({"event_id": "|".join(map(str, token)), "zone_id": key, "from": token[1], "to": token[2], "reason_code": reason, "time": now})
        self.last_events.add(token)
        return True


class FourBarTracker:
    def __init__(self):
        self.active: dict[str, dict] = {}
        self.completed: dict[str, dict] = {}

    def start(self, candidate_id: str, entry: float, stop: float, target: float, started_at: int) -> None:
        if candidate_id not in self.active and candidate_id not in self.completed and abs(entry - stop) > 0:
            direction = "LONG" if target > entry else "SHORT"
            self.active[candidate_id] = {"candidate_id": candidate_id, "direction": direction, "entry": entry, "stop": stop, "target": target, "started_at": started_at, "bars": 0, "mfe": 0.0, "mae": 0.0, "final_r": None, "first_hit": None, "hit_1r": False, "hit_1_5r": False, "hit_2r": False, "status": "INCOMPLETE"}

    def observe(self, candidate_id: str, candle: Candle, horizon: int = 4) -> dict | None:
        item = self.active.get(candidate_id)
        if item is None and candidate_id in self.completed:
            return self.completed[candidate_id]
        if not item or candle.close_time <= item["started_at"]:
            return item
        risk = abs(item["entry"] - item["stop"])
        if risk <= 0 or item["status"] != "INCOMPLETE":
            return item
        item["bars"] += 1
        if item["direction"] == "LONG":
            favorable, adverse = (candle.high - item["entry"]) / risk, (item["entry"] - candle.low) / risk
            stop_hit, target_hit = candle.low <= item["stop"], candle.high >= item["target"]
        else:
            favorable, adverse = (item["entry"] - candle.low) / risk, (candle.high - item["entry"]) / risk
            stop_hit, target_hit = candle.high >= item["stop"], candle.low <= item["target"]
        item["mfe"] = max(item["mfe"], favorable)
        item["mae"] = max(item["mae"], adverse)
        item["hit_1r"] |= item["mfe"] >= 1
        item["hit_1_5r"] |= item["mfe"] >= 1.5
        item["hit_2r"] |= item["mfe"] >= 2
        if item["first_hit"] is None and stop_hit and target_hit:
            item["first_hit"] = {"kind": "INTRABAR_AMBIGUOUS", "bar": item["bars"], "time": candle.close_time}
        elif item["first_hit"] is None and stop_hit:
            item["first_hit"] = {"kind": "STOP", "bar": item["bars"], "time": candle.close_time}
        elif item["first_hit"] is None and target_hit:
            item["first_hit"] = {"kind": "TARGET", "bar": item["bars"], "time": candle.close_time}
        if item["bars"] >= horizon:
            item["final_r"] = ((candle.close - item["entry"]) if item["direction"] == "LONG" else (item["entry"] - candle.close)) / risk
            item["status"] = "COMPLETED"
            self.completed[candidate_id] = item
            del self.active[candidate_id]
        return item
