//! ★★★ **THE PER-DOCUMENT BOX CENSUS** — every box an archived information return PRINTS carries
//! exactly one recorded decision. (`design/SPEC_interview.md` r2 R4 + §5.2, fold finding I2; built by
//! T2, populated with real decisions by T5/T9.)
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
//! ★★ **The census key is the box LABEL and the value is the box's own printed CAPTION**, quoted
//! verbatim from `design/forms/extract/<stem>--<year>.txt`. So the three kills are one set comparison:
//! a missing entry is a printed box nobody decided, an extra entry is a box the form does not print
//! (a stale revision), and a caption changed by one character is both at once.
//!
//! ## What T2 owns and what it does not (fold D7)
//!
//! > *"T2 owns the extract, so T2's kill is the **unentered-caption** red and T5's is the entries
//! > themselves."*
//!
//! So this module lands the **instrument** and a **populated fixture**: 115 entries across the seven
//! documents, each naming either the struct field that holds the box today, the refusal the box drives
//! today, or the reason the return does not read it. The entries deliberately do **not** yet join to
//! `FieldId` or `RefuseReason` variants — those are T5/T9's, and a join written before the variants
//! exist would be a hand-list pretending to be a check.
//!
//! ## The enumerator, and why each rule is the document's rather than ours
//!
//! 1. **The face block.** Every information return's PDF opens with the same *Attention* preamble and
//!    closes Copy A with a `Cat. No.` footer. The block between them is the form's own box grid, and
//!    the preamble's last sentence — *"See Publications 1141, 1167, and 1179 …"* — occurs **exactly
//!    once** in each of the seven extracts (measured). Bounding on the document's own furniture keeps
//!    the prose pages, the recipient instructions and the repeated Copy B/C/1/2 faces out without a
//!    page-number hand-list.
//! 2. **Runs.** `pdftotext -layout` separates columns by two or more spaces, so each line splits into
//!    runs; a run beginning `<label> <Capital>` is a caption, and a run that is *only* a label adopts
//!    the next run on its line unless that run is itself a label. That second rule is not a nicety:
//!    Form 1099-DIV prints `3    Nondividend distributions` with the label in its own column, and
//!    without it boxes **3, 4, 5 and 6 vanished** from a list that still looked plausible.
//! 3. **`OMB No.` ends a caption.** The Paperwork Reduction Act control number is page furniture on
//!    every IRS form and is never a box caption; on Form 1099-G the layout collapses the column gap to
//!    a single space (`1 Unemployment compensation OMB No. 1545-0120`) and it would otherwise be
//!    transcribed as part of box 1's caption. The marker is asserted present in all seven extracts, so
//!    it is a reading of the documents rather than a taste.
//! 4. **The contiguity guard.** The numeric labels must run `1..=max` with no gap (a lettered box
//!    counts for its number), and lettered labels must run from `a`. This is the guard ON the
//!    enumerator, not the enumeration: it is what turns a blind spot into a red instead of a short
//!    list nobody counts. It has already earned itself — the first draft's run-splitter silently
//!    dropped four boxes of Form 1099-DIV and the gap is what showed it.
//!
//! ★ **An empty caption is a real answer**, not a failure: the 2025 Form W-2 prints box `9` shaded
//! with no caption at all, and boxes `12b`/`12c`/`12d` as bare labels beside a vertical *Code* rail.
//! Recording them with a reason is the point; dropping them because they carry no words is exactly the
//! "we forgot this box" defect.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// The last sentence of the *Attention* preamble every information return opens with. Occurs exactly
/// once per extract, and the box grid begins on the next line.
const PREAMBLE_END: &str = "1141, 1167, and 1179";

/// The Copy A footer. The box grid ends on the line before the first one.
const FACE_END: &str = "Cat. No.";

/// Page furniture that is never part of a box caption. See rule 3 above.
const FURNITURE: &str = "OMB No.";

/// One archived information return the interview transcribes boxes from.
pub struct DocumentAuthority {
    /// The IRS stem, e.g. `"f1099int"` — also the `design/forms/extract/` filename stem.
    pub stem: &'static str,
    /// The tax year the archived revision governs. **Six of the seven documents are `--2024`**: the
    /// 1099-INT/DIV/G family is continuous-use and `irs-prior` serves no `--2025` edition, so the
    /// revision in force for TY2025 is filed under the year of its own *Rev.* date.
    pub year: &'static str,
    /// The identically-numbered instructions booklet — **except the 1098-E**, whose instructions are
    /// the combined 1098-E/1098-T booklet `i1098et`; there is no `i1098e` document. The W-2's are
    /// `iw2w3`, and the 1099-INT's booklet is shared with the 1099-OID.
    pub instructions: &'static str,
}

/// The seven documents `SPEC_interview.md` R4 names, as archived by T2.
pub const DOCUMENTS: &[DocumentAuthority] = &[
    DocumentAuthority {
        stem: "fw2",
        year: "2025",
        instructions: "iw2w3",
    },
    DocumentAuthority {
        stem: "f1099int",
        year: "2024",
        instructions: "i1099int",
    },
    DocumentAuthority {
        stem: "f1099div",
        year: "2024",
        instructions: "i1099div",
    },
    DocumentAuthority {
        stem: "f1099g",
        year: "2024",
        instructions: "i1099g",
    },
    DocumentAuthority {
        stem: "f1099b",
        year: "2025",
        instructions: "i1099b",
    },
    DocumentAuthority {
        stem: "f1098",
        year: "2025",
        instructions: "i1098",
    },
    DocumentAuthority {
        stem: "f1098e",
        year: "2025",
        instructions: "i1098et",
    },
];

