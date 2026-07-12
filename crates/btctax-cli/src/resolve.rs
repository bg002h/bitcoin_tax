//! **Single profile-source resolver** (full-return v1, Phase 1 task 3 / SPEC §4.12 / G4).
//!
//! Every consumer (`report`, TUI, `optimize`, `what-if` defaults, `export`) must resolve the tax profile
//! through ONE function so the app never shows two different liabilities for one year (the cardinal sin).
//! Precedence (SPEC §4.12): `ReturnInputs` (full return) → stored `TaxProfile` (raw override) →
//! pseudo-reconcile placeholder → missing.
//!
//! **P1 skeleton (plan re-review I2):** the `ReturnInputs` arm is STUBBED here — `derive_tax_profile` is
//! Phase 2, so a year that has `ReturnInputs` resolves to [`Provenance::ReturnInputs`] with `profile: None`
//! (derivation pending). Full precedence + the derive arm land in P2. No vault can hold `ReturnInputs`
//! until the `income …` subcommands ship (same phase), and callers treat the pending state explicitly.
use crate::{return_inputs, tax_profile, CliError};
use btctax_core::{Carryforward, FilingStatus, TaxProfile, Usd};
use rusqlite::Connection;

/// Which source produced the resolved profile (printed on every output so a reviewer can audit — G4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// A full-return `ReturnInputs` blob (derivation is Phase 2 — `profile` is `None` in P1).
    ReturnInputs,
    /// A raw hand-entered `TaxProfile` (the escape hatch).
    StoredProfile,
    /// The pseudo-reconcile all-$0 placeholder (mode on, nothing stored).
    PseudoPlaceholder,
    /// No profile source for the year.
    Missing,
}

/// The resolved profile + its provenance. `profile` is `None` for [`Provenance::Missing`] and (in P1)
/// [`Provenance::ReturnInputs`] (derivation pending).
#[derive(Debug, Clone)]
pub struct Resolved {
    pub profile: Option<TaxProfile>,
    pub provenance: Provenance,
}

impl Resolved {
    /// A `ReturnInputs` exists but its derivation is not yet implemented (Phase 2). Callers must surface
    /// this rather than silently treating the year as profile-less (which would be a wrong number).
    pub fn is_derivation_pending(&self) -> bool {
        self.provenance == Provenance::ReturnInputs && self.profile.is_none()
    }
}

/// The pseudo-reconcile PLACEHOLDER profile: Single, $0 income / MAGI / qualified-dividends / carryforward.
/// Injected (never persisted) only when the mode is on and nothing else resolves; clears
/// `TaxProfileMissing` ONLY (it is applied after the projection, so it can never clear a Hard gate).
pub fn placeholder_tax_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Usd::ZERO,
        magi_excluding_crypto: Usd::ZERO,
        qualified_dividends_and_other_pref_income: Usd::ZERO,
        other_net_capital_gain: Usd::ZERO,
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: Usd::ZERO,
        w2_medicare_wages: Usd::ZERO,
        schedule_c_expenses: Usd::ZERO,
    }
}

/// Resolve the tax profile for `year` in SPEC §4.12 precedence order. `pseudo_reconcile` is the config
/// flag. The single entry point for every consumer.
pub fn resolve_profile(
    conn: &Connection,
    year: i32,
    pseudo_reconcile: bool,
) -> Result<Resolved, CliError> {
    // 1. Full return (highest precedence). P1: exists ⇒ pending (derivation is P2).
    if return_inputs::exists(conn, year)? {
        return Ok(Resolved {
            profile: None,
            provenance: Provenance::ReturnInputs,
        });
    }
    // 2. Raw hand-entered profile (the escape hatch).
    if let Some(p) = tax_profile::get(conn, year)? {
        return Ok(Resolved {
            profile: Some(p),
            provenance: Provenance::StoredProfile,
        });
    }
    // 3. Pseudo-reconcile placeholder (mode on).
    if pseudo_reconcile {
        return Ok(Resolved {
            profile: Some(placeholder_tax_profile()),
            provenance: Provenance::PseudoPlaceholder,
        });
    }
    // 4. Nothing.
    Ok(Resolved {
        profile: None,
        provenance: Provenance::Missing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use rust_decimal_macros::dec;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        tax_profile::init_table(&c).unwrap();
        return_inputs::init_table(&c).unwrap();
        c
    }
    fn prof() -> TaxProfile {
        let mut p = placeholder_tax_profile();
        p.filing_status = FilingStatus::Mfj;
        p.ordinary_taxable_income = dec!(120000);
        p
    }

    #[test]
    fn missing_when_nothing_stored_and_mode_off() {
        let c = mem();
        let r = resolve_profile(&c, 2024, false).unwrap();
        assert_eq!(r.provenance, Provenance::Missing);
        assert!(r.profile.is_none());
    }

    #[test]
    fn pseudo_placeholder_when_mode_on_and_nothing_stored() {
        let c = mem();
        let r = resolve_profile(&c, 2024, true).unwrap();
        assert_eq!(r.provenance, Provenance::PseudoPlaceholder);
        assert_eq!(r.profile.unwrap(), placeholder_tax_profile());
    }

    #[test]
    fn stored_profile_beats_pseudo() {
        let c = mem();
        tax_profile::set(&c, 2024, &prof()).unwrap();
        let r = resolve_profile(&c, 2024, true).unwrap(); // mode ON, but a stored profile wins
        assert_eq!(r.provenance, Provenance::StoredProfile);
        assert_eq!(r.profile.unwrap(), prof());
    }

    #[test]
    fn return_inputs_beats_stored_profile_but_is_pending_in_p1() {
        let c = mem();
        tax_profile::set(&c, 2024, &prof()).unwrap();
        return_inputs::set(&c, 2024, &ReturnInputs::default()).unwrap();
        let r = resolve_profile(&c, 2024, true).unwrap();
        assert_eq!(r.provenance, Provenance::ReturnInputs); // highest precedence
        assert!(r.is_derivation_pending()); // P1: derive is P2
        assert!(r.profile.is_none());
    }
}
