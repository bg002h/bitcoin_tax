//! ★★★ **N1 — the CONFORMANCE CHECKER for the §1211/§1212 Capital Loss Carryover Worksheet.**
//!
//! `btctax_core::tax::capital_loss_carryover` transcribes the *Capital Loss Carryover Worksheet —
//! Lines 6 and 14* from the 2025 Schedule D instructions. `CLAUDE.md`: *"Is every form line present?
//! Does each doc comment match the instruction text? are ASSERTIONS, not opinions."* So this file
//! asserts them, on every commit, rather than asking a reviewer to read a form.
//!
//! **Four checks, because each is satisfiable by a defect the others miss.**
//!
//! 1. **Verbatim** — every quote in `LINES` (plus the header prose and the two interstitial
//!    "go to line N" instructions) appears in the extract, modulo whitespace. Catches a paraphrase.
//! 2. **Whole** — and each match must END where the form ends the instruction ([`Terminator`]).
//!    ★★ Added because mutation caught check (1) **passing a truncation**: a shortened citation is a
//!    substring of the real one, which is the Form 6251 line-33 defect class exactly.
//! 3. **Complete (numbered)** — the set of line numbers transcribed is exactly the set the **extract's
//!    own worksheet block** enumerates. Catches an OMISSION, which (1) and (2) cannot: thirteen
//!    faithful whole quotes are still wrong if the form has fourteen lines. ★ The expected set is read
//!    off the form, never written by hand and never a `1..=N` range — the two ways this repo has
//!    already got the same check wrong once each.
//! 4. **Complete (UNNUMBERED)** — ★★★ and check (3) is *structurally blind* to everything that is not
//!    numbered. [`line_numbers_in_the_form`] requires `N.` to parse as a `u8`, so a physical line
//!    beginning *"If you and your spouse…"* is skipped before it can be counted. The worksheet header
//!    carries two GOVERNING CONDITIONS in exactly that shape — the MFS-after-joint-return sourcing
//!    rule and the §108(b)(2)(G) canceled-debt condition — and both were dropped from the
//!    transcription while checks (1)–(3) all reported success, because none of them could see a
//!    sentence that has no number. So the header is enumerated as **paragraphs read off the form**,
//!    and every one must be *accounted for*: matched by a transcribed constant, or listed in
//!    [`CARRIES_NO_DECISION`] with a reason. `CLAUDE.md`: *"A checker that cannot distinguish 'this
//!    line encodes no decision' from 'we forgot this line' is not a conformance check."*
//!
//! **Why it lives in xtask.** It reads `design/forms/extract/`, outside every published crate; an
//! `include_str!` reaching there from `btctax-core` ships a tarball that builds in the workspace and
//! is broken for everyone else, with exit 0.

use btctax_core::tax::capital_loss_carryover::{
    APPLICABILITY, CANCELED_DEBT_EXCLUSION, GOTO_LINE5_OR_LINE9, GOTO_LINE9_OR_SKIP,
    JOINT_RETURN_SOURCING, LINES, OTHERWISE_NO_CARRYOVERS, SOURCE_EXTRACT,
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
///
/// ★★★ **AND IT IS STRUCTURALLY BLIND TO EVERY UNNUMBERED CONDITION.** `N.` must parse as a `u8`, so a
/// physical line beginning *"If you and your spouse…"* is discarded before it can be counted. That is
/// not a bug in this function — a set of line *numbers* is what it is for — but it does mean this
/// function's silence is NOT evidence that the header is transcribed. It was silent while both of the
/// header's governing conditions were missing. [`unnumbered_conditions_in_the_form`] is the other half.
fn line_numbers_in_the_form(block: &str) -> BTreeSet<u8> {
    block
        .lines()
        .filter_map(|raw_line| numbered_instruction(raw_line.trim_start()))
        .collect()
}

/// ★★★ Every PARAGRAPH of the worksheet's HEADER — the prose above numbered line 1 — normalised.
///
/// This is check (4), and it exists because check (3) cannot exist here: [`line_numbers_in_the_form`]
/// parses a leading `N.` as a `u8`, so *every* unnumbered sentence is skipped before it can be
/// counted. The two governing conditions in the header were dropped from the transcription and no
/// half of this file went red.
///
/// **Paragraphs, not sentences, and the boundary is read off the form's own layout** — `pdftotext
/// -layout` wraps a paragraph across physical lines and terminates it with a full stop, so a run of
/// physical lines ends at a blank line or at a line ending in `.`. Splitting on sentences instead
/// would have to know that *"see Pub. 4681."* is one sentence and not two, which is a rule about
/// English, not about this form.
///
/// ★ Bounded above by worksheet line 1, detected the same way [`line_numbers_in_the_form`] detects a
/// numbered instruction — so the two halves cannot disagree about where the header stops.
fn unnumbered_conditions_in_the_form(block: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    for raw_line in block.lines() {
        let line = raw_line.trim();
        // The header ends where the first NUMBERED instruction begins.
        if is_numbered_instruction(line) {
            break;
        }
        if line.is_empty() {
            if !cur.is_empty() {
                out.push(normalize(&cur.join(" ")));
                cur.clear();
            }
            continue;
        }
        cur.push(line);
        if line.ends_with('.') {
            out.push(normalize(&cur.join(" ")));
            cur.clear();
        }
    }
    if !cur.is_empty() {
        out.push(normalize(&cur.join(" ")));
    }
    out
}

/// Whether a trimmed physical line is a numbered worksheet instruction (`N. <alphabetic>`).
///
/// Factored out of [`line_numbers_in_the_form`] so the header bound above and the numbered-line scan
/// share ONE definition of "this line is a numbered instruction". Two copies would drift into a header
/// that swallows line 1, or a line 1 that never terminates the header.
fn is_numbered_instruction(line: &str) -> bool {
    numbered_instruction(line).is_some()
}

/// The worksheet line number a trimmed physical line introduces, if it introduces one.
fn numbered_instruction(line: &str) -> Option<u8> {
    let dot = line.find('.')?;
    let n = line[..dot].parse::<u8>().ok()?;
    if n == 0 {
        return None;
    }
    // `N.` alone is an answer-box label; `N. <word>` is an instruction.
    line[dot + 1..]
        .trim_start()
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic())
        .then_some(n)
}

