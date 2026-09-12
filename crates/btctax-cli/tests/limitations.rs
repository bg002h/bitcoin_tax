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

/// ★★★ **T12 / FOLLOWUPS FR-73 — THE INTERVIEW'S STOP LIST IS IN THE FILER-FACING DOC, AND IT IS
///     DERIVED FROM THE CENSUS RATHER THAN TYPED.**
///
/// `LIMITATIONS.md` is the document a filer reads to find out where btctax stops. §2.2's excluded
/// families are exactly the document-census rows that carry an exit sentence — so the expectation
/// here is `DocumentRow::ALL` filtered by `exit_sentence().is_some()`, not a hand-list. A family
/// added to the census tomorrow reds this test until the filer-facing doc names it, which is the
/// only mechanism that keeps the doc from going quietly stale behind the code.
///
/// ★ It asserts the row's own **designation** — the filer's words for the piece of paper in their
///   hand — because that is what they are holding when they go looking.
#[test]
fn limitations_names_every_excluded_document_family_the_census_refuses() {
    use btctax_core::tax::document_census::DocumentRow;
    let doc = shipped_doc();
    let mut checked = 0usize;
    for row in DocumentRow::ALL {
        if row.exit_sentence().is_none() {
            continue; // transcribable today — not an excluded family, and saying so would be false
        }
        checked += 1;
        // The designation as the census words it, with the markdown emphasis stripped out of the
        // haystack so a bolded name still counts as named.
        let hay = doc.replace("**", "");
        assert!(
            hay.contains(row.designation()),
            "LIMITATIONS.md must name the excluded family {:?} in the filer's own words ({:?}) — \
             the interview refuses it and the doc is where a filer looks to find out why",
            row,
            row.designation()
        );
    }
    assert!(
        checked >= 8,
        "only {checked} excluded families were checked — a walk that finds nothing passes by \
         finding nothing"
    );
}

/// ★★★ **T12 / FOLLOWUPS FR-73 — THE VENUE/ACCOUNT GRANULARITY NOTE HAS A DOCS HOME.**
///
/// `step0::VENUE_GRANULARITY_NOTE` is printed by `income answer` and by the TUI's Step 0 panel, and
/// until T12 it appeared nowhere a filer could read it outside a live session. The two load-bearing
/// tokens are taken **out of the shipped constant at test time**, so this reds if either surface
/// drifts: change the venue-key shape or the statutory cite in the code and the doc stops matching.
#[test]
fn limitations_carries_the_venue_account_granularity_note() {
    let doc = shipped_doc();
    let note = btctax_cli::step0::VENUE_GRANULARITY_NOTE;
    for token in ["exchange:<venue>:default", "§1012(c)(1)"] {
        assert!(
            note.contains(token),
            "the shipped constant must still carry {token:?} — if it does not, this test is \
             checking the wrong thing: {note}"
        );
        assert!(
            doc.contains(token),
            "LIMITATIONS.md must carry the venue/account granularity note's {token:?} (FR-73): a \
             filer with two accounts at one venue has no other way to learn that the standing \
             order is recorded per VENUE"
        );
    }
}

/// ★★★ **PHASE 4's EXIT GATE, in the shipped doc** — `btctax limitations` is a surface a filer can
/// read without ever exporting a packet, and it is the one that explains why this product prints no
/// mailing address at all.
///
/// The two facts are carried VERBATIM from `btctax_cli`'s constants, so the doc cannot drift from the
/// packet manifest and `btctax extension`. That drift is the whole reason this test exists: the
/// manifest is generated from the constants and reds if either fact is dropped, but LIMITATIONS.md is
/// a committed file that nothing would otherwise hold.
///
/// Mutation: soften either fact in LIMITATIONS.md and this reds naming that fact.
#[test]
fn limitations_carries_the_where_to_file_facts_and_the_retention_guidance() {
    let doc = shipped_doc();

    assert_eq!(
        btctax_cli::missing_where_to_file_facts(&doc, btctax_cli::WHERE_TO_FILE_1040_SOURCE),
        Vec::<&str>::new(),
        "the shipped doc must carry BOTH facts and the RETURN's table pointer, verbatim"
    );
    let norm = btctax_cli::normalize_guidance(&doc);
    assert!(
        norm.contains(&btctax_cli::normalize_guidance(
            btctax_cli::RECORD_RETENTION_GUIDANCE
        )),
        "…and the retention guidance, verbatim"
    );
    // ★ The extension's address is NOT the return's, and a filer reading only this doc must be told
    //   so — Form 4868 is posted weeks earlier, in its own envelope, to a different center.
    assert!(
        norm.contains("Where To File a Paper Form 4868"),
        "…and that Form 4868 has its own table"
    );
    // ★★ **The doc must not print an address itself — and that claim is NOT asserted here.** The
    //    first draft of this test hand-typed two of the six addresses as a negative list, and
    //    `xtask`'s `service_center_check` immediately reported this very file: a line asserting
    //    `!contains("…, NC 28201-1214")` contains a service-center address, in a file the guard
    //    scans. That is the guard working, not a false positive — and the right fix was to delete the
    //    assertion rather than to excuse the file, because a two-entry negative list beside a
    //    six-address table is exactly the shape `CLAUDE.md`'s *"derive the list, or make the compiler
    //    hold it"* forbids: it would pass on the four it never named.
    //
    //    `crates/xtask/src/service_center_check.rs` holds that half, by SHAPE, over this document and
    //    every `.rs` file in the workspace.
}
