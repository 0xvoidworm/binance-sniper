import asyncio
import pytest
from binance_sniper.app import Runtime

@pytest.mark.asyncio
async def test_broadcast_throttle():
    class Client:
        def __init__(self): self.messages=[]
        async def send_json(self, value): self.messages.append(value)
    runtime=Runtime.__new__(Runtime)
    runtime.last_broadcast=asyncio.get_event_loop().time()
    runtime.broadcast_pending=False
    runtime.broadcast_task=None
    client=Client(); runtime.clients={client}
    runtime.snapshot=lambda: {}
    await runtime.broadcast()
    assert runtime.broadcast_pending is True
    await asyncio.sleep(.3)
    assert client.messages == [{}]
    assert runtime.broadcast_pending is False