/// Header paragraphs that encode NO instruction, each with the reason it encodes none.
///
/// ★★★ This is the half that separates *"this line carries no decision"* from *"we forgot this
/// line"* — without it, check (4) would be a wall a future maintainer widens until it is silent. It is
/// an explicit, reasoned excuse list and it is **short by construction**: everything the form states
/// as a condition belongs in the transcription instead.
const CARRIES_NO_DECISION: &[(&str, &str)] = &[
    (
        "Capital Loss Carryover Worksheet—Lines 6 and 14",
        "the worksheet's TITLE — it names the sheet and the two Schedule D lines its results land on; \
         both are already carried by LINES 8 and 13's own instruction text",
    ),
    (
        "Keep for Your Records",
        "the IRS retention notice: this sheet is not filed and reaches no printed line, so it \
         conditions nothing btctax computes (see the module doc's note that no testimony is at stake)",
    ),
];

/// Every header sentence btctax claims to have TRANSCRIBED, in the order the form prints them.
fn transcribed_header_prose() -> Vec<&'static str> {
    vec![
        APPLICABILITY,
        OTHERWISE_NO_CARRYOVERS,
        JOINT_RETURN_SOURCING,
        CANCELED_DEBT_EXCLUSION,
    ]
}

/// Header paragraphs not fully accounted for by `accounted`, as human-readable errors.
///
/// A paragraph is accounted for when it can be consumed, front to back, by accounted strings: the
/// applicability paragraph is the applicability sentence FOLLOWED BY "Otherwise, you don't have any
/// carryovers.", and demanding a whole-paragraph match would force those two decisions to be
/// transcribed as one string. Anything left over is reported verbatim, because the residue IS the
/// finding.
fn unaccounted_header_paragraphs(block: &str, accounted: &[&str]) -> Vec<String> {
    let norm: Vec<String> = accounted.iter().map(|s| normalize(s)).collect();
    let mut errs = Vec::new();
    for para in unnumbered_conditions_in_the_form(block) {
        let mut rest = para.as_str();
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            // Longest first: a short excuse must not shadow a longer transcription.
            let best = norm
                .iter()
                .filter(|a| rest.starts_with(a.as_str()))
                .max_by_key(|a| a.len());
            match best {
                Some(a) => rest = &rest[a.len()..],
                None => {
                    errs.push(format!(
                        "{SOURCE_EXTRACT}: the worksheet HEADER states something nothing accounts \
                         for — {rest:?}. Transcribe it as a constant (and raise the refusal it \
                         implies), or list it in CARRIES_NO_DECISION with the reason it encodes no \
                         decision. It is NOT enough that the numbered lines are complete: a leading \
                         `N.` is what check (3) reads, so an unnumbered condition is invisible to it."
                    ));
                    break;
                }
            }
        }
    }
    errs
}

