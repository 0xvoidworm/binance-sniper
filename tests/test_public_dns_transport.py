import httpcore
import httpx
import pytest

from binance_sniper.binance.public_transport import HOST, PublicDNSBackend, PublicOnlyTransport
from binance_sniper.binance.public_rest import BinancePublicRest


@pytest.mark.asyncio
@pytest.mark.parametrize('method,url,headers',[
 ('POST',f'https://{HOST}/fapi/v1/order',{}),
 ('GET',f'https://{HOST}/fapi/v2/account',{}),
 ('GET',f'https://{HOST}/fapi/v1/time?signature=secret',{}),
 ('GET',f'https://{HOST}/fapi/v1/time',{'X-MBX-APIKEY':'fake-test-key'}),
 ('GET','https://example.com/fapi/v1/time',{}),
 ('GET',f'http://{HOST}/fapi/v1/time',{}),
 ('GET',f'https://{HOST}:444/fapi/v1/time',{}),
])
async def test_public_route_rejects_nonpublic_requests(method,url,headers):
    t=PublicOnlyTransport()
    try:
        with pytest.raises(httpx.RequestError,match='READ_ONLY_ROUTE_REJECTED'):
            await t.handle_async_request(httpx.Request(method,url,headers=headers))
    finally: await t.aclose()


@pytest.mark.asyncio
async def test_dns_fallback_cache_and_original_tls_hostname(monkeypatch):
    t=PublicOnlyTransport();calls=[]
    class Backend:
        async def connect_tcp(self,host,*args):
            calls.append(host)
            if host==HOST:raise httpcore.ConnectTimeout()
            return 'stream'
    t.network.backend=Backend()
    await t.network.resolver.aclose()
    t.network.resolver=httpx.AsyncClient(transport=httpx.MockTransport(lambda r:httpx.Response(200,json={'Status':0,'Answer':[{'type':1,'TTL':60,'data':'13.35.1.10'}]})))
    assert await t.network.connect_tcp(HOST,443,10)=='stream'
    assert await t.network.connect_tcp(HOST,443,10)=='stream'
    assert calls==[HOST,'13.35.1.10','13.35.1.10'] and t.network.resolutions==1
    class Pool:
        async def handle_async_request(self,request):
            assert request.url.host==HOST.encode() and request.url.scheme==b'https'
            assert dict(request.headers)[b'Host']==HOST.encode()
            return httpcore.Response(200,content=b'{}')
        async def aclose(self):pass
    await t.pool.aclose();t.pool=Pool()
    response=await t.handle_async_request(httpx.Request('GET',f'https://{HOST}/fapi/v1/time'))
    await response.aclose();await t.aclose()


@pytest.mark.asyncio
async def test_dns_rejects_private_address_and_pin():
    b=PublicDNSBackend();await b.resolver.aclose()
    b.resolver=httpx.AsyncClient(transport=httpx.MockTransport(lambda r:httpx.Response(200,json={'Status':0,'Answer':[{'type':1,'TTL':60,'data':'127.0.0.1'}]})))
    with pytest.raises(httpcore.ConnectError,match='NON_GLOBAL'):await b.resolve()
    with pytest.raises(httpcore.ConnectError,match='PIN_REJECTED'):await b.connect_tcp('example.com',443)
    await b.close()


@pytest.mark.asyncio
async def test_rate_limit_blocks_subsequent_network_calls():
    calls=[];c=BinancePublicRest(f'https://{HOST}')
    await c.client.aclose()
    def handler(r):
        calls.append(r)
        return httpx.Response(429,headers={'Retry-After':'60'})
    c.client=httpx.AsyncClient(base_url=f'https://{HOST}',transport=httpx.MockTransport(handler))
    with pytest.raises(httpx.HTTPStatusError):await c.server_time()
    with pytest.raises(httpx.RequestError,match='BACKOFF'):await c.server_time()
    assert len(calls)==1 and c.telemetry()['rate_backoff_remaining_ms']>0
    await c.close()
