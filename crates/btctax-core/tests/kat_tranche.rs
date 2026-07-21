//! KATs for the conservative-filing `DeclareTranche` core (Phase 1).
//!
//! See `design/conservative-filing/{SPEC,IMPLEMENTATION_PLAN}.md`. A tranche is undocumented BTC
//! declared at $0 basis (the IRS fallback), tagged `BasisSource::EstimatedConservative`, homed at
//! `acquired_at = window_end`; filing-ready (NOT pseudo).

use btctax_core::event::BasisSource;
use btctax_core::forms::how_acquired_from;
use btctax_core::Form8283HowAcquired;

/// Task 1 (tax min-6): `EstimatedConservative` is NOT an 8949 column; on Form 8283 (donation) it needs
/// manual review — an LT tranche donation → FMV; an ST-held tranche donation → deduction limited to
/// basis = $0 (§170(e)(1)(A)).
#[test]
fn estimated_conservative_donor_field_is_review() {
    assert_eq!(
        how_acquired_from(BasisSource::EstimatedConservative),
        Form8283HowAcquired::Review
    );
}
