# Brief — interview build T8: row (7) computed, HoH / QSS as the instruction's tests, the TY2025 dependents grid mapped and filled

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

★ **Standing rules:** no decision keys on a list you typed beside derived data; a build's kills must
ask what the NEXT SURFACE does with what the build wrote; a form is TRANSCRIBED from its text layer
(`pdftotext -layout`), never from the rendered page; every line has a determinate PROVENANCE, and a
blank is normal — "blank because the inputs say so" is correct, "blank because nothing populated
it" is the defect.

## The contract
`design/SPEC_interview.md` r2 **R6 — the computed half**: row (7) COMPUTED, never asked (CTC when the
Step 1–3 chain passes; ODC when the chain reaches *Credit for other dependents*; both blank
otherwise); rows (5) and (6) print from the gates; the emitter fills the TY2025+ grid from the
computation, TY2024's grid unchanged; line 19 stays a forgo SIZED in the panel from the year's
parameters and blank on a params-less year. **R7** — HoH and QSS are assertions that ask the
instruction's tests: `hoh_marital_basis: Option<HohMaritalBasis { NotMarried,
LegallySeparatedByDecree, MarriedLivedApart, NraSpouseNoElection }>` live iff `filing_status ==
Hoh`; `MarriedLivedApart` refuses naming *Married persons who live apart* (`:1247`) unless you
transcribe its five conditions as five gates (optional, same task); `NraSpouseNoElection` refuses
naming *Nonresident aliens and dual-status aliens* — and this is where **FR-67** (the §6013(g)/(h)
election gate, T3 review I5) lands: a return-level gate beside the HoH/QSS ones whose `Yes` refuses
naming the election and a preparer; `hoh_qualifying_person` and
`hoh_paid_over_half_cost_of_keeping_up_home` (`:1164-1200`), any `No` ⇒ `HohTestNotMet`; the
non-dependent qualifying child's name as a `Text` field live iff HoH and no dependent row is the
qualifying person; the five QSS `FormQuestion`s live iff `Qss` (the two-year window DERIVED from
`tax_year`); the EIC separated-spouse checkbox is NOT an R7 gate. **§7 row T8** (its kills). **T7's
output is your input:** the per-row flowchart verdict enum T7 exposes, `date_of_birth` required,
`full_time_student` / `permanently_and_totally_disabled` on the row. Build AS WRITTEN; the tree's
real names win; deviations recorded.

## Settled facts (controller-measured at `c3754e53`; re-measure the T7-dependent ones at dispatch)
- `crates/btctax-forms/forms/2025/f1040.map.toml` is 17 lines: capital-gains cells only (`line7a`,
  `da_yes`, `da_no`); the rest of the form sits in `field_census.rs`'s `UNCENSUSED` register
  (`:75-92`, `UNCENSUSED_ENTRIES: usize = 5`, with a per-(year, stem) count that may only shrink).
  The TY2025 grid prints rows (1)–(7) × four dependents plus the *more than four dependents* box
  (`design/forms/extract/f1040--2025.txt:38-52`); the label reader (`xtask` — find `labels` /
  `label-reader`) is how a map row is derived from the extract, never typed.
- `form1040_full.rs:390` writes the identity block (names, SSNs, address, the checkbox row, the
  dependents table) through `dependents_statement::DEPENDENTS_GRID_ROWS`; a deliberate fail-closed
  blank for the row boxes exists (search *deliberate*); `ctc_odc_line19` (`advisories.rs:857`) and
  `CtcOdcOmitted` stay.
- `FilingStatusArg::Hoh` / `Qss` (`crates/btctax-core/src/tax/types.rs:15`, `cli.rs:1176`) are
  offered with no test; `filing_status` liveness anchor `return_inputs.rs` (grep `filing_status ==`);
  T4b's `FilingStatusConfirmed` is live on an opened year and re-asks when the status changes.
- `FullReturnParams` (`tables.rs:453`) carries the per-year figures; the CTC/ODC per-child amounts
  for the line-19 forgo size come from the year's package (TY2024/25 present, TY2026 absent).

## What T8 delivers
1. **Row (7) computed** from T7's verdict enum + the DOB (under 17 at year end) + Step 3's TIN gates:
   `CreditColumn { Ctc, Odc, None }` per row, never a field; rows (5)(a)/(b) and (6) from the gates.
2. **The TY2025 dependents grid MAPPED** into `forms/2025/f1040.map.toml` — rows (1)–(7) × four
   dependents + the *more than four* box — each cell's AcroForm name read through the label reader
   against the extract, the `[census]` entries for the cells you map, and the `UNCENSUSED` register's
   `f1040` count falling by EXACTLY the cells mapped (the register kill).
3. **The emitter** fills rows (5)–(7) on TY2025+ from the computation (replacing the deliberate
   blank); TY2024's output byte-identical (its grid has no such rows); more than four dependents ⇒
   the box checked and the statement attached as the instructions say (`i1040gi--2025.txt`, *If more
   than four dependents*) — or refuse naming it if the statement is outside the build; say which.
4. **HoH**: `hoh_marital_basis` (the enum, a `Choice` skippable-shaped class-(A) question), the two
   tests, the name `Text` field, the `MarriedLivedApart` refusal (or the five conditions transcribed);
   **FR-67** the NRA-spouse election gate; **QSS**: the five questions with the derived window;
   `HohTestNotMet` / `QssTestUnanswered` / `QssTestNotMet` refusals with the exit *"choose another
   filing status"*; classifier rows; `LEAF_SOURCE`; the no-brick property covers every one.
5. **The line-19 forgo sized in the panel**: *"child tax credit not computed — n children with a
   credit box; up to $X each"* from the year's params; blank on a params-less year (`interview_state
   _with_params`).

## Kills (each seen red once)
The flowchart truth table INCLUDING the born-in-year row landing on CTC (T7's KAT extended with the
credit column). TY2024 emitter byte-identical (the existing golden). TY2025 fixture rows filled — a
fixture with two dependents (one CTC, one ODC) prints the six row-(5)/(6) boxes and the two row-(7)
boxes per dependent from the answers, read back from the filled PDF; `UNCENSUSED`'s `f1040` count
falls by exactly the mapped cells (plant: map one cell without a census entry → red; leave the
register count unchanged → red). `Single`/`Mfj` ask no HoH/QSS question; `Hoh` with
`hoh_marital_basis = None` or a test `None` refuses; `MarriedLivedApart` refuses naming *Married
persons who live apart*; `NraSpouseNoElection` refuses naming *Nonresident aliens*; the FR-67 gate's
`Yes` refuses naming §6013(g)/(h); `Hoh` with a `Some(false)` test refuses with the exit; `Qss` with a
`None` refuses `QssTestUnanswered`, a `Some(false)` refuses `QssTestNotMet` naming the test, all five
`Some(true)` computes at the joint rates; the window derived from `tax_year` (a TY2026 fixture names
2024/2025). The forgo size present with params, absent without. The no-brick test extends.

## Constraints
- `record_answer` stays the only writer; nothing asked that the form computes; every prompt cites
  its `i1040gi--2025.txt` lines.
- Nothing prints differently on TY2024; the R6 slice path unchanged.
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T8-implementation.md`: per numbered item what
landed, the mapped cells (AcroForm name → row/dependent), the register count old → new, the HoH/QSS
question table (prompt cite → refusal), every deviation, every kill with its red text, every pinned
number moved, suite lines per crate. Return only a 4-line summary plus the path.
