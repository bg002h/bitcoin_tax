//! **Schedule 1-A (Form 1040), Additional Deductions — TY2025**, transcribed line by line.
//!
//! One field per printed line LABEL, named for it, in the form's own numbering, carrying the line's
//! own instruction text verbatim as its doc comment (`CLAUDE.md`'s standing rule). Nothing here is
//! derived, closed-form or compressed: every defect in this repo's 2026-07-27 AMT sequence was a line
//! that was never typed in, and the dropped term becomes invisible once the line is gone.
//!
//! ★★★ **THE COUNT IS 48 LABELS AND 52 LEAVES, and both numbers are asserted, not asserted-about.**
//! 38 is merely the highest line NUMBER. The lettered runs are `2a-2e`, `4a-4c`, `14a-14c`, `22a-22b`,
//! `36a-36b`; per part I 7, II 12, III 10, IV 10, V 8, VI 1 = **48** entry lines. Lines **4** and
//! **22** are headings that carry instruction text and no amount box of their own, so they are labels
//! the census must account for and NOT fields here. The 52nd leaf appears once line 22's three
//! columns — (i) VIN, (ii) deducted on Schedule C/E/F, (iii) Schedule 1-A — are counted: 46 single-leaf
//! lines + 2 rows × 3 columns = 52. The expected set is never a range: a `BTreeSet` built from `1..=38`
//! either reds on every lettered field or forces the struct to collapse and lose a sub-line.
//!
//! ★★ **EVERY MONEY LEAF IS `Option<Usd>`, AND THAT IS THE POINT.** *Blank is the normal case.* A filer
//! with no tips, no overtime, no car loan and no senior leaves most of this schedule empty and that is
//! the CORRECT return. A `Usd` here would make *not completed* inexpressible and force a `0`, which on
//! a filed page is sworn testimony that the amount IS zero (`FOLLOWUPS.md` §G-11 — an entry is
//! testimony). The three lawful moves are collect, refuse, or genuinely blank; "silently zero" is none
//! of them.
//!
//! ★★ **COMPLETION IS SCOPED PER LINE, NEVER PER PART**, and the SOURCE is named per part — see
//! [`Schedule1aCompletion`] and [`is_completed`]. Only Parts II, III and IV print a completion
//! condition in their Caution. Part V's Caution is an **eligibility bar** (SSN and joint filing, no
//! birth date anywhere in it), so transcribing it as the completion predicate lets a non-senior
//! "complete" Part V and **line 35 prints $6,000 for a non-senior**. Part I prints no Caution at all
//! and its predicate covers **lines 2a-2e only** — a part-scoped reading blanks line 3, the MAGI that
//! lines 8, 16, 25 and 31 each read.
//!
//! **Sources.** The form is `design/forms/extract/f1040s1a--2025.txt` (`f1040s1a--2025.pdf`,
//! sha256 `64f97b38…`), committed in-crate at `fixtures/schedule_1a_2025_form.txt`; the instructions
//! are `i1040gi--2025.pdf` pp. 101-110, committed in-crate at
//! `fixtures/schedule_1a_2025_instructions.txt`. Both are the TEXT LAYER, never a rendered page: a
//! rendered `12` and `22` differ by a few pixels, and that confusion once inflated a tentative minimum
//! tax by $200,000.
//!
//! **What lives elsewhere.** The conformance KAT is in `tables.rs::schedule_1a_conformance`
//! (per-line quotation, the four worksheets, provenance and completion) and in
//! `xtask::schedule_1a_membership` (the 48 form labels, driven from `label_reader`'s two witnesses).
//! Provenance rows are `line_coverage::cover_schedule1a`. The arithmetic is T4, the input surface is
//! T3, and the PDF is B4.

#![deny(unused_variables)]

use crate::conventions::Usd;

/// One data leaf of the filed schedule, tagged with what KIND of thing the box holds.
///
/// ★ `Steps` is not money and must not be confused with it: lines 11, 19 and 28 hold the WHOLE-NUMBER
/// quotient the form's own *"Divide line N by $1,000"* produces, and whole-dollar rounding applied to
/// them would be a category error. Form 8995-A line 24 is the in-repo precedent for a non-money
/// printed line (`line_coverage.rs` records it as an `Exception` for exactly this reason).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leaf<'a> {
    /// A dollar box. Blank when the line is not completed or the inputs say so.
    Money(Option<Usd>),
    /// A whole-number step count (lines 11, 19, 28). Not dollars.
    Steps(Option<Usd>),
    /// A non-money entry — line 22's VIN column. Never a figure.
    Text(Option<&'a str>),
}

// ───────────────────────────── Part I — MAGI ─────────────────────────────

/// **Part I — Modified Adjusted Gross Income (MAGI) Amount.**
///
/// The form prints **no Caution** for this part, and its completion predicate is scoped to lines
/// 2a-2e: *"If you don't have income from Puerto Rico that you excluded from your income, or you
/// aren't filing Form 2555 or 4563, then enter the amount from Form 1040, 1040-SR, or 1040-NR,
/// line 11b, on Schedule 1-A, line 3. If you do have excluded income from Puerto Rico, or you are
/// filing Form 2555 or 4563, complete lines 2a through 2e in Part I of Schedule 1-A to figure your
/// MAGI."*
///
/// ★ **The operative sentence is the SECOND one.** The first states the condition as a disjunction
/// (*"don't have … or you aren't filing …"*) which, read literally, is satisfied by a filer who has
/// excluded Puerto Rico income but files no Form 2555 — the case the second sentence sends to lines
/// 2a-2e. The affirmative statement governs, and it is also the fail-closed direction: completing
/// 2a-2e can only ADD to MAGI, which can only shrink a phase-out-reduced deduction.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aPartI {
    /// **Line 1** — "Enter the amount from Form 1040, 1040-SR, or 1040-NR, line 11b"
    pub line1: Option<Usd>,
    /// **Line 2a** — "Enter any income from Puerto Rico that you excluded"
    pub line2a: Option<Usd>,
    /// **Line 2b** — "Enter the amount from Form 2555, line 45"
    pub line2b: Option<Usd>,
    /// **Line 2c** — "Enter the amount from Form 2555, line 50"
    pub line2c: Option<Usd>,
    /// **Line 2d** — "Enter the amount from Form 4563, line 15"
    pub line2d: Option<Usd>,
    /// **Line 2e** — "Add lines 2a, 2b, 2c, and 2d"
    pub line2e: Option<Usd>,
    /// **Line 3** — "Add lines 1 and 2e"
    ///
    /// ★ The MAGI. Lines 8, 16, 25 and 31 each read it, which is why Part I's predicate is
    /// line-scoped: a part-level "skip Part I" would blank this and silently zero four phase-outs.
    pub line3: Option<Usd>,
}

