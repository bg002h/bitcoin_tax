# BUILD report — spec 1099-DA **R6** (FR-62): the crypto slice files a LIVE year from the stored answers

Single implementer, shared main tree, branch `main`, base HEAD `6e23e503`. Nothing committed, nothing
pushed. No subagents. Suite at base HEAD: **3153 passed**; after this build: **3178 passed, 0 failed,
12 skipped** (`cargo nextest run --locked --no-fail-fast --workspace`).

---

## 1. What landed, per R6 item

### THE DISPATCH (three-way)

`crates/btctax-cli/src/cmd/admin.rs`

| R6 item | where |
|---|---|
| the predicate is a function call, never a declared status — `BundledFullReturnTables::load().full_return_for(year).is_none()` | `admin.rs:729-735` (`params_bundled`) |
| arm (1) — inputs stored **AND** params bundled → the full packet; the `exists` branch keeps its early `return` **only** here (M-2) | `admin.rs:736` |
| arm (2) — the ANSWERS stored **AND** params **not** bundled → the crypto slice, reached by NOT returning early | `admin.rs:755-760` (`files_from_answers`) |
| arm (3) — everything else, incl. the fourth cell (answers stored, params bundled, no committed row — M-13) | `admin.rs:843-848` |

### T9 — reading the answers without committing a return

`crates/btctax-cli/src/input_form_store.rs`

| R6 item | where |
|---|---|
| `working_return(conn, year) -> Result<(Option<ReturnInputs>, Option<StaleNote>), CliError>`, a thin wrapper over `load` (§6.1 precedence: a draft shadows the committed row; §6.3 stale split unchanged) | `input_form_store.rs:242-262` |
| `Loaded::Draft { parked: true, .. }` → `None` (withdrawn testimony is never filed) | `input_form_store.rs:247` |
| `broker_answers(conn, year)` — the `.broker_reporting` projection arm (2)'s predicate reads | `input_form_store.rs:265-270` |
| the accessor is called only **after** arm (1) is ruled out (N-7) | `admin.rs:755` (below the `return Ok(report)` at `:751`) |
| **every** arm-(2) gate reads that ONE resolution — `screen_broker_reporting(ri, …)`, the Form 8283 restriction row, `route_8949_boxes` — never `return_inputs::get` | `admin.rs:810-843` (all read `working.as_ref()`) |
| `Session::broker_reporting_answers` resolves through it, over the **union** of `return_inputs::years` and the draft table | `session.rs:600-616`, `input_form_store.rs:109-120` (`draft_years`) |
| `btctax export --csv` resolves through it | `admin.rs:209-218` |
| the `StaleNote` printed **before** the file list (M-14) | report field `admin.rs:1138`; printed `main.rs:866` |
| no row is ever created; `ReturnInputs::default()` is never persisted; `resolve.rs` untouched; I-11 untouched | `resolve.rs` is **not in the diff**; kills below |

### Arm (2)'s gates, in order, before any byte (I-5)

All in `export_irs_pdf_from_session_with_regime`:

1. the promote gate — `admin.rs:768`
2. **the form-level gate** — `admin.rs:783-791` (`slice_map_gate`, `admin.rs:2219`), naming the first
   missing stem (`first_unresolved_map`, `admin.rs:2271`); `--forms` narrows the set through `wants()`;
   `f8283`/`schedule_se` only when this year's data reaches them, `f8275` unconditionally when a
   promoted leg files. `out_dir` is never created.
3. the pseudo-attestation gate — `admin.rs:800-804`
4. `screen_broker_reporting` — `admin.rs:810-823`, in the **slice's own sentence** carrying the same
   `reason`/`detail`, never "the return is not computable" (M-1)
5. the Form 8283 restriction row in its slice form — `admin.rs:827-836`
6. `year_readiness::price_coverage_or_refuse(year)` on **both** arms — `admin.rs:853` (slice),
   `admin.rs:1487` (full return); the function `year_readiness.rs:268`, its pure half `:283`.
   `YearReadiness::problems`/`sentence` are unchanged.
7. the re-worded `--pay-by-check` (`admin.rs:863-880`) and `--forms full-return` (`admin.rs:883-901`)
   refusals — re-worded **only** when the year's parameters are not bundled, so arm (3)'s existing
   wording (and its two KATs) is untouched.

`screen_inputs` and `screen_compute_dependent` are **not** run on this arm.

