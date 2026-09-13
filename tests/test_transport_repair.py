import asyncio
import time
from unittest.mock import AsyncMock
import pytest
from binance_sniper.market.bars import ClosedBarCache
from binance_sniper.binance.private_readonly import PrivateReadOnlyClient, ReadOnlyApiError


def row(t, close, close_time=None):
    return [t, str(close-1), str(close+1), str(close-2), str(close), '10', close_time or t+299999, '0', 1, '0', '0', '0']


def test_rest_warmup_is_not_websocket_live_and_exact_gap_is_counted():
    cache = ClosedBarCache('BTCUSDT', '5m')
    cache.warmup([row(0, 10), row(300000, 11), row(900000, 13)], now=1300000)
    assert cache.quality.status == 'REST_WARM'
    assert cache.quality.transport == 'REST_WARM'
    assert cache.quality.gaps == 1


def test_preview_and_final_are_separate():
    cache = ClosedBarCache('BTCUSDT', '5m')
    cache.warmup([row(0, 10, 299999), row(300000, 11, 999999)], now=500000)
    assert len(cache.values()) == 1
    assert cache.preview_value().open_time == 300000


def test_ws_silent_and_fallback_statuses():
    cache = ClosedBarCache('BTCUSDT', '5m')
    cache.mark_ws_silent()
    assert cache.quality.status == 'WS_SILENT'
    cache.mark_fallback()
    assert cache.quality.status == 'LIVE_REST_FALLBACK'

@pytest.mark.asyncio
async def test_private_host_pinning():
    with pytest.raises(ReadOnlyApiError):
        PrivateReadOnlyClient('https://evil.example', 'key', 'secret')
