//! **Form 6251 (2024) — Alternative Minimum Tax—Individuals**, transcribed line by line.
//!
//! Every field below is one numbered line of the form, named for that line, carrying the official
//! printed instruction **verbatim** as its doc comment (`CLAUDE.md`, "Transcribe IRS forms").
//!
//! ★ **The converse does NOT yet hold, and that is Tier 1's boundary.** Lines **2c–2t** (18 numbered
//! adjustment lines: ISO exercise, §1202, §4952 investment interest, depreciation, NOL, passive and
//! loss-limitation items, …) have **no fields here**, and `line4 = line1 + line2a + line2b + line3`
//! treats them all as $0. Each is closed at the input surface instead — most because v1 captures no
//! such input at all, and three (lines 3, 2k, 2l) because a `FORM_QUESTIONS` declaration refuses
//! unless the filer affirms the adjustment away. The `must_attach` condition-4 argument therefore
//! rests on that input-surface audit (`amt.rs`'s module doc), **not** on this struct's shape. Tier 2,
//! which files the form rather than only computing it, must give 2c–2t real fields.
//!
//! Nothing
//! here is derived: the form never asks anyone to derive anything, it says "enter the amount from",
//! "enter the smaller of", "if X, skip to Y". The full transcription with per-line provenance is
//! `design/amt-form6251/PART_III.md`; the blank form is `btctax-forms/forms/2024/f6251.pdf`.
//!
//! **Why this shape.** The predecessor AMT code compressed the *screening worksheet* into
//! `AGI − QBI` and conflated Schedule A line **7** (taxes) with line **17** (itemized total) — a bug
//! that shipped in v0.6.0–v0.13.0. Later drafts of this module's plan dropped line 2b and the line-4
//! MFS kicker for the same reason: they were terms in a formula rather than lines on a form. As
//! fields, they cannot be dropped.
//!
//! **Ordering note (i6251 p.10).** Line 10 is computed **before** line 8: *"Before figuring your
//! AMTFTC … fill in Form 6251, line 10, as instructed. If the amount on line 10 is greater than or
//! equal to the amount on line 7 … Leave line 8 blank and enter -0- on line 11."* So the evaluation
//! order is 7 → 10 → 8 → 9 → 11, not the printed order.
//!
//! **Precision.** Exact cents, living in the absolute chain beside `AbsoluteReturn`
//! (`design/full-return/ROUNDING_AUTHORITY.md` Reading A). The *printed* form rounds per line and
//! cross-foots the rounded lines; that is the forms crate's job, not this module's.

use crate::conventions::Usd;
use crate::tax::tables::{AmtParams, LtcgBreakpoints};
use crate::tax::types::FilingStatus;
use rust_decimal::Decimal;

