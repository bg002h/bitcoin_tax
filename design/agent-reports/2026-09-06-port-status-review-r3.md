# Independent re-verification (r3) — `xtask port-status` fold `2545c506`

**Reviewer:** independent read-only verifier (sonnet subagent), no file edits made outside this report.
**Date:** 2026-09-06
**Fold under review:** `2545c5064fa086b0fcec5b5ae365d79f1886ee73` (`main`; HEAD at time of review is
`457cec3a`, which does not touch `crates/xtask/`).
**Answers:** `design/agent-reports/2026-09-06-port-status-review-r2.md` (0C/1I/1M/1N: N1/N2/N3) and
`design/agent-reports/2026-09-06-port-status-review-r2-VERIFICATION.md` (3/3 TRUE, settled).

**Command run once** (per instructions):
```
cargo nextest run --locked -p xtask -E 'test(work_list) | test(port_status)' --no-capture
  → /tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r3.txt
```
Result: **3 tests run: 3 passed, 126 skipped.** `the_committed_work_list_matches_form_delta_at_head`
printed `work list: 14 compared, 4 excused, 18 stems on the emitting surface` — matches the r2 report's
own baseline exactly. No regression in either targeted test.

---

## N1–N3 checklist

| # | r2 severity | Status at `2545c506` |
|---|---|---|
| N1 | Important — `claims_no_draft`'s NO FINAL acceptance never plant-exercised | **Partially resolved.** Three new plants added; the one that matches the historically-manifested bug (`new_stem` checking the wrong suffix) is genuinely witnessed and confirmed (by reasoning) to have failed under the pre-fold code. But `claims_no_new`'s own OR-disjunct (`\|\| ty2026_cell.contains("NO FINAL")`) remains unwitnessed **in isolation** — new finding N4. |
| N2 | Minor — the #8 plant compared `partial.keys()` against `surface` instead of `shrunk` | **Resolved.** A second `assert_eq!(partial_keys, shrunk, …)` was added; confirmed by reasoning to catch both the silent-printer and the parameter-ignoring mutation independent of the earlier `assert_eq!` at lines 619-623. |
| N3 | Nit — `claims_no_draft` undersells what it matches | **Resolved.** Renamed to `claims_no_new` everywhere; `grep -n "claims_no_draft" crates/xtask/src/form_delta.rs` → no hits. |

---

## Verdicts

### (a) N1 — the three new NO FINAL plants

Diff `2545c506` adds three `check_work_list(...)` calls (`crates/xtask/src/form_delta.rs:710-737`):

1. **`zzz-not-a-form` / NO FINAL — a "true synthetic"**, `wrong.is_empty() && excused == [...]`.
2. **`f8995a` / NO FINAL — true**, same assertion shape, plus a pre-assertion pinning the archive state:
   `pdf_for("f8995a--2026-DRAFT").is_some() && pdf_for("f8995a--2026").is_none()`.
3. **`f8995a` / NO DRAFT — false**, `wrong.len() == 1`.

Traced each through `check_work_list`'s `(None, Err(_))` arm (`crates/xtask/src/form_delta.rs:806-829`):

- **`zzz-not-a-form` (NO FINAL, true).** `new_stem = "zzz-not-a-form--2026"` (the `if ty2026_cell.contains("NO
  FINAL")` branch fires). `draft = archived(new_stem) = false`; `prior = archived(...--2025) = false`.
  `claims_no_prior = true` (prior cell says `**NO PRIOR SIDE**`). `ok` evaluates true. **But this plant does
  not discriminate anything about `new_stem` or `claims_no_new`**: because the stem is wholly fictitious,
  `archived()` returns `false` for *either* suffix (`--2026` or `--2026-DRAFT`), so the old, pre-fold code
  (`draft = archived("{form}--2026-DRAFT")` unconditionally) would *also* compute `draft = false` and accept
  the row. This is the same role as its pre-existing NO-DRAFT sibling at line 740-746, documented there as
  "the control tests the PREDICATE on a stem that can never be archived" — a denominator/regression guard,
  not a discriminator for this specific bug. Confirmed by reasoning, not by execution (consistent with the
  "reasoned from the code" style the r2 report itself used for this same branch).

- **`f8995a` (NO FINAL, true) — the discriminating plant.** `new_stem = "f8995a--2026"`.
  `draft = pdf_for("f8995a--2026").is_none()` → `false` (pinned immediately above by the assertion). `prior =
  false` (no `f8995a--2025`). `ok` evaluates true → excused, `wrong` empty. **Reasoned reversion:** under the
  pre-fold code, `draft` was computed unconditionally as `archived("{form}--2026-DRAFT")`. For `f8995a` that
  is `pdf_for("f8995a--2026-DRAFT").is_some()` → **true** (the draft genuinely is archived). `claims_no_new`
  (the OR-disjunct, unchanged by this fold) is `true` for a `NO FINAL` cell. So the third conjunct
  `(!claims_no_new || !draft)` becomes `(false || false) = false`, forcing `ok = false` and
  `wrong.push(...)` — which breaks the test's own `assert!(wrong.is_empty() && excused == ["f8995a"])`. This
  matches the commit message's own claim verbatim ("The first run of the f8995a plant FAILED... a NO FINAL
  claim was judged by the draft's presence"). **Confirmed live-history claim by reasoning from the code.**

