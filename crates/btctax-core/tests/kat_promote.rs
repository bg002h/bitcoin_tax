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

use btctax_core::conservative::{
    basis_methodology, flagged_years, overpayment_nudge_lines, promote_prior_year_advisory,
    self_custody_nudge, tranche_dip_advisory, Coverage, Direction,
};
use btctax_core::conservative_promote::{
    clamped_leg_basis, clamped_promote_year_saving, consent_terms, filed_basis_for,
    promote_drift_advisory, PromoteEntry, PromoteRefusal,
};
use btctax_core::event::*;
use btctax_core::forms::form_8283;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{
    evaluate_disposal, project, would_conflict, CandidateDisposal, LotMethod, ProjectionConfig,
};
use btctax_core::state::{BlockerKind, DisposalLeg, LedgerState, RemovalKind};
use btctax_core::tax::form8275::disclosure_8275;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::return_inputs::{Owner, ReturnInputs, ScheduleAInputs, W2};
use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
use btctax_core::tax::{compute_tax_year, Carryforward, FilingStatus, TaxOutcome, TaxProfile};
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

/// ★ Phase-1a T7 (BG-D9): a `PromoteTranche` whose `target` is an ABSENT decision (never recorded at
/// all, not merely voided) is a `DecisionConflict` too — the `live_promotes` None-arm (resolve.rs
/// `by_id.get(&p.target)` misses) fires the SAME remedy as a wrong-type/voided target, so a promote can
/// never dangle silently off a bad ref. No `DeclareTranche` is even declared here — the promote is the
/// only decision in the whole projection, so this isolates the None arm from the ≥2-live-promotes arm
/// (a different `promote_count`-keyed branch just above it in `live_promotes`).
#[test]
fn promote_targeting_an_absent_decision_conflicts() {
    let p = promote_ev(1, EventId::decision(999), dec!(12_000));
    let st = project(&[p], &prices(), &cfg());
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict),
        "a PromoteTranche targeting an absent decision id is a DecisionConflict (never a dangling \
         promote): {:?}",
        st.blockers
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 8 — BG-D9 prior-year fold-diff advisory (disposal ∪ removal legs) + carryover-cascade naming +
// the VOID-direction wiring. The advisory FOLDS the ledger twice (promote EVENT present vs excluded),
// diffs the per-year disposal ∪ removal LEG SETS (NOT tax_total — None for 2018–2023; NOT Σ-gain —
// blind to an equal-basis/different-date reorder), and names the amendment implication PER YEAR by the
// SIGN of that year's Δ. PRIVACY: synthetic values only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// An empty tax-table set (`table_for` → None for every year) so a table-less audience year (2018–2023,
/// and here every year) reads as NOT computable — the fold-diff STILL fires (it never keys on tax_total).
fn no_tables() -> BTreeMap<i32, btctax_core::TaxTable> {
    BTreeMap::new()
}

/// A `current` cutoff far beyond any fixture year below — these tests are NOT about the T10 `< current`
/// filter (that is pinned separately, `the_current_cutoff_excludes_the_year_still_being_authored`), so a
/// sentinel this large is a no-op: every fixture year stays `< FAR_FUTURE`.
const FAR_FUTURE: i32 = 9999;

/// A pre-2025 disposal-reorder scenario: a documented 0.6-BTC lot ($5,000/BTC) co-held with a promoted
/// 0.4-BTC tranche (floor $12,000 whole ⇒ $30,000/BTC — HIGHER per-sat, so HIFO draws it FIRST once
/// promoted; unpromoted at $0 it sorts LAST). A 2018 sell of EXACTLY 0.4 BTC therefore drains the tranche
/// WITH the promote (gain $8,000) and the documented lot WITHOUT it (gain $18,000) — a leg-set diff. The
/// promote is `EventId::decision(2)`. Sold ABOVE the floor so both folds file a real positive gain.
fn mixed_vintage_hifo_2018_disposal() -> Vec<LedgerEvent> {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2017-01-01 00:00 UTC),
        &w,
        60_000_000,
        3_000,
    );
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 03 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let sell = sell_ev(
        "SELL",
        datetime!(2018-09-01 00:00 UTC),
        &w,
        40_000_000,
        20_000,
    );
    vec![buy, t, p, sell]
}

/// A below-floor variant of the disposal reorder: proceeds $8,000 < the $12,000 floor, so WITH the promote
/// the tranche leg's gain clamps to $0 (D-4), while WITHOUT it the documented lot files a $6,000 gain — a
/// net-capital-gain change that must name the §1212(b) carryover cascade.
fn loss_stealing_reorder() -> Vec<LedgerEvent> {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2017-01-01 00:00 UTC),
        &w,
        60_000_000,
        3_000,
    );
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 03 - 31),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let sell = sell_ev(
        "SELL",
        datetime!(2018-09-01 00:00 UTC),
        &w,
        40_000_000,
        8_000,
    );
    vec![buy, t, p, sell]
}

/// A SHORT-TERM donation-only reorder: a documented 0.4-BTC lot ($40,000 basis) co-held with a promoted
/// 0.4-BTC tranche (floor $50,000 ⇒ higher per-sat, HIFO draws it FIRST). A 2024 donation of EXACTLY 0.4
/// BTC therefore draws the tranche WITH the promote (D-11 documented-only $0 ⇒ ST deduction $0) and the
/// documented lot WITHOUT it (ST deduction min($60k,$40k)=$40k) — a removal-leg diff with ZERO disposal
/// change (the disposal∪removal diff catches it; a disposals-only diff would miss it entirely).
fn prior_donation_only_reorder() -> Vec<LedgerEvent> {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2024-01-05 00:00 UTC),
        &w,
        40_000_000,
        40_000,
    );
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(50_000));
    let out = transfer_out("OUT", datetime!(2024-06-01 00:00 UTC), &w, 40_000_000);
    let recl = donate_reclass(3, "OUT", dec!(60_000));
    vec![buy, t, p, out, recl]
}

/// A GIFT-only reorder (same shape as the donation reorder, GiftOut instead of Donate): WITH the promote
/// the gift draws the tranche (§1015 carryover = documented-only $0), WITHOUT it the documented lot
/// ($40,000 carryover). A gift changes NO line of the donor's 1040 → the advisory must NOT tell the donor
/// to amend.
fn prior_gift_only_reorder() -> Vec<LedgerEvent> {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2024-01-05 00:00 UTC),
        &w,
        40_000_000,
        40_000,
    );
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(50_000));
    let out = transfer_out("OUT", datetime!(2024-06-01 00:00 UTC), &w, 40_000_000);
    let recl = gift_reclass(3, "OUT", dec!(50_000));
    vec![buy, t, p, out, recl]
}

