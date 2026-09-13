from __future__ import annotations

import asyncio
import json
import random
import time
import websockets


class MarketStream:
    def __init__(self, base_url: str, symbols: tuple[str, ...], timeframes: tuple[str, ...], on_event, max_backoff: int = 60):
        self.base_url, self.symbols, self.timeframes, self.on_event, self.max_backoff = base_url, symbols, timeframes, on_event, max_backoff
        self.stop_event = asyncio.Event()
        self.reconnects = 0

    def stream_url(self) -> str:
        streams = "/".join(f"{symbol.lower()}@kline_{tf}" for symbol in self.symbols for tf in self.timeframes)
        return f"{self.base_url}?streams={streams}"

    async def run(self) -> None:
        backoff = 1
        while not self.stop_event.is_set():
            try:
                async with websockets.connect(self.stream_url(), ping_interval=20, ping_timeout=20, close_timeout=5) as socket:
                    backoff = 1
                    await self.on_event({"type": "connected", "reconnects": self.reconnects})
                    async for raw in socket:
                        if self.stop_event.is_set():
                            break
                        try:
                            await self.on_event({"type": "kline", "received_ms": int(time.time() * 1000), "payload": json.loads(raw)})
                        except (ValueError, TypeError):
                            await self.on_event({"type": "malformed"})
            except (OSError, asyncio.TimeoutError, websockets.WebSocketException) as exc:
                self.reconnects += 1
                await self.on_event({"type": "reconnecting", "reconnects": self.reconnects, "error": type(exc).__name__})
                await asyncio.sleep(min(self.max_backoff, backoff) + random.random())
                backoff = min(self.max_backoff, backoff * 2)

    async def stop(self) -> None:
        self.stop_event.set()
