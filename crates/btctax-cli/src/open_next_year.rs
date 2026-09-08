//! ★★★ **T4b — THE YEAR-N+1 OPENER** (`SPEC_interview.md` R10 part 4, §6 J-27).
//!
//! *"Year N+1 opens with year N's **identities** — each payer by TIN, each dependent, each venue —
//! as fresh tri-state prompts. Yes opens a screen with the identity pre-named and **every box
//! blank**; every `PerYear` gate re-asked blank; every `Durable` fact shown and confirmed by the
//! same keystroke a fresh answer takes; carryforwards arrive as data. **Never a carried amount.**"*
//!
//! **The one rule this module exists to keep**, from `questions.rs:21-25`: *"a prior-year answer
//! must NEVER silently satisfy this year's provenance."* Three consequences, and every one of them
//! is a structural property of [`seed`] rather than a promise:
//!
//! 1. **The seed starts from [`ReturnInputs::default`] and copies IN**, never from year N's row with
//!    fields blanked out. The two are not the same shape of mistake: a copy-and-blank forgets a
//!    field by leaving year N's answer in it, and nothing reds. A copy-in forgets a field by leaving
//!    it BLANK — which the census, the registry and `screen_inputs` already refuse. The direction of
//!    failure is the whole design.
//! 2. **`answer_log` is empty.** A `Durable` fact (a date of birth) is *displayed* on the seeded
//!    draft, and it carries no [`AnswerRecord`] until the filer confirms it with a fresh `SetField`
//!    — which writes a fresh record through `record_answer`, dated by the `BTCTAX_NOW` seam and
//!    hashing the prompt as it reads TODAY. This module writes no record; `record_answer` remains
//!    the one writer.
//! 3. **The only figures that cross the boundary are the carryforwards**, read from year N's frozen
//!    RETURN (the carryover-OUT chain) and never from year N's inputs, and stamped
//!    [`CarryProvenance::ComputedFromPriorReturn`] so a reader can tell a figure btctax computed
//!    from one the filer typed — and so a computed ZERO stops being indistinguishable from an unasked
//!    one (§G-23's "stated zero").
//!
//! It writes the **draft** (R11: on a year whose package has not arrived, the draft is the only
//! store there is), never `return_inputs::set` — so `return_inputs::get(N+1)` still answers `None`
//! after a successful open, and nothing lands at `resolve.rs` precedence 1.
//!
//! ★ **Retention is untouched.** Nothing about year N is read-modify-written, nothing is deleted,
//!   nothing is shredded (§G-14 is not built here; `FIELD_PROVENANCE.md:443-446` says the retention
//!   window is the filer's decision, not one this tool picks).

use crate::{input_form_store, return_inputs, CliError, Session};
use btctax_core::tax::document_census::DocumentRow;
use btctax_core::tax::return_inputs::{
    CarryProvenance, Form1099B, Form1099Div, Form1099G, Form1099Int, HouseholdHeader, Person,
    ReturnInputs, W2,
};

/// The earliest tax year this build has any table for, and the latest year it will open INTO — the
/// bounds `--from` is checked against (N-2). Deliberately generous at the top: opening a future year
/// is how the interview is meant to be used in January, and the refusals below say what is missing.
const MIN_YEAR: i32 = 2014;
const MAX_YEAR: i32 = 2100;

/// One identity year N knew about, presented to the filer as its own tri-state prompt.
///
/// ★★ **The prompt is per IDENTITY; the ANSWER is the census row.** R10.4's sentence names one
/// payer — *"Last year Acme (EIN 12-3456789) issued you a W-2. Did Acme issue one for 2027?"* — but
/// `FORM_QUESTIONS` is a `&'static` array and cannot grow a row per employer, and a second
/// answer-log key kind would be a second writer of `answer_log`, which R10.3 forbids. So the
/// identity list is what the filer READS, and the kind's census row is what they ANSWER: `Yes` keeps
/// the pre-named rows to transcribe into, `No` removes them
/// ([`btctax_core::tax::document_census::answer_row`]), and `None` leaves them pending and blocking,
/// exactly as the census does for every other filer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The census row whose tri-state IS this identity's answer, where one exists. `None` for a
    /// dependent and for a venue: neither has a census row today (a dependent's own declarations are
    /// T7's `DEPENDENT_GATES`; a venue's are the per-cohort Form 1099-DA answers, whose
    /// answered-ness lives in the key set).
    pub census_row: Option<DocumentRow>,
    /// R10.4's sentence, in the form's words, naming the identity as the filer's paperwork names it.
    pub prompt: String,
    /// The answer standing on the seeded draft. `None` on every identity the opener seeds — that is
    /// the guarantee, not an observation: nothing here answers for the filer.
    pub answer: Option<bool>,
}

