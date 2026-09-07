//! ★★★ **THE PER-DOCUMENT, PER-EDITION BOX CENSUS** — every box an archived information return
//! PRINTS carries exactly one recorded decision, **in every edition the archive holds**.
//! (`design/SPEC_interview.md` r2 R4 + §5.2, fold finding I2; built by T2, widened to three tax
//! years by the T2 seam review's C1.)
//!
//! **Why this exists.** `CLAUDE.md`: *"a conformance KAT must enumerate the expected line set FROM the
//! form's extracted text, never from a range or a hand-written list"*, and *"blank because nothing
//! populated it"* is the defect that is invisible on the printed page. One level below the form line
//! sits the **document box**, and it has the same two-blanks problem: a 1099-INT box the interview
//! never asks for and a 1099-INT box the filer legitimately left empty are identical downstream. R4
//! states the rule for boxes exactly as the repo states it for lines:
//!
//! > **Every supported document carries a `[boxes]` census beside its `Field`s, ENUMERATED from the
//! > archived extract — the table above is a reading list, never the authority.** […] A caption in the
//! > extract with no entry **reds**; an entry naming a caption the extract does not have **reds**.
//!
//! ★★ **The census key is (edition, box LABEL) and the value is that box's own printed CAPTION**,
//! quoted verbatim from `design/forms/extract/<stem>--<edition>.txt`. So the three kills are one set
//! comparison per edition: a missing entry is a printed box nobody decided, an extra entry is a box
//! that edition does not print, and a caption changed by one character is both at once.
//!
//! ## ★★★ ONE EDITION PER DOCUMENT WAS THE CRITICAL — the archive is per REVISION
//!
//! T2 archived one edition per document and pinned it. The seam review measured what that hides: the
//! **Rev. December 2026 Form 1099-G prints a box 10 "Family leave benefits"** — an INCOME box — and
//! renumbers the state boxes `10a/10b/11 → 11a/11b/12`, while `box-census` printed *"115 printed
//! boxes … every one decided"* against the Rev. March 2024 grid. The instrument was green one layer
//! above a box nothing decided.
//!
//! The fix is not a newer pin. The interview must serve **TY2024** (the only computable year),
//! **TY2025** and **TY2026** (the target), and those filers hold **different paper**. So the archive
//! is per revision, and which revision governs a tax year is read off the rule the IRS prints on the
//! documents themselves (`design/forms/extract/f1098e--2026.txt:2-5`, *"Which Revision To Use for
//! Which Year"*):
//!
//! > **the year of the revision date is the first year for which issuers are to use the form to
//! > report amounts.**
//!
//! [`revision_in_force`] is that sentence, and nothing else: an **annual** edition governs its own
//! tax year; a **periodic** (*"Rev. Month Year"*) edition governs from its revision year until a
//! later revision displaces it; and a year no archived edition governs is `None`, never a nearest
//! guess.
//!
//! ## What T2 owns and what it does not (fold D7)
//!
//! > *"T2 owns the extract, so T2's kill is the **unentered-caption** red and T5's is the entries
//! > themselves."*
//!
//! So this module lands the **instrument** and a **populated fixture**: 123 entries covering 246
//! printed boxes across the 15 archived form editions, each naming either the struct field that
//! holds the box today, the refusal the box drives today, or the reason the return does not read it.
//! The entries deliberately do **not** yet join to `FieldId` or `RefuseReason` variants — those are
//! T5/T9's, and a join written before the variants exist would be a hand-list pretending to be a
//! check.
//!
//! ★ **An entry carries the EDITIONS its caption is printed in**, rather than being copied once per
//! edition. A duplicated row per edition would be ~250 hand-copies of the same decision text, and it
//! would *hide* the thing worth seeing: where an entry's edition list splits, the caption moved
//! between revisions. Form W-2 box 13 (`13 Statutory` → `13 employee`), Form 1099-B box 7 and the
//! whole 1099-G state block are visible as splits in the diff, and every listed edition still has its
//! caption checked against that edition's own extract.
//!
//! ## The enumerator, and why each rule is the document's rather than ours
//!
//! 1. **The face block.** Every information return's PDF opens with an *Attention* preamble and
//!    closes Copy A with a `Cat. No.` footer. The block between them is the form's own box grid.
//!    ★★ **The preamble's last sentence is PER EDITION and is recorded on the authority**, because
//!    it is revision-fragile: fourteen of the fifteen archived editions end it *"See Publications
//!    1141, 1167, and 1179 …"*, and the **2026 Form W-2 rewrote its preamble** to *"See IRS
//!    Publication 1141 …"* /
//!    *"See IRS Publication 1223 …"*. A single hard-coded marker would have hard-failed the
//!    enumerator on the newest W-2 the day it was archived. Each marker is asserted to occur exactly
//!    once in its own edition, so this stays a reading of the document rather than a fallback.
//! 2. **Runs.** `pdftotext -layout` separates columns by two or more spaces, so each line splits into
//!    runs; a run beginning `<label> <Capital>` is a caption, and a run that is *only* a label adopts
//!    the next run on its line unless that run is itself a label. That second rule is not a nicety:
//!    Form 1099-DIV prints `3    Nondividend distributions` with the label in its own column, and
//!    without it boxes **3, 4, 5 and 6 vanished** from a list that still looked plausible.
//! 3. **`OMB No.` ends a caption.** The Paperwork Reduction Act control number is page furniture on
//!    every IRS form and is never a box caption; on Form 1099-G the layout collapses the column gap to
//!    a single space (`1 Unemployment compensation OMB No. 1545-0120`) and it would otherwise be
//!    transcribed as part of box 1's caption. The marker is asserted present in every edition, so
//!    it is a reading of the documents rather than a taste.
//! 4. **The contiguity guard.** The numeric labels must run `1..=max` with no gap (a lettered box
//!    counts for its number), and lettered labels must run from `a`. This is the guard ON the
//!    enumerator, not the enumeration: it is what turns a blind spot into a red instead of a short
//!    list nobody counts. It has already earned itself twice — the first draft's run-splitter
//!    silently dropped four boxes of Form 1099-DIV, and rule 5 below exists because this guard red on
//!    a document it had never seen.
//! 5. **`For calendar year` marks the blank year stub, which is not a box.** The Rev. January 2022
//!    Form 1098 prints a bare `20` under *For calendar year* (the blank the issuer completes), and
//!    the enumerator read it as box 20 — caught, loudly, by rule 4 (*"not contiguous: 12..19 missing
//!    from 1..=20"*). The stub is the document's own furniture, named by the document's own words,
//!    exactly like `OMB No.`
//! 6. **A bare LETTERED run is the vertical `Code` rail, never a box.** Form W-2 prints the word
//!    *Code* down the side of box 12 and `pdftotext` emits `C`, `o`, `d`, `e` as separate runs. Every
//!    captionless box these forms print is NUMBERED (W-2 box 9 and 12b–12d), so a run that is only a
//!    letter is rail furniture. ★ This rule is what makes widening [`is_label`] past `f` safe: without
//!    it, `o` would enter the census and the letter-contiguity guard would demand `a..=o`.
//!
//! ★ **An empty caption is a real answer**, not a failure: the Form W-2 prints box `9` shaded with no
//! caption at all, and boxes `12b`/`12c`/`12d` as bare labels beside that *Code* rail. Recording them
//! with a reason is the point; dropping them because they carry no words is exactly the "we forgot
//! this box" defect.
//!
//! ## ★ The two limits this enumerator HAS, declared rather than discovered
//!
//! - **A caption the layout wraps is quoted to its first printed line** (1099-INT box 9 →
//!   `"9 Specified private activity bond"`, its second line *"interest"* sitting in another column's
//!   vertical run). The wrap is layout, not text; quoting across it would quote something the page
//!   never prints contiguously.
//! - **An UNLABELLED printed box is outside its reach.** [`printed_boxes`] keys on a label, so the
//!   1099-INT's *FATCA filing requirement* checkbox and its *2nd TIN not.* box — both printed on the
//!   face (`design/forms/extract/f1099int--2024.txt:47`, `:51`) — appear in no census. Neither
//!   reaches a 1040 line, so nothing is wrong today; it is declared here because the census's whole
//!   claim is *"every box the form PRINTS"*, and a limit nobody wrote down is a limit nobody can
//!   check.

use btctax_core::tax::return_refuse::RefuseReason;
use btctax_input_form::{FieldId, SectionId};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// The last sentence of the *Attention* preamble on every edition but one. The box grid begins on
/// the next line.
pub const PREAMBLE_1141: &str = "1141, 1167, and 1179";

/// ★ The 2026 Form W-2's preamble, which does NOT carry [`PREAMBLE_1141`]: it ends *"See IRS
/// Publication 1141 …"* / *"See IRS Publication 1223 for more information about printing substitute
/// Forms W-2c and W-3c."* (`design/forms/extract/fw2--2026.txt:26-28`).
pub const PREAMBLE_W2_2026: &str = "See IRS Publication 1223";

/// The Copy A footer. The box grid ends on the line before the first one.
const FACE_END: &str = "Cat. No.";

/// Page furniture that is never part of a box caption. See rule 3 above.
const FURNITURE: &str = "OMB No.";

/// The blank year the issuer completes on a continuous-use form. See rule 5 above.
const YEAR_STUB: &str = "For calendar year";

/// How a document's editions are named — and therefore how an edition maps to a tax year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    /// The face carries a TAX YEAR (`2025` beside *Form W-2 Wage and Tax Statement*). That edition
    /// is the one a filer for that year holds, and no other.
    Annual,
    /// The face carries a *"Rev. Month Year"*. The IRS prints the mapping rule itself: *"the year of
    /// the revision date is the first year for which issuers are to use the form to report
    /// amounts"*, so the edition governs from its revision year until a later revision displaces it.
    Periodic,
}

/// One archived EDITION of an information return.
///
/// ★★ **One entry per edition, not per document.** The archive holds three Forms W-2 (2024, 2025,
/// 2026) and two Forms 1099-G (Rev. 3-2024, Rev. 12-2026) because a TY2024 filer and a TY2026 filer
/// hold different paper — and the 1099-G proves the difference is not cosmetic.
pub struct DocumentAuthority {
    /// The IRS stem, e.g. `"f1099int"` — also the `design/forms/extract/` filename stem.
    pub stem: &'static str,
    /// The archive's filename year: `design/forms/<edition>/<stem>--<edition>.pdf`.
    pub edition: &'static str,
    /// The revision year **read off the document's own text** and recorded in its `.pdf.txt` note —
    /// the year on an annual face, the *Rev.* year on a periodic one.
    pub revision_year: u32,
    pub cadence: Cadence,
    /// The identically-numbered instructions booklet — **except the 1098-E**, whose instructions are
    /// the combined 1098-E/1098-T booklet `i1098et`; there is no `i1098e` document. The W-2's are
    /// `iw2w3`, and the 1099-INT's booklet is shared with the 1099-OID. ★ The booklet's own EDITION
    /// is resolved per tax year by [`revision_in_force`], because a booklet can be revised when its
    /// form is not: `i1098--2026` is Rev. December 2026 while the form is still Rev. April 2025.
    pub instructions: &'static str,
    /// This edition's own preamble-ending sentence. See enumerator rule 1.
    pub preamble_end: &'static str,
}

/// One archived EDITION of an instructions booklet, so [`revision_in_force`] can answer for an
/// `i…` stem too.
pub struct BookletEdition {
    pub stem: &'static str,
    pub edition: &'static str,
    pub revision_year: u32,
    pub cadence: Cadence,
}