// ───────────────────────────── Part II — No Tax on Tips ─────────────────────────────

/// **Part II — No Tax on Tips.**
///
/// Completion source: the form's own Caution — *"Fill out Part II only if you received qualified
/// tips. These tips must have been received in an occupation listed at IRS.gov/TippedOccupations. You
/// and/or your spouse who received qualified tips must have a valid social security number to claim
/// the deduction. If married, you must file jointly to claim this deduction. See instructions."*
///
/// ★ Line **4** is a heading — it carries instruction text and no amount box of its own — so it is a
/// label the census accounts for and not a field. Its text is *"Qualified tips received as an
/// employee. If you received tips as an employee with respect to employment with more than one
/// employer, enter -0- on lines 4a and 4b and see the instructions to determine the amount to enter on
/// line 4c."*
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aPartII {
    /// **Line 4a** — "Enter qualified tips included on Form W-2, box 7, but see the instructions if
    /// Form W-2, box 5 is more than $176,100 or you received tips that are not subject to social
    /// security and Medicare taxes"
    pub line4a: Option<Usd>,
    /// **Line 4b** — "Qualified tips included on Form 4137, line 1, row A, column (c). If Form 4137 is
    /// not filed, enter -0-"
    ///
    /// ★★ **The `-0-` here is the FORM'S OWN conditional constant, not a guess about the filer.**
    /// btctax emits no Form 4137 on any return, so *"If Form 4137 is not filed"* is true of every
    /// return it produces — a fact about our output, which no declaration could make more or less
    /// true. ★ The existing guard `return_refuse.rs` applies to `w2.box8_allocated_tips` (*allocated*
    /// tips) is a guard for a DIFFERENT defect — a return that ought to have carried a Form 4137 — and
    /// it is partial, because Form 4137 is also required for tips the employee never reported to the
    /// employer, which `W2` cannot see. That partiality shrinks line 4b, and line 4c takes the LARGER
    /// of 4a/4b, so the direction is fail-closed.
    pub line4b: Option<Usd>,
    /// **Line 4c** — "If you only received qualified tips as an employee with respect to employment
    /// with one employer, enter the larger of line 4a or line 4b. Otherwise, see the instructions to
    /// determine the amount to enter on line 4c. If you received tips as an employee in more than one
    /// occupation, see the instructions"
    ///
    /// The *"Otherwise"* branch is [`QualifiedTipsFromMoreThanOneEmployerWorksheet`].
    pub line4c: Option<Usd>,
    /// **Line 5** — "Qualified tips received in the course of a trade or business. Qualified tip
    /// amount included in Form 1099-NEC, box 1; Form 1099-MISC, box 3; or Form 1099-K, box 1a. Do not
    /// enter more than the net profit from the trade or business. If you received qualified tips in
    /// the course of more than one trade or business or in more than one occupation, see instructions"
    ///
    /// ★★★ **THE CEILING IS A PROPERTY OF THIS LINE, NOT OF THE WORKSHEET, and it is NOT net profit.**
    /// It is net profit (Schedule C line 31 / the total of Schedule E lines 28(g)-28(k) / Schedule F
    /// line 34) **minus** the deductible part of self-employment tax, the deduction for contributions
    /// to self-employed SEP/SIMPLE/qualified plans, and the self-employed health insurance deduction,
    /// floored at zero, and expressly **not** reduced by the qualified-tips deduction itself (which is
    /// what keeps it acyclic). `min(tips, net_profit)` would overstate the ceiling by half the SE tax
    /// ⇒ larger lines 6/7/13/38 ⇒ **understates tax**.
    ///
    /// ★★ **A tension, adjudicated here rather than rediscovered.** The instructions' narrative gives
    /// the one-business case outright — *"The net income limitation will be the net profit shown on
    /// the Schedule C for the business, less the amount from Schedule 1, line 15"* — while the two
    /// numbered *Examples* that follow compute $4,500 and $1,000 **without** subtracting Schedule 1
    /// line 15. **The narrative governs**, and it is also the fail-closed direction (a lower ceiling
    /// gives a smaller deduction and a higher tax). An oracle or reviewer pointing at the examples
    /// meets this note instead of re-deriving it.
    ///
    /// ★ **T3a REFUSES rather than computing this**, because the ceiling is un-implementable from what
    /// btctax collects: printed Schedule 1 Part II carries lines 15/18/21 only — no SEP/SIMPLE field,
    /// no self-employed-health-insurance field — and there is no Schedule E or Schedule F input at
    /// all. Computing it anyway would be the compression this project's standing rule forbids.
    pub line5: Option<Usd>,
    /// **Line 6** — "Add lines 4c and 5"
    pub line6: Option<Usd>,
    /// **Line 7** — "Enter the smaller of the amount on line 6 or $25,000"
    ///
    /// ★ The cap prints NO filing-status variant, unlike lines 9, 15, 17, 26 and 32 — it is
    /// per-return regardless of status (spec S-3). Transcribed, not inferred.
    pub line7: Option<Usd>,
    /// **Line 8** — "Enter the amount from line 3"
    pub line8: Option<Usd>,
    /// **Line 9** — "Enter $150,000 ($300,000 if married filing jointly)"
    pub line9: Option<Usd>,
    /// **Line 10** — "Subtract line 9 from line 8. If zero or less, enter the amount from line 7 on
    /// line 13"
    ///
    /// ★★ **A JUMP PAST THE PHASE-OUT, NOT A CLAMP TO ZERO.** Transcribing this as *"enter -0-"* would
    /// zero the whole tips deduction for every filer under the threshold — most of them. The same
    /// routing appears at lines 18 and 27; each is quoted as the form prints it rather than as one
    /// bracketed composite, because a synthesized quotation is not a citation.
    pub line10: Option<Usd>,
    /// **Line 11** — "Divide line 10 by $1,000. If the resulting number isn’t a whole number, decrease
    /// the result to the next lower whole number. (For example, decrease 1.5 to 1, and decrease 0.05
    /// to 0.)"
    ///
    /// ★★★ **FLOOR — and the direction is read off this sentence, never hand-assigned.** Part IV's
    /// line 28 says *increase … to the next higher whole number*. A shared `phase_out` helper with one
    /// direction is silently wrong on one side by exactly $100 and $200, which is what the
    /// instructions' own worked examples (b) $2,300 and (c) $5,000 measure.
    pub line11_steps: Option<Usd>,
    /// **Line 12** — "Multiply line 11 by $100"
    pub line12: Option<Usd>,
    /// **Line 13** — "Qualified tips deduction. Subtract line 12 from line 7. If zero or less, enter
    /// -0-"
    pub line13: Option<Usd>,
}