/// One filled Form 6251. Field names are the form's line numbers.
///
/// Lines skipped by a routing instruction (line 6's "enter -0- here and on lines 7, 9, and 11", line
/// 32's "skip lines 33 through 37", line 34's "skip lines 35 through 37") hold `0`, which is what the
/// form directs be entered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Form6251 {
    // ── Part I — Alternative Minimum Taxable Income ──────────────────────────────────────────────
    /// L1 — "Enter the amount from Form 1040 or 1040-SR, line 15, if more than zero. If Form 1040 or
    /// 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from line 11 of Form 1040
    /// or 1040-SR and enter the result here. (If less than zero, enter as a negative amount.)"
    pub line1: Usd,
    /// L2a — "If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; otherwise,
    /// enter the amount from Form 1040 or 1040-SR, line 12."
    ///
    /// ★ Line **12**, not line 14 — line 1's fallback cites line 14. They differ by the §199A QBI
    /// deduction, so a filer with QBI > 0 distinguishes them.
    ///
    /// i6251 p.2 adds: "except any generation-skipping transfer taxes on income distributions" — v1
    /// has no GST input, so nothing is excluded.
    pub line2a: Usd,
    /// L2b — "Tax refund from Schedule 1 (Form 1040), line 1 or line 8z." **A NEGATIVE amount**
    /// (i6251 p.2: "Enter the total as a negative amount"). v1 has no Schedule 1 line 8z input.
    pub line2b: Usd,
    /// L3 — "Other adjustments, including income-based related adjustments."
    ///
    /// Reaches mortgage interest on a non-qualified dwelling (§56(b)(1)(C)) and a §170(e) gift of
    /// property with a different AMT basis. They are closed by DIFFERENT mechanisms, and only the first
    /// is a declaration:
    ///   - the dwelling half is gated by `QuestionId::AmtQualifiedDwelling`, which refuses unless
    ///     answered AMT-neutral;
    ///   - the §170(e) half has **no declaration**, and needs none: btctax's only donatable property is
    ///     bitcoin, whose AMT basis is identically its regular basis (no §56 adjustment applies to a
    ///     capital asset's cost). The channel is empty rather than guarded.
    ///
    /// Either way this is `0` on every computable return.
    pub line3: Usd,
    /// L4 — "**Alternative minimum taxable income.** Combine lines 1 through 3. (If married filing
    /// separately and line 4 is more than $875,950, see instructions.)"
    ///
    /// The parenthetical is the MFS kicker; i6251 p.9 supplies the rule it points to.
    pub line4: Usd,

    // ── Part II — Alternative Minimum Tax (AMT) ──────────────────────────────────────────────────
    /// L5 — "Exemption." The status table on the form, phased out per the Exemption Worksheet.
    pub line5: Usd,
    /// L6 — "Subtract line 5 from line 4. If more than zero, go to line 7. If zero or less, enter -0-
    /// here and on lines 7, 9, and 11, and go to line 10."
    pub line6: Usd,
    /// L7 — Part III's line 40 when the form routes there, else the flat 26/28% on line 6:
    /// "**All others:** If line 6 is $232,600 or less ($116,300 or less if married filing
    /// separately), multiply line 6 by 26% (0.26). Otherwise, multiply line 6 by 28% (0.28) and
    /// subtract $4,652 ($2,326 if married filing separately) from the result."
    pub line7: Usd,
    /// L8 — "Alternative minimum tax foreign tax credit (see instructions)."
    ///
    /// i6251 p.10: blank when line 10 ≥ line 7; otherwise, for a filer electing the foreign tax
    /// credit without Form 1116, "your AMTFTC is the same as the foreign tax credit on Schedule 3
    /// (Form 1040), line 1" — which is exactly btctax's ≤$300/$600 §904(j) screen.
    pub line8: Usd,
    /// L9 — "**Tentative minimum tax.** Subtract line 8 from line 7."
    pub line9: Usd,
    /// L10 — "Add Form 1040 or 1040-SR, line 16 (minus any tax from Form 4972), and Schedule 2 (Form
    /// 1040), line 1z. Subtract from the result Schedule 3 (Form 1040), line 1 and any negative
    /// amount reported on Form 8978, line 14 (treated as a positive number). If zero or less, enter
    /// -0-. If you used Schedule J to figure your tax on Form 1040 or 1040-SR, line 16, refigure that
    /// tax without using Schedule J before completing this line."
    ///
    /// Form 4972, Form 8978 and Schedule J are inputs v1 has no surface for ⇒ each term is 0.
    pub line10: Usd,
    /// L11 — "**AMT.** Subtract line 10 from line 9. If zero or less, enter -0-. Enter here and on
    /// Schedule 2 (Form 1040), line 2."
    pub line11: Usd,

    // ── Part III — Tax Computation Using Maximum Capital Gains Rates ─────────────────────────────
    /// L12 — "Enter the amount from Form 6251, line 6."
    pub line12: Usd,
    /// L13 — "Enter the amount from line 4 of the Qualified Dividends and Capital Gain Tax Worksheet
    /// … **(as refigured for the AMT, if necessary)**." The gain's *amount*, AMT-side.
    pub line13: Usd,
    /// L14 — "Enter the amount from Schedule D (Form 1040), line 19 (as refigured for the AMT, if
    /// necessary)." Unrecaptured §1250 gain — always 0 for btctax (no §1250 property).
    pub line14: Usd,
    /// L15 — "If you did not complete a Schedule D Tax Worksheet for the regular tax or the AMT,
    /// enter the amount from line 13. Otherwise, add lines 13 and 14, and enter the smaller of that
    /// result or the amount from line 10 of the Schedule D Tax Worksheet."
    pub line15: Usd,
    /// L16 — "**Enter the smaller of line 12 or line 15.**" The cap: only `min(taxable excess, gain)`
    /// receives §1(h) rates.
    pub line16: Usd,
    /// L17 — "Subtract line 16 from line 12." The ordinary slice, floored at 0 by the cap above.
    pub line17: Usd,
    /// L18 — "If line 17 is $232,600 or less ($116,300 or less if married filing separately),
    /// multiply line 17 by 26% (0.26). Otherwise, multiply line 17 by 28% (0.28) and subtract $4,652
    /// ($2,326 if married filing separately) from the result."
    pub line18: Usd,
    /// L19 — "Enter: $94,050 if married filing jointly or qualifying surviving spouse, $47,025 if
    /// single or married filing separately, or $63,000 if head of household." Top of the 0% band.
    pub line19: Usd,
    /// L20 — "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet
    /// or the amount from line 14 of the Schedule D Tax Worksheet, whichever applies **(as figured
    /// for the regular tax)**."
    ///
    /// ★ This is the line that settles where the bands sit: **regular-side**, not AMT-side.
    pub line20: Usd,
    /// L21 — "Subtract line 20 from line 19. If zero or less, enter -0-." Unused 0% room.
    pub line21: Usd,
    /// L22 — "**Enter the smaller of line 12 or line 13.**" The second cap.
    pub line22: Usd,
    /// L23 — "Enter the smaller of line 21 or line 22. **This amount is taxed at 0%.**"
    pub line23: Usd,
    /// L24 — "Subtract line 23 from line 22."
    pub line24: Usd,
    /// L25 — "Enter: $518,900 if single, $291,850 if married filing separately, $583,750 if married
    /// filing jointly or qualifying surviving spouse, or $551,350 if head of household." Top of 15%.
    pub line25: Usd,
    /// L26 — "Enter the amount from line 21."
    pub line26: Usd,
    /// L27 — "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet
    /// or the amount from line 21 of the Schedule D Tax Worksheet, whichever applies **(as figured
    /// for the regular tax)**."
    pub line27: Usd,
    /// L28 — "Add line 26 and line 27."
    pub line28: Usd,
    /// L29 — "Subtract line 28 from line 25. If zero or less, enter -0-."
    pub line29: Usd,
    /// L30 — "Enter the smaller of line 24 or line 29." The amount taxed at 15%.
    pub line30: Usd,
    /// L31 — "Multiply line 30 by 15% (0.15)."
    pub line31: Usd,
    /// L32 — "Add lines 23 and 30. **If lines 32 and 12 are the same, skip lines 33 through 37 and go
    /// to line 38. Otherwise, go to line 33.**"
    pub line32: Usd,
    /// L33 — "**Subtract line 32 from line 22.**"
    ///
    /// ★ From **line 22**, not line 12. Transcribing this from the rendered page as "line 12" taxed
    /// the ordinary slice twice (26/28% at L18, then 20% here) and inflated TMT by $200,000 on one
    /// vector. `pdftotext -layout` is the authority, not the picture.
    pub line33: Usd,
    /// L34 — "Multiply line 33 by 20% (0.20). **If line 14 is zero or blank, skip lines 35 through 37
    /// and go to line 38. Otherwise, go to line 35.**"
    pub line34: Usd,
    /// L35 — "Add lines 17, 32, and 33." Skipped (0) whenever line 14 is zero.
    pub line35: Usd,
    /// L36 — "Subtract line 35 from line 12."
    pub line36: Usd,
    /// L37 — "Multiply line 36 by 25% (0.25)." The unrecaptured-§1250 tranche.
    pub line37: Usd,
    /// L38 — "Add lines 18, 31, 34, and 37."
    pub line38: Usd,
    /// L39 — "If line 12 is $232,600 or less ($116,300 or less if married filing separately),
    /// multiply line 12 by 26% (0.26). Otherwise, multiply line 12 by 28% (0.28) and subtract $4,652
    /// ($2,326 if married filing separately) from the result." The all-ordinary ceiling.
    pub line39: Usd,
    /// L40 — "**Enter the smaller of line 38 or line 39** here and on line 7."
    pub line40: Usd,
}

impl Form6251 {
    /// **Who Must File, condition 1** (i6251 p.1): *"Form 6251, line 7, is greater than line 10."*
    ///
    /// This — not `amt() > 0` — is the attach test. When line 7 exceeds line 10 the AMTFTC is figured
    /// (line 8), so line 9 = line 7 − FTC can still land at or below line 10: **the form is required
    /// while the AMT is $0.** Conditions 2–4 are unreachable in v1 (no general business credit, no
    /// Form 8834/8911/8801 input; condition 4 needs lines 2c–3 negative, and every one of those lines
    /// is 0 or refused).
    pub fn must_attach(&self) -> bool {
        self.line7 > self.line10
    }

    /// The AMT itself — Form 6251 line 11 → Schedule 2 (Form 1040), line 2.
    pub fn amt(&self) -> Usd {
        self.line11
    }

    /// Tentative minimum tax — Form 6251 line **9** (after the line-8 AMTFTC subtraction), which is
    /// what the form labels "Tentative minimum tax". Line 7 is the pre-credit figure.
    pub fn tentative_minimum_tax(&self) -> Usd {
        self.line9
    }
}

