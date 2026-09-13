# Test Results

Command: `.\.venv\Scripts\python.exe -m pytest -q`

Latest offline result: `20 passed, 1 third-party Starlette deprecation warning`.

Coverage: security redaction, mutation rejection, signed GET shape, closed-candle gate, duplicate/out-of-order handling, pivot confirmation, deterministic ranking, and dashboard smoke.

Live smoke result on 2026-09-13: `24/24` feeds LIVE (six symbols x four timeframes), 0 reconnects at check time, account auth CONNECTED, Futures enabled, ONE_WAY, 0 positions, 0 orders. Dashboard screenshot verified closed candlesticks and local overlays/status.

Phase 1C live evidence: PID `18592`; Binance WebSocket valid payloads `0`, runtime reports `WS_SILENT` globally and `24/24 LIVE_REST_FALLBACK` per feed. Preview timestamps present on `24/24`, max M5 lag `0` bars, duplicate total `0` during observed normal polling. Account snapshot age `0ms`, `regular_orders=0`, `conditional_orders=0`, `execution_algo_orders=0`, `total_orders=0`.

Replay pipeline test: `tests/test_replay_pipeline.py` runs production `MtfCache` and `Scanner` twice and compares semantic event output. Four-bar exact direction/ambiguity tests pass in `tests/test_four_bar_outcomes.py`.

`pip check`: No broken requirements found.

Live smoke is reported separately after startup. No credential, signature, account secret, or order mutation belongs in test output.
