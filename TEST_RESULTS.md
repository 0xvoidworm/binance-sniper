# Test Results

Updated: 2026-09-13

Offline command: `.\.venv\Scripts\python.exe -m pytest -q`

Result: `44 passed`, with one third-party Starlette `TestClient` / AnyIO alias deprecation warning. Project lifespan warnings are gone. `pip check` reports no broken requirements.

Semantic replay result:

`REGIME → IMPULSE → DETECTED → ARMED → TOUCHED → TRIGGERED → CANDIDATE → COMPLETED → OUTCOME`

The deterministic long fixture records MFE `2.1R`, MAE `0.4R`, final `1.6R`, and target first-hit on outcome bar 3. Tests also cover future-candle isolation, snapshot purity, persistence reload, duplicate-bar rejection, long/short direction, zero risk, and same-bar ambiguity.

Live read-only soak after Phase 1D restart:

- Global transport stayed `LIVE_REST_FALLBACK` for all 40 samples.
- REST fallback stayed `24/24`; stale maximum `0`.
- Task health stayed `27/27 RUNNING`.
- Maximum lag was `0` bars; duplicate delta was `0`.
- Maximum preview age was about `29.9s`, matching the slowest configured 4h fallback cadence.
- Account snapshot age advanced from about `1.2s` to `31.4s` and reset on refresh, proving it is no longer a static zero.
- Account remained `CONNECTED`; regular/conditional/execution-algo/total orders were `0/0/0/0`.
- WebSocket delivered `0` valid payloads. Its substate cycled through connect/silent/reconnect while REST feed state remained stable.
- Public REST requests increased from `165` to `352` during the sample; observed used-weight telemetry ended at `338` against the advertised minute limit `2400`.

Security checks: no secret material is tracked; the Binance client exposes GET only and blocks non-allowlisted paths before network. No order or account mutation was sent.