/// What [`open_next_year`] did — the identity prompts, and the only figures that crossed the year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// Year N — the year the identities and carryforwards came from.
    pub from: i32,
    /// Year N+1 — the year whose DRAFT was seeded.
    pub to: i32,
    /// One per identity, in a stable order (documents by census order, then dependents, then venues).
    pub identities: Vec<Identity>,
    /// ★★★ **T4b seam review I-1 — EVERYTHING THAT CROSSED, NAMED.**
    ///
    /// The build said *"every box is blank and every question is unanswered"* in four places while
    /// the filing status, the taxpayer's name, SSN and mailing address crossed silently — *"a filer
    /// who reads any of those has no reason to look"*. This list is what `render`, `--help`, the man
    /// page and the TUI offer all print, so the assertion of blankness is bounded by its own
    /// exceptions.
    pub carried_identity: Vec<String>,
    /// What was carried, DERIVED FROM THE ASSIGNMENT rather than from a hand-list — the same rule
    /// `write_back_carryover`'s summary learned the hard way (its capital-loss line printed on the
    /// branch where the gate skipped the write).
    pub carried: Vec<String>,
    /// ★★ **NAMED EITHER WAY** (T9's rule, and `write_back_carryover`'s own): `Some(sentence)` when
    /// the §1212(b) capital-loss roll was DELIBERATELY not stamped, because year N was never asked
    /// about a carryover and produced none of its own. A silent omission is a worse answer than a
    /// truthful *"not this one, and here is why"* — the filer would otherwise read four carryovers
    /// into a list of three.
    pub not_stamped: Option<String>,
    /// `Some(why)` when year N's carryforward chain could not be computed — no package for that
    /// year, a pseudo-tainted or blocked ledger, a return that refuses. The identities are still
    /// seeded; NOTHING is stamped, so next year's `BenefitCarryoversNotStated` advisory stays live
    /// about a carryover the filer may genuinely have.
    pub not_carried: Option<String>,
}

impl Opened {
    /// The opener's report, as `btctax income open-next-year` prints it.
    #[must_use]
    pub fn render(&self) -> String {
        let (to, from) = (self.to, self.from);
        // ★★★ I-1 — the blankness claim is now BOUNDED BY ITS EXCEPTIONS, in the first sentence.
        //     Saying "everything is blank" while the filing status and the household identity cross
        //     is the defect; saying which ones crossed is the fix, and it is said before anything
        //     else so a filer cannot miss it.
        let mut s = format!(
            "Opened TY{to} from TY{from}.\n  Carried from TY{from} — CONFIRM each: {}.\n  \
             Everything else is blank and every question is unanswered.\n",
            self.carried_identity.join(", ")
        );
        s.push_str(&format!(
            "  What TY{from} knew about, each its own question:\n"
        ));
        // ★ N-1 — a `format!`, not a literal placeholder patched by `s.replace` over the whole
        //   accumulated string (which was correct only because the header had already interpolated
        //   its own `{from}`).
        if self.identities.is_empty() {
            s.push_str(&format!(
                "  (TY{from} carried no payer, dependent or venue to confirm.)\n"
            ));
        }
        for id in &self.identities {
            s.push_str(&format!("  · {}\n", id.prompt));
        }
        if !self.carried.is_empty() {
            s.push_str(&format!(
                "  carried from the TY{from} return: {}\n",
                self.carried.join("; "),
                from = self.from
            ));
        }
        if let Some(why) = &self.not_stamped {
            s.push_str(&format!("  {why}\n"));
        }
        if let Some(why) = &self.not_carried {
            s.push_str(&format!(
                "\n★ NOT CARRIED: no carryforward was stamped onto TY{to}. {why}\n  Nothing was \
                 written for them, so TY{to} still counts them as never asked.\n",
                to = self.to
            ));
        }
        s.push_str(&format!(
            "\nThe answers live in the TY{to} DRAFT — `btctax income answer --year {to}`, or the \
             tax-inputs form.\n"
        ));
        // ★★ M-1 — the opener's normal output is a non-trivial TY{to} draft, which permanently
        //    REFUSES `report --tax-year {from} --write-carryover`; that refusal then prescribes
        //    `--discard-draft`, and following it destroys the year that was just opened. Said here,
        //    once, at the moment the draft is created.
        if !self.carried.is_empty() {
            s.push_str(&format!(
                "  (TY{from}'s carryforwards are already on it, so `report --tax-year {from} \
                 --write-carryover` is not needed and will refuse while this draft exists. Do not \
                 pass `--discard-draft` to it — that discards the year you just opened.)\n"
            ));
        }
        s
    }
}