// ───────────────────────────── Part III — No Tax on Overtime ─────────────────────────────

/// **Part III — No Tax on Overtime.**
///
/// Completion source: the form's own Caution — *"Fill out Part III only if you received qualified
/// overtime compensation. You and/or your spouse who received the qualified overtime compensation
/// must have a valid social security number to claim this deduction. If married, you must file jointly
/// to claim this deduction. See instructions."*
///
/// ★ Line **14** merges with `14a` in the text layer (they share a printed row), which is a measured
/// fact about the form and not a dropped line: `14a` is a real entry line, and the two box-less
/// headings on this schedule are **4** and **22**.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aPartIII {
    /// **Line 14a** — "Qualified overtime compensation included in Form W-2, box 1. If you received
    /// qualified overtime compensation not reported on Form W-2, box 1, see instructions"
    pub line14a: Option<Usd>,
    /// **Line 14b** — "Qualified overtime compensation included in Form 1099-NEC, box 1, or Form
    /// 1099-MISC, box 3 (see instructions)"
    ///
    /// ★ There is no 1099-NEC or 1099-MISC struct anywhere in btctax's input model, so this line is
    /// blank *because nothing can populate it*. T3a records the disposition per line rather than
    /// printing a laundered zero: the 1099-NEC side is held by the Part III claim gate conjoined with
    /// `schedule_c.is_some()`, and the 1099-MISC box 3 side (Other Income, Schedule 1 line 8z) is held
    /// by `other_out_of_scope_income`, which a filer must answer `Some(false)` for a return to be
    /// produced at all.
    pub line14b: Option<Usd>,
    /// **Line 14c** — "Add lines 14a and 14b"
    pub line14c: Option<Usd>,
    /// **Line 15** — "Enter the smaller of the amount on line 14c or $12,500 ($25,000 if married
    /// filing jointly)"
    pub line15: Option<Usd>,
    /// **Line 16** — "Enter the amount from line 3"
    pub line16: Option<Usd>,
    /// **Line 17** — "Enter $150,000 ($300,000 if married filing jointly)"
    pub line17: Option<Usd>,
    /// **Line 18** — "Subtract line 17 from line 16. If zero or less, enter the amount from line 15 on
    /// line 21"
    pub line18: Option<Usd>,
    /// **Line 19** — "Divide line 18 by $1,000. If the resulting number isn’t a whole number, decrease
    /// the result to the next lower whole number. (For example, decrease 1.5 to 1, and decrease 0.05
    /// to 0.)"
    pub line19_steps: Option<Usd>,
    /// **Line 20** — "Multiply line 19 by $100"
    pub line20: Option<Usd>,
    /// **Line 21** — "Qualified overtime compensation deduction. Subtract line 20 from line 15. If
    /// zero or less, enter -0-"
    pub line21: Option<Usd>,
}

// ───────────────────────────── Part IV — No Tax on Car Loan Interest ─────────────────────────────

/// One row of line 22's applicable-passenger-vehicle table — the form prints rows **a and b only**.
///
/// ★★ **LINE 22's ARITY IS DECIDED HERE, and the decision is TWO ROWS + A REFUSAL above two.** The form
/// prints two rows and says *"If more than two VINs, see instructions"*; the instructions say *"attach
/// a statement to your return showing the information required on line 22."* btctax emits no such
/// statement, so the two alternatives were: cap silently at two, which drops a third vehicle's
/// interest, or sum three vehicles into two rows, which yields a correct line 23 while omitting a VIN
/// the deduction is expressly conditioned on. Neither is lawful. The structure is therefore the form's
/// own arity, and a third vehicle is a **refusal** — owned by T3, where the input surface lands, and
/// pinned here so B4 cannot discover it.
///
/// ★ Column (i) is the only non-money leaf on the schedule. A VIN is a new class of filed data the
/// generic PII scanner does not cover (risk R-4), so B4's emitter tests assert no VIN-shaped literal
/// reaches a fixture, and none appears here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line22Row {
    /// **Line 22 (i)** — "(i) Vehicle identification number (VIN)"
    pub col_i_vin: Option<String>,
    /// **Line 22 (ii)** — "(ii) Deducted on"
    ///
    /// ★ The header is truncated **by the form's own text layer, not by this transcription**. It reads
    /// *"(ii) Deducted on Schedule C, Schedule E, or Schedule F"* on the page, but `pdftotext -layout`
    /// transposes the three-column header block, so the words arrive interleaved with column (i)'s and
    /// column (iii)'s and no contiguous run longer than `"(ii) Deducted on"` survives. That is the same
    /// class `line_coverage_check` records for Form 8949's transposed header block, and it is why the
    /// checkable quote is the fragment while the full header is stated here.
    pub col_ii_deducted_elsewhere: Option<Usd>,
    /// **Line 22 (iii)** — "(iii) Schedule 1-A"
    ///
    /// Defined by Part IV's Caution: *"Column (iii) is the total QPVLI paid in 2025 less the amounts
    /// reported in column (ii)."* Line 23 adds this column across both rows.
    pub col_iii_schedule_1a: Option<Usd>,
}

