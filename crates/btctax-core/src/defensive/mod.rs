//! Defensive Filing Wizard: derived, read-only signals over the projected `LedgerState` — never new tax
//! logic, never a second source of truth. `discovery` is the DFW-D4 structured-shortfall/triage layer
//! (Task 5); Task 6 adds `journey_view` (the guided-dashboard read alongside it) — the composed,
//! pure/read-only view a dashboard renders from: candidates to declare, shortfalls to resolve first,
//! per-tranche status + derived advisories + clamped saving figures, the pool-level "still short" state,
//! the BG-D9 fold-diff export year-set, and the safe-harbor mutual-exclusion flag.
//!
//! **Pseudo discipline (DFW-D6):** `journey_view` opens with `debug_assert!(!state.pseudo_active())` —
//! the precondition Task 7's dashboard entry gate enforces (a defensive-filing journey over synthetic
//! (pseudo-reconciled) estimates is incoherent: a Phase-B `SelfTransferMine{$0}` default can silently
//! clear a REAL shortfall, masking exactly the gap this feature exists to surface). Every shadow
//! projection this module runs (the over-covered / fee-only / now-displacing / clamped-saving folds)
//! forces `pseudo_reconcile = false` on its OWN config copy — mirroring `would_conflict`
//! (`project/mod.rs:119`) — so this module's derivations never depend on the caller's pseudo bit, only on
//! `state.pseudo_active()` being false to begin with.
//!
//! **No new tax logic.** Every advisory/saving here is DERIVED from already-shipped signals:
//! `discovery::{shortfalls, triage}` (Task 5), `conservative::{method_inversion_advisory,
//! tranche_dip_advisory, flagged_years}`, `conservative_promote::{filed_basis_for,
//! clamped_promote_year_saving}`, and `tranche_guard::{in_force_allocation_exists,
//! pre2025_tranche_exists}`. `journey_view` composes and diffs; it computes no new tax rule and files no
//! number (a `PromoteTranche`/`DeclareTranche` is never written here).

pub mod discovery;

use crate::conservative::{flagged_years, method_inversion_advisory, tranche_dip_advisory};
use crate::conservative_promote::{
    clamped_promote_year_saving, filed_basis_for, with_synthetic_promote,
};
use crate::defensive::discovery::{shortfalls, triage, Shortfall, Triage};
use crate::event::{BasisSource, DeclareTranche, EventPayload};
use crate::identity::EventId;
use crate::price::PriceProvider;
use crate::project::pools::{pool_key, PoolKey};
use crate::project::{in_force_methods, project, ProjectionConfig};
use crate::state::LedgerState;
use crate::tax::{compute_tax_year, TaxOutcome, TaxTables};
use crate::tranche_guard::{in_force_allocation_exists, pre2025_tranche_exists, void_targets};
use crate::LedgerEvent;
use crate::Usd;
use std::collections::BTreeSet;

/// One row per LIVE (non-voided) `DeclareTranche` in the vault (★ I-3: status is `DeclaredZero` OR
/// `Promoted` ONLY — DFW-D3/D5.3 forbid recording a per-tranche "did/didn't cover" attribution; that
/// question is answered pool-level, in `DefensiveFilingView.still_short`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrancheRow {
    /// The `DeclareTranche` decision's OWN `EventId` (never a promote's id).
    pub target: EventId,
    pub sat: i64,
    pub status: TrancheStatus,
    pub clamped_saving: Vec<SavingFlavor>,
    pub advisories: Vec<Advisory>,
}

/// ★ I-3: NO per-tranche `DidNotCover` variant — DFW-D3 (revocable-until-promoted, no step-tracking) and
/// DFW-D5.3 (no per-tranche coverage attribution) both forbid it. A tranche is either still `$0`
/// (`DeclaredZero`) or has a live promote (`Promoted`) — nothing else is ever recorded per-tranche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrancheStatus {
    DeclaredZero,
    Promoted,
}

