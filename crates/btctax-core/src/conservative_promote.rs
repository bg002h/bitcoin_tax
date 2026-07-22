//! Approach-B / BG-D3: the `filed_basis` compute for promoting a `$0` conservative-filing tranche to a
//! filed `>$0` basis FLOOR, gated hard on `Coverage::Full` (a `Partial`-covered window min can EXCEED the
//! true window min — `conservative::window_reference`'s doc — so filing on it could UNDERSTATE the true
//! floor; refused rather than silently downgraded). Also defines `PromoteEntry`/`PromoteSet` — the ★
//! shared decomposition-key types every leg-builder task (T4/T5/T6) consumes, given ONE owner here (arch
//! r1 I-1) rather than each leg builder inventing its own shape.

use crate::conservative::{window_reference, Coverage};
use crate::conventions::{round_cents, tax_date, Sat, TaxDate, Usd, SATS_PER_BTC};
use crate::event::{
    Acknowledgment, ConsentTerm, EventPayload, FloorMethod, LedgerEvent, PromoteTranche,
};
use crate::identity::EventId;
use crate::price::PriceProvider;
use crate::project::resolve::resolve;
use crate::project::{project, ProjectionConfig};
use crate::state::{Disposal, LedgerState, Removal, RemovalKind};
use crate::tax::{compute_tax_year, TaxOutcome, TaxProfile, TaxTables};
use std::collections::BTreeSet;
use time::UtcOffset;

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

