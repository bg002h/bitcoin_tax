

### whatif-sell-btc-input
**WHAT:** `what-if sell --sell` takes an integer SAT amount; users think in BTC (0.05 BTC = 5,000,000 sat — unintuitive).
**FIX:** accept a BTC decimal (e.g. `--sell 0.05`) and convert to sat (reject ambiguous precision), or add a `--btc` alias. Surfaced whatif P0+P1 whole-diff (2026-07-06); non-blocking UX.
