"""Bounded phase-3 protocol. Selection never reads a future test-window outcome."""
from __future__ import annotations

import argparse
import hashlib
import json
import time
from bisect import bisect_left,bisect_right
from collections import Counter
from dataclasses import asdict
from pathlib import Path

from binance_sniper.config import Settings
from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.research.model import Costs,outcome
from binance_sniper.research.store import digest,encoded
from binance_sniper.strategy_lab.evaluation import DAY,describe,validate
from binance_sniper.strategy_lab.store import LabStore
from binance_sniper.strategy_lab.strategies import Context,PatternEngine,VARIANTS,atr

ROOT=Path(__file__).resolve().parents[3]


def specification():
    source=ROOT/'src/binance_sniper'
    paths=[*[source/'strategy_lab'/name for name in ('strategies.py','evaluation.py','runner.py','store.py')],source/'research/model.py',source/'research/report.py',
           source/'engine.py',*sorted((source/'analysis').glob('*.py')),source/'domain.py',source/'market/bars.py']
    return {'protocol_sha256':hashlib.sha256((ROOT/'docs/PHASE3_PROTOCOL.md').read_bytes()).hexdigest(),
            'configuration_sha256':hashlib.sha256((ROOT/'config/settings.yaml').read_text(encoding='utf-8').encode()).hexdigest(),
            'sources':{str(p.relative_to(source)).replace('\\','/'):hashlib.sha256(p.read_text(encoding='utf-8').encode()).hexdigest() for p in paths},
            'variants':[asdict(v) for v in VARIANTS], 'base_costs':asdict(Costs()),'stress_costs':asdict(Costs(7.5,5)),
            'schema':'phase3-v1','blocks':'calendar-aligned 7-day clusters, all six symbols together, 1000 draws, seed31013'}


def save_signals(store,run,signals,m5,funding,horizon):
    times=[b.open_time for b in m5]
    funding_times=[f['at'] for f in funding] if funding is not None else []
    values=[]
    signal_rows=[]
    for signal in signals:
        at=signal['decision_time']
        index=bisect_left(times,at+1)
        future=m5[index:index+horizon]
        charged=funding[bisect_left(funding_times,at+1):bisect_right(funding_times,at+horizon*300000)] if funding is not None else None
        base=outcome(signal,future,Costs(5,2,horizon),charged)
        stress=outcome(signal,future,Costs(7.5,5,horizon),charged)
        for result in (base,stress):
            if result['status']=='PENDING':result['status']='INCOMPLETE'
        payload={'status':'COMPLETED','partition':'LAB','base':base,'stress':stress,'score':signal['score']}
        values.append((run,signal['candidate_id'],'COMPLETED',encoded(payload)))
        signal_rows.append((run,signal['candidate_id'],signal['symbol'],at,encoded(signal)))
    with store.db:
        store.db.executemany('INSERT OR IGNORE INTO signals VALUES(?,?,?,?,?)',signal_rows)
        store.db.executemany('INSERT OR IGNORE INTO outcomes VALUES(?,?,?,?)',values)


