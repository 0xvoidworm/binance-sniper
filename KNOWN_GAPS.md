# Known Gaps

- Binance market WebSocket connects but does not deliver payloads on this local network. Silent reconnect behavior is tested deterministically; a real WS recovery handover still requires a network where payloads arrive.
- The longest completed runtime observation is a short local soak, not a multi-day reliability run.
- The initial 90-day event study has only 27 signals and negative modeled net expectancy in both fixed partitions. This baseline has not demonstrated an edge; no independent walk-forward or forward profitability validation exists.
- Scanner lifecycle remains in atomic JSON; research journal uses SQLite WAL. Neither has a multi-day outage/restart soak. SQLite decision journaling and scanner state are not a single atomic transaction; a long crash can leave explicitly missing observations.
- Execution, leverage selection, liquidation modeling, and live order management are deliberately absent.
- The terminal chart pans/zooms within the most recent 80 closed bars exposed by the existing snapshot API. It is not an unlimited historical chart or a detachable native MT5 window system.
- Interactive step/play replay remains a synthetic six-frame example. Historical production-scanner replay now runs offline; the terminal's historical event dialog is a fixed hindsight window, not a tick-by-tick visual simulator.
- Fill model uses M5 OHLC, assumed 5 bps fee plus 2 bps slippage per side (stress: 7.5 + 5), no order-book depth, queue, latency fill, tick-size rounding, sizing, or portfolio allocation. Funding timing on intrabar exits is uncertain; possible charges are included conservatively.
- Live journal does not reconcile actual funding yet: all-cost net is null, before-funding outcomes are separately labelled. Historical public funding is available for all six pairs in this dataset.
- Historical next-open fills assume immediate decision availability. Actual feed delays are journaled for live decisions but not simulated as executions. Delayed/gapped observations are not proof an order could have been filled.
- Gate totals describe the displayed decision state each M5 close; archived zones can remain visible. They are not counts of independently rejected trade opportunities. No counterfactual profitability is assigned to blocked bars.
- The baseline's zone is centered on impulse origin, not automatically on the accepted 25–70% pullback area. Studying an alternative geometry is a new strategy hypothesis, not an aesthetic UI change.
