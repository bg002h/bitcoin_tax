# REVIEW — `c6c4a7dc` (archive reconciliation 7 → 0, tickle retirement)

Reviewer: sonnet, one agent, no subagents. Scope: the commit itself, per
`BRIEF-archive-resolution-review.md`. Not re-litigating the decision (that is
`design/agent-reports/2026-09-04-archive-tickle-fable.md` + `31cc7d36`).

## VERDICT

**0 Critical / 0 Important / 1 Minor / 3 Nit.**

Nothing broke. Every one of the ten deleted archive files (`design/forms/periodic/`
×6, `legal/primary-sources/irs-forms/` ×5 counting binaries+manifest rows, minus
overlap) has its citation and bytes still reachable from `legal/SOURCES.md`; the
guard that replaces the tickle is strictly correct and was never actually weaker
than claimed (the deleted `assert!` was already redundant with the surviving
`assert_eq!`, confirmed by reading both the old and new code side by side); and
`make check`-equivalent commands are green, live, independent of the commit's own
report. The four findings below are all documentation/comment staleness — no
guarantee quietly stopped being enforced, and no gate was defanged.

## FINDINGS

### Minor — `CONTINUITY.md:424` contradicts the same file's own "✅ CLOSED" line

`git blame` dates this row to `7bde148` (2026-07-30), well before `c6c4a7dc`, and
`c6c4a7dc`'s diff to `CONTINUITY.md` does not touch it — but it is exactly the
kind of doc-drift sub-question 4 asks about, because it is a live, tracked claim
about the very mechanism this commit deleted:

> `**4**` \| `ARCHIVE_RECONCILIATION_REVIEW_BY = "2026-08-13"` (`archive_check.rs:174`)
> — re-decide the residual archive duplication or consciously reset the date with
> a written reason. ... \| hours \| ⬜ **DATED — reds the WHOLE SUITE once passed**,
> blocking everything else. Owner decision. \|

This is in "THE RANKED BACKLOG" table, inside "THE RANKED BACKLOG — from the
2026-07-30 'what next' recon." `ARCHIVE_RECONCILIATION_REVIEW_BY` no longer
exists (deleted by `c6c4a7dc`), `archive_check.rs:174` is no longer that
constant (the RETIRED comment block lives there now), and the row's own table
convention — every other row in the same table gets `~~struck through~~` plus a
✅/❌ marker once resolved (rows 1, 2, 3, 5, 7 all are) — was not applied here,
even though `c6c4a7dc` correctly struck through and closed the *other* two
mentions of this exact item in the same file: the RESUME POINT § step 5
(`~~archive review-by 2026-08-13~~ — ✅ CLOSED 2026-09-04...`) and step ③
(`~~Reconcile the archives~~ | ✅ DONE ... RESOLVED 2026-09-04...`). Three places
in one file describe the same fact; two were updated, one was not, so the file
now disagrees with itself.

This does not affect any gate, test, or guarantee — `archive-check` and
`authority-manifest` both report the resolved state correctly and independently
of this table (verified live, see below). The risk is purely a future reader of
`CONTINUITY.md`'s backlog table believing there is still an owner decision
pending on a dated, suite-blocking gate that no longer exists.

**Smallest fix:** strike through row 4 and replace the status cell, e.g.
`~~4~~ | ~~`ARCHIVE_RECONCILIATION_REVIEW_BY`~~ | ✅ **CLOSED 2026-09-04** — see
RESUME POINT §0 step 5.` One line.

### Nit — `crates/xtask/src/authority_manifest.rs:774` cites a path this commit deleted

