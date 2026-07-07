

### whatif-sell-btc-input
**WHAT:** `what-if sell --sell` takes an integer SAT amount; users think in BTC (0.05 BTC = 5,000,000 sat — unintuitive).
**FIX:** accept a BTC decimal (e.g. `--sell 0.05`) and convert to sat (reject ambiguous precision), or add a `--btc` alias. Surfaced whatif P0+P1 whole-diff (2026-07-06); non-blocking UX.

### whatif-tui-parser-dedup
**WHAT:** the TUI what-if panel re-implements the harvest-target parser (`zero-ltcg|fifteen-ltcg|gain=$X|tax=$X`) + the refusal formatter locally, to avoid importing `cmd::` (which KAT-E10's source-gate forbids in btctax-tui). Small UI-layer duplication, not tax logic.
**FIX:** move the target parser to `btctax-core::whatif` (a `FromStr` on `HarvestTarget`) so both `cmd::` and the panel share one source. Surfaced whatif P3 whole-diff (2026-07-06); non-blocking.
