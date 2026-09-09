//! ★★★ **R3 — THE DOCUMENT CENSUS.** One TRI-STATE per document type, stored as class-(A)
//! declarations (`SPEC_interview.md` R3 / §5.1).
//!
//! **The defect it closes.** `w2s: Vec<W2>` empty means either *no W-2* or *never asked* — the
//! answered-ness trap at the money level. Two blanks that look identical on the printed page and are
//! not the same thing (`blank-is-the-normal-case`). The census row makes them distinct:
//!
//! | value | meaning | verdict |
//! |---|---|---|
//! | `None` | not asked / not yet received | **blocks commit** (class A). A broker that has not mailed by February is *unanswered*, never *none* |
//! | `Some(false)` | none | the section is non-live and the type's `Vec` **must** be empty |
//! | `Some(true)` | one or more | the section is live; zero rows refuses **for a kind that requires transcription** — see the two predicates below |
//!
//! And for an **unsupported** type (`SPEC_interview.md` §2.2) `Some(true)` refuses with that family's
//! own exit sentence — *"a filer cannot answer no to a category they were never shown"*, so the row
//! exists even where btctax can do nothing with a yes.
//!
//! ★ **Every row is a [`crate::tax::questions::FormQuestion`]**, so `screen_inputs` refuses a live
//! `None`, `income answer` asks it, and the Declarations adapter renders it — zero new plumbing, and
//! exactly one liveness predicate per row.
//!
//! ★★★ **"Did you receive one" and "how many rows did you transcribe" are TWO questions, and the
//! census keeps two predicates for them.** [`declared_rows`] answers *how many rows of this document
//! does the return carry* — `Some(n)` for every kind with a `Vec` on `ReturnInputs`, `None` where
//! there is no section whose rows could be counted at all. [`requires_transcription`] answers the
//! different question *must a declared document of this kind appear as rows* — and for two kinds the
//! honest answer is **no**, because the form itself says so:
//!
//! - a **1099-B** whose transactions all go on Form 8949 leaves the summary line blank by the form's
//!   own printed instruction, and for btctax's own population — a crypto filer whose dispositions
//!   are in the ledger and print on Form 8949 / Schedule D — zero `[[b_1099]]` rows IS the correct
//!   return;
//! - a **1099-G** reporting only box 2 (a prior-year state refund, the commonest 1099-G there is)
//!   has no field to be transcribed into: `Form1099G` models box 1 and box 4, and box 2 reaches the
//!   return today through the `sch1.state_refund_taxable` scalar. **T5** adds the box and the screen.
//!
//! Collapsing the two into one predicate refused both of those filers for answering truthfully,
//! which is the T3 seam review's I2/I3. The DECLARATION is still required of every live row, and the
//! contradiction rule (`Some(false)` beside transcribed rows) still runs on all five `Vec`s — only
//! the *demand for rows* is now the narrower of the two.

use serde::{Deserialize, Serialize};

/// One row of the census — a document TYPE, never a document.
///
/// ★ A fieldless enum with an exhaustive [`Self::ALL`], so a new row is a compile error in every
/// `match` below and in the classifier until a human places it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRow {
    /// Form W-2 — wages. The one row with a transcription section at HEAD (`w2s`).
    W2,
    /// Form 1099-INT — interest.
    Int1099,
    /// Form 1099-DIV — dividends.
    Div1099,
    /// Form 1099-B — broker proceeds (securities totals).
    B1099,
    /// Form 1099-G — state refunds and unemployment.
    G1099,
    /// Form 1098 — mortgage interest.
    Form1098,
    /// Form 1098-E — student loan interest.
    Form1098e,
    /// Form 1099-SA — distributions from an HSA, Archer MSA or MA MSA (T16).
    Sa1099,
    /// Form 5498-SA — HSA / Archer MSA / MA MSA contribution and FMV information (T16).
    Sa5498,
    /// Form 1099-R — IRA / pension / annuity distributions (§2.2).
    R1099,
    /// Form SSA-1099 / RRB-1099 — Social Security and railroad retirement (§2.2).
    Ssa1099,
    /// Form 1099-NEC / 1099-MISC / 1099-K (§2.2).
    NecMiscK1099,
    /// Schedule K-1, any flavour (§2.2).
    K1,
    /// Rental real estate or royalties — Schedule E (§2.2).
    ScheduleERental,
    /// Form 1099-S — proceeds from a real-estate transaction (§2.2).
    S1099,
    /// Form 1099-OID — original issue discount (§2.2).
    Oid1099,
    /// Form W-2G — gambling winnings (§2.2).
    W2g,
    /// Form 1099-C — cancellation of debt (§2.2).
    C1099,
    /// Form 1095-A — Marketplace health coverage (§2.2).
    A1095,
    /// Form 1098-T — tuition (§2.2).
    T1098,
}