/// Derived, read-only advisories on a `TrancheRow` — never a gate, never a filed number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advisory {
    /// The tranche's declared sat exceeds the shortfall it actually covers by `by_sat` (DFW-D5.3, M-1
    /// scope) — a without-this-tranche sat-count shadow shows the pool would still be short, just by
    /// LESS than the whole tranche. Never fired for a fully-undisposed tranche (`covered_sat == 0`).
    OverCovered { by_sat: i64 },
    /// A LIVE promote's floor now displaces documented basis on a real disposal (mirrors
    /// `promote_drift_advisory`'s with/without pattern, but over `basis_source` COMPOSITION rather than a
    /// price recompute): a with/without-this-promote fold-diff shows a disposal that drew a documented
    /// (non-`EstimatedConservative`) leg WITHOUT the promote, but no longer draws it WITH the promote.
    NowDisplacing,
    /// `conservative::method_inversion_advisory`, surfaced VERBATIM.
    MethodInversion(String),
    /// `conservative::tranche_dip_advisory`, surfaced VERBATIM (one entry per disposal touching this
    /// tranche's own `EstimatedConservative` legs).
    TrancheDip(String),
    /// The shortfall(s) this tranche covers (the SAME without-this-tranche shadow behind `OverCovered`)
    /// are ALL fee-component (`Shortfall.short_sat == Shortfall.fee_sat`) — promoting this tranche would
    /// only ever substantiate fee-sat basis, never principal.
    FeeOnlyPromoteNoop,
}

/// The three-flavor clamped promote-saving discipline (BG-D6/DFW-D10): a bare `$X (year Y)` is NEVER
/// shown for a non-computing year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavingFlavor {
    /// Both folds (with the tranche promoted to its computed floor / without) price `year` — a matching
    /// bundled table AND a stored `TaxProfile` AND no Hard blocker anywhere in EITHER fold. `delta` is the
    /// CLAMPED (`clamped_promote_year_saving`) federal-tax saving, never negative.
    ComputedTax { year: i32, delta: Usd },
    /// At least one fold cannot price `year` (no bundled table / no stored profile / a Hard blocker) — the
    /// profile-free realized-gain delta (`gain(without-promote) - gain(with-promote)` for `year`) instead
    /// of a dollar tax figure.
    Uncomputable { year: i32, gain_delta: Usd },
    /// Nothing year-keyed is computable at all (e.g. the declared window lacks the `Coverage::Full`
    /// price data `filed_basis_for` requires) — a named, unquantified note.
    Named(String),
}

/// The pool-level "still short" state (★ I-3/arch-I-5): ONE combined row per pool, never a per-tranche
/// attribution. `short_sat`/`live_tranche_sat` are the RESIDUAL values (tax-M-2) — not just a count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolShort {
    pub pool: PoolKey,
    pub short_sat: i64,
    pub live_tranche_sat: i64,
}

/// The composed, pure/read-only Defensive Filing Wizard dashboard view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefensiveFilingView {
    pub candidates: Vec<Shortfall>,
    pub resolve_first: Vec<Triage>,
    pub tranches: Vec<TrancheRow>,
    pub still_short: Vec<PoolShort>,
    /// = `conservative::flagged_years(events, state, prices, tables, cfg, current)` — the SAME
    /// `< current`-filtered structured year-set the DFW-D11 export uses (tax-N-2: no display-vs-export
    /// drift). NEVER re-derived from `promote_prior_year_advisory`'s `Vec<String>` (that would re-enter
    /// the banned string-parse).
    pub flagged_years: BTreeSet<i32>,
    pub safe_harbor_blocked: bool,
}

/// Force `pseudo_reconcile = false` on an OWN copy of `cfg` (DFW-D6) — the pattern every shadow
/// projection in this module uses, mirroring `would_conflict` (`project/mod.rs:119`).
fn pseudo_off(cfg: &ProjectionConfig) -> ProjectionConfig {
    let mut c = *cfg;
    c.pseudo_reconcile = false;
    c
}

