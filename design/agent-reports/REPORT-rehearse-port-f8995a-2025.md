# REPORT — the PORT REHEARSAL: `f8995a/2025` walked through all 24 runbook steps

**Agent:** one opus agent, isolated worktree `368bb547`, `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`.
**Nothing committed, nothing pushed, nothing merged.** The tree is left dirty on purpose; the findings are the deliverable.
**Date:** 2026-09-12.

## 0. The headline, in four numbers

| question | answer |
|---|---|
| **Step 22's five bindings** (`include_bytes!`, `*_pdf` arm, `include_str!`, `tyYYYY()`, `for_year` arm) | **85 -> 0.** Not one of the five was touched. `Form8995AMap::for_year` is fully year-generic; `packet.rs` calls it with `year`. Phase 5 delivered exactly what it was for. |
| **What replaced them** | **6 hand-edits per new `(stem, year)`** — 5 in `line_set.rs`, 1 in `f6251_revision.rs`. Before Phase 5 it was 5 per `(stem, year)` (85 / 17). **The keystroke count did not fall; it rose by one and moved.** What fell sharply is the *risk*: all 6 are in 2 files, 4 of the 6 are compiler- or glob-enforced, and no `include_str!` path can escape the published tarball any more. |
| **Total hand-edits to take `f8995a/2025` from "archived authority" to a green workspace** | **13** in 6 Rust files (list in section 2), of which **6 are hardcoded counts or absence-assertions**, **1 is a factually false admission**, and **2 are the intended judgment slots**. Plus 1 regenerated golden (a command, not an edit) and 6 data artifacts. |
| **2024->2025 field delta** | **111 common, 0 added, 0 removed, 0 printed labels moved** (`xtask form-delta f8995a--2024 f8995a--2025`). The cleanest possible port. |

And the one result that matters most:

> **A TY2025 map whose line-21 doc comment quotes the TY2024 thresholds passes the entire suite: 3,614 tests run, 3,614 passed.**
> Planted, measured, restored. That is the Form-6251-line-33 defect class, at the exact moment the port introduces it, with no gate looking.

---

## 1. The 2024->2025 delta, measured

### Field axis (`xtask form-delta f8995a--2024 f8995a--2025`)

```
  fields: 111 common, 0 added, 0 removed
  line->label: 109 of 111 common field(s) COMPARED; none of them changed the printed line it sits beside
  * 2 of 111 common field(s) yielded NO comparable label:
    2 — BOX-WITH-NO-LABEL on both sides — no printed line claims this box:
      topmostSubform[0].Page1[0].f1_01[0]     (name)
      topmostSubform[0].Page1[0].f1_02[0]     (TIN)
```

`xtask label-census f8995a--2025` -> **40 labels, 111 boxes; 40 entry lines, 0 without a box.**
`xtask label-proof f8995a--2025` -> **111 boxes filled with their assigned label, 2 printed `?`** (the same name/TIN pair), **0 labels with no box.**

### Text axis (`diff -b` of the two extracts — runbook step 24's P3)

The whole 2024->2025 text delta is **8 hunks / 36 diff lines**, and exactly **two numbered lines** changed printed text:

| what changed | 2024 | 2025 |
|---|---|---|
| masthead year | 2024 | 2025 |
| **OMB number** | **1545-2294** | **1545-0074** |
| Part I use-this-form sentence | $191,950 / $383,900 | $197,300 / $394,600 |
| **line 3** (quoted in the map) | $191,950 / $383,900 | $197,300 / $394,600 |
| Part III header sentence | $191,950-$241,950 / $383,900-$483,900 | $197,300-$247,300 / $394,600-$494,600 |
| **line 21** (quoted in the map) | $191,950 / $383,900 | $197,300 / $394,600 |
| footer | `Form 8995-A (2024)` | `Form 8995-A (2025) Created 9/12/25` |

Two judgment calls fell out of this, and both were cheap:

* The footer's **`Created 9/12/25`** stamp looks like a draft marker on a final document. Resolved by one grep: **22 of the TY2025 extracts carry it and only 1 TY2024 extract does** — it is the TY2025 print convention. About a minute.
* The **OMB number changed on a form whose line set did not.** Nothing in the repo reads it; recorded, not acted on.

### Census axis (runbook step 21 / P4)

**65 `[census]` entries carried forward unchanged**, and P4's rule is satisfied *by measurement* rather than by assertion: name axis 0 changes, label axis 0 moves, and the only two lines whose printed text moved (3 and 21) are **mapped**, not censused. The new map is **292 lines / 45,436 B — byte-for-byte the same size as the 2024 map, differing on 18 lines.**