/// ★ BG-D3 verify-drift advisory (Task 11; tax r2 I-1 / arch r2 I-2): for each LIVE promote, recompute
/// `filed_basis_for` against the CURRENT `prices` and compare to the STORED `filed_basis`. A later
/// price-data correction can move the recomputed window min, so a filed floor can DRIFT away from what the
/// same method would compute today:
///
/// - stored **above** the recomputed reference → the stored floor is OVERSTATED (an over-claimed basis, a
///   §6662 exposure). The engine has NO filed-year concept (Opus r3 tax M-1, mirroring BG-D9), so this is
///   CONDITIONAL COPY, not an engine branch: *"if this position is not yet filed, void + re-promote to the
///   corrected lower number $X (G-4); if already filed, the filed number stands — advisory only."*
/// - stored **below** the recomputed reference → the stored floor is UNDERSTATED (conservative — it
///   reports at least as much gain as the corrected floor would). Tax-safe, but still SURFACED.
///
/// A promote whose window no longer has `Coverage::Full` (a price gap opened) yields no trustworthy
/// recompute → skipped (never fabricate a drift claim over a data gap, mirroring `filed_basis_for`'s
/// refusal). The live-promote set is read from `resolve` (so a voided/conflicting promote is excluded,
/// exactly as the fold sees it). ★ The FOLD is UNCHANGED — it always folds the STORED `filed_basis` (T3);
/// this builder is informational only, NOTHING is written.
pub fn promote_drift_advisory(events: &[LedgerEvent], prices: &dyn PriceProvider) -> Vec<String> {
    // Liveness (voids/conflicts) is config-independent, and `filed_basis_for` reads only prices + window,
    // so a default projection config suffices to extract the live promote set.
    let config = ProjectionConfig::default();
    let promotes = resolve(events, prices, &config).promotes;
    let mut lines: Vec<String> = Vec::new();
    for (target, entry) in &promotes {
        // The promoted tranche's window (for the recompute) — always present for a live promote target.
        let Some((ws, we)) = events.iter().find_map(|e| match &e.payload {
            EventPayload::DeclareTranche(t) if e.id == *target => {
                Some((t.window_start, t.window_end))
            }
            _ => None,
        }) else {
            continue;
        };
        let stored = entry.filed_basis;
        let recomputed = match filed_basis_for(prices, entry.tranche_sat, ws, we) {
            Ok(cf) => cf.filed_basis,
            Err(_) => continue, // current data can't produce a trustworthy floor → no drift claim
        };
        match stored.cmp(&recomputed) {
            std::cmp::Ordering::Greater => lines.push(format!(
                "Promote-drift — the filed basis floor for the {ws}\u{2013}{we} tranche (${stored:.2} \
                 stored) now recomputes to ${recomputed:.2} against current price data \u{2014} LOWER, so \
                 the stored floor is OVERSTATED. If this position is not yet filed, void the promote and \
                 re-promote to the corrected lower number ${recomputed:.2} (G-4). If it is already filed, \
                 the filed number stands \u{2014} advisory only (the engine keeps folding the stored \
                 basis)."
            )),
            std::cmp::Ordering::Less => lines.push(format!(
                "Promote-drift — the filed basis floor for the {ws}\u{2013}{we} tranche (${stored:.2} \
                 stored) now recomputes to ${recomputed:.2} against current price data \u{2014} HIGHER, so \
                 the stored floor is UNDERSTATED (conservative: it reports at least as much gain as the \
                 corrected floor would). No amendment is required; you may re-promote to the higher floor \
                 to reduce the reported gain going forward."
            )),
            std::cmp::Ordering::Equal => {} // no drift
        }
    }
    lines
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

/// BG-D4 — the disposal-leg loss clamp (Opus r3 tax I-1 formula). Files the promoted `filed_basis` floor
/// as basis but NEVER manufactures a loss off the estimate: the estimate component is clamped against the
/// proceeds REMAINING after the documented component, so it can neither drive a leg negative nor crowd the
/// documented basis below zero (an estimate-ENABLED loss).
///
/// - `promote`: the stored promotion for this leg's tranche (`promotes.get(&lot_id.origin_event_id)`);
///   `None` for a non-promoted lot ⇒ `usd_basis_share` is returned UNCHANGED (identity, behavior-preserving).
/// - `leg_sat`: this leg's consumed sat (the pro-ration numerator).
/// - `usd_basis_share`: the lot's pro-rata basis for this leg (`Consumed::gain_basis`) — MAY include a
///   TP8(c) self-transfer fee carry re-homed into `usd_basis`, indistinguishable from the estimate.
/// - `net_proceeds_share`: this leg's pro-rata share of the disposal's net proceeds.
///
/// ```text
///   estimate_share   = round_cents(filed_basis × leg_sat / tranche_sat)   (from the STORED promote)
///   documented_share = usd_basis_share − estimate_share                   (UNCLAMPED; may be < 0 at cent scale)
///   estimate_basis   = clamp(net_proceeds_share − documented_share, $0, estimate_share)
///   reported_basis   = documented_share + estimate_basis
/// ```
///
/// ★ The clamp bound is `net − documented`, NOT bare `net`: without the `− documented_share` a documented
/// fee carry would let the estimate absorb proceeds the documented basis also needs, filing a loss that is
/// 100% but-for the estimate (floor $12k + $30 documented fee sold at $8k → `$30 + min($12k, $8k) = $8,030`
/// = a −$30 estimate-enabled loss; the correct `net − documented` bound files `$8,000` = gain $0). A GENUINE
/// documented loss (documented ALONE `> net`) still reaches negative: `estimate_basis → $0`,
/// `reported = documented > net`. The unclaimed floor simply EVAPORATES (it never shifts to another leg).
pub fn clamped_leg_basis(
    promote: Option<&PromoteEntry>,
    leg_sat: Sat,
    usd_basis_share: Usd,
    net_proceeds_share: Usd,
) -> Usd {
    let Some(p) = promote else {
        return usd_basis_share; // not a promoted lot → basis unchanged
    };
    let estimate_share = round_cents(p.filed_basis * Usd::from(leg_sat) / Usd::from(p.tranche_sat));
    let documented_share = usd_basis_share - estimate_share; // UNCLAMPED (a documented fee carry ≥ 0)
    let estimate_basis = estimate_share.min((net_proceeds_share - documented_share).max(Usd::ZERO));
    documented_share + estimate_basis
}

/// The crypto-attributable federal tax for `year` under `state`, or `None` when the year is not
/// computable (missing table/profile / a Hard blocker). Mirrors `conservative::tax_total` (private
/// there); `consent_terms`'s fold-pair delta is a clean `without − with` cancellation over this.
fn crypto_tax_of(
    events: &[LedgerEvent],
    state: &LedgerState,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
) -> Option<Usd> {
    match compute_tax_year(events, state, year, profile, tables) {
        TaxOutcome::Computed(r) => Some(r.total_federal_tax_attributable),
        TaxOutcome::NotComputable(_) => None,
    }
}

/// The whole-tranche `sat` DENOMINATOR for `tranche_id` (its `DeclareTranche.sat`), the pro-ration
/// denominator for the undisposed hypothetical. Falls back to `remaining` (⇒ the full `filed_basis` as
/// the pro-rata floor) only if the target is absent — which never happens for a real promote target.
fn declare_tranche_sat(events: &[LedgerEvent], tranche_id: &EventId, remaining: Sat) -> Sat {
    events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::DeclareTranche(t) if e.id == *tranche_id => Some(t.sat),
            _ => None,
        })
        .unwrap_or(remaining)
}