/// An equal-basis / different-DATE reorder that changes the 8283 rows but NOT the deduction: two LONG-TERM
/// lots (documented $12,000 acquired 2022-01-05; tranche promoted floor $30,000, window_end 2022-01-10)
/// donated 2024-06-01. LT ⇒ deduction = FMV either way (Δded = $0), but the drawn lot's acquisition date /
/// 8283 basis column differ — the leg SET changes with ZERO gain/deduction Δ (the BG-D9 corner Σ-gain
/// misses).
fn equal_basis_date_swap_reorder() -> Vec<LedgerEvent> {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2022-01-05 00:00 UTC),
        &w,
        40_000_000,
        12_000,
    );
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2022 - 01 - 01),
        date!(2022 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(30_000));
    let out = transfer_out("OUT", datetime!(2024-06-01 00:00 UTC), &w, 40_000_000);
    let recl = donate_reclass(3, "OUT", dec!(50_000));
    vec![buy, t, p, out, recl]
}

/// A short-term donation reorder whose DEDUCTION changes (Δded ≠ 0) — so the §170(d) charitable-carryover
/// cascade arm fires. Same shape as `prior_donation_only_reorder`.
fn prior_donation_reorder_over_ceiling() -> Vec<LedgerEvent> {
    prior_donation_only_reorder()
}

/// A void-of-promote over a filed floor-year: the SAME disposal-reorder events, adjudicated in the VOID
/// direction (voiding the live promote reverts the tranche floor → $0, raising the filed year's gain).
fn void_promote_over_filed_year() -> Vec<LedgerEvent> {
    mixed_vintage_hifo_2018_disposal()
}

/// ★ BG-D9 (tax r2 I-3 / M-1): a table-less/profile-less 2018 disposal reorder STILL fires the advisory
/// (leg-set diff, not tax_total), and a basis-INCREASE (gain-decrease) year names §6511 for the refund —
/// NOT "additional tax". A `tax_total`-keyed predicate would read `None == None` and MISS the rewrite.
#[test]
fn undisposed_promote_that_hifo_reorders_a_prior_year_fires_the_advisory() {
    let events = mixed_vintage_hifo_2018_disposal();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        FAR_FUTURE,
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("2018") && l.contains("1040-X")),
        "the 2018 rewrite is named with its Form 1040-X implication: {lines:?}"
    );
    let joined = lines.join(" ");
    assert!(
        joined.contains("§6511") && !joined.contains("additional tax"),
        "a basis-increase (refund) year names §6511, NOT 'additional tax' (amend direction by Δ sign): {joined}"
    );
}

/// BG-D9 (tax r3 I-2): a removal-leg (donation) reorder with ZERO disposal change is caught by the
/// disposal∪removal diff and names the deduction change.
#[test]
fn promote_reordering_a_prior_donation_only_year_fires_and_names_the_deduction() {
    let events = prior_donation_only_reorder();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        FAR_FUTURE,
    );
    assert!(
        lines.iter().any(|l| l.contains("charitable deduction")),
        "the donation-only reorder names the charitable-deduction change: {lines:?}"
    );
}

/// BG-D9 (tax r4 I-1): a reorder that changes a filed year's net capital gain/loss names the §1212(b)
/// carryforward cascade into later filed years.
#[test]
fn a_loss_stealing_reorder_names_the_1212b_carryover_cascade() {
    let events = loss_stealing_reorder();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        FAR_FUTURE,
    );
    assert!(
        lines.iter().any(
            |l| l.contains("carryover-linked lines of later filed years") && l.contains("§1212(b)")
        ),
        "the net-cap-gain change names the §1212(b) carryover cascade: {lines:?}"
    );
}

/// BG-D9 (tax r4 M-1): a gift changes NO line of the donor's 1040 → the §1015 carryover-Δ is named
/// (donee-basis documentation) with NO 1040-X, and never a bare "$0 / $0".
#[test]
fn a_gift_only_reorder_quotes_the_1015_carryover_and_asserts_no_1040x() {
    let events = prior_gift_only_reorder();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        FAR_FUTURE,
    );
    let joined = lines.join(" ");
    assert!(
        joined.contains("donee-basis") && !joined.contains("$0 / $0"),
        "the gift reorder names the donee-basis (§1015) change, never a bare $0 / $0: {joined}"
    );
    assert!(
        !joined.contains("1040-X"),
        "a gift reorder must NOT tell the donor to amend (no 1040-X): {joined}"
    );
}

/// BG-D9: an equal-basis / different-date reorder (Δgain = Δded = $0) that changes the 8949/8283 dates
/// names the CHANGED CONTENT, never a bare "$0".
#[test]
fn a_both_deltas_zero_flagged_year_names_the_changed_content_not_a_bare_zero() {
    let events = equal_basis_date_swap_reorder();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        FAR_FUTURE,
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("acquisition date") || l.contains("donee")),
        "a zero-Δ reorder names the changed 8949/8283 content: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.trim().ends_with("$0")),
        "no line reports a bare $0: {lines:?}"
    );
}

/// BG-D9: a donation reorder whose deduction changed names the §170(d) charitable-carryover cascade.
#[test]
fn a_donation_reorder_names_the_170d_charitable_carryover_direction() {
    let events = prior_donation_reorder_over_ceiling();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        FAR_FUTURE,
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("§170(d)") && l.contains("charitable carryover")),
        "the donation reorder names the §170(d) charitable-carryover cascade: {lines:?}"
    );
}

/// §6: the SAME advisory in the VOID direction — voiding a live promote over a filed floor-year reverts
/// the floor → $0, RAISING the filed year's gain → amend-to-PAY (1040-X, additional tax).
#[test]
fn the_void_direction_fires_amend_to_pay() {
    let events = void_promote_over_filed_year();
    let tables = no_tables();
    let lines = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Void,
        None,
        &tables,
        FAR_FUTURE,
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("1040-X") && l.to_lowercase().contains("additional tax")),
        "voiding a promote over a filed floor-year is amend-to-pay (1040-X, additional tax): {lines:?}"
    );
}

