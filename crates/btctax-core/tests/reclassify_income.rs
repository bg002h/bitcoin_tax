//! SE Chunk C — `ReclassifyIncome` decision KATs (Task 1).
//!
//! All fixtures are SYNTHETIC. No real user data is read.
//!
//! Covered (per spec Task-1 KAT list):
//!  - Headline flip: River-style Income{Reward, false} + reclassify{business=true, kind=Mining}
//!    → IncomeRecord{Mining, true} → compute_se_tax Some (P2-D math). Before decision: None.
//!  - Business-only flip: kind stays original.
//!  - Engine-B invariance: compute_tax_year figures IDENTICAL before vs after a business-only flip.
//!  - Kind flip NIIT (NON-VACUOUS): MAGI $205,000 > Single threshold $200,000; exact NONZERO deltas
//!    both directions. Hand-derived values in comments.
//!  - Duplicate → DecisionConflict + FIRST-WINS projected value.
//!  - Void reverts: VoidDecisionEvent → original business/kind.
//!  - Bad target ×2: missing event; non-Income event → Hard DecisionConflict + projection unchanged.
//!  - Back-compat: old vault (no variant) loads unchanged.
//!  - fingerprint() == None and serde round-trips: covered in event.rs tests.
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::tax::compute::compute_tax_year;
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
use btctax_core::{compute_se_tax, se_net_income};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{datetime, offset};

// ── Fixture helpers ────────────────────────────────────────────────────────────────────────────

fn river_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "river".into(),
        account: "default".into(),
    }
}

/// Build a `ReclassifyIncome` decision event.
fn reclassify_ev(
    seq: u64,
    ts: time::OffsetDateTime,
    income_id: EventId,
    business: bool,
    kind: Option<IncomeKind>,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ReclassifyIncome(ReclassifyIncome {
            income_event: income_id,
            business,
            kind,
        }),
    }
}

/// Build a `VoidDecisionEvent` decision.
fn void_ev(seq: u64, ts: time::OffsetDateTime, target: EventId) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: target,
        }),
    }
}

/// A minimal synthetic TaxTable (Single filing status only; synthetic values not real IRS).
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
                    lower: dec!(250000),
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

/// A minimal synthetic TaxTable for the SE-tax engine (same ss_wage_base as real TY2025).
fn synth_se(year: i32) -> btctax_core::TaxTable {
    synth(year).0
}

/// Profile for engine-B invariance and NIIT tests.
/// Single, ordinary_taxable_income=$0, magi_excluding_crypto=$205,000 (above $200k threshold),
/// qd=$0, no carryforward, no W-2.
fn niit_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(0),
        magi_excluding_crypto: dec!(205000),
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

// ── Canonical timestamps ───────────────────────────────────────────────────────────────────────
fn ts_income() -> time::OffsetDateTime {
    datetime!(2025-03-01 12:00:00 UTC)
}
fn ts_decision() -> time::OffsetDateTime {
    datetime!(2025-03-02 00:00:00 UTC)
}

// ── Income event identity ─────────────────────────────────────────────────────────────────────
fn income_id() -> EventId {
    EventId::import(Source::River, SourceRef::new("in|2025-03-01|income|100000"))
}

// ── KATs ──────────────────────────────────────────────────────────────────────────────────────

