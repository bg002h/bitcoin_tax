//! ★★★ §G-11 LINE COVERAGE — every printed MONEY field declares which production it transcribes.
//!
//! **Why this exists, and why it is a TEST rather than a document.** Two independent Opus review
//! rounds measured, by hand, how many money lines fit the SPEC's combinator grammar: r1 found ≈25
//! misfits against four productions, r2 found 10–29 against eight. Both exceeded the SPEC's own
//! threshold, and both cost a full review round to learn a number. `CLAUDE.md` already says what to do
//! about that: *"Is every form line present? Does each doc comment match the instruction text? are
//! ASSERTIONS, not opinions"* — **conformance ⇒ test.** So the count moves here, where it is computed
//! on every commit and cannot go stale.
//!
//! **The compile-time half.** Every printed struct is destructured with **NO `..`**, under
//! `#![deny(unused_variables)]`, exactly as [`crate::tax::classifier`] does for answered-ness. A newly
//! added money field is a *pattern does not mention field* COMPILE ERROR until a human assigns it a
//! production. That is the §G-11 SPEC's "the 65th site cannot choose a silent zero", arrived at from
//! the front — before the types, rather than after the migration.
//!
//! ★ **The `_` rule, extended to money.** `classifier.rs` deliberately PERMITS `_` on `Usd` leaves,
//! which is why it is blind to all 64 fabrication sites. Here `_` is **FORBIDDEN** on money: a `Usd`
//! must be passed to [`Coverage::line`], which consumes it. Non-money leaves (`String`, `bool`) may be
//! `_`-bound, because they are not this module's business.
//!
//! ★★ **THE HONEST LIMIT, stated so it is not mistaken for completeness.** This covers every money
//! field declared DIRECTLY on a covered struct. Money that lives in a NESTED type is not yet reached:
//! `ScheduleBRow.amount` (the interest/dividend payer rows, two `Vec`s on `ScheduleBLines`) and
//! `ScheduleDRouting::NetLoss.line21`. Those are real figures that reach paper, and their absence here
//! is a gap in coverage, NOT an assertion that they are safe. Filed as §G-11 nested-money; the fix is
//! one `cover_*` per nested type, exactly like the ones below.
//!
//! **The text half** lives in `xtask` (`publish = false`, so it may read the repo tree): it checks
//! every `instruction` here against `design/forms/extract/`, verbatim modulo whitespace. An
//! `include_str!` reaching out of this crate would ship a broken tarball with exit 0 — the trap
//! `crate-publishing-state` records — so the quote travels as data and the checking travels to xtask.

#![deny(unused_variables)]

use crate::conventions::Usd;

/// Which sentence-shape of the IRS instruction language this line transcribes.
///
/// ★ These are **productions of the forms' own grammar**, not categories we invented: each is keyed to
/// a closed, quotable phrase family that recurs across independent forms. A line that fits none is an
/// [`Production::Exception`] **with a written reason** — never a silent nearest-fit, which is how the
/// SPEC's r1 `computed` became a residual bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Production {
    /// *(the filer supplies it)* — blank exactly when not asked.
    Collected,
    /// *"Enter the amount from line N"* — blank when the source line is blank.
    Carry,
    /// *"Add lines X through Y"* / *"Combine lines X and Y"*, **with no clamp clause**.
    /// Blank iff every operand is blank.
    ///
    /// ★★ The trigger is the ABSENCE OF A CLAMP, not the verb. SPEC r2 keyed this on the verb while
    /// its own section title said otherwise, which routed 12 unfloored `"Subtract"` lines — including
    /// 1040 line 37, live on every refund return — into "always print a zero".
    Combine,
    /// Arithmetic **plus an explicit clamp-to-zero clause**. Always entered: the form instructs the
    /// figure, so a blank would be the mirror defect.
    Clamped(Polarity),
    /// *"Multiply line N by 20% (0.20)"* — blank when the source is blank.
    Scaled,
    /// *"Enter the smaller of line X or line Y"* — blank when all inputs are blank.
    Bounded,
    /// *"Threshold based on filing status"* — a figure the form or statute supplies. Never blank.
    Constant,
    /// Fits no production. **Requires a reason**, and the count is ratcheted.
    Exception,
}

/// Which way an *"enter -0-"* clause clamps.
///
/// ★★ **Polarity is transcribed, never inferred.** Form 8995 lines 4 and 8 read *"If zero or less,
/// enter -0-"* while lines 16 and 17 read *"If **greater** than zero, enter -0-"* — the latter are
/// parenthesised loss carryforwards, where a floor would be exactly backwards. SPEC r2 quoted only the
/// first sentence and would have mis-transcribed all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// *"If zero or less, enter -0-"* — clamp up.
    FloorAtZero,
    /// *"If greater than zero, enter -0-"* — clamp down (parenthesised loss lines).
    CeilAtZero,
}

