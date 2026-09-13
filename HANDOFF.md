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

## Phase 2 handoff

- `research/store.py`: local SQLite WAL, schema v1, datasets/candles/funding/runs/decisions/signals/outcomes. Composite keys deduplicate restarts; completed modeled outcomes cannot be overwritten. Each run is isolated by mode and source fingerprint.
- `research/download.py`: public historical klines pagination/resumption, validated OHLC/timestamps, complete feed coverage and checksums, public funding history. Existing 90-day dataset is retained under `state/`, not tracked in git.
- `research/study.py`: sequential closed-M5 clock, higher-timeframe promotion only after their close, memory-only production Scanner, explicit development/holdout purge and embargo. Warmup cannot backdate a newly discovered zone or trigger.
- `research/model.py`: next-open hypothetical entry, directional SL/TP, four-bar maximum, gap handling, stop-first ambiguity, per-side fee/slippage assumptions, funding accounting with conservative boundary treatment. This is separate from the legacy gross four-bar excursion tracker.
- `research/recorder.py`: supervised `research` task; captures live closed bars and new decisions, recovers pending observations from local history, journals delay/coherence, and refreshes reports every 30 seconds. No exchange requests from this module. SQLite lock timeout is 200 ms to bound feed blocking.
- `research/report.py`: frozen source/config fingerprint, grouped event metrics and gate/state counts. Missing funding means no all-cost `net_r`; live before-funding metrics have separate labels.
- GET `/api/research` and `/api/research/event` serve reports and at most 54 recorded M5 candles. Event endpoint uses parameterized SQL and bounded IDs. No run-start, mutation, or order route.
- `web/assets/research-panel.js` + `research.css`: separate historical/live sources, evidence dialog, explicit hindsight shading. Memoized renders preserve details/scroll while live snapshots stream.

Completed study: `history-6d32c2f80979f7e5dec0486f`. Source fingerprint: `17473b25bc8a6cf97154ea957a9810481a2688d4b59f5da64a35500292ceae72`. An earlier instrumentation run remains in SQLite for provenance; it produced the same 27 signals and outcomes, not an independent validation sample. Latest report is displayed first per mode.

Do not silently tune against the already-viewed holdout or turn this baseline into a live trading engine. Next research should predeclare a new pattern hypothesis, examine the origin-centered zone geometry and 20-minute horizon, and reserve genuinely new validation data. A change of source code creates a new journal/study version.
