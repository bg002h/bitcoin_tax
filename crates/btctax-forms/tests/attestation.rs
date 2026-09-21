//! **P9 — attestation that has to read the PAPER.**
//!
//! The core half of P9 is in `crates/btctax-core/tests/kat_attestation.rs`. These four are here
//! because `PrintedForms` is not paper:
//!
//!   * **§G-28 — a value-check cannot see a missing form.** "This filer's packet contains no
//!     Schedule A and no Form 8283" is not an assertion about any number; it is an assertion about
//!     the SET of forms, and only the filled packet has one.
//!   * **A printed struct is not a printed cell.** The §1211(b) surface is three different sign
//!     conventions on three lines — a leading minus on 1040 line 7, a parenthesised magnitude on
//!     Schedule D line 21 — and the struct carries none of that.
//!   * **The all-zero return's whole content is which cells are BLANK**, which no struct can say.
//!
//! Every KAT here was watched RED against a planted defect; the plant is named in its doc comment.

mod common;

use btctax_core::conventions::Usd;
use btctax_core::state::LedgerState;
use btctax_core::tax::packet::assemble_printed_return;
use btctax_core::tax::return_1040::{assemble_absolute, screen_absolute, AbsoluteReturn};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::return_refuse::screen_inputs;
use btctax_core::tax::testonly::{
    amt_owing_household, answer_all_live_declarations, build_golden_return, kitchen_sink_header,
    ty2024_params, ty2024_table, ty2025_table, GoldenInputs,
};
use btctax_forms::testonly::*;
use btctax_forms::{fill_full_return, NamedForm};
use common::{on_paper_signed, Sign};
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// Assembling an arbitrary household to PAPER.
//
// `common::full_return` takes a `GoldenHousehold` (a corpus row with baked oracle answers). These KATs
// need households the corpus does not carry — a crypto donor, an all-zero filer — so they take the
// same three steps by hand: `assemble_absolute` → `assemble_printed_return` → `fill_full_return`.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

struct Filed {
    ar: AbsoluteReturn,
    forms: Vec<NamedForm>,
}

/// Assemble and FILE a household, asserting it does not refuse at either screen.
///
/// ★ Both screens run. After phase 2 that is load-bearing rather than belt-and-braces: `screen_inputs`
/// carries the always-live P7 Form 4952 declaration, and `screen_absolute` carries P4's §170(f)(8)
/// acknowledgment gate. A fixture that quietly refused would produce no packet at all, and a test that
/// unwrapped its way past that would be asserting about a return nobody could file.
fn file(ri: &ReturnInputs, state: &LedgerState) -> Filed {
    file_for_year(ri, state, 2024)
}

/// The same, for a named year. ★ Separate rather than a default argument because the year decides the
/// FORM REVISION, and a KAT about a revision-specific cell must name the revision it is about.
fn file_for_year(ri: &ReturnInputs, state: &LedgerState, year: i32) -> Filed {
    // ★ `ty2024_params()` for BOTH years, following `kat_broker_reporting.rs`: only TY2024 has a
    //   bundled `FullReturnParams` (`full_return_for(2025)` is `None` — see `xtask blockers`), and the
    //   YEAR argument is what selects the form revision, which is what these KATs are about. The §86(c)
    //   thresholds this test depends on are statutory and NOT indexed, so they come from
    //   `ss_benefits_worksheet::line8_base` rather than from params at all.
    let (params, table) = match year {
        2024 => (ty2024_params(), ty2024_table()),
        2025 => (ty2024_params(), ty2025_table()),
        y => panic!("this harness bundles TY2024 and TY2025 only, not TY{y}"),
    };
    assert!(
        screen_inputs(ri, &table, &params).is_none(),
        "this fixture must FILE, but screen_inputs refused: {:?}",
        screen_inputs(ri, &table, &params).map(|r| r.reason)
    );
    let ar = assemble_absolute(ri, state, &params, &table, year);
    assert!(
        screen_absolute(
            ri,
            &ar,
            &params,
            state,
            year,
            btctax_core::InformationReturnRegime::NONE
        )
        .is_none(),
        "this fixture must FILE, but screen_absolute refused: {:?}",
        screen_absolute(
            ri,
            &ar,
            &params,
            state,
            year,
            btctax_core::InformationReturnRegime::NONE
        )
        .map(|r| r.reason)
    );
    let pr = assemble_printed_return(
        ri,
        state,
        &BTreeMap::new(),
        &ar,
        &table,
        year,
        &[],
        btctax_core::InformationReturnRegime::NONE,
    )
    .expect("the fixture carries a well-formed SSN");
    let packet = fill_full_return(&pr, year).expect("the packet must fill");
    Filed {
        ar,
        forms: packet.forms,
    }
}

/// A `GoldenInputs` with every money axis at zero — the all-zero return, and the base every other
/// household here is one field away from.
///
/// ★ Written as a literal rather than `..Default::default()` so that a NEW income axis added to
/// `GoldenInputs` fails to compile here (`E0063`) instead of defaulting itself into every fixture
/// below. That blast radius is the review: a household these KATs believe has no self-employment
/// income must say so, not inherit it.
fn zero_inputs(filing_status: &str) -> GoldenInputs {
    GoldenInputs {
        filing_status: filing_status.into(),
        w2_income: 0.0,
        taxable_interest: 0.0,
        qualified_dividends: 0.0,
        ordinary_dividends: 0.0,
        short_term_capital_gains: 0.0,
        long_term_capital_gains: 0.0,
        self_employment_income: 0.0,
        itemized_deductions: 0.0,
        state_income_tax: 0.0,
        real_estate_tax: 0.0,
        mortgage_interest: 0.0,
        charitable_cash: 0.0,
        // ★ T11 fold — the STANDARD deduction, said rather than inherited: these households carry
        //   no Schedule A at all, so 1040 line 12 can only be §63(c)'s figure.
        standard_or_itemized: btctax_core::tax::testonly::GoldenDeduction::Standard,
        // ★ T16 — no HSA on these households, said rather than inherited.
        hsa_deduction: 0.0,
        // ★ T11 — no unemployment, and no DEPENDENTS BLOCK. Said rather than inherited, for the
        //   reason the whole function exists: these KATs assert what a childless household's 1040
        //   line 19 prints, and a dependent that arrived by `..Default::default()` would change the
        //   answer they are pinning. The rows each test wants are added by `with_dependents`, which
        //   builds them on the RETURN rather than on the oracle row.
        unemployment: 0.0,
        dependents: Vec::new(),
        // The FR-29 adult sentinel, restated: `build_golden_return`'s taxpayer has always been 44,
        // and these fixtures inherit nothing.
        age_head: Some(btctax_core::tax::testonly::GOLDEN_ADULT_AGE),
        age_spouse: None,
        blind_head: false,
        blind_spouse: false,
    }
}

fn form_names(forms: &[NamedForm]) -> BTreeSet<&str> {
    forms.iter().map(|f| f.name.as_str()).collect()
}

