from dataclasses import replace
from types import SimpleNamespace

import pytest
from fastapi.testclient import TestClient

from binance_sniper.domain import Candle
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.reference.engine import DAY, HOUR, M5, Rules, evaluate_session, specification
from binance_sniper.reference.live import ReferenceMonitor, MarketView
from binance_sniper.reference.replay import build_reference_replay
from binance_sniper.reference.study import metrics
from binance_sniper.web import build_app


def bar(at, o=100, h=101, l=99, c=100, closed=True):
    return Candle('BTCUSDT', '5m', at, at+M5-1, o, h, l, c, 10, closed)


def range_bars():
    return [bar(at) for at in range(3*HOUR, 6*HOUR, M5)]


def long_session(count=144):
    return range_bars()+[bar(6*HOUR+i*M5, 100 if i == 0 else 102+i*.01,
                            102.5+i*.01, 99.5 if i == 0 else 101.5+i*.01, 102.2+i*.01)
                         for i in range(count)]


def test_rules_match_frozen_single_reference():
    rules=Rules()
    assert (rules.range_start_hour,rules.range_end_hour,rules.exit_hour)==(3,6,18)
    assert specification()['take_profit'] is None
    assert specification()['trailing'] is False
    for kwargs in ({'exit_hour':6},{'range_start_hour':3.5},{'fee_bps':float('nan')},{'slippage_bps':-1}):
        with pytest.raises(ValueError): Rules(**kwargs)


def test_range_requires_all_closed_bars_and_never_future_or_preview():
    bars=range_bars()
    assert evaluate_session('BTCUSDT',bars,0,3*HOUR)['status']=='BUILDING_RANGE'
    assert evaluate_session('BTCUSDT',bars,0,6*HOUR)['status']=='ARMED'
    assert evaluate_session('BTCUSDT',bars[:-1],0,6*HOUR)['status']=='DATA_GAP'
    future=bar(6*HOUR,100,200,1,100)
    base=evaluate_session('BTCUSDT',bars,0,6*HOUR)
    assert evaluate_session('BTCUSDT',bars+[future],0,6*HOUR)==base
    preview=replace(future,closed=False)
    assert evaluate_session('BTCUSDT',bars+[preview],0,6*HOUR+M5)['entry'] is None


def test_long_is_held_beyond_four_and_eight_bars_without_target():
    bars=long_session(20)
    result=evaluate_session('BTCUSDT',bars,0,bars[-1].close_time+1)
    assert result['status']=='HOLDING'
    assert result['held_bars']==20
    assert result['direction']=='LONG' and result['stop']==99
    assert result['target'] is None and result['trailing'] is False
    assert result['net_r'] is None and result['order_sent'] is False


def test_short_symmetry_and_opposite_stop():
    bars=[replace(b,open=200-b.open,high=200-b.low,low=200-b.high,close=200-b.close) for b in long_session(20)]
    result=evaluate_session('BTCUSDT',bars,0,bars[-1].close_time+1)
    assert result['status']=='HOLDING' and result['direction']=='SHORT'
    assert result['stop']==101 and result['entry']<99


def test_exit_end_of_session_not_a_four_bar_horizon():
    result=evaluate_session('BTCUSDT',long_session(),0,DAY,funding=[])
    assert result['status']=='CLOSED' and result['reason']=='SESSION_END'
    assert result['held_bars']==144
    assert result['exit_at']==18*HOUR-1
    assert result['net_r']==result['after_cost_r']


def test_stop_gap_and_one_position_per_day():
    bars=long_session()
    bars[38]=bar(6*HOUR+2*M5,97,98,96,97)
    result=evaluate_session('BTCUSDT',bars,0,DAY,funding=[])
    assert result['reason']=='STOP_GAP' and result['exit']<97
    assert result['net_r'] < -1
    assert result['exit_at']==6*HOUR+2*M5


def test_gap_first_entry_is_not_filled_at_old_range():
    bars=range_bars()+[bar(6*HOUR,105,106,104,105.5)]
    result=evaluate_session('BTCUSDT',bars,0,6*HOUR+M5)
    assert result['entry']>105 and result['entry_precision']=='BOUNDARY'


def test_both_sides_ambiguous_inside_open_is_not_a_made_up_trade():
    bars=range_bars()+[bar(6*HOUR,100,102,98,100)]
    result=evaluate_session('BTCUSDT',bars,0,6*HOUR+M5)
    assert result['status']=='AMBIGUOUS' and result['entry'] is None
    assert metrics([result])['closed']==0
    assert metrics([result])['status_counts']['AMBIGUOUS']==1


def test_open_outside_range_establishes_first_side_before_opposite_touch():
    bars=range_bars()+[bar(6*HOUR,102,103,98,100)]
    result=evaluate_session('BTCUSDT',bars,0,6*HOUR+M5)
    assert result['direction']=='LONG' and result['reason']=='STOP'


def test_missing_post_entry_bar_is_unresolved_not_session_profit():
    bars=long_session()
    del bars[40]
    result=evaluate_session('BTCUSDT',bars,0,DAY)
    assert result['status']=='DATA_GAP' and result['unresolved_position'] is True
    assert result['net_r'] is None


def test_stop_precedes_session_exit_on_final_bar():
    bars=long_session()
    bars[-1]=bar(18*HOUR-M5,102,103,98,100)
    result=evaluate_session('BTCUSDT',bars,0,DAY)
    assert result['reason']=='STOP'


