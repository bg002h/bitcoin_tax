# Brief — interview build T5: the 1099-INT / DIV / B / G and 1098-E sections (R4, R5, R3's paired questions)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once; quote the red. Every pinned number
moved: old → new with cause. `/tmp` is a 32 GB tmpfs — build in the repo's target dirs. Process in
force (owner S6): ONE seam review and ONE re-verification after you.

★ **Standing rule from the T2/T3/T4 reviews (each Critical was this):** no decision may key on a
list you typed beside derived data. Compare against the structure (the census, the extract, the
registry, `LEAF_SOURCE`) or make the list the single source and derive the rest from it.

## The contract
`design/SPEC_interview.md` r2 **R4** (one repeating section per document type; the box captions
verbatim from the archived extract; every box carries exactly one census entry —
`collected(FieldId)` / `refuse_if_nonzero(RefuseReason)` / `not_read(reason)`; the arithmetic checks
are WARNINGS the tool displays and never writes; the all-zero-row warning; `transcribed_on = None`
printed as *transcribed without a date*), **R5** (`FilerRecords` sources for the lines with no
issuer), **R3** (the three document-less income questions and `schedule_b_filer_records`, each live
iff its census row is `Some(false)`; `itemized_prior_year` return-level), **§5.1**, **§5.2** (the
struct fields; `student_loan_interest_paid` removed; **`mortgage_interest_1098` and Form 1098 are T9,
not yours**), **§5.7** (the coverage fixture), **§7 row T5** (its kills), **§8**. `FOLLOWUPS.md`
**FR-65** (the 2026-only boxes: 1099-G Rev. Dec 2026 box 10 *Family leave benefits*; W-2 2026 box
14b *Treasury Tipped Occupation Code(s)*), **FR-66** (the census join's scope — do NOT widen it here;
only note what you touch), **FR-68** (the PAREN flip — not yours). Build AS WRITTEN; the tree's real
names win; deviations recorded.

## Settled facts (controller-measured at `68467dde`)
- Structs today (`crates/btctax-core/src/tax/return_inputs.rs`): `Form1099Int` 11 pub fields,
  `Form1099Div` 15, `Form1099B` 9, `Form1099G` 6 (`payer, payer_tin, transcribed_on, box1_unemployment,
  box4_fed_withheld`, + owner) — **no box 2**; `sch1.student_loan_interest_paid: Usd` (`:725`).
  T3's `document_census.rs` has `declared_rows` / `requires_transcription` (INT/DIV `true`; B `false`
  by the form's own option; G `false` **until you add box 2** — flip it, with its kill).
- The input form (`crates/btctax-input-form/src/spec/sections.rs`): the W-2 section is the model
  (`SectionId::W2s`, `FieldId::W2Owner/W2Employer/W2Ein`, `w2_money!(FieldId::Box1Wages, …)` at
  `:642-711`); 75 `FieldId::` uses. `EXEMPT_PREFIXES` (`coverage.rs:381-388`) currently exempts
  `int_1099`, `div_1099`, `g_1099`, `b_1099`, the two carryforward `_in` leaves and more — a ratchet
  that may only shrink. `attribute.rs` has 34 `NotInForm` anchors; the five R4 names are at
  `:235-269`.
- The per-edition box census (`crates/xtask/src/box_census.rs`, T2 + its fold): `BoxEntry { stem,
  editions, label, caption, decision: Collected(..) | RefuseIfNonzero(..) | NotRead(..) }`, 246
  printed boxes / 15 editions / 123 entries; `Collected` today names a field PATH as a string — T5
  joins it to the input form's `FieldId` (the T2 report's deferred "no FieldId join"). A `NotRead`
  whose reason begins *"T5"* is yours to decide.
- The TUI's repeating-row machinery: `crates/btctax-tui-edit/src/edit/tax_inputs.rs::add_row` (`:309`)
  and the W-2 flow tests at `:1119-1167`.
- T3's census rows, T1's `record_answer` (the only writer of `answer_log`), T4's draft rules.

