//! KATs for BG-D3 — the `filed_basis` compute + `Coverage::Full` hard-refuse guard (Task 2 of the
//! conservative-filing promotion engine). `filed_basis_for` is a PURE wrapper over `window_reference`
//! (conservative.rs): it turns the window's min daily CLOSE (a per-BTC PRICE) into a whole-tranche basis
//! by scaling `min * sat / SATS_PER_BTC` (the SAME formula `overpayment_delta_one` uses), and REFUSES to
//! produce a floor unless the window has `Coverage::Full` (a `Partial`-covered min can EXCEED the true
//! window min — conservative.rs `window_reference` doc; filing on it would UNDERSTATE a floor).
//!
//! Task 3 adds the by-construction KATs for the PASS-2 rewrite itself (BG-D1, `resolve.rs` step 2): a
//! `PromoteTranche` decision rewrites its target `DeclareTranche`'s `Op::Acquire.usd_cost` to the stored
//! `filed_basis` — INSIDE `resolve`, before step-3's `universal_snapshot` runs — while `basis_source`
//! stays `EstimatedConservative` (no new tag) and `acquired_at`/the relocation tag-carry are untouched.
//! PRIVACY: synthetic values only.

use btctax_core::conservative::Coverage;
use btctax_core::conservative_promote::{
    clamped_leg_basis, filed_basis_for, PromoteEntry, PromoteRefusal,
};
use btctax_core::event::*;
use btctax_core::forms::form_8283;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{
    evaluate_disposal, project, would_conflict, CandidateDisposal, LotMethod, ProjectionConfig,
};
use btctax_core::state::{BlockerKind, DisposalLeg, LedgerState, RemovalKind};
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::return_inputs::{Owner, ReturnInputs, ScheduleAInputs, W2};
use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
use btctax_core::tax::FilingStatus;
use btctax_core::voidable_decisions;
use btctax_core::Usd;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

// ── fixture harness (mirrors tests/kat_tranche.rs / tests/kat_conservative.rs) ─────────────────────

/// A short, FULLY-covered window (2017-12-01..2017-12-03) whose min daily close is `min_price`.
fn prices_with_window_min(min_price: i64) -> StaticPrices {
    StaticPrices(
        [
            (
                date!(2017 - 12 - 01),
                rust_decimal::Decimal::from(min_price),
            ),
            (
                date!(2017 - 12 - 02),
                rust_decimal::Decimal::from(min_price + 3_000),
            ),
            (
                date!(2017 - 12 - 03),
                rust_decimal::Decimal::from(min_price + 5_000),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

/// A window (2013-01-01..2013-01-03) where the middle day has NO bundled close — a gap, so
/// `window_reference` returns `Coverage::Partial` over the covered days.
fn prices_with_partial_window() -> StaticPrices {
    StaticPrices(
        [
            (date!(2013 - 01 - 01), dec!(100)),
            // 2013-01-02 missing — the gap.
            (date!(2013 - 01 - 03), dec!(80)),
        ]
        .into_iter()
        .collect(),
    )
}

/// A window with NO bundled close on any day — `window_reference` returns `None`.
fn prices_with_no_coverage() -> StaticPrices {
    StaticPrices(
        [(date!(2019 - 06 - 01), dec!(9_000))] // outside the queried window
            .into_iter()
            .collect(),
    )
}

/// BG-D3: whole-tranche scaling (per-BTC price × sat / SATS_PER_BTC), NOT a per-BTC price.
#[test]
fn filed_basis_is_whole_tranche_scaled() {
    let prices = prices_with_window_min(12_000); // min daily close = $12,000/BTC, Full coverage
    let cf = filed_basis_for(
        &prices,
        50_000_000, // 0.5 BTC
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 03),
    )
    .unwrap();
    assert_eq!(cf.filed_basis, dec!(6_000)); // 12_000 × 0.5, not 12_000
    assert_eq!(cf.coverage, Coverage::Full);
}

/// BG-D3: a Coverage::Partial window is HARD-refused — never file a floor that could exceed the true
/// window min.
#[test]
fn partial_coverage_is_hard_refused() {
    let prices = prices_with_partial_window(); // 2013-01-02 has no close
    let err = filed_basis_for(
        &prices,
        100_000_000,
        date!(2013 - 01 - 01),
        date!(2013 - 01 - 03),
    )
    .unwrap_err();
    assert!(matches!(err, PromoteRefusal::PartialCoverage));
}

/// BG-D3: a window with NO covered day at all is likewise HARD-refused (`NoCoverage`) — never fabricate
/// a floor over a total data gap.
#[test]
fn no_coverage_is_hard_refused() {
    let prices = prices_with_no_coverage();
    let err = filed_basis_for(
        &prices,
        100_000_000,
        date!(2013 - 01 - 01),
        date!(2013 - 01 - 03),
    )
    .unwrap_err();
    assert!(matches!(err, PromoteRefusal::NoCoverage));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 3 — the pass-2 rewrite itself (BG-D1), by construction. Fixture harness mirrors
// tests/kat_tranche.rs (`dec_ev`/`tranche_ev`/`void_ev`/`imp`/`sell_ev`/`self_transfer`/`alloc_ev`).
// ════════════════════════════════════════════════════════════════════════════════════════════════

fn exch() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: p,
    }
}
/// A DeclareTranche decision event (mirrors kat_tranche.rs `tranche_ev`).
fn tranche_ev(seq: u64, w: &WalletId, sat: i64, ws: time::Date, we: time::Date) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-01-01 00:00 UTC),
        EventPayload::DeclareTranche(DeclareTranche {
            sat,
            wallet: w.clone(),
            window_start: ws,
            window_end: we,
        }),
    )
}
/// A PromoteTranche decision event: promotes `target`'s $0 basis to `filed_basis` (BG-D1/D2/D3).
/// The consent/attestation/narrative fields are fixed valid placeholders — Task 3 exercises the
/// resolve-side rewrite, not the record-time guard (narrative-emptiness validation is Task 10's).
fn promote_ev(seq: u64, target: EventId, filed_basis: rust_decimal::Decimal) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-02-01 00:00 UTC),
        EventPayload::PromoteTranche(PromoteTranche {
            target,
            method: FloorMethod::WindowLowClose,
            filed_basis,
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I understand and accept the risk".into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
    )
}
fn imp(rf: &str, ts: time::OffsetDateTime, w: &WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: p,
    }
}
fn sell_ev(
    rf: &str,
    ts: time::OffsetDateTime,
    w: &WalletId,
    sat: i64,
    proceeds: i64,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: rust_decimal::Decimal::from(proceeds),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
/// A confirmed self-transfer: TransferOut (from) + TransferIn (to) + a TransferLink decision
/// (mirrors kat_tranche.rs `self_transfer`).
fn self_transfer(
    out_rf: &str,
    in_rf: &str,
    from: &WalletId,
    to: &WalletId,
    sat: i64,
    ts: time::OffsetDateTime,
    link_seq: u64,
) -> Vec<LedgerEvent> {
    vec![
        imp(
            out_rf,
            ts,
            from,
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            in_rf,
            ts,
            to,
            EventPayload::TransferIn(TransferIn {
                sat,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            link_seq,
            ts,
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new(out_rf)),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Coinbase,
                    SourceRef::new(in_rf),
                )),
            }),
        ),
    ]
}
/// A SafeHarborAllocation decision event (mirrors kat_tranche.rs `alloc_ev`).
fn alloc_ev(
    seq: u64,
    attested: bool,
    pre2025_method: LotMethod,
    lots: Vec<AllocLot>,
) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2024-12-01 00:00 UTC),
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots,
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ActualPosition,
            timely_allocation_attested: attested,
            pre2025_method,
        }),
    )
}
fn alloc_lot(w: &WalletId, sat: i64, basis: i64, acq: time::Date) -> AllocLot {
    AllocLot {
        wallet: w.clone(),
        sat,
        usd_basis: rust_decimal::Decimal::from(basis),
        acquired_at: acq,
        dual_loss_basis: None,
        donor_acquired_at: None,
    }
}
fn prices() -> StaticPrices {
    StaticPrices::default()
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default()
}

