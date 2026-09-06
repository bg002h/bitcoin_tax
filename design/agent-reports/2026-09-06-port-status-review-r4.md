# Independent re-verification (r4) — `xtask port-status` fold `ff339967`

**Reviewer:** independent read-only verifier (sonnet subagent), no file edits made outside this report.
**Date:** 2026-09-06
**Fold under review:** `ff339967a68264167e34120c4c52a663bb4d9531` (`main` HEAD at time of review; later
commits touch only `design/` docs).
**Prior chain:** `design/agent-reports/2026-09-06-port-status-review-r3.md` (0C/2I: N4/N5) and its
settled `-VERIFICATION.md` (2/2 TRUE).

**Command run once**, per instructions:
```
cargo nextest run --locked -p xtask -E 'test(work_list) | test(port_status)' --no-capture
  → /tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r4.txt
```
Result: **3 tests run: 3 passed, 126 skipped.**
`the_committed_work_list_matches_form_delta_at_head` printed `work list: 14 compared, 4 excused, 18
stems on the emitting surface` — matches r2/r3's baseline and the fold's own commit message exactly. No
regression in either targeted test.

---

## N4 / N5 checklist

| # | r3 severity | Status at `ff339967` |
|---|---|---|
| N4 | Important — `claims_no_new`'s `NO FINAL` disjunct never load-bearing in any plant (prior side always `**NO PRIOR SIDE**`) | **Resolved.** New plant (`crates/xtask/src/form_delta.rs:750-766`): `f6251`, prior side a real filename (`` `f6251--2025` ``, `claims_no_prior=false`), new side `**NO FINAL**` under tag `("2025","2026")`. Confirmed by reasoning (§c below) that deleting the `NO FINAL` disjunct flips this exact assertion to fail; confirmed live-passing under the actual (undeleted) code. |
| N5 | Important — `check_work_list`'s pair computation hardcoded to `--2026-DRAFT` regardless of the doc's own claims; a post-finals regeneration cannot validate | **Resolved for the targeted defect**, with one new, narrower gap surfaced (N6 below). `design/TY2026_WORK_LIST.md:2` now carries `<!-- tags: 2025 2026-DRAFT -->`; `work_list_tags` (new, `form_delta.rs:789-798`) parses it; `check_work_list` (`:802-806`) and `check_work_list_with` (`:808-...`) thread `(prior_tag, new_tag)` through both the pair computation (`:822-825`, no longer hardcoded) and a new `claim_word_matches_tag` gate (`:881-884`) that requires the claim's WORD to match the tag. |

---

## Verdicts

### (a) N5 — the tags line and the silent fallback

`design/TY2026_WORK_LIST.md:2` (directly beneath the `# TY2026 port work list` title, confirmed by
reading the file) reads exactly:
```
<!-- tags: 2025 2026-DRAFT -->
```
`work_list_tags` (`form_delta.rs:789-798`) finds the first line starting with `<!-- tags:`, strips the
delimiters, and splits on whitespace into `(prior, new)`. `check_work_list` (`:802-806`):
```rust
let (prior_tag, new_tag) =
    work_list_tags(doc).unwrap_or_else(|| ("2025".to_string(), "2026-DRAFT".to_string()));
```
falls back to the literal old hardcoded default when the line is absent or unparsable.

**Verdict: the silent fallback is not the right behaviour — it should red (or at minimum be
plant-proven) rather than default, and this fold leaves that gap unwitnessed.** Reasoning:

1. **The fallback tuple is byte-identical to the real document's declared tags** (`"2025"`,
   `"2026-DRAFT"`). This means the *only* call site that runs `check_work_list` end-to-end against a
   real, multi-row document with an actual `<!-- tags -->` line —
   `the_committed_work_list_matches_form_delta_at_head` (`form_delta.rs:553-554`, reads
   `design/TY2026_WORK_LIST.md` from disk) — **cannot discriminate a working parser/wiring from a
   completely broken one**, because both produce the identical `(prior_tag, new_tag)`. Grepping the
   whole test module (`grep -n "tags:" crates/xtask/src/form_delta.rs`) shows the *only* other place a
   `<!-- tags: -->` line appears is one isolated unit assertion (`:770-773`,
   `work_list_tags("<!-- tags: 2025 2026 -->\n...")`) that checks the parser **in isolation** — it is
   never threaded through `check_work_list` on a multi-row document. So: mutate `check_work_list` to
   ignore its own `work_list_tags(doc)` result and unconditionally call
   `check_work_list_with(doc, "2025", "2026-DRAFT")` (i.e., silently revert N5's actual fix while leaving
   the parser intact) — **no test in the suite goes red**, because (i) the real doc's declared tags equal
   that hardcoded pair, and (ii) no other document with a *different* declared tags line is ever run
   through plain `check_work_list`. This is exactly the class the repo's B1 rule targets: an instrument
   ("the checker reads the doc's tags") never observed discriminating true from false.
2. Consequently, once the doc is genuinely regenerated with real finals (`<!-- tags: 2025 2026 -->`) and
   that line is accidentally dropped (a plausible authoring slip — the person forgets to add/update it,
   the exact failure mode N5 was written to close), `check_work_list` silently reverts to checking
   against the *draft* pair while the doc's cells were generated against the *final* pair:
   - **Excused rows are still safe** — a truthful `**NO FINAL**` row's `claim_word_matches_tag` gate
     mismatches the fallback's draft expectation deterministically (traced in §c) and reds.
   - **Numeric rows are only probabilistically safe.** The checker recomputes
     `compute(form--2025, form--2026-DRAFT)` and compares its counts against the doc's committed
     final-vs-2025 counts; these differ (and red) *only if* the form's final content actually differs
     from its draft — "the entire premise of form-delta's existence" per the r3 report, but not a logical
     guarantee for every single stem. A form whose final happens to be byte-identical to its already-
     archived draft would validate "correctly" by coincidence, against the wrong reference file, with no
     plant anywhere proving this either way.

This is filed as new finding **N6** below (Important) — it does not undermine N4 or N5's resolution of
their own named defects, but it is a new gap this fold's own mechanism introduces and leaves unwitnessed.

### (b) The claim-word rule, walked over the real doc's four excused rows

Confirmed via `grep -n "NO PRIOR SIDE\|NO DRAFT\|NO FINAL" design/TY2026_WORK_LIST.md` → rows 57
(`f1040`), 58 (`f8283`), 59 (`f8275`), 60 (`f8995a`) — matches the live run's "4 excused" exactly.
Walked each under the doc's own declared tags `("2025", "2026-DRAFT")`, cross-checked against the
filesystem (`design/forms/…`, `crates/btctax-forms/forms/…`):

| Row | `claims_no_prior` | `claims_no_new` | word matches tag? | `prior` on disk | `draft` on disk | `ok` |
|---|---|---|---|---|---|---|
| f1040 (NO DRAFT) | false (`` `f1040--2025` ``) | true | yes (NO DRAFT under draft tag) | `true` (`design/forms/2025/f1040--2025.pdf`) | `false` (`design/forms/2026/f1040--2026-DRAFT.pdf` absent, confirmed by `find`) | **true** |
| f8283 (NO DRAFT) | false | true | yes | `true` | `false` (no `f8283--2026-DRAFT.pdf`) | **true** |
| f8275 (NO DRAFT) | true (`**NO PRIOR SIDE**`) | true | yes | `false` (no `f8275--2025.pdf`) | `false` (no `f8275--2026-DRAFT.pdf`) | **true** |
| f8995a (neither word; cell reads "draft archived (...)")| true | **false** (cell contains neither literal) | vacuously yes | `false` (no `f8995a--2025.pdf`) | n/a (vacuous) | **true** |

**All four still excused** — confirmed both by direct filesystem check and by the passing
`the_committed_work_list_matches_form_delta_at_head` run.