/// **Part IV — No Tax on Car Loan Interest.**
///
/// Completion source: the form's own Caution — *"Fill out Part IV only if you, or your spouse if
/// married filing jointly, paid or accrued qualified passenger vehicle loan interest (QPVLI). Column
/// (iii) is the total QPVLI paid in 2025 less the amounts reported in column (ii). See instructions."*
///
/// ★ Line **22** is a heading — it carries the table caption and the column headers and no amount box
/// of its own — so it is a label the census accounts for and not a field. Its sub-rows `22a` and `22b`
/// are the entry lines.
///
/// ★ Part IV prints **no** valid-SSN requirement and its instructions have no *Valid SSN* paragraph,
/// so gating it on one would deny a deduction §163(h)(4) allows. The SSN bar is Parts II, III and V.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aPartIV {
    /// Line 22, row **a** — the first applicable passenger vehicle. Its three columns carry the
    /// quoted headers, on [`Line22Row`]; the row itself prints a bare `a` and no text of its own.
    pub line22a: Line22Row,
    /// Line 22, row **b** — the second applicable passenger vehicle.
    pub line22b: Line22Row,
    /// **Line 23** — "Add lines 22a and 22b, column (iii)"
    pub line23: Option<Usd>,
    /// **Line 24** — "Enter the smaller of the amount on line 23 or $10,000"
    pub line24: Option<Usd>,
    /// **Line 25** — "Enter the amount from line 3"
    pub line25: Option<Usd>,
    /// **Line 26** — "Enter $100,000 ($200,000 if married filing jointly)"
    pub line26: Option<Usd>,
    /// **Line 27** — "Subtract line 26 from line 25. If zero or less, enter the amount from line 24 on
    /// line 30"
    pub line27: Option<Usd>,
    /// **Line 28** — "Divide line 27 by $1,000. If the resulting number isn’t a whole number, increase
    /// the result to the next higher whole number. (For example, increase 1.5 to 2, and increase 0.05
    /// to 1.)"
    ///
    /// ★★★ **CEIL — the one part that rounds the other way, and the exhaustion identity moves with
    /// it.** Because this ceils, the deduction exhausts one dollar past the last full step:
    /// `threshold + (cap/per_step − 1) × step + 1` = **+$49,001**, not +$50,000. At excess $49,000
    /// line 28 = 49 → line 29 = $9,800 → line 30 = $200; at $49,001 line 28 = 50 → line 30 = $0.
    pub line28_steps: Option<Usd>,
    /// **Line 29** — "Multiply line 28 by $200"
    pub line29: Option<Usd>,
    /// **Line 30** — "Qualified passenger vehicle loan interest deduction. Subtract line 29 from line
    /// 24. If zero or less, enter -0-"
    pub line30: Option<Usd>,
}

// ───────────────────────────── Part V — Enhanced Deduction for Seniors ─────────────────────────────

/// **Part V — Enhanced Deduction for Seniors.**
///
/// ★★★ **THE CAUTION IS AN ELIGIBILITY BAR, NOT THE COMPLETION PREDICATE.** In full it reads *"You
/// and/or your spouse must have a valid social security number. If married, you must file jointly to
/// claim this deduction."* — **no birth date anywhere in it.** The completion condition is
/// instructions-only: *"Fill out Schedule 1-A, Part V, only if: • You (and/or your spouse if filing a
/// joint return) were born before January 2, 1961. • You have a valid social security number (SSN)."*
/// Transcribe the Caution alone and a non-senior single filer with a valid SSN "completes" Part V,
/// lines 31-35 are computed, and **line 35 prints $6,000 for a non-senior** — a fabricated-testimony
/// defect (§G-11's class), not a wrong figure, because lines 36a/36b do gate on the birth date and
/// line 37 is still $0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aPartV {
    /// **Line 31** — "Enter the amount from line 3"
    pub line31: Option<Usd>,
    /// **Line 32** — "Enter $75,000 ($150,000 if married filing jointly)"
    pub line32: Option<Usd>,
    /// **Line 33** — "Subtract line 32 from line 31. If zero or less, enter $6,000 on line 35"
    ///
    /// ★★★ **A JUMP THAT WRITES A NONZERO CONSTANT INTO A LATER LINE.** Transcribing this as *"enter
    /// -0-"* yields **$0 instead of $6,000** — the whole senior deduction lost for every filer under
    /// the threshold, which is most of them. It happens to agree with `max(0, …)` only because
    /// 6% × 0 = 0, so a `max(0, …)` transcription passes for the wrong reason and breaks the moment
    /// the rate moves. The branch itself is pinned, not the arithmetic that shadows it.
    pub line33: Option<Usd>,
    /// **Line 34** — "Multiply line 33 by 6% (0.06)"
    ///
    /// ★★ **A THIRD ROUNDING SITE, easily missed because the form states no direction for it.** This
    /// is its own printed dollar line, so the general IRS whole-dollar convention governs
    /// (`round_dollar`, `MidpointAwayFromZero`, `conventions.rs`), and line 35 subtracts the PRINTED
    /// line 34. `6,000 − round(0.06 × L33)` and `round(6,000 − 0.06 × L33)` differ by $1 whenever
    /// `0.06 × L33` lands on a half-dollar (excess ≡ 25 mod 50, e.g. $50,025 → $3,001.50) — doubled on
    /// a two-senior MFJ return — and the "round the difference" form is the one that understates tax.
    pub line34: Option<Usd>,
    /// **Line 35** — "Subtract line 34 from $6,000. If zero or less, enter -0-"
    pub line35: Option<Usd>,
    /// **Line 36a** — "If you have a valid social security number (see instructions) and were born
    /// before January 2, 1961, enter the amount from line 35"
    pub line36a: Option<Usd>,
    /// **Line 36b** — "If you are married filing jointly, your spouse has a valid social security
    /// number (see instructions), and your spouse was born before January 2, 1961, enter the amount
    /// from line 35"
    pub line36b: Option<Usd>,
    /// **Line 37** — "Enhanced deduction for seniors. Add lines 36a and 36b"
    ///
    /// ★ It is **line 37**, the senior subtotal, that reaches Form 6251 line 1a — not line 38.
    pub line37: Option<Usd>,
}

// ───────────────────────────── Part VI — Total ─────────────────────────────

/// **Part VI — Total Additional Deductions.**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aPartVI {
    /// **Line 38** — "Add lines 13, 21, 30, and 37. Enter here and on Form 1040 or 1040-SR, line 13b,
    /// or on Form 1040-NR, line 13c"
    pub line38: Option<Usd>,
}

/// **Schedule 1-A (Form 1040) 2025** — the filed page, 48 labels and 52 leaves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1A {
    pub part1: Schedule1aPartI,
    pub part2: Schedule1aPartII,
    pub part3: Schedule1aPartIII,
    pub part4: Schedule1aPartIV,
    pub part5: Schedule1aPartV,
    pub part6: Schedule1aPartVI,
}

