//! TASK 7 — End-to-end golden KATs against the bundled TY2025 table.
//!
//! Each KAT is hand-derived from Rev. Proc. 2024-40 §2.01/§2.03 (exact bundled values) and asserts
//! that `compute_tax_year` with `BundledTaxTables::load()` produces the same number.  No synthetic
//! tax table is used — every assertion is pinned to the real TY2025 Rev. Proc. data.
//!
//! `LedgerState` is constructed directly (no projection / event parse / PriceProvider required)
//! because `compute_tax_year` only reads `state.disposals[*].legs[*].{gain, term}`,
//! `state.income_recognized[*].usd_fmv`, and `state.blockers`.  All other fields stay at
//! `LedgerState::default()`.  This is explicitly permitted by the plan ("or build the LedgerState
//! directly ... with internally-consistent Disposal legs").
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    compute_tax_year, BasisSource, BlockerKind, Carryforward, Disposal, DisposalLeg, DisposeKind,
    EventId, FilingStatus, LedgerState, LotId, Source, SourceRef, TaxOutcome, TaxProfile,
    TaxResult, Term, Usd,
};
use rust_decimal_macros::dec;
use time::macros::date;

// ── Profile factories ─────────────────────────────────────────────────────────────────────────────

/// Single-filer profile.  `ord` = ordinary taxable income (excl. crypto); `magi` = MAGI excl.
/// crypto (must already include QD per B.1/ambiguity-#5 contract); `qd` = qualified dividends +
/// other preferential income sharing the §1(h) stack.
fn single(ord: Usd, magi: Usd, qd: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: qd,
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
    }
}

/// MFJ profile (no QD in these KATs).
fn mfj_profile(ord: Usd, magi: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Mfj,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
    }
}

// ── LedgerState / Disposal factories ──────────────────────────────────────────────────────────────

fn test_eid(n: u64) -> EventId {
    EventId::import(Source::Coinbase, SourceRef::new(format!("kat-{n}")))
}

fn test_lot(n: u64) -> LotId {
    LotId {
        origin_event_id: test_eid(n),
        split_sequence: 0,
    }
}

/// Minimal `DisposalLeg` with the given signed gain and term.
/// `compute_tax_year` reads only `leg.gain` and `leg.term`; the other fields are synthetic but
/// internally consistent (proceeds = max(gain,0), basis = proceeds − gain).
fn leg(gain: Usd, term: Term) -> DisposalLeg {
    let proceeds = if gain >= dec!(0) { gain } else { dec!(0) };
    let basis = proceeds - gain; // ≥ 0; = −gain when gain < 0 (a loss leg)
    DisposalLeg {
        lot_id: test_lot(1),
        sat: 1,
        proceeds,
        basis,
        gain,
        term,
        basis_source: BasisSource::ExchangeProvided,
        gift_zone: None,
    }
}

/// `LedgerState` with one disposal on 2025-06-15 carrying the supplied legs.
fn state_with_legs(legs: Vec<DisposalLeg>) -> LedgerState {
    LedgerState {
        disposals: vec![Disposal {
            event: test_eid(0),
            kind: DisposeKind::Sell,
            disposed_at: date!(2025 - 06 - 15),
            legs,
            fee_mini_disposition: false,
        }],
        ..LedgerState::default()
    }
}

fn state_lt(gain: Usd) -> LedgerState {
    state_with_legs(vec![leg(gain, Term::LongTerm)])
}

fn state_st(gain: Usd) -> LedgerState {
    state_with_legs(vec![leg(gain, Term::ShortTerm)])
}

fn state_st_lt(st: Usd, lt: Usd) -> LedgerState {
    state_with_legs(vec![leg(st, Term::ShortTerm), leg(lt, Term::LongTerm)])
}

// ── Computation helpers ───────────────────────────────────────────────────────────────────────────

fn computed(state: LedgerState, profile: TaxProfile) -> TaxResult {
    match compute_tax_year(&[], &state, 2025, Some(&profile), &BundledTaxTables::load()) {
        TaxOutcome::Computed(r) => r,
        TaxOutcome::NotComputable(b) => panic!("unexpected not-computable: {:?}", b),
    }
}

fn computed_mfj(state: LedgerState, oti: Usd, magi: Usd) -> TaxResult {
    computed(state, mfj_profile(oti, magi))
}