**Hypothetical post-finals row**, `("2025","2026")`, `` | `f8275` | yes | **NO PRIOR SIDE** | **NO
FINAL** | — | ``: `claims_no_prior = true`; `claims_no_new = true`; `tag_is_draft = false`;
`claim_word_matches_tag = (!false && ty2026_cell.contains("NO FINAL")) = true`. `ok` reduces to
`(!claims_no_new || !draft) = !draft = !archived("f8275--2026")`. **Confirmed: excused iff no
`f8275--2026` PDF exists** — exactly as hypothesized, traced directly from the `ok` formula.

### (c) N4 — the `f6251` plant

`crates/xtask/src/form_delta.rs:750-766`:
```rust
assert!(super::pdf_for("f6251--2026").is_none(), "the plant assumes no TY2026 final for f6251");
let (_, excused, wrong) = check_work_list_with(
    "| `f6251` | yes | `f6251--2025` | **NO FINAL** — planted | — |\n", "2025", "2026",
);
assert!(wrong.is_empty() && excused == ["f6251"], ...);
let (_, _, wrong) = check_work_list_with(..., "2025", "2026-DRAFT");
assert_eq!(wrong.len(), 1, "NO FINAL under a draft tag names the wrong word: {wrong:?}");
```
- **`pdf_for("f6251--2026")` is `None` today** — confirmed both by the assertion's own live pass (in the
  captured nextest run) and independently by `find design/forms crates/btctax-forms/forms -iname
  '*--2026.pdf'` → empty (no non-draft `2026` final anywhere in the repo).
- **Prior side is present**: `` `f6251--2025` `` is a real filename, not `**NO PRIOR SIDE**`, so
  `claims_no_prior = false` — confirmed also on disk (`design/forms/2025/f6251--2025.pdf` exists).
- **Reasoned reversion (deleting the `NO FINAL` disjunct):** with `claims_no_prior = false`, the first
  OR-gate `(claims_no_prior || claims_no_new)` depends entirely on `claims_no_new`. If
  `claims_no_new` were `ty2026_cell.contains("NO DRAFT")` alone (the `NO FINAL` disjunct deleted), then
  for this row (`ty2026_cell` contains `"NO FINAL"`, not `"NO DRAFT"`) `claims_no_new` becomes `false`,
  the OR-gate becomes `(false || false) = false`, `ok = false`, and `wrong.push(...)` fires — which
  breaks `assert!(wrong.is_empty() && excused == ["f6251"])` under tag `("2025","2026")`. **This is
  exactly the assertion N4 named as unwitnessed; it is now witnessed and load-bearing.**
- **Under `("2025","2026-DRAFT")`, the same row is wrong via `claim_word_matches_tag`**:
  `tag_is_draft = true`; `ty2026_cell` contains `"NO FINAL"`, not `"NO DRAFT"`, so
  `claim_word_matches_tag = false || (true && false) || (false && true) = false` → `ok = false` →
  `wrong.len() == 1`. Matches the test's own assertion and the live passing run.

### (d) `port_status_over`'s NO DRAFT/NO FINAL choice vs. the checker's rule

`port_status_over` (`form_delta.rs:469`): `let no_new = if new_tag.contains("DRAFT") { "**NO DRAFT**" }
else { "**NO FINAL**" };`. `check_work_list_with`'s `tag_is_draft` (`:881`):
`new_tag.contains("DRAFT")`. **Identical predicate, on the identical `new_tag` string.** So any doc
freshly regenerated by `port_status_over(surface, prior_tag, new_tag)` is structurally guaranteed to
excuse-cell-word-match its own tag — a printer-emitted `**NO DRAFT**` can only occur when `new_tag`
contains `"DRAFT"`, and `**NO FINAL**` only when it does not — as long as `check_work_list` is invoked
with the *same* `(prior_tag, new_tag)` the printer used. That "as long as" is precisely the tags-line
dependency examined in (a): the consistency is real and structural for `check_work_list_with` called
explicitly, but for the plain `check_work_list(doc)` entry point it is mediated by `work_list_tags`
parsing the doc's own declaration correctly — a step this fold leaves unwitnessed (N6).

