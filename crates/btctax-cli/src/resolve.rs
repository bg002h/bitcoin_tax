//! **Single profile-source resolver** (full-return v1, SPEC §4.12 / G4).
//!
//! Every consumer (`report`, TUI, `optimize`, `what-if` defaults, `export`) must resolve the tax profile
//! through ONE function so the app never shows two different liabilities for one year (the cardinal sin).
//! Precedence (SPEC §4.12): `ReturnInputs` (full return) → stored `TaxProfile` (raw override) →
//! pseudo-reconcile placeholder → missing.
//!
//! **P2 (task 5):** the `ReturnInputs` arm now DERIVES the frozen [`TaxProfile`] via
//! [`btctax_core::tax::derive_tax_profile`], gated **fail-closed** by the [`screen_inputs`] refuse-guard:
//! an input-screenable refusal — or a year without full-return tables (v1 = TY2024) — yields
//! `profile: None` rather than a wrong number, carrying the [`Refusal`] so the caller can surface it.
use crate::{return_inputs, tax_profile, CliError};
use btctax_core::state::LedgerState;
use btctax_core::tax::derive_tax_profile;
use btctax_core::tax::return_1040::screen_compute_dependent;
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::return_refuse::{screen_inputs, Refusal};
use btctax_core::tax::tables::FullReturnParams;
use btctax_core::{Carryforward, FilingStatus, TaxProfile, TaxTable, Usd};
use rusqlite::Connection;

/// Which source produced the resolved profile (printed on every output so a reviewer can audit — G4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// A full-return `ReturnInputs` blob, derived to a `TaxProfile` (or `None` if refused/unsupported).
    ReturnInputs,
    /// A raw hand-entered `TaxProfile` (the escape hatch).
    StoredProfile,
    /// The pseudo-reconcile all-$0 placeholder (mode on, nothing stored).
    PseudoPlaceholder,
    /// No profile source for the year.
    Missing,
}

/// The resolved profile + its provenance (+ any refusal). `profile` is `None` for [`Provenance::Missing`],
/// and for [`Provenance::ReturnInputs`] when the inputs were refused by the fail-closed guard or the year
/// lacks full-return tables — `refusal` distinguishes those two.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub profile: Option<TaxProfile>,
    pub provenance: Provenance,
    /// Set (with `profile: None`) when `ReturnInputs` were present but the refuse-guard refused them.
    pub refusal: Option<Refusal>,
}