/// One printed money line and the production it transcribes.
#[derive(Debug, Clone)]
pub struct LineCoverage {
    /// The extract stem the instruction is quoted from, e.g. `"f8995"`.
    pub form: &'static str,
    /// The form's own line label, e.g. `"16"` — for humans and for the map cross-check.
    pub line: &'static str,
    /// The Rust field, e.g. `"line16"`.
    pub field: &'static str,
    pub production: Production,
    /// **Verbatim from `design/forms/extract/<form>--YYYY.txt`.** Compared modulo whitespace, because
    /// `pdftotext -layout` wraps clauses mid-sentence (f8995 line 8's *"If zero / or less"*).
    pub instruction: &'static str,
    /// Required **iff** `production == Exception`; forbidden otherwise. Both directions are checked.
    pub reason: Option<&'static str>,
}

/// Accumulator. [`Self::line`] takes the money value itself so `#![deny(unused_variables)]` bites on
/// any field the destructure names but does not classify.
#[derive(Debug, Default)]
pub struct Coverage(pub Vec<LineCoverage>);

impl Coverage {
    /// Record one money line. `_value` exists to be CONSUMED — that is the compile-time guarantee.
    #[allow(clippy::too_many_arguments)]
    pub fn line(
        &mut self,
        _value: Usd,
        form: &'static str,
        line: &'static str,
        field: &'static str,
        production: Production,
        instruction: &'static str,
    ) {
        self.0.push(LineCoverage {
            form,
            line,
            field,
            production,
            instruction,
            reason: None,
        });
    }

    /// Record a line that fits no production. The reason is mandatory and is checked.
    #[allow(clippy::too_many_arguments)]
    pub fn exception(
        &mut self,
        _value: Usd,
        form: &'static str,
        line: &'static str,
        field: &'static str,
        instruction: &'static str,
        reason: &'static str,
    ) {
        self.0.push(LineCoverage {
            form,
            line,
            field,
            production: Production::Exception,
            instruction,
            reason: Some(reason),
        });
    }
}

/// **Form 8995** — the whole struct, destructured with no `..`.
///
/// Chosen as the first covered form because it exercises six of the eight productions AND both clamp
/// polarities, so the mechanism is proved on a hard case rather than an easy one.
pub fn cover_f8995(l: &crate::tax::qbi::Form8995Lines) -> Coverage {
    let crate::tax::qbi::Form8995Lines {
        business_name: _, // not money — out of this module's scope
        line2,
        line3,
        line4,
        line5,
        line6,
        line7,
        line8,
        line9,
        line10,
        line11,
        line12,
        line13,
        line14,
        line15,
        line16,
        line17,
    } = l;
    let f = "f8995";
    let mut c = Coverage::default();
    c.line(
        *line2,
        f,
        "2",
        "line2",
        Production::Combine,
        "Total qualified business income or (loss). Combine lines 1i through 1v, column (c)",
    );
    c.line(
        *line3,
        f,
        "3",
        "line3",
        Production::Collected,
        "Qualified business net (loss) carryforward from the prior year",
    );
    c.line(
        *line4,
        f,
        "4",
        "line4",
        Production::Clamped(Polarity::FloorAtZero),
        "Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-",
    );
    c.line(
        *line5,
        f,
        "5",
        "line5",
        Production::Scaled,
        "Qualified business income component. Multiply line 4 by 20% (0.20)",
    );
    c.line(
        *line6,
        f,
        "6",
        "line6",
        Production::Collected,
        "Qualified REIT dividends and publicly traded partnership (PTP) income or (loss)",
    );
    c.line(
        *line7,
        f,
        "7",
        "line7",
        Production::Collected,
        "Qualified REIT dividends and qualified PTP (loss) carryforward from the prior",
    );
    c.line(*line8, f, "8", "line8", Production::Clamped(Polarity::FloorAtZero),
        "Total qualified REIT dividends and PTP income. Combine lines 6 and 7. If zero or less, enter -0-");
    c.line(
        *line9,
        f,
        "9",
        "line9",
        Production::Scaled,
        "REIT and PTP component. Multiply line 8 by 20% (0.20)",
    );
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Combine,
        "Qualified business income deduction before the income limitation. Add lines 5 and 9",
    );
    c.line(
        *line11,
        f,
        "11",
        "line11",
        Production::Carry,
        "Taxable income before qualified business income deduction (see instructions)",
    );
    c.line(
        *line12,
        f,
        "12",
        "line12",
        Production::Collected,
        "Enter your net capital gain, if any, increased by any qualified dividends",
    );
    c.line(
        *line13,
        f,
        "13",
        "line13",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 12 from line 11. If zero or less, enter -0-",
    );
    c.line(
        *line14,
        f,
        "14",
        "line14",
        Production::Scaled,
        "Income limitation. Multiply line 13 by 20% (0.20)",
    );
    c.line(
        *line15,
        f,
        "15",
        "line15",
        Production::Bounded,
        "Qualified business income deduction. Enter the smaller of line 10 or line 14.",
    );
    // ★★ Lines 16/17 are the INVERTED clause — a ceiling, not a floor. See [`Polarity`].
    c.line(*line16, f, "16", "line16", Production::Clamped(Polarity::CeilAtZero),
        "Total qualified business (loss) carryforward. Combine lines 2 and 3. If greater than zero, enter -0-");
    c.line(*line17, f, "17", "line17", Production::Clamped(Polarity::CeilAtZero),
        "Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 6 and 7. If greater than zero, enter -0-");
    c
}