/// ★ The load-bearing KAT (BG-D1): a promoted tranche's lot reads the FILED floor as `usd_basis`, while
/// `basis_source` stays `EstimatedConservative` (no new tag — the D-8 backstop keys on the TAG) and
/// `acquired_at` stays `window_end` (term-invariance: the rewrite touches ONLY `usd_cost`).
#[test]
fn promote_rewrites_usd_cost_but_keeps_the_tag() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let st = project(&[t, p], &prices(), &cfg());
    let lot = st.lots.iter().find(|l| l.wallet == w).unwrap();
    assert_eq!(
        lot.usd_basis,
        dec!(12_000),
        "usd_cost rewritten to the floor"
    );
    assert_eq!(
        lot.basis_source,
        BasisSource::EstimatedConservative,
        "NO new BasisSource (BG-D1)"
    );
    assert_eq!(
        lot.acquired_at,
        date!(2017 - 12 - 31),
        "term-invariance: acquired_at still window_end (BG-D9/M-7)"
    );
}

/// BG-D1: the D-8 backstop keys on the `EstimatedConservative` TAG, not the $0 amount — so a PROMOTED
/// (>$0) tranche still denies a `SafeHarborAllocation` effectiveness, even one whose totals are crafted
/// to match the PROMOTED residue exactly (proving the denial isn't a totals-mismatch coincidence — mirrors
/// kat_tranche.rs's Task-5 backstop KAT `allocation_that_would_conserve_over_a_tranche_residue_...`).
#[test]
fn promoted_pre2025_tranche_still_trips_the_d8_backstop() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(4_200));
    let a = alloc_ev(
        3,
        true,
        LotMethod::Hifo,
        vec![alloc_lot(&w, 100_000_000, 4_200, date!(2018 - 12 - 31))],
    );
    let st = project(&[t, p, a], &prices(), &cfg());
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SafeHarborUnconservable),
        "a promoted pre-2025 tranche still trips the D-8 backstop (tag-keyed, BG-D1): {:?}",
        st.blockers
    );
    let lot = st
        .lots
        .iter()
        .find(|l| l.basis_source == BasisSource::EstimatedConservative)
        .expect("the promoted tranche survives via Path A (not discarded by Path B)");
    assert_eq!(
        lot.usd_basis,
        dec!(4_200),
        "the floor rides Path A untouched"
    );
}

