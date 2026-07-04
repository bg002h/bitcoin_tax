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

/// Per-disposal identification compliance status (§A.5).
///
/// - `StandingOrder`      — a dated `MethodElection` was in-force at the time of sale (§A.5(a)).
/// - `Contemporaneous`    — a `LotSelection` was recorded on or before the day of sale (§A.5(b)).
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

    // ── 4. Build sel_made: disposal_event → made-date of the covering LotSelection ──────────────
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

    let mut sel_made: BTreeMap<EventId, TaxDate> = BTreeMap::new();
    for (_seq, e) in &selections {
        if let EventPayload::LotSelection(ls) = &e.payload {
            // insert/overwrite: ascending iteration → highest seq is the final value.
            sel_made.insert(
                ls.disposal_event.clone(),
                tax_date(e.utc_timestamp, e.original_tz),
            );
        }
    }

    // ── 5. Classifier ──────────────────────────────────────────────────────────────────────────
    // §A.5 priority, with the load-bearing cross-cutting override (SPEC §Cross-cutting: "no
    // artifact, command, or doc may describe post-hoc selection as compliant"):
    //   1. 2027+ broker-communication envelope → NonCompliant.
    //   2. A `LotSelection` APPLIED to this disposal drives the reported basis/gain, so the
    //      selection's OWN timeliness governs: made-date ≤ sale → Contemporaneous, else →
    //      NonCompliant. A standing order may NEVER rescue a post-hoc selection.
    //   3. Only when NO selection was applied: an in-force `MethodElection` → StandingOrder.
    //   4. Otherwise → NonCompliant.
    let classify = |disposal: &EventId, wallet: &WalletId, date: TaxDate| -> ComplianceStatus {
        // (1) Broker-communication envelope (2027+): own-books identification is insufficient for
        // broker-custodied units — the broker side must communicate the basis. `AttestedRecording`
        // (§C.2) is the C gate; A cannot confer it here.
        let broker = matches!(wallet, WalletId::Exchange { .. });
        if broker && date.year() >= 2027 {
            return ComplianceStatus::NonCompliant;
        }

        // (2) §A.5(b): a `LotSelection` applied to this disposal drove the reported result, so the
        // selection's own timeliness governs. A post-hoc selection (made-date AFTER the sale) is
        // NonCompliant and must NOT fall through to the standing-order check — a standing order
        // would never produce a cherry-picked post-hoc set, so labeling it StandingOrder would
        // present a forbidden post-hoc identification as compliant (§1.1012-1(j)).
        if let Some(made) = sel_made.get(disposal) {
            if *made <= date {
                return ComplianceStatus::Contemporaneous;
            }
            return ComplianceStatus::NonCompliant;
        }

        // (3) §A.5(a) standing order — only reachable when NO selection was applied: the SHARED
        // wallet-aware `resolve_election` (the SAME resolver the fold uses) selects the in-force
        // election for THIS wallet via two independent tiers (scoped, then global) [R0-I1/R0-M2]. Its
        // `effective_from` becomes the StandingOrder date. A scoped election on a DIFFERENT wallet
        // never taints this disposal (tier 1 empty, tier 2 global empty ⇒ None ⇒ NonCompliant).
        if let Some(ef) = resolve_election(date, wallet, &elections).map(|e| e.effective_from) {
            return ComplianceStatus::StandingOrder { effective_from: ef };
        }

        // (4) No envelope hit, no applied selection, no in-force election.
        ComplianceStatus::NonCompliant
    };

    // ── 6. Emit one row per post-2025 disposal / removal ───────────────────────────────────────
    let mut out: Vec<DisposalCompliance> = Vec::new();

    for d in &state.disposals {
        // Exclude fee mini-dispositions (TP8-b recognition records) and pre-2025 disposals.
        if d.fee_mini_disposition || d.disposed_at < TRANSITION_DATE {
            continue;
        }
        if let Some(w) = wallet_of.get(&d.event) {
            out.push(DisposalCompliance {
                disposal: d.event.clone(),
                wallet: w.clone(),
                date: d.disposed_at,
                status: classify(&d.event, w, d.disposed_at),
            });
        }
    }

    for r in &state.removals {
        if r.removed_at < TRANSITION_DATE {
            continue;
        }
        if let Some(w) = wallet_of.get(&r.event) {
            out.push(DisposalCompliance {
                disposal: r.event.clone(),
                wallet: w.clone(),
                date: r.removed_at,
                status: classify(&r.event, w, r.removed_at),
            });
        }
    }

    // NFR4: total order by `EventId: Ord` → byte-identical output regardless of fold order.
    out.sort_by(|a, b| a.disposal.cmp(&b.disposal));
    out
}