/// ★★★ **`btctax income open-next-year --from N`** — seed year N+1's DRAFT from year N.
///
/// Refuses, writing nothing, when:
/// - year N+1 holds a **parked** draft, or a WIP draft holding work and `--discard-draft` was not
///   given (T4's rule and T4's flag — this is a write onto year N+1, and it is not privileged);
/// - year **N** has no committed row (there is nothing to open FROM, and a draft-only year N is a
///   year whose identities were themselves never confirmed);
/// - year **N+1** already has a committed row (it has been opened, imported or committed already,
///   and re-seeding would overwrite a return that exists — the opener is not an editor).
///
/// ★ The draft-coherence check comes FIRST, for T4's M-1 reason: a parked year has no committed row,
///   so a "year N+1 already exists" / "year N does not" message would otherwise shadow the parked
///   refusal's own remedy.
pub fn open_next_year(
    sess: &mut Session,
    from: i32,
    discard_draft: bool,
) -> Result<Opened, CliError> {
    // ★ N-2 — the arithmetic came BEFORE every refusal, so `--from 2147483647` panicked under
    //   `debug_assertions` before any of them could speak. Bounded to the years this build knows
    //   about at all: `checked_add` alone would still accept a year no table, form or filer has.
    let to = from
        .checked_add(1)
        .filter(|_| (MIN_YEAR..=MAX_YEAR).contains(&from))
        .ok_or_else(|| {
            CliError::Usage(format!(
                "{from} is not a tax year this build can open from — btctax knows \
                 {MIN_YEAR}..={MAX_YEAR}."
            ))
        })?;
    // (1) Every refusal a write onto year N+1 can raise, raised before anything is computed, and
    //     DELETING NOTHING — `coherence_check` is the read half; the destructive half runs below,
    //     next to the write it belongs to.
    let coherence = input_form_store::coherence_check(sess.conn(), to, discard_draft)?;
    // (2) There must be a year N to open FROM.
    let prior = return_inputs::get(sess.conn(), from)?.ok_or_else(|| {
        CliError::Usage(format!(
            "there are no full-return inputs for {from}, so there is nothing to open {to} from. The \
             opener carries {from}'s identities and its computed carryforwards forward; import or \
             finish {from} first (`btctax income import --year {from} --file <toml>`)."
        ))
    })?;
    // (3) And year N+1 must not already exist. Re-seeding a committed row would replace a return the
    //     filer has already screened and stored with a blank one.
    if return_inputs::get(sess.conn(), to)?.is_some() {
        return Err(CliError::Usage(format!(
            "year {to} already has a stored full return, so it is not opened from {from} — the \
             opener starts a year, it does not reset one. Edit it in the tax-inputs form, or clear \
             it first (`btctax income clear --year {to}`, which discards what it holds)."
        )));
    }

    let mut seeded = seed(&prior, to);
    let identities = identities_of(&prior, to, &seeded);

    // (4) The carryforwards, read off year N's FROZEN RETURN by the same chain
    //     `report --write-carryover` uses — including every gate that decides whether btctax may
    //     hand a figure across a year boundary at all (pseudo-tainted ledger, a not-computable
    //     ledger, a §170(f)(8) or Reg §1.170A-7 carryover it cannot vouch for).
    let mut carried: Vec<String> = Vec::new();
    let mut not_carried: Option<String> = None;
    let mut not_stamped: Option<String> = None;
    let seed_for_roll = seeded.clone();
    match crate::cmd::tax::roll_carryover_onto(sess, from, |_| Ok(seed_for_roll), false) {
        Ok(rolled) => {
            let mut updated = rolled.updated;
            // ★★★ THE ONE THING R10.4 ADDS to the write-back's stamp: the figure crossed a YEAR.
            //     `Computed` means *"this year's write-back derived it"*; `ComputedFromPriorReturn`
            //     says which return it came off, and closes §G-23's "stated zero".
            restamp_from_prior_return(&mut updated, from);
            let (c, n) = describe_carried(&updated, &rolled.ar, &rolled.ri);
            carried = c;
            not_stamped = n;
            seeded = updated;
        }
        // ★ The `usage: ` prefix `CliError::Usage` renders is dropped: inside the opener's own
        //   sentence it reads as though the FILER mistyped something, when what happened is that
        //   year N could not be computed. The sentence itself — the refusal's own words, naming the
        //   remedy — is kept verbatim.
        Err(e) => {
            let text = e.to_string();
            not_carried = Some(text.strip_prefix("usage: ").unwrap_or(&text).to_string());
        }
    }

    // ★ AFTER the roll, deliberately: the carryforwards are part of what crossed, and a list built
    //   before they were written would name every field except the ones carrying money. (Caught by
    //   `every_leaf_the_seed_carries_is_named_in_the_report`, which is what that kill is for.)
    let carried_identity = carried_identity(&seeded);

    // (5) The write. `coherence_clear` first, so the draft this open supersedes is gone before the
    //     seed is written into its place; `save_draft` reaches disk (I-7).
    input_form_store::coherence_clear(sess.conn(), to, &coherence)?;
    input_form_store::save_draft(sess, to, &seeded)?;
    Ok(Opened {
        from,
        to,
        identities,
        carried_identity,
        carried,
        not_stamped,
        not_carried,
    })
}