/// ★ BG-D1 / arch r1 M-3 — the LOAD-BEARING placement guarantee: the rewrite must land INSIDE `resolve`'s
/// step 2, so step-3's `universal_snapshot` folds the PROMOTED (not the stale $0) tranche when evaluating
/// a `SafeHarborAllocation`'s conservation.
///
/// Design (why this actually discriminates the timing, unlike a bare presence-check): HIFO consumption
/// order is PER-SAT-COST keyed (`pools.rs::hifo_cmp`) — a `$0` lot is EXPLICITLY sorted LAST regardless of
/// remaining_sat; once promoted to a floor whose per-sat cost EXCEEDS a co-held documented lot's, it sorts
/// FIRST instead. A pre-2025 disposal sized to EXACTLY the tranche's own sat therefore drains a DIFFERENT
/// lot depending on whether the floor is visible at fold time:
///   - floor VISIBLE (this task, rewrite inside step 2): the disposal drains the (now-highest-cost)
///     tranche FIRST and EXACTLY exhausts it (fully consumed → removed from the pool, `remaining_sat=0`
///     ⇒ the D-8 `has_tranche_residue` prong is FALSE), leaving the documented lot COMPLETELY untouched.
///     The 2025-01-01 residue is the documented lot alone (60M sat / $3,000) — an allocation listing
///     exactly that conserves cleanly: NO `SafeHarborUnconservable`.
///   - floor BLIND (the `overpayment_delta_one`/post-resolve timing bug — equivalent, from `resolve`'s own
///     step-3 perspective, to never having applied the rewrite at all): the tranche still sorts LAST
///     (its `usd_cost` never left `$0` inside THIS `resolve()` call), so the SAME disposal instead drains
///     the documented lot first, leaving the TRANCHE fully intact — its own residue trips `has_tranche_residue`
///     AND its basis ($0) leaves `snap.basis` at $1,000 instead of $3,000. EITHER way the SAME allocation
///     now fails conservation.
///
/// So "this allocation conserves cleanly" is possible ONLY when step 3 saw the floor. Reverting the
/// rewrite's placement (or removing it) flips this test from green to a wrongly-fired
/// `SafeHarborUnconservable` — red, exactly the "Mutation to kill" the brief names.
#[test]
fn snapshot_timing_the_floor_is_visible_to_pass1_conservation() {
    let w = exch();
    // The promoted tranche: 0.4 BTC, filed floor $12,000 ⇒ $30,000/BTC — well above the documented lot's
    // per-sat cost below, so once promoted it ranks FIRST under HIFO (unpromoted, $0, it would rank LAST).
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2015 - 01 - 01),
        date!(2015 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    // A documented lot: 0.6 BTC for $3,000 ($5,000/BTC) — cheaper per-sat than the promoted floor.
    let buy = imp(
        "BUY",
        datetime!(2014-06-01 00:00 UTC),
        &w,
        EventPayload::Acquire(Acquire {
            sat: 60_000_000,
            usd_cost: dec!(3_000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    // A pre-2025 disposal for EXACTLY the tranche's own sat — drains a DIFFERENT lot depending on whether
    // the floor is visible (see doc above).
    let sell = sell_ev(
        "SELL",
        datetime!(2016-06-01 00:00 UTC),
        &w,
        40_000_000,
        50_000,
    );
    // An allocation listing EXACTLY the floor-VISIBLE residue: the untouched documented lot alone.
    let a = alloc_ev(
        4,
        true,
        LotMethod::Hifo,
        vec![alloc_lot(&w, 60_000_000, 3_000, date!(2014 - 06 - 01))],
    );
    let st = project(&[t, p, buy, sell, a], &prices(), &cfg());
    assert!(
        !st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SafeHarborUnconservable),
        "conservation adjudicated against the FLOOR-visible residue (rewrite is in step 2, not \
         post-resolve) — a floor-blind snapshot would instead see the tranche undrained (still ranked \
         last under HIFO) and wrongly deny effectiveness: {:?}",
        st.blockers
    );
}

/// §6: a promoted tranche self-transferred Exchange→SelfCustody keeps BOTH `EstimatedConservative` AND
/// the floor (fold.rs `SelfTransfer` relocation carries `usd_basis`/tag verbatim from the already-rewritten
/// source lot — no fold.rs change needed; the rewrite happening upstream, in `resolve`, is what makes this
/// hold by construction). Post-2025 tranche (mirrors kat_tranche.rs `tranche_tag_survives_self_transfer_relocation`)
/// isolates the relocation exemption from pre-2025 Path-A reconstruction.
#[test]
fn relocated_promoted_tranche_keeps_tag_and_floor() {
    let ex = exch();
    let sc = cold();
    let t = tranche_ev(
        1,
        &ex,
        50_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 02 - 01),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let mut evs = vec![t, p];
    evs.extend(self_transfer(
        "XO",
        "XI",
        &ex,
        &sc,
        50_000_000,
        datetime!(2025-03-01 00:00 UTC),
        9,
    ));
    let st = project(&evs, &prices(), &cfg());
    let lot = st
        .lots
        .iter()
        .find(|l| matches!(l.wallet, WalletId::SelfCustody { .. }))
        .expect("a relocated lot in self-custody");
    assert_eq!(
        lot.basis_source,
        BasisSource::EstimatedConservative,
        "tag survives relocation (D-8)"
    );
    assert_eq!(
        lot.usd_basis,
        dec!(12_000),
        "the floor rides the relocation"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 4 — BG-D4 disposal-leg loss clamp (net − documented) + stored-`filed_basis` decomposition +
// PromoteSet threaded through ALL FOUR FoldCtx sites. The clamp NEVER manufactures a loss off the
// estimate; a promoted-tranche leg's estimate-attributable gain is ≥ 0 and its estimate basis is ≥ $0.
// PRIVACY: synthetic values only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The single disposal leg across the whole projection (a promoted tranche drains as one lot).
fn only_disposal_leg(st: &LedgerState) -> &DisposalLeg {
    let legs: Vec<&DisposalLeg> = st.disposals.iter().flat_map(|d| &d.legs).collect();
    assert_eq!(legs.len(), 1, "exactly one disposal leg in this scenario");
    legs[0]
}

/// A documented Acquire (real basis) the fee-draw will consume.
fn documented_buy(
    rf: &str,
    ts: time::OffsetDateTime,
    w: &WalletId,
    sat: i64,
    cost: i64,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: rust_decimal::Decimal::from(cost),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}

/// A confirmed self-transfer paying an on-chain `fee_sat` (mirrors `self_transfer`, adds the fee).
/// The fee-sats are consumed FIFO from the SOURCE pool AFTER principal (TP8(c), TreatmentC default).
#[allow(clippy::too_many_arguments)]
fn self_transfer_with_fee(
    out_rf: &str,
    in_rf: &str,
    from: &WalletId,
    to: &WalletId,
    sat: i64,
    fee_sat: i64,
    ts: time::OffsetDateTime,
    link_seq: u64,
) -> Vec<LedgerEvent> {
    vec![
        imp(
            out_rf,
            ts,
            from,
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: Some(fee_sat),
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            in_rf,
            ts,
            to,
            EventPayload::TransferIn(TransferIn {
                sat,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            link_seq,
            ts,
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new(out_rf)),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Coinbase,
                    SourceRef::new(in_rf),
                )),
            }),
        ),
    ]
}

fn promote_entry(filed_basis: rust_decimal::Decimal, tranche_sat: i64) -> PromoteEntry {
    PromoteEntry {
        filed_basis,
        tranche_sat,
    }
}

// ── clamp-formula corners (direct, confounder-free unit tests of `clamped_leg_basis`) ──────────────

/// ★ the `net − documented` bound (Opus r3 tax I-1): documented ALONE (> net) still reaches a REAL
/// §1001(b) loss — the estimate evaporates to $0, reported basis = the documented component alone.
#[test]
fn a_genuine_documented_loss_still_reaches_negative() {
    let e = promote_entry(dec!(12_000), 100_000_000);
    // usd_basis $17k = $12k estimate + $5k documented; sold at net $3k. documented ($5k) > net ($3k).
    let basis = clamped_leg_basis(Some(&e), 100_000_000, dec!(17_000), dec!(3_000));
    assert_eq!(
        basis,
        dec!(5_000),
        "estimate → $0; reported = documented alone"
    );
    assert!(
        dec!(3_000) - basis < dec!(0),
        "gain = net − basis < 0: a genuine documented loss (attribution intact)"
    );
}

/// ★ the crowd-out band `estimate ≤ net < estimate + documented` — the `net − documented` bound files $0,
/// where a bare-`net` bound would file an estimate-ENABLED loss.
#[test]
fn sold_just_above_floor_band_still_files_zero_gain() {
    let e = promote_entry(dec!(12_000), 100_000_000);
    // usd_basis $14k = $12k estimate + $2k documented; sold at net $13k. estimate_basis = min(12k, 13k−2k).
    let basis = clamped_leg_basis(Some(&e), 100_000_000, dec!(14_000), dec!(13_000));
    assert_eq!(
        basis,
        dec!(13_000),
        "estimate fills only the room documented leaves → gain $0 (bare-net would file −$1,000)"
    );
}

/// The `max(·, $0)` floor: `fee_usd > proceeds` drives net < $0, but the estimate basis never goes negative.
#[test]
fn estimate_basis_never_goes_negative_when_fee_exceeds_proceeds() {
    let e = promote_entry(dec!(12_000), 100_000_000);
    let basis = clamped_leg_basis(Some(&e), 100_000_000, dec!(12_000), dec!(-500)); // net < 0
    assert_eq!(
        basis,
        dec!(0),
        "estimate basis floored at $0, never negative"
    );
    assert!(basis >= dec!(0));
}

/// The `None` arm is the identity: a non-promoted lot's basis is returned unchanged (behavior-preserving).
#[test]
fn clamped_leg_basis_is_identity_when_not_promoted() {
    assert_eq!(
        clamped_leg_basis(None, 100_000_000, dec!(5_000), dec!(3_000)),
        dec!(5_000),
        "None ⇒ usd_basis_share unchanged"
    );
}

// ── end-to-end clamp through the fold + optimizer (threaded FoldCtx) ────────────────────────────────

/// A promoted tranche sold below its window-low floor files $0 gain, NOT a loss off the estimate —
/// reported basis = proceeds (fold `make_disposal_legs`, `fold`'s FoldCtx).
#[test]
fn floor_below_window_low_files_zero_gain_not_a_loss() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let sell = sell_ev(
        "SELL",
        datetime!(2025-06-01 00:00 UTC),
        &w,
        100_000_000,
        8_000,
    );
    let st = project(&[t, p, sell], &prices(), &cfg());
    let leg = only_disposal_leg(&st);
    assert_eq!(leg.gain, dec!(0), "estimate gain clamped ≥ 0 (BG-D4)");
    assert_eq!(
        leg.basis, leg.proceeds,
        "basis = proceeds; no fabricated loss"
    );
}

/// ★ Opus r3 tax I-1: a promoted tranche self-transferred with a $30 DOCUMENTED fee carry, then sold at
/// net $8k, files gain $0 — the estimate yields to the documented fee (`net − documented`), NOT the −$30
/// estimate-ENABLED loss a bare-`net` bound would file. The documented $30 is drawn from a SEPARATE
/// documented lot (so it survives T5's fee-evaporation): HIFO takes the (highest-per-sat) tranche for
/// principal, the FIFO fee-draw then takes the documented remainder, and `rehome_onto_lot` bakes the $30
/// into the relocated tranche's `usd_basis` ($12,030) BEFORE the sale — the exact SPEC crowd-out corner.
#[test]
fn relocated_with_fee_then_promoted_sold_below_floor_files_zero_gain_not_an_estimate_enabled_loss()
{
    let ex = exch();
    let sc = cold();
    let t = tranche_ev(
        1,
        &ex,
        100_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    // A separate documented lot the fee FIFO-draws from: 0.3 BTC for $30 (cheap per-sat, so HIFO takes
    // the tranche for principal and leaves this whole lot for the fee → a $30 documented carry).
    let doc = documented_buy("DOC", datetime!(2025-01-15 00:00 UTC), &ex, 30_000_000, 30);
    let mut evs = vec![t, p, doc];
    evs.extend(self_transfer_with_fee(
        "XO",
        "XI",
        &ex,
        &sc,
        100_000_000,
        30_000_000,
        datetime!(2025-02-01 00:00 UTC),
        9,
    ));
    evs.push(sell_ev(
        "SELL",
        datetime!(2025-06-01 00:00 UTC),
        &sc,
        100_000_000,
        8_000,
    ));
    let st = project(&evs, &prices(), &cfg());
    let leg = only_disposal_leg(&st);
    assert_eq!(
        leg.gain,
        dec!(0),
        "no estimate-enabled loss; estimate yields to the documented fee (net − documented)"
    );
}

/// The clamp also reaches a PRE-2025 disposal (Universal-pool fold path), not only post-2025 wallet-pool
/// sales. (The `universal_snapshot` FoldCtx now carries the SAME promote set — the T5 fee-evaporation
/// precondition; its snapshot-basis discrimination activates with T5's evaporation logic.)
#[test]
fn a_pre2025_promoted_disposal_below_floor_clamps_on_the_real_fold_path() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let sell = sell_ev(
        "SELL",
        datetime!(2020-06-01 00:00 UTC),
        &w,
        100_000_000,
        8_000,
    );
    let st = project(&[t, p, sell], &prices(), &cfg());
    assert_eq!(
        only_disposal_leg(&st).gain,
        dec!(0),
        "the clamp reaches pre-2025 (Universal-pool) disposals too"
    );
}

/// ★ Opus r3 arch I-1: the optimizer's per-disposal scoring (`evaluate_disposal` → `fold`, the SAME
/// FoldCtx the real fold uses) applies the clamp — a promoted below-floor synthetic sale scores gain $0,
/// not a phantom loss. Threading an empty promote set into that fold would file the −$4,000 phantom.
#[test]
fn the_optimizer_sees_the_clamped_promoted_basis_not_a_phantom() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let cand = CandidateDisposal {
        existing_event: None,
        wallet: w.clone(),
        date: date!(2025 - 06 - 01),
        sat: 100_000_000,
        kind: DisposeKind::Sell,
        proceeds: Some(dec!(8_000)), // below the $12k floor
    };
    let out = evaluate_disposal(&[t, p], &prices(), &cfg(), &cand, None).unwrap();
    assert_eq!(
        out.st_gain + out.lt_gain,
        dec!(0),
        "the optimizer scores the clamped promoted basis (gain $0, not the −$4k phantom)"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 5 — BG-D4 fee-draw evaporation at `consume_fee` (TreatmentC): a promoted tranche is usually the
// OLDEST lot, so a FIFO fee draw hits it first. `rehome_onto_lot` would otherwise re-home the RAW
// floor-derived `gain_basis` onto the survivor, letting estimate money leak into a later disposal's
// reported basis — a below-floor sale could then file a loss that is 100% estimate money. Task 5 makes
// the ESTIMATE component of a consumed promoted fee fragment EVAPORATE; only the documented remainder
// re-homes. PRIVACY: synthetic values only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The worked corner from the review: 1 BTC promoted to $12,000; a self-transfer moves the WHOLE
/// position (no separate documented lot exists), paying a 10,000-sat fee drawn FIFO from the tranche —
/// its only, hence oldest, lot. The fee's own $1.20 floor-derived basis must EVAPORATE rather than
/// re-home onto the relocated surviving lot.
///
/// The final sale's proceeds are deliberately $0 (not a realistic price): the D-4 crowd-out clamp
/// (`clamped_leg_basis`) already absorbs a small un-evaporated leak invisibly whenever proceeds exceed
/// it (reported gain clamps to $0 either way — the leak is invisible at a normal "sold below the $12k
/// floor" price like $8,000). The leak only surfaces as a REAL reported LOSS once it exceeds proceeds,
/// so $0 is the sharpest value that discriminates: WITHOUT the fix the un-evaporated $1.20 re-homes
/// onto the sole surviving lot and re-appears at sale as a documented-looking $1.20 basis exceeding the
/// $0 proceeds — a genuine-looking §1001(b) loss of exactly the estimate money (Mutation to kill: the
/// $1.20 estimate loss reappears). WITH the fix the fee's basis is $0, so the sale's basis clamps to
/// the $0 proceeds too — gain $0, never negative.
fn promote_then_self_transfer_fee_then_sell_below_floor() -> Vec<LedgerEvent> {
    let ex = exch();
    let sc = cold();
    let t = tranche_ev(
        1,
        &ex,
        100_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let mut evs = vec![t, p];
    // principal 99,990,000 sat + fee 10,000 sat = the WHOLE tranche (100,000,000 sat): the fee's FIFO
    // draw has no other lot to hit, so it takes the tranche's LAST 10,000 sat (fully draining it).
    evs.extend(self_transfer_with_fee(
        "XO",
        "XI",
        &ex,
        &sc,
        99_990_000,
        10_000,
        datetime!(2025-02-01 00:00 UTC),
        9,
    ));
    evs.push(sell_ev(
        "SELL",
        datetime!(2025-06-01 00:00 UTC),
        &sc,
        99_990_000,
        0,
    ));
    evs
}

#[test]
fn tranche_fee_draw_evaporates_estimate_then_sale_files_zero_loss() {
    let st = project(
        &promote_then_self_transfer_fee_then_sell_below_floor(),
        &prices(),
        &cfg(),
    );
    let leg = only_disposal_leg(&st);
    assert!(
        leg.gain >= Usd::ZERO,
        "the burned fee-sats' estimate component evaporated, not a filed loss (leg.gain = {})",
        leg.gain
    );
}

/// ★ the discriminating snapshot KAT Task 4 correctly deferred (arch r2 I-1 / the T5 fee-evaporation
/// precondition for `universal_snapshot`'s threaded `PromoteSet`, transition.rs:65-67): a pre-2025
/// promoted tranche (1 BTC, floor $12,000, $0.00012/sat) whose fee-sats are FIFO-drawn pre-2025 and are
/// FULLY consumed in the process (100,000,000-sat fee — the whole tranche), landing the fee's carry on
/// a SEPARATE documented lot (0.5 BTC / $8,000, $0.00016/sat — HIGHER per-sat, so HIFO, the default
/// pre-2025 method, drains it FIRST for principal, leaving the tranche wholly untouched until the fee
/// draw) relocated by the SAME self-transfer. `universal_snapshot` re-folds this SAME self-transfer
/// independently, via the SAME shared `fold_event`/`consume_fee` (I-1) — so its residue basis for the
/// surviving documented lot reflects T5's evaporation too, PROVIDED the snapshot's own `FoldCtx` also
/// carries the real (non-empty) `PromoteSet` (Task 4's threading).
///
/// An allocation listing EXACTLY the EVAPORATED (documented-only) residue — $8,000, the lot's own
/// original cost, no floor leaked in — conserves cleanly ONLY when the snapshot ALSO evaporates:
///   - evaporation ON (this task): the fee's $12,000 floor-derived basis withholds entirely (estimate
///     share == the fragment's whole gain_basis, since the tranche was untouched before the fee draw) →
///     the documented lot's re-homed basis stays $8,000 → `alloc_basis` ($8,000) == `snap.basis`
///     ($8,000) → NOT `SafeHarborUnconservable`.
///   - evaporation OFF (the mutation / an empty `PromoteSet` at this FoldCtx site): the WHOLE $12,000
///     floor re-homes onto the documented lot → `snap.basis` is a PHANTOM $20,000 ($8,000 + $12,000) →
///     `alloc_basis` ($8,000) != `snap.basis` ($20,000) → `SafeHarborUnconservable` WRONGLY fires, even
///     though sat conserves exactly either way (`alloc_sat` == `snap.held_sat` == 50,000,000 always).
///
/// So "this allocation conserves cleanly" is possible ONLY when the snapshot's own re-fold evaporated
/// the fee too — exactly what makes the Task-4 `universal_snapshot` threading non-vacuous.
#[test]
fn the_pre2025_conservation_snapshot_sees_the_fee_evaporation_not_a_phantom_basis() {
    let w = exch();
    let sc = cold();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2015 - 01 - 01),
        date!(2015 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let doc = documented_buy(
        "DOC",
        datetime!(2016-01-01 00:00 UTC),
        &w,
        50_000_000,
        8_000,
    );
    let mut evs = vec![t, p, doc];
    // principal (50M sat, HIFO-first) fully consumes DOC, leaving the tranche wholly untouched; the
    // fee (100M sat) then FIFO-draws the ENTIRE tranche — fully consuming it in this SAME event.
    evs.extend(self_transfer_with_fee(
        "XO",
        "XI",
        &w,
        &sc,
        50_000_000,
        100_000_000,
        datetime!(2017-01-01 00:00 UTC),
        9,
    ));
    let a = alloc_ev(
        10,
        true,
        LotMethod::Hifo,
        vec![alloc_lot(&sc, 50_000_000, 8_000, date!(2016 - 01 - 01))],
    );
    evs.push(a);
    let st = project(&evs, &prices(), &cfg());
    assert!(
        !st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SafeHarborUnconservable),
        "the snapshot's OWN re-fold shares T5's fee-evaporation (I-1): the allocation's documented-only \
         $8,000 residue conserves cleanly. Without evaporation the snapshot would see the whole $12,000 \
         floor leaked onto the documented lot ($20,000 phantom), wrongly mismatching alloc_basis: {:?}",
        st.blockers
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 6 — BG-D11 removal-leg-builder documented-only decomposition. A removal (Gift / Donation) drawn
// from a PROMOTED lot files its DOCUMENTED component ONLY: the estimate EVAPORATES (a removal recognizes
// no gain, so there is nothing to clamp the estimate into). The fix is at ONE site (`make_removal_legs`),
// so BOTH §170(e) emitters — the fold's `claimed_deduction` AND the full-return engine's
// `crypto_charitable_gifts` → Schedule A line 12 — plus the Form 8283 `cost_basis` column inherit by
// construction. The estimate must NEVER fund a charitable DEDUCTION or an outbound §1015 carryover.
// PRIVACY: synthetic values only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A raw outbound movement (TransferOut import) to be reclassified as a gift/donation.
fn transfer_out(rf: &str, ts: time::OffsetDateTime, w: &WalletId, sat: i64) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::TransferOut(TransferOut {
            sat,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    )
}

/// A ReclassifyOutflow decision tagging `out_rf`'s TransferOut as a charitable Donation at FMV `fmv`
/// (mirrors kat_tax.rs `donation_over_5k_flags_appraisal_required`).
fn donate_reclass(seq: u64, out_rf: &str, fmv: rust_decimal::Decimal) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-06-15 00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new(out_rf)),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: fmv,
            fee_usd: None,
            donee: None,
        }),
    )
}

