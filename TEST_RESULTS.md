# Test Results

Updated: 2026-09-13

## Latest: Phase 2

- Python: **63 passed**, one existing Starlette/AnyIO dependency deprecation warning. JavaScript model: **8 passed**. Research/terminal JS syntax, Python compile, and `pip check` pass.
- New tests cover next-open rather than signal-close fills; long/short fee/slippage; missing funding; conservative funding timing; stop/target ambiguity; gap stop; invalidated entry; incomplete/gapped horizons; preview/future exclusion; purge/embargo; empty metrics; immutable completed outcomes; SQLite restart/dedupe; live/historical separation; recorded event-window bounds; GET-only research routes; and warmup anti-backdating.
- One run exposed a pre-existing Windows timer race: a planned-renewal timeout could be labelled WS_SILENT when the clock read fell slightly before the deadline. Classification now uses the deadline that was armed; the regression suite passes.
- Public download: 228,840 validated candles with 24/24 complete feed windows; 1,620 funding records. Final causal study: 155,520 decisions, 27 signals, 16 development / 11 holdout; modeled net means -0.395672R / -0.403454R. Repeat instrumentation run has identical signals/outcomes, not new independent evidence.
- Isolated browser integration: Research API/assets 200; historical/live source switch displays distinct samples; event dialog returns 54 recorded M5 candles and matching levels/model outcome; no JS console errors during normal testing. Frequent live renders preserve expanded research details. Responsive document width verified at 390px. Screenshot inspected inline; saving through the browser tool to Workspace/outputs was denied by that tool's path policy, so no screenshot file is claimed.
- Service restarted using verified project PID/port ownership. Latest owner PID **26676**; **28/28 tasks RUNNING**, **24/24 LIVE_REST_FALLBACK**, max lag **0**; research recorder RUNNING with no restarts. The first new M5 close created **6 live decision rows**, distinct from historical rows. Account CONNECTED; positions and all order counts **0**.
- No execution path was added. IP restriction policy, account credentials, margin, leverage, and order state were not modified. A normal closed-candle capture and synthetic restart test are not a multi-day live soak.

The remaining sections record earlier phases and should not be mistaken for additional current validation samples.

Offline command: `.\.venv\Scripts\python.exe -m pytest -q`

Result after terminal UI patch: `48 passed`, with one third-party Starlette `TestClient` / AnyIO alias deprecation warning. Project lifespan warnings are gone. `pip check` reports no broken requirements. JavaScript: `node --test tests/terminal-model.test.cjs` reports `8 passed`; `node --check web/assets/terminal.js` succeeds.

Terminal-specific verification:

- REST warmup/update preview is `closed=false`, remains outside the decision cache, and is removed when finalized. Delayed WS previews cannot resurrect a finalized candle.
- Python decision explanations expose the actual neutral-H4 blocker and preserve dependency states. No strategy threshold was changed.
- Six independent replay frames progress from ARMED to TRIGGERED and four completed outcome candles. Each frame excludes later candles/events.
- JavaScript tests cover offline/stale snapshots, HTTP fallback, replay symbol selection, zone-matched candidate selection, explicit archive selection, and missing-value freshness/distance behavior.
- Isolated browser: ETHUSDT → REPLAY selected BTCUSDT automatically; step exposed TRIGGERED with matching candidate/zone and zero outcome bars; play ended at COMPLETED with four bars and gross final 1.6R in the synthetic fixture.
- Browser offline emulation retained DISCONNECTED through repeated renders, with “Data belum siap” and a cached-snapshot chart label. No Binance service network setting was changed.
- Desktop and 390px mobile visual checks verified the terminal layout. Mobile document width stayed 390px; chart bitmap matched CSS size × devicePixelRatio. Fixed an intrinsic canvas sizing issue found during resize QA.
- No application console warnings/errors during normal live/replay inspection. Expected network errors occurred only during explicit offline emulation.
- The Windows Computer Use helper stopped native browser control because it could not confidently determine the current browser URL. Native control was not retried after that stop. Visual and interaction checks were completed in the separate browser debugging context; the user's existing tab may still require manual refresh.

Runtime after this patch: verified owner PID 8768, 24/24 LIVE_REST_FALLBACK feeds, lag 0, 27/27 tasks RUNNING, account CONNECTED, all order counts 0. This is a point-in-time check; the earlier Phase 1D soak below remains historical evidence rather than a new multi-day soak.

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
