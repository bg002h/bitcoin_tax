//! Full-return v1 **packet** (P6.1): the identity header + the assembled printed return.
//!
//! Two things live here, and both exist so that the *filed* artifact is derived exactly once:
//!
//! - [`ReturnHeader`] — who the return is for. Every IRS form carries a name + SSN header, and the
//!   semantics are NOT uniform: "Name(s) shown on return" is the **joint** name line on an MFJ return,
//!   but Schedule C wants the **proprietor alone** with that person's SSN. Deriving it once here means a
//!   filler can only transcribe, never decide.
//! - [`PrintedReturn`] — every form's printed line chain, composed in dependency order (SPEC §3.1).
//!
//! **The composition is the tax semantics.** Schedule 2 line 11 is Form 8959's *printed* line 18, and
//! 1040 line 23 is Schedule 2's *printed* line 21 — so the packet must be assembled where that knowledge
//! belongs (core), not in `btctax-forms` (which does zero tax arithmetic) and not in the CLI (which is
//! the one place core's composition KATs cannot reach). `assemble_printed_return` is the single
//! composition site; the KATs below go *through* it, so the tested wiring is the shipped wiring.

use crate::donation::DonationDetails;
use crate::event::LedgerEvent;
use crate::identity::EventId;
use crate::state::LedgerState;
use crate::tax::other_taxes::{form_8959_lines, form_8960_lines, Form8959Lines, Form8960Lines};
use crate::tax::printed::{
    form_1040_income_lines, form_1040_lines, form_8949_printed, schedule_1_lines, schedule_2_lines,
    schedule_3_lines, schedule_a_lines, schedule_b_lines, schedule_c_lines, schedule_d_lines,
    schedule_se_lines, Form1040Lines, Printed8949, Schedule1Lines, Schedule2Lines, Schedule3Lines,
    ScheduleALines, ScheduleBLines, ScheduleCLines, ScheduleDLines, ScheduleSeLines,
};
use crate::tax::printed::{form_8283_printed, Printed8283Rows, FORM_8283_THRESHOLD};
use crate::tax::printed::{printed_8275, Printed8275};
use crate::tax::qbi::{form_8995_lines, Form8995Lines};
use crate::tax::questions::{QuestionId, FORM_QUESTIONS};
use crate::tax::return_1040::{is_aged, AbsoluteReturn};
use crate::tax::return_inputs::{Owner, Person, ReturnInputs};
use crate::tax::tables::TaxTable;
use crate::tax::types::FilingStatus;
use std::collections::BTreeMap;
use std::fmt;

// ── Identity ────────────────────────────────────────────────────────────────────────────────────

/// A canonical U.S. Social Security Number: **exactly nine digits**, however the human typed it.
///
/// The raw `Person.ssn` is stored AS ENTERED (`123-45-6789`, `123456789`, or with stray spaces). A form
/// cell is not so forgiving, and the forms do not even agree with each other: the **1040's** SSN widgets
/// are 9-character combs (`/MaxLen 9` — bare digits), while every **schedule's** is `/MaxLen 11` (the
/// hyphenated form). A value that does not fit is silently truncated by the viewer, so the rendering is
/// chosen per-cell from the PDF's own declared capacity (`btctax_forms::cells::render_ssn`), never
/// assumed. Canonicalization happens ONCE, here, and fails loudly — §3.4: an SSN that cannot be printed
/// is an uncomputable line, not a best-effort cell.
///
/// `Debug` is **masked**: an SSN that leaks into a log or a panic message is a PII incident, and the
/// derived `Debug` on every struct that transitively holds one would do exactly that.
#[derive(Clone, PartialEq, Eq)]
pub struct Ssn(String);

impl Ssn {
    /// Strip formatting (hyphens, whitespace), then require exactly nine digits.
    pub fn canonical(raw: &str) -> Result<Self, SsnError> {
        let digits: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect();
        if digits.is_empty() {
            return Err(SsnError::Missing);
        }
        if let Some(c) = digits.chars().find(|c| !c.is_ascii_digit()) {
            return Err(SsnError::NotDigits(c));
        }
        if digits.len() != 9 {
            return Err(SsnError::WrongLength(digits.len()));
        }
        Ok(Self(digits))
    }

    /// The nine bare digits — for a 9-character cell.
    pub fn digits(&self) -> &str {
        &self.0
    }

    /// `NNN-NN-NNNN` — for an 11-character comb cell (exactly 11 characters, by construction).
    pub fn hyphenated(&self) -> String {
        format!("{}-{}-{}", &self.0[0..3], &self.0[3..5], &self.0[5..9])
    }
}

impl fmt::Debug for Ssn {
    /// Masked — never print an SSN, not even in a panic.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ssn(***-**-{})", &self.0[5..9])
    }
}

/// Why a captured SSN is not an SSN. Carries no digits (it is rendered into a user-facing refusal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsnError {
    /// No SSN was captured at all.
    Missing,
    /// A character that is neither a digit nor formatting.
    NotDigits(char),
    /// Some number of digits other than nine.
    WrongLength(usize),
}

impl fmt::Display for SsnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => write!(f, "no SSN was entered"),
            Self::NotDigits(c) => write!(f, "contains {c:?}, which is not a digit"),
            Self::WrongLength(n) => write!(f, "has {n} digits — an SSN has exactly 9"),
        }
    }
}

