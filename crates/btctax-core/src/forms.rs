//! IRS **Form 8949** (per-disposition rows) + **Schedule D** (aggregated part totals) generation
//! (Phase-2 sub-project 2). Both are **pure, year-scoped projections over `state.disposals`** — no
//! tax math, no rounding, no float (NFR5: the BTC-amount description is EXACT `Decimal`). Federal-only.
//!
//! **Scope boundary (D3):** these are the RAW, pre-netting Form 8949 rows / Schedule D part totals.
//! The §1222 ST/LT netting + §1211/§1212 loss limit + carryforward live in engine B
//! (`compute_tax_year` / `report --tax-year`), NOT here. The reconciliation KAT ties the raw part
//! gains to B's within-character `st_net`/`lt_net` (before B's carryforward/other-LT netting) so the
//! forms and the tax engine can never silently diverge.
use crate::conventions::{Sat, TaxDate, Usd, SATS_PER_BTC};
use crate::event::{BasisSource, DisposeKind};
use crate::identity::{EventId, LotId, WalletId};
use crate::state::{LedgerState, RemovalKind, RemovalLeg, Term};
use crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD;
use rust_decimal::Decimal;

/// Which Form 8949 part / holding-period a row belongs to. **Part I = short-term** (held ≤ 1 yr);
/// **Part II = long-term** (held > 1 yr). Derived 1:1 from the leg's `Term`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form8949Part {
    /// Part I — short-term.
    ShortTerm,
    /// Part II — long-term.
    LongTerm,
}

/// The Form 8949 "box" (the reporting category). We **only ever** emit the conservative
/// "**not reported on a 1099-B**" default — **C** for short-term, **F** for long-term — because the
/// model carries no 1099-B / basis-reported signal (D4). We NEVER auto-assign A/B (ST) or D/E (LT):
/// asserting a 1099-B was issued / basis reported would fabricate an unsubstantiated box. The
/// `box_needs_review` flag surfaces exchange dispositions that MAY carry a 1099-B/1099-DA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form8949Box {
    /// Box **C** — short-term, not reported on a 1099-B.
    C,
    /// Box **F** — long-term, not reported on a 1099-B.
    F,
}

/// One Form 8949 row = one `DisposalLeg` disposed in the tax year. A pure projection of the leg;
/// no gain/basis/term math is performed here (all of it is already on the leg from the fold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form8949Row {
    /// Part I (ST) or Part II (LT), from `leg.term`.
    pub part: Form8949Part,
    /// The conservative C (ST) / F (LT) "not reported on a 1099-B" default (D4).
    pub box_: Form8949Box,
    /// `true` iff the disposing wallet is an **Exchange** (`matches!(leg.wallet, Exchange { .. })`)
    /// — such a disposition MAY have been reported on a 1099-B/1099-DA (2025+ broker reporting), so
    /// the C/F default should be reviewed and reclassified to A/B (ST) or D/E (LT) if a 1099-B was
    /// issued. Direct match on `leg.wallet` (D4/[R0-M2]) — never the private `optimize.rs::is_broker`.
    pub box_needs_review: bool,
    /// Column (a): the BTC amount, 8dp + `" BTC"` (e.g. `"0.53000000 BTC"`). Computed as EXACT
    /// `Decimal` (`Decimal::from(sat) / SATS_PER_BTC`) — NEVER `sat as f64 / 1e8` [R0-M5].
    pub description: String,
    /// Column (b): date acquired = the leg's zone-aware holding-period start (`leg.acquired_at`).
    pub date_acquired: TaxDate,
    /// Column (c): date sold = the disposal's `disposed_at`.
    pub date_sold: TaxDate,
    /// Column (d): proceeds (allocated net proceeds, from the leg).
    pub proceeds: Usd,
    /// Column (e): cost basis (tax-reported basis, from the leg).
    pub cost_basis: Usd,
    /// Column (f): adjustment code — always empty. No §1091 (wash sale is N/A to crypto) and no
    /// other adjustments are modelled.
    pub adjustment_code: String,
    /// Column (g): adjustment amount — always `0`. See `adjustment_code`.
    pub adjustment_amount: Usd,
    /// Column (h): gain/loss (from the leg; for a NoGainNoLoss dual-basis gift leg this is `0`).
    pub gain: Usd,
    /// The disposing wallet (CSV `wallet` column + the `box_needs_review` source).
    pub wallet: WalletId,
    /// The disposition kind (Sell/Spend), for the CSV `disposition_kind` column.
    pub disposition_kind: DisposeKind,
}