impl DocumentRow {
    /// Every row, in census order. The completeness tests iterate this; the exhaustive `match`es
    /// below make a new variant a compile error until it is listed.
    pub const ALL: &'static [DocumentRow] = &[
        DocumentRow::W2,
        DocumentRow::Int1099,
        DocumentRow::Div1099,
        DocumentRow::B1099,
        DocumentRow::G1099,
        DocumentRow::Form1098,
        DocumentRow::Form1098e,
        DocumentRow::Sa1099,
        DocumentRow::Sa5498,
        DocumentRow::R1099,
        DocumentRow::Ssa1099,
        DocumentRow::NecMiscK1099,
        DocumentRow::K1,
        DocumentRow::ScheduleERental,
        DocumentRow::S1099,
        DocumentRow::Oid1099,
        DocumentRow::W2g,
        DocumentRow::C1099,
        DocumentRow::A1095,
        DocumentRow::T1098,
    ];

    /// The IRS designation of the document this row asks about — **the filer's own words for the
    /// piece of paper in their hand**, used in the prompt and named in every refusal.
    ///
    /// ★ Separate from [`Self::exit_sentence`] on purpose: §2.2's sentences are quoted VERBATIM and
    /// two of them (SSA-1099, W-2G) name no form number at all, so a refusal built only from the
    /// sentence could not tell the filer which of their answers produced it.
    #[must_use]
    pub const fn designation(self) -> &'static str {
        match self {
            DocumentRow::W2 => "Form W-2",
            DocumentRow::Int1099 => "Form 1099-INT",
            DocumentRow::Div1099 => "Form 1099-DIV",
            DocumentRow::B1099 => "Form 1099-B",
            DocumentRow::G1099 => "Form 1099-G",
            DocumentRow::Form1098 => "Form 1098",
            DocumentRow::Form1098e => "Form 1098-E",
            DocumentRow::Sa1099 => "Form 1099-SA",
            DocumentRow::Sa5498 => "Form 5498-SA",
            DocumentRow::R1099 => "Form 1099-R",
            DocumentRow::Ssa1099 => "Form SSA-1099 or RRB-1099",
            DocumentRow::NecMiscK1099 => "Form 1099-NEC, 1099-MISC or 1099-K",
            DocumentRow::K1 => "Schedule K-1",
            DocumentRow::ScheduleERental => "a rental or royalty statement (Schedule E)",
            DocumentRow::S1099 => "Form 1099-S",
            DocumentRow::Oid1099 => "Form 1099-OID",
            DocumentRow::W2g => "Form W-2G",
            DocumentRow::C1099 => "Form 1099-C",
            DocumentRow::A1095 => "Form 1095-A",
            DocumentRow::T1098 => "Form 1098-T",
        }
    }

    /// ★★★ **§2.2's exit sentence, VERBATIM** — what a filer who holds this document is told.
    ///
    /// `None` for the five rows btctax can hold the rows for today (`W2` and the four 1099 families
    /// `income import` fills) and for the two whose amount is collected by a scalar (`Form1098`,
    /// `Form1098e`) — the latter are not live, so no answer of theirs can reach a refusal. A row
    /// with no exit sentence is not an excluded family, and telling its filer to leave would be
    /// false.
    #[must_use]
    pub const fn exit_sentence(self) -> Option<&'static str> {
        Some(match self {
            // ★ Transcribable TODAY, so no exit: the W-2 and — since T5 — the four 1099 families,
            //   each through its own form section as well as `income import`. A row btctax can hold
            //   the rows for is not an excluded family, and telling its filer to leave would be
            //   false.
            DocumentRow::W2
            | DocumentRow::Int1099
            | DocumentRow::Div1099
            | DocumentRow::B1099
            | DocumentRow::G1099 => return None,
            // ★ T5 — the 1098-E has its own section now, so it is transcribable and no exit is
            //   owed. `form_1098` is still not live (its amount is a scalar until T9), so no answer
            //   of its can reach a refusal either.
            DocumentRow::Form1098 | DocumentRow::Form1098e => return None,
            // ★ T16 — both HSA information returns have their own section, so neither is an
            //   excluded family and neither is owed an exit.
            DocumentRow::Sa1099 | DocumentRow::Sa5498 => return None,
            // ── §2.2, verbatim. ─────────────────────────────────────────────────────────────────
            DocumentRow::R1099 => {
                "btctax cannot take a Form 1099-R for this year: Form 1040 lines 4a–5b and the \
                 Simplified Method are not built (T14, on S2). File with a preparer, or wait for T14."
            }
            DocumentRow::Ssa1099 => {
                "btctax cannot take Social Security or railroad retirement benefits: the Social \
                 Security Benefits Worksheet is not built (T14, on S2)."
            }
            DocumentRow::NecMiscK1099 => {
                "btctax cannot take a Form 1099-NEC, 1099-MISC or 1099-K this year. Box 1 / \
                 non-employee compensation is Schedule C income and Schedule C Part II is not built; \
                 1099-MISC box 3 — prizes, awards, research-study pay — is Schedule 1 line 8z \
                 income, which is not built either (T13, on S2). A crypto-only Schedule C still \
                 fills from the ledger."
            }
            DocumentRow::K1 => {
                "btctax cannot take a Schedule K-1: partnership, S-corporation, estate and trust \
                 items reach nothing. A preparer is the exit for this year."
            }
            DocumentRow::ScheduleERental => {
                "btctax has no Schedule E. A rental or royalty is a preparer's return for this year."
            }
            DocumentRow::S1099 => {
                "btctax does not compute a sale of real property. Form 8949 and Schedule D are the \
                 exit for a vacant lot, an inherited house or a rental; if it was your main home, \
                 Form 8949 code H and the Pub. 523 worksheet."
            }
            DocumentRow::Oid1099 => {
                "btctax takes Form 1099-INT only; a 1099-OID's boxes differ (box 1 OID, box 8/11 \
                 tax-exempt). Enter with a preparer or wait for its screen."
            }
            DocumentRow::W2g => "btctax cannot take gambling winnings or losses.",
            DocumentRow::C1099 => "btctax cannot take canceled debt (Form 982 is not built).",
            DocumentRow::A1095 => {
                "btctax cannot reconcile the premium tax credit (Form 8962 is not built)."
            }
            DocumentRow::T1098 => {
                "btctax cannot take education expenses (Form 8863 is not built)."
            }
        })
    }

    /// The registry identity of this row's question. The two are 1:1 by construction: a census row
    /// with no `FormQuestion` would never be asked, and a `Doc*` question with no row would have no
    /// leaf to write.
    #[must_use]
    pub const fn question_id(self) -> crate::tax::questions::QuestionId {
        use crate::tax::questions::QuestionId as Q;
        match self {
            DocumentRow::W2 => Q::DocW2,
            DocumentRow::Int1099 => Q::DocInt1099,
            DocumentRow::Div1099 => Q::DocDiv1099,
            DocumentRow::B1099 => Q::DocB1099,
            DocumentRow::G1099 => Q::DocG1099,
            DocumentRow::Form1098 => Q::DocForm1098,
            DocumentRow::Form1098e => Q::DocForm1098e,
            DocumentRow::Sa1099 => Q::DocSa1099,
            DocumentRow::Sa5498 => Q::DocSa5498,
            DocumentRow::R1099 => Q::DocR1099,
            DocumentRow::Ssa1099 => Q::DocSsa1099,
            DocumentRow::NecMiscK1099 => Q::DocNecMiscK1099,
            DocumentRow::K1 => Q::DocK1,
            DocumentRow::ScheduleERental => Q::DocScheduleERental,
            DocumentRow::S1099 => Q::DocS1099,
            DocumentRow::Oid1099 => Q::DocOid1099,
            DocumentRow::W2g => Q::DocW2g,
            DocumentRow::C1099 => Q::DocC1099,
            DocumentRow::A1095 => Q::DocA1095,
            DocumentRow::T1098 => Q::DocT1098,
        }
    }

    /// ★★★ **How a filer ENTERS this document today** — named in the
    /// [`DocumentDeclaredNotTranscribed`] refusal, because *"you declared one and none is
    /// transcribed"* is only half a message: the other half is where to put it.
    ///
    /// The W-2 has a screen. The four 1099 families do not yet — **T5** builds them — but their rows
    /// exist on `ReturnInputs` and `income import` fills them from the TOML wire today, so the
    /// honest instruction names that route and says which task replaces it. `None` for a row that
    /// cannot be transcribed at all (the §2.2 families refuse on `Yes` before this is ever read, and
    /// the two scalar-shadowed rows are not live).
    ///
    /// [`DocumentDeclaredNotTranscribed`]: crate::tax::return_refuse::RefuseReason::DocumentDeclaredNotTranscribed
    #[must_use]
    pub const fn entry_route(self) -> Option<&'static str> {
        Some(match self {
            DocumentRow::W2 => {
                "enter it in the W-2 section of the tax-inputs form, or as a `[[w2s]]` table through \
                 `btctax income import`"
            }
            DocumentRow::Int1099 => {
                "enter it in the Form 1099-INT section of the tax-inputs form, or as an \
                 `[[int_1099]]` table through `btctax income import` — Form 1040 lines 2a/2b and \
                 Schedule B line 1 read the rows"
            }
            DocumentRow::Div1099 => {
                "enter it in the Form 1099-DIV section of the tax-inputs form, or as a \
                 `[[div_1099]]` table through `btctax income import` — Form 1040 lines 3a/3b and \
                 Schedule B line 5 read the rows"
            }
            // ★★★ NOT "enter the rows". A `[[b_1099]]` row is the Schedule D line 1a/8a SUMMARY
            //     option, and `form_1099b_gains` is ADDED to the ledger's `capital_net` — so
            //     telling a crypto filer whose dispositions are already in the ledger to enter
            //     their exchange 1099-B would double-count every gain. The form's own instruction
            //     is the honest route, and it is a blank.
            DocumentRow::B1099 => {
                "leave the summary blank if the transactions are already on this return — your \
                 ledger's dispositions print per transaction on Form 8949 and carry to Schedule D, \
                 which is the Form 1099-B's own printed option (\"if you choose to report all these \
                 transactions on Form 8949, leave this line blank and go to line 1b\"). A \
                 `[[b_1099]]` table through `btctax income import` is for the Schedule D line \
                 1a/8a SUMMARY option only, and entering one beside ledger dispositions would \
                 count the same gains twice"
            }
            // ★ T5 gave box 2 a field, so the route is no longer split: every 1099-G a filer holds
            //   is transcribed as a row, and `itemized_prior_year` decides §111(a) on box 2.
            DocumentRow::G1099 => {
                "enter it in the Form 1099-G section of the tax-inputs form, or as a `[[g_1099]]` \
                 table through `btctax income import` — box 1 (unemployment compensation) reaches \
                 Schedule 1 line 7, and box 2 (a state or local income tax refund) reaches Schedule \
                 1 line 1 through the prior-year-itemized question"
            }
            DocumentRow::Form1098e => {
                "enter it in the Form 1098-E section of the tax-inputs form, or as a \
                 `[[form_1098e]]` table through `btctax income import` — Schedule 1 line 21 reads \
                 the SUM of the rows' box 1 (§221)"
            }
            DocumentRow::Sa1099 => {
                "enter it in the Form 1099-SA section of the tax-inputs form, or as an `[[sa_1099]]` \
                 table through `btctax income import` — Form 8889 line 14a reads the SUM of the \
                 rows' box 1, and Part II decides how much of it is taxable"
            }
            DocumentRow::Sa5498 => {
                "enter it in the Form 5498-SA section of the tax-inputs form, or as an `[[sa_5498]]` \
                 table through `btctax income import` — no line of Form 8889 sums it, so it is \
                 transcribed so you can CHECK the contributions you entered on line 2 against what \
                 your trustee reported"
            }
            _ => return None,
        })
    }

    /// The census question the filer is asked, phrased as the **form's own attribution** (R3): the
    /// document says who should send it, so the prompt says so too.
    #[must_use]
    pub const fn prompt(self) -> &'static str {
        match self {
            DocumentRow::W2 => {
                "Did you receive one or more Form W-2 (every employer must send you one — Form 1040 \
                 line 1a instructions)?"
            }
            DocumentRow::Int1099 => {
                "Did you receive one or more Form 1099-INT (each payer should send you one — Form \
                 1040 line 2b instructions)?"
            }
            DocumentRow::Div1099 => {
                "Did you receive one or more Form 1099-DIV (each payer should send you one — Form \
                 1040 line 3b instructions)?"
            }
            DocumentRow::B1099 => {
                "Did you receive one or more Form 1099-B (each broker should send you one — Form \
                 8949 and Schedule D instructions)?"
            }
            DocumentRow::G1099 => {
                "Did you receive one or more Form 1099-G (a state that refunded income tax, or paid \
                 unemployment compensation, should send you one — Schedule 1 line 1 and line 7 \
                 instructions)?"
            }
            DocumentRow::Form1098 => {
                "Did you receive one or more Form 1098 (your mortgage lender should send you one — \
                 Schedule A line 8a instructions)?"
            }
            DocumentRow::Form1098e => {
                "Did you receive one or more Form 1098-E (your student loan servicer should send you \
                 one — Schedule 1 line 21 instructions)?"
            }
            DocumentRow::Sa1099 => {
                "Did you receive one or more Form 1099-SA (the trustee of a health savings account \
                 must send you one for any distribution — Form 8889 line 14a instructions)?"
            }
            // ★★★ **FR-107 (journey walk finding #6) — THE ONE ROW WHOSE y/n WAS NOT ANSWERABLE.**
            //
            //     Every other row asks about a document the filer either has or will not get. Form
            //     5498-SA is neither: the trustee's own furnishing deadline falls AFTER the filing
            //     deadline (HSA contributions run to the return's due date), so an early filer can
            //     never truthfully answer *Yes* — yet *"I have not received it"* and *"I will never
            //     receive one"* are the same `No`, and the walk answered `n` calling it "defensible
            //     but not literally true".
            //
            // ★★ **Reworded rather than given the census's `None` tri-state, and the reason is that
            //    the tri-state would BRICK this row.** `None` means *not asked / not yet received*
            //    and BLOCKS commit (this module's own table) — which is right for a broker who has
            //    not mailed by February and will mail in March. Here the form cannot arrive before
            //    the return is due, so *not yet received* would block a correct return until June,
            //    permanently, for every early filer. The honest fix is to ask a question whose
            //    answer the filer HAS: what is in their hands today.
            //
            // ★ The timing clause is the instruction's own sentence, VERBATIM and year-free
            //   (`i1099sa--2025.txt`, *Statements to Participants*), pinned by
            //   `xtask prompt-check`. It also states the consequence, because a filer who reads
            //   "No" as "I am filing without a document I need" would go looking for one they
            //   cannot get: no line of Form 8889 sums any box of this form.
            //
            // ★ The timing comes BEFORE the question, not after it, for two reasons that happen to
            //   agree: the row's own invariants require a prompt that ENDS in a question mark and
            //   cites its attributing instruction (`every_prompt_names_the_document_and_is_a_question`),
            //   and a filer who is told the deadline before being asked answers once instead of
            //   answering and then wondering.
            DocumentRow::Sa5498 => {
                "Form 5498-SA is furnished AFTER the filing deadline: the instructions for Forms \
                 1099-SA and 5498-SA tell the trustee \"you must provide a statement to the \
                 participant (generally Copy B) by June 1\", and no line of Form 8889 reads this \
                 form — so if it has not arrived, answer No, and nothing on your return changes. \
                 Do you have one or more Form 5498-SA IN HAND for this year (the trustee of a \
                 health savings account sends one reporting the year's contributions and the \
                 account's fair market value)?"
            }
            DocumentRow::R1099 => {
                "Did you receive one or more Form 1099-R (the payer of an IRA, pension or annuity \
                 distribution must send you one — Form 1040 lines 4a–5b instructions)?"
            }
            DocumentRow::Ssa1099 => {
                "Did you receive a Form SSA-1099 or RRB-1099 (the Social Security Administration or \
                 the Railroad Retirement Board sends one to every beneficiary — Form 1040 lines \
                 6a–6c instructions)?"
            }
            DocumentRow::NecMiscK1099 => {
                "Did you receive one or more Form 1099-NEC, Form 1099-MISC or Form 1099-K (a payer, \
                 or a payment settlement entity, should send you one — Schedule 1 lines 3 and 8z \
                 instructions)?"
            }
            DocumentRow::K1 => {
                "Did you receive one or more Schedule K-1 (a partnership, S corporation, estate or \
                 trust must send one to every partner, shareholder or beneficiary — Schedule 1 line \
                 5 instructions)?"
            }
            DocumentRow::ScheduleERental => {
                "Did you receive rent or royalties from real estate or from property you own \
                 (reported on Schedule E — Schedule 1 line 5 instructions)?"
            }
            DocumentRow::S1099 => {
                "Did you receive a Form 1099-S (the closing agent for a sale or exchange of real \
                 estate should send you one — Form 8949 and Schedule D instructions)?"
            }
            DocumentRow::Oid1099 => {
                "Did you receive one or more Form 1099-OID (the issuer of a discounted bond should \
                 send you one — Form 1040 line 2a/2b instructions)?"
            }
            DocumentRow::W2g => {
                "Did you receive one or more Form W-2G (a payer of gambling winnings must send you \
                 one — Schedule 1 line 8b instructions)?"
            }
            DocumentRow::C1099 => {
                "Did you receive one or more Form 1099-C (a lender that canceled a debt must send \
                 you one — Schedule 1 line 8c instructions)?"
            }
            DocumentRow::A1095 => {
                "Did you receive a Form 1095-A (the Health Insurance Marketplace sends one to every \
                 enrollee — Schedule 2 line 1a and Schedule 3 line 9 instructions)?"
            }
            DocumentRow::T1098 => {
                "Did you receive a Form 1098-T (an eligible educational institution must send you \
                 one — Schedule 3 line 3 instructions)?"
            }
        }
    }
}

