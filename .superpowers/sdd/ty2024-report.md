# Task 1 Report — TY2024 tax-table backfill

**Branch:** `feat/ty2024-tables`  
**Base:** `81bcd4f`  
**Spec:** `design/SPEC_ty2024_tables.md`

---

## Files changed

### 1. `crates/btctax-adapters/src/tax_tables.rs` (primary)

**Builder added:** `ty2024() -> TaxTable` inserted immediately before `ty2025()`.

Ordinary brackets transcribed verbatim from Rev. Proc. 2023-34 §3.01:
- **Single** (Table 3): 10%/$0, 12%/$11,600, 22%/$47,150, 24%/$100,525, 32%/$191,950, 35%/$243,725, 37%/$609,350
- **MFJ** (Table 1): 10%/$0, 12%/$23,200, 22%/$94,300, 24%/$201,050, 32%/$383,900, 35%/$487,450, 37%/$731,200
- **HoH** (Table 2): 10%/$0, 12%/$16,550, 22%/$63,100, 24%/$100,500, 32%/$191,950, **35%/$243,700** (not $243,725), 37%/$609,350
- **MFS** (Table 4): 10%/$0, 12%/$11,600, 22%/$47,150, 24%/$100,525, 32%/$191,950, 35%/$243,725, **37%/$365,600**
- QSS: not inserted (maps to MFJ via `TaxTable::key`)

LTCG breakpoints from §3.03:
- Single: max_zero=$47,025, max_fifteen=$518,900
- MFJ: max_zero=$94,050, max_fifteen=$583,750
- HoH: max_zero=$63,000, max_fifteen=$551,350
- MFS: max_zero=$47,025, **max_fifteen=$291,850** (not $291,875 — independent rounding per Rev. Proc.)

Ancillary fields:
- `gift_annual_exclusion`: $18,000 (§3.43)
- `ss_wage_base`: $168,600 (SSA 2023-10-12)
- `gift_lifetime_exclusion`: $13,610,000 (§3.41)

`source` field: `"Rev. Proc. 2023-34 §3.01/§3.03 + §3.43 + §3.41 (TY2024); SSA 2023-10-12 (ss_wage_base $168,600)"`

**Registration:** `by_year.insert(2024, ty2024());` added before `by_year.insert(2025, ty2025());` in `BundledTaxTables::load()`.

**Docstring (module, §"# Source citation"):** Updated to include separate TY2024 citation block (Rev. Proc. 2023-34 §3.01/§3.03/§3.43/§3.41 + SSA 2023-10-12); OBBBA note kept TY2025-scoped ("OBBBA is a 2025 enactment and does not affect TY2024 values").

**Five "TY2025 only" comment sites updated:**
1. Module docstring line 1 — "TY2024 and TY2025 indexed numbers from Rev. Proc. 2023-34 and Rev. Proc. 2024-40 respectively"
2. `BundledTaxTables` struct doc — "Currently contains **TY2024** (Rev. Proc. 2023-34) and **TY2025** (Rev. Proc. 2024-40)"
3. `load()` docstring — "TY2024 and TY2025 bundled"
4. `crates/btctax-cli/src/cmd/optimize.rs:162` — "tables (TY2024 and TY2025)"
5. `crates/btctax-cli/tests/optimize_accept.rs:83` — "bundled tables cover TY2024 and TY2025"

**Statutory constants:** untouched (`NIIT_RATE`, `niit_threshold`, `loss_limit`, SE constants in `tables.rs`).

---

## KAT results (all in `tax_tables::tests`)

| KAT | Name | Expected | Actual | Pass |
|-----|------|----------|--------|------|
| A1 | `ty2024_single_ordinary_brackets_match_rev_proc_2023_34` | brackets[1].lower=11600, brackets[6].lower=609350 | matches | PASS |
| A2 | `ty2024_mfs_37_pct_starts_at_365600_and_mfj_at_731200` | MFS last.lower=365600; MFJ last.lower=731200 | matches | PASS |
| A3 | `ty2024_ltcg_breakpoints_all_statuses` | all 5 statuses per spec | matches | PASS |
| A4 | `ty2024_ancillary_fields` | gift_annual=18000, ss_wage=168600, lifetime=13610000 | matches | PASS |
| A5 | `ty2024_table_is_available` | `table_for(2024).is_some()` | true | PASS |
| A6a | `ty2024_a6a_single_22pct_bracket_entry` | total=$220.00, niit=$0 | $220.00 / $0 | PASS |
| A6b | `ty2024_a6b_mfj_22_24_boundary` | total=$459.00, niit=$0 | $459.00 / $0 | PASS |
| A6c | `ty2024_a6c_hoh_12_22_boundary` | total=$100.00, niit=$0 | $100.00 / $0 | PASS |
| A6d | `ty2024_a6d_mfs_35_37_boundary_with_niit` | total=$396.00, niit=$38.00 | $396.00 / $38.00 | PASS |
| A7  | `ty2024_a7_single_ltcg_0_to_15_threshold` | ltcg=$446.25, total=$446.25, niit=$0 | $446.25 / $0 | PASS |

---

## Regressions

| Test | Pass |
|------|------|
| `tax_tables::tests::ty2025_single_ordinary_brackets_match_rev_proc_2024_40` | PASS |
| `tax_tables::tests::ty2025_ltcg_breakpoints_all_statuses` | PASS |
| `tax_tables::tests::mfs_37_pct_starts_at_375800_and_mfj_at_751600` | PASS |
| `tax_tables::tests::missing_year_returns_none` (year 2099) | PASS |
| `tax_tables::tests::ty2025_gift_annual_exclusion_is_19000` | PASS |
| `tax_tables::tests::statutory_values_are_not_in_the_table_and_constant_across_years` | PASS |
| `tabs::tests::tax_tab_year_change_updates_figures` (TUI blocker flip confirmed benign) | PASS |
| `optimize_run_pre2025_is_usage_error` | PASS |

---

## `cargo test --workspace` authoritative count

**692 passed; 0 failed** (52 test suites across all workspace crates)

`cargo clippy --workspace --all-targets -- -D warnings`: clean  
`cargo fmt --all -- --check`: clean
