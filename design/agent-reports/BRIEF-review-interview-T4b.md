# Brief — seam review of interview build T4b (the year-N+1 opener)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T4b build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`). **Environment, not findings:** the
archived PDFs are gitignored, so six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Can the opener carry anything from year N into year N+1 that the filer did not freshly assert this
year — an answer record, a non-zero amount that is not a computed carryforward, a gate answered, a
`Durable` fact recorded without a keystroke — or seed a carryforward from the wrong chain (year N's
INPUTS rather than year N's frozen RETURN)? And does its write obey T4's draft rules? Not a fresh
audit of T1–T4; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T4b build report `2026-09-07-build-interview-T4b-implementation.md` lists its identity model,
kills and deviations; the controller has confirmed the suite line it states and that `census-join`
and `line-coverage` are unmoved. Spec: `design/SPEC_interview.md` r2 R10 part 4 and its kill, §7 row
T4b, §6 J-27; T4's rules (`input_form_store::draft_is_disposable`, `--discard-draft`, `save_draft`);
T1's `LEAF_SOURCE` walk (`crates/btctax-core/src/tax/provenance.rs`) and `record_answer`;
`Durability` (`questions.rs:28-35`); the frozen carryforward-out chain (`return_1040.rs:1600-1632`)
and `report --write-carryover` (`cmd/tax.rs`, the year-N+1 writer).

## Seams
1. **The seed's leaves.** Walk every leaf of the seeded draft with `LEAF_SOURCE` (not the report's
   list): every `Usd` is default except the carryforwards; every `Option<bool>` gate is `None`;
   `documents.*` all `None`; `answer_log` and `answer_log_history` empty. Plant: a builder-style
   shortcut that copies a `PerYear` gate or a money box from year N → which test reds?
2. **The carryforward chain.** The seed reads year N's frozen RETURN's `_out` values through the same
   code `report --write-carryover` uses. Plant: a year-N committed row whose `_in` differs from the
   return's `_out` — the seed takes the `_out`; each seeded carryforward carries
   `ComputedFromPriorReturn { year: N }` and a `_provenance` sibling; the QBI pair handled the same
   way or its absence named. Does `report --write-carryover` still behave identically after any
   sharing/refactor (its tests green; a diff of its output on a fixture before/after)?
3. **Identities and `Durable` facts.** Each payer (EIN/TIN + name), dependent (name, SSN,
   relationship, DOB) and venue appears as a pre-named row and a tri-state prompt; the DOB is
   DISPLAYED but has no `AnswerRecord` until a fresh `SetField` writes one dated `BTCTAX_NOW` with
   the current prompt hash; a `No` on an identity removes its pre-named row; `None` blocks. Which
   `FormQuestion` / census row carries the identity answer, is it in the classifier (no `..`), and
   can `interview_state()` see it? Plant: a seeded DOB with a copied `AnswerRecord` → red?
4. **T4's rules on the opener's write.** Year N+1 with a committed row refuses; a non-trivial draft
   refuses without `--discard-draft` (and the structural predicate, not a category list, decides);
   year N without a committed row refuses; `return_inputs::get(N+1)` is `None` after opening;
   `resolve.rs` never sees the seed; a second `open-next-year` on the same year refuses or is
   idempotent — which, and is it documented?
5. **The TUI entry action.** Present only when year N has a committed row and N+1 has neither row
   nor draft; the payload-confirm on a non-trivial draft is required, not advisory; snapshot.
6. **Retention.** Year N is byte-identical after opening (committed row, draft table, answer log).

## Severity
A carried answer or amount, a seed from the wrong chain, or a draft destroyed unconfirmed is
**Critical**; a kill that does not red, a registry/classifier omission, or an identity the seed
silently drops is **Important**. Secret-handling defects are never Critical/Important. Design
opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T4b-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
