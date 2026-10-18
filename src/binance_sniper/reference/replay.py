"""Clearly synthetic visual fixture through the actual reference engine."""
from dataclasses import asdict

from binance_sniper.domain import Candle
from binance_sniper.reference.engine import DAY, HOUR, M5, evaluate_session, specification


def build_reference_replay():
    day = 1_735_689_600_000  # 2025-01-01 UTC; intentionally not live market data.
    symbol = 'DEMOUSDT'
    bars = []
    for index in range(180):
        at = day+3*HOUR+index*M5
        if index < 36:
            open_, high, low, close = 100, 101, 99, 100
        else:
            close = 101.5+(index-36)*.045
            open_ = 100 if index == 36 else close-.045
            high, low = close+.15, min(open_, close)-.08
        bars.append(Candle(symbol, '5m', at, at+M5-1, open_, high, low, close, 100, True))
    frames = []
    for count in (12, 36, 37, 44, 90, 180):
        available = bars[:count]
        now = available[-1].close_time+1
        session = {**evaluate_session(symbol, available, day, now), 'origin': 'SYNTHETIC', 'data_age_ms': 0}
        frames.append({'synthetic': True, 'snapshot_at': now, 'symbols': {symbol: {}},
                       'charts': {symbol+':5m': {'closed': [asdict(b) for b in available], 'preview': None}},
                       'feed_quality': {}, 'instrument_meta': {symbol: {'tick_size': '0.01'}},
                       'reference': {'status': 'SYNTHETIC', 'updated_at': now, 'specification': specification(),
                                     'sessions': {symbol: session}, 'recent': [session], 'history': None}})
    return {'synthetic': True, 'note': 'Demonstrasi sintetis. Bukan bukti profit atau transaksi.', 'frames': frames}
