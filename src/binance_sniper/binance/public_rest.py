from __future__ import annotations

import time
import httpx
from binance_sniper.binance.public_transport import PublicOnlyTransport


class BinancePublicRest:
    def __init__(self, base_url: str, *, dns_fallback: bool = False):
        self.transport = PublicOnlyTransport() if dns_fallback else None
        self.client = httpx.AsyncClient(base_url=base_url, timeout=20.0, follow_redirects=False, transport=self.transport, trust_env=not dns_fallback, headers={"User-Agent": "BinanceSniper/0.1"})
        self.requests_total = 0
        self.used_weight_1m: int | None = None
        self.last_status: int | None = None
        self.last_request_at = 0
        self.weight_limit_1m: int | None = None
        self.retry_after_until = 0.0

    def _record(self, response: httpx.Response) -> None:
        self.requests_total += 1
        self.last_status = response.status_code
        self.last_request_at = int(time.time() * 1000)
        value = response.headers.get("x-mbx-used-weight-1m")
        try:
            self.used_weight_1m = int(value) if value is not None else self.used_weight_1m
        except ValueError:
            pass

    async def close(self) -> None:
        await self.client.aclose()

    async def _get(self, path, params=None):
        if time.monotonic() < self.retry_after_until:
            raise httpx.RequestError("PUBLIC_RATE_BACKOFF_ACTIVE")
        response = await self.client.get(path, params=params)
        self._record(response)
        if response.status_code in (418, 429):
            try:
                delay = float(response.headers.get("retry-after", "60" if response.status_code == 429 else "900"))
            except ValueError:
                delay = 900
            self.retry_after_until = time.monotonic() + max(1, delay)
        response.raise_for_status()
        return response

    async def server_time(self) -> int:
        response = await self._get("/fapi/v1/time")
        return int(response.json()["serverTime"])

    async def exchange_info(self) -> dict:
        response = await self._get("/fapi/v1/exchangeInfo")
        return response.json()

    async def klines(self, symbol: str, interval: str, limit: int, start_time: int | None = None, end_time: int | None = None) -> list[list]:
        params = {"symbol": symbol, "interval": interval, "limit": limit}
        if start_time is not None:
            params["startTime"] = start_time
        if end_time is not None:
            params["endTime"] = end_time
        response = await self._get("/fapi/v1/klines", params=params)
        payload = response.json()
        if not isinstance(payload, list):
            raise ValueError("BINANCE_KLINES_SCHEMA_INVALID")
        return payload

    def telemetry(self) -> dict:
        return {
            "public_network_route": self.transport.network.last_route if self.transport else "SYSTEM_DNS",
            "public_dns_resolutions": self.transport.network.resolutions if self.transport else 0,
            "rate_backoff_remaining_ms": max(0, int((self.retry_after_until - time.monotonic()) * 1000)),
            "requests_total": self.requests_total,
            "used_weight_1m": self.used_weight_1m,
            "weight_limit_1m": self.weight_limit_1m,
            "remaining_weight_1m": max(0, self.weight_limit_1m - self.used_weight_1m) if self.weight_limit_1m is not None and self.used_weight_1m is not None else None,
            "last_status": self.last_status,
            "last_request_at": self.last_request_at,
        }
