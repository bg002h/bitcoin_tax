use crate::conventions::{
    is_long_term, round_cents, split_pro_rata, Sat, TaxDate, Usd, TRANSITION_DATE,
};
use crate::event::{BasisSource, DisposeKind};
use crate::identity::{EventId, LotId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::pools::{pool_key, Consumed, PoolKey, PoolSet};
use crate::project::resolve::{sort_canonical, Eff, ElectionRec, Op, Resolution};
use crate::project::transition;
use crate::state::{
    BlockerKind, Disposal, DisposalLeg, FoldStats, GiftZone, IncomeRecord, LedgerState, Lot,
    PendingLeg, PendingTransfer, Removal, RemovalKind, RemovalLeg, Term,
};
use crate::{FeeTreatment, LotMethod, ProjectionConfig};
use std::collections::BTreeMap;

/// Read-only context threaded through the PASS-2 fold: the projection config plus the resolved
/// forward method elections (§A.5(a)) and per-disposal named-lot selections (§A.4). Carried as ONE
/// borrow so `fold_event` (and `transition::universal_snapshot`, which reuses it) stay in lock-step.
pub(crate) struct FoldCtx<'a> {
    pub config: &'a ProjectionConfig,
    pub elections: &'a [ElectionRec],
    pub selections: &'a BTreeMap<EventId, Vec<crate::event::LotPick>>,
}

/// The lot-identification method applicable to a disposal at `date`:
/// pre-2025 (Universal pool) → the declared `pre2025_method`; post-2025 (Wallet pool) → the
/// latest-in-force `MethodElection` on/before `date` (total order: `effective_from`, tie `decision_seq`),
/// FIFO before any election (the §1.1012-1(j)(3) regulatory default).
fn applicable_method(date: TaxDate, ctx: &FoldCtx) -> LotMethod {
    if date < TRANSITION_DATE {
        ctx.config.pre2025_method
    } else {
        ctx.elections
            .iter()
            .filter(|e| e.effective_from <= date)
            .max_by(|a, b| {
                a.effective_from
                    .cmp(&b.effective_from)
                    .then(a.decision_seq.cmp(&b.decision_seq))
            })
            .map(|e| e.method)
            .unwrap_or(LotMethod::Fifo)
    }
}

/// Consume a method-honoring op's principal: the applicable method plus any `LotSelection` for `ev`.
/// On a selection-validation failure → hard `LotSelectionInvalid` (carrying the disposal id + reason);
/// consumption falls back to method order so Σsat conservation holds and the hard blocker gates tax.
/// (Selections are an empty map this task; the fallback path is exercised once Task 4 populates them.)
fn consume_principal(
    pools: &mut PoolSet,
    key: &PoolKey,
    need: Sat,
    date: TaxDate,
    ctx: &FoldCtx,
    st: &mut LedgerState,
    ev: &EventId,
) -> (Vec<Consumed>, Sat) {
    let method = applicable_method(date, ctx);
    let selection = ctx.selections.get(ev).map(|v| v.as_slice());
    let r = pools.consume(key, need, method, selection);
    if let Some(reason) = r.selection_error {
        st.add_blocker(BlockerKind::LotSelectionInvalid, Some(ev.clone()), reason);
    }
    (r.consumed, r.shortfall)
}

/// TP4 term for a consumed fragment given the disposition date (gain side / no-dual uses gain_hp_start).
fn term_for(start: TaxDate, disposed: TaxDate) -> Term {
    if is_long_term(start, disposed) {
        Term::LongTerm
    } else {
        Term::ShortTerm
    }
}

/// §7.4: emit the pre-2025 disposal advisory ONCE (a Dispose/Removal consumed the Universal pool),
/// naming the DECLARED `pre2025_method`. Pre-2025 ⇔ the disposition routed through `PoolKey::Universal`.
/// `attested` branches the advisory text (D2): unattested → actionable warning; attested → informational.
/// Severity is always Advisory (never gates `compute_tax_year`).
fn note_pre2025_once(
    st: &mut LedgerState,
    date: TaxDate,
    ev: &EventId,
    method: LotMethod,
    attested: bool,
) {
    if date < TRANSITION_DATE
        && !st
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::Pre2025MethodNote)
    {
        let m = match method {
            LotMethod::Fifo => "FIFO",
            LotMethod::Lifo => "LIFO",
            LotMethod::Hifo => "HIFO",
        };
        let detail = if attested {
            format!(
                "pre-2025 lots reconstructed under your DECLARED + ATTESTED filed method {m} (§7.4); \
                 carryforward basis into 2025 reflects that method"
            )
        } else {
            format!(
                "pre-2025 lots reconstructed under {m} (FIFO is the §7.4 legal default); \
                 you have NOT declared your filed pre-2025 lot method — if your filed pre-2025 returns \
                 used a different method your carryforward basis may differ. \
                 Declare it: config --set-pre2025-method <m> --attest-pre2025-method"
            )
        };
        st.add_blocker(BlockerKind::Pre2025MethodNote, Some(ev.clone()), detail);
    }
}