### The port itself

`crates/btctax-forms/forms/2025/f8995a.map.toml` was produced by a **script**, not by hand — 11 substitutions over 7 patterns (`year`, `template_sha256`, `line_set`, four threshold figures, two paths, one prose year). Then verified: **44 of 44 quoted captions verbatim against `f8995a--2025.txt`, 0 failures** — and the kill was watched: the **unported** 2024 map run against the 2025 extract reds on exactly lines 3 and 21 and nothing else.

---

## 2. Step 22, measured exactly — the payoff question

### The five bindings the runbook names: **0 of 5 touched.**

```rust
// crates/btctax-forms/src/map.rs — Form8995AMap::for_year, unchanged by this port
let text = crate::bundled::map_text(crate::bundled::Stem::F8995a, year)
    .ok_or(crate::FormsError::UnsupportedYear(year))?;
let row = MapRow::read(text)?;
let ls  = crate::line_set::LineSet::parse(&row.line_set)?;
match crate::line_set::schema(ls) { ... }
```

`include_bytes!` const: gone (build.rs glob). `*_pdf` arm: gone. `include_str!` const: gone. `tyYYYY()`: still present as a convenience, **not required** — nothing was added. `for_year` arm: gone.

### The 13 edits it actually cost

| # | file | edit | forced by |
|---|---|---|---|
| 1 | `src/line_set.rs` | `LineSet::F8995a_2025` variant | — (the new row) |
| 2 | `src/line_set.rs` | `parse` arm | runtime refusal -> `map_rows` red |
| 3 | `src/line_set.rs` | `as_str` arm | **E0004** |
| 4 | `src/line_set.rs` | `ALL` entry | `map_rows` / round-trip test |
| 5 | `src/line_set.rs` | `schema()` arm -> `Schema::Form8995AMap` | **E0004** — the one real judgment |
| 6 | `src/f6251_revision.rs` | the underscore-free "not this schema" arm | **E0004** — an unrelated form's module |
| 7 | `src/line_set.rs` (tests) | `LineSet::ALL.len()` 38 -> 39 | hardcoded count |
| 8 | `tests/map_pdf_conformance.rs` | split the 8995-A refusal loop (TY2025 no longer refuses) | a test asserting the ABSENCE of what I added |
| 9 | `tests/year_record.rs` | `expected_count` 2025: 18 -> 19 | hardcoded count |
| 10 | `tests/year_record.rs` | the B1 phantom plant re-keyed `f8995a` -> `f8275` | **the kill's plant stopped being a defect** (F4) |
| 11 | `tests/supported_years_cross_product.rs` | `BUNDLED_FORMS_PER_YEAR` 2025: 18 -> 19 | hardcoded count |
| 12 | `crates/xtask/src/cite_check.rs` | `AUTHORITY_NOT_YET_ARCHIVED` f8995a `&[2024]` -> `&[2024, 2025]` | **a false admission** (F5) |
| 13 | `crates/xtask/src/cite_check.rs` | map-row count 38 -> 39 | hardcoded count |

Plus **1 regeneration**, not an edit: `docs/examples/examples.md` moved `TY2025 — preparing (18 forms; ...)` -> `(19 forms; ...)` on two lines, because `YearReadiness` counts the glob. **That is design step 4 working**, and it is the shape every one of edits 7/9/11/13 should have.

### Data artifacts

`forms/2025/f8995a.map.toml` (new, scripted) · `forms/2025/f8995a.pdf` (byte-identical copy) · `forms/2025/YEAR.toml` (2 lines: f8995a out of `[forms_absent]`, into `forms_expected`) · `design/forms/2025/i8995a--2025.pdf.txt` (new note) · `design/forms/extract/i8995a--2025.txt` (new, `pdftotext` plain) · `design/forms/MANIFEST.json` (1 entry, regenerated).

### Extrapolated to a 17-form year

Per-form: **6** (5 `line_set.rs` + 1 `f6251_revision.rs`) -> **102**, plus about 6 fixed per-year edits. Against TY2024's 85. **Step 22's claim is refuted in letter — the five bindings are gone — and the aggregate number is not better.** The fix is named in F10: four of the five `line_set.rs` edits are pure transcription of the row string and belong in `build.rs`; only `schema()` encodes a decision.

### Closing gate (evidence run)

```
find crates -name '*.rs' -exec touch {} +      # the `make gate` freshness forcing
cargo nextest run --workspace --no-fail-fast   -> 3614 tests run: 3614 passed, 12 skipped
cargo clippy --workspace --all-targets --all-features -- -D warnings -> exit 0
```