/// A ReclassifyOutflow decision tagging `out_rf`'s TransferOut as a (non-charitable) GiftOut at FMV `fmv`.
fn gift_reclass(seq: u64, out_rf: &str, fmv: rust_decimal::Decimal) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-06-15 00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new(out_rf)),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: fmv,
            fee_usd: None,
            donee: None,
        }),
    )
}

/// Promote a whole tranche (window `[ws, we]`, floor `floor`), then move the WHOLE position out at
/// `out_ts` and reclassify it as a Donation at FMV `fmv`.
fn promote_then_donate(
    sat: i64,
    floor: rust_decimal::Decimal,
    fmv: rust_decimal::Decimal,
    ws: time::Date,
    we: time::Date,
    out_ts: time::OffsetDateTime,
) -> Vec<LedgerEvent> {
    let w = exch();
    let t = tranche_ev(1, &w, sat, ws, we);
    let p = promote_ev(2, EventId::decision(1), floor);
    let out = transfer_out("OUT", out_ts, &w, sat);
    let recl = donate_reclass(3, "OUT", fmv);
    vec![t, p, out, recl]
}

/// The full-return SECOND emitter, read off the COMPUTED 1040: `assemble_absolute` runs
/// `crypto_charitable_gifts` → `apply_170b` → Schedule A line 12 noncash charitable. A minimal Single
/// filer with $200k wages (AGI headroom well above any §170(b) ceiling the mutation would need) and an
/// otherwise-empty Schedule A, so line 12 reflects ONLY the ledger's crypto donation. Mirrors the
/// `golden_returns.rs` harness (`ty2024_table`/`ty2024_params` + `assemble_absolute`).
fn full_return_noncash_12(st: &LedgerState, year: i32) -> Usd {
    let ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        w2s: vec![W2 {
            owner: Owner::Taxpayer,
            employer: "E".into(),
            box1_wages: dec!(200_000),
            box3_ss_wages: dec!(200_000),
            box5_medicare_wages: dec!(200_000),
            ..Default::default()
        }],
        schedule_a: Some(ScheduleAInputs::default()),
        ..Default::default()
    };
    let ar = assemble_absolute(&ri, st, &ty2024_params(), &ty2024_table(), year);
    ar.schedule_a
        .expect("a Schedule A is present (ri.schedule_a is Some)")
        .charitable_noncash_12
}

