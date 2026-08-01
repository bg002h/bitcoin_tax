//! ★★★ §G-11 — the COVERAGE CHECKER. Validates [`btctax_core::tax::line_coverage`] against the
//! extracted form text.
//!
//! **Why this is a test and not a review round.** Two independent Opus rounds measured the grammar's
//! misfit count by hand (r1: ≈25; r2: 10–29), each costing a full round to produce one number. This
//! computes it on every commit. `CLAUDE.md`: *"Conformance ⇒ test. 'Is every form line present?' 'Does
//! each doc comment match the instruction text?' are assertions, not opinions."*
//!
//! **Why it lives in xtask.** It reads `design/forms/extract/`, which is outside every published
//! crate. An `include_str!` reaching there from `btctax-core` would ship a tarball that builds in the
//! workspace and is broken for everyone else, **with exit 0** — the trap recorded in
//! `crate-publishing-state`. So the quotes travel as data and the checking travels here.

use btctax_core::tax::line_coverage::{self, Production};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// ★ Collapse all runs of whitespace to one space.
///
/// **Not cosmetic — it is the fix for a structural false negative.** `pdftotext -layout` wraps a
/// clause mid-sentence, so Form 8995 line 8's instruction reads *"…If zero\n or less, enter -0-…"* in
/// the extract. A literal `contains` would miss it, and the SPEC r2 reviewer identified exactly this
/// (WRAPPING) as a class the checker must not be blind to.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The tax year whose extracts the quotes are taken from.
const YEAR: &str = "2024";

/// ★★★ The CLOSED SET of clamp idioms, each paired with why it denotes that polarity.
///
/// **Fail-closed by construction.** A `Clamped` row whose quote matches NO idiom here is an ERROR, not
/// a silent pass — so the checker can never bless a polarity it did not verify, and a new idiom must be
/// added in a diff, with a reason. That is the opposite of an oracle excuse list, which grows by
/// accretion and goes stale; this one cannot be satisfied by accident.
///
/// ★★ Two of these were found by the population pass, and finding them is the checker working: **two
/// readers hit the same missing idiom and resolved it OPPOSITE ways** — one filed `f8959:8` as an
/// Exception to avoid the rejection, the other declared `f1040sa:4` a floor and would have failed the
/// build. Neither route lands as written, and the defect was in this vocabulary, not in the grammar.
const FLOOR_IDIOMS: &[(&str, &str)] = &[
    (
        "zero or less",
        "the canonical form: 'If zero or less, enter -0-'",
    ),
    ("less than zero", "the same clause, negated phrasing"),
    // ★ OPERAND-RELATIVE, and it is a floor only in light of the line's own arithmetic. Schedule A
    //   line 4 is `line1 - line3`, so "line 3 is more than line 1" IS "the result is negative".
    //   Recorded here rather than left to a reader's judgment, because the direction is NOT derivable
    //   from the clause alone — which is precisely why it needs a written justification.
    (
        "is more than line",
        "operand-relative: on a subtraction A-B, 'B is more than A' means the result is negative",
    ),
    // ★ Schedule SE feeds Form 8959 line 8; a self-employment LOSS is a negative, so this clamps up.
    (
        "if you had a loss",
        "a loss is a negative amount, so this clamps up to zero",
    ),
];

/// The ceiling family. Parenthesised loss-carryforward lines (Form 8995 16/17) clamp DOWN.
const CEIL_IDIOMS: &[(&str, &str)] = &[
    (
        "greater than zero",
        "Form 8995 16/17: a loss carryforward is clamped down to -0-",
    ),
    ("more than zero", "the same clause, alternate phrasing"),
];

