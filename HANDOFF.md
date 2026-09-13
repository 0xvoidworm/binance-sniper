# Handoff

Project: `C:\Users\yuri\Desktop\Workspace\projects\binance-sniper`

Runtime: FastAPI/Uvicorn on `127.0.0.1:8765`. Main entrypoint: `binance_sniper.app:app`. Logs are emitted by the foreground process; no secret-bearing log is written.

The service warms production USD-M Futures public klines for six symbols and four timeframes, then consumes combined kline WebSocket streams. Only `x=true` candles update the decision cache. Private account data uses DPAPI and a GET-only endpoint allowlist. All mutation methods/endpoints are rejected before network.

Next actions: run offline tests, setup the `.venv`, start the service, verify `/api/state`, and perform a bounded live smoke. Keep the service running only if public feed is healthy; never enable execution because this phase has no execution path.
