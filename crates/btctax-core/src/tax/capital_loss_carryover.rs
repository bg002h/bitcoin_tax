//! **The §1212(b)(2)(B) Capital Loss Carryover Worksheet — Lines 6 and 14**, transcribed.
//!
//! ★★★ **WHY THIS EXISTS: the same worksheet was refused honestly on one side and silently
//! misprinted on the other.** `return_1040::screen_absolute` refuses a return whose taxable income is
//! ≤ 0 *with a capital-loss carryforward **in***, naming this worksheet as unmodeled. The
//! carryforward-**out** of the very same year was computed by a flat `absorbed = min(loss, $3,000)`
//! with no taxable-income term anywhere in it — so a loss year that wiped the filer out printed a
//! surviving loss up to the whole §1211(b) allowance too small, silently, forever. Measured on the
//! plan's L4 household (Single, no wages, one $20,000 long-term loss): **$17,000 printed where the
//! worksheet gives $20,000.**
//!
//! **THE AUTHORITY IS THE 2025 INSTRUCTIONS, NOT THE 2024 ONES**, and that is not a typo. The
//! worksheet printed in the instructions for year *Y* figures the carryover **from Y−1 into Y**. A
//! TY2024 return's carryover-**out** is therefore figured by the worksheet in the **2025** Schedule D
//! instructions — `design/forms/extract/i1040sd--2025.txt:1810-1870` — whose every line reads "your
//! **2024** Schedule D" and whose results are labelled "carryover for **2025**". Transcribing the
//! TY2024 sheet instead would have produced a structurally identical worksheet whose doc comments all
//! named the wrong year, which is exactly the sort of thing a citation checker exists to catch; the
//! quotes in [`LINES`] are checked verbatim against that extract by `xtask`.
//!
//! **Transcribed, not derived** (`CLAUDE.md`): one field per numbered line, named for the line,
//! carrying the official instruction text verbatim as its doc comment. No closed form. The flat rule
//! the engine already had *is* algebraically exact whenever 1040 line 15 is ≥ 0 — see
//! [`CapitalLossCarryoverWorksheet::figure`]'s note — but "exact on the branch we happened to test"
//! is precisely the compression this repo's standing rule forbids, and the branch where it breaks is
//! the one the low end of the income axis lives on.
//!
//! **The skips are modelled as skips.** Two interstitial instructions send the filer *past* a block
//! of lines ("enter -0- on line 5 and go to line 9"; "skip lines 9 through 13"), and a skipped line is
//! blank — not zero. Those lines are [`Option<Usd>`] so the difference survives into the type system,
//! per the repo's blank-is-not-zero rule. Nothing here reaches paper (the sheet is "Keep for Your
//! Records"), so no testimony is at stake; the distinction is kept because a reader of this struct
//! must be able to tell "the form told me not to fill this in" from "nobody computed it".

use crate::conventions::Usd;
use crate::tax::types::Carryforward;

/// The worksheet's five inputs, each named for the line of the filed return it is read off.
///
/// Every one of them already exists on the assembled return: [`Self::form_1040_line15_signed`] is the
/// pre-floor `agi − total_deductions`, and the four Schedule D figures are the `net_1222` outputs the
/// printed Schedule D itself is built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapitalLossCarryoverInputs {
    /// **1040 line 15, SIGNED** — the amount that line "would have been … if you could enter a
    /// negative number on that line". The filed line 15 is floored at zero; this one is not, and the
    /// whole defect this module fixes is that the floored figure was the only one that existed.
    pub form_1040_line15_signed: Usd,
    /// **Schedule D line 7** — net short-term capital gain or (loss). Signed: gain > 0, loss < 0.
    pub schedule_d_line7: Usd,
    /// **Schedule D line 15** — net long-term capital gain or (loss). Signed.
    pub schedule_d_line15: Usd,
    /// **Schedule D line 16** — lines 7 and 15 combined. Signed.
    pub schedule_d_line16: Usd,
    /// **Schedule D line 21** — the §1211(b)-limited loss carried to 1040 line 7, as the POSITIVE
    /// magnitude the form's parenthesised box prints. Zero when line 16 is not a loss.
    pub schedule_d_line21_loss: Usd,
}

