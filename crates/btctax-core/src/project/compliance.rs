//! §A.5 per-disposal compliance projection. Side-effect-free; reusable by `verify` (Task 8) and by C.
//!
//! Produces one `DisposalCompliance` entry per post-2025 realized disposal/removal.  The classifier
//! produces three states: `StandingOrder` / `Contemporaneous` / `NonCompliant`.  The fourth variant
//! `AttestedRecording` is defined here (reserved) but is conferred by Sub-project C, not A.
use crate::conventions::{tax_date, TaxDate, TRANSITION_DATE};
use crate::event::{EventPayload, LedgerEvent};
use crate::identity::{EventId, WalletId};
use crate::project::resolve::{method_election_is_forward, resolve_election, ElectionRec};
use crate::state::LedgerState;
use std::collections::{BTreeMap, BTreeSet};
use time::OffsetDateTime;

/// ★★★ **THE §1.1012-1(j)(2) timeliness rule — ONE predicate, and this is it.**
///
/// > "…the taxpayer specifies the particular units … **no later than the date and time of the sale,
/// > disposition, or transfer**…"
/// > (archived: `legal/primary-sources/regulations-cfr/26CFR_1.1012-1_basis.xml`)
///
/// "No later than" is inclusive, so the comparison is `<=` and an identification made at the exact
/// instant of the sale is timely.
///
/// **Why a two-line function has a doc comment this long (I-1, 2026-09-05).** This rule was written
/// out at THREE sites and FR-35 narrowed only one of them from `TaxDate` to `OffsetDateTime`. For
/// seven hours of every sale day the three then disagreed about the same disposal: the fold DROPPED a
/// 17:00 selection against a 10:00 sale and filed the method-order basis, while `verify` and
/// `optimize` printed `Contemporaneous` for it. Nothing on a filed form was wrong — the drop moves
/// the number conservatively — but the tool contradicted itself, and
/// `design/SPEC_lot_optimization_program.md:218` forbids ANY artifact, command or doc describing
/// post-hoc selection as compliant.
///
/// So: every site that asks "was this identification in time?" calls THIS. There are four callers and
/// no other `<=` between an identification instant and a disposition instant anywhere in the tree:
///
/// | caller | what it decides |
/// |---|---|
/// | `resolve`'s §A.4 timeliness `retain` | whether the fold APPLIES the selection — the filed number |
/// | `identification_made` (below) | the §A.5 verdict `verify` prints, and the fold's FR-34 advisory |
/// | `optimize::persistability` | whether `optimize accept` may write without an attestation |
/// | `optimize::proposed_compliance_status` | the status `optimize` prints for a proposed pick |
///
/// **BOTH arguments are instants on purpose.** Truncating either to a calendar date re-opens I-1 at
/// that caller alone, which is why each caller carries its own same-day-late test
/// (`crates/btctax-core/tests/timeliness_one_predicate.rs`) rather than trusting this one function to
/// hold them all: a site can still narrow on its way IN.
pub fn identification_is_timely(made: OffsetDateTime, disposition: OffsetDateTime) -> bool {
    made <= disposition
}

/// The two facts about one disposition that the §A.5 gates need — carried together because they are
/// at DIFFERENT granularities and the difference is load-bearing (I-1).
///
/// It is a struct rather than two parameters for a mechanical reason: `persistability` and
/// `proposed_compliance_status` also take the identification's made-INSTANT, and three bare time
/// arguments (one `TaxDate`, two `OffsetDateTime`) let a caller transpose the sale instant and the
/// made instant with no type error at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispositionMoment {
    /// The tax DATE of the disposition. Decides the CALENDAR rules: the §6045 broker-communication
    /// envelope (`year >= 2027`) and the §1.1012-1(j)(6) applicability window (on or after
    /// 2025-01-01). Both are stated in the authorities as dates, not moments, so this is not a
    /// narrowing — it is the granularity those rules are written at.
    pub date: TaxDate,
    /// The INSTANT of the disposition. The §1.1012-1(j)(2) identification deadline, and the ONLY
    /// thing `identification_is_timely` may be handed. Never derive this from `date`.
    pub at: OffsetDateTime,
}

