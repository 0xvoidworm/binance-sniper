from __future__ import annotations
from binance_sniper.domain import Candle
from binance_sniper.analysis.pivots import confirmed_pivots


def detect(candles: list[Candle], direction: str, radius: int = 2) -> dict | None:
    if len(candles) < 25 or direction == "NEUTRAL":
        return None
    pivots = confirmed_pivots(candles, radius)
    relevant = [p for p in pivots if (direction == "BULLISH" and p[0] == "LOW") or (direction == "BEARISH" and p[0] == "HIGH")]
    if not relevant:
        return None
    origin = relevant[-1]
    end = candles[-1]
    size = abs(end.close - origin[2])
    rng = max(end.high - end.low, 1e-12)
    quality = abs(end.close - end.open) / rng
    if size <= 0 or quality < .4:
        return None
    return {"id": f"{candles[0].symbol}:{origin[3]}", "origin": origin[2], "end": end.close, "quality": quality, "confirmed_at": origin[3]}
