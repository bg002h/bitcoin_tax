//! ★★★ **N1 — the CONFORMANCE CHECKER for the §1211/§1212 Capital Loss Carryover Worksheet.**
//!
//! `btctax_core::tax::capital_loss_carryover` transcribes the *Capital Loss Carryover Worksheet —
//! Lines 6 and 14* from the 2025 Schedule D instructions. `CLAUDE.md`: *"Is every form line present?
//! Does each doc comment match the instruction text? are ASSERTIONS, not opinions."* So this file
//! asserts them, on every commit, rather than asking a reviewer to read a form.
//!
//! **Three checks, because each is satisfiable by a defect the others miss.**
//!
//! 1. **Verbatim** — every quote in `LINES` (plus the applicability sentence and the two interstitial
//!    "go to line N" instructions) appears in the extract, modulo whitespace. Catches a paraphrase.
//! 2. **Whole** — and each match must END where the form ends the instruction ([`Terminator`]).
//!    ★★ Added because mutation caught check (1) **passing a truncation**: a shortened citation is a
//!    substring of the real one, which is the Form 6251 line-33 defect class exactly.
//! 3. **Complete** — the set of line numbers transcribed is exactly the set the **extract's own
//!    worksheet block** enumerates. Catches an OMISSION, which (1) and (2) cannot: thirteen faithful
//!    whole quotes are still wrong if the form has fourteen lines. ★ The expected set is read off the
//!    form, never written by hand and never a `1..=N` range — the two ways this repo has already got
//!    the same check wrong once each.
//!
//! **Why it lives in xtask.** It reads `design/forms/extract/`, outside every published crate; an
//! `include_str!` reaching there from `btctax-core` ships a tarball that builds in the workspace and
//! is broken for everyone else, with exit 0.