/// Why a [`ReturnHeader`] cannot be built — the fail-closed PRINT boundary (P9 §3.2, P8a I3). `SsnError`
/// alone cannot say all of this: a header fails to build not only on a malformed identity but also when a
/// live class-(A) DECLARATION is unanswered (an unanswered box must never reach a filed form), or when a
/// joint return carries no spouse identity to fill the joint header cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// An SSN in the household (taxpayer, spouse, or a dependent) cannot be canonicalized.
    Ssn(SsnError),
    /// ★ A live DECLARATION is unanswered. At PRINT there is no conservative direction — an unchecked box is
    /// a false "No" and a checked box is a false "Yes" — so refusal is the ONLY fail-closed behaviour. This
    /// is the second boundary behind `screen_inputs` (P8a I3): even a caller that skips the screen cannot
    /// print an unaffirmed box, and it closes the Schedule B Part III `unwrap_or(false)` print site.
    Unanswered(QuestionId),
    /// A joint (MFJ) return with no spouse `Person` — the joint name line and the spouse SSN cell cannot be
    /// filled (r3 M-6). Distinct from `Unanswered`: the spouse DECLARATION may be answered, yet the spouse
    /// IDENTITY is still absent.
    MfjWithoutSpouse,
}

impl From<SsnError> for HeaderError {
    fn from(e: SsnError) -> Self {
        HeaderError::Ssn(e)
    }
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ssn(e) => write!(f, "an SSN {e} — fix the identity and re-run"),
            Self::Unanswered(id) => write!(
                f,
                "the {id:?} question is unanswered, and an unanswered box must not reach a filed form — \
                 run `btctax income answer`"
            ),
            Self::MfjWithoutSpouse => write!(
                f,
                "a married-filing-jointly return has no spouse on file — the joint name and SSN cannot be \
                 printed; add the spouse's identity (`btctax set-pii`) or change the filing status"
            ),
        }
    }
}

/// An IRS **Identity Protection PIN** — six digits, and PII.
///
/// A paper return that omits an issued IP PIN is rejected or delayed, so this is the one header omission
/// with a concrete processing consequence (ARCH-P6.3a Q7 item 5). `Debug` is **masked**, exactly like
/// [`Ssn`]: a PIN in a log or a panic message is an identity-theft credential in a log.
#[derive(Clone, PartialEq, Eq)]
pub struct IpPin(String);

impl IpPin {
    /// Six digits, however typed (spaces stripped). Anything else is not an IP PIN.
    pub fn canonical(raw: &str) -> Result<Self, SsnError> {
        let digits: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        if digits.is_empty() {
            return Err(SsnError::Missing);
        }
        if let Some(c) = digits.chars().find(|c| !c.is_ascii_digit()) {
            return Err(SsnError::NotDigits(c));
        }
        if digits.len() != 6 {
            return Err(SsnError::WrongLength(digits.len()));
        }
        Ok(Self(digits))
    }

    /// The six digits — the form's cell is a 6-character comb.
    pub fn digits(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for IpPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpPin(******)")
    }
}

/// A person as they appear ON the filed return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiledPerson {
    pub first_name: String,
    pub last_name: String,
    pub ssn: Ssn,
    /// The signature-block occupation (a nicety; blank is acceptable).
    pub occupation: String,
}

impl FiledPerson {
    fn build(p: &Person) -> Result<Self, SsnError> {
        Ok(Self {
            first_name: p.first_name.clone(),
            last_name: p.last_name.clone(),
            ssn: Ssn::canonical(&p.ssn)?,
            occupation: p.occupation.clone(),
        })
    }

    /// "First Last" — one cell, for the forms whose header is a single name field.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

/// A dependent's row on the 1040. The CTC/ODC credit boxes are deliberately NOT modeled here: v1 omits
/// the credit entirely (L19 = 0, with the `CtcOdcOmitted` advisory), and a checked credit box beside a
/// zero credit is a form contradicting itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependentRow {
    pub name: String,
    pub ssn: Ssn,
    pub relationship: String,
}

/// The four §63(f) aged/blind checkboxes on 1040 page 1.
///
/// ★ These are **load-bearing, not decorative**. The IRS validates a nonstandard standard deduction by
/// COUNTING the checked boxes: L12 must equal the basic deduction plus `count()` × the per-box amount.
/// A return that claims the addition without checking the boxes fails the Service's own arithmetic
/// cross-check. That is why [`Self::count`] is the single source core's own L12 consumes — the checkbox
/// count and the deduction are derived from one place, so they cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgedBlindBoxes {
    pub taxpayer_aged: bool,
    pub taxpayer_blind: bool,
    pub spouse_aged: bool,
    pub spouse_blind: bool,
}

impl AgedBlindBoxes {
    /// The §63(f) boxes for a return. The spouse's boxes count **only on a joint return** — on MFS the
    /// spouse's blindness is not the taxpayer's checkbox, and no other status has a spouse at all.
    pub fn for_return(ri: &ReturnInputs, year: i32) -> Self {
        let t = &ri.header.taxpayer;
        let joint_spouse = ri
            .header
            .spouse
            .as_ref()
            .filter(|_| ri.filing_status == FilingStatus::Mfj);
        Self {
            taxpayer_aged: is_aged(t.date_of_birth, year),
            taxpayer_blind: t.blind == Some(true),
            spouse_aged: joint_spouse.is_some_and(|s| is_aged(s.date_of_birth, year)),
            spouse_blind: joint_spouse.is_some_and(|s| s.blind == Some(true)),
        }
    }

    /// How many boxes are checked (0–4) — the multiplier on the §63(f) per-box addition.
    pub fn count(&self) -> u32 {
        u32::from(self.taxpayer_aged)
            + u32::from(self.taxpayer_blind)
            + u32::from(self.spouse_aged)
            + u32::from(self.spouse_blind)
    }
}

