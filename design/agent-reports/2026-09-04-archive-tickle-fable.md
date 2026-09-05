# The archive tickle — verdict (Fable, 2026-09-04)

**Brief:** `design/agent-reports/BRIEF-archive-tickle.md`. One agent, no subagents, nothing
committed. Every count below that is not in the brief was measured in this session; the commands
are named where the number matters.

## 1. VERDICT

**Do the residue now, and retire the tickle with it — bridged by a second, terminal reset whose
recorded reason is a decision rather than a deferral.** Concretely: (1) land a one-line reset
`2026-08-28 → 2026-09-11` whose log row states the resolution of both duplicate groups and names
this as the *execution* deadline, not a third window for deciding — this is the only honest change
that unblocks every commit in the repo today; (2) persist this report; (3) land the resolution:
retire `design/forms/periodic/` (group A) and delete (B)'s five `irs-forms/` form copies (group B),
`DUPLICATE_SOURCE_GROUPS` 7 → 0, and **delete `ARCHIVE_RECONCILIATION_REVIEW_BY`, its test and the
`run()` branch** — stated plainly, that is removing the test, and it is not the mute the doc warns
of because it happens *after* the tickle's subject is gone, the guard that replaces it (a pin at 0)
is stronger than a date, and the reset log stays in the file with a RETIRED line. A second reset
*alone* is the decoration; a second reset *as the last entry before retirement* is what the log's
"two entries is a decision" sentence describes.

## 2. WHY

- **Of the gate's two stated outs, only one is open.** `run()` and the test both say *"retire a
  tree, or set a new date with a reason."* No tree can be retired: the hybrid decision of
  2026-07-30 (`CONTINUITY.md:788`) made three the *end state*, and `the_archive_count_may_only_shrink`
  pins it with `assert_eq!(…, 3)` (`archive_check.rs:539`). So the reconciliation the constant was
  set for (`:172-173`, "step ③, the very next piece of work") already happened; what the date now
  guards is the 7-group residue, and a date is honest for that only as a deadline for executing a
  decision already made.
- **The residue is two chores, not two decisions, and the measurements say which way each goes.**
  Group A: no code resolves through `periodic/` (the only non-doc mentions are the manifest and one
  `map.toml` comment); its three manifest entries carry `extract: ""` while their notes point at a
  text layer that does not exist (`periodic/f8275.pdf.txt` names `extract/f8275.txt`; the file is
  `f8275--periodic.txt`); its URLs are the moving `irs-pdf/{stem}.pdf` ones, which is exactly what
  the hybrid rationale rejects for forms; and the year directory already holds a revision
  `periodic/` cannot — `extract/f8283--2024.txt` is Rev. December 2023 from the bundled asset while
  `periodic/f8283.pdf` is Rev. December 2025. Group B: nothing in the repo reads
  `legal/text/irs-forms/` (grep over `*.md,*.rs,*.py,*.sh,*.toml`: zero hits), and the two text
  layers of one document are **not the same text** — `extract/i1040sd--2025.txt` vs
  `legal/text/irs-forms/Instructions_Schedule_D.txt` differ by 3,494 lines because (B)'s instruction
  extracts are `-layout` column-interleaved and (A)'s are reading-order, which is the README's own
  rule for instruction booklets. That is the transcribe-discipline harm duplication actually does,
  and it is present today, not hypothetical.
- **Reset #1's reason has been satisfied, so a same-reason #2 is the reflex the log exists to
  expose.** #1 deferred to *"a window when model usage is expected to be more available"*;
  `CONTINUITY.md` now says *"Nothing is in flight."* The gate has fired twice in five weeks, and each
  firing blocks every commit repo-wide (pre-commit → `make check`, `--no-verify` denied) over a
  question with no bearing on a signed 1040. That is the permanently-red gate `HARNESS.md:161-167`
  says must not exist — arriving in fortnightly instalments.
