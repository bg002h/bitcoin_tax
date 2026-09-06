# Independent re-verification (r2) — `xtask port-status` fold `36d17139`

**Reviewer:** independent read-only verifier (sonnet subagent), no file edits made outside this report.
**Date:** 2026-09-06
**Fold under review:** `36d171395153fb0d9170bf81c84d1b325f767eb6` (HEAD @ time of review; `main`)
**Answers:** `design/agent-reports/2026-09-06-port-status-review.md` (0C/4I/4M) and the controller's ledger
`design/agent-reports/2026-09-06-port-status-review-VERIFICATION.md` (8/8 TRUE).

**Commands run (once each, per instructions):**
```
cargo nextest run --locked -p xtask -E 'test(work_list) | test(port_status)' --no-capture
  → /tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r2.txt — 3 tests run: 3 passed, 126 skipped (all green)
cargo run -q -p xtask -- port-status 2025 2026-DRAFT
  → /tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r2-out.md
cargo run -q -p xtask -- port-status 2025 2026
  → /tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r2-final.md
```
Both `cargo run` invocations exited 0 with empty stderr. Diff of the whole fold saved for reference at
`/tmp/claude-1000/-scratch-code-bitcoin-tax/2c87994c-b998-4466-94ed-3391d7906762/scratchpad/fold-diff.txt`.

---

## Checklist #1–#8

| # | Original severity | Status at `36d17139` |
|---|---|---|
| 1 | Important | **Printer side RESOLVED and witnessed.** **Checker side plausibly fixed by inspection, but UNWITNESSED — new finding N1.** |
| 2 | Important | **RESOLVED**, witnessed by a passing test and independently confirmed. |
| 3 | Important | **RESOLVED**, witnessed by a passing test and independently confirmed; `f8275` corrected. |
| 4 | Minor | Recorded as accepted (no fix required by the original review). Not re-verified, per instructions. |
| 5 | Minor | **RESOLVED**, confirmed by direct execution on both tags. |
| 6 | Minor | **RESOLVED**, stale sentence gone. |
| 7 | Minor | **RESOLVED**, both subcommands now in the catch-all usage. |
| 8 | Important | **RESOLVED** — the plant now calls real production code with a genuine mutation and catches a real bug class (parameter-ignoring). One precision gap noted as new finding N2 (Minor, no live blind spot). |

---

## Verdicts

### (a) #1 — NO FINAL on a final tag

Ran `cargo run -q -p xtask -- port-status 2025 2026` (no DRAFT in the new tag). Every one of the 18
excused/degraded rows in `ps-r2-final.md` prints `**NO FINAL**`; `grep -c "NO FINAL"` → 18, `grep -c
"NO DRAFT"` → 0. So the printer genuinely branches on `new_tag.contains("DRAFT")`
(`crates/xtask/src/form_delta.rs:469-473`, `no_new`) and this is witnessed on real output by the new
in-test assertion at line 646-647: `let final_tag = super::port_status_over(&shrunk, "2025",
"2026").unwrap(); assert!(final_tag.contains("**NO FINAL**") && !final_tag.contains("NO DRAFT"));` —
this ran and passed in the nextest run above. **Printer side is resolved and witnessed.**

