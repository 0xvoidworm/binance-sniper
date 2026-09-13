from binance_sniper.market.bars import ClosedBarCache
from binance_sniper.market.mtf_cache import MtfCache


def row(t, close):
    return [t, str(close-1), str(close+1), str(close-2), str(close), '10', t+299999, '0', 1, '0', '0', '0']


def test_warmup_excludes_open_candle_and_rejects_duplicate_order():
    cache = ClosedBarCache('BTCUSDT', '5m')
    cache.warmup([row(0, 10), row(300000, 11), row(600000, 12)])
    assert [x.open_time for x in cache.values()] == [0, 300000]
    event = {'stream': 'btcusdt@kline_5m', 'data': {'s':'BTCUSDT','k': {'s':'BTCUSDT','i':'5m','t':300000,'T':599999,'o':'10','h':'12','l':'9','c':'11','v':'10','x':True}}}
    assert cache.update_ws(event) is False
    assert cache.quality.duplicates == 1


def test_unconfirmed_event_never_updates_cache():
    cache = ClosedBarCache('BTCUSDT', '5m')
    event = {'stream': 'btcusdt@kline_5m', 'data': {'s':'BTCUSDT','k': {'s':'BTCUSDT','i':'5m','t':0,'T':299999,'o':'10','h':'12','l':'9','c':'11','v':'10','x':False}}}
    assert cache.update_ws(event) is False
    assert cache.values() == []