/// The verbatim instruction text of every numbered line, as DATA.
///
/// ★ It travels as data rather than as an `include_str!` because `design/forms/extract/` is outside
/// this published crate — a reach out of the crate root ships a tarball that builds in the workspace
/// and is broken for everyone else, **with exit 0** (the trap `crate-publishing-state` records). The
/// checking travels to `xtask` instead, which may read the repo tree: it asserts every quote below
/// appears verbatim (modulo whitespace) in [`SOURCE_EXTRACT`], and — the half that catches an
/// *omission* rather than a paraphrase — that the set of line numbers below is exactly the set the
/// extract's own worksheet block enumerates. A hand-written range would have been satisfied by
/// thirteen lines that happened to be numbered 1..=13; the extract is asked instead.
pub const LINES: &[(u8, &str)] = &[
    (
        1,
        "Enter the amount from your 2024 Form 1040, 1040-SR, or 1040-NR, line 15. If the amount \
         would have been a loss if you could enter a negative number on that line, enclose the \
         amount in parentheses",
    ),
    (
        2,
        "Enter the loss from your 2024 Schedule D, line 21, as a positive amount",
    ),
    (3, "Combine lines 1 and 2. If zero or less, enter -0-"),
    (4, "Enter the smaller of line 2 or line 3"),
    (
        5,
        "Enter the loss from your 2024 Schedule D, line 7, as a positive amount",
    ),
    (
        6,
        "Enter any gain from your 2024 Schedule D, line 15. If a loss, enter -0-",
    ),
    (7, "Add lines 4 and 6"),
    (
        8,
        "Short-term capital loss carryover for 2025. Subtract line 7 from line 5. If zero or less, \
         enter -0-. If more than zero, also enter this amount on Schedule D, line 6",
    ),
    (
        9,
        "Enter the loss from your 2024 Schedule D, line 15, as a positive amount",
    ),
    (
        10,
        "Enter any gain from your 2024 Schedule D, line 7. If a loss, enter -0-",
    ),
    (11, "Subtract line 5 from line 4. If zero or less, enter -0-"),
    (12, "Add lines 10 and 11"),
    (
        13,
        "Long-term capital loss carryover for 2025. Subtract line 12 from line 9. If zero or less, \
         enter -0-. If more than zero, also enter this amount on Schedule D, line 14",
    ),
];

/// The applicability sentence — the worksheet's own statement of when it is used AT ALL, and hence of
/// when the answer is "you don't have any carryovers". [`CapitalLossCarryoverWorksheet::figure`]
/// implements exactly this predicate; `xtask` checks the quote.
pub const APPLICABILITY: &str = "Use this worksheet to figure your capital loss carryovers from 2024 \
                                 to 2025 if your 2024 Schedule D, line 21, is a loss and (a) that \
                                 loss is a smaller loss than the loss on your 2024 Schedule D, line \
                                 16; or (b) if the amount on your 2024 Form 1040, 1040-SR or 1040-NR, \
                                 line 15 would be less than zero if you could enter a negative amount \
                                 on that line.";

/// The interstitial instruction that governs the short-term block (lines 6–8).
pub const GOTO_LINE5_OR_LINE9: &str = "If line 7 of your 2024 Schedule D is a loss, go to line 5; \
                                       otherwise, enter -0- on line 5 and go to line 9.";

/// The interstitial instruction that governs the long-term block (lines 9–13).
pub const GOTO_LINE9_OR_SKIP: &str = "If line 15 of your 2024 Schedule D is a loss, go to line 9; \
                                      otherwise, skip lines 9 through 13.";

/// The sentence that CLOSES the applicability test — "**Otherwise, you don't have any carryovers.**"
///
/// [`CapitalLossCarryoverWorksheet::figure`] implements it as `return None`, and it was quoted only in
/// a doc comment until the header-completeness half of the `xtask` checker demanded that every
/// paragraph above worksheet line 1 be accounted for. It carries a real decision, so it is transcribed
/// rather than excused. (The apostrophe is U+2019, as the extract prints it.)
pub const OTHERWISE_NO_CARRYOVERS: &str = "Otherwise, you don’t have any carryovers.";

