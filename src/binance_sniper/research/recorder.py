"""Local-only live research journal. Never sends an exchange request."""
from __future__ import annotations

import json
import time
from pathlib import Path

from binance_sniper.market.bars import INTERVAL_MS
from binance_sniper.research.model import Costs, outcome
from binance_sniper.research.report import build_report, decision_row, specification
from binance_sniper.research.store import ResearchStore, encoded


class ResearchRecorder:
    def __init__(self, path: Path, settings, now: int):
        self.store=ResearchStore(path)
        # Research must not stall feed workers waiting behind a historical writer.
        self.store.db.execute('PRAGMA busy_timeout=200')
        self.settings=settings
        self.fingerprint,self.spec=specification(settings)
        self.run='live-'+self.fingerprint[:24]
        self.dataset=self.run
        self.store.run(self.run,self.dataset,'LIVE',self.fingerprint,self.spec)
        self.started=self.store.db.execute('SELECT started_at FROM runs WHERE id=?',(self.run,)).fetchone()[0]
        self.store.dataset(self.dataset,'LIVE_OBSERVED_CLOSED_BARS',self.started,now,'CAPTURING',{'source':'runtime closed decision cache','strategy_hash':self.fingerprint})
        self.last_bar={}
        self.last_decision={r['symbol']:r['at'] for r in self.store.db.execute('SELECT symbol,max(at) at FROM decisions WHERE run=? GROUP BY symbol',(self.run,))}
        self.seen={r[0] for r in self.store.db.execute('SELECT id FROM signals WHERE run=?',(self.run,))}
        self.pending={r['id']:json.loads(r['payload']) for r in self.store.db.execute("SELECT s.id,s.payload FROM signals s LEFT JOIN outcomes o ON s.run=o.run AND s.id=o.id WHERE s.run=? AND (o.status IS NULL OR o.status='PENDING')",(self.run,))}
        self.snapshot={'status':'STARTING','reports':self.store.latest_reports()}
        self.last_report=0

    def close(self):
        self.store.close()

    def capture(self, runtime, now):
        for (symbol,tf), cache in runtime.cache.caches.items():
            candles=cache.values()
            new=[b for b in candles if b.close_time>self.last_bar.get((symbol,tf),0)]
            if new:
                self.store.put_bars(self.dataset,new)
                self.last_bar[(symbol,tf)]=new[-1].close_time
        for symbol in self.settings.symbols:
            m5=runtime.cache.candles(symbol,'5m')
            if not m5:
                continue
            at=m5[-1].close_time
            if at<self.started or at<=self.last_decision.get(symbol,0):
                continue
            coherent=all(runtime.cache.candles(symbol,tf) and runtime.cache.candles(symbol,tf)[-1].close_time==((at+1)//INTERVAL_MS[tf])*INTERVAL_MS[tf]-1 for tf in self.settings.timeframes)
            if not coherent and now-at<60_000:
                continue
            row=list(decision_row(self.run,symbol,at,runtime.scanner.latest[symbol],coherent))
            features=json.loads(row[-1]);features.update(recorded_at=now,observation_delay_ms=max(0,now-at))
            row[-1]=encoded(features)
            self.store.decisions([tuple(row)])
            self.last_decision[symbol]=at
        for key,signal in runtime.scanner.ledger.candidates.items():
            if key in self.seen or signal['decision_time']<self.started:
                continue
            self.store.signal(self.run,signal)
            self.seen.add(key)
            self.pending[key]=signal
        for key,signal in list(self.pending.items()):
            bars=runtime.cache.candles(signal['symbol'],'5m')
            # After a long outage, recover from locally archived bars when available.
            if bars and signal['decision_time']+1<bars[0].open_time:
                bars=self.store.bars(self.dataset,signal['symbol'],'5m')
            base=outcome(signal,bars,Costs(**self.spec['costs']))
            stress=outcome(signal,bars,Costs(**self.spec['stress_costs']))
            status='PENDING' if 'PENDING' in {base['status'],stress['status']} else 'COMPLETED'
            self.store.outcome(self.run,key,{'status':status,'partition':'LIVE','base':base,'stress':stress,'score':signal['score']})
            if status=='COMPLETED':
                del self.pending[key]
        if now-self.last_report>=30_000:
            metadata={'mode':'LIVE','dataset':self.dataset,'strategy_hash':self.fingerprint,'specification':self.spec,
                      'symbols':list(self.settings.symbols),'start':self.started,'end':now,'generated_at':now,'partitions':['LIVE'],
                      'funding_note':'Live funding belum direkonsiliasi: hanya after-fee/slippage, net termasuk funding ditampilkan kosong.',
                      'execution_note':'Observasi sinyal live, BUKAN eksekusi. Model next-open dapat mendahului saat payload diterima; delay dicatat terpisah.'}
            report=build_report(self.store,self.run,metadata)
            self.store.finish(self.run,report,'CAPTURING')
            self.snapshot={'status':'CAPTURING','updated_at':now,'active_run':self.run,'reports':self.store.latest_reports()}
            self.last_report=now
