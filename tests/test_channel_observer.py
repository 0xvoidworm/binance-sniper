from dataclasses import replace
from types import SimpleNamespace

from fastapi.testclient import TestClient
import pytest

from binance_sniper.domain import Candle
from binance_sniper.round01.data import H4
from binance_sniper.round01.observer import ChannelObserver, SYMBOLS, WARMUP
from binance_sniper.web import build_app


def bar(i, s="BTCUSDT", **kwargs):
    values=dict(symbol=s,timeframe="4h",open_time=i*H4,close_time=(i+1)*H4-1,
                open=100.,high=102.,low=98.,close=100.,volume=1.,closed=True)
    values.update(kwargs)
    return Candle(**values)


class Feed:
    def __init__(self, bars):
        self.bars=bars
        self.age=0
        self.error=None
        self.preview=None
    def values(self): return self.bars
    def preview_value(self): return self.preview
    def audit(self, now):
        return {"success_age_ms": self.age, "preview_age_ms": self.age,
                "last_error_class": self.error, "transport": "LIVE_REST_FALLBACK"}


def cache():
    return SimpleNamespace(caches={(s,"4h"):Feed([bar(i,s) for i in range(WARMUP)]) for s in SYMBOLS})


def capture(obs, data, i=220): obs.capture(data,i*H4+100)


def test_warmup_does_not_generate_past_trades_or_pending(tmp_path):
    data=cache();obs=ChannelObserver(tmp_path/"o.db",220*H4+100)
    capture(obs,data)
    assert all(e.position is None and e.pending is None and e.trades==[] for es in obs.engines.values() for e in es)
    rows=obs.snapshot(data,220*H4+100)["rows"]
    assert len(rows)==4 and all(r["warmup_bars"]==220 and r["observed_closed_bars"]==0 for r in rows)
    assert all(r["net_complete"] is None and not r["cost_complete_for_model"] for r in rows)
    obs.close()


def test_signal_next_open_stop_restart_and_idempotence(tmp_path):
    data=cache();path=tmp_path/"o.db";obs=ChannelObserver(path,220*H4+100);capture(obs,data)
    for i, values in [(220,dict(close=106.,high=107.)),(221,dict(open=106.,close=107.,high=110.,low=105.)),
                      (222,dict(open=107.,close=92.,high=108.,low=90.))]:
        for s in SYMBOLS: data.caches[s,"4h"].bars.append(bar(i,s,**values))
        capture(obs,data,i+1)
        if i==220: assert obs.engines["BTCUSDT"][0].pending and obs.engines["BTCUSDT"][0].position is None
        if i==221: assert obs.engines["BTCUSDT"][0].position is not None
    before=[e.state() for es in obs.engines.values() for e in es]
    count=obs.db.execute("SELECT COUNT(*) FROM observer_inputs").fetchone()[0]
    capture(obs,data,223)
    assert count==obs.db.execute("SELECT COUNT(*) FROM observer_inputs").fetchone()[0]==446
    assert len(obs.engines["BTCUSDT"][0].trades)==1
    snap=obs.snapshot(data,223*H4+100)
    assert snap["rows"][0]["trades"][0]["origin"]=="CLOSED_BAR_MODEL"
    assert snap["rows"][0]["trades"][0]["net_complete"] is None
    obs.close();obs=ChannelObserver(path,224*H4)
    assert before==[e.state() for es in obs.engines.values() for e in es]
    assert obs.started_at==220*H4+100
    obs.close()


def test_preview_is_never_consumed_stale_gap_revision_fail_closed(tmp_path):
    data=cache();obs=ChannelObserver(tmp_path/"o.db",220*H4+100);capture(obs,data)
    f=data.caches["BTCUSDT","4h"]
    f.preview=bar(220,close=200.,high=201.,closed=False)
    capture(obs,data);assert obs.engines["BTCUSDT"][0].pending is None
    f.age=100_000;capture(obs,data)
    assert obs.status["BTCUSDT"]=="PAUSED_STALE_DATA"
    f.age=0;f.bars.append(bar(221));capture(obs,data,222)
    assert obs.status["BTCUSDT"]=="PAUSED_MISSING_H4_HISTORY"
    f.bars.insert(-1,bar(220));capture(obs,data,222)
    assert obs.status["BTCUSDT"]=="OBSERVING"
    f.bars[-1]=replace(f.bars[-1],volume=5.);capture(obs,data,222)
    assert obs.status["BTCUSDT"]=="PAUSED_INTEGRITY_ERROR"
    f.bars[-1]=replace(f.bars[-1],volume=1.);capture(obs,data,222)
    assert obs.status["BTCUSDT"]=="PAUSED_INTEGRITY_ERROR" # latched in process
    obs.close()


def test_delayed_bars_labelled_and_code_identity_guard(tmp_path):
    data=cache();path=tmp_path/"o.db";obs=ChannelObserver(path,220*H4+100);capture(obs,data)
    for s in SYMBOLS:data.caches[s,"4h"].bars.append(bar(220,s))
    obs.capture(data,221*H4+120_000)
    assert obs.snapshot(data,221*H4+120_000)["rows"][0]["delayed_closed_bars"]==1
    with obs.db:obs.db.execute("UPDATE observer_meta SET value='different' WHERE key='identity'")
    obs.close()
    with pytest.raises(ValueError,match="IDENTITY_CHANGED"):ChannelObserver(path,222*H4)


def test_api_and_ui_observer_are_get_only():
    state=SimpleNamespace(clients=set(),observer_snapshot=lambda:{"status":"OBSERVING_LIVE_MARKET","rows":[]})
    client=TestClient(build_app(state))
    assert client.get('/api/observer').json()["status"]=="OBSERVING_LIVE_MARKET"
    assert client.post('/api/observer').status_code==405
    html=client.get('/observe').text
    assert 'NO LIVE EXECUTION' in html and 'observer.js' in html
    assert '/observe' in client.get('/').text
    assert client.get('/assets/observer.js').status_code==200
    assert all(not getattr(r,'methods',set())-{'GET','HEAD'} for r in client.app.routes)


def test_preview_freshness_drives_live_status(tmp_path):
    data=cache();now=220*H4+100;obs=ChannelObserver(tmp_path/"o.db",now);capture(obs,data)
    for s in SYMBOLS:
        f=Feed([]);f.preview=bar(220,s,timeframe='5m',close_time=220*H4+300_000-1,closed=False)
        data.caches[s,'5m']=f
    assert obs.snapshot(data,now)['status']=='OBSERVING_LIVE_MARKET'
    data.caches['BTCUSDT','5m'].age=10_000
    assert obs.snapshot(data,now)['status']=='DEGRADED'
    obs.close()
