from __future__ import annotations

from dataclasses import asdict, dataclass
from statistics import mean

from binance_sniper.domain import Candle

M5 = 300_000


@dataclass(frozen=True)
class Variant:
    name: str
    family: str
    impulse_tf: str = '15m'
    target_r: float = 1.0
    horizon: int = 4
    max_cost_r: float = .35


VARIANTS = (
    Variant('P15_T1_4', 'PULLBACK'),
    Variant('P15_T15_4', 'PULLBACK', target_r=1.5),
    Variant('P60_T1_4', 'PULLBACK', impulse_tf='1h'),
    Variant('RETEST_T1_4', 'RETEST'),
    Variant('SWEEP_T1_4', 'SWEEP', impulse_tf='5m'),
    Variant('SWEEP_T15_4', 'SWEEP', impulse_tf='5m', target_r=1.5),
    Variant('P15_T1_8', 'PULLBACK', horizon=8),
    Variant('RETEST_T1_8', 'RETEST', horizon=8),
)


def atr(bars: list[Candle], period=14) -> float:
    if len(bars) < period+1:
        return 0.0
    return mean(max(b.high-b.low, abs(b.high-a.close), abs(b.low-a.close)) for a,b in zip(bars[-period-1:-1],bars[-period:]))


def trend(bars: list[Candle]) -> dict:
    if len(bars)<60:
        return {'direction':0,'ema20':None,'ema50':None,'slope':None}
    sample=bars[-200:]
    fast=slow=sample[0].close
    fast_values=[]
    for bar in sample:
        fast+=(bar.close-fast)*2/21
        slow+=(bar.close-slow)*2/51
        fast_values.append(fast)
    slope=fast-fast_values[-4]
    direction=1 if fast>slow and slope>0 else (-1 if fast<slow and slope<0 else 0)
    return {'direction':direction,'ema20':fast,'ema50':slow,'slope':slope}


class Context:
    """Memoize only identical closed-bar sequences, including revisions."""
    def __init__(self):
        self.keys={}
        self.values={}

    def read(self, symbol, h4, h1):
        for tf,bars in [('4h',h4),('1h',h1)]:
            key=(symbol,tf)
            sequence=tuple(bars)
            if self.keys.get(key)!=sequence:
                self.keys[key]=sequence
                self.values[key]=trend(bars)
        high=self.values[(symbol,'4h')]
        low=self.values[(symbol,'1h')]
        direction=low['direction'] if high['direction'] in (0,low['direction']) else 0
        return {'direction':direction,'h4':high,'h1':low,
                'reason':'H1_NEUTRAL' if not low['direction'] else ('H4_OPPOSED' if not direction else 'CONTEXT_ALIGNED'),
                'available_at':max(h4[-1].close_time if h4 else 0,h1[-1].close_time if h1 else 0)}


@dataclass
class Area:
    area_id: str
    symbol: str
    direction: int
    lower: float
    upper: float
    invalidation: float
    structure_target: float
    created_at: int
    expires_at: int
    impulse_start: int
    impulse_end: int
    impulse_origin: float
    impulse_tip: float
    touched: bool = False
    extreme: float | None = None
    state: str = 'ARMED'


