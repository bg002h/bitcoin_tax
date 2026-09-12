//! ★★★ **NO IRS SERVICE-CENTER POSTAL ADDRESS MAY ENTER SHIPPED TEXT.**
//!
//! The long-range plan's Phase 4 decided the mailing-address help is *"a link + the two facts, not a
//! bundled table that rots"* (§3 Phase 4, decision **D-H**), and the reason is load-bearing rather
//! than stylistic: **a rotted address fails silently.** A wrong URL 404s in front of the filer; a
//! signed return posted to a closed service center produces no error message, no bounce and no
//! notification — it is simply late, or lost, months later.
//!
//! The authority says the set is moving. The IRS corrected the Form 1040-ES addresses mid-2026
//! (recon-efile §5), and the Instructions for Form 1040 print the warning themselves
//! (`design/forms/extract/i1040gi--2025.txt:40973-40978`):
//!
//! ```text
//! Over the next several years, the IRS will be reducing the number of paper tax return processing
//! sites. Because of this, you may need to mail your return to a different address than you have in
//! the past.
//! ```
//!
//! ★★ **Why a checker and not a note in `CLAUDE.md`.** The plan wrote the decision down; a written
//! decision is exactly what this repo has watched get violated the same day (`design/HARNESS.md`,
//! the doctrine this module answers). Someone reading *"a filer holding the packet can post it
//! without consulting anything outside it"* will eventually conclude the helpful thing is to paste
//! the six addresses in, and nothing would red. This is `CLAUDE.md`'s *"derive the list, or make the
//! compiler hold it"* applied to a **policy** rather than to a list: the guard makes undoing the
//! decision a build failure instead of a quiet edit.
//!
//! ## What a finding is — two shapes, both derived rather than enumerated
//!
//! | shape | why it is the discriminator |
//! |---|---|
//! | [`usps_last_line`] | a USPS last line, `<…>, <ST> <ZIP>` — the comma-then-two-capitals-then-five-digits run no legal citation, EIN, SSN or dollar amount produces |
//! | [`post_office_box`] | `P.O. Box <number>` — every with-payment IRS address is a lockbox box number, and the phrase has no other use in this domain |
//!
//! **Neither lists a city, a state or a ZIP.** A list of the six current centers would be the same
//! rotting artifact one level up: it would go stale the moment the IRS opens a seventh, and it would
//! pass a *new* center's address through in silence. The shapes decide.
//!
//! ## Where it is blind — stated, not implied
//!
//! 1. **The bundled official IRS templates.** Measured 2026-09-11 with `pdftotext -layout`: real
//!    service-center addresses ship inside the binary via `btctax-forms`' `include_bytes!` —
//!    `forms/2024/f1040v.pdf` **4** last-line matches, `forms/2024/f4868.pdf` **18**,
//!    `forms/2025/f1040v.pdf` **3**, `forms/2025/f4868.pdf` **14**. That is correct and must not be
//!    "fixed": they are the IRS's own pages, they are *per year*, and Form 1040-V's own table is what
//!    a paying filer is pointed at. This module guards the text **btctax authors**, not the forms it
//!    reproduces.
//! 2. **The archived text-layer extracts** (`design/forms/extract/`,
//!    `crates/btctax-core/src/tax/fixtures/`) carry the tables verbatim and are not scanned. They are
//!    `# GENERATED — do not hand-edit` transcriptions of the authority, and they are not `.rs`.
//! 3. **Comments and `#[cfg(test)]` items**, stripped and skipped by
//!    [`crate::r15_stop_list::production_source`]. This header prints two real addresses on purpose,
//!    and the kill below constructs several more; neither reports itself, and neither needs a
//!    path-keyed excuse to avoid it (the stale-excuse shape `CLAUDE.md` names). A comment cannot put
//!    an address in front of a filer.
//! 4. ★ **An address ASSEMBLED at runtime is invisible.** `format!("{city}, {st} {zip}")` reads as
//!    three fields here and as a service-center address on the envelope. This is a source scanner, and
//!    closing that would need a data-flow analysis; naming it is the honest boundary. Nothing in the
//!    tree does this today — there is no city, state or ZIP field anywhere on the export path to do it
//!    with.
//! 5. Shipped text that is neither `.rs` nor [`SHIPPED_DOC`]. A future filer-facing markdown
//!    `include_str!`'d into a binary would not be scanned until it is added to [`scanned_text`].