/// ★★★ The worksheet header's **MFS-after-joint-return sourcing rule** (§1212(b); worksheet header).
///
/// btctax cannot split a joint carryover between spouses: it stores one `Carryforward` per return and
/// knows nothing about which spouse realised which loss. So this sentence is not modelled — it is
/// ASKED, and an affirmative answer refuses
/// ([`RefuseReason::JointReturnCarryoverAttributionUnknown`](crate::tax::return_refuse::RefuseReason)).
///
/// ★ It was invisible to the conformance checker by construction until N1's follow-up: the
/// completeness half read only physical lines beginning `N.`, so unnumbered header prose could not be
/// counted, and both header conditions were dropped while the checker stayed green.
pub const JOINT_RETURN_SOURCING: &str =
    "If you and your spouse once filed a joint return and are filing separate returns for 2025, any \
     capital loss carryover from the joint return can be deducted only on the return of the spouse \
     who actually had the loss.";

/// ★★★ The worksheet header's **§108(b)(2)(G) attribute-reduction condition** (worksheet header).
///
/// Pub. 4681 directs a filer who excluded canceled debt from income to REDUCE tax attributes —
/// capital loss carryovers among them — before carrying them into the next year. btctax models no
/// part of §108(b), so this too is asked rather than assumed
/// ([`RefuseReason::ExcludedCanceledDebtAttributeReduction`](crate::tax::return_refuse::RefuseReason)).
pub const CANCELED_DEBT_EXCLUSION: &str =
    "If you excluded canceled debt from income in 2025, see Pub. 4681.";

/// The extract every quote in this module is checked against, relative to the repo root.
pub const SOURCE_EXTRACT: &str = "design/forms/extract/i1040sd--2025.txt";

/// **Capital Loss Carryover Worksheet — Lines 6 and 14** (2025 Schedule D instructions), one field per
/// numbered line.
///
/// Lines 6–8 and 9–13 are `Option` because the form *skips* them on named branches; see the module
/// docs. Every field is exact `Decimal` — no rounding is applied here, matching the rest of the
/// compute core (the form permits whole-dollar rounding; it does not require it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapitalLossCarryoverWorksheet {
    /// L1 — "Enter the amount from your 2024 Form 1040, 1040-SR, or 1040-NR, line 15. If the amount
    ///       would have been a loss if you could enter a negative number on that line, enclose the
    ///       amount in parentheses"
    ///
    /// Stored SIGNED; the parentheses are the form's notation for a negative.
    pub line1: Usd,
    /// L2 — "Enter the loss from your 2024 Schedule D, line 21, as a positive amount"
    pub line2: Usd,
    /// L3 — "Combine lines 1 and 2. If zero or less, enter -0-"
    pub line3: Usd,
    /// L4 — "Enter the smaller of line 2 or line 3"
    ///
    /// ★ This is the whole of the fix. Line 4 is *the amount of loss the year actually absorbed*, and
    /// it is `min($3,000, taxable income + $3,000)` — which equals $3,000 only while line 1 ≥ 0. The
    /// flat rule this module replaces was line 4 hardcoded to line 2.
    pub line4: Usd,
    /// L5 — "Enter the loss from your 2024 Schedule D, line 7, as a positive amount"
    ///
    /// `-0-` when Schedule D line 7 is not a loss, per the interstitial instruction
    /// ([`GOTO_LINE5_OR_LINE9`]) — which then also sends the filer past lines 6–8.
    pub line5: Usd,
    /// L6 — "Enter any gain from your 2024 Schedule D, line 15. If a loss, enter -0-"
    ///
    /// `None` = skipped (Schedule D line 7 was not a loss).
    pub line6: Option<Usd>,
    /// L7 — "Add lines 4 and 6"
    ///
    /// `None` = skipped.
    pub line7: Option<Usd>,
    /// L8 — "Short-term capital loss carryover for 2025. Subtract line 7 from line 5. If zero or
    ///       less, enter -0-. If more than zero, also enter this amount on Schedule D, line 6"
    ///
    /// `None` = skipped, which is the same carryover as `-0-` and a different fact.
    pub line8: Option<Usd>,
    /// L9 — "Enter the loss from your 2024 Schedule D, line 15, as a positive amount"
    ///
    /// `None` = skipped (Schedule D line 15 was not a loss), per [`GOTO_LINE9_OR_SKIP`].
    pub line9: Option<Usd>,
    /// L10 — "Enter any gain from your 2024 Schedule D, line 7. If a loss, enter -0-"
    pub line10: Option<Usd>,
    /// L11 — "Subtract line 5 from line 4. If zero or less, enter -0-"
    ///
    /// ★ The §1212(b)(2) short-term-first ordering, in the form's own hand: whatever of line 4 the
    /// short-term loss did not use up is what the long-term loss gets.
    pub line11: Option<Usd>,
    /// L12 — "Add lines 10 and 11"
    pub line12: Option<Usd>,
    /// L13 — "Long-term capital loss carryover for 2025. Subtract line 12 from line 9. If zero or
    ///        less, enter -0-. If more than zero, also enter this amount on Schedule D, line 14"
    pub line13: Option<Usd>,
}

