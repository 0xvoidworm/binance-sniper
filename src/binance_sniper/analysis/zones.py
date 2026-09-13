from __future__ import annotations
from binance_sniper.domain import Zone


def build(impulse: dict | None, direction: str, candles: list, width: float = .2) -> Zone | None:
    if not impulse or len(candles) < 14:
        return None
    atr = sum(x.high - x.low for x in candles[-14:]) / 14
    origin = float(impulse["origin"])
    if direction == "BULLISH":
        lower, upper, invalidation = origin - width * atr, origin + width * atr, origin - .3 * atr
    else:
        lower, upper, invalidation = origin - width * atr, origin + width * atr, origin + .3 * atr
    return Zone(candles[-1].symbol, direction, lower, upper, candles[-1].close_time, candles[-1].close_time + 86_400_000, impulse["id"], invalidation, {"atr": atr, "impulse_quality": impulse["quality"]})