impl Schedule1A {
    /// Every data leaf of the filed schedule as `(label, leaf)` pairs — **52** of them over **48**
    /// line labels.
    ///
    /// ★★★ **THIS IS THE ACTUAL SET THE CONFORMANCE KAT COMPARES, and it is tied to the struct by the
    /// compiler.** Every part is destructured with **NO `..`**, so a field added to any part is
    /// `E0027` here until it is listed, and a field deleted is `E0026`. The label is a literal beside
    /// the field it names, which is what makes "we forgot this line" a build failure rather than a
    /// count nobody asserts on. The EXPECTED set never appears here — it is read from the form, by
    /// `label_reader` in `xtask` and by `printed_line` in `tables.rs`.
    pub fn leaves(&self) -> Vec<(&'static str, Leaf<'_>)> {
        let Schedule1A {
            part1,
            part2,
            part3,
            part4,
            part5,
            part6,
        } = self;
        let Schedule1aPartI {
            line1,
            line2a,
            line2b,
            line2c,
            line2d,
            line2e,
            line3,
        } = part1;
        let Schedule1aPartII {
            line4a,
            line4b,
            line4c,
            line5,
            line6,
            line7,
            line8,
            line9,
            line10,
            line11_steps,
            line12,
            line13,
        } = part2;
        let Schedule1aPartIII {
            line14a,
            line14b,
            line14c,
            line15,
            line16,
            line17,
            line18,
            line19_steps,
            line20,
            line21,
        } = part3;
        let Schedule1aPartIV {
            line22a,
            line22b,
            line23,
            line24,
            line25,
            line26,
            line27,
            line28_steps,
            line29,
            line30,
        } = part4;
        let Line22Row {
            col_i_vin: vin_a,
            col_ii_deducted_elsewhere: ded_a,
            col_iii_schedule_1a: sch_a,
        } = line22a;
        let Line22Row {
            col_i_vin: vin_b,
            col_ii_deducted_elsewhere: ded_b,
            col_iii_schedule_1a: sch_b,
        } = line22b;
        let Schedule1aPartV {
            line31,
            line32,
            line33,
            line34,
            line35,
            line36a,
            line36b,
            line37,
        } = part5;
        let Schedule1aPartVI { line38 } = part6;

        vec![
            ("1", Leaf::Money(*line1)),
            ("2a", Leaf::Money(*line2a)),
            ("2b", Leaf::Money(*line2b)),
            ("2c", Leaf::Money(*line2c)),
            ("2d", Leaf::Money(*line2d)),
            ("2e", Leaf::Money(*line2e)),
            ("3", Leaf::Money(*line3)),
            ("4a", Leaf::Money(*line4a)),
            ("4b", Leaf::Money(*line4b)),
            ("4c", Leaf::Money(*line4c)),
            ("5", Leaf::Money(*line5)),
            ("6", Leaf::Money(*line6)),
            ("7", Leaf::Money(*line7)),
            ("8", Leaf::Money(*line8)),
            ("9", Leaf::Money(*line9)),
            ("10", Leaf::Money(*line10)),
            ("11", Leaf::Steps(*line11_steps)),
            ("12", Leaf::Money(*line12)),
            ("13", Leaf::Money(*line13)),
            ("14a", Leaf::Money(*line14a)),
            ("14b", Leaf::Money(*line14b)),
            ("14c", Leaf::Money(*line14c)),
            ("15", Leaf::Money(*line15)),
            ("16", Leaf::Money(*line16)),
            ("17", Leaf::Money(*line17)),
            ("18", Leaf::Money(*line18)),
            ("19", Leaf::Steps(*line19_steps)),
            ("20", Leaf::Money(*line20)),
            ("21", Leaf::Money(*line21)),
            ("22a(i)", Leaf::Text(vin_a.as_deref())),
            ("22a(ii)", Leaf::Money(*ded_a)),
            ("22a(iii)", Leaf::Money(*sch_a)),
            ("22b(i)", Leaf::Text(vin_b.as_deref())),
            ("22b(ii)", Leaf::Money(*ded_b)),
            ("22b(iii)", Leaf::Money(*sch_b)),
            ("23", Leaf::Money(*line23)),
            ("24", Leaf::Money(*line24)),
            ("25", Leaf::Money(*line25)),
            ("26", Leaf::Money(*line26)),
            ("27", Leaf::Money(*line27)),
            ("28", Leaf::Steps(*line28_steps)),
            ("29", Leaf::Money(*line29)),
            ("30", Leaf::Money(*line30)),
            ("31", Leaf::Money(*line31)),
            ("32", Leaf::Money(*line32)),
            ("33", Leaf::Money(*line33)),
            ("34", Leaf::Money(*line34)),
            ("35", Leaf::Money(*line35)),
            ("36a", Leaf::Money(*line36a)),
            ("36b", Leaf::Money(*line36b)),
            ("37", Leaf::Money(*line37)),
            ("38", Leaf::Money(*line38)),
        ]
    }
}

/// The LINE label a leaf label belongs to — `"22a(iii)"` ⇒ `"22a"`, `"13"` ⇒ `"13"`.
pub fn line_label_of(leaf_label: &str) -> &str {
    leaf_label.split_once('(').map_or(leaf_label, |(l, _)| l)
}

/// The leaves that carry **no money production**, each with the reason it carries none.
///
/// ★★★ **THIS IS THE OTHER HALF OF THE PROVENANCE INVARIANT, and without it the check cannot tell two
/// blanks apart.** Every money leaf must carry a `Production` in `line_coverage::cover_schedule1a`;
/// a leaf that carries none is a defect *unless it is recorded here with a reason*. The conformance
/// KAT compares this list to the struct's `Leaf::Text` leaves in **both** directions, so a new
/// non-money leaf cannot be waved through by silence, and an entry here that is not actually a
/// non-money leaf cannot sit unused.
pub const NON_MONEY_LEAVES: [(&str, &str); 2] = [
    (
        "22a(i)",
        "a vehicle identification number, not a figure — the only non-money entry on the schedule, \
         and the reason risk R-4 exists (the generic PII scanner does not cover VINs)",
    ),
    (
        "22b(i)",
        "the second vehicle's identification number; same class as 22a(i)",
    ),
];

/// The two printed labels that head their sub-rows and take **no entry of their own**.
///
/// ★ They are labels the census must ACCOUNT FOR, not fields. Recording them here with a reason is
/// what distinguishes *"this line encodes no decision"* from *"we forgot this line"* — the whole
/// difference between a conformance check and a checker that cannot tell two blanks apart.
pub const BOX_LESS_HEADINGS: [(&str, &str); 2] = [
    (
        "4",
        "heads lines 4a-4c; carries instruction text and no amount box of its own",
    ),
    (
        "22",
        "heads the applicable-passenger-vehicle table; its entry rows are 22a and 22b, and its \
         columns are (i)/(ii)/(iii)",
    ),
];

// ───────────────────────────── completion ─────────────────────────────