/// ★★★ **`ReturnInputs.documents`** — one `Option<bool>` per document type (§5.1).
///
/// Every field is `#[serde(default)]`: absence on the wire is *never asked*, which is a lawful state
/// the screen then refuses (§4.3). A `Default` is all-`None`, which is the honest starting point —
/// a fresh return has been asked nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentCensus {
    /// Form W-2 — wages, Form 1040 line 1a.
    #[serde(default)]
    pub w2: Option<bool>,
    /// Form 1099-INT — interest, Form 1040 line 2a/2b and Schedule B line 1.
    #[serde(default)]
    pub int_1099: Option<bool>,
    /// Form 1099-DIV — dividends, Form 1040 lines 3a/3b and Schedule B line 5.
    #[serde(default)]
    pub div_1099: Option<bool>,
    /// Form 1099-B — broker proceeds, Schedule D lines 1a/8a.
    #[serde(default)]
    pub b_1099: Option<bool>,
    /// Form 1099-G — state refunds (Schedule 1 line 1) and unemployment (Schedule 1 line 7).
    #[serde(default)]
    pub g_1099: Option<bool>,
    /// Form 1098 — mortgage interest, Schedule A line 8a. **Not live until T9** replaces the
    /// `schedule_a.mortgage_interest_1098` scalar; see [`row_is_live`].
    #[serde(default)]
    pub form_1098: Option<bool>,
    /// Form 1098-E — student loan interest, Schedule 1 line 21. **Live since T5**, which replaced
    /// the `sch1.student_loan_interest_paid` scalar with `form_1098e` rows; see [`row_is_live`].
    #[serde(default)]
    pub form_1098e: Option<bool>,
    /// Form 1099-SA — HSA distributions, Form 8889 Part II (T16).
    #[serde(default)]
    pub sa_1099: Option<bool>,
    /// Form 5498-SA — HSA contribution and FMV information (T16).
    #[serde(default)]
    pub sa_5498: Option<bool>,
    /// Form 1099-R — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub r_1099: Option<bool>,
    /// Form SSA-1099 / RRB-1099 — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub ssa_1099: Option<bool>,
    /// Form 1099-NEC / 1099-MISC / 1099-K — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub nec_misc_k_1099: Option<bool>,
    /// Schedule K-1 — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub k1: Option<bool>,
    /// Rental real estate or royalties (Schedule E) — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub schedule_e_rental: Option<bool>,
    /// Form 1099-S — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub s_1099: Option<bool>,
    /// Form 1099-OID — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub oid_1099: Option<bool>,
    /// Form W-2G — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub w2g: Option<bool>,
    /// Form 1099-C — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub c_1099: Option<bool>,
    /// Form 1095-A — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub a_1095: Option<bool>,
    /// Form 1098-T — §2.2, refuses on `Some(true)`.
    #[serde(default)]
    pub t_1098: Option<bool>,
}