### T8 — the slice's Schedule D, per box

| R6 item | where |
|---|---|
| `btctax_core::forms::schedule_d_by_box(rows) -> BTreeMap<Form8949Box, ScheduleDPart>`, keyed on the box alone | `crates/btctax-core/src/forms.rs:498`; re-exported `core/src/lib.rs:28` |
| the box→line table 1b = A\|G, 2 = B\|H, **3 = C\|I**, 8b = D\|J, 9 = E\|K, **10 = F\|L** | `crates/btctax-forms/src/schedule_d.rs:125-247` (doc table at `:110-124`) |
| `fill_schedule_d_totals(totals, by_box, map)` / `fill_schedule_d(&totals, &by_box, year)` (M-8) | `schedule_d.rs:125`, `forms/src/lib.rs:188` |
| the unbound-row refusal (`need`), mirroring `schedule_d_full::need` | `schedule_d.rs:99-108` |
| routed rows reach the filler from the arm that routed them | `admin.rs:902` (`by_box` from the routed `rows`) → `admin.rs:985`; `render.rs:1285` |
| `schedule_d.csv` gains a `box` column, one row per (part, box) | `render.rs:1285-1306`, `part_of_box` at `:1308` |
| the three-artifact cross-check in `btctax-cli`, reading the written CSV from a tempdir | `tests/slice_from_answers.rs::the_csv_the_schedule_d_and_the_8949_carry_the_same_per_box_partition` |
| the forms-side half re-scoped per box and **renamed** | `kats.rs::schedule_d_box_lines_match_the_form8949_page_set_totals` (was `schedule_d_totals_match_form8949_and_csv`) |

**The fifteen call sites, all updated as transcription (none relaxed):** twelve `fill_schedule_d` —
`admin.rs:985`, `kats.rs` ×6 (`:85, :289, :417, :508, :602, :762`), `sp3.rs` ×2, `sp3b.rs` ×3 — and
three direct `fill_schedule_d_totals` — `kats.rs:475`, `kats.rs:492` (the two halves of the B1 kill
`a_swapped_yes_no_map_fails_closed_instead_of_rendering_a_blank_box`) and `sp3b.rs:603`
(`fault_injected_2017_schedule_d_column_swap_is_red`). Both B1 kills still red on their own plants;
neither assertion was weakened. Hand-built-totals sites pass `not_reported_by_box(&totals)` (new
helper, `forms/tests/common/mod.rs:143-152`), which puts each part on its not-reported box — the
same two lines the pre-T8 fill wrote, so **every golden hash is unchanged**
(`GOLDEN_F8949_SHA256`, `GOLDEN_2024_SCHED_D`, `GOLDEN_2017_SCHED_D` all still green).

### The surfaces

| R6 item | where |
|---|---|
| the arm-(2) report note, the attachment-set wording, printed AFTER the file list (M-6) | `IrsPdfReport::slice_attachment_note` (`admin.rs:311-315`), set `admin.rs:1126`, printed `main.rs:887` |
| the two exit sentences (M-13) | `slice_broker_refusal`, `admin.rs:2317-2325` |
| `uncomputable_sentence` + `import_note` gain the slice clause; `import_note` no longer says `report` will refuse (I-4) | `year_readiness.rs:176-183`, `:196-205` |
| `report` (2a)/(2b): the flag to `render_tax_outcome`, the answers block reading the T9 resolution (M-15) | `cmd/tax.rs:455` (`slice_prints_from_answers`), `:474-489` (`stored_answers_reach_the_slice`), `:544-556` (the answers block), `render.rs:1382` + `:1400-1413`, `main.rs:158/177` |
| the TUI export screens + routes BEFORE `mkdir_owner_only_exclusive`, from `snap.broker_answers.get(&year)` (I-2) | `crates/btctax-tui/src/export.rs:180-194`, `:222` |
| the `--pay` NOTE (same M-10 class) no longer claims "no full-return inputs" on arm (2) | `admin.rs:1166-1184` |

### The test seam (stated plainly)

`btctax_cli::testonly::export_irs_pdf_with_regime` (`testonly.rs:671`) →
`cmd::admin::export_irs_pdf_from_session_with_regime` (`admin.rs:679`). Everything else is the
production path (one `Session::open`, one projection, the same gates in the same order); `None` is
production and the year's record decides, exactly as before.

