# Brief — fold the interview T1 seam review (1C/1I/4M/3N)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee you add needs a kill seen red once (plant, observe, revert via a `cp`
backup); the report says how, with the red text.

Read first: `design/agent-reports/2026-09-07-build-interview-T1-review.md` (findings from line 194) and
its ledger (C1 and I1 HOLD — fold, do not re-verify). The contract is `design/SPEC_interview.md` R10.

## Fold (the review's "Minimal change" is the default; deviate only with a reason)
- **C1** `crates/btctax-cli/src/cmd/tax.rs::import_return_inputs`: discard whatever the TOML says about
  `answer_log` AND `answer_log_history` and re-attach the stored row's (empty on a fresh year) — the
  same shape as the `CarryProvenance` normalisation block beside it. Update
  `scrub_refusal::the_scrubbed_toml_round_trips_back_through_import` so both sides get the same
  normalisation. Kills: (a) a TOML carrying a forged record (an old date, a valid current hash, state
  `Given`) → after import the row's log has NO such record; (b) a routine re-import of a year with
  14 genuine records → all 14 survive; plant the normalisation away → both red.
- **I1** make `record_answer` itself historise a record it is about to replace whose `prompt_hash`
  differs from the one it writes (option ii — the guarantee lives in the one writer), and ALSO run
  the idempotent sweep at the one read boundary (`return_inputs::row_to_inputs`) so a prompt changed
  between sessions is superseded on load without a direct call. Kills: the existing prompt-hash kill
  gains the second half — the history entry appears WITHOUT any direct call to the sweep; a per-load
  sweep does not grow history on repeated loads.
- **M1** record in the T5 brief's residue (the controller does) — you change nothing; say so.
- **M2** move the fixture's three notes into `fullreturn_oracle.rs` beside the fixture path,
  regenerate the fixture with its documented command, and add `fullreturn_fixture_matches_its_emitter`
  (a test that regenerates in-memory and diffs against the file); if the emitter is not idempotent,
  make it so (stable key order) and say what changed. Kill: hand-edit one byte of the fixture → red.
- **M3** restore the two corrupted doc comments (`apply(SetField)` / `apply(&mut form.working, Edit::…, now)`).
- **M4** replace the money detector's "honest limit" paragraph with the two real limits the review
  names, keeping the note about the classifier's second net.
- **N1** the fixture-shrink floor to 100 (measured 106). **N2** re-word `answer.rs:307-310`'s claim to
  what the code enforces. **N3** a doc note that `Declined` gets its production reader in T3/T12.

## Constraints
Do not touch the spec or any `design/agent-reports/*.md` other than your report. Goldens move only
as a direct consequence (the regenerated fixture and, if the emitter's order changed, the examples
golden) — list every diff line.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T1-fold-implementation.md`: per finding the change
(file:line), the kill and its red text, deviations, golden diffs, the exact nextest commands with
summary lines. Return ONLY a 3-line summary.