/// Build disposal legs from consumed fragments and a TOTAL net proceeds amount, allocated pro-rata by sat
/// (remainder-takes-the-rest so Σproceeds is exact). Dual-basis gift logic (TP11) is added in Task 10;
/// here every leg is the simple `gift_zone = None` path.
fn make_disposal_legs(
    consumed: &[Consumed],
    total_net_proceeds: Usd,
    disposed: TaxDate,
    st: &mut LedgerState,
    ev: &EventId,
) -> Vec<DisposalLeg> {
    let total_sat: i64 = consumed.iter().map(|c| c.sat).sum();
    let mut legs = Vec::new();
    let mut allocated = Usd::ZERO;
    for (i, c) in consumed.iter().enumerate() {
        let proceeds = if i + 1 == consumed.len() {
            total_net_proceeds - allocated
        } else {
            let (p, _) = split_pro_rata(total_net_proceeds, c.sat, total_sat);
            allocated += p;
            p
        };
        if c.basis_pending {
            // FMV-missing income / unknown-basis gift in this lot's history → gate the gain (§7.3).
            st.add_blocker(
                BlockerKind::FmvMissing,
                Some(ev.clone()),
                "disposal consumes a basis-pending lot",
            );
        }
        // Task 10: four-zone §1015(a) dual-basis computation (TP11).
        // When `c.dual = false` (no dual basis): simple single-carryover path.
        // When `c.dual = true` (dual-basis gift, FMV-at-gift < donor-basis at gift date):
        //   Gain zone  : proceeds > gain_basis  → basis = gain_basis, term tacks (gain_hp_start).
        //   Loss zone  : proceeds < loss_basis  → basis = loss_basis, HP from gift date (loss_hp_start).
        //   NoGainNoLoss: otherwise             → reported basis = proceeds, gain = 0, term from gain_hp_start.
        // Note: in the NoGainNoLoss zone, `lot.usd_basis` was already reduced by pro-rata `gain_basis`
        // on consume (pools.rs), so Σbasis is conserved exactly even though we report basis = proceeds.
        // acquired_at is set from the SAME HP-start branch that selects term_for's first arg,
        // so it can never contradict the leg's ST/LT classification [R0-C1].
        let (basis, gain, term, gift_zone, acquired_at) = if c.dual {
            let loss_basis = c.loss_basis.expect("dual=true implies loss_basis is Some");
            if proceeds > c.gain_basis {
                // Gain zone: basis = gain_basis (tacks, gain_hp_start).
                let t = term_for(c.gain_hp_start, disposed);
                (
                    c.gain_basis,
                    round_cents(proceeds - c.gain_basis),
                    t,
                    Some(GiftZone::Gain),
                    c.gain_hp_start, // tacked donor date
                )
            } else if proceeds < loss_basis {
                // Loss zone: basis = FMV-at-gift (loss_basis), HP from gift date.
                let t = term_for(c.loss_hp_start, disposed);
                (
                    loss_basis,
                    round_cents(proceeds - loss_basis),
                    t,
                    Some(GiftZone::Loss),
                    c.loss_hp_start, // gift date — loss basis does NOT tack (Pub 551)
                )
            } else {
                // NoGainNoLoss zone: reported basis = proceeds → gain = 0; term from gain_hp_start.
                let t = term_for(c.gain_hp_start, disposed);
                (
                    proceeds,
                    Usd::ZERO,
                    t,
                    Some(GiftZone::NoGainNoLoss),
                    c.gain_hp_start,
                )
            }
        } else {
            let basis = c.gain_basis;
            let t = term_for(c.gain_hp_start, disposed);
            (
                basis,
                round_cents(proceeds - basis),
                t,
                None,
                c.gain_hp_start,
            )
        };
        legs.push(DisposalLeg {
            lot_id: c.lot_id.clone(),
            sat: c.sat,
            proceeds,
            basis,
            gain,
            term,
            basis_source: c.basis_source,
            gift_zone,
            acquired_at,
            wallet: c.wallet.clone(),
        });
    }
    legs
}

/// Build removal legs from consumed fragments and a TOTAL FMV amount, allocated pro-rata by sat
/// (remainder-takes-the-rest so Σfmv is exact). Zero recognized gain (TP10): no Disposal emitted.
/// Returns (legs, donor_acquired_at) where donor_acquired_at is the first non-None across lots.
fn make_removal_legs(
    consumed: &[Consumed],
    total_fmv: Usd,
    removed: TaxDate,
    st: &mut LedgerState,
    ev: &EventId,
) -> (Vec<RemovalLeg>, Option<TaxDate>) {
    let total_sat: i64 = consumed.iter().map(|c| c.sat).sum();
    let mut legs = Vec::new();
    let mut allocated = Usd::ZERO;
    let mut donor = None;
    for (i, c) in consumed.iter().enumerate() {
        if c.basis_pending {
            st.add_blocker(
                BlockerKind::UnknownBasisInbound,
                Some(ev.clone()),
                "removal consumes a basis-pending lot",
            );
        }
        let fmv = if i + 1 == consumed.len() {
            total_fmv - allocated
        } else {
            let (f, _) = split_pro_rata(total_fmv, c.sat, total_sat);
            allocated += f;
            f
        };
        donor = donor.or(c.donor_acquired_at);
        legs.push(RemovalLeg {
            lot_id: c.lot_id.clone(),
            sat: c.sat,
            basis: c.gain_basis,
            fmv_at_transfer: fmv,
            // acquired_at MUST be the SAME HP-start argument fed to `term_for` below so it can never
            // contradict `term`. Removals recognize no gain/loss → no loss-zone branching (unlike
            // disposals): this is always `gain_hp_start` (the tacked donor date for received gifts,
            // §1223). [D1/R0-M2]
            term: term_for(c.gain_hp_start, removed),
            basis_source: c.basis_source,
            acquired_at: c.gain_hp_start,
        });
    }
    (legs, donor)
}

/// Carried basis of the burned fee-sats, to be RE-HOMED onto a surviving destination lot / removal leg
/// under TP8 (c) so the FULL basis carries (C1). Under (b) this is empty (basis rode the mini-disposition).
#[derive(Default)]
struct FeeCarry {
    gain_basis: Usd,
    loss_basis: Option<Usd>,
}

