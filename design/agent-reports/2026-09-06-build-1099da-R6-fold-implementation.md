# FOLD — the R6 build's seam review (1C / 2I / 4M / 1N), all seven items landed

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatched at
`8732a805`. Brief: `design/agent-reports/BRIEF-fold-1099da-R6-build.md`. Review folded:
`design/agent-reports/2026-09-06-build-1099da-R6-review.md` (findings from line 146); its ledger
says every checkable claim HOLDS, so each finding was folded rather than re-verified — **with one
exception, M-1, whose premise is factually wrong about this codebase and is recorded as a deviation
below.**

Nothing committed, nothing pushed. No subagents. No spec or other `design/agent-reports/*.md`
touched. **No golden and no man page moved** (verified: `xtask::examples` goldens,
`btctax-tui::tabs` walkthrough goldens and `btctax-forms::golden_packet` all pass unchanged; no
`--help` text was edited).

## Files changed (11)

```
crates/btctax-cli/src/cmd/admin.rs            C-1, M-3, M-2, N-1
crates/btctax-cli/src/cmd/tax.rs              I-1, M-4 (call sites)
crates/btctax-cli/src/resolve.rs              M-4 (call sites)
crates/btctax-cli/src/year_readiness.rs       I-1, M-4
crates/btctax-cli/tests/export_irs_pdf.rs     N-1 kill
crates/btctax-cli/tests/slice_from_answers.rs C-1 kill (2×2), M-3 kill, I-1/M-4 kills
crates/btctax-core/src/tax/printed.rs         C-1 (the invariant comment)
crates/btctax-core/src/tax/return_1040.rs     M-3 (full return calls the shared gate)
crates/btctax-core/src/tax/return_refuse.rs   M-3 (the shared gate)
crates/btctax-forms/src/schedule_d.rs         M-1
crates/btctax-forms/tests/kats.rs             I-2
```

## Suite

| run | before | after |
|---|---|---|
| whole workspace | 3178 passed (brief) | **3185 passed, 12 skipped** |
| `-p btctax-forms -p btctax-core -p btctax-cli -p btctax-tui` | — | **2451 passed, 7 skipped** |

`+7`, all new kills, cause-by-cause:

| new test | crate | finding |
|---|---|---|
| `schedule_d::tests::all_boxes_is_every_variant` | forms | M-1 (support) |
| `schedule_d::tests::a_box_in_no_group_refuses_and_the_real_groups_partition_every_box` | forms | M-1 |
| `cmd::admin::slice_broker_tests::the_form_level_gate_runs_on_arm_3_too` | cli | M-2 |
| `slice_from_answers::an_unanswered_restriction_refuses_a_section_b_8283_and_prints_a_section_a_one` | cli | M-3 |
| `slice_from_answers::the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored` | cli | I-1 + M-4 |
| `slice_from_answers::report_does_not_promise_the_slice_on_a_year_with_no_bundled_templates` | cli | I-1 |
| `export_irs_pdf::schedule_d_selected_without_form_8949_is_refused_and_writes_nothing` | cli | N-1 |

C-1 and I-2 add no test — they **widen an existing one** (the sweep and the pre-2025 KAT), which is
what the brief specified.

### Exact commands, with their summary lines

```
$ cargo nextest run --locked -p btctax-forms
     Summary [   5.278s] 368 tests run: 368 passed, 4 skipped          (was 366 — +2, M-1)

$ cargo nextest run --locked -p btctax-cli --no-fail-fast
     Summary [   6.330s] 712 tests run: 712 passed, 1 skipped

$ cargo nextest run --locked -p btctax-core -p btctax-tui --no-fail-fast
     Summary [   1.797s] 1371 tests run: 1371 passed, 2 skipped

$ cargo nextest run --locked -p btctax-forms -p btctax-core -p btctax-cli -p btctax-tui --no-fail-fast
     Summary [  11.390s] 2451 tests run: 2451 passed, 7 skipped

$ cargo nextest run --locked --workspace --no-fail-fast
     Summary [  19.049s] 3185 tests run: 3185 passed, 12 skipped

$ cargo fmt --all                                                       # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 1.49s   # zero warnings
```

---

## C-1 (Critical) — the Form 8283 restriction gate now runs on BOTH slice arms