impl DocumentCensus {
    /// Read one row.
    ///
    /// ★ An exhaustive `match` on [`DocumentRow`], so a new row cannot be added without a getter.
    #[must_use]
    pub fn get(&self, row: DocumentRow) -> Option<bool> {
        match row {
            DocumentRow::W2 => self.w2,
            DocumentRow::Int1099 => self.int_1099,
            DocumentRow::Div1099 => self.div_1099,
            DocumentRow::B1099 => self.b_1099,
            DocumentRow::G1099 => self.g_1099,
            DocumentRow::Form1098 => self.form_1098,
            DocumentRow::Form1098e => self.form_1098e,
            DocumentRow::Sa1099 => self.sa_1099,
            DocumentRow::Sa5498 => self.sa_5498,
            DocumentRow::R1099 => self.r_1099,
            DocumentRow::Ssa1099 => self.ssa_1099,
            DocumentRow::NecMiscK1099 => self.nec_misc_k_1099,
            DocumentRow::K1 => self.k1,
            DocumentRow::ScheduleERental => self.schedule_e_rental,
            DocumentRow::S1099 => self.s_1099,
            DocumentRow::Oid1099 => self.oid_1099,
            DocumentRow::W2g => self.w2g,
            DocumentRow::C1099 => self.c_1099,
            DocumentRow::A1095 => self.a_1095,
            DocumentRow::T1098 => self.t_1098,
        }
    }

    /// Write one row (`None` un-answers it — the `ClearField` path).
    pub fn set(&mut self, row: DocumentRow, v: Option<bool>) {
        match row {
            DocumentRow::W2 => self.w2 = v,
            DocumentRow::Int1099 => self.int_1099 = v,
            DocumentRow::Div1099 => self.div_1099 = v,
            DocumentRow::B1099 => self.b_1099 = v,
            DocumentRow::G1099 => self.g_1099 = v,
            DocumentRow::Form1098 => self.form_1098 = v,
            DocumentRow::Form1098e => self.form_1098e = v,
            DocumentRow::Sa1099 => self.sa_1099 = v,
            DocumentRow::Sa5498 => self.sa_5498 = v,
            DocumentRow::R1099 => self.r_1099 = v,
            DocumentRow::Ssa1099 => self.ssa_1099 = v,
            DocumentRow::NecMiscK1099 => self.nec_misc_k_1099 = v,
            DocumentRow::K1 => self.k1 = v,
            DocumentRow::ScheduleERental => self.schedule_e_rental = v,
            DocumentRow::S1099 => self.s_1099 = v,
            DocumentRow::Oid1099 => self.oid_1099 = v,
            DocumentRow::W2g => self.w2g = v,
            DocumentRow::C1099 => self.c_1099 = v,
            DocumentRow::A1095 => self.a_1095 = v,
            DocumentRow::T1098 => self.t_1098 = v,
        }
    }
}

/// ★★★ **How many rows of this document does the return CARRY?** `None` = *there is no section
/// whose rows could be counted at all*.
///
/// This is a fact about the return, not a demand: it answers the contradiction rule (a `Some(false)`
/// beside transcribed rows) and the `apply(SetField(No))` guard, both of which need a count for every
/// kind that has one. Five kinds have a `Vec` on `ReturnInputs` — `w2s`, `int_1099`, `div_1099`,
/// `b_1099`, `g_1099` — and `income import` fills all five today; the printed return already reads
/// them (Form 1040 line 1a; line 2a/2b and Schedule B line 1; lines 3a/3b and Schedule B line 5;
/// Schedule D lines 1a/8a; Schedule 1 lines 1 and 7).
///
/// ★ Whether a declared document MUST appear as rows is the separate question
/// [`requires_transcription`] answers. The two were one predicate until the T3 seam review; see the
/// module header.
#[must_use]
pub fn declared_rows(
    ri: &crate::tax::return_inputs::ReturnInputs,
    row: DocumentRow,
) -> Option<usize> {
    match row {
        DocumentRow::W2 => Some(ri.w2s.len()),
        DocumentRow::Int1099 => Some(ri.int_1099.len()),
        DocumentRow::Div1099 => Some(ri.div_1099.len()),
        DocumentRow::B1099 => Some(ri.b_1099.len()),
        DocumentRow::G1099 => Some(ri.g_1099.len()),
        // ★ T5 — the 1098-E gained a `Vec` when `Form1098E` replaced the
        //   `sch1.student_loan_interest_paid` scalar.
        DocumentRow::Form1098e => Some(ri.form_1098e.len()),
        // ★ T16 — both HSA information returns gained a `Vec` with the Form 8889 build.
        DocumentRow::Sa1099 => Some(ri.sa_1099.len()),
        DocumentRow::Sa5498 => Some(ri.sa_5498.len()),
        // ★ T9 — the 1098 gained a `Vec` when `Form1098` replaced the
        //   `schedule_a.mortgage_interest_1098` scalar.
        DocumentRow::Form1098 => Some(ri.form_1098.len()),
        DocumentRow::R1099
        | DocumentRow::Ssa1099
        | DocumentRow::NecMiscK1099
        | DocumentRow::K1
        | DocumentRow::ScheduleERental
        | DocumentRow::S1099
        | DocumentRow::Oid1099
        | DocumentRow::W2g
        | DocumentRow::C1099
        | DocumentRow::A1095
        | DocumentRow::T1098 => None,
    }
}

