# A3 — FR-138 and FR-150, burned down

**Agent:** A3 (opus). **Worktree:** `376d1141` (`.claude/worktrees/agent-a0ba27530fdf2bddf`).
**Owned files, and the only two touched:** `crates/xtask/src/cite_check.rs`,
`crates/xtask/src/archive_check.rs`. **Date:** 2026-09-12.
**`CARGO_TARGET_DIR`:** `/scratch/code/bitcoin_tax/target-a3` (clippy: `target-a3-clippy`).

Both findings are closed. **FR-138's headline number: the excuse list went from 31 pairs to 1, and not
one archive was created to do it** — 37 of the 38 emitted `(form, year)` pairs were *already* archived,
extracted, hashed and manifested, and the ratchet could not see a single one of them.

---

## 1. FR-138 — what changed, where, and why that mechanism

### The defect, restated from measurement

`archived_form_years()` counted a `(form, year)` as archived only if the hand-written
`cite_check::FORMS` const had a row for it **and** a duplicate extract pair existed under
`crates/btctax-core/src/tax/fixtures/<stem>_{form,instructions}.txt`. `FORMS` held **7** rows (measured
from the pre-change file, not counted by eye). Everything else had to be excused, so
`AUTHORITY_NOT_YET_ARCHIVED` carried **31** bare `(form, years)` pairs — with no reason attached to any
of them — and the only cheap way to keep the ratchet green when a document *was* archived was to write
`"not-yet-archived"` about it.

### The fix: one notion of "archived", read off the two conventions the repo already uses

`archived_form_years()` now joins the **year-package table's rows** (`crates/btctax-forms/forms/<year>/<stem>.map.toml`,
design r2 §4) to **`design/forms/MANIFEST.json`**. A pair is archived when, for the
row's own PDF **and** its instructions' PDF *for that year*:

1. there is a manifest entry (that is what carries the URL, sha256 and byte count — the provenance that
   makes a file an archive rather than a text file someone typed);
2. the entry is IRS-final, not a draft (`Entry::is_authority()`);
3. the entry records its text layer at the conventional path
   `design/forms/extract/<stem>--<year>.txt`; and
4. that file is on disk.

Why *these* inputs: `btctax_forms::MapRow`'s own doc comment already says the extract paths are
*"derived by convention, never stored"*, and `design/forms/extract/` is the surface every consumer reads
(`xtask line-coverage`, `census_join`, `btctax-forms/tests/map_rows.rs`). The `btctax-core` fixtures root
the old join required is the **second** root design r2 §9 plans to retire. The join needs no PDF bytes,
so it runs in an isolated worktree where all 125 archived PDFs are absent.

**Measured corroboration that this is the right notion, not merely a looser one.** Joining every row
this way reproduces the *declared* excuse set exactly — **37 of 38 archived, 1 not** — and the one
exception is `f8283--2024`, the single row whose own map header has said so all along:

    authority = "not-yet-archived: f8283--2024 (Rev. 12-2023) has no design/forms/2024/ note or
                 manifest entry; only the extract exists"

Zero extra, zero missing. Two independently-authored artifacts (the map headers and the manifest) agree
on the boundary.

### The excuse list: derived, not typed

`AUTHORITY_NOT_YET_ARCHIVED` is **deleted**. `excused_form_years()` now reads each row's
`authority = "not-yet-archived: <reason>"` header — design r2 §9's stated destination — and **refuses**
any other spelling or an empty reason. Three consequences, all improvements:

- the reason travels with the row it excuses, where a porter is looking. The 31 pairs it replaced
  carried **no reason at all**;
- a new tax year cannot be pre-excused in bulk. There is no wildcard and none is expressible, because an
  excuse is a key on one map file;
- `phantom_excuses` (excused but not emitted) is now **unreachable from real data by construction** — an
  excuse comes from a `.map.toml` and `emitted_form_years()` refuses a map with no blank beside it. Its
  kill moved to planted sets, where it already lived.