/// ★ Task 10 handoff (progress.md): `current` filters candidate years to `< current` — the year still
/// being authored (>= current) must NOT be told it needs a Form 1040-X. `mixed_vintage_hifo_2018_disposal`
/// flags exactly year 2018 (its 2018-09-01 sell). `current = 2018` excludes it (2018 is NOT < 2018);
/// `current = 2019` includes it (2018 < 2019) — the same fixture, only the cutoff differs.
#[test]
fn the_current_cutoff_excludes_the_year_still_being_authored() {
    let events = mixed_vintage_hifo_2018_disposal();
    let tables = no_tables();
    let excluded = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        2018,
    );
    assert!(
        excluded.is_empty(),
        "current=2018 must exclude year 2018 (still being authored, not yet filed): {excluded:?}"
    );
    let included = promote_prior_year_advisory(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(2),
        Direction::Promote,
        None,
        &tables,
        2019,
    );
    assert!(
        included
            .iter()
            .any(|l| l.contains("2018") && l.contains("1040-X")),
        "current=2019 must still include the (now presumed-filed) year 2018: {included:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Defensive Filing Wizard sub-2, Task 3 (DFW-D11) — `flagged_years`: the STRUCTURED (`BTreeSet<i32>`)
// twin of `promote_prior_year_advisory`'s year-set, reusing the SAME fixtures above at the unit-test
// altitude (the CLI-level two-arm characterization + the removal-reorder / two-live-promote KATs live in
// `btctax-cli/tests/promote_cli.rs` per the task brief).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// `flagged_years` pins the SAME disposal-reorder year (2018) `promote_prior_year_advisory` names above,
/// filtered by the SAME `< current` cutoff (mirrors `the_current_cutoff_excludes_the_year_still_being_
/// authored`, at the structured-fn altitude).
#[test]
fn flagged_years_pins_the_prior_disposal_reorder_filtered_by_current() {
    let events = mixed_vintage_hifo_2018_disposal();
    let state = project(&events, &prices(), &cfg());
    let tables = no_tables();

    let excluded = flagged_years(&events, &state, &prices(), &tables, &cfg(), 2018);
    assert!(
        excluded.is_empty(),
        "current=2018 must exclude year 2018 (still being authored): {excluded:?}"
    );

    let included = flagged_years(&events, &state, &prices(), &tables, &cfg(), 2019);
    assert!(
        included.contains(&2018),
        "current=2019 must still include the (now presumed-filed) year 2018: {included:?}"
    );
}

/// `flagged_years` catches a REMOVAL-only (donation) reorder with ZERO disposal change — the
/// disposal-legs-only `promoted_filing_years` (chokepoint/mod.rs, the 8275-gate enumeration) would MISS
/// this year entirely; `flagged_years` must not (DFW-D11's whole reason to exist).
#[test]
fn flagged_years_pins_a_prior_donation_only_reorder() {
    let events = prior_donation_only_reorder();
    let state = project(&events, &prices(), &cfg());
    let tables = no_tables();

    let years = flagged_years(&events, &state, &prices(), &tables, &cfg(), FAR_FUTURE);
    assert!(
        years.contains(&2024),
        "a donation-only reorder with no disposal change must still be flagged: {years:?}"
    );
}

/// `flagged_years` ALSO catches a GIFT-only reorder (the other removal kind) — a gift changes no 1040
/// line but still rewrites the §1015 donee-basis carryover, so it must be in the export set too.
#[test]
fn flagged_years_pins_a_prior_gift_only_reorder() {
    let events = prior_gift_only_reorder();
    let state = project(&events, &prices(), &cfg());
    let tables = no_tables();

    let years = flagged_years(&events, &state, &prices(), &tables, &cfg(), FAR_FUTURE);
    assert!(
        years.contains(&2024),
        "a gift-only reorder must still be flagged: {years:?}"
    );
}

/// ★ DFW M-new-1 (both P-A gate-review lenses, CONFIRMED at source): `promote_changed_years` — and thus
/// `flagged_years` — now forces `pseudo_reconcile = false` on its OWN config copy, mirroring
/// `would_conflict` (`project/mod.rs:119`), so the result is stable regardless of the CALLER's `cfg`.
///
/// Fixture: an unresolved `ImportConflict` whose ORIGINAL import already carries a real ($100) basis and
/// whose conflicting re-import proposes a HIGHER ($700) basis (mirrors `pseudo_reconcile.rs`'s shipped
/// `import_conflict_cleared_via_accept_first`) — pseudo-OFF the $100 original stands; pseudo-ON the
/// accept-first default adopts $700. A co-held, LIVE-promoted tranche (floor $400 — strictly between the
/// two) means: pseudo-OFF, the tranche's $400/sat OUTRANKS the $100/sat alternate under HIFO, so the
/// PROMOTE's own dollar leg is what changes between with/without-promote (flags 2026); pseudo-ON, the
/// $700/sat accept-first lot OUTRANKS the tranche EVEN WHEN PROMOTED ($400), so the identical lot is
/// drawn with/without-promote (2026 is NOT flagged) — UNLESS pseudo is forced off internally, in which
/// case both calls agree (mutation: remove the forced pseudo-off line inside `promote_changed_years` →
/// this KAT reds, since the pseudo-true call would then disagree with the pseudo-false call).
#[test]
fn flagged_years_forces_pseudo_off_regardless_of_caller_cfg() {
    let w = exch();
    let original = imp(
        "ORIG",
        datetime!(2026-01-01 00:00 UTC),
        &w,
        EventPayload::Acquire(Acquire {
            sat: 1_000_000,
            usd_cost: dec!(100),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let new_payload = EventPayload::Acquire(Acquire {
        sat: 1_000_000,
        usd_cost: dec!(700),
        fee_usd: dec!(0),
        basis_source: BasisSource::ExchangeProvided,
    });
    let fp = Fingerprint::of_bytes(b"m-new-1-fixture");
    let conflict = LedgerEvent {
        id: EventId::conflict(Source::Coinbase, SourceRef::new("ORIG"), &fp),
        utc_timestamp: datetime!(2026-01-02 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: EventPayload::ImportConflict(ImportConflict {
            target: EventId::import(Source::Coinbase, SourceRef::new("ORIG")),
            new_payload: Box::new(new_payload),
            new_fingerprint: fp,
        }),
    };
    let t = tranche_ev(
        1,
        &w,
        1_000_000,
        date!(2026 - 01 - 03),
        date!(2026 - 01 - 05),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(400));
    let sell = sell_ev(
        "SELL",
        datetime!(2026-06-01 00:00 UTC),
        &w,
        1_000_000,
        2_000,
    );
    let events = vec![original, conflict, t, p, sell];

    let mut cfg_true = cfg();
    cfg_true.pseudo_reconcile = true;
    let mut cfg_false = cfg();
    cfg_false.pseudo_reconcile = false;

    // `state` (for the `live_promote_ids` lookup) — promote liveness is pseudo-independent either way.
    let state = project(&events, &prices(), &cfg_false);
    let tables = no_tables();

    let years_true = flagged_years(&events, &state, &prices(), &tables, &cfg_true, FAR_FUTURE);
    let years_false = flagged_years(&events, &state, &prices(), &tables, &cfg_false, FAR_FUTURE);
    assert_eq!(
        years_true, years_false,
        "flagged_years must force pseudo off internally regardless of the caller's cfg: \
         pseudo-true={years_true:?} pseudo-false={years_false:?}"
    );
    assert!(
        years_false.contains(&2026),
        "sanity: the tranche's own promote/basis change genuinely flags 2026 in the pseudo-off \
         computation: {years_false:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 9 — BG-D6 `consent_terms`: the two-sided informed-consent figures the filer sees and the
// promote records. Fold pair mirrors T8 (`promote_prior_year_advisory`) EXCEPT the promote is
// SYNTHESIZED (consent runs BEFORE the promote is recorded), so the fixtures below carry NO
// PromoteTranche event — `consent_terms` threads its own so the BG-D4 clamp binds. PRIVACY: synthetic.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The `CURRENT_YEAR` for the sell-this-year KAT — the year a same-year sale+promote lands in. Core is
/// clock-free, so "current" is just the sale's year; the point is `consent_terms` must NOT drop it.
const CURRENT_YEAR: i32 = 2024;

/// A profile that prices a 2024 short-term gain at a positive marginal rate (Single, $200k base income —
/// the 32% bracket), so the clamped saving is a real, non-zero figure.
fn consent_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(200_000),
        magi_excluding_crypto: dec!(200_000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    }
}

/// The TY2024 table indexed for `compute_tax_year` (2024 ships; other years are uncomputable).
fn tables_2024() -> BTreeMap<i32, btctax_core::TaxTable> {
    let mut m = BTreeMap::new();
    m.insert(2024, ty2024_table());
    m
}

/// The crypto-attributable federal tax for `year` under `st` (the same figure `consent_terms` folds).
fn crypto_tax(events: &[LedgerEvent], st: &LedgerState, year: i32, profile: &TaxProfile) -> Usd {
    match compute_tax_year(events, st, year, Some(profile), &tables_2024()) {
        TaxOutcome::Computed(r) => r.total_federal_tax_attributable,
        TaxOutcome::NotComputable(b) => panic!("expected computable {year}: {b:?}"),
    }
}

/// The `ComputedTax.delta_usd` recorded for `year`, if any.
fn computed_saving(terms: &[ConsentTerm], year: i32) -> Option<Usd> {
    terms.iter().find_map(|t| match t {
        ConsentTerm::ComputedTax {
            year: y, delta_usd, ..
        } if *y == year => Some(*delta_usd),
        _ => None,
    })
}

// ── promote-FREE fixtures (consent_terms synthesizes the promote) ──────────────────────────────────

/// A whole 1-BTC tranche (window 2024-01) sold WHOLE at $8,000 — BELOW the $12,000 window-low floor.
fn consent_sell_below_low() -> Vec<LedgerEvent> {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2024-06-01 00:00 UTC),
        &w,
        100_000_000,
        8_000,
    );
    vec![t, sell]
}

/// A whole 1-BTC tranche (window 2024-01) sold WHOLE at $20,000 — ABOVE the $12,000 floor, in the
/// current year (sold 2024-06, promoting 2024) — a normal positive saving.
fn consent_sell_this_year() -> Vec<LedgerEvent> {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2024-06-01 00:00 UTC),
        &w,
        100_000_000,
        20_000,
    );
    vec![t, sell]
}

/// A whole 1-BTC tranche (2017 window), never disposed. No disposal ⇒ NO year is flagged.
fn consent_undisposed() -> Vec<LedgerEvent> {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 03),
    );
    vec![t]
}

