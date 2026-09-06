# S9 — drop the TY2017 form package: implementation report

**Brief:** `design/agent-reports/BRIEF-build-S9-drop-ty2017.md`
**Ruling:** owner, 2026-09-06 — "S9 drop" (`design/ROADMAP_STATUS.md` §0a S9, `FOLLOWUPS.md` FR-61)
**Branch/tree:** `main`, shared tree `/scratch/code/bitcoin_tax`, from HEAD `f9a4679e`
**Not committed, not pushed.**

---

## 1. Gate output

```
cargo fmt --all --check                                          → FMT CLEAN
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace \
  --all-targets --all-features -- -D warnings                    → Finished, 0 warnings

cargo nextest run --locked --workspace --no-fail-fast
  Summary [ 17.643s] 3168 tests run: 3168 passed, 12 skipped

cargo nextest run --locked -p btctax-forms   --no-fail-fast
  Summary [  5.640s]  350 tests run:  350 passed,  4 skipped
cargo nextest run --locked -p btctax-cli     --no-fail-fast
  Summary [  6.758s]  713 tests run:  713 passed,  1 skipped
cargo nextest run --locked -p xtask          --no-fail-fast
  Summary [  6.361s]  130 tests run:  130 passed,  1 skipped
cargo nextest run --locked -p btctax-core    --no-fail-fast
  Summary [  0.551s] 1211 tests run: 1211 passed,  0 skipped
cargo nextest run --locked -p btctax-adapters --no-fail-fast
  Summary [  0.025s]  103 tests run:  103 passed,  0 skipped
```

### Suite count: 3185 → 3168 (−17), fully accounted

| change | Δ |
|---|---|
| `tests/sp3b.rs` deleted (20 `#[test]` fns, all TY2017) | −20 |
| `form1040.rs::the_2017_band_excludes_a_cents_widget_the_unified_band_admits` deleted | −1 |
| `schedule_se.rs::the_2017_bands_exclude_the_cents_widgets_the_unified_bands_admit` deleted | −1 |
| `export_irs_pdf.rs::ty2017_real_ledger_fills_box_c_f_and_line13_no_da` deleted (see §4) | −1 |
| `cells.rs::money_pair_splits_dollars_and_cents` (moved out of sp3b) | +1 |
| `tests/s9_ty2017_package_is_gone.rs` (new, 3 tests) | +3 |
| `export_irs_pdf.rs::ty2017_is_refused_by_the_export_and_the_refusal_names_the_bundled_years` | +1 |
| `tax_report.rs::report_tax_year_2017_still_computes_after_the_form_package_was_dropped` | +1 |
| **net** | **−17** |

---

## 2. Deleted

**The form package** (`git rm -r crates/btctax-forms/forms/2017/`): `YEAR.toml`, and
`f1040`/`f8283`/`f8949`/`schedule_d`/`schedule_se` × `.pdf` + `.map.toml` — 11 files.
`build.rs` globs the directory, so `BUNDLED`, `BUNDLED_YEARS`, `TEMPLATE_YEARS`, `template()`,
`map_text()` and `year_record_text()` all lost TY2017 with no code edit.

**Code:**

| what | where |
|---|---|
| `LineSet::{F1040_2017, F8283_2017, F8949_2017, ScheduleD_2017, ScheduleSe_2017}` + their `parse` / `as_str` / `ALL` / `schema` arms (5 each, 25 lines) | `btctax-forms/src/line_set.rs` |
| 5 × `Form*Map::ty2017()` constructors | `btctax-forms/src/map.rs` |
| `F1040_CLUSTERS_2017` + its `f1040_clusters` arm | `btctax-forms/src/form1040.rs` |
| `SE_CLUSTERS_2017` + its `se_clusters` arm | `btctax-forms/src/schedule_se.rs` |
| `SEC_A_CLUSTERS_2017`, `SEC_B_CLUSTERS_2017` + their two `sec_clusters` arms | `btctax-forms/src/form8283.rs` |
| `tests/sp3b.rs` (807 lines) — **wholly TY2017**, see §3 | `btctax-forms/tests/` |

---

## 3. `sp3b.rs` was wholly TY2017 — with two exceptions, both preserved