`kind_is_derived_from_the_path`'s doc comment says "Anchored to real repo
paths," and one of its 8 `(path, want)` cases is
`"legal/primary-sources/irs-forms/Instructions_8949.pdf"` → `Kind::Instructions`
— a file `c6c4a7dc` deletes in the same commit. This test is pure string-pattern
matching (`Kind::of` never touches the filesystem — confirmed: `cargo nextest
run -p xtask -E 'test(kind_is_derived_from_the_path)'` passes, and would pass
even with the file absent), so nothing is functionally broken. But the doc
comment's claim is now false for this one entry. Not introduced by `c6c4a7dc`
as a line-diff (this test wasn't touched), but `c6c4a7dc` is what makes the path
non-real.

**Smallest fix:** swap the literal to a path that still exists, e.g.
`legal/primary-sources/irs-forms/Instructions_1099-DA.pdf` (same `Kind::Instructions`).

### Nit — `crates/xtask/src/authority_conflicts.rs:121-123` doc comment now describes a call that doesn't happen

```
/// ★ `pub(crate)` because `archive_check` tickles the archive reconciliation on the same mechanism.
/// One implementation of "what day is it", not two that can drift.
fn today() -> String {
```

`c6c4a7dc` narrowed `today()` from `pub(crate)` to private (correct — its only
remaining caller is inside `authority_conflicts.rs` itself, for
`AUTHORITY_CONFLICTS.md` review-by checks; confirmed via
`grep -rn "authority_conflicts::today\|fn today" crates/xtask/src/` — one hit,
the definition). The doc comment justifying the now-removed `pub(crate)` and
describing `archive_check`'s now-deleted call was left in place, describing a
cross-module mechanism that is gone.

**Smallest fix:** delete the `★ pub(crate) because...` sentence (2 lines).

### Nit — `FOLLOWUPS.md`'s updated §G-12 unblock command sources from the URL pattern this commit's own rationale rejects

`c6c4a7dc` repointed the destination of the (not-yet-executed) Form 8275-R fetch
from `design/forms/periodic/f8275r.pdf` to `design/forms/2025/f8275r--2025.pdf`,
correctly following the new periodic-forms-live-in-year-directories convention
— but left the source as `https://www.irs.gov/pub/irs-pdf/f8275r.pdf`, the
"moving" URL this same commit's Group-A rationale explicitly rejects ("its URLs
were the moving `irs-pdf/` ones the hybrid rationale rejects for forms").
Verified live: `curl -sSL -o /dev/null -w '%{http_code}'
https://www.irs.gov/pub/irs-prior/f8275r--2025.pdf` → **404**;
`.../irs-pdf/f8275r.pdf` → **200**. So the command isn't wrong exactly — the
IRS does not host a year-pinned archive of this form at `irs-prior/`, and
`design/forms/README.md`'s just-updated sourcing convention ("sourced from
`irs-prior/{stem}--{TY}.pdf` when the IRS holds that edition, else from the
bundled runtime asset") does not name this third case (no `irs-prior` edition,
no bundled asset) that the command actually falls into. This is speculative,
future, blocked-on-an-owner-decision work (§G-12), not live behavior — nobody
has run this command.

**Smallest fix:** either extend the README table's sourcing rule to name a third
case ("else the current `irs-pdf/{stem}.pdf` edition, hashed at fetch time"), or
leave as-is with a one-line comment in FOLLOWUPS.md noting `irs-prior` 404s for
this form. Not urgent — filed here rather than as a new follow-up per the
brief's "do not propose new checkers" scope.

## WHAT I VERIFIED AND HOW

All commands run at `c6c4a7dc` (working tree clean before and after; no tracked
file modified; the one scratch download was written to and deleted from the
scratchpad dir).

1. **`cargo run -p xtask -- archive-check`** →
   `archive-check: no primary source outside the 5 accounted-for tree(s)` /
   `archive-check: 3 accounted-for archive(s) — hybrid, decided 2026-07-30;
   duplicates reconciled 2026-09-04 and pinned at 0 by authority-manifest`,
   exit 0. Matches the brief exactly, confirmed live rather than trusted.

2. **`cargo run -p xtask -- authority-manifest`** →
   `102 entries — 42 committed, 60 note-only` / `0 document(s) archived under
   more than one path (pinned 0 ...)` / `OK — every entry resolves and every
   source is listed`, exit 0.

