# Brief — seam review of interview build T7 (the dependents gates)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T7 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- line-coverage` / `census-join` / `stop-list`. **Environment, not findings:** the archived
PDFs are gitignored, so six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Can a dependent be claimed on a chain that was never completed, or a gate be answered by anything
but the filer this year? Concretely: (a) every flowchart edge in R6's table yields the instruction's
outcome and nothing else; (b) a live gate left `None` blocks and is listed; (c) no Step 1 predicate
reads a declinable value (the taxpayer's DOB skippable); (d) the age test is computed from the
row's REQUIRED `date_of_birth`, and a row without one is refused before any print path; (e) the
per-row liveness emulation (I-4) hides a non-live gate without ever dropping a set silently; (f)
`gross_income_under_limit` WAITS on a params-less year and blocks with params; (g) the opener's
seeded dependent (FR-70) is blocking through the gates and its records are keyed by identity. Not a
fresh audit of T1–T6; not a spec re-review; T8 (row (7), the grid emitter, HoH/QSS) is not yours.

## Settled (machine-verified by the controller; do not re-measure)
The T7 build report `2026-09-07-build-interview-T7-implementation.md` lists its gate table, the
truth-table KAT's rows, the FR-70 identity-vs-declaration decision, kills and deviations; the
controller has confirmed the suite line it states and that `line-coverage` / `census-join` /
`stop-list` are as stated. Spec: `design/SPEC_interview.md` r2 R6 (the gates half and its kill),
§7 row T7, R10 parts 1–3 (`record_answer`, `AnswerKey::DependentGate { ssn_hash, gate }`,
`prompt_hash`), R12; `i1040gi--2025.txt` (the instruction's steps and edges — cite by line). Prior
builds: T1's key and one writer; T3's classifier discipline and `interview_state()`; T4's tiers;
T4b's opener; T5's grouped `live_questions`.

## Seams
1. **The truth table, re-driven.** Build fixtures at every edge of R6's table yourself and run
   `screen_inputs` + the verdict enum: Step 1 relationship No → Step 4; support-over-half Yes → Step
   4; joint-return-not-refund-only → Step 4; the CAUTION (qualifying child of another) Yes → refuse
   naming *Qualifying child of more than one person*; Step 2 citizen No → refuse; married Yes →
   refuse naming *Married person*; `DependentTaxpayer = Some(true)` → refuse *"You can't claim any
   dependents"*; Step 3 TIN-by-due-date No → no credit box (a forgo, not a refusal); under-17 from
   DOB; the SSN-valid-for-employment split (CTC vs ODC verdict); Step 4 relationship/member No →
   refuse; qualifying-child-of-any-taxpayer Yes → refuse; `gross_income_under_limit` No → refuse;
   support No → refuse; the divorced/multiple-support/kidnapped rule Yes → refuse naming the three
   rules; the born-in-year child with the Exception → the CTC edge. Any cell the KAT does not cover
   is a finding.
2. **Nothing declinable feeds Step 1.** Grep every gate's `live` and every computation for
   `DobTaxpayer` / `DobSpouse` / any `SkippableQuestion` value; the age test reads only the row's
   DOB and `tax_year`; `younger_than_you_or_spouse` is asked, never computed. Plant: compute it from
   the taxpayer DOB → which test reds?
3. **The required DOB.** `date_of_birth = None` on a row ⇒ `DependentGateUnanswered { gate:
   DateOfBirth }` — and the row reaches NO print path (`dependents_statement`, the emitter, `report`)
   while unanswered; plant a print that ignores the refusal → red?
4. **Per-row liveness and the answer key.** A Step 4 gate on a row still at Step 1 is hidden (`get`
   → `None`), a `set` on it refuses `NoSuchRow` (never silently dropped); two rows' gates answered,
   row 0 deleted ⇒ row 1's records intact; row 1's SSN changed ⇒ its records move to history; the
   `prompt_hash` re-ask on a changed gate prompt; `income answer` asks each live gate once per row
   (T5's sweep) keyed by identity. Plant: key a gate by row index → red?
5. **Waiting versus blocking.** On the TY2026 fixture (no `FullReturnParams`) a row at Step 4 lists
   `gross_income_under_limit` in `waiting`, never `blocking`, and commit is still refused by the year
   gate (T4) not by this gate; insert params ⇒ blocking with the figure in the prompt; the figure
   and its cite (`i1040gi--2025.txt:1688-1693`, the Rev. Proc.) match for TY2024 and TY2025.
6. **The opener (FR-70).** A prior dependent is seeded with identity fields only, every gate
   `None`, no `AnswerRecord`; it is in `interview_state().blocking` through the gates; the T4b test
   that pinned "not seeded" was rewritten, not deleted; `filer_tin_issued_by_due_date` becomes live
   with the seeded row and is `None`.

## Severity
A dependent printable with an incomplete chain, a gate fed by a declinable value, a set silently
dropped, a refusal naming the wrong rule, or a kill that does not red is **Critical** or
**Important**. Secret-handling defects are never Critical/Important. Design opinions outside the
one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T7-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