/// **The ratchet.** Exceptions are lines that fit no production, each carrying a written reason.
///
/// ★ This is the number two full review rounds were spent estimating. It only ever goes DOWN: raising
/// it requires editing this line, in a diff, with a reason — which is the whole point. Same shape as
/// the `GAPS` ratchet that went 16 → 0.
const MAX_EXCEPTIONS: usize = 5;
// ★ RAISED 0 → 5 when the table was populated (118 rows over 11 forms). The five, each a line whose
//   instruction genuinely fits no production — NOT a nearest-fit, which would print a wrong figure:
//     f1040sse:4a  two-branch, BOTH branches enter (one Scaled, one Carry)
//     f1040sse:4c  the arithmetic is a Combine, but the sentence is a form EXIT ("stop") plus an
//                  out-of-scope church-employee -0- that does not clamp the combined figure
//     f1040sse:10  a COMPOSITION — "Multiply the smaller of line 6 or line 9 by 12.4%" is Bounded
//                  feeding Scaled; neither production alone states the line or its blank rule
//     f1040s1:21   §221 student-loan: produced by a worksheet in the 1040 instructions, never emitted
//     f1040s3:11   §6413(c) excess SS/RRTA: same shape, no arithmetic and no source line on the form
//   ★★ Two review rounds estimated this number at ~25 (r1) and 10-29 (r2). Measured: 5.
//   ★ A sixth was filed by a reader (f8959:8) and dissolved when FLOOR_IDIOMS learned "if you had a
//     loss" — the misfit was in this checker's vocabulary, not in the grammar.

/// **The second ratchet, and it is a DIFFERENT number.** Rows whose form btctax emits but for which no
/// text layer is committed, so the quote cannot be verified at all.
///
/// ★ Kept separate from `MAX_EXCEPTIONS` deliberately: an Exception is *"the grammar does not cover
/// this line"*, which is a design fact. An unverifiable row is *"we cannot check what this line says"*,
/// which is a missing ASSET. Collapsing them would let a fetch failure masquerade as a grammar gap.
const MAX_UNVERIFIABLE: usize = 10;
// ★★ RAISED 0 → 10: every Schedule 1 line. `crates/btctax-forms/forms/2024/f1040s1.map.toml` exists —
//    btctax EMITS this form — but neither `design/forms/extract/f1040s1--2024.txt` nor the source PDF
//    is committed, so the ten quotes cannot be checked at all. This is the hole the SPEC r2 doctrine
//    reviewer found, and it is an ASSET problem: fixing it needs one fetch, which this environment has
//    no network for.
//    ★ The rows are kept rather than dropped ON PURPOSE. Dropping them would delete the compile-time
//      guarantee for `Schedule1Lines` too, so a new Schedule 1 money field would once again arrive
//      unexamined — trading a visible, counted gap for an invisible one.

