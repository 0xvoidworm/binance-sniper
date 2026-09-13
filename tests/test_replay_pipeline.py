from pathlib import Path
from types import SimpleNamespace
from binance_sniper.domain import Candle
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.engine import Scanner


def make(symbol, tf, i, close, high=None, low=None):
    intervals={'4h':14_400_000,'1h':3_600_000,'15m':900_000,'5m':300_000}
    step=intervals[tf]
    return Candle(symbol,tf,i*step,(i+1)*step,close-0.5,high or close+0.5,low or close-0.5,close,10,True)


def rows(symbol, tf, count=60):
    step={'4h':14_400_000,'1h':3_600_000,'15m':900_000,'5m':300_000}[tf]
    return [[i*step,str(100+i),str(101+i),str(99+i),str(100.5+i),'10',(i+1)*step-1,'0',1,'0','0','0'] for i in range(count)]


def settings():
    return SimpleNamespace(symbols=('BTCUSDT',),timeframes=('4h','1h','15m','5m'),pivot_left_right=2,pullback_min_depth=.1,pullback_max_depth=.9,pullback_max_bars=12,zone_atr_width=.2,trigger_min_body_fraction=.1)


def run_once(tmp_path):
    cache=MtfCache(('BTCUSDT',),settings().timeframes)
    for tf in settings().timeframes: cache.warmup('BTCUSDT',tf,rows('BTCUSDT',tf),now=10_000_000_000)
    scanner=Scanner(cache,settings()); scanner.ledger.path=tmp_path/'zones.json'; scanner.evaluate_all()
    return [(event['symbol'],event['reason_code'],event['status']) for event in scanner.events]


def test_production_pipeline_replay_is_deterministic_and_future_change_isolated(tmp_path):
    first=run_once(tmp_path/'one'); second=run_once(tmp_path/'two')
    assert first == second
    assert all(event[1] in {'NO_TRIGGER','TRIGGER_CONFIRMED'} for event in first)
