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
//! ★★ **The `_` rule — and what it does NOT give you.** `classifier.rs` deliberately PERMITS `_` on
//! `Usd` leaves, which is why it is blind to all 64 fabrication sites. Here the destructure NAMES every
//! money field, and the named value is consumed by [`Coverage::line`].
//!
//! ★★★ **But the guarantee is "a new money field must be NAMED", not "must be CLASSIFIED"** — review
//! r6 (I-4) proved it: changing `line4,` to `line4: _,` and deleting its `c.line(..)` compiles, drops
//! Schedule 2 line 4 (self-employment tax, a real printed line) from the table, and the only visible
//! difference is a row count nothing asserts on. `#![deny(unused_variables)]` is satisfied by `_`.
//! An earlier version of this comment claimed `_` was FORBIDDEN on money. **It is not.**
//!
//! So the compiler forces *a human must EDIT this file*, and the minimum edit that silences it is two
//! mechanical lines. That is the same honest limit `classifier.rs` records for itself — the residual
//! evasion is grep-able review residue, not a compile error — and it is worth having, because every
//! recurrence of this class began with a field nobody looked at. Filed as §G-27.
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

/// The year every currently-covered form is quoted from. A TY2025+ form sets its own.
pub const DEFAULT_ROW_YEAR: &str = "2024";

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
    /// The tax year of the extract this line's instruction is quoted from.
    ///
    /// ★ Per-row, not a module constant: a form is year-shaped. Schedule 1-A exists only for TY2025+,
    /// so a global year would send its quotes to an extract that does not exist. r4 buildability I-2.
    pub year: &'static str,
    /// The form's own line label, e.g. `"16"` — for humans and for the map cross-check.
    ///
    /// ★ Owned, not `&'static str`: a type serving several form lines part-qualifies its label at the
    /// call site (`"I-1(d)"` vs `"II-1(d)"` on Form 8949), which cannot be a literal.
    pub line: String,
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
        line: &str,
        field: &'static str,
        production: Production,
        instruction: &'static str,
    ) {
        self.0.push(LineCoverage {
            form,
            year: DEFAULT_ROW_YEAR,
            line: line.to_string(),
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
        line: &str,
        field: &'static str,
        instruction: &'static str,
        reason: &'static str,
    ) {
        self.0.push(LineCoverage {
            form,
            year: DEFAULT_ROW_YEAR,
            line: line.to_string(),
            field,
            production: Production::Exception,
            instruction,
            reason: Some(reason),
        });
    }
}

/// §G-28/B1b — Form 8995-A **Part II** (lines 2-16), column A.
///
/// ★★ These 15 lines and Part III's 10 shipped OUTSIDE this table, and the module's blast-radius
/// guarantee did not fire: an omitted field on a struct already destructured here is a compile error,
/// but a whole NEW struct produces no error at all. The consequence was that every Part II and Part III
/// doc comment went unverified against `design/forms/extract/f8995a--2024.txt` — i.e. the one instrument
/// that exists to catch a Form 6251-line-33-class slip did not see the form it was most needed on.
pub fn cover_form8995apartii(p: &crate::tax::qbi_a::Form8995APartIi) -> Coverage {
    let crate::tax::qbi_a::Form8995APartIi {
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
    } = p;
    let f = "f8995a";
    let mut c = Coverage::default();
    c.line(
        *line2,
        f,
        "2",
        "line2",
        Production::Carry,
        "Qualified business income from the trade, business, or aggregation. See instructions",
    );
    c.line(*line3, f, "3", "line3", Production::Scaled,
        "Multiply line 2 by 20% (0.20). If your taxable income is $191,950 or less ($383,900 if married filing jointly), skip lines 4 through 12 and enter the amount from line 3 on line 13");
    c.line(
        *line4,
        f,
        "4",
        "line4",
        Production::Collected,
        "Allocable share of W-2 wages from the trade, business, or aggregation",
    );
    c.line(
        *line5,
        f,
        "5",
        "line5",
        Production::Scaled,
        "Multiply line 4 by 50% (0.50)",
    );
    c.line(
        *line6,
        f,
        "6",
        "line6",
        Production::Scaled,
        "Multiply line 4 by 25% (0.25)",
    );
    c.line(*line7, f, "7", "line7", Production::Collected,
        "Allocable share of the unadjusted basis immediately after acquisition (UBIA) of all qualified property");
    c.line(
        *line8,
        f,
        "8",
        "line8",
        Production::Scaled,
        "Multiply line 7 by 2.5% (0.025)",
    );
    c.line(
        *line9,
        f,
        "9",
        "line9",
        Production::Combine,
        "Add lines 6 and 8",
    );
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Bounded,
        "Enter the greater of line 5 or line 9",
    );
    c.line(*line11, f, "11", "line11", Production::Bounded,
        "W-2 wage and UBIA of qualified property limitation. Enter the smaller of line 3 or line 10");
    // ★★ `Carry`, NOT an exception. Its own definition is *"Enter the amount from line N" — blank when
    //    the source line is blank*, and that is this line verbatim: outside the §199A phase-in range
    //    there is no line 26, so the cell stays BLANK (the emitter uses `push_money_opt`). Filing it as
    //    an exception cost the ratchet a unit of unearned slack, and filed a FOLLOW-UP proposing a
    //    production the grammar already has — "I conclude from not having looked", on the one file
    //    that documents its productions three lines apart.
    c.line(
        line12.unwrap_or(Usd::ZERO),
        f,
        "12",
        "line12",
        Production::Carry,
        "Phased-in reduction. Enter the amount from line 26, if any",
    );
    c.line(*line13, f, "13", "line13", Production::Bounded,
        "Qualified business income deduction before patron reduction. Enter the greater of line 11 or line 12");
    // ★★ Also `Carry` — "Enter the amount from Schedule D (Form 8995-A), line 6". Its source line is
    //    always blank here, because btctax fills no Schedule D: a cooperative patron REFUSES
    //    (`RefuseReason::CooperativePatron`) rather than file with this line empty.
    c.line(line14.unwrap_or(Usd::ZERO), f, "14", "line14", Production::Carry,
        "Patron reduction. Enter the amount from Schedule D (Form 8995-A), line 6, if any. See instructions");
    c.line(
        *line15,
        f,
        "15",
        "line15",
        Production::Combine,
        "Qualified business income component. Subtract line 14 from line 13",
    );
    c.line(
        *line16,
        f,
        "16",
        "line16",
        Production::Combine,
        "Total qualified business income component. Add all amounts reported on line 15",
    );
    c
}