use btctax_core::tax::capital_loss_carryover::{
    APPLICABILITY, GOTO_LINE5_OR_LINE9, GOTO_LINE9_OR_SKIP, LINES, SOURCE_EXTRACT,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// Collapse every run of whitespace to a single space.
///
/// `pdftotext -layout` wraps a clause mid-sentence — worksheet line 1's text is broken across three
/// physical lines — so a literal `contains` would reject a faithful quote and thereby reward a
/// truncated one. Identical to the normalisation `line_coverage_check` uses, deliberately: two
/// authorities on "is this quote verbatim?" that normalise differently means the weaker one blesses a
/// degraded citation.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The extract, RAW — physical lines preserved. The line-number scan below needs them; the quote scan
/// normalises its own copy.
fn extract_raw() -> String {
    let p = repo_root().join(SOURCE_EXTRACT);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The extract text, whitespace-normalised.
fn extract() -> String {
    normalize(&extract_raw())
}

/// The worksheet's own block of the extract: from its title to the end of line 13's instruction.
///
/// Bounded on BOTH sides on purpose. Unbounded, the "line number" scan below would sweep up every
/// numbered paragraph in a 100-page instruction booklet and the completeness half would report a
/// hundred missing lines — a check that always fails is as useless as one that never does.
fn worksheet_block(raw: &str) -> String {
    let title = "Capital Loss Carryover Worksheet—Lines 6 and 14";
    let start = raw
        .find(title)
        .unwrap_or_else(|| panic!("{SOURCE_EXTRACT} does not contain the worksheet title"));
    let rest = &raw[start..];
    // The last numbered instruction on the sheet; the block ends where its sentence does.
    let tail = "also enter this amount on Schedule D, line 14";
    let end = rest.find(tail).map(|i| i + tail.len()).unwrap_or_else(|| {
        panic!("{SOURCE_EXTRACT}: the worksheet block has no line-13 terminator")
    });
    rest[..end].to_string()
}

/// Every line number the extract's worksheet block ENUMERATES: a **physical line** that begins `N.`
/// followed by prose.
///
/// ★★★ **Anchored at the start of a physical line, and that is not a stylistic choice — the first
/// draft was not, and it was WRONG.** A whitespace-normalised token scan counted the cross-reference
/// inside line 1's own sentence — *"…1040-NR, **line 15.** If the amount would have been a loss…"* —
/// and reported the form as having a line 15 the transcription had "missed". The form's numbered items
/// always begin a line in `pdftotext -layout` output; a mid-sentence cross-reference never does. The
/// bare `N.` tokens the sheet's answer boxes emit are excluded by the same rule, since nothing follows
/// them on their line.
///
/// ★ This is the half that cannot be satisfied by a hand-written list: the answer comes from the form.
fn line_numbers_in_the_form(block: &str) -> BTreeSet<u8> {
    let mut out = BTreeSet::new();
    for raw_line in block.lines() {
        let line = raw_line.trim_start();
        let Some(dot) = line.find('.') else { continue };
        let Ok(n) = line[..dot].parse::<u8>() else {
            continue;
        };
        if n == 0 {
            continue;
        }
        // `N.` alone is an answer-box label; `N. <word>` is an instruction.
        if line[dot + 1..]
            .trim_start()
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic())
        {
            out.insert(n);
        }
    }
    out
}

/// How a quote's END is proved to be the instruction's end, not a place the citation stopped.
///
/// ★★★ **A bare `contains` is NOT a verbatim check, and mutation proved it here.** Planting the
/// truncation *"Combine lines 1 and 2"* for line 3's *"Combine lines 1 and 2. If zero or less, enter
/// -0-"* left this file **GREEN** — the shortened text is a substring of the real one. That dropped
/// clause is the entire N1 fix (it is the floor that makes line 4 partial), so the checker was blessing
/// a transcription of the old flat rule wearing the worksheet's field names. It is the Form 6251
/// line-33 class exactly: *a citation shortened until the checker was satisfied.*
///
/// So a match must also be shown to END where the form ends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Terminator {
    /// A numbered worksheet instruction. `pdftotext` renders the dot leaders that run to the answer
    /// box as spaced periods, so every one of the thirteen is followed by `" ."`. Mid-sentence AND
    /// sentence-boundary truncations both fail this: line 1 cut after its first sentence is followed
    /// by `" If"`, not `" ."`.
    DotLeader,
    /// A prose sentence in the worksheet's header or an interstitial instruction. These end at a full
    /// stop and are followed by the next sentence or the next line number, so the next token must
    /// begin with a capital or a digit — a truncation lands mid-clause, on a lowercase word.
    ///
    /// ★ **Weaker than [`Self::DotLeader`], and the limit is stated rather than hidden**: it would
    /// accept a truncation that happened to fall on an internal sentence boundary. None of the three
    /// quotes it guards has an internal full stop, so no such cut exists to make — but a future quote
    /// that spans two sentences must not be given this terminator.
    SentenceEnd,
}

/// Every quote the transcription claims is verbatim: label, text, and how its end is proved.
fn quotes() -> Vec<(String, &'static str, Terminator)> {
    let mut v: Vec<(String, &'static str, Terminator)> = LINES
        .iter()
        .map(|(n, q)| (format!("line {n}"), *q, Terminator::DotLeader))
        .collect();
    v.push((
        "the applicability sentence".to_string(),
        APPLICABILITY,
        Terminator::SentenceEnd,
    ));
    v.push((
        "the lines 6-8 interstitial".to_string(),
        GOTO_LINE5_OR_LINE9,
        Terminator::SentenceEnd,
    ));
    v.push((
        "the lines 9-13 interstitial".to_string(),
        GOTO_LINE9_OR_SKIP,
        Terminator::SentenceEnd,
    ));
    v
}

/// Whether `needle` occurs in `haystack` AND ends where `term` says the form ends it.
///
/// Every occurrence is tried, not just the first: a short phrase can appear both mid-sentence and as a
/// whole instruction, and finding the wrong one first must not reject a faithful quote.
fn occurs_and_terminates(haystack: &str, needle: &str, term: Terminator) -> bool {
    let mut from = 0usize;
    while let Some(i) = haystack[from..].find(needle) {
        let end = from + i + needle.len();
        let rest = &haystack[end..];
        let ok = match term {
            Terminator::DotLeader => rest.starts_with(" ."),
            Terminator::SentenceEnd => {
                rest.is_empty()
                    || rest.strip_prefix(' ').is_some_and(|r| {
                        r.chars()
                            .next()
                            .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit() || c == '.')
                    })
            }
        };
        if ok {
            return true;
        }
        from = from + i + 1;
    }
    false
}

