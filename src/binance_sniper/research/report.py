from __future__ import annotations

import hashlib
import json
import statistics
from collections import Counter
from dataclasses import asdict
from pathlib import Path

from binance_sniper.research.model import Costs
from binance_sniper.research.store import digest, encoded

STRATEGY_FIELDS = ('pivot_left_right','pullback_min_depth','pullback_max_depth','pullback_max_bars','zone_atr_width','trigger_min_body_fraction','four_bar_horizon')


def specification(settings):
    root = Path(__file__).resolve().parents[1]
    paths = [root/'engine.py', root/'domain.py', root/'presentation.py', *sorted((root/'analysis').glob('*.py')),
             *sorted((root/'research').glob('*.py'))]
    sources = {str(p.relative_to(root)).replace('\\','/'): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    spec = {'version': 'phase2-causal-next-open-v1', 'strategy': {k: getattr(settings,k) for k in STRATEGY_FIELDS},
            'costs': asdict(Costs(horizon=settings.four_bar_horizon)), 'stress_costs': asdict(Costs(7.5,5.0,settings.four_bar_horizon)),
            'sources': sources, 'optimization': 'NONE', 'split': 'first two-thirds development / last third holdout; horizon purge and embargo'}
    return digest(spec), spec


def decision_row(run, symbol, at, state, coherent=True):
    decision = state.get('decision', {})
    gates = decision.get('gates', [])
    first = next((g for g in gates if g['state'] != 'PASS'), None)
    reason = (f"{first['tf']}:{first['state']}" if first else 'ALL_GATES_PASS') if coherent else 'DATA_NOT_COHERENT'
    features = {'efficiency': state.get('efficiency'), 'gates': gates, 'setup_id': decision.get('setup_id'),
                'coherent': coherent, 'evaluated_bar_at': decision.get('evaluated_bar_at')}
    return (run, symbol, at, decision.get('stage','WAITING'), state.get('bias','NEUTRAL'), reason, encoded(features))


def metrics(items, field='net_r'):
    valid = sorted((v for v in items if v.get('status') == 'COMPLETED' and v.get(field) is not None), key=lambda x:x['decision_time'])
    values = [v[field] for v in valid]
    positive = sum(max(v,0) for v in values)
    negative = -sum(min(v,0) for v in values)
    equity, peak, drawdown = 0.0, 0.0, 0.0
    for value in values:
        equity += value
        peak = max(peak, equity)
        drawdown = max(drawdown, peak-equity)
    return {'n':len(values), 'mean_r':statistics.mean(values) if values else None,
            'median_r':statistics.median(values) if values else None,
            'win_rate':sum(v>0 for v in values)/len(values) if values else None,
            'profit_factor':positive/negative if negative else None,
            'sum_event_r':sum(values) if values else None, 'event_drawdown_r':drawdown if values else None,
            'ambiguous':sum(bool(v.get('ambiguous')) for v in valid),
            'sample_warning':'NO_SAMPLES' if not values else ('SMALL_SAMPLE' if len(values)<100 else 'DEPENDENT_EVENTS_NOT_PORTFOLIO')}


def build_report(store, run, metadata):
    results = [json.loads(r[0]) for r in store.db.execute('SELECT payload FROM outcomes WHERE run=?',(run,))]
    total = store.db.execute('SELECT count(*) FROM decisions WHERE run=?',(run,)).fetchone()[0]
    gate_counts = [dict(r) for r in store.db.execute('SELECT reason, count(*) AS count FROM decisions WHERE run=? GROUP BY reason ORDER BY count DESC',(run,))]
    stages = [dict(r) for r in store.db.execute('SELECT stage, count(*) AS count FROM decisions WHERE run=? GROUP BY stage ORDER BY count DESC',(run,))]
    groups = []
    for split in ('DEVELOPMENT','HOLDOUT','LIVE'):
        group = [v for v in results if v.get('partition') == split]
        if group or (split in metadata.get('partitions', [])):
            base = [v['base'] for v in group]
            stress = [v['stress'] for v in group]
            groups.append({'partition':split, 'signals':len(group), 'base':metrics(base), 'stress':metrics(stress),
                           'gross_after_slippage':metrics(base,'gross_after_slippage_r'),
                           'mean_fee_r':statistics.mean(v['fee_r'] for v in base if v.get('fee_r') is not None) if any(v.get('fee_r') is not None for v in base) else None,
                           'before_funding':metrics(base,'after_fee_slippage_r'),
                           'statuses':dict(Counter(v['status'] for v in base))})
    by_symbol = []
    for symbol in metadata['symbols']:
        group = [v['base'] for v in results if v.get('partition') in ('DEVELOPMENT','HOLDOUT','LIVE') and v['base']['symbol']==symbol]
        by_symbol.append({'symbol':symbol, **metrics(group)})
    return {**metadata, 'run_id':run, 'decisions':total, 'gate_counts':gate_counts, 'stages':stages,
            'signal_count':len(results), 'excluded':dict(Counter(v.get('partition') for v in results if v.get('partition') not in ('DEVELOPMENT','HOLDOUT','LIVE'))),
            'groups':groups, 'by_symbol':by_symbol, 'recent_outcomes':sorted(results,key=lambda v:v['base']['decision_time'],reverse=True)[:50],
            'interpretation':'Event study, bukan return akun. Event antarpair dapat berkorelasi/bertumpuk. Tidak ada leverage, sizing, fill nyata, atau klaim profitabilitas.',
            'limits':['OHLC M5 tidak memberikan urutan tick; stop-first untuk candle ambigu.',
                      'R memakai jarak modeled entry ke stop; bukan persen modal. Gap dapat menghasilkan rugi lebih dari 1R.',
                      'Fee dan slippage adalah asumsi tetap, bukan tier akun atau order-book historis.',
                      'Gate counts adalah observasi per candle, bukan jumlah trade gagal atau peluang profit yang terlewat.',
                      'Holdout satu periode bukan walk-forward. Universe dipilih sekarang; ada selection bias.',
                      'Scanner memakai zona yang telah dibekukan meskipun arah H4 kemudian berubah.']}
