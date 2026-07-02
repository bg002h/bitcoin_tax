//! Sub-project C, Task 5 — Compliance + persistability overlay KAT suite.
//!
//! Dedicated tests for `proposed_compliance_status`, `compliance_overlay`, `persistability`,
//! and their end-to-end wiring through `optimize_year`.
//!
//! Invariants under test:
//!   R0-C2 — a divergent post-hoc proposed pick is NEVER rescued as `StandingOrder`
//!            (§1.1012-1(j); no compliant post-hoc).
//!   R2-I1 — an attestation binds ONLY the exact attested selection; a divergent re-run pick
//!            is NOT laundered as `AttestedRecording` even when the disposal is in `attested`.
//!
//! All fixtures are synthetic (privacy — no real reads); exact Decimal, no float (NFR5);
//! federal-only (NFR4 determinism).
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::optimize::{
    compliance_overlay, optimize_year, persistability, proposed_compliance_status, Persistability,
};
use btctax_core::price::StaticPrices;
use btctax_core::project::{ComplianceStatus, DisposalCompliance, LotMethod, ProjectionConfig};
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxProfile};
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::{date, datetime, offset};

const LOT: i64 = 100_000_000; // 1 BTC in satoshis

// ── Synthetic tax table + profile ───────────────────────────────────────────────────────────

struct OneTable(TaxTable);
impl TaxTables for OneTable {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        (year == self.0.year).then_some(&self.0)
    }
}

fn synth(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.10),
                },
                OrdinaryBracket {
                    lower: dec!(50000),
                    rate: dec!(0.22),
                },
                OrdinaryBracket {
                    lower: dec!(90000),
                    rate: dec!(0.32),
                },
            ],
        },
    );
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(40000),
            max_fifteen: dec!(400000),
        },
    );
    OneTable(TaxTable {
        year,
        source: "SYNTHETIC",
        ordinary,
        ltcg,
        gift_annual_exclusion: dec!(19000),
        ss_wage_base: dec!(176100),
        gift_lifetime_exclusion: dec!(13_990_000),
    })
}

/// Single filer; ordinary == MAGI so a chosen ordinary income places the marginal rate.
fn profile(ordinary: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: ordinary,
        magi_excluding_crypto: ordinary,
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
    }
}

// ── Event / id builders ─────────────────────────────────────────────────────────────────────

fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}

fn exchange() -> WalletId {
    WalletId::Exchange {
        provider: "kraken".into(),
        account: "main".into(),
    }
}

fn eid(rf: &str) -> EventId {
    EventId::import(Source::Swan, SourceRef::new(rf))
}

fn lid(rf: &str) -> LotId {
    LotId {
        origin_event_id: eid(rf),
        split_sequence: 0,
    }
}

fn pick(rf: &str, sat: i64) -> LotPick {
    LotPick { lot: lid(rf), sat }
}

fn ev(rf: &str, ts: time::OffsetDateTime, w: WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: eid(rf),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w),
        payload: p,
    }
}

fn buy(rf: &str, ts: time::OffsetDateTime, w: WalletId, sat: i64, cost: Usd) -> LedgerEvent {
    ev(
        rf,
        ts,
        w,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}

fn sell(rf: &str, ts: time::OffsetDateTime, w: WalletId, sat: i64, proceeds: Usd) -> LedgerEvent {
    ev(
        rf,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}

/// A standing-order `MethodElection` decision. `effective_from` = 2025-01-01 covers all
/// post-transition disposals used in these KATs.
fn method_election(seq: u64, ts: time::OffsetDateTime, method: LotMethod) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::MethodElection(MethodElection {
            effective_from: date!(2025 - 01 - 01),
            method,
        }),
    }
}

