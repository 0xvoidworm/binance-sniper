import json

from binance_sniper.research.report import metrics
from binance_sniper.research.store import ResearchStore
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