/// Everything Form 6251 reads off the rest of the return.
#[derive(Debug, Clone, Copy)]
pub struct Form6251Inputs {
    pub status: FilingStatus,
    /// 1040 line 15.
    pub taxable_income_l15: Usd,
    /// 1040 line 11.
    pub agi_l11: Usd,
    /// 1040 line **12** — the deduction taken (standard or itemized), WITHOUT the QBI deduction.
    /// Line 2a's else-branch reads this one.
    pub deduction_l12: Usd,
    /// 1040 line **14** — line 12 + the §199A QBI deduction. Line 1's zero-taxable-income branch
    /// reads this one. ★ Lines 2a and 1 cite DIFFERENT 1040 lines; they coincide only when QBI is 0.
    pub deduction_l14: Usd,
    /// Schedule A line 7 (capped SALT + other taxes); ignored unless `itemized`.
    pub schedule_a_line7: Usd,
    pub itemized: bool,
    /// Schedule 1 line 1 — the taxable state/local income-tax refund. Enters line 2b **negated**.
    pub state_refund_sch1_l1: Usd,
    /// §1(h) net capital gain, as refigured for the AMT (equal to the regular figure for every v1
    /// input: no depreciation, ISO or §56(a)(6) event ever touches btctax's basis).
    pub net_capital_gain: Usd,
    /// 1040 line 3a qualified dividends.
    pub qualified_dividends: Usd,
    /// QDCGT Worksheet line 5 **as figured for the regular tax** — lines 20 and 27.
    pub qdcgt_line5_regular: Usd,
    /// 1040 line 16.
    pub regular_tax_l16: Usd,
    /// Schedule 2 line 1z (excess APTC). No v1 input ⇒ 0.
    pub schedule_2_line1z: Usd,
    /// Schedule 3 line 1 — the §904(j) foreign tax credit.
    pub schedule_3_line1: Usd,
}

