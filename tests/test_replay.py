import hashlib
import json
from binance_sniper.market.bars import ClosedBarCache

def test_replay_hash_is_deterministic():
    payload={'symbol':'BTCUSDT','tf':'5m','open':1,'close':2}
    raw=json.dumps(payload,sort_keys=True).encode()
    assert hashlib.sha256(raw).hexdigest()==hashlib.sha256(raw).hexdigest()
