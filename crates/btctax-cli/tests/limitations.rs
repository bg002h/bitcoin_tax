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