/// BG-D6 (Task 9): the two-sided informed-consent figures the filer sees on the promotion screen AND that
/// are snapshotted verbatim into `Acknowledgment.shown_terms` (T1). Promoting `tranche_id` to the
/// whole-tranche `filed_basis` floor can rewrite a filed year's realized gain (a below-window-low sale
/// files a CLAMPED $0 gain, never a loss), reorder a prior year's disposals/donations, and shift §1212(b)/
/// §170(d) carryovers into later years — this quantifies each effect as a `ConsentTerm`.
///
/// ★ The figures re-fold through the **clamped promoted path**, not the raw `overpayment_delta_one` basis
/// swap: a SYNTHETIC `PromoteTranche(tranche_id, filed_basis)` is threaded into the ledger and re-projected
/// so the T3 rewrite AND the BG-D4 disposal/removal clamp bind (a $8k sale below a $12k floor quotes the
/// saving on the CLAMPED $0 gain, never the $4k loss the promote could not file). Consent runs BEFORE the
/// promote is recorded, so `events` carries no promote yet — the baseline fold is `events` as-is.
///
/// The year set ranges over EVERY year the fold-diff flags, INCLUDING the current/latest year (no
/// `< current` filter — a sell-earlier-this-year-then-promote has its current-year realized delta quoted).
/// Per flagged year: both folds price it (matching table + profile) ⇒ `ComputedTax { delta_usd (the tax
/// SAVING = tax_without − tax_with), deduction_delta_usd }`, where `deduction_delta_usd = Some(the removal
/// effect engine B CANNOT price — §170 charitable-deduction + §1015 gift-basis reduction)` when a removal
/// leg diffed, else `None`. Engine B EXCLUDES crypto donations, so a DONATION-only computing year prices at
/// `delta_usd: 0` while `deduction_delta_usd: Some(D≠0)` — CORRECT, not a bare $0 (only `{delta:0,
/// deduction:None}` for a real change is forbidden; a pure 8949-date swap with no money is skipped, its
/// dates named by the T8 prose advisory). An uncomputable year ⇒ `Uncomputable { gain_delta_usd,
/// deduction_delta_usd }` (profile-free from the fold pair), never a silent $0.
///
/// Undisposed tranche sats ⇒ `Unrealized { sat, hypothetical_reduction, as_of }` — NEVER a bare
/// nothing (a fully-undisposed promote flags no year, so this is what keeps the screen non-empty).
/// `hypothetical_reduction = Some(min(today-proceeds, the pro-rata floor))` = the CLAMPED gain reduction if
/// you sold the remainder at the current close, else `None` (the render names the floor itself as the max).
/// "Today" is the deterministic, clock-free `as_of`: the ledger's latest recorded event date (core carries
/// no wall clock); bundled prices often lack a close there ⇒ the `None` fallback.
///
/// A carryover-affecting flagged year (net capital gain/loss OR charitable deduction moved) reshapes LATER
/// years' §1212(b)/§170(d) carryover-in, which the per-year engine CANNOT chain (it reads a static profile
/// `carryforward_in`) ⇒ `CascadeNamed { year }` for each later ACTIVITY year the promote does not itself
/// reorder (unflagged). NOTHING is written and NOTHING `>$0` is filed here — this feeds the consent screen.
#[allow(clippy::too_many_arguments)]
pub fn consent_terms(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    tranche_id: &EventId,
    filed_basis: Usd,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
) -> Vec<ConsentTerm> {
    // The fold pair. `without` = the events AS THEY STAND (the $0-tranche baseline; the promote is not yet
    // recorded). `with` = the same events PLUS a synthetic `PromoteTranche(tranche_id, filed_basis)`,
    // applied through the T3 rewrite path so the BG-D4 clamp binds (★ the clamped promoted path, not the
    // un-clamped `overpayment_delta_one` swap).
    let without_state = project(events, prices, config);
    let with_events = with_synthetic_promote(events, tranche_id, filed_basis);
    let with_state = project(&with_events, prices, config);

    let mut terms: Vec<ConsentTerm> = Vec::new();

    // Candidate years: every year in EITHER fold's disposals ∪ removals — INCLUDING the current/latest year
    // (no `< current` filter, tax r3 I-1).
    let mut years: BTreeSet<i32> = BTreeSet::new();
    for st in [&with_state, &without_state] {
        for d in &st.disposals {
            years.insert(d.disposed_at.year());
        }
        for r in &st.removals {
            years.insert(r.removed_at.year());
        }
    }

    // The earliest flagged year whose net capital gain/loss or charitable deduction moved (§1212(b)/§170(d)
    // carryover source), and the set of ALL flagged years (so the cascade names only UNFLAGGED later years).
    let mut carryover_source: Option<i32> = None;
    let mut flagged: BTreeSet<i32> = BTreeSet::new();

    for &y in &years {
        // Per-year leg SETS (whole Disposal/Removal; a Vec-eq catches a HIFO reorder AND an equal-basis /
        // different-date swap — the BG-D9 corner a Σ-compare misses). Mirrors `promote_prior_year_advisory`.
        let disp = |st: &LedgerState| -> Vec<Disposal> {
            st.disposals
                .iter()
                .filter(|d| d.disposed_at.year() == y)
                .cloned()
                .collect()
        };
        let rem = |st: &LedgerState, k: RemovalKind| -> Vec<Removal> {
            st.removals
                .iter()
                .filter(|r| r.removed_at.year() == y && r.kind == k)
                .cloned()
                .collect()
        };
        let disp_changed = disp(&with_state) != disp(&without_state);
        let don_changed =
            rem(&with_state, RemovalKind::Donation) != rem(&without_state, RemovalKind::Donation);
        let gift_changed =
            rem(&with_state, RemovalKind::Gift) != rem(&without_state, RemovalKind::Gift);
        if !(disp_changed || don_changed || gift_changed) {
            continue; // this year's filed content is unchanged by the promote
        }
        flagged.insert(y);

        // Profile-free per-year deltas, oriented so a POSITIVE figure is what the promote REDUCES
        // (without − with): gain over disposal legs, §170(e) deduction over donations, §1015 basis over gifts.
        let gain = |st: &LedgerState| -> Usd {
            st.disposals
                .iter()
                .filter(|d| d.disposed_at.year() == y)
                .flat_map(|d| &d.legs)
                .map(|l| l.gain)
                .sum()
        };
        let ded = |st: &LedgerState| -> Usd {
            st.removals
                .iter()
                .filter(|r| r.removed_at.year() == y && r.kind == RemovalKind::Donation)
                .filter_map(|r| r.claimed_deduction)
                .sum()
        };
        let gift_basis = |st: &LedgerState| -> Usd {
            st.removals
                .iter()
                .filter(|r| r.removed_at.year() == y && r.kind == RemovalKind::Gift)
                .flat_map(|r| &r.legs)
                .map(|l| l.basis)
                .sum()
        };
        let gain_delta = gain(&without_state) - gain(&with_state);
        let don_delta = ded(&without_state) - ded(&with_state); // §170 charitable deduction reduction
        let gift_delta = gift_basis(&without_state) - gift_basis(&with_state); // §1015 carryover reduction
                                                                               // The removal effect engine B CANNOT price (tax r3 I-2): the charitable-deduction change PLUS the
                                                                               // §1015 gift-basis change. `Some` iff a removal leg actually diffed.
        let removal_delta = don_delta + gift_delta;
        let removal_diffed = don_changed || gift_changed;

        // Carryover source: net capital gain/loss changed (§1212(b)) OR the charitable deduction changed
        // (§170(d)). A gift never touches a 1040 line, so it is EXCLUDED here.
        if (disp_changed && gain_delta != Usd::ZERO) || (don_changed && don_delta != Usd::ZERO) {
            carryover_source = Some(carryover_source.map_or(y, |c| c.min(y)));
        }

        // Computable iff BOTH folds price `y` (matching table + profile). A donation-only year prices with
        // delta_usd == 0 because engine B excludes crypto donations — the effect rides `deduction_delta_usd`.
        match (
            crypto_tax_of(events, &with_state, y, profile, tables),
            crypto_tax_of(events, &without_state, y, profile, tables),
        ) {
            (Some(t_with), Some(t_without)) => {
                let delta_usd = t_without - t_with; // the federal-tax SAVING from promoting
                let deduction_delta_usd = if removal_diffed {
                    Some(removal_delta)
                } else {
                    None
                };
                // A bare `{delta:0, deduction:None}` (a real change with no money and no removal — a pure
                // 8949-date swap) is FORBIDDEN; it is out of consent_terms' money-scope (the T8 prose
                // advisory names those dates), so skip it rather than emit the "bare $0" defect.
                if delta_usd != Usd::ZERO || deduction_delta_usd.is_some() {
                    terms.push(ConsentTerm::ComputedTax {
                        year: y,
                        delta_usd,
                        deduction_delta_usd,
                    });
                }
            }
            _ => {
                // Uncomputable (no table/profile/blocked): the profile-free fold-pair deltas, never a
                // silent $0. Skip only the all-zero degenerate date-swap.
                if gain_delta != Usd::ZERO || removal_delta != Usd::ZERO {
                    terms.push(ConsentTerm::Uncomputable {
                        year: y,
                        gain_delta_usd: gain_delta,
                        deduction_delta_usd: removal_delta,
                    });
                }
            }
        }
    }

    // Cascade (§1212(b)/§170(d), named-unquantified): a carryover-affecting flagged year reshapes LATER
    // years' carryover-in, which the per-year engine cannot chain (it reads a static profile
    // `carryforward_in`). Name each later ACTIVITY year the promote does NOT itself reorder (unflagged) —
    // an already-flagged later year is quantified by its own term.
    if let Some(src) = carryover_source {
        for &y in &years {
            if y > src && !flagged.contains(&y) {
                terms.push(ConsentTerm::CascadeNamed { year: y });
            }
        }
    }

    // Undisposed tranche sats ⇒ the UNREALIZED hypothetical (the with-promote holding). NEVER a bare
    // nothing: a fully-undisposed promote flags no year, so this is what keeps the screen non-empty.
    let remaining: Sat = with_state
        .lots
        .iter()
        .filter(|l| l.lot_id.origin_event_id == *tranche_id)
        .map(|l| l.remaining_sat)
        .sum();
    if remaining > 0 {
        let tranche_sat = declare_tranche_sat(events, tranche_id, remaining);
        let (hypothetical_reduction, as_of) =
            unrealized_reduction(events, prices, filed_basis, tranche_sat, remaining);
        terms.push(ConsentTerm::Unrealized {
            sat: remaining,
            hypothetical_reduction,
            as_of,
        });
    }

    terms
}

