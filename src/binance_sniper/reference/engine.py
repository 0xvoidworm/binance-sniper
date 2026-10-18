"""Single deterministic range breakout model; no order or network client."""
from __future__ import annotations

from dataclasses import asdict, dataclass
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path

from binance_sniper.domain import Candle

M5 = 300_000
HOUR = 3_600_000
DAY = 24 * HOUR
FINAL = {'CLOSED', 'NO_BREAKOUT', 'AMBIGUOUS', 'DATA_GAP', 'INVALID_RANGE'}


@dataclass(frozen=True)
class Rules:
    range_start_hour: int = 3
    range_end_hour: int = 6
    exit_hour: int = 18
    fee_bps: float = 5.0
    slippage_bps: float = 2.0

    def __post_init__(self):
        if not all(type(x) is int for x in (self.range_start_hour, self.range_end_hour, self.exit_hour)):
            raise ValueError('INVALID_SESSION_HOURS')
        if not 0 <= self.range_start_hour < self.range_end_hour < self.exit_hour <= 24:
            raise ValueError('INVALID_SESSION_HOURS')
        if not all(math.isfinite(x) and 0 <= x < 1000 for x in (self.fee_bps, self.slippage_bps)):
            raise ValueError('INVALID_COSTS')


def specification(rules: Rules = Rules()) -> dict:
    root = Path(__file__).resolve().parents[3]
    paths = [Path(__file__), root / 'docs' / 'REFERENCE_ENGINE.md']
    sources = {p.name: hashlib.sha256(p.read_text(encoding='utf-8').encode()).hexdigest() for p in paths}
    data = {'engine': 'RANGE_BREAKOUT_REFERENCE_V1', 'rules': asdict(rules), 'sources': sources,
            'timezone': 'UTC', 'take_profit': None, 'trailing': False, 'max_trades_per_symbol_day': 1,
            'reference_url': 'https://www.youtube.com/watch?v=zYTdc0Q1BCs&t=216s',
            'evidence': 'PUBLIC_RULES / BINANCE_ADAPTATION / NOT_PROFIT_VALIDATED'}
    data['id'] = hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()[:24]
    return data


def valid_bar(bar: Candle, symbol: str) -> bool:
    values = (bar.open, bar.high, bar.low, bar.close, bar.volume)
    return (bar.symbol == symbol and bar.timeframe == '5m' and bar.closed
            and bar.open_time % M5 == 0 and bar.close_time == bar.open_time + M5 - 1
            and all(math.isfinite(v) for v in values) and bar.volume >= 0
            and 0 < bar.low <= min(bar.open, bar.close) <= max(bar.open, bar.close) <= bar.high)


