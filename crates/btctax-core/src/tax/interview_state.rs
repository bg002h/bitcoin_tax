//! ★★★ **R12 — THE ANSWER PANEL, derived from the REGISTRIES, and CLOSED by `screen_inputs`.**
//!
//! `SPEC_input_form.md` §7 forbids refactoring `screen_inputs` to collect all refusals — its
//! early-return tiers are semantic, and each tier's precedence is load-bearing. The panel needs none
//! of that for the items it can address: every question already declares its own liveness, its own
//! prompt and its own refusal, so the walk is a walk and not a second screen.
//!
//! ★★ **But the walk is not the gate**, and pretending otherwise is what the T12 seam review found
//! (I-1). The registries can produce five of `RefuseReason`'s 126 variants; `screen_inputs` — what
//! `input_form_store::commit` runs before it writes — raises all of them. So the walk ENDS by
//! running that screen at its value tier ([`crate::tax::return_refuse::screen_param_free`]) and
//! taking any refusal the registries did not already model. The panel's list is therefore empty only
//! when the screen raises nothing at that tier, which is the only thing that makes the zero-open
//! sentence *"no answer refuses"* true rather than merely usually true. What it still cannot see is
//! stated where that sentence is printed, and in `LIMITATIONS.md`: the five package-gated rules,
//! which compare a figure against the year's TaxTable and which this walk does not hold.
//!
//! **Seven states, exactly R12's table:**
//!
//! | state | listed as |
//! |---|---|
//! | not live | counted, silent ([`InterviewState::not_live`]) |
//! | live, answered, prompt unchanged | counted ([`InterviewState::answered`]) |
//! | live, answered, `prompt_hash` ≠ the current prompt | **blocking** (class A) / **forgoing** (class B), reason [`WORDING_CHANGED_REASON`] |
//! | live, unanswered, class (A) | **blocking** |
//! | live, unanswered, class (B) | **forgoing** |
//! | live, answered, **and the answer refuses** | **refusing**, with the refusal's own exit sentence |
//! | live, class (B), `Declined` | **forgoing, marked *(declined)*** |
//! | live but **unstatable** — the prompt must quote a year-package figure that has not arrived | **waiting** |
//!
//! ★★ **`Declined` finally has its production reader.** `provenance.rs` records that
//! [`AnswerState::Declined`] was *"WRITTEN here and READ by nothing in production yet … Its reader is
//! T3 — `interview_state()`"*. It is [`InterviewState::forgoing`], marked, and never
//! [`InterviewState::blocking`]: declining is provenance (asked, refused), and dropping the item from
//! the list exactly when the forgo becomes FINAL is backwards.
//!
//! ★ **No progress bar and no persisted "remaining"** (R15, `FIELD_PROVENANCE.md:123-125`). The panel
//! is computed on demand from the registries and the return; nothing about it is stored.

use crate::conventions::Usd;
use crate::tax::dependent_gates::{
    gate_is_answered, walk_dependent, DependentVerdict, DEPENDENT_GATES,
};
use crate::tax::document_census::{row_of_question, DocumentRow};
use crate::tax::provenance::{
    answer_status, dependent_ssn_hash, AnswerKey, AnswerStatus, DependentGate,
    WORDING_CHANGED_REASON,
};
use crate::tax::questions::{QuestionId, SkippableId, FORM_QUESTIONS, SKIPPABLE_QUESTIONS};
use crate::tax::return_inputs::ReturnInputs;
use crate::tax::return_refuse::RefuseReason;
use crate::tax::tables::FullReturnParams;
use crate::tax::types::FilingStatus;

/// The identity of one panel item — the same key an answer is stored under, so the renderer can move
/// the cursor to it and the filer's next keystroke lands on the thing the panel named.
///
/// ★ [`AnswerKey`] rather than a new enum, deliberately: a second identity space is a second thing to
/// keep in step, and the panel's whole job is to point at answers.
pub type PanelItem = AnswerKey;

/// A live class-(A) declaration that is not answered under today's words. **Commit is blocked.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocking {
    pub item: PanelItem,
    /// The words the filer will be shown.
    pub prompt: std::borrow::Cow<'static, str>,
    /// Why it is listed: unanswered, or answered under earlier words ([`WORDING_CHANGED_REASON`]).
    pub reason: &'static str,
    /// What the answer accounts for — the refusal's own detail, which carries the statutory cite and
    /// the remedy. Not derived from the prompt: a prompt-derived text drops both.
    pub accounts_for: &'static str,
}

/// A live class-(B) prompt whose silence FORGOES a benefit. **Commit is not blocked.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forgo {
    pub item: PanelItem,
    pub prompt: std::borrow::Cow<'static, str>,
    /// What skipping forgoes — the registry's own `help` text, or the changed-wording reason.
    pub benefit: &'static str,
    /// ★★ The size of the forgone benefit **where computable**. `None` on a params-less year: R12
    /// says blank rather than guessed, because a figure invented from no package is worse than a
    /// gap the filer can see.
    pub size: Option<Usd>,
    /// ★★★ The filer was ASKED and passed over — [`AnswerState::Declined`]. Still forgoing, and
    /// marked, because declining is provenance and the benefit is still gone.
    ///
    /// [`AnswerState::Declined`]: crate::tax::provenance::AnswerState::Declined
    pub declined: bool,
}

/// A live question whose ANSWER refuses — a census row `Some(true)` on a type btctax cannot take,
/// or any `Yes` whose gate carries a refusal. Shown **while authoring**, so the filer meets it here
/// instead of at commit (J-32).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusing {
    /// The answer to go and change — `None` for a refusal the COMMIT SCREEN raises that no single
    /// registry answer owns (T12 fold, seam review I-1). A screen rule can read several fields at
    /// once (the home sale reads four), and inventing a key for it would point the filer's cursor
    /// at one of them as though it were the cause.
    pub item: Option<PanelItem>,
    pub prompt: std::borrow::Cow<'static, str>,
    /// Derived from the question's own refusal, never re-decided here.
    pub reason: RefuseReason,
    /// The exit sentence the filer is given.
    pub exit: String,
}