**Why a seam at all.** R6's printing kills need a year with BOTH a live (basis) regime AND bundled
templates, and no bundled year has both: TY2026's record is live and bundles **zero** templates
(R6 N-5 — nothing about a printed slice is observable there); TY2025 bundles fifteen templates under
a `proceeds`-only regime. So the printing kills run on **TY2025's templates with the LIVE regime
injected**, the pattern `btctax-core/tests/kat_broker_reporting.rs` uses throughout (there the regime
is already a parameter). Every kill that does not need a template — the refusals, the CSV boxes,
`report`, the TUI export — runs on the real command with the real record.

---

## 2. Kills — each seen RED once on a planted defect

Plants were applied with a `cp` backup, the named test run, then restored.
★ One process note: the driver first used `shutil.copy2`, which **preserves mtime** — the restored
file then looked older than the build artifacts and cargo reused the **planted** binary on the next
run. That produced one phantom failure before it was caught; the driver now `touch`es on restore, and
the whole workspace suite was re-run from a forced rebuild afterwards.

| # | planted defect | test that went RED | red text (abridged) |
|---|---|---|---|
| 1 | the Box G group routed back onto line 3 (`box_group(by_box, &[C, I, G])`) | `kats::a_basis_matches_slice_puts_the_g_total_on_line_1b_and_leaves_line_3_blank` + `kats::schedule_d_box_lines_match_the_form8949_page_set_totals` | *assertion `left == right` failed: line 3 is "Box C or Box I" — with no I rows it must be BLANK, not zero* |
| 2 | an unbound per-box row silently skipped instead of `need` | `kats::an_unbound_box_row_with_rows_to_print_refuses_and_without_them_fills` | panicked at `kats.rs:728` — *a routed G group with no line-1b cells must REFUSE, never drop the total* |
| 3 | `schedule_d.csv` reverted to two PART rows | `slice_from_answers::the_csv_…_per_box_partition` + `export::export_writes_year_scoped_form8949_and_schedule_d` | *assertion failed: one CSV row per box group, named by box* / *the CSV names the BOX its total belongs to* |
| 4 | `working_return` reads `return_inputs::get` instead of `load` | `a_draft_shadows_the_committed_row_on_every_surface`, `ty2025_with_stored_answers_prints_the_slice_from_either_row`, `a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283` (3 of 4) | *the PDF files Box I (draft NotReported over committed BasisMatches)* |
| 5 | a PARKED draft treated as the working return | `a_parked_draft_is_not_the_working_return_and_the_export_falls_to_arm_three` | *the accessor returns None for a parked draft* |
| 6 | the `exists` branch keeps its early return unconditionally (pre-R6) | `ty2025_with_stored_answers_prints_the_slice_from_either_row` | panicked at `slice_from_answers.rs:187` — *the slice must print* |
| 7 | `Snapshot.broker_answers` enumerates only the COMMITTED years | `ty2025_with_stored_answers_prints_the_slice_from_either_row` | *draft_only=true: the year's answers reach the Snapshot* |
| 8 | the form-level gate always passes | `cmd::admin::slice_broker_tests::the_form_level_gate_names_the_first_missing_stem` | panicked at `admin.rs:2369` — *a year with no f8949 map must refuse* |
| 9 | the export-time price gate always passes | `year_readiness::tests::the_export_time_price_gate_refuses_a_dataset_that_stops_short` | panicked at `year_readiness.rs:328` — *a short dataset must refuse* |
| 10 | the Form 8283 restriction row removed | `a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283` | panicked at `slice_from_answers.rs:926` — *a declared restriction must refuse* |
| 11 | `screen_broker_reporting` not run before the route | `an_unanswered_key_refuses_in_the_slices_own_sentence_and_writes_nothing`, `a_stored_answer_no_row_reads_refuses_before_any_byte` | *the slice sentence carries the screen's REASON and names the key: … the stored Form 1099-DA answers do not settle every Form 8949 row* |
| 12 | the voucher / `--forms` refusals reverted to the params-bundled wording | `the_voucher_and_full_return_refusals_are_reworded_on_arm_two` | *the voucher refusal names the real reason: … Form 1040-V accompanies a full return; see `income import`* |
| 13 | the `--pay` NOTE keeps the params-bundled wording on arm (2) | `the_voucher_and_full_return_refusals_are_reworded_on_arm_two` | *the note names the real reason on arm (2): … 2025 has no full-return inputs …* |
| 14 | ONE exit sentence for both states | `cmd::admin::slice_broker_tests::the_slice_arm_refuses_only_on_a_live_year` | panicked at `admin.rs:2412` — the params-LESS year's exit must be the slice itself |
| 15 | the arm-(2) attachment-set note never set | `ty2025_with_stored_answers_prints_the_slice_from_either_row` | panicked at `slice_from_answers.rs:197` — *arm (2) carries the attachment-set note* |
| 16 | the `StaleNote` dropped instead of surfaced | `the_stale_draft_split_holds_on_the_export_path` | *the export surfaces the StaleNote the way `scrub` does* |
| 17 | `slice_prints_from_answers` hardcoded `false` | `report_in_state_2a_keeps_its_exit_code_and_gains_the_slice_clause` | panicked at `slice_from_answers.rs:1016` — the NOT-COMPUTABLE line must not tell the filer the year is dead |
| 18 | `report`'s answers block reads the COMMITTED row again | `report_in_state_2a_keeps_its_exit_code_and_gains_the_slice_clause` | *the Form 1099-DA answers block prints from the draft* |
| 19 | the state-(2b) sentences lose their slice clause | `state_2b_is_unchanged_and_both_sentences_name_the_slice` | *the uncomputable sentence keeps the inputs AND names the slice: …* |
| 20 | the TUI export screens AFTER `mkdir_owner_only_exclusive` | `btctax-tui export::tests::the_tui_export_screens_and_routes_from_the_snapshots_answers` | panicked at `export.rs:352` — *the refusal leaves NO directory* |
| 21 | the TUI export passes `None` answers (pre-R6) | same test | panicked at `export.rs:379` — *an answered live year exports* |