/// A post-hoc `LotSelection` decision persisting `picks` as the explicit lot selection for the
/// disposal identified by `disposal_rf`. The timestamp governs whether the selection is
/// contemporaneous (made ≤ sale) or post-hoc (made > sale) per `disposal_compliance`.
fn lot_selection_decision(
    seq: u64,
    ts: time::OffsetDateTime,
    disposal_rf: &str,
    picks: Vec<LotPick>,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::LotSelection(LotSelection {
            disposal_event: eid(disposal_rf),
            lots: picks,
        }),
    }
}

fn cfg() -> ProjectionConfig {
    ProjectionConfig::default() // FIFO default, TreatmentC
}

/// `proposal_made` = 2026-07-01 (AFTER all 2026 disposal sale dates in these KATs → post-hoc).
fn made() -> time::Date {
    date!(2026 - 07 - 01)
}

fn no_attest() -> BTreeSet<EventId> {
    BTreeSet::new()
}

// ── Helpers for pure-function tests ─────────────────────────────────────────────────────────

fn dc(
    rf: &str,
    wallet: WalletId,
    date: time::Date,
    status: ComplianceStatus,
) -> DisposalCompliance {
    DisposalCompliance {
        disposal: eid(rf),
        wallet,
        date,
        status,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════
// § persistability — pure unit tests
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// Self-custody, made ≤ sale (both same day and made before sale) → ContemporaneousNow.
#[test]
fn persistability_self_custody_contemporaneous() {
    // Same-day: made == sale → contemporaneous
    assert_eq!(
        persistability(&cold(), date!(2026 - 06 - 01), date!(2026 - 06 - 01)),
        Persistability::ContemporaneousNow
    );
    // Made strictly before sale → still contemporaneous
    assert_eq!(
        persistability(&cold(), date!(2027 - 03 - 15), date!(2026 - 12 - 01)),
        Persistability::ContemporaneousNow
    );
}

/// Self-custody, made > sale (any year) → NeedsAttestation.
#[test]
fn persistability_self_custody_needs_attestation() {
    assert_eq!(
        persistability(&cold(), date!(2026 - 06 - 01), date!(2026 - 07 - 01)),
        Persistability::NeedsAttestation
    );
    // Still NeedsAttestation even in 2028 for self-custody (the 2027+ envelope only forbids
    // BROKER-held; self-custody is always within own-books).
    assert_eq!(
        persistability(&cold(), date!(2028 - 01 - 01), date!(2028 - 02 - 01)),
        Persistability::NeedsAttestation
    );
}

/// Broker-held, sale year 2026 (pre-2027), made > sale → NeedsAttestation (envelope not yet active).
#[test]
fn persistability_broker_pre_2027_needs_attestation() {
    assert_eq!(
        persistability(&exchange(), date!(2026 - 06 - 01), date!(2026 - 07 - 01)),
        Persistability::NeedsAttestation
    );
}

/// Broker-held, sale year ≥ 2027, made > sale → ForbiddenBroker2027 (own-books insufficient).
#[test]
fn persistability_broker_2027_forbidden() {
    assert_eq!(
        persistability(&exchange(), date!(2027 - 06 - 01), date!(2027 - 07 - 01)),
        Persistability::ForbiddenBroker2027
    );
    // 2028 is also forbidden (≥2027 envelope).
    assert_eq!(
        persistability(&exchange(), date!(2028 - 03 - 15), date!(2028 - 04 - 01)),
        Persistability::ForbiddenBroker2027
    );
}

/// Broker-held, sale year ≥ 2027, made ≤ sale → ForbiddenBroker2027 (NOT ContemporaneousNow).
/// §1.1012-1(j): the 2027+ broker envelope is AUTHORITATIVE and precedes the contemporaneous branch —
/// own-books identification is INSUFFICIENT for a 2027+ broker lot (own-books relief under
/// Notices 2025-07/2026-20 ENDS in 2026; broker-communicated specific-ID is required 2027+), so even a
/// genuinely contemporaneous (made ≤ sale) own-books pick CANNOT rescue it. This kills the latent
/// asymmetry (FOLLOWUPS Task-4) where `persistability` returned `ContemporaneousNow` while
/// `proposed_compliance_status` returned `NonCompliant` for the same input. FAILS without the fix.
#[test]
fn persistability_broker_2027_contemporaneous_is_forbidden() {
    // made == sale (contemporaneous timing) — still forbidden for a 2027+ broker lot.
    assert_eq!(
        persistability(&exchange(), date!(2027 - 06 - 01), date!(2027 - 06 - 01)),
        Persistability::ForbiddenBroker2027,
        "2027+ broker, made == sale: own-books contemporaneous ID is insufficient → ForbiddenBroker2027"
    );
    // made strictly BEFORE sale (contemporaneous timing) — also forbidden (envelope is authoritative).
    assert_eq!(
        persistability(&exchange(), date!(2027 - 06 - 01), date!(2027 - 05 - 01)),
        Persistability::ForbiddenBroker2027
    );
    // 2028 made ≤ sale — also forbidden (≥2027 envelope).
    assert_eq!(
        persistability(&exchange(), date!(2028 - 06 - 01), date!(2028 - 06 - 01)),
        Persistability::ForbiddenBroker2027
    );
    // Anti-regression: the broker-envelope precedence means it is NEVER ContemporaneousNow.
    assert_ne!(
        persistability(&exchange(), date!(2027 - 06 - 01), date!(2027 - 06 - 01)),
        Persistability::ContemporaneousNow,
        "anti-regression: the old made≤sale-first ordering must not surface ContemporaneousNow"
    );
}

/// Broker-held, sale year 2026 (pre-2027), made ≤ sale → ContemporaneousNow (REGRESSION).
/// Own-books relief still applies through 2026, so a genuinely contemporaneous broker pick persists
/// freely; the 2027+ envelope must NOT capture a pre-2027 broker lot. Confirms the fix is scoped to
/// 2027+ only (pre-2027 broker behavior is unchanged).
#[test]
fn persistability_broker_pre_2027_contemporaneous() {
    assert_eq!(
        persistability(&exchange(), date!(2026 - 06 - 01), date!(2026 - 06 - 01)),
        Persistability::ContemporaneousNow
    );
    // made strictly before the 2026 sale → still ContemporaneousNow.
    assert_eq!(
        persistability(&exchange(), date!(2026 - 12 - 01), date!(2026 - 03 - 15)),
        Persistability::ContemporaneousNow
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════
// § compliance_overlay — pure unit tests (R2-I1)
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// NonCompliant self-custody disposal in both `attested` AND `unchanged` → AttestedRecording.
/// This is the legitimate upgrade path: the user attested pick P1, persisted it, and the re-run
/// proposed the same P1 (= current selection) → the attestation confers AttestedRecording.
#[test]
fn overlay_attested_and_unchanged_upgrades_to_attested_recording() {
    let row = dc(
        "D",
        cold(),
        date!(2026 - 06 - 01),
        ComplianceStatus::NonCompliant,
    );
    let attested: BTreeSet<EventId> = [eid("D")].into();
    let unchanged: BTreeSet<EventId> = [eid("D")].into();
    let out = compliance_overlay(&[row], &attested, &unchanged);
    assert_eq!(out[0].status, ComplianceStatus::AttestedRecording);
}

/// NonCompliant disposal in `unchanged` but NOT in `attested` → stays NonCompliant.
/// The selection is unchanged but no user attestation exists.
#[test]
fn overlay_not_attested_stays_noncompliant() {
    let row = dc(
        "D",
        cold(),
        date!(2026 - 06 - 01),
        ComplianceStatus::NonCompliant,
    );
    let attested: BTreeSet<EventId> = BTreeSet::new();
    let unchanged: BTreeSet<EventId> = [eid("D")].into();
    let out = compliance_overlay(&[row], &attested, &unchanged);
    assert_eq!(out[0].status, ComplianceStatus::NonCompliant);
}

/// NonCompliant disposal in `attested` but NOT in `unchanged` (a divergent re-run pick P2 ≠ P1)
/// → stays NonCompliant. This is the R2-I1 no-laundering invariant: the attestation was for P1;
/// a later re-run that finds a strictly better P2 must NOT inherit the attestation as compliant.
#[test]
fn overlay_attested_but_divergent_stays_noncompliant() {
    let row = dc(
        "D",
        cold(),
        date!(2026 - 06 - 01),
        ComplianceStatus::NonCompliant,
    );
    let attested: BTreeSet<EventId> = [eid("D")].into();
    let unchanged: BTreeSet<EventId> = BTreeSet::new(); // D ∉ unchanged → proposed diverged from current
    let out = compliance_overlay(&[row], &attested, &unchanged);
    assert_eq!(
        out[0].status,
        ComplianceStatus::NonCompliant,
        "R2-I1: attestation for P1 must NOT launder a divergent re-run pick P2"
    );
}

/// 2027+ broker-held NonCompliant in both `attested` AND `unchanged` → stays NonCompliant.
/// The §A.5 / R2-M5 envelope forbids own-books attestation for broker-held 2027+ units.
#[test]
fn overlay_broker_2027_stays_noncompliant() {
    let row = dc(
        "D",
        exchange(),
        date!(2027 - 06 - 01),
        ComplianceStatus::NonCompliant,
    );
    let attested: BTreeSet<EventId> = [eid("D")].into();
    let unchanged: BTreeSet<EventId> = [eid("D")].into();
    let out = compliance_overlay(&[row], &attested, &unchanged);
    assert_eq!(
        out[0].status,
        ComplianceStatus::NonCompliant,
        "broker 2027+ must remain NonCompliant even if attested+unchanged"
    );
}

/// A Contemporaneous row is left untouched (no spurious downgrade or upgrade).
#[test]
fn overlay_contemporaneous_row_unchanged() {
    let row = dc(
        "D",
        cold(),
        date!(2026 - 06 - 01),
        ComplianceStatus::Contemporaneous,
    );
    let attested: BTreeSet<EventId> = [eid("D")].into();
    let unchanged: BTreeSet<EventId> = [eid("D")].into();
    let out = compliance_overlay(&[row], &attested, &unchanged);
    assert_eq!(out[0].status, ComplianceStatus::Contemporaneous);
}

/// A StandingOrder row is left untouched (no spurious downgrade or upgrade).
#[test]
fn overlay_standing_order_row_unchanged() {
    let effective = date!(2025 - 01 - 01);
    let row = dc(
        "D",
        cold(),
        date!(2026 - 06 - 01),
        ComplianceStatus::StandingOrder {
            effective_from: effective,
        },
    );
    let attested: BTreeSet<EventId> = [eid("D")].into();
    let unchanged: BTreeSet<EventId> = [eid("D")].into();
    let out = compliance_overlay(&[row], &attested, &unchanged);
    assert_eq!(
        out[0].status,
        ComplianceStatus::StandingOrder {
            effective_from: effective
        }
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════
// § proposed_compliance_status — pure unit tests (R0-C2)
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// proposed == current → returns `baseline_status` verbatim.
/// This is the ONLY path that may report StandingOrder: adopting the identical pick binds nothing
/// new, so the disposal's genuine compliance status (StandingOrder/Contemporaneous/NonCompliant) stands.
#[test]
fn proposed_status_unchanged_preserves_standing_order() {
    let p1 = vec![pick("A", LOT)];
    let baseline = ComplianceStatus::StandingOrder {
        effective_from: date!(2025 - 01 - 01),
    };
    let status = proposed_compliance_status(
        &cold(),
        date!(2026 - 06 - 01),
        date!(2026 - 07 - 01),
        &p1,
        &p1, // current == proposed → no divergence
        &baseline,
    );
    assert_eq!(
        status,
        ComplianceStatus::StandingOrder {
            effective_from: date!(2025 - 01 - 01)
        }
    );
}

/// Divergent pick, self-custody, made > sale → NonCompliant.
/// §1.1012-1(j): a standing order does NOT rescue a divergent post-hoc cherry-pick. The function
/// must never return StandingOrder for a divergent proposed pick.
#[test]
fn proposed_status_divergent_post_hoc_noncompliant() {
    let baseline = ComplianceStatus::StandingOrder {
        effective_from: date!(2025 - 01 - 01),
    };
    let status = proposed_compliance_status(
        &cold(),
        date!(2026 - 06 - 01), // sale
        date!(2026 - 07 - 01), // made AFTER sale → post-hoc
        &[pick("B", LOT)],     // proposed (divergent)
        &[pick("A", LOT)],     // current (different → diverges)
        &baseline,
    );
    assert_eq!(
        status,
        ComplianceStatus::NonCompliant,
        "standing order must NOT rescue a divergent post-hoc pick (§1.1012-1(j))"
    );
}

/// Divergent pick, made ≤ sale → Contemporaneous.
/// A proposed pick that is contemporaneous with the sale does not need attestation.
#[test]
fn proposed_status_divergent_contemporaneous() {
    let status = proposed_compliance_status(
        &cold(),
        date!(2026 - 06 - 01), // sale
        date!(2026 - 06 - 01), // made == sale → contemporaneous
        &[pick("B", LOT)],
        &[pick("A", LOT)],
        &ComplianceStatus::NonCompliant,
    );
    assert_eq!(status, ComplianceStatus::Contemporaneous);
}

/// Divergent pick, 2027+ broker, any made (both before and after sale) → NonCompliant.
/// The 2027+ broker envelope blocks own-books identification regardless of timing.
#[test]
fn proposed_status_divergent_broker_2027_noncompliant() {
    // Divergent + made BEFORE sale (contemporaneous timing) — still NonCompliant for broker 2027+.
    let status_before = proposed_compliance_status(
        &exchange(),
        date!(2027 - 06 - 01), // sale ≥2027
        date!(2027 - 05 - 01), // made < sale (contemporaneous timing)
        &[pick("B", LOT)],
        &[pick("A", LOT)],
        &ComplianceStatus::Contemporaneous,
    );
    assert_eq!(
        status_before,
        ComplianceStatus::NonCompliant,
        "broker 2027+ contemporaneous-timed divergent pick is still NonCompliant"
    );

    // Divergent + made AFTER sale (post-hoc) — also NonCompliant.
    let status_after = proposed_compliance_status(
        &exchange(),
        date!(2027 - 06 - 01), // sale ≥2027
        date!(2027 - 07 - 01), // made > sale (post-hoc)
        &[pick("B", LOT)],
        &[pick("A", LOT)],
        &ComplianceStatus::NonCompliant,
    );
    assert_eq!(status_after, ComplianceStatus::NonCompliant);
}

// ═══════════════════════════════════════════════════════════════════════════════════════════
// § End-to-end R0-C2: divergent post-hoc pick through optimize_year
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// Fixture: a HIFO standing order + two lots (LT low-basis vs ST high-basis, mirroring the
/// rate_aware_naive_hifo_loses_to_long_term KAT in optimize_mode1) with an already-executed
/// disposal (sale 2026-06-01 < proposal_made 2026-07-01).
///
/// The HIFO standing order makes the BASELINE pick the ST high-basis lot (ST_HB, basis $9,500,
/// $500 ST gain @ 32% = $160 tax). The rate-aware optimizer finds the LT low-basis lot
/// (LT_LB, basis $9,000, $1,000 LT gain @ 15% = $150 tax) is cheaper, so it proposes a
/// DIVERGENT pick (LT_LB ≠ ST_HB = current).
///
/// Assertions:
///   (a) `status == NonCompliant` — the in-force HIFO standing order does NOT rescue a divergent
///       post-hoc cherry-pick (§1.1012-1(j)); `proposed_compliance_status` must never return
///       StandingOrder for a divergent pick.
///   (b) `persistable == NeedsAttestation` — NOT ContemporaneousNow (kills the old
///       `persistability(date, date)` bug that made every disposal freely persistable).
///   (c) Every already-executed disposal in the proposal is NOT ContemporaneousNow.
#[test]
fn e2e_divergent_posthoc_pick_is_noncompliant() {
    let events = vec![
        method_election(1, datetime!(2025 - 01 - 01 00:00:00 UTC), LotMethod::Hifo),
        buy(
            "LT_LB",
            datetime!(2025 - 01 - 02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(9000),
        ),
        buy(
            "ST_HB",
            datetime!(2026 - 05 - 01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(9500),
        ),
        sell(
            "D",
            datetime!(2026 - 06 - 01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000)); // 32% marginal bracket

    // proposal_made (2026-07-01) > sale (2026-06-01) → D is already-executed (post-hoc).
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    // The optimizer finds LT_LB ($150 tax) < HIFO baseline ST_HB ($160 tax).
    // proposed [LT_LB] ≠ current [ST_HB] → diverges.
    let row = p
        .per_disposal
        .iter()
        .find(|d| d.disposal == eid("D"))
        .expect("disposal D in proposal");

    assert_eq!(
        row.proposed_selection,
        vec![pick("LT_LB", LOT)],
        "optimizer must pick LT lot (rate-aware)"
    );
    assert_ne!(
        row.proposed_selection, row.current_selection,
        "picks must diverge"
    );

    // (a) Standing order does NOT rescue a divergent post-hoc pick (R0-C2 / §1.1012-1(j)).
    assert_eq!(
        row.status,
        ComplianceStatus::NonCompliant,
        "divergent post-hoc pick must be NonCompliant, not rescued by StandingOrder"
    );

    // (b) Persistability: already-executed → NeedsAttestation, NOT ContemporaneousNow.
    assert_eq!(
        row.persistable,
        Persistability::NeedsAttestation,
        "already-executed disposal must require attestation, not be freely persistable"
    );
    assert_ne!(
        row.persistable,
        Persistability::ContemporaneousNow,
        "anti-regression: old persistability(date,date) bug must not surface"
    );

    // (c) Every disposal in the proposal (all already-executed) must NOT be ContemporaneousNow.
    for d in &p.per_disposal {
        assert_ne!(
            d.persistable,
            Persistability::ContemporaneousNow,
            "disposal {} — all already-executed disposals must not be ContemporaneousNow",
            d.disposal.canonical()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════════════
// § End-to-end R2-I1: attestation binds only the exact attested selection, through optimize_year
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// (i) No-change re-run: the user attested the post-hoc LotSelection P1 = [lot A] for disposal
/// D. On a re-run where only lot A exists (optimizer must keep P1 → proposed == current → D ∈
/// unchanged), D's row is `AttestedRecording`.
///
/// The post-hoc LotSelection (made 2026-06-15 > sale 2026-06-01) makes the baseline status
/// `NonCompliant` (via `disposal_compliance`). The overlay upgrades it to `AttestedRecording`
/// because D ∈ attested ∩ unchanged and the envelope is satisfied (self-custody).
#[test]
fn e2e_attested_unchanged_is_attested_recording() {
    let sale_ts = datetime!(2026 - 06 - 01 00:00:00 UTC);
    // Post-hoc selection: made 2026-06-15 > sale 2026-06-01 → NonCompliant in disposal_compliance.
    let sel_ts = datetime!(2026 - 06 - 15 00:00:00 UTC);

    let events = vec![
        buy(
            "A",
            datetime!(2025 - 01 - 02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(8000),
        ),
        sell("D", sale_ts, cold(), LOT, dec!(10000)),
        // Explicit post-hoc selection P1 = [lot A]; only lot A is available → optimizer keeps it.
        lot_selection_decision(100, sel_ts, "D", vec![pick("A", LOT)]),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));

    // D is attested (user has attested this post-hoc selection).
    let attested: BTreeSet<EventId> = [eid("D")].into_iter().collect();

    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &attested,
        made(),
    )
    .expect("computable");

    let row = p
        .per_disposal
        .iter()
        .find(|d| d.disposal == eid("D"))
        .expect("disposal D in proposal");

    // Only lot A exists → optimizer must keep P1 = [A] = current.
    assert_eq!(row.proposed_selection, vec![pick("A", LOT)]);
    assert_eq!(row.current_selection, vec![pick("A", LOT)]);

    // attested ∧ unchanged ∧ within own-books envelope → overlay upgrades to AttestedRecording.
    assert_eq!(
        row.status,
        ComplianceStatus::AttestedRecording,
        "attested + unchanged post-hoc selection must be AttestedRecording"
    );
}

/// (ii) Divergent re-run (R2-I1 — no laundering): after attesting P1 = [lot A_ST] for disposal
/// D, a newly-acquired lower-tax lot B_LT causes the re-run's optimizer to propose a strictly
/// better P2 = [lot B_LT] ≠ P1 (so D ∉ `unchanged`). D's row must be `NonCompliant` — the
/// attestation for P1 does NOT launder the never-attested P2.
///
/// Fixture: lot A_ST = ST high-basis ($9,500 basis, $500 ST gain @ 32% = $160 tax); lot B_LT
/// = LT low-basis ($9,000 basis, $1,000 LT gain @ 15% = $150 tax). Explicit selection P1 = [A_ST]
/// (post-hoc, made 2026-06-15 > sale 2026-06-01 → baseline NonCompliant). Optimizer finds B_LT →
/// proposes P2 = [B_LT] ≠ [A_ST] = P1 → D ∉ unchanged → overlay does NOT fire.
#[test]
fn e2e_attested_divergent_stays_noncompliant() {
    let sale_ts = datetime!(2026 - 06 - 01 00:00:00 UTC);
    let sel_ts = datetime!(2026 - 06 - 15 00:00:00 UTC); // post-hoc selection of P1 = [A_ST]

    let events = vec![
        // P1: ST high-basis lot — the persisted-and-attested selection.
        buy(
            "A_ST",
            datetime!(2026 - 05 - 01 00:00:00 UTC), // ST at 2026-06-01 (< 1 yr)
            cold(),
            LOT,
            dec!(9500),
        ),
        // P2 candidate: LT low-basis lot — the optimizer's preferred pick.
        buy(
            "B_LT",
            datetime!(2025 - 01 - 02 00:00:00 UTC), // LT at 2026-06-01 (> 1 yr)
            cold(),
            LOT,
            dec!(9000),
        ),
        sell("D", sale_ts, cold(), LOT, dec!(10000)),
        // Explicit post-hoc selection P1 = [A_ST] for D.
        lot_selection_decision(100, sel_ts, "D", vec![pick("A_ST", LOT)]),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000)); // 32% ST bracket, 15% LT

    // D is attested (user attested P1 = [A_ST]).
    let attested: BTreeSet<EventId> = [eid("D")].into_iter().collect();

    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &attested,
        made(),
    )
    .expect("computable");

    let row = p
        .per_disposal
        .iter()
        .find(|d| d.disposal == eid("D"))
        .expect("disposal D in proposal");

    // Optimizer finds B_LT ($150 tax) < A_ST ($160 tax) → proposes P2 = [B_LT].
    assert_eq!(
        row.proposed_selection,
        vec![pick("B_LT", LOT)],
        "optimizer must pick the LT lot (lower tax)"
    );
    // current is P1 = [A_ST] (from the explicit LotSelection in the baseline fold).
    assert_eq!(row.current_selection, vec![pick("A_ST", LOT)]);

    // proposed ≠ current → D ∉ unchanged → overlay does NOT fire.
    // R2-I1: attestation for P1 must NOT launder the never-attested P2.
    assert_eq!(
        row.status,
        ComplianceStatus::NonCompliant,
        "R2-I1: attestation for P1 must not launder a divergent re-run pick P2"
    );
}
