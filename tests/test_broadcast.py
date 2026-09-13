import asyncio
import pytest
from binance_sniper.app import Runtime

@pytest.mark.asyncio
async def test_broadcast_throttle():
    runtime=Runtime.__new__(Runtime)
    runtime.last_broadcast=asyncio.get_event_loop().time()
    runtime.broadcast_pending=False
    runtime.clients=set()
    runtime.snapshot=lambda: {}
    await runtime.broadcast()
    assert runtime.broadcast_pending is True
