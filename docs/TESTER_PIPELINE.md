# MT5 evidence pipeline — 14 September 2026

## Latest result (supersedes the archived implementation snapshot below)

User completed manual MQL5 login. Official product 122237 / Jimmy Peter Eriksson / v5.20 demo is installed and one visual compatibility test completed. Current store: **1 candidate + 1 legacy**. The candidate report reconciles 44 trading deals, net +671.44 USD, 22 trades, reported PF 1.98 and equity DD 3.70%. Quality 83%; XAUUSD real ticks start September 3 although test starts September 1; four traded markets and no DE40; randomization logged. These limitations prevent interpreting this as established profitability. Full artifacts, provenance and next actions: [DEMO_RUN_01.md](DEMO_RUN_01.md). No purchase, live order, VPS or Algo Trading activation. The scope/current-result and initial-verification paragraphs below are the preceding implementation snapshot, retained for history.

## Scope and current result

Main UI: **Research Desk** (`/` and `/tester`). It tracks the original Jimmy Eriksson candidate, not a rewritten approximation. Existing Binance reference model, negative exploratory results and read-only account remain at `/market`.

Implemented: **offline result ingestion and inspection**, not an automated MT5 runner or a profitable execution engine. Official MT5 Market product 122237 / Jimmy Peter Eriksson / v5.20 was inspected. The Download demo action was **not clicked**: installation awaits the action-time confirmation required by the computer-use skill. No purchase, subscription, live order, Algo Trading activation or exchange permission change occurred.

Current production store: **0 candidate reports; 1 old SniperFX parser-check report**. Eight existing strategy/integration reports passed a read-only format/reconciliation check; one unit-harness report was rejected for unsupported period/currency. These are not eight new backtests or independent strategy samples.

## Flow and commands

1. Export a completed **English MT5 HTML Strategy Tester report**. A screenshot, optimization summary or account statement is not interchangeable.
2. Copy `config/tester-candidate-context.json` to a separate run context, then record actual version, purpose, broker timezone, tester model, delay, costs, dataset identity and randomizer settings. Unknown fields stay `null`.
3. Import with explicit paths. Optional binary/set/config/log arguments hash existing artifacts; none is executed.
4. Parse and reconcile, then atomically commit original HTML and sanitized result to local SQLite. Identical bundles are idempotent; ordinary update/delete is prohibited by database triggers.
5. Dashboard reads GET-only routes. Press **Refresh bukti** after imports. No automatic export, test runner or folder watcher is claimed.

From the project directory:

```powershell
.\.venv\Scripts\python.exe -m binance_sniper.tester --help
.\.venv\Scripts\python.exe -m binance_sniper.tester status
.\.venv\Scripts\python.exe -m binance_sniper.tester import --report 'C:\path\run-A.htm' --context 'C:\path\run-A-context.json' --binary 'C:\path\Range Breakout EA with Range Filters.ex5' --set 'C:\path\run-A.set' --tester-config 'C:\path\run-A.ini' --tester-log 'C:\path\run-A.log'
.\.venv\Scripts\python.exe -m binance_sniper.tester compare LEFT_RUN_ID RIGHT_RUN_ID
```

Optional `--db` goes **before the subcommand** for an isolated test store. Exit codes: 0 success; 1 malformed/unsupported input or IO failure; 2 imported report failed numerical reconciliation. Unsupported data is rejected before storing; numerical failures are preserved without a balance chart/comparable-repeat verdict. CLI errors do not dump file contents or secret-bearing paths.

Storage: `state/tester-evidence.sqlite3` (git-ignored). Raw HTML may contain private inputs/account details: it is local only and has no HTTP route. API excludes arbitrary input values, comments and account names; report inputs are represented by a hash. Never put credentials in context JSON or upload raw DB/reports without review.

## Validation and interpretation

