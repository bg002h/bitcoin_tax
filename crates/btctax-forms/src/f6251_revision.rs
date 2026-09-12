//! ★★★ **The year-varying CELLS of the OBBBA-era Form 6251 — one entry per revision, held by the
//! compiler.**
//!
//! ## The defect this module exists to make impossible
//!
//! [`crate::map::Form6251ObbbaMap`] is ONE field map serving TWO revisions of Form 6251. That is not
//! laziness: `xtask form-delta design/forms/2025/f6251--2025.pdf
//! design/forms/2026/f6251--2026-DRAFT.pdf` reports the AcroForm delta as **62 fields, 0 renamed, 0
//! moved** (`design/TY2026_WORK_LIST.md`), so every FQN, every `/Rect` and every `/MaxLen` is
//! identical. A single struct genuinely does fill both years' PDFs.
//!
//! ★★★ **And that is exactly the hazard.** The two revisions print DIFFERENT TEXT on eight numbered
//! lines, and two of those differences are **cross-references** — pointers at another document's
//! line number, where being one line off is a wrong figure and not a typo:
//!
//! | line | TY2025 (final) | TY2026 (draft) | kind |
//! |---|---|---|---|
//! | **1a** | "Subtract Schedule 1-A (Form 1040), line **37**, …" | "… line **43**, …" | **cross-reference** |
//! | **7** | "…capital gain distributions directly on Form 1040 or 1040-SR, line **7**;" | "… line **7a**;" | **cross-reference** |
//! | 4 | MFS threshold **$900,350** | **$640,200** | printed constant |
//! | 5, 7, 18, 19, 25, 39 | exemption table, 26/28% breakpoints, 0 %/15 % brackets | all moved | printed constants |
//!
//! ★★★ **37 → 43 IS A COLLISION, NOT A RENUMBER — and this paragraph used to say "renumber".** Read
//! off the two extracts' own label columns:
//!
//! | Schedule 1-A | line 37 | line 43 |
//! |---|---|---|
//! | **TY2025 final** (`f1040s1a--2025.txt:108`) | "Enhanced deduction for seniors. Add lines 36a and 36b" | *(the schedule ends at 38)* |
//! | **TY2026 draft** (`f1040s1a--2026-DRAFT.txt:213,222`) | **"Enter the amount from line 3"** — modified AGI | "Enhanced deduction for seniors. Add lines 42a and 42b" |
//!
//! The senior subtotal moved 37 → 43 **and the vacated number was refilled**, so one revision prints
//! a ≤$6,000 deduction where the next prints a six-figure income. Line 1a subtracts it from 1040 line
//! 14 and line 1b subtracts 1a from line 11b, so substituting MAGI for the deduction drives 1a
//! sharply negative and **overstates the AMT base by ≈MAGI** — taxpayer-adverse, six figures, and the
//! OPPOSITE direction from the TY2025 understatement the core-side kills pin. A renumber would have
//! been an off-by-one between two neighbouring deduction subtotals; this is not that.
//!
//! Read the wrong one and the AMT base is wrong while every instrument stays green: the field map
//! cannot see it (same FQNs), the geometry fixture cannot see it (same rects), read-back cannot see
//! it (the value lands in the box it was sent to), and both oracles take the figure as INPUT. It is
//! the Form 6251 line-33 defect one line up — a cross-reference transcribed one line off, which on
//! that occasion inflated the tentative minimum tax by **$200,000** on one vector.
//!
//! ## How it is closed
//!
//! [`revision`] is an **`_`-free match over [`LineSet`]**, so `f6251/2026` cannot be pointed at
//! [`crate::line_set::Schema::Form6251ObbbaMap`] without an arm here — that is `E0004`, a build
//! error, per `CLAUDE.md`'s *"derive the list, or make the compiler hold it."* And an arm written by
//! copying TY2025's cells does not survive either: `tests/f6251_obbba.rs` asserts every cell below
//! is **verbatim in that revision's own archived extract**, so TY2025's "line 37" reds against
//! `f6251--2026.txt`.
//!
//! ★★★ **And the third leg, which closes the COMPUTATION side:**
//! [`ObbbaRevision::schedule_1a_line_agreeing_with`] compares the line this revision PRINTS against
//! the line the figure was actually read off — carried from core as
//! `btctax_core::tax::schedule_1a::SeniorDeductionSubtotal`, which can only be obtained from the
//! schedule revision that printed it. ★ **That sentence only became true on 2026-09-11** (the step-5
//! re-verification's F1): the vouched type reached core's line-1 *rule* and was then unpacked into a
//! bare `schedule_1a_line: u32` on `Form6251Line1::Y2025`, so the number arriving here was one its
//! caller could have chosen. The variant carries the type now, and the forge is a named test-only
//! route that `xtask::forge_reach_check` keeps out of shipped code.
//! The emitter calls it, so a chain reusing TY2025's Schedule 1-A
//! line while filling a revision that cites 43 **refuses instead of printing a figure**. Before that
//! join existed the emitter resolved the right line number and dropped it on the floor, and the
//! cross-reference on the core side was a FIELD NAME (`schedule_1a_l37`) — the one form of a cell no
//! test, no `_`-free match and no extract comparison can read.
//!
//! ## What is deliberately NOT here — the printed CONSTANTS
//!
//! The six constant-only lines (5, 7's third bullet, 18, 19, 25, 39) and line 4's dollar figure are
//! **not transcribed as a number here.** What IS held per revision is the line-4 SENTENCE whose
//! parenthetical names the threshold, so [`ObbbaRevision::mfs_threshold_printed`] parses out what the
//! paper prints and it can be compared against the params by whoever owns both — and `btctax-forms`
//! does not: it does not depend on `btctax-adapters`, where the per-year params live. **That join is
//! the honest boundary of this module**, stated rather than papered over (`CLAUDE.md`, rule 3).
//!
//! ★★ **The reason is the SINGLE-AUTHORITY rule, and the "it is already per-year in `AmtParams`"
//! sentence that used to stand here was measurably false for the one revision this module holds.**
//! Measured over the workspace: **every** `AmtParams::mfs_kicker_start` literal is `dec!(875950)`
//! (TY2024) or `dec!(640200)` (TY2026) — `crates/btctax-adapters/src/tax_tables.rs:185,315` and the
//! fixtures — and **none is TY2025's printed $900,350**. The rule is still the right one: a figure
//! typed twice is two lists that can disagree, and the sentence-plus-parser keeps one authority (the
//! form) with a derived reading. But the claim that a first authority already exists for *this*
//! revision's threshold does not hold, and the gap it hides is real: `mfs_threshold_printed()` has
//! nothing to be compared against for TY2025 today (`FOLLOWUPS.md` FR-120).