/// A SHORT-TERM donation-only reorder WITHOUT the promote event: documented 0.4-BTC lot ($40k basis)
/// co-held with a 0.4-BTC tranche (consent floor $50k ⇒ higher per-sat, HIFO draws it FIRST). A 2024
/// donation of 0.4 BTC draws the tranche WITH the (synthetic) promote (documented-only $0 deduction) and
/// the documented lot WITHOUT it ($40k) — a removal-leg diff with ZERO disposal change.
fn consent_donation_only() -> Vec<LedgerEvent> {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2024-01-05 00:00 UTC),
        &w,
        40_000_000,
        40_000,
    );
    let t = tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let out = transfer_out("OUT", datetime!(2024-06-01 00:00 UTC), &w, 40_000_000);
    let recl = donate_reclass(3, "OUT", dec!(60_000));
    vec![buy, t, out, recl]
}

/// A carryover-cascade fixture (all POST-2025 so draws are per-wallet, not the pre-2025 global snapshot):
/// wallet A holds a documented 0.4-BTC lot + a 0.4-BTC tranche; a 2025 sale of 0.4 BTC HIFO-reorders
/// (promoted tranche floor $30k/BTC drawn FIRST; unpromoted $0 drawn last) → 2025's net capital gain
/// changes (§1212(b) source). Wallet B holds an unrelated documented lot sold in 2026 — the promote never
/// touches it (unflagged), but its carryover-linked lines still shift → CascadeNamed{2026}.
fn consent_cascade() -> Vec<LedgerEvent> {
    let a = exch();
    let b = cold();
    let buy_a = documented_buy(
        "BUYA",
        datetime!(2025-01-05 00:00 UTC),
        &a,
        40_000_000,
        4_000,
    );
    let t = tranche_ev(
        1,
        &a,
        40_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 10),
    );
    let sell_a = sell_ev(
        "SELLA",
        datetime!(2025-09-01 00:00 UTC),
        &a,
        40_000_000,
        20_000,
    );
    let buy_b = documented_buy(
        "BUYB",
        datetime!(2025-02-01 00:00 UTC),
        &b,
        50_000_000,
        10_000,
    );
    let sell_b = sell_ev(
        "SELLB",
        datetime!(2026-03-01 00:00 UTC),
        &b,
        50_000_000,
        25_000,
    );
    vec![buy_a, t, sell_a, buy_b, sell_b]
}