/// Per-disposal identification compliance status (§A.5).
///
/// - `StandingOrder`      — a dated `MethodElection` was in-force at the time of sale (§A.5(a)).
/// - `Contemporaneous`    — a `LotSelection` was recorded no later than the date **and time** of the
///   sale (§A.5(b) / §1.1012-1(j)(2)). ★ This variant said "on or before the DAY of sale" until
///   I-1 (2026-09-05); that was the standard the code applied at only one of its three sites, and
///   a selection made at 17:00 on the day of a 10:00 sale is not one.
/// - `AttestedRecording`  — reserved; conferred by Sub-project C (§C.2).
/// - `NonCompliant`       — none of the above apply (no post-hoc identification, §1.1012-1(j)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceStatus {
    StandingOrder { effective_from: TaxDate },
    Contemporaneous,
    AttestedRecording,
    NonCompliant,
}

/// One row of A.5 compliance output per post-2025 realized disposal/removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalCompliance {
    pub disposal: EventId,
    pub wallet: WalletId,
    pub date: TaxDate,
    pub status: ComplianceStatus,
}

/// The §A.5 identification verdict for ONE disposition, computed from the only two facts that decide
/// it: the made-INSTANT of a `LotSelection` applied to it (`None` = none applied), and the
/// `effective_from` of the `MethodElection` in force for its wallet (`None` = none in force, which is
/// where the fold falls through to its default method).
///
/// ★★ **SHARED, and that is the point (FR-34).** `disposal_compliance` calls it to REPORT a verdict
/// after the fold, and `fold::consume_principal` calls it to decide, WHILE the basis is being built,
/// whether the units it is about to charge were identified by the filer at all. Before FR-34 the
/// verdict had no reader on the filing path: a no-election disposal computed at the HIFO default
/// while this classifier called that same disposal `NonCompliant`, and the row reached only `verify`.
/// One function means the reported verdict and the filed number can never diverge.
///
/// ★ **That sentence was FALSE for one day (I-1).** FR-34 shared the classifier while handing it
/// `TaxDate`s, thirty minutes after FR-35 had moved the fold's own timeliness pass to instants — so
/// the two "one computation" halves compared different things for seven hours of every sale day.
/// `disposition` and `selection_made` are `OffsetDateTime` now, and the comparison itself is
/// `identification_is_timely`, which every other site calls too.
///
/// `election_from` stays a `TaxDate` and is NOT a leftover narrowing: a `MethodElection` is a standing
/// order *effective from a date* (§A.5(a)), it is resolved date-wise by `resolve_election`, and no
/// comparison against the disposition's instant is made with it here.
///
/// Priority, per §A.5 and the load-bearing cross-cutting SPEC rule ("no artifact, command, or doc may
/// describe post-hoc selection as compliant"):
///   1. A `LotSelection` applied to this disposal drives the reported basis, so the selection's OWN
///      timeliness governs: timely → `Contemporaneous`, else `NonCompliant`. A standing order may
///      NEVER rescue a post-hoc selection.
///   2. Only when NO selection applied: an in-force `MethodElection` → `StandingOrder`.
///      §1.1012-1(j)(3)(ii): "A standing order or instruction for the specific identification of
///      digital assets is treated as an adequate identification made at the time of sale."
///   3. Otherwise → `NonCompliant`: nothing identified these units. §1.1012-1(j)(1) / (j)(3)(i) answer
///      that case with the deemed acquisition order, and FR-34 is the disclosure that btctax's default
///      is not that order.
///
/// The §1.1012-1(j)(3) broker-communication envelope is NOT applied here — it is an overlay on a
/// verdict, not a fact about what the filer identified, and it is applied by `disposal_compliance`.
pub(crate) fn identification_made(
    disposition: OffsetDateTime,
    selection_made: Option<OffsetDateTime>,
    election_from: Option<TaxDate>,
) -> ComplianceStatus {
    if let Some(made) = selection_made {
        if identification_is_timely(made, disposition) {
            return ComplianceStatus::Contemporaneous;
        }
        return ComplianceStatus::NonCompliant;
    }
    match election_from {
        Some(effective_from) => ComplianceStatus::StandingOrder { effective_from },
        None => ComplianceStatus::NonCompliant,
    }
}

