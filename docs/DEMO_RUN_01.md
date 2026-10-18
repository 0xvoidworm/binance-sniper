# Official EA demo — actual run 01 (14 September 2026)

The official free Market demo is now installed and its first visual Strategy Tester run completed. Manual MQL5 login was successful; do not treat authentication as still blocked. Computer-use kept the run inside Strategy Tester, with no purchase, VPS, live attachment, Algo Trading activation or real-account mutation.

## Actual settings and result

- Range Breakout EA with Range Filters, Jimmy Peter Eriksson, product 122237, version 5.20, FBS-Real build 6184.
- Untouched vendor input defaults; One Chart Setup true and all five market inputs true. Risk enum values are retained in the exported set, not interpreted as guessed percentages.
- XAUUSD H1 primary chart; 2026-09-01 00:00 through 2026-09-13 00:00, broker-local end exclusive. UI preset Last month resolved to these bounds.
- Simulated deposit 10000 USD; leverage 1:100; execution delay 100 ms; optimization disabled; visual mode enabled; profit-in-pips disabled; Every tick based on real ticks requested and confirmed in agent log.
- 22 reported trades, 44 buy/sell deals, plus initial deposit. Final balance 10671.44 USD, net +671.44 USD (+6.7144% of initial deposit).
- Reported profit factor 1.98; maximal equity drawdown 406.57 USD (3.70%); maximal balance drawdown 351.56 USD (3.22%). These DD metrics are from MT5, not reconstructed equity data.
- Actual report ledger reconciled every balance movement and net total, with zero errors. Dashboard GET result and visible metrics/45-row ledger/balance chart match.

Net by reported deal symbol, including commission and swap (no hindsight market selection):

| Symbol | Entry deals | Exit deals | Net USD |
| --- | ---: | ---: | ---: |
| BTCUSD | 7 | 7 | 96.82 |
| US30 | 8 | 8 | 98.85 |
| USDJPY | 4 | 4 | 584.19 |
| XAUUSD | 3 | 3 | -108.42 |

## What this does not prove

1. Only one short compatibility sample; not an independent holdout, multi-year result, exact IC Markets replication or proof of future/live profits.
2. Report history-quality text is `83% real ticks`; log shows XAUUSD real ticks start on 2026-09-03, later than the requested start. Do not describe the full test as 100% real-tick covered. Other sampled markets report real ticks beginning September 1.
3. Only four markets actually participated. DE40 input is true but there is no DE40 symbol initialization/trade sequence in this run. Cause not established; broker alias/availability or EA handling needs a specific check, not an invented explanation.
4. Vendor logs contain daily OCS randomization values 0–10. Seed/mechanism not exposed in exported inputs. A repeat has not been run; do not claim reproducibility or deterministic vendor internals.
5. Report commission is 0 USD and swap -1.19 USD. These are observed report values, not proof the real broker's complete costs are represented. Cost model, timezone and frozen dataset/specification identity remain unverified.
6. Exported INI contains relative `Dates=1`, not fixed FromDate/ToDate. Capture fixed boundaries before repeating another day. The original file is preserved unchanged.
7. The dashboard is an evidence viewer: deal-balance chart, not live execution, price/zone replay or reconstructed equity. MT5 visualizer itself showed the genuine price replay and simulated orders.

## Artifacts and identity

Artifact directory: `%USERPROFILE%\Desktop\Workspace\outputs\range-breakout-demo-2026-09-14`

- `PROTOCOL.md`: written before pressing Start.
- `vendor-default-v5.20.set`: exported before Start; no inputs changed.
- `run-01-fbs-20260901-20260913.htm` and four MT5-generated PNG companion charts.
- `run-01-tester.ini`: UI-exported configuration after the run, settings unchanged.
- `run-01-agent.log`: copied completed agent log.
- `run-01-context.json`: operator-declared context; unknown values null.
- Installed binary: `%USERPROFILE%\AppData\Roaming\MetaQuotes\Terminal\D0E8209F77C8CF37AD8BF550E51FF075\MQL5\Experts\Market\Range Breakout EA with Range Filters.ex5`.

Run ID: `2c033220a6748fb8e65adfa55466b4f5f15408b4b339c96b6ff5c3ecb61229ba`.

| Artifact | SHA256 |
| --- | --- |
| Binary | cf6ab14b12da7aeebf6d2cdb65182b182848287d3b9d76e7b080d6d254c8f38e |
| Report HTML | 0b3d5ff626f57d1b74100325f95bdfaca750c90fbd551d996835ec4baf73f5b4 |
| Set | e0cc2d86a092082eab664756f74fa8e8b46b055eec3824b903a3adfa01510f5c |
| Tester INI | e587154d8b313b31f823653a6b664ff5eea324f91f1c26e0e5235a17932b57a6 |
| Agent log | 18db2bfbd52df8416f5bd432fc74923e372b732cf4bb814ee8e69fb0670c4cb0 |

Report status `REPORT_RECONCILED`; repeat gate still missing `broker_timezone`, `cost_model`, `dataset_id`. Hashes preserve identity, not independent execution attestation. Raw HTML/logs remain local. No application code or dependency changed; prior 130 Python / 19 JS suite was not rerun for data-only import.

Next: resolve DE40 and real-tick coverage, record fixed-date broker specs/costs, then run a preregistered repeat and broader periods preserving every outcome. Do not increase risk or select USDJPY solely from this sample.
