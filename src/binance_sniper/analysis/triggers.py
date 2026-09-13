from __future__ import annotations
from binance_sniper.domain import Candle, Zone, ZoneState


def detect(candles: list[Candle], zone: Zone | None, direction: str, min_body: float = .45) -> dict | None:
    if len(candles) < 3 or not zone or zone.state != ZoneState.TOUCHED:
        return None
    bar, previous = candles[-1], candles[-2]
    touched = bar.low <= zone.upper and bar.high >= zone.lower
    reclaim = bar.close > zone.upper if direction == "BULLISH" else bar.close < zone.lower
    micro_break = bar.close > previous.high if direction == "BULLISH" else bar.close < previous.low
    body = abs(bar.close - bar.open) / max(bar.high - bar.low, 1e-12)
    if touched and reclaim and micro_break and body >= min_body:
        return {"time": bar.close_time, "price": bar.close, "body_fraction": body, "reason_code": "M5_RECLAIM_MICRO_BREAK"}
    return None