### (e) Regression check

Captured once (`/tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r4.txt`):
```
work list: 14 compared, 4 excused, 18 stems on the emitting surface
test form_delta::tests::port_status_prints_the_committed_work_list ... ok
test form_delta::tests::the_committed_work_list_matches_form_delta_at_head ... ok
test form_delta::tests::the_work_list_checker_reds_on_every_planted_row ... ok
Summary [ 0.807s ] 3 tests run: 3 passed, 126 skipped
```
**3/3; 14/4/18 — matches the fold's own commit message exactly. No regression.**

---

## New findings

| # | Severity | Where | What is wrong | Minimal change |
|---|---|---|---|---|
| N6 | **Important** | `crates/xtask/src/form_delta.rs:789-798` (`work_list_tags`), `:802-806` (`check_work_list`'s `unwrap_or_else` fallback), and its only real-document call site `:553-554` | The fallback tuple `("2025", "2026-DRAFT")` is byte-identical to `design/TY2026_WORK_LIST.md`'s actual declared tags line, so the *only* test exercising `check_work_list` end-to-end against a real multi-row document cannot discriminate a working tags-parse-and-thread path from a completely broken one (e.g., a mutation that ignores `work_list_tags`'s result and always calls `check_work_list_with(doc, "2025", "2026-DRAFT")` passes every existing test). No plant feeds a document whose declared tags differ from the fallback default through the plain `check_work_list` entry point. Consequence for the post-finals regeneration this fold targets: if the doc's `<!-- tags -->` line is ever dropped or left stale after finals land, excused rows still red (via `claim_word_matches_tag`, deterministic), but numeric rows only red *if* a given form's final content actually differs from its still-archived draft — not a logical guarantee for every stem, so a narrow, coincidence-dependent false-pass window remains at exactly the moment N5 was written to close. Fails safe in the common case; not fail-safe by construction. Dormant today (no `--2026` final PDF exists anywhere in the repo — confirmed via `find design/forms crates/btctax-forms/forms -iname '*--2026.pdf'` → empty). | Add a plant that runs a full document string with `<!-- tags: 2025 2026 -->` (a tag pair differing from the fallback) through plain `check_work_list` (not `_with`), containing at least one excused NO-FINAL row, and assert it excuses correctly (this would red under the "ignore the parsed tags" mutation above, unlike anything in the suite today). Separately, consider making `work_list_tags`'s absence a hard error for the production call site (`:554`) — via `.expect("design/TY2026_WORK_LIST.md must declare its own <!-- tags: --> line")` — rather than silently defaulting, since a defaulted real document is indistinguishable on the page from one whose tags were never written. |

No Critical findings: neither `check_work_list` nor `port_status_prints_the_committed_work_list` is
structurally incapable of failing — every `assert!`/`assert_eq!` pair traced in (b)/(c) depends on real
production-code output and reds under the specific mutation examined. N6 fails safe (spurious red is
still the *likely* outcome even if the tags line is dropped), not silently, in the cases actually
witnessed.

---

## Counts

**0 Critical / 1 Important / 0 Minor / 0 Nit** (new finding N6; N4 and N5 both resolved and re-verified
with direct evidence — filesystem state, live test run, and code-level reasoning traced to the exact
lines and assertions involved).

**Headline:** `ff339967` genuinely resolves N4 (the `NO FINAL` disjunct is now load-bearing, witnessed by
a plant with a present prior side) and N5 (the hardcoded `--2026-DRAFT` pair computation is gone; the
checker reads the document's own declared tags and gates excuse wording against them) — 3/3, 14/4/18,
no regression. But the fix introduces one new, narrower gap: the tags-parsing-and-threading mechanism
itself is never witnessed discriminating a working wiring from a silently-reverted one, because the real
document's declared tags happen to equal the old hardcoded default — the identical "never seen
discriminating" shape N4/N5 themselves were filed against, one layer up, and dormant only because no
`--2026` final exists yet.
