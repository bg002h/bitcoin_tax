# Brief — fold the interview T7 seam review (C-1, I-1 … I-5, M-1 … M-5, N-1 … N-3)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all`, a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
and `make docs` before finishing. Every guarantee lands with a kill seen red once that RUNS the
instrument (never an emulation of it); quote the red. Synthetic identifiers only from the
never-issued SSN space (area 000/666, group 00, serial 0000) or `scripts/pii-scan-generic.sh`'s
`ALLOWED_EIN`.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T7-review.md` (read whole) and the
ledger `2026-09-07-build-interview-T7-review-VERIFICATION.md` — every claim is machine-verified; its
Disposition is the fold list and decides I-2. The build you are folding:
`2026-09-07-build-interview-T7-implementation.md` (commit `282a8a32`). The rules this fold re-learns:
a prompt hash is keyed on the registry's WORDS, never on display chrome (T5's D-1, `answer_status`'s
own doc); a KAT drives the real command, never an emulation (T5's I-1); a `Durable` fact is shown,
never pre-filled (T4b's C-1); a decision the code deferred to T7 is decided, not met.

## The fold
1. **C-1 — hash the words, show the banner.** In `answer.rs` (`:747`, `:817`): `let words =
   gate.prompt_text(&ri, params.as_ref()); let shown = format!("[{banner}] {words}");` — `shown` goes
   to `out`, `words` to `record_answer`. Kill: the sweep KAT (item 2) reds with the banner planted
   back into the hashed string.
2. **I-1 — the KAT drives the real command.** `income_answer_asks_the_dependent_gates_and_the_sweep_
   settles` runs `answer_return_inputs` itself (the `open_next_year_t4b.rs::answer_the_draft` harness
   pattern) and asserts `screen_inputs(&ri, &table, &params).is_none()` AND that every gate's
   `answer_status` is `Given`/`Current`, never `WordingChanged`; the emulation goes. This is C-1's
   kill — quote its red at `282a8a32`'s code.
3. **I-2 — the identity decision.** (a) In `screen_dependent_gates`, BEFORE any gate: a row with a
   blank `ssn` refuses `DependentIdentityUnanswered { row }` (class A, the same tier as
   `DateOfBirth`); two rows with the same `ssn` refuse `DependentSsnDuplicated { rows }` (INVALID —
   at import too, it is a value rule). (b) The sweep's session `asked` set is keyed by `(row, gate)`
   (never stored); the `answer_log` key stays the identity. (c) `retire_dependent_identity` cannot
   cross-delete once no two rows share a key — assert it. Kills: the review's two-row probe (row 1's
   gates are all asked; the interview does not end with a live gate unasked); a blank-SSN row blocks
   naming the row; a duplicated SSN refuses at import and at commit; `income answer` on a
   blank-SSN row asks for the SSN first (or refuses naming it — say which and why).
4. **I-3 — the seeded DOB is shown, not pre-filled.** `open_next_year::seed` leaves `date_of_birth:
   None`; the gate's prompt on an opened year shows year N's date as a hint (the `opened_from`
   pattern `carry_person` uses for the taxpayer); a bare Enter leaves the gate blocking; typing the
   date is a fresh `SetField` → a fresh record. Update
   `a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not`. Kill: the review's probe (seed,
   bare Enter → no record, the gate still blocking).
5. **I-4 — the shipped words.** Rewrite `cli.rs:573-584`'s two sentences to FR-70's behaviour (the
   person crosses with every §152 gate blank and the row blocks until this year's flowchart is
   answered or the row is removed; the closed list gains dependents' identity fields); `make docs`
   regenerates the man page; a snapshot holds the new sentence.
6. **I-5 — the missing truth-table rows.** `the_step_four_truth_table` gains the Step 4 citizen STOP
   (asserting the gate and a fragment of its own "adopted person" rule string — pinning that the
   Step 4 arm ran), Step 4 q4 (`Mfj` on the relative path ⇒ claimed, on to Step 5) and Step 5 q1
   answered No on the relative path. Kill: delete the Step 4 citizen arm → red.
7. **M-1** — `DependentVerdict::RefusedByQuestion(QuestionId)` so a return-level answer's refusal
   anchors on `DependentTaxpayer`, not a row gate. **M-2** — the 22-space runs. **M-3** — `income
   scrub` replaces a dependent's DOB with a synthetic date preserving `considered_age_at_year_end`
   (the module's `synthetic_*` pattern), the man page sentence updated to say so; kill: the scrubbed
   file carries no dependent's real DOB and every age-dependent screen reads the same. **M-4** —
   `DependentGateRefused` (a stated answer that STOPs) fires outside the `unanswered_refuses` gate,
   at import too; `DependentGateUnanswered` stays inside. **M-5** — the typed §152(d) table leaves
   the `help`; the figure comes from params only. **N-1** — a Jan-1/Jan-2 truth-table pair. **N-2**
   — `Census::dependent_gate` records its leaf so a swapped pairing reds. **N-3** — `clear` checks
   liveness like `set`.

## Constraints
- `record_answer` stays the only writer; the T7 truth table's existing rows unchanged; `stop-list`,
  `census-join`, `line-coverage`, `prompt-check` unmoved or as stated (list each).
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
APPEND a section `## Fold (seam review C-1, I-1 … I-5, M-1 … M-5, N-1 … N-3)` to
`design/agent-reports/2026-09-07-build-interview-T7-implementation.md`: per item what changed, the
I-2 decision as built, every kill with its red text, every pinned number moved, suite lines per
crate. Return only a 4-line summary.
