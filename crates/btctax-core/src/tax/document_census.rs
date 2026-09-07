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
//! | `Some(true)` | one or more | the section is live; **zero transcribed rows refuses** |
//!
//! And for an **unsupported** type (`SPEC_interview.md` §2.2) `Some(true)` refuses with that family's
//! own exit sentence — *"a filer cannot answer no to a category they were never shown"*, so the row
//! exists even where btctax can do nothing with a yes.
//!
//! ★ **Every row is a [`crate::tax::questions::FormQuestion`]**, so `screen_inputs` refuses a live
//! `None`, `income answer` asks it, and the Declarations adapter renders it — zero new plumbing, and
//! exactly one liveness predicate per row.
//!
//! ★★ **The rows are NOT all countable, and that is recorded rather than papered over.**
//! [`transcribed_rows`] returns `Some(n)` only for a kind btctax has a *screen* for; `None` says
//! *"there is no section whose rows could be counted"*. The three-rule engine below then applies only
//! the rules a `None` can honestly support — which is why a kind with no section refuses outright on
//! `Some(true)` instead of pretending to check a row count it does not have.

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
    /// `None` for a row btctax can actually take (`W2`) and for the two rows whose amount is
    /// collected today by a scalar (`Form1098`, `Form1098e`) — those are not live, so no answer of
    /// theirs can reach a refusal.
    ///
    /// ★ The four rows whose SCREEN is task T5 carry a sentence of the same shape as §2.2's: the
    /// family is in scope, the transcription surface is not built yet, and a filer who has one is
    /// refused rather than under-filed.
    #[must_use]
    pub const fn exit_sentence(self) -> Option<&'static str> {
        Some(match self {
            // Supported and transcribable today — no exit.
            DocumentRow::W2 => return None,
            // Not live (their amount is a scalar until T9 / T5) — no answer, so no exit.
            DocumentRow::Form1098 | DocumentRow::Form1098e => return None,
            // ── In scope, screen not built (T5). ────────────────────────────────────────────────
            DocumentRow::Int1099 => {
                "btctax cannot yet take a Form 1099-INT: the 1099-INT screen and its box census are \
                 not built (T5). Interest reaches Form 1040 line 2a/2b and Schedule B line 1, so \
                 filing without it would understate the tax."
            }
            DocumentRow::Div1099 => {
                "btctax cannot yet take a Form 1099-DIV: the 1099-DIV screen and its box census are \
                 not built (T5). Dividends reach Form 1040 lines 3a/3b and Schedule B line 5, so \
                 filing without them would understate the tax."
            }
            DocumentRow::B1099 => {
                "btctax cannot yet take a Form 1099-B: the 1099-B screen and its box census are not \
                 built (T5). Broker proceeds reach Schedule D lines 1a/8a, so filing without them \
                 would understate the tax."
            }
            DocumentRow::G1099 => {
                "btctax cannot yet take a Form 1099-G: the 1099-G screen, box 2 and the State and \
                 Local Income Tax Refund Worksheet are not built (T5). A taxable refund reaches \
                 Schedule 1 line 1, so filing without it would understate the tax."
            }
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
    /// Form 1098-E — student loan interest, Schedule 1 line 21. **Not live until T5** replaces the
    /// `sch1.student_loan_interest_paid` scalar; see [`row_is_live`].
    #[serde(default)]
    pub form_1098e: Option<bool>,
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

/// ★★★ **How many rows of this document has the filer actually transcribed?** `None` = *there is no
/// section whose rows could be counted*.
///
/// **The controller's recorded decision (T3, folded as a deviation).** Only `W2` has a transcription
/// section at HEAD. `int_1099` / `div_1099` / `b_1099` / `g_1099` have a `Vec` on `ReturnInputs` but
/// **no screen and no box census** — T5 builds those — so counting their rows would report a number
/// nobody was ever asked to fill, and a `Some(false)` beside an imported row would be refused for a
/// question the filer was never given a way to answer. They therefore count as `None` here and refuse
/// outright on `Some(true)`, which is §2.2's rule applied to a family that is in scope but not yet
/// transcribable: **refuse rather than under-file.** T5 flips them exactly as T13 flips
/// `nec_misc_k_1099`.
///
/// `form_1098` / `form_1098e` are `None` for a different reason: their amount IS collected today, by
/// a scalar, so a `No` on the row would contradict an entered amount. They are not live at all
/// ([`row_is_live`]).
#[must_use]
pub fn transcribed_rows(
    ri: &crate::tax::return_inputs::ReturnInputs,
    row: DocumentRow,
) -> Option<usize> {
    match row {
        DocumentRow::W2 => Some(ri.w2s.len()),
        DocumentRow::Int1099
        | DocumentRow::Div1099
        | DocumentRow::B1099
        | DocumentRow::G1099
        | DocumentRow::Form1098
        | DocumentRow::Form1098e
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
        | DocumentRow::T1098 => None,
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
/// Every row is live except `form_1098` and `form_1098e`, whose amounts are collected today by a
/// scalar (`schedule_a.mortgage_interest_1098`, `sch1.student_loan_interest_paid`): a `No` there
/// would contradict a figure already entered, and a `Yes` would demand a transcription section that
/// does not exist. §5.1's `schedule_a.is_some()` liveness for `form_1098` lands with **T9**; the
/// `form_1098e` row opens with **T5**.
#[must_use]
pub fn row_is_live(_ri: &crate::tax::return_inputs::ReturnInputs, row: DocumentRow) -> bool {
    !matches!(row, DocumentRow::Form1098 | DocumentRow::Form1098e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every row round-trips through `get`/`set`, and `ALL` is complete (the exhaustive `match`es
    /// make a new variant a compile error; this pins that `ALL` lists it too).
    #[test]
    fn every_row_is_listed_and_round_trips() {
        assert_eq!(DocumentRow::ALL.len(), 18, "§5.1 declares eighteen rows");
        let mut c = DocumentCensus::default();
        for row in DocumentRow::ALL {
            assert_eq!(c.get(*row), None, "{row:?} starts unanswered");
            c.set(*row, Some(true));
            assert_eq!(c.get(*row), Some(true), "{row:?} reads back what was set");
        }
        // Every leaf is now `Some(true)` — i.e. `set` wrote eighteen DISTINCT leaves, not one
        // eighteen times.
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
                None => assert!(
                    matches!(
                        row,
                        DocumentRow::W2 | DocumentRow::Form1098 | DocumentRow::Form1098e
                    ),
                    "{row:?} has no exit sentence but is neither transcribable nor non-live"
                ),
            }
        }
        assert_eq!(
            refusing, 15,
            "fifteen rows refuse on Yes: eleven §2.2 families plus the four T5 screens"
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

    /// The two scalar-shadowed rows are the ONLY non-live ones, and the reason is recorded in the
    /// predicate rather than in a comment somewhere else.
    #[test]
    fn only_the_two_scalar_shadowed_rows_are_not_live() {
        let ri = crate::tax::return_inputs::ReturnInputs::default();
        let dead: Vec<_> = DocumentRow::ALL
            .iter()
            .filter(|r| !row_is_live(&ri, **r))
            .collect();
        assert_eq!(
            dead,
            vec![&DocumentRow::Form1098, &DocumentRow::Form1098e],
            "only Form 1098 (T9) and Form 1098-E (T5) are shadowed by a scalar"
        );
    }
}
