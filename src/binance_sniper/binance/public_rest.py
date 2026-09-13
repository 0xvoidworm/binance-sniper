from __future__ import annotations

import time
import httpx


class BinancePublicRest:
    def __init__(self, base_url: str):
        self.client = httpx.AsyncClient(base_url=base_url, timeout=20.0, follow_redirects=False, headers={"User-Agent": "BinanceSniper/0.1"})
        self.requests_total = 0
        self.used_weight_1m: int | None = None
        self.last_status: int | None = None
        self.last_request_at = 0
        self.weight_limit_1m: int | None = None

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

    async def server_time(self) -> int:
        response = await self.client.get("/fapi/v1/time")
        self._record(response)
        response.raise_for_status()
        return int(response.json()["serverTime"])

    async def exchange_info(self) -> dict:
        response = await self.client.get("/fapi/v1/exchangeInfo")
        self._record(response)
        response.raise_for_status()
        return response.json()

    async def klines(self, symbol: str, interval: str, limit: int, start_time: int | None = None, end_time: int | None = None) -> list[list]:
        params = {"symbol": symbol, "interval": interval, "limit": limit}
        if start_time is not None:
            params["startTime"] = start_time
        if end_time is not None:
            params["endTime"] = end_time
        response = await self.client.get("/fapi/v1/klines", params=params)
        self._record(response)
        response.raise_for_status()
        payload = response.json()
        if not isinstance(payload, list):
            raise ValueError("BINANCE_KLINES_SCHEMA_INVALID")
        return payload

    def telemetry(self) -> dict:
        return {
            "requests_total": self.requests_total,
            "used_weight_1m": self.used_weight_1m,
            "weight_limit_1m": self.weight_limit_1m,
            "remaining_weight_1m": max(0, self.weight_limit_1m - self.used_weight_1m) if self.weight_limit_1m is not None and self.used_weight_1m is not None else None,
            "last_status": self.last_status,
            "last_request_at": self.last_request_at,
        }