/// The Form 8283 `cost_basis` column for the (first/only) donation leg — sourced from `leg.basis`.
fn form_8283_cost_basis(st: &LedgerState, year: i32) -> Usd {
    let rows = form_8283(st, year, &BTreeMap::new());
    assert!(
        !rows.is_empty(),
        "a donation produces at least one 8283 row"
    );
    rows[0].cost_basis
}

/// ★ BG-D11 — an ST donation of a promoted tranche files a $0/documented §170(e)(1)(A) deduction (NOT the
/// floor) on BOTH emitters: the FOLD's `claimed_deduction` AND the full-return engine's
/// `crypto_charitable_gifts` → Schedule A line 12. The 8283 basis column prints the documented component
/// too. floor $60,000, whole tranche, all estimate (no documented component) ⇒ documented-only = $0.
#[test]
fn promoted_tranche_donated_short_term_deducts_documented_only_on_both_emitters() {
    // acquired 2024-01-10 (window_end), donated 2024-06-01 ⇒ held < 1 yr ⇒ SHORT-TERM.
    let events = promote_then_donate(
        100_000_000,
        dec!(60_000),
        dec!(50_000),
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
        datetime!(2024-06-01 00:00 UTC),
    );
    let st = project(&events, &prices(), &cfg());
    let rem = st
        .removals
        .iter()
        .find(|r| r.kind == RemovalKind::Donation)
        .expect("a Donation removal");
    // Emitter 1 — the fold's claimed_deduction: ST ⇒ min(FMV, basis) = min($50k, $0) = $0.
    assert_eq!(
        rem.claimed_deduction,
        Some(Usd::ZERO),
        "fold claimed_deduction documented-only (estimate never funds the deduction)"
    );
    // ★ Emitter 2 — the full-return engine (the whole point of the Critical): Schedule A line 12 == $0.
    // Patching only emitter 1 would leave this NON-zero (min($50k, $60k floor) = $50k) → this catches it.
    assert_eq!(
        full_return_noncash_12(&st, 2024),
        Usd::ZERO,
        "the full-return engine (crypto_charitable_gifts → Schedule A line 12) also deducts documented-only"
    );
    // Form 8283 basis column prints the documented component.
    assert_eq!(
        form_8283_cost_basis(&st, 2024),
        Usd::ZERO,
        "Form 8283 cost_basis column is documented-only"
    );
}

