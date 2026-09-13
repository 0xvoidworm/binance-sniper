from __future__ import annotations

import math
import tempfile
from pathlib import Path
from types import SimpleNamespace

from binance_sniper.analysis.impulse import detect as detect_impulse
from binance_sniper.domain import Candle
from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache


SYMBOL = "BTCUSDT"
END_TIME = 1_008_000_000_000
INTERVALS = {"4h": 14_400_000, "1h": 3_600_000, "15m": 900_000, "5m": 300_000}


def _wave(timeframe: str, count: int, trend: float, amplitude: float) -> list[Candle]:
    step = INTERVALS[timeframe]
    start = END_TIME - count * step
    result = []
    for index in range(count):
        close = 100 + index * trend + amplitude * math.sin(2 * math.pi * index / 8)
        result.append(Candle(SYMBOL, timeframe, start + index * step, start + (index + 1) * step - 1, close - 1, close + 1, close - 1, close, 10, True))
    return result


def fixture_bars() -> tuple[dict[str, list[Candle]], int]:
    h4 = _wave("4h", 60, 0.5, 3.0)
    # H1 ends exactly at the impulse confirmation. No later H1 candle is
    # available when the pullback/trigger decision is made.
    h1 = _wave("1h", 57, 0.35, 2.5)
    impulse = detect_impulse(h1, "BULLISH", 2)
    if impulse is None:
        raise RuntimeError("REPLAY_IMPULSE_FIXTURE_INVALID")

    step = INTERVALS["15m"]
    start = END_TIME - 20 * step
    m15 = [Candle(SYMBOL, "15m", start + i * step, start + (i + 1) * step - 1, 118.8, 120, 118, 119, 10, True) for i in range(32)]
    pullback_index = next(index for index, candle in enumerate(m15) if candle.close_time > impulse["end_time"])
    pullback = m15[pullback_index]
    m15[pullback_index] = Candle(SYMBOL, "15m", pullback.open_time, pullback.close_time, 119, 120, 118, 118.7, 10, True)

    step = INTERVALS["5m"]
    start = END_TIME - 12 * step
    m5 = [Candle(SYMBOL, "5m", start + i * step, start + (i + 1) * step - 1, 116, 116.2, 115.9, 116.1, 10, True) for i in range(30)]
    created_at = m15[pullback_index].close_time
    trigger_index = next(index for index, candle in enumerate(m5) if candle.open_time > created_at)
    previous = m5[trigger_index - 1]
    m5[trigger_index - 1] = Candle(SYMBOL, "5m", previous.open_time, previous.close_time, 115.8, 116, 115.5, 115.9, 10, True)
    trigger = m5[trigger_index]
    m5[trigger_index] = Candle(SYMBOL, "5m", trigger.open_time, trigger.close_time, 115.3, 117, 115.2, 116.8, 10, True)
    outcome_values = [(117, 118, 116), (118, 119, 116.5), (120, 121, 117), (120, 120.5, 118)]
    for offset, (close, high, low) in enumerate(outcome_values, start=1):
        candle = m5[trigger_index + offset]
        open_ = 116.8 if offset == 1 else outcome_values[offset - 2][0]
        m5[trigger_index + offset] = Candle(SYMBOL, "5m", candle.open_time, candle.close_time, open_, high, low, close, 10, True)
    return {"4h": h4, "1h": h1, "15m": m15, "5m": m5}, trigger_index


def _row(candle: Candle) -> list:
    return [candle.open_time, str(candle.open), str(candle.high), str(candle.low), str(candle.close), str(candle.volume), candle.close_time, "0", 1, "0", "0", "0"]


def replay_settings():
    return SimpleNamespace(
        symbols=(SYMBOL,), timeframes=("4h", "1h", "15m", "5m"), pivot_left_right=2,
        pullback_min_depth=0.25, pullback_max_depth=0.70, pullback_max_bars=12,
        zone_atr_width=0.20, trigger_min_body_fraction=0.45, four_bar_horizon=4,
    )


def run_replay(state_dir: Path, future_mutator=None) -> dict:
    bars, trigger_index = fixture_bars()
    if future_mutator is not None:
        future_mutator(bars["5m"], trigger_index)
    settings = replay_settings()
    cache = MtfCache(settings.symbols, settings.timeframes)
    trigger_time = bars["5m"][trigger_index].open_time
    cache.warmup(SYMBOL, "4h", [_row(candle) for candle in bars["4h"]], trigger_time)
    cache.warmup(SYMBOL, "1h", [_row(candle) for candle in bars["1h"]], trigger_time)
    causal_m15 = [candle for candle in bars["15m"] if candle.close_time <= trigger_time]
    cache.warmup(SYMBOL, "15m", [_row(candle) for candle in causal_m15], trigger_time)
    initial_m5 = bars["5m"][:trigger_index]
    cache.warmup(SYMBOL, "5m", [_row(candle) for candle in initial_m5], trigger_time)
    scanner = Scanner(cache, settings, state_dir)
    scanner.evaluate_all()
    for candle in bars["5m"][trigger_index: trigger_index + 5]:
        cache.update_rest(SYMBOL, "5m", [_row(candle)], candle.close_time + 1)
        scanner.evaluate(SYMBOL)
    cache.mark_all_fallback()
    snapshot = scanner.snapshot()
    replay_now = bars["5m"][trigger_index + 4].close_time + 1
    snapshot["feed_quality"] = cache.quality(replay_now)
    snapshot.update(transport_state="REPLAY", ws_state="REPLAY", snapshot_at=replay_now, task_health={}, rest_telemetry={})
    return snapshot


def build_replay_snapshot() -> dict:
    with tempfile.TemporaryDirectory(prefix="binance-sniper-replay-") as directory:
        return run_replay(Path(directory))
