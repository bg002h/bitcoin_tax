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
use crate::tax::dependent_gates::CreditColumn;
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

/// A dependent's row on the 1040.
///
/// ★★★ **T8 / R6 — the credit boxes ARE modelled now.** This doc used to say *"The CTC/ODC credit
/// boxes are deliberately NOT modeled here: v1 omits the credit entirely (L19 = 0, with the
/// `CtcOdcOmitted` advisory), and a checked credit box beside a zero credit is a form contradicting
/// itself."* Two things changed and both are the instruction's, not ours:
///
/// 1. **Row (7) is a COMPUTED box, not a claimed amount.** The flowchart *Who Qualifies as Your
///    Dependent* decides it (`i1040gi--2025.txt:1456-1812`), and [`crate::tax::dependent_gates`]
///    walks that flowchart. Checking it is transcription.
/// 2. **Line 19 is still a forgo, and that is not a contradiction.** The credit itself is claimed on
///    Schedule 8812, which btctax does not file; the box says the dependent QUALIFIES, the line says
///    what the schedule computed. `ctc_odc_line19` and `Advisory::CtcOdcOmitted` are unchanged, so
///    the filer is still told what they forgo — see `advisories.rs`'s own note.
///
/// TY2024's grid has no rows (5)/(6) and its map declares no `[dependents_grid]`, so nothing here
/// reaches its printed page: [`Self::grid`] is written only through the TY2025+ grid cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependentRow {
    pub name: String,
    pub ssn: Ssn,
    pub relationship: String,
    /// Rows (5), (6) and (7) of the TY2025+ Dependents grid.
    pub grid: DependentGridRow,
}

