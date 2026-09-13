from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import os
import yaml


@dataclass(frozen=True)
class Settings:
    host: str
    port: int
    rest_base: str
    websocket_base: str
    symbols: tuple[str, ...]
    timeframes: tuple[str, ...]
    warmup_limit: int
    reconnect_max_seconds: int
    pivot_left_right: int
    pullback_min_depth: float
    pullback_max_depth: float
    pullback_max_bars: int
    zone_atr_width: float
    trigger_min_body_fraction: float
    four_bar_horizon: int
    credentials_path: Path
    ws_silent_seconds: int = 15
    rest_poll_seconds: int = 5

    @classmethod
    def load(cls, path: Path) -> "Settings":
        raw = yaml.safe_load(path.read_text(encoding="utf-8"))
        app, binance, strategy, security = raw["app"], raw["binance"], raw["strategy"], raw["security"]
        credential_path = security.get("credentials_path") or os.path.join(
            os.environ.get("LOCALAPPDATA", ""), "CodexBinance", "credentials.dpapi"
        )
        return cls(
            host=app["host"], port=int(app["port"]), rest_base=binance["rest_base"],
            websocket_base=binance["websocket_base"], symbols=tuple(x.upper() for x in binance["symbols"]),
            timeframes=tuple(binance["timeframes"]), warmup_limit=int(binance["warmup_limit"]),
            reconnect_max_seconds=int(binance["reconnect_max_seconds"]),
            pivot_left_right=int(strategy["pivot_left_right"]),
            pullback_min_depth=float(strategy["pullback_min_depth"]),
            pullback_max_depth=float(strategy["pullback_max_depth"]),
            pullback_max_bars=int(strategy["pullback_max_bars"]), zone_atr_width=float(strategy["zone_atr_width"]),
            trigger_min_body_fraction=float(strategy["trigger_min_body_fraction"]),
            four_bar_horizon=int(strategy["four_bar_horizon"]), credentials_path=Path(credential_path),
        )
