from __future__ import annotations
from binance_sniper.domain import Candle


def confirmed_pivots(candles: list[Candle], radius: int = 2) -> list[tuple[str, int, float, int]]:
    result = []
    for i in range(radius, len(candles) - radius):
        current = candles[i]
        left = candles[i - radius:i]
        right = candles[i + 1:i + radius + 1]
        if all(current.high > x.high for x in left + right):
            result.append(("HIGH", i, current.high, candles[i + radius].close_time))
        if all(current.low < x.low for x in left + right):
            result.append(("LOW", i, current.low, candles[i + radius].close_time))
    return result