- Accepts UTF-8/BOM and BOM-marked UTF-16; report limit 25 MiB / 100,000 HTML rows. Required summary/deal fields, finite dot-decimal values, positive trading volume/price, broker timestamps, period boundaries, chronology and duplicate IDs are checked.
- Buy/sell deal count must match Total Deals; initial deposit is separate. Each balance movement must match profit + commission + swap; aggregate ledger net must match reported net within 0.02 account-currency units.
- One initial deposit only. Extra balance adjustments, unsupported fee/credit deal types, localized decimal commas, unsupported locale/currency and partial exports fail closed. Two-decimal currency precision is assumed by the reconciliation tolerance.
- PF, drawdown, gross profit and Total Trades are **reported metrics**, not independently reconstructed equity/position statistics. Partial exits/reversals are not naively paired into trades. Definitions: [official MT5 testing report](https://www.metatrader5.com/en/terminal/help/algotrading/testing_report).
- History Quality is **not real-tick coverage**. Model, costs and data identity require separate capture. Zero reported commission is a warning, not proof of cost-free trading.
- Chart uses reported deal balances and unconverted broker time; it is **balance, not equity**. HTML lacks equity series, candles, zones and per-trade MFE/MAE: none is invented. Failed reads clear the displayed results rather than retain an apparently current result.

## Repeat comparison

Requires equal input hash, binary/set/config hashes, expert/version, symbol/timeframe/period, deposit/currency/leverage, server/build and declared dataset/cost/timezone/model/delay/randomizer context. Missing provenance or failed reconciliation returns `NOT_COMPARABLE`.

`MATCH_REPORTED_OUTPUT` / `DIFFERENT_REPORTED_OUTPUT` compare metrics and the complete reported deal sequence. An identical import is not another experiment; byte-identical reports need distinct supplied run-log hashes to pass the duplicate-evidence gate. This does **not attest independent execution**. Logs/artifacts are hashed, not independently verified; broker historical files/specifications are not automatically frozen. A changed result does not diagnose randomization or prove an improved strategy. No optimizer, best-run selection or profitable-pair promotion was added.

Hashes/append-only SQL protect ordinary integrity and overwrite, not against a malicious filesystem owner. Operator-declared context is not execution attestation. Local FBS differs from the IC Markets reference, so a local result is a compatibility test, not exact replication or proof of live profitability.

## Actual legacy integration evidence

Source: `Workspace/outputs/sniperfx-mt5/mt5-backtest-sandbox/reports/backtest-eurusd.htm`.

- Source SHA-256: `a1179eb47639bc71e258aaa7a028710092e5c89ed400c33459ddcfb93630a2d4`.
- Import ID: `28baf79aa5108c98baa2914fa3c30d4fba6399d805384f866fdfdf6c9a54897a`.
- SniperFX / FBS-Real build 6184 / EURUSD M5 / 2026-01-01–2026-09-01.
- Deposit 10,000 USD; 11 reported trades, 22 trading deals plus deposit; net -219.09 USD; final balance 9,780.91 USD; reported PF 0.45. **Old SniperFX results, not Jimmy EA results.**

## Verification and next action

Python **130 passed**, JavaScript **19 passed**, clean `pip check`, one existing Starlette/AnyIO deprecation warning. Tests cover parsing/reconciliation failures, secret exclusion, identity separation, idempotence, restart, concurrent duplicate imports, immutable storage, corrupt-store 503, GET-only routes, and comparison gates. No new dependency.

Browser: desktop 1440 px, mobile 390 px (no page-level overflow), real archive metrics/23 ledger rows/balance chart, empty candidate view, offline refresh clears results. No online console messages after recovery. Native screenshots inspected inline; no screenshot file claimed. This is not a multi-day soak or EA profitability test.

Current next action: investigate DE40 participation, freeze a fixed-date dataset/specification/cost context, and evaluate repeatability with vendor randomization recorded before broader tests. Do not tune/select runs based on this first profit. The approved official demo has already been installed/run; do not repeat the old login/installation blocker. Preserve all runs, configuration and logs; do not buy or attach to a live chart. [Official candidate](https://www.mql5.com/en/market/product/122237), [vendor test guide](https://www.mql5.com/en/blogs/post/771906). Vendor internals/presets were not fabricated.