/// Prices with a current close at the tranche declaration date (2026-01-01) — the deterministic clock-free
/// "as-of" (the ledger's latest recorded instant) — so the undisposed hypothetical resolves to `Some`.
fn prices_with_current_close() -> StaticPrices {
    StaticPrices(
        [(date!(2026 - 01 - 01), dec!(40_000))]
            .into_iter()
            .collect(),
    )
}

#[test]
fn below_window_low_sale_quotes_the_clamped_saving_not_an_unclaimable_loss() {
    // window-min $12k, sold at $8k. True promoted saving = tax on the $8k gain (clamped to $0 gain),
    // NEVER a $4k loss the promote can't file (which the un-clamped `overpayment_delta_one` swap would).
    let events = consent_sell_below_low();
    let profile = consent_profile();
    // The un-promoted baseline files exactly the $8k gain; its crypto tax IS `tax_on_gain(8_000)`.
    let baseline = project(&events, &prices(), &cfg());
    let tax_on_8k = crypto_tax(&events, &baseline, 2024, &profile);
    let terms = consent_terms(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(1),
        dec!(12_000),
        Some(&profile),
        &tables_2024(),
    );
    let saving = computed_saving(&terms, 2024).expect("a 2024 ComputedTax term");
    assert!(
        saving > Usd::ZERO,
        "the clamped saving is a real positive figure"
    );
    assert_eq!(
        saving, tax_on_8k,
        "the recorded saving equals tax on the CLAMPED $8k gain (with-promote gain clamps to $0), \
         never the inflated saving an un-clamped $4k loss would produce"
    );
}

#[test]
fn fully_undisposed_promote_records_an_unrealized_term_not_empty() {
    // A fully-undisposed promote flags NO year → the Σ is empty; BG-D6 mandates an UNREALIZED line
    // (never a bare nothing). With a current close, the hypothetical reduction is the clamped floor.
    let events = consent_undisposed();
    let terms = consent_terms(
        &events,
        &prices_with_current_close(),
        &cfg(),
        &EventId::decision(1),
        dec!(12_000),
        None, // no profile
        &tables_2024(),
    );
    assert!(
        terms
            .iter()
            .any(|t| matches!(t, ConsentTerm::Unrealized { .. })),
        "unrealized hypothetical line present: {terms:?}"
    );
    assert!(
        terms.iter().all(
            |t| !matches!(t, ConsentTerm::ComputedTax { delta_usd, .. } if *delta_usd == Usd::ZERO)
        ),
        "never a bare $0 ComputedTax: {terms:?}"
    );
    // today $40k/BTC ≥ floor $12k ⇒ the clamped gain reduction is the whole $12k floor.
    assert!(
        terms.iter().any(|t| matches!(t,
            ConsentTerm::Unrealized { sat, hypothetical_reduction: Some(r), .. }
                if *sat == 100_000_000 && *r == dec!(12_000))),
        "the Some(reduction) is min(today-proceeds, floor) = the whole floor here: {terms:?}"
    );
}

#[test]
fn no_current_price_falls_back_to_the_floor_as_max_reduction() {
    // Bundled prices end at release; "today" often has no close ⇒ fallback (None), never a dropped line.
    let events = consent_undisposed();
    let terms = consent_terms(
        &events,
        &prices(), // empty ⇒ no close at the as-of date
        &cfg(),
        &EventId::decision(1),
        dec!(12_000),
        None,
        &tables_2024(),
    );
    assert!(
        terms.iter().any(|t| matches!(
            t,
            ConsentTerm::Unrealized {
                hypothetical_reduction: None,
                ..
            }
        )),
        "no-price ⇒ the floor itself ($filed_basis) named as the max reduction, not $0: {terms:?}"
    );
}

#[test]
fn a_computing_removal_flagged_year_carries_the_deduction_delta() {
    // 2024 (table ships) + profile + a donation reorder → ComputedTax with Some(deduction_delta), NOT
    // labeled uncomputable and NOT dropping the Schedule-A change (engine B can't price it). Donation-ONLY
    // ⇒ the tax-Δ is exactly $0 AND the deduction-Δ is Some(≠0): pin BOTH.
    let events = consent_donation_only();
    let profile = consent_profile();
    let terms = consent_terms(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(1),
        dec!(50_000),
        Some(&profile),
        &tables_2024(),
    );
    assert!(
        terms.iter().any(|t| matches!(t,
            ConsentTerm::ComputedTax { year, delta_usd, deduction_delta_usd: Some(d) }
                if *year == 2024 && *delta_usd == Usd::ZERO && *d != Usd::ZERO)),
        "a donation-only computing year records {{delta:0, deduction:Some(≠0)}}, never uncomputable / bare $0: {terms:?}"
    );
}

#[test]
fn sell_this_year_then_promote_includes_the_current_year_term() {
    // A sell-earlier-this-year-then-promote must have its CURRENT-year realized delta quoted, not dropped
    // (no `< current` filter on the year set).
    let events = consent_sell_this_year();
    let profile = consent_profile();
    let terms = consent_terms(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(1),
        dec!(12_000),
        Some(&profile),
        &tables_2024(),
    );
    assert!(
        terms.iter().any(|t| matches!(t,
            ConsentTerm::ComputedTax { year, delta_usd, .. }
                if *year == CURRENT_YEAR && *delta_usd > Usd::ZERO)),
        "the current-year realized saving is quoted, not dropped: {terms:?}"
    );
}