use std::path::{Path, PathBuf};

/// The one shipped **non-Rust** filer-facing document: `btctax limitations` prints it verbatim, and
/// `main.rs` `include_str!`s it into the binary.
pub const SHIPPED_DOC: &str = "crates/btctax-cli/LIMITATIONS.md";

/// The walk must see at least this many files. **A scan that reads nothing passes by finding
/// nothing** — `crates/**/*.rs` measured **355** on 2026-09-11
/// (`find crates -name "*.rs" -not -path "*/target*" | wc -l`), plus [`SHIPPED_DOC`]. Deliberately a
/// loose floor rather than a pinned count: it exists so an empty scan cannot report clean.
const FILE_FLOOR: usize = 300;

/// ★★ **The closed USPS two-letter set** — the only two-letter tokens that can be the state of a
/// postal address.
///
/// **Why a typed list is lawful here, when `CLAUDE.md` says never to type one beside a set that
/// grows.** That rule is about a set *this repo* widens beneath the list in a later, unrelated edit
/// (a new `RefuseReason`, a new document family, a new filing status). This set is not ours and does
/// not move: it is the ANSI/USPS state-and-territory codes. And the list is not trusted on faith —
/// [`tests::the_state_set_covers_every_state_the_archived_tables_actually_use`] DERIVES the tokens
/// that appear in the archived where-to-file tables and asserts every one is here, so the members
/// that are actually load-bearing come from the authority rather than from memory.
///
/// ★ **It is required because shape alone cannot separate an address from a citation.** `Austin, TX
/// 73301` and `the regulation, TD 10000` are the same shape: comma, two capitals, a five-digit run.
/// Measured on this tree, `[A-Z]{2} [0-9]{5}` matches 12 times and all 12 are citations — `TD 10000`
/// (Treasury Decision) and `89 FR 85279` (Federal Register) — and two of those sites put a comma
/// immediately before the reporter (`year_record.rs:43`, *"(TY2025+, TD 10000)"*). Without this set
/// the guard reds on a regulation citation, which is how a guard gets deleted.
///
/// Blind spot, named: if a NEW state or territory code is ever issued, an address in it is missed
/// until it is added here — and the derived test above reds on the next authority refresh that prints
/// one, which is the direction to fail in.
pub const USPS_STATES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "DC", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
    "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM",
    "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA",
    "WV", "WI", "WY", "AS", "GU", "MP", "PR", "VI", "AA", "AE", "AP",
];

