//! **P9 — attestation for things that were already correct and held by nothing.**
//!
//! Every KAT here pins a guarantee btctax already honours. That is the point: *"a guarantee without a
//! test that reds when it is removed does not exist"* (`CLAUDE.md`). None of these found a defect when
//! it was written — each was watched RED against a planted one, and the plant is named in the doc
//! comment so a future reader can re-run the kill rather than take it on trust (B1).
//!
//! What lives HERE (core) versus in `btctax-forms`: these are chains *inside* the assembled return —
//! two computations of the same figure, or a form line defined by another form's line. The ones that
//! must read the emitted **PDF** back (the §1211(b) printed surface, the absence of a form from the
//! packet, the all-zero return's paper) live in `crates/btctax-forms/tests/attestation.rs`, because
//! `PrintedForms` is not paper and a value-check cannot see a missing form (§G-28).

use btctax_core::conventions::Usd;
use btctax_core::state::LedgerState;
use btctax_core::tax::packet::{assemble_printed_forms, PrintedForms};
use btctax_core::tax::return_1040::{assemble_absolute, screen_absolute, AbsoluteReturn};
use btctax_core::tax::return_inputs::{
    CharitableClass, CharitableGift, ReturnInputs, ScheduleAInputs,
};
use btctax_core::tax::return_refuse::screen_inputs;
use btctax_core::tax::testonly::{
    amt_owing_household, answer_all_live_declarations, build_golden_return, golden_households,
    ty2024_params, ty2024_table, GoldenInputs,
};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// Fixtures. Households are described in the SAME `GoldenInputs` vocabulary the oracle corpus uses, so
// a household here and a household there are the same household — one builder (`build_golden_return`),
// no second copy to drift.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

fn household(json: &str) -> (ReturnInputs, LedgerState) {
    let i: GoldenInputs = serde_json::from_str(json).expect("the fixture parses as GoldenInputs");
    build_golden_return(&i)
}

/// Assemble a household all the way to its printed forms, asserting it does not refuse.
///
/// ★ BOTH screens are run, and that is load-bearing after phase 2: `screen_inputs` now carries the P7
/// Form 4952 declaration (ALWAYS live) and the P1 §163(h)(3)(B) mortgage ceiling, and `screen_absolute`
/// carries P4's §170(f)(8) acknowledgment gate. A fixture that quietly refused would otherwise be
/// "asserted" by a test that never reached its assertions.
fn computed(ri: &ReturnInputs, state: &LedgerState) -> (AbsoluteReturn, PrintedForms) {
    let params = ty2024_params();
    let table = ty2024_table();
    if let Some(r) = screen_inputs(ri, &table, &params) {
        panic!("this fixture must COMPUTE, but screen_inputs refused: {r:?}");
    }
    let ar = assemble_absolute(ri, state, &params, &table, 2024);
    if let Some(r) = screen_absolute(ri, &ar, &params, state, 2024) {
        panic!("this fixture must COMPUTE, but screen_absolute refused: {r:?}");
    }
    let pf = assemble_printed_forms(ri, state, &BTreeMap::new(), &ar, &table, 2024, &[]);
    (ar, pf)
}

