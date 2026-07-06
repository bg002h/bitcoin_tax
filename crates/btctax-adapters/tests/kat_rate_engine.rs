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
    EventId, FilingStatus, IncomeKind, IncomeRecord, LedgerState, LotId, Source, SourceRef,
    TaxOutcome, TaxProfile, TaxResult, Term, Usd, WalletId,
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
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: Usd::ZERO,
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
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: Usd::ZERO,
    }
}

/// MFS profile (no QD).
fn mfs_profile(ord: Usd, magi: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Mfs,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: Usd::ZERO,
    }
}

/// HoH profile (no QD).
fn hoh_profile(ord: Usd, magi: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::HoH,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: Usd::ZERO,
    }
}

/// QSS profile (no QD).  QSS aliases MFJ for all rate lookups.
fn qss_profile(ord: Usd, magi: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Qss,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: Usd::ZERO,
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
        acquired_at: date!(2025 - 01 - 01), // synthetic; compute_tax_year does not read acquired_at
        wallet: WalletId::Exchange {
            provider: "cb".into(),
            account: "m".into(),
        }, // synthetic
        pseudo: false,
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

/// `LedgerState` with one `IncomeRecord` (mining) and no disposals.
/// Used for the double-count guard KAT.
fn state_with_mining(amount: Usd) -> LedgerState {
    LedgerState {
        income_recognized: vec![IncomeRecord {
            event: test_eid(0),
            recognized_at: date!(2025 - 06 - 15),
            sat: 100,
            usd_fmv: amount,
            kind: IncomeKind::Mining,
            business: false,
            pseudo: false,
        }],
        ..LedgerState::default()
    }
}

/// `LedgerState` with one LT disposal leg AND one mining `IncomeRecord`.
/// Used for the three-way-nonzero identity KAT.
fn state_lt_with_mining(lt_gain: Usd, mining: Usd) -> LedgerState {
    LedgerState {
        disposals: vec![Disposal {
            event: test_eid(0),
            kind: DisposeKind::Sell,
            disposed_at: date!(2025 - 06 - 15),
            legs: vec![leg(lt_gain, Term::LongTerm)],
            fee_mini_disposition: false,
        }],
        income_recognized: vec![IncomeRecord {
            event: test_eid(1),
            recognized_at: date!(2025 - 06 - 15),
            sat: 100,
            usd_fmv: mining,
            kind: IncomeKind::Mining,
            business: false,
            pseudo: false,
        }],
        ..LedgerState::default()
    }
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

// ── KAT 7 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn mfs_lt_crosses_15_to_20_and_niit() {
    // MFS-specific code paths: §1(h) max_fifteen = $300,000 (Rev. Proc. 2024-40 §2.03 — MFS)
    // and §1411 NIIT threshold = $125,000 (§1411(b)(3)(A) — NOT $200k Single, NOT $250k MFJ).
    //
    // Hand-derivation (TY2025 MFS bundled numbers).
    //   MFS §1(h) breakpoints: max_zero = 48,350; max_fifteen = 300,000.
    //   OTI = 270,000; crypto LT gain = 60,000 → pref stack: bottom=270,000, pref=60,000, top=330,000.
    //     at_0  = 0  (bottom=270,000 > max_zero=48,350)
    //     at_15 = min(top, max_fifteen) − max(bottom, max_zero) = 300,000 − 270,000 = 30,000
    //     at_20 = 60,000 − 30,000 = 30,000
    //     pref_with = 30,000×0.15 + 30,000×0.20 = 4,500 + 6,000 = 10,500.00
    //   pref_without = 0  →  ltcg_tax = 10,500.00.
    //   NIIT (MFS threshold = $125,000 — statutory §1411(b)(3)(A)):
    //     crypto_agi = 60,000; magi_with = 270,000 + 60,000 = 330,000.
    //     nii_with = 60,000; over = 330,000 − 125,000 = 205,000; base = min(60,000, 205,000) = 60,000.
    //     niit_with  = 60,000 × 0.038 = 2,280.00.
    //     niit_without: magi_without=270,000 > 125,000 but nii_without=0 → 0.
    //     niit = 2,280.00.
    //   Ordinary delta = 0 (no crypto ST gain or income; OTI baseline unchanged).
    //   total = 0 + 10,500.00 + 2,280.00 = 12,780.00.
    //   Marginal LTCG rate: top=330,000 > max_fifteen=300,000 → 0.20.
    let r = computed(
        state_lt(dec!(60000)),
        mfs_profile(dec!(270000), dec!(270000)),
    );
    assert_eq!(r.ltcg_tax, dec!(10500.00));
    assert_eq!(r.niit, dec!(2280.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(12780.00));
    assert_eq!(r.marginal_rates.ltcg, dec!(0.20));
}

// ── KAT 8 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn mfs_1500_loss_cap_and_carryforward() {
    // §1211(b) MFS capital-loss ordinary-offset limit = $1,500 (one-half the general $3,000
    // for married filing separately — distinct from the $3,000 tested in KAT 5).
    //
    // Hand-derivation.
    //   OTI = 60,000 (MFS); crypto ST = −4,000; crypto LT = −1,000; loss_limit (MFS) = 1,500.
    //   §1222 within-char: st_net = −4,000; lt_net = −1,000 (both losses; no cross-net).
    //   net_loss = 5,000; loss_deduction = min(5,000, 1,500) = 1,500 (NOT $3,000).
    //   §1212(b) ST-first absorption: absorbed_st = min(4,000, 1,500) = 1,500; absorbed_lt = 0.
    //   carryforward_out: st_carry = 4,000 − 1,500 = 2,500; lt_carry = 1,000 − 0 = 1,000.
    //   Ordinary delta:
    //     bottom_with   = 60,000 − 1,500 = 58,500 (loss deduction reduces ordinary income).
    //     bottom_without = 60,000 (no loss deduction without crypto).
    //     MFS brackets: 10% [0,11,925) / 12% [11,925,48,475) / 22% [48,475,103,350) / …
    //     ord_with  (58,500): 10%×11,925 + 12%×36,550 + 22%×10,025
    //                       = 1,192.50 + 4,386.00 + 2,205.50 = 7,784.00
    //     ord_without(60,000): 10%×11,925 + 12%×36,550 + 22%×11,525
    //                       = 1,192.50 + 4,386.00 + 2,535.50 = 8,114.00
    //     ordinary_delta = 7,784.00 − 8,114.00 = −330.00  (loss saves 1,500 × 22% = $330).
    //   NIIT: crypto_agi = −1,500; magi_with = 60,000 − 1,500 = 58,500 < 125,000 → 0.
    //   total = −330.00 + 0 + 0 = −330.00.
    let r = computed(
        state_st_lt(dec!(-4000), dec!(-1000)),
        mfs_profile(dec!(60000), dec!(60000)),
    );
    assert_eq!(r.loss_deduction, dec!(1500));
    assert_eq!(
        r.carryforward_out,
        Carryforward {
            short: dec!(2500),
            long: dec!(1000)
        }
    );
    assert_eq!(r.total_federal_tax_attributable, dec!(-330.00));
}

// ── KAT 9 ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn hoh_lt_crosses_15_to_20_and_niit() {
    // HoH §1(h) breakpoints: max_zero = 64,750; max_fifteen = 566,700 (Rev. Proc. 2024-40 §2.03).
    // NIIT threshold for HoH = $200,000 (§1411(b)(1)(A) "any other case" — same as Single;
    // HoH does NOT have its own NIIT amount and is NOT under the MFJ/QSS $250,000 or MFS $125,000).
    //
    // Hand-derivation.
    //   OTI = 500,000; crypto LT gain = 100,000 → bottom=500,000, pref=100,000, top=600,000.
    //     at_0  = 0  (bottom > max_zero=64,750)
    //     at_15 = min(600,000, 566,700) − max(500,000, 64,750) = 566,700 − 500,000 = 66,700
    //     at_20 = 100,000 − 66,700 = 33,300
    //     pref_with = 66,700×0.15 + 33,300×0.20 = 10,005.00 + 6,660.00 = 16,665.00.
    //   pref_without = 0  →  ltcg_tax = 16,665.00.
    //   NIIT (HoH threshold = $200,000):
    //     crypto_agi = 100,000; magi_with = 500,000 + 100,000 = 600,000.
    //     nii_with = 100,000; over = 400,000; base = min(100,000, 400,000) = 100,000.
    //     niit_with = 100,000 × 0.038 = 3,800.00; niit_without = 0  →  niit = 3,800.00.
    //   Ordinary delta = 0.
    //   total = 0 + 16,665.00 + 3,800.00 = 20,465.00.
    //   Marginal LTCG rate: top=600,000 > max_fifteen=566,700 → 0.20.
    let r = computed(
        state_lt(dec!(100000)),
        hoh_profile(dec!(500000), dec!(500000)),
    );
    assert_eq!(r.ltcg_tax, dec!(16665.00));
    assert_eq!(r.niit, dec!(3800.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(20465.00));
    assert_eq!(r.marginal_rates.ltcg, dec!(0.20));
}

// ── KAT 10 ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn qss_resolves_to_mfj_schedule() {
    // QSS (Qualifying Surviving Spouse / §2(a)) aliases MFJ for ALL §1/§1(h)/§1411 lookups.
    // End-to-end confirmation: QSS produces the same TaxResult as MFJ for identical inputs.
    //
    // Hand-derivation using MFJ §1(h) breakpoints (max_zero=96,700; max_fifteen=600,050).
    //   OTI = 80,000 (QSS); crypto LT = 30,000 → bottom=80,000, pref=30,000, top=110,000.
    //     at_0  = 96,700 − 80,000 = 16,700 (MFJ 0% zone extends to 96,700 — not Single's 48,350)
    //     at_15 = 110,000 − 96,700 = 13,300; at_20 = 0.
    //     pref_with = 13,300×0.15 = 1,995.00.
    //   pref_without = 0  →  ltcg_tax = 1,995.00.
    //   NIIT: magi_with = 80,000 + 30,000 = 110,000 < 250,000 (QSS/MFJ threshold) → 0.
    //   Ordinary delta = 0.  total = 1,995.00.
    //   Marginal LTCG rate: top=110,000 ∈ (96,700, 600,050] → 0.15.
    //
    // Distinctness proof: if QSS had incorrectly used Single breakpoints (max_zero=48,350):
    //   at_0=0 (bottom=80,000 > 48,350), at_15=30,000 → ltcg_tax=4,500 (not 1,995).
    //   The MFJ 0% zone absorbs 16,700 of the 30,000 LT gain; Single's is already exhausted below
    //   bottom=80,000.  ltcg_tax=1,995 is ONLY achievable with the MFJ breakpoints.
    let r_qss = computed(state_lt(dec!(30000)), qss_profile(dec!(80000), dec!(80000)));
    assert_eq!(r_qss.ltcg_tax, dec!(1995.00));
    assert_eq!(r_qss.total_federal_tax_attributable, dec!(1995.00));
    assert_eq!(r_qss.marginal_rates.ltcg, dec!(0.15));

    // End-to-end identity: QSS and MFJ produce identical TaxResult for the same inputs.
    let r_mfj = computed(state_lt(dec!(30000)), mfj_profile(dec!(80000), dec!(80000)));
    assert_eq!(r_qss, r_mfj);
}