/// Thread a SYNTHETIC `PromoteTranche(tranche_id, filed_basis)` onto `events` so a fresh `project` binds
/// the T3 rewrite + BG-D4 clamp. The decision id is `max(existing Decision seq) + 1` (collision-free); the
/// consent/attestation fields are placeholders (they never affect the projection — a `PromoteTranche`
/// folds as `Op::Skip`, and `live_promotes` reads only `target`/`filed_basis`, order-independent).
fn with_synthetic_promote(
    events: &[LedgerEvent],
    tranche_id: &EventId,
    filed_basis: Usd,
) -> Vec<LedgerEvent> {
    let seq = events
        .iter()
        .filter_map(|e| match e.id {
            EventId::Decision { seq } => Some(seq),
            _ => None,
        })
        .max()
        .map_or(1, |m| m + 1);
    let utc = events
        .iter()
        .map(|e| e.utc_timestamp)
        .max()
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let mut out = events.to_vec();
    out.push(LedgerEvent {
        id: EventId::Decision { seq },
        utc_timestamp: utc,
        original_tz: UtcOffset::UTC,
        wallet: None,
        payload: EventPayload::PromoteTranche(PromoteTranche {
            target: tranche_id.clone(),
            method: FloorMethod::WindowLowClose,
            filed_basis,
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: String::new(),
                shown_terms: Vec::new(),
                provenance_text: String::new(),
                provenance_version: String::new(),
            },
            part_ii_narrative: String::new(),
        }),
    });
    out
}