/// §G-28/B1b — Form 8995-A **Part III** (lines 17-26), the phased-in reduction.
///
/// ★ Line 24 is a PERCENTAGE, not a dollar amount, and is the one line here that is not `Usd`. It is
/// covered through its printed form (ratio × 100) so the table's shape is uniform; the note records it.
pub fn cover_form8995apartiii(p: &crate::tax::qbi_a::Form8995APartIii) -> Coverage {
    let crate::tax::qbi_a::Form8995APartIii {
        line17,
        line18,
        line19,
        line20,
        line21,
        line22,
        line23,
        line24_ratio,
        line25,
        line26,
    } = p;
    let f = "f8995a";
    let mut c = Coverage::default();
    c.line(
        *line17,
        f,
        "17",
        "line17",
        Production::Carry,
        "Enter the amounts from line 3",
    );
    c.line(
        *line18,
        f,
        "18",
        "line18",
        Production::Carry,
        "Enter the amounts from line 10",
    );
    c.line(
        *line19,
        f,
        "19",
        "line19",
        Production::Combine,
        "Subtract line 18 from line 17",
    );
    c.line(
        *line20,
        f,
        "20",
        "line20",
        Production::Carry,
        "Taxable income before qualified business income deduction",
    );
    c.line(
        *line21,
        f,
        "21",
        "line21",
        Production::Constant,
        "Threshold. Enter $191,950 ($383,900 if married filing jointly)",
    );
    c.line(
        *line22,
        f,
        "22",
        "line22",
        Production::Combine,
        "Subtract line 21 from line 20",
    );
    c.line(
        *line23,
        f,
        "23",
        "line23",
        Production::Constant,
        "Phase-in range. Enter $50,000 ($100,000 if married filing jointly)",
    );
    c.exception(*line24_ratio, f, "24", "line24_ratio",
        "Phase-in percentage. Divide line 22 by line 23",
        "A PERCENTAGE, not a dollar amount, and the only non-money printed line on this form. Stored as the RATIO (line 22 / line 23) because that is what line 25 multiplies by; the emitter scales it by 100 once, at the single point where it becomes ink, so the arithmetic and the printing cannot disagree. Whole-dollar rounding would collapse every percentage to 0 or 1.");
    c.line(
        *line25,
        f,
        "25",
        "line25",
        Production::Scaled,
        "Total phase-in reduction. Multiply line 19 by line 24",
    );
    c.line(*line26, f, "26", "line26", Production::Combine,
        "Qualified business income after phase-in reduction. Subtract line 25 from line 17. Enter this amount here and on line 12, for the corresponding trade or business");
    c
}

/// §G-28/B1a — **Form 8995-A Part IV**. Every line, in the form's own numbering.
///
/// ★ Part IV is Form 8995's lines 5-17 with a DPAD line inserted, and `qbi_a` carries the written
/// equivalence proof plus the KAT that pins the branch where it would break. The rows below quote
/// **8995-A's own sentences**, which differ in wording from 8995's at two clamps even though they
/// never differ in value — so these are transcribed from `f8995a--2024.txt`, not copied from the
/// sibling coverage fn.
pub fn cover_form8995apartiv(p: &crate::tax::qbi_a::Form8995APartIv) -> Coverage {
    let crate::tax::qbi_a::Form8995APartIv {
        line27,
        line28,
        line29,
        line30,
        line31,
        line32,
        line33,
        line34,
        line35,
        line36,
        line37,
        line38,
        line39,
        line40,
    } = p;
    let f = "f8995a";
    let mut c = Coverage::default();
    c.line(*line27, f, "27", "line27", Production::Carry,
        "Total qualified business income component from all qualified trades, businesses, or aggregations. Enter the amount from line 16");
    c.line(*line28, f, "28", "line28", Production::Collected,
        "Qualified REIT dividends and publicly traded partnership (PTP) income or (loss). See instructions");
    c.line(
        *line29,
        f,
        "29",
        "line29",
        Production::Collected,
        "Qualified REIT dividends and PTP (loss) carryforward from prior years",
    );
    c.line(*line30, f, "30", "line30", Production::Clamped(Polarity::FloorAtZero),
        "Total qualified REIT dividends and PTP income. Combine lines 28 and 29. If less than zero, enter -0-");
    c.line(
        *line31,
        f,
        "31",
        "line31",
        Production::Scaled,
        "REIT and PTP component. Multiply line 30 by 20% (0.20)",
    );
    c.line(
        *line32,
        f,
        "32",
        "line32",
        Production::Combine,
        "Qualified business income deduction before the income limitation. Add lines 27 and 31",
    );
    c.line(
        *line33,
        f,
        "33",
        "line33",
        Production::Carry,
        "Taxable income before qualified business income deduction",
    );
    c.line(*line34, f, "34", "line34", Production::Carry,
        "Enter your net capital gain, if any, increased by any qualified dividends (see instructions)");
    c.line(
        *line35,
        f,
        "35",
        "line35",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 34 from line 33. If zero or less, enter -0-",
    );
    c.line(
        *line36,
        f,
        "36",
        "line36",
        Production::Scaled,
        "Income limitation. Multiply line 35 by 20% (0.20)",
    );
    c.line(*line37, f, "37", "line37", Production::Bounded,
        "Qualified business income deduction before the domestic production activities deduction (DPAD) under section 199A(g). Enter the smaller of line 32 or line 36");
    // ★★ The DPAD line is the ONE Part IV line btctax never writes. It is a CONDITIONAL entry with no
    //    `-0-` clause — "Don't enter more than line 33 minus line 37" presumes an amount a cooperative
    //    allocated to the filer — and btctax fills no Schedule D (Form 8995-A), so none exists. It is
    //    an `Option<Usd>` that is always `None`, and the emitter writes NOTHING through it.
    c.exception(line38.unwrap_or(Usd::ZERO), f, "38", "line38",
        "DPAD under section 199A(g) allocated from an agricultural or horticultural cooperative. Don’t enter more than line 33 minus line 37",
        "NEVER WRITTEN. A conditional entry with no \"-0-\" clause, whose condition requires a Schedule D (Form 8995-A) allocation btctax does not fill. `Option<Usd>`, always `None`; the emitter uses `push_money_opt` so the cell stays BLANK. A printed 0 would swear the filer received an allocation of zero.");
    c.line(
        *line39,
        f,
        "39",
        "line39",
        Production::Combine,
        "Total qualified business income deduction. Add lines 37 and 38",
    );
    c.line(*line40, f, "40", "line40", Production::Clamped(Polarity::CeilAtZero),
        "Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 28 and 29. If zero or greater, enter -0-");
    c
}

