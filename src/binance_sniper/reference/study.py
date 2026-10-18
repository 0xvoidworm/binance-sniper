"""One fixed reference replay over existing data. No training or parameter search."""
from __future__ import annotations

from collections import Counter, defaultdict
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
import sqlite3
from statistics import mean

from binance_sniper.domain import Candle
from binance_sniper.reference.engine import DAY, Rules, evaluate_session, specification


def metrics(results):
    closed = [r for r in results if r['status'] == 'CLOSED' and r.get('net_r') is not None]
    values = [r['net_r'] for r in closed]
    positive = sum(v for v in values if v > 0)
    negative = -sum(v for v in values if v < 0)
    return {'sessions': len(results), 'status_counts': dict(Counter(r['status'] for r in results)),
            'closed': len(closed), 'mean_net_r': mean(values) if values else None,
            'win_rate': sum(v > 0 for v in values)/len(values) if values else None,
            'profit_factor': positive/negative if negative else None,
            'mean_net_price_pct': mean(r['net_pct'] for r in closed) if closed else None,
            'mean_held_bars': mean(r['held_bars'] for r in closed) if closed else None,
            'exit_counts': dict(Counter(r['reason'] for r in closed)),
            'scope': 'RESOLVED_MODELED_EVENTS_ONLY / NOT_PORTFOLIO_RETURN'}


def main():
    root = Path(__file__).resolve().parents[3]
    source = root / 'state' / 'research.sqlite3'
    db = sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)
    db.row_factory = sqlite3.Row
    # The two already-seen, explicitly frozen datasets. Never select by outcome.
    ids = ['binance-1750377600000-1781481600000-v1', 'binance-1781481600000-1789257600000-v1']
    datasets = [dict(r) for r in db.execute('SELECT * FROM datasets WHERE id IN (?,?) ORDER BY start_ms', ids)]
    if not datasets:
        raise ValueError('FROZEN_DATASET_NOT_FOUND')
    spec = specification()
    all_results, stress_results, manifests = [], [], []
    for dataset in datasets:
        key, start, end = dataset['id'], dataset['start_ms'], dataset['end_ms']
        manifests.append({'id': key, 'start': start, 'end': end,
                          'manifest_sha256': hashlib.sha256(dataset['manifest'].encode()).hexdigest()})
        symbols = [r[0] for r in db.execute("SELECT DISTINCT symbol FROM candles WHERE dataset=? AND tf='5m' ORDER BY symbol", (key,))]
        for symbol in symbols:
            by_day = defaultdict(list)
            for row in db.execute("SELECT * FROM candles WHERE dataset=? AND symbol=? AND tf='5m' AND open_time>=? AND close_time<? ORDER BY open_time",
                                  (key, symbol, start, end)):
                bar = Candle(symbol, '5m', row['open_time'], row['close_time'], row['o'], row['h'], row['l'], row['c'], row['v'], True)
                by_day[bar.open_time//DAY*DAY].append(bar)
            funding = [dict(r) for r in db.execute('SELECT at,rate,mark FROM funding WHERE dataset=? AND symbol=? ORDER BY at', (key, symbol))]
            if not funding:
                raise ValueError('HISTORICAL_FUNDING_MISSING')
            for day in range(start//DAY*DAY, end, DAY):
                if day < start or day+DAY > end:
                    continue
                charges = [f for f in funding if day <= f['at'] < day+DAY]
                if not charges:
                    raise ValueError('DAILY_FUNDING_COVERAGE_MISSING')
                result = evaluate_session(symbol, by_day[day], day, day+DAY, funding=charges)
                stress = evaluate_session(symbol, by_day[day], day, day+DAY,
                                          Rules(fee_bps=7.5, slippage_bps=5), charges)
                all_results.append({**result, 'dataset': key, 'origin': 'HISTORICAL_RECONSTRUCTION'})
                stress_results.append(stress)
    db.close()
    summary = metrics(all_results)
    report = {'status': 'EXPLORATORY_ONLY', 'specification': spec, 'datasets': manifests,
              'base': summary, 'stress': metrics(stress_results),
              'by_symbol': {s: metrics([r for r in all_results if r['symbol'] == s]) for s in sorted({r['symbol'] for r in all_results})},
              'by_dataset': {d['id']: metrics([r for r in all_results if r['dataset'] == d['id']]) for d in manifests},
              'note': 'Data sudah pernah dilihat; tanpa training/optimasi. Hasil resolved-only, bukan validasi baru atau return akun.',
              'profitability': 'NOT_ESTABLISHED', 'source_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'results_sha256': hashlib.sha256(json.dumps(all_results, sort_keys=True).encode()).hexdigest(),
              'recent': sorted(all_results, key=lambda r: (r['day'], r['symbol']), reverse=True)[:36]}
    target = root / 'state' / 'reference-history.json'
    temp = target.with_suffix('.tmp')
    temp.write_text(json.dumps(report, indent=2, allow_nan=False), encoding='utf-8')
    temp.replace(target)
    export = root.parents[1] / 'outputs' / 'binance-range-reference-results.json'
    export.write_text(json.dumps({**report, 'sessions': all_results}, indent=2, allow_nan=False), encoding='utf-8')
    print(json.dumps({'report': str(target), 'export': str(export), 'base': summary,
                      'stress': report['stress'], 'datasets': [d['id'] for d in manifests]}, indent=2))


if __name__ == '__main__':
    main()
