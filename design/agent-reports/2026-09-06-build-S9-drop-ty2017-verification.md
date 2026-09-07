# S9 — drop the TY2017 form package: independent verification

**Verifier:** independent read-only agent, own worktree, HEAD `fe517611` (main tip; `b3852a3e` is
`HEAD~1`, the S9 commit itself — `fe517611` is a docs-only commit on top of it, so nothing in the
diff under review is stale relative to what I measured).

**Commands used** (all under `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`):

```
git show b3852a3e --stat
git show b3852a3e -- <path>              # per-file diffs, old → new
git show b3852a3e^:<path>                # pre-image for direct counts
cargo nextest run --locked -p btctax-forms -p btctax-cli -p btctax-core -p btctax-adapters --no-fail-fast
cargo nextest run --locked -p xtask -E 'not test(form_delta::) and not test(harness_check::)' --no-fail-fast
cargo fmt --all --check
cargo clippy -p btctax-forms -p btctax-cli -p btctax-core -p btctax-adapters -p xtask --all-targets --all-features -- -D warnings
cargo run -q -p xtask -- docs            # man-page regeneration, diffed against committed
```

**Summary:** 2377/2377 tests pass in the four crates S9 touches directly (btctax-forms, btctax-cli,
btctax-core, btctax-adapters); 107/107 relevant xtask tests pass. `cargo fmt --check` and `clippy -D
warnings` are clean. `cargo run -p xtask -- docs` regenerates every man page byte-identically
(`git diff --stat` empty afterward), including `btctax-export-irs-pdf.1`. Seven xtask test failures
observed in an unscoped run are **environmental, not caused by S9** — see "Excluded from this
verification" below. Every plant described in the implementer's report reproduced the claimed red
text; every plant was reverted and the tree confirmed clean (`git status --short` / `git diff --stat`
empty) after each. One report inaccuracy found (§ New findings): a narrative miscount in §5's own
table, not a code or test defect — **0 Critical / 0 Important / 1 Minor**.

## Excluded from this verification (and why)

A first unscoped run of `-p btctax-forms -p btctax-cli -p xtask -p btctax-core -p btctax-adapters`
showed 7 failures, all in `xtask`:

