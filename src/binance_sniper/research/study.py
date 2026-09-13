"""Replay audited public candles through the SAME scanner; no network or trading."""
from __future__ import annotations

import argparse
import hashlib
import json
import time
from dataclasses import asdict
from bisect import bisect_left
from pathlib import Path

from binance_sniper.config import Settings
from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.research.model import Costs, outcome, temporal_partition
from binance_sniper.research.report import build_report, decision_row, specification
from binance_sniper.research.store import ResearchStore, digest, encoded

ROOT = Path(__file__).resolve().parents[3]


def run_study(store, dataset, settings, progress=None):
    row = store.db.execute('SELECT * FROM datasets WHERE id=?',(dataset,)).fetchone()
    if row is None or row['status'] != 'COMPLETE':
        raise ValueError('DATASET_NOT_COMPLETE')
    manifest = json.loads(row['manifest'])
    start, end = row['start_ms'], row['end_ms']
    funding_hashes = {}
    for symbol in settings.symbols:
        funding_hashes[symbol] = digest(store.funding_rows(dataset,symbol))
        for tf in settings.timeframes:
            checksums = store.db.execute('SELECT checksum FROM candles WHERE dataset=? AND symbol=? AND tf=? ORDER BY open_time',(dataset,symbol,tf))
            actual = hashlib.sha256(''.join(r[0] for r in checksums).encode()).hexdigest()
            if actual != manifest.get('feeds',{}).get(f'{symbol}:{tf}',{}).get('sha256'):
                raise ValueError('DATASET_CHECKSUM_MISMATCH')
    # Boundary set before evaluating any outcomes; day-aligned 60/30 for 90 days.
    split = start + ((end-start)//86_400_000*2//3)*86_400_000
    fingerprint, spec = specification(settings)
    run = f"history-{digest([dataset,manifest,funding_hashes,fingerprint])[:24]}"
    existing = store.db.execute('SELECT status,report FROM runs WHERE id=?',(run,)).fetchone()
    if existing and existing['status']=='COMPLETED':
        return json.loads(existing['report'])
    store.run(run,dataset,'HISTORICAL',fingerprint,spec)
    for symbol in settings.symbols:
        bars = {tf:store.bars(dataset,symbol,tf) for tf in settings.timeframes}
        for tf, candles in bars.items():
            if not candles or manifest.get('feeds',{}).get(f'{symbol}:{tf}',{}).get('complete') is not True:
                raise ValueError(f'INCOMPLETE_FEED:{symbol}:{tf}')
            actual = hashlib.sha256(''.join(digest(asdict(c)) for c in candles).encode()).hexdigest()
            if actual != manifest['feeds'][f'{symbol}:{tf}']['sha256']:
                raise ValueError('CANDLE_CONTENT_CHECKSUM_MISMATCH')
        cache = MtfCache((symbol,),settings.timeframes)
        scanner = Scanner(cache,settings,persist=False)
        pointers = {tf:0 for tf in settings.timeframes}
        for tf, candles in bars.items():
            while pointers[tf] < len(candles) and candles[pointers[tf]].close_time < start:
                cache.caches[(symbol,tf)]._accept(candles[pointers[tf]])
                pointers[tf]+=1
        # Causal initial state at start-1; no earlier signals are synthesized.
        scanner.evaluate(symbol)
        pending_rows = []
        m5 = bars['5m']
        for bar in m5[pointers['5m']:]:
            for tf in settings.timeframes:
                candles = bars[tf]
                while pointers[tf] < len(candles) and candles[pointers[tf]].close_time <= bar.close_time:
                    cache.caches[(symbol,tf)]._accept(candles[pointers[tf]])
                    pointers[tf]+=1
            scanner.evaluate(symbol)
            pending_rows.append(decision_row(run,symbol,bar.close_time,scanner.latest[symbol]))
            if len(pending_rows)>=500:
                store.decisions(pending_rows)
                pending_rows.clear()
        store.decisions(pending_rows)
        funding = store.funding_rows(dataset,symbol) if manifest.get('funding',{}).get(symbol,{}).get('status')=='AVAILABLE' else None
        times = [b.open_time for b in m5]
        for signal in scanner.ledger.candidates.values():
            store.signal(run,signal)
            partition = temporal_partition(signal['decision_time'],start,split,end,settings.four_bar_horizon)
            index = bisect_left(times,signal['decision_time']+1)
            horizon = m5[index:index+settings.four_bar_horizon]
            base = outcome(signal,horizon,Costs(**spec['costs']),funding)
            stress = outcome(signal,horizon,Costs(**spec['stress_costs']),funding)
            if base['status']=='PENDING':
                base['status']='INCOMPLETE'
            if stress['status']=='PENDING':
                stress['status']='INCOMPLETE'
            store.outcome(run,signal['candidate_id'],{'status':'COMPLETED','partition':partition,'base':base,'stress':stress,'score':signal['score']})
        if progress:
            progress({'symbol':symbol,'decisions':len(m5)-220,'signals':len(scanner.ledger.candidates),'zones':len(scanner.ledger.zones)})
    metadata = {'mode':'HISTORICAL','dataset':dataset,'dataset_manifest':manifest,'strategy_hash':fingerprint,
                'specification':spec,'symbols':list(settings.symbols),'start':start,'split':split,'end':end,
                'funding_hashes':funding_hashes,'partitions':['DEVELOPMENT','HOLDOUT'],'generated_at':int(time.time()*1000)}
    report = build_report(store,run,metadata)
    store.finish(run,report)
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset')
    parser.add_argument('--export',type=Path,default=ROOT/'state'/'research-report.json')
    args=parser.parse_args()
    settings = Settings.load(ROOT/'config/settings.yaml')
    store=ResearchStore(ROOT/'state'/'research.sqlite3')
    try:
        row = store.db.execute("SELECT id FROM datasets WHERE status='COMPLETE' ORDER BY end_ms DESC LIMIT 1").fetchone()
        dataset=args.dataset or (row['id'] if row else None)
        report=run_study(store,dataset,settings,lambda value:print(encoded(value),flush=True))
        args.export.parent.mkdir(parents=True,exist_ok=True)
        args.export.write_text(json.dumps(report,indent=2,ensure_ascii=False,allow_nan=False),encoding='utf-8')
        print(encoded({'run':report['run_id'],'signals':report['signal_count'],'groups':report['groups'],'export':str(args.export)}),flush=True)
    finally:
        store.close()


if __name__=='__main__':
    main()
