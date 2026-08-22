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
    ty2024_params, ty2024_table, GoldenInputs,
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
    let params = ty2024_params();
    let table = ty2024_table();
    assert!(
        screen_inputs(ri, &table, &params).is_none(),
        "this fixture must FILE, but screen_inputs refused: {:?}",
        screen_inputs(ri, &table, &params).map(|r| r.reason)
    );
    let ar = assemble_absolute(ri, state, &params, &table, 2024);
    assert!(
        screen_absolute(ri, &ar, &params, state, 2024).is_none(),
        "this fixture must FILE, but screen_absolute refused: {:?}",
        screen_absolute(ri, &ar, &params, state, 2024).map(|r| r.reason)
    );
    let pr = assemble_printed_return(ri, state, &BTreeMap::new(), &ar, &table, 2024, &[])
        .expect("the fixture carries a well-formed SSN");
    let packet = fill_full_return(&pr, 2024).expect("the packet must fill");
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
    let f1040 = cells(&filed.forms, "f1040", F1040_MAP_2024);
    let schd = cells(&filed.forms, "schedule_d", SCHEDULE_D_MAP_2024);

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
    let f1040 = cells(&filed.forms, "f1040", F1040_MAP_2024);
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
/// ★★ Same for the P1 mortgage ceiling and the Form 4952 line-9 bound, which phase 2's review C
/// re-scoped to returns that DEDUCT: this filer has neither, and must not be caught by either.
///
/// **If this KAT ever shows L7 being asked the acknowledgment question, that is a P4 liveness defect
/// — the KAT is not to be adjusted to accept it.**
#[test]
fn a_five_thousand_dollar_gift_under_the_standard_deduction_files_no_schedule_a_and_no_8283() {
    let mut i = zero_inputs("Single");
    i.w2_income = 30_000.0;
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
        "this household must take the STANDARD deduction — a $5,000 gift does not clear $14,600, and \
         if it ever did the whole KAT would be testing the other branch"
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
    let sch_a = cells(&twin.forms, "f1040sa", SCHEDULE_A_MAP_2024);
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
    let refusal = screen_absolute(&ri, &ar, &params, &state, 2024)
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
    ("line19", "0"),     // CTC / credit for other dependents
    ("line20", "0"),     // Schedule 3 line 8
    ("line21", "0"),     // add 19 and 20
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

    let f1040 = cells(&filed.forms, "f1040", F1040_MAP_2024);

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
        "the all-zero 1040 stopped printing {vanished:?}. Each is a line the form's arithmetic \
         reaches — 'add lines …' with nothing to add is still a computed zero, and the cell is where \
         a reader looks to see the return was completed rather than abandoned."
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
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// KAT 5 — E4: every Form 6251 cell, read back off the PDF, against the struct that produced it.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The 41 modelled Form 6251 cells, each paired with the `Form6251::printed()` field it must carry.
///
/// ★ A hand-written pairing is exactly what is under test here — the map says which AcroForm widget
/// line N lives in, and this says which VALUE belongs in line N. Its completeness is not taken on
/// trust: the test asserts the set of cells checked equals `Form6251Map::money_cells()`, so a line
/// added to the form cannot be silently skipped.
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
    let paper = extract_lines(&pdf, F6251_MAP_2024).expect("the filled 6251 transcribes");
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
    let f6251 = cells(&filed.forms, "f6251", F6251_MAP_2024);
    let sch2 = cells(&filed.forms, "f1040s2", SCHEDULE_2_MAP_2024);
    let f1040 = cells(&filed.forms, "f1040", F1040_MAP_2024);

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
