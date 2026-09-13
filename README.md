# Binance Sniper — Phase 2 Research Terminal

Read-only Binance USDⓈ-M Futures market observer with a causal H4 → H1 → M15 → M5 pipeline. It has no order, cancel, leverage, margin, or account-mutation capability.

## Setup and run

```powershell
.\scripts\setup.ps1
.\scripts\run.ps1
```

Open `http://127.0.0.1:8765`. Refresh an older browser tab after upgrades. The amber terminal includes Market Watch, a zoomable/pannable chart with crosshair, H4/H1/M15/M5 context, scanner explanations, and Journal / Setup history / Connection / Account tabs. Choose a timeframe in the toolbar or context strip; choose an archived candidate from Setup history and use Reset view to return to current context.

Use `http://127.0.0.1:8765/?mode=replay` for an explicitly synthetic six-frame example. Play, step, or scrub from the armed zone through the trigger and four outcome candles. Replay uses its own symbol list and does not display live account data. This example is not market-performance evidence.

Inspect the service with `.\scripts\status.ps1` and stop it with `.\scripts\stop.ps1`.

The account panel loads `%LOCALAPPDATA%\CodexBinance\credentials.dpapi` only in memory. Credentials are not copied into this repository. `ipRestrict=false` is an accepted operator policy and appears as a warning, not a blocker.

## What Phase 1D proves

- Validated closed bars drive every analysis decision; the current candle is visual preview only.
- Per-feed REST fallback remains live independently of a silent WebSocket.
- Zone, candidate, transition ledger, and four-bar outcome are deterministic and restart-safe.
- Replay exercises the production cache and scanner through regime, impulse, pullback, zone, trigger, candidate, and four-bar completion.
- Account access is GET-only and separates regular, conditional, and execution-algo orders.

## Test

```powershell
.\.venv\Scripts\python.exe -m pytest -q
.\.venv\Scripts\python.exe -m pip check
node --test tests/terminal-model.test.cjs
```

This phase validates correctness and observability. It does not prove profitability and intentionally contains no trading execution.

## Phase 2: measured baseline

Open the **Research · Phase 2** terminal tab. Historical results and the live observation journal are separate. **Lihat candle** opens the stored M5 evidence for an event; its four post-signal candles are explicitly hindsight, not decision-time inputs.

The first frozen 90-day study (2026-06-15 through 2026-09-12 UTC) processed 228,840 public candles including warmup, 155,520 M5 decisions, and 27 signals. Modeled net mean was **-0.396R** in development (16 samples) and **-0.403R** in holdout (11 samples). This baseline has not demonstrated an edge. R is event risk-distance, not account return or a fixed percentage of capital.

Reproduce with the existing project environment; no new dependency is required:

```powershell
.\.venv\Scripts\python.exe -m binance_sniper.research.download --days 90
.\.venv\Scripts\python.exe -m binance_sniper.research.study
```

Download uses public market-data GET requests. Study replays local SQLite data, without network access. The runtime separately journals live observations to `state/research.sqlite3`. Research GET endpoints only read stored reports/events; opening the panel never starts a study or an order.

See [Phase 2 methodology and findings](docs/PHASE2_RESEARCH.md), `STATUS.md`, and `TEST_RESULTS.md`. Historical JSON export: `C:\Users\yuri\Desktop\Workspace\outputs\binance-sniper-phase2-90d.json`.
