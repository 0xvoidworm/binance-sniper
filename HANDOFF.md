# Handoff

Project: `C:\Users\yuri\Desktop\Workspace\projects\binance-sniper`

Entrypoint: `binance_sniper.app:app`, bound to `127.0.0.1:8765`. The service is observation-only. Binance mutations are not implemented and non-allowlisted paths are rejected before network access.

Runtime design:

- `MarketStream` owns WebSocket connect/watchdog/reconnect/renewal.
- Twenty-four independently supervised REST feed loops maintain live fallback with timeframe-specific cadence.
- `ClosedBarCache` separates closed decision bars from visual preview and exposes quality/freshness counters.
- `Scanner` owns the causal lifecycle; `snapshot()` is pure and cannot create candidates or advance outcomes.
- `ZoneLedger` and `FourBarTracker` persist atomically under `state/`.
- `/api/replay` runs a deterministic production-pipeline fixture with no external mutation.
- `/api/account` computes snapshot age at read time and preserves last-known data as `STALE` on refresh failure.
- `/assets` serves local terminal CSS and JavaScript. `terminal-model.js` contains testable view selection/freshness rules; `terminal.js` owns the interactive chart and mode lifecycle.
- `presentation.py` projects backend scanner gates into Indonesian explanations. `MIN_EFFICIENCY` is shared with the existing H4 strategy rule.
- Snapshot includes zone identities and public tick-size metadata. Frontend overlays must match the selected zone and candidate; archived candidates are explicitly selected.
- `/api/replay` also exposes six independent synthetic frames for step/play controls. Live account information is hidden in replay mode.

On this network, Binance market WebSocket connections remain silent. The runtime therefore reports the WebSocket substate honestly while keeping all validated REST feeds independently live. Do not infer strategy profitability from transport health or replay correctness.