/// Who the return is for — derived ONCE, so a filler can only transcribe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnHeader {
    /// "Name(s) shown on return" — the **joint** line on MFJ, the taxpayer alone otherwise. This is the
    /// string every attached schedule carries at its top… except Schedule C (see `proprietor`).
    pub name_line: String,
    pub taxpayer: FiledPerson,
    /// Present whenever the household has one — including MFS, where the spouse's name has its own 1040
    /// cell but is NOT part of `name_line`.
    pub spouse: Option<FiledPerson>,
    pub address_street: String,
    pub address_city: String,
    pub address_state: String,
    pub address_zip: String,
    pub aged_blind: AgedBlindBoxes,
    /// 1040 "Someone can claim: **You** as a dependent". Load-bearing, not decorative: when this is set,
    /// core's L12 uses the §63(c)(5) DEPENDENT FLOOR instead of the basic standard deduction. A return
    /// that claims the smaller deduction without checking the box is a form contradicting its own
    /// arithmetic — the same defect class as the aged/blind boxes.
    pub claimed_as_dependent_taxpayer: bool,
    /// 1040 "Someone can claim: **Your spouse** as a dependent". (A claimable spouse REFUSES upstream —
    /// `DependentSpouseUnsupported` — so this is captured for completeness and never reaches a filed
    /// return in v1.)
    pub claimed_as_dependent_spouse: bool,
    /// 1040 "**Spouse itemizes** on a separate return or you were a dual-status alien" — the §63(c)(6)
    /// MFS coupling core already applies to L12. Same class again: the arithmetic is visible on the
    /// form only if the box is checked.
    pub mfs_spouse_itemizes: bool,
    /// The §6096 Presidential Election Campaign boxes (you / spouse). Pure election — it changes neither
    /// tax nor refund — but it is CAPTURED input, and a captured election that silently fails to print
    /// is a return that does not say what the filer said.
    pub presidential_fund_taxpayer: bool,
    pub presidential_fund_spouse: bool,
    /// The taxpayer's IRS-issued Identity Protection PIN, when they have one. A paper return that omits
    /// an issued IP PIN is REJECTED or delayed (ARCH-P6.3a Q7 item 5). The spouse's IP PIN is not
    /// captured by `ReturnInputs` at all — a capture gap, recorded in LIMITATIONS rather than fabricated.
    pub ip_pin: Option<IpPin>,
    pub dependents: Vec<DependentRow>,
    /// Schedule C's header is "Name of **proprietor**", not the return's name line: a spouse-owned
    /// business files under the SPOUSE's name and SSN even on a joint return. `None` when there is no
    /// Schedule C. (v1 has at most one Schedule C — ≥ 2 SE earners already refuse, §4.4a.)
    pub proprietor: Option<FiledPerson>,
}

impl ReturnHeader {
    /// Build the header from the captured household. Fails if ANY SSN in it — taxpayer, spouse, or a
    /// dependent — cannot be canonicalized: fail-closed, since a return cannot be built around an
    /// identity that cannot be printed.
    pub fn build(ri: &ReturnInputs, year: i32) -> Result<Self, HeaderError> {
        // ★ P8a I3 — the fail-closed print boundary. Every live class-(A) declaration must be answered
        // before any form is composed: an unchecked box is a false "No" and a checked box a false "Yes",
        // so there is no conservative direction at print. This is the second boundary behind
        // `screen_inputs`; it also closes the Schedule B Part III `unwrap_or(false)` print site.
        for q in FORM_QUESTIONS {
            if (q.live)(ri) && (q.get)(ri).is_none() {
                return Err(HeaderError::Unanswered(q.id));
            }
        }
        // r3 M-6 — a joint return needs a spouse IDENTITY to fill the joint name line and the spouse SSN
        // cell; the spouse DECLARATION being answered is not enough.
        if ri.filing_status == FilingStatus::Mfj && ri.header.spouse.is_none() {
            return Err(HeaderError::MfjWithoutSpouse);
        }

        let taxpayer = FiledPerson::build(&ri.header.taxpayer)?;
        let spouse = ri
            .header
            .spouse
            .as_ref()
            .map(FiledPerson::build)
            .transpose()?;

        let name_line = match (ri.filing_status, &spouse) {
            (FilingStatus::Mfj, Some(s)) => format!("{} & {}", taxpayer.full_name(), s.full_name()),
            _ => taxpayer.full_name(),
        };

        let proprietor = match ri.schedule_c.as_ref().map(|c| c.owner) {
            None => None,
            Some(Owner::Taxpayer) => Some(taxpayer.clone()),
            // A spouse-owned Schedule C with no spouse on the return is already refused upstream
            // (`Owner::Spouse` on a non-joint return); fall back to the taxpayer rather than panic.
            Some(Owner::Spouse) => Some(spouse.clone().unwrap_or_else(|| taxpayer.clone())),
        };

        let dependents = ri
            .header
            .dependents
            .iter()
            .map(|d| {
                Ok(DependentRow {
                    name: d.name.clone(),
                    ssn: Ssn::canonical(&d.ssn)?,
                    relationship: d.relationship.clone(),
                })
            })
            .collect::<Result<Vec<_>, SsnError>>()?;

        Ok(Self {
            name_line,
            taxpayer,
            spouse,
            address_street: ri.header.address_street.clone(),
            address_city: ri.header.address_city.clone(),
            address_state: ri.header.address_state.clone(),
            address_zip: ri.header.address_zip.clone(),
            aged_blind: AgedBlindBoxes::for_return(ri, year),
            // `== Some(true)`: an UNANSWERED flag already refused upstream (`DependentStatusUnanswered`),
            // so it never reaches a printed form. Collapsing it here is a projection, not a guess (D-8).
            claimed_as_dependent_taxpayer: ri.header.can_be_claimed_as_dependent_taxpayer
                == Some(true),
            claimed_as_dependent_spouse: ri.header.can_be_claimed_as_dependent_spouse == Some(true),
            // Only meaningful on MFS (§63(c)(6)); `None` on MFS already refuses upstream
            // (`MfsSpouseItemizeUnknown`), so an unanswered flag never reaches a filed return.
            mfs_spouse_itemizes: ri.filing_status == FilingStatus::Mfs
                && ri.mfs_spouse_itemizes == Some(true),
            presidential_fund_taxpayer: ri.header.presidential_fund_taxpayer,
            presidential_fund_spouse: ri.header.presidential_fund_spouse,
            ip_pin: ri
                .header
                .ip_pin
                .as_deref()
                .map(IpPin::canonical)
                .transpose()?,
            dependents,
            proprietor,
        })
    }
}