/// A zero-valued Form 8995 for table extraction.
///
/// ★ Deliberately NOT `#[derive(Default)]` on `Form8995Lines` — §G-11's SPEC forbids `Default` on money
/// -bearing printed structs, because an implicit constructor is exactly how a field arrives unexamined.
/// Naming every field here gives a SECOND compile guarantee: a new field is `E0063` here as well as
/// *pattern does not mention field* in [`cover_f8995`].
fn zero_f8995() -> crate::tax::qbi::Form8995Lines {
    crate::tax::qbi::Form8995Lines {
        business_name: String::new(),
        line2: Usd::ZERO,
        line3: Usd::ZERO,
        line4: Usd::ZERO,
        line5: Usd::ZERO,
        line6: Usd::ZERO,
        line7: Usd::ZERO,
        line8: Usd::ZERO,
        line9: Usd::ZERO,
        line10: Usd::ZERO,
        line11: Usd::ZERO,
        line12: Usd::ZERO,
        line13: Usd::ZERO,
        line14: Usd::ZERO,
        line15: Usd::ZERO,
        line16: Usd::ZERO,
        line17: Usd::ZERO,
    }
}

/// **ScheduleBLines** (f1040sb) — destructured with no `..`; a new money field cannot compile.
pub fn cover_scheduleblines(l: &crate::tax::printed::ScheduleBLines) -> Coverage {
    let crate::tax::printed::ScheduleBLines {
        part1_rows: _,
        line2,
        line4,
        part2_rows: _,
        line6,
        foreign_accounts_7a: _,
        foreign_trust_8: _,
        line7b_countries: _,
        fbar_filing_required: _,
    } = l;
    let f = "f1040sb";
    let mut c = Coverage::default();
    c.line(
        *line2,
        f,
        "2",
        "line2",
        Production::Combine,
        "Add the amounts on line 1",
    );
    c.line(
        *line4,
        f,
        "4",
        "line4",
        Production::Combine,
        "Subtract line 3 from line 2. Enter the result here and on Form 1040 or 1040-SR, line 2b",
    );
    c.line(
        *line6,
        f,
        "6",
        "line6",
        Production::Combine,
        "Add the amounts on line 5. Enter the total here and on Form 1040 or 1040-SR, line 3b",
    );
    c
}

fn zero_scheduleblines() -> crate::tax::printed::ScheduleBLines {
    crate::tax::printed::ScheduleBLines {
        part1_rows: Vec::new(),
        line2: Usd::ZERO,
        line4: Usd::ZERO,
        part2_rows: Vec::new(),
        line6: Usd::ZERO,
        foreign_accounts_7a: None,
        foreign_trust_8: None,
        line7b_countries: String::new(),
        fbar_filing_required: None,
    }
}

/// **ScheduleALines** (f1040sa) — destructured with no `..`; a new money field cannot compile.
pub fn cover_schedulealines(l: &crate::tax::printed::ScheduleALines) -> Coverage {
    let crate::tax::printed::ScheduleALines {
        line5a_is_sales_tax: _,
        line18_elects_smaller: _,
        line8_mixed_use_box: _,
        line1,
        line2,
        line3,
        line4,
        line5a,
        line5b,
        line5c,
        line5d,
        line5e,
        line7,
        line8a,
        line8e,
        line10,
        line11,
        line12,
        line13,
        line14,
        line17,
    } = l;
    let f = "f1040sa";
    let mut c = Coverage::default();
    c.line(
        *line1,
        f,
        "1",
        "line1",
        Production::Collected,
        "Medical and dental expenses (see instructions)",
    );
    c.line(
        *line2,
        f,
        "2",
        "line2",
        Production::Carry,
        "Enter amount from Form 1040 or 1040-SR, line 11",
    );
    c.line(
        *line3,
        f,
        "3",
        "line3",
        Production::Scaled,
        "Multiply line 2 by 7.5% (0.075)",
    );
    c.line(
        *line4,
        f,
        "4",
        "line4",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 3 from line 1. If line 3 is more than line 1, enter -0-",
    );
    c.line(*line5a, f, "5a", "line5a", Production::Collected,
        "State and local income taxes or general sales taxes. You may include either income taxes or general sales taxes on line 5a, but not both");
    c.line(
        *line5b,
        f,
        "5b",
        "line5b",
        Production::Collected,
        "State and local real estate taxes (see instructions)",
    );
    c.line(
        *line5c,
        f,
        "5c",
        "line5c",
        Production::Collected,
        "State and local personal property taxes",
    );
    c.line(
        *line5d,
        f,
        "5d",
        "line5d",
        Production::Combine,
        "Add lines 5a through 5c",
    );
    c.line(
        *line5e,
        f,
        "5e",
        "line5e",
        Production::Bounded,
        "Enter the smaller of line 5d or $10,000 ($5,000 if married filing separately)",
    );
    c.line(
        *line7,
        f,
        "7",
        "line7",
        Production::Combine,
        "Add lines 5e and 6",
    );
    c.line(
        *line8a,
        f,
        "8a",
        "line8a",
        Production::Collected,
        "Home mortgage interest and points reported to you on Form 1098.",
    );
    c.line(
        *line8e,
        f,
        "8e",
        "line8e",
        Production::Combine,
        "Add lines 8a through 8c",
    );
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Combine,
        "Add lines 8e and 9",
    );
    c.line(
        *line11,
        f,
        "11",
        "line11",
        Production::Collected,
        "Gifts by cash or check. If you made any gift of $250 or more",
    );
    c.line(
        *line12,
        f,
        "12",
        "line12",
        Production::Collected,
        "Other than by cash or check. If you made any gift of $250 or more",
    );
    c.line(
        *line13,
        f,
        "13",
        "line13",
        Production::Collected,
        "Carryover from prior year",
    );
    c.line(
        *line14,
        f,
        "14",
        "line14",
        Production::Combine,
        "Add lines 11 through 13",
    );
    c.line(
        *line17,
        f,
        "17",
        "line17",
        Production::Combine,
        "Add the amounts in the far right column for lines 4 through 16.",
    );
    c
}

