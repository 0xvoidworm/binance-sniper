# Test Results

Command: `.\.venv\Scripts\python.exe -m pytest -q`

Latest offline result: `18 passed, 5 deprecation warnings`.

Coverage: security redaction, mutation rejection, signed GET shape, closed-candle gate, duplicate/out-of-order handling, pivot confirmation, deterministic ranking, and dashboard smoke.

Live smoke result on 2026-09-13: `24/24` feeds LIVE (six symbols x four timeframes), 0 reconnects at check time, account auth CONNECTED, Futures enabled, ONE_WAY, 0 positions, 0 orders. Dashboard screenshot verified closed candlesticks and local overlays/status.

Repair-gate live evidence: Binance WebSocket connected but delivered `0` valid market payloads, so runtime correctly reports `WS_SILENT` and `24/24 LIVE_REST_FALLBACK`; it does not claim `LIVE_WS`. Account snapshot reports `regular_orders=0`, `algo_orders=0`, `total_orders=0`.

`pip check`: No broken requirements found.

Live smoke is reported separately after startup. No credential, signature, account secret, or order mutation belongs in test output.