fn cells(forms: &[NamedForm], name: &str, map: &str) -> BTreeMap<String, String> {
    let f = forms
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("the packet is missing {name}"));
    extract_lines(&f.bytes, map).expect("the filled form transcribes")
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 7 — §1211(b) and §1212(b), read off the PDF.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **THE LOSS YEAR'S PRINTED SURFACE, IN ITS THREE SIGN CONVENTIONS.**
///
/// §1211(b) lets an individual deduct a capital loss against ordinary income only up to $3,000, and
/// §1212(b) carries the rest forward. Three lines say so on paper, and each says it differently:
///
///   * **1040 line 7** carries a LEADING MINUS — `-3000`. The cell pre-prints no parentheses.
///   * **Schedule D line 21** is a PRE-PRINTED PARENTHESISED box, so it holds the positive magnitude
///     `3000` and the parentheses supply the sign. Writing `-3000` there would render `(-3000)`,
///     which reads as a POSITIVE $3,000 on a return signed under 26 USC §6065.
///   * **Schedule D line 16** carries the whole net loss, `-20000`, with a leading minus.
///
/// The corpus witnesses the loss-year arithmetic through both oracles. No in-repo KAT had ever read
/// these three cells back OFF THE PDF, which is where the sign conventions live.
///
/// ★★ **AND THE PAIR IS ASSERTED TOGETHER, ON PURPOSE.** At positive taxable income the carryforward
/// is $17,000 (the $3,000 was actually absorbed); at taxable income on the floor it is the full
/// $20,000 (none of it was). The §1212(b)(2)(B) worksheet N1 landed produces both, and the flat rule
/// it replaced produced $17,000 for both. Pinning only the floor case would let a "fix" overshoot in
/// the other direction and stay green.
#[test]
fn the_1211b_cap_and_the_1212b_carryforward_print_and_pair() {
    // ── L3: positive taxable income. The $3,000 IS absorbed, so $17,000 carries. ────────────────────
    let mut i = zero_inputs("Single");
    i.w2_income = 40_000.0;
    i.long_term_capital_gains = -20_000.0;
    let (ri, state) = build_golden_return(&i);
    let filed = file(&ri, &state);
    let f1040 = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    let schd = cells(
        &filed.forms,
        "schedule_d",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::ScheduleD, 2024).unwrap(),
    );

    assert_eq!(
        on_paper_signed(&f1040, "line7a", Sign::Leading),
        Some(-3000),
        "1040 line 7 is the §1211(b)-limited loss, with a LEADING MINUS — the cell pre-prints no \
         parentheses, so the minus has to be in the digits"
    );
    assert_eq!(
        f1040.get("line7a").map(String::as_str),
        Some("-3000"),
        "…and literally so on the paper: the magnitude alone would read as a $3,000 GAIN"
    );
    // ★ The map keys these two cells `line15_h` / `line16_h` — the `_h` is the form's COLUMN (h),
    //   "Gain or (loss)". Lines 15 and 16 have only that column, but the map names it anyway, and a
    //   test that guessed `line16` reads `None` and would have passed vacuously under a laxer
    //   assertion. `on_paper_signed` returning `None` for an absent key is why this one did not.
    assert_eq!(
        on_paper_signed(&schd, "line16_h", Sign::Leading),
        Some(-20000),
        "Schedule D line 16 column (h) carries the WHOLE net loss, not the §1211(b)-capped slice"
    );
    assert_eq!(
        on_paper_signed(&schd, "line15_h", Sign::Leading),
        Some(-20000),
        "…and line 15, the long-term subtotal it comes from, carries it too — this loss is entirely \
         long-term, so 15 and 16 coincide and line 7 (short-term) is blank"
    );
    assert_eq!(
        schd.get("line21").map(String::as_str),
        Some("3000"),
        "Schedule D line 21 is a PRE-PRINTED PARENTHESISED box: it takes the positive magnitude and \
         the parentheses supply the sign. `-3000` here renders `(-3000)` — a positive $3,000 on a \
         form signed under penalties of perjury."
    );
    assert_eq!(
        on_paper_signed(&schd, "line21", Sign::ParenMagnitude),
        Some(-3000),
        "…which READS as −3,000 under the paren convention (§6.3)"
    );

    assert_eq!(
        filed.ar.capital_loss_carryforward_out.long,
        dec!(17000),
        "§1212(b): at POSITIVE taxable income the whole $3,000 allowance was absorbed, so exactly \
         $17,000 of the $20,000 loss survives to next year"
    );
    assert!(
        filed.ar.taxable_income > Usd::ZERO,
        "the $17,000 figure is only correct while taxable income is positive — that is the branch \
         where the flat `min(loss, 3000)` rule and the §1212(b)(2)(B) worksheet agree"
    );

    // ── L4: taxable income AT THE FLOOR. None of the $3,000 is absorbed, so all $20,000 carries. ────
    let mut i = zero_inputs("Single");
    i.long_term_capital_gains = -20_000.0;
    let (ri, state) = build_golden_return(&i);
    let filed = file(&ri, &state);
    let f1040 = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    assert_eq!(
        on_paper_signed(&f1040, "line7a", Sign::Leading),
        Some(-3000),
        "the §1211(b) cap prints identically on both households — the two differ only in what the \
         cap ABSORBS"
    );
    assert_eq!(
        filed.ar.taxable_income,
        Usd::ZERO,
        "this is the floor case: 1040 line 15 is zero, and the signed figure behind it is negative"
    );
    assert_eq!(
        filed.ar.capital_loss_carryforward_out.long,
        dec!(20000),
        "§1212(b)(2)(B): with taxable income on the floor the $3,000 allowance absorbed NOTHING, so \
         the FULL $20,000 carries forward. The flat rule this replaced printed $17,000 here — a \
         $3,000 loss the filer would never have got back."
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 8 — the standard-deduction election, and the forms it makes DISAPPEAR.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// A $5,000 long-term crypto donation on the ledger — the gift that WOULD drive Schedule A line 12
/// and a Form 8283 if the return itemized.
fn crypto_gift_5k() -> btctax_core::state::Removal {
    use btctax_core::event::BasisSource;
    use btctax_core::identity::{EventId, LotId};
    use btctax_core::state::{Removal, RemovalKind, RemovalLeg, Term};
    use time::macros::date;
    Removal {
        event: EventId::decision(77),
        kind: RemovalKind::Donation,
        removed_at: date!(2024 - 07 - 01),
        legs: vec![RemovalLeg {
            lot_id: LotId {
                origin_event_id: EventId::decision(777),
                split_sequence: 0,
            },
            sat: 100_000_000,
            basis: dec!(1000),
            fmv_at_transfer: dec!(5000),
            term: Term::LongTerm, // LT ⇒ deductible at FMV, no §170(e) reduction
            basis_source: BasisSource::ExchangeProvided,
            acquired_at: date!(2020 - 01 - 01),
            pseudo: false,
        }],
        appraisal_required: false,
        donor_acquired_at: None,
        claimed_deduction: Some(dec!(5000)),
        donee: Some("ACME CHARITY".into()),
    }
}

/// ★★★ **THE ELECTION THAT DELETES TWO FORMS — AND THE NEGATIVE TEST FOR P4's LIVENESS.**
///
/// A filer who gives $5,000 of bitcoin and then takes the standard deduction claims **no §170
/// deduction at all**. So the packet must carry **no Schedule A and no Form 8283** — TY2024 has no
/// non-itemizer charitable line, and Form 8283 exists to substantiate a deduction that is not being
/// taken. This is the §G-28 assertion a value-check cannot make: there is no number to check, only a
/// form that must not be there.
///
/// ★★★ **AND IT IS THE NEGATIVE TEST FOR P4.** §170(f)(8) conditions a DEDUCTION on holding a
/// contemporaneous written acknowledgment. Phase 2 made an unanswered acknowledgment REFUSE — but only
/// on a return that actually claims the deduction (`deduction_is_itemized`). A standard-deduction
/// filer claims none, so gating them would be asking a question whose answer changes no figure and
/// refusing a return that is already correct. This household never answers it, and files.
///
/// ★★ **AND THE ELECTION IS THE ONLY THING SHUTTING THE GATE** — see the `state_income_tax` comment
/// in the body. The gate is `deduction_is_itemized && (cwa_claimed > 0 || cwa_deferred > 0)`; this
/// fixture makes the second conjunct TRUE ($5,000 on Schedule A line 12) so that the first is the
/// binding one. Both premises are asserted below, so the KAT cannot pass by never reaching the gate.
/// The first version of this KAT had no Schedule-A axis at all, which left every conjunct false — it
/// was green under the very mutation its commit message claimed had killed it (phase-4 review I-1),
/// and phase 2's own scoping test at `return_1040.rs:7268-7276` had already written down the trap.
///
/// ★ The P1 mortgage ceiling and the Form 4952 line-9 bound are NOT exercised here: this filer has
/// neither a mortgage nor investment interest, so those two refusals are unreachable for reasons that
/// have nothing to do with the election, and this KAT proves nothing about them. They are covered
/// non-vacuously by `kat_attestation.rs`'s
/// `the_mortgage_debt_limit_question_is_asked_on_inputs_and_refuses_on_the_deduction`, which is the
/// model this KAT should have followed.
///
/// **If this KAT ever shows L7 being asked the acknowledgment question, that is a P4 liveness defect
/// — the KAT is not to be adjusted to accept it.**
#[test]
fn a_five_thousand_dollar_gift_under_the_standard_deduction_files_no_schedule_a_and_no_8283() {
    let mut i = zero_inputs("Single");
    i.w2_income = 30_000.0;
    // ★★★ THE $1,000 THAT MAKES THIS KAT NON-VACUOUS — do not delete it as an unused axis.
    //
    // Without a Schedule-A axis, `build_golden_return` leaves `ri.schedule_a` = `None`, so
    // `ar.schedule_a` is `None` too and `cwa_claimed` is $0; the gift is under the CapGainProp30
    // ceiling so `charitable_carryover_out` is empty and `cwa_deferred_to_carryover` is $0. ALL THREE
    // conjuncts of the P4 gate would then be false, `deduction_is_itemized` would never be the
    // BINDING one, and dropping it from the gate would leave this KAT green — which is exactly what
    // happened (the phase-4 review re-ran the claimed kill and it did not reproduce).
    //
    // $1,000 of state income tax gives the return a Schedule A carrying `charitable_noncash_12` =
    // $5,000, so `cwa_claimed` = $5,000 > 0 and the gift clears the $250 threshold — while $6,000 of
    // itemized deductions still loses to the $14,600 standard deduction, so the ELECTION is
    // unchanged. `deduction_is_itemized` is now the ONLY thing shutting the gate.
    i.state_income_tax = 1_000.0;
    let (ri, mut state) = build_golden_return(&i);
    state.removals.push(crypto_gift_5k());

    // ★ The acknowledgment is deliberately left UNANSWERED (`charitable_cwa_obtained` is `None`).
    //   `file()` asserts both screens pass, so if P4 ever became live here this test reds on the
    //   refusal — which is the liveness assertion, not an accident of the fixture.
    assert!(
        ri.charitable_cwa_obtained.is_none(),
        "the §170(f)(8) acknowledgment must be UNANSWERED for this KAT to test P4's liveness at all"
    );
    let filed = file(&ri, &state);

    assert!(
        !filed.ar.deduction_is_itemized,
        "this household must take the STANDARD deduction — $6,000 of itemized deductions does not \
         clear $14,600, and if it ever did the whole KAT would be testing the other branch"
    );
    // ★★ THE PREMISE ASSERTIONS — each one names a conjunct that must be TRUE, so the gate can only
    //    be shut by the election. A fixture that stopped reaching the gate would red HERE rather than
    //    passing by never arriving.
    assert!(
        filed
            .ar
            .schedule_a
            .as_ref()
            .is_some_and(|a| a.charitable_noncash_12 == dec!(5000)),
        "the return must CARRY the $5,000 gift on Schedule A line 12 — that is `cwa_claimed` > 0, \
         the conjunct that makes `deduction_is_itemized` the binding one. Schedule A: {:?}",
        filed
            .ar
            .schedule_a
            .as_ref()
            .map(|a| a.charitable_noncash_12)
    );
    assert!(
        filed.ar.charitable_carryover_out.is_empty(),
        "…and nothing defers: the $5,000 gift is under the CapGainProp30 ceiling, so the OTHER \
         disjunct is genuinely $0 and the claimed-amount one is doing the work"
    );
    let names = form_names(&filed.forms);
    assert!(
        !names.contains("f1040sa"),
        "a standard-deduction return files NO Schedule A — TY2024 has no non-itemizer charitable \
         line, so there is nowhere for this gift to go. Packet: {names:?}"
    );
    assert!(
        !names.contains("f8283"),
        "…and no Form 8283: the form substantiates a noncash deduction, and this return claims none. \
         Filing one would attest to a deduction that is not on the return. Packet: {names:?}"
    );
    assert_eq!(
        names,
        BTreeSet::from(["f1040"]),
        "and nothing else appears either — this is a one-form return"
    );

    // ── THE DISCRIMINATING TWIN: itemize, and BOTH forms appear. ────────────────────────────────────
    //
    // Without this, the absences above would be satisfied by an emitter that never produced a
    // Schedule A or an 8283 for anyone — the green-and-blind instrument. The election must SEPARATE.
    let mut i = zero_inputs("Single");
    i.w2_income = 30_000.0;
    i.mortgage_interest = 25_000.0;
    let (mut ri, mut state) = build_golden_return(&i);
    state.removals.push(crypto_gift_5k());
    // The itemizing twin DOES claim the deduction, so §170(f)(8) is live for it and it must answer.
    // That asymmetry is the guarantee: the same gift, the same charity, and only the election differs.
    ri.charitable_cwa_obtained = Some(true);
    answer_all_live_declarations(&mut ri);
    let twin = file(&ri, &state);
    assert!(
        twin.ar.deduction_is_itemized,
        "the twin must itemize or it discriminates nothing"
    );
    let twin_names = form_names(&twin.forms);
    assert!(
        twin_names.contains("f1040sa") && twin_names.contains("f8283"),
        "the ITEMIZING twin must carry both forms — otherwise their absence above proves nothing \
         about the election. Packet: {twin_names:?}"
    );
    let sch_a = cells(
        &twin.forms,
        "f1040sa",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040sa, 2024).unwrap(),
    );
    assert_eq!(
        sch_a.get("line12").map(String::as_str),
        Some("5000"),
        "the twin's Schedule A line 12 carries the noncash gift — the very figure the \
         standard-deduction filer does not claim"
    );
}

