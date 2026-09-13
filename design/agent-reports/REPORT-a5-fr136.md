# REPORT — A5 / FR-136: a B1 kill whose plant destroyed itself

**Agent:** A5 (opus), worktree at `376d1141`.
**Finding:** FR-136 (port-rehearsal F4) — *a B1 kill-test's planted defect is keyed to a form that
HAPPENS to be absent, so porting that form destroys the kill.*
**Owned files, and the only two touched:** `crates/btctax-forms/tests/year_record.rs`,
`crates/xtask/src/form_delta.rs`.
**Verdict:** closed. Both instruments now plant from a set they DERIVE or an archive they DECLARE, and
both were observed discriminating on a **complete** year/archive — the case the old plants cannot
express at all.

---

## 0. The four numbers

| measure | value |
|---|---|
| `make gate` (touch every `.rs`, then `nextest --workspace --no-fail-fast` + `clippy --workspace --all-targets --all-features -D warnings`) | **3614 tests run, 3613 passed, 1 failed, 12 skipped**; clippy CLEAN |
| the one failure | `xtask::harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` — **FR-147**, measured identically at BASELINE before any edit of mine (xtask 185/186) |
| `cargo fmt --all --check` | exit **0** |
| planted defects the new `year_record` kill asserts, per run | **177** on the committed tree (60 TY2024 + 54 TY2025 + 63 synthetic complete year); **183** once `f8995a/2025` and `f1040s1/2025` are supplied |

---

## 1. What changed, and why that mechanism

### 1a. `crates/btctax-forms/tests/year_record.rs`

`a_phantom_expected_form_and_an_undeclared_bundled_form_are_both_reported` (committed lines 140-164)
planted the literal `"f8995a"` into TY2025's `forms_expected` and pushed the literal `"f1040s1"` onto
the measured present list. Both were defects **only because those two forms are not bundled for
TY2025 today**. The test now builds its cases from the build itself and plants three ways per stem:

| plant | the edit | the message it must produce, EXACTLY and ALONE |
|---|---|---|
| **phantom** | drop the victim from the measured `present` | *STEM is expected but not bundled* |
| **undeclared** | drop the victim from `forms_expected` | *STEM is bundled but not expected (and not declared absent either)* |
| **contradiction** | MOVE the victim from `forms_expected` into `forms_absent` | *STEM is bundled but not expected (and declared ABSENT — a contradiction)* |

Mechanism, in the terms of `CLAUDE.md`'s *derive the list*:

1. **The victim is drawn from the measured set**, not named. Every stem in `present_for(year)` is
   planted in all three ways, for every bundled year with a non-empty glob. A year that GAINS a form
   gains three plants; it can no longer lose one.
2. **Each plant removes something that is there**, rather than naming something that is not. That is
   what makes it independent of how complete the year is — the report's proposal, extended (see §4).
3. **A synthetic COMPLETE year** is appended: `Stem::ALL` expected, `Stem::ALL` present, `forms_absent`
   empty, and its own `partition_problems()` asserted empty so the case is a legitimate declaration.
   No bundled year can supply this — TY2024 still declares `f1040s1a` absent, TY2025 cannot bundle the
   periodic `f8275`, TY2026 bundles nothing — and it is what a January port sequence converges on.
4. **Misses are collected, not panicked on**, so one run names EVERY case that stopped discriminating
   instead of the first. That is what made the complete-year evidence in §3 readable.
5. **Guard the guard:** at least one real bundled year must have been planted (the synthetic case alone
   would make the test independent of the build), and `plants == 3 * victims`.
6. **Exactly one message** is required per plant, not "contains somewhere". This is strictly stronger
   than the old assertion and it caught a mutation the old test could not (§3d, mutation M-3).