/// Task 11 (§3 item 3 / tax r1 I-3): the CLAMPED federal-tax saving for `year` from promoting `tranche_id`
/// to `filed_basis`, through the SAME synthetic-promote clamped path `consent_terms` uses (the T3 rewrite
/// AND the BG-D4 disposal/removal clamp bind) — NOT the un-clamped `overpayment_delta_one` basis swap.
/// Returns `tax_without − tax_with` for `year`, clamped `≥ $0`; `$0` when `year` is not computable in
/// either fold (no table/profile/blocked). A below-window-low sale therefore quotes the saving on the
/// CLAMPED `$0` gain, never the loss the promote could not file. NOTHING is written and NOTHING `>$0` is
/// filed — this feeds the overpayment funnel line only. `events` carries no promote yet (the baseline
/// fold is `events` as-is; the with-fold threads a synthetic promote).
#[allow(clippy::too_many_arguments)]
pub fn clamped_promote_year_saving(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    tranche_id: &EventId,
    filed_basis: Usd,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
) -> Usd {
    let without_state = project(events, prices, config);
    let with_events = with_synthetic_promote(events, tranche_id, filed_basis);
    let with_state = project(&with_events, prices, config);
    match (
        crypto_tax_of(events, &with_state, year, profile, tables),
        crypto_tax_of(events, &without_state, year, profile, tables),
    ) {
        (Some(t_with), Some(t_without)) => (t_without - t_with).max(Usd::ZERO),
        _ => Usd::ZERO,
    }
}

