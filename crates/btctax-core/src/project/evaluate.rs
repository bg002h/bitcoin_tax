//! §A.6 side-effect-free evaluate entrypoint.
//!
//! `evaluate_disposal` folds a candidate disposal (existing-ledger **or** synthetic/hypothetical)
//! plus a candidate lot selection through the same `consume`/validation/scoring path as the real
//! projection, then returns the resulting legs/gains/lots WITHOUT mutating the ledger.
//!
//! Pattern: clone → append-synthetic-if-needed → fold → read → discard (mirrors
//! `transition::universal_snapshot`'s clone-fold-discard approach).
//!
//! `--proceeds` is required when no dataset price exists for the candidate date (Mode-2 future
//! disposals have no price entry). For an existing disposal the proceeds already live in the event.
use crate::conventions::{Sat, Usd};
use crate::event::{DisposeKind, LedgerEvent, LotPick};
use crate::identity::{EventId, SourceRef, WalletId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::fold::fold;
use crate::project::resolve::{resolve, Eff, Op};
use crate::state::{Blocker, BlockerKind, DisposalLeg, Lot, Term};
use crate::{ProjectionConfig, TaxDate};
use time::UtcOffset;

/// A candidate disposal to be evaluated (without persisting anything).
///
/// - `existing_event = Some(id)` — re-score an event already in the ledger with a candidate
///   selection (the event's own proceeds are used; `proceeds` on the candidate is ignored).
/// - `existing_event = None` — a synthetic/hypothetical disposal (Mode-2 consultation). The
///   engine appends a temporary `Op::Dispose` with the given `proceeds` (or FMV from the
///   dataset when `proceeds` is `None`), folds, and discards the result.
#[derive(Debug, Clone)]
pub struct CandidateDisposal {
    /// `Some(id)` → score an existing disposal; `None` → synthetic (Mode-2).
    pub existing_event: Option<EventId>,
    pub wallet: WalletId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: DisposeKind,
    /// Explicit proceeds (wins over FMV). Required for synthetic disposals on dates with no
    /// price entry — `evaluate_disposal` returns `Err(ProceedsRequired)` when both this and
    /// the dataset FMV are absent.
    pub proceeds: Option<Usd>,
}

/// The result of scoring a candidate disposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluateOutcome {
    /// Disposal legs for this candidate's disposal event, in fold order.
    pub legs: Vec<DisposalLeg>,
    /// Σ gain on short-term legs.
    pub st_gain: Usd,
    /// Σ gain on long-term legs.
    pub lt_gain: Usd,
    /// Remaining lots after the fold (the full post-fold pool — allows the caller to inspect
    /// what remains available for future disposals).
    pub lots_after: Vec<Lot>,
    /// Hard/advisory blockers attributed to this candidate's disposal event, plus any
    /// principal-conservation violation in the candidate selection.
    pub blockers: Vec<Blocker>,
}

/// Error returned by `evaluate_disposal` when the fold cannot proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluateError {
    /// A synthetic disposal on a date with no dataset price requires an explicit `--proceeds`.
    ProceedsRequired,
    /// `existing_event` named an event that does not resolve to a method-honoring disposal op
    /// (Dispose / GiftOut / Donate / SelfTransfer) in the current timeline.
    UnknownExistingDisposal,
}

/// True when an `Op` is one of the four method-honoring disposal variants.
fn honoring(op: &Op) -> bool {
    matches!(
        op,
        Op::Dispose { .. } | Op::GiftOut { .. } | Op::Donate { .. } | Op::SelfTransfer { .. }
    )
}

