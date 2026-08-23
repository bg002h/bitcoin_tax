//! `btctax limitations` — the versioned LIMITATIONS / supported-forms doc (SPEC §9.2).
//!
//! [★ P5-N4] The subcommand's whole job is to put the shipped doc in front of the filer, and nothing
//! tested that it did. `include_str!` guarantees the doc is *embedded*; only driving the binary
//! proves it is *printed*, on stdout, in full, and byte-identical to the file that ships.
//!
//! [★ P5-I4] The doc lives at `crates/btctax-cli/LIMITATIONS.md` — INSIDE the package root. It was
//! at the repo root, reached by `include_str!("../../../LIMITATIONS.md")`, which put it outside the
//! `.crate` tarball: the publish-verification build of the packaged crate could not compile. The
//! path assertion below fails loudly if anyone moves it back out.
use std::path::Path;
use std::process::Command;

/// The doc, as it ships inside the crate. If this path changes, `cargo publish` breaks (P5-I4).
fn shipped_doc() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("LIMITATIONS.md");
    assert!(
        p.exists(),
        "LIMITATIONS.md must live inside crates/btctax-cli/ or it is not in the .crate tarball \
         and `cargo publish` fails to compile the packaged crate (P5-I4): {}",
        p.display()
    );
    std::fs::read_to_string(p).expect("read LIMITATIONS.md")
}

#[test]
fn limitations_prints_the_shipped_doc_verbatim() {
    let out = Command::new(env!("CARGO_BIN_EXE_btctax"))
        .arg("limitations")
        .output()
        .expect("run btctax limitations");

    assert!(out.status.success(), "exit: {:?}", out.status);
    let stdout = String::from_utf8(out.stdout).expect("utf-8");
    assert_eq!(
        stdout,
        shipped_doc(),
        "`btctax limitations` must print the shipped doc byte-for-byte"
    );
    assert!(out.stderr.is_empty(), "nothing belongs on stderr");
}

/// The doc is the *contract* for what v1 does and does not do, so its three §3.4-aligned lists must
/// actually be present — a truncated or reorganized doc that silently lost one of them would still
/// pass a byte-identity check against itself.
#[test]
fn limitations_doc_has_its_three_lists() {
    let doc = shipped_doc();
    for heading in ["REFUS", "OMISSION", "UNREPRESENTABLE"] {
        assert!(
            doc.contains(heading),
            "LIMITATIONS.md must still carry its {heading} list"
        );
    }
}

/// The **NOTICE** clauses are load-bearing legal text, not prose that may drift. They disclaim
/// authorisation, warranty and liability for filing — deliberately WITHOUT restricting the MIT /
/// Unlicense grant or purporting to forbid filing (which would be unenforceable, and would contradict
/// the fact that btctax produces a filable packet). If someone softens or deletes one of these, the
/// tool's legal posture changes silently. Pin the load-bearing sentences.
#[test]
fn limitations_carries_the_no_authorisation_notice() {
    // Normalize whitespace: the clauses are legal SENTENCES, and a markdown reflow must not be able
    // to break the check (nor to hide a deletion behind one).
    let doc = shipped_doc()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for clause in [
        // no authorisation
        "No right is granted, and no authorisation is given",
        "to prepare or file a tax return",
        // no warranty of fitness for filing
        "no representation and give no warranty",
        "a refusal is a best effort, not a guarantee",
        // you are the preparer
        "entirely on your own responsibility",
        "accept **no liability**",
        "The signature on it is yours alone.",
        // not tax advice
        "is a substitute for a qualified professional",
    ] {
        assert!(
            doc.contains(clause),
            "the NOTICE clause {clause:?} has been weakened or removed from LIMITATIONS.md"
        );
    }

    // …and the licence grant itself must remain UNRESTRICTED. The notice is a liability posture, not
    // a use restriction: if someone converts it into one, the software stops being open source and
    // `license = \"MIT OR Unlicense\"` in Cargo.toml becomes false.
    assert!(
        doc.contains("**MIT OR Unlicense**) — unchanged and unrestricted"),
        "the licence grant must stay unrestricted — the NOTICE disclaims, it does not forbid"
    );
}

/// ★ P10 (FILING-READINESS-PLAN rank 15) — **the Schedule 8812 row must be CONDITIONAL**, and this
/// is the kill-test that keeps it so. `grep "8812\|overstated" tests/limitations.rs` returned NOTHING
/// before this: the doc's most consequential claim was pinned by no test at all.
///
/// The row used to say, flatly: *"1040 line 19 is pinned to **$0**. File Schedule 8812 yourself. Your
/// tax is overstated by up to that amount."* That is precisely the claim `ctc_provably_zero` exists to
/// stop an advisory making. A filer above the §24(b) phase-out reads it, prepares a Schedule 8812 that
/// pays $0, and the doc has contradicted the advisory their own report printed — *"CTC/ODC NOT
/// COMPUTED, AND NOT AVAILABLE TO YOU … there is no Schedule 8812 for you to file."* The filing trial
/// that produced `ctc_provably_zero` found exactly this shape in the advisory (AGI $2,085,000, nine
/// children, $18,000 of credit §24(b) had already removed); the fix never reached the document.
///
/// Two things are asserted, and the second is what makes this more than a spelling check: the doc must
/// quote the phrase that **DISTINGUISHES** the advisory's two branches, and that phrase is checked
/// against the live `Advisory::message()` for both branches — so it cannot be a phrase common to both,
/// and it reds if either the advisory text or the doc drifts away from the other.
#[test]
fn the_schedule_8812_row_is_conditional_on_the_24b_phase_out() {
    use btctax_core::tax::advisories::Advisory;

    let doc = shipped_doc();
    let row = doc
        .lines()
        .find(|l| l.starts_with("| **Child Tax Credit"))
        .expect("the OMISSIONS table must still carry the Child Tax Credit row")
        .to_string();

    // The old, unconditional claim must be gone. Restoring it reds here.
    assert!(
        !row.contains("File Schedule 8812 yourself. Your tax is overstated by up to that amount."),
        "the Schedule 8812 row must not assert overstatement unconditionally — that is false for \
         every filer above the §24(b) phase-out, and it contradicts the advisory their own report \
         printed: {row}"
    );
    // Both branches must be named, so a filer can tell which one they are in.
    assert!(
        row.contains("§24(b)"),
        "the row must name the phase-out that makes the $0 correct: {row}"
    );
    assert!(
        row.contains("correct") && row.contains("overstated"),
        "the row must state BOTH outcomes — the $0 is the CORRECT figure above the phase-out, and \
         the tax is OVERSTATED below it: {row}"
    );

    // ★ The doc must quote the phrase that discriminates the two advisory branches, and the phrase
    //   must actually discriminate — checked against the live messages, not asserted by hand.
    const QUOTED: &str = "NOT AVAILABLE TO YOU";
    let provably_zero = Advisory::CtcOdcOmitted {
        dependents: 1,
        provably_zero: true,
    }
    .message();
    let not_proven = Advisory::CtcOdcOmitted {
        dependents: 1,
        provably_zero: false,
    }
    .message();
    assert!(
        provably_zero.contains(QUOTED) && !not_proven.contains(QUOTED),
        "{QUOTED:?} must be the phrase that DISTINGUISHES the two CtcOdcOmitted branches, or the doc \
         is pointing the filer at something they cannot match:\n  zero: {provably_zero}\n  other: \
         {not_proven}"
    );
    assert!(
        row.contains(QUOTED),
        "the row must quote the advisory's distinguishing phrase, so the filer can match the \
         advisory they actually saw: {row}"
    );
}
