# Brief — seam review of interview build T4 (the year gate and draft protection)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T4 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`). **Environment, not findings:** the
archived PDFs are gitignored, so six `form_delta` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
PDF-less worktree with a redirected target dir.

## The one question
Can a filer's work be lost or poisoned across the year gate? Concretely: is there any path by which
(a) a committed row reaches precedence 1 unscreened on a params-less year, (b) a non-trivial draft is
deleted without the filer's confirmation, (c) an answer written into a draft-only year leaks into
the committed row or vanishes on the next `load`, or (d) a rule exists in `screen_inputs` that the
import-time tier omits (or vice versa)? Not a fresh audit of T1–T3; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T4 build report `2026-09-07-build-interview-T4-implementation.md` lists its writer enumeration,
kills and deviations; the controller has confirmed the suite line it states and that `line-coverage`
and `census-join` are unmoved. Spec: `design/SPEC_interview.md` r2 R11 (the mechanism and its seven
kills), §7 row T4, R12 (T3's `interview_state()` is the interview-complete predicate), R3 (the
census invariants are the param-free screens); `design/SPEC_input_form.md` §6.2/§6.3; the writer
sites named in `BRIEF-build-interview-T4.md` (`return_inputs::set` from `input_form_store::commit`,
`income import`, `income answer`, and the year+1 write at `cmd/tax.rs:1024`).

## Seams
1. **The shared param-free tier.** Diff the list `income import` runs against what `screen_inputs`
   runs without a `TaxTable`/`FullReturnParams`. Enumerate both from the code, not the report. A rule
   present in one and absent from the other is a finding. Plant: remove one rule from the shared
   list → which test reds? Add a param-free rule to `screen_inputs` outside the list → does anything
   notice?
2. **Import writes nothing on refusal.** On a params-less year, a TOML with `documents.k1 = true`;
   with `documents.w2 = false` beside a `[[w2s]]` row; with a `NegativeAmount`. After each refusal:
   `return_inputs::get` is `None`, no draft was created or altered, and the note/exit is the row's
   own §2.2 sentence. Then the same TOML corrected imports. Then a params-BEARING year: the
   param-dependent rules still run at commit, not at import (plant a §402(g) excess and confirm import
   accepts and commit refuses).
3. **Draft protection versus coherence.** Every path that deletes a draft: `load`'s stale-WIP
   discard, `coherence_clear_or_refuse`, `delete_draft` callers, the TUI. For each: a draft with one
   `answer_log` entry, a draft with one document row, a draft with a dependent, a `default()` draft,
   an answer-free non-default draft. Which survive without `--force`, which are deleted with a note,
   and does the note name what was lost (counts)? Plant: a new deleting path that skips the guard
   → does a test red? The TUI's payload-confirm: is the confirm actually required before the delete,
   or is it advisory?
4. **`income answer` into a draft-only year.** Trace the read → `record_answer` → `save_draft` path:
   after answering, `return_inputs::get` is `None`, `resolve.rs`'s ladder never sees the draft, the
   draft holds the `AnswerRecord` with the prompt hash and date, `Session::open` reloads it, a
   subsequent `commit` on a params-bearing year carries the answers into the committed row, and a
   second `income answer` on the same year updates the same key (no duplicate). Both-absent still
   refuses with `answer.rs:121`'s reason. Plant: make `income answer` write via `return_inputs::set`
   on the draft path → red?
5. **The two entry states.** `interview_complete` from T3's `interview_state()` and `YearReadiness
   .params`: on a TY2026 fixture with every gate answered, complete + not computable, with R11's
   sentence in the snapshot; on TY2024 the sentence is absent. Where else is "authorable" decided
   (`year_readiness::slice_prints_from_answers`, the TUI year picker, `income import`'s year check)
   and do they agree?
6. **The year+1 write and the slice path.** What `cmd/tax.rs:1024` is, whether T4's rules bind it,
   and whether the TY2026 slice-filing path (spec 1099-DA R6) still commits via `income import` and
   the TUI `commit` once params exist — its tests green.

## Severity
Data loss (a draft deleted without confirmation), poison (an unscreened committed row on a
params-less year), an answer that leaks or vanishes, and a rule split between the two tiers are
**Critical** or **Important**. A kill that does not red is **Important**. Secret-handling defects
are never Critical/Important. Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T4-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