/// The NIIT household: MFJ, over the §1411 $250,000 threshold, with investment income of three kinds
/// (interest, a long-term capital gain, and — via `mortgage_interest`/SALT — an itemizing Schedule A
/// for the gift to land on). `gift` is a §170(b)(1)(G) cash gift to a public charity.
fn niit_household(gift: Option<Usd>) -> (ReturnInputs, LedgerState) {
    let (mut ri, state) = household(
        r#"{"filing_status":"Married/Joint","w2_income":300000,"taxable_interest":40000,
            "long_term_capital_gains":100000,"mortgage_interest":30000,
            "state_income_tax":8000,"real_estate_tax":9000}"#,
    );
    if let Some(amount) = gift {
        let a = ri.schedule_a.get_or_insert_with(ScheduleAInputs::default);
        a.charitable.push(CharitableGift {
            class: CharitableClass::Cash60,
            amount,
        });
        // §170(f)(8): a gift of $250 or more needs a contemporaneous written acknowledgment, and P4
        // refuses an itemizing return that has not said whether it holds one. This household holds
        // one — which is a fact about the fixture, not grease: the point of the pair is that the two
        // returns differ ONLY in the gift.
        ri.charitable_cwa_obtained = Some(true);
    }
    answer_all_live_declarations(&mut ri);
    (ri, state)
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 1 — the charitable deduction does not touch net investment income (N-7).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **§170 IS NOT PROPERLY ALLOCABLE TO NET INVESTMENT INCOME, AND NOTHING HELD THAT.**
///
/// On the INDIVIDUAL branch of Form 8960 there is no charitable line at all: charitable deductions
/// appear only at line 18b, inside Part III's *Estates and Trusts* block. Reg. §1.1411-4(f) is a closed
/// list and §170 is not on it — i8960: *"Unless a deduction is specifically identified as properly
/// allocable to net investment income in the section 1411 regulations … the deduction isn't
/// permitted."* And the gift is below-the-line, so it cannot reach line 13 (MAGI) either: AGI is 1040
/// line 11, computed ABOVE Schedule A.
///
/// btctax agrees structurally — `form_8960_lines` takes six arguments and no charitable term can reach
/// it — but structure is not a test. This is the test: the SAME household with and without a
/// $1,000,000 gift, asserting every Form 8960 line is byte-identical while the 1040 moves.
///
/// ★ **WHAT THE 1040 ACTUALLY DOES, versus what the plan predicted.** The plan said line 15 moves by
/// $1,000,000. It does not, and the difference is the law rather than a defect: §170(b)(1)(G) caps a
/// cash gift to a public charity at 60% of AGI, so on this $440,000-AGI household only $264,000 is
/// deducted this year and the remaining $736,000 becomes a §170(d)(1) carryover. The assertion below
/// is therefore against the ceiling-limited figure, cross-checked against the carryover so the two
/// halves must add back to the whole gift — a KAT that asserted $1,000,000 would have been asserting
/// a bug.
#[test]
fn a_million_dollar_gift_moves_the_1040_and_not_one_line_of_form_8960() {
    let (ri_no, st_no) = niit_household(None);
    let (ri_yes, st_yes) = niit_household(Some(dec!(1_000_000)));
    let (ar_no, pf_no) = computed(&ri_no, &st_no);
    let (ar_yes, pf_yes) = computed(&ri_yes, &st_yes);

    // ── The non-interaction, line by line — and WHETHER THE FORM FILES AT ALL. ─────────────────────
    //
    // ★ The comparison is over the `Option`, deliberately. A charitable term wired into the MAGI
    //   argument does not merely move line 13: on this household it drops MAGI below the §1411
    //   $250,000 threshold and the form STOPS FILING. Comparing the unwrapped structs would report
    //   that as a panic on an `expect` rather than as this guarantee's own failure, so the guarantee
    //   owns both outcomes.
    assert_eq!(
        pf_no.f8960, pf_yes.f8960,
        "a $1,000,000 charitable gift changed Form 8960 — either a line moved, or the form stopped \
         filing entirely. Neither is lawful: §170 is not in Reg. §1.1411-4(f)'s properly-allocable \
         list, and the deduction is below the line so it cannot reach line 13's MAGI either. \
         Charitable belongs to line 18b, which is in Part III's ESTATES AND TRUSTS block — an \
         individual return has no charitable line on this form at all."
    );
    let f8960_no = pf_no.f8960.as_ref().expect(
        "this household must FILE Form 8960 or the non-interaction above holds vacuously — two \
         absent forms are trivially equal",
    );
    assert!(
        f8960_no.line17 > Usd::ZERO,
        "…and it must owe NIIT: a $0 line 17 on both sides would also be trivially equal"
    );
    // The exact-cents NIIT the total tax is built from must not move either — same guarantee, other
    // chain (see the two-chain KAT below for why both are checked).
    assert_eq!(
        ar_no.niit, ar_yes.niit,
        "the exact-cents Form 8960 chain moved on a charitable gift"
    );

    // ── …while the INCOME TAX moves, by exactly the amount §170(b)(1)(G) lets through. ─────────────
    let ceiling_allowed = pf_yes
        .sch_a
        .as_ref()
        .expect("the gifting household itemizes")
        .line11;
    assert!(
        ceiling_allowed > Usd::ZERO,
        "the gift must actually be deducted somewhere, or this KAT proves nothing: a gift that \
         deducted $0 would leave every 1040 line unmoved too, and the non-interaction above would \
         hold vacuously"
    );
    assert_eq!(
        pf_no.f1040.line15 - pf_yes.f1040.line15,
        ceiling_allowed,
        "taxable income must fall by exactly the charitable amount Schedule A line 11 claims"
    );
    assert!(
        pf_yes.f1040.line16 < pf_no.f1040.line16,
        "the gift must reduce the income tax — otherwise the pair is not exercising the deduction"
    );

    // ── The §170(b) ceiling accounts for the whole gift: deducted now + carried forward = $1,000,000.
    let carried: Usd = ar_yes
        .charitable_carryover_out
        .iter()
        .filter(|c| c.origin_year == 2024)
        .map(|c| c.amount)
        .sum();
    assert_eq!(
        ceiling_allowed + carried,
        dec!(1_000_000),
        "every dollar of the gift is either deducted this year or carried forward under §170(d)(1); \
         a dollar that is neither has been LOST"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 2 — the two NIIT chains (N-8).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **TWO INDEPENDENT DERIVATIONS OF THE SAME TAX, AND NOTHING COMPARED THEM.**
///
/// `form_8960` (cent-rounded) feeds `total_tax`; `form_8960_lines` (dollar-rounded) feeds Schedule 2
/// line 12 and the printed Form 8960. They are two separate computations from two separate call sites
/// with separately-assembled arguments. Their inputs are the same values *today* — and nothing said
/// so. This is the "a figure with no reader / two chains ⇒ the test IS the comparison" shape that once
/// left `total_tax` short by the whole AMT while every test stayed green.
///
/// The tolerance is the lawful one and no wider: the two chains round at different places (§6102
/// Σround vs roundΣ), so they may differ by strictly less than half a dollar and by nothing else.
///
/// ★ Run over EVERY corpus household that files Form 8960, not one — the divergence this catches is a
/// mis-wired argument, and an argument can be right on one household's operands and wrong on another's.
#[test]
fn the_two_niit_chains_reconcile_on_every_household_that_files_form_8960() {
    let params = ty2024_params();
    let table = ty2024_table();
    let mut seen = 0;
    for h in golden_households() {
        let (ri, state) = build_golden_return(&h.inputs);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        let pf = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, &table, 2024, &[]);
        let Some(f8960) = pf.f8960.as_ref() else {
            continue;
        };
        seen += 1;
        let printed = f8960.line17;
        let exact = ar.niit.tax;
        assert!(
            (exact - printed).abs() < dec!(0.50),
            "{}: the two NIIT chains disagree by more than the lawful rounding residual — \
             `form_8960` (exact cents, feeds 1040 line 24) says {exact}, `form_8960_lines` (printed, \
             feeds Schedule 2 line 12 and the filed Form 8960) says {printed}. These are two separate \
             computations from two separate call sites; a difference this size means one of them was \
             handed a different operand.",
            h.name
        );
    }
    assert!(
        seen >= 3,
        "only {seen} corpus household(s) file Form 8960 — this KAT is nearly inert. The §1411 \
         threshold households are what hold it live; if the corpus lost them, restore one rather \
         than lowering this bound."
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 3 — Form 6251 line 20 IS the QDCGT worksheet's line 5.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **THE FORM DEFINES LINE 20 BY REFERENCE, SO THE TEST IS THE REFERENCE.**
///
/// i6251, line 20: *"Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax
/// Worksheet … (as figured for the regular tax)."* Line 27 cites the same worksheet line. Neither is
/// a formula btctax is free to choose — and today each is computed by a separate expression in
/// `form6251_inputs_from_parts`, with the worksheet itself computed in `method::qdcgt_line16`. Nothing
/// held the two together.
///
/// The worksheet's own arithmetic, transcribed from the Form 1040 instructions and not paraphrased:
///   L1 = 1040 line 15 (taxable income) · L2 = 1040 line 3a (qualified dividends) ·
///   L3 = the §1(h) net capital gain · L4 = L2 + L3 · **L5 = L1 − L4, "if zero or less, enter -0-"**.
///
/// ★ The cap branch matters and is asserted separately: when the preferential slice EXCEEDS taxable
/// income (the low-end shape — a $0-wage filer with a large gain), L5 is 0 rather than negative, and
/// that is the branch where a missing `.max(0)` would file a negative figure on a signed page.
#[test]
fn form_6251_lines_20_and_27_are_the_qdcgt_worksheets_line_5() {
    let params = ty2024_params();
    let table = ty2024_table();

    // (household json, must the L5 floor bind?)
    let cases: &[(&str, bool)] = &[
        // The AMT-owing shape: preferential income well under taxable income.
        (
            r#"{"filing_status":"Married/Joint","w2_income":300000,"qualified_dividends":8000,
                "ordinary_dividends":10000,"long_term_capital_gains":100000}"#,
            false,
        ),
        // ★ THE FLOOR BRANCH — $0 of ordinary income and a $30,000 long-term gain, so the
        //   preferential slice exceeds taxable income and worksheet line 5 is driven to zero.
        (
            r#"{"filing_status":"Single","long_term_capital_gains":30000}"#,
            true,
        ),
        // Qualified dividends alone (L2 without L3), and no capital gain at all.
        (
            r#"{"filing_status":"Single","w2_income":90000,"qualified_dividends":8000,
                "ordinary_dividends":10000}"#,
            false,
        ),
    ];

    for (json, floor_must_bind) in cases {
        let (ri, state) = household(json);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        let pf = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, &table, 2024, &[]);

        // The worksheet, transcribed. Its operands are the ones `qdcgt_line16` itself is handed at
        // `return_1040.rs` — 1040 line 15, 1040 line 3a and the §1(h) preferential net capital gain.
        let l1 = ar.taxable_income;
        let l2 = ar.qualified_dividends;
        let l3 = ar.net_ltcg;
        let l4 = l2 + l3;
        let l5 = (l1 - l4).max(Usd::ZERO); // "If zero or less, enter -0-"

        if *floor_must_bind {
            assert!(
                l4 > l1,
                "{json}: this case exists to drive worksheet line 5 to its floor, but the \
                 preferential slice ({l4}) does not exceed taxable income ({l1}) — the branch is not \
                 being exercised and the KAT is decoration here"
            );
            assert_eq!(l5, Usd::ZERO, "the floor must actually bind on this case");
        }

        assert_eq!(
            ar.amt.line20, l5,
            "{json}: Form 6251 line 20 must BE the QDCGT worksheet's line 5 (L1 {l1} − L4 {l4}, \
             floored at zero). i6251 defines it by reference to that worksheet; it is not an \
             independent formula."
        );
        assert_eq!(
            ar.amt.line27, l5,
            "{json}: Form 6251 line 27 cites the SAME worksheet line as line 20 — the two must be \
             equal on every return, and a divergence means one of them was re-derived"
        );
        // The printed struct must carry the same figure the engine computed: `Form6251::printed()`
        // is what reaches paper, and a rounding applied on one path only would show up here.
        assert_eq!(
            pf.f6251.as_ref().map(|f| f.line20),
            pf.f6251.as_ref().map(|f| f.line27),
            "{json}: the PRINTED lines 20 and 27 diverged even though the computed ones agree"
        );
    }
}

