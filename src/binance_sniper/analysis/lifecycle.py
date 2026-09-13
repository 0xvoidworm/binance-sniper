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
    recent = candles[-max_bars:]
    extreme = min(c.low for c in recent) if direction == "BULLISH" else max(c.high for c in recent)
    depth = ((float(impulse["end"]) - extreme) / span if direction == "BULLISH" else (extreme - float(impulse["end"])) / span)
    return min_depth <= depth <= max_depth and (recent[-1].close > float(impulse["origin"]) if direction == "BULLISH" else recent[-1].close < float(impulse["origin"]))


class ZoneLedger:
    def __init__(self, path: Path):
        self.path = path
        self.zones: dict[str, Zone] = {}
        self.last_events: set[tuple] = set()

    def load(self) -> None:
        if not self.path.exists():
            return
        try:
            payload = json.loads(self.path.read_text(encoding="utf-8"))
            for key, value in payload.get("zones", {}).items():
                value["state"] = ZoneState(value["state"])
                self.zones[key] = Zone(**value)
        except (OSError, ValueError, TypeError, KeyError):
            self.zones = {}

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temp = self.path.with_suffix(".tmp")
        temp.write_text(json.dumps({"zones": {k: {**asdict(v), "state": v.state.value} for k, v in self.zones.items()}}, sort_keys=True), encoding="utf-8")
        temp.replace(self.path)

    def upsert(self, zone: Zone, key: str) -> Zone:
        if key not in self.zones:
            self.zones[key] = zone
        return self.zones[key]

    def transition(self, key: str, state: ZoneState, reason: str, now: int) -> bool:
        zone = self.zones[key]
        token = (key, zone.state.value, state.value, reason)
        if zone.state == state or token in self.last_events:
            return False
        zone.state = state
        self.last_events.add(token)
        return True


class FourBarTracker:
    def __init__(self):
        self.active: dict[str, dict] = {}

    def start(self, candidate_id: str, entry: float, stop: float, target: float, started_at: int) -> None:
        self.active.setdefault(candidate_id, {"entry": entry, "stop": stop, "target": target, "started_at": started_at, "bars": 0, "mfe": 0.0, "mae": 0.0, "final_r": None, "hit_1r": False, "hit_1_5r": False, "hit_2r": False, "status": "INCOMPLETE"})

    def observe(self, candidate_id: str, candle: Candle, horizon: int = 4) -> dict | None:
        item = self.active.get(candidate_id)
        if not item or candle.close_time <= item["started_at"]:
            return item
        risk = abs(item["entry"] - item["stop"])
        item["bars"] += 1
        item["mfe"] = max(item["mfe"], (candle.high - item["entry"]) / risk, (item["entry"] - candle.low) / risk)
        item["mae"] = max(item["mae"], (item["entry"] - candle.low) / risk, (candle.high - item["entry"]) / risk)
        item["hit_1r"] |= item["mfe"] >= 1
        item["hit_1_5r"] |= item["mfe"] >= 1.5
        item["hit_2r"] |= item["mfe"] >= 2
        if item["bars"] >= horizon:
            item["final_r"] = (candle.close - item["entry"]) / risk
            item["status"] = "COMPLETED"
        return item
