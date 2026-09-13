from __future__ import annotations

import hashlib
import hmac
import time
from urllib.parse import urlencode
import httpx


class ReadOnlyApiError(RuntimeError):
    pass


class PrivateReadOnlyClient:
    ALLOWED_GET = frozenset({"/fapi/v2/account", "/fapi/v2/positionRisk", "/fapi/v1/openOrders", "/fapi/v1/apiTradingStatus", "/sapi/v1/account/apiRestrictions", "/fapi/v1/time"})
    MUTATING = frozenset({"POST", "PUT", "DELETE", "PATCH"})

    def __init__(self, base_url: str, api_key: str, secret: str, account_base_url: str = "https://api.binance.com"):
        self._api_key, self._secret = api_key, secret
        self.client = httpx.AsyncClient(base_url=base_url, timeout=20.0, follow_redirects=False, headers={"X-MBX-APIKEY": api_key, "User-Agent": "BinanceSniper/0.1-readonly"})
        self.account_client = httpx.AsyncClient(base_url=account_base_url, timeout=20.0, follow_redirects=False, headers={"X-MBX-APIKEY": api_key, "User-Agent": "BinanceSniper/0.1-readonly"})
        self.offset_ms = 0

    async def close(self) -> None:
        await self.client.aclose()
        await self.account_client.aclose()

    async def _get(self, path: str, params: dict | None = None) -> dict | list:
        if path not in self.ALLOWED_GET:
            raise ReadOnlyApiError("READ_ONLY_ENDPOINT_BLOCKED")
        query = dict(params or {})
        query.update(timestamp=int(time.time() * 1000) + self.offset_ms, recvWindow=5000)
        payload = urlencode(query)
        query["signature"] = hmac.new(self._secret.encode(), payload.encode(), hashlib.sha256).hexdigest()
        response = await (self.account_client if path.startswith("/sapi/") else self.client).get(path, params=query)
        if response.status_code >= 400:
            raise ReadOnlyApiError(f"BINANCE_READ_FAILED_{response.status_code}")
        return response.json()

    async def snapshot(self) -> dict:
        server = await self._get("/fapi/v1/time")
        self.offset_ms = int(server["serverTime"]) - int(time.time() * 1000)
        restrictions = await self._get("/sapi/v1/account/apiRestrictions")
        account = await self._get("/fapi/v2/account")
        positions = await self._get("/fapi/v2/positionRisk")
        orders = await self._get("/fapi/v1/openOrders", {"symbol": "BTCUSDT"})
        return {"auth": "CONNECTED", "ip_restrict": restrictions.get("ipRestrict"), "futures_enabled": restrictions.get("enableFutures"), "account_locked": account.get("canTrade") is False, "position_mode": "HEDGE" if account.get("dualSidePosition") else "ONE_WAY", "multi_assets": account.get("multiAssetsMargin"), "positions": sum(1 for x in positions if abs(float(x.get("positionAmt", 0))) > 0), "orders": len(orders), "available_balance": float(account.get("availableBalance", 0)), "warning": "API IP restriction disabled (accepted operator policy)" if restrictions.get("ipRestrict") is False else None}
