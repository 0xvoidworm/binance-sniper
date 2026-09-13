import pytest

from binance_sniper.domain import Candle
from binance_sniper.research.model import Costs, outcome, temporal_partition


def signal(direction='BULLISH'):
    return {'candidate_id':'a','symbol':'BTCUSDT','direction':direction,'decision_time':299999,
            'entry_reference':100,'stop_reference':90 if direction=='BULLISH' else 110,
            'target_reference':120 if direction=='BULLISH' else 80}


def bars(direction='BULLISH'):
    close=105 if direction=='BULLISH' else 95
    return [Candle('BTCUSDT','5m',(i+1)*300000,(i+2)*300000-1,100,106,94,close,1,True) for i in range(4)]


@pytest.mark.parametrize('direction',['BULLISH','BEARISH'])
def test_next_open_costs_and_direction(direction):
    s=signal(direction)
    s['entry_reference']=1_000_000  # signal close is not the modeled fill
    zero=outcome(s,bars(direction),Costs(0,0),[])
    net=outcome(s,bars(direction),Costs(),[])
    assert zero['entry']==100
    assert zero['net_r']==.5
    assert net['net_r']<zero['net_r']
    assert net['fee_r']>0
    assert net['exit_reason']=='FOUR_BAR_CLOSE'


def test_ambiguity_is_stop_first_with_explicit_upper_bound():
    b=bars()
    b[0]=Candle('BTCUSDT','5m',300000,599999,100,125,85,110,1,True)
    result=outcome(signal(),b,Costs(0,0),[])
    assert result['ambiguous']
    assert result['net_r']==-1
    assert result['optimistic_net_r']==2
    assert result['bars']==1


def test_gaps_missing_data_and_entry_invalidation():
    b=bars()
    assert outcome(signal(),[],funding=[])['status']=='PENDING'
    assert outcome(signal(),b[1:],funding=[])['status']=='GAP'
    assert outcome(signal(),[b[0],b[2]],funding=[])['status']=='GAP'
    assert outcome(signal(),b[:2],funding=[])['status']=='PENDING'
    b[0]=Candle('BTCUSDT','5m',300000,599999,89,91,87,90,1,True)
    assert outcome(signal(),b,funding=[])['status']=='ENTRY_INVALIDATED'


def test_gap_stop_loses_more_than_one_r():
    b=bars()
    b[1]=Candle('BTCUSDT','5m',600000,899999,80,85,79,82,1,True)
    result=outcome(signal(),b,Costs(0,0),[])
    assert result['exit_reason']=='STOP_GAP'
    assert result['net_r']==-2


def test_funding_unknown_not_net_and_directional_charge():
    assert outcome(signal(),bars())['net_r'] is None
    funding=[{'at':700000,'rate':.001,'mark':100}]
    long=outcome(signal(),bars(),Costs(0,0),funding)
    short=outcome(signal('BEARISH'),bars('BEARISH'),Costs(0,0),funding)
    assert long['net_r']==pytest.approx(.49)
    assert short['net_r']==pytest.approx(.51)


def test_boundary_funding_does_not_invent_credit():
    result=outcome(signal('BEARISH'),bars('BEARISH'),Costs(0,0),[{'at':300000,'rate':.001,'mark':100}])
    assert result['net_r']==.5
    assert result['funding_timing_ambiguous']


def test_preview_and_post_horizon_cannot_change_outcome():
    b=bars()
    base=outcome(signal(),b,Costs(0,0),[])
    b.append(Candle('BTCUSDT','5m',1500000,1799999,100,1000,1,200,1,True))
    assert outcome(signal(),b,Costs(0,0),[])==base
    preview=Candle('BTCUSDT','5m',300000,599999,100,1000,1,200,1,False)
    assert outcome(signal(),[preview],funding=[])['status']=='PENDING'


def test_temporal_partition_purges_and_embargoes_full_horizon():
    assert temporal_partition(299999,0,3000000,6000000)=='DEVELOPMENT'
    assert temporal_partition(2699999,0,3000000,6000000)=='PURGED'
    assert temporal_partition(3299999,0,3000000,6000000)=='EMBARGO'
    assert temporal_partition(4499999,0,3000000,6000000)=='HOLDOUT'
    assert temporal_partition(5699999,0,3000000,6000000)=='BOUNDARY_EXCLUDED'


def test_cost_validation():
    with pytest.raises(ValueError):
        Costs(float('nan'))
