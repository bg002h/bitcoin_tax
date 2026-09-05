//! **Half 1a of the Schedule 1-A conformance KAT — MEMBERSHIP against the form's own labels.**
//!
//! ★★★ **WHY THIS HALF LIVES IN `xtask` AND THE OTHER THREE DO NOT.** Its instrument is
//! [`crate::label_reader`]'s two witnesses, which read `design/forms/geometry/f1040s1a--2025.json` —
//! a repo-root fixture `btctax-core` deliberately cannot reach, because an `include_str!` escaping the
//! crate root has shipped a broken tarball from this repo before, with exit 0. `xtask` depends on
//! `btctax-core` and `btctax-core` names no `xtask`, so the direction is forced; the reverse would be
//! a cycle.
//!
//! ★★ **THE EXPECTED SET IS NEVER A RANGE AND NEVER A HAND-LIST.** `1..=38` is the trap this form was
//! made for: it either reds on every lettered field as an unexpected extra, or the struct gets
//! collapsed to match and a sub-line is lost — which is verbatim the shipped defect `CLAUDE.md`
//! records as *"later drafts dropped Form 6251 line 2b."* The text layer alone cannot answer it
//! either: it yields **50** labels and cannot say which of them take an entry, because distinguishing
//! a heading from a label means knowing whether the row has an amount box. So the expected set is the
//! adjudication of two witnesses — the printed margin column, and the AcroForm geometry — which
//! together resolve 50 printed labels into **48 entry lines** and the **2** headings `4` and `22`.
//!
//! ★ The ACTUAL set comes from [`Schedule1A::leaves`], whose exhaustive destructure ties it to the
//! struct by the compiler: deleting a field is `E0026` there, and deleting the tuple entry as well
//! shrinks the set this module compares. That is the pair of edits the planted defect below performs.
//!
//! `#[cfg(test)]` because it has no operator-facing mode — the answer is yes or no, and it belongs in
//! the suite, where `make check` asks it on every commit rather than when someone remembers to.

use btctax_core::tax::schedule_1a::{line_label_of, Schedule1A, BOX_LESS_HEADINGS};
use std::collections::BTreeSet;

/// What the two witnesses say the form prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedLabels {
    /// Every printed label, entry line or heading.
    pub all: Vec<String>,
    /// The labels whose row carries an amount box.
    pub entry: BTreeSet<String>,
    /// The labels whose row carries none — headings for their lettered sub-rows.
    pub headings: Vec<String>,
}

/// Run both witnesses over the committed geometry and adjudicate.
fn printed_labels(stem: &str) -> Result<PrintedLabels, String> {
    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    let labels = crate::label_reader::witness_text(&g)?;
    let boxes = crate::label_reader::witness_boxes(&g);
    let counts = crate::label_reader::assign_boxes(&labels, &boxes);
    if labels.is_empty() {
        return Err(
            "no printed labels — a census with nothing to check reports conformance".into(),
        );
    }
    Ok(PrintedLabels {
        all: labels.iter().map(|(l, _, _)| l.clone()).collect(),
        entry: labels
            .iter()
            .zip(&counts)
            .filter(|(_, c)| **c > 0)
            .map(|((l, _, _), _)| l.clone())
            .collect(),
        headings: labels
            .iter()
            .zip(&counts)
            .filter(|(_, c)| **c == 0)
            .map(|((l, _, _), _)| l.clone())
            .collect(),
    })
}

/// Is every printed label ACCOUNTED FOR — as a field, or as a heading with a recorded reason — and is
/// every field a printed label?
///
/// ★★ **Closed at both ends, and the second end is the one that matters.** A missing line is the
/// obvious defect; an *extra* field is how a struct drifts away from the form it claims to be. And a
/// heading is not an exemption: it must appear in the recorded list with a reason, because *"this line
/// encodes no decision"* and *"we forgot this line"* are the two blanks a conformance check exists to
/// tell apart.
///
/// Pure over its inputs, so the planted defect below can hand it a mutated struct set — a check that
/// can only read the real thing cannot be watched going red.
pub fn membership_violations(
    fields: &BTreeSet<String>,
    printed: &PrintedLabels,
    recorded_headings: &[(&str, &str)],
) -> Vec<String> {
    let mut errs = Vec::new();
    for missing in printed.entry.difference(fields) {
        errs.push(format!(
            "the form prints entry line {missing:?} and the struct has no field for it — this is \
             \"we forgot this line\", and it is invisible in the emitted PDF"
        ));
    }
    for extra in fields.difference(&printed.entry) {
        errs.push(format!(
            "the struct carries {extra:?}, which is not an entry line on the form"
        ));
    }
    let recorded: BTreeSet<&str> = recorded_headings.iter().map(|(l, _)| *l).collect();
    for h in &printed.headings {
        if !recorded.contains(h.as_str()) {
            errs.push(format!(
                "line {h:?} prints a label and takes no entry, and nothing records WHY — a \
                 classification nobody justified is \"we forgot this line\" in disguise"
            ));
        }
    }
    for (label, reason) in recorded_headings {
        if !printed.headings.iter().any(|h| h == label) {
            errs.push(format!(
                "{label:?} is recorded as a box-less heading but the form gives it an entry box"
            ));
        }
        if reason.trim().is_empty() {
            errs.push(format!("heading {label:?} is recorded with no reason"));
        }
    }
    errs
}