/// ★★ **AND THE ACKNOWLEDGMENT GATE ITSELF STILL BITES ON THE BRANCH THAT CLAIMS THE DEDUCTION.**
///
/// The KAT above asserts P4 does NOT fire for a standard-deduction filer. On its own that is
/// satisfiable by a P4 that fires for nobody. This is the other half: the same gift, itemizing, with
/// the acknowledgment unanswered, must REFUSE.
#[test]
fn the_same_gift_on_an_itemizing_return_refuses_until_the_acknowledgment_is_answered() {
    let mut i = zero_inputs("Single");
    i.w2_income = 30_000.0;
    i.mortgage_interest = 25_000.0;
    let (mut ri, mut state) = build_golden_return(&i);
    state.removals.push(crypto_gift_5k());
    ri.charitable_cwa_obtained = None;
    answer_all_live_declarations(&mut ri);

    let params = ty2024_params();
    let table = ty2024_table();
    assert!(
        screen_inputs(&ri, &table, &params).is_none(),
        "the §170(f)(8) gate is an ABSOLUTE-return screen (it needs the computed §63(e) election), \
         so nothing may refuse at the input screen here"
    );
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    let refusal = screen_absolute(
        &ri,
        &ar,
        &params,
        &state,
        2024,
        btctax_core::InformationReturnRegime::NONE,
    )
    .expect("an ITEMIZING return with an unanswered §170(f)(8) acknowledgment must REFUSE");
    assert_eq!(
        format!("{:?}", refusal.reason),
        "CharitableCwaUnresolved",
        "and it must refuse for the acknowledgment, not for something else"
    );
    assert!(
        refusal
            .detail
            .contains("contemporaneous written acknowledgment"),
        "the refusal must name what the filer has to go and get"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 9 — the all-zero return.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// **EVERY money cell the all-zero 1040 writes, and what it writes there.** Everything not listed is
/// BLANK on the paper.
///
/// ★★★ **THIS IS A PROVENANCE TABLE, NOT AN "EVERY LINE IS ZERO" CHECK, and the difference is the
/// whole point of the KAT.** Most lines of a tax return are blank, intentionally, and a test that
/// demanded every line carry `0` would push the emitter toward printing sworn zeros on lines nobody
/// was asked about — worse than the gap it closes. Equally, a test that only checked the cells that
/// happen to be present cannot tell *"this line encodes no decision"* from *"we forgot this line"*.
///
/// So the assertion is an EQUALITY over the whole money map: exactly these keys, exactly these values.
/// A line that starts printing a spurious zero reds; a line that stops printing one reds; a value that
/// changes reds.
///
/// ★★ **TWO CELLS ARE NOT ZERO, AND THEY ARE THE INTERESTING ONES.** Line 12 is the §63(c)(2)
/// standard deduction — $14,600 for Single in TY2024 — and it prints in full even though there is no
/// income to apply it to, because that is what the line says: *"Standard deduction or itemized
/// deductions"*. Line 14 is *"Add lines 12 and 13"*, so it carries the same figure. Line 15 then
/// floors at zero (*"If zero or less, enter -0-"*), which is where the excess deduction disappears —
/// on the line whose own instruction says to drop it, and not one line earlier. A `0` on line 12
/// would be a different return: it would say this filer claimed no standard deduction.
///
/// ★★ **LINES 19 AND 20 ARE ABSENT FROM THIS TABLE ON PURPOSE** — they are BLANK on the paper, and
/// the assertions that hold them blank are the `!contains_key` pair at the foot of the test (FR-1,
/// FR-27). Because `spurious`/`vanished` above make this table an EQUALITY, a row here is what would
/// make a printed zero mandatory; the row is the bug, not the omission.
///
/// ★★ **LINE 21 IS ABSENT TOO, as of FR-39 (owner ruling 2026-09-05)** — and its `!contains_key`
/// assertion sits with the other two at the foot of the test. *"Add lines 19 and 20"* has two blank
/// operands on this return, and the census's own `Combine` rule is *"blank iff every operand is
/// blank"*, so the row this table used to carry was the code contradicting the rule. FR-1's decision
/// in `5094bfc5` was sound while line 20 was unconditionally present — it stopped being complete
/// when FR-27 made line 20 blank-capable, not wrong. Form 8960 line 11 is the identical shape and
/// moved in the same commit, as this comment always said it must.
const ALL_ZERO_1040_PAPER: &[(&str, &str)] = &[
    ("line1a", "0"),     // "Total amount from Form(s) W-2, box 1" — no W-2
    ("line1z", "0"),     // "Add lines 1a through 1h"
    ("line2a", "0"),     // tax-exempt interest
    ("line2b", "0"),     // taxable interest
    ("line3a", "0"),     // qualified dividends
    ("line3b", "0"),     // ordinary dividends
    ("line7a", "0"),     // capital gain or (loss)
    ("line8", "0"),      // Schedule 1 line 10
    ("line9", "0"),      // total income
    ("line10", "0"),     // Schedule 1 line 26
    ("line11", "0"),     // AGI
    ("line12", "14600"), // ★ §63(c)(2) standard deduction, Single TY2024 — NOT zero
    ("line13", "0"),     // §199A QBI deduction
    ("line14", "14600"), // ★ "Add lines 12 and 13"
    ("line15", "0"),     // taxable income — "If zero or less, enter -0-": the floor is HERE
    ("line16", "0"),     // tax
    ("line17", "0"),     // Schedule 2 line 3
    ("line18", "0"),     // add 16 and 17
    ("line22", "0"),     // subtract 21 from 18
    ("line23", "0"),     // Schedule 2 line 21
    ("line24", "0"),     // TOTAL TAX
    ("line25a", "0"),    // withholding — Form(s) W-2
    ("line25b", "0"),    // withholding — Form(s) 1099
    ("line25c", "0"),    // withholding — other forms
    ("line25d", "0"),    // add 25a through 25c
    ("line26", "0"),     // estimated tax payments
    ("line31", "0"),     // Schedule 3 line 15
    ("line32", "0"),     // total other payments and refundable credits
    ("line33", "0"),     // TOTAL PAYMENTS
];

/// ★★★ **THE ALL-ZERO RETURN FILES, AND NOTHING PINNED THAT IT KEEPS DOING SO.**
///
/// A filer with no income at all still files a 1040 — one form, standard deduction, tax $0. The oracle
/// corpus structurally CANNOT contain this household: `corpus.py`'s "no all-none row" constraint
/// excludes the degenerate zero-income return, so no engine has ever scored it and no sweep ever will.
/// That is precisely why it needs a KAT: it is the one shape with no independent witness at all.
///
/// ★ **Note what is NOT on this paper.** Lines 34 and 35a — the refund block — are BLANK, not zero,
/// because line 33 does not exceed line 24; the form's own arithmetic ("If line 33 is more than line
/// 24, subtract line 24 from line 33") never fires. A `0` there would assert a $0 refund was computed
/// rather than that no overpayment exists. Two blanks that look identical on the page, and only one
/// of them is this return.
#[test]
fn the_all_zero_return_files_one_form_whose_every_money_line_is_zero_or_blank() {
    let (ri, state) = build_golden_return(&zero_inputs("Single"));
    let filed = file(&ri, &state);

    assert_eq!(
        form_names(&filed.forms),
        BTreeSet::from(["f1040"]),
        "an all-zero return is ONE form: no schedule has anything to say"
    );
    assert_eq!(filed.ar.agi, Usd::ZERO);
    assert_eq!(filed.ar.taxable_income, Usd::ZERO);
    assert_eq!(filed.ar.total_tax, Usd::ZERO);

    let f1040 = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );

    // The money map: every `lineN*` key on the paper. Identity cells (name, SSN) and the filing-status
    // checkbox are not money and are separately covered by the packet identity sweep.
    let got: BTreeMap<&str, &str> = f1040
        .iter()
        .filter(|(k, _)| k.starts_with("line"))
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let want: BTreeMap<&str, &str> = ALL_ZERO_1040_PAPER.iter().copied().collect();

    let spurious: Vec<_> = got.keys().filter(|k| !want.contains_key(*k)).collect();
    let vanished: Vec<_> = want.keys().filter(|k| !got.contains_key(*k)).collect();
    let changed: Vec<String> = got
        .iter()
        .filter_map(|(k, v)| {
            want.get(k)
                .filter(|w| *w != v)
                .map(|w| format!("{k}: paper {v:?}, pinned {w:?}"))
        })
        .collect();

    assert!(
        spurious.is_empty(),
        "the all-zero 1040 grew a printed cell on {spurious:?}. A hardcoded zero and a computed zero \
         are indistinguishable on the page and are not the same testimony: a figure on a line the \
         filer was never asked about is an assertion they never made (26 USC §6065). If the new cell \
         is genuinely reached by the form's own arithmetic, add it to ALL_ZERO_1040_PAPER with the \
         line's own words."
    );
    assert!(
        vanished.is_empty(),
        "the all-zero 1040 stopped printing {vanished:?}. These are of TWO kinds and both belong on \
         the paper: the COMPUTED ones (1z, 9, 11, 14, 15, 18, 21, 22, 24, 25d, 32, 33) are lines the \
         form's arithmetic reaches — 'add lines …' with nothing to add is still a computed zero — \
         and the ENTRY ones (1a, 2a, 2b, 3a, 3b, 25a, 25b, 25c, 26) are lines this filer was asked \
         about and answered with a zero. Either way the cell is where a reader looks to see the \
         return was completed rather than abandoned."
    );
    assert!(
        changed.is_empty(),
        "a value on the all-zero 1040 moved: {changed:?}. On a return with no income of any kind \
         every figure is fixed by §63(c) and the form's own additions; nothing here is free."
    );

    // ★ The two non-zero cells, called out so a future reader cannot mistake them for noise in the
    //   table above — and so that flattening them to zero reds with its own message.
    assert_eq!(
        got.get("line12"),
        Some(&"14600"),
        "1040 line 12 carries the FULL §63(c)(2) standard deduction ($14,600, Single, TY2024) even \
         though there is no income to apply it to. Printing 0 here would say the filer claimed no \
         standard deduction — a different return."
    );
    assert_eq!(
        got.get("line15"),
        Some(&"0"),
        "…and the excess disappears on line 15, whose own instruction says 'If zero or less, enter \
         -0-'. That is the line the floor belongs to, and it is one line below where it would have \
         to be if line 12 were clamped."
    );
    assert!(
        !got.contains_key("line34") && !got.contains_key("line35a"),
        "the refund block must be BLANK, not zero: line 33 does not exceed line 24, so the form's \
         'If line 33 is more than line 24' never fires. A `0` there asserts a computed $0 refund \
         rather than the absence of an overpayment. Paper: {got:?}"
    );

    // ★★★ FR-1 — AND LINE 19 IS BLANK, which is the third kind of absence on this paper.
    //
    // Lines 34/35a above are blank because the form's own CONDITION never fired. Line 19 is blank for
    // a different reason: it is a CARRY from Schedule 8812 line 14, and btctax figures no §24 credit,
    // so there is no source figure to carry. This filer has no dependents on file — and an empty
    // `#[serde(default)] dependents` cannot be told from a question never asked, so btctax cannot
    // prove Schedule 8812 line 12 answers "No" either. Neither branch of the form's instruction is
    // established; the cell belongs to the filer. It printed `0` in every release up to this one.
    assert!(
        !got.contains_key("line19"),
        "1040 line 19 must be BLANK, not zero. It carries Schedule 8812 line 14, which btctax never \
         figures; a `0` swears the filer worked that schedule and it came to nothing. Paper: {got:?}"
    );

    // ★★★ FR-27 — AND SO IS LINE 20, for the same reason and about a different absent document.
    //
    // "Amount from Schedule 3, line 8". This packet is ONE form; there is no Schedule 3 in it, so
    // there is no line 8 to name. And btctax could not swear to the absence even if it wanted to:
    // six of Schedule 3 line 8's seven operands are credits v1 does not model, which is why
    // `OtherCreditsOmitted` fires on every return. The cell printed `0` in every release up to this
    // one — the same shape as line 19, one line down, about a page rather than a credit.
    assert!(
        !got.contains_key("line20"),
        "1040 line 20 must be BLANK, not zero. It reads \"Amount from Schedule 3, line 8\" and this \
         packet contains no Schedule 3; a `0` is a figure sworn to have been read off a page that \
         does not exist. Paper: {got:?}"
    );

    // ★★★ FR-39 — AND SO IS LINE 21, because it is the SUM of the two blanks above.
    //
    // "Add lines 19 and 20" is a `Combine`, whose transcribed rule in this repo's own census is
    // "blank iff every operand is blank" — and on this return both operands are blank, for the two
    // distinct reasons argued above. The owner put it plainly: the sum of two lines that each do not
    // require an answer does not require an answer either. No figure moves, because line 22
    // subtracts a blank as nothing; what changes is only that the paper stops swearing to a total
    // the filer never worked.
    assert!(
        !got.contains_key("line21"),
        "1040 line 21 must be BLANK, not zero. It is \"Add lines 19 and 20\" and BOTH operands are \
         blank on this return; a `0` swears to a total computed from two figures that were never \
         established. Paper: {got:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 10 — FR-1: 1040 line 19, the child tax credit btctax does not figure.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Hang `deps` children on an otherwise-golden household.
///
/// ★ Real SSNs, because the packet refuses a malformed one at `ReturnHeader::build` — a dependent's
///   just as surely as the taxpayer's (`return_refuse.rs`'s SSN sweep pins all three).
fn with_dependents(mut ri: ReturnInputs, deps: usize) -> ReturnInputs {
    // ★★★ T7 / R6 — a dependent row now brings the §152 flowchart with it: every gate the walk
    //     demands must be answered, and Step 5 question 1 (`filer_tin_issued_by_due_date`) becomes
    //     live the moment the first row exists. This helper hangs children on a GOLDEN household,
    //     so it answers them the way that household's filer would — through the registry-derived
    //     helper, never a hand-list here.
    for i in 0..deps {
        ri.header
            .dependents
            .push(btctax_core::tax::return_inputs::Dependent {
                name: format!("Golden Child {i}"),
                ssn: format!("40000000{i}"),
                relationship: "Daughter".into(),
                date_of_birth: None,
                ..Default::default()
            });
    }
    if deps > 0 {
        // ★ The golden households are TY2024 vectors that leave `tax_year` at §G-15's "not stated"
        //   sentinel, so the year is stated here — the helper derives each child's date of birth
        //   from it and asserts rather than computing one against year zero.
        ri.tax_year = 2024;
        btctax_core::tax::testonly::answer_all_dependent_gates(&mut ri);
        ri.header.filer_tin_issued_by_due_date = Some(true);
    }
    ri
}

/// ★★★ **FR-1 — THE CELL THAT SWORE A FAMILY'S CHILD TAX CREDIT WAS ZERO.**
///
/// 1040 line 19 is *"Child tax credit or credit for other dependents from Schedule 8812"*
/// (`design/forms/extract/f1040--2025.txt`). It is a **carry**: the figure comes from Schedule 8812
/// line 14, and btctax emits no Schedule 8812 and computes no §24 credit. Printing `0` there is not a
/// conservative omission — it is testimony, under 26 USC §6065, that this filer worked Schedule 8812
/// and it came to nothing. For a household with children and income under the §24(b) threshold that
/// testimony is FALSE and taxpayer-ADVERSE: the return overstates tax by up to $2,000 a child.
///
/// ★★ **But an unconditional blank is wrong too, and Schedule 8812 says so in as many words**
/// (`design/forms/extract/f1040s8--2024.txt`):
///
/// ```text
/// 12  Is the amount on line 8 more than the amount on line 11?
///       No. STOP. You cannot take the child tax credit, credit for other dependents, or additional
///           child tax credit. Skip Parts II-A and II-B. Enter -0- on lines 14 and 27.
///       Yes. Subtract line 11 from line 8. Enter the result.
/// 14  Enter the smaller of line 12 or line 13. This is your child tax credit and credit for other
///     dependents … Enter this amount on Form 1040, 1040-SR, or 1040-NR, line 19.
/// ```
///
/// So when the §24(b) phase-out provably kills the credit, the form itself instructs `-0-` on line 14
/// and line 14 routes that figure to 1040 line 19. Blank there would be the mirror defect — declining
/// a figure the form asks for.
///
/// **The two households below are the two sides of that one instruction, read off the paper.**
///
/// ★ B1 kill: pin `ctc_odc_line19` to `Some(Usd::ZERO)` (the shipped hardcode) and the first
///   household reds; pin it to `None` and the second reds. Both were watched.
#[test]
fn form_1040_line_19_is_blank_unless_schedule_8812_provably_says_minus_zero() {
    // ── A family the credit BELONGS to: two children, $60,000 of wages, nowhere near §24(b)'s
    //    $200,000 threshold. btctax cannot figure their credit, so it must not answer for them.
    let (ri, state) = build_golden_return(&GoldenInputs {
        w2_income: 60_000.0,
        ..zero_inputs("Single")
    });
    let filed = file(&with_dependents(ri, 2), &state);
    let paper = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    assert!(
        !paper.contains_key("line19"),
        "1040 line 19 must be BLANK for a family whose child tax credit btctax never figured. A `0` \
         here is sworn testimony (26 USC §6065) that Schedule 8812 was worked and came to nothing — \
         for this household it is false AND it overstates their tax by up to $4,000. Paper: {:?}",
        paper.get("line19")
    );

    // ── A family §24(b) has provably wiped out: MFJ, NINE children, $2,085,000 of wages. Schedule
    //    8812 line 9 = $400,000, line 10 = $1,685,000, line 11 = $84,250 — and the CEILING of line 8
    //    (9 × $2,000 = $18,000, every dependent counted as a qualifying child) loses. Line 12 is
    //    "No", which instructs -0- on line 14, which line 14 routes to 1040 line 19.
    let (ri, state) = build_golden_return(&GoldenInputs {
        w2_income: 2_085_000.0,
        ..zero_inputs("Married/Joint")
    });
    let filed = file(&with_dependents(ri, 9), &state);
    let paper = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    assert_eq!(
        paper.get("line19").map(String::as_str),
        Some("0"),
        "1040 line 19 must print -0- when Schedule 8812 line 12 answers NO. The form does not leave \
         this blank: line 12-No says \"Enter -0- on lines 14 and 27\" and line 14 says \"Enter this \
         amount on Form 1040 … line 19\". Paper: {paper:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 5 — E4: every Form 6251 cell, read back off the PDF, against the struct that produced it.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The 41 modelled Form 6251 cells, each paired with the `Form6251::printed()` field it must carry.
///
/// ★ A hand-written pairing is exactly what is under test here — the map says which AcroForm widget
/// line N lives in, and this says which VALUE belongs in line N. Its completeness is not taken on
/// trust: the test asserts this list's LENGTH equals `Form6251Map::money_cells()`'s, so a line added
/// to the form cannot be silently skipped.
///
/// ★★ It compares COUNTS, not sets, and that is adequate rather than sloppy — but only because of a
/// second mechanism, so say which (phase-4 review N-2, an overclaim in this comment: it used to say
/// "the set of cells checked equals"). `money_cells()` is built from an exhaustive
/// `let Self { .. }` destructure, so a cell cannot be added to the map without the compiler
/// demanding it there; the count then cannot match unless this list gained the same cell. Take the
/// destructure away and the count check alone would admit a swap.
fn expected_cells(f: &btctax_core::tax::form6251::Form6251) -> Vec<(&'static str, Usd)> {
    use btctax_core::tax::form6251::Form6251Line1;
    let line1 = match f.line1 {
        Form6251Line1::Y2024 { line1 } => line1,
        _ => panic!("this fixture is a TY2024 return"),
    };
    vec![
        ("line1", line1),
        ("line2a", f.line2a),
        // ★ line 2b is the PARENTHESISED box: the paper carries the MAGNITUDE, and the pre-printed
        //   parentheses supply the sign. Compared as `abs()` for that reason and no other.
        ("line2b", f.line2b.abs()),
        ("line3", f.line3),
        ("line4", f.line4),
        ("line5", f.line5),
        ("line6", f.line6),
        ("line7", f.line7),
        ("line8", f.line8),
        ("line9", f.line9),
        ("line10", f.line10),
        ("line11", f.line11),
        ("line12", f.line12),
        ("line13", f.line13),
        ("line14", f.line14),
        ("line15", f.line15),
        ("line16", f.line16),
        ("line17", f.line17),
        ("line18", f.line18),
        ("line19", f.line19),
        ("line20", f.line20),
        ("line21", f.line21),
        ("line22", f.line22),
        ("line23", f.line23),
        ("line24", f.line24),
        ("line25", f.line25),
        ("line26", f.line26),
        ("line27", f.line27),
        ("line28", f.line28),
        ("line29", f.line29),
        ("line30", f.line30),
        ("line31", f.line31),
        ("line32", f.line32),
        ("line33", f.line33),
        ("line34", f.line34),
        ("line35", f.line35),
        ("line36", f.line36),
        ("line37", f.line37),
        ("line38", f.line38),
        ("line39", f.line39),
        ("line40", f.line40),
    ]
}

/// ★★★ **E4 — NOTHING HAD EVER COMPARED THE EMITTED FORM 6251, LINE BY LINE, TO THE STRUCT.**
///
/// The existing read-back (`f6251_fill.rs`) reads all 41 cells back and asserts each is a whole
/// dollar; the per-line VALUE assertions covered three lines on a synthetic struct. Placement is
/// separately well corroborated — `verify_flat` enforces monotone y-descent per page and
/// `f6251_map.rs` pins the per-page counts and the quoted instruction text — but the ASSIGNMENT had
/// never been checked against the values on a real assembled household. A systematic map offset of
/// the kind the map's own header warns about (`f1_N = line N−2`) would file a Form 6251 with every
/// figure one line out, and nothing would red.
///
/// This drives a REAL household (the AMT-owing fixture, which completes Part III) all the way through
/// `fill_form_6251_with_map` and reads every one of the 41 cells back off the serialized PDF.
#[test]
fn every_form_6251_cell_carries_the_value_the_struct_computed() {
    let (ri, state) = amt_owing_household();
    let params = ty2024_params();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert!(
        ar.amt.part_iii_completed,
        "this fixture must complete Part III, or 29 of the 41 cells go unwritten and the sweep is \
         checking a dozen lines while claiming to check the form"
    );
    assert!(
        ar.amt.line11 > Usd::ZERO,
        "…and it must actually OWE alternative minimum tax, so line 11 is a figure and not a floor"
    );

    let map = Form6251Map::ty2024();
    let pdf = fill_form_6251_with_map(&ar.amt, &kitchen_sink_header(), &map)
        .expect("the AMT household's Form 6251 must fill");
    let paper = extract_lines(
        &pdf,
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F6251, 2024).unwrap(),
    )
    .expect("the filled 6251 transcribes");
    let printed = ar.amt.printed();

    let expected = expected_cells(&printed);
    for (line, want) in &expected {
        let raw = paper.get(*line).unwrap_or_else(|| {
            panic!(
                "Form 6251 {line} is BLANK on the filed form, but the struct computed {want}. Part \
                 III is completed on this household, so every modelled line has a value to print."
            )
        });
        let got: i64 = raw
            .parse()
            .unwrap_or_else(|_| panic!("Form 6251 {line} is not an integer on the paper: {raw:?}"));
        assert_eq!(
            Usd::from(got),
            *want,
            "Form 6251 {line}: the paper says {raw}, the struct says {want}. This is the assignment \
             check E4 names — a systematic map offset would move every figure one line and no \
             geometric or whole-dollar check would notice."
        );
    }

    // ★ COMPLETENESS, taken from the MAP rather than from this list's own length: a line added to
    //   Form 6251 that nobody paired here must fail, not silently go unchecked.
    assert_eq!(
        expected.len(),
        map.money_cells().len(),
        "the pairing above covers {} of the map's {} money cells — a line was added to the form and \
         never given a value to check",
        expected.len(),
        map.money_cells().len()
    );
}

/// ★★★ **E4's OTHER HALF — the Σround/roundΣ identity down the 6251 → Schedule 2 → 1040 chain.**
///
/// Form 6251 line 11 is the AMT. It is carried, unchanged, to Schedule 2 line 2, then to Schedule 2
/// line 3 (Part I's total, which in v1 has no other member), then to 1040 line 17. Four cells, one
/// figure — and each hop is a separate transcription that could round again. They must be EQUAL, not
/// approximately equal: every one is already a whole dollar, so any difference at all is a dropped or
/// re-rounded figure on a signed return.
///
/// Read off the PAPER, not the structs, because that is where a hop is actually lost.
#[test]
fn the_amt_is_the_same_figure_on_form_6251_schedule_2_and_the_1040() {
    let (ri, state) = amt_owing_household();
    let filed = file(&ri, &state);
    assert!(
        filed.ar.amt.line11 > Usd::ZERO,
        "the identity is only load-bearing on a return that OWES AMT — at $0 every cell is trivially \
         equal and three of the four are blank"
    );

    let names = form_names(&filed.forms);
    assert!(
        names.contains("f6251") && names.contains("f1040s2"),
        "an AMT-owing return attaches Form 6251 AND Schedule 2. Packet: {names:?}"
    );
    let f6251 = cells(
        &filed.forms,
        "f6251",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F6251, 2024).unwrap(),
    );
    let sch2 = cells(
        &filed.forms,
        "f1040s2",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040s2, 2024).unwrap(),
    );
    let f1040 = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );

    let l11 = f6251.get("line11").expect("Form 6251 line 11 is the AMT");
    assert_eq!(
        sch2.get("line2"),
        Some(l11),
        "Schedule 2 line 2 is Form 6251 line 11, carried — i1040s2: \"Alternative minimum tax. \
         Attach Form 6251\""
    );
    assert_eq!(
        sch2.get("line3"),
        Some(l11),
        "Schedule 2 line 3 adds Part I, whose only v1 member is line 2 — so it is the same figure"
    );
    assert_eq!(
        f1040.get("line17"),
        Some(l11),
        "1040 line 17 is Schedule 2 line 3. Four cells, one figure: a re-round at any hop files a \
         different tax than the form computed."
    );
}

/// ★★★ **A COLLECTED 9b THAT ZEROES THE TAX MUST NOT DELETE THE FORM THAT CARRIES IT** (P3-1).
///
/// `form_8960_lines` used to return `None` on `line17 <= 0`. That was sound while line 11 was
/// structurally zero — line 17 could then only be zero because the form was not engaged. Phase 3 made
/// line 11 a **collected deduction**, so a large enough 9b became a second route to line 17 = 0: the
/// bound on 9b is `min(5a, 5e)` and is never related to line 8, so an allocation can exceed the whole
/// of Part I and still pass the refusal.
///
/// The result was a return with MAGI over the threshold, real investment income, and total tax
/// reduced by the entire former NIIT **by a sworn filer allocation that printed nowhere** — this
/// repo's "figure with no reader", and invisible to both oracles (§G-9: neither models a Part II
/// deduction). i8960's Who Must File is categorical: *"Attach Form 8960 to your return if your
/// modified adjusted gross income (MAGI) is greater than the applicable threshold amount."*
///
/// This KAT reads the assembled PACKET, not the line struct, because §G-28 is the point: a
/// value-check cannot see a missing form.
///
/// **Plant:** restore `if line17 <= Usd::ZERO { return None; }` in `form_8960_lines` ⇒ reds.
#[test]
fn a_line_9b_that_zeroes_the_niit_keeps_form_8960_in_the_packet() {
    let mut i = zero_inputs("Single");
    i.w2_income = 300_000.0; // MAGI far over the $200,000 Single threshold
    i.taxable_interest = 6_000.0; // real net investment income
    i.state_income_tax = 20_000.0; // 5a — puts the 9b bound at the $10,000 §164(b)(6) cap
    i.mortgage_interest = 30_000.0; // …and makes the return itemize, or the bound is $0
    let (mut ri, state) = build_golden_return(&i);
    // The filer's own §1411(c)(1)(B) allocation — "any reasonable method" is theirs to choose, and
    // $7,000 exceeds the whole of Part I ($6,000) while staying under the $10,000 SALT bound.
    ri.form_8960_line9b = Some(dec!(7000));
    answer_all_live_declarations(&mut ri);

    let filed = file(&ri, &state);

    // ── PREMISES, so the KAT cannot pass by never reaching the region. ─────────────────────────────
    assert_eq!(
        filed.ar.niit.tax,
        Usd::ZERO,
        "★ the premise: the filer's own allocation has zeroed the §1411 tax. If this is ever \
         non-zero the fixture has drifted out of the region the finding is about."
    );
    assert!(
        filed.ar.niit.magi > dec!(200000),
        "…while MAGI is still OVER the $200,000 Single threshold, which is the whole of Who Must \
         File. MAGI: {}",
        filed.ar.niit.magi
    );

    // ── THE FORM IS ON THE PAPER. ─────────────────────────────────────────────────────────────────
    let names = form_names(&filed.forms);
    assert!(
        names.contains("f8960"),
        "★★ Form 8960 must be IN the packet: i8960's Who Must File turns on MAGI, not on tax due, \
         and the allocation that zeroed the tax is testimony that has to appear somewhere. \
         Packet: {names:?}"
    );

    // ── …AND IT CARRIES THE ALLOCATION, read back off the filled PDF. ─────────────────────────────
    let f8960_cells = cells(
        &filed.forms,
        "f8960",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8960, 2024).unwrap(),
    );
    assert_eq!(
        f8960_cells.get("line9b").map(String::as_str),
        Some("7000"),
        "the sworn allocation must be on the paper — printing the tax it produces while hiding the \
         entry that produced it is the defect. Cells: {f8960_cells:?}"
    );
    assert_eq!(
        f8960_cells.get("line12").map(String::as_str),
        Some("0"),
        "line 12 carries its own instruction, \"If zero or less, enter -0-\" — $6,000 − $7,000 is \
         not printed as a negative net investment income"
    );

    // ══ THE DISCRIMINATING TWIN: no allocation ⇒ the same filer OWES the tax. ═════════════════════
    //
    // Without this, "the form is present" is satisfied by an emitter that always files Form 8960.
    let (mut ri_t, state_t) = build_golden_return(&i);
    ri_t.form_8960_line9b = None;
    answer_all_live_declarations(&mut ri_t);
    let twin = file(&ri_t, &state_t);
    assert_eq!(
        twin.ar.niit.tax,
        dec!(228),
        "3.8% × $6,000 — the allocation is worth $228, which is exactly why it may not move the tax \
         without printing"
    );
    assert!(
        form_names(&twin.forms).contains("f8960"),
        "the twin files the form too — the two differ in the FIGURES, not in whether the form exists"
    );
    let twin_cells = cells(
        &twin.forms,
        "f8960",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8960, 2024).unwrap(),
    );
    assert_eq!(
        twin_cells.get("line9b").map(String::as_str),
        None,
        "★ and with no allocation made, 9b is BLANK — never a computed zero. Two blanks look alike \
         on paper; a printed 0 here would swear the filer allocated nothing when they were never \
         asked. Cells: {twin_cells:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 11 — FR-12: Form 8960 line 9d, a total added over three blank lines.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **FR-12 — THE PART II TOTAL THAT SWORE A ZERO ITS OWN OPERANDS NEVER SAID.**
///
/// Form 8960 line 9d is *"Add lines 9a, 9b, and 9c"* (`design/forms/extract/f8960--2024.txt:42`).
/// It carries **no** *"enter -0-"* clause — Part II has none anywhere; the first one on the form is
/// line 12's. So it is a [`Production::Combine`] in this repo's own census, whose transcribed rule is
/// *"Blank iff every operand is blank."*
///
/// And on a btctax return all three operands can be blank at once. 9a (investment interest expense)
/// and 9c (miscellaneous investment expenses) are **unmodelled — they have no field in
/// `Form8960Lines` at all**, so they are blank on every packet this program has ever emitted. 9b is
/// the filer's own §1411(c)(1)(B) allocation, `Option<Usd>`, and P8 made it blank-when-unasked for
/// exactly this reason. The total then printed `0` over three empty cells: sworn testimony (26 USC
/// §6065) that this filer worked Part II and it came to nothing.
///
/// ★★ **An unconditional blank is the mirror defect and this KAT rejects it too.** When the filer
/// DOES allocate, 9d is the figure the form asks for and it must print — the same shape as FR-1,
/// where Schedule 8812 line 12-No instructs `-0-` in as many words. The two households below are the
/// two sides of that one instruction, read off the emitted PDF.
///
/// ★ The fixture is the P3-1 pair, reused deliberately: `a_line_9b_that_zeroes_the_niit_keeps_form_
/// 8960_in_the_packet` already establishes that BOTH of these households file Form 8960, so a failure
/// here can only be about the figure on line 9d.
///
/// **Plant:** `let line9d = Some(line9b.unwrap_or(Usd::ZERO));` (the shipped hardcode) ⇒ the first
/// household reds; `let line9d = None;` ⇒ the second reds. Both were watched.
#[test]
fn form_8960_line_9d_is_blank_unless_the_filer_allocated_something_to_part_ii() {
    let mut i = zero_inputs("Single");
    i.w2_income = 300_000.0; // MAGI far over the $200,000 Single threshold ⇒ the form files
    i.taxable_interest = 6_000.0; // real net investment income
    i.state_income_tax = 20_000.0; // 5a — puts the 9b bound at the $10,000 §164(b)(6) cap
    i.mortgage_interest = 30_000.0; // …and makes the return itemize, or the bound is $0

    // ── A filer who allocated NOTHING to Part II. 9a, 9b and 9c are all blank, so their sum is not
    //    a figure this return contains.
    let (mut ri, state) = build_golden_return(&i);
    ri.form_8960_line9b = None;
    answer_all_live_declarations(&mut ri);
    let filed = file(&ri, &state);
    let paper = cells(
        &filed.forms,
        "f8960",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8960, 2024).unwrap(),
    );
    assert_eq!(
        paper.get("line9b").map(String::as_str),
        None,
        "premise: 9b is blank (P8). If this ever prints, the fixture has left the region."
    );
    assert!(
        !paper.contains_key("line9d"),
        "Form 8960 line 9d must be BLANK when 9a, 9b and 9c all are. \"Add lines 9a, 9b, and 9c\" \
         carries no \"-0-\" clause, and a sum over three empty cells is not a figure the filer \
         supplied — printing 0 swears they worked Part II and allocated nothing. Paper: {:?}",
        paper.get("line9d")
    );

    // ── The same filer, having allocated $7,000 of state income tax. Now 9b is on the paper, so
    //    "Add lines 9a, 9b, and 9c" has an operand and the total is the form's own arithmetic.
    let (mut ri, state) = build_golden_return(&i);
    ri.form_8960_line9b = Some(dec!(7000));
    answer_all_live_declarations(&mut ri);
    let filed = file(&ri, &state);
    let paper = cells(
        &filed.forms,
        "f8960",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F8960, 2024).unwrap(),
    );
    assert_eq!(
        paper.get("line9d").map(String::as_str),
        Some("7000"),
        "…and it must PRINT the total once any operand exists: 9a and 9c are unmodelled (blank), so \
         9d is 9b. Blanking it unconditionally would decline a figure the form asks for — the mirror \
         defect. Paper: {paper:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 12 — FR-27: 1040 line 20, an amount carried from a schedule that was never filed.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **FR-27 — THE CARRY FROM A FORM THAT DOES NOT EXIST.**
///
/// 1040 line 20 is *"Amount from Schedule 3, line 8"* (`design/forms/extract/f1040--2025.txt:112`).
/// A [`Production::Carry`] — *"blank when the source line is blank"* — and, like line 19 beside it,
/// carrying no *"enter -0-"* clause of its own. When btctax files no Schedule 3, there is no
/// Schedule 3 line 8, and a `0` on line 20 is an amount transcribed from a document that was never
/// written.
///
/// ★★ **The honest difficulty, and why the answer is still blank.** A `0` here is *arithmetically*
/// right whenever the filer genuinely has no nonrefundable credits. The question is whether btctax
/// omitted Schedule 3 because the FILER has none or because btctax **does not model** the credit —
/// and those are indistinguishable on the page. Schedule 3 line 8 is *"Add lines 1 through 4, 5a, 5b,
/// and 7"*, and of those seven operands btctax models exactly one: line 1, the foreign tax credit.
/// Dependent-care (2441), education (8863), saver's (8880), residential clean energy and energy
/// efficient home improvement (5695) and every §6a–6z other credit are §3.4 conservative omissions —
/// which is why `Advisory::OtherCreditsOmitted` fires **unconditionally**: *"v1 captures no input
/// that could establish eligibility, so it cannot know whether this filer qualifies, only that it
/// did not try."* An absence btctax can never prove is not an absence it may swear to.
///
/// ★ **An unconditional blank is wrong too**, and the second household is the proof: a filer who paid
/// with a Form 4868 extension files Schedule 3 for Part II alone. The schedule then EXISTS, its line
/// 8 prints `0`, and line 20 is a true carry of a figure a reader can go and check. This KAT reads
/// both forms and asserts the carry against the source cell, so the two cannot drift apart.
///
/// **Plant:** `let line20 = Some(sch_3.map_or(Usd::ZERO, |s| s.line8));` (the shipped hardcode) ⇒ the
/// first household reds; `let line20 = None;` ⇒ the second reds. Both were watched.
#[test]
fn form_1040_line_20_is_blank_unless_a_schedule_3_was_actually_filed() {
    // ── No foreign tax credit, no excess Social Security, no extension payment ⇒ no Schedule 3.
    let (mut ri, state) = build_golden_return(&GoldenInputs {
        w2_income: 60_000.0,
        ..zero_inputs("Single")
    });
    answer_all_live_declarations(&mut ri);
    let filed = file(&ri, &state);
    let names = form_names(&filed.forms);
    assert!(
        !names.contains("f1040s3"),
        "premise: this household files NO Schedule 3. Packet: {names:?}"
    );
    let paper = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    assert!(
        !paper.contains_key("line20"),
        "1040 line 20 must be BLANK when no Schedule 3 was filed. It reads \"Amount from Schedule 3, \
         line 8\"; with no Schedule 3 there is no line 8, and a `0` is an amount sworn (26 USC \
         §6065) to have been read off a form that does not exist. Six of Schedule 3 line 8's seven \
         operands are credits btctax never models, so its absence is UNPROVEN, not established. \
         Paper: {:?}",
        paper.get("line20")
    );

    // ── The same household, having paid $1,000 with a Form 4868 extension. Schedule 3 Part II now
    //    has something to say, so the schedule FILES — and its line 8 is on the paper to be carried.
    let (mut ri, state) = build_golden_return(&GoldenInputs {
        w2_income: 60_000.0,
        ..zero_inputs("Single")
    });
    ri.payments.extension_payment = dec!(1000);
    answer_all_live_declarations(&mut ri);
    let filed = file(&ri, &state);
    assert!(
        form_names(&filed.forms).contains("f1040s3"),
        "premise: an extension payment files Schedule 3 for Part II alone. Packet: {:?}",
        form_names(&filed.forms)
    );
    let sch3 = cells(
        &filed.forms,
        "f1040s3",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040s3, 2024).unwrap(),
    );
    assert_eq!(
        sch3.get("line8").map(String::as_str),
        Some("0"),
        "the SOURCE cell: Schedule 3 line 8 is on this filer's paper and reads 0. Cells: {sch3:?}"
    );
    let paper = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    assert_eq!(
        paper.get("line20").map(String::as_str),
        sch3.get("line8").map(String::as_str),
        "…so 1040 line 20 must carry it. \"Amount from Schedule 3, line 8\" is a transcription when \
         the schedule exists, and blanking it unconditionally would decline a figure a reader can \
         verify against the attached page. 1040: {:?}, Sch 3 line 8: {:?}",
        paper.get("line20"),
        sch3.get("line8")
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 14 — THE RETIREMENT BLOCK REACHES THE PAPER.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **LINES 4a–6b, READ BACK OFF THE FILLED PDF.**
///
/// T14 computed six figures and wired them into line 9. It did not print them: `form1040_full.rs`
/// never referenced `line4b`, so a return owing tax on an IRA distribution filed a paper whose line 9
/// included that distribution and whose line 4b was **empty** — the arithmetic visible on the page did
/// not add up, and the filer's own copy could not be reconciled against the total it was signed under.
/// Three thousand nine hundred and eighty-six tests were green the whole time. Both halves of the
/// defect are held here: removing the 4b write, or the 6a write, reds this test.
///
/// ★★ **Every expected figure is a DIFFERENT number**, and none is a multiple of another, so a cell
/// wired to its neighbour (4b ← 4a is the easy slip, they are adjacent in the map and adjacent in the
/// struct) cannot pass. This is the FR-230 discipline applied to an emitter: an expectation derived
/// from the thing under test measures nothing, so each value here is arithmetic done from the
/// statutory rule, written down, and then checked against what the paper says.
///
/// | cell | figure  | why that number |
/// |------|---------|-----------------|
/// | 4a   | 130,000 | Σ box 1 over BOTH IRA documents — 100,000 + 30,000 |
/// | 4b   | 100,000 | only the first is taxable; the second is a code-`Q` qualified Roth distribution |
/// | 5a   |   7,000 | the pension's box 1, printed because box 2a is LESS than box 1 |
/// | 5b   |   4,000 | the pension's box 2a — the partially-taxable branch |
/// | 6a   |  24,000 | net benefits, box 3 − box 4 (24,000 − 0) |
/// | 6b   |  20,400 | §86(a)(2): 85% of 24,000, the ceiling this much other income reaches |
#[test]
fn the_retirement_block_prints_all_six_cells_and_each_carries_its_own_figure() {
    let (mut ri, state) = build_golden_return(&zero_inputs("Single"));

    let ira_taxable = btctax_core::tax::form1099r::Form1099R {
        box1_gross_distribution: dec!(100_000),
        box2a_taxable_amount: Some(dec!(100_000)),
        box7_distribution_codes: "7".into(),
        exception_applies: Some(false),
        ..btctax_core::tax::testonly::form_1099r_all_boxes_populated()
    };
    // A code-`Q` qualified Roth distribution: gross on 4a, nothing on 4b. This row is what makes the
    // two IRA cells differ, and it is the reason the form prints 4a at all.
    let ira_qualified_roth = btctax_core::tax::form1099r::Form1099R {
        box1_gross_distribution: dec!(30_000),
        box2a_taxable_amount: None,
        box7_distribution_codes: "Q".into(),
        exception_applies: Some(true),
        ..btctax_core::tax::testonly::form_1099r_all_boxes_populated()
    };
    let pension = btctax_core::tax::form1099r::Form1099R {
        kind: btctax_core::tax::form1099r::Form1099RKind::PensionOrAnnuity,
        box1_gross_distribution: dec!(7_000),
        box2a_taxable_amount: Some(dec!(4_000)),
        box7_distribution_codes: "7".into(),
        exception_applies: Some(false),
        ..btctax_core::tax::testonly::form_1099r_all_boxes_populated()
    };
    ri.r_1099 = vec![ira_taxable, ira_qualified_roth, pension];
    // The DOCUMENT census is the filer's own answer to "do you hold one of these?", and it is checked
    // against the rows: a census saying no beside a transcribed row is a CONTRADICTION, not a row to
    // trust. Both families must be declared, or `screen_inputs` refuses before any figure is computed.
    ri.documents.r_1099 = Some(true);
    ri.documents.ssa_1099 = Some(true);

    // R-7: the §86 worksheet is barred outright by a §911/§931/§933 exclusion, so the question must be
    // ANSWERED before a benefit figure may be computed. `Some(false)` is the filer saying no — not a
    // default, and the refusal above proves the unanswered state is reachable.
    ri.has_income_exclusion = Some(false);
    ri.ssa_1099 = vec![btctax_core::tax::form_ssa1099::FormSsa1099 {
        box3_benefits_paid: dec!(24_000),
        box4_benefits_repaid: dec!(0),
        ..btctax_core::tax::testonly::form_ssa1099_all_boxes_populated()
    }];

    let filed = file(&ri, &state);
    let f1040 = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );

    let want = [
        ("line4a", "130000"),
        ("line4b", "100000"),
        ("line5a", "7000"),
        ("line5b", "4000"),
        ("line6a", "24000"),
        ("line6b", "20400"),
    ];
    let got: Vec<(&str, Option<&str>)> = want
        .iter()
        .map(|(k, _)| (*k, f1040.get(*k).map(String::as_str)))
        .collect();
    let expected: Vec<(&str, Option<&str>)> = want.iter().map(|(k, v)| (*k, Some(*v))).collect();
    assert_eq!(
        got, expected,
        "the retirement block did not reach the paper. A figure that is `None` here is a cell the \
         emitter never wrote — line 9 still includes it, so the printed return does not cross-foot."
    );
}

/// ★★★ **AND THE OTHER HALF: A PRINTED `-0-` ON 6b IS TESTIMONY THE WORKSHEET ASKED FOR.**
///
/// The sibling KAT above proves the six cells print when there is something to say. This one pins the
/// case they are most easily confused with, and which the T14 fold itself got wrong in the other
/// direction: a filer whose ONLY income is Social Security.
///
/// §86 does not tax them. The worksheet says so by STOPPING — *"None of your social security benefits
/// are taxable. Enter -0- on Form 1040 … line 6b"* — and `-0-` is a figure the filer was told to write.
/// So 6b is `Some(ZERO)`, **not** blank, while the all-zero return one field away prints neither cell.
/// Two identical-looking zeros:
///
/// | return | 6a | 6b | because |
/// |---|---|---|---|
/// | no benefit statement at all | blank | blank | no form, no line, no testimony |
/// | this one — $12,000 of benefits | 12000 | 0 | the worksheet's own instruction on its STOP branch |
///
/// The arithmetic, from the statute rather than from the code: line 1 is 12,000, so line 2 is 6,000 and
/// line 5 is 6,000 with no other income. Line 8 is the §86(c)(1)(A) base amount for a single filer,
/// **$25,000** — not indexed, then or now. Line 9 asks whether 25,000 is less than 6,000; it is not, so
/// the worksheet stops. AGI is $0 on twelve thousand dollars of benefits, and line 9 says so.
#[test]
fn a_benefits_only_filer_prints_gross_benefits_on_6a_and_the_worksheets_own_zero_on_6b() {
    let (mut ri, state) = build_golden_return(&zero_inputs("Single"));
    ri.documents.ssa_1099 = Some(true);
    ri.has_income_exclusion = Some(false);
    ri.ssa_1099 = vec![btctax_core::tax::form_ssa1099::FormSsa1099 {
        box3_benefits_paid: dec!(12_000),
        box4_benefits_repaid: dec!(0),
        federal_withholding: dec!(0),
        ..btctax_core::tax::testonly::form_ssa1099_all_boxes_populated()
    }];

    let filed = file(&ri, &state);
    assert_eq!(
        filed.ar.taxable_social_security,
        Some(Usd::ZERO),
        "★ Some(ZERO), not None: the worksheet instructed a -0-, and that instruction is the whole \
         difference between this return and the all-zero one"
    );
    assert_eq!(
        filed.ar.agi,
        Usd::ZERO,
        "§86 taxes none of these benefits, so AGI is zero on $12,000 received"
    );

    let f1040 = cells(
        &filed.forms,
        "f1040",
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    );
    assert_eq!(
        (f1040.get("line6a").map(String::as_str), f1040.get("line6b").map(String::as_str)),
        (Some("12000"), Some("0")),
        "6a carries the gross benefit and 6b carries the instructed zero; neither may be blank on a \
         return that HAS a benefit statement"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 15 — THE MFS LIVED-APART DISCLOSURE.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Assemble (but do not FILL) a return for a named year — the compute layer only.
///
/// ★ Separate from [`file_for_year`] because TY2025 assembles and does not fill: its `f1040.map.toml`
/// has no `[header]` block and `YEAR.toml` says `status = "preparing"`. A KAT about a revision-specific
/// DECISION must still be able to reach that revision.
fn absolute_for_year(ri: &ReturnInputs, state: &LedgerState, year: i32) -> AbsoluteReturn {
    let table = match year {
        2024 => ty2024_table(),
        2025 => ty2025_table(),
        y => panic!("this harness bundles TY2024 and TY2025 only, not TY{y}"),
    };
    let params = ty2024_params();
    assert!(
        screen_inputs(ri, &table, &params).is_none(),
        "this fixture must FILE, but screen_inputs refused: {:?}",
        screen_inputs(ri, &table, &params).map(|r| r.reason)
    );
    assemble_absolute(ri, state, &params, &table, year)
}

/// Build an MFS filer who lived apart from their spouse all year and received Social Security.
///
/// ★ `mfs_lived_apart_all_year: Some(true)` is what earns the §86(c)(1)(A) **$25,000** base amount
/// instead of MFS-lived-with's **$0** — a large favourable difference, and the reason both revisions
/// ask the filer to say so on the paper.
fn mfs_lived_apart_with_benefits() -> (ReturnInputs, LedgerState) {
    let (mut ri, state) = build_golden_return(&zero_inputs(
        btctax_core::tax::testonly::golden_filing_status_token(
            btctax_core::tax::types::FilingStatus::Mfs,
        ),
    ));
    ri.documents.ssa_1099 = Some(true);
    ri.has_income_exclusion = Some(false);
    ri.mfs_lived_apart_all_year = Some(true);
    ri.ssa_1099 = vec![btctax_core::tax::form_ssa1099::FormSsa1099 {
        box3_benefits_paid: dec!(30_000),
        box4_benefits_repaid: dec!(0),
        federal_withholding: dec!(0),
        ..btctax_core::tax::testonly::form_ssa1099_all_boxes_populated()
    }];
    (ri, state)
}

/// ★★★ **THE DISCLOSURE IS DECIDED, PER REVISION, AND IT WAS NOT DECIDED AT ALL BEFORE 2026-09-20.**
///
/// `ss_benefits_worksheet::mfs_lived_apart_disclosure` had **no production caller**: btctax computed
/// which disclosure a revision requires and then printed nothing. Meanwhile it had already used the
/// lived-apart fact to pick the $25,000 base amount over $0, so line 6b carried the benefit of an
/// election the paper never made. Both revisions price the omission identically — *"you may get a math
/// error notice from the IRS"* — meaning the Service recomputes 6b with the $0 base and corrects the
/// return against the filer.
///
/// ★★ **THREE STATES, and the two `None`s are the ones a too-wide condition gets wrong.** The
/// disclosure annotates line 6a: with no benefit statement there is no line 6a and nothing to
/// disclose, however true the lived-apart fact is.
#[test]
fn the_mfs_lived_apart_disclosure_is_the_revisions_own_and_only_when_owed() {
    use btctax_core::tax::ss_benefits_worksheet::MfsLivedApartDisclosure as D;

    // TY2024 — no line 6d exists on that form; the instruction is a WRITE-IN.
    let (ri, state) = mfs_lived_apart_with_benefits();
    let ar24 = absolute_for_year(&ri, &state, 2024);
    assert_eq!(
        ar24.mfs_lived_apart_disclosure,
        Some(D::WriteInDBesideBenefitsOnLine6a),
        "TY2024 discloses by writing \"D\" beside the word \"benefits\" on line 6a"
    );
    // TY2025 — the revision that replaced the write-in with a checkbox.
    let ar25 = absolute_for_year(&ri, &state, 2025);
    assert_eq!(
        ar25.mfs_lived_apart_disclosure,
        Some(D::CheckboxOnLine6d),
        "TY2025 discloses by checking line 6d"
    );

    // NOT OWED (1): the filer lived WITH their spouse. A tick or a "D" here would be a false
    // statement about where they lived, made under §6065 on their behalf — and they get the $0 base.
    let mut lived_with = ri.clone();
    lived_with.mfs_lived_apart_all_year = Some(false);
    assert_eq!(
        absolute_for_year(&lived_with, &state, 2025).mfs_lived_apart_disclosure,
        None
    );

    // NOT OWED (2): no benefit statement, so no line 6a to annotate.
    let mut no_statement = ri.clone();
    no_statement.ssa_1099 = Vec::new();
    no_statement.documents.ssa_1099 = Some(false);
    assert_eq!(
        absolute_for_year(&no_statement, &state, 2025).mfs_lived_apart_disclosure,
        None,
        "the lived-apart fact is still true; there is simply no line 6a on this return"
    );

    // ★ The PRINTED flag's own derivation from this decision is pinned where its types live:
    //   `printed.rs::the_line_6d_flag_is_true_only_on_the_checkbox_revision`.
}

/// ★★★ **THE TY2025 MAP CELL IS THE WIDGET THE LABEL READER JOINS TO `6d`, WITH ITS DECLARED
/// ON-STATE.**
///
/// ★★ **Why this and not an end-to-end tick.** TY2025 is not fillable: its `f1040.map.toml` has no
/// `[header]` block and `YEAR.toml` says `status = "preparing"`, so `fill_full_return(.., 2025)`
/// refuses before reaching any cell. Asserting the mapping against the TEMPLATE is the strongest claim
/// available this year, and it is not a weak one — it catches the two ways a checkbox mapping goes
/// wrong: the wrong widget (6c's `c1_41` is one row above, and ticking it would elect the §86(e)
/// lump-sum method the filer never chose) and an on-state the widget does not declare, which renders
/// as unchecked while the map reports success. The end-to-end tick is owed when TY2025 becomes
/// fillable; `FOLLOWUPS.md` FR-260 carries it.
#[test]
fn the_ty2025_line_6d_cell_is_the_labelled_widget_with_a_declared_on_state() {
    let map: btctax_forms::testonly::Form1040Map = toml::from_str(
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2025).unwrap(),
    )
    .expect("the TY2025 1040 map parses");
    let cell = map
        .line6d
        .as_ref()
        .expect("the TY2025 map carries line6d — that revision's form HAS a 6d");
    assert_eq!(cell.field, "topmostSubform[0].Page1[0].c1_42[0]");

    let doc = btctax_forms::testonly::load(
        btctax_forms::testonly::f1040_pdf(2025).expect("the TY2025 template is bundled"),
    )
    .expect("the template loads");
    let fields = btctax_forms::testonly::collect_fields(&doc).expect("fields");
    let idx = btctax_forms::testonly::index(&fields);
    let f = idx
        .get(cell.field.as_str())
        .unwrap_or_else(|| panic!("{} is not a field on the TY2025 template", cell.field));
    let on_states = btctax_forms::testonly::button_on_states(&doc, f.id);
    assert!(
        on_states.contains(&cell.on),
        "the map writes on-state {:?} to line 6d, and the widget declares {on_states:?}. \
         An undeclared on-state renders as UNCHECKED while every write reports success",
        cell.on
    );

    // And TY2024 must NOT have the cell: that form has no 6d, and a cell there would be a write to a
    // widget belonging to some other line.
    let map24: btctax_forms::testonly::Form1040Map = toml::from_str(
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2024).unwrap(),
    )
    .expect("the TY2024 1040 map parses");
    assert!(
        map24.line6d.is_none(),
        "the TY2024 Form 1040 prints no line 6d — it instructs a write-in instead"
    );
}

/// ★★★ **FR-260 CLOSED — THE TY2025 LINE 6d TICK, READ BACK OFF A FILLED PDF.**
///
/// This is the assertion FR-260 was opened for, and it took the whole TY2025 port to reach: the identity
/// block (29 cells), the filing status (5, whose on-state order is NOT TY2024's), the money lines (35,
/// three of them spelled `11a`/`12e`/`13a` as this revision prints them), and a descent grouping derived
/// from each widget's PAGE because TY2025 moved the page break.
///
/// ★★★ **Nothing here is hand-set.** The filer answers that they are MFS and lived apart all year;
/// `mfs_lived_apart_disclosure` returns `CheckboxOnLine6d` for this revision; the printed flag follows;
/// the emitter writes `c1_42[0]`. A map-only check cannot reach the last two hops, and the trap that
/// blocked the first attempt — TY2024's `taxpayer_ssn` FQN existing on this template as a `/MaxLen 2`
/// cell — is closed by the port rather than worked around.
///
/// ★★ Still `fill_form_1040_full_with_map` rather than `fill_full_return`: `full_return_for(2025)` is
/// `None` and `YEAR.toml` says `status = "preparing"`, so a whole PACKET cannot be assembled for TY2025.
/// The 1040 PAGE fills, which is what FR-260 asked to see.
#[test]
fn the_ty2025_line_6d_box_is_ticked_in_a_filled_pdf() {
    let map: btctax_forms::testonly::Form1040Map = toml::from_str(
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2025).unwrap(),
    )
    .expect("the TY2025 1040 map parses");
    assert_eq!(
        map.line6d.as_ref().map(|c| c.field.as_str()),
        Some("topmostSubform[0].Page1[0].c1_42[0]"),
        "6d must still be the widget `label_reader` joins to that label"
    );

    let (ri, state) = mfs_lived_apart_with_benefits();
    let ar = absolute_for_year(&ri, &state, 2025);
    assert_eq!(
        ar.mfs_lived_apart_disclosure,
        Some(btctax_core::tax::ss_benefits_worksheet::MfsLivedApartDisclosure::CheckboxOnLine6d),
        "the TY2025 revision discloses by checkbox, so the flag below must come out true by itself"
    );
    let printed = btctax_core::tax::packet::assemble_printed_return(
        &ri,
        &state,
        &BTreeMap::new(),
        &ar,
        &ty2025_table(),
        2025,
        &[],
        btctax_core::InformationReturnRegime::NONE,
    )
    .expect("the fixture carries a well-formed SSN");
    assert!(
        printed.forms.f1040.line6d_mfs_lived_apart,
        "the printed flag must be true by COMPUTATION on this household, not by assignment"
    );

    let bytes = btctax_forms::testonly::fill_form_1040_full_with_map(
        &printed.forms.f1040,
        &printed.header,
        printed.filing_status,
        &map,
    )
    .expect("the TY2025 1040 page fills");

    let filled = btctax_forms::testonly::load(&bytes).expect("the filled form loads");
    let idx = btctax_forms::testonly::index(
        &btctax_forms::testonly::collect_fields(&filled).expect("fields"),
    );
    assert_eq!(
        btctax_forms::testonly::checkbox_on(&filled, idx["topmostSubform[0].Page1[0].c1_42[0]"].id)
            .as_deref(),
        Some("1"),
        "line 6d must be CHECKED in the filled TY2025 PDF"
    );

    // ★★ Three more read-backs, because a ticked box on an otherwise-wrong page proves little.
    //    The SSN lands in the nine-character cell (the trap that blocked the first attempt); the MFS
    //    filing-status radio carries THIS revision's on-state `3`, not TY2024's `4`; and 6b holds the
    //    §86 figure computed for this household.
    let ssn =
        btctax_forms::testonly::text_value(&filled, idx["topmostSubform[0].Page1[0].f1_16[0]"].id);
    assert_eq!(
        ssn.as_deref().map(str::len),
        Some(9),
        "the taxpayer SSN must be nine digits in f1_16 — got {ssn:?}"
    );
    assert_eq!(
        btctax_forms::testonly::checkbox_on(
            &filled,
            idx["topmostSubform[0].Page1[0].Checkbox_ReadOrder[0].c1_8[2]"].id
        )
        .as_deref(),
        Some("3"),
        "★ MFS is on-state 3 on the TY2025 form. TY2024 numbers MFS 4, and copying that order would \
         file this return under a different filing status — every bracket and phase-out with it."
    );
    // ★★★ 6a and 6b as the PAIR, and this household lands on the STOP branch: MFS filing separately
    //     but living apart all year gets the §86(c)(1)(A) base amount of $25,000, half of its $30,000 of
    //     benefits is $15,000, and 15,000 is below 25,000 — so the worksheet stops and NONE of the
    //     benefits are taxable. 6b therefore prints the `-0-` the worksheet instructs, which IS the
    //     filer's testimony, beside a 6a carrying the full benefit. A blank 6b would be a return that
    //     never ran the worksheet, and those are different facts on identical-looking pixels.
    //     ★ My first expectation here was 20,400 — copied from the TY2024 KAT, whose household has
    //       24,000 of benefits AND 31,000 of other income. Wrong household, right arithmetic.
    for (what, fqn, want) in [
        ("6a", "topmostSubform[0].Page1[0].f1_68[0]", "30000"),
        ("6b", "topmostSubform[0].Page1[0].f1_69[0]", "0"),
    ] {
        assert_eq!(
            btctax_forms::testonly::text_value(&filled, idx[fqn].id).as_deref(),
            Some(want),
            "{what} must read {want} on the filled TY2025 page"
        );
    }
}