**Both halves, everywhere.** Each refusal kill is paired with the case that must still FILL, so a
gate that refused everything could not pass: the unbound-row kill fills clean with no G rows; the
8283 restriction kill exports `form_8283.pdf` on its `false` twin; the form-level gate has
`every_bundled_slice_year_passes_the_form_level_gate`; the price gate passes TY2024/TY2025 live and
refuses TY2026 live; `the_slice_clause_is_absent_when_no_answers_are_stored` reds a hardcoded `true`.

### R6's own kill list, mapped

| R6 kill | test |
|---|---|
| TY2025 + stored answers (committed row **and** DRAFT-only) → the slice PRINTS | `ty2025_with_stored_answers_prints_the_slice_from_either_row` |
| LIVE regime + `basis_matches` → the T8 kill (G on 1b, 3 blank; G page-set) | `a_basis_matches_answer_lands_on_schedule_d_line_1b_not_line_3`; forms half `kats::a_basis_matches_slice_puts_the_g_total_on_line_1b_and_leaves_line_3_blank` |
| G+I mix cross-foots; 3 artifacts agree | `the_csv_the_schedule_d_and_the_8949_carry_the_same_per_box_partition`, `kats::schedule_d_box_lines_match_the_form8949_page_set_totals` |
| TY2024 slice → line 3 keeps the whole Part I total, 1b blank | `kats::a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3` |
| a map with `line1b` removed + a routed G group → `FormsError`, zero bytes; same map, no G rows → fills clean | `kats::an_unbound_box_row_with_rows_to_print_refuses_and_without_them_fills` |
| one key unanswered → the slice's sentence, `wrote_nothing` | `an_unanswered_key_refuses_in_the_slices_own_sentence_and_writes_nothing` |
| a stored answer no row reads → the unread refusal | `a_stored_answer_no_row_reads_refuses_before_any_byte` |
| TY2024 + inputs → the full packet still | `ty2024_with_inputs_still_exports_the_full_return` |
| `--pay-by-check` on arm (2) → the I-7 refusal | `the_voucher_and_full_return_refusals_are_reworded_on_arm_two` |
| the partially-ported-year refusal (one map bound, the next missing, naming `schedule_d`) | `the_form_level_gate_names_the_first_missing_stem` |
| the 8283 restriction refusal + its `false` twin | `a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283` |
| the TUI export's two states | `btctax-tui export::tests::the_tui_export_screens_and_routes_from_the_snapshots_answers` |
| `report` (2a) exits 0 with a profile / 1 without, both with the answers block + the clause | `report_in_state_2a_keeps_its_exit_code_and_gains_the_slice_clause` |
| state (2b) unchanged | `state_2b_is_unchanged_and_both_sentences_name_the_slice` |
| the two exit sentences | `the_slice_arm_refuses_only_on_a_live_year`, `params_bundled_with_a_draft_only_return_names_committing_as_the_exit` |
| the price-coverage refusal | `the_export_time_price_gate_refuses_a_dataset_that_stops_short` |
| draft vs committed, both directions, on the CLI export / `export --csv` / the TUI export | `a_draft_shadows_the_committed_row_on_every_surface` |
| draft-only vault: `return_inputs::exists` FALSE, the export reaches arm (2) | `ty2025_with_stored_answers_prints_the_slice_from_either_row` |
| a stored `tax_profile` still resolves `StoredProfile` (the create-a-row kill) | `saving_the_answers_creates_no_row_and_a_stored_profile_still_wins` |
| stale `parked = 1` → refuse, `out_dir` absent; stale `parked = 0` → discarded, arm (3) | `the_stale_draft_split_holds_on_the_export_path` |
| a current `parked = 1` draft → arm (3) | `a_parked_draft_is_not_the_working_return_and_the_export_falls_to_arm_three` |
| a params-less commit of the FULL return → `NoTables` + draft (I-11 unmoved) | `a_params_less_year_still_refuses_to_commit_and_keeps_the_draft` |

