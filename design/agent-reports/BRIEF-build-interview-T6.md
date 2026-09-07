# Brief — interview build T6: the exchange seam (R9)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once; quote the red. Every pinned number
moved: old → new with cause. `/tmp` is a 32 GB tmpfs — build in the repo's target dirs. Synthetic
identifiers: SSNs from the never-issued space (area 000/666, group 00, serial 0000), EINs only from
`scripts/pii-scan-generic.sh`'s `ALLOWED_EIN` list. Process in force (owner S6): ONE seam review and
ONE re-verification after you.

★ **Standing rule from every review so far:** no decision keys on a list you typed beside derived
data — compare against the structure or the source. And a build's kills must ask what the NEXT
SURFACE does with what the build wrote.

## The contract
`design/SPEC_interview.md` r2 **R9** (two question sets, two stores, one journey: Step 0 is a
STATUS PANEL, never a question set; the crypto section asks only return-side questions; the
Digital Assets question is a class-(A) `FormQuestion` cross-checked against the predicate that
already exists — a contradicted `No` refuses naming the first qualifying event, an unwitnessed `Yes`
is accepted with a WARNING and never refused; venue/account granularity is documented, not asked),
**§7 row T6** (its kills), **R12** (the panel function shape), **R15** (no `reconcile` question in a
return registry — the grep-KAT exists from T3: extend, do not duplicate), spec 1099-DA
`design/SPEC_1099da_broker_reporting.md` (R1: answered-ness lives in the key set; R6: the slice
path) and the standing order in `legal/text/irs-guidance/Notice_2026-20.txt` §4.02(2). Build AS
WRITTEN; the tree's real names win; deviations recorded.

## Settled facts (controller-measured at `85962806`)
- `digital_asset_activity(state: &LedgerState, year) -> bool` exists,
  `crates/btctax-core/src/tax/return_1040.rs:2468` (`pub(crate)`; the instruction's own list in its
  doc comment); `screen_compute_dependent` at `:907` is the tier that sees the ledger. Today the
  printed box is decided in `crates/btctax-cli/src/cmd/admin.rs:1122` as `da_yes = !rows.is_empty()
  || income recognized in the year …` — never `No` from an answer; T6 replaces that with the
  answer, cross-checked.
- `BlockerKind` (`crates/btctax-core/src/state.rs:23`) has 21 variants; `MethodElection`
  (`crates/btctax-core/src/event.rs:282-296`: `effective_from`, a `WalletId::Exchange{..}` scope —
  only Exchange wallets are electable; FIFO before any election) is the standing-order object.
- `BrokerReporting(BTreeMap<String, CohortAnswers>)` per provider (`forms.rs:339`), seeded from the
  ledger by `btctax-input-form/src/spec/mod.rs` (`broker_row_provider`); presence in the key set IS
  answered-ness (T4b's I-3 — never insert a key without an answer).
- T3's `interview_state()` and `FORM_QUESTIONS` discipline (classifier rows, no `..`/`_`); T1's
  `record_answer`; T4's `screen_param_free` tier (does NOT include ledger-dependent rules — the DA
  cross-check is compute-dependent and runs at commit, not at import); T4b's `opened_from`.
- `Pane::RowList` and the pane machinery in `crates/btctax-tui-edit/src/draw_edit.rs:2445`.

## What T6 delivers
1. **The Step 0 status panel** — a `btctax-cli` function over the held session (no new store):
   blockers by `BlockerKind`; unresolved conflicts; imports per venue; venues with dispositions in
   the year versus providers with a `BrokerReporting` answer (a venue in the year's 8949 rows with no
   answer is NAMED); venues with a custodial disposition in the year and no `MethodElection` with
   an Exchange scope effective before it — the Notice 2026-20 §4.02(2) standing order, stated with
   its consequence (*"expect box 1g to reflect the broker's default; a `BasisDiffers` answer refuses;
   `select-lots` / `import-selections` is the exit"*); owner actions with dates. Every row hands off
   to the `reconcile` command that answers it. Rendered as a TUI pane (the interview's entry) and
   printed by `income answer` before the census (document-first order stays: the panel is status,
   not a question). Authoring proceeds with an unresolved ledger; commit and export stay hard-gated
   by the gates that already exist (assert, do not add).
2. **`digital_asset_activity: Option<bool>`** on `ReturnInputs` — a class-(A) `FormQuestion`, always
   live, `PerYear`, prompt from `i1040gi--2025.txt:1346-1362` verbatim; classifier row;
   `#[serde(default)]`; `LEAF_SOURCE`; `interview_state` lists it. A `screen_compute_dependent` rule:
   `digital_asset_activity(state, year)` ∧ `No` ⇒ refuse `DigitalAssetAnswerContradictsLedger`
   naming the FIRST qualifying event (date, venue, kind); `!activity` ∧ `Yes` ⇒ accept with the
   off-ledger WARNING (R9's sentence) — never a refusal. The printed box (`da_yes`/`da_no`, the
   `admin.rs:1122` site) is the ANSWER, not the ledger predicate; `None` blocks commit.
3. **The standing-order warning** (Notice 2026-20 §4.02(2)) on a fixture ledger with a 2026 Coinbase
   disposal and no scoped election; silent with one effective the day before.
4. **Venue-vs-answer listing** in the panel and the commit modal.
5. **Documented, not asked:** venue/account granularity (the `default` account segment,
   `btctax-adapters/src/normalize.rs:63-69`) — a doc sentence, no field.

## Kills (each seen red once)
The five-row DA table as one test: activity ∧ `No` refuses naming the first qualifying event;
activity ∧ `Yes` prints `Yes`; no-activity ∧ `No` prints `No`; no-activity ∧ `Yes` prints `Yes` with
the off-ledger warning and NO refusal; a ledger holding only `Acquire` events and linked
self-transfers answered `No` prints `No`. The standing-order fixture pair. The unnamed-venue
fixture (a venue in the year's 8949 rows, absent from the answers, named in the panel). The
grep-KAT on registry prompts (no *transfer* / *lot* / *FMV* — T3's `stop-list`, extended if the
prompt set grew). `digital_asset_activity = None` blocks commit and is listed by `interview_state`;
the answer survives `income import` round-trip; T4b's opener seeds it `None`.

## Constraints
- No `reconcile` question moves into a return registry; the panel reads the ledger and writes
  nothing. `record_answer` stays the only writer of `answer_log`.
- The R6 slice-filing path and every existing commit/export gate unchanged (their tests green).
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T6-implementation.md`: per numbered item what
landed, the panel's row list as printed on a fixture, the five-row table's outputs, every deviation,
every kill with its red text, every pinned number moved, suite lines per crate. Return only a 4-line
summary plus the path.