- **After resolution the structural guard is stronger than the tickle.** `duplicate_source_groups_may_only_shrink`
  at pin 0 reds on *any* duplicate, immediately, with no date to push; `strays()` + `KNOWN_ARCHIVES == 3`
  red on any new tree. The in-repo model is `URL_NOT_RECOVERABLE` (`authority_manifest.rs:115-121`):
  emptied, left in place with an EMPTIED note. A dated test whose subject no longer exists is
  decoration by the gate's own definition (`:496-497`), so retiring it is the rule applied, not evaded.
- **The harness forces the sequencing.** While the test is red nothing can be committed — not this
  report, not the pending `CONTINUITY.md`. The unblock must therefore be a change that is honest on
  its own: the terminal reset row. Persist and fold stay separate commits.
- **Cost is an afternoon.** ~1–2 h of deletions and redirects, one mechanical review round,
  905,833 bytes of committed form binaries leaving the tree (they remain in history and are
  re-fetchable from the (A) notes).

## 3. EXACT ACTIONS

Three commits, in this order. Nothing here is folded yet; every count is what the tree will show.

### Commit 1 — the terminal reset (unblocks every commit; owner's decision)

`crates/xtask/src/archive_check.rs:193`: `"2026-08-28"` → `"2026-09-11"`.

`crates/xtask/src/archive_check.rs:185` — append this row directly under row 1 (one line, as the
table requires):

```
/// | 2 | 2026-08-28 | 2026-09-11 | 2026-09-04 | owner | **DECIDED, not deferred — and the last entry.** Both residual groups resolve and the tickle retires with them. Group A: retire `design/forms/periodic/` — no code resolves through it, its notes name a text layer that does not exist (`extract/f8275.txt`), its `irs-pdf/` URLs are the moving ones, and the year directory already holds a revision it cannot (`extract/f8283--2024.txt` = Rev. 12-2023 vs `periodic/f8283.pdf` = Rev. 12-2025). Group B: delete (B)'s five `irs-forms/` form copies per the hybrid rule (forms are note+sha256 in (A)); nothing reads `legal/text/irs-forms/`, and its instruction extracts are `-layout` column-interleaved — the wrong text layer to transcribe from. What changed since #1: the window it waited for has arrived (nothing in flight since the 2026-08-30 push). This date is the EXECUTION deadline for landing that diff; when it lands, `DUPLICATE_SOURCE_GROUPS` is 0 and this constant, its test and the `run()` branch are deleted, leaving this table as the record. There is no #3. |
```

Why 2026-09-11: the smallest window that still fits one review round; if the owner lands commit 3
the same day, the date is a ceiling that was never reached, which is fine. Gate: `make check` green,
`cargo fmt --all --check` clean.

### Commit 2 — persist this report verbatim (this file, its own commit, ~five-line message).

### Commit 3 — the resolution, one commit, gate output in its message