- `harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` — the
  test hardcodes `target/debug/xtask` (`on-write.sh`'s own lookup path) and fails only because this
  review set a non-default `CARGO_TARGET_DIR`. Confirmed by reading `harness_check.rs:665`, which
  panics with exactly that explanation.
- 6 `form_delta::tests::*` failures, all `no PDF found for f6251--2026-DRAFT` (or the same root
  cause). `design/forms/**/*.pdf` is gitignored (`.gitignore:63`); a fresh `git worktree add` legally
  lacks these locally-fetched IRS draft PDFs that exist in the shared tree. Confirmed via
  `git check-ignore -v`. Unrelated to `forms/2017` or anything S9 touched (these are TY2026-DRAFT
  Form 6251 diffing tests).

Both are worktree/environment artifacts, not regressions from `b3852a3e`. Excluding them, every test
in the packages S9 touches is green.

## 1. Pin table (report §5, re-measured)

All 23 rows MEASURED-MATCH except one (GRID_MAPS), which is a narrative miscount only — no code or
test asserts the wrong number.

| # | pin | file | reported old → new | re-measured | verdict |
|---|---|---|---|---|---|
| 1 | `LineSet::ALL.len()` | `line_set.rs` | 41 → 36 | `assert_eq!(LineSet::ALL.len(), 36)` | MATCH |
| 2 | `BUNDLED.len()` floor | `supported_years_cross_product.rs:323` | 41 → 36 | `>= 36`; actual `BUNDLED.len()` = 36 (19+17 pdfs on disk, `find … -name '*.pdf' \| wc -l` = 36) | MATCH |
| 3 | on-disk map floor | `bundled.rs` | 37 → 36 | `>= 36`, comment confirms "floor was already stale… set to the real count" | MATCH |
| 4 | `rows.len()` floor | `map_rows.rs` | 41 → 36 | `>= 36` | MATCH |
| 5 | `from_rows.len()` | `cite_check.rs` | 41 → 36 | `assert_eq!(from_rows.len(), 36, …)` | MATCH |
| 6 | `bundled_years()` | `bundled.rs` (test) | `[2017,2024,2025,2026]`→`[2024,2025,2026]` | `assert_eq!(bundled_years(), dirs.as_slice())` **and** `assert_eq!(bundled_years(), &[2024,2025,2026], …)`, both against the real `read_dir` | MATCH |
| 7 | `bundled_years()` | `year_record.rs` (test) | same | `assert_eq!(bundled_years(), &[2024, 2025, 2026]);` | MATCH |
| 8 | `SEQUENCE_UNVERIFIABLE` | `map_rows.rs` | 5 entries → 0 | `const SEQUENCE_UNVERIFIABLE: &[(i32, &str)] = &[];` | MATCH |
| 9 | `EXCUSED` | `map_rows.rs` | 6 → 1 | `const EXCUSED: &[(i32, &str)] = &[(2024, "f8283")];` | MATCH |
| 10 | `MapRow::authority` doc | `map.rs:137` | "Six rows today"→"ONE row today" | doc comment reads "ONE row today — `forms/2024/f8283.pdf` — down from six…" | MATCH |
| 11 | `UNCENSUSED_ENTRIES` | `field_census.rs` | 10 → 5 | `const UNCENSUSED_ENTRIES: usize = 5;`, register has 5 tuples | MATCH |
| 12 | `UNCENSUSED_FIELDS` | `field_census.rs` | 713 → 310 | `const UNCENSUSED_FIELDS: usize = 310;`; hand-summed register (196+63+12+24+15) = 310 | MATCH |
| 13 | `BUNDLED_FORMS_PER_YEAR` | `supported_years_cross_product.rs` | `[(2017,5),(2024,19),(2025,17)]`→`[(2024,19),(2025,17)]` | `const … = &[(2024, 19), (2025, 17)];`; disk count 19+17=36 confirms | MATCH |
| 14 | `KNOWN_GAPS` | `supported_years_cross_product.rs` | 13 cells/36 obligations → 8/11 | counted 8 tuples in the const; obligations 4 (f8283 row) + 7×1 = 11 | MATCH |
| 15 | `KNOWN_ALIASES` | `supported_years_cross_product.rs` | `[(2017,"f8275"),(2025,"f8275")]`→`[(2025,"f8275")]` | `const KNOWN_ALIASES: &[(i32, &str)] = &[(2025, "f8275")];` | MATCH |
| 16 | matrix headline (module doc) | `supported_years_cross_product.rs` | historical "21/37 / 54" paired with measured "8/36 / 11"; the report flags its own earlier self-correction from a wrong "16/36 / 29" | current doc reads "8 of the 36 bundled cells carry a gap, 11 obligations unmet — down from 13 cells / 36 obligations"; no stray "16 of 36 / 29" found anywhere in the file | MATCH |
| 17 | emitting surface doc | `cite_check.rs::emitted_form_years` | "20 stems/41 pairs" → "20 stems/36 pairs" | doc reads "…is **20 stems / 36** since S9 dropped the five TY2017 pairs the same day" | MATCH |
| 18 | `AUTHORITY_NOT_YET_ARCHIVED` | `cite_check.rs` | f1040, f1040sd, f1040sse, f8283, f8949 each `[2017,2024,2025]`→`[2024,2025]` | confirmed via `git show b3852a3e` diff AND current file: exactly these 5 entries lost `2017`, no others touched | MATCH |
| 19 | `GRID_MAPS` | `label_reader.rs` | **"7 entries → 5"** | actual count: **8 entries → 6** (pre-image has 8 tuples: `(2017,f8949)`, `(2024,f8949)`, `(2025,f8949)`, `(2024,f8275)`, `(2017,f8283)`, `(2025,f8283)`, `(2024,f1040v)`, `(2025,f1040v)`; post-image drops exactly `(2017,f8949)` and `(2017,f8283)`, leaving 6) | **MISMATCH — see New findings N1** |
| 20 | `YEAR_FLOORS` | `label_reader.rs` | TY2017 floor removed (`min_joins: 0, max_unwitnessed: 5`) — qualitative, no count pinned | diff confirms exactly the one `YearFloor { year: "2017", … }` struct literal removed, replaced with an explanatory comment | MATCH |
| 21 | geometry-fixture hash join | `label_reader.rs` doc | "the 6 that match none are TY2017's five and `2024/f8283.pdf`" → "`2024/f8283.pdf` alone" | diff confirms exact wording change | MATCH |
| 22 | `field_census.rs` scan-scope comment | `field_census.rs` | "2017/2024/2025 = 37 maps" → "2024/2025 = 36 maps"; `GAPS` still 0 | both strings present verbatim; `const GAPS: usize = 0;` unchanged | MATCH |
| 23 | Schedule D line-17 on-state loops | `kats.rs` ×3 (+ the lookup closure) | `[2017, 2024, 2025]` → `[2024, 2025]` | `for year in [2024, 2025]` × 3 sites; the lookup closure changed from a presumed wildcard to `(2024 \| 2025, …) => …, (other, _) => panic!(…)` | MATCH |

**23 of 23 counted; 22 MEASURED-MATCH, 1 MISMATCH (narrative-only, see N1).**

## 2. Kept — `ty2017()` and its KAT

`crates/btctax-adapters/src/tax_tables.rs::ty2017()` and `ty2017_table_matches_rev_proc_2016_55`
are untouched; `BundledTaxTables::load()` still calls `by_year.insert(2017, ty2017())` at line 76.

**Plant:** commented out that line, ran `-p btctax-cli -p btctax-adapters`. Five tests reported red,
matching the report's claim ("the year survives, the forms do not"):

```
tax_tables::tests::ty2017_table_matches_rev_proc_2016_55
tax_tables::tests::all_bundled_years_are_tax_table_binnable
tax_tables::tests::ty2017_ancillary_fields
tax_tables::tests::ty2017_table_is_available
btctax-cli::tax_report report_tax_year_2017_still_computes_after_the_form_package_was_dropped
  panicked at crates/btctax-cli/tests/tax_report.rs:228:
  TY2017 must still compute — its TaxTable was KEPT by the S9 ruling:
  Federal tax attributable to crypto — tax year 2017
    NOT COMPUTABLE [TaxTableMissing]: no bundled tax table for 2017
```

Reverted (`cp` from a scratch backup); `git status --short` and `git diff --stat` both empty
afterward. Also independently confirmed live (not planted): `report --tax-year 2017`'s KAT passes at
HEAD and the raw Schedule D projection assertion in it ($500/$200/$300 short-term) is present in
`tax_report.rs`.

## 3. Gone — TY2017 form package

- `crates/btctax-forms/forms/` on disk today: `2024/`, `2025/`, `2026/` only. No `2017/`.
- `grep -rn "_2017" crates/btctax-forms/src/line_set.rs` — no output. No `LineSet::*_2017` variants.
- `grep -rn "ty2017" crates/btctax-forms/src/map.rs` — no output. No `Form*Map::ty2017()`.
- `bundled_years()` == `[2024, 2025, 2026]` (both `bundled.rs` and `year_record.rs` tests, § pin 6/7).

**Plant 1 — `export-irs-pdf --tax-year 2017`:** test
`ty2017_is_refused_by_the_export_and_the_refusal_names_the_bundled_years` passes at HEAD (not a
plant; a direct-behavior check). Read the body: asserts `CliError::FormFill(FormsError::UnsupportedYear(2017))`,
the message names every `btctax_forms::SUPPORTED_YEARS` entry, does not contain `"2017 "`, and
`!out.exists()` (no bytes written). All confirmed present, matching report §6.

**Plant 2 — "no code names `forms/2017`":** wrote a scratch file
`crates/btctax-forms/tests/scratch_plant_forms2017_ref.rs` containing
`let _p = "forms/2017/f1040.map.toml";` and ran
`no_code_in_the_workspace_names_the_deleted_forms_2017_path`. Red, exact match to the report's claim:

```
panicked at crates/btctax-forms/tests/s9_ty2017_package_is_gone.rs:183:5:
these are CODE naming the deleted TY2017 form package (a comment recording the pre-drop measurement is fine; a path a program can follow is not):
  …/scratch_plant_forms2017_ref.rs:4: let _p = "forms/2017/f1040.map.toml";
```

Deleted the scratch file immediately after; `git status --short` empty. All 3 tests in
`s9_ty2017_package_is_gone.rs` pass at HEAD (`no_ty2017_form_package_is_on_disk_or_bound`,
`no_ty2017_line_set_revision_parses`, `no_code_in_the_workspace_names_the_deleted_forms_2017_path`).

## 4. Re-pointed KATs (report §4) — old vs. new bodies compared

Diffed every file the report names against `git show b3852a3e`, not just the current state, so a
transplant that quietly dropped an assertion would show up as a `-` line with no matching `+`.

- **`export_irs_pdf.rs::ty2017_real_ledger_fills_box_c_f_and_line13_no_da`** — deleted. Its 6
  assertions checked against the deleted test's diff and the current
  `ty2024_real_ledger_fills_box_c_f_and_line7_and_da`: `!report.watermarked`, 8949 starts `%PDF` + no
  XFA, Box C `/3`, 1040 capital-gain line = $300 all pre-existed in the TY2024 twin; the two that did
  not (`!pdf_has_xfa` on the 1040, `report.form_1040_filled_7a`) are now present, added with an
  explicit "Carried over from the deleted TY2017 twin" comment. All 6 land. MATCH.
- **`the_two_pipelines_cannot_overwrite_each_others_files`** — slice year 2017 → 2025; a new
  `real_events_2025_for_the_slice_arm()` fixture (distinct source refs) is appended to the ledger, and
  a new assertion checks `f8949.pdf`/`schedule_d.pdf` were actually written by the slice. MATCH.
- **`promote_cli.rs`**: `a_promoted_2025_export_fills_the_8275_and_the_gate_passes` loop `[2025,
  2017]` → single `year = 2025` (unrolled, with a comment explaining the clippy
  `single_element_loop` reason); `all_years_snapshot_writes_one_8275_txt_per_promoted_year`'s
  `(2024, 2017)` → `(2024, 2025)` and `form_8275_2017.txt` → `form_8275_2025.txt`. MATCH.
- **`extension.rs::a_ty2017_shaped_record_never_prints_04_15_and_its_june_date_is_the_15th`** — the
  record is built by substituting `year`/`return_due` into the committed TY2024 record text and
  parsing it back through the real `YearRecord::parse`; both literal dates (`2018-04-17`,
  `2018-06-15`) are unchanged. MATCH.

  **Plant:** changed `extension_due_date`'s out-of-country branch to
  `return_due + time::Duration::days(61)` (naive "+2 months", which for 2018-04-17 also lands on
  2018-06-17). Red:
  ```
  panicked at crates/btctax-cli/tests/extension.rs:438:5:
  assertion `left == right` failed: the form names June 15 itself; 2018-06-15 is a Friday, so §7503 leaves it alone
    left: 2018-06-17
   right: 2018-06-15
  ```
  Reverted; clean.

- **`year_readiness.rs`**: the slice-year declaration is now constructed from TY2025's committed
  record with `status` rewritten to `"slice"`, with a premise assertion the rewrite actually took.
  MATCH.

  **Plant:** `if d.status == YearStatus::Slice && !self.table` → `if false && …`. Red:
  ```
  panicked at crates/btctax-cli/src/year_readiness.rs:490:9:
  assertion failed: r.problems().iter().any(|m| m.contains("crypto-slice year but no TaxTable"))
  ```
  Reverted; clean.

- **`map_rows.rs::a_row_with_no_extract_is_reported_as_unverifiable`** — new `plant_as(src_year,
  dst_year, stem, edit)` helper copies a real map into a tempdir under a different year; the kill
  plants `2025/f8949` as TY1999, guarded by an assertion the planted year truly has no archived
  extract. MATCH.

  **Plant A** (year with an extract, TY2024): guard fired —
  `panicked at crates/btctax-forms/tests/map_rows.rs:574:5: the planted year must have NO archived extract, or this kill proves nothing`.
  **Plant B** (same, guard also removed): `panicked at crates/btctax-forms/tests/map_rows.rs:579:5: []`
  (the `problems` vec is empty). Both match report §6 exactly (their line numbers differ slightly —
  583/579 for me vs. their reported 579 for the no-guard case — because their harness used literal
  `1999` where I substituted `2024` throughout for a real archived-extract year; same mechanism, same
  outcome). Reverted; clean.

- **`sp4.rs::an_unknown_key_inside_an_inline_money_pair_is_refused_too`** — constructs a `MoneyPair`
  by rewriting the committed TY2025 Schedule SE map's `line2` from a single field into a
  `{dollars_field, cents_field}` table, with a guard that the source spelling is still present and a
  positive control that the constructed pair parses before the `pennies_field` plant. MATCH.

  **Plant:** made the "doctored" text a no-op (kept `pennies_field` out). Red at the plant-application
  guard (`assertion left != right failed: the money-pair plant did not apply`) — a different line than
  the report's cited 1047 (its plant left `doctored` different-but-still-valid, landing on the
  `expect_err` line instead), but the same substance: the test cannot report success without a real
  defect present. Reverted; clean.