use crate::line_set::LineSet;
use btctax_core::Usd;

/// One revision of the OBBBA-era Form 6251: the cells whose PRINTED TEXT differs from its siblings'.
///
/// Every field is the form's own words, from the TEXT LAYER of that revision's archived PDF
/// (`design/forms/extract/<extract>.txt`) — never from the rendered page, and never from a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObbbaRevision {
    /// The revision these cells belong to.
    pub line_set: LineSet,
    /// Archive stem of the extract every cell below is asserted verbatim against — the file is
    /// `design/forms/extract/<extract>.txt`.
    ///
    /// ★ A DRAFT extract (`…--2026-DRAFT`) is not an authority and must never appear here: the
    /// AUTHORITY/DRAFT split (`design/forms/MANIFEST.json`, `Entry::is_draft`) exists to make that
    /// enforceable, and a revision wired against a draft would pin this build to text the IRS has
    /// not finalised.
    pub extract: &'static str,

    /// **L1a, verbatim.** Carries the Schedule 1-A line number the AMT base subtracts — the single
    /// most dangerous cell on this form, because the two revisions differ here and nothing else can
    /// tell them apart. Read by [`ObbbaRevision::schedule_1a_line`], which parses the number OUT of
    /// this sentence rather than carrying it as a second key that could disagree with it (the
    /// `SubtractSentence` precedent, `crate::map`).
    pub line1a: &'static str,

    /// **L4, verbatim** — the AMTI line, whose parenthetical names the MFS line-4 kicker's start.
    ///
    /// ★ "Combine lines **1b** through 3", not "lines 1 through 3": line 1a is EXCLUDED from the
    /// combine. Reading 1a into AMTI would put the *deduction* where the income goes.
    ///
    /// ★★ The dollar figure is EVIDENCE here, never a source. The value btctax computes with is
    /// `AmtParams::mfs_kicker_start` for the year; [`ObbbaRevision::mfs_threshold_printed`] exposes
    /// what the paper prints so the two can be compared where both are in scope.
    pub line4: &'static str,

    /// **L7, second bullet — the clause that routes to Part III**, verbatim, up to its semicolon.
    ///
    /// ★ WHY A CLAUSE AND NOT THE WHOLE BULLET, stated rather than silently truncated: the text
    /// layer interrupts this bullet with the line-7 box marker (`…complete Part III on the . . 7
    /// back and enter…`, extract rows 65–68), so no contiguous quote of the full bullet is verbatim
    /// in it. The clause chosen is the one that ENDS AT A SEMICOLON and carries the year-varying
    /// cell; the rest of the bullet says the same thing in both revisions.
    ///
    /// ★★ The cross-reference here is the IRS's own stale one, and transcribing it faithfully is the
    /// point: the **TY2025** Form 1040 already labels capital gain as line **7a**
    /// (`design/forms/extract/f1040--2025.txt` row 81), while the TY2025 Form 6251 still says "line
    /// 7". TY2026's draft corrects it to "line 7a". A transcription that "helpfully" wrote 7a for
    /// TY2025 would be adjudicating against the form, which is backwards — the IRS PDF is the
    /// authority and an inconsistency in it is recorded, not repaired.
    pub line7_capital_gain_clause: &'static str,
}