/// Collect all non-voided, non-backdated `MethodElection` decisions that are on or after
/// `TRANSITION_DATE` and whose `effective_from` ≥ their made-date (the backdating guard) into
/// `ElectionRec`s — CARRYING THE PER-WALLET SCOPE — so the SHARED `resolve_election` resolver
/// (the same one the fold uses) can apply the two-independent-tiers precedence here [R0-I1]. Without
/// the scope, a scoped `Coinbase→HIFO` election would falsely tag a `Gemini` disposal as
/// `StandingOrder` (over-reporting §A.5(a)); with it, tier 1 (scoped) is empty for Gemini and tier 2
/// (global) is empty, so the Gemini disposal is correctly `NonCompliant`.
///
/// Uses the shared `method_election_is_forward` predicate from `resolve.rs` so that both callers
/// stay in sync with the §A.5(a) spec rule without duplicating the guard condition.
fn collect_elections(events: &[LedgerEvent], voided: &BTreeSet<EventId>) -> Vec<ElectionRec> {
    let mut out = Vec::new();
    for e in events {
        let EventId::Decision { seq } = e.id else {
            continue;
        };
        if voided.contains(&e.id) {
            continue;
        }
        if let EventPayload::MethodElection(me) = &e.payload {
            let made = tax_date(e.utc_timestamp, e.original_tz);
            if method_election_is_forward(me, made) {
                out.push(ElectionRec {
                    effective_from: me.effective_from,
                    method: me.method,
                    decision_seq: seq,
                    wallet: me.wallet.clone(),
                });
            }
        }
    }
    out
}

