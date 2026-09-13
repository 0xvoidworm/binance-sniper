from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.replay import SYMBOL, fixture_bars, replay_settings


def test_warmup_cannot_backdate_trigger_into_cached_history():
    bars,index=fixture_bars()
    settings=replay_settings()
    cache=MtfCache(settings.symbols,settings.timeframes)
    cutoff=bars['5m'][index+4].close_time
    for tf, candles in bars.items():
        for bar in candles:
            if bar.close_time<=cutoff:
                cache.caches[(SYMBOL,tf)]._accept(bar)
    scanner=Scanner(cache,settings,persist=False)
    scanner.evaluate(SYMBOL)
    assert not scanner.ledger.candidates
    assert all(z.created_at>=cutoff for z in scanner.ledger.zones.values())
