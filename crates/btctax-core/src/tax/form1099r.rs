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

    /// ★★★ **The class-(A) declaration the whole compute turns on (SPEC S-3), and it was MISSING from
    /// this struct until 2026-09-20.**
    ///
    /// *"Does any of the exceptions in the line 4a/4b (or 5a/5b) instructions apply to this
    /// distribution?"* — one question per document over the form's OWN exception list, not one per
    /// exception. `None` ⇒ **R-1** (IRA) / **R-3** (pension): silence is not testimony that none
    /// applies, and it matters in both directions. `Some(true)` ⇒ **R-2** / **R-4**, unless the document
    /// is on the Roth `Q`/`T` sub-branch the instructions close themselves. `Some(false)` computes.
    ///
    /// ★★ **How it went missing is the interesting part.** `tests::the_field_set_is_exactly_what_the_
    /// dispositions_require` holds this struct's fields against `BOXES` — and `exception_applies` is not
    /// a box, so the gate was blind to it. It was equally blind to
    /// [`Self::roth_contribution_before_lookback`], which I happened to include. Care caught one of two;
    /// structure caught neither. [`QUESTIONS`] is the fix: the same destructure now partitions into
    /// boxes and QUESTIONS, and a question with no declared `None` behaviour fails the gate.
    #[serde(default)]
    pub exception_applies: Option<bool>,
}

/// ★★★ **Every field on [`Form1099R`] that is a QUESTION rather than a printed box, with what its
/// silence means.**
///
/// A printed box's provenance comes from [`BOXES`]; a question has none there, which is exactly how
/// `exception_applies` was omitted from the struct for two commits without any gate noticing. Each entry
/// carries `refuses_on_none`, because that is the decision that cannot be left implicit: a question
/// whose silence is lawful and one whose silence understates tax look identical in the type system.
pub const QUESTIONS: &[(&str, bool, &str)] = &[
    // (field, refuses when `None`, why)
    (
        "exception_applies",
        true,
        "S-3 — silence is not testimony that no exception applies, and it matters in BOTH \
         directions: R-1 (IRA) / R-3 (pension)",
    ),
    (
        "roth_contribution_before_lookback",
        true,
        "r2 I-1 — code T means the PAYER did not know whether the 5-year period was met, so \
         silence would admit the filer to the 4b = -0- branch and understate tax",
    ),
];

/// Why one document stops the return. One enum for both kinds: the IRA and pension sides share every
/// refusal but the two that are branch-specific, and a second enum would be two lists to keep in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocRefusal {
    /// R-1 (IRA) / R-3 (pension) — `exception_applies` is `None`. Silence is not testimony.
    ExceptionUnanswered,
    /// R-2 (IRA) / R-4 (pension) — an exception applies and this build cannot follow it.
    ExceptionUnsupported,
    /// r2 I-1 — box 7 is exactly `T` and the lookback question is unanswered.
    RothLookbackUnanswered,
    /// r2 I-1 — box 7 is exactly `T` and the filer answered NO, so Form 8606 is genuinely needed.
    RothLookbackNo,
    /// R-5 — box 2b's *"Taxable amount not determined"* checkbox. The PAYER declined to determine it.
    TaxableAmountNotDetermined,
    /// R-5 — a pension with a BLANK box 2a: the case the instructions send to the General Rule /
    /// Simplified Method (`i1040gi--2025.txt:2888-2891`), which v1 does not build.
    PensionTaxableAmountMissing,
    /// ★★★ A box whose [`V1Disposition`] is [`V1Disposition::RefuseIfPresent`] carries a figure. The
    /// label is the box's own, so the refusal can name it.
    ///
    /// Until this variant existed those five dispositions were **decorative**: `BOXES` declared that a
    /// non-zero box 3, 6, 8 or 10 must stop the return and nothing read the declaration. That is the
    /// `a-figure-with-no-reader` shape applied to a refusal rather than to a figure.
    RefusingBoxPresent(&'static str),
}