/// ★★ **The 15 archived FORM editions.** Derived-and-asserted, not merely written down:
/// `documents_equal_the_archived_information_returns` requires this list to equal, in both
/// directions, the `MANIFEST.json` `kind: form` entries whose stem is in the W / 1098 / 1099 series.
/// Dropping a document — the seam review dropped `f1098e` and watched all six census tests pass —
/// now reds.
pub const DOCUMENTS: &[DocumentAuthority] = &[
    DocumentAuthority {
        stem: "fw2",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Annual,
        instructions: "iw2w3",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "fw2",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Annual,
        instructions: "iw2w3",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "fw2",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Annual,
        instructions: "iw2w3",
        preamble_end: PREAMBLE_W2_2026,
    },
    DocumentAuthority {
        stem: "f1099int",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Periodic,
        instructions: "i1099int",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1099div",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Periodic,
        instructions: "i1099div",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1099g",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Periodic,
        instructions: "i1099g",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1099g",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Periodic,
        instructions: "i1099g",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1099b",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Annual,
        instructions: "i1099b",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1099b",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Annual,
        instructions: "i1099b",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1099b",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Annual,
        instructions: "i1099b",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1098",
        edition: "2022",
        revision_year: 2022,
        cadence: Cadence::Periodic,
        instructions: "i1098",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1098",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Periodic,
        instructions: "i1098",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1098e",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Annual,
        instructions: "i1098et",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1098e",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Annual,
        instructions: "i1098et",
        preamble_end: PREAMBLE_1141,
    },
    DocumentAuthority {
        stem: "f1098e",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Annual,
        instructions: "i1098et",
        preamble_end: PREAMBLE_1141,
    },
];

/// ★★ **The 16 archived BOOKLET editions**, asserted against `MANIFEST.json` `kind: instructions`
/// the same way [`DOCUMENTS`] is.
pub const BOOKLETS: &[BookletEdition] = &[
    BookletEdition {
        stem: "iw2w3",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "iw2w3",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "iw2w3",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "i1099int",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1099div",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1099g",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1099g",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1099b",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "i1099b",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "i1099b",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "i1098",
        edition: "2022",
        revision_year: 2022,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1098",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1098",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Periodic,
    },
    BookletEdition {
        stem: "i1098et",
        edition: "2024",
        revision_year: 2024,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "i1098et",
        edition: "2025",
        revision_year: 2025,
        cadence: Cadence::Annual,
    },
    BookletEdition {
        stem: "i1098et",
        edition: "2026",
        revision_year: 2026,
        cadence: Cadence::Annual,
    },
];

/// ★★★ **WHICH ARCHIVED EDITION GOVERNS A TAX YEAR — the IRS's own rule, and nothing else.**
///
/// *"the year of the revision date is the first year for which issuers are to use the form to report
/// amounts"* (`design/forms/extract/f1098e--2026.txt:2-5`). So:
///
/// - **annual** — the edition whose face year IS the tax year, and no other. A TY2024 filer holds
///   the 2024 Form W-2; the 2025 edition says nothing about their return.
/// - **periodic** — the archived revision with the greatest revision year `<=` the tax year. Form
///   1099-G Rev. March 2024 governs TY2024 *and* TY2025; Rev. December 2026 takes over for TY2026.
///
/// ★ `None` when nothing archived governs that year, which is a real answer and a load-bearing one:
/// a `Collected` row whose tax year has no governing edition **reds** in `xtask line-coverage`
/// rather than being checked against whichever edition happened to be pinned.
#[must_use]
pub fn revision_in_force(stem: &str, tax_year: u32) -> Option<&'static str> {
    let forms = DOCUMENTS
        .iter()
        .filter(|d| d.stem == stem)
        .map(|d| (d.edition, d.revision_year, d.cadence));
    let booklets = BOOKLETS
        .iter()
        .filter(|b| b.stem == stem)
        .map(|b| (b.edition, b.revision_year, b.cadence));
    forms
        .chain(booklets)
        .filter(|(_, rev, cadence)| match cadence {
            Cadence::Annual => *rev == tax_year,
            Cadence::Periodic => *rev <= tax_year,
        })
        .max_by_key(|(_, rev, _)| *rev)
        .map(|(edition, _, _)| edition)
}

/// What the return does with one printed box. Exactly one per caption — R4's *"`collected(FieldId)`,
/// `refuse_if_nonzero(RefuseReason)` … or `not_read(reason)`"*.
///
/// ★★★ **T5 JOINED IT TO THE REGISTRY.** T2 landed the instrument with the decisions as PROSE —
/// `Collected("Form1099Int.box1_interest → …")` — and said so in terms: *"the entries deliberately
/// do not yet join to `FieldId` or `RefuseReason` variants — those are T5/T9's, and a join written
/// before the variants exist would be a hand-list pretending to be a check."* T5 built the sections,
/// so the variants exist and the join is now the real thing:
///
/// - a [`FieldId`] that does not exist **does not compile**;
/// - a `FieldId` that is not a `Field` of that document's own section **reds**
///   ([`tests::every_collected_box_names_a_field_of_its_own_section`]);
/// - a [`RefuseReason`] variant that is deleted or renamed **does not compile**;
/// - and the box's own printed CAPTION must appear in the field's label or help, so a box whose
///   wording moved between revisions cannot keep pointing at a field that says something else.
///
/// ★★★ **ONE RULE FOR AN INCOME BOX WITH NO READER, and it is stated here rather than re-argued per
/// box.** *An income box with no reader UNDERSTATES, so it fails closed* — if a box carries an amount
/// the Form 1040 chain calls income, and no `Field` holds it and no printed line reads it, the entry
/// is [`Self::RefuseIfNonzero`] naming the line it should have reached, never [`Self::NotRead`].
///
/// FR-65 decided 1099-G box 10 by that sentence and then four boxes on the same form and three on two
/// others still said `NotRead` on verbatim the same argument (seam review M-1). One census may not
/// answer one question two ways, so the seven were flipped together: **1099-G boxes 5, 6, 7 and 9,
/// 1099-B box 13, and 1099-DIV boxes 9 and 10.**
///
/// ★ What stays `NotRead` is everything that is NOT income with no reader, and the distinction is the
/// point of the rule rather than an exception to it:
/// - **identifiers and codes** — a state's two-letter code, a payer's state ID, a CUSIP, a country
///   name, a control number, a FATCA checkbox: no amount, so nothing to understate;
/// - **state and local figures** — no federal line reads a state wage or a state withholding;
/// - **expenses, not income** — 1099-INT box 5 / 1099-DIV box 6 investment expenses, suspended by
///   §67(g);
/// - **an amount already INSIDE a collected box** — W-2 box 11 (in box 1), 1099-DIV boxes 2e/2f (in
///   1a/2a for a U.S. filer): the reader exists, it is the box that contains it;
/// - **a basis adjustment that reaches no line THIS year** — 1099-DIV box 3 nondividend
///   distributions. ★ It looks exactly like boxes 9/10 and is not the same: a liquidating
///   distribution IS a disposition in the year received, box 3 is a return of capital that only
///   changes basis;
/// - **withholding** — 1099-B box 4, 1099-INT box 17, 1099-DIV box 16, 1099-G boxes 11/12: a forgone
///   CREDIT overstates the tax, the direction §3.4 permits silently, so it is not this class.
///
/// ★ The prose survives as `note`, because the field name alone does not say WHICH LINE the box
///   reaches, and that sentence is the whole reason a reader can audit the table.
/// ★ NO `PartialEq`. [`Self::RefuseIfNonzero`] carries a `fn` pointer, and Rust's own
/// `unpredictable_function_pointer_comparisons` lint says why comparing one is meaningless: two
/// distinct constructors may be merged to the same address, and one constructor may have several.
/// Nothing here needs decision equality — the census compares LABELS and CAPTIONS — so the type
/// simply does not offer an operation that cannot be trusted.
#[derive(Debug, Clone, Copy)]
pub enum BoxDecision {
    /// One or more `Field`s of **this document's own section** hold the box, and its figure reaches
    /// a printed line.
    Collected {
        fields: &'static [FieldId],
        note: &'static str,
    },
    /// The box IS collected, but by `Field`s **outside** this document's row — the 1040 header
    /// prints the employee's SSN once for the return rather than once per W-2, the W-2's box-12
    /// slots are their own nested section, and Form 1098's box 1 is still a Schedule A leaf until
    /// T9. Every id must still be a real `Field`, and NONE may belong to this document's own
    /// section (or the entry should have been [`Self::Collected`]).
    CollectedElsewhere {
        fields: &'static [FieldId],
        note: &'static str,
    },
    /// A `Field` of this document's section holds the box **only in order to refuse on it** —
    /// nothing it carries reaches a line. `reason` is a constructor rather than a value because a
    /// `&'static [BoxEntry]` cannot hold a type with drop glue; it is still the real variant, so
    /// deleting it fails to compile.
    RefuseIfNonzero {
        field: FieldId,
        reason: fn() -> RefuseReason,
        note: &'static str,
    },
    /// No field holds it. The string is the reason, and it is the whole value of the entry: it is what
    /// distinguishes *"the return has no use for this box"* from *"we forgot this box"*.
    NotRead(&'static str),
}

/// The `FormSpec` section that holds one document's ROWS, or `None` for a document with no section
/// of its own yet.
///
/// ★ DERIVED from the stem, and exhaustive over the archived stems: a document added to
/// [`DOCUMENTS`] with no arm here reds in [`tests::every_stem_maps_to_a_section_or_says_why`]
/// rather than silently landing in the "no section" bucket.
#[must_use]
pub fn section_of_stem(stem: &str) -> Option<SectionId> {
    match stem {
        "fw2" => Some(SectionId::W2s),
        "f1099int" => Some(SectionId::Int1099s),
        "f1099div" => Some(SectionId::Div1099s),
        "f1099b" => Some(SectionId::B1099s),
        "f1099g" => Some(SectionId::G1099s),
        "f1098e" => Some(SectionId::Form1098Es),
        // ★ Form 1098 has no section until **T9**: its one collected box is still the Schedule A
        //   scalar `mortgage_interest_1098`, so every one of its entries is `CollectedElsewhere` or
        //   `NotRead`.
        "f1098" => None,
        _ => None,
    }
}

/// One printed box, its caption as the form prints it, and the decision.
pub struct BoxEntry {
    pub stem: &'static str,
    /// **The editions this caption is printed in.** An entry is in scope for an edition iff the
    /// edition is listed here, so a box that appears (1099-G box 10), disappears (1099-G box 10a) or
    /// is re-worded between revisions is a visible split rather than a silent overwrite.
    pub editions: &'static [&'static str],
    /// The box's own label — `"1"`, `"2a"`, `"12b"`, `"a"`.
    pub label: &'static str,
    /// **VERBATIM** from `design/forms/extract/<stem>--<edition>.txt`, label included, as the extract
    /// prints it on the label's own line. A caption the layout wraps is quoted to its first printed
    /// line, because the wrap is layout and quoting across it would quote text the page never prints
    /// contiguously.
    pub caption: &'static str,
    pub decision: BoxDecision,
}

