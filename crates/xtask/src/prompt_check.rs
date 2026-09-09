//! `cargo run -p xtask -- prompt-check` — **FR-29 / SPEC §9 G7: the Form 8615 prompts and help
//! strings are TRANSCRIBED, and this is what holds them there.**
//!
//! The filer-facing text of Form 8615's three questions is the only place btctax puts the form's own
//! words in front of a person. `CLAUDE.md`'s standing rule is *transcribe, never paraphrase*, and a
//! prose review finds a paraphrase once; a check finds it forever.
//!
//! **Each clause is asserted TWICE, and both halves are load-bearing:**
//!
//! - **(a)** it appears verbatim (normalised) in **the extract that clause is sourced from**, and
//! - **(b)** it appears verbatim (normalised) in that question's `prompt` (or `help`).
//!
//! (b) alone lets the clause table drift from the form; (a) alone lets the prompt drift from the
//! table. Together, a paraphrase anywhere reds — the property harness rule **B1** calls *"cannot be
//! satisfied performatively"*.
//!
//! ★★ **The per-clause SOURCE column is not decoration.** i8615 says *"at least age 19 **and** under
//! age 24"* where i1040gi says *"at least age 19 **but** under age 24"* — same rule, one conjunction
//! apart. A single-extract table would either fail on that clause or silently check the prompt
//! against the wrong document.
//!
//! ★ **Normalisation is `cite_check::normalise`, named rather than described**: it folds Unicode
//! punctuation to ASCII, strips markdown emphasis and quote marks, removes the CAUTION/TIP icon
//! labels, de-hyphenates across line breaks, replaces every non-alphanumeric other than `$`, `%` and
//! whitespace with a space, collapses runs of whitespace, and lowercases. This module **reuses it
//! directly** and does not define a second one. Case-folding and whitespace-collapsing leave the B1
//! pairing red: *"most of your support"* still does not contain *"more than half of your support"*
//! under any amount of case folding.

use crate::cite_check::{normalise, repo_root};
use btctax_core::tax::provenance::DependentGate;
use btctax_core::tax::questions::{SkippableId, SKIPPABLE_QUESTIONS};
use std::fmt::Write as _;

/// Which of a question's two filer-facing strings a clause is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Face {
    Prompt,
    Help,
}

/// One clause: the question it belongs to, which of its strings carries it, the clause itself, and
/// **the extract it is sourced from**.
struct Clause {
    id: SkippableId,
    face: Face,
    text: &'static str,
    extract: &'static str,
}

/// SPEC §9 G7's clause table. Eight clauses, each with its own source.
const CLAUSES: &[Clause] = &[
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "under age 18",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "age 18",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "didn\u{2019}t have earned income that was more than half of your support",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "a full-time student at least age 19 but under age 24",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition4ParentAlive,
        face: Face::Prompt,
        text: "at least one of your parents was alive",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Help,
        text: "These rules apply whether or not the child is a dependent",
        extract: "design/forms/extract/i8615--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Help,
        text: "wages, tips, and other payments received for personal services performed",
        extract: "design/forms/extract/i8615--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615ParentIdentityUnobtainable,
        face: Face::Help,
        text: "The name, address, social security number (SSN) (if known), and filing status (if \
               known) of the parent",
        extract: "design/forms/extract/i8615--2025.txt",
    },
    // ── ★★★ R7 / T8 — the HoH MARITAL BASIS, the registry's second `Choice`. ────────────────────
    Clause {
        id: SkippableId::HohMaritalBasis,
        face: Face::Prompt,
        text: "if you are unmarried and provide a home for certain other persons",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::HohMaritalBasis,
        face: Face::Prompt,
        text: "considered unmarried for this purpose",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::HohMaritalBasis,
        face: Face::Prompt,
        text:
            "You were legally separated according to your state law under a decree of divorce or \
               separate maintenance at the end of",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::HohMaritalBasis,
        face: Face::Prompt,
        text: "You are married but lived apart from your spouse for the last 6 months of",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::HohMaritalBasis,
        face: Face::Prompt,
        text:
            "You are married and your spouse was a nonresident alien at any time during the year \
               and the election to treat the alien spouse as a resident alien is not made",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
];

