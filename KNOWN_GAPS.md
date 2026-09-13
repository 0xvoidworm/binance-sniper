# Known Gaps

- Binance WS is silent on this network; the production REST fallback is active and correctly labeled. WS recovery reconciliation needs a longer controlled outage test.
- Dashboard chart has live OHLC and preview rendering, but full time/price axis labels and complete replay-mode overlay screenshot remain open.
- Four-bar outcome tracking is wired for candidates and tested, but long-running persistence/restart integration with live candidate events needs a dedicated soak test.
- Task registry/rate-limit header telemetry and planned 24-hour WS renewal remain operational follow-up work.
- No order mutation endpoint exists by design.
- Live smoke depends on network and Binance availability; offline tests do not prove production connectivity.
