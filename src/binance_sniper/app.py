from __future__ import annotations

import asyncio
import logging
import os
import time
from pathlib import Path
from typing import Awaitable, Callable

from binance_sniper.binance.market_stream import MarketStream
from binance_sniper.binance.private_readonly import PrivateReadOnlyClient
from binance_sniper.binance.public_rest import BinancePublicRest
from binance_sniper.config import Settings
from binance_sniper.reference.live import MarketView, ReferenceMonitor
from binance_sniper.market.mtf_cache import MtfCache
from binance_sniper.market.bars import INTERVAL_MS
from binance_sniper.secrets import load_credentials
from binance_sniper.web import build_app
from binance_sniper.research.recorder import ResearchRecorder
from binance_sniper.strategy_lab.live import ShadowLab
from binance_sniper.round01.observer import ChannelObserver


TaskFactory = Callable[[], Awaitable[None]]


class Runtime:
    def __init__(self, settings: Settings):
        self.settings = settings
        self.cache = MtfCache(settings.symbols, settings.timeframes, capacity=max(400, settings.warmup_limit))
        project_root = Path(__file__).resolve().parents[2]
        self.scanner = MarketView(self.cache, settings)
        self.clients: set = set()
        self.account = {"auth": "AUTH_UNAVAILABLE", "note": "Private read-only panel unavailable; public scanner may continue."}
        self.account_success_at = 0
        self.account_error_class: str | None = None
        self.public = BinancePublicRest(settings.rest_base, dns_fallback=settings.public_dns_fallback)
        self.public_semaphore = asyncio.Semaphore(settings.rest_concurrency)
        self.stream = MarketStream(
            settings.websocket_base, settings.symbols, settings.timeframes,
            self.handle_stream, settings.reconnect_max_seconds,
            settings.ws_silent_seconds, settings.websocket_renew_seconds,
        )
        self.tasks: dict[str, asyncio.Task] = {}
        self.task_registry: dict[str, dict] = {}
        self.last_ws_payload = 0
        self.ws_state = "STARTING"
        self.last_broadcast = 0.0
        self.broadcast_pending = False
        self.broadcast_task: asyncio.Task | None = None
        self.server_offset_ms = 0
        self.instrument_meta: dict[str, dict] = {}
        self.stopping = False
        self.state_dir = project_root / "state"
        self.pid_file = self.state_dir / "binance-sniper.pid"
        self.lock_file = self.state_dir / "binance-sniper.lock"
        self.research = None
        self.lab = None
        self.reference = None
        self.observer = None

    def server_now(self) -> int:
        return int(time.time() * 1000) + self.server_offset_ms

    def _heartbeat(self, name: str) -> None:
        if name in self.task_registry:
            self.task_registry[name]["last_heartbeat"] = int(time.time() * 1000)

    async def _supervise(self, name: str, factory: TaskFactory) -> None:
        info = self.task_registry.setdefault(name, {"status": "STARTING", "restarts": 0, "last_heartbeat": 0, "last_error_class": None})
        while not self.stopping:
            info.update(status="RUNNING", last_heartbeat=int(time.time() * 1000), last_error_class=None)
            try:
                await factory()
            except asyncio.CancelledError:
                info["status"] = "STOPPED"
                raise
            except Exception as exc:
                logging.getLogger(__name__).exception("task %s failed: %s", name, type(exc).__name__)
                info.update(status="RESTARTING", restarts=info["restarts"] + 1, last_error_class=type(exc).__name__)
            else:
                if self.stopping:
                    break
                info.update(status="RESTARTING", restarts=info["restarts"] + 1, last_error_class="TASK_RETURNED")
            await asyncio.sleep(min(30, 2 ** min(info["restarts"], 4)))
        info["status"] = "STOPPED"

    def _spawn(self, name: str, factory: TaskFactory) -> None:
        self.tasks[name] = asyncio.create_task(self._supervise(name, factory), name=f"binance-sniper:{name}")

    async def _update_server_time(self) -> None:
        server_time = await self.public.server_time()
        self.server_offset_ms = server_time - int(time.time() * 1000)

    async def warmup(self) -> None:
        await self._update_server_time()
        try:
            info = await self.public.exchange_info()
            for instrument in info.get("symbols", []):
                symbol = instrument.get("symbol")
                if symbol in self.settings.symbols:
                    tick = next((f.get("tickSize") for f in instrument.get("filters", []) if f.get("filterType") == "PRICE_FILTER"), None)
                    if tick:
                        self.instrument_meta[symbol] = {"tick_size": tick}
            for limit in info.get("rateLimits", []):
                if limit.get("rateLimitType") == "REQUEST_WEIGHT" and limit.get("interval") == "MINUTE":
                    self.public.weight_limit_1m = int(limit["limit"])
                    break
        except Exception as exc:
            logging.getLogger(__name__).warning("exchange info unavailable: %s", type(exc).__name__)

        async def one(symbol: str, timeframe: str) -> None:
            async with self.public_semaphore:
                rows = await self.public.klines(symbol, timeframe, self.settings.warmup_limit)
            self.cache.warmup(symbol, timeframe, rows, self.server_now())

        await asyncio.gather(*(one(symbol, tf) for symbol in self.settings.symbols for tf in self.settings.timeframes))
        self.scanner.evaluate_all()
        self.ws_state = "REST_WARM"

    async def _fetch_feed(self, symbol: str, timeframe: str, promote: bool = True) -> bool:
        cache = self.cache.caches[(symbol, timeframe)]
        now = self.server_now()
        try:
            last = cache.values()[-1] if cache.values() else None
            lag = max(0, (now - last.close_time) // INTERVAL_MS[timeframe]) if last else 0
            limit = min(self.settings.warmup_limit, max(3, lag + 2))
            async with self.public_semaphore:
                rows = await self.public.klines(symbol, timeframe, limit)
            changed = self.cache.update_rest(symbol, timeframe, rows, now)
            valid = cache.quality.last_success_at == now and cache.quality.last_error_class is None
            if not valid:
                cache.mark_error(cache.quality.last_error_class or "REST_NO_VALID_ROWS", now)
                return False
            if promote:
                cache.mark_fallback()
            if changed:
                self.scanner.evaluate(symbol)
            return True
        except Exception as exc:
            cache.mark_error(type(exc).__name__, now)
            return False

    async def handle_stream(self, event: dict) -> None:
        event_type = event.get("type")
        if event_type == "connected":
            self.ws_state = "CONNECTING"
        elif event_type == "kline":
            received = int(event.get("received_ms", int(time.time() * 1000)))
            payload = event.get("payload", {})
            data = payload.get("data", {})
            key = (data.get("s"), data.get("k", {}).get("i"))
            feed = self.cache.caches.get(key)
            if feed is not None:
                recovering = feed.quality.transport != "LIVE_WS"
                reconciled = True
                if recovering:
                    self.ws_state = "RECOVERING"
                    reconciled = await self._fetch_feed(key[0], key[1], promote=False)
                accepted = self.cache.update(payload, promote=reconciled)
                if feed.quality.last_valid_payload_at == received:
                    self.last_ws_payload = received
                    if reconciled:
                        self.ws_state = "LIVE_WS"
                    if accepted:
                        self.scanner.evaluate(key[0])
        elif event_type == "reconnecting":
            self.scanner.reconnects = int(event.get("reconnects", self.scanner.reconnects))
            self.ws_state = "RECONNECTING"
        elif event_type == "ws_silent":
            self.ws_state = "WS_SILENT"
        elif event_type == "planned_renewal":
            self.ws_state = "RENEWING"
        elif event_type == "malformed":
            self.ws_state = "MALFORMED"
        await self.broadcast()

    async def broadcast(self) -> None:
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

    async def rest_feed_loop(self, symbol: str, timeframe: str) -> None:
        name = f"rest:{symbol}:{timeframe}"
        cadence = self.settings.rest_poll_seconds_by_timeframe[timeframe]
        while not self.stopping:
            self._heartbeat(name)
            feed = self.cache.caches[(symbol, timeframe)]
            now = self.server_now()
            ws_fresh = feed.quality.transport == "LIVE_WS" and now - feed.quality.last_valid_payload_at <= self.settings.ws_silent_seconds * 1000
            if not ws_fresh:
                await self._fetch_feed(symbol, timeframe)
            if feed.quality.last_success_at and now - feed.quality.last_success_at > cadence * self.settings.feed_stale_multiplier * 1000:
                feed.mark_stale()
            await self.broadcast()
            await asyncio.sleep(cadence)

    def _derived_transport(self, quality: dict) -> str:
        values = list(quality.values())
        if values and all(item["transport"] == "LIVE_WS" for item in values):
            return "LIVE_WS"
        if values and all(item["transport"] in {"LIVE_WS", "LIVE_REST_FALLBACK"} for item in values):
            return "LIVE_REST_FALLBACK" if any(item["transport"] == "LIVE_REST_FALLBACK" for item in values) else "LIVE_WS"
        if any(item["transport"] in {"LIVE_WS", "LIVE_REST_FALLBACK"} for item in values):
            return "DEGRADED"
        if self.ws_state in {"CONNECTING", "RECONNECTING", "RECOVERING", "RENEWING", "WS_SILENT"}:
            return self.ws_state
        return "STALE"

    def task_health(self) -> dict:
        now = int(time.time() * 1000)
        return {
            name: {**info, "heartbeat_age_ms": max(0, now - info["last_heartbeat"]) if info["last_heartbeat"] else None}
            for name, info in self.task_registry.items()
        }

    def snapshot(self) -> dict:
        snapshot = self.scanner.snapshot()
        quality = snapshot["feed_quality"]
        for key, item in quality.items():
            timeframe = key.rsplit(":", 1)[-1]
            item["stale_after_ms"] = self.settings.rest_poll_seconds_by_timeframe[timeframe] * self.settings.feed_stale_multiplier * 1000
        snapshot.update(
            transport_state=self._derived_transport(quality), ws_state=self.ws_state,
            last_ws_payload=self.last_ws_payload, task_health=self.task_health(),
            rest_telemetry=self.public.telemetry(), server_offset_ms=self.server_offset_ms,
            snapshot_at=int(time.time() * 1000),
            instrument_meta=self.instrument_meta,
            reference=self.reference_snapshot(),
        )
        return snapshot

    def account_snapshot(self) -> dict:
        result = dict(self.account)
        snapshot_at = int(result.get("snapshot_at", 0) or 0)
        result["snapshot_age_ms"] = max(0, int(time.time() * 1000) - snapshot_at) if snapshot_at else None
        if self.account_success_at and self.account_error_class:
            result["auth"] = "STALE"
            result["error_class"] = self.account_error_class
        return result

    async def load_account(self) -> None:
        try:
            credentials = load_credentials(self.settings.credentials_path)
            client = PrivateReadOnlyClient(self.settings.rest_base, credentials.api_key, credentials.secret)
            try:
                snapshot = await client.snapshot()
                self.account = snapshot
                self.account_success_at = int(snapshot["snapshot_at"])
                self.account_error_class = None
            finally:
                await client.close()
        except Exception as exc:
            logging.getLogger(__name__).warning("account panel unavailable: %s", type(exc).__name__)
            self.account_error_class = type(exc).__name__
            if not self.account_success_at:
                self.account = {"auth": "AUTH_UNAVAILABLE", "note": "Private read-only panel unavailable; public scanner continues.", "error_class": self.account_error_class}

    async def account_refresh_loop(self) -> None:
        while not self.stopping:
            self._heartbeat("account")
            await self.load_account()
            await self.broadcast()
            await asyncio.sleep(30)

    async def server_time_loop(self) -> None:
        while not self.stopping:
            self._heartbeat("server-time")
            await self._update_server_time()
            await asyncio.sleep(60)

    async def research_loop(self) -> None:
        try:
            self.research = ResearchRecorder(self.state_dir / "research.sqlite3", self.settings, self.server_now())
            while not self.stopping:
                self.research.capture(self, self.server_now())
                self._heartbeat("research")
                await asyncio.sleep(5)
        finally:
            if self.research is not None:
                self.research.close()
                self.research = None

    def research_snapshot(self) -> dict:
        if self.reference is not None:
            return {'status': 'ARCHIVED', 'reports': [], 'note': 'Phase 2 retired; original reports retained in research.sqlite3 and docs.'}
        snapshot = dict(self.research.snapshot) if self.research is not None else {"status":"STARTING", "reports":[]}
        health = self.task_health().get("research", {})
        if health.get("status") not in {None, "RUNNING"}:
            snapshot["status"] = health["status"]
        snapshot["task"] = health
        snapshot["snapshot_age_ms"] = max(0, self.server_now()-snapshot["updated_at"]) if snapshot.get("updated_at") else None
        return snapshot

    def research_event(self, run_id: str, candidate_id: str):
        return self.research.store.event_view(run_id, candidate_id) if self.research is not None else None

    async def lab_loop(self) -> None:
        try:
            self.lab = ShadowLab(self.state_dir / "research.sqlite3", self.settings, self.server_now())
            while not self.stopping:
                self.lab.capture(self, self.server_now())
                self._heartbeat("strategy-lab")
                await asyncio.sleep(5)
        finally:
            if self.lab is not None:
                self.lab.close()
                self.lab = None

    def lab_snapshot(self) -> dict:
        if self.reference is not None:
            return self.reference.archived
        result = dict(self.lab.snapshot) if self.lab is not None else {"status":"STARTING", "report":None, "shadow":{}}
        health = self.task_health().get("strategy-lab", {})
        result["task"] = health
        if health.get("status") not in {None,"RUNNING"}:
            result["status"] = health["status"]
        return result

    def lab_chart(self, variant: str, symbol: str):
        return self.lab.chart(variant, symbol) if self.lab is not None else None

    def lab_events(self, trial: str, variant: str):
        return self.lab.events(trial, variant) if self.lab is not None else None

    async def reference_loop(self) -> None:
        try:
            self.reference = ReferenceMonitor(self.state_dir, self.settings, self.server_now())
            while not self.stopping:
                self.reference.capture(self, self.server_now())
                self._heartbeat('range-reference')
                await asyncio.sleep(5)
        finally:
            if self.reference is not None:
                self.reference.close()
                self.reference = None

    async def observer_loop(self) -> None:
        try:
            self.observer = ChannelObserver(self.state_dir / "channel-observer.sqlite3", self.server_now())
            while not self.stopping:
                self.observer.capture(self.cache, self.server_now())
                self._heartbeat("channel-observer")
                await asyncio.sleep(2)
        finally:
            if self.observer is not None:
                self.observer.close()
                self.observer = None

    def observer_snapshot(self) -> dict:
        result = self.observer.snapshot(self.cache, self.server_now()) if self.observer else {
            "status": "STARTING", "rows": [], "execution": "DISABLED_NO_ORDER_ROUTER"}
        result["task"] = self.task_health().get("channel-observer", {})
        result["network"] = self.public.telemetry()
        if result["task"].get("status") not in (None, "RUNNING"):
            result["status"] = "DEGRADED"
        return result

    def reference_snapshot(self) -> dict:
        result = dict(self.reference.snapshot) if self.reference is not None else {'status': 'STARTING', 'sessions': {}}
        health = self.task_health().get('range-reference', {})
        result['task'] = health
        if health.get('status') not in {None, 'RUNNING'}:
            result['status'] = health['status']
        return result

    def reference_chart(self, symbol, day, dataset=None):
        result = self.reference.chart(symbol, day, dataset) if self.reference is not None else None
        if result is not None:
            result['instrument_meta'] = self.instrument_meta
        return result

    @staticmethod
    def _pid_alive(pid: int) -> bool:
        if pid <= 0:
            return False
        try:
            os.kill(pid, 0)
            return True
        except OSError:
            return False

    def _claim_instance(self) -> None:
        self.state_dir.mkdir(parents=True, exist_ok=True)
        if self.lock_file.exists():
            try:
                old_pid = int(self.lock_file.read_text(encoding="ascii").strip())
            except (OSError, ValueError):
                old_pid = -1
            if self._pid_alive(old_pid):
                raise RuntimeError("BINANCE_SNIPER_INSTANCE_ALREADY_RUNNING")
            self.lock_file.unlink(missing_ok=True)
        descriptor = os.open(self.lock_file, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        with os.fdopen(descriptor, "w", encoding="ascii") as handle:
            handle.write(str(os.getpid()))
        self.pid_file.write_text(str(os.getpid()), encoding="ascii")

    async def start(self) -> None:
        self._claim_instance()
        try:
            await self.warmup()
            self._spawn("market-stream", self.stream.run)
            self._spawn("account", self.account_refresh_loop)
            self._spawn("server-time", self.server_time_loop)
            self._spawn("range-reference", self.reference_loop)
            self._spawn("channel-observer", self.observer_loop)
            for symbol in self.settings.symbols:
                for timeframe in self.settings.timeframes:
                    self._spawn(f"rest:{symbol}:{timeframe}", lambda s=symbol, tf=timeframe: self.rest_feed_loop(s, tf))
        except Exception:
            self._release_instance()
            raise

    def _release_instance(self) -> None:
        for path in (self.pid_file, self.lock_file):
            try:
                if path.exists() and path.read_text(encoding="ascii").strip() == str(os.getpid()):
                    path.unlink()
            except OSError:
                pass

    async def stop(self) -> None:
        self.stopping = True
        await self.stream.stop()
        for task in self.tasks.values():
            task.cancel()
        await asyncio.gather(*self.tasks.values(), return_exceptions=True)
        if self.broadcast_task is not None:
            self.broadcast_task.cancel()
            await asyncio.gather(self.broadcast_task, return_exceptions=True)
        await self.public.close()
        self.ws_state = "STOPPED"
        self._release_instance()


settings = Settings.load(Path(__file__).resolve().parents[2] / "config" / "settings.yaml")
runtime = Runtime(settings)
app = build_app(runtime)
