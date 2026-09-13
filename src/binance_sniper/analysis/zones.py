from __future__ import annotations
from binance_sniper.domain import Zone


def build(impulse: dict | None, direction: str, candles: list, width: float = .2, created_at: int | None = None, pullback: dict | None = None) -> Zone | None:
    if not impulse or len(candles) < 14:
        return None
    decision_time = created_at or candles[-1].close_time
    available = [candle for candle in candles if candle.close_time <= decision_time]
    if len(available) < 14:
        return None
    atr = sum(x.high - x.low for x in available[-14:]) / 14
    origin = float(impulse["origin"])
    if direction == "BULLISH":
        lower, upper, invalidation = origin - width * atr, origin + width * atr, origin - .3 * atr
    else:
        lower, upper, invalidation = origin - width * atr, origin + width * atr, origin + .3 * atr
    quality = {"atr": atr, "impulse_quality": impulse["quality"]}
    if pullback:
        quality.update({"pullback_depth": pullback["depth"], "pullback_quality": pullback["quality"]})
    return Zone(available[-1].symbol, direction, lower, upper, decision_time, decision_time + 86_400_000, impulse["id"], invalidation, quality)
