# SE Chunk B Task 1 — Implementation Report

**Branch:** `feat/se-chunkb-expenses`
**Commit:** `5af71ee`
**Status:** GREEN

## Summary

Task 1 of the SE Chunk B (Schedule C expenses, advisory-only) is complete.
All spec requirements from `design/SPEC_se_chunkB_expenses.md` are implemented.

## Deliverables

### D1 — TaxProfile field + CLI flag
- `#[serde(default)] pub schedule_c_expenses: Usd` added to `TaxProfile` in
  `btctax-core/src/tax/types.rs` after `w2_medicare_wages`
- `--schedule-c-expenses` CLI flag added to `cmd TaxProfile` in `main.rs`
  (optional, default $0, negative → `CliError::Usage`)
- `--show` updated to display `schedule_c_expenses`
- KAT: `optional_profile_fields_default_to_zero` asserts `schedule_c_expenses == Usd::ZERO`
- KAT: `tax_profile_serde_round_trips` updated with new field

### D2 — `compute_se_tax` 7th parameter
- Signature: `compute_se_tax(..., schedule_c_expenses: Usd) -> Option<SeTaxResult>`
- `gross_se = se_net_income(state, year)`
- `net_se = max(0, gross_se − schedule_c_expenses)`; `net_se == 0 → None`
- `SeTaxResult.net_se` doc updated to reflect expensed net
- All three call sites updated: `cmd/tax.rs`, `cmd/admin.rs`, `tui/tabs/tax.rs`

### D3 — `render_schedule_se` three-way None split [R0-I1]
- New signature: `(year, result, gross_se, table_present, schedule_c_expenses, w2_ss, w2_medicare)`
- `gross_se == 0` → `None` (no section — no business income)
- `!table_present && gross_se > 0` → wage-base-unavailable note (unchanged)
- `table_present && result == None` → "fully expensed" disclosure line
- When `result == Some(r) && expenses > 0`: gross/net breakout + §164(f)-style advisory
- When `result == Some(r) && expenses == 0`: note "no Schedule C expenses supplied"

### N4 — CSV-skip comment updated
Comment at `render.rs` ~720-724 updated to explain both no-income and fully-expensed
cases yield `None` and are omitted from the CSV.

## Golden Tests (all exact Decimal, no float)

| Test | net_se | ss | medicare | addl | total | deductible_half |
|---|---|---|---|---|---|---|
| `chunkb_headline_expenses_20k_no_w2` | 80000 | 9161.12 | 2142.52 | 0 | 11303.64 | 5651.82 |
| `chunkb_fully_expensed_mining_10k_expenses_15k_is_none` | N/A (None) | — | — | — | — | — |
| `chunkb_expenses_w2_combined` | 80000 | 3236.40 | 2142.52 | 214.92 | 5593.84 | 2689.46 |
| `chunkb_regression_zero_expenses_byte_identical_to_golden1` | byte-identical | — | — | — | — | — |
| `chunkb_expenses_equal_to_gross_is_none` | N/A (None) | — | — | — | — | — |

## Test Results

- **678 tests, 0 failures** (full `cargo test --workspace`)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean

## Files Changed (24 files, +639/-78 lines)

Core:
- `btctax-core/src/tax/types.rs` — TaxProfile field
- `btctax-core/src/tax/se.rs` — compute_se_tax 7th param + goldens

CLI:
- `btctax-cli/src/main.rs` — CLI flag + --show update
- `btctax-cli/src/cmd/tax.rs` — call site update
- `btctax-cli/src/cmd/admin.rs` — call site update
- `btctax-cli/src/render.rs` — render_schedule_se three-way split
- `btctax-cli/src/tax_profile.rs` — field presence

TUI:
- `btctax-tui/src/tabs/tax.rs` — call site update

Tests (16 files updated for new param / new assertions):
- `btctax-core/tests/`: kat_tax.rs, method_election.rs, optimize_*.rs (5), tax_compute.rs, reclassify_income.rs
- `btctax-adapters/tests/kat_rate_engine.rs`
- `btctax-cli/tests/`: optimize_accept.rs, optimize_consult.rs, optimize_run.rs, tax_profile.rs, tax_report.rs
- `btctax-tui/src/tabs/tests.rs`

## Concerns

None. All spec constraints satisfied; advisory-only (income-tax stack unchanged);
serde default ensures backward-compatible vault deserialization.

---

# SE Chunk B Task 2 — Spec-Mandated Test Additions

**Branch:** `feat/se-chunkb-expenses`
**Base commit:** `5af71ee`
**Status:** GREEN

## Summary

Added the four tests required by the Task 2 spec (two I-level, one Minor split into
two test functions). TEST-ONLY — no production code changed, no goldens edited.

## New Tests

### I1 — `engine_b_invariance_schedule_c_expenses_zero_vs_20k`
**File:** `crates/btctax-core/tests/reclassify_income.rs`
Proves `compute_tax_year` output (`ordinary_from_crypto`, `niit`, `ltcg_tax`,
`total_federal_tax_attributable`) is bit-identical when `schedule_c_expenses` is $0
vs $20,000. Same business-Mining $100,000 fixture; only the profile field differs.
Locks the spec's "engine B is agnostic to schedule_c_expenses" guarantee.

### I2a — `chunkb_expensed_profile_report_and_csv_parity`
**File:** `crates/btctax-cli/tests/tax_report.rs`
Integration test: vault with business Mining $100,000, profile `schedule_c_expenses=
$20,000`. Drives the REAL `report_tax_year` AND `export_snapshot`. Asserts:
- Report SE section: breakout line (gross/expenses/net), ss $9,161.12, total $11,303.64.
- `schedule_se.csv`: ss_component $9,161.12, total_se_tax $11,303.64 (parity).

### I2b — `chunkb_fully_expensed_integration_no_se_tax_no_csv`
**File:** `crates/btctax-cli/tests/tax_report.rs`
Integration test: vault with business Mining $10,000, profile `schedule_c_expenses=
$15,000`. Asserts:
- Report SE section: "fully expensed" line present; "no §1401 SE tax" present;
  "SS wage base unavailable" absent.
- Export: `schedule_se.csv` NOT written (se_result=None → CSV omitted).

### Minor — `tax_profile_negative_schedule_c_expenses_rejected`
**File:** `crates/btctax-cli/tests/tax_report.rs`
Real-binary test (CARGO_BIN_EXE_btctax pattern): `btctax tax-profile --year 2025
--filing-status single --ordinary-taxable-income 40000 --magi-excluding-crypto 60000
--qualified-dividends 0 --schedule-c-expenses=-5` → exit non-zero. All mandatory
fields supplied so the specific negative-value guard (not "required field") is exercised.

## Validation

- `cargo test --workspace`: 682 passed, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean
