from pathlib import Path
from binance_sniper.analysis.lifecycle import FourBarTracker, ZoneLedger, pullback_valid
from binance_sniper.domain import Candle, Zone, ZoneState


def candle(i, close, high=None, low=None):
    return Candle('BTCUSDT','15m',i*900000,(i+1)*900000,close-1,high or close+1,low or close-2,close,1,True)


def test_zone_identity_persists_and_transition_dedupes(tmp_path):
    ledger=ZoneLedger(tmp_path/'state.json')
    zone=Zone('BTCUSDT','BULLISH',90,110,1000,2000,'impulse',80)
    ledger.upsert(zone,'impulse:1000')
    assert ledger.transition('impulse:1000',ZoneState.ARMED,'valid',1100)
    assert not ledger.transition('impulse:1000',ZoneState.ARMED,'valid',1200)
    ledger.save(); restored=ZoneLedger(tmp_path/'state.json'); restored.load()
    assert restored.zones['impulse:1000'].state == ZoneState.ARMED


def test_four_bar_tracker_completes():
    tracker=FourBarTracker(); tracker.start('c',100,90,120,0)
    for i in range(1,5): tracker.observe('c',candle(i,105,110,95),4)
    assert tracker.completed['c']['status']=='COMPLETED'
    assert tracker.completed['c']['mfe'] > 0