/// Where a part's completion condition is PRINTED.
///
/// ★★ **Named per part, because the parts disagree** (this is the r5 I-1 correction, and getting it
/// wrong in either direction is a defect): only Parts II, III and IV print a completion condition in
/// their Caution. Part V's Caution is an eligibility bar with no birth date in it, and Part I prints no
/// Caution at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionSource {
    /// The form's own Caution prints the condition.
    FormCaution,
    /// The INSTRUCTIONS print it and the form does not — so a transcription that reads only the form
    /// will be confidently, silently wrong.
    InstructionsOnly,
    /// Nothing is printed: the line is entered on every return that files this schedule.
    Unconditional,
}

/// Each part, its completion source, and where that source is printed.
pub const COMPLETION_SOURCES: [(&str, CompletionSource, &str); 6] = [
    (
        "I",
        CompletionSource::InstructionsOnly,
        "the form prints no Caution; the instructions scope the condition to lines 2a-2e only",
    ),
    (
        "II",
        CompletionSource::FormCaution,
        "\"Fill out Part II only if you received qualified tips.\"",
    ),
    (
        "III",
        CompletionSource::FormCaution,
        "\"Fill out Part III only if you received qualified overtime compensation.\"",
    ),
    (
        "IV",
        CompletionSource::FormCaution,
        "\"Fill out Part IV only if you, or your spouse if married filing jointly, paid or accrued \
         qualified passenger vehicle loan interest (QPVLI).\"",
    ),
    (
        "V",
        CompletionSource::InstructionsOnly,
        "the Caution is an ELIGIBILITY bar (SSN, joint filing) and omits the birth date; the \
         instructions carry \"were born before January 2, 1961\"",
    ),
    (
        "VI",
        CompletionSource::Unconditional,
        "no Caution; line 38 is the total and is always entered",
    ),
];

/// The answers the completion predicate reads — **not** the deduction figures, but whether each part
/// is to be filled out at all.
///
/// ★ Every field defaults to `false`, which is the fail-closed direction: an omission leaves a part
/// NOT completed, so a line is blank rather than carrying a fabricated figure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Schedule1aCompletion {
    /// Part I, **lines 2a-2e only** — *"If you do have excluded income from Puerto Rico, or you are
    /// filing Form 2555 or 4563, complete lines 2a through 2e in Part I of Schedule 1-A to figure your
    /// MAGI."* Lines 1 and 3 are entered regardless.
    pub part1_excluded_income_or_2555_or_4563: bool,
    /// Part II — *"Fill out Part II only if you received qualified tips."*
    pub part2_received_qualified_tips: bool,
    /// Part III — *"Fill out Part III only if you received qualified overtime compensation."*
    pub part3_received_qualified_overtime: bool,
    /// Part IV — *"Fill out Part IV only if you, or your spouse if married filing jointly, paid or
    /// accrued qualified passenger vehicle loan interest (QPVLI)."*
    pub part4_paid_qpvli: bool,
    /// Part V — *"Fill out Schedule 1-A, Part V, only if: • You (and/or your spouse if filing a joint
    /// return) were born before January 2, 1961."*
    ///
    /// ★★ **NOT the Caution's SSN/joint-filing bar.** Those are eligibility conditions on the
    /// deduction, and using them as the completion predicate is exactly the defect that prints $6,000
    /// on line 35 for a non-senior.
    pub part5_born_before_january_2_1961: bool,
}

/// Is line `label` COMPLETED on this return? `None` for a label that is not on the form.
///
/// ★★★ **PER LINE, NEVER PER PART.** Part I is the proof that the granularity matters: lines 1 and 3
/// are always entered while 2a-2e are conditional, so a part-scoped predicate is wrong whichever way
/// it resolves — blanking line 3 (the MAGI four other lines read) or printing `$0` on 2a-2e.
///
/// ★ Takes the LINE label; strip a column suffix with [`line_label_of`] first.
pub fn is_completed(label: &str, c: &Schedule1aCompletion) -> Option<bool> {
    Some(match label {
        // Part I — lines 1 and 3 are ALWAYS entered; only 2a-2e are conditional.
        "1" | "3" => true,
        "2a" | "2b" | "2c" | "2d" | "2e" => c.part1_excluded_income_or_2555_or_4563,
        // Part II.
        "4a" | "4b" | "4c" | "5" | "6" | "7" | "8" | "9" | "10" | "11" | "12" | "13" => {
            c.part2_received_qualified_tips
        }
        // Part III.
        "14a" | "14b" | "14c" | "15" | "16" | "17" | "18" | "19" | "20" | "21" => {
            c.part3_received_qualified_overtime
        }
        // Part IV.
        "22a" | "22b" | "23" | "24" | "25" | "26" | "27" | "28" | "29" | "30" => c.part4_paid_qpvli,
        // Part V — the birth date, from the INSTRUCTIONS, not the Caution.
        "31" | "32" | "33" | "34" | "35" | "36a" | "36b" | "37" => {
            c.part5_born_before_january_2_1961
        }
        // Part VI.
        "38" => true,
        _ => return None,
    })
}

// ───────────────────────────── the four worksheets ─────────────────────────────

/// The lettered rows every one of the four worksheets prints.
pub const WORKSHEET_ROW_LABELS: [&str; 5] = ["A", "B", "C", "D", "E"];

/// What a transcribed worksheet is, as the conformance KAT reads it: a title, the Schedule 1-A line
/// its total feeds, its lettered rows and its lettered columns.
///
/// ★ Every field is DERIVED from the transcription (the columns come out of an exhaustive destructure
/// of the row type, the rows out of the array's own length), never declared beside it — so deleting a
/// column or a row cannot leave this shape intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorksheetShape<'a> {
    pub title: &'static str,
    /// The Schedule 1-A line the worksheet's line 2 total is entered on.
    pub target_line: &'static str,
    pub rows: Vec<&'static str>,
    /// `(column letter, the column's printed header, the leaf)`.
    pub columns: Vec<(&'static str, &'static str, Leaf<'a>)>,
    /// Worksheet line 2 — the total that is entered on [`Self::target_line`].
    pub total: Leaf<'a>,
}

/// One employer's row on the *Qualified Tips From More Than One Employer Worksheet*.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TipsByEmployerRow {
    /// Column (a) — "Name of employer"
    pub col_a_name_of_employer: Option<String>,
    /// Column (b) — "Amount of qualified tips reported by this employer on Form W-2, or reported by
    /// you to this employer on Form(s) 4070"
    pub col_b_reported_by_this_employer: Option<Usd>,
    /// Column (c) — "Qualified tips reported on Form 4137, column 1(c), for this employer"
    pub col_c_reported_on_form_4137: Option<Usd>,
    /// Column (d) — "Enter the greater of column (b) or column (c)"
    pub col_d_greater_of_b_or_c: Option<Usd>,
}

