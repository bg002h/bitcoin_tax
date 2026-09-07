//! ★★★ **R4 — THE CHECKS THE DOCUMENT ITSELF GUARANTEES, DISPLAYED AND NEVER WRITTEN.**
//!
//! R4: *"Checks the tool may run and **display**: W-2 box 4 ≈ 6.2% × box 3, box 6 ≈ 1.45% × box 5,
//! box 3 ≤ the wage base — **warnings**, typo detectors; the filer corrects the box, the tool never
//! writes it."*
//!
//! ## Why a warning and not a refusal, and why not a correction
//!
//! Each of these is arithmetic the ISSUER performed and printed. When a transcribed row breaks it,
//! the overwhelmingly likely cause is a **typing slip** — box 1 typed into box 3 — and the second
//! most likely is a genuinely unusual W-2 the tool has no business overruling (a corrected form, a
//! mid-year successor employer, a §3121 exemption). So:
//!
//! - **Never a refusal.** A refusal on a lawful edge case is a brick, and every one of these has an
//!   edge case. The figures still reach their lines exactly as transcribed.
//! - **Never a correction.** *The filer corrects the box.* A tool that "fixed" box 3 would be
//!   writing testimony the filer never gave — the whole class this codebase exists to refuse — and
//!   would make the printed return disagree with the paper it was copied from.
//!
//! ## The all-zero row
//!
//! R4's second warning has a different mechanism and is worth stating separately: *"a row whose
//! every income box is zero is most likely a row the filer began and did not finish — a payer issues
//! a 1099-INT at $10 or more."* The **threshold is the ISSUER'S**, printed in that form's own filing
//! instructions, which is why it applies to the information returns and NOT to the Form W-2: an
//! employer must issue a W-2 whatever the wages, so an all-zero W-2 is unremarkable and warning
//! about it would train the filer to ignore the whole class.
//!
//! ★ Nothing here reads [`crate::tax::tables::FullReturnParams`], and only the wage-base check reads
//!   the year's [`crate::tax::tables::TaxTable`] — passed as an `Option`, so the whole set runs while
//!   the filer is editing a year whose package has not arrived (R11).

use crate::conventions::Usd;
use crate::tax::return_inputs::ReturnInputs;
use rust_decimal_macros::dec;

/// §3101(a) — the employee OASDI rate box 4 withholds at.
const OASDI_RATE: Usd = dec!(0.062);
/// §3101(b)(1) — the employee Medicare rate box 6 withholds at.
const MEDICARE_RATE: Usd = dec!(0.0145);
/// §3101(b)(2) — Medicare plus the 0.9% Additional Medicare Tax, which is ALSO withheld into box 6.
///
/// ★ Used as an UPPER BOUND rather than as a second exact rule, so the check needs no $200,000
///   threshold of its own: the surtax applies only to the excess over that figure, so a box 6 above
///   `2.35% × box 5` cannot be explained by it at any wage.
const MEDICARE_PLUS_SURTAX_RATE: Usd = dec!(0.0235);

/// The slack a rounded figure is allowed before it is called a slip: one dollar, or a tenth of a
/// percent of the expected amount, whichever is larger.
///
/// ★ Deliberately loose. These exist to catch a figure in the WRONG BOX — an error of tens of
///   thousands — never a cent of rounding, and a check that cries on a rounding difference is a
///   check people learn to skip.
fn tolerance(expected: Usd) -> Usd {
    let proportional = expected.abs() * dec!(0.001);
    if proportional > dec!(1) {
        proportional
    } else {
        dec!(1)
    }
}

/// Which transcribed row a warning is about — the section a renderer must put the filer's cursor on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarnedDocument {
    W2,
    Form1099Int,
    Form1099Div,
    Form1099G,
    Form1098E,
}

impl WarnedDocument {
    /// The document's IRS designation, for the message.
    #[must_use]
    pub const fn designation(self) -> &'static str {
        match self {
            WarnedDocument::W2 => "Form W-2",
            WarnedDocument::Form1099Int => "Form 1099-INT",
            WarnedDocument::Form1099Div => "Form 1099-DIV",
            WarnedDocument::Form1099G => "Form 1099-G",
            WarnedDocument::Form1098E => "Form 1098-E",
        }
    }
}

