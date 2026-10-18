"""Public-only DNS fallback. Original hostname and TLS verification never change."""
from __future__ import annotations

import asyncio
import ipaddress
import time

import httpcore
import httpx

HOST = "fapi.binance.com"
PATHS = {"/fapi/v1/time", "/fapi/v1/exchangeInfo", "/fapi/v1/klines"}
PARAMS = {"symbol", "interval", "limit", "startTime", "endTime"}


class PublicDNSBackend(httpcore.AsyncNetworkBackend):
    def __init__(self):
        self.backend = httpcore.AnyIOBackend()
        self.resolver = httpx.AsyncClient(timeout=5, follow_redirects=False, trust_env=False)
        self.lock = asyncio.Lock()
        self.addresses: list[str] = []
        self.expires = 0.0
        self.fallback_until = 0.0
        self.last_route = "SYSTEM_DNS"
        self.resolutions = 0

    async def resolve(self):
        async with self.lock:
            if self.addresses and time.monotonic() < self.expires:
                return self.addresses
            response = await self.resolver.get("https://dns.google/resolve", params={"name": HOST, "type": "A"})
            response.raise_for_status()
            payload = response.json()
            if payload.get("Status") != 0:
                raise httpcore.ConnectError("PUBLIC_DNS_LOOKUP_FAILED")
            rows = [r for r in payload.get("Answer", []) if r.get("type") == 1]
            addresses = []
            for row in rows:
                ip = ipaddress.ip_address(row["data"])
                if ip.version != 4 or not ip.is_global:
                    raise httpcore.ConnectError("PUBLIC_DNS_NON_GLOBAL_ADDRESS")
                addresses.append(str(ip))
            if not addresses:
                raise httpcore.ConnectError("PUBLIC_DNS_NO_ADDRESS")
            self.addresses = addresses
            self.expires = time.monotonic() + max(1, min(300, min(int(r.get("TTL", 1)) for r in rows)))
            self.resolutions += 1
            return addresses

    async def connect_tcp(self, host, port, timeout=None, local_address=None, socket_options=None):
        if host != HOST or port != 443:
            raise httpcore.ConnectError("PUBLIC_HOST_PIN_REJECTED")
        budget = min(10.0, timeout if timeout is not None else 10.0)
        deadline = time.monotonic() + budget
        async with asyncio.timeout(budget):
            if time.monotonic() >= self.fallback_until:
                try:
                    stream = await self.backend.connect_tcp(host, port, min(2.0, budget), local_address, socket_options)
                    self.last_route = "SYSTEM_DNS"
                    return stream
                except (httpcore.ConnectError, httpcore.ConnectTimeout):
                    self.fallback_until = time.monotonic() + 300
            addresses = await self.resolve()
            last_error = None
            for address in addresses[:2]:
                try:
                    stream = await self.backend.connect_tcp(address, port, max(.01, deadline-time.monotonic()), local_address, socket_options)
                    self.last_route = "PUBLIC_DOH_TLS_VERIFIED"
                    return stream
                except (httpcore.ConnectError, httpcore.ConnectTimeout) as exc:
                    last_error = exc
            raise httpcore.ConnectError("PUBLIC_DNS_CONNECT_FAILED") from last_error

    async def sleep(self, seconds):
        await asyncio.sleep(seconds)

    async def close(self):
        await self.resolver.aclose()


class ResponseStream(httpx.AsyncByteStream):
    def __init__(self, stream):
        self.stream = stream

    async def __aiter__(self):
        async for chunk in self.stream:
            yield chunk

    async def aclose(self):
        if hasattr(self.stream, "aclose"):
            await self.stream.aclose()


class PublicOnlyTransport(httpx.AsyncBaseTransport):
    def __init__(self):
        self.network = PublicDNSBackend()
        self.pool = httpcore.AsyncConnectionPool(network_backend=self.network, max_connections=8, max_keepalive_connections=6)

    async def handle_async_request(self, request):
        if (request.method != "GET" or request.url.scheme != "https" or request.url.host != HOST
                or request.url.port not in (None, 443) or request.url.path not in PATHS
                or not set(request.url.params).issubset(PARAMS)
                or any(h.lower() in {"x-mbx-apikey", "authorization", "cookie"} for h in request.headers)):
            raise httpx.RequestError("PUBLIC_READ_ONLY_ROUTE_REJECTED", request=request)
        core_request = httpcore.Request(method=request.method,
            url=httpcore.URL(scheme=request.url.raw_scheme, host=request.url.raw_host,
                            port=request.url.port, target=request.url.raw_path),
            headers=request.headers.raw, content=request.stream, extensions=request.extensions)
        try:
            response = await self.pool.handle_async_request(core_request)
        except (httpcore.TimeoutException, TimeoutError) as exc:
            raise httpx.ConnectTimeout("PUBLIC_DATA_TIMEOUT", request=request) from exc
        except httpcore.NetworkError as exc:
            raise httpx.ConnectError("PUBLIC_DATA_CONNECT_ERROR", request=request) from exc
        return httpx.Response(response.status, headers=response.headers,
                              stream=ResponseStream(response.stream), extensions=response.extensions)

    async def aclose(self):
        try:
            await self.pool.aclose()
        finally:
            await self.network.close()