All re-pointed KATs preserve their original meaning; none was weakened.

## 5. §7 — the three "no wildcard" panic sites

`form1040.rs::f1040_clusters`, `schedule_se.rs::se_clusters`, `form8283.rs::sec_clusters` all still
`panic!` on an unrecognized year (verified via `grep -n panic!` in all three — no wildcard `_ =>` arm
in any). The TY2017 column-band evidence is kept verbatim in the `f1040_clusters` doc comment,
explicitly marked "now HISTORY, not a live check," and the panic message cites it.

**Plant:** `form1040.rs`'s `match year { 2024 | 2025 => …, other => panic!(…) }` → `match year { 2024
=> …, other => panic!(…) }` (dropping 2025's arm — the "remove one supported year's band" the brief
asked for). Ran all three `geometry_is_recorded_for_exactly_the_supported_years` tests:

```
FAIL form1040::cluster_year_guard::geometry_is_recorded_for_exactly_the_supported_years
  panicked at crates/btctax-forms/src/form1040.rs:261:13:
  assertion `left == right` failed: TY2025: SUPPORTED_YEARS.contains = true but f1040_clusters panicked — a supported year with no band recorded: measure the amount column off the year's blank PDF (xtask dump-fields) and add the arm
    left: false
   right: true
PASS form8283::geometry_year_tests::form8283_geometry_is_recorded_for_exactly_the_supported_years
PASS schedule_se::cluster_year_guard::geometry_is_recorded_for_exactly_the_supported_years
```

Exactly the discrimination claimed: the mutated file's guard reds, the untouched two stay green.
Reverted; clean.

## 6. §8 — product docs

```
grep -n '2017' README.md
grep -rn '2017' docs/man/*.1
grep -n '2017' crates/btctax-cli/src/cli.rs
```

- **README.md**: line 190-194, a blockquote stating TY2017's package was dropped, why, that
  `export-irs-pdf --tax-year 2017` refuses, and that `report --tax-year 2017` still computes — all
  true. Line 467: "`2017–2023`" is explicitly listed as OUTSIDE the bundled-templates set (correct;
  was `2018–2023` before the drop, correctly widened by one year).
- **docs/man/btctax-export-irs-pdf.1**: "this build bundles TY2024 and TY2025; other years are
  refused — TY2017's package was dropped in 2026, though `report --tax-year 2017` still computes the
  crypto delta" — true, and regenerated byte-for-byte by `cargo run -p xtask -- docs` (confirmed:
  `git diff --stat` empty after a full regen of all 20 man pages).
- **crates/btctax-cli/src/cli.rs:207**: identical wording to the man page (single source, per repo
  convention). True.

No remaining mention of 2017 anywhere in these three surfaces claims TY2017 is a supported *export*
year; every mention is either historical/honest-refusal framing or the correctly-kept `report`
computation. MATCH to report §8.

Also spot-checked `crates/btctax-core/src/{donation,forms}.rs` diffs (named in report §8 as
"stale doc comments corrected" beyond the brief's list): both edits are accurate restatements, no
factual regression (`donation.rs`'s Rev. 12-2014 cross-reference now reads as history rather than a
live TY2017 claim; `forms.rs`'s `InformationReturnRegime::NONE` doc now says "TY2024 and every
earlier year" instead of naming TY2017 specifically — still true).

## 7. §9 — kept, exactly as the ruling says

All spot-checked and present: `MoneyCell::Pair` / `MoneyPair` / `fmt_money_pair` (live in `cells.rs`,
confirmed unused by any bundled map — `grep -rl dollars_field crates/btctax-forms/forms/` returns
nothing); `Form8283Map::{deduction, btc_property_note}` (`Option`s, present in `map.rs`);
`YearStatus::Slice` (present in `year_record.rs`, doc comment updated to say no bundled year carries
it today); `EraPreset::Y2015To2017` (present in `btctax-core/src/defensive/era.rs`); a
`Pre2025Method…` declaration type (present, referenced across 10 files in `btctax-core`/`btctax-cli`).

## New findings

### N1 (Minor) — GRID_MAPS pin miscounted in the implementer's report

**Claim:** report §5 states `GRID_MAPS | label_reader.rs | 7 entries → 5 (2017/f8949, 2017/f8283
removed)`.

**Where:** `design/agent-reports/2026-09-06-build-S9-drop-ty2017-implementation.md` §5, the
`GRID_MAPS` row.

**What is wrong:** the actual counts, both pre- and post-image, are one higher than stated. Counting
tuples in `git show b3852a3e^:crates/xtask/src/label_reader.rs`'s `GRID_MAPS` block: 8 entries
(`(2017,f8949)`, `(2024,f8949)`, `(2025,f8949)`, `(2024,f8275)`, `(2017,f8283)`, `(2025,f8283)`,
`(2024,f1040v)`, `(2025,f1040v)`). The post-image (current HEAD) has 6:
`(2024,f8949)`, `(2025,f8949)`, `(2024,f8275)`, `(2025,f8283)`, `(2024,f1040v)`, `(2025,f1040v)`.
The delta (−2) and the specific entries removed (`(2017,f8949)`, `(2017,f8283)`) are exactly as
claimed — only the two absolute counts in the table are off by one each.

**Evidence:**
```
awk '/const GRID_MAPS/,/^    \];$/' <pre-image> | grep -c '^        ($'   → 8
awk '/const GRID_MAPS/,/^    \];$/' <current file> | grep -c '^        ($' → 6
```
Confirmed via the actual diff (`git show b3852a3e -- crates/xtask/src/label_reader.rs`), which shows
exactly 2 tuples removed from a hunk whose surrounding context includes the untouched `f1040v` pair
not shown in the report's arithmetic.

**Impact:** none on shipped behavior or test correctness — `GRID_MAPS` is consumed only via
`.iter().any(…)` in `label_reader.rs:1239`; no test pins `GRID_MAPS.len()` to a literal, so nothing
in the suite depends on the wrong count. This is a documentation/report-accuracy defect only, in an
otherwise very precisely self-checked implementation report.

**Minimal change:** correct the report's §5 row to "8 entries → 6" (or, if the report is treated as a
frozen historical artifact per this repo's persist-verbatim convention, leave it and record this
verification's correction alongside it rather than editing the original).

---

**Counts: C=0 I=0 M=1 N=0**