/// ★★★ **WHAT CROSSES, IN THE FILER'S WORDS — and the leaves each phrase stands for.**
///
/// The seam review's I-1: four surfaces asserted *"every box is blank and every question is
/// unanswered"* while the filing status, the taxpayer's name, SSN and mailing address crossed. This
/// table is the one place that answers *"what crossed?"*, and
/// [`tests::every_leaf_the_seed_carries_is_named_in_the_report`] holds it to the seed: any leaf the
/// seed writes that no phrase here claims fails the build. So the report cannot fall behind `seed`.
///
/// `always` marks a field that crosses **by construction** even when its value equals the default —
/// [`ReturnInputs::filing_status`] has no `None`, so a Single filer's carried status is
/// byte-identical to a defaulted one, and a diff can never see it. That is the miniature of the
/// finding itself: *"the assertion cannot distinguish carried from defaulted"*.
const CARRIED_IDENTITY: &[(&str, &[&str], bool)] = &[
    (
        "the filing status (a divorce or a death changes it, and §7703(a)(1) determines it on the last day of the year)",
        &["filing_status", "filing_status_confirmed"],
        true,
    ),
    (
        "your name and SSN",
        &["header.taxpayer", "header.spouse"],
        false,
    ),
    ("your mailing address", &["header.address_"], false),
    (
        "each employer and payer, by name and EIN/TIN, with every box blank",
        &["w2s", "int_1099", "div_1099", "g_1099", "b_1099"],
        false,
    ),
    (
        "the carryforwards computed on that return",
        &[
            "capital_loss_carryforward_in",
            "charitable_carryover_in",
            "qbi.",
        ],
        false,
    ),
    (
        "and which year this one was opened from",
        &["opened_from", "tax_year"],
        true,
    ),
];

