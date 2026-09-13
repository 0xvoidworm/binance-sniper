"""Window-purged, training-only selection and block-cluster uncertainty."""
from __future__ import annotations

import random
from collections import Counter
from statistics import mean

from binance_sniper.research.report import metrics

DAY=86_400_000
PURGE=8*300_000


def window(items, start, end):
    # Uniform maximum-horizon purge and embargo prevents horizon variants from
    # benefiting from different boundary samples.
    return [v for v in items if start+PURGE<=v['base']['decision_time'] and v['base']['decision_time']+PURGE<end]


def lower_bound(items, start, end, scenario='base', draws=1000):
    complete=[v[scenario] for v in items if v[scenario].get('net_r') is not None and v[scenario]['status']=='COMPLETED']
    if len(complete)<10:
        return None
    blocks=[[0.0,0] for _ in range(max(1,(end-start+7*DAY-1)//(7*DAY)))]
    for item in complete:
        index=min(len(blocks)-1,(item['decision_time']-start)//(7*DAY))
        blocks[index][0]+=item['net_r'];blocks[index][1]+=1
    rng=random.Random(31013)
    samples=[]
    for _ in range(draws):
        selected=rng.choices(blocks,k=len(blocks))
        count=sum(b[1] for b in selected)
        if count:samples.append(sum(b[0] for b in selected)/count)
    samples.sort()
    return samples[int(.05*(len(samples)-1))] if samples else None


def describe(items,start,end):
    selected=window(items,start,end)
    base=[v['base'] for v in selected]
    stress=[v['stress'] for v in selected]
    counts=Counter(v['symbol'] for v in base if v.get('net_r') is not None and v['status']=='COMPLETED')
    return {'start':start,'end':end,'signals':len(selected),'base':metrics(base),'stress':metrics(stress),
            'lower95_net_r':lower_bound(selected,start,end),'lower95_stress_r':lower_bound(selected,start,end,'stress'),
            'by_symbol':{s:metrics([v for v in base if v['symbol']==s]) for s in sorted(counts)},
            'statuses':dict(Counter(v['status'] for v in base)),
            'exit_reasons':dict(Counter(v.get('exit_reason',v['status']) for v in base)),
            'mean_fee_r':mean(v['fee_r'] for v in base if 'fee_r' in v) if any('fee_r' in v for v in base) else None}


def select(histories,start,end):
    summaries={name:describe(items,start,end) for name,items in histories.items()}
    eligible=[name for name,s in summaries.items() if s['base']['n']>=50 and len(s['by_symbol'])>=3 and s['lower95_stress_r'] is not None]
    pool=eligible or [name for name,s in summaries.items() if s['base']['n']>0]
    ranked=sorted(pool,key=lambda name:(-(summaries[name]['lower95_stress_r'] if summaries[name]['lower95_stress_r'] is not None else -1e12),
                                      -(summaries[name]['base']['mean_r'] if summaries[name]['base']['mean_r'] is not None else -1e12),name))
    return {'candidate':ranked[0] if ranked else None,'ranking':ranked,'ranking_eligible':bool(eligible),
            'training_end':end,'training':summaries}


def validate(histories,start,end):
    if end-start!=360*DAY:
        raise ValueError('PROTOCOL_REQUIRES_360_DAYS')
    folds=[]
    pooled=[]
    for day in (120,180,240):
        boundary=start+day*DAY
        choice=select(histories,start,boundary)
        candidate=choice['candidate']
        test=describe(histories[candidate],boundary,boundary+60*DAY) if candidate else None
        folds.append({'selection':choice,'test':test})
        if candidate:pooled.extend(window(histories[candidate],boundary,boundary+60*DAY))
    final_start=start+300*DAY
    frozen=select(histories,start,final_start)
    candidate=frozen['candidate']
    final=describe(histories[candidate],final_start,end) if candidate else None
    reasons=[]
    if not candidate:
        reasons.append('NO_CANDIDATE')
    else:
        train=frozen['training'][candidate]
        if train['base']['n']<100:reasons.append('TRAINING_SAMPLE_LT_100')
        if (train['base']['mean_r'] or 0)<=0:reasons.append('TRAINING_NET_NOT_POSITIVE')
        if (train['stress']['mean_r'] or 0)<=0:reasons.append('TRAINING_STRESS_NOT_POSITIVE')
        if final['base']['n']<100:reasons.append('HOLDOUT_SAMPLE_LT_100')
        if (final['base']['mean_r'] or 0)<=0:reasons.append('HOLDOUT_NET_NOT_POSITIVE')
        if (final['stress']['mean_r'] or 0)<=0:reasons.append('HOLDOUT_STRESS_NOT_POSITIVE')
        if (final['lower95_net_r'] or 0)<=0:reasons.append('HOLDOUT_UNCERTAINTY_INCLUDES_ZERO')
        # Null PF (no losses) is not converted into a made-up numeric infinity.
        pf=final['base']['profit_factor']
        if pf is not None and pf<1.2:reasons.append('HOLDOUT_PROFIT_FACTOR_LT_1_2')
        if sum(s['n']>=10 for s in final['by_symbol'].values())<4:reasons.append('INSUFFICIENT_PAIR_COVERAGE')
    wf=metrics([v['base'] for v in pooled])
    if (wf['mean_r'] or 0)<=0:reasons.append('WALK_FORWARD_NET_NOT_POSITIVE')
    if sum(f['test'] is not None and (f['test']['base']['mean_r'] or 0)>0 for f in folds)<2:
        reasons.append('FEWER_THAN_TWO_POSITIVE_FOLDS')
    return {'folds':folds,'walk_forward':wf,'selection':frozen,'holdout':final,
            'promotion':'SHADOW_ELIGIBLE' if not reasons else 'NO_PROMOTION','reasons':reasons,
            'warning':'Retrospective validation; not portfolio returns, not a live fill guarantee. No real orders enabled.'}
