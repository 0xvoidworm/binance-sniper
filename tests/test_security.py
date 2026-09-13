import asyncio
from pathlib import Path
from unittest.mock import AsyncMock

import pytest
from binance_sniper.binance.private_readonly import PrivateReadOnlyClient, ReadOnlyApiError
from binance_sniper.secrets import Credentials


def test_credentials_repr_redacted():
    value = repr(Credentials('dummy-key', 'dummy-secret'))
    assert 'dummy-key' not in value and 'dummy-secret' not in value

@pytest.mark.asyncio
async def test_mutation_endpoint_rejected_before_network():
    client = PrivateReadOnlyClient('https://fapi.binance.com', 'key', 'secret')
    client.client.get = AsyncMock()
    with pytest.raises(ReadOnlyApiError):
        await client._get('/fapi/v1/order')
    client.client.get.assert_not_called()
    await client.close()

@pytest.mark.asyncio
async def test_private_get_uses_signature_but_never_mutation():
    client = PrivateReadOnlyClient('https://fapi.binance.com', 'key', 'secret')
    response = type('R', (), {'status_code': 200, 'json': lambda self: {}})()
    client.client.get = AsyncMock(return_value=response)
    await client._get('/fapi/v1/time')
    request = client.client.get.call_args
    assert request.args[0] == '/fapi/v1/time'
    assert 'signature' in request.kwargs['params']
    await client.close()