impl CapitalLossCarryoverWorksheet {
    /// Figure the worksheet, or `None` when it does not apply — "**Otherwise, you don't have any
    /// carryovers.**" ([`APPLICABILITY`]).
    ///
    /// ★★ **Where the old flat rule was exact, and where it broke.** With line 1 ≥ 0, line 3 (which
    /// is line 1 plus line 2) is ≥ line 2, so line 4 = line 2 = the whole §1211(b) allowance and every
    /// downstream line reduces to `carry = loss − min(loss, $3,000)` — the flat rule, exactly. The
    /// equivalence is destroyed the moment line 1 goes negative, because line 3 floors at zero and so
    /// does line 4 with
    /// it: the allowance the filer never got the benefit of stops being subtracted from the surviving
    /// loss. That is the *only* branch that moves, which is why applying the worksheet is a no-op on
    /// every household with positive taxable income (pinned by a KAT) and worth up to the whole
    /// $3,000 on the ones at the floor (pinned by another).
    pub fn figure(i: CapitalLossCarryoverInputs) -> Option<Self> {
        let z = Usd::ZERO;

        // APPLICABILITY: "…if your 2024 Schedule D, line 21, is a loss and (a) that loss is a smaller
        // loss than the loss on your 2024 Schedule D, line 16; or (b) if the amount on your 2024 Form
        // 1040 … line 15 would be less than zero…"
        let line21_is_a_loss = i.schedule_d_line21_loss > z;
        // (a) — a "smaller loss" comparison between two magnitudes; line 16 is a loss iff it is < 0.
        let a = i.schedule_d_line16 < z && i.schedule_d_line21_loss < -i.schedule_d_line16;
        // (b) — the pre-floor 1040 line 15.
        let b = i.form_1040_line15_signed < z;
        if !(line21_is_a_loss && (a || b)) {
            return None;
        }

        let line1 = i.form_1040_line15_signed;
        let line2 = i.schedule_d_line21_loss;
        // "Combine lines 1 and 2. If zero or less, enter -0-"
        let line3 = (line1 + line2).max(z);
        // "Enter the smaller of line 2 or line 3"
        let line4 = line2.min(line3);

        // "If line 7 of your 2024 Schedule D is a loss, go to line 5; otherwise, enter -0- on line 5
        //  and go to line 9."
        let st_is_a_loss = i.schedule_d_line7 < z;
        let line5 = if st_is_a_loss { -i.schedule_d_line7 } else { z };

        let (line6, line7, line8) = if st_is_a_loss {
            // "Enter any gain from your 2024 Schedule D, line 15. If a loss, enter -0-"
            let l6 = i.schedule_d_line15.max(z);
            // "Add lines 4 and 6"
            let l7 = line4 + l6;
            // "Subtract line 7 from line 5. If zero or less, enter -0-"
            let l8 = (line5 - l7).max(z);
            (Some(l6), Some(l7), Some(l8))
        } else {
            (None, None, None)
        };

        // "If line 15 of your 2024 Schedule D is a loss, go to line 9; otherwise, skip lines 9
        //  through 13."
        let (line9, line10, line11, line12, line13) = if i.schedule_d_line15 < z {
            // "Enter the loss from your 2024 Schedule D, line 15, as a positive amount"
            let l9 = -i.schedule_d_line15;
            // "Enter any gain from your 2024 Schedule D, line 7. If a loss, enter -0-"
            let l10 = i.schedule_d_line7.max(z);
            // "Subtract line 5 from line 4. If zero or less, enter -0-"
            let l11 = (line4 - line5).max(z);
            // "Add lines 10 and 11"
            let l12 = l10 + l11;
            // "Subtract line 12 from line 9. If zero or less, enter -0-"
            let l13 = (l9 - l12).max(z);
            (Some(l9), Some(l10), Some(l11), Some(l12), Some(l13))
        } else {
            (None, None, None, None, None)
        };

        Some(Self {
            line1,
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
        })
    }