class PatternEngine:
    """A single preregistered hypothesis; same class in historical and shadow mode."""
    def __init__(self, variant: Variant):
        self.variant=variant
        self.areas: dict[str,Area]={}
        self.last_source={}
        self.last_decision={}
        self.cooldown={}
        self.latest={}

    def _expansion(self, symbol, bars, side, at, atr5):
        if len(bars)<22 or not side:
            return None
        bar=bars[-1]
        key=(symbol,side)
        if self.last_source.get(key)==bar.close_time:
            return None
        self.last_source[key]=bar.close_time
        prior=bars[:-1]
        unit=atr(prior)
        width=bar.high-bar.low
        body=side*(bar.close-bar.open)/max(width,1e-12)
        level=max(b.high for b in prior[-6:]) if side==1 else min(b.low for b in prior[-6:])
        volume=mean(b.volume for b in prior[-20:])
        if unit<=0 or width<1.1*unit or body<.55 or side*(bar.close-level)<=0 or bar.volume<1.2*volume:
            return None
        origin,tip=(bar.low,bar.high) if side==1 else (bar.high,bar.low)
        if self.variant.family=='RETEST':
            lower,upper=level-.15*unit,level+.15*unit
        else:
            a,b=tip-side*.65*width,tip-side*.35*width
            lower,upper=min(a,b),max(a,b)
        return Area(f'{self.variant.name}:{symbol}:{side}:{bar.close_time}',symbol,side,lower,upper,
                    origin-side*.15*atr5,tip,at,at+12*M5,bar.open_time,bar.close_time,origin,tip)

    def _sweep(self, symbol, m5, side, unit):
        bar=m5[-1]
        prior=m5[-13:-1]
        lower,upper=min(b.low for b in prior),max(b.high for b in prior)
        level=lower if side==1 else upper
        swept=bar.low<=lower-.1*unit if side==1 else bar.high>=upper+.1*unit
        reclaimed=side*(bar.close-level)>0
        location=(bar.close-bar.low)/max(bar.high-bar.low,1e-12) if side==1 else (bar.high-bar.close)/max(bar.high-bar.low,1e-12)
        body=side*(bar.close-bar.open)/max(bar.high-bar.low,1e-12)
        if not swept or not reclaimed or location<.6 or body<.4:
            return None
        extreme=bar.low if side==1 else bar.high
        return Area(f'{self.variant.name}:{symbol}:{side}:{bar.close_time}',symbol,side,level-.1*unit,level+.1*unit,
                    extreme-side*.15*unit,upper if side==1 else lower,m5[-2].close_time,bar.close_time,
                    prior[0].open_time,m5[-2].close_time,level,upper if side==1 else lower,True,extreme)

    def _signal(self, area, bar, unit, context):
        side=area.direction
        observed=area.extreme if area.extreme is not None else (bar.low if side==1 else bar.high)
        stop=observed-side*.15*unit
        # A volatility floor widens a structural stop; it never moves it inside the observed extreme.
        risk=max(side*(bar.close-stop),.8*unit)
        stop=bar.close-side*risk
        available=side*(area.structure_target-bar.close)
        target_distance=min(self.variant.target_r*risk,available)
        if risk<=0 or target_distance<.8*risk:
            return None,'INSUFFICIENT_ROOM'
        estimated_cost_r=bar.close*.0014/risk
        if estimated_cost_r>self.variant.max_cost_r:
            return None,'COST_DOMINATED'
        target=bar.close+side*target_distance
        return {'candidate_id':f'{area.area_id}:{bar.close_time}','symbol':area.symbol,
                'direction':'BULLISH' if side==1 else 'BEARISH','decision_time':bar.close_time,
                'entry_reference':bar.close,'stop_reference':stop,'target_reference':target,
                'zone_id':area.area_id,'reason_code':self.variant.family+'_DIRECTIONAL_RECLAIM',
                'score':max(0,min(1,1-estimated_cost_r)),'variant':self.variant.name,'horizon':self.variant.horizon,
                'area':{**asdict(area),'state':'TRIGGERED'},'context':context,'atr5':unit,'estimated_cost_r':estimated_cost_r,
                'reference_room_r':target_distance/risk},'SIGNAL'

    def step(self, symbol, feeds, context):
        m5=feeds['5m']
        if len(m5)<22:
            return None
        bar=m5[-1]
        at=bar.close_time
        if self.last_decision.get(symbol,0)>=at:
            return None
        # Feeds come from the validated, ordered ClosedBarCache. Check their
        # decision frontier, not 960 historical objects for every hypothesis.
        if any(series and (not series[-1].closed or series[-1].close_time>at) for series in feeds.values()):
            raise ValueError('NONCAUSAL_PATTERN_INPUT')
        previous=self.last_decision.get(symbol)
        self.last_decision[symbol]=at
        unit=context['atr5'] if 'atr5' in context else atr(m5[:-1])
        side=context['direction']
        gate='WAIT_IMPULSE'
        signal=None
        area=self.areas.get(symbol)
        if previous is not None and at-previous!=M5:
            if area:area.state='GAP_CANCELLED'
            self.latest[symbol]={'stage':'DATA_GAP','at':at,'area':asdict(area) if area else None,'context':context}
            return None
        if not side or unit<=0:
            if area and area.state in {'ARMED','TOUCHED'}:area.state='CONTEXT_CANCELLED'
            gate=context['reason'] if not side else 'WARMUP'
        elif at<=self.cooldown.get(symbol,0):
            gate='COOLDOWN'
        elif self.variant.family=='SWEEP':
            area=self._sweep(symbol,m5,side,unit)
            gate='WAIT_SWEEP'
            if area:
                signal,gate=self._signal(area,bar,unit,context)
                area.state='TRIGGERED' if signal else gate
                self.areas[symbol]=area
        else:
            if area and area.state in {'ARMED','TOUCHED'}:
                if side!=area.direction:
                    area.state='CONTEXT_CANCELLED'
                elif at>area.expires_at:
                    area.state='EXPIRED'
                elif (bar.low<=area.invalidation if side==1 else bar.high>=area.invalidation):
                    area.state='INVALIDATED'
            if not area or area.state not in {'ARMED','TOUCHED'}:
                area=self._expansion(symbol,feeds[self.variant.impulse_tf],side,at,unit)
                if area:self.areas[symbol]=area
            if area:
                gate=area.state
                if at>area.created_at and area.state in {'ARMED','TOUCHED'}:
                    touched=bar.low<=area.upper and bar.high>=area.lower
                    if touched:area.touched=True;area.state='TOUCHED'
                    if area.touched:
                        extreme=bar.low if side==1 else bar.high
                        area.extreme=extreme if area.extreme is None else (min(area.extreme,extreme) if side==1 else max(area.extreme,extreme))
                        directional_body=side*(bar.close-bar.open)/max(bar.high-bar.low,1e-12)
                        micro=bar.close>m5[-2].high if side==1 else bar.close<m5[-2].low
                        reclaim=side*(bar.close-(area.lower+area.upper)/2)>0
                        gate='WAIT_DIRECTIONAL_TRIGGER'
                        if directional_body>=.4 and micro and reclaim:
                            signal,gate=self._signal(area,bar,unit,context)
                            if signal:area.state='TRIGGERED'
        if signal:
            self.cooldown[symbol]=at+self.variant.horizon*M5
        self.latest[symbol]={'stage':gate,'at':at,'area':asdict(area) if area else None,'context':context,'variant':self.variant.name}
        return signal
