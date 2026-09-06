# Independent verification — `xtask port-status` (FR-50 closure)

**Reviewer:** independent read-only verifier (sonnet subagent), no edits made outside this report.
**Date:** 2026-09-06
**Commit under review:** `bb37c352a4bfd8a748e2902d2bc97ac15786b2c2`
**Repo state at review time:** `main` @ `c5b142cf` (3 commits ahead of bb37c352). Confirmed by
`git diff bb37c352..HEAD --stat -- crates/xtask/src/form_delta.rs crates/xtask/src/main.rs
design/TY2026_WORK_LIST.md FOLLOWUPS.md`: only `FOLLOWUPS.md` changed (an unrelated FR-59
blast-radius edit), and the FR-50 entry itself is untouched. `form_delta.rs`, `main.rs`, and
`design/TY2026_WORK_LIST.md` in the working tree are byte-identical to their state at bb37c352, so
all commands below were run against the exact reviewed code.

**Commands run (once each, per instructions):**
```
cargo nextest run --locked -p xtask -E 'test(work_list) | test(port_status)' --no-capture
  → 3 tests run: 3 passed, 126 skipped (all green)
cargo run -q -p xtask -- port-status 2025 2026-DRAFT
  → captured, cross-checked against design/TY2026_WORK_LIST.md
```
One additional single-purpose diagnostic (not a re-run of the above suite): `cargo run -q -p xtask
-- form-delta f8949--2025 f8949--2026-DRAFT`, to resolve question (c)'s `label_compared` concern for
`f8949` directly from the tool's own diagnostic output (`line->label: 186 of 202 common field(s)
COMPARED`).

---

## Verdicts

### (a) `emitting_surface()` — 18 stems, NOT byte-identical to the old scan

`the_committed_work_list_matches_form_delta_at_head`'s own stdout confirms the count: `work list: 14
compared, 4 excused, 18 stems on the emitting surface`. The 18, reconstructed from the printer's own
output (14 numeric + 4 excused):

`f1040s1, f1040s1a, f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, f1040sd, f1040sse, f6251, f8949,
f8959, f8960, f8995` (numeric table) + `f1040, f8275, f8283, f8995a` (excused table).

**Not byte-identical in behaviour to the doc-test scan it replaced.** Diffing the removed test-local
scan against the new `emitting_surface()`:
- Old: `std::fs::read_dir(root.join(...)).unwrap().flatten()` — **panics** if the top-level
  `crates/btctax-forms/forms` directory is missing.
- New: `std::fs::read_dir(&forms).into_iter().flatten().flatten()` — **silently yields an empty
  iterator** on the same condition.
- Old: `std::fs::read_to_string(&p).unwrap()` on each `.map.toml` — panics if unreadable/non-UTF8.
- New: `let Ok(text) = std::fs::read_to_string(&p) else { continue };` — silently skips it.

Functionally equivalent on the current repo (directory present, every map file readable — verified:
all 18 stems recovered, matching the pre-existing `the_committed_work_list_matches_form_delta_at_head`
test which now consumes the same function and still passes). But it is a real behavioural divergence,
not just a refactor — see Finding 4 (Minor).

### (b) Numeric rows: cell-for-cell equal; shape column NOT held by any test

Programmatic diff of the printer's 14 numeric rows against `design/TY2026_WORK_LIST.md`'s committed
table (common/added/removed/moved/shape) shows **exact agreement on every cell, including the shape
column**, for all 14 rows (only the UNWITNESSED explanatory prose text for `f1040s1` differs — cosmetic,
not a parsed cell; both sides contain the substring `UNWITNESSED`).

However: `parse_work_list_row` returns `(form, Option<(common, added, removed, moved)>, prior_cell,
ty2026_cell)` — **the shape cell (column 6) is not part of the returned tuple at all**, and the new
test's `rows()` closure further discards it (`.map(|(form, cells, _, _)| (form, cells))`). So
`port_status_prints_the_committed_work_list` verifies only common/added/removed/moved, never shape.
`grep -n shape crates/xtask/src/form_delta.rs` confirms the shape `if/else if/else` (lines 468–478) is
the *only* place shape logic exists — `check_work_list` (the other conformance checker, pre-existing)
also never reads column 6. **The shape column — the commit's headline change (3 rows flipped
port→REBUILT) — has never been observed red by any test.** See Finding 2 (Important).

### (c) Shape rule reproduces all 14 printed shapes; `f8949`'s "unchanged" is safe

Applying `REBUILT` iff `added+removed > common`, else `unchanged` iff `added.is_empty() &&
removed.is_empty() && label_compared > 0 && label_moved.is_empty()`, else `port`, by hand against each
row's actual common/added/removed reproduces the printed shape exactly for all 14 (spot-checked:
`f1040sa` 33+19=52>14→REBUILT ✓; `f1040sc` 50+46=96>59→REBUILT ✓; `f8995` 14+11=25>22→REBUILT ✓;
`f1040s2` 24+19=43, common=44, 43≯44→port ✓; `f1040s1` common=72 but `label_compared==0` (moved prints
UNWITNESSED) so the `unchanged` branch's `label_compared > 0` conjunct fails → falls to `port` ✓).

`f8949` specifically: printed row is `202 | 0 | 0 | 0 | unchanged`. The concern was whether
`label_compared` could be 0 (which would make a printed `moved=0` indistinguishable from
`label_compared==0`, i.e. wrongly "unchanged" instead of UNWITNESSED). Ran `form-delta f8949--2025
f8949--2026-DRAFT` directly: **`line->label: 186 of 202 common field(s) COMPARED; none of them changed
the printed line it sits beside`** — `label_compared = 186`, not 0. The `moved` branch's guard
(`d.label_compared == 0 && !d.common.is_empty()`) is correctly false, so the code takes the
`d.label_moved.len().to_string()` arm and prints a genuine `"0"`, and the shape's `unchanged` branch
correctly requires `label_compared > 0` before firing. **No bug: `f8949`'s `unchanged` is a real
zero-moved verdict from 186 witnessed comparisons, not a masked UNWITNESSED.**

### (d) `f8275` excused cell is TRUE for the tag asked; excused-table PROSE unheld; "NO FINAL" never emitted

`pdf_for` checks: confirmed on disk that **no `f8275--2025.pdf` and no `f8275--2026-DRAFT.pdf` exist
anywhere** (`find design/forms crates/btctax-forms/forms -iname '*8275*'` — only `f8275--2024.pdf`
bundled/archived and a *different* stem `f8275r--2025.pdf`, Form 8275-R, not an alias). So
`pdf_for("f8275--2025").is_none()` is genuinely true, and the printer's `**NO PRIOR SIDE**` cell is
**TRUE for the exact tag it was asked about**.

The doc test's excuse check still passes on the doc's own prose row: `check_work_list`'s
`(None, Err(_))` branch computes `claims_no_prior = prior_cell.contains("NO PRIOR SIDE")` (false — the
doc's prior-side cell text is `` `f8275--2024` (Rev. 10-2024, periodic; aliased by hash for 2025) ``,
which never contains that literal substring) and `claims_no_draft = ty2026_cell.contains("NO DRAFT")`
(true). Since only one claim is made and it evaluates true against `archived("f8275--2026-DRAFT") ==
false`, `ok = true`. Empirically confirmed: the nextest run shows `the_committed_work_list_matches_
form_delta_at_head` passing with `4 excused` and no `wrong` entries.

**But** `port_status_prints_the_committed_work_list` never inspects excused-row *content* at all — its
`rows()` maps every excused row to `(form, None)`, so only the *set* of excused form names is checked
(via the surface-equality assertion), never the prior/TY2026/cell prose. This matters concretely: the
committed doc's excused table carries hand-authored, citation-bearing prose (`packet.rs:218`, periodic
revision dates, an actionable "archive `f8995a--2025` + `i8995a--2025`, then diff") that the mechanical
printer does **not** reproduce — confirmed by diff: printer's `f8275` prior-side cell is the bare
`**NO PRIOR SIDE**`, nothing like the doc's explanatory sentence. The doc's own new instruction (added
by this commit) says "Regenerate with `cargo run -p xtask -- port-status 2025 2026-DRAFT`"; followed
literally (paste replaces both tables), this prose is silently destroyed and no test would notice. See
Finding 3 (Important).

**"NO FINAL" does not exist anywhere in the printer.** `grep -n "NO FINAL" crates/xtask/src/
form_delta.rs` → no matches. FR-50's original text (`git show bb37c352^:FOLLOWUPS.md`) explicitly asked
for `NO PRIOR SIDE` / `NO DRAFT` / `NO FINAL` per cell. The printer's `new`/`cell` construction
hardcodes the literal string `"**NO DRAFT**"` unconditionally on `pdf_for(&b).is_none()`, with no
branch on whether `new_tag` names a draft or a final — so `port-status 2025 2026` run post-finals
against a form whose final PDF is still missing will print the misleading `NO DRAFT` for what is
actually a missing FINAL. Worse: `check_work_list`'s `claims_no_draft` only recognizes the literal
substring `"NO DRAFT"` — so even a manually-corrected doc row that legitimately says `**NO FINAL**`
would fail `claims_no_draft`, and (assuming `claims_no_prior` is also false) the conformance test would
**red on a truthful, legitimately-excused row.** This is a gap that will bite exactly at the
post-finals regeneration the doc's own new header now instructs readers to run. See Finding 1
(Important).

### (e) The kill test's two real assertions hold; the trailing "plant" is a tautology, not a discriminator

- **Surface-equality assertion** (`assert_eq!(p.keys()..., surface, ...)`, using the genuinely-computed
  `printed` output and the genuinely-computed `emitting_surface()`): **yes**, a printer that silently
  dropped a stem's row (from either table) would shrink `p.keys()` below `surface`'s 18 elements and
  fail this assertion. This is a real discriminator.
- **Per-row equality assertion** (`assert_eq!(d.get(form), Some(cells), ...)` for every printed form):
  **yes**, a printer that computed a wrong common/added/removed/moved count would diverge from the
  doc's currently-correct committed numbers and fail here. Real discriminator (contingent on the doc
  itself being correct, which the sibling `the_committed_work_list_matches_form_delta_at_head` test
  independently verifies against `compute()` at HEAD).
- **The trailing plant** (drop the `f6251` line from the already-computed `printed` string via
  `.lines().filter(...)`, reparse with the test's own `rows()`/`parse_work_list_row` helpers, and
  assert the reparsed count shrank): **this never re-invokes `port_status()` or `emitting_surface()`
  with any mutation.** It performs a string-level deletion on already-correct output and re-runs the
  test's own parsing/counting logic on it — proving only that "removing a line from a list shrinks the
  parsed count of that list," which is a tautology about the test harness's own row-counter, not a
  mutation-kill of any production code path in `port_status` or `emitting_surface`. The commit message
  attributes "Kill: ... a dropped row changes the parsed set" to this construct, but the actual kill
  property (would a real bug get caught?) already lives entirely in the surface-equality assertion one
  block earlier, which uses genuine tool output. The trailing plant adds no additional protection beyond
  what that assertion already provides. Per this review's own severity rubric, this is logged as
  Finding 8 (Important — "a plant that does not discriminate").

### (f) `main.rs` arm: consistent with the sibling `form-delta` arm

Both arms: `let (Some(a), Some(b)) = (args.get(1), args.get(2)) else { eprintln!("usage: ..."); exit(2)
}`, both print a one-line usage plus an `e.g.` example, both dispatch to a `Result<_, String>`-returning
function and on `Err` print `eprintln!("xtask <cmd>: {e}"); exit(1)`. Identical shape, identical exit
codes (2 for bad args, 1 for handler error, implicit 0 on success). `port_status` returns `Ok(String)`
printed via `print!("{s}")` (appropriate — it's a document to be captured/pasted, unlike `form_delta::
run`'s side-effecting `println!`s) — a reasonable, deliberate API difference, not an inconsistency.

One pre-existing gap, not introduced by this commit: the top-level `_ => { eprintln!("usage: ...
<docs|examples|...|dump-fields>") }` catch-all help string in `main.rs` omits **both** `form-delta` and
`port-status` — `form-delta` was already missing before this commit; `port-status` simply inherits the
same omission rather than introducing a new one. Logged as Finding 7 (Minor).

---

## Findings

| # | Severity | Where | What is wrong | Minimal change |
|---|----------|-------|----------------|-----------------|
| 1 | **Important** | `crates/xtask/src/form_delta.rs` port_status ~L486–500 (printer); L735–736 `check_work_list` (checker) | FR-50 asked for three cell states (`NO PRIOR SIDE`/`NO DRAFT`/`NO FINAL`); the printer hardcodes `"**NO DRAFT**"` regardless of what `new_tag` names, and the checker's `claims_no_draft` only recognizes the literal substring `"NO DRAFT"`. `grep -n "NO FINAL"` → zero matches in the file. Post-finals (`port-status 2025 2026`), a genuinely-missing final will be mislabeled "NO DRAFT", and a correctly-labeled "NO FINAL" row would fail `the_committed_work_list_matches_form_delta_at_head`. Bites at the very regeneration this commit's new doc header instructs. | Branch the label on whether `new_tag` contains `"DRAFT"` (emit `NO DRAFT` vs `NO FINAL`); widen `claims_no_draft` to match either substring. |
| 2 | **Important** | `crates/xtask/src/form_delta.rs` `parse_work_list_row` (no shape slot in `Cells`); test `rows()` closure L~573 drops it | The shape column — this commit's headline change (3 rows port→REBUILT) — is verified correct today by hand/programmatic diff, but is not held by any test. `parse_work_list_row`'s returned tuple has no shape field, and the kill test explicitly discards it (`.map(|(form, cells, _, _)| (form, cells))`). A future regression in the shape `if/else if/else` that left common/added/removed/moved unchanged would leave this test green. | Extend `Cells`/`parse_work_list_row` to also capture column 6, and assert it alongside the numeric cells in the per-row loop. |
| 3 | **Important** | `crates/xtask/src/form_delta.rs` port_status excused-row assembly ~L484–503; test `rows()` maps excused rows to `(form, None)` | Excused-table content (prior-side/TY2026-side/cell prose) is entirely unverified — only the *set* of excused form names is checked. Confirmed the printer's terse generic prose (`f8275` → bare `**NO PRIOR SIDE**`) diverges from the doc's committed citation-rich prose (`packet.rs:218`, periodic-alias explanation, `f8995a` archive-then-diff action item). The doc's own new instruction says to regenerate by pasting the printer's output; literally doing so for the excused table would silently destroy that prose, and nothing would catch it. | Either state explicitly (doc + tool doc-comment) that only the numeric table is meant to be pasted verbatim and the excused table stays hand-maintained, or extend the test to assert on excused-row text too. |
| 4 | Minor | `crates/xtask/src/form_delta.rs` `emitting_surface()` L419–421, L431–433 | Not byte-identical to the scan it replaced: old code `.unwrap()`s (panics) on a missing top-level forms dir or an unreadable `.map.toml`; new code silently continues (`into_iter().flatten()` / `let Ok(..) = .. else { continue }`). Functionally identical today (all directories/files present and readable, confirmed by the 18-stem count matching the pre-existing conformance test), but is a real behavioural change toward silent degradation rather than a loud failure. | None required now; note if silent absorption of a missing/corrupt fixture is undesired. |
| 5 | Minor | `crates/xtask/src/form_delta.rs` port_status "cell" construction ~L496–501 | When BOTH sides of a pair are missing (true today for `f8275`: neither `f8275--2025` nor `f8275--2026-DRAFT` has a PDF on disk), the summary "cell" column always resolves to `**NO DRAFT**` (from `if pdf_for(&b).is_none() {...} else {...}`) and never surfaces that the prior side is *also* missing in that one cell. No information is actually lost to a reader, since the adjacent "prior side" column in the same row does correctly show `**NO PRIOR SIDE**`. | None required; cosmetic only. |
| 6 | Minor | `design/TY2026_WORK_LIST.md`, "## Not listed" section, first paragraph | Stale prose left over: "Until `forms port-status <year>` exists ... the cells it would print are:" — the tool now exists (as `xtask port-status <prior-tag> <new-tag>`, correctly named three paragraphs above by this same commit), and even the stale sentence's own command name/signature (`forms port-status <year>`) never matched what was actually built. Self-contradictory within one document; no wrong numbers, doc-freshness only. | Update or remove the stale "Until X exists" sentence. |
| 7 | Minor | `crates/xtask/src/main.rs` catch-all `_ =>` usage string (~L238) | Top-level help lists available subcommands but omits both `form-delta` (pre-existing) and `port-status` (inherits the same gap, not a new regression). | Add both to the catch-all usage string. |
| 8 | **Important** | `crates/xtask/src/form_delta.rs` test `port_status_prints_the_committed_work_list`, trailing "plant" (~L591–604) | The commit-message-labeled "kill" (a dropped row changes the parsed set) is a string-manipulation of already-computed output re-parsed by the test's own helpers — it never re-invokes `port_status`/`emitting_surface` with a mutation, and proves only that removing a line shrinks a parsed line count (tautological about the harness's own counter). The actual, real discriminator against a printer that silently drops a stem already exists one block earlier (the surface-equality assertion using genuine tool output); the trailing plant adds no additional protection and is mislabeled as the kill. | Either remove the trailing plant as redundant, or replace it with a genuine mutation (e.g. a test-only wrapper that calls `port_status` after temporarily shrinking a test-local copy of `emitting_surface`'s inputs) that actually exercises production code. |

---

## Counts

**0 Critical / 4 Important / 4 Minor / 0 Nit**

All three targeted tests (`port_status_prints_the_committed_work_list`,
`the_committed_work_list_matches_form_delta_at_head`, `the_work_list_checker_reds_on_every_planted_row`)
pass (3/3, confirmed by a single nextest run). The committed numeric table matches the printer
cell-for-cell for all 14 rows including shape (verified by direct diff, not by the test — see Finding
2). No Critical found: neither conformance checker is structurally incapable of failing — both real
assertions in the new test (surface equality, per-row numeric equality) do fail under a genuine
mutation, reasoned from code (edits were out of scope for this read-only review). The four Important
findings are two gaps that will bite at the specific, foreseeable post-finals regeneration (FR-50's
"NO FINAL" state, and excused-table prose loss on a literal paste-regeneration), one column with zero
test coverage despite being the commit's headline change, and one non-discriminating plant mislabeled
as a kill.

**Headline:** FR-50's numeric-table machinery is solid and verified cell-for-cell against 3 fresh
command runs, but its own regeneration instructions and the "NO FINAL" case it was explicitly asked to
handle are unbuilt and unguarded — both will surface at the next (post-finals) regeneration this
commit's own doc header now tells the operator to run.