/// ★★★ **EVERY TY2025 HEADER CELL IS A REAL WIDGET OF THE RIGHT SHAPE — because nothing FILLS the
///        TY2025 page yet, so the emitter's own guards never run on it.**
///
/// The identity block was mapped before the money lines, which means its 29 cells had **no test at all**:
/// a wrong FQN would sit in the map until the money lines landed, and the `/MaxLen` guard that caught the
/// first porting attempt only fires during a fill. Measured: pointing `taxpayer_ssn` back at the
/// fiscal-year row's `f1_06[0]` red NOTHING across the whole suite.
///
/// So this checks the two things a fill would have: every cell exists, and the SSN cells can hold nine
/// digits. The second is the one that matters — `f1_06[0]` EXISTS on this template and is a two-character
/// box, so existence alone is not a check.
#[test]
fn every_ty2025_header_cell_exists_and_the_ssn_cells_hold_nine_digits() {
    let map: btctax_forms::testonly::Form1040Map = toml::from_str(
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2025).unwrap(),
    )
    .expect("the TY2025 1040 map parses");
    let h = map
        .header
        .as_ref()
        .expect("the TY2025 identity block is mapped");
    let doc = btctax_forms::testonly::load(
        btctax_forms::testonly::f1040_pdf(2025).expect("the TY2025 template is bundled"),
    )
    .expect("the template loads");
    let fields = btctax_forms::testonly::collect_fields(&doc).expect("fields");
    let by = |fqn: &str| fields.iter().find(|f| f.fqn == fqn);

    let text_cells = [
        ("taxpayer_first", &h.taxpayer_first),
        ("taxpayer_last", &h.taxpayer_last),
        ("taxpayer_ssn", &h.taxpayer_ssn),
        ("spouse_first", &h.spouse_first),
        ("spouse_last", &h.spouse_last),
        ("spouse_ssn", &h.spouse_ssn),
        ("address_street", &h.address_street),
        ("address_apt", &h.address_apt),
        ("address_city", &h.address_city),
        ("address_state", &h.address_state),
        ("address_zip", &h.address_zip),
        ("mfs_spouse_name", &h.mfs_spouse_name),
        ("occupation_taxpayer", &h.occupation_taxpayer),
        ("occupation_spouse", &h.occupation_spouse),
        ("ip_pin", &h.ip_pin),
        ("spouse_ip_pin", &h.spouse_ip_pin),
        ("phone", &h.phone),
        ("foreign_country", &h.foreign_country),
        ("foreign_province", &h.foreign_province),
        ("foreign_postal_code", &h.foreign_postal_code),
    ];
    assert_eq!(text_cells.len(), 20, "all twenty text cells are checked");
    for (what, fqn) in text_cells {
        assert!(
            by(fqn).is_some(),
            "{what} maps to {fqn}, which is NOT a field on the TY2025 template"
        );
    }
    // ★★ The SSN cells, by capacity. This is the assertion that would have caught the first port.
    for (what, fqn) in [
        ("taxpayer_ssn", &h.taxpayer_ssn),
        ("spouse_ssn", &h.spouse_ssn),
    ] {
        assert_eq!(
            by(fqn).and_then(|f| f.max_len),
            Some(9),
            "{what} ({fqn}) must be a nine-digit box. TY2024's SSN FQN (`f1_06[0]`) EXISTS on this \
             template and holds TWO characters — it is part of the fiscal-year row TY2025 inserted — so \
             existence is not the check here, capacity is."
        );
    }
    // ★ Every checkbox must exist AND declare the on-state the map writes: an undeclared on-state
    //   renders as unchecked while the write reports success.
    let checks = [
        ("presidential_taxpayer", &h.presidential_taxpayer),
        ("presidential_spouse", &h.presidential_spouse),
        ("claimed_dependent_taxpayer", &h.claimed_dependent_taxpayer),
        ("claimed_dependent_spouse", &h.claimed_dependent_spouse),
        ("mfs_spouse_itemizes", &h.mfs_spouse_itemizes),
        ("taxpayer_aged", &h.taxpayer_aged),
        ("taxpayer_blind", &h.taxpayer_blind),
        ("spouse_aged", &h.spouse_aged),
        ("spouse_blind", &h.spouse_blind),
    ];
    assert_eq!(checks.len(), 9, "all nine header checkboxes are checked");
    for (what, c) in checks {
        let f = by(&c.field)
            .unwrap_or_else(|| panic!("{what} maps to {}, not a field on the template", c.field));
        let on = btctax_forms::testonly::button_on_states(&doc, f.id);
        assert!(
            on.contains(&c.on),
            "{what} writes on-state {:?} to {} and the widget declares {on:?}",
            c.on,
            c.field
        );
    }
    // ★★ And the TY2025 shape: the grid owns the dependents block, so these two are empty/absent.
    assert!(
        h.dependent_rows.is_empty() && h.more_than_four_dependents.is_none(),
        "the TY2025 header must leave the dependents block to `[dependents_grid]`"
    );
}

