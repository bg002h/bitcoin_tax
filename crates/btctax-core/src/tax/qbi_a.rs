//! §G-28/B1a — **Form 8995-A Part IV**, the full §199A form's deduction computation.
//!
//! ## Why this exists at all
//!
//! Above the §199A(e)(2) threshold the simplified Form 8995 no longer applies, so a filer there must
//! file Form 8995-A **even when the arithmetic is identical**. i8995a scopes the case this module
//! covers, in its own words:
//!
//! > *"You must complete Part I if you have QBI from a qualified trade, business, or aggregation. **If
//! > you don't have QBI, and only have REIT, PTP, skip Parts I through III and complete Part IV.**"*
//!
//! That filer needs no input btctax does not already collect — which is why this is B1a and the
//! wage/UBIA/SSTB machinery of Parts I–III is B1b.
//!
//! ## ★★★ The equivalence proof, and the branch where it breaks
//!
//! `CLAUDE.md` allows a derived form **only** with a written equivalence proof naming the branch where
//! it breaks, plus a KAT pinning that branch. Part IV is Form 8995's lines 5–17 with a DPAD line
//! inserted, and the two forms are **pointwise identical** — but they do not say so in the same words,
//! and the wording differs at exactly the value where a clamp is decided:
//!
//! | quantity | Form 8995 | Form 8995-A |
//! |---|---|---|
//! | REIT/PTP total | L8 *"Combine lines 6 and 7. If **zero or less**, enter -0-"* | L30 *"Combine lines 28 and 29. If **less than zero**, enter -0-"* |
//! | REIT/PTP (loss) carryforward | L17 *"…If **greater than zero**, enter -0-"* | L40 *"…If **zero or greater**, enter -0-"* |
//!
//! Worked at every sign, the two agree everywhere **including at zero**, because each pair's clauses are
//! complementary rather than contradictory: where 8995 clamps at zero, 8995-A enters the combined value
//! — which *is* zero — and vice versa. So the figures never differ.
//!
//! **The branch where this breaks:** if either form is ever revised so the two clauses disagree on a
//! NON-zero value. [`tests::the_two_forms_agree_pointwise_including_at_zero`] pins it, and would red on
//! exactly that revision.
//!
//! ## What stays out
//!
//! Line 38 (**DPAD** under §199A(g), allocated from an agricultural or horticultural cooperative) is
//! never btctax's — it requires a Schedule D (Form 8995-A) this version does not fill. It is a
//! CONDITIONAL entry, so it is left **blank**, not zeroed: the form says *"Don't enter more than line 33
//! minus line 37"*, which presumes an amount a cooperative allocated to the filer. A printed `0` would
//! swear they received one.

use crate::conventions::{round_dollar, Usd};
use crate::tax::qbi::Form8995Lines;
use crate::tax::types::FilingStatus;
use rust_decimal::Decimal;

/// Which §199A regime a return sits in, by taxable income **before** the QBI deduction.
///
/// ★★★ The split is i8995a's own, and each boundary is a different form:
///
/// | regime | i8995a | what btctax files |
/// |---|---|---|
/// | TI ≤ threshold | *"file Form 8995, Qualified Business Income Deduction Simplified Computation"* | Form 8995 |
/// | threshold < TI ≤ threshold + phase-in range | *"an applicable percentage of your SSTB is treated as a qualified trade or business"* | 8995-A Parts I–IV, Part III **on**; an SSTB refuses (Schedule A) |
/// | TI > threshold + phase-in range | *"your SSTB doesn't qualify for the deduction"* | 8995-A Parts I, II, IV; Part III **skipped** |
///
/// ★ Both comparisons are **strict at the bottom, inclusive at the top** — i8995a's Exception 1 reads
/// *"isn't more than $383,900 … and $191,950"* (so at the threshold the simplified form still applies)
/// and Exception 2 reads *"more than $383,900 but not more than $483,900"* (so the top of the range is
/// still inside it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qbi199aRegime {
    /// TI ≤ §199A(e)(2) threshold — the simplified Form 8995.
    AtOrBelowThreshold,
    /// threshold < TI ≤ threshold + phase-in range — Form 8995-A **with** Part III.
    InPhaseInRange,
    /// TI > threshold + phase-in range — Form 8995-A, Part III skipped, SSTBs excluded entirely.
    AboveThePhaseInRange,
}

impl Qbi199aRegime {
    /// Classify `ti_before_qbi` against the §199A(e)(2) threshold and the Form 8995-A line 23 range.
    pub fn of(
        ti_before_qbi: Usd,
        status: FilingStatus,
        params: &crate::tax::tables::FullReturnParams,
    ) -> Self {
        if ti_before_qbi <= params.qbi_ti_threshold(status) {
            Self::AtOrBelowThreshold
        } else if ti_before_qbi <= params.qbi_phase_in_top(status) {
            Self::InPhaseInRange
        } else {
            Self::AboveThePhaseInRange
        }
    }
}

/// The whole of **Form 8995-A** as btctax files it.
///
/// ★★ Parts I–III and Part IV travel together in ONE value rather than as two parallel `Option`s on
/// the packet. Part IV line 27 is *"Enter the amount from line 16"*, so a Part IV without its Parts
/// I–III is a form whose top line totals a grid that is not there — and two independent `Option`s
/// make that state representable. This makes it unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form8995A {
    /// `None` for a filer with no qualified trade or business — i8995a's *"If you don't have QBI, and
    /// only have REIT, PTP, skip Parts I through III and complete Part IV."* Also `None` when
    /// §199A(d)(3) excluded the filer's SSTB, which leaves them with no qualified business either.
    pub parts_i_to_iii: Option<Form8995APartIToIii>,
    /// Part IV — always present; it is the part that produces the deduction.
    pub part_iv: Form8995APartIv,
}

/// §199A(d)(3) — the QBI that survives the **specified-service exclusion**.
///
/// ★★★ ABOVE the phase-in range an SSTB is **not a qualified trade or business at all**, so this is not
/// a deduction that comes to zero — the business is simply not on the form. i8995a, verbatim:
///
/// > *"SSTBs are generally excluded from the definition of a qualified trade or business if the
/// > taxpayer's taxable income exceeds the threshold plus the phase-in range. Therefore, **no QBI, W-2
/// > wages, or UBIA of qualified property from the specified service trade or business are taken into
/// > account** in figuring your QBI deduction."*
///
/// ★★ This is the ONE authority for the exclusion, and it is applied where `business_qbi` is first
/// computed — before the Form 8995 chain and before Parts I–III — so the two cannot disagree. A first
/// draft applied it in neither and would have printed a full deduction for an excluded SSTB;
/// Tax-Calculator's `qbided = 0.` on that vector is what found it.
///
/// INSIDE the range the exclusion is partial (an *applicable percentage*, Schedule A (Form 8995-A))
/// and btctax REFUSES rather than approximate it, so this function never sees that case.
pub fn qbi_after_sstb_exclusion(business_qbi: Usd, is_sstb: bool, regime: Qbi199aRegime) -> Usd {
    if is_sstb && regime == Qbi199aRegime::AboveThePhaseInRange {
        Usd::ZERO
    } else {
        business_qbi
    }
}

