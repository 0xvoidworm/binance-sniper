# Range Breakout Reference — frozen implementation contract

Registered 2026-09-14 before running this engine on local historical outcomes. This replaces the old multi-pattern engine as the main **read-only model**, not as a profit-validated execution strategy.

## Source and adoption boundary

Reference: René Balke, 2026-02-11, https://www.youtube.com/watch?v=zYTdc0Q1BCs&t=216s . Public captions at 03:36–04:19 describe range 03:00–06:00, opposite-range stop, no take profit, no trailing, first trade only, and closure at 18:00. Cached captions are in Workspace/outputs/youtube/zYTdc0Q1BCs. This is an independent implementation of the described rules, not the vendor's EA, source code, or independently verified track record.

Explicit adaptations: Binance USD-M perpetuals instead of USDJPY/MT5; UTC session clock (the video's broker timezone is not established); existing six-symbol universe; daily including weekends; closed M5 OHLC modeling instead of tick/broker execution. These differences prevent a claim of exact performance replication. No discretionary trend filters, retracement gates, score ranking, recovery, or optimization are added. Source claims do not certify this adaptation.

## Single fixed rule set

- Range: high/low of all 36 closed M5 candles in [03:00,06:00) UTC, displayed also as 10:00–13:00 WIB.
- Entry window: [06:00,18:00) UTC. Model a buy-stop at range high and sell-stop at range low; first side to touch triggers and cancels the other. One modeled position per symbol/day.
- Stop: opposite range boundary. Gap entry/stop uses available open plus adverse modeled slippage. No take-profit and no trailing; the UI must not invent either.
- Exit: stop or last M5 close at 18:00 UTC (01:00 WIB next day). No four/eight-bar horizon. Stop takes priority over session-close on the last bar.
- If both boundaries are crossed in the same bar from an inside-range open, first direction is unknowable: mark AMBIGUOUS, do not invent an entry or profit. Report those sessions separately; resolved-only statistics are not all-session profitability.
- Missing/revised/noncontiguous data must not be treated as a clean session. Preview candles never trigger the model. Zero-width/nonfinite/invalid OHLC data cannot produce a trade.
- Fee/slippage assumptions: base 5/2 bps per side; stress 7.5/5 bps. Costs are not actual verified account fees. Historical funding is included when supplied, with conservative ambiguity at entry/exit bars. Live funding is not available: show after-fee/slippage separately; all-cost net stays null.
- Each result is hypothetical per-unit/event output. It is not a broker fill, dollar account return, margin/liquidation simulation, or position sizing recommendation.

## Evidence and operation

One engine shared by live closed-bar reconstruction, synthetic fixture, and offline historical replay. Source/config fingerprints isolate namespaces. Persist session state and event identity atomically. Completed results are immutable; corrections are flagged, not rewritten. Backfilled sessions are RECONSTRUCTED, never new live executions. Startup after a historical breakout cannot imply an order was sent.

The already-seen 450 historical days may be replayed for exploratory diagnosis only. No parameter search, pair selection, or promotion based on those results. Preserve old Phase 2/3 results unchanged. Disable their live research loops; keep legacy code/tests for reproducibility. Do not enable orders, cancel, leverage, margin, or change API whitelist policy.

## UI contract

Main workspace: one Range Breakout engine; actual range box, high/low, modeled entry/stop, held duration, and explicit reason for holding/exit. Market transport, hypothetical position state, and actual account status remain distinct. No confidence score, target-R card, H4→M5 strategy gates, or four-candle countdown. Session timezone and model/no-order status always visible. Historical analysis, today's reconstruction, synthetic demonstration, and legacy archive are explicitly separated.
