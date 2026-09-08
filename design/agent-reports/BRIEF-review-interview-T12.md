# Brief — seam review of interview build T12 (the panel in the TUI, the modal, the manifest, the docs)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T12 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); `make docs` for the generated
pages; the instruments `cargo run -q -p xtask -- stop-list` / `census-join` / `line-coverage`.
**Environment, not findings:** six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Does every surface RENDER `interview_state()` faithfully — never recomputing it, never flattening
class (B) into class (A), never dropping a `Declined` item, never hiding a refusing row before
commit, never omitting an undated document row — and does no surface write anything? Not a fresh
audit of T1–T11 or T16; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T12 build report `2026-09-07-build-interview-T12-implementation.md` lists the pane and modal
snapshots, the manifest block, the docs regenerated, kills and deviations; the controller has
confirmed the suite line it states, `make docs` clean and byte-stable, and the instruments' lines.
Spec: `design/SPEC_interview.md` r2 R12 (the seven states), §4.1, §4.4, §7 row T12, R15;
`FOLLOWUPS.md` FR-73 (the granularity note into `LIMITATIONS.md`); T3's `interview_state()` /
`interview_state_with_params()`; T6's Step 0 panel and `contradictions`; T7/T8's gates and the
line-19 forgo size; T9's home-sale decision; T5's *transcribed without a date* row; T4's entry
gate; the `binary-docs-infra` convention (man pages generated from clap doc-comments).

## Seams
1. **Derived, not recomputed.** Trace the pane, the modal, `report` and the manifest to
   `interview_state()`; plant a divergence (the pane filtering one state locally) → which snapshot
   reds? Grep the render code for any liveness predicate or registry walk of its own → a finding.
2. **The seven states rendered.** A fixture with N unanswered live items across the registries and
   the document invariant renders N blocking rows in the pane AND the modal; a `Declined` skippable
   renders *(declined)* in forgoing with its size, never in blocking, and `Given` removes it; a
   refusing row (an unsupported census `Some(true)`, a `BasisDiffers`, a `Yes` on a refusing gate)
   renders its exit sentence BEFORE commit; a hash-mismatched record renders the changed-wording
   reason; `waiting` renders the package it waits on; the forgo size present with params and
   absent without (TY2026).
3. **The modal and the gate.** The commit modal prints forgoing AND refusing; a commit with a
   non-empty `refusing` is refused by the existing gate (assert, and plant the gate away → red);
   the modal never mentions a gate that does not exist on the year (T6's I-1 lesson).
4. **`report` and the manifest.** `report` prints the census (type → declared / rows), the panel,
   the home-sale decision and its answers, row (7) per dependent, and every collected figure's
   `LEAF_SOURCE` provenance with the payer TIN MASKED (plant an unmasked TIN → red); the manifest's
   "COMPLETE BY HAND" block carries the forgoing list with `(declined)` marks and names undated
   document rows; a fixture packet shows it.
5. **Docs.** `make docs` regenerates every `income` subcommand page the interview added and the
   TUI walkthrough; a second run is byte-identical; `LIMITATIONS.md` carries §2.2's exits in the
   filer's words, line 19 as a forgo, the home sale, the 1099-DA answers as keystrokes, no
   self-custody import, the venue granularity note (FR-73); the R15 greps still clean; no
   progress bar or "remaining" anywhere (plant the word → `stop-list` red).
6. **Nothing writes.** Every new render path takes `&ReturnInputs` / `&InterviewState`; no
   `save_draft`, `return_inputs::set` or `record_answer` in render code (grep); the walkthrough
   goldens moved only where a screen changed (each explained — check three).

## Severity
A state dropped or flattened, a refusing row hidden before commit, a render that writes, an
unmasked TIN, or a kill that does not red is **Critical** or **Important**. Secret-handling
defects are logged, never Critical/Important. Design opinions outside the one question are Minor
at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T12-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
