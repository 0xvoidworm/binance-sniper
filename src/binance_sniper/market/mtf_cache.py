from __future__ import annotations

from collections import defaultdict
from binance_sniper.market.bars import ClosedBarCache


class MtfCache:
    def __init__(self, symbols: tuple[str, ...], timeframes: tuple[str, ...], capacity: int = 240):
        self.caches = {(s, tf): ClosedBarCache(s, tf, capacity) for s in symbols for tf in timeframes}

    def warmup(self, symbol: str, timeframe: str, rows: list[list]) -> None:
        self.caches[(symbol, timeframe)].warmup(rows)

    def update(self, payload: dict) -> bool:
        stream = payload.get("stream", "")
        data = payload.get("data", {})
        symbol = data.get("s") or data.get("k", {}).get("s")
        timeframe = data.get("k", {}).get("i")
        if not symbol or not timeframe or (symbol, timeframe) not in self.caches:
            return False
        return self.caches[(symbol, timeframe)].update_ws(payload)

    def candles(self, symbol: str, timeframe: str):
        return self.caches[(symbol, timeframe)].values()

    def quality(self) -> dict:
        return {f"{s}:{tf}": vars(cache.quality) for (s, tf), cache in self.caches.items()}