★ The blind spot is stated in the doc comment rather than left to be found: these plants mutate a
**parsed** `YearRecord`, so they say nothing about the text-to-record path. That path is planted by
`a_form_dropped_from_the_absent_list_is_the_third_state_and_is_reported` (a text edit that must still
parse) and refused by `a_mistyped_record_is_refused`.

### 1b. `crates/xtask/src/form_delta.rs`

The coordinator's citation resolves: the committed file asserts *"the plant assumes f8995a has NO 2017
prior side and a 2026 draft"* at line **703** (the assertion spans 700-704, its `compute` companion
705-708, the plant call 709-712 — the cited 698-710 is that block). Two more of the same shape sit at
**747** (*"the plant below assumes f8995a has a draft and no final"*) and **778** (*"the plant assumes
no TY2026 final for f6251"*).

The fix makes the archive a **parameter of the checker instead of a call inside it**:

- new `real_archive(stem)` — the filesystem oracle, carrying the r3 R3 rationale that moved with it;
- `check_work_list_against(doc, prior_tag, new_tag, archived)` holds the body;
- `check_work_list_with` and `check_work_list_tagged` delegate with `real_archive`, so the
  committed-document checks read the real archive exactly as before;
- five plants now DECLARE their archive with a one-file predicate and a synthetic stem:
  `zzz-drafted` (its archive holds only `zzz-drafted--2026-DRAFT`: a draft, no final, no prior side)
  and `zzz-with-prior` (its archive holds only `zzz-with-prior--2025`: a prior side, no final).

Nothing the plants proved was dropped — the stem changed, not the arm:

| plant (and its origin) | before | after |
|---|---|---|
| `claims_no_new` isolation (r3 R2) | `f8995a`, tags 2017 / 2026-DRAFT, real archive | `zzz-drafted`, tags 2025 / 2026-DRAFT, declared archive |
| NO FINAL is TRUE with a draft on disk (r2 N1) | `f8995a`, 2017 / 2026 | `zzz-drafted`, 2025 / 2026 |
| NO DRAFT is FALSE — the draft/final isolation | `f8995a`, 2017 / 2026-DRAFT | `zzz-drafted`, 2025 / 2026-DRAFT |
| NO FINAL load-bearing, prior side PRESENT (r3 N4) | `f6251`, 2025 / 2026 and 2025 / 2026-DRAFT | `zzz-with-prior`, same tags |
| the tags line is load-bearing end to end (r4 N6) | `f8995a` through `check_work_list` | `zzz-drafted` through `check_work_list_tagged` |
| the two `zzz-not-a-form` controls, and the `f1040` / `f6251` / `f1040s1` numeric and prior-side plants | real archive | **unchanged**, real archive |
| **NEW** — the real oracle must DISCRIMINATE | — | true for every (stem, year) in `BUNDLED` (derived, never a typed pair), false for `zzz-not-a-form--2025` |

That last row is the debt a declared archive incurs: if the plants stop reading the filesystem, a
`real_archive` stuck at one answer would make every excused row vacuous. Both directions of the new
guard are structural rather than accidental — every bundled (stem, year) has a COMMITTED template
under `crates/btctax-forms/forms/<year>/` which `pdf_for` resolves before the archive, and
`zzz-not-a-form` is a stem no IRS form has and no archive can hold. §3e shows it is the ONLY thing in
the suite that reds when `pdf_for` stops resolving bundled templates.

**No product behaviour changed.** Every `form_delta.rs` edit is inside `mod tests`; `year_record.rs` is
a test file. The only non-test line touched anywhere was a mutation, restored by `cp` and verified by
checksum (§6).

---

## 2. Demonstration 1 — the CURRENT plants, and what a port does to them

### 2a. `year_record`: supply `f8995a/2025` and the kill returns nothing

`f8995a/2025` was supplied in the worktree the way a port supplies it: the archived TY2025 PDF copied
to `crates/btctax-forms/forms/2025/f8995a.pdf`, a map beside it, `"f8995a"` added to TY2025's
`forms_expected` and deleted from the `forms_absent` table (FR-149's step), and the anti-shrink count
moved 18 to 19. `BUNDLED` then carries the (F8995a, 2025) pair, which is the only thing this test reads.

    thread 'a_phantom_expected_form_and_an_undeclared_bundled_form_are_both_reported' panicked at
    crates/btctax-forms/tests/year_record.rs:152:5:
    []

    Summary [0.072s] 11 tests run: 10 passed, 1 failed, 0 skipped

Every other test in the binary passed, so the port is otherwise consistent — the ONLY casualty is the
kill. This is the rehearsal's F4 reproduced to the character: *"the test reds with `[]`."*

### 2b. The second direction goes the same way

`f1040s1/2025` was supplied the same way (TY2025 then has 20 expected, and `forms_absent` holds `f8275`
alone). Running the old test's second half in isolation:

    glob_problems after pushing f1040s1 = []

    thread '...' panicked at crates/btctax-forms/tests/year_record.rs:469:5: []

So **both** halves of the committed kill are borrowed absences, not one. And TY2025 cannot be driven to
literal completeness at all — `f8275` is periodic by design and is served from `forms/2024/` — which is
why the complete-year case in §3c had to be synthesised.

### 2c. `form_delta`: archive the January finals and three plants invert

Copying a TY2026 **final** for `f8995a` and `f6251` into `design/forms/2026/` — precisely what January
does — produced, in order, with each preceding assertion neutralised in turn to reach the next:

    panicked at crates/xtask/src/form_delta.rs:744:9:
      the plant below assumes f8995a has a draft and no final       [the PREMISE is now false]

    DEMO A5 -- plant@751 wrong=["f8995a: excused as prior=**NO PRIOR SIDE** /
      TY2026=**NO FINAL** planted, but on disk prior=false draft=true"] excused=[]

    DEMO A5 -- premise f6251--2026 is_none = false

    DEMO A5 -- plant@776 wrong=["f6251: excused as having no pair, but form-delta computes one"]
      excused=[]

    panicked at crates/xtask/src/form_delta.rs:799:9:
      declared final tag: ["f8995a: excused as prior=..., but on disk prior=false draft=true"]

(The two `wrong=` lines are verbatim except that the runner's backslash-escaped quotes around the cell
text are unescaped here for legibility.)

Three plants that asserted *excused* now report *wrong*, and `f6251`'s leaves the excused arm
altogether because the pair finally computes. ★ The tell that this is a class and not an accident is in
the committed source: the `f8995a` plants had ALREADY been rescued once for exactly this reason —
*"★ 2026-09-06 (residue sweep 1, item 6) — the PRIOR TAG here is `2017`, not the document's `2025` …
archiving `f8995a--2025` as an authority gave it a prior side"*. That repair was to borrow a different
absence, and it bought nine days.

---

## 3. Demonstrations 2 and 3 — the NEW plants, seen RED (B1)

### 3a. Committed tree, unported

    Summary [0.079s] 11 tests run: 11 passed, 0 skipped       (-p btctax-forms --test year_record)
    Summary [0.555s] 12 tests run: 12 passed, 175 skipped     (-p xtask, test filter form_delta)

### 3b. With `f8995a/2025` supplied — the case that killed the old plant

    Starting 1 test across 1 binary (10 tests skipped)
        PASS [0.003s] (1/1) btctax-forms::year_record
          a_phantom_expected_form_and_an_undeclared_bundled_form_are_both_reported

and with the January finals for `f8995a` and `f6251` archived:

    PASS [0.108s] (1/1) xtask::bin/xtask
      form_delta::tests::the_work_list_checker_reds_on_every_planted_row

### 3c. On a COMPLETE archive — the case the old plants cannot express

Every stem with a TY2025 side was given a TY2026 **final** (39 created; `design/forms/2026/` then held
**50** finals beside its 15 drafts), so no stem anywhere lacks a final and there is nothing left for a
borrowed-absence plant to be re-pointed at:

    PASS [0.108s] (1/1) xtask::bin/xtask
      form_delta::tests::the_work_list_checker_reds_on_every_planted_row

For `year_record` the complete case is a first-class element of the test rather than a scenario, and
§3d shows it discriminating.

### 3d. The `year_record` kills, observed RED on planted defects

`glob_problems` mutated three ways (each restored by `cp`, checksum-verified):

| mutation | new test | old test |
|---|---|---|
| **M-1** return no problems at all | **RED — 177 of 177 plants unreported**, listing 60 TY2024 + 54 TY2025 + **63 on the synthetic COMPLETE year** | red |
| **M-2** drop the bundled-but-not-expected direction | **RED — 118 of 177** (0 phantom, 59 undeclared, 59 contradiction) | red |
| **M-3** collapse *declared ABSENT* into *not declared absent either* | **RED — 59 of 177**, all contradiction plants | **GREEN** — it asserted only that a message contains *"… is bundled but not expected"*, which the wrong arm still satisfies |

M-1 run under the supplied `f8995a/2025` reported **183 of 183** and named the complete-year case
throughout, e.g.

    a synthetic COMPLETE year (every form expected AND bundled) / contradiction: glob_problems must
      report exactly "schedule_se is bundled but not expected (and declared ABSENT — a contradiction)",
      said []

M-3's diagnostic is the reason the exactly-one-message form was chosen:

    TY2024 / contradiction: glob_problems must report exactly "f1040 is bundled but not expected
      (and declared ABSENT — a contradiction)", said [
        "2024: f1040 is bundled but not expected (and not declared absent either)",

### 3e. The `form_delta` kills, observed RED — including one nothing else catches

All four mutations were applied with the COMPLETE archive of §3c in place, i.e. with no real absence
anywhere for a plant to lean on:

| mutation in `check_work_list_against` or `pdf_for` | result |
|---|---|
| **M1** drop the draft-in-archive conjunct | RED at the `zzz-drafted` NO DRAFT plant: *a NO DRAFT claim with the draft in the archive: [] left: 0 right: 1* |
| **M2** `claim_word_matches_tag` always true | RED at the `zzz-with-prior` plant: *NO FINAL under a draft tag names the wrong word: [] left: 0 right: 1* |
| **M3** `real_archive` always false | RED at a PRE-EXISTING real-archive plant (*a NO PRIOR SIDE claim with the 2025 fixture on disk*) — so the real oracle was already under a kill in that direction, and the new guard is belt-and-braces there |
| **M4** `pdf_for` stops resolving BUNDLED templates | RED **only** at the new guard: *the real archive oracle must see every bundled template*. Eleven of twelve `form_delta` tests passed; this is coverage that did not exist before |

---

## 4. Refuted, sharpened, and residual

**R1 — the prescribed fix is incomplete, and was implemented wider.** FR-136 and the rehearsal's F4
both say: *"plant by REMOVING a stem from the measured `present` list."* That fixes the **phantom**
direction only. Removing a stem from the glob can never produce *"bundled but not expected"* — that
half has to be planted on the DECLARATION side. The implemented fix therefore plants on both sides
(three plants per stem, §1a), and the *"not declared absent either"* arm — which the committed test
never reached, because `f1040s1` is declared absent for TY2025 — is now exercised too.

**R2 — "the same shape" is right, but the two failure modes differ, and the difference belongs in the
harness wording.** `year_record`'s plant quietly stops being a defect and the test then reds with `[]`
— a false alarm ABOUT THE PLANT, whose cheapest discharge is deletion. `form_delta`'s plants assert
their premises out loud, so they fail with a sentence naming the assumption. The loud form is strictly
better and should be the minimum bar — but it is **not a fix**, because when the set is complete the
loud failure still has no repair. That is the sentence a harness rule needs to carry.

**R3 — `f8995a--2017` is NOT absent "by construction".** The committed comment argues TY2017 was
dropped whole (S9) so the absence is structural. It is structural only against *today's* archive
policy: nothing prevents a TY2017 authority from being archived, and the comment's own history shows
the tag was moved from 2025 to 2017 precisely because a policy changed underneath it. Declared, not
borrowed, is the only version of that argument that holds.

**R4 — confirmed, not refuted.** F4's *"after this port the supply of usable stems for TY2025 is two
(`f1040s1`, `f8275`)"* checks out: TY2025 declares exactly `f1040s1`, `f8275` and `f8995a` absent, so
porting `f8995a` leaves two — and one of those two was already the other direction's plant.

**Residual, same class, NOT fixed — and why.** Two plants in
`the_work_list_checker_reds_on_every_planted_row` still borrow an absence, and both borrow one that a
*repair elsewhere* destroys rather than a port:

1. the `f1040` numeric plant — *"a numeric row whose pair does not exist"*. It errs because **no
   `f1040--2026-DRAFT` was ever archived** (measured: zero `f1040--` files in `design/forms/2026/`, and
   `crates/btctax-forms/forms/2026/` holds only `YEAR.toml`). Archive that draft and the row computes a
   pair, so the plant reds for the wrong reason.
2. the `f1040s1` numeric plant — *"0 moved printed for a pair the reader cannot witness"*. It needs
   `label_compared == 0`, which holds because that draft's label set cannot be read (**FR-58**, recorded
   in `crates/btctax-forms/forms/2026/YEAR.toml` line 20). Fix FR-58 and the plant inverts.

Making these two declared would mean injecting `compute` — the delta computation itself — the way the
archive oracle was injected. I did **not** do it: `compute` reads real PDFs and produces the field and
label sets the numeric cells are checked against, so a declared `compute` would decouple these plants
from the artefact they exist to measure, and that trade is worse than the gap. The honest move is a
follow-up: **state each borrowed premise in an assertion with a message** (the loud form of R2), so the
day it is repaired the test says which assumption moved instead of which cell is off by one. Recommend
filing that against FR-58's owning phase and the port machine — one assertion each.

**Nothing was blocked for want of a file.** Both fixes fitted inside my two owned files. One file I do
not own was touched as a *mutation only* (`crates/btctax-forms/src/year_record.rs`, for the B1 reds in
§3d); it is byte-identical to HEAD, verified in §6.

---

## 5. Should `design/HARNESS.md` name this shape? Yes — as B1b. Proposed wording

It should, for the reason B1a exists: B1 as written constrains the *checker* and says nothing about the
*plant*, and this repo has now shipped five plants that satisfied B1 on the day they were written and
were disarmed later by unrelated, CORRECT work. B1a's own diagnosis — *"the thing that decides was not
the thing that knows"* — applies one level up. I have **not** edited `design/HARNESS.md`; it is not my
file. Proposed insertion immediately after B1a, in B1a's own amendment shape:

> ##### B1b — the PLANT is the other half of the kill (amended 2026-09-12, FR-136)
>
> **A plant may not borrow an accidental absence. Its victim is DERIVED from the measured set, or its
> premise is DECLARED in the test — never named because it happens to be missing today.** And the plant
> must be **expressible when the set is COMPLETE**: if it could not be written on the day the last gap
> is filled, its only future is deletion.
>
> B1a says a derived checker must not be fed a hand-written fixture. B1b says a derived checker must not
> be planted with a hand-picked *gap*. Same disease, opposite end of the instrument: there, the fixture
> silently decided what the checker could see; here, the repo's own progress silently decides whether
> the plant is still a defect.
>
> ★★ **The plant's failure mode is the trap.** A borrowed-absence plant does not go green — it goes
> RED, with a message about the instrument rather than the code, at exactly the moment someone is
> shipping the port that broke it. The cheapest discharge available then is to delete the test, and when
> the set is complete it is the ONLY one. So a loud premise assertion is the minimum bar and not the
> fix: *"the plant assumes f8995a has a draft and no final"* is a good failure message and still a dead
> end.
>
> **Measured (FR-136, 2026-09-12).** `year_record`'s glob kill planted the literal `"f8995a"` into
> TY2025's `forms_expected` and pushed the literal `"f1040s1"` onto the measured present list; supplying
> both forms — which is all a port is — made `glob_problems` return `[]` for each half, and the test
> failed with the bare message `[]`. `form_delta`'s work-list plants named `f8995a` under the prior tag
> 2017 and asserted that no `f8995a--2026` PDF exists; copying one TY2026 final in falsified the premise
> and inverted three plants. **★ Every one of those five plants was correct when written, and the
> `f8995a` ones had already been rescued once** — the committed comment records the 2026-09-06 move from
> tag 2025 to 2017 after `f8995a--2025` was archived. A plant that needs rescuing on a schedule is a
> plant with no premise of its own.
>
> **The two forms that hold.** Derive the victim: remove a member of the list the build measured
> (`year_record` now plants every stem in `present_for(year)` three ways, and appends a synthetic
> COMPLETE year that no bundled year can supply). Or declare the premise: a synthetic stem plus an
> injected oracle (`form_delta`'s `check_work_list_against(.., archived)`, whose plants state their
> archive in one line). ★ A declared premise incurs one debt — the REAL oracle must then be shown to
> discriminate — and that guard should itself be derived: `form_delta`'s is true for every (stem, year)
> in `BUNDLED` and false for a stem no archive can hold, and it is the only test in the suite that reds
> when `pdf_for` stops resolving bundled templates.
>
> ★ Where neither form is reachable — a plant that needs a real artefact's real content — the honest
> version is the B1a escape hatch: assert the borrowed premise with a message that names it, and record
> what will break it. Two such plants remain in `form_delta` (an unarchived `f1040--2026-DRAFT`, and
> FR-58's unreadable label set); they are listed rather than silently carried.

---

## 6. Demonstration artefacts — none is in the change set

A porcelain status at the end of this work, verbatim:

     M crates/btctax-forms/tests/year_record.rs
     M crates/xtask/src/form_delta.rs

Confirmed, item by item:

| artefact | state |
|---|---|
| `crates/btctax-forms/forms/2025/f8995a.pdf` and its map, and `f1040s1.pdf` and its map | **deleted**; a porcelain status including ignored files over `crates/btctax-forms/forms` prints nothing |
| `crates/btctax-forms/forms/2025/YEAR.toml` (the expected/absent port edits) | **restored from a cp backup**; not in the status above |
| the temporary `demo_direction_two_of_the_old_plant_after_the_port` test and the demo `expected_count` edits | **removed** with the whole file, restored from a cp backup before the real fix was written |
| `crates/btctax-forms/src/year_record.rs` (three `glob_problems` mutations) | **restored**; md5 `74fe9cb1901c6163ec9621063bcd40d7`, equal to the pre-mutation checksum |
| `crates/xtask/src/form_delta.rs` (four mutations plus premise-neutralising demo edits) | **restored** from the fixed-version cp backup; md5 `2d23331e101af39c7a9c29c4f32b8663` on both |
| the copied authority PDFs under `design/forms/2019`, `2022`, `2024`, `2025`, `2026` | still present in the worktree and **ignored** by the `design/forms/**/*.pdf` rule, so outside the change set. They are byte copies of the main tree's, needed because an isolated worktree carries none, and the per-directory counts match the main tree exactly (2 / 4 / 100 / 96 / 48). The 41 simulated TY2026 finals of §3c were deleted and those directories re-copied from the main tree |

The mandated `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-a5` was used for every run; nothing was
built under `/tmp`. No commit, push, stash, checkout or revert; every restore was a `cp`. No subagents.