fn zero_schedulealines() -> crate::tax::printed::ScheduleALines {
    crate::tax::printed::ScheduleALines {
        line5a_is_sales_tax: false,
        line18_elects_smaller: false,
        line8_mixed_use_box: false,
        line1: Usd::ZERO,
        line2: Usd::ZERO,
        line3: Usd::ZERO,
        line4: Usd::ZERO,
        line5a: Usd::ZERO,
        line5b: Usd::ZERO,
        line5c: Usd::ZERO,
        line5d: Usd::ZERO,
        line5e: Usd::ZERO,
        line7: Usd::ZERO,
        line8a: Usd::ZERO,
        line8e: Usd::ZERO,
        line10: Usd::ZERO,
        line11: Usd::ZERO,
        line12: Usd::ZERO,
        line13: Usd::ZERO,
        line14: Usd::ZERO,
        line17: Usd::ZERO,
    }
}

/// **ScheduleCLines** (f1040sc) — destructured with no `..`; a new money field cannot compile.
pub fn cover_scheduleclines(l: &crate::tax::printed::ScheduleCLines) -> Coverage {
    let crate::tax::printed::ScheduleCLines {
        line_a_business: _,
        line_b_naics: _,
        line_f_accrual: _,
        line_i_1099_required: _,
        line_j_1099_filed: _,
        line1,
        line3,
        line5,
        line7,
        line28,
        line29,
        line31,
    } = l;
    let f = "f1040sc";
    let mut c = Coverage::default();
    c.line(*line1, f, "1", "line1", Production::Collected,
        "Gross receipts or sales. See instructions for line 1 and check the box if this income was reported to you on Form W-2 and the “Statutory employee” box on that form was checked");
    c.line(
        *line3,
        f,
        "3",
        "line3",
        Production::Combine,
        "Subtract line 2 from line 1",
    );
    c.line(
        *line5,
        f,
        "5",
        "line5",
        Production::Combine,
        "Gross profit. Subtract line 4 from line 3",
    );
    c.line(
        *line7,
        f,
        "7",
        "line7",
        Production::Combine,
        "Gross income. Add lines 5 and 6",
    );
    c.line(
        *line28,
        f,
        "28",
        "line28",
        Production::Collected,
        "Total expenses before expenses for business use of home. Add lines 8 through 27b",
    );
    c.line(
        *line29,
        f,
        "29",
        "line29",
        Production::Combine,
        "Tentative profit or (loss). Subtract line 28 from line 7",
    );
    c.line(
        *line31,
        f,
        "31",
        "line31",
        Production::Combine,
        "Net profit or (loss). Subtract line 30 from line 29.",
    );
    c
}

fn zero_scheduleclines() -> crate::tax::printed::ScheduleCLines {
    crate::tax::printed::ScheduleCLines {
        line_a_business: String::new(),
        line_b_naics: String::new(),
        line_f_accrual: false,
        line_i_1099_required: None,
        line_j_1099_filed: None,
        line1: Usd::ZERO,
        line3: Usd::ZERO,
        line5: Usd::ZERO,
        line7: Usd::ZERO,
        line28: Usd::ZERO,
        line29: Usd::ZERO,
        line31: Usd::ZERO,
    }
}