#[test]
fn a_carryover_source_names_the_cascade_into_an_unflagged_later_year() {
    // §1212(b)/§170(d): a flagged year that shifts a carryover reshapes LATER years' carryover-in, which
    // the per-year engine cannot chain (static profile carryforward-in). Name the unflagged later
    // activity year (2026), unquantified.
    let events = consent_cascade();
    let profile = consent_profile();
    let mut tables = BTreeMap::new();
    tables.insert(2025, ty2024_table()); // 2025 computes; 2026 has no table (the cascade target).
    let terms = consent_terms(
        &events,
        &prices(),
        &cfg(),
        &EventId::decision(1),
        dec!(12_000),
        Some(&profile),
        &tables,
    );
    assert!(
        computed_saving(&terms, 2025).is_some(),
        "the 2025 reorder is a computing ComputedTax term (carryover source): {terms:?}"
    );
    assert!(
        terms
            .iter()
            .any(|t| matches!(t, ConsentTerm::CascadeNamed { year } if *year == 2026)),
        "the §1212(b) carryover cascade into the unflagged 2026 year is named: {terms:?}"
    );
    assert!(
        !terms
            .iter()
            .any(|t| matches!(t, ConsentTerm::CascadeNamed { year } if *year == 2025)),
        "the carryover-SOURCE year itself is not cascade-named (it has its own term): {terms:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 11 — §3 tag-side census: promote-aware advisories + the BG-D3 verify-drift advisory. A promoted
// tranche is now `>$0` but keeps the `EstimatedConservative` tag, so the `$0`-assuming copy that keys on
// that tag is FALSE for a promoted leg. These KATs pin: (1) `basis_methodology` no longer claims a `>$0`
// is "never the estimate" for a promoted leg; (2) the dip/self-custody copy is basis-as-filed for a
// promoted tranche; (3) the overpayment nudge's `promote-tranche` funnel quotes the CLAMPED delta (never
// the un-clamped over-quote); (4) the direction-aware `promote_drift_advisory`, and that the FOLD still
// uses the STORED number. PRIVACY: synthetic values only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A projected state whose single disposal leg is a PROMOTED tranche (>$0 basis = the estimate re-homed):
/// 1-BTC tranche promoted to a $12,000 floor, sold WHOLE at $20,000 (above the floor ⇒ leg files $12,000
/// basis, $8,000 gain). `state.promoted_origins` therefore contains `decision(1)`.
fn promoted_state() -> LedgerState {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    let sell = sell_ev(
        "SELL",
        datetime!(2024-06-01 00:00 UTC),
        &w,
        100_000_000,
        20_000,
    );
    project(&[t, p, sell], &prices(), &cfg())
}

/// ★ tag-side census (§6662 honesty): the `>$0` promoted basis IS the estimate re-homed, so the
/// disclosure must NOT claim a `>$0` is "never the estimate", and the promoted leg gets the estimate
/// (Cohan) disclosure. Mutation to kill: leaving the "never the estimate" sentence.
#[test]
fn basis_methodology_no_longer_claims_never_the_estimate_for_a_promoted_leg() {
    let text =
        basis_methodology(&promoted_state(), 2024).expect("a filed tranche has a disclosure");
    assert!(
        !text.contains("never the estimate"),
        "a promoted `>$0` basis IS the estimate re-homed — the false 'never the estimate' sentence \
         must be gone: {text}"
    );
    assert!(
        text.contains("estimated at the minimum daily closing price"),
        "the promoted leg gets the estimate (Cohan) disclosure: {text}"
    );
    // provenance-neutral regression (tax min-8c): still never a purchase.
    let low = text.to_lowercase();
    assert!(
        !low.contains("purchase") && !low.contains("bought"),
        "provenance-neutral: {text}"
    );
}

/// §3 item 2: a promoted tranche is no longer "$0-basis" — the dip advisory prints its basis AS FILED
/// (never `$0`), and the self-custody nudge for a REMAINING promoted exchange tranche no longer asserts a
/// `$0`-basis unit.
#[test]
fn dip_and_self_custody_copy_distinguishes_a_promoted_tranche() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
    );
    let p = promote_ev(2, EventId::decision(1), dec!(12_000));
    // Sell HALF (0.5 BTC at $10,000; floor for 0.5 BTC = $6,000) so a promoted exchange tranche lot
    // REMAINS to trigger the self-custody nudge, and a promoted disposal leg exists for the dip.
    let sell = sell_ev(
        "SELL",
        datetime!(2024-06-01 00:00 UTC),
        &w,
        50_000_000,
        10_000,
    );
    let st = project(&[t, p, sell], &prices(), &cfg());
    let disposal = st.disposals.first().expect("a disposal");
    let dip = tranche_dip_advisory(disposal);
    assert!(
        dip.as_deref().is_some_and(|s| !s.contains("$0")),
        "a promoted dip is basis-as-filed (never $0): {dip:?}"
    );
    let nudge =
        self_custody_nudge(&st).expect("a remaining exchange promoted tranche still nudges");
    assert!(
        !nudge.contains("$0"),
        "the self-custody copy no longer asserts a $0-basis unit for a promoted tranche: {nudge}"
    );
}

/// A FULLY-covered 2024 window (2024-01-01..03) whose min daily close is $12,000/BTC.
fn prices_2024_window_min_12k() -> StaticPrices {
    StaticPrices(
        [
            (date!(2024 - 01 - 01), dec!(12_000)),
            (date!(2024 - 01 - 02), dec!(15_000)),
            (date!(2024 - 01 - 03), dec!(14_000)),
        ]
        .into_iter()
        .collect(),
    )
}

/// An UNPROMOTED 1-BTC tranche (window 2024-01-01..03, min $12,000) sold WHOLE at $8,000 — BELOW the
/// window-low floor, so the un-clamped basis swap would over-quote a $4k loss the promote cannot file.
fn unpromoted_below_low_tranche() -> Vec<LedgerEvent> {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 03),
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2024-06-01 00:00 UTC),
        &w,
        100_000_000,
        8_000,
    );
    vec![t, sell]
}

/// Parse the `~$N` saving out of the `promote-tranche` funnel line.
fn funnel_quoted_saving(lines: &[String]) -> Usd {
    let l = lines
        .iter()
        .find(|l| l.contains("promote-tranche"))
        .expect("a promote-tranche funnel line is present");
    let after = l.split("~$").nth(1).expect("the funnel line quotes ~$N");
    let num: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();
    num.replace(',', "")
        .parse::<Usd>()
        .expect("a numeric saving")
}

/// The expected CLAMPED promote saving for the below-low fixture — the SAME `clamped_promote_year_saving`
/// helper the impl quotes (NOT the un-clamped `overpayment_delta_one`), rounded to whole dollars.
fn clamped_promote_saving(events: &[LedgerEvent], px: &StaticPrices, profile: &TaxProfile) -> Usd {
    let cf = filed_basis_for(
        px,
        100_000_000,
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 03),
    )
    .unwrap();
    clamped_promote_year_saving(
        events,
        px,
        &cfg(),
        &EventId::decision(1),
        cf.filed_basis,
        2024,
        Some(profile),
        &tables_2024(),
    )
    .round_dp(0)
}

