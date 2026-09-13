# Known Gaps

- Binance market WebSocket connects but does not deliver payloads on this local network. Silent reconnect behavior is tested deterministically; a real WS recovery handover still requires a network where payloads arrive.
- The longest completed runtime observation is a short local soak, not a multi-day reliability run.
- Strategy parameters and score weights are explicit hypotheses. They have not yet passed an event study, walk-forward analysis, fee/slippage model, or profitability validation.
- Persistence uses small atomic JSON ledgers suitable for this observer phase, not a transactional event database.
- Execution, leverage selection, liquidation modeling, and live order management are deliberately absent.
