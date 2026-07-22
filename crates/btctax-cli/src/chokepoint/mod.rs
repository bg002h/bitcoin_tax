//! Defensive Filing Wizard, sub-project-2 Phase P-A, Task 1 — the PROMOTE chokepoint: a reusable
//! plan/confirm/apply pipeline extracted VERBATIM from the shipped CLI verb
//! (`cmd::promote::promote_tranche`, Approach-B Task 10, `cmd/promote.rs:364-488`) so a future TUI can
//! drive the EXACT SAME gated pipeline as the CLI. `cmd::promote::promote_tranche` is now a thin driver
//! over this module: `Session::open` → build args → `plan_promote` (map `Refusal` → `CliError`) →
//! `println!("{}", render_consent(&plan))` → prompt/collect ack → `apply_promote`.
//!
//! **Gate ordering (DFW-D2) MUST match `cmd/promote.rs:378-485` exactly:** resolve-live → BG-D5
//! provenance → BG-D7 Part II → BG-D3 floor/coverage → BG-D6 `consent_terms` → synthetic-promote advisory
//! → gift-only relabel → consent render (incl. `wide_window_note`) → **ack inside `apply_promote`,
//! fail-closed** → `would_conflict` → append.
//!
//! ★ **I-1 (byte-parity):** the shipped verb prints, IN ORDER (`promote.rs:443-455`), the
//! synthetic-promote ADVISORY (pre-consent) → `render_consent(&terms, &gift_only_years)` → the
//! `wide_window_note` (post-consent). `PromotePlan` therefore carries THREE ordered pieces
//! (`advisory_lines`, `gift_only_years`, `post_consent_note`) so this module's `render_consent(&plan)`
//! reproduces `advisory → consent → note` byte-for-byte when printed via a single `println!` — a single
//! flat `Vec` cannot place `terms` BETWEEN the pre-advisory and the note; do NOT collapse the three. The
//! shipped `render_consent(terms, gift_only_years)` stays in `cmd::promote` (still `pub` — external KATs
//! in `tests/promote_cli.rs` call it directly — and is invoked from here); `gift_only_flagged_years`/
//! `wide_window_note` move HERE.
//!
//! ★ **DFW-D6 (the ONE intended behavior change — the sub-1 pseudo-off fix):** `plan_promote` forces
//! `cfg.pseudo_reconcile = false` on its own COPY (`ProjectionConfig` is `Copy`) before `consent_terms` /
//! `promote_prior_year_advisory` / `gift_only_flagged_years` — mirroring `would_conflict`
//! (`project/mod.rs:118`). Without this, a pseudo-active vault's consent screen — and the RECORDED
//! `Acknowledgment.shown_terms`, the §6664(c) good-faith artifact — could fold in a synthetic default that
//! was never persisted, misstating what the filer actually acknowledged.
//!
//! ★ **arch-m-6/tax-N-1:** `Refusal::Target` covers the resolve-live gate — unknown/voided/wrong-type
//! target only (`resolve_live_tranche`). Already-promoted (a DOUBLE promote) is NOT caught here; it
//! surfaces as `would_conflict` at APPLY time (a `CliError`, never a plan `Refusal`) — mirroring
//! `promote.rs:475-483`.
//!
//! ★ **arch-m-new-3:** `plan_promote` takes no `Session`/`state` — the shipped pipeline rebuilds
//! everything from `events` (`promote.rs:364-488`) — so a caller (CLI or future TUI) supplies its own
//! already-loaded `events`/`prices`/`cfg`.

use crate::cmd::promote::{
    render_consent as render_consent_terms, ProvenanceKind, PROMOTE_ACK_PHRASE, PROVENANCE_TEXT,
    PROVENANCE_VERSION,
};
use crate::{CliError, Session};
use btctax_core::conservative::{self, Direction};
use btctax_core::conservative_promote::{self, PromoteRefusal};
use btctax_core::conventions::tax_date;
use btctax_core::event::{
    Acknowledgment, ConsentTerm, DeclareTranche, EventPayload, FloorMethod, PromoteTranche,
};
use btctax_core::persistence::{append_decision, load_all};
use btctax_core::price::PriceProvider;
use btctax_core::project::ProjectionConfig;
use btctax_core::{project, EventId, LedgerEvent, RemovalKind, TaxDate, Usd};
use std::collections::BTreeSet;
use time::{OffsetDateTime, UtcOffset};