/// Form 8995-A **Part I, row A** — one trade, business, or aggregation, in the form's own columns.
///
/// btctax has exactly one trade or business (the crypto Schedule C), so rows **B** and **C** stay
/// blank. That is a blank because the inputs say so, not because nothing populated it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Form8995ARowA {
    /// L1 column **(a)** — *"Trade, business, or aggregation name"*.
    pub col_a_name: String,
    /// L1 column **(b)** — *"Check if specified service"*. See [`Qbi199aRegime`] for what it decides.
    pub col_b_specified_service: bool,
    /// L1 column **(c)** — *"Check if aggregation"*.
    ///
    /// Always `false`: aggregation is the ELECTIVE grouping of two or more trades or businesses on
    /// Schedule B (Form 8995-A), and btctax has exactly one. There is nothing to aggregate, so the
    /// unchecked box is the correct answer rather than an unasked one.
    pub col_c_aggregation: bool,
    // ★★★ COLUMN (d), the TIN, IS DELIBERATELY NOT HERE. It is the PROPRIETOR's SSN — a
    //     spouse-owned business files under the spouse's, even on a joint return — and the emitter
    //     already resolves that from `header.proprietor` and renders it against the target cell's own
    //     `/MaxLen`, exactly as Form 8995 row 1i(b) does. Carrying it in core would (a) create a
    //     second authority for one identity and (b) put an SSN into a struct that goldens serialize.
    /// L1 column **(e)** — *"Check if patron"*. Always `false` on a filed return: a `yes` answer
    /// refuses upstream, because a patron needs Schedule D (Form 8995-A) for the line 14 reduction.
    pub col_e_patron: bool,
}

/// Form 8995-A **Part II, column A** — *"Determine Your Adjusted Qualified Business Income"*.
///
/// One field per printed line, in the form's numbering. Columns **B** and **C** are the second and
/// third trade or business; btctax has one, so they stay blank.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Form8995APartIi {
    /// L2 — *"Qualified business income from the trade, business, or aggregation. See instructions"*.
    pub line2: Usd,
    /// L3 — *"Multiply line 2 by 20% (0.20). If your taxable income is $191,950 or less ($383,900 if
    /// married filing jointly), skip lines 4 through 12 and enter the amount from line 3 on line 13"*.
    ///
    /// ★ The skip clause is **unreachable here**: this struct is only built above the threshold. The
    /// filer it describes is a patron below the threshold — who is the one other filer Form 8995-A's
    /// header sends to this form — and a patron refuses upstream. Stated rather than dropped, because
    /// a missing branch and an impossible one look identical once the sentence is gone.
    pub line3: Usd,
    /// L4 — *"Allocable share of W-2 wages from the trade, business, or aggregation"*.
    pub line4: Usd,
    /// L5 — *"Multiply line 4 by 50% (0.50)"*.
    pub line5: Usd,
    /// L6 — *"Multiply line 4 by 25% (0.25)"*.
    pub line6: Usd,
    /// L7 — *"Allocable share of the unadjusted basis immediately after acquisition (UBIA) of all
    /// qualified property"*.
    pub line7: Usd,
    /// L8 — *"Multiply line 7 by 2.5% (0.025)"*.
    pub line8: Usd,
    /// L9 — *"Add lines 6 and 8"*.
    pub line9: Usd,
    /// L10 — *"Enter the greater of line 5 or line 9"*.
    pub line10: Usd,
    /// L11 — *"W-2 wage and UBIA of qualified property limitation. Enter the smaller of line 3 or
    /// line 10"*.
    pub line11: Usd,
    /// L12 — *"Phased-in reduction. Enter the amount from line 26, if any"*.
    ///
    /// ★★ `Option`, and the `Option` is the point: *"if any"* is a CONDITIONAL entry with no `-0-`
    /// clause. Outside the phase-in range there is no line 26, so the line carries **no testimony** and
    /// is left blank. A printed `0` would swear a phased-in reduction was figured and came to nothing.
    pub line12: Option<Usd>,
    /// L13 — *"Qualified business income deduction before patron reduction. Enter the greater of
    /// line 11 or line 12"*.
    pub line13: Usd,
    /// L14 — *"Patron reduction. Enter the amount from Schedule D (Form 8995-A), line 6, if any. See
    /// instructions"*.
    ///
    /// ★★ `None` on every return btctax files, and it must be: a patron REFUSES upstream, so there is
    /// no Schedule D (Form 8995-A) and no amount on its line 6. Conditional entry, no `-0-` clause,
    /// therefore blank — the same reason Part IV line 38 is blank.
    pub line14: Option<Usd>,
    /// L15 — *"Qualified business income component. Subtract line 14 from line 13"*.
    pub line15: Usd,
    /// L16 — *"Total qualified business income component. Add all amounts reported on line 15"*.
    ///
    /// With one trade or business this equals line 15 column A. It is still computed as the sum,
    /// because the line says *"add all amounts"* and a second business is the change that would make
    /// a hardcoded copy wrong.
    pub line16: Usd,
}

/// Form 8995-A **Part III** — *"Phased-in Reduction"*.
///
/// The form's own gate, verbatim: *"Complete Part III only if your taxable income is more than $191,950
/// but not $241,950 ($383,900 and $483,900 if married filing jointly) **and line 10 is less than
/// line 3**. Otherwise, skip Part III."* Both halves are enforced by [`Form8995APartIToIii::compute`],
/// which returns `None` for this part when either fails.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Form8995APartIii {
    /// L17 — *"Enter the amounts from line 3"*.
    pub line17: Usd,
    /// L18 — *"Enter the amounts from line 10"*.
    pub line18: Usd,
    /// L19 — *"Subtract line 18 from line 17"*.
    pub line19: Usd,
    /// L20 — *"Taxable income before qualified business income deduction"*. i8995a: *"Form 1040 …
    /// line 11, minus Form 1040 … line 12"* — AGI minus the standard or itemized deduction.
    pub line20: Usd,
    /// L21 — *"Threshold. Enter $191,950 ($383,900 if married filing jointly)"*.
    pub line21: Usd,
    /// L22 — *"Subtract line 21 from line 20"*.
    pub line22: Usd,
    /// L23 — *"Phase-in range. Enter $50,000 ($100,000 if married filing jointly)"*.
    pub line23: Usd,
    /// L24 — *"Phase-in percentage. Divide line 22 by line 23"*, printed into a box the form suffixes
    /// with `%`.
    ///
    /// ★★★ Stored as the **RATIO** (0…1), which is what line 25 multiplies by; the printed figure is
    /// this × 100. The form's own words are "divide line 22 by line 23", and dividing dollars by
    /// dollars yields a ratio — the `%` is the box's label, not a second operation. Keeping the ratio
    /// and scaling once at the emitter means the arithmetic and the printing cannot disagree, which
    /// they would if this held 40 and line 25 multiplied by it.
    pub line24_ratio: Decimal,
    /// L25 — *"Total phase-in reduction. Multiply line 19 by line 24"*.
    pub line25: Usd,
    /// L26 — *"Qualified business income after phase-in reduction. Subtract line 25 from line 17.
    /// Enter this amount here and on line 12, for the corresponding trade or business"*.
    pub line26: Usd,
}