/// ★★★ **ONE DOCUMENT's taxable amount, or why the return refuses. Per document, never per return.**
///
/// The first version of the spec's rule table keyed its rows on a per-DOCUMENT condition and its columns
/// on per-RETURN 1040 lines, and fold-review r2's C-1 (Critical) showed that a qualified Roth `Q`
/// document beside a fully-taxable traditional IRA then files **`4b = -0-`** — the whole traditional
/// distribution missing from total income, with nothing refusing. The instructions compose per PART and
/// say so: *"enter the part that is not a QCD on line 4b **unless Exception 2 applies to that part**"*
/// (`i1040gi--2025.txt:2731`).
///
/// So nothing here may name a 1040 line. [`line_4b`] / [`line_5b`] and [`line_4a`] / [`line_5a`] are the
/// only places a line is written.
///
/// ★★ **Box 7 is matched by EQUALITY, not membership** (r2 I-5): Table 1's *used with* column for `Q`
/// reads `None`, and the Note inside both the `Q` and `T` entries instructs the payer to print `J`
/// INSTEAD when another code applies. So `"QJ"` is a document this build does not understand and
/// refuses; `contains("Q")` would admit it to the `-0-` branch and UNDERSTATE tax.
///
/// ★ **Both kinds run the same gate order**, and the order is the argument: the payer's own
/// *"not determined"* checkbox and the boxes v1 cannot carry are prior to the filer's declaration, which
/// is prior to any reading of box 7.
pub fn document_taxable(f: &Form1099R) -> Result<Usd, DocRefusal> {
    // (1) The payer declined to determine the taxable amount. v1 may not determine it either.
    if f.box2b_taxable_amount_not_determined {
        return Err(DocRefusal::TaxableAmountNotDetermined);
    }
    // (2) Any box whose disposition says it must refuse, carrying a figure. DERIVED from `BOXES`, so a
    //     disposition changed there changes this without a second list.
    for (label, value) in [
        ("3", f.box3_capital_gain),
        ("6", f.box6_net_unrealized_appreciation),
        ("8", f.box8_other),
        ("10", f.box10_allocable_to_irr),
    ] {
        if refuses_on(label, value) {
            return Err(DocRefusal::RefusingBoxPresent(match label {
                "3" => "3",
                "6" => "6",
                "8" => "8",
                _ => "10",
            }));
        }
    }
    // (3) The filer's declaration. Silence is prior to every reading of the form.
    let Some(exception) = f.exception_applies else {
        return Err(DocRefusal::ExceptionUnanswered);
    };
    match f.kind {
        Form1099RKind::Ira => {
            if !exception {
                // "enter the total distribution on line 4b" (`:2664-2667`) — this document's taxable
                // amount IS its box 1 on the fully-taxable branch.
                return Ok(f.box1_gross_distribution);
            }
            match f.box7_distribution_codes.trim() {
                // "b. Distribution code Q is shown in box 7" ⇒ "enter -0- on line 4b" (`:2707-2715`).
                // The payer certified the 5-year period AND age/death/disability, so nothing is asked.
                "Q" => Ok(Usd::ZERO),
                // "a. Distribution code T ... and you made a contribution (including a conversion) to a
                // Roth IRA for <YEAR> or an earlier year". The payer did NOT know the 5-year fact.
                "T" => match f.roth_contribution_before_lookback {
                    Some(true) => Ok(Usd::ZERO),
                    Some(false) => Err(DocRefusal::RothLookbackNo),
                    None => Err(DocRefusal::RothLookbackUnanswered),
                },
                _ => Err(DocRefusal::ExceptionUnsupported),
            }
        }
        Form1099RKind::PensionOrAnnuity => {
            // ★★ R-4. The pension exceptions (rollover, PSO, a pre-minimum-retirement-age disability
            //    pension reported on line 1h) all route somewhere v1 has no line for. This arm was
            //    MISSING until 2026-09-20: `line_5b` read only box 2a, so a pension whose filer declared
            //    an exception computed anyway and the declaration had no reader.
            if exception {
                return Err(DocRefusal::ExceptionUnsupported);
            }
            // The taxable part comes from box 2a when the form shows it; a blank sends the filer to the
            // General Rule / Simplified Method, which v1 refuses.
            f.box2a_taxable_amount
                .ok_or(DocRefusal::PensionTaxableAmountMissing)
        }
    }
}