/// §3 item 2 / tax r1 I-3: an unpromoted tranche's nudge advertises a saving the CLAMPED promote can
/// deliver (tax on the clamped $8k gain), NEVER an un-clamped over-quote (the $4k unfileable loss).
#[test]
fn the_promote_funnel_line_quotes_the_clamped_delta() {
    let events = unpromoted_below_low_tranche();
    let px = prices_2024_window_min_12k();
    let profile = consent_profile();
    let st = project(&events, &px, &cfg());
    let lines = overpayment_nudge_lines(
        &events,
        &st,
        &px,
        &cfg(),
        2024,
        Some(&profile),
        &tables_2024(),
    );
    assert!(
        lines.iter().any(|l| l.contains("promote-tranche")),
        "an unpromoted tranche gets a promote-tranche funnel line: {lines:?}"
    );
    assert_eq!(
        funnel_quoted_saving(&lines),
        clamped_promote_saving(&events, &px, &profile),
        "the funnel quotes the CLAMPED promote delta, never the un-clamped over-quote"
    );
}

/// A 1-BTC tranche (window 2017-12-01..03) promoted to a STORED `stored` floor. The recompute in
/// `promote_drift_advisory` runs against the CURRENT prices passed to it — so passing a window whose min
/// recomputes above/below `stored` drives the two directions.
fn promote_at_stored_floor(stored: rust_decimal::Decimal) -> Vec<LedgerEvent> {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 03),
    );
    let p = promote_ev(2, EventId::decision(1), stored);
    vec![t, p]
}

/// ★ BG-D3 (tax r2 I-1 / arch r2 I-2): the verify-drift advisory is direction-aware, and the FOLD is
/// unaffected — it always uses the STORED `filed_basis`, never the recomputed reference.
#[test]
fn verify_drift_advisory_is_direction_aware_and_the_fold_still_uses_the_stored_number() {
    let ev = promote_at_stored_floor(dec!(12_000));
    // Corrected data LOWERS the window min ($9k) ⇒ stored $12k recomputes ABOVE the reference (overstated).
    let overstated = promote_drift_advisory(&ev, &prices_with_window_min(9_000));
    assert!(
        overstated
            .iter()
            .any(|l| l.contains("void") && l.contains("re-promote") && l.contains("not yet filed")),
        "stored ABOVE recomputed → conditional void+re-promote copy (BG-D9-style 'if not yet filed'): \
         {overstated:?}"
    );
    // Corrected data RAISES the window min ($15k) ⇒ stored $12k recomputes BELOW the reference (understated).
    let understated = promote_drift_advisory(&ev, &prices_with_window_min(15_000));
    assert!(
        !understated.is_empty(),
        "the below-direction understated-floor advisory also fires (tax-safe, still surfaced): \
         {understated:?}"
    );
    // ★ the FOLD is unaffected — it uses the STORED filed_basis, not the recomputed one.
    let st = project(&ev, &prices_with_window_min(9_000), &cfg());
    assert_eq!(
        tranche_lot_basis(&st),
        dec!(12_000),
        "the fold uses the STORED number forever, regardless of a later price-data change"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 13 — Form 8275 content (Part I auto + Part II narrative) + BG-D7/D10 honest copy. PRIVACY:
// synthetic values only.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The raw events behind `promoted_state()` (Task 11): a 1-BTC tranche promoted to a $12,000 floor,
/// sold WHOLE at $20,000 — ABOVE the floor, so BG-D4's clamp does NOT bind.
fn promoted_disposal_events() -> Vec<LedgerEvent> {
    let w = exch();
    vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2024 - 01 - 01),
            date!(2024 - 01 - 10),
        ),
        promote_ev(2, EventId::decision(1), dec!(12_000)),
        sell_ev(
            "SELL",
            datetime!(2024-06-01 00:00 UTC),
            &w,
            100_000_000,
            20_000,
        ),
    ]
}

/// ★ `disclosure_8275` is `Some` iff a PROMOTED disposal leg is filed in `year` — an unpromoted
/// (still-$0) tranche takes no estimated position, so there is nothing to disclose.
#[test]
fn disclosure_is_some_iff_a_promoted_leg_is_filed_this_year() {
    let events = promoted_disposal_events();
    let state = promoted_state();
    assert!(
        disclosure_8275(&events, &state, 2024).is_some(),
        "a promoted disposal leg filed this year yields a disclosure"
    );

    let unpromoted = unpromoted_below_low_tranche();
    let unpromoted_state = project(&unpromoted, &prices(), &cfg());
    assert!(
        disclosure_8275(&unpromoted, &unpromoted_state, 2024).is_none(),
        "an UNPROMOTED tranche (still filed at $0) takes no estimated position — nothing to disclose"
    );
}

/// ★ Phase-1a T13: the happy path — Part II carries the promote's OWN recorded `part_ii_narrative`
/// VERBATIM (not empty, not a placeholder), and the disclosure is COMPLETE (`!incomplete`) whenever
/// that narrative is non-empty. Mirrors `disclosure_is_some_iff_a_promoted_leg_is_filed_this_year`'s
/// fixture (`promoted_disposal_events`/`promoted_state`), whose `promote_ev` records the real narrative
/// `"cash P2P purchase, no records; window bounded on-chain"` (see the `promote_ev` helper above).
#[test]
fn disclosure_part_ii_carries_the_narrative_verbatim_and_is_complete() {
    let events = promoted_disposal_events();
    let state = promoted_state();
    let d =
        disclosure_8275(&events, &state, 2024).expect("a promoted disposal leg files this year");
    assert_eq!(
        d.part_ii, "cash P2P purchase, no records; window bounded on-chain",
        "Part II carries the fixture's OWN recorded narrative verbatim"
    );
    assert!(
        !d.incomplete,
        "a non-empty Part II narrative is a COMPLETE disclosure"
    );
}

