//! Approach-B / BG-D3: the `filed_basis` compute for promoting a `$0` conservative-filing tranche to a
//! filed `>$0` basis FLOOR, gated hard on `Coverage::Full` (a `Partial`-covered window min can EXCEED the
//! true window min — `conservative::window_reference`'s doc — so filing on it could UNDERSTATE the true
//! floor; refused rather than silently downgraded). Also defines `PromoteEntry`/`PromoteSet` — the ★
//! shared decomposition-key types every leg-builder task (T4/T5/T6) consumes, given ONE owner here (arch
//! r1 I-1) rather than each leg builder inventing its own shape.

use crate::conservative::{window_reference, Coverage};
use crate::conventions::{round_cents, Sat, TaxDate, Usd, SATS_PER_BTC};
use crate::identity::EventId;
use crate::price::PriceProvider;

/// BG-D3 refusal: `filed_basis_for` cannot produce a trustworthy promotion floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromoteRefusal {
    /// No day in `[window_start, window_end]` has a bundled price — never fabricate a floor over a
    /// total data gap (mirrors `window_reference`'s `None`, D-7).
    NoCoverage,
    /// Some day in the window has no bundled close, so the covered-part min is not provably the TRUE
    /// window min (it can only be `>=` it) — a promoted floor must never rest on that (tax r1 N-3).
    PartialCoverage,
}

/// BG-D3 computed promotion floor for one tranche: the WHOLE-tranche `filed_basis` (USD, cents),
/// scaled from the window's `Coverage::Full` min daily close — never a bare per-BTC price (the exact
/// bug `filed_basis_is_whole_tranche_scaled` kills). `coverage` is always `Coverage::Full` here (`Partial`
/// and no-coverage windows are refused before a `ComputedFloor` is ever built); it rides along so the
/// caller can snapshot it verbatim onto the filed `PromoteTranche.coverage` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputedFloor {
    pub filed_basis: Usd,
    pub coverage: Coverage,
}

/// BG-D3: compute the whole-tranche `filed_basis` floor for `sat` sats over `[window_start,
/// window_end]`. Delegates entirely to `window_reference` (conservative.rs) for the min daily close +
/// coverage caveat: no covered day → `Err(NoCoverage)`; a gap in the window → `Err(PartialCoverage)` —
/// only a `Coverage::Full` window yields a floor, scaled `round_cents(min * sat / SATS_PER_BTC)`, the
/// SAME whole-lot scaling `overpayment_delta_one` uses (conservative.rs:309) — `min` is a PRICE (USD per
/// WHOLE BTC), not a whole-lot basis, so a bare `min` would file a per-BTC price as the tranche's basis.
pub fn filed_basis_for(
    prices: &dyn PriceProvider,
    sat: Sat,
    window_start: TaxDate,
    window_end: TaxDate,
) -> Result<ComputedFloor, PromoteRefusal> {
    match window_reference(prices, window_start, window_end) {
        None => Err(PromoteRefusal::NoCoverage),
        Some(wr) if wr.coverage == Coverage::Partial => Err(PromoteRefusal::PartialCoverage),
        Some(wr) => Ok(ComputedFloor {
            filed_basis: round_cents(wr.min * Usd::from(sat) / Usd::from(SATS_PER_BTC)),
            coverage: Coverage::Full,
        }),
    }
}

/// The BG-D4/D11 decomposition key (★ the shared type every leg-builder task (T4/T5/T6) consumes;
/// defined HERE so it has one owner, arch r1 I-1): the promoted whole-tranche `filed_basis` alongside its
/// `tranche_sat` DENOMINATOR (the tranche's total sat at promotion time) — both fields are needed at the
/// leg builders to pro-rate a partial disposal's share of the promoted basis. Built by `resolve` (T3) and
/// threaded to the fold (T4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromoteEntry {
    pub filed_basis: Usd,
    pub tranche_sat: Sat,
}

/// Keyed by the target `DeclareTranche` `EventId` == a promoted leg's `lot_id.origin_event_id` (BG-D4).
pub type PromoteSet = std::collections::BTreeMap<EventId, PromoteEntry>;
