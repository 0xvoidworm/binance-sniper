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
- GET `/api/research` and `/api/research/event` serve reports and bounded recorded M5 evidence. Phase 3 extends the window to 50 preceding + actual horizon (4 or 8; hard cap 82) candles. Event endpoint uses parameterized SQL and bounded IDs. No run-start, mutation, or order route.
- `web/assets/research-panel.js` + `research.css`: separate historical/live sources, evidence dialog, explicit hindsight shading. Memoized renders preserve details/scroll while live snapshots stream.

Completed study: `history-6d32c2f80979f7e5dec0486f`. Source fingerprint: `17473b25bc8a6cf97154ea957a9810481a2688d4b59f5da64a35500292ceae72`. An earlier instrumentation run remains in SQLite for provenance; it produced the same 27 signals and outcomes, not an independent validation sample. Latest report is displayed first per mode.

Do not silently tune against the already-viewed holdout or turn this baseline into a live trading engine. Next research should predeclare a new pattern hypothesis, examine the origin-centered zone geometry and 20-minute horizon, and reserve genuinely new validation data. A change of source code creates a new journal/study version.

## Phase 3 handoff — NO_PROMOTION

The above Phase 2 suggestions were tested in Phase 3; they are no longer untested ideas. Read `docs/PHASE3_PROTOCOL.md` and `docs/PHASE3_RESULTS.md` before another strategy change. Both the original 90 days and the new 360 days have now been seen. Do not recycle either as a fresh holdout.

- `strategy_lab/strategies.py`: eight immutable variant definitions, causal H1/H4 context, retracement/retest/sweep areas, directional trigger, volatility stop floor, target-room/cost checks, area expiry/cancellation, one trigger per area, same-symbol horizon cooldown. Shared `PatternEngine.step` drives historical replay and experimental live observation. Score is a cost-feasibility heuristic, NOT a probability of profit.
- `strategy_lab/evaluation.py`: training-prefix selection, uniform eight-bar purge/embargo, three expanding walk-forward folds and final holdout. Deterministic seven-day calendar-block bootstrap clusters symbols together. Failed gates are reported, not relaxed after seeing results.
- `strategy_lab/runner.py`: fixed dated protocol, feed checksum verification, source/config/protocol/data-manifest fingerprints, resumable per-symbol historical replay and daily gate counts. Historical trials live in `lab_trials/lab_gates/lab_progress`; their `LAB` runs do not pollute Phase 2 latest reports.
- `strategy_lab/live.py`: supervised `strategy-lab` task captures only new, coherent closed-M5 decisions after its start. Eight hypotheses remain EXPERIMENTAL_MONITOR; this is data collection, not strategy promotion. Signal, modeled outcome, and engine state commit in one SQLite transaction. A failed capture propagates to supervision/reload; completed outcomes stay immutable. Source-bound shadow namespace also fingerprints the adapter. SQLite busy timeout: 200 ms.
- GET `/api/lab`, `/api/lab/events`, `/api/lab/chart`: stored result, latest 30 variant events, and at most 80 actual shadow M5 candles. Parameter bounds and unknown-ID 404s tested. No mutation routes.
- `lab-panel.js`: historical results/experimental observation separation, evidence actions, error/stale handling, preserved details/scroll. Main chart and original scanner remain baseline; the panel says so explicitly. `research-panel.js` shares recorded event dialogs and draws frozen area bounds only when present.
- `research/download.py`: explicit dated end boundary and funding pagination past 1,000 records. `engine.py`: revision-sensitive memoization of closed H4/H1 context; baseline rules unchanged. `research/model.py`: conservative funding handling at exact gap-exit boundary.
- Internal legacy exit identifier `FOUR_BAR_CLOSE` is retained for report compatibility; actual numeric horizon is `costs.horizon`. Tests prove eight-bar completion uses the eighth close and the UI labels it `TIME STOP / 8 M5`. Do not infer four bars from that old identifier.
- `.gitattributes` preserves the registered protocol's LF bytes so its fingerprint is stable on Windows checkout. Other source/config fingerprints normalize text newlines.

Final trial `lab-e98337744f4aefe8ba8625fc`, dataset `binance-1750377600000-1781481600000-v1`. Earlier `lab-6b33792f035df1f24d6cbbac` is retained; the second run added missing config provenance, with identical validation, baseline and variant results. These are the same experiment, not two independent validations. JSON: `Workspace\outputs\binance-sniper-phase3-360d.json`.

Runtime at handoff: verified PID 25112, `http://127.0.0.1:8765/?tab=lab`. Keep `ipRestrict=false` as the existing warning policy. No credentials, leverage, margin, orders, or positions were changed. Multi-day shadow soak, live funding reconciliation and executable fills remain absent. Next work needs a genuinely new causal hypothesis/data source and a newly declared validation plan; do not cherry-pick the positive SOL subgroup of 19 holdout events or tune the already-opened holdout until it looks profitable.