def _mark_result(result, raw_exit, at, reason, bar_open, funding, rules):
    side = 1 if result['direction'] == 'LONG' else -1
    entry = result['entry']
    exit_price = raw_exit * (1 - side * rules.slippage_bps / 10000)
    risk = result['initial_risk']
    gross = side * (exit_price - entry)
    fee = (entry + exit_price) * rules.fee_bps / 10000
    funding_cost = 0.0
    uncertain = False
    for event in funding or []:
        when = int(event['at'])
        if result['entry_bar_open'] <= when <= at:
            value = side * float(event['rate']) * float(event['mark'])
            if not math.isfinite(value):
                raise ValueError('INVALID_FUNDING')
            timing_uncertain = (when < result['entry_bar_open'] + M5 and result['entry_precision'] == 'BAR'
                                or reason == 'STOP' and when >= bar_open or when == at)
            funding_cost += max(0.0, value) if timing_uncertain else value
            uncertain = uncertain or timing_uncertain
    result.update(status='CLOSED', reason=reason, exit=exit_price, exit_at=at,
                  exit_precision='BAR' if reason == 'STOP' else 'BOUNDARY',
                  price_return_pct=side * (exit_price / entry - 1) * 100,
                  after_cost_pct=(gross - fee) / entry * 100,
                  after_cost_r=(gross - fee) / risk, fee_r=fee / risk,
                  net_r=(gross - fee - funding_cost) / risk if funding is not None else None,
                  net_pct=(gross - fee - funding_cost) / entry * 100 if funding is not None else None,
                  funding_r=funding_cost / risk if funding is not None else None,
                  funding_timing_ambiguous=uncertain,
                  held_bars=(bar_open - result['entry_bar_open']) // M5 + 1)


def evaluate_session(symbol: str, bars: list[Candle], day: int, now: int,
                     rules: Rules = Rules(), funding: list[dict] | None = None) -> dict:
    """Evaluate only available closed bars. Output is reconstruction, never a broker trade.

    Both-side entry ambiguity is excluded explicitly, not assigned a favorable path.
    Post-entry stop/exit precedence is conservative. No TP and no trailing.
    """
    if day % DAY:
        raise ValueError('DAY_MUST_BE_UTC_MIDNIGHT')
    start, armed, end = (day + h * HOUR for h in
                        (rules.range_start_hour, rules.range_end_hour, rules.exit_hour))
    result = {'id': f'{symbol}:{day}', 'symbol': symbol,
              'date': datetime.fromtimestamp(day / 1000, timezone.utc).date().isoformat(),
              'day': day, 'as_of': now, 'range_start': start, 'range_end': armed, 'session_end': end,
              'status': 'WAIT_SESSION', 'reason': 'WAIT_RANGE_START', 'range_count': 0,
              'range_expected': (armed-start)//M5, 'upper': None, 'lower': None,
              'direction': None, 'entry': None, 'stop': None, 'target': None, 'trailing': False,
              'entry_at': None, 'exit_at': None, 'net_r': None, 'net_pct': None,
              'after_cost_r': None, 'after_cost_pct': None,
              'funding_status': 'SUPPLIED' if funding is not None else 'UNKNOWN',
              'held_bars': 0, 'mode': 'MODEL', 'order_sent': False, 'last_bar_at': None}
    if now < start:
        return result
    relevant = [b for b in bars if start <= b.open_time < end and b.close_time < now]
    # Previews cannot contaminate a closed row at the same timestamp.
    relevant = [b for b in relevant if b.closed]
    by_time = {}
    for bar in relevant:
        if not valid_bar(bar, symbol) or (bar.open_time in by_time and by_time[bar.open_time] != bar):
            return {**result, 'status': 'DATA_GAP', 'reason': 'INVALID_OR_REVISED_DATA'}
        by_time[bar.open_time] = bar
    cutoff = min(armed, now // M5 * M5)
    formation = []
    for at in range(start, cutoff, M5):
        bar = by_time.get(at)
        if bar is None:
            return {**result, 'status': 'DATA_GAP', 'reason': 'RANGE_DATA_INCOMPLETE'}
        formation.append(bar)
    if formation:
        result.update(upper=max(b.high for b in formation), lower=min(b.low for b in formation),
                      range_count=len(formation), last_bar_at=formation[-1].close_time)
    if now < armed:
        return {**result, 'status': 'BUILDING_RANGE', 'reason': 'WAIT_RANGE_COMPLETE'}
    if len(formation) != result['range_expected']:
        return {**result, 'status': 'DATA_GAP', 'reason': 'RANGE_DATA_INCOMPLETE'}
    upper, lower = result['upper'], result['lower']
    if not upper or not lower or upper <= lower:
        return {**result, 'status': 'INVALID_RANGE', 'reason': 'ZERO_WIDTH_RANGE'}
    result.update(status='ARMED', reason='WAIT_FIRST_BREAKOUT')
    for at in range(armed, min(end, now // M5 * M5), M5):
        bar = by_time.get(at)
        if bar is None:
            return {**result, 'status': 'DATA_GAP', 'reason': 'EXECUTION_DATA_INCOMPLETE',
                    'unresolved_position': result['entry'] is not None}
        result['last_bar_at'] = bar.close_time
        new_entry = False
        if result['entry'] is None:
            long_hit, short_hit = bar.high >= upper, bar.low <= lower
            if bar.open >= upper:
                side, raw_entry = 1, bar.open
            elif bar.open <= lower:
                side, raw_entry = -1, bar.open
            elif long_hit and short_hit:
                return {**result, 'status': 'AMBIGUOUS', 'reason': 'BOTH_BREAKOUTS_SAME_BAR',
                        'ambiguous_at': bar.close_time}
            elif long_hit:
                side, raw_entry = 1, upper
            elif short_hit:
                side, raw_entry = -1, lower
            else:
                continue
            entry = raw_entry * (1 + side * rules.slippage_bps / 10000)
            stop = lower if side == 1 else upper
            result.update(direction='LONG' if side == 1 else 'SHORT', entry=entry, stop=stop,
                          initial_risk=side * (entry-stop), entry_at=bar.close_time,
                          entry_bar_open=bar.open_time,
                          entry_precision='BOUNDARY' if raw_entry == bar.open else 'BAR',
                          status='HOLDING', reason='HOLD_UNTIL_STOP_OR_SESSION_END')
            new_entry = True
        side = 1 if result['direction'] == 'LONG' else -1
        stop = result['stop']
        if not new_entry and side * (bar.open-stop) <= 0:
            _mark_result(result, bar.open, bar.open_time, 'STOP_GAP', bar.open_time, funding, rules)
            return result
        if (bar.low <= stop if side == 1 else bar.high >= stop):
            _mark_result(result, stop, bar.close_time, 'STOP', bar.open_time, funding, rules)
            return result
        result.update(held_bars=(bar.open_time-result['entry_bar_open'])//M5+1,
                      last_price=bar.close, unrealized_price_pct=side*(bar.close/result['entry']-1)*100)
        if bar.close_time + 1 == end:
            _mark_result(result, bar.close, bar.close_time, 'SESSION_END', bar.open_time, funding, rules)
            return result
    if now >= end and result['entry'] is None:
        result.update(status='NO_BREAKOUT', reason='SESSION_FINISHED_NO_ENTRY')
    return result
