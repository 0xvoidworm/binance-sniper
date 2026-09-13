from __future__ import annotations

import asyncio
import json
import random
import time
import websockets


class MarketStream:
    def __init__(self, base_url: str, symbols: tuple[str, ...], timeframes: tuple[str, ...], on_event, max_backoff: int = 60, silent_seconds: int = 15, renew_seconds: int = 82_800):
        self.base_url, self.symbols, self.timeframes, self.on_event, self.max_backoff = base_url, symbols, timeframes, on_event, max_backoff
        self.stop_event = asyncio.Event()
        self.reconnects = 0
        self.silent_seconds = silent_seconds
        self.renew_seconds = renew_seconds

    def stream_url(self) -> str:
        streams = "/".join(f"{symbol.lower()}@kline_{tf}" for symbol in self.symbols for tf in self.timeframes)
        return f"{self.base_url}?streams={streams}"

    async def run(self) -> None:
        backoff = 1
        while not self.stop_event.is_set():
            reason = "SERVER_CLOSED"
            try:
                async with websockets.connect(self.stream_url(), ping_interval=20, ping_timeout=20, close_timeout=5) as socket:
                    connected_at = time.monotonic()
                    await self.on_event({"type": "connected", "reconnects": self.reconnects})
                    while not self.stop_event.is_set():
                        renew_in = self.renew_seconds - (time.monotonic() - connected_at)
                        if renew_in <= 0:
                            reason = "PLANNED_RENEWAL"
                            await self.on_event({"type": "planned_renewal", "reconnects": self.reconnects})
                            await socket.close(code=1000, reason="planned renewal")
                            break
                        try:
                            raw = await asyncio.wait_for(socket.recv(), timeout=min(self.silent_seconds, renew_in))
                        except asyncio.TimeoutError:
                            if time.monotonic() - connected_at >= self.renew_seconds:
                                reason = "PLANNED_RENEWAL"
                                await self.on_event({"type": "planned_renewal", "reconnects": self.reconnects})
                            else:
                                reason = "WS_SILENT"
                                await self.on_event({"type": "ws_silent", "reconnects": self.reconnects})
                            await socket.close(code=1000, reason=reason.lower())
                            break
                        if self.stop_event.is_set():
                            break
                        try:
                            await self.on_event({"type": "kline", "received_ms": int(time.time() * 1000), "payload": json.loads(raw)})
                            backoff = 1
                        except (ValueError, TypeError):
                            await self.on_event({"type": "malformed"})
            except (OSError, asyncio.TimeoutError, websockets.WebSocketException) as exc:
                reason = type(exc).__name__
            if self.stop_event.is_set():
                break
            self.reconnects += 1
            await self.on_event({"type": "reconnecting", "reconnects": self.reconnects, "error": reason})
            await asyncio.sleep(min(self.max_backoff, backoff) + random.random())
            backoff = min(self.max_backoff, backoff * 2)

    async def stop(self) -> None:
        self.stop_event.set()