/// BG-D11 — a promoted tranche GIFTED (not donated) carries its DOCUMENTED component only as the §1015(a)
/// carryover basis to the donee; the estimate never becomes outbound carryover. floor $12,000, whole
/// tranche, no documented component ⇒ every removal leg's basis is $0.
#[test]
fn promoted_tranche_gifted_carries_documented_only_1015_basis() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let out = transfer_out("OUT", datetime!(2024-06-01 00:00 UTC), &w, 100_000_000);
    let recl = gift_reclass(3, "OUT", dec!(50_000));
    let st = project(&[t, p, out, recl], &prices(), &cfg());
    let rem = st
        .removals
        .iter()
        .find(|r| r.kind == RemovalKind::Gift)
        .expect("a Gift removal");
    assert!(
        rem.legs.iter().all(|l| l.basis == Usd::ZERO),
        "§1015 carryover documented-only (BG-D11): the estimate evaporates, never carries: {:?}",
        rem.legs.iter().map(|l| l.basis).collect::<Vec<_>>()
    );
    assert_eq!(
        rem.claimed_deduction, None,
        "a Gift is not a §170 deduction"
    );
}

/// BG-D11 (§6, tax r4 M-3) — a LONG-TERM donation still deducts FMV (the estimate/basis is uninvolved in
/// the LT §170(e) amount), BUT the Form 8283 `cost_basis` COLUMN must print the documented component only
/// (never the floor) — for LT as well as ST. floor $12,000 (all estimate), FMV $50,000 ⇒ deduction $50k,
/// 8283 basis $0.
#[test]
fn long_term_donation_deduction_is_fmv_and_8283_column_is_documented_only() {
    // acquired 2022-01-10 (window_end), donated 2024-06-01 ⇒ held > 1 yr ⇒ LONG-TERM.
    let events = promote_then_donate(
        100_000_000,
        dec!(12_000),
        dec!(50_000),
        date!(2022 - 01 - 01),
        date!(2022 - 01 - 10),
        datetime!(2024-06-01 00:00 UTC),
    );
    let st = project(&events, &prices(), &cfg());
    let rem = st
        .removals
        .iter()
        .find(|r| r.kind == RemovalKind::Donation)
        .expect("a Donation removal");
    assert_eq!(
        rem.claimed_deduction,
        Some(dec!(50_000)),
        "LT deduction = FMV (basis/estimate uninvolved in the LT §170(e) amount)"
    );
    // The 8283 basis column is documented-only even when the deduction itself is FMV.
    assert_eq!(
        form_8283_cost_basis(&st, 2024),
        Usd::ZERO,
        "8283 basis column documented-only, LT too (must not print the floor)"
    );
}