impl FeeCarry {
    /// Re-home the fee-sat basis onto the surviving destination lot (C1: full basis carries).
    /// `gain_basis` always carries onto `lot.usd_basis` (C1 invariant; must not be dropped).
    /// `loss_basis` carries onto `lot.dual_loss_basis` ONLY when the survivor is ALREADY a
    /// dual-basis lot (`Some(existing)` → add to existing). When the survivor is non-dual
    /// (`None`), the `loss_basis` fragment is dropped instead of promoting the lot to `Some`:
    /// promoting would set `dual_loss_basis.is_some() == true`, causing a later disposition to
    /// route through the §1015(a) four-zone logic (`make_disposal_legs` keys on this field)
    /// and misclassify a normal purchased/transferred lot as a received-gift dual-basis lot —
    /// a worse error than the cents-scale conservative loss-basis understatement that results
    /// from the drop. Conservative: future loss-zone basis understated by fee-cents; gain basis
    /// fully conserved (C1 intact).
    fn rehome_onto_lot(&self, lot: &mut Lot) {
        lot.usd_basis += self.gain_basis;
        if let Some(l) = self.loss_basis {
            // Add to existing dual_loss_basis only; when None (non-dual survivor) the fragment
            // is dropped — promoting None → Some would misroute a later disposition through the
            // §1015(a) four-zone logic (see doc comment above for full rationale).
            if let Some(dl) = lot.dual_loss_basis.as_mut() {
                *dl += l;
            }
        }
    }

    /// Re-home the fee-sat gain basis onto the last removal leg (C1: full basis carries to donee).
    /// Note: `loss_basis` is a donor's private tax attribute and does not carry onto removal legs.
    fn rehome_onto_removal_leg(&self, leg: &mut RemovalLeg) {
        leg.basis += self.gain_basis;
    }

    /// Re-home the fee-sat gain basis onto the last disposal leg (I-1: Dispose+fee_sat, TP8 (c)).
    /// Under (c): adds gain_basis to the reported leg basis → gain decreases by carry amount.
    /// Under (b): carry is empty (gain_basis = 0) so this is a no-op; the fee-sat basis rode the
    /// mini-disposition emitted by consume_fee instead.
    fn rehome_onto_disposal_leg(&self, leg: &mut DisposalLeg) {
        leg.basis += self.gain_basis;
        leg.gain = round_cents(leg.proceeds - leg.basis);
    }
}

/// Consume `fee_sat` FIFO from the source pool, record them in the FR9 fee-sat home, and (per config)
/// either return their carried basis for re-homing (c) or emit a mini-disposition recognition record (b).
/// §7.1 totality: a fee shortfall raises `uncovered_disposal`, never panics.
#[allow(clippy::too_many_arguments)]
fn consume_fee(
    pools: &mut PoolSet,
    key: &PoolKey,
    fee_sat: Sat,
    config: &ProjectionConfig,
    prices: &dyn PriceProvider,
    date: TaxDate,
    stats: &mut FoldStats,
    st: &mut LedgerState,
    ev: &EventId,
) -> FeeCarry {
    if fee_sat <= 0 {
        return FeeCarry::default();
    }
    let (consumed, shortfall) = pools.consume_fifo(key, fee_sat);
    if shortfall > 0 {
        st.add_blocker(
            BlockerKind::UncoveredDisposal,
            Some(ev.clone()),
            format!("self-transfer/gift fee short by {shortfall} sat"),
        );
    }
    stats.fee_sats_consumed += consumed.iter().map(|c| c.sat).sum::<Sat>(); // sole FR9 home
    match config.self_transfer_fee {
        FeeTreatment::TreatmentC => {
            // Non-taxable: return the fee-sat basis for re-homing onto the survivor (C1: full basis carries).
            let gain_basis: Usd = consumed.iter().map(|c| c.gain_basis).sum();
            let has_loss = consumed.iter().any(|c| c.loss_basis.is_some());
            let loss_basis = has_loss.then(|| consumed.iter().filter_map(|c| c.loss_basis).sum());
            FeeCarry {
                gain_basis,
                loss_basis,
            }
        }
        FeeTreatment::TreatmentB => {
            // mini-disposition recognition record; proceeds = FMV(fee_sat); basis rides it (NOT re-homed).
            if !consumed.is_empty() {
                let net = fmv_of(prices, date, fee_sat).unwrap_or(Usd::ZERO);
                let legs = make_disposal_legs(&consumed, net, date, st, ev);
                st.disposals.push(Disposal {
                    event: ev.clone(),
                    kind: DisposeKind::Spend,
                    disposed_at: date,
                    legs,
                    fee_mini_disposition: true,
                });
            }
            FeeCarry::default()
        }
    }
}

pub fn fold(
    mut res: Resolution,
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
) -> LedgerState {
    sort_canonical(&mut res.timeline);
    // Eng-review Minor (§7.4): the boundary seed must fire on the TAX-DATE, not raw UTC order. A STABLE
    // partition by tax-date side (pre-2025 first) means a sub-day offset straddling 2025-01-01 (e.g. a
    // +12:00 post-2025 event with an earlier UTC than a −05:00 pre-2025 event) folds on the correct side
    // of the one-shot seed, and the pre-seed Universal residue matches `transition::universal_snapshot`
    // exactly (I-1). `sort_by_key` is stable, so canonical FIFO order is preserved within each side.
    res.timeline.sort_by_key(|e| e.date() >= TRANSITION_DATE);
    let mut st = LedgerState {
        blockers: res.blockers,
        ..Default::default()
    };
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default(); // M3/FR9: fee_sats_consumed (Task 11), sigma_in here
    let mut seeded = false;

    // Thread the resolved elections/selections (and config) through every per-event arm (NFR4).
    let ctx = FoldCtx {
        config,
        elections: &res.elections,
        selections: &res.selections,
    };

    for eff in &res.timeline {
        if !seeded && eff.date() >= TRANSITION_DATE {
            // Path A drain / Path B seed of the per-wallet pools from the Universal residue, ONCE (§7.4).
            transition::seed_transition(&res.transition, &mut pools, &mut st);
            seeded = true;
        }
        fold_event(eff, prices, &ctx, &mut pools, &mut st, &mut stats);
    }

    finalize(&mut st, pools, stats); // if no ≥2025 event ever seeds, Universal lots remain (carry their wallet)
    st
}