// ── KAT 1 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn single_lt_crosses_0_to_15() {
    // Hand-derivation (Rev. Proc. 2024-40 §2.03 — Single §1(h) breakpoints).
    //   TY2025 Single: max_zero = 48,350; max_fifteen = 533,400.
    //   OTI = 40,000; crypto LT gain = 20,000 → pref stack: bottom=40,000, top=60,000.
    //     at_0  = 48,350 − 40,000 = 8,350
    //     at_15 = 60,000 − 48,350 = 11,650
    //     ltcg_tax = 11,650 × 0.15 = 1,747.50
    //   NIIT (§1411): magi_with = magi_excl(60,000) + crypto_agi(20,000) = 80,000 < 200,000 → 0.
    //   Ordinary delta: OTI unchanged by LT gain → ord_delta = 0.
    //   total = 0 + 1,747.50 + 0 = 1,747.50.
    //   Marginal LTCG rate: top=60,000 ∈ (48,350, 533,400] → 15%.
    let r = computed(
        state_lt(dec!(20000)),
        single(dec!(40000), dec!(60000), dec!(0)),
    );
    assert_eq!(r.ltcg_tax, dec!(1747.50));
    assert_eq!(r.total_federal_tax_attributable, dec!(1747.50));
    assert_eq!(r.marginal_rates.ltcg, dec!(0.15));
}

// ── KAT 2 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn single_lt_crosses_15_to_20() {
    // Hand-derivation.
    //   TY2025 Single: max_zero = 48,350; max_fifteen = 533,400.
    //   OTI = 500,000; crypto LT gain = 100,000 → bottom=500,000, top=600,000.
    //     at_0  = max(48,350 − 500,000, 0) = 0
    //     at_15 = 533,400 − 500,000 = 33,400
    //     at_20 = 100,000 − 33,400 = 66,600
    //     ltcg_tax = 33,400 × 0.15 + 66,600 × 0.20 = 5,010 + 13,320 = 18,330.00
    //   NIIT (§1411, §1.1411-10): nii_with = 100,000; magi_with = 500,000 + 100,000 = 600,000.
    //     over   = 600,000 − 200,000 = 400,000
    //     base   = min(100,000, 400,000) = 100,000
    //     niit_with = 100,000 × 0.038 = 3,800.00
    //   niit_without: nii_without = 0; magi_without = 500,000 > 200,000 but nii=0 → 0.
    //   niit_delta = 3,800.00.
    //   total = 0 + 18,330.00 + 3,800.00 = 22,130.00.
    //   Marginal LTCG rate: top = 600,000 > 533,400 → 20%.
    let r = computed(
        state_lt(dec!(100000)),
        single(dec!(500000), dec!(500000), dec!(0)),
    );
    assert_eq!(r.ltcg_tax, dec!(18330.00));
    assert_eq!(r.niit, dec!(3800.00));
    assert_eq!(
        r.total_federal_tax_attributable,
        dec!(18330.00) + dec!(3800.00)
    );
    assert_eq!(r.marginal_rates.ltcg, dec!(0.20));
}

// ── KAT 3 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn single_qd_pushes_crypto_lt_from_15_to_20() {
    // I9: QD shares the §1(h) 0/15/20 stack with crypto LT.
    // Hand-derivation.
    //   TY2025 Single: max_zero = 48,350; max_fifteen = 533,400.
    //   OTI = 450,000; QD = 80,000; crypto LT = 20,000.
    //   WITHOUT crypto (LT=0): pref = 80,000, top = 530,000 < 533,400 → 80,000 @ 15% = 12,000.
    //   WITH crypto    (LT=20,000): pref = 100,000, top = 550,000 > 533,400.
    //     at_0  = 0 (bottom=450,000 > max_zero)
    //     at_15 = 533,400 − 450,000 = 83,400
    //     at_20 = 100,000 − 83,400 = 16,600
    //     pref_with = 83,400 × 0.15 + 16,600 × 0.20 = 12,510 + 3,320 = 15,830
    //   ltcg_tax (delta) = 15,830 − 12,000 = 3,830.00
    //     (Without QD sharing: 20,000 @ 15% = 3,000; QD pushed 16,600 into the 20% band → 830 extra.)
    //
    // B-M3 note: magi_excluding_crypto = 530,000 = OTI(450,000) + QD(80,000); this is internally
    // consistent per the B.1 contract (MAGI already includes QD; B only adds the crypto delta).
    // ltcg_tax is computed from the §1(h) stack and is independent of MAGI; the assertion stands.
    let r = computed(
        state_lt(dec!(20000)),
        single(dec!(450000), dec!(530000), dec!(80000)),
    );
    assert_eq!(r.ltcg_tax, dec!(3830.00));
}