---

## 3. Deviations from R6 as written

**(D-1) The TUI commit modal's sentence was NOT added.**

> R6, T9: *"the TUI block is the primary authoring surface on a params-less year, and its commit
> modal says the answers are held in the draft and that the slice reads them."*

What I did instead: nothing on that surface. Why: the `CommitOutcome::NoTables` status line
(`crates/btctax-tui-edit/src/main.rs:1404-1408`) is a **no-wrap NOTICE line kept ≤ ~104 chars** — an
explicit r1-M1 review finding — and `main.rs:10769-10777` asserts the RENDERED buffer still contains
all three of "no full-return tables", "SAVED as a draft" and "finalize". The shortest wording that
carries the slice clause as well measured 120 characters, so adding it would silently truncate the
"finalize" clause the existing kill protects. The fact itself is not lost: `import_note` (which
`income import` prints on exactly this year) and `uncomputable_sentence` (which `report` prints) both
now say `export-irs-pdf --tax-year {y}` still prints the crypto slice, and the export itself carries
the attachment-set note. **This is the one R6 sentence not built; it needs either a second NOTICE
line or a modal-body slot, which is a TUI-layout change the brief's surface list does not include.**

**(D-2) The form-level gate treats Form 8949 and Schedule D through `wants()` like every other form.**

> R6: *"`Form8949Map` and `ScheduleDMap` (the slice always selects them — the same principle, not an
> exemption)"*

Read literally that could mean "always required regardless of `--forms`". I applied `wants()`
uniformly, so `--forms schedule-se` does not demand the f8949 map. With no `--forms` (the normal
case, and the one the parenthetical describes) the two readings coincide, and the uniform one is the
"same principle, not an exemption" the sentence asks for. Stated here because the two differ under a
narrowing `--forms`.