/// **Form 8995** — the whole struct, destructured with no `..`.
///
/// Chosen as the first covered form because it exercises six of the eight productions AND both clamp
/// polarities, so the mechanism is proved on a hard case rather than an easy one.
pub fn cover_form8995lines(l: &crate::tax::qbi::Form8995Lines) -> Coverage {
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
/// *pattern does not mention field* in [`cover_form8995lines`].
fn zero_form8995lines() -> crate::tax::qbi::Form8995Lines {
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
        part1_rows,
        line2,
        line4,
        part2_rows,
        line6,
        foreign_accounts_7a: _,
        foreign_trust_8: _,
        line7b_countries: _,
        fbar_filing_required: _,
    } = l;
    let f = "f1040sb";
    let mut c = Coverage::default();
    // ★ NESTED MONEY. `ScheduleBRow.amount` is a printed figure on a type one level down, and the
    //   same type serves TWO different form lines — Part I line 1 (interest) and Part II line 5
    //   (dividends) — so the context travels as a parameter rather than being baked into the type.
    //   `zero_schedulebLines()` seeds one row in each Vec so this is never vacuous.
    for r in part1_rows {
        c.0.extend(
            cover_schedulebrow(
                r,
                "1",
                "List name of payer. If any interest is from a seller-financed mortgage and the",
            )
            .0,
        );
    }
    for r in part2_rows {
        c.0.extend(cover_schedulebrow(r, "5", "List name of payer").0);
    }
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
        // ★ One representative row each, so the nested coverage above is never vacuous — an empty
        // Vec would silently emit no rows and the checker would report success over nothing.
        part1_rows: vec![zero_schedulebrow()],
        line2: Usd::ZERO,
        line4: Usd::ZERO,
        part2_rows: vec![zero_schedulebrow()],
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
    c.line(
        *line1,
        f,
        "1",
        "line1",
        Production::Collected,
        "Taxable refunds, credits, or offsets of state and local income taxes",
    );
    c.line(
        *line3,
        f,
        "3",
        "line3",
        Production::Carry,
        "Business income or (loss). Attach Schedule C",
    );
    c.line(
        *line7,
        f,
        "7",
        "line7",
        Production::Collected,
        "Unemployment compensation",
    );
    c.line(
        *line8v,
        f,
        "8v",
        "line8v",
        Production::Collected,
        "Digital assets received as ordinary income not reported elsewhere. See",
    );
    c.line(
        *line9,
        f,
        "9",
        "line9",
        Production::Combine,
        "Total other income. Add lines 8a through 8z",
    );
    c.line(
        *line10,
        f,
        "10",
        "line10",
        Production::Combine,
        "Combine lines 1 through 7 and 9. This is your additional income.",
    );
    c.line(
        *line15,
        f,
        "15",
        "line15",
        Production::Carry,
        "Deductible part of self-employment tax. Attach Schedule SE",
    );
    c.line(
        *line18,
        f,
        "18",
        "line18",
        Production::Collected,
        "Penalty on early withdrawal of savings",
    );
    c.exception(*line21, f, "21", "line21",
        "Student loan interest deduction",
        "The §221 deduction is produced by the \"Student Loan Interest Deduction Worksheet\" in the Form 1040 instructions (a MAGI phase-out btctax implements in student_loan_deduction()); Schedule 1 line 21 itself carries no arithmetic and the worksheet is never emitted, so there is no form-side line to carry from.");
    c.line(
        *line26,
        f,
        "26",
        "line26",
        Production::Combine,
        "Add lines 11 through 23 and 25. These are your adjustments to income.",
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
        "The §6413(c) credit is produced by the \"Excess Social Security and Tier 1 RRTA Tax Withheld\" \
         computation in the Form 1040 general instructions; Schedule 3 line 11 itself carries no \
         arithmetic, no source line, and the worksheet is never emitted. ★ Per person, never pooled, \
         and it REQUIRES MORE THAN ONE EMPLOYER: Σ over employers of min(that employer's Σ box 4, MAX), \
         less MAX, floored at zero — where MAX = 6.2% × the wage base and employers are identified by \
         CANONICALIZED EIN. This reason previously recorded max(0, Σ box 4 − MAX), which omitted both \
         conditions and was a live understatement of tax.");
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
        line1a_d,
        line1a_e,
        line1a_h,
        line3_d,
        line3_e,
        line3_h,
        line6,
        line7,
        line8a_d,
        line8a_e,
        line8a_h,
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
    // §G-28/B4 — Schedule D's own totals-without-Form-8949 lines.
    c.line(*line1a_d, f, "1a(d)", "line1a_d", Production::Collected, "Totals for all short-term transactions reported on Form 1099-B for which basis was reported to the IRS and for which you have no adjustments (see instructions). However, if you choose to report all these transactions on Form 8949, leave this line blank and go to line 1b");
    c.line(*line1a_e, f, "1a(e)", "line1a_e", Production::Collected, "Totals for all short-term transactions reported on Form 1099-B for which basis was reported to the IRS and for which you have no adjustments (see instructions). However, if you choose to report all these transactions on Form 8949, leave this line blank and go to line 1b");
    // ★ Column (h) quotes the LINE's own sentence, like lines 3 and 10 beside it — the column header
    //   ("Subtract column (e) from column (d)…") is a four-row wrap in the text layer and is carried
    //   by the `1a(h)` LABEL, not by the quote.
    c.line(*line1a_h, f, "1a(h)", "line1a_h", Production::Combine, "Totals for all short-term transactions reported on Form 1099-B for which basis was reported to the IRS and for which you have no adjustments (see instructions). However, if you choose to report all these transactions on Form 8949, leave this line blank and go to line 1b");
    c.line(*line8a_d, f, "8a(d)", "line8a_d", Production::Collected, "Totals for all long-term transactions reported on Form 1099-B for which basis was reported to the IRS and for which you have no adjustments (see instructions). However, if you choose to report all these transactions on Form 8949, leave this line blank and go to line 8b");
    c.line(*line8a_e, f, "8a(e)", "line8a_e", Production::Collected, "Totals for all long-term transactions reported on Form 1099-B for which basis was reported to the IRS and for which you have no adjustments (see instructions). However, if you choose to report all these transactions on Form 8949, leave this line blank and go to line 8b");
    c.line(*line8a_h, f, "8a(h)", "line8a_h", Production::Combine, "Totals for all long-term transactions reported on Form 1099-B for which basis was reported to the IRS and for which you have no adjustments (see instructions). However, if you choose to report all these transactions on Form 8949, leave this line blank and go to line 8b");
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
        line1a_d: Usd::ZERO,
        line1a_e: Usd::ZERO,
        line1a_h: Usd::ZERO,
        line8a_d: Usd::ZERO,
        line8a_e: Usd::ZERO,
        line8a_h: Usd::ZERO,
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

/// **ScheduleBRow** — the nested payer row. `line`/`instruction` come from the caller because the
/// same type prints on Schedule B line 1 (interest) and line 5 (dividends).
pub fn cover_schedulebrow(
    r: &crate::tax::printed::ScheduleBRow,
    line: &'static str,
    instruction: &'static str,
) -> Coverage {
    let crate::tax::printed::ScheduleBRow {
        payer: _, // not money
        amount,
    } = r;
    let mut c = Coverage::default();
    c.line(
        *amount,
        "f1040sb",
        line,
        "amount",
        Production::Collected,
        instruction,
    );
    c
}

fn zero_schedulebrow() -> crate::tax::printed::ScheduleBRow {
    crate::tax::printed::ScheduleBRow {
        payer: String::new(),
        amount: Usd::ZERO,
    }
}

/// **ScheduleDRouting** — money inside an ENUM VARIANT.
///
/// ★ Matched exhaustively with NO `_` arm, so a new variant carrying money is a non-exhaustive-match
/// COMPILE ERROR. Same guarantee as the struct destructures, one type-constructor over.
pub fn cover_scheduledrouting(r: &crate::tax::printed::ScheduleDRouting) -> Coverage {
    use crate::tax::printed::ScheduleDRouting as R;
    let mut c = Coverage::default();
    match r {
        R::BothGains => {}
        R::ShortGainLongLoss { line22_yes: _ } => {}
        R::NetLoss {
            line21,
            line22_yes: _,
        } => c.line(
            *line21,
            "f1040sd",
            "21",
            "line21",
            Production::Bounded,
            "If line 16 is a loss, enter here and on Form 1040, 1040-SR, or 1040-NR, line 7, the smaller of:",
        ),
        R::Zero { line22_yes: _ } => {}
    }
    c
}

/// **Form1040Lines** — the main form. Destructured with no `..`.
///
/// ★ Field keys are STRUCT-QUALIFIED (`Form1040Lines.lineN`) because `Form1040Income` re-declares eleven of
/// `Form1040Lines`' lines with identical names on the identical form — rule (5) keys on
/// `(form, line, field)` and would otherwise report each as covered twice.
pub fn cover_form1040lines(l: &crate::tax::printed::Form1040Lines) -> Coverage {
    let crate::tax::printed::Form1040Lines {
        line1z,
        line1a,
        line2a,
        line2b,
        line3a,
        line3b,
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
        line18,
        line19,
        line20,
        line21,
        line22,
        line23,
        line24,
        line25a,
        line25b,
        line25c,
        line25d,
        line26,
        line31,
        line32,
        line33,
        line34,
        line37,
        digital_asset_yes: _,
    } = l;
    let mut c = Coverage::default();
    c.line(
        *line1z,
        "f1040",
        "1z",
        "Form1040Lines.line1z",
        Production::Combine,
        "Add lines 1a through 1h",
    );
    c.line(
        *line1a,
        "f1040",
        "1a",
        "Form1040Lines.line1a",
        Production::Collected,
        "Total amount from Form(s) W-2, box 1 (see instructions)",
    );
    c.line(
        *line2a,
        "f1040",
        "2a",
        "Form1040Lines.line2a",
        Production::Collected,
        "Tax-exempt interest",
    );
    c.line(
        *line2b,
        "f1040",
        "2b",
        "Form1040Lines.line2b",
        Production::Collected,
        "Taxable interest",
    );
    c.line(
        *line3a,
        "f1040",
        "3a",
        "Form1040Lines.line3a",
        Production::Collected,
        "Qualified dividends",
    );
    c.line(
        *line3b,
        "f1040",
        "3b",
        "Form1040Lines.line3b",
        Production::Collected,
        "Ordinary dividends",
    );
    c.line(
        *line7,
        "f1040",
        "7",
        "Form1040Lines.line7",
        Production::Carry,
        "Capital gain or (loss). Attach Schedule D if required. If not required, check here",
    );
    c.line(
        *line8,
        "f1040",
        "8",
        "Form1040Lines.line8",
        Production::Carry,
        "Additional income from Schedule 1, line 10",
    );
    c.line(
        *line9,
        "f1040",
        "9",
        "Form1040Lines.line9",
        Production::Combine,
        "Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income",
    );
    c.line(
        *line10,
        "f1040",
        "10",
        "Form1040Lines.line10",
        Production::Carry,
        "Adjustments to income from Schedule 1, line 26",
    );
    c.line(
        *line11,
        "f1040",
        "11",
        "Form1040Lines.line11",
        Production::Combine,
        "Subtract line 10 from line 9. This is your adjusted gross income",
    );
    c.exception(*line12, "f1040", "12", "Form1040Lines.line12",
        "Standard deduction or itemized deductions (from Schedule A)",
        "Two branches that BOTH enter a figure, and the line states neither an operation nor a source: itemizing is a Carry of Schedule A's printed line 17, not itemizing is a Constant the form prints in its own left margin (\"• Single or Married filing separately, $14,600\"). No single production states the figure or its blank-ness rule — the f1040sse:4a shape. The instructions adjudicate it as \"the larger of your itemized deductions or standard deduction\", and there is no larger-of production (Bounded is \"the smaller of\").");
    c.line(
        *line13,
        "f1040",
        "13",
        "Form1040Lines.line13",
        Production::Carry,
        "Qualified business income deduction from Form 8995 or Form 8995-A",
    );
    c.line(
        *line14,
        "f1040",
        "14",
        "Form1040Lines.line14",
        Production::Combine,
        "Add lines 12 and 13",
    );
    c.line(
        *line15,
        "f1040",
        "15",
        "Form1040Lines.line15",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 14 from line 11. If zero or less, enter -0-. This is your taxable income",
    );
    c.exception(*line16, "f1040", "16", "Form1040Lines.line16",
        "Tax (see instructions). Check if any from Form(s):",
        "The figure is produced by the Tax Table, the Tax Computation Worksheet, or the Qualified Dividends and Capital Gain Tax Worksheet in the Form 1040 instructions (i1040gi \"Line 16\": \"Figure the tax using one of the methods described later\"). Line 16 itself carries no arithmetic and names no source line, and none of those worksheets is ever emitted — so there is no form-side line to carry from. Same shape as the f1040s1:21 and f1040s3:11 exceptions.");
    c.line(
        *line17,
        "f1040",
        "17",
        "Form1040Lines.line17",
        Production::Carry,
        "Amount from Schedule 2, line 3",
    );
    c.line(
        *line18,
        "f1040",
        "18",
        "Form1040Lines.line18",
        Production::Combine,
        "Add lines 16 and 17",
    );
    c.line(
        *line19,
        "f1040",
        "19",
        "Form1040Lines.line19",
        Production::Carry,
        "Child tax credit or credit for other dependents from Schedule 8812",
    );
    c.line(
        *line20,
        "f1040",
        "20",
        "Form1040Lines.line20",
        Production::Carry,
        "Amount from Schedule 3, line 8",
    );
    c.line(
        *line21,
        "f1040",
        "21",
        "Form1040Lines.line21",
        Production::Combine,
        "Add lines 19 and 20",
    );
    c.line(
        *line22,
        "f1040",
        "22",
        "Form1040Lines.line22",
        Production::Clamped(Polarity::FloorAtZero),
        "Subtract line 21 from line 18. If zero or less, enter -0-",
    );
    c.line(
        *line23,
        "f1040",
        "23",
        "Form1040Lines.line23",
        Production::Carry,
        "Other taxes, including self-employment tax, from Schedule 2, line 21",
    );
    c.line(
        *line24,
        "f1040",
        "24",
        "Form1040Lines.line24",
        Production::Combine,
        "Add lines 22 and 23. This is your total tax",
    );
    c.line(
        *line25a,
        "f1040",
        "25a",
        "Form1040Lines.line25a",
        Production::Collected,
        "Federal income tax withheld from: a Form(s) W-2",
    );
    c.line(
        *line25b,
        "f1040",
        "25b",
        "Form1040Lines.line25b",
        Production::Collected,
        "Form(s) 1099",
    );
    c.line(
        *line25c,
        "f1040",
        "25c",
        "Form1040Lines.line25c",
        Production::Collected,
        "Other forms (see instructions)",
    );
    c.line(
        *line25d,
        "f1040",
        "25d",
        "Form1040Lines.line25d",
        Production::Combine,
        "Add lines 25a through 25c",
    );
    c.line(
        *line26,
        "f1040",
        "26",
        "Form1040Lines.line26",
        Production::Collected,
        "2024 estimated tax payments and amount applied from 2023 return",
    );
    c.line(
        *line31,
        "f1040",
        "31",
        "Form1040Lines.line31",
        Production::Carry,
        "Amount from Schedule 3, line 15",
    );
    c.line(
        *line32,
        "f1040",
        "32",
        "Form1040Lines.line32",
        Production::Combine,
        "Add lines 27, 28, 29, and 31. These are your total other payments and refundable credits",
    );
    c.line(
        *line33,
        "f1040",
        "33",
        "Form1040Lines.line33",
        Production::Combine,
        "Add lines 25d, 26, and 32. These are your total payments",
    );
    c.exception(*line34, "f1040", "34", "Form1040Lines.line34",
        "If line 33 is more than line 24, subtract line 24 from line 33. This is the amount you overpaid",
        "A CONDITIONAL entry, not a clamp. The sentence states the condition (\"If line 33 is more than line 24\") but prints no \"enter -0-\", so when the condition fails the line is BLANK — the figure belongs on line 37 instead. No production carries that blank-ness rule: Combine is blank iff its operands are blank (both are populated on an owing return), and Clamped requires an \"enter -0-\" clause the form does not give, which is exactly why checker rule (3)'s \"-0-\" half rejects it even though its clause DOES match the FLOOR_IDIOM \"is more than line\". Its mutually exclusive twin, line 37, states the same subtraction with no condition at all.");
    c.line(
        *line37,
        "f1040",
        "37",
        "Form1040Lines.line37",
        Production::Combine,
        "Subtract line 33 from line 24. This is the amount you owe.",
    );
    c
}

fn zero_form1040lines() -> crate::tax::printed::Form1040Lines {
    crate::tax::printed::Form1040Lines {
        line1z: Usd::ZERO,
        line1a: Usd::ZERO,
        line2a: Usd::ZERO,
        line2b: Usd::ZERO,
        line3a: Usd::ZERO,
        line3b: Usd::ZERO,
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
        line18: Usd::ZERO,
        line19: Usd::ZERO,
        line20: Usd::ZERO,
        line21: Usd::ZERO,
        line22: Usd::ZERO,
        line23: Usd::ZERO,
        line24: Usd::ZERO,
        line25a: Usd::ZERO,
        line25b: Usd::ZERO,
        line25c: Usd::ZERO,
        line25d: Usd::ZERO,
        line26: Usd::ZERO,
        line31: Usd::ZERO,
        line32: Usd::ZERO,
        line33: Usd::ZERO,
        line34: Usd::ZERO,
        line37: Usd::ZERO,
        digital_asset_yes: false,
    }
}

/// **Form1040Income** — the main form. Destructured with no `..`.
///
/// ★ Field keys are STRUCT-QUALIFIED (`Form1040Income.lineN`) because `Form1040Income` re-declares eleven of
/// `Form1040Lines`' lines with identical names on the identical form — rule (5) keys on
/// `(form, line, field)` and would otherwise report each as covered twice.
pub fn cover_form1040income(l: &crate::tax::printed::Form1040Income) -> Coverage {
    let crate::tax::printed::Form1040Income {
        qdcgt_net_capital_gain,
        line1a,
        line1z,
        line2a,
        line2b,
        line3a,
        line3b,
        line7,
        line8,
        line9,
        line10,
        line11,
    } = l;
    let mut c = Coverage::default();
    c.exception(*qdcgt_net_capital_gain, "i1040gi", "QDCGT Worksheet, 3", "Form1040Income.qdcgt_net_capital_gain",
        "Yes. Enter the smaller of line 15 or line 16 of Schedule D. If either line 15 or line 16 is blank or a loss, enter -0-.",
        "A COMPOSITION whose clamp idiom the checker does not know. The operand is Bounded (\"the smaller of line 15 or line 16 of Schedule D\") and the result is then clamped by \"If either line 15 or line 16 is blank or a loss, enter -0-\" — a phrasing in neither FLOOR_IDIOMS nor CEIL_IDIOMS, so declaring Clamped(FloorAtZero) would assert a polarity the quote does not say and rule (3) would reject it. It is also a two-branch worksheet line (the \"No\" branch enters Form 1040 line 7), and this worksheet is never emitted: the value is a worksheet OPERAND that reaches no filed page. Same shape as the f1040sse:10 exception.");
    c.line(
        *line1a,
        "f1040",
        "1a",
        "Form1040Income.line1a",
        Production::Collected,
        "Total amount from Form(s) W-2, box 1 (see instructions)",
    );
    c.line(
        *line1z,
        "f1040",
        "1z",
        "Form1040Income.line1z",
        Production::Combine,
        "Add lines 1a through 1h",
    );
    c.line(
        *line2a,
        "f1040",
        "2a",
        "Form1040Income.line2a",
        Production::Collected,
        "Tax-exempt interest",
    );
    c.line(
        *line2b,
        "f1040",
        "2b",
        "Form1040Income.line2b",
        Production::Collected,
        "Taxable interest",
    );
    c.line(
        *line3a,
        "f1040",
        "3a",
        "Form1040Income.line3a",
        Production::Collected,
        "Qualified dividends",
    );
    c.line(
        *line3b,
        "f1040",
        "3b",
        "Form1040Income.line3b",
        Production::Collected,
        "Ordinary dividends",
    );
    c.line(
        *line7,
        "f1040",
        "7",
        "Form1040Income.line7",
        Production::Carry,
        "Capital gain or (loss). Attach Schedule D if required. If not required, check here",
    );
    c.line(
        *line8,
        "f1040",
        "8",
        "Form1040Income.line8",
        Production::Carry,
        "Additional income from Schedule 1, line 10",
    );
    c.line(
        *line9,
        "f1040",
        "9",
        "Form1040Income.line9",
        Production::Combine,
        "Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income",
    );
    c.line(
        *line10,
        "f1040",
        "10",
        "Form1040Income.line10",
        Production::Carry,
        "Adjustments to income from Schedule 1, line 26",
    );
    c.line(
        *line11,
        "f1040",
        "11",
        "Form1040Income.line11",
        Production::Combine,
        "Subtract line 10 from line 9. This is your adjusted gross income",
    );
    c
}

fn zero_form1040income() -> crate::tax::printed::Form1040Income {
    crate::tax::printed::Form1040Income {
        qdcgt_net_capital_gain: Usd::ZERO,
        line1a: Usd::ZERO,
        line1z: Usd::ZERO,
        line2a: Usd::ZERO,
        line2b: Usd::ZERO,
        line3a: Usd::ZERO,
        line3b: Usd::ZERO,
        line7: Usd::ZERO,
        line8: Usd::ZERO,
        line9: Usd::ZERO,
        line10: Usd::ZERO,
        line11: Usd::ZERO,
    }
}

/// **Printed8949Row** — one disposal row. `part` is "I" (short-term) or "II" (long-term).
///
/// ★★ ONE TYPE, TWO PARTS, and both number their rows "1". Without the part qualifier rule (5) reports
/// every field covered twice — the same shape as `ScheduleBRow` on Schedule B lines 1 and 5.
///
/// ★★ THE QUOTES ARE COLUMN NAMES, NOT FULL HEADERS, AND THAT IS A LIMIT OF THE TEXT LAYER.
/// `pdftotext -layout` renders the 8949 header block as horizontal BANDS crossing all seven columns,
/// so no column's header cell is contiguous after whitespace collapse. The transcription pass verified
/// that "Subtract column (e) from column (d) and combine the result with column (g)" — which
/// `printed.rs` presents as *"the form's own column-(h) header"* — is NOT findable contiguously. The
/// words are on the page; the sentence is not quotable. `gain_h` is still `Combine`: the
/// subtract-and-combine arithmetic is printed, just not in one span.
pub fn cover_printed8949row(r: &crate::tax::printed::Printed8949Row, part: &str) -> Coverage {
    let crate::tax::printed::Printed8949Row {
        description: _,
        date_acquired: _,
        date_sold: _,
        proceeds_d,
        cost_e,
        gain_h,
    } = r;
    let mut c = Coverage::default();
    c.line(
        *proceeds_d,
        "f8949",
        &fmt_part(part, "1", "d"),
        "proceeds_d",
        Production::Collected,
        "Proceeds",
    );
    c.line(
        *cost_e,
        "f8949",
        &fmt_part(part, "1", "e"),
        "cost_e",
        Production::Collected,
        "Cost or other basis",
    );
    c.line(
        *gain_h,
        "f8949",
        &fmt_part(part, "1", "h"),
        "gain_h",
        Production::Combine,
        "Gain or (loss)",
    );
    c
}

/// **Printed8949Totals** — the per-part totals row.
pub fn cover_printed8949totals(t: &crate::tax::printed::Printed8949Totals, part: &str) -> Coverage {
    let crate::tax::printed::Printed8949Totals {
        proceeds_d,
        cost_e,
        gain_h,
    } = t;
    let mut c = Coverage::default();
    c.line(*proceeds_d, "f8949", &fmt_part(part, "2", "d"), "proceeds_d", Production::Combine,
        "Totals. Add the amounts in columns (d), (e), (g), and (h) (subtract negative amounts). Enter each total here and include on your");
    c.line(*cost_e, "f8949", &fmt_part(part, "2", "e"), "cost_e", Production::Combine,
        "Totals. Add the amounts in columns (d), (e), (g), and (h) (subtract negative amounts). Enter each total here and include on your");
    c.line(*gain_h, "f8949", &fmt_part(part, "2", "h"), "gain_h", Production::Combine,
        "Totals. Add the amounts in columns (d), (e), (g), and (h) (subtract negative amounts). Enter each total here and include on your");
    c
}

/// Part-qualify a line label: `("I", "1", "d")` -> `"I-1(d)"`.
fn fmt_part(part: &str, line: &str, col: &str) -> String {
    format!("{part}-{line}({col})")
}

fn zero_printed8949row() -> crate::tax::printed::Printed8949Row {
    crate::tax::printed::Printed8949Row {
        description: String::new(),
        date_acquired: crate::conventions::TaxDate::from_ordinal_date(2024, 1).unwrap(),
        date_sold: crate::conventions::TaxDate::from_ordinal_date(2024, 1).unwrap(),
        proceeds_d: Usd::ZERO,
        cost_e: Usd::ZERO,
        gain_h: Usd::ZERO,
    }
}

fn zero_printed8949totals() -> crate::tax::printed::Printed8949Totals {
    crate::tax::printed::Printed8949Totals {
        proceeds_d: Usd::ZERO,
        cost_e: Usd::ZERO,
        gain_h: Usd::ZERO,
    }
}

/// **`SeTaxResult`** — the CRYPTO-SLICE Schedule SE.
///
/// ★★★ Found by the derived scope predicate, not by anyone looking: `btctax-forms/src/lib.rs:149`
/// takes `se: &SeTaxResult` to fill a Schedule SE, so these figures reach paper — and no hand-list
/// would have contained it, because the full-return path uses a *different* type
/// (`ScheduleSeLines`) that was already covered. Two shapes of one form, one covered and one not.
///
/// ★ One value can serve several lines: the emitter writes `se.base` to 4a, 4c and 6, and `se.net_se`
/// to 2 and 3 — so the rows are per LINE, which is why rule (5) keys on `(form, line, field)`.
pub fn cover_setaxresult(r: &crate::tax::se::SeTaxResult) -> Coverage {
    let crate::tax::se::SeTaxResult {
        net_se,
        base,
        ss,
        medicare,
        addl,
        total,
        deductible_half,
    } = r;
    let f = "f1040sse";
    let mut c = Coverage::default();
    c.line(*net_se, f, "2", "net_se", Production::Collected,
        "Net profit or (loss) from Schedule C, line 31; and Schedule K-1 (Form 1065), box 14, code A");
    c.line(
        *net_se,
        f,
        "3",
        "net_se",
        Production::Combine,
        "Combine lines 1a, 1b, and 2",
    );
    c.exception(*base, f, "4a", "base",
        "If line 3 is more than zero, multiply line 3 by 92.35% (0.9235). Otherwise, enter amount from line 3",
        "A two-branch line where BOTH branches enter a figure — one Scaled, one Carry. Same class as \
         the full-return f1040sse:4a exception.");
    c.line(
        *base,
        f,
        "4c",
        "base",
        Production::Combine,
        "Combine lines 4a and 4b",
    );
    c.line(
        *base,
        f,
        "6",
        "base",
        Production::Combine,
        "Add lines 4c and 5b",
    );
    c.line(
        *ss,
        f,
        "10",
        "ss",
        Production::Scaled,
        "Multiply the smaller of line 6 or line 9 by 12.4% (0.124)",
    );
    c.line(
        *medicare,
        f,
        "11",
        "medicare",
        Production::Scaled,
        "Multiply line 6 by 2.9% (0.029)",
    );
    c.line(
        *total,
        f,
        "12",
        "total",
        Production::Combine,
        "Self-employment tax. Add lines 10 and 11.",
    );
    c.line(
        *deductible_half,
        f,
        "13",
        "deductible_half",
        Production::Scaled,
        "Deduction for one-half of self-employment tax. Multiply line 12 by 50% (0.50). Enter here \
         and on Schedule 1 (Form 1040), line 15",
    );
    // ★ `addl` is Additional Medicare Tax — a FORM 8959 figure, not a Schedule SE line. The emitter
    //   never writes it here, and `schedule_se.rs:75` says so: "SS + regular Medicare ONLY (addl is a
    //   Form 8959 item)". Recorded rather than dropped, because a silently-unmentioned money field is
    //   the gap this module exists to close.
    //   ★★★ Its instruction is EMPTY, and must be: it used to quote Schedule SE line 12's
    //   "Self-employment tax. Add lines 10 and 11." — a verbatim sentence from a line this very row
    //   says it is not. Rule (2) checked only that the sentence existed somewhere on the form, so the
    //   misattribution passed; rule (2c) now rejects any quote on a "(none)" row.
    c.exception(*addl, f, "(none)", "addl",
        "",
        "NOT a Schedule SE line. Additional Medicare Tax is computed here for convenience and printed \
         on FORM 8959, whose own coverage carries it. Never written by schedule_se.rs.");
    c
}

fn zero_setaxresult() -> crate::tax::se::SeTaxResult {
    crate::tax::se::SeTaxResult {
        net_se: Usd::ZERO,
        base: Usd::ZERO,
        ss: Usd::ZERO,
        medicare: Usd::ZERO,
        addl: Usd::ZERO,
        total: Usd::ZERO,
        deductible_half: Usd::ZERO,
    }
}

/// Every covered form, in one place. Grows one entry per form as coverage lands.
///
/// ★ The values are irrelevant — only the TABLE is extracted. The instances exist so the exhaustive
/// destructures run, which is where the compile-time guarantee lives.
pub fn all() -> Coverage {
    let mut c = Coverage::default();
    c.0.extend(cover_form8995lines(&zero_form8995lines()).0);
    c.0.extend(cover_form8995apartiv(&crate::tax::qbi_a::Form8995APartIv::default()).0);
    // §G-28/B1b — Parts II and III. ★ A NEW STRUCT produces no compile error here, unlike a new FIELD
    // on an existing one, so these two lines are the only thing standing between 25 printed money
    // lines and no instruction-text check at all. They shipped missing once.
    c.0.extend(cover_form8995apartii(&crate::tax::qbi_a::Form8995APartIi::default()).0);
    c.0.extend(cover_form8995apartiii(&crate::tax::qbi_a::Form8995APartIii::default()).0);
    c.0.extend(cover_setaxresult(&zero_setaxresult()).0);
    c.0.extend(cover_form1040lines(&zero_form1040lines()).0);
    c.0.extend(cover_form1040income(&zero_form1040income()).0);
    // ★ Both 8949 parts, because one type serves Part I and Part II.
    for part in ["I", "II"] {
        c.0.extend(cover_printed8949row(&zero_printed8949row(), part).0);
        c.0.extend(cover_printed8949totals(&zero_printed8949totals(), part).0);
    }
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
    // ★ The routing enum's money lives in one variant; cover that variant explicitly.
    c.0.extend(
        cover_scheduledrouting(&crate::tax::printed::ScheduleDRouting::NetLoss {
            line21: Usd::ZERO,
            line22_yes: false,
        })
        .0,
    );
    c
}