/// **Qualified Tips From More Than One Employer Worksheet** — its line 2 total is Schedule 1-A
/// line 4c, and it is the *"Otherwise"* branch line 4c refers to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QualifiedTipsFromMoreThanOneEmployerWorksheet {
    /// Worksheet line 1, rows A-E.
    pub line1: [TipsByEmployerRow; 5],
    /// Worksheet line 2 — "Add lines 1A through 1E, column (d), and enter this amount on
    /// Schedule 1-A, line 4c"
    pub line2: Option<Usd>,
}

/// One business's row on the *Multiple Trades or Businesses Worksheet*.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultipleBusinessesRow {
    /// Column (a) — "Name of your business"
    pub col_a_name_of_your_business: Option<String>,
    /// Column (b) — "Net profit of business from Schedule C, line 31; the total of Schedule E,
    /// line 28(g) through 28(k); or Schedule F, line 34"
    pub col_b_net_profit: Option<Usd>,
    /// Column (c) — "Other deductions allocable to the trade or business and not reported on
    /// Schedule C, Schedule E, or Schedule F (as applicable)"
    pub col_c_other_deductions: Option<Usd>,
    /// Column (d) — "Subtract column (c) from column (b)"
    pub col_d_b_less_c: Option<Usd>,
    /// Column (e) — "Qualified tip amount from first Form 1099-NEC, box 1; Form 1099-MISC, box 3; or
    /// Form 1099-K, box 1a"
    pub col_e_first_1099: Option<Usd>,
    /// Column (f) — "Qualified tip amount from second Form 1099-NEC, box 1; Form 1099-MISC, box 3; or
    /// Form 1099-K, box 1a"
    pub col_f_second_1099: Option<Usd>,
    /// Column (g) — "Qualified tip amount from third Form 1099-NEC, box 1; Form 1099-MISC, box 3; or
    /// Form 1099-K, box 1a"
    pub col_g_third_1099: Option<Usd>,
    /// Column (h) — "Qualified tip amount from fourth Form 1099-NEC, box 1; Form 1099-MISC, box 3; or
    /// Form 1099-K, box 1a"
    pub col_h_fourth_1099: Option<Usd>,
    /// Column (i) — "Total qualified tip amount. Add columns (e), (f), (g), and (h)"
    pub col_i_total_tips: Option<Usd>,
    /// Column (j) — "Enter the lesser of column (d) and column (i)"
    pub col_j_lesser_of_d_and_i: Option<Usd>,
}

/// **Multiple Trades or Businesses Worksheet** — its line 2 total is Schedule 1-A line 5.
///
/// ★ The worksheet is the MULTI-ROW aggregation of line 5's ceiling; it is not where the ceiling
/// lives. `ReturnInputs::schedule_c` is singular, so the multi-business branch is unreachable today
/// and the single-business branch is the whole surface — which is precisely why putting the ceiling
/// only here would have understated tax.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultipleTradesOrBusinessesWorksheet {
    /// Worksheet line 1, rows A-E.
    pub line1: [MultipleBusinessesRow; 5],
    /// Worksheet line 2 — "Add lines 1A through 1E, column (j), and enter the total on Schedule 1-A,
    /// line 5"
    pub line2: Option<Usd>,
}

/// One employer's row on the *Qualified Overtime Compensation From More Than One Employer Worksheet*.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OvertimeByEmployerRow {
    /// Column (a) — "Name of employer"
    pub col_a_name_of_employer: Option<String>,
    /// Column (b) — "Qualified overtime reported on Form W-2, box 1"
    pub col_b_reported_on_w2_box1: Option<Usd>,
}

/// **Qualified Overtime Compensation From More Than One Employer Worksheet** — the **W-2 side**; its
/// line 2 total is Schedule 1-A line 14a.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QualifiedOvertimeFromMoreThanOneEmployerWorksheet {
    /// Worksheet line 1, rows A-E.
    pub line1: [OvertimeByEmployerRow; 5],
    /// Worksheet line 2 — "Add the amounts from lines 1A through 1E, column (b), and enter this
    /// amount on Schedule 1-A, line 14a"
    pub line2: Option<Usd>,
}

/// One payor's row on the *Qualified Overtime Compensation From More Than One Payor Worksheet*.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OvertimeByPayorRow {
    /// Column (a) — "Payor’s name"
    pub col_a_payors_name: Option<String>,
    /// Column (b) — "Qualified overtime reported on Form 1099-NEC, box 1, or Form 1099-MISC, box 3"
    pub col_b_reported_on_1099: Option<Usd>,
}

/// **Qualified Overtime Compensation From More Than One Payor Worksheet** — the **1099 side**; its
/// line 2 total is Schedule 1-A line 14b.
///
/// ★ This and the employer worksheet above are two distinct forms of the same idea, and a plan draft
/// that collapsed them lost one: they read different boxes (W-2 box 1 vs 1099-NEC box 1 / 1099-MISC
/// box 3) and feed different Schedule 1-A lines (14a vs 14b).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QualifiedOvertimeFromMoreThanOnePayorWorksheet {
    /// Worksheet line 1, rows A-E.
    pub line1: [OvertimeByPayorRow; 5],
    /// Worksheet line 2 — "Add the amounts from lines 1A through 1E, column (b) and enter this amount
    /// on Schedule 1-A, line 14b"
    pub line2: Option<Usd>,
}

/// The four *Keep for Your Records* worksheets the Schedule 1-A instructions print.
///
/// ★★★ **THEY EXIST ONLY IN THE INSTRUCTIONS, WHICH IS WHY A FORM-DRIVEN CENSUS CANNOT SEE THEM.**
/// `grep -c "Keep for Your Records"` on the FORM extract is **0**. A label census driven off the form
/// alone could never red on a worksheet omission — it would have passed by finding nothing, the exact
/// false completeness this transcription exists to prevent (census F-4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule1aWorksheets {
    pub tips_more_than_one_employer: QualifiedTipsFromMoreThanOneEmployerWorksheet,
    pub multiple_trades_or_businesses: MultipleTradesOrBusinessesWorksheet,
    pub overtime_more_than_one_employer: QualifiedOvertimeFromMoreThanOneEmployerWorksheet,
    pub overtime_more_than_one_payor: QualifiedOvertimeFromMoreThanOnePayorWorksheet,
}

fn rows_of(n: usize) -> Vec<&'static str> {
    WORKSHEET_ROW_LABELS.iter().take(n).copied().collect()
}

