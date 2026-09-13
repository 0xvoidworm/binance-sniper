from __future__ import annotations

from collections import deque
from dataclasses import dataclass

from binance_sniper.domain import Candle


@dataclass
class Quality:
    status: str = "DISCONNECTED"
    last_closed: int = 0
    duplicates: int = 0
    out_of_order: int = 0
    malformed: int = 0
    gaps: int = 0


INTERVAL_MS = {"5m": 300_000, "15m": 900_000, "1h": 3_600_000, "4h": 14_400_000}


class ClosedBarCache:
    def __init__(self, symbol: str, timeframe: str, capacity: int = 240):
        self.symbol, self.timeframe = symbol, timeframe
        self.items: deque[Candle] = deque(maxlen=capacity)
        self.quality = Quality()
        self._last_open = -1

    def _from_rest(self, row: list) -> Candle:
        return Candle(self.symbol, self.timeframe, int(row[0]), int(row[6]), float(row[1]), float(row[2]), float(row[3]), float(row[4]), float(row[5]), True)

    def warmup(self, rows: list[list]) -> None:
        parsed = [self._from_rest(row) for row in rows if len(row) >= 7]
        parsed.sort(key=lambda x: x.open_time)
        for candle in parsed[:-1]:
            self._accept(candle)
        self.quality.status = "LIVE" if self.items else "STALE"

    def update_ws(self, event: dict) -> bool:
        kline = event.get("data", {}).get("k", {}) if event.get("stream") else event.get("k", {})
        if not kline or kline.get("s") != self.symbol or kline.get("i") != self.timeframe:
            self.quality.malformed += 1
            return False
        try:
            candle = Candle(self.symbol, self.timeframe, int(kline["t"]), int(kline["T"]), float(kline["o"]), float(kline["h"]), float(kline["l"]), float(kline["c"]), float(kline["v"]), bool(kline["x"]))
        except (KeyError, TypeError, ValueError):
            self.quality.malformed += 1
            return False
        if not candle.closed:
            return False
        return self._accept(candle)

    def _accept(self, candle: Candle) -> bool:
        if candle.open_time < self._last_open:
            self.quality.out_of_order += 1
            return False
        if candle.open_time == self._last_open:
            self.quality.duplicates += 1
            if self.items and self.items[-1] != candle:
                self.items[-1] = candle
            return False
        if self._last_open >= 0 and candle.open_time - self._last_open > INTERVAL_MS[self.timeframe] * 2:
            self.quality.gaps += 1
        self.items.append(candle)
        self._last_open = candle.open_time
        self.quality.last_closed = candle.close_time
        self.quality.status = "LIVE"
        return True

    def values(self) -> list[Candle]:
        return list(self.items)