/// Headline flip: River-style Income{Reward, business:false} + ReclassifyIncome{business=true,
/// kind=Mining} → IncomeRecord{Mining, true} → compute_se_tax Some (P2-D math).
/// Before the decision: se_net_income=0 → compute_se_tax=None.
///
/// Hand-derived SE-tax values (fmv=$10,000, Single, no W-2, ss_wage_base=$176,100):
///   net_se  = $10,000.00
///   base    = round_cents($10,000 × 0.9235) = $9,235.00
///   ss      = round_cents(0.124 × $9,235)   = $1,145.14
///   medicare= round_cents(0.029 × $9,235)   = $267.82  (HALF_EVEN: $267.815 → .82, 2 is even)
///   addl    = 0  (base $9,235 < Single addl-threshold $200,000)
///   total   = $1,412.96
///   deductible_half = round_cents(($1,145.14 + $267.82) / 2) = round_cents($706.48) = $706.48
#[test]
fn headline_flip_reward_to_mining_business_true() {
    let fmv = dec!(10000);
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(fmv),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };

    // ── BEFORE decision: no SE income ─────────────────────────────────────────────────────────
    let cfg = ProjectionConfig::default();
    let before = project(
        std::slice::from_ref(&income),
        &StaticPrices::default(),
        &cfg,
    );
    assert_eq!(
        se_net_income(&before, 2025),
        dec!(0),
        "before: Reward+business=false → not SE-eligible"
    );
    let tbl = synth_se(2025);
    assert!(
        compute_se_tax(
            &before,
            2025,
            FilingStatus::Single,
            &tbl,
            dec!(0),
            dec!(0),
            dec!(0)
        )
        .is_none(),
        "before: compute_se_tax must be None (no SE income)"
    );

    // ── AFTER decision: business=true, kind=Mining ─────────────────────────────────────────────
    let reclassify = reclassify_ev(
        1,
        ts_decision(),
        income_id(),
        true,
        Some(IncomeKind::Mining),
    );
    let after = project(&[income, reclassify], &StaticPrices::default(), &cfg);
    assert!(
        after.blockers.is_empty(),
        "no blockers expected: {:?}",
        after.blockers
    );

    // Projected IncomeRecord must reflect the override.
    assert_eq!(after.income_recognized.len(), 1);
    let rec = &after.income_recognized[0];
    assert_eq!(
        rec.kind,
        IncomeKind::Mining,
        "kind must be Mining after flip"
    );
    assert!(rec.business, "business must be true after flip");
    assert_eq!(rec.usd_fmv, fmv, "FMV must be unchanged");

    // SE-eligible now.
    assert_eq!(
        se_net_income(&after, 2025),
        dec!(10000),
        "after: se_net_income must be $10,000"
    );
    let r = compute_se_tax(
        &after,
        2025,
        FilingStatus::Single,
        &tbl,
        dec!(0),
        dec!(0),
        dec!(0),
    )
    .expect("compute_se_tax must be Some after flip");
    assert_eq!(r.net_se, dec!(10000.00));
    assert_eq!(r.base, dec!(9235.00));
    assert_eq!(r.ss, dec!(1145.14));
    assert_eq!(r.medicare, dec!(267.82)); // HALF_EVEN: 267.815 → 267.82 (2 is even)
    assert_eq!(r.addl, dec!(0.00));
    assert_eq!(r.total, dec!(1412.96));
    assert_eq!(r.deductible_half, dec!(706.48));
}

/// Business-only flip: kind stays original (Reward); only `business` changes.
#[test]
fn business_only_flip_kind_unchanged() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };
    // kind=None → keep original kind (Reward)
    let reclassify = reclassify_ev(1, ts_decision(), income_id(), true, None);
    let after = project(
        &[income, reclassify],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(after.blockers.is_empty());
    let rec = &after.income_recognized[0];
    assert_eq!(
        rec.kind,
        IncomeKind::Reward,
        "kind must stay Reward (no kind override)"
    );
    assert!(rec.business, "business must flip to true");
}

