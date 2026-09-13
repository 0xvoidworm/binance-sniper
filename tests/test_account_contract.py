import pytest
from binance_sniper.binance.private_readonly import PrivateReadOnlyClient

@pytest.mark.asyncio
async def test_account_client_allowlist_contains_required_read_endpoints():
    required={'/fapi/v1/positionSide/dual','/fapi/v1/multiAssetsMargin','/fapi/v1/openOrders','/sapi/v1/algo/futures/openOrders'}
    assert required <= PrivateReadOnlyClient.ALLOWED_GET