impl ObbbaRevision {
    /// The Schedule 1-A line number line 1a subtracts, **parsed out of [`Self::line1a`]**.
    ///
    /// Never a second field: a number typed beside the sentence could disagree with it, and then
    /// two readers of this struct would compute two different AMT bases. Returns `None` if the
    /// sentence does not have the shape the form prints, which a test turns into a red rather than a
    /// default.
    #[must_use]
    pub fn schedule_1a_line(&self) -> Option<u32> {
        let tail = after_the_only(self.line1a, "Schedule 1-A (Form 1040), line ")?;
        let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    }

    /// ★★★ **THE JOIN — the line THIS revision prints, checked against the line the FIGURE was read
    /// off.** Returns the agreed line number, or a refusal naming both.
    ///
    /// **Why this exists at all.** The two halves of the most dangerous cell on this form live in
    /// different crates: the form's own sentence here, and the money in
    /// `btctax_core::tax::form6251::Form6251Line1::Y2025`. Until this method existed the emitter
    /// resolved [`Self::schedule_1a_line`] and **discarded the value** — an existence check on the
    /// sentence, never a comparison — while the cross-reference on the core side was spelled into a
    /// FIELD NAME (`schedule_1a_l37`), the one form of a cell that no test, no `_`-free match and no
    /// extract comparison can read. Both sides were individually pinned and nothing joined them.
    ///
    /// ★★ **What it stops.** `f6251/2026` reuses this schema (62 fields, 0 renamed), so a TY2026 arm
    /// that reuses core's `Y2025` shape compiles — the TY2026 Schedule 1-A really does print a line
    /// 37, it just means *"Enter the amount from line 3"*, i.e. modified AGI. The figure would be a
    /// six-figure income where a ≤$6,000 deduction belongs, the AMT base overstated by ≈MAGI, and
    /// every other instrument green: same FQNs, same rects, the value lands in the box it was sent
    /// to, and both oracles take it as INPUT. This is the one check that can see it, because it is
    /// the one place holding the printed sentence and the figure's provenance at once.
    ///
    /// ★ It fails closed on BOTH legs: a sentence with no parsable cross-reference refuses too,
    /// rather than defaulting to a line nobody printed.
    pub fn schedule_1a_line_agreeing_with(
        &self,
        read_by_the_computation: u32,
    ) -> Result<u32, crate::FormsError> {
        let printed = self.schedule_1a_line().ok_or_else(|| {
            crate::FormsError::Structure(format!(
                "Form 6251 {}: the revision's line-1a sentence carries no Schedule 1-A line number \
                 — the AMT base's source line is unknown and the form must not be filed.",
                self.line_set.as_str()
            ))
        })?;
        if printed != read_by_the_computation {
            return Err(crate::FormsError::Structure(format!(
                "Form 6251 {}: line 1a on this revision subtracts Schedule 1-A line {printed}, but \
                 the figure was read off line {read_by_the_computation}. These are DIFFERENT \
                 QUANTITIES, not a renumber: the senior-deduction subtotal moved 37 -> 43 for \
                 TY2026 and line 37 was REFILLED with \"Enter the amount from line 3\" (modified \
                 AGI), so substituting one for the other overstates the AMT base by approximately \
                 MAGI. Transcribe this revision's Schedule 1-A and read the subtotal off ITS own \
                 line, through that schedule's own accessor — never re-point the previous \
                 revision's.",
                self.line_set.as_str()
            )));
        }
        Ok(printed)
    }