/// This tranche's own pool (the pool its `$0`/floor lot lives in): `pool_key(window_end, wallet)` — the
/// tranche's lot is homed at `window_end` (its `acquired_at`), so that is the correct pool-membership
/// date, exactly as the fold assigns it.
fn tranche_pool(t: &DeclareTranche) -> PoolKey {
    pool_key(t.window_end, &t.wallet)
}

/// The live (non-voided) `PromoteTranche` `EventId` targeting `tranche_id`, or `None` if it is not (yet)
/// promoted / its promote was voided. At most one exists for any tranche `state.promoted_origins`
/// recognizes as promoted (a conflicting second promote never reaches `promoted_origins`).
fn find_live_promote_id(events: &[LedgerEvent], tranche_id: &EventId) -> Option<EventId> {
    let voided = void_targets(events);
    events.iter().find_map(|e| match &e.payload {
        EventPayload::PromoteTranche(p) if p.target == *tranche_id && !voided.contains(&e.id) => {
            Some(e.id.clone())
        }
        _ => None,
    })
}

/// The without-THIS-tranche sat-count shadow (DFW-D5.3, M-1 scope): re-project with `tranche_id`'s own
/// `DeclareTranche` event excluded (forcing pseudo off, its own copy), and return the aggregate shortfall
/// sat STILL short in `tranche_id`'s own pool. `covered_sat > 0` means this tranche is (at least
/// partially) load-bearing — removing it reopens a real shortfall of that size; `covered_sat == 0` means
/// it is a legitimate forward-hold (fully-undisposed, or every bit of coverage it appeared to provide was
/// actually redundant with other supply) — DFW-D5.3 forbids treating that as over-covered.
fn covering_shortfalls(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    off_cfg: &ProjectionConfig,
    tranche_id: &EventId,
    pool: &PoolKey,
) -> Vec<Shortfall> {
    let without_events: Vec<LedgerEvent> = events
        .iter()
        .filter(|e| e.id != *tranche_id)
        .cloned()
        .collect();
    let without_state = project(&without_events, prices, off_cfg);
    shortfalls(&without_state)
        .into_iter()
        .filter(|s| {
            s.wallet
                .as_ref()
                .is_some_and(|w| pool_key(s.date, w) == *pool)
        })
        .collect()
}

/// `Advisory::NowDisplacing` (mirrors `promote_drift_advisory`'s with/without shape, over `basis_source`
/// COMPOSITION rather than a price recompute — DFW-D5.3/tax-N-1): for each disposal that draws a leg from
/// `tranche_id` in the CURRENT (with-promote) fold, does the SAME disposal event, WITHOUT this promote,
/// draw a documented (non-`EstimatedConservative`) source that the with-promote fold no longer draws?
/// Composition (the SET of basis sources present), not leg-Vec inequality (★ tax-M negative KAT): a
/// correctly-sized cover's OWN leg changing `$0` → floor is NOT a composition change (both sets are
/// `{EstimatedConservative}`) and must not fire.
fn now_displacing(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    off_cfg: &ProjectionConfig,
    tranche_id: &EventId,
    promote_id: &EventId,
) -> bool {
    let with_state = project(events, prices, off_cfg);
    let without_events: Vec<LedgerEvent> = events
        .iter()
        .filter(|e| e.id != *promote_id)
        .cloned()
        .collect();
    let without_state = project(&without_events, prices, off_cfg);

    for wd in &with_state.disposals {
        let touches_tranche = wd
            .legs
            .iter()
            .any(|l| l.lot_id.origin_event_id == *tranche_id);
        if !touches_tranche {
            continue;
        }
        let Some(wod) = without_state.disposals.iter().find(|d| d.event == wd.event) else {
            continue;
        };
        let with_sources: Vec<BasisSource> = wd.legs.iter().map(|l| l.basis_source).collect();
        let displaced = wod.legs.iter().any(|l| {
            l.basis_source != BasisSource::EstimatedConservative
                && !with_sources.contains(&l.basis_source)
        });
        if displaced {
            return true;
        }
    }
    false
}

