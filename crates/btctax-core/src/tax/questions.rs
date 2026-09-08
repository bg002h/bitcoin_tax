//! ★ P9 — the FORM QUESTION REGISTRY (`SPEC_form_questions.md` §3.1).
//!
//! The one place that knows the set of yes/no DECLARATIONS a return must answer. `screen_inputs`,
//! `income answer`, and `ReturnHeader::build` all DERIVE from this list, so no liveness predicate is
//! written twice — which is the whole point: the answered-ness invariant was the last load-bearing
//! invariant held by convention instead of construction (see [`super::return_inputs`]'s doc and D-8).

use crate::conventions::Usd;
use crate::tax::return_inputs::ReturnInputs;
use crate::tax::return_refuse::RefuseReason;
use crate::tax::types::FilingStatus;
use time::Date;

/// ★★★ **§G-15 — does this answer survive into the NEXT tax year?**
///
/// Answers are stored per tax year and **nothing carries forward silently** — correct, because a
/// prior year's "no" is not testimony for this year. But re-asking *everything* every year is waste
/// where the subject cannot change, and at the question counts the field census implies that waste
/// compounds annually.
///
/// ★★ **A prior-year answer must NEVER silently satisfy this year's provenance.** That is the
/// answered-ness invariant crossing a year boundary — software answering for the filer, one year
/// removed. The lawful shape is a **confirmation**, not a carry: *"Last year you said no. Still true
/// for 2025?"* is a NEW answer, given this year, bearing this year's date.
///
/// ★ **Defaults toward [`Durability::PerYear`]** — the fail-closed direction is to re-ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// The subject can differ year to year (foreign accounts, blindness, an election). Re-ask
    /// **blank**: for a one-keystroke answer, showing the prior buys nothing but anchoring.
    PerYear,
    /// The subject cannot change once known — a birth date. The prior MAY be displayed, but it still
    /// requires the same explicit keystroke as a fresh ask: never Enter-to-accept, never pre-filled.
    /// (Here a forced retype invites typos, and for a DOB a typo is the worse failure.)
    Durable,
}

/// A DECLARATION (§2, class A) — the filer ASSERTS it under §6065's jurat, so there is NO lawful default
/// and an unanswered one must REFUSE.
///
/// ONE entry per question, owning the prompt, the refusal, the refusal DETAIL, the liveness scope, and the
/// accessors. `screen_inputs`, `income answer`, and `ReturnHeader::build` DERIVE from this list.
pub struct FormQuestion {
    pub id: QuestionId,
    /// The prompt, phrased as the FORM phrases it (the words the filer can check against their paperwork).
    pub prompt: &'static str,
    /// The `RefuseReason` for an unanswered (`None`) live question.
    pub unanswered: RefuseReason,
    /// ★ The FULL refusal detail (r1 I-1). NOT derived from `prompt`: the shipped texts carry the statutory
    /// cite and the REMEDY (`run btctax income answer`) — doctrine requires the exit ("a refusal with no
    /// exit is just a brick with better prose"). A prompt-derived text would drop both.
    pub unanswered_detail: &'static str,
    /// ★ THE liveness predicate — the ONLY copy in the codebase.
    pub live: fn(&ReturnInputs) -> bool,
    /// Read the current answer.
    pub get: fn(&ReturnInputs) -> Option<bool>,
    /// Write an answer. Called only on a LIVE question (so, e.g., the mortgage setter may assume a
    /// `schedule_a` exists — its liveness requires one).
    pub set: fn(&mut ReturnInputs, bool),
    /// ★ §G-15 — whether this answer survives into the next tax year. See [`Durability`].
    pub durability: Durability,
    /// ★ The answer that requires **no adjustment and forgoes no benefit** — the "nothing to see here"
    /// reply. Most declarations are neutral at `false` ("no, I have no foreign trust"), but not all:
    /// the mortgage box is neutral at `true` (all of the loan bought/built/improved the home, so
    /// Schedule A line 8a stays full), and all three Form 6251 declarations are neutral at `true` (the
    /// dwelling IS AMT-qualified; the AMT carryover IS the same; the AMT depreciation IS the same).
    ///
    /// Declared per question rather than inferred, because polarity used to live as a hard-coded
    /// `matches!` in `testonly.rs` — knowledge a new question could silently get wrong.
    pub neutral: bool,
}

impl FormQuestion {
    /// ★★★ **The words PUT TO THE FILER for THIS return.**
    ///
    /// [`Self::prompt`] verbatim for every question whose subject is fixed text — and the RENDERED
    /// sentence for the few whose subject is a value on the return itself. The distinction is not
    /// cosmetic: [`crate::tax::provenance::record_answer`] hashes the words that were shown, and
    /// R10.3's re-ask rule treats a changed hash as unanswered. So a question that quotes a value
    /// gets its re-ask **for free** when that value changes — which is exactly what
    /// [`QuestionId::FilingStatusConfirmed`] needs: change the status, and the confirmation that
    /// named the old one stops standing.
    ///
    /// ★ Rendered prompts live in [`RENDERED_PROMPTS`], a table, rather than in a `match` with a `_`
    ///   arm: a table can be walked by a KAT (every entry names a real registry question), and a
    ///   wildcard arm cannot.
    #[must_use]
    pub fn prompt_text(&self, ri: &ReturnInputs) -> std::borrow::Cow<'static, str> {
        RENDERED_PROMPTS
            .iter()
            .find(|(id, _)| *id == self.id)
            .map_or(std::borrow::Cow::Borrowed(self.prompt), |(_, render)| {
                std::borrow::Cow::Owned(render(ri))
            })
    }
}

/// One question whose prompt is rendered from the return, and the renderer.
pub type RenderedPrompt = (QuestionId, fn(&ReturnInputs) -> String);

/// The questions whose prompt is RENDERED FROM THE RETURN. See [`FormQuestion::prompt_text`].
pub const RENDERED_PROMPTS: &[RenderedPrompt] = &[
    (QuestionId::FilingStatusConfirmed, filing_status_prompt),
    // ★ R3 — both quote a YEAR off the return, because the instruction's own sentence does: *"if you
    //   received a refund … in 2025"* and the tax-benefit rule looks at *"the year you paid the
    //   tax"*. Naming the wrong year would make the question uncheckable against the filer's papers.
    (QuestionId::StateRefundWithout1099g, state_refund_prompt),
    (QuestionId::ItemizedPriorYear, itemized_prior_year_prompt),
    // ★★★ R9 / T6 — the Digital Assets question quotes the YEAR because the form's own sentence
    //   does (*"At any time during 2025, did you…"*), and it is the year the filer must check their
    //   records against. Rendered, so a return whose `tax_year` changes re-asks it for free (R10.4).
    (QuestionId::DigitalAssetActivity, digital_asset_prompt),
    // ★★★ R7 / T8 — QSS condition 1 quotes the TWO-YEAR WINDOW, and the window is derived from
    //     `tax_year` rather than typed: the instruction's own sentence names the years (*"Your spouse
    //     died in 2023 or 2024 and you didn't remarry before the end of 2025"*), every revision
    //     shifts them, and a prompt naming the wrong pair is a question the filer cannot check
    //     against their own facts. Rendered, so a return whose `tax_year` changes re-asks it for
    //     free (R10.3/R10.4).
    (
        QuestionId::QssSpouseDiedInWindowAndNotRemarried,
        qss_window_prompt,
    ),
];

/// QSS condition 1 with THIS return's two-year window: *"Your spouse died in 2023 or 2024 and you
/// didn't remarry before the end of 2025."* (`i1040gi--2025.txt:1293-1297`.)
fn qss_window_prompt(ri: &ReturnInputs) -> String {
    let y = ri.tax_year;
    format!(
        "Qualifying surviving spouse, condition 1: \"Your spouse died in {a} or {b} and you didn't \
         remarry before the end of {y}.\" (If your spouse died in {y}, you can't file as qualifying \
         surviving spouse; see the instructions for Married Filing Jointly.)",
        a = y - 2,
        b = y - 1,
    )
}

/// *"Did you receive a refund, credit or offset of state or local income taxes in 2026? …"*
fn state_refund_prompt(ri: &ReturnInputs) -> String {
    format!(
        "Did you receive a refund, credit or offset of state or local income taxes in {}? (Form 1040 \
         instructions, Schedule 1 line 1: \"Report any taxable refund you received even if you \
         didn't receive Form 1099-G.\")",
        ri.tax_year
    )
}

/// *"Did you itemize deductions on your 2025 federal return …?"* — the §111(a) tax-benefit test,
/// asked of the year the tax was PAID, which is the year before the refund arrived.
fn itemized_prior_year_prompt(ri: &ReturnInputs) -> String {
    format!(
        "Did you itemize deductions on your {} federal return — that is, did you file a Schedule A \
         instead of taking the standard deduction? (Form 1040 instructions, Schedule 1 line 1: none \
         of a state or local income tax refund is taxable if, in the year you paid the tax, you did \
         not itemize.)",
        ri.tax_year - 1
    )
}

/// ★★★ **R9 — Form 1040 page 1's Digital Assets question, in the FORM's own words.**
///
/// The sentence is transcribed from the printed line (`design/forms/extract/f1040--2025.txt:36-37`)
/// with the year taken off the return, and the instruction's two carve-outs are quoted after it
/// (`i1040gi--2025.txt:1382-1391` + `:1395-1396`) — *"The following actions or transactions in
/// 2025, alone, generally don't require you to check 'Yes': … Purchasing digital assets using U.S.
/// or other real currency"* — because those are exactly the two states a btctax ledger is FULL of.
/// A filer who bought monthly on Swan and moved coins to their own wallet answers **No**, and without the
/// carve-outs in front of them the ledger they can see would push them to answer **Yes**.
///
/// ★ The wallet-to-wallet carve-out is stated as the instruction states it — *"one wallet or
///   account you own or control to another wallet or account that you own or control"* — and NOT
///   with the word `reconcile` uses for the same movement, which R15's stop list forbids in a
///   return-registry prompt (the ledger's question about that movement belongs to `reconcile`).
fn digital_asset_prompt(ri: &ReturnInputs) -> String {
    format!(
        "At any time during {y}, did you: (a) receive (as a reward, award, or payment for property \
         or services); or (b) sell, exchange, or otherwise dispose of a digital asset (or a \
         financial interest in a digital asset)? (Form 1040 instructions, Digital Assets: holding a \
         digital asset; moving one between wallets or accounts you own or control; and purchasing \
         digital assets with U.S. or other real currency do NOT, alone, require a \"Yes\".)",
        y = ri.tax_year
    )
}

/// *"Your TY2024 return filed as Head of Household. Is Head of Household your filing status for
/// TY2025? (Marital status is determined on the last day of the tax year — Form 1040 instructions,
/// Filing Status.)"*
///
/// ★ Both years come off the return (`opened_from` and `tax_year`), and the status is named in the
///   form's own words, so the sentence is checkable against the paperwork the filer holds.
/// ★★★ **The status in FORM 1040's OWN WORDS** — the five Filing Status checkboxes as the 2024 and
/// 2025 forms print them.
///
/// It exists so a prompt can name the status the way the filer's paperwork does: a question that says
/// *"Head of household (HOH)"* is checkable against the box on the page, and one that says `HoH` is
/// not.
///
/// ★ It lives HERE and not on `FilingStatus` because `tax/types.rs` is a **frozen** file
///   (`frozen_guard`): the delta engine is never edited, and a display helper is not worth the
///   documented exception process.
#[must_use]
pub const fn filing_status_words(status: FilingStatus) -> &'static str {
    match status {
        FilingStatus::Single => "Single",
        FilingStatus::Mfj => "Married filing jointly",
        FilingStatus::Mfs => "Married filing separately (MFS)",
        FilingStatus::HoH => "Head of household (HOH)",
        FilingStatus::Qss => "Qualifying surviving spouse (QSS)",
    }
}

fn filing_status_prompt(ri: &ReturnInputs) -> String {
    let status = filing_status_words(ri.filing_status);
    match ri.opened_from {
        Some(from) => format!(
            "Your TY{from} return filed as {status}. Is {status} your filing status for TY{to}? \
             (Marital status is determined on the last day of the tax year — Form 1040 \
             instructions, Filing Status.)",
            to = ri.tax_year
        ),
        // Not reachable through `live`, and it still says something true rather than nothing.
        None => format!(
            "Is {status} your filing status for TY{to}? (Marital status is determined on the last \
             day of the tax year — Form 1040 instructions, Filing Status.)",
            to = ri.tax_year
        ),
    }
}

/// The identity of each registry question. `ALL` is the anchor the completeness test iterates; a new
/// variant is a compile error in that test until it is listed (§3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuestionId {
    DependentTaxpayer,
    DependentSpouse,
    MfsSpouseItemizes,
    ForeignAccounts,
    ForeignTrust,
    /// §2.4 — whether a Form 8889 trigger fired (renamed from the old "do you hold an HSA?").
    HsaActivity,
    /// §2.5 — the 1040 header dual-status-alien box.
    DualStatusAlien,
    /// §2.7 — the Schedule A line-8 mixed-use-mortgage box.
    MortgageAllUsedToBuyBuildImprove,
    /// **Form 6251 line 3** — is the mortgaged dwelling AMT-qualified? (i6251 p.8.)
    AmtQualifiedDwelling,
    /// **Form 6251 line 2k** — does the AMT capital-loss carryover equal the regular one?
    AmtCarryoverSameAsRegular,
    /// **Form 6251 line 2l** — is the depreciation inside the Schedule C expense total the same for the
    /// AMT as for the regular tax?
    AmtDepreciationSameAsRegular,
    /// **§164(b)(7)(B)(iv) / Schedule 1-A Part I** — did the filer exclude income under §911/931/933?
    HasIncomeExclusion,
    /// **§G-22 / B11** — the scope attestation: income this tool never asked about.
    OtherOutOfScopeIncome,
    /// **§163(h)(3)(B) / Schedule A line 8a** — is the combined home-acquisition debt inside every
    /// i1040sca *"Limits on home mortgage interest"* ceiling? ★ APPENDED AT THE END: `decl_tristate!`
    /// couples to this array's INDEX, so a mid-array insert silently repoints every later question.
    MortgageWithinDebtLimit,
    /// **Schedule D line 20 / Schedule A line 9** — is the filer filing Form 4952? ★ APPENDED AT THE
    /// END, for the `decl_tristate!` array-index reason above.
    FilingForm4952,
    /// **Capital Loss Carryover Worksheet header / §1212(b)** — does the carryover-in include a loss
    /// that was the SPOUSE'S, from a joint year now being filed separately? ★ APPENDED AT THE END,
    /// for the `decl_tristate!` array-index reason above.
    CarryoverIncludesSpousesJointLoss,
    /// **Capital Loss Carryover Worksheet header / §108(b)(2)(G)** — did the filer exclude canceled
    /// debt from income, requiring attribute reduction? ★ APPENDED AT THE END, same reason.
    ExcludedCanceledDebt,
    // ── ★★★ R3 / §5.1 — THE DOCUMENT CENSUS, one tri-state per document type. ───────────────────
    //
    // ★ ALL EIGHTEEN APPENDED AT THE END, and it is not cosmetic: `decl_tristate!`
    //   (`btctax-input-form/src/spec/registries.rs`) couples to this array's INDEX, so a mid-array
    //   insert silently repoints every later question at the wrong leaf.
    //
    // ★ Each carries its `DocumentRow` in its own name rather than a payload, because `QuestionId`
    //   is the key of an `AnswerKey` and a payload-carrying key would not be `Copy`-cheap to store
    //   or stable to parse back off the wire.
    /// **§5.1 document census** — did the filer receive one or more Form W-2?
    DocW2,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-INT?
    DocInt1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-DIV?
    DocDiv1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-B?
    DocB1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-G?
    DocG1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1098?
    DocForm1098,
    /// **§5.1 document census** — did the filer receive one or more Form 1098-E?
    DocForm1098e,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-R?
    DocR1099,
    /// **§5.1 document census** — did the filer receive one or more Form SSA-1099 / RRB-1099?
    DocSsa1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-NEC / 1099-MISC / 1099-K?
    DocNecMiscK1099,
    /// **§5.1 document census** — did the filer receive one or more Schedule K-1?
    DocK1,
    /// **§5.1 document census** — did the filer receive one or more rental real estate / royalties (Schedule E)?
    DocScheduleERental,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-S?
    DocS1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-OID?
    DocOid1099,
    /// **§5.1 document census** — did the filer receive one or more Form W-2G?
    DocW2g,
    /// **§5.1 document census** — did the filer receive one or more Form 1099-C?
    DocC1099,
    /// **§5.1 document census** — did the filer receive one or more Form 1095-A?
    DocA1095,
    /// **§5.1 document census** — did the filer receive one or more Form 1098-T?
    DocT1098,
    /// ★★★ **R10.4 / T4b seam review I-1 — is the CARRIED filing status still this year's status?**
    ///
    /// Live only on a year the opener made (`opened_from.is_some()`). APPENDED AT THE END for the
    /// `decl_tristate!` array-index reason recorded above.
    FilingStatusConfirmed,
    // ── ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. Three paired questions, each live iff its
    //    census row(s) answered NO, plus the return-level prior-year-itemize gate. APPENDED AT THE
    //    END for the `decl_tristate!` array-index reason recorded above.
    /// **R3** — wages from an employer who issued no Form W-2. Live iff `documents.w2 == Some(false)`.
    WagesWithoutW2Question,
    /// **R3** — taxable interest or dividends for which no Form 1099-INT or 1099-DIV was issued.
    /// Live iff EITHER `documents.int_1099` or `documents.div_1099` is `Some(false)` — one question
    /// paired with two rows, so each row's own `No` opens it.
    InterestOrDividendsWithout1099,
    /// **R3** — a refund, credit or offset of state or local income taxes with no Form 1099-G. Live
    /// iff `documents.g_1099 == Some(false)`.
    StateRefundWithout1099g,
    /// **R3/I1** — did the PRIOR-YEAR return itemize? Return-level, live iff
    /// `state_refund_without_1099g == Some(true)` **or** any `g_1099[].box2_state_refund > 0`.
    ItemizedPriorYear,
    /// ★★★ **R9 / T6 — the Form 1040 page-1 DIGITAL ASSETS question.** ALWAYS live: the form prints
    /// it on every return and the instruction says *"You must answer the digital asset question on
    /// Form 1040 whether or not you received a Form 1099-DA"* (`i1040gi--2025.txt:1399-1401`).
    /// APPENDED AT THE END for the `decl_tristate!` array-index reason recorded above.
    DigitalAssetActivity,
    // ── ★★★ T16 / FR-76 — Form 8889 (Health Savings Accounts). APPENDED AT THE END for the
    //    `decl_tristate!` array-index reason recorded above. Two census rows for the HSA
    //    information returns, then the seven questions Form 8889 asks that nothing else on the
    //    return can answer. All seven are live iff `sch1.hsa_activity == Some(true)` — a filer with
    //    no HSA trigger is asked none of them, and a filer with one is refused until every one is
    //    answered.
    /// R3 / T16 — the Form 1099-SA census row.
    DocSa1099,
    /// R3 / T16 — the Form 5498-SA census row.
    DocSa5498,
    /// Form 8889 **line 1** — self-only or FAMILY high-deductible health plan coverage.
    HsaFamilyCoverage,
    /// Form 8889 **line 3**'s own condition — eligible on the first day of every month, with the
    /// same coverage. `No` is the form's *"All others"*, which the instructions send to the Line 3
    /// Limitation Chart and Worksheet.
    HsaEligibleEveryMonth,
    /// Form 8889 **line 3 item (6) / line 7** — age 55 or older at the end of the tax year.
    HsaAge55OrOlder,
    /// Form 8889 Part I / the Line 3 Limitation Chart's first question — enrolled in Medicare for
    /// any month.
    HsaMedicareEnrollment,
    /// Form 8889's Part I / II / III heading condition — both spouses have separate HSAs, which the
    /// form answers with *"Complete a separate Form 8889 for each spouse."*
    HsaBothSpousesHaveHsas,
    /// Form 8889 **line 4** — Archer MSA contributions, which come from **Form 8853**.
    HsaArcherMsaActivity,
    /// Form 8889 **Part III** — failure to remain an eligible individual during a testing period.
    HsaTestingPeriodFailure,
    // ── ★★★ SEAM REVIEW I-3 / M-1 — appended at the END, for the `decl_tristate!` array-index
    //    reason recorded above. ─────────────────────────────────────────────────────────────────
    /// Form 8889 **line 1 and line 3 rule 1** — did your SPOUSE have family HDHP coverage? The
    /// instructions ask about *"you or your spouse"* and say the answer counts *"regardless of
    /// whether you file jointly or separately"*.
    HsaSpouseFamilyCoverage,
    /// **R3's document-less door for Form 8889 line 14a** — a distribution taken with no Form
    /// 1099-SA behind it. Live exactly on a `Sa1099` census `No` beside an affirmed HSA trigger.
    HsaDistributionWithout1099sa,
    /// ★★★ **T7 / R6 — Step 5, question 1** of *Who Qualifies as Your Dependent*: the one dependent
    /// gate that is about the FILER rather than about a row, so it is a return-level declaration
    /// rather than a `DependentGate`. Live iff this return carries at least one dependent row.
    /// APPENDED AT THE END for the `decl_tristate!` array-index reason recorded above.
    FilerTinIssuedByDueDate,
    // ── ★★★ R7 / T8 — HEAD OF HOUSEHOLD and QUALIFYING SURVIVING SPOUSE, the two filing statuses
    //    btctax offered with NO TEST AT ALL. Each choice is an assertion about the filer's household
    //    and each unlocks money — HoH a wider bracket and standard deduction, QSS the JOINT rates —
    //    so each now asks the instruction's own tests. APPENDED AT THE END for the `decl_tristate!`
    //    array-index reason recorded above.
    /// **HoH Test 1 or Test 2** (`i1040gi--2025.txt:1164-1200`). Live iff `filing_status == HoH`.
    HohQualifyingPerson,
    /// **HoH — the cost of keeping up a home**, which both tests state (`:1164`, `:1172`). Live iff
    /// `filing_status == HoH`.
    HohPaidOverHalfCostOfKeepingUpHome,
    /// **FR-67 — the §6013(g)/(h) nonresident-alien-spouse election** (`:1059-1072`). Live iff the
    /// return carries a spouse; a `Yes` REFUSES.
    NraSpouseResidentElection,
    /// **QSS condition 1** (`:1293-1297`) — the two-year window, DERIVED from `tax_year`.
    QssSpouseDiedInWindowAndNotRemarried,
    /// **QSS condition 2** (`:1298-1306`).
    QssChildYouCanClaim,
    /// **QSS condition 3** (`:1273-1277`).
    QssChildLivedInYourHomeAllYear,
    /// **QSS condition 4** (`:1278-1279`).
    QssPaidOverHalfCostOfKeepingUpHome,
    /// **QSS condition 5** (`:1280-1283`).
    QssCouldHaveFiledJointlyInYearOfDeath,
    // ── ★★★ R8 / T9 — REAL ESTATE: the Form 8396 gate and the sale of a main home. APPENDED AT THE
    //    END for the `decl_tristate!` array-index reason recorded above.
    /// **Schedule A Line 8a's Caution** (`i1040sca--2025.txt:1091-1096`) — is the filer claiming the
    /// §25 mortgage interest credit? Live iff the return carries a Form 1098 row or a line 8b row;
    /// a `Yes` REFUSES, because btctax holds no Form 8396 line 3 to subtract.
    ClaimingMortgageInterestCredit,
    /// **Schedule D, *Sale of Your Home*** (`i1040sd--2025.txt:313-316`) — always live.
    SoldMainHome,
    /// **Test 1** (`i1040sd--2025.txt:335-343`) — owned 2 of 5 years and lived in it 2 of 5.
    HomeSaleTest1OwnedAndLived,
    /// **Test 2** (`i1040sd--2025.txt:344-348`) — no exclusion on another main home in 2 years.
    HomeSaleTest2NoRecentExclusion,
    /// **The reporting rule's first bullet** (`i1040sd--2025.txt:321-322`), asked positively — can
    /// the filer exclude ALL of the gain?
    HomeSaleCanExcludeAllGain,
}

