import json
from types import SimpleNamespace

from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.replay import SYMBOL, fixture_bars, replay_settings
from binance_sniper.research.recorder import ResearchRecorder


def test_live_recorder_restart_dedupe_separation_and_unknown_funding(tmp_path,monkeypatch):
    data,index=fixture_bars()
    settings=replay_settings()
    cache=MtfCache(settings.symbols,settings.timeframes)
    initial=data['5m'][index].open_time
    for tf,candles in data.items():
        for bar in candles:
            if bar.close_time<initial:
                cache.caches[(SYMBOL,tf)]._accept(bar)
    scanner=Scanner(cache,settings,persist=False)
    scanner.evaluate(SYMBOL)
    runtime=SimpleNamespace(cache=cache,scanner=scanner)
    monkeypatch.setattr('time.time',lambda:initial/1000)
    path=tmp_path/'research.sqlite3'
    recorder=ResearchRecorder(path,settings,initial)
    recorder.capture(runtime,initial)
    assert recorder.snapshot['reports'][0]['report']['signal_count']==0
    for bar in data['5m'][index:index+5]:
        cache.caches[(SYMBOL,'5m')]._accept(bar)
        scanner.evaluate(SYMBOL)
        recorder.capture(runtime,bar.close_time+65000)
    report=recorder.snapshot['reports'][0]['report']
    assert report['signal_count']==1
    assert report['groups'][0]['base']['n']==0
    assert report['groups'][0]['before_funding']['n']==1
    first=recorder.store.db.execute('SELECT payload FROM outcomes').fetchone()[0]
    event=recorder.store.event_view(recorder.run,next(iter(recorder.seen)))
    assert event['mode']=='LIVE'
    assert len(event['bars'])<=54
    assert max(b['close_time'] for b in event['bars'])<=event['signal']['decision_time']+1200000
    recorder.close()
    again=ResearchRecorder(path,settings,initial+1800000)
    again.capture(runtime,initial+1800000)
    assert again.store.db.execute('SELECT count(*) FROM signals').fetchone()[0]==1
    assert again.store.db.execute('SELECT payload FROM outcomes').fetchone()[0]==first
    assert not again.pending
    again.close()


def test_research_get_routes_are_read_only_and_bound_input():
    from fastapi.testclient import TestClient
    from binance_sniper.web import build_app
    from test_dashboard import State
    client=TestClient(build_app(State()))
    assert client.get('/api/research').json()['status']=='UNAVAILABLE'
    assert client.post('/api/research').status_code==405
    assert client.get('/api/research/event',params={'run_id':'x','candidate_id':'a'}).status_code==404
    assert client.get('/api/research/event',params={'run_id':'x'*101,'candidate_id':'a'}).status_code==422
    for asset in ('research-panel.js','research.css'):
        assert client.get('/assets/'+asset).status_code==200