Read before deletion. 20 tests; 18 are TY2017-only by construction (`Form8949Map::ty2017()` and
friends, 2017 field names, the Rev. 12-2014 8283, the §B long SE). The two that were not:

1. **`ty2017_and_2024_bitcoin_use_box_c_f`** looped `[2017, 2024]`. The 2024 arm is already held,
   assertion for assertion, by **`sp3.rs::ty2024_bitcoin_uses_box_c_f`** (same field
   `c1_1[2]`, same on-state `/3`). Nothing lost.
2. **`money_pair_splits_dollars_and_cents`** — its first half asserts the pure formatter
   `fmt_money_pair`, which is **not about any year** and was tested **nowhere else** (grep:
   `fmt_money_pair` appeared only in `cells.rs` and `sp3b.rs`). Those five assertions are **moved
   verbatim** into a new `#[cfg(test)] mod tests` in `crates/btctax-forms/src/cells.rs`, beside the
   function. Its second half (a real 2017 SE fill) died with the package.

**The three `fill_schedule_d` / `fill_schedule_d_totals` call sites the R6 build transcribed
(sp3b.rs:338, 530, 710 plus the `fill_schedule_d_totals` fault-injection at 613) went with the
file — all four were TY2017-only.** They are *not* the only coverage of those functions:
`kats.rs` calls them at lines 85, 289, 417, 479, 496, 508, 602, 661, 719, 756, 770, 784 and
`sp3.rs` at 209 and 504, on 2024 and 2025. Both entry points remain exercised on every bundled
revision, including the fault-injection arm (`kats.rs:479`, `kats.rs:756`).

**`MoneyCell::Pair` / `MoneyPair` is now used by NO bundled map** (`grep -l dollars_field
crates/btctax-forms/forms/` → only the three deleted 2017 files). The type, the `push_money`
branch and `fmt_money_pair` are **kept** — deleting them would be a schema change the brief did not
ask for, and the moved unit test is what says what the branch must print if a revision needs it
again. Recorded as an observation, not a change.

---

## 4. Re-pointed, and where each KAT now runs

### `btctax-cli/tests/export_irs_pdf.rs`

**`ty2017_real_ledger_fills_box_c_f_and_line13_no_da` — DELETED, its assertions folded into the
surviving TY2024 twin.** This is the one place I did not follow the brief's "re-point to TY2024"
literally, and here is the measurement behind that:

