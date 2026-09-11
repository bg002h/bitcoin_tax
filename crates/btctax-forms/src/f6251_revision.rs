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
//! Schedule 1-A line 37 is the *senior* deduction subtotal and line 43 is its TY2026 renumber. Read
//! the wrong one and the AMT base is wrong while every instrument stays green: the field map cannot
//! see it (same FQNs), the geometry fixture cannot see it (same rects), read-back cannot see it (the
//! value lands in the box it was sent to), and both oracles take the figure as INPUT. It is the
//! Form 6251 line-33 defect one line up — a cross-reference transcribed one line off, which on that
//! occasion inflated the tentative minimum tax by **$200,000** on one vector.
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
//! ## What is deliberately NOT here — the printed CONSTANTS
//!
//! The six constant-only lines (5, 7's third bullet, 18, 19, 25, 39) and line 4's dollar figure are
//! **not transcribed per revision**, and that is the rule rather than an omission: those figures are
//! already per-year in `btctax_core::tax::tables::AmtParams` and the year's `TaxTable`
//! (line 4's is `AmtParams::mfs_kicker_start` — `dec!(875950)` for TY2024, `dec!(640200)` for
//! TY2026). Re-typing one here would create a **second authority** for a number that already has
//! one, which is how two lists come to disagree. What IS held per revision is the line-4 sentence
//! whose parenthetical *names* the threshold, so [`ObbbaRevision::mfs_threshold_printed`] can be
//! compared against the params by whoever owns both — and `btctax-forms` does not: it does not
//! depend on `btctax-adapters`, where the per-year params live. **That join is the honest boundary
//! of this module**, stated rather than papered over (`CLAUDE.md`, rule 3).

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
        let tail = self.line1a.split_once("Schedule 1-A (Form 1040), line ")?.1;
        let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    }

    /// The MFS line-4 kicker threshold **printed on the page**, parsed out of [`Self::line4`].
    ///
    /// ★ This is what the PAPER says, for comparison against `AmtParams::mfs_kicker_start`. It is
    /// not a source of truth for any computation in this crate — nothing here computes AMT.
    #[must_use]
    pub fn mfs_threshold_printed(&self) -> Option<Usd> {
        let tail = self.line4.split_once("is more than $")?.1;
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
        let tail = self
            .line7_capital_gain_clause
            .split_once("Form 1040 or 1040-SR, line ")?
            .1;
        tail.split_once(';').map(|(n, _)| n)
    }
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
