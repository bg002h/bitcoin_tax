# SPEC — P2-C: Form 8283 generation + Form 709 gift advisory (Phase-2, sub-project 3)

**Source baseline:** `origin/main` @ `f7159c5` (post B-M1).
**Goal:** Generate **IRS Form 8283** (Noncash Charitable Contributions) rows for a tax year from donation
data — Section A (deduction ≤ $5,000) vs Section B (> $5,000) — emitting every computable field and
honestly flagging the user-input gaps (donee, appraiser, FMV-method); plus a thin **Form 709** (gift tax)
over-annual-exclusion **advisory** (no donee identifier is modeled, so full 709 generation is deferred).
Standalone informational artifacts — do NOT feed engine B.

**SemVer:** additive `RemovalLeg.acquired_at` field + `TaxTable.gift_annual_exclusion` field + new CSV +
advisory ⇒ **MINOR** (pre-1.0). No tax-figure change.

## Legal grounding
- **Form 8283** (§170(f)(11)): required for noncash charitable contributions > $500. **Section A** =
  deduction ≤ $5,000 (+ publicly-traded securities); **Section B** = deduction > $5,000 (requires a
  qualified appraisal + appraiser signature; crypto has no readily-valued exception per CCA 202302012).
  The Section A/B split is driven by the **claimed deduction** — which is exactly P2-A's
  `Removal.claimed_deduction` (> `QUALIFIED_APPRAISAL_THRESHOLD` $5,000 → Section B), already the
  `QualifiedAppraisalNote` trigger.
- **Form 709 / §2503(b):** a gift tax return is required if gifts to any single donee exceed the annual
  exclusion ($18,000 TY2024, $19,000 TY2025 — **inflation-indexed** under §2503(b)(2)). Because no donee
  identifier is modeled, this app cannot compute per-donee totals → a thin advisory only.

## Current-state (recon @ f7159c5)
- Donation = `Removal{kind:Donation}`; `RemovalLeg{sat, basis, fmv_at_transfer, term, basis_source}`
  (`state.rs:149-156`); `Removal.claimed_deduction` (P2-A, `state.rs:168`) = exact §170(e) deduction.
  Present for 8283: description (via sat), date of contribution (`removed_at`), Σbasis, Σfmv,
  claimed_deduction, Section A/B (from claimed_deduction > $5k). **GAP — date acquired:** `RemovalLeg` has
  no `acquired_at` (like `DisposalLeg` before P2-B); `make_removal_legs` (`fold.rs:219-255`) uses
  `gain_hp_start` for `term` (`fold.rs:251`) but doesn't store it. **GAP (user-input) — donee, appraiser,
  FMV-method:** entirely unmodeled (`OutflowClass::Donate{appraisal_required:bool}` only). "How acquired"
  is derivable from `basis_source` for the common cases (ExchangeProvided/ComputedFromCost→purchased;
  FmvAtIncome→income; GiftCarryover/GiftFmvFallback→gift), ambiguous for CarriedFromTransfer /
  SafeHarborAllocated / ReconstructedPerWallet (origin lost) → needs_review.
- Gift = `Removal{kind:Gift}`; value = Σfmv_at_transfer; `claimed_deduction:None`. **No donee identifier,
  no annual-exclusion constant, no lifetime-exemption tracking** anywhere.
- `TaxTable` (`crates/btctax-adapters` bundled, TY2025 only) holds year-indexed figures; the gift annual
  exclusion is inflation-indexed → belongs here (NOT a `tables.rs` fixed constant).
- Output pattern: `write_csv_exports` year-scoped block (form8949.csv/schedule_d.csv) + `render_report`
  text sections (P2-B). Form 8283 does NOT feed `compute_tax_year` (Schedule-A-adjacent, like §170).

## Design

### D1 — `RemovalLeg.acquired_at` (prerequisite; mirror of P2-B's DisposalLeg change)
Add `pub acquired_at: TaxDate` to `RemovalLeg`, set in `make_removal_legs` from the SAME HP-start fed to
`term_for` for that leg (`gain_hp_start` — removals recognize no loss, so there is no loss-zone HP-start
divergence like disposals; confirm at impl and set from the exact `term_for` argument so acquired_at can
never contradict `term`). Add an `acquired_at` column to `removals.csv`. Projection struct (no serde) →
additive, migration-free; update `RemovalLeg` literals. **[R0-M2]** For a gift-received-then-donated lot,
`gain_hp_start` is the tacked donor acquisition date (§1223) — this is the correct 8283 "date acquired"
because it matches the leg's holding-period `term`; document this in the field/CSV doc.

