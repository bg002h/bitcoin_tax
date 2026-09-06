# REVIEW — the R6 build (spec 1099-DA R6 / FR-62), seam-scoped

Independent adversarial build review. Worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-ac859db39db8157fc`, detached at **`a2f71ea6`**
(HEAD of `main`); the build under review is **`fb5e7fc3`**. Read-only: no source edit survives this
review — every plant was `cp`-backed-up, run, restored and `touch`ed (the implementer's §7 mtime
trap), and `git status --porcelain` is EMPTY at the time of writing.

Contract: `design/SPEC_1099da_broker_reporting.md` **R6** (five spec rounds). The implementer's
account is `design/agent-reports/2026-09-06-build-1099da-R6-implementation.md`; its claims were
treated as claims.

## Commands run, with their summary lines

```
$ export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
$ cargo build --locked --workspace --tests                        # exit 0

# baseline, tree clean, after every plant was restored
$ cargo nextest run --locked -p btctax-cli -p btctax-forms -p btctax-tui -p btctax-core
     Summary [  11.087s] 2444 tests run: 2444 passed, 7 skipped

$ cargo nextest run --locked -p btctax-cli -E 'binary(slice_from_answers)'
     Summary [   0.792s] 17 tests run: 17 passed, 0 skipped

# PLANT A — Box F dropped from Schedule D line 10 (`&[B::F, B::L]` → `&[B::L]`)
$ cargo nextest run --locked -p btctax-forms
     Summary [   5.203s] 366 tests run: 366 passed, 4 skipped        ← GREEN. no kill.
$ cargo nextest run --locked -p btctax-cli -p btctax-core
     Summary [   5.869s] 1918 tests run: 1918 passed, 1 skipped      ← GREEN. no kill.

# PLANT B — Box C dropped from Schedule D line 3 (`&[B::C, B::I]` → `&[B::I]`)
$ cargo nextest run --locked -p btctax-forms -p btctax-cli -p btctax-core
     Summary [  10.964s] 2284 tests run: 2284 passed, 5 skipped      ← GREEN. no kill.

# PLANT C — `return_inputs::get` in `Session::broker_reporting_answers`
$ cargo nextest run --locked -p btctax-cli -p btctax-tui
     Summary [   5.240s] 648/867 tests run: 646 passed, 2 failed     ← RED, correctly.

