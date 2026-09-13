# Test Results

Command: `.\.venv\Scripts\python.exe -m pytest -q`

Latest offline result: `8 passed`.

Coverage: security redaction, mutation rejection, signed GET shape, closed-candle gate, duplicate/out-of-order handling, pivot confirmation, deterministic ranking, and dashboard smoke.

Live smoke result on 2026-09-13: `24/24` feeds LIVE (six symbols x four timeframes), 0 reconnects at check time, account auth CONNECTED, Futures enabled, ONE_WAY, 0 positions, 0 orders. Dashboard screenshot verified closed candlesticks and local overlays/status.

Live smoke is reported separately after startup. No credential, signature, account secret, or order mutation belongs in test output.