    /// The MFS line-4 kicker threshold **printed on the page**, parsed out of [`Self::line4`].
    ///
    /// ★ This is what the PAPER says, for comparison against `AmtParams::mfs_kicker_start`. It is
    /// not a source of truth for any computation in this crate — nothing here computes AMT.
    #[must_use]
    pub fn mfs_threshold_printed(&self) -> Option<Usd> {
        let tail = after_the_only(self.line4, "is more than $")?;
        let figure: String = tail
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ',')
            .filter(|c| *c != ',')
            .collect();
        figure.parse().ok()
    }

    /// The Form 1040 line the Part III routing test reads, **parsed out of
    /// [`Self::line7_capital_gain_clause`]** — `"7"` in TY2025, `"7a"` in TY2026's draft.
    #[must_use]
    pub fn form_1040_capital_gain_line(&self) -> Option<&'static str> {
        let tail = after_the_only(
            self.line7_capital_gain_clause,
            "Form 1040 or 1040-SR, line ",
        )?;
        tail.split_once(';').map(|(n, _)| n)
    }
}

/// The text after `anchor` in `sentence`, **and `None` unless the anchor occurs EXACTLY ONCE**.
///
/// ★★ `split_once` silently takes the FIRST occurrence, so a future revision whose clause repeated
/// its anchor before the delimiter would have a second reading available and nothing would say which
/// one was taken. The ambiguity was not expressible: only the zero-occurrence case failed closed
/// (seam review N-1). Two occurrences now refuse exactly as zero does — a cross-reference that can be
/// read two ways is not a cross-reference, and on this cell the difference is the AMT base.
fn after_the_only<'a>(sentence: &'a str, anchor: &str) -> Option<&'a str> {
    let mut it = sentence.split(anchor);
    let _before = it.next()?;
    let tail = it.next()?;
    it.next().is_none().then_some(tail)
}

/// **TY2025 — the final document** (`design/forms/2025/f6251--2025.pdf`, sha256 `6995bfd2…`,
/// "Form 6251 (2025) Created 9/17/25"). Transcribed from the text layer, rows 18–19, 44–45 and
/// 65–66 of `design/forms/extract/f6251--2025.txt`.
const F6251_2025: ObbbaRevision = ObbbaRevision {
    line_set: LineSet::F6251_2025,
    extract: "f6251--2025",
    line1a:
        "Subtract Schedule 1-A (Form 1040), line 37, from Form 1040, 1040-SR, or 1040-NR, line 14",
    line4: "Alternative minimum taxable income. Combine lines 1b through 3. (If married filing \
            separately and line 4 is more than $900,350, see instructions.)",
    line7_capital_gain_clause:
        "If you reported capital gain distributions directly on Form 1040 or 1040-SR, line 7;",
};