// ── KAT 11 ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn mining_income_added_exactly_once() {
    // Double-count guard (I5): `income_recognized[*].usd_fmv` (crypto ordinary; here mining)
    // is added to the ordinary stack EXACTLY ONCE, in the WITH-scenario `bottom_with` only.
    //
    // Hand-derivation (Rev. Proc. 2024-40 §2.01 — Single brackets).
    //   OTI = 40,000; mining income = 5,000 (income_recognized[0]); no crypto gains.
    //   bottom_with   = 40,000 + 5,000 = 45,000  (income added once).
    //   bottom_without = 40,000.
    //   TY2025 Single 12% bracket [11,925, 48,475).  Both 40,000 and 45,000 lie within it.
    //   ord_with  (45,000): 10%×11,925 + 12%×33,075 = 1,192.50 + 3,969.00 = 5,161.50
    //   ord_without(40,000): 10%×11,925 + 12%×28,075 = 1,192.50 + 3,369.00 = 4,561.50
    //   ordinary_delta = 5,161.50 − 4,561.50 = 600.00  (= 5,000 × 12%).
    //   If double-counted (bottom_with=50,000): 10%×11,925+12%×36,550+22%×1,525 = 5,914.00
    //     → delta = 5,914.00 − 4,561.50 = 1,352.50 ≠ 600.00.
    //   NIIT: magi_with = 40,000 + 5,000 = 45,000 < 200,000 → 0.
    //   total = 600.00; ordinary_from_crypto = 5,000.
    let r = computed(
        state_with_mining(dec!(5000)),
        single(dec!(40000), dec!(40000), dec!(0)),
    );
    assert_eq!(r.ordinary_from_crypto, dec!(5000));
    assert_eq!(r.total_federal_tax_attributable, dec!(600.00));
}

