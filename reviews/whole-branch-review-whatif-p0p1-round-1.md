# Whole-diff review (Phase E) — feat/whatif STAGE P0+P1 — round 1

**Verdict: 0 Critical / 0 Important — SHIP (P0+P1).**

Diff `main (283238f)..c88ea08` — 3 task commits (P0 `8324245` engine delta, P1a `8960518` core+consult fix,
P1b `c88ea08` CLI+docs). Contract: `design/SPEC_synthesize_whatif.md` (R0-GREEN, 2 rounds). **P2 (harvest) +
P3 (TUI) deferred.**

## ★ The non-negotiable invariant — CONFIRMED two ways
- **`whatif_never_persists`** + `whatif_adhoc_profile_magi_defaults_and_never_persists` KATs pass.
- **My independent binary check:** a REAL executed `what-if sell 5,000,000 sat @ $100k, single/$60k, 2026`
  left the vault **SHA-256 byte-identical** before/after. Clone-fold-discard holds end-to-end.

## Verified by KAT + a real run
- **[P0 regression] every existing tax number byte-identical** — full workspace 1348/0, zero KAT value
  changes; `taxresult_pref_split_matches_internal` proves the surfaced split == `preferential_tax(bottom_with…)`.
  `TaxResult`/`PrefSplit` now `#[non_exhaustive]` + PrefSplit crate-root re-export.
- **[marginal] exact** — `whatif_marginal_equals_withhyp_minus_baseline` + `…_cancels_no_crypto_term`. My run:
  LT gain $4,514.75 → 15% → marginal $677.21 (= gain × 0.15, exact).
- **[★ R0-I1 §1212] delta-based** — `carryforward_delta` = carryforward_out delta; the this-year offset =
  `loss_deduction` delta (NOT hard-coded $3,000). `whatif_sell_loss_reports_carryforward_delta` +
  `whatif_sell_offset_delta_is_zero_when_baseline_caps` pass.
- **[★ R0-I2 NIIT] incremental** — `niit_incremental` = `niit` delta; `whatif_niit_incremental_not_raw_flag`
  (loss harvest on a real-gain year → niit_incremental < 0, raw flag NOT used) passes.
- **[consult fix] `consult_marginal_subtracts_baseline`** — on a year with real disposals, marginal ≠ the
  whole-year figure; the render headlines marginal, relabels the whole-year figure; consult goldens still pass.
- **[bracket/NIIT/rate]** `sell_reports_correct_ltcg_bracket`, `sell_niit_crossing`, `sell_effective_rate` pass;
  my run showed the 15% bracket + room + effective 0.1500 correctly.
- **[ad-hoc profile] [R0-M4]** `--magi` defaults to `--income` (never $0) + the printed caveat — confirmed live.
- **[refusals]** inherited taxonomy (missing table/profile, pre-2025, future-no-price, NoLots, Hard blocker).

## Scope / suite
btctax-core (+`whatif` module, +`pref_split`/`bottom_with` on TaxResult, +the pub(crate) seam lifts) + btctax-cli
(`what-if sell` + ad-hoc profile + render + the consult fix). 23 new KATs. Full workspace 1348/0 (my close-out
re-running); clippy -D + fmt clean; isolation OK. Breaking (new pub TaxResult fields) → next release **0.4.0**.

## FOLLOWUP (non-blocking)
`--sell` takes an integer SAT amount; users think in BTC (0.05 BTC = 5,000,000 sat). Consider accepting a BTC
decimal (parse → sat) for the CLI ergonomics. Filed.

**SHIP P0+P1 — the what-if `sell` marginal + §1212 carryforward + incremental NIIT are correct, the consult bug
is fixed, and the vault is provably never written. P2 (segment-walk harvest optimizer) + P3 (TUI) remain.**