/// ★★ **The STRUCTURAL half, and it is what actually pins the three limbs of condition 3.**
///
/// Clause 2 (`"age 18"`) is WEAK: it is a substring of `"under age 18"`, so clause 1 satisfies it and
/// **deleting limb (b) from the prompt would not red the clause table.** Limb (b) cannot be pinned
/// textually at all — the prompt hoists the year qualifier, so the extract's *"Age 18 at the end of
/// 2025 and"* and the prompt's *"age 18 and"* share no span longer than `"age 18"` itself.
///
/// So the prompt must also contain each limb OPENING exactly once, and the support clause exactly
/// twice (once for limb (b), once for limb (c) — which is how the form writes it). Deleting limb (b)
/// reds on both counts.
///
/// ★ **This half is PROMPT-ONLY, and the message says so**: these spans are the prompt's own hoisted
/// phrasing and are *not* in the extract, so it detects a limb being **dropped**, never a limb
/// **drifting** from the form. A structural assertion that looks like a conformance assertion is
/// precisely the green-and-blind instrument this whole check exists to avoid.
///
/// ★ Do **not** assert on the bare markers `(a)`/`(b)`/`(c)`: `normalise` strips the parentheses, and
/// the trailing sentence *"Answer YES if any one of (a), (b) or (c) is true."* makes their counts
/// 3/2/2, not 1/1/1.
const STRUCTURE: &[(&str, usize)] = &[
    (
        "didn\u{2019}t have earned income that was more than half of your support",
        2,
    ),
    ("(a) under age 18", 1),
    ("(b) age 18 and didn\u{2019}t have", 1),
    ("(c) a full-time student", 1),
];

fn face_text(id: SkippableId, face: Face) -> &'static str {
    let q = SKIPPABLE_QUESTIONS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("{id:?} is not in SKIPPABLE_QUESTIONS"));
    match face {
        Face::Prompt => q.prompt,
        Face::Help => q.help,
    }
}

