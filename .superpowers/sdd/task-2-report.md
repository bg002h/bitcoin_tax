# Task 2 report — `oracle_diff` §3.1-printing-on-oracle-figures reproduction

**Status: DONE / GREEN.** Branch `feat/oracle-sweep`. Commit on top of T1 (1b204d3).

## Implemented items

New test-support module `crates/btctax-core/src/tax/oracle_diff.rs`, registered in
`crates/btctax-core/src/tax/mod.rs` as `#[doc(hidden)] pub mod oracle_diff;` (plain `pub mod`,
NOT `#[cfg(test)]` — so the `tests/` integration suite can reach it; mirrors `testonly`). Placed
alphabetically between `method` and `other_taxes`.

Public (test-support) surface — the five §3.1 reproductions + the predicate:

- `usd(x: f64) -> Usd` — `Usd::try_from(x).expect("finite oracle figure")` (mirrors `golden_usd`).
- `round_leaf(oracle_line: f64) -> Usd` = `round_dollar(usd(x))` (Leaf pattern).
- `sum_round(components: &[f64]) -> Usd` = `Σ round_dollar(usd(c))` (Cross-footed pattern —
  the `golden_packet.rs:120-123` L24 pattern generalized; operates on per-line LEGS, never a
  single exact total).
- `rate_on_printed(rate: Usd, printed_operand: Usd) -> Usd` = `round_dollar(rate * printed_operand)`
  (Rate-on-printed pattern — 8959 L7/L13, 8960 L17).
- `table_l16(status, ti, qd_l3a, net_ltcg_qd_excl) -> Usd` = `qdcgt_line16(schedule, bp, ti, qd,
  net_ltcg)` with the per-status `OrdinarySchedule` + `LtcgBreakpoints` pulled from `ty2024_table()`
  (Tax-table pattern; `Table_btctax`).
- `consulted_table(status, ti, qd_l3a, net_ltcg_qd_excl) -> bool` — true iff any worksheet operand
  (`L5 = max(0, ti − (qd+ltcg))` OR the full `ti`) is `< TAX_TABLE_CEILING`, computed from the same
  operands `qdcgt_line16` consumes (`method.rs:83-89`, and `worksheet_tax` reads the Tax Table iff
  its operand `< TAX_TABLE_CEILING`, `method.rs:49`).

Header doc-comment marks the module test-support with the brief's specified wording (§6.2 seam).

## TDD evidence

**RED** — `cargo test -p btctax-core --lib oracle_diff` with the test module written but the
reproductions absent:

```
error[E0425]: cannot find function `sum_round` in this scope
error[E0425]: cannot find function `round_leaf` in this scope
error[E0425]: cannot find function `usd` in this scope
error[E0425]: cannot find function `table_l16` in this scope
error[E0425]: cannot find function `consulted_table` in this scope
```

**GREEN** — after implementing the reproductions:

```
running 3 tests
test tax::oracle_diff::tests::sum_round_cross_foots_the_legs_not_the_exact_total ... ok
test tax::oracle_diff::tests::table_l16_reproduces_ots_above_ceiling ... ok
test tax::oracle_diff::tests::consulted_table_tracks_the_worksheet_operands ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 337 filtered out
```

**Full gate** — `make check`: `Summary [6.907s] 1898 tests run: 1898 passed, 1 skipped`.
`make lint` (clippy `--all-targets --all-features -- -D warnings`): exit 0, clean.

## Baked figures read from `full_return_goldens.json` and CONFIRMED against the brief

- `mfj_se_over_the_addl_medicare_threshold`: `taxable_income = 253942.94`,
  `income_tax_before_credits = 47031.31`. Brief's `table_l16(Mfj, usd(253_942.94), 0, 0) == dec!(47031)`
  = `round_dollar(47031.31)` — AGREES. (No preferential income ⇒ pure TCW; independently verified the
  MFJ marginal formula yields 47031.3056 ≈ 47031.31.)
- `single_qdcgt_both_slices`: `taxable_income = 112400`, `qualified_dividends = 8000`,
  `long_term_capital_gains = 25000` ⇒ remainder `112400 − 33000 = 79400 < 100000` ⇒ `consulted_table
  == true` — AGREES.
- MFJ 253943 (≈ 253942.94 rounded), no preferential ⇒ remainder = TI = 253943 ≥ 100000 ⇒
  `consulted_table == false` — AGREES.

No brief literal disagreed with the JSON. The `sum_round` synthetic-literal test (274.50/499.50 → 775)
uses literals only; JSON was not read for it (per r2-I1).

## Files changed

- `crates/btctax-core/src/tax/oracle_diff.rs` (new, 126 lines incl. tests).
- `crates/btctax-core/src/tax/mod.rs` (+5: module registration).

No frozen files (`tax/{types,compute,se}.rs`) touched. T1's oracle-expectation structs untouched.

## Self-review / concerns

- `FilingStatus` is fully-qualified as `crate::tax::FilingStatus` in the two module signatures
  (rather than a module-level `use`) deliberately: the test module does `use super::*;` AND the
  brief's verbatim `use crate::tax::FilingStatus;`; a module-level bare import would risk a
  redundant-import clippy warning under `-D warnings`. Fully-qualifying keeps the test's explicit
  import the sole binding. Clippy is clean.
- Added `use rust_decimal_macros::dec;` inside the `#[cfg(test)] mod tests` block — necessary because
  the implementation does not use `dec!`, so `super::*` would not carry it. The brief's assertions and
  their literal values are kept verbatim; only this one import was added to make them compile.
- `consulted_table` takes `status` for §6.2 signature symmetry with `table_l16` but does not use it
  (the Tax-Table-vs-TCW choice is a pure magnitude test against the ceiling; the schedule is
  irrelevant). Bound out with `let _ = status;` and a doc note, so it stays warning-clean and the
  intent is explicit rather than a silent unused param.
- `sum_round` uses `Decimal`'s `Sum` impl; empty slice ⇒ `Decimal::ZERO` (correct; unused by tests).