/// ★★★ **THE DEPENDENTS BLOCK IS DECLARED EXACTLY ONCE — both failure directions, driven through the
///        real emitter.**
///
/// Making `dependent_rows` legitimately empty (so the TY2025 grid can own the block) created a new way to
/// get it wrong: declare NEITHER, and no dependent prints anywhere while the map looks fine. Both
/// directions are refusals, and both are checked here because neither had a kill — planted, and the
/// unguarded version red nothing.
#[test]
fn a_map_declaring_both_dependents_blocks_or_neither_is_refused() {
    let base: btctax_forms::testonly::Form1040Map = toml::from_str(
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, 2025).unwrap(),
    )
    .expect("parses");
    let (ri, state) = mfs_lived_apart_with_benefits();
    let ar = absolute_for_year(&ri, &state, 2025);
    let printed = btctax_core::tax::packet::assemble_printed_return(
        &ri,
        &state,
        &BTreeMap::new(),
        &ar,
        &ty2025_table(),
        2025,
        &[],
        btctax_core::InformationReturnRegime::NONE,
    )
    .expect("the fixture files");
    let fill = |m: &btctax_forms::testonly::Form1040Map| {
        btctax_forms::testonly::fill_form_1040_full_with_map(
            &printed.forms.f1040,
            &printed.header,
            printed.filing_status,
            m,
        )
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default()
    };

    // NEITHER: drop the grid, leaving `dependent_rows` empty.
    let mut neither = base.clone();
    neither.dependents_grid = None;
    let e = fill(&neither);
    assert!(
        e.contains("NEITHER"),
        "a map with no dependents block at all must be REFUSED, or a return prints no dependent while \
         looking complete: {e}"
    );

    // ★ And the baseline still fails for the MONEY lines, not the dependents block — which is what makes
    //   the assertion above about the dependents guard rather than about any old error.
    // ★★★ And the unmutated map FILLS — which is what makes the NEITHER assertion above about the
    //     dependents guard rather than about whatever happened to be missing first. Until the TY2025
    //     money lines and identity block were ported this could only be stated as "fails for some other
    //     reason"; it can now be stated as success.
    let baseline = fill(&base);
    assert!(
        baseline.is_empty(),
        "the unmutated TY2025 map must FILL now that the identity block, filing status and money lines \
         are ported: {baseline}"
    );
}

