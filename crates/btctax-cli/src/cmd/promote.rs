//! Approach-B / Task 10 — the `promote-tranche` CLI verb: BG-D5 (record-time purchase-provenance
//! attestation), BG-D6 (two-sided informed-consent recording), and BG-D7 (Form 8275 Part II present-by-
//! construction). Mirrors `cmd/tranche.rs::declare_tranche` end-to-end: resolve inputs, guard at record
//! time (fail-closed, nothing appended on refusal), append exactly one decision, save.
//!
//! Order of operations (SPEC §BG-D5/D6/D7, arch/tax r1 I-3): resolve the target → assert it is a LIVE
//! `DeclareTranche` → BG-D5 provenance gate → BG-D7 narrative gate → BG-D3 `filed_basis_for` (hard-refuse
//! on Partial/No coverage) → BG-D6 `consent_terms` → the T8 prior-year advisory (`Direction::Promote` —
//! this is the ONLY promote-direction call site) → the consent screen → the acknowledgment gate → build
//! + would_conflict pre-check → append + save.
use crate::{CliError, Session};
use btctax_core::conservative::{self, Direction};
use btctax_core::conservative_promote::{self, PromoteRefusal};
use btctax_core::conventions::tax_date;
use btctax_core::event::{
    Acknowledgment, ConsentTerm, DeclareTranche, FloorMethod, PromoteTranche,
};
use btctax_core::persistence::{append_decision, load_all};
use btctax_core::price::PriceProvider;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::{EventId, EventPayload, LedgerEvent, RemovalKind, TaxDate, Usd};
use btctax_store::Passphrase;
use std::collections::BTreeSet;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

use crate::eventref::parse_event_id;

/// The units' real acquisition provenance (BG-D5). A CLOSED enumeration: only `Purchase` clears the
/// promote gate — every other value has a documented FMV-at-receipt/carryover basis from the return
/// (Notice 2014-21; Rev. Rul. 2019-24) and is refused, pointed at modeling the real acquisition instead.
/// CLI-facing only (not part of the persisted schema — only `provenance_attested: bool` plus the fixed
/// `PROVENANCE_TEXT`/`PROVENANCE_VERSION` are stored on the event, per T1's `Acknowledgment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ProvenanceKind {
    Purchase,
    Gift,
    Inheritance,
    Mining,
    Earned,
    Airdrop,
    Fork,
}

impl ProvenanceKind {
    fn label(self) -> &'static str {
        match self {
            ProvenanceKind::Purchase => "purchase",
            ProvenanceKind::Gift => "gift",
            ProvenanceKind::Inheritance => "inheritance",
            ProvenanceKind::Mining => "mining",
            ProvenanceKind::Earned => "staking/earning",
            ProvenanceKind::Airdrop => "airdrop",
            ProvenanceKind::Fork => "fork",
        }
    }
}

/// The exact phrase a filer must affirm to RECORD a `promote-tranche` decision (BG-D6). Distinct from the
/// pseudo-export `ATTEST_PHRASE` (`lib.rs`) — promoting is an estimated-basis FILING CHOICE, not an
/// attestation that a fictional draft is being exported on purpose; conflating the two phrases would let a
/// scripted pseudo-export attest ALSO silently satisfy this gate (N-1). Compared TRIMMED, case-SENSITIVE
/// (mirrors `require_attestation`'s compare, `lib.rs:208`).
pub const PROMOTE_ACK_PHRASE: &str = "I understand and accept this estimated-basis risk";

/// BG-D5 attested statement (verbatim, stored on `Acknowledgment.provenance_text`). The affirmative clause
/// is operative; the negative enumeration is CLOSED so it cannot be misread `expressio unius` (tax r1
/// M-6).
pub const PROVENANCE_TEXT: &str = "these units were acquired by purchase within the declared window — \
     not by gift, inheritance, mining, staking/earning, airdrop, fork, or any acquisition other than \
     purchase";
/// Attestation-text version (BG-D5) — bump if `PROVENANCE_TEXT`'s wording ever changes.
pub const PROVENANCE_VERSION: &str = "v1";