/// Compute per-disposal compliance status for all post-2025 realized disposals and removals.
///
/// **Scope boundary — `SelfTransfer` is intentionally excluded.**
/// This function flags the §1.1012-1(j) adequacy of identification at a **taxable disposition**
/// (Dispose / GiftOut / Donate).  A `SelfTransfer` is a non-taxable positioning move — the
/// taxpayer may choose which lots to relocate via `LotSelection` (§A.3 lists it as
/// method-honoring), but there is no recognized gain/loss and no §1.1012-1(j) identification
/// obligation at the self-transfer itself.  Accordingly, a `SelfTransfer` never produces a
/// `Disposal` or `Removal` record in `LedgerState`, and this function (which iterates only
/// `state.disposals` / `state.removals`) is **correctly out of scope for self-transfers by
/// design**.
///
/// Note: §A.3 of the spec lists `SelfTransfer` as method-honoring because the lot-routing
/// choice affects future per-wallet HIFO/LIFO positioning; that is about the *selection
/// mechanism*, not about compliance-flagging the non-taxable transfer itself.
///
/// **NFR4 determinism:** `sel_made` is built by iterating `LotSelection` decisions in ascending
/// `decision_seq` order (R0-plan M1).  When a disposal has more than one `LotSelection` (a
/// `DecisionConflict` handled separately by `resolve`), the highest-seq made-date wins — stable
/// and load-order-independent.  Output is sorted by `disposal` (`EventId: Ord`).
///
/// **Read-only:** no events are appended; the function is a pure function of its inputs.
pub fn disposal_compliance(events: &[LedgerEvent], state: &LedgerState) -> Vec<DisposalCompliance> {
    // ── 1. Build the voided set ──────────────────────────────────────────────────────────────────
    let voided: BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect();

    // ── 2. Collect eligible elections ───────────────────────────────────────────────────────────
    let elections = collect_elections(events, &voided);

    // ── 3. Index disposal-event → WalletId (from the import event's wallet field) ──────────────
    let wallet_of: BTreeMap<EventId, WalletId> = events
        .iter()
        .filter_map(|e| e.wallet.clone().map(|w| (e.id.clone(), w)))
        .collect();

    // ── 3b. Index disposal-event → its INSTANT (I-1) ────────────────────────────────────────────
    // The §1.1012-1(j)(2) deadline is the moment of the sale, and `LedgerState::Disposal` carries
    // only `disposed_at: TaxDate`. The instant is right here in `events` — it is the same
    // `utc_timestamp` `resolve` reads into `Eff::utc` for the fold's own timeliness pass
    // (`resolve.rs`, `identification_deadline`), so the reporting side and the filing side are
    // reading ONE fact. This lookup is why they cannot drift apart again.
    let instant_of: BTreeMap<EventId, OffsetDateTime> = events
        .iter()
        .map(|e| (e.id.clone(), e.utc_timestamp))
        .collect();

    // ── 4. Build sel_made: disposal_event → made-INSTANT of the covering LotSelection ───────────
    // NFR4 (M1): iterate decisions in ascending `decision_seq` order so the last write (highest
    // seq) wins; deterministic regardless of the slice order in `events`.
    let mut selections: Vec<(u64, &LedgerEvent)> = events
        .iter()
        .filter_map(|e| match e.id {
            EventId::Decision { seq } => Some((seq, e)),
            _ => None,
        })
        .filter(|(_, e)| {
            !voided.contains(&e.id) && matches!(e.payload, EventPayload::LotSelection(_))
        })
        .collect();
    selections.sort_by_key(|(s, _)| *s); // ascending seq → last write wins

    let mut sel_made: BTreeMap<EventId, OffsetDateTime> = BTreeMap::new();
    for (_seq, e) in &selections {
        if let EventPayload::LotSelection(ls) = &e.payload {
            // insert/overwrite: ascending iteration → highest seq is the final value.
            // I-1: the decision's raw INSTANT, not `tax_date(..)` of it. `resolve`'s §A.4 pass reads
            // exactly this field (`d.utc_timestamp`) for the drop that decides the filed number.
            sel_made.insert(ls.disposal_event.clone(), e.utc_timestamp);
        }
    }

    // ── 5. Classifier ──────────────────────────────────────────────────────────────────────────
    // §A.5 priority, with the load-bearing cross-cutting override (SPEC §Cross-cutting: "no
    // artifact, command, or doc may describe post-hoc selection as compliant"):
    //   1. 2027+ broker-communication envelope → NonCompliant.
    //   2. A `LotSelection` APPLIED to this disposal drives the reported basis/gain, so the
    //      selection's OWN timeliness governs: made no later than the date AND TIME of the sale →
    //      Contemporaneous, else → NonCompliant. A standing order may NEVER rescue a post-hoc
    //      selection.
    //   3. Only when NO selection was applied: an in-force `MethodElection` → StandingOrder.
    //   4. Otherwise → NonCompliant.
    //
    // I-1: `date` and `at` are BOTH passed and they are not interchangeable — `date` decides the
    // calendar rule in (1) (broker reporting is a year test), `at` decides the (j)(2) deadline in (2).
    let classify = |disposal: &EventId,
                    wallet: &WalletId,
                    date: TaxDate,
                    at: OffsetDateTime|
     -> ComplianceStatus {
        // (1) Broker-communication envelope (2027+): own-books identification is insufficient for
        // broker-custodied units — the broker side must communicate the basis. `AttestedRecording`
        // (§C.2) is the C gate; A cannot confer it here.
        let broker = matches!(wallet, WalletId::Exchange { .. });
        if broker && date.year() >= 2027 {
            return ComplianceStatus::NonCompliant;
        }

        // (2)–(4) What the filer actually identified. Delegated to `identification_made` — the SAME
        // function `fold::consume_principal` calls while it builds the basis (FR-34), so the verdict
        // reported here and the verdict that governs the filed number are one computation. The
        // wallet-aware `resolve_election` is likewise the SAME resolver the fold uses, applying the two
        // independent tiers (scoped, then global) [R0-I1/R0-M2]; a scoped election on a DIFFERENT
        // wallet never taints this disposal (tier 1 empty, tier 2 global empty ⇒ None ⇒ NonCompliant).
        identification_made(
            at,
            sel_made.get(disposal).copied(),
            resolve_election(date, wallet, &elections).map(|e| e.effective_from),
        )
    };

    // ── 6. Emit one row per post-2025 disposal / removal ───────────────────────────────────────
    let mut out: Vec<DisposalCompliance> = Vec::new();

    for d in &state.disposals {
        // Exclude fee mini-dispositions (TP8-b recognition records) and pre-2025 disposals.
        if d.fee_mini_disposition || d.disposed_at < TRANSITION_DATE {
            continue;
        }
        // I-1: both the wallet and the instant come from the SAME import event. A disposal whose
        // import event is missing already emitted no row (the `wallet_of` guard has always been
        // there); requiring the instant too cannot silently drop one, because `instant_of` is keyed
        // over every event and `wallet_of` is a subset of it.
        if let (Some(w), Some(&at)) = (wallet_of.get(&d.event), instant_of.get(&d.event)) {
            out.push(DisposalCompliance {
                disposal: d.event.clone(),
                wallet: w.clone(),
                date: d.disposed_at,
                status: classify(&d.event, w, d.disposed_at, at),
            });
        }
    }

    for r in &state.removals {
        if r.removed_at < TRANSITION_DATE {
            continue;
        }
        if let (Some(w), Some(&at)) = (wallet_of.get(&r.event), instant_of.get(&r.event)) {
            out.push(DisposalCompliance {
                disposal: r.event.clone(),
                wallet: w.clone(),
                date: r.removed_at,
                status: classify(&r.event, w, r.removed_at, at),
            });
        }
    }

    // NFR4: total order by `EventId: Ord` → byte-identical output regardless of fold order.
    out.sort_by(|a, b| a.disposal.cmp(&b.disposal));
    out
}
