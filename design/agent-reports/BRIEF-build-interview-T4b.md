# Brief — interview build T4b: the year-N+1 opener (R10 part 4)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create (revert a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E
'<filter>'` (never `cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and
a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D
warnings` before finishing. Every guarantee lands with a kill seen red once; quote the red. Every
pinned number moved: old → new with cause. `/tmp` is a 32 GB tmpfs — build in the repo's target dirs.
Process in force (owner S6): ONE seam review and ONE re-verification after you.

## The contract
`design/SPEC_interview.md` r2 **R10 part 4** (the year-N+1 opener: identities as fresh tri-state
prompts, every box blank, every `PerYear` gate `None`, every `Durable` fact shown and confirmed by a
fresh keystroke, carryforwards as DATA with `ComputedFromPriorReturn { year }`, never a carried
amount), **§7 row T4b** (its kills), **R11/T4** (the draft is the year-N+1 store; `income answer`
answers into a draft-only year; the T4 draft-protection rules bind the opener's write), **R10
parts 1–3 / T1** (`record_answer` is the one writer of `answer_log`; `prompt_hash`; `Durability` at
`questions.rs:28-35`), **§6 J-27** (February 2028 opens TY2027). Build AS WRITTEN; the tree's real
names win; deviations recorded.

## Settled facts (controller-measured at `bf26f1e8`)
- Identity fields: `W2 { owner, employer, ein: Option<String>, … }`; the 1099 structs carry `payer`
  + `payer_tin`; `Dependent { name, ssn, relationship, date_of_birth: Option<Date>, … }`; venues are
  the per-provider `BrokerReporting(BTreeMap<String, CohortAnswers>)` answers (`forms.rs:339`,
  reached through `input_form_store::broker_answers`), keyed by the provider name.
- The carryforward-OUT chain lives on the frozen return (`return_1040.rs:1600-1632`:
  `capital_loss_carryforward_out`, `charitable_carryover_out`, `qbi_carryforward_out`,
  `qbi_reit_ptp_carryforward_out` — the doc there says *"this is NOT `TaxResult::carryforward_out`,
  and the difference is the defect"*: read the frozen one). `report --write-carryover` (T4 report:
  `cmd/tax.rs:1062`, coherence on year N+1 at `:973`) already writes year N's computed carryovers
  onto year N+1's COMMITTED row with `--discard-draft` threaded; the `_in` leaves carry a sibling
  `*_provenance: CarryProvenance` (`return_inputs.rs:1031-1043`) and `ComputedFromPriorReturn`
  exists (`:612`).
- `Durability::{PerYear, Durable}` (`questions.rs:28-35`); `SkippableQuestion.durability`,
  `FormQuestion.durability`; the DOB is the one `Durable` occurrence (`questions.rs:971-980`).
- T4: `save_draft` (`input_form_store.rs:140`); a non-trivial WIP draft is discarded only on
  `--discard-draft` / a TUI payload-confirm; `income answer` writes a draft-only year through
  `record_answer` + `save_draft`.

## What T4b delivers
1. **`open_next_year(sess, from: i32) -> Result<Opened, CliError>` in `btctax-cli`** (a module of
   its own). Reads year N's committed row (`return_inputs::get`) and year N's frozen return (the
   carryforward-out chain, computed the way `report --write-carryover` computes it — share that code,
   do not duplicate it), and seeds year N+1's **draft** (`save_draft`; never `return_inputs::set`)
   with: each W-2 employer (name + EIN), each 1099 payer (name + TIN) per kind, each dependent (name,
   SSN, relationship, `date_of_birth`), and each venue — as **pre-named rows with every money box
   default and every `PerYear` gate `None`**; `documents.*` all `None` (the census is re-asked);
   `answer_log` EMPTY (no record is carried — a `Durable` fact is displayed but has no
   `AnswerRecord` until the filer confirms it with a fresh `SetField`, which writes a fresh record
   through `record_answer`); the two carryforwards (`capital_loss_carryforward_in`,
   `charitable_carryover_in`, and the QBI pair if year N's return computed them) set from the frozen
   chain with `CarryProvenance::ComputedFromPriorReturn { year: N }`; every other `Usd` leaf
   default. Refuse (nothing written) when year N has no committed row, when year N+1 already has a
   committed row, or when year N+1 has a non-trivial WIP draft and `--discard-draft` was not given
   (T4's rule, same flag).
2. **Each identity presented as its own tri-state prompt.** `Opened` carries the identity list; the
   CLI `btctax income open-next-year --from N` prints, per identity, R10's sentence in the form's
   words (*"Last year Acme (EIN 12-3456789) issued you a W-2. Did Acme issue one for 2027?"*) and
   the seeded row exists only if the answer is Yes — model the answers as the corresponding
   `documents.*` census row plus the pre-named row (a No removes the pre-named row; a `None` leaves
   it pending and blocking, as the census does). Decide and record whether the per-identity answer
   is a new `FormQuestion` per row or the census row itself with the identity list rendered beside
   it; either way no new writer of `answer_log` and no default answer.
3. **The TUI entry action.** On the year picker (`editor.rs:96,385,413`; `draw_edit.rs:194`), a
   year N+1 with no committed row and no draft, whose year N has a committed row, offers *"Open
   TY(N+1) from TY(N)"*; the same refusals as the CLI; the payload-confirm for a non-trivial draft.
4. **Retention untouched:** nothing is shredded, nothing is deleted from year N.

## Kills (each seen red once)
R10.4's: a fixture year N with two payers yields exactly two identity prompts, both `None`, and the
opened screen's boxes are all default. Plus: no `Usd` leaf of the seeded draft is non-zero except
the carryforwards, each carrying `ComputedFromPriorReturn { year: N }` (walk the leaves with T1's
`LEAF_SOURCE` machinery, not a hand list); every seeded `PerYear` gate is `None`; every seeded
`Durable` fact has NO `AnswerRecord` until confirmed, and a confirmation writes a fresh record dated
`BTCTAX_NOW` with the current prompt hash; the carryforwards are read from year N's frozen RETURN —
plant a year-N committed row whose `capital_loss_carryforward_in` differs from the return's `_out`
and assert the seed takes the `_out`; year N+1 with a committed row refuses; a non-trivial draft
refuses without `--discard-draft`; `return_inputs::get(N+1)` is `None` after opening; the TUI action
is absent when year N has no committed row.

## Constraints
- `record_answer` stays the only writer of `answer_log`; the opener writes none.
- Nothing about year N changes; `report --write-carryover`'s behaviour is unchanged (if you share
  its chain, its tests stay green).
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T4b-implementation.md`: per numbered item what
landed, the identity model you chose and why, every deviation, every kill with its red text, every
pinned number moved, suite lines per crate. Return only a 4-line summary plus the path.