/// A live prompt that cannot be STATED until the year's package arrives — it must quote a
/// [`FullReturnParams`] figure the year does not yet have. **Never blocking**; it becomes blocking
/// the moment the package lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    pub item: PanelItem,
    pub prompt: std::borrow::Cow<'static, str>,
    /// The package it waits on, named so the filer knows what they are waiting for.
    pub waiting_on: &'static str,
}

/// ★★★ **R6 / T8 — a benefit the FORM computes on a schedule btctax does not file.**
///
/// Not a question and not a skippable, so it has no [`PanelItem`]: nothing the filer can answer
/// changes it. It is listed because a forgone credit the filer never hears about is the same defect
/// as an unasked question — 1040 line 19 is a carry from Schedule 8812, btctax emits none, and the
/// line is therefore the filer's own blank (`advisories::ctc_odc_line19`).
///
/// ★★ **The row-(7) boxes ARE printed** on the TY2025+ grid (R6): the box says the dependent
/// QUALIFIES, the line says what the schedule computed. This item is what tells the filer the
/// difference is money.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotComputed {
    /// What is not computed, in the filer's own terms.
    pub what: &'static str,
    /// How many rows on this return it applies to.
    pub count: usize,
    /// The per-row ceiling from the YEAR'S PACKAGE. ★ `None` on a params-less year — R12's rule,
    /// the same one [`Forgo::size`] follows: a figure invented from no package is worse than a gap
    /// the filer can see.
    pub each: Option<Usd>,
    /// Where the figure would come from — named so a filer on a params-less year knows what they
    /// are waiting for, rather than seeing an unexplained blank.
    pub sized_from: &'static str,
}

impl NotComputed {
    /// The panel's sentence — assembled here, once, so the CLI and the TUI cannot word it
    /// differently. `None` for `each` prints the count with no figure.
    #[must_use]
    pub fn line(&self) -> String {
        match self.each {
            Some(x) => format!(
                "{} not computed \u{2014} {} with a credit box; up to {x} each",
                self.what,
                self.rows()
            ),
            None => format!(
                "{} not computed \u{2014} {} with a credit box (size waiting on {})",
                self.what,
                self.rows(),
                self.sized_from
            ),
        }
    }
    fn rows(&self) -> String {
        match self.count {
            1 => "1 dependent".to_string(),
            n => format!("{n} dependents"),
        }
    }
}

/// Every open item on the return, in one call (R12).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterviewState {
    pub blocking: Vec<Blocking>,
    pub forgoing: Vec<Forgo>,
    pub refusing: Vec<Refusing>,
    pub waiting: Vec<Waiting>,
    /// ★★★ R6 / T8 — benefits the form computes elsewhere. Never blocking; never answerable.
    pub not_computed: Vec<NotComputed>,
    /// Live and answered under today's words.
    pub answered: usize,
    /// Not live on this return — counted, silent.
    pub not_live: usize,
}

impl InterviewState {
    /// Nothing blocks commit. (`forgoing` may be non-empty and still allow a filing — that is the
    /// whole point of class (B).)
    #[must_use]
    pub fn is_committable(&self) -> bool {
        self.blocking.is_empty() && self.refusing.is_empty()
    }
    /// How many items the panel would print.
    #[must_use]
    pub fn open_items(&self) -> usize {
        self.blocking.len()
            + self.forgoing.len()
            + self.refusing.len()
            + self.waiting.len()
            + self.not_computed.len()
    }
}

/// ★★★ **The prompts that cannot be stated without a year-package figure**, as `(question, the
/// package it waits on)`.
///
/// **Empty today, and deliberately so** — and the mechanism is nonetheless exercised, because an
/// empty table would otherwise be an instrument nobody has watched discriminating (harness B1). The
/// walk takes this table as a PARAMETER ([`interview_state_with`]), so
/// [`tests::a_params_gated_prompt_waits_instead_of_blocking_and_moves_when_the_package_lands`]
/// plants an occupant and watches it move between `waiting` and `blocking`.
///
/// ★ Its first real occupant is **T7's** `gross_income_under_limit`, whose prompt must quote the
/// §152(d) gross-income figure for the year (R6 / M7). When T7 lands, its entry goes here and the
/// planted test keeps meaning the same thing.
pub const PARAMS_GATED_PROMPTS: &[(QuestionId, &str)] = &[];

/// What a params-quoting prompt is WAITING ON, named so the filer knows what they are waiting for.
///
/// ★★★ **T7 landed `gross_income_under_limit` as a `DependentGate`, not a `QuestionId`**, so it
/// could not go in [`PARAMS_GATED_PROMPTS`] — that table is keyed by the return-level registry. It
/// needs no table at all: a gate declares its own params dependence by carrying a
/// `prompt_from_params` renderer, so *waiting* is DERIVED from the registry entry rather than looked
/// up in a second list keyed by hand. [`PARAMS_GATED_PROMPTS`] stays for the return-level case, with
/// its planted-occupant test unchanged.
pub const YEAR_PACKAGE: &str = "the tax year's parameter package (FullReturnParams)";

/// ★★★ **THE PANEL.** Every blocking, forgoing, refusing and waiting item on this return, derived
/// from the registries (R12).
///
/// Sizes are blank: a forgone §63(f) box has no size without the year's package. Use
/// [`interview_state_with_params`] where the package is in hand.
#[must_use]
pub fn interview_state(ri: &ReturnInputs) -> InterviewState {
    interview_state_with(ri, None, PARAMS_GATED_PROMPTS)
}