**Change.** `crates/btctax-cli/src/cmd/admin.rs:809-855` — the gate is hoisted out of the
`if files_from_answers` block and now runs whenever `working` is `Some`, before any byte, on both
arms. The old arm-(2)-only row at the previous `:822-836` is gone (`:873` carries a one-line pointer
in its place). It sits after `regime_or_refuse` and before the arm (2)/(3) branch, so on arm (3) it
now precedes `slice_broker_refusal` — deliberate: both are hard refusals that write nothing, and the
restriction is the one the filer already answered.

`crates/btctax-core/src/tax/printed.rs:200-210` — the invariant comment on
`form_8283_printed`'s `no_donation_restrictions` parameter no longer says *"`Some(true)` cannot
arrive: the year refuses upstream"*. It now names **the one decision and both enforcement sites**
(`return_refuse::donation_restriction_gate`, called by `return_1040::assemble_absolute` and by
`cmd::admin::export_irs_pdf_from_session_with_regime` on both arms), and records what the comment was
asserting while the code did not hold it.

**Kill.** `a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283`
(`slice_from_answers.rs:1022-1111`) is now the 2×2 sweep the review named:
`{draft row, committed row} × {answers stored, no answers}`, on a params-less year with a $20,000
restricted donation. Two changes make it real rather than nominal:

1. **A fresh vault per cell** (`donation_vault()`), because a draft written for one cell shadows the
   committed row of the next under §6.1 precedence — with one vault the sweep silently ran arm (2)
   four times. That mistake was made and caught here: the first run of the sweep failed with
   `committed=true with_answers=false` returning `Ok` because the previous cell's draft was still
   present.
2. **The PRODUCTION path** (`cmd::admin::export_irs_pdf`), not
   `testonly::export_irs_pdf_with_regime(…, LIVE)`. On TY2025's real proceeds-only regime
   `broker_question_is_live` is false, so arm (3) does not refuse and the 8283 gate is the only
   thing between the filer and the overstated form — which is the defect. Under the injected LIVE
   regime R1's own refusal masks it, and the sweep would have gone red on the wrong sentence.

The `Some(false)` twin is the control, and it is cell-aware: on arm (3) it **exports and prints
`form_8283.pdf`** (so a gate that refused every donation year could not pass), and on arm (2) — where
stored answers on a proceeds-only year are unread — it asserts the refusal is *not* the §1.170A-7 one.

**Seen red.** Plant: `if let Some(ri) = working.as_ref().filter(|_| files_from_answers)` — the gate
re-scoped to arm (2), as `fb5e7fc3` had it.

```
thread 'a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1070:26:
a declared restriction must refuse: IrsPdfReport { … regime: InformationReturnRegime { proceeds: true,
basis: false }, … form_8283_path: Some("/tmp/.tmpbPzgjS/slice/form_8283.pdf"),
form_8283_needs_review: true, form_8283_section_b: Some(true), … }
     Summary [   0.373s] 1 test run: 0 passed, 1 failed, 712 skipped
```

That is the review's PROBE 1 reproduced inside the suite: TY2025's real regime, a declared
restriction, `form_8283.pdf` written. Restored from a `cp` backup; whole suite green after.

---

## M-3 (Minor, rides with C-1) — the UNANSWERED row, and the predicate is now shared

**Change.** New in `crates/btctax-core/src/tax/return_refuse.rs:882-925`:

```rust
pub enum DonationRestrictionGate { Declared, UnansweredSectionB }

pub fn donation_restriction_gate(
    answer: Option<bool>, attaches_8283: bool, section_b: bool,
) -> Option<DonationRestrictionGate>
```

`return_1040.rs:2665-2703` (the full return) and `admin.rs:809-855` (the slice) both call it. The
**decision** — which row fires and in which order — is shared; only the **premises** are local,
because the two callers genuinely see different things and the doc comment says so:

| premise | full return | crypto slice |
|---|---|---|
| `attaches_8283` | Schedule A line 12 > $0 | the year EMITS an 8283 (`!rows_8283.is_empty()`) |
| `section_b` | line 12 > $500 **and** year aggregate > $5,000 | the printed rows carry `Form8283Section::B` |

The slice's `section_b` is not a second rule: `forms::form_8283` sets the section from
`year_donation_deduction(state, year) > QUALIFIED_APPRAISAL_THRESHOLD`, the same `donated` term the
full return reads.

