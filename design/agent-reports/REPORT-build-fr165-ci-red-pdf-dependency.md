# REPORT — FR-165: the 6 `xtask::form_delta` tests are hermetic. CI's `test` job is green.

**Agent:** opus, own worktree (`.claude/worktrees/agent-a9f310e7358b6e6a5`), `CARGO_TARGET_DIR=<worktree>/target-fr165`.
**Status:** done, **not committed, not pushed** — worktree left dirty. **6 files changed** (named in §7).
**Gate:** `cargo nextest run --workspace` **3649 tests run: 3649 passed, 12 skipped**; clippy `-D warnings`
clean; `cargo fmt --all --check` clean.

---

## 1. §5 first: the brief's premise is REFUTED, and the fix is much smaller than it assumed

> ★ *"Specifically worth testing: whether the **geometry** JSONs already carry the field names, in which
> case the new fixture may be unnecessary and the fix is smaller than this brief assumes."*

**They do, exactly and completely. No new fixture was created.**

`design/forms/geometry/<stem>.json` already holds `boxes[]`, one entry per AcroForm widget, each with a
`name` — and `form_geometry::extract` builds those boxes from **the same**
`btctax_forms::testonly::collect_fields(&doc)` call that `form_delta::field_set` was making on the PDF.
Measured 2026-09-13 over **all 70** committed fixtures, comparing `boxes[].name` against
`xtask dump-fields <pdf>` on the PDF `pdf_for` resolves (bundled template preferred, else the archive):

| measurement | result |
|---|---|
| fixtures compared | **70 of 70** (every committed fixture; every one has a resolvable PDF on this box) |
| names only in the PDF | **0**, on all 70 |
| names only in the fixture | **0**, on all 70 |
| `boxes.len()` vs the PDF's whole AcroForm field count | **equal on all 70** (so zero rect-less drops) |
| fixtures whose sets differ in any way | **0** |

Independently re-confirmed afterwards by the new checker itself, over the real PDFs:

    extract-geometry: 70 committed fixture(s) — 70 reproduce byte-for-byte, 0 rewritten, 0 unresolved
    extract-geometry: OK — every committed observation is exactly what its own PDF produces, and every
    AcroForm field the PDFs declare is in one.

So the direction in §2 of the brief — *"commit a derived field-list fixture per archived form"* — would
have committed a **71st..140th artifact that is a strict subset of one already committed**, and given
`form_delta` a third artifact to keep in sync. **I did the smaller thing:** `field_set` reads the
geometry fixture the label axis was **already** reading. Both of `form_delta`'s axes now rest on ONE
committed observation instead of two, which is better than the brief's design rather than merely
cheaper: there is no second fixture that can drift from the first.

### The brief's other premises, checked

* **"All 6 failures share one cause."** TRUE. Reproduced in my worktree before any edit —
  `209 passed; 6 failed`, identical to CI's counts — and every one of the six panics traces to
  `field_set`'s `no PDF found for …`, directly or through `compute`. Confirmed by the fix: one function
  changed, all six green.
* **"The extract genuinely lacks field spellings."** TRUE, and not worth re-deriving: the text layer is
  `pdftotext` output and cannot carry AcroForm FQNs at all.
* ✗ **One sub-claim in the fact table is imprecise, and it changes the predicted failure set.** *"PDFs:
  125 on disk, 0 tracked"* is true of `design/forms/`, but the repo tracks **73** PDFs elsewhere —
  `crates/btctax-forms/forms/<year>/*.pdf` (38) and `legal/primary-sources/**` (35) — and `pdf_for`
  resolves the **bundled** ones FIRST. So some stems did resolve in CI and some did not, which is why the
  failures are a mixture of *"no PDF found for f6251--2026-DRAFT"* (an archived side) and *"no PDF found
  for f1040s1--2025"* (a prior side with no bundled template). Nothing about the fix depends on it, but a
  reader of the brief would predict the wrong set.

---

## 2. What changed

### (a) `crates/xtask/src/form_delta.rs` — the one-line root cause

BEFORE (the FR-165 defect, `form_delta.rs:64`): the field spellings came from the PDF's AcroForm —

    let pdf = pdf_for(stem).ok_or_else(|| format!("no PDF found for {stem}"))?;
    let bytes = std::fs::read(&pdf)?;
    let doc = btctax_forms::testonly::load(&bytes)?;
    Ok(collect_fields(&doc)?.into_iter().map(|f| f.fqn).collect())

AFTER: they come from the committed geometry observation, the same one the label axis already reads —

    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    Ok(g.boxes.into_iter().map(|b| b.name).collect())