/// The [`CARRIED_IDENTITY`] phrases that are TRUE of this seed.
fn carried_identity(seeded: &ReturnInputs) -> Vec<String> {
    let changed = leaves_the_seed_writes(seeded);
    CARRIED_IDENTITY
        .iter()
        .filter(|(_, prefixes, always)| {
            *always
                || changed
                    .iter()
                    .any(|leaf| prefixes.iter().any(|p| leaf.starts_with(p)))
        })
        .map(|(label, _, _)| (*label).to_string())
        .collect()
}

/// Every serde leaf on which `seeded` DIFFERS from a blank return for the same year — the honest
/// answer to *"what did the opener put here?"*, asked of the seed itself rather than of a comment.
///
/// ★ It walks with T1's own machinery (`provenance::leaf_walk`), so a field added to
///   [`ReturnInputs`] tomorrow is compared the day it is added.
fn leaves_the_seed_writes(seeded: &ReturnInputs) -> Vec<String> {
    use btctax_core::tax::provenance::leaf_walk;
    let blank = serde_json::to_value(ReturnInputs {
        tax_year: seeded.tax_year,
        ..Default::default()
    })
    .expect("ReturnInputs serializes");
    let doc = serde_json::to_value(seeded).expect("ReturnInputs serializes");
    let mut leaves = Vec::new();
    leaf_walk::walk(&doc, "", &mut leaves);
    leaves
        .into_iter()
        .filter(|path| leaf_walk::at(&blank, path) != leaf_walk::at(&doc, path))
        .collect()
}