/// `TrancheRow.clamped_saving` (DFW-D10, BG-D6 three-flavor discipline; `clamped_promote_year_saving`,
/// CLAMPED ONLY — never the unclamped `overpayment_delta` reconstruction what-if). Only meaningful for a
/// NOT-YET-promoted (`DeclaredZero`) tranche — once promoted, its basis IS filed, so there is nothing
/// left to quote a "what if promoted" saving on (mirrors `overpayment_nudge_lines`'s own promoted-tranche
/// status-line carve, Task 11 §3 item 3).
///
/// Scope: every year this tranche's OWN `EstimatedConservative` legs were actually disposed (a fully
/// undisposed tranche yields no years, hence an empty Vec — nothing realized to quantify yet). For each
/// such year: `ComputedTax` only when BOTH the without-promote and the with-synthetic-promote fold price
/// it (`compute_tax_year` succeeds on both — table ∧ stored `TaxProfile` ∧ no Hard blocker anywhere);
/// else the profile-free `gain_delta = gain(without) − gain(with)` for that year (`Uncomputable`). When
/// `filed_basis_for` itself cannot produce a trustworthy floor (no/partial price coverage over the
/// declared window), nothing year-keyed can be computed at all — one `Named` note, no per-year loop.
///
/// ★ `journey_view` carries no `TaxProfile` parameter (pure core; a stored profile is a CLI-side-table
/// concept) — every `compute_tax_year` call here passes `profile: None`, so every year computed by THIS
/// function will structurally fall through to `Uncomputable` (never `ComputedTax`) until a caller with
/// profile access re-derives the figure. `ComputedTax`'s branch is kept live (not dead code) for
/// signature/behavior symmetry with `clamped_promote_year_saving`'s own three-flavor contract.
#[allow(clippy::too_many_arguments)]
fn clamped_saving_for(
    events: &[LedgerEvent],
    state: &LedgerState,
    prices: &dyn PriceProvider,
    tables: &dyn TaxTables,
    off_cfg: &ProjectionConfig,
    tranche_id: &EventId,
    t: &DeclareTranche,
) -> Vec<SavingFlavor> {
    let cf = match filed_basis_for(prices, t.sat, t.window_start, t.window_end) {
        Ok(cf) => cf,
        Err(_) => {
            return vec![SavingFlavor::Named(
                "no clamped saving is computable for this tranche — the declared window lacks full \
                 price coverage (Coverage::Full) to compute a promotion floor"
                    .to_string(),
            )];
        }
    };

    let mut years: BTreeSet<i32> = BTreeSet::new();
    for d in &state.disposals {
        if d.legs.iter().any(|l| {
            l.lot_id.origin_event_id == *tranche_id
                && l.basis_source == BasisSource::EstimatedConservative
        }) {
            years.insert(d.disposed_at.year());
        }
    }
    if years.is_empty() {
        return Vec::new();
    }

    let without_state = project(events, prices, off_cfg);
    let with_events = with_synthetic_promote(events, tranche_id, cf.filed_basis);
    let with_state = project(&with_events, prices, off_cfg);

    let gain_for_year = |st: &LedgerState, year: i32| -> Usd {
        st.disposals
            .iter()
            .filter(|d| d.disposed_at.year() == year)
            .flat_map(|d| &d.legs)
            .map(|l| l.gain)
            .sum()
    };

    years
        .into_iter()
        .map(|year| {
            let with_outcome = compute_tax_year(events, &with_state, year, None, tables);
            let without_outcome = compute_tax_year(events, &without_state, year, None, tables);
            match (with_outcome, without_outcome) {
                (TaxOutcome::Computed(_), TaxOutcome::Computed(_)) => {
                    let delta = clamped_promote_year_saving(
                        events,
                        prices,
                        off_cfg,
                        tranche_id,
                        cf.filed_basis,
                        year,
                        None,
                        tables,
                    );
                    SavingFlavor::ComputedTax { year, delta }
                }
                _ => {
                    let gain_delta =
                        gain_for_year(&without_state, year) - gain_for_year(&with_state, year);
                    SavingFlavor::Uncomputable { year, gain_delta }
                }
            }
        })
        .collect()
}