**(D-3) A test seam was added.** `export_irs_pdf_with_regime` (§1, above). R6 says the kills run on
"TY2025's templates with the LIVE regime injected"; that injection needs a hook, and this is it. It
is a `pub` fn in `btctax_cli::testonly` delegating to a `pub(crate)` inner; production
(`export_irs_pdf`, `export_irs_pdf_from_session`, the chokepoint's `apply_export`) passes `None` and
reads the year's record exactly as before.

**(D-4) `schedule_d.csv` carries ONLY the per-(part, box) rows — no part-total row.** R6 says "one
row per (part, box) group"; the part total is asserted as the sum of the groups. A file mixing group
rows with a total row double-counts under any naive `SUM()` (the hazard `removals.csv`'s per-leg
`claimed_deduction` column already documents), so no total row was added. Consequence: a year with
no long-term rows now writes **one** row where it used to write two (an LT row of zeros asserting a
Part II that does not exist).

**(D-5) The `--pay` NOTE was re-worded too**, alongside the two refusals R6 names. Not asked for
explicitly; it is the same sentence class (M-10) and my change is what made it reachable on a year
whose filer HAS authored inputs, where "2025 has no full-return inputs" is false. `admin.rs:1166`.

---

## 4. Pinned numbers moved

| pinned thing | old → new | cause |
|---|---|---|
| whole-workspace suite | **3153 → 3178** passed | +25 tests (17 `slice_from_answers`, +3 forms KATs, +2 form-gate, +2 price-gate, +1 TUI) |
| `export.rs::export_writes_year_scoped_form8949_and_schedule_d`: `schedule_d.csv` header | `["part","proceeds","cost_basis","gain"]` → `["part","box","proceeds","cost_basis","gain"]` | T8 |
| …its row count | `2` (ST, LT) → `1` | T8 — one row per (part, box) group; this fixture has one Box I short-term group and no long-term rows (D-4) |
| `kats::schedule_d_totals_match_form8949_and_csv` | renamed → `schedule_d_box_lines_match_the_form8949_page_set_totals` | I-9/I-13 — it never opened a CSV; the real three-artifact check moved to `btctax-cli` |
| `render_tax_outcome` arity | 4 → 5 args (`slice_prints_from_answers`) | M-15; 7 test call sites updated |
| `fill_schedule_d` / `fill_schedule_d_totals` arity | +1 (`by_box`) | M-8; 15 call sites updated as transcription |

**Not moved:** every golden hash (`GOLDEN_F8949_SHA256`, `GOLDEN_2024_F8949`, `GOLDEN_2024_SCHED_D`,
`GOLDEN_2017_F8949`, `GOLDEN_2017_SCHED_D`), the census registers, `max_unwitnessed`,
`YearReadiness::problems`/`sentence`, `SUPPORTED_YEARS`, `resolve.rs`.

## 5. Golden / doc diffs

**None.** `cargo run -q -p xtask -- examples` reproduces `docs/examples/examples.md` byte-identically
(`diff -q` clean, and `examples_golden_matches_committed` is green in the suite);
`cargo run -q -p xtask -- docs` left every page in `docs/man/` unmodified (`git status` shows no
non-`.rs` change). The R6 sentences that moved — `import_note`, `uncomputable_sentence`, the export
notes, the two refusals — are not reached by any journey in the examples corpus or by `--help`.

## 6. Commands run, with their summary lines

```
$ cargo nextest run --locked --no-fail-fast --workspace
     Summary [  18.139s] 3178 tests run: 3178 passed, 12 skipped

$ cargo nextest run --locked -p btctax-cli -E 'binary(slice_from_answers)'
     Summary [   0.788s] 17 tests run: 17 passed, 0 skipped

$ cargo nextest run --locked -p btctax-forms -E 'binary(kats) or binary(sp3) or binary(sp3b)'
     Summary [   0.471s] 58 tests run: 58 passed, 0 skipped

$ cargo nextest run --locked -p btctax-cli
     Summary [   5.617s] 707 tests run: 707 passed, 1 skipped

$ cargo nextest run --locked -p btctax-forms
     Summary [   5.224s] 366 tests run: 366 passed, 4 skipped

$ cargo nextest run --locked -p btctax-core
     Summary [   0.537s] 1211 tests run: 1211 passed, 0 skipped

$ cargo nextest run --locked -p btctax-tui
     Summary [   1.599s] 160 tests run: 160 passed, 2 skipped

$ cargo fmt --all --check           # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.21s     # 0 warnings
```

## 7. What I could not do / notes for the reviewer

- **D-1** above is the one R6 sentence not built.
- **`make check` was not run** (the brief reserves it for the controller's pre-commit gate); the
  workspace nextest run, `cargo fmt --all --check` and the clean clippy are what is above. The
  CI-only jobs (msrv, pii-scan, net-isolation) were not run either.
- **Behaviour widening a reviewer should look at directly.** A year with a COMMITTED `ReturnInputs`
  row and no bundled parameters used to reach the full-return arm and refuse ("no full-return tables
  for 2025"). It now falls through: with non-empty answers it is arm (2) and PRINTS; with empty
  answers it is arm (3) and prints the slice as a no-inputs year would. That is what R6's arm-(1)
  predicate (`inputs stored AND full_return_for is Some`) requires, and the headline kill asserts the
  first half — but the second half (a committed row, no answers, params-less year → the slice prints)
  is a state R6 does not name explicitly and no kill pins. Worth a reviewer's eye.
- **The plant-driver mtime trap** (§2) is a process finding, not a code one, but it is the shape that
  makes a whole B1 round meaningless: a restore that preserves mtime leaves cargo serving the planted
  binary, so the *next* green run is a lie. Anyone re-running these plants should `touch` on restore.
