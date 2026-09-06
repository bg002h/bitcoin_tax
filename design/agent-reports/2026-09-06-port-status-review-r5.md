# Independent re-verification (r5) — `xtask port-status` fold `386d01c7`

**Reviewer:** independent read-only verifier (sonnet subagent), no file edits made outside this report.
**Date:** 2026-09-06
**Fold under review:** `386d01c7068234e0ea1ebdb4b723332374dedee4` (`main` HEAD at time of review; later
commits do not touch `crates/xtask/`).
**Prior chain:** `design/agent-reports/2026-09-06-port-status-review-r4.md` (0C/1I: N6) and its settled
`-VERIFICATION.md` (1/1 TRUE).

**Command run once**, per instructions:
```
cargo nextest run --locked -p xtask -E 'test(work_list) | test(port_status)' --no-capture
  → /tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r5.txt
```
Result: **3 tests run: 3 passed, 126 skipped.**
`the_committed_work_list_matches_form_delta_at_head` printed `work list: 14 compared, 4 excused, 18
stems on the emitting surface` — byte-identical to r4's baseline and to this fold's own commit message.
No regression.

---

## N6 checklist

| # | r4 severity | Status at `386d01c7` |
|---|---|---|
| N6 | Important — `check_work_list`'s `unwrap_or_else(("2025","2026-DRAFT"))` fallback is byte-identical to the committed document's own declared tags, so the parse-and-thread mechanism was never witnessed discriminating a working wiring from a silently-reverted one | **Resolved.** `check_work_list` (`crates/xtask/src/form_delta.rs:819-828`) no longer defaults: `let Some((prior_tag, new_tag)) = work_list_tags(doc) else { return (..., vec!["... declares no ... line ..."]) }`. A document with no parseable tags line is unconditionally `wrong`, with empty `compared`/`excused`. `the_committed_work_list_matches_form_delta_at_head` (`:551-583`) now also asserts `work_list_tags(&doc) == Some(("2025","2026-DRAFT"))` directly against the committed file, *before* calling `check_work_list(&doc)`. `the_work_list_checker_reds_on_every_planted_row` (`:771-789`) runs the **same** `f8995a` `**NO FINAL**` row through plain `check_work_list` under three documents: `<!-- tags: 2025 2026 -->` (excused), `<!-- tags: 2025 2026-DRAFT -->` (wrong — word/tag mismatch), and no tags line at all (wrong — refusal message, asserted to contain `"tags"`). |

---

## Verdicts

### (a) Would a document that lost its tags line red `the_committed_work_list_matches_form_delta_at_head`?

Yes, twice over — read directly from `form_delta.rs:551-583`:

1. `assert_eq!(work_list_tags(&doc), Some(("2025".to_string(), "2026-DRAFT".to_string())), ...)`
   (`:554-558`) compares the parse of whatever is on disk against the literal expected pair. If the
   `<!-- tags: -->` line were removed from `design/TY2026_WORK_LIST.md`, `work_list_tags` returns `None`
   (its scan finds no line starting with `<!-- tags:`), `None != Some(("2025","2026-DRAFT"))`, and this
   assertion fails on its own, independent of anything downstream.
2. Even absent assertion 1, `check_work_list(&doc)` (`:559`) would itself take the new `else` branch
   (`:820-825`) and return `wrong = ["the work list declares no \`<!-- tags: <prior> <new> -->\` line —
   nothing can be checked against it"]` — a non-empty `wrong`, which fails the later
   `assert!(wrong.is_empty(), ...)` (`:578-582`).

Both assertions are live production-code paths over the real file at `root.join("design/TY2026_WORK_LIST.md")`
(`:552-553`), not synthetic strings — confirmed by reading the test and independently confirmed by
`grep -n "<!-- tags:" design/TY2026_WORK_LIST.md` returning exactly one match, `2:<!-- tags: 2025
2026-DRAFT -->`, matching the asserted value byte-for-byte.

### (b) The N6 plant — three mutations, three assertions

