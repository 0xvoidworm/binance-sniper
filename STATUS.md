# Status

Updated: 2026-09-13

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
- [ ] Strategy edge/event study and profitability validation
- [ ] Trading execution (intentionally absent)

Latest offline verification: `48 passed` Python tests plus `8 passed` JavaScript model tests, one third-party Starlette/AnyIO deprecation warning, and clean `pip check`. See `TEST_RESULTS.md` for runtime evidence.