/// Format a satoshi quantity as its exact BTC amount, 8dp + `" BTC"` (e.g. `"0.53000000 BTC"`).
///
/// **Exact `Decimal` (NFR5 / [R0-M5]):** `Decimal::from(sat) / SATS_PER_BTC`. `sat` is an integer
/// count and `SATS_PER_BTC` is `100_000_000`, so the quotient has at most 8 decimal places — the 8dp
/// format is lossless (no rounding, ever). A float (`sat as f64 / 1e8`) would be non-exact and is
/// forbidden.
fn btc_amount_description(sat: Sat) -> String {
    let btc = Decimal::from(sat) / Decimal::from(SATS_PER_BTC);
    format!("{btc:.8} BTC")
}

/// Build the Form 8949 rows for tax year `year`: **one row per `DisposalLeg`** whose
/// `Disposal.disposed_at.year() == year`. Pure over `state.disposals`.
///
/// Rows for ALL legs in the year are emitted, INCLUDING NoGainNoLoss dual-basis gift-zone legs — the
/// fold already set `basis == proceeds` for that zone, so the row is internally consistent
/// (proceeds, basis, gain = 0) with no special 8949 adjustment code needed [R0-M1].
///
/// **Deterministic ordering (NFR4):** rows are sorted by `disposed_at`, then the disposal's `event`
/// id, then the leg's `lot_id` — a total order over the (event, lot) space.
pub fn form_8949(state: &LedgerState, year: i32) -> Vec<Form8949Row> {
    // Key each row by (disposed_at, event, lot_id) so ordering is a deterministic total order
    // independent of the projection's disposal/leg iteration order.
    let mut keyed: Vec<(
        TaxDate,
        &crate::identity::EventId,
        &crate::identity::LotId,
        Form8949Row,
    )> = Vec::new();
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in &d.legs {
            let (part, box_) = match leg.term {
                Term::ShortTerm => (Form8949Part::ShortTerm, Form8949Box::C),
                Term::LongTerm => (Form8949Part::LongTerm, Form8949Box::F),
            };
            let row = Form8949Row {
                part,
                box_,
                box_needs_review: matches!(leg.wallet, WalletId::Exchange { .. }),
                description: btc_amount_description(leg.sat),
                date_acquired: leg.acquired_at,
                date_sold: d.disposed_at,
                proceeds: leg.proceeds,
                cost_basis: leg.basis,
                adjustment_code: String::new(),
                adjustment_amount: Usd::ZERO,
                gain: leg.gain,
                wallet: leg.wallet.clone(),
                disposition_kind: d.kind,
            };
            keyed.push((d.disposed_at, &d.event, &leg.lot_id, row));
        }
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)).then(a.2.cmp(b.2)));
    keyed.into_iter().map(|(_, _, _, r)| r).collect()
}

/// One Schedule D part total (Part I / ST or Part II / LT): the RAW, pre-netting sums over the
/// year's Form 8949 rows (equivalently, the year's disposal legs of that term).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScheduleDPart {
    /// Σ proceeds over the part's legs.
    pub proceeds: Usd,
    /// Σ cost basis over the part's legs.
    pub cost_basis: Usd,
    /// Σ gain/loss over the part's legs (signed).
    pub gain: Usd,
}

/// Schedule D part totals for a tax year: **Part I (ST)** + **Part II (LT)**.
///
/// These are the RAW pre-netting totals. §1222/§1211/§1212 netting + carryforward is applied by
/// engine B (`compute_tax_year` / `report --tax-year`), NOT here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScheduleDTotals {
    /// Part I — short-term.
    pub st: ScheduleDPart,
    /// Part II — long-term.
    pub lt: ScheduleDPart,
}