/// Everything computed BEFORE the filer types the acknowledgment phrase (the `PromoteTranche` decision
/// id, `target`, is not yet known — it is assigned at `apply_promote`'s `append_decision`). ★ I-1: the
/// three ordered fields (`advisory_lines`, `gift_only_years`, `post_consent_note`) let `render_consent`
/// reproduce the shipped verb's `advisory → consent → note` byte order — do NOT collapse them into one
/// `Vec` or pre-render `gift_only_years` into a string.
#[derive(Debug, Clone)]
pub struct PromotePlan {
    /// The `DeclareTranche` decision this promotes (BG-D1) — the `PromoteTranche.target` field.
    pub target: EventId,
    /// BG-D6 `consent_terms` output — ALSO snapshotted verbatim onto `payload`'s
    /// `Acknowledgment.shown_terms` (the §6664(c) good-faith artifact).
    pub terms: Vec<ConsentTerm>,
    /// The PRE-consent synthetic-promote advisory lines (`promote.rs:443`, `for line in &advisory`).
    pub advisory_lines: Vec<String>,
    /// T9 handoff: an INPUT to the shipped `render_consent(terms, gift_only_years)`
    /// (`promote.rs:333`/`:453`) — NOT a pre-rendered string.
    pub gift_only_years: BTreeSet<i32>,
    /// `wide_window_note`, printed AFTER the consent screen (`promote.rs:454`).
    pub post_consent_note: Option<String>,
    /// The `PromoteTranche` payload `apply_promote` appends on a successful acknowledgment.
    pub payload: EventPayload,
}

/// A `plan_promote` refusal — fail-closed, BEFORE any computation past the failing gate. Each variant
/// carries the exact filer-facing message (byte-identical to the shipped verb's `CliError::Usage` text),
/// so mapping to `CliError` (the thin CLI driver) or a distinct TUI error surface is trivial either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The resolve-live gate (`resolve_live_tranche`): `target` is absent, wrong-type, or voided.
    Target(String),
    /// BG-D5: a non-`Purchase` provenance.
    Provenance(String),
    /// BG-D3: `filed_basis_for` could not produce a trustworthy floor (`NoCoverage`/`PartialCoverage`).
    Coverage(String),
    /// BG-D7: an empty/whitespace Form 8275 Part II narrative.
    PartII(String),
}

impl From<Refusal> for CliError {
    fn from(r: Refusal) -> CliError {
        let msg = match r {
            Refusal::Target(m) => m,
            Refusal::Provenance(m) => m,
            Refusal::Coverage(m) => m,
            Refusal::PartII(m) => m,
        };
        CliError::Usage(msg)
    }
}

/// True iff a live (non-voided) `VoidDecisionEvent` names `id`. Moved verbatim from `cmd/promote.rs`.
fn is_voided(events: &[LedgerEvent], id: &EventId) -> bool {
    events.iter().any(
        |e| matches!(&e.payload, EventPayload::VoidDecisionEvent(v) if v.target_event_id == *id),
    )
}

/// Resolve `target_event_id` to a LIVE (present, non-voided) `DeclareTranche`, or `Refusal::Target`. A
/// record-time convenience guard — the engine's own `DecisionConflict` adjudication is the backstop for
/// any target this misses (moved verbatim from `cmd/promote.rs::resolve_live_tranche`, DFW-D2 gate 1).
fn resolve_live_tranche(
    events: &[LedgerEvent],
    target_event_id: &EventId,
) -> Result<DeclareTranche, Refusal> {
    let not_live = || {
        Refusal::Target(format!(
            "{} is not a live DeclareTranche (absent, wrong type, or voided) — see `btctax events list` \
             for event refs + decision status",
            target_event_id.canonical()
        ))
    };
    if is_voided(events, target_event_id) {
        return Err(not_live());
    }
    events
        .iter()
        .find(|e| e.id == *target_event_id)
        .and_then(|e| match &e.payload {
            EventPayload::DeclareTranche(t) => Some(t.clone()),
            _ => None,
        })
        .ok_or_else(not_live)
}