/// ★★ **THE POPULATION — 123 entries covering 246 printed boxes across 15 archived form editions,
/// every caption read off that edition's own extract.**
///
/// The `Collected` strings name fields that exist at T2 (`crates/btctax-core/src/tax/return_inputs.rs`);
/// a `NotRead` whose reason begins *"T5"* / *"T9"* is a box a later task collects, and its reason names
/// the line it will reach so the schedule is legible rather than implied.
///
/// ★★★ **Two boxes here were invisible until the archive went per-revision**, and both are the
/// review's C1 made concrete: Form 1099-G box 10 *Family leave benefits* (an INCOME box the
/// Rev. December 2026 grid added, which now `RefuseIfNonzero`s rather than passing as decided), and
/// Form W-2 box 14b *Treasury Tipped Occupation Code(s)*, which the 2026 revision added beside the
/// old box 14 and which Schedule 1-A line 4a's tips deduction turns on.
pub const BOXES: &[BoxEntry] = &[
    // ── Form W-2 — 2024, 2025 and 2026 (29 / 29 / 30 printed boxes) ─────────────────────────────────
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "a", caption: "a Employee’s social security number",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::TpSsn, FieldId::SpSsn], note: "header.taxpayer.ssn / header.spouse.ssn — the 1040 header prints it once for the return, not per W-2 row" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "b", caption: "b Employer identification number (EIN)",
        decision: BoxDecision::Collected { fields: &[FieldId::W2Ein], note: "W2.ein — the only thing that can answer §6413(c)'s 'more than one employer' test" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "c", caption: "c Employer’s name, address, and ZIP code",
        decision: BoxDecision::Collected { fields: &[FieldId::W2Employer], note: "W2.employer" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "d", caption: "d Control number",
        decision: BoxDecision::NotRead("the employer's internal payroll number; no line of the return reads it") },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "e", caption: "e Employee’s first name and initial",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::TpFirstName, FieldId::TpLastName, FieldId::SpFirstName, FieldId::SpLastName], note: "header.taxpayer.name / header.spouse.name — printed once on the 1040 header" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "f", caption: "f Employee’s address and ZIP code",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::AddrStreet, FieldId::AddrCity, FieldId::AddrState, FieldId::AddrZip], note: "header.address_street / address_city / address_state / address_zip" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "1", caption: "1 Wages, tips, other compensation",
        decision: BoxDecision::Collected { fields: &[FieldId::Box1Wages], note: "W2.box1_wages → 1040 line 1a" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "2", caption: "2 Federal income tax withheld",
        decision: BoxDecision::Collected { fields: &[FieldId::Box2FedWh], note: "W2.box2_fed_withheld → 1040 line 25a" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "3", caption: "3 Social security wages",
        decision: BoxDecision::Collected { fields: &[FieldId::Box3SsWages], note: "W2.box3_ss_wages → the per-earner SS cap and the excess-SS credit" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "4", caption: "4 Social security tax withheld",
        decision: BoxDecision::Collected { fields: &[FieldId::Box4SsWh], note: "W2.box4_ss_withheld → Schedule 3 line 11, excess social security" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "5", caption: "5 Medicare wages and tips",
        decision: BoxDecision::Collected { fields: &[FieldId::Box5MedWages], note: "W2.box5_medicare_wages → Form 8959 Part I" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "6", caption: "6 Medicare tax withheld",
        decision: BoxDecision::Collected { fields: &[FieldId::Box6MedWh], note: "W2.box6_medicare_withheld → Form 8959 Part V → 1040 line 25c" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "7", caption: "7 Social security tips",
        decision: BoxDecision::Collected { fields: &[FieldId::Box7SsTips], note: "W2.box7_ss_tips → the §6413(c) wage total, and Schedule 1-A line 4a's tips" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "8", caption: "8 Allocated tips",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Box8AllocTips, reason: || RefuseReason::AllocatedTips, note: "W2.box8_allocated_tips — allocated tips are unreported income needing Form 4137; > 0 refuses" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "9", caption: "9",
        decision: BoxDecision::NotRead("the 2025 revision prints box 9 shaded and CAPTIONLESS — there is no figure to collect; the box exists on the paper and is recorded so the census cannot silently gain a caption in a later revision") },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "10", caption: "10 Dependent care benefits",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Box10DepCare, reason: || RefuseReason::DependentCareBenefit, note: "W2.box10_dependent_care — dependent care benefits need Form 2441; > 0 refuses" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "11", caption: "11 Nonqualified plans",
        decision: BoxDecision::NotRead("distributions from a nonqualified deferred compensation plan, already included in box 1 for income tax; the SSA reads it, no 1040 line does") },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "12a", caption: "12a See instructions for box 12",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::Box12Code, FieldId::Box12Amount], note: "W2.box12 — a Vec<Box12Entry> of (code, amount), so all four printed slots are one repeating field" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "12b", caption: "12b",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::Box12Code, FieldId::Box12Amount], note: "W2.box12 — the second of the form's four printed slots; the label prints bare beside a vertical 'Code' rail" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "12c", caption: "12c",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::Box12Code, FieldId::Box12Amount], note: "W2.box12 — the third printed slot" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "12d", caption: "12d",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::Box12Code, FieldId::Box12Amount], note: "W2.box12 — the fourth printed slot" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025"], label: "13", caption: "13 Statutory",
        decision: BoxDecision::Collected { fields: &[FieldId::W2Box13StatutoryEmployee],
            note: "W2.box13_statutory_employee — the 'Statutory employee' checkbox. A checked box 13 sends box 1 to SCHEDULE C LINE 1, not to 1040 line 1a, so `true` refuses StatutoryEmployeeW2 rather than file the wages on the wrong line. ★ The other two checkboxes printed under the same label — 'Retirement plan' and 'Third-party sick pay' — reach no line btctax computes: the first qualifies the §219(g) IRA phase-out, and a claimed IRA deduction refuses outright (IraDeductionClaimed); the second is already inside box 1" } },
    BoxEntry { stem: "fw2", editions: &["2026"], label: "13", caption: "13 employee",
        decision: BoxDecision::Collected { fields: &[FieldId::W2Box13StatutoryEmployee],
            note: "W2.box13_statutory_employee — the same checkbox as the 2024/2025 grid, whose caption the 2026 layout wraps one word later ('13 Statutory' / '13 employee'). The split is visible here precisely so the wrap cannot be mistaken for a new box" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025"], label: "14", caption: "14 Other",
        decision: BoxDecision::NotRead("free-text employer reporting; nothing reaches a line until the filer identifies the item, and the residual scope attestation covers what they cannot") },
    BoxEntry { stem: "fw2", editions: &["2026"], label: "14a", caption: "14a Other",
        decision: BoxDecision::NotRead("free-text employer reporting; nothing reaches a line until the filer identifies the item, and the residual scope attestation covers what they cannot") },
    BoxEntry { stem: "fw2", editions: &["2026"], label: "14b", caption: "14b Treasury Tipped Occupation Code(s)",
        decision: BoxDecision::Collected { fields: &[FieldId::W2Box14bTtoc],
            note: "W2.box14b_treasury_tipped_occupation_codes — the Treasury Tipped Occupation Code(s) the 2026 revision added beside box 14a. It is the employer's statement of the occupation the tips were earned in, which is exactly what Schedule 1-A Part II's own Caution turns on: 'These tips must have been received in an occupation listed at IRS.gov/TippedOccupations.' A CODE, never an amount, so it reaches no line by arithmetic — its reader is Advisory::TipsDeductionForgoneWithTtoc, which fires when a W-2 carries a code and the return claims no qualified tips. The overstatement direction, so §3.4 makes it an advisory and never a refusal" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "15", caption: "15 State",
        decision: BoxDecision::NotRead("the state's two-letter code and the employer's state ID number; the federal return prints neither") },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "16", caption: "16 State wages, tips, etc.",
        decision: BoxDecision::NotRead("state wages; no federal line reads a state wage figure") },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "17", caption: "17 State income tax",
        decision: BoxDecision::Collected { fields: &[FieldId::Box17StateWh], note: "W2.box17_state_tax_withheld → Schedule A line 5a on the income-tax election" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "18", caption: "18 Local wages, tips, etc.",
        decision: BoxDecision::NotRead("local wages; no federal line reads a local wage figure") },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "19", caption: "19 Local income tax",
        decision: BoxDecision::Collected { fields: &[FieldId::Box19LocalTax], note: "W2.box19_local_tax → Schedule A line 5a" } },
    BoxEntry { stem: "fw2", editions: &["2024", "2025", "2026"], label: "20", caption: "20 Locality name",
        decision: BoxDecision::NotRead("the locality's name; Schedule A line 5a takes the amount, never the locality") },
    // ── Form 1099-INT (Rev. January 2024) — 17 boxes; still in force for TY2026 ─────────────────────
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "1", caption: "1 Interest income",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box1Interest], note: "Form1099Int.box1_interest → Schedule B line 1 → 1040 line 2b" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "2", caption: "2 Early withdrawal penalty",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box2EarlyWithdrawal], note: "Form1099Int.box2_early_withdrawal_penalty → Schedule 1 line 18" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "3", caption: "3 Interest on U.S. Savings Bonds and Treasury obligations",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box3Treasury], note: "Form1099Int.box3_treasury_interest → 1040 line 2b" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box4FedWithheld], note: "Form1099Int.box4_fed_withheld → 1040 line 25b" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "5", caption: "5 Investment expenses",
        decision: BoxDecision::NotRead("a miscellaneous itemized deduction, suspended for 2018–2025 by §67(g); no Schedule A line takes it") },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "6", caption: "6 Foreign tax paid",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box6ForeignTax], note: "Form1099Int.box6_foreign_tax → the §904(j) foreign tax credit election on Schedule 3 line 1" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "7", caption: "7 Foreign country or U.S. territory",
        decision: BoxDecision::NotRead("the country's name; the §904(j) election reads the AMOUNT in box 6 and Form 1116 — which would read the country — is out of scope") },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "8", caption: "8 Tax-exempt interest",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box8TaxExempt], note: "Form1099Int.box8_tax_exempt_interest → 1040 line 2a" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "9", caption: "9 Specified private activity bond",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Int1099Box9PrivateActivity, reason: || RefuseReason::PrivateActivityBondAmt, note: "Form1099Int.box9_private_activity_bond_amt — a Form 6251 line 2g AMT preference; > 0 refuses" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "10", caption: "10 Market discount",
        decision: BoxDecision::Collected { fields: &[FieldId::Int1099Box10MarketDiscount],
            note: "Form1099Int.box10_market_discount → Schedule B line 1 and the 1040 line 2b sum — i1040sb, 'Also include any accrued market discount that is includible in income'. Income, so omitting it UNDERSTATES" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "11", caption: "11 Bond premium",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Int1099Box11BondPremium, reason: || RefuseReason::AmortizableBondPremiumNotComputed,
            note: "Form1099Int.box11_bond_premium — §171 amortizable bond premium REDUCES reported interest through a named Schedule B line-1 adjustment (Pub. 550), which btctax does not compute. A reduction, so refusing is both the conservative and the honest direction: dropping it would report more interest than the filer owes tax on" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "12", caption: "12 Bond premium on Treasury obligations",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Int1099Box12BondPremiumTreasury, reason: || RefuseReason::AmortizableBondPremiumNotComputed,
            note: "Form1099Int.box12_bond_premium_treasury — as box 11: a §171 reduction btctax does not compute, so any amount refuses" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "13", caption: "13 Bond premium on tax-exempt bond",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Int1099Box13BondPremiumTaxExempt, reason: || RefuseReason::AmortizableBondPremiumNotComputed,
            note: "Form1099Int.box13_bond_premium_tax_exempt — as box 11: a §171 reduction btctax does not compute, so any amount refuses" } },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "14", caption: "14 Tax-exempt and tax credit",
        decision: BoxDecision::NotRead("the tax-exempt and tax credit bond CUSIP number — an identifier, not an amount") },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "15", caption: "15 State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none") },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "16", caption: "16 State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099int", editions: &["2024"], label: "17", caption: "17 State tax withheld",
        decision: BoxDecision::NotRead("state income tax the payer withheld; Schedule A line 5a is collected from the W-2 and the filer's records, and no Form1099Int field holds this box") },
    // ── Form 1099-DIV (Rev. January 2024) — 22 boxes; still in force for TY2026 ─────────────────────
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "1a", caption: "1a Total ordinary dividends",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box1aOrdinary], note: "Form1099Div.box1a_ordinary → 1040 line 3b (it INCLUDES box 1b)" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "1b", caption: "1b Qualified dividends",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box1bQualified], note: "Form1099Div.box1b_qualified → 1040 line 3a, the preferential-rate slice" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "2a", caption: "2a Total capital gain distr.",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box2aCapGain], note: "Form1099Div.box2a_capgain_distr → Schedule D line 13" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "2b", caption: "2b Unrecap. Sec. 1250 gain",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Div1099Box2bUnrecap1250, reason: || RefuseReason::UnrecapturedOrSpecialRateGain, note: "Form1099Div.box2b_unrecap_1250 — the 25% rate group needs the Schedule D unrecaptured-gain worksheet; > 0 refuses" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "2c", caption: "2c Section 1202 gain",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Div1099Box2cSection1202, reason: || RefuseReason::UnrecapturedOrSpecialRateGain, note: "Form1099Div.box2c_section_1202 — qualified small business stock exclusion; > 0 refuses" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "2d", caption: "2d Collectibles (28%) gain",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Div1099Box2dCollectibles, reason: || RefuseReason::UnrecapturedOrSpecialRateGain, note: "Form1099Div.box2d_collectibles_28 — the 28% rate group; > 0 refuses" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "2e", caption: "2e Section 897 ordinary dividends",
        decision: BoxDecision::NotRead("§897 (FIRPTA) reporting, which the instructions address to foreign persons; for a U.S. filer the amount is already inside box 1a and reaches no line of its own") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "2f", caption: "2f Section 897 capital gain",
        decision: BoxDecision::NotRead("§897 (FIRPTA) reporting for foreign persons; already inside box 2a for a U.S. filer") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "3", caption: "3 Nondividend distributions",
        decision: BoxDecision::NotRead("R4's decision: it reduces basis and does not reach a line this year; Pub. 550") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box4FedWithheld], note: "Form1099Div.box4_fed_withheld → 1040 line 25b" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "5", caption: "5 Section 199A dividends",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box5Section199a], note: "Form1099Div.box5_section_199a → the QBI deduction (Form 8995 line 6)" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "6", caption: "6 Investment expenses",
        decision: BoxDecision::NotRead("a miscellaneous itemized deduction, suspended for 2018–2025 by §67(g)") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "7", caption: "7 Foreign tax paid",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box7ForeignTax], note: "Form1099Div.box7_foreign_tax → the §904(j) foreign tax credit election on Schedule 3 line 1" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "8", caption: "8 Foreign country or U.S. possession",
        decision: BoxDecision::NotRead("the country's name; the §904(j) election reads the AMOUNT in box 7") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "9", caption: "9 Cash liquidation distributions",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Div1099Box9CashLiquidation, reason: || RefuseReason::LiquidationDistributionNotComputed("Form 1099-DIV box 9 (cash liquidation distributions)".to_string()), note: "a liquidating distribution is treated as full payment in EXCHANGE for the stock — a Form 8949 / Schedule D disposition in the year received, not a dividend. btctax holds no basis for that stock and builds no Form 8949 row for it, so the gain has no reader: > 0 REFUSES. ★ Distinct from box 3, which stays NotRead because it is a BASIS ADJUSTMENT reaching no line this year"} },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "10", caption: "10 Noncash liquidation distributions",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Div1099Box10NoncashLiquidation, reason: || RefuseReason::LiquidationDistributionNotComputed("Form 1099-DIV box 10 (noncash liquidation distributions)".to_string()), note: "as box 9, paid in kind rather than in cash — the same exchange treatment, the same missing basis, so > 0 REFUSES"} },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "11", caption: "11 FATCA filing",
        decision: BoxDecision::NotRead("the FATCA filing requirement checkbox — a chapter 4 obligation of the PAYER; no line of the filer's return reads it") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "12", caption: "12 Exempt-interest dividends",
        decision: BoxDecision::Collected { fields: &[FieldId::Div1099Box12ExemptInterest], note: "Form1099Div.box12_exempt_interest_dividends → 1040 line 2a" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "13", caption: "13 Specified private activity",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::Div1099Box13PrivateActivity, reason: || RefuseReason::PrivateActivityBondAmt, note: "Form1099Div.box13_private_activity_amt — specified private activity bond interest dividends, a Form 6251 AMT preference; > 0 refuses" } },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "14", caption: "14 State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "15", caption: "15 State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099div", editions: &["2024"], label: "16", caption: "16 State tax withheld",
        decision: BoxDecision::NotRead("state income tax the payer withheld; no Form1099Div field holds it") },
    // ── Form 1099-G — Rev. March 2024 (12 boxes) and Rev. December 2026 (13: box 10 is NEW) ─────────
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "1", caption: "1 Unemployment compensation",
        decision: BoxDecision::Collected { fields: &[FieldId::G1099Box1Unemployment], note: "Form1099G.box1_unemployment → Schedule 1 line 7" } },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "2", caption: "2 State or local income tax",
        decision: BoxDecision::Collected { fields: &[FieldId::G1099Box2StateRefund],
            note: "Form1099G.box2_state_refund → Schedule 1 line 1, through the RETURN-LEVEL itemized_prior_year gate (R3/I1 — a gate may not ride on a row that might not exist). §111(a): No ⇒ no tax benefit ⇒ line 1 blank BY DECISION; Yes ⇒ refuse naming the State and Local Income Tax Refund Worksheet, which btctax does not compute" } },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "3", caption: "3 Box 2 amount is for tax year",
        decision: BoxDecision::NotRead("the tax year box 2's refund relates to; it is read by the State and Local Income Tax Refund Worksheet, which is not transcribed (owner Q1)") },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::Collected { fields: &[FieldId::G1099Box4FedWithheld], note: "Form1099G.box4_fed_withheld → 1040 line 25b" } },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "5", caption: "5 RTAA payments",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::G1099Box5Rtaa, reason: || RefuseReason::OtherIncomeLine8zNotModeled("Form 1099-G box 5 (RTAA payments)".to_string()), note: "Reemployment Trade Adjustment Assistance — INCOME whose only home is Schedule 1 line 8z, for which btctax models no inflow, so > 0 REFUSES. (It said NotRead until the seam review's M-1: box 10 six lines down refused on verbatim this argument, and one census may not answer one question two ways)"} },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "6", caption: "6 Taxable grants",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::G1099Box6TaxableGrants, reason: || RefuseReason::OtherIncomeLine8zNotModeled("Form 1099-G box 6 (taxable grants)".to_string()), note: "a taxable grant is INCOME reaching Schedule 1 line 8z, which btctax fills from nothing, so > 0 REFUSES (seam review M-1)"} },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "7", caption: "7 Agriculture payments",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::G1099Box7Agriculture, reason: || RefuseReason::ScheduleFIncomeNotModeled("Form 1099-G box 7 (agriculture payments)".to_string()), note: "agriculture program payments are Schedule F INCOME, an excluded family (§2.2). ★ The document census announces that exclusion on a FARM document; this figure arrives on a 1099-G, which btctax admits, so nothing announces it — > 0 REFUSES (seam review M-1)"} },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "8", caption: "8 Check if box 2 is",
        decision: BoxDecision::NotRead("the checkbox saying box 2's refund is of a tax on TRADE OR BUSINESS income. It qualifies box 2 — which btctax collects — but the qualification only matters to a filer with a Schedule C whose state tax was a business expense, and btctax takes Schedule C expenses as a flat total it never itemizes. The §111(a) gate reads the AMOUNT in box 2, never this box") },
    BoxEntry { stem: "f1099g", editions: &["2024", "2026"], label: "9", caption: "9 Market gain",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::G1099Box9MarketGain, reason: || RefuseReason::ScheduleFIncomeNotModeled("Form 1099-G box 9 (market gain on a CCC loan)".to_string()), note: "gain on the repayment of a Commodity Credit Corporation loan — Schedule F INCOME on an admitted document, as box 7, so > 0 REFUSES (seam review M-1)"} },
    BoxEntry { stem: "f1099g", editions: &["2026"], label: "10", caption: "10 Family leave benefits",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::G1099Box10FamilyLeave, reason: || RefuseReason::FamilyLeaveBenefits, note: "T5: paid family leave benefits — an INCOME box the Rev. December 2026 grid added, reportable on Schedule 1. No field holds it and no line reads it, so > 0 REFUSES until T5 decides the line: an income box with no reader understates, and this fails closed instead" } },
    BoxEntry { stem: "f1099g", editions: &["2024"], label: "10a", caption: "10a State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none") },
    BoxEntry { stem: "f1099g", editions: &["2024"], label: "10b", caption: "10b State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099g", editions: &["2024"], label: "11", caption: "11 State income tax withheld",
        decision: BoxDecision::NotRead("state income tax withheld; no Form1099G field holds it") },
    BoxEntry { stem: "f1099g", editions: &["2026"], label: "11a", caption: "11a State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none. Numbered 10a before the Rev. December 2026 revision") },
    BoxEntry { stem: "f1099g", editions: &["2026"], label: "11b", caption: "11b State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none. Numbered 10b before the Rev. December 2026 revision") },
    BoxEntry { stem: "f1099g", editions: &["2026"], label: "12", caption: "12 State income tax withheld",
        decision: BoxDecision::NotRead("state income tax withheld; no Form1099G field holds it. Numbered 11 before the Rev. December 2026 revision") },
    // ── Form 1099-B — 2024, 2025 and 2026 (22 boxes each) ───────────────────────────────────────────
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1a", caption: "1a Description of property (Example: 100 sh. XYZ Co.)",
        decision: BoxDecision::NotRead("btctax takes the Schedule D line 1a/8a TOTALS, which the form's own instruction permits when basis was reported and there are no adjustments; a per-transaction description belongs on Form 8949, and a row needing one is refused (Form1099BNeedsForm8949)") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1b", caption: "1b Date acquired",
        decision: BoxDecision::NotRead("per-transaction, and Schedule D lines 1a/8a carry no dates; a row needing Form 8949 is refused") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1c", caption: "1c Date sold or disposed",
        decision: BoxDecision::NotRead("per-transaction, and Schedule D lines 1a/8a carry no dates; a row needing Form 8949 is refused") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1d", caption: "1d Proceeds",
        decision: BoxDecision::Collected { fields: &[FieldId::B1099ShortTermProceeds, FieldId::B1099LongTermProceeds], note: "Form1099B.short_term_proceeds / long_term_proceeds → Schedule D line 1a(d) / 8a(d)" } },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1e", caption: "1e Cost or other basis",
        decision: BoxDecision::Collected { fields: &[FieldId::B1099ShortTermBasis, FieldId::B1099LongTermBasis], note: "Form1099B.short_term_basis / long_term_basis → Schedule D line 1a(e) / 8a(e)" } },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1f", caption: "1f Accrued market discount",
        decision: BoxDecision::NotRead("an ADJUSTMENT: a row carrying one fails basis_reported_and_no_adjustments, so the gate refuses Form1099BNeedsForm8949 rather than dropping the figure") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "1g", caption: "1g Wash sale loss disallowed",
        decision: BoxDecision::NotRead("an ADJUSTMENT: the row fails the 1a/8a gate and is refused to Form 8949") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "2", caption: "2 Short-term gain or loss",
        decision: BoxDecision::Collected { fields: &[FieldId::B1099ShortTermProceeds, FieldId::B1099ShortTermBasis, FieldId::B1099LongTermProceeds, FieldId::B1099LongTermBasis], note: "Form1099B's short_term_* vs long_term_* pair — the box decides WHICH Schedule D lines a row's totals reach" } },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "3", caption: "3 Check if proceeds from:",
        decision: BoxDecision::NotRead("collectibles or QOF proceeds; either is an adjustment case the gate refuses to Form 8949") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::NotRead("backup withholding on broker proceeds would reach 1040 line 25b; no Form1099B field holds it, so a filer with backup withholding forgoes a credit — the OVERSTATEMENT direction, which §3.4 permits SILENTLY. ★ This reason used to claim the forgone credit was 'announced rather than silent'; the seam review's M-1 measured the only announcement (the unconditional OtherCreditsOmitted advisory, which names nothing specific) and the claim was withdrawn rather than the decision changed — §3.4 requires no announcement in this direction") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "5", caption: "5 Check if noncovered",
        decision: BoxDecision::NotRead("a noncovered security has no basis reported to the IRS, so the row fails the 1a/8a gate and is refused to Form 8949") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "6", caption: "6 Reported to IRS:",
        decision: BoxDecision::NotRead("gross versus net proceeds; either way box 1d is the figure Schedule D's total reads") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025"], label: "7", caption: "7 Check if loss is not allowed",
        decision: BoxDecision::NotRead("an ADJUSTMENT (a loss disallowed because proceeds are less than the amount reported); the row is refused to Form 8949") },
    BoxEntry { stem: "f1099b", editions: &["2026"], label: "7", caption: "7 Check if loss is not",
        decision: BoxDecision::NotRead("an ADJUSTMENT (a loss disallowed because proceeds are less than the amount reported); the row is refused to Form 8949") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "8", caption: "8 Profit or (loss) realized in",
        decision: BoxDecision::NotRead("§1256 regulated futures and forward contracts, which reach Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "9", caption: "9 Unrealized profit or (loss) on",
        decision: BoxDecision::NotRead("§1256 open contracts at the prior year end, Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "10", caption: "10 Unrealized profit or (loss) on",
        decision: BoxDecision::NotRead("§1256 open contracts at this year end, Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "11", caption: "11 Aggregate profit or (loss)",
        decision: BoxDecision::NotRead("the §1256 aggregate, Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "12", caption: "12 Check if basis reported to",
        decision: BoxDecision::Collected { fields: &[FieldId::B1099BasisReportedNoAdjustments], note: "Form1099B.basis_reported_and_no_adjustments — the first half of the gate Schedule D lines 1a/8a require" } },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "13", caption: "13 Bartering",
        decision: BoxDecision::RefuseIfNonzero { field: FieldId::B1099Box13Bartering, reason: || RefuseReason::OtherIncomeLine8zNotModeled("Form 1099-B box 13 (bartering)".to_string()), note: "barter exchange INCOME, reaching Schedule 1 line 8z or Schedule C. btctax fills line 8z from nothing and will not route income to a Schedule C the filer never declared, so > 0 REFUSES (seam review M-1)"} },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "14", caption: "14 State name",
        decision: BoxDecision::NotRead("the state's name; the federal return prints none") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "15", caption: "15 State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099b", editions: &["2024", "2025", "2026"], label: "16", caption: "16 State tax withheld",
        decision: BoxDecision::NotRead("state income tax withheld; no Form1099B field holds it") },
    // ── Form 1098 — Rev. January 2022 and Rev. April 2025 (11 boxes each) ───────────────────────────
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "1", caption: "1 Mortgage interest received from payer(s)/borrower(s)",
        decision: BoxDecision::CollectedElsewhere { fields: &[FieldId::SaMortgage1098], note: "ScheduleAInputs.mortgage_interest_1098 → Schedule A line 8a; §5.2 replaces it with Form1098.box1_interest in T9" } },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "2", caption: "2 Outstanding mortgage",
        decision: BoxDecision::NotRead("T9: Form1098.box2_outstanding_principal feeds the AGGREGATE, status-adjusted §163(h)(3)(B) ceiling check; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "3", caption: "3 Mortgage origination date",
        decision: BoxDecision::NotRead("T9: Form1098.box3_origination_date decides the $750,000 versus $1,000,000 ceiling by whether it precedes 2017-12-16; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "4", caption: "4 Refund of overpaid",
        decision: BoxDecision::NotRead("T9 (fold I3): > 0 REFUSES MortgageInterestRefundNotComputed naming Schedule 1 line 8z — i1098 is explicit that the refund is not netted against the deduction, so a figure held with no reader would understate; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "5", caption: "5 Mortgage insurance",
        decision: BoxDecision::NotRead("T9: mortgage insurance premiums reach Schedule A line 8d only if a final reinstates the §163(h)(3)(E) deduction; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "6", caption: "6 Points paid on purchase of principal residence",
        decision: BoxDecision::NotRead("T9: Form1098.box6_points is added to Schedule A line 8a with box 1; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "7", caption: "7 If address of property securing mortgage is the same",
        decision: BoxDecision::NotRead("T9: Form1098.box7_property_address_same_as_payer, the checkbox box 8's address answers to; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "8", caption: "8 Address or description of property securing mortgage (see",
        decision: BoxDecision::NotRead("T9: Form1098.box8_property_address; no field holds it at T2") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "9", caption: "9 Number of properties securing the",
        decision: BoxDecision::NotRead("the count of properties one mortgage secures; no Schedule A line reads it, and the ceiling check reads box 2's principal") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "10", caption: "10 Other",
        decision: BoxDecision::NotRead("T9: Form1098.box10_other — free-text lender reporting (real estate taxes are the common one), which reaches no line until the filer identifies the item") },
    BoxEntry { stem: "f1098", editions: &["2022", "2025"], label: "11", caption: "11 Mortgage",
        decision: BoxDecision::NotRead("the mortgage ACQUISITION date — when the present lender acquired the loan, printed only on a transferred mortgage. The §163(h)(3)(B) ceiling test reads box 3, the ORIGINATION date, so this box reaches no line") },
    // ── Form 1098-E — 2024, 2025 and 2026 (2 boxes each) ────────────────────────────────────────────
    BoxEntry { stem: "f1098e", editions: &["2024", "2025", "2026"], label: "1", caption: "1 Student loan interest received by lender",
        decision: BoxDecision::Collected { fields: &[FieldId::Form1098eBox1Interest],
            note: "Form1098E.box1_interest → Schedule 1 line 21 (§221), as the SUM over every transcribed row. It replaced the `sch1.student_loan_interest_paid` scalar at T5: a bare Usd with no lender, no TIN and no transcription date made $0 indistinguishable from 'never asked'" } },
    BoxEntry { stem: "f1098e", editions: &["2024", "2025", "2026"], label: "2", caption: "2 Check if box 1 does not include loan origination fees",
        decision: BoxDecision::NotRead("a qualifier on box 1 for loans made before September 1, 2004: it says the lender left origination fees and capitalized interest OUT. Schedule 1 line 21 takes the box-1 amount as printed, and btctax does not compute the omitted fees") },
];

// ── The enumerator ────────────────────────────────────────────────────────────────────────────────

/// The `-layout` lines of the form's own box grid: strictly between the *Attention* preamble's last
/// sentence — **this edition's own**, see enumerator rule 1 — and Copy A's `Cat. No.` footer.
pub fn face_block<'a>(text: &'a str, preamble_end: &str) -> Result<Vec<&'a str>, String> {
    let lines: Vec<&str> = text.lines().collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(preamble_end))
        .map(|(i, _)| i)
        .collect();
    if starts.len() != 1 {
        return Err(format!(
            "the preamble marker {preamble_end:?} occurs {} times, expected exactly 1 — the face \
             block cannot be bounded and a hand-picked page range would be the hand-list this \
             census exists to avoid",
            starts.len()
        ));
    }
    let start = starts[0];
    let end = lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, l)| l.contains(FACE_END))
        .map(|(i, _)| i)
        .ok_or_else(|| {
            format!("no {FACE_END:?} footer after the preamble — Copy A never closes")
        })?;
    Ok(lines[start + 1..end].to_vec())
}

/// Is `s` exactly a box label — `1`, `2a`, `12b`, or a lettered box `a`..`z`?
///
/// ★ The single-letter arm runs the whole alphabet rather than stopping at `f` (the largest lettered
/// box any archived edition prints). Stopping at the largest observed letter is the shape that makes
/// a future box invisible *and* silences the contiguity guard, which builds its expected set from the
/// largest letter it found. Enumerator rule 6 is what makes the widening safe.
fn is_label(s: &str) -> bool {
    let b = s.as_bytes();
    match b.len() {
        1 => b[0].is_ascii_digit() || b[0].is_ascii_lowercase(),
        2 => b[0].is_ascii_digit() && (b[1].is_ascii_digit() || b[1].is_ascii_lowercase()),
        3 => b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2].is_ascii_lowercase(),
        _ => false,
    }
}

