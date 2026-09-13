# Binance Sniper Phase 1

Read-only live observation scanner for Binance USDⓈ-M Futures. It uses production public REST/WebSocket market data, causal closed-bar H4/H1/M15/M5 analysis, and a local dark dashboard. It has no order endpoint and cannot submit, cancel, or mutate orders.

## Setup

```powershell
.\scripts\setup.ps1
```

## Run

```powershell
.\scripts\run.ps1
```

Open `http://127.0.0.1:8765`. Stop with `.scripts\stop.ps1`; inspect with `.scripts\status.ps1`.

The account panel loads `%LOCALAPPDATA%\CodexBinance\credentials.dpapi` in memory. Credentials are never copied to this project. Public market scanning continues if private authentication is unavailable. `ipRestrict=false` is shown as an informational warning only.

## Test

```powershell
.\.venv\Scripts\python.exe -m pytest -q
```

Live smoke is separate from offline tests and must never be used as the only acceptance proof.
