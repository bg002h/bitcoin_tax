# Brief — seam review of interview build T5 (the 1099-INT / DIV / B / G and 1098-E sections)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T5 build commit, or its continuation). Read-only for the record: every plant is made
in YOUR worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- box-census` / `line-coverage` / `census-join`. **Environment, not findings:** the
archived PDFs are gitignored, so six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Does every box a filer can hold on these five documents reach exactly the line the form says, or
refuse, or carry a recorded reason — with nothing hand-listed between the archived extract and the
printed return — and does the document-less income door (R3's paired questions) open and close
exactly on the census row's `Some(false)`? Not a fresh audit of T1–T4b; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T5 build report `2026-09-07-build-interview-T5-implementation.md` (and any continuation
section) lists its census decision tables, kills and deviations; the controller has confirmed the
suite line it states, `box-census` OK, `line-coverage` and `census-join` lines. Spec:
`design/SPEC_interview.md` r2 R4 (the reach table is a READING LIST, the census is the authority),
R5, R3 (the paired questions), §5.1, §5.2, §5.7, §7 row T5; `FOLLOWUPS.md` FR-65 (1099-G box 10,
W-2 box 14b), FR-66 (the join's scope, not T5's), FR-68. The prior builds: T2's per-edition
`BoxEntry`, T3's `DocumentCensus` with `declared_rows` / `requires_transcription`, T1's
`LEAF_SOURCE` / `record_answer`, T4's draft rules, T4b's `opened_from`.

## Seams
1. **The census ↔ registry join.** For every archived edition of the five documents (and the W-2),
   every `Collected(FieldId)` names a `FieldId` that exists, sits in that document's section, and
   whose `help` is the caption verbatim; every `RefuseIfNonzero` names a `RefuseReason` that fires;
   every `NotRead` carries the instruction's pointer. Plant: a `Collected` naming a field in another
   section → red? A caption changed by one character in the extract → red? A box whose entry is
   deleted → red? Which edition governs a TY2024 row versus a TY2026 row (T2's
   `revision_in_force`), and does a 2026-only box (FR-65) appear only there?
2. **Reach.** Pick five boxes across the documents (1099-INT box 10, 1099-DIV box 2a, 1099-G box 2,
   1099-B basis-not-reported, 1098-E box 1) and trace each from the `Field` to the printed line
   through `printed.rs` / the 1040 chains; a fixture with the box set to $100 moves exactly the line
   R4 names by $100 and nothing else. Then the refusals: W-2 box 13 checked → `StatutoryEmployeeW2`
   naming Schedule C line 1; 1099-INT box 11 > 0 → `AmortizableBondPremiumNotComputed`; the FR-65
   decisions as the instructions say.
3. **The document-less income door.** Each paired question is live iff its census row is
   `Some(false)`; `w2 = Some(true)` never asks it; `Yes` on the wage and refund questions refuse
   naming their exits; `Yes` on the interest question with no Schedule B record refuses while one
   record passes and reaches Schedule B lines 1/5; `itemized_prior_year` live on a return with no
   1099-G row when the refund question is `Some(true)`, and on one with a box-2 row; classifier
   rows for every new leaf (no `..`, no `_`). Plant: a paired question left out of the classifier →
   compile error or test?
4. **Warnings never write.** The W-2 arithmetic and the all-zero-row warnings: fire on their
   fixtures, the row byte-identical after; rendered where the row is edited and in `report`; a
   warning is never a refusal.
5. **The replacement chain.** `sch1.student_loan_interest_paid` is gone; Schedule 1 line 21 reads
   the 1098-E rows' sum (with the phase-out unchanged); the TY2024 example fixtures and goldens moved
   only where the scalar became a row (each moved line explained); `EXEMPT_PREFIXES` shrank by the
   five names and the ratchet holds; the five `NotInForm` anchors became `Field`/`Section` anchors
   and the count fell by exactly five; `IraDeductionClaimed` untouched.
6. **TOML and provenance.** `income import` round-trips every new table, refuses unknown keys naming
   them, and screens the new rows at import (T4's tier); `LEAF_SOURCE` covers every new leaf with
   the right source; a row with `transcribed_on = None` prints *transcribed without a date* in the
   manifest; `record_answer` stays the only writer of `answer_log`.

## Severity
A box that reaches the wrong line, a box with no entry, a paired question that asks on the wrong
census state or not at all, a warning that writes, or a kill that does not red is **Important** or
**Critical** (a wrong printed figure is Critical). Secret-handling defects are never
Critical/Important. Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T5-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