/// One `TrancheRow` for a live `DeclareTranche` — status, derived advisories, and the clamped-saving
/// figures.
#[allow(clippy::too_many_arguments)]
fn build_tranche_row(
    events: &[LedgerEvent],
    state: &LedgerState,
    prices: &dyn PriceProvider,
    tables: &dyn TaxTables,
    off_cfg: &ProjectionConfig,
    current: i32,
    tranche_id: &EventId,
    t: &DeclareTranche,
) -> TrancheRow {
    let promoted = state.promoted_origins.contains(tranche_id);
    let status = if promoted {
        TrancheStatus::Promoted
    } else {
        TrancheStatus::DeclaredZero
    };

    let mut advisories: Vec<Advisory> = Vec::new();

    // OverCovered / FeeOnlyPromoteNoop — the SAME without-this-tranche sat-count shadow (DFW-D5.3, M-1).
    let pool = tranche_pool(t);
    let covering = covering_shortfalls(events, prices, off_cfg, tranche_id, &pool);
    let covered_sat: i64 = covering.iter().map(|s| s.short_sat).sum();
    if covered_sat > 0 {
        if t.sat > covered_sat {
            advisories.push(Advisory::OverCovered {
                by_sat: t.sat - covered_sat,
            });
        }
        if covering.iter().all(|s| s.short_sat == s.fee_sat) {
            advisories.push(Advisory::FeeOnlyPromoteNoop);
        }
    }

    // NowDisplacing — only meaningful once a live promote exists to exclude in the shadow.
    if promoted {
        if let Some(promote_id) = find_live_promote_id(events, tranche_id) {
            if now_displacing(events, prices, off_cfg, tranche_id, &promote_id) {
                advisories.push(Advisory::NowDisplacing);
            }
        }
    }

    // MethodInversion — wallet-level, keyed at the clock-free `current` year-end (mirrors
    // `tranche_report_advisory`'s own `in_force_methods(..., Dec 31 of the report year, ...)` lookup).
    if let Ok(as_of) = time::Date::from_calendar_date(current, time::Month::December, 31) {
        let wallets = [t.wallet.clone()];
        let methods = in_force_methods(events, prices, off_cfg, as_of, &wallets);
        if let Some(m) = methods.first() {
            if let Some(msg) = method_inversion_advisory(state, &t.wallet, m.method) {
                advisories.push(Advisory::MethodInversion(msg));
            }
        }
    }

    // TrancheDip — verbatim, one entry per disposal that drew this tranche's own EstimatedConservative
    // leg(s); reads the CALLER's real (already pseudo-off, per the entry precondition) `state` directly —
    // no shadow needed (this is a plain read, not a with/without diff).
    for d in state.disposals.iter().filter(|d| {
        d.legs.iter().any(|l| {
            l.lot_id.origin_event_id == *tranche_id
                && l.basis_source == BasisSource::EstimatedConservative
        })
    }) {
        if let Some(msg) = tranche_dip_advisory(d) {
            advisories.push(Advisory::TrancheDip(msg));
        }
    }

    let clamped_saving = if promoted {
        Vec::new()
    } else {
        clamped_saving_for(events, state, prices, tables, off_cfg, tranche_id, t)
    };

    TrancheRow {
        target: tranche_id.clone(),
        sat: t.sat,
        status,
        clamped_saving,
        advisories,
    }
}