/// **ScheduleSeLines** (f1040sse) — destructured with no `..`; a new money field cannot compile.
pub fn cover_scheduleselines(l: &crate::tax::printed::ScheduleSeLines) -> Coverage {
    let crate::tax::printed::ScheduleSeLines {
        line2,
        line3,
        line4a,
        line4c,
        line6,
        line8a,
        line8d,
        line9,
        line10,
        line11,
        line12,
        line13,
    } = l;
    let f = "f1040sse";
    let mut c = Coverage::default();
    c.line(*line2, f, "2", "line2", Production::Carry,
        "Net profit or (loss) from Schedule C, line 31; and Schedule K-1 (Form 1065), box 14, code A (other than farming). See instructions for other income to report or if you are a minister or member of a religious order");
    c.line(
        *line3,
        f,
        "3",
        "line3",
        Production::Combine,
        "Combine lines 1a, 1b, and 2",
    );
    c.exception(*line4a, f, "4a", "line4a",
        "If line 3 is more than zero, multiply line 3 by 92.35% (0.9235). Otherwise, enter amount from line 3",
        "A two-branch line where BOTH branches enter a figure — the first branch is Scaled (92.35% of line 3), the second is a Carry of line 3 unchanged — so no single production transcribes it.");
    c.exception(*line4c, f, "4c", "line4c",
        "Combine lines 4a and 4b. If less than $400, stop; you don’t owe self-employment tax. Exception: If less than $400 and you had church employee income, enter -0- and continue",
        "The arithmetic is a Combine, but the sentence continues with a form-EXIT (\"stop\") and an out-of-scope church-employee branch that enters -0-, so the honest full quote carries a \"-0-\" that is not a clamp on the combined figure; Combine is rejected by rule (4) and truncating the quote to pass it would hide a real branch.");
    c.line(
        *line6,
        f,
        "6",
        "line6",
        Production::Combine,
        "Add lines 4c and 5b",
    );
    c.line(*line8a, f, "8a", "line8a", Production::Collected,
        "Total social security wages and tips (total of boxes 3 and 7 on Form(s) W-2) and railroad retirement (tier 1) compensation. If $168,600 or more, skip lines 8b through 10, and go to line 11");
    c.line(
        *line8d,
        f,
        "8d",
        "line8d",
        Production::Combine,
        "Add lines 8a, 8b, and 8c",
    );
    c.line(*line9, f, "9", "line9", Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 8d from line 7. If zero or less, enter -0- here and on line 10 and go to line 11");
    c.exception(*line10, f, "10", "line10",
        "Multiply the smaller of line 6 or line 9 by 12.4% (0.124)",
        "A composition the grammar cannot express — the operand is Bounded (\"the smaller of line 6 or line 9\") and the result is Scaled (\"by 12.4% (0.124)\"), so neither production alone states the line or its blank-ness rule.");
    c.line(
        *line11,
        f,
        "11",
        "line11",
        Production::Scaled,
        "Multiply line 6 by 2.9% (0.029)",
    );
    c.line(*line12, f, "12", "line12", Production::Combine,
        "Self-employment tax. Add lines 10 and 11. Enter here and on Schedule 2 (Form 1040), line 4, or Form 1040-SS, Part I, line 3");
    c.line(*line13, f, "13", "line13", Production::Scaled,
        "Deduction for one-half of self-employment tax. Multiply line 12 by 50% (0.50). Enter here and on Schedule 1 (Form 1040), line 15");
    c
}

fn zero_scheduleselines() -> crate::tax::printed::ScheduleSeLines {
    crate::tax::printed::ScheduleSeLines {
        line2: Usd::ZERO,
        line3: Usd::ZERO,
        line4a: Usd::ZERO,
        line4c: Usd::ZERO,
        line6: Usd::ZERO,
        line8a: Usd::ZERO,
        line8d: Usd::ZERO,
        line9: Usd::ZERO,
        line10: Usd::ZERO,
        line11: Usd::ZERO,
        line12: Usd::ZERO,
        line13: Usd::ZERO,
    }
}

/// **Schedule1Lines** (f1040s1) — destructured with no `..`; a new money field cannot compile.
pub fn cover_schedule1lines(l: &crate::tax::printed::Schedule1Lines) -> Coverage {
    let crate::tax::printed::Schedule1Lines {
        line1,
        line3,
        line7,
        line8v,
        line9,
        line10,
        line15,
        line18,
        line21,
        line26,
    } = l;
    let f = "f1040s1";
    let mut c = Coverage::default();
    c.line(*line1, f, "1", "line1", Production::Collected, "NO EXTRACT");
    c.line(*line3, f, "3", "line3", Production::Carry, "NO EXTRACT");
    c.line(*line7, f, "7", "line7", Production::Collected, "NO EXTRACT");
    c.line(
        *line8v,
        f,
        "8v",
        "line8v",
        Production::Collected,
        "NO EXTRACT",
    );
    c.line(*line9, f, "9", "line9", Production::Combine, "NO EXTRACT");
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Combine,
        "NO EXTRACT",
    );
    c.line(*line15, f, "15", "line15", Production::Carry, "NO EXTRACT");
    c.line(
        *line18,
        f,
        "18",
        "line18",
        Production::Collected,
        "NO EXTRACT",
    );
    c.exception(*line21, f, "21", "line21",
        "NO EXTRACT",
        "The §221 deduction is produced by the \"Student Loan Interest Deduction Worksheet\" in the Form 1040 instructions (a MAGI phase-out btctax implements in student_loan_deduction()); Schedule 1 line 21 itself carries no arithmetic and the worksheet is never emitted, so there is no form-side line to carry from.");
    c.line(
        *line26,
        f,
        "26",
        "line26",
        Production::Combine,
        "NO EXTRACT",
    );
    c
}

fn zero_schedule1lines() -> crate::tax::printed::Schedule1Lines {
    crate::tax::printed::Schedule1Lines {
        line1: Usd::ZERO,
        line3: Usd::ZERO,
        line7: Usd::ZERO,
        line8v: Usd::ZERO,
        line9: Usd::ZERO,
        line10: Usd::ZERO,
        line15: Usd::ZERO,
        line18: Usd::ZERO,
        line21: Usd::ZERO,
        line26: Usd::ZERO,
    }
}