**No fallback.** There is no PDF read anywhere on the test path, so the committed fixture is the thing
under test on every machine — the trap §2.3 of the brief names. `pdf_for` survives for one job only:
`tests::real_archive`, the ARCHIVE oracle (see §6).

Its doc comment carries the measurement, and states the **one thing a geometry fixture structurally
cannot hold**: `extract-geometry` drops a field with no widget rect (no position, so no label join).
There are 0 such fields today, and that is now *asserted* rather than assumed — see (b).

### (b) `crates/xtask/src/form_geometry.rs` — the verification half, on the A4 pattern

`extract` was one function that observed and wrote. Split into:

* `observe(stem) -> Observation` — the PDF read. `Observation` carries the geometry **plus**
  `acroform_fields` and `dropped_without_rect`, so the drop leaves the reader as data rather than as a
  `println!` nobody reads.
* `serialise(&Geometry) -> String` — the exact committed bytes, used by the writer **and** the checker,
  so they cannot disagree about formatting and report whitespace as a changed form.
* `reproduces(stem, committed, &obs)` — byte equality.
* `unrepresentable_fields(stem, &obs)` — the separate finding that **regenerating cannot repair**.
* `verify(…)` — both, in that order. Pure: the observation is a parameter, so the kill needs no PDF.
* `committed_stems(root)` — the `--all` denominator, enumerated from the directory, never a list.
* `run(args)` — `extract-geometry <stem> | --all [--check]`, mirroring `forms extract --all --check`.
  An absent PDF is **unresolved**, never a skip. Zero fixtures is a **refusal**, not an OK.
* `extract` / `write_fixture` — unchanged behaviour, and `--all` without `--check` adopts the observation
  it already made rather than reading the PDF a second time.

`--check` is deliberately **not a test**: it needs the PDF, and a test that needed the PDF would be
FR-165 again.

### (c) `crates/xtask/src/main.rs`

The `SUBCOMMANDS` row becomes `("extract-geometry", &["--all", "--check"], "<stem> | --all [--check]")`
and the dispatch arm calls `form_geometry::run(&args[1..])`. Verified live: a bare stem still works, no
args prints usage and exits 2, and an unknown flag is refused by name — *"unrecognised argument
\"--regen\" — it accepts only --all --check."*

### (d) the ignore file's comment (lines 68-87) — the false claim, fixed

It said the committed text layer *"IS what the tests read"*. It now states that there are **two**
committed derived observations, names each one's checker, records that the old sentence was false for
these six tests and cost eight days of red CI, and tells the next author to add a third artifact here if
one appears. The `design/forms/**/*.pdf` rule **stays** (now line 87). **No PDF was committed.**

### (e) `design/forms/README.md` — the same false sentence, one file over

*"the conformance tests read the extract, so they need no PDF and no network."* Same omission, same week.
Replaced with a two-artifact table (extract + geometry, each with the command that holds it to its PDF)
and the same warning.

### (f) `FOLLOWUPS.md`

FR-165 closed with its measurement; FR-172 and FR-173 filed (§6).

---

## 3. B1, direction 1 — a checkout with NO PDFs must pass

A pristine tree was built at `/scratch/fr165-clone` from the tracked file list (working-tree contents, so
the patch is included; nothing ignored copied), then initialised as a repository and committed so the
tracked-blob checks have something to read:

    tracked files listed:    2163
    files copied:            2163
    pdfs under design/forms: 0
    geometry fixtures:       70

`cargo test -p xtask --locked` with `CI=1` — CI's own environment, because `harness_check::should_check`
returns false when `CI` is set, which is how CI skips the hooks-installed gate:

    test result: ok. 217 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 19.93s

**And the exact command CI runs**, `cargo test --workspace --locked` with `CI=1`, in the PDF-less
worktree — 153 test binaries, totalled from the `test result` lines:

    passed=3649 failed=0 ignored=13     (exit 0)
    ...of which the xtask binary:
    test result: ok. 217 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 19.94s

**Baseline, same tree, before the change:** `test result: FAILED. 209 passed; 6 failed; 1 ignored` —
identical to CI's reported counts, with all six panicking on a missing PDF.

★ Two side effects worth recording: the suite is **215 → 217 tests** (two new), and the xtask binary went
**89.6s → 19.9s**, because one JSON parse replaced 42 lopdf parses per `port_status` run.

