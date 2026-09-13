# Known Gaps

- Binance market WebSocket connects but does not deliver payloads on this local network. Silent reconnect behavior is tested deterministically; a real WS recovery handover still requires a network where payloads arrive.
- The longest completed runtime observation is a short local soak, not a multi-day reliability run.
- Strategy parameters and score weights are explicit hypotheses. They have not yet passed an event study, walk-forward analysis, fee/slippage model, or profitability validation.
- Persistence uses small atomic JSON ledgers suitable for this observer phase, not a transactional event database.
- Execution, leverage selection, liquidation modeling, and live order management are deliberately absent.
- The terminal chart pans/zooms within the most recent 80 closed bars exposed by the existing snapshot API. It is not an unlimited historical chart or a detachable native MT5 window system.
- Replay playback currently covers the deterministic synthetic six-frame example. Market-data replay and statistical event studies remain separate future work.