/// Count non-overlapping occurrences of `needle` in `haystack` (both already normalised).
fn count(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut n = 0;
    let mut from = 0;
    while let Some(i) = haystack[from..].find(needle) {
        n += 1;
        from += i + needle.len();
    }
    n
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// T7 / R6 — THE DEPENDENT GATES. Same rule, one registry over.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **Row (5)(a)'s help carries the *Exception to time lived with you* VERBATIM, and the span is
/// ENUMERATED FROM THE EXTRACT'S OWN LINES rather than pasted here.**
///
/// R6 makes the exception part of the condition, not a branch out of the flowchart: a child born in
/// November whose home was the filer's for more than half the time they were alive answers the bare
/// question *No*, reaches Step 4, passes every qualifying-relative test, and prints the SMALLER
/// credit. So the exception has to be in front of the filer, in the instruction's own words.
///
/// ★★ **The comparand is the FILE, at the cited lines.** A pasted constant here would be a second
///      copy of the manual, and this whole module exists because a document does not need to agree
///      with itself — it needs to agree with the form. The line numbers are the ones the gate's own
///      `cite` names, so moving one without the other reds.
const EXCEPTION_SPAN: (&str, usize, usize) = ("design/forms/extract/i1040gi--2025.txt", 1905, 1913);

/// One dependent-gate clause: the gate, which of its two filer-facing strings carries it, and the
/// clause. Every one is sourced from `i1040gi--2025.txt`, the flowchart's own booklet.
struct GateClause {
    gate: DependentGate,
    face: Face,
    text: &'static str,
}

/// The clause table. One clause per flowchart CONDITION whose wording decides an edge — the words a
/// filer checks against their own facts.
const GATE_CLAUSES: &[GateClause] = &[
    GateClause {
        gate: DependentGate::QcRelationship,
        face: Face::Prompt,
        text: "Son, daughter, stepchild, foster child, brother, sister, stepbrother, stepsister, \
               half brother, half sister, or a descendant of any of them",
    },
    GateClause {
        gate: DependentGate::YoungerThanYouOrSpouse,
        face: Face::Help,
        text: "younger than you (or your spouse if filing jointly)",
    },
    GateClause {
        gate: DependentGate::FullTimeStudent,
        face: Face::Help,
        text: "It doesn\u{2019}t include an on-the-job training course, correspondence school, or \
               school offering courses only through the Internet",
    },
    GateClause {
        gate: DependentGate::PermanentlyAndTotallyDisabled,
        face: Face::Prompt,
        text: "can\u{2019}t engage in any substantial gainful activity because of a physical or \
               mental condition",
    },
    GateClause {
        gate: DependentGate::ProvidedOverHalfOwnSupport,
        face: Face::Help,
        text: "Who didn\u{2019}t provide over half of their own support",
    },
    GateClause {
        gate: DependentGate::JointReturnOnlyToClaimRefund,
        face: Face::Help,
        text: "only to claim a refund of withheld income tax or estimated tax paid",
    },
    GateClause {
        gate: DependentGate::QualifyingChildOfAnotherPerson,
        face: Face::Help,
        text: "If the child meets the conditions to be a qualifying child of any other person \
               (other than your spouse if filing jointly)",
    },
    GateClause {
        gate: DependentGate::CitizenNationalResidentOrCanadaMexico,
        face: Face::Prompt,
        text:
            "a U.S. citizen, U.S. national, U.S. resident alien, or a resident of Canada or Mexico",
    },
    GateClause {
        gate: DependentGate::TinIssuedByDueDate,
        face: Face::Prompt,
        text:
            "an SSN, ITIN, or adoption taxpayer identification number (ATIN) issued on or before \
               the due date of your return (including extensions)",
    },
    GateClause {
        gate: DependentGate::CitizenNationalOrResidentAlien,
        face: Face::Prompt,
        text: "a U.S. citizen, U.S. national, or U.S. resident alien",
    },
    GateClause {
        gate: DependentGate::SsnsValidForEmploymentIssuedByDueDate,
        face: Face::Prompt,
        text: "have SSNs valid for employment and issued before the due date of your",
    },
    GateClause {
        gate: DependentGate::QrRelationshipOrMemberOfHousehold,
        face: Face::Prompt,
        text:
            "Any other person (other than your spouse) who lived with you all year as a member of \
               your household if your relationship didn\u{2019}t violate local law",
    },
    GateClause {
        gate: DependentGate::QualifyingChildOfAnyTaxpayer,
        face: Face::Help,
        text: "Who wasn\u{2019}t a qualifying child (see Step 1) of any taxpayer",
    },
    GateClause {
        gate: DependentGate::GrossIncomeUnderLimit,
        face: Face::Help,
        text: "If the person was permanently and totally disabled, see Exception to gross income \
               test, later",
    },
    GateClause {
        gate: DependentGate::YouProvidedOverHalfSupport,
        face: Face::Help,
        text: "For whom you provided over half of the person\u{2019}s support",
    },
    GateClause {
        gate: DependentGate::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
        face: Face::Help,
        text:
            "But see Children of divorced or separated parents, Multiple support agreements, and \
               Kidnapped child, later",
    },
];

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// R7 / T8 — HEAD OF HOUSEHOLD and QUALIFYING SURVIVING SPOUSE. Same rule, the third registry over.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// One return-level declaration's clause. `rendered` says the comparand is the prompt AS RENDERED for
/// a return, not the static fallback — QSS condition 1 quotes a two-year window derived from
/// `tax_year`, and the static text deliberately names no year at all.
struct QuestionClause {
    id: btctax_core::tax::questions::QuestionId,
    text: &'static str,
    /// `Some(year)` ⇒ compare against `prompt_text` on a return of that year.
    rendered: Option<i32>,
    /// ★★★ **FR-107 — the extract this clause is SOURCED FROM, per clause.**
    ///
    /// It was a single hardcoded `i1040gi--2025.txt` for the whole table, which is precisely the
    /// shape [`Clause`]'s own doc warns against three hundred lines up — *"a single-extract table
    /// would either fail on that clause or silently check the prompt against the wrong document"*.
    /// The document census asks about documents whose words live in THEIR OWN instructions (Form
    /// 5498-SA's furnishing deadline is in `i1099sa`, and appears nowhere in the 1040 booklet), so
    /// the column had to become real before such a clause could be checked at all.
    extract: &'static str,
}

/// The Form 1040 instruction booklet — where all but the census clauses are sourced.
const I1040GI: &str = "design/forms/extract/i1040gi--2025.txt";

/// The clause table for R7's ten filer-facing strings.
///
/// ★★★ **Every span stops short of a YEAR and of a FIGURE, and that is the point.** The
/// instructions' own sentences name both (*"the main home for all of 2025"*, *"gross income of
/// $5,200 or more"*); a prompt that typed either would be a second copy of derived data — the year
/// is not necessarily this return's, and the figure lives in `FullReturnParams`. The one place a
/// year is quoted is QSS condition 1, where the window IS the question — and there it is RENDERED
/// from `tax_year`, so this table checks the rendered sentence against the instruction's own.
const QUESTION_CLAUSES: &[QuestionClause] = &[
    // ★★★ **TEST 1's OPERATIVE CONDITION — added by the T8 seam-review fold (M-2's sweep).** T8
    //     quoted the two NAMES in Test 1 and Test 2's whole condition, but left Test 1's own
    //     condition — the half that says what the filer must have PAID — as an unchecked paraphrase
    //     beside them. The span stops one word short of the instruction's year (*"the main home for
    //     all of 2025 of your parent"*), which is why it ends at *"the main home"*: the prompts type
    //     no year, and this table is what keeps that honest.
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::HohQualifyingPerson,
        text: "You paid over half the cost of keeping up a home that was the main home",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::HohQualifyingPerson,
        text: "your parent whom you can claim as a dependent, except under a multiple support \
               agreement",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::HohQualifyingPerson,
        text: "Your parent didn\u{2019}t have to live with you.",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::HohQualifyingPerson,
        text:
            "you paid over half the cost of keeping up a home in which you lived and in which one \
               of the following also lived for more than half of the year",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::HohPaidOverHalfCostOfKeepingUpHome,
        text: "You paid over half the cost of keeping up a home",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::NraSpouseResidentElection,
        text: "you and your spouse can choose to be treated as U.S. residents for the entire year \
               and file a joint return",
        rendered: None,
        extract: I1040GI,
    },
    // ★★★ THE RENDERED ONE. TY2025's window is the instruction's own printed sentence, verbatim.
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::QssSpouseDiedInWindowAndNotRemarried,
        text:
            "Your spouse died in 2023 or 2024 and you didn\u{2019}t remarry before the end of 2025.",
        rendered: Some(2025),
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::QssChildYouCanClaim,
        text:
            "You have a child or stepchild (not a foster child) whom you can claim as a dependent",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::QssChildYouCanClaim,
        text: "The child filed a joint return",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::QssChildLivedInYourHomeAllYear,
        text: "This child lived in your home for all of",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::QssPaidOverHalfCostOfKeepingUpHome,
        text: "You paid over half the cost of keeping up your home.",
        rendered: None,
        extract: I1040GI,
    },
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::QssCouldHaveFiledJointlyInYearOfDeath,
        text:
            "You could have filed a joint return with your spouse the year your spouse died, even \
               if you didn\u{2019}t actually do so.",
        rendered: None,
        extract: I1040GI,
    },
    // ── ★★★ FR-107 — THE FORM 5498-SA CENSUS ROW'S TIMING CLAUSE. ───────────────────────────────
    //
    //     The row asks a yes/no about a document that CANNOT have arrived by the filing deadline,
    //     and the prompt now says so in the instruction's own words rather than in a paraphrase a
    //     reader would have to take on trust. Sourced from `i1099sa`, which is why this table needed
    //     a per-clause `extract` at all: the sentence appears nowhere in the 1040 booklet.
    //
    // ★ The span stops short of the YEAR the instructions print (*"by June 1, 2026"*), for the same
    //   reason every other span here does — the prompt is asked of every year, and typing one would
    //   be a second copy of derived data.
    QuestionClause {
        id: btctax_core::tax::questions::QuestionId::DocSa5498,
        text: "you must provide a statement to the participant (generally Copy B) by June 1",
        rendered: None,
        extract: "design/forms/extract/i1099sa--2025.txt",
    },
];