/// Side-effect-free evaluation of a candidate disposal + lot selection.
///
/// **No mutation:** events/prices/config are borrowed read-only; `resolve` + `fold` operate
/// on an owned clone of the `Resolution`; the resulting `LedgerState` is read then discarded.
///
/// **Proceeds resolution (synthetic path only):**
/// 1. `candidate.proceeds` wins when `Some`.
/// 2. Dataset FMV (from `prices`) is used when `proceeds` is `None` and a price exists.
/// 3. `Err(ProceedsRequired)` is returned when both are absent (typical for future dates).
///
/// For an existing disposal (`existing_event = Some(id)`) the proceeds already live in the
/// event's `Op::Dispose { proceeds, .. }`; step 1-3 are not consulted.
pub fn evaluate_disposal(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    candidate: &CandidateDisposal,
    selection: Option<&[LotPick]>,
) -> Result<EvaluateOutcome, EvaluateError> {
    let mut res = resolve(events, prices, config);

    let target_id = match &candidate.existing_event {
        Some(id) => {
            // Verify the event resolves to a method-honoring disposal op in the current timeline.
            if !res.timeline.iter().any(|e| &e.id == id && honoring(&e.op)) {
                return Err(EvaluateError::UnknownExistingDisposal);
            }
            id.clone()
        }
        None => {
            // Synthetic disposal: resolve proceeds (explicit > FMV > error).
            let proceeds = match candidate.proceeds {
                Some(p) => p,
                None => fmv_of(prices, candidate.date, candidate.sat)
                    .ok_or(EvaluateError::ProceedsRequired)?,
            };
            // Use a reserved sentinel seq (u64::MAX) as the synthetic event id.
            // Real decision sequences start at 0 and are assigned by `append_decision`; u64::MAX
            // is unreachable in practice and is never persisted (NFR4: no I/O in this path).
            let id = EventId::Decision { seq: u64::MAX };
            // midnight().assume_utc() gives OffsetDateTime at UTC 00:00:00 on the candidate date;
            // tax_date(utc, UTC) == candidate.date exactly (§6.1 day-granularity convention).
            let utc = candidate.date.midnight().assume_utc();
            res.timeline.push(Eff {
                id: id.clone(),
                utc,
                tz: UtcOffset::UTC,
                src_priority: 0,
                src_ref: SourceRef::new("__synthetic__"),
                wallet: Some(candidate.wallet.clone()),
                op: Op::Dispose {
                    sat: candidate.sat,
                    proceeds,
                    fee_usd: Usd::ZERO,
                    fee_sat: None,
                    kind: candidate.kind,
                },
            });
            id
        }
    };

    // Inject the candidate selection (overrides any persisted selection for this event), after
    // mirroring resolve's principal-conservation guard: Σpick.sat MUST equal candidate.sat.
    let mut extra: Vec<Blocker> = Vec::new();
    if let Some(picks) = selection {
        let picked: Sat = picks.iter().map(|p| p.sat).sum();
        if picked != candidate.sat {
            extra.push(Blocker {
                kind: BlockerKind::LotSelectionInvalid,
                event: Some(target_id.clone()),
                detail: format!(
                    "candidate selection must conserve principal: {picked} != {}",
                    candidate.sat
                ),
            });
        } else {
            res.selections.insert(target_id.clone(), picks.to_vec());
        }
    }

    // Fold through the same consume/validation/scoring path as the real projection.
    // The resulting `LedgerState` is read then immediately discarded — no I/O, no persistence.
    let state = fold(res, prices, config);

    // Extract legs, gains, and blockers attributed to the candidate event.
    let legs: Vec<DisposalLeg> = state
        .disposals
        .iter()
        .filter(|d| d.event == target_id)
        .flat_map(|d| d.legs.clone())
        .collect();
    let st_gain: Usd = legs
        .iter()
        .filter(|l| l.term == Term::ShortTerm)
        .map(|l| l.gain)
        .sum();
    let lt_gain: Usd = legs
        .iter()
        .filter(|l| l.term == Term::LongTerm)
        .map(|l| l.gain)
        .sum();
    let mut blockers: Vec<Blocker> = state
        .blockers
        .iter()
        .filter(|b| b.event.as_ref() == Some(&target_id))
        .cloned()
        .collect();
    blockers.extend(extra);

    Ok(EvaluateOutcome {
        legs,
        st_gain,
        lt_gain,
        lots_after: state.lots,
        blockers,
    })
}