/// ★★★ **THE SEED — built by copying IN, never by blanking out.**
///
/// Everything not named here is [`ReturnInputs::default`]: every money box, every `PerYear` gate,
/// every census row, the whole `answer_log`. See the module header for why that direction is the
/// design and not a detail.
///
/// **What crosses, and why each one is an identity rather than an answer:**
///
/// | carried | what it is |
/// |---|---|
/// | `filing_status` | not expressible as *unanswered* (the field has no `None`), and `Default` would assert **Single** — a fabricated answer, and the wrong one for every MFJ filer. It is carried as the STARTING POINT the year's own form asks the filer to confirm, exactly as `draft_is_disposable` already treats *"a tax year and a filing status"* as opening a year rather than as work. |
/// | taxpayer / spouse name + SSN | who the return is FOR. Not testimony about the tax year. |
/// | the two dates of birth | the `Durable` facts (`questions.rs`) — *shown*, and confirmed by the same keystroke a fresh answer takes. **No `AnswerRecord` comes with them.** |
/// | the mailing address | where the return is sent; the same class as a payer's name. |
/// | each dependent's name, SSN, relationship, date of birth | R10.4's *"each dependent"*. Every §152 gate on the row stays `None`. |
/// | each W-2 employer + EIN, each 1099 payer + TIN, per kind | R10.4's *"each payer by TIN"* — pre-named rows with **every box default**. |
/// | each venue | the Form 1099-DA provider keys, each with **both cohort slots unanswered** (answered-ness lives in the slot, so an empty `CohortAnswers` claims nothing and `screen_broker_reporting` reads nothing from it). |
///
/// **What deliberately does NOT cross:** the IP PIN (the IRS issues a new one every filing season, so
/// last year's is not merely unconfirmed but *wrong*); `occupation` and every header tri-state
/// (`can_be_claimed_as_dependent_*`, the died-during-year pair, blindness, a date of death); the
/// document census itself; Schedule A, Schedule C, Schedule 1, Schedule 1-A, payments; and every
/// single dollar except the carryforwards.
#[must_use]
pub fn seed(prior: &ReturnInputs, to: i32) -> ReturnInputs {
    let carry_person = |p: &Person| Person {
        first_name: p.first_name.clone(),
        last_name: p.last_name.clone(),
        ssn: p.ssn.clone(),
        // ★★★ **C-1 — the `Durable` date of birth is SHOWN, NEVER PRE-FILLED.**
        //
        //     It used to cross. `Durability::Durable` says *"the prior MAY be displayed, but it
        //     still requires the same explicit keystroke as a fresh ask: never Enter-to-accept,
        //     never pre-filled"* — and pre-filling it broke that in one command: `income answer`'s
        //     `skippable_state` decides `Given` vs `Declined` by reading the VALUE, so the
        //     documented SKIP keystroke (a bare Enter) left year N's date in place and wrote a
        //     fresh `AnswerRecord { state: Given }` dated this year. A prior-year answer satisfying
        //     this year's provenance, which is the one thing R10 exists to prevent — and *"a
        //     diligence record that lies is worse than none"*.
        //
        //     So the seed leaves it blank and `opened_from` carries the year instead: `income
        //     answer` reads year N's row at prompt time and SHOWS the date as a hint the filer must
        //     type to confirm. A bare Enter then records `Declined` and the §63(f) addition is
        //     lawfully forgone — which is the truthful outcome of skipping.
        date_of_birth: None,
        ..Default::default()
    };
    ReturnInputs {
        tax_year: to,
        filing_status: prior.filing_status,
        header: HouseholdHeader {
            taxpayer: carry_person(&prior.header.taxpayer),
            spouse: prior.header.spouse.as_ref().map(carry_person),
            address_street: prior.header.address_street.clone(),
            address_city: prior.header.address_city.clone(),
            address_state: prior.header.address_state.clone(),
            address_zip: prior.header.address_zip.clone(),
            // ★★★ **I-2 — DEPENDENTS ARE NOT SEEDED.** A `Dependent` row IS the claim: it prints
            //     the person, their SSN and their relationship in the 1040 Dependents grid — sworn
            //     testimony — and there is nothing on this year's return that can answer for it. No
            //     census row, no `FormQuestion`, no `RefuseReason`, and (machine-checked in the
            //     review) no `interview_state` item. The child who aged out, moved out, or is
            //     claimed by the other parent would ride across the year silently.
            //
            //     R10.4 says each identity is SHOWN as its own prompt, and a shown identity that is
            //     not seeded is exactly that: `identities_of` names every prior dependent from year
            //     N's row, and the filer adds back the ones still theirs. **FR-70 (T7)** is where
            //     the row returns — once `DEPENDENT_GATES` exist, there is something to answer.
            dependents: Vec::new(),

            ..Default::default()
        },
        w2s: prior
            .w2s
            .iter()
            .map(|w| W2 {
                owner: w.owner,
                employer: w.employer.clone(),
                ein: w.ein.clone(),
                ..Default::default()
            })
            .collect(),
        int_1099: prior
            .int_1099
            .iter()
            .map(|r| Form1099Int {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            })
            .collect(),
        div_1099: prior
            .div_1099
            .iter()
            .map(|r| Form1099Div {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            })
            .collect(),
        g_1099: prior
            .g_1099
            .iter()
            .map(|r| Form1099G {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            })
            .collect(),
        b_1099: prior
            .b_1099
            .iter()
            .map(|r| Form1099B {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            })
            .collect(),
        // ★★★ **T16 — the HSA TRUSTEE is a payer identity like any other.** An HSA trustee sends a
        //     Form 1099-SA every year money leaves the account, so R10.4's sentence — *"Last year
        //     Fidelity (TIN 12-3456789) issued you a Form 1099-SA. Did they issue one for 2027?"* —
        //     is exactly as true of them as of a bank. Every BOX is blank, including box 5's
        //     account-type checkbox: the seed carries an identity, never testimony, and a blank box
        //     5 REFUSES until the filer transcribes it, which is the fail-closed direction.
        sa_1099: prior
            .sa_1099
            .iter()
            .map(|r| btctax_core::tax::return_inputs::Form1099Sa {
                payer: r.payer.clone(),
                payer_tin: r.payer_tin.clone(),
                ..Default::default()
            })
            .collect(),
        sa_5498: prior
            .sa_5498
            .iter()
            .map(|r| btctax_core::tax::return_inputs::Form5498Sa {
                trustee: r.trustee.clone(),
                trustee_tin: r.trustee_tin.clone(),
                ..Default::default()
            })
            .collect(),
        // ★★★ **I-3 — NO VENUE KEY.** `broker_reporting`'s own contract is *"absent = unanswered:
        //     answered-ness lives in the KEY SET, never in a sentinel value"*, and an inserted key
        //     with an empty `CohortAnswers` is precisely that sentinel. Three call sites read
        //     presence in the key set as *"the filer stored 1099-DA answers"* — `admin.rs`'s
        //     `answers_stored`, `resolve.rs`, `cmd/tax.rs` — and the first resolves through the
        //     DRAFT, so the seeded key reintroduced the sentence R6 fold M-4 had just fixed:
        //     *"prints the crypto slice from the stored answers"*, on a year holding none.
        //
        //     The key bought nothing: the venue prompt below reads `prior`, and
        //     `screen_broker_reporting` derives the keys that need answering from the LEDGER's own
        //     Form 8949 rows.
        opened_from: Some(prior.tax_year),
        ..Default::default()
    }
}