// ── The assembled packet ────────────────────────────────────────────────────────────────────────

/// Every printed form of one filed return, composed in dependency order.
///
/// An `Option` member is a form that is **not filed** — and that is as load-bearing as the `Some`s: a
/// blank Schedule C stapled to a return with no business is a wrong return. `fill_full_return`
/// destructures this struct with no `..`, so a member added here without a filler is a compile error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedReturn {
    pub header: ReturnHeader,
    pub filing_status: FilingStatus,
    /// Every printed form of the return. Split from the identity on purpose: the FORM CHAINS are
    /// infallible and PII-free, so the REPORT can render exactly what the PDF will print even for a
    /// household that has entered no identity yet. Only the filable ARTIFACT needs a name and an SSN,
    /// and only it fails closed without them.
    pub forms: PrintedForms,
}

/// Every printed form of one return — the figures, with no identity attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedForms {
    pub f1040: Form1040Lines,
    pub sch_1: Option<Schedule1Lines>,
    pub sch_2: Option<Schedule2Lines>,
    pub sch_3: Option<Schedule3Lines>,
    pub sch_a: Option<ScheduleALines>,
    pub sch_b: Option<ScheduleBLines>,
    pub sch_c: Option<ScheduleCLines>,
    /// Schedule D always files on a full return (the crypto engine's whole point), so it is not optional.
    pub sch_d: ScheduleDLines,
    /// Form 8949 — the transaction detail Schedule D lines 3 and 10 CITE as their source ("Totals for
    /// all transactions reported on Form(s) 8949 with Box C/F checked" pre-2025; "with Box C or Box I
    /// checked" / "Box F or Box L checked" on the 2025 digital-asset revision). `None` when the year
    /// has no disposals: a carryover/distribution-only Schedule D files with lines 3/10 blank and no 8949.
    pub f8949: Option<Printed8949>,
    /// Schedule SE — the form Schedule 2 line 4 CITES ("Self-employment tax. **Attach Schedule SE**").
    /// `None` below the §6017 $400 floor, where no SE tax is owed and none is filed.
    pub sch_se: Option<ScheduleSeLines>,
    /// Always BUILT (Schedule 2 and the 1040 read its printed lines), but filed only when
    /// [`Form8959Lines::must_file`] — the chain and the filing decision are different questions.
    pub f8959: Form8959Lines,
    pub f8960: Option<Form8960Lines>,
    pub f8995: Option<Form8995Lines>,
    /// Form 8283 — REQUIRED when the return itemizes and its printed noncash gifts exceed $500 (the
    /// threshold is printed on Schedule A line 12 itself: "You must attach Form 8283 if over $500").
    ///
    /// Unlike Schedule D ← 8949, Schedule A L12 does NOT re-derive from these rows: L12 merely REQUIRES
    /// the attachment, and the §170(b) ceilings legitimately make it smaller than the sum of the 8283's
    /// per-donation amounts (SPEC §3.1, the citation-composition rule).
    pub f8283: Option<Printed8283Rows>,
    /// Form 8275 (Disclosure Statement) — Approach-B Task 16. `Some` iff a promoted-basis Form 8949
    /// DISPOSAL leg files in `year` ([`crate::tax::form8275::disclosure_8275`]'s own scoping: a
    /// promoted REMOVAL-only year files documented-only and takes no estimated position to disclose,
    /// BG-D11). Reg §1.6662-4(f) makes disclosure adequate only on a COMPLETED Form 8275, which is why
    /// an incomplete Part II gates the export (`cmd::admin::promote_export_gate`) rather than filing a
    /// silently-blank one.
    pub f8275: Option<Printed8275>,
}

/// ★ **The single composition site.** Build every printed chain from one `AbsoluteReturn`, in dependency
/// order: the upstream chains are ARGUMENTS to the downstream ones, which is precisely what makes the
/// filed packet tie out — Schedule 2 line 11 is Form 8959's *printed* line 18, and 1040 line 23 is
/// Schedule 2's *printed* line 21, never a re-rounding of the exact figure behind either.
///
/// Every input the chains need comes from `ar` (including [`crate::tax::return_1040::PrintedInputs`],
/// captured at derivation): nothing here re-derives a sum. A second summation is exactly how a filed
/// form comes to disagree with the tax the report computed from it.
pub fn assemble_printed_return(
    ri: &ReturnInputs,
    state: &LedgerState,
    donation_details: &BTreeMap<EventId, DonationDetails>,
    ar: &AbsoluteReturn,
    table: &TaxTable,
    year: i32,
    events: &[LedgerEvent],
) -> Result<PrintedReturn, HeaderError> {
    Ok(PrintedReturn {
        header: ReturnHeader::build(ri, year)?,
        filing_status: ri.filing_status,
        forms: assemble_printed_forms(ri, state, donation_details, ar, table, year, events),
    })
}

