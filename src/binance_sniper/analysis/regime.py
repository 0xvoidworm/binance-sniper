from __future__ import annotations
from math import log
from binance_sniper.domain import Candle
from binance_sniper.analysis.pivots import confirmed_pivots
MIN_EFFICIENCY = 0.25


def efficiency(candles: list[Candle], lookback: int = 16) -> float:
    sample = candles[-lookback:]
    if len(sample) < 3:
        return 0.0
    path = sum(abs(sample[i].close - sample[i - 1].close) for i in range(1, len(sample)))
    return abs(sample[-1].close - sample[0].close) / path if path else 0.0


def regime(candles: list[Candle], radius: int = 2) -> tuple[str, float, list]:
    if len(candles) < 40:
        return "NEUTRAL", 0.0, []
    pivots = confirmed_pivots(candles, radius)
    highs = [p for p in pivots if p[0] == "HIGH"][-2:]
    lows = [p for p in pivots if p[0] == "LOW"][-2:]
    er = efficiency(candles)
    if len(highs) == 2 and len(lows) == 2 and highs[-1][2] > highs[-2][2] and lows[-1][2] > lows[-2][2] and er >= MIN_EFFICIENCY:
        return "BULLISH", er, pivots
    if len(highs) == 2 and len(lows) == 2 and highs[-1][2] < highs[-2][2] and lows[-1][2] < lows[-2][2] and er >= MIN_EFFICIENCY:
        return "BEARISH", er, pivots
    return "NEUTRAL", er, pivots
