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
use crate::event::DisposeKind;
use crate::identity::WalletId;
use crate::state::{LedgerState, Term};
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
