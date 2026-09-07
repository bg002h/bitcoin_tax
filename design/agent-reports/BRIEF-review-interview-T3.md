# Brief — seam review of interview build T3 (the document census, the direction tables, the panel)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T3 build is `0807335b`; its pre-review fold of D1/D11 is the commit after it). Read-only
for the record: every plant is made in YOUR worktree and reverted (`git checkout -- <file>` is fine
there); no commits, no subagents. `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
before any cargo command; scoped runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`);
the instruments are `cargo run -q -p xtask -- census-join` / `stop-list` / `line-coverage`. **Environment,
not findings:** the archived PDFs are gitignored, so six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a PDF-less
worktree with a redirected target dir.

## The one question
Does the census-and-direction machinery DISCRIMINATE, and does the census chain hold end to end —
from a `DocumentCensus` row through the classifier, `screen_inputs`, `apply`, `income answer` and
`interview_state()` — such that no filer is refused for a truthful answer and no income line can be
covered by a sentence that does not name it? Not a fresh audit of T1/T2; not a re-review of the spec.

## Settled (machine-verified by the controller; do not re-measure)
Suite at the build: 3241 passed / 12 skipped. `census-join`: 298 unmodeled entries across 13 maps,
every one placed and covered (Advisory 127 / QuestionId 124 / RefuseReason 47 before the fold).
`stop-list` clean. `line-coverage` 341 / 24 / 0 / 12 unmoved. `[[direction]]` headers on disk: 28
across 11 map files (the report said 27 — the fold appendix explains). `DocumentCensus`: 18
`Option<bool>` rows. The builder's report `2026-09-07-build-interview-T3-implementation.md` lists 15
deviations (D1–D15) and an "Open / not done" section; the appendix "Pre-review fold (D1, D11)"
records the two folds the controller ordered before you: D1 (the four 1099 rows carry the same
three rules as `w2`; fixtures truthful) and D11 (a derived flip — a line the same form's text
subtracts takes the opposite direction of its block; Schedule B line 3 now `Overstates` with an
Advisory cover).

## Seams (each earlier round could not see these; spend your budget here)
1. **The census chain.** For one supported row (`int_1099`) and one unsupported row (`k1`), walk
   the whole chain in code: the `FormQuestion` entry (prompt, `unanswered`, `live`, `get`, `set`,
   neutral answer), the classifier row, `screen_inputs`' refusal tier, `apply`'s `SetField` guard,
   `income answer`'s ask, `interview_state()`'s listing, the TOML wire (`#[serde(default)]`, a v3
   row missing `documents` entirely), and `income import`. Plant: a row whose `get` and `set`
   disagree; a `live` predicate that is never true; a row missing from the classifier — each must
   red somewhere you can name.
2. **Truthful answers never refuse.** Enumerate every committed fixture that carries `documents.*`
   values and check each against the rows it holds. Then the four 1099 rows after D1: `Some(true)`
   with rows passes; the TY2024 golden corpus (`crates/btctax-cli/tests/fixtures/…`) still files.
3. **The direction tables discriminate.** Plant (a) an `Advisory` cover on a Schedule 1 Part I entry
   → red, the same on a Part II entry → green; (b) delete one block caption → every entry in it
   reds; (c) move a numbered income line into a `NoDollar` block via a `part` key → red (D9b); (d)
   change a caption by one character → red; (e) after D11, remove the flip → Schedule B line 3's
   Advisory cover reds; a subtract sentence the extract does not print → red. Then read the
   `NoDollar` blocks and the `Overstates` covers the builder flagged as loosest (f1040 line 36; the
   "dead sum" lines Sch 2 1z / 7 / 18) and say whether each cover is honest.
4. **REACH.** For ten `QuestionId::OtherOutOfScopeIncome` covers chosen across Schedule 1 Part I and
   Form 1040, confirm the prompt names the line in words a filer reads (not a keyword buried in a
   list they can answer No to without reading). Plant: delete one named phrase → that entry reds.
   Judge the 4,641-character prompt as a filer would: is it answerable?
5. **`interview_state()` vs the registries.** N unanswered live items ⇒ N listed in one call;
   `Declined` in `forgoing` marked, never `blocking`; a `prompt_hash` mismatch listed with the
   changed-wording reason; `refusing` populated from the `FormQuestion`'s own refusal for an
   unsupported row's `Some(true)`; `waiting` exercised (D6's planted occupant); the extended no-brick
   test. Plant: a registry entry omitted from the walk → which test reds?
6. **What the build left out, and whether the hole is named.** D7 (`DEPENDENT_GATES × rows` not
   walked — T7's); D15 (24 declarations asked of a Single TY2024 filer, was 8): is every new ask
   mandated by R3, and is the ordering sane for a filer? Any `unmodeled` entry outside the seven
   forms that the interview reaches and the join does not cover.

## Severity
A truthful answer that refuses, an income line coverable by a sentence that does not name it, a
kill that does not red, or a registry/classifier omission that compiles are **Important** or
**Critical** (wrong result / a refusal that does not refuse / a false PASS). Secret-handling defects
are never Critical/Important. Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T3-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
