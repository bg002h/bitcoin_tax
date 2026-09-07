# Brief — interview build T4: the year gate and draft protection (R11)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create (revert a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E
'<filter>'` (never `cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and
a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D
warnings` before finishing. Every guarantee lands with a kill seen red once; the report quotes the
red. Every pinned number moved: old → new with cause. `/tmp` is a 32 GB tmpfs — build only in the
repo's target dirs. Process in force (owner S6): ONE seam review and ONE re-verification after you.

## The contract
`design/SPEC_interview.md` r2 **R11** (the year gate: interview-complete vs return-computable at
entry; a long-lived draft is protected; `income answer` into a draft-only year; `income import`
runs the param-free screens before it writes), **§7 row T4** (its seven kills), **R12** (T3's
`interview_state()` is the interview-complete predicate), **R3** (the census invariants are the
param-free screens), `design/SPEC_input_form.md` §6.2/§6.3 (draft coherence, the stale-WIP split)
and §3.2 of `SPEC_input_surface.md` (the poison of an unscreened row at precedence 1). Build AS
WRITTEN; the tree's real names win; deviations recorded. What T1–T3 built is the floor.

## Every WRITER of the return, enumerated (controller-measured at `fc10033a`; re-grep and list
## them in your report — a writer this list misses is a finding, not a surprise)
Committed row (`return_inputs::set`, `crates/btctax-cli/src/return_inputs.rs:108`): (1)
`input_form_store::commit` (`input_form_store.rs:362`; I-11 `NoTables` early return at `:370`, the
write at `:381`); (2) `income import` → `cmd/tax.rs:256` (after `coherence_clear_or_refuse` at
`:157` and the prior-row carry at `:163`); (3) `income answer` → `cmd/answer.rs:418` (reads only a
committed row at `:232`, so a draft-only year is refused today); (4) `cmd/tax.rs:1024` writes year
N+1 — identify what it is and whether it is inside T4's rules. Draft (`save_draft`,
`input_form_store.rs:140`; `delete_draft` `:86`): the TUI autosave; `load`'s stale-WIP discard
(`:194-206`); `coherence_clear_or_refuse` (`:298-310`, notes and deletes a non-default WIP draft on
any committed-row write). `resolve.rs:72-84` is the precedence ladder that makes an unscreened
committed row poison; the draft is invisible to it (`input_form_store.rs:1-2`).

## What T4 delivers
1. **Two states at entry.** `interview_complete(ri) = interview_state(ri).blocking.is_empty()`
   (T3's function) and `return_computable = YearReadiness.params`
   (`year_readiness.rs:19-27`, `sentence()` at `:115`). The TUI entry screen and `income answer`'s
   panel header state both, in R11's words for a params-less year: *"authoring and saving work;
   computing and committing wait for the TY2026 package (expected Jan 2027)"*. Kill: a TY2026
   fixture with every gate answered reports complete while `params` is false; the sentence appears
   in a snapshot.
2. **Confirm-before-discard of a non-trivial WIP draft.** A WIP draft with any `answer_log` entry or
   any document row (a `w2s`/`int_1099`/`div_1099`/`b_1099`/`g_1099` row, a dependent, a Schedule
   A) is **never** deleted on a note: `coherence_clear_or_refuse` and `load`'s stale-WIP path refuse
   with a message naming what the draft holds and the remedy (`--force` on the CLI; a
   payload-confirm in the TUI — the `DeleteSection` I-10 precedent). `ReturnInputs::default()`-shaped
   and answer-free drafts keep today's behaviour. Kill: `income import` over a draft with one
   answered census row and no `--force` refuses and the draft survives byte-identical; with
   `--force` it is deleted and the note names what was lost (the count of answers and rows).
3. **`income answer` into a draft-only year.** When the year has a draft and no committed row,
   `income answer` reads the draft, records answers through `record_answer` (T1's one writer) and
   writes the draft back via `save_draft`, never `return_inputs::set`; `return_inputs::get` still
   returns `None` afterwards. It still refuses when neither exists (`answer.rs:121`'s reason
   stands). Kill: the round trip; `get` is `None`; the committed-row path is unchanged.
4. **`income import` runs the param-free screens BEFORE it writes.** R3's three census invariants,
   the unsupported-row refusals (`DocumentTypeUnsupported`), `NegativeAmount`, and every other
   `screen_inputs` rule that needs neither a `TaxTable` nor `FullReturnParams` run at import; a
   refusal writes nothing. Factor the param-free tier so it is ONE list both `screen_inputs` and
   import call (a rule that exists in one and not the other is the defect). The param-dependent
   rules (§402(g), SALT, excess SS, the §152(d) figure) still wait for commit. Kill: on a
   params-less year (TY2026 today) a TOML with `documents.k1 = true` refuses at import naming the
   exit and leaves no committed row; the same TOML with every census row supported imports; a
   rule removed from the shared list reds a KAT that enumerates it.
5. **`commit` → `NoTables` writes nothing** (exists; re-assert with a kill that the write at `:381`
   is unreachable on a params-less year — plant its removal).
6. The TY2026 slice-filing path (spec 1099-DA R6) is unchanged; its tests stay green.

## Constraints
- `record_answer` remains the only writer of `answer_log`; T4 adds no second one.
- Every new prompt or sentence is the form's words or R11's; every refusal names its exit.
- Goldens under `docs/examples-tui-walkthrough/` and `docs/examples/` may move only where the entry
  screen changed; regenerate via the Makefile targets and list each moved line's cause.
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T4-implementation.md`: per numbered item what
landed (files, functions), the writer enumeration you measured, every deviation with its reason,
every kill with its red text, every pinned number moved, suite lines per crate. Return only a
4-line summary plus the path.