Without `CI=1` the pristine tree fails exactly one test,
`harness_check::this_working_tree_has_the_harness_installed` — a freshly initialised copy has no wired
hooks. That is the check working as designed (*"it is supposed to fail on a fresh clone — that is the
whole mechanism"*), and it is why CI sets `CI`.

## 4. B1, direction 2 — the checker must RED on a planted fixture edit, GREEN on revert

Run in a tree that **has** the 125 PDFs. Single stem first, because the byte counts are the interesting
part:

    === PLANT: rename one AcroForm box in the committed fixture (f1_10[0] -> f1_99[0]) ===
    15507c15507
    < "name":"topmostSubform[0].Page1[0].f1_10[0]"}
    ---
    > "name":"topmostSubform[0].Page1[0].f1_99[0]"}
    --- xtask extract-geometry f6251--2025 --check ---
    xtask extract-geometry: f6251--2025: the committed geometry fixture is NOT what its PDF produces
    (190432 bytes committed, 190432 regenerated; 2553 words, 62 boxes, sha256:6995bfd2... observed).
    Review the change before regenerating — a changed observation means the DOCUMENT changed.
    exit=1

    === REVERT ===
    extract-geometry: f6251--2025 reproduces byte-for-byte from its PDF.
    exit=0

★ **190432 committed, 190432 regenerated** — a length check would have been blind to this plant. The
comparison is bytes.

Then the mode a developer actually runs, plus the vacuity guard:

    === PLANT: rename one AcroForm box in a committed fixture (f8949--2026-DRAFT) ===
    extract-geometry: 70 committed fixture(s) — 69 reproduce byte-for-byte, 0 rewritten, 1 unresolved
      ★ f8949--2026-DRAFT: the committed geometry fixture is NOT what its PDF produces (120679 bytes
        committed, 120677 regenerated; 1194 words, 202 boxes, sha256:891d869c... observed). ...
    xtask extract-geometry: 1 of 70 geometry fixture(s) could not be verified.
    exit=1

    === REVERT ===
    extract-geometry: 70 committed fixture(s) — 70 reproduce byte-for-byte, 0 rewritten, 0 unresolved
    extract-geometry: OK — every committed observation is exactly what its own PDF produces, and every
    AcroForm field the PDFs declare is in one.
    exit=0

    === VACUITY GUARD: --all --check with no fixtures at all ===
    xtask extract-geometry: no committed geometry fixtures under
    /scratch/fr165-clone/design/forms/geometry — refusing to report success over nothing
    exit=1

**The other direction of "still checked where the PDF exists":** the whole xtask suite in the tree **with**
the 125 PDFs, same code — `test result: ok. 217 passed; 0 failed; 1 ignored` (19.96s). Identical to the
PDF-less run, which is the coherence claim FR-173 in §6 rests on.

### The in-suite B1 pairing (hermetic, so CI runs it)

`form_geometry::tests::the_fixture_checker_reds_on_every_planted_fixture_defect` synthesises an
`Observation` and asserts the comparison reds on **six** byte plants — a renamed box, a moved coordinate,
an edited word, a rewritten `pdf_sha256`, a deleted box, a dropped trailing newline — each preceded by
`assert_ne!(edited, committed)` so a plant that failed to plant cannot pass, plus a control asserting the
unmodified bytes verify. It then asserts `unrepresentable_fields` fires on `dropped_without_rect: 1`
**and not** on `0`, so that oracle cannot answer the same for both.
`the_all_walk_enumerates_every_committed_fixture` holds the `--all` denominator to the directory.

## 5. The two plant-based tests, re-run — and one of them had an unwatched branch

Both pass:

    test form_delta::tests::the_work_list_checker_reds_on_every_planted_row ... ok
    test form_delta::tests::port_status_prints_the_committed_work_list ... ok
    test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 216 filtered out

Passing is not evidence that a plant still bites, so each was **mutation-verified against the new code
path**:

| mutation | expected | observed |
|---|---|---|
| `port_status_over` ignores the surface it is given (`for form in &emitting_surface()`) | `port_status_prints_the_committed_work_list` reds | **RED** — *"a printer that dropped a stem must fail the surface equality"* |
| the moved-count comparison neutralised (`Some(n) if false && n != d.label_moved.len()`) | `the_work_list_checker_reds_on_every_planted_row` reds | **RED** — *"a cell off by one: []  left: 0  right: 1"* |
| **the counts comparison neutralised (`if counts != (common, added, removed)` becomes `if false`)** | should red | ✗ **GREEN — nothing noticed** |

