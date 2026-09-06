# Brief — S9: drop TY2017's form package (owner ruling 2026-09-06: "S9 drop")

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every pinned number you move: old → new with its cause, in the report.

## The ruling and its shape
`design/ROADMAP_STATUS.md` §0a S9 (RULED) and `FOLLOWUPS.md` FR-61. Drop the TY2017 FORM PACKAGE
entirely; KEEP the TY2017 `TaxTable` (`crates/btctax-adapters/src/tax_tables.rs::ty2017` and its
Rev. Proc. 2016-55 KAT) so `report --tax-year 2017` still computes the crypto delta. Nothing else
about 2017 changes (the price dataset, ledger fixtures with 2017 dates, `Pre2025MethodDeclaration`).

## Delete
- `crates/btctax-forms/forms/2017/` — five `.map.toml`, five `.pdf`, `YEAR.toml` (`build.rs` globs
  the directory; nothing else registers it).
- `LineSet::{F1040_2017, F8283_2017, F8949_2017, ScheduleD_2017, ScheduleSe_2017}` and their
  `parse`/`as_str`/`schema` arms; the `Form*Map::ty2017()` constructors and any `ty2017` helpers in
  `crates/btctax-forms/src/*.rs`; `crates/btctax-forms/tests/sp3b.rs` if it is wholly TY2017 (read
  it first; move any test that is NOT about TY2017).
- Every gate row that named TY2017, computed rather than hand-edited where the gate is computed:
  `tests/map_rows.rs` (the `authority = "not-yet-archived"` excuse set 6 → 1 — only `forms/2024/f8283.pdf`
  remains; the row-count floor), `tests/field_census.rs` (`UNCENSUSED` loses its five 2017 entries;
  `UNCENSUSED_ENTRIES` 10 → 5, `UNCENSUSED_FIELDS` 713 → 310 — measured), `tests/year_record.rs`,
  `tests/supported_years_cross_product.rs` (`BUNDLED_FORMS_PER_YEAR`, the `BUNDLED.len()` floor
  41 → 36), `crates/xtask/src/cite_check.rs` (`AUTHORITY_NOT_YET_ARCHIVED` years, `rows()` count),
  `crates/xtask/src/label_reader.rs` (`GRID_MAPS` 2017 entries, the TY2017 `YearFloor`),
  `line_set.rs` (`LineSet::ALL.len()` 41 → 36), `crates/btctax-forms/src/lib.rs` (`SUPPORTED_YEARS` /
  `BUNDLED_BUT_NOT_SUPPORTED` if they name 2017), `tests/kats.rs` / `sp4.rs` / `map_pdf_conformance.rs`
  / any test enumerating bundled years. Let the compiler and the suite find the rest.

## Re-point, do not delete
- `crates/btctax-cli/tests/export_irs_pdf.rs` and `tests/promote_cli.rs`: the TY2017 end-to-end
  KATs (real 2017 events; "this build bundles 2017/2024/2025") — move them to a bundled year (TY2025
  for the Form 8275 aliasing KATs: `f8275--2024` is aliased by hash for 2025; TY2024 for the rest),
  adjusting fixture dates and prices to that year. Keep every assertion's meaning; say in the report
  which year each KAT now runs on and why.
- `crates/btctax-cli/tests/extension.rs`: the "TY2017-shaped record" kills (`return_due` 2018-04-17,
  out-of-country 2018-06-15) keep their DATES as literals by constructing a `YearRecord` in the test
  (or a tempdir `YEAR.toml`), since `forms/2017/YEAR.toml` no longer exists.
- `docs/examples/examples.md`: the "not bundled (this build bundles …)" sentences lose 2017 —
  regenerate (`cargo run -q -p xtask -- examples > docs/examples/examples.md`) and list the diff lines.
- `design/TY2026_WORK_LIST.md` if the printer's table changes (`cargo run -p xtask -- port-status 2025 2026-DRAFT`).

## Kills
- `btctax_forms::bundled::bundled_years() == [2024, 2025, 2026]` and `SUPPORTED_YEARS` == the template
  years without 2017; `export-irs-pdf --tax-year 2017` refuses `UnsupportedYear(2017)` (the message
  names the bundled years); `report --tax-year 2017` with 2017 events STILL computes (a KAT with the
  existing `real_events_2017` fixture, moved next to `report`'s tests).
- No source or test names `forms/2017` (a grep test in `crates/btctax-forms/tests/`, or extend an
  existing "no hand-list" gate), and the year-record partition test still holds for the remaining years.

## Report — your FINAL action
`design/agent-reports/2026-09-06-build-S9-drop-ty2017-implementation.md`: files deleted / edited /
re-pointed, every pinned number old → new with cause, each kill and how it was seen red, the exact
nextest commands with summary lines, anything you could not do. Return ONLY a 3-line summary.