/// ★★★ **Must a DECLARED document of this kind appear on the return as transcribed rows?**
///
/// The only consumer is [`RefuseReason::DocumentDeclaredNotTranscribed`]: *"you said you received
/// one, and there is none here"* is a true and useful refusal exactly where zero rows cannot be the
/// correct return — and a false one where the form itself offers the filer a blank.
///
/// [`RefuseReason::DocumentDeclaredNotTranscribed`]: crate::tax::return_refuse::RefuseReason::DocumentDeclaredNotTranscribed
#[must_use]
pub const fn requires_transcription(row: DocumentRow) -> bool {
    match row {
        // ★ Nothing else carries these amounts onto the return. A W-2 reaches Form 1040 line 1a only
        //   through `w2s`; a 1099-INT reaches line 2a/2b and Schedule B line 1 only through
        //   `int_1099` (boxes 1–9 are fully modelled); a 1099-DIV reaches lines 3a/3b and Schedule B
        //   line 5 only through `div_1099` (boxes 1a–13). A declared one with no row is therefore
        //   exactly "nothing ever populated it".
        DocumentRow::W2 | DocumentRow::Int1099 | DocumentRow::Div1099 => true,
        // ★★★ 1098-E — YES, from T5. Schedule 1 line 21 reads the SUM of the rows' box 1 and
        //     NOTHING ELSE carries student-loan interest onto the return: the
        //     `sch1.student_loan_interest_paid` scalar that used to is gone. A declared 1098-E with
        //     no row is therefore exactly "nothing ever populated it".
        DocumentRow::Form1098e => true,
        // ★★★ 1099-B — NO, and the FORM says so. `Form1099B`'s own quoted instruction reads
        //     *"However, if you choose to report all these transactions on Form 8949, leave this
        //     line blank and go to line 1b."* The `[[b_1099]]` row exists for the Schedule D line
        //     1a/8a summary OPTION only. btctax's own population is the other case: a crypto
        //     filer's exchange 1099-B covers dispositions that are already in the ledger and print
        //     per-transaction on Form 8949 and Schedule D — their truthful census answer is YES and
        //     their correct row count is ZERO. Demanding the row would refuse them, and entering one
        //     would double-count every gain (`form_1099b_gains` is ADDED to the ledger's
        //     `capital_net`). The declaration is still required; only the rows are optional.
        DocumentRow::B1099 => false,
        // ★★★ 1099-G — **YES, from T5, and this flipped from `false`.**
        //
        //     It was `false` because the commonest 1099-G is an itemizer's prior-year state refund,
        //     box 2 alone, and `Form1099G` HAD NO BOX 2 — so demanding a row would have forced
        //     either false testimony ("no 1099-G") or a hollow all-zero row beside a refund the
        //     census never mentions, which is the census laundering the very thing it exists to
        //     catch. T5 added `box2_state_refund`, so that filer now has a field to transcribe
        //     into, `itemized_prior_year` decides §111(a) on it, and a declared 1099-G with no row
        //     is once again exactly "nothing ever populated it".
        DocumentRow::G1099 => true,
        // ★★★ 1099-SA — YES. Form 8889 line 14a reads the SUM of the rows' box 1 and NOTHING ELSE
        //     carries an HSA distribution onto the return: there is no scalar behind it, and the
        //     payer *"isn't required to compute the taxable amount of any distribution"*
        //     (`f1099sa--2019.txt:61`), so the figure exists nowhere else. A declared 1099-SA with
        //     no row is exactly "nothing ever populated it" — and what it hides is a DISTRIBUTION,
        //     which is gross income plus a 20% additional tax under §223(f). The understatement
        //     direction, so the demand is right.
        DocumentRow::Sa1099 => true,
        // ★★★ 5498-SA — **NO, and the reason is the mirror of the 1099-B's.** No line of Form 8889
        //     sums any box of it: line 2 asks for the contributions the FILER made, and box 2 is
        //     the trustee's employer-and-employee total by calendar year. So zero rows cannot
        //     understate anything, and a filer who holds the form but has already entered their own
        //     contributions on line 2 has a correct return with no row. The DECLARATION is still
        //     required — it is what makes "I hold one" recorded rather than blank — and the
        //     contradiction rule still runs, so a `Some(false)` beside a transcribed row refuses.
        DocumentRow::Sa5498 => false,
        // ★★★ 1098 — **YES, from T9, and this flipped from `false`.** Schedule A line 8a reads the
        //     SUM of the rows' box 1 plus box 6 and NOTHING ELSE carries mortgage interest reported
        //     on a Form 1098 onto the return: the `schedule_a.mortgage_interest_1098` scalar that
        //     used to is gone. A declared 1098 with no row is therefore exactly "nothing ever
        //     populated it". ★ The DIRECTION is the forgiving one — a missing deduction only
        //     overstates tax — but the row is also where the §163(h)(3)(B) ceiling warning and the
        //     box-4 refusal live, and both of those move the other way.
        DocumentRow::Form1098 => true,
        // The §2.2 families refuse on `Yes` before this is ever read — none of them has rows to
        // demand.
        DocumentRow::R1099
        | DocumentRow::Ssa1099
        | DocumentRow::NecMiscK1099
        | DocumentRow::K1
        | DocumentRow::ScheduleERental
        | DocumentRow::S1099
        | DocumentRow::Oid1099
        | DocumentRow::W2g
        | DocumentRow::C1099
        | DocumentRow::A1095
        | DocumentRow::T1098 => false,
    }
}

/// The census row a registry question belongs to, or `None` for a question that is not a census row.
///
/// ★ DERIVED from [`DocumentRow::question_id`] rather than written a second time, so the two
/// directions cannot disagree — [`tests::the_question_map_is_a_bijection`] pins it.
#[must_use]
pub fn row_of_question(id: crate::tax::questions::QuestionId) -> Option<DocumentRow> {
    DocumentRow::ALL
        .iter()
        .copied()
        .find(|r| r.question_id() == id)
}

