# Brief — interview build T7: the dependents gates (R6, the gates half)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once; quote the red. Every pinned number
moved: old → new with cause. `/tmp` is a 32 GB tmpfs — build in the repo's target dirs. Synthetic
identifiers: SSNs from the never-issued space (area 000/666, group 00, serial 0000); EINs only from
`scripts/pii-scan-generic.sh`'s `ALLOWED_EIN`. Process in force (owner S6): ONE seam review and ONE
re-verification after you.

★ **Standing rules from every review so far:** no decision keys on a list you typed beside derived
data; a build's kills must ask what the NEXT SURFACE does with what the build wrote; every prompt
is the instruction's own words with its cite.

## The contract
`design/SPEC_interview.md` r2 **R6** — the gates half: Steps 1–5 as per-dependent `Option<bool>`
gates phrased as `i1040gi--2025.txt` phrases them, each with the instruction's own edge; the age
test COMPUTED from a REQUIRED `date_of_birth` (+ `full_time_student`, `permanently_and_totally_disabled`,
`younger_than_you_or_spouse` as its own gate, never from the taxpayer's declinable DOB); the
*Exception to time lived with you* as part of row (5)(a)'s condition with `:1905-1913` verbatim in
its `help`; the REFUSE edges naming their rule; `DEPENDENT_GATES: &[DependentGate]` in core with
`live(&ReturnInputs, row)`, get/set over `&Dependent`, the refusal, `Durability`; per-row liveness
by the I-4 emulation (`registries.rs:44-50`), NOT a widened seam (§10's freeze — decided);
`screen_inputs` rows × gates; `live_questions` + the Dependents section per row; the return-level
`filer_tin_issued_by_due_date` live iff any dependent row exists; the §152(d) gross-income figure
in `FullReturnParams` (TY2024/25/26) quoted in `gross_income_under_limit`'s prompt, so that gate
WAITS (never blocks) on a params-less year. **§7 row T7** (its kills). **R12** (T3's
`interview_state()` gains the `DEPENDENT_GATES × rows` walk — the hole T3 left, D7 — and `waiting`
gets its first real occupant). **T8 is NOT yours:** row (7)'s computation, rows (5)/(6)'s printing,
the TY2025 grid map and emitter, HoH/QSS gates. **FR-70 IS yours:** T4b's opener names dependents
in the prompt only; with the gates in place, decide identity-vs-declaration per `Dependent` field and
seed the identity fields (name, SSN, relationship, DOB) with every gate `None` — the `Dependent {..}`
literal in `open_next_year::seed` was written with no `..Default` tail so it fails to compile when
you add fields; that is the intended forcing function. **T15 is not yours** (line 19 stays a forgo).
Build AS WRITTEN; the tree's real names win; deviations recorded.

## Settled facts (controller-measured at `d9684adb`)
- `Dependent { name, ssn, relationship, date_of_birth: Option<Date>, … }`
  (`crates/btctax-core/src/tax/return_inputs.rs`); the classifier's `classify_dependent` currently
  says *"No classifiable leaves"* — every gate you add must be classified (no `..`, no `_`).
- T1 already defined the KEY: `AnswerKey::DependentGate { ssn_hash, gate }` with `enum DependentGate
  { QcRelationship, ProvidedOverHalfOwnSupport, FilingJointReturn, JointReturnOnlyToClaimRefund,
  QualifyingChildOfAnotherPerson, CitizenNationalResidentOrCanadaMexico, … }`
  (`provenance.rs:381`) and `record_answer` (the only writer of `answer_log`) keyed by identity — use
  it; a gate not in that enum is added there, with the `prompt_hash` rule and `answer_log_history`
  behaving as for questions (T1's kills extend to gates).
- The I-4 per-row emulation: `registries.rs:40-52` (`live: |_| true`; `get` returns `None` when not
  live; a set on a non-live question refuses `NoSuchRow`). `Field.live` is `fn(&ReturnInputs) ->
  bool` and §10 freezes the seam types.
- `FullReturnParams` at `crates/btctax-core/src/tax/tables.rs:453`; `full_return_for(year)` at
  `:567-571`; `YEAR.toml` for 2024/2025/2026 under `crates/btctax-forms/forms/`; TY2026 has no
  `FullReturnParams` (the params-less year — the `waiting` fixture).
- The taxpayer DOB is the `Durable` skippable `DobTaxpayer` (`questions.rs:1736,2009`), declinable;
  `DependentTaxpayer` is the existing return-level declaration; `ctc_odc_line19`
  (`advisories.rs:857`) stays as is.
- `dependents_statement.rs` prints the identity grid today; `form1040_full.rs` writes the identity
  block (`:390`) — T8's surface; do not change what prints.
- T4b: `open_next_year.rs` names dependents from `prior` in the prompt; `identities_of`'s dependent
  arm has `census_row: None`, `answer: None`.

## What T7 delivers
1. **The gate fields on `Dependent`**, per R6's table (Steps 1–5), each `Option<bool>`
  `#[serde(default)]`, plus `full_time_student`, `permanently_and_totally_disabled` (row (6) facts)
  and `younger_than_you_or_spouse`; `date_of_birth` becomes REQUIRED in the sense R6 gives (`None`
  refuses `DependentGateUnanswered { row, gate: DateOfBirth }` and the row is not printed — the
  printing half is T8's; you make the refusal fire before any print path). `LEAF_SOURCE` entries;
  classifier rows; TOML round-trip.
2. **`DEPENDENT_GATES`** in core: prompt (the instruction's words, cite in the doc comment), `live
  (&ReturnInputs, row)` encoding the flowchart edges (Step 4 gates live only when Step 1 sent the
  row there; the Step 2 gates reused at Step 4; `gross_income_under_limit` live iff the year's
  `FullReturnParams` exists AND the row is at Step 4), `get`/`set` over `&Dependent`, the refusal
  (every REFUSE edge names its rule: *Qualifying child of more than one person* `:1967`, *Married
  person* `:1945`, *"You can't claim this child as a dependent"*, *"You can't claim any
  dependents"*, the three rules at `:1823/:1949`), `Durability::PerYear`.
3. **`screen_inputs` rows × gates** (`DependentGateUnanswered { row, gate }` on a live `None`;
  the REFUSE edges), **`interview_state()`'s `DEPENDENT_GATES × rows` walk** (blocking / refusing /
  waiting — `gross_income_under_limit` is *waiting for the year package* on TY2026 and blocking once
  params exist), **`live_questions` + the Dependents section per row** (the I-4 emulation), and
  `income answer` asking them (keyed by `AnswerKey::DependentGate { ssn_hash, gate }` through
  `record_answer`).
4. **`filer_tin_issued_by_due_date`** — return-level class-(A) `FormQuestion`, live iff any
  dependent row exists; prompt from `:1765-1790`.
5. **The §152(d) figure** in `FullReturnParams` for TY2024 / TY2025 (TY2025: $5,200 — verify against
  the instruction text `:1688-1693` and the archived Rev. Proc., cite both), TY2026 absent until its
  package; `gross_income_under_limit`'s prompt quotes the figure.
6. **FR-70:** the opener seeds each prior dependent's identity fields with every gate `None`, its
  prompt line unchanged; the seeded row is then blocking through the gates (the answer surface
  exists now). The identity-vs-declaration decision per field is recorded in the report.

## Kills (each seen red once)
A truth-table KAT per flowchart edge (every row of R6's table: a fixture at that edge yields the
instruction's outcome — Step 4, the named refusal, or the credit-column verdict T8 will print —
expose the verdict as an enum so T8 reads it). Every gate `None` while live ⇒
`DependentGateUnanswered`; every REFUSE message contains its named rule. `date_of_birth = None`
refuses. A `Single` return whose taxpayer-DOB skippable is `Declined` still resolves Step 1 for a
row with `younger_than_you_or_spouse = Some(true)`. A child with `date_of_birth = 2026-11-01` and
`lived_with_you_over_half_year = Some(true)` lands on the CTC edge (the Exception is part of the
condition), and `line-coverage` checks row (5)(a)'s `help` quotes `:1905-1913` verbatim (the
`FilerRecords` production for that row). `gross_income_under_limit` waits on a params-less fixture
and blocks with params inserted, its prompt containing the year's figure. The classifier reds until
each new leaf is classified. A `Single` return with no dependents asks no gate and not
`filer_tin_issued_by_due_date`. Two dependent rows' gates answered, row 0 deleted ⇒ row 1's records
intact (T1's identity key, re-asserted through the real gates). The opener's seeded dependent is
listed in `interview_state().blocking` (FR-70 closes; the T4b test that pinned "not seeded"
rewritten).

## Constraints
- `record_answer` stays the only writer; no default answer; no gate computed from a declinable value.
- Nothing prints differently on TY2024 (the emitter is T8's); `report` output on the existing
  fixtures may gain the new blocking items only where a fixture has dependents — regenerate goldens
  with each moved line explained, or give those fixtures the answers a real filer would give.
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T7-implementation.md`: per numbered item what
landed, the gate table as built (gate → prompt cite → edge → refusal), the truth-table KAT's rows,
the FR-70 identity-vs-declaration decision per field, every deviation, every kill with its red
text, every pinned number moved, suite lines per crate. Return only a 4-line summary plus the path.