**Kill.** `an_unanswered_restriction_refuses_a_section_b_8283_and_prints_a_section_a_one`
(`slice_from_answers.rs:1114-1165`): `donations_had_restrictions = None`, two claimed amounts —
`$20,000` (Section B ⇒ refuses, names SECTION B and §1.170A-7, writes nothing) and `$2,000`
(Section A ⇒ **prints**, because below $5,000 lines 5a/5b/5c are never posed and silence forgoes
nothing).

**Seen red.** Plant: the `UnansweredSectionB` row disabled in the shared gate.

```
thread 'an_unanswered_restriction_refuses_a_section_b_8283_and_prints_a_section_a_one' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1143:18:
an unanswered Section B restriction question must refuse: IrsPdfReport { …
form_8283_path: Some("/tmp/.tmp3MWJ48/slice/form_8283.pdf"), … form_8283_section_b: Some(true), … }
```

**And the same single plant reds the FULL return's own kills** — evidence that the predicate really is
shared rather than duplicated:

```
$ cargo nextest run --locked -p btctax-core --no-fail-fast          # with the plant in place
   FAIL btctax-core tax::return_1040::tests::a_declared_restriction_refuses_at_any_amount_not_just_over_5000
   FAIL btctax-core tax::return_1040::tests::an_itemizer_whose_170b_ceiling_zeroes_the_gift_files_no_8283_and_is_not_blocked
     Summary [   0.533s] 1211 tests run: 1209 passed, 2 failed, 0 skipped
```

**Note on M-3's severity.** The review filed it Minor and suggested owner adjudication rather than a
fix by default; the brief directed the fix, so it is implemented. It is a *widening of a refusal*, so
it fails closed: a year that previously printed a Section-B 8283 with the restriction question
unanswered now refuses and names `btctax income answer` as the exit.

---

## I-1 (Important) — the slice promise is conditioned on the year's templates

**Change.** `crates/btctax-cli/src/year_readiness.rs:166-195`:

```rust
pub fn slice_can_print(year: i32) -> bool {
    btctax_forms::Form8949Map::for_year(year).is_ok()
        && btctax_forms::ScheduleDMap::for_year(year).is_ok()
}

fn slice_clause(year: i32, answers_stored: bool) -> String   // "" when either half fails
```

Three surfaces now take the predicate the export actually applies:

- `cmd/tax.rs::stored_answers_reach_the_slice` (`:475-495`) — third term added.
- `year_readiness::uncomputable_sentence(year, answers_stored)` (`:207-218`).
- `year_readiness::import_note(year, answers_stored)` (`:226-239`).

When the clause does not hold it is **absent**, not replaced with a different promise: the minimal
change the review names, and the divergence only earns a sentence if the wrong outcome is worse than
saying nothing.

**Call sites threaded** (both read the `ReturnInputs` already in hand — no second lookup that could
see a different row):

- `resolve.rs:184-187` — `answers_stored` derived from the `ri` `resolve_core` returned, passed to
  all three `uncomputable_detail` call sites (`:190`, `:201`, `:206`).
- `cmd/tax.rs:203` — `import_note(year, !ri.broker_reporting.0.is_empty())`, on the row being stored.

**Kills.** Two, at both layers.

`the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored` — the three
cells the brief names (TY2026+answers ⇒ absent; TY2025+answers ⇒ present; TY2025+no answers ⇒
absent), for **both** sentences, plus a fourth loop asserting everything else the sentences say
survives in every cell.

`report_does_not_promise_the_slice_on_a_year_with_no_bundled_templates` — the same divergence one
layer up: a TY2026 vault with draft answers, `report_tax_year(2026)` ⇒
`slice_prints_from_answers == false` and the rendered outcome carries no clause, while the answers
block still prints (the clause changed, not the block).

