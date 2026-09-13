from binance_sniper.analysis.lifecycle import FourBarTracker
from binance_sniper.domain import Candle


def bar(i, high, low, close):
    return Candle('BTCUSDT','5m',i*300000,(i+1)*300000,100,high,low,close,1,True)


def test_long_short_directional_results_and_ambiguity():
    long_tracker=FourBarTracker(); long_tracker.start('long',100,90,120,0)
    for i in range(1,5): result=long_tracker.observe('long',bar(i,110,95,105),4)
    assert result['direction']=='LONG' and result['mfe']==1 and result['mae']==.5
    assert result['final_r']==.5 and result['status']=='COMPLETED'
    assert long_tracker.observe('long',bar(5,150,50,100),4)['status']=='COMPLETED'

    short_tracker=FourBarTracker(); short_tracker.start('short',100,110,80,0)
    for i in range(1,5): result=short_tracker.observe('short',bar(i,105,90,95),4)
    assert result['direction']=='SHORT' and result['mfe']==1 and result['mae']==.5
    assert result['final_r']==.5

    ambiguous=FourBarTracker(); ambiguous.start('amb',100,90,110,0)
    result=ambiguous.observe('amb',bar(1,110,90,100),4)
    assert result['first_hit']['kind']=='INTRABAR_AMBIGUOUS'


def test_active_tracker_persists_and_dedupes_same_bar(tmp_path):
    path=tmp_path/'four-bar.json'
    tracker=FourBarTracker(path); assert tracker.start('c',100,90,120,0,'BTCUSDT','z')
    tracker.observe('c',bar(1,105,98,102),4)
    restored=FourBarTracker(path); restored.load()
    restored.observe('c',bar(1,150,50,100),4)
    assert restored.active['c']['bars']==1
    for i in range(2,5): restored.observe('c',bar(i,110,95,105),4)
    assert restored.completed['c']['bars']==4
    assert not restored.start('c',100,90,120,0)


def test_zero_risk_candidate_is_rejected():
    tracker=FourBarTracker()
    assert tracker.start('zero',100,100,120,0) is False