/// What the return does with one printed box. Exactly one per caption — R4's *"`collected(FieldId)`,
/// `refuse_if_nonzero(RefuseReason)` … or `not_read(reason)`"*, at the fidelity T2 can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxDecision {
    /// A field holds this box and its figure reaches a printed line. The string names the field.
    Collected(&'static str),
    /// A field holds this box **only in order to refuse on it** — nothing it carries reaches a line.
    RefuseIfNonzero(&'static str),
    /// No field holds it. The string is the reason, and it is the whole value of the entry: it is what
    /// distinguishes *"the return has no use for this box"* from *"we forgot this box"*.
    NotRead(&'static str),
}

/// One printed box, its caption as the form prints it, and the decision.
pub struct BoxEntry {
    pub stem: &'static str,
    /// The box's own label — `"1"`, `"2a"`, `"12b"`, `"a"`.
    pub label: &'static str,
    /// **VERBATIM** from `design/forms/extract/<stem>--<year>.txt`, label included, as the extract
    /// prints it on the label's own line. A caption the layout wraps is quoted to its first printed
    /// line, because the wrap is layout and quoting across it would quote text the page never prints
    /// contiguously.
    pub caption: &'static str,
    pub decision: BoxDecision,
}

/// ★★ **THE POPULATION — 115 boxes across seven documents, every caption read off the extract.**
///
/// The `Collected` strings name fields that exist at T2 (`crates/btctax-core/src/tax/return_inputs.rs`);
/// a `NotRead` whose reason begins *"T5"* / *"T9"* is a box a later task collects, and its reason names
/// the line it will reach so the schedule is legible rather than implied.
pub const BOXES: &[BoxEntry] = &[
    // ── Form W-2 (2025) — 29 boxes ────────────────────────────────────────────────────────────────
    BoxEntry { stem: "fw2", label: "a", caption: "a Employee’s social security number",
        decision: BoxDecision::Collected("header.taxpayer.ssn / header.spouse.ssn — the 1040 header prints it once for the return, not per W-2 row") },
    BoxEntry { stem: "fw2", label: "b", caption: "b Employer identification number (EIN)",
        decision: BoxDecision::Collected("W2.ein — the only thing that can answer §6413(c)'s 'more than one employer' test") },
    BoxEntry { stem: "fw2", label: "c", caption: "c Employer’s name, address, and ZIP code",
        decision: BoxDecision::Collected("W2.employer") },
    BoxEntry { stem: "fw2", label: "d", caption: "d Control number",
        decision: BoxDecision::NotRead("the employer's internal payroll number; no line of the return reads it") },
    BoxEntry { stem: "fw2", label: "e", caption: "e Employee’s first name and initial",
        decision: BoxDecision::Collected("header.taxpayer.name / header.spouse.name — printed once on the 1040 header") },
    BoxEntry { stem: "fw2", label: "f", caption: "f Employee’s address and ZIP code",
        decision: BoxDecision::Collected("header.address_street / address_city / address_state / address_zip") },
    BoxEntry { stem: "fw2", label: "1", caption: "1 Wages, tips, other compensation",
        decision: BoxDecision::Collected("W2.box1_wages → 1040 line 1a") },
    BoxEntry { stem: "fw2", label: "2", caption: "2 Federal income tax withheld",
        decision: BoxDecision::Collected("W2.box2_fed_withheld → 1040 line 25a") },
    BoxEntry { stem: "fw2", label: "3", caption: "3 Social security wages",
        decision: BoxDecision::Collected("W2.box3_ss_wages → the per-earner SS cap and the excess-SS credit") },
    BoxEntry { stem: "fw2", label: "4", caption: "4 Social security tax withheld",
        decision: BoxDecision::Collected("W2.box4_ss_withheld → Schedule 3 line 11, excess social security") },
    BoxEntry { stem: "fw2", label: "5", caption: "5 Medicare wages and tips",
        decision: BoxDecision::Collected("W2.box5_medicare_wages → Form 8959 Part I") },
    BoxEntry { stem: "fw2", label: "6", caption: "6 Medicare tax withheld",
        decision: BoxDecision::Collected("W2.box6_medicare_withheld → Form 8959 Part V → 1040 line 25c") },
    BoxEntry { stem: "fw2", label: "7", caption: "7 Social security tips",
        decision: BoxDecision::Collected("W2.box7_ss_tips → the §6413(c) wage total, and Schedule 1-A line 4a's tips") },
    BoxEntry { stem: "fw2", label: "8", caption: "8 Allocated tips",
        decision: BoxDecision::RefuseIfNonzero("W2.box8_allocated_tips — allocated tips are unreported income needing Form 4137; > 0 refuses") },
    BoxEntry { stem: "fw2", label: "9", caption: "9",
        decision: BoxDecision::NotRead("the 2025 revision prints box 9 shaded and CAPTIONLESS — there is no figure to collect; the box exists on the paper and is recorded so the census cannot silently gain a caption in a later revision") },
    BoxEntry { stem: "fw2", label: "10", caption: "10 Dependent care benefits",
        decision: BoxDecision::RefuseIfNonzero("W2.box10_dependent_care — dependent care benefits need Form 2441; > 0 refuses") },
    BoxEntry { stem: "fw2", label: "11", caption: "11 Nonqualified plans",
        decision: BoxDecision::NotRead("distributions from a nonqualified deferred compensation plan, already included in box 1 for income tax; the SSA reads it, no 1040 line does") },
    BoxEntry { stem: "fw2", label: "12a", caption: "12a See instructions for box 12",
        decision: BoxDecision::Collected("W2.box12 — a Vec<Box12Entry> of (code, amount), so all four printed slots are one repeating field") },
    BoxEntry { stem: "fw2", label: "12b", caption: "12b",
        decision: BoxDecision::Collected("W2.box12 — the second of the form's four printed slots; the label prints bare beside a vertical 'Code' rail") },
    BoxEntry { stem: "fw2", label: "12c", caption: "12c",
        decision: BoxDecision::Collected("W2.box12 — the third printed slot") },
    BoxEntry { stem: "fw2", label: "12d", caption: "12d",
        decision: BoxDecision::Collected("W2.box12 — the fourth printed slot") },
    BoxEntry { stem: "fw2", label: "13", caption: "13 Statutory",
        decision: BoxDecision::NotRead("T5: R4 collects the three box-13 checkboxes, of which 'Statutory employee' refuses StatutoryEmployeeW2 naming Schedule C line 1 — a checked box 13 sends box 1 to Schedule C, not to 1040 line 1a. No field holds it at T2") },
    BoxEntry { stem: "fw2", label: "14", caption: "14 Other",
        decision: BoxDecision::NotRead("free-text employer reporting; nothing reaches a line until the filer identifies the item, and the residual scope attestation covers what they cannot") },
    BoxEntry { stem: "fw2", label: "15", caption: "15 State",
        decision: BoxDecision::NotRead("the state's two-letter code and the employer's state ID number; the federal return prints neither") },
    BoxEntry { stem: "fw2", label: "16", caption: "16 State wages, tips, etc.",
        decision: BoxDecision::NotRead("state wages; no federal line reads a state wage figure") },
    BoxEntry { stem: "fw2", label: "17", caption: "17 State income tax",
        decision: BoxDecision::Collected("W2.box17_state_tax_withheld → Schedule A line 5a on the income-tax election") },
    BoxEntry { stem: "fw2", label: "18", caption: "18 Local wages, tips, etc.",
        decision: BoxDecision::NotRead("local wages; no federal line reads a local wage figure") },
    BoxEntry { stem: "fw2", label: "19", caption: "19 Local income tax",
        decision: BoxDecision::Collected("W2.box19_local_tax → Schedule A line 5a") },
    BoxEntry { stem: "fw2", label: "20", caption: "20 Locality name",
        decision: BoxDecision::NotRead("the locality's name; Schedule A line 5a takes the amount, never the locality") },

    // ── Form 1099-INT (Rev. January 2024) — 17 boxes ──────────────────────────────────────────────
    BoxEntry { stem: "f1099int", label: "1", caption: "1 Interest income",
        decision: BoxDecision::Collected("Form1099Int.box1_interest → Schedule B line 1 → 1040 line 2b") },
    BoxEntry { stem: "f1099int", label: "2", caption: "2 Early withdrawal penalty",
        decision: BoxDecision::Collected("Form1099Int.box2_early_withdrawal_penalty → Schedule 1 line 18") },
    BoxEntry { stem: "f1099int", label: "3", caption: "3 Interest on U.S. Savings Bonds and Treasury obligations",
        decision: BoxDecision::Collected("Form1099Int.box3_treasury_interest → 1040 line 2b") },
    BoxEntry { stem: "f1099int", label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::Collected("Form1099Int.box4_fed_withheld → 1040 line 25b") },
    BoxEntry { stem: "f1099int", label: "5", caption: "5 Investment expenses",
        decision: BoxDecision::NotRead("a miscellaneous itemized deduction, suspended for 2018–2025 by §67(g); no Schedule A line takes it") },
    BoxEntry { stem: "f1099int", label: "6", caption: "6 Foreign tax paid",
        decision: BoxDecision::Collected("Form1099Int.box6_foreign_tax → the §904(j) foreign tax credit election on Schedule 3 line 1") },
    BoxEntry { stem: "f1099int", label: "7", caption: "7 Foreign country or U.S. territory",
        decision: BoxDecision::NotRead("the country's name; the §904(j) election reads the AMOUNT in box 6 and Form 1116 — which would read the country — is out of scope") },
    BoxEntry { stem: "f1099int", label: "8", caption: "8 Tax-exempt interest",
        decision: BoxDecision::Collected("Form1099Int.box8_tax_exempt_interest → 1040 line 2a") },
    BoxEntry { stem: "f1099int", label: "9", caption: "9 Specified private activity bond",
        decision: BoxDecision::RefuseIfNonzero("Form1099Int.box9_private_activity_bond_amt — a Form 6251 line 2g AMT preference; > 0 refuses") },
    BoxEntry { stem: "f1099int", label: "10", caption: "10 Market discount",
        decision: BoxDecision::NotRead("T5: R4 collects it to Schedule B line 1 and the 1040 line 2b sum — i1040sb, 'Also include any accrued market discount that is includible in income'. Income, so the understatement direction; no field holds it at T2") },
    BoxEntry { stem: "f1099int", label: "11", caption: "11 Bond premium",
        decision: BoxDecision::NotRead("T5: R4 refuses on > 0 (AmortizableBondPremiumNotComputed), naming the amortizable-bond-premium adjustment and Pub. 550. A reduction, so refusing is both the conservative and the honest direction; no field holds it at T2") },
    BoxEntry { stem: "f1099int", label: "12", caption: "12 Bond premium on Treasury obligations",
        decision: BoxDecision::NotRead("T5: as box 11 — refuses on > 0 until the bond-premium adjustment is transcribed") },
    BoxEntry { stem: "f1099int", label: "13", caption: "13 Bond premium on tax-exempt bond",
        decision: BoxDecision::NotRead("T5: as box 11 — refuses on > 0 until the bond-premium adjustment is transcribed") },
    BoxEntry { stem: "f1099int", label: "14", caption: "14 Tax-exempt and tax credit",
        decision: BoxDecision::NotRead("the tax-exempt and tax credit bond CUSIP number — an identifier, not an amount") },
    BoxEntry { stem: "f1099int", label: "15", caption: "15 State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none") },
    BoxEntry { stem: "f1099int", label: "16", caption: "16 State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099int", label: "17", caption: "17 State tax withheld",
        decision: BoxDecision::NotRead("state income tax the payer withheld; Schedule A line 5a is collected from the W-2 and the filer's records, and no Form1099Int field holds this box") },

    // ── Form 1099-DIV (Rev. January 2024) — 22 boxes ──────────────────────────────────────────────
    BoxEntry { stem: "f1099div", label: "1a", caption: "1a Total ordinary dividends",
        decision: BoxDecision::Collected("Form1099Div.box1a_ordinary → 1040 line 3b (it INCLUDES box 1b)") },
    BoxEntry { stem: "f1099div", label: "1b", caption: "1b Qualified dividends",
        decision: BoxDecision::Collected("Form1099Div.box1b_qualified → 1040 line 3a, the preferential-rate slice") },
    BoxEntry { stem: "f1099div", label: "2a", caption: "2a Total capital gain distr.",
        decision: BoxDecision::Collected("Form1099Div.box2a_capgain_distr → Schedule D line 13") },
    BoxEntry { stem: "f1099div", label: "2b", caption: "2b Unrecap. Sec. 1250 gain",
        decision: BoxDecision::RefuseIfNonzero("Form1099Div.box2b_unrecap_1250 — the 25% rate group needs the Schedule D unrecaptured-gain worksheet; > 0 refuses") },
    BoxEntry { stem: "f1099div", label: "2c", caption: "2c Section 1202 gain",
        decision: BoxDecision::RefuseIfNonzero("Form1099Div.box2c_section_1202 — qualified small business stock exclusion; > 0 refuses") },
    BoxEntry { stem: "f1099div", label: "2d", caption: "2d Collectibles (28%) gain",
        decision: BoxDecision::RefuseIfNonzero("Form1099Div.box2d_collectibles_28 — the 28% rate group; > 0 refuses") },
    BoxEntry { stem: "f1099div", label: "2e", caption: "2e Section 897 ordinary dividends",
        decision: BoxDecision::NotRead("§897 (FIRPTA) reporting, which the instructions address to foreign persons; for a U.S. filer the amount is already inside box 1a and reaches no line of its own") },
    BoxEntry { stem: "f1099div", label: "2f", caption: "2f Section 897 capital gain",
        decision: BoxDecision::NotRead("§897 (FIRPTA) reporting for foreign persons; already inside box 2a for a U.S. filer") },
    BoxEntry { stem: "f1099div", label: "3", caption: "3 Nondividend distributions",
        decision: BoxDecision::NotRead("R4's decision: it reduces basis and does not reach a line this year; Pub. 550") },
    BoxEntry { stem: "f1099div", label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::Collected("Form1099Div.box4_fed_withheld → 1040 line 25b") },
    BoxEntry { stem: "f1099div", label: "5", caption: "5 Section 199A dividends",
        decision: BoxDecision::Collected("Form1099Div.box5_section_199a → the QBI deduction (Form 8995 line 6)") },
    BoxEntry { stem: "f1099div", label: "6", caption: "6 Investment expenses",
        decision: BoxDecision::NotRead("a miscellaneous itemized deduction, suspended for 2018–2025 by §67(g)") },
    BoxEntry { stem: "f1099div", label: "7", caption: "7 Foreign tax paid",
        decision: BoxDecision::Collected("Form1099Div.box7_foreign_tax → the §904(j) foreign tax credit election on Schedule 3 line 1") },
    BoxEntry { stem: "f1099div", label: "8", caption: "8 Foreign country or U.S. possession",
        decision: BoxDecision::NotRead("the country's name; the §904(j) election reads the AMOUNT in box 7") },
    BoxEntry { stem: "f1099div", label: "9", caption: "9 Cash liquidation distributions",
        decision: BoxDecision::NotRead("a liquidating distribution is a return of capital and then a Form 8949 disposition of the stock; no chain of this return reads the box") },
    BoxEntry { stem: "f1099div", label: "10", caption: "10 Noncash liquidation distributions",
        decision: BoxDecision::NotRead("as box 9 — a liquidating distribution, reached through basis and Form 8949, not through a 1099-DIV field") },
    BoxEntry { stem: "f1099div", label: "11", caption: "11 FATCA filing",
        decision: BoxDecision::NotRead("the FATCA filing requirement checkbox — a chapter 4 obligation of the PAYER; no line of the filer's return reads it") },
    BoxEntry { stem: "f1099div", label: "12", caption: "12 Exempt-interest dividends",
        decision: BoxDecision::Collected("Form1099Div.box12_exempt_interest_dividends → 1040 line 2a") },
    BoxEntry { stem: "f1099div", label: "13", caption: "13 Specified private activity",
        decision: BoxDecision::RefuseIfNonzero("Form1099Div.box13_private_activity_amt — specified private activity bond interest dividends, a Form 6251 AMT preference; > 0 refuses") },
    BoxEntry { stem: "f1099div", label: "14", caption: "14 State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none") },
    BoxEntry { stem: "f1099div", label: "15", caption: "15 State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099div", label: "16", caption: "16 State tax withheld",
        decision: BoxDecision::NotRead("state income tax the payer withheld; no Form1099Div field holds it") },

    // ── Form 1099-G (Rev. March 2024) — 12 boxes ──────────────────────────────────────────────────
    BoxEntry { stem: "f1099g", label: "1", caption: "1 Unemployment compensation",
        decision: BoxDecision::Collected("Form1099G.box1_unemployment → Schedule 1 line 7") },
    BoxEntry { stem: "f1099g", label: "2", caption: "2 State or local income tax",
        decision: BoxDecision::NotRead("T5: §5.2 adds Form1099G.box2_state_refund, and the RETURN-LEVEL itemized_prior_year gate decides Schedule 1 line 1 — No ⇒ blank by decision, Yes ⇒ refuse naming the State and Local Income Tax Refund Worksheet. No field holds it at T2") },
    BoxEntry { stem: "f1099g", label: "3", caption: "3 Box 2 amount is for tax year",
        decision: BoxDecision::NotRead("the tax year box 2's refund relates to; it is read by the State and Local Income Tax Refund Worksheet, which is not transcribed (owner Q1)") },
    BoxEntry { stem: "f1099g", label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::Collected("Form1099G.box4_fed_withheld → 1040 line 25b") },
    BoxEntry { stem: "f1099g", label: "5", caption: "5 RTAA payments",
        decision: BoxDecision::NotRead("Reemployment Trade Adjustment Assistance, Schedule 1 line 8z; btctax models no line 8z inflow and the residual scope attestation names what it cannot take") },
    BoxEntry { stem: "f1099g", label: "6", caption: "6 Taxable grants",
        decision: BoxDecision::NotRead("a taxable grant reaches Schedule 1 line 8z; btctax models no line 8z inflow") },
    BoxEntry { stem: "f1099g", label: "7", caption: "7 Agriculture payments",
        decision: BoxDecision::NotRead("Schedule F income; farm income is an excluded family (§2.2) and its census row refuses") },
    BoxEntry { stem: "f1099g", label: "8", caption: "8 Check if box 2 is",
        decision: BoxDecision::NotRead("the checkbox saying box 2 is trade or business income; it qualifies box 2, which is T5's") },
    BoxEntry { stem: "f1099g", label: "9", caption: "9 Market gain",
        decision: BoxDecision::NotRead("CCC loan market gain, Schedule F; farm income is an excluded family (§2.2)") },
    BoxEntry { stem: "f1099g", label: "10a", caption: "10a State",
        decision: BoxDecision::NotRead("the state's two-letter code; the federal return prints none") },
    BoxEntry { stem: "f1099g", label: "10b", caption: "10b State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099g", label: "11", caption: "11 State income tax withheld",
        decision: BoxDecision::NotRead("state income tax withheld; no Form1099G field holds it") },

    // ── Form 1099-B (2025) — 22 boxes ─────────────────────────────────────────────────────────────
    BoxEntry { stem: "f1099b", label: "1a", caption: "1a Description of property (Example: 100 sh. XYZ Co.)",
        decision: BoxDecision::NotRead("btctax takes the Schedule D line 1a/8a TOTALS, which the form's own instruction permits when basis was reported and there are no adjustments; a per-transaction description belongs on Form 8949, and a row needing one is refused (Form1099BNeedsForm8949)") },
    BoxEntry { stem: "f1099b", label: "1b", caption: "1b Date acquired",
        decision: BoxDecision::NotRead("per-transaction, and Schedule D lines 1a/8a carry no dates; a row needing Form 8949 is refused") },
    BoxEntry { stem: "f1099b", label: "1c", caption: "1c Date sold or disposed",
        decision: BoxDecision::NotRead("per-transaction, and Schedule D lines 1a/8a carry no dates; a row needing Form 8949 is refused") },
    BoxEntry { stem: "f1099b", label: "1d", caption: "1d Proceeds",
        decision: BoxDecision::Collected("Form1099B.short_term_proceeds / long_term_proceeds → Schedule D line 1a(d) / 8a(d)") },
    BoxEntry { stem: "f1099b", label: "1e", caption: "1e Cost or other basis",
        decision: BoxDecision::Collected("Form1099B.short_term_basis / long_term_basis → Schedule D line 1a(e) / 8a(e)") },
    BoxEntry { stem: "f1099b", label: "1f", caption: "1f Accrued market discount",
        decision: BoxDecision::NotRead("an ADJUSTMENT: a row carrying one fails basis_reported_and_no_adjustments, so the gate refuses Form1099BNeedsForm8949 rather than dropping the figure") },
    BoxEntry { stem: "f1099b", label: "1g", caption: "1g Wash sale loss disallowed",
        decision: BoxDecision::NotRead("an ADJUSTMENT: the row fails the 1a/8a gate and is refused to Form 8949") },
    BoxEntry { stem: "f1099b", label: "2", caption: "2 Short-term gain or loss",
        decision: BoxDecision::Collected("Form1099B's short_term_* vs long_term_* pair — the box decides WHICH Schedule D lines a row's totals reach") },
    BoxEntry { stem: "f1099b", label: "3", caption: "3 Check if proceeds from:",
        decision: BoxDecision::NotRead("collectibles or QOF proceeds; either is an adjustment case the gate refuses to Form 8949") },
    BoxEntry { stem: "f1099b", label: "4", caption: "4 Federal income tax withheld",
        decision: BoxDecision::NotRead("backup withholding on broker proceeds would reach 1040 line 25b; no Form1099B field holds it, so a filer with backup withholding forgoes a credit — the OVERSTATEMENT direction, announced rather than silent") },
    BoxEntry { stem: "f1099b", label: "5", caption: "5 Check if noncovered",
        decision: BoxDecision::NotRead("a noncovered security has no basis reported to the IRS, so the row fails the 1a/8a gate and is refused to Form 8949") },
    BoxEntry { stem: "f1099b", label: "6", caption: "6 Reported to IRS:",
        decision: BoxDecision::NotRead("gross versus net proceeds; either way box 1d is the figure Schedule D's total reads") },
    BoxEntry { stem: "f1099b", label: "7", caption: "7 Check if loss is not allowed",
        decision: BoxDecision::NotRead("an ADJUSTMENT (a loss disallowed because proceeds are less than the amount reported); the row is refused to Form 8949") },
    BoxEntry { stem: "f1099b", label: "8", caption: "8 Profit or (loss) realized in",
        decision: BoxDecision::NotRead("§1256 regulated futures and forward contracts, which reach Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", label: "9", caption: "9 Unrealized profit or (loss) on",
        decision: BoxDecision::NotRead("§1256 open contracts at the prior year end, Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", label: "10", caption: "10 Unrealized profit or (loss) on",
        decision: BoxDecision::NotRead("§1256 open contracts at this year end, Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", label: "11", caption: "11 Aggregate profit or (loss)",
        decision: BoxDecision::NotRead("the §1256 aggregate, Form 6781; an excluded family (§2.2)") },
    BoxEntry { stem: "f1099b", label: "12", caption: "12 Check if basis reported to",
        decision: BoxDecision::Collected("Form1099B.basis_reported_and_no_adjustments — the first half of the gate Schedule D lines 1a/8a require") },
    BoxEntry { stem: "f1099b", label: "13", caption: "13 Bartering",
        decision: BoxDecision::NotRead("barter exchange income, which reaches Schedule 1 line 8z or Schedule C; btctax models no line 8z inflow") },
    BoxEntry { stem: "f1099b", label: "14", caption: "14 State name",
        decision: BoxDecision::NotRead("the state's name; the federal return prints none") },
    BoxEntry { stem: "f1099b", label: "15", caption: "15 State identification no.",
        decision: BoxDecision::NotRead("the payer's state identification number; the federal return prints none") },
    BoxEntry { stem: "f1099b", label: "16", caption: "16 State tax withheld",
        decision: BoxDecision::NotRead("state income tax withheld; no Form1099B field holds it") },

    // ── Form 1098 (Rev. April 2025) — 11 boxes ────────────────────────────────────────────────────
    BoxEntry { stem: "f1098", label: "1", caption: "1 Mortgage interest received from payer(s)/borrower(s)",
        decision: BoxDecision::Collected("ScheduleAInputs.mortgage_interest_1098 → Schedule A line 8a; §5.2 replaces it with Form1098.box1_interest in T9") },
    BoxEntry { stem: "f1098", label: "2", caption: "2 Outstanding mortgage",
        decision: BoxDecision::NotRead("T9: Form1098.box2_outstanding_principal feeds the AGGREGATE, status-adjusted §163(h)(3)(B) ceiling check; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "3", caption: "3 Mortgage origination date",
        decision: BoxDecision::NotRead("T9: Form1098.box3_origination_date decides the $750,000 versus $1,000,000 ceiling by whether it precedes 2017-12-16; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "4", caption: "4 Refund of overpaid",
        decision: BoxDecision::NotRead("T9 (fold I3): > 0 REFUSES MortgageInterestRefundNotComputed naming Schedule 1 line 8z — i1098 is explicit that the refund is not netted against the deduction, so a figure held with no reader would understate; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "5", caption: "5 Mortgage insurance",
        decision: BoxDecision::NotRead("T9: mortgage insurance premiums reach Schedule A line 8d only if a final reinstates the §163(h)(3)(E) deduction; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "6", caption: "6 Points paid on purchase of principal residence",
        decision: BoxDecision::NotRead("T9: Form1098.box6_points is added to Schedule A line 8a with box 1; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "7", caption: "7 If address of property securing mortgage is the same",
        decision: BoxDecision::NotRead("T9: Form1098.box7_property_address_same_as_payer, the checkbox box 8's address answers to; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "8", caption: "8 Address or description of property securing mortgage (see",
        decision: BoxDecision::NotRead("T9: Form1098.box8_property_address; no field holds it at T2") },
    BoxEntry { stem: "f1098", label: "9", caption: "9 Number of properties securing the",
        decision: BoxDecision::NotRead("the count of properties one mortgage secures; no Schedule A line reads it, and the ceiling check reads box 2's principal") },
    BoxEntry { stem: "f1098", label: "10", caption: "10 Other",
        decision: BoxDecision::NotRead("T9: Form1098.box10_other — free-text lender reporting (real estate taxes are the common one), which reaches no line until the filer identifies the item") },
    BoxEntry { stem: "f1098", label: "11", caption: "11 Mortgage",
        decision: BoxDecision::NotRead("the mortgage ACQUISITION date — when the present lender acquired the loan, printed only on a transferred mortgage. The §163(h)(3)(B) ceiling test reads box 3, the ORIGINATION date, so this box reaches no line") },

    // ── Form 1098-E (2025) — 2 boxes ──────────────────────────────────────────────────────────────
    BoxEntry { stem: "f1098e", label: "1", caption: "1 Student loan interest received by lender",
        decision: BoxDecision::Collected("Schedule1Inputs.student_loan_interest_paid → Schedule 1 line 21; §5.2 replaces it with Form1098E.box1_interest in T5") },
    BoxEntry { stem: "f1098e", label: "2", caption: "2 Check if box 1 does not include loan origination fees",
        decision: BoxDecision::NotRead("a qualifier on box 1 for loans made before September 1, 2004: it says the lender left origination fees and capitalized interest OUT. Schedule 1 line 21 takes the box-1 amount as printed, and btctax does not compute the omitted fees") },
];

// ── The enumerator ────────────────────────────────────────────────────────────────────────────────

/// The `-layout` lines of the form's own box grid: strictly between the *Attention* preamble's last
/// sentence and Copy A's `Cat. No.` footer.
pub fn face_block(text: &str) -> Result<Vec<&str>, String> {
    let lines: Vec<&str> = text.lines().collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(PREAMBLE_END))
        .map(|(i, _)| i)
        .collect();
    if starts.len() != 1 {
        return Err(format!(
            "the preamble marker {PREAMBLE_END:?} occurs {} times, expected exactly 1 — the face \
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

/// Is `s` exactly a box label — `1`, `2a`, `12b`, or one of the W-2's lettered boxes `a`..`f`?
fn is_label(s: &str) -> bool {
    let b = s.as_bytes();
    match b.len() {
        1 => b[0].is_ascii_digit() || (b'a'..=b'f').contains(&b[0]),
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
pub fn printed_boxes(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for line in face_block(text)? {
        let runs: Vec<&str> = line
            .split("  ")
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .collect();
        for (i, run) in runs.iter().enumerate() {
            if is_label(run) {
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
    }
    contiguous(&found)?;
    Ok(found)
}

/// ★★ **THE GUARD ON THE ENUMERATOR** — the numeric labels must run `1..=max` with no gap, and any
/// lettered labels must run from `a`. A reader that drops a box otherwise returns a shorter list that
/// still looks plausible; this turns that into a red. It has already caught one: the first
/// run-splitter lost Form 1099-DIV boxes 3, 4, 5 and 6, which print with the label in its own column.
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

fn extract_path(root: &Path, stem: &str, year: &str) -> PathBuf {
    root.join(format!("design/forms/extract/{stem}--{year}.txt"))
}

/// `xtask box-census` — the operator-facing run.
pub fn run() -> Result<(), String> {
    let root = repo_root();
    let mut total = 0usize;
    let mut failures = Vec::new();
    for doc in DOCUMENTS {
        let path = extract_path(&root, doc.stem, doc.year);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let printed = printed_boxes(&text).map_err(|e| format!("{}: {e}", doc.stem))?;
        let entries: Vec<(&str, &str)> = BOXES
            .iter()
            .filter(|b| b.stem == doc.stem)
            .map(|b| (b.label, b.caption))
            .collect();
        let (mut collected, mut refusing, mut unread) = (0, 0, 0);
        for b in BOXES.iter().filter(|b| b.stem == doc.stem) {
            match b.decision {
                BoxDecision::Collected(_) => collected += 1,
                BoxDecision::RefuseIfNonzero(_) => refusing += 1,
                BoxDecision::NotRead(_) => unread += 1,
            }
        }
        match verdict(&printed, &entries) {
            Ok(()) => println!(
                "  {}--{} ({} instructions): {} boxes — {collected} collected, {refusing} \
                 refuse-if-nonzero, {unread} not read",
                doc.stem,
                doc.year,
                doc.instructions,
                printed.len()
            ),
            Err(e) => failures.push(format!("{}--{}:\n  {e}", doc.stem, doc.year)),
        }
        total += printed.len();
    }
    if !failures.is_empty() {
        return Err(format!(
            "the box census failed for {} document(s):\n{}",
            failures.len(),
            failures.join("\n")
        ));
    }
    println!(
        "box-census OK: {total} printed boxes across {} archived information returns, every one \
         decided",
        DOCUMENTS.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(doc: &DocumentAuthority) -> String {
        let p = extract_path(&repo_root(), doc.stem, doc.year);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
    }

    /// ★★★ **THE CENSUS ITSELF** — every printed box decided, every entry a box the form prints,
    /// every caption verbatim. This is R4's kill in both directions at once.
    #[test]
    fn every_printed_box_carries_exactly_one_entry() {
        let mut total = 0usize;
        let mut failures = Vec::new();
        for doc in DOCUMENTS {
            let printed = printed_boxes(&read(doc)).unwrap_or_else(|e| panic!("{}: {e}", doc.stem));
            let entries: Vec<(&str, &str)> = BOXES
                .iter()
                .filter(|b| b.stem == doc.stem)
                .map(|b| (b.label, b.caption))
                .collect();
            if let Err(e) = verdict(&printed, &entries) {
                failures.push(format!("{}--{}:\n  {e}", doc.stem, doc.year));
            }
            total += printed.len();
        }
        assert!(
            failures.is_empty(),
            "the box census failed for {} of {} documents:\n{}",
            failures.len(),
            DOCUMENTS.len(),
            failures.join("\n")
        );
        // A census that walked nothing would pass by finding nothing — the F4 shape.
        assert_eq!(
            total,
            BOXES.len(),
            "every entry must correspond to a printed box and vice versa"
        );
        assert!(
            total >= 100,
            "the census walked only {total} boxes across {} documents — it must actually read the \
             forms",
            DOCUMENTS.len()
        );
        eprintln!(
            "box census: {total} boxes across {} documents",
            DOCUMENTS.len()
        );
    }

    /// ★ Every entry names a document the registry knows, so a typo'd stem cannot park an entry
    /// where no extract will ever check it.
    #[test]
    fn every_entry_belongs_to_an_archived_document() {
        let known: BTreeSet<&str> = DOCUMENTS.iter().map(|d| d.stem).collect();
        for b in BOXES {
            assert!(
                known.contains(b.stem),
                "box entry {}/{} names stem {:?}, which is in no DOCUMENTS row — nothing would ever \
                 check it",
                b.stem,
                b.label,
                b.stem
            );
        }
    }

    /// ★ The three furniture/bound markers are the DOCUMENTS' own text, so the enumerator's bounds
    /// are a reading of the forms rather than a convention we assert about them.
    #[test]
    fn the_enumerator_markers_are_present_in_every_extract() {
        for doc in DOCUMENTS {
            let text = read(doc);
            for marker in [PREAMBLE_END, FACE_END, FURNITURE] {
                assert!(
                    text.contains(marker),
                    "{}--{} does not contain {marker:?}; the enumerator's bounds are not this \
                     document's own furniture and the face block is being guessed",
                    doc.stem,
                    doc.year
                );
            }
        }
    }

    /// ★★ Both halves of the archive obligation: the extract AND the provenance note are on disk for
    /// every document the census reads. Without the note the extract cannot be reproduced, and
    /// "archived" would mean a text file nobody can re-derive.
    #[test]
    fn every_document_has_its_extract_and_its_note() {
        let root = repo_root();
        for doc in DOCUMENTS {
            assert!(
                extract_path(&root, doc.stem, doc.year).is_file(),
                "{}--{}: no committed extract",
                doc.stem,
                doc.year
            );
            for stem in [doc.stem, doc.instructions] {
                let note = root.join(format!(
                    "design/forms/{}/{stem}--{}.pdf.txt",
                    doc.year, doc.year
                ));
                assert!(
                    note.is_file(),
                    "{stem}--{}: no provenance note at {} — the PDF is gitignored, so without the \
                     note the text layer cannot be reproduced",
                    doc.year,
                    note.display()
                );
            }
        }
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
        let boxes = printed_boxes(good).expect("the baseline must enumerate");
        assert_eq!(boxes.len(), 3, "baseline: {boxes:?}");

        let gapped = good.replace("2 Early withdrawal penalty\n", "");
        let err = printed_boxes(&gapped).expect_err("a dropped box must RED, not shorten the list");
        assert!(err.contains("not contiguous"), "wrong reason: {err}");

        let unbounded = good.replace("Cat. No. 14410K", "");
        let err = printed_boxes(&unbounded).expect_err("an unbounded face block must RED");
        assert!(err.contains("Copy A never closes"), "wrong reason: {err}");
    }
}