impl QuestionId {
    pub const ALL: &'static [QuestionId] = &[
        QuestionId::DependentTaxpayer,
        QuestionId::DependentSpouse,
        QuestionId::MfsSpouseItemizes,
        QuestionId::ForeignAccounts,
        QuestionId::ForeignTrust,
        QuestionId::HsaActivity,
        QuestionId::DualStatusAlien,
        QuestionId::MortgageAllUsedToBuyBuildImprove,
        QuestionId::AmtQualifiedDwelling,
        QuestionId::AmtCarryoverSameAsRegular,
        QuestionId::AmtDepreciationSameAsRegular,
        QuestionId::HasIncomeExclusion,
        QuestionId::OtherOutOfScopeIncome,
        QuestionId::MortgageWithinDebtLimit,
        QuestionId::FilingForm4952,
        QuestionId::CarryoverIncludesSpousesJointLoss,
        QuestionId::ExcludedCanceledDebt,
        QuestionId::DocW2,
        QuestionId::DocInt1099,
        QuestionId::DocDiv1099,
        QuestionId::DocB1099,
        QuestionId::DocG1099,
        QuestionId::DocForm1098,
        QuestionId::DocForm1098e,
        QuestionId::DocR1099,
        QuestionId::DocSsa1099,
        QuestionId::DocNecMiscK1099,
        QuestionId::DocK1,
        QuestionId::DocScheduleERental,
        QuestionId::DocS1099,
        QuestionId::DocOid1099,
        QuestionId::DocW2g,
        QuestionId::DocC1099,
        QuestionId::DocA1095,
        QuestionId::DocT1098,
        QuestionId::FilingStatusConfirmed,
        QuestionId::WagesWithoutW2Question,
        QuestionId::InterestOrDividendsWithout1099,
        QuestionId::StateRefundWithout1099g,
        QuestionId::ItemizedPriorYear,
        QuestionId::DigitalAssetActivity,
        // ★★★ T16 / FR-76 — indices 41..=49, appended at the END (see the enum's own note).
        QuestionId::DocSa1099,
        QuestionId::DocSa5498,
        QuestionId::HsaFamilyCoverage,
        QuestionId::HsaEligibleEveryMonth,
        QuestionId::HsaAge55OrOlder,
        QuestionId::HsaMedicareEnrollment,
        QuestionId::HsaBothSpousesHaveHsas,
        QuestionId::HsaArcherMsaActivity,
        QuestionId::HsaTestingPeriodFailure,
        // ★★★ Seam review I-3 and M-1 — indices 50 and 51.
        QuestionId::HsaSpouseFamilyCoverage,
        QuestionId::HsaDistributionWithout1099sa,
        // ★★★ T7 / R6 — index 52.
        QuestionId::FilerTinIssuedByDueDate,
        // ★★★ R7 / T8 — indices 53..=60.
        QuestionId::HohQualifyingPerson,
        QuestionId::HohPaidOverHalfCostOfKeepingUpHome,
        QuestionId::NraSpouseResidentElection,
        QuestionId::QssSpouseDiedInWindowAndNotRemarried,
        QuestionId::QssChildYouCanClaim,
        QuestionId::QssChildLivedInYourHomeAllYear,
        QuestionId::QssPaidOverHalfCostOfKeepingUpHome,
        QuestionId::QssCouldHaveFiledJointlyInYearOfDeath,
        // ★★★ R8 / T9 — indices 61..=65.
        QuestionId::ClaimingMortgageInterestCredit,
        QuestionId::SoldMainHome,
        QuestionId::HomeSaleTest1OwnedAndLived,
        QuestionId::HomeSaleTest2NoRecentExclusion,
        QuestionId::HomeSaleCanExcludeAllGain,
    ];
}

/// ★★ §G-20 — do this return's SPOUSE §63(f) boxes count at all? MFJ always; **MFS only when all
/// three of i1040gi's conditions are affirmatively answered in the claiming direction.**
///
/// The ONE definition, shared by `AgedBlindBoxes::for_return` (which decides the deduction) and by the
/// liveness of `SpouseDiedDuringYear` / `DodSpouse` (which decide whether the questions are even
/// asked). Two copies would drift into asking a question whose answer nothing reads, or — worse —
/// counting a box whose carve-out was never posed.
pub fn spouse_63f_boxes_count(ri: &ReturnInputs) -> bool {
    ri.header.spouse.is_some() && spouse_63f_status_permits(ri)
}

/// Does the FILING STATUS (plus, on MFS, the three i1040gi conditions) permit a spouse §63(f) box —
/// **ignoring whether a spouse record exists**?
///
/// ★★★ r3 I-1 — the two predicates are split because their consumers ask different questions, and
/// collapsing them broke a case. [`spouse_63f_boxes_count`] decides the **deduction**, so it needs a
/// spouse record: no record, no date of birth, no box. The §63(f) **advisories** are about boxes that
/// were FORGONE, and an absent MFJ spouse record is *itself* one of the ways to forgo one — so they
/// must fire precisely where there is nothing to count. Gating them on `spouse_63f_boxes_count` made
/// the advisory silent in the case it exists to report (`mfj_with_no_spouse_record_still_advises_the_
/// aged_box_p5_m2`, which caught it).
///
/// Everything except the record test lives here, so the two can never disagree about the *status*
/// half — which is the coupling §G-20 was about.
pub fn spouse_63f_status_permits(ri: &ReturnInputs) -> bool {
    match ri.filing_status {
        FilingStatus::Mfj => true,
        FilingStatus::Mfs => {
            ri.header.spouse_had_no_income == Some(true)
                && ri.header.spouse_not_filing_a_return == Some(true)
                && ri.header.can_be_claimed_as_dependent_spouse == Some(false)
        }
        _ => false,
    }
}

/// Is `id`'s question LIVE on this return? The single accessor for a liveness predicate outside the
/// registry loop.
///
/// ★ Exists so a VALUE-refusal (`Some(false)` ⇒ refuse) can share the exact predicate its UNANSWERED
/// half uses, instead of re-deriving it. An ungated value-refusal is an exit-less brick: a stale
/// adverse answer left over from a Schedule A that no longer carries mortgage interest, or a Schedule C
/// whose expenses dropped to $0, would refuse a return whose add-back is structurally $0 — with no way
/// for the filer to clear it, because the question is no longer asked. Re-deriving the predicate at the
/// refusal site is exactly the duplication `FormQuestion::live` was introduced to end (§3.1).
pub fn question_is_live(id: QuestionId, ri: &ReturnInputs) -> bool {
    FORM_QUESTIONS
        .iter()
        .find(|q| q.id == id)
        .is_some_and(|q| (q.live)(ri))
}

/// ★★ Does this return bring a capital-loss carryforward IN at all?
///
/// The ONE predicate behind all three carryforward-conditioned declarations — Form 6251 line 2k, the
/// Capital Loss Carryover Worksheet's joint-return sourcing rule, and its §108(b)(2)(G) canceled-debt
/// condition. Factored out so a fourth cannot be written with a fourth copy of the same test.
///
/// ★★★ **It is deliberately NOT widened with a taxable-income term.** The tempting shape — "only ask
/// when the year is at the floor, since that is where the carryover matters" — is the understatement
/// direction: a positive-taxable-income year with a mis-attributed joint carryover still deducts a
/// loss that is not the filer's, and still rolls it forward. `widening-an-exemption-is-never-the-safe-
/// edit`: enumerate the YES-condition (a carryforward exists) and let every other case fail closed.
pub fn carryforward_in_present(ri: &ReturnInputs) -> bool {
    let cf = ri.capital_loss_carryforward_in;
    cf.short > Usd::ZERO || cf.long > Usd::ZERO
}

/// Whether an AMT capital-loss-carryover twin could exist — Form 6251 line 2k's liveness.
fn amt_carryover_question_live(ri: &ReturnInputs) -> bool {
    carryforward_in_present(ri)
}

/// Whether a Form 6251 line 2l depreciation adjustment could be hiding inside the Schedule C expense
/// total — line 2l's liveness.
///
/// ★ An INPUT predicate, like [`amt_carryover_question_live`] and unlike anything compute-dependent:
/// `schedule_c` present with a nonzero expense total. We cannot ask a narrower question, and that is
/// precisely the point — [`ScheduleCInputs::expenses`] is a flat total, so btctax can never see whether
/// Schedule C Part II line 13 ("Depreciation and section 179 expense deduction") is $0 or $200,000. Any
/// filer with business expenses at all must therefore affirm. See [`RefuseReason::
/// AmtDepreciationDeclarationUnanswered`] for why the alternative — assuming $0 — is unsound.
///
/// ★ **The prompt enumerates a narrow YES-list and defaults to NO — deliberately, structurally.**
///
/// This wording took THREE tries, and the first two failed in opposite directions. That history is the
/// design rationale, so it is recorded rather than tidied away:
///   1. v1 listed the 200%-DB trigger broadly and would have refused every filer who owns equipment.
///      Fail-closed, so merely bricking.
///   2. v2 "fixed" that by granting exemptions — and asserted an UNCONDITIONAL straight-line exemption.
///      i6251 qualifies every "isn't refigured" bullet with **placed in service after 1998**, and its
///      must-refigure list carries "Tangible property placed in service after 1986 and before 1999"
///      with no method qualifier. A filer with a 1990s building would have answered yes truthfully and
///      omitted a required add-back — an UNDERSTATEMENT.
///   3. v3 narrowed the pre-1999 hole but still said "no adjustment applies to post-1998 property
///      depreciated ... 150% declining balance", dropping the instructions' parenthetical **"(other
///      than section 1250 property)"**. Post-1998 §1250 property not depreciated straight-line — 15-
///      and 20-year land improvements: paving, fencing, site utilities — is on the MUST-refigure list.
///      Another understatement, and note the qualifier was present in THIS doc comment and lost on the
///      way into the prompt: a paraphrase of a paraphrase.
///
/// **So the structure, not the wording, is the fix.** Enumerating NO-triggers with a broad "otherwise
/// yes" fallback makes every omission an understatement. Enumerating YES-conditions with a "otherwise
/// no" fallback makes every omission an over-refusal, which is fail-closed and recoverable. The prompt
/// now does the latter and says "if you are unsure, answer NO" outright. Adding a missing exemption
/// later is a safe edit; widening the fallback is not.
///
/// Each permitted YES is individually grounded in i6251 (2024) p.5:
///   - no depreciation claimed AND none capitalized ⇒ line 2l is $0 by arithmetic. The capitalization
///     rider is not pedantry: i6251 says "you must refigure depreciation for the AMT, **including
///     depreciation allocable to inventory costs**", and a filer who capitalized rather than deducted
///     would otherwise read "claimed no depreciation" as true;
///   - "Any part of the cost of any property for which you elected to take a section 179 expense
///     deduction" (a fully-§179'd asset leaves no remaining basis to refigure);
///   - "Qualified property that is or was eligible for a special depreciation allowance …" plus "It
///     isn't subject to an AMT adjustment for depreciation if it was placed in service after 2015".
///     ★ A gloss reading "(most equipment bought since 2016)" was REMOVED from this condition: it is
///     not sourced to i6251, and bonus-INELIGIBLE 200%-DB equipment exists (used property acquired
///     before 9/28/2017; related-party and carryover-basis acquisitions), which the MUST list catches.
///     The operative words "qualified for bonus depreciation" already exclude it — but reassuring
///     prose next to a gate is what produced all three earlier defects, so the reassurance goes;
///   - the four straight-line bullets, which between them cover post-1998 §1250 and non-§1250 property.
///     ★ "for the regular tax" is load-bearing and is stated in the prompt: i6251 always writes
///     "depreciated **for the regular tax** using the straight line method", because post-1998 §1250
///     property is straight-line *for the AMT* while possibly 150%-DB for the regular tax. Dropping
///     those three words would re-admit the land improvements that v3 got wrong;
///   - "Property for which you elected to use the alternative depreciation system (ADS) of section
///     168(g) for the regular tax" (no date limit).
///
/// Passive, at-risk, partnership-basis and farm-shelter depreciation route to lines 2m/2n/3 instead, so
/// the prompt's silence on them is correct.
fn amt_depreciation_question_live(ri: &ReturnInputs) -> bool {
    ri.schedule_c
        .as_ref()
        .is_some_and(|c| c.expenses > Usd::ZERO)
}

/// ★★★ **The liveness of the three Form 1098 DECLARATIONS** — the §163(h)(3)(F) mixed-use box, Form
/// 6251 line 3's AMT-qualified dwelling, and the §163(h)(3)(B) debt-limit testimony.
///
/// **R8 / I8 — `schedule_a.is_some() && !form_1098.is_empty()`.** Both conjuncts are load-bearing and
/// each was measured:
///
/// - **The itemize election.** Until T9 this read
///   `.is_some_and(|a| a.mortgage_interest_1098 > Usd::ZERO)`, which already required a Schedule A.
///   Keeping only the document half would refuse a **standard-deduction** filer on a truthful
///   `MortgageWithinDebtLimit = Some(false)` for a $900,000 2019 loan — a refusal over a deduction
///   they are not claiming, and a return they could not file. `AmtQualifiedDwelling` is Form 6251
///   line 3, whose own text begins *"If you deducted home mortgage interest on Schedule A"*.
/// - **The document.** An itemizer with no Form 1098 row has no line 8a for any of the three to
///   modify, and the mixed-use checkbox is printed on line 8 beside it.
///
/// ★ Deliberately an INPUT predicate, never "Schedule A files": the latter is compute-dependent and
///   would brick the standard-deduction-wins filer (§2.7, r3 I-2).
fn mortgage_question_live(ri: &ReturnInputs) -> bool {
    ri.schedule_a.is_some() && !ri.form_1098.is_empty()
}

/// ★ THE REGISTRY. Eleven declarations; the liveness lifted from the shipped refusals EXCEPT the two P9
/// corrections — `DependentSpouse` widened to `Mfj || spouse.is_some()` (= P8a I1) and the two foreign
/// questions made live ALWAYS (= §2.9, the circular-liveness bug in shipped code).
/// ★ The refusal DETAIL every census row shares. It is one constant rather than eighteen because the
/// remedy and the reason are identical for all of them; the row's own identity travels in the
/// [`RefuseReason::DocumentCensusUnanswered`] payload, and `screen_inputs` names the document there.
///
/// [`RefuseReason::DocumentCensusUnanswered`]: crate::tax::return_refuse::RefuseReason::DocumentCensusUnanswered
const DOC_CENSUS_UNANSWERED_DETAIL: &str =
    "a document type must be ANSWERED, not merely absent: \"none\" and \"nobody asked\" are the same \
     blank on the printed page and are not the same testimony. A broker that has not mailed yours yet \
     is UNANSWERED, never \"none\" — run `btctax income answer`";