/// Fold the canonical timeline up to (but NOT including) `target`, returning the `PoolSet` exactly as the
/// real `fold` holds it at the instant it is about to process `target`. Used by the optimizer's
/// available-lots pre-pass (`optimize::available_lots_before`) so the pool it sees at a disposal matches
/// the real fold under BOTH transition paths.
///
/// **The §7.4 boundary seed fires at the correct boundary.** Exactly as in `fold`, the one-shot seed
/// (Path A drain / Path B seed) fires when the first `≥ TRANSITION_DATE` event is crossed — CRUCIALLY it
/// therefore also fires when `target` is ITSELF that first post-2025 event (the seed check runs before the
/// `target` short-circuit below). The old "truncate-then-refold" approach broke precisely here: when the
/// target disposal was the chronologically-first 2025 timeline event, the truncated prefix contained no
/// `≥ TRANSITION_DATE` event, so the re-fold never seeded and `finalize` surfaced the UN-seeded Universal
/// residue — harmless under Path A (the residue relocates by wallet, lot_ids/basis preserved) but WRONG
/// under Path B (the seed DISCARDS the residue and installs allocation lots with different lot_ids/basis,
/// so the residue lot_ids don't exist in the real pool). Reusing the real `seed_transition` (never a
/// re-implementation of its Path-A/Path-B behavior) guarantees the returned pool matches the live fold.
///
/// If `target` is absent from the timeline this folds the whole timeline; callers needing the
/// "not found ⇒ empty" contract must check existence first (`available_lots_before` does).
pub fn pools_before(
    mut res: Resolution,
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    target: &EventId,
) -> PoolSet {
    // Mirror `fold`'s exact ordering: canonical FIFO, then the stable pre-2025 (tax-date) partition.
    sort_canonical(&mut res.timeline);
    res.timeline.sort_by_key(|e| e.date() >= TRANSITION_DATE);
    let mut st = LedgerState::default(); // discarded — we only read the pool residue (blockers irrelevant)
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default();
    let mut seeded = false;
    let ctx = FoldCtx {
        config,
        elections: &res.elections,
        selections: &res.selections,
    };
    for eff in &res.timeline {
        // Fire the one-shot boundary seed the instant we cross into ≥2025 — BEFORE `target` is reached,
        // so even a target that is the first post-2025 event reads the POST-seed pool (matches `fold`).
        if !seeded && eff.date() >= TRANSITION_DATE {
            transition::seed_transition(&res.transition, &mut pools, &mut st);
            seeded = true;
        }
        if &eff.id == target {
            return pools; // stop BEFORE folding the target (the seed has already fired if applicable)
        }
        fold_event(eff, prices, &ctx, &mut pools, &mut st, &mut stats);
    }
    pools
}

/// Fold the canonical timeline TRUNCATED to events with `date() <= at`, returning the finalized
/// `LedgerState` exactly as the real `fold` would hold it AS OF `at`. Used by the optimizer's Mode-2
/// consult (`optimize::consult_sale`) so the available-lots pool reflects holdings as of `at` (R0-M3):
/// the end-of-timeline pool would be WRONG for an interleaved/past `at` (lots later disposed are
/// missing; lots later acquired are wrongly present). Sibling of `pools_before` (which truncates before
/// a specific disposal id); both reuse `fold`'s exact ordering so truncation is by TIME, not load order.
///
/// **The §7.4 boundary seed fires at the correct boundary (matches `pools_before`).** The one-shot seed
/// (Path A drain / Path B install) fires when the first `>= TRANSITION_DATE` event is crossed. If no
/// real `>= 2025` event precedes `at` but `at >= TRANSITION_DATE`, the seed is FORCED before `finalize`:
/// a hypothetical disposal at `at` is itself a `>= 2025` event, so the real fold (which would include it)
/// fires the seed before it. Forcing here makes the as-of pool match that fold under BOTH transition
/// paths — Path A (residue relocated, lot_ids/basis preserved) and Path B (residue DISCARDED, allocation
/// seed lots installed). Skipping the force would surface the un-seeded Universal residue (harmless under
/// Path A, but WRONG under Path B). `continue` (not `break`) is required for heterogeneous-timezone
/// timelines: `sort_canonical` orders by `utc` ascending, but `date() = tax_date(utc, original_tz)`
/// uses each event's own `original_tz`. Under mixed timezones utc-ascending does NOT imply
/// date-ascending — an event dated `at+1` in a +14:00 timezone can sort UTC-before one dated `at` in
/// +00:00. A `break` would fire on the `at+1` event and skip the later-sorted `at` event. `continue`
/// folds every event with `date() <= at` regardless of its utc position (trivially cheap: timelines
/// are short). The boundary seed fires exactly once (guarded by `seeded`); the `continue` path does
/// not re-fire or skip it.
pub fn state_as_of(
    mut res: Resolution,
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    at: TaxDate,
) -> LedgerState {
    sort_canonical(&mut res.timeline);
    res.timeline.sort_by_key(|e| e.date() >= TRANSITION_DATE);
    let mut st = LedgerState {
        blockers: res.blockers,
        ..Default::default()
    };
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default();
    let mut seeded = false;
    let ctx = FoldCtx {
        config,
        elections: &res.elections,
        selections: &res.selections,
    };
    for eff in &res.timeline {
        // Fire the one-shot boundary seed the instant we cross into >= 2025, BEFORE folding any such
        // event (matches `fold`/`pools_before`).
        if !seeded && eff.date() >= TRANSITION_DATE {
            transition::seed_transition(&res.transition, &mut pools, &mut st);
            seeded = true;
        }
        // Truncate by TIME: skip events strictly after `at`. Cannot break early: `sort_canonical`
        // orders by utc but `date()` uses each event's own `original_tz`, so utc-ascending ≠
        // date-ascending under heterogeneous timezones (e.g. an event dated `at+1` in +14:00 sorts
        // UTC-before one dated `at` in +00:00). `continue` ensures every `date() <= at` event is folded.
        if eff.date() > at {
            continue;
        }
        fold_event(eff, prices, &ctx, &mut pools, &mut st, &mut stats);
    }
    // Force the seed when `at` is on/after the boundary but no real >= 2025 event preceded it: a
    // hypothetical disposal at `at` would itself trigger the seed (Path A drain / Path B install).
    if !seeded && at >= TRANSITION_DATE {
        transition::seed_transition(&res.transition, &mut pools, &mut st);
    }
    finalize(&mut st, pools, stats);
    st
}