**Step 0 (network, owner's machine, before deleting anything):** the README's round-trip for the
three notes that survive group A — `curl -sL <url> | sha256sum` for
`design/forms/2024/f8275--2024.pdf.txt` (`irs-prior/f8275--2024.pdf`, expect `9b4b82e3…`),
`2024/i8275--2024.pdf.txt` (`irs-prior/i8275--2024.pdf`, expect `a7b4d8b9…`),
`2025/f8283--2025.pdf.txt` (`irs-prior/f8283--2025.pdf`, expect `389ab1b7…`). The manifest records
both URLs against one hash but only `f8995--2025` was ever round-tripped (`design/forms/README.md`).
If a year-named URL 404s, write the `irs-pdf/` URL into *that year-named note* — the path is the
decision, the URL is data — and keep going.

**Group A — retire `periodic/` (6 tracked files):**

1. `git rm design/forms/periodic/{f8275,i8275,f8283}.pdf.txt` (the PDFs are gitignored; nothing
   tracked but the notes).
2. `git rm design/forms/extract/{f8275,i8275,f8283}--periodic.txt` (byte-near-identical to the
   year-named extracts; only `map.toml:84` cites one, as the *wrong* revision).
3. `design/forms/README.md:20` — rewrite the periodic row's location column: a periodic form is
   archived under **the tax year its revision governs**, `{TY}/{stem}--{TY}.pdf.txt`, sourced from
   `irs-prior/{stem}--{revision year}.pdf` when the IRS holds it, else from the bundled asset (the
   `extract/f8283--2024.txt` model). Delete `README.md:42` (`design/forms/periodic/*.pdf.txt`).
4. `crates/btctax-forms/forms/2024/f8283.map.toml:83-84` — cite `design/forms/2025/f8283--2025.pdf`
   / `design/forms/extract/f8283--2025.txt` instead of the periodic pair (same bytes, same point).
5. `FOLLOWUPS.md:1297` — the §G-12 unblock command targets `design/forms/periodic/f8275r.pdf`;
   retarget to `design/forms/2025/f8275r--2025.pdf` so a future session does not recreate the tree.

**Group B — delete (B)'s five form copies (10 tracked files, 905,833 committed bytes):**

6. `git rm legal/primary-sources/irs-forms/{Form_8949,Instructions_8949,Schedule_D_1040,Instructions_Schedule_D,Form_8283_Noncash_Charitable}.pdf`
   and `git rm legal/text/irs-forms/{same five}.txt`. `Form_1099-DA` and `Instructions_1099-DA`
   stay (not duplicated; see §4).
7. `legal/SHA256SUMS:3,4,6,7,8` — delete the five lines (47 → 42). `legal/SOURCES.md:8` "47
   documents" → 42; `:14` "expect: 47 OK" → 42.
8. `legal/SOURCES.md:60-63,66` — do **not** delete the rows; change the File column to the (A) path
   (`design/forms/2025/f8949--2025.pdf.txt` etc.) so the legal-defense index still resolves every
   citation, with the same sha prefixes (they are the same bytes).
9. `legal/_scripts/fetch_sources.sh:48-51,54` — delete the five `dl` lines (or comment each with
   `# → design/forms/2025/…`); a re-run must not recreate files the manifest no longer lists (the
   census would red, correctly, but the script should not set the trap).
10. `legal/research/ADDENDUM_open_questions_verified.md:40` → `design/forms/extract/i1040sd--2025.txt`;
    `:90` → `design/forms/extract/f8283--2025.txt`. `legal/_provenance/fetch_log.tsv` is a log —
    leave it.

**The pin and the manifest:**

11. `cargo run -p xtask -- authority-manifest --regen` — never hand-edit `MANIFEST.json`; entries
    110 → 102. Then `authority_manifest.rs:139` `7` → `0`, and replace the doc table at `:123-138`
    with an `EMPTIED 2026-09-xx` note in the `URL_NOT_RECOVERABLE` style (`:116-121`): what the 7
    were, which way each group went, and that any future duplicate reds the suite with no date to
    push. `:632-633` — drop "may only shrink — CONTINUITY.md §0 step ③" from the printed line.

**Retire the tickle (this is the part that deletes a test — see §1):**

12. `archive_check.rs:193` — delete the constant. `:165-174` — replace the "must be re-decided by"
    doc with a **RETIRED 2026-09-xx** paragraph: subject resolved (7 → 0), guard is now the pin at 0
    plus the ratchet at 3; **keep the RESET LOG table (`:175-186`) verbatim** — its whole point is
    visibility without git history.
13. `archive_check.rs:327-343` — delete `now`/`overdue` and the `Err` branch; reword the `println!`
    at `:331` from "pending reconciliation (CONTINUITY.md §0 step ③)" to "accounted-for archive(s) —
    hybrid, decided 2026-07-30; duplicates pinned at 0 by `authority-manifest`".
14. `archive_check.rs:483-512` — delete `the_archive_reconciliation_is_not_past_its_review_by`.
15. `archive_check.rs:153` "47 binaries" → 42; `:159` "25 real extracts" → 20.
16. `authority_conflicts.rs:123-125` — `today()` has one consumer left; make it private and drop the
    `pub(crate)` rationale.
17. `CONTINUITY.md:522` — residue sentence → "residue RESOLVED 2026-09-xx (7 → 0); the tickle
    retired with it; the pin at 0 is the standing guard." `design/HARNESS.md:167` — one appended
    sentence saying the same, so the design record does not describe a mechanism that is gone.

**Gate for the commit message (all machine-checkable):** `cargo run -p xtask -- authority-manifest`
prints `0 document(s) archived under more than one path` and `OK`; `cargo run -p xtask -- archive-check`
green; `cd legal && sha256sum -c SHA256SUMS` → 42 OK; `grep -rn` for the eight deleted archive
paths and the five `legal/text/irs-forms/` extracts outside `reviews/`, `design/agent-reports/` and
`_provenance/` → 0; `make check` + `cargo fmt --all --check` green.

**B1:** no new checker is introduced, so no new kill is owed. The instrument that now carries the
guarantee already discriminates in the live tree: with the pin at 0, `cp` any surviving note to a
second path, `--regen`, and `duplicate_source_groups_may_only_shrink` reds. Run that once in the
review and quote the red.

**Review:** one independent round on the commit-3 diff, sonnet-tier — the questions are mechanical
(every consumer redirected; the census and pin red on the planted duplicate; no `periodic/` or
`irs-forms/` form path survives outside history). Review first, then commit 3.

## 4. WHAT THIS COSTS

- **The dated pressure on archive hygiene is gone.** What remains is structural and covers exactly
  two classes — a duplicate (pin 0) and a new tree (ratchet 3). Residue that is *neither* has no
  tickle afterwards: `Form_1099-DA` + `Instructions_1099-DA` still live in (B) as committed binaries
  contrary to the hybrid storage rule for forms; `f8275r.pdf` is still unarchived (§G-12);
  `extract/f8283--2024.txt` has no manifest subject because its source is a runtime asset. None is
  a duplicate, none was the tickle's subject, all belong in `FOLLOWUPS.md` with an owning phase —
  file them in commit 3's message. Undetected if not filed: they become furniture, quietly.
- **A future *legitimate* same-stem duplicate will red the pin at 0** — an unrevised periodic form
  archived under two tax years (`2025/f8275--2025` byte-equal to `2024/f8275--2024`). That is the
  moment to teach `duplicates()` the alias mechanism (same tree, same stem, different year) with a
  planted-defect test — not now; `HARNESS.md` says grow from observed failures.
- **`legal/` is no longer a self-contained offline copy of the five forms.** The hybrid decision
  already accepted that trade for forms; the (A) notes re-fetch them, and the bytes stay in history
  (public repo, no users). If an `irs-prior` URL from step 0 turns out not to exist, the moving
  `irs-pdf/` URL becomes that note's provenance — weaker than frozen, no weaker than today.
- **One review round and an afternoon**, against a reset that returns here on 2026-09-11 with the
  same afternoon still owed.

## 5. REJECTED ALTERNATIVES

- **(a) A plain second reset** (e.g. to 2026-09-30, "behind tag/publish"). Its stated reason would
  be the one #1 already used, whose condition has arrived; each firing blocks every commit for a
  hygiene question; and it schedules a third firing with nothing decided. The doc's own words:
  decoration.
- **(c-hard) Delete the tickle, leave the pin at 7.** The mute. "A known defect with no deadline is
  indistinguishable from a forgotten one" (`:169-170`) applies with full force while 7 stands.
- **(c-soft) Make the tickle advisory** (print `OVERDUE`, exit 0). A warning nobody must read is the
  decoration one step earlier; and `AUTHORITY_CONFLICTS.md` shares the mechanism for a real
  understatement risk and must stay hard.
- **(c-register) A per-group `posture/decided/review-by` register** like `AUTHORITY_CONFLICTS.md`.
  Right for legal positions where the world moves; wrong for byte-identical files where nothing
  does. Machinery for two chores that take an afternoon.
- **(b-alt) Keep `periodic/`, delete the year-named copies.** `periodic/` holds one revision, no code
  resolves through it, its notes already point at a missing extract, and the TY2024 Form 8283 proves
  the year directory must carry revisions `periodic/` cannot.
- **(b-alt2) Move (B)'s five into (A) as committed binaries.** Violates the hybrid storage rule for
  forms and keeps ~906 KB the notes make unnecessary.
- **Commit the report first by exploiting the hook's working-tree check** (resolution unstaged in
  the tree, only the report staged). It passes the hook on the strength of uncommitted work and
  leaves a red committed tree — a route-around in spirit of the gate it would be sidestepping.