/// The HoH MARITAL BASIS lives in `SKIPPABLE_QUESTIONS` (it is a `Choice`), so its clauses go in
/// [`CLAUSES`]'s shape rather than [`QUESTION_CLAUSES`]'s — see the two entries added there.
///
/// The R7 half of the check: `Ok(n)` assertions passed, `Err` names every failure.
fn check_questions(root: &std::path::Path) -> Result<usize, String> {
    use btctax_core::tax::questions::FORM_QUESTIONS;
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;
    for (i, c) in QUESTION_CLAUSES.iter().enumerate() {
        let n = i + 1;
        let clause = normalise(c.text);
        // (a) — the clause really is the manual's. ★ FR-107: the manual is named PER CLAUSE, so a
        //       census row's clause is checked against ITS OWN document's instructions.
        let extract = c.extract;
        let raw = std::fs::read_to_string(root.join(extract))
            .map_err(|e| format!("cannot read {extract}: {e}"))?;
        let hay = normalise(&raw);
        if hay.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "question clause {n} ({:?}) is NOT in {extract}: {:?}",
                c.id, c.text
            ));
        }
        // (b) — and it is what the filer is shown. For a RENDERED prompt the comparand is the
        //       sentence a return of that year would actually be asked, never the static fallback.
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == c.id)
            .ok_or_else(|| format!("{:?} is not in FORM_QUESTIONS", c.id))?;
        let face = match c.rendered {
            None => normalise(q.prompt),
            Some(year) => {
                let ri = btctax_core::tax::return_inputs::ReturnInputs {
                    tax_year: year,
                    ..Default::default()
                };
                normalise(&q.prompt_text(&ri))
            }
        };
        if face.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "question clause {n} ({:?}) is NOT in the string the filer reads: {:?}",
                c.id, c.text
            ));
        }
    }
    if failures.is_empty() {
        Ok(passed)
    } else {
        let mut msg = String::new();
        for f in &failures {
            let _ = writeln!(msg, "  {f}");
        }
        Err(msg)
    }
}