pub const FORM_QUESTIONS: &[FormQuestion] = &[
    FormQuestion {
        id: QuestionId::DependentTaxpayer,
        prompt: "Can someone claim YOU as a dependent on their return?",
        unanswered: RefuseReason::DependentStatusUnanswered,
        unanswered_detail:
            "every return must state whether someone can claim YOU as a dependent (it selects the \
             §63(c)(5) standard-deduction floor and is a checkbox on the 1040) — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.header.can_be_claimed_as_dependent_taxpayer,
        set: |ri, v| ri.header.can_be_claimed_as_dependent_taxpayer = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DependentSpouse,
        prompt: "Can someone claim YOUR SPOUSE as a dependent on their return?",
        unanswered: RefuseReason::DependentSpouseStatusUnanswered,
        unanswered_detail:
            "this return has (or is) a joint filing, so it must state whether someone can claim YOUR \
             SPOUSE as a dependent (it is a checkbox on the 1040) — run `btctax income answer`",
        // ★ = P8a I1: MFJ makes the box live even when the spouse `Person` is absent; a stale spouse on a
        // non-MFJ return is a recorded over-ask (§3.1), never an under-ask.
        live: |ri| ri.filing_status == FilingStatus::Mfj || ri.header.spouse.is_some(),
        get: |ri| ri.header.can_be_claimed_as_dependent_spouse,
        set: |ri, v| ri.header.can_be_claimed_as_dependent_spouse = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::MfsSpouseItemizes,
        prompt: "Does your spouse ITEMIZE deductions on their separate return? (§63(c)(6) forces your \
                 choice to match theirs)",
        unanswered: RefuseReason::MfsSpouseItemizeUnknown,
        unanswered_detail:
            "a married-filing-separately return must state whether the spouse itemizes (§63(c)(6)) — \
             run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Mfs,
        get: |ri| ri.mfs_spouse_itemizes,
        set: |ri, v| ri.mfs_spouse_itemizes = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ForeignAccounts,
        prompt: "Schedule B line 7a: did you have a financial interest in, or signature authority over, \
                 a FOREIGN financial account?",
        unanswered: RefuseReason::ScheduleBPart3Unanswered,
        unanswered_detail:
            "Schedule B Part III line 7a (a foreign financial account) must be answered on every return — \
             it is the FBAR/FinCEN disclosure, and its own answer is what decides whether Schedule B files — \
             run `btctax income answer`",
        // ★ = §2.9: live ALWAYS. It CANNOT be scoped by `schedule_b_files`, because that predicate reads
        // this very answer — the circular liveness that silently omitted Schedule B in shipped code.
        live: |_ri| true,
        get: |ri| ri.foreign_accounts,
        set: |ri, v| ri.foreign_accounts = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ForeignTrust,
        prompt: "Schedule B line 8: did you receive a distribution from — or were you the grantor of, or \
                 transferor to — a FOREIGN TRUST?",
        unanswered: RefuseReason::ScheduleBPart3Unanswered,
        unanswered_detail:
            "Schedule B Part III line 8 (a foreign trust) must be answered on every return — a foreign \
             trust independently requires Part III, so it cannot be scoped by whether Schedule B otherwise \
             files — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.foreign_trust,
        set: |ri, v| ri.foreign_trust = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaActivity,
        prompt: "In this tax year, did ANY of these happen with a health savings account? — (a) anyone \
                 (you, your employer, or anyone else on your behalf) put money into one for you; (b) you \
                 took money out of one; (c) you inherited one; or (d) you stopped being HSA-eligible after \
                 using the last-month rule or an IRA-to-HSA funding distribution in a prior year.",
        unanswered: RefuseReason::HsaActivityUnanswered,
        unanswered_detail:
            "a return must state whether a Form 8889 trigger fired for a health savings account (a \
             contribution by anyone, a distribution, a testing-period inclusion, or an inheritance) — an \
             unasked distribution omits gross income and a 20% additional tax (§223(f)) — run `btctax \
             income answer`",
        live: |_ri| true,
        get: |ri| ri.sch1.hsa_activity,
        set: |ri, v| ri.sch1.hsa_activity = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DualStatusAlien,
        prompt: "Were you a DUAL-STATUS ALIEN this year (a nonresident alien for part of the year and a \
                 resident for the rest)?",
        unanswered: RefuseReason::DualStatusAlienUnanswered,
        unanswered_detail:
            "a return must state whether you were a dual-status alien — the 1040 header prints that box, \
             and §63(c)(6)(B) zeroes a nonresident alien's standard deduction — run `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.dual_status_alien,
        set: |ri, v| ri.dual_status_alien = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::MortgageAllUsedToBuyBuildImprove,
        prompt: "Did you use ALL of your home-mortgage loan(s) to buy, build, or improve that home? \
                 (Schedule A line 8: if not, the box is checked.)",
        unanswered: RefuseReason::MixedUseMortgageUnanswered,
        unanswered_detail:
            "this Schedule A reports mortgage interest, so it must state whether the loan(s) were all used \
             to buy, build, or improve the home (§163(h)(3)(F) — Schedule A line 8) — run `btctax income \
             answer`",
        live: mortgage_question_live,
        get: |ri| {
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_all_used_to_buy_build_improve)
        },
        // Live requires `schedule_a.is_some()`, so the `if let` always fires when this is called on a live
        // question; the guard is defensive (a caller that set on a non-live question is a no-op, not a panic).
        set: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.mortgage_all_used_to_buy_build_improve = Some(v);
            }
        },
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // §2.7: "yes, all of it" keeps Schedule A line 8a full
    },
    FormQuestion {
        id: QuestionId::AmtQualifiedDwelling,
        prompt: "Is the home your Form 1098 mortgage interest relates to a principal residence, or a \
                 house, apartment, condominium or mobile home NOT used on a transient basis? (Form 6251 \
                 line 3 — a houseboat or recreational vehicle is NOT an AMT-qualified dwelling.)",
        unanswered: RefuseReason::AmtQualifiedDwellingUnanswered,
        unanswered_detail:
            "this Schedule A reports mortgage interest, so Form 6251 line 3 must know whether the dwelling \
             is AMT-qualified — interest on a dwelling that is not a principal residence or an \
             AMT-qualified dwelling is ADDED BACK for the alternative minimum tax (i6251, Line 3). \
             Guessing would understate the tax — run `btctax income answer`",
        live: mortgage_question_live,
        get: |ri| {
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_dwelling_is_amt_qualified)
        },
        set: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.mortgage_dwelling_is_amt_qualified = Some(v);
            }
        },
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // "yes, AMT-qualified" ⇒ Form 6251 line 3 adds nothing back
    },
    FormQuestion {
        id: QuestionId::AmtCarryoverSameAsRegular,
        prompt: "Is your capital-loss carryover for the alternative minimum tax the SAME as your \
                 regular-tax carryover? (Form 6251 line 2k — answer no if you have ever tracked a \
                 separate AMT basis or AMT capital-loss carryforward.)",
        unanswered: RefuseReason::AmtCarryoverDeclarationUnanswered,
        unanswered_detail:
            "this return carries a capital-loss carryforward, so Form 6251 line 2k must know whether the \
             AMT carryover differs from the regular-tax one — btctax tracks only the regular figure, and \
             a divergent AMT twin is an ADD-BACK. Guessing would understate the tax — run \
             `btctax income answer`",
        live: amt_carryover_question_live,
        get: |ri| ri.amt_carryover_same_as_regular,
        set: |ri, v| ri.amt_carryover_same_as_regular = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // "yes, the same" ⇒ Form 6251 line 2k adds nothing back
    },
    FormQuestion {
        id: QuestionId::AmtDepreciationSameAsRegular,
        prompt: "Is the depreciation included in your Schedule C expenses the SAME for the alternative \
                 minimum tax as for the regular tax? (Form 6251 line 2l.) Answer YES only if one of \
                 these is true of EVERY depreciable asset in that total: you claimed no depreciation at \
                 all, and none was capitalized into inventory; or you deducted its whole cost under \
                 section 179; or it was placed in service after 2015 AND qualified for bonus \
                 depreciation; or it is depreciated STRAIGHT-LINE FOR THE REGULAR TAX and was placed in \
                 service after 1998; or you elected \
                 ADS for it. Answer NO if any asset falls outside that list — in particular anything \
                 placed in service before 1999, 200% declining-balance property from 1999-2015, and \
                 land improvements or other section 1250 property depreciated 150% declining balance. \
                 If you are unsure, answer NO: that refuses the return rather than risking an \
                 understated tax.",
        unanswered: RefuseReason::AmtDepreciationDeclarationUnanswered,
        unanswered_detail:
            "this return carries Schedule C expenses, and btctax accepts that as a FLAT TOTAL — it never \
             sees Part II line 13 ('Depreciation and section 179 expense deduction'), so it cannot tell \
             whether a Form 6251 line 2l adjustment is hiding inside it. A divergent AMT amount is an \
             ADD-BACK, so it is never guessed away: the prompt lists the conditions that permit a yes, \
             and if none clearly applies the answer is NO. Guessing yes would understate the tax — run \
             `btctax income answer`",
        live: amt_depreciation_question_live,
        get: |ri| ri.amt_depreciation_same_as_regular,
        set: |ri, v| ri.amt_depreciation_same_as_regular = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: true, // "yes, the same" ⇒ Form 6251 line 2l adds nothing back
    },
    FormQuestion {
        id: QuestionId::HasIncomeExclusion,
        prompt: "Did you exclude any income from gross income under section 911 (foreign earned \
                 income or housing), section 931 (American Samoa) or section 933 (Puerto Rico)? \
                 (Schedule 1-A Part I / the Schedule A state-and-local-tax worksheet — these \
                 exclusions are ADDED BACK to figure modified AGI.)",
        unanswered: RefuseReason::IncomeExclusionUnanswered,
        unanswered_detail:
            "modified AGI is adjusted gross income increased by any §911/931/933 exclusion, and it \
             drives the §164(b) SALT phase-down and all four Schedule 1-A deductions. Treating an \
             unasked exclusion as zero UNDERSTATES modified AGI, which RAISES those deductions and \
             understates the tax — run `btctax income answer`",
        // ★★ §G-15 — YEAR-SCOPED at last. This shipped ALWAYS LIVE because `live` received only
        // `&ReturnInputs`, which carried no tax year, so it could not be scoped to "years that
        // compute modified AGI" — and TY2024 filers were therefore asked a TY2025 question. That was
        // defensible only because a bespoke neutrality proof existed for it: `Some(false)` ⇒
        // modified AGI = AGI, exactly what TY2024's `FlatCap` assumes and never reads, so no TY2024
        // figure could move.
        //
        // ★ The proof does NOT generalise, which is why the workaround had to go rather than be
        // repeated: Schedule 1-A Part IV asks about a deduction that did not exist in TY2024, so a
        // "no" there answers a question with no TY2024 legal meaning — testimony about nothing.
        //
        // ★ `ReturnInputs::tax_year` is stamped from the storage row key on read, so this predicate
        // reads a year that is true by construction. A year-0 (never stored, never stated) fixture
        // is NOT ≥ 2025, so it is not live — which is the fail-closed direction: an unstated year
        // must not conjure a TY2025 question.
        live: |ri| ri.tax_year >= 2025,
        get: |ri| ri.has_income_exclusion,
        set: |ri, v| ri.has_income_exclusion = Some(v),
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false, // "no exclusions" is the AMT/MAGI-neutral answer, but it is still an ANSWER
    },
    // ── §G-22 / B11: the SCOPE ATTESTATION. ─────────────────────────────────────────────────────────
    FormQuestion {
        id: QuestionId::OtherOutOfScopeIncome,
        prompt: "In this tax year, did ANY of these happen? (a) You received income other than what \
                 you have entered here — a PENSION, ANNUITY or IRA DISTRIBUTION (Form 1099-R), \
                 SOCIAL SECURITY or railroad retirement benefits (Form SSA-1099 or RRB-1099), rent \
                 or royalties, a farm, a partnership, S corporation, estate or trust (any Schedule \
                 K-1), unreported tips, gambling winnings, alimony, a business this tool did not \
                 capture, or anything else it never asked about. EACH OF THESE IS ITS OWN LINE OF \
                 THE RETURN, in the form's own words, and this question is the only thing that asks \
                 about any of them: Household employee wages not reported on a Form W-2; Tip income \
                 not reported on line 1a; Medicaid waiver payments; wages from Form 8919 for work \
                 an employer treated as non-employee; Other earned income; a Nontaxable combat pay \
                 election; Alimony received, and the date of the divorce or separation agreement; \
                 other gains or losses from a sale of business property (Form 4797); Farm income; a \
                 Net operating loss carried in; the foreign earned income exclusion (Form 2555); \
                 income from an Archer MSA or a long-term-care contract (Form 8853); Alaska \
                 Permanent Fund dividends; Jury duty pay; Prizes and awards; Activity not engaged \
                 in for profit income; Stock options; income from the rental of personal property; \
                 Olympic and Paralympic medals and USOC prize money; a section 951(a) or 951A(a) \
                 inclusion from a foreign corporation; a taxable distribution from an ABLE account; \
                 Scholarship and fellowship grants not reported on a Form W-2; a pension or annuity \
                 from a nonqualified deferred compensation or section 457 plan; Wages earned while \
                 incarcerated; or anything you would enter as an 8z write-in on Schedule 1. It also \
                 covers a capital transaction this tool never asked about — an installment sale \
                 (Form 6252), a casualty or theft loss (Form 4684), a section 1256 contract or \
                 straddle (Form 6781), a like-kind exchange (Form 8824), or an undistributed \
                 capital gain (Form 2439) — each of which puts a figure on Schedule D that btctax \
                 cannot see. And it covers OTHER INCOME ON A SCHEDULE C, including a federal or \
                 state gasoline or fuel tax credit or refund (Form 4136) — Schedule C line 6, which \
                 btctax never asks about and leaves blank, so a blank there would UNDERSTATE your \
                 business income. (b) You \
                 EXERCISED AN INCENTIVE STOCK OPTION (ISO) and still held the stock at the end of the \
                 year — you would have a Form 3921. (c) You had any other item this tool never asked \
                 about that changes your ALTERNATIVE MINIMUM TAX — depletion, a tax-shelter farm \
                 activity, a passive activity, or research and experimental costs. (d) You owe an \
                 ADDITION TO TAX that is not income tax on income — an additional tax on an IRA or \
                 other tax-favored account (Form 5329), HOUSEHOLD EMPLOYMENT TAXES for someone you \
                 paid to work in your home (Schedule H), repayment of an excess advance premium tax \
                 credit from a Marketplace health plan (Form 8962), or recapture of a federal \
                 mortgage subsidy. IT ALSO COVERS, EACH ITS OWN LINE OF SCHEDULE 2: a repayment of \
                 a clean vehicle credit you transferred to a dealer (Form 8936); an excessive \
                 payment or recapture reported on Form 4255; Social Security and Medicare tax on \
                 tips you did not report to your employer (Form 4137), or uncollected on wages \
                 (Form 8919); repayment of the first-time homebuyer credit (Form 5405); interest on \
                 tax due on an installment sale; recapture of the low-income housing credit (Form \
                 8611); a Recapture of other credits write-in; recapture of a charitable deduction \
                 for a fractional interest in tangible personal property; income from a section \
                 409A or a section 457A nonqualified deferred compensation plan; a section 72(m)(5) \
                 excess benefits tax; tax on an accumulation distribution of a trust (Form 4970); \
                 an excise tax on insider stock compensation from an expatriated corporation; \
                 look-back interest (Form 8697 or Form 8866); interest from Form 8621 on a passive \
                 foreign investment company; a section 965 installment (Form 965-A); an estimated \
                 tax penalty you want to figure yourself rather than be billed for (Form 2210); and \
                 any other write-in taxes. It also covers an EXEMPTION from self-employment tax you \
                 hold IRS approval for — Form 4361 (a minister, member of a religious order, or \
                 Christian Science practitioner) or Form 4029 (a member of a recognised religious \
                 sect) — and the write-in exemption cases beside them, a notary public's fees among \
                 them. (e) Your tax for the year comes from a form this tool does not fill — the \
                 parent's election to report a child's interest and dividends (Form 8814), the tax \
                 on a lump-sum distribution (Form 4972), or any other alternative form whose amount \
                 belongs on Form 1040 line 16.",
        unanswered: RefuseReason::OtherIncomeUnanswered,
        unanswered_detail:
            "btctax asks about HSA activity, dual-status alien status and foreign accounts, and a \
             `yes` to any of them stops the return — but it never asked whether you had rental, \
             royalty, farm or K-1 income, so silence LOOKED like `none` and a return could file with \
             §61 income left off it. It now also asks whether you EXERCISED AN INCENTIVE STOCK OPTION \
             (Form 6251 line 2i), which is not income at all for the regular tax and so was invisible \
             to the income half of this question. Silence is not testimony that there is none: answer \
             it — run `btctax income answer`",
        // ★★★ LIMB (b) IS AN ISO EXERCISE, AND IT IS NOT INCOME — which is exactly why it had to be
        //     added here rather than left to the income half. i6251, first sentence of the line-2i
        //     instruction: *"For the regular tax, no income is recognized when an incentive stock
        //     option (ISO), as defined in section 422(b), is exercised. However, this rule doesn't
        //     apply for the AMT."* So a truthful filer with a $180,000 ISO adjustment answered the OLD
        //     prompt — *"did you RECEIVE any income…"* — with a truthful **No**, and the gate stayed
        //     shut.
        //
        //     ★★★ AND THE GAP HID ITS OWN DETECTION. `Form6251::must_attach()` is `line7 > line10`,
        //     and the missing 2i add-back is exactly what would have pushed line 7 past line 10. So
        //     the return did not merely print a wrong 2i — it never tripped `AmtScreenTriggered` at
        //     all and filed clean, with no Form 6251 and no AMT, on a return signed under §6065.
        //     Understating, invisible to both oracles, and invisible to every value test.
        //
        //     ★★ Limb (c) is the same argument generalised: `form6251.rs` models lines 2, 2a and 2b
        //     only, so every other Part I add-back (2c–2t) is silently zero. Naming the four that a
        //     btctax filer could plausibly have — rather than asking "any AMT item?" — follows the
        //     enumerate-the-YES-conditions rule: a filer cannot answer `no` to a category they were
        //     never shown.
        //
        //     ★ WIDENING a mandatory question's YES-conditions is the SAFE direction of edit, which is
        //     why it is done here rather than by adding a second question. The unsafe direction —
        //     widening an EXEMPTION — is what `widening-an-exemption-is-never-the-safe-edit` names.
        //
        // ★★★ ALWAYS LIVE, and deliberately so. Every other class-(A) declaration is scoped to the
        // years or shapes that read it; this one is read by NOTHING — it exists precisely because
        // there is no field for the income it asks about. A liveness predicate here could only be
        // "was the filer likely to have some?", which is the guess the question exists to refuse to
        // make. ★ It is also stable across years in a way per-schedule questions are not: the
        // out-of-scope SET moves every tax year, the union does not.
        live: |_| true,
        get: |ri| ri.other_out_of_scope_income,
        set: |ri, v| ri.other_out_of_scope_income = Some(v),
        durability: Durability::PerYear,
        // ★★ NOT neutral. "No other income" is an affirmative statement about the filer's year that no
        // default may make on their behalf — that is the whole finding. `false` here would let the
        // answer be assumed, restoring the silence this question exists to break.
        neutral: false,
    },
    // ── §G-9: the §63(f) death carve-out. Two entries, because i1040gi states the rule twice — once
    // under "Death of a taxpayer" and once under "Death of spouse" — and each is a separate fact.
    //

    // ★★★ §163(h)(3)(B) — THE ACQUISITION-DEBT CEILING. Index 13; APPENDED AT THE END for the
    //     array-index reason stated on `QuestionId::MortgageWithinDebtLimit`.
    //
    // ★★ THE PROMPT ENUMERATES THE YES-CONDITIONS AND FALLS BACK TO NO, which is the whole shape of
    //    it (`widening-an-exemption-is-never-the-safe-edit`). i1040sca states FOUR limits under
    //    *"Limits on home mortgage interest"*; one of them — the mixed-use limit — is already its own
    //    question (index 7), so the three AMOUNT limits are listed here individually. A vaguer
    //    "were you within the limits?" would be laundered into three answers the filer never gave,
    //    and each omission would fail OPEN into a full deduction the statute caps.
    //
    // ★ Same liveness as the other two mortgage questions — `mortgage_question_live`, the EXISTING
    //   predicate, unchanged. A Schedule A that reports 1098 interest is exactly the return on which
    //   line 8a can be wrong, and the limit is not derivable from anything btctax holds: it collects
    //   the INTEREST, never the balance, the origination date, or the home's fair market value.
    FormQuestion {
        id: QuestionId::MortgageWithinDebtLimit,
        prompt: "Schedule A line 8a — were you inside EVERY home-mortgage debt limit this year? \
                 Answer YES only if all of these are true of your home mortgages counted together: \
                 (a) qualifying debt taken out AFTER December 15, 2017 never exceeded $750,000 \
                 ($375,000 if married filing separately); (b) qualifying debt taken out ON OR BEFORE \
                 December 15, 2017 never exceeded $1,000,000 ($500,000 if married filing separately) \
                 — and if you have both kinds, the $750,000 limit is REDUCED by the amount of the \
                 older debt; and (c) the total of all your mortgages was never more than the home's \
                 fair market value. Answer NO if any one of them was exceeded, and answer NO if you \
                 are unsure: a NO refuses the return and sends you to Pub. 936's Deductible Home \
                 Mortgage Interest Worksheet, rather than risking an understated tax.",
        unanswered: RefuseReason::MortgageDebtLimitUnanswered,
        unanswered_detail:
            "this Schedule A reports mortgage interest, so it must state whether your combined home \
             acquisition debt stayed inside the §163(h)(3)(B) limits (i1040sca, \"Limits on home \
             mortgage interest\": $750,000/$375,000 for qualifying debt taken out after December 15, \
             2017; $1,000,000/$500,000 for debt taken out on or before it; and the home's fair market \
             value). btctax collects the INTEREST, never the balance — so left unasked it deducts the \
             whole Form 1098 amount, which for a filer over the limit UNDERSTATES the tax. Neither \
             oracle can catch that: both take line 8a as an INPUT (§G-9). Run `btctax income answer`",
        live: mortgage_question_live,
        get: |ri| {
            ri.schedule_a
                .as_ref()
                .and_then(|a| a.mortgage_within_debt_limit)
        },
        set: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.mortgage_within_debt_limit = Some(v);
            }
        },
        // ★ §G-15 — every class-(A) DECLARATION asserts about a TAX YEAR ("in this tax year, did…"),
        // so none is durable: last year's answer is not testimony for this one. Debt balances move.
        durability: Durability::PerYear,
        neutral: true, // "yes, inside every limit" ⇒ Schedule A line 8a stays the full 1098 amount
    },
    // ★★★ SCHEDULE D LINE 20 / SCHEDULE A LINE 9 — THE FORM 4952 DECLARATION. Index 14; appended at
    //     the END for the array-index reason above.
    //
    // ★★★ ALWAYS LIVE, and it has to be. Line 20 prints on every return whose Schedule D routes
    //     both-gains, and that routing comes from the LEDGER — which `live` (a `&ReturnInputs`
    //     predicate) cannot see. Scoping it to `schedule_a.is_some()` would under-ask exactly the
    //     population the plan measured: the $0-income, standard-deduction crypto household whose
    //     Schedule D still prints the same sworn "Yes". A question that vanishes for the filer it was
    //     written for is the shape §G-9 exists to kill.
    //
    // ★★ THE PROMPT ENUMERATES WHEN FORM 4952 IS REQUIRED and falls back to YES — which is the
    //    fail-closed direction here, because YES refuses and refusals are recoverable. The list is
    //    i4952's own exception, negated: you may answer NO only if every one of its three conditions
    //    holds. A filer who is unsure answers YES and gets a refusal they can undo, rather than a
    //    filed return whose line 20 they never saw.
    FormQuestion {
        id: QuestionId::FilingForm4952,
        prompt: "Are you filing Form 4952 (Investment Interest Expense Deduction)? Answer NO only \
                 if ALL THREE of these are true — they are Form 4952's own exception: (a) your \
                 investment interest expense is not more than your investment income from interest \
                 and ordinary dividends minus any qualified dividends; (b) you have no other \
                 deductible investment expenses; and (c) you have no disallowed investment interest \
                 expense carried over from last year. Answer YES if you borrowed to invest and any \
                 of those fails, and answer YES if you are unsure: a YES refuses the return rather \
                 than filing a Schedule D line 20 that swears you are not filing a form you are. \
                 (Schedule D line 20 asks \"Are lines 18 and 19 both zero or blank and you are not \
                 filing Form 4952?\"; Schedule A line 9 is \"Investment interest. Attach Form 4952 \
                 if required.\")",
        unanswered: RefuseReason::Form4952DeclarationUnanswered,
        unanswered_detail:
            "Schedule D line 20 asks whether lines 18 and 19 are both zero or blank AND you are not \
             filing Form 4952 — and its answer decides which worksheet computes your tax. btctax \
             checked \"Yes\" on every both-gains return without ever asking you, which is sworn \
             testimony it invented. It will not do that: answer it and the return files (a \"No\" \
             routes to the Schedule D Tax Worksheet, which btctax does not fill). The same answer \
             governs Schedule A line 9, investment interest — run `btctax income answer`",
        // ★ See the block comment above: it CANNOT be scoped by whether Schedule D files, because
        //   that is a ledger fact and `live` receives only `ReturnInputs`.
        live: |_ri| true,
        get: |ri| ri.filing_form_4952,
        set: |ri, v| ri.filing_form_4952 = Some(v),
        // ★ §G-15 — PER-YEAR: whether you file Form 4952 is a fact about this tax year.
        durability: Durability::PerYear,
        // ★ NOT filing Form 4952 is the answer that needs no form btctax lacks: line 20 = Yes ⇒ the
        //   Qualified Dividends and Capital Gain Tax Worksheet, which btctax does compute.
        neutral: false,
    },
    // ★★★ THE CAPITAL LOSS CARRYOVER WORKSHEET'S TWO UNNUMBERED HEADER CONDITIONS. Indices 15 and 16;
    //     appended at the END for the `decl_tristate!` array-index reason above.
    //
    // ★★★ WHY THEY EXIST AT ALL. The worksheet header states two governing conditions in prose, above
    //     line 1 — and the conformance checker's completeness half reads only physical lines beginning
    //     `N.`, so both were STRUCTURALLY INVISIBLE to it and were dropped while it stayed green
    //     (`xtask::capital_loss_carryover_check::unnumbered_conditions_in_the_form` is the half that
    //     now sees them). `CLAUDE.md`: *"If the form asks something our input surface cannot answer,
    //     collect it. That is following instructions, not scope creep."*
    //
    // ★★★ AND WHY THEY BECAME LOAD-BEARING NOW. Before `--write-carryover` rolled the §1212(b)
    //     figure, a mis-attributed or unreduced carryover was at worst the filer's own bad input.
    //     After it, btctax re-emits that figure as its OWN `Computed` value on next year's Schedule D
    //     lines 6 and 14 — sworn under §6065. That is the one edit that turns a user error into a
    //     btctax assertion, so both questions fail CLOSED: `None` refuses, and so does `Some(true)`.
    FormQuestion {
        id: QuestionId::CarryoverIncludesSpousesJointLoss,
        prompt: "Does any part of your capital-loss carryover come from a JOINT return for a year \
                 you are now filing separately from, where the loss was your SPOUSE'S? (Capital Loss \
                 Carryover Worksheet header: \"If you and your spouse once filed a joint return and \
                 are filing separate returns for 2025, any capital loss carryover from the joint \
                 return can be deducted only on the return of the spouse who actually had the \
                 loss.\") Answer NO only if the whole carryover is your own loss — because you have \
                 never filed jointly, or because you are still filing jointly with the same spouse, \
                 or because every dollar of it was realized on property that was yours. Answer YES \
                 if any part of it was your spouse's, and answer YES if you are unsure: a YES \
                 refuses the return rather than deducting a loss that is not yours.",
        unanswered: RefuseReason::JointReturnCarryoverDeclarationUnanswered,
        unanswered_detail:
            "this return carries a capital-loss carryforward, and the Capital Loss Carryover \
             Worksheet's header says a carryover from a joint return \"can be deducted only on the \
             return of the spouse who actually had the loss\" (§1212(b)). btctax stores ONE \
             carryover per return and has no way to tell whose loss it was, so it cannot make that \
             split for you — and with `--write-carryover` it would re-emit the figure as its own \
             computed entry on next year's Schedule D lines 6 and 14. Run `btctax income answer`",
        live: carryforward_in_present,
        get: |ri| ri.carryover_includes_spouses_joint_loss,
        set: |ri, v| ri.carryover_includes_spouses_joint_loss = Some(v),
        // ★ §G-15 — PER-YEAR. Filing status changes; so does which spouse's loss is still running.
        durability: Durability::PerYear,
        // ★ NOT neutral at true: a YES is the ADVERSE answer here, and it refuses. `false` — "all of
        //   it is mine" — is the answer that needs no split btctax cannot perform.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ExcludedCanceledDebt,
        prompt: "Did you exclude cancelled or forgiven debt from your income this year — for \
                 example under the insolvency, bankruptcy, or qualified-principal-residence rules \
                 (Form 982)? (Capital Loss Carryover Worksheet header: \"If you excluded canceled \
                 debt from income in 2025, see Pub. 4681.\") Answer NO only if you excluded none. \
                 Answer YES if you filed or should have filed Form 982, and answer YES if you are \
                 unsure: a YES refuses the return rather than carrying forward a loss that \
                 §108(b)(2)(G) requires you to reduce.",
        unanswered: RefuseReason::ExcludedCanceledDebtDeclarationUnanswered,
        unanswered_detail:
            "this return carries a capital-loss carryforward, and the Capital Loss Carryover \
             Worksheet's header sends a filer who excluded canceled debt to Pub. 4681 — because \
             §108(b) then requires TAX ATTRIBUTE REDUCTION, and §108(b)(2)(G) puts capital loss \
             carryovers on that list. btctax models no part of §108(b), so a carryover it has not \
             been told to reduce is too large, and with `--write-carryover` it would persist that \
             figure as next year's computed input. Run `btctax income answer`",
        live: carryforward_in_present,
        get: |ri| ri.excluded_canceled_debt,
        set: |ri, v| ri.excluded_canceled_debt = Some(v),
        // ★ §G-15 — PER-YEAR: a debt exclusion is an event of one tax year.
        durability: Durability::PerYear,
        // ★ NOT neutral at true: a YES is the ADVERSE answer and refuses.
        neutral: false,
    },
    // ── ★★★ R3 / §5.1 — THE DOCUMENT CENSUS. Eighteen rows, indices 17..=34. ────────────────────
    //
    // ★ Every row's `prompt` and `unanswered_detail` come from [`DocumentRow`] itself, so the words
    //   the filer reads, the words the refusal names and the words `income answer` prints are ONE
    //   string — the `prompt_hash` would otherwise disagree with itself across surfaces.
    //
    // ★ `neutral: false` on every row: "no, I received none" is the answer that needs no section and
    //   forgoes nothing. It is still an ANSWER — the whole point of the census is that a `false` and
    //   an absence are different testimony.
    //
    // ★ APPENDED AT THE END for the `decl_tristate!` array-index reason recorded above.

    FormQuestion {
        id: QuestionId::DocW2,
        prompt: crate::tax::document_census::DocumentRow::W2.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::W2,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::W2,
            )
        },
        get: |ri| ri.documents.w2,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::W2,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocInt1099,
        prompt: crate::tax::document_census::DocumentRow::Int1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Int1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Int1099,
            )
        },
        get: |ri| ri.documents.int_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Int1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocDiv1099,
        prompt: crate::tax::document_census::DocumentRow::Div1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Div1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Div1099,
            )
        },
        get: |ri| ri.documents.div_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Div1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocB1099,
        prompt: crate::tax::document_census::DocumentRow::B1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::B1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::B1099,
            )
        },
        get: |ri| ri.documents.b_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::B1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocG1099,
        prompt: crate::tax::document_census::DocumentRow::G1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::G1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::G1099,
            )
        },
        get: |ri| ri.documents.g_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::G1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocForm1098,
        prompt: crate::tax::document_census::DocumentRow::Form1098.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Form1098,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Form1098,
            )
        },
        get: |ri| ri.documents.form_1098,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Form1098,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocForm1098e,
        prompt: crate::tax::document_census::DocumentRow::Form1098e.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Form1098e,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Form1098e,
            )
        },
        get: |ri| ri.documents.form_1098e,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Form1098e,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocR1099,
        prompt: crate::tax::document_census::DocumentRow::R1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::R1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::R1099,
            )
        },
        get: |ri| ri.documents.r_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::R1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocSsa1099,
        prompt: crate::tax::document_census::DocumentRow::Ssa1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Ssa1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Ssa1099,
            )
        },
        get: |ri| ri.documents.ssa_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Ssa1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocNecMiscK1099,
        prompt: crate::tax::document_census::DocumentRow::NecMiscK1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::NecMiscK1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::NecMiscK1099,
            )
        },
        get: |ri| ri.documents.nec_misc_k_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::NecMiscK1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocK1,
        prompt: crate::tax::document_census::DocumentRow::K1.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::K1,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::K1,
            )
        },
        get: |ri| ri.documents.k1,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::K1,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocScheduleERental,
        prompt: crate::tax::document_census::DocumentRow::ScheduleERental.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::ScheduleERental,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::ScheduleERental,
            )
        },
        get: |ri| ri.documents.schedule_e_rental,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::ScheduleERental,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocS1099,
        prompt: crate::tax::document_census::DocumentRow::S1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::S1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::S1099,
            )
        },
        get: |ri| ri.documents.s_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::S1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocOid1099,
        prompt: crate::tax::document_census::DocumentRow::Oid1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Oid1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Oid1099,
            )
        },
        get: |ri| ri.documents.oid_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Oid1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocW2g,
        prompt: crate::tax::document_census::DocumentRow::W2g.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::W2g,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::W2g,
            )
        },
        get: |ri| ri.documents.w2g,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::W2g,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocC1099,
        prompt: crate::tax::document_census::DocumentRow::C1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::C1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::C1099,
            )
        },
        get: |ri| ri.documents.c_1099,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::C1099,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocA1095,
        prompt: crate::tax::document_census::DocumentRow::A1095.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::A1095,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::A1095,
            )
        },
        get: |ri| ri.documents.a_1095,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::A1095,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocT1098,
        prompt: crate::tax::document_census::DocumentRow::T1098.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::T1098,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::T1098,
            )
        },
        get: |ri| ri.documents.t_1098,
        // ★★★ R10.4 / T4b — THE ONE WRITER of a census answer. A `No` also removes the
        //     opener's PRE-NAMED rows, so a filer who says the document did not arrive is
        //     never left holding a row they did not type beside the contradiction refusal.
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::T1098,
                v,
            );
        },
        // ★ §G-15 — PER-YEAR: which documents arrived is a fact about ONE tax year, and last
        //   year's shoebox is not testimony for this one.
        durability: Durability::PerYear,
        neutral: false,
    },
    // ── ★★★ R10.4 / T4b seam review I-1 — THE CARRIED FILING STATUS'S OWN SURFACE ───────────────
    //
    // APPENDED AT THE END for the `decl_tristate!` array-index reason recorded above.
    FormQuestion {
        id: QuestionId::FilingStatusConfirmed,
        // ★ The STATIC fallback. The words actually put to the filer are rendered from the return by
        //   [`FormQuestion::prompt_text`] — they quote the status and both years — and that is what
        //   `record_answer` hashes, so CHANGING the status changes the hash and R10.3's re-ask rule
        //   returns this question to unanswered with no code of its own.
        prompt: "Is the filing status carried from last year's return still your filing status for \
                 this tax year? (Marital status is determined on the last day of the tax year — Form \
                 1040 instructions, Filing Status.)",
        unanswered: RefuseReason::FilingStatusUnconfirmed,
        unanswered_detail:
            "this return was opened from the prior year, which carried its FILING STATUS forward — \
             and §7703(a)(1) determines marital status on the LAST DAY of the tax year, so last \
             year's status is not testimony for this one. Confirm it (or say no and change it) — run \
             `btctax income answer`",
        // ★ Live ONLY on a year the opener made. A year the filer started themselves stated its own
        //   status through serde (`filing_status` has no `#[serde(default)]`), which is the ground
        //   the classifier's exemption cites; this question is the ground on the other path.
        live: |ri| ri.opened_from.is_some(),
        get: |ri| ri.filing_status_confirmed,
        set: |ri, v| ri.filing_status_confirmed = Some(v),
        // ★ §G-15 — PER-YEAR by statute: §7703(a)(1) redetermines marital status every December 31.
        durability: Durability::PerYear,
        // ★ Neutral at TRUE: "yes, unchanged" is the answer that needs no adjustment. A `No` REFUSES
        //   (`screen_inputs`), because the status the return would compute on is the wrong one.
        neutral: true,
    },
    // ── ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. Four entries, indices 36..=39. ─────────────
    //
    // ★ *"A census `No` never closes income the instructions say to report WITHOUT the document."*
    //   The document-first rule is right for amounts and wrong as a stop: the form names incomes
    //   that exist with no information return behind them, and each is small money in the
    //   UNDERSTATEMENT direction, where the residual attestation is the only net.
    //
    // ★ APPENDED AT THE END for the `decl_tristate!` array-index reason recorded above.
    FormQuestion {
        id: QuestionId::WagesWithoutW2Question,
        prompt: "Did you receive wages, salary or tips from an employer who issued no Form W-2? \
                 (Form 1040 line 1a instructions: \"Even if you don't get a Form W-2, you must \
                 still report your earnings.\")",
        unanswered: RefuseReason::WagesWithoutW2Unanswered,
        unanswered_detail:
            "you answered that you received NO Form W-2, and the Form 1040 instructions say \
             earnings must be reported whether or not the form arrives — so btctax has to ask \
             whether there were any. It is not a formality: wages with no W-2 behind them are the \
             commonest income a document-first interview would drop, and dropping income \
             UNDERSTATES the tax on a return signed under §6065. Run `btctax income answer`",
        // ★ Live EXACTLY on its census row's `Some(false)` — the R3 pairing. A filer who received a
        //   W-2 is never asked it (their wages have a document and a section).
        live: |ri| {
            ri.documents.get(crate::tax::document_census::DocumentRow::W2) == Some(false)
        },
        get: |ri| ri.w2_wages_without_w2,
        set: |ri, v| ri.w2_wages_without_w2 = Some(v),
        // ★ §G-15 — PER-YEAR: whether an undocumented employer paid you is a fact about ONE year.
        durability: Durability::PerYear,
        // ★ Neutral at FALSE: "no such earnings" needs no section and forgoes nothing. A `Yes`
        //   REFUSES — btctax has no surface for wages that arrive without the document.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::InterestOrDividendsWithout1099,
        prompt: "Did you receive taxable interest or dividends for which no Form 1099-INT or Form \
                 1099-DIV was issued — a bank paying under $10, a seller-financed mortgage you \
                 hold, or a nominee distribution? (Schedule B line 1: \"Report on line 1 all of \
                 your taxable interest.\" Form 1040 line 3b instructions: \"See Pub. 550 … if you \
                 received dividends not reported on Form 1099-DIV.\")",
        unanswered: RefuseReason::InterestOrDividendsWithout1099Unanswered,
        unanswered_detail:
            "you answered that you received no Form 1099-INT and/or no Form 1099-DIV, and Schedule \
             B line 1 says to report ALL of your taxable interest — a payer is not required to \
             issue a 1099-INT below $10, a seller-financed mortgage has no payer at all, and a \
             nominee distribution is reported by someone else. Answering YES opens a place to enter \
             them; answering NO records that there were none. Run `btctax income answer`",
        // ★ ONE question paired with TWO census rows, so EITHER row's `No` opens it: a filer with
        //   1099-INTs but no 1099-DIV can still hold dividends nobody reported.
        live: |ri| {
            ri.documents.get(crate::tax::document_census::DocumentRow::Int1099) == Some(false)
                || ri.documents.get(crate::tax::document_census::DocumentRow::Div1099)
                    == Some(false)
        },
        get: |ri| ri.interest_or_dividends_without_1099,
        set: |ri, v| ri.interest_or_dividends_without_1099 = Some(v),
        durability: Durability::PerYear,
        // ★ Neutral at FALSE. Unlike its two siblings a `Yes` does NOT refuse — it OPENS the
        //   filer's-records rows, because Schedule B lines 1 and 5 take exactly what they carry.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::StateRefundWithout1099g,
        // ★ The STATIC fallback; the words actually shown quote the tax year (`RENDERED_PROMPTS`).
        prompt: "Did you receive a refund, credit or offset of state or local income taxes this \
                 year? (Form 1040 instructions, Schedule 1 line 1: \"Report any taxable refund you \
                 received even if you didn't receive Form 1099-G.\")",
        unanswered: RefuseReason::StateRefundWithout1099gUnanswered,
        unanswered_detail:
            "you answered that you received no Form 1099-G, and the Form 1040 instructions say to \
             report a taxable state or local income tax refund \"even if you didn't receive Form \
             1099-G\" — some states now publish it only electronically, and some never send one. \
             Whether any of it is taxable turns on §111(a), which is asked next. Run `btctax income \
             answer`",
        live: |ri| {
            ri.documents.get(crate::tax::document_census::DocumentRow::G1099) == Some(false)
        },
        get: |ri| ri.state_refund_without_1099g,
        set: |ri, v| ri.state_refund_without_1099g = Some(v),
        durability: Durability::PerYear,
        // ★ Neutral at FALSE: no refund, nothing on Schedule 1 line 1, nothing forgone.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::ItemizedPriorYear,
        // ★ The STATIC fallback; the words shown quote year N−1 (`RENDERED_PROMPTS`).
        prompt: "Did you itemize deductions on your PRIOR-YEAR federal return — that is, did you \
                 file a Schedule A instead of taking the standard deduction? (Form 1040 \
                 instructions, Schedule 1 line 1: none of a state or local income tax refund is \
                 taxable if, in the year you paid the tax, you did not itemize.)",
        unanswered: RefuseReason::ItemizedPriorYearUnanswered,
        unanswered_detail:
            "this return reports a state or local income tax refund, and §111(a)'s TAX-BENEFIT RULE \
             decides whether any of it is income: a refund of tax that never reduced your federal \
             tax is not income at all. The only thing that answers it is whether you ITEMIZED in \
             the year you paid the tax. btctax will not assume either way — assuming \"no\" would \
             blank Schedule 1 line 1 on your behalf and understate the tax. Run `btctax income \
             answer`",
        // ★★★ RETURN-LEVEL, and live from EITHER side, because *a gate may not ride on a row that
        //     might not exist*: the identical answer is owed by a filer with a transcribed 1099-G
        //     box 2 and by one who received the refund with no 1099-G at all (R3/I1).
        // ★ The refund-question limb goes through `question_is_live`, never through a second copy
        //   of that question's own predicate: a STALE `Some(true)` on a return whose 1099-G census
        //   row has since flipped to `Yes` must not keep this gate live, or the filer meets a
        //   blocking question that no surface still asks. The 1099-G limb needs no such guard — a
        //   transcribed box 2 is a figure on the return, not an answer that can go stale.
        live: |ri| {
            (question_is_live(QuestionId::StateRefundWithout1099g, ri)
                && ri.state_refund_without_1099g == Some(true))
                || ri.g_1099.iter().any(|g| g.box2_state_refund > Usd::ZERO)
        },
        get: |ri| ri.itemized_prior_year,
        set: |ri, v| ri.itemized_prior_year = Some(v),
        // ★ §G-15 — PER-YEAR: it names a DIFFERENT prior year every year, so an answer given for
        //   2025's return is not an answer about 2026's.
        durability: Durability::PerYear,
        // ★ Neutral at FALSE: §111(a) then makes none of the refund income and Schedule 1 line 1 is
        //   blank BY DECISION. A `Yes` REFUSES — the State and Local Income Tax Refund Worksheet is
        //   not built.
        neutral: false,
    },
    // ── ★★★ R9 / T6 — THE DIGITAL ASSETS QUESTION. Appended at the END for the `decl_tristate!`
    //    array-index reason recorded above.
    FormQuestion {
        id: QuestionId::DigitalAssetActivity,
        // ★ The STATIC fallback; the words shown quote the year (`RENDERED_PROMPTS`).
        prompt: "At any time during this tax year, did you: (a) receive (as a reward, award, or \
                 payment for property or services); or (b) sell, exchange, or otherwise dispose of \
                 a digital asset (or a financial interest in a digital asset)? (Form 1040 \
                 instructions, Digital Assets: holding a digital asset; moving one between wallets \
                 or accounts you own or control; and purchasing digital assets with U.S. or other \
                 real currency do NOT, alone, require a \"Yes\".)",
        unanswered: RefuseReason::DigitalAssetActivityUnanswered,
        unanswered_detail:
            "Form 1040 page 1 asks the Digital Assets question above line 1a, and the instructions \
             are explicit that it is not optional: \"You must answer the digital asset question on \
             Form 1040 whether or not you received a Form 1099-DA\". btctax will not answer it for \
             you — a \"No\" it cannot vouch for is sworn testimony under §6065, and a blank is a \
             mandatory question left unanswered on a signed return. Run `btctax income answer`",
        // ★★★ ALWAYS LIVE, and NOT scoped to "the ledger has crypto". The form prints the question
        //     on every 1040, so a filer with an empty vault owes the same answer as one with fifty
        //     disposals — and scoping it to the ledger would be the circular liveness §2.9 records
        //     (the answer would be asked only where btctax already knew it).
        live: |_ri| true,
        get: |ri| ri.digital_asset_activity,
        set: |ri, v| ri.digital_asset_activity = Some(v),
        // ★ §G-15 — PER-YEAR: the question is "at any time during <year>", so last year's answer is
        //   not testimony for this one.
        durability: Durability::PerYear,
        // ★ Neutral at FALSE: "no receipt and no disposition" needs no adjustment and forgoes no
        //   benefit. The polarity is declared here rather than inferred — and note that the NEUTRAL
        //   answer is the one the ledger can CONTRADICT, which is why the cross-check lives in
        //   `screen_compute_dependent` and not in this registry: liveness is an input predicate.
        neutral: false,
    },
    // ── ★★★ T16 / FR-76 — the two HSA information-return census rows. ────────────────────────────
    FormQuestion {
        id: QuestionId::DocSa1099,
        prompt: crate::tax::document_census::DocumentRow::Sa1099.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Sa1099,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Sa1099,
            )
        },
        get: |ri| ri.documents.sa_1099,
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Sa1099,
                v,
            );
        },
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::DocSa5498,
        prompt: crate::tax::document_census::DocumentRow::Sa5498.prompt(),
        unanswered: RefuseReason::DocumentCensusUnanswered {
            kind: crate::tax::document_census::DocumentRow::Sa5498,
        },
        unanswered_detail: DOC_CENSUS_UNANSWERED_DETAIL,
        live: |ri| {
            crate::tax::document_census::row_is_live(
                ri,
                crate::tax::document_census::DocumentRow::Sa5498,
            )
        },
        get: |ri| ri.documents.sa_5498,
        set: |ri, v| {
            crate::tax::document_census::answer_row(
                ri,
                crate::tax::document_census::DocumentRow::Sa5498,
                v,
            );
        },
        durability: Durability::PerYear,
        neutral: false,
    },
    // ── ★★★ T16 / FR-76 — FORM 8889's OWN SEVEN QUESTIONS. Every one is live iff the §223 trigger
    //    declaration is affirmed, and every one is a question the FORM asks that nothing else on
    //    the return can answer. ──────────────────────────────────────────────────────────────────
    FormQuestion {
        id: QuestionId::HsaFamilyCoverage,
        prompt: "Form 8889 line 1: was YOUR OWN high-deductible health plan (HDHP) coverage FAMILY \
                 coverage? (Yes = the form's \"Family\" box; No = its \"Self-only\" box. \"If you were \
                 covered, or considered covered, by a self-only HDHP and a family HDHP at different \
                 times during the year, check the box for the plan that was in effect for a longer \
                 period. If you were covered by both a self-only HDHP and a family HDHP at the same \
                 time, you are treated as having family coverage during that period.\" Your \
                 spouse's plan is asked separately by the next question, because the instructions \
                 count it too.)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaFamilyCoverage,
        },
        unanswered_detail: "Form 8889 line 1 asks you to check your HDHP coverage, and line 3's contribution limit is \
             a different figure for each box — btctax will not choose one for you. Run `btctax income \
             answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.family_coverage,
        set: |ri, v| ri.hsa.family_coverage = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaEligibleEveryMonth,
        prompt: "Form 8889 line 3: on the first day of EVERY month this year, were you (or were you \
                 considered) an eligible individual with the SAME coverage? (The last-month rule \
                 counts as \"considered\": if you were an eligible individual on the first day of the \
                 last month of your tax year, you are treated as eligible for the whole year.)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaEligibleEveryMonth,
        },
        unanswered_detail: "Form 8889 line 3 gives one flat limit to a filer eligible with the same coverage every \
             month and sends everyone else to the Line 3 Limitation Chart and Worksheet, which btctax \
             does not carry. Entering the flat limit for a part-year filer would overstate the \
             deduction. Run `btctax income answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.eligible_every_month_same_coverage,
        set: |ri, v| ri.hsa.eligible_every_month_same_coverage = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::HsaAge55OrOlder,
        prompt: "Form 8889 lines 3 and 7: were you age 55 or older at the END of this tax year? (\u{a7}223(b)(3) \
                 lets you contribute an additional $1,000; the form puts it on line 3 if you are \
                 unmarried, or married with self-only coverage all year, and on line 7 if you are \
                 married with family coverage.)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaAge55OrOlder,
        },
        unanswered_detail: "Form 8889's contribution limit is $1,000 higher for a filer who reached 55 by the end of \
             the year (\u{a7}223(b)(3)(B)), and the form asks it on two different lines depending on your \
             marital status and coverage. Run `btctax income answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.age_55_or_older_at_year_end,
        set: |ri, v| ri.hsa.age_55_or_older_at_year_end = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaMedicareEnrollment,
        prompt: "Form 8889 line 3: were you enrolled in MEDICARE for any month this year? (The Line 3 \
                 Limitation Chart's first question, and Part I's rule: \"You cannot deduct any \
                 contributions for any month in which you were enrolled in Medicare.\")",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaMedicareEnrollment,
        },
        unanswered_detail: "Medicare enrolment for even one month means the flat line-3 limit is not your limit — the \
             Line 3 Limitation Chart and Worksheet computes the reduced one, and btctax does not carry \
             it. Run `btctax income answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.enrolled_in_medicare_any_month,
        set: |ri, v| ri.hsa.enrolled_in_medicare_any_month = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaBothSpousesHaveHsas,
        prompt: "Form 8889 Parts I, II and III: do BOTH you and your spouse each have a separate HSA? (The \
                 form's own answer is \"Complete a separate Form 8889 for each spouse\".)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaBothSpousesHaveHsas,
        },
        unanswered_detail: "Form 8889 says to complete a separate form for each spouse who has an HSA, and btctax \
             produces one. Run `btctax income answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.both_spouses_have_hsas,
        set: |ri, v| ri.hsa.both_spouses_have_hsas = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaArcherMsaActivity,
        prompt: "Form 8889 line 4: did you or your employer contribute to an ARCHER MSA this year, or did \
                 you have a Medicare Advantage MSA? (Line 4 takes the amount from Form 8853, lines 1 \
                 and 2, and the form's first instruction is \"Complete Form 8853 \u{2026} if required\".)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaArcherMsaActivity,
        },
        unanswered_detail: "Form 8889 line 4 subtracts your Archer MSA contributions from your HSA limit, and it takes \
             them from Form 8853 — which btctax does not build. A blank line 4 on a return that has \
             Archer MSA contributions overstates the HSA limit. Run `btctax income answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.archer_msa_activity,
        set: |ri, v| ri.hsa.archer_msa_activity = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HsaTestingPeriodFailure,
        prompt: "Form 8889 Part III: did you STOP being an eligible individual during a testing period \
                 this year \u{2014} after using the last-month rule, or after an IRA-to-HSA qualified funding \
                 distribution, in an earlier year? (Part III: \"Income and Additional Tax for Failure \
                 To Maintain HDHP Coverage\".)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaTestingPeriodFailure,
        },
        unanswered_detail: "Failing the testing period puts the earlier year's excess contribution back into income on \
             Schedule 1 line 8f and adds a 10% tax on Schedule 2 line 17d. Computing it needs the \
             PRIOR year's Line 3 Limitation Chart and Worksheet, which btctax carries for no year, so \
             the answer decides whether this return can be filed at all. Run `btctax income answer`",
        live: hsa_question_live,
        get: |ri| ri.hsa.testing_period_failure,
        set: |ri, v| ri.hsa.testing_period_failure = Some(v),
        // ★ §G-15 — PER-YEAR: every one of these asserts about ONE tax year's coverage, age,
        //   enrolment or accounts, so none is durable.
        durability: Durability::PerYear,
        neutral: false,
    },

    // ── ★★★ SEAM REVIEW I-3 — THE SPOUSE'S PLAN, which the instructions ask about and no other
    //    field on the return can answer. Appended at the END for the `decl_tristate!` array-index
    //    reason recorded above. ───────────────────────────────────────────────
    FormQuestion {
        id: QuestionId::HsaSpouseFamilyCoverage,
        // ★ The INSTRUCTIONS' own two sentences, not a paraphrase of them.
        prompt: "Form 8889 lines 1 and 3: did your SPOUSE have an HDHP with FAMILY coverage? (\"If \
                 you and your spouse are considered covered by a family HDHP, you are considered \
                 covered by a family HDHP regardless of whether you file jointly or separately.\" \
                 And line 3 rule 1: \"Use the family coverage amount if you or your spouse had an \
                 HDHP with family coverage. Disregard any plan with self-only coverage.\")",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaSpouseFamilyCoverage,
        },
        unanswered_detail: "Form 8889 line 1's box and line 3's contribution limit both turn on whether EITHER spouse \
             had family coverage \u{2014} the instructions say so in as many words, and say it counts \
             \"regardless of whether you file jointly or separately\". Answering only for your own \
             plan puts the self-only figure on line 3 where the instructions say the family one, \
             which can make a lawful contribution look like an excess one. Run `btctax income answer`",
        live: hsa_spouse_question_live,
        get: |ri| ri.hsa.spouse_family_coverage,
        set: |ri, v| ri.hsa.spouse_family_coverage = Some(v),
        durability: Durability::PerYear,
        neutral: false,
    },
    // ── ★★★ SEAM REVIEW M-1 — R3's DOCUMENT-LESS DOOR, for Form 8889 line 14a. ────────────
    FormQuestion {
        id: QuestionId::HsaDistributionWithout1099sa,
        prompt: "Form 8889 line 14a: did you take any money out of an HSA this year that no Form \
                 1099-SA reports? (Line 14a is \"Total distributions you received in 2024 from all \
                 HSAs\", and its instruction is \"These amounts should be shown on Form 1099-SA, box \
                 1.\" A trustee must issue one for every distribution, so this is normally No \u{2014} \
                 but a census \"no\" must never close a line the form says to report.)",
        unanswered: RefuseReason::Form8889Unanswered {
            question: QuestionId::HsaDistributionWithout1099sa,
        },
        unanswered_detail: "you answered that you received no Form 1099-SA, and Form 8889 line 14a asks for the TOTAL \
             distributions you received from all HSAs. btctax fills line 14a only from transcribed \
             Form 1099-SA rows, so a \"no\" on the census would leave that line blank \u{2014} and a \
             distribution missing from line 14a is missing gross income under \u{a7}223(f) plus the 20% \
             additional tax. Run `btctax income answer`",
        // ★ Live EXACTLY on R3's pairing: the census row says "I received none" AND the §223
        //   trigger is affirmed. A filer with no HSA is never asked it, and a filer who transcribed
        //   a 1099-SA takes their distributions from the document.
        live: |ri| {
            ri.sch1.hsa_activity == Some(true)
                && ri.documents.get(crate::tax::document_census::DocumentRow::Sa1099) == Some(false)
        },
        get: |ri| ri.hsa_distribution_without_1099sa,
        set: |ri, v| ri.hsa_distribution_without_1099sa = Some(v),
        durability: Durability::PerYear,
        // ★ Neutral at FALSE: no undocumented distribution needs no row and forgoes nothing. A
        //   `Yes` REFUSES — btctax has no surface for a distribution that arrives with no form.
        neutral: false,
    },
    // ── ★★★ T7 / R6 — STEP 5, QUESTION 1 of *Who Qualifies as Your Dependent*. ────
    FormQuestion {
        id: QuestionId::FilerTinIssuedByDueDate,
        prompt: "Did you, and your spouse if filing a joint return, have either an SSN or ITIN issued \
                 on or before the due date of your return (including extensions)? (Answer \"Yes\" if \
                 you are applying for an ITIN on or before the return due date (including \
                 extensions).)",
        unanswered: RefuseReason::FilerTinUnanswered,
        unanswered_detail:
            "this return claims one or more dependents, so Step 5 of Who Qualifies as Your Dependent \
             (i1040gi--2025.txt:1743-1747) asks whether YOU \u{2014} and your spouse on a joint return \
             \u{2014} had an SSN or ITIN issued on or before the due date of the return, including \
             extensions. A \"No\" forgoes the credit for other dependents; it does not stop you \
             claiming the dependent. Run `btctax income answer`",
        // ★ Live iff the return carries a dependent row: Step 5 is reached only through a dependent,
        //   and a return with none is never asked it (R6's own liveness rule for this question).
        live: |ri| !ri.header.dependents.is_empty(),
        get: |ri| ri.header.filer_tin_issued_by_due_date,
        set: |ri, v| ri.header.filer_tin_issued_by_due_date = Some(v),
        durability: Durability::PerYear,
        // ★ Neutral at TRUE: an SSN or ITIN issued by the due date is the ordinary case, it needs no
        //   adjustment and forgoes nothing. A "No" is what costs the filer the credit.
        neutral: true,
    },
    // ── ★★★ R7 / T8 — HEAD OF HOUSEHOLD. Two tests, both live iff the filer checked the box. ────
    FormQuestion {
        id: QuestionId::HohQualifyingPerson,
        // ★ NO TYPED YEAR and NO TYPED FIGURE. The instruction's own sentences name the year
        //   ("for all of 2025"); this return's year is not necessarily that one, and a prompt that
        //   states the wrong year is a question the filer cannot check against their own facts.
        //   Every span `xtask prompt-check` holds against the extract is one the year does not
        //   cross.
        prompt: "Head of household \u{2014} does Test 1 or Test 2 apply to you? TEST 1: \"You paid over \
                 half the cost of keeping up a home that was the main home\", for the WHOLE tax year, \
                 of \"your parent whom you can claim as a dependent, except under a multiple support \
                 agreement\". \"Your parent didn't have to live with you.\" TEST 2: you paid over half \
                 the cost of keeping up a home in which you lived and in which one of the following \
                 also lived for more than half of the year \u{2014} any person whom you can claim as a dependent \
                 (but not a child you claim under the rule for Children of divorced or separated \
                 parents, a person who is your dependent only because they lived with you all year, \
                 or a person you claimed under a multiple support agreement); your unmarried \
                 qualifying child who isn't your dependent; your married qualifying child who isn't \
                 your dependent only because you can be claimed as a dependent on someone else's \
                 return; or your qualifying child who, even though you are the custodial parent, \
                 isn't your dependent because of the rule for Children of divorced or separated \
                 parents.",
        unanswered: RefuseReason::HohTestUnanswered {
            question: QuestionId::HohQualifyingPerson,
        },
        unanswered_detail:
            "you are filing as HEAD OF HOUSEHOLD, and the Form 1040 instructions say to \"Check the \
             'Head of household' box only if you are unmarried (or considered unmarried) and either \
             Test 1 or Test 2 applies\" (i1040gi--2025.txt:1164-1200). Head of household gives a \
             wider bracket and a larger standard deduction than Single, so the box is an assertion \
             that one of those tests is met. Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::HoH,
        get: |ri| ri.header.hoh_qualifying_person,
        set: |ri, v| ri.header.hoh_qualifying_person = Some(v),
        durability: Durability::PerYear,
        // ★ Neutral at TRUE: a filer who checked the HoH box is asserting the test IS met, and a
        //   `No` is what refuses. There is no "nothing to see here" answer that leaves the status
        //   standing.
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::HohPaidOverHalfCostOfKeepingUpHome,
        // ★★★ **The OPERATIVE clause is the instruction's own sentence, quoted — T8 seam review M-2.**
        //     `xtask prompt-check` holds the QUOTED spans of a prompt against the extract; it does
        //     not hold a lead-in that merely paraphrases them. This prompt used to ask *"did you pay
        //     over half the cost of keeping up a home"* beside a quote of the real test, so an edit
        //     to the lead-in alone — *"over half"* to *"most"*, which is a different test entirely
        //     (with three contributors, 40% can be the most) — changed the question the filer answers
        //     with every instrument green. Measured. Now the sentence the filer reads IS the checked
        //     span, so there is no second, unchecked statement of the test to drift.
        prompt: "Head of household \u{2014} for this tax year, is this true: \"You paid over half the \
                 cost of keeping up a home\"? (Both Test 1 and Test 2 begin with it. See Cost of \
                 keeping up a home in the Form 1040 instructions, and Pub. 501, for what counts.)",
        unanswered: RefuseReason::HohTestUnanswered {
            question: QuestionId::HohPaidOverHalfCostOfKeepingUpHome,
        },
        unanswered_detail:
            "you are filing as HEAD OF HOUSEHOLD, and both of the instructions' tests begin with \
             \"You paid over half the cost of keeping up a home\" (i1040gi--2025.txt:1164, :1172). \
             It is asked on its own because the two tests share it. Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::HoH,
        get: |ri| ri.header.hoh_paid_over_half_cost_of_keeping_up_home,
        set: |ri, v| ri.header.hoh_paid_over_half_cost_of_keeping_up_home = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    // ── ★★★ FR-67 — the \u{a7}6013(g)/(h) NONRESIDENT-ALIEN-SPOUSE ELECTION. ─────────────────────────
    FormQuestion {
        id: QuestionId::NraSpouseResidentElection,
        prompt: "Was your spouse a nonresident alien or a dual-status alien at any time during the \
                 tax year? (\"Generally, a married couple can't file a joint return if either spouse \
                 is a nonresident alien at any time during the year. However, you and your spouse \
                 can choose to be treated as U.S. residents for the entire year and file a joint \
                 return\" \u{2014} the \u{a7}6013(g) and \u{a7}6013(h) elections. Answer YES if that describes \
                 either of you, whether or not you make the choice.)",
        unanswered: RefuseReason::NraSpouseElectionUnanswered,
        unanswered_detail:
            "this return carries a spouse, and the Form 1040 instructions' Nonresident aliens and \
             dual-status aliens rule (i1040gi--2025.txt:1059-1072) decides whether a joint return \
             may be filed at all. btctax asks nothing else about an alien spouse, so a return \
             computed without this answer would be a joint return on the U.S. spouse's income alone. \
             Run `btctax income answer`",
        // ★ Live iff the return carries a spouse. The rule is about a MARRIED COUPLE, and a return
        //   with no spouse `Person` has nobody it could be about.
        live: |ri| ri.header.spouse.is_some(),
        get: |ri| ri.header.nra_spouse_resident_election,
        set: |ri, v| ri.header.nra_spouse_resident_election = Some(v),
        durability: Durability::PerYear,
        // ★ Neutral at FALSE: no alien spouse needs no election and forgoes nothing. A `Yes`
        //   REFUSES \u{2014} the election puts the alien spouse's WORLDWIDE income on the return and
        //   btctax collects none of it.
        neutral: false,
    },
    // ── ★★★ R7 / T8 — QUALIFYING SURVIVING SPOUSE. Five conditions, all live iff the box. ───────
    FormQuestion {
        id: QuestionId::QssSpouseDiedInWindowAndNotRemarried,
        // ★ RENDERED: the two-year window is derived from `tax_year` (see `RENDERED_PROMPTS`). This
        //   static text is the fallback and names no years, so it can never state the wrong ones.
        prompt: "Qualifying surviving spouse, condition 1: did your spouse die in one of the two \
                 years before this tax year, and you didn't remarry before the end of this tax year? \
                 (If your spouse died during this tax year you can't file as qualifying surviving \
                 spouse; see the instructions for Married Filing Jointly.)",
        unanswered: RefuseReason::QssTestUnanswered {
            question: QuestionId::QssSpouseDiedInWindowAndNotRemarried,
        },
        unanswered_detail:
            "you are filing as QUALIFYING SURVIVING SPOUSE, which uses the JOINT return tax rates \
             and the joint standard deduction. The Form 1040 instructions allow the box only \"if \
             all of the following apply\" (i1040gi--2025.txt:1288-1316), and this is condition 1. \
             Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Qss,
        get: |ri| ri.header.qss_spouse_died_in_window_and_not_remarried,
        set: |ri, v| ri.header.qss_spouse_died_in_window_and_not_remarried = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::QssChildYouCanClaim,
        // ★ The instruction's sentence names a YEAR and a FIGURE (the §152(d)(1)(B) limit). Neither
        //   is typed here: the year is not necessarily this return's, and the figure lives in
        //   `FullReturnParams::qualifying_relative_gross_income_limit` — a table of years typed into
        //   a prompt is a second copy of derived data that nothing checks.
        prompt: "Qualifying surviving spouse, condition 2: \"You have a child or stepchild (not a \
                 foster child) whom you can claim as a dependent\" \u{2014} or could claim as a \
                 dependent except that, for the tax year: (a) the child's gross income reached the \
                 \u{a7}152(d)(1)(B) limit for the year, (b) \"The child filed a joint return\", or (c) you \
                 could be claimed as a dependent on someone else's return.",
        unanswered: RefuseReason::QssTestUnanswered {
            question: QuestionId::QssChildYouCanClaim,
        },
        unanswered_detail:
            "you are filing as QUALIFYING SURVIVING SPOUSE; this is condition 2 of the five the \
             Form 1040 instructions require (i1040gi--2025.txt:1298-1306). Note that it is a child \
             or STEPCHILD, and that a foster child does not count. Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Qss,
        get: |ri| ri.header.qss_child_you_can_claim,
        set: |ri, v| ri.header.qss_child_you_can_claim = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::QssChildLivedInYourHomeAllYear,
        prompt: "Qualifying surviving spouse, condition 3: \"This child lived in your home for all of\" \
                 the tax year. \"If the child didn't live with you for the required time, see \
                 Exception to time lived with you, later.\" (The exception counts temporary absences \
                 by you or the child for special circumstances \u{2014} school, vacation, business, \
                 medical care, military service \u{2014} and a child who was born or died during the \
                 year.)",
        unanswered: RefuseReason::QssTestUnanswered {
            question: QuestionId::QssChildLivedInYourHomeAllYear,
        },
        unanswered_detail:
            "you are filing as QUALIFYING SURVIVING SPOUSE; this is condition 3 of the five \
             (i1040gi--2025.txt:1273-1277). Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Qss,
        get: |ri| ri.header.qss_child_lived_in_your_home_all_year,
        set: |ri, v| ri.header.qss_child_lived_in_your_home_all_year = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::QssPaidOverHalfCostOfKeepingUpHome,
        prompt: "Qualifying surviving spouse, condition 4: \"You paid over half the cost of keeping \
                 up your home.\" (See Cost of keeping up a home in the Form 1040 instructions, and \
                 Pub. 501, for what counts.)",
        unanswered: RefuseReason::QssTestUnanswered {
            question: QuestionId::QssPaidOverHalfCostOfKeepingUpHome,
        },
        unanswered_detail:
            "you are filing as QUALIFYING SURVIVING SPOUSE; this is condition 4 of the five \
             (i1040gi--2025.txt:1278-1279). Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Qss,
        get: |ri| ri.header.qss_paid_over_half_cost_of_keeping_up_home,
        set: |ri, v| ri.header.qss_paid_over_half_cost_of_keeping_up_home = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::QssCouldHaveFiledJointlyInYearOfDeath,
        prompt: "Qualifying surviving spouse, condition 5: \"You could have filed a joint return with \
                 your spouse the year your spouse died, even if you didn't actually do so.\"",
        unanswered: RefuseReason::QssTestUnanswered {
            question: QuestionId::QssCouldHaveFiledJointlyInYearOfDeath,
        },
        unanswered_detail:
            "you are filing as QUALIFYING SURVIVING SPOUSE; this is condition 5 of the five \
             (i1040gi--2025.txt:1280-1283). Run `btctax income answer`",
        live: |ri| ri.filing_status == FilingStatus::Qss,
        get: |ri| ri.header.qss_could_have_filed_jointly_in_year_of_death,
        set: |ri, v| ri.header.qss_could_have_filed_jointly_in_year_of_death = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    // ── ★★★ R8 / T9 — REAL ESTATE. Indices 61..=65, APPENDED AT THE END for the `decl_tristate!`
    //    array-index reason recorded on `QuestionId::MortgageWithinDebtLimit`.
    FormQuestion {
        id: QuestionId::ClaimingMortgageInterestCredit,
        prompt: "Schedule A line 8a \u{2014} are you claiming the MORTGAGE INTEREST CREDIT? (Form 8396, \
                 for holders of a qualified mortgage credit certificate issued by a state or local \
                 governmental unit or agency. Answer yes only if you hold such a certificate; an \
                 ordinary mortgage is not one.)",
        unanswered: RefuseReason::MortgageInterestCreditUnanswered,
        unanswered_detail:
            "this return reports home mortgage interest, and Schedule A's Line 8a Caution says \"If \
             you are claiming the mortgage interest credit \u{2026} subtract the amount shown on Form \
             8396, line 3, from the total deductible interest you paid on your home mortgage. Enter \
             the result on line 8a\" (i1040sca--2025.txt:1091-1096). btctax builds no Form 8396 and \
             holds no line 3, so if you hold a certificate it cannot make that subtraction and line \
             8a would OVERSTATE your deduction \u{2014} run `btctax income answer`",
        live: mortgage_interest_credit_question_live,
        get: |ri| ri.claiming_mortgage_interest_credit,
        set: |ri, v| ri.claiming_mortgage_interest_credit = Some(v),
        durability: Durability::PerYear,
        // ★ NEUTRAL AT `false`: "no, I hold no mortgage credit certificate" is the answer that needs
        //   no adjustment — line 8a stays the full Form 1098 figure. A `true` REFUSES.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::SoldMainHome,
        prompt: "Did you SELL OR EXCHANGE YOUR MAIN HOME during the year? (Schedule D, \"Sale of Your \
                 Home\" \u{2014} answer for the home you lived in, not a rental, a piece of vacant \
                 land or a second home.)",
        unanswered: RefuseReason::HomeSaleGateUnanswered {
            question: QuestionId::SoldMainHome,
        },
        unanswered_detail:
            "every return must state whether a main home was sold: the Schedule D instructions' own \
             answer is \"You may not need to report the sale or exchange of your main home\" \
             (i1040sd--2025.txt:315-316), and which of its branches you are on decides whether a \
             Form 8949 belongs on this return at all. A blank here is not \"no\" \u{2014} run \
             `btctax income answer`",
        live: |_ri| true,
        get: |ri| ri.home_sale.sold_main_home,
        set: |ri, v| ri.home_sale.sold_main_home = Some(v),
        durability: Durability::PerYear,
        // ★★ NOT neutral. "I sold no home this year" is an affirmative statement about the filer's
        //    year; a default would answer a Schedule D question on their behalf.
        neutral: false,
    },
    FormQuestion {
        id: QuestionId::HomeSaleTest1OwnedAndLived,
        prompt: "Sale of your home, Test 1: \"During the 5-year period ending on the date you sold or \
                 exchanged your home, you owned it for 2 years or more (the ownership requirement) \
                 and lived in it as your main home for 2 years or more (the use requirement).\" Is \
                 that true?",
        unanswered: RefuseReason::HomeSaleGateUnanswered {
            question: QuestionId::HomeSaleTest1OwnedAndLived,
        },
        unanswered_detail:
            "you sold your main home, so Schedule D's Test 1 must be answered \
             (i1040sd--2025.txt:335-343). Run `btctax income answer`",
        live: home_sale_test_live,
        get: |ri| {
            ri.home_sale
                .test1_owned_2_years_and_lived_2_years_of_last_5
        },
        set: |ri, v| {
            ri.home_sale
                .test1_owned_2_years_and_lived_2_years_of_last_5 = Some(v);
        },
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::HomeSaleTest2NoRecentExclusion,
        prompt: "Sale of your home, Test 2: \"You haven't excluded gain on the sale or exchange of \
                 another main home during the 2-year period ending on the date of the sale or \
                 exchange of your home.\" Is that true?",
        unanswered: RefuseReason::HomeSaleGateUnanswered {
            question: QuestionId::HomeSaleTest2NoRecentExclusion,
        },
        unanswered_detail:
            "you sold your main home, so Schedule D's Test 2 must be answered \
             (i1040sd--2025.txt:344-348). Run `btctax income answer`",
        live: home_sale_test_live,
        get: |ri| ri.home_sale.test2_no_exclusion_on_another_home_in_2_years,
        set: |ri, v| ri.home_sale.test2_no_exclusion_on_another_home_in_2_years = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
    FormQuestion {
        id: QuestionId::HomeSaleCanExcludeAllGain,
        prompt: "Sale of your home: can you EXCLUDE ALL of your gain from income? (Schedule D reports \
                 the sale on Form 8949 if \"You can't exclude all of your gain from income\". The \
                 exclusion is up to $250,000 of gain, or $500,000 on a joint return where both of \
                 you meet the tests \u{2014} Pub. 523 has the worksheet. Answer NO if you are unsure.)",
        unanswered: RefuseReason::HomeSaleGateUnanswered {
            question: QuestionId::HomeSaleCanExcludeAllGain,
        },
        unanswered_detail:
            "you sold your main home, so Schedule D's own reporting rule must be answered: \"Report \
             the sale or exchange of your main home on Form 8949 if: \u{2022} You can't exclude all of \
             your gain from income\" (i1040sd--2025.txt:318-322). Run `btctax income answer`",
        live: home_sale_test_live,
        get: |ri| ri.home_sale.can_exclude_all_gain,
        set: |ri, v| ri.home_sale.can_exclude_all_gain = Some(v),
        durability: Durability::PerYear,
        neutral: true,
    },
];

/// ★★★ **T9 / R8 — the Form 8396 gate's liveness.**
///
/// Schedule A's Caution sits under **Line 8a** and modifies it, so the question is live exactly
/// where there is mortgage interest for it to modify: a Form 1098 row, or a line 8b row (interest
/// paid to a recipient who issued none). A filer with neither has no line 8a and no line 8b, so the
/// Caution addresses nobody.
///
/// ★ It does NOT read `schedule_a.is_some()` on its own: the 1098 SECTION is already gated on the
///   itemize election, so a row can only exist on an itemizing return, and an 8b row lives on
///   `ScheduleAInputs` by construction.
fn mortgage_interest_credit_question_live(ri: &ReturnInputs) -> bool {
    !ri.form_1098.is_empty()
        || ri
            .schedule_a
            .as_ref()
            .is_some_and(|a| !a.mortgage_interest_not_on_1098.is_empty())
}

/// ★★★ **T9 / R8 — the three home-sale tests are live iff a main home was sold.**
///
/// The Schedule D block opens *"You may not need to report the sale or exchange of your main
/// home"* — every test under it is a question about **that sale**, so asking them of a filer who
/// sold nothing would be asking about an event that did not happen.
fn home_sale_test_live(ri: &ReturnInputs) -> bool {
    ri.home_sale.sold_main_home == Some(true)
}

/// ★★★ **T16 — the ONE liveness predicate for every Form 8889 question.**
///
/// The §223 trigger declaration is what opens the form: `Some(true)` means a contribution, a
/// distribution, an inheritance or a testing-period inclusion happened, and Form 8889 is then
/// mandatory. `Some(false)` and `None` both leave the seven silent — `None` because `HsaActivity`
/// itself is the live question refusing first, which is the tiering `screen_inputs` already has.
///
/// ★ NOT scoped to "there is a Form 1099-SA row": a contribution with no distribution files a Form
/// 8889 with an empty Part II, so scoping the questions to a document would be the circular liveness
/// §2.9 records — asking only where btctax already knew the answer.
fn hsa_question_live(ri: &ReturnInputs) -> bool {
    ri.sch1.hsa_activity == Some(true)
}

/// ★★★ **Seam review I-3 — Form 8889's one question about the OTHER person's plan.**
///
/// Live iff the form is open AND there is a spouse to have a plan. **MFS counts**, and that is the
/// instruction's own word rather than an inference: *"If you and your spouse are considered covered
/// by a family HDHP, you are considered covered by a family HDHP **regardless of whether you file
/// jointly or separately**"* (`i8889--2024.txt:466-470`). Scoping it to MFJ would ask a married
/// filer the question on one return and not on the other, and get line 3's limit wrong on the
/// second.
///
/// ★ A single filer has no spouse's plan to declare, so the question is not live and the leaf stays
///   `None` — the "absent" state, which is distinct from "unanswered".
fn hsa_spouse_question_live(ri: &ReturnInputs) -> bool {
    hsa_question_live(ri)
        && matches!(
            ri.filing_status,
            crate::tax::types::FilingStatus::Mfj | crate::tax::types::FilingStatus::Mfs
        )
}

/// The identity of each SKIPPABLE prompt (§2, class B) — the questions where silence is LAWFUL: a bare
/// Enter leaves the value `None`, forgoing a benefit whose burden to CLAIM is the filer's (New Colonial
/// Ice), and the matching advisory then fires (never in silence — the owner mandate).
///
/// ★ A SEPARATE identity space from [`QuestionId`] (spec §5.3 HARD RULE). A skippable is `None`-legal; a
/// [`FormQuestion`] declaration is not. Merging the two registries would brick `screen_inputs` — it would
/// refuse a lawfully-unanswered skippable — so the two lists must never be one.
/// ★ `Ord` is derived because [`crate::tax::provenance::AnswerKey`] is a `BTreeMap` key; the ordering
/// is the declaration order and carries no meaning beyond "some stable total order".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkippableId {
    /// ★ §63(f) BLINDNESS (taxpayer). Always live; `None` forgoes the addition and fires the advisory.
    BlindTaxpayer,
    /// ★ §63(f) BLINDNESS (spouse) — live only with a spouse `Person` (a `set_bool` on an absent spouse is
    /// silently discarded, so the prompt is gated to match).
    BlindSpouse,
    /// ★ §164(b)(5) sales-tax election — live only with a `schedule_a` (nowhere to write it otherwise).
    SalesTaxElection,
    /// §63(f) aged addition (taxpayer). A mandatory DOB prompt would force the filer to INVENT a birthday,
    /// and an invented-old one understates tax — so `None` must stay reachable.
    DobTaxpayer,
    /// §63(f) aged addition (spouse) — live only with a spouse `Person` (its `set_date` twin gate).
    DobSpouse,
    /// §G-9: the DATE OF DEATH (taxpayer). Class (B) — skipping it leaves `is_aged` unable to show the
    /// taxpayer reached 65, so the addition is FORGONE, never granted. Live only once
    /// [`Self::TaxpayerDiedDuringYear`] is answered `Some(true)`: nobody else has a date to give.
    DodTaxpayer,
    /// §G-9: the DATE OF DEATH (spouse). Live only once [`Self::SpouseDiedDuringYear`] is `Some(true)`.
    DodSpouse,
    /// ★★ **1040 line 12a / §63(f) — the death carve-out (`FOLLOWUPS.md` §G-9), taxpayer.** i1040gi:
    /// *"Death of a taxpayer … If a taxpayer was born before January 2, 1961, but died in 2025 before
    /// reaching age 65, then the taxpayer doesn't qualify."*
    ///
    /// ★★★ **It was a class-(A) declaration that REFUSED, and it was `live: |_| true` — so it blocked
    /// EVERY return btctax could compute.** That is the single biggest usability cost in the registry,
    /// and it bought nothing: [`crate::tax::return_1040::is_aged`]'s `(None, None)` arm already returns
    /// `false`, so silence FORGOES the addition. The refusal was redundant with a fail-safe sitting
    /// directly beneath it. Class (B) by the sharp test — *does the silence ASSERT, or FORGO?* It
    /// forgoes, and [`crate::tax::advisories::Advisory::AgedBoxForfeitedDeathUnanswered`] says so, but
    /// only when a qualifying date of birth is on file and the skip therefore actually costs money.
    TaxpayerDiedDuringYear,
    /// **1040 line 12a / §63(f) — the death carve-out, spouse.** i1040gi: *"If your spouse was born
    /// before January 2, 1960, but died in 2024 before reaching age 65, don't check the box that says
    /// 'Spouse was born before January 2, 1960.'"* Same class-(B) reasoning as
    /// [`Self::TaxpayerDiedDuringYear`].
    ///
    /// ★ **Live exactly when the spouse's §63(f) boxes can count** — `spouse_63f_boxes_count`, not
    /// merely "a spouse record exists". On MFJ that is any spouse; on MFS it is a spouse meeting all
    /// three i1040gi conditions. Anywhere else the answer could not move a figure, and it was
    /// previously asked (and, before that, REFUSED) on returns where it was inert.
    SpouseDiedDuringYear,
    /// ★★ **Schedule B line 7a's unnumbered FBAR sub-question** — *"If 'Yes,' are you required to file
    /// FinCEN Form 114 … ?"* Live only when 7a is answered **Yes**: the form itself conditions it on 7a
    /// ("If 'Yes,'"), so a filer with no foreign account is never asked.
    ///
    /// ★★★ **It was briefly a class-(A) refusal, and that was WRONG.** Refusal is justified only when
    /// proceeding without the answer would produce a wrong number, put fabricated testimony on a signed
    /// return, or silently expose the filer to a penalty or a lost right. This box fails all three:
    /// **no figure on the return reads it** (grep `fbar_filing_required` — the printed chain writes the
    /// checkbox and nothing else), a blank is *no testimony* rather than false testimony, and the
    /// penalty the form's Caution warns of attaches to **not filing FinCEN Form 114** — a FinCEN
    /// obligation that exists whatever this box says — not to leaving the box blank. That exposure is
    /// already put in front of the filer by [`super::advisories::Advisory::FbarFinCen`], which fires on
    /// 7a = Yes alone.
    ///
    /// So silence is lawful and prints a genuine blank; [`super::advisories::Advisory::FbarSubQuestionNotAnswered`]
    /// quotes the form's Caution **verbatim** when it is skipped. btctax takes no position on the
    /// answer: FinCEN Notice 2020-2 leaves accounts holding ONLY virtual currency outside the FBAR
    /// requirement for now, that is under active reconsideration, and an account holding crypto PLUS
    /// fiat or securities may well be reportable.
    FbarFilingRequired,
    /// ★★ **Schedule C line I** — *"Did you make any payments in 2024 that would require you to file
    /// Form(s) 1099?"* A compliance declaration about the filer's own information-reporting.
    ///
    /// Class (B), not a refusal, by the refusal review's criterion applied honestly: no figure on the
    /// return reads it, a blank is no testimony, and — **unlike Schedule B's FBAR sub-question** —
    /// the form prints NO Caution beside it. That difference is what decided it. The §6721/§6722
    /// exposure IS real, so the skip is not silent:
    /// [`super::advisories::Advisory::ScheduleC1099NotAnswered`] names both sections.
    ScheduleC1099Required,
    /// **Schedule C line J** — *"If 'Yes,' did you or will you file required Form(s) 1099?"*
    ///
    /// ★★ Live only when line I is answered **Yes** — the form says *"If 'Yes,'"* — which makes this
    /// the registry's live example of a question whose liveness depends on another question's
    /// **NON-NEUTRAL** answer. That shape silently broke the registry's own property harness once: a
    /// loop answering every other live question at its neutral switched this class of question OFF, so
    /// it could never be exercised. The `is_none()` guards in `return_refuse.rs` were written for it
    /// and, until now, had no live case.
    ScheduleC1099Filed,
    /// ★★★ **Form 8283 Section B lines 5a / 5b / 5c**, asked as ONE return-level universal: *"did any
    /// of your donations have strings attached?"* A **Yes** to any of the three limbs shrinks or kills
    /// the §170 deduction (Reg §1.170A-7), and btctax deducts at full FMV — so a Yes means the number
    /// is WRONG. Asked here so silence never auto-refuses a filer who donated NOTHING; the refusal is scoped by
    /// `screen_absolute`, which has the computed §63(e) ELECTION that liveness cannot see, and is MANDATORY there:
    /// on a Section-B year, unanswered refuses and `Some(true)` refuses — only `Some(false)` proceeds.
    ///
    /// ★★ **This is what dissolved §G-21.** The three boxes are per-donation and the registry is
    /// return-shaped, which looked like it needed new per-row machinery. Asking the UNIVERSAL makes
    /// them return-shaped too: from *"none of my donations had any of these"* each box's answer follows
    /// for every donation. The prompt therefore enumerates all three limbs in the form's own words — a
    /// "No" to something vaguer would be laundered into three answers the filer never gave.
    DonationsHadRestrictions,
    /// **Form 8995-A Part I column (b)** — is the trade or business a specified service trade or
    /// business? (§G-28/B1b.)
    ScheduleCIsSstb,
    /// **Form 8995-A Part I column (e)** — is the filer a patron of an agricultural or horticultural
    /// cooperative? (§G-28/B1b.) Unlike every other question here this one decides **which form is
    /// filed**, at any level of income.
    ScheduleCIsCooperativePatron,
    /// ★★★ **§170(f)(8) — the CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT**, Schedule A lines 11/12's own
    /// *"If you made any gift of $250 or more, see instructions"*. Asked as one return-level
    /// universal, exactly like [`Self::DonationsHadRestrictions`]; the MANDATORY half lives in
    /// `screen_absolute`, which has the ledger AND the computed §63(e) itemize election.
    CharitableCwaObtained,
    /// ★★★ **Form 8615, condition 3** (FR-29) — *"You were either: a. Under age 18 …, b. Age 18 …
    /// and didn't have earned income that was more than half of your support, or c. A full-time
    /// student at least age 19 but under age 24 … and didn't have earned income that was more than
    /// half of your support."* (`design/forms/extract/i1040gi--2025.txt:3932-3940`.)
    ///
    /// Class-(A) SEMANTICS in the class-(B) registry, deliberately (SPEC §3.1): condition 1 needs the
    /// LEDGER, which `live` cannot see, so a class-(A) declaration would either refuse every DOB-less
    /// return or brick a crypto-only filer whose question is never offered. Liveness is broad; the
    /// MANDATORY half is `screen_compute_dependent`, which can see what the input screen cannot.
    Form8615Condition3AgeSupport,
    /// ★★★ **Form 8615, condition 4** — *"At least one of your parents was alive at the end of
    /// 2025."* (`i1040gi--2025.txt:3941-3942`.) The registry's ONLY
    /// [`SkippableKind::Choice`]: *"I cannot know"* is a THIRD ANSWER, distinct from unanswered.
    Form8615Condition4ParentAlive,
    /// ★★★ **The SPEC §6.3 dead-end FACT** — *"Can you give the IRS your parent's name and address?"*
    /// Live ONLY once condition 4 is answered `CannotKnow`, so the certification is unreachable until
    /// the filer has testified to the dead end. A general opt-out would rebuild FR-29 behind a nicer
    /// interface.
    ///
    /// ★★ **This is the registry's only POLARITY-INVERTING entry**: the prompt asks what the filer
    /// CAN do and the leaf records what they cannot, so `get_bool`/`set_bool` carry a `!`.
    Form8615ParentIdentityUnobtainable,
    /// ★★★ **R7 / T8 — the HEAD-OF-HOUSEHOLD marital basis**, the instruction's four named states
    /// (`i1040gi--2025.txt:1143-1163`). In THIS registry because its answer shape is a
    /// [`SkippableKind::Choice`], and class **(A)** all the same — see
    /// [`SkippableQuestion::unanswered`], which is what says so.
    HohMaritalBasis,
}

impl SkippableId {
    /// Every skippable identity, in registry order. The anchor a caller iterates when it needs the
    /// SET of skippables rather than their prompts — notably
    /// [`crate::tax::provenance::AnswerKey`]'s wire parser, which must resolve a stored key back to
    /// its id without going through [`SKIPPABLE_QUESTIONS`] (a key that outlived its registry entry
    /// must still parse, so the refuse-and-reimport gate can report it).
    ///
    /// ★ Mirrors [`QuestionId::ALL`], and is pinned the same way: the completeness test's `match` is
    /// exhaustive, so a NEW variant is a compile error until it is listed here.
    pub const ALL: &'static [SkippableId] = &[
        SkippableId::BlindTaxpayer,
        SkippableId::BlindSpouse,
        SkippableId::SalesTaxElection,
        SkippableId::DobTaxpayer,
        SkippableId::DobSpouse,
        SkippableId::DodTaxpayer,
        SkippableId::DodSpouse,
        SkippableId::TaxpayerDiedDuringYear,
        SkippableId::SpouseDiedDuringYear,
        SkippableId::FbarFilingRequired,
        SkippableId::ScheduleC1099Required,
        SkippableId::ScheduleC1099Filed,
        SkippableId::DonationsHadRestrictions,
        SkippableId::ScheduleCIsSstb,
        SkippableId::ScheduleCIsCooperativePatron,
        SkippableId::CharitableCwaObtained,
        SkippableId::Form8615Condition3AgeSupport,
        SkippableId::Form8615Condition4ParentAlive,
        SkippableId::Form8615ParentIdentityUnobtainable,
        // ★★★ R7 / T8 — index 19.
        SkippableId::HohMaritalBasis,
    ];
}