/// R10.4's sentence for every identity the seed carries, in a stable order.
///
/// ★ The document rows are walked from `DocumentRow::ALL` and `declared_rows`, never from a
///   hand-list of five `Vec` fields, so a kind that gains a transcription section is prompted for
///   the day it gains one.
fn identities_of(prior: &ReturnInputs, to: i32, seeded: &ReturnInputs) -> Vec<Identity> {
    use btctax_core::tax::document_census::declared_rows;
    let mut out = Vec::new();
    for row in DocumentRow::ALL {
        let n = declared_rows(prior, *row).unwrap_or(0);
        for i in 0..n {
            let (who, id_clause) = payer_of(prior, *row, i);
            out.push(Identity {
                census_row: Some(*row),
                prompt: format!(
                    "Last year {who}{id_clause} issued you a {doc}. Did {who} issue one for {to}?",
                    doc = row.designation()
                ),
                answer: seeded.documents.get(*row),
            });
        }
    }
    for d in &prior.header.dependents {
        out.push(Identity {
            census_row: None,
            // ★ The SSN is NOT printed. `income show` masks a dependent's SSN and so does this: the
            //   payer identifiers below are a business's, and a child's is not.
            prompt: format!(
                "Last year you claimed {name} ({rel}) as a dependent. Is {name} your dependent for \
                 {to}?",
                name = d.name,
                rel = d.relationship
            ),
            answer: None,
        });
    }
    for provider in prior.broker_reporting.0.keys() {
        out.push(Identity {
            census_row: None,
            prompt: format!(
                "Last year you answered the Form 1099-DA questions for {provider}. What did \
                 {provider} report for your {to} dispositions?"
            ),
            answer: None,
        });
    }
    out
}

/// The payer's own name and identifying number for one transcribed row — *"Acme"*, *" (EIN
/// 12-3456789)"*.
fn payer_of(ri: &ReturnInputs, row: DocumentRow, i: usize) -> (String, String) {
    let clause = |label: &str, id: &str| {
        if id.is_empty() {
            String::new()
        } else {
            format!(" ({label} {id})")
        }
    };
    match row {
        DocumentRow::W2 => ri.w2s.get(i).map_or_else(Default::default, |w| {
            (
                w.employer.clone(),
                clause("EIN", w.ein.as_deref().unwrap_or_default()),
            )
        }),
        DocumentRow::Int1099 => ri.int_1099.get(i).map_or_else(Default::default, |r| {
            (r.payer.clone(), clause("TIN", &r.payer_tin))
        }),
        DocumentRow::Div1099 => ri.div_1099.get(i).map_or_else(Default::default, |r| {
            (r.payer.clone(), clause("TIN", &r.payer_tin))
        }),
        DocumentRow::G1099 => ri.g_1099.get(i).map_or_else(Default::default, |r| {
            (r.payer.clone(), clause("TIN", &r.payer_tin))
        }),
        DocumentRow::B1099 => ri.b_1099.get(i).map_or_else(Default::default, |r| {
            (r.payer.clone(), clause("TIN", &r.payer_tin))
        }),
        // ★ M-2 — EXHAUSTIVE, no `_`. A kind that gains a transcription section must fail to
        //   COMPILE here rather than yield a nameless prompt (*"Last year  issued you a Form
        //   1099-R…"*) and be silently unseeded. The compensating kill existed, but it lived in
        //   another crate — the compiler is the right instrument for an omission this shape.
        // ★ T16 — the HSA trustee, named the same way every other payer is.
        DocumentRow::Sa1099 => ri.sa_1099.get(i).map_or_else(Default::default, |r| {
            (r.payer.clone(), clause("TIN", &r.payer_tin))
        }),
        DocumentRow::Sa5498 => ri.sa_5498.get(i).map_or_else(Default::default, |r| {
            (r.trustee.clone(), clause("TIN", &r.trustee_tin))
        }),
        DocumentRow::Form1098
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
        | DocumentRow::T1098 => Default::default(),
    }
}