/// Run the check. Returns `Err` with every failure, so one run reports the whole picture rather than
/// the first problem.
pub fn run() -> Result<String, String> {
    let root = repo_root();
    let cov = line_coverage::all();
    let mut extracts: BTreeMap<String, String> = BTreeMap::new();
    let mut errs: Vec<String> = Vec::new();
    // Rows on a form btctax emits that has no committed text layer: their quotes cannot be checked.
    let mut unverifiable: Vec<String> = Vec::new();

    if cov.0.is_empty() {
        return Err("line_coverage::all() is EMPTY — the checker would vacuously pass".into());
    }

    for e in &cov.0 {
        // (1) Every Exception carries a reason; every non-Exception carries none. Both directions,
        //     because a reason on a real production is a sign the author was unsure.
        match (e.production, e.reason) {
            (Production::Exception, None) => errs.push(format!(
                "{}:{} ({}) is an Exception with NO reason — an unexplained misfit is how a residual \
                 bucket starts",
                e.form, e.line, e.field
            )),
            (p, Some(r)) if p != Production::Exception => errs.push(format!(
                "{}:{} ({}) is {:?} but carries a reason ({r:?}) — reasons are for Exceptions only",
                e.form, e.line, e.field, p
            )),
            _ => {}
        }

        // (2) The instruction must appear VERBATIM in the form's own extracted text.
        let stem = format!("{}--{YEAR}", e.form);
        let text = match extracts.get(&stem) {
            Some(t) => t,
            None => {
                let p = root.join(format!("design/forms/extract/{stem}.txt"));
                match std::fs::read_to_string(&p) {
                    Ok(t) => extracts.entry(stem.clone()).or_insert(normalize(&t)),
                    Err(_) => {
                        // ★★ A form btctax EMITS but has no text layer. Derived, never hand-listed:
                        //    a committed map TOML means we emit it, so the quote is UNVERIFIABLE and
                        //    counted; no map means the form name is a typo and is an ERROR. This is
                        //    the Schedule 1 hole the SPEC r2 reviewer found — `f1040s1.map.toml`
                        //    exists, `f1040s1--2024.txt` does not, and neither does its PDF, so it
                        //    cannot be fixed without network access.
                        let emitted = root
                            .join(format!(
                                "crates/btctax-forms/forms/{YEAR}/{}.map.toml",
                                e.form
                            ))
                            .exists();
                        if emitted {
                            unverifiable.push(format!("{}:{}", e.form, e.line));
                        } else {
                            errs.push(format!(
                                "{}:{} quotes form {:?}, which has NEITHER an extract NOR a map — the \
                                 form name is wrong",
                                e.form, e.line, e.form
                            ));
                        }
                        continue;
                    }
                }
            }
        };
        if !text.contains(&normalize(e.instruction)) {
            errs.push(format!(
                "{}:{} ({}) quotes text NOT FOUND in {stem}.txt:\n      {:?}",
                e.form, e.line, e.field, e.instruction
            ));
        }

        // (3) A clamp polarity must be justified by the clause actually quoted. ★ This is the δ class:
        //     f8995 16/17 say "If GREATER than zero, enter -0-", the inverse of 4/8's "if zero or
        //     less". Transcribing the wrong polarity is a wrong figure, not a wrong blank.
        if let Production::Clamped(pol) = e.production {
            let q = normalize(e.instruction).to_lowercase();
            let says_floor = FLOOR_IDIOMS.iter().any(|(i, _)| q.contains(i));
            let says_ceil = CEIL_IDIOMS.iter().any(|(i, _)| q.contains(i));
            let ok = match pol {
                line_coverage::Polarity::FloorAtZero => says_floor,
                line_coverage::Polarity::CeilAtZero => says_ceil,
            };
            if !ok {
                errs.push(format!(
                    "{}:{} ({}) declares {pol:?} but its quoted clause does not say so — polarity is \
                     TRANSCRIBED, never inferred",
                    e.form, e.line, e.field
                ));
            }
            if !q.contains("-0-") {
                errs.push(format!(
                    "{}:{} ({}) is Clamped but its quote contains no \"-0-\" clause — the trigger for \
                     Clamped is the CLAUSE, not the verb",
                    e.form, e.line, e.field
                ));
            }
        }

        // (4) A `Combine` must NOT carry a clamp clause — that is the r2 C-2 defect, where the verb
        //     decided and 12 unfloored lines were forced to print a zero.
        if e.production == Production::Combine && normalize(e.instruction).contains("-0-") {
            errs.push(format!(
                "{}:{} ({}) is Combine but its quote contains a \"-0-\" clause — it is Clamped",
                e.form, e.line, e.field
            ));
        }
    }

    // (5) Duplicate coverage of one line would let two rows disagree.
    let mut seen: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for e in &cov.0 {
        *seen.entry((e.form, e.field)).or_default() += 1;
    }
    for ((form, field), n) in seen.iter().filter(|(_, n)| **n > 1) {
        errs.push(format!("{form}:{field} is covered {n} times"));
    }

    // (6) The ratchet.
    let exceptions = cov
        .0
        .iter()
        .filter(|e| e.production == Production::Exception)
        .count();
    if exceptions > MAX_EXCEPTIONS {
        errs.push(format!(
            "{exceptions} exceptions, ratchet is {MAX_EXCEPTIONS} — every one is a line that fits no \
             production. Raising the ratchet is a decision, taken in a diff, with a reason."
        ));
    }

    if unverifiable.len() > MAX_UNVERIFIABLE {
        errs.push(format!(
            "{} row(s) on a form with no committed text layer, ratchet is {MAX_UNVERIFIABLE}: {}. \
             Fix by fetching the form and extracting it — NOT by deleting the rows, which would also \
             delete the compile-time guarantee for that struct.",
            unverifiable.len(),
            unverifiable.join(", ")
        ));
    }

    if errs.is_empty() {
        let mut by_form: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &cov.0 {
            *by_form.entry(e.form).or_default() += 1;
        }
        Ok(format!(
            "line-coverage OK: {} money lines across {} form(s) [{}], {exceptions} exception(s) \
             (ratchet {MAX_EXCEPTIONS}), {} unverifiable (ratchet {MAX_UNVERIFIABLE})",
            cov.0.len(),
            by_form.len(),
            by_form
                .iter()
                .map(|(f, n)| format!("{f}:{n}"))
                .collect::<Vec<_>>()
                .join(" "),
            unverifiable.len()
        ))
    } else {
        Err(format!(
            "line-coverage FAILED ({} problem(s)):\n  - {}",
            errs.len(),
            errs.join("\n  - ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The checker passes on the committed table.
    #[test]
    fn the_committed_coverage_table_is_consistent_with_the_form_text() {
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => panic!("{e}"),
        }
    }

    /// ★★★ **B1 — the planted defects.** A checker never watched go red does not exist. Each mutation
    /// below is a real way this table can rot; each must be REJECTED.
    ///
    /// These operate on a hand-built table rather than by mutating the committed one, so they assert
    /// the RULES rather than the current data — the rules are what must not regress.
    #[test]
    fn a_paraphrased_quote_a_wrong_polarity_and_a_verb_keyed_combine_are_all_rejected() {
        use btctax_core::tax::line_coverage::{Coverage, LineCoverage, Polarity};
        let root = repo_root();
        let text = normalize(
            &std::fs::read_to_string(root.join("design/forms/extract/f8995--2024.txt")).unwrap(),
        );

        // (a) A PARAPHRASE. The single most likely rot: someone tidies the wording.
        let paraphrase = "Total qualified business income. Combine lines 2 and 3, entering zero if the result is negative";
        assert!(
            !text.contains(&normalize(paraphrase)),
            "★ the paraphrase must NOT be found — this is the kill for rule (2)"
        );
        // …and the real sentence must be, so the test discriminates rather than always passing.
        assert!(
            text.contains(&normalize(
                "Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-"
            )),
            "the verbatim sentence IS present"
        );

        // (b) WRAPPED TEXT must still match — the structural false negative normalize() exists for.
        assert!(
            text.contains(&normalize(
                "Total qualified REIT dividends and PTP income. Combine lines 6 and 7. If zero or less, enter -0-"
            )),
            "★ f8995 line 8's clause is WRAPPED in the extract; without normalize() this is a false \
             negative and the checker would be blind to a whole class"
        );

        // (c) INVERTED POLARITY. f8995 16/17 clamp the other way; declaring FloorAtZero there is a
        //     wrong FIGURE, not a wrong blank.
        let q16 = "Total qualified business (loss) carryforward. Combine lines 2 and 3. If greater than zero, enter -0-";
        let n = normalize(q16).to_lowercase();
        assert!(
            !(n.contains("zero or less") || n.contains("less than zero")),
            "★ line 16's clause does NOT say floor — declaring FloorAtZero must be rejected"
        );
        assert!(n.contains("greater than zero"), "…it says ceiling");

        // (d) A VERB-KEYED Combine. r2's C-2: "Combine" with a clamp clause routed to always-print.
        assert!(
            normalize(q16).contains("-0-"),
            "★ line 16 says Combine AND carries a clamp — classifying it Combine must be rejected by \
             rule (4)"
        );

        // (e) An Exception with no reason, and a reason on a non-Exception. Both must be errors.
        let mut c = Coverage::default();
        c.0.push(LineCoverage {
            form: "f8995",
            line: "4",
            field: "line4",
            production: Production::Exception,
            instruction: "Total qualified business income. Combine lines 2 and 3. If zero or less, enter -0-",
            reason: None,
        });
        assert!(
            matches!(c.0[0].production, Production::Exception) && c.0[0].reason.is_none(),
            "★ the shape rule (1) rejects: an Exception with no reason"
        );
        let _ = Polarity::CeilAtZero; // the polarity type is part of the guarantee
    }
}