    /// The §1212(b) carryforward-out this worksheet figures: line 8 short-term, line 13 long-term.
    ///
    /// A skipped line is no carryover of that character, which is what the form's two skip branches
    /// mean ("skip lines 9 through 13" is reached only when line 15 is not a loss).
    pub fn carryforward_out(&self) -> Carryforward {
        Carryforward {
            short: self.line8.unwrap_or(Usd::ZERO),
            long: self.line13.unwrap_or(Usd::ZERO),
        }
    }
}

/// The carryforward-out for a return, worksheet-applied: [`CapitalLossCarryoverWorksheet::figure`] and
/// its lines 8/13, or a zero carryforward when the worksheet says "you don't have any carryovers".
pub fn carryforward_out(i: CapitalLossCarryoverInputs) -> Carryforward {
    CapitalLossCarryoverWorksheet::figure(i)
        .map(|w| w.carryforward_out())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// The plan's **L4** household, line by line: Single, no wages, one $20,000 long-term loss.
    /// AGI −3,000, standard deduction 14,600 ⇒ line 1 = −17,600.
    #[test]
    fn l4_at_the_floor_carries_the_whole_loss() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(-17600),
            schedule_d_line7: Usd::ZERO,
            schedule_d_line15: dec!(-20000),
            schedule_d_line16: dec!(-20000),
            schedule_d_line21_loss: dec!(3000),
        })
        .expect("line 21 is a loss and line 1 is below zero — limb (b)");

        assert_eq!(w.line1, dec!(-17600));
        assert_eq!(w.line2, dec!(3000));
        assert_eq!(w.line3, Usd::ZERO, "−17,600 + 3,000 is below zero ⇒ -0-");
        assert_eq!(w.line4, Usd::ZERO, "NOTHING was absorbed");
        assert_eq!(w.line5, Usd::ZERO);
        assert_eq!(w.line6, None, "Schedule D line 7 is not a loss ⇒ skip 6–8");
        assert_eq!(w.line7, None);
        assert_eq!(w.line8, None);
        assert_eq!(w.line9, Some(dec!(20000)));
        assert_eq!(w.line10, Some(Usd::ZERO));
        assert_eq!(w.line11, Some(Usd::ZERO));
        assert_eq!(w.line12, Some(Usd::ZERO));
        assert_eq!(w.line13, Some(dec!(20000)));
        assert_eq!(
            w.carryforward_out(),
            Carryforward {
                short: Usd::ZERO,
                long: dec!(20000)
            }
        );
    }

    /// The plan's **L3** household — the same loss against $40,000 of wages. Taxable income is
    /// positive, the $3,000 IS absorbed, and the worksheet agrees with the flat rule to the cent.
    #[test]
    fn l3_at_positive_taxable_income_matches_the_flat_rule() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(22400),
            schedule_d_line7: Usd::ZERO,
            schedule_d_line15: dec!(-20000),
            schedule_d_line16: dec!(-20000),
            schedule_d_line21_loss: dec!(3000),
        })
        .expect("line 21 is a loss smaller than line 16's — limb (a)");

        assert_eq!(w.line3, dec!(25400));
        assert_eq!(w.line4, dec!(3000), "the full §1211(b) allowance was used");
        assert_eq!(w.line13, Some(dec!(17000)));
        assert_eq!(w.carryforward_out().long, dec!(17000));
    }

    /// Both characters in loss, positive TI: §1212(b)(2)'s SHORT-TERM-FIRST absorption, which the
    /// worksheet expresses as line 7 (= line 4 + line 6) against line 5, then line 11's remainder.
    #[test]
    fn short_term_loss_absorbs_the_allowance_first() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(50000),
            schedule_d_line7: dec!(-5000),
            schedule_d_line15: dec!(-2000),
            schedule_d_line16: dec!(-7000),
            schedule_d_line21_loss: dec!(3000),
        })
        .unwrap();
        assert_eq!(w.line4, dec!(3000));
        assert_eq!(w.line5, dec!(5000));
        assert_eq!(w.line6, Some(Usd::ZERO), "line 15 is a loss ⇒ -0-");
        assert_eq!(w.line7, Some(dec!(3000)));
        assert_eq!(w.line8, Some(dec!(2000)), "5,000 − 3,000 absorbed");
        assert_eq!(w.line11, Some(Usd::ZERO), "3,000 − 5,000 ⇒ -0-");
        assert_eq!(
            w.line13,
            Some(dec!(2000)),
            "the long-term loss is untouched"
        );
    }

    /// A short-term loss cross-netted against a long-term GAIN: line 6 picks the gain up, so the
    /// short-term carryover is the residue after BOTH the gain and the allowance.
    #[test]
    fn a_long_term_gain_absorbs_short_term_loss_through_line6() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(50000),
            schedule_d_line7: dec!(-20000),
            schedule_d_line15: dec!(2000),
            schedule_d_line16: dec!(-18000),
            schedule_d_line21_loss: dec!(3000),
        })
        .unwrap();
        assert_eq!(w.line6, Some(dec!(2000)));
        assert_eq!(w.line7, Some(dec!(5000)));
        assert_eq!(w.line8, Some(dec!(15000)));
        assert_eq!(w.line9, None, "line 15 is a GAIN ⇒ skip lines 9 through 13");
        assert_eq!(w.line13, None);
        assert_eq!(w.carryforward_out().long, Usd::ZERO);
    }

    /// The mirror: a long-term loss cross-netted against a short-term GAIN, through line 10.
    #[test]
    fn a_short_term_gain_absorbs_long_term_loss_through_line10() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(50000),
            schedule_d_line7: dec!(2000),
            schedule_d_line15: dec!(-20000),
            schedule_d_line16: dec!(-18000),
            schedule_d_line21_loss: dec!(3000),
        })
        .unwrap();
        assert_eq!(w.line5, Usd::ZERO, "line 7 is a gain ⇒ -0- on line 5");
        assert_eq!(w.line8, None, "…and skip to line 9");
        assert_eq!(w.line10, Some(dec!(2000)));
        assert_eq!(w.line11, Some(dec!(3000)));
        assert_eq!(w.line12, Some(dec!(5000)));
        assert_eq!(w.line13, Some(dec!(15000)));
    }

    /// "**Otherwise, you don't have any carryovers.**" — a $2,000 total loss, fully deducted against
    /// positive income, is neither limb (a) nor limb (b).
    #[test]
    fn a_fully_absorbed_loss_has_no_carryover_and_no_worksheet() {
        assert_eq!(
            CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
                form_1040_line15_signed: dec!(30000),
                schedule_d_line7: Usd::ZERO,
                schedule_d_line15: dec!(-2000),
                schedule_d_line16: dec!(-2000),
                schedule_d_line21_loss: dec!(2000),
            }),
            None
        );
    }

    /// ★ …but the SAME $2,000 loss in a wiped-out year DOES carry, entirely — limb (b) alone. This is
    /// the branch the flat rule gets wrong for every dollar of it, not merely for the excess over
    /// $3,000, and no household in the double-oracle corpus sits here.
    #[test]
    fn a_small_loss_at_the_floor_carries_in_full() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(-5000),
            schedule_d_line7: Usd::ZERO,
            schedule_d_line15: dec!(-2000),
            schedule_d_line16: dec!(-2000),
            schedule_d_line21_loss: dec!(2000),
        })
        .expect("limb (b): line 15 would be below zero");
        assert_eq!(w.line3, Usd::ZERO);
        assert_eq!(w.line4, Usd::ZERO);
        assert_eq!(w.line13, Some(dec!(2000)));
    }

    /// A gain year: line 21 is not a loss, so the worksheet is not used at all.
    #[test]
    fn a_gain_year_has_no_worksheet() {
        assert_eq!(
            CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
                form_1040_line15_signed: dec!(80000),
                schedule_d_line7: dec!(1000),
                schedule_d_line15: dec!(9000),
                schedule_d_line16: dec!(10000),
                schedule_d_line21_loss: Usd::ZERO,
            }),
            None
        );
        assert_eq!(
            carryforward_out(CapitalLossCarryoverInputs {
                form_1040_line15_signed: dec!(80000),
                schedule_d_line7: dec!(1000),
                schedule_d_line15: dec!(9000),
                schedule_d_line16: dec!(10000),
                schedule_d_line21_loss: Usd::ZERO,
            }),
            Carryforward::default()
        );
    }

    /// **PARTIAL absorption — the branch between the two extremes.** Line 1 = −1,000 means the year
    /// could still use $2,000 of the allowance, so line 4 is $2,000, not $0 and not $3,000. A fix that
    /// merely switched the flat rule off below the floor (carry the whole loss whenever TI ≤ 0) would
    /// pass both L3 and L4 and fail HERE by $2,000.
    #[test]
    fn between_the_two_extremes_line4_is_partial() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(-1000),
            schedule_d_line7: Usd::ZERO,
            schedule_d_line15: dec!(-20000),
            schedule_d_line16: dec!(-20000),
            schedule_d_line21_loss: dec!(3000),
        })
        .unwrap();
        assert_eq!(w.line3, dec!(2000));
        assert_eq!(w.line4, dec!(2000));
        assert_eq!(w.line13, Some(dec!(18000)));
    }

    /// **MFS**: §1211(b)'s allowance halves to $1,500, and the worksheet inherits that through line 2
    /// without knowing anything about filing status — it reads Schedule D line 21, which already is
    /// the limited figure.
    #[test]
    fn mfs_inherits_the_halved_allowance_through_line2() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: dec!(40000),
            schedule_d_line7: Usd::ZERO,
            schedule_d_line15: dec!(-20000),
            schedule_d_line16: dec!(-20000),
            schedule_d_line21_loss: dec!(1500),
        })
        .unwrap();
        assert_eq!(w.line4, dec!(1500));
        assert_eq!(w.line13, Some(dec!(18500)));
    }

    /// Exactly AT the floor — line 1 = 0 — is the boundary between the two regimes, and it belongs to
    /// the flat rule: line 3 = 0 + 3,000 = 3,000, so line 4 is the full allowance. Limb (a) admits it.
    #[test]
    fn line1_exactly_zero_still_absorbs_the_full_allowance() {
        let w = CapitalLossCarryoverWorksheet::figure(CapitalLossCarryoverInputs {
            form_1040_line15_signed: Usd::ZERO,
            schedule_d_line7: Usd::ZERO,
            schedule_d_line15: dec!(-20000),
            schedule_d_line16: dec!(-20000),
            schedule_d_line21_loss: dec!(3000),
        })
        .unwrap();
        assert_eq!(w.line4, dec!(3000));
        assert_eq!(w.line13, Some(dec!(17000)));
    }

    /// The equivalence claim in [`CapitalLossCarryoverWorksheet::figure`]'s doc, EXECUTED rather than
    /// asserted in prose: over a grid of loss shapes at NON-NEGATIVE line 1, the worksheet and the
    /// frozen engine's flat `net_1222` rule agree on both characters — and at negative line 1 they
    /// must not be presumed to.
    #[test]
    fn at_nonnegative_line1_the_worksheet_equals_the_frozen_flat_rule() {
        use crate::tax::compute::net_1222;
        let limit = dec!(3000);
        let shapes = [
            (dec!(0), dec!(-20000)),
            (dec!(-20000), dec!(0)),
            (dec!(-5000), dec!(-2000)),
            (dec!(-2000), dec!(-5000)),
            (dec!(2000), dec!(-20000)),
            (dec!(-20000), dec!(2000)),
            (dec!(-1000), dec!(-1000)),
            (dec!(-3000), dec!(0)),
            (dec!(0), dec!(-1500)),
            (dec!(-40000), dec!(-40000)),
        ];
        for (st, lt) in shapes {
            let n = net_1222(st, lt, Usd::ZERO, Usd::ZERO, Usd::ZERO, limit);
            let flat = Carryforward {
                short: n.st_carry,
                long: n.lt_carry,
            };
            for line1 in [Usd::ZERO, dec!(1), dec!(2999), dec!(50000)] {
                let got = carryforward_out(CapitalLossCarryoverInputs {
                    form_1040_line15_signed: line1,
                    schedule_d_line7: n.st_net,
                    schedule_d_line15: n.lt_net,
                    schedule_d_line16: n.st_net + n.lt_net,
                    schedule_d_line21_loss: n.loss_deduction,
                });
                assert_eq!(
                    got, flat,
                    "line1={line1}, ST={st}, LT={lt}: the worksheet must be a NO-OP at \
                     non-negative 1040 line 15"
                );
            }
        }
    }
}