`form_delta.rs:771-789`, over the shared row `"| \`f8995a\` | yes | **NO PRIOR SIDE** | **NO FINAL** —
planted | — |\n"`:

| Case | Document | Expected | Assertion |
|---|---|---|---|
| 1 | `<!-- tags: 2025 2026 -->\n{row}` | excused, `wrong` empty | `wrong.is_empty() && excused == ["f8995a"]` |
| 2 | `<!-- tags: 2025 2026-DRAFT -->\n{row}` | wrong (word/tag mismatch) | `wrong.len() == 1` |
| 3 | `{row}` alone (no tags line) | wrong (refusal) | `wrong.len() == 1 && wrong[0].contains("tags")` |

**Mutation "ignore `work_list_tags`, always call `check_work_list_with(doc, "2025", "2026-DRAFT")`"**
(i.e. revert to the always-draft fallback, silently dropping the `else`-branch refusal):
- Case 1: pair forced to draft. `tag_is_draft = true`; the row contains `"NO FINAL"`, not `"NO DRAFT"`,
  so `claim_word_matches_tag = false` → `ok = false` → `wrong` non-empty → **assertion 1 fails**
  (`wrong.is_empty()` is false).
- Case 3: the mutation bypasses the `None`-refusal entirely, so this document is now evaluated with the
  forced draft pair exactly like case 2 — `wrong.len() == 1` still holds (coincidentally correct count),
  but the message is the ordinary `"{form}: excused as prior=... TY2026=..., but on disk prior=... draft=..."`
  string, which does **not** contain the substring `"tags"` → **assertion 3's second half fails**
  (`wrong[0].contains("tags")` is false).

**Mutation "ignore `work_list_tags`, always call `check_work_list_with(doc, "2025", "2026")`"** (force the
final pair unconditionally):
- Case 2: pair forced to final regardless of the declared draft tag. `tag_is_draft = false`; row contains
  `"NO FINAL"` → `claim_word_matches_tag = true`; `archived("f8995a--2026")` is `false` (confirmed no
  `--2026` final PDF anywhere in the repo, per r4's `find`) and `archived("f8995a--2025")` is `false` →
  `ok = true` → `wrong` empty → **assertion 2 fails** (`wrong.len() == 1` expected, got 0).
- Case 3: same forced-final evaluation as case 2 → `wrong` empty → **assertion 3's first half fails**
  (`wrong.len() == 1` expected, got 0).

So: for **each** of the two "ignore the parsed tags, use one fixed pair" mutations, at least one of the
three assertions fails — the draft-forcing mutation is caught by cases 1 and 3; the final-forcing
mutation is caught by cases 2 and 3. Case 3's `.contains("tags")` check specifically is what catches a
mutation that would otherwise reproduce case 2's *numeric* outcome (`wrong.len()==1`) by accident — a
count-only assertion there would have been vacuous against exactly this mutation.

### (c) Do the older (pre-r4) plants still discriminate under `dflt`?

`dflt = |doc: &str| check_work_list_with(doc, "2025", "2026-DRAFT")` (`:661`) calls the untouched
`check_work_list_with` directly — the diff makes no change to that function's body (confirmed: the diff
hunk for `check_work_list` only rewrites the `unwrap_or_else` into a `let...else`; `check_work_list_with`,
`:830-924`, has zero lines changed). Before this fold, these same bare rows (no `<!-- tags -->` line) ran
through `check_work_list`, which parsed `None` and fell back to the *identical* hardcoded pair
`("2025","2026-DRAFT")` — so `dflt` reproduces that exact prior code path verbatim, just via direct call
instead of indirection through the now-removed fallback. Spot-checked three, by tracing
`check_work_list_with`'s live logic (`:830-924`) against each:

