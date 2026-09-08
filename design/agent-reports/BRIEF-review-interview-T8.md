# Brief — seam review of interview build T8 (row (7), HoH / QSS, the TY2025 dependents grid)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T8 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- line-coverage` / `census-join` / `stop-list`. The archived PDFs are gitignored — the
TY2025 f1040 template is bundled under `crates/btctax-forms/forms/2025/` (check) so the emitter
kills can run; if a PDF is absent say so rather than marking a claim passed. **Environment, not
findings:** six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_
once_per_new_directory` fail in a PDF-less worktree with a redirected target dir.

## The one question
Can a credit box, a filing status or a grid cell be printed on Form 1040 page 1 that the filer's
answers do not establish? Concretely: (a) row (7)'s CTC/ODC box follows T7's verdict and the DOB
exactly — never asked, never defaulted; (b) rows (5)/(6) print from the gates; (c) the TY2025 grid
cells are mapped through the label reader and censused, with the `UNCENSUSED` register falling by
exactly the cells mapped; (d) TY2024's emitter output is byte-identical; (e) HoH and QSS ask the
instruction's tests and refuse on any `None` or `No` naming the exit; (f) the FR-67 nonresident-alien
election gate refuses on `Yes`; (g) the line-19 forgo is sized from params and absent without. Not
a fresh audit of T1–T7 or T16; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T8 build report `2026-09-07-build-interview-T8-implementation.md` lists the mapped cells, the
register count old → new, the HoH/QSS question table, kills and deviations; the controller has
confirmed the suite line it states and that `line-coverage` / `census-join` / `stop-list` are as
stated. Spec: `design/SPEC_interview.md` r2 R6 (the computed half), R7, §5.4, §7 row T8;
`FOLLOWUPS.md` FR-67; `i1040gi--2025.txt` (HoH `:1149-1200`, *Married persons who live apart*
`:1247-1268`, QSS `:1288-1316`; row (7) and the credit edges from R6's table);
`design/forms/extract/f1040--2025.txt:38-52` (the grid). Prior builds: T7's verdict enum and
`DEPENDENT_GATES`; T4b's `FilingStatusConfirmed`; T3's classifier discipline and `interview_state`;
`field_census.rs`'s `UNCENSUSED` register.

## Seams
1. **Row (7) and rows (5)/(6), re-driven.** Fixtures: a CTC child; an ODC dependent (a qualifying
   relative; a child over 16; a child without an employment-valid SSN); a row whose chain refuses;
   the born-in-year child (CTC). Read the boxes back off the filled TY2025 PDF; plant a row (7)
   computed from a field the filer typed → red? A `Dependent` with `date_of_birth = None` never
   reaches the emitter.
2. **The map and the register.** Every mapped cell's AcroForm name resolves in the TY2025 template
   (the label reader's output, not a typed list); each has a `[census]` entry; `UNCENSUSED`'s
   `f1040` count fell by exactly the number of cells mapped (plant: map one more cell without a
   census entry → red; leave the count → red); the *more than four dependents* box and the
   statement path — refusal or attachment, as the report says, verified against the instructions.
3. **TY2024 byte-identical.** The existing TY2024 golden and emitter snapshot unchanged; a TY2024
   fixture with dependents prints exactly what it printed before T8.
4. **HoH.** `hoh_marital_basis` is an enum of the instruction's four states (never a compound
   yes/no); `Single`/`Mfj` ask nothing; `Hoh` with basis `None` or a test `None` refuses;
   `MarriedLivedApart` refuses naming *Married persons who live apart* (or asks the five transcribed
   conditions — then each `None` blocks and each `No` refuses); `NraSpouseNoElection` refuses naming
   *Nonresident aliens*; a `No` on either test refuses `HohTestNotMet` with *"choose another filing
   status"*; the qualifying-child name field is live only when no dependent row is the qualifying
   person; the EIC separated-spouse checkbox is NOT asked here.
5. **QSS.** Five questions live iff `Qss`; the two-year window derived from `tax_year` (a TY2026
   fixture's prompt names 2024 and 2025; a TY2024 fixture's names 2022 and 2023); any `None`
   refuses `QssTestUnanswered`, any `No` refuses `QssTestNotMet` naming the test; all five `Some(true)`
   computes at the joint rates (assert the tax, not the flag).
6. **FR-67 and the forgo.** The §6013(g)/(h) gate refuses on `Yes` naming the election and a
   preparer, and is live on the statuses where it applies (which ones? read `:1149-1163`); the
   line-19 forgo row in `interview_state_with_params` sizes from the year's CTC/ODC amounts and is
   absent on a params-less year; the no-brick test covers every new question.

## Severity
A credit box or filing status printed that the answers do not establish, a mapped cell with no
census entry, a TY2024 change, or a kill that does not red is **Critical** or **Important**.
Secret-handling defects are never Critical/Important. Design opinions outside the one question are
Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T8-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
