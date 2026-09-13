import asyncio

import pytest

import binance_sniper.binance.market_stream as stream_module
from binance_sniper.binance.market_stream import MarketStream


class SilentSocket:
    async def recv(self):
        await asyncio.Event().wait()
    async def close(self, **_):
        return None


class Connection:
    async def __aenter__(self): return SilentSocket()
    async def __aexit__(self, *_): return None


@pytest.mark.asyncio
async def test_silent_socket_counts_reconnect_and_exposes_reason(monkeypatch):
    events=[]
    stream=None
    async def on_event(event):
        events.append(event)
        if event['type']=='reconnecting': await stream.stop()
    stream=MarketStream('wss://fstream.binance.com/stream',('BTCUSDT',),('5m',),on_event,max_backoff=0,silent_seconds=.01,renew_seconds=60)
    monkeypatch.setattr(stream_module.websockets,'connect',lambda *_args,**_kwargs:Connection())
    monkeypatch.setattr(stream_module.random,'random',lambda:0)
    await asyncio.wait_for(stream.run(),1)
    assert [event['type'] for event in events]==['connected','ws_silent','reconnecting']
    assert stream.reconnects==1


@pytest.mark.asyncio
async def test_planned_renewal_is_reported(monkeypatch):
    events=[]
    stream=None
    async def on_event(event):
        events.append(event)
        if event['type']=='reconnecting': await stream.stop()
    stream=MarketStream('wss://fstream.binance.com/stream',('BTCUSDT',),('5m',),on_event,max_backoff=0,silent_seconds=60,renew_seconds=.01)
    monkeypatch.setattr(stream_module.websockets,'connect',lambda *_args,**_kwargs:Connection())
    monkeypatch.setattr(stream_module.random,'random',lambda:0)
    await asyncio.wait_for(stream.run(),1)
    assert any(event['type']=='planned_renewal' for event in events)
