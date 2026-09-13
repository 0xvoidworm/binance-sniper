from binance_sniper.config import Settings
from pathlib import Path

def test_config_loads_versioned_symbols():
    settings=Settings.load(Path('config/settings.yaml'))
    assert len(settings.symbols)==6
    assert settings.timeframes == ('4h','1h','15m','5m')
    assert settings.ws_silent_seconds > 0