- **`f8995a` (NO DRAFT, false).** `new_stem = "f8995a--2026-DRAFT"` (else branch, since the cell does not
  contain `"NO FINAL"`). `draft = pdf_for("f8995a--2026-DRAFT").is_some() = true`. `claims_no_new = true`
  (`NO DRAFT` claim). Third conjunct `(!true || !true) = false` → `ok = false` → `wrong.len() == 1`. Matches
  the test's own assertion. This correctly discriminates the false-direction claim on real archive state.

**New finding surfaced (N4, Important): `claims_no_new`'s recognition of `"NO FINAL"` (the OR-disjunct
itself, unchanged by this fold, originally added in the r1/#1 fix) is still never load-bearing in any
plant.** All three new plants — and the pre-existing sibling — carry `**NO PRIOR SIDE**` in the prior cell,
so `claims_no_prior = true` unconditionally satisfies the first-term OR-gate, and genuine absence of the
prior-side PDF unconditionally satisfies the second conjunct. The third conjunct `(!claims_no_new ||
!draft)` is the *only* place `claims_no_new`'s value can matter, and it is only non-vacuous when
`claims_no_new = true`. Deleting `|| ty2026_cell.contains("NO FINAL")` from `claims_no_new` was checked by
reasoning against all three new plants: for each, `claims_no_new` would become `false`, making the third
conjunct vacuously `true` regardless of `draft` — **`ok` stays `true` in every one of the three cases, and
none of the plants would red.** Grepping the whole test module for `"NO FINAL"` confirms every occurrence
sits inside a row whose prior cell is `**NO PRIOR SIDE**`; no plant exists with the prior side genuinely
*present* (a real filename, `claims_no_prior = false`) paired with a `NO FINAL` claim in either direction —
which is exactly the shape needed to make `claims_no_new`'s disjunct load-bearing (see N4 below for the
minimal fix). This is the same class of gap N1 itself described, one level narrower: the specific historical
bug (`new_stem`) is now witnessed; the disjunct it sits beside is not.

### (b) The claim-derived stem across excused-row shapes, and the post-finals regeneration

