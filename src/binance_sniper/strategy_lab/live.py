"""Experimental shadow observation; no HTTP client, credentials, or order API."""
from __future__ import annotations

import json
import hashlib
from dataclasses import asdict
from pathlib import Path

from binance_sniper.market.bars import INTERVAL_MS
from binance_sniper.research.model import Costs,outcome
from binance_sniper.research.store import digest,encoded
from binance_sniper.strategy_lab.runner import specification
from binance_sniper.strategy_lab.store import LabStore
from binance_sniper.strategy_lab.strategies import Area,Context,PatternEngine,VARIANTS,atr


class ShadowLab:
    def __init__(self,path,settings,now):
        self.store=LabStore(path)
        self.store.db.execute('PRAGMA busy_timeout=200')
        self.store.db.execute('CREATE TABLE IF NOT EXISTS lab_shadow_state(id TEXT PRIMARY KEY,started_at INTEGER,payload TEXT,updated_at INTEGER)')
        self.settings=settings
        self.spec=specification()
        self.spec['shadow_adapter_sha256']=hashlib.sha256(Path(__file__).read_text(encoding='utf-8').encode()).hexdigest()
        self.key='shadow-'+digest(self.spec)[:24]
        self.engines={v.name:PatternEngine(v) for v in VARIANTS}
        self.context=Context()
        prior=self.store.db.execute('SELECT * FROM lab_shadow_state WHERE id=?',(self.key,)).fetchone()
        self.started=prior['started_at'] if prior else now
        if prior:self.restore(json.loads(prior['payload']))
        self.store.dataset(self.key,'EXPERIMENTAL_LIVE_SHADOW',self.started,now,'CAPTURING',{'specification':self.spec})
        for v in VARIANTS:self.store.run(self.key+':'+v.name,self.key,'LAB_SHADOW',digest(self.spec),asdict(v))
        self.pending={}
        for row in self.store.db.execute("SELECT s.run,s.id,s.payload FROM signals s LEFT JOIN outcomes o ON o.run=s.run AND o.id=s.id WHERE s.run LIKE ? AND (o.status IS NULL OR o.status='PENDING')",(self.key+':%',)):
            self.pending[(row['run'],row['id'])]=json.loads(row['payload'])
        self.last_archive={r['symbol']:r['at'] for r in self.store.db.execute('SELECT symbol,max(close_time) at FROM candles WHERE dataset=? GROUP BY symbol',(self.key,))}
        self.snapshot={'status':'STARTING','report':None,'shadow':{}}
        self.last_refresh=0

    def restore(self,payload):
        for name,state in payload.items():
            if name not in self.engines:continue
            engine=self.engines[name]
            engine.areas={s:Area(**a) for s,a in state['areas'].items()}
            engine.last_source={(s,int(side)):at for s,side,at in state['last_source']}
            engine.last_decision=state['last_decision']
            engine.cooldown=state['cooldown']
            engine.latest=state['latest']

    def state(self):
        return {name:{'areas':{s:asdict(a) for s,a in e.areas.items()},
                      'last_source':[[s,side,at] for (s,side),at in e.last_source.items()],
                      'last_decision':e.last_decision,'cooldown':e.cooldown,'latest':e.latest} for name,e in self.engines.items()}

    def close(self):
        self.store.close()

    def capture(self,runtime,now):
        new_signals=[]
        for symbol in self.settings.symbols:
            feeds={tf:runtime.cache.candles(symbol,tf) for tf in self.settings.timeframes}
            m5=feeds['5m']
            if not m5:continue
            new=[b for b in m5 if b.close_time>self.last_archive.get(symbol,0)]
            if new:
                self.store.put_bars(self.key,new)
                self.last_archive[symbol]=new[-1].close_time
            at=m5[-1].close_time
            if at<self.started:continue
            if all(e.last_decision.get(symbol,0)>=at for e in self.engines.values()):continue
            coherent=all(feeds[tf] and feeds[tf][-1].close_time==((at+1)//INTERVAL_MS[tf])*INTERVAL_MS[tf]-1 for tf in self.settings.timeframes)
            if not coherent:
                for name,engine in self.engines.items():
                    engine.latest[symbol]={'stage':'DATA_NOT_COHERENT','at':at,'area':None,'variant':name}
                continue
            context=self.context.read(symbol,feeds['4h'],feeds['1h'])
            context['atr5']=atr(m5[:-1])
            for name,engine in self.engines.items():
                signal=engine.step(symbol,feeds,context)
                if signal:
                    signal.update(observed_at=now,observation_delay_ms=max(0,now-at),experimental=True)
                    run=self.key+':'+name
                    new_signals.append((run,signal))
                    self.pending[(run,signal['candidate_id'])]=signal
        completed=[]
        updates=[]
        for (run,key),signal in self.pending.items():
            bars=runtime.cache.candles(signal['symbol'],'5m')
            if bars and signal['decision_time']+1<bars[0].open_time:bars=self.store.bars(self.key,signal['symbol'],'5m')
            base=outcome(signal,bars,Costs(5,2,signal['horizon']))
            stress=outcome(signal,bars,Costs(7.5,5,signal['horizon']))
            status='PENDING' if 'PENDING' in {base['status'],stress['status']} else 'COMPLETED'
            updates.append((run,key,status,encoded({'status':status,'base':base,'stress':stress,'partition':'SHADOW'})))
            if status=='COMPLETED':completed.append((run,key))
        # Signal, modeled outcome and exact engine state are committed together.
        # A failed transaction propagates to the supervisor, which reloads durable state.
        with self.store.db:
            self.store.db.executemany('INSERT OR IGNORE INTO signals VALUES(?,?,?,?,?)',[(run,s['candidate_id'],s['symbol'],s['decision_time'],encoded(s)) for run,s in new_signals])
            self.store.db.executemany("INSERT INTO outcomes VALUES(?,?,?,?) ON CONFLICT(run,id) DO UPDATE SET status=excluded.status,payload=excluded.payload WHERE outcomes.status='PENDING'",updates)
            self.store.db.execute('INSERT INTO lab_shadow_state VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,updated_at=excluded.updated_at',(self.key,self.started,encoded(self.state()),now))
        for key in completed:del self.pending[key]
        if now-self.last_refresh>=15_000:
            row=self.store.db.execute("SELECT report FROM lab_trials WHERE status='COMPLETED' ORDER BY rowid DESC LIMIT 1").fetchone()
            recent=[{'run_id':r['run'],'signal':json.loads(r['payload']),'outcome':json.loads(r['result']) if r['result'] else None} for r in self.store.db.execute('SELECT s.run,s.payload,o.payload result FROM signals s LEFT JOIN outcomes o ON s.run=o.run AND s.id=o.id WHERE s.run LIKE ? ORDER BY s.at DESC LIMIT 30',(self.key+':%',))]
            self.snapshot={'status':'EXPERIMENTAL_MONITOR','updated_at':now,'report':json.loads(row['report']) if row else None,
                           'shadow':{'key':self.key,'started_at':self.started,'variants':[asdict(v) for v in VARIANTS],
                                     'states':{name:e.latest for name,e in self.engines.items()},'pending':len(self.pending),'recent':recent},
                           'note':'Hipotesis eksperimental, bukan strategi yang dipromosikan. Baseline tetap terpisah. Tidak ada order.'}
            self.last_refresh=now

    def chart(self,variant,symbol):
        engine=self.engines.get(variant)
        if engine is None or symbol not in self.settings.symbols:return None
        state=engine.latest.get(symbol,{})
        rows=self.store.db.execute("SELECT * FROM (SELECT * FROM candles WHERE dataset=? AND symbol=? AND tf='5m' ORDER BY open_time DESC LIMIT 80) ORDER BY open_time",(self.key,symbol))
        return {'mode':'SHADOW','symbol':symbol,'variant':variant,'state':state,'at':state.get('at'),
                'bars':[{'open_time':r['open_time'],'close_time':r['close_time'],'open':r['o'],'high':r['h'],'low':r['l'],'close':r['c']} for r in rows],
                'note':'Area dari observasi closed-bar eksperimen. Tidak ada entry/order yang dikirim.'}

    def events(self,trial,variant):
        if variant not in self.engines:return None
        found=self.store.db.execute("SELECT id FROM lab_trials WHERE id=? AND status='COMPLETED'",(trial,)).fetchone()
        if found is None:return None
        run=trial+':'+variant
        rows=self.store.db.execute('SELECT s.id,s.payload,o.payload result FROM signals s LEFT JOIN outcomes o ON o.run=s.run AND o.id=s.id WHERE s.run=? ORDER BY s.at DESC LIMIT 30',(run,))
        return {'trial':trial,'variant':variant,'run_id':run,'events':[{'id':r['id'],'signal':json.loads(r['payload']),'outcome':json.loads(r['result']) if r['result'] else None} for r in rows]}