/// PASS-2 per-event dispatcher. Lifted out of `fold` so that BOTH the real fold and the pass-1
/// `transition::universal_snapshot` pre-fold run the IDENTICAL per-event arms — the conservation guard's
/// pre-2025 residue therefore provably matches the real fold's pre-seed residue (I-1). Pure: mutates only
/// the passed pools/state/stats.
pub(crate) fn fold_event(
    eff: &Eff,
    prices: &dyn PriceProvider,
    ctx: &FoldCtx,
    pools: &mut PoolSet,
    st: &mut LedgerState,
    stats: &mut FoldStats,
) {
    let date = eff.date();
    match &eff.op {
        Op::Acquire(a) => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::Unclassified,
                        Some(eff.id.clone()),
                        "acquire without wallet",
                    );
                    return;
                }
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: a.sat,
                remaining_sat: a.sat,
                usd_basis: a.usd_cost + a.fee_usd, // TP2: basis = cost + acquisition fee
                basis_source: a.basis_source,
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: false,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += a.sat; // FR9 Σin: externally-sourced acquisition
        }
        Op::Dispose {
            sat,
            proceeds,
            fee_usd,
            fee_sat,
            kind,
        } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "dispose without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            note_pre2025_once(
                st,
                date,
                &eff.id,
                ctx.config.pre2025_method,
                ctx.config.pre2025_method_attested,
            ); // §7.4: pre-2025 disposal advisory (once)
            let (consumed, shortfall) =
                consume_principal(pools, &key, *sat, date, ctx, st, &eff.id);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("dispose short by {shortfall} sat"),
                );
            }
            if !consumed.is_empty() {
                let net = round_cents(*proceeds - *fee_usd); // TP2: disposition fee reduces proceeds
                let mut legs = make_disposal_legs(&consumed, net, date, st, &eff.id);
                // I-1: Task 11 fee step — consume fee_sat FIFO from source pool AFTER principal.
                // Mirrors the gift/SelfTransfer pattern; native Dispose passes fee_sat=None (no-op).
                // (c) default: re-home carry onto last disposal leg; fee-sat basis rolls into the
                //     disposition (reported basis increases → gain decreases); fee non-taxable.
                // (b) config:  emits mini-disposition; returns empty carry; leg basis unchanged.
                let carry = consume_fee(
                    pools,
                    &key,
                    fee_sat.unwrap_or(0),
                    ctx.config,
                    prices,
                    date,
                    stats,
                    st,
                    &eff.id,
                );
                if let Some(last) = legs.last_mut() {
                    carry.rehome_onto_disposal_leg(last);
                } else if carry.gain_basis > Usd::ZERO {
                    // m3: degenerate guard — no surviving leg (principal == 0); unreachable for real events.
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "fee carry has no surviving disposal leg to re-home onto (principal == 0)",
                    );
                }
                st.disposals.push(Disposal {
                    event: eff.id.clone(),
                    kind: *kind,
                    disposed_at: date,
                    legs,
                    fee_mini_disposition: false,
                });
            }
        }
        Op::Income {
            sat,
            fmv,
            kind,
            business,
        } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income without wallet",
                    );
                    return;
                }
            };
            let (basis, pending) = match fmv {
                Some(v) => {
                    st.income_recognized.push(IncomeRecord {
                        event: eff.id.clone(),
                        recognized_at: date,
                        sat: *sat,
                        usd_fmv: *v,
                        kind: *kind,
                        business: *business,
                    });
                    (*v, false)
                }
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income FMV missing",
                    );
                    (Usd::ZERO, true) // basis pending; lot still created so Σsat conservation holds (§7.3)
                }
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis: basis,
                basis_source: BasisSource::FmvAtIncome,
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: pending,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat; // FR9 Σin: income is externally-sourced (counts even while FMV is pending)
        }
        Op::PendingOut { sat, fee_sat } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "pending out without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            let total_sat = *sat + fee_sat.unwrap_or(0);
            let (consumed, shortfall) = pools.consume_fifo(&key, total_sat);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("pending out short by {shortfall} sat"),
                );
            }
            let legs: Vec<PendingLeg> = consumed
                .iter()
                .map(|c| PendingLeg {
                    lot_id: c.lot_id.clone(),
                    sat: c.sat,
                    usd_basis: c.gain_basis,
                    acquired_at: c.acquired_at,
                })
                .collect();
            st.pending_reconciliation.push(PendingTransfer {
                event: eff.id.clone(),
                principal_sat: *sat,
                fee_sat: *fee_sat,
                legs,
            });
            // Advisory blocker: unmatched outflow (may be resolved by a later TransferLink in Task 8+).
            st.add_blocker(
                BlockerKind::UnmatchedOutflows,
                Some(eff.id.clone()),
                "unmatched transfer out",
            );
        }
        Op::SelfTransfer { sat, fee_sat, dest } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "self transfer without source wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            let (consumed, shortfall) =
                consume_principal(pools, &key, *sat, date, ctx, st, &eff.id);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("self transfer short by {shortfall} sat"),
                );
            }
            // Relocate consumed fragments to the destination pool: carry basis, HP, donor_acquired_at.
            // Non-taxable (TP7): no Disposal or Removal records. basis_source = CarriedFromTransfer.
            let mut relocated: Vec<Lot> = Vec::new();
            for c in &consumed {
                let seq = pools.bump_split(&c.lot_id.origin_event_id);
                relocated.push(Lot {
                    lot_id: LotId {
                        origin_event_id: c.lot_id.origin_event_id.clone(),
                        split_sequence: seq,
                    },
                    wallet: dest.clone(),
                    acquired_at: c.acquired_at,
                    original_sat: c.sat,
                    remaining_sat: c.sat,
                    usd_basis: c.gain_basis,
                    basis_source: BasisSource::CarriedFromTransfer,
                    dual_loss_basis: c.loss_basis,
                    donor_acquired_at: c.donor_acquired_at,
                    basis_pending: c.basis_pending,
                });
            }
            // Task 11: fee handling — consume fee_sat FIFO from source pool AFTER principal (FIFO order).
            // (c) default: returns FeeCarry to re-home onto relocated.last(), so FULL basis carries (C1).
            // (b) config:  emits mini-disposition; returns empty carry; destination lot stays at principal basis.
            let carry = consume_fee(
                pools,
                &key,
                fee_sat.unwrap_or(0),
                ctx.config,
                prices,
                date,
                stats,
                st,
                &eff.id,
            );
            if let Some(last) = relocated.last_mut() {
                carry.rehome_onto_lot(last);
            } else if carry.gain_basis > Usd::ZERO {
                // m3: degenerate guard — no surviving lot to re-home onto (principal == 0).
                // Unreachable for a real TransferLink (always moves principal > 0), but never silent.
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    "fee carry has no surviving lot to re-home onto (principal == 0)",
                );
            }
            let dest_key = pool_key(date, dest);
            for lot in relocated {
                pools.push_lot(dest_key.clone(), lot);
            }
        }
        Op::UnknownInbound { sat: _ } => {
            // Hard blocker: basis is unknown. NO lot — sats not yet in the ledger (FR9/§7.3).
            st.add_blocker(
                BlockerKind::UnknownBasisInbound,
                Some(eff.id.clone()),
                "unclassified TransferIn — basis unknown",
            );
        }
        Op::IncomeInbound {
            sat,
            fmv,
            kind,
            business,
        } => {
            // Identical to Op::Income: income lot at FMV + IncomeRecord. sigma_in += sat.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income inbound without wallet",
                    );
                    return;
                }
            };
            let (basis, pending) = match fmv {
                Some(v) => {
                    st.income_recognized.push(IncomeRecord {
                        event: eff.id.clone(),
                        recognized_at: date,
                        sat: *sat,
                        usd_fmv: *v,
                        kind: *kind,
                        business: *business,
                    });
                    (*v, false)
                }
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income inbound FMV missing",
                    );
                    (Usd::ZERO, true)
                }
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis: basis,
                basis_source: BasisSource::FmvAtIncome,
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: pending,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat;
        }
        Op::GiftReceived {
            sat,
            donor_basis,
            donor_acquired_at,
            fmv_at_gift,
        } => {
            // Task 10: §1015(a) dual-basis lot construction (TP11).
            // Four cases by (donor_basis, donor_acquired_at) × (fmv_at_gift vs donor_basis):
            //   1. donor_basis=Some(b), fmv_at_gift >= b  → single carryover (Gain zone only); tacks.
            //   2. donor_basis=Some(b), fmv_at_gift < b   → dual basis; tacks on gain side.
            //   3. donor_basis=None, donor_acquired_at=Some(d) → GiftFmvFallback: look up price at d.
            //   4. donor_basis=None, donor_acquired_at=None    → basis unknown; hard blocker + pending lot.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "gift received without wallet",
                    );
                    return;
                }
            };
            let (usd_basis, dual_loss_basis, basis_source, pending) = match donor_basis {
                Some(b) => {
                    if *fmv_at_gift >= *b {
                        // Case 1: FMV ≥ donor basis — single carryover; no dual.
                        (*b, None, BasisSource::GiftCarryover, false)
                    } else {
                        // Case 2: FMV < donor basis — dual: gain basis = donor basis, loss basis = FMV.
                        (*b, Some(*fmv_at_gift), BasisSource::GiftCarryover, false)
                    }
                }
                None => match donor_acquired_at {
                    Some(d) => {
                        // Case 3: GiftFmvFallback — derive basis from BTC price at donor's acquisition date.
                        match fmv_of(prices, *d, *sat) {
                            Some(fmv) => (fmv, None, BasisSource::GiftFmvFallback, false),
                            None => {
                                // Price unavailable at donor acquisition date → basis indeterminate.
                                st.add_blocker(
                                    BlockerKind::UnknownBasisInbound,
                                    Some(eff.id.clone()),
                                    "gift received: donor basis unknown and price unavailable at donor acquisition date",
                                );
                                (Usd::ZERO, None, BasisSource::GiftFmvFallback, true)
                            }
                        }
                    }
                    None => {
                        // Case 4: both donor basis and acquisition date unknown — hard blocker.
                        st.add_blocker(
                            BlockerKind::UnknownBasisInbound,
                            Some(eff.id.clone()),
                            "gift received: donor basis and acquisition date both unknown",
                        );
                        (Usd::ZERO, None, BasisSource::GiftCarryover, true)
                    }
                },
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis,
                basis_source,
                dual_loss_basis,
                donor_acquired_at: *donor_acquired_at,
                basis_pending: pending,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat; // classified GiftReceived is externally-sourced (FR9)
        }
        Op::SelfTransferInbound {
            sat,
            basis,
            acquired_at,
        } => {
            // Cycle A: "my own coins" returning — a NON-taxable receipt that CREATES a fresh origin lot.
            // Modeled on Op::IncomeInbound (fold.rs), but (G1) NEVER basis_pending, (G2) pushes NO
            // IncomeRecord, and (G3) donor_acquired_at: None (not a gift — no tacking).
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    // [R0-M2 / G5] No wallet → nowhere to create the lot. Emit a Hard UnknownBasisInbound
                    // with a self-transfer message (NOT the IncomeInbound FmvMissing guard, which is
                    // semantically wrong for a non-income receipt) and return without creating a lot.
                    st.add_blocker(
                        BlockerKind::UnknownBasisInbound,
                        Some(eff.id.clone()),
                        "self-transfer inbound without wallet — nowhere to create the lot",
                    );
                    return;
                }
            };
            let usd_basis = basis.unwrap_or(Usd::ZERO); // conservative $0 default (max eventual gain)
            let acq = acquired_at.unwrap_or(date); // conservative receipt-date default (date = event date)
            if basis.is_none() {
                // (G4) ADVISORY only — fires on None, NOT on usd_basis == 0 (an attested Some(0) is silent).
                st.add_blocker(
                    BlockerKind::SelfTransferInboundZeroBasis,
                    Some(eff.id.clone()),
                    "basis defaulted to $0 — likely overstates your eventual gain; supply real cost if \
                     you have it (btctax reconcile classify-inbound-self-transfer --basis). Holding \
                     period also defaults to the receipt date (short-term) unless --acquired is supplied.",
                );
            }
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: acq, // HP start; gain_hp_start() == acq (donor_acquired_at is None → no tacking)
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis,
                basis_source: BasisSource::SelfTransferInbound,
                dual_loss_basis: None,
                donor_acquired_at: None, // NOT a gift — it's your own coin
                basis_pending: false, // (G1) $0 is computable → NEVER gated (contrast Income-FMV-missing)
            };
            // pool_key uses the RECEIPT date (`date`), while acquired_at carries the supplied-or-receipt
            // date — orthogonal (mirrors the gift path): a real old date lands in the receipt-year pool.
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat; // FR9 Σin: coins enter the ledger (externally-sourced)
        }
        Op::GiftOut {
            sat,
            fmv,
            fee_sat,
            donee,
            ..
        } => {
            // TP10: gift outbound → Removal with zero recognized gain; no Disposal.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "gift out without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            note_pre2025_once(
                st,
                date,
                &eff.id,
                ctx.config.pre2025_method,
                ctx.config.pre2025_method_attested,
            ); // §7.4: pre-2025 removal advisory (once)
            let (consumed, shortfall) =
                consume_principal(pools, &key, *sat, date, ctx, st, &eff.id);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("gift out short by {shortfall} sat"),
                );
            }
            if !consumed.is_empty() {
                let (mut legs, donor_acquired_at) =
                    make_removal_legs(&consumed, *fmv, date, st, &eff.id);
                // Task 11: fee step — consume fee_sat FIFO from source pool AFTER principal.
                // (c) default: re-home carry onto last removal leg so donee carries FULL basis (C1).
                // (b) config:  emits mini-disposition; empty carry; donee gets principal-only basis.
                let carry = consume_fee(
                    pools,
                    &key,
                    fee_sat.unwrap_or(0),
                    ctx.config,
                    prices,
                    date,
                    stats,
                    st,
                    &eff.id,
                );
                if let Some(last) = legs.last_mut() {
                    carry.rehome_onto_removal_leg(last);
                } else if carry.gain_basis > Usd::ZERO {
                    // m3: degenerate guard (unreachable for real gifts, which always move principal > 0).
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "fee carry has no surviving removal leg to re-home onto (principal == 0)",
                    );
                }
                st.removals.push(Removal {
                    event: eff.id.clone(),
                    kind: RemovalKind::Gift,
                    removed_at: date,
                    legs,
                    appraisal_required: false,
                    donor_acquired_at,
                    claimed_deduction: None,
                    donee: donee.clone(),
                });
            }
        }
        Op::Donate {
            sat,
            fmv,
            appraisal_required,
            fee_sat,
            donee,
            ..
        } => {
            // TP10: donation outbound → Removal with zero recognized gain; no Disposal.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "donate without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            note_pre2025_once(
                st,
                date,
                &eff.id,
                ctx.config.pre2025_method,
                ctx.config.pre2025_method_attested,
            ); // §7.4: pre-2025 removal advisory (once)
            let (consumed, shortfall) =
                consume_principal(pools, &key, *sat, date, ctx, st, &eff.id);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("donate short by {shortfall} sat"),
                );
            }
            if !consumed.is_empty() {
                let (mut legs, donor_acquired_at) =
                    make_removal_legs(&consumed, *fmv, date, st, &eff.id);
                // Task 11: fee step — consume fee_sat FIFO from source pool AFTER principal.
                // (c) default: re-home carry onto last removal leg so donee carries FULL basis (C1).
                // (b) config:  emits mini-disposition; empty carry; donee gets principal-only basis.
                let carry = consume_fee(
                    pools,
                    &key,
                    fee_sat.unwrap_or(0),
                    ctx.config,
                    prices,
                    date,
                    stats,
                    st,
                    &eff.id,
                );
                if let Some(last) = legs.last_mut() {
                    carry.rehome_onto_removal_leg(last);
                } else if carry.gain_basis > Usd::ZERO {
                    // m3: degenerate guard (unreachable for real donations, which always move principal > 0).
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "fee carry has no surviving removal leg to re-home onto (principal == 0)",
                    );
                }
                // §170(e)(1)(A) charitable-deduction amount: exact for a non-dealer individual
                // investor donating a capital asset to a public charity (the modeled universe).
                // Computed from the FINAL persisted legs (after both make_removal_legs AND
                // carry.rehome_onto_removal_leg) so the re-homed fee-cent basis is included.
                // LT legs → FMV (the LT-capital-gain deduction under §170(e)); would-be gain is
                // LTCG → no reduction; also FMV when depreciated (a would-be loss is not a reduction).
                // ST legs → min(FMV, basis): when appreciated (basis<FMV) the ST gain reduces FMV
                // to basis; when depreciated (basis>FMV) there is no would-be gain → deduction = FMV.
                // Stored on the Removal; also drives §170(f)(11)(C) QualifiedAppraisalNote.
                // Does NOT read or modify the user's `appraisal_required` bool (independent cross-check).
                let claimed_deduction: Usd = legs
                    .iter()
                    .map(|leg| {
                        if leg.term == Term::LongTerm {
                            leg.fmv_at_transfer
                        } else {
                            leg.fmv_at_transfer.min(leg.basis)
                        }
                    })
                    .sum();
                if claimed_deduction > crate::tax::tables::QUALIFIED_APPRAISAL_THRESHOLD {
                    st.add_blocker(
                        BlockerKind::QualifiedAppraisalNote,
                        Some(eff.id.clone()),
                        format!(
                            "Claimed deduction ${claimed_deduction:.2} exceeds the \
                             §170(f)(11)(C) $5,000 threshold. Qualified appraisal likely required: \
                             CCA 202302012 — a crypto donation with a claimed deduction >$5,000 \
                             requires a qualified appraisal; the exchange-price/readily-valued \
                             exception does NOT apply to crypto. \
                             This is the exact §170(e) deduction for a non-dealer individual \
                             investor donating a capital asset (LT→FMV; ST→min(FMV,basis)). \
                             Caveat (a) dealer/inventory: crypto held as inventory/for sale in a \
                             trade or business (§1221(a)(1)) or other ordinary-income property \
                             deducts at basis under §170(e) REGARDLESS of holding period — this \
                             figure assumes capital-asset (investor) status and would OVER-STATE \
                             for a dealer; verify. \
                             Caveat (b) donee type: LT→FMV assumes a public charity (50%-limit org); \
                             a non-operating private foundation reduces appreciated LT crypto to \
                             basis (§170(e)(1)(B)(ii); crypto is not qualified appreciated stock) — \
                             donee type is not modeled; would OVER-STATE for a private-foundation \
                             gift; verify. \
                             §170(f)(11)(F) aggregation: this flags a single donation; the $5,000 \
                             test also aggregates similar donated items across the tax year — \
                             cross-donation aggregation is not considered here."
                        ),
                    );
                }
                st.removals.push(Removal {
                    event: eff.id.clone(),
                    kind: RemovalKind::Donation,
                    removed_at: date,
                    legs,
                    appraisal_required: *appraisal_required,
                    donor_acquired_at,
                    claimed_deduction: Some(claimed_deduction),
                    donee: donee.clone(),
                });
            }
        }
        Op::Unclassified => {
            st.add_blocker(
                BlockerKind::Unclassified,
                Some(eff.id.clone()),
                "unclassified BTC-side row",
            );
        }
        Op::Skip => {}
    }
}

