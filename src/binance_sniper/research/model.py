"""Deterministic event-study fills. Not an exchange execution simulator."""
from __future__ import annotations

import math
from dataclasses import asdict, dataclass

from binance_sniper.domain import Candle

M5 = 300_000


@dataclass(frozen=True)
class Costs:
    fee_bps: float = 5.0
    slippage_bps: float = 2.0
    horizon: int = 4

    def __post_init__(self):
        if not all(math.isfinite(x) and x >= 0 for x in (self.fee_bps, self.slippage_bps)) or self.horizon < 1:
            raise ValueError("INVALID_COST_MODEL")


def outcome(signal: dict, bars: list[Candle], costs: Costs = Costs(), funding: list[dict] | None = None) -> dict:
    """Next-open entry; market-style adverse slippage both sides; conservative O/H/L ambiguity.

    Missing funding is NEVER silently called all-cost net. `net_r` is then null,
    while `after_fee_slippage_r` remains available. Exit-bar funding timing is
    conservative: include possible charges but not uncertain credits.
    """
    started = int(signal['decision_time']) + 1
    side = 1 if signal['direction'] == 'BULLISH' else -1
    stop, target = float(signal['stop_reference']), float(signal['target_reference'])
    future = sorted((b for b in bars if b.closed and b.open_time >= started), key=lambda b: b.open_time)[:costs.horizon]
    result = {'status': 'PENDING', 'candidate_id': signal['candidate_id'], 'symbol': signal['symbol'],
              'direction': signal['direction'], 'decision_time': started - 1, 'costs': asdict(costs),
              'funding_status': 'KNOWN' if funding is not None else 'UNKNOWN', 'ambiguous': False}
    if not future:
        return result
    if future[0].open_time != started:
        return {**result, 'status': 'GAP', 'reason': 'MISSING_NEXT_OPEN'}
    entry = future[0].open * (1 + side * costs.slippage_bps / 10000)
    risk = side * (entry - stop)
    if risk <= 0 or side * (target - entry) <= 0:
        return {**result, 'status': 'ENTRY_INVALIDATED', 'reason': 'NEXT_OPEN_OUTSIDE_BRACKET'}
    result.update(entry_time=started, entry=entry, initial_risk=risk, stop=stop, target=target)
    used = []
    reason, raw_exit, exit_at, upper_price = None, None, None, None
    for index, bar in enumerate(future):
        if bar.symbol != signal['symbol'] or bar.timeframe != '5m' or bar.open_time != started + index*M5:
            return {**result, 'status': 'GAP', 'reason': 'NONCONTIGUOUS_HORIZON'}
        used.append(bar)
        if side * (bar.open - stop) <= 0:
            reason, raw_exit, exit_at = 'STOP_GAP', bar.open, bar.open_time
        elif side * (bar.open - target) >= 0:
            # Take-profit reference receives no favorable gap bonus.
            reason, raw_exit, exit_at = 'TARGET_GAP', target, bar.open_time
        else:
            stop_hit = bar.low <= stop if side == 1 else bar.high >= stop
            target_hit = bar.high >= target if side == 1 else bar.low <= target
            if stop_hit:
                reason, raw_exit, exit_at = 'STOP', stop, bar.close_time
                if target_hit:
                    reason, upper_price, result['ambiguous'] = 'AMBIGUOUS_STOP_FIRST', target, True
            elif target_hit:
                reason, raw_exit, exit_at = 'TARGET', target, bar.close_time
        if reason:
            break
    if reason is None:
        if len(used) < costs.horizon:
            return {**result, 'bars_observed': len(used)}
        reason, raw_exit, exit_at = 'FOUR_BAR_CLOSE', used[-1].close, used[-1].close_time
    exit_price = raw_exit * (1 - side * costs.slippage_bps / 10000)
    gross = side * (exit_price - entry) / risk
    fees = (entry + exit_price) * costs.fee_bps / 10000 / risk
    funding_cost, uncertain_funding = 0.0, False
    for event in funding or []:
        at = int(event['at'])
        if not started <= at <= exit_at:
            continue
        amount = side * float(event['rate']) * float(event['mark']) / risk
        uncertain = at in {started, exit_at} or (reason not in {'FOUR_BAR_CLOSE', 'STOP_GAP', 'TARGET_GAP'} and at >= used[-1].open_time)
        uncertain_funding = uncertain_funding or uncertain
        funding_cost += max(0.0, amount) if uncertain else amount
    after_costs = gross - fees
    optimistic = None
    if upper_price is not None:
        upper_exit = upper_price * (1 - side * costs.slippage_bps / 10000)
        optimistic = side * (upper_exit - entry) / risk - (entry + upper_exit)*costs.fee_bps/10000/risk - funding_cost
    result.update(status='COMPLETED', bars=len(used), exit_reason=reason, exit_time=exit_at,
                  exit_time_precision='BAR' if reason in {'STOP','TARGET','AMBIGUOUS_STOP_FIRST'} else 'BOUNDARY',
                  exit=exit_price, gross_after_slippage_r=gross, fee_r=fees,
                  funding_r=funding_cost if funding is not None else None,
                  funding_timing_ambiguous=uncertain_funding,
                  after_fee_slippage_r=after_costs, net_r=after_costs-funding_cost if funding is not None else None,
                  optimistic_net_r=optimistic if funding is not None else None)
    return result


def temporal_partition(signal_time: int, start: int, split: int, end: int, horizon: int = 4) -> str:
    """Purge complete horizon before split; embargo horizon at holdout start."""
    if signal_time < start or signal_time + horizon*M5 >= end:
        return 'BOUNDARY_EXCLUDED'
    if signal_time < split:
        return 'DEVELOPMENT' if signal_time + horizon*M5 < split else 'PURGED'
    return 'HOLDOUT' if signal_time >= split + horizon*M5 else 'EMBARGO'