impl Resolved {
    /// `ReturnInputs` were present but no profile could be produced — either the refuse-guard refused them
    /// (`refusal` is `Some`) or the year has no full-return tables (v1 = TY2024; `refusal` is `None`).
    /// Callers MUST surface this, never treat the year as profile-less (which would be a wrong number).
    pub fn is_return_inputs_uncomputable(&self) -> bool {
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

/// The SINGLE SPEC §4.12 precedence ladder — `ReturnInputs` (input-screened + derived) → stored
/// `TaxProfile` → pseudo placeholder → missing. Returns the [`Resolved`] **and the fetched
/// `ReturnInputs`** so [`resolve_and_screen`] can run the compute-dependent screen on the SAME bytes (one
/// fetch — M3) WITHOUT a second copy of the ladder: both public entry points share this, so the
/// precedence invariant lives in exactly one place and its KAT pins the live code (review N2). Not public.
fn resolve_core(
    conn: &Connection,
    year: i32,
    pseudo_reconcile: bool,
    full_return: Option<&FullReturnParams>,
    tax_table: Option<&TaxTable>,
) -> Result<(Resolved, Option<ReturnInputs>), CliError> {
    // 1. Full return (highest precedence): derive the frozen profile, gated fail-closed by the guard.
    if let Some(ri) = return_inputs::get(conn, year)? {
        // A year without full-return tables (v1 = TY2024) cannot be derived — fail closed, no refusal.
        let (Some(params), Some(table)) = (full_return, tax_table) else {
            let r = Resolved {
                profile: None,
                provenance: Provenance::ReturnInputs,
                refusal: None,
            };
            return Ok((r, Some(ri)));
        };
        // Fail-closed: an input-screenable refusal blocks derivation (never a silently-wrong number).
        if let Some(refusal) = screen_inputs(&ri, table, params) {
            let r = Resolved {
                profile: None,
                provenance: Provenance::ReturnInputs,
                refusal: Some(refusal),
            };
            return Ok((r, Some(ri)));
        }
        let r = Resolved {
            profile: Some(derive_tax_profile(&ri, params, year)),
            provenance: Provenance::ReturnInputs,
            refusal: None,
        };
        return Ok((r, Some(ri)));
    }
    // 2. Raw hand-entered profile (the escape hatch).
    if let Some(p) = tax_profile::get(conn, year)? {
        let r = Resolved {
            profile: Some(p),
            provenance: Provenance::StoredProfile,
            refusal: None,
        };
        return Ok((r, None));
    }
    // 3. Pseudo-reconcile placeholder (mode on).
    if pseudo_reconcile {
        let r = Resolved {
            profile: Some(placeholder_tax_profile()),
            provenance: Provenance::PseudoPlaceholder,
            refusal: None,
        };
        return Ok((r, None));
    }
    // 4. Nothing.
    let r = Resolved {
        profile: None,
        provenance: Provenance::Missing,
        refusal: None,
    };
    Ok((r, None))
}

/// Resolve the tax profile for `year` in SPEC §4.12 precedence order (ReturnInputs → stored → pseudo →
/// missing), screening only the input-screenable refuse rows. A COMPUTING consumer that has the ledger
/// `state` should use [`resolve_and_screen`] instead — it shares this exact ladder (`resolve_core`) and
/// additionally runs the compute-dependent refuse-guard. `full_return`/`tax_table` are `None` for a year
/// v1 doesn't support (fails the ReturnInputs arm closed).
pub fn resolve_profile(
    conn: &Connection,
    year: i32,
    pseudo_reconcile: bool,
    full_return: Option<&FullReturnParams>,
    tax_table: Option<&TaxTable>,
) -> Result<Resolved, CliError> {
    Ok(resolve_core(conn, year, pseudo_reconcile, full_return, tax_table)?.0)
}

/// The result of resolving AND screening a year's profile for a COMPUTING consumer (report / optimize /
/// what-if / export). Unlike [`resolve_profile`] (which screens only the input-screenable rows), this also
/// runs the **compute-dependent** refuse-guard ([`screen_compute_dependent`]) that needs the ledger `state`.
pub enum ProfileOutcome {
    /// Ready to compute with. `profile` is `None` only for a genuinely profile-less year (missing).
    Ready {
        profile: Option<TaxProfile>,
        provenance: Provenance,
    },
    /// The year's full-return inputs cannot be computed — refused by a guard, or the year is unsupported.
    /// The caller MUST surface `detail` and NOT compute (fail-closed). `detail` is user-facing.
    Uncomputable { detail: String },
}

/// Resolve `year`'s profile through the single resolver AND apply BOTH refuse-guards (input-screenable +
/// compute-dependent) fail-closed — the one entry point every computing consumer should use so the app
/// never shows two different liabilities, or a wrong number, for one year (SPEC §4.12 / §4.10 / G4).
pub fn resolve_and_screen(
    conn: &Connection,
    state: &LedgerState,
    year: i32,
    pseudo_reconcile: bool,
    full_return: Option<&FullReturnParams>,
    tax_table: Option<&TaxTable>,
) -> Result<ProfileOutcome, CliError> {
    // ONE precedence ladder (shared with `resolve_profile`): `resolve_core` fetches the ReturnInputs once
    // and hands them back so the compute-dependent screen runs on the SAME bytes (M3), with no second copy
    // of the precedence logic (N2).
    let (resolved, ri) = resolve_core(conn, year, pseudo_reconcile, full_return, tax_table)?;
    // Input-screenable refusal / unsupported year (resolve_core already screened those).
    if resolved.is_return_inputs_uncomputable() {
        return Ok(ProfileOutcome::Uncomputable {
            detail: uncomputable_detail(year, resolved.refusal.as_ref()),
        });
    }
    // Compute-dependent refuse rows (need `state`) — on the SAME ReturnInputs `resolve_core` fetched.
    if resolved.provenance == Provenance::ReturnInputs {
        // `provenance == ReturnInputs` ⇒ `resolve_core` returned the fetched `ri` (all three of its RI
        // exits do) on a supported year (`params` is `Some`, else it would have been unsupported/uncomputable
        // and returned above). If that invariant is ever broken by a refactor, fail CLOSED — never silently
        // skip the compute-dependent screen and hand back a number (review M-r3-3).
        let (Some(ri), Some(params)) = (ri.as_ref(), full_return) else {
            return Ok(ProfileOutcome::Uncomputable {
                detail: uncomputable_detail(year, None),
            });
        };
        if let Some(refusal) = screen_compute_dependent(ri, state, year, params) {
            return Ok(ProfileOutcome::Uncomputable {
                detail: uncomputable_detail(year, Some(&refusal)),
            });
        }
    }
    Ok(ProfileOutcome::Ready {
        profile: resolved.profile,
        provenance: resolved.provenance,
    })
}

/// The user-facing message for a `ReturnInputs` year that cannot be computed — a refusal (with its reason)
/// or an unsupported year — both pointing at the `income clear` recovery.
fn uncomputable_detail(year: i32, refusal: Option<&Refusal>) -> String {
    match refusal {
        Some(r) => format!(
            "tax year {year} cannot be computed from its full-return inputs: {}; run \
             `income clear --year {year}` to remove them and use a raw `tax-profile`",
            r.detail
        ),
        None => format!(
            "tax year {year} has full-return inputs, but full-return computation is not supported for \
             {year} in this version (v1 supports TY2024); run `income clear --year {year}` to remove \
             them and use a raw `tax-profile`"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
    use btctax_core::tax::return_inputs::{ReturnInputs, W2};
    use btctax_core::tax::tables::FullReturnTables;
    use btctax_core::TaxTables;
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
    // The bundled TY2024 full-return params + tax table (v1 supports TY2024 only).
    fn ty2024() -> (BundledFullReturnTables, BundledTaxTables) {
        (BundledFullReturnTables::load(), BundledTaxTables::load())
    }
    fn resolve(
        c: &Connection,
        year: i32,
        pseudo: bool,
        fr: &BundledFullReturnTables,
        tt: &BundledTaxTables,
    ) -> Resolved {
        resolve_profile(
            c,
            year,
            pseudo,
            fr.full_return_for(year),
            tt.table_for(year),
        )
        .unwrap()
    }

    #[test]
    fn missing_when_nothing_stored_and_mode_off() {
        let c = mem();
        let (fr, tt) = ty2024();
        let r = resolve(&c, 2024, false, &fr, &tt);
        assert_eq!(r.provenance, Provenance::Missing);
        assert!(r.profile.is_none());
    }

    #[test]
    fn pseudo_placeholder_when_mode_on_and_nothing_stored() {
        let c = mem();
        let (fr, tt) = ty2024();
        let r = resolve(&c, 2024, true, &fr, &tt);
        assert_eq!(r.provenance, Provenance::PseudoPlaceholder);
        assert_eq!(r.profile.unwrap(), placeholder_tax_profile());
    }

    #[test]
    fn stored_profile_beats_pseudo() {
        let c = mem();
        let (fr, tt) = ty2024();
        tax_profile::set(&c, 2024, &prof()).unwrap();
        let r = resolve(&c, 2024, true, &fr, &tt); // mode ON, but a stored profile wins
        assert_eq!(r.provenance, Provenance::StoredProfile);
        assert_eq!(r.profile.unwrap(), prof());
    }

    #[test]
    fn return_inputs_beats_stored_profile_and_derives_a_profile() {
        let c = mem();
        let (fr, tt) = ty2024();
        tax_profile::set(&c, 2024, &prof()).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                box1_wages: dec!(100000),
                box5_medicare_wages: dec!(100000),
                ..Default::default()
            }],
            ..Default::default()
        };
        return_inputs::set(&c, 2024, &ri).unwrap();
        let r = resolve(&c, 2024, true, &fr, &tt);
        assert_eq!(r.provenance, Provenance::ReturnInputs); // highest precedence
        assert!(!r.is_return_inputs_uncomputable());
        // Derived (not the stored raw profile): Single, AGI = $100k − nothing.
        let p = r.profile.unwrap();
        assert_eq!(p.filing_status, FilingStatus::Single);
        assert_eq!(p.magi_excluding_crypto, dec!(100000));
        assert_ne!(p, prof());
    }

    /// [N2] The SPEC §4.12 precedence invariant on the LIVE path every consumer uses (`resolve_and_screen`),
    /// not just `resolve_profile`: with BOTH a stored profile and `ReturnInputs` for one year, the DERIVED
    /// profile wins — the two-liabilities cardinal sin (C1) must be impossible on the production ladder.
    #[test]
    fn resolve_and_screen_gives_return_inputs_precedence_over_stored() {
        let c = mem();
        let (fr, tt) = ty2024();
        tax_profile::set(&c, 2024, &prof()).unwrap(); // a raw MFJ/$120k stored profile
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                box1_wages: dec!(100000),
                box5_medicare_wages: dec!(100000),
                ..Default::default()
            }],
            ..Default::default()
        };
        return_inputs::set(&c, 2024, &ri).unwrap();
        let state = btctax_core::LedgerState::default();
        match resolve_and_screen(
            &c,
            &state,
            2024,
            true,
            fr.full_return_for(2024),
            tt.table_for(2024),
        )
        .unwrap()
        {
            ProfileOutcome::Ready {
                profile,
                provenance,
            } => {
                assert_eq!(provenance, Provenance::ReturnInputs); // RI beats the stored profile
                let p = profile.unwrap();
                assert_eq!(p.filing_status, FilingStatus::Single); // DERIVED, not the MFJ stored one
                assert_eq!(p.magi_excluding_crypto, dec!(100000));
                assert_ne!(p, prof());
            }
            ProfileOutcome::Uncomputable { detail } => panic!("expected Ready, got: {detail}"),
        }
    }

    #[test]
    fn return_inputs_refused_by_guard_is_uncomputable_with_reason() {
        let c = mem();
        let (fr, tt) = ty2024();
        // An HSA present ⇒ the refuse-guard refuses (Form 8889); derivation must NOT proceed.
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        ri.sch1.hsa_activity = Some(true);
        return_inputs::set(&c, 2024, &ri).unwrap();
        let r = resolve(&c, 2024, false, &fr, &tt);
        assert_eq!(r.provenance, Provenance::ReturnInputs);
        assert!(r.is_return_inputs_uncomputable());
        assert!(r.profile.is_none());
        assert!(r.refusal.is_some()); // carries WHY
    }

    #[test]
    fn return_inputs_for_unsupported_year_is_uncomputable_without_refusal() {
        let c = mem();
        let (fr, tt) = ty2024();
        return_inputs::set(&c, 2025, &ReturnInputs::default()).unwrap();
        // 2025 has no bundled full-return tables (v1 = TY2024) → params/table are None → fail closed.
        let r = resolve(&c, 2025, true, &fr, &tt);
        assert_eq!(r.provenance, Provenance::ReturnInputs);
        assert!(r.is_return_inputs_uncomputable());
        assert!(r.refusal.is_none()); // not a refusal — an unsupported year
    }
}
