from binance_sniper.config import Settings
from pathlib import Path
import yaml
import pytest

def test_config_loads_versioned_symbols():
    settings=Settings.load(Path('config/settings.yaml'))
    assert len(settings.symbols)==6
    assert settings.timeframes == ('4h','1h','15m','5m')
    assert settings.ws_silent_seconds > 0


@pytest.mark.parametrize(('path','value'),[
    (('app','host'),'0.0.0.0'),
    (('binance','websocket_base'),'wss://evil.example/stream'),
    (('binance','rest_concurrency'),0),
    (('strategy','pullback_max_depth'),0.1),
])
def test_invalid_security_and_feed_settings_fail_closed(tmp_path,path,value):
    raw=yaml.safe_load(Path('config/settings.yaml').read_text(encoding='utf-8'))
    raw[path[0]][path[1]]=value
    target=tmp_path/'settings.yaml';target.write_text(yaml.safe_dump(raw),encoding='utf-8')
    with pytest.raises(ValueError): Settings.load(target)