/// Which of a gate's two filer-facing strings a clause is checked against.
fn gate_face_text(gate: DependentGate, face: Face) -> &'static str {
    let q = btctax_core::tax::dependent_gates::entry(gate);
    match face {
        Face::Prompt => q.prompt,
        Face::Help => q.help,
    }
}

/// The cited span of the extract, normalised — read from the FILE, never pasted.
fn cited_span(
    root: &std::path::Path,
    file: &str,
    first: usize,
    last: usize,
) -> Result<String, String> {
    let raw =
        std::fs::read_to_string(root.join(file)).map_err(|e| format!("cannot read {file}: {e}"))?;
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() < last {
        return Err(format!("{file} has {} lines, want :{last}", lines.len()));
    }
    Ok(normalise(&lines[first - 1..last].join(" ")))
}

/// The T7 half of the check: `Ok(n)` assertions passed, `Err` names every failure.
fn check_gates(root: &std::path::Path) -> Result<usize, String> {
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;
    let extract = "design/forms/extract/i1040gi--2025.txt";
    let raw = std::fs::read_to_string(root.join(extract))
        .map_err(|e| format!("cannot read {extract}: {e}"))?;
    let hay = normalise(&raw);
    for (i, c) in GATE_CLAUSES.iter().enumerate() {
        let n = i + 1;
        let clause = normalise(c.text);
        // (a) — the clause really is the manual's.
        if hay.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "gate clause {n} ({:?}) is NOT in {extract}: {:?}",
                c.gate, c.text
            ));
        }
        // (b) — and it is what the filer is shown.
        if normalise(gate_face_text(c.gate, c.face)).contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "gate clause {n} ({:?} {:?}) is NOT in the string the filer reads: {:?}",
                c.gate, c.face, c.text
            ));
        }
    }
    // Row (5)(a)'s exception, verbatim, at its own cited lines.
    let (file, first, last) = EXCEPTION_SPAN;
    let span = cited_span(root, file, first, last)?;
    if exception_is_quoted(
        &span,
        gate_face_text(DependentGate::LivedWithYouOverHalfYear, Face::Help),
    ) {
        passed += 1;
    } else {
        failures.push(format!(
            "row (5)(a)'s help does NOT quote {file}:{first}-{last} verbatim \u{2014} the Exception \
             to time lived with you is PART OF THE CONDITION (R6), and a child born in November \
             whose help omits it answers the bare question \"no\" and prints the smaller credit"
        ));
    }
    // …and the gate's own `cite` names those lines, so the two cannot drift apart.
    let cite =
        btctax_core::tax::dependent_gates::entry(DependentGate::LivedWithYouOverHalfYear).cite;
    if cite.contains(&format!(":{first}-{last}")) {
        passed += 1;
    } else {
        failures.push(format!(
            "row (5)(a)'s `cite` must name :{first}-{last}, the span this check reads: got {cite:?}"
        ));
    }
    if failures.is_empty() {
        Ok(passed)
    } else {
        let mut msg = String::new();
        for f in &failures {
            let _ = writeln!(msg, "  {f}");
        }
        Err(msg)
    }
}