## What T5 delivers
1. **The four 1099 sections and the 1098-E section in the input form**, each a repeating section
   like `W2s`: per row the payer identity (`payer`, `payer_tin`, `transcribed_on`) plus one `Field`
   per collected box named for the box (`Int1099Box1`, …), the caption verbatim as `help`, reaching
   the line R4's table names. `Form1099G` gains `box2_state_refund: Usd`; `Form1098E { lender,
   lender_tin, transcribed_on, box1_interest }` replaces `sch1.student_loan_interest_paid` (the
   Schedule 1 line 21 chain reads the rows' sum). Every box in every ARCHIVED EDITION carries exactly
   one census decision: `Collected(FieldId)` (joined to the registry — a `FieldId` that does not
   exist or is not in that section reds), `RefuseIfNonzero(RefuseReason)` or `NotRead(reason with the
   instruction's pointer)`. Decide R4's named boxes as the spec decides them: 1099-INT box 10
   collected into the Schedule B line-1 / 2b sum; boxes 11–13 `RefuseIfNonzero
   (AmortizableBondPremiumNotComputed)` naming Pub. 550; W-2 box 13 *Statutory employee*
   `Collected` as a `Bool` whose `true` refuses `StatutoryEmployeeW2` naming Schedule C line 1;
   1099-DIV box 3 `NotRead` with its reason. **FR-65:** 1099-G box 10 *Family leave benefits* (Rev.
   Dec 2026 edition) — read `i1099g--2026` and `i1040gi--2025` for the line it reaches and decide it
   as the instructions say (collected to that line, or `RefuseIfNonzero` naming it if the line is
   outside the build); W-2 box 14b *Treasury Tipped Occupation Code(s)* — join it to Schedule 1-A
   line 4a's tips gate (read `i1040s1a--2025` for the rule) or refuse when non-empty, never
   `NotRead`. Remove every `NotRead("T5 …")` by deciding it.
2. **The paired document-less income questions** (R3), on `ReturnInputs`, each a `FormQuestion`
   live iff its census row is `Some(false)`: `w2_wages_without_w2` (`Yes` refuses `WagesWithoutW2`
   naming Form 1040 line 1a), `interest_or_dividends_without_1099` (`Yes` opens
   `schedule_b_filer_records: Vec<ScheduleBRecord { payer_name, payer_ssn, payer_address, amount,
   kind: Interest | Dividend }>` — `Source::FilerRecords`; non-empty then required; the rows reach
   Schedule B lines 1 / 5 and the 2b / 3b sums), `state_refund_without_1099g` (`Yes` refuses naming
   the State and Local Income Tax Refund Worksheet — the same exit 1099-G box 2 takes); and
   `itemized_prior_year: Option<bool>` return-level, live iff `state_refund_without_1099g ==
   Some(true)` or any `g_1099[].box2_state_refund > 0` — find where the prior-year-itemized flag
   lives today (grep `itemized`) and move it, with the classifier rows (no `..`, no `_`). Prompts are
   the instructions' words with the cites R3 gives.
3. **Warnings, never writes:** W-2 box 4 ≈ 6.2% × box 3, box 6 ≈ 1.45% × box 5, box 3 ≤ the wage
   base; a row whose every income box is zero (*"a payer issues one at $10 or more — check the row"*);
   rendered where the row is edited and in `report`; a warning never changes a stored value (kill:
   a planted box-1-in-box-3 fixture warns and the row is byte-identical).
4. **`EXEMPT_PREFIXES` shrinks** by `int_1099`, `div_1099`, `g_1099`, `b_1099` and
   `sch1.student_loan_interest_paid` (the carryforward `_in` leaves stay — T4b's); the coverage KAT's
   pinned Field count rises from its current value and the exempt count is asserted ≤ its new value.
   **The five `NotInForm` anchors** (`PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`,
   `InconsistentDividendSubset`, `ForeignTaxOverCeiling`, `Form1099BNeedsForm8949`) become `Field`
   / `Section` anchors; a test asserts the `NotInForm` count fell by exactly five;
   `IraDeductionClaimed` stays `NotInForm`.
5. **`LEAF_SOURCE`** (T1) gains every new leaf with its source (`Document(kind)` for the boxes;
   `FilerRecords` for the Schedule B records); the packet manifest prints a row with
   `transcribed_on = None` as *transcribed without a date* (§4.4).
6. **Fixtures** (§5.7): `maximal_fixture` carries one row of each new `Vec` (box 9 = 0 on the INT row;
   2b/2c/2d/13 = 0 on the DIV row; the B row's gate `Some(true)`; one 1098-E row; one Schedule B
   record on the variant whose supported rows are `Some(false)`); the TY2024 example fixtures gain
   whatever the removed scalar held (the 1098-E row replacing `student_loan_interest_paid`), goldens
   regenerated with each moved line explained.

## Kills (each seen red once)
The `[boxes]` census complete for every archived edition: a deleted entry reds; an entry naming a
caption the extract does not carry reds; a `Collected(FieldId)` naming a field outside its section
reds. Box 13 checked refuses naming Schedule C line 1; box 11 > 0 refuses naming Pub. 550; box 10
reaches the 2b sum (a fixture with box 10 = $100 prints 2b higher by $100). `NotInForm` fell by five.
The coverage KAT count and ratchet. Each paired question live exactly on its row's `Some(false)`
(R3 kill (f) verbatim: `w2 = Some(false)` with the wage question `None` refuses; `Some(true)` never
asks it; `Yes` on the wage and refund questions refuse naming their exits; `Yes` on the interest
question with no Schedule B record refuses while one record passes); `itemized_prior_year` live on a
return with NO 1099-G row when the refund question is `Some(true)` (R3 kill (g)); `g_1099 =
Some(true)` with one row whose only amount is box 2 passes and reaches the worksheet refusal at
commit. The W-2 arithmetic warning and the all-zero-row warning fire on their fixtures and write
nothing. The 1098-E row's sum reaches Schedule 1 line 21 (the old scalar's test rewritten). The
manifest prints *transcribed without a date* for a dated-`None` row.

## Constraints
- `record_answer` stays the only writer of `answer_log`; no default answers; every refusal names its
  exit in the form's words; `#[serde(default)]` on every new field; `income import` accepts the new
  tables and refuses unknown keys naming them (the existing KAT).
- Form 1098 / Schedule A 8a / `mortgage_interest_1098` are T9's — do not touch them.
- `line-coverage` counts move only where a printed line's production changed (list each); the join's
  scope (13 maps) is unchanged.
- **This task is large. If context runs short:** leave the tree compiling, fmt-clean and green on
  every crate you touched, and state precisely which numbered item and which document is
  unfinished — the controller dispatches a continuation from your report.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T5-implementation.md`: per numbered item what
landed, the census decision table per document (label → decision, with the FR-65 decisions and
their cites), every deviation with its reason, every kill with its red text, every pinned number
moved (old → new, cause), suite lines per crate. Return only a 4-line summary plus the path.
