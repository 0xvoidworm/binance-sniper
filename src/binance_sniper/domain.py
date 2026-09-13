from __future__ import annotations

from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Any


class FeedStatus(str, Enum):
    LIVE = "LIVE"
    STALE = "STALE"
    RECONNECTING = "RECONNECTING"
    DISCONNECTED = "DISCONNECTED"


class ZoneState(str, Enum):
    DETECTED = "DETECTED"
    ARMED = "ARMED"
    TOUCHED = "TOUCHED"
    TRIGGERED = "TRIGGERED"
    EXPIRED = "EXPIRED"
    INVALIDATED = "INVALIDATED"
    COMPLETED = "COMPLETED"


@dataclass(frozen=True)
class Candle:
    symbol: str
    timeframe: str
    open_time: int
    close_time: int
    open: float
    high: float
    low: float
    close: float
    volume: float
    closed: bool


@dataclass
class Zone:
    symbol: str
    direction: str
    lower: float
    upper: float
    created_at: int
    expires_at: int
    source_impulse: str
    invalidation: float
    quality: dict[str, float] = field(default_factory=dict)
    state: ZoneState = ZoneState.DETECTED
    touch_count: int = 0


@dataclass(frozen=True)
class Candidate:
    symbol: str
    direction: str
    decision_time: int
    score: float
    net_room_r: float
    trend_efficiency: float
    impulse_quality: float
    zone_id: str
    reason_code: str
    entry_reference: float
    stop_reference: float
    target_reference: float

    def json(self) -> dict[str, Any]:
        return asdict(self)