/// THE RULE, as a pure function of its two inputs, so a planted defect can reach it (B1).
fn exception_is_quoted(cited_span: &str, help: &str) -> bool {
    normalise(help).contains(cited_span)
}

/// Run the check over the live registry. `Ok(count)` is the number of clause assertions that passed
/// (both halves each); `Err` names every failure.
pub fn check() -> Result<usize, String> {
    let root = repo_root();
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;

    for (i, c) in CLAUSES.iter().enumerate() {
        let n = i + 1;
        let clause = normalise(c.text);
        // (a) — against the extract this clause is SOURCED FROM, named per clause.
        let path = root.join(c.extract);
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("clause {n}: cannot read {}: {e}", c.extract))?;
        let hay = normalise(&raw);
        if hay.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "clause {n} ({:?}) is NOT in its own source {}: {:?}",
                c.id, c.extract, c.text
            ));
        }
        // (b) — against the filer-facing string it belongs to.
        let face = normalise(face_text(c.id, c.face));
        if face.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "clause {n} ({:?} {:?}) is NOT in the string the filer reads: {:?}",
                c.id, c.face, c.text
            ));
        }
    }

    // The structural half — PROMPT-ONLY (see `STRUCTURE`).
    let prompt = normalise(face_text(
        SkippableId::Form8615Condition3AgeSupport,
        Face::Prompt,
    ));
    for (span, want) in STRUCTURE {
        let got = count(&prompt, &normalise(span));
        if got == *want {
            passed += 1;
        } else {
            failures.push(format!(
                "STRUCTURAL (prompt-only \u{2014} this detects a limb being DROPPED, never a limb \
                 DRIFTING from the form): condition 3's prompt contains {span:?} {got} time(s), \
                 want {want}"
            ));
        }
    }

    if failures.is_empty() {
        Ok(passed + check_gates(&root)? + check_questions(&root)?)
    } else {
        let mut msg = String::new();
        for f in &failures {
            let _ = writeln!(msg, "  {f}");
        }
        Err(msg)
    }
}