- **Cell off-by-one** (`| \`f6251\` | 62 | 0 | 0 | 1 | port |`, `:662-663`): a numeric row, `cells = Some((62,0,0,Some(1)))`,
  `pair = compute("f6251--2025","f6251--2026-DRAFT")`. The claimed cell values are compared against
  `d.common.len(), d.added.len(), d.removed.len()` and `d.label_moved.len()` computed live at HEAD
  (`:852-881`) — a genuine recomputation, not a tautology. Confirmed passing live (`the_work_list_checker_reds_on_every_planted_row ... ok`), i.e. the live counts do differ from the planted `(62,0,0,1)` claim, exactly as the assertion requires.
- **NO PRIOR SIDE with prior on disk** (`| \`f1040\` | yes | **NO PRIOR SIDE** — planted | no draft either | — |`, `:698-704`):
  `claims_no_prior = true`; `claims_no_new = false` (cell text is lowercase prose, matches neither
  `"NO DRAFT"` nor `"NO FINAL"`); `claim_word_matches_tag = true` (vacuously, since `!claims_no_new`);
  `prior = archived("f1040--2025")` — `design/forms/2025/f1040--2025.pdf` exists (per r4's filesystem
  table) → `prior = true`. `ok = (true) && true && (!true || !true) = false` → **red**, for the exact
  claimed reason (a false "no prior side" claim contradicted by a real archived fixture).
- **"Claims neither"** (`| \`f1040\` | yes | \`f1040--2025\` | nothing claimed | — |`, `:705-709`): prior
  cell is a real filename (not `**NO PRIOR SIDE**`) so `claims_no_prior = false`; new cell contains
  neither `"NO DRAFT"` nor `"NO FINAL"` so `claims_no_new = false`. `ok = (false || false) && ... = false`
  unconditionally (first AND-term is false regardless of the rest) — a row that names an excuse-shaped
  layout but claims nothing is structurally incapable of excusing itself. **Red**, as claimed, and this
  is a real discriminator: any row that names at least one real claim word takes the other arms of the
  `match`.

All three still discriminate for the reasons their own assertion messages claim, and the live run
confirms every assertion in the function held (`... ok`, not a partial pass — a single `#[test]` fn
reports failure on the first failing `assert!`/`assert_eq!`, so a green result here means all of them,
old and new, passed).

### (d) Is `work_list_tags` robust to placement and whitespace? Is a second tags line ambiguous?

`form_delta.rs:804-815`, **unchanged by this diff** (confirmed: no hunk touches this function's body —
only its doc comment context shifted; the added prose is about `check_work_list`, not this function):

```rust
fn work_list_tags(doc: &str) -> Option<(String, String)> {
    let l = doc.lines().find(|l| l.trim_start().starts_with("<!-- tags:"))?;
    let inner = l.trim().trim_start_matches("<!-- tags:").trim_end_matches("-->").trim();
    let mut it = inner.split_whitespace();
    Some((it.next()?.to_string(), it.next()?.to_string()))
}
```

- **Placement:** `doc.lines().find(...)` scans every line of the document, not just line 1 — the tags
  line can appear anywhere. Confirmed live: the plant strings in (b) prepend it (`"<!-- tags: ... -->\n{row}"`);
  the real document carries it at line 2, under the `#` title (confirmed above).
- **Whitespace:** `.trim_start()` before the prefix check tolerates leading indentation; `.trim()`,
  `trim_start_matches`, `trim_end_matches`, and a second `.trim()` before `split_whitespace()` tolerate
  arbitrary internal/trailing whitespace around the delimiters and between the two tokens.