**Was deleting the const a weakening?** No, and this is the one judgment call in the change, so here is
the argument in full. The const's value was that *loosening the ratchet took a second, visible edit*.
That property is preserved, at a different pair of sites: `btctax-forms/tests/map_rows.rs::EXCUSED` is a
hand-typed, shrink-only declaration that pins the header excuse set **exactly, in both directions**, and
`map_rows.rs` separately refuses any `authority` value not spelled `not-yet-archived: `. So adding an
excuse still reds until a human edits a declaration. What is gone is a *third* copy of one set — and
that copy was the one that had 30 false entries in it. Keeping it as a cross-checked declaration was
considered and rejected: three declarations of one set is the shape `CLAUDE.md` forbids, and the
cross-check would have bought nothing `EXCUSED` does not already buy.

### Supporting changes in the same file

- **`map_rows()`** — a non-`cfg(test)` reader of the year-package table, because the ratchet also runs as
  a command and a coverage line that only reports is an instrument that cannot fail. The test module's
  old private `rows()` walk (a second reader of the same glob, with its own `unwrap`s) now delegates to
  it: one reader, so a checker cannot read something the live path does not. It **refuses** a row whose
  `year` key disagrees with its directory (every archive path is keyed on the year) and refuses a walk
  that finds no rows or no `f1040`.
- **`re_excusing_an_archived_pair_reds_the_ratchet`** — its plant enumerated from `FORMS` (7 of 38 pairs)
  and asserted `planted == 7`, a literal. It now enumerates from the archived set itself and asserts
  `planted == archived.len()` plus the exact partition `archived + excused == emitted` — written as
  addition, never `usize` subtraction, so a set-relation defect stays an assertion rather than becoming
  an overflow panic. Coverage of that kill went **7 → 37 pairs**, with no literal.
- **`FORMS`'s doc comment** now says plainly that it is no longer the ratchet's input — it survives only
  as the driver for `extract-schedule-1a`, which regenerates the root §9 retires.
- **`archive_check.rs`** — its `KNOWN_ARCHIVES` doc cited `AUTHORITY_NOT_YET_ARCHIVED` as the in-repo
  model for a ratchet. Re-pointed at `cite_check::excused_form_years`, plus the cautionary half, which
  belongs beside a ratchet: *an entry that is cheaper to write than the fix is how a gate turns into the
  noise it was built to replace.*
- **The `unaccounted` message** now tells an operator to archive the **pair** and extract both text
  layers, instead of naming `extract-schedule-1a` and the const. Each `problems_for` arm names the
  document and what is wrong with it.

### One thing I deliberately did NOT claim

The new doc states the boundary in the source: *"archived and extracted" means the authority exists to
check a transcription against — it does **not** mean any instrument has checked one.* Conflating the
precondition with the verification would turn this into a false completeness claim over 37 pairs, which
would be a worse defect than the one I was sent to fix.

---

## 2. FR-138's kills — red, then green

Four mutations, each reverted from a `cp` backup of the green file. Every red below was captured from a
real run, not reconstructed.

### Kill 1a — a genuinely archived form-year is ACCEPTED, with nobody writing "not-yet-archived"

New test `cite_check::tests::an_archived_form_year_is_accepted_and_an_unarchived_one_still_reds`.
It takes a **real committed map row** and re-points it at another year — exactly the edit runbook step 21
makes — with `row.authority = None`, so no excuse exists anywhere. Plant (a) is Schedule 1 ported to
TY2025; `f1040s1--2025` and `i1040gi--2025` are both manifested with their extracts on disk.

**Mutation M1 — restore the OLD notion** (require a `FORMS` row with an `extract_stem`):

    FAIL  cite_check::tests::an_archived_form_year_is_accepted_and_an_unarchived_one_still_reds
    panicked at crates/xtask/src/cite_check.rs:1494:9:
    assertion `left == right` failed: f1040s1--2025 and i1040gi--2025 are both manifested with their
    text layers committed, so a TY2025 Schedule 1 row is ARCHIVED — the old join could not say so for
    any form without a FORMS row and a second extract pair, and the only cheap discharge was a false
    sentence
      left: ["f1040s1--2025: no FORMS row with an extract_stem"]
     right: []