/// ★ THE liveness predicate for one census row — the ONLY copy, read by the row's `FormQuestion`.
///
/// ★★★ **`form_1098` is live iff `schedule_a.is_some()`, and every other row is always live** (§5.1
/// / R8 / I8). The 1098 arrives whether or not the filer itemizes — what the itemize election
/// governs is the row's LIVENESS, not the document's home.
///
/// Making it always live would force **every homeowner** to transcribe a 1098 and would then refuse
/// a **standard-deduction** filer on a truthful `MortgageWithinDebtLimit = Some(false)` for a
/// $900,000 2019 loan: a refusal over a deduction they are not claiming, and a return they could not
/// file (`SPEC_interview.md` R8). Their 1098 rows would also be a `Field` set nothing reads, which
/// R2 mechanism 3's reverse join reds.
///
/// ★ **`form_1098e` OPENED AT T5**, when [`crate::tax::return_inputs::Form1098E`] replaced the
///   `sch1.student_loan_interest_paid` scalar: it now has a `Vec` to count, a form section to
///   transcribe into and a Schedule 1 line 21 chain that reads the rows, so the row is askable and
///   both its `Yes` and its `No` mean something. **`form_1098` OPENED AT T9** for the same reason —
///   [`crate::tax::return_inputs::Form1098`] replaced `schedule_a.mortgage_interest_1098` — with the
///   itemize election as its gate.
#[must_use]
pub fn row_is_live(ri: &crate::tax::return_inputs::ReturnInputs, row: DocumentRow) -> bool {
    match row {
        DocumentRow::Form1098 => ri.schedule_a.is_some(),
        _ => true,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// R10.4 / T4b — PRE-NAMED rows, and what answering a census row NO does to them
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ **A PRE-NAMED row is an IDENTITY the year-N+1 opener carried, never testimony.**
///
/// `open_next_year` (T4b) seeds each of year N's payers as a row with the identity fields set — the
/// employer and its EIN, the payer and its TIN — and **every box default**, because R10.4's
/// sentence is *"Last year Acme (EIN 12-3456789) issued you a W-2. Did Acme issue one for 2027?"*
/// and the answer to it is not a figure.
///
/// The test is a **comparison against the row's own identity-only seed**, never a list of boxes to
/// check for zero, for exactly the reason [`crate::tax::return_inputs::ReturnInputs`]'s draft
/// predicate is one (`input_form_store::draft_is_disposable`): a box added to `W2` tomorrow is
/// covered the day it is added, with no edit here. And the direction is **fail-CLOSED** — anything
/// the filer has typed, including a `transcribed_on` date, makes the row unequal to its seed, so it
/// is KEPT and the contradiction refusal still fires. Nothing [`drop_pre_named_rows`] removes was
/// ever testimony.
#[must_use]
pub fn row_is_pre_named(
    ri: &crate::tax::return_inputs::ReturnInputs,
    row: DocumentRow,
    i: usize,
) -> bool {
    use crate::tax::return_inputs::{Form1099B, Form1099Div, Form1099G, Form1099Int, W2};
    match row {
        DocumentRow::W2 => ri.w2s.get(i).is_some_and(|w| {
            *w == W2 {
                owner: w.owner,
                employer: w.employer.clone(),
                ein: w.ein.clone(),
                ..Default::default()
            }
        }),
        DocumentRow::Int1099 => ri.int_1099.get(i).is_some_and(|r| {
            *r == Form1099Int {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            }
        }),
        DocumentRow::Div1099 => ri.div_1099.get(i).is_some_and(|r| {
            *r == Form1099Div {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            }
        }),
        DocumentRow::G1099 => ri.g_1099.get(i).is_some_and(|r| {
            *r == Form1099G {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            }
        }),
        DocumentRow::B1099 => ri.b_1099.get(i).is_some_and(|r| {
            *r == Form1099B {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            }
        }),
        // ★ T5 — the 1098-E HAS a section now, but the year-N+1 opener does not seed lenders (it
        //   seeds W-2 employers, 1099 payers and venues — R10.4). So no `form_1098e` row can be
        //   pre-named, and a `No` beside a transcribed row correctly refuses instead of deleting it.
        //   If the opener ever carries a servicer identity, this arm is where it lands.
        // ★★★ T16 — the HSA TRUSTEE **is** seeded (`open_next_year` carries every prior
        //     `sa_1099` / `sa_5498` identity, because a trustee that sent a Form 1099-SA last year
        //     will send one again whenever money leaves the account). So these rows CAN be
        //     pre-named, and the comparison is the same identity-only seed every other payer gets:
        //     anything the filer has typed — a box, a date, an account-type checkbox — makes the
        //     row unequal and it is KEPT, with the contradiction refusal still standing.
        DocumentRow::Sa1099 => ri.sa_1099.get(i).is_some_and(|r| {
            *r == crate::tax::return_inputs::Form1099Sa {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            }
        }),
        DocumentRow::Sa5498 => ri.sa_5498.get(i).is_some_and(|r| {
            *r == crate::tax::return_inputs::Form5498Sa {
                trustee: r.trustee.clone(),
                trustee_tin: r.trustee_tin.clone(),
                ..Default::default()
            }
        }),
        // ★★★ T9 — the Form 1098 LENDER **is** seeded: a lender that sent a Form 1098 last year
        //     sends one again for every year the loan is outstanding, so R10.4's sentence — *"Last
        //     year Acme Savings (TIN 00-0000000) issued you a Form 1098. Did Acme Savings issue one
        //     for 2027?"* — is exactly as true of them as of a bank. Every BOX is blank, including
        //     box 3's origination date and the shared-interest gate: the seed carries an identity,
        //     never testimony.
        DocumentRow::Form1098 => ri.form_1098.get(i).is_some_and(|r| {
            *r == crate::tax::return_inputs::Form1098 {
                lender: r.lender.clone(),
                lender_tin: r.lender_tin.clone(),
                ..Default::default()
            }
        }),
        DocumentRow::Form1098e
        // No section exists, so there is no row to be pre-named — and it is a `match`, so a kind
        // that GAINS a section reds here.
        | DocumentRow::R1099
        | DocumentRow::Ssa1099
        | DocumentRow::NecMiscK1099
        | DocumentRow::K1
        | DocumentRow::ScheduleERental
        | DocumentRow::S1099
        | DocumentRow::Oid1099
        | DocumentRow::W2g
        | DocumentRow::C1099
        | DocumentRow::A1095
        | DocumentRow::T1098 => false,
    }
}

/// Remove every [`row_is_pre_named`] row of `row`'s kind, keeping every row that carries anything.
/// Returns how many were removed.
/// `Vec::retain` driven by a positional keep-list — one helper because a single closure cannot
/// be inferred at five element types.
fn retain_by<T>(v: &mut Vec<T>, keep: &[bool]) {
    let mut i = 0usize;
    v.retain(|_| {
        let k = keep.get(i).copied().unwrap_or(true);
        i += 1;
        k
    });
}

/// Remove every [`row_is_pre_named`] row of `row`'s kind, keeping every row that carries anything.
/// Returns how many were removed.
pub fn drop_pre_named_rows(
    ri: &mut crate::tax::return_inputs::ReturnInputs,
    row: DocumentRow,
) -> usize {
    let before = declared_rows(ri, row).unwrap_or(0);
    let keep: Vec<bool> = (0..before).map(|i| !row_is_pre_named(ri, row, i)).collect();
    match row {
        DocumentRow::W2 => retain_by(&mut ri.w2s, &keep),
        DocumentRow::Int1099 => retain_by(&mut ri.int_1099, &keep),
        DocumentRow::Div1099 => retain_by(&mut ri.div_1099, &keep),
        DocumentRow::G1099 => retain_by(&mut ri.g_1099, &keep),
        DocumentRow::B1099 => retain_by(&mut ri.b_1099, &keep),
        // ★ T16 — both HSA rows have a `Vec` to retain over. `row_is_pre_named` is `false` for
        //   them today, so this removes nothing; it is here because the two predicates must agree,
        //   and a kind whose rows CAN be pre-named must also be able to shed them.
        DocumentRow::Sa1099 => retain_by(&mut ri.sa_1099, &keep),
        DocumentRow::Sa5498 => retain_by(&mut ri.sa_5498, &keep),
        // ★ T9 — the 1098's lender IS seeded, so this arm really removes rows.
        DocumentRow::Form1098 => retain_by(&mut ri.form_1098, &keep),
        DocumentRow::Form1098e
        | DocumentRow::R1099
        | DocumentRow::Ssa1099
        | DocumentRow::NecMiscK1099
        | DocumentRow::K1
        | DocumentRow::ScheduleERental
        | DocumentRow::S1099
        | DocumentRow::Oid1099
        | DocumentRow::W2g
        | DocumentRow::C1099
        | DocumentRow::A1095
        | DocumentRow::T1098 => {}
    }
    before - declared_rows(ri, row).unwrap_or(0)
}

/// ★★★ **THE ONE WRITER of a census answer** — every census [`crate::tax::questions::FormQuestion`]'s
/// `set` delegates here, so `income answer`, the input form and any future surface share one rule.
///
/// Writing the tri-state is all it did before T4b. What it adds is R10.4's *"a No removes the
/// pre-named row"*: the opener seeds identities the filer has not confirmed, and a filer who says
/// *"no, Acme sent me nothing this year"* must not be left holding a row they never typed, beside a
/// refusal (`Some(false)` + rows) that `income answer` offers no way to clear. A row carrying
/// anything at all is KEPT — see [`row_is_pre_named`] for why that direction is the safe one.
pub fn answer_row(ri: &mut crate::tax::return_inputs::ReturnInputs, row: DocumentRow, v: bool) {
    ri.documents.set(row, Some(v));
    if !v {
        drop_pre_named_rows(ri, row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// ★★★ **R10.4 / T4b — a `No` removes the opener's PRE-NAMED rows and NOTHING else.**
    ///
    /// The opener seeds year N's payers as identity-only rows. If answering *"no, none arrived this
    /// year"* left them behind, the filer would meet the census contradiction refusal (`Some(false)`
    /// beside transcribed rows) on rows they never typed — and `income answer` has no delete.
    /// Equally, a row carrying ANY figure is testimony and survives, so the refusal still fires for
    /// the filer who really did transcribe one and then answered no.
    #[test]
    fn answering_a_census_row_no_drops_the_pre_named_rows_and_keeps_a_transcribed_one() {
        use crate::tax::return_inputs::{ReturnInputs, W2};
        use rust_decimal_macros::dec;
        let mut ri = ReturnInputs {
            w2s: vec![
                // pre-named by the opener: identity only
                W2 {
                    employer: "Acme".into(),
                    ein: Some("12-3456789".into()),
                    ..Default::default()
                },
                // typed by the filer: a box carries a figure
                W2 {
                    employer: "Beta".into(),
                    box1_wages: dec!(1),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        answer_row(&mut ri, DocumentRow::W2, false);
        assert_eq!(ri.documents.w2, Some(false), "the answer is written");
        assert_eq!(
            ri.w2s
                .iter()
                .map(|w| w.employer.as_str())
                .collect::<Vec<_>>(),
            vec!["Beta"],
            "the pre-named row goes; the transcribed one is testimony and stays"
        );
    }

    /// The other direction: a `Yes` touches no row (the filer is about to transcribe into them).
    #[test]
    fn answering_yes_keeps_every_pre_named_row() {
        use crate::tax::return_inputs::{Form1099Int, ReturnInputs};
        let mut ri = ReturnInputs {
            int_1099: vec![Form1099Int {
                payer: "Big Bank".into(),
                payer_tin: "99-9999999".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        answer_row(&mut ri, DocumentRow::Int1099, true);
        assert_eq!(ri.int_1099.len(), 1);
    }

    /// ★★★ **M-4 — EVERY census setter delegates to the one writer, ONE KIND AT A TIME.**
    ///
    /// The seam review measured the gap: planting non-delegating `set` closures on FOUR kinds reds
    /// exactly one test, because `screen_inputs` short-circuits at the first
    /// `DocumentCensusContradicted` — so a kind that skips `answer_row` is caught only if it happens
    /// to be the first offender in a fixture that realizes it. This walks the kinds and gives each
    /// its OWN fixture, driven through the REGISTRY's `set` (not `answer_row` directly, which is the
    /// other half of what made the old kill blind).
    #[test]
    fn every_census_setter_drops_its_own_pre_named_rows_through_the_registry() {
        use crate::tax::questions::FORM_QUESTIONS;
        use crate::tax::return_inputs::{
            Form1099B, Form1099Div, Form1099G, Form1099Int, ReturnInputs, W2,
        };
        for row in DocumentRow::ALL {
            let mut ri = ReturnInputs::default();
            if declared_rows(&ri, *row).is_none() {
                continue; // no section: nothing could be pre-named
            }
            match row {
                DocumentRow::W2 => ri.w2s.push(W2 {
                    employer: "E".into(),
                    ..Default::default()
                }),
                DocumentRow::Int1099 => ri.int_1099.push(Form1099Int {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::Div1099 => ri.div_1099.push(Form1099Div {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::G1099 => ri.g_1099.push(Form1099G {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::B1099 => ri.b_1099.push(Form1099B {
                    payer: "P".into(),
                    ..Default::default()
                }),
                // ★ T5 — see the sibling test: a 1098-E row is testimony, never a pre-named
                //   identity, so the registry setter correctly keeps it.
                DocumentRow::Form1098e => {
                    ri.form_1098e.push(crate::tax::return_inputs::Form1098E {
                        lender: "Servicer".into(),
                        ..Default::default()
                    });
                    let q = FORM_QUESTIONS
                        .iter()
                        .find(|q| q.id == row.question_id())
                        .expect("every census row is a registry question");
                    (q.set)(&mut ri, false);
                    assert_eq!(
                        declared_rows(&ri, *row),
                        Some(1),
                        "a transcribed 1098-E row must survive a `No` through the registry too"
                    );
                    continue;
                }
                // ★ T9 — the Form 1098 HAS a section, and the opener DOES seed its LENDER (a lender
                //   sends one every year the loan is outstanding), so a pre-named row is real here
                //   and a `No` must drop it.
                DocumentRow::Form1098 => {
                    ri.form_1098.push(crate::tax::return_inputs::Form1098 {
                        lender: "P".into(),
                        ..Default::default()
                    });
                }
                // ★ T16 — both HSA information returns HAVE a section, and the opener DOES seed
                //   their trustee (an HSA trustee sends a Form 1099-SA every year money leaves the
                //   account), so a pre-named row is real here and a `No` must drop it.
                DocumentRow::Sa1099 => ri.sa_1099.push(crate::tax::return_inputs::Form1099Sa {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::Sa5498 => ri.sa_5498.push(crate::tax::return_inputs::Form5498Sa {
                    trustee: "P".into(),
                    ..Default::default()
                }),
                other => panic!("{other:?} gained a section — give it a pre-named seed here"),
            }
            let q = FORM_QUESTIONS
                .iter()
                .find(|q| q.id == row.question_id())
                .expect("every census row is a registry question");
            (q.set)(&mut ri, false);
            assert_eq!(
                declared_rows(&ri, *row),
                Some(0),
                "{row:?}: its registry setter does not go through `answer_row`, so the opener's \
                 pre-named row survives a `No` and the year refuses on a contradiction the filer \
                 cannot clear"
            );
        }
    }

    /// ★ **The predicate FAILS CLOSED, and here is the smallest thing that proves it**: a row whose
    /// only mark is a `transcribed_on` date — no figure at all — is still the filer's, and is kept.
    /// A predicate written as *"every money box is zero"* would delete it.
    #[test]
    fn a_row_marked_only_with_a_transcription_date_is_not_pre_named() {
        use crate::tax::return_inputs::{Form1099Div, ReturnInputs};
        let mut ri = ReturnInputs {
            div_1099: vec![Form1099Div {
                payer: "Fund".into(),
                payer_tin: "11-1111111".into(),
                transcribed_on: Some(time::macros::date!(2027 - 02 - 01)),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!row_is_pre_named(&ri, DocumentRow::Div1099, 0));
        answer_row(&mut ri, DocumentRow::Div1099, false);
        assert_eq!(ri.div_1099.len(), 1, "a marked row is never dropped");
    }

    /// Every kind with a section is reachable through the one writer — derived from
    /// [`declared_rows`], never a hand-list of five.
    #[test]
    fn every_kind_with_a_section_drops_its_pre_named_rows() {
        use crate::tax::return_inputs::{
            Form1099B, Form1099Div, Form1099G, Form1099Int, ReturnInputs, W2,
        };
        for row in DocumentRow::ALL {
            let mut ri = ReturnInputs::default();
            if declared_rows(&ri, *row).is_none() {
                continue; // no section: nothing could be pre-named
            }
            match row {
                DocumentRow::W2 => ri.w2s.push(W2 {
                    employer: "E".into(),
                    ..Default::default()
                }),
                DocumentRow::Int1099 => ri.int_1099.push(Form1099Int {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::Div1099 => ri.div_1099.push(Form1099Div {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::G1099 => ri.g_1099.push(Form1099G {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::B1099 => ri.b_1099.push(Form1099B {
                    payer: "P".into(),
                    ..Default::default()
                }),
                // ★★★ T5 — the 1098-E HAS a section now, and it is deliberately NOT pre-namable:
                //     `open_next_year` seeds W-2 employers, 1099 payers and venues, never loan
                //     servicers. So a transcribed row is TESTIMONY, survives a `No`, and the census
                //     contradiction refusal fires — which is the correct behaviour, not an
                //     omission. Asserted positively so this arm can never become a silent skip.
                DocumentRow::Form1098e => {
                    ri.form_1098e.push(crate::tax::return_inputs::Form1098E {
                        lender: "Servicer".into(),
                        ..Default::default()
                    });
                    assert!(
                        !row_is_pre_named(&ri, *row, 0),
                        "the opener does not seed loan servicers, so no `form_1098e` row can be \
                         pre-named — if that changes, give this arm a seed and drop this assert"
                    );
                    answer_row(&mut ri, *row, false);
                    assert_eq!(
                        declared_rows(&ri, *row),
                        Some(1),
                        "a transcribed 1098-E row must survive a `No` — it is testimony, not a \
                         carried identity"
                    );
                    continue;
                }
                // ★ T9 — the Form 1098 HAS a section, and the opener DOES seed its LENDER (a lender
                //   sends one every year the loan is outstanding), so a pre-named row is real here
                //   and a `No` must drop it.
                DocumentRow::Form1098 => {
                    ri.form_1098.push(crate::tax::return_inputs::Form1098 {
                        lender: "P".into(),
                        ..Default::default()
                    });
                }
                // ★ T16 — both HSA information returns HAVE a section, and the opener DOES seed
                //   their trustee (an HSA trustee sends a Form 1099-SA every year money leaves the
                //   account), so a pre-named row is real here and a `No` must drop it.
                DocumentRow::Sa1099 => ri.sa_1099.push(crate::tax::return_inputs::Form1099Sa {
                    payer: "P".into(),
                    ..Default::default()
                }),
                DocumentRow::Sa5498 => ri.sa_5498.push(crate::tax::return_inputs::Form5498Sa {
                    trustee: "P".into(),
                    ..Default::default()
                }),
                other => panic!("{other:?} gained a section — give it a pre-named seed here"),
            }
            assert_eq!(declared_rows(&ri, *row), Some(1));
            answer_row(&mut ri, *row, false);
            assert_eq!(
                declared_rows(&ri, *row),
                Some(0),
                "{row:?}: the pre-named row must be dropped by the one writer"
            );
        }
    }

    /// Every row round-trips through `get`/`set`, and `ALL` is complete (the exhaustive `match`es
    /// make a new variant a compile error; this pins that `ALL` lists it too).
    #[test]
    fn every_row_is_listed_and_round_trips() {
        assert_eq!(
            DocumentRow::ALL.len(),
            20,
            "§5.1's eighteen rows plus T16's two HSA information returns"
        );
        let mut c = DocumentCensus::default();
        for row in DocumentRow::ALL {
            assert_eq!(c.get(*row), None, "{row:?} starts unanswered");
            c.set(*row, Some(true));
            assert_eq!(c.get(*row), Some(true), "{row:?} reads back what was set");
        }
        // Every leaf is now `Some(true)` — i.e. `set` wrote twenty DISTINCT leaves, not one
        // twenty times.
        for row in DocumentRow::ALL {
            assert_eq!(c.get(*row), Some(true), "{row:?} was not overwritten");
        }
        let ids: BTreeSet<_> = DocumentRow::ALL.iter().collect();
        assert_eq!(ids.len(), DocumentRow::ALL.len(), "no duplicate rows");
    }

    /// ★★ **R3 kill (d), the table half.** Every refusing row carries a DISTINCT §2.2 exit sentence,
    /// and every non-refusing row carries none. A sentence copied from a sibling row — the shape a
    /// bulk edit produces — makes two rows share one, and reds here.
    #[test]
    fn every_refusing_row_has_its_own_exit_sentence_and_the_rest_have_none() {
        let mut seen: BTreeSet<&'static str> = BTreeSet::new();
        let mut refusing = 0usize;
        for row in DocumentRow::ALL {
            match row.exit_sentence() {
                Some(s) => {
                    assert!(!s.trim().is_empty(), "{row:?}'s exit sentence is empty");
                    assert!(
                        seen.insert(s),
                        "{row:?} shares its exit sentence with another row — a filer would be told \
                         about a document they do not hold"
                    );
                    refusing += 1;
                }
                // ★ A row with NO exit is one btctax can hold the rows for today — and that set
                //   is DERIVED, not listed: `entry_route` is `Some` exactly for the transcribable
                //   rows, and the two scalar-shadowed rows are the ones `row_is_live` excludes. A
                //   hand-list here would have to be edited twice when T5 or T9 lands.
                None => assert!(
                    row.entry_route().is_some()
                        || matches!(row, DocumentRow::Form1098 | DocumentRow::Form1098e),
                    "{row:?} has no exit sentence but is neither transcribable nor non-live"
                ),
            }
        }
        assert_eq!(
            refusing, 11,
            "exactly §2.2's eleven excluded families refuse on Yes — the W-2 and the four 1099 \
             families are transcribable TODAY (D1), and the two scalar-shadowed rows are not live"
        );
    }

    /// Each row's prompt names the document by the words on the paper, so the filer can check it
    /// against their shoebox rather than against a category name btctax invented.
    #[test]
    fn every_prompt_names_the_document_and_is_a_question() {
        for row in DocumentRow::ALL {
            let p = row.prompt();
            assert!(p.ends_with('?'), "{row:?}'s prompt must be a question: {p}");
            assert!(
                p.contains("instructions"),
                "{row:?}'s prompt must cite the instruction that attributes the document: {p}"
            );
        }
    }

    /// The row ⇄ question map is a BIJECTION. A copy-paste that pointed two rows at one question
    /// would make one census leaf unaskable and the other doubly asked, with nothing else red.
    #[test]
    fn the_question_map_is_a_bijection() {
        let ids: BTreeSet<_> = DocumentRow::ALL.iter().map(|r| r.question_id()).collect();
        assert_eq!(
            ids.len(),
            DocumentRow::ALL.len(),
            "two rows share one QuestionId"
        );
        for row in DocumentRow::ALL {
            assert_eq!(row_of_question(row.question_id()), Some(*row));
        }
        assert_eq!(
            row_of_question(crate::tax::questions::QuestionId::ForeignTrust),
            None,
            "a non-census question must not resolve to a row"
        );
    }

    /// The scalar-shadowed row is the ONLY non-live one, and the reason is recorded in the predicate
    /// rather than in a comment somewhere else.
    ///
    /// ★ **T5 moved this from two rows to one.** `form_1098e` was shadowed by
    /// `sch1.student_loan_interest_paid`; T5 deleted that scalar, gave the 1098-E its own rows and
    /// its own form section, and the row opened. `form_1098` waits for T9.
    #[test]
    fn only_the_one_scalar_shadowed_row_is_not_live() {
        let ri = crate::tax::return_inputs::ReturnInputs::default();
        let dead: Vec<_> = DocumentRow::ALL
            .iter()
            .filter(|r| !row_is_live(&ri, **r))
            .collect();
        assert_eq!(
            dead,
            vec![&DocumentRow::Form1098],
            "only Form 1098 (T9) is still shadowed by a scalar"
        );
    }
}