impl Schedule1aWorksheets {
    /// The four transcribed worksheets, as shapes the conformance KAT can compare to the
    /// instructions' own text.
    ///
    /// ★ The **array length is the count**: deleting a worksheet here reds against the fixture's four
    /// `— Keep for Your Records` anchors rather than passing quietly with three.
    pub fn shapes(&self) -> [WorksheetShape<'_>; 4] {
        let Schedule1aWorksheets {
            tips_more_than_one_employer,
            multiple_trades_or_businesses,
            overtime_more_than_one_employer,
            overtime_more_than_one_payor,
        } = self;
        [
            {
                let QualifiedTipsFromMoreThanOneEmployerWorksheet { line1, line2 } =
                    tips_more_than_one_employer;
                let TipsByEmployerRow {
                    col_a_name_of_employer,
                    col_b_reported_by_this_employer,
                    col_c_reported_on_form_4137,
                    col_d_greater_of_b_or_c,
                } = &line1[0];
                WorksheetShape {
                    title: "Qualified Tips From More Than One Employer Worksheet",
                    target_line: "4c",
                    rows: rows_of(line1.len()),
                    columns: vec![
                        (
                            "a",
                            "Name of employer",
                            Leaf::Text(col_a_name_of_employer.as_deref()),
                        ),
                        (
                            "b",
                            "Amount of qualified tips reported by this employer on Form W-2, or \
                             reported by you to this employer on Form(s) 4070",
                            Leaf::Money(*col_b_reported_by_this_employer),
                        ),
                        (
                            "c",
                            "Qualified tips reported on Form 4137, column 1(c), for this employer",
                            Leaf::Money(*col_c_reported_on_form_4137),
                        ),
                        (
                            "d",
                            "Enter the greater of column (b) or column (c)",
                            Leaf::Money(*col_d_greater_of_b_or_c),
                        ),
                    ],
                    total: Leaf::Money(*line2),
                }
            },
            {
                let MultipleTradesOrBusinessesWorksheet { line1, line2 } =
                    multiple_trades_or_businesses;
                let MultipleBusinessesRow {
                    col_a_name_of_your_business,
                    col_b_net_profit,
                    col_c_other_deductions,
                    col_d_b_less_c,
                    col_e_first_1099,
                    col_f_second_1099,
                    col_g_third_1099,
                    col_h_fourth_1099,
                    col_i_total_tips,
                    col_j_lesser_of_d_and_i,
                } = &line1[0];
                WorksheetShape {
                    title: "Multiple Trades or Businesses Worksheet",
                    target_line: "5",
                    rows: rows_of(line1.len()),
                    columns: vec![
                        (
                            "a",
                            "Name of your business",
                            Leaf::Text(col_a_name_of_your_business.as_deref()),
                        ),
                        (
                            "b",
                            "Net profit of business from Schedule C, line 31; the total of \
                             Schedule E, line 28(g) through 28(k); or Schedule F, line 34",
                            Leaf::Money(*col_b_net_profit),
                        ),
                        (
                            "c",
                            "Other deductions allocable to the trade or business and not reported \
                             on Schedule C, Schedule E, or Schedule F (as applicable)",
                            Leaf::Money(*col_c_other_deductions),
                        ),
                        (
                            "d",
                            "Subtract column (c) from column (b)",
                            Leaf::Money(*col_d_b_less_c),
                        ),
                        (
                            "e",
                            "Qualified tip amount from first Form 1099-NEC, box 1; Form 1099-MISC, \
                             box 3; or Form 1099-K, box 1a",
                            Leaf::Money(*col_e_first_1099),
                        ),
                        (
                            "f",
                            "Qualified tip amount from second Form 1099-NEC, box 1; Form \
                             1099-MISC, box 3; or Form 1099-K, box 1a",
                            Leaf::Money(*col_f_second_1099),
                        ),
                        (
                            "g",
                            "Qualified tip amount from third Form 1099-NEC, box 1; Form 1099-MISC, \
                             box 3; or Form 1099-K, box 1a",
                            Leaf::Money(*col_g_third_1099),
                        ),
                        (
                            "h",
                            "Qualified tip amount from fourth Form 1099-NEC, box 1; Form \
                             1099-MISC, box 3; or Form 1099-K, box 1a",
                            Leaf::Money(*col_h_fourth_1099),
                        ),
                        (
                            "i",
                            "Total qualified tip amount. Add columns (e), (f), (g), and (h)",
                            Leaf::Money(*col_i_total_tips),
                        ),
                        (
                            "j",
                            "Enter the lesser of column (d) and column (i)",
                            Leaf::Money(*col_j_lesser_of_d_and_i),
                        ),
                    ],
                    total: Leaf::Money(*line2),
                }
            },
            {
                let QualifiedOvertimeFromMoreThanOneEmployerWorksheet { line1, line2 } =
                    overtime_more_than_one_employer;
                let OvertimeByEmployerRow {
                    col_a_name_of_employer,
                    col_b_reported_on_w2_box1,
                } = &line1[0];
                WorksheetShape {
                    title: "Qualified Overtime Compensation From More Than One Employer Worksheet",
                    target_line: "14a",
                    rows: rows_of(line1.len()),
                    columns: vec![
                        (
                            "a",
                            "Name of employer",
                            Leaf::Text(col_a_name_of_employer.as_deref()),
                        ),
                        (
                            "b",
                            "Qualified overtime reported on Form W-2, box 1",
                            Leaf::Money(*col_b_reported_on_w2_box1),
                        ),
                    ],
                    total: Leaf::Money(*line2),
                }
            },
            {
                let QualifiedOvertimeFromMoreThanOnePayorWorksheet { line1, line2 } =
                    overtime_more_than_one_payor;
                let OvertimeByPayorRow {
                    col_a_payors_name,
                    col_b_reported_on_1099,
                } = &line1[0];
                WorksheetShape {
                    title: "Qualified Overtime Compensation From More Than One Payor Worksheet",
                    target_line: "14b",
                    rows: rows_of(line1.len()),
                    columns: vec![
                        (
                            "a",
                            "Payor’s name",
                            Leaf::Text(col_a_payors_name.as_deref()),
                        ),
                        (
                            "b",
                            "Qualified overtime reported on Form 1099-NEC, box 1, or Form \
                             1099-MISC, box 3",
                            Leaf::Money(*col_b_reported_on_1099),
                        ),
                    ],
                    total: Leaf::Money(*line2),
                }
            },
        ]
    }
}