/// Engine-B invariance: a business-only flip (Reward, false → true) does NOT change any
/// compute_tax_year figures. crypto_ord sums ALL income regardless of kind/business; Reward is not
/// NII (interest_nii=0 both ways). The delta in ordinary tax + NIIT is exactly zero.
///
/// This is the spec's "crypto_ord is kind/business-AGNOSTIC" guarantee.
#[test]
fn engine_b_invariance_business_only_flip() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward, // NOT Interest → interest_nii=0
            business: false,
        }),
    };
    let cfg = ProjectionConfig::default();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let p = niit_profile();
    let year = 2025;

    let before_state = project(std::slice::from_ref(&income), &prices, &cfg);
    let reclassify = reclassify_ev(1, ts_decision(), income_id(), true, None); // business only
    let after_state = project(&[income, reclassify], &prices, &cfg);

    let TaxOutcome::Computed(before_r) =
        compute_tax_year(&[], &before_state, year, Some(&p), &tables)
    else {
        panic!("engine B must be computable BEFORE");
    };
    let TaxOutcome::Computed(after_r) =
        compute_tax_year(&[], &after_state, year, Some(&p), &tables)
    else {
        panic!("engine B must be computable AFTER");
    };

    // All engine-B figures must be IDENTICAL (business flip does not move crypto_ord or interest_nii).
    assert_eq!(
        before_r.ordinary_from_crypto, after_r.ordinary_from_crypto,
        "ordinary_from_crypto must be identical"
    );
    assert_eq!(
        before_r.niit, after_r.niit,
        "NIIT delta must be identical (Reward is not NII)"
    );
    assert_eq!(
        before_r.total_federal_tax_attributable, after_r.total_federal_tax_attributable,
        "total_federal_tax_attributable must be identical"
    );
    assert_eq!(
        before_r.ltcg_tax, after_r.ltcg_tax,
        "ltcg_tax must be identical"
    );
}

/// Kind flip moves NIIT correctly — NON-VACUOUS KAT.
///
/// Fixture: Single, MAGI_excluding_crypto=$205,000 > $200,000 threshold. Income FMV=$10,000.
/// No QD, no disposals, no W-2.
///
/// Hand-derived NIIT computations:
///
/// When kind=Reward (not Interest):
///   interest_nii = $0 → nii_with = $0 → niit_with = $0
///   nii_without = $0 → niit_without = $0
///   niit_delta = $0
///
/// When kind=Interest (IS NII per §1411(c)(1)(A)(i)):
///   interest_nii = $10,000
///   nii_with = $10,000
///   magi_with = $205,000 + $10,000 = $215,000
///   over = $215,000 − $200,000 = $15,000
///   capped = min($10,000, $15,000) = $10,000
///   niit_with = round_cents(3.8% × $10,000) = $380.00
///   nii_without = $0 → niit_without = $0
///   niit_delta = $380.00 − $0 = $380.00
///
/// When kind=Mining (not Interest):
///   interest_nii = $0 → niit_delta = $0 (same as Reward)
///
/// Reward→Interest: niit rises by $380.00 (NONZERO ✓)
/// Interest→Mining: niit falls by $380.00 to $0 (NONZERO delta ✓)
#[test]
fn kind_flip_niit_non_vacuous_reward_to_interest_raises_interest_to_mining_lowers() {
    let fmv = dec!(10000);
    let cfg = ProjectionConfig::default();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let p = niit_profile();
    let year = 2025;

    // ── Case A: base Income{Reward, false} — no decision ──────────────────────────────────────
    let income_reward = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(fmv),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };
    let state_reward = project(std::slice::from_ref(&income_reward), &prices, &cfg);
    let TaxOutcome::Computed(r_reward) =
        compute_tax_year(&[], &state_reward, year, Some(&p), &tables)
    else {
        panic!("must be computable for Reward base");
    };
    assert_eq!(r_reward.niit, dec!(0), "Reward: niit_delta must be $0");

    // ── Case B: Reward → Interest (flip kind only, business stays false) ──────────────────────
    let reclassify_to_interest = reclassify_ev(
        1,
        ts_decision(),
        income_id(),
        false,
        Some(IncomeKind::Interest),
    );
    let state_interest = project(
        &[income_reward.clone(), reclassify_to_interest],
        &prices,
        &cfg,
    );
    assert!(
        state_interest.blockers.is_empty(),
        "Reward→Interest: no blockers"
    );
    let TaxOutcome::Computed(r_interest) =
        compute_tax_year(&[], &state_interest, year, Some(&p), &tables)
    else {
        panic!("must be computable for Interest case");
    };
    // niit_delta = $380.00 (derived above)
    assert_eq!(
        r_interest.niit,
        dec!(380.00),
        "Reward→Interest: niit_delta must be $380.00 (NONZERO)"
    );
    // Rise: from Reward ($0) to Interest ($380.00) = +$380.00
    let rise = r_interest.niit - r_reward.niit;
    assert_eq!(rise, dec!(380.00), "Reward→Interest: niit rises by $380.00");

    // ── Case C: Income{Interest, false} → Mining (flip kind, business=true) ──────────────────
    // New income event with kind=Interest (as if originally imported as Interest).
    let interest_id = EventId::import(
        Source::River,
        SourceRef::new("in|2025-03-15|interest|100000"),
    );
    let income_interest = LedgerEvent {
        id: interest_id.clone(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(fmv),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Interest,
            business: false,
        }),
    };
    // Baseline with just Interest (no decision).
    let state_interest_base = project(std::slice::from_ref(&income_interest), &prices, &cfg);
    let TaxOutcome::Computed(r_interest_base) =
        compute_tax_year(&[], &state_interest_base, year, Some(&p), &tables)
    else {
        panic!("must be computable for Interest base");
    };
    assert_eq!(
        r_interest_base.niit,
        dec!(380.00),
        "Interest base: niit_delta must be $380.00"
    );

    // Reclassify Interest → Mining (business=true).
    let reclassify_to_mining = reclassify_ev(
        1,
        ts_decision(),
        interest_id,
        true,
        Some(IncomeKind::Mining),
    );
    let state_mining = project(&[income_interest, reclassify_to_mining], &prices, &cfg);
    assert!(
        state_mining.blockers.is_empty(),
        "Interest→Mining: no blockers"
    );
    let TaxOutcome::Computed(r_mining) =
        compute_tax_year(&[], &state_mining, year, Some(&p), &tables)
    else {
        panic!("must be computable for Mining case");
    };
    // Interest→Mining: interest_nii falls to $0 → niit_delta = $0
    assert_eq!(
        r_mining.niit,
        dec!(0.00),
        "Interest→Mining: niit_delta must be $0 (NONZERO fall)"
    );
    // Fall: from Interest ($380.00) to Mining ($0) = −$380.00
    let fall = r_interest_base.niit - r_mining.niit;
    assert_eq!(
        fall,
        dec!(380.00),
        "Interest→Mining: niit falls by $380.00 (NONZERO)"
    );
}

