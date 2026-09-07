# Residue sweep 1 — implementation report

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatch HEAD
`8eeb11d4`. Brief: `design/agent-reports/BRIEF-build-residue-sweep-1.md`. **All six items landed.**
Nothing committed, nothing pushed; no subagents were spawned; no file I did not create was reverted
with `git checkout --` / `git restore` (every plant was reverted from a `cp` backup under the
scratchpad).

## Validation

| gate | command | result |
|---|---|---|
| whole suite | `cargo nextest run --workspace --no-fail-fast` | `3175 tests run: 3175 passed, 12 skipped` |
| clippy | `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean, `Finished dev profile in 4.12s` |
| fmt | `cargo fmt --all` | applied, no residual diff |
| cite-check | `cargo run -q -p xtask -- cite-check` | `OK — 51 quotations, all verbatim` + `5/36 emitted pairs archived; 31 excused, 0 unaccounted` |
| manifest | `cargo run -q -p xtask -- authority-manifest` | `OK — every entry resolves and every source is listed` |

Baseline at dispatch was **3168**; **+7** is exactly the seven tests added (four in item 2, one each
in items 1, 3 and 4). Per-crate runs, all `--locked`:

```
cargo nextest run --locked -p btctax-core        1212 tests run: 1212 passed, 0 skipped
cargo nextest run --locked -p btctax-cli          713 tests run:  713 passed, 1 skipped
cargo nextest run --locked -p btctax-tui-edit     382 tests run:  382 passed, 2 skipped
cargo nextest run --locked -p btctax-forms        354 tests run:  354 passed, 4 skipped
cargo nextest run --locked -p xtask               131 tests run:  131 passed, 1 skipped
cargo nextest run --locked -p btctax-tui          160 tests run:  160 passed, 2 skipped
cargo nextest run --locked -p btctax-adapters     103 tests run:  103 passed, 0 skipped
cargo nextest run --locked -p btctax-input-form    65 tests run:   65 passed, 0 skipped
cargo nextest run --locked -p btctax-update-prices  5 tests run:    5 passed, 1 skipped
cargo nextest run --locked -p btctax-oracle-harness 5 tests run:    5 passed, 1 skipped
```

---

## 1. FR-63 — the TUI commit modal's slice clause

**Change.**

| file:line | what |
|---|---|
| `crates/btctax-tui-edit/src/main.rs:1415` | the `CommitOutcome::NoTables` arm builds the existing ≤104-char line as `first`, then appends a SECOND `\n`-separated notice row when the slice would print |
| `crates/btctax-tui-edit/src/draw_edit.rs:2311` | `draw_tax_inputs_status` draws one row per `\n` segment of `app.status` instead of one row for the whole string |
| `crates/btctax-tui-edit/src/draw_edit.rs:2010` | `draw_tax_inputs_form` sizes the status block `5.max(4 + notice_rows)` from the same split, so a second row cannot be built and then clipped |
| `crates/btctax-tui-edit/src/draw_edit.rs:2236` | the P2-a discard modal renders segments too (a `\n` inside one `Line` would print literally) |
| `crates/btctax-tui-edit/src/draw_edit.rs:251` | the Browse footer is one centred row, so segments are JOINED with `" · "` — nothing dropped |
| `crates/btctax-cli/src/year_readiness.rs:197` | new `pub fn slice_prints_from_answers(year, answers_stored)`; the private `slice_clause` now calls it |

**Design decision — a second NOTICE row, not a modal-body line.** The brief offered either. The
modal is *closed* by the `NoTables` arm (`form.modal = None`), so there is no modal body left to
write into; the notice surface IS the only thing on screen. Making that surface multi-line is 5
small edits and leaves every one-line state pixel-identical (the block's height floor stays 5).

**The predicate is shared, not new.** `slice_prints_from_answers` is `answers_stored &&
slice_can_print(year)` — exactly what `report`'s NOT-COMPUTABLE sentence and `income import`'s note
already applied through the private `slice_clause`. **I deviated from the brief here**: the brief
said *"show it when the year's answers are non-empty … **or** the year's regime reports basis"*, and
I did not implement that disjunct. A regime-reports-basis year with no bundled Form 8949 / Schedule
D templates is TY2026 — the exact year the R6 I-1 fold added `slice_can_print` for, because
promising a slice `export-irs-pdf` then refuses to write is the defect that fold closed. Widening
the notice's predicate past the export's own would re-open it. The answers are read off the working
return being committed (`ri.broker_reporting`) rather than through the T9 store accessor: after
`form_save_draft` they are the same bytes, and a fallible read in this arm could only make the
notice less true.

**Wording** (103 chars at a 4-digit year, measured):

```
Form 1099-DA answers are held in the draft; `export-irs-pdf --tax-year 2025` fills the slice from them.
```

**Kill** — `crates/btctax-tui-edit/src/main.rs:10836`
`tax_inputs_notables_notice_carries_the_slice_clause_only_when_the_slice_would_print`. Three
fixtures, one variable each, both premises read off the build rather than assumed
(`slice_can_print(2025)` true, `slice_can_print(2099)` false):

| year | answers | expect |
|---|---|---|
| 2025 | seeded | two rows; row 2 carries the clause; row 1 still has "SAVED as a draft" and "finalize" |
| 2025 | none | one row |
| 2099 | seeded | one row |

Every row is asserted **present in the rendered buffer** (not just in `app.status`) and **≤ 104
chars**, so the r1-M1 invariant is re-held rather than assumed.

**Seen red, four plants** (all reverted from `cp` backups):

| plant | red text |
|---|---|
| the clause is never added (`if false`) | `assertion left == right failed: the notice is two rows here: ["2025 has no full-return tables yet (v1: TY2024) — inputs SAVED as a draft; finalize when tables publish."]  left: 1  right: 2` |
| `slice_prints_from_answers` drops the templates term | `assertion left == right failed: one row only: ["2099 has no full-return tables yet …", "Form 1099-DA answers are held in the draft; \`export-irs-pdf --tax-year 2099\` fills the slice from them."]  left: 2  right: 1` |
| the notice surface draws `s.lines().take(1)` | `★ every NOTICE row must reach the rendered buffer, not just \`app.status\`:` |
| the status block height is pinned at 5 | `★ every NOTICE row must reach the rendered buffer, not just \`app.status\`:` |