/// The value shape of a [`SkippableQuestion`] — a yes/no answer, a calendar date, or a fixed set of
/// named answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkippableKind {
    YesNo,
    Date,
    /// ★ A fixed set of named answers, for a question whose third answer is not "unanswered". The
    /// only member today is Form 8615's condition 4
    /// ([`crate::tax::return_inputs::ParentAliveAnswer`]).
    Choice(&'static [&'static str]),
}

/// A SKIPPABLE prompt (§2.2, class B). The same fn-pointer shape as [`FormQuestion`], but silence is a
/// LAWFUL outcome here: a bare Enter leaves the value `None`. The accessors split by [`kind`](Self::kind):
/// the pair that does not apply to this question's `kind` returns `None` / is a no-op (the catch-all lifted
/// from the old `answer.rs::Skippable`). Each `set` is also a no-op when its target row is absent — which
/// is exactly why `live` gates the spouse/Schedule-A prompts, so the prompt scope tracks the WRITE scope.
pub struct SkippableQuestion {
    /// ★ §G-15 — whether this answer survives into the next tax year. See [`Durability`].
    ///
    /// ★★ This is where `Durable` actually occurs: a **date of birth cannot change**. Everything
    /// else here can — blindness, a §164(b)(5) election, and a date of DEATH (gated on
    /// `…DiedDuringYear`, itself a per-year declaration).
    pub durability: Durability,
    pub id: SkippableId,
    /// The prompt, phrased as the FORM phrases it (the words the filer can check against their paperwork).
    pub prompt: &'static str,
    /// What skipping forgoes — the advisory framing, for a UI that shows help beside the prompt.
    pub help: &'static str,
    /// Whether this prompt reads a yes/no or a date — the answer loop branches on it.
    pub kind: SkippableKind,
    /// ★ THE liveness predicate — the ONLY copy, lifted from the old `answer.rs::live_questions` gates.
    pub live: fn(&ReturnInputs) -> bool,
    /// The yes/no on file (`None` for the `Date` kinds).
    pub get_bool: fn(&ReturnInputs) -> Option<bool>,
    /// Record a yes/no (a no-op for the `Date` kinds, or when the target row is absent).
    pub set_bool: fn(&mut ReturnInputs, bool),
    /// The date on file (`None` for the `YesNo` kinds).
    pub get_date: fn(&ReturnInputs) -> Option<Date>,
    /// Record a date (a no-op for the `YesNo` kinds, or when the target row is absent).
    pub set_date: fn(&mut ReturnInputs, Date),
    /// The named answer on file (`None` for every kind but [`SkippableKind::Choice`], and `None` on a
    /// `Choice` question the filer has not answered).
    pub get_choice: fn(&ReturnInputs) -> Option<&'static str>,
    /// Record a named answer (a no-op for every kind but [`SkippableKind::Choice`], and a no-op on a
    /// string outside that kind's option list — the caller parses, this only stores).
    pub set_choice: fn(&mut ReturnInputs, &'static str),
    /// ★★★ **THE CLASS, DECLARED — because this registry is no longer uniformly class (B).**
    ///
    /// `None` is the registry's rule and every entry's answer until R7: silence is LAWFUL, the panel
    /// lists the entry as *forgoing*, and nothing refuses. `Some(reason)` says silence is **not**
    /// lawful here — [`crate::tax::return_refuse::screen_inputs`] refuses with that reason and the
    /// R12 panel lists the entry as **blocking**.
    ///
    /// ★★ **Why a class-(A) entry lives in this registry at all.** The split between
    /// [`FORM_QUESTIONS`] and this one is not class — it is the ANSWER SHAPE. `FormQuestion` reads
    /// and writes an `Option<bool>`; this one carries the yes/no, date and **named-choice**
    /// accessors. R7's HoH marital basis is one of the instruction's four named states
    /// ([`crate::tax::return_inputs::HohMaritalBasis`]), so it needs the `Choice` shape — and its
    /// silence is not lawful, because a filer who checked the Head-of-household box has already
    /// asserted they are unmarried or considered unmarried, and *which of the four ways* is the
    /// testimony behind that box.
    ///
    /// ★ DECLARED rather than inferred, for the same reason [`FormQuestion::neutral`] is: a
    ///   hand-written list of "the class-(A) ones" beside the registry is a second copy of derived
    ///   data, and both readers (the screen and the panel) would have to keep it in step.
    pub unanswered: Option<crate::tax::return_refuse::RefuseReason>,
    /// The FULL refusal detail, for a class-(A) entry. `""` where [`Self::unanswered`] is `None`.
    pub unanswered_detail: &'static str,
}

/// ★ THE SKIPPABLE REGISTRY. Thirteen prompts — SEPARATE from [`FORM_QUESTIONS`] (spec §5.3). The
/// liveness gates and prompts are lifted verbatim from the old `answer.rs::Skippable`; the `income answer`
/// flow and the form engine both DERIVE their skippable prompts from this one list.
///
/// ★★ **Two species, and the second is why class (B) is not simply "forgone benefit".**
///
/// - **Benefit claims** (*New Colonial Ice*: the burden to claim is the filer's, so forgoing is
///   lawful) — blindness ×2, the SALT election, the DOBs, the dates of death, and the §G-9 death
///   gates, whose silence forgoes the §63(f) age-65 addition.
/// - **Compliance boxes NO FIGURE READS** — [`SkippableId::FbarFilingRequired`] and the Schedule C
///   pair [`SkippableId::ScheduleC1099Required`] / [`SkippableId::ScheduleC1099Filed`]. Their silence
///   neither asserts nor forgoes, because nothing on the return depends on them. They are class (B)
///   for a different reason from everything else here, and each has an advisory naming the exposure
///   the blank does NOT remove (FinCEN 114; §6721/§6722).
///
/// **Class (B) is the set of questions whose silence is LAWFUL, not only the set that costs money.**
pub const SKIPPABLE_QUESTIONS: &[SkippableQuestion] = &[
    SkippableQuestion {
        id: SkippableId::BlindTaxpayer,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "Are YOU legally blind? (§63(f) additional deduction)",
        help: "§63(f): legal blindness adds an extra standard-deduction amount. Skipping leaves it \
               unclaimed — lawful, since the burden to claim is yours — and the forgone-benefit advisory fires.",
        kind: SkippableKind::YesNo,
        live: |_ri| true,
        get_bool: |ri| ri.header.taxpayer.blind,
        set_bool: |ri, v| ri.header.taxpayer.blind = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::BlindSpouse,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "Is YOUR SPOUSE legally blind? (§63(f) additional deduction)",
        help: "§63(f): the spouse's legal blindness adds an extra standard-deduction amount. Skipping \
               leaves it unclaimed and the forgone-benefit advisory fires.",
        kind: SkippableKind::YesNo,
        live: |ri| ri.header.spouse.is_some(),
        get_bool: |ri| ri.header.spouse.as_ref().and_then(|s| s.blind),
        set_bool: |ri, v| {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.blind = Some(v);
            }
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::SalesTaxElection,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "Deduct general SALES taxes instead of state/local income taxes? (§164(b)(5))",
        help: "§164(b)(5): elect to deduct general sales taxes instead of state and local income taxes. \
               Skipping keeps income taxes on the return; the election is advised when a Schedule A exists.",
        kind: SkippableKind::YesNo,
        live: |ri| ri.schedule_a.is_some(),
        get_bool: |ri| ri.schedule_a.as_ref().and_then(|a| a.salt_use_sales_tax),
        set_bool: |ri, v| {
            if let Some(a) = ri.schedule_a.as_mut() {
                a.salt_use_sales_tax = Some(v);
            }
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::DobTaxpayer,
        unanswered: None,
        unanswered_detail: "",
        // ★★ §G-15 — DURABLE: a date of birth cannot change. The prior MAY be shown, but it still
        // takes the same explicit keystroke as a fresh ask — a forced retype invites a typo, and
        // for a DOB a typo is the worse failure.
        durability: Durability::Durable,
        prompt: "YOUR date of birth",
        help: "§63(f): your date of birth establishes the age-65 additional standard deduction. Skipping \
               leaves it unclaimed — a mandatory prompt would force you to invent a birthday, so silence stays reachable.",
        kind: SkippableKind::Date,
        live: |_ri| true,
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |ri| ri.header.taxpayer.date_of_birth,
        set_date: |ri, v| ri.header.taxpayer.date_of_birth = Some(v),
    },
    SkippableQuestion {
        id: SkippableId::DobSpouse,
        unanswered: None,
        unanswered_detail: "",
        // ★★ §G-15 — DURABLE: a date of birth cannot change. The prior MAY be shown, but it still
        // takes the same explicit keystroke as a fresh ask — a forced retype invites a typo, and
        // for a DOB a typo is the worse failure.
        durability: Durability::Durable,
        prompt: "YOUR SPOUSE's date of birth",
        help: "§63(f): the spouse's date of birth establishes the age-65 additional standard deduction. \
               Skipping leaves it unclaimed.",
        kind: SkippableKind::Date,
        live: |ri| ri.header.spouse.is_some(),
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |ri| ri.header.spouse.as_ref().and_then(|s| s.date_of_birth),
        set_date: |ri, v| {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.date_of_birth = Some(v);
            }
        },
    },
    SkippableQuestion {
        id: SkippableId::DodTaxpayer,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "YOUR date of death",
        help: "§63(f) / §G-9: a taxpayer who died during the year before reaching age 65 does not get \
               the age-65 addition. The date decides it — a person reaches 65 on the DAY BEFORE their \
               65th birthday. Skipping leaves the addition unclaimed (it is never granted on an \
               unknown date).",
        kind: SkippableKind::Date,
        live: |ri| ri.header.taxpayer_died_during_year == Some(true),
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |ri| ri.header.taxpayer.date_of_death,
        set_date: |ri, v| ri.header.taxpayer.date_of_death = Some(v),
    },
    SkippableQuestion {
        id: SkippableId::DodSpouse,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: this subject can differ between years, so re-ask blank.
        durability: Durability::PerYear,
        prompt: "YOUR SPOUSE's date of death",
        help: "§63(f) / §G-9: a spouse who died during the year before reaching age 65 does not get the \
               age-65 addition. A person reaches 65 on the DAY BEFORE their 65th birthday. Skipping \
               leaves the addition unclaimed.",
        kind: SkippableKind::Date,
        // Three conditions: MFJ (the only status whose spouse box `AgedBlindBoxes::for_return` counts),
        // a spouse `Person` to write the date onto, AND the gate saying they died. The MFJ term matches
        // `SkippableId::SpouseDiedDuringYear`'s own liveness — a date whose gate is never asked would be
        // unreachable, and asking for a date that cannot move a figure is the waste this pass removed.
        live: |ri| spouse_63f_boxes_count(ri) && ri.header.spouse_died_during_year == Some(true),
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |ri| ri.header.spouse.as_ref().and_then(|s| s.date_of_death),
        set_date: |ri, v| {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.date_of_death = Some(v);
            }
        },
    },
    SkippableQuestion {
        id: SkippableId::FbarFilingRequired,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: whether an FBAR is required turns on the year's account balances.
        durability: Durability::PerYear,
        prompt: "Schedule B line 7a (sub-question): you said you had a foreign financial account \u{2014} \
                 are you REQUIRED to file FinCEN Form 114 (the FBAR) to report that financial interest or \
                 signature authority?",
        help: "Schedule B's own Caution: \"If required, failure to file FinCEN Form 114 may result in \
               substantial penalties. Additionally, you may be required to file Form 8938, Statement of \
               Specified Foreign Financial Assets.\" That penalty attaches to NOT FILING FinCEN Form 114 \
               \u{2014} an obligation independent of this box \u{2014} not to leaving the box blank, so \
               skipping is lawful and prints a true blank. btctax takes no position on the answer: FinCEN \
               Notice 2020-2 leaves crypto-only accounts outside the requirement for now, that is under \
               active reconsideration, and an account holding crypto PLUS fiat or securities may well be \
               reportable.",
        kind: SkippableKind::YesNo,
        // ★ The FORM conditions this on 7a \u{2014} "If 'Yes,' are you required to file\u{2026}".
        live: |ri| ri.foreign_accounts == Some(true),
        get_bool: |ri| ri.fbar_filing_required,
        set_bool: |ri, v| ri.fbar_filing_required = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★ THE DEATH PAIR (§G-9). Downgraded from class-(A) refusing declarations, where
    // `TaxpayerDiedDuringYear` was `live: |_| true` and therefore blocked EVERY return. See the
    // `SkippableId` docs for why the refusal was redundant: `is_aged`'s `(None, None)` arm already
    // forgoes the addition, so silence has always failed in the safe direction.
    //
    // ★ WHY A GATE PLUS A DATE, rather than asking "did they die before reaching 65?" directly: the
    // day-before-the-birthday convention is exactly the sort of arithmetic a filer gets wrong at the
    // boundary, and we can do it exactly. The gate carries answered-ness, `date_of_death` carries the
    // fact, and `is_aged` applies the convention. Both halves are now class (B).
    SkippableQuestion {
        id: SkippableId::TaxpayerDiedDuringYear,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR by definition: the question names a tax year.
        durability: Durability::PerYear,
        prompt: "Did YOU (the taxpayer named on this return) die during the tax year? (A final return \
                 is filed by a personal representative or surviving spouse. 1040 line 12a: a taxpayer \
                 who died before reaching age 65 does not get the age-65 addition to the standard \
                 deduction, however early in the year they were born.)",
        help: "§63(f) / §G-9. Skipping is lawful and costs nothing unless a date of birth on file would \
               otherwise have qualified you for the age-65 addition — in that case the addition is \
               FORGONE (never granted on an unresolved death carve-out), your tax is OVERSTATED, and \
               `Advisory::AgedBoxForfeitedDeathUnanswered` says so with the amount.",
        kind: SkippableKind::YesNo,
        // ★ ALWAYS LIVE, and that is now harmless. It cannot be scoped to "a DOB old enough to matter"
        // because `live` receives no tax year (the constraint `HasIncomeExclusion` documents), and
        // scoping it to "a DOB is on file" would make the question vanish and reappear as the filer
        // edits — the never-asked-then-silently-relevant shape §G-9 exists to kill. Always-live is only
        // a problem when the question REFUSES; as a skippable it is one Enter.
        live: |_ri| true,
        get_bool: |ri| ri.header.taxpayer_died_during_year,
        set_bool: |ri, v| ri.header.taxpayer_died_during_year = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::SpouseDiedDuringYear,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        prompt: "Did YOUR SPOUSE die during the tax year? (You may still file jointly for the year of \
                 death. 1040 line 12a: a spouse who died before reaching age 65 does not get the \
                 age-65 addition to the standard deduction, however early in the year they were born.)",
        help: "§63(f) / §G-9, spouse. Skipping forgoes the spouse's age-65 box if a qualifying date of \
               birth is on file; the advisory names the amount.",
        kind: SkippableKind::YesNo,
        // ★ Asked exactly when a spouse box can COUNT — the same predicate `AgedBlindBoxes::for_return`
        // uses, so the question and the figure can never disagree. Previously `spouse.is_some()`, which
        // asked (and refused) on returns where nothing could read the reply.
        live: spouse_63f_boxes_count,
        get_bool: |ri| ri.header.spouse_died_during_year,
        set_bool: |ri, v| ri.header.spouse_died_during_year = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },

    // ★★ SCHEDULE C's Form-1099 COMPLIANCE PAIR (lines I and J). Class (B) by the refusal review's
    // criterion: no figure reads them, a blank is no testimony, and the form prints NO Caution beside
    // them — unlike Schedule B's FBAR sub-question, which is the distinction that decided this. The
    // §6721/§6722 exposure is real, so the skip fires an advisory naming both sections.
    SkippableQuestion {
        id: SkippableId::ScheduleC1099Required,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        prompt: "Schedule C line I: did you make any payments this year that would require you to file \
                 Form(s) 1099? (For example, $600 or more to a contractor or service provider for your \
                 business. See the Schedule C instructions.)",
        help: "§6721/§6722 penalise failing to file a required information return and failing to \
               furnish the payee's copy. Skipping is lawful — no figure on your return reads this box, \
               and btctax will never answer it for you — but the box goes out BLANK and the exposure \
               is yours either way.",
        kind: SkippableKind::YesNo,
        live: |ri| ri.schedule_c.is_some(),
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.payments_requiring_1099),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.payments_requiring_1099 = Some(v);
            }
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    SkippableQuestion {
        id: SkippableId::ScheduleC1099Filed,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        prompt: "Schedule C line J: did you, or will you, file those required Form(s) 1099?",
        help: "Asked only because you answered line I \"Yes\" — the form itself says \"If 'Yes,'\". \
               Skipping leaves the box blank; §6721/§6722 still apply.",
        kind: SkippableKind::YesNo,
        // ★ THE FORM CONDITIONS IT ON LINE I — "If 'Yes,'". A Schedule C AND a Yes on I.
        live: |ri| {
            ri.schedule_c
                .as_ref()
                .is_some_and(|c| c.payments_requiring_1099 == Some(true))
        },
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.will_file_required_1099),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.will_file_required_1099 = Some(v);
            }
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },

    // ★★★ Form 8283 Section B lines 5a/5b/5c, asked as ONE return-level universal (§G-21).
    SkippableQuestion {
        id: SkippableId::DonationsHadRestrictions,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        // ★★ THE PROMPT ENUMERATES ALL THREE LIMBS, in the form's own words. A "No" to something
        // vaguer ("any strings?") would be laundered into three specific answers the filer never gave
        // — the widening-an-exemption failure. Enumerate the YES-conditions; anything not listed is
        // then something the filer has NOT denied, so every omission fails closed.
        prompt: "Did ANY property you donated this year have strings attached? Answer YES if any of \
                 these is true of ANY donation: (a) there is a restriction, temporary or permanent, on \
                 the charity's right to USE or DISPOSE of it; (b) you gave anyone other than the \
                 charity a right to its INCOME, to POSSESS it, to VOTE it, or to ACQUIRE it; or (c) \
                 there is a restriction limiting it to a PARTICULAR USE. (Form 8283 lines 5a/5b/5c.)",
        help: "Skipping is harmless if you donated nothing, or nothing over $5,000 — Form 8283 asks \
               these only in Section B. On a year that DOES file a Section B, it is MANDATORY: a \
               \"Yes\" to any limb reduces or denies the §170 deduction (Reg §1.170A-7 — a gift with \
               retained rights is not a gift of the whole thing), and btctax deducts at full fair \
               market value, so it refuses rather than file a number it knows is too large.",
        kind: SkippableKind::YesNo,
        // ★ ALWAYS OFFERED: `live` receives only `ReturnInputs`, and the donations are in the LEDGER.
        //   Silence is lawful HERE so a filer who donated nothing is never blocked; the mandatory half
        //   lives in `screen_absolute`, which has the ledger AND the itemize election, and can tell.
        live: |_ri| true,
        get_bool: |ri| ri.donations_had_restrictions,
        set_bool: |ri, v| ri.donations_had_restrictions = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ §G-28/B1b — Form 8995-A Part I column (b), the SSTB checkbox.
    //
    // ★ APPENDED AT THE END, like every other entry here. `skippable_tristate!` in the input-form
    //   registry couples to this array's INDEX, so inserting mid-array silently repoints every later
    //   question — placing this before `DonationsHadRestrictions` in draft did exactly that.
    SkippableQuestion {
        id: SkippableId::ScheduleCIsSstb,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        prompt: "Is your business a SPECIFIED SERVICE trade or business? Answer YES if its principal \
                 asset is the reputation or skill of its owners or employees, or if it is in health, \
                 law, accounting, actuarial science, performing arts, consulting, athletics, financial \
                 services, brokerage services, or investing and investment management. (Form 8995-A, \
                 Part I, column (b).)",
        help: "Skipping is harmless below the §199A(e)(2) threshold — the simplified Form 8995 does \
               not ask, because the answer changes nothing there. ABOVE it, it is MANDATORY: past the \
               phase-in range an SSTB's qualified business income is EXCLUDED ENTIRELY \
               (§199A(d)(3)), so an unasked \"no\" would hand you a deduction the statute denies and \
               understate your tax. btctax cannot infer it from your business description — it is a \
               checkbox on the form because only you can answer it.",
        kind: SkippableKind::YesNo,
        // ★★ ALWAYS OFFERED, MANDATORY ONLY WHERE IT MATTERS — the `DonationsHadRestrictions` shape,
        //    and for the same reason. `live` sees only `ReturnInputs`, which cannot know taxable
        //    income; making this a live DECLARATION refused EVERY Schedule C return at every income
        //    level, including the great majority for whom the answer is irrelevant. The mandatory half
        //    lives in `screen_absolute`, which knows the threshold.
        live: |ri| ri.schedule_c.is_some(),
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.is_sstb),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.is_sstb = Some(v);
            }
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ §G-28/B1b — Form 8995-A Part I column (e), the PATRON checkbox. Appended at the END, for the
    //     array-index reason above.
    SkippableQuestion {
        id: SkippableId::ScheduleCIsCooperativePatron,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        prompt: "Are you a patron of an agricultural or horticultural cooperative? Answer YES if a \
                 cooperative paid you patronage dividends, per-unit retain allocations, or passed \
                 through a section 199A(g) deduction for this business. (Form 8995-A, Part I, \
                 column (e).)",
        help: "Skipping is harmless if this business has no qualified business income. Where it DOES, \
               the answer decides which form is filed at ANY income: Form 8995-A's own header says to \
               use it if your taxable income is above the threshold \"or you're a patron of an \
               agricultural or horticultural cooperative\", and Form 8995 says the same in reverse. A \
               \"Yes\" REFUSES — the patron reduction comes from Schedule D (Form 8995-A), which \
               btctax does not fill, and filing without it would OVERSTATE your deduction.",
        kind: SkippableKind::YesNo,
        // ★ Offered wherever there is a trade or business to be a patron through; the mandatory half
        //   is in `screen_absolute`, which knows whether a §199A form is actually being printed.
        live: |ri| ri.schedule_c.is_some(),
        get_bool: |ri| ri.schedule_c.as_ref().and_then(|c| c.is_cooperative_patron),
        set_bool: |ri, v| {
            if let Some(c) = ri.schedule_c.as_mut() {
                c.is_cooperative_patron = Some(v);
            }
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ §170(f)(8) — THE CONTEMPORANEOUS WRITTEN ACKNOWLEDGMENT. Index 15; appended at the END for
    //     the array-index reason stated above.
    //
    // ★★ THE PROMPT MUST BE ANSWERABLE **YES** BY A FILER WITH NO ≥$250 GIFT. i1040sca: *"In figuring
    //    whether a gift is $250 or more, don't combine separate donations"* — so a filer who gave $25
    //    a week has no ≥$250 gift at all and answers yes vacuously. That matters because btctax holds
    //    non-crypto gifts as one amount per entry, which may itself be a roll-up of small gifts: the
    //    gate can over-ASK, but the question must never over-CLAIM.
    SkippableQuestion {
        id: SkippableId::CharitableCwaObtained,
        unanswered: None,
        unanswered_detail: "",
        durability: Durability::PerYear,
        // ★★★ THE WORDING COVERS THE DEFERRED CLAIM TOO (final whole-branch review, finding 2).
        //
        // This used to be scoped to gifts "you are deducting this year". Phase 2's R3 fold widened
        // the GATE to §170(b)-ceiling-deferred claims but left these words alone — so a wholly
        // deferred filer deducts nothing this year, reads the question literally, answers YES with
        // perfect honesty while holding no acknowledgment, and the gate passes. The cure then dies at
        // filing anyway. Mechanism and text were adjudicated in different rounds and nobody re-read
        // the words after the population widened.
        prompt: "For EVERY charitable gift of $250 or more that this return DEDUCTS — this year, or \
                 in a later year because it exceeded its §170(b) percentage-of-income ceiling and is \
                 carrying forward — do you \
                 already hold — or will you obtain before you file — a CONTEMPORANEOUS WRITTEN \
                 ACKNOWLEDGMENT from the charity showing (1) the amount of money and a description \
                 (but not the value) of any property donated, and (2) whether the organization gave \
                 you any goods or services in return, with a description and estimate of their value \
                 if it did? (Schedule A lines 11 and 12: \"If you made any gift of $250 or more, see \
                 instructions.\" In figuring whether a gift is $250 or more, don't combine separate \
                 donations — so answer YES if you made no single gift that large. Don't attach the \
                 acknowledgment to your return; keep it for your records. \
                 ★ ANSWER FOR THE CARRYOVER TOO: a gift held back by the ceiling is DEFERRED, not \
                 denied, and §170(f)(8)(C)'s deadline still runs from THIS return — so \"I am \
                 deducting nothing this year\" is not a reason to answer yes.)",
        help: "Skipping is harmless if you claim no charitable deduction AND none of this year's \
               gifts is carrying forward, or if you made no single gift of \
               $250 or more. Where a deduction IS claimed — now or later — it is MANDATORY: \
               §170(f)(8)(A) says \"No \
               deduction shall be allowed … for any contribution of $250 or more unless the taxpayer \
               substantiates the contribution by a contemporaneous written acknowledgment\" — a \
               precondition of the deduction itself, not a recordkeeping nicety. \
               ★ THE DEADLINE IS WHY THIS IS ASKED NOW: §170(f)(8)(C) makes an acknowledgment \
               contemporaneous only if you get it \"by the date you file your return or the due date \
               (including extensions) for filing your return, whichever is earlier\". You can still \
               get one — right up until you file. Once you file without it, the cure is gone.",
        kind: SkippableKind::YesNo,
        // ★ ALWAYS OFFERED — the `DonationsHadRestrictions` shape, and for the same two reasons:
        //   `live` sees only `ReturnInputs`, so it can see neither the LEDGER (where the crypto
        //   donations are) nor the computed §63(e) itemize election. Making this a live class-(A)
        //   DECLARATION would refuse every return at every income level, including the standard-
        //   deduction filers the adjudication says must never be asked. The mandatory half is in
        //   `screen_absolute`, which can tell.
        live: |_ri| true,
        get_bool: |ri| ri.charitable_cwa_obtained,
        set_bool: |ri, v| ri.charitable_cwa_obtained = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ FR-29 — Form 8615's condition 3. **Index 16; APPENDED at the END, and the reason is that
    //     the array-index hazard is REAL here, not that it isn't.** `skippable_tristate!`
    //     (`btctax-input-form/src/spec/registries.rs`) reads `SKIPPABLE_QUESTIONS[$idx]` for its
    //     `label`, `help`, `live`, `get` AND `set` while taking its `id` from a separate argument, and
    //     the call sites pass LITERAL indices — so a mid-array insert repoints every later entry's
    //     prompt, liveness and accessors. It is not silent (the delegation loop reads back through the
    //     `Field`), but it is not free either.
    SkippableQuestion {
        id: SkippableId::Form8615Condition3AgeSupport,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: age and support both change between years.
        durability: Durability::PerYear,
        // ★ Year-free, because `prompt` is `&'static str`. The four departures from
        //   `i1040gi--2025.txt:3932-3940` are deliberate and enumerated in SPEC §4.1: the year
        //   qualifier is generalised and hoisted, four clause openings are recased by the hoist and
        //   the interrogative, and one sentence is appended because the form states the disjunction
        //   STRUCTURALLY ("You were either:") and a flat prompt cannot.
        prompt: "Form 8615, condition 3 — at the end of the tax year, were you either: (a) under age 18, \
                 (b) age 18 and didn’t have earned income that was more than half of your support, or \
                 (c) a full-time student at least age 19 but under age 24 and didn’t have earned income \
                 that was more than half of your support? Answer YES if any one of (a), (b) or (c) is \
                 true.",
        // ★★ The definitions come from i8615, which writes them FOR THIS FORM — not from Chart B,
        //    whose own scope line is the filing-requirement test and which counts scholarships the
        //    opposite way. Both remaining simplifications push toward answering YES — toward refusal,
        //    never away from it (`the_condition_three_help_never_narrows_the_support_test`).
        help: "Form 8615 taxes part of a child's unearned income at the parent's rate (§1(g)). The \
               Instructions for Form 8615 say: \"These rules apply whether or not the child is a \
               dependent.\" Being nobody's dependent, or supporting yourself, does not put you outside \
               them. Skipping is harmless if your unearned income is at or below the §1(g) threshold \
               for the year, or if btctax can already see from your date of birth that you were 24 or \
               older at the end of the year — condition 3 cannot be true at 24. Where it does matter, \
               btctax refuses rather than answer for you: §1(g)(1) takes the GREATER of your own rate \
               and the parent's-rate figure, so a wrong \"no\" can only understate your tax.\n\
               Your SUPPORT is all amounts spent to provide you with food, lodging, clothing, \
               education, medical and dental care, recreation, transportation, and similar necessities, \
               counted from every source — you, your parents and anyone else. A scholarship you \
               received is not counted as support if you are a full-time student.\n\
               EARNED INCOME is wages, tips, and other payments received for personal services \
               performed. If you are a sole proprietor or a partner it can also include a reasonable \
               allowance for your personal services, capped at 30% of your share of the net profits; \
               and it includes any taxable distribution from a qualified disability trust. Income from \
               investments, crypto or a trust is not earned income. The test is whether your EARNED \
               income covered more than half of your support.",
        kind: SkippableKind::YesNo,
        // ★ `!= Mfj` is condition 5, computed. `!provably_24_or_older` is the computed half of
        //   condition 3 — which is what keeps the interview from asking a 60-year-old about full-time
        //   student status. **Condition 1 is deliberately NOT here** (it needs the ledger, SPEC §3.1),
        //   so the live set is a strict SUPERSET of the demanded set.
        live: |ri| {
            ri.filing_status != FilingStatus::Mfj
                && !crate::tax::return_1040::provably_24_or_older(ri, ri.tax_year)
        },
        get_bool: |ri| ri.header.form8615_condition3_age_support,
        set_bool: |ri, v| ri.header.form8615_condition3_age_support = Some(v),
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ FR-29 — Form 8615's condition 4. Index 17, and the registry's ONLY `Choice`.
    SkippableQuestion {
        id: SkippableId::Form8615Condition4ParentAlive,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR: a parent's survival, and what the filer can find out about it, both
        //   change between years.
        durability: Durability::PerYear,
        // ★★ DECLARATIVE, not interrogative: the form's own words "at least one of your parents was
        //    alive" survive only in the declarative, and a three-answer question fits a proposition
        //    better than an inverted verb anyway.
        prompt: "Form 8615, condition 4 — is this true of you: \"at least one of your parents was alive \
                 at the end of the tax year\"? Answer YES, NO, or CANNOT KNOW. Choose CANNOT KNOW only \
                 if you are unable to find out — for example, you do not know who your parents are.",
        help: "Form 8615's condition 4. It is one of five conditions, all of which must hold before \
               Form 8615 is required; btctax asks it only after you answered YES to condition 3, \
               because that is the order the form asks them in. Skipping is harmless if condition 3 is \
               \"no\", or if your unearned income is at or below the §1(g) threshold for the year; \
               otherwise btctax refuses rather than answer for you.\n\
               If neither of your parents was living at the end of the year, answer NO — these rules do \
               not apply to you. If you cannot find out because you do not know who your parents are, \
               answer CANNOT KNOW: that is a different answer from leaving this blank, and btctax will \
               then ask you one further question and explain what it can and cannot do.",
        kind: SkippableKind::Choice(PARENT_ALIVE_CHOICES),
        // ★ Additionally requires condition 3 not to be a definite "no". `None` keeps it live, so both
        //   are offered together on a first run rather than one per pass.
        live: |ri| {
            ri.filing_status != FilingStatus::Mfj
                && !crate::tax::return_1040::provably_24_or_older(ri, ri.tax_year)
                && ri.header.form8615_condition3_age_support != Some(false)
        },
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_choice: |ri| {
            ri.header
                .form8615_condition4_parent_alive
                .map(parent_alive_token)
        },
        set_choice: |ri, v| {
            // ★ An unlisted string is a NO-OP, never an answer: the caller parses, this only stores.
            if let Some(a) = parent_alive_from_token(v) {
                ri.header.form8615_condition4_parent_alive = Some(a);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ FR-29 — the SPEC §6.3 dead-end FACT. Index 18.
    SkippableQuestion {
        id: SkippableId::Form8615ParentIdentityUnobtainable,
        unanswered: None,
        unanswered_detail: "",
        // ★ §G-15 — PER-YEAR, and here it is more than bookkeeping: an attested dead end is testimony
        //   about ONE tax year, and carrying it forward silently would re-file last year's
        //   certification without asking.
        durability: Durability::PerYear,
        // ★ Phrased in the POSITIVE — *can you* — and the certification unlocks on NO. It asks the
        //   filer what they CAN do, which is a fact they can answer without reading anything into it,
        //   rather than inviting them to affirm a conclusion.
        prompt: "Can you give the IRS your parent's name and address? Form 8615 asks for your parent's \
                 name, social security number and filing status, and the only way to get those from the \
                 IRS requires you to supply your parent's name and address. Answer YES if you can \
                 supply both for either parent. Answer NO only if you can supply neither.",
        help: "btctax asks this only because you said you cannot know whether a parent was alive. Form \
               8615 cannot be completed without your parent's name, social security number and filing \
               status (lines A, B and C), and the tax on it cannot be computed without your parent's \
               taxable income. The IRS will send you that information if you ask — but the request must \
               contain \"The name, address, social security number (SSN) (if known), and filing status \
               (if known) of the parent whose information is to be shown on Form 8615\". The SSN and \
               the filing status may be unknown. The name and the address may not.\n\
               If you can supply a name and an address, answer YES: the route is open, and btctax will \
               tell you to use it. If you can supply neither, answer NO, and btctax will attach a \
               disclosure explaining why your return does not include Form 8615. Read that explanation \
               before you file — it is not a ruling in your favour, and it is not free of risk.",
        kind: SkippableKind::YesNo,
        // ★★★ LIVE ONLY ON `Some(CannotKnow)` — this is the owner ruling's first constraint discharged
        //     in the liveness predicate. A filer must FIRST testify that they cannot know before btctax
        //     will even ask the second question. **A general opt-out would rebuild FR-29 behind a nicer
        //     interface**, and the place that cannot happen is here: a question that is never live is
        //     never asked and its `None` never clears anything. Unlike conditions 3 and 4 this depends
        //     on another answer's EXACT value — `!= Some(Yes)` would make it live for an unanswered
        //     condition 4, which is one keystroke from an opt-out.
        live: |ri| {
            ri.filing_status != FilingStatus::Mfj
                && !crate::tax::return_1040::provably_24_or_older(ri, ri.tax_year)
                && ri.header.form8615_condition3_age_support != Some(false)
                && ri.header.form8615_condition4_parent_alive
                    == Some(crate::tax::return_inputs::ParentAliveAnswer::CannotKnow)
        },
        // ★★★ **THE ONLY INVERTING PAIR IN THIS REGISTRY.** The argument and the return are ANSWERS TO
        //     THE PROMPT ("can you supply a name and address?"), which is the negation of the leaf.
        //     Both consumers hand the filer's own boolean straight to `set_bool` and render `get_bool`
        //     straight back as the current answer, so the inversion has to live HERE. Written straight
        //     through, a filer who answered YES — the one for whom the IRS route is open — would be
        //     stored as `unobtainable = Some(true)`, reach ladder step 6's first arm and receive a
        //     computed return with no Form 8615. That is FR-29 rebuilt inside the fix for FR-29, and
        //     `the_certification_is_unreachable_without_the_dead_end`'s polarity row is what kills it.
        get_bool: |ri| ri.header.form8615_parent_identity_unobtainable.map(|v| !v),
        set_bool: |ri, can_supply| {
            ri.header.form8615_parent_identity_unobtainable = Some(!can_supply);
        },
        get_choice: |_ri| None,
        set_choice: |_ri, _v| {},
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
    // ★★★ R7 / T8 — THE HEAD-OF-HOUSEHOLD MARITAL BASIS. Index 19, and the registry's SECOND
    //     `Choice` and its FIRST class-(A) entry — see `SkippableQuestion::unanswered`.
    SkippableQuestion {
        id: SkippableId::HohMaritalBasis,
        unanswered: Some(crate::tax::return_refuse::RefuseReason::HohMaritalBasisUnanswered),
        unanswered_detail:
            "you are filing as HEAD OF HOUSEHOLD. The Form 1040 instructions allow that box only if \
             \"you are unmarried and provide a home for certain other persons\", and they state four \
             ways to be unmarried or considered unmarried for this purpose \
             (i1040gi--2025.txt:1143-1163). Head of household gives a wider bracket and a larger \
             standard deduction than Single or MFS, so which of the four applies is the testimony \
             behind the box \u{2014} it is not one yes/no, because the four are distinct legal \
             predicates. Run `btctax income answer`",
        // ★ §G-15 — PER-YEAR: §7703(a)(1) determines marital status on the LAST DAY of the tax year,
        //   so this is precisely a fact that changes at the year boundary.
        durability: Durability::PerYear,
        // ★ The instruction's bullets name the year ("at the end of 2025"); the quoted spans here
        //   stop short of it, for the reason recorded on the HoH tests above.
        prompt: "Head of household \u{2014} you can check that box \"if you are unmarried and provide a \
                 home for certain other persons\". Which is true of you? Answer NotMarried if you were \
                 not married at the end of the tax year. Otherwise you are \"considered unmarried for \
                 this purpose\" only if one of these applies: LegallySeparatedByDecree \u{2014} \"You were \
                 legally separated according to your state law under a decree of divorce or separate \
                 maintenance at the end of\" the tax year (but an interlocutory decree means you are \
                 considered married); MarriedLivedApart \u{2014} \"You are married but lived apart from \
                 your spouse for the last 6 months of\" the tax year \"and you meet the other rules \
                 under Married persons who live apart\"; NraSpouseNoElection \u{2014} \"You are married and \
                 your spouse was a nonresident alien at any time during the year and the election to \
                 treat the alien spouse as a resident alien is not made\".",
        help: "This is not a benefit you may skip: it is the assertion the Head-of-household box \
               makes. btctax models the first two answers. The last two send you to rules it has \
               not transcribed \u{2014} Married persons who live apart is five further conditions of its \
               own, and Nonresident aliens and dual-status aliens governs whether a joint return \
               may be filed at all \u{2014} so answering either of those stops btctax rather than \
               computing a return on a rule it has not read.",
        kind: SkippableKind::Choice(HOH_MARITAL_BASIS_CHOICES),
        live: |ri| ri.filing_status == FilingStatus::HoH,
        get_bool: |_ri| None,
        set_bool: |_ri, _v| {},
        get_choice: |ri| ri.header.hoh_marital_basis.map(hoh_marital_basis_token),
        set_choice: |ri, v| {
            // ★ An unlisted string is a NO-OP, never an answer: the caller parses, this only stores.
            if let Some(b) = hoh_marital_basis_from_token(v) {
                ri.header.hoh_marital_basis = Some(b);
            }
        },
        get_date: |_ri| None,
        set_date: |_ri, _v| {},
    },
];

/// The stable tokens [`SkippableId::HohMaritalBasis`] stores and a renderer presents — the
/// [`crate::tax::return_inputs::HohMaritalBasis`] variant names, which is what serde writes into a
/// vault. ★ ONE definition, consumed by the registry entry's `kind`, by both accessors and by the
/// input-form `FieldKind::Enum`.
pub const HOH_MARITAL_BASIS_CHOICES: &[&str] = &[
    "NotMarried",
    "LegallySeparatedByDecree",
    "MarriedLivedApart",
    "NraSpouseNoElection",
];

/// [`crate::tax::return_inputs::HohMaritalBasis`] → its stable token.
#[must_use]
pub fn hoh_marital_basis_token(b: crate::tax::return_inputs::HohMaritalBasis) -> &'static str {
    use crate::tax::return_inputs::HohMaritalBasis as H;
    match b {
        H::NotMarried => "NotMarried",
        H::LegallySeparatedByDecree => "LegallySeparatedByDecree",
        H::MarriedLivedApart => "MarriedLivedApart",
        H::NraSpouseNoElection => "NraSpouseNoElection",
    }
}

/// A stable token → its variant, or `None` for anything else.
///
/// ★ No fallback variant and no `#[serde(other)]`: an unknown string must fail rather than silently
/// become an answer.
#[must_use]
pub fn hoh_marital_basis_from_token(t: &str) -> Option<crate::tax::return_inputs::HohMaritalBasis> {
    use crate::tax::return_inputs::HohMaritalBasis as H;
    match t {
        "NotMarried" => Some(H::NotMarried),
        "LegallySeparatedByDecree" => Some(H::LegallySeparatedByDecree),
        "MarriedLivedApart" => Some(H::MarriedLivedApart),
        "NraSpouseNoElection" => Some(H::NraSpouseNoElection),
        _ => None,
    }
}

/// The stable tokens [`SkippableId::Form8615Condition4ParentAlive`] stores and a renderer presents —
/// the `ParentAliveAnswer` variant names, which is what serde writes into a vault.
///
/// ★ ONE definition, consumed by the registry entry's `kind`, by both accessors and by the input-form
/// `FieldKind::Enum`, so a renderer's option list and the parser can never disagree.
pub const PARENT_ALIVE_CHOICES: &[&str] = &["Yes", "No", "CannotKnow"];

/// [`crate::tax::return_inputs::ParentAliveAnswer`] → its stable token.
pub fn parent_alive_token(a: crate::tax::return_inputs::ParentAliveAnswer) -> &'static str {
    use crate::tax::return_inputs::ParentAliveAnswer as P;
    match a {
        P::Yes => "Yes",
        P::No => "No",
        P::CannotKnow => "CannotKnow",
    }
}

/// A stable token → its variant, or `None` for anything else.
///
/// ★ There is no fallback variant and no `#[serde(other)]`: an unknown string must fail rather than
/// silently become an answer.
pub fn parent_alive_from_token(t: &str) -> Option<crate::tax::return_inputs::ParentAliveAnswer> {
    use crate::tax::return_inputs::ParentAliveAnswer as P;
    match t {
        "Yes" => Some(P::Yes),
        "No" => Some(P::No),
        "CannotKnow" => Some(P::CannotKnow),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ THE COMPLETENESS ANCHOR (§3.5). Anchored to the ENUM, not to `FORM_QUESTIONS` — an anti-vacuity
    /// test that ITERATED the list would silently drop its own scenario when an entry was dropped (r1 I-4).
    /// The `match` is exhaustive, so a NEW `QuestionId` variant is a COMPILE ERROR until it is listed here —
    /// a human MUST edit this test, right next to the hardcoded `len() == 8` tripwires. The index round-trip
    /// (r2 M-3) then catches a MIS-ORDERED `ALL`. (Honest limit, IMPL r1 M-1: a human who adds the match arm
    /// but forgets the `ALL` element AND the count still slips through — the compiler forces the edit, not
    /// its correctness, exactly as §3.3 states.)
    #[test]
    fn every_question_id_is_in_all_in_order_and_has_exactly_one_entry() {
        for (i, id) in QuestionId::ALL.iter().enumerate() {
            let idx = match id {
                QuestionId::DependentTaxpayer => 0,
                QuestionId::DependentSpouse => 1,
                QuestionId::MfsSpouseItemizes => 2,
                QuestionId::ForeignAccounts => 3,
                QuestionId::ForeignTrust => 4,
                QuestionId::HsaActivity => 5,
                QuestionId::DualStatusAlien => 6,
                QuestionId::MortgageAllUsedToBuyBuildImprove => 7,
                QuestionId::AmtQualifiedDwelling => 8,
                QuestionId::AmtCarryoverSameAsRegular => 9,
                QuestionId::AmtDepreciationSameAsRegular => 10,
                QuestionId::HasIncomeExclusion => 11,
                QuestionId::OtherOutOfScopeIncome => 12,
                QuestionId::MortgageWithinDebtLimit => 13,
                QuestionId::FilingForm4952 => 14,
                QuestionId::CarryoverIncludesSpousesJointLoss => 15,
                QuestionId::ExcludedCanceledDebt => 16,
                // ★ R3 / §5.1 — the eighteen document-census rows, indices 17..=34.
                QuestionId::DocW2 => 17,
                QuestionId::DocInt1099 => 18,
                QuestionId::DocDiv1099 => 19,
                QuestionId::DocB1099 => 20,
                QuestionId::DocG1099 => 21,
                QuestionId::DocForm1098 => 22,
                QuestionId::DocForm1098e => 23,
                QuestionId::DocR1099 => 24,
                QuestionId::DocSsa1099 => 25,
                QuestionId::DocNecMiscK1099 => 26,
                QuestionId::DocK1 => 27,
                QuestionId::DocScheduleERental => 28,
                QuestionId::DocS1099 => 29,
                QuestionId::DocOid1099 => 30,
                QuestionId::DocW2g => 31,
                QuestionId::DocC1099 => 32,
                QuestionId::DocA1095 => 33,
                QuestionId::DocT1098 => 34,
                QuestionId::FilingStatusConfirmed => 35,
                // ★ R3 / T5 — the document-less income door, indices 36..=39.
                QuestionId::WagesWithoutW2Question => 36,
                QuestionId::InterestOrDividendsWithout1099 => 37,
                QuestionId::StateRefundWithout1099g => 38,
                QuestionId::ItemizedPriorYear => 39,
                // ★ R9 / T6 — Form 1040 page 1's Digital Assets question, index 40.
                QuestionId::DigitalAssetActivity => 40,
                // ★ T16 / FR-76 — the two HSA census rows and Form 8889's seven questions.
                QuestionId::DocSa1099 => 41,
                QuestionId::DocSa5498 => 42,
                QuestionId::HsaFamilyCoverage => 43,
                QuestionId::HsaEligibleEveryMonth => 44,
                QuestionId::HsaAge55OrOlder => 45,
                QuestionId::HsaMedicareEnrollment => 46,
                QuestionId::HsaBothSpousesHaveHsas => 47,
                QuestionId::HsaArcherMsaActivity => 48,
                QuestionId::HsaTestingPeriodFailure => 49,
                // ★ Seam review I-3 (the spouse's plan) and M-1 (the document-less distribution).
                QuestionId::HsaSpouseFamilyCoverage => 50,
                QuestionId::HsaDistributionWithout1099sa => 51,
                QuestionId::FilerTinIssuedByDueDate => 52,
                // ★ R7 / T8 — HoH and QSS, the two filing statuses that had no test.
                QuestionId::HohQualifyingPerson => 53,
                QuestionId::HohPaidOverHalfCostOfKeepingUpHome => 54,
                QuestionId::NraSpouseResidentElection => 55,
                QuestionId::QssSpouseDiedInWindowAndNotRemarried => 56,
                QuestionId::QssChildYouCanClaim => 57,
                QuestionId::QssChildLivedInYourHomeAllYear => 58,
                QuestionId::QssPaidOverHalfCostOfKeepingUpHome => 59,
                QuestionId::QssCouldHaveFiledJointlyInYearOfDeath => 60,
                // ★★★ R8 / T9 — real estate.
                QuestionId::ClaimingMortgageInterestCredit => 61,
                QuestionId::SoldMainHome => 62,
                QuestionId::HomeSaleTest1OwnedAndLived => 63,
                QuestionId::HomeSaleTest2NoRecentExclusion => 64,
                QuestionId::HomeSaleCanExcludeAllGain => 65,
            };
            assert_eq!(idx, i, "QuestionId::ALL is out of order / missing {id:?}");
            assert_eq!(
                FORM_QUESTIONS.iter().filter(|q| q.id == *id).count(),
                1,
                "exactly one FORM_QUESTIONS entry for {id:?}"
            );
        }
        assert_eq!(
            QuestionId::ALL.len(),
            66,
            "17 declarations + the 20 R3 document-census rows + R10.4's filing-status confirmation \
             + T5's four document-less-income-door questions + R9/T6's Digital Assets question \
             + T16's seven Form 8889 questions + the T16 seam review's two (the SPOUSE's HDHP \
             plan, and R3's document-less distribution door) + T7/R6's filer-TIN question \
             + R7/T8's eight (two HoH tests, FR-67's \u{a7}6013(g)/(h) election gate, five QSS \
             conditions) + R8/T9's five (the Form 8396 gate and the four sale-of-a-main-home \
             answers)"
        );
        assert_eq!(FORM_QUESTIONS.len(), 66, "one entry per declaration");
    }

    /// ★★★ §G-6/ISO — THE OUT-OF-SCOPE QUESTION MUST NAME THE ISO EXERCISE, WHICH IS NOT INCOME.
    ///
    /// `form6251.rs` models Part I lines 2, 2a and 2b only; 2c–2t are absent, and line **2i** is the
    /// exercise of an incentive stock option — the dominant real AMT trigger post-TCJA, since the 2017
    /// Act removed the SALT and miscellaneous-deduction add-backs that used to drive individual AMT.
    ///
    /// ★★★ THE OLD PROMPT COULD NOT CATCH IT, and the reason is in i6251's own first sentence:
    /// *"For the regular tax, no income is recognized when an incentive stock option (ISO), as defined
    /// in section 422(b), is exercised. However, this rule doesn't apply for the AMT."* The question
    /// asked *"did you RECEIVE any income…"*, so a filer with a $180,000 ISO adjustment answered a
    /// truthful **No** and the gate stayed shut.
    ///
    /// ★★ AND THE GAP HID ITS OWN DETECTION: `must_attach()` is `line7 > line10`, and the missing 2i
    /// add-back is exactly what would have pushed line 7 past line 10 — so the return never tripped
    /// `AmtScreenTriggered`, filed clean with no Form 6251 and no AMT, and understated tax on a return
    /// signed under §6065. Invisible to both oracles and to every value-checking test.
    ///
    /// This pins the PROMPT, because the prompt is the whole mechanism: nothing computes from it.
    #[test]
    fn the_out_of_scope_question_names_the_iso_exercise_and_the_amt_items() {
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::OtherOutOfScopeIncome)
            .expect("the out-of-scope declaration exists");
        let p = q.prompt.to_ascii_lowercase();
        assert!(
            p.contains("incentive stock option") && p.contains("3921"),
            "the prompt must name the ISO exercise AND the form the filer already holds: {}",
            q.prompt
        );
        assert!(
            p.contains("alternative minimum tax"),
            "…and must name the AMT category, since lines 2c-2t are all silently zero: {}",
            q.prompt
        );
        // ★★★ **RETIREMENT INCOME, added 2026-09-04, and the reason is an UNDERSTATEMENT path.**
        //     This enumeration primes the filer: someone holding a 1099-R and an SSA-1099 who reads
        //     "rent or royalties, a farm, a partnership…" and finds nothing resembling their pension
        //     can answer a truthful-feeling **No** on the strength of the list, even though the
        //     trailing "or anything else it never asked about" formally covers it. §61 and §86 income
        //     then leaves the return silently. btctax models no line 4a-6b at all, so the ONLY thing
        //     standing between a retiree and an understated return is this sentence naming their
        //     forms. Name the FORM NUMBERS, not just the category — the filer is holding the paper.
        //     ★ Removing any of these must red: that is the whole guarantee, since nothing computes
        //     from this prompt. See design/ty2025/SPEC_retirement_income.md (OQ-1).
        for limb in [
            "pension",
            "annuity",
            "1099-r",
            "social security",
            "ssa-1099",
        ] {
            assert!(
                p.contains(limb),
                "the prompt must name retirement income and the form the filer already holds — \
                 `{limb}` is missing, and a retiree who answers No on the strength of this list \
                 files omitting §61/§86 income: {}",
                q.prompt
            );
        }
        // ★★★ **ADDITIONS TO TAX, added 2026-09-05 (FR-30).** The prompt asked about income
        //     received, an ISO, and AMT items. An ADDITION TO TAX is none of the three, so a filer
        //     owing Schedule 2 Part II tax could answer every question truthfully and file without
        //     it. Verified against the form (`design/forms/extract/f1040s2--2025.txt`): line 8
        //     "Additional tax on IRAs or other tax-favored accounts. Attach Form 5329", line 9
        //     "Household employment taxes. Attach Schedule H", line 17b "Recapture of federal
        //     mortgage subsidy". Grep confirms btctax models NONE of them.
        //     ★ Naming the FORM is the point — the filer is holding the paper, not the statute.
        for limb in ["form 5329", "schedule h", "8962", "mortgage subsidy"] {
            assert!(
                p.contains(limb),
                "the prompt must name the additions to tax and the form each arrives on — `{limb}` \
                 is missing, and a filer owing Schedule 2 Part II tax can answer No and omit it: {}",
                q.prompt
            );
        }
        // ★ The income limbs must SURVIVE the widening — this question's original job is unchanged.
        for limb in ["rent", "royalt", "k-1", "alimony", "gambling"] {
            assert!(
                p.contains(limb),
                "the income limbs must remain: {limb} missing"
            );
        }
        // ★★ And it must still be a MANDATORY class-(A) declaration that refuses unanswered. A prompt
        //    nobody has to answer would make all of the above decoration.
        assert!((q.live)(&ReturnInputs::default()), "always live");
        assert_eq!(q.unanswered, RefuseReason::OtherIncomeUnanswered);
        assert_eq!(
            (q.get)(&ReturnInputs::default()),
            None,
            "and `Default` must not answer it"
        );
    }

    /// ★ **`SkippableId::ALL` is complete and in registry order** — the mirror of
    /// `every_question_id_is_in_all_in_order_and_has_exactly_one_entry`, and it exists for a load-
    /// bearing reason: [`crate::tax::provenance::AnswerKey`]'s wire parser scans `ALL`, so a variant
    /// missing from it is a stored answer that can never be read back. The `match` is exhaustive, so
    /// a NEW variant is a compile error here until a human lists it.
    #[test]
    fn every_skippable_id_is_in_all_in_registry_order_with_exactly_one_entry() {
        for (i, id) in SkippableId::ALL.iter().enumerate() {
            let idx = match id {
                SkippableId::BlindTaxpayer => 0,
                SkippableId::BlindSpouse => 1,
                SkippableId::SalesTaxElection => 2,
                SkippableId::DobTaxpayer => 3,
                SkippableId::DobSpouse => 4,
                SkippableId::DodTaxpayer => 5,
                SkippableId::DodSpouse => 6,
                SkippableId::TaxpayerDiedDuringYear => 7,
                SkippableId::SpouseDiedDuringYear => 8,
                SkippableId::FbarFilingRequired => 9,
                SkippableId::ScheduleC1099Required => 10,
                SkippableId::ScheduleC1099Filed => 11,
                SkippableId::DonationsHadRestrictions => 12,
                SkippableId::ScheduleCIsSstb => 13,
                SkippableId::ScheduleCIsCooperativePatron => 14,
                SkippableId::CharitableCwaObtained => 15,
                SkippableId::Form8615Condition3AgeSupport => 16,
                SkippableId::Form8615Condition4ParentAlive => 17,
                SkippableId::Form8615ParentIdentityUnobtainable => 18,
                // ★ R7 / T8 — index 19, the registry's first class-(A) entry.
                SkippableId::HohMaritalBasis => 19,
            };
            assert_eq!(idx, i, "SkippableId::ALL is out of order / missing {id:?}");
            assert_eq!(
                SKIPPABLE_QUESTIONS.iter().filter(|s| s.id == *id).count(),
                1,
                "exactly one SKIPPABLE_QUESTIONS entry for {id:?}"
            );
        }
        assert_eq!(SkippableId::ALL.len(), SKIPPABLE_QUESTIONS.len());
    }

    #[test]
    fn the_skippable_registry_is_separate_and_has_twenty_entries_with_correct_liveness() {
        use crate::tax::types::FilingStatus;
        assert_eq!(
            SKIPPABLE_QUESTIONS.len(),
            20,
            "blind ×2, SALT, DOB ×2, DOD ×2, FBAR, the §G-9 death pair, Schedule C I/J, 8283 5a/5b/5c, \
             8995-A SSTB + patron, §170(f)(8) CWA, FR-29's Form 8615 trio (condition 3, condition 4 \
             and the §6.3 dead-end fact), and R7/T8's HoH marital basis"
        );
        // ★★★ R7 / T8 — the registry is class (B) BY RULE, and this is the ONE entry that declares
        //     otherwise. Asserted as the whole SET so a second class-(A) entry has to be looked at:
        //     the panel and the screen both branch on `unanswered`, and "class (B) except where
        //     declared" is only a safe rule while its exceptions are visible.
        let class_a: Vec<_> = SKIPPABLE_QUESTIONS
            .iter()
            .filter(|s| s.unanswered.is_some())
            .map(|s| s.id)
            .collect();
        assert_eq!(
            class_a,
            vec![SkippableId::HohMaritalBasis],
            "exactly one class-(A) entry: a CHOICE whose silence is not lawful"
        );
        for s in SKIPPABLE_QUESTIONS {
            assert_eq!(
                s.unanswered.is_some(),
                !s.unanswered_detail.is_empty(),
                "{:?}: a class-(A) entry carries a refusal detail and a class-(B) one carries none",
                s.id
            );
        }
        // SALT is live iff a schedule_a exists; spouse-blind iff a spouse Person exists.
        let salt = SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == SkippableId::SalesTaxElection)
            .unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        assert!(!(salt.live)(&ri));
        ri.schedule_a = Some(Default::default());
        assert!((salt.live)(&ri));
        // The skippables are NOT in FORM_QUESTIONS (merging would brick screen_inputs on a None-legal skippable).
        for s in SKIPPABLE_QUESTIONS {
            assert!(
                !FORM_QUESTIONS
                    .iter()
                    .any(|q| format!("{:?}", q.id) == format!("{:?}", s.id)),
                "a skippable must not also be a mandatory FORM_QUESTIONS declaration"
            );
        }
    }

    /// ★★★ **§G-15 — the year gate, and the reason the always-live workaround had to go.**
    ///
    /// `HasIncomeExclusion` computes modified AGI, which only TY2025+ reads (TY2024's SALT cap is a
    /// `FlatCap` that ignores `magi` entirely). It shipped ALWAYS LIVE because `live` had no year to
    /// consult, so TY2024 filers were asked a TY2025 question — defensible ONLY because a bespoke
    /// neutrality proof existed for that one question.
    ///
    /// ★★ The proof does not generalise, and this test exists so the next year-scoped question is
    /// written as a gate rather than as another workaround: Schedule 1-A Part IV asks about a
    /// deduction that **did not exist in TY2024**, so a "no" there is testimony about nothing.
    #[test]
    fn the_income_exclusion_question_is_live_only_from_ty2025() {
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::HasIncomeExclusion)
            .expect("the question is in the registry");

        let at = |y: i32| ReturnInputs {
            tax_year: y,
            ..Default::default()
        };

        assert!(
            !(q.live)(&at(2024)),
            "TY2024 must NOT be asked a TY2025 MAGI question — that was the §G-15 defect"
        );
        assert!(
            (q.live)(&at(2025)),
            "TY2025 computes modified AGI, so it must be asked"
        );
        assert!((q.live)(&at(2026)), "and every later year");

        // ★ Fail-closed on an unstated year: a fixture that never went through storage has
        // `tax_year: 0`, and 0 must not conjure a TY2025 question out of nothing.
        assert!(
            !(q.live)(&at(0)),
            "an UNSTATED year must not be treated as 2025 — the gate fails closed"
        );
    }

    /// ★★★ **§G-15 — durability, and the rule that decides it.**
    ///
    /// **A question is `Durable` only if its subject cannot change once known.** Two consequences,
    /// and the first is structural rather than a hand-list:
    ///
    /// 1. **No class-(A) DECLARATION may ever be `Durable`.** Every one asserts about a *tax year*
    ///    ("in this tax year, did…"), so last year's answer is not testimony for this one. Marking
    ///    one durable would let a prior year's answer satisfy this year's provenance — the
    ///    answered-ness invariant breached across a year boundary, which is software answering for
    ///    the filer, one year removed.
    /// 2. Among the class-(B) skippables, exactly the **dates of birth** qualify. Blindness can
    ///    change; a §164(b)(5) election is made per year; and a date of DEATH is gated on
    ///    `…DiedDuringYear`, itself a per-year declaration.
    #[test]
    fn only_facts_that_cannot_change_are_durable() {
        let durable_decls: Vec<_> = FORM_QUESTIONS
            .iter()
            .filter(|q| q.durability == Durability::Durable)
            .map(|q| q.id)
            .collect();
        assert!(
            durable_decls.is_empty(),
            "no DECLARATION may be Durable — each asserts about a tax year, so a prior year's answer \
             is not testimony for this one. Offending: {durable_decls:?}"
        );

        let durable_skips: Vec<_> = SKIPPABLE_QUESTIONS
            .iter()
            .filter(|s| s.durability == Durability::Durable)
            .map(|s| s.id)
            .collect();
        assert_eq!(
            durable_skips,
            vec![SkippableId::DobTaxpayer, SkippableId::DobSpouse],
            "exactly the dates of BIRTH are durable. Blindness changes, the sales-tax election is \
             per-year, and a date of DEATH is gated on a per-year declaration — so adding anything \
             here needs an argument that its subject genuinely cannot change"
        );
    }

    /// ★ **The default direction is to RE-ASK.** Anything not deliberately marked durable must be
    /// `PerYear`, so a new registry entry fails toward asking the filer rather than toward reusing
    /// an answer they did not give this year.
    #[test]
    fn everything_not_explicitly_durable_is_per_year() {
        let total = FORM_QUESTIONS.len() + SKIPPABLE_QUESTIONS.len();
        let accounted = FORM_QUESTIONS
            .iter()
            .filter(|q| matches!(q.durability, Durability::PerYear | Durability::Durable))
            .count()
            + SKIPPABLE_QUESTIONS
                .iter()
                .filter(|s| matches!(s.durability, Durability::PerYear | Durability::Durable))
                .count();
        assert_eq!(
            accounted, total,
            "every registry entry must state a durability — the field is not optional, so this can \
             only fail if a variant is added without deciding what it means for carry-forward"
        );
    }
}