/// The printed form chains, with **no identity** — infallible, and PII-free.
///
/// The report renders THESE, so the terminal shows exactly the figures the filed PDF will carry (SPEC
/// §3.1: whole dollars, cross-footing). A report in exact cents beside a whole-dollar PDF would give the
/// filer two authoritative answers to "what do I owe", and "amount you owe" is not an analytical figure —
/// it is an instruction to write a check (ARCH-P6 Q3).
pub fn assemble_printed_forms(
    ri: &ReturnInputs,
    state: &LedgerState,
    donation_details: &BTreeMap<EventId, DonationDetails>,
    ar: &AbsoluteReturn,
    table: &TaxTable,
    year: i32,
    events: &[LedgerEvent],
) -> PrintedForms {
    let status = ri.filing_status;
    let pi = &ar.printed_inputs;

    // The 8949 is built FIRST: Schedule D's lines 3 and 10 are its printed column totals, so the
    // detail form is upstream of the schedule that summarizes it.
    let f8949 = form_8949_printed(&crate::forms::form_8949(state, year));

    // Attachments first — each downstream chain takes the printed lines of the ones above it.
    let f8959 = form_8959_lines(
        status,
        pi.medicare_wages,
        pi.medicare_withheld,
        ar.se.as_ref(),
    );
    let f8960 = form_8960_lines(
        status,
        ar.taxable_interest,
        ar.ordinary_dividends,
        ar.capital_gain,
        pi.crypto_lending_interest,
        ar.agi,
    );
    let f8995 = form_8995_lines(
        // Row 1i(a): the trade or business the §199A deduction is claimed for. Line 2's own text says
        // "Combine lines 1i through 1v, column (c)" — a total over an empty column names no business.
        &pi.schedule_c_header.business_description,
        pi.business_qbi,
        pi.reit_dividends,
        pi.reit_ptp_carryforward_in,
        pi.ti_before_qbi,
        pi.qbi_net_capital_gain,
    );

    let sch_b = schedule_b_lines(ri);
    let sch_c = schedule_c_lines(ar);
    let sch_d = schedule_d_lines(ar, f8949.as_ref());
    let sch_1 = schedule_1_lines(ar);

    // ★ The income block FIRST: Schedule A line 2 cites the 1040's printed line 11, so L11 must exist
    // before Schedule A does. No cycle — L11 depends on Schedules B/1/D, never on Schedule A (L12).
    let income = form_1040_income_lines(ar, sch_b.as_ref(), sch_1.as_ref(), &sch_d);
    let sch_a = schedule_a_lines(ar, income.line11);
    // Schedule SE is upstream of Schedule 2: L4 IS its printed line 12.
    let sch_se = sch_c.as_ref().and_then(|c| schedule_se_lines(ar, c));
    let sch_2 = schedule_2_lines(sch_se.as_ref(), &f8959, f8960.as_ref());
    let sch_3 = schedule_3_lines(ar);

    // Form 8283 files only when the return ITEMIZES and its printed noncash gifts clear the $500
    // threshold printed on Schedule A line 12 — a standard-deduction year with donations files none.
    let f8283 = sch_a
        .as_ref()
        .filter(|a| a.line12 > FORM_8283_THRESHOLD)
        .and_then(|_| form_8283_printed(&crate::forms::form_8283(state, year, donation_details)));

    // Form 8275 (Task 16) — `Some` iff a promoted-basis DISPOSAL leg files in `year`; the printed
    // (whole-dollar-rounded Part I) content of `crate::tax::form8275::disclosure_8275`, whose own
    // scoping already omits a promoted REMOVAL-only year (BG-D11).
    let f8275 =
        crate::tax::form8275::disclosure_8275(events, state, year).map(|d| printed_8275(&d));

    let f1040 = form_1040_lines(
        ar,
        &income,
        sch_a.as_ref(),
        sch_2.as_ref(),
        sch_3.as_ref(),
        &f8959,
        f8995.as_ref(),
        table,
        status,
        ri.payments.other_withholding,
        ri.payments.estimated_tax_payments,
        pi.digital_asset_activity,
    );

    PrintedForms {
        f1040,
        sch_1,
        sch_2,
        sch_3,
        sch_a,
        sch_b,
        sch_c,
        sch_d,
        f8949,
        sch_se,
        f8959,
        f8960,
        f8995,
        f8283,
        f8275,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_1040::assemble_absolute;
    use crate::tax::return_inputs::{Dependent, HouseholdHeader, ScheduleCInputs, W2};
    use crate::tax::testonly::{
        kitchen_sink_household, ty2024_params, ty2024_table, w2_only_household,
    };
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn person(first: &str, last: &str, ssn: &str) -> Person {
        Person {
            first_name: first.into(),
            last_name: last.into(),
            ssn: ssn.into(),
            ..Default::default()
        }
    }

    // ── Ssn::canonical (p1-ssn-normalization) ───────────────────────────────────────────────────────

    /// An SSN is captured AS ENTERED, so the canonical form must absorb the ways a human types one:
    /// hyphenated, bare, or spaced. All three are the same nine digits, and the hyphenated rendering is
    /// what an 11-character comb cell on the form expects.
    #[test]
    fn ssn_canonical_accepts_hyphenated_bare_and_spaced() {
        for raw in ["123-45-6789", "123456789", " 123 45 6789 "] {
            let ssn = Ssn::canonical(raw).expect("a nine-digit SSN is canonicalizable");
            assert_eq!(ssn.digits(), "123456789", "raw = {raw:?}");
            assert_eq!(ssn.hyphenated(), "123-45-6789", "raw = {raw:?}");
        }
    }

    /// Anything that is not exactly nine digits is NOT an SSN. It fails here, at compute time (§3.4: an
    /// unprintable SSN is an uncomputable line) — never as a silently truncated or padded form cell.
    #[test]
    fn ssn_canonical_rejects_anything_that_is_not_nine_digits() {
        for raw in ["", "12345678", "1234567890", "123-45-678X", "not an ssn"] {
            assert!(
                Ssn::canonical(raw).is_err(),
                "{raw:?} must not canonicalize into an SSN"
            );
        }
    }

    // ── ReturnHeader ────────────────────────────────────────────────────────────────────────────────

    /// "Name(s) shown on return" is the JOINT name line on a joint return — both spouses, one string.
    #[test]
    fn header_name_line_is_joint_on_a_joint_return() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123-45-6789");
        ri.header.spouse = Some(person("Jane", "Doe", "987-65-4321"));

        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        assert_eq!(h.name_line, "John Doe & Jane Doe");
        assert_eq!(h.taxpayer.ssn.hyphenated(), "123-45-6789");
        assert_eq!(
            h.spouse.as_ref().map(|s| s.ssn.hyphenated()),
            Some("987-65-4321".to_string())
        );
    }

    /// On every NON-joint status the name line is the taxpayer alone — including MFS, where a spouse
    /// exists on the return but is NOT part of the name line (their name has its own 1040 cell).
    #[test]
    fn header_name_line_is_the_taxpayer_alone_when_not_joint() {
        for status in [FilingStatus::Single, FilingStatus::Mfs, FilingStatus::HoH] {
            let mut ri = ReturnInputs {
                filing_status: status,
                ..Default::default()
            };
            ri.header.taxpayer = person("John", "Doe", "123456789");
            ri.header.spouse = Some(person("Jane", "Doe", "987654321"));

            let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
            assert_eq!(h.name_line, "John Doe", "status = {status:?}");
        }
    }

    /// Schedule C's header is "Name of **proprietor**", not the return's name line: a spouse-owned
    /// business files under the SPOUSE's name and SSN even on a joint return. A shared writer that put
    /// the joint name line here would file a Schedule C for the wrong person.
    #[test]
    fn schedule_c_proprietor_is_the_business_owner_not_the_joint_name_line() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Spouse,
                ..Default::default()
            }),
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123456789");
        ri.header.spouse = Some(person("Jane", "Roe", "987654321"));

        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        let p = h
            .proprietor
            .as_ref()
            .expect("a Schedule C has a proprietor");
        assert_eq!(p.full_name(), "Jane Roe");
        assert_eq!(p.ssn.hyphenated(), "987-65-4321");
        // …and the return's own name line is still the joint one.
        assert_eq!(h.name_line, "John Doe & Jane Roe");
    }

    /// ★ The GATING tie-out (`p6-aged-blind-checkboxes-missing`). The §63(f) aged/blind additions are
    /// folded into printed 1040 **L12**, and the IRS validates that nonstandard standard deduction by
    /// COUNTING the checked boxes. So the header's box count must equal the count core actually used to
    /// build L12 — if the two can drift, a filed return claims an amount its own checkboxes do not
    /// support, which is exactly the defect this KAT exists to make impossible.
    #[test]
    fn aged_blind_box_count_matches_the_standard_deduction_core_actually_computed() {
        let p = ty2024_params();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![W2 {
                box1_wages: dec!(90000),
                ..Default::default()
            }],
            ..Default::default()
        };
        // Taxpayer is 65+ AND blind (2 boxes); spouse is blind only (1 box) ⇒ 3 boxes.
        ri.header.taxpayer = Person {
            date_of_birth: Some(date!(1955 - 03 - 02)),
            blind: Some(true),
            ..person("John", "Doe", "123456789")
        };
        ri.header.spouse = Some(Person {
            blind: Some(true),
            ..person("Jane", "Doe", "987654321")
        });

        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        assert!(h.aged_blind.taxpayer_aged);
        assert!(h.aged_blind.taxpayer_blind);
        assert!(!h.aged_blind.spouse_aged);
        assert!(h.aged_blind.spouse_blind);
        assert_eq!(h.aged_blind.count(), 3);

        let ar = assemble_absolute(&ri, &Default::default(), &p, &ty2024_table(), 2024);
        // MFJ ⇒ the married per-box rate. base + count × per_box IS the standard deduction core used.
        let expected = p.std_deduction_for(FilingStatus::Mfj)
            + p.std_aged_blind_married * rust_decimal::Decimal::from(h.aged_blind.count());
        assert_eq!(ar.standard_deduction, expected);
    }

    /// A spouse's boxes count ONLY on a joint return — the same rule core uses for L12. On MFS the
    /// spouse's blindness is not the taxpayer's checkbox.
    #[test]
    fn aged_blind_ignores_the_spouse_unless_the_return_is_joint() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfs,
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123456789");
        ri.header.spouse = Some(Person {
            blind: Some(true),
            date_of_birth: Some(date!(1950 - 01 - 01)),
            ..person("Jane", "Doe", "987654321")
        });

        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        assert!(!h.aged_blind.spouse_aged);
        assert!(!h.aged_blind.spouse_blind);
        assert_eq!(h.aged_blind.count(), 0);
    }

    /// The dependents rows carry through to the header (the CTC/ODC boxes stay deliberately unchecked —
    /// consistent with L19 = 0 and the `CtcOdcOmitted` advisory), and their SSNs canonicalize too.
    #[test]
    fn dependents_carry_through_with_canonical_ssns() {
        let ri = ReturnInputs {
            // Single (not MFJ): this test is about DEPENDENT carry-through, and an MFJ return with no
            // spouse `Person` now correctly refuses at build (`MfjWithoutSpouse`, r3 M-6).
            filing_status: FilingStatus::Single,
            header: HouseholdHeader {
                taxpayer: person("John", "Doe", "123456789"),
                dependents: vec![Dependent {
                    name: "Sam Doe".into(),
                    ssn: "111223333".into(),
                    relationship: "Son".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };

        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        assert_eq!(h.dependents.len(), 1);
        assert_eq!(h.dependents[0].name, "Sam Doe");
        assert_eq!(h.dependents[0].ssn.hyphenated(), "111-22-3333");
        assert_eq!(h.dependents[0].relationship, "Son");
    }

    /// A bad SSN anywhere in the household — taxpayer, spouse, or a dependent — fails the whole header.
    /// Fail-closed: a return cannot be built around an SSN that cannot be printed.
    #[test]
    fn a_bad_ssn_anywhere_in_the_household_fails_the_header() {
        let base = |ssn_t: &str, ssn_s: &str, ssn_d: &str| ReturnInputs {
            filing_status: FilingStatus::Mfj,
            header: HouseholdHeader {
                taxpayer: person("John", "Doe", ssn_t),
                spouse: Some(person("Jane", "Doe", ssn_s)),
                dependents: vec![Dependent {
                    name: "Sam Doe".into(),
                    ssn: ssn_d.into(),
                    relationship: "Son".into(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let good = "123456789";
        // `answered(..)` so the SSN is the ONLY defect under test (build now also refuses unanswered
        // declarations — P8a I3, tested below). A malformed SSN still refuses; the good case builds.
        let a = crate::tax::testonly::answered;
        assert!(ReturnHeader::build(&a(base(good, good, good)), 2024).is_ok());
        assert!(ReturnHeader::build(&a(base("12345", good, good)), 2024).is_err());
        assert!(ReturnHeader::build(&a(base(good, "nope", good)), 2024).is_err());
        assert!(ReturnHeader::build(&a(base(good, good, "1234567890")), 2024).is_err());
    }

    /// ★ P8a I3 — `ReturnHeader::build` is the fail-closed PRINT boundary: even a caller that skips
    /// `screen_inputs` cannot print an unaffirmed box. A live declaration left `None` refuses here too.
    #[test]
    fn build_refuses_an_unanswered_live_declaration() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123456789"); // a printable identity
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        assert!(
            ReturnHeader::build(&ri, 2024).is_ok(),
            "fully answered ⇒ builds"
        );

        // Blank ONE declaration (the HSA activity) — build must refuse, naming it.
        ri.sch1.hsa_activity = None;
        assert_eq!(
            ReturnHeader::build(&ri, 2024),
            Err(HeaderError::Unanswered(QuestionId::HsaActivity)),
            "an unanswered live declaration must not reach a printed form (P8a I3)"
        );
    }

    /// ★ r3 M-6 — a joint return with no spouse `Person` cannot fill the joint name line or the spouse SSN
    /// cell. Distinct from the unanswered check: the spouse DECLARATION is answered, the IDENTITY is absent.
    #[test]
    fn build_refuses_mfj_with_no_spouse_identity() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123456789");
        ri.header.spouse = None; // no spouse identity on a joint return
        crate::tax::testonly::answer_all_live_declarations(&mut ri); // every declaration answered
        assert_eq!(
            ReturnHeader::build(&ri, 2024),
            Err(HeaderError::MfjWithoutSpouse),
            "MFJ with no spouse Person cannot print the joint header (r3 M-6)"
        );
    }

    /// ★ The OTHER two checkbox-consistency gaps, of the same class as the aged/blind boxes and found
    /// the same way (dumping the 1040's fields and correlating them against the printed form).
    ///
    /// Core's L12 already CONSUMES both flags: `can_be_claimed_as_dependent_taxpayer` swaps the basic
    /// standard deduction for the §63(c)(5) dependent floor, and on MFS `mfs_spouse_itemizes` couples
    /// the spouses' §63(c)(6) election. Each has a 1040 checkbox — "Someone can claim: You as a
    /// dependent" and "Spouse itemizes on a separate return" — and a return that claims the arithmetic
    /// without checking the box is a form contradicting itself, exactly like a nonstandard standard
    /// deduction with zero aged/blind boxes ticked. So the header carries them, and the filler prints
    /// them.
    #[test]
    fn the_header_carries_the_dependent_claim_and_mfs_itemize_flags_that_l12_depends_on() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123456789");
        ri.header.can_be_claimed_as_dependent_taxpayer = Some(true);

        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        assert!(
            h.claimed_as_dependent_taxpayer,
            "the §63(c)(5) floor is claimed ⇒ the box must print"
        );
        assert!(!h.mfs_spouse_itemizes);

        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfs,
            mfs_spouse_itemizes: Some(true),
            ..Default::default()
        };
        ri.header.taxpayer = person("John", "Doe", "123456789");
        let h = ReturnHeader::build(&crate::tax::testonly::answered(ri.clone()), 2024).unwrap();
        assert!(
            h.mfs_spouse_itemizes,
            "§63(c)(6) coupling is in force ⇒ the box must print"
        );
    }

    // ── assemble_printed_return — the ONE composition site ───────────────────────────────────────────

    /// ★ The packet ties out to its own attachments. Every one of these equalities is a figure the 1040
    /// CARRIES from a schedule: if the packet ever re-derives instead of transcribing, one of these
    /// breaks — which is precisely how a filed return comes to disagree with the forms stapled behind it.
    #[test]
    fn the_assembled_packet_ties_the_1040_to_its_attachments() {
        let (ri, state) = kitchen_sink_household();
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
        .unwrap();

        let sch_1 = pr
            .forms
            .sch_1
            .expect("the kitchen sink has Schedule 1 income");
        let sch_2 = pr.forms.sch_2.expect("…and SE tax ⇒ Schedule 2");
        let sch_a = pr.forms.sch_a.expect("…and itemized deductions");
        let sch_b = pr
            .forms
            .sch_b
            .expect("…and > $1,500 of interest ⇒ Schedule B");
        let sch_c = pr.forms.sch_c.expect("…and business mining ⇒ Schedule C");
        let f8995 = pr.forms.f8995.expect("…and REIT dividends ⇒ Form 8995");

        assert_eq!(
            pr.forms.f1040.line2b, sch_b.line4,
            "1040 2b ← Schedule B line 4"
        );
        assert_eq!(
            pr.forms.f1040.line3b, sch_b.line6,
            "1040 3b ← Schedule B line 6"
        );
        assert_eq!(
            pr.forms.f1040.line8, sch_1.line10,
            "1040 8 ← Schedule 1 line 10"
        );
        assert_eq!(
            pr.forms.f1040.line10, sch_1.line26,
            "1040 10 ← Schedule 1 line 26"
        );
        assert_eq!(
            pr.forms.f1040.line12, sch_a.line17,
            "1040 12 ← Schedule A line 17"
        );
        assert_eq!(
            pr.forms.f1040.line13, f8995.line15,
            "1040 13 ← Form 8995 line 15"
        );
        assert_eq!(
            pr.forms.f1040.line23, sch_2.line21,
            "1040 23 ← Schedule 2 line 21"
        );
        assert_eq!(
            sch_2.line11, pr.forms.f8959.line18,
            "Sch 2 11 ← Form 8959 line 18"
        );

        // ★ The ATTACHMENT tie-outs (ARCH-P6.3a). Every one of these is a citation printed on the form
        // itself, so a packet that fails any of them is a form disagreeing with the paper behind it.
        let sch_se = pr.forms.sch_se.expect("…and business mining ⇒ Schedule SE");
        let f8949 = pr
            .forms
            .f8949
            .as_ref()
            .expect("…and a disposal ⇒ Form 8949");
        assert_eq!(
            sch_2.line4, sch_se.line12,
            "Sch 2 L4 ← Schedule SE's printed L12"
        );
        assert_eq!(
            sch_1.line15, sch_se.line13,
            "Sch 1 L15 ← Schedule SE's printed L13"
        );
        assert_eq!(
            pr.forms.f8959.line8, sch_se.line6,
            "8959 L8 ← Schedule SE Part I L6"
        );
        assert_eq!(
            sch_se.line2, sch_c.line31,
            "SE L2 ← Schedule C's printed L31"
        );
        assert_eq!(
            pr.forms.sch_d.line10_d, f8949.lt_totals.proceeds_d,
            "Sch D L10(d) ← the 8949's printed long-term column total"
        );
        assert_eq!(
            pr.forms.sch_d.line10_h,
            pr.forms.sch_d.line10_d - pr.forms.sch_d.line10_e,
            "…and Schedule D Part II cross-foots on its own printed cells"
        );

        // ★ The extension payment reaches the filed page (ARCH-P6.3a D1). Without Schedule 3 line 10 the
        // return would demand a payment the filer had ALREADY made: L31 falls ⇒ L37 "amount you owe"
        // rises by exactly that amount.
        let sch_3 = pr
            .forms
            .sch_3
            .expect("…and an extension payment + FTC ⇒ Schedule 3");
        assert_eq!(
            pr.forms.f1040.line31, sch_3.line15,
            "1040 31 ← Schedule 3 line 15"
        );
        assert_eq!(
            sch_3.line10,
            dec!(500),
            "the kitchen sink paid $500 with its extension"
        );
        assert!(
            sch_3.line15 >= sch_3.line10,
            "L15 = 'add 9 through 12 and 14' — it can never DROP the extension payment"
        );
    }

    /// A form that is not required is not in the packet — a plain W-2 household files a 1040 and nothing
    /// else. (`fill_full_return` emits exactly the `Some` members, so an over-eager `Some` here would
    /// staple a blank Schedule C to a return that has no business.)
    #[test]
    fn the_packet_omits_every_form_that_is_not_required() {
        let (ri, state) = w2_only_household();
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
        .unwrap();

        assert!(
            pr.forms.sch_1.is_none(),
            "no additional income or adjustments"
        );
        assert!(
            pr.forms.sch_2.is_none(),
            "no SE / Additional Medicare / NIIT"
        );
        assert!(pr.forms.sch_3.is_none(), "no credits");
        assert!(pr.forms.sch_a.is_none(), "standard deduction");
        assert!(
            pr.forms.sch_b.is_none(),
            "interest under the $1,500 threshold"
        );
        assert!(pr.forms.sch_c.is_none(), "no business");
        assert!(pr.forms.f8960.is_none(), "no NIIT");
        assert!(pr.forms.f8995.is_none(), "no QBI");
        assert!(
            !pr.forms.f8959.must_file(),
            "no Additional Medicare Tax, none withheld"
        );
    }

    /// The printed Form 8959 sees the SAME household Σ box-5 that the COMPUTED 8959 saw. The printed
    /// chain must never re-derive its own inputs: a second summation is exactly how the filed form comes
    /// to disagree with the tax the report computed from it.
    #[test]
    fn the_printed_8959_reads_the_same_box5_sum_the_computed_8959_used() {
        let (ri, state) = kitchen_sink_household();
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
        .unwrap();

        let box5_sum: crate::conventions::Usd = ri.w2s.iter().map(|w| w.box5_medicare_wages).sum();
        assert_eq!(ar.printed_inputs.medicare_wages, box5_sum);
        assert_eq!(
            pr.forms.f8959.line1,
            crate::conventions::round_dollar(box5_sum)
        );
    }
}