/// **Schedule2Lines** (f1040s2) — destructured with no `..`; a new money field cannot compile.
pub fn cover_schedule2lines(l: &crate::tax::printed::Schedule2Lines) -> Coverage {
    let crate::tax::printed::Schedule2Lines {
        line4,
        line11,
        line12,
        line21,
    } = l;
    let f = "f1040s2";
    let mut c = Coverage::default();
    c.line(
        *line4,
        f,
        "4",
        "line4",
        Production::Carry,
        "Self-employment tax. Attach Schedule SE",
    );
    c.line(
        *line11,
        f,
        "11",
        "line11",
        Production::Carry,
        "Additional Medicare Tax. Attach Form 8959",
    );
    c.line(
        *line12,
        f,
        "12",
        "line12",
        Production::Carry,
        "Net investment income tax. Attach Form 8960",
    );
    c.line(*line21, f, "21", "line21", Production::Combine,
        "Add lines 4, 7 through 16, 18, and 19. These are your total other taxes. Enter here and on Form 1040 or 1040-SR, line 23, or Form 1040-NR, line 23b");
    c
}

fn zero_schedule2lines() -> crate::tax::printed::Schedule2Lines {
    crate::tax::printed::Schedule2Lines {
        line4: Usd::ZERO,
        line11: Usd::ZERO,
        line12: Usd::ZERO,
        line21: Usd::ZERO,
    }
}

/// **Schedule3Lines** (f1040s3) — destructured with no `..`; a new money field cannot compile.
pub fn cover_schedule3lines(l: &crate::tax::printed::Schedule3Lines) -> Coverage {
    let crate::tax::printed::Schedule3Lines {
        line1,
        line8,
        line10,
        line11,
        line15,
    } = l;
    let f = "f1040s3";
    let mut c = Coverage::default();
    c.line(
        *line1,
        f,
        "1",
        "line1",
        Production::Collected,
        "Foreign tax credit. Attach Form 1116 if required",
    );
    c.line(*line8, f, "8", "line8", Production::Combine,
        "Add lines 1 through 4, 5a, 5b, and 7. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 20");
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Collected,
        "Amount paid with request for extension to file (see instructions)",
    );
    c.exception(*line11, f, "11", "line11",
        "Excess social security and tier 1 RRTA tax withheld",
        "The §6413(c) credit is produced by the \"Excess Social Security and Tier 1 RRTA Tax Withheld\" computation in the Form 1040 general instructions (per person, max(0, Σ W-2 box 4 − 6.2% × wage base)); Schedule 3 line 11 carries no arithmetic, no source line, and the worksheet is never emitted.");
    c.line(
        *line15,
        f,
        "15",
        "line15",
        Production::Combine,
        "Add lines 9 through 12 and 14. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 31",
    );
    c
}

fn zero_schedule3lines() -> crate::tax::printed::Schedule3Lines {
    crate::tax::printed::Schedule3Lines {
        line1: Usd::ZERO,
        line8: Usd::ZERO,
        line10: Usd::ZERO,
        line11: Usd::ZERO,
        line15: Usd::ZERO,
    }
}

/// **Form8959Lines** (f8959) — destructured with no `..`; a new money field cannot compile.
pub fn cover_form8959lines(l: &crate::tax::other_taxes::Form8959Lines) -> Coverage {
    let crate::tax::other_taxes::Form8959Lines {
        line1,
        line4,
        line5,
        line6,
        line7,
        line8,
        line9,
        line10,
        line11,
        line12,
        line13,
        line18,
        line19,
        line20,
        line21,
        line22,
        line24,
    } = l;
    let f = "f8959";
    let mut c = Coverage::default();
    c.line(*line1, f, "1", "line1", Production::Collected,
        "Medicare wages and tips from Form W-2, box 5. If you have more than one Form W-2, enter the total of the amounts from box 5");
    c.line(
        *line4,
        f,
        "4",
        "line4",
        Production::Combine,
        "Add lines 1 through 3",
    );
    c.line(
        *line5,
        f,
        "5",
        "line5",
        Production::Constant,
        "Enter the following amount for your filing status:",
    );
    c.line(
        *line6,
        f,
        "6",
        "line6",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 5 from line 4. If zero or less, enter -0-",
    );
    c.line(*line7, f, "7", "line7", Production::Scaled,
        "Additional Medicare Tax on Medicare wages. Multiply line 6 by 0.9% (0.009). Enter here and go to Part II");
    c.line(*line8, f, "8", "line8", Production::Clamped(Polarity::FloorAtZero),
        "Self-employment income from Schedule SE (Form 1040), Part I, line 6. If you had a loss, enter -0-");
    c.line(
        *line9,
        f,
        "9",
        "line9",
        Production::Constant,
        "Enter the following amount for your filing status:",
    );
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Carry,
        "Enter the amount from line 4",
    );
    c.line(
        *line11,
        f,
        "11",
        "line11",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 10 from line 9. If zero or less, enter -0-",
    );
    c.line(
        *line12,
        f,
        "12",
        "line12",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 11 from line 8. If zero or less, enter -0-",
    );
    c.line(*line13, f, "13", "line13", Production::Scaled,
        "Additional Medicare Tax on self-employment income. Multiply line 12 by 0.9% (0.009). Enter here and go to Part III");
    c.line(*line18, f, "18", "line18", Production::Combine,
        "Add lines 7, 13, and 17. Also include this amount on Schedule 2 (Form 1040), line 11 (Form 1040-SS filers, see instructions), and go to Part V");
    c.line(*line19, f, "19", "line19", Production::Collected,
        "Medicare tax withheld from Form W-2, box 6. If you have more than one Form W-2, enter the total of the amounts from box 6");
    c.line(
        *line20,
        f,
        "20",
        "line20",
        Production::Carry,
        "Enter the amount from line 1",
    );
    c.line(*line21, f, "21", "line21", Production::Scaled,
        "Multiply line 20 by 1.45% (0.0145). This is your regular Medicare tax withholding on Medicare wages");
    c.line(*line22, f, "22", "line22", Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 21 from line 19. If zero or less, enter -0-. This is your Additional Medicare Tax withholding on Medicare wages");
    c.line(*line24, f, "24", "line24", Production::Combine,
        "Total Additional Medicare Tax withholding. Add lines 22 and 23. Also include this amount with federal income tax withholding on Form 1040, 1040-SR, or 1040-NR, line 25c (Form 1040-SS filers, see instructions)");
    c
}

