# Brief — interview build T1: the provenance schema (the first task, because it cannot be back-filled)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once (plant, observe, revert via a `cp`
backup); the report says how, with the red text. Every pinned number moved: old → new with cause.
Process in force (owner S6): this build gets ONE seam review and ONE re-verification after you.

## The contract
`design/SPEC_interview.md` (r2, GREEN by S6): rule **R10** (provenance — `LEAF_SOURCE`, the answer
log with one writer, `Declined`, the `prompt_hash` mismatch rule, `CarryProvenance`, the year-N+1
mechanism), rule **R12** where it names the four-state panel's inputs from the log, **§5** (the data
model — every new struct/field with its `#[serde(default)]` discipline; the `SCHEMA_VERSION` bump),
**§7 row T1** (the task, its kills and its "once" cadence) and **§8** (consolidated kills). The fold
report `design/agent-reports/2026-09-07-spec-interview-fold.md` has a 17-row "what the build must now
prove" table — the rows tagged T1 (I9, I10 and any other) are yours. Build T1 AS WRITTEN; where the
text and the tree disagree, the tree's real names win and you say so in the report; where a sentence
cannot be built as written, build the nearest thing that keeps the guarantee and record it under
"Deviations" — never silently.

## What T1 delivers (the spec's own words are the spec; this is the map)
- `LEAF_SOURCE`: the per-leaf provenance table (which document box / gate / computation / refusal each
  `ReturnInputs` leaf comes from) with the KAT that every in-scope leaf appears in it exactly once
  and every entry names a real leaf — both directions.
- Document identity on document rows (`payer_tin`, `transcribed_on`) and the `Declined` state as a
  structural third value where the spec puts it (never a default).
- `answer_log` keyed by IDENTITY (`DependentGate { ssn_hash, gate }` etc., never a row index) +
  `answer_log_history` + `record_answer` as the ONE writer, reached identically by the TUI's `apply`
  and by `income answer` (kill: the two paths produce identical records).
- The `prompt_hash` mismatch rule: an answer given under earlier prompt text is re-asked; the old
  record moves to history, never stays current (kill: change one fixture prompt's text → its answer
  reappears in the blocking/forgoing panel with the changed-wording reason and the superseded record
  in history; restoring the text un-does it and adds no history entry).
- `CarryProvenance::ComputedFromPriorReturn` on carryovers.
- `SCHEMA_VERSION` 3 with the refuse / discard / parked-refuse split for older drafts (kills: a v2
  committed row refuses; a v2 WIP draft discards with the note; a v2 parked draft refuses).
- Stable dependent identity (I10): answer gates on two dependent rows, delete row 0 → row 1's records
  unchanged and row 0's gone; change row 1's SSN → its records move to history.
- Anything else §7's T1 row lists that this map omits — the row wins.

## Where the seams are (read before writing)
`crates/btctax-core/src/tax/return_inputs.rs` (`ReturnInputs`, the nested structs, the serde-default
discipline), `crates/btctax-core/src/tax/classifier.rs` (the answered-ness classes the new fields
join), `crates/btctax-input-form/src/{seam.rs,apply.rs,spec/coverage.rs}` (the coverage KAT and its
`EXEMPT_PREFIXES` — a provenance leaf that is not testimony belongs in the exempt list WITH a reason,
never silently), `crates/btctax-cli/src/input_form_store.rs` (`SCHEMA_VERSION`, `load`, the stale
split, `save_draft`, `commit`), `crates/btctax-cli/src/cmd/answer.rs` (the CLI writer),
`crates/btctax-tui-edit/src/edit/{tax_inputs,persist}.rs` (the TUI writer). The kitchen-sink fixture
`crates/btctax-core/src/tax/testonly.rs` and the examples fixture
`crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` (GENERATED — regenerate it with
the command in its header if the schema adds fields, and say what changed).

## Constraints
- No filer-facing prompt text changes beyond what T1 needs (the prompt-hash rule needs the hash, not
  new prompts). No new document screens (that is T5). No year-N+1 opener (T4b).
- `docs/examples/examples.md` and the man pages regenerate ONLY as a direct consequence; list every
  diff line. The census registers and `max_unwitnessed` do not move.
- Do not touch the spec or any `design/agent-reports/*.md` other than your report.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T1-implementation.md`: per T1 item the change
(file:line), the kill and its red text, deviations with reasons, golden/fixture diffs, pinned numbers
moved, the exact nextest commands with summary lines, anything not done and why. Return ONLY a 3-line
summary (what landed, suite results for the crates you touched, the report path).