/// BG-D7/D10: `render()` names the underpayment as the penalty base, the §6662(h) 40% worst-case, and
/// the corrected §6664(c)(2) cite — and NEVER "safe harbor" (a promoted floor is a disclosed estimate,
/// not a harbor).
#[test]
fn disclosure_copy_names_the_underpayment_penalty_base_never_safe_harbor() {
    let events = promoted_disposal_events();
    let state = promoted_state();
    let d = disclosure_8275(&events, &state, 2024).unwrap();
    let text = d.render();
    assert!(
        !text.to_lowercase().contains("safe harbor"),
        "never 'safe harbor' (BG-D7): {text}"
    );
    assert!(
        text.contains("of the resulting additional tax"),
        "the penalty base is the underpayment, not the disallowed basis (BG-D10 / tax r1 M-3): {text}"
    );
    assert!(text.contains("40%"), "the §6662(h) worst-case rate: {text}");
    assert!(
        text.contains("\u{00a7}6664(c)(2)"),
        "the corrected reasonable-cause cite (tax r2 N-1): {text}"
    );
}

/// A 1-BTC tranche promoted to a $12,000 floor, sold WHOLE at $8,000 — BELOW the floor, so BG-D4's
/// loss clamp binds: the leg files `basis == proceeds == $8,000`, never the $12,000 pre-clamp floor.
fn promote_sold_below_floor_events() -> Vec<LedgerEvent> {
    let w = exch();
    vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2024 - 01 - 01),
            date!(2024 - 01 - 10),
        ),
        promote_ev(2, EventId::decision(1), dec!(12_000)),
        sell_ev(
            "SELL",
            datetime!(2024-06-01 00:00 UTC),
            &w,
            100_000_000,
            8_000,
        ),
    ]
}

/// ★ BG-D7 (tax r1 M-4/I-6): the Part I amount is the AS-FILED 8949 col (e) = the clamped basis (= net
/// proceeds), NOT the floor — disclosing the floor while filing less recreates the examiner mismatch.
#[test]
fn a_clamped_leg_disclosure_adds_the_no_loss_sentence_and_files_the_clamped_amount() {
    let events = promote_sold_below_floor_events();
    let state = project(&events, &prices(), &cfg());
    let d =
        disclosure_8275(&events, &state, 2024).expect("a promoted disposal leg files this year");
    let text = d.render();
    assert!(
        text.contains("limited so as not to report a loss from the estimate"),
        "the clamped-leg no-loss sentence: {text}"
    );
    let disposal = state.disposals.first().expect("the disposal");
    let leg = disposal.legs.first().expect("the leg");
    assert_eq!(
        leg.basis,
        dec!(8_000),
        "BG-D4's clamp bound to the $8,000 net proceeds"
    );
    assert_eq!(
        d.part_i[0].amount, leg.basis,
        "Part I amount = the AS-FILED 8949 col (e) basis"
    );
    assert_ne!(
        d.part_i[0].amount,
        dec!(12_000),
        "NOT the pre-clamp $12,000 floor"
    );
}

/// ★ Whole-branch tax M1 guard: the no-loss suffix condition is `leg.basis >= leg.proceeds`
/// (gain <= 0), widened from the old `== leg.proceeds` so it also catches a below-floor promoted leg
/// whose documented fee carry pushes basis ABOVE proceeds (gain < 0 — that exotic corner is the M1
/// target). This KAT pins the OTHER edge — the change must NOT over-fire: a promoted leg sold ABOVE its
/// floor takes the full floor as basis (no clamp), files a POSITIVE gain, and must carry NO suffix.
/// Mutation-proven: replacing `>=` with a tautology (e.g. `>= Decimal::ZERO`) reds this.
#[test]
fn an_above_floor_promoted_sale_files_positive_gain_and_no_no_loss_suffix() {
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2024 - 01 - 01),
            date!(2024 - 01 - 10),
        ),
        promote_ev(2, EventId::decision(1), dec!(5_000)), // a LOW floor
        sell_ev(
            "SELL",
            datetime!(2024-06-01 00:00 UTC),
            &w,
            100_000_000,
            8_000, // sold ABOVE the $5,000 floor
        ),
    ];
    let state = project(&events, &prices(), &cfg());
    let leg = only_disposal_leg(&state);
    assert!(leg.gain > dec!(0), "sold above the floor ⇒ positive gain");
    assert_eq!(leg.basis, dec!(5_000), "the full floor is filed (no clamp)");
    let d =
        disclosure_8275(&events, &state, 2024).expect("a promoted disposal leg files this year");
    assert!(
        !d.part_i[0]
            .description
            .contains("limited so as not to report a loss"),
        "an above-floor (gain > 0) promoted sale must NOT carry the no-loss suffix: {}",
        d.part_i[0].description
    );
}

/// BG-D11: a promoted tranche DONATED short-term files documented-only ($0 — the estimate evaporates,
/// `conservative_promote::clamped_leg_basis` with a $0 removal `net_proceeds_share`), so it takes NO
/// estimated position on the return. Part I must be 8949-DISPOSAL-scoped only — no 8283/removal item.
#[test]
fn removal_donation_legs_are_absent_from_part_i() {
    let events = promote_then_donate(
        100_000_000,
        dec!(60_000),
        dec!(50_000),
        date!(2024 - 01 - 01),
        date!(2024 - 01 - 10),
        datetime!(2024-06-01 00:00 UTC),
    );
    let state = project(&events, &prices(), &cfg());
    let d = disclosure_8275(&events, &state, 2024);
    assert!(
        d.as_ref()
            .is_none_or(|d| d.part_i.iter().all(|i| i.form == "8949")),
        "no 8283/removal items in Part I: {d:?}"
    );
}

/// `printed_8275` mirrors `Printed8283Rows`: Part I `amount`s are whole-dollar rounded (IRS half-up,
/// away from zero) at the line; Part II (text, not money) carries through unrounded.
#[test]
fn printed_8275_rounds_part_i_amounts_to_whole_dollars() {
    let events = promoted_disposal_events();
    let state = promoted_state();
    let mut d = disclosure_8275(&events, &state, 2024).unwrap();
    d.part_i[0].amount = dec!(12_000.50);
    let printed = btctax_core::tax::printed::printed_8275(&d);
    assert_eq!(
        printed.part_i[0].amount,
        dec!(12_001),
        "whole-dollar rounded at the line (IRS half-up: $12,000.50 -> $12,001)"
    );
    assert_eq!(
        printed.part_ii, d.part_ii,
        "Part II (text, not money) carries through unrounded"
    );
}