# PLANT D — the pre-R6 dispatch condition restored
#   (`if params_bundled && return_inputs::exists(…)` → `if return_inputs::exists(…)`)
#   run against reviewer PROBE 1 (below): the probe PASSES, i.e. pre-R6 refused.
```

Three reviewer probes were appended to `crates/btctax-cli/tests/slice_from_answers.rs`, run, and
removed. Their bodies are reproduced inside the findings so each is reconstructible.

---

## Seam 1 — THE DISPATCH

**The decision table, read off `admin.rs:729-760`.** `params_bundled` =
`BundledFullReturnTables::load().full_return_for(year).is_some()` (`:729-735`) — a function call, as
R6 N-2 requires. `working` / `stale_note` = `input_form_store::working_return` (`:755`), called only
after arm (1)'s early `return` at `:751`. `answers_stored` = `working` is `Some` with a non-empty
`broker_reporting` (`:757-759`). `files_from_answers = answers_stored && !params_bundled` (`:760`).

| committed row | params | answers | arm taken | R6 |
|---|---|---|---|---|
| yes | yes | any | (1) full packet, early `return` | ✔ arm 1 |
| any | no | yes | (2) slice from answers | ✔ arm 2 |
| no | yes | yes (draft) | (3), exit names COMMITTING | ✔ the fourth cell, M-13 |
| no | any | no | (3), exit names ANSWERING | ✔ arm 3 |
| **yes** | **no** | **no** | **(3)** | **R6 names no cell — see C-1** |

**Verdict: the four cells R6 names are correct; the fifth is a fail-open.** See C-1.

The controller's added seam — *a COMMITTED `ReturnInputs` row on a params-less year with EMPTY
broker answers* — decided:

- **On the LIVENESS axis it is correctly fail-closed.** Arm (3) calls `slice_broker_refusal`
  (`admin.rs:845-847`), whose first act is `broker_question_is_live(rows, regime)` =
  `regime.basis && rows.iter().any(|r| broker_key(r).is_some())` (`btctax-core/src/forms.rs:130-132`)
  — literally R1's liveness rule. It runs on arm (3) whether or not a committed row exists, so a live
  year with ≥1 exchange disposition and no answers still refuses before any byte. R1 is not violated.
- **On the GATE-CHAIN axis it is fail-OPEN, and that is the real finding.** Falling to arm (3) is
  *not* "the slice as a no-inputs year would print it": a `ReturnInputs` row is now PRESENT and
  UNREAD. Two gates that read that row live inside `if files_from_answers` — the Form 8283 restriction
  row (`admin.rs:827-836`) and the form-level map gate (`admin.rs:783-791`). Before `fb5e7fc3` a
  committed row could never reach arm (3) at all, because the dispatch was
  `if return_inputs::exists(…)` unconditionally. **The kill that should hold it** is a parameterised
  companion to `a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283`, swept over
  {draft row, committed row} × {answers present, answers absent} — all four must refuse. Today only
  {draft, answers present} is covered.

## Seam 2 — T9 and the readers

`working_return` (`input_form_store.rs:242-256`) is a thin wrapper over `load`: `Draft{parked:true}`
→ `None`, `Draft{parked:false}` → `Some`, `Committed` → `Some`, `Fresh` → `None`, and the `StaleNote`
is passed through, so §6.1 precedence and the §6.3 stale split are inherited rather than restated.
`broker_answers` (`:265-270`) is the `.broker_reporting` projection.

Every reader checked and confirmed to route through it: the CLI export (`admin.rs:755`),
`export --csv` (`admin.rs:209-213`), `Session::broker_reporting_answers` (`session.rs:600-616`, over
the union of `return_inputs::years` and `draft_years`), the TUI export and the viewer's Box column
(via `unlock.rs:209` → `snap.broker_answers`), and `report`'s answers block (`cmd/tax.rs:552`). No
production `return_inputs::get` remains on any `broker_reporting` path.

**PLANT C** replaced the `working_return` call in `Session::broker_reporting_answers` — the reader
kill #4 does *not* cover — with `return_inputs::get`. Two tests went red
(`a_draft_shadows_the_committed_row_on_every_surface`,
`ty2025_with_stored_answers_prints_the_slice_from_either_row`).

**Verdict: sound, and killed.** No finding.

## Seam 3 — arm (2)'s gates, before any byte

`mkdir_out(out_dir)` is at `admin.rs:960`. Every gate R6 lists precedes it: promote `:765`, form-level
`:783`, attestation `:800-804`, `screen_broker_reporting` `:810-823`, the 8283 restriction row
`:827-836`, the router `:837`, `price_coverage_or_refuse` `:853`, the two re-worded refusals
`:863-901`, then the Form 8275 overflow pre-checks `:905-946` and the `SUPPORTED_YEARS` refusal
`:954-958`. "Before any byte" holds structurally on both arms.

**Verdict: the ORDER and the placement are correct.** The defect is not that a gate runs late — it is
that two of them are scoped to `files_from_answers` (C-1, M-2).

## Seam 4 — T8

The box→line grouping (`btctax-forms/src/schedule_d.rs:141-207`): 1b ← `[G]`, 2 ← `[H]`,
3 ← `[C, I]`, 8b ← `[J]`, 9 ← `[K]`, 10 ← `[F, L]`. The pairing matches
`printed::schedule_d_lines`, and the `need` refusal (`:95-108`) fires for a bound-less line with an
active group. The unbound-row kill and the CSV-partition kill both behave as the report describes.

**Verdict: the production table is CORRECT; two of its six groups are unkilled.** See I-2 and M-1.

## Seam 5 — `report` in states (2a)/(2b) and the readiness sentences

`stored_answers_reach_the_slice` (`cmd/tax.rs:474-486`) checks both halves R6 names — answers
non-empty AND `full_return_for(year).is_none()` — and the answers block reads the T9 accessor
(`:552`). Exit codes are unchanged. `uncomputable_sentence` (`year_readiness.rs:172-182`) and
`import_note` (`:188-199`) carry the slice clause.

**Verdict: the predicate R6 specifies is implemented exactly, and it is the WRONG predicate.** See
I-1.

## Seam 6 — the kills

Spot-checked by plant rather than by reading the list. PLANT C (seam 2) red, correctly. PLANTS A and
B (seam 4) **green** — two named guarantees have no kill (I-2). PLANT D established that C-1 is a
regression and not pre-existing.

---

## Findings

### C-1 (Critical) — a declared donation restriction prints an overstated Form 8283 on arm (3); R6's dispatch widening made the state reachable

**Where.** `crates/btctax-cli/src/cmd/admin.rs:806` (`if files_from_answers {`) enclosing the Form 8283
restriction row at `:827-836`; the write at `:1047-1054`; the widened dispatch at `:736`.

**What is wrong.** The Form 8283 restriction gate is inside `if files_from_answers` — arm (2) only —
but `form_8283.pdf` is written on **both** slice arms from the same `rows_8283` (`:777`, `:1047`).
Before `fb5e7fc3` the dispatch was `if crate::return_inputs::exists(session.conn(), tax_year)?`, so a
vault holding a committed `ReturnInputs` row **always** went down the full-return arm and, on a
params-less year, refused there. `fb5e7fc3` narrowed that to
`if params_bundled && crate::return_inputs::exists(…)`, so a committed row on a params-less year now
falls through — and when its `broker_reporting` is empty it lands on arm (3), where the row is present
but **unread**. `ri.donations_had_restrictions == Some(true)` is then never consulted and the 8283
prints at full fair market value.

This is not a hypothetical corner. TY2025 is a bundled slice year whose declared regime is
`proceeds = true, basis = false` (`crates/btctax-forms/forms/2025/YEAR.toml:42-43`), so
`broker_question_is_live` is FALSE for it and arm (3) never refuses; and because the 1099-DA question
is not live on TY2025, an empty `[broker_reporting]` is the **normal** shape of a TY2025 import, not
an exotic one. The harm is the one the arm-(2) refusal states in its own words: *"btctax values every
donation at full fair market value — so the Form 8283 it would print for {tax_year} overstates the
gift."* `btctax-core/src/tax/printed.rs:201` records the invariant this breaks verbatim —
*"`Some(true)` cannot arrive: the year refuses upstream."* On arm (3) it does arrive.

**Evidence.** Reviewer PROBE 1, on the pure production path (`cmd::admin::export_irs_pdf`, no regime
injection), TY2025's real proceeds-only regime, a $20,000 donation, a **committed** row with
`donations_had_restrictions = Some(true)` and `answers(&[])`:

```
PROBE result ok=true
PROBE form_8283.pdf exists: true
PROBE wrote: ["f8949.pdf", "form_1040_capgains.pdf", "form_8283.pdf", "schedule_d.pdf"]

thread '…' panicked at crates/btctax-cli/tests/slice_from_answers.rs:1281:
FAIL-OPEN: a declared donation restriction printed an 8283 at full FMV on arm (3)
```

The same probe against **PLANT D** (the pre-R6 dispatch condition) passes — confirming this is a
regression introduced by `fb5e7fc3`, not a pre-existing gap:

```
PROBE result ok=false
PROBE err: usage: no full-return tables for 2025 — the full-return packet needs a supported tax year (TY2024)
PROBE form_8283.pdf exists: false
```

Note that lines 5a/5b/5c are *blank* on the printed form — the slice's `fill_form_8283`
(`btctax-forms/src/form8283.rs:126-132`) passes `None` for the restriction answer. The defect is
therefore not fabricated testimony but an **overstated claimed deduction** on a form the filer signs
and mails, in a state the full return refuses outright.

**Minimal change.** Hoist the restriction row out of the `files_from_answers` block so it runs
whenever a working return exists — it already reads `working`, not the answers:

```rust
// before the `if files_from_answers` block, or in a shared prelude to both arms
if let Some(ri) = working.as_ref() {
    if ri.donations_had_restrictions == Some(true) && !rows_8283.is_empty() {
        return Err(/* the existing §1.170A-7 refusal */);
    }
}
```

…and land it with the 2×2 kill named in seam 1.

---

### I-1 (Important) — the slice promise is not conditioned on the year's templates being bundled: TY2026 is told the slice prints, and the export refuses

**Where.** `crates/btctax-cli/src/cmd/tax.rs:474-486` (`stored_answers_reach_the_slice`);
`crates/btctax-cli/src/year_readiness.rs:172-182` (`uncomputable_sentence`) and `:188-199`
(`import_note`).

**What is wrong.** All three surfaces promise *"`export-irs-pdf --tax-year {y}` still prints the
crypto slice from the stored answers"* on the strength of `full_return_for(year).is_none()` (plus, in
`report`'s case, answers being stored). Neither term is what makes the slice printable. Printing also
requires the year's **form templates** to be bundled, which `slice_map_gate` (`admin.rs:783-791`) and
the `SUPPORTED_YEARS` refusal (`:954-958`) enforce and these sentences do not model.

TY2026 is exactly that year: `crates/btctax-forms/forms/2026/` contains **only `YEAR.toml`** — no
maps — so `full_return_for(2026)` is `None` (the clause fires) while `Form8949Map::for_year(2026)`
fails (the export refuses). This is the headline year of the owner's own ruling (*"we will need to
have option to file 2026 tax year with crypto sales"*), and R6 states the fact plainly — *"TY2026
prints nothing until its Form 8949 and Schedule D FINAL revisions are bundled"* — while the sentence
R6 also mandates contradicts it. Every existing kill for this clause
(`report_in_state_2a_keeps_its_exit_code_and_gains_the_slice_clause`,
`state_2b_is_unchanged_and_both_sentences_name_the_slice`) runs on **TY2025**, where templates *are*
bundled, so the divergence is invisible to the suite.

Two independent falsifiers, in fact: `uncomputable_sentence` and `import_note` carry **no predicate
at all** — not even the answers-stored half — so they also assert "from the stored answers" on a
params-less year whose vault holds none.

**Evidence.** Reviewer PROBE 3, TY2026 vault, draft answers stored, one 2026 disposition:

```
PROBE3 report ok; slice_prints_from_answers=true
PROBE3 export ok=false
PROBE3 export err: usage: cannot export TY2026: this build has no usable `f8949` map for the year
  (unsupported tax year 2026: this build bundles IRS forms for 2017, 2024 and 2025 only), and the
  crypto slice would write that form. […] No forms were written; […]
PROBE3 dir exists: false
```

The refusal itself is clean and fail-closed — nothing is written — so no wrong figure reaches paper.
What is defective is the tool's account of what it will do, on the one year the feature exists for.

**Minimal change.** Give the three sentences the predicate the export actually applies. Add a
templates term to `stored_answers_reach_the_slice`, and make the two readiness sentences take the same
helper rather than asserting unconditionally:

```rust
fn slice_can_print(year: i32) -> bool {
    btctax_forms::Form8949Map::for_year(year).is_ok()
        && btctax_forms::ScheduleDMap::for_year(year).is_ok()
}
```

Kill: the TY2026 half of `report_in_state_2a_…` — same assertions, `2026` for `2025`, expecting the
clause to be ABSENT.

---

### I-2 (Important) — T8's two pre-2025 box halves have no kill, and the test named as the pre-2025 kill exercises the post-2025 box

**Where.** `crates/btctax-forms/src/schedule_d.rs:159` (line 3 ← `[B::C, B::I]`) and `:198` (line 10
← `[B::F, B::L]`); `crates/btctax-forms/tests/kats.rs:698-719`
(`a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3`);
`crates/btctax-forms/tests/common/mod.rs:148-151` (`not_reported_by_box`).

**What is wrong.** The production grouping is correct — C and F are both present. But **neither half
is held by a test.** Deleting `B::C` from line 3, and separately deleting `B::F` from line 10, each
leaves the entire relevant surface green.

The reason is a fixture choice. `not_reported_by_box`, the helper the T8 change introduced for
hand-built totals, hardcodes `Form8949Box::I` and `Form8949Box::L` for **every** year, including 2017
and 2024 — years whose real routing produces **C** and **F**. And the test the implementer's report
names as the pre-2025 kill says so in its own first line:

```rust
fn a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3() {
    let rows = mixed_rows(); // I/L fixtures — line 3 is "Box C **or Box I**" on every revision
    …
    let sd = btctax_forms::fill_schedule_d(&totals, &by_box, 2024).unwrap();
```

The `2024` selects the map *revision*; the rows are Box **I**, a box TY2024 cannot emit. So the test
proves the 2024 map binds line 3 — it does not prove a TY2024 slice's Box C total reaches it. This is
the "green and blind instrument" shape: the checker is green because the case it is named for never
runs.

The consequence if the pairing were ever broken is a wrong result, not a crash: a TY2024 or TY2017
slice with not-reported rows would print Schedule D line 3 (or line 10) **blank** while line 7 / 15 /
16 still carry the total — an internally inconsistent filed schedule whose detail line silently
vanished. TY2024 is a bundled, supported slice year, so the path is live today.

**Evidence.**

```
# PLANT A — `&[B::F, B::L]` → `&[B::L]`
     Summary [   5.203s] 366 tests run: 366 passed, 4 skipped     (btctax-forms)
     Summary [   5.869s] 1918 tests run: 1918 passed, 1 skipped   (btctax-cli + btctax-core)

# PLANT B — `&[B::C, B::I]` → `&[B::I]`
     Summary [  10.964s] 2284 tests run: 2284 passed, 5 skipped   (forms + cli + core)
```

**Minimal change.** Make the pre-2025 KAT use pre-2025 boxes — build its `by_box` from C/F rows
(`schedule_d_by_box` over rows whose `box_` is `C`/`F`) rather than from `mixed_rows()`, and assert
line 3 and line 10 each carry their group. That single edit reds both plants. `not_reported_by_box`
may keep I/L for the golden-comparability reason its doc comment gives; it is the *named kill* that
must move.

---

### M-1 (Minor) — `fill_schedule_d_totals` hand-lists 8 of the 12 `Form8949Box` variants, with no assertion that the groups partition `by_box`

`schedule_d.rs:141-207`. The six groups cover `{G, H, C, I, J, K, F, L}`; `A`, `B`, `D`, `E` appear in
no group. `schedule_d_by_box` (`btctax-core/src/forms.rs:498-507`) keys on whatever `row.box_` holds,
so a box outside the eight would be aggregated and then dropped — invisibly, since a missing Schedule
D line is a normal blank. Unreachable today (`route_8949_boxes` emits only G/H/I/J/K/L and `form_8949`
only C/F/I/L), which is why this is Minor and not Important. The project's own preference is that an
omission not compile: an exhaustive `match` over `Form8949Box` returning the line, or a debug
assertion that the union of the six groups covers `by_box.keys()`, removes the class.

Related nit in the same block: the doc comment at `:113` renders line 1b as *"Box A or Box G"* and
line 2 as *"Box B or Box H"* — the form's true text — while the code groups only `[G]` and `[H]`. The
divergence is justified (A and B are securities boxes btctax never emits) but is not stated at the
grouping, only in prose four lines above `1b`.

### M-2 (Minor) — arm (3) does not run the form-level map gate

`admin.rs:783` scopes `slice_map_gate` to `if files_from_answers`. A partially ported year reached on
arm (3) would therefore not get R6's "write nothing" guarantee. Latent rather than live: the
`SUPPORTED_YEARS` refusal at `:954` catches a wholly unported year (TY2026) before `mkdir_out`, and
`every_bundled_slice_year_passes_the_form_level_gate` shows no bundled year is partially ported. It
becomes reachable the moment TY2026 gets one of its two maps but not the other — which is precisely
the Nov 2026 – Jan 2027 port R6 schedules. Same one-line fix shape as C-1: the gate does not depend on
`files_from_answers`.

### M-3 (Minor) — the UNANSWERED restriction question prints a Section-B 8283 on arm (2)

The full return has a two-part gate (`btctax-core/src/tax/return_1040.rs:2667-2695`): refuse on
`Some(true)` at any amount, **and** refuse on `None` when the year files a Section B 8283
(`claimed_noncash > $500 && donated > $5,000`). R6 specifies, and the build implements, only the first.
Reviewer PROBE 2 (arm (2), draft answers, `donations_had_restrictions = None`, a $20,000 donation)
returned `ok=true` with `form_8283.pdf` written.

Filed as Minor rather than Important on the merits: the slice's `fill_form_8283` passes `None` for the
restriction answer, so lines 5a/5b/5c print **blank** — lawful silence that asserts nothing — and the
slice has no Schedule A on which to claim the deduction, which is the harm the full-return refusal
names. A filer who never told btctax about a restriction is not being misreported by btctax. Worth an
owner adjudication rather than a fix by default.

### M-4 (Minor) — `uncomputable_sentence` / `import_note` assert "from the stored answers" with no answers-stored term

Covered in I-1's evidence; separated here because the fix is independent of the templates term. Both
sentences state the clause unconditionally, so a params-less year whose vault holds **no** 1099-DA
answers is still told the slice prints "from the stored answers". On a live year with exchange rows
that export refuses at `slice_broker_refusal`.

### N-1 (Nit) — D-2's `--forms` narrowing lets a Schedule D cite page-sets the packet does not contain

`--forms schedule-d` on arm (2) probes only the Schedule D map (`slice_map_gate`'s uniform `wants()`),
then writes a `schedule_d.pdf` whose lines 1b/3 are captioned as totals from Form 8949 page-sets that
the directory does not hold. The implementer flags the reading divergence honestly as D-2. The
behaviour is pre-existing on arm (3) and `--forms` is an explicit narrowing the filer asked for, so
this is a documentation-only note, not a change.

---

## What I did not examine

- **The whole workspace suite was not re-run** (the brief states it green at `fb5e7fc3`: 3178 passed).
  Every count above is from a scoped `-p` run.
- **`btctax-tui` beyond the export screen ordering.** I confirmed `Session::broker_reporting_answers`
  is killed and feeds `snap.broker_answers`, but did not drive the TUI export or the viewer's Box
  column interactively; I relied on the implementer's kills #20/#21 for the mkdir ordering.
- **The three-artifact CSV cross-check was read, not plant-tested.** The implementer's kill #3 names
  two tests; I did not revert the CSV writer myself. D-4 (no part-total row) I accepted on its stated
  `SUM()` reasoning.
- **PDF read-back geometry.** I did not open any produced PDF; line-placement claims rest on the
  existing verifier and goldens.
- **`report`'s exit codes** were read from the code and the existing kills, not re-driven through
  `main.rs`.
- **The price-coverage gate, the promote gate, the pseudo gate and the two re-worded refusals** were
  read for placement and order but not individually plant-tested; the implementer's kills #9, #12, #13
  cover them and none is in the seam C-1 opened.
- **D-1** (the TUI commit modal's missing sentence) I accepted as reported; it is a documented
  omission with the fact carried on two other surfaces, and the ≤104-char NOTICE constraint is real.

Counts: C=1 I=2 M=4 N=1