/// ★★★ **Rows (5), (6) and (7) of the TY2025+ Dependents grid, for ONE dependent.**
///
/// The three printed rows the TY2024 form does not have (`f1040--2025.txt:44-54`): *"(5) Check if
/// lived with you more than half of 2025 — (a) Yes / (b) And in the U.S."*, *"(6) Check if — Full-time
/// student / Permanently and totally disabled"*, and *"(7) Credits — Child tax credit / Credit for
/// other dependents"*.
///
/// ★★ **`bool`, not `Option<bool>`, and the projection is the point.** The form prints a CHECKBOX:
/// the only thing it can say is *checked*. `Some(false)` and `None` are different facts about the
/// interview and the SAME mark on the page — an empty box — so this is the projection to the filed
/// page, taken once, in one place. The distinction that matters upstream is kept upstream: a live
/// gate left `None` refuses through `screen_dependent_gates` before any packet is built, and a gate
/// the walk never demanded (row (5)(b) under a *No* on (5)(a)) is lawfully blank.
///
/// ★★ **That last sentence is held by construction, not by convention (T8 seam review I-1).**
/// [`ReturnHeader::build`] projects every one of these bools as
/// `walk.demands(gate) && leaf == Some(true)`, so a leaf left over from an answer the filer has since
/// retracted cannot reach the page: the walk stops demanding the gate, and the box goes blank. Before
/// that fix the projection read the raw leaf and printed (5)(b) checked under an unchecked (5)(a) —
/// a mark the filer could neither see (the form seam's `get` returns `None` for an undemanded gate)
/// nor clear (`clear` returns `SetError::NoSuchRow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DependentGridRow {
    /// Row **(5)(a)** — *"Check if lived with you more than half of 2025 … (a) Yes"*.
    pub lived_with_you_over_half_year: bool,
    /// Row **(5)(b)** — *"(b) And in the U.S."*.
    pub lived_with_you_in_us: bool,
    /// Row **(6)** — *"Full-time student"*.
    pub full_time_student: bool,
    /// Row **(6)** — *"Permanently and totally disabled"*.
    pub permanently_and_totally_disabled: bool,
    /// Row **(7)** — computed by the flowchart, never asked.
    pub credit: CreditColumn,
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
    /// The §63(f) boxes for a return.
    ///
    /// ★★★ **§G-9a ADJUDICATED 2026-07-30 — the BLINDNESS box has NO death interaction, and the
    /// asymmetry with the aged box is deliberate.** i1040gi (2024) states the carve-out for exactly one
    /// box, naming it by its printed label:
    ///
    /// > *"**Death of spouse in 2024.** If your spouse was born before January 2, 1960, but died in
    /// > 2024 before reaching age 65, don't check the box that says **'Spouse was born before January
    /// > 2, 1960.'**"*
    ///
    /// There is no matching sentence for blindness, and the MECHANISM says why there could not be. The
    /// age test is a DURATION test with a gap the year can straddle: a person "is considered to reach
    /// age 65 on the day before the person's 65th birthday", so a birthday falling after their death
    /// is an event that never happened — which is precisely what the carve-out resolves. Blindness is a
    /// POINT-IN-TIME status test, anchored in its own words: *"blind at the end of 2024"* / *"totally
    /// blind as of December 31, 2024"*. A decedent's tax year ends at death, so someone blind when
    /// they died was blind at the end of their year. **There is nothing to carve out.**
    /// The test `the_blind_box_has_no_death_carve_out_but_the_aged_box_does` pins the asymmetry, so a
    /// future "harmonisation" of the two boxes reds.
    ///
    /// ★ **Honest limit on that adjudication:** i1040gi routes a decedent's preparer to **Pub. 501**,
    /// which is NOT in `legal/primary-sources/` — and neither is **26 USC §63**. So this rests on
    /// rung 2 plus the mechanism, not on rung 3 or 4. It is recorded that way rather than as settled.
    ///
    /// ★★ **The spouse's boxes are decided by `questions::spouse_63f_boxes_count`, ONE predicate
    /// shared with the liveness of the questions that feed them.** i1040gi: *"If your filing status is
    /// married filing separately and your spouse was born before January 2, 1960, or was blind at the
    /// end of 2024, you can check the appropriate box(es) … if your spouse had no income, isn't filing
    /// a return, and can't be claimed as a dependent on another person's return."* All three are now
    /// captured, so MFS CAN claim them — and the gate fails closed: any unanswered or adverse
    /// condition forgoes, because forgoing costs a deduction the filer can recover by answering while
    /// granting one they are not entitled to understates a signed return. §G-20 is closed.
    ///
    /// ★ r3 I-1: the four §63(f) ADVISORIES ask this same predicate. They were left on `== Mfj` when
    /// the deduction moved, and told a filer whose boxes were already claimed to claim them again.
    pub fn for_return(ri: &ReturnInputs, year: i32) -> Self {
        let t = &ri.header.taxpayer;
        // ★★★ §G-20 — a spouse's boxes count on MFJ, **or on MFS when all THREE of i1040gi's
        // conditions are affirmatively true**. This is the only place on the branch where an answer
        // can only ever REDUCE tax, so it is written to fail closed: every condition must be an
        // explicit answer in the claiming direction, and ANY unanswered one forgoes.
        //
        // > *"…married filing separately … you can check the appropriate box(es) … if your spouse had
        // > no income, isn't filing a return, and can't be claimed as a dependent on another person's
        // > return."*
        //
        // Forgoing costs the filer a deduction, which they can recover by answering. Granting one they
        // are not entitled to understates a signed return, which they cannot. The asymmetry decides
        // the default.
        // ★ ONE definition, in `questions::spouse_63f_boxes_count` — shared with the liveness of
        // `SpouseDiedDuringYear` / `DodSpouse`, so a box can never be counted for a spouse whose death
        // carve-out was never even asked.
        let joint_spouse = ri
            .header
            .spouse
            .as_ref()
            .filter(|_| crate::tax::questions::spouse_63f_boxes_count(ri));
        Self {
            taxpayer_aged: is_aged(
                t.date_of_birth,
                ri.header.taxpayer_died_during_year,
                t.date_of_death,
                year,
            ),
            taxpayer_blind: t.blind == Some(true),
            spouse_aged: joint_spouse.is_some_and(|s| {
                is_aged(
                    s.date_of_birth,
                    ri.header.spouse_died_during_year,
                    s.date_of_death,
                    year,
                )
            }),
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

/// ★★★ **The 1040 header's SHARED ENTRY SPACE — whose name goes in the one cell, decided ONCE.**
///
/// TY2024's form prints one sentence and one widget for three filing statuses: *"If you checked the
/// MFS box, enter the name of your spouse. If you checked the HOH or QSS box, enter the child's name
/// if the qualifying person is a child but not your dependent"* (`f1040--2024.txt:28-29`, mapped as
/// `mfs_spouse_name = "…f1_18[0]"`). Two answers, one cell — so which status claims it has to be one
/// decision that the interview seam and the emitter both read, or they drift. That is exactly how T8
/// seam review **I-3** happened: the seam collected the child's name on `HoH`, and the emitter's only
/// write to the cell was reachable on `Mfs` alone.
///
/// ★ **This `impl` block is in `packet.rs` and not beside the enum on purpose.** `tax/types.rs` is
///   content-pinned by [`crate::tax::frozen_guard`] (SPEC_full_return §2, additive-only): adding a
///   method there would trip `frozen_engine_files_are_unchanged` and require the documented exception
///   process — a separately reviewed pin bump — for a predicate that belongs to the printed header
///   anyway. An inherent impl in another module of the same crate is the same API at every call site.
impl FilingStatus {
    /// The CHILD's-name half of the shared entry space — **HoH and QSS**, the two statuses the form's
    /// own sentence names for it.
    #[must_use]
    pub fn wants_qualifying_child_name(self) -> bool {
        matches!(self, FilingStatus::HoH | FilingStatus::Qss)
    }

    /// The SPOUSE's-name half of the same cell — *"If you checked the MFS box, enter the name of your
    /// spouse"* (`f1040--2024.txt:28`).
    #[must_use]
    pub fn wants_spouse_name_in_the_shared_entry_space(self) -> bool {
        matches!(self, FilingStatus::Mfs)
    }

    /// Every filing status, for a check that must be TOTAL over the enum rather than over a hand-list.
    ///
    /// ★ It is itself a written list, so it is held the way `DependentGate::ALL` is
    ///   (`provenance.rs`'s `every_dependent_gate_is_in_all`): the exhaustive, `_`-free `match` in
    ///   [`shared_entry_space_tests::every_filing_status_is_in_all`] makes a new variant a COMPILE
    ///   error, and the index equality is what catches one that was named but left out of `ALL`.
    pub const ALL: [FilingStatus; 5] = [
        FilingStatus::Single,
        FilingStatus::Mfj,
        FilingStatus::Mfs,
        FilingStatus::HoH,
        FilingStatus::Qss,
    ];
}

#[cfg(test)]
mod shared_entry_space_tests {
    use super::FilingStatus;

    /// ★★ Every filing status is listed in `ALL` — the exhaustive `match` makes a new variant a
    /// compile error here, and the index equality catches one added to the enum and forgotten in
    /// `ALL`. Same shape as `provenance.rs`'s `every_dependent_gate_is_in_all`, for the same reason:
    /// the test below loops `ALL`, so a status missing from it would simply never be checked.
    #[test]
    fn every_filing_status_is_in_all() {
        for (i, st) in FilingStatus::ALL.into_iter().enumerate() {
            let idx = match st {
                FilingStatus::Single => 0,
                FilingStatus::Mfj => 1,
                FilingStatus::Mfs => 2,
                FilingStatus::HoH => 3,
                FilingStatus::Qss => 4,
            };
            assert_eq!(idx, i, "FilingStatus::ALL is out of order / missing {st:?}");
        }
        assert_eq!(FilingStatus::ALL.len(), 5, "§1(a)-(d) and §2(a)/(b)");
    }

    /// ★★★ **ONE cell, at most ONE claimant — asserted, not assumed (T8 seam review I-3).** The
    /// emitter writes the spouse's name into `mfs_spouse_name` on MFS and the qualifying child's name
    /// into the SAME cell on HoH/QSS. If any status could answer both, one filer's testimony would
    /// overwrite another's on the printed page. Total over [`FilingStatus::ALL`], so a sixth status
    /// cannot join without being given an answer here.
    #[test]
    fn the_shared_entry_space_has_exactly_one_claimant_per_status() {
        for st in FilingStatus::ALL {
            assert!(
                !(st.wants_qualifying_child_name()
                    && st.wants_spouse_name_in_the_shared_entry_space()),
                "{st:?} claims the shared 1040 entry space twice"
            );
        }
        // …and the form's own sentence names three statuses, so exactly three claim it.
        let claimants: Vec<FilingStatus> = FilingStatus::ALL
            .into_iter()
            .filter(|s| {
                s.wants_qualifying_child_name() || s.wants_spouse_name_in_the_shared_entry_space()
            })
            .collect();
        assert_eq!(
            claimants,
            vec![FilingStatus::Mfs, FilingStatus::HoH, FilingStatus::Qss],
            "\"If you checked the MFS box … If you checked the HOH or QSS box\" — three statuses"
        );
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
    /// ★★★ **The 1040 header's shared entry space, HoH/QSS half** — *"If you checked the HOH or QSS
    /// box, enter the child's name if the qualifying person is a child but not your dependent"*
    /// (`f1040--2024.txt:28-29`, the cell mapped as `mfs_spouse_name`). Empty when the filer left it
    /// blank, which is LAWFUL — *"If you don't enter the name, it will take us longer to process your
    /// return"* (`i1040gi--2025.txt:1206-1210`) — and empty on every other status, where the same
    /// cell belongs to MFS's spouse name.
    ///
    /// ★ T8 seam review **I-3**: this leaf was collected, classified, scrubbed, covered and helped by
    ///   T8, and had **no reader on any year** — the emitter's only write to that cell sat inside
    ///   `if let Some(sp) = &header.spouse` under `status == Mfs`, which a HoH or QSS return can
    ///   never reach. Carrying it here is what gives it one.
    pub qualifying_child_name: String,
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

        // ★★★ **T8 / R6 — rows (5), (6) and (7), computed HERE and nowhere else, from ONE walk.**
        //     The credit column and the row-(5)/(6) checkboxes are both read off the same
        //     `walk_dependent` the screens ran, so the printed boxes and the refusal that let the row
        //     through can never be two different readings of the flowchart.
        //
        // ★★★ **T8 seam review I-1 — a box is printed only if the walk DEMANDED its gate.** Reading
        //     the raw leaf alone printed row (5)(b) *checked* under an unchecked (5)(a): a filer who
        //     answered (5)(a) `Yes` + (5)(b) `Yes` and then flipped (5)(a) to `No` left a stale
        //     `Some(true)` behind, which nothing clears (`sections.rs`'s `clear` returns
        //     `NoSuchRow` for a gate the walk does not demand) and nothing screens — a sub-condition
        //     asserted on the filed page under a condition the return does not assert.
        //     `DependentWalk::demands` is this module's stated definition of liveness, so it is the
        //     thing to ask. **All four** row-(5)/(6) gates are gated on it even though only
        //     `LivedWithYouInUs` is conditional in today's Step 1 block (the other three are demanded
        //     unconditionally, `dependent_gates.rs`'s Step 1 loop): the guarantee is
        //     *not-demanded ⇒ blank*, and a gate that becomes conditional later must not silently
        //     re-open the hole.
        let dependents = ri
            .header
            .dependents
            .iter()
            .enumerate()
            .map(|(row, d)| {
                use crate::tax::provenance::DependentGate as G;
                let walk = crate::tax::dependent_gates::walk_dependent(ri, row);
                let checked =
                    |gate: G, leaf: Option<bool>| walk.demands(gate) && leaf == Some(true);
                Ok(DependentRow {
                    name: d.name.clone(),
                    ssn: Ssn::canonical(&d.ssn)?,
                    relationship: d.relationship.clone(),
                    grid: DependentGridRow {
                        lived_with_you_over_half_year: checked(
                            G::LivedWithYouOverHalfYear,
                            d.lived_with_you_over_half_year,
                        ),
                        lived_with_you_in_us: checked(G::LivedWithYouInUs, d.lived_with_you_in_us),
                        full_time_student: checked(G::FullTimeStudent, d.full_time_student),
                        permanently_and_totally_disabled: checked(
                            G::PermanentlyAndTotallyDisabled,
                            d.permanently_and_totally_disabled,
                        ),
                        credit: walk.verdict.credit_column(),
                    },
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
            // Carried for every status; the EMITTER decides which statuses print it (HoH and QSS —
            // `FilingStatus::wants_qualifying_child_name`), because the cell is shared with MFS.
            qualifying_child_name: ri.header.qualifying_child_name.clone(),
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
    /// **Form 8995-A Part IV** (§G-28/B1a) — the FULL §199A form, filed instead of [`Self::f8995`]
    /// above the §199A(e)(2) threshold, where i8995a's "Who Must File" retires the simplified one.
    ///
    /// ★★ The two are ALTERNATIVES: exactly one is `Some` on any return that claims §199A at all.
    /// Filing both would claim the deduction twice on paper.
    pub f8995a: Option<crate::tax::qbi_a::Form8995A>,
    /// **Form 6251** (§G-6) — `Some` exactly when i6251's *Who Must File* condition 1 holds:
    /// *"Form 6251, line 7, is greater than line 10."*
    ///
    /// ★★ That is the ATTACH test, not `amt > 0`: when line 7 exceeds line 10 the AMT foreign tax
    /// credit is figured, so the AMT can be $0 while the form is still required. btctax has computed
    /// this form for every return since v0.14.0 — what was missing was the ability to FILE it.
    pub f6251: Option<crate::tax::form6251::Form6251>,
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
    /// ★★★ **Form 8889 (T16)** — `Some` exactly when the §223 trigger declaration is affirmed
    /// ([`crate::tax::form8889::Form8889::must_file`]), which is the filer's own answer and never a
    /// threshold over the figures. An all-zero Part II and a $0 line 13 are legitimately blank parts
    /// of a form the IRS still requires; a figure-derived gate would drop the form for a filer whose
    /// contributions exactly equalled their employer's, leaving Schedule 1 line 13 with no
    /// attachment behind it.
    pub f8889: Option<crate::tax::form8889::Form8889>,
}

#[allow(clippy::too_many_arguments)] // the Form 1099-DA regime is the eighth (spec 1099-DA R1); a params struct is a later tidy
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
    regime: crate::forms::InformationReturnRegime,
) -> Result<PrintedReturn, HeaderError> {
    Ok(PrintedReturn {
        header: ReturnHeader::build(ri, year)?,
        filing_status: ri.filing_status,
        forms: assemble_printed_forms(ri, state, donation_details, ar, table, year, events, regime),
    })
}

#[allow(clippy::too_many_arguments)] // the Form 1099-DA regime is the eighth (spec 1099-DA R1); a params struct is a later tidy
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
    regime: crate::forms::InformationReturnRegime,
) -> PrintedForms {
    let status = ri.filing_status;
    let pi = &ar.printed_inputs;

    // The 8949 is built FIRST: Schedule D's lines 3 and 10 are its printed column totals, so the
    // detail form is upstream of the schedule that summarizes it.
    // ★ spec 1099-DA R2 — the boxes are ROUTED from the filer's Form 1099-DA answers on a live year.
    //   `screen_absolute` is the gate that refuses an unanswered key, Mixed or BasisDiffers before
    //   any packet is assembled; this is the backstop, and it is loud, never a silent I/L.
    let mut rows = crate::forms::form_8949(state, year);
    crate::forms::route_8949_boxes(&mut rows, regime, &ri.broker_reporting).unwrap_or_else(|e| {
        panic!("assemble_printed_forms reached an unscreened Form 1099-DA key — screen_absolute is the gate and must run first: {e:?}")
    });
    let f8949 = form_8949_printed(&rows);

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
        // Part II line 9b — the filer's own allocation, carried through `PrintedInputs` so this chain
        // and the absolute one at `return_1040.rs` read the SAME value (see `form_8960`'s doc).
        pi.form_8960_line9b,
    );
    let f8995 = form_8995_lines(
        // Row 1i(a): the trade or business the §199A deduction is claimed for. Line 2's own text says
        // "Combine lines 1i through 1v, column (c)" — a total over an empty column names no business.
        &pi.schedule_c_header.business_description,
        pi.business_qbi,
        // ★★★ §G-28/B1b — the SAME Form 8995-A line 16 the 1040's own chain read. Transcribed from
        //     `AbsoluteReturn`, never re-derived: the printed form's line 27 is "Enter the amount
        //     from line 16", and a second derivation here is exactly how it once printed $45,267
        //     against its own line 16 of $27,357.
        ar.f8995a_parts_i_to_iii.as_ref().map(|f| f.part_ii.line16),
        pi.reit_dividends,
        pi.reit_ptp_carryforward_in,
        pi.qbi_carryforward_in,
        pi.ti_before_qbi,
        pi.qbi_net_capital_gain,
    );

    // ★★★ §G-28/B1a — WHICH §199A FORM. Above the §199A(e)(2) threshold the simplified Form 8995 no
    //     longer applies (i8995a, "Who Must File"), so the same chain is transcribed onto Form 8995-A
    //     Part IV instead. `qbi_a` carries the written proof that the two forms' arithmetic is
    //     pointwise identical, so this is a FORM choice and never a figure change.
    //
    //     ★★ Exactly one of the two is `Some`. Filing both would claim the deduction twice on paper.
    //     ★ Read from `AbsoluteReturn`, which decided it where `FullReturnParams` is in scope. The
    //       printer transcribes a decision; it does not re-derive one.
    let on_8995a = ar.uses_8995a;
    let f8995a = on_8995a
        .then(|| {
            f8995.as_ref().map(|l| {
                // ★★★ §G-28/B1b — Parts I–III travel WITH Part IV, built from the same `PrintedInputs`
                //     `AbsoluteReturn` decided. Parts I–III are `None` for a filer with no qualified
                //     trade or business (REIT/PTP only, or an SSTB §199A(d)(3) excluded), which is
                //     i8995a's own "skip Parts I through III and complete Part IV".
                crate::tax::qbi_a::Form8995A {
                    // ★ TRANSCRIBED from `AbsoluteReturn`, which decided it where
                    //   `FullReturnParams` is in scope. The printer never re-derives.
                    parts_i_to_iii: ar.f8995a_parts_i_to_iii.clone(),
                    part_iv: crate::tax::qbi_a::Form8995APartIv::from_8995(l),
                }
            })
        })
        .flatten();
    let f8995 = if f8995a.is_some() { None } else { f8995 };

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
    // §G-6 — the attached Form 6251, decided by its OWN Who-Must-File test. Bound before Schedule 2
    // because Schedule 2 line 2 is Form 6251's printed line 11 and must not re-derive it.
    let f6251 = ar.amt.must_attach().then(|| ar.amt.clone());
    let sch_2 = schedule_2_lines(
        sch_se.as_ref(),
        &f8959,
        f8960.as_ref(),
        f6251.as_ref(),
        ar.form_8889.as_ref(),
    );
    let sch_3 = schedule_3_lines(ar);

    // Form 8283 files only when the return ITEMIZES and its printed noncash gifts clear the $500
    // threshold printed on Schedule A line 12 — a standard-deduction year with donations files none.
    let f8283 = sch_a
        .as_ref()
        .filter(|a| a.line12 > FORM_8283_THRESHOLD)
        .and_then(|_| {
            form_8283_printed(
                &crate::forms::form_8283(state, year, donation_details),
                // ★ §G-21 — the filer's answer to lines 5a/5b/5c, carried straight through. A
                // `Some(true)` never gets here: `screen_absolute` refuses the year.
                ri.donations_had_restrictions,
            )
        });

    // Form 8275 (Task 16) — `Some` iff a promoted-basis DISPOSAL leg files in `year`, OR the return
    // computes on FR-29's §1(g) no-path certification; the printed (whole-dollar-rounded Part I)
    // content of `crate::tax::form8275::disclosure_8275`, whose own scoping already omits a promoted
    // REMOVAL-only year (BG-D11).
    //
    // ★★★ FR-29 / SPEC §6.3.3 — the certification is the ONE outcome of the Form 8615 ladder that is
    //     not a refusal: the return computes and files. It must not file SILENTLY, so the §1(g)
    //     position rides Part I here. `ar.form8615_certification` was decided once in
    //     `assemble_absolute`; nothing re-derives it. Column (f) is 1040 line 16 AS FILED, which is
    //     `ar.regular_tax` — the tax computed WITHOUT §1(g), i.e. exactly the position disclosed.
    let section_1g = ar
        .form8615_certification
        .map(|c| crate::tax::form8275::Section1gPosition {
            unearned: c.unearned,
            threshold: c.threshold,
            tax_line16: ar.regular_tax,
        });
    let f8275 = crate::tax::form8275::disclosure_8275(events, state, year, section_1g.as_ref())
        .map(|d| printed_8275(&d));

    let f1040 = form_1040_lines(
        ar,
        &income,
        sch_a.as_ref(),
        sch_2.as_ref(),
        sch_3.as_ref(),
        &f8959,
        f8995.as_ref(),
        f8995a.as_ref(),
        table,
        status,
        ri.payments.other_withholding,
        ri.payments.estimated_tax_payments,
        pi.digital_asset_answer,
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
        f8995a,
        // §G-6 — attached exactly when the form's own Who-Must-File test says so.
        f6251,
        f8283,
        f8275,
        // ★ T16 — the ONE derivation, computed in `assemble_absolute` beside Form 6251 and read
        //   here; Schedule 1 lines 8f/13 and Schedule 2 lines 17c/17d read the same struct.
        f8889: ar.form_8889.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_1040::assemble_absolute;
    use crate::tax::return_inputs::{Dependent, HouseholdHeader, ScheduleCInputs, W2};
    use crate::tax::testonly::{
        amt_owing_household, every_money_leaf_household, kitchen_sink_household, ty2024_params,
        ty2024_table, w2_only_household,
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
    /// ★★★ §G-20 — the MFS spouse's §63(f) boxes are claimable, and the gate FAILS CLOSED.
    ///
    /// i1040gi: *"…married filing separately … you can check the appropriate box(es) … **if your spouse
    /// had no income, isn't filing a return, and can't be claimed as a dependent on another person's
    /// return**."* All three must be affirmatively answered in the claiming direction.
    ///
    /// ★★ **This is the only change on the branch where an answer can ONLY reduce tax**, so every
    /// negative case matters more than the positive one. Forgoing costs the filer a deduction they can
    /// recover by answering; granting one they are not entitled to understates a signed return, which
    /// they cannot recover. The asymmetry is why *any* unanswered condition forgoes.
    #[test]
    fn the_mfs_spouse_63f_boxes_need_all_three_conditions_and_fail_closed() {
        let build = |no_income: Option<bool>, not_filing: Option<bool>, claimable: Option<bool>| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Mfs,
                ..Default::default()
            };
            ri.header.taxpayer = person("John", "Doe", "123456789");
            ri.header.spouse = Some(Person {
                date_of_birth: Some(date!(1955 - 03 - 02)), // 65+
                blind: Some(true),                          // both boxes would qualify
                ..person("Jane", "Doe", "987654321")
            });
            ri.header.spouse_died_during_year = Some(false);
            ri.header.spouse_had_no_income = no_income;
            ri.header.spouse_not_filing_a_return = not_filing;
            ri.header.can_be_claimed_as_dependent_spouse = claimable;
            AgedBlindBoxes::for_return(&ri, 2024)
        };

        // ★ ALL THREE in the claiming direction ⇒ both boxes count.
        let b = build(Some(true), Some(true), Some(false));
        assert!(
            b.spouse_aged && b.spouse_blind,
            "all three answered ⇒ claimed"
        );
        assert_eq!(b.count(), 2);

        // ★★ Every other combination forgoes. Each row is a DIFFERENT way to be un-entitled or
        //    unanswered, and any one of them must be enough on its own.
        for (label, a, b_, c) in [
            ("no-income unanswered", None, Some(true), Some(false)),
            ("no-income denied", Some(false), Some(true), Some(false)),
            ("not-filing unanswered", Some(true), None, Some(false)),
            ("not-filing denied", Some(true), Some(false), Some(false)),
            ("dependent-flag unanswered", Some(true), Some(true), None),
            (
                "spouse IS claimable elsewhere",
                Some(true),
                Some(true),
                Some(true),
            ),
            ("nothing answered", None, None, None),
        ] {
            let boxes = build(a, b_, c);
            assert_eq!(
                boxes.count(),
                0,
                "★ {label}: the spouse's boxes must be FORGONE. Granting one here understates a \
                 signed return; forgoing merely costs a deduction the filer can recover by answering."
            );
        }
    }

    /// ★★ The gate and the QUESTIONS it enables are one predicate — `questions::spouse_63f_boxes_count`.
    ///
    /// Two copies would drift into the worst shape available here: counting a spouse's aged box on a
    /// return where `SpouseDiedDuringYear` was never asked, so the §G-9 death carve-out — the whole
    /// point of that gate — could not have been applied.
    #[test]
    fn the_mfs_spouse_death_gate_is_asked_exactly_when_the_boxes_count() {
        use crate::tax::questions::{spouse_63f_boxes_count, SkippableId, SKIPPABLE_QUESTIONS};
        let gate = SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == SkippableId::SpouseDiedDuringYear)
            .unwrap();
        let mk = |claimable: bool| {
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Mfs,
                ..Default::default()
            };
            ri.header.spouse = Some(Person::default());
            if claimable {
                ri.header.spouse_had_no_income = Some(true);
                ri.header.spouse_not_filing_a_return = Some(true);
                ri.header.can_be_claimed_as_dependent_spouse = Some(false);
            }
            ri
        };
        for claimable in [true, false] {
            let ri = mk(claimable);
            assert_eq!(
                (gate.live)(&ri),
                spouse_63f_boxes_count(&ri),
                "the death gate must be asked exactly when the boxes can count (claimable={claimable})"
            );
            assert_eq!((gate.live)(&ri), claimable);
        }
    }

    /// ★★★ §G-9a — **the blindness box has NO death interaction; the aged box does.** One person,
    /// one date of death, both conditions true: the aged box is carved out and the blind box is not.
    ///
    /// This is the executable form of the adjudication written on [`AgedBlindBoxes::for_return`]. The
    /// asymmetry looks like an oversight — the same statute, the same dollars, the same worksheet line
    /// — so it is exactly the kind of thing a later reader "tidies up". i1040gi states the carve-out
    /// for one box by its printed label and gives blindness its own date anchor (*"blind at the end of
    /// 2024"*); a decedent's tax year ends at death, so someone blind when they died was blind at the
    /// end of it. Harmonising the two boxes forfeits a deduction the filer is entitled to, and this
    /// test is what stops it happening quietly.
    #[test]
    fn the_blind_box_has_no_death_carve_out_but_the_aged_box_does() {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        // Born 1959-02-14 ⇒ reaches 65 on 1960-02-13… i.e. on 2024-02-13. Died 2024-02-12: ONE DAY
        // short, the i1040gi worked example. Blind at death.
        ri.header.taxpayer = Person {
            date_of_birth: Some(date!(1959 - 02 - 14)),
            date_of_death: Some(date!(2024 - 02 - 12)),
            blind: Some(true),
            ..person("John", "Doe", "123456789")
        };
        ri.header.taxpayer_died_during_year = Some(true);
        let b = AgedBlindBoxes::for_return(&ri, 2024);
        assert!(
            !b.taxpayer_aged,
            "died one day before reaching 65 ⇒ the AGE box is carved out (i1040gi's own example)"
        );
        assert!(
            b.taxpayer_blind,
            "★ …and the BLINDNESS box is NOT. Blindness is tested at the end of the tax year, and a \
             decedent's year ends at death. Harmonising these two forfeits a real deduction."
        );
        assert_eq!(b.count(), 1);

        // The control: one day later, and the age box is checked too (so the assertion above is
        // about the CARVE-OUT, not about the fixture failing some other way).
        ri.header.taxpayer.date_of_death = Some(date!(2024 - 02 - 13));
        let b = AgedBlindBoxes::for_return(&ri, 2024);
        assert!(b.taxpayer_aged && b.taxpayer_blind);
        assert_eq!(b.count(), 2);
    }

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
        // ★ The age-65 box is now CLAIMED, not defaulted: since the §G-9 death gates became class-(B)
        // skippables, `is_aged` forgoes the addition while "did you die during the year?" is unanswered.
        // A fixture that wants the box must say so, exactly as it must state `blind` and the DOB.
        ri.header.taxpayer_died_during_year = Some(false);
        ri.header.spouse = Some(Person {
            blind: Some(true),
            ..person("Jane", "Doe", "987654321")
        });

        // ★ ONE `ri` for both halves. It used to answer the declarations for `ReturnHeader::build` and
        // pass the RAW inputs to `assemble_absolute`, which the §G-9 death gate exposed: the header saw
        // an answered "did not die" and granted the aged box while the raw inputs did not, so the two
        // halves of this very equality were computed from different returns.
        let ri = crate::tax::testonly::answered(ri);
        let h = ReturnHeader::build(&ri, 2024).unwrap();
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
            // ★ T7 — STATED, not left at §G-15's `0` sentinel: `answer_all_dependent_gates` derives
            //   the dependent's date of birth from it (see its own assertion).
            tax_year: 2024,
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
            // ★ T7 — STATED, not left at §G-15's `0` sentinel (see `answer_all_dependent_gates`).
            tax_year: 2024,
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
            crate::forms::InformationReturnRegime::NONE,
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
    /// ★★★ §G-6 — SCHEDULE 2 LINE 2 IS NON-BLANK **EXACTLY** WHEN FORM 6251 IS IN THE PACKET.
    ///
    /// Two failures live here, in opposite directions, and both are Critical:
    ///
    ///   * a packet carrying **Form 6251 with no Schedule 2 line 2** files a form claiming an AMT the
    ///     return never assesses — the return UNDERSTATES tax while the evidence for the shortfall is
    ///     stapled to it. (That is the state this tree was in mid-build, and the class the 1040-vs-8995-A
    ///     pair was caught in.)
    ///   * a packet carrying **line 2 with no Form 6251** swears the filer figured an AMT on a form the
    ///     IRS never receives (§G-11 — *an entry is testimony*).
    ///
    /// ★ Both directions collapse to one `assert_eq!` on the two `is_some()`s, so neither can be
    /// satisfied without the other, and the VALUE is checked against the form's own line 11 so the
    /// figure cannot be re-derived. `must_attach()` is the single predicate: it decides the attachment,
    /// and the attachment decides the line.
    #[test]
    fn schedule_2_line_2_is_non_blank_exactly_when_form_6251_is_attached() {
        // ★★★ THE `expect_attached` COLUMN IS THE ANTI-VACUITY GUARD, and it is not decoration.
        //     The first draft of this test iterated only the two general fixtures — NEITHER of which
        //     owes AMT — so it asserted `None == None` twice and passed against a tree whose Schedule 2
        //     had no line 2 at all. Declaring the expected side per household means the biconditional
        //     can never again be satisfied by both sides being empty.
        for (name, hh, expect_attached) in [
            ("kitchen sink", kitchen_sink_household(), false),
            ("W-2 only", w2_only_household(), false),
            ("AMT-owing", amt_owing_household(), true),
        ] {
            let (ri, state) = hh;
            let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
            let pr = assemble_printed_return(
                &ri,
                &state,
                &BTreeMap::new(),
                &ar,
                &ty2024_table(),
                2024,
                &[],
                crate::forms::InformationReturnRegime::NONE,
            )
            .unwrap();
            let line2 = pr.forms.sch_2.as_ref().and_then(|s| s.line2);
            assert_eq!(
                pr.forms.f6251.is_some(),
                expect_attached,
                "{name}: this fixture is here to exercise attached = {expect_attached}. If the AMT \
                 case stops attaching, the biconditional below goes VACUOUS and stops discriminating."
            );
            assert_eq!(
                pr.forms.f6251.is_some(),
                line2.is_some(),
                "{name}: Form 6251 attached = {}, but Schedule 2 line 2 = {line2:?}. A packet may not \
                 carry one without the other in EITHER direction.",
                pr.forms.f6251.is_some()
            );
            if name == "AMT-owing" {
                // ★★★ PINNED TO THE ORACLE-VALIDATED VECTOR
                //     (`design/direction/G6-AMT-ORACLE-VALIDATION.md`). If this fixture drifts, it stops
                //     being the return the two engines witnessed and these KATs start validating a
                //     taxpayer nobody checked.
                assert_eq!(
                    crate::conventions::round_dollar(pr.forms.f6251.as_ref().unwrap().line11),
                    dec!(11322),
                    "AMT must stay the figure Tax-Calculator reproduces to $3 and OTS corroborates"
                );
                assert_eq!(
                    pr.forms.f1040.line24,
                    dec!(481225),
                    "total tax, same vector"
                );
            }
            if let (Some(f), Some(l2)) = (pr.forms.f6251.as_ref(), line2) {
                assert_eq!(
                    l2,
                    crate::conventions::round_dollar(f.line11),
                    "{name}: Schedule 2 line 2 must TRANSCRIBE Form 6251 line 11, never re-derive it"
                );
                assert_eq!(
                    f.printed().line11,
                    l2,
                    "{name}: …and the two must agree AS PRINTED — same rounding, one figure, two pages"
                );
            }
        }
    }

    /// ★★★ **THE TWO-CHAIN COMPARISON'S HOUSEHOLDS, AND THE ANTI-VACUITY GUARD EACH ONE CARRIES.**
    ///
    /// Two named households that exercise a specific term, and — since the T16 seam review — one
    /// STRUCTURAL fixture that exercises **every money leaf `ReturnInputs` has**
    /// ([`every_money_leaf_household`], derived from `maximal_sentinel` and the type-driven money
    /// detector, so a leaf added tomorrow is populated with nobody remembering to do it).
    ///
    /// ★★ **Why the third row is not just a third fixture.** The first two are a HAND LIST, and a
    /// hand list is written by whoever adds the term — which is exactly how T16 added Schedule 1
    /// line 8f and Schedule 2 lines 17c/17d to the printed chain, left them out of
    /// `AbsoluteReturn`, and left this test green. The structural row cannot be defeated that way:
    /// any future money leaf that reaches a printed line without reaching the absolute chain reds
    /// here **without anyone naming a household**.
    ///
    /// Each row asserts its own non-vacuity, because a comparison that no longer exercises the term
    /// it was written for passes forever.
    fn two_chain_households() -> Vec<(&'static str, AbsoluteReturn, PrintedReturn)> {
        let mut out = Vec::new();
        for (name, hh) in [
            ("kitchen sink", kitchen_sink_household()),
            ("AMT-owing", amt_owing_household()),
            ("every money leaf", every_money_leaf_household()),
        ] {
            let (ri, state) = hh;
            let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
            let pr = assemble_printed_return(
                &ri,
                &state,
                &BTreeMap::new(),
                &ar,
                &ty2024_table(),
                2024,
                &[],
                crate::forms::InformationReturnRegime::NONE,
            )
            .unwrap();
            let zero = crate::conventions::Usd::ZERO;
            match name {
                // §G-6 — the AMT term, which is what this comparison was originally written for.
                "kitchen sink" => assert!(
                    ar.amt.line11 <= zero,
                    "kitchen sink must NOT owe AMT, or the FALSE case of the Schedule-2-line-2 \
                     biconditional has no witness"
                ),
                "AMT-owing" => assert!(
                    ar.amt.line11 > zero,
                    "AMT-owing must exercise AMT, else this comparison goes vacuous on the one term \
                     it exists to catch"
                ),
                // The T16 seam review's two legs, plus the claim that makes the row structural.
                "every money leaf" => {
                    let money = crate::tax::provenance::leaf_walk::money_leaves(&ri);
                    let nonzero = crate::tax::provenance::leaf_walk::nonzero_money_leaves(&ri);
                    let blank: Vec<&String> =
                        money.iter().filter(|p| !nonzero.contains_key(*p)).collect();
                    assert!(
                        blank.is_empty(),
                        "every money leaf must carry a figure or this row is silent on it: {blank:?}"
                    );
                    let s1 = pr.forms.sch_1.as_ref().expect("Schedule 1 files");
                    let s2 = pr.forms.sch_2.as_ref().expect("Schedule 2 files");
                    assert!(
                        s1.line8f > zero,
                        "the fixture must produce a non-zero Schedule 1 line 8f — the income leg \
                         C-1 found missing from `schedule_1_income`"
                    );
                    assert!(
                        s2.line17c.is_some_and(|v| v > zero),
                        "…and a non-zero Schedule 2 line 17c — the additional-tax leg I-1 found \
                         missing from `schedule_2_other_taxes`"
                    );
                }
                other => panic!("no anti-vacuity guard for {other:?}"),
            }
            out.push((name, ar, pr));
        }
        out
    }

    /// ★★★ §G-6 — THE ABSOLUTE `total_tax` EQUALS THE PRINTED 1040 LINE 24.
    ///
    /// btctax carries two chains: the unrounded `AbsoluteReturn` and the whole-dollar printed packet.
    /// They are assembled by different code, and **only the printed one is read today** — `total_tax`
    /// has no consumer outside `assemble_absolute`. That is precisely how it came to be short by the
    /// entire AMT: `l18` was hardcoded to `regular_tax` with a comment citing the AMT refusal as its
    /// warrant, the refusal was removed, and the printed chain was fixed while the absolute one was not.
    /// Nothing was red, because nothing compared them.
    ///
    /// ★ This is the comparison. A figure with no reader is not thereby correct — it is a wrong number
    ///   waiting for its first caller.
    ///
    /// ★★ ON THE STRICTNESS: `round_dollar(ar.total_tax)` rounds an exact-cents sum, while
    /// `f1040.line24` sums independently rounded lines, so under SPEC §3.1's per-line rounding these
    /// are not equal *in general* — they can legitimately differ by a dollar or two. Exact equality is
    /// kept anyway, because it is the strongest statement these fixtures support and it is what caught
    /// a $13,461 understatement. **If a future fixture reds this for a rounding reason, the fix is to
    /// say so in the message and widen to a named tolerance — NOT to weaken it to something that would
    /// also have passed with the AMT missing.**
    ///
    /// ★★★ **T16 SEAM REVIEW I-1 — it happened again, and this test did not see it.** Form 8889 line
    /// 17b → Schedule 2 line 17c and line 21 → 17d were summed by the printed chain and not by
    /// `schedule_2_other_taxes`. The test was green because its fixture list held two households,
    /// neither with an HSA. The list is now `two_chain_households`, whose third row is derived from
    /// every money leaf the input type has — so the next term added to one chain and not the other
    /// reds here whether or not anyone thinks to add a household.
    #[test]
    fn the_absolute_total_tax_equals_the_printed_1040_line_24() {
        for (name, ar, pr) in two_chain_households() {
            assert_eq!(
                crate::conventions::round_dollar(ar.total_tax),
                pr.forms.f1040.line24,
                "{name}: the absolute total tax and the FILED 1040 line 24 must be the same number. \
                 A gap here is what the filer signs versus what the engine believes."
            );
        }
    }

    /// ★★★ **THE TWIN, ONE STAGE UPSTREAM — the absolute AGI equals the printed 1040 line 11, and
    ///     the absolute taxable income equals line 15.**
    ///
    /// T16 seam review **C-1**. `total_tax` is not the only figure the two chains both produce, and
    /// it is not the earliest: `agi` is the argument to `student_loan_deduction`, `form_8960` (the
    /// §1411 MAGI), `assemble_amt`, the §170(b) contribution base, the §213(a) medical floor,
    /// `ctc_odc_line19`, `ti_before_qbi`, the itemize-vs-standard election and `regular_tax`. Four
    /// of those produce PRINTED figures, so an absolute AGI that disagrees with the filed line 11
    /// does not merely sit unread — it moves a signed line. It did: with Schedule 1 line 8f missing
    /// from `schedule_1_income`, an $85,000 household with a $9,000 HSA distribution printed a
    /// §221 student-loan deduction $1,500 too large.
    ///
    /// ★ Line 24's test could not catch it. On a fully-excepted distribution the §223(f)(4) tax is
    ///   zero, so `total_tax` moved only through the phase-out — and a comparison that starts at
    ///   line 24 sees an AGI error only when it survives every credit and every floor between here
    ///   and there. The earliest common figure is the right place to compare.
    #[test]
    fn the_absolute_agi_equals_the_printed_1040_line_11() {
        for (name, ar, pr) in two_chain_households() {
            assert_eq!(
                crate::conventions::round_dollar(ar.agi),
                pr.forms.f1040.line11,
                "{name}: the absolute AGI and the FILED 1040 line 11 must be the same number — \
                 every Schedule 1 Part I income reaches line 9 through line 10, and every Part II \
                 adjustment reaches line 11 through line 10a"
            );
            assert_eq!(
                crate::conventions::round_dollar(ar.taxable_income),
                pr.forms.f1040.line15,
                "{name}: …and so must taxable income, which is figured from that AGI"
            );
        }
    }

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
            crate::forms::InformationReturnRegime::NONE,
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

        assert!(
            !pr.forms.sch_d.must_file(),
            "a W-2-only filer has no Schedule D to attach"
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
            crate::forms::InformationReturnRegime::NONE,
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
