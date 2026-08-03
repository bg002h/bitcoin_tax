//! §G-28/B2 + §G-6 — the NINE-DEPENDENT scenario, kept executable so it cannot rot.
//!
//! One return that exercises four things v0.14.0 refused or could not express: more than four
//! dependents, non-ledger Schedule C receipts, Form 1099-B totals, and an AMT that must be attached
//! alongside a Form 8995-A. A fixture nobody runs is a fixture that quietly stops describing the
//! program, so the input lives in `fixtures/examples/nine_dependents_amt_inputs.toml` and this drives
//! it through the real assembly path.

use btctax_core::tax::packet::assemble_printed_return;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("fixtures/examples/nine_dependents_amt_inputs.toml");

fn load() -> ReturnInputs {
    let mut ri: ReturnInputs = toml::from_str(FIXTURE)
        .expect("the committed nine-dependent fixture parses as ReturnInputs");
    ri.tax_year = 2024;
    ri
}

/// ★★★ The scenario FILES, and files the six things it exists to pin.
#[test]
fn the_nine_dependent_amt_return_files_a_complete_packet() {
    let ri = load();
    assert_eq!(ri.header.dependents.len(), 9, "the fixture's whole point");

    let state = Default::default();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
    let pr = assemble_printed_return(
        &ri,
        &state,
        &BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2024,
        &[],
    )
    .expect("this return must FILE — every one of its four features was a refusal before v0.15.0");

    // §G-6 — Form 6251 is attached, and Schedule 2 carries it.
    let f6251 = pr
        .forms
        .f6251
        .as_ref()
        .expect("AMT: line 7 exceeds line 10");
    assert_eq!(
        btctax_core::conventions::round_dollar(f6251.line11),
        dec!(9077),
        "the AMT both oracles witnessed on this vector"
    );
    let sch2 = pr.forms.sch_2.as_ref().expect("Schedule 2 files");
    assert_eq!(
        sch2.line2,
        Some(dec!(9077)),
        "Sch 2 L2 transcribes 6251 L11"
    );

    // §G-28/B1b — above the §199A threshold, so 8995-A governs and the simplified 8995 is absent.
    assert!(
        pr.forms.f8995a.is_some(),
        "Form 8995-A files above the threshold"
    );
    assert!(
        pr.forms.f8995.is_none(),
        "the two are ALTERNATIVES; filing both would claim the deduction twice on paper"
    );

    // ★ Total tax, pinned to the oracle-validated figure. Dependents do not move it — see below.
    assert_eq!(pr.forms.f1040.line24, dec!(492035));

    // ★★ Schedule B files ($45,000 of interest clears the $1,500 floor) and so does Schedule A --
    //    $18,000 of mortgage interest CLEARS the $14,600 standard deduction, so the filer itemizes.
    assert!(
        pr.forms.sch_b.is_some(),
        "interest over $1,500 files Schedule B"
    );
    let sch_a = pr
        .forms
        .sch_a
        .as_ref()
        .expect("$18,000 itemized beats the $14,600 standard deduction");
    assert_eq!(sch_a.line8a, dec!(18000), "Form 1098 mortgage interest");
    assert_eq!(
        pr.forms.f1040.line12,
        dec!(18000),
        "1040 L12 is the itemized total"
    );

    // ★★★ FORM 6251 LINE 2a IS ZERO, AND THAT IS THE WHOLE POINT OF ITEMIZING HERE.
    //     "If filing Schedule A, enter the taxes from Schedule A, **line 7**; otherwise, enter the
    //     amount from Form 1040 line 12." This filer reports no taxes, so line 2a is $0 -- NOT the
    //     $18,000 deduction. Conflating Schedule A line 7 (taxes) with line 17 (the itemized total)
    //     is the defect that shipped in v0.6.0-v0.13.0, and a standard-deduction fixture cannot
    //     exercise the branch it lived on.
    assert_eq!(
        f6251.line2a,
        btctax_core::conventions::Usd::ZERO,
        "an itemizer adds back TAXES (Sch A line 7 = 0), never the itemized total"
    );
    assert_eq!(
        btctax_core::conventions::round_dollar(f6251.line4),
        dec!(2105995),
        "AMTI = taxable income + line 2a(0)"
    );
}

/// ★★★ THE CTC ADVISORY — this fixture is also the regression case for it.
///
/// At this income §24(b) has phased the Child Tax Credit out entirely, so 1040 line 19 is $0 AND that
/// is the correct figure. v0.15.0 nonetheless told this filer their tax was "OVERSTATED by up to
/// $2,000 per qualifying child" — up to $18,000 — and sent them to file a Schedule 8812 that would
/// have paid them nothing. The provable-zero branch existed and was tested, but production could not
/// reach it: it needed a gate that is `live: |ri| ri.tax_year >= 2025` and so is never asked on the
/// only year btctax can file.
#[test]
fn the_ctc_advisory_tells_this_filer_the_credit_is_gone_not_that_it_is_owed() {
    let ri = load();
    let state = Default::default();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
    let msgs: Vec<String> =
        btctax_core::tax::advisories::advisories_for(&ri, &state, &ar, &ty2024_params(), 2024)
            .iter()
            .map(|a| a.message())
            .collect();
    let ctc = msgs
        .iter()
        .find(|m| m.contains("CTC/ODC"))
        .expect("nine dependents must produce a CTC advisory of some kind");
    assert!(
        ctc.contains("NOT AVAILABLE TO YOU") && ctc.contains("costs you NOTHING"),
        "the filer must be told the credit is phased out, not sent to Schedule 8812: {ctc}"
    );
    assert!(
        !ctc.contains("OVERSTATED"),
        "nothing is overstated — §24(b) removed the credit entirely: {ctc}"
    );
}