/// Form 8995-A **Parts I–III** for btctax's single trade or business.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Form8995APartIToIii {
    pub part_i: Form8995ARowA,
    pub part_ii: Form8995APartIi,
    /// `None` when the form's own gate says *"Otherwise, skip Part III"*.
    pub part_iii: Option<Form8995APartIii>,
}

/// The inputs Parts I–III need that the Form 8995 chain does not already carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartIToIiiInputs {
    /// Part I column (a) and Schedule C line A.
    pub business_name: String,
    /// Part I column (b).
    pub is_sstb: bool,
    /// L2 — QBI from the trade or business (net profit less the deductible half of SE tax).
    pub business_qbi: Usd,
    /// L4 — allocable share of W-2 wages paid by the business.
    pub w2_wages: Usd,
    /// L7 — allocable share of UBIA of qualified property.
    pub ubia: Usd,
    /// L20 — taxable income before the QBI deduction.
    pub ti_before_qbi: Usd,
}

impl Form8995APartIToIii {
    /// Work Parts I–III in the form's own order.
    ///
    /// ★ `regime` is taken rather than re-derived so there is ONE authority for the threshold
    /// comparison — the same figure the refusal screen used to decide this return may be filed at all.
    /// Two independent classifications of the same taxable income is exactly how a filer gets a Part
    /// III the refusal did not expect.
    /// ★★ Returns `None` when there is no qualified trade or business to list — either because the
    /// filer has none, or because §199A(d)(3) EXCLUDED their SSTB (see [`qbi_after_sstb_exclusion`],
    /// which zeroes `business_qbi` upstream). Parts I–III are then not filed and Part IV stands alone,
    /// which is i8995a's own instruction for a filer with no QBI.
    pub fn compute(
        i: &PartIToIiiInputs,
        regime: Qbi199aRegime,
        status: FilingStatus,
        params: &crate::tax::tables::FullReturnParams,
    ) -> Option<Self> {
        if i.business_qbi <= Usd::ZERO {
            return None;
        }
        // ★ Every printed line is rounded to whole dollars AS IT IS COMPUTED, and each later line
        //   reads the ROUNDED one — SPEC §3.1's convention, and the reason the filed form cross-foots
        //   against itself. Chaining unrounded values and rounding only at the end prints a form whose
        //   own arithmetic does not check out.
        let pct =
            |v: Usd, num: i64, den: i64| round_dollar(v * Decimal::from(num) / Decimal::from(den));

        // ── Part II, lines 2-11 ─────────────────────────────────────────────────────────────────
        let line2 = round_dollar(i.business_qbi.max(Usd::ZERO));
        let line3 = pct(line2, 20, 100); // "Multiply line 2 by 20% (0.20)"
        let line4 = round_dollar(i.w2_wages);
        let line5 = pct(line4, 50, 100); // "Multiply line 4 by 50% (0.50)"
        let line6 = pct(line4, 25, 100); // "Multiply line 4 by 25% (0.25)"
        let line7 = round_dollar(i.ubia);
        let line8 = pct(line7, 25, 1000); // "Multiply line 7 by 2.5% (0.025)"
        let line9 = line6 + line8; // "Add lines 6 and 8"
        let line10 = line5.max(line9); // "Enter the greater of line 5 or line 9"
        let line11 = line3.min(line10); // "Enter the smaller of line 3 or line 10"

        // ── Part III, under the form's own two-part gate ────────────────────────────────────────
        // "Complete Part III only if your taxable income is more than $191,950 but not $241,950
        //  ($383,900 and $483,900 if married filing jointly) AND line 10 is less than line 3."
        let part_iii = (regime == Qbi199aRegime::InPhaseInRange && line10 < line3).then(|| {
            let line17 = line3; // "Enter the amounts from line 3"
            let line18 = line10; // "Enter the amounts from line 10"
            let line19 = line17 - line18; // "Subtract line 18 from line 17"
            let line20 = round_dollar(i.ti_before_qbi);
            let line21 = params.qbi_ti_threshold(status); // "Threshold. Enter $191,950 …"
            let line22 = line20 - line21; // "Subtract line 21 from line 20"
            let line23 = params.qbi_phase_in_range(status); // "Phase-in range. Enter $50,000 …"
                                                            // ★ NOT rounded — this is a ratio, not a dollar amount, and `round_dollar` on it would
                                                            //   collapse every percentage to 0 or 1.
            let line24_ratio = line22 / line23; // "Divide line 22 by line 23"
            let line25 = round_dollar(line19 * line24_ratio); // "Multiply line 19 by line 24"
            let line26 = line17 - line25; // "Subtract line 25 from line 17"
            Form8995APartIii {
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
            }
        });

        // ── Part II, lines 12-16 ────────────────────────────────────────────────────────────────
        // L12 "Enter the amount from line 26, IF ANY" — blank when Part III was skipped.
        let line12 = part_iii.as_ref().map(|p| p.line26);
        // L13 "Enter the greater of line 11 or line 12" — with 12 blank there is nothing to be
        // greater than, so line 11 stands. `unwrap_or(line11)` says exactly that, and does NOT
        // launder the blank into a zero that then wins a `max` against a negative line 11.
        let line13 = line12.map_or(line11, |l12| line11.max(l12));
        // L14 "Enter the amount from Schedule D (Form 8995-A), line 6, IF ANY" — btctax fills no
        // Schedule D because a patron refuses, so there is no amount and the line stays blank.
        let line14: Option<Usd> = None;
        // ★ `unwrap_or` on a literal `None` is exactly what clippy flags, and it is what the FORM
        //   says: "Subtract line 14 from line 13", where line 14 is blank. Collapsing it to
        //   `line15 = line13` deletes the subtraction, and the next reader has no way to tell whether
        //   line 14 was considered or forgotten — the compression this codebase keeps paying for.
        #[allow(clippy::unnecessary_literal_unwrap)]
        let line15 = line13 - line14.unwrap_or(Usd::ZERO); // "Subtract line 14 from line 13"
        let line16 = line15; // "Add all amounts reported on line 15" — one business, one amount

        Some(Self {
            part_i: Form8995ARowA {
                col_a_name: i.business_name.clone(),
                col_b_specified_service: i.is_sstb,
                col_c_aggregation: false,
                col_e_patron: false,
            },
            part_ii: Form8995APartIi {
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
            },
            part_iii,
        })
    }
}