/// ★★★ **EVERY TY2025 MONEY CELL LANDS IN THE SAME COLUMN AS ITS TY2024 COUNTERPART.**
///
/// `label_reader::every_mapped_line_lands_on_its_own_printed_label` checks the LINE. It cannot check the
/// COLUMN, because a line's sub-line and amount widgets carry the SAME printed label — the form prints
/// `2a` once over a pair, and `16` once over a pair. So a cell pointed at the wrong member of its own
/// pair passes that gate.
///
/// Measured: pointing `line16` at `f2_07[0]` (the MID widget) instead of `f2_08[0]` (the AMOUNT one) red
/// **nothing** across the whole suite. The geometry oracle that would catch it, `verify_flat`, runs at
/// FILL time — and TY2025 cannot fill, so every column assignment in that map was unheld.
///
/// ★★ The check is a CROSS-REVISION one because the column layout is shared: the three bands are
/// `form1040_full`'s own (`SUBLINE [252,324]`, `MID [410,482]`, `AMOUNT [504,576]`) and page 2's
/// occupation/PIN columns sit at identical x in both revisions. So a cell whose band differs from its
/// TY2024 counterpart's is a mis-map, and that is a fact about the pair, not about a number I typed.
#[test]
fn every_ty2025_money_cell_is_in_the_same_column_band_as_its_ty2024_counterpart() {
    const BANDS: [(&str, f32, f32); 3] = [
        ("SUBLINE", 252.0, 324.0),
        ("MID", 410.0, 482.0),
        ("AMOUNT", 504.0, 576.0),
    ];
    let band = |x: f32| -> String {
        BANDS
            .iter()
            .find(|(_, lo, hi)| x >= lo - 1.0 && x <= hi + 1.0)
            .map_or_else(|| format!("other({x:.0})"), |(n, _, _)| (*n).to_string())
    };
    let cells = |year: i32| -> BTreeMap<String, String> {
        let doc = btctax_forms::testonly::load(btctax_forms::testonly::f1040_pdf(year).unwrap())
            .expect("template loads");
        let fields = btctax_forms::testonly::collect_fields(&doc).expect("fields");
        let x_of = |fqn: &str| {
            fields
                .iter()
                .find(|f| f.fqn == fqn)
                .and_then(|f| f.rect.map(|r| r[0]))
        };
        btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040, year)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("line"))
            .filter_map(|l| l.split_once('='))
            .filter_map(|(k, v)| {
                let v = v.trim();
                // Money cells only — a checkbox's value is an inline table.
                if v.starts_with('{') {
                    return None;
                }
                // ★ Take the FIRST QUOTED SPAN, not the trimmed remainder: TY2024's cells carry a
                //   trailing `# comment` on the same line, and `trim_matches('"')` left that inside the
                //   FQN — every TY2024 cell then failed to resolve, the comparison silently had nothing
                //   to compare, and three planted mis-maps passed. Measured, not hypothetical.
                let fqn = v.split('"').nth(1)?;
                Some((k.trim().to_string(), fqn.to_string()))
            })
            .filter_map(|(k, fqn)| x_of(&fqn).map(|x| (k, band(x))))
            .collect()
    };
    let (a, b) = (cells(2024), cells(2025));
    // ★★★ BOTH SIDES get a floor, and that is the correction this test was born from: my first version
    //     asserted only the TY2025 count, the TY2024 side silently resolved to NOTHING (a parser bug),
    //     and three planted mis-maps passed a green test. A comparison with an empty operand is not a
    //     weaker check, it is no check.
    assert!(
        a.len() >= 35 && b.len() >= 30,
        "TY2024 resolved {} money cells and TY2025 {} — a comparison needs both sides, and an empty \
         operand makes this gate pass on everything",
        a.len(),
        b.len()
    );
    let shared = a.keys().filter(|k| b.contains_key(*k)).count();
    assert!(
        shared >= 30,
        "only {shared} money cells are in BOTH maps, so almost nothing is actually compared"
    );
    let mut wrong: Vec<String> = Vec::new();
    for (k, band25) in &b {
        if let Some(band24) = a.get(k) {
            if band24 != band25 {
                wrong.push(format!("{k}: TY2024 {band24}, TY2025 {band25}"));
            }
        }
    }
    wrong.sort();
    assert!(
        wrong.is_empty(),
        "{} TY2025 money cell(s) sit in a different COLUMN from their TY2024 counterpart:\n  {}\n\n\
         A line's sub-line and amount widgets share one printed label, so the label gate cannot see \
         this: the figure would print on the right LINE in the wrong COLUMN of a signed return. If the \
         TY2025 form really moved a line to another column, say so here with the caption that proves it.",
        wrong.len(),
        wrong.join("\n  ")
    );
}