/// Fill Form 6251 for one return, following the form's own routing.
pub fn compute_6251(i: Form6251Inputs, amt: &AmtParams, bp: &LtcgBreakpoints) -> Form6251 {
    let z = Usd::ZERO;
    let st = i.status;

    // ── Part I ──
    let line1 = if i.taxable_income_l15 > z {
        i.taxable_income_l15
    } else {
        i.agi_l11 - i.deduction_l14 // may be negative; the form says "enter as a negative amount"
    };
    let line2a = if i.itemized {
        i.schedule_a_line7
    } else {
        i.deduction_l12 // 1040 line 12 — NOT line 14; they differ by the QBI deduction
    };
    let line2b = -i.state_refund_sch1_l1;
    let line3 = z;
    let mut line4 = line1 + line2a + line2b + line3;
    // i6251 p.9, line 4 — the MFS kicker.
    if st == FilingStatus::Mfs && line4 > amt.mfs_kicker_start {
        let excess = line4 - amt.mfs_kicker_start;
        line4 += (amt.phaseout_rate * excess).min(amt.mfs_kicker_max);
    }

    // ── Part II, lines 5-6 ──
    let exemption = amt.exemption(st);
    let phase_start = amt.phaseout_start(st);
    let line5 = (exemption - amt.phaseout_rate * (line4 - phase_start).max(z)).max(z);
    let line6 = (line4 - line5).max(z);

    let bp28 = amt.breakpoint_28pct(st);
    let sub28 = if st == FilingStatus::Mfs {
        amt.rate_28_subtrahend_mfs
    } else {
        amt.rate_28_subtrahend
    };
    // The 26/28% schedule, used identically by lines 7 ("All others"), 18 and 39.
    let flat_26_28 = |x: Usd| -> Usd {
        if x <= bp28 {
            amt.rate_26 * x
        } else {
            amt.rate_28 * x - sub28
        }
    };

    let mut f = Form6251 {
        line1,
        line2a,
        line2b,
        line3,
        line4,
        line5,
        line6,
        line7: z,
        line8: z,
        line9: z,
        line10: z,
        line11: z,
        line12: z,
        line13: z,
        line14: z,
        line15: z,
        line16: z,
        line17: z,
        line18: z,
        line19: z,
        line20: z,
        line21: z,
        line22: z,
        line23: z,
        line24: z,
        line25: z,
        line26: z,
        line27: z,
        line28: z,
        line29: z,
        line30: z,
        line31: z,
        line32: z,
        line33: z,
        line34: z,
        line35: z,
        line36: z,
        line37: z,
        line38: z,
        line39: z,
        line40: z,
    };

    // L10 is filled BEFORE L8 (i6251 p.10). "If zero or less, enter -0-."
    f.line10 = (i.regular_tax_l16 + i.schedule_2_line1z - i.schedule_3_line1).max(z);

    // L6's routing: "If zero or less, enter -0- here and on lines 7, 9, and 11, and go to line 10."
    if line6 <= z {
        return f;
    }

    let preferential = (i.net_capital_gain + i.qualified_dividends).max(z);
    if preferential > z {
        // L7's middle bullet — "complete Part III on the back and enter the amount from line 40 here".
        f.line12 = line6;
        f.line13 = preferential;
        f.line14 = z; // Schedule D line 19; no §1250 property in btctax
                      // L15's FIRST branch is the applicable one: "If you did not complete a Schedule D Tax Worksheet
                      // for the regular tax or the AMT, enter the amount from line 13." btctax figures preferential
                      // income through the QDCGT Worksheet and never completes a Schedule D Tax Worksheet, so the
                      // "Otherwise, add lines 13 and 14, and enter the smaller of that result or [SDTW line 10]"
                      // branch — with its cap we could not evaluate — is unreachable here. (Numerically identical
                      // today because `line14 ≡ 0`; taking the right branch is a transcription-fidelity fix, and it
                      // removes a cap that a Tier-2 author adding Schedule D would otherwise inherit silently.)
        f.line15 = f.line13;
        f.line16 = f.line12.min(f.line15);
        f.line17 = f.line12 - f.line16;
        f.line18 = flat_26_28(f.line17);
        f.line19 = bp.max_zero;
        f.line20 = i.qdcgt_line5_regular;
        f.line21 = (f.line19 - f.line20).max(z);
        f.line22 = f.line12.min(f.line13);
        f.line23 = f.line21.min(f.line22);
        f.line24 = f.line22 - f.line23;
        f.line25 = bp.max_fifteen;
        f.line26 = f.line21;
        f.line27 = i.qdcgt_line5_regular;
        f.line28 = f.line26 + f.line27;
        f.line29 = (f.line25 - f.line28).max(z);
        f.line30 = f.line24.min(f.line29);
        f.line31 = Decimal::new(15, 2) * f.line30;
        f.line32 = f.line23 + f.line30;
        // "If lines 32 and 12 are the same, skip lines 33 through 37 and go to line 38."
        if f.line32 != f.line12 {
            f.line33 = f.line22 - f.line32; // ★ from line 22
            f.line34 = Decimal::new(20, 2) * f.line33;
            // "If line 14 is zero or blank, skip lines 35 through 37 and go to line 38."
            if f.line14 != z {
                f.line35 = f.line17 + f.line32 + f.line33;
                f.line36 = f.line12 - f.line35;
                f.line37 = Decimal::new(25, 2) * f.line36;
            }
        }
        f.line38 = f.line18 + f.line31 + f.line34 + f.line37;
        f.line39 = flat_26_28(f.line12);
        f.line40 = f.line38.min(f.line39);
        f.line7 = f.line40;
    } else {
        // L7's "All others" bullet.
        f.line7 = flat_26_28(line6);
    }

    // i6251 p.10: line 8 is blank when line 10 ≥ line 7; otherwise the §904(j) elector's AMTFTC
    // equals Schedule 3 line 1.
    f.line8 = if f.line10 >= f.line7 {
        z
    } else {
        i.schedule_3_line1
    };
    f.line9 = f.line7 - f.line8;
    f.line11 = (f.line9 - f.line10).max(z);
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::LtcgBreakpoints;
    use rust_decimal_macros::dec;
    use std::collections::BTreeSet;
    use std::str::FromStr;

    /// The T1 fixture: `fixtures/form6251_vectors.json`, emitted by the line-by-line transcription
    /// in `design/amt-form6251/f6251_reference.py` and committed BEFORE this module existed. V1–V6
    /// were derived independently in plan review r1, before any code — so a match here is a real
    /// cross-check, not a tautology.
    // ★ IN-CRATE, deliberately. An `include_str!` reaching outside the crate root is invisible to
    //   `cargo publish`: the verification build compiles lib+bins only, so a `#[cfg(test)]` include
    //   never resolves and the publish SUCCEEDS while shipping a tarball whose tests cannot compile.
    //   `cargo package --list` carried zero files under `design/`. Caught pre-release; the vector KATs
    //   are the whole subject of this release, so they would have gone dark for anyone building from
    //   the published crate.
    const VECTORS: &str = include_str!("fixtures/form6251_vectors.json");

    fn params() -> AmtParams {
        AmtParams {
            exemption_single_hoh: dec!(85700),
            exemption_mfj_qss: dec!(133300),
            exemption_mfs: dec!(66650),
            phaseout_start_single_hoh_mfs: dec!(609350),
            phaseout_start_mfj_qss: dec!(1218700),
            breakpoint_28pct: dec!(232600),
            breakpoint_28pct_mfs: dec!(116300),
            mfs_kicker_start: dec!(875950),
            mfs_kicker_max: dec!(66650),
            phaseout_rate: dec!(0.25),
            rate_26: dec!(0.26),
            rate_28: dec!(0.28),
            rate_28_subtrahend: dec!(4652),
            rate_28_subtrahend_mfs: dec!(2326),
        }
    }
    /// §1(h) breakpoints, per status — these ARE Form 6251 lines 19 and 25, which print all four
    /// sets ("$94,050 if married filing jointly …, $47,025 if single or married filing separately,
    /// or $63,000 if head of household").
    ///
    /// ★ EXHAUSTIVE, deliberately. This was `Mfs => …, _ => …`, i.e. every non-MFS status silently
    ///   got the MFJ bands. That was invisible while every vector was MFJ or MFS, and it broke the
    ///   moment the E2 population added Single and HoH: V11 came out with MFJ's $133,300 exemption
    ///   and no phase-out. A wildcard arm in a per-status table is the same defect class as a
    ///   compressed form line — it makes an omission compile.
    fn bps(status: FilingStatus) -> LtcgBreakpoints {
        match status {
            FilingStatus::Single => LtcgBreakpoints {
                max_zero: dec!(47025),
                max_fifteen: dec!(518900),
            },
            FilingStatus::Mfj | FilingStatus::Qss => LtcgBreakpoints {
                max_zero: dec!(94050),
                max_fifteen: dec!(583750),
            },
            FilingStatus::Mfs => LtcgBreakpoints {
                max_zero: dec!(47025),
                max_fifteen: dec!(291850),
            },
            FilingStatus::HoH => LtcgBreakpoints {
                max_zero: dec!(63000),
                max_fifteen: dec!(551350),
            },
        }
    }
    fn d(v: &serde_json::Value) -> Usd {
        Usd::from_str(v.as_str().expect("fixture value is a decimal string")).expect("parses")
    }

    /// Build a vector's [`Form6251Inputs`] and run [`compute_6251`] on it — the ONE place the fixture is
    /// turned into a computation, so a test that wants to exercise real behaviour cannot accidentally
    /// settle for reading the fixture back instead.
    fn compute_vector(v: &serde_json::Value, p: &AmtParams) -> Form6251 {
        let inp = &v["inputs"];
        let der = &v["derived"];
        // ★ PANICS on an unknown token rather than falling back. The `_ => Mfj` this replaced
        //   turned every Single and HoH vector into an MFJ one — the fixture said `single`, the
        //   computation said MFJ, and the KAT still passed because the expected figures were MFJ
        //   figures too. A fixture that silently answers for the filer is the answered-ness defect
        //   in test clothing.
        let status = match inp["filing_status"].as_str().unwrap() {
            "single" => FilingStatus::Single,
            "mfj" => FilingStatus::Mfj,
            "mfs" => FilingStatus::Mfs,
            "hoh" => FilingStatus::HoH,
            "qss" => FilingStatus::Qss,
            other => panic!("fixture filing_status {other:?} is not a FilingStatus"),
        };
        let ti = d(&der["taxable_income_1040_L15"]);
        let pref = d(&inp["net_ltcg"]);
        // QDCGT worksheet line 5 = taxable income − preferential (capped at TI), as figured for
        // the regular tax. Lines 20 and 27 read this.
        let qdcgt_l5 = (ti - pref.min(ti)).max(Usd::ZERO);
        compute_6251(
            Form6251Inputs {
                status,
                taxable_income_l15: ti,
                agi_l11: d(&der["agi_1040_L11"]),
                deduction_l12: d(&der["deduction_1040_L14"]),
                deduction_l14: d(&der["deduction_1040_L14"]),
                schedule_a_line7: Usd::ZERO,
                itemized: der["itemized"].as_bool().unwrap(),
                state_refund_sch1_l1: d(&inp["state_refund"]),
                net_capital_gain: pref,
                qualified_dividends: Usd::ZERO,
                qdcgt_line5_regular: qdcgt_l5,
                regular_tax_l16: d(&der["regular_tax_1040_L16"]),
                schedule_2_line1z: Usd::ZERO,
                schedule_3_line1: d(&inp["sch3_line1_ftc"]),
            },
            p,
            &bps(status),
        )
    }

    /// ★ THE conformance KAT: every fixture vector, **every line the fixture carries**, exact cents.
    ///
    /// Mutations this kills (each verified by making the edit and watching this red):
    /// - line 33 `f.line22 - f.line32` → `f.line12 - f.line32` (the transcription bug) ⇒ V3/V4/V5/V6
    /// - drop the line-2b subtraction ⇒ V7, and now V29, where it moves the AMT itself
    /// - drop the MFS kicker ⇒ V8, V23–V25
    /// - line 16/22's `.min(...)` caps removed ⇒ V2b
    /// - any per-status constant (exemption, phase-out start, 26/28% breakpoint, lines 19/25)
    ///   collapsed onto one status ⇒ the E2 population, which spans all four
    ///
    /// ★ The line list is CLOSED at both ends. Enumerating the lines catches a field that stops
    ///   being computed; the `checked == fixture keys` assertion at the bottom catches the reverse —
    ///   a line present in the fixture that this test forgot to look at. Without it, adding a line
    ///   to the fixture and not to the list is a silent hole, which is how a 41-line form ends up
    ///   with 20 lines under test.
    #[test]
    fn every_vector_reproduces_the_form_line_by_line() {
        let json: serde_json::Value = serde_json::from_str(VECTORS).expect("fixture parses");
        let vectors = json["vectors"].as_array().expect("vectors array");
        assert!(
            vectors.len() >= 30,
            "fixture shrank: {} vectors",
            vectors.len()
        );
        let p = params();
        for v in vectors {
            let id = v["id"].as_str().unwrap();
            let want = &v["form6251"];
            let got = compute_vector(v, &p);
            // ★ INDEXED, not `get()`: a fixture key that is renamed or dropped must PANIC, not be
            // silently skipped — a skipped key is a vector that quietly stops checking a line.
            // Enumerated as DATA, not asserted in a closure: the same list then serves
            // as the `checked` set below, so the two can never drift apart.
            let lines: [(&str, Usd); 41] = [
                // Part I
                ("line1", got.line1),
                ("line2a", got.line2a),
                ("line2b", got.line2b),
                ("line3", got.line3),
                ("line4", got.line4),
                // Part II
                ("line5", got.line5),
                ("line6", got.line6),
                ("line7", got.line7),
                ("line8", got.line8),
                ("line9", got.line9),
                ("line10", got.line10),
                ("line11", got.line11),
                // Part III
                ("line12", got.line12),
                ("line13", got.line13),
                ("line14", got.line14),
                ("line15", got.line15),
                ("line16", got.line16),
                ("line17", got.line17),
                ("line18", got.line18),
                ("line19", got.line19),
                ("line20", got.line20),
                ("line21", got.line21),
                ("line22", got.line22),
                ("line23", got.line23),
                ("line24", got.line24),
                ("line25", got.line25),
                ("line26", got.line26),
                ("line27", got.line27),
                ("line28", got.line28),
                ("line29", got.line29),
                ("line30", got.line30),
                ("line31", got.line31),
                ("line32", got.line32),
                ("line33", got.line33),
                ("line34", got.line34),
                ("line35", got.line35),
                ("line36", got.line36),
                ("line37", got.line37),
                ("line38", got.line38),
                ("line39", got.line39),
                ("line40", got.line40),
            ];
            for (n, actual) in lines {
                assert_eq!(actual, d(&want[n]), "{id}: Form 6251 {n}");
            }
            let checked: BTreeSet<&str> = lines.iter().map(|&(n, _)| n).collect();

            let carried: BTreeSet<&str> = want
                .as_object()
                .expect("form6251 is an object")
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                carried, checked,
                "{id}: the fixture and this test disagree about which lines exist"
            );
            assert_eq!(
                got.must_attach(),
                v["attach_required_who_must_file_cond1"].as_bool().unwrap(),
                "{id}: Who Must File condition 1 (line 7 > line 10)"
            );
        }
    }

    /// i6251 p.9's OWN worked example: "if the amount on line 4 is $895,950, enter $900,950
    /// instead—the additional $5,000 is 25% of $20,000 ($895,950 minus $875,950)."
    #[test]
    fn mfs_kicker_reproduces_the_irs_worked_example() {
        let p = params();
        // Pre-kicker line 4 = line1 + line2a = 881,350 + 14,600 = 895,950.
        let f = compute_6251(
            Form6251Inputs {
                status: FilingStatus::Mfs,
                taxable_income_l15: dec!(881350),
                agi_l11: dec!(895950),
                deduction_l12: dec!(14600),
                deduction_l14: dec!(14600),
                schedule_a_line7: Usd::ZERO,
                itemized: false,
                state_refund_sch1_l1: Usd::ZERO,
                net_capital_gain: dec!(395950),
                qualified_dividends: Usd::ZERO,
                qdcgt_line5_regular: dec!(485400),
                regular_tax_l16: dec!(1),
                schedule_2_line1z: Usd::ZERO,
                schedule_3_line1: Usd::ZERO,
            },
            &p,
            &bps(FilingStatus::Mfs),
        );
        assert_eq!(f.line4, dec!(900950), "the IRS's own example");
        // The two boundaries the example brackets.
        let at = |l4_pre: Usd| {
            let f = compute_6251(
                Form6251Inputs {
                    status: FilingStatus::Mfs,
                    taxable_income_l15: l4_pre - dec!(14600),
                    agi_l11: l4_pre,
                    deduction_l12: dec!(14600),
                    deduction_l14: dec!(14600),
                    schedule_a_line7: Usd::ZERO,
                    itemized: false,
                    state_refund_sch1_l1: Usd::ZERO,
                    net_capital_gain: Usd::ZERO,
                    qualified_dividends: Usd::ZERO,
                    qdcgt_line5_regular: l4_pre - dec!(14600),
                    regular_tax_l16: dec!(1),
                    schedule_2_line1z: Usd::ZERO,
                    schedule_3_line1: Usd::ZERO,
                },
                &p,
                &bps(FilingStatus::Mfs),
            );
            f.line4 - l4_pre
        };
        assert_eq!(
            at(dec!(875950)),
            Usd::ZERO,
            "at the start, nothing is added"
        );
        assert_eq!(at(dec!(1142550)), dec!(66650), "at the cap, a flat $66,650");
        assert_eq!(
            at(dec!(2000000)),
            dec!(66650),
            "above the cap, still $66,650"
        );
    }

    /// ★ The attach test is Who Must File condition 1 — **line 7 > line 10** — not `amt() > 0`.
    /// V9 is the whole reason that distinction is load-bearing.
    #[test]
    fn v9_must_attach_while_the_amt_is_zero() {
        let json: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
        let v9 = json["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == "V9")
            .expect("V9 present");

        // ★ COMPUTED, not read back. This test previously loaded `v9["form6251"]` and asserted the
        //   FIXTURE was self-consistent — it never called `compute_6251` or `must_attach()`, so
        //   breaking either left it green. It is the one test named for the property that
        //   `attach iff amt > 0` is WRONG, and it was not holding it.
        let got = compute_vector(v9, &params());

        assert!(
            got.must_attach(),
            "V9: line 7 ({}) must exceed line 10 ({}) — attachment required",
            got.line7,
            got.line10
        );
        assert_eq!(got.amt(), Usd::ZERO, "V9: yet the AMT itself is zero");
        assert!(
            got.line7 > got.line10 && got.amt() == Usd::ZERO,
            "V9 exists precisely to prove `attach iff amt > 0` is WRONG"
        );
        // The fixture must agree with the computation, or the vector has drifted from its own label.
        assert!(
            v9["attach_required_who_must_file_cond1"].as_bool().unwrap(),
            "V9's fixture label disagrees with the computed form"
        );
    }

    /// ★ Line 40's `min` is a **provable no-op** for btctax's input class — and this test is what
    /// makes that a checked fact rather than an assumption.
    ///
    /// Mutation M6 (drop the `.min(f.line39)`) survived every vector, which is the correct outcome:
    /// line 38 ≤ line 39 always here. `flat_26_28` has slope 0.26/0.28, both strictly above the
    /// §1(h) ceiling of 0.20, and the $4,652 subtrahend cancels whenever both terms clear the
    /// breakpoint. So taxing the gain preferentially can never cost more than taxing everything at
    /// 26/28%.
    ///
    /// **It stays in the code because it is on the form**, and this test guards the proof: if the
    /// input class ever widens to a preferential rate ≥ 26% — §1(h)(4) **collectibles at 28%**, or
    /// unrecaptured §1250 at 25% combined with the subtrahend edge — the no-op claim dies and this
    /// reds, forcing the `min` to be re-examined rather than silently trusted.
    #[test]
    fn line40_min_is_a_proved_no_op_for_this_input_class() {
        let p = params();
        // ★ BOTH §55(b)(1) schedules. This swept only the general one — never the MFS schedule the
        //   v0.14.0 branch added, whose breakpoint ($116,300) and subtrahend ($2,326) are both HALVED.
        //   The proof has to hold on the schedule a filer is actually put on, not just the common one.
        for status in [FilingStatus::Mfj, FilingStatus::Mfs] {
            line40_min_sweep_for(&p, status);
        }
    }

    /// One §55(b)(1) schedule's worth of the line-40 sweep. Split out so both the general and the MFS
    /// schedule are exercised with the SAME proof rather than the general one twice.
    fn line40_min_sweep_for(p: &AmtParams, status: FilingStatus) {
        let bp28 = p.breakpoint_28pct(status);
        let subtrahend = if status == FilingStatus::Mfs {
            p.rate_28_subtrahend_mfs
        } else {
            p.rate_28_subtrahend
        };
        let flat = |x: Usd| {
            if x <= bp28 {
                p.rate_26 * x
            } else {
                p.rate_28 * x - subtrahend
            }
        };
        let mut checked = 0u32;
        let mut step = 0i64;
        while step < 1_200_000 {
            let l17 = Usd::from(step);
            let mut g = 0i64;
            while g < 1_200_000 {
                let gain = Usd::from(g);
                // The worst case for the claim is the HIGHEST preferential rate btctax can produce.
                let line38 = flat(l17) + Decimal::new(20, 2) * gain;
                let line39 = flat(l17 + gain);
                assert!(
                    line38 <= line39,
                    "line 40's min WOULD bind for {status:?} at line17={l17}, gain={gain}: 38={line38} > 39={line39}. The input class has widened — re-examine the min, do not delete it."
                );
                checked += 1;
                g += 9_973; // a prime-ish stride, to land off the round breakpoints too
            }
            step += 9_973;
        }
        assert!(
            checked > 10_000,
            "the sweep must actually cover ground: {checked}"
        );
    }

    /// ★ Line 2a cites 1040 line **12**; line 1's fallback cites line **14**. They differ by exactly
    /// the §199A QBI deduction, so only a filer with QBI > 0 tells them apart — and no fixture vector
    /// has QBI, which is why this needs its own KAT. Reading line 14 into line 2a inflates AMTI by the
    /// QBI deduction and over-refuses.
    #[test]
    fn line2a_reads_1040_line12_not_line14_which_qbi_distinguishes() {
        let mk = |l12: Usd, l14: Usd| {
            compute_6251(
                Form6251Inputs {
                    status: FilingStatus::Mfj,
                    taxable_income_l15: dec!(400000),
                    agi_l11: dec!(450000),
                    deduction_l12: l12,
                    deduction_l14: l14,
                    schedule_a_line7: Usd::ZERO,
                    itemized: false,
                    state_refund_sch1_l1: Usd::ZERO,
                    net_capital_gain: Usd::ZERO,
                    qualified_dividends: Usd::ZERO,
                    qdcgt_line5_regular: dec!(400000),
                    regular_tax_l16: dec!(80000),
                    schedule_2_line1z: Usd::ZERO,
                    schedule_3_line1: Usd::ZERO,
                },
                &params(),
                &bps(FilingStatus::Mfj),
            )
        };
        // L12 = 29,200 standard deduction; L14 = 50,000 because a $20,800 QBI deduction sits on L13.
        let f = mk(dec!(29200), dec!(50000));
        assert_eq!(f.line2a, dec!(29200), "line 2a is 1040 L12");
        assert_eq!(f.line4, dec!(429200), "AMTI = 400,000 + 29,200");
        // Reading L14 instead would add the QBI deduction back into AMTI.
        let wrong = mk(dec!(50000), dec!(50000));
        assert_eq!(wrong.line4, dec!(450000));
        assert_ne!(
            f.line4, wrong.line4,
            "the two 1040 lines must not be interchangeable"
        );
    }

    /// TY2024 ordinary schedules (Rev. Proc. 2023-34 §2.01), inline so this needs no adapter crate —
    /// the same construction `method.rs`'s own tests use.
    fn schedule(status: FilingStatus) -> crate::tax::tables::OrdinarySchedule {
        use crate::tax::tables::{OrdinaryBracket, OrdinarySchedule};
        let edges: &[(Decimal, Decimal)] = match status {
            FilingStatus::Single => &[
                (dec!(0), dec!(0.10)),
                (dec!(11600), dec!(0.12)),
                (dec!(47150), dec!(0.22)),
                (dec!(100525), dec!(0.24)),
                (dec!(191950), dec!(0.32)),
                (dec!(243725), dec!(0.35)),
                (dec!(609350), dec!(0.37)),
            ],
            FilingStatus::Mfj | FilingStatus::Qss => &[
                (dec!(0), dec!(0.10)),
                (dec!(23200), dec!(0.12)),
                (dec!(94300), dec!(0.22)),
                (dec!(201050), dec!(0.24)),
                (dec!(383900), dec!(0.32)),
                (dec!(487450), dec!(0.35)),
                (dec!(731200), dec!(0.37)),
            ],
            FilingStatus::Mfs => &[
                (dec!(0), dec!(0.10)),
                (dec!(11600), dec!(0.12)),
                (dec!(47150), dec!(0.22)),
                (dec!(100525), dec!(0.24)),
                (dec!(191950), dec!(0.32)),
                (dec!(243725), dec!(0.35)),
                (dec!(365600), dec!(0.37)),
            ],
            FilingStatus::HoH => &[
                (dec!(0), dec!(0.10)),
                (dec!(16550), dec!(0.12)),
                (dec!(63100), dec!(0.22)),
                (dec!(100500), dec!(0.24)),
                (dec!(191950), dec!(0.32)),
                (dec!(243700), dec!(0.35)),
                (dec!(609350), dec!(0.37)),
            ],
        };
        OrdinarySchedule {
            brackets: edges
                .iter()
                .map(|&(lower, rate)| OrdinaryBracket { lower, rate })
                .collect(),
        }
    }

    fn std_deduction(status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj | FilingStatus::Qss => dec!(29200),
            FilingStatus::HoH => dec!(21900),
            FilingStatus::Single | FilingStatus::Mfs => dec!(14600),
        }
    }

    /// ★ THREE PART III ROUTINGS ARE UNREACHABLE WITH AMT OWED, and they are all one fact.
    ///
    /// E2 (`FOLLOWUPS §G-6b`) asked for an AMT-owing vector per live Part III routing. Three cells
    /// came back empty, and this test is why the fixture has no vector for them and why that is a
    /// result rather than a gap:
    ///
    /// - **the line-32 skip** ("If lines 32 and 12 are the same, skip lines 33 through 37")
    /// - **line 39 on the 26% side** — line 12 at or under the §55(b)(1) breakpoint
    /// - **no exemption phase-out** — line 5 still at the full §55(d)(1) amount
    ///
    /// The root fact is the third: **in btctax's input class, an attachable Form 6251 exists only
    /// once the §55(d)(3) phase-out has begun.** The exemption is simply worth more than the flat
    /// 26/28% rate's excess over the graduated schedule, everywhere the exemption survives intact.
    /// Measured over the intact region at its widest — largest §63(f) deduction, largest §904(j)
    /// FTC — `line 7 − line 10` is **never positive**, and at most −$1,098.50 wherever line 7 > 0:
    /// Single −$6,377, MFJ −$2,193, MFS −$1,098.50, HoH −$2,785. (Its unrestricted maximum is
    /// exactly $0, at the degenerate corner where line 7 = line 10 = 0; `must_attach` is a strict
    /// `>`, so $0 does not attach.)
    ///
    /// Given that, once the phase-out has begun line 12 is at least the phase-out start minus the
    /// full exemption — $523,650 for Single/HoH, $542,700 MFS, $1,085,400 MFJ.
    ///
    /// - **(b) follows outright.** All four floors clear the §55(b)(1) breakpoint ($232,600; MFS
    ///   $116,300) with room to spare, so line 39 can never take the 26% side.
    /// - **(a) follows for three of the four.** Lines 23/30 are each bounded by their own band, and
    ///   line 29 is CLAMPED at zero (`line29 = max(0, line25 − line28)`), so:
    ///   `line32 = line23 + line30 ≤ line21 + max(0, line25 − line21 − line27)
    ///    = max(line21, line25 − line27) ≤ line25`, the last step because `line21 ≤ line19 < line25`
    ///   on every status and `line27 ≥ 0`. So `line12 > line25` forecloses the skip — true for
    ///   Single ($523,650 > $518,900, though by only $4,750), MFS and MFJ.
    ///   ★ **NOT true for HoH**, whose line-25 top is $551,350 — *above* the $523,650 floor. In that
    ///   $27,700 window the inequality says nothing, and the skip additionally needs
    ///   `line27 ≤ line25 − line12`, i.e. an ordinary bottom under $27,700. **HoH's (a) is carried
    ///   by the sweep below, not by this argument.**
    ///
    /// ★ THAT CHAIN IS WRITTEN OUT IN FULL BECAUSE IT WAS WRONG TWICE. The original claimed line 12
    /// always exceeds the 15%-band top — false for HoH. The r2 fold replaced it with
    /// `… ≤ line25 − line27 ≤ line25`, which drops line 29's clamp and is falsified by seven of this
    /// module's own fixture vectors (V1: line 27 = $915,000 > line 25 = $583,750, so line 32 = 0
    /// while the claimed bound is −$331,250). Same conclusion, wrong reason, twice — which is exactly
    /// the failure mode `CLAUDE.md` names as this project's most expensive recurring defect. The
    /// `max(0, …)` is the whole content of the fix.
    ///
    /// ★ WHY A SWEEP AND NOT A COMMENT. If btctax's input surface widens — an ISO exercise at line
    /// 2i, a §1202 exclusion at 2h, §1250 gain reaching line 14 — a preference item can push AMTI
    /// up without moving the regular tax, and the form becomes attachable with the exemption intact.
    /// At that moment three routings go live with **no vector and no oracle behind them**, and the
    /// only thing that would notice is this test. It is the same tripwire, for the same reason, as
    /// `line40_min_is_a_proved_no_op_for_this_input_class`.
    ///
    /// ★ ITS SENSITIVITY IS MEASURED, NOT ASSUMED: **~$4,400**. Injecting a flat preference into
    /// line 4 and bisecting, $4,200 leaves this green and $4,400 reds it (on MFS, standard deduction
    /// at the §63(f) maximum, FTC at the §904(j) ceiling). That floor is arithmetic, not slack — a
    /// ΔAMTI moves the tentative minimum tax by only 26–28% of itself, so closing MFS's $1,098.50
    /// margin needs ~$4,000. A smaller preference than that would go unnoticed here, which is why
    /// E6 (lines 2c–2t as real fields) is a *design* obligation and not something this test replaces.
    /// Before r2 the floor was far worse: the sweep pinned the BASE standard deduction and a zero
    /// FTC, and stayed green at $5,000.
    ///
    /// ★ WHAT THIS IS NOT. A stride sweep is a tripwire, not a proof. Two finer scans back it: a
    /// ~450M-point scan over (wages, gain, gift, refund, FTC) at $500/$1,000 resolution run while
    /// building the E2 population, and review r2's exhaustive $100-grid scan of the entire
    /// intact-exemption region at the maximum deduction and FTC. Both found the same three cells
    /// empty on all four statuses. This test carries the claim forward — and does it against the
    /// PRODUCTION regular tax (`qdcgt_line16`, Tax-Table quantization and all), which the first
    /// scan approximated with the exact schedule.
    #[test]
    fn amt_is_owed_only_once_the_exemption_phaseout_has_begun() {
        let p = params();
        for status in [
            FilingStatus::Single,
            FilingStatus::Mfj,
            FilingStatus::Mfs,
            FilingStatus::HoH,
        ] {
            // ★ BOTH the base standard deduction and the §63(f) MAXIMUM. Line 2a adds the whole
            //   standard deduction back, so a bigger one lifts AMTI at a FIXED regular tax — the one
            //   direction that can break the claim, and the only axis of the real input surface that
            //   does. Pinning the base amount left the tripwire blind to a widening of up to
            //   ~$6,200: with a $5,000 preference injected, the base-only sweep stayed green while an
            //   MFS filer at the §63(f) maximum owed AMT with the exemption intact at 7,895 points.
            for deduction in [
                std_deduction(status),
                std_deduction(status) + max_aged_blind_addition(status),
            ] {
                for itemized in [true, false] {
                    // ★ FTC at the §904(j) de-minimis ceiling as well as zero — see the guard below.
                    for ftc in [Usd::ZERO, ftc_ceiling(status)] {
                        phaseout_precondition_sweep_for(&p, status, itemized, deduction, ftc);
                    }
                }
            }
        }
    }

    /// The largest §63(f) aged/blind addition btctax can grant: $1,550 per box married / $1,950
    /// unmarried (Rev. Proc. 2023-34 §3.15(3)), times the boxes `AgedBlindBoxes::count()` can reach.
    /// Spouse boxes exist only on a joint return, and `standard_deduction` conservatively declines to
    /// count the rare no-income-spouse box on MFS — so MFS gets the taxpayer's two, not four.
    fn max_aged_blind_addition(status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj | FilingStatus::Qss => dec!(1550) * dec!(4),
            FilingStatus::Mfs => dec!(1550) * dec!(2),
            FilingStatus::Single | FilingStatus::HoH => dec!(1950) * dec!(2),
        }
    }

    /// The §904(j) no-Form-1116 ceiling — the largest Schedule 3 line 1 that can reach Form 6251
    /// without refusing upstream as `ForeignTaxOverCeiling`.
    fn ftc_ceiling(status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj | FilingStatus::Qss => dec!(600),
            _ => dec!(300),
        }
    }

    fn phaseout_precondition_sweep_for(
        p: &AmtParams,
        status: FilingStatus,
        itemized: bool,
        std_deduction_amount: Usd,
        ftc: Usd,
    ) {
        let sched = schedule(status);
        let bp = bps(status);
        let full_exemption = p.exemption(status);
        let bp28 = p.breakpoint_28pct(status);
        // Itemizers here deduct a cash gift only, so Schedule A line 7 (taxes) is zero and line 2a
        // adds nothing back; non-itemizers add the standard deduction back at line 2a.
        let deduction = if itemized {
            std_deduction_amount + dec!(10000) // any itemized total; only `itemized` routes line 2a
        } else {
            std_deduction_amount
        };
        let mut checked = 0u32;
        let mut bottom = 0i64;
        while bottom < 1_300_000 {
            let ordinary = Usd::from(bottom);
            let mut g = 0i64;
            while g < 4_000_000 {
                let gain = Usd::from(g);
                let ti = ordinary + gain;
                let f = compute_6251(
                    Form6251Inputs {
                        status,
                        taxable_income_l15: ti,
                        agi_l11: ti + deduction,
                        deduction_l12: deduction,
                        deduction_l14: deduction,
                        schedule_a_line7: Usd::ZERO,
                        itemized,
                        state_refund_sch1_l1: Usd::ZERO,
                        net_capital_gain: gain,
                        qualified_dividends: Usd::ZERO,
                        qdcgt_line5_regular: ordinary,
                        // ★ the PRODUCTION regular tax for this very household. Feeding an
                        //   arbitrary line 16 would make the whole sweep vacuous: AMT is a
                        //   COMPARISON, and half of it would be fiction.
                        regular_tax_l16: crate::tax::method::qdcgt_line16(
                            &sched,
                            &bp,
                            ti,
                            Usd::ZERO,
                            gain,
                        ),
                        schedule_2_line1z: Usd::ZERO,
                        schedule_3_line1: ftc,
                    },
                    p,
                    &bps(status),
                );
                // ★ GUARDED ON `must_attach()`, NOT `amt() > 0` — Who Must File condition 1, which is
                //   what Tier 2 will actually attach on. The two coincide only when line 8 is zero
                //   (V9 is the standing counterexample: attach required, AMT $0, FTC $600), so a
                //   sweep that pinned the FTC to zero was guarding a strictly narrower population
                //   than the one that gets filed. `must_attach()` is the superset, so every
                //   assertion below is now stronger than it was.
                let ctx = || {
                    format!("{status:?} itemized={itemized} deduction={deduction} ftc={ftc} (ordinary {ordinary}, gain {gain})")
                };
                // ★ The bound (a)'s argument rests on, ASSERTED rather than argued — it was written
                //   wrongly twice, both times surviving a review of the prose. Holds on EVERY point,
                //   attachable or not: line 23 ≤ line 21 and line 30 ≤ line 29 = max(0, line25 −
                //   line28), so line 32 ≤ max(line21, line25 − line27) ≤ line 25. The `max(0, …)` is
                //   the term the second wrong version dropped; without it the bound goes negative
                //   whenever the regular-side ordinary bottom clears the 15%-band top.
                assert!(
                    f.line32 <= f.line21.max(f.line25 - f.line27),
                    "{}: line 32 ({}) exceeds max(line21 {}, line25 − line27 {}) — the Part III \
                     band bound is broken",
                    ctx(),
                    f.line32,
                    f.line21,
                    f.line25 - f.line27
                );
                assert!(
                    f.line32 <= f.line25,
                    "{}: line 32 ({}) exceeds line 25 ({})",
                    ctx(),
                    f.line32,
                    f.line25
                );
                if f.must_attach() {
                    assert!(
                        f.line5 < full_exemption,
                        "{}: Form 6251 must be ATTACHED (line 7 {} > line 10 {}) with the exemption \
                         INTACT at {}. The input class has widened — three Part III routings just \
                         went live with no vector behind them.",
                        ctx(),
                        f.line7,
                        f.line10,
                        f.line5
                    );
                    assert!(
                        f.line32 != f.line12,
                        "{}: attachable while line 32 equals line 12, i.e. the line-32 skip fired. \
                         The fixture has no vector for that routing.",
                        ctx()
                    );
                    assert!(
                        f.line12 > bp28,
                        "{}: attachable with line 12 ({}) at or under the §55(b)(1) breakpoint \
                         {bp28}, i.e. line 39 on the 26% side. The fixture has no vector for that \
                         routing.",
                        ctx(),
                        f.line12
                    );
                    checked += 1;
                }
                g += 19_997; // prime-ish strides, to land off the round breakpoints
            }
            bottom += 9_973;
        }
        // The sweep has to actually FIND AMT, or it proves nothing about AMT-owing returns.
        assert!(
            checked > 500,
            "{status:?} itemized={itemized}: only {checked} AMT-owing points swept"
        );
    }

    /// Line 6's routing: "If zero or less, enter -0- here and on lines 7, 9, and 11, and go to line 10."
    #[test]
    fn line6_zero_or_less_zeroes_lines_7_9_and_11_but_still_fills_line_10() {
        let f = compute_6251(
            Form6251Inputs {
                status: FilingStatus::Mfj,
                taxable_income_l15: dec!(50000),
                agi_l11: dec!(79200),
                deduction_l12: dec!(29200),
                deduction_l14: dec!(29200),
                schedule_a_line7: Usd::ZERO,
                itemized: false,
                state_refund_sch1_l1: Usd::ZERO,
                net_capital_gain: Usd::ZERO,
                qualified_dividends: Usd::ZERO,
                qdcgt_line5_regular: dec!(50000),
                regular_tax_l16: dec!(5536),
                schedule_2_line1z: Usd::ZERO,
                schedule_3_line1: Usd::ZERO,
            },
            &params(),
            &bps(FilingStatus::Mfj),
        );
        assert_eq!(f.line4, dec!(79200));
        assert_eq!(f.line6, Usd::ZERO, "AMTI below the exemption");
        assert_eq!(
            (f.line7, f.line9, f.line11),
            (Usd::ZERO, Usd::ZERO, Usd::ZERO)
        );
        assert_eq!(
            f.line10,
            dec!(5536),
            "line 10 is still filled — the form says 'go to line 10'"
        );
        assert!(!f.must_attach());
    }
}
