# Status

Updated: 2026-09-14 WIB

- [x] GET-only private client with credential redaction and exact official-host pinning
- [x] Six-symbol, four-timeframe production REST warmup and per-feed fallback scheduler
- [x] WebSocket silent watchdog, counted reconnects, reconciliation gate, and planned renewal
- [x] Shared REST/WS candle validation, preview separation, freshness, lag, and rate telemetry
- [x] Stable causal impulse/zone identity and monotonic persisted lifecycle
- [x] Deterministic candidate scoring with component breakdown
- [x] Persisted four-bar directional MFE/MAE/final-R and intrabar ambiguity tracking
- [x] Semantic replay with anti-lookahead and restart/idempotency tests
- [x] Supervised task health registry and server-time refresh
- [x] Exact PID/port/project ownership scripts and stale-lock recovery
- [x] Dark dashboard with live/replay modes, axes, time/price overlays, freshness, account age, task, and rate status
- [x] MT5/Bloomberg-inspired local terminal with Market Watch, chart crosshair/zoom/pan, decision gates, and terminal tabs
- [x] Browser disconnect/freshness protection, matching candidate/zone overlays, REST preview finality, and dataset-specific symbol selection
- [x] Synthetic step/play replay through six causal frames with isolated account display
- [x] Phase 2 immutable decision/signal journal, public-data historical event study, cost model, and terminal evidence panel
- [x] 90-day causal baseline with a fixed 60/30 chronological split; 27 signals, negative modeled net means in both partitions
- [x] Phase 3 preregistered eight-variant strategy lab with causal shared historical/live pattern engine
- [x] New 360-day dataset, three training-selected walk-forward folds, final 60-day holdout, cost stress, and clustered uncertainty
- [x] Historical area evidence, explicit 4/8-bar horizons, and separate restart-persisted experimental live observation
- [ ] Demonstrated strategy edge or future/live profitability: all eight alternatives remain negative; NO_PROMOTION
- [ ] Trading execution (intentionally absent)

Latest verification: Python `80 passed`, JavaScript `10 passed`, one third-party Starlette/AnyIO deprecation warning, clean `pip check`. Latest service owner: PID 25112, 29/29 tasks RUNNING, 24/24 fallback feeds, no positions/orders. Phase 3 result: `lab-e98337744f4aefe8ba8625fc`; selected candidate `SWEEP_T1_4`, 76 holdout events with -0.227142R mean net. The implementation is more testable and observable; a profitable trading engine has NOT been established. See `TEST_RESULTS.md` and `docs/PHASE3_RESULTS.md`.
