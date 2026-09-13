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
    # Freeze the impulse at the first qualifying expansion after the pivot is
    # confirmed.  Using candles[-1] made the same impulse change identity on
    # every H1 close.
    for end in candles[origin[1] + radius:]:
        if end.close_time < origin[3]:
            continue
        signed_size = end.close - origin[2] if direction == "BULLISH" else origin[2] - end.close
        rng = max(end.high - end.low, 1e-12)
        quality = abs(end.close - end.open) / rng
        if signed_size <= 0 or quality < .4:
            continue
        return {
            "id": f"{candles[0].symbol}:{direction}:{candles[origin[1]].open_time}:{end.close_time}",
            "direction": direction,
            "origin": origin[2],
            "end": end.close,
            "start_time": candles[origin[1]].open_time,
            "end_time": end.close_time,
            "quality": quality,
            "confirmed_at": origin[3],
        }
    return None
