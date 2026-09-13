from binance_sniper.analysis.impulse import detect
from binance_sniper.domain import Candle
from binance_sniper.market.bars import ClosedBarCache


def bar(i, close, high=None, low=None):
    return Candle('BTCUSDT', '1h', i * 3_600_000, (i + 1) * 3_600_000, close - .2, high or close + .3, low or close - .3, close, 10, True)


def test_bullish_impulse_is_reachable_and_deterministic():
    candles = [bar(i, 100 + i * .2, high=101 + i * .2, low=99 + i * .2) for i in range(30)]
    candles[10] = bar(10, 100, high=101, low=90)
    candles[-1] = Candle('BTCUSDT', '1h', 29 * 3_600_000, 30 * 3_600_000, 100, 113, 99, 112, 10, True)
    result = detect(candles, 'BULLISH', 2)
    assert result is not None
    assert result['start_time'] < result['end_time']
    assert result['confirmed_at'] > 0
    assert result['id'] == detect(candles, 'BULLISH', 2)['id']


def test_bearish_impulse_is_reachable_and_directional():
    candles = [bar(i, 120 - i * .2, high=121 - i * .2, low=119 - i * .2) for i in range(30)]
    candles[10] = Candle('BTCUSDT', '1h', 10 * 3_600_000, 11 * 3_600_000, 120, 130, 119, 120, 10, True)
    candles[-1] = Candle('BTCUSDT', '1h', 29 * 3_600_000, 30 * 3_600_000, 112, 113, 107, 108, 10, True)
    result = detect(candles, 'BEARISH', 2)
    assert result is not None
    assert result['direction'] == 'BEARISH'
    assert result['end'] < result['origin']


def test_malformed_rest_rows_are_filtered_without_crash():
    cache = ClosedBarCache('BTCUSDT', '5m')
    valid = [0, '100', '101', '99', '100.5', '10', '299999']
    malformed = [300000, 'not-a-price', '101', '99', '100', '10', '599999']
    cache.warmup([valid, malformed], now=400000)
    assert len(cache.values()) == 1
    assert cache.quality.malformed >= 1
