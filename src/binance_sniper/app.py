from __future__ import annotations

import asyncio
import logging
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
        self.public = BinancePublicRest(settings.rest_base)
        self.stream = MarketStream(settings.websocket_base, settings.symbols, settings.timeframes, self.handle_stream, settings.reconnect_max_seconds)
        self.tasks: list[asyncio.Task] = []

    async def warmup(self) -> None:
        await self.public.server_time()
        async def one(symbol, timeframe):
            rows = await self.public.klines(symbol, timeframe, self.settings.warmup_limit)
            self.cache.warmup(symbol, timeframe, rows)
        await asyncio.gather(*(one(symbol, tf) for symbol in self.settings.symbols for tf in self.settings.timeframes))
        self.scanner.evaluate_all()

    async def handle_stream(self, event: dict) -> None:
        if event.get("type") == "kline":
            accepted = self.cache.update(event["payload"])
            if accepted:
                data = event["payload"].get("data", {})
                symbol = data.get("s")
                if symbol:
                    self.scanner.evaluate(symbol)
        elif event.get("type") == "reconnecting":
            self.scanner.reconnects = event.get("reconnects", self.scanner.reconnects)
        await self.broadcast()

    async def broadcast(self):
        message = self.snapshot()
        dead = set()
        for client in self.clients:
            try:
                await client.send_json(message)
            except Exception:
                dead.add(client)
        self.clients.difference_update(dead)

    def snapshot(self) -> dict:
        return self.scanner.snapshot()

    async def load_account(self) -> None:
        try:
            credentials = load_credentials(self.settings.credentials_path)
            client = PrivateReadOnlyClient(self.settings.rest_base, credentials.api_key, credentials.secret)
            try:
                self.account = await client.snapshot()
            finally:
                await client.close()
        except (CredentialError, Exception) as exc:
            logging.getLogger(__name__).warning("account panel unavailable: %s", type(exc).__name__)
            self.account = {"auth": "AUTH_UNAVAILABLE", "note": "Private read-only panel unavailable; public scanner continues."}

    async def start(self):
        await self.warmup()
        self.tasks = [asyncio.create_task(self.stream.run()), asyncio.create_task(self.load_account())]

    async def stop(self):
        await self.stream.stop()
        for task in self.tasks:
            task.cancel()
        await self.public.close()


settings = Settings.load(Path(__file__).resolve().parents[2] / "config" / "settings.yaml")
runtime = Runtime(settings)
app = build_app(runtime)


@app.on_event("startup")
async def startup():
    await runtime.start()


@app.on_event("shutdown")
async def shutdown():
    await runtime.stop()