/// A run that BEGINS with a label followed by a space: `"1 Interest income"`, `"a Employee’s …"`.
fn label_head(run: &str) -> Option<(&str, &str)> {
    let (head, rest) = run.split_once(' ')?;
    if is_label(head) && rest.starts_with(|c: char| c.is_uppercase() || c == '(') {
        Some((head, rest))
    } else {
        None
    }
}

/// Cut a run at every embedded label, so `"4 Federal income tax withheld 5 Investment expenses"`
/// yields both boxes. Positions are byte offsets of each label's first character.
fn label_cuts(run: &str) -> Vec<usize> {
    let bytes = run.as_bytes();
    let mut cuts = Vec::new();
    for (i, _) in run.char_indices() {
        if i != 0 && bytes[i - 1] != b' ' {
            continue;
        }
        // The longest label spelling at `i` that is followed by a space and a caption start.
        for len in [3usize, 2, 1] {
            let Some(tok) = run.get(i..i + len) else {
                continue;
            };
            if !is_label(tok) {
                continue;
            }
            let after = &run[i + len..];
            if let Some(rest) = after.strip_prefix(' ') {
                if rest.starts_with(|c: char| c.is_uppercase() || c == '(') {
                    cuts.push(i);
                    break;
                }
            }
        }
    }
    cuts
}