`check_work_list`'s acceptance: `crates/xtask/src/form_delta.rs:778-780`:
```rust
let claims_no_prior = prior_cell.contains("NO PRIOR SIDE");
let claims_no_draft =
    ty2026_cell.contains("NO DRAFT") || ty2026_cell.contains("NO FINAL");
```
By inspection this correctly accepts a doc row whose TY2026-side cell says `**NO FINAL**` (trivial
substring containment — not executed, since I cannot add a test under the no-edit constraint). **But
`grep -n "NO FINAL" crates/xtask/src/form_delta.rs` shows the only other appearances are the printer's
own emission and the test's `claim()` closure (lines 605-606, used only to LABEL a claim for the
per-row equality check, never to assert the CHECKER's verdict) — `the_work_list_checker_reds_on_every_
planted_row` (the one test that exercises `check_work_list` directly with hand-planted rows) plants
zero rows containing `NO FINAL`, in neither the accept nor reject direction.** So the widened
`claims_no_draft` disjunct has never been observed discriminating true from false — see new finding
N1 below.

Yes: the checker's variable name `claims_no_draft` is now misleading (it accepts `NO FINAL` too) —
cosmetic, recorded as N3 (Nit).

### (b) #2 — shape column held cell-for-cell

Diffed `ps-r2-out.md`'s 14 numeric rows against `design/TY2026_WORK_LIST.md` lines 31-44: **exact
agreement on every cell in every row, including the shape column** (`port`/`unchanged`/`**REBUILT**`),
for all 14 rows (the only textual difference is the non-parsed UNWITNESSED prose on `f1040s1`, already
noted as cosmetic by the original review).

Reasoned from the code: `parse_work_list_row_full` (`crates/xtask/src/form_delta.rs:791` area) now
captures `last = cells.get(6)` for numeric rows (the shape cell), and the test's `rows()` closure
(lines 593-616) folds it into the map value as `(Cells, held)` where `held` is that shape string for
numeric rows. The per-row loop (`crates/xtask/src/form_delta.rs:624-631`) does `assert_eq!(d.get(form),
Some(held), ...)` on the **whole tuple**, so a printer that printed `port` for `f1040sa` (whose
committed doc cell is `**REBUILT**`, confirmed at `design/TY2026_WORK_LIST.md:35`) would produce
`held = "port"` on the printer side vs. `Some((cells, "**REBUILT**"))` on the doc side — a genuine tuple
mismatch, and `assert_eq!` would panic/red. **Held, not merely diffed — confirmed both by passing test
and independent manual diff.**

### (c) #3 — excused rows held on two claims

Four excused rows, doc (`design/TY2026_WORK_LIST.md:56-59`) vs printer (`ps-r2-out.md`, `--2025`/
`--2026-DRAFT`):

| form | doc: prior claim | doc: new claim | printer: prior claim | printer: new claim | match? |
|---|---|---|---|---|---|
| `f1040` | present (`f1040--2025`) | NO DRAFT | present (`f1040--2025`) | NO DRAFT | yes |
| `f8283` | present (`f8283--2025`) | NO DRAFT | present (`f8283--2025`) | NO DRAFT | yes |
| `f8275` | **NO PRIOR SIDE** for the `--2025` tag | NO DRAFT | **NO PRIOR SIDE** | NO DRAFT | yes |
| `f8995a` | NO PRIOR SIDE | present (`f8995a--2026-DRAFT`) | NO PRIOR SIDE | present | yes |

All four match at HEAD, confirmed both by the passing test (`port_status_prints_the_committed_work_
list`, which asserts exactly this two-claim tuple per excused row via the `claim()` closure at lines
600-611) and by this independent side-by-side read. `f8275` is confirmed corrected: its prior-side cell
now reads `**NO PRIOR SIDE** for the `--2025` tag — ...` (`design/TY2026_WORK_LIST.md:58`), not a false
claim of a `--2025` fixture that does not exist. "present / NO DRAFT" for `f1040` and `f8283` is exactly
what the doc's cells say (a real filename in the prior-side column, `**NO DRAFT**` in the TY2026-side
column) — confirmed above.

### (d) #8 — the plant genuinely runs the printer; one precision gap, no live blind spot

**The plant calls real production code with genuinely-shrunk input** (`crates/xtask/src/
form_delta.rs:634-636`):
```rust
let mut shrunk = surface.clone();
assert!(shrunk.remove("f6251"));
let partial = rows(&super::port_status_over(&shrunk, "2025", "2026-DRAFT").unwrap());
```
This is a real change from the original tautological string-slice plant — `port_status_over` is invoked
with an actual 17-element surface and its actual `Result<String, String>` output is parsed. The
subsequent `assert_ne!(partial.keys()..., surface, ...)` (comparing to the FULL, unshrunk 18-element
`surface` from line 618) genuinely depends on the printer's output, not on string manipulation of an
already-computed string.

**Would deleting `port_status_over`'s loop body (printing nothing) make this specific assertion pass
trivially? Yes.** `partial` would then be an empty `BTreeMap`, so `partial.keys()` is `{}`, and `{} !=
surface(18 elements)` is true regardless — the `assert_ne!` at lines 637-644 would NOT catch a
completely silent printer, on its own. This confirms the concern raised in the brief.

**However, there is no live blind spot in the test as a whole.** The same test function's *earlier*
assertion (`crates/xtask/src/form_delta.rs:619-623`) —
```rust
assert_eq!(
    p.keys().cloned().collect::<std::collections::BTreeSet<_>>(),
    surface,
    "the printer must print one row per stem on the emitting surface"
);
```
— uses `p`, parsed from `printed = super::port_status("2025", "2026-DRAFT").unwrap()` at line 586, the
**genuine, unshrunk** call. If `port_status_over`'s loop body were deleted, `port_status()` (which
delegates to it with the full surface) would ALSO print nothing, `p` would be `{}`, and this earlier
`assert_eq!` would fail (panic) — before the test ever reaches the trailing plant. So the "prints
nothing" mutation is caught by the test function, just not by the specific block that self-labels as
addressing #8.

**Conversely, the trailing plant catches a real bug class nothing else in the test reaches**: if
`port_status_over` were mutated to *ignore* its `surface` parameter and always iterate the true, full
`emitting_surface()` internally, `p` at line 617 would still be correct (since `port_status()` always
passes the full surface anyway — the mutation is invisible there), so the earlier `assert_eq!` would
NOT catch it. But `partial` (fed the 17-element `shrunk`) would then also be the full 18-row output,
making `partial.keys() == surface` exactly — and `assert_ne!` would correctly fail. **This is a genuine,
non-trivial discriminator for a real defect (parameter-ignoring) that only the plant reaches.**

Net: **#8 is resolved** — no longer a tautology, exercises production code, and catches a real bug the
rest of the suite cannot see. The residual imprecision (comparing against `surface` instead of its own
input `shrunk`) is recorded as new finding N2, Minor (not Important, since the "silent printer" case it
misses is independently covered by line 619-623). The reviewer-suggested tightening — `assert_eq!
(partial.keys()..., shrunk, ...)` — would make the plant self-sufficient for both mutation classes in
one assertion, without relying on an earlier, differently-purposed check for coverage.

### (e) #5/#6/#7

- **#5** (both-sides-missing cell names both): confirmed on real output. `--2025`/`--2026-DRAFT` run
  (`ps-r2-out.md`): `f8275 | yes | **NO PRIOR SIDE** | **NO DRAFT** | **NO PRIOR SIDE** + **NO DRAFT**`.
  `--2025`/`--2026` run (`ps-r2-final.md`): both `f8275` and `f8995a` (which is not missing its new
  side under the DRAFT tag but is under the final tag) print `**NO PRIOR SIDE** + **NO FINAL**`. Code:
  `crates/xtask/src/form_delta.rs:516-519`, `match (has_prior, has_new) { (false, false) =>
  format!("**NO PRIOR SIDE** + {no_new}"), ... }`. Note: the test's `rows()` closure discards the
  summary "cell" column for excused rows entirely (only the prior/new claim columns are held) — the doc
  itself now says explicitly that only the numeric table is meant to be pasted verbatim and the excused
  table's prose (including the cell column) stays hand-maintained (`design/TY2026_WORK_LIST.md:47-49`),
  which matches the original review's own suggested resolution for #3. The original review already
  rated #5 as needing no fix; present.
- **#6**: `grep -n "Until" design/TY2026_WORK_LIST.md` → no matches. The stale sentence is replaced
  with the doc-comment-accurate text at `design/TY2026_WORK_LIST.md:47-49`. Resolved.
- **#7**: `grep -n "form-delta\|port-status" crates/xtask/src/main.rs` shows both now present in the
  catch-all usage string at lines 243-244 (`... extract-schedule-1a | dump-fields <pdf> | form-delta
  <old> <new> | port-status <prior-tag> <new-tag>>`). Resolved.

### (f) #4

Recorded as accepted per the original review (Minor, "None required now"). Not re-verified, per
instructions.

---

## New findings (surfaced by this re-verification)

| # | Severity | Where | What is wrong | Minimal change |
|---|---|---|---|---|
| N1 | **Important** | `crates/xtask/src/form_delta.rs:778-780` (`check_work_list`'s `claims_no_draft`); test `the_work_list_checker_reds_on_every_planted_row` (~L653-717) | The widened `claims_no_draft = ty2026_cell.contains("NO DRAFT") \|\| ty2026_cell.contains("NO FINAL")` — the fix for #1's checker-side gap — has never been exercised by any planted row. `the_work_list_checker_reds_on_every_planted_row` plants only `NO DRAFT` cells; there is no plant asserting the checker accepts a truthful `**NO FINAL**` row, nor one asserting it rejects a falsely-claimed one. This is the exact class the repo's own B1 harness rule targets ("no checker exists until observed RED on a planted defect") — the branch is currently unwitnessed in either direction, and it is precisely the path FR-50's own post-finals regeneration will exercise. | Add two plants to `the_work_list_checker_reds_on_every_planted_row`: one row correctly claiming `**NO FINAL**` for a form whose final PDF genuinely does not exist (`wrong` stays empty), and one row falsely claiming `**NO FINAL**` for a form whose final/draft PDF is actually on disk (`wrong.len() == 1`). |
| N2 | Minor | `crates/xtask/src/form_delta.rs:637-644` (the #8 plant's `assert_ne!`) | The assertion compares `partial.keys()` (from a call fed the 17-element `shrunk` surface) against `surface` (the original, unshrunk 18-element set) rather than against `shrunk` itself. In isolation this does not catch a `port_status_over` whose loop body prints nothing (an empty set trivially `!=` any 18-element set). No live gap exists today because the earlier `assert_eq!` at lines 619-623 (using the genuine unshrunk `port_status()` call) already reds on that exact mutation — but the plant is not self-sufficient. | `assert_eq!(partial.keys().cloned().collect::<BTreeSet<_>>(), shrunk, "a printer must print exactly one row per stem in the surface it was given");` — catches both the silent-printer and the parameter-ignoring mutation in one assertion, independent of the earlier check. |
| N3 | Nit | `crates/xtask/src/form_delta.rs:779` | `check_work_list`'s local variable `claims_no_draft` now also matches `"NO FINAL"`, so its name undersells what it checks. Cosmetic only. | Rename to `claims_no_new_side` (or similar) when next touched. |

---

## Counts

**0 Critical / 1 Important / 1 Minor / 1 Nit** (new findings; all eight original findings from
`2026-09-06-port-status-review.md` are resolved or accepted as recorded, per the checklist above).

All 3 targeted tests pass (`port_status_prints_the_committed_work_list`,
`the_committed_work_list_matches_form_delta_at_head`, `the_work_list_checker_reds_on_every_planted_row`
— 3/3, one nextest run, `/tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r2.txt`). No Critical: neither
conformance checker is structurally incapable of failing — both live `assert_eq!`/`assert_ne!` pairs in
the shape/claims/plant paths were traced to real production-code dependencies and confirmed failing
under a genuine or reasoned mutation (N1's finding is an *unwitnessed* correctness claim, not a checker
that provably cannot fail — its logic is a one-line, easily-verified substring disjunction).

**Headline:** the fold resolves the review's core complaints — NO FINAL now genuinely emits from the
printer (witnessed on real `port-status 2025 2026` output, 18/18 rows), the shape column and the
excused-row claims are now held cell-for-cell by a passing test, and the #8 plant now runs real
production code and catches a real bug class. The one open thread is that the checker-side half of #1's
fix (accepting `NO FINAL` in `check_work_list`) is plausible by inspection but has never been observed
discriminating a true claim from a false one — exactly the gap this repo's B1 doctrine exists to close,
and exactly the path the doc's own new regeneration instructions will walk at the next finals cycle.
