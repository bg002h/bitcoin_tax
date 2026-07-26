//! DFW-D4: the structured shortfall signal + triage classifier.
//!
//! Derived, read-only over the projected `LedgerState` (+ the raw `events` log, for `triage`'s
//! pool/date lookups) — additive/derived, NEVER a second source of truth, and NEVER a `Blocker`-message
//! parse (a grep-guard KAT pins this: `shortfalls_never_parses_blocker_detail`).
//!
//! `shortfalls` aggregates the fold's raw per-emission `state.shortfalls` (`ShortfallRecord`s) per
//! EVENT into a `Shortfall` (arch-I-2: `short_sat` = Σ(principal+fee) over every record on that event;
//! `fee_sat` = the fee component alone, so `principal_sat = short_sat - fee_sat`).
//!
//! `triage` classifies per DFW-D4 (§5):
//!   - a `Shortfall` correlated — same `pool_key(date, wallet)`, and the OTHER blocker's own date
//!     `<=` the shortfall's date — with an open acquisition-side blocker (`UnknownBasisInbound` /
//!     `Unclassified` / `ImportConflict` / `UnmatchedOutflows`) → `ResolveFirst` (a pending-out
//!     short's OWN co-emitted `UnmatchedOutflows` advisory is the same event, hence trivially the same
//!     pool/date — the C-1 double-count guard, tax-I-1/arch-I-5: a later `TransferLink` may reshape it);
//!   - else → `DeclareCandidate`;
//!   - an `UncoveredDisposal` blocker with NO matching `Shortfall` (a without-wallet / degenerate site
//!     — `fold.rs` never records a sat amount there, it `return`s before any pool draw) → `DataFix`.

use crate::conventions::{tax_date, TaxDate};
use crate::identity::{EventId, WalletId};
use crate::project::pools::{pool_key, PoolKey};
use crate::state::{BlockerKind, LedgerState};
use crate::LedgerEvent;
use std::collections::{BTreeMap, BTreeSet};

/// A per-event aggregate shortfall (DFW-D4/D7/D8 clearance + prefill): `short_sat` is the
/// Σ(principal+fee) shortfall on `event`; `fee_sat` is the fee component alone
/// (`principal_sat = short_sat - fee_sat`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortfall {
    pub event: EventId,
    pub wallet: Option<WalletId>,
    pub date: TaxDate,
    pub short_sat: i64,
    pub fee_sat: i64,
}

/// DFW-D4 triage classification for an unresolved `UncoveredDisposal` site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Triage {
    /// Cleanly declarable now — no open acquisition-side blocker stands behind it.
    DeclareCandidate(Shortfall),
    /// A same-pool/timeframe open acquisition blocker (or, for a pending-out short, its own
    /// co-emitted `UnmatchedOutflows`) may still reshape this shortfall — resolve that FIRST.
    ResolveFirst {
        shortfall: Shortfall,
        blocker: BlockerKind,
    },
    /// A without-wallet / degenerate `UncoveredDisposal` — no sat amount to declare; fix the data.
    DataFix(EventId),
}

/// Aggregate the fold's raw `state.shortfalls` records per EVENT (arch-I-2). NEVER parses
/// the `Blocker`'s free-text message — the fold already recorded the sat amounts structurally.
pub fn shortfalls(state: &LedgerState) -> Vec<Shortfall> {
    let mut by_event: BTreeMap<EventId, (Option<WalletId>, TaxDate, i64, i64)> = BTreeMap::new();
    for r in &state.shortfalls {
        let entry = by_event
            .entry(r.event.clone())
            .or_insert_with(|| (r.wallet.clone(), r.date, 0, 0));
        entry.2 += r.principal_sat + r.fee_sat;
        entry.3 += r.fee_sat;
    }
    by_event
        .into_iter()
        .map(|(event, (wallet, date, short_sat, fee_sat))| Shortfall {
            event,
            wallet,
            date,
            short_sat,
            fee_sat,
        })
        .collect()
}

/// The four `BlockerKind`s DFW-D4 treats as an "open acquisition blocker" — a not-yet-resolved
/// classification/basis question whose eventual resolution may still SUPPLY the sats a later
/// shortfall needs (or, for `UnmatchedOutflows`, reshape a pending-out via a later `TransferLink`).
fn is_open_acquisition_kind(kind: BlockerKind) -> bool {
    matches!(
        kind,
        BlockerKind::UnknownBasisInbound
            | BlockerKind::Unclassified
            | BlockerKind::ImportConflict
            | BlockerKind::UnmatchedOutflows
    )
}

/// DFW-D4 triage (§5). `events` is the raw ledger log — consulted ONLY to look up an open-acquisition
/// blocker's own event date/wallet (via `tax_date`/`LedgerEvent::wallet`, exactly like every other
/// core caller resolves a date), never to parse the `Blocker`'s free-text message.
pub fn triage(events: &[LedgerEvent], state: &LedgerState) -> Vec<Triage> {
    let by_id: BTreeMap<&EventId, &LedgerEvent> = events.iter().map(|e| (&e.id, e)).collect();

    // Every open-acquisition-kind blocker's own (pool, date, kind) — looked up from the RAW event log.
    let open: Vec<(PoolKey, TaxDate, BlockerKind)> = state
        .blockers
        .iter()
        .filter(|b| is_open_acquisition_kind(b.kind))
        .filter_map(|b| {
            let ev_id = b.event.as_ref()?;
            let le = by_id.get(ev_id)?;
            let w = le.wallet.as_ref()?;
            let d = tax_date(le.utc_timestamp, le.original_tz);
            Some((pool_key(d, w), d, b.kind))
        })
        .collect();

    let sf = shortfalls(state);
    let sf_events: BTreeSet<EventId> = sf.iter().map(|s| s.event.clone()).collect();

    let mut out = Vec::new();

    // Without-wallet / degenerate UncoveredDisposal sites: an UncoveredDisposal blocker whose event
    // has NO matching Shortfall aggregate (fold.rs never records a sat amount for these — they
    // `return` before any pool consumption is attempted).
    for b in &state.blockers {
        if b.kind == BlockerKind::UncoveredDisposal {
            if let Some(ev_id) = &b.event {
                if !sf_events.contains(ev_id) {
                    out.push(Triage::DataFix(ev_id.clone()));
                }
            }
        }
    }

    for s in sf {
        let Some(w) = s.wallet.clone() else {
            // Defensive: every sat-carrying fold site resolves a wallet before drawing on the pool, so
            // a Shortfall should always carry one — never observed in practice, but never silently
            // declared if it somehow didn't.
            out.push(Triage::DataFix(s.event.clone()));
            continue;
        };
        let pool = pool_key(s.date, &w);
        match open.iter().find(|(p, d, _)| *p == pool && *d <= s.date) {
            Some((_, _, kind)) => out.push(Triage::ResolveFirst {
                shortfall: s,
                blocker: *kind,
            }),
            None => out.push(Triage::DeclareCandidate(s)),
        }
    }

    out
}