The last two are the ones that matter most: they are the two independent ways a correctly-built
second line could still never reach a filer's eyes.

The pre-existing sibling `tax_inputs_notables_status_is_visible_in_flow_render` (the r1-M1 kill) is
untouched and still passes.

---

## 2. The §7503 District of Columbia legal-holiday calendar

**Change** — `crates/btctax-forms/src/year_record.rs`, the weekend-only shifter replaced in place
(so every caller gets the holiday half; no weekend-only function survives, nothing needed one):

| line | what |
|---|---|
| `:230` | `section_7503_shift` — now the first day on or after `d` that is neither a weekend nor a DC legal holiday, iterating because a holiday can abut a weekend. Bounded at 30 with a panic; the 140-year sweep below makes the bound unreachable. Idempotent by construction. |
| `:247` | `is_weekend` |
| `:252` / `:263` | `nth_weekday_of` / `last_weekday_of` (walked forward, so no month-length table can be wrong) |
| `:276` | `observed` — 5 U.S.C. §6103(b): Saturday → preceding Friday, Sunday → following Monday |
| `:290` | `dc_legal_holidays_observed_from(year)` — the eleven §6103(a) holidays in the statute's own order, D.C. Code §1-612.02 Emancipation Day (April 16), and §6103(c) Inauguration Day |
| `:333` | `pub fn is_dc_legal_holiday` — consults years `y-1 ..= y+1`, because observance moves a holiday off its own year (New Year's Day 2022 was a Saturday, observed **2021-12-31**) |
| `crates/btctax-cli/src/cmd/admin.rs:1962` | `extension_due_date`'s non-out-of-country branch now returns `section_7503_shift(return_due)` instead of `return_due` |
| `crates/btctax-cli/src/cli.rs:287` | `--out-of-country` help: "shifted off a weekend" → "shifted off a weekend or a District of Columbia legal holiday" |
| `crates/btctax-cli/tests/extension.rs:11,~375` | module doc and the June-date kill's doc updated to point at where the holiday half is killed |

Nothing was fetched. The holiday list is transcribed from **5 U.S.C. §6103** with the citation, and
the doc comment cites **26 U.S.C. §7503** and **Treas. Reg. §301.7503-1(b)** for why the District's
calendar binds a filer in Anchorage.

**Two judgment calls, both recorded in the code:**

1. **Inauguration Day gets no Saturday in-lieu day.** §6103(c) grants an alternate day only for
   Sunday (the 21st), so it deliberately does not go through the §6103(b) `observed` rule. It can
   never affect an April or June due date; it is there for completeness and is killed in both
   directions.
2. **`extension_due_date` now applies the shifter to the committed `return_due`.** The old comment
   said shifting an already-shifted date "would move a correct date". With a real calendar the
   shifter is a fixed-point map, so it cannot — and applying it makes the April date DERIVED rather
   than trusted: a `YEAR.toml` declaring a blocked day would now print the date the IRS recognises.
   `ty2017s_committed_return_due_is_derived_by_the_dc_holiday_calendar` holds every bundled record
   to that fixed point, so the line is a proven no-op on every year the build carries.

**Kills** — `crates/btctax-forms/tests/year_record.rs`. Every expected value is a date the shifter
must WALK TO, never one it was handed.

| test:line | what |
|---|---|
| `:197` `the_section_7503_shifter_walks_past_weekends_and_dc_legal_holidays` | 2018-04-15→**17**, 2023-04-15→**18**, 2026-04-15→**15** (control), 2024-06-15→**17**, 2025-07-04→**07-07**, 2022-12-25→**12-27** |
| `:241` `ty2017s_committed_return_due_is_derived_by_the_dc_holiday_calendar` | TY2017's committed `return_due` is PARSED out of a shaped `YearRecord` and the shifter must reach it from April 15 alone — neither side is a literal the other was written from. Plus: every bundled record's `return_due` is already a fixed point. |
| `:275` `the_dc_legal_holiday_predicate_names_the_right_days` | 22 rows, each true case paired with a false one a day away: the four floating Mondays and the Thursday, §6103(b) in both directions, the year-boundary New Year's Day, §6103(c) Sunday-alternate (2013-01-21 true) and the missing Saturday rule (2029-01-19/20/22 all false) |
| `:408` `the_shifter_lands_on_the_first_free_day_for_every_date_in_140_years` | 1960-01-01 … 2099-12-31: monotone, lands unblocked, lands on the FIRST free day, idempotent there. Guards the guard: `days == 51_135` and `14_000 ≤ moved < 24_000`. |

**Seen red, five plants:**

| plant | red text |
|---|---|
| Emancipation Day removed | `assertion left == right failed: the shifter must DERIVE the committed 2018-04-17 from the statutory April 15  left: 2018-04-16  right: 2018-04-17` (and two more tests) |
| §6103(b) Saturday in-lieu day removed | `assertion left == right failed: 2021-12-24: Christmas 2021 was a Saturday → observed Friday the 24th  left: false  right: true` |
| shifter takes a single step (`0..1`) | 3 tests FAIL, incl. `the_shifter_lands_on_the_first_free_day_for_every_date_in_140_years` |
| predicate consults only its own year | `assertion left == right failed: 2021-12-31: New Year's Day 2022 was a Saturday → observed Friday 2021-12-31  left: false  right: true` |
| Memorial Day as the THIRD Monday | `assertion left == right failed: 2025-05-26: Memorial Day, LAST Monday in May  left: false  right: true` |

**Two of my own test rows were wrong first and the suite caught them**, which is the point of
deriving rather than asserting: `days == 51_134` was off by one (inclusive span), and `2037-01-19`
was chosen as an Inauguration-Day-Saturday control when Jan 20 2037 is a Tuesday and the 19th is MLK
Jr. Day. Replaced with 2029 (Jan 20 a Saturday, MLK on the 15th) plus the 2013 Sunday-alternate row.

**Golden / man-page movement** — exactly one line, a direct consequence of the `--help` text:

```
docs/man/btctax-extension.1
-…June 15 of the following year (shifted off a weekend under §7503)
+…June 15 of the following year (shifted off a weekend or a District of Columbia legal holiday under §7503)
```

Regenerated with `make docs-man`; `git diff --stat docs/` = `1 file changed, 1 insertion(+), 1 deletion(-)`.

---

## 3. Cite-check fixtures for the four 4868 / 1040-V pairs

**Change** — `crates/xtask/src/cite_check.rs`:

- `:748` `FORMS` goes from **1 row to 5**: `f1040v/2024`, `f1040v/2025` (`instructions = "f1040v"`,
  `instr_pages = (1,2)`), `f4868/2024`, `f4868/2025` (`instructions = "f4868"`, `instr_pages =
  (1,4)`). `instructions` is the form itself because the IRS publishes no `i4868` / `i1040v`; the
  page ranges are the ones each map row MEASURED from its extract's form-feed breaks, and
  `the_forms_const_row_agrees_with_its_map_row` (already existing) now holds all five rows to those
  declarations.
- `:934` the four pairs removed from `AUTHORITY_NOT_YET_ARCHIVED`, with the reason recorded in place
  of the old excuse note.
- Eight new committed fixtures under `crates/btctax-core/src/tax/fixtures/`, generated by
  `cargo run -q -p xtask -- extract-schedule-1a` (the registry-driven extractor, not by hand):
  `f1040v_2024_{form,instructions}.txt` (149 / 258 lines), `f1040v_2025_…` (145 / 254),
  `f4868_2024_…` (329 / 587), `f4868_2025_…` (311 / 555). The two `schedule_1a_2025` fixtures were
  regenerated by the same run and are **byte-identical** (`git diff --stat` empty).

**Pinned numbers moved:**

| pin | old → new | cause |
|---|---|---|
| `cite-check` archived pairs | **1/36 → 5/36** | four `FORMS` rows + their fixtures |
| `cite-check` excused pairs | **35 → 31** | the same four leaving `AUTHORITY_NOT_YET_ARCHIVED` |
| `AUTHORITY_NOT_YET_ARCHIVED` rows | 19 → 17 | `f1040v` and `f4868` rows deleted whole |
| `design/FORM_AUTHORITY_TABLE_DESIGN.md:212` | "36 of 37 pairs excused" → "**31 of 36** … was 36 of 37 when this was written" | this change plus the S9 TY2017 drop |
| `design/FORM_AUTHORITY_TABLE_DESIGN.md:211` | `FORMS` "one row" → "**five rows**" | this change |

**Deviation from the brief.** The brief said the list shrinks **40 → 36**. Measured at the dispatch
HEAD it was **35 excused of 36 emitted**, not 40 of 41 — the 40/41 figure is from the
`2026-09-06-build-4868-T1-T5` report and predates S9, which dropped the five TY2017 pairs the same
day. So the real move is **35 → 31**, the same four pairs. Verified before and after with
`cargo run -q -p xtask -- cite-check`.

**Kill** — `crates/xtask/src/cite_check.rs:1156` `re_excusing_an_archived_pair_reds_the_ratchet`.
The ratchet's `stale_excuses` arm — the one that stops a closed gap silently reopening — **had no
kill**: the existing planted test `a_prior_year_archive_does_not_discharge_a_new_year_obligation`
exercises `unaccounted` and `phantom_excuses` only. The new test plants on the **real** emitted /
archived sets, one archived pair at a time, enumerated **from `FORMS`** so it cannot rot; it asserts
the stale arm fires and that no other arm fires instead, with a control that today's sets are clean
and a `planted == 5` guard so a loop that ran zero times cannot pass.

**Seen red, two plants:**

| plant | red text |
|---|---|
| `stale_excuses: Vec::new()` | `assertion left == right failed: re-excusing ("f1040s1a", 2025) — which HAS an archived, extracted authority — must be reported as a stale excuse; the ratchet may only shrink  left: []  right: [("f1040s1a", 2025)]` |
| `("f4868", &[2025])` re-added to the excuse list | `[f4868--2025] now HAS an archived authority for that YEAR — remove the year from AUTHORITY_NOT_YET_ARCHIVED so the ratchet actually tightens` — and the CLI fails too: `xtask cite-check: authority coverage is not accounted for — unaccounted: []; stale excuses: [f4868--2025]; phantom excuses: []` |

**One doc rule corrected** — `design/forms/README.md:63` said *"`AUTHORITY_NOT_YET_ARCHIVED`
therefore stays as it is until a form reaches **step 3**"*. The ratchet makes that impossible:
`archived_form_years()` counts a pair as covered when a `FORMS` row names it and both fixtures are on
disk (**step 2**), so adding the fixtures turns the excuse stale and reds
`authority_coverage_may_only_improve`. Rewritten to say step 2 and why, keeping the real warning
(shrinking on step 1 — an archived PDF nothing can check against — is the false completeness the
archive exists to prevent).

---

## 4. A named kill for the full return's Section-B unanswered-restriction row

**Change** — `crates/btctax-core/src/tax/return_1040.rs`:

- `:3741` `screened_restriction(claimed, answer)` — the closure that lived inside
  `a_declared_restriction_refuses_at_any_amount_not_just_over_5000` extracted to a module-level
  helper, so the DECLARED arm and the UNANSWERED arm share one fixture and cannot drift onto two.
  The sibling test's assertions are unchanged.
- `:3753` `screened_restriction_at(wages, claimed, answer)` — the same fixture with the wage figure,
  and therefore the §170(b)(1)(C) 30% ceiling, under the caller's control.
- `:3860` **`an_unanswered_restriction_question_refuses_on_a_section_b_year_only`** — the new named
  kill. No production behaviour changed.

**Why the wage knob exists, and a correction I had to make mid-build.** The gate's Section-B premise
is a conjunction: Schedule A line 12 `> FORM_8283_THRESHOLD` ($500) **and** the year's donation
aggregate `> QUALIFIED_APPRAISAL_THRESHOLD` ($5,000). My first draft falsified the $500 conjunct with
a $400 donation — which also falsifies the $5,000 one, so the row proved nothing. **The plant caught
it: dropping the $500 term from the call site left the test green.** The only shape in which the
first is false and the second true is a ceiling-squeezed year, so the row is now AGI $1,600 (30%
ceiling = $480 on line 12) with a $50,000 aggregate. That plant now reds.

**Rows.** Both thresholds are asserted as premises first, so the test cannot silently re-key.

| answer | size | expect |
|---|---|---|
| `None` | line 12 $9,000, aggregate $9,000 | **refuse** — the row |
| `None` | line 12 $4,000, aggregate $4,000 (Section A) | no refusal — conjunct 2 falsified |
| `None` | line 12 $480, aggregate $50,000 | no refusal — conjunct 1 falsified |
| `Some(false)` | $9,000 | no refusal — it is the ANSWER that lifts it, not the size |
| `Some(true)` | $9,000 | refuse — the ternary is complete here |

**Seen red, four plants:**

| plant | red text |
|---|---|
| the `answer.is_none() && section_b` arm removed | `assertion left == right failed: a Section B year PRINTS 5a/5b/5c and btctax holds no answer — it may not file the year  left: None  right: Some(DonationRestrictionsUnresolved)` |
| the `section_b` conjunct dropped inside the gate | `assertion left == right failed: a Section A year never poses the question — silence forgoes nothing  left: Some(DonationRestrictionsUnresolved)  right: None` |
| the $500 conjunct dropped at the call site | `assertion left == right failed: the ceiling held line 12 at $480: no 8283 over $500, so nothing asks the filer  left: Some(DonationRestrictionsUnresolved)  right: None` |
| the $5,000 conjunct dropped at the call site | `assertion left == right failed: a Section A year never poses the question — silence forgoes nothing  left: Some(DonationRestrictionsUnresolved)  right: None` |

Small correction to the brief's parenthetical for the record: the gate's second term is the **year
donation aggregate** over $5,000 (`forms::year_donation_deduction`), not "a single donated item" —
the test is written to the code's real premise and says so.

---

## 5. The orphaned doc block

`crates/btctax-cli/src/cmd/admin.rs`: the 4-line block written for `slice_broker_refusal` sat at
`:2262`, above `slice_map_gate` (`:2277`) and 100 lines from its own function. Moved verbatim onto
`pub fn slice_broker_refusal` — now `:2366`, function at `:2370`. Zero characters changed inside the
block; no behaviour change.

`slice_map_gate` (`:2279`) and `first_unresolved_map` (`:2347`) **already have** doc comments, so
nothing was added — the brief's "if they lack them" does not fire.

**Check.** `CARGO_TARGET_DIR=target-doc cargo doc -p btctax-cli --no-deps` exits 0. The crate has 16
pre-existing rustdoc warnings (unresolved intra-doc links, public-links-to-private); the seven that
originate in `admin.rs` are at lines **308, 327, 402, 410, 510, 620, 1988** — none in the moved
block's range (2262–2370), so the move introduced none and resolved none.

---

## 6. `f8995a--2025` archived as an authority (evidence only)

Followed `b60c600c` exactly. **No map, no filler, no bundled template.**

| artifact | detail |
|---|---|
| fetch | `https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf` → HTTP **200**, **117,129** bytes |
| sha256 | `3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa` |
| note | `design/forms/2025/f8995a--2025.pdf.txt`, the existing 2025 note convention |
| extract | `design/forms/extract/f8995a--2025.txt` — `pdftotext -layout`, 103 lines |
| geometry | `design/forms/geometry/f8995a--2025.json` — `1257 words, 111 boxes, 2 pages (sha256:3362db81…)` |
| manifest | `authority-manifest --regen` → `regenerated 139 entries`; then `authority-manifest` → **OK** |

The PDF itself is gitignored (`design/forms/**/*.pdf`), as every archived source is.

**Year read off the DOCUMENT, not the filename.** Page 1's masthead prints `8995-A … Qualified
Business Income Deduction … 2025`, and the Part I threshold sentence reads *"above $197,300 ($394,600
if married filing jointly)"* — the TY2025 figures. The committed `f8995a--2024.txt` at the same line
reads `$191,950 ($383,900 …)`, so the two revisions are distinguishable from their own text. Both
facts are recorded in the note. (Independent corroboration found after the fact: the 2026-09-05
recon report `2026-09-05-ty2026-port-authority.md:94` had already measured 117,129 bytes and the same
sha256 — my fetch matches it exactly.)

**Manifest pin moved: 138 → 139 entries** (one new `form` document). By kind: `form 56, guidance 22,
instructions 30, publication 6, regulation 6, statute 19`. Duplicate-source groups stay at the pinned
**0**.

**`port-status` changed, so `design/TY2026_WORK_LIST.md` was regenerated from it.** Exactly one row
moved:

- OUT of the "Not listed" table, where its cell read *"**NO PRIOR SIDE** — bundled for 2024 only; no
  `f8995a--2025` authority or extract archived"*;
- INTO the numeric table as **`| f8995a | 111 | 3 | 0 | 0 | port |`**.

Numeric table 14 rows → **15**; excused table 6 → **5**. The prose note explaining the move is added
above the closing paragraph. `the_committed_work_list_matches_form_delta_at_head` and
`port_status_prints_the_committed_work_list` both pass against the regenerated document.

**One test's premise had to be repaired, and this is the interesting part of item 6.**
`form_delta::tests::the_work_list_checker_reds_on_every_planted_row` used **`f8995a` as its
prior-side-absent stem** in three plants, because it was the only stem in the tree with a 2026 draft
and no `--2025` prior. Archiving `f8995a--2025` destroyed that premise, and the failure was not
cosmetic: with a computable pair the row leaves the excused arm entirely, so one plant would have
kept `wrong.len() == 1` **for the wrong reason** (`"excused as having no pair, but form-delta
computes one"`) and gone on passing while testing nothing.

Observed red first:

```
NO FINAL is TRUE for f8995a today (a draft is not a final):
  ["f8995a: excused as prior=\"**NO PRIOR SIDE**\" / TY2026=\"**NO FINAL** — planted\",
    but on disk prior=true draft=false"]
```

Fixed by moving those three plants to prior tag **`2017`** (`crates/xtask/src/form_delta.rs:692`,
`:803`) — TY2017 was dropped whole by S9, so `f8995a--2017` is absent *by construction* rather than by
which years happen to be archived, and no future archive can silently re-break the premise. Both new
premises are asserted rather than assumed (`pdf_for("f8995a--2017").is_none()` and
`compute("f8995a--2017", "f8995a--2026-DRAFT").is_err()`), and the NO-DRAFT / NO-FINAL pair now
differs in exactly one variable — the new-side tag — so it still isolates the draft-vs-final
distinction it was written for.

---

## Constraints observed

- No spec file touched; no `design/agent-reports/*.md` touched but this report.
- Census registers and `max_unwitnessed` unmoved (`git diff` touches neither).
- Goldens/man pages moved only as a consequence: **one** line in `docs/man/btctax-extension.1`,
  listed above. No TUI golden moved — the two-row notice is a new state, and the one-row states are
  byte-identical.
- Nothing committed or pushed. Working tree carries the change for the controller's `make check` and
  the one seam verification.

## Files changed

```
FOLLOWUPS.md                                        (FR-63 / FR-49 residue / FR-50 marked closed)
crates/btctax-cli/src/cli.rs
crates/btctax-cli/src/cmd/admin.rs
crates/btctax-cli/src/year_readiness.rs
crates/btctax-cli/tests/extension.rs
crates/btctax-core/src/tax/return_1040.rs
crates/btctax-forms/src/year_record.rs
crates/btctax-forms/tests/year_record.rs
crates/btctax-tui-edit/src/draw_edit.rs
crates/btctax-tui-edit/src/main.rs
crates/xtask/src/cite_check.rs
crates/xtask/src/form_delta.rs
design/FORM_AUTHORITY_TABLE_DESIGN.md
design/TY2026_WORK_LIST.md
design/forms/MANIFEST.json
design/forms/README.md
docs/man/btctax-extension.1
new: crates/btctax-core/src/tax/fixtures/{f1040v_2024,f1040v_2025,f4868_2024,f4868_2025}_{form,instructions}.txt
new: design/forms/2025/f8995a--2025.pdf.txt
new: design/forms/extract/f8995a--2025.txt
new: design/forms/geometry/f8995a--2025.json
untracked (gitignored): design/forms/2025/f8995a--2025.pdf
```