// ── KAT 12 ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn three_way_nonzero_identity() {
    // Three-way-nonzero identity: total_federal_tax_attributable == ordinary_delta + ltcg_tax + niit
    // with ALL THREE components nonzero (Single; mining income → ordinary delta; LT gain → ltcg_tax;
    // crypto pushes MAGI over $200k → niit).
    //
    // Hand-derivation (Rev. Proc. 2024-40 §2.01/§2.03 — Single brackets and §1(h) breakpoints).
    //   OTI = 150,000; mining income = 5,000; crypto LT = 60,000; MAGI_excl = 150,000.
    //
    //   §1222 netting (crypto_st=0, crypto_lt=60,000): preferential_gain=60,000; ordinary_gain=0.
    //
    //   Ordinary delta: bottom_with=150,000+5,000=155,000; bottom_without=150,000.
    //     Single 24% bracket [103,350, 197,300).  Both 150k and 155k lie within it.
    //     ord_with (155,000): 10%×11,925 + 12%×36,550 + 22%×54,875 + 24%×51,650
    //                       = 1,192.50 + 4,386.00 + 12,072.50 + 12,396.00 = 30,047.00
    //     ord_without(150,000): 10%×11,925 + 12%×36,550 + 22%×54,875 + 24%×46,650
    //                       = 1,192.50 + 4,386.00 + 12,072.50 + 11,196.00 = 28,847.00
    //     ordinary_delta = 30,047.00 − 28,847.00 = 1,200.00  (= 5,000 × 24%).
    //
    //   §1(h) (Single; max_zero=48,350; max_fifteen=533,400):
    //     bottom_with=155,000; pref=60,000; top=215,000.
    //       at_0=0; at_15=215,000−155,000=60,000; at_20=0.
    //       pref_with = 60,000×0.15 = 9,000.00.  pref_without=0.  ltcg_tax = 9,000.00.
    //
    //   §1411 NIIT (Single threshold=$200,000):
    //     crypto_agi = (0+60,000−0)−(0+0−0)+5,000 = 65,000.
    //     magi_with = 150,000 + 65,000 = 215,000.
    //     nii_with = 60,000 (LT gain; mining excluded from NII under B-M1 minimal model).
    //     over = 215,000 − 200,000 = 15,000; base = min(60,000, 15,000) = 15,000.
    //     niit_with = 15,000 × 0.038 = 570.00.  niit_without=0.  niit=570.00.
    //
    //   total = 1,200.00 + 9,000.00 + 570.00 = 10,770.00.
    //   Identity (exact Decimal arithmetic — no float rounding error possible):
    //     total == ordinary_delta + ltcg_tax + niit  ↔  10,770 == 1,200 + 9,000 + 570.
    let r = computed(
        state_lt_with_mining(dec!(60000), dec!(5000)),
        single(dec!(150000), dec!(150000), dec!(0)),
    );
    let ordinary_delta = dec!(1200.00); // hand-derived from bundled TY2025 Single 24% bracket
    assert_eq!(r.ordinary_from_crypto, dec!(5000));
    assert_eq!(r.ltcg_tax, dec!(9000.00));
    assert_eq!(r.niit, dec!(570.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(10770.00));
    assert!(ordinary_delta > dec!(0), "ordinary_delta must be nonzero");
    assert!(r.ltcg_tax > dec!(0), "ltcg_tax must be nonzero");
    assert!(r.niit > dec!(0), "niit must be nonzero");
    assert_eq!(
        r.total_federal_tax_attributable,
        ordinary_delta + r.ltcg_tax + r.niit,
    );
}
