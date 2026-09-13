from __future__ import annotations

from collections import defaultdict
from binance_sniper.market.bars import ClosedBarCache


class MtfCache:
    def __init__(self, symbols: tuple[str, ...], timeframes: tuple[str, ...], capacity: int = 240):
        self.caches = {(s, tf): ClosedBarCache(s, tf, capacity) for s in symbols for tf in timeframes}

    def warmup(self, symbol: str, timeframe: str, rows: list[list], now: int | None = None) -> None:
        self.caches[(symbol, timeframe)].warmup(rows, now)

    def update(self, payload: dict) -> bool:
        stream = payload.get("stream", "")
        data = payload.get("data", {})
        symbol = data.get("s") or data.get("k", {}).get("s")
        timeframe = data.get("k", {}).get("i")
        if not symbol or not timeframe or (symbol, timeframe) not in self.caches:
            return False
        return self.caches[(symbol, timeframe)].update_ws(payload)

    def mark_all_ws_silent(self) -> None:
        for cache in self.caches.values():
            cache.mark_ws_silent()

    def mark_all_fallback(self) -> None:
        for cache in self.caches.values():
            cache.mark_fallback()

    def mark_all_ws_live(self) -> None:
        for cache in self.caches.values():
            cache.mark_ws_live()

    def update_rest(self, symbol: str, timeframe: str, rows: list[list], now: int) -> bool:
        return self.caches[(symbol, timeframe)].update_rest(rows, now)

    def candles(self, symbol: str, timeframe: str):
        return self.caches[(symbol, timeframe)].values()

    def quality(self, now: int | None = None) -> dict:
        import time
        timestamp = now or int(time.time() * 1000)
        return {f"{s}:{tf}": cache.audit(timestamp) for (s, tf), cache in self.caches.items()}