/// Duplicate → DecisionConflict + FIRST-WINS projected value.
/// Two non-voided ReclassifyIncome on the same income event; the SECOND fires DecisionConflict,
/// and the FIRST-WINS override (decision seq 1 = business=true, kind=Mining) stays in the projection.
#[test]
fn duplicate_reclassify_income_conflict_first_wins() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };
    // First decision (seq=1): business=true, kind=Mining.
    let first = reclassify_ev(
        1,
        ts_decision(),
        income_id(),
        true,
        Some(IncomeKind::Mining),
    );
    // Second decision (seq=2): business=false, kind=Staking (must be IGNORED).
    let second = reclassify_ev(
        2,
        ts_decision(),
        income_id(),
        false,
        Some(IncomeKind::Staking),
    );
    let state = project(
        &[income, first, second],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // Exactly one DecisionConflict blocker (for the second/duplicate decision).
    let conflicts: Vec<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::DecisionConflict)
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "exactly one DecisionConflict expected: {:?}",
        state.blockers
    );

    // FIRST-WINS: seq-1 override (business=true, kind=Mining) governs.
    assert_eq!(state.income_recognized.len(), 1);
    let rec = &state.income_recognized[0];
    assert_eq!(
        rec.kind,
        IncomeKind::Mining,
        "FIRST-WINS: kind must be Mining"
    );
    assert!(rec.business, "FIRST-WINS: business must be true");
}