/// **Line 4b = Σ of every IRA document's own taxable amount.**
///
/// *"figure the taxable amount of each distribution and enter the total of the taxable amounts on line
/// 4b"* — `i1040gi--2025.txt:2789-2795`, identical at TY2024 `:2726-2732`.
///
/// `Err` carries the FIRST refusing document's index and reason: a return that refuses has no line 4b at
/// all, and returning a figure beside a refusal is how a refusal gets ignored.
pub fn line_4b(rows: &[Form1099R]) -> Result<Usd, (usize, DocRefusal)> {
    sum_taxable(rows, Form1099RKind::Ira)
}

/// **Line 5b = Σ box 2a over pension documents**, subject to the same per-document gate.
pub fn line_5b(rows: &[Form1099R]) -> Result<Usd, (usize, DocRefusal)> {
    sum_taxable(rows, Form1099RKind::PensionOrAnnuity)
}

fn sum_taxable(rows: &[Form1099R], kind: Form1099RKind) -> Result<Usd, (usize, DocRefusal)> {
    let mut total = Usd::ZERO;
    for (i, f) in rows.iter().enumerate().filter(|(_, f)| f.kind == kind) {
        total += document_taxable(f).map_err(|r| (i, r))?;
    }
    Ok(total)
}

/// **Line 4a.** `None` means the form instructs a BLANK, which is not the same as zero.
///
/// Populated when **more than one** IRA document exists — *"Enter the total amount of those
/// distributions on line 4a"* (`:2789-2795`, unscoped, unlike the pension analogue at `:2978-2980`, which
/// is limited to *"more than one PARTIALLY TAXABLE pension"*) — **or** when any document is on the Roth
/// `-0-` sub-branch, whose own instruction is *"enter the total distribution on line 4a"* (`:2698`).
///
/// Blank on a single fully-taxable distribution: *"enter the total distribution on line 4b; **don't make
/// an entry on line 4a**"* (`:2664-2667`).
///
/// ★★ A `Some(Usd::ZERO)` here would be sworn testimony that the filer received no IRA distribution, so
/// the blank case is `None` and never a zero ([`an-entry-is-testimony`]).
#[must_use]
pub fn line_4a(rows: &[Form1099R]) -> Option<Usd> {
    let ira: Vec<&Form1099R> = rows
        .iter()
        .filter(|f| f.kind == Form1099RKind::Ira)
        .collect();
    if ira.is_empty() {
        return None;
    }
    let on_sub_branch = ira.iter().any(|f| {
        f.exception_applies == Some(true) && matches!(f.box7_distribution_codes.trim(), "Q" | "T")
    });
    if ira.len() > 1 || on_sub_branch {
        Some(ira.iter().map(|f| f.box1_gross_distribution).sum())
    } else {
        None
    }
}

/// **Line 5a.** Populated for **any partially taxable** pension — box 2a shown and less than box 1.
///
/// *"**Partially Taxable Pensions and Annuities.** Enter the total pension or annuity payments (from Form
/// 1099-R, box 1) on line 5a"* — `i1040gi--2025.txt:2888-2891`. ★ This is the authority fold-review r2's
/// I-3 supplied; the earlier draft rested on the Simplified Method Worksheet, which this spec REFUSES.
///
/// Blank on a single fully-taxable pension: *"don't make an entry on line 5a"* (`:2876-2880`).
#[must_use]
pub fn line_5a(rows: &[Form1099R]) -> Option<Usd> {
    let pens: Vec<&Form1099R> = rows
        .iter()
        .filter(|f| f.kind == Form1099RKind::PensionOrAnnuity)
        .collect();
    if pens.is_empty() {
        return None;
    }
    let partially_taxable = |f: &Form1099R| {
        f.box2a_taxable_amount
            .is_some_and(|t| t < f.box1_gross_distribution)
    };
    if pens.iter().any(|f| partially_taxable(f)) {
        Some(pens.iter().map(|f| f.box1_gross_distribution).sum())
    } else {
        None
    }
}

