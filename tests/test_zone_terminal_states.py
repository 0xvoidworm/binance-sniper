from types import SimpleNamespace

from binance_sniper.domain import Candle, Zone, ZoneState
from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache


def settings():
    return SimpleNamespace(symbols=('BTCUSDT',),timeframes=('4h','1h','15m','5m'),four_bar_horizon=4,trigger_min_body_fraction=.45)


def candle(i,low=95,high=105):
    return Candle('BTCUSDT','5m',i*300000,(i+1)*300000-1,100,high,low,101,10,True)


def scanner_with_zone(tmp_path,expires=10_000_000):
    scanner=Scanner(MtfCache(settings().symbols,settings().timeframes),settings(),tmp_path)
    zone=Zone('BTCUSDT','BULLISH',95,105,0,expires,'impulse',90,zone_id='zone')
    scanner.ledger.upsert(zone,'zone');scanner.ledger.transition('zone',ZoneState.ARMED,'ready',0)
    return scanner,zone


def test_invalidation_is_terminal_and_cannot_regress(tmp_path):
    scanner,zone=scanner_with_zone(tmp_path)
    scanner._advance_zone('BTCUSDT','BULLISH',.5,{'end':110,'quality':.5},zone,[candle(1,low=89)])
    assert zone.state==ZoneState.INVALIDATED
    assert not scanner.ledger.transition('zone',ZoneState.ARMED,'illegal',999)


def test_expiry_is_terminal(tmp_path):
    scanner,zone=scanner_with_zone(tmp_path,expires=300000)
    scanner._advance_zone('BTCUSDT','BULLISH',.5,{'end':110,'quality':.5},zone,[candle(2)])
    assert zone.state==ZoneState.EXPIRED
