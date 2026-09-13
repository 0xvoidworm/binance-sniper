# Known Gaps

- Binance WS is silent on this network; the production REST fallback is active and correctly labeled. WS recovery reconciliation needs a longer controlled outage test.
- The dashboard canvas is a lightweight audit chart; advanced time/price axes and replay fixture controls remain Phase 2 work.
- Four-bar tracker primitives exist, but full candidate persistence/replay and MFE/MAE event ledger integration remain Phase 2 work.
- No order mutation endpoint exists by design.
- Live smoke depends on network and Binance availability; offline tests do not prove production connectivity.