/// Collect remaining lots + holdings; sort all output deterministically (NFR4); commit the FoldStats (M3).
pub fn finalize(st: &mut LedgerState, pools: PoolSet, mut stats: FoldStats) {
    let mut holdings: BTreeMap<crate::identity::WalletId, Sat> = BTreeMap::new();
    let mut lots: Vec<Lot> = Vec::new();
    for (_key, pool) in pools.pools {
        for lot in pool {
            if lot.remaining_sat > 0 {
                *holdings.entry(lot.wallet.clone()).or_insert(0) += lot.remaining_sat;
                lots.push(lot);
            }
        }
    }
    lots.sort_by(|a, b| {
        a.wallet
            .cmp(&b.wallet)
            .then(a.acquired_at.cmp(&b.acquired_at))
            .then(a.lot_id.cmp(&b.lot_id))
    });
    st.lots = lots;
    st.holdings_by_wallet = holdings;
    // M1: sort blockers by the DERIVED Ord of (kind, Option<EventId>, detail) — a total order, no Debug strings.
    st.blockers.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.event.cmp(&b.event))
            .then_with(|| a.detail.cmp(&b.detail))
    });
    // Σpending is reconstructable from the queue; sigma_in/fee_sats_consumed are accumulated during the fold.
    stats.sigma_pending = st
        .pending_reconciliation
        .iter()
        .map(|p| p.principal_sat + p.fee_sat.unwrap_or(0))
        .sum();
    st.stats = stats;
}
