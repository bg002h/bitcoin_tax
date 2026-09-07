# Brief — interview build T9: real estate (R8) — a document, four gates, two refusals

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
ask what the NEXT SURFACE does with what the build wrote; a figure the tool holds must never reach a
line it does not compute (box 4 refuses — it may not sit beside a blank 8z with a note).

## The contract
`design/SPEC_interview.md` r2 **R8** (the Form 1098 document screen, top-level `form_1098:
Vec<Form1098>` whose LIVENESS is keyed on the itemize election — live iff `schedule_a.is_some()` —
with the three declarations `MortgageAllUsed` / `AmtQualifiedDwelling` / `MortgageWithinDebtLimit`
keyed on `schedule_a.is_some() && !form_1098.is_empty()`; the AGGREGATE, status-adjusted
§163(h)(3)(B) ceiling WARNING over Σ box 2 against the year's params, halved for MFS; the per-row
`other_borrower_paid_interest` gate refusing `SharedMortgageInterest`; **box 4 > 0 refuses
`MortgageInterestRefundNotComputed` naming Schedule 1 line 8z**; box 5 collected against the
reserved 8d; Schedule A **8b** as a repeating `mortgage_interest_not_on_1098` with the recipient's
name / TIN / address (empty TIN refuses — the $50-penalty clause); **8c** `points_not_on_1098`;
**8e = 8a + 8b + 8c**; the Form 8396 gate `claiming_mortgage_interest_credit` live iff any 1098 or
8b row, `Yes` refuses; the sale-of-a-main-home gates from `i1040sd--2025.txt:313-347` with the
8-branch table — exactly (Y, Y, Y, no 1099-S) is blank by decision, every other branch refuses
naming Pub. 523 and Form 8949 code H; rentals / royalties / K-1 stay census refusals), **§5.5**,
**§5.1**'s `form_1098` liveness sentence, **§7 row T9** (its kills), **R4** (the 1098 box captions
from T2's archive — TWO editions, Rev. January 2022 (TY2024) and Rev. April 2025 (TY2025/26), per
`revision_in_force`), **R2.2** (`line-coverage` on 8a/8b/8c/8e). Build AS WRITTEN; the tree's real
names win; deviations recorded.

## Settled facts (controller-measured at `ac192ed2`)
- `ScheduleAInputs.mortgage_interest_1098: Usd` (`return_inputs.rs`, "8a only") is what T9 removes;
  the three declarations' liveness today is `.is_some_and(|a| a.mortgage_interest_1098 > Usd::ZERO)`
  (`questions.rs` — grep `mortgage_interest_1098`); T3 made the `form_1098` census row non-live
  until T9 (`document_census.rs`, `row_is_live`; D16) — you flip it to `schedule_a.is_some()`.
- The 8b/8c cells exist unmapped in `crates/btctax-forms/forms/2024/f1040sa.map.toml` (`f1_17` /
  `f1_19` and `f1_18` / `f1_20`, around `:99-102`); the 8e chain prints `= 8a` today (`printed.rs`,
  grep `8e`); TY2025's Schedule A map exists too (check its cells).
- T2 archived `f1098--2022` and `f1098--2025` (+ `i1098--2025`, `i1098--2026`); the per-edition box
  census (`xtask box-census`) already has entries for both editions with `NotRead("T9 …")` reasons —
  decide every one. T5 established the section/`Field`/census-join pattern (`Collected(FieldId)`
  checked by `box-census`); follow it exactly.
- `FullReturnParams` (`tables.rs:453`) — add the §163(h)(3)(B) ceilings ($750,000 / $1,000,000, MFS
  half) for TY2024/25 with their cites (`i1040sca--2025.txt`, Pub. 936); TY2026 absent.
- T1's `LEAF_SOURCE` (`Source::Document(Form1098)` — extend `DocumentKind`) and `record_answer`;
  T3's classifier discipline; T4's import tier (the 1098 census invariants and the box-4 refusal
  are param-free → they fire at import); T4b's opener (a 1098 lender is a payer identity — extend
  `identities_of` and the seed with every box blank).