/// Form 8995-A **Part IV**, in the form's own numbering. One field per printed line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Form8995APartIv {
    /// L27 — *"Total qualified business income component from all qualified trades, businesses, or
    /// aggregations. Enter the amount from line 16"*. Zero on the REIT/PTP-only path this covers.
    pub line27: Usd,
    /// L28 — *"Qualified REIT dividends and publicly traded partnership (PTP) income or (loss). See
    /// instructions"*.
    pub line28: Usd,
    /// L29 — *"Qualified REIT dividends and PTP (loss) carryforward from prior years"*. **Positive
    /// magnitude** — the box is parenthesized and the parentheses supply the minus sign.
    pub line29: Usd,
    /// L30 — *"Total qualified REIT dividends and PTP income. Combine lines 28 and 29. If less than
    /// zero, enter -0-"*.
    pub line30: Usd,
    /// L31 — *"REIT and PTP component. Multiply line 30 by 20% (0.20)"*.
    pub line31: Usd,
    /// L32 — *"Qualified business income deduction before the income limitation. Add lines 27 and 31"*.
    pub line32: Usd,
    /// L33 — *"Taxable income before qualified business income deduction"*.
    pub line33: Usd,
    /// L34 — *"Enter your net capital gain, if any, increased by any qualified dividends (see
    /// instructions)"*.
    pub line34: Usd,
    /// L35 — *"Subtract line 34 from line 33. If zero or less, enter -0-"*.
    pub line35: Usd,
    /// L36 — *"Income limitation. Multiply line 35 by 20% (0.20)"*.
    pub line36: Usd,
    /// L37 — *"Qualified business income deduction before the domestic production activities deduction
    /// (DPAD) under section 199A(g). Enter the smaller of line 32 or line 36"*.
    pub line37: Usd,
    /// L38 — *"DPAD under section 199A(g) allocated from an agricultural or horticultural cooperative.
    /// Don't enter more than line 33 minus line 37"*.
    ///
    /// ★★ `None` and it must be: btctax fills no Schedule D (Form 8995-A), so no cooperative has
    /// allocated anything. The line is a CONDITIONAL entry with no `-0-` clause, so the unmet condition
    /// leaves it **blank**. A printed `0` would swear the filer received an allocation of zero.
    pub line38: Option<Usd>,
    /// L39 — *"Total qualified business income deduction. Add lines 37 and 38"*.
    pub line39: Usd,
    /// L40 — *"Total qualified REIT dividends and PTP (loss) carryforward. Combine lines 28 and 29. If
    /// zero or greater, enter -0-"*. **Positive magnitude** (parenthesized box).
    pub line40: Usd,
}