/// BG-D5: refuse a non-`Purchase` provenance — the closed enumeration, fail-closed, before any
/// computation. Moved verbatim from `cmd/promote.rs::refuse_non_purchase`.
fn refuse_non_purchase(provenance: ProvenanceKind) -> Refusal {
    Refusal::Provenance(format!(
        "promote-tranche requires purchase provenance: {PROVENANCE_TEXT}. This tranche was declared as \
         acquired by {label} — a {label} recipient already has a documented, real basis (income \
         FMV-at-receipt, or donor/decedent carryover) — model the real acquisition instead (a documented \
         Acquire/Income/gift-received event), not a conservative-filing tranche promote.",
        label = provenance.label(),
    ))
}

/// BG-D3: translate a `filed_basis_for` refusal into a record-time message. Moved verbatim from
/// `cmd/promote.rs::refuse_no_floor`.
fn refuse_no_floor(e: PromoteRefusal, window_start: TaxDate, window_end: TaxDate) -> Refusal {
    let detail = match e {
        PromoteRefusal::NoCoverage => {
            "no bundled daily-close price exists anywhere in the window — never fabricate a floor over a \
             total data gap"
        }
        PromoteRefusal::PartialCoverage => {
            "the window has a gap in bundled daily-close data — the covered-part minimum is not provably \
             the window's true minimum, so it cannot be filed as a trustworthy floor"
        }
    };
    Refusal::Coverage(format!(
        "cannot compute a promotion floor for the window [{window_start}, {window_end}]: {detail}. \
         Narrow the window to a fully-covered range, or leave this tranche at its filed $0 basis."
    ))
}

/// A filer-facing caution (SPEC §1, "two honest limits"): a wide acquisition window yields a LOW
/// ("trivial") floor relative to a tight one. Purely informational, non-gating; conditioned on the
/// window exceeding one year. Moved verbatim from `cmd/promote.rs::wide_window_note`.
fn wide_window_note(window_start: TaxDate, window_end: TaxDate) -> Option<String> {
    let days = (window_end - window_start).whole_days();
    if days > 365 {
        Some(format!(
            "note: this tranche's declared window spans {days} days (over a year). A WIDE window tends \
             to produce a LOW (\"trivial\") floor relative to a tight one — for some filers it may be \
             simpler, and just as conservative, to leave this tranche at its filed $0 basis and skip the \
             Form 8275 disclosure surface entirely."
        ))
    } else {
        None
    }
}

