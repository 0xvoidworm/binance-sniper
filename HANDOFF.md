# Handoff

Project: `C:\Users\yuri\Desktop\Workspace\projects\binance-sniper`

Runtime: FastAPI/Uvicorn on `127.0.0.1:8765`. Main entrypoint: `binance_sniper.app:app`. Logs are emitted by the foreground process; no secret-bearing log is written.

The service warms production USD-M Futures public klines for six symbols and four timeframes, then consumes combined kline WebSocket streams. Only `x=true` candles update the decision cache. When WS is silent, the watchdog transitions to `WS_SILENT` and polls production REST as `LIVE_REST_FALLBACK`; it never labels warmup as `LIVE_WS`. Private account data uses DPAPI and a GET-only endpoint allowlist. All mutation methods/endpoints are rejected before network.

Phase 1C verification: 20 offline tests pass, pip check is clean, and the service is running at PID 18592. WS is silent with 0 valid payloads, so every feed is labeled `LIVE_REST_FALLBACK`; preview and closed-bar freshness are populated. Account read-only is connected with regular/conditional/execution-algo/total order counts all zero. No mutation endpoint exists.

Remaining work before any future phase: complete replay-mode visual screenshot/overlay fixture, add supervised task/rate-limit telemetry, and run persistence/restart soak. Do not enable execution; this project remains observation-only.
