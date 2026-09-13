from __future__ import annotations

from collections import deque
from dataclasses import dataclass
import math

from binance_sniper.domain import Candle


@dataclass
class Quality:
    status: str = "STARTING"
    last_closed: int = 0
    last_payload_at: int = 0
    last_valid_payload_at: int = 0
    last_preview_at: int = 0
    ws_messages: int = 0
    rest_updates: int = 0
    duplicates: int = 0
    out_of_order: int = 0
    malformed: int = 0
    gaps: int = 0
    rest_reconciles: int = 0
    transport: str = "STARTING"
    last_attempt_at: int = 0
    last_success_at: int = 0
    last_error_class: str | None = None
    reconnect_count: int = 0


INTERVAL_MS = {"5m": 300_000, "15m": 900_000, "1h": 3_600_000, "4h": 14_400_000}


class ClosedBarCache:
    def __init__(self, symbol: str, timeframe: str, capacity: int = 240):
        self.symbol, self.timeframe = symbol, timeframe
        self.items: deque[Candle] = deque(maxlen=capacity)
        self.quality = Quality()
        self._last_open = -1
        self.preview: Candle | None = None

    def _parse(self, values: list | dict, closed: bool) -> Candle | None:
        try:
            if isinstance(values, dict):
                open_time, close_time = int(values["t"]), int(values["T"])
                open_, high, low, close, volume = (float(values[key]) for key in ("o", "h", "l", "c", "v"))
            else:
                if len(values) < 7:
                    return None
                open_time, close_time = int(values[0]), int(values[6])
                open_, high, low, close, volume = (float(values[index]) for index in (1, 2, 3, 4, 5))
            interval = INTERVAL_MS[self.timeframe]
            valid = all(math.isfinite(value) for value in (open_, high, low, close, volume))
            valid = valid and open_ > 0 and min(high, low, close, volume) >= 0
            valid = valid and high >= max(open_, close) and low <= min(open_, close) and high >= low
            valid = valid and close_time > open_time and open_time % interval == 0
            if not valid:
                return None
            return Candle(self.symbol, self.timeframe, open_time, close_time, open_, high, low, close, volume, closed)
        except (KeyError, TypeError, ValueError, OverflowError):
            return None

    def _from_rest(self, row: list) -> Candle | None:
        return self._parse(row, True)

    def warmup(self, rows: list[list], now: int | None = None) -> None:
        parsed = [candle for row in rows if (candle := self._from_rest(row)) is not None]
        parsed.sort(key=lambda x: x.open_time)
    def warmup(self, rows: list[list], now: int | None = None) -> None:
        parsed = [self._from_rest(row) for row in rows if len(row) >= 7]
        parsed.sort(key=lambda x: x.open_time)
        cutoff = now if now is not None else (parsed[-1].open_time if parsed else 0)
        for candle in parsed:
            if candle.close_time <= cutoff:
                self._accept(candle)
            else:
                self.preview = candle
        self.quality.status = "REST_WARM" if self.items else "STALE"
        self.quality.transport = "REST_WARM"
        if now is not None and self.items:
            self.quality.last_success_at = now

    def update_ws(self, event: dict) -> bool:
        now = int(event.get("received_ms", 0))
        self.quality.ws_messages += 1
        self.quality.last_payload_at = now
        kline = event.get("data", {}).get("k", {}) if event.get("stream") else event.get("k", {})
        if not kline or kline.get("s") != self.symbol or kline.get("i") != self.timeframe:
            self.quality.malformed += 1
            return False
        candle = self._parse(kline, bool(kline.get("x")))
        if candle is None:
            self.quality.malformed += 1
            return False
        self.quality.last_valid_payload_at = now
        self.quality.last_success_at = now
        self.quality.status = "LIVE_WS"
        self.quality.transport = "LIVE_WS"
        if not candle.closed:
            self.preview = candle
            self.quality.last_preview_at = now
            return False
        return self._accept(candle, source="WS")

    def update_rest(self, rows: list[list], now: int) -> bool:
        changed = False
        self.quality.last_attempt_at = now
        parsed = [candle for row in rows if (candle := self._from_rest(row)) is not None]
        for candle in sorted(parsed, key=lambda x: x.open_time):
            if candle.close_time <= now:
                if candle.open_time > self._last_open:
                    changed = self._accept(candle, source="REST") or changed
            else:
                self.preview = candle
                self.quality.last_preview_at = now
        if changed:
            self.quality.rest_updates += 1
            self.quality.rest_reconciles += 1
            self.quality.last_valid_payload_at = now
            self.quality.last_success_at = now
        return changed

    def mark_ws_silent(self) -> None:
        self.quality.status = "WS_SILENT"
        self.quality.transport = "WS_SILENT"

    def mark_fallback(self) -> None:
        self.quality.status = "LIVE_REST_FALLBACK"
        self.quality.transport = "LIVE_REST_FALLBACK"

    def mark_error(self, error_class: str, now: int) -> None:
        self.quality.last_attempt_at = now
        self.quality.last_error_class = error_class
        if self.quality.last_success_at == 0:
            self.mark_stale()

    def mark_ws_live(self) -> None:
        self.quality.status = "LIVE_WS"
        self.quality.transport = "LIVE_WS"

    def mark_stale(self) -> None:
        self.quality.status = "STALE"
        self.quality.transport = "STALE"

    def _accept(self, candle: Candle, source: str = "REST") -> bool:
        if candle.open_time < self._last_open:
            self.quality.out_of_order += 1
            return False
        if candle.open_time == self._last_open:
            self.quality.duplicates += 1
            if self.items and self.items[-1] != candle:
                self.items[-1] = candle
            return False
        if self._last_open >= 0 and candle.open_time - self._last_open >= INTERVAL_MS[self.timeframe] * 2:
            self.quality.gaps += 1
        self.items.append(candle)
        self._last_open = candle.open_time
        self.quality.last_closed = candle.close_time
        if source == "WS":
            self.quality.status = "LIVE_WS"
            self.quality.transport = "LIVE_WS"
        return True

    def values(self) -> list[Candle]:
        return list(self.items)

    def preview_value(self) -> Candle | None:
        return self.preview

    def audit(self, now: int) -> dict:
        last = self.quality.last_closed
        interval = INTERVAL_MS[self.timeframe]
        return {**vars(self.quality), "age_ms": max(0, now - last) if last else None,
                "lag_bars": max(0, (now - last) // interval - 1) if last else None}