/// Void reverts: ReclassifyIncome + VoidDecisionEvent → original business/kind project again.
#[test]
fn void_reverts_to_original() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };
    let reclassify_id = EventId::decision(1);
    let reclassify = LedgerEvent {
        id: reclassify_id.clone(),
        utc_timestamp: ts_decision(),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ReclassifyIncome(ReclassifyIncome {
            income_event: income_id(),
            business: true,
            kind: Some(IncomeKind::Mining),
        }),
    };
    let void = void_ev(2, ts_decision(), reclassify_id);

    let state = project(
        &[income, reclassify, void],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        state.blockers.is_empty(),
        "void of a revocable decision must not produce a blocker: {:?}",
        state.blockers
    );

    // Reverted to original.
    let rec = &state.income_recognized[0];
    assert_eq!(
        rec.kind,
        IncomeKind::Reward,
        "after void: kind reverts to Reward"
    );
    assert!(!rec.business, "after void: business reverts to false");
}

/// Bad target ×2: missing event → Hard DecisionConflict; projection unchanged.
#[test]
fn bad_target_missing_event_yields_hard_blocker() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };
    // ReclassifyIncome points at a completely non-existent event.
    let bogus_id = EventId::import(Source::River, SourceRef::new("in|bogus|no-such-event"));
    let bad = reclassify_ev(1, ts_decision(), bogus_id, true, Some(IncomeKind::Mining));

    let state = project(
        &[income, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // Hard DecisionConflict blocker for the decision with the bad target.
    let conflicts: Vec<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::DecisionConflict)
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "missing-target must yield exactly 1 DecisionConflict: {:?}",
        state.blockers
    );
    // The decision is EXCLUDED → projection unchanged (original Reward/false).
    let rec = &state.income_recognized[0];
    assert_eq!(
        rec.kind,
        IncomeKind::Reward,
        "bad target: kind must remain Reward"
    );
    assert!(!rec.business, "bad target: business must remain false");
}