/// The consent-screen intro (BG-D6/D10): the penalty base, "plus interest", and the mitigation framing —
/// NEVER "safe harbor" (SPEC BG-D7/D10 — the copy must not use that phrase even to deny it).
const CONSENT_INTRO: &str = "Promoting this tranche is a KNOWING choice to file a >$0 basis floor \
     (the minimum daily closing price over the attested acquisition window) instead of the IRS-fallback \
     $0. If an exam determines the correct basis is $0, the penalty is 20% ordinary / 40% worst-case of \
     the resulting additional tax (the underpayment attributable to the misstatement), plus interest; \
     the Form 8275 disclosure and the good-faith window-low-close methodology mitigate this exposure, \
     but do not eliminate it and do not guarantee immunity from penalty.";

/// True iff a live (non-voided) `VoidDecisionEvent` names `id`.
fn is_voided(events: &[LedgerEvent], id: &EventId) -> bool {
    events.iter().any(
        |e| matches!(&e.payload, EventPayload::VoidDecisionEvent(v) if v.target_event_id == *id),
    )
}

/// Resolve `target_event_id` to a LIVE (present, non-voided) `DeclareTranche`, or refuse. This is a
/// record-time convenience guard — the engine's own `DecisionConflict` adjudication (Task 7) is the
/// backstop for any target this misses (e.g. a target that only goes stale between this check and
/// `append_decision`, which cannot happen within a single synchronous CLI invocation).
fn resolve_live_tranche(
    events: &[LedgerEvent],
    target_event_id: &EventId,
) -> Result<DeclareTranche, CliError> {
    let not_live = || {
        CliError::Usage(format!(
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
/// computation. A miner/earner/airdrop/fork/gift/inheritance recipient has a real, documented basis
/// (income FMV-at-receipt, or donor/decedent carryover) — a promote is never the right tool for it.
fn refuse_non_purchase(provenance: ProvenanceKind) -> CliError {
    CliError::Usage(format!(
        "promote-tranche requires purchase provenance: {PROVENANCE_TEXT}. This tranche was declared as \
         acquired by {label} — a {label} recipient already has a documented, real basis (income \
         FMV-at-receipt, or donor/decedent carryover) — model the real acquisition instead (a documented \
         Acquire/Income/gift-received event), not a conservative-filing tranche promote.",
        label = provenance.label(),
    ))
}

/// BG-D3: translate a `filed_basis_for` refusal into a record-time message.
fn refuse_no_floor(e: PromoteRefusal, window_start: TaxDate, window_end: TaxDate) -> CliError {
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
    CliError::Usage(format!(
        "cannot compute a promotion floor for the window [{window_start}, {window_end}]: {detail}. \
         Narrow the window to a fully-covered range, or leave this tranche at its filed $0 basis."
    ))
}

/// A filer-facing caution (SPEC §1, "two honest limits"): a wide acquisition window yields a LOW
/// ("trivial") floor relative to a tight one, because the window-min daily close only falls as the window
/// widens. Purely informational, non-gating; conditioned on the window exceeding one year.
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
/// `conservative_promote::with_synthetic_promote`, private there — duplicated here so this CLI layer can
/// also classify gift-vs-donation flagged years, below, from the SAME with/without fold pair the T8
/// advisory call needs). The consent/attestation fields are placeholders (never read by `project` — a
/// `PromoteTranche` folds as `Op::Skip`).
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
/// change and the §1015 gift-basis change into ONE `deduction_delta_usd` figure per year. Neither
/// `ConsentTerm` variant carries which flavor occurred, so this CLI layer re-derives it directly from the
/// SAME with/without fold pair the T8 advisory already builds: a year is "gift-only" iff its GIFT removal
/// legs changed and its DONATION removal legs did NOT — the render must label that year's Δ as a §1015
/// donee-basis documentation change, never a Schedule-A deduction.
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

/// Render one `ConsentTerm` line for the consent screen (BG-D6/D10 copy).
fn render_term(term: &ConsentTerm, gift_only_years: &BTreeSet<i32>) -> String {
    match term {
        ConsentTerm::ComputedTax {
            year,
            delta_usd,
            deduction_delta_usd,
        } => {
            let mut line = if *delta_usd >= Usd::ZERO {
                format!(
                    "Year {year}: promoting this tranche SAVES ~${} in computed federal tax.",
                    delta_usd.round_dp(2)
                )
            } else {
                format!(
                    "Year {year}: promoting this tranche INCREASES computed federal tax by ~${}.",
                    (-*delta_usd).round_dp(2)
                )
            };
            if let Some(d) = deduction_delta_usd {
                line.push_str(&format!(
                    " The computed tax figure does NOT capture this charitable-deduction change (priced \
                     only on the full return); its own Δ is ~${}.",
                    d.round_dp(2)
                ));
                if gift_only_years.contains(year) {
                    line.push_str(
                        " That Δ is a donee-basis (§1015) documentation change; the donor's 1040 is \
                         unaffected — NOT a Schedule-A deduction.",
                    );
                }
            }
            line
        }
        ConsentTerm::Uncomputable {
            year,
            gain_delta_usd,
            deduction_delta_usd,
        } => {
            let mut line = format!(
                "Year {year}: tax not computable here (no table/profile/blocked) — promoting changes the \
                 reported gain by ~${} and the deduction/basis by ~${}.",
                gain_delta_usd.round_dp(2),
                deduction_delta_usd.round_dp(2)
            );
            if *deduction_delta_usd != Usd::ZERO && gift_only_years.contains(year) {
                line.push_str(
                    " The deduction/basis figure is a donee-basis (§1015) documentation change; the \
                     donor's 1040 is unaffected — NOT a Schedule-A deduction.",
                );
            }
            line
        }
        ConsentTerm::CascadeNamed { year } => format!(
            "Year {year}: this promote's cross-year effects may also shift that year's §1212(b)/§170(d) \
             carryover-in (named here, not separately quantified)."
        ),
        ConsentTerm::Unrealized {
            sat,
            hypothetical_reduction,
            as_of,
        } => match (hypothetical_reduction, as_of) {
            (Some(r), Some(d)) => format!(
                "{sat} sat remain undisposed: at the {d} close, promoting would reduce a future sale's \
                 reported gain by up to ~${} (hypothetical, not a filed figure) — saving and exposure \
                 accrue only at disposal.",
                r.round_dp(2)
            ),
            _ => format!(
                "{sat} sat remain undisposed: no current bundled price is available — the filed floor \
                 itself is the maximum possible gain reduction on a future sale (hypothetical, not a \
                 filed figure) — saving and exposure accrue only at disposal."
            ),
        },
    }
}

/// The consent screen (BG-D6/D10): the penalty-base/interest/mitigation intro, then one line per
/// `ConsentTerm`. `gift_only_years` (T9 handoff) relabels a gift-only flagged year's deduction/basis-Δ as
/// a §1015 donee-basis change rather than a Schedule-A deduction.
pub fn render_consent(terms: &[ConsentTerm], gift_only_years: &BTreeSet<i32>) -> String {
    let mut out = String::new();
    out.push_str(CONSENT_INTRO);
    for term in terms {
        out.push('\n');
        out.push_str(&render_term(term, gift_only_years));
    }
    out
}

/// The acknowledgment gate (BG-D6) — a PURE exact-compare, no I/O (mirrors `require_attestation`,
/// `lib.rs:208`, the phrase-gate PRECEDENT). `None` (no phrase supplied) and `Some` non-matching are
/// DISTINCT refusals so the CLI can phrase each correctly.
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

/// Append a `PromoteTranche` decision (BG-D1) promoting `target_ref`'s `$0` `DeclareTranche` to a filed
/// `>$0` basis floor, behind the BG-D5 provenance gate, the BG-D7 Part II narrative gate, and the BG-D6
/// informed-consent acknowledgment. `now` is the injected decision creation-time (deterministic in
/// tests) — it ALSO doubles as the clock-free "current tax year" the T8 prior-year advisory is filtered
/// against (years `>= current` are still being authored, not yet filed, so no 1040-X pointer is owed).
pub fn promote_tranche(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    provenance: ProvenanceKind,
    part_ii: String,
    acknowledge: Option<&str>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let target_event_id = parse_event_id(target_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let events = load_all(session.conn())?;

    // Resolve + assert live (BG-D1).
    let tranche = resolve_live_tranche(&events, &target_event_id)?;

    // BG-D5: purchase provenance only — fail-closed, before any computation.
    if provenance != ProvenanceKind::Purchase {
        return Err(refuse_non_purchase(provenance));
    }

    // BG-D7: an empty/whitespace Part II narrative is refused at record time (present-by-construction).
    if part_ii.trim().is_empty() {
        return Err(CliError::Usage(
            "promote-tranche requires a non-empty Form 8275 Part II narrative (filer facts, Reg. \
             §1.6662-4(f) — 'in sufficient detail') — pass --part-ii-file pointing at a file with real \
             acquisition/window facts, not an empty or blank file"
                .into(),
        ));
    }

    // BG-D3: the computed whole-tranche filed_basis floor — hard-refuse on Partial/No coverage.
    let cfg = session.config()?.to_projection();
    let floor = conservative_promote::filed_basis_for(
        session.prices(),
        tranche.sat,
        tranche.window_start,
        tranche.window_end,
    )
    .map_err(|e| refuse_no_floor(e, tranche.window_start, tranche.window_end))?;

    let tables = btctax_adapters::BundledTaxTables::load();
    // A single stored TaxProfile cannot fit the multi-year span this consent/advisory ranges over, so
    // `None` is passed throughout — mirrors the void-direction path (`cmd/reconcile.rs`
    // `promote_void_advisory_lines`): the tax-Δ arm falls back to the gain/deduction-Δ sign, and the
    // amend direction is still correct.
    let terms = conservative_promote::consent_terms(
        &events,
        session.prices(),
        &cfg,
        &target_event_id,
        floor.filed_basis,
        None,
        &tables,
    );

    // Thread ONE synthetic promote so the Direction::Promote advisory AND this CLI's own gift-vs-donation
    // year classification (T9 handoff) see the SAME post-promote fold.
    let with_events = with_synthetic_promote(&events, &target_event_id, floor.filed_basis, now);
    let synthetic_id = with_events
        .last()
        .expect("with_synthetic_promote always pushes exactly one event")
        .id
        .clone();

    // T8 handoff (progress.md): `current` is the injected `now`'s tax year (the BTCTAX_NOW seam) — NEVER
    // a wall clock. Years `< current` are presumed already filed (they get the 1040-X pointer); the year
    // still being authored (>= current) is excluded, so it is never told it needs an amended return.
    let current = tax_date(now, UtcOffset::UTC).year();
    let advisory = conservative::promote_prior_year_advisory(
        &with_events,
        session.prices(),
        &cfg,
        &synthetic_id,
        Direction::Promote,
        None,
        &tables,
        current,
    );
    for line in &advisory {
        println!("{line}");
    }

    // T9 handoff: which flagged years are GIFT-ONLY (no donation) — relabels that year's deduction/basis-Δ
    // as a §1015 donee-basis change, never Schedule-A, in the consent screen below.
    let gift_only_years = gift_only_flagged_years(session.prices(), &cfg, &events, &with_events);

    // The consent screen — printed BEFORE the acknowledgment gate so a refusal (missing/wrong phrase)
    // still surfaces the computed figures on stdout (N-2, mirrors the non-TTY --i-acknowledge contract).
    println!("{}", render_consent(&terms, &gift_only_years));
    if let Some(note) = wide_window_note(tranche.window_start, tranche.window_end) {
        println!("{note}");
    }

    require_promote_ack(acknowledge)?;

    let payload = EventPayload::PromoteTranche(PromoteTranche {
        target: target_event_id,
        method: FloorMethod::WindowLowClose,
        filed_basis: floor.filed_basis,
        coverage: floor.coverage,
        provenance_attested: true,
        acknowledgment: Acknowledgment {
            phrase: PROMOTE_ACK_PHRASE.to_string(),
            shown_terms: terms,
            provenance_text: PROVENANCE_TEXT.to_string(),
            provenance_version: PROVENANCE_VERSION.to_string(),
        },
        part_ii_narrative: part_ii,
    });

    // BG-D9: pre-check `would_conflict` (a second live promote on this target, or any other resolver-level
    // conflict, e.g. UX-P4-3) — refuse BEFORE appending (fail-closed). NOT last-wins.
    if let Some(detail) =
        btctax_core::would_conflict(&events, session.prices(), &cfg, &payload, now)
    {
        return Err(CliError::Usage(format!(
            "cannot record this promote — a decision conflict: {detail}"
        )));
    }

    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}