One environmental red was met and diagnosed, not reported as a suite failure — see **F13**.

---

## 3. The 24 steps, per-step verdict

Legend: **tag** = is the runbook's M / M-star / H right? **today** = is the "today" column still true? **cost** = what it actually took.

| # | step | tag | tag right? | "today" still true? | actual cost for `f8995a/2025` |
|---|---|---|---|---|---|
| 1 | Decide the year's form list | H | **yes** | yes | About 2 min. `irs-prior/f8995a--2025.pdf` -> HTTP 200, hash-exact to the note. **The runbook omits step 1's real output:** `YEAR.toml`. f8995a was declared **absent** for 2025, so the port needs 2 `YEAR.toml` edits nobody tells you about until 3 tests red (**F15**). |
| 2 | Fetch the authority PDF | M-star | **yes** | **yes — confirmed** | 1 `curl` from the note's own first line; sha256 `3362db81...` matched exactly. `archive_drafts.py` hardcodes the `irs-dft/<stem>--dft.pdf` draft URL (lines 131/754) — drafts only, as claimed. Its `STEMS` is a hand-typed 18 against `Stem::ALL`'s 21 (**F9**). |
| 3 | Write the `.pdf.txt` provenance note | M-star | **yes** | **no** | The form's note already existed; the **instructions** note did not — I wrote it. The runbook says *"format fixed by `design/forms/README.md`"*: the README says what the note *carries*, never its parse format. The real contract is in `authority_manifest.rs` (line 1 starts `http`, plus a 64-hex run somewhere). About 5 min of reading source. |
| 4 | Add/refresh the `MANIFEST.json` entry | M | **no — M-star at best** | **NO** | `--regen` **REFUSES** in any tree not holding all 125 gitignored PDFs: *"REFUSING to regenerate: it would drop 124 document(s) from MANIFEST.json without a word."* A fresh clone or isolated worktree holds **0**. I had to copy 125 PDFs in from the main tree (**F6**). Also the documented flag `--regenerate` does not exist — it is `--regen` — and the unknown flag is silently ignored, running the *checker* instead (exit 1) (**F7**). Once the tree was whole: 1 command, correct entry. |
| 5 | Name the instructions document + page range | H | **yes, but trivially** | yes | 30 s. f8995a -> i8995a by the plain rule; a standalone booklet, so **no `instr_pages`**. One of the "12 of 17 mechanical" cases. |
| 6 | Fetch + note + manifest the instructions | M-star | **yes** | yes | 1 `curl` (`i8995a--2025.pdf`, 248,825 B, sha256 `6df1301c...`) + note + `--regen`. Ordering matters and the runbook has it right: `--regen` before the note writes `url: ""`, and the read-only checker catches it by name. Good instrument. |
| 7 | Extract **both** text layers | M-star | **yes** | **NO** | `cargo run -p xtask -- forms extract` **does not exist** — yet **58 committed extracts name it as their regeneration command**. And **52 of 126 extracts carry no regeneration line at all**, including both f8995a ones, so the flag set is unrecorded: I had to *guess* `-layout` and confirm by `cmp` (it reproduces byte-identically). Instructions extracted plain, per the 2024 note's two-column rule (**F8**). |
| 8 | Hash the extract **into** the manifest | M-star | **yes** | **yes — confirmed ABSENT** | `authority_manifest::Entry` has `sha256` (of the PDF), `bytes`, `url`, `extract` — **`extract` is a path, never a hash.** Nothing detects a hand-edited or stale text layer. Not done; cannot be done today. |
| 9 | Generate the geometry fixture | M | **yes** | **NO — REFUTED** | 1 command, 0.12 s, and the committed fixture **reproduces byte-for-byte**. The *"broken on drafts"* claim is **stale**: `extract-geometry f8995a--2026-DRAFT` prints *"dropped the DRAFT cover sheet and renumbered pages"* and also reproduces its committed fixture exactly. Design section 4's proposed `cover_pages` / `page_src` fields are still **absent** from the fixture schema. |
| 10 | Assert `geometry.pdf_sha256 == MANIFEST.sha256` | M-star | **arguably M** | **PARTLY REFUTED** | No test states that literal equality, but the chain is enforced over the glob: `map_rows.rs` kills 2+3 hold `template_sha256` == the bundled PDF == an `is_authority()` MANIFEST entry, and `label_reader::every_map()` pairs a map to its geometry **by sha256 of the bundled PDF, never by name**. Planted: a TY2024 `template_sha256` carried forward reds `map_rows`; a deleted geometry fixture reds **4** tests. Cost: 0. |
| 11 | Copy the PDF to `forms/<year>/<crate_stem>.pdf` | M | **yes** | **partly stale** | 1 copy, hash-verified. No alias needed (f8995a = f8995a). `STEM_ALIASES` still exists with 2 rows but is now held equal to each map header's `irs_stem` by test, so it is no longer something the operator consults. **This step breaks the build until step 21 lands** (`build.rs`: *"forms/2025/f8995a has a .pdf but no .map.toml"* — observed) (**F11**). |
| 12 | Dump the AcroForm field inventory | M | **yes** | **NO — a DEADLOCK** | **Step 12 cannot follow step 11.** `xtask` depends on `btctax-forms`, whose `build.rs` refuses the unpaired PDF step 11 just created — so `cargo run -p xtask -- dump-fields` exits **101** with a build-script panic. The tool you need to *author* the map is locked out by the map's absence. I had to move the template back out, dump (111 fields), then put it back (**F1**). |
| 13 | Prior-year delta on both axes | M | **yes** | yes | 1 command, instant, and it is excellent: it reports the 2 uncompared fields **by name** rather than folding them into a pass. See section 1. |
| 14 | Enumerate the printed line set from the extract | M | **yes** | yes | 1 command. 40 labels / 111 boxes / 0 labels without a box. |
| 15 | Join line -> field with both witnesses | M | **yes** | yes | 1 command (`label-proof`), 111/111 assigned. It writes to a hardcoded `/tmp/<stem>-label-proof.pdf` (**F14**). |
| 16 | **Adjudicate witness disagreement** | H | untested here | n/a | **Cost 0 — no disagreement arose.** 109 of 111 fields compared and every one agreed. This rehearsal therefore says **nothing** about the step the runbook calls load-bearing; a form with filing-status blocks (the f8959 case) would. Recorded as a gap in the rehearsal, not as a closed step. |
| 17 | **Decide whether a moved line is the SAME line** | H | untested here | n/a | **Vacuous — nothing moved.** Same caveat as 16. |
| 18 | Diff the map key set vs the `*Map` field set, both directions | M-star | **no — it is M today** | **NO — REFUTED** | Both directions are enforced over the glob **now**: a missing key by serde; an extra key by `deny_unknown_fields`. Planted an extra `line41`: **5 tests red**, headed by `line_set_wiring::every_revision_parses_into_the_struct_its_own_row_names`. Cost 0. |
| 19 | **Amend the `*Map` struct** | H | **yes** | yes | **Cost 0 — no amendment needed.** The 2025 revision is constants-only, so `line_set = "f8995a/2025"` resolves to the existing `Form8995AMap` (the design's many-to-one, used as intended). Note sections 4 and 10 of the design **disagree** about whether a constants-only year gets its own `line_set`; all **39** maps on disk carry a per-year one, so the convention in practice is `<stem>/<year>` always (**F18**). |
| 20 | **Transcribe each mapped line's instruction text verbatim** | H | **yes** | **the check is NOT year-generic** | The transcription was 2 re-quoted lines. The **check** exists as `f8995a_map.rs::every_quoted_instruction_is_verbatim_on_the_form` (44/44) but is pinned to TY2024 by two `include_str!`s and never sees the 2025 map. I ran an equivalent check by hand: **44/44 verbatim**, and watched it red on the un-ported map. **My script is not a committed gate** (**F2**). |
| 21 | **Write the `[census]`** | H | **yes, but cost 0 here** | yes | 65 reasons carried forward verbatim, licensed by measurement (P4: 0 name changes, 0 label moves, and neither changed line is censused). A rebuilt form would pay the full 551-reason price; a constants-only one pays nothing. |
| 22 | **Emit the five code bindings** | M-star | **the tag is now wrong — there are no bindings to emit** | **REFUTED: 85 -> 0** | See section 2. The five are gone; 13 other edits took their place, 6 of them hardcoded counts. |
| 23 | Run the gates | M | **yes** | **REFUTED: 4 of 5 are year-generic, not 1** | Measured by planting, not by reading. See the table below. |
| 24 | **Decide whether a line's MEANING changed while its number did not** | M for form text + quoted sentences, H for the residual | **the M half is half-deployed** | **PARTLY REFUTED** | The form-text half worked: `diff -b` gave the whole delta in one command. The quoted-sentence half — *"`Coverage::quoting(year)` re-verifies each instruction against the new extract and REDS on a carried-forward sentence"* — is **true of the mechanism and false of the deployment**: **26 of the 27** coverage blocks say `Coverage::quoting("2024")`, and all three f8995a blocks do. Adding a year re-verifies **nothing**. Proven discriminating: re-pointing those three blocks at 2025 named **exactly lines 3 and 21** (**F3**). |

### Step 23, measured by planted defect

Each plant was applied to the *finished* port, the whole workspace run, then restored.

| plant | gate that caught it | year-generic? |
|---|---|---|
| **A** `line27` -> a field the 2025 PDF has no such name for | `map_pdf_conformance::every_committed_map_field_exists_in_its_own_pdf` + `field_census::census_accounts_for_every_field` | **yes** |
| **B** line 21's doc comment reverted to the TY2024 thresholds | **NOTHING. 3614 run, 3614 passed.** | **NO** — the hole |
| **C** lines 27 and 28's fields swapped (both real; the labels now disagree) | `label_reader::every_mapped_line_lands_on_its_own_printed_label` | **yes** |
| **D** `template_sha256` carried forward from TY2024 | `map_rows::every_committed_map_has_a_row_that_parses_and_all_four_kills_are_green` | **yes** |
| **E** the 2025 geometry fixture deleted | 4 tests, incl. `supported_years_cross_product` and both `form_delta` work-list tests | **yes** |
| **F** an extra `line41` key the struct has no field for | 5 tests, headed by `line_set_wiring` | **yes** |

**Four of the five step-23 gates now walk the glob and red on the new year. The fifth — doc comment against the extract — is the one that guards the defect class this repo's CLAUDE.md opens with.**

---

## 4. Findings

Severity per `STANDARD_WORKFLOW.md`. "Owning phase" is a proposal.

### Important

**F1 — Step 12 cannot follow step 11; the runbook's dependency order deadlocks.**
`build.rs` panics on a template without a map, which breaks `btctax-forms`, which `xtask` depends on — so `cargo run -p xtask -- dump-fields` exits 101 after step 11. The tool that tells you the field names is locked out by the absence of the file it exists to help you write. The panic message is about the map pairing and says nothing about reordering.
*Fix:* dump fields from `design/forms/<year>/<stem>--<year>.pdf` (as I did), and say so in the step; or have `forms port` do it before the copy. **Owning phase: the port machine (`forms` namespace) plus a one-line runbook fix.**

**F2 — The doc-comment-against-extract gate is not year-generic, and a stale instruction quote in a new year's map is invisible.**
Planted and measured: line 21 quoting *"Threshold. Enter $191,950 ($383,900 if married filing jointly)"* in the **TY2025** map -> **3,614 tests, 3,614 passed.** The gate exists (`crates/btctax-forms/tests/f8995a_map.rs`, 44 captions) and is pinned by `include_str!("../forms/2024/f8995a.map.toml")` and `include_str!(".../f8995a--2024.txt")`. Every per-form map test in the crate has this shape.
*Fix:* one glob-walking test that, for every `.map.toml` on disk, re-runs the caption extraction against `design/forms/extract/<irs_stem>--<year>.txt`. My throwaway Python version found 44/44 and red on 2/44 for the un-ported map, so the checker is about 40 lines and its B1 kill is free.
**Owning phase: NOW — before any TY2026 map is written.**

**F3 — `LineCoverage`'s quoting year is a literal in each block, so a new year re-verifies nothing.**
26 of 27 blocks are `Coverage::quoting("2024")`; f8995a's three all are. The instrument is not blind — re-pointing them at 2025 named exactly the two lines whose text changed — it is simply aimed at the old year, and adding a year does not move it. When TY2026 lands, 377 money lines keep being checked against TY2024 extracts unless someone edits 26 literals.
*Fix:* derive the quoting year from the revision being described, or at minimum assert that every bundled year has coverage rows quoting it.
**Owning phase: the port machine / step-24 widening.**

**F4 — A B1 kill-test's planted defect is keyed to a form that happens to be absent, and porting that form destroys the kill.**
`year_record::a_phantom_expected_form_and_an_undeclared_bundled_form_are_both_reported` plants `"f8995a"` into TY2025's `forms_expected` to prove `glob_problems` reports *"expected but not bundled"*. Once f8995a **is** bundled, the plant is no longer a defect and the test reds with `[]`. After this port the supply of usable stems for TY2025 is **two** (`f1040s1`, `f8275`), and the same test already uses `f1040s1` for the opposite direction. When a year is complete the supply is **zero** — and the path of least resistance at that point is to delete the test.
*Fix:* plant by **removing a stem from the measured `present` list** rather than by naming a form that is absent today. Then the kill is independent of how complete the year is.
**Owning phase: NOW — this bites on every port, and it is the exact CLAUDE.md "typed list beside a set that grows" shape applied to an instrument's plant.**

**F5 — A fully archived form-year still cannot satisfy `authority_coverage_may_only_improve`, and the only cheap discharge is a false statement.**
`archived_form_years()` counts a `(form, year)` as archived **only** if `cite_check::FORMS` has a row for it **and** a duplicate extract pair exists under `crates/btctax-core/src/tax/fixtures/<stem>_form.txt` and `_instructions.txt`. f8995a/2025 has: the PDF archived, a note with URL, sha256 and bytes, a MANIFEST entry, and committed text layers for **both** `f8995a--2025` and `i8995a--2025`. It is still *unaccounted*, and the failure message reads *"btctax can print [f8995a--2025] with no archived primary source for THAT YEAR"* — which would send a January operator back to re-check an archive that is complete. The alternatives are (a) grow the hand-written `FORMS` const and the second extract root that design r2 section 9 says must be **retired**, or (b) add the year to `AUTHORITY_NOT_YET_ARCHIVED` — a list whose own name makes that false. **I did (b), with a source comment saying it is a finding rather than an admission.**
*Fix:* make the ratchet read the `design/forms/extract/` convention (the manifest join already does), and retire the second root as section 9 plans. Until then, fix the message to say what it measures.
**Owning phase: design r2 section 9 / the port machine.**

**F6 — Runbook step 4 is tagged M and is not executable in the repo's own isolated-worktree workflow.**
`xtask authority-manifest --regen` refuses — correctly, loudly, naming 124 documents — in any tree that does not hold every gitignored authority PDF. An isolated worktree holds **0** of the 125; a fresh clone holds 0. There is no committed fetch script (step 2 is M-star), so the precondition for step 4 is "re-fetch 125 documents by hand". I discharged it by copying the PDFs from the main tree, which a January port could do only because the main tree happens to hold them.
*Fix:* `forms fetch --restore` reading every note's URL and sha256 — the notes already contain exactly that data, and `authority-refresh --check --from-dir` already proves the plumbing.
**Owning phase: the port machine (`forms fetch`).**

**F8 — 58 committed extracts name a regeneration command that does not exist; 52 record none at all.**
Measured over `design/forms/extract/`: **126** extracts; **74** carry a `# Regenerate:` line — **58** of them `cargo run -p xtask -- forms extract` (no `forms` subcommand exists; the usage string confirms), 15 `archive_drafts.py --relayout 2026`, 1 a literal `pdftotext`. The remaining **52 carry nothing**, including `f8995a--2024.txt` and `f8995a--2025.txt`, so the `-layout` decision — which the 2024 instructions note calls out as load-bearing — is unrecorded for them. I recovered it by trial (`pdftotext -layout` reproduces byte-identically; `cmp` exit 0).
*Fix:* build `forms extract` (design section 4 already specifies reading each file's own recorded flags) **and** backfill the header on the 52 that record nothing, since that is the input the tool needs.
**Owning phase: the port machine (`forms extract`), build-order item 6.**

**F10 — The per-`(stem, year)` cost moved into two files, and one of them is a Form-6251-specific module.**
`f6251_revision::revision()` is an underscore-free match over **every** `LineSet`, so porting f8995a produced an `E0004` in a module about Form 6251. The trap is right; its blast radius is wrong. Matching on `Schema` (or keying the revision table by schema) would confine it to 6251 revisions and cost nothing.
Separately: of the 5 `line_set.rs` edits, **4 are pure transcription of the row string** (variant, `parse`, `as_str`, `ALL`) and could be generated by `build.rs` from the headers; only `schema()` encodes a decision. That single change would take the per-form cost from 6 to 1 and make the answer to step 22 unambiguous.
**Owning phase: year-package table step 6 / the port machine.**

**F17 — Nothing fills the new year's template.**
All 10 fill cases in `crates/btctax-forms/tests/f8995a_fill.rs` use `Form8995AMap::ty2024()`. `packet.rs` is year-generic (`Form8995AMap::for_year(year)`), so the path exists — but no test drives a TY2025 fill or read-back, and none can while `full_return_for(2025)` is `None`. So this port establishes *parses and dispatches*, never *prints the right numbers in the right boxes*.
*Fix:* parameterise the fill tests over `bundled_years()` where the map schema is shared, so a new year inherits the read-back for free.
**Owning phase: the TY2026 build.**

### Minor

**F7 — Step 4's documented flag is wrong, and unknown flags are ignored rather than refused.**
The runbook says `xtask authority-manifest --regenerate`; the code accepts only `--regen` (`main.rs:136`). `--regenerate` falls through to the read-only checker, which exits 1 — loud, but for the wrong reason. *Fix: the doc, plus refusing an unrecognised argument.* **Owning phase: doc fix plus argument hygiene.**

**F9 — `archive_drafts.py::STEMS` is a hand-typed 18 against `Stem::ALL`'s 21.** `f1040v`, `f4868` and `f8889` are silently never fetched by the draft archiver. *Fix: derive from `Stem::ALL` through `irs_stem`.* **Owning phase: the port machine.**

**F11 — The port of one form is one atomic commit, and the runbook does not say so.** Between step 11 (copy the template) and step 21 (write the map) `build.rs` refuses, so nothing in steps 11 to 22 can be committed separately. That collides with the repo's persist-then-fold commit discipline for anything found mid-port. *Fix: say it in the runbook; `forms port` should write template and map together.* **Owning phase: runbook fix.**

**F12 — Two `LineSet` doc comments are attached to the wrong variant.** The doc line for `"f8959/2024"` sits above `F8889_2024`, leaving `F8959_2024` undocumented; the 2025 block repeats the same slip. **Owning phase: now, one line each.**

**F13 — One suite test cannot pass under the `CARGO_TARGET_DIR` override this brief mandates.**
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` requires a binary at `<repo>/target/debug/xtask`, and its `ensure_xtask_binary` helper inherits `CARGO_TARGET_DIR`, so it builds into the override and then asserts the default path. **Mechanism proven, not assumed:** copying the built binary to `<worktree>/target/debug/xtask` took xtask from 185/186 to **186/186**. This is one of the "environmental failures" class from 2026-09-09, now with a named cause. *Fix: have the helper build with `CARGO_TARGET_DIR` cleared, or resolve the hook's lookup through `cargo metadata`.* **Owning phase: now (environmental-red hygiene).**

**F14 — `xtask label-proof` writes to a hardcoded `/tmp/<stem>-label-proof.pdf`.** On this box `/tmp` is a 32 GB tmpfs shared with builds. *Fix: honour `TMPDIR`, or take an output path.* **Owning phase: nit.**

**F15 — Runbook step 1's real output, `YEAR.toml`, is not in the runbook.** Porting a form the year declares ABSENT needs a `forms_expected` addition and a `[forms_absent]` removal, and the operator discovers this only when `year_record` (twice) and `field_census` red. The messages are excellent (*"2025: f8995a is bundled but not expected (and declared ABSENT — a contradiction)"*) — the step is just missing. **Owning phase: runbook fix.**

**F16 — A hardcoded count whose own failure message argues against hardcoded counts.** `cite_check.rs:1490`: *"38 rows on disk today ... a new year adds files, not a list"* — asserted by a hand-typed `38`. Worth quoting verbatim in the design as evidence for generating the counts. **Owning phase: nit / design citation.**

**F18 — The design contradicts itself on `line_set` for a constants-only year.** Section 4: *"constants-only year => same line_set; renumber => new one."* Section 10 step 1: the 15 maps served by 5 shared structs get **per-year** `line_set`s. All **39** maps on disk carry `<stem>/<year>`, so section 4's sentence has zero instances and I had to pick (I followed the tree). *Fix: strike the section 4 sentence, or record that the convention is per-year always and the many-to-one lives in `schema()`.* **Owning phase: design r2 to r3 edit.**

### Not a finding, recorded

* The TY2025 footer `Created 9/12/25` is the year's print convention (22 TY2025 extracts carry it, 1 TY2024 one does), not a draft marker.
* Form 8995-A's **OMB number changed 1545-2294 to 1545-0074** between TY2024 and TY2025 with no line-set change. Nothing in the repo reads it.
* `.venv` and `design/forms/*.pdf` are both absent from an isolated worktree (gitignored). Absolute paths to the main tree's `.venv/bin/python` worked; worth one line in whatever the port runbook becomes.

---

## 5. Runbook claims refuted, confirmed, and left untested

| claim | verdict |
|---|---|
| **22** *"85 hand-edits for a 17-form year"* (the five bindings) | **REFUTED — 0 of 5.** But the aggregate did not improve: 6 per `(stem, year)` now against 5 then (F10). |
| **23** *"1 of 5 is year-generic today (`map_pdf_conformance.rs`)"* | **REFUTED — 4 of 5**, each observed red on a planted defect. The exception is the doc-comment check (F2). |
| **9** *"`xtask extract-geometry` — broken on drafts"* | **REFUTED — fixed.** The draft cover sheet is dropped and pages renumbered; both the final and the draft fixtures reproduce byte-for-byte. |
| **18** *"M-star — the check that would have caught `line1a`/`line1b` at commit time"* | **REFUTED — it exists and it is M.** Both directions, over the glob. An extra key reds 5 tests. |
| **24** *"`Coverage::quoting(year)` re-verifies each instruction against the new extract and REDS on a carried-forward sentence"* | **PARTLY REFUTED.** True of the mechanism, false of the deployment: 26 of 27 blocks quote 2024, and a new year moves none of them (F3). |
| **10** *"48/48 hold; no test says so"* | **PARTLY REFUTED.** No test states that literal equality, but `map_rows` plus the hash-keyed geometry join enforce the chain over the glob, with kills. |
| **11** *"needs a 2-row alias table"* | **STALE.** The 2 rows survive but are now held equal to each map header's `irs_stem` by test; for a non-aliased stem the step is a copy. |
| **4** *"`xtask authority-manifest --regenerate`"* | **REFUTED twice:** wrong flag name (F7), and the command refuses outright in a tree without all 125 PDFs, so the step is not M (F6). |
| **7** *"wired only for the 1-row `FORMS` registry, so the 64 committed extracts have no regeneration command"* | **CONFIRMED and worse.** 126 extracts now; 58 name a command that does not exist, 52 name nothing (F8). |
| **2** *"no committed script; `archive_drafts.py` covers drafts only"* | **CONFIRMED** (the draft URL is hardcoded at lines 131 and 754). |
| **8** *"absent"* | **CONFIRMED.** `Entry` has no extract hash. |
| **1, 5, 19, 20, 21** (the H steps I did reach) | **tags right.** Costs here were small because the revision is constants-only; section 3 records each. |
| **16, 17** (adjudicate a witness disagreement; is a moved line the same line) | **UNTESTED — no disagreement and no move arose.** The runbook calls these load-bearing and this rehearsal says nothing about them. A form with filing-status blocks (f8959) or a renumber (Schedule 1-A) is needed to exercise them. |

---

## 6. What I did NOT complete, and why

1. **Steps 16 and 17 were never exercised.** f8995a/2025 is 0 added / 0 removed / 0 moved, so the two H steps the runbook says *"are precisely where this repo's year-port defects have actually come from"* had nothing to adjudicate. **The cheapest target does not rehearse the expensive steps.** A second rehearsal on a *renumbered* prior-year pair would be worth more than this one was.
2. **The form was never filled.** No TY2025 `FullReturnParams` (owner ruling: TY2025 is never filed), so no packet, no read-back, no oracle. "Wired" is proven; "prints the right number in the right box" is not (F17).
3. **Step 8 was not done** — there is no extract-hash slot to write to.
4. **The port machine of section 4 was not built.** `forms fetch / extract / geometry / delta / port / wire --check / port-status` remain unbuilt; `port-status <prior-tag> <new-tag>` and `authority-refresh --check` exist and both do less than the section 4 table describes. Out of scope for a rehearsal.
5. **The `cite_check::FORMS` row plus duplicate extract pair for (f8995a, 2025) were not created.** I took the excuse path and labelled it a finding (F5) rather than grow the second extract root that design r2 section 9 exists to retire.
6. **`cover_pages` and `page_src`** were not added to the geometry fixture (design section 4's proposal, still unbuilt).
7. **Nothing was committed or pushed.** The worktree holds 9 modified and 4 new tracked-path files; `design/forms/` also holds 126 copied gitignored PDFs and 2 I fetched. All of it is throwaway.

---

## 7. If only three things get done before January

1. **F2** — the glob-walking doc-comment gate. A TY2026 map with a carried-forward TY2025 sentence is green today, and that is the one defect class CLAUDE.md opens with.
2. **F4** — re-plant the `year_record` kill so it survives a complete year. Otherwise the first port of each remaining form quietly disarms a B1 instrument, and the tempting fix at that point is deletion.
3. **F10** — generate the four transcription edits in `line_set.rs` from the headers, and confine the `f6251_revision` match to its own schema. That is what turns step 22's honest answer from *"the bindings are gone but the count went up"* into *"a new year is a data change."*

And one sentence for the runbook itself: **step 12 before step 11, step 1 writes `YEAR.toml`, and steps 11 through 22 are one commit.**