def test_no_breakout_and_invalid_or_revised_data():
    bars=range_bars()+[bar(t,100,100.5,99.5,100) for t in range(6*HOUR,18*HOUR,M5)]
    assert evaluate_session('BTCUSDT',bars,0,DAY)['status']=='NO_BREAKOUT'
    assert evaluate_session('BTCUSDT',bars+[replace(bars[0],high=110)],0,DAY)['status']=='DATA_GAP'
    flat=[bar(t,100,100,100,100) for t in range(3*HOUR,6*HOUR,M5)]
    assert evaluate_session('BTCUSDT',flat,0,6*HOUR)['status']=='INVALID_RANGE'


def test_funding_missing_remains_unknown_and_charges_reduce_net():
    bars=long_session()
    unknown=evaluate_session('BTCUSDT',bars,0,DAY)
    known=evaluate_session('BTCUSDT',bars,0,DAY,funding=[{'at':8*HOUR,'rate':.001,'mark':102}])
    assert unknown['net_r'] is None and unknown['after_cost_r'] is not None
    assert known['funding_r']>0 and known['net_r']<unknown['after_cost_r']


def runtime_seed(bars,now):
    cache=MtfCache(('BTCUSDT',),('5m',),capacity=400)
    rows=[[b.open_time,b.open,b.high,b.low,b.close,b.volume,b.close_time] for b in bars]
    cache.warmup('BTCUSDT','5m',rows,now)
    return SimpleNamespace(cache=cache),SimpleNamespace(symbols=('BTCUSDT',))


def test_persistence_restart_and_backfill_are_not_new_live_orders(tmp_path):
    runtime,settings=runtime_seed(long_session(),DAY)
    monitor=ReferenceMonitor(tmp_path,settings,DAY)
    monitor.capture(runtime,DAY-1)
    value=monitor.snapshot['sessions']['BTCUSDT']
    assert value['origin']=='RECONSTRUCTED' and value['status']=='CLOSED'
    expected=value['after_cost_r']
    monitor.close()
    restored=ReferenceMonitor(tmp_path,settings,DAY+100)
    restored.capture(runtime,DAY-1)
    assert restored.snapshot['sessions']['BTCUSDT']['after_cost_r']==expected
    assert restored.db.execute('SELECT count(*) FROM sessions').fetchone()[0]==1
    restored.close()


def test_completed_result_is_immutable_but_revision_is_flagged(tmp_path):
    bars=long_session()
    runtime,settings=runtime_seed(bars,DAY)
    monitor=ReferenceMonitor(tmp_path,settings,DAY)
    monitor.capture(runtime,DAY-1)
    expected=monitor.snapshot['sessions']['BTCUSDT']['after_cost_r']
    changed=bars[:-1]+[replace(bars[-1],close=bars[-1].open)]
    revised,_=runtime_seed(changed,DAY)
    monitor.capture(revised,DAY-1)
    result=monitor.snapshot['sessions']['BTCUSDT']
    assert result['after_cost_r']==expected and result['revision_warning'] is True
    monitor.close()


def test_reference_replay_and_routes_are_causal_and_get_only():
    replay=build_reference_replay()
    assert len(replay['frames'])==6
    results=[next(iter(f['reference']['sessions'].values())) for f in replay['frames']]
    assert [r['status'] for r in results]==['BUILDING_RANGE','ARMED','HOLDING','HOLDING','HOLDING','CLOSED']
    assert results[4]['held_bars']>8
    for frame in replay['frames']:
        assert frame['synthetic'] is True
        assert all(b['close_time']<frame['snapshot_at'] for b in next(iter(frame['charts'].values()))['closed'])
    state=SimpleNamespace(clients=set(),account={})
    client=TestClient(build_app(state))
    assert client.get('/api/reference/replay').status_code==200
    assert client.post('/api/reference').status_code==405
    assert client.post('/api/reference/replay').status_code==405
    assert client.get('/api/reference').json()['status']=='UNAVAILABLE'


def test_market_view_does_not_run_legacy_scanner():
    runtime,settings=runtime_seed(range_bars(),6*HOUR)
    view=MarketView(runtime.cache,settings)
    view.evaluate('BTCUSDT')
    payload=view.snapshot()
    assert payload['legacy_scanner']=='RETIRED'
    assert payload['candidates']==[] and payload['zones']=={}


def test_recorded_chart_is_bounded_and_preserves_session_identity(tmp_path):
    runtime,settings=runtime_seed(long_session(),DAY)
    monitor=ReferenceMonitor(tmp_path,settings,DAY)
    monitor.capture(runtime,DAY-1)
    recorded=monitor.chart('BTCUSDT',0)
    assert recorded['historical'] is True and recorded['synthetic'] is False
    session=recorded['reference']['sessions']['BTCUSDT']
    assert session['id']=='BTCUSDT:0' and session['reason']=='SESSION_END'
    assert len(recorded['charts']['BTCUSDT:5m']['closed'])==180
    assert monitor.chart('UNKNOWN',0) is None
    assert monitor.chart('BTCUSDT',1) is None
    assert monitor.chart('BTCUSDT',0,'unregistered-dataset') is None
    client=TestClient(build_app(SimpleNamespace(reference_chart=monitor.chart)))
    assert client.get('/api/reference/session',params={'symbol':'BTCUSDT','day':0}).status_code==200
    assert client.get('/api/reference/session',params={'symbol':'BTCUSDT','day':-1}).status_code==422
    assert client.post('/api/reference/session',params={'symbol':'BTCUSDT','day':0}).status_code==405
    monitor.close()


def test_runtime_registers_only_reference_loop_not_old_pattern_loops():
    from pathlib import Path
    text=Path('src/binance_sniper/app.py').read_text(encoding='utf-8')
    assert 'self._spawn("range-reference", self.reference_loop)' in text
    assert 'self._spawn("strategy-lab", self.lab_loop)' not in text
    assert 'self._spawn("research", self.research_loop)' not in text