/// One displayed warning about one transcribed row. It carries no severity and no remedy the tool
/// can apply: the only action is the filer re-reading the paper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionWarning {
    pub document: WarnedDocument,
    /// The row's index in its `Vec`, so a renderer can point at it. Zero-based; messages print it
    /// one-based, because the filer counts documents from one.
    pub row: usize,
    pub message: String,
}

/// ★★★ **Every transcription warning this return raises, in row order. NOTHING IS WRITTEN.**
///
/// `ss_wage_base` is the year's [`crate::tax::tables::TaxTable::ss_wage_base`], or `None` on a year
/// whose package has not arrived — the box-3 ceiling check is then simply not run, exactly like
/// every other params-gated rule (R11), rather than guessing a base.
#[must_use]
pub fn transcription_warnings(
    ri: &ReturnInputs,
    ss_wage_base: Option<Usd>,
) -> Vec<TranscriptionWarning> {
    let mut out = Vec::new();
    let mut warn = |document: WarnedDocument, row: usize, message: String| {
        out.push(TranscriptionWarning {
            document,
            row,
            message,
        });
    };

    for (i, w) in ri.w2s.iter().enumerate() {
        let who = if w.employer.trim().is_empty() {
            format!("Form W-2 #{}", i + 1)
        } else {
            format!("the Form W-2 from {}", w.employer.trim())
        };
        // ── box 4 ≈ 6.2% × box 3 ──────────────────────────────────────────────────────────────
        let expected4 = w.box3_ss_wages * OASDI_RATE;
        if (w.box4_ss_withheld - expected4).abs() > tolerance(expected4) {
            warn(
                WarnedDocument::W2,
                i,
                format!(
                    "{who}: box 4 (Social security tax withheld) is {}, but 6.2% of box 3 \
                     (Social security wages, {}) is {}. Employers withhold box 4 at exactly that \
                     rate, so one of the two boxes is probably mistyped — check the paper. btctax \
                     has changed nothing.",
                    money(w.box4_ss_withheld),
                    money(w.box3_ss_wages),
                    money(expected4)
                ),
            );
        }
        // ── box 6 between 1.45% and 2.35% of box 5 ────────────────────────────────────────────
        let low = w.box5_medicare_wages * MEDICARE_RATE;
        let high = w.box5_medicare_wages * MEDICARE_PLUS_SURTAX_RATE;
        if w.box6_medicare_withheld < low - tolerance(low)
            || w.box6_medicare_withheld > high + tolerance(high)
        {
            warn(
                WarnedDocument::W2,
                i,
                format!(
                    "{who}: box 6 (Medicare tax withheld) is {}, and 1.45% of box 5 (Medicare \
                     wages and tips, {}) is {}. Box 6 also carries the 0.9% Additional Medicare \
                     Tax on wages over $200,000, so anything between {} and {} is ordinary — this \
                     is outside that. Check the paper; btctax has changed nothing.",
                    money(w.box6_medicare_withheld),
                    money(w.box5_medicare_wages),
                    money(low),
                    money(low),
                    money(high)
                ),
            );
        }
        // ── box 3 ≤ the year's Social Security wage base ──────────────────────────────────────
        if let Some(base) = ss_wage_base {
            if w.box3_ss_wages > base {
                warn(
                    WarnedDocument::W2,
                    i,
                    format!(
                        "{who}: box 3 (Social security wages) is {}, which is above this year's \
                         Social Security wage base of {}. No single employer reports more than the \
                         base in box 3 — box 1 is the usual figure typed here by mistake. Check the \
                         paper; btctax has changed nothing.",
                        money(w.box3_ss_wages),
                        money(base)
                    ),
                );
            }
        }
    }

    // ── The all-zero row, on the documents whose ISSUER has a reporting threshold. ────────────
    for (i, r) in ri.int_1099.iter().enumerate() {
        let income = r.box1_interest
            + r.box3_treasury_interest
            + r.box8_tax_exempt_interest
            + r.box10_market_discount;
        if income == Usd::ZERO {
            warn(
                WarnedDocument::Form1099Int,
                i,
                all_zero(
                    WarnedDocument::Form1099Int,
                    i,
                    &r.payer,
                    "$10 or more of interest",
                ),
            );
        }
    }
    for (i, r) in ri.div_1099.iter().enumerate() {
        let income = r.box1a_ordinary + r.box2a_capgain_distr + r.box12_exempt_interest_dividends;
        if income == Usd::ZERO {
            warn(
                WarnedDocument::Form1099Div,
                i,
                all_zero(
                    WarnedDocument::Form1099Div,
                    i,
                    &r.payer,
                    "$10 or more of dividends",
                ),
            );
        }
    }
    for (i, r) in ri.g_1099.iter().enumerate() {
        let income = r.box1_unemployment + r.box2_state_refund + r.box10_family_leave_benefits;
        if income == Usd::ZERO {
            warn(
                WarnedDocument::Form1099G,
                i,
                all_zero(
                    WarnedDocument::Form1099G,
                    i,
                    &r.payer,
                    "$10 or more of unemployment compensation, or any refund of state or local \
                     income tax",
                ),
            );
        }
    }
    for (i, r) in ri.form_1098e.iter().enumerate() {
        if r.box1_interest == Usd::ZERO {
            warn(
                WarnedDocument::Form1098E,
                i,
                all_zero(
                    WarnedDocument::Form1098E,
                    i,
                    &r.lender,
                    "$600 or more of student loan interest",
                ),
            );
        }
    }
    out
}