/// `DefensiveFilingView.still_short` (★ I-3/arch-I-5): ONE combined `PoolShort` per pool — never a
/// per-tranche attribution. Reads the CALLER's real (pseudo-off) `state` directly (the RESIDUAL shortfall
/// after every available lot, including any live tranche, has already been drawn — no shadow needed): for
/// every structured shortfall whose pool has at least one live `DeclareTranche` with `window_end <=` the
/// short date, the pool enters "still short", with `short_sat` = Σ residual shortfall and
/// `live_tranche_sat` = the matching live tranche(s)' own declared sat (the RESIDUAL VALUES, tax-M-2 —
/// not just a count).
fn still_short_pools(
    state: &LedgerState,
    live_tranches: &[(EventId, DeclareTranche)],
) -> Vec<PoolShort> {
    use std::collections::BTreeMap;
    let mut by_pool: BTreeMap<PoolKey, (i64, i64)> = BTreeMap::new();
    for sf in shortfalls(state) {
        let Some(w) = &sf.wallet else { continue };
        let pool = pool_key(sf.date, w);
        let matching_tranche_sat: i64 = live_tranches
            .iter()
            .filter(|(_, t)| tranche_pool(t) == pool && t.window_end <= sf.date)
            .map(|(_, t)| t.sat)
            .sum();
        if matching_tranche_sat > 0 {
            let entry = by_pool.entry(pool).or_insert((0, 0));
            entry.0 += sf.short_sat;
            entry.1 = entry.1.max(matching_tranche_sat);
        }
    }
    by_pool
        .into_iter()
        .map(|(pool, (short_sat, live_tranche_sat))| PoolShort {
            pool,
            short_sat,
            live_tranche_sat,
        })
        .collect()
}

/// The composed, pure Defensive Filing Wizard dashboard view (DFW-D1/D4/D5.3/D6/D7/D10/D11; ★ I-3).
/// `current` is the wizard's clock-free current tax year (as `plan_export` uses it) — never a wall clock.
///
/// Opens with `debug_assert!(!state.pseudo_active())` (★ arch-m-new-4 — the DFW-D6 precondition Task 7's
/// dashboard entry gate enforces): a defensive-filing journey over a pseudo-active projection is
/// incoherent, since a Phase-B `SelfTransferMine{$0}` default can silently clear a REAL shortfall this
/// feature exists to surface. Every shadow projection this fn (or a helper it calls) runs forces
/// `pseudo_reconcile = false` on its own config copy (DFW-D6) — never depending on the caller's `cfg`.
pub fn journey_view(
    events: &[LedgerEvent],
    state: &LedgerState,
    prices: &dyn PriceProvider,
    tables: &dyn TaxTables,
    cfg: &ProjectionConfig,
    current: i32,
) -> DefensiveFilingView {
    debug_assert!(
        !state.pseudo_active(),
        "journey_view precondition (DFW-D6): state must not be pseudo-active — the Task 7 dashboard \
         entry gate enforces this before ever calling journey_view"
    );

    let off_cfg = pseudo_off(cfg);

    let triaged = triage(events, state);
    let candidates: Vec<Shortfall> = triaged
        .iter()
        .filter_map(|t| match t {
            Triage::DeclareCandidate(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    let resolve_first: Vec<Triage> = triaged
        .into_iter()
        .filter(|t| matches!(t, Triage::ResolveFirst { .. }))
        .collect();

    let voided = void_targets(events);
    let live_tranches: Vec<(EventId, DeclareTranche)> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::DeclareTranche(t) if !voided.contains(&e.id) => {
                Some((e.id.clone(), t.clone()))
            }
            _ => None,
        })
        .collect();

    let tranches: Vec<TrancheRow> = live_tranches
        .iter()
        .map(|(id, t)| build_tranche_row(events, state, prices, tables, &off_cfg, current, id, t))
        .collect();

    let still_short = still_short_pools(state, &live_tranches);

    // ★ tax-N-2: the SAME `< current`-filtered structured year-set `plan_export` computes — no
    // display-vs-export drift. `flagged_years` (M-new-1) forces pseudo off internally regardless, but this
    // call passes the already-forced-off config for belt-and-suspenders consistency with every other
    // shadow in this module.
    let flagged = flagged_years(events, state, prices, tables, &off_cfg, current);

    let safe_harbor_blocked = in_force_allocation_exists(events) || pre2025_tranche_exists(events);

    DefensiveFilingView {
        candidates,
        resolve_first,
        tranches,
        still_short,
        flagged_years: flagged,
        safe_harbor_blocked,
    }
}