def execute(store,dataset,settings,progress=print):
    row=store.db.execute('SELECT * FROM datasets WHERE id=?',(dataset,)).fetchone()
    if row is None or row['status']!='COMPLETE':raise ValueError('DATASET_NOT_COMPLETE')
    start,end=row['start_ms'],row['end_ms']
    if start!=1750377600000 or end!=1781481600000:
        raise ValueError('FROZEN_PROTOCOL_WINDOW_MISMATCH')
    manifest=json.loads(row['manifest'])
    spec=specification()
    spec['dataset_manifest']=manifest
    trial='lab-'+digest(spec)[:24]
    existing=store.db.execute('SELECT status,report FROM lab_trials WHERE id=?',(trial,)).fetchone()
    if existing and existing['status']=='COMPLETED':return json.loads(existing['report'])
    store.trial(trial,dataset,spec)
    for name in [v.name for v in VARIANTS]+['BASELINE']:
        store.run(trial+':'+name,dataset,'LAB',digest(spec),{'variant':name,'trial':trial})
    for symbol in settings.symbols:
        done=store.db.execute("SELECT status FROM lab_progress WHERE trial=? AND symbol=?",(trial,symbol)).fetchone()
        if done and done['status']=='COMPLETED':continue
        feeds={tf:store.bars(dataset,symbol,tf) for tf in settings.timeframes}
        for tf,bars in feeds.items():
            declared=manifest['feeds'][symbol+':'+tf]
            actual=hashlib.sha256(''.join(digest(asdict(b)) for b in bars).encode()).hexdigest()
            if not declared['complete'] or len(bars)!=declared['expected'] or actual!=declared['sha256']:
                raise ValueError('DATASET_INTEGRITY_ERROR')
        cache=MtfCache((symbol,),settings.timeframes)
        pointers={tf:0 for tf in settings.timeframes}
        for tf,bars in feeds.items():
            while pointers[tf]<len(bars) and bars[pointers[tf]].close_time<start:
                cache.caches[(symbol,tf)]._accept(bars[pointers[tf]])
                pointers[tf]+=1
        legacy=Scanner(cache,settings,persist=False)
        legacy.evaluate(symbol)
        engines={v.name:PatternEngine(v) for v in VARIANTS}
        context=Context()
        signals={name:[] for name in engines}
        counts=Counter()
        m5=feeds['5m']
        for index,bar in enumerate(m5[pointers['5m']:],start=1):
            for tf,bars in feeds.items():
                while pointers[tf]<len(bars) and bars[pointers[tf]].close_time<=bar.close_time:
                    cache.caches[(symbol,tf)]._accept(bars[pointers[tf]])
                    pointers[tf]+=1
            available={tf:cache.candles(symbol,tf) for tf in settings.timeframes}
            ctx=context.read(symbol,available['4h'],available['1h'])
            ctx['atr5']=atr(available['5m'][:-1])
            for name,engine in engines.items():
                signal=engine.step(symbol,available,ctx)
                if signal:signals[name].append(signal)
                counts[(name,bar.open_time//DAY*DAY,engine.latest.get(symbol,{}).get('stage','WARMUP'))]+=1
            legacy.evaluate(symbol)
            if index%25920==0:progress(encoded({'symbol':symbol,'scanned_days':index//288,'stage':'SCANNING_NO_OUTCOME_SELECTION_YET'}))
        funding=store.funding_rows(dataset,symbol) if manifest['funding'].get(symbol,{}).get('status')=='AVAILABLE' else None
        for variant in VARIANTS:
            save_signals(store,trial+':'+variant.name,signals[variant.name],m5,funding,variant.horizon)
        save_signals(store,trial+':BASELINE',list(legacy.ledger.candidates.values()),m5,funding,4)
        with store.db:
            store.db.executemany('INSERT OR REPLACE INTO lab_gates VALUES(?,?,?,?,?,?)',[(trial,name,symbol,day,gate,count) for (name,day,gate),count in counts.items()])
            store.db.execute("INSERT OR REPLACE INTO lab_progress VALUES(?,?,'COMPLETED')",(trial,symbol))
        progress(encoded({'symbol':symbol,'stage':'SEALED_SYMBOL_COMPLETE'}))
    histories={v.name:store.histories(trial+':'+v.name) for v in VARIANTS}
    validation=validate(histories,start,end)
    baseline=store.histories(trial+':BASELINE')
    report={'trial':trial,'dataset':dataset,'specification':spec,'symbols':list(settings.symbols),'start':start,'end':end,
            'generated_at':int(time.time()*1000),'validation':validation,
            'baseline':{'full':describe(baseline,start,end),'holdout':describe(baseline,start+300*DAY,end)},
            'variants':[{'variant':asdict(v),'run_id':trial+':'+v.name,'full_posthoc':describe(histories[v.name],start,end),
                         'gate_counts':[dict(r) for r in store.db.execute('SELECT gate,sum(count) count FROM lab_gates WHERE trial=? AND variant=? GROUP BY gate ORDER BY count DESC',(trial,v.name))]} for v in VARIANTS],
            'limitations':['Known Phase 2 period excluded; this remains retrospective evidence on a currently selected universe.',
                           'Full-period rows are post-hoc diagnostics, not selection criteria. Only training-prefix ranking chooses a candidate.',
                           'OHLC uncertainty, synthetic next-open fills, assumed costs, no sizing/portfolio or true execution latency.',
                           '8-bar alternatives are explicitly 40-minute tests; do not describe them as four-bar snipers.',
                           'No live order capability; a passed gate permits observation only.']}
    store.finish_trial(trial,report)
    return report


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--dataset',default='binance-1750377600000-1781481600000-v1')
    parser.add_argument('--export',type=Path,default=ROOT/'state/phase3-report.json')
    args=parser.parse_args()
    store=LabStore(ROOT/'state/research.sqlite3')
    try:
        report=execute(store,args.dataset,Settings.load(ROOT/'config/settings.yaml'),lambda s:print(s,flush=True))
        args.export.parent.mkdir(parents=True,exist_ok=True)
        args.export.write_text(json.dumps(report,indent=2,ensure_ascii=False,allow_nan=False),encoding='utf-8')
        print(encoded({'trial':report['trial'],'candidate':report['validation']['selection']['candidate'],
                       'promotion':report['validation']['promotion'],'holdout':report['validation']['holdout'],'export':str(args.export)}),flush=True)
    finally:store.close()


if __name__=='__main__':
    main()