/// The archived IRS text the state set is pinned against — the two where-to-file tables and the
/// voucher's payments table, as committed text layers. Repo-relative.
const ARCHIVED_ADDRESS_TABLES: &[&str] = &[
    "design/forms/extract/i1040gi--2025.txt",
    "crates/btctax-core/src/tax/fixtures/f4868_2025_instructions.txt",
    "crates/btctax-core/src/tax/fixtures/f1040v_2025_form.txt",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// Every `.rs` file under `crates/` plus [`SHIPPED_DOC`], sorted, as `(repo-relative label, text)`.
///
/// ★ `tests/` is NOT excluded, unlike `forge_reach_check`'s production scan. A table pasted into a
/// fixture is one copy-paste from the manifest, and excluding a directory here would be a boundary
/// drawn by path rather than by shape. The kill's own plants are safe from it because they live in a
/// `#[cfg(test)]` item, which [`crate::r15_stop_list::production_source`] skips wherever it sits.
#[must_use]
pub fn scanned_text() -> Vec<(String, String)> {
    let root = repo_root();
    let mut files = Vec::new();
    let mut stack = vec![root.join("crates")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    files.sort();
    let mut out: Vec<(String, String)> = files
        .into_iter()
        .map(|p| {
            let label = p
                .strip_prefix(&root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            (label, src)
        })
        .collect();
    let doc = root.join(SHIPPED_DOC);
    out.push((
        SHIPPED_DOC.to_string(),
        std::fs::read_to_string(&doc).unwrap_or_default(),
    ));
    out
}

/// A USPS **last line** — `<anything>, <ST> <ZIP5>[…]` — anywhere in `line`.
///
/// The comma is the whole discriminator, and it is what keeps this off the repo's real ZIP-shaped
/// digit runs. Measured on the committed tree, `[A-Z]{2} [0-9]{5}` alone matches **12** times and
/// every one is a legal citation — `TD 10000` (Treasury Decision, the Form 1099-DA regulation, 4
/// sites) and `89 FR 85279` (Federal Register, 8 sites). None has a comma before the two capitals,
/// because a citation is `<volume> <reporter> <page>` and an address is `<city>, <state> <zip>`.
///
/// The state token must be **exactly** two capitals AND a member of [`USPS_STATES`]: a third letter
/// means an ordinary capitalised word (`…, THE 12345`), and two capitals that are not a state code
/// mean a legal citation (`…, TD 10000`) — which is why the set is needed at all. The ZIP run must be
/// at least five digits, so a tax year (`, TY 2024`) is not a finding either. A `+4` suffix is
/// accepted but never required — a malformed suffix must not rescue a real ZIP5.
#[must_use]
pub fn usps_last_line(line: &str) -> bool {
    let c: Vec<char> = line.chars().collect();
    for i in 0..c.len() {
        if c[i] != ',' {
            continue;
        }
        let mut j = i + 1;
        while j < c.len() && c[j] == ' ' {
            j += 1;
        }
        if j + 1 >= c.len() || !(c[j].is_ascii_uppercase() && c[j + 1].is_ascii_uppercase()) {
            continue;
        }
        let mut k = j + 2;
        // A third letter makes it a word, not a state abbreviation.
        if k < c.len() && c[k].is_ascii_alphabetic() {
            continue;
        }
        // …and the two letters must be a real USPS code, or every `, TD 10000` citation is an
        // address. See [`USPS_STATES`].
        let state: String = [c[j], c[j + 1]].iter().collect();
        if !USPS_STATES.contains(&state.as_str()) {
            continue;
        }
        let mut spaces = 0;
        while k < c.len() && c[k] == ' ' {
            k += 1;
            spaces += 1;
        }
        if spaces == 0 {
            continue;
        }
        let mut digits = 0;
        while k < c.len() && c[k].is_ascii_digit() {
            k += 1;
            digits += 1;
        }
        if digits >= 5 {
            return true;
        }
    }
    false
}

/// A post-office box **with a number** — `P.O. Box 1214`, `PO Box 931000`, `Post Office Box 1302`.
///
/// ★ The number is required, and that is what makes the guidance this work added lawful: it discusses
/// P.O. boxes at length (*"Only the U.S. Postal Service can deliver to P.O. boxes"*, *"the
/// with-payment address is always a P.O. Box"*) and names not one. Measured on the committed tree:
/// **6** `p.o. box`-shaped mentions in `.rs`, all of them that guidance, all of them numberless.
///
/// Punctuation and spacing are normalised away first, so `P.O.Box`, `P O Box` and `PO  Box` are one
/// shape. `box 3` on its own is not a finding — the repo says that about a Form 1040-V field roughly
/// a thousand times.
#[must_use]
pub fn post_office_box(line: &str) -> bool {
    let lower = line.to_lowercase();
    let mut flat = String::with_capacity(lower.len());
    let mut last_space = false;
    for ch in lower.chars() {
        if ch == '.' {
            continue;
        }
        if ch.is_whitespace() {
            if !last_space {
                flat.push(' ');
                last_space = true;
            }
            continue;
        }
        last_space = false;
        flat.push(ch);
    }
    for needle in ["po box ", "p o box ", "post office box "] {
        let mut from = 0;
        while let Some(at) = flat[from..].find(needle) {
            let after = from + at + needle.len();
            if flat[after..].starts_with(|c: char| c.is_ascii_digit()) {
                return true;
            }
            from = from + at + needle.len();
        }
    }
    false
}

/// ★★★ **Every service-center address in shipped text**, as `label:line: text` findings.
///
/// Pure over `(label, text)` pairs so a planted defect can reach it (B1). `.rs` files are reduced to
/// their production half first — comments stripped, `#[cfg(test)]` items skipped — and [`SHIPPED_DOC`]
/// is scanned whole, because a markdown document has no test half and every line of it is printed to
/// the filer by `btctax limitations`.
#[must_use]
pub fn service_center_addresses(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (label, text) in files {
        let scanned = if label.ends_with(".rs") {
            crate::r15_stop_list::production_source(text)
        } else {
            text.clone()
        };
        for (i, line) in scanned.lines().enumerate() {
            if usps_last_line(line) || post_office_box(line) {
                out.push(format!("{label}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed tree carries no service-center postal address in any text btctax authors.
    #[test]
    fn no_shipped_text_carries_an_irs_service_center_address() {
        let files = scanned_text();
        assert!(
            files.len() >= FILE_FLOOR,
            "the walk saw {} files (floor {FILE_FLOOR}) — a scan that reads nothing reports clean",
            files.len()
        );
        assert!(
            files.iter().any(|(l, t)| l == SHIPPED_DOC && !t.is_empty()),
            "the shipped filer-facing doc must be in the scan and must not be empty — it is what \
             `btctax limitations` prints, and a silently unreadable path would make this check blind \
             to the one non-Rust surface it covers"
        );
        let found = service_center_addresses(&files);
        assert!(
            found.is_empty(),
            "an IRS service-center postal address is in shipped text. The plan's Phase 4 decision \
             (D-H) is a LINK PLUS THE TWO FACTS and never a bundled table: the IRS corrected the \
             1040-ES addresses mid-2026 and warns in the instructions that it is reducing the number \
             of paper processing sites, so a compiled-in address goes stale between releases and a \
             signed return posted to a closed center fails with NO error message. Point the filer at \
             the year's own \"Where Do You File?\" table instead (see \
             `btctax_cli::WHERE_TO_FILE_1040_SOURCE`): {found:?}"
        );
    }

    /// ★★★ **B1 — watched RED on real planted addresses, and on the near misses beside them.**
    ///
    /// The near misses are the point. A guard that reds on every five-digit run is deleted by the
    /// next person it trips; a guard seen red on nothing was never seen discriminating. Every near
    /// miss below is a shape that is really in this repo — legal citations, EINs, SSNs from the
    /// never-issued space, dollar amounts, `box N` field references, and the numberless P.O. Box
    /// prose this very feature added.
    #[test]
    fn a_planted_service_center_address_reds_and_its_near_misses_do_not() {
        let hits = |label: &str, text: &str| {
            service_center_addresses(&[(label.into(), text.into())]).len()
        };

        // ══ THE PLANTS — the six-address table the decision forbids, as someone would paste it.
        assert_eq!(
            hits(
                "crates/btctax-cli/src/cmd/admin.rs",
                "        \"Department of the Treasury, Internal Revenue Service, Austin, TX 73301-0002\","
            ),
            1,
            "the no-payment last line is the defect this guard exists for"
        );
        assert_eq!(
            hits(
                "crates/btctax-cli/src/cmd/admin.rs",
                "        \"Internal Revenue Service, P.O. Box 1214, Charlotte, NC 28201-1214\","
            ),
            1,
            "…and the with-payment lockbox line"
        );
        // A bare ZIP5, no +4 — a malformed suffix must not rescue a real address.
        assert_eq!(
            hits("crates/btctax-cli/src/render.rs", "    \"Ogden, UT 84201\""),
            1,
            "a five-digit ZIP with no +4 is still an address"
        );
        // The BOX alone, with the city line dropped.
        assert_eq!(
            hits("crates/btctax-cli/src/render.rs", "    \"PO Box 931000\""),
            1,
            "a lockbox number alone is still half a service-center address"
        );
        // The SHIPPED DOC — no `.rs`, no test half, every line printed by `btctax limitations`.
        assert_eq!(
            hits(
                SHIPPED_DOC,
                "- If you owe, post it to Charlotte, NC 28201-1214."
            ),
            1,
            "the filer-facing markdown is in scope"
        );
        // …and it is scanned WHOLE: markdown has no `#[cfg(test)]`, so a plant behind one must not
        // be skipped the way a Rust test module is.
        assert_eq!(
            hits(
                SHIPPED_DOC,
                "#[cfg(test)]\nSee the table: Kansas City, MO 64999-0002\n"
            ),
            1,
            "a markdown line cannot hide behind a Rust attribute"
        );

        // ══ NEAR MISS 1 — legal citations. Measured: 12 `[A-Z]{2} [0-9]{5}` runs on this tree and
        //    every one of them is one of these two.
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/printed.rs",
                "    pub proceeds: bool; // From TY2026 brokers report BASIS on Form 1099-DA (TD 10000)"
            ),
            0,
            "a Treasury Decision citation is not an address"
        );
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/testonly.rs",
                "    let src = \"an SSA determination, taken here from 89 FR 85279\";"
            ),
            0,
            "a Federal Register citation is not an address"
        );
        // ══ NEAR MISS 2 — A COMMA RIGHT BEFORE A CITATION, which is the closest shape there is: the
        //    comma is present, the two capitals are present, the five digits are present. Only
        //    `USPS_STATES` separates it, and this exact text is committed at `year_record.rs:43`
        //    (*"(TY2025+, TD 10000)"*). The first draft of this guard reported it as an address.
        for citation in [
            "    let s = \"Brokers report basis on Form 1099-DA (TY2025+, TD 10000).\";",
            "    let s = \"an SSA determination, 89 FR 85279, Vol. 89 No. 207\";",
            "    let s = \"the year, TY 2024, and the volume, FR 85279\";",
        ] {
            assert_eq!(
                hits("crates/btctax-core/src/tax/printed.rs", citation),
                0,
                "a comma before a citation must not manufacture an address: {citation}"
            );
        }
        // …and the state set is what does it, so the same line with a REAL state code is a finding.
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/printed.rs",
                "    let s = \"Brokers report basis, TX 10000.\";"
            ),
            1,
            "the state set is the discriminator, not an excuse — swap the reporter for a state code \
             and the same line is an address"
        );

        // ══ NEAR MISS 3 — the identifiers the PII scan already governs: EINs from
        //    `scripts/pii-scan-generic.sh`'s ALLOWED_EIN, and SSNs from the never-issued space.
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/document_census.rs",
                "    payer_ein: \"99-9999999\", second: \"12-3456789\", third: \"00-0000000\","
            ),
            0,
            "an EIN is not a ZIP"
        );
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/packet.rs",
                "    let ssn = \"987-65-4321\"; // never-issued space"
            ),
            0,
            "an SSN is not a ZIP"
        );
        assert_eq!(
            hits(
                "scripts/pii-scan-generic.sh.rs",
                "    let allowed = \"^(90-0000001|91-0000002|55-5555555|99-1000000)$\";"
            ),
            0,
            "the PII scan's own allow-list of synthetic EINs is not a table of addresses"
        );

        // ══ NEAR MISS 4 — money. Grouped thousands put a comma beside five digits.
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/tables.rs",
                "    let refused = \"the §55(d)(3) threshold, MFS 875950, and $1,234,567 besides\";"
            ),
            0,
            "a grouped dollar amount is not an address"
        );

        // ══ NEAR MISS 5 — `box N`, said about 1,180 times in this tree about form fields.
        assert_eq!(
            hits(
                "crates/btctax-core/src/tax/form1099.rs",
                "    let msg = \"Form 1040-V box 3, 1099-DA box 1g, W-2 box 12a\";"
            ),
            0,
            "a form field is not a lockbox"
        );
        // ══ NEAR MISS 6 — the guidance THIS feature added: P.O. boxes discussed, none named. All 6
        //    committed mentions have this shape, and a guard that reds on them would be deleted.
        assert_eq!(
            hits(
                "crates/btctax-cli/src/cmd/admin.rs",
                "        \"Only the U.S. Postal Service can deliver to P.O. boxes; every \
                 with-payment address is a P.O. Box, so a paying envelope goes by USPS.\","
            ),
            0,
            "naming the RULE about P.O. boxes without naming a box is exactly what the decision asks \
             for, and must stay lawful"
        );

        // ══ NEAR MISS 7 — a comment, and a `#[cfg(test)]` item. Both are stripped/skipped, which is
        //    why this module's own header can print real addresses without excusing itself by path.
        assert_eq!(
            hits(
                "crates/btctax-cli/src/cmd/admin.rs",
                "    // the no-payment column is Austin, TX 73301-0002 — do not print it"
            ),
            0,
            "a comment cannot put an address in front of a filer"
        );
        assert_eq!(
            hits(
                "crates/btctax-cli/src/cmd/admin.rs",
                "#[cfg(test)]\nmod t {\n    const A: &str = \"Ogden, UT 84201-0002\";\n}\n"
            ),
            0,
            "a test fixture is not shipped text"
        );
    }

    /// ★★★ **The state set's load-bearing members are DERIVED from the authority, not remembered.**
    ///
    /// Every two-letter token that appears in an address last line in the archived where-to-file
    /// tables must be in [`USPS_STATES`]. Measured 2026-09-11 the three files yield
    /// `{DC, KY, MI, MO, NC, TX, UT}` — and a service center opening in a state the list somehow
    /// lacks reds here on the next authority refresh rather than passing an address through in
    /// silence.
    ///
    /// The scan here is deliberately the RAW shape (any two capitals), not [`usps_last_line`] — using
    /// the checker to validate its own set would be circular.
    #[test]
    fn the_state_set_covers_every_state_the_archived_tables_actually_use() {
        let root = repo_root();
        let mut seen: Vec<String> = Vec::new();
        for rel in ARCHIVED_ADDRESS_TABLES {
            let text = std::fs::read_to_string(root.join(rel))
                .unwrap_or_else(|e| panic!("the archived table {rel} must be readable: {e}"));
            for line in text.lines() {
                let c: Vec<char> = line.chars().collect();
                for i in 0..c.len() {
                    if c[i] != ',' {
                        continue;
                    }
                    let mut j = i + 1;
                    while j < c.len() && c[j] == ' ' {
                        j += 1;
                    }
                    if j + 2 >= c.len()
                        || !(c[j].is_ascii_uppercase() && c[j + 1].is_ascii_uppercase())
                        || c[j + 2].is_ascii_alphabetic()
                    {
                        continue;
                    }
                    let mut k = j + 2;
                    let mut spaces = 0;
                    while k < c.len() && c[k] == ' ' {
                        k += 1;
                        spaces += 1;
                    }
                    if spaces == 0 {
                        continue;
                    }
                    let mut digits = 0;
                    while k < c.len() && c[k].is_ascii_digit() {
                        k += 1;
                        digits += 1;
                    }
                    if digits >= 5 {
                        let tok: String = [c[j], c[j + 1]].iter().collect();
                        if !seen.contains(&tok) {
                            seen.push(tok);
                        }
                    }
                }
            }
        }
        seen.sort();
        assert!(
            seen.len() >= 5,
            "the derivation read the archived tables and found almost no addresses ({seen:?}) — a \
             scan that reads nothing pins nothing, and the where-to-file tables print at least five \
             service-center states"
        );
        let missing: Vec<&String> = seen
            .iter()
            .filter(|t| !USPS_STATES.contains(&t.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "the archived IRS where-to-file tables print a state code that USPS_STATES does not \
             carry, so `usps_last_line` would wave that center's address straight through: \
             {missing:?} (derived set {seen:?})"
        );
    }

    /// ★★★ **THE POINTER'S TARGET MUST EXIST — the complement of the guard above.**
    ///
    /// Declining to print the table is only safe if the thing btctax points AT really carries it. Two
    /// claims in the shipped guidance are claims about the forms themselves, and both would become
    /// lies in silence if a later revision dropped a table:
    ///
    /// | claim | surface |
    /// |---|---|
    /// | *"the \"Where To File a Paper Form 4868\" table printed on FORM 4868 ITSELF — the last page of the f4868.pdf this command just wrote"* | `btctax_cli::WHERE_TO_FILE_4868_SOURCE` |
    /// | *"Form 1040-V's own second page prints a \"Mailing Address for Payments\" table"* | the packet manifest's paying arm |
    ///
    /// ★ The year set is **derived from the bundled templates**, never typed: whatever years
    /// `btctax-forms/forms/<year>/` ships a template for are the years checked. The 2026 package
    /// currently carries only `YEAR.toml`, so it is silently and correctly out of scope until its PDFs
    /// land — at which point this test demands their extracts too, which is the direction to fail in.
    ///
    /// Read from the committed text layers under `design/forms/extract/` rather than by shelling out
    /// to `pdftotext`, so the check runs in the suite and in CI's network-isolated job.
    #[test]
    fn every_bundled_year_of_the_two_forms_we_point_at_really_carries_its_address_table() {
        let root = repo_root();
        // (stem, the heading its own page must print)
        let pointed_at = [
            ("f4868", "Where To File a Paper Form 4868"),
            ("f1040v", "Mailing Address for Payments"),
        ];
        let mut checked = 0;
        for (stem, heading) in pointed_at {
            let years = std::fs::read_dir(root.join("crates/btctax-forms/forms"))
                .expect("the bundled forms directory must be readable")
                .flatten()
                .filter(|e| e.path().join(format!("{stem}.pdf")).is_file())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>();
            assert!(
                !years.is_empty(),
                "no bundled year ships a {stem}.pdf — either the template moved or this derivation \
                 reads the wrong directory, and a check that finds no years passes by finding nothing"
            );
            for year in years {
                let extract = root.join(format!("design/forms/extract/{stem}--{year}.txt"));
                let text = std::fs::read_to_string(&extract).unwrap_or_else(|e| {
                    panic!(
                        "{stem}.pdf is bundled for {year} but its committed text layer is missing \
                         ({}): btctax POINTS THE FILER at a table on that page, and nothing else \
                         checks the table is there. Run `cargo run -p xtask -- forms extract`: {e}",
                        extract.display()
                    )
                });
                assert!(
                    text.contains(heading),
                    "the {year} {stem} no longer prints its {heading:?} table, and the shipped \
                     guidance sends the filer to it. Re-read the revision and fix the pointer before \
                     a filer follows it to a page that does not exist."
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 4,
            "only {checked} (form, year) pairs were checked; TY2024 and TY2025 each bundle both \
             forms, so fewer than four means the derivation lost a year"
        );
    }
}
