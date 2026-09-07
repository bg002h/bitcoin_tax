# Brief — fold the interview T4 seam review (C-1, M-1, M-2, M-3, N-1)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create (revert a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E
'<filter>'` (never `cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and
a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D
warnings` before finishing. Every guarantee lands with a kill seen red once; quote the red.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T4-review.md` (read whole) and the
ledger `2026-09-07-build-interview-T4-review-VERIFICATION.md` — every claim is machine-verified; the
Disposition fixes the shape. Your own T4 report `2026-09-07-build-interview-T4-implementation.md` is
what exists.

## The fold
1. **C-1 — the disposable predicate is structural.** In `input_form_store.rs`:
   `pub fn draft_is_disposable(ri: &ReturnInputs) -> bool` = the draft equals the year's fresh seed,
   `ReturnInputs { tax_year: ri.tax_year, filing_status: ri.filing_status, ..Default::default() }`
   (the filing status alone is not work — the existing
   `a_disposable_draft_is_still_superseded_without_a_flag` fixture stays disposable). No category list
   anywhere in the DECISION; `coherence_check` and `load`'s stale-WIP path key on this predicate.
   `DraftHoldings::describe()` remains for the MESSAGE, with a fallback clause (*"and work not
   otherwise itemised"*) whenever the draft is non-disposable but every counted category is zero, so
   the filer is never told "nothing" about a draft that holds something. Kills (each red first): the
   reviewer's three plants — a TY2026 draft holding two providers' `CohortAnswers` and a Schedule C,
   saved with `save_draft`, then (a) `import_return_inputs(.., discard_draft=false)`, (b) `income
   clear` without the flag, (c) `load` on a stale schema version — each REFUSES and the draft
   survives byte-identical; `report --write-carryover` on year N shares (a)'s predicate (assert it);
   and a structural kill: a draft differing from the seed in ONE uncounted field (pick
   `sch1.state_refund_taxable` or `schedule_c`) is non-disposable — assert without naming the
   category list.
2. **M-1** — the discard-only screen's heading, block title and prompt derive from which error
   opened it (`StaleParkedDraft` → the parked wording; `StaleDraftHoldsInterview` → *"draft this build
   cannot open"* wording, never "parked"); snapshot kill.
3. **M-2** — `SalesTaxElectionWithoutAmount`'s detail drops the `income answer` clause (the first exit,
   the TOML, stands); check the whole param-free tier once more for any other `income answer` exit
   (the reviewer found one) and assert by a grep-KAT over the tier's details that none names it.
4. **M-3** — the read-only projections `working_return` and `broker_answers` map
   `StaleDraftHoldsInterview` to `(None, Some(note))` the way they handle `StaleNote`; the write
   paths keep the hard refusal. Kill: `income show-broker-answers` / `report` on a year with a
   stale interview draft renders with the note; `income import` on it still refuses.
5. **N-1** — the two `#[error]` literals get their `\` continuations (no run of spaces in the
   rendered text); assert the rendered messages contain no `"  "`.

## Constraints
- No new writer of the return or the draft; `record_answer` stays the only writer of `answer_log`.
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
APPEND a section `## Fold (seam review C-1, M-1, M-2, M-3, N-1)` to
`design/agent-reports/2026-09-07-build-interview-T4-implementation.md`: per item what changed,
every kill with its red text, every pinned number moved, suite lines per crate. Return only a
4-line summary.
