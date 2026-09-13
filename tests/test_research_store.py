import json

from binance_sniper.research.report import metrics
from binance_sniper.research.store import ResearchStore
from binance_sniper.domain import Candle
from test_research_model import signal


def test_restart_idempotence_and_immutable_completed(tmp_path):
    path=tmp_path/'research.sqlite3'
    store=ResearchStore(path)
    store.run('r','d','LIVE','hash',{})
    store.decisions([('r','BTCUSDT',1,'WAIT_H4','NEUTRAL','4h:WAIT','{}')]*2)
    store.signal('r',signal())
    store.outcome('r','a',{'status':'PENDING'})
    store.close()
    store=ResearchStore(path)
    store.signal('r',signal())
    store.outcome('r','a',{'status':'COMPLETED','net_r':1})
    store.outcome('r','a',{'status':'COMPLETED','net_r':100})
    assert store.db.execute('SELECT count(*) FROM signals').fetchone()[0]==1
    assert store.db.execute('SELECT count(*) FROM decisions').fetchone()[0]==1
    assert json.loads(store.db.execute('SELECT payload FROM outcomes').fetchone()[0])['net_r']==1
    store.run('other','d','HISTORICAL','hash',{})
    assert store.db.execute('SELECT count(*) FROM signals WHERE run=?',('other',)).fetchone()[0]==0
    store.close()


def test_empty_metrics_never_imply_profit():
    assert metrics([])['mean_r'] is None
    assert metrics([])['win_rate'] is None
    assert metrics([])['sample_warning']=='NO_SAMPLES'


def test_eight_bar_event_evidence_has_actual_horizon(tmp_path):
    store=ResearchStore(tmp_path/'research.sqlite3')
    store.run('r','d','LAB','hash',{})
    s={**signal(),'decision_time':50*300000-1,'horizon':8}
    store.signal('r',s)
    store.put_bars('d',[Candle('BTCUSDT','5m',i*300000,(i+1)*300000-1,100,106,94,105,1,True) for i in range(65)])
    evidence=store.event_view('r','a')
    assert evidence['horizon']==8
    assert len(evidence['bars'])==58
    assert evidence['bars'][-1]['close_time']==58*300000-1
    assert '8 candle' in evidence['note']
    store.close()