/// The None arm is behavior-preserving (identity): a NON-promoted documented lot donated short-term still
/// deducts its FULL real basis on BOTH emitters — the fix must not zero every removal leg. A $9,000 lot
/// donated ST at FMV $50,000 ⇒ claimed_deduction = min($50k, $9k) = $9,000; Schedule A line 12 = $9,000.
#[test]
fn non_promoted_donation_still_deducts_full_documented_basis() {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2024-01-10 00:00 UTC),
        &w,
        100_000_000,
        9_000,
    );
    let out = transfer_out("OUT", datetime!(2024-06-01 00:00 UTC), &w, 100_000_000);
    let recl = donate_reclass(1, "OUT", dec!(50_000));
    let st = project(&[buy, out, recl], &prices(), &cfg());
    let rem = st
        .removals
        .iter()
        .find(|r| r.kind == RemovalKind::Donation)
        .expect("a Donation removal");
    assert_eq!(
        rem.claimed_deduction,
        Some(dec!(9_000)),
        "non-promoted ST donation deducts min(FMV, real basis) unchanged (None arm is identity)"
    );
    assert_eq!(
        full_return_noncash_12(&st, 2024),
        dec!(9_000),
        "the full-return engine sees the full documented basis for a non-promoted lot"
    );
    assert_eq!(form_8283_cost_basis(&st, 2024), dec!(9_000));
}

/// ★ The evaporate-estimate-but-KEEP-documented corner (the exact `net − documented` decomposition the
/// disposal path uses, applied to a removal): a promoted tranche (floor $12,000) self-transferred with a
/// $30 DOCUMENTED fee carry (drawn FIFO from a separate documented lot, baked onto the relocated tranche
/// by `rehome_onto_lot` → usd_basis $12,030), then donated SHORT-TERM. Only the $12,000 ESTIMATE
/// evaporates; the $30 documented component SURVIVES as the reported basis. Kills BOTH mutations at once:
/// under-clamp (`basis = gain_basis` → $12,030 → deduction $12,030) AND over-clamp (`basis = $0` for a
/// promoted lot → deduction $0 → documented money wrongly evaporated). Mirrors Task 4's
/// `relocated_with_fee_then_promoted_sold_below_floor_...` setup, donated instead of sold.
#[test]
fn promoted_removal_evaporates_estimate_but_keeps_the_documented_fee_carry() {
    let ex = exch();
    let sc = cold();
    let t = tranche_ev(
        1,
        &ex,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    // A separate documented lot the fee FIFO-draws from: 0.3 BTC for $30 (cheap per-sat, so HIFO takes
    // the tranche for principal and leaves this whole lot for the fee → a $30 documented carry).
    let doc = documented_buy("DOC", datetime!(2024-01-15 00:00 UTC), &ex, 30_000_000, 30);
    let mut evs = vec![t, p, doc];
    evs.extend(self_transfer_with_fee(
        "XO",
        "XI",
        &ex,
        &sc,
        100_000_000, // principal: the whole tranche
        30_000_000, // fee: drains the documented lot → $30 documented carry onto the relocated tranche
        datetime!(2024-02-01 00:00 UTC),
        9,
    ));
    // Donate the relocated tranche short-term (acquired 2024-01-10, donated 2024-06-01) at FMV $50,000.
    evs.push(transfer_out(
        "OUT",
        datetime!(2024-06-01 00:00 UTC),
        &sc,
        100_000_000,
    ));
    evs.push(donate_reclass(10, "OUT", dec!(50_000)));
    let st = project(&evs, &prices(), &cfg());
    let rem = st
        .removals
        .iter()
        .find(|r| r.kind == RemovalKind::Donation)
        .expect("a Donation removal");
    // ST deduction = min(FMV $50k, basis) = the DOCUMENTED $30 (estimate $12k evaporated; $30 survived).
    assert_eq!(
        rem.claimed_deduction,
        Some(dec!(30)),
        "estimate evaporates but the documented fee carry survives: deduction = the $30 documented \
         component, NOT $12,030 (under-clamp) and NOT $0 (over-clamp)"
    );
    assert_eq!(
        full_return_noncash_12(&st, 2024),
        dec!(30),
        "the full-return engine also sees the documented-only $30 (not the $12k estimate)"
    );
    assert_eq!(form_8283_cost_basis(&st, 2024), dec!(30));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 7 — BG-D9 engine-adjudicated lifecycle: a promote is revocable; a second promote on one target
// conflicts (neither applies); a RAW void of a tranche held by a live promote is INERT + DecisionConflict
// (never a dangling target); BOTH voids converge in either order with no brick; and a promoted tranche is
// excluded from the bulk-void candidate set (the promote itself stays voidable). PRIVACY: synthetic only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A VoidDecisionEvent decision targeting `target` (mirrors kat_tranche.rs `void_ev`).
fn void_ev(seq: u64, target: EventId) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-03-01 00:00 UTC),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: target,
        }),
    )
}

