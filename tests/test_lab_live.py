from dataclasses import replace
from types import SimpleNamespace

from fastapi.testclient import TestClient

from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.market.bars import INTERVAL_MS
from binance_sniper.replay import replay_settings
from binance_sniper.strategy_lab.live import ShadowLab
from binance_sniper.web import build_app
from test_dashboard import State
from test_strategy_lab import candle,setup


def seed():
    feeds,nextbar,_=setup()
    offset=360*INTERVAL_MS['4h']
    def shift(bar):return replace(bar,open_time=bar.open_time+offset,close_time=bar.close_time+offset)
    feeds={tf:[shift(b) for b in bars] for tf,bars in feeds.items()}
    nextbar=shift(nextbar)
    at=feeds['5m'][-1].close_time
    for tf in ('1h','4h'):
        last=(at+1)//INTERVAL_MS[tf]-1
        feeds[tf]=[candle(tf,i,100+(i-last)*.1 if tf=='1h' else 100,
                          101+(i-last)*.1 if tf=='1h' else 101,
                          99+(i-last)*.1 if tf=='1h' else 99,
                          100+(i-last)*.1 if tf=='1h' else 100) for i in range(last-100,last+1)]
    settings=replay_settings();cache=MtfCache(settings.symbols,settings.timeframes)
    for tf,bars in feeds.items():
        for b in bars:cache.caches[('BTCUSDT',tf)]._accept(b)
    return settings,cache,nextbar,at


def test_shadow_state_signal_and_restart_are_idempotent(tmp_path):
    settings,cache,nextbar,at=seed()
    runtime=SimpleNamespace(cache=cache)
    lab=ShadowLab(tmp_path/'lab.sqlite3',settings,at)
    lab.capture(runtime,at+1)
    assert lab.engines['P15_T1_4'].areas['BTCUSDT'].state=='ARMED'
    cache.caches[('BTCUSDT','5m')]._accept(nextbar)
    lab.capture(runtime,nextbar.close_time+1)
    signals=lab.store.db.execute('SELECT count(*) FROM signals').fetchone()[0]
    assert signals>0
    state=lab.state()
    key=lab.key
    assert lab.chart('P15_T1_4','BTCUSDT')['state']['area']['state']=='TRIGGERED'
    lab.close()
    restored=ShadowLab(tmp_path/'lab.sqlite3',settings,nextbar.close_time+1000)
    assert restored.key==key
    assert restored.state()==state
    restored.capture(runtime,nextbar.close_time+15000)
    assert restored.store.db.execute('SELECT count(*) FROM signals').fetchone()[0]==signals
    assert restored.snapshot['status']=='EXPERIMENTAL_MONITOR'
    assert restored.chart('not-a-variant','BTCUSDT') is None
    assert restored.events('unknown-trial','P15_T1_4') is None
    restored.close()


def test_shadow_does_not_relabel_warmup_as_new_signals(tmp_path):
    settings,cache,_,at=seed()
    lab=ShadowLab(tmp_path/'lab.sqlite3',settings,at+1)
    lab.capture(SimpleNamespace(cache=cache),at+5000)
    assert lab.store.db.execute('SELECT count(*) FROM signals').fetchone()[0]==0
    assert all(not e.last_decision for e in lab.engines.values())
    lab.close()


def test_lab_routes_are_get_only_and_assets_present():
    client=TestClient(build_app(State()))
    assert client.get('/api/lab').json()['status']=='UNAVAILABLE'
    assert client.post('/api/lab').status_code==405
    assert client.get('/api/lab/chart',params={'variant':'P15_T1_4','symbol':'BTCUSDT'}).status_code==404
    assert client.get('/api/lab/events',params={'trial':'x','variant':'P15_T1_4'}).status_code==404
    assert client.get('/api/lab/chart',params={'variant':'x'*41,'symbol':'BTCUSDT'}).status_code==422
    assert client.get('/assets/lab-panel.js').status_code==200


def test_new_configuration_is_bound_to_experiment_signature():
    from binance_sniper.strategy_lab.runner import specification
    assert len(specification()['configuration_sha256'])==64