/// Thread ONE synthetic `PromoteTranche(tranche_id, filed_basis)` onto `events` (mirrors
/// `conservative_promote::with_synthetic_promote`, private there). Moved verbatim from
/// `cmd/promote.rs::with_synthetic_promote`.
fn with_synthetic_promote(
    events: &[LedgerEvent],
    tranche_id: &EventId,
    filed_basis: Usd,
    now: OffsetDateTime,
) -> Vec<LedgerEvent> {
    let seq = events
        .iter()
        .filter_map(|e| match e.id {
            EventId::Decision { seq } => Some(seq),
            _ => None,
        })
        .max()
        .map_or(1, |m| m + 1);
    let mut out = events.to_vec();
    out.push(LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: now,
        original_tz: UtcOffset::UTC,
        wallet: None,
        payload: EventPayload::PromoteTranche(PromoteTranche {
            target: tranche_id.clone(),
            method: FloorMethod::WindowLowClose,
            filed_basis,
            coverage: conservative::Coverage::Full,
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

/// T9 handoff (progress.md Task 9): `consent_terms`/`Uncomputable` sum the §170(e) charitable-deduction
/// change and the §1015 gift-basis change into ONE `deduction_delta_usd` figure per year. This re-derives
/// which flagged years are GIFT-only directly from the SAME with/without fold pair the T8 advisory
/// already builds. Moved verbatim from `cmd/promote.rs::gift_only_flagged_years`.
fn gift_only_flagged_years(
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    events: &[LedgerEvent],
    with_events: &[LedgerEvent],
) -> BTreeSet<i32> {
    let without_state = project(events, prices, config);
    let with_state = project(with_events, prices, config);

    let mut years: BTreeSet<i32> = BTreeSet::new();
    for st in [&with_state, &without_state] {
        for r in &st.removals {
            years.insert(r.removed_at.year());
        }
    }

    let rem =
        |st: &btctax_core::LedgerState, y: i32, k: RemovalKind| -> Vec<btctax_core::Removal> {
            st.removals
                .iter()
                .filter(|r| r.removed_at.year() == y && r.kind == k)
                .cloned()
                .collect()
        };

    years
        .into_iter()
        .filter(|&y| {
            let gift_changed =
                rem(&with_state, y, RemovalKind::Gift) != rem(&without_state, y, RemovalKind::Gift);
            let don_changed = rem(&with_state, y, RemovalKind::Donation)
                != rem(&without_state, y, RemovalKind::Donation);
            gift_changed && !don_changed
        })
        .collect()
}

/// The acknowledgment gate (BG-D6) — a PURE exact-compare, no I/O (mirrors `require_attestation`,
/// `lib.rs:208`). Moved verbatim from `cmd/promote.rs::require_promote_ack`; now called from
/// `apply_promote`, fail-closed, BEFORE `would_conflict`/append.
fn require_promote_ack(acknowledge: Option<&str>) -> Result<(), CliError> {
    match acknowledge.map(str::trim) {
        Some(p) if p == PROMOTE_ACK_PHRASE => Ok(()),
        Some(_) => Err(CliError::Usage(format!(
            "the acknowledgment phrase did not match. Type it EXACTLY (trimmed, case-sensitive): {PROMOTE_ACK_PHRASE:?}."
        ))),
        None => Err(CliError::Usage(format!(
            "promote-tranche requires acknowledging the estimated-basis risk shown above — pass \
             --i-acknowledge {PROMOTE_ACK_PHRASE:?} (or type it at the interactive prompt)."
        ))),
    }
}

/// Plan a `PromoteTranche` decision — the DFW-D2 gate order, everything computable BEFORE the filer types
/// the acknowledgment phrase: resolve-live → BG-D5 provenance → BG-D7 Part II → BG-D3 floor/coverage →
/// BG-D6 `consent_terms` → synthetic-promote advisory → gift-only relabel. `events`/`prices`/`cfg` are the
/// caller's own already-loaded state (arch-m-new-3: no `Session` here — the CLI's thin driver and a
/// future TUI each supply their own).
pub fn plan_promote(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    cfg: &ProjectionConfig,
    target: &EventId,
    provenance: ProvenanceKind,
    part_ii: &str,
    now: OffsetDateTime,
) -> Result<PromotePlan, Refusal> {
    // Resolve + assert live (BG-D1).
    let tranche = resolve_live_tranche(events, target)?;

    // BG-D5: purchase provenance only — fail-closed, before any computation.
    if provenance != ProvenanceKind::Purchase {
        return Err(refuse_non_purchase(provenance));
    }

    // BG-D7: an empty/whitespace Part II narrative is refused at record time (present-by-construction).
    if part_ii.trim().is_empty() {
        return Err(Refusal::PartII(
            "promote-tranche requires a non-empty Form 8275 Part II narrative (filer facts, Reg. \
             §1.6662-4(f) — 'in sufficient detail') — pass --part-ii-file pointing at a file with real \
             acquisition/window facts, not an empty or blank file"
                .into(),
        ));
    }

    // BG-D3: the computed whole-tranche filed_basis floor — hard-refuse on Partial/No coverage.
    let floor = conservative_promote::filed_basis_for(
        prices,
        tranche.sat,
        tranche.window_start,
        tranche.window_end,
    )
    .map_err(|e| refuse_no_floor(e, tranche.window_start, tranche.window_end))?;

    // ★ DFW-D6 (the ONE intended behavior change): force pseudo OFF on an own COPY (ProjectionConfig is
    // Copy) before consent_terms / promote_prior_year_advisory / gift_only_flagged_years — mirrors
    // `would_conflict` (`project/mod.rs:118`). The recorded Acknowledgment.shown_terms must always
    // reflect the HONEST (non-synthetic) figures, never a pseudo-active default folded in.
    let mut honest_cfg = *cfg;
    honest_cfg.pseudo_reconcile = false;

    let tables = btctax_adapters::BundledTaxTables::load();
    // A single stored TaxProfile cannot fit the multi-year span this consent/advisory ranges over, so
    // `None` is passed throughout — mirrors the void-direction path (`cmd/reconcile.rs`
    // `promote_void_advisory_lines`): the tax-Δ arm falls back to the gain/deduction-Δ sign, and the
    // amend direction is still correct.
    let terms = conservative_promote::consent_terms(
        events,
        prices,
        &honest_cfg,
        target,
        floor.filed_basis,
        None,
        &tables,
    );

    // Thread ONE synthetic promote so the Direction::Promote advisory AND this layer's own
    // gift-vs-donation year classification (T9 handoff) see the SAME post-promote fold.
    let with_events = with_synthetic_promote(events, target, floor.filed_basis, now);
    let synthetic_id = with_events
        .last()
        .expect("with_synthetic_promote always pushes exactly one event")
        .id
        .clone();

    // T8 handoff (progress.md): `current` is the injected `now`'s tax year (the BTCTAX_NOW seam) — NEVER
    // a wall clock. Years `< current` are presumed already filed; the year still being authored
    // (>= current) is excluded, so it is never told it needs an amended return.
    let current = tax_date(now, UtcOffset::UTC).year();
    let advisory_lines = conservative::promote_prior_year_advisory(
        &with_events,
        prices,
        &honest_cfg,
        &synthetic_id,
        Direction::Promote,
        None,
        &tables,
        current,
    );

    // T9 handoff: which flagged years are GIFT-ONLY (no donation) — relabels that year's deduction/basis-Δ
    // as a §1015 donee-basis change, never Schedule-A, in the consent screen below.
    let gift_only_years = gift_only_flagged_years(prices, &honest_cfg, events, &with_events);

    let payload = EventPayload::PromoteTranche(PromoteTranche {
        target: target.clone(),
        method: FloorMethod::WindowLowClose,
        filed_basis: floor.filed_basis,
        coverage: floor.coverage,
        provenance_attested: true,
        acknowledgment: Acknowledgment {
            phrase: PROMOTE_ACK_PHRASE.to_string(),
            shown_terms: terms.clone(),
            provenance_text: PROVENANCE_TEXT.to_string(),
            provenance_version: PROVENANCE_VERSION.to_string(),
        },
        part_ii_narrative: part_ii.to_string(),
    });

    Ok(PromotePlan {
        target: target.clone(),
        terms,
        advisory_lines,
        gift_only_years,
        post_consent_note: wide_window_note(tranche.window_start, tranche.window_end),
        payload,
    })
}

/// Re-emit the shipped verb's ordered filer-visible text: `advisory_lines` → the shipped
/// `render_consent(&plan.terms, &plan.gift_only_years)` (`cmd::promote`) → `post_consent_note` — ★ I-1:
/// byte-identical to `promote.rs:443-455` when the RESULT is printed via a single
/// `println!("{}", render_consent(&plan))` (the shipped verb instead used three separate `println!`
/// calls; a single combined string reproduces the exact same stdout bytes because `println!` always adds
/// exactly one trailing `\n`). Do NOT collapse this into `plan.terms`/`plan.gift_only_years` alone — the
/// pre-advisory must land BEFORE the consent screen and the note AFTER it.
pub fn render_consent(plan: &PromotePlan) -> String {
    let mut out = String::new();
    for line in &plan.advisory_lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&render_consent_terms(&plan.terms, &plan.gift_only_years));
    if let Some(note) = &plan.post_consent_note {
        out.push('\n');
        out.push_str(note);
    }
    out
}

/// Apply a planned promote: the acknowledgment gate (BG-D6, fail-closed, INSIDE apply) → `would_conflict`
/// pre-check (BG-D9 — a second live promote on this target, or any other resolver-level conflict; refuses
/// BEFORE appending, NOT last-wins) → append + save. Reloads `events`/`cfg` fresh from `session`
/// (arch-m-new-3: `plan_promote` took no `Session`) — a single synchronous CLI/TUI invocation cannot
/// append anything between `plan_promote` and `apply_promote`, so this is behavior-preserving.
pub fn apply_promote(
    session: &mut Session,
    plan: PromotePlan,
    acknowledge: Option<&str>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    require_promote_ack(acknowledge)?;

    let events = load_all(session.conn())?;
    let cfg = session.config()?.to_projection();

    // BG-D9: pre-check `would_conflict` (a second live promote on this target, or any other resolver-level
    // conflict, e.g. UX-P4-3) — refuse BEFORE appending (fail-closed). NOT last-wins.
    if let Some(detail) =
        btctax_core::would_conflict(&events, session.prices(), &cfg, &plan.payload, now)
    {
        return Err(CliError::Usage(format!(
            "cannot record this promote — a decision conflict: {detail}"
        )));
    }

    let id = append_decision(session.conn(), plan.payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}