/// Aggregate the year's disposal legs into Schedule D part totals (Part I ST + Part II LT), summing
/// proceeds/basis/gain within each character. Pure over `state.disposals`; year-scoped by
/// `Disposal.disposed_at.year() == year`. An empty year yields all-zero totals.
///
/// The ST/LT `gain` totals reconcile with engine B's within-character `st_net`/`lt_net` on an
/// all-gains fixture with zero carryforward-in + zero other-net-capital-gain (the R0-M3
/// reconciliation KAT) — `schedule_d` and `compute_tax_year` are separate functions reading the same
/// `state.disposals`, so the equality is a genuine cross-check, not a tautology.
pub fn schedule_d(state: &LedgerState, year: i32) -> ScheduleDTotals {
    let mut totals = ScheduleDTotals::default();
    for d in state
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == year)
    {
        for leg in &d.legs {
            let part = match leg.term {
                Term::ShortTerm => &mut totals.st,
                Term::LongTerm => &mut totals.lt,
            };
            part.proceeds += leg.proceeds;
            part.cost_basis += leg.basis;
            part.gain += leg.gain;
        }
    }
    totals
}

// ── Sub-project C (P2-C): IRS Form 8283 (Noncash Charitable Contributions) ───────────────────────
//
// Pure, year-scoped projection over `state.removals` where `kind == Donation`. No tax math is done
// here (the §170(e) `claimed_deduction` was computed by the fold and lives on the `Removal`). Like
// Form 8949 this is STANDALONE — it does NOT feed `compute_tax_year` / engine B (Schedule-A-adjacent,
// §170). Federal-only. No float (NFR5: the BTC-amount description is EXACT `Decimal`).

/// The Form 8283 part a donation is reported in, driven by the **§170(f)(11)(F) year-aggregate**
/// claimed deduction over ALL BTC donations in the tax year (all BTC is "similar property"):
/// - **Section A** — year-aggregate ≤ $5,000 (§170(f)(11)(C): "more than $5,000" is the threshold;
///   exactly $5,000 → Section A).
/// - **Section B** — year-aggregate > $5,000: a **qualified appraisal + appraiser signature** is
///   required (CCA 202302012 confirms the readily-valued exception does NOT apply to crypto).
///
/// The section is **UNIFORM** across all donations in the year (all BTC is one similar-property
/// class). This replaces the prior per-donation threshold test (§170(f)(11)(F) aggregates similar
/// items across the year; a per-donation test under-triggers Section B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form8283Section {
    /// Section A — year-aggregate ≤ $5,000.
    A,
    /// Section B — year-aggregate > $5,000 (qualified appraisal required).
    B,
}

/// The Form 8283 "How acquired by donor" category, derived from the leg's `BasisSource` [R0-N2]:
/// - `ExchangeProvided` / `ComputedFromCost` → **Purchased**
/// - `GiftCarryover` / `GiftFmvFallback` → **Gift**
/// - `FmvAtIncome` → **Other** ("income" is NOT a literal Form 8283 how-acquired category)
/// - `CarriedFromTransfer` / `SafeHarborAllocated` / `ReconstructedPerWallet` → **Review** (origin
///   lost — the acquisition provenance cannot be soundly asserted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form8283HowAcquired {
    /// Purchased (basis from an exchange record or a computed cost).
    Purchased,
    /// Gift (a received-gift lot with carryover / FMV-fallback basis).
    Gift,
    /// Other (income-recognized basis — mining/staking/airdrops/rewards).
    Other,
    /// Review — the acquisition origin was lost (transferred/safe-harbor/reconstructed basis).
    Review,
}

/// Map a leg's `BasisSource` to its Form 8283 "how acquired" category [R0-N2].
fn how_acquired_from(bs: BasisSource) -> Form8283HowAcquired {
    use Form8283HowAcquired as H;
    match bs {
        BasisSource::ExchangeProvided | BasisSource::ComputedFromCost => H::Purchased,
        BasisSource::GiftCarryover | BasisSource::GiftFmvFallback => H::Gift,
        BasisSource::FmvAtIncome => H::Other,
        BasisSource::CarriedFromTransfer
        | BasisSource::SafeHarborAllocated
        | BasisSource::ReconstructedPerWallet => H::Review,
    }
}