/// The same identity on the fixture household that actually OWES alternative minimum tax — the branch
/// where lines 20 and 27 are load-bearing rather than incidental (Part III only runs there).
#[test]
fn the_qdcgt_line_5_identity_holds_where_part_iii_actually_runs() {
    let (ri, state) = amt_owing_household();
    let params = ty2024_params();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert!(
        ar.amt.part_iii_completed,
        "this fixture exists to exercise Part III; if it stopped completing it, lines 20 and 27 are \
         no longer load-bearing here and the KAT has gone quiet"
    );
    let l5 = (ar.taxable_income - (ar.qualified_dividends + ar.net_ltcg)).max(Usd::ZERO);
    assert_eq!(
        ar.amt.line20, l5,
        "Form 6251 line 20 = QDCGT worksheet line 5"
    );
    assert_eq!(
        ar.amt.line27, l5,
        "Form 6251 line 27 = QDCGT worksheet line 5"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 6 — the 0% preferential band at $0 of ordinary income.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **A $30,000 LONG-TERM GAIN AT $0 OF WAGES OWES NO TAX, AND NO KAT NAMED THE CELL.**
///
/// §1(h)(1)(B) taxes net capital gain at **0%** to the extent it would otherwise fall below the
/// §1(j)(5) breakpoint — $47,025 of taxable income for Single in TY2024. A filer with no ordinary
/// income and a $30,000 long-term gain has taxable income of $15,400 after the standard deduction,
/// entirely inside that band: 1040 line 16 is **$0**, and the return still files.
///
/// The corpus has zero-wage-with-gain rows, so the double-oracle sweep witnesses the arithmetic — but
/// the sweep's own admission gate is the only thing that keeps such a row in it. This names the cell.
///
/// ★★ **THE PLAN'S ASSERTION IS INTACT BUT NO LONGER UNCONDITIONAL — P7 CHANGED THE PRECONDITION.**
/// This household's Schedule D routes `BothGains`, so it prints line 20, and line 20 swears *"you are
/// not filing Form 4952"*. Phase 2 made that declaration ALWAYS live: unanswered, the return now
/// REFUSES rather than checking a box nobody was asked about. So the KAT asserts BOTH halves — the
/// refusal when the declaration is missing, and `line16 == 0` once it is answered — because the
/// second is only reachable through the first, and a KAT that silently relied on a fixture helper
/// answering for the filer would be pinning the pre-P7 behaviour under a post-P7 name.
#[test]
fn a_thirty_thousand_dollar_gain_at_zero_ordinary_income_lands_in_the_zero_percent_band() {
    let params = ty2024_params();
    let table = ty2024_table();
    let (mut ri, state) =
        household(r#"{"filing_status":"Single","long_term_capital_gains":30000}"#);

    // ── Half 1 (P7): with the Form 4952 declaration UNANSWERED, this return refuses. ───────────────
    ri.filing_form_4952 = None;
    let refusal = screen_inputs(&ri, &table, &params).expect(
        "P7: a both-gains return whose Form 4952 declaration is unanswered must REFUSE — Schedule D \
         line 20 would otherwise swear to a fact the filer was never asked",
    );
    assert!(
        format!("{:?}", refusal.reason).contains("Form4952"),
        "the refusal must be the Form 4952 declaration's, not an unrelated screen: {:?}",
        refusal.reason
    );

    // ── Half 2: answered "no, not filing Form 4952" — the return computes, and the tax is $0. ──────
    ri.filing_form_4952 = Some(false);
    let (ar, pf) = computed(&ri, &state);

    assert_eq!(
        pf.sch_d.routing,
        btctax_core::tax::printed::ScheduleDRouting::BothGains { line20_yes: true },
        "this household must route BOTH-GAINS with line 20 checked Yes — that routing is what sends \
         it to the Qualified Dividends and Capital Gain Tax Worksheet, and it is what makes the \
         declaration above live"
    );
    assert_eq!(
        ar.taxable_income,
        dec!(15400),
        "$30,000 gain − the $14,600 TY2024 Single standard deduction"
    );
    assert!(
        ar.taxable_income < dec!(47025),
        "the cell only means something while taxable income stays under the §1(j)(5) 0%-band \
         breakpoint of $47,025 (Single, TY2024); above it the gain is taxed at 15% and this KAT \
         would be asserting the wrong law"
    );
    assert_eq!(
        pf.f1040.line16,
        Usd::ZERO,
        "1040 line 16 must be $0: every dollar of the gain sits in §1(h)(1)(B)'s 0% band"
    );
    assert_eq!(
        pf.f1040.line24,
        Usd::ZERO,
        "…and so must the total tax — there is no other tax on this return"
    );
    assert_eq!(
        ar.net_ltcg,
        dec!(30000),
        "the whole gain must reach the preferential slice; if it did not, line 16 would be $0 for \
         the wrong reason"
    );
}

/// ★ The DISCRIMINATING twin: the same shape with a gain large enough to leave the 0% band pays tax.
///
/// Without this, `line16 == 0` above could be satisfied by an engine that returns zero for every
/// preferential household — the classic green-and-blind instrument. The breakpoint must SEPARATE.
#[test]
fn the_same_shape_above_the_zero_percent_breakpoint_owes_tax() {
    let (ri, state) = household(r#"{"filing_status":"Single","long_term_capital_gains":80000}"#);
    let (ar, pf) = computed(&ri, &state);
    assert!(
        ar.taxable_income > dec!(47025),
        "this twin must sit ABOVE the §1(j)(5) breakpoint or it discriminates nothing"
    );
    assert!(
        pf.f1040.line16 > Usd::ZERO,
        "a long-term gain above the 0%-band breakpoint owes tax at 15% on the excess; a $0 here \
         would mean the 0%-band assertion in the previous KAT is vacuous"
    );
}
