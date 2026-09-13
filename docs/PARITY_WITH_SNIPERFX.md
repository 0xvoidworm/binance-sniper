# Parity With SniperFX

| MT5 baseline | Python observer |
|---|---|
| H4 regime | `analysis/regime.py` confirmed pivots plus efficiency |
| H1 impulse | `analysis/impulse.py` confirmed pivot origin and body quality |
| M15 pullback/origin zone | `analysis/zones.py` ATR-scaled origin zone |
| Closed M5 reclaim/micro-break | `analysis/triggers.py` |
| Lexicographic candidate ranking | `analysis/ranker.py` |
| Timer/cache health | REST warmup plus WebSocket `market_stream.py` |
| CSV audit | Runtime event buffer and JSON API in phase 1 |

Differences: Binance uses millisecond exchange timestamps and kline `x` finality instead of MT5 series indices. This implementation selects only final klines for strategy decisions. It does not place pending orders, calculate account risk, or reconcile trades because phase 1 is observation-only.

The repair baseline additionally enforces the configured pullback depth/max-bars gate, directional impulse evidence, persistent zone identities, material lifecycle events, and a four-bar observation primitive. Binance transport health is stricter than the original MT5 timer: REST warmup is `REST_WARM`, WS payloads are required for `LIVE_WS`, and silent WS uses `LIVE_REST_FALLBACK`.