| assertion | already held by |
|---|---|
| `!report.watermarked` | `ty2024_real_ledger_fills_box_c_f_and_line7_and_da` |
| `f8949` starts `%PDF`; no XFA on the 8949 | same |
| Box C `c1_1[2] == "3"` | same |
| the 1040 capital-gain line == the $300 gain | same (line 7 vs 2017's line 13 — the same fact, per revision) |
| **no XFA on the 1040** | **nowhere — transplanted into the TY2024 twin** |
| **`report.form_1040_filled_7a`** | **nowhere in that test — transplanted into the TY2024 twin** |

Re-pointed onto TY2024 the test becomes a byte-for-byte duplicate of a test that already exists;
re-pointed onto TY2025 the Box C assertion is simply false (2025 files BTC under Box I/L —
`real_ledger_fills_clean_official_pdfs` is the 2025 twin). So the two assertions that were unique
were moved into `ty2024_real_ledger_fills_box_c_f_and_line7_and_da` and the shell was deleted.
Every assertion's meaning survives; no duplicate test was landed.

**`the_two_pipelines_cannot_overwrite_each_others_files` → slice year 2017 → 2025.** The test runs
the full-return packet for TY2024 and then the crypto slice for *another* year into the same
directory. Re-pointing needed the slice year to have crypto of its own, or the collision it exists
to detect never happens — so a new fixture `real_events_2025_for_the_slice_arm()` (distinct source
refs: `slice-buy-2025` / `slice-sell-2025`; 0.02 BTC @ $400 → $900) is appended to
`real_events_2024()`, and an explicit assertion now checks the slice really wrote `f8949.pdf` and
`schedule_d.pdf`. Seen red (§6).

**Stale comment fixed:** `unsupported_year_is_refused` said *"This build bundles TY2017 + TY2024 +
TY2025"* and *"only 2017/2024/2025 are bundled"*.

### `btctax-cli/tests/promote_cli.rs`

- **`a_promoted_2025_export_fills_the_8275_and_the_gate_passes`**: loop `[2025, 2017]` → **TY2025**
  only (unrolled to `let year = 2025;` — a one-element loop is `clippy::single_element_loop`).
  TY2025 is the only non-2024 fillable year left, and TY2025 has no `f8275` of its own, so it still
  exercises the periodic alias end to end. The year-general half is
  `sp4.rs::for_year_answers_exactly_what_the_bundled_asset_registry_answers`, which has no year list
  at all.
- **`all_years_snapshot_writes_one_8275_txt_per_promoted_year`**: `(2024, 2017)` → **`(2024, 2025)`**,
  and the asserted file `form_8275_2017.txt` → `form_8275_2025.txt`. This test *passed* unchanged
  (it writes `.txt`, not PDFs, so no form package is involved) — re-pointed anyway because naming
  2017 there reads as a supported-year claim.
- `T14_YEAR`'s doc comment ("this build bundles 2017/2024/2025") corrected.

### `btctax-cli/tests/extension.rs`

**`a_ty2017_shaped_record_never_prints_04_15_and_its_june_date_is_the_15th`** read
`YearRecord::for_year(2017).return_due`. Per the brief the record is now **constructed**: the
committed TY2024 record text with `year 2024 → 2017` and `return_due 2025-04-15 → 2018-04-17`
substituted, parsed back through the real `YearRecord`. **Both dates stay literals and both
assertions are unchanged** (`2018-04-17` ≠ `04-15`; `2018-06-15` ≠ `06-17`). Seen red (§6).

### `btctax-cli/src/year_readiness.rs`

**`a_filable_declaration_without_params_is_reported`** had a *slice*-year arm on TY2017, which was
the **only** bundled year declaring `YearStatus::Slice`. The declaration is now constructed from
TY2025's committed record with `status = "preparing"` → `"slice"`, plus a premise assertion that
the plant actually changed the status. The `YearStatus::Slice` branch in `problems()` stays live
code — it is the honest declaration for a future partial year. Seen red (§6).
**`the_regime_is_joined_from_the_year_record`** lost its `r(2017)` row (no record to read).

### `btctax-cli/tests/tax_report.rs`

`real_events_2017()` moved here **verbatim** from `export_irs_pdf.rs`, plus a
`make_vault_from_events` helper, for the new report KAT (§5, kill 4).

### `btctax-forms/tests/map_rows.rs`

**`a_row_with_no_extract_is_reported_as_unverifiable`** planted `2017/f8949`, which had no archived
extract only because the TY2017 package shipped unarchived. **Every committed row now has an
extract**, so a new `plant_as(src_year, dst_year, stem, edit)` helper copies a real map into a
tempdir under a different year (rewriting its `year` and `line_set` keys) and the kill plants
`2025/f8949` as **TY1999** — the *mechanism* (no extract for this (stem, year)) rather than a year
that happened to be unarchived. A guard asserts `design/forms/extract/f8949--1999.txt` really does
not exist. Seen red twice (§6).

### `btctax-forms/tests/sp4.rs`

**`an_unknown_key_inside_an_inline_money_pair_is_refused_too`** read
`forms/2017/schedule_se.map.toml` — the only committed map using dollars/cents pairs. It now
**builds** the pair: the committed TY2025 Schedule SE map with `line2` rewritten from a single money
field into `{ dollars_field = …, cents_field = … }`, with (a) a guard that the single-field spelling
is still there to rewrite, and (b) a **positive control** that the constructed pair parses before
the unknown key is planted — without which the refusal could be caused by the pair being
unparseable for any reason. Seen red (§6).

---

## 5. Every pinned number moved — old → new, with cause

All causes are the same event (S9 dropped 5 maps / 5 PDFs / 1 YEAR.toml). Every value below was
**measured by running the gate**, never hand-counted.

| pin | file | old → new | cause |
|---|---|---|---|
| `LineSet::ALL.len()` | `line_set.rs` (test) | **41 → 36** | the five `*_2017` revisions |
| `BUNDLED.len()` floor | `supported_years_cross_product.rs:323` | **41 → 36** | the five (pdf, map) pairs |
| on-disk map floor | `bundled.rs::the_generated_bindings_…` | **37 → 36** | floor was already stale (41 on disk); set to the real count |
| `rows.len()` floor | `map_rows.rs` | **41 → 36** | the five rows |
| `from_rows.len()` | `cite_check.rs::the_row_set_equals_the_emitting_surface…` | **41 → 36** | the five rows |
| `bundled_years()` | `bundled.rs` (test) | **`[2017,2024,2025,2026]` → `[2024,2025,2026]`** | the year directory |
| `bundled_years()` | `year_record.rs` (test) | **`[2017,2024,2025,2026]` → `[2024,2025,2026]`** | same |
| `SEQUENCE_UNVERIFIABLE` | `map_rows.rs` | **5 entries → 0** (empty) | the five TY2017 rows were the whole list; **every committed row now has an archived extract** |
| `EXCUSED` (`authority = "not-yet-archived"`) | `map_rows.rs` | **6 → 1** (`(2024, "f8283")` alone) | five of the six were TY2017 |
| `MapRow::authority` doc | `map.rs:137` | **"Six rows today" → "ONE row today"** | same |
| `UNCENSUSED_ENTRIES` | `field_census.rs` | **10 → 5** | the five TY2017 register lines |
| `UNCENSUSED_FIELDS` | `field_census.rs` | **713 → 310** | TY2017's 403 (254+62+8+40+39); TY2025's 310 unchanged |
| `BUNDLED_FORMS_PER_YEAR` | `supported_years_cross_product.rs` | **`[(2017,5),(2024,19),(2025,17)]` → `[(2024,19),(2025,17)]`** | the year |
| `KNOWN_GAPS` | `supported_years_cross_product.rs` | **13 cells / 36 obligations → 8 / 11** | the five TY2017 cells × 5 artifacts each = 25 obligations |
| `KNOWN_ALIASES` | `supported_years_cross_product.rs` | **`[(2017,"f8275"),(2025,"f8275")]` → `[(2025,"f8275")]`** | 2017 is no longer a bundled year, so the periodic alias cannot reach it |
| matrix headline | `supported_years_cross_product.rs` module doc | the historical "21 of 37 / 54" (as-written, 2026-09-05) is now paired with a **measured** current line: **8 of 36 cells / 11 obligations** | ★ I first wrote "16 of 36 / 29" by subtracting 5 and 25 from the *historical* figure — derived, not measured, and wrong. Corrected by extracting `KNOWN_GAPS` and counting: HEAD 13/36 → now 8/11 |
| emitting surface | `cite_check.rs::emitted_form_years` doc | **20 stems / 41 pairs → 20 stems / 36 pairs** | **no stem was lost** — all five have a 2024 and/or 2025 revision |
| `AUTHORITY_NOT_YET_ARCHIVED` | `cite_check.rs` | `f1040`, `f1040sd`, `f1040sse`, `f8283`, `f8949` each **`[2017,2024,2025]` → `[2024,2025]`** | 5 excused pairs removed |
| `GRID_MAPS` | `label_reader.rs` | **7 entries → 5** (`2017/f8949`, `2017/f8283` removed) | the maps |
| `YEAR_FLOORS` | `label_reader.rs` | **TY2017 floor removed** (`min_joins: 0`, `max_unwitnessed: 5`) | it was the table's only zero-join year |
| geometry-fixture hash join | `label_reader.rs` doc | **"the 6 that match none are TY2017's five and `2024/f8283.pdf`" → `2024/f8283.pdf` alone** | measured by the same walk |
| `field_census.rs` scan scope | comment | **"2017/2024/2025 = 37 maps" → "2024/2025 = 36 maps"**; `GAPS` still **0** | TY2017 contributed no `gap` records (it had no census at all) |
| Schedule D line-17 on-state loops | `kats.rs` ×3 | **`[2017,2024,2025]` → `[2024,2025]`** | the year |

**One number I did NOT move, deliberately:** `field_census.rs::GAPS` stays **0**. TY2017 had zero
`[census]` sections, so it contributed no `gap` records to lose — the drop from 713 to 310 is
*deletion of the year*, not census work, and every one of those comments now says so explicitly so
the number is never read as burndown.

---

## 6. Kills, and how each was seen RED

Every kill below was **run against a planted defect and the exact panic text recorded.** Reverts
were done by `cp` from a scratch backup (never `git checkout --`).

### New — `crates/btctax-forms/tests/s9_ty2017_package_is_gone.rs` (3 tests)

| kill | what it holds |
|---|---|
| `no_ty2017_form_package_is_on_disk_or_bound` | the directory is gone; 2017 ∉ `bundled_years()`, ∉ `SUPPORTED_YEARS`; no `BUNDLED` pair; `template()` **and** `map_text()` are `None` for **every** `Stem`; and `periodic_template()` is `None` for both periodic stems (F8275, F8283) — the alias is the one path that could reach a non-file year |
| `no_ty2017_line_set_revision_parses` | all five `"<stem>/2017"` strings → `LineSet::parse == None`, and no `/2017` survives in `LineSet::ALL`. A re-committed TY2017 map would otherwise parse cleanly into a struct with no PDF behind it |
| `no_code_in_the_workspace_names_the_deleted_forms_2017_path` | a filesystem walk of `crates/**/*.{rs,toml}` for `forms/2017` in **non-comment** lines |

**Red observation — plant: restore `forms/2017/{f8949.pdf, f8949.map.toml, YEAR.toml}` from HEAD.**
13 tests went red across the crate, including two of the three new ones:

```
no_ty2017_form_package_is_on_disk_or_bound
  panicked at tests/s9_ty2017_package_is_gone.rs:41
no_code_in_the_workspace_names_the_deleted_forms_2017_path
  panicked at tests/s9_ty2017_package_is_gone.rs:175
```

and the pre-existing gates:

```
form1040::cluster_year_guard::geometry_is_recorded_for_exactly_the_supported_years
  TY2017: SUPPORTED_YEARS.contains = true but f1040_clusters panicked — a supported year
  with no band recorded: measure the amount column off the year's blank PDF and add the arm
schedule_se::cluster_year_guard::geometry_is_recorded_for_exactly_the_supported_years   (same shape)
form8283::geometry_year_tests::form8283_geometry_is_recorded_for_exactly_the_supported_years (same)
bundled::tests::bundled_years_are_the_year_directories
packet::sequence_order_tests::a_shuffled_packet_sorts_into_stapling_order_for_every_bundled_year
field_census::every_emittable_form_is_reached_by_the_gate_or_named_absent
field_census::census_accounts_for_every_field
map_rows::every_committed_map_has_a_row_that_parses_and_all_four_kills_are_green
year_record::the_declared_statuses_are_the_measured_ones
year_record::every_bundled_year_has_a_record_that_partitions_the_closed_set_and_matches_the_glob
supported_years_cross_product::the_cross_product_matrix_matches_the_recorded_gaps
```

`no_ty2017_line_set_revision_parses` correctly stayed **green** under this plant (the `LineSet`
variants were not restored) — the discrimination is real, not "everything reds".

★ `no_code_in_the_workspace_names_the_deleted_forms_2017_path` carries its **own** B1 pair inline,
so it cannot pass blind: the predicate must accept a live path literal
(`join("forms/2017/f1040.map.toml")`) and must reject a comment recording the pre-drop measurement.
Plus a liveness floor on the walk itself (`files >= 300`; **measured 2026-09-06: 389 `.rs`/`.toml`
files under `crates/`**, 388 excluding the test itself).

### New — the two halves of "the year survives, the forms do not"

**`export_irs_pdf.rs::ty2017_is_refused_by_the_export_and_the_refusal_names_the_bundled_years`** —
`export-irs-pdf --tax-year 2017` → `CliError::FormFill(FormsError::UnsupportedYear(2017))`, the
message names every `SUPPORTED_YEAR` and does *not* offer 2017, and the refusal writes **no bytes**.
(The message-to-constant join is separately held by
`supported_years_cross_product::the_unsupported_year_refusal_names_exactly_the_supported_years`,
which derives its expected set from `SUPPORTED_YEARS`.) This test only exists **because** the drop
happened, so its red state is the pre-change build.

**`tax_report.rs::report_tax_year_2017_still_computes_after_the_form_package_was_dropped`** — the
KEPT half. Uses the moved `real_events_2017()` fixture; asserts the raw Schedule D projection
($500 proceeds / $200 basis / $300 **short**-term) and that the rendered report is not
`NOT COMPUTABLE`.

**Red observation — plant: comment out `by_year.insert(2017, ty2017());` in
`btctax-adapters/src/tax_tables.rs`:**

```
report_tax_year_2017_still_computes_after_the_form_package_was_dropped
  panicked at crates/btctax-cli/tests/tax_report.rs:233:
  NOT COMPUTABLE [TaxTableMissing]: no bundled tax table for 2017
```

### Re-pointed kills — each seen red

| kill | plant | red text |
|---|---|---|
| `map_rows::a_row_with_no_extract_is_reported_as_unverifiable` | plant at TY**2024** (a year that HAS an extract) | `the planted year must have NO archived extract, or this kill proves nothing` |
| — same, with that guard removed too | | `panicked at map_rows.rs:579: []` (the `problems` vec is empty — `SequenceUnverifiable` is not reported) |
| `sp4::an_unknown_key_inside_an_inline_money_pair_is_refused_too` | remove the `pennies_field` plant | `panicked at sp4.rs:1047` (the `expect_err` on `parse` — an undoctored pair parses) |
| `year_readiness::a_filable_declaration_without_params_is_reported` (slice arm) | `if false && d.status == YearStatus::Slice && !self.table` | `panicked at year_readiness.rs:490` (`crypto-slice year but no TaxTable` never reported) |
| `extension::a_ty2017_shaped_record_never_prints_04_15_and_its_june_date_is_the_15th` | `extension_due_date` returns `return_due + 2 months` for `out_of_country` | `panicked at extension.rs:438` (the `2018-06-15` assertion — the mutant yields `2018-06-17`) |
| `export_irs_pdf::the_two_pipelines_cannot_overwrite_each_others_files` | skip the TY2025 slice export entirely | `the TY2025 slice must have written f8949.pdf, or the collision never happened` |

---

## 7. What was LOST — three B1 kills that cannot be re-pointed

Recorded because it is a real cost of the ruling, not a defect in the work.

`form1040.rs`, `schedule_se.rs` and `form8283.rs` each hold a **panic** rather than a wildcard in
their column-x-band lookup, and the *evidence* for that panic was a cross-revision measurement made
against the TY2017 PDFs — the only two revisions of one form this repo held both sides of with a
dollars/cents split:

```
TY2017 line-13 dollars  cx 518.1  IN     F1040_CLUSTERS_2017     [482,555]  <- real cell
TY2017 line-13 cents    cx 565.2  NOT IN F1040_CLUSTERS_2017     [482,555]  <- swap fails closed
TY2017 line-13 cents    cx 565.2  IN     F1040_CLUSTERS_UNIFIED  [504,576]  ★
```

Two tests re-derived those numbers from the bundled PDF **on every run**
(`the_2017_band_excludes_a_cents_widget_the_unified_band_admits` and its Schedule SE sibling, with
its four-row equivalent). They are deleted; the numbers are **kept verbatim in the comments**,
explicitly relabelled as history that can no longer be re-derived in-suite, and the panic messages
now say "admitted … (measured before the S9 drop, see the comment above)".

**What survives:** `geometry_is_recorded_for_exactly_the_supported_years` in all three files —
derived from `SUPPORTED_YEARS`, red in **both** directions, and demonstrated red under the §6 plant
(a supported year with no band). The "no wildcard" guarantee still has a live kill; what is gone is
the *why-a-wildcard-is-wrong* measurement.

Nothing was rewritten to make a checker pass: `no_code_in_the_workspace_names_the_deleted_forms_2017_path`
deliberately exempts comments for exactly this reason, and says so in its own doc.

---

## 8. Beyond the brief's explicit list — live product claims about TY2017

The brief said "let the compiler and the suite find the rest." Neither can see a `--help` string.
These were false the moment the package was deleted and are user-facing:

- **`crates/btctax-cli/src/cli.rs`** — the single source for `--help` **and** the committed man
  pages. `--tax-year`'s help read *"this build bundles TY2017, TY2024 and TY2025 … TY2017
  additionally uses the OLD forms — the §B long Schedule SE, Form 8283 Rev. 12-2014 …"*. Rewritten
  to name TY2024/TY2025, to say TY2017's package was dropped, and to point the filer at
  `report --tax-year 2017`. Three more paragraphs in the same doc comment (Box C/F years, 8283
  property-type box, the 1040 capital-gain line) lost their 2017 clauses.
- **`docs/man/btctax-export-irs-pdf.1`** — regenerated with `cargo run -p xtask -- docs`; it is the
  only man page that changed, and re-running `docs` after every later edit reproduces it byte for
  byte.
- **`README.md`** — the "Supported years: **2017**, **2024** and **2025**" paragraph and four
  bullets. Replaced with 2024/2025 plus a blockquote stating what was dropped, why (no archived
  primary source), and that `report --tax-year 2017` still computes. Also `**2017, 2024 and 2025**`
  → `**2024 and 2025**` in the TUI export section, with its "which includes 2018–2023" widened to
  "2017–2023".
- **`docs/examples/examples.md`** — regenerated (`cargo run -q -p xtask -- examples`): **no diff**,
  it never named 2017.
- **`design/TY2026_WORK_LIST.md`** — `cargo run -p xtask -- port-status 2025 2026-DRAFT` compares
  TY2025 to the TY2026 drafts and is untouched by this change; the two xtask tests that hold the
  committed table to the tool (`the_committed_work_list_matches_form_delta_at_head`,
  `port_status_prints_the_committed_work_list`) are green. **No regeneration needed.**

Stale doc comments asserting the build *contains* something it no longer does were also corrected
in `btctax-forms/src/{lib,pdf,form8275,schedule_d,map,year_record,cells}.rs` and
`btctax-core/src/{donation,forms}.rs`. Descriptive statements about the Rev. 12-2014 IRS document
itself, about TCJA/§2017 dates, about the `EraPreset::Y2015To2017` window, and about the **tax
tables** (`docs/architecture/*.md`, `conservative.rs`, `testonly.rs`, `shipped_tables_*.rs`) were
left alone — all remain true, and the TY2017 table is KEPT.

---

## 9. Kept, exactly as the ruling says

- `btctax-adapters::tax_tables::ty2017` and its `ty2017_table_matches_rev_proc_2016_55` KAT —
  untouched. `BundledTaxTables::load()` still inserts 2017.
- The price dataset, ledger fixtures with 2017 dates (`promote_cli.rs:64`,
  `experimental_notice.rs:65`, `scrub_refusal.rs:170`), `Pre2025MethodDeclaration`,
  `EraPreset::Y2015To2017`.
- `YearStatus::Slice`, `MoneyCell::Pair` / `MoneyPair` / `fmt_money_pair`, `Form8283Map`'s
  `btc_property_note` and `deduction` `Option`s, `Form1040Map::line7a`'s `MoneyCell` — all live code
  with no bundled user today. Each now carries a comment saying so, and the `Slice` case is asserted
  absent (`year_record.rs::the_declared_statuses_are_the_measured_ones`) rather than left implicit.

---

## 10. Anything I could not do

Nothing was blocked. Three judgment calls are flagged above rather than hidden:

1. **§4** — the TY2017 end-to-end export KAT was deleted with its two unique assertions transplanted,
   instead of re-pointed to TY2024 where it would have been a byte-identical duplicate of
   `ty2024_real_ledger_fills_box_c_f_and_line7_and_da`.
2. **§7** — three B1 measurements are unrecoverable; their numbers are preserved as comments.
3. **§8** — `cli.rs` and `README.md` were outside the brief's file list. They carried live false
   claims about what the build fills, so they were fixed.

Two small improvements made while re-pointing, both in-doctrine and both reported here rather than
left silent: `kats.rs`'s Schedule D line-17 on-state lookup lost its `(_, …)` wildcard for an
enumerated `2024 | 2025` + panic (a new revision must be dumped, not inherited — the exact reason
the 2017 Rev. used `"Yes"`/`"No"` where 2024/2025 use `"1"`/`"2"`), and `bundled.rs`'s
`template(Stem::F6251, 2017) == None` probe — which the drop made vacuously true — was re-pointed to
`(F6251, 2026)`, a bundled year with no template of its own, with the old `(F1040, 2017)` shape kept
beside it as a probe against a year the glob has never known.

**Not committed. `git status` also shows `CONTINUITY.md`, `FOLLOWUPS.md`, `design/ROADMAP_STATUS.md`
and `design/agent-reports/2026-09-06-build-1099da-R6-review-r2.md` as modified/new — those are the
controller's concurrent edits, not mine.**
