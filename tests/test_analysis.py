from binance_sniper.analysis.pivots import confirmed_pivots
from binance_sniper.analysis.ranker import rank
from binance_sniper.domain import Candle, Candidate


def candle(i, high, low, close=None):
    close = close if close is not None else (high + low) / 2
    return Candle('BTCUSDT','5m',i*300000,(i+1)*300000,low,high,low,close,1,True)


def test_pivot_is_confirmed_only_after_right_bars_exist():
    candles = [candle(i, 10, 0) for i in range(5)]
    candles[2] = candle(2, 20, 0)
    assert confirmed_pivots(candles, 2) == [("HIGH", 2, 20, 1500000)]
    assert confirmed_pivots(candles[:4], 2) == []


def test_rank_is_deterministic():
    base = dict(direction='BULLISH', decision_time=1, score=0, trend_efficiency=.5, impulse_quality=.6, zone_id='z', reason_code='x', entry_reference=1, stop_reference=0, target_reference=2)
    values = [Candidate('ETHUSDT', net_room_r=1.5, **base), Candidate('BTCUSDT', net_room_r=2, **base)]
    assert rank(values)[0].symbol == 'BTCUSDT'