/// One Form 8283 row = one `RemovalLeg` of a `Donation` contributed in the tax year. A pure
/// projection of the leg; no gain/basis/deduction math is done here.
///
/// **First-leg convention (no CSV SUM double-count):** the per-DONATION `section`,
/// `claimed_deduction`, and `fmv_method` appear on the FIRST leg row only; subsequent leg rows carry
/// `None`/`""` — so a naive SUM over the deduction column equals the correct per-donation total
/// (mirrors P2-A's removals.csv).
///
/// **Unmodeled user-input (honest gaps, never fabricated):** `donee` and `appraiser` are always EMPTY
/// (not modeled), so `needs_review` is ALWAYS `true`. `fmv_method` is derived from the section
/// (Section B → `"qualified appraisal"`; Section A → `""`) — no `FmvStatus` dependency (RemovalLeg
/// carries no FMV provenance). For a Section B donation the qualified-appraiser signature is
/// additionally mandatory (not modeled until Chunk 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form8283Row {
    /// Section A/B — on the FIRST leg row only (`None` on subsequent legs). Driven by the donation's
    /// `claimed_deduction` (> $5,000 → B).
    pub section: Option<Form8283Section>,
    /// Column: description — the EXACT BTC amount, 8dp + `" BTC"` (`btc_amount_description`; NFR5).
    pub description: String,
    /// Column: how the donor acquired the property, from the leg's `BasisSource` [R0-N2].
    pub how_acquired: Form8283HowAcquired,
    /// Column: date acquired = the leg's holding-period start (`leg.acquired_at`; §1223 tacked donor
    /// date for received gifts). Consistent with the leg's `term` by construction (Task 1).
    pub date_acquired: TaxDate,
    /// Column: date contributed = the donation's `removed_at`.
    pub date_contributed: TaxDate,
    /// Column: donor's cost basis (from the leg).
    pub cost_basis: Usd,
    /// Column: fair market value at the contribution (from the leg).
    pub fmv: Usd,
    /// Column: the per-DONATION §170(e) claimed deduction — on the FIRST leg row only (`None` on
    /// subsequent legs, so a CSV SUM does not double-count).
    pub claimed_deduction: Option<Usd>,
    /// Column: FMV determination method — on the carrier (first-leg) row: `"qualified appraisal"`
    /// for Section B (a qualified appraisal IS the required FMV-determination method for Section B),
    /// or `""` for Section A (FMV method is not modeled; honest gap, never fabricated). Empty on
    /// subsequent (non-carrier) leg rows. No `FmvStatus` dependency — derived from the section only.
    pub fmv_method: String,
    /// Column: donee organization — EMPTY (unmodeled user-input; a donee identifier is deferred).
    pub donee: String,
    /// Column: appraiser — EMPTY (unmodeled user-input; the Section B appraiser struct is deferred).
    pub appraiser: String,
    /// ALWAYS `true`: `fmv_method`/`donee`/`appraiser` are unmodeled, so every row needs review
    /// (and a Section B row additionally requires a qualified appraiser signature).
    pub needs_review: bool,
}