/// The single conservative-filing tranche lot's basis (the promoted/unpromoted DeclareTranche lot).
fn tranche_lot_basis(st: &LedgerState) -> Usd {
    let lots: Vec<&_> = st
        .lots
        .iter()
        .filter(|l| l.basis_source == BasisSource::EstimatedConservative)
        .collect();
    assert_eq!(
        lots.len(),
        1,
        "exactly one conservative-filing tranche lot in this scenario: {:?}",
        st.lots
    );
    lots[0].usd_basis
}

/// BG-D9: two live promotes naming the SAME tranche → `DecisionConflict`, and NEITHER applies (NOT
/// last-wins) — the tranche basis stays $0. (Mutation: last-wins would rewrite it to $20,000.)
#[test]
fn second_promote_on_one_target_conflicts_neither_applies() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 31),
    );
    let p1 = promote_ev(2, EventId::decision(1), dec!(12_000));
    let p2 = promote_ev(3, EventId::decision(1), dec!(20_000));
    let st = project(&[t, p1, p2], &prices(), &cfg());
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict),
        "two live promotes on one tranche → DecisionConflict: {:?}",
        st.blockers
    );
    assert_eq!(
        tranche_lot_basis(&st),
        Usd::ZERO,
        "neither promote applies under conflict (NOT last-wins): basis stays $0"
    );
}

/// ★ BG-D9: a RAW void of the `DeclareTranche` while a promote is LIVE is resolver-INERT + `DecisionConflict`
/// (never a dangling target). Adjudicated against the FINAL non-voided-promote set (deferred): the
/// tranche-void does not apply, so the promote still rewrites the basis to the floor.
#[test]
fn void_of_tranche_with_live_promote_is_inert_and_conflicts() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let v = void_ev(3, EventId::decision(1)); // raw void of the DeclareTranche while the promote is live
    let st = project(&[t, p, v], &prices(), &cfg());
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict),
        "voiding a tranche with a live promote → DecisionConflict (never a dangling target): {:?}",
        st.blockers
    );
    assert_eq!(
        tranche_lot_basis(&st),
        dec!(12_000),
        "the tranche-void is INERT; the promote still applies (lot basis = floor)"
    );
}

/// ★ BG-D9 acyclicity: voiding the tranche AND the promote converges in EITHER order — promote dead +
/// tranche dropped, with NO spurious DecisionConflict (arch r3 N-1). Varies BOTH the pass-1a vec order and
/// the decision-seq order. (Mutation: classifying the tranche-void inline in pass-1a reds this.)
#[test]
fn both_voids_either_order_converge_no_brick() {
    let w = exch();
    for order in [
        // void the tranche (seq 3) then the promote (seq 4)
        vec![
            void_ev(3, EventId::decision(1)),
            void_ev(4, EventId::decision(2)),
        ],
        // void the promote (seq 3) then the tranche (seq 4) — reversed vec + seq order
        vec![
            void_ev(3, EventId::decision(2)),
            void_ev(4, EventId::decision(1)),
        ],
    ] {
        let mut evs = vec![
            tranche_ev(
                1,
                &w,
                100_000_000,
                date!(2017 - 12 - 01),
                date!(2017 - 12 - 31),
            ),
            promote_ev(2, EventId::decision(1), dec!(12_000)),
        ];
        evs.extend(order);
        let st = project(&evs, &prices(), &cfg());
        assert!(
            st.lots.iter().all(|l| l.wallet != w),
            "both voids converge: promote dead + tranche dropped (no tranche lot): {:?}",
            st.lots
        );
        assert!(
            !st.blockers
                .iter()
                .any(|b| b.kind == BlockerKind::DecisionConflict),
            "no spurious DecisionConflict when BOTH are voided (arch r3 N-1): {:?}",
            st.blockers
        );
    }
}

/// BG-D9: a promoted `DeclareTranche` is EXCLUDED from the bulk-void candidate set (voiding it is inert),
/// but the `PromoteTranche` decision ITSELF is voidable (revoke → revert to $0). Mirrors the #7
/// effective-allocation exclusion.
#[test]
fn a_promoted_tranche_target_is_not_bulk_voidable() {
    let w = exch();
    let events = vec![
        tranche_ev(
            7,
            &w,
            100_000_000,
            date!(2017 - 12 - 01),
            date!(2017 - 12 - 31),
        ),
        promote_ev(8, EventId::decision(7), dec!(12_000)),
    ];
    let voidable = voidable_decisions(&events, &[]);
    assert!(
        !voidable.iter().any(|e| e.id == EventId::decision(7)),
        "the promoted DeclareTranche is excluded while a promote is live"
    );
    assert!(
        voidable.iter().any(|e| e.id == EventId::decision(8)),
        "but the PromoteTranche decision itself IS voidable"
    );
}

/// §6 / BG-D9: voiding ONLY the promote reverts the tranche to $0 with its `EstimatedConservative` tag
/// intact and NO conflict (distinct from the both-voids end state, where the tranche is dropped).
#[test]
fn void_of_promote_alone_reverts_to_zero_tag_intact() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let v = void_ev(3, EventId::decision(2)); // void the PROMOTE, not the tranche
    let st = project(&[t, p, v], &prices(), &cfg());
    let lot = st
        .lots
        .iter()
        .find(|l| l.wallet == w)
        .expect("the tranche lot survives (only the promote was voided)");
    assert_eq!(
        lot.usd_basis,
        Usd::ZERO,
        "voiding the promote reverts the tranche to $0"
    );
    assert_eq!(
        lot.basis_source,
        BasisSource::EstimatedConservative,
        "the intact tranche keeps its EstimatedConservative tag"
    );
    assert!(
        !st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict),
        "a plain promote-void is clean — no conflict: {:?}",
        st.blockers
    );
}

/// BG-D9: `would_conflict` surfaces the second-promote `DecisionConflict` at RECORD time (before the
/// decision is appended) — the T10 CLI record-time guard depends on this free surfacing.
#[test]
fn would_conflict_surfaces_a_second_promote_at_record_time() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 31),
    );
    let p1 = promote_ev(2, EventId::decision(1), dec!(12_000));
    // The incoming (not-yet-recorded) second promote on the already-promoted tranche.
    let incoming = promote_ev(99, EventId::decision(1), dec!(20_000)).payload;
    let hit = would_conflict(
        &[t, p1],
        &prices(),
        &cfg(),
        &incoming,
        datetime!(2026-03-01 00:00 UTC),
    );
    assert!(
        hit.is_some(),
        "would_conflict flags the second promote at record time: {hit:?}"
    );
}