## What T9 delivers
1. **`Form1098 { lender, lender_tin, transcribed_on, box1_interest, box2_outstanding_principal,
   box3_origination_date: Option<Date>, box4_refund_overpaid_interest, box5_mortgage_insurance,
   box7_8_property_address, box10_other, other_borrower_paid_interest: Option<bool> }`** as a
   top-level repeating section, every box's caption verbatim as `help` from the edition in force
   for the row's year, every censused box decided (`Collected(FieldId)` / `RefuseIfNonzero` /
   `NotRead` with the pointer); `mortgage_interest_1098` REMOVED (8a = Σ(box 1 + box 6) — read the
   1098 for which box carries points and cite it); liveness as R8 states; the census row live iff
   `schedule_a.is_some()`.
2. **The ceiling warning** (aggregate Σ box 2, by box 3 vs 2017-12-16, MFS halved, from params;
   displayed beside `MortgageWithinDebtLimit`, which stays the filer's testimony); the box-4 refusal;
   the `SharedMortgageInterest` refusal; box 5 → the reserved 8d slot.
3. **Schedule A 8b rows** (`NonForm1098Interest { recipient_name, recipient_tin,
   recipient_address, amount }`, the TY2024 (and TY2025) map cells mapped, empty TIN refuses), **8c**
   `points_not_on_1098` with the instruction's *"generally deductible over the life of the loan"* in
   its help, **8e = 8a + 8b + 8c** in the printed chain; `line-coverage` productions for 8a/8b/8c/8e
   (`FilerRecords` for 8b/8c with their instruction lines).
4. **The Form 8396 gate** — `claiming_mortgage_interest_credit` on `ReturnInputs`, live iff any 1098
   or 8b row; `Yes` refuses `MortgageInterestCreditUnsupported`.
5. **`home_sale: HomeSale`** — `sold_main_home` always live; the three tests live iff `Yes`; the
   `s_1099` census row (exists, T3) joins the table: (Y, Y, Y, `s_1099 = Some(false)`) ⇒ blank by
   decision with the four answers on record; every other branch ⇒ `HomeSaleNotComputed` naming
   Pub. 523 and Form 8949 code H. No amount asked.
6. **Fixtures** (§5.7): the coverage fixture carries one 1098 row (box 4 = 0), one 8b row, 8c, the
   home sale on its blank branch; the TY2024 example fixtures gain the row that replaces the scalar
   (goldens regenerated, each moved line explained); the two-oracle sweep still reconciles
   (`e19200` absorbs 8b/8c per R13).

## Kills (each seen red once)
8e = 8a + 8b + 8c on a fixture with one of each; the sweep reconciles. The ceiling set: box 2 =
$900,000 with box 3 in 2019 warns, $700,000 silent; two rows at $500,000 warn while one does not; MFS
warns at $400,000 where Single does not; `MortgageWithinDebtLimit = None` still refuses.
`other_borrower_paid_interest = Some(true)` refuses `SharedMortgageInterest`. **Box 4 = $1 refuses
`MortgageInterestRefundNotComputed` with *Schedule 1 line 8z* in the message; box 4 = 0 does not.**
**A standard-deduction fixture with a $900,000 1098 asks none of the three declarations, has no live
1098 row and does not refuse; the same fixture with a `ScheduleAInputs` present asks them and
refuses on `MortgageWithinDebtLimit = Some(false)`.** The 8396 gate refuses on `Yes`. The 8-branch
home-sale table — exactly one blank, seven refusals naming Pub. 523. An 8b row with an empty
`recipient_tin` refuses. The census: a deleted 1098 box entry reds; a caption drift reds; the
TY2024 row verifies against the Rev. January 2022 edition and the TY2025 row against Rev. April 2025
(plant a caption only the other edition prints → red). The opener seeds a prior lender pre-named
with every box blank. `import` of a TOML with a box-4 row refuses at import naming line 8z and writes
nothing.

## Constraints
- `record_answer` stays the only writer; no amount is asked for a home sale; nothing prints on a
  standard-deduction return that did not print before.
- If context runs short: leave the tree compiling, fmt-clean and green, and say which numbered item
  is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T9-implementation.md`: per numbered item what
landed, the 1098 census decision table per edition, the ceiling figures with their cites, the
home-sale table as tested, every deviation, every kill with its red text, every pinned number moved,
suite lines per crate. Return only a 4-line summary plus the path.