/// ★★★ The identifier of the `RefuseReason` variant that refused a TI≤0 year with a capital-loss
/// carryforward brought in, DELETED when the owner authorised that household to file.
///
/// ★ **Assembled from two halves at compile time, so the literal never appears in this file
/// either.** The obvious spelling made the checker find ITSELF — and the fix must not be a
/// path exclusion for this file, because an exclusion keyed to a path stops working the moment the
/// file is moved and then silently passes forever. `concat!` leaves the whole identifier nowhere in
/// the tree while still comparing against it exactly.
///
/// See [`tests::the_lifted_refusal_leaves_no_trace_in_the_tree`].
const LIFTED_REFUSAL_IDENT: &str = concat!("TaxableIncomeNonPositive", "WithCarryforward");

/// Every path under `dir` (recursively) whose text contains `needle`, relative to `dir`.
///
/// ★ Skips `target/` — a build directory holds compiler artifacts that quote source, and a checker
/// that reports its own `.d` files is a checker nobody keeps.
fn files_containing(dir: &Path, needle: &str) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, needle: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(&p, base, needle, out);
            } else if std::fs::read_to_string(&p).is_ok_and(|t| t.contains(needle)) {
                out.push(p.strip_prefix(base).unwrap_or(&p).display().to_string());
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, needle, &mut out);
    out.sort();
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
    /// accept a truncation that happened to fall on an internal sentence boundary. A future quote that
    /// spans two sentences must not be given this terminator.
    ///
    /// ★★ **The limit is now REACHED, and saying so is the point of stating it.** Six quotes carry
    /// this terminator; five have no internal full stop, but [`CANCELED_DEBT_EXCLUSION`] ends *"see
    /// Pub. 4681."* and the abbreviation's period is an internal full stop followed by a digit. A
    /// citation truncated to *"…see Pub."* would therefore be ACCEPTED here. It is a one-sentence
    /// quote whose remaining half is a publication number, so the truncation moves no figure and
    /// changes no refusal — but the blind spot is real, is recorded rather than papered over, and is
    /// the reason this variant's doc exists at all.
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
    // ★ The header prose. `APPLICABILITY` above is only the first HALF of the header's first
    //   paragraph; the three below complete it, and the last two are the governing conditions
    //   check (3) can never see.
    v.push((
        "the applicability closing sentence".to_string(),
        OTHERWISE_NO_CARRYOVERS,
        Terminator::SentenceEnd,
    ));
    v.push((
        "the MFS-after-joint-return sourcing condition".to_string(),
        JOINT_RETURN_SOURCING,
        Terminator::SentenceEnd,
    ));
    v.push((
        "the §108(b)(2)(G) canceled-debt condition".to_string(),
        CANCELED_DEBT_EXCLUSION,
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

    /// ★★★ **K19 — the lifted refusal left NO trace, and the grep is the assertion.**
    ///
    /// (A) could have been implemented by keeping `RefuseReason::TaxableIncomeNonPositive-
    /// WithCarryforward` and deleting only its emit site. Everything would compile; `attribute.rs`
    /// would go on mapping an unreachable reason to an anchor; the SPEC lists would stay stale; and
    /// no test in the workspace would red. Deleting the VARIANT is what closed that path — it
    /// E0599'd across three crates and enumerated every consumer for free — and this is what proves
    /// the deletion was total rather than partial.
    ///
    /// ★ The positive control is not decoration. A scanner that silently reads nothing (a wrong
    /// root, an unreadable tree) reports "zero occurrences" forever, which is the exact shape of
    /// every green-and-blind instrument this repo has shipped. So a string that MUST be present is
    /// looked for first.
    #[test]
    fn the_lifted_refusal_leaves_no_trace_in_the_tree() {
        let crates = repo_root().join("crates");

        // Positive control — the scanner can find something that is really there.
        let control = files_containing(&crates, "CapitalLossCarryoverWorksheet");
        assert!(
            control.len() >= 2,
            "the scanner found {} file(s) containing a symbol that is definitely present — it is \
             not reading the tree: {control:?}",
            control.len()
        );

        let hits = files_containing(&crates, LIFTED_REFUSAL_IDENT);
        assert!(
            hits.is_empty(),
            "★ the lifted TI≤0-with-carryforward refusal still appears in {} file(s) under \
             crates/: {hits:?}. Deleting only the emit site and keeping the variant compiles \
             cleanly and reds nothing — which is why the variant itself had to go. A doc comment, \
             a SPEC list or an attribution-table entry naming it is the same stale claim wearing a \
             different hat.",
            hits.len()
        );
    }

    /// Half 4 — every paragraph of the worksheet HEADER is accounted for.
    #[test]
    fn the_worksheets_unnumbered_conditions_are_transcribed_and_checked() {
        let block = worksheet_block(&extract_raw());
        let paras = unnumbered_conditions_in_the_form(&block);
        assert!(
            paras.len() >= 3,
            "the header enumerated {} paragraph(s) — a scan that reads nothing reports OK forever: \
             {paras:?}",
            paras.len()
        );
        assert!(
            paras
                .iter()
                .any(|p| p.contains("the spouse who actually had the loss")),
            "the MFS-after-joint-return condition must be ENUMERATED off the form: {paras:?}"
        );
        assert!(
            paras.iter().any(|p| p.contains("Pub. 4681")),
            "the §108(b)(2)(G) canceled-debt condition must be ENUMERATED off the form: {paras:?}"
        );
        assert!(
            !paras
                .iter()
                .any(|p| p.starts_with("1. Enter the amount from your 2024 Form 1040")),
            "the header must STOP at numbered line 1: {paras:?}"
        );

        let mut accounted = transcribed_header_prose();
        accounted.extend(CARRIES_NO_DECISION.iter().map(|(s, _)| *s));
        let errs = unaccounted_header_paragraphs(&block, &accounted);
        assert!(errs.is_empty(), "{}", errs.join("\n"));

        for (_, reason) in CARRIES_NO_DECISION {
            assert!(
                reason.len() > 30,
                "an excuse without a REASON is the hand-list failure wearing a table: {reason:?}"
            );
        }
    }

    /// ★★★ **THE KILL TEST for check (4) — the half whose blindness is the reason it exists.**
    ///
    /// (a) **Delete a transcribed header condition.** The new enumeration must go RED — *and*
    ///     [`the_transcribed_line_set_is_the_forms_own_line_set`]'s instrument must stay GREEN on the
    ///     same defect. Asserting that the OLD half does not fire is what proves the new one is
    ///     load-bearing rather than a second opinion: this is exactly how both conditions were dropped
    ///     with the file green.
    ///
    /// (b) **Paraphrase it.** "the spouse who actually had the loss" → "either spouse" is a change of
    ///     tax law, not of wording — it is the sourcing rule inverted. The verbatim half must reject
    ///     it, and it is a DIFFERENT half: a paraphrase is still one accounted paragraph as far as (a)
    ///     is concerned, provided the constant is edited to match.
    ///
    /// (c) **Excuse it instead of transcribing it.** A future maintainer silencing (a) by dropping the
    ///     sentence into `CARRIES_NO_DECISION` is the failure mode of every excuse list in this repo,
    ///     so the reason string is asserted to exist — and the verbatim half still holds the words.
    #[test]
    fn a_dropped_header_condition_is_caught_by_the_unnumbered_half_alone() {
        let block = worksheet_block(&extract_raw());

        // (a) plant: the joint-return condition is simply not transcribed.
        let mut without_joint = transcribed_header_prose();
        without_joint.retain(|s| *s != JOINT_RETURN_SOURCING);
        let mut accounted = without_joint.clone();
        accounted.extend(CARRIES_NO_DECISION.iter().map(|(s, _)| *s));
        let errs = unaccounted_header_paragraphs(&block, &accounted);
        assert_eq!(
            errs.len(),
            1,
            "dropping the sourcing rule must be caught EXACTLY once: {errs:?}"
        );
        assert!(
            errs[0].contains("the spouse who actually had the loss"),
            "…and the residue reported must be the dropped sentence itself: {}",
            errs[0]
        );
        // …and the NUMBERED half is blind to it, which is the whole finding.
        let from_form = line_numbers_in_the_form(&block);
        let transcribed: BTreeSet<u8> = LINES.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            transcribed, from_form,
            "★ check (3) must still report the transcription COMPLETE with a governing condition \
             missing — if this ever fails, the claim that the two halves are disjoint has changed"
        );

        // (b) plant: a paraphrase that inverts the rule. Caught by the VERBATIM half, not by (a).
        let hay = extract();
        assert!(
            occurs_and_terminates(
                &hay,
                &normalize(JOINT_RETURN_SOURCING),
                Terminator::SentenceEnd
            ),
            "the real sourcing rule must pass — a checker that rejects everything proves nothing"
        );
        let paraphrase = "If you and your spouse once filed a joint return and are filing separate \
                          returns for 2025, any capital loss carryover from the joint return can be \
                          deducted only on the return of either spouse.";
        assert!(
            !occurs_and_terminates(&hay, &normalize(paraphrase), Terminator::SentenceEnd),
            "★ \"either spouse\" is the sourcing rule INVERTED and must be REJECTED"
        );

        // (c) the truncation cut this terminator CANNOT make on the canceled-debt quote, recorded as
        //     an executable statement of the blind spot rather than a comment claiming there is none.
        let cut = normalize("If you excluded canceled debt from income in 2025, see Pub.");
        assert!(
            occurs_and_terminates(&hay, &cut, Terminator::SentenceEnd),
            "★ documented limit of `SentenceEnd`: the abbreviation's period passes as a sentence \
             end. If this ever fails, the terminator got stronger and the doc comment must be \
             updated to say so."
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