### D2 — Form 8283 generation (`form_8283` + `form8283.csv`)
Add `Form8283Row` + `form_8283(state, year) -> Vec<Form8283Row>` to `forms.rs` (pure over
`state.removals` where `kind==Donation` && `removed_at.year()==year`). Granularity: **one row per
donation LEG** (like form8949; each leg has its own description/basis/fmv/how_acquired/acquired_at);
the per-donation `claimed_deduction` + `section` appear on the FIRST leg row only (blank on subsequent —
the P2-A first-leg convention, so a naive CSV SUM of the deduction column doesn't double-count). Columns:
`section (A/B), description, how_acquired, date_acquired, date_contributed, cost_basis, fmv,
claimed_deduction (first leg), fmv_method, donee, appraiser, needs_review`. Where:
- **section** = B if the donation's `claimed_deduction > QUALIFIED_APPRAISAL_THRESHOLD` else A.
- **how_acquired** = derived from `basis_source` (purchased/income/gift); ambiguous sources → "review".
- **fmv_method / donee / appraiser** = EMPTY (unmodeled user-input) + `needs_review=true` set on the
  row; for a Section B donation, `needs_review` is always true (appraiser required + unmodeled).
- **how_acquired mapping [R0-N2]:** ExchangeProvided/ComputedFromCost → "Purchased"; GiftCarryover/
  GiftFmvFallback → "Gift"; FmvAtIncome → **"Other"** ("income" is NOT a literal Form 8283 how-acquired
  category); CarriedFromTransfer/SafeHarborAllocated/ReconstructedPerWallet → "Review" (origin lost).
- **[R0-I1] Section A/B aggregation caveat (BLOCKING disclosure — do NOT implement aggregation).** The
  A/B split is computed PER-DONATION off that donation's `claimed_deduction`, but §170(f)(11)(F)
  AGGREGATES similar items across the tax year: e.g. two $3,000 crypto donations = $6,000 aggregate →
  Section B + a qualified appraisal is REQUIRED (CCA 202302012 confirms this applies to crypto), yet each
  row would show Section A and NO `QualifiedAppraisalNote` fires (it triggers only on a single donation
  > $5k). Under-classifying → skipping the mandatory appraisal → the whole deduction can be disallowed.
  Emit a STANDING caveat with the form8283 output (CSV header comment + the text/advisory path): "Section
  A/B is per-donation; the $5,000 appraisal threshold AGGREGATES similar crypto items donated across the
  year — rows shown as Section A may require Section B + a qualified appraisal; verify your AGGREGATE
  similar-item totals." (Year-aggregation is a FOLLOWUP — Task 4.)
- **[R0-M1] $500 form-filing floor:** Form 8283 is required only when total noncash contributions for the
  year exceed $500. Emit the rows regardless (informational), but add a note when the year's total
  donation deduction ≤ $500 that Form 8283 is not required at that level.
- Export `form8283.csv` (0o600, stable snake_case) in the `if let Some(year)` block of
  `write_csv_exports`, alongside form8949.csv. Deterministic order (removed_at, event, lot_id).

### D3 — Form 709 gift over-annual-exclusion advisory (thin)
Add `gift_annual_exclusion: Usd` to `TaxTable` (**[R0-M4]** also update the `synthetic_table` test helper
~tables.rs:187 + any other `TaxTable` literal; cite source **Rev. Proc. 2024-40 §2.43** for the $19,000
TY2025 value — NOT §2.01/§2.03) + populate in `BundledTaxTables` (TY2025 = $19,000; TY2024 = $18,000 only
if a TY2024 table is added — else the advisory is skipped for years with no bundled table). **[R0-M5] Do
NOT silently skip:** when the year has gifts but no bundled table (gift_annual_exclusion unavailable),
emit a short note ("gift annual-exclusion table unavailable for {year}; Form 709 exposure not evaluated")
rather than nothing. Add `render_gift_advisory(state, year, tables) -> Option<String>`: sum
`Σ fmv_at_transfer` over `Removal{Gift}` for the year; if the total exceeds the year's
`gift_annual_exclusion`, emit an advisory: "Total gifts in {year}: ${total}; the §2503(b) annual
exclusion is ${excl} per donee (TY{year}). If any single donee received more than ${excl}, Form 709 may
be required. NOTE: donee identity is not modeled — verify per-donee totals; this is a total-exposure
signal, not a per-donee determination." **[R0-m6 — resolve the M5 contradiction in the EMIT-the-note
direction]** the OMIT/None cases are ONLY: (a) no gifts in the year, or (b) gifts present but total ≤ the
exclusion. When gifts ARE present but the year has **no bundled table** (gift_annual_exclusion
unavailable), return `Some(note)` = "gift annual-exclusion table unavailable for {year}; Form 709
exposure not evaluated — {total} in gifts recorded" (do NOT return None — M5). Surface it in the
tax-report path alongside `render_schedule_d`.

