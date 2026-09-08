# Verification ledger — interview T7 seam review (`2026-09-07-build-interview-T7-review.md`, 1C/5I/5M/3N)

Controller machine-checks from the main tree at `4a849401`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| C-1 | `income answer` records dependent gates under a banner-prefixed prompt while `current_prompt` resolves the bare one | grep | `answer.rs:747 let prompt = format!("[{banner}] {}", gate.prompt_text(…))`, `:817 record_answer(&mut ri, key, &prompt, …)`; `provenance.rs:761 (!q.needs_params()).then(\|\| q.prompt_text(ri, None))` | true — every gate reads `WordingChanged` on the spot; the exit named is the command just run: a brick; **FOLD** |
| I-1 | the sweep KAT emulates the loop without `record_answer` | grep over the test body | 0 occurrences of `record_answer` in the KAT | true; **FOLD** (drive the real command) |
| I-2 | the sweep's `asked` set is keyed by `(ssn_hash, gate)`; blank/duplicate SSNs collide; no SSN gate at authoring; `provenance.rs` deferred the decision to T7 | grep | `key_of` uses `dependent_ssn_hash(`; `return_refuse.rs:1910 "NO SSN GATE HERE, DELIBERATELY"`; `provenance.rs:784 "Recorded here so T7 decides it rather than meets it"` | true; **FOLD** (decide: a blank SSN blocks, a duplicate refuses, before any gate; the session `asked` set keyed by `(row, gate)`) |
| I-3 | the opener seeds the dependent's DOB and a bare Enter records `Given` | grep | `open_next_year.rs:444 date_of_birth: d.date_of_birth`; `answer.rs:768 Some(_) => break` then the unconditional record | true — T4b's C-1 rule broken on T7's one `Durable` gate; **FOLD** |
| I-4 | the shipped help says *"no row is created"* | grep | `cli.rs:583` and the man page `:13` | true; **FOLD** |
| I-5 | no truth-table row covers the Step 4 citizen STOP | grep over `the_step_four_truth_table` | 0 `CitizenNationalResidentOrCanadaMexico` rows | true; **FOLD** (rows for the citizen STOP, Step 4 q4, Step 5 q1) |
| M-1 … M-5, N-1 … N-3 | attribution on the wrong control; 22-space runs; scrub emits a dependent's real DOB; the STOP gated behind `unanswered_refuses`; the §152(d) figures typed again in `help` (`dependent_gates.rs:1112`); the Jan-1 boundary unpinned; `Census::dependent_gate` ignores its leaf; `clear` unguarded | read / grep | true | fold inline (M-3 folded with the module's own `synthetic_*` pattern — non-gating class, but the pattern exists and the file is stamped shareable) |
| (a)–(d) | the pointed questions | the review's answers | (a) scrub carries a dependent's real DOB — M-3; (b) the boundary is correct, unpinned — N-1; (c) reachable before the packet boundary — I-2; (d) three edges asserted only in prose — I-5 | recorded |

## Disposition
- **C-1 / I-1 — FOLD.** The banner is display only; `record_answer` hashes the registry's words
  (the `Declaration` arm's pattern at `answer.rs:574`); the sweep KAT drives the REAL
  `answer_return_inputs` (the `answer_the_draft` harness) and asserts `screen_inputs(...).is_none()`
  — plant the banner back into the hashed string → red.
- **I-2 — FOLD, decided:** a dependent row with a blank SSN is class-(A) blocking before any gate
  (`DependentIdentityUnanswered { row }`, the same tier as `DateOfBirth`); two rows with the same SSN
  refuse `DependentSsnDuplicated` (INVALID, at import too); the session's `asked` set is keyed by
  `(row, gate)`; `retire_dependent_identity` cannot cross-delete because no two rows share a key.
- **I-3 — FOLD:** the seed leaves `date_of_birth: None`; the prompt shows year N's date as a hint;
  a bare Enter does not satisfy the gate (it blocks until typed); T4b's DOB test updated.
- **I-4 — FOLD:** the two `cli.rs` sentences rewritten to what FR-70 does; `make docs`.
- **I-5 — FOLD:** the three truth-table rows, each asserting the gate and a fragment of its own rule.
- **M-1 … M-5, N-1 … N-3 — fold inline.**

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T7-review.md`.