- **A second tags line is ambiguous — first wins, silently.** `.find()` returns the first matching line
  and stops; a document with two `<!-- tags: -->` lines (e.g. a stale one left behind by a bad merge,
  plus a freshly regenerated one) would silently use the first and never surface the second. This
  behavior predates `386d01c7` (the function body is untouched by this diff — it was already this way
  under N5's original fix) and is not exercised by any plant. **Noting per instructions, not counted as
  a new finding of this fold** — it existed in `ff339967` too, and the r4 report's N6 did not name it.

### (e) Regression check

Captured once (`/tmp/claude-1000/-scratch-code-bitcoin-tax/ps-r5.txt`):
```
test form_delta::tests::port_status_prints_the_committed_work_list ... ok
work list: 14 compared, 4 excused, 18 stems on the emitting surface
test form_delta::tests::the_committed_work_list_matches_form_delta_at_head ... ok
test form_delta::tests::the_work_list_checker_reds_on_every_planted_row ... ok
Summary [ 0.808s ] 3 tests run: 3 passed, 126 skipped
```
**3/3; 14/4/18 — matches the fold's own commit message and r4's baseline exactly. No regression.**

---

## New findings

No Critical or Important findings. `check_work_list` is no longer structurally incapable of failing on a
missing/unparseable tags line (it refuses unconditionally, per (a)); the tags-parse-and-thread mechanism
is now witnessed discriminating a working wiring from either single-fixed-pair reversion (per (b)); the
pre-existing per-row plants are unaffected and still discriminate for their claimed reasons (per (c)).

| # | Severity | Where | What is wrong | Minimal change |
|---|---|---|---|---|
| N7 | Minor | `crates/xtask/src/form_delta.rs:804-815` (`work_list_tags`) | A tags line present but **malformed** (e.g. `<!-- tags: 2025 -->` with only one token, or an empty comment) also returns `None` from `work_list_tags`, and `check_work_list`'s refusal message reads *"the work list declares no `<!-- tags: <prior> <new> -->` line"* — which is imprecise when a line *was* present but failed to parse. This is a strict improvement over `ff339967` either way (previously a malformed line silently defaulted with no signal at all), so it is not a regression and does not gate; it is a wording nit for a case no plant currently exercises. | Distinguish "no line found" from "line found but unparseable" in the message, or add a plant for the malformed-but-present case. |
| N8 | Minor / documentation | (inherent to the design, not this diff) | A tags line that is present, well-formed, and **stale** (declares `2026-DRAFT` after the document's cells were actually regenerated against a real `2026` final, e.g. a hand-edit slip) is *not* caught by this fold's fix — `work_list_tags` parses it successfully, so the new hard-refusal path never triggers. This is the residual half of r4's own N6 discussion ("dropped **or left stale**"): excused rows still red deterministically under a stale draft tag (traced: a genuine `**NO FINAL**` excuse cell mismatches `claim_word_matches_tag` against a stale draft tag exactly as in plant case 2 of (b) above), but numeric rows retain the same probabilistic exposure r4 already named — they only red if the given form's final content actually differs from its still-archived draft. `386d01c7`'s commit message says the tags line is "load-bearing end to end," which is true for its *absence*; it does not extend to a self-declaration that is present but wrong. Not new to this commit (inherited from N5's original design, and dormant — no `--2026` final exists anywhere in the repo today), and not what N6 as filed in r4 was about (N6 was specifically the fallback-equals-real-tags blind spot, now closed). | If ever promoted from a repo-regression test to a generator-time check: have `port_status_over` write the tags line itself as part of regeneration rather than relying on a hand-maintained comment, removing the staleness channel structurally. |

---

## Counts

**0 Critical / 0 Important / 2 Minor / 0 Nit.**

**Headline:** `386d01c7` genuinely resolves N6 — `check_work_list` now hard-refuses (empty
`compared`/`excused`, one `wrong` entry naming the missing tags line) instead of silently defaulting, the
committed document's own declared tags are asserted directly (`assert_eq!(work_list_tags(&doc), Some(...))`),
and the parse-and-thread mechanism is witnessed discriminating both "ignore the parse, force draft" and
"ignore the parse, force final" reversions via three assertions over the same planted row; the
pre-existing per-row plants are unaffected and re-verified still discriminating under `dflt`. 3/3,
14/4/18, no regression. Two Minor/non-gating notes recorded (imprecise refusal wording on a malformed-but-present
line; a present-but-stale tags line retains the same probabilistic numeric-row exposure r4 already named
for that half of the "dropped or left stale" phrase) — neither is introduced by this commit, and neither
is what N6 as filed was about.
