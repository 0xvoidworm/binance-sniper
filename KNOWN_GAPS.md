# Known Gaps

- The current dashboard canvas uses a lightweight live chart projection; richer OHLC plotting and overlays can be expanded in Phase 2 without changing the scanner contract.
- Private snapshot currently reads a bounded BTCUSDT open-order view; it does not enumerate every symbol's conditional order history.
- Zone state persistence/replay and four-bar MFE/MAE storage are baseline placeholders for the next research phase.
- No order mutation endpoint exists by design.
- Live smoke depends on network and Binance availability; offline tests do not prove production connectivity.
