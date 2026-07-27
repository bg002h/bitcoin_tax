//! Read-only Approach-B status: `btctax defensive status`.
//!
//! This is the SAME pure `btctax_core::defensive::journey_view` the (now-retired) TUI wizard dashboard
//! rendered, driven over the CLI's own session-backed projection. No new computation lives here — this
//! module only loads inputs, refuses on a pseudo-active projection (`journey_view`'s own DFW-D6
//! precondition), and hands the composed view to `render::render_defensive_status`.

use crate::{CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::defensive::{journey_view, DefensiveFilingView};
use btctax_core::experimental::uses_approach_b;
use btctax_store::Passphrase;
use std::path::Path;
use time::OffsetDateTime;

/// The `defensive status` read: the composed [`DefensiveFilingView`], plus whether Approach-B is
/// currently IN USE — a live (non-voided) `DeclareTranche`/`PromoteTranche` on file. The SAME
/// `uses_approach_b` predicate the CLI's `declare-tranche`/`promote-tranche`/`report`/`export-*` verbs
/// already gate their stderr disclosure on (never the dashboard's old "being here at all is using it"
/// rule — this is a plain read, not itself an act of filing).
pub struct DefensiveStatusReport {
    pub view: DefensiveFilingView,
    pub experimental_notice_active: bool,
}

/// Load the ledger, refuse if pseudo-active, and compute `journey_view` over the projected state.
///
/// DFW-D6: `journey_view` opens with `debug_assert!(!state.pseudo_active())` — a defensive-filing
/// journey over synthetic (pseudo-reconciled) estimates is incoherent, since a Phase-B
/// `SelfTransferMine{$0}` default can silently clear a REAL shortfall this command exists to surface.
/// This refuses gracefully BEFORE that assertion could ever fire, mirroring `plan_export`'s own
/// pseudo-active refusal (the composed export step never prompts an attest-phrase override either).
///
/// `now` resolves the wizard's clock-free "current tax year" input (mirrors `plan_export`'s own
/// `current_year` parameter) — never read from the wall clock anywhere else in this module.
pub fn status(
    vault_path: &Path,
    pp: &Passphrase,
    now: OffsetDateTime,
) -> Result<DefensiveStatusReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, cfg) = session.load_events_and_project()?;
    if state.pseudo_active() {
        return Err(CliError::Usage(
            "cannot show defensive-filing status: the ledger is pseudo-reconciled (a synthetic, \
             non-persisted default contributes to the projection), which could silently mask a real \
             shortfall this command exists to surface. Run `btctax reconcile pseudo off` (or approve + \
             attest the pending defaults through `reconcile pseudo approve`) first, then re-run."
                .to_string(),
        ));
    }
    let tables = BundledTaxTables::load();
    let current = now.year();
    let view = journey_view(&events, &state, session.prices(), &tables, &cfg, current);
    Ok(DefensiveStatusReport {
        experimental_notice_active: uses_approach_b(&events),
        view,
    })
}
