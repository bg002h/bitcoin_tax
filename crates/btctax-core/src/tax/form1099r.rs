//! **Form 1099-R — the transcription, all 21 printed boxes, one entry per box.**
//!
//! `design/ty2025/SPEC_retirement_income.md` §7 carried **five** of them (1, 2a, 2b, 4, 7) as a
//! hand-written list. Review r1's I-10 found that no primary source for this document was archived at
//! all, and predicted that deriving the box set from the form would surface omissions. It did, on the
//! first pass: **box 14, *State tax withheld*, feeds Schedule A line 5a** and was absent — the same
//! money defect interview T11 had already fixed for W-2 boxes 17 and 19 ([`FR-91`], and [`FR-258`] for
//! the rest of the family).
//!
//! ## What this module is, and what it is NOT
//!
//! It is the **transcription and the provenance declaration**: every box the form prints, its caption
//! verbatim, and what v1 does with it. Per [*blank is the normal case*], the gate is never *"does every
//! box carry a value"* — most boxes on a real 1099-R are empty and that is the correct document. The
//! invariant is that **every box has a determinate disposition**, because two blanks look identical on
//! the page and are not the same thing: *"the form printed nothing here"* and *"we forgot this box"*.
//!
//! ★★ **Nothing consumes this yet.** No compute reads these boxes, no `ReturnInputs` field holds them,
//! and `DocumentKind::Form1099R` does not exist — so the census keeps this document authority-only by
//! construction (`box_census::modelled_stems`). Wiring it is the next task and is deliberately separate:
//! this one can be verified against the form with no engine changes at all, and a wrong transcription
//! caught here costs nothing, while the same error found after wiring is a money defect.
//!
//! ## Why the captions stop where they do
//!
//! Each caption is quoted **to its first printed line**, which is the convention `box_census::BoxEntry`
//! already states: a caption the layout wraps is not printed contiguously, so quoting across the wrap
//! would quote text the page never shows. *"3 Capital gain (included in"* really is what
//! `f1099r--2024.txt` prints on that line; *"box 2a)"* is on the next.
//!
//! ## The two editions are identical here, and that is measured
//!
//! `f1099r--2024` and `f1099r--2025` print a byte-identical label→caption map, so one table serves
//! both. `xtask`'s `the_form1099r_box_set_equals_the_form` asserts that against the enumerator for each
//! edition separately, so a future revision that reworded or added a box reds rather than being absorbed.

use crate::conventions::Usd;
use serde::{Deserialize, Serialize};
use time::Date;

