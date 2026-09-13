from __future__ import annotations

import hashlib
import hmac
import time
from urllib.parse import urlencode
import httpx
from urllib.parse import urlparse


class ReadOnlyApiError(RuntimeError):
    pass


class PrivateReadOnlyClient:
    ALLOWED_GET = frozenset({"/fapi/v2/account", "/fapi/v2/positionRisk", "/fapi/v1/openOrders", "/fapi/v1/openAlgoOrders", "/fapi/v1/positionSide/dual", "/fapi/v1/multiAssetsMargin", "/fapi/v1/exchangeInfo", "/sapi/v1/algo/futures/openOrders", "/fapi/v1/apiTradingStatus", "/sapi/v1/account/apiRestrictions", "/fapi/v1/time"})
    MUTATING = frozenset({"POST", "PUT", "DELETE", "PATCH"})
    FUTURES_HOST = "fapi.binance.com"
    ACCOUNT_HOST = "api.binance.com"

    def __init__(self, base_url: str, api_key: str, secret: str, account_base_url: str = "https://api.binance.com"):
        if base_url != "https://fapi.binance.com" or account_base_url != "https://api.binance.com":
            raise ReadOnlyApiError("OFFICIAL_BINANCE_HOST_REQUIRED")
        for value, host in ((base_url, self.FUTURES_HOST), (account_base_url, self.ACCOUNT_HOST)):
            parsed = urlparse(value)
            if parsed.scheme != "https" or parsed.hostname != host or parsed.username or parsed.password or parsed.port:
                raise ReadOnlyApiError("OFFICIAL_BINANCE_HOST_REQUIRED")
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
        dual_side = await self._get("/fapi/v1/positionSide/dual")
        multi_assets = await self._get("/fapi/v1/multiAssetsMargin")
        regular_orders = await self._get("/fapi/v1/openOrders")
        algo_payload = await self._get("/sapi/v1/algo/futures/openOrders")
        algo_orders = algo_payload.get("orders", []) if isinstance(algo_payload, dict) else algo_payload
        conditional_payload = await self._get("/fapi/v1/openAlgoOrders")
        conditional_orders = conditional_payload.get("orders", []) if isinstance(conditional_payload, dict) else conditional_payload
        if not isinstance(restrictions, dict) or not isinstance(account, dict) or not isinstance(positions, list) or not isinstance(regular_orders, list) or not isinstance(conditional_orders, list) or not isinstance(algo_orders, list):
            raise ReadOnlyApiError("ACCOUNT_SCHEMA_INVALID")
        return {"auth": "CONNECTED", "ip_restrict": restrictions.get("ipRestrict"), "futures_enabled": restrictions.get("enableFutures"), "account_locked": account.get("canTrade") is False, "position_mode": "HEDGE" if dual_side.get("dualSidePosition") else "ONE_WAY", "multi_assets": multi_assets.get("multiAssetsMargin"), "positions": sum(1 for x in positions if abs(float(x.get("positionAmt", 0))) > 0), "regular_orders": len(regular_orders), "conditional_orders": len(conditional_orders), "execution_algo_orders": len(algo_orders), "algo_orders": len(algo_orders), "total_orders": len(regular_orders) + len(conditional_orders) + len(algo_orders), "orders": len(regular_orders) + len(conditional_orders) + len(algo_orders), "available_balance": float(account.get("availableBalance", 0)), "warning": "API IP restriction disabled (accepted operator policy)" if restrictions.get("ipRestrict") is False else None, "snapshot_at": int(time.time() * 1000)}