/// Bad target ×2: non-Income event → Hard DecisionConflict; projection unchanged.
/// (Points at an Acquire event instead of an Income event.)
#[test]
fn bad_target_non_income_event_yields_hard_blocker() {
    let acquire_id = EventId::import(Source::Coinbase, SourceRef::new("in|acq-001"));
    let acquire = LedgerEvent {
        id: acquire_id.clone(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange {
            provider: "coinbase".into(),
            account: "main".into(),
        }),
        payload: EventPayload::Acquire(Acquire {
            sat: 1_000_000,
            usd_cost: dec!(60000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };
    // ReclassifyIncome wrongly points at the Acquire event.
    let bad = reclassify_ev(1, ts_decision(), acquire_id, true, Some(IncomeKind::Mining));

    let state = project(
        &[acquire, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // Hard DecisionConflict blocker for the decision with the bad (non-Income) target.
    let conflicts: Vec<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::DecisionConflict)
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "non-Income target must yield exactly 1 DecisionConflict: {:?}",
        state.blockers
    );
    // The Acquire event is unaffected (no income_recognized at all).
    assert!(
        state.income_recognized.is_empty(),
        "no income should be recognized (Acquire is not Income)"
    );
    // NOT a panic: the test reaching here confirms it.
}

/// Back-compat: an old-vault event stream WITHOUT any ReclassifyIncome loads and projects unchanged.
/// (Trivially true — this KAT pins the property.)
#[test]
fn old_vault_without_variant_loads_unchanged() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    };
    // No ReclassifyIncome events in the stream — old vault.
    let state = project(
        &[income],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(state.blockers.is_empty(), "old vault: no blockers");
    assert_eq!(state.income_recognized.len(), 1);
    let rec = &state.income_recognized[0];
    assert_eq!(rec.kind, IncomeKind::Reward);
    assert!(!rec.business);
}

/// Serde round-trip: ReclassifyIncome with kind=Some and kind=None both survive JSON serialization.
/// (This mirrors the coverage in event.rs::every_variant_serde_round_trips but is here for the
/// KAT inventory completeness per the spec.)
#[test]
fn reclassify_income_serde_round_trips_both_kind_arms() {
    let ri_some = EventPayload::ReclassifyIncome(ReclassifyIncome {
        income_event: income_id(),
        business: true,
        kind: Some(IncomeKind::Mining),
    });
    let ri_none = EventPayload::ReclassifyIncome(ReclassifyIncome {
        income_event: income_id(),
        business: false,
        kind: None,
    });
    for p in [ri_some, ri_none] {
        let json = serde_json::to_string(&p).unwrap();
        let back: EventPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}

/// No-fingerprint KAT: ReclassifyIncome.fingerprint() == None (decision variant; catch-all covers it).
/// (Also pinned in event.rs::reclassify_income_decision_has_no_fingerprint — here for completeness.)
#[test]
fn reclassify_income_fingerprint_is_none() {
    let ri = EventPayload::ReclassifyIncome(ReclassifyIncome {
        income_event: income_id(),
        business: true,
        kind: Some(IncomeKind::Mining),
    });
    assert!(
        btctax_core::persistence::fingerprint(&ri).is_none(),
        "decision variants must have no fingerprint"
    );
    let ri_no_kind = EventPayload::ReclassifyIncome(ReclassifyIncome {
        income_event: income_id(),
        business: false,
        kind: None,
    });
    assert!(btctax_core::persistence::fingerprint(&ri_no_kind).is_none());
}

/// [Chunk B / I1] Engine-B invariance: `schedule_c_expenses` $0 vs $20,000 does NOT change any
/// `compute_tax_year` figures. Engine B always uses GROSS `crypto_ord` (Σ usd_fmv over all income,
/// regardless of kind or business); `schedule_c_expenses` is advisory-only and only enters
/// `compute_se_tax` (the standalone SE engine — D5: not folded into engine B).
///
/// Fixture: Single, business Mining $100,000 FMV, TY2025 synthetic table. Profile A:
/// `schedule_c_expenses=$0`; Profile B: `schedule_c_expenses=$20,000`. All other profile fields
/// identical (MAGI $205,000 for NIIT sensitivity — Mining is not NII, so NIIT is also unaffected).
///
/// Asserts `ordinary_from_crypto`, `niit`, `ltcg_tax`, and `total_federal_tax_attributable` are
/// bit-identical across both profiles.
///
/// This is the spec's "engine B is agnostic to schedule_c_expenses" guarantee.
#[test]
fn engine_b_invariance_schedule_c_expenses_zero_vs_20k() {
    let income = LedgerEvent {
        id: income_id(),
        utc_timestamp: ts_income(),
        original_tz: offset!(+00:00),
        wallet: Some(river_wallet()),
        payload: EventPayload::Income(Income {
            sat: 100_000_000,
            usd_fmv: Some(dec!(100000)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Mining,
            business: true,
        }),
    };
    let state = project(
        std::slice::from_ref(&income),
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let tables = synth(2025);
    let year = 2025;

    // Profile A: no Schedule C expenses.
    let profile_zero = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(0),
        magi_excluding_crypto: dec!(205000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    // Profile B: $20,000 Schedule C expenses — only this field differs.
    let profile_20k = TaxProfile {
        schedule_c_expenses: dec!(20000),
        ..profile_zero.clone()
    };

    let TaxOutcome::Computed(r_zero) =
        compute_tax_year(&[], &state, year, Some(&profile_zero), &tables)
    else {
        panic!("engine B must be computable with schedule_c_expenses=$0");
    };
    let TaxOutcome::Computed(r_20k) =
        compute_tax_year(&[], &state, year, Some(&profile_20k), &tables)
    else {
        panic!("engine B must be computable with schedule_c_expenses=$20,000");
    };

    // All engine-B output figures must be bit-identical (schedule_c_expenses does not enter engine B).
    assert_eq!(
        r_zero.ordinary_from_crypto, r_20k.ordinary_from_crypto,
        "ordinary_from_crypto must be identical (engine B uses GROSS crypto_ord, not net)"
    );
    assert_eq!(
        r_zero.niit, r_20k.niit,
        "NIIT must be identical (schedule_c_expenses does not affect MAGI or NII)"
    );
    assert_eq!(
        r_zero.ltcg_tax, r_20k.ltcg_tax,
        "ltcg_tax must be identical (no disposals in fixture)"
    );
    assert_eq!(
        r_zero.total_federal_tax_attributable, r_20k.total_federal_tax_attributable,
        "total_federal_tax_attributable must be identical"
    );
}
