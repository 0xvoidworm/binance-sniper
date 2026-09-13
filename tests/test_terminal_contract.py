from binance_sniper.market.bars import ClosedBarCache
from binance_sniper.presentation import decision_view
from binance_sniper.replay import replay_settings, run_replay


def row(t):
    return [t, '100', '102', '99', '101', '10', t + 299999]


def test_rest_preview_finality_and_removal_after_close():
    cache = ClosedBarCache('BTCUSDT', '5m')
    cache.warmup([row(0), row(300000)], now=500000)
    assert cache.preview_value().closed is False
    assert cache.quality.last_preview_at == 500000
    assert all(bar.closed for bar in cache.values())
    cache.update_rest([row(300000)], now=600000)
    assert cache.preview_value() is None
    assert cache.quality.last_preview_at == 0
    cache.update_rest([row(600000)], now=700000)
    assert cache.preview_value().closed is False


def test_delayed_ws_preview_cannot_resurrect_closed_bar():
    cache = ClosedBarCache('BTCUSDT', '5m')
    cache.warmup([row(0)], now=400000)
    event = {'received_ms': 400000, 'k': {'s':'BTCUSDT','i':'5m','t':0,'T':299999,'o':'100','h':'102','l':'99','c':'101','v':'10','x':False}}
    assert cache.update_ws(event) is False
    assert cache.preview_value() is None


def test_neutral_regime_explains_first_blocker_without_claiming_h1_passed():
    decision = decision_view('NEUTRAL', 0.07, None, None, [], replay_settings(), 220)
    assert decision['stage'] == 'WAIT_H4'
    assert '0.070 < 0.25' in decision['next_condition']
    assert [gate['state'] for gate in decision['gates']] == ['WAIT','BLOCKED','BLOCKED','BLOCKED']
    assert decision['setup_id'] is None


def test_replay_frames_are_causal_and_independent(tmp_path):
    payload = run_replay(tmp_path, include_frames=True)
    frames = payload['frames']
    assert len(frames) == 6
    assert frames[0]['candidates'] == []
    assert frames[0]['symbols']['BTCUSDT']['zone']['state'] == 'ARMED'
    assert frames[1]['symbols']['BTCUSDT']['zone']['state'] == 'TRIGGERED'
    assert list(frames[1]['outcomes'].values())[0]['bars'] == 0
    assert list(frames[-1]['outcomes'].values())[0]['bars'] == 4
    assert frames[-1]['symbols']['BTCUSDT']['decision']['stage'] == 'COMPLETED'
    for frame in frames:
        assert frame['synthetic'] is True
        assert all(e['time'] <= frame['snapshot_at'] for e in frame['events'])
        assert all(bar['close_time'] <= frame['snapshot_at'] for chart in frame['charts'].values() for bar in chart['closed'])