**Seen red.** Plant: `slice_can_print` returns `true` unconditionally (R6's predicate).

```
thread 'the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1336:5:
TY2026 bundles no Form 8949 / Schedule D map — the export refuses, so the sentence must not promise a
slice: tax year 2026 has full-return inputs, but full-return computation is not available for it in this
build — TY2026 — preparing (0 forms; TaxTable yes; full-return params no; 1099-DA proceeds+basis). The
inputs are KEPT and will compute when the year's package is bundled. `export-irs-pdf --tax-year 2026`
still prints the crypto slice from the stored answers. …

thread 'report_does_not_promise_the_slice_on_a_year_with_no_bundled_templates' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1381:5:
TY2026 bundles no Form 8949 / Schedule D map — the slice cannot print, whatever is stored
     Summary [   0.150s] 2 tests run: 0 passed, 2 failed, 711 skipped
```

---

## M-4 (Minor, rides with I-1) — the answers-stored term

**Change.** Carried by the same `slice_clause(year, answers_stored)` above; `uncomputable_sentence`
and `import_note` both gained the parameter. Previously neither carried **any** predicate — not even
the answers half — so a params-less year whose vault held no 1099-DA answers was still told the slice
prints "from the stored answers".

**Seen red.** Plant: `slice_clause` ignores `answers_stored`.

```
thread 'the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1351:5:
no stored answers ⇒ no "from the stored answers" promise: tax year 2025 has full-return inputs, but
full-return computation is not available for it in this build — TY2025 — preparing (17 forms; TaxTable
yes; full-return params no; 1099-DA proceeds). The inputs are KEPT and will compute when the year's
package is bundled. `export-irs-pdf --tax-year 2025` still prints the crypto slice from the stored
answers. …
```

---

## I-2 (Important) — the pre-2025 T8 kill now uses pre-2025 boxes

**Change.** `crates/btctax-forms/tests/kats.rs:694-741`
(`a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3`): the fixture is re-boxed from
`mixed_rows()`'s TY2025 I/L defaults to **C/F**, and the test gained the line-10 assertion so both
halves of the pairing are held. `Form8949Box` added to the file's `btctax_core` import.
`not_reported_by_box` keeps I/L, as the review allowed — it is the *named kill* that moved.

**Seen red — both plants the review reported GREEN.** Run before the M-1 guard existed, so each
red is the value assertion itself rather than a refusal:

```
# PLANT A — `&[B::C, B::I]` → `&[B::I]` (line 3 loses Box C)
thread 'a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3' panicked at kats.rs:723:5:
assertion `left == right` failed: the WHOLE Part I total (Box C) on line 3
  left: None
 right: Some("36000.50")

# PLANT B — `&[B::F, B::L]` → `&[B::L]` (line 10 loses Box F)
thread 'a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3' panicked at kats.rs:728:5:
assertion `left == right` failed: the WHOLE Part II total (Box F) on line 10
  left: None
 right: Some("60000")
```

---

## M-1 (Minor) — the six groups are proved to partition `by_box`

### DEVIATION, and the reason

**The review's premise is wrong about this codebase.** It says `fill_schedule_d_totals` "hand-lists 8
of the 12 `Form8949Box` variants" and that "`A`, `B`, `D`, `E` appear in no group", and its kill is
*"a `by_box` carrying box A reds an implementation that drops it."* `Form8949Box`
(`btctax-core/src/forms.rs:43-62`) has **eight** variants — `C, F, I, L, G, H, J, K`. There is no
`Form8949Box::A`; A/B/D/E are the 1099-B *securities* boxes named in the form's captions, which
btctax never emits and which have no variant to group. The review's kill is therefore unwritable as
stated, and "map A/B/D/E to 1b/2/8b/9" would be adding dead variants to a type.

What the finding is **right** about is the shape of the risk: `schedule_d_by_box` keys on whatever
`row.box_` holds, and a box in no group is summed and then dropped invisibly. So the guarantee is
implemented and the kill is written against the same defect with the boxes that exist.

**Change.** `crates/btctax-forms/src/schedule_d.rs:104-165` and `:204-266`:

- `G_LINE1B / G_LINE2 / G_LINE3 / G_LINE8B / G_LINE9 / G_LINE10` + `BOX_GROUPS` — the six groups
  written **once**, shared by the guard and the fill, so a box cannot be dropped from one and not the
  other. The `A`/`B`/`D`/`E` fact the review's "related nit" asked for is stated **at the grouping**,
  not only in prose above `1b`.
- `partition_or_refuse(by_box, groups, year)` — pure, split out on the `first_unresolved_map` pattern
  so a kill can plant a group list; called at the top of `fill_schedule_d_totals` (`:204`), before a
  single cell is written. A box on **0** groups refuses ("dropped off the filed page"); a box on **2**
  refuses ("printed twice on"). It uses `FormsError::Geometry`, consistent with the neighbouring
  `need()` refusal, rather than adding a variant (whose cross-crate exhaustive-match blast radius is
  not worth a Minor).

**Kill.** `schedule_d::tests::a_box_in_no_group_refuses_and_the_real_groups_partition_every_box` —
three halves: a planted group list missing Box C refuses naming it; a planted list carrying Box C on
two lines refuses naming it; and the **real** `BOX_GROUPS` accept every variant. The third half is
what stops a guard that refuses everything from passing.

"Every variant" is not a hand-list: `ALL_BOXES` is checked by `all_boxes_is_every_variant`, which
maps it through `box_index`'s **exhaustive match** and asserts the sorted indices are `0..8`. A new
`Form8949Box` variant fails to compile until it is given an index; a variant missing from `ALL_BOXES`
shows up as a gap.

**Seen red, twice.**

```
# PLANT 1 — the guard removed (`if true { return Ok(()); }`)
thread 'schedule_d::tests::a_box_in_no_group_refuses_and_the_real_groups_partition_every_box' panicked
at crates/btctax-forms/src/schedule_d.rs:471:14:
a box in no group must REFUSE, never be dropped off the page: ()

# PLANT 2 — the REAL group mutated: `G_LINE3` = `&[Form8949Box::I]`
thread 'schedule_d::tests::a_box_in_no_group_refuses_and_the_real_groups_partition_every_box' panicked
at crates/btctax-forms/src/schedule_d.rs:499:21:
the real BOX_GROUPS must carry C on exactly one line — geometric read-back FAILED (mis-mapped cell):
the TY2024 Schedule D box→line table carries Form 8949 Box C on 0 of its six lines ([]), not exactly
one — this year's rows include that box, so its total would be dropped off the filed page. Fix the box
groups (spec 1099-DA T8).
```

The first attempt at plant 2 red on the *wrong* assertion (the two-line half, which was reusing
`G_LINE3`); the `doubled` fixture now spells its line-3 group out literally so the third half is what
a mutated real group reds. Recorded because the fix is the difference between a kill and a
coincidence.

---

## M-2 (Minor) — the form-level map gate runs on arm (3) too

**Change.** `crates/btctax-cli/src/cmd/admin.rs:779-793` — the call site is now
`if form_level_gate_runs(files_from_answers, tax_year)`, with the predicate at `:2324-2339`:

```rust
fn form_level_gate_runs(files_from_answers: bool, tax_year: i32) -> bool {
    files_from_answers || btctax_forms::SUPPORTED_YEARS.contains(&tax_year)
}
```

### DEVIATION — the `SUPPORTED_YEARS` term, and why the kill is not end-to-end

**(a) Not literally unconditional.** Making the gate run on arm (3) for *every* year changes what a
wholly unported year reports there: TY2023 on arm (3) currently raises the typed
`CliError::FormFill(FormsError::UnsupportedYear(2023))` (pinned by
`export_irs_pdf::unsupported_year_is_refused:508-542`, which also asserts zero bytes), and an
unconditional gate would replace it with a `CliError::Usage` form-level message — a different error
*type*, hence potentially a different exit code, for a question the year-level check already answers.
The design comment at `admin.rs:779-782` states plainly that `SUPPORTED_YEARS` is a **year**-level
answer and the map gate a **form**-level one; reordering them fights that. So the gate is
unconditional on arm (2), where R6 mandates the form-level message even for TY2026, and held behind
`SUPPORTED_YEARS` on arm (3), where the year-level answer already exists. Both still write nothing.

**(b) No end-to-end kill is available.** The brief's kill — *"a partially ported fixture year reached
on arm (3) → refusal, no `out_dir`"* — cannot be written: no bundled year is partially ported
(`every_bundled_slice_year_passes_the_form_level_gate` proves it), and `slice_map_gate` has no
injection seam, which is precisely why R6 split `first_unresolved_map` out as the pure half. The
guarantee is therefore killed in the two halves that *are* testable, and their conjunction is the
guarantee:

- **does the gate run on arm (3)** — `the_form_level_gate_runs_on_arm_3_too` (new), asserting
  `form_level_gate_runs(false, y)` for every `SUPPORTED_YEARS` entry, `true` for arm (2) on 2026/2023,
  and `false` for arm (3) on 2026/2023.
- **what it says when a map is missing** — the existing `first_unresolved_map` kills
  (`admin.rs:2384-2409`), unchanged.

**Seen red.** Plant: `form_level_gate_runs` reduced to `files_from_answers` (R6's condition).

```
thread 'cmd::admin::slice_broker_tests::the_form_level_gate_runs_on_arm_3_too' panicked at
crates/btctax-cli/src/cmd/admin.rs:2495:13:
TY2017 is a bundled slice year: arm (3) must get the form-level gate too
```

---

## N-1 (Nit) — `--forms schedule-d` without `f8949` is refused

**Change.** `crates/btctax-cli/src/cmd/admin.rs:938-956`, beside the existing `--forms full-return`
refusal (both are "your `--forms` selection cannot be honoured"), before `mkdir_out`. The brief said
*pick the refusal (fail closed) and say so* — done; the message quotes the caption that does the
citing and names the fix (`add f8949`, or drop `--forms`).

### DEVIATION — both arms, not arm (2) only

The brief scopes N-1 to arm (2). It is implemented for the whole slice export, on both arms, because
the reason is arm-independent (the Schedule D's captions cite a page-set whatever produced the boxes)
and because an arm-conditional `--forms` rule would mean the same command line is honoured or refused
depending on whether a vault holds answers. Blast radius checked before widening: **no test and no
doc selects that narrowing** — `grep -rn "FormArg::ScheduleD"` outside `cli.rs` hits only the two
production sites in `admin.rs`, and `docs/examples/examples.md:124` /
`xtask/src/examples.rs:705` both use `f8949,schedule-d` together. No help text changed, so no man
page or example golden moved.

**Kill.** `schedule_d_selected_without_form_8949_is_refused_and_writes_nothing` — the refusal fires
and `out_dir` is never created; and the pairing the docs print (`f8949,schedule-d`) still exports
both, so a refusal that fired on any narrowing would not pass.

**Seen red.** Plant: the refusal disabled.

```
thread 'schedule_d_selected_without_form_8949_is_refused_and_writes_nothing' panicked at
crates/btctax-cli/tests/export_irs_pdf.rs:485:6:
called `Result::unwrap_err()` on an `Ok` value: IrsPdfReport { … f8949_path: None,
schedule_d_path: Some("/tmp/.tmpNVRr2b/slice/schedule_d.pdf"), … }
```

---

## Deviations, collected

| # | brief said | landed | why |
|---|---|---|---|
| M-1 | groups must handle A/B/D/E; kill with a `by_box` carrying Box A | partition guard over the **8** variants that exist; kill plants a group list missing Box C | `Form8949Box` has no A/B/D/E — the review miscounted the enum against the form's captions. Same defect class, written against the real type. |
| M-2 | gate "does not depend on `files_from_answers`" | `files_from_answers \|\| SUPPORTED_YEARS.contains(year)` | an unconditional gate replaces a wholly unported year's **typed** `UnsupportedYear` refusal on arm (3) (pinned, with a zero-bytes assertion) with a `Usage` one — a different error type for a question already answered at the year level. |
| M-2 | kill = a partially ported fixture year on arm (3) | pure predicate kill + the existing `first_unresolved_map` kills | no bundled year is partially ported and `slice_map_gate` has no injection seam; R6 split the pure half out for exactly this reason. |
| N-1 | arm (2) | both arms | the reason is arm-independent; no test or doc selects the narrowing; an arm-conditional `--forms` rule would make one command line mean two things. |
| M-3 | (review filed it Minor + "worth an owner adjudication") | implemented as the brief directed | it is a refusal **widening**, so it fails closed; noted here so the owner can still retract it. |
| C-1 kill | "the 2×2 sweep of the existing restriction test" | same, plus a fresh vault per cell and the production path instead of the LIVE-regime injection | with one vault the draft shadows the committed row and the sweep runs arm (2) four times; with LIVE injected, R1's refusal masks the 8283 print and the sweep would red on the wrong sentence. |

## Observations, not folded (out of scope, no action taken)

- **A pre-existing orphaned doc comment.** `admin.rs`'s `slice_broker_refusal` doc block sits four
  functions above its `fn` (verified at `HEAD`: the doc is at `2208`-ish, the `fn` at `2290`), so it
  currently documents `slice_map_gate`. My first placement of `form_level_gate_runs` landed inside
  that block and re-parented it; the function was moved below `slice_map_gate` so the pre-existing
  oddity is left exactly as it was at `8732a805`. Worth a follow-up; not touched here.
- **The full return's Section-B unanswered row has thin coverage.** Under the M-3 plant only two
  `return_1040` tests red, and neither is named for the unanswered row — they cover it incidentally.
  Not a regression and not in the review's scope, but a candidate follow-up.