/// Page furniture never belongs to a caption. See rule 3 in the module doc.
fn trim_furniture(caption: &str) -> &str {
    match caption.find(FURNITURE) {
        Some(i) => caption[..i].trim_end(),
        None => caption,
    }
}

/// **Every box the form PRINTS, label → caption, read off the extract.** The caption includes the
/// label, because that is how the form prints it and quoting half a printed run is not a quotation.
pub fn printed_boxes(text: &str, preamble_end: &str) -> Result<BTreeMap<String, String>, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    let mut previous = "";
    for line in face_block(text, preamble_end)? {
        let runs: Vec<&str> = line
            .split("  ")
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .collect();
        for (i, run) in runs.iter().enumerate() {
            if is_label(run) {
                // Rule 6: a bare LETTERED run is the vertical `Code` rail, not a box. Every
                // captionless box these forms print carries a number.
                if !run.bytes().any(|c| c.is_ascii_digit()) {
                    continue;
                }
                // Rule 5: the blank year stub under `For calendar year` is the issuer's blank.
                if previous.contains(YEAR_STUB) && runs.len() == 1 {
                    continue;
                }
                // A bare label adopts the next run on its line — unless that run is itself a box.
                let next = runs.get(i + 1).copied().unwrap_or_default();
                let caption = if next.is_empty() || is_label(next) || label_head(next).is_some() {
                    (*run).to_string()
                } else {
                    format!("{run} {}", trim_furniture(next))
                };
                found.entry((*run).to_string()).or_insert(caption);
                continue;
            }
            let mut cuts = label_cuts(run);
            if cuts.is_empty() {
                continue;
            }
            cuts.push(run.len());
            for w in cuts.windows(2) {
                let seg = run[w[0]..w[1]].trim();
                if let Some((label, _)) = label_head(seg) {
                    found
                        .entry(label.to_string())
                        .or_insert_with(|| trim_furniture(seg).to_string());
                }
            }
        }
        previous = line;
    }
    contiguous(&found)?;
    Ok(found)
}

/// ★★ **THE GUARD ON THE ENUMERATOR** — the numeric labels must run `1..=max` with no gap, and any
/// lettered labels must run from `a`. A reader that drops a box otherwise returns a shorter list that
/// still looks plausible; this turns that into a red. It has caught two: the first run-splitter lost
/// Form 1099-DIV boxes 3, 4, 5 and 6, and the Rev. January 2022 Form 1098's blank *For calendar year*
/// stub entered as a box 20 (enumerator rule 5).
fn contiguous(found: &BTreeMap<String, String>) -> Result<(), String> {
    let mut numbers: BTreeSet<u32> = BTreeSet::new();
    let mut letters: BTreeSet<char> = BTreeSet::new();
    for label in found.keys() {
        let digits: String = label.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            letters.insert(label.chars().next().expect("non-empty label"));
        } else {
            numbers.insert(digits.parse().expect("digits parse"));
        }
    }
    if let Some(&max) = numbers.iter().next_back() {
        let missing: Vec<u32> = (1..=max).filter(|n| !numbers.contains(n)).collect();
        if !missing.is_empty() {
            return Err(format!(
                "the box numbers this enumerator found are not contiguous: {missing:?} missing from \
                 1..={max}. A form does not skip a box number, so this is the READER dropping boxes, \
                 not the form omitting them"
            ));
        }
    }
    if !letters.is_empty() {
        let expected: BTreeSet<char> =
            ('a'..=*letters.iter().next_back().expect("non-empty")).collect();
        if letters != expected {
            return Err(format!(
                "the lettered boxes are not contiguous from 'a': found {letters:?}, expected \
                 {expected:?} — the reader is dropping boxes"
            ));
        }
    }
    Ok(())
}

// ── The gate ──────────────────────────────────────────────────────────────────────────────────────

