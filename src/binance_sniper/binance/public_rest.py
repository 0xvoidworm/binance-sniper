from __future__ import annotations

import httpx


class BinancePublicRest:
    def __init__(self, base_url: str):
        self.client = httpx.AsyncClient(base_url=base_url, timeout=20.0, follow_redirects=False, headers={"User-Agent": "BinanceSniper/0.1"})

    async def close(self) -> None:
        await self.client.aclose()

    async def server_time(self) -> int:
        response = await self.client.get("/fapi/v1/time")
        response.raise_for_status()
        return int(response.json()["serverTime"])

    async def exchange_info(self) -> dict:
        response = await self.client.get("/fapi/v1/exchangeInfo")
        response.raise_for_status()
        return response.json()

    async def klines(self, symbol: str, interval: str, limit: int) -> list[list]:
        response = await self.client.get("/fapi/v1/klines", params={"symbol": symbol, "interval": interval, "limit": limit})
        response.raise_for_status()
        return response.json()