pub fn run() -> Result<(), String> {
    let passed = check()?;
    println!("xtask prompt-check: OK \u{2014} {passed} assertions, all verbatim");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **FR-107's B1 PAIRING — the Form 5498-SA timing clause, watched RED on the wording it
    /// replaced.**
    ///
    /// The journey walk found the one census row whose y/n a filer cannot answer truthfully: Form
    /// 5498-SA is furnished AFTER the filing deadline, so an early filer can never say *Yes*, while
    /// *No* reads as *"I will never receive one"*. The reword answers it by saying what the
    /// instructions say — and a citation nobody checks is a paraphrase waiting to happen, which is
    /// this module's whole reason for existing.
    ///
    /// ★ Three states, and the middle one is the kill: GREEN on the shipped prompt, RED on the
    ///   PRE-FR-107 wording (pasted verbatim below — the actual text that shipped), RED on a
    ///   plausible softening of the deadline itself.
    #[test]
    fn the_5498sa_prompt_quotes_the_furnishing_deadline_and_the_old_wording_reds() {
        use btctax_core::tax::questions::{QuestionId, FORM_QUESTIONS};
        let root = repo_root();
        let c = QUESTION_CLAUSES
            .iter()
            .find(|c| c.id == QuestionId::DocSa5498)
            .expect("FR-107 added the 5498-SA clause");
        let clause = normalise(c.text);

        // (a) the clause really is the instruction's, in ITS OWN document.
        let raw = std::fs::read_to_string(root.join(c.extract)).expect("the extract is committed");
        assert!(
            normalise(&raw).contains(&clause),
            "the timing clause must be `i1099sa`'s own sentence, not a paraphrase of it"
        );

        // (b) GREEN on the shipped prompt.
        let prompt = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::DocSa5498)
            .expect("the census row is a registry question")
            .prompt;
        assert!(
            normalise(prompt).contains(&clause),
            "the unmutated prompt must PASS — a checker that reds on everything is \
             indistinguishable from one that works"
        );

        // (c) RED on the wording FR-107 replaced. It asks about RECEIPT, in the past tense, and
        //     cites nothing — so the filer holding no form has no way to know that *No* is the
        //     truthful answer rather than a claim they will never get one.
        const PRE_FR107: &str =
            "Did you receive one or more Form 5498-SA (the trustee of a health \
                                 savings account sends one reporting the year's contributions and \
                                 the account's fair market value — see the instructions for Forms \
                                 1099-SA and 5498-SA)?";
        assert!(
            !normalise(PRE_FR107).contains(&clause),
            "PLANTED DEFECT NOT CAUGHT: the prompt the walk was asked passed the check that exists \
             to hold the reword in place"
        );

        // (d) RED on a softening of the deadline — the shape an editor tidying prose would commit.
        let softened = prompt.replace("(generally Copy B) by June 1", "in June");
        assert_ne!(softened, prompt, "the mutation must change the prompt");
        assert!(
            !normalise(&softened).contains(&clause),
            "PLANTED DEFECT NOT CAUGHT: the deadline was softened out of the instruction's own \
             words and the check still passed"
        );

        // ★ …and the answerable half, which is what the finding was actually about: the prompt asks
        //   what the filer HAS, and says plainly what a No costs them (nothing).
        for needle in [
            "IN HAND",
            "if it has not arrived, answer No",
            "no line of Form 8889 reads",
        ] {
            assert!(
                prompt.contains(needle),
                "the prompt must be answerable at the keyboard on filing day — missing {needle:?}: \
                 {prompt}"
            );
        }
    }

    /// ★★★ **T7 / R6 — THE B1 PAIRING FOR ROW (5)(a): the verbatim check is observed RED on a
    /// planted defect and GREEN on the shipped help.**
    ///
    /// The rule is `exception_is_quoted(cited_span, help)`, a pure function of its two inputs, so the
    /// mutation reaches it without touching the repo. Three defects are planted, and each is one a
    /// well-meaning editor would actually commit:
    ///
    /// 1. **a paraphrase** — *"count as time the person lived with you"* softened to *"may count"*;
    /// 2. **a TRUNCATION** — the born-or-died sentence dropped, which is the whole reason the
    ///    exception is in the help at all (a November baby answers the bare question *no*);
    /// 3. **the empty help**, the degenerate case a `contains` check must not pass.
    #[test]
    fn the_row_five_a_help_quotes_the_exception_and_a_mutated_help_reds() {
        let root = repo_root();
        let (file, first, last) = EXCEPTION_SPAN;
        let span = cited_span(&root, file, first, last).expect("the extract is committed");
        assert!(
            span.contains("exception to time lived with you")
                && span.contains("born or died in 2025"),
            "the CITED SPAN itself must be the exception \u{2014} if this fails the line numbers \
             moved, and every assertion below would be checking the wrong paragraph: {span:?}"
        );
        let help = gate_face_text(DependentGate::LivedWithYouOverHalfYear, Face::Help);

        // GREEN on the shipped help.
        assert!(
            exception_is_quoted(&span, help),
            "the unmutated help must PASS \u{2014} a checker that reds on everything is \
             indistinguishable from one that works"
        );

        // RED on a paraphrase.
        let paraphrased = help.replace(
            "count as time the person lived with you",
            "may count as time the person lived with you",
        );
        assert_ne!(paraphrased, help, "the mutation must change the help");
        assert!(
            !exception_is_quoted(&span, &paraphrased),
            "PLANTED DEFECT NOT CAUGHT: a one-word paraphrase of the exception passed"
        );

        // RED on a TRUNCATION \u{2014} the born-or-died sentence is the one that decides a November baby.
        let cut = help
            .find("If the person meets all other requirements")
            .expect("the born-or-died sentence is in the help");
        let truncated = &help[..cut];
        assert!(
            !exception_is_quoted(&span, truncated),
            "PLANTED DEFECT NOT CAUGHT: the born-or-died sentence was dropped and the check still \
             passed \u{2014} which is exactly the misroute R6 exists to prevent"
        );

        // RED on the degenerate case.
        assert!(
            !exception_is_quoted(&span, ""),
            "PLANTED DEFECT NOT CAUGHT: an EMPTY help passed a `contains` check"
        );
    }

    /// ★★★ **The gate clause table is observed RED too**, on the same shape of defect the Form 8615
    /// table is: a real sentence replaced by a plausible paraphrase of it.
    #[test]
    fn a_paraphrased_gate_prompt_is_rejected() {
        let real = gate_face_text(DependentGate::TinIssuedByDueDate, Face::Prompt);
        let clause = normalise(
            "an SSN, ITIN, or adoption taxpayer identification number (ATIN) issued on or before \
             the due date of your return (including extensions)",
        );
        assert!(
            normalise(real).contains(&clause),
            "GREEN on the real prompt"
        );
        // "by the due date" is not "on or before the due date" \u{2014} and the difference is a day.
        let paraphrased = real.replace("on or before the due date", "by the due date");
        assert_ne!(paraphrased, real, "the mutation must change the prompt");
        assert!(
            !normalise(&paraphrased).contains(&clause),
            "PLANTED DEFECT NOT CAUGHT: the paraphrase passed the gate clause table"
        );
    }

    /// ★★ **The `cite` and the span this check reads are the SAME lines.** Without this, moving
    /// the cite (or the span) would leave the check silently reading a different paragraph from the
    /// one the gate claims to quote \u{2014} the FOLLOWUPS \u{a7}G-10 shape, one registry over.
    #[test]
    fn the_row_five_a_cite_names_the_span_this_check_reads() {
        let (_, first, last) = EXCEPTION_SPAN;
        let cite =
            btctax_core::tax::dependent_gates::entry(DependentGate::LivedWithYouOverHalfYear).cite;
        assert!(
            cite.contains(&format!(":{first}-{last}")),
            "the gate cites {cite:?} but this check reads :{first}-{last}"
        );
    }

    /// The check passes against the live registry.
    #[test]
    fn the_real_prompts_pass() {
        match check() {
            Ok(n) => assert!(n >= 20, "expected at least 20 assertions, got {n}"),
            Err(e) => panic!("prompt-check failed on the real prompts:\n{e}"),
        }
    }

    /// ★★★ **THE B1 PAIRING (harness rule B1, `CLAUDE.md`): the check is observed RED on a planted
    /// defect, AND observed GREEN on the real thing.**
    ///
    /// Modelled on `cite_check::a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`. The
    /// planted defect is the one the SPEC names: *"more than half of your support"* paraphrased to
    /// *"most of your support"*. That is a real paraphrase a well-meaning editor would make, and it
    /// changes the test the filer applies.
    ///
    /// ★ **Both directions are asserted**, because a checker that rejects everything is
    /// indistinguishable from one that works. The mutation is applied to a COPY of the prompt rather
    /// than to the registry, so the harness cannot leave the repo mutated.
    #[test]
    fn prompt_check_rejects_a_paraphrased_prompt_and_accepts_the_real_one() {
        let real = face_text(SkippableId::Form8615Condition3AgeSupport, Face::Prompt);
        let clause =
            normalise("didn\u{2019}t have earned income that was more than half of your support");

        // GREEN on the real one.
        assert!(
            normalise(real).contains(&clause),
            "the unmutated prompt must PASS — a checker that reds on everything is \
             indistinguishable from one that works"
        );

        // RED on the paraphrase. Case-folding and whitespace-collapsing do not save it: "most of
        // your support" does not contain "more than half of your support" under any amount of either.
        let paraphrased = real.replace("more than half of your support", "most of your support");
        assert_ne!(
            paraphrased, real,
            "the mutation must actually change the prompt"
        );
        assert!(
            !normalise(&paraphrased).contains(&clause),
            "PLANTED DEFECT NOT CAUGHT: the paraphrase 'most of your support' passed the clause \
             check, so this instrument has never been seen discriminating and does not exist (B1)"
        );

        // …and the structural half reds on the OTHER named mutation: dropping limb (b).
        let without_b = real.replace(
            "(b) age 18 and didn\u{2019}t have earned income that was more than half of your \
             support, or ",
            "",
        );
        let dropped = normalise(&without_b);
        assert_ne!(
            dropped,
            normalise(real),
            "the limb-(b) mutation must actually change the prompt"
        );
        assert_eq!(
            count(&dropped, &normalise("(b) age 18 and didn\u{2019}t have")),
            0,
            "PLANTED DEFECT NOT CAUGHT: limb (b) was deleted and the structural count did not move \
             \u{2014} which is exactly the hole clause 2 (\"age 18\", a substring of \"under age \
             18\") leaves in the clause table"
        );
        assert_eq!(
            count(&dropped, &clause),
            1,
            "…and the support clause drops from twice to once"
        );
    }
}
