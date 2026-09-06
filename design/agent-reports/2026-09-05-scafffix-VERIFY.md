# scafffix — adversarial verification of the eight fail-open fixes

**Agent:** scafffix-VERIFY · **Date:** 2026-09-05 · **Method:** every claimed test located and run
scoped; six defects re-planted by hand and watched; every planted file restored from a `cp` backup and
`cmp`-verified byte-exact. No `git`, no `make check`, no workspace run.

> **VERDICT: all eight fixes HOLD. 0 fixes fail, 0 of the fixers' new tests pass with their own defect
> restored.** Findings are **1 Critical · 3 Important · 4 Minor · 1 Nit** — all of them *outside* the
> eight fixed files, and all in the same class the round was called for.

## What was re-planted, and what happened

| plant | file | result |
|---|---|---|
| `Err(u) => {}` — the original silent skip | `form_delta.rs` | **5 RED**, incl. the real-artifact census naming `f1040s1a--2025 -> f1040s1a--2026-DRAFT: compared 8 + unwitnessed 0 != 10 common` |
| a fresh `forms/2026/` with a `[census]`-less map | `field_census.rs` (real dir, removed after) | **2 RED**: `census_accounts_for_every_field` ("2026/f8995: … NO [census] … NO entry on the UNCENSUSED register") and `every_emittable_form_is_reached_by_the_gate_or_named_absent` |
| `validated_table_for(2025) = ty2024_table()` | `shipped_tables…rs` | **RED**: "TY2025 Single ordinary bracket 1: shipped (11925, 0.12) vs validated (11600, 0.12)" |
| `Coverage::default()` on `cover_form6251` | `line_coverage.rs` | **RED**: `all()` panics, "the collector for \"f6251\" (41 row(s)) never named the year" |
| `quoting("2025")` on `cover_form6251` (product level) | `line_coverage.rs` | `xtask line-coverage` **FAILED (5 problem(s))**, f6251 lines 4/18/19/25/39 — the year genuinely drives the checker |
| `Form6251Line1Rule::Y2024` restored verbatim | `return_1040.rs` | **3 of 4 RED**, exactly as reported |
| `masthead_year` = `max()` over pages 1-3 (the shipped bug) | `archive_drafts.py` | corpus audit **FAILED**, naming `f1040--2024` and `f1040--2025` |

Independently re-measured and confirmed: `label-proof` per-year census 2017 0/5, 2024 99/1, 2025 82/0;
`xtask line-coverage` 329 / 24 exceptions / 0 unverifiable / 12 not-line-bound before and after;
`archive_drafts.py --self-test` 7+8+2+3 kill rows and 46 declared / 3 excused over 49 globbed PDFs;
`form-delta f8959--2025 f8959--2026-DRAFT` now prints "24 of 26 common field(s) COMPARED" with the 2
named. 1040 TY2025 line 11b **is** AGI (`f1040--2025.txt:89`), so the new `form6251_line1_rule`
argument is right.

**No fixer touched a file outside their brief.** The only tree mutation beyond the eight owned files is
the archiver's deletion of `design/forms/2026/f1040--2026-DRAFT.pdf`, its `.txt`, and
`design/forms/geometry/f1040--2026-DRAFT.json` — all three derived from a PDF that really was the TY2025
form, so the deletion is right; its unremoved manifest entry is C-2/I-2 below.
`design/ROADMAP_STATUS.md` (mtime 19:19) belongs to none of the eight reports and is unattributed.

---

## C-1 · CRITICAL — a guard named "…and_hashes_true" that cannot fail for 79 of 121 manifest entries

**`crates/xtask/src/authority_manifest.rs:282-289`** (and the kill at `:943`, `:972`)

`verify()`'s `Storage::Committed` arm hashes the file and pushes `HashMismatch`. Its `Storage::Note`
arm does **only** `note.is_file()` — it never opens the binary, even when the binary is on disk. I
measured the manifest: **79 entries are `storage: note`, and 78 of those 79 PDFs are present on disk**
(gitignored, not absent), so `every_manifest_entry_resolves_and_hashes_true` evaluates the hash half of
its own name for **42 of 121 entries** and structurally cannot fail on the other 79. A form PDF that has
been swapped for a different revision passes.

Its B1 kill (`every_problem_class_is_caught`) does not notice, because it plants the tampered hash on a
`Storage::Committed` entry (`:943`) and plants only `MissingNote` on the `Storage::Note` one (`:972`).
That is the round's own class one level up: the checker for the checker also fails open.