/// **PURE, so the kill can plant defects without touching the tree** (the `field_census.rs:329`
/// shape). `printed` is what the extract prints; `entries` is what the census claims.
pub fn verdict(printed: &BTreeMap<String, String>, entries: &[(&str, &str)]) -> Result<(), String> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut problems = Vec::new();
    for (label, caption) in entries {
        if !seen.insert(label) {
            problems.push(format!("box {label} has more than one entry"));
            continue;
        }
        match printed.get(*label) {
            None => problems.push(format!(
                "box {label} has an entry ({caption:?}) but the extract does not print it — a stale \
                 entry survives a revision that dropped the box"
            )),
            Some(printed_caption) if printed_caption != caption => problems.push(format!(
                "box {label}'s caption does not match the extract: census {caption:?} vs printed \
                 {printed_caption:?}"
            )),
            Some(_) => {}
        }
    }
    for (label, caption) in printed {
        if !seen.contains(label.as_str()) {
            problems.push(format!(
                "box {label} ({caption:?}) is printed on the form and NOTHING decides it — we forgot \
                 this box, which is invisible on the page and to every value assertion"
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n  "))
    }
}

fn extract_path(root: &Path, stem: &str, edition: &str) -> PathBuf {
    root.join(format!("design/forms/extract/{stem}--{edition}.txt"))
}

/// The census entries in scope for one archived edition.
#[must_use]
pub fn entries_for(doc: &DocumentAuthority) -> Vec<&'static BoxEntry> {
    BOXES
        .iter()
        .filter(|b| b.stem == doc.stem && b.editions.contains(&doc.edition))
        .collect()
}

/// `(stem, edition)` pairs, forms and booklets — what [`archived_information_returns`] returns.
pub type ArchivedEditions = (BTreeSet<(String, String)>, BTreeSet<(String, String)>);

/// ★★★ **I2 — THE ARCHIVED W / 1098 / 1099 EDITIONS, READ OUT OF `MANIFEST.json`.**
///
/// Returns `(forms, booklets)` as `(stem, edition)` pairs. This is the join [`DOCUMENTS`] and
/// [`BOOKLETS`] are asserted equal to, in both directions — the seam review deleted a whole archived
/// form from the census and `box-census` plus all six census tests reported success, because the
/// document set was a hand-list with no join to anything.
pub fn archived_information_returns(root: &Path) -> Result<ArchivedEditions, String> {
    let entries = crate::authority_manifest::load(root)?;
    let (mut forms, mut booklets) = (BTreeSet::new(), BTreeSet::new());
    for e in &entries {
        let name = e.path.rsplit('/').next().unwrap_or_default();
        let Some((stem, rest)) = name.split_once("--") else {
            continue;
        };
        if !crate::archive_check::is_information_return_stem(stem) {
            continue;
        }
        let edition = rest.trim_end_matches(".pdf").to_string();
        match e.kind {
            crate::authority_manifest::Kind::Form => {
                forms.insert((stem.to_string(), edition));
            }
            crate::authority_manifest::Kind::Instructions => {
                booklets.insert((stem.to_string(), edition));
            }
            _ => {}
        }
    }
    Ok((forms, booklets))
}

/// The I2 assertion itself, so the operator command and the suite ask the archive the same question.
pub fn check_document_set(root: &Path) -> Result<(), String> {
    let (forms, booklets) = archived_information_returns(root)?;
    if forms.len() < 7 {
        return Err(format!(
            "the manifest join found only {} information-return forms — the derivation itself is \
             broken, and a census over nothing passes",
            forms.len()
        ));
    }
    let censused_forms: BTreeSet<(String, String)> = DOCUMENTS
        .iter()
        .map(|d| (d.stem.to_string(), d.edition.to_string()))
        .collect();
    let censused_booklets: BTreeSet<(String, String)> = BOOKLETS
        .iter()
        .map(|b| (b.stem.to_string(), b.edition.to_string()))
        .collect();
    let mut problems = Vec::new();
    for (what, archived, censused) in [
        ("form", &forms, &censused_forms),
        ("instructions", &booklets, &censused_booklets),
    ] {
        for m in archived.difference(censused) {
            problems.push(format!(
                "{}--{} is archived as a {what} but nothing censuses it — an archived information \
                 return outside the census is invisible to every box assertion",
                m.0, m.1
            ));
        }
        for m in censused.difference(archived) {
            problems.push(format!(
                "{}--{} is censused as a {what} but is not in MANIFEST.json — the census names a \
                 document the archive does not hold",
                m.0, m.1
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n  "))
    }
}

/// `xtask box-census` — the operator-facing run.
pub fn run() -> Result<(), String> {
    let root = repo_root();
    check_document_set(&root)
        .map_err(|e| format!("the censused document set is not the archive:\n  {e}"))?;
    let mut total = 0usize;
    let mut decided = 0usize;
    let mut failures = Vec::new();
    for doc in DOCUMENTS {
        let path = extract_path(&root, doc.stem, doc.edition);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let printed = printed_boxes(&text, doc.preamble_end)
            .map_err(|e| format!("{}--{}: {e}", doc.stem, doc.edition))?;
        let in_scope = entries_for(doc);
        let entries: Vec<(&str, &str)> = in_scope.iter().map(|b| (b.label, b.caption)).collect();
        let (mut collected, mut refusing, mut unread) = (0, 0, 0);
        for b in &in_scope {
            match b.decision {
                BoxDecision::Collected { .. } | BoxDecision::CollectedElsewhere { .. } => {
                    collected += 1;
                }
                BoxDecision::RefuseIfNonzero { .. } => refusing += 1,
                BoxDecision::NotRead(_) => unread += 1,
            }
        }
        match verdict(&printed, &entries) {
            Ok(()) => println!(
                "  {}--{} ({} instructions): {} boxes — {collected} collected, {refusing} \
                 refuse-if-nonzero, {unread} not read",
                doc.stem,
                doc.edition,
                doc.instructions,
                printed.len()
            ),
            Err(e) => failures.push(format!("{}--{}:\n  {e}", doc.stem, doc.edition)),
        }
        total += printed.len();
        decided += in_scope.len();
    }
    // ★★★ R4 / T5 — THE JOIN, checked by the command a human types when a form changes.
    let join = field_join_failures();
    if !join.is_empty() {
        failures.push(format!(
            "the box → FieldId join failed on {} entr(ies):\n  {}",
            join.len(),
            join.join("\n  ")
        ));
    }
    if !failures.is_empty() {
        return Err(format!(
            "the box census failed for {} document(s):\n{}",
            failures.len(),
            failures.join("\n")
        ));
    }
    println!(
        "box-census OK: {total} printed boxes across {} archived editions of {} information \
         returns, every one decided ({decided} entries)",
        DOCUMENTS.len(),
        DOCUMENTS
            .iter()
            .map(|d| d.stem)
            .collect::<BTreeSet<_>>()
            .len()
    );
    Ok(())
}

// ── ★★★ R4 / T5 — THE JOIN: a box decision, the form registry, and the printed caption ──────────

/// Every `Field` of `form_spec()`, as `(SectionId, FieldId)` — the structure the census joins to.
#[must_use]
pub fn form_fields() -> Vec<(SectionId, FieldId)> {
    btctax_input_form::form_spec()
        .iter()
        .flat_map(|s| s.fields.iter().map(move |f| (s.id, f.id)))
        .collect()
}

/// The words a `Field` puts in front of the filer — its label and its help, run together.
#[must_use]
pub fn field_words(id: FieldId) -> Option<String> {
    btctax_input_form::form_spec().iter().find_map(|s| {
        s.fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| format!("{} {}", f.label, f.help))
    })
}

/// A caption or a field's words, reduced to what a comparison can honestly be made on: lowercase,
/// single-spaced, and with the typographic apostrophe folded to the ASCII one.
///
/// ★ The fold is not cosmetic. The IRS extracts print `Employee’s`, and a Rust source literal
/// naturally carries `Employee's`; comparing them raw would make the check fail on a difference no
/// reader would call a difference, and the usual repair for that is to delete the check.
#[must_use]
pub fn normalize(s: &str) -> String {
    s.replace(['\u{2019}', '\u{2018}'], "'")
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The caption's own WORDS — the caption with its leading label stripped. `None` when the caption is
/// nothing but the label (the W-2's shaded box 9 and its bare `12b`/`12c`/`12d` slots), which is a
/// real answer and not a failure: there is no wording to check.
#[must_use]
pub fn caption_words(entry: &BoxEntry) -> Option<String> {
    let rest = entry.caption.strip_prefix(entry.label)?.trim();
    (!rest.is_empty()).then(|| normalize(rest))
}

/// ★★★ **THE JOIN, AS A CHECK THE COMMAND ITSELF RUNS.**
///
/// Every failure the census's own decisions can carry, in one pass:
/// - a `Collected`/`RefuseIfNonzero` naming a `FieldId` that is not a `Field` of the document's OWN
///   section (a `FieldId` that does not exist cannot compile);
/// - a `CollectedElsewhere` naming one that IS on the document's own row (say `Collected`, or the
///   distinction stops meaning anything);
/// - a collected box whose printed CAPTION appears in none of the fields that collect it — the
///   revision-drift case a caption-only census cannot see;
/// - a `NotRead` with an empty reason, which is *"we forgot this box"* with extra steps.
///
/// ★ It runs inside [`run`] rather than only in a test, because `xtask box-census` is what a human
///   types when they change a form, and a check that only the suite performs is a check they meet a
///   round late.
///
/// ★★★ **It takes the entries rather than reading [`BOXES`] directly, and that is B1, not style.**
/// A negative test can only *run the instrument* on a planted table if the instrument accepts one;
/// with the constant baked in, the only kill available is a re-implementation of the checker's own
/// predicates — which stays green when the checker is deleted, and is therefore not a kill at all.
/// See [`tests::the_join_reds_on_a_field_from_the_wrong_section_and_on_a_reworded_caption`].
#[must_use]
pub fn field_join_failures() -> Vec<String> {
    join_failures_for(BOXES)
}

/// [`field_join_failures`] over an arbitrary entry table — the form the kill test drives.
#[must_use]
pub fn join_failures_for(entries: &[BoxEntry]) -> Vec<String> {
    let fields = form_fields();
    let sections_of = |id: FieldId| -> Vec<SectionId> {
        fields
            .iter()
            .filter(|(_, f)| *f == id)
            .map(|(s, _)| *s)
            .collect()
    };
    let mut bad = Vec::new();
    for b in entries {
        let own = section_of_stem(b.stem);
        let words = caption_words(b);
        // The fields that must carry the box's own printed words, if any.
        let mut carriers: Vec<FieldId> = Vec::new();
        match b.decision {
            BoxDecision::Collected { fields: fs, note } => {
                if note.trim().is_empty() {
                    bad.push(format!(
                        "{}/{}: a Collected entry with no note",
                        b.stem, b.label
                    ));
                }
                let Some(section) = own else {
                    bad.push(format!(
                        "{}/{}: `Collected` needs the document's OWN section, and {} has none —                          use `CollectedElsewhere`",
                        b.stem, b.label, b.stem
                    ));
                    continue;
                };
                if fs.is_empty() {
                    bad.push(format!("{}/{}: Collected names no field", b.stem, b.label));
                }
                for f in fs {
                    let secs = sections_of(*f);
                    if secs.is_empty() {
                        bad.push(format!(
                            "{}/{}: {f:?} is not a Field of the form spec",
                            b.stem, b.label
                        ));
                    } else if !secs.contains(&section) {
                        bad.push(format!(
                            "{}/{}: {f:?} is in {secs:?}, not in this document's own section                              ({section:?}) — a box collected somewhere else is                              `CollectedElsewhere`, with the reason said out loud",
                            b.stem, b.label
                        ));
                    }
                }
                carriers = fs.to_vec();
            }
            BoxDecision::CollectedElsewhere { fields: fs, note } => {
                if note.trim().is_empty() {
                    bad.push(format!(
                        "{}/{}: a CollectedElsewhere entry with no note — the note IS the reason it                          is elsewhere",
                        b.stem, b.label
                    ));
                }
                if fs.is_empty() {
                    bad.push(format!(
                        "{}/{}: CollectedElsewhere names no field",
                        b.stem, b.label
                    ));
                }
                for f in fs {
                    let secs = sections_of(*f);
                    if secs.is_empty() {
                        bad.push(format!(
                            "{}/{}: {f:?} is not a Field of the form spec",
                            b.stem, b.label
                        ));
                    } else if own.is_some_and(|o| secs.contains(&o)) {
                        bad.push(format!(
                            "{}/{}: {f:?} IS in this document's own section — say `Collected`",
                            b.stem, b.label
                        ));
                    }
                }
            }
            BoxDecision::RefuseIfNonzero {
                field,
                reason,
                note,
            } => {
                if note.trim().is_empty() {
                    bad.push(format!(
                        "{}/{}: a refuse-guard with no note",
                        b.stem, b.label
                    ));
                }
                // ★ Construct the refusal, so the entry names a variant that really exists AND is
                //   constructible — not merely a path that compiles behind a closure.
                let _ = reason();
                match own {
                    None => bad.push(format!(
                        "{}/{}: a refuse-guard needs the document's own section",
                        b.stem, b.label
                    )),
                    Some(section) if !sections_of(field).contains(&section) => bad.push(format!(
                        "{}/{}: the refuse-guard field {field:?} must live on this document's own                          row — a guard the filer cannot reach is a brick",
                        b.stem, b.label
                    )),
                    Some(_) => carriers.push(field),
                }
            }
            BoxDecision::NotRead(reason) => {
                if reason.trim().is_empty() {
                    bad.push(format!(
                        "{}/{}: a NotRead with an EMPTY reason — the reason is the whole value of                          the entry, and without it this is \"we forgot this box\" with extra steps",
                        b.stem, b.label
                    ));
                }
            }
        }
        if let Some(w) = words {
            if !carriers.is_empty()
                && !carriers
                    .iter()
                    .any(|f| field_words(*f).is_some_and(|s| normalize(&s).contains(w.as_str())))
            {
                bad.push(format!(
                    "{}/{}: the box prints {w:?}, and no field that collects it ({carriers:?}) says                      so — a box whose caption moved between revisions must not keep pointing at a                      field describing the old one",
                    b.stem, b.label
                ));
            }
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(stem: &str, edition: &str) -> String {
        let p = extract_path(&repo_root(), stem, edition);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
    }

    /// ★★★ **THE CENSUS ITSELF** — every printed box of every archived EDITION decided, every entry
    /// a box that edition prints, every caption verbatim. This is R4's kill in both directions at
    /// once, now once per edition.
    #[test]
    fn every_printed_box_carries_exactly_one_entry() {
        let mut total = 0usize;
        let mut decided = 0usize;
        let mut failures = Vec::new();
        for doc in DOCUMENTS {
            let printed = printed_boxes(&read(doc.stem, doc.edition), doc.preamble_end)
                .unwrap_or_else(|e| panic!("{}--{}: {e}", doc.stem, doc.edition));
            let in_scope = entries_for(doc);
            let entries: Vec<(&str, &str)> =
                in_scope.iter().map(|b| (b.label, b.caption)).collect();
            if let Err(e) = verdict(&printed, &entries) {
                failures.push(format!("{}--{}:\n  {e}", doc.stem, doc.edition));
            }
            total += printed.len();
            decided += in_scope.len();
        }
        assert!(
            failures.is_empty(),
            "the box census failed for {} of {} editions:\n{}",
            failures.len(),
            DOCUMENTS.len(),
            failures.join("\n")
        );
        // A census that walked nothing would pass by finding nothing — the F4 shape.
        assert_eq!(
            total, decided,
            "every entry must correspond to a printed box of an edition it lists, and vice versa"
        );
        assert!(
            total >= 200,
            "the census walked only {total} boxes across {} editions — it must actually read the \
             forms",
            DOCUMENTS.len()
        );
        eprintln!(
            "box census: {total} boxes across {} editions, {} entries",
            DOCUMENTS.len(),
            BOXES.len()
        );
    }

    /// ★ Every entry names an EDITION the registry knows, so neither a typo'd stem nor a typo'd year
    /// can park an entry where no extract will ever check it.
    #[test]
    fn every_entry_belongs_to_an_archived_edition() {
        let known: BTreeSet<(&str, &str)> = DOCUMENTS.iter().map(|d| (d.stem, d.edition)).collect();
        for b in BOXES {
            assert!(
                !b.editions.is_empty(),
                "box entry {}/{} lists NO edition — nothing would ever check it",
                b.stem,
                b.label
            );
            for ed in b.editions {
                assert!(
                    known.contains(&(b.stem, ed)),
                    "box entry {}/{} names edition {}--{ed}, which is in no DOCUMENTS row — nothing \
                     would ever check it",
                    b.stem,
                    b.label,
                    b.stem
                );
            }
        }
    }

    /// ★ The bound and furniture markers are each EDITION's own text, so the enumerator's bounds are
    /// a reading of that document rather than a convention we assert about all of them. The
    /// preamble marker is required to occur **exactly once**, which is what turned the 2026 Form
    /// W-2's rewritten preamble into a recorded per-edition bound instead of a hard failure.
    #[test]
    fn the_enumerator_markers_are_present_in_every_extract() {
        for doc in DOCUMENTS {
            let text = read(doc.stem, doc.edition);
            assert_eq!(
                text.lines()
                    .filter(|l| l.contains(doc.preamble_end))
                    .count(),
                1,
                "{}--{}: its recorded preamble marker {:?} does not occur exactly once; the face \
                 block is being guessed",
                doc.stem,
                doc.edition,
                doc.preamble_end
            );
            for marker in [FACE_END, FURNITURE] {
                assert!(
                    text.contains(marker),
                    "{}--{} does not contain {marker:?}; the enumerator's bounds are not this \
                     document's own furniture",
                    doc.stem,
                    doc.edition
                );
            }
        }
    }

    /// ★★ Both halves of the archive obligation: the extract AND the provenance note are on disk for
    /// every edition the census reads, form and booklet. Without the note the extract cannot be
    /// reproduced, and "archived" would mean a text file nobody can re-derive.
    #[test]
    fn every_document_has_its_extract_and_its_note() {
        let root = repo_root();
        let mut editions: Vec<(&str, &str)> =
            DOCUMENTS.iter().map(|d| (d.stem, d.edition)).collect();
        editions.extend(BOOKLETS.iter().map(|b| (b.stem, b.edition)));
        for (stem, edition) in editions {
            assert!(
                extract_path(&root, stem, edition).is_file(),
                "{stem}--{edition}: no committed extract"
            );
            let note = root.join(format!("design/forms/{edition}/{stem}--{edition}.pdf.txt"));
            assert!(
                note.is_file(),
                "{stem}--{edition}: no provenance note at {} — the PDF is gitignored, so without \
                 the note the text layer cannot be reproduced",
                note.display()
            );
        }
    }

    /// ★★★ **C1c — WHICH EDITION GOVERNS WHICH TAX YEAR, pinned for all three years the interview
    /// serves.** Every cell is the IRS's printed rule applied to a revision read off a document, and
    /// the table is what `line-coverage` resolves a `DocBox` row's edition through.
    ///
    /// ★ The 1099-G row is the review's C1 in one line: TY2024 and TY2025 are the Rev. March 2024
    /// grid, TY2026 is the Rev. December 2026 grid with its new income box 10.
    #[test]
    fn revision_in_force_pins_the_whole_table() {
        // (stem, TY2024, TY2025, TY2026)
        /// (stem, the edition in force for TY2024, for TY2025, for TY2026).
        type Row = (
            &'static str,
            Option<&'static str>,
            Option<&'static str>,
            Option<&'static str>,
        );
        let expected: &[Row] = &[
            // Annual forms: the edition IS the tax year.
            ("fw2", Some("2024"), Some("2025"), Some("2026")),
            ("iw2w3", Some("2024"), Some("2025"), Some("2026")),
            ("f1099b", Some("2024"), Some("2025"), Some("2026")),
            ("i1099b", Some("2024"), Some("2025"), Some("2026")),
            ("f1098e", Some("2024"), Some("2025"), Some("2026")),
            ("i1098et", Some("2024"), Some("2025"), Some("2026")),
            // Periodic: Rev. January 2024, never superseded — one edition serves all three years.
            ("f1099int", Some("2024"), Some("2024"), Some("2024")),
            ("i1099int", Some("2024"), Some("2024"), Some("2024")),
            ("f1099div", Some("2024"), Some("2024"), Some("2024")),
            ("i1099div", Some("2024"), Some("2024"), Some("2024")),
            // Periodic, DISPLACED: Rev. March 2024 → Rev. December 2026.
            ("f1099g", Some("2024"), Some("2024"), Some("2026")),
            ("i1099g", Some("2024"), Some("2024"), Some("2026")),
            // Periodic: Rev. January 2022 governs TY2024; Rev. April 2025 takes over for TY2025.
            ("f1098", Some("2022"), Some("2025"), Some("2025")),
            // ★ The BOOKLET was revised for TY2026 when the form was not.
            ("i1098", Some("2022"), Some("2025"), Some("2026")),
        ];
        for (stem, y24, y25, y26) in expected {
            assert_eq!(revision_in_force(stem, 2024), *y24, "{stem} @ TY2024");
            assert_eq!(revision_in_force(stem, 2025), *y25, "{stem} @ TY2025");
            assert_eq!(revision_in_force(stem, 2026), *y26, "{stem} @ TY2026");
        }
        // ★ `None` is a real answer, in both directions.
        assert_eq!(
            revision_in_force("fw2", 2023),
            None,
            "no 2023 Form W-2 is archived, and an annual edition never governs a neighbouring year"
        );
        assert_eq!(
            revision_in_force("f1098", 2021),
            None,
            "Rev. January 2022 is first used to report 2022 amounts, so it governs nothing earlier"
        );
        assert_eq!(
            revision_in_force("f1099nec", 2024),
            None,
            "a document that is not archived governs nothing"
        );
    }

    /// ★★★ **I2 — THE CENSUSED DOCUMENT SET IS DERIVED FROM THE ARCHIVE, both directions.**
    ///
    /// The seam review deleted the whole `f1098e` authority and its two entries, and `box-census`
    /// plus all six census tests reported success: the document set was a hand-list with no join to
    /// anything. It is now the `MANIFEST.json` W / 1098 / 1099 series — `kind: form` for
    /// [`DOCUMENTS`], `kind: instructions` for [`BOOKLETS`] — so an archived edition that is never
    /// censused reds, and a censused edition that is not archived reds.
    #[test]
    fn documents_equal_the_archived_information_returns() {
        let root = repo_root();
        let (forms, booklets) =
            archived_information_returns(&root).expect("the manifest join must resolve");
        assert!(
            forms.len() >= 7 && booklets.len() >= 7,
            "the derivation found {} forms and {} booklets — a census over nothing passes",
            forms.len(),
            booklets.len()
        );
        check_document_set(&root).expect("DOCUMENTS/BOOKLETS must equal the archived series");
        // ★ Every form's booklet stem must itself be archived, or the `instructions` field names a
        //   document nobody holds.
        for doc in DOCUMENTS {
            assert!(
                BOOKLETS.iter().any(|b| b.stem == doc.instructions),
                "{}--{} names instructions {:?}, which is in no BOOKLETS row",
                doc.stem,
                doc.edition,
                doc.instructions
            );
        }
        // ★★ B1 — the join watched RED on the exact defect the seam review planted: an archived
        //    edition the census does not carry. Planted here rather than in the tree, so the kill
        //    needs no mutation of a committed table.
        let dropped: BTreeSet<(String, String)> = DOCUMENTS
            .iter()
            .skip(1)
            .map(|d| (d.stem.to_string(), d.edition.to_string()))
            .collect();
        let missing: Vec<_> = forms.difference(&dropped).collect();
        assert_eq!(
            missing.len(),
            1,
            "dropping one authority must leave exactly one archived edition uncensused"
        );
    }

    /// ★★★ **B1 — the gate watched going RED on every defect class it claims to catch.**
    ///
    /// Each plant is the minimal mutation of an exactly-accounted document. If any of these starts
    /// returning `Ok`, the corresponding arm of [`verdict`] has been gutted and
    /// [`every_printed_box_carries_exactly_one_entry`] is green over a blind check.
    #[test]
    fn the_gate_reds_on_every_planted_defect() {
        let printed: BTreeMap<String, String> = [
            ("1", "1 Interest income"),
            ("2", "2 Early withdrawal penalty"),
            (
                "3",
                "3 Interest on U.S. Savings Bonds and Treasury obligations",
            ),
        ]
        .iter()
        .map(|(l, c)| (l.to_string(), c.to_string()))
        .collect();

        let clean: &[(&str, &str)] = &[
            ("1", "1 Interest income"),
            ("2", "2 Early withdrawal penalty"),
            (
                "3",
                "3 Interest on U.S. Savings Bonds and Treasury obligations",
            ),
        ];
        assert!(
            verdict(&printed, clean).is_ok(),
            "the baseline must PASS, or every red below is meaningless"
        );

        /// One planted defect: (what it is, the entries to feed [`verdict`], the substring the
        /// refusal must contain). Named so `-D clippy::type-complexity` is satisfied by making the
        /// shape readable rather than by an `allow`.
        type Plant = (
            &'static str,
            &'static [(&'static str, &'static str)],
            &'static str,
        );

        let plants: &[Plant] = &[
            // (1) THE defect this census exists for: a box on the paper that nothing decided.
            (
                "a printed box with no entry",
                &[
                    ("1", "1 Interest income"),
                    ("2", "2 Early withdrawal penalty"),
                ],
                "we forgot this box",
            ),
            // (2) The stale entry — a revision dropped the box and the census kept it.
            (
                "an entry for a box the extract does not print",
                &[
                    ("1", "1 Interest income"),
                    ("2", "2 Early withdrawal penalty"),
                    (
                        "3",
                        "3 Interest on U.S. Savings Bonds and Treasury obligations",
                    ),
                    ("4", "4 Federal income tax withheld"),
                ],
                "the extract does not print it",
            ),
            // (3) ONE CHARACTER of a caption — the transcription defect this repo's standing rule is
            //     written against ("Treasury" → "Treasry").
            (
                "a caption changed by one character",
                &[
                    ("1", "1 Interest income"),
                    ("2", "2 Early withdrawal penalty"),
                    (
                        "3",
                        "3 Interest on U.S. Savings Bonds and Treasry obligations",
                    ),
                ],
                "caption does not match the extract",
            ),
            // (4) Two entries for one box — a contradiction, and the shape that lets a second entry
            //     quietly override the first if the reader takes the last one.
            (
                "two entries for one box",
                &[
                    ("1", "1 Interest income"),
                    ("1", "1 Interest income"),
                    ("2", "2 Early withdrawal penalty"),
                    (
                        "3",
                        "3 Interest on U.S. Savings Bonds and Treasury obligations",
                    ),
                ],
                "more than one entry",
            ),
        ];

        for (what, entries, needle) in plants {
            let got = verdict(&printed, entries);
            let msg = got.expect_err(&format!("planting {what} must RED, and it did not"));
            assert!(
                msg.contains(needle),
                "planting {what} red for the wrong reason: expected a message containing \
                 {needle:?}, got:\n{msg}"
            );
        }
    }

    /// ★★ **The guard on the READER, planted.** A face block with box 2 removed must red rather than
    /// return a shorter, plausible list — the exact failure the first run-splitter produced on Form
    /// 1099-DIV.
    #[test]
    fn the_enumerator_reds_when_it_drops_a_box() {
        let good = "See Publications 1141, 1167, and 1179 for more information.\n\
             1 Interest income\n\
             2 Early withdrawal penalty\n\
             3 Interest on U.S. Savings Bonds\n\
             Form 1099-INT   Cat. No. 14410K\n";
        let boxes = printed_boxes(good, PREAMBLE_1141).expect("the baseline must enumerate");
        assert_eq!(boxes.len(), 3, "baseline: {boxes:?}");

        let gapped = good.replace("2 Early withdrawal penalty\n", "");
        let err = printed_boxes(&gapped, PREAMBLE_1141)
            .expect_err("a dropped box must RED, not shorten the list");
        assert!(err.contains("not contiguous"), "wrong reason: {err}");

        let unbounded = good.replace("Cat. No. 14410K", "");
        let err =
            printed_boxes(&unbounded, PREAMBLE_1141).expect_err("an unbounded face block must RED");
        assert!(err.contains("Copy A never closes"), "wrong reason: {err}");

        // ★ A marker that is not this edition's own must red rather than silently fall back.
        let err = printed_boxes(good, PREAMBLE_W2_2026)
            .expect_err("a preamble marker absent from the document must RED");
        assert!(err.contains("occurs 0 times"), "wrong reason: {err}");
    }

    /// ★★ **Enumerator rules 5 and 6, planted** — the two furniture readings that keep the widened
    /// [`is_label`] honest. Both are lifted from real documents: the Rev. January 2022 Form 1098's
    /// blank *For calendar year* stub, and the Form W-2's vertical *Code* rail.
    #[test]
    fn the_enumerator_reads_the_year_stub_and_the_code_rail_as_furniture() {
        let with_stub = "See Publications 1141, 1167, and 1179 for more information.\n\
             \x20                        For calendar year                 Statement\n\
             \x20                              20\n\
             1 Mortgage interest received\n\
             2 Outstanding mortgage\n\
             Form 1098   Cat. No. 14402K\n";
        let boxes = printed_boxes(with_stub, PREAMBLE_1141)
            .expect("the calendar-year stub must not be read as box 20");
        assert_eq!(
            boxes.keys().collect::<Vec<_>>(),
            ["1", "2"],
            "the blank year stub entered the census: {boxes:?}"
        );

        let with_rail = "See Publications 1141, 1167, and 1179 for more information.\n\
             a Employee’s social security number\n\
             1 Wages, tips, other compensation                 C\n\
             \x20                                              o\n\
             \x20                                              d\n\
             \x20                                              e\n\
             Form W-2   Cat. No. 10134D\n";
        let boxes = printed_boxes(with_rail, PREAMBLE_1141)
            .expect("the vertical Code rail must not be read as boxes o/d/e");
        assert_eq!(
            boxes.keys().collect::<Vec<_>>(),
            ["1", "a"],
            "the Code rail entered the census: {boxes:?}"
        );
    }

    // ── ★★★ R4 / T5 — THE JOIN TESTS. ──────────────────────────────────────────────────────────

    /// ★★★ **THE JOIN IS CLEAN, and the check the COMMAND runs is the one the suite runs.**
    ///
    /// [`field_join_failures`] is a single pass over every entry: a `Collected` naming a field of
    /// another section, a `CollectedElsewhere` naming one of its own, a caption no collecting field
    /// carries, and a `NotRead` with an empty reason. Sharing it with [`run`] is what stops the
    /// suite and the command from checking two different things.
    #[test]
    fn the_box_to_field_join_is_clean() {
        let bad = field_join_failures();
        assert!(
            bad.is_empty(),
            "the box → FieldId join failed:\n  {}",
            bad.join("\n  ")
        );
    }

    /// Every archived stem either HAS a form section or is recorded as having none, and the record is
    /// [`section_of_stem`]'s own `match` rather than a comment.
    #[test]
    fn every_stem_maps_to_a_section_or_says_why() {
        let stems: BTreeSet<&str> = DOCUMENTS.iter().map(|d| d.stem).collect();
        let without: Vec<&str> = stems
            .iter()
            .copied()
            .filter(|s| section_of_stem(s).is_none())
            .collect();
        assert_eq!(
            without,
            vec!["f1098"],
            "Form 1098 is the only archived document with no form section (its one collected box is \
             still the Schedule A scalar until T9). Anything else here is a document whose section \
             landed without `section_of_stem` being told."
        );
    }

    /// ★★★ **B1 — THE JOIN, SEEN RED ON PLANTED DEFECTS.** Four planted tables, one per mechanism,
    /// each **run through [`join_failures_for`] itself** — plus the honest table through the same
    /// door, so the test measures the checker DISCRIMINATING rather than merely complaining.
    ///
    /// ★★★ **It calls the instrument on purpose.** The version this replaces re-derived
    /// [`form_fields`] / [`field_words`] / [`normalize`] / [`caption_words`] and asserted that
    /// those *predicates* behave — every one of which stays green with [`field_join_failures`]
    /// gutted to `Vec::new()`, so the answer to B1's one reviewable sentence (*"which test reds
    /// when this checker is removed?"*) was **none**. It also disarmed the M1 plant: with the
    /// checker blind, 14 `Collected` entries naming another document's fields passed the whole
    /// suite. A kill that re-implements a checker's reasoning is B1 satisfied performatively.
    #[test]
    fn the_join_reds_on_a_field_from_the_wrong_section_and_on_a_reworded_caption() {
        let real = BOXES
            .iter()
            .find(|b| b.stem == "f1099int" && b.label == "1")
            .expect("1099-INT box 1 is censused");
        let entry = |stem, label, caption, decision| BoxEntry {
            stem,
            editions: &["2024"],
            label,
            caption,
            decision,
        };
        // Every plant is asserted by MESSAGE, not merely by count: a checker that returned one
        // failure for everything would satisfy `!is_empty()` and diagnose nothing.
        let reds = |planted: &[BoxEntry], needle: &str, what: &str| {
            let bad = join_failures_for(planted);
            assert!(
                !bad.is_empty(),
                "★ THE KILL ({what}): the join must red on this planted table, and it returned \
                 NOTHING — the checker is not checking"
            );
            assert!(
                bad.iter().any(|m| m.contains(needle)),
                "★ THE KILL ({what}): no failure said {needle:?}; got:\n  {}",
                bad.join("\n  ")
            );
        };

        // (0) THE POSITIVE CONTROL — the honest entry, through the same door, is silent.
        assert!(
            join_failures_for(&[entry("f1099int", "1", "1 Interest income", real.decision)])
                .is_empty(),
            "premise: the real 1099-INT box 1 entry passes the join — without this the three \
             plants below would be satisfied by a checker that fails everything"
        );

        // (1) A `Collected` naming a field of ANOTHER section. `Box1Wages` is a W-2 field; the
        //     1099-INT's own section is `Int1099s`. (This is the M1 plant's shape, one entry wide.)
        reds(
            &[entry(
                "f1099int",
                "1",
                "1 Interest income",
                BoxDecision::Collected {
                    fields: &[FieldId::Box1Wages],
                    note: "planted: a W-2 field claimed as the 1099-INT's own",
                },
            )],
            "not in this document's own section",
            "a Collected naming a field of another section",
        );

        // (2) A caption the collecting field's words do not carry — the revision-drift case. The
        //     real caption "1 Interest income" IS in `Int1099Box1Interest`'s words (0 proves it).
        reds(
            &[entry(
                "f1099int",
                "1",
                "1 Interest income from a source nobody wrote down",
                real.decision,
            )],
            "and no field that collects it",
            "a re-worded caption",
        );

        // (3) A `CollectedElsewhere` whose field IS on the document's own row. `Box1Wages` is a
        //     `W2s` field, so claiming it is collected "elsewhere" than the W-2 row is the defect.
        reds(
            &[entry(
                "fw2",
                "1",
                "1 Wages, tips, other compensation",
                BoxDecision::CollectedElsewhere {
                    fields: &[FieldId::Box1Wages],
                    note: "planted: the document's OWN field claimed as elsewhere",
                },
            )],
            "IS in this document's own section",
            "a CollectedElsewhere naming the row's own field",
        );

        // (4) A `NotRead` with an empty reason — "we forgot this box" with extra steps.
        reds(
            &[entry(
                "f1099int",
                "1",
                "1 Interest income",
                BoxDecision::NotRead(""),
            )],
            "a NotRead with an EMPTY reason",
            "an empty NotRead reason",
        );
    }
}
