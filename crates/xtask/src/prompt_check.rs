//! `cargo run -p xtask -- prompt-check` — **FR-29 / SPEC §9 G7: the Form 8615 prompts and help
//! strings are TRANSCRIBED, and this is what holds them there.**
//!
//! The filer-facing text of Form 8615's three questions is the only place btctax puts the form's own
//! words in front of a person. `CLAUDE.md`'s standing rule is *transcribe, never paraphrase*, and a
//! prose review finds a paraphrase once; a check finds it forever.
//!
//! **Each clause is asserted TWICE, and both halves are load-bearing:**
//!
//! - **(a)** it appears verbatim (normalised) in **the extract that clause is sourced from**, and
//! - **(b)** it appears verbatim (normalised) in that question's `prompt` (or `help`).
//!
//! (b) alone lets the clause table drift from the form; (a) alone lets the prompt drift from the
//! table. Together, a paraphrase anywhere reds — the property harness rule **B1** calls *"cannot be
//! satisfied performatively"*.
//!
//! ★★ **The per-clause SOURCE column is not decoration.** i8615 says *"at least age 19 **and** under
//! age 24"* where i1040gi says *"at least age 19 **but** under age 24"* — same rule, one conjunction
//! apart. A single-extract table would either fail on that clause or silently check the prompt
//! against the wrong document.
//!
//! ★ **Normalisation is `cite_check::normalise`, named rather than described**: it folds Unicode
//! punctuation to ASCII, strips markdown emphasis and quote marks, removes the CAUTION/TIP icon
//! labels, de-hyphenates across line breaks, replaces every non-alphanumeric other than `$`, `%` and
//! whitespace with a space, collapses runs of whitespace, and lowercases. This module **reuses it
//! directly** and does not define a second one. Case-folding and whitespace-collapsing leave the B1
//! pairing red: *"most of your support"* still does not contain *"more than half of your support"*
//! under any amount of case folding.

use crate::cite_check::{normalise, repo_root};
use btctax_core::tax::questions::{SkippableId, SKIPPABLE_QUESTIONS};
use std::fmt::Write as _;

/// Which of a question's two filer-facing strings a clause is checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Face {
    Prompt,
    Help,
}

/// One clause: the question it belongs to, which of its strings carries it, the clause itself, and
/// **the extract it is sourced from**.
struct Clause {
    id: SkippableId,
    face: Face,
    text: &'static str,
    extract: &'static str,
}

/// SPEC §9 G7's clause table. Eight clauses, each with its own source.
const CLAUSES: &[Clause] = &[
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "under age 18",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "age 18",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "didn\u{2019}t have earned income that was more than half of your support",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Prompt,
        text: "a full-time student at least age 19 but under age 24",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition4ParentAlive,
        face: Face::Prompt,
        text: "at least one of your parents was alive",
        extract: "design/forms/extract/i1040gi--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Help,
        text: "These rules apply whether or not the child is a dependent",
        extract: "design/forms/extract/i8615--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615Condition3AgeSupport,
        face: Face::Help,
        text: "wages, tips, and other payments received for personal services performed",
        extract: "design/forms/extract/i8615--2025.txt",
    },
    Clause {
        id: SkippableId::Form8615ParentIdentityUnobtainable,
        face: Face::Help,
        text: "The name, address, social security number (SSN) (if known), and filing status (if \
               known) of the parent",
        extract: "design/forms/extract/i8615--2025.txt",
    },
];

/// ★★ **The STRUCTURAL half, and it is what actually pins the three limbs of condition 3.**
///
/// Clause 2 (`"age 18"`) is WEAK: it is a substring of `"under age 18"`, so clause 1 satisfies it and
/// **deleting limb (b) from the prompt would not red the clause table.** Limb (b) cannot be pinned
/// textually at all — the prompt hoists the year qualifier, so the extract's *"Age 18 at the end of
/// 2025 and"* and the prompt's *"age 18 and"* share no span longer than `"age 18"` itself.
///
/// So the prompt must also contain each limb OPENING exactly once, and the support clause exactly
/// twice (once for limb (b), once for limb (c) — which is how the form writes it). Deleting limb (b)
/// reds on both counts.
///
/// ★ **This half is PROMPT-ONLY, and the message says so**: these spans are the prompt's own hoisted
/// phrasing and are *not* in the extract, so it detects a limb being **dropped**, never a limb
/// **drifting** from the form. A structural assertion that looks like a conformance assertion is
/// precisely the green-and-blind instrument this whole check exists to avoid.
///
/// ★ Do **not** assert on the bare markers `(a)`/`(b)`/`(c)`: `normalise` strips the parentheses, and
/// the trailing sentence *"Answer YES if any one of (a), (b) or (c) is true."* makes their counts
/// 3/2/2, not 1/1/1.
const STRUCTURE: &[(&str, usize)] = &[
    (
        "didn\u{2019}t have earned income that was more than half of your support",
        2,
    ),
    ("(a) under age 18", 1),
    ("(b) age 18 and didn\u{2019}t have", 1),
    ("(c) a full-time student", 1),
];

fn face_text(id: SkippableId, face: Face) -> &'static str {
    let q = SKIPPABLE_QUESTIONS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("{id:?} is not in SKIPPABLE_QUESTIONS"));
    match face {
        Face::Prompt => q.prompt,
        Face::Help => q.help,
    }
}

