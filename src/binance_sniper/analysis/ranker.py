from __future__ import annotations
from binance_sniper.domain import Candidate


def rank(candidates: list[Candidate]) -> list[Candidate]:
    return sorted(candidates, key=lambda c: (-c.net_room_r, -c.trend_efficiency, -c.impulse_quality, c.symbol, c.decision_time))
