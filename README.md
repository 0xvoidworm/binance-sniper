# Binance Sniper Phase 1D

Read-only Binance USDⓈ-M Futures market observer with a causal H4 → H1 → M15 → M5 pipeline. It has no order, cancel, leverage, margin, or account-mutation capability.

## Setup and run

```powershell
.\scripts\setup.ps1
.\scripts\run.ps1
```

Open `http://127.0.0.1:8765`. Use `http://127.0.0.1:8765/?mode=replay` for the deterministic full-overlay proof. Inspect the service with `.\scripts\status.ps1` and stop it with `.\scripts\stop.ps1`.

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
```

This phase validates correctness and observability. It does not prove profitability and intentionally contains no trading execution.
