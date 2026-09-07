# Brief — fold the interview T6 seam review (C-1, I-1, M-1, M-3, M-4, N-1 … N-4; M-2's fixture)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once that RUNS the instrument; quote
the red. Synthetic identifiers only from the never-issued SSN space or `scripts/pii-scan-generic.sh`'s
`ALLOWED_EIN`.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T6-review.md` (read whole) and the
ledger `2026-09-07-build-interview-T6-review-VERIFICATION.md` — every claim is machine-verified; its
Disposition is the fold list. The build you are folding: `2026-09-07-build-interview-T6-implementation.md`
(commit `a79492ee`). Spec 1099-DA `design/SPEC_1099da_broker_reporting.md` R6 — the controller has
amended its premise sentence so that arm (2) runs the Digital Assets cross-check (read it as
amended). An entry is testimony: a printed box the filer never answered is btctax answering a
§6065 declaration for a human.

## The fold
1. **C-1 — the slice prints the ANSWER, never the ledger.** `Form1040Inputs.da_yes: bool` becomes
   `digital_asset_answer: Option<bool>` (`crates/btctax-forms/src/form1040.rs`), fed on the arm-(2)/(3)
   path from the working return the arm already resolved (`admin.rs:783`,
   `working.as_ref().and_then(|ri| ri.digital_asset_activity)`); `fill_form_1040_capgains` checks
   `da_yes` on `Some(true)`, `da_no` on `Some(false)`, and NEITHER on `None` — the slice's
   `hand_marks` / note then names the unanswered box (the fail-closed backstop fires on this path).
   The produce/skip decision for the 1040 page (today `!da_yes` ⇒ skip) gets its own predicate: rows
   or recognized income or removals in the year, from the ledger, as before. Arm (2) runs the DA
   cross-check before any byte: the same rule `screen_compute_dependent` applies — a contradicted
   `No` REFUSES the export naming the first qualifying event; an unwitnessed `Yes` prints with the
   off-ledger advisory (the slice's `advisories` carries it — `Vec::new()` goes). The full-return
   path is unchanged. Kills (read the box back off the emitted `form_1040_capgains.pdf` through
   `Form1040Map::ty2025()`, the review's own method — TY2025 templates with the live regime
   injected as `slice_from_answers.rs` does): stored `Some(false)` → `da_no` only; `Some(true)` →
   `da_yes` only; `None` → neither and the mark present; `Some(false)` against a ledger with a 2025
   disposal → the export refuses naming the event; an empty-ledger `Yes` prints Yes with the
   advisory. Plant the old `da_yes = !rows.is_empty()` back → the first kill reds.
2. **I-1 — the venue list obeys the regime.** `step0_panel` takes the year's
   `InformationReturnRegime` (callers: `admin.rs` via `year_readiness::regime_or_refuse`; the TUI's
   `TaxInputsFormState.broker_regime`); the `venues` list is built only when
   `btctax_core::forms::broker_question_is_live(&rows, regime)`; on a non-live year the panel states
   the regime (*"TY2024: no Form 1099-DA is issued for this year; nothing to answer"*) instead of a
   missing answer; the commit modal's heading no longer asserts *"the EXPORT is [blocked]"* on such a
   year. Kills: TY2024 and TY2025 fixtures with an exchange disposal and no answer → NO venue row;
   TY2026 → the row; plant the regime gate away → the 2024 fixture reds.
3. **M-1** — the standing-order row (and its §4.02(2) citation + consequence sentence) is emitted only
   inside the Notice's relief period (2025-01-01 → 2026-12-31, `Notice_2026-20.txt:299-300`); a
   pre-2025 year hands to the `Pre2025MethodNote` exit; after 2026 the row states that the relief
   period ended. Kill: TY2024 and TY2027 fixtures → no §4.02(2) row.
4. **M-2 — the same-day fixture, honestly labelled.** Add the half (b) case with the election made
   the day OF the sale, after it, and assert which way it falls today (silent, inheriting
   `resolve_election`'s `<=`); the doc comment says *"on or before"*, not *"before"*, and names
   FR-77. Do NOT change `resolve_election` — that decides filed basis and is FR-77's.
5. **M-3** — the granularity negative test also renders `RENDERED_PROMPTS` on a fixture and scans
   the rendered text; kill: plant *"how many accounts"* into a rendered prompt → red.
6. **M-4** — the `income answer` Step 0 print (which holds the ledger) surfaces the DA contradiction
   as a row (*"your answer No contradicts a 2025-06-15 Coinbase disposal — commit will be refused at
   export"*); the TUI commit gate is unchanged (a tier boundary — record the decision in the code at
   the commit site).
7. **N-1** — restore `disposal_compliance`'s doc comment to `disposal_compliance`; **N-2** — correct
   the report's row-1 date and present 3,363 as the workspace total; **N-3** — the two citations;
   **N-4** — the KAT comment (or make it test what it says).

## Constraints
- `record_answer` stays the only writer; the panel still writes nothing; `stop-list`, `census-join`
  and `line-coverage` unmoved; the R6 slice tests (`slice_from_answers.rs`) and the TY2024 golden
  corpus green.
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
APPEND a section `## Fold (seam review C-1, I-1, M-1 … M-4, N-1 … N-4)` to
`design/agent-reports/2026-09-07-build-interview-T6-implementation.md`: per item what changed, the
PDF read-back table for the slice's box, every kill with its red text, every pinned number moved,
suite lines per crate. Return only a 4-line summary.