★ **That third row is a real finding and it sits exactly on top of FR-165.** The test's first assertion
calls itself *"a cell off by one"* while planting the **moved** cell; **no planted row had ever exercised
the `common`/`added`/`removed` comparison at all.** It matters now because FR-165 replaced the *source*
of those three numbers — they come from the geometry fixture's box names instead of the PDF's AcroForm —
so the one comparison that would notice the new source disagreeing with the committed document was the
unwatched one.

Fixed in place with three new plants (COMMON, ADDED, REMOVED each off by one against `f6251`), and
re-verified both ways:

    # with the new plants, unmutated
    test form_delta::tests::the_work_list_checker_reds_on_every_planted_row ... ok

    # with the counts comparison neutralised to `if false`
    test form_delta::tests::the_work_list_checker_reds_on_every_planted_row ... FAILED
    panicked at crates/xtask/src/form_delta.rs:698:9:
    assertion `left == right` failed: the COMMON cell off by one: []
      left: 0
     right: 1

The first assertion's message is also corrected to *"the MOVED cell off by one"*, which is what it plants.
**No test was weakened; one was strengthened where my change moved the ground under it.**

Finally, the printer's real output in the PDF-less tree reproduces the committed
`design/TY2026_WORK_LIST.md` row for row — all 15 numeric rows identical (`f6251 | 62 | 0 | 0 | 0 |
unchanged`, `f1040s1 | 72 | 1 | 1 | UNWITNESSED | port`, and the rest) and all 6 excused rows' claims
correct — with **no PDF present at all**.

## 6. Follow-ups filed

**FR-172 — a 187 KB IRS PDF is COMMITTED at the repo root under the filename `--out`. Minor. Owning
phase: NOW, with any commit touching the ignore file.** Found only because `tar -T` refused the filename:
the tracked file list contains an entry literally named `--out`; `file` says *PDF document, version 1.7,
2 page(s)*; 187,143 bytes; added in `a1c6fc849` (2026-09-05 — the same day FR-165's trigger landed).
Plainly an `xtask label-proof ... --out` whose flag became a path, swept in by an `add -A`. **It is a
committed IRS PDF, the one thing the ignore rule exists to prevent** — the rule is path-scoped to
`design/forms/**/*.pdf`, so nothing guarded the repo root. Not fixed here: removing it is a commit, and I
was told not to commit. Worth considering whether the rule should become `*.pdf` with `!` exceptions for
the bundled templates, so a stray PDF anywhere is refused by default.

**FR-173 — `form_delta`'s excused-arm predicate and its ARCHIVE oracle now read different artifacts.
Minor. Owning phase: with the FR-136 borrowed-absence work.** `compute` now fails on a missing **geometry
fixture** while `port_status_over`'s excused arm and `tests::real_archive` still answer *"is this side
present"* from `pdf_for`. **Measured both ways (§4): every stem on the emitting surface answers
identically with and without the PDFs**, because each prior side in the excused table has a *committed*
bundled template and each absent side has neither artifact — a latent seam, not a live defect, and it is
stated in `real_archive`'s doc comment rather than hidden. The state that would diverge is a stem whose
PDF has been archived but whose fixture has not yet been extracted. Deliberately not collapsed inside
FR-165: the two predicates are what the FR-136 declared-archive plants are built around, and the brief
puts that file out of scope.

Also corrected in place rather than deferred, because FR-165 made them false: `Unwitnessed::
GeometryFixtureMissing`'s operator-facing message and the `NoFixture` banner both said *"one side has NO
geometry fixture — run `xtask extract-geometry <stem>`"*. A missing fixture is now a hard `Err` out of
`compute` that names the file and the command; what reaches that variant is a fixture that exists and
yields no printed label column. The name is kept (it is load-bearing in the review history) with the
narrowing stated in the source — the honest-boundary option of the derive rule.

## 7. Files changed — worktree left dirty, nothing committed

    .gitignore                        |  20 +-
    FOLLOWUPS.md                      |  35 ++++
    crates/xtask/src/form_delta.rs    | 111 ++++++++---
    crates/xtask/src/form_geometry.rs | 395 ++++++++++++++++++++++++++++++++++++--
    crates/xtask/src/main.rs          |  15 +-
    design/forms/README.md            |  19 +-
    6 files changed, 543 insertions(+), 52 deletions(-)

**Out of scope and untouched, as instructed:** `scripts/oracle/*`,
`crates/btctax-core/tests/golden_returns.rs`, `crates/btctax-forms/tests/map_pdf_conformance.rs`, and
`.github/workflows/ci.yml` — CI runs what it ran; the dependency was the defect.
