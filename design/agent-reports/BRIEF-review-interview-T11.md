# Brief — seam review of interview build T11 (the oracle path)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T11 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the Python stack is
`.venv/bin/python`; OpenTaxSolver per `scripts/oracle/README*` (if unavailable, say so and mark every
two-oracle claim UNVERIFIED — never passed on one oracle). **Environment, not findings:** six
`form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
fail in a PDF-less worktree with a redirected target dir.

## The one question
Is the projection a faithful, box-named inverse of the golden path — every `Usd` leaf either
projects or is listed as invisible, the dependents block round-trips, and the only excused
differences are those computed from a named mechanism at their exact size — and can the projection
ever carry identity? Not a fresh audit of T1–T10 or T16; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T11 build report `2026-09-07-build-interview-T11-implementation.md` lists the projection table,
`ORACLE_INVISIBLE`, the excuse mechanisms with sizes, OTS availability, kills and deviations; the
controller has confirmed the suite line it states. Spec: `design/SPEC_interview.md` r2 R13
(mechanism + kill), §7 row T11; `CLAUDE.md` "Two oracles" (excuses computed from mechanism, never
by vector name; a value the oracles take as INPUT is never validated by their agreement — §G-9);
`scripts/oracle/gen_goldens.py`, `ots_direct.py`, `sweep.py`, `check_return.py`; T16's HSA
addition to the harness; T1's `LEAF_SOURCE` / `leaf_walk`; T7/T8's verdict enum and DOBs.

## Seams
1. **The inverse.** `project_to_golden(build_golden_return(g).0) == g` over every golden household
   in `crates/btctax-core/tests/goldens/` and over the new two-dependent household; plant a
   projected leaf mapped to the wrong box (`e00300` ← dividends) → red; delete the dependents block
   from the projection → red.
2. **Completeness.** `ORACLE_INVISIBLE` is asserted complete by walking `LEAF_SOURCE` /
   `leaf_walk`, not a hand list: plant a new `Usd` leaf with neither a projection nor a listing →
   red; every `Option<bool>` and the census are listed (not projected); the HSA deduction (T16)
   projects to `e03290` and the OTS line.
3. **The excuses.** On a one-CTC-child fixture `oracle_line19 > 0` and the excuse equals it
   exactly; an off-by-one plant fails; EIC (line 27) and any other forgone credit excused by
   mechanism with size; grep `check_return.py` and the KATs for any vector-name key → a finding.
4. **Identity.** `GoldenInputs` carries no name / SSN / address by TYPE; `income project` output on
   a fixture with identity contains none of it (grep the output for the fixture's strings); the
   projection carries no `ssn_hash`.
5. **The oracles.** `check_return.py` on the TY2024 example fixture runs the harness and BOTH
   oracles and diffs the compared lines; the existing sweep still reconciles; a disagreement, if
   any, is recorded against the FORM in the report, not encoded.
6. **The gates the oracles take as input.** The §G-9 limit stated in the docs: gates (declarations,
   the census, the DA answer) are inputs the oracles take as given and are validated by R2's
   transcription checks instead — confirm the report and `ORACLE_INVISIBLE`'s doc say so, and that
   no test claims oracle agreement validates a gate.

## Severity
A projected leaf reaching the wrong box, an invisible leaf unlisted, an excuse keyed by name, a
figure "validated" on one oracle, or identity in the projection is **Critical** or **Important**;
a kill that does not red is **Important**. Design opinions outside the one question are Minor at
most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T11-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