/// Count non-overlapping occurrences of `needle` in `haystack` (both already normalised).
fn count(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut n = 0;
    let mut from = 0;
    while let Some(i) = haystack[from..].find(needle) {
        n += 1;
        from += i + needle.len();
    }
    n
}

/// Run the check over the live registry. `Ok(count)` is the number of clause assertions that passed
/// (both halves each); `Err` names every failure.
pub fn check() -> Result<usize, String> {
    let root = repo_root();
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;

    for (i, c) in CLAUSES.iter().enumerate() {
        let n = i + 1;
        let clause = normalise(c.text);
        // (a) — against the extract this clause is SOURCED FROM, named per clause.
        let path = root.join(c.extract);
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("clause {n}: cannot read {}: {e}", c.extract))?;
        let hay = normalise(&raw);
        if hay.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "clause {n} ({:?}) is NOT in its own source {}: {:?}",
                c.id, c.extract, c.text
            ));
        }
        // (b) — against the filer-facing string it belongs to.
        let face = normalise(face_text(c.id, c.face));
        if face.contains(&clause) {
            passed += 1;
        } else {
            failures.push(format!(
                "clause {n} ({:?} {:?}) is NOT in the string the filer reads: {:?}",
                c.id, c.face, c.text
            ));
        }
    }

    // The structural half — PROMPT-ONLY (see `STRUCTURE`).
    let prompt = normalise(face_text(
        SkippableId::Form8615Condition3AgeSupport,
        Face::Prompt,
    ));
    for (span, want) in STRUCTURE {
        let got = count(&prompt, &normalise(span));
        if got == *want {
            passed += 1;
        } else {
            failures.push(format!(
                "STRUCTURAL (prompt-only \u{2014} this detects a limb being DROPPED, never a limb \
                 DRIFTING from the form): condition 3's prompt contains {span:?} {got} time(s), \
                 want {want}"
            ));
        }
    }

    if failures.is_empty() {
        Ok(passed)
    } else {
        let mut msg = String::new();
        for f in &failures {
            let _ = writeln!(msg, "  {f}");
        }
        Err(msg)
    }
}

pub fn run() -> Result<(), String> {
    let passed = check()?;
    println!("xtask prompt-check: OK \u{2014} {passed} assertions, all verbatim");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check passes against the live registry.
    #[test]
    fn the_real_prompts_pass() {
        match check() {
            Ok(n) => assert!(n >= 20, "expected at least 20 assertions, got {n}"),
            Err(e) => panic!("prompt-check failed on the real prompts:\n{e}"),
        }
    }

    /// ★★★ **THE B1 PAIRING (harness rule B1, `CLAUDE.md`): the check is observed RED on a planted
    /// defect, AND observed GREEN on the real thing.**
    ///
    /// Modelled on `cite_check::a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`. The
    /// planted defect is the one the SPEC names: *"more than half of your support"* paraphrased to
    /// *"most of your support"*. That is a real paraphrase a well-meaning editor would make, and it
    /// changes the test the filer applies.
    ///
    /// ★ **Both directions are asserted**, because a checker that rejects everything is
    /// indistinguishable from one that works. The mutation is applied to a COPY of the prompt rather
    /// than to the registry, so the harness cannot leave the repo mutated.
    #[test]
    fn prompt_check_rejects_a_paraphrased_prompt_and_accepts_the_real_one() {
        let real = face_text(SkippableId::Form8615Condition3AgeSupport, Face::Prompt);
        let clause =
            normalise("didn\u{2019}t have earned income that was more than half of your support");

        // GREEN on the real one.
        assert!(
            normalise(real).contains(&clause),
            "the unmutated prompt must PASS — a checker that reds on everything is \
             indistinguishable from one that works"
        );

        // RED on the paraphrase. Case-folding and whitespace-collapsing do not save it: "most of
        // your support" does not contain "more than half of your support" under any amount of either.
        let paraphrased = real.replace("more than half of your support", "most of your support");
        assert_ne!(
            paraphrased, real,
            "the mutation must actually change the prompt"
        );
        assert!(
            !normalise(&paraphrased).contains(&clause),
            "PLANTED DEFECT NOT CAUGHT: the paraphrase 'most of your support' passed the clause \
             check, so this instrument has never been seen discriminating and does not exist (B1)"
        );

        // …and the structural half reds on the OTHER named mutation: dropping limb (b).
        let without_b = real.replace(
            "(b) age 18 and didn\u{2019}t have earned income that was more than half of your \
             support, or ",
            "",
        );
        let dropped = normalise(&without_b);
        assert_ne!(
            dropped,
            normalise(real),
            "the limb-(b) mutation must actually change the prompt"
        );
        assert_eq!(
            count(&dropped, &normalise("(b) age 18 and didn\u{2019}t have")),
            0,
            "PLANTED DEFECT NOT CAUGHT: limb (b) was deleted and the structural count did not move \
             \u{2014} which is exactly the hole clause 2 (\"age 18\", a substring of \"under age \
             18\") leaves in the clause table"
        );
        assert_eq!(
            count(&dropped, &clause),
            1,
            "…and the support clause drops from twice to once"
        );
    }
}