/// The all-zero-row message, one sentence per document, naming that issuer's own threshold.
fn all_zero(doc: WarnedDocument, i: usize, issuer: &str, threshold: &str) -> String {
    let named = if issuer.trim().is_empty() {
        format!("{} #{}", doc.designation(), i + 1)
    } else {
        format!("the {} from {}", doc.designation(), issuer.trim())
    };
    format!(
        "{named}: every income box on this row is zero. An issuer sends a {} only for {threshold}, \
         so a row with nothing on it is most likely one you began and did not finish — check the \
         paper, or remove the row. btctax has changed nothing.",
        doc.designation()
    )
}

/// Dollars, for a message — two decimals, with a leading `$`.
fn money(v: Usd) -> String {
    format!("${v:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::{Form1098E, Form1099Div, Form1099G, Form1099Int, W2};

    fn w2(f: impl FnOnce(&mut W2)) -> W2 {
        let mut w = W2 {
            employer: "ACME".into(),
            box1_wages: dec!(100000),
            box3_ss_wages: dec!(100000),
            box4_ss_withheld: dec!(6200),
            box5_medicare_wages: dec!(100000),
            box6_medicare_withheld: dec!(1450),
            ..Default::default()
        };
        f(&mut w);
        w
    }

    /// A correctly transcribed W-2 raises nothing — or every other assertion here is vacuous.
    #[test]
    fn a_correct_w2_raises_no_warning() {
        let ri = ReturnInputs {
            w2s: vec![w2(|_| {})],
            ..Default::default()
        };
        assert_eq!(
            transcription_warnings(&ri, Some(dec!(168600))),
            Vec::new(),
            "a W-2 whose boxes agree must be silent"
        );
    }

    /// ★★★ **THE PLANTED SLIP R4 NAMES: box 1 typed into box 3.** It fires all three W-2 checks at
    ///     once on a real household — the wages are over the base, so box 3 exceeds it, and box 4
    ///     (correctly 6.2% of the REAL Social Security wages) no longer matches the inflated box 3.
    ///
    /// ★ And the kill that matters more than the firing: **the row is byte-identical afterwards.**
    ///   A "warning" that quietly corrected a box would be writing testimony the filer never gave.
    #[test]
    fn a_box_1_in_box_3_slip_warns_and_writes_nothing() {
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                w.box1_wages = dec!(250000);
                w.box3_ss_wages = dec!(250000); // ← the slip: the wage base caps it at 168,600
                w.box4_ss_withheld = dec!(10453.20); // 6.2% of the REAL 168,600
                w.box5_medicare_wages = dec!(250000);
                w.box6_medicare_withheld = dec!(4075); // 1.45% + the 0.9% surtax over 200,000
            })],
            ..Default::default()
        };
        let before = ri.clone();
        let got = transcription_warnings(&ri, Some(dec!(168600)));
        assert_eq!(
            got.len(),
            2,
            "the slip breaks the box-4 rate AND the wage base: {got:#?}"
        );
        assert!(
            got.iter().any(|w| w.message.contains("wage base")),
            "the wage-base ceiling must be named: {got:#?}"
        );
        assert!(
            got.iter().any(|w| w.message.contains("6.2% of box 3")),
            "the box-4 rate must be named: {got:#?}"
        );
        assert!(
            got.iter()
                .all(|w| w.document == WarnedDocument::W2 && w.row == 0),
            "each warning must name the ROW the filer has to open: {got:#?}"
        );
        // ★★★ THE KILL: nothing was written.
        assert_eq!(ri, before, "a warning must never change a stored value");
    }

    /// The 0.9% Additional Medicare Tax lives in box 6 too, so a high earner is NOT warned — the
    /// check brackets [1.45%, 2.35%] rather than pinning one rate. Without this the warning would
    /// fire on every filer over $200,000 and be trained away.
    #[test]
    fn the_additional_medicare_tax_in_box_6_is_not_a_slip() {
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                w.box5_medicare_wages = dec!(300000);
                // 1.45% × 300,000 + 0.9% × 100,000 = 4,350 + 900
                w.box6_medicare_withheld = dec!(5250);
                w.box3_ss_wages = dec!(168600);
                w.box4_ss_withheld = dec!(10453.20);
            })],
            ..Default::default()
        };
        assert_eq!(transcription_warnings(&ri, Some(dec!(168600))), Vec::new());
        // …and a box 6 outside the bracket still warns, so the widening did not disarm the check.
        let mut broken = ri.clone();
        broken.w2s[0].box6_medicare_withheld = dec!(30000);
        let got = transcription_warnings(&broken, Some(dec!(168600)));
        assert_eq!(got.len(), 1, "{got:#?}");
        assert!(got[0].message.contains("Additional Medicare Tax"));
    }

    /// ★ The wage-base check is PARAMS-GATED: on a year whose table has not arrived it does not run,
    ///   rather than guessing a base. The other two still do.
    #[test]
    fn the_wage_base_check_waits_for_the_years_table() {
        let ri = ReturnInputs {
            w2s: vec![w2(|w| {
                w.box3_ss_wages = dec!(9999999);
                w.box4_ss_withheld = dec!(619999.94); // 6.2%, so only the base check can fire
            })],
            ..Default::default()
        };
        assert_eq!(
            transcription_warnings(&ri, None),
            Vec::new(),
            "with no table there is no base to compare against"
        );
        assert_eq!(transcription_warnings(&ri, Some(dec!(168600))).len(), 1);
    }

    /// ★★★ **THE ALL-ZERO ROW, on every document whose ISSUER has a threshold — and not on the W-2,
    ///     whose employer must issue one whatever the wages.**
    #[test]
    fn an_all_zero_row_warns_on_the_information_returns_and_never_on_a_w2() {
        let ri = ReturnInputs {
            w2s: vec![W2 {
                employer: "ACME".into(),
                ..Default::default()
            }],
            int_1099: vec![Form1099Int {
                payer: "First Bank".into(),
                box4_fed_withheld: dec!(5), // withholding is NOT income: the row is still empty
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                payer: "Fund".into(),
                ..Default::default()
            }],
            g_1099: vec![Form1099G {
                payer: "State".into(),
                ..Default::default()
            }],
            form_1098e: vec![Form1098E {
                lender: "Servicer".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let before = ri.clone();
        let got = transcription_warnings(&ri, Some(dec!(168600)));
        let docs: Vec<WarnedDocument> = got.iter().map(|w| w.document).collect();
        assert_eq!(
            docs,
            vec![
                WarnedDocument::Form1099Int,
                WarnedDocument::Form1099Div,
                WarnedDocument::Form1099G,
                WarnedDocument::Form1098E
            ],
            "an all-zero W-2 is ordinary (an employer issues one at any wage) and must NOT warn; \
             each information return must: {got:#?}"
        );
        assert!(
            got[0].message.contains("$10 or more of interest"),
            "the message names the ISSUER'S own threshold: {}",
            got[0].message
        );
        // ★★★ THE KILL: nothing was written.
        assert_eq!(ri, before, "a warning must never change a stored value");
    }

    /// A row with an income box filled is silent — otherwise the all-zero check is just "warn on
    /// every row" and the fixture above proves nothing.
    #[test]
    fn a_populated_information_return_row_is_silent() {
        let ri = ReturnInputs {
            int_1099: vec![Form1099Int {
                payer: "First Bank".into(),
                box10_market_discount: dec!(12), // ★ box 10 alone counts as income
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(transcription_warnings(&ri, None), Vec::new());
    }
}