/// **Line 25b's share — Σ box 4 over EVERY Form 1099-R**, IRA and pension alike.
///
/// Review r1's C-2: this box was dropped entirely, so a retiree's withholding never reached line 25b and
/// the return overstated the balance due by the whole of it.
#[must_use]
pub fn withholding_line_25b(rows: &[Form1099R]) -> Usd {
    rows.iter().map(|f| f.box4_fed_withheld).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
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
                // ── QUESTIONS, not printed boxes. Held by `QUESTIONS` below, not by `BOXES`. ─────
                roth_contribution_before_lookback,
                exception_applies,
            } = f;
            // ★★★ Every question destructured above must appear in `QUESTIONS`, and vice versa. This is
            //     the half that was missing: `exception_applies` was absent from the STRUCT for two
            //     commits and no gate could see it, because the box join has nothing to say about a
            //     field that is not a box.
            let questioned = ["roth_contribution_before_lookback", "exception_applies"];
            let declared: Vec<&str> = QUESTIONS.iter().map(|(f, _, _)| *f).collect();
            assert_eq!(
                questioned.iter().copied().collect::<BTreeSet<_>>(),
                declared.iter().copied().collect::<BTreeSet<_>>(),
                "a question field and `QUESTIONS` have diverged — a question with no declared `None` \
                 behaviour is how a silent understatement gets in"
            );
            for (field, _refuses, why) in QUESTIONS {
                assert!(
                    why.len() > 30,
                    "{field}: a question's `None` behaviour needs a REASON, not a label"
                );
            }
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

    /// Build one IRA row: `(box1, exception_applies, box7, lookback)`.
    fn ira(box1: i64, exc: Option<bool>, code: &str, lookback: Option<bool>) -> Form1099R {
        Form1099R {
            kind: Form1099RKind::Ira,
            box1_gross_distribution: rust_decimal::Decimal::from(box1),
            box2a_taxable_amount: Some(rust_decimal::Decimal::from(box1)),
            box7_distribution_codes: code.into(),
            exception_applies: exc,
            roth_contribution_before_lookback: lookback,
            ..sample()
        }
    }

    /// Build one pension row: `(box1, box2a)`.
    fn pension(box1: i64, box2a: Option<i64>) -> Form1099R {
        Form1099R {
            kind: Form1099RKind::PensionOrAnnuity,
            box1_gross_distribution: rust_decimal::Decimal::from(box1),
            box2a_taxable_amount: box2a.map(rust_decimal::Decimal::from),
            box7_distribution_codes: "7".into(),
            exception_applies: Some(false),
            ..sample()
        }
    }

    /// ★★★ **M-14 — THE CRITICAL, AS A KAT. Fold-review r2's C-1.**
    ///
    /// A qualified Roth `Q` document (box 1 = 10,000) beside a fully-taxable traditional IRA (box 1 =
    /// 20,000). The spec's first rule table — rows keyed per DOCUMENT, columns labelled with per-RETURN
    /// 1040 lines — read *box 7 contains `Q` ⇒ line 4b = `-0-`* and filed **4b = -0-**, losing the whole
    /// traditional distribution from total income with no refusal behind it.
    ///
    /// The correct figures come from the instructions' own composition: 4b = Σ of each document's taxable
    /// amount, 4a = Σ box 1 over all of them.
    #[test]
    fn m14_the_mixed_return_files_4a_30000_and_4b_20000() {
        let rows = vec![
            ira(10_000, Some(true), "Q", None),  // qualified Roth: taxable -0-
            ira(20_000, Some(false), "7", None), // fully taxable traditional
        ];
        assert_eq!(
            line_4b(&rows),
            Ok(dec!(20_000)),
            "4b is the SUM of per-document taxable amounts — the Roth contributes -0-, the traditional \
             its whole box 1. `-0-` here is the Critical."
        );
        assert_eq!(
            line_4a(&rows),
            Some(dec!(30_000)),
            "4a is Σ box 1 over ALL IRA documents once more than one exists"
        );
        // ★ And the per-document step must not be readable as a return-level answer: each document
        //   answers for itself, which is the structural fix rather than the arithmetic one.
        assert_eq!(document_taxable(&rows[0]), Ok(Usd::ZERO));
        assert_eq!(document_taxable(&rows[1]), Ok(dec!(20_000)));
    }

    /// ★★ A SINGLE fully-taxable IRA distribution leaves 4a BLANK — `None`, never `Some(0)`.
    ///
    /// *"enter the total distribution on line 4b; don't make an entry on line 4a"* (`:2664-2667`). A zero
    /// would be sworn testimony that no distribution was received.
    #[test]
    fn a_single_fully_taxable_ira_leaves_4a_blank_and_never_zero() {
        let rows = vec![ira(20_000, Some(false), "7", None)];
        assert_eq!(line_4b(&rows), Ok(dec!(20_000)));
        assert_eq!(
            line_4a(&rows),
            None,
            "the form instructs a BLANK, which is not zero"
        );
        assert_ne!(line_4a(&rows), Some(Usd::ZERO));
    }

    /// ★★★ A single code-`Q` Roth distribution — the ordinary retiree r1's I-3 found being REFUSED —
    /// files 4a = the total and 4b = `-0-`, and asks nothing.
    #[test]
    fn a_lone_qualified_roth_files_4a_and_a_zero_4b() {
        let rows = vec![ira(15_000, Some(true), "Q", None)];
        assert_eq!(line_4b(&rows), Ok(Usd::ZERO));
        assert_eq!(
            line_4a(&rows),
            Some(dec!(15_000)),
            "Exception 2's lead sentence: \"enter the total distribution on line 4a\" (:2698)"
        );
    }

    /// ★★★ **Box 7 is EQUALITY, not membership (r2 I-5), and the direction of the error is why.**
    ///
    /// The payer instructions' Note inside both the `Q` and `T` entries says that if another code
    /// applies, use Code **J** instead — so a composed `"QJ"` is not a plain qualified Roth. A
    /// `contains("Q")` test would put it on the `-0-` branch and UNDERSTATE tax.
    #[test]
    fn a_composed_box7_refuses_rather_than_taking_the_zero_branch() {
        for code in ["QJ", "Q8", "8Q", "TQ", "T4", " Q7"] {
            let rows = vec![ira(9_000, Some(true), code, Some(true))];
            assert_eq!(
                line_4b(&rows),
                Err((0, DocRefusal::ExceptionUnsupported)),
                "box 7 {code:?} must refuse, not take the -0- branch"
            );
        }
        // ★ And the two bare codes still work, including with surrounding whitespace, which a
        //   transcription realistically carries.
        assert_eq!(
            document_taxable(&ira(9_000, Some(true), " Q ", None)),
            Ok(Usd::ZERO)
        );
        assert_eq!(
            document_taxable(&ira(9_000, Some(true), "T", Some(true))),
            Ok(Usd::ZERO)
        );
    }

    /// ★★ Code `T` asks ONE question and its silence REFUSES — the asymmetry with `Q` that S-3's single
    /// boolean destroyed. `Q` means the payer KNEW the 5-year period was met; `T` means it did not.
    #[test]
    fn code_t_asks_one_question_and_silence_refuses() {
        assert_eq!(
            document_taxable(&ira(9_000, Some(true), "T", None)),
            Err(DocRefusal::RothLookbackUnanswered),
            "silence must not be read as either answer"
        );
        assert_eq!(
            document_taxable(&ira(9_000, Some(true), "T", Some(false))),
            Err(DocRefusal::RothLookbackNo),
            "answered NO ⇒ Form 8606 is genuinely needed"
        );
        assert_eq!(
            document_taxable(&ira(9_000, Some(true), "T", Some(true))),
            Ok(Usd::ZERO)
        );
        // ★ `Q` asks nothing: the lookback answer is irrelevant on that branch, in all three states.
        for l in [None, Some(false), Some(true)] {
            assert_eq!(
                document_taxable(&ira(9_000, Some(true), "Q", l)),
                Ok(Usd::ZERO)
            );
        }
    }

    /// ★★★ R-1 — an unanswered `exception_applies` refuses, and it is checked BEFORE box 7 is read.
    #[test]
    fn an_unanswered_exception_refuses_whatever_box_7_says() {
        for code in ["7", "Q", "T", ""] {
            assert_eq!(
                document_taxable(&ira(20_000, None, code, Some(true))),
                Err(DocRefusal::ExceptionUnanswered),
                "silence on the declaration is prior to any reading of box 7 ({code:?})"
            );
        }
    }

    /// ★ A refusing document must not yield a FIGURE alongside its refusal, and the refusal must name
    /// the document — otherwise a caller can print a line for a return that cannot be filed.
    #[test]
    fn line_4b_refuses_with_the_documents_index_and_never_a_figure() {
        let rows = vec![
            ira(20_000, Some(false), "7", None),
            ira(5_000, None, "7", None), // the second document is the unanswered one
        ];
        assert_eq!(line_4b(&rows), Err((1, DocRefusal::ExceptionUnanswered)));
    }

    /// ★★ The pension side: 5b = Σ box 2a, 5a = Σ box 1 when any pension is PARTIALLY taxable.
    ///
    /// Authority is `i1040gi--2025.txt:2888-2891` — NOT the Simplified Method Worksheet, which this spec
    /// refuses (r2 I-3 corrected exactly this).
    #[test]
    fn the_pension_side_fills_5a_only_when_partially_taxable() {
        // Fully taxable: box 2a == box 1 ⇒ 5a blank, 5b the whole amount.
        let full = vec![pension(30_000, Some(30_000))];
        assert_eq!(line_5b(&full), Ok(dec!(30_000)));
        assert_eq!(
            line_5a(&full),
            None,
            "\"don't make an entry on line 5a\" (:2876-2880)"
        );

        // Partially taxable: box 2a < box 1 ⇒ 5a = Σ box 1, 5b = Σ box 2a.
        let part = vec![pension(30_000, Some(24_000))];
        assert_eq!(line_5b(&part), Ok(dec!(24_000)));
        assert_eq!(line_5a(&part), Some(dec!(30_000)));

        // ★ Two pensions, one partially taxable: 5a takes Σ box 1 over BOTH, per :2978-2980.
        let two = vec![pension(30_000, Some(24_000)), pension(10_000, Some(10_000))];
        assert_eq!(line_5a(&two), Some(dec!(40_000)));
        assert_eq!(line_5b(&two), Ok(dec!(34_000)));

        // ★★ A blank box 2a REFUSES — it is the case the instructions send to the General Rule /
        //    Simplified Method, which v1 does not build. It must never read as zero.
        let blank = vec![pension(30_000, None)];
        assert_eq!(
            line_5b(&blank),
            Err((0, DocRefusal::PensionTaxableAmountMissing))
        );
    }

    /// ★★ The two sides do not contaminate each other: an IRA row must not reach 5a/5b, nor a pension
    /// row 4a/4b. `kind` is the only thing that routes, and it comes from box 7's own checkbox.
    #[test]
    fn the_two_line_pairs_are_disjoint() {
        let rows = vec![
            ira(20_000, Some(false), "7", None),
            pension(30_000, Some(24_000)),
        ];
        assert_eq!(
            line_4b(&rows),
            Ok(dec!(20_000)),
            "the pension must not enter 4b"
        );
        assert_eq!(
            line_4a(&rows),
            None,
            "one IRA document, fully taxable ⇒ 4a blank"
        );
        assert_eq!(
            line_5b(&rows),
            Ok(dec!(24_000)),
            "the IRA must not enter 5b"
        );
        assert_eq!(line_5a(&rows), Some(dec!(30_000)));
    }

    /// ★★★ **R-4 — a PENSION whose filer declared an exception must refuse, and this arm was MISSING.**
    ///
    /// Until 2026-09-20 `line_5b` read only box 2a, so a pension document with
    /// `exception_applies == Some(true)` computed its box 2a and filed — the declaration had no reader.
    /// The pension exceptions (a rollover, the PSO premium exclusion, a disability pension before minimum
    /// retirement age reported on line 1h) all route somewhere v1 has no line for.
    #[test]
    fn r4_a_pension_with_a_declared_exception_refuses() {
        let mut p = pension(30_000, Some(24_000));
        p.exception_applies = Some(true);
        assert_eq!(
            line_5b(&[p.clone()]),
            Err((0, DocRefusal::ExceptionUnsupported)),
            "the pension side must read the declaration, not just box 2a"
        );
        // ★ And R-3: silence on a pension refuses too, symmetrically with R-1 on the IRA side.
        p.exception_applies = None;
        assert_eq!(line_5b(&[p]), Err((0, DocRefusal::ExceptionUnanswered)));
    }

    /// ★★★ **The five `RefuseIfPresent` dispositions are ENFORCED, and until now they were decorative.**
    ///
    /// `BOXES` declared that a non-zero box 3, 6, 8 or 10 must stop the return, and nothing read the
    /// declaration — `a-figure-with-no-reader` applied to a refusal. Each of these figures changes the
    /// tax and v1 has no line for any of them, so filing past one understates or misstates.
    #[test]
    fn a_refusing_box_carrying_a_figure_stops_the_return() {
        type Plant = fn(&mut Form1099R);
        let cases: [(&str, Plant); 4] = [
            ("3", |f| f.box3_capital_gain = dec!(1_500)),
            ("6", |f| f.box6_net_unrealized_appreciation = dec!(4_000)),
            ("8", |f| f.box8_other = dec!(250)),
            ("10", |f| f.box10_allocable_to_irr = dec!(900)),
        ];
        for (label, set) in cases {
            let mut f = ira(20_000, Some(false), "7", None);
            set(&mut f);
            assert_eq!(
                document_taxable(&f),
                Err(DocRefusal::RefusingBoxPresent(label)),
                "box {label} carrying a figure must refuse — its disposition says so"
            );
        }
        // ★★ And a printed ZERO on every one of them must NOT refuse: that is the ordinary document, and
        //    a refusal here would be the too-wide kind that costs the filer their return.
        let ordinary = ira(20_000, Some(false), "7", None);
        assert_eq!(document_taxable(&ordinary), Ok(dec!(20_000)));
        // ★ The refusing set itself is derived, so this cross-checks the enforcement against it.
        assert_eq!(refusing_boxes(), vec!["2b", "3", "6", "8", "10"]);
    }

    /// ★★ R-5 — box 2b's *"Taxable amount not determined"* checkbox refuses, and it is checked BEFORE
    /// everything else: the payer declining to determine the figure is prior to any question the filer
    /// could answer.
    #[test]
    fn r5_the_payer_declining_to_determine_the_amount_refuses_first() {
        for kind in [Form1099RKind::Ira, Form1099RKind::PensionOrAnnuity] {
            let mut f = ira(20_000, Some(false), "7", None);
            f.kind = kind;
            f.box2b_taxable_amount_not_determined = true;
            assert_eq!(
                document_taxable(&f),
                Err(DocRefusal::TaxableAmountNotDetermined)
            );
            // ★ Prior to the declaration: an unanswered exception must not shadow it, because the
            //   filer answering the question would not make the payer's figure appear.
            f.exception_applies = None;
            assert_eq!(
                document_taxable(&f),
                Err(DocRefusal::TaxableAmountNotDetermined),
                "the payer's own blank is the prior fact; sending the filer to a question they can \
                 answer would be advice that does not help"
            );
        }
    }

    /// ★★★ C-2 — withholding is summed over EVERY row, both kinds. r1 found this box dropped entirely,
    /// which overstated the balance due by the whole of it.
    #[test]
    fn withholding_sums_across_both_kinds() {
        let mut a = ira(20_000, Some(false), "7", None);
        a.box4_fed_withheld = dec!(2_000);
        let mut b = pension(30_000, Some(24_000));
        b.box4_fed_withheld = dec!(3_000);
        assert_eq!(withholding_line_25b(&[a, b]), dec!(5_000));
        assert_eq!(withholding_line_25b(&[]), Usd::ZERO);
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
            exception_applies: None,
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
