import httpx

from binance_sniper.binance.public_rest import BinancePublicRest


def test_weight_header_is_recorded_without_network():
    client=BinancePublicRest('https://fapi.binance.com')
    response=httpx.Response(200,headers={'x-mbx-used-weight-1m':'321'})
    client._record(response)
    client.weight_limit_1m=2400
    telemetry=client.telemetry()
    assert telemetry['requests_total']==1
    assert telemetry['used_weight_1m']==321
    assert telemetry['remaining_weight_1m']==2079