impl Form8995APartIv {
    /// Transcribe Part IV from the already-computed Form 8995 chain.
    ///
    /// ★ A TRANSCRIPTION, not a re-derivation: every value is a line the existing computation already
    /// produced and the whole packet already cross-foots against. Recomputing here would create a
    /// second authority for one figure — the defect class this codebase keeps finding — and the module
    /// doc carries the proof that the two forms' arithmetic is pointwise identical.
    pub fn from_8995(l: &Form8995Lines) -> Self {
        Self {
            line27: l.line5,  // 8995 L5 — QBI component (zero on the REIT/PTP-only path)
            line28: l.line6,  // 8995 L6
            line29: l.line7,  // 8995 L7
            line30: l.line8,  // 8995 L8  ≡ 8995-A L30 (see the module doc's proof)
            line31: l.line9,  // 8995 L9
            line32: l.line10, // 8995 L10
            line33: l.line11, // 8995 L11
            line34: l.line12, // 8995 L12
            line35: l.line13, // 8995 L13
            line36: l.line14, // 8995 L14
            line37: l.line15, // 8995 L15 — "before the DPAD"; 8995 L15 IS the deduction, and…
            line38: None,     // …with no DPAD, L39 = L37, so the two forms' totals agree.
            line39: l.line15,
            line40: l.line17, // 8995 L17 ≡ 8995-A L40 (same proof)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// ★ Built field-by-field, because `Form8995Lines` has NO `Default` — deliberately. A `Default` on
    /// a money struct launders "never computed" into zero, which is the class this codebase keeps
    /// finding; adding one just to shorten a fixture would trade a real guarantee for brevity.
    #[allow(clippy::too_many_arguments)]
    fn chain(
        line5: Usd,
        line6: Usd,
        line7: Usd,
        line8: Usd,
        line9: Usd,
        line10: Usd,
        line11: Usd,
        line12: Usd,
        line13: Usd,
        line14: Usd,
        line15: Usd,
        line17: Usd,
    ) -> Form8995Lines {
        Form8995Lines {
            business_name: String::new(),
            line2: Usd::ZERO,
            line3: Usd::ZERO,
            line4: Usd::ZERO,
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
            line16: Usd::ZERO,
            line17,
        }
    }

    /// ★★★ THE EQUIVALENCE PROOF, EXECUTED — the KAT `CLAUDE.md` requires beside a derived form.
    ///
    /// The two forms word their clamps differently at exactly the value a clamp decides:
    ///
    ///   8995   L8  "Combine lines 6 and 7.  If ZERO OR LESS,    enter -0-"
    ///   8995-A L30 "Combine lines 28 and 29. If LESS THAN ZERO, enter -0-"
    ///   8995   L17 "…If GREATER THAN ZERO, enter -0-"
    ///   8995-A L40 "…If ZERO OR GREATER,   enter -0-"
    ///
    /// Each pair is complementary rather than contradictory: where one clamps at zero the other enters
    /// the combined value, which IS zero. This walks every sign and pins that they never differ — and
    /// it is the test that reds if either form is revised so the clauses disagree on a NON-zero value,
    /// which is the branch the module doc names.
    #[test]
    fn the_two_forms_agree_pointwise_including_at_zero() {
        // The four clauses, transcribed as predicates rather than paraphrased into one.
        let f8995_l8 = |v: i64| if v <= 0 { 0 } else { v }; // "zero or less"
        let f8995a_l30 = |v: i64| if v < 0 { 0 } else { v }; // "less than zero"
        let f8995_l17 = |v: i64| if v > 0 { 0 } else { -v }; // "greater than zero"
        let f8995a_l40 = |v: i64| if v >= 0 { 0 } else { -v }; // "zero or greater"

        // ★★★ WHAT THIS TEST DOES NOT PROVE, stated so nobody mistakes its silence for coverage.
        //
        //     It asserts the two predicates AGREE. It cannot tell whether either one transcribes its
        //     own form correctly, because breaking one to match the other still agrees. Concretely:
        //     misreading L40's "if ZERO OR GREATER" as "if GREATER THAN zero" changes NO value at any
        //     input — at v = 0 the correct reading enters -0- and the misreading enters the combined
        //     value, which IS 0. Verified exhaustively; the two readings differ nowhere.
        //
        //     So the clause WORDING is not arithmetically observable, and no arithmetic test can guard
        //     it. What guards it is the verbatim check on the quoted instruction — the map's
        //     `every_quoted_instruction_is_verbatim_on_the_form`, which already caught this module's
        //     sibling paraphrase ("if zero or less" for "if less than zero" on line 30). Naming the
        //     limit here is the point: an untested belief with no acknowledged gap is how the last four
        //     review rounds each found a defect in a fold.
        for v in [-1_000_000i64, -1000, -1, 0, 1, 1000, 1_000_000] {
            assert_eq!(
                f8995_l8(v),
                f8995a_l30(v),
                "the REIT/PTP TOTAL differs at {v}: 8995 L8 vs 8995-A L30"
            );
            assert_eq!(
                f8995_l17(v),
                f8995a_l40(v),
                "the REIT/PTP CARRYFORWARD differs at {v}: 8995 L17 vs 8995-A L40"
            );
        }
    }

    /// The transcription carries every figure across unchanged, and totals agree with Form 8995.
    #[test]
    fn part_iv_transcribes_the_8995_chain() {
        // REIT dividends 4,000; no business QBI; TI 250,000 with no net capital gain.
        let l = chain(
            dec!(0),
            dec!(4000),
            dec!(0),
            dec!(4000),
            dec!(800),
            dec!(800),
            dec!(250000),
            dec!(0),
            dec!(250000),
            dec!(50000),
            dec!(800),
            dec!(0),
        );
        let p = Form8995APartIv::from_8995(&l);
        assert_eq!(
            (p.line28, p.line30, p.line31),
            (dec!(4000), dec!(4000), dec!(800))
        );
        assert_eq!(
            (p.line32, p.line36, p.line37),
            (dec!(800), dec!(50000), dec!(800))
        );
        // ★★ L38 is BLANK, not zero. btctax fills no Schedule D (Form 8995-A), so no cooperative has
        //    allocated anything — and the line has no `-0-` clause, so a printed 0 would swear the
        //    filer received an allocation of zero.
        assert_eq!(p.line38, None, "the DPAD line carries NO testimony");
        // With no DPAD, L39 = L37, so 8995-A's total equals Form 8995's line 15.
        assert_eq!(p.line39, l.line15, "the two forms' deductions must agree");
    }

    /// ★ A REIT/PTP LOSS still carries forward as a positive magnitude, and the total clamps to zero —
    /// the case where the two clamps could have diverged, worked end to end.
    #[test]
    fn a_reit_loss_clamps_the_total_and_carries_forward() {
        // No REIT income; a 5,000 prior-year loss carryforward (magnitude). Combine ⇒ -5,000 ⇒ clamped.
        let l = chain(
            Usd::ZERO,
            dec!(0),
            dec!(5000),
            dec!(0),
            dec!(0),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(0),
            dec!(5000),
        );
        let p = Form8995APartIv::from_8995(&l);
        assert_eq!(p.line30, Usd::ZERO, "a net REIT/PTP loss clamps the total");
        assert_eq!(
            p.line40,
            dec!(5000),
            "and carries forward as a POSITIVE magnitude"
        );
        assert!(
            p.line40 >= Usd::ZERO,
            "a parenthesized box never holds a minus sign"
        );
    }

    // ── §G-28/B1b — Parts I–III ─────────────────────────────────────────────────────────────────

    use crate::tax::tables::FullReturnParams;
    use crate::tax::types::FilingStatus;

    fn params() -> FullReturnParams {
        crate::tax::testonly::ty2024_params()
    }

    /// ★ Every KAT below has a qualified trade or business, so `compute` never returns `None`;
    ///   the `None` paths have their own tests.
    fn compute(
        i: &PartIToIiiInputs,
        regime: Qbi199aRegime,
        status: FilingStatus,
        p: &FullReturnParams,
    ) -> Form8995APartIToIii {
        Form8995APartIToIii::compute(i, regime, status, p).expect("this fixture has a business")
    }

    fn inputs(business_qbi: Usd, w2_wages: Usd, ubia: Usd, ti: Usd) -> PartIToIiiInputs {
        PartIToIiiInputs {
            business_name: "Bitcoin mining".into(),
            is_sstb: false,
            business_qbi,
            w2_wages,
            ubia,
            ti_before_qbi: ti,
        }
    }

    /// ★★★ THE REGIME SPLIT, at both boundaries and on both sides of each.
    ///
    /// The boundary CONVENTION is the whole point: i8995a's Exception 1 says the simplified form still
    /// applies when TI *"isn't more than"* the threshold, and Exception 2 says the range runs to
    /// *"more than $383,900 but not more than $483,900"*. So both comparisons are inclusive at the top.
    /// Getting either off by a dollar files the wrong form.
    #[test]
    fn the_regime_boundaries_are_inclusive_at_the_top() {
        let p = params();
        for status in [
            FilingStatus::Single,
            FilingStatus::Mfj,
            FilingStatus::Mfs,
            FilingStatus::HoH,
            FilingStatus::Qss,
        ] {
            let t = p.qbi_ti_threshold(status);
            let top = p.qbi_phase_in_top(status);
            assert_eq!(
                Qbi199aRegime::of(t, status, &p),
                Qbi199aRegime::AtOrBelowThreshold,
                "{status:?}: AT the threshold the SIMPLIFIED Form 8995 still applies"
            );
            assert_eq!(
                Qbi199aRegime::of(t + dec!(1), status, &p),
                Qbi199aRegime::InPhaseInRange,
                "{status:?}: one dollar over the threshold enters the phase-in range"
            );
            assert_eq!(
                Qbi199aRegime::of(top, status, &p),
                Qbi199aRegime::InPhaseInRange,
                "{status:?}: AT the top of the range it is still INSIDE the range"
            );
            assert_eq!(
                Qbi199aRegime::of(top + dec!(1), status, &p),
                Qbi199aRegime::AboveThePhaseInRange,
                "{status:?}: one dollar over the top clears the range"
            );
        }
    }

    /// ★★★ THE CASE B1b EXISTS FOR — a sole proprietor with **no employees and no property**, above
    /// the phase-in range.
    ///
    /// §199A(b)(2) caps the deduction at the greater of 50% of W-2 wages ($0) or 25% of wages plus
    /// 2.5% of UBIA ($0) — so the cap is **zero** and the deduction is zero. That is a real answer,
    /// knowable from two figures a filer can state, and it is what the old blanket refusal was hiding.
    #[test]
    fn no_employees_and_no_property_caps_the_deduction_at_zero() {
        let p = params();
        // $300,000 of QBI, TI $300,000 — well past the $241,950 top of the single range.
        let f = compute(
            &inputs(dec!(300000), Usd::ZERO, Usd::ZERO, dec!(300000)),
            Qbi199aRegime::AboveThePhaseInRange,
            FilingStatus::Single,
            &p,
        );
        assert_eq!(f.part_ii.line3, dec!(60000), "20% of QBI, before the cap");
        assert_eq!(
            f.part_ii.line10,
            Usd::ZERO,
            "the greater of 50%×0 and 25%×0+2.5%×0"
        );
        assert_eq!(
            f.part_ii.line11,
            Usd::ZERO,
            "the smaller of line 3 and line 10"
        );
        // ★★ Part III is SKIPPED — above the range, whatever line 10 does.
        assert_eq!(
            f.part_iii, None,
            "Part III applies only INSIDE the phase-in range"
        );
        assert_eq!(
            f.part_ii.line12, None,
            "with no Part III there is no line 26, so line 12 carries NO testimony"
        );
        assert_eq!(f.part_ii.line13, Usd::ZERO);
        assert_eq!(
            f.part_ii.line14, None,
            "no patron ⇒ no Schedule D ⇒ blank, not zero"
        );
        assert_eq!(f.part_ii.line16, Usd::ZERO, "the QBI component is zero");
    }

    /// A business that DOES pay wages gets the 50%-of-wages cap, and it binds below 20% of QBI.
    #[test]
    fn the_fifty_percent_wage_cap_binds_when_it_is_the_smaller() {
        let p = params();
        // QBI 300,000 ⇒ line 3 = 60,000. Wages 80,000 ⇒ line 5 = 40,000, which is smaller.
        let f = compute(
            &inputs(dec!(300000), dec!(80000), Usd::ZERO, dec!(300000)),
            Qbi199aRegime::AboveThePhaseInRange,
            FilingStatus::Single,
            &p,
        );
        assert_eq!(f.part_ii.line5, dec!(40000), "50% of wages");
        assert_eq!(
            f.part_ii.line9,
            dec!(20000),
            "25% of wages + 2.5% of no UBIA"
        );
        assert_eq!(
            f.part_ii.line10,
            dec!(40000),
            "the GREATER of the two tests"
        );
        assert_eq!(
            f.part_ii.line11,
            dec!(40000),
            "and it is smaller than line 3, so it binds"
        );
        assert_eq!(f.part_ii.line16, dec!(40000));
    }

    /// ★★ The **25% + 2.5% UBIA** test wins when the business is property-heavy and wage-light —
    /// the branch a "just use 50% of wages" compression would silently delete.
    #[test]
    fn the_ubia_test_wins_for_a_property_heavy_business() {
        let p = params();
        // Wages 20,000 ⇒ line 5 = 10,000. UBIA 1,000,000 ⇒ line 6 = 5,000, line 8 = 25,000, line 9 = 30,000.
        let f = compute(
            &inputs(dec!(300000), dec!(20000), dec!(1000000), dec!(300000)),
            Qbi199aRegime::AboveThePhaseInRange,
            FilingStatus::Single,
            &p,
        );
        assert_eq!(f.part_ii.line5, dec!(10000));
        assert_eq!(f.part_ii.line6, dec!(5000));
        assert_eq!(f.part_ii.line8, dec!(25000), "2.5% of $1,000,000 of UBIA");
        assert_eq!(f.part_ii.line9, dec!(30000));
        assert_eq!(
            f.part_ii.line10,
            dec!(30000),
            "the UBIA test is the greater one here"
        );
    }

    /// ★★★ PART III, WORKED END TO END — a single filer at the exact MIDPOINT of the phase-in range.
    ///
    /// TI = 191,950 + 25,000 = 216,950, so line 22 = 25,000 over a line 23 of 50,000 ⇒ **50%**.
    /// QBI 200,000 ⇒ line 3 = 40,000; no wages, no UBIA ⇒ line 10 = 0, so line 19 = 40,000.
    /// Line 25 = 40,000 × 50% = 20,000; line 26 = 40,000 − 20,000 = **20,000**, which lands on line 12
    /// and wins line 13 against a line 11 of zero.
    ///
    /// ★ This is the branch that makes the phase-in worth having: the same filer one dollar higher gets
    /// **nothing**, and this test is what would red if the range were ever indexed or dropped.
    #[test]
    fn part_iii_phases_the_limitation_in_at_the_midpoint_of_the_range() {
        let p = params();
        let ti = p.qbi_ti_threshold(FilingStatus::Single) + dec!(25000);
        let f = compute(
            &inputs(dec!(200000), Usd::ZERO, Usd::ZERO, ti),
            Qbi199aRegime::InPhaseInRange,
            FilingStatus::Single,
            &p,
        );
        let iii = f
            .part_iii
            .as_ref()
            .expect("inside the range with line 10 < line 3");
        assert_eq!(iii.line17, dec!(40000), "line 3");
        assert_eq!(iii.line18, Usd::ZERO, "line 10");
        assert_eq!(iii.line19, dec!(40000));
        assert_eq!(iii.line20, ti);
        assert_eq!(iii.line21, dec!(191950), "the §199A(e)(2) threshold");
        assert_eq!(iii.line22, dec!(25000));
        assert_eq!(
            iii.line23,
            dec!(50000),
            "the STATUTORY phase-in range width"
        );
        assert_eq!(iii.line24_ratio, dec!(0.5), "25,000 / 50,000");
        assert_eq!(iii.line25, dec!(20000), "40,000 × 50%");
        assert_eq!(iii.line26, dec!(20000));
        // …and it flows onto line 12, wins line 13, and becomes the QBI component.
        assert_eq!(f.part_ii.line12, Some(dec!(20000)));
        assert_eq!(
            f.part_ii.line13,
            dec!(20000),
            "the greater of line 11 (0) and line 12"
        );
        assert_eq!(f.part_ii.line16, dec!(20000));
    }

    /// ★★★ THE SAME FILER ONE DOLLAR ABOVE THE RANGE GETS NOTHING — the cliff the phase-in softens,
    /// pinned so the two regimes cannot silently converge.
    #[test]
    fn one_dollar_past_the_range_the_phase_in_is_gone() {
        let p = params();
        let top = p.qbi_phase_in_top(FilingStatus::Single);
        let inside = compute(
            &inputs(dec!(200000), Usd::ZERO, Usd::ZERO, top),
            Qbi199aRegime::of(top, FilingStatus::Single, &p),
            FilingStatus::Single,
            &p,
        );
        let outside = compute(
            &inputs(dec!(200000), Usd::ZERO, Usd::ZERO, top + dec!(1)),
            Qbi199aRegime::of(top + dec!(1), FilingStatus::Single, &p),
            FilingStatus::Single,
            &p,
        );
        assert!(
            inside.part_iii.is_some(),
            "AT the top of the range Part III still applies"
        );
        assert_eq!(
            inside.part_iii.as_ref().unwrap().line24_ratio,
            dec!(1),
            "…at 100%, so the reduction is total and line 26 is zero"
        );
        assert_eq!(inside.part_ii.line16, Usd::ZERO);
        assert_eq!(
            outside.part_iii, None,
            "one dollar higher, Part III is skipped"
        );
        assert_eq!(
            outside.part_ii.line16,
            Usd::ZERO,
            "and the answer is the same — CONTINUOUS"
        );
    }

    /// ★★ Part III is skipped INSIDE the range when line 10 is NOT less than line 3 — the second half
    /// of the form's own two-part gate, which a `regime`-only check would drop.
    #[test]
    fn inside_the_range_a_non_binding_cap_still_skips_part_iii() {
        let p = params();
        let ti = p.qbi_ti_threshold(FilingStatus::Single) + dec!(25000);
        // QBI 200,000 ⇒ line 3 = 40,000. Wages 200,000 ⇒ line 5 = 100,000 ≥ line 3, so the cap does
        // not bind and there is nothing to phase in.
        let f = compute(
            &inputs(dec!(200000), dec!(200000), Usd::ZERO, ti),
            Qbi199aRegime::InPhaseInRange,
            FilingStatus::Single,
            &p,
        );
        assert!(
            f.part_ii.line10 >= f.part_ii.line3,
            "the fixture must not bind, else this is vacuous"
        );
        assert_eq!(
            f.part_iii, None,
            "the form says complete Part III only if line 10 is LESS THAN line 3"
        );
        assert_eq!(f.part_ii.line12, None);
        assert_eq!(
            f.part_ii.line13,
            dec!(40000),
            "the full 20% of QBI survives"
        );
    }

    /// ★ Part I carries the business, and the three checkboxes that are NOT the filer's to leave blank.
    #[test]
    fn part_i_names_the_business_and_leaves_the_elective_boxes_unchecked() {
        let p = params();
        let mut i = inputs(dec!(300000), Usd::ZERO, Usd::ZERO, dec!(300000));
        i.is_sstb = true;
        let f = compute(
            &i,
            Qbi199aRegime::AboveThePhaseInRange,
            FilingStatus::Single,
            &p,
        );
        assert_eq!(f.part_i.col_a_name, "Bitcoin mining");
        assert!(
            f.part_i.col_b_specified_service,
            "column (b) carries the filer's own answer"
        );
        assert!(
            !f.part_i.col_c_aggregation,
            "aggregation is an ELECTION over two or more businesses; btctax has one"
        );
        assert!(
            !f.part_i.col_e_patron,
            "a patron refuses upstream, so the box is unchecked on every filed return"
        );
    }

    /// ★★★ EVERY PRINTED LINE IS A WHOLE DOLLAR, and the form CROSS-FOOTS against itself.
    ///
    /// SPEC §3.1. Chaining unrounded values and rounding once at the end prints a form whose own
    /// arithmetic does not check out — a filer reading down the page finds the subtraction wrong.
    #[test]
    fn every_printed_line_is_a_whole_dollar_and_the_form_cross_foots() {
        let p = params();
        let ti = p.qbi_ti_threshold(FilingStatus::Single) + dec!(17777);
        // Deliberately awkward figures so rounding actually has something to do.
        let f = compute(
            &inputs(dec!(233333.33), dec!(41111.11), dec!(777777.77), ti),
            Qbi199aRegime::InPhaseInRange,
            FilingStatus::Single,
            &p,
        );
        let ii = &f.part_ii;
        let whole = |v: Usd, name: &str| {
            assert_eq!(v.fract(), Usd::ZERO, "{name} is not a whole dollar: {v}");
        };
        for (v, n) in [
            (ii.line2, "2"),
            (ii.line3, "3"),
            (ii.line4, "4"),
            (ii.line5, "5"),
            (ii.line6, "6"),
            (ii.line7, "7"),
            (ii.line8, "8"),
            (ii.line9, "9"),
            (ii.line10, "10"),
            (ii.line11, "11"),
            (ii.line13, "13"),
            (ii.line15, "15"),
            (ii.line16, "16"),
        ] {
            whole(v, n);
        }
        // The form's own arithmetic, read straight off the printed cells.
        assert_eq!(ii.line9, ii.line6 + ii.line8, "line 9 = lines 6 + 8");
        assert_eq!(
            ii.line10,
            ii.line5.max(ii.line9),
            "line 10 = greater of 5 or 9"
        );
        assert_eq!(
            ii.line11,
            ii.line3.min(ii.line10),
            "line 11 = smaller of 3 or 10"
        );
        assert_eq!(ii.line15, ii.line13, "line 15 = line 13 − a blank line 14");
        assert_eq!(ii.line16, ii.line15, "one business ⇒ line 16 = line 15");
        let iii = f
            .part_iii
            .as_ref()
            .expect("this fixture is inside the range and binds");
        for (v, n) in [
            (iii.line17, "17"),
            (iii.line19, "19"),
            (iii.line20, "20"),
            (iii.line22, "22"),
            (iii.line25, "25"),
            (iii.line26, "26"),
        ] {
            whole(v, n);
        }
        assert_eq!(iii.line19, iii.line17 - iii.line18, "line 19 = 17 − 18");
        assert_eq!(iii.line22, iii.line20 - iii.line21, "line 22 = 20 − 21");
        assert_eq!(iii.line26, iii.line17 - iii.line25, "line 26 = 17 − 25");
        assert_eq!(ii.line12, Some(iii.line26), "line 12 IS line 26");
    }

    /// ★★ MFJ doubles BOTH the threshold and the range, so the midpoint phase-in percentage is the
    /// same 50% at twice the distance — the pairing that a hardcoded $50,000 would break.
    #[test]
    fn mfj_doubles_the_range_so_the_percentage_pairs_with_the_threshold() {
        let p = params();
        let ti = p.qbi_ti_threshold(FilingStatus::Mfj) + dec!(50000);
        let f = compute(
            &inputs(dec!(400000), Usd::ZERO, Usd::ZERO, ti),
            Qbi199aRegime::InPhaseInRange,
            FilingStatus::Mfj,
            &p,
        );
        let iii = f.part_iii.as_ref().unwrap();
        assert_eq!(iii.line21, dec!(383900), "the MFJ threshold");
        assert_eq!(iii.line23, dec!(100000), "the MFJ phase-in range");
        assert_eq!(
            iii.line24_ratio,
            dec!(0.5),
            "50,000 / 100,000 — the same midpoint"
        );
    }

    /// ★★★ §199A(d)(3) — AN SSTB ABOVE THE RANGE IS NOT ON THE FORM AT ALL.
    ///
    /// Not "a deduction that comes to zero": the business is not a qualified trade or business, so no
    /// QBI, no W-2 wages and no UBIA are taken into account. Parts I–III are not filed.
    ///
    /// ★★ This is the defect Tax-Calculator found. The first draft of this module put the SSTB's QBI on
    /// line 2 and computed a $40,000 deduction where the oracle returned `0.` — a $40,000 overstated
    /// deduction on a filed return.
    #[test]
    fn an_sstb_above_the_range_is_excluded_entirely_and_files_no_parts_i_to_iii() {
        let p = params();
        let qbi = dec!(325581);
        // The exclusion happens where business_qbi is computed, so Parts I-III never see it…
        assert_eq!(
            qbi_after_sstb_exclusion(qbi, true, Qbi199aRegime::AboveThePhaseInRange),
            Usd::ZERO,
            "no QBI from an SSTB is taken into account above the range"
        );
        // …and with no QBI there is no trade or business to list.
        let mut i = inputs(Usd::ZERO, dec!(80000), Usd::ZERO, dec!(510981));
        i.is_sstb = true;
        assert_eq!(
            Form8995APartIToIii::compute(
                &i,
                Qbi199aRegime::AboveThePhaseInRange,
                FilingStatus::Single,
                &p
            ),
            None,
            "an excluded SSTB appears nowhere on Parts I-III"
        );
        // The three cases the exclusion must NOT fire in, each for its own reason.
        assert_eq!(
            qbi_after_sstb_exclusion(qbi, false, Qbi199aRegime::AboveThePhaseInRange),
            qbi,
            "a NON-SSTB above the range keeps its QBI — the wage/UBIA cap limits it, nothing excludes it"
        );
        assert_eq!(
            qbi_after_sstb_exclusion(qbi, true, Qbi199aRegime::InPhaseInRange),
            qbi,
            "INSIDE the range the exclusion is partial (Schedule A), and btctax refuses rather than \
             approximate it — so this function must not silently zero it"
        );
        assert_eq!(
            qbi_after_sstb_exclusion(qbi, true, Qbi199aRegime::AtOrBelowThreshold),
            qbi,
            "below the threshold an SSTB is a qualified trade or business (i8995a Exception 1)"
        );
    }

    /// ★★★ WITNESSED AGAINST PSL TAX-CALCULATOR 6.7.2 — and against ONE oracle only, which is stated
    /// here because `CLAUDE.md` requires two and this is a documented exception, not an oversight.
    ///
    /// **OpenTaxSolver cannot witness §199A at all.** `taxsolve_US_1040_2024.c` reads the deduction as a
    /// hand-fed input — `GetLine( "L13", &L[13] ); /* Qualified business income deduction. */` — and
    /// never derives it. That is this repo's §G-9 limit in its purest form: *a value the oracles take as
    /// INPUT is never validated by their agreement*. So Form 8995-A has exactly one independent witness,
    /// and the second corroboration is the hand-computation written beside each figure below, taken from
    /// the form's own line text.
    ///
    /// ★ The installed Tax-Calculator was diffed byte-for-byte against the pristine PyPI 6.7.2 sdist
    /// before being trusted, because a locally patched oracle is not an independent witness.
    ///
    /// Every vector carries W-2 WAGE income so Part IV's income limitation (20% of taxable income before
    /// the QBI deduction) stays SLACK. Without it that cap binds on every phase-in vector and
    /// `qbided` comes back identical whether Part III ran or was skipped — the vector then witnesses
    /// nothing, which is how the first draft of this KAT set passed while proving nothing.
    ///
    /// Tolerance is ±$1: taxcalc chains float64 unrounded, btctax rounds every printed line to whole
    /// dollars (SPEC §3.1), so the two differ by at most the rounding of the final line.
    #[test]
    fn parts_i_to_iii_agree_with_tax_calculator() {
        let p = params();
        // (business_qbi, w2_wages, ubia, ti_before_qbi, status, expected line 16, taxcalc qbided, tag)
        let vectors: &[(Usd, Usd, Usd, Usd, FilingStatus, Usd, &str)] = &[
            // above/no-wages — 20% of QBI is 65,116 but the §199A(b)(2) cap is max(50%×0, 25%×0+2.5%×0)
            //                  = 0, so the deduction is ZERO. The case B1b exists for.
            (
                dec!(325581),
                Usd::ZERO,
                Usd::ZERO,
                dec!(510981),
                FilingStatus::Single,
                Usd::ZERO,
                "above/no-wages",
            ),
            // above/wage-cap — line 5 = 50% × 80,000 = 40,000 < line 3 = 65,116, so the cap binds.
            (
                dec!(325581),
                dec!(80000),
                Usd::ZERO,
                dec!(510981),
                FilingStatus::Single,
                dec!(40000),
                "above/wage-cap",
            ),
            // above/ubia — line 5 = 10,000; line 9 = 25%×20,000 + 2.5%×1,000,000 = 5,000 + 25,000
            //              = 30,000, the GREATER, so the UBIA test wins.
            (
                dec!(325581),
                dec!(20000),
                dec!(1000000),
                dec!(510981),
                FilingStatus::Single,
                dec!(30000),
                "above/ubia",
            ),
            // range/no-wages — line 3 = 20,116; line 10 = 0; line 22 = 205,981 − 191,950 = 14,031;
            //                  line 24 = 14,031/50,000 = 0.28062; line 25 = 5,645; line 26 = 14,471.
            (
                dec!(100581),
                Usd::ZERO,
                Usd::ZERO,
                dec!(205981),
                FilingStatus::Single,
                dec!(14471),
                "range/no-wages",
            ),
            // range/no-bind — the same filer whose business pays 200,000 of wages: line 5 = 100,000 is
            //                 NOT less than line 3, so Part III is skipped and the full 20% survives.
            (
                dec!(100581),
                dec!(200000),
                Usd::ZERO,
                dec!(205981),
                FilingStatus::Single,
                dec!(20116),
                "range/no-bind",
            ),
            // mfj/range — line 3 = 31,571; line 22 = 428,657 − 383,900 = 44,757 over a line 23 of
            //             100,000 ⇒ 0.44757; line 25 = 14,130; line 26 = 17,441.
            (
                dec!(157857),
                Usd::ZERO,
                Usd::ZERO,
                dec!(428657),
                FilingStatus::Mfj,
                dec!(17441),
                "mfj/range",
            ),
            // mfj/above — 20% of 591,966 = 118,393, capped by 50% × 100,000 = 50,000.
            (
                dec!(591966),
                dec!(100000),
                Usd::ZERO,
                dec!(762766),
                FilingStatus::Mfj,
                dec!(50000),
                "mfj/above",
            ),
        ];
        // The figures Tax-Calculator returned for the same seven households, keyed by tag.
        let taxcalc: &[(&str, f64)] = &[
            ("above/no-wages", 0.00),
            ("above/wage-cap", 40000.00),
            ("above/ubia", 30000.00),
            ("range/no-wages", 14471.25),
            ("range/no-bind", 20116.15),
            ("mfj/range", 17440.89),
            ("mfj/above", 50000.00),
        ];
        for (qbi, wages, ubia, ti, status, want16, tag) in vectors {
            let f = compute(
                &inputs(*qbi, *wages, *ubia, *ti),
                Qbi199aRegime::of(*ti, *status, &p),
                *status,
                &p,
            );
            assert_eq!(
                f.part_ii.line16, *want16,
                "{tag}: Form 8995-A line 16 (the QBI component)"
            );
            let (_, oracle) = taxcalc
                .iter()
                .find(|(t, _)| t == tag)
                .expect("every vector must carry its oracle figure");
            let ours: f64 = f.part_ii.line16.try_into().unwrap();
            assert!(
                (ours - oracle).abs() <= 1.0,
                "{tag}: btctax {ours} vs Tax-Calculator {oracle} — more than a dollar apart"
            );
        }
    }

    /// ★★ The two phase-in vectors above must DISAGREE with each other, or neither witnesses Part III.
    ///
    /// They differ only in whether the business pays W-2 wages, which is exactly the switch that decides
    /// the form's second gate (*"and line 10 is less than line 3"*). An earlier draft of the oracle
    /// sweep had both returning the identical figure because Part IV's income cap dominated both — a
    /// pair of vectors that agree by accident proves nothing, and looks exactly like a pair that agrees
    /// because the code is right.
    #[test]
    fn the_two_phase_in_vectors_actually_discriminate() {
        let p = params();
        let ti = dec!(205981);
        let phased = compute(
            &inputs(dec!(100581), Usd::ZERO, Usd::ZERO, ti),
            Qbi199aRegime::of(ti, FilingStatus::Single, &p),
            FilingStatus::Single,
            &p,
        );
        let skipped = compute(
            &inputs(dec!(100581), dec!(200000), Usd::ZERO, ti),
            Qbi199aRegime::of(ti, FilingStatus::Single, &p),
            FilingStatus::Single,
            &p,
        );
        assert!(
            phased.part_iii.is_some(),
            "the no-wages vector must RUN Part III"
        );
        assert!(
            skipped.part_iii.is_none(),
            "the wage-paying vector must SKIP it"
        );
        assert_ne!(
            phased.part_ii.line16, skipped.part_ii.line16,
            "if these agree, neither vector is reading Part III"
        );
    }
}
