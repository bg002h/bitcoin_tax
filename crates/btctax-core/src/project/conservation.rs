//! FR9 sat-conservation report: proves that every satoshi entering externally is accounted for.
//!
//! Identity (FR9): Σin == Σdisposed + Σremoved + Σheld + Σfee_sats + Σpending
//!
//! CRITICAL: `fee_mini_disposition == true` Disposals are EXCLUDED from Σdisposed.  A config-(b)
//! mini-disposition is a recognition record only — its sats are already counted in `fee_sats_consumed`
//! (the sole FR9 conservation home for network-fee sats).  Double-counting them would break the identity.
use crate::conventions::Sat;
use crate::state::{BlockerKind, LedgerState};

/// FR9 sat-conservation report produced by [`conservation_report`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConservationReport {
    /// Externally-sourced sats: Acquire + Income + classified GiftReceived.
    pub sigma_in: Sat,
    /// Disposal legs where `!fee_mini_disposition` (Sell / Spend / reclassified outflow).
    pub sigma_disposed: Sat,
    /// Removal legs (Gift / Donation).
    pub sigma_removed: Sat,
    /// Σ lots remaining_sat.
    pub sigma_held: Sat,
    /// Sole FR9 home for network-fee sats (TP8).
    pub sigma_fee_sats: Sat,
    /// Principal + fee sats sitting in `pending_reconciliation`.
    pub sigma_pending: Sat,
    /// `sigma_in == disposed + removed + held + fee + pending` AND no uncovered disposal.
    pub balanced: bool,
    /// At least one `BlockerKind::UncoveredDisposal` is open (identity undefined).
    pub has_uncovered: bool,
}

/// Compute the FR9 conservation report from a projected `LedgerState`.
///
/// Pure: reads `st` once, no I/O, no allocation beyond the return value.
/// `sigma_in` and `fee_sats_consumed` are read from `st.stats` (M3) because they are not
/// directly reconstructable from the post-fold vectors alone — the fold accumulates them.
pub fn conservation_report(st: &LedgerState) -> ConservationReport {
    let sigma_disposed = st
        .disposals
        .iter()
        .filter(|d| !d.fee_mini_disposition)
        .flat_map(|d| &d.legs)
        .map(|l| l.sat)
        .sum();
    let sigma_removed = st
        .removals
        .iter()
        .flat_map(|r| &r.legs)
        .map(|l| l.sat)
        .sum();
    let sigma_held: Sat = st.lots.iter().map(|l| l.remaining_sat).sum();
    let has_uncovered = st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UncoveredDisposal);
    let (sigma_in, sigma_fee_sats, sigma_pending) = (
        st.stats.sigma_in,
        st.stats.fee_sats_consumed,
        st.stats.sigma_pending,
    );
    let balanced = !has_uncovered
        && sigma_in == sigma_disposed + sigma_removed + sigma_held + sigma_fee_sats + sigma_pending;
    ConservationReport {
        sigma_in,
        sigma_disposed,
        sigma_removed,
        sigma_held,
        sigma_fee_sats,
        sigma_pending,
        balanced,
        has_uncovered,
    }
}