/// [`interview_state`] on a year whose package HAS arrived — the forgo sizes are then filled in.
///
/// ★ Two entry points rather than one optional argument at the call sites, because R12's kill is
/// exactly the pair: *"the forgo size present when params exist and absent when they do not"*.
#[must_use]
pub fn interview_state_with_params(ri: &ReturnInputs, p: &FullReturnParams) -> InterviewState {
    interview_state_with(ri, Some(p), PARAMS_GATED_PROMPTS)
}

/// The walk, with both variables exposed so a test can drive them.
fn interview_state_with(
    ri: &ReturnInputs,
    params: Option<&FullReturnParams>,
    params_gated: &[(QuestionId, &'static str)],
) -> InterviewState {
    let mut st = InterviewState::default();

    // ── 1. FORM_QUESTIONS — class (A), and the census rows live here too (R3 makes each census row
    //       a registry entry, so this is ONE walk and not two). ────────────────────────────────────
    for q in FORM_QUESTIONS {
        if !(q.live)(ri) {
            st.not_live += 1;
            continue;
        }
        let item = AnswerKey::Question(q.id);
        // A prompt that cannot be STATED yet waits, whatever its answer: asking a question whose
        // words are not yet knowable is how a filer is made to answer something else.
        if let Some((_, pkg)) = params_gated.iter().find(|(id, _)| *id == q.id) {
            st.waiting.push(Waiting {
                item,
                prompt: q.prompt_text(ri),
                waiting_on: pkg,
            });
            continue;
        }
        match (q.get)(ri) {
            None => st.blocking.push(Blocking {
                item,
                prompt: q.prompt_text(ri),
                reason: "this question has not been answered",
                accounts_for: q.unanswered_detail,
            }),
            Some(v) => {
                // ★★★ R10.3 — an answer given under EARLIER words does not stand under later ones.
                // ★ R10.4 — hashed against the words RENDERED for this return, so a question that
                //   quotes a value (the carried filing status) is re-asked when the value changes.
                if answer_status(ri, &item) == AnswerStatus::WordingChanged {
                    st.blocking.push(Blocking {
                        item,
                        prompt: q.prompt_text(ri),
                        reason: WORDING_CHANGED_REASON,
                        accounts_for: q.unanswered_detail,
                    });
                    continue;
                }
                // ★★ An ANSWERED question whose answer REFUSES — shown while authoring rather than
                //    met at commit (J-32). Derived from the question's own refusal.
                if let Some(r) = refusal_of_answer(ri, q.id, v) {
                    st.refusing.push(Refusing {
                        item: Some(item),
                        prompt: q.prompt_text(ri),
                        reason: r.0,
                        exit: r.1,
                    });
                    continue;
                }
                st.answered += 1;
            }
        }
    }

    // ── 2. SKIPPABLE_QUESTIONS — class (B). Silence is lawful, and it still forgoes. ─────────────
    for s in SKIPPABLE_QUESTIONS {
        if !(s.live)(ri) {
            st.not_live += 1;
            continue;
        }
        let item = AnswerKey::Skippable(s.id);
        let has_value = skippable_has_value(s, ri);
        let status = answer_status(ri, &item);
        // ★★★ **R7 / T8 — the CLASS-(A) entries of this registry are BLOCKING, not forgoing.**
        //
        // The split between the two registries is the ANSWER SHAPE, not the class: R7's HoH marital
        // basis is one of the instruction's four named states, so it needs the `Choice` accessors,
        // and its silence is not lawful. `SkippableQuestion::unanswered` is what declares that, and
        // this reads the declaration rather than a list — the screen reads the same one, so the
        // panel and the refusal cannot drift.
        if let Some(reason) = s.unanswered.as_ref() {
            let _ = reason;
            if !has_value || status == AnswerStatus::WordingChanged {
                st.blocking.push(Blocking {
                    item,
                    prompt: std::borrow::Cow::Borrowed(s.prompt),
                    reason: if has_value {
                        WORDING_CHANGED_REASON
                    } else {
                        "this question has not been answered"
                    },
                    accounts_for: s.unanswered_detail,
                });
            } else {
                st.answered += 1;
            }
            continue;
        }
        match status {
            AnswerStatus::WordingChanged => st.forgoing.push(Forgo {
                item,
                prompt: std::borrow::Cow::Borrowed(s.prompt),
                benefit: WORDING_CHANGED_REASON,
                size: forgo_size(s.id, ri, params),
                declined: false,
            }),
            // ★★★ ASKED AND PASSED OVER. Listed, marked, and never blocking. Only `Given` removes it.
            AnswerStatus::Declined => st.forgoing.push(Forgo {
                item,
                prompt: std::borrow::Cow::Borrowed(s.prompt),
                benefit: s.help,
                size: forgo_size(s.id, ri, params),
                declined: true,
            }),
            AnswerStatus::Given | AnswerStatus::NeverAsked => {
                if has_value {
                    st.answered += 1;
                } else {
                    st.forgoing.push(Forgo {
                        item,
                        prompt: std::borrow::Cow::Borrowed(s.prompt),
                        benefit: s.help,
                        size: forgo_size(s.id, ri, params),
                        declined: false,
                    });
                }
            }
        }
    }

    // ── 3. The dependent gates × rows (T7 / R6). ─────────────────────────────────────────────────
    //
    // ★★ **Liveness is the WALK's**, so it is computed ONCE per row and both the per-gate liveness
    //    and the flowchart's own STOP are read off the same value — two calls would be two chances
    //    to disagree.
    debug_assert!(
        DependentGate::ALL.len() == DEPENDENT_GATES.len(),
        "DEPENDENT_GATES is total over the gate identities"
    );
    for (row, d) in ri.header.dependents.iter().enumerate() {
        let walk = walk_dependent(ri, row);
        for g in DEPENDENT_GATES {
            if !walk.demands(g.gate) {
                st.not_live += 1;
                continue;
            }
            let item = AnswerKey::DependentGate {
                ssn_hash: dependent_ssn_hash(&d.ssn),
                gate: g.gate,
            };
            // ★★★ **R12's `waiting` row, and its first real occupant.** A prompt that must QUOTE a
            //     year-package figure cannot be STATED until the package arrives, so it is listed as
            //     *waiting*, never as blocking: asking a question whose words are not yet knowable
            //     is how a filer is made to answer something else. It becomes blocking the moment
            //     params exist — which is what the two entry points to this walk are for.
            if g.needs_params() && params.is_none() {
                st.waiting.push(Waiting {
                    item,
                    prompt: g.prompt_text(ri, None),
                    waiting_on: YEAR_PACKAGE,
                });
                continue;
            }
            let prompt = g.prompt_text(ri, params);
            if !gate_is_answered(d, g.gate) {
                st.blocking.push(Blocking {
                    item,
                    prompt,
                    reason: "this question has not been answered",
                    accounts_for: g.unanswered_detail,
                });
                continue;
            }
            // R10.3 — an answer given under earlier words does not stand under later ones. For the
            // params-quoting gate the comparand is the RENDERED prompt, which only exists here.
            let stale = if g.needs_params() {
                ri.answer_log
                    .get(&item)
                    .is_some_and(|r| r.prompt_hash != crate::tax::provenance::prompt_hash(&prompt))
            } else {
                answer_status(ri, &item) == AnswerStatus::WordingChanged
            };
            if stale {
                st.blocking.push(Blocking {
                    item,
                    prompt,
                    reason: WORDING_CHANGED_REASON,
                    accounts_for: g.unanswered_detail,
                });
                continue;
            }
            st.answered += 1;
        }
        // ★★ An ANSWERED gate whose answer STOPS the flowchart — shown while authoring rather than
        //    met at commit (J-32), and derived from the walk's own refusal, never re-decided here.
        //
        // ★★ Two shapes, and the difference is what the item POINTS AT (seam review M-1): a gate
        //    the filer can change on this row, or the RETURN-level question that refused every row
        //    at once. The panel item is the thing to go and fix.
        let stopped = match walk.verdict {
            DependentVerdict::Refused(r) => Some((
                AnswerKey::DependentGate {
                    ssn_hash: dependent_ssn_hash(&d.ssn),
                    gate: r.gate,
                },
                crate::tax::dependent_gates::entry(r.gate).prompt_text(ri, params),
                RefuseReason::DependentGateRefused { row, gate: r.gate },
                r.exit,
                r.rule,
            )),
            DependentVerdict::RefusedByQuestion(r) => Some((
                AnswerKey::Question(r.question),
                crate::tax::questions::FORM_QUESTIONS
                    .iter()
                    .find(|q| q.id == r.question)
                    .map_or(std::borrow::Cow::Borrowed(""), |q| q.prompt_text(ri)),
                RefuseReason::DependentRefusedByQuestion {
                    row,
                    question: r.question,
                },
                r.exit,
                r.rule,
            )),
            _ => None,
        };
        if let Some((item, prompt, reason, exit, rule)) = stopped {
            st.refusing.push(Refusing {
                item: Some(item),
                prompt,
                reason,
                exit: format!(
                    "dependent row {}: {exit} The rule the Form 1040 instructions send you to is \
                     {rule}.",
                    row + 1,
                ),
            });
        }
    }

    // ── 4. The declared-document / rows invariant (R3), the one panel item that is not a registry
    //       entry: it is a fact about the RETURN, not about a question. ────────────────────────────
    for row in DocumentRow::ALL {
        if let Some(r) = census_row_invariant(ri, *row) {
            st.refusing.push(r);
        }
    }

    // ── 5. ★★★ **R6 / T8 — 1040 LINE 19, a forgo the filer cannot answer away.** ─────────────────
    //
    //    The row-(7) credit boxes are computed and PRINTED (R6), and the amount they entitle the
    //    filer to is computed on Schedule 8812, which btctax does not file — so line 19 stays the
    //    filer's own blank. `Advisory::CtcOdcOmitted` already says so on the report; this puts the
    //    same fact in the panel, WITH ITS SIZE, while the return is being authored.
    //
    // ★ Counted from the flowchart's own verdict, not from the dependent count: a row with NEITHER
    //   box forgoes nothing, and printing it in the size would overstate what the filer is losing.
    let with_a_credit_box = (0..ri.header.dependents.len())
        .filter(|row| {
            crate::tax::dependent_gates::credit_column(ri, *row)
                != crate::tax::dependent_gates::CreditColumn::Neither
        })
        .count();
    if with_a_credit_box > 0 {
        st.not_computed.push(NotComputed {
            what: "child tax credit / credit for other dependents",
            count: with_a_credit_box,
            // ★★ The CHILD-credit ceiling, which is the LARGER of the two and the one a filer needs
            //    to hear. Deliberately not a blended figure: "up to $X each" is a ceiling, and the
            //    ceiling for a household is the child credit.
            each: params.map(|p| p.child_tax_credit_per_child),
            sized_from: YEAR_PACKAGE,
        });
    }

    // ── 6. ★★★ **THE COMMIT SCREEN ITSELF — so `refusing` is TOTAL over `RefuseReason`.** ────────
    //
    //    T12 seam review I-1. Sections 1–4 above build `refusing` out of the registries, and they
    //    can produce exactly FIVE of the 126 `RefuseReason` variants. `screen_inputs` — the gate
    //    `input_form_store::commit` runs before it writes the vault — raises all of them. So a
    //    return stopped by any of the other 121 (`HomeSaleNotComputed` is the one T9 shipped and
    //    §4.4 asks `report` to print) had an EMPTY refusing list, the commit modal showed nothing at
    //    all, and with nothing else open `panel_lines` printed *"no answer refuses"* — an
    //    affirmatively false sentence beside a gate that refuses the write, against the promise
    //    `LIMITATIONS.md` makes that a filer meets a refusal *"while you are still authoring rather
    //    than at commit"* (J-32).
    //
    // ★★ **Derived, not extended.** Five hand-written variants standing beside a set of 126 is this
    //    arc's dominant defect shape (FOLLOWUPS FR-99), and the fix for that shape is never to add
    //    the missing entries — it is to run the deciding function. This is the same move T12 made
    //    for the home sale itself: one decider, two readers.
    //
    // ★★ **The VALUE tier, not the unanswered tier**, and that is the panel's own vocabulary rather
    //    than a convenience: `Refusing` is documented as *"an answer you have ALREADY GIVEN that
    //    stops the return"*, and an unanswered class-(A) declaration is already listed — with its
    //    own heading, its own prompt and its own cursor target — as `blocking`. Running
    //    `unanswered_refuses: true` here would print every blocking item a second time under a
    //    heading that told the filer their answer was wrong when they have not given one.
    //
    // ★ It reports the FIRST refusal only, because `screen_inputs_tiered` is a first-refusal-wins
    //   gate by construction (`SPEC_input_form.md` §7 forbids refactoring it to collect them all —
    //   its tier precedence is load-bearing). That is enough for the guarantee this exists to make:
    //   *the panel's refusing list is empty only if the screen raises nothing at this tier.*
    if let Some(r) = crate::tax::return_refuse::screen_param_free(ri) {
        // ★ Already modelled by the walk above ⇒ keep the registry's version: it carries the panel
        //   ITEM (the answer to go and change), which a screen refusal does not have.
        if !st.refusing.iter().any(|x| x.reason == r.reason) {
            st.refusing.push(Refusing {
                item: None,
                prompt: std::borrow::Cow::Borrowed(""),
                reason: r.reason,
                exit: r.detail,
            });
        }
    }

    st
}

/// Does this skippable hold a value of its own kind?
fn skippable_has_value(s: &crate::tax::questions::SkippableQuestion, ri: &ReturnInputs) -> bool {
    use crate::tax::questions::SkippableKind;
    match s.kind {
        SkippableKind::YesNo => (s.get_bool)(ri).is_some(),
        SkippableKind::Date => (s.get_date)(ri).is_some(),
        SkippableKind::Choice(_) => (s.get_choice)(ri).is_some(),
    }
}

/// ★★ The §63(f) add-on a skipped box forgoes — **from the year's package, never guessed**.
///
/// `None` for every other skippable, and `None` on a params-less year: R12 wants the size blank
/// rather than invented, because a figure with no package behind it is the answered-ness defect
/// wearing a dollar sign.
fn forgo_size(
    id: SkippableId,
    ri: &ReturnInputs,
    params: Option<&FullReturnParams>,
) -> Option<Usd> {
    let p = params?;
    matches!(
        id,
        SkippableId::BlindTaxpayer
            | SkippableId::BlindSpouse
            | SkippableId::DobTaxpayer
            | SkippableId::DobSpouse
    )
    .then(|| {
        // The same selection `advisories.rs` makes for `per_box`, and for the same reason: §63(f)'s
        // addition is the married rate for every married-rate status.
        if matches!(
            ri.filing_status,
            FilingStatus::Mfj | FilingStatus::Mfs | FilingStatus::Qss
        ) {
            p.std_aged_blind_married
        } else {
            p.std_aged_blind_unmarried
        }
    })
}

/// ★★★ **An ANSWERED question whose answer refuses** — derived from the question, never re-decided.
///
/// Today the census rows are the whole population: a `Some(true)` on a type §2.2 excludes, or on one
/// whose screen is task T5. Returns the refusal and the exit sentence the filer is shown.
fn refusal_of_answer(
    ri: &ReturnInputs,
    id: QuestionId,
    answer: bool,
) -> Option<(RefuseReason, String)> {
    let row = row_of_question(id)?;
    if !answer || !crate::tax::document_census::row_is_live(ri, row) {
        return None;
    }
    let exit = row.exit_sentence()?;
    Some((
        RefuseReason::DocumentTypeUnsupported { kind: row },
        format!(
            "you answered that you received one or more {}. {exit}",
            row.designation()
        ),
    ))
}

/// ★★★ **The declared-document / rows invariant** (R3), as a panel item.
///
/// The two states a *question* cannot express, because they are facts about the return rather than
/// about an answer: a declared document with nothing transcribed, and a "no" beside transcribed rows.
/// Both are `refusing` — the filer must act before commit, and both have an in-form exit.
fn census_row_invariant(ri: &ReturnInputs, row: DocumentRow) -> Option<Refusing> {
    use crate::tax::document_census::{declared_rows, requires_transcription, row_is_live};
    if !row_is_live(ri, row) {
        return None;
    }
    let rows = declared_rows(ri, row)?;
    let item = AnswerKey::Question(row.question_id());
    let doc = row.designation();
    match ri.documents.get(row) {
        // ★ Gated on `requires_transcription`, exactly as `screen_document_census` is — the panel
        //   and the screen must name the same refusals or the panel is a second, drifting rule.
        Some(true)
            if rows == 0 && requires_transcription(row) && row.exit_sentence().is_none() =>
        {
            Some(Refusing {
            item: Some(item),
            prompt: std::borrow::Cow::Borrowed(row.prompt()),
            reason: RefuseReason::DocumentDeclaredNotTranscribed { kind: row },
                exit: format!(
                    "you declared one or more {doc} and none is transcribed — enter the document, \
                     or change the answer to \"no\""
                ),
            })
        }
        Some(false) if rows > 0 => Some(Refusing {
            item: Some(item),
            prompt: std::borrow::Cow::Borrowed(row.prompt()),
            reason: RefuseReason::DocumentCensusContradicted { kind: row },
            exit: format!(
                "you answered NO to {doc} and this return carries {rows} transcribed row(s) of it — \
                 remove the row(s), or change the answer to \"yes\""
            ),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::document_census::DocumentRow;
    use crate::tax::provenance::{prompt_hash, record_answer, AnswerRecord, AnswerState};
    use crate::tax::return_inputs::{Owner, W2};
    use rust_decimal_macros::dec;
    use time::macros::date;

    /// A return whose every live class-(A) declaration is answered, so each test below perturbs
    /// exactly one thing.
    fn answered_single() -> ReturnInputs {
        let mut ri = ReturnInputs {
            tax_year: 2024,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        ri
    }

    fn params() -> FullReturnParams {
        crate::tax::testonly::ty2024_params()
    }

    /// ★★★ **R6 / T8 — THE LINE-19 FORGO IS SIZED FROM THE YEAR'S PACKAGE, AND BLANK WITHOUT ONE.**
    ///
    /// The row-(7) credit boxes are printed and the amount is computed on Schedule 8812, which btctax
    /// does not file — so the panel says so, with a size where the package supplies one. Both halves
    /// are asserted, because a size that is always present would pass on a hardcoded figure and a
    /// size that is always absent would pass on a panel that never learned to read the package.
    #[test]
    fn the_line_19_forgo_is_sized_from_the_package_and_blank_without_one() {
        let mut ri = answered_single();
        ri.header.dependents = vec![crate::tax::return_inputs::Dependent {
            name: "Kid Example".into(),
            ssn: "000-00-1111".into(),
            relationship: "Daughter".into(),
            ..Default::default()
        }];
        crate::tax::testonly::answer_all_live_declarations(&mut ri);
        assert_eq!(
            crate::tax::dependent_gates::credit_column(&ri, 0),
            crate::tax::dependent_gates::CreditColumn::ChildTaxCredit,
            "the fixture's row really does carry a credit box"
        );

        // ── with the package ──
        let p = params();
        let st = interview_state_with_params(&ri, &p);
        assert_eq!(st.not_computed.len(), 1);
        let n = &st.not_computed[0];
        assert_eq!(n.count, 1);
        assert_eq!(n.each, Some(p.child_tax_credit_per_child));
        let line = n.line();
        assert!(line.contains("1 dependent"), "{line}");
        assert!(
            line.contains("2000"),
            "the size is the year's own figure: {line}"
        );

        // ── without it ──
        let st = interview_state(&ri);
        assert_eq!(st.not_computed.len(), 1);
        assert_eq!(
            st.not_computed[0].each, None,
            "no package ⇒ no invented figure"
        );
        let line = st.not_computed[0].line();
        assert!(!line.contains('$'), "no figure at all, not a zero: {line}");
        assert!(
            line.contains(YEAR_PACKAGE),
            "and it names what it waits on: {line}"
        );

        // ── a dependent with NEITHER box forgoes nothing, so nothing is listed ──
        let mut none = ri.clone();
        none.header.dependents[0].tin_issued_by_due_date = Some(false);
        assert_eq!(
            crate::tax::dependent_gates::credit_column(&none, 0),
            crate::tax::dependent_gates::CreditColumn::Neither
        );
        assert!(
            interview_state_with_params(&none, &p)
                .not_computed
                .is_empty(),
            "a row the instruction gives NO credit box forgoes no credit"
        );

        // ── …and a return with no dependents at all says nothing. ──
        assert!(interview_state_with_params(&answered_single(), &p)
            .not_computed
            .is_empty());
    }

    /// ★★★ **R12's headline kill: N unanswered live class-(A) items are listed as N blocking items
    ///     in ONE call.** `screen_inputs` returns the FIRST refusal by design (its tiers are
    ///     semantic); the panel must return them all, or a filer answers one question per run.
    #[test]
    fn n_unanswered_live_declarations_are_listed_as_n_blocking_in_one_call() {
        let base = answered_single();
        assert!(
            interview_state(&base).blocking.is_empty(),
            "the baseline must have nothing blocking, or every count below is meaningless"
        );

        // Blank four live declarations, of two different kinds (two ordinary, two census rows).
        let mut ri = base.clone();
        ri.header.can_be_claimed_as_dependent_taxpayer = None;
        ri.foreign_accounts = None;
        ri.documents.set(DocumentRow::W2, None);
        ri.documents.set(DocumentRow::K1, None);

        let st = interview_state(&ri);
        assert_eq!(
            st.blocking.len(),
            4,
            "four blanked declarations must yield FOUR blocking items in one call, got: {:#?}",
            st.blocking.iter().map(|b| &b.item).collect::<Vec<_>>()
        );
        for b in &st.blocking {
            assert!(!b.prompt.is_empty(), "every blocking item shows its prompt");
            assert!(
                !b.accounts_for.is_empty(),
                "every blocking item says what the answer accounts for"
            );
        }
        assert!(!st.is_committable(), "blocking ⇒ not committable");
    }

    /// ★★★ **`Declined` is FORGOING, marked, and never BLOCKING; `Given` removes it.**
    ///
    /// This is the reader `provenance.rs` promised for `AnswerState::Declined` — until now it was
    /// written by `income answer` and read by nothing in production.
    #[test]
    fn a_declined_skippable_forgoes_marked_and_never_blocks_and_given_removes_it() {
        let mut ri = answered_single();
        let sk = SKIPPABLE_QUESTIONS
            .iter()
            .find(|s| s.id == SkippableId::BlindTaxpayer)
            .expect("the blindness skippable is in the registry");
        record_answer(
            &mut ri,
            AnswerKey::Skippable(SkippableId::BlindTaxpayer),
            sk.prompt,
            date!(2026 - 09 - 01),
            AnswerState::Declined,
        );

        let st = interview_state(&ri);
        let f = st
            .forgoing
            .iter()
            .find(|f| f.item == AnswerKey::Skippable(SkippableId::BlindTaxpayer))
            .expect("a declined skippable is listed in forgoing");
        assert!(f.declined, "…and it is MARKED as declined");
        assert!(
            !st.blocking
                .iter()
                .any(|b| b.item == AnswerKey::Skippable(SkippableId::BlindTaxpayer)),
            "a declined class-(B) prompt must NEVER be blocking"
        );

        // `Given` removes it — answering the question, not merely re-recording it.
        (sk.set_bool)(&mut ri, false);
        record_answer(
            &mut ri,
            AnswerKey::Skippable(SkippableId::BlindTaxpayer),
            sk.prompt,
            date!(2026 - 09 - 02),
            AnswerState::Given,
        );
        assert!(
            !interview_state(&ri)
                .forgoing
                .iter()
                .any(|f| f.item == AnswerKey::Skippable(SkippableId::BlindTaxpayer)),
            "only `Given` removes a forgoing item"
        );
    }

    /// ★★ **The forgo SIZE is present when the year's package exists and absent when it does not.**
    #[test]
    fn a_forgo_size_appears_only_with_a_year_package() {
        let ri = answered_single();
        let blind = |st: &InterviewState| -> Option<Usd> {
            st.forgoing
                .iter()
                .find(|f| f.item == AnswerKey::Skippable(SkippableId::BlindTaxpayer))
                .expect("blindness is live and unanswered on this fixture")
                .size
        };
        assert_eq!(
            blind(&interview_state(&ri)),
            None,
            "a params-less year must show NO size rather than a guessed one"
        );
        assert_eq!(
            blind(&interview_state_with_params(&ri, &params())),
            Some(dec!(1950)),
            "with the package in hand the §63(f) unmarried add-on is named"
        );
    }

    /// ★★★ **A hash-mismatched record is BLOCKING (class A) / FORGOING (class B), with the
    ///     changed-wording reason.** An answer given under earlier words does not stand under later
    ///     ones (R10.3).
    #[test]
    fn a_hash_mismatched_record_reappears_with_the_changed_wording_reason() {
        let mut ri = answered_single();
        // Class (A): the record exists, hashed against words that are not today's.
        ri.answer_log.insert(
            AnswerKey::Question(QuestionId::ForeignTrust),
            AnswerRecord {
                answered_on: date!(2026 - 09 - 01),
                prompt_hash: prompt_hash("the words this question used to carry"),
                state: AnswerState::Given,
            },
        );
        // Class (B): the same, on a skippable that HOLDS a value (so it would otherwise be counted).
        ri.header.taxpayer.blind = Some(false);
        ri.answer_log.insert(
            AnswerKey::Skippable(SkippableId::BlindTaxpayer),
            AnswerRecord {
                answered_on: date!(2026 - 09 - 01),
                prompt_hash: prompt_hash("older words"),
                state: AnswerState::Given,
            },
        );

        let st = interview_state(&ri);
        let b = st
            .blocking
            .iter()
            .find(|b| b.item == AnswerKey::Question(QuestionId::ForeignTrust))
            .expect("a class-(A) record under earlier words BLOCKS");
        assert_eq!(b.reason, WORDING_CHANGED_REASON);
        let f = st
            .forgoing
            .iter()
            .find(|f| f.item == AnswerKey::Skippable(SkippableId::BlindTaxpayer))
            .expect("a class-(B) record under earlier words FORGOES");
        assert_eq!(f.benefit, WORDING_CHANGED_REASON);
        assert!(!f.declined, "changed wording is not a decline");
    }

    /// ★★★ **A census `Some(true)` on an unsupported type is in `refusing` BEFORE commit**, with its
    ///     exit sentence — the filer meets it while authoring, not at the end (J-32).
    #[test]
    fn an_unsupported_census_yes_is_refusing_with_its_exit_before_commit() {
        let mut ri = answered_single();
        ri.documents.set(DocumentRow::K1, Some(true));
        let st = interview_state(&ri);
        let r = st
            .refusing
            .iter()
            .find(|r| r.item == Some(AnswerKey::Question(QuestionId::DocK1)))
            .expect("a Schedule K-1 declared must be listed as refusing");
        assert_eq!(
            r.reason,
            RefuseReason::DocumentTypeUnsupported {
                kind: DocumentRow::K1
            }
        );
        assert!(
            r.exit.contains("A preparer is the exit for this year."),
            "the panel must carry §2.2's own exit sentence: {}",
            r.exit
        );
        assert!(!st.is_committable(), "a refusing item blocks commit");
    }

    /// ★★ **The declared-document / rows invariant reaches the panel too** — both directions.
    #[test]
    fn the_declared_document_and_rows_invariant_is_a_panel_item() {
        // Declared, nothing transcribed.
        let mut a = answered_single();
        a.documents.set(DocumentRow::W2, Some(true));
        assert!(
            interview_state(&a).refusing.iter().any(|r| r.reason
                == RefuseReason::DocumentDeclaredNotTranscribed {
                    kind: DocumentRow::W2
                }),
            "a declared W-2 with nothing transcribed must be listed"
        );

        // "No" beside a transcribed row.
        let mut b = answered_single();
        b.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(1000),
            ..Default::default()
        }];
        b.documents.set(DocumentRow::W2, Some(false));
        assert!(
            interview_state(&b).refusing.iter().any(|r| r.reason
                == RefuseReason::DocumentCensusContradicted {
                    kind: DocumentRow::W2
                }),
            "a \"no\" beside a transcribed W-2 must be listed"
        );

        // …and the coherent pair is silent, or the two reds above are one blanket refusal.
        let mut ok = b.clone();
        ok.documents.set(DocumentRow::W2, Some(true));
        assert!(interview_state(&ok).refusing.is_empty());

        // ★★★ **THE PANEL AND THE SCREEN NAME THE SAME REFUSALS.** The rows invariant is two
        //     predicates since the T3 seam fold, and a panel that kept the OLD one-predicate rule
        //     would tell a truthful 1099-B filer their return is refusing while `screen_inputs`
        //     files it — a second rule drifting from the first. `requires_transcription` gates both.
        let mut on_8949 = answered_single();
        on_8949.documents.set(DocumentRow::B1099, Some(true));
        assert!(
            interview_state(&on_8949).refusing.is_empty(),
            "a Form 1099-B whose transactions are all on Form 8949 is a CORRECT return with zero \
             summary rows — the panel must not refuse what the screen files: {:?}",
            interview_state(&on_8949).refusing
        );
        // ★★★ **T5 FLIPPED THE 1099-G HALF, so the panel must flip with it.** Box 2 now HAS a
        //     field, `requires_transcription(G1099)` is `true`, and a declared 1099-G with nothing
        //     transcribed is once again "nothing ever populated it". The point of asserting it here
        //     is the same as before: the panel and the screen must name the SAME refusals, so a
        //     panel still excusing the 1099-G would be the second rule drifting from the first.
        let mut box2_only = answered_single();
        box2_only.documents.set(DocumentRow::G1099, Some(true));
        assert!(
            interview_state(&box2_only).refusing.iter().any(|r| r.reason
                == RefuseReason::DocumentDeclaredNotTranscribed {
                    kind: DocumentRow::G1099
                }),
            "since T5 the 1099-G has a box-2 field, so a declared one with no row must be listed \
             as refusing: {:?}",
            interview_state(&box2_only).refusing
        );
        // …and the row the filer then enters, box 2 alone, leaves the panel naming only the
        // §111(a) gate — the question that decides whether the refund is income at all.
        let mut transcribed = box2_only.clone();
        transcribed.g_1099 = vec![crate::tax::return_inputs::Form1099G {
            payer: "State of Example".into(),
            box2_state_refund: dec!(900),
            ..Default::default()
        }];
        assert!(
            interview_state(&transcribed).refusing.is_empty(),
            "a transcribed box-2 row is not a REFUSING state — the prior-year-itemized question is \
             blocking, not refusing: {:?}",
            interview_state(&transcribed).refusing
        );
        assert!(
            interview_state(&transcribed).blocking.iter().any(|b| b.item
                == crate::tax::provenance::AnswerKey::Question(
                    crate::tax::questions::QuestionId::ItemizedPriorYear
                )),
            "…and it must be listed as BLOCKING: {:?}",
            interview_state(&transcribed).blocking
        );
    }

    /// ★★★ **A params-gated prompt WAITS rather than blocks, and becomes blocking the moment the
    ///     package lands.**
    ///
    /// [`PARAMS_GATED_PROMPTS`] is empty today (its first occupant is T7's
    /// `gross_income_under_limit`), so the occupant is PLANTED here: an instrument nobody has
    /// watched discriminating is not an instrument (harness B1), and an empty table would make this
    /// state pass vacuously forever.
    #[test]
    fn a_params_gated_prompt_waits_instead_of_blocking_and_moves_when_the_package_lands() {
        let mut ri = answered_single();
        ri.foreign_accounts = None; // unanswered ⇒ it would BLOCK

        let waiting_table: &[(QuestionId, &str)] =
            &[(QuestionId::ForeignAccounts, "the TY2026 year package")];
        let st = interview_state_with(&ri, None, waiting_table);
        assert!(
            st.waiting.iter().any(
                |w| w.item == AnswerKey::Question(QuestionId::ForeignAccounts)
                    && w.waiting_on == "the TY2026 year package"
            ),
            "a prompt that cannot be STATED yet waits, and names the package"
        );
        assert!(
            !st.blocking
                .iter()
                .any(|b| b.item == AnswerKey::Question(QuestionId::ForeignAccounts)),
            "★ and it must NOT also block — that is the whole distinction"
        );

        // The package lands (the entry leaves the table) ⇒ the same unanswered question blocks.
        let st2 = interview_state_with(&ri, None, &[]);
        assert!(st2.waiting.is_empty());
        assert!(
            st2.blocking
                .iter()
                .any(|b| b.item == AnswerKey::Question(QuestionId::ForeignAccounts)),
            "with the package in hand it becomes blocking"
        );
    }

    /// The counters are a partition: every registry entry lands in exactly one bucket, so a walk
    /// that silently dropped entries would show up as a shrinking total.
    #[test]
    fn every_registry_entry_lands_in_exactly_one_bucket() {
        let ri = answered_single();
        let st = interview_state(&ri);
        // The census invariant can add `refusing` items that are NOT registry entries, so count the
        // registry-derived buckets only.
        let registry_refusing = st
            .refusing
            .iter()
            .filter(|r| {
                !matches!(
                    r.reason,
                    RefuseReason::DocumentDeclaredNotTranscribed { .. }
                        | RefuseReason::DocumentCensusContradicted { .. }
                )
            })
            .count();
        let total = st.blocking.len()
            + st.forgoing.len()
            + registry_refusing
            + st.waiting.len()
            + st.answered
            + st.not_live;
        assert_eq!(
            total,
            FORM_QUESTIONS.len() + SKIPPABLE_QUESTIONS.len(),
            "every registry entry must be counted exactly once"
        );
    }
}