// ── KAT 4 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn mfj_st_gain_stacks_on_ordinary() {
    // ST gain treated as ordinary (§1222); stacks on top of OTI.
    // Hand-derivation (Rev. Proc. 2024-40 §2.01 — MFJ rate schedule).
    //   TY2025 MFJ brackets: 10% [0, 23,850); 12% [23,850, 96,950); 22% [96,950, 206,700); …
    //   OTI = 90,000; crypto ST gain = 20,000 → bottom_with = 90,000 + 20,000 = 110,000.
    //
    //   tax(110,000 MFJ):
    //     10% × 23,850              = 2,385.00
    //     12% × (96,950 − 23,850)  = 12% × 73,100 = 8,772.00
    //     22% × (110,000 − 96,950) = 22% × 13,050 = 2,871.00
    //     total = 14,028.00
    //
    //   tax(90,000 MFJ):
    //     10% × 23,850             = 2,385.00
    //     12% × (90,000 − 23,850) = 12% × 66,150 = 7,938.00
    //     total = 10,323.00
    //
    //   ordinary_delta = 14,028 − 10,323 = 3,705.00
    //   NIIT: magi_with = 90,000 + 20,000 = 110,000 < 250,000 (MFJ threshold) → 0.
    //   total = 3,705.00 + 0 + 0 = 3,705.00.
    //   st_net = crypto_st − cf_short = 20,000 − 0 = 20,000.
    //   Marginal ordinary rate: 110,000 ∈ (96,950, 206,700) → 22%.
    let r = computed_mfj(state_st(dec!(20000)), dec!(90000), dec!(90000));
    assert_eq!(r.st_net, dec!(20000));
    assert_eq!(r.total_federal_tax_attributable, dec!(3705.00));
    assert_eq!(r.marginal_rates.ordinary, dec!(0.22));
}

// ── KAT 5 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn single_3k_loss_limit_and_multiyear_st_first_carryforward() {
    // §1222 netting + §1211(b) $3k limit + §1212(b) ST-first character-preserving carryforward.
    //
    // Year 2025 — both characters are losses.
    //   crypto ST = −5,000; crypto LT = −2,000.
    //   §1222 within-char: st_net = −5,000; lt_net = −2,000 (both losses → no cross-net).
    //   net_loss = 7,000 → loss_deduction = min(7,000, 3,000) = 3,000 (§1211(b) $3k cap).
    //   ST-first absorption (§1212(b)(2)): absorbed_st = min(5,000, 3,000) = 3,000; absorbed_lt = 0.
    //   st_carry = 5,000 − 3,000 = 2,000;  lt_carry = 2,000 − 0 = 2,000.
    let r25 = computed(
        state_st_lt(dec!(-5000), dec!(-2000)),
        single(dec!(60000), dec!(60000), dec!(0)),
    );
    assert_eq!(r25.loss_deduction, dec!(3000));
    assert_eq!(
        r25.carryforward_out,
        Carryforward {
            short: dec!(2000),
            long: dec!(2000)
        }
    );

    // Multi-year chain: feed carryforward_out into the next year's carryforward_in.
    // Model as a second 2025 run with the carried losses in the profile.
    //   carryforward_in = {short:2,000, long:2,000}; crypto LT gain = 10,000.
    //   §1222 WITH crypto: st_net = 0 − 2,000 = −2,000; lt_net = 10,000 − 2,000 = 8,000.
    //   Cross-net: ST loss (2,000) offsets LT gain → preferential_gain = 6,000; no loss.
    //   loss_deduction = 0; carryforward_out = {short:0, long:0}.
    let profile_y2 = TaxProfile {
        capital_loss_carryforward_in: r25.carryforward_out,
        ..single(dec!(60000), dec!(60000), dec!(0))
    };
    let r_y2 = match compute_tax_year(
        &[],
        &state_lt(dec!(10000)),
        2025,
        Some(&profile_y2),
        &BundledTaxTables::load(),
    ) {
        TaxOutcome::Computed(r) => r,
        TaxOutcome::NotComputable(b) => panic!("unexpected not-computable in y2: {:?}", b),
    };
    // lt_net is the within-character net BEFORE cross-net: 10,000 − cf_long(2,000) = 8,000.
    assert_eq!(r_y2.lt_net, dec!(8000));
    assert_eq!(r_y2.loss_deduction, dec!(0));
    assert_eq!(r_y2.carryforward_out, Carryforward::default());
}

// ── KAT 6 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn refusal_and_missing_table_end_to_end() {
    // B.4 / I6: when no bundled table exists for the requested year, compute_tax_year returns
    // NotComputable(TaxTableMissing) — not a panic, not a wrong number.
    // The Hard-blocker gate passes first (state has no blockers); then the table lookup fails.
    let tables = BundledTaxTables::load();
    let st = LedgerState::default(); // no disposals, no blockers
    let outcome = compute_tax_year(
        &[],
        &st,
        2099,
        Some(&single(dec!(1), dec!(1), dec!(0))),
        &tables,
    );
    assert!(matches!(
        &outcome,
        TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxTableMissing
    ));
}