Traced `port_status_over`'s excused-row construction (`crates/xtask/src/form_delta.rs:504-521`):
```rust
let (has_prior, has_new) = (pdf_for(&a).is_some(), pdf_for(&b).is_some());
let prior = if has_prior { format!("`{a}`") } else { "**NO PRIOR SIDE**".to_string() };
let new = if has_new { format!("`{b}`") } else { no_new.to_string() };
let cell = match (has_prior, has_new) {
    (false, false) => format!("**NO PRIOR SIDE** + {no_new}"),
    (false, true) => "**NO PRIOR SIDE**".to_string(),
    _ => no_new.to_string(),
};
```
The row prints as `| \`{form}\` | yes | {prior} | {new} | {cell} |` — five cells. `parse_work_list_row_full`
binds `prior_cell = cells[3]` and `ty2026_cell = cells[4]` — i.e. exactly the `prior` and `new` variables
above, **never** the combined `cell` (column 5, discarded by both `check_work_list` — via
`parse_work_list_row`'s `.map(|(f, c, p, t, _)| ...)` — and by `port_status_prints_the_committed_work_list`'s
`rows()` closure, per the r2 verification's own finding). Crucially, `new` is **always** either a bare
backtick filename or **exactly** `no_new` (`"**NO DRAFT**"` or `"**NO FINAL**"`, never both) — the `(false,
false)` combination lives only in `cell`, which `new_stem`'s `.contains("NO FINAL")` check never reads. So:

- A `(false, false)` row's `ty2026_cell` is `"**NO FINAL**"` (or `"**NO DRAFT**"`) alone, never
  `"**NO PRIOR SIDE** + **NO FINAL**"`. **No disagreement arises** — the risk the brief hypothesized (a cell
  containing both words) does not materialize given the printer's column layout, confirmed by direct code
  reading, not merely inspection: `pdf_for`, `port_status_over`, `parse_work_list_row_full`, and
  `check_work_list` were all read end to end.
- Verified against the real doc: `f8275`'s prior cell (the only real `(false, ...)` shaped row) is `**NO
  PRIOR SIDE** for the \`--2025\` tag — …` and its TY2026 cell is `**NO DRAFT** — the draft URL served a
  2024 document; archiver refused` — two separate cells, confirmed by reading `design/TY2026_WORK_LIST.md:58`
  directly.
- A row whose new side transitions to *present* (a final archived) leaves the excused table entirely —
  `compute()` succeeds in `port_status_over`, producing a numeric row — so `new_stem` logic in
  `check_work_list`'s `(None, Err(_))` arm is simply not reached for that form any more. Confirmed by
  reasoning through `port_status_over`'s `match compute(&a, &b) { Ok(d) => ..., Err(_) => ... }` dispatch.

**However — new finding N5 (Important), found beyond the brief's specific sub-question, but squarely
within "does it behave correctly for the post-finals regeneration":** `check_work_list`'s **own** pair
computation, `crates/xtask/src/form_delta.rs:763` —
```rust
let pair = super::compute(&format!("{form}--2025"), &format!("{form}--2026-DRAFT"));
```
— is **hardcoded to `--2026-DRAFT`**, unconditionally, for every row, regardless of what the row's own
cells claim. This line is untouched by `2545c506`. It determines which `(cells, pair)` match arm a row
falls into *before* `new_stem`'s logic is ever reached. Reasoned consequence once the doc is regenerated
with `port-status 2025 2026` (real finals): for **any** excused row that truthfully claims `**NO FINAL**`
for a form whose **draft is archived** (the common case — `f8995a`'s own draft is on disk today, and drafts
are described as "archived… EVIDENCE ONLY", persisting rather than being deleted when a final lands) —
`pair = compute(form--2025, form--2026-DRAFT)` **succeeds** (`Ok`, since the draft PDF still exists), while
the doc row is still an excused row (`cells = None`). The match falls into `(None, Ok(_)) =>
wrong.push("excused as having no pair, but form-delta computes one")` — **before `new_stem` or
`claims_no_new` is ever evaluated.** The row is genuinely, truthfully excused (no final exists yet), but the
checker reports it wrong. Symmetrically, a numeric row for a form whose **final** now differs from its
still-archived **draft** would have its doc-committed counts (final-vs-2025) compared against
`d`-from-DRAFT-vs-2025 inside `check_work_list`, which will very likely mismatch (that divergence is the
entire premise of `form-delta`'s existence, per the file's own header). **Net effect: `new_stem`'s fix
(this fold's actual contribution) is correct and witnessed for the narrow excused-row conjunct it patches,
but the surrounding routing in `check_work_list` remains draft-only, so `the_committed_work_list_matches_
form_delta_at_head` will not actually validate a doc regenerated with real finals without a further,
unmade change to line 763** — this contradicts the commit message's own framing ("the exact post-finals
path the r1 verification warned about" is closed). The failure mode is fail-**safe** (spurious red, not a
false pass), and it is not tracked in `FOLLOWUPS.md` (`grep -n -i "check_work_list\|port-status" FOLLOWUPS.md`
shows only FR-50, marked "✅ … FR-50 is closed" — this residual is not mentioned there or anywhere else
tagged `AFTER FINALS`).

### (c) N2 — does `partial_keys == shrunk` discriminate a `surface`-ignoring printer?

Confirmed by reasoning. Mutation: `port_status_over` ignores its `surface` argument and iterates the full,
internal `emitting_surface()` (18 stems) instead. `p` (built from the genuine, unshrunk `port_status()`
call at line 617) is unaffected — the mutation is invisible there (as the r2 report already established).
`partial` (fed the 17-element `shrunk`) becomes the full 18-row output under this mutation. The pre-fold
`assert_ne!(partial_keys, surface)` — `18-elem == 18-elem` — would fail to red (this was N2's original gap).
The **new** `assert_eq!(partial_keys, shrunk, ...)` at lines 642-645 — `18-elem != 17-elem shrunk` — **reds
correctly**, independent of the earlier line 619-623 check. Silent-printer mutation (loop body deleted):
`partial_keys = {}`; the new `assert_eq!({}, shrunk)` also reds (17-elem `shrunk` ≠ `{}`), closing the gap
N2 identified in the old `assert_ne!`-only form. **Resolved, self-sufficient.**

### (d) N3 — renamed

`grep -n "claims_no_draft" crates/xtask/src/form_delta.rs` → no hits. Every use (the variable declaration,
its two consuming expressions, and the nearby comment) now reads `claims_no_new`. **Resolved.**

### (e) Regression check

`cargo nextest` output (`/tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r3.txt`):
```
work list: 14 compared, 4 excused, 18 stems on the emitting surface
test form_delta::tests::the_committed_work_list_matches_form_delta_at_head ... ok
test form_delta::tests::port_status_prints_the_committed_work_list ... ok
test form_delta::tests::the_work_list_checker_reds_on_every_planted_row ... ok
Summary [1.198s] 3 tests run: 3 passed, 126 skipped
```
Counts (14/4/18) match the r2 baseline exactly. **No regression in either test.**

---

## New findings

| # | Severity | Where | What is wrong | Minimal change |
|---|---|---|---|---|
| N4 | **Important** | `crates/xtask/src/form_delta.rs:818-819` (`claims_no_new`'s `\|\| ty2026_cell.contains("NO FINAL")` disjunct); plants at `crates/xtask/src/form_delta.rs:710-737` | Every plant carrying a `NO FINAL` claim also carries `**NO PRIOR SIDE**` in the prior cell, which makes the whole `ok` formula true regardless of `claims_no_new`'s value (confirmed by reasoning: deleting the `NO FINAL` disjunct leaves all three new plants passing). Only `new_stem`'s stem-selection is witnessed by the f8995a plant; the disjunct that decides whether `"NO FINAL"` is even recognized as a claim at all remains unwitnessed in isolation — the same class of gap the original N1 named, one conjunct over. | Add a plant with the **prior side genuinely present** (a real `--2025` filename, `claims_no_prior = false`) paired with a truthful `**NO FINAL**` claim on a stem with no `--2026` PDF today (e.g. `f6251`, whose `--2025` is archived and no `f6251--2026` exists): `` "| `f6251` | yes | `f6251--2025` | **NO FINAL** — planted | — |\n" ``, asserting `wrong.is_empty()`. This makes `claims_no_new`'s value load-bearing in both the first-term OR-gate and the third conjunct, since `claims_no_prior` is false. |
| N5 | **Important** | `crates/xtask/src/form_delta.rs:763` (`check_work_list`'s `let pair = super::compute(&format!("{form}--2025"), &format!("{form}--2026-DRAFT"));`), untouched by this fold | The pair used to decide whether a doc row is treated as numeric/excused/wrong is hardcoded to the `--2026-DRAFT` tag regardless of the row's own claims. Once the doc is regenerated with `port-status 2025 2026`, any truthfully-excused `**NO FINAL**` row for a form whose draft is still archived (the expected, common case — drafts persist as evidence) will compute `pair = Ok(d)` (against the still-present draft) and fall into `(None, Ok(_)) => wrong.push("excused as having no pair, but form-delta computes one")` — before `new_stem`/`claims_no_new` are ever reached. Symmetrically, numeric rows built from real final-vs-2025 counts will be compared against draft-vs-2025 counts and very likely mismatch. This means `the_committed_work_list_matches_form_delta_at_head` cannot actually validate a post-finals-regenerated doc without a further code change — contradicting the commit message's claim to have closed "the exact post-finals path." Fails safe (spurious red), not silently. Not tracked in FOLLOWUPS.md (checked: only FR-50 mentions `port-status`, marked closed). | Derive the checked tag the same way `new_stem` now does, or from an explicit `new_tag: &str` parameter threaded into `check_work_list` (and its two callers) so the pair is computed against `--2026` when the row is excused-with-NO-FINAL / numeric-post-finals, and `--2026-DRAFT` otherwise — mirroring the fix already applied to the excused-row branch. File as a FOLLOWUPS.md entry owned by "the TY2026 port (AFTER FINALS)" if not fixed immediately, since nothing breaks *today* (no `--2026` final PDF exists anywhere in the repo — confirmed via `find design/forms crates/btctax-forms/forms -iname '*--2026.pdf'` → empty). |

No Minor or Nit findings beyond what N1-N3 already recorded (resolved).

---

## Counts

**0 Critical / 2 Important / 0 Minor / 0 Nit** (new findings; N1 and N2 from r2 are resolved with one
residual folded into N4 rather than left open under the old number; N3 is fully resolved).

No Critical: neither `check_work_list` nor the `port_status_prints_the_committed_work_list` plant is
structurally incapable of failing — every live `assert_eq!`/`assert_ne!`/`assert!` pair traced was
confirmed, by reasoning from the code (and one by the live nextest run), to depend on real production-code
output and to red under a genuine mutation. Both new findings fail **safe** (spurious red / silent gap in a
plant), never silent-pass.

**Headline:** `2545c506` resolves N2 and N3 cleanly, and resolves N1's central, historically-manifested bug
(`new_stem`'s stem-selection is now witnessed and confirmed to have broken the f8995a plant under the old
code) — but the fold's own commit message overclaims: `claims_no_new`'s `"NO FINAL"` recognition is still
never load-bearing in any plant (N4, narrower descendant of N1), and a wholly separate, untouched line —
`check_work_list`'s hardcoded `--2026-DRAFT` pair computation — means the conformance test cannot actually
validate a doc regenerated against real finals at all (N5). Neither is urgent today (no `--2026` final PDF
exists anywhere in the repo yet), and both fail safe rather than silently, but "the exact post-finals path"
is not yet closed.