/// The undisposed hypothetical (tax r3 N-2): if the `remaining` tranche sats were sold at the current
/// close, the CLAMPED gain reduction promoting would provide, and the `as_of` date it was priced at.
/// `Some(min(today-proceeds, the pro-rata floor))` when a close exists at the clock-free `as_of` (the
/// ledger's latest event date); `None` when it does not (the render then names the floor itself as the
/// theoretical max, since a $0-basis unit's gain can be reduced by at most its whole filed floor).
fn unrealized_reduction(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    filed_basis: Usd,
    tranche_sat: Sat,
    remaining: Sat,
) -> (Option<Usd>, Option<TaxDate>) {
    let prorata_floor = if tranche_sat > 0 {
        round_cents(filed_basis * Usd::from(remaining) / Usd::from(tranche_sat))
    } else {
        filed_basis
    };
    let as_of = events
        .iter()
        .map(|e| tax_date(e.utc_timestamp, e.original_tz))
        .max();
    match as_of.and_then(|d| prices.usd_per_btc(d).map(|px| (d, px))) {
        Some((d, px)) => {
            let today_proceeds = round_cents(px * Usd::from(remaining) / Usd::from(SATS_PER_BTC));
            // Clamped: a $0-basis unit's gain can be reduced by at most the proceeds (never below $0) and
            // never by more than the filed floor — exactly the BG-D4 leg clamp, applied to a today sale.
            (Some(today_proceeds.min(prorata_floor)), Some(d))
        }
        None => (None, None),
    }
}