/// Quotes that are not verbatim-and-complete in the extract, as human-readable errors.
fn paraphrases(haystack: &str) -> Vec<String> {
    quotes()
        .into_iter()
        .filter(|(_, q, t)| !occurs_and_terminates(haystack, &normalize(q), *t))
        .map(|(label, q, _)| {
            format!(
                "{SOURCE_EXTRACT}: {label} is NOT the form's own sentence, whole — the transcription \
                 says {:?}. Either the words differ, or the quote stops before the instruction does.",
                normalize(q)
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Half 1 — every transcribed instruction is the form's own sentence.
    #[test]
    fn every_worksheet_quote_is_verbatim() {
        let errs = paraphrases(&extract());
        assert!(errs.is_empty(), "{}", errs.join("\n"));
    }

    /// Half 2 — the transcription covers exactly the line set the FORM enumerates.
    #[test]
    fn the_transcribed_line_set_is_the_forms_own_line_set() {
        let block = worksheet_block(&extract_raw());
        let from_form = line_numbers_in_the_form(&block);
        let transcribed: BTreeSet<u8> = LINES.iter().map(|(n, _)| *n).collect();
        assert!(
            !from_form.is_empty(),
            "the extract's worksheet block enumerated NO lines — the parser is broken, and a \
             checker that reads nothing reports OK forever"
        );
        assert_eq!(
            transcribed, from_form,
            "the worksheet's line set is decided by the FORM, not by us: missing {:?}, invented {:?}",
            from_form.difference(&transcribed).collect::<Vec<_>>(),
            transcribed.difference(&from_form).collect::<Vec<_>>(),
        );
    }

    /// ★★★ **THE KILL TEST (harness B1: no checker exists until it has been observed RED on a planted
    /// defect).** Three defects, because each of the three halves is blind to the others'.
    ///
    /// (a) A **paraphrase** — words that are simply not the form's.
    ///
    /// (b) A **TRUNCATION** — the exact class that produced the Form 6251 line-33 defect: a citation
    ///     shortened until the checker was satisfied. ★★ This one was found by *mutating the live
    ///     transcription*, and the first version of this file **passed it**: dropping line 3's *"If
    ///     zero or less, enter -0-"* leaves a string that is still a substring of the real sentence.
    ///     That clause IS the N1 fix — it is the floor that makes line 4 partial — so a green checker
    ///     was blessing the old flat rule wearing the worksheet's field names. Hence
    ///     [`Terminator`]; hence this assertion, which reds if that end-of-instruction proof is
    ///     removed.
    ///
    /// (c) An **omission** — line 13 (the long-term carryover, the very figure N1 is about) simply not
    ///     transcribed. (a) and (b) both pass: every *remaining* quote is still whole and verbatim.
    ///     Only the form-derived line set catches it, which is why the expected set is read off the
    ///     extract instead of written down.
    #[test]
    fn a_paraphrase_a_truncation_and_an_omission_are_all_caught() {
        let hay = extract();

        // The faithful quote is ACCEPTED — a checker that rejects everything proves nothing.
        assert!(
            occurs_and_terminates(
                &hay,
                &normalize("Combine lines 1 and 2. If zero or less, enter -0-"),
                Terminator::DotLeader
            ),
            "the real line-3 instruction must pass"
        );

        // (a) planted paraphrase — words the form does not use.
        assert!(
            !occurs_and_terminates(
                &hay,
                &normalize("Add lines 1 and 2. If negative, enter zero"),
                Terminator::DotLeader
            ),
            "a reworded instruction must be REJECTED"
        );

        // (b) planted TRUNCATION — a strict prefix of the real sentence, which a bare `contains`
        //     accepts and the terminator rejects.
        let truncated = normalize("Combine lines 1 and 2");
        assert!(
            hay.contains(&truncated),
            "the plant must be vacuously true under a bare `contains`, or it tests nothing"
        );
        assert!(
            !occurs_and_terminates(&hay, &truncated, Terminator::DotLeader),
            "★ a citation that stops before the instruction does must be REJECTED — this is the \
             assertion that reds if the end-of-instruction proof is deleted"
        );
        // …and the same holds at a SENTENCE boundary, which is the subtler cut: line 1 has two
        // sentences, and dropping the second leaves a period behind to look like an ending.
        let sentence_cut =
            normalize("Enter the amount from your 2024 Form 1040, 1040-SR, or 1040-NR, line 15.");
        assert!(hay.contains(&sentence_cut), "the plant must be findable");
        assert!(
            !occurs_and_terminates(&hay, &sentence_cut, Terminator::DotLeader),
            "line 1 cut at its internal full stop must be REJECTED too"
        );

        // (c) planted omission — drop line 13 from the transcribed set and watch the form disagree.
        let block = worksheet_block(&extract_raw());
        let from_form = line_numbers_in_the_form(&block);
        let mut short: BTreeSet<u8> = LINES.iter().map(|(n, _)| *n).collect();
        assert!(short.remove(&13), "line 13 must be in the real set");
        assert_ne!(
            short, from_form,
            "dropping the long-term carryover line must be REJECTED by the form-derived set"
        );
        assert!(
            from_form.contains(&13),
            "…and it is rejected because the FORM says 13 exists, not because we said so"
        );
        let _ = hay;
    }

    /// ★★★ **The checker's OWN planted defect, kept as a test because it was a REAL false positive.**
    ///
    /// The first draft scanned whitespace-normalised tokens and counted *"…1040-NR, **line 15.** If
    /// the amount…"* — a cross-reference inside line 1's own sentence — as a fourteenth worksheet
    /// line, reporting the faithful transcription as incomplete. A checker that cries wolf gets
    /// widened until it is silent, which is how a real omission gets through later. Anchoring on the
    /// start of a physical line is the fix, and this pins it.
    #[test]
    fn a_mid_sentence_cross_reference_is_not_a_worksheet_line() {
        let planted = "\
Capital Loss Carryover Worksheet—Lines 6 and 14
1. Enter the amount from your 2024 Form 1040, 1040-SR, or 1040-NR, line 15. If the amount would
have been a loss if you could enter a negative number on that line, enclose the amount in
parentheses . . .
1.
13. Long-term capital loss carryover for 2025. Subtract line 12 from line 9. If zero or less, enter -0-. If
more than zero, also enter this amount on Schedule D, line 14
";
        let got = line_numbers_in_the_form(&worksheet_block(planted));
        assert_eq!(
            got,
            BTreeSet::from([1u8, 13]),
            "only the two lines that BEGIN a line count; the `line 15.` cross-reference and the \
             bare `1.` answer box must not"
        );
    }

    /// ★ The block bound is load-bearing: without it the scan would sweep the whole booklet. Pin that
    /// the block is small and that it stops before the next section's prose.
    #[test]
    fn the_worksheet_block_is_bounded_to_the_worksheet() {
        let block = worksheet_block(&extract_raw());
        assert!(
            block.len() < 3_000,
            "the worksheet block ballooned to {} chars — the terminator stopped matching",
            block.len()
        );
        assert!(
            !block.contains("Disposal of QOF"),
            "the block must end at line 13, not run into the next section"
        );
    }
}