/// The line labels the struct actually declares, derived from its own leaves.
fn struct_line_labels() -> BTreeSet<String> {
    Schedule1A::default()
        .leaves()
        .iter()
        .map(|(l, _)| line_label_of(l).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **HALF 1a — the 48 entry lines, adjudicated by two witnesses and compared to the struct.**
    #[test]
    fn every_printed_label_is_a_field_or_a_recorded_heading() {
        let printed = printed_labels("f1040s1a--2025").expect("Schedule 1-A geometry");
        // The adjudication itself, restated here so this test fails loudly if the witnesses drift —
        // 50 printed labels, 48 of which take an entry, 2 of which are headings.
        assert_eq!(printed.all.len(), 50, "{:?}", printed.all);
        assert_eq!(printed.entry.len(), 48, "{:?}", printed.entry);
        assert_eq!(printed.headings, ["4", "22"]);

        let fields = struct_line_labels();
        assert_eq!(fields.len(), 48, "{fields:?}");
        let errs = membership_violations(&fields, &printed, &BOX_LESS_HEADINGS);
        assert!(errs.is_empty(), "{errs:#?}");

        // 52 LEAVES over those 48 labels — line 22's two rows carry three columns each.
        assert_eq!(Schedule1A::default().leaves().len(), 52);
    }

    /// ★★★ **B1 for half 1a — the planted defects, on the real adjudicated label set.**
    ///
    /// Each plant is the second half of a two-edit mutation: deleting a struct field is already a
    /// compile error (`E0026` in `Schedule1A::leaves`), and the author's next move is to delete the
    /// tuple entry too. This is what happens then.
    #[test]
    fn a_dropped_line_an_invented_line_and_an_unrecorded_heading_are_all_rejected() {
        let printed = printed_labels("f1040s1a--2025").expect("Schedule 1-A geometry");
        let fields = struct_line_labels();
        assert!(
            membership_violations(&fields, &printed, &BOX_LESS_HEADINGS).is_empty(),
            "the control must PASS, or every plant below passes for the wrong reason"
        );

        // (1) A DROPPED LINE — `2b`, the exact shape `CLAUDE.md` records as shipped on Form 6251.
        //     Dropping it under-adds MAGI, so every phase-out is too small: it UNDERSTATES TAX.
        let mut dropped = fields.clone();
        assert!(dropped.remove("2b"));
        assert!(
            membership_violations(&dropped, &printed, &BOX_LESS_HEADINGS)
                .iter()
                .any(|e| e.contains("\"2b\"") && e.contains("no field for it")),
            "a dropped line must red"
        );

        // (2) AN INVENTED LINE — the other end of the comparison, and the one a "missing lines only"
        //     check would wave through while the struct drifts away from the form.
        let mut invented = fields.clone();
        invented.insert("39".to_string());
        assert!(
            membership_violations(&invented, &printed, &BOX_LESS_HEADINGS)
                .iter()
                .any(|e| e.contains("\"39\"")),
            "a field that is not a line on the form must red"
        );

        // (3) A HEADING WITH NO RECORDED REASON — the "this line encodes no decision" half. Without
        //     it, a census could quietly forget line 22 and still report conformance.
        assert!(
            membership_violations(&fields, &printed, &BOX_LESS_HEADINGS[..1])
                .iter()
                .any(|e| e.contains("\"22\"") && e.contains("nothing records WHY")),
            "an unrecorded box-less heading must red"
        );

        // (4) …and a heading recorded for a line that DOES take an entry — the mirror, which is how an
        //     exemption list grows by accretion.
        let bogus = [("13", "not actually a heading")];
        assert!(
            membership_violations(&fields, &printed, &bogus)
                .iter()
                .any(|e| e.contains("\"13\"") && e.contains("entry box")),
            "recording an entry line as a heading must red"
        );

        // (5) ★★ AND THE READER: a geometry with no labels must ERROR, never return an empty set that
        //     would make every comparison above vacuously true.
        assert!(printed_labels("no-such-form--2025").is_err());
    }
}
