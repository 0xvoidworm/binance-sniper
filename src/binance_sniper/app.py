from __future__ import annotations

import asyncio
import logging
import time
import os
from pathlib import Path

from binance_sniper.binance.market_stream import MarketStream
from binance_sniper.binance.private_readonly import PrivateReadOnlyClient
from binance_sniper.binance.public_rest import BinancePublicRest
from binance_sniper.config import Settings
from binance_sniper.engine import Scanner
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.secrets import CredentialError, load_credentials
from binance_sniper.web import build_app


class Runtime:
    def __init__(self, settings: Settings):
        self.settings = settings
        self.cache = MtfCache(settings.symbols, settings.timeframes)
        self.scanner = Scanner(self.cache, settings)
        self.clients: set = set()
        self.account = {"auth": "AUTH_UNAVAILABLE", "note": "Private read-only panel unavailable; public scanner may continue."}
        self.account_success_at = 0
        self.account_error_class = None
        self.public = BinancePublicRest(settings.rest_base)
        self.stream = MarketStream(settings.websocket_base, settings.symbols, settings.timeframes, self.handle_stream, settings.reconnect_max_seconds, settings.ws_silent_seconds)
        self.tasks: list[asyncio.Task] = []
        self.last_ws_payload = 0
        self.last_broadcast = 0.0
        self.broadcast_pending = False
        self.broadcast_task: asyncio.Task | None = None
        self.transport_state = "STARTING"
        self.pid_file = Path("state") / "binance-sniper.pid"
        self.lock_file = Path("state") / "binance-sniper.lock"

    async def warmup(self) -> None:
        server_now = await self.public.server_time()
        async def one(symbol, timeframe):
            rows = await self.public.klines(symbol, timeframe, self.settings.warmup_limit)
            self.cache.warmup(symbol, timeframe, rows, server_now)
        await asyncio.gather(*(one(symbol, tf) for symbol in self.settings.symbols for tf in self.settings.timeframes))
        self.scanner.evaluate_all()
        self.transport_state = "REST_WARM"

    async def handle_stream(self, event: dict) -> None:
        if event.get("type") == "kline":
            received = int(event.get("received_ms", int(time.time() * 1000)))
            accepted = self.cache.update(event["payload"])
            data = event["payload"].get("data", {})
            feed = self.cache.caches.get((data.get("s"), data.get("k", {}).get("i")))
            if feed and feed.quality.transport == "LIVE_WS":
                self.last_ws_payload = received
                self.transport_state = "LIVE_WS"
            if accepted:
                symbol = data.get("s")
                if symbol:
                    self.scanner.evaluate(symbol)
        elif event.get("type") == "reconnecting":
            self.scanner.reconnects = event.get("reconnects", self.scanner.reconnects)
            self.transport_state = "RECONNECTING"
        elif event.get("type") == "malformed":
            self.transport_state = "WS_SILENT"
        elif event.get("type") == "ws_silent":
            self.transport_state = "WS_SILENT"
            self.cache.mark_all_ws_silent()
        await self.broadcast()

    async def broadcast(self):
        now = time.monotonic()
        if now - self.last_broadcast < 0.25:
            self.broadcast_pending = True
            task = getattr(self, "broadcast_task", None)
            if task is None or task.done():
                self.broadcast_task = asyncio.create_task(self._delayed_broadcast(0.25 - (now - self.last_broadcast)))
            return
        self.last_broadcast = now
        self.broadcast_pending = False
        message = self.snapshot()
        dead = set()
        for client in self.clients:
            try:
                await client.send_json(message)
            except Exception:
                dead.add(client)
        self.clients.difference_update(dead)

    async def _delayed_broadcast(self, delay: float) -> None:
        await asyncio.sleep(max(0.0, delay))
        if self.broadcast_pending:
            await self.broadcast()

    async def rest_fallback_loop(self):
        while True:
            await asyncio.sleep(self.settings.rest_poll_seconds)
            now = int(time.time() * 1000)
            silent = self.last_ws_payload == 0 or now - self.last_ws_payload > self.settings.ws_silent_seconds * 1000
            if silent:
                self.transport_state = "WS_SILENT"
                self.cache.mark_all_ws_silent()
                successful = 0
                for symbol in self.settings.symbols:
                    for timeframe in self.settings.timeframes:
                        try:
                            rows = await self.public.klines(symbol, timeframe, 3)
                            if self.cache.update_rest(symbol, timeframe, rows, now):
                                self.scanner.evaluate(symbol)
                            self.cache.caches[(symbol, timeframe)].mark_fallback()
                            successful += 1
                        except Exception as exc:
                            self.cache.caches[(symbol, timeframe)].mark_error(type(exc).__name__, now)
                if successful:
                    self.transport_state = "LIVE_REST_FALLBACK"
                await self.broadcast()

    def snapshot(self) -> dict:
        snapshot = self.scanner.snapshot()
        snapshot["transport_state"] = self.transport_state
        snapshot["last_ws_payload"] = self.last_ws_payload
        return snapshot

    async def load_account(self) -> None:
        try:
            credentials = load_credentials(self.settings.credentials_path)
            client = PrivateReadOnlyClient(self.settings.rest_base, credentials.api_key, credentials.secret)
            try:
                self.account = await client.snapshot()
                self.account_success_at = self.account.get("snapshot_at", int(time.time() * 1000))
                self.account["snapshot_age_ms"] = 0
                self.account_error_class = None
            finally:
                await client.close()
        except (CredentialError, Exception) as exc:
            logging.getLogger(__name__).warning("account panel unavailable: %s", type(exc).__name__)
            self.account_error_class = type(exc).__name__
            if self.account_success_at:
                self.account = {**self.account, "auth": "STALE", "snapshot_age_ms": int(time.time() * 1000) - self.account_success_at, "error_class": self.account_error_class}
            else:
                self.account = {"auth": "AUTH_UNAVAILABLE", "note": "Private read-only panel unavailable; public scanner continues.", "error_class": self.account_error_class}

    async def account_refresh_loop(self):
        while True:
            await asyncio.sleep(30)
            await self.load_account()
            await self.broadcast()

    async def start(self):
        self.pid_file.parent.mkdir(parents=True, exist_ok=True)
        try:
            descriptor = os.open(self.lock_file, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            with os.fdopen(descriptor, "w", encoding="ascii") as handle:
                handle.write(str(os.getpid()))
        except FileExistsError:
            raise RuntimeError("BINANCE_SNIPER_INSTANCE_ALREADY_RUNNING") from None
        self.pid_file.write_text(str(os.getpid()), encoding="ascii")
        await self.warmup()
        self.tasks = [asyncio.create_task(self.stream.run()), asyncio.create_task(self.load_account()), asyncio.create_task(self.rest_fallback_loop()), asyncio.create_task(self.account_refresh_loop())]

    async def stop(self):
        await self.stream.stop()
        for task in self.tasks:
            task.cancel()
        await asyncio.gather(*self.tasks, return_exceptions=True)
        if self.broadcast_task is not None:
            self.broadcast_task.cancel()
            await asyncio.gather(self.broadcast_task, return_exceptions=True)
        await self.public.close()
        for path in (self.pid_file, self.lock_file):
            try:
                if path.exists() and path.read_text(encoding="ascii").strip() == str(os.getpid()):
                    path.unlink()
            except OSError:
                pass


settings = Settings.load(Path(__file__).resolve().parents[2] / "config" / "settings.yaml")
runtime = Runtime(settings)
app = build_app(runtime)



