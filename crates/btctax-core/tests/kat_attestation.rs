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

/// ★★★ **…AND THE SAME NON-INTERACTION HAS A TWO-ORACLE WITNESS (N-6).**
///
/// The KAT above is btctax checking btctax: it proves the guarantee is held *consistently*, not that
/// it is held *correctly*. Until this corpus cell existed, `grep charit scripts/oracle/corpus.py`
/// returned nothing — **no corpus household combined a gift with NIIT**, so neither OpenTaxSolver nor
/// Tax-Calculator had ever scored the pair and the differential sweep reported OK having checked
/// nothing about it.
///
/// This asserts the cell is still THERE and still exercises both halves. It is deliberately an
/// assertion about the corpus rather than about a number: a cell that is dropped, renamed, or
/// re-steered until its gift stops being deducted or its MAGI stops clearing the §1411 threshold takes
/// the witness with it, silently, while every other test stays green.
#[test]
fn the_corpus_carries_a_household_where_a_gift_and_the_niit_actually_meet() {
    let params = ty2024_params();
    let table = ty2024_table();
    let mut witnesses = 0;
    for h in golden_households() {
        if h.inputs.charitable_cash <= 0.0 {
            continue;
        }
        let (ri, state) = build_golden_return(&h.inputs);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        let pf = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, &table, 2024, &[]);
        let Some(f8960) = pf.f8960.as_ref() else {
            continue; // a gifting household that owes no NIIT witnesses only half of the pair
        };
        if f8960.line17 <= Usd::ZERO {
            continue;
        }
        // Both halves must be LIVE on the same return: a deducted gift AND a positive NIIT.
        let claimed = pf.sch_a.as_ref().map(|a| a.line11).unwrap_or(Usd::ZERO);
        if claimed <= Usd::ZERO {
            continue; // the standard deduction won; no §170 deduction is claimed
        }
        // ★ The §170(b) 60%-of-AGI ceiling must NOT bind, or OTS is disqualified as a witness:
        //   OTS 2024 applies no cash ceiling, and a household that crosses it diverges for that
        //   reason alone (the fate of the Form 6251 fixture's V2b).
        assert_eq!(
            claimed,
            Usd::try_from(h.inputs.charitable_cash).expect("a finite fixture figure"),
            "{}: the whole gift must be deducted this year. A corpus cell whose gift crosses the \
             §170(b)(1)(G) 60%-of-AGI ceiling loses OTS as a witness, because OTS 2024 applies no \
             such ceiling — so it would diverge for a reason that has nothing to do with the \
             non-interaction under test.",
            h.name
        );
        // ★★★ **AND THE CELL MUST BE DISCRIMINATING, WHICH IS A SEPARATE FACT FROM BEING PRESENT.**
        //
        // Form 8960 line 16 is *"Enter the smaller of line 12 or line 15"* — NII, or MAGI over the
        // §1411 threshold. While NII is the smaller, a charitable term wrongly wired into MAGI can
        // move line 13 without moving line 16, and the household reconciles against both oracles
        // WITH THE DEFECT PRESENT. That is not hypothetical: the first steering of this cell had
        // AGI $440,000 and NII $140,000, so MAGI-over-threshold was $190,000 — and the planted
        // defect (charitable subtracted from the MAGI argument) changed nothing at all, because
        // $440,000 − $50,000 − $250,000 is exactly $140,000. The cell existed, was admitted by both
        // engines, and witnessed nothing.
        //
        // Re-steered to $380,000 of AGI, line 15 ($130,000) is the smaller and every dollar of MAGI
        // reaches the tax. This assertion is what stops it drifting back.
        assert!(
            f8960.line15 < f8960.line12,
            "{}: this cell must take Form 8960 line 16's EXCESS-MAGI leg (line 15 {} < line 12 {}), \
             or a charitable term wired into MAGI moves no figure and the cell witnesses nothing \
             while still reconciling against both oracles",
            h.name,
            f8960.line15,
            f8960.line12
        );
        witnesses += 1;
    }
    assert!(
        witnesses >= 1,
        "no corpus household has BOTH a deducted charitable gift and a positive Form 8960 line 17. \
         The charitable × NIIT non-interaction is then witnessed by btctax alone — which is the \
         state N-6 recorded, and the reason `mfj_niit_with_a_large_charitable_gift` was added. \
         Restore a cell; do not delete this assertion."
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
/// Worksheet … (as figured for the regular tax)."* Neither line is
/// a formula btctax is free to choose — and today each is computed by a separate expression in
/// `form6251_inputs_from_parts`, with the worksheet itself computed in `method::qdcgt_line16`. Nothing
/// held the two together.
///
/// ★★★ **LINES 20 AND 27 ARE NOT THE SAME SENTENCE, AND THE ELLIPSIS ABOVE HID WHERE THEY PART**
/// (phase-4 review M-6). From the committed text layer, `design/forms/extract/f6251--2024.txt:107`
/// and `:125` — the two differ in exactly one clause:
///
///   * **L20** — *"…line 5 of the Qualified Dividends and Capital Gain Tax Worksheet **or the amount
///     from line 14 of the Schedule D Tax Worksheet**, whichever applies…"*
///   * **L27** — *"…line 5 of the Qualified Dividends and Capital Gain Tax Worksheet **or the amount
///     from line 21 of the Schedule D Tax Worksheet**, whichever applies…"*
///
/// So "line 27 cites the same worksheet line" is true of the **QDCGTW alternative only**. The
/// equality this KAT asserts holds because of a PRECONDITION, not because the form says so:
/// **v1 refuses every route to the Schedule D Tax Worksheet** (`Form4952Required` /
/// `Form4952DeclarationUnanswered`, with lines 18/19 refused upstream), so the SDTW alternative is
/// unreachable and both lines collapse onto QDCGTW line 5.
///
/// ★ Naming that precondition is the point. If the SDTW path ever lands, lines 20 and 27 legitimately
/// DIVERGE — 14 versus 21 — and this KAT's failure message ("a divergence means one of them was
/// re-derived") would send the next reader hunting for a bug that is not there. Compression hiding
/// the dropped term, in the one place this repo has been burned by it most.
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
            "{json}: lines 20 and 27 both resolve to QDCGT Worksheet line 5 on every return v1 can \
             file, so they must be equal and a divergence means one of them was re-derived. \
             ★ BUT CHECK THE PRECONDITION FIRST: the two sentences differ — line 20 offers Schedule \
             D Tax Worksheet line 14 as its alternative, line 27 offers line 21. They coincide only \
             because v1 refuses every route to the SDTW. If that path has landed, a divergence here \
             may be CORRECT and this KAT is what needs changing."
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

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// P10 — the LIMITATIONS.md promise surface, held by assertions instead of by prose.
//
// `crates/btctax-cli/LIMITATIONS.md` is what a filer reads to decide whether they can rely on this
// return. A stale row there is a false statement to them, and the phase 1-3 refusal work moved things
// the document had to be rewritten around. Reviewing a document for whether it still describes the
// code is the wrong instrument (`CLAUDE.md`); these are the assertions that keep it honest.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **THE §1211/§1212 EDGE: THE IN SIDE NOW FILES, AND THE ADVISORY REACHES IT.**
///
/// This row used to read *"the §1211/§1212 Capital Loss Carryover Worksheet edge is unmodeled"* — one
/// sentence covering both directions — and then, after N1, *"IN still refuses"*. Both are now false.
/// What is true:
///
///   * **OUT — modelled.** The §1212(b)(2)(B) worksheet computes the carryforward, correctly at the
///     floor (`btctax-forms`'s `the_1211b_cap_and_the_1212b_carryforward_print_and_pair` holds the
///     $17,000/$20,000 pair).
///   * **IN — FILES** (owner-authorised). The refusal was never about arithmetic; it was about
///     emitting a return for this household at all.
///   * **The advisory that tells the filer what to carry is only REACHABLE because of the lift.** On
///     the `report` path advisories are built inside the `None` arm of the `screen_absolute` match
///     (`cmd/tax.rs`), and the export path returns at `cmd/admin.rs` before
///     `advisories_for` is called. So while the refusal stood, the household that most needed
///     *"CARRY $100,000, NOT $97,000"* was the one household that could never be shown it.
///
/// ★ **This is K15, and the mutation is the lift itself**: restore the deleted `screen_absolute`
/// block and the two "must file" assertions red — which is what proves the LIFT, not the advisory
/// code, is what makes the figure reachable.
#[test]
fn the_limitations_row_for_the_1211_1212_edge_is_true_of_the_code() {
    let params = ty2024_params();
    let table = ty2024_table();

    // ── IN side: taxable income $0 with a carryforward brought in ⇒ FILES. ─────────────────────────
    let (mut ri, state) = household(r#"{"filing_status":"Single","w2_income":10000}"#);
    ri.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
        short: Usd::ZERO,
        long: dec!(5000),
    };
    answer_all_live_declarations(&mut ri);
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert_eq!(
        ar.taxable_income,
        Usd::ZERO,
        "the fixture must land taxable income on the floor, or the lifted screen is never reached"
    );
    assert!(
        screen_inputs(&ri, &table, &params).is_none(),
        "premise: the worksheet's two header declarations are answered, so the INPUT screen is clean"
    );
    assert_eq!(
        screen_absolute(&ri, &ar, &params, &state, 2024),
        None,
        "★ taxable income of $0 WITH a capital-loss carryforward-in now FILES"
    );

    // ── …and the same filer WITHOUT a carryforward-in still files. ────────────────────────────────
    let (ri_no, state_no) = household(r#"{"filing_status":"Single","w2_income":10000}"#);
    let ar_no = assemble_absolute(&ri_no, &state_no, &params, &table, 2024);
    assert!(
        screen_absolute(&ri_no, &ar_no, &params, &state_no, 2024).is_none(),
        "the refund-only filer with NO carryforward was never refused, and still is not"
    );

    // ── K15 — the advisory the lift makes reachable, with its four figures. ───────────────────────
    //
    // H3: $10,000 of wages, $40,000 short-term and $60,000 long-term carried in. 1040 line 7 shows
    // the §1211(b) −$3,000 cap, but taxable income is already $0, so line 4 of the worksheet absorbs
    // NOTHING and the whole $100,000 survives — where the flat "loss minus $3,000" rule would forfeit
    // $3,000 of it permanently.
    let (mut h3, h3_state) = household(r#"{"filing_status":"Single","w2_income":10000}"#);
    h3.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
        short: dec!(40000),
        long: dec!(60000),
    };
    answer_all_live_declarations(&mut h3);
    let h3_ar = assemble_absolute(&h3, &h3_state, &params, &table, 2024);
    assert!(
        screen_inputs(&h3, &table, &params).is_none()
            && screen_absolute(&h3, &h3_ar, &params, &h3_state, 2024).is_none(),
        "★ H3 must FILE — while it refused, the `report` path never entered the arm that builds \
         advisories and the export path returned before `advisories_for` was called, so the one \
         household this advisory was written for could not be shown it"
    );
    let advs = btctax_core::tax::advisories::advisories_for(&h3, &h3_state, &h3_ar, &params, 2024);
    let found = advs
        .iter()
        .find(|a| {
            matches!(
                a,
                btctax_core::tax::advisories::Advisory::
                    CapitalLossCarryoverWorksheetIncreasesCarryover { .. }
            )
        })
        .expect("the worksheet advisory must reach a household that brought the loss IN");
    assert_eq!(
        *found,
        btctax_core::tax::advisories::Advisory::CapitalLossCarryoverWorksheetIncreasesCarryover {
            // ★ MEASURED, not transcribed. The plan predicted the flat rule would reduce the
            //   LONG-term limb (40,000/57,000). It reduces the SHORT-term one, because §1212(b)(2)
            //   absorbs short-term loss first and `net_1222` implements that faithfully. The TOTAL
            //   is the same $97,000, so the advisory's headline is unaffected — but a KAT asserting
            //   the predicted split would have been asserting a bug that is not there.
            flat_short: dec!(37000),
            flat_long: dec!(60000),
            worksheet_short: dec!(40000),
            worksheet_long: dec!(60000),
            absorbed: Usd::ZERO,
        },
        "worksheet line 8 = $40,000 and line 13 = $60,000 against the flat rule's $97,000 — and \
         line 4 is ZERO, which is the whole reason they differ"
    );
    let text = found.message();
    assert!(
        text.contains("$100,000") && text.contains("$97,000"),
        "the filer must be told both figures by name: {text}"
    );

    // ── The write-back rolls FOUR carryovers, and the capital-loss one is now among them. ─────────
    //
    // ★★★ THIS HALF USED TO ASSERT THE OPPOSITE, and its own message said what to do about it: *"If
    //     the write-back learns to roll it, the row must change in the same commit."* It learned;
    //     the row changed; this assertion is the flipped one. The trip wire worked — it went RED on
    //     the write-back commit before a single word of LIMITATIONS.md had been touched.
    let (ri, state) = household(r#"{"filing_status":"Single","long_term_capital_gains":-20000}"#);
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert_eq!(
        ar.capital_loss_carryforward_out.long,
        dec!(20000),
        "the figure that is rolled must first exist, or this asserts nothing"
    );
    let next = btctax_core::tax::return_1040::apply_carryover_writeback(
        &ar,
        &ri,
        &state,
        2024,
        ReturnInputs::default(),
        false,
    )
    .expect("the write-back succeeds over an empty next year");
    assert_eq!(
        next.capital_loss_carryforward_in,
        btctax_core::tax::types::Carryforward {
            short: Usd::ZERO,
            long: dec!(20000),
        },
        "★ `--write-carryover` NOW rolls the §1212(b) carryover-out into next year's Schedule D \
         lines 6 and 14, as WHOLE DOLLARS — the figure the filer will read off the page"
    );
    assert_eq!(
        next.capital_loss_carryforward_in_provenance,
        btctax_core::tax::return_inputs::CarryProvenance::Computed,
        "…and the stamp is founded, because year Y computed a real carryover-out of its own"
    );

    // ★★★ …AND THE ROW NOW SAYS **FOUR**, SO THE FOUR ARE ASSERTED (final review 6 / phase-4 N-1).
    //
    // The bullet once named "the charitable and QBI-REIT/PTP carryovers" — two, where the code wrote
    // three. The enumeration beside the load-bearing claim was held by nothing, which is how it came
    // to be short. A promise surface that lists what a command does needs the LIST pinned.
    assert_eq!(
        next.charitable_carryover_in_provenance,
        btctax_core::tax::return_inputs::CarryProvenance::Computed,
        "(1 of 3) the charitable carryover is stamped"
    );
    assert_eq!(
        next.qbi.reit_ptp_carryforward_in_provenance,
        btctax_core::tax::return_inputs::CarryProvenance::Computed,
        "(2 of 3) the REIT/PTP carryforward — Form 8995 line 17"
    );
    assert_eq!(
        next.qbi.qbi_carryforward_in_provenance,
        btctax_core::tax::return_inputs::CarryProvenance::Computed,
        "★ (3 of 4) the QUALIFIED-BUSINESS loss carryforward — Form 8995 line 16. This is the one \
         the LIMITATIONS bullet omitted while the paragraph two sections above it counted correctly."
    );
    assert_eq!(
        next.capital_loss_carryforward_in_provenance,
        btctax_core::tax::return_inputs::CarryProvenance::Computed,
        "★ (4 of 4) the §1212(b) CAPITAL-LOSS carryover — Schedule D lines 6 and 14. Newly among \
         them, and the reason every 'all three' in the write-back's own refusal texts had to become \
         'all four' in the same commit: those texts tell a filer what was NOT persisted."
    );
}

/// ★★★ **`Form4952Required` REACHES A STANDARD-DEDUCTION FILER, AND THE PROMISE SURFACE SAID IT
/// COULD NOT** (final whole-branch review, finding 4).
///
/// LIMITATIONS.md carried *"Only ever raised on a return that itemizes — line 9 never prints
/// otherwise."* That is true of the phase-2 **line-9 bound** leg, and FALSE of the `Some(true)` leg,
/// which deliberately stayed in `screen_inputs`: answering "yes, I am filing Form 4952" answers
/// Schedule D line 20 NO and routes the return to the Schedule D Tax Worksheet, which btctax does not
/// fill. That routing is decided by Schedule D, not by Schedule A, so the §63(e) election has nothing
/// to do with it — and the doc's own adjacent bullet described this leg without the qualifier, so the
/// two bullets contradicted each other about which filers the reason code can reach.
///
/// A filer told by the promise surface that a refusal cannot reach them, and then refused, has been
/// misled about the one thing that document exists to say.
///
/// **Plant:** add `ar.deduction_is_itemized &&` to the `filing_form_4952 == Some(true)` refusal in
/// `screen_inputs` ⇒ half 1 reds.
#[test]
fn the_limitations_row_for_form_4952_is_true_of_both_of_its_legs() {
    let params = ty2024_params();
    let table = ty2024_table();

    // ── Half 1: a RENTER with no Schedule A at all, answering YES, is refused. ────────────────────
    let (mut ri, state) = household(r#"{"filing_status":"Single","w2_income":50000}"#);
    ri.filing_form_4952 = Some(true);
    assert!(
        ri.schedule_a.is_none(),
        "the fixture must have NO Schedule A — that is what makes the standard deduction certain, \
         and it is stronger than merely losing the comparison"
    );
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert!(
        !ar.deduction_is_itemized,
        "…so this return takes the STANDARD deduction"
    );
    let r = screen_inputs(&ri, &table, &params).expect(
        "★ answering YES refuses on EVERY return — Schedule D line 20 routes the tax to a \
                 worksheet btctax does not fill, whatever the §63(e) election",
    );
    assert_eq!(format!("{:?}", r.reason), "Form4952Required");
    assert!(
        r.detail.contains("SCHEDULE D TAX WORKSHEET"),
        "…and it must say WHY, which is the routing and not the deduction. Got: {}",
        r.detail
    );

    // ── Half 2: the same filer answering NO is not refused. The answer must SEPARATE. ─────────────
    let (mut ri_no, state_no) = household(r#"{"filing_status":"Single","w2_income":50000}"#);
    ri_no.filing_form_4952 = Some(false);
    answer_all_live_declarations(&mut ri_no);
    assert_eq!(
        screen_inputs(&ri_no, &table, &params).map(|r| format!("{:?}", r.reason)),
        None,
        "a standard-deduction filer NOT filing Form 4952 computes normally — otherwise half 1 is \
         satisfied by a refusal that catches everyone"
    );
    let _ = state_no;
}

/// ★★★ **THE TWO HALVES OF THE §163(h)(3)(B) ROW ARE SCOPED DIFFERENTLY, AND THE ROW SAYS SO.**
///
/// A first draft of the LIMITATIONS row said the debt-limit question is "asked only on a return that
/// actually deducts the interest — a standard-deduction filer is never asked." **That was false**, and
/// this test is why it does not read that way now. `mortgage_question_live` is deliberately an INPUT
/// predicate (`schedule_a.is_some() ∧ mortgage_interest_1098 > 0`) rather than "Schedule A files",
/// because whether Schedule A wins is compute-dependent. So the question IS asked of a filer whose
/// standard deduction ends up larger — while the ADVERSE answer refuses only when the return itemizes,
/// since line 8a never prints otherwise.
#[test]
fn the_mortgage_debt_limit_question_is_asked_on_inputs_and_refuses_on_the_deduction() {
    let params = ty2024_params();
    let table = ty2024_table();

    // A SMALL mortgage: Schedule A inputs carry Form 1098 interest, but the standard deduction wins.
    let build =
        || household(r#"{"filing_status":"Single","w2_income":60000,"mortgage_interest":4000}"#);

    // ── ASKED: unanswered refuses at the INPUT screen even though this filer will not itemize. ──────
    let (mut ri, state) = build();
    ri.schedule_a.as_mut().unwrap().mortgage_within_debt_limit = None;
    let refusal = screen_inputs(&ri, &table, &params)
        .expect("a Schedule A reporting 1098 interest must be asked the §163(h)(3)(B) question");
    assert_eq!(
        format!("{:?}", refusal.reason),
        "MortgageDebtLimitUnanswered"
    );

    // ── ANSWERED ADVERSELY: this filer still FILES, because the standard deduction wins. ────────────
    let (mut ri, state2) = build();
    ri.schedule_a.as_mut().unwrap().mortgage_within_debt_limit = Some(false);
    assert!(
        screen_inputs(&ri, &table, &params).is_none(),
        "the ADVERSE answer is a deduction-level refusal, not an input-level one"
    );
    let ar = assemble_absolute(&ri, &state2, &params, &table, 2024);
    assert!(
        !ar.deduction_is_itemized,
        "the fixture must lose to the standard deduction, or it tests the other branch"
    );
    assert!(
        screen_absolute(&ri, &ar, &params, &state2, 2024).is_none(),
        "an over-the-limit filer whose Schedule A LOSES to the standard deduction files normally: \
         line 8a never prints, so the answer changes no figure. Refusing here is the misfire phase \
         2's review C fixed."
    );

    // ── …and the ITEMIZING twin with the same answer REFUSES. ──────────────────────────────────────
    let (mut ri, state3) =
        household(r#"{"filing_status":"Single","w2_income":60000,"mortgage_interest":25000}"#);
    ri.schedule_a.as_mut().unwrap().mortgage_within_debt_limit = Some(false);
    let ar = assemble_absolute(&ri, &state3, &params, &table, 2024);
    assert!(ar.deduction_is_itemized, "the twin must itemize");
    let refusal = screen_absolute(&ri, &ar, &params, &state3, 2024)
        .expect("an ITEMIZING return over the §163(h)(3)(B) limit must refuse");
    assert_eq!(format!("{:?}", refusal.reason), "MortgageOverDebtLimit");
    let _ = state;
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// FINAL-REVIEW FINDING 1 — the STANDARD-DEDUCTION DEFERRAL DONOR.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// A long-term appreciated-BTC gift on the ledger, at `fmv`. Long-term ⇒ deductible at FMV with no
/// §170(e) reduction, and it lands in the `CapGainProp30` class whose ceiling is 30% of AGI.
fn ledger_gift(fmv: rust_decimal::Decimal) -> btctax_core::state::Removal {
    use btctax_core::event::BasisSource;
    use btctax_core::identity::{EventId, LotId};
    use btctax_core::state::{Removal, RemovalKind, RemovalLeg, Term};
    use time::macros::date;
    Removal {
        event: EventId::decision(91),
        kind: RemovalKind::Donation,
        removed_at: date!(2024 - 07 - 01),
        legs: vec![RemovalLeg {
            lot_id: LotId {
                origin_event_id: EventId::decision(911),
                split_sequence: 0,
            },
            sat: 100_000_000,
            basis: dec!(1000),
            fmv_at_transfer: fmv,
            term: Term::LongTerm,
            basis_source: BasisSource::ExchangeProvided,
            acquired_at: date!(2020 - 01 - 01),
            pseudo: false,
        }],
        appraisal_required: false,
        donor_acquired_at: None,
        claimed_deduction: Some(fmv),
        donee: Some("ACME CHARITY".into()),
    }
}

/// ★★★ **THE FILER THE §170(f)(8) GATE NEVER ASKS — AND THE TWO PLACES THAT NOW DO.**
///
/// Phase 2's R2 fold put `deduction_is_itemized` in front of the acknowledgment refusal; its R3 fold
/// added the `cwa_deferred_to_carryover` disjunct BEHIND it. Both are individually right, and they
/// collide on exactly one population: **standard deduction AND a gift that defers.** §170(d)(1)
/// creates the carryover regardless of §63(e) — `apply_170b` runs unconditionally for that very
/// reason — so R3's mechanism survives on the other branch of the election, where R2 had just
/// declared everything safe.
///
/// Single renter, AGI $40,000, one $25,000 appreciated-BTC gift: the CapGainProp30 ceiling is 30% ×
/// $40,000 = $12,000, so $13,000 defers; $12,000 of itemized deductions loses to the $14,600 standard
/// deduction, so the election is standard and BOTH screens pass. That filer files, §170(f)(8)(C)'s
/// cure dies with the filing (*Durden*), and next year's Schedule A line 13 — deliberately outside the
/// gate — deducts $13,000 unsubstantiated.
///
/// ★★ **THE FIX IS NOT A WIDER REFUSAL, AND THAT IS THE DESIGN DECISION THIS KAT PINS.** The return
/// itself is correct: it claims no §170 deduction, so §170(f)(8)(A) denies nothing on it. What was
/// wrong was (1) btctax told the filer the carryover was *"deduction you have already paid for"*
/// without mentioning §170(f)(8) at all, and (2) `--write-carryover` would have stamped it `Computed`
/// into next year's inputs, past every gate. So the return still FILES; the roll-forward block warns
/// while the cure is still alive; and the write-back refuses to persist what btctax cannot vouch for.
///
/// **Plant to re-run the kill (both halves):**
///   * delete the `cwa_unvouched_carryover` guard from `apply_carryover_writeback` ⇒ the write-back
///     assertion below reds;
///   * make `cwa_unvouched_carryover` return `None` unconditionally ⇒ both halves red.
#[test]
fn the_standard_deduction_deferral_donor_still_files_but_is_neither_told_nothing_nor_written_back()
{
    use btctax_core::tax::return_1040::cwa_unvouched_carryover;

    let params = ty2024_params();
    let table = ty2024_table();
    let (mut ri, mut state) = household(r#"{"filing_status":"Single","w2_income":40000}"#);
    state.removals.push(ledger_gift(dec!(25000)));
    // ★★ THE RESTRICTION QUESTION IS ANSWERED, DELIBERATELY. A $25,000 property gift is over the
    //    §170(f)(11)(D) $5,000 appraisal threshold, so it files a Form 8283 SECTION B — and the I-2
    //    sibling gate in `apply_carryover_writeback` refuses an UNANSWERED 5a/5b/5c before this one
    //    is ever reached. Leaving it `None` would have produced a green test whose refusal came from
    //    a completely different statute: the KAT-8 vacuity defect, one finding over. Answering it
    //    `Some(false)` satisfies that gate and makes §170(f)(8) the BINDING reason, which is asserted
    //    on the message below.
    ri.donations_had_restrictions = Some(false);

    // ── THE PREMISES. Each is asserted, so this KAT cannot pass by never reaching the population. ──
    assert!(
        ri.charitable_cwa_obtained.is_none(),
        "the acknowledgment must be UNANSWERED, or there is nothing unvouched-for to find"
    );
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    assert!(
        !ar.deduction_is_itemized,
        "the election must be STANDARD — $12,000 of allowed gift loses to the $14,600 standard \
         deduction. This is the branch R2 declared safe."
    );
    let deferred: Usd = ar
        .charitable_carryover_out
        .iter()
        .filter(|c| c.origin_year == 2024)
        .map(|c| c.amount)
        .sum();
    assert_eq!(
        deferred,
        dec!(13000),
        "…and the gift must DEFER: §170(b) CapGainProp30 ceiling = 30% × $40,000 = $12,000, so \
         $13,000 of the $25,000 carries. This is the branch R3 was about."
    );

    // ── HALF 1: THE RETURN STILL FILES. Widening the refusal would have broken this. ───────────────
    assert!(
        screen_inputs(&ri, &table, &params).is_none(),
        "nothing refuses at the input screen"
    );
    assert!(
        screen_absolute(&ri, &ar, &params, &state, 2024).is_none(),
        "★ and the RETURN IS STILL FILABLE. It claims no §170 deduction, so §170(f)(8)(A) — which \
         conditions *a deduction* — denies nothing on it. If this ever refuses, the fix went in as a \
         wider gate and the `Some(false)` cure (\"remove that gift from the deduction\") is now \
         being offered to a filer with no deduction to remove."
    );

    // ── HALF 2: THE FILER IS TOLD, BEFORE FILING. ─────────────────────────────────────────────────
    assert_eq!(
        cwa_unvouched_carryover(&ri, &ar, &state, 2024),
        Some(dec!(13000)),
        "★ the roll-forward block's §170(f)(8) warning is keyed to this, and it must name the \
         DEFERRED amount — the whole $25,000 is not carrying, $13,000 is"
    );

    // ── HALF 3: THE WRITE-BACK REFUSES TO LAUNDER IT INTO NEXT YEAR'S INPUTS. ─────────────────────
    let err = btctax_core::tax::return_1040::apply_carryover_writeback(
        &ar,
        &ri,
        &state,
        2024,
        ReturnInputs::default(),
        false,
    )
    .expect_err(
        "★ `--write-carryover` must REFUSE: stamping this `Computed` into 2025's inputs puts it \
         beyond every check, because 2025's line-13 claim is deliberately outside the P4 gate",
    );
    assert!(
        err.contains("contemporaneous written acknowledgment") && err.contains("170(f)(8)(A)"),
        "the refusal must name the statute the filer has to satisfy. Got: {err}"
    );
    assert!(
        err.contains("STANDARD deduction"),
        "…and must explain why NOTHING refused at filing time, or the filer reads it as a \
         contradiction of the return they just filed. Got: {err}"
    );

    // ── AND `--force` DOES NOT OPEN IT. Same as its I-2 sibling: force overwrites a USER figure. ──
    assert!(
        btctax_core::tax::return_1040::apply_carryover_writeback(
            &ar,
            &ri,
            &state,
            2024,
            ReturnInputs::default(),
            true,
        )
        .is_err(),
        "`--force` overrides the user-provenance guard, never a vouch-for guard"
    );

    // ══ THE DISCRIMINATING TWIN: answer YES, and everything proceeds. ═════════════════════════════
    //
    // Without this, the three assertions above are satisfied by a guard that refuses EVERYONE with a
    // carryover — the green-and-blind instrument. The ANSWER must separate.
    let (mut ri_yes, mut state_yes) = household(r#"{"filing_status":"Single","w2_income":40000}"#);
    state_yes.removals.push(ledger_gift(dec!(25000)));
    ri_yes.donations_had_restrictions = Some(false); // same premise as above — one variable differs
    ri_yes.charitable_cwa_obtained = Some(true);
    let ar_yes = assemble_absolute(&ri_yes, &state_yes, &params, &table, 2024);
    assert_eq!(
        cwa_unvouched_carryover(&ri_yes, &ar_yes, &state_yes, 2024),
        None,
        "a filer who holds the acknowledgment is vouched-for and must NOT be warned"
    );
    let next = btctax_core::tax::return_1040::apply_carryover_writeback(
        &ar_yes,
        &ri_yes,
        &state_yes,
        2024,
        ReturnInputs::default(),
        false,
    )
    .expect("…and their write-back must SUCCEED — the same gift, the same year, one answer apart");
    assert_eq!(
        next.charitable_carryover_in
            .iter()
            .map(|c| c.amount)
            .sum::<Usd>(),
        dec!(13000),
        "…rolling the identical $13,000 the refused filer was denied"
    );

    // ══ THE SECOND TWIN: a gift UNDER $250 is never in scope at all. ══════════════════════════════
    //
    // §170(f)(8)(A) is per contribution and starts at $250; a $200 gift needs no acknowledgment. A
    // guard keyed to "has a carryover" rather than to the threshold would catch this filer too.
    let (ri_small, mut state_small) = household(r#"{"filing_status":"Single","w2_income":200}"#);
    state_small.removals.push(ledger_gift(dec!(200)));
    let ar_small = assemble_absolute(&ri_small, &state_small, &params, &table, 2024);
    let deferred_small: Usd = ar_small
        .charitable_carryover_out
        .iter()
        .filter(|c| c.origin_year == 2024)
        .map(|c| c.amount)
        .sum();
    assert!(
        deferred_small > Usd::ZERO,
        "the small-gift twin must ALSO defer (30% × $200 = $60 ceiling), or it discriminates the \
         wrong variable — it has to differ from the case above ONLY in the $250 threshold"
    );
    assert_eq!(
        cwa_unvouched_carryover(&ri_small, &ar_small, &state_small, 2024),
        None,
        "★ a $200 gift is under §170(f)(8)(A)'s $250 threshold — i1040sca: \"don't combine separate \
         donations\" — so it needs no acknowledgment and must not be warned about or refused"
    );
}

/// ★★★ **MIXED-USE × OVER-LIMIT: THE INTERSECTION WHERE ALL THREE ANSWERS REFUSED.**
///
/// `MortgageAllUsedToBuyBuildImprove == Some(false)` (base question tree, §2.7) already zeroes
/// Schedule A line 8a and CHECKS THE LINE-8 BOX — a disclosed conservative zero. The phase-1
/// `MortgageOverDebtLimit` refusal fired independently on `mortgage_within_debt_limit == Some(false)`
/// without checking whether 8a was already $0, so for that filer:
///
///   * `None`        ⇒ `MortgageDebtLimitUnanswered` (input screen)
///   * `Some(false)` ⇒ `MortgageOverDebtLimit`
///   * `Some(true)`  ⇒ false testimony under §6065
///
/// …no honest answer, on a return btctax can compute today. And both premises the refusal STATES are
/// false for them: btctax would deduct $0 rather than the full 1098 figure, and the box that
/// "does not exist" is checked on their own Schedule A.
///
/// ★★ **THE PROOF THAT THE ANSWER MOVES NOTHING** is the third assertion below: the return printed
/// for the over-limit mixed-use filer is line-for-line the return printed when the same filer answers
/// the debt-limit question YES. That is why scoping the refusal is not weakening it — there was
/// nothing for it to protect here.
///
/// **Plant:** delete `&& mixed_use_mortgage_forgone(ri).is_none()` from the `MortgageOverDebtLimit`
/// gate ⇒ the first half reds. Widen it to fire on every itemizer ⇒ the twin reds.
#[test]
fn a_mixed_use_mortgage_that_is_also_over_the_debt_limit_files_a_disclosed_zero() {
    let params = ty2024_params();
    let table = ty2024_table();
    let base = r#"{"filing_status":"Single","w2_income":300000,"mortgage_interest":45000,
                   "state_income_tax":12000,"charitable_cash":20000}"#;

    let build = |mixed: Option<bool>, within: Option<bool>| {
        let (mut ri, state) = household(base);
        {
            let a = ri.schedule_a.as_mut().expect("the fixture itemizes");
            a.mortgage_all_used_to_buy_build_improve = mixed;
            a.mortgage_within_debt_limit = within;
        }
        // §170(f)(8): a $20,000 cash gift on an ITEMIZING return trips the acknowledgment gate, which
        // is a different statute entirely. Answer it so the mortgage question is the binding one.
        ri.charitable_cwa_obtained = Some(true);
        answer_all_live_declarations(&mut ri);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        (ri, state, ar)
    };

    // ── THE FILER FROM THE FINDING: mixed-use AND over the limit. ──────────────────────────────────
    let (ri, state, ar) = build(Some(false), Some(false));

    // Premises first, so this KAT cannot pass by never reaching the intersection.
    assert!(
        ar.deduction_is_itemized,
        "the fixture must ITEMIZE — the refusal under test is gated on the election, and a \
         standard-deduction filer never reaches it"
    );
    let sa = ar
        .schedule_a
        .as_ref()
        .expect("an itemizing return has a Schedule A");
    assert_eq!(
        sa.mortgage_8a,
        Usd::ZERO,
        "★ line 8a must ALREADY be zero — that is the whole premise: the mixed-use answer forgoes \
         the entire 1098 amount before the debt limit is ever consulted"
    );
    assert!(
        sa.mortgage_mixed_use_box,
        "★ …and the line-8 box must be CHECKED, which is what makes the zero DISCLOSED rather than \
         silent. The refusal's text says no such box exists; on this return it does."
    );

    assert_eq!(
        screen_inputs(&ri, &table, &params),
        None,
        "nothing refuses at the input screen once the question is answered"
    );
    assert_eq!(
        screen_absolute(&ri, &ar, &params, &state, 2024),
        None,
        "★★ AND THE RETURN FILES. Refusing here stopped a filer whose printed return btctax can \
         compute honestly today, and offered them no honest answer: `None` refuses as unanswered, \
         `Some(true)` is false testimony under §6065, and this was the third door."
    );

    // ── THE ANSWER MOVES NOTHING: the same filer answering YES prints the identical return. ────────
    let (_, _, ar_yes) = build(Some(false), Some(true));
    assert_eq!(
        ar.schedule_a, ar_yes.schedule_a,
        "★ the over-limit fact changes NO figure on this return — the whole 1098 amount is already \
         forgone by the mixed-use answer, so 8a is $0 and the box is checked either way. A refusal \
         defending a number that cannot move is defending nothing."
    );
    assert_eq!(
        ar.taxable_income, ar_yes.taxable_income,
        "…all the way down to taxable income"
    );

    // ══ TWIN 1: OVER-LIMIT WITHOUT MIXED-USE STILL REFUSES. ═══════════════════════════════════════
    //
    // Without this the scope above is satisfied by deleting the refusal outright. Here 8a really
    // WOULD carry the full $45,000 with nothing on the form to disclose it — the case the refusal's
    // text describes, and the case it exists for.
    let (ri_x, state_x, ar_x) = build(Some(true), Some(false));
    let sa_x = ar_x.schedule_a.as_ref().expect("itemizing");
    assert_eq!(
        sa_x.mortgage_8a,
        dec!(45000),
        "the twin must carry the FULL 1098 figure on 8a, or it is not the population the refusal \
         protects"
    );
    assert!(
        !sa_x.mortgage_mixed_use_box,
        "…with the line-8 box UNCHECKED — the undisclosed zero problem the refusal's text names"
    );
    let r = screen_absolute(&ri_x, &ar_x, &params, &state_x, 2024)
        .expect("★ an over-limit itemizer whose 8a is NOT already zeroed must still REFUSE");
    assert_eq!(
        format!("{:?}", r.reason),
        "MortgageOverDebtLimit",
        "…and for the debt limit specifically"
    );

    // ══ TWIN 2: MIXED-USE ALONE, NOT OVER THE LIMIT — the control. ════════════════════════════════
    //
    // Proves the first case files because of the SCOPE and not because the fixture stopped being
    // over the limit: this filer prints the same disclosed zero and always could.
    let (ri_m, state_m, ar_m) = build(Some(false), Some(true));
    assert_eq!(
        screen_absolute(&ri_m, &ar_m, &params, &state_m, 2024),
        None,
        "a mixed-use filer inside the debt limit files a disclosed zero — this was never in doubt, \
         and it is the return the over-limit filer now gets too"
    );
}