**Correction to `scafffix-archiver`'s report.** Its "16 manifest entries have no URL and no sha256
anywhere / unrecoverable provenance" is **wrong**: `design/forms/MANIFEST.json` carries a populated
`sha256` and `url` for all 16 TY2026 entries (verified by parsing it). What is genuinely missing is the
redundant `.txt` note — the archiver writes the `pdftotext` layer into that slot — and the `extract`
field, which is `""` for every 2026 entry and therefore *skipped* by `if !e.extract.is_empty()`. The
checker defect above is real and is the more serious half; the provenance-loss claim is not.

**Minimal fix:** in the `Storage::Note` arm, when `abs.is_file()`, hash it and push `HashMismatch` on a
mismatch — the three lines the `Committed` arm already has — and add a `Storage::Note` hash plant to
`every_problem_class_is_caught`.

---

## I-1 · IMPORTANT — the new per-year join gate reads 43% of the bindings and prints "0 unreachable"

**`crates/xtask/src/label_reader.rs:1047-1072` (`line_bindings`) and `:1409` (`else { continue }`)**

`line_bindings` strips one leading and one trailing `"` from the whole trimmed RHS, so a binding
carrying a trailing comment —

```
line1 = "topmostSubform[0].Page1[0].f1_3[0]"   # "Medicare wages…"
```

— yields an FQN with the comment glued on. It is in no geometry, `join.get(&fqn)` returns `None`, and
`let Some(got) = … else { continue }` drops it **with no count and no name**. Measured by me over all
37 committed maps: **435 bindings accepted, 247 of them malformed (57%)**, worst
`2025/f1040s1a` 46 of 46 and `2024/f1040` 34 of 35.

So `every_mapped_line_lands_on_its_own_printed_label` — the gate whose whole subject is *"a mapped line
must land on the box the form prints that line's number beside"* — checks 185 of 432 real bindings, and
its brand-new census prints `2025: 15 map(s), 82 join(s) checked, **0 unreachable**`. "0 unreachable" is
a false-completeness claim of exactly the shape the fix was commissioned to remove, and the new
`YEAR_FLOORS` ratchets (99 / 82) are pinned to the crippled reader. `scafffix-labelreader` found this
(its F2), stated the size correctly, and declined to fix it; the fix holds for what it claims, but the
instrument it hardens is still the largest fail-open in the eight files' blast radius.

**Minimal fix:** take the first quoted span (`rhs.split('"').nth(1)`), and replace the
`else { continue }` at `:1409` with a push into `r.unwitnessed` so an unresolvable binding is *named*
rather than dropped. Expect ~247 new joins and reds in map files; that is the point, and it needs its own
task with the floors re-measured afterwards.

---

## I-2 · IMPORTANT — two tests left RED in the shared tree, one of them by an unowned side effect

1. **`crates/xtask/src/authority_manifest.rs:917`** — `every_manifest_entry_resolves_and_hashes_true`
   fails: *"design/forms/2026/f1040--2026-DRAFT.pdf — gitignored, and its `.txt` provenance note is
   missing"*. Cause: the archiver correctly deleted the mislabelled PDF but left
   **`design/forms/MANIFEST.json:570`** naming it. This is a correct red with a one-line fix and no
   owner; leaving it is how a real red gets muted.
   **Minimal fix:** delete that manifest entry (or run `xtask authority-manifest --regen`).

2. **`crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs:755`** —
   `every_shipped_year_has_a_validated_counterpart` is *deliberately* red on TY2017/TY2025/TY2026 and
   **cannot be closed inside the repo**: the fixer states, and I confirmed, that Rev. Procs 2016-55,
   2024-40 and 2025-32 are not on disk (`legal/text/irs-guidance/` holds only `RevProc_2024-28.txt`).
   Shipping a permanently-red gate with no in-tree path to green invites the one closure the file
   forbids — pasting `tax_tables.rs::ty2025()` into `testonly`, which makes the equality vacuous.
   The finding itself is correct and valuable (I spot-checked: none of the TY2025 MFJ/HoH interior
   thresholds 23850/96950/206700/394600/501050/17000/64850/197300/250500 appears more than once in
   `tax_tables.rs`, i.e. only in the definition).
   **Minimal fix:** an owner's call, not a fixer's — either `#[ignore = "…"]` with the follow-up id in
   the message, or an explicit decision to carry the red. It must not sit unlabelled in the suite.