/// What v1 does with one box. **Every box has exactly one**, and `NoFederalLine` is a decision rather
/// than a gap — it records that the box encodes no federal tax consequence, with the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum V1Disposition {
    /// The spec routes this box to a printed line. The string names it.
    RoutesTo(&'static str),
    /// Read only in order to DECIDE something — a code or a checkbox, never a figure on a line.
    Decides(&'static str),
    /// v1 cannot carry this figure, so a present/non-zero value REFUSES the return rather than being
    /// dropped. Failing closed: the alternative is filing a line that silently omits it.
    RefuseIfPresent(&'static str),
    /// Transcribed and carried, but no v1 line reads it. The reason names what WOULD read it, so the
    /// box is scheduled rather than merely unused.
    CarriedUnread(&'static str),
    /// An identifier, a payer flag or a state/local figure that no FEDERAL line reads.
    NoFederalLine(&'static str),
}

/// One printed box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Box1099R {
    /// The box's own label as the form prints it: `"1"`, `"2a"`, `"9b"`, `"14"`.
    pub label: &'static str,
    /// **VERBATIM** from `design/forms/extract/f1099r--<edition>.txt`, label included, quoted to the
    /// label's own printed line.
    pub caption: &'static str,
    pub v1: V1Disposition,
}

use V1Disposition::*;

/// **All 21 boxes Form 1099-R prints.** Held to the form by `xtask`'s
/// `the_form1099r_box_set_equals_the_form`, which derives the set with `box_census::printed_boxes` and
/// asserts equality in both directions for both editions — so a box added, removed or reworded by a
/// future revision reds rather than being absorbed silently.
pub const BOXES: &[Box1099R] = &[
    Box1099R {
        label: "1",
        caption: "1 Gross distribution",
        v1: RoutesTo("1040 line 4a/4b (IRA) or 5a/5b (pension), per the document's kind"),
    },
    Box1099R {
        label: "2a",
        caption: "2a Taxable amount",
        v1: RoutesTo("1040 line 5b on the pension side; the IRA side takes box 1 (SPEC §4.1)"),
    },
    Box1099R {
        // ★ ONE label, TWO printed checkboxes: "Taxable amount not determined" and "Total
        //   distribution". The first refuses (R-5) because a taxable amount the payer would not
        //   determine is one v1 must not invent; the second is informational.
        label: "2b",
        caption: "2b Taxable amount",
        v1: RefuseIfPresent(
            "the \"taxable amount not determined\" checkbox ⇒ R-5: the payer declined to determine \
             it, so v1 may not. The adjacent \"total distribution\" checkbox is informational",
        ),
    },
    Box1099R {
        label: "3",
        caption: "3 Capital gain (included in",
        v1: RefuseIfPresent(
            "the capital-gain part of a lump-sum distribution routes to Form 4972's 20% election, \
             which v1 does not model; it is INCLUDED in box 2a, so ignoring it would file 2a's figure \
             at ordinary rates and overstate tax",
        ),
    },
    Box1099R {
        label: "4",
        caption: "4 Federal income tax",
        v1: RoutesTo("1040 line 25b — withholding. Review r1's C-2 was this box being dropped"),
    },
    Box1099R {
        label: "5",
        caption: "5 Employee contributions/",
        v1: CarriedUnread(
            "the Simplified Method's cost in the plan (R-3/R-5 refuse it in v1) and the A-1 retired \
             public safety officer premium exclusion (advisory only, SPEC §9)",
        ),
    },
    Box1099R {
        label: "6",
        caption: "6 Net unrealized",
        v1: RefuseIfPresent(
            "net unrealized appreciation in employer securities is excluded from box 2a and deferred \
             until sale; an employer-securities distribution carries basis machinery v1 has none of",
        ),
    },
    Box1099R {
        label: "7",
        caption: "7 Distribution",
        v1: Decides(
            "the Roth qualified sub-branch — box 7 EXACTLY \"Q\", or exactly \"T\" plus the \
             contribution-year answer, files 4b = -0- (SPEC §8); everything else reaches R-2. The \
             adjacent IRA/SEP/SIMPLE checkbox decides which line pair the document reaches",
        ),
    },
    Box1099R {
        label: "8",
        caption: "8 Other",
        v1: RefuseIfPresent(
            "a figure and a percentage v1 has no line for; it appears on annuity and insurance \
             arrangements that carry their own machinery",
        ),
    },
    Box1099R {
        label: "9a",
        caption: "9a Your percentage of total",
        v1: CarriedUnread(
            "one recipient's share of a distribution split between several — needed only once v1 \
             supports multiple recipients of one contract",
        ),
    },
    Box1099R {
        label: "9b",
        caption: "9b Total employee contributions",
        v1: CarriedUnread("the Simplified Method's investment in the contract (refused in v1)"),
    },
    Box1099R {
        label: "10",
        caption: "10 Amount allocable to IRR",
        v1: RefuseIfPresent(
            "an in-plan Roth rollover inside the 5-year window carries a recapture rule v1 does not \
             model",
        ),
    },
    Box1099R {
        label: "11",
        caption: "11 1st year of desig.",
        v1: CarriedUnread(
            "the first year of designated Roth contributions — the 5-year fact the code-T question \
             asks the filer for, and the box that lets a later version answer it from the document \
             instead",
        ),
    },
    Box1099R {
        label: "12",
        caption: "12 FATCA filing",
        v1: NoFederalLine(
            "a chapter-4 FATCA flag the PAYER sets to record its own filing obligation; it asserts \
             nothing about the filer and changes no line on this return",
        ),
    },
    Box1099R {
        label: "13",
        caption: "13 Date of",
        v1: NoFederalLine(
            "the payment date, which matters to the 5-year and early-distribution rules the payer \
             already resolved into the box-7 code; no 1040 line asks for a date",
        ),
    },
    Box1099R {
        label: "14",
        caption: "14 State tax withheld",
        v1: RoutesTo(
            "Schedule A line 5a — \"Forms W-2G, 1099-G, 1099-R, 1099-MISC, and 1099-NEC may also \
             show state and local income taxes withheld\" (i1040sca--2024.txt:324). THE BOX §7 \
             OMITTED; see FR-258",
        ),
    },
    Box1099R {
        label: "15",
        caption: "15 State/Payer’s state no.",
        v1: NoFederalLine(
            "the payer's state registration number — it identifies the payer to a STATE revenue \
             department, and btctax files no state return",
        ),
    },
    Box1099R {
        label: "16",
        caption: "16 State distribution",
        v1: NoFederalLine(
            "the distribution as the STATE measures it, which can differ from box 1; the federal \
             return takes box 1 and the withholding, never the state's own figure",
        ),
    },
    Box1099R {
        label: "17",
        caption: "17 Local tax withheld",
        v1: CarriedUnread(
            "Schedule A line 5a by the same instruction as box 14 — DEFERRED with the QCD widening \
             and recorded in FR-258 so it cannot be lost",
        ),
    },
    Box1099R {
        label: "18",
        caption: "18 Name of locality",
        v1: NoFederalLine(
            "the locality's name, which identifies where box 17 was withheld; Schedule A line 5a \
             takes the AMOUNT, and no federal line asks which locality it went to",
        ),
    },
    Box1099R {
        label: "19",
        caption: "19 Local distribution",
        v1: NoFederalLine(
            "the distribution as the LOCALITY measures it — the same reasoning as box 16: the \
             federal return never reads a state or local re-measurement of box 1",
        ),
    },
];

/// The boxes whose presence must REFUSE the return in v1, derived from [`BOXES`] rather than listed.
///
/// ★ Derived on purpose: a list typed beside [`BOXES`] would be correct the day it was written and
/// would not know when a later edit changed a box's disposition. Widening the set is now a consequence
/// of editing the box, which is the only place the decision lives.
#[must_use]
pub fn refusing_boxes() -> Vec<&'static str> {
    BOXES
        .iter()
        .filter(|b| matches!(b.v1, RefuseIfPresent(_)))
        .map(|b| b.label)
        .collect()
}

/// Whether a transcribed figure on a refusing box should stop the return.
///
/// ★ `Usd::ZERO` does **not** refuse: a payer printing `0` in box 3 has told us there is no capital
/// gain, which is exactly the case v1 handles. Refusing on a printed zero would turn away a return the
/// form says is ordinary — the too-wide refusal that costs the filer their return.
#[must_use]
pub fn refuses_on(label: &str, value: Usd) -> bool {
    value != Usd::ZERO && refusing_boxes().contains(&label)
}

/// Which pair of 1040 lines this document reaches. Decided by box 7's adjacent **IRA/SEP/SIMPLE**
/// checkbox, which is why box 7's disposition is [`V1Disposition::Decides`] and not a figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form1099RKind {
    /// The IRA/SEP/SIMPLE box is checked: lines 4a/4b.
    Ira,
    /// It is not: lines 5a/5b.
    PensionOrAnnuity,
}

/// **One transcribed Form 1099-R — the 15 boxes v1 must hold, and nothing else.**
///
/// ★★★ **The field set is DERIVED from [`BOXES`], not typed beside it.** A box needs a field iff its
/// disposition is anything but [`V1Disposition::NoFederalLine`] — you cannot refuse on a figure you did
/// not collect, and you cannot route or decide on one either. The six `NoFederalLine` boxes (12, 13, 15,
/// 16, 18, 19) deliberately have **no field**: that is a recorded decision, not a gap.
///
/// `tests::the_field_set_is_exactly_what_the_dispositions_require` holds the join by destructuring this
/// struct **exhaustively**, so adding or removing a field breaks the build until the mapping is updated —
/// the compiler holds it rather than a reviewer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Form1099R {
    /// *"PAYER'S name"* as printed.
    pub payer: String,
    /// **R10.2 — document identity.**
    #[serde(default)]
    pub payer_tin: String,
    /// **R10.2 — when this row was transcribed.**
    #[serde(default)]
    pub transcribed_on: Option<Date>,
    /// Which line pair this document reaches — box 7's IRA/SEP/SIMPLE checkbox.
    pub kind: Form1099RKind,

    /// **Box 1 — gross distribution.** No `serde(default)`: a 1099-R without box 1 is a mistyped row,
    /// not a lawful state.
    pub box1_gross_distribution: Usd,
    /// **Box 2a — taxable amount.** `Option`, and the `None` is load-bearing: an unfilled box 2a is
    /// what box 2b's *"not determined"* checkbox reports, and it must not read as `0`.
    #[serde(default)]
    pub box2a_taxable_amount: Option<Usd>,
    /// **Box 2b, first checkbox — *Taxable amount not determined*.** ⇒ R-5.
    #[serde(default)]
    pub box2b_taxable_amount_not_determined: bool,
    /// **Box 2b, second checkbox — *Total distribution*.** Informational.
    #[serde(default)]
    pub box2b_total_distribution: bool,
    /// **Box 3 — capital gain included in box 2a.** Refuses when non-zero.
    #[serde(default)]
    pub box3_capital_gain: Usd,
    /// **Box 4 — federal income tax withheld.** → 1040 line 25b (review r1's C-2).
    #[serde(default)]
    pub box4_fed_withheld: Usd,
    /// **Box 5 — employee contributions / designated Roth contributions or insurance premiums.**
    #[serde(default)]
    pub box5_employee_contributions: Usd,
    /// **Box 6 — net unrealized appreciation in employer's securities.** Refuses when non-zero.
    #[serde(default)]
    pub box6_net_unrealized_appreciation: Usd,
    /// **Box 7 — the distribution code(s).** Read by EQUALITY against `"Q"` / `"T"`; every Table 1 code
    /// is a single character, so a composed box 7 such as `"QJ"` reaches R-2 rather than the `-0-`
    /// sub-branch (r2 I-5).
    pub box7_distribution_codes: String,
    /// **Box 8 — *Other*.** Refuses when non-zero.
    #[serde(default)]
    pub box8_other: Usd,
    /// **Box 9a — this recipient's percentage of the total distribution.** A percentage, not money.
    #[serde(default)]
    pub box9a_percentage_of_total: Option<Usd>,
    /// **Box 9b — total employee contributions.**
    #[serde(default)]
    pub box9b_total_employee_contributions: Usd,
    /// **Box 10 — amount allocable to an in-plan Roth rollover within 5 years.** Refuses when non-zero.
    #[serde(default)]
    pub box10_allocable_to_irr: Usd,
    /// **Box 11 — the first year of designated Roth contributions.** A YEAR, not money.
    #[serde(default)]
    pub box11_first_year_desig_roth: Option<i32>,
    /// **Box 14 — state tax withheld.** → Schedule A line 5a. The box §7 omitted; see FR-258.
    #[serde(default)]
    pub box14_state_tax_withheld: Option<Usd>,
    /// **Box 17 — local tax withheld.** Schedule A line 5a by the same instruction; DEFERRED (FR-258).
    #[serde(default)]
    pub box17_local_tax_withheld: Option<Usd>,

    /// ★★ **The seventh question (r2 I-1), not a box.** Asked only when [`Self::box7_distribution_codes`]
    /// is exactly `"T"` on an IRA document, because code T means the payer did NOT know whether the
    /// 5-year holding period was met. `None` REFUSES — unlike the two silent advisory questions, silence
    /// here would admit the filer to the `-0-` branch and understate tax.
    #[serde(default)]
    pub roth_contribution_before_lookback: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn extract(edition: &str) -> String {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(format!("design/forms/extract/f1099r--{edition}.txt"));
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    /// ★★★ **THE JOIN: the struct's field set is exactly what the dispositions require — held by an
    /// EXHAUSTIVE destructure, so the compiler enforces it rather than a reviewer.**
    ///
    /// A box needs a field iff its disposition is anything but [`V1Disposition::NoFederalLine`]: you
    /// cannot refuse on a figure you never collected, and you cannot route or decide on one either.
    ///
    /// ★★ **Why the destructure and not a list of names.** A `const FIELDS: &[&str]` beside the struct
    /// would be a third hand-written list — correct the day it is typed and silent when a later edit
    /// widens the struct. The `let Form1099R { .. } = ` below has no `..` rest pattern, so **adding a
    /// field fails to compile here** until someone says which box it carries, and removing one fails
    /// too. That is the only mechanism that survives an unrelated future edit.
    #[test]
    fn the_field_set_is_exactly_what_the_dispositions_require() {
        // The mapping, one line per field. Adding a field to `Form1099R` breaks this destructure.
        #[allow(unused_variables)]
        fn map(f: &Form1099R) -> Vec<&'static str> {
            let Form1099R {
                // ── identity and routing, not boxes ───────────────────────────────────────────────
                payer,
                payer_tin,
                transcribed_on,
                kind,
                // ── one per box needing a field ───────────────────────────────────────────────────
                box1_gross_distribution,
                box2a_taxable_amount,
                box2b_taxable_amount_not_determined,
                box2b_total_distribution,
                box3_capital_gain,
                box4_fed_withheld,
                box5_employee_contributions,
                box6_net_unrealized_appreciation,
                box7_distribution_codes,
                box8_other,
                box9a_percentage_of_total,
                box9b_total_employee_contributions,
                box10_allocable_to_irr,
                box11_first_year_desig_roth,
                box14_state_tax_withheld,
                box17_local_tax_withheld,
                // ── the seventh question (r2 I-1): a question, not a printed box ─────────────────
                roth_contribution_before_lookback,
            } = f;
            // `2b` appears once: two checkboxes, one printed box label.
            vec![
                "1", "2a", "2b", "2b", "3", "4", "5", "6", "7", "8", "9a", "9b", "10", "11", "14",
                "17",
            ]
        }

        let covered: BTreeSet<&str> = map(&sample()).into_iter().collect();
        let required: BTreeSet<&str> = BOXES
            .iter()
            .filter(|b| !matches!(b.v1, NoFederalLine(_)))
            .map(|b| b.label)
            .collect();
        assert_eq!(
            covered, required,
            "the struct's boxes and the dispositions requiring a field have diverged"
        );

        // ★ And the six NoFederalLine boxes must have NO field — asserted, so that a future edit which
        //   quietly starts collecting one has to change this line and say why.
        let no_field: BTreeSet<&str> = BOXES
            .iter()
            .filter(|b| matches!(b.v1, NoFederalLine(_)))
            .map(|b| b.label)
            .collect();
        assert_eq!(
            no_field,
            ["12", "13", "15", "16", "18", "19"]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );
        assert!(
            covered.is_disjoint(&no_field),
            "a NoFederalLine box gained a field"
        );
    }

    fn sample() -> Form1099R {
        Form1099R {
            payer: "P".into(),
            payer_tin: String::new(),
            transcribed_on: None,
            kind: Form1099RKind::Ira,
            box1_gross_distribution: Usd::ZERO,
            box2a_taxable_amount: None,
            box2b_taxable_amount_not_determined: false,
            box2b_total_distribution: false,
            box3_capital_gain: Usd::ZERO,
            box4_fed_withheld: Usd::ZERO,
            box5_employee_contributions: Usd::ZERO,
            box6_net_unrealized_appreciation: Usd::ZERO,
            box7_distribution_codes: String::new(),
            box8_other: Usd::ZERO,
            box9a_percentage_of_total: None,
            box9b_total_employee_contributions: Usd::ZERO,
            box10_allocable_to_irr: Usd::ZERO,
            box11_first_year_desig_roth: None,
            box14_state_tax_withheld: None,
            box17_local_tax_withheld: None,
            roth_contribution_before_lookback: None,
        }
    }

    /// ★★★ **Every caption is verbatim in BOTH archived editions.** This is the transcription gate: a
    /// caption someone paraphrased, or read off the rendered page instead of the text layer, reds here.
    /// The standing example of why is Form 6251 line 33, where a rendered `12` was transcribed for the
    /// form's `22` and taxed one slice twice.
    #[test]
    fn every_caption_is_verbatim_in_both_editions() {
        for ed in ["2024", "2025"] {
            let text = extract(ed);
            for b in BOXES {
                assert!(
                    text.contains(b.caption),
                    "f1099r--{ed} does not print {:?} (box {})",
                    b.caption,
                    b.label
                );
            }
        }
    }

    /// ★★ **Every box has a determinate disposition, and no label appears twice.** The provenance
    /// invariant: the defect this module exists to prevent is a box nothing ever decided about, which
    /// is invisible on the printed page and invisible to any test that checks a value.
    #[test]
    fn every_box_has_one_disposition_and_labels_are_unique() {
        let labels: BTreeSet<&str> = BOXES.iter().map(|b| b.label).collect();
        assert_eq!(labels.len(), BOXES.len(), "a duplicated box label");
        for b in BOXES {
            let reason = match b.v1 {
                RoutesTo(s) | Decides(s) | RefuseIfPresent(s) | CarriedUnread(s)
                | NoFederalLine(s) => s,
            };
            assert!(
                reason.len() > 20,
                "box {} has a disposition with no real reason: {reason:?}",
                b.label
            );
            assert!(
                b.caption.starts_with(b.label),
                "box {}'s caption must begin with its own label, as the form prints it: {:?}",
                b.label,
                b.caption
            );
        }
    }

    /// ★★★ **A printed ZERO on a refusing box must NOT refuse.** A payer writing `0` in box 3 has
    /// testified there is no capital gain — the ordinary case. Refusing on it is the too-wide refusal,
    /// and a refusal that is too wide costs the filer their return.
    #[test]
    fn a_printed_zero_does_not_refuse_but_a_figure_does() {
        use rust_decimal_macros::dec;
        assert!(!refuses_on("3", Usd::ZERO));
        assert!(refuses_on("3", dec!(1)));
        // And a box that does not refuse never refuses, whatever it carries.
        assert!(
            !refuses_on("14", dec!(9_999)),
            "box 14 ROUTES, it does not refuse"
        );
        assert!(!refuses_on("1", dec!(50_000)));
    }

    /// ★ The refusing set is DERIVED, so this pins what the dispositions currently say rather than a
    /// parallel list. If a box's disposition changes, this reds and names it — which is the point.
    #[test]
    fn the_refusing_set_is_what_the_dispositions_say() {
        assert_eq!(
            refusing_boxes(),
            vec!["2b", "3", "6", "8", "10"],
            "the refusing set changed; confirm the new disposition is intended"
        );
    }
}