/// Build the Form 8283 rows for tax year `year`: **one row per `RemovalLeg`** of a `Donation` whose
/// `Removal.removed_at.year() == year`. Pure over `state.removals`. Gifts (`kind == Gift`) produce
/// NO rows (a gift is not a charitable contribution; `claimed_deduction` is `None` and they are NOT
/// §170 — Gifts must NOT enter the donation aggregate). An empty year yields an empty vec.
///
/// **Section A/B (D1 — §170(f)(11)(F) year-aggregate):** the year's total `claimed_deduction` over
/// ALL Donation removals determines the section UNIFORMLY (all BTC is "similar property"). The per-
/// donation threshold test is replaced by this year-aggregate test: if `Σ claimed_deduction > $5,000`
/// → every carrier row is Section B; otherwise Section A. The `$5,000` threshold is strict `>`
/// (§170(f)(11)(C): "more than $5,000"; exactly $5,000 → Section A).
///
/// **`fmv_method` (D3 — honest, section-derived):** Section B → `"qualified appraisal"` (a qualified
/// appraisal IS required and IS the FMV-determination method for Section B); Section A → `""` (FMV
/// method is not modeled for Section A; honest gap, never fabricated). `RemovalLeg` carries no FMV
/// provenance, so `fmv_method` cannot be sourced from price status without an event-schema change
/// (out of scope for Chunk 1).
///
/// **First-leg convention:** `section`, `claimed_deduction`, and `fmv_method` are emitted on the
/// FIRST leg row only (carrier = smallest `lot_id`), so a naive CSV SUM of the deduction column does
/// not double-count. Subsequent legs carry `None`/`""` for these fields.
///
/// **Deterministic ordering (NFR4):** rows are sorted by `removed_at`, then the donation's `event`
/// id, then the leg's `lot_id` — a total order over the (event, lot) space (mirrors `form_8949`).
pub fn form_8283(state: &LedgerState, year: i32) -> Vec<Form8283Row> {
    // D1: §170(f)(11)(F) year-aggregate — sum claimed_deduction over ALL Donation removals in year.
    // Gifts have claimed_deduction == None and are NOT §170 — they must NOT enter this aggregate.
    let year_agg_deduction: Usd = state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Donation && r.removed_at.year() == year)
        .filter_map(|r| r.claimed_deduction)
        .sum();
    // §170(f)(11)(C): "more than $5,000" — strict `>` (exactly $5,000 → Section A).
    // The section is UNIFORM across the year: all BTC is "similar property", one aggregate class.
    let section = if year_agg_deduction > QUALIFIED_APPRAISAL_THRESHOLD {
        Form8283Section::B
    } else {
        Form8283Section::A
    };
    // D3: honest fmv_method — derived from the section only (no FMV provenance on RemovalLeg;
    // no FmvStatus dependency; no fabrication). Carrier row only; empty on subsequent legs.
    let carrier_fmv_method = match section {
        Form8283Section::B => "qualified appraisal".to_string(),
        Form8283Section::A => String::new(),
    };

    let mut keyed: Vec<(TaxDate, &EventId, &LotId, Form8283Row)> = Vec::new();
    for r in state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Donation && r.removed_at.year() == year)
    {
        // The FIRST leg (smallest lot_id; `min_by` returns the first minimum, so it is unique even
        // if two legs shared a lot_id) alone carries the per-donation section + claimed_deduction
        // + fmv_method (the first-leg / carrier-row convention).
        let carrier_idx: Option<usize> = r
            .legs
            .iter()
            .enumerate()
            .min_by(|(_, a): &(usize, &RemovalLeg), (_, b)| a.lot_id.cmp(&b.lot_id))
            .map(|(i, _)| i);
        for (i, leg) in r.legs.iter().enumerate() {
            let is_first = Some(i) == carrier_idx;
            let row = Form8283Row {
                section: is_first.then_some(section),
                description: btc_amount_description(leg.sat),
                how_acquired: how_acquired_from(leg.basis_source),
                date_acquired: leg.acquired_at,
                date_contributed: r.removed_at,
                cost_basis: leg.basis,
                fmv: leg.fmv_at_transfer,
                claimed_deduction: if is_first { r.claimed_deduction } else { None },
                fmv_method: if is_first {
                    carrier_fmv_method.clone()
                } else {
                    String::new()
                },
                donee: String::new(),
                appraiser: String::new(),
                needs_review: true,
            };
            keyed.push((r.removed_at, &r.event, &leg.lot_id, row));
        }
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)).then(a.2.cmp(b.2)));
    keyed.into_iter().map(|(_, _, _, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_is_exact_decimal_8dp() {
        // 0.53 BTC, exact — must be "0.53000000 BTC" (NOT a float artifact).
        assert_eq!(btc_amount_description(53_000_000), "0.53000000 BTC");
        // one satoshi → smallest representable unit at 8dp.
        assert_eq!(btc_amount_description(1), "0.00000001 BTC");
        // whole BTC.
        assert_eq!(btc_amount_description(SATS_PER_BTC), "1.00000000 BTC");
        // an amount a binary float cannot represent exactly is still exact here.
        assert_eq!(btc_amount_description(12_345_678), "0.12345678 BTC");
    }
}