fn zero_form8959lines() -> crate::tax::other_taxes::Form8959Lines {
    crate::tax::other_taxes::Form8959Lines {
        line1: Usd::ZERO,
        line4: Usd::ZERO,
        line5: Usd::ZERO,
        line6: Usd::ZERO,
        line7: Usd::ZERO,
        line8: Usd::ZERO,
        line9: Usd::ZERO,
        line10: Usd::ZERO,
        line11: Usd::ZERO,
        line12: Usd::ZERO,
        line13: Usd::ZERO,
        line18: Usd::ZERO,
        line19: Usd::ZERO,
        line20: Usd::ZERO,
        line21: Usd::ZERO,
        line22: Usd::ZERO,
        line24: Usd::ZERO,
    }
}

/// **Form8960Lines** (f8960) — destructured with no `..`; a new money field cannot compile.
pub fn cover_form8960lines(l: &crate::tax::other_taxes::Form8960Lines) -> Coverage {
    let crate::tax::other_taxes::Form8960Lines {
        line1,
        line2,
        line5a,
        line5d,
        line7,
        line8,
        line9d,
        line11,
        line12,
        line13,
        line14,
        line15,
        line16,
        line17,
    } = l;
    let f = "f8960";
    let mut c = Coverage::default();
    c.line(
        *line1,
        f,
        "1",
        "line1",
        Production::Carry,
        "Taxable interest (see instructions)",
    );
    c.line(
        *line2,
        f,
        "2",
        "line2",
        Production::Carry,
        "Ordinary dividends (see instructions)",
    );
    c.line(
        *line5a,
        f,
        "5a",
        "line5a",
        Production::Carry,
        "Net gain or loss from disposition of property (see instructions)",
    );
    c.line(
        *line5d,
        f,
        "5d",
        "line5d",
        Production::Combine,
        "Combine lines 5a through 5c",
    );
    c.line(
        *line7,
        f,
        "7",
        "line7",
        Production::Collected,
        "Other modifications to investment income (see instructions)",
    );
    c.line(
        *line8,
        f,
        "8",
        "line8",
        Production::Combine,
        "Total investment income. Combine lines 1, 2, 3, 4c, 5d, 6, and 7",
    );
    c.line(
        *line9d,
        f,
        "9d",
        "line9d",
        Production::Combine,
        "Add lines 9a, 9b, and 9c",
    );
    c.line(
        *line11,
        f,
        "11",
        "line11",
        Production::Combine,
        "Total deductions and modifications. Add lines 9d and 10",
    );
    c.line(*line12, f, "12", "line12", Production::Clamped(Polarity::FloorAtZero),
        "Net investment income. Subtract Part II, line 11, from Part I, line 8. Individuals, complete lines 13–17. Estates and trusts, complete lines 18a–21. If zero or less, enter -0-");
    c.line(
        *line13,
        f,
        "13",
        "line13",
        Production::Carry,
        "Modified adjusted gross income (see instructions)",
    );
    c.line(
        *line14,
        f,
        "14",
        "line14",
        Production::Constant,
        "Threshold based on filing status (see instructions)",
    );
    c.line(
        *line15,
        f,
        "15",
        "line15",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 14 from line 13. If zero or less, enter -0-",
    );
    c.line(
        *line16,
        f,
        "16",
        "line16",
        Production::Bounded,
        "Enter the smaller of line 12 or line 15",
    );
    c.line(*line17, f, "17", "line17", Production::Scaled,
        "Net investment income tax for individuals. Multiply line 16 by 3.8% (0.038). Enter here and include on your tax return (see instructions)");
    c
}