/// Re-stamp every carryover the write-back marked `Computed` as
/// [`CarryProvenance::ComputedFromPriorReturn`], naming the year it was computed on.
///
/// ★ `Computed` and `ComputedFromPriorReturn` are not the same claim: the first says *"this year's
///   `report --write-carryover` derived it"*, the second that the figure crossed a year boundary and
///   is the ONLY thing the opener carries. An ungrounded capital-loss roll is never stamped by the
///   write-back and is therefore never re-stamped here — `User` on a zero still means *never asked*,
///   which is what keeps `BenefitCarryoversNotStated` honest.
fn restamp_from_prior_return(next: &mut ReturnInputs, from: i32) {
    let mark = |p: &mut CarryProvenance| {
        if *p == CarryProvenance::Computed {
            *p = CarryProvenance::ComputedFromPriorReturn { year: from };
        }
    };
    mark(&mut next.capital_loss_carryforward_in_provenance);
    mark(&mut next.charitable_carryover_in_provenance);
    mark(&mut next.qbi.reit_ptp_carryforward_in_provenance);
    mark(&mut next.qbi.qbi_carryforward_in_provenance);
    for c in &mut next.charitable_carryover_in {
        mark(&mut c.provenance);
    }
}

/// What crossed, DERIVED FROM THE ASSIGNMENT — never from the destination field.
///
/// ★★ The distinction is `write_back_carryover`'s B-1, and it is repeated here rather than
///    re-derived: the capital-loss roll is the one GATED write, so on an ungrounded year the field
///    holds a zero nobody assigned. Reading the field would report a write that did not happen. The
///    predicate is [`btctax_core::capital_loss_roll_is_grounded`] — the same one the write asked.
fn describe_carried(
    updated: &ReturnInputs,
    ar: &btctax_core::AbsoluteReturn,
    prior: &ReturnInputs,
) -> (Vec<String>, Option<String>) {
    let mut out = vec![
        format!(
            "{} charitable carryover item(s)",
            updated.charitable_carryover_in.len()
        ),
        format!(
            "QBI REIT/PTP carryforward ${:.2}",
            updated.qbi.reit_ptp_carryforward_in
        ),
        format!(
            "QBI business-loss carryforward ${:.2}",
            updated.qbi.qbi_carryforward_in
        ),
    ];
    if btctax_core::capital_loss_roll_is_grounded(ar, prior) {
        out.push(format!(
            "capital-loss carryover short ${:.2} / long ${:.2}",
            updated.capital_loss_carryforward_in.short, updated.capital_loss_carryforward_in.long
        ));
        return (out, None);
    }
    (
        out,
        Some(format!(
            "★ NOT the capital-loss carryover: TY{year} was never asked about one and produced none \
             of its own, so btctax has no §1212(b) figure it can vouch for and stamps nothing — \
             TY{next} still counts it as never asked.",
            year = prior.tax_year,
            next = prior.tax_year + 1
        )),
    )
}
