from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import os
import yaml
import re
from urllib.parse import urlparse

TIMEFRAMES = ("4h", "1h", "15m", "5m")
SYMBOL_RE = re.compile(r"^[A-Z0-9]{5,20}$")


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
        settings = cls(
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
            ws_silent_seconds=int(binance.get("ws_silent_seconds", 15)),
            rest_poll_seconds=int(binance.get("rest_poll_seconds", 5)),
        )
        settings.validate()
        return settings

    def validate(self) -> None:
        if self.host != "127.0.0.1" or not 1024 <= self.port <= 65535:
            raise ValueError("Dashboard must bind to localhost and a valid port")
        if self.rest_base != "https://fapi.binance.com" or self.websocket_base != "wss://fstream.binance.com/stream":
            raise ValueError("Official Binance hosts required")
        if not self.symbols or any(not SYMBOL_RE.fullmatch(symbol) for symbol in self.symbols):
            raise ValueError("Invalid symbol universe")
        if self.timeframes != TIMEFRAMES or self.warmup_limit < 40 or self.ws_silent_seconds <= 0 or self.rest_poll_seconds <= 0:
            raise ValueError("Invalid feed configuration")
        if not 0 < self.pullback_min_depth < self.pullback_max_depth <= 1 or self.pullback_max_bars < 1:
            raise ValueError("Invalid pullback configuration")
        if self.four_bar_horizon < 1 or not 0 < self.trigger_min_body_fraction <= 1:
            raise ValueError("Invalid strategy configuration")