---

## I-3 · IMPORTANT — the THIRD geometry wildcard is still open, and its fix was written twice today

**`crates/btctax-forms/src/form8283.rs:70-77`**

```rust
(_, Form8283Section::A) => SEC_A_CLUSTERS_2023,
(_, Form8283Section::B) => SEC_B_CLUSTERS_2023,
```

This is byte-for-byte the defect `scafffix-panics` removed from `form1040.rs` and `schedule_se.rs` in
this same round, with the measurement showing a wildcard *disarms* the dollars/cents column guard rather
than merely skipping it. The fix — enumerate the arms, plus
`geometry_is_recorded_for_exactly_the_supported_years`, which derives its expected set from
`SUPPORTED_YEARS` — exists in the branch and was not carried across. `scafffix-panics` flagged it and
said its test "transplants directly". This is `CLAUDE.md` B3's own case: the field of view, not the
knowledge.

**Minimal fix:** enumerate `(2017, A/B)` and `(2024 | 2025, A/B)`, delete both `_` arms, and copy
`geometry_is_recorded_for_exactly_the_supported_years` into `form8283.rs`.

---

## Minor

**M-1 · `crates/xtask/src/form_geometry.rs:194`** — the dead `unwrap_or("2025")` survives in the sibling
of the site `scafffix-labelreader` fixed. `rsplit` always yields at least one item, so it can never fire,
and it reads as a deliberate fallback-to-2025 policy in the site most likely to be copied when the next
stem-consuming command is added. **Fix:** `let year = crate::label_reader::stem_year(stem)?;` — it is
`pub(crate)` for exactly this.

**M-2 · `scripts/archive_drafts.py:343`** — `_corpus_audit`'s glob is
`design/forms/[0-9][0-9][0-9][0-9]/f*.pdf`, so the **29 committed `i*.pdf` instruction booklets** under
`design/forms/{2024,2025}` (49 of 78 archived PDFs audited) are outside the year audit entirely, and the
exclusion is recorded nowhere in the audit's output. Those booklets are subject to the same
`max()`-over-year-tokens mis-filing the round exists to close. **Fix:** state the exclusion and its
count in the audit line, or run them with `require_stamp=False` — an option `adjudicate` already
supports and nothing calls.

**M-3 · `scripts/archive_drafts.py:286-289`** — `classify_form_pdf` **re-derives** the revision-dated
excuse instead of consuming `declared_year`'s decision, and does it more loosely: `declared_year`
requires `mast is None AND not stamps`, `classify_form_pdf` requires only `masthead is None and
revision_date(...)`. A document with no masthead, a `(Rev. …)` marker **and** a contradicting footer
stamp would be `excused` where the adjudicator says PROBLEM. Not reachable on today's corpus.
**Fix:** return the reason code out of `declared_year` and branch on it; the codes already exist.

**M-4 · `crates/xtask/src/form_delta.rs:801`** — in
`no_archived_pair_reports_a_clean_verdict_from_zero_comparisons`, `assert!(compared > 0, …)` is
tautological: `label_verdict()` returns `Unwitnessed` whenever `label_compared == 0`, so no
`Unchanged`/`Moved` value can ever carry `compared == 0`. The test's real teeth are the accounting
invariant (which I watched go red on plant P3, on `f1040s1a`), and the pure test
`zero_comparisons_is_unwitnessed_and_never_a_clean_verdict` covers the guard directly — but this line
reads as a check and is not one. **Fix:** delete it, or assert the `Unwitnessed` reason set instead.

## Nit

**N-1 ·** `scafffix-archiver`'s plant-A claim ("+ f1040--2024 and 3 more", i.e. 5 files) is overstated:
restoring `max()` over pages 1-3 reds the corpus audit on exactly **two** — `design/forms/2024/f1040--2024.pdf`
and `design/forms/2025/f1040--2025.pdf`. The kill is real; the count is not.

## Explicitly empty

**No fix failed to hold, and no new test passes with its own defect restored.** The nearest misses,
both already disclosed by their authors, are: the archiver's `_reader_kills` exercise the pure
`pick_masthead` only, so restoring the shipped `max()` inside `masthead_year` leaves all 7 reader rows
green (the corpus audit is what catches it — the kill table alone would not); and the archiver's plants
**O** and **P** are green and stated as such, being unreachable fail-closed branches.

**No secret-handling findings** (out of severity scope per `CLAUDE.md`).
