from dataclasses import replace

import pytest

from binance_sniper.domain import Candle
from binance_sniper.strategy_lab.strategies import Area,Context,PatternEngine,VARIANTS,Variant,atr
from binance_sniper.strategy_lab.evaluation import DAY,select,window,validate


def candle(tf,index,o=106,h=107,l=105,c=106,v=10):
    step={'5m':300000,'15m':900000,'1h':3600000,'4h':14400000}[tf]
    return Candle('BTCUSDT',tf,index*step,(index+1)*step-1,o,h,l,c,v,True)


def setup(side=1):
    m15=[candle('15m',i,101,103,100,102) for i in range(30)]
    m15[-1]=candle('15m',29,100,110,100,109,30)
    feeds={'5m':[candle('5m',i) for i in range(90)],'15m':m15,'1h':[],'4h':[]}
    nextbar=candle('5m',90,105.5,107.8,105.5,107.6)
    if side==-1:
        def mirror(b):return replace(b,open=200-b.open,high=200-b.low,low=200-b.high,close=200-b.close)
        feeds={tf:[mirror(b) for b in bars] for tf,bars in feeds.items()}
        nextbar=mirror(nextbar)
    context={'direction':side,'reason':'CONTEXT_ALIGNED','atr5':1,'available_at':feeds['5m'][-1].close_time}
    return feeds,nextbar,context


@pytest.mark.parametrize('side',[1,-1])
def test_retracement_geometry_directional_entry_and_stop(side):
    feeds,nextbar,context=setup(side)
    engine=PatternEngine(VARIANTS[0])
    assert engine.step('BTCUSDT',feeds,context) is None
    zone=engine.areas['BTCUSDT']
    expected=(103.5,106.5) if side==1 else (93.5,96.5)
    assert (zone.lower,zone.upper)==expected
    assert zone.created_at==feeds['5m'][-1].close_time
    feeds['5m'].append(nextbar)
    signal=engine.step('BTCUSDT',feeds,context)
    assert signal is not None
    assert side*(signal['entry_reference']-signal['stop_reference'])>0
    assert side*(signal['target_reference']-signal['entry_reference'])>0
    assert signal['area']['state']=='TRIGGERED'
    assert engine.step('BTCUSDT',feeds,context) is None
    assert signal['estimated_cost_r']<=.35


def test_wrong_direction_body_rejected_and_gap_cancels_area():
    feeds,nextbar,context=setup()
    engine=PatternEngine(VARIANTS[0])
    engine.step('BTCUSDT',feeds,context)
    feeds['5m'].append(replace(nextbar,open=107.7))
    assert engine.step('BTCUSDT',feeds,context) is None
    assert engine.latest['BTCUSDT']['stage']=='WAIT_DIRECTIONAL_TRIGGER'
    feeds['5m'].append(candle('5m',93))
    assert engine.step('BTCUSDT',feeds,context) is None
    assert engine.latest['BTCUSDT']['stage']=='DATA_GAP'
    assert engine.areas['BTCUSDT'].state=='GAP_CANCELLED'


def test_context_reversal_cancels_frozen_area():
    feeds,nextbar,context=setup()
    engine=PatternEngine(VARIANTS[0]);engine.step('BTCUSDT',feeds,context)
    feeds['5m'].append(nextbar)
    engine.step('BTCUSDT',feeds,{**context,'direction':0,'reason':'H4_OPPOSED'})
    assert engine.areas['BTCUSDT'].state=='CONTEXT_CANCELLED'


def test_future_higher_timeframe_cannot_be_input():
    feeds,_,context=setup()
    feeds['1h']=[candle('1h',100)]
    with pytest.raises(ValueError,match='NONCAUSAL'):
        PatternEngine(VARIANTS[0]).step('BTCUSDT',feeds,context)


def test_cost_and_room_are_explicit_gates():
    engine=PatternEngine(VARIANTS[0])
    area=Area('x','BTCUSDT',1,99,100,98,110,0,100,0,0,98,110,True,99.99)
    b=candle('5m',1,100,100.1,99.99,100)
    assert engine._signal(area,b,.01,{})[1]=='COST_DOMINATED'
    area.structure_target=100.01;area.extreme=98
    assert engine._signal(area,b,1,{})[1]=='INSUFFICIENT_ROOM'


def test_sweep_stores_preexisting_area_without_future_pivot():
    feeds={'5m':[candle('5m',i,100,104,99,100) for i in range(30)],'15m':[],'1h':[],'4h':[]}
    feeds['5m'].append(candle('5m',30,99.3,100.7,98.7,100.5))
    signal=PatternEngine(VARIANTS[4]).step('BTCUSDT',feeds,{'direction':1,'reason':'CONTEXT_ALIGNED','atr5':1})
    assert signal['reason_code']=='SWEEP_DIRECTIONAL_RECLAIM'
    assert signal['area']['created_at']<signal['decision_time']


def test_h4_neutral_is_not_a_blanket_veto_and_revisions_recompute():
    up=[candle('1h',i,100+i,102+i,99+i,101+i) for i in range(80)]
    neutral=[candle('4h',i,100,101,99,100) for i in range(80)]
    context=Context()
    assert context.read('BTCUSDT',neutral,up)['direction']==1
    down=[candle('4h',i,200-i,201-i,198-i,199-i) for i in range(80)]
    assert context.read('BTCUSDT',down,up)['direction']==0
    assert atr(up)>0


def sample(at,value,symbol='BTCUSDT'):
    b={'status':'COMPLETED','net_r':value,'decision_time':at,'symbol':symbol,'fee_r':.1,'ambiguous':False}
    return {'base':b,'stress':{**b,'net_r':value-.1}}


def test_future_holdout_cannot_change_training_selection():
    a=[sample(i*DAY+5_000_000,.2,('BTCUSDT','ETHUSDT','SOLUSDT')[i%3]) for i in range(100)]
    b=[sample(i*DAY+5_000_000,-.1,('BTCUSDT','ETHUSDT','SOLUSDT')[i%3]) for i in range(100)]
    original=select({'a':a,'b':b},0,120*DAY)
    changed=select({'a':a+[sample(200*DAY,-999)],'b':b+[sample(200*DAY,99999)]},0,120*DAY)
    assert original==changed
    assert original['candidate']=='a'


def test_uniform_boundary_purge_and_no_false_promotion():
    assert window([sample(120*DAY-300000,1)],0,120*DAY)==[]
    assert window([sample(120*DAY+300000,1)],120*DAY,180*DAY)==[]
    report=validate({v.name:[] for v in VARIANTS},0,360*DAY)
    assert report['promotion']=='NO_PROMOTION'
    assert 'NO_CANDIDATE' in report['reasons']
    assert len(VARIANTS)==8 and sum(v.horizon==8 for v in VARIANTS)==2