The same mutation reds the live ratchet, and this is the measurement that states FR-138's severity:

    FAIL  cite_check::tests::authority_coverage_may_only_improve
    panicked at crates/xtask/src/cite_check.rs:1319:9:
    30 map row(s) carry no `authority` header — so they assert their primary source is archived — and
    the manifest join does not back that claim:
      forms/2024/f1040: f1040--2024: no FORMS row with an extract_stem
      forms/2024/f1040s1: f1040s1--2024: no FORMS row with an extract_stem
      forms/2024/f1040s2: f1040s2--2024: no FORMS row with an extract_stem
      forms/2024/f1040s3: f1040s3--2024: no FORMS row with an extract_stem
      forms/2024/f1040sa: f1040sa--2024: no FORMS row with an extract_stem
      forms/2024/f1040sb: f1040sb--2024: no FORMS row with an extract_stem
      forms/2024/f1040sc: f1040sc--2024: no FORMS row with an extract_stem
      forms/2024/f6251: f6251--2024: no FORMS row with an extract_stem
      ... (30 rows in all; the full list is section 4's RETIRED table)
    Summary  16 tests run: 13 passed, 3 failed

**30 rows.** That is the number of false sentences the old gate's cheapest discharge required.

GREEN (reverted): `16 tests run: 16 passed, 174 skipped`.

### Kill 1b — a genuinely UNarchived form-year still reds, by name, on the right document

Plant (b) in the same test is the rehearsal's own case: Form 8995-A ported to TY2025.
**Measured in this tree:** `f8995a--2025.pdf` IS a manifest entry with its extract; `i8995a--2025` has
**no manifest entry and no extract**. So the port is genuinely not fully archived, and the ratchet must
say so — and say *which* document.

**Mutation M2 — look only at the form, never its instructions:**

    FAIL  cite_check::tests::an_archived_form_year_is_accepted_and_an_unarchived_one_still_reds
    panicked at crates/xtask/src/cite_check.rs:1495:9:
    assertion `left == right` failed: exactly one of the two documents is missing; got []
      left: 0
     right: 1
    Summary  16 tests run: 15 passed, 1 failed

The gate stopped complaining and the kill caught it. The green assertions also pin that the message
names `design/forms/2025/i8995a--2025.pdf` and does **not** blame `f8995a--2025.pdf`, whose archive is
complete — which is the sharpest correction to the old message the rehearsal quoted.

A third plant in the same test is the control: the same row pointed at TY2099 must red on **both**
documents, so a `problems_for` that always returned empty could not pass plant (a).

### Kill 1c — the one surviving excuse is CHECKED, not a permanent free pass

**Mutation M3 — ignore a row's excuse and assume the join succeeds:**

    FAIL  cite_check::tests::authority_coverage_may_only_improve
    panicked at crates/xtask/src/cite_check.rs:1318:9:
    [f8283--2024] now HAS an archived authority for that YEAR — delete the `authority` header from that
    map row (and from map_rows.rs::EXCUSED) so the ratchet actually tightens
    FAIL  cite_check::tests::re_excusing_an_archived_pair_reds_the_ratchet
    Summary  16 tests run: 14 passed, 2 failed

So the stale-excuse arm fires on **real** data, on the one remaining excuse. The moment `f8283--2024` is
archived, leaving the header in place reds.

### Kill 1d — the excuse slot may not accept an arbitrary sentence

New test `cite_check::tests::an_excuse_must_be_spelled_as_an_excuse_with_a_reason`, planting against
`excuse_reason` — the same function `excused_form_years` decides with, not a copy of its logic.

**Mutation M4 — any non-empty string excuses:**

    FAIL  cite_check::tests::an_excuse_must_be_spelled_as_an_excuse_with_a_reason
    panicked at crates/xtask/src/cite_check.rs:1558:13:
    "archived" must NOT satisfy the excuse spelling `excused_form_years` enforces
    Summary  16 tests run: 15 passed, 1 failed

Four plants (`"archived"`, the prefix with no reason, no separator, a leading-space prefix) and a control
that the real spelling IS accepted, so a decider that rejected everything could not pass.

---

## 3. FR-150 — what changed, and its kill

### What it was

    assert_eq!(from_rows.len(), 38,
        "38 rows on disk today (37 -> 41 on 2026-09-06 ... then 41 -> 36 ... then 36 -> 38 on
         2026-09-07 ...); a new year adds files, not a list");

A failure message that narrates three past movements of the number and explains why it will move again,
asserted by a hand-typed `38`.

### What it is now — `CLAUDE.md` option 1, derived from the set

The expectation derives from each year's **`YEAR.toml` `forms_expected`** — design r2 §6's year record,
*"the forms this year INTENDS to bundle — runbook step 1's output, committed"*. So
`the_row_set_equals_the_emitting_surface_both_ways_through_irs_stem` now runs **two** symmetric
differences over **three independently-authored views of one set**:

| view | authored by | read from |
|---|---|---|
| `emitted` | the tree | `crates/btctax-forms/forms/<year>/*.pdf` + `*.map.toml` |
| `from_rows` | the map author | each map's `irs_stem` header |
| `declared` | the porter, runbook step 1 | each year's `YEAR.toml` `forms_expected` |

A new year needs **no edit here at all**: the port writes its year record, and the count follows. And
`forms_expected` is not a second hand-list smuggled in — `btctax-forms/tests/year_record.rs`
(`YearRecord::glob_problems`) already binds it to the glob in both directions, by name.

**No count remains, and no floor either.** A vacuous pass is impossible *structurally*: `map_rows()`
refuses a walk that finds no rows or no `f1040`, so `from_rows` cannot be empty, so an empty `declared`
would name every row. That is why the test needs no number — which was the whole of FR-150.

### The kill

The only case the literal `38` could see and the row-vs-template diff cannot is **a template and its map
deleted together**: both derived sets shrink in step and their difference stays empty. New test
`cite_check::map_row_tests::a_silently_deleted_form_and_an_undeclared_one_are_both_caught` plants exactly
that, in both directions, on the real derived sets — the same way the file's existing
`a_row_pointing_at_no_template_is_caught_in_both_directions` plants, because the *inputs* are files this
agent does not own and a planted input is the honest alternative to a planted file.

**Mutation M5 — the declaration walk silently loses a year** (the FR-150 shape one layer down):

    FAIL  cite_check::map_row_tests::the_row_set_equals_the_emitting_surface_both_ways_through_irs_stem
    panicked at crates/xtask/src/cite_check.rs:1834:9:
    the year records and the map rows disagree — declared in YEAR.toml with no map row: []; map row no
    YEAR.toml declares: [("f1040", 2025), ("f1040s1a", 2025), ("f1040s2", 2025), ("f1040s3", 2025),
    ("f1040sa", 2025), ("f1040sb", 2025), ("f1040sc", 2025), ("f1040sd", 2025), ("f1040sse", 2025),
    ("f1040v", 2025), ("f4868", 2025), ("f6251", 2025), ("f8283", 2025), ("f8889", 2025),
    ("f8949", 2025), ("f8959", 2025), ("f8960", 2025), ("f8995", 2025)]. `forms_expected` is runbook
    step 1's committed output; if a form is genuinely gone from a year, that declaration is where it
    leaves.
    FAIL  cite_check::map_row_tests::a_silently_deleted_form_and_an_undeclared_one_are_both_caught
    panicked at crates/xtask/src/cite_check.rs:1867:9:
    assertion failed: only_declared.is_empty() && only_rows.is_empty()
    Summary  16 tests run: 14 passed, 2 failed

GREEN (reverted): `16 tests run: 16 passed`.

---

## 4. Which committed "not-yet-archived" statements can now be retired

**All 30 of the false ones, and they are already gone** — they lived only in the const this change
deleted. Machine-computed by intersecting the pre-change `AUTHORITY_NOT_YET_ARCHIVED` (parsed from the
`cp` backup) with the post-change archived set (parsed from `xtask cite-check`'s own output line), never
hand-listed:

    OLD excuse-list pairs (hand-typed): 31
    NEW archived pairs (manifest join):  37

    RETIRED — was written "not-yet-archived", is in fact archived + extracted: 30
      f1040--2024    f1040--2025     f1040s1--2024   f1040s2--2024   f1040s2--2025
      f1040s3--2024  f1040s3--2025   f1040sa--2024   f1040sa--2025   f1040sb--2024
      f1040sb--2025  f1040sc--2024   f1040sc--2025   f1040sd--2024   f1040sd--2025
      f1040sse--2024 f1040sse--2025  f6251--2024     f6251--2025     f8275--2024
      f8283--2025    f8949--2024     f8949--2025     f8959--2024     f8959--2025
      f8960--2024    f8960--2025     f8995--2024     f8995--2025     f8995a--2024

    KEPT — genuinely not archived: 1
      f8283--2024      (no manifest entry; its map header's reason is TRUE and stays)

    Unaffected — the 7 that were already archived under the old notion: 7
      f1040s1a--2025  f1040v--2024  f1040v--2025  f4868--2024  f4868--2025
      f8889--2024     f8889--2025

30 + 1 + 7 = 38 = the emitting surface. **The one `"not-yet-archived"` statement in a committed map
header (`forms/2024/f8283.map.toml`) is TRUE and must not be retired** — there is no manifest entry for
`design/forms/2024/f8283--2024.pdf`, only the extract, exactly as its reason says.

---

## 5. Refuted premises

1. **"A genuinely archived form-year must be described as not-yet-archived."** True of the mechanism,
   and now closed — but the brief's framing understated the scale. It was not one document at the
   boundary: **30 of the 31 excused pairs were archived**, i.e. the gate's cheapest discharge was 30
   false sentences, in one committed const.
2. **The rehearsal report's F5 says f8995a/2025 has "committed text layers for BOTH `f8995a--2025` and
   `i8995a--2025`."** That was true in the rehearsal's own worktree and is **NOT true in `main`**:
   `design/forms/extract/` holds `f8995a--2025.txt` but no `i8995a--2025.txt`, and `MANIFEST.json` has
   no `design/forms/2025/i8995a--2025.pdf` entry. The port was never committed. So the rehearsal's
   headline case is, in this tree, a *correctly* unarchived form-year — which is why I could use it as
   the discrimination half of the kill rather than the acceptance half. (The report itself admits this
   at §4 item 5: *"the `cite_check::FORMS` row plus duplicate extract pair ... were not created."*)
3. **`FOLLOWUPS.md` FR-138's suggested fix, *"have the ratchet read the `design/forms/extract/`
   convention ... and retire the second root"*, is two changes, and only the first is mine.** Reading
   the convention is done. Retiring `crates/btctax-core/src/tax/fixtures/` means deleting the committed
   fixture pairs, re-pointing `schedule_1a_docs()`, the `extract-schedule-1a` command's outputs, and the
   `extract_override` header on `forms/2025/f1040s1a.map.toml` — files in three crates I do not own. The
   ratchet no longer *depends* on that root, which was the blocking half; the deletion is design r2 §9's
   own work item and is **not** closed by this report.
4. **The extract-path convention is not merely a convention — it is what the manifest records.**
   Verified over every row: for all 37 archived documents the manifest entry's `extract` field is
   byte-exactly `design/forms/extract/<stem>--<year>.txt`. One divergence in the whole set, and it is
   the missing `f8283--2024` entry. So binding the join to the convention costs nothing today and reds
   if a future entry points elsewhere.

---

## 6. What I could not do for want of a file

None of these blocked the fix. All are one-line or one-declaration edits outside my ownership.

| what | where | why it needs doing |
|---|---|---|
| 4 doc comments describe `AUTHORITY_NOT_YET_ARCHIVED` as carrying `("f6251", &[2024, 2025])` in the present tense | `crates/btctax-core/src/tax/line_coverage.rs:688` (A1), `crates/xtask/src/line_coverage_check.rs:1554` (A1), `crates/btctax-forms/src/map.rs:637` (unowned), `crates/xtask/src/verdict_reach.rs:69` (unowned) | the const is gone and f6251 is now archived, so the sentence is false. **No build impact** — every one is plain backticks, not an intra-doc link; clippy `-D warnings` is clean |
| `design/FORM_AUTHORITY_TABLE_DESIGN.md:212` says `AUTHORITY_NOT_YET_ARCHIVED` is *"shrink-only, **31 of 36** pairs excused"* and *"a DIFFERENT 'archived' from the manifest join"* | S4's file | there is now **one** notion of archived, and it is the manifest join. §212 and §231's "six rows" both want updating; §9's plan to retire the second root is now half-done |
| `design/HARNESS.md:140-141` cites `cite_check.rs:685` and `:750` | unowned | already stale before this change (the items were at `:943` and `:1116`); now stale differently |
| `crates/btctax-forms/forms/2025/YEAR.toml`'s `forms_absent.f1040s1` reason reads *"archive f1040s1--2025, then port"* | unowned | **the archive step is already done.** `design/forms/2025/f1040s1--2025.pdf` is a manifest entry with URL `irs-prior/f1040s1--2025.pdf`, sha256 `8dafec71...`, and extract `design/forms/extract/f1040s1--2025.txt` on disk; `i1040gi--2025` likewise. The blocker named there has been discharged, and the next TY2025 port is a map, not an archive hunt. Kill 1a is built on exactly this fact |
| `FOLLOWUPS.md` FR-138 / FR-150 | coordinator | per the plan, the coordinator marks closure |

---

## 7. Gate numbers

Measured in this worktree, each suite captured once to a file and grepped (never run twice).

| gate | baseline (`376d1141`) | after |
|---|---|---|
| `cargo nextest run -p xtask` | 186 run: **179 passed, 7 failed**, 1 skipped | 189 run: **182 passed, 7 failed**, 1 skipped |
| `cite_check` tests specifically | 13 run, 13 passed | **16 run, 16 passed** (+3) |
| `cargo nextest run --workspace` | not run at baseline | 3617 run: **3610 passed, 7 failed**, 12 skipped |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | — | **clean** |
| `cargo fmt --all --check` | — | **clean** |
| `cargo run -p xtask -- cite-check` | 7/38 archived, 31 excused, 0 unaccounted | **37/38 archived, 1 excused, 0 unaccounted** |

**The 7 failures are the same 7 test names before and after, and both are environmental with the
mechanism shown in the code** (per the plan's standing caveat — not asserted, quoted from the runs):

- 6 x `form_delta::tests::*` — the gitignored `design/forms/<year>/*.pdf` are not shared across
  worktrees. Mechanism: `the TY2026 draft is archived and its geometry extracted: "no PDF found for
  f6251--2026-DRAFT"`, and `ls design/forms/2026/` holds only `*.pdf.txt` files.
- 1 x `harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` — this
  is **FR-147 itself** (S3's item), caused by the mandated `CARGO_TARGET_DIR` override. Mechanism:
  `cargo build -p xtask succeeded but left no binary where on-write.sh looks`.

**New tests added (3), all passing, each with a planted defect seen red above:**

    cite_check::tests::an_archived_form_year_is_accepted_and_an_unarchived_one_still_reds     (M1, M2)
    cite_check::tests::an_excuse_must_be_spelled_as_an_excuse_with_a_reason                   (M4)
    cite_check::map_row_tests::a_silently_deleted_form_and_an_undeclared_one_are_both_caught  (M5)

**Existing tests whose kill coverage grew:** `re_excusing_an_archived_pair_reds_the_ratchet` plants
**7 -> 37** pairs and its guard is derived rather than the literal `7`;
`authority_coverage_may_only_improve`'s stale-excuse arm is now seen red on real data (M3).

**Diff:** `crates/xtask/src/cite_check.rs` and `crates/xtask/src/archive_check.rs` only — 2 files
changed. No product behaviour changed: `xtask` is `publish = false` developer tooling, and nothing
outside it reads any symbol touched here (verified by grep for `excused_form_years`,
`archived_form_years`, `adjudicate_coverage`, `CoverageVerdict` and `cite_check::FORMS` across
`crates/` — no hits outside `cite_check.rs`).

**Mutation hygiene:** every mutation was reverted by `cp` from a backup of the green file
(`scratchpad/a3/cite_check.rs.green`); `grep -c MUTATION` confirmed 0 after each revert. No commits, no
pushes, no stashing, no checkouts, no subagents.
