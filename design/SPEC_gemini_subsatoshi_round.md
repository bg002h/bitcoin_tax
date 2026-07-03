# SPEC — round sub-satoshi BTC amounts (fix: Gemini import blocked)

**Source baseline:** `main` @ `719e9fe` (branch `fix/gemini-subsatoshi-round` OFF MAIN — isolated from the
in-flight Cycle-2 bulk-resolve-conflict work, which lives on its own branch and touches different crates).
**Review status: R0 round 1 folded (0C / 1I / 2M / 2N — GATE BLOCKED then folded); awaiting R0 round 2.
Review: `reviews/R0-spec-gemini-subsatoshi-round-round-1.md`.**
**Lineage:** user-reported bug (2026-07-03): `btctax import ~/…/ReadOnly/*` →
`error: gemini row 2: fractional satoshi in BTC amount "0.0010216163"`. User-approved fix: **round to the
nearest satoshi.**

## The bug
`parse_btc_to_sat` (`crates/btctax-adapters/src/parse.rs:57`) converts `btc × SATS_PER_BTC` and REJECTS any
result with a fractional part (`AdapterError::FractionalSat`, parse.rs:85 / lib.rs:60), by design ("never a
silent round", parse.rs:56). But Gemini legitimately exports **sub-satoshi precision** (10 decimals) for
internal-ledger artifacts (fee splits, interest/staking accruals, averaged fills): the user's file has
**8 of 825 BTC-Amount cells** finer than a satoshi (e.g. `0.0010216163` = 102161.63 sat, `0.0997506234` =
9975062.34 sat, `-0.1156442018` = -11564420.18 sat). The FIRST data row is one of them → the entire
multi-file import aborts. The strict guard is correct for a *tax value* but wrong for a *BTC quantity* —
sub-satoshi BTC is inherently un-representable (1 sat is the min unit), so it must be normalized to the
grid, not rejected.

## The fix — round to nearest satoshi
In `parse_btc_to_sat`, replace the fractional-satoshi REJECT with **round-to-nearest**:
```rust
// was: let sats = btc * Decimal::from(SATS_PER_BTC);
//      if !sats.fract().is_zero() { return Err(AdapterError::FractionalSat { .. }); }
//      sats.trunc().to_i64() ...
let sats = (btc * Decimal::from(SATS_PER_BTC)).round();   // nearest satoshi; < 1 sat (≈ <$0.001) error
sats.to_i64().ok_or_else(|| AdapterError::Parse { .. "satoshi value out of i64 range" .. })
```
- **Rounding convention [R0-M1 — RESOLVED, do not re-open]:** use `Decimal::round()` =
  `MidpointNearestEven` (round-half-to-even). This ALREADY MATCHES the app's money-rounding `round_cents`
  (`conventions.rs:13/22`, also `MidpointNearestEven`). **Keep `.round()`; do NOT switch to
  `MidpointAwayFromZero`.** `.round()` returns an integer-valued `Decimal`, so `.to_i64()` is exact.
- **Sign preserved:** Gemini's negative BTC-Amount cells (sends) round toward the nearest sat
  (`-11564420.18 → -11564420`); `.round()` is sign-correct (the gemini adapter also `.abs()`es where it
  needs magnitude — unaffected).
- **Sub-half-satoshi → 0:** an amount `< 0.5 sat` rounds to `0` (`0.000000001` BTC = 0.1 sat → 0 sat).
  Acceptable — dust; none of the 8 real Gemini rows are dust (whole-sat part dominates → all nonzero).

## The xlsx read path [R0-I1 — the load-bearing addition]
A Gemini `.xlsx` BTC amount reaches `parse_btc_to_sat` only AFTER the read layer stringifies the cell:
`cell_to_string` (`crates/btctax-adapters/src/read.rs:169`) does `Data::Float(f) => format!("{f}")`.
Gemini stores amounts as **numeric** cells (`Data::Float`), so the sub-sat value flows through
`format!("{f}")` (Rust's shortest-round-trip f64→string) BEFORE parsing. This is arithmetically SAFE
(empirically, `format!("{f}")` reproduces all 5 real values AND the clean-8dp / sub-half cases exactly —
Gemini's 10-dp values at realistic magnitudes are 11–14 significant digits, well within f64's ~15-sig-dig
clean range). Two REQUIRED consequences:
1. **The integration KAT MUST exercise the numeric path** — write the sub-sat `BTC Amount BTC` cells with
   `write_number` (→ `Data::Float`), NOT `write_string`. Cover BOTH numeric AND string cell types (the
   spec does not pin Gemini's exact cell type, and the shipped fixtures use both), asserting the rounded
   sats end-to-end.
2. **Update the `read.rs:169` doc comment** — it currently claims the f64→string conversion is guaranteed
   only for "the intended ≤8-dp exchange decimal"; state that >8-dp (sub-satoshi) quantities now flow
   through and are rounded downstream, with `format!("{f}")` recovering any decimal in f64's clean range.

## Cleanup
- **Remove `AdapterError::FractionalSat`** (`lib.rs:60`) — now unreachable. Internal (in-workspace) error
  variant; referenced only at def + reject + doc + one test; not `#[non_exhaustive]`, not `Serialize`,
  never exhaustively matched. Removal is safe.
- **Update the doc comment** (parse.rs:56): finer-than-satoshi precision is now ROUNDED to the nearest
  satoshi (not an error) — normalizing a QUANTITY to the representable grid. **USD/tax values are still
  never silently rounded** (that guarantee is unaffected — this is BTC quantity only).
- **Repurpose the existing test** (parse.rs:229) that asserts `FractionalSat` on `"0.000000001"` → now
  assert `Ok(0)` (0.1 sat → 0). [R0-N1: `"0.000000001"` and `"0.12345678"` are already asserted at
  parse.rs:215-224 — repurpose/rename those rather than add duplicate inputs.]

## Conservation note
Rounding is per-cell; the ledger tracks the rounded sats and stays internally consistent (FR9 conservation
is computed on the integer sats in `LedgerState`, not re-derived from BTC strings; Gemini's trade BTC leg
is a SINGLE cell, so no two-legs-of-one-trade drift). Total drift on the user's file is **≤ 4 sats**
[R0-N2: worst case 0.5 sat/row × 8 rows, signs may cancel] — inherent to sub-satoshi source data,
negligible (`< $0.001`). No `btctax-core` change.

## SemVer / lockstep
- **btctax-adapters:** behavior change (reject → round) + remove the now-unused `FractionalSat` variant +
  the `read.rs:169` doc update. In-workspace only; a bug fix. No `btctax-core`/cli/tui change. No
  `docs/manual`/GUI mirror.

## KATs (btctax-adapters)
- `subsatoshi_btc_rounds_to_nearest` — the real Gemini values: `"0.0010216163" → 102162`,
  `"0.0997506234" → 9975062`, `"0.7674706206" → 76747062`, `"-0.1156442018" → -11564420`,
  `"0.00076035204" → 76035`.
- `clean_8dp_btc_unchanged` — a whole-satoshi amount (repurpose the existing `"0.12345678" → 12345678`) is
  byte-identical (round is a no-op) — proves the fix doesn't perturb normal CSV imports.
- `sub_half_satoshi_rounds_to_zero` — repurpose the existing `"0.000000001" → 0` (was the FractionalSat test).
- `half_satoshi_tie` [R0-M2] — pin values that PROVE half-even: `"0.000000005"` (0.5 sat) → **0**;
  `"0.000000025"` (2.5 sat) → **2**. (A `1.5→2` case is identical under half-up — non-discriminating; use
  these.)
- **Integration [R0-I1]:** a SYNTHETIC `.xlsx` fixture (the crate builds them via `rust_xlsxwriter`,
  Cargo.toml:21) with the 8 sub-sat rows written as **`Data::Float` numeric cells** (and a string-cell
  variant) — the ingest → read → `parse_btc_to_sat` chain completes WITHOUT error and yields the rounded
  sats. **Do NOT commit the user's real financial file — synthetic only.**

## Plan (TDD)
- **Task 1 — the fix** (parse_btc_to_sat round-to-nearest + remove FractionalSat + parse.rs:56 & read.rs:169
  docs + repurpose the one existing test; the KATs above incl. the numeric-cell integration KAT).
- **Task 2 — whole-diff review (Phase E)** + full workspace suite + FOLLOWUPS (Gemini sub-sat now rounds).

## Gotchas
- **BTC quantity only** — rounds a *quantity* to the satoshi grid; does NOT touch USD/tax rounding; the
  "never silently round money" guarantee for tax VALUES is unchanged. Say so in the doc.
- **Do not truncate** (biases low) and **do not skip** (loses real transactions) — round-to-nearest.
- **The read-layer step is part of the path** [R0-I1]: an xlsx numeric cell goes `Data::Float →
  format!("{f}") → parse_btc_to_sat`; the integration KAT must use `Data::Float` cells, not strings.
- `parse_btc_to_sat` is the SOLE BTC→sat import path (all 4 adapters route through it; verified).
- Use a SYNTHETIC xlsx fixture for the integration KAT (never commit the user's real financial data).