### Decisions
- **Donee identifier + full Form 709 generation: DEFERRED.** Adding a donee to `OutflowClass::Donate` /
  `Op::GiftOut` (+ per-donee-per-year exclusion + lifetime-exemption tracking) is an event-schema change
  and its own sub-project — out of scope. P2-C emits Form 8283 with donee as a needs_review blank + a
  thin 709 total-exposure advisory. Honest-placeholder pattern (like P2-B's box_needs_review).
- **Section B appraiser-info struct: DEFERRED.** Section B rows emit with `needs_review=true` +
  blank appraiser columns; a full appraiser struct is a follow-up.
- **Form 8283 is standalone** — does NOT feed engine B (Schedule-A-adjacent, like §170).
- **Section A/B driver = the engine-computed `claimed_deduction`** (> $5k → B), NOT the user
  `appraisal_required` bool (kept decoupled, as in P2-A).

## Plan (TDD)

### Task 1 — `RemovalLeg.acquired_at` + removals.csv column
- **Files:** `crates/btctax-core/src/state.rs`, `crates/btctax-core/src/project/fold.rs` (`make_removal_legs`), `crates/btctax-cli/src/render.rs` (removals.csv).
- Set `acquired_at` from the same HP-start `term_for` uses for the leg. KATs: an ordinary donation leg's `acquired_at` == the lot's HP-start and matches its `term`; a gift-received-then-donated lot's `acquired_at` is consistent with its `term`; removals.csv shows the column. No FMV/basis/term math change (existing removal KATs unchanged).

### Task 2 — `form_8283` + `form8283.csv`
- **Files:** `crates/btctax-core/src/forms.rs` (Form8283Row + builder), `crates/btctax-cli/src/render.rs` (`write_form8283_csv`).
- KATs: Section A donation (deduction ≤$5k) → section A; Section B (>$5k) → section B + `needs_review=true`; how_acquired mapping (purchased/income/gift + ambiguous→review); claimed_deduction on first leg only (multi-leg → no SUM double-count); donee/appraiser/fmv_method blank + needs_review; date_acquired/date_contributed correct; how_acquired mapping incl. FmvAtIncome→"Other" and ambiguous→"Review"; year-filter (prior-year excluded); deterministic order; a gift (kind=Gift) produces NO 8283 row; **[R0-I1] the aggregation caveat is present in the output**; **[R0-M1] the ≤$500-total note** appears when the year's donation total is ≤ $500.

### Task 3 — `gift_annual_exclusion` + 709 advisory
- **Files:** `crates/btctax-core/src/tax/tables.rs` (TaxTable field + `synthetic_table` ~187 + cite §2.43), `crates/btctax-adapters/src/...` (BundledTaxTables TY2025 $19,000), `crates/btctax-cli/src/render.rs` (`render_gift_advisory`), and the wiring sites **[R0-M3]** `crates/btctax-cli/src/cmd/tax.rs::report_tax_year` (has state+tables+year) + `crates/btctax-cli/src/main.rs` (~381-387, the tax-year report dispatch).
- KATs: gifts over the TY2025 $19,000 exclusion → advisory emitted with the total + the donee-not-modeled caveat; gifts under → None; no gifts → None; **[R0-m6] a year WITH gifts but no bundled table → `Some(note)` = "exclusion table unavailable … Form 709 exposure not evaluated" (NOT None, no panic)**; the $19,000 value asserted against BundledTaxTables. Independently confirm $19,000 = TY2025 §2503(b) annual exclusion.

### Task 4 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: `acquired_at` matches `term` (no contradiction); Form 8283 Section A/B ties to
  `claimed_deduction`/`QualifiedAppraisalNote`; deduction column first-leg-only (no double-count); donee/
  appraiser honestly flagged (never fabricated); 709 advisory is total-exposure-only with the donee
  caveat; NO engine-B / capital-gains / FMV / basis math change; CSV 0o600 + stable columns; year-scoped;
  determinism; no float; privacy.
- FOLLOWUPS: **§170(f)(11)(F) similar-item YEAR-AGGREGATION for the Section A/B split** (currently
  per-donation + disclosed via the I1 caveat — the aggregate-of-small-donations case needs computing);
  donee identifier on Donate/GiftOut → full Form 709 (per-donee exclusion + lifetime exemption) + Form
  8283 donee/FMV-method fields; Section B appraiser-info struct; gift-exclusion tables for TY2024/2026+
  (inflation-indexed, year-dependent); how_acquired origin-loss for CarriedFromTransfer/SafeHarborAllocated
  lots; future-interest / non-citizen-spouse gift cases in the 709 advisory (R0-N3).

## Out of scope
- Donee identifier + full Form 709 generation (per-donee/lifetime exemption/split gifts/Schedule B);
  Section B appraiser-info capture; Form 8283 donee-name/address + FMV-method fields (user-input,
  flagged); filled-PDF; feeding any of this into engine B; 2026/2027 income-tax tables.