/// ★★★ **THE TRAP.** Exhaustive over [`LineSet`] with **no `_` arm**: adding a revision — and in
/// particular pointing `f6251/2026` at [`crate::line_set::Schema::Form6251ObbbaMap`] — is a compile
/// error until its own cells are stated here.
///
/// `None` means "this revision is not the OBBBA-era Form 6251", which every non-6251 line set is.
/// A 6251 revision on this schema that answered `None` would compile, so
/// `tests/f6251_obbba.rs::every_revision_on_this_schema_has_its_year_varying_cells` closes that leg
/// too — derived from `LineSet::ALL` and [`crate::line_set::schema`], never from a list.
#[must_use]
pub fn revision(ls: LineSet) -> Option<&'static ObbbaRevision> {
    match ls {
        LineSet::F6251_2025 => Some(&F6251_2025),

        // ── Not this schema. Listed rather than swept by `_`, which is the whole mechanism. ──
        LineSet::F1040_2024
        | LineSet::F1040s1_2024
        | LineSet::F1040s2_2024
        | LineSet::F1040s3_2024
        | LineSet::F1040sa_2024
        | LineSet::F1040sb_2024
        | LineSet::F1040sc_2024
        | LineSet::F1040v_2024
        | LineSet::F4868_2024
        | LineSet::F6251_2024
        | LineSet::F8275_2024
        | LineSet::F8283_2024
        | LineSet::F8949_2024
        | LineSet::F8889_2024
        | LineSet::F8959_2024
        | LineSet::F8960_2024
        | LineSet::F8995_2024
        | LineSet::F8995a_2024
        | LineSet::ScheduleD_2024
        | LineSet::ScheduleSe_2024
        | LineSet::F1040_2025
        | LineSet::F1040s1a_2025
        | LineSet::F1040s2_2025
        | LineSet::F1040s3_2025
        | LineSet::F1040sa_2025
        | LineSet::F1040sb_2025
        | LineSet::F1040sc_2025
        | LineSet::F1040v_2025
        | LineSet::F4868_2025
        | LineSet::F8283_2025
        | LineSet::F8949_2025
        | LineSet::F8889_2025
        | LineSet::F8959_2025
        | LineSet::F8960_2025
        | LineSet::F8995_2025
        | LineSet::ScheduleD_2025
        | LineSet::ScheduleSe_2025 => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// The three parsed accessors, on the one revision that exists. The numbers are read OUT of the
    /// form's own sentences, so this is the assertion that the parse works — the assertion that the
    /// sentences are the form's is `tests/f6251_obbba.rs`, against the extract.
    #[test]
    fn the_cross_references_and_the_threshold_parse_out_of_the_form_s_own_sentences() {
        let r = revision(LineSet::F6251_2025).expect("TY2025 is on this schema");
        assert_eq!(
            r.schedule_1a_line(),
            Some(37),
            "TY2025 line 1a subtracts Schedule 1-A line 37 — the SENIOR deduction subtotal, not \
             line 38's total, and not TY2026's 43"
        );
        assert_eq!(r.mfs_threshold_printed(), Some(dec!(900350)));
        assert_eq!(r.form_1040_capital_gain_line(), Some("7"));
    }

    /// ★ B1 — the accessors FAIL CLOSED on a sentence that does not have the form's shape, rather
    /// than defaulting to a number nobody printed. A silent `0` or `1` here is a wrong AMT base.
    #[test]
    fn a_sentence_without_the_cross_reference_yields_none_rather_than_a_default() {
        let bogus = ObbbaRevision {
            line_set: LineSet::F6251_2025,
            extract: "f6251--2025",
            line1a: "Subtract the senior deduction from adjusted gross income",
            line4: "Alternative minimum taxable income. Combine lines 1b through 3.",
            line7_capital_gain_clause:
                "If you reported capital gain distributions, see instructions",
        };
        assert_eq!(bogus.schedule_1a_line(), None);
        assert_eq!(bogus.mfs_threshold_printed(), None);
        assert_eq!(bogus.form_1040_capital_gain_line(), None);
    }

    /// ★★★ **THE CENTRAL KILL — the TY2026 arm the source used to advertise as "a one-line edit",
    /// refused.**
    ///
    /// The scenario, exactly as a future porter would reach it: TY2026's finals land, the field map is
    /// reused (62 fields, 0 renamed, 0 moved), a revision is stated whose line-1a sentence is TY2026's
    /// own — **verbatim from `design/forms/extract/f6251--2026-DRAFT.txt:51-52`, citing Schedule 1-A
    /// line 43** — and the `2026 =>` arm in core reuses the `Y2025` shape while still reading the
    /// TY2025 schedule's senior subtotal, i.e. line **37**. That compiles: the TY2026 schedule really
    /// does print a line 37. It means *"Enter the amount from line 3"* — modified AGI — so the AMT base
    /// would be overstated by ≈MAGI with every other instrument green.
    ///
    /// This is the check that stops it, and it is watched in BOTH directions: agreement is silent,
    /// disagreement refuses, and the message names both numbers so the next reader is not sent hunting.
    #[test]
    fn a_revision_citing_line_43_refuses_a_figure_read_off_line_37() {
        // The one wired revision agrees with itself — the rule must not refuse the real case.
        let ty2025 = revision(LineSet::F6251_2025).expect("TY2025 is on this schema");
        assert_eq!(
            ty2025.schedule_1a_line_agreeing_with(37).ok(),
            Some(37),
            "TY2025 prints line 37 and the TY2025 chain reads line 37; this must be SILENT"
        );

        // ★ A TY2026-shaped revision. `line_set` only names the row in the message; what makes this
        //   the real scenario is the SENTENCE, which is the TY2026 draft's own.
        let ty2026 = ObbbaRevision {
            line_set: LineSet::F6251_2025,
            extract: "f6251--2026",
            line1a: "Subtract Schedule 1-A (Form 1040), line 43, from Form 1040, 1040-SR, or \
                     1040-NR, line 14",
            line4: F6251_2025.line4,
            line7_capital_gain_clause: F6251_2025.line7_capital_gain_clause,
        };
        assert_eq!(
            ty2026.schedule_1a_line(),
            Some(43),
            "the plant must really cite 43, or the kill proves nothing"
        );
        let err = ty2026
            .schedule_1a_line_agreeing_with(37)
            .expect_err(
                "a revision printing \"Schedule 1-A line 43\" ACCEPTED a figure read off line 37 — \
                 that is modified AGI in the senior deduction's slot and the AMT base is overstated \
                 by roughly the whole AGI",
            )
            .to_string();
        assert!(
            err.contains("line 43") && err.contains("line 37"),
            "the refusal must name BOTH lines, because the whole defect is that they look \
             interchangeable: {err}"
        );
        assert!(
            err.contains("not a renumber"),
            "…and must say why substituting one for the other is not an off-by-one: {err}"
        );

        // …and the OTHER direction: a TY2025 revision handed a figure read off line 43.
        assert!(
            ty2025.schedule_1a_line_agreeing_with(43).is_err(),
            "the comparison must be symmetric — a chain that read line 43 must not fill a form \
             citing 37 either"
        );
    }

    /// ★ Seam review N-1 — an anchor that occurs TWICE is ambiguous, and ambiguity now fails closed.
    ///
    /// `split_once` takes the FIRST occurrence silently, so a future revision whose clause repeated
    /// its anchor had a second reading available and nothing said which was taken. Only the
    /// zero-occurrence case failed closed; this is the case that was not expressible.
    #[test]
    fn a_repeated_anchor_is_ambiguous_and_yields_none_rather_than_the_first_reading() {
        let doubled = ObbbaRevision {
            line_set: LineSet::F6251_2025,
            extract: "f6251--2025",
            // Both readings are present: 43 first, 37 second. Taking "the first" would be a silent
            // choice between a deduction and an income.
            line1a: "Subtract Schedule 1-A (Form 1040), line 43, from Form 1040, 1040-SR, or \
                     1040-NR, line 14, or Schedule 1-A (Form 1040), line 37, if applicable",
            line4: "Alternative minimum taxable income. Combine lines 1b through 3. (If married \
                    filing separately and line 4 is more than $900,350, see instructions, and if \
                    line 4 is more than $1,000,000 see the worksheet.)",
            line7_capital_gain_clause:
                "If you reported capital gain distributions directly on Form 1040 or 1040-SR, line \
                 7, or on Form 1040 or 1040-SR, line 7a;",
        };
        assert_eq!(doubled.schedule_1a_line(), None);
        assert_eq!(doubled.mfs_threshold_printed(), None);
        assert_eq!(doubled.form_1040_capital_gain_line(), None);
        // ★ …and the refusal is the JOIN's too, not only the accessor's: an unreadable sentence must
        //   not become "agrees with whatever was asked".
        assert!(doubled.schedule_1a_line_agreeing_with(37).is_err());
        assert!(doubled.schedule_1a_line_agreeing_with(43).is_err());
    }

    /// ★★ Every revision this build wires onto the OBBBA field map has cells here. Derived from
    /// `LineSet::ALL` and the schema match — no list to go stale. (Restated as an integration test
    /// in `tests/f6251_obbba.rs`, which also holds the cells against the extract; kept here so the
    /// unit-test binary alone cannot be green on a missing arm.)
    #[test]
    fn every_obbba_revision_has_cells() {
        for ls in LineSet::ALL {
            if crate::line_set::schema(*ls) == crate::line_set::Schema::Form6251ObbbaMap {
                assert!(
                    revision(*ls).is_some(),
                    "{} is wired to the OBBBA field map but states none of its own year-varying \
                     cells — it would inherit another revision's Schedule 1-A line number",
                    ls.as_str()
                );
            }
        }
    }
}