3. **`cargo run -p xtask -- authority-manifest --regen`** → `regenerated 102
   entries`, and `git status --porcelain` + `git diff --stat` **empty**
   afterward — the committed `MANIFEST.json` is byte-identical to a fresh
   regen. This is a stronger, independent check than the brief's list covers:
   it directly validates the FR-25 gap is not *currently* masking a live drift
   (only that nothing stops a *future* one — FR-25's own point).

4. **`cd legal && sha256sum -c SHA256SUMS`** → 42 lines, all present, `grep -v
   ": OK"` on the output is empty (i.e. zero non-OK lines) — 42/42, matching
   `legal/SOURCES.md`'s stated count and `find legal/primary-sources -type f |
   wc -l` = 42.

5. **`cargo nextest run -p xtask`** → 66 tests run, 66 passed, 1 skipped
   (0 failed). Includes `duplicate_source_groups_may_only_shrink`,
   `every_manifest_entry_resolves_and_hashes_true`,
   `every_accounted_for_tree_still_exists`,
   `this_repo_has_no_unaccounted_primary_source`. Also ran the guard test in
   isolation: `cargo nextest run -p xtask -E
   'test(duplicate_source_groups_may_only_shrink)'` → 1 passed.

6. **`cargo clippy -p xtask --all-targets`** and **`cargo fmt --all --check`**
   → both clean, exit 0.

7. **Read the guard's current code** (`authority_manifest.rs:805-819`) and
   diffed it against the pre-commit version: the deleted `assert!(dups.len()
   <= DUPLICATE_SOURCE_GROUPS)` was **redundant**, not load-bearing — the
   `assert_eq!` that survived was already present *before* this commit and
   already reds on both a rise and a fall by construction. So "did the guard
   survive intact" is answered by code reading, not just by trusting the
   commit message: the guarantee was never weakened at any point, only the
   dead/misleading duplicate assertion was removed. The surviving
   `assert_eq!`'s message branches correctly on `dups.len() >
   DUPLICATE_SOURCE_GROUPS` vs `<` and appends the `dups.iter()` document list
   in both cases.

8. **Live network round-trips**, independent of the two the brief already
   verified: `f1040s1--2024.pdf` (the defect-1 repair) — `curl -sSL
   https://www.irs.gov/pub/irs-prior/f1040s1--2024.pdf` → HTTP 200,
   `sha256sum` = `2fe9bc204a82a037a50b21c6eda9949e266eafa423e604f312bc177fc721b552`,
   **exact match** to the note and to `MANIFEST.json`. Also spot-checked
   `f8283--2025.pdf` (one of the brief's three pre-verified round-trips,
   reconfirmed independently) — HTTP 200, hash `389ab1b7c01b...`, exact match.
   Both scratch files deleted after hashing.

9. **Sub-question 1 (consumer redirection), full sweep**: `git grep` across the
   whole tree for the deleted binaries' filenames, the deleted
   `legal/text/irs-forms/*.txt` extract paths, and `forms/periodic`. Every hit
   outside the brief's five named files falls into one of: (a) a bare filename
   used as a `Shape::witness` string-pattern example in `archive_check.rs`
   (`Form_8949.pdf` as a shape-matching exemplar — `classify()` and the shape
   functions are pure string matchers, never touch disk; confirmed by reading
   `human_readable_form()`, `SHAPES`, and the `every_shape_fires_on_its_witness`
   /`a_third_archive_at_a_novel_path_is_caught` tests, both passing), (b) the
   two agent-report files documenting the decision itself (deliberate
   records), or (c) the one stale test path noted above (Nit). No runtime crate
   code (`crates/btctax-forms`, `crates/btctax-core`, etc.) references any
   deleted path — swept with `grep -rln` across `crates/`.

10. **Sub-question 3 (authority reachability)**: for all five retired
    documents, confirmed programmatically that `legal/SOURCES.md`'s new table
    cites a `design/forms/2025/*.pdf.txt` note whose `sha256`/`bytes` match the
    prior `legal/SHA256SUMS` row exactly, that the corresponding
    `design/forms/extract/*.txt` file exists (all 5, via `test -f` + line
    counts), and that `legal/text/irs-forms/` now contains only the two
    surviving `1099-DA` extracts, matching the only two `MANIFEST.json` entries
    that still point into that tree (checked via a small Python script over
    the JSON, not by hand-counting).

11. **`design/no-testimony/CONSULT-architect-fable.md`** and
    **`legal/_provenance/fetch_log.tsv`**: read enough of each around their
    grep hits to confirm they are exactly what the brief's judgment says — a
    captured transcript from an earlier session (embedded JSON with a
    timestamp of `2026-08-01`) and a literal fetch log (status/bytes/hash/url
    columns for a fetch that happened), respectively. Both genuinely
    unmodifiable historical records; the brief's judgment holds for these two.

## WHAT I COULD NOT CHECK

- Did **not** re-run the full workspace `make check` (2765 tests, ~minutes);
  relied on the brief's pre-verified figure plus my own targeted `cargo
  nextest run -p xtask` (66/66) and `cargo clippy`/`cargo fmt --all --check`
  (both clean) as corroboration scoped to the crate this commit actually
  touches. This does not re-verify the workspace-wide count itself, only that
  nothing in the touched crate is red.
- Did not re-run the B1 kill-test (planting a duplicate, observing both
  directions red) — the brief lists this as already machine-verified, and
  re-planting risks the exact collision hazard the commit message itself
  documents (the first kill attempt destroyed a real tracked note). Instead I
  verified the guard's logic by reading the code and confirming the surviving
  `assert_eq!` predates this commit and is unconditionally correct.
- Did not attempt to fetch `irs-pdf/f8275.pdf`, `irs-pdf/i8275.pdf`, or the
  bundled-asset round-trip for `f8283--2024` — these three were already
  verified by the controller per the brief ("Round-trip before deletion:
  `irs-prior/{f8275--2024,i8275--2024,f8283--2025}` all HTTP 200 and
  hash-exact"); I independently reconfirmed only `f8283--2025` as a spot check
  (finding 8 above) rather than repeat all three.
- Did not check whether other repositories/CI configs outside this repo (e.g.
  a hypothetical external mirror) reference the deleted paths — out of scope,
  no such external system is known to exist for btctax.