fn zero_form8960lines() -> crate::tax::other_taxes::Form8960Lines {
    crate::tax::other_taxes::Form8960Lines {
        line1: Usd::ZERO,
        line2: Usd::ZERO,
        line5a: Usd::ZERO,
        line5d: Usd::ZERO,
        line7: Usd::ZERO,
        line8: Usd::ZERO,
        line9d: Usd::ZERO,
        line11: Usd::ZERO,
        line12: Usd::ZERO,
        line13: Usd::ZERO,
        line14: Usd::ZERO,
        line15: Usd::ZERO,
        line16: Usd::ZERO,
        line17: Usd::ZERO,
    }
}

/// **ScheduleDLines** (f1040sd) — destructured with no `..`; a new money field cannot compile.
pub fn cover_scheduledlines(l: &crate::tax::printed::ScheduleDLines) -> Coverage {
    let crate::tax::printed::ScheduleDLines {
        line3_d,
        line3_e,
        line3_h,
        line6,
        line7,
        line10_d,
        line10_e,
        line10_h,
        line13,
        line14,
        line15,
        line16,
        routing: _,
    } = l;
    let f = "f1040sd";
    let mut c = Coverage::default();
    c.line(
        *line3_d,
        f,
        "3(d)",
        "line3_d",
        Production::Carry,
        "Totals for all transactions reported on Form(s) 8949 with Box C checked",
    );
    c.line(
        *line3_e,
        f,
        "3(e)",
        "line3_e",
        Production::Carry,
        "Totals for all transactions reported on Form(s) 8949 with Box C checked",
    );
    c.line(
        *line3_h,
        f,
        "3(h)",
        "line3_h",
        Production::Carry,
        "Totals for all transactions reported on Form(s) 8949 with Box C checked",
    );
    c.line(*line6, f, "6", "line6", Production::Collected,
        "Short-term capital loss carryover. Enter the amount, if any, from line 8 of your Capital Loss Carryover Worksheet in the instructions");
    c.line(
        *line7,
        f,
        "7",
        "line7",
        Production::Combine,
        "Net short-term capital gain or (loss). Combine lines 1a through 6 in column (h).",
    );
    c.line(
        *line10_d,
        f,
        "10(d)",
        "line10_d",
        Production::Carry,
        "Totals for all transactions reported on Form(s) 8949 with Box F checked",
    );
    c.line(
        *line10_e,
        f,
        "10(e)",
        "line10_e",
        Production::Carry,
        "Totals for all transactions reported on Form(s) 8949 with Box F checked",
    );
    c.line(
        *line10_h,
        f,
        "10(h)",
        "line10_h",
        Production::Carry,
        "Totals for all transactions reported on Form(s) 8949 with Box F checked",
    );
    c.line(
        *line13,
        f,
        "13",
        "line13",
        Production::Collected,
        "Capital gain distributions. See the instructions",
    );
    c.line(*line14, f, "14", "line14", Production::Collected,
        "Long-term capital loss carryover. Enter the amount, if any, from line 13 of your Capital Loss Carryover Worksheet in the instructions");
    c.line(
        *line15,
        f,
        "15",
        "line15",
        Production::Combine,
        "Net long-term capital gain or (loss). Combine lines 8a through 14 in column (h).",
    );
    c.line(
        *line16,
        f,
        "16",
        "line16",
        Production::Combine,
        "Combine lines 7 and 15 and enter the result",
    );
    c
}

fn zero_scheduledlines() -> crate::tax::printed::ScheduleDLines {
    crate::tax::printed::ScheduleDLines {
        line3_d: Usd::ZERO,
        line3_e: Usd::ZERO,
        line3_h: Usd::ZERO,
        line6: Usd::ZERO,
        line7: Usd::ZERO,
        line10_d: Usd::ZERO,
        line10_e: Usd::ZERO,
        line10_h: Usd::ZERO,
        line13: Usd::ZERO,
        line14: Usd::ZERO,
        line15: Usd::ZERO,
        line16: Usd::ZERO,
        routing: crate::tax::printed::ScheduleDRouting::BothGains,
    }
}

/// Every covered form, in one place. Grows one entry per form as coverage lands.
///
/// ★ The values are irrelevant — only the TABLE is extracted. The instances exist so the exhaustive
/// destructures run, which is where the compile-time guarantee lives.
pub fn all() -> Coverage {
    let mut c = Coverage::default();
    c.0.extend(cover_f8995(&zero_f8995()).0);
    c.0.extend(cover_scheduleblines(&zero_scheduleblines()).0);
    c.0.extend(cover_schedulealines(&zero_schedulealines()).0);
    c.0.extend(cover_scheduleclines(&zero_scheduleclines()).0);
    c.0.extend(cover_scheduleselines(&zero_scheduleselines()).0);
    c.0.extend(cover_schedule1lines(&zero_schedule1lines()).0);
    c.0.extend(cover_schedule2lines(&zero_schedule2lines()).0);
    c.0.extend(cover_schedule3lines(&zero_schedule3lines()).0);
    c.0.extend(cover_form8959lines(&zero_form8959lines()).0);
    c.0.extend(cover_form8960lines(&zero_form8960lines()).0);
    c.0.extend(cover_scheduledlines(&zero_scheduledlines()).0);
    c
}
