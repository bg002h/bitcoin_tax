//! `income answer` (D-8) — the ONLY in-app path to the fail-loud tri-states and the dates of birth.
//!
//! **Why it must exist.** The D-8 migration's recovery story was "just re-import one TOML line" — which
//! assumes the user still HAS the TOML. The spec tells them to delete it (plaintext hygiene), `income
//! show` emits masked JSON and so cannot regenerate it, and `set-pii` prompts for secrets only. Without
//! `answer`, a TOML-less user faces a permanently-refusing year and no way to answer a single boolean: a
//! wall, landing hardest on the people who did exactly what the spec told them to.
//!
//! **What it deliberately does NOT own: secrets.** SSNs and the IP PIN belong to `set-pii`, which is
//! no-echo. `answer` is an ordinary echoing prompt — routing a secret through it would print a crown jewel
//! into terminal scrollback.
use crate::{return_inputs, CliError, Session};
use btctax_core::tax::dependent_gates::{
    gate_is_answered, DependentGateQuestion, GateKind, DEPENDENT_GATES,
};
use btctax_core::tax::provenance::{
    answer_status, dependent_ssn_hash, record_answer, AnswerKey, AnswerState, AnswerStatus,
    DependentGate,
};
use btctax_core::tax::questions::{
    FormQuestion, QuestionId, SkippableKind, SkippableQuestion, FORM_QUESTIONS, SKIPPABLE_QUESTIONS,
};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::tables::FullReturnParams;
use btctax_store::Passphrase;
use std::io::Write;
use std::path::Path;

/// One thing `income answer` asks: a MANDATORY declaration (from the [`FORM_QUESTIONS`] registry — a bare
/// Enter with nothing on file is refused, never accepted as an answer) or a SKIPPABLE prompt (from the
/// core [`SKIPPABLE_QUESTIONS`] registry — a bare Enter leaves `None`, forgoing the benefit lawfully).
pub enum Ask {
    Declaration(&'static FormQuestion),
    Skippable(&'static SkippableQuestion),
    /// ★★★ **T7 / R6 — one gate of *Who Qualifies as Your Dependent*, on one dependent ROW.**
    ///
    /// A third variant rather than a per-row `FormQuestion`, because the answer's identity is the
    /// row's salted SSN hash and the prompt's liveness depends on the row — neither of which the
    /// return-level registry can express.
    DependentGate {
        gate: &'static DependentGateQuestion,
        /// The row's index in `header.dependents`, resolved at enumeration time.
        row: usize,
    },
}

impl Ask {
    /// The registry `QuestionId` if this is a declaration — for tests that assert WHICH questions are live.
    pub fn declaration_id(&self) -> Option<QuestionId> {
        match self {
            Ask::Declaration(q) => Some(q.id),
            Ask::Skippable(_) | Ask::DependentGate { .. } => None,
        }
    }
    /// The gate identity and row if this is a dependent gate — for tests that assert WHICH gates are live.
    pub fn dependent_gate(&self) -> Option<(DependentGate, usize)> {
        match self {
            Ask::DependentGate { gate, row } => Some((gate.gate, *row)),
            Ask::Declaration(_) | Ask::Skippable(_) => None,
        }
    }
    /// Whether a bare Enter with nothing on file is a legitimate outcome. True for skippables (DOBs), false
    /// for declarations — silence on a declaration is exactly what D-8 forbids, and a dependent gate is a
    /// declaration (class A) on a row.
    pub fn is_skippable(&self) -> bool {
        matches!(self, Ask::Skippable(_))
    }
}

/// ★ EXACTLY the questions this return needs — the MANDATORY declarations DERIVED from the registry (so the
/// prompt scope IS the refusal scope, by identity: the no-brick property is now true by construction, not by
/// a second hand-written list that can drift, r1 M-1), then the skippable DOBs.
///
/// **Every live declaration is asked in ONE pass** — including ones already answered, whose current value
/// is offered as the default. A blob the screen refuses cannot be stored, so answering only *some*
/// declarations would leave the return refused and unstorable; asking everything at once prevents that
/// deadlock. The spouse DOB prompt is gated on `header.spouse.is_some()` (r3 I-7).
pub fn live_questions(ri: &ReturnInputs) -> Vec<Ask> {
    live_questions_with(ri, None)
}

/// [`live_questions`] on a year whose PACKAGE is in hand.
///
/// ★★★ **The only difference is R6's params-quoting gate.** `gross_income_under_limit`'s prompt must
/// QUOTE the year's §152(d)(1)(B) figure, so on a params-less year it is *waiting for the year
/// package* (R12) and must not be put to the filer at all — a prompt that cannot state the figure
/// asks them to derive it. Two entry points rather than an `Option` at every call site, exactly as
/// `interview_state` / `interview_state_with_params` are.
pub fn live_questions_with(ri: &ReturnInputs, params: Option<&FullReturnParams>) -> Vec<Ask> {
    // ★★★ **DOCUMENT-FIRST** (T3 seam review, M5). A real interview asks the shoebox first: what
    //     did you receive? The registry ARRAY cannot say so — `decl_tristate!`/`census_tristate!`
    //     couple to a literal index, so the eighteen census rows had to be APPENDED at 17..=34 —
    //     and a filer therefore answered eight gate declarations, the 4,673-character residual
    //     attestation among them, before being asked *"Did you receive one or more Form W-2?"*.
    //     Worse, five of that attestation's own limbs (1099-R, SSA-1099, K-1, Schedule E rental,
    //     W-2G) are then asked again one at a time, twelve questions later.
    //
    //     ★ So the ORDER is fixed here, in what the filer is shown, and the array is left exactly as
    //     it is: the index coupling is untouched, and `FORM_QUESTIONS`' own order still decides
    //     everything inside each group. Correctness never depended on this — `screen_document_census`
    //     runs before the `OtherOutOfScopeIncome` refusal either way — so this is a journey fix, and
    //     it is a stable partition, never a sort.
    let live = || FORM_QUESTIONS.iter().filter(|q| (q.live)(ri));
    // ★★★ SEAM REVIEW M-3 — R3's DOOR QUESTIONS GROUP WITH THE CENSUS, because they ARE the
    //     census's own follow-ups: each is live only *because* a census row was answered, and a
    //     filer who has just said "no Form W-2" should be asked "did you have wages anyway?" next —
    //     not after all nineteen skippables (the DOBs, the blindness pair, the sales-tax election).
    //
    // ★★★ **DERIVED, never a hand-list**, and that is the whole point: a question belongs to this
    //     group iff it is live NOW and would NOT be live with every census row blanked. So a door
    //     question added later joins the group with no edit here, and one whose liveness stops
    //     depending on the census leaves it — the same rule the census's own `row_of_question`
    //     membership follows. (`ItemizedPriorYear` is the interesting case: it groups here when the
    //     refund was DECLARED, and does not when a transcribed 1099-G box 2 makes it live, which is
    //     exactly the dependency each of those returns actually has.)
    let census_blanked = {
        let mut probe = ri.clone();
        for row in btctax_core::tax::document_census::DocumentRow::ALL {
            probe.documents.set(*row, None);
        }
        probe
    };
    let is_census = |q: &&'static FormQuestion| {
        btctax_core::tax::document_census::row_of_question(q.id).is_some()
    };
    let is_census_followup =
        |q: &&'static FormQuestion| !is_census(q) && !(q.live)(&census_blanked);
    let mut asks: Vec<Ask> = live()
        .filter(is_census)
        .chain(live().filter(is_census_followup))
        .chain(live().filter(|q| !is_census(q) && !is_census_followup(q)))
        .map(Ask::Declaration)
        .collect();
    // ★ P9 §2.2 class-(B) skippables — DERIVED from the core [`SKIPPABLE_QUESTIONS`] registry (the DOBs, the
    // blindness pair, and the §164(b)(5) sales-tax election), each gated by its own `live` predicate so the
    // prompt scope tracks the WRITE scope (a `set` on an absent spouse / Schedule A is silently discarded).
    // ★★★ **T7 / R6 — THE DEPENDENT GATES, one row at a time, before the skippables.**
    //
    //     Grouped by ROW rather than by gate, because that is the unit the filer holds in their
    //     head: one child, one chain of the instruction's own questions. Within a row the order is
    //     `DEPENDENT_GATES`' own, which is the flowchart's.
    //
    // ★ Liveness is the WALK's (`DependentGateQuestion::live`), so a Step 4 gate appears for a row
    //   exactly when Step 1 sent that row to Step 4 — and the sweep below re-asks `live_questions`
    //   after every answer, which is how a block opens.
    for row in 0..ri.header.dependents.len() {
        asks.extend(
            DEPENDENT_GATES
                .iter()
                .filter(|g| g.live(ri, row))
                // R12's `waiting`: not asked until the year's package can state the figure.
                .filter(|g| !g.needs_params() || params.is_some())
                .map(move |g| Ask::DependentGate { gate: g, row }),
        );
    }
    asks.extend(
        SKIPPABLE_QUESTIONS
            .iter()
            .filter(|s| (s.live)(ri))
            .map(Ask::Skippable),
    );
    asks
}

/// ★★★ **THE SWEEP BOUND** — a guard against a liveness CYCLE (A opens B opens A), which no
/// registry entry has. It is module-level so the tests can assert against the REAL number rather
/// than a second copy of it: T7's dependent chain opens one flowchart STEP per sweep, and a
/// liveness that opened one GATE per sweep would exceed this and fail loudly rather than silently
/// ending a filer's session mid-interview.
pub const MAX_SWEEPS: usize = 8;

/// ★★★ **THE KEY ONE ASK IS TRACKED UNDER *WITHIN ONE SESSION*** — which is NOT the key its answer
///     is stored under, and the difference is the whole of seam review I-2.
///
/// The stored [`AnswerKey::DependentGate`] is the row's salted SSN hash, because a record must
/// survive `remove` on the row above it (R10.3 / T1: *"delete row 0 and an index-keyed record would
/// move one child's diligence onto another"*). The **asked** set has the opposite requirement: it
/// exists so one session does not put the same question twice, and it must therefore distinguish two
/// ROWS even when they hash the same.
///
/// ★★★ **They did hash the same, and it starved a row.** Two rows with a blank `ssn` — or the same
///     digits typed twice — share one `dependent_ssn_hash`. Within a round both asks were still
///     collected, so the collision was invisible; ACROSS rounds it was not. A gate row 0 was asked
///     in round *n* filtered row 1's identical key out of round *n+1*, so it was never put to the
///     filer, stayed `None`, and `screen_dependent_gates` refused it forever while the command
///     exited reporting nothing left to ask. Fail-closed on the claim, fail-OPEN on the interview.
///
/// ★ The index is safe *here* and nowhere else: this set is per-session and is never stored, so no
///   `remove` can happen underneath it. Both halves of the fix are needed — `screen_dependent_gates`
///   now refuses a blank or duplicated identity outright, and this key means a session cannot starve
///   a row even before that refusal is reached.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum AskedKey {
    Question(QuestionId),
    Skippable(btctax_core::tax::questions::SkippableId),
    /// The ROW INDEX, deliberately — see the type's own doc.
    DependentGate {
        row: usize,
        gate: DependentGate,
    },
}

/// ★★★ **FR-109 — WHICH of the live questions are put to the filer.**
///
/// Liveness (`(q.live)(&ri)`, `walk_dependent(ri, row).demands(gate)`) says which questions *apply
/// to this return*. It says nothing about whether the filer has already answered them, and until
/// FR-109 nothing else did either: every live question was asked every session — measured on the
/// 2026-09-07 journey walk at **33 already-answered declarations plus 15 dependent gates in one
/// run**, all of them re-confirmed with a bare Enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AskScope {
    /// The default: ask what the answer log says still needs asking. See [`needs_asking`].
    #[default]
    StillNeeded,
    /// `--re-answer`: put EVERY live question again, including ones already on file. The behaviour
    /// every session had before FR-109, kept because re-reading the whole set is a legitimate thing
    /// to want — a filer walking their return end to end before signing it, or one who is not sure
    /// what a past self answered.
    Every,
}

/// The two knobs [`answer_return_inputs`] takes beyond the year and the streams.
///
/// ★ A struct rather than two more parameters: the function already takes seven, and clippy's
///   `too_many_arguments` is on at `-D warnings`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnswerOptions {
    /// Discard a work-in-progress tax-inputs DRAFT for this year that holds an interview.
    pub discard_draft: bool,
    /// Which live questions to ask.
    pub scope: AskScope,
}

/// The `answer_log` key one [`Ask`] is stored under — `None` for a dependent gate whose row has
/// gone (the walk will not offer one, so this is defence, not a path).
fn answer_key_of(ri: &ReturnInputs, ask: &Ask) -> Option<AnswerKey> {
    Some(match ask {
        Ask::Declaration(q) => AnswerKey::Question(q.id),
        Ask::Skippable(sk) => AnswerKey::Skippable(sk.id),
        Ask::DependentGate { gate, row } => AnswerKey::DependentGate {
            ssn_hash: dependent_ssn_hash(&ri.header.dependents.get(*row)?.ssn),
            gate: gate.gate,
        },
    })
}

/// ★★★ **FR-109 — does this live question still need putting to the filer?**
///
/// The line is drawn by [`AnswerStatus`], whose variants already draw it, and the answer is a
/// conjunction of two different facts because **the log and the leaf can disagree**:
///
/// | status | verdict | why |
/// |---|---|---|
/// | `NeverAsked` | **ASK** | no record. Either the leaf is empty (the ordinary unanswered case) or it holds a value that arrived by `income import` — and an imported value has no prompt hash, so R10.3's re-ask on reworded question could never fire for it. Asking stamps a record, so this CONVERGES: asked once after an import, skipped every session after. |
/// | `WordingChanged` | **ASK** | the variant's own doc is *"Treated as UNANSWERED everywhere"*. The filer answered a different sentence, so there is no answer to the question now on the screen. Consistency, not an exception. |
/// | `Given` | **SKIP** | answered, under these words. |
/// | `Declined` | **SKIP** | *asked and deliberately passed over* — a recorded decision, and R12 still lists the benefit as forgone, so the filer is not left uninformed by the silence. |
///
/// ★★ **…and then the LEAF, for a class-(A) ask only.** A `Given` record standing over an empty
///    leaf is a record of testimony the return does not carry. Skipping there would be the one
///    outcome worse than re-asking: the question is class (A), so `screen_inputs` refuses the
///    commit, R12 lists it as blocking, and the command that exists to fix it would refuse to ask
///    — a **brick**, and the same shape as the T7 seam review's starved dependent row. The rule
///    there is the rule here: *fail-closed on the claim, fail-OPEN on the interview.* A class-(B)
///    skippable takes no leaf test, because for it an empty leaf IS a lawful answer (`Declined`).
#[must_use]
pub fn needs_asking(ri: &ReturnInputs, ask: &Ask) -> bool {
    let Some(key) = answer_key_of(ri, ask) else {
        return true;
    };
    match answer_status(ri, &key) {
        AnswerStatus::NeverAsked | AnswerStatus::WordingChanged => true,
        AnswerStatus::Given | AnswerStatus::Declined => match ask {
            Ask::Declaration(q) => (q.get)(ri).is_none(),
            Ask::DependentGate { gate, row } => !ri
                .header
                .dependents
                .get(*row)
                .is_some_and(|d| gate_is_answered(d, gate.gate)),
            Ask::Skippable(_) => false,
        },
    }
}

fn asked_key_of(a: &Ask) -> AskedKey {
    match a {
        Ask::Declaration(q) => AskedKey::Question(q.id),
        Ask::Skippable(sk) => AskedKey::Skippable(sk.id),
        Ask::DependentGate { gate, row } => AskedKey::DependentGate {
            row: *row,
            gate: gate.gate,
        },
    }
}

/// Parse one yes/no reply. `""` (a bare Enter) means "keep `default`", and is only an ANSWER when there
/// already is one — otherwise the caller must re-ask, because silence is exactly what D-8 forbids.
pub fn parse_yes_no(line: &str, default: Option<bool>) -> Option<bool> {
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        "" => default,
        _ => None,
    }
}

/// ★★★ FR-29 — parse one reply to a [`SkippableKind::Choice`] question, into one of `options`.
///
/// **Accepted, case-insensitively and after trimming:** `y` / `yes` ⇒ `Yes`; `n` / `no` ⇒ `No`;
/// `?` / `cannot know` / `cannotknow` / `cant know` / `unknown` ⇒ `CannotKnow`. Anything else is
/// `None`, and the caller re-asks — the same three-way behaviour [`parse_yes_no`] already has,
/// extended by one variant rather than replaced.
///
/// ★ The returned token is taken from `options` itself, never minted here, so a token the registry
/// does not offer can never be stored — the direction is fail-closed in both halves.
pub fn parse_parent_alive_choice(
    line: &str,
    options: &'static [&'static str],
) -> Option<&'static str> {
    let want = match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => "Yes",
        "n" | "no" => "No",
        "?" | "cannot know" | "cannotknow" | "cant know" | "can't know" | "unknown" => "CannotKnow",
        _ => return None,
    };
    options.iter().copied().find(|o| *o == want)
}

/// Parse one date reply. `Ok(None)` = the user SKIPPED (a bare Enter) — a legitimate outcome for a DOB.
pub fn parse_date(line: &str) -> Result<Option<time::Date>, String> {
    let t = line.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    time::Date::parse(t, fmt)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// `income answer --year N` — ask every live question, then store.
///
/// **Refuses on a year with no row**: only `income import` creates one. Answering questions about a return
/// that does not exist would materialize a near-empty blob, which then takes PRECEDENCE over the user's
/// `tax-profile` (the resolver ranks `ReturnInputs` first) — silently replacing a working profile with an
/// empty return. A missing row is a mistake to report, not a shape to invent.
/// ★★★ **R10.3 — what a skippable's ANSWER STATE is, after the prompt has been dealt with.**
///
/// The rule is mechanical and reads off the value, not off the keystroke: a live skippable that ends
/// the prompt holding a value was **`Given`**; one that ends it holding nothing was offered and passed
/// over, which is **`Declined`**. A bare Enter is the same keystroke in both cases and must not be the
/// same record — declining is provenance (*asked, refused*), and R12 keeps a `Declined` benefit in the
/// *forgoing* list precisely because of it.
fn skippable_state(sk: &SkippableQuestion, ri: &ReturnInputs) -> AnswerState {
    let answered = match sk.kind {
        SkippableKind::Date => (sk.get_date)(ri).is_some(),
        SkippableKind::YesNo => (sk.get_bool)(ri).is_some(),
        SkippableKind::Choice(_) => (sk.get_choice)(ri).is_some(),
    };
    if answered {
        AnswerState::Given
    } else {
        AnswerState::Declined
    }
}

/// ★★★ **R12 / §4.2 — THE ANSWER PANEL, rendered.** `income answer` prints it BEFORE the first
/// question and AFTER the last.
///
/// **Why both ends.** Before, so the filer sees the whole shape of what is open rather than meeting
/// it one prompt at a time; after, so they see what their answers left — a forgone benefit, a
/// refusing answer, or a prompt still waiting on the year's package. The second print is what makes
/// the session's OUTCOME visible: a run that answers everything and still cannot commit must say so
/// while the filer is at the keyboard, not at export.
///
/// ★ No progress bar and no "N of M" (R15): the panel lists ITEMS, and a count of items is not a
/// measure of how far through anything the filer is.
pub fn write_panel(
    out: &mut impl Write,
    st: &btctax_core::tax::interview_state::InterviewState,
    when: &str,
) -> std::io::Result<()> {
    // ★ The leading blank line is the CLI's own framing (it separates the panel from whatever the
    //   command printed above it) and is deliberately not part of [`panel_lines`], which a pane
    //   draws inside a bordered block where a stray first row would be a hole.
    writeln!(out)?;
    for line in panel_lines(st, when) {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

/// ★★★ **T12 / R12 — THE PANEL'S LINES, BUILT ONCE FOR EVERY SURFACE THAT SHOWS IT.**
///
/// `income answer` writes them to stdout, the TUI draws them in a pane, and the commit modal takes
/// two of the sections verbatim. The T12 rule is the one every task before it learned: **render,
/// never recompute.** A second renderer beside this one would be a second chance to word the same
/// forgo differently — the exact shape `step0::tui_lines` exists to prevent for the ledger panel,
/// one surface up.
///
/// The first line is the heading (`── The answer panel (when) ──`, blank-prefixed by the caller's
/// own framing rather than here, so a pane can draw it without a stray leading row).
#[must_use]
pub fn panel_lines(
    st: &btctax_core::tax::interview_state::InterviewState,
    when: &str,
) -> Vec<String> {
    let mut out = vec![format!("── The answer panel ({when}) ──")];
    if st.open_items() == 0 {
        out.push(
            "  nothing is open: every live question is answered, nothing is forgone, and no answer \
             refuses."
                .to_string(),
        );
        // ★★★ **AND THE BOUNDARY, STATED** (T12 fold, seam review I-1). `interview_state` now ends
        //     by running the commit screen's VALUE tier, so the sentence above is true of every
        //     `RefuseReason` that tier raises rather than of the five the registries happened to
        //     model. What it still cannot run is the tier that needs the year's TAX TABLE — the
        //     walk holds `FullReturnParams` at most, never a `TaxTable` — so those rules are met at
        //     commit and nowhere earlier. A surface that asserted completeness it does not have is
        //     the finding; saying which half it has is the fix, and the sentence rides WITH the
        //     claim so neither can be edited away without the other.
        out.push(
            "  (the commit screen runs a few further checks that compare an amount against a \
             figure in the year's tax table; those run at commit, and only there.)"
                .to_string(),
        );
    }
    if !st.blocking.is_empty() {
        out.push(format!(
            "  BLOCKING ({}) — commit waits on these:",
            st.blocking.len()
        ));
        for b in &st.blocking {
            out.push(format!("    • {} [{}]", b.prompt, b.reason));
        }
        // ★★★ **FR-106 (journey walk finding #5) — THE COUNT CAN GROW, AND NOW IT SAYS SO.**
        //
        //     The walk read *"BLOCKING (36)"*, worked through the list, and met two questions that
        //     had not been in it — opened mid-session by its own earlier census answers. That is
        //     the M-3 sweep design working exactly as intended (`live_questions` is re-derived
        //     after every answer, because a question whose liveness depends on an answer cannot be
        //     known before it), and the return is correct either way. What was missing is that the
        //     number a filer reads to gauge *how much is left* is a snapshot of NOW, and nothing
        //     told them so — which reads from the driver's seat as the interview changing its mind
        //     about how many questions there are.
        //
        // ★ It states the RULE and enumerates no mechanism. A list of "a census No opens…, a
        //   dependent's answer opens…" would be correct today and one registry entry away from
        //   being a lie, which is the FR-99 shape; the rule covers every entry that exists and
        //   every entry that will.
        out.push(
            "  ↑ this is what is live NOW, and answering can make it GROW: some questions \
             become live only once an earlier answer opens them, so the list is re-derived after \
             every answer rather than fixed at the start."
                .to_string(),
        );
    }
    if !st.refusing.is_empty() {
        out.extend(refusing_lines(st));
    }
    if !st.forgoing.is_empty() {
        out.extend(forgoing_lines(st));
    }
    if !st.waiting.is_empty() {
        out.push(format!(
            "  WAITING ({}) — these cannot be asked until a year package arrives:",
            st.waiting.len()
        ));
        for w in &st.waiting {
            out.push(format!("    • {} [waiting on {}]", w.prompt, w.waiting_on));
        }
    }
    // ★★★ R6 / T8 — benefits the FORM computes on a schedule btctax does not file. There is nothing
    //     to answer, which is why they are their own heading rather than a `FORGOING` row: telling a
    //     filer to "answer" 1040 line 19 would send them looking for a question that does not exist.
    if !st.not_computed.is_empty() {
        out.extend(not_computed_lines(st));
    }
    out.push(format!(
        "  ({} answered, {} not applicable to this return)",
        st.answered, st.not_live
    ));
    out
}

/// ★★★ **T12 / R12 — THE REFUSING LIST, one rendering for the panel AND the commit modal (J-32).**
///
/// The commit modal shows it because J-32 is a filer who answered `k1 = Yes` at the census, saw
/// nothing blocking, and committed: the refusal is already decided and the modal is the last screen
/// before the write. Sharing the lines with the panel is what keeps the two from describing one
/// refusal in two ways.
#[must_use]
pub fn refusing_lines(st: &btctax_core::tax::interview_state::InterviewState) -> Vec<String> {
    let mut out = vec![format!(
        "  REFUSING ({}) — an answer already given that stops the return:",
        st.refusing.len()
    )];
    for r in &st.refusing {
        out.push(format!("    • {}", r.exit));
    }
    out
}

/// ★★★ **T12 fold / R12 — THE *NOT COMPUTED* LIST, one rendering for the panel AND the manifest.**
///
/// ★★ **The heading is the actionable half, and it is why this is a function** (T12 seam review
///    M-1). The manifest's FORGONE block used to append `st.not_computed` under
///    *"FORGONE — benefits you are lawfully entitled to skip, and did"* using `n.line()` alone, so
///    on PAPER — the artifact the filer follows while assembling the envelope — 1040 line 19 read
///    as something they chose to skip, and the sentence telling them *the credit boxes on your
///    return are printed, the amount is yours to enter* was gone. A benefit the FORM computes
///    elsewhere is not a forgo: there is nothing to answer, and the instruction is the whole point
///    of listing it.
#[must_use]
pub fn not_computed_lines(st: &btctax_core::tax::interview_state::InterviewState) -> Vec<String> {
    let mut out = vec![format!(
        "  NOT COMPUTED ({}) — btctax does not file the schedule these are figured on; the \
         credit boxes on your return are printed, the amount is yours to enter:",
        st.not_computed.len()
    )];
    for n in &st.not_computed {
        out.push(format!("    • {}", n.line()));
    }
    out
}

/// ★★★ **T12 / R12 — THE FORGOING LIST, one rendering for the panel, the commit modal (J-12) AND
/// the packet manifest (J-15).**
///
/// ★★ **A `Declined` benefit is STILL FORGONE and is still listed, marked *(declined)*.** Dropping
///    it from the list exactly when the forgo becomes FINAL is backwards — declining is provenance
///    (asked, refused), not absence. Only `Given` removes an item.
///
/// ★ The size is printed only where the year's package could compute it: a figure invented from no
///   package is worse than a gap the filer can see.
#[must_use]
pub fn forgoing_lines(st: &btctax_core::tax::interview_state::InterviewState) -> Vec<String> {
    let mut out = vec![format!(
        "  FORGOING ({}) — lawful to skip; each one costs YOU, not the Treasury:",
        st.forgoing.len()
    )];
    for f in &st.forgoing {
        let mark = if f.declined { " (declined)" } else { "" };
        // ★ Plain `$N` rather than `advisories::fmt_usd`, which is `pub(crate)` to core.
        let size = f
            .size
            .map_or_else(String::new, |s| format!(" — up to ${s}"));
        out.push(format!("    • {}{mark}{size}", f.prompt));
    }
    out
}

/// The R12 panel for `ri`, with the year's package where there is one.
///
/// ★ Two entry points in core (`interview_state` / `interview_state_with_params`) exist precisely so
///   the *waiting* / *blocking* split is decided by whether the caller HOLDS the package — and this
///   is the one production caller that knows.
fn panel_state(
    ri: &ReturnInputs,
    params: Option<&FullReturnParams>,
) -> btctax_core::tax::interview_state::InterviewState {
    match params {
        Some(p) => btctax_core::tax::interview_state::interview_state_with_params(ri, p),
        None => btctax_core::tax::interview_state::interview_state(ri),
    }
}

/// ★★★ **T4 / `SPEC_interview.md` R11 — WHERE `income answer` WRITES.**
///
/// Two stores, and which one is the answer target is a fact about the year, not a flag:
///
/// - a **committed row** exists ⇒ answer it, exactly as before, and supersede the year's WIP draft
///   on the write (the §6.2 coherence rule, now with T4's confirmation);
/// - **no committed row but a WIP draft** ⇒ answer the DRAFT. R11: the draft is the Sep–Dec store
///   for a year whose package has not arrived, and on such a year nothing else can hold answers.
enum AnswerTarget {
    Committed {
        ri: ReturnInputs,
        coherence: crate::input_form_store::DraftCoherence,
    },
    Draft(ReturnInputs),
}

/// Resolve [`AnswerTarget`], raising every refusal a write could raise BEFORE a question is asked.
///
/// ★ M-1 order, kept: the parked-draft refusal comes first, because a parked year has no committed
///   row and the generic "no inputs" message would otherwise shadow its remedy.
fn answer_target(
    sess: &crate::Session,
    year: i32,
    discard_draft: bool,
) -> Result<AnswerTarget, CliError> {
    use crate::input_form_store::{self, Loaded};
    if input_form_store::parked_flag(sess.conn(), year)? == Some(true) {
        return Err(CliError::ParkedDraftBlocksWrite { year });
    }
    if let Some(ri) = return_inputs::get(sess.conn(), year)? {
        // The committed row wins; the draft is what this write supersedes.
        let coherence = input_form_store::coherence_check(sess.conn(), year, discard_draft)?;
        return Ok(AnswerTarget::Committed { ri, coherence });
    }
    // ★★★ R11 — no committed row. `load` applies the §6.1 precedence and the §6.3 stale split
    //     (a stale WIP draft holding an interview REFUSES here rather than being discarded, T4).
    let (loaded, stale) = input_form_store::load(sess.conn(), year)?;
    if let Some(note) = stale {
        eprintln!("note: {note}");
    }
    match loaded {
        Loaded::Draft { ri, parked: false } => Ok(AnswerTarget::Draft(ri)),
        // Unreachable in practice (parked was refused above), and it must not become an answer
        // target if it ever is: a parked return is testimony the filer WITHDREW.
        Loaded::Draft { parked: true, .. } => Err(CliError::ParkedDraftBlocksWrite { year }),
        Loaded::Committed(ri) => Ok(AnswerTarget::Committed {
            ri,
            coherence: crate::input_form_store::DraftCoherence::Absent,
        }),
        Loaded::Fresh => Err(CliError::Usage(format!(
            "no full-return inputs and no draft for tax year {year} — `income answer` fills in the \
             questions on an EXISTING return. Create one with `btctax income import --year {year} \
             --file <toml>`, or start one in the tax-inputs form (which saves a draft even on a \
             year whose package has not arrived)."
        ))),
    }
}

pub fn answer_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    now: time::Date,
    input: &mut impl std::io::BufRead,
    out: &mut impl Write,
    opts: AnswerOptions,
) -> Result<(), CliError> {
    let AnswerOptions {
        discard_draft,
        scope,
    } = opts;
    let mut s = Session::open(vault, pp)?;
    let target = answer_target(&s, year, discard_draft)?;
    let (mut ri, into_draft, coherence) = match target {
        AnswerTarget::Committed { ri, coherence } => (ri, false, coherence),
        AnswerTarget::Draft(ri) => (ri, true, crate::input_form_store::DraftCoherence::Absent),
    };

    // ★★★ **SEAM REVIEW I-2 — A ROW WITH NO IDENTITY, OR A SHARED ONE, IS REFUSED BEFORE THE
    //     FIRST QUESTION.**
    //
    //     A dependent gate's answer is stored under `dependent_ssn_hash(row.ssn)`. Two rows with a
    //     blank `ssn` — or the same digits typed twice — are therefore ONE key: whatever the second
    //     row is asked overwrites the first row's record, and R10.3's log says *"the blank
    //     dependent answered this"* rather than naming a person. *"A diligence record that lies is
    //     worse than none."*
    //
    //     ★★ So this command refuses rather than asking. There is no SSN QUESTION to put to the
    //        filer — a dependent's name, SSN and relationship are identity fields the row is
    //        CREATED with (`income import`, or the tax-inputs form's Dependents section), not
    //        registry questions — so "ask for the SSN first" has nowhere to ask from, and inventing
    //        a gate for it would widen `DependentGate::ALL` for a field no §152 test reads. The
    //        exit named is the one that exists, and `screen_inputs` refuses the identical state at
    //        the commit gate (`DependentIdentityUnanswered` / `DependentSsnDuplicated`), so this is
    //        the same rule met earlier rather than a second one.
    {
        let digits = |ssn: &str| -> String { ssn.chars().filter(char::is_ascii_digit).collect() };
        let named = |row: usize, d: &btctax_core::tax::return_inputs::Dependent| {
            if d.name.trim().is_empty() {
                format!("row {}", row + 1)
            } else {
                format!("row {} ({})", row + 1, d.name.trim())
            }
        };
        for (row, d) in ri.header.dependents.iter().enumerate() {
            if digits(&d.ssn).is_empty() {
                return Err(CliError::Usage(format!(
                    "dependent {} has no Social Security number, so this year's answers about that \
                     person would have no owner: btctax files each dependent's §152 answers under \
                     their SSN. Enter it in the tax-inputs form's Dependents section (or in the \
                     TOML you import), or remove the row — then run `btctax income answer` again. \
                     Nothing was stored.",
                    named(row, d)
                )));
            }
            if let Some(first) = ri.header.dependents[..row]
                .iter()
                .position(|o| digits(&o.ssn) == digits(&d.ssn))
            {
                return Err(CliError::Usage(format!(
                    "dependent {} and dependent {} carry the same Social Security number, so one \
                     row's answers would be filed as the other's. Correct the number, or remove \
                     the duplicate row, then run `btctax income answer` again. Nothing was stored.",
                    named(first, &ri.header.dependents[first]),
                    named(row, d)
                )));
            }
        }
    }

    // ★★★ **T7 / R6 — THE YEAR'S PACKAGE, for the WORDS of one gate and nothing else.**
    //
    //     `gross_income_under_limit`'s prompt must QUOTE the year's §152(d)(1)(B) figure. On a year
    //     with no package it is *waiting* (R12) and is not asked at all; with one, the figure is in
    //     the sentence the filer answers and in the hash `record_answer` stores. Nothing else on
    //     this path reads the package.
    let params: Option<FullReturnParams> = {
        use btctax_core::tax::tables::FullReturnTables;
        btctax_adapters::BundledFullReturnTables::load()
            .full_return_for(year)
            .cloned()
    };

    // ★ r3 NIT-2 — the questions say "in this tax year" but the registry prompts are `&'static str` and
    // cannot interpolate the year; a one-line banner anchors them so the filer need not hold it in their head.
    writeln!(out, "Answering full-return questions for tax year {year}:")?;
    // ★★★ **FR-109 — SAY WHICH QUESTIONS ARE COMING, and the sentence must track the CODE.**
    //
    //     FR-105 put a sentence here saying *"every question that applies to this return is asked
    //     again each time — this command does not skip the ones already on file"*. That was true of
    //     the command as it then stood and is **false of it now**, which is the FR-108 class in
    //     prose: a filer-facing sentence left standing over a mechanism that moved underneath it.
    //     So it is rewritten rather than kept, and it is written per SCOPE — a `--re-answer` run
    //     really does put everything again, and must say so.
    //
    // ★ The wording states the MECHANISM, not a motive, which is the rule FR-105 established: the
    //   skip is *"already answered, in the words it is asked in now"* (`answer_status`), never a
    //   claim about what the filer meant.
    writeln!(
        out,
        "{}",
        match scope {
            AskScope::StillNeeded =>
                "  (only what this return still needs is asked: a question already answered — in \
                 the words it is asked in now — is skipped. A value that arrived by `income \
                 import` is asked once, so that it gets a record. Press Enter to keep an answer \
                 shown, or re-run with `--re-answer` to be put through every question again.)",
            AskScope::Every =>
                "  (`--re-answer`: every question that applies to this return is put again, \
                 including the ones already answered. Press Enter to keep the answer shown.)",
        }
    )?;
    // ★★★ T4/R11 — THE YEAR GATE, stated before the first question: which of the two states this
    //     year has, in R11's own words, so a filer on a params-less year knows that authoring and
    //     saving work while computing and committing wait for the package.
    writeln!(
        out,
        "{}",
        crate::year_readiness::EntryStates::for_year(year, Some(&ri)).sentence()
    )?;
    if into_draft {
        writeln!(
            out,
            "  (answering the {year} DRAFT — this year has no committed return, so the answers are \
             saved to the input form's draft.)"
        )?;
    }

    // ★★★ **C-1 / R10.4 — THE PRIOR YEAR'S ROW, FOR THE `Durable` HINT AND NOTHING ELSE.**
    //
    //     `Durability::Durable` is *"the prior MAY be displayed, but it still requires the same
    //     explicit keystroke as a fresh ask: never Enter-to-accept, never pre-filled"*. The opener
    //     therefore seeds the date of birth BLANK and stamps `opened_from`; this read is what makes
    //     *displayed* possible. Nothing is written from it: the hint is text in a prompt, the filer
    //     types the date to confirm it, and a bare Enter leaves the value `None` — so
    //     `skippable_state` records `Declined`, which is the truthful outcome of skipping.
    //
    //     ★ A missing prior row is not an error: the hint is a convenience, and its absence costs
    //       the filer a lookup, never an answer.
    let prior_year_row = ri
        .opened_from
        .and_then(|n| return_inputs::get(s.conn(), n).ok().flatten());
    // ★★★ **R9 / T6 — STEP 0, BEFORE THE CENSUS.**
    //
    //     The ledger's status comes first because it is what a filer needs in hand while they answer
    //     the return's questions — which venue has no Form 1099-DA answer, which blocker still stops
    //     the commit they are working toward. **Document-first order is untouched:** Step 0 is
    //     STATUS, not a question set, so the first thing the filer is ASKED is still the shoebox.
    //
    // ★ A projection failure is not fatal here. The panel is a courtesy over the ledger, and a
    //   filer whose vault cannot project must still be able to answer their W-2 boxes — R9's
    //   "authoring proceeds in parallel with an unresolved ledger" is the whole point of the panel
    //   being a report rather than a gate.
    match s.project() {
        Ok((state, _)) => {
            let events = btctax_core::persistence::load_all(s.conn())?;
            // ★ (seam review I-1) the YEAR'S Form 1099-DA regime, joined the way every other
            //   surface joins it. `regime_for`, not `regime_or_refuse`: a status panel may not
            //   refuse, and `None` — no bundled record — is an honest unknown the panel states.
            let regime = crate::year_readiness::regime_for(year);
            let panel = crate::step0::step0_panel(&state, &events, Some(&ri), year, regime);
            crate::step0::write_step0(out, &panel)?;
            if !panel.standing_orders.is_empty() {
                writeln!(out, "  {}", crate::step0::VENUE_GRANULARITY_NOTE)?;
            }
        }
        Err(e) => writeln!(
            out,
            "
(the Step 0 ledger panel is unavailable: {e})"
        )?,
    }

    // ★★★ R12 / §4.2 — THE PANEL, BEFORE the first question.
    // ★ T7 — with the year's package where there is one, so a params-quoting gate is listed as
    //   BLOCKING (its words can be stated) rather than as *waiting* on a year that has arrived.
    write_panel(out, &panel_state(&ri, params.as_ref()), "before")?;

    // ★★★ **R3 / T5 — ASKING IS A SWEEP, NOT A SNAPSHOT.**
    //
    //     `live_questions` reads the return as it stands, and R3's document-less income door makes
    //     that insufficient in one pass: `w2_wages_without_w2` is live EXACTLY when the W-2 census
    //     row says `No`, so a filer who answers "no W-2 this year" makes a NEW question live in the
    //     middle of their own session. Asking from a single snapshot would end the run with that
    //     question unanswered, print it in the "after" panel as blocking, and leave the filer to run
    //     the command again to reach a question their previous answer created.
    //
    // ★ Each item is asked AT MOST ONCE per session, keyed by its `AnswerKey`, so a skippable the
    //   filer deliberately skipped is never re-asked in the same run; and the sweep stops as soon as
    //   a pass finds nothing new. The bound is a guard against a liveness CYCLE (A opens B opens A),
    //   which no registry entry has today — it fails loudly rather than looping forever.
    let mut asked: std::collections::BTreeSet<AskedKey> = std::collections::BTreeSet::new();

    let mut sweeps = 0usize;
    loop {
        let round: Vec<Ask> = live_questions_with(&ri, params.as_ref())
            .into_iter()
            .filter(|a| !asked.contains(&asked_key_of(a)))
            // ★★★ FR-109 — and of those, the ones that still NEED asking. `Every` is `--re-answer`,
            //     which is the pre-FR-109 behaviour verbatim.
            //
            // ★ The sweep still terminates: a skipped ask is not inserted into `asked`, but nothing
            //   this loop does can turn a `Given` back into a `NeverAsked`, so the same question is
            //   filtered out again on the next pass and `round` empties. The one direction that
            //   DOES change mid-session is toward asking MORE — an answer given now can reword a
            //   question that quotes it (R10.4), and the next sweep picks that up, which is exactly
            //   right.
            .filter(|a| scope == AskScope::Every || needs_asking(&ri, a))
            .collect();
        if round.is_empty() {
            break;
        }
        sweeps += 1;
        if sweeps > MAX_SWEEPS {
            return Err(CliError::Usage(
                "the question set did not settle: answering one question kept making another live. \
                 This is a liveness cycle in the registry, not something you can answer your way \
                 out of — nothing was stored"
                    .into(),
            ));
        }
        for ask in round {
            // ★★★ SEAM REVIEW M-3 — LIVENESS IS RE-CHECKED IMMEDIATELY BEFORE ASKING, not once per
            //     sweep. `round` is a snapshot, and an answer given EARLIER IN THIS ROUND can kill a
            //     question later in it: flip `state_refund_without_1099g` to `n` and the §111(a)
            //     gate `ItemizedPriorYear` dies, yet the snapshot would still put it to the filer
            //     and `record_answer` would write an answer to a question nobody is asking. The
            //     stored answer is harmless today only because the refusal that reads it is itself
            //     liveness-gated — which is a second guarantee holding this one up, not a reason.
            //
            // ★ It is NOT marked asked: a question that died here may legitimately come back to
            //   life on a later sweep (the filer changes the answer that killed it), and the sweep
            //   loop recomputes `live_questions` each pass, so nothing can spin.
            let still_live = match &ask {
                Ask::Declaration(q) => (q.live)(&ri),
                Ask::Skippable(s) => (s.live)(&ri),
                // ★ A dependent gate's liveness is the WALK's, and an answer given earlier in this
                //   same round can move a row onto another branch — so it is re-checked here like
                //   every other ask, and a row that has been REMOVED simply reads as not live.
                Ask::DependentGate { gate, row } => gate.live(&ri, *row),
            };
            if !still_live {
                continue;
            }
            asked.insert(asked_key_of(&ask));
            match ask {
                // A MANDATORY declaration — silence with nothing on file is refused, never accepted (D-8).
                Ask::Declaration(q) => {
                    let cur = (q.get)(&ri);
                    // ★★★ R8 / T9 — THE §163(h)(3)(B) CEILING, BESIDE THE DECLARATION IT INFORMS.
                    //
                    //     R8: the aggregate box-2 check is *"displayed as a warning beside the
                    //     existing `MortgageWithinDebtLimit` declaration, which stays the filer's
                    //     testimony"*. The question used to be asked with NO FIGURE — the filer was
                    //     made to add up their own balances while btctax was holding box 2.
                    //
                    // ★★ DISPLAY CHROME, printed BEFORE the prompt and never folded into it. T7's
                    //    C-1 is the reason: `record_answer` hashes the words put to the filer, so a
                    //    computed figure inside the prompt would change the hash whenever a balance
                    //    changed and re-ask a question nobody's answer had gone stale on.
                    if q.id == QuestionId::MortgageWithinDebtLimit {
                        if let Some(w) = btctax_core::tax::transcription_warnings::acquisition_debt_ceiling_warning(
                            &ri,
                            params.as_ref().map(|p| p.acquisition_debt_ceiling),
                        ) {
                            writeln!(out, "  warning · {w}")?;
                        }
                    }
                    // ★★★ R10.4 — the words PUT TO THE FILER, rendered from the return. For the carried
                    //     filing status that sentence QUOTES the status, and it is what `record_answer`
                    //     hashes below, so editing the status changes the hash and R10.3's re-ask rule
                    //     returns this question to unanswered on its own.
                    let prompt = q.prompt_text(&ri).into_owned();
                    loop {
                        let shown = match cur {
                            Some(true) => "y/n, currently y",
                            Some(false) => "y/n, currently n",
                            None => "y/n",
                        };
                        write!(out, "{prompt} [{shown}]: ")?;
                        out.flush()?;
                        let mut line = String::new();
                        if input.read_line(&mut line)? == 0 {
                            return Err(CliError::Usage(
                            "input ended before every question was answered — nothing was stored"
                                .into(),
                        ));
                        }
                        match parse_yes_no(&line, cur) {
                            Some(v) => {
                                (q.set)(&mut ri, v);
                                // ★★★ R10.3 — THE ONE WRITER, reached from the keyboard. The form
                                //     engine's `apply` reaches the same function with the same `now`, so
                                //     the same answer produces a byte-identical record on either surface.
                                //     A class-(A) declaration has no lawful decline, so it is always
                                //     `Given` — the loop cannot exit without a value.
                                record_answer(
                                    &mut ri,
                                    AnswerKey::Question(q.id),
                                    &prompt,
                                    now,
                                    AnswerState::Given,
                                );
                                break;
                            }
                            // ★ No default and no answer ⇒ ASK AGAIN. Accepting silence here would reintroduce
                            // D-8 through the front door.
                            None => writeln!(out, "  please answer y or n")?,
                        }
                    }
                }
                // A SKIPPABLE prompt — a bare Enter KEEPS whatever is on file (which may be `None`, forgoing the
                // benefit; the matching advisory then tells the filer). Two value shapes, branched by `kind()`.
                Ask::Skippable(sk) => match sk.kind {
                    SkippableKind::Date => {
                        let cur = (sk.get_date)(&ri);
                        // ★ C-1 — the HINT: year N's date for THIS skippable, read through the
                        //   skippable's own accessor (never a hand-list of the two DOBs), shown only
                        //   where this year has no answer yet. Typing it is a fresh answer; skipping it
                        //   declines. Nothing is pre-filled.
                        let hint = cur.is_none().then_some(()).and_then(|()| {
                            let prior = prior_year_row.as_ref()?;
                            let d = (sk.get_date)(prior)?;
                            Some(format!(
                                "; TY{n}'s return gave {d} — type it to confirm",
                                n = prior.tax_year
                            ))
                        });
                        loop {
                            let shown = cur.map_or_else(|| "none".to_string(), |d| d.to_string());
                            write!(
                                out,
                                "{} [{}{}; Enter to skip]: ",
                                sk.prompt,
                                shown,
                                hint.as_deref().unwrap_or_default()
                            )?;
                            out.flush()?;
                            let mut line = String::new();
                            if input.read_line(&mut line)? == 0 {
                                return Err(CliError::Usage(
                                "input ended before every question was answered — nothing was stored".into(),
                            ));
                            }
                            match parse_date(&line) {
                                Ok(None) => break,
                                Ok(Some(d)) => {
                                    (sk.set_date)(&mut ri, d);
                                    break;
                                }
                                Err(e) => writeln!(out, "  not a date (YYYY-MM-DD): {e}")?,
                            }
                        }
                    }
                    SkippableKind::YesNo => {
                        let cur = (sk.get_bool)(&ri);
                        loop {
                            let shown = match cur {
                                Some(true) => "y/n, currently y",
                                Some(false) => "y/n, currently n",
                                None => "y/n",
                            };
                            write!(out, "{} [{}; Enter to skip]: ", sk.prompt, shown)?;
                            out.flush()?;
                            let mut line = String::new();
                            if input.read_line(&mut line)? == 0 {
                                return Err(CliError::Usage(
                                "input ended before every question was answered — nothing was stored".into(),
                            ));
                            }
                            // ★ A bare Enter KEEPS whatever is on file (may be `None` ⇒ skip); only y/n sets a
                            // value; garbage re-asks. Silence is a legitimate outcome here — unlike a declaration.
                            if line.trim().is_empty() {
                                break;
                            }
                            match parse_yes_no(line.trim(), None) {
                                Some(v) => {
                                    (sk.set_bool)(&mut ri, v);
                                    break;
                                }
                                None => writeln!(out, "  please answer y or n, or Enter to skip")?,
                            }
                        }
                    }
                    // ★★★ FR-29 — the THIRD-ANSWER arm. `parse_enum`
                    //     (`btctax-input-form/src/parse.rs`) is an exact `options.contains(&raw)` with no
                    //     trimming and no case-folding, because there the options are stable tokens a
                    //     renderer presents as a closed choice. At the keyboard they are not: the prompt
                    //     tells the filer *"Answer YES, NO, or CANNOT KNOW"*, which is not the token
                    //     `"CannotKnow"`. **The parser accommodates the filer's words, never the other way
                    //     round** — the prompt's wording comes from the form and from plain English.
                    SkippableKind::Choice(options) => {
                        let cur = (sk.get_choice)(&ri);
                        loop {
                            let shown = cur.unwrap_or("unanswered");
                            write!(
                                out,
                                "{} [{}; currently {shown}; Enter to skip]: ",
                                sk.prompt,
                                options.join("/")
                            )?;
                            out.flush()?;
                            let mut line = String::new();
                            if input.read_line(&mut line)? == 0 {
                                return Err(CliError::Usage(
                                "input ended before every question was answered — nothing was stored".into(),
                            ));
                            }
                            // A bare Enter KEEPS whatever is on file (which may be `None` ⇒ still skipped).
                            if line.trim().is_empty() {
                                break;
                            }
                            match parse_parent_alive_choice(&line, options) {
                                Some(tok) => {
                                    (sk.set_choice)(&mut ri, tok);
                                    break;
                                }
                                // ★ Fail-closed: an unmatched string cannot become an answer, so the worst
                                //   case is a filer who is asked again.
                                None => writeln!(
                                    out,
                                    "  please answer yes, no, or \"cannot know\", or Enter to skip"
                                )?,
                            }
                        }
                    }
                },
                // ★★★ **T7 / R6 — ONE DEPENDENT GATE ON ONE ROW.** A class-(A) declaration, so
                //     silence with nothing on file is refused exactly as it is for a return-level
                //     one; the only differences are the row banner and the identity key.
                Ask::DependentGate { gate, row } => {
                    let who = ri
                        .header
                        .dependents
                        .get(row)
                        .map_or_else(String::new, |d| d.name.trim().to_string());
                    let banner = if who.is_empty() {
                        format!("dependent row {}", row + 1)
                    } else {
                        who
                    };
                    // ★★★ R10.4 — the words PUT TO THE FILER. For `gross_income_under_limit` those
                    //     words QUOTE the year's §152(d)(1)(B) figure, so the hash changes for free
                    //     when the figure moves and R10.3 re-asks it — which is exactly right: a
                    //     different limit is a different question.
                    //
                    // ★★★ **SEAM REVIEW C-1 — THE BANNER IS SHOWN, THE REGISTRY'S WORDS ARE
                    //     HASHED.** `record_answer` must hash what `current_prompt` resolves, and
                    //     that resolver is per-GATE (`provenance.rs`: *"the ssn_hash selects the
                    //     record and the gate selects the words"*) — it knows nothing of a row
                    //     banner. Folding the banner into the hashed string made EVERY gate read
                    //     `WordingChanged` the instant it was answered: the trailing panel listed
                    //     all fifteen as blocking, and the commit gate refused naming `income
                    //     answer` — the command that had just produced the state. A brick, not a
                    //     delay. This is D-1's class, closed one layer up by `answer_status` taking
                    //     the KEY rather than a prompt, and reintroduced here because display
                    //     chrome was put inside the comparand. The `Declaration` arm above already
                    //     has the right shape: `let prompt = q.prompt_text(&ri).into_owned();`.
                    let words = gate.prompt_text(&ri, params.as_ref());
                    let shown = format!("[{banner}] {words}");
                    match gate.kind {
                        GateKind::Date => {
                            let cur = ri.header.dependents[row].date_of_birth;
                            // ★★★ **SEAM REVIEW I-3 — THE `Durable` HINT, and nothing else.**
                            //
                            //     `DateOfBirth` is the one `Durability::Durable` gate, which is
                            //     *"the prior MAY be displayed, but it still requires the same
                            //     explicit keystroke as a fresh ask: never Enter-to-accept, never
                            //     pre-filled"*. The opener therefore seeds it BLANK and stamps
                            //     `opened_from`; this read is what makes *displayed* possible.
                            //
                            //     ★ Matched by SSN, never by row index — the rows may have been
                            //       reordered, added to or deleted between the two years, and the
                            //       whole point of the identity key is that it survives that.
                            //       Nothing is written from it: the hint is text in a prompt, the
                            //       filer types the date to confirm it, and a bare Enter leaves the
                            //       gate unanswered and blocking (a class-(A) date has no lawful
                            //       decline, so the loop re-asks rather than accepting silence).
                            let hint = cur.is_none().then_some(()).and_then(|()| {
                                let prior = prior_year_row.as_ref()?;
                                let mine = dependent_ssn_hash(&ri.header.dependents[row].ssn);
                                let d = prior
                                    .header
                                    .dependents
                                    .iter()
                                    .find(|p| dependent_ssn_hash(&p.ssn) == mine)?
                                    .date_of_birth?;
                                Some(format!(
                                    "; TY{n}'s return gave {d} — type it to confirm",
                                    n = prior.tax_year
                                ))
                            });
                            loop {
                                let current =
                                    cur.map_or_else(|| "none".to_string(), |d| d.to_string());
                                write!(
                                    out,
                                    "{shown} [YYYY-MM-DD; currently {current}{}]: ",
                                    hint.as_deref().unwrap_or_default()
                                )?;
                                out.flush()?;
                                let mut line = String::new();
                                if input.read_line(&mut line)? == 0 {
                                    return Err(CliError::Usage(
                                        "input ended before every question was answered — nothing was stored"
                                            .into(),
                                    ));
                                }
                                match parse_date(&line) {
                                    // ★ A bare Enter KEEPS what is on file — and with nothing on
                                    //   file it is silence on a class-(A) declaration, which D-8
                                    //   forbids, so it re-asks.
                                    Ok(None) => match cur {
                                        Some(_) => break,
                                        None => writeln!(
                                            out,
                                            "  a dependent row needs a date of birth: Step 1's age \
                                             test is computed from it and has no \"unknown\" edge"
                                        )?,
                                    },
                                    Ok(Some(d)) => {
                                        ri.header.dependents[row].date_of_birth = Some(d);
                                        break;
                                    }
                                    Err(e) => writeln!(out, "  not a date (YYYY-MM-DD): {e}")?,
                                }
                            }
                        }
                        GateKind::YesNo => {
                            let cur = (gate.get)(&ri.header.dependents[row]);
                            loop {
                                let current = match cur {
                                    Some(true) => "y/n, currently y",
                                    Some(false) => "y/n, currently n",
                                    None => "y/n",
                                };
                                write!(out, "{shown} [{current}]: ")?;
                                out.flush()?;
                                let mut line = String::new();
                                if input.read_line(&mut line)? == 0 {
                                    return Err(CliError::Usage(
                                        "input ended before every question was answered — nothing was stored"
                                            .into(),
                                    ));
                                }
                                match parse_yes_no(&line, cur) {
                                    Some(v) => {
                                        (gate.set)(&mut ri.header.dependents[row], v);
                                        break;
                                    }
                                    None => writeln!(out, "  please answer y or n")?,
                                }
                            }
                        }
                    }
                    // ★★★ R10.3 — the ONE writer, keyed by the row's IDENTITY. A class-(A) gate has
                    //     no lawful decline, so it is always `Given`: neither loop above can exit
                    //     without a value.
                    let key = AnswerKey::DependentGate {
                        ssn_hash: dependent_ssn_hash(&ri.header.dependents[row].ssn),
                        gate: gate.gate,
                    };
                    // ★ `words`, never `shown`: the banner names the row on screen, and the row
                    //   is already in the KEY. Hashing it would put display chrome inside the
                    //   comparand — seam review C-1.
                    record_answer(&mut ri, key, &words, now, AnswerState::Given);
                }
            }
            // ★★★ R10.3 — one record per prompt PUT TO THE FILER. Placed after the `Ask` match so the
            //     three SKIPPABLE shapes (date / yes-no / choice) share one recording site instead of
            //     three, and the state is read back off `ri` once each has written its value.
            //
            // ★ **What this placement does NOT enforce, stated because the previous wording claimed it
            //   did** (seam review N2). The declaration does not pass through this line — it records
            //   inside its own branch, at the `parse_yes_no` success arm, because a declaration's record
            //   is always `Given` and its loop cannot exit without a value. And this site is an `if let`,
            //   not a `match`: a THIRD `Ask` variant would compile here and record nothing. The `match`
            //   above IS exhaustive and would red on a new variant — that is the real net, and it is a
            //   compile error that forces someone to look at this line, not a guarantee that they will
            //   add a recording arm. Same shape as the classifier's carefully-stated `_` limit.
            if let Ask::Skippable(sk) = ask {
                let state = skippable_state(sk, &ri);
                record_answer(&mut ri, AnswerKey::Skippable(sk.id), sk.prompt, now, state);
            }
        }
    }

    // ★★★ **FR-109 — A SESSION THAT ASKED NOTHING SAYS SO.**
    //
    //     Before FR-109 this could not happen: something was always asked. Now the common case for
    //     a filer who imported a TOML and ran this yesterday is that nothing is due — and a command
    //     that prints two panels and no questions looks broken. It is not enough for the panel to
    //     say the return is complete; the filer asked to be *asked*, and the honest answer is that
    //     there was nothing to ask **and how to be asked anyway**.
    if asked.is_empty() {
        writeln!(
            out,
            "{}",
            match scope {
                AskScope::StillNeeded => format!(
                    "\nNothing to ask: every question this return needs is already answered, in \
                     the words it is asked in now. Run `btctax income answer --year {year} \
                     --re-answer` to be put through all of them again."
                ),
                AskScope::Every =>
                    "\nNothing to ask: no question in the registry applies to this return."
                        .to_string(),
            }
        )?;
    }

    // ★★★ R12 / §4.2 — THE PANEL, AFTER the last question. Printed BEFORE the write, so a filer
    //     whose session ends in a refusing answer sees it while they are still at the keyboard.
    write_panel(out, &panel_state(&ri, params.as_ref()), "after")?;

    if into_draft {
        // ★★★ R11 — the draft, never `return_inputs::set`. The draft is invisible to `resolve.rs`
        //     (`input_form_store.rs` header), so writing it carries none of the precedence-1 hazard
        //     that made `income answer` refuse an absent committed row in the first place;
        //     `return_inputs::get` still answers `None` after this.
        crate::input_form_store::save_draft(&mut s, year, &ri)?;
        return Ok(());
    }
    crate::input_form_store::coherence_clear(s.conn(), year, &coherence)?;
    return_inputs::set(s.conn(), year, &ri)?;
    s.save()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::questions::SkippableId;
    use btctax_core::tax::return_inputs::{Form1099Int, Person};
    use btctax_core::FilingStatus;
    use rust_decimal_macros::dec;

    fn single() -> ReturnInputs {
        ReturnInputs {
            // ★★ §G-15 — a fixture must state its year. `Default` gives `0` ("not stated"), under
            // which a year-scoped question is correctly NOT live — so a yearless fixture would
            // silently stop exercising `HasIncomeExclusion`. 2025 is the year every registry
            // question is live in; `a_ty2024_single_filer_is_not_asked_the_ty2025_magi_question`
            // below pins the other side of the gate.
            tax_year: 2025,
            filing_status: FilingStatus::Single,
            ..Default::default()
        }
    }
    fn with_spouse(mut ri: ReturnInputs) -> ReturnInputs {
        ri.header.spouse = Some(Person {
            first_name: "Pat".into(),
            last_name: "Doe".into(),
            ssn: "987654321".into(),
            ..Default::default()
        });
        ri
    }

    /// The registry `QuestionId`s asked for a return, in order.
    fn declaration_ids(ri: &ReturnInputs) -> Vec<QuestionId> {
        live_questions(ri)
            .iter()
            .filter_map(Ask::declaration_id)
            .collect()
    }
    fn has_spouse_dob(ri: &ReturnInputs) -> bool {
        live_questions(ri)
            .iter()
            .any(|a| matches!(a, Ask::Skippable(s) if s.id == SkippableId::DobSpouse))
    }

    /// ★★★ **§G-15 — the other side of the year gate, at the CLI.**
    ///
    /// The registry test proves `HasIncomeExclusion.live` is year-scoped; this proves the ANSWER LOOP
    /// honours it, which is what the filer actually experiences. Before §G-15 a TY2024 filer was
    /// asked a TY2025 modified-AGI question — harmless only because a bespoke neutrality proof
    /// existed for that one question, and Schedule 1-A Part IV has none.
    #[test]
    fn a_ty2024_single_filer_is_not_asked_the_ty2025_magi_question() {
        let ty2024 = ReturnInputs {
            tax_year: 2024,
            ..single()
        };
        let asked = declaration_ids(&ty2024);
        assert!(
            !asked.contains(&QuestionId::HasIncomeExclusion),
            "a TY2024 filer must not be asked a question whose subject TY2024 never reads; got {asked:?}"
        );
        // …and the year-agnostic declarations are still asked, so the gate narrowed exactly one thing.
        assert!(asked.contains(&QuestionId::ForeignAccounts));
        assert!(asked.contains(&QuestionId::DualStatusAlien));
    }

    /// A Single filer is asked the five unconditional declarations (dependent-taxpayer, both foreign
    /// questions, HSA activity, dual-status), the TY2025 MAGI question (§G-15 year-scoped it, and this
    /// fixture is TY2025), and — skippably — a DOB. Nothing about a spouse who does not
    /// exist, and (post-§2.9) the foreign questions appear even with no Schedule B.
    #[test]
    fn a_single_filer_is_asked_the_always_live_declarations_and_no_spouse_question() {
        assert_eq!(
            declaration_ids(&single()),
            vec![
                // ★★★ R3 / §5.1 — THE DOCUMENT CENSUS, ASKED FIRST (T3 seam review, M5). NINETEEN
                // of the twenty rows are live for every filer: a document type must be ANSWERED,
                // and "a filer cannot answer no to a category they were never shown". The only
                // missing one is `DocForm1098`, whose amount is collected today by a scalar — it
                // opens with T9, and until then a `No` on the row would contradict a figure the
                // filer already entered. ★ `DocForm1098e` was the other; T5 replaced its scalar
                // with `Form1098E` rows and the row opened.
                //
                // ★ They come first because a real interview asks the SHOEBOX first, and because
                // five of the residual attestation's own limbs (1099-R, SSA-1099, K-1, Schedule E
                // rental, W-2G) are these very rows asked again one at a time. The registry array is
                // unchanged and still 17..=34 (`decl_tristate!` couples to the index); the ORDER is
                // a stable partition inside `live_questions`.
                QuestionId::DocW2,
                QuestionId::DocInt1099,
                QuestionId::DocDiv1099,
                QuestionId::DocB1099,
                QuestionId::DocG1099,
                // ★ T5 — the 1098-E row opened when `Form1098E` replaced the student-loan scalar.
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
                // ★ T16 — the two HSA information returns. Appended at the END of `QuestionId::ALL`
                //   (indices 41 and 42), so they sort after the eighteen §5.1 rows in the census
                //   partition `live_questions` builds.
                QuestionId::DocSa1099,
                QuestionId::DocSa5498,
                // ── …then the gate declarations, in registry order, unchanged. ──────────────────
                QuestionId::DependentTaxpayer,
                QuestionId::ForeignAccounts,
                QuestionId::ForeignTrust,
                QuestionId::HsaActivity,
                QuestionId::DualStatusAlien,
                // §911/931/933 exclusion gate — always live, like DualStatusAlien: one yes/no every
                // filer can answer, and TY2025's SALT worksheet and Schedule 1-A both need it.
                QuestionId::HasIncomeExclusion,
                // ★★★ §G-22/B11 — the scope attestation, always live for EVERY filer by construction.
                // Every other declaration is scoped to the years or shapes that read it; this one is
                // read by nothing, because it exists precisely where there is no field for the income
                // it asks about. A liveness predicate could only guess whether the filer has some,
                // which is the guess it exists to refuse to make.
                QuestionId::OtherOutOfScopeIncome,
                // ★★★ Schedule D line 20 / Schedule A line 9 — ALWAYS LIVE, for exactly the reason
                // above. Line 20 prints on every both-gains Schedule D, and that routing comes from
                // the LEDGER, which `live` cannot see; any narrower predicate would be a guess about
                // whether the filer borrowed to invest. It reaches the $0-income household too, which
                // is the population the plan measured this defect on.
                QuestionId::FilingForm4952,
                // ★★★ R9 / T6 — Form 1040 page 1's DIGITAL ASSETS question. ALWAYS LIVE, and the
                // instruction says why in one sentence: *"You must answer the digital asset question
                // on Form 1040 whether or not you received a Form 1099-DA"*. It is asked of the
                // empty-vault filer too — the question is on the FORM, not on the ledger, and
                // scoping it to "btctax saw crypto" would be the circular liveness §2.9 records.
                QuestionId::DigitalAssetActivity,
                // ★★★ R8 / T9 — "did you sell your main home?", always live and NOT neutral: the
                //     Schedule D block's own answer is "You may not need to report the sale", and
                //     which branch a filer is on decides whether a Form 8949 belongs on the return.
                //     Its three tests are live only on a YES, and the Form 8396 gate only on a
                //     return that carries a Form 1098 or a line 8b row — so neither appears here.
                QuestionId::SoldMainHome,
            ]
        );
        assert!(!has_spouse_dob(&single()), "no spouse ⇒ no spouse DOB");

        // ★★★ THE PARTITION, stated as the property rather than only as this one vector: EVERY live
        //     census row precedes EVERY other live declaration, and neither group's internal order
        //     moved. A future registry entry inserted mid-array must not be able to slip a gate in
        //     front of the shoebox.
        let ids = declaration_ids(&single());
        let last_census = ids
            .iter()
            .rposition(|id| btctax_core::tax::document_census::row_of_question(*id).is_some())
            .expect("a Single filer is asked seventeen census rows");
        let first_gate = ids
            .iter()
            .position(|id| btctax_core::tax::document_census::row_of_question(*id).is_none())
            .expect("…and eight gate declarations");
        assert!(
            last_census < first_gate,
            "the document census must be asked BEFORE every other declaration: {ids:?}"
        );
        let registry_order = |id: QuestionId| {
            FORM_QUESTIONS
                .iter()
                .position(|q| q.id == id)
                .expect("every asked id is a registry entry")
        };
        for group in [&ids[..first_gate], &ids[first_gate..]] {
            assert!(
                group
                    .windows(2)
                    .all(|w| registry_order(w[0]) < registry_order(w[1])),
                "★ the reorder is a stable PARTITION, not a sort — registry order still decides \
                 everything inside each group: {group:?}"
            );
        }
    }

    /// ★ The prompt scope must track the REFUSAL scope. A spouse question asked of a spouse-less return is
    /// the prompt-level twin of the refusal-level bug D-8 fixed.
    #[test]
    fn spouse_questions_appear_exactly_when_a_spouse_does() {
        assert!(declaration_ids(&with_spouse(single())).contains(&QuestionId::DependentSpouse));
        assert!(has_spouse_dob(&with_spouse(single())));
        assert!(!declaration_ids(&single()).contains(&QuestionId::DependentSpouse));
        assert!(!has_spouse_dob(&single()));
    }

    /// ★ §2.9 — the foreign-account/-trust questions are asked on EVERY return, INCLUDING below the
    /// Schedule B threshold. Scoping them by `schedule_b_files` was the circular-liveness bug: that
    /// predicate reads `foreign_accounts` itself, so a never-asked account silently omitted Schedule B.
    #[test]
    fn foreign_questions_are_asked_even_below_the_schedule_b_threshold() {
        let ids = declaration_ids(&single()); // no interest at all — well below $1,500
        assert!(ids.contains(&QuestionId::ForeignAccounts));
        assert!(ids.contains(&QuestionId::ForeignTrust));
    }

    /// ★★★ **SEAM REVIEW M-3's KILL (i) — R3's DOOR QUESTIONS SIT WITH THE CENSUS.**
    ///
    /// They are the census's own follow-ups — live only because a census row was answered — and the
    /// old partition put them in the "other declarations" group, so a filer who had just answered
    /// *"no Form W-2"* met all nineteen skippables (both DOBs, the blindness pair, the sales-tax
    /// election) before being asked *"did you receive wages from an employer who issued no Form
    /// W-2?"*.
    ///
    /// ★ The assertion is a POSITION, not a membership: "is asked" was already true and is exactly
    ///   what could not see this.
    #[test]
    fn the_document_less_income_door_is_asked_with_the_census_not_after_the_skippables() {
        use btctax_core::tax::document_census::DocumentRow;
        let mut ri = single();
        for row in DocumentRow::ALL {
            ri.documents.set(*row, Some(false));
        }
        let asks = live_questions(&ri);
        let pos = |id: QuestionId| {
            asks.iter()
                .position(|a| a.declaration_id() == Some(id))
                .unwrap_or_else(|| panic!("{id:?} must be asked on an all-No census"))
        };
        let first_skippable = asks
            .iter()
            .position(|a| matches!(a, Ask::Skippable(_)))
            .expect("the skippables are asked");
        let last_census = asks
            .iter()
            .rposition(|a| {
                a.declaration_id().is_some_and(|id| {
                    btctax_core::tax::document_census::row_of_question(id).is_some()
                })
            })
            .expect("the census is asked");

        for door in [
            QuestionId::WagesWithoutW2Question,
            QuestionId::InterestOrDividendsWithout1099,
            QuestionId::StateRefundWithout1099g,
        ] {
            assert!(
                pos(door) > last_census,
                "★ {door:?} must come AFTER the census rows that make it live"
            );
            assert!(
                pos(door) < first_skippable,
                "★ THE KILL: {door:?} is the census's own follow-up and must be asked with it — \
                 at index {}, before the first skippable at {first_skippable}. It used to land \
                 after all nineteen.",
                pos(door)
            );
        }

        // …and an ordinary gate declaration still comes after the door, so the group is a real
        // third partition rather than "everything moved up".
        assert!(
            pos(QuestionId::DependentTaxpayer) > pos(QuestionId::WagesWithoutW2Question),
            "★ THE KILL: the gate declarations stay behind the census and its follow-ups"
        );
    }

    #[test]
    fn mfs_is_asked_whether_the_spouse_itemizes() {
        let mut mfs = single();
        mfs.filing_status = FilingStatus::Mfs;
        assert!(declaration_ids(&mfs).contains(&QuestionId::MfsSpouseItemizes));
        assert!(!declaration_ids(&single()).contains(&QuestionId::MfsSpouseItemizes));
    }

    /// ★ Every question the SCREEN can refuse for must be ASKABLE — otherwise `answer` cannot clear the
    /// refusal it exists to clear, and the year stays bricked. This is the property that ties the two
    /// scopes together; it is the whole point of the command.
    #[test]
    fn every_live_question_can_actually_be_answered_and_clears_the_screen() {
        let mut ri = with_spouse(single());
        ri.filing_status = FilingStatus::Mfj;
        ri.int_1099.push(Form1099Int {
            box1_interest: dec!(2000),
            ..Default::default()
        });
        use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
        use btctax_core::tax::return_refuse::screen_inputs;
        use btctax_core::tax::tables::FullReturnTables;
        use btctax_core::TaxTables;
        let fr = BundledFullReturnTables::load();
        let tt = BundledTaxTables::load();
        let params = fr.full_return_for(2024).expect("TY2024 params are bundled");
        let table = tt.table_for(2024).expect("TY2024 table is bundled");

        assert!(
            screen_inputs(&ri, table, params).is_some(),
            "an all-unanswered return must refuse — else this test proves nothing"
        );
        // ★★★ **THE SWEEP, mirrored from `answer_return_inputs`.** R3's document-less income door
        //     makes a question live only once its census row is answered `No`, so a SINGLE pass
        //     over `live_questions` cannot clear the screen — and this test is the property that
        //     says the command can. It re-derives the live set until it stops growing, exactly as
        //     the command does.
        let mut asked: std::collections::BTreeSet<QuestionId> = std::collections::BTreeSet::new();
        // ★★★ T7 — the per-row gates join the sweep, keyed as the command keys them: by IDENTITY,
        //     never by row index.
        let mut asked_gates: std::collections::BTreeSet<AskedKey> =
            std::collections::BTreeSet::new();
        for _ in 0..8 {
            let round: Vec<Ask> = live_questions(&ri)
                .into_iter()
                .filter(|a| a.declaration_id().is_none_or(|id| !asked.contains(&id)))
                .filter(|a| !asked_gates.contains(&asked_key_of(a)))
                .collect();
            if round.is_empty() {
                break;
            }
            for ask in round {
                if let Some(id) = ask.declaration_id() {
                    asked.insert(id);
                }
                if matches!(ask, Ask::DependentGate { .. }) {
                    asked_gates.insert(asked_key_of(&ask));
                }
                match ask {
                    // ★★ Answer "no" — EXCEPT on a document-census row, which is answered from what
                    //    this return actually carries. Since D1 a blanket "no" is itself a refusable
                    //    contradiction: this fixture holds a transcribed Form 1099-INT, and swearing it
                    //    received none is exactly `DocumentCensusContradicted`. That is the census
                    //    working, so the fixture answers TRUTHFULLY rather than the rule being relaxed.
                    Ask::Declaration(q) => {
                        let truth = btctax_core::tax::document_census::row_of_question(q.id)
                            .and_then(|row| {
                                btctax_core::tax::document_census::declared_rows(&ri, row)
                            })
                            .is_some_and(|n| n > 0);
                        (q.set)(&mut ri, truth);
                    }
                    Ask::Skippable(_) => {} // skippable by design
                    // ★★★ T7 / R6 — THE NO-BRICK PROPERTY REACHES THE PER-ROW GATES. Answered at
                    //     the registry's own declared claim-path polarity, which is the only
                    //     polarity a fixture may use — the alternative is a hand-list here, which
                    //     is exactly what `FormQuestion::neutral` exists to prevent one form over.
                    Ask::DependentGate { gate, row } => match gate.kind {
                        GateKind::Date => {
                            ri.header.dependents[row].date_of_birth = Some(
                                time::Date::from_calendar_date(
                                    ri.tax_year - 10,
                                    time::Month::June,
                                    1,
                                )
                                .unwrap(),
                            );
                        }
                        GateKind::YesNo => (gate.set)(
                            &mut ri.header.dependents[row],
                            gate.claim_path
                                .expect("a YesNo gate declares its claim path"),
                        ),
                    },
                }
            }
        }
        assert!(
            screen_inputs(&ri, table, params).is_none(),
            "answering every LIVE declaration must clear the screen — if it does not, `answer` cannot \
             rescue a bricked year and the whole command is a dead end"
        );
    }

    // ── The REAL-COMMAND harness (seam review I-1) ────────────────────────────────────────────────
    //
    // ★★★ These four helpers exist so the T7 KATs drive `answer_return_inputs` ITSELF rather than a
    //     re-implementation of its loop. The pattern is `open_next_year_t4b.rs::answer_the_draft`'s:
    //     derive the keystrokes by simulating the sweep, run the command, read the draft back.

    fn t7_pp() -> Passphrase {
        Passphrase::new("pw".into())
    }

    fn t7_vault() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        crate::cmd::init::run(&vault, &t7_pp(), &dir.path().join("k.asc")).unwrap();
        (dir, vault)
    }

    fn t7_draft(vault: &std::path::Path, year: i32) -> ReturnInputs {
        let s = Session::open(vault, &t7_pp()).unwrap();
        match crate::input_form_store::load(s.conn(), year).unwrap().0 {
            crate::input_form_store::Loaded::Draft { ri, .. } => ri,
            _ => panic!("`income answer` writes the {year} draft"),
        }
    }

    /// The keystroke script for one `income answer` run, DERIVED by simulating the command's own
    /// sweep — declarations at their `neutral` polarity, skippables skipped, every dependent gate at
    /// its declared `claim_path`. Returns the script, how many sweeps it took, and the `(row, gate)`
    /// pairs that were put to the filer.
    fn t7_script(
        seed: &ReturnInputs,
        params: Option<&FullReturnParams>,
    ) -> (
        String,
        usize,
        std::collections::BTreeSet<(usize, DependentGate)>,
    ) {
        let mut ri = seed.clone();
        let mut decl: std::collections::BTreeSet<QuestionId> = std::collections::BTreeSet::new();
        let mut skip: std::collections::BTreeSet<btctax_core::tax::questions::SkippableId> =
            std::collections::BTreeSet::new();
        let mut gates: std::collections::BTreeSet<(usize, DependentGate)> =
            std::collections::BTreeSet::new();
        let mut script = String::new();
        let mut sweeps = 0usize;
        loop {
            let round: Vec<Ask> = live_questions_with(&ri, params)
                .into_iter()
                .filter(|a| match a {
                    Ask::Declaration(q) => !decl.contains(&q.id),
                    Ask::Skippable(sk) => !skip.contains(&sk.id),
                    Ask::DependentGate { gate, row } => !gates.contains(&(*row, gate.gate)),
                })
                .collect();
            if round.is_empty() {
                break;
            }
            sweeps += 1;
            assert!(
                sweeps <= MAX_SWEEPS + 1,
                "the script generator did not settle"
            );
            for ask in round {
                // The command re-checks liveness immediately before each ask; a question that died
                // earlier in this same round is never put to the filer and takes no keystroke.
                let still_live = match &ask {
                    Ask::Declaration(q) => (q.live)(&ri),
                    Ask::Skippable(s) => (s.live)(&ri),
                    Ask::DependentGate { gate, row } => gate.live(&ri, *row),
                };
                if !still_live {
                    continue;
                }
                match ask {
                    Ask::Declaration(q) => {
                        decl.insert(q.id);
                        script.push_str(if q.neutral { "y\n" } else { "n\n" });
                        (q.set)(&mut ri, q.neutral);
                    }
                    Ask::Skippable(sk) => {
                        skip.insert(sk.id);
                        script.push('\n');
                    }
                    Ask::DependentGate { gate, row } => {
                        gates.insert((row, gate.gate));
                        match gate.kind {
                            GateKind::Date => {
                                let dob = time::Date::from_calendar_date(
                                    ri.tax_year - 10,
                                    time::Month::June,
                                    1,
                                )
                                .unwrap();
                                script.push_str(&format!("{dob}\n"));
                                ri.header.dependents[row].date_of_birth = Some(dob);
                            }
                            GateKind::YesNo => {
                                let v = gate
                                    .claim_path
                                    .expect("a YesNo gate declares its claim path");
                                script.push_str(if v { "y\n" } else { "n\n" });
                                (gate.set)(&mut ri.header.dependents[row], v);
                            }
                        }
                    }
                }
            }
        }
        (script, sweeps, gates)
    }

    /// ★★★ **T7 / R6 — `income answer` ASKS every live dependent gate, keyed by IDENTITY, the SWEEP
    /// SETTLES, and the RETURN THEN COMMITS.**
    ///
    /// Four things at once, because they are one property: a gate that is never asked cannot be
    /// answered (the no-brick rule, one row deeper); a gate keyed by ROW INDEX would move one
    /// child's diligence onto another; the flowchart opens a whole STEP at a time, so the sweep must
    /// reach the end of the chain inside `MAX_SWEEPS`; and every answer the command wrote must still
    /// be the answer to the question the filer would be asked TODAY.
    ///
    /// ★★★ **IT DRIVES THE REAL `answer_return_inputs`, NOT A COPY OF ITS LOOP** (seam review I-1).
    ///     The previous version re-implemented `live_questions_with`, the `asked` set, the key, the
    ///     liveness re-check and the `set` — and left out `record_answer`, the ONE line of the real
    ///     loop that can make assertion 4 false. With no record in the log `answer_status` reads
    ///     `NeverAsked`, the R10.3 staleness branch never runs, and the test reported PASS on a
    ///     return the real command could not commit. An emulation that omits the step under test is
    ///     green because it never ran it (B1).
    #[test]
    fn income_answer_asks_the_dependent_gates_and_the_sweep_settles() {
        use btctax_core::tax::dependent_gates::{walk_dependent, DependentVerdict};
        use btctax_core::tax::provenance::{answer_status, dependent_ssn_hash, AnswerStatus};
        use btctax_core::tax::return_refuse::screen_inputs;
        let (_dir, vault) = t7_vault();
        let mut seed = single();
        seed.tax_year = 2024;
        seed.header.dependents = vec![
            btctax_core::tax::return_inputs::Dependent {
                name: "First Kid".into(),
                ssn: "000-00-1111".into(),
                relationship: "Daughter".into(),
                ..Default::default()
            },
            btctax_core::tax::return_inputs::Dependent {
                name: "Second Kid".into(),
                ssn: "000-00-2222".into(),
                relationship: "Son".into(),
                ..Default::default()
            },
        ];
        let params = {
            use btctax_core::tax::tables::FullReturnTables;
            btctax_adapters::BundledFullReturnTables::load()
                .full_return_for(2024)
                .cloned()
                .expect("TY2024 params are bundled")
        };
        {
            let mut s = Session::open(&vault, &t7_pp()).unwrap();
            crate::input_form_store::save_draft(&mut s, 2024, &seed).unwrap();
        }

        // The keystrokes, derived by simulating the command's own sweep — never a magic count.
        let (script, sweeps, asked) = t7_script(&seed, Some(&params));
        assert!(
            sweeps <= MAX_SWEEPS,
            "the dependent chain must SETTLE inside the command's own guard — it did not, which \
             means liveness opens one gate at a time instead of one STEP at a time"
        );

        let mut keys = script.as_bytes();
        let mut screen: Vec<u8> = Vec::new();
        answer_return_inputs(
            &vault,
            &t7_pp(),
            2024,
            time::macros::date!(2026 - 02 - 03),
            &mut keys,
            &mut screen,
            AnswerOptions::default(),
        )
        .expect("every live question is scripted");
        let screen = String::from_utf8(screen).unwrap();
        let ri = t7_draft(&vault, 2024);

        // ── 1. Both rows completed the flowchart. ──
        for row in 0..2 {
            assert_eq!(
                walk_dependent(&ri, row).verdict,
                DependentVerdict::ChildTaxCredit,
                "row {row} was asked its whole chain"
            );
        }
        // ── 2. Every gate the walk demanded was ASKED and RECORDED, keyed by the row's own
        //       identity — and the record still answers the question asked TODAY. ──
        for row in 0..2 {
            let hash = dependent_ssn_hash(&ri.header.dependents[row].ssn);
            for g in walk_dependent(&ri, row).demanded_gates() {
                let key = AnswerKey::DependentGate {
                    ssn_hash: hash.clone(),
                    gate: g,
                };
                assert!(
                    asked.contains(&(row, g)),
                    "row {row}'s {g:?} is live and was never asked — a gate nobody asks is a gate \
                     nobody can answer"
                );
                assert!(
                    ri.answer_log.contains_key(&key),
                    "row {row}'s {g:?} was asked but no record was written under its identity"
                );
                // ★★★ C-1's KILL. `record_answer` hashes the REGISTRY'S WORDS; the row banner is
                //     display chrome. Hash the banner in and every gate reads `WordingChanged` the
                //     instant it is answered, the "after" panel lists all fifteen as blocking, and
                //     the commit gate refuses with an exit the filer has just followed.
                let status = answer_status(&ri, &key);
                assert_eq!(
                    status,
                    AnswerStatus::Given,
                    "row {row}'s {g:?} reads {status:?} immediately after being answered — the \
                     prompt hash is keyed on something no reader can reproduce (a class-(A) gate \
                     has no lawful decline, so `Given` is the only status it can hold)"
                );
            }
        }
        // ── 3. The two rows' keys are DISJOINT: no gate answer is shared between children. ──
        let h0 = dependent_ssn_hash("000-00-1111");
        let h1 = dependent_ssn_hash("000-00-2222");
        assert_ne!(h0, h1);
        let count = |h: &str| {
            ri.answer_log
                .keys()
                .filter(|k| matches!(k, AnswerKey::DependentGate { ssn_hash, .. } if ssn_hash == h))
                .count()
        };
        assert!(
            count(&h0) > 0 && count(&h0) == count(&h1),
            "one key set per child: {} vs {}",
            count(&h0),
            count(&h1)
        );
        // ── 4. The whole return then screens clean — the no-brick property, with gates. ──
        let table = {
            use btctax_core::TaxTables;
            btctax_adapters::BundledTaxTables::load()
                .table_for(2024)
                .cloned()
                .expect("TY2024 table is bundled")
        };
        assert!(
            screen_inputs(&ri, &table, &params).is_none(),
            "answering every live gate must clear the screen: {:?}",
            screen_inputs(&ri, &table, &params).map(|r| r.reason)
        );
        // ── 5. …and the filer was told so. The "after" panel is what they read at the keyboard. ──
        assert!(
            !screen.contains(btctax_core::tax::provenance::WORDING_CHANGED_REASON),
            "the command's own trailing panel told the filer their answers were stale:\n{screen}"
        );
    }

    /// ★★★ **SEAM REVIEW I-2 — THE SESSION'S `asked` SET IS KEYED BY THE ROW, SO TWO ROWS THAT
    ///     SHARE AN IDENTITY ARE BOTH ASKED THEIR WHOLE CHAIN.**
    ///
    /// The stored key is the row's `ssn_hash` and must stay that way (T1: delete row 0 and an
    /// index-keyed record moves one child's diligence onto another). The SESSION key must be the
    /// opposite, and this is why: two rows whose SSNs hash the same collided in the `asked` set, and
    /// the collision was invisible within one round and fatal across rounds — a gate row 0 was asked
    /// in round *n* was filtered out of row 1's round *n+1*, so it was never put to the filer,
    /// stayed `None`, and the screen refused it forever while the command exited reporting nothing
    /// left to ask. **Fail-closed on the claim, fail-OPEN on the interview.**
    ///
    /// ★ `screen_dependent_gates` now refuses this state outright and `answer_return_inputs` will
    ///   not enter its loop on it, so this is defence in depth — and it is the half that would still
    ///   hold if a future gate made two rows collide some other way. Revert `asked_key_of` to the
    ///   identity key and row 1 starves here.
    #[test]
    fn the_sessions_asked_key_is_the_row_so_a_shared_identity_starves_nobody() {
        use btctax_core::tax::dependent_gates::walk_dependent;
        use btctax_core::tax::provenance::dependent_ssn_hash;
        let mut ri = single();
        ri.tax_year = 2024;
        // The degenerate state the review probed: two rows, one identity.
        ri.header.dependents = vec![
            btctax_core::tax::return_inputs::Dependent {
                name: "First Kid".into(),
                ssn: "000-00-1111".into(),
                relationship: "Daughter".into(),
                ..Default::default()
            },
            btctax_core::tax::return_inputs::Dependent {
                name: "Second Kid".into(),
                ssn: "000001111".into(), // the SAME digits — one `dependent_ssn_hash`
                relationship: "Son".into(),
                ..Default::default()
            },
        ];
        assert_eq!(
            dependent_ssn_hash(&ri.header.dependents[0].ssn),
            dependent_ssn_hash(&ri.header.dependents[1].ssn),
            "the premise: these two rows share one stored key"
        );
        let mut asked: std::collections::BTreeSet<AskedKey> = std::collections::BTreeSet::new();
        for _ in 0..=MAX_SWEEPS {
            let round: Vec<Ask> = live_questions(&ri)
                .into_iter()
                .filter(|a| !asked.contains(&asked_key_of(a)))
                .collect();
            if round.is_empty() {
                break;
            }
            for ask in round {
                asked.insert(asked_key_of(&ask));
                match ask {
                    // Step 2 q4 and Step 5 q1 are RETURN-level, so the chain waits on them too.
                    Ask::Declaration(q) => (q.set)(&mut ri, q.neutral),
                    Ask::Skippable(_) => {}
                    Ask::DependentGate { gate, row } => match gate.kind {
                        GateKind::Date => {
                            ri.header.dependents[row].date_of_birth =
                                Some(time::macros::date!(2014 - 06 - 01));
                        }
                        GateKind::YesNo => (gate.set)(
                            &mut ri.header.dependents[row],
                            gate.claim_path
                                .expect("a YesNo gate declares its claim path"),
                        ),
                    },
                }
            }
        }
        for row in 0..2 {
            for g in walk_dependent(&ri, row).demanded_gates() {
                assert!(
                    asked.contains(&AskedKey::DependentGate { row, gate: g }),
                    "row {row}'s {g:?} is live and was never asked — the other row's identical key \
                     filtered it out of every later round, so the interview ENDS with a live gate \
                     nobody put to the filer"
                );
            }
        }
        // …and the interview really did finish: nothing live is left blank on either row.
        for row in 0..2 {
            let verdict = walk_dependent(&ri, row).verdict;
            assert!(
                !matches!(
                    verdict,
                    btctax_core::tax::dependent_gates::DependentVerdict::Unanswered(_)
                ),
                "row {row} ended the interview on an UNANSWERED gate: {verdict:?}"
            );
        }
    }

    /// ★★★ **SEAM REVIEW I-2 — `income answer` REFUSES A ROW WITH NO IDENTITY, OR A SHARED ONE,
    ///     BEFORE THE FIRST QUESTION — and names the row.**
    ///
    /// It refuses rather than asking, and the reason is that there is nothing to ask: a dependent's
    /// name, SSN and relationship are identity fields the row is CREATED with (`income import`, or
    /// the tax-inputs form's Dependents section), not registry questions. The exit named is the one
    /// that exists. `screen_inputs` refuses the identical state at the commit gate, so this is the
    /// same rule met earlier rather than a second one.
    #[test]
    fn income_answer_refuses_a_dependent_row_with_no_identity_or_a_shared_one() {
        for (label, rows, wanted) in [
            (
                "blank",
                vec![("Kid Example", "")],
                vec!["row 1 (Kid Example)", "no Social Security number"],
            ),
            (
                "shared",
                vec![("First Kid", "000-00-1111"), ("Second Kid", "000001111")],
                vec![
                    "row 1 (First Kid)",
                    "row 2 (Second Kid)",
                    "the same Social Security number",
                ],
            ),
        ] {
            let (_dir, vault) = t7_vault();
            let mut ri = single();
            ri.tax_year = 2024;
            ri.header.dependents = rows
                .iter()
                .map(|(name, ssn)| btctax_core::tax::return_inputs::Dependent {
                    name: (*name).into(),
                    ssn: (*ssn).into(),
                    relationship: "Daughter".into(),
                    ..Default::default()
                })
                .collect();
            {
                let mut s = Session::open(&vault, &t7_pp()).unwrap();
                crate::input_form_store::save_draft(&mut s, 2024, &ri).unwrap();
            }
            let mut keys: &[u8] = b"";
            let mut screen: Vec<u8> = Vec::new();
            let err = answer_return_inputs(
                &vault,
                &t7_pp(),
                2024,
                time::macros::date!(2026 - 02 - 03),
                &mut keys,
                &mut screen,
                AnswerOptions::default(),
            )
            .expect_err("a row with no owner cannot be interviewed");
            let msg = err.to_string();
            for want in wanted {
                assert!(msg.contains(want), "{label}: {msg}");
            }
            assert!(
                msg.contains("Nothing was stored"),
                "{label}: the refusal says nothing was written: {msg}"
            );
            // ★ And it refused BEFORE the first question — no prompt reached the filer.
            assert!(
                String::from_utf8(screen).unwrap().is_empty(),
                "{label}: the refusal precedes the interview"
            );
        }
    }

    /// ★★★ **THE PARAMS-QUOTING GATE IS NOT PUT TO THE FILER ON A PARAMS-LESS YEAR.** R12 lists it
    /// as *waiting*; asking it would mean asking a question whose words are not yet knowable.
    #[test]
    fn the_gross_income_gate_is_not_asked_without_the_years_package() {
        use btctax_core::tax::provenance::DependentGate;
        let mut ri = single();
        ri.tax_year = 2026;
        ri.header.dependents = vec![btctax_core::tax::return_inputs::Dependent {
            name: "Grandparent".into(),
            ssn: "000-00-3333".into(),
            relationship: "Mother".into(),
            date_of_birth: Some(time::macros::date!(1950 - 03 - 04)),
            qc_relationship: Some(false),
            younger_than_you_or_spouse: Some(false),
            full_time_student: Some(false),
            permanently_and_totally_disabled: Some(false),
            provided_over_half_own_support: Some(false),
            filing_joint_return: Some(false),
            lived_with_you_over_half_year: Some(true),
            lived_with_you_in_us: Some(true),
            ..Default::default()
        }];
        let gate_of = |a: &Ask| a.dependent_gate().map(|(g, _)| g);
        let without: Vec<_> = live_questions(&ri).iter().filter_map(gate_of).collect();
        assert!(
            !without.contains(&DependentGate::GrossIncomeUnderLimit),
            "on a params-less year the gate is WAITING, not asked: {without:?}"
        );
        assert!(
            without.contains(&DependentGate::QrRelationshipOrMemberOfHousehold),
            "…while the rest of Step 4 IS asked, so the exclusion is the figure and nothing else"
        );
        let mut p = btctax_core::tax::testonly::ty2024_params();
        p.qualifying_relative_gross_income_limit = rust_decimal_macros::dec!(5300);
        let with: Vec<_> = live_questions_with(&ri, Some(&p))
            .iter()
            .filter_map(gate_of)
            .collect();
        assert!(
            with.contains(&DependentGate::GrossIncomeUnderLimit),
            "with the package it IS asked: {with:?}"
        );
    }

    /// A return set up so `id` is LIVE (nothing answered yet).
    fn scenario_for(id: QuestionId) -> ReturnInputs {
        use btctax_core::tax::return_inputs::ScheduleAInputs;
        let mut r = single();
        match id {
            QuestionId::DependentSpouse => r.filing_status = FilingStatus::Mfj,
            QuestionId::MfsSpouseItemizes => r.filing_status = FilingStatus::Mfs,
            // ★ R10.4 / T4b — live only on a year the OPENER made.
            QuestionId::FilingStatusConfirmed => r.opened_from = Some(2024),
            // ★★★ R8 / T9 — the Form 8396 gate shares the three mortgage declarations' precondition
            //     (a transcribed Form 1098 on an itemizing return), so it shares their scenario.
            QuestionId::ClaimingMortgageInterestCredit
            | QuestionId::MortgageAllUsedToBuyBuildImprove
            | QuestionId::AmtQualifiedDwelling
            | QuestionId::MortgageWithinDebtLimit => {
                r.schedule_a = Some(ScheduleAInputs {
                    ..Default::default()
                });
                r.form_1098 = vec![btctax_core::tax::testonly::form_1098_with_interest(dec!(
                    9000
                ))];
            }
            // ★★★ R8 / T9 — the three sale-of-a-main-home tests are live only on a YES to
            //     `sold_main_home`, which is a SIBLING registry entry's answer — so the scenario
            //     sets it directly rather than leaving it to the neutral loop, which answers NO.
            QuestionId::HomeSaleTest1OwnedAndLived
            | QuestionId::HomeSaleTest2NoRecentExclusion
            | QuestionId::HomeSaleCanExcludeAllGain => {
                r.home_sale.sold_main_home = Some(true);
            }
            // ★ The three carryforward-conditioned declarations share ONE liveness predicate
            //   (`questions::carryforward_in_present`), so they share one scenario.
            QuestionId::AmtCarryoverSameAsRegular
            | QuestionId::CarryoverIncludesSpousesJointLoss
            | QuestionId::ExcludedCanceledDebt => {
                r.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
                    short: dec!(1000),
                    long: dec!(0),
                };
            }
            QuestionId::AmtDepreciationSameAsRegular => {
                r.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
                    expenses: dec!(5000),
                    ..Default::default()
                });
            }
            // ★★★ T16 — Form 8889's seven questions share ONE liveness predicate: the §223 trigger
            //     declaration is affirmed. One scenario, exactly as the carryforward trio above.
            QuestionId::HsaFamilyCoverage
            | QuestionId::HsaEligibleEveryMonth
            | QuestionId::HsaAge55OrOlder
            | QuestionId::HsaMedicareEnrollment
            | QuestionId::HsaBothSpousesHaveHsas
            | QuestionId::HsaArcherMsaActivity
            | QuestionId::HsaTestingPeriodFailure => {
                r.sch1.hsa_activity = Some(true);
            }
            // ★★★ Seam review I-3 — the SPOUSE's plan needs the trigger AND a spouse; the
            //     instruction's *"regardless of whether you file jointly or separately"* makes both
            //     married statuses live, and MFJ is the one that drags in no §63(f) side conditions.
            QuestionId::HsaSpouseFamilyCoverage => {
                r.sch1.hsa_activity = Some(true);
                r.filing_status = btctax_core::tax::types::FilingStatus::Mfj;
            }
            // ★★★ Seam review M-1 — R3's fourth door: the trigger affirmed and the Form 1099-SA
            //     census row answered "I received none".
            QuestionId::HsaDistributionWithout1099sa => {
                r.sch1.hsa_activity = Some(true);
                r.documents.set(
                    btctax_core::tax::document_census::DocumentRow::Sa1099,
                    Some(false),
                );
            }
            // ★★★ T7 / R6 — Step 5 question 1 is live iff the return carries a DEPENDENT ROW.
            QuestionId::FilerTinIssuedByDueDate => {
                r.header.dependents = vec![btctax_core::tax::return_inputs::Dependent {
                    name: "Kid Example".into(),
                    ssn: "000-00-1111".into(),
                    relationship: "Daughter".into(),
                    ..Default::default()
                }];
            }
            // ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. Each of the three paired questions is
            //     live EXACTLY when its census row says `No` — the pairing R3 states — so the
            //     scenario answers that row and nothing else.
            // ★★★ R7 / T8 — HoH's two tests and QSS's five conditions are live iff the FILING STATUS
            //     asserts them, and a return has exactly one status. FR-67's election gate needs a
            //     spouse `Person`.
            QuestionId::HohQualifyingPerson | QuestionId::HohPaidOverHalfCostOfKeepingUpHome => {
                r.filing_status = FilingStatus::HoH;
            }
            QuestionId::NraSpouseResidentElection => {
                r.filing_status = FilingStatus::Mfj;
                r.header.spouse = Some(btctax_core::tax::return_inputs::Person {
                    first_name: "Sam".into(),
                    last_name: "Roe".into(),
                    ssn: "000-00-5555".into(),
                    ..Default::default()
                });
            }
            QuestionId::QssSpouseDiedInWindowAndNotRemarried
            | QuestionId::QssChildYouCanClaim
            | QuestionId::QssChildLivedInYourHomeAllYear
            | QuestionId::QssPaidOverHalfCostOfKeepingUpHome
            | QuestionId::QssCouldHaveFiledJointlyInYearOfDeath => {
                r.filing_status = FilingStatus::Qss;
            }
            QuestionId::WagesWithoutW2Question => {
                r.documents.set(
                    btctax_core::tax::document_census::DocumentRow::W2,
                    Some(false),
                );
            }
            QuestionId::InterestOrDividendsWithout1099 => {
                r.documents.set(
                    btctax_core::tax::document_census::DocumentRow::Int1099,
                    Some(false),
                );
            }
            QuestionId::StateRefundWithout1099g => {
                r.documents.set(
                    btctax_core::tax::document_census::DocumentRow::G1099,
                    Some(false),
                );
            }
            // ★ R3/I1 — the §111(a) gate, made live from the DOCUMENT side (a transcribed box 2),
            //   so the scenario does not depend on another registry entry's answer.
            QuestionId::ItemizedPriorYear => {
                r.g_1099 = vec![btctax_core::tax::return_inputs::Form1099G {
                    payer: "State of Example".into(),
                    box2_state_refund: dec!(900),
                    ..Default::default()
                }];
                r.documents.set(
                    btctax_core::tax::document_census::DocumentRow::G1099,
                    Some(true),
                );
            }
            _ => {}
        }
        r
    }

    /// ★★★ **R12 — THE NO-BRICK PROPERTY, EXTENDED TO THE PANEL.**
    ///
    /// The registry-derived version above proves every live declaration is ASKED. This proves the
    /// other half: answering every blocking item **through its own setter** empties `blocking`, and
    /// `refusing` is empty before `screen_inputs` passes. Without the second clause a return could
    /// have nothing left to answer and still be unfileable — which is exactly the brick the panel
    /// exists to make visible while the filer is at the keyboard.
    #[test]
    fn answering_every_blocking_item_empties_the_panel_and_refusing_is_empty_before_the_screen_passes(
    ) {
        use btctax_core::tax::interview_state::interview_state;
        use btctax_core::tax::provenance::AnswerKey;

        // A fresh Single return: nothing answered at all, so `blocking` is the whole live class-(A)
        // set — including R3's census rows.
        let mut ri = ReturnInputs {
            tax_year: 2024,
            filing_status: btctax_core::FilingStatus::Single,
            ..Default::default()
        };
        let before = interview_state(&ri);
        assert!(
            before.blocking.len() >= 20,
            "a fresh return blocks on every live declaration (got {})",
            before.blocking.len()
        );

        // Answer each blocking item through the registry's OWN setter — the same call `income
        // answer` makes — at its neutral. Loop to a fixpoint: answering one question can make
        // another live (liveness is a predicate over the return).
        for _ in 0..8 {
            let st = interview_state(&ri);
            if st.blocking.is_empty() {
                break;
            }
            for b in &st.blocking {
                let AnswerKey::Question(id) = b.item else {
                    continue;
                };
                let q = FORM_QUESTIONS
                    .iter()
                    .find(|q| q.id == id)
                    .expect("a blocking item names a registry question");
                (q.set)(&mut ri, q.neutral);
            }
        }

        let after = interview_state(&ri);
        assert!(
            after.blocking.is_empty(),
            "answering every blocking item through its own setter must empty `blocking`: {:#?}",
            after.blocking.iter().map(|b| &b.item).collect::<Vec<_>>()
        );
        assert!(
            after.refusing.is_empty(),
            "★ and nothing may be left REFUSING — a return with nothing to answer and no way to \
             file is the brick: {:#?}",
            after.refusing.iter().map(|r| &r.exit).collect::<Vec<_>>()
        );
        assert!(after.is_committable());

        // …and the screen agrees. The panel is derived from the registries and the screen is not,
        // so this is a real cross-check rather than a restatement.
        assert!(
            btctax_core::tax::return_refuse::screen_inputs(
                &ri,
                &btctax_core::tax::testonly::ty2024_table(),
                &btctax_core::tax::testonly::ty2024_params(),
            )
            .is_none(),
            "with the panel empty, `screen_inputs` must report no UNANSWERED-class refusal"
        );
    }

    /// ★★★ **T12 fold / seam review I-1 — THE PANEL NEVER SAYS *"no answer refuses"* ON A RETURN
    ///     THE COMMIT GATE REFUSES.**
    ///
    /// J-32's filer, one refusal on: every live declaration answered, nothing forgone — and a main
    /// home sold that they cannot exclude all the gain on. `screen_inputs`, which
    /// `input_form_store::commit` runs before it writes the vault, refuses with
    /// `HomeSaleNotComputed`. The panel's `refusing` list used to be assembled from the registries
    /// alone, five of `RefuseReason`'s 126 variants, so it printed *"nothing is open: … and no
    /// answer refuses"* — an affirmatively false sentence, on the surface `LIMITATIONS.md` promises
    /// will show the filer a refusal *"while you are still authoring rather than at commit"*.
    ///
    /// ★★ Asserted on the RENDERED LINES, not on the walk: the sentence is the artifact, and the
    ///    commit modal (which shows forgoing + refusing only) would otherwise be blank on this
    ///    return and the write refused after Enter.
    #[test]
    fn the_panel_does_not_claim_nothing_refuses_when_the_commit_screen_refuses() {
        use btctax_core::tax::document_census::DocumentRow;
        use btctax_core::tax::interview_state::interview_state_with_params;

        let mut ri = ReturnInputs {
            tax_year: 2024,
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        // …and every live SKIPPABLE answered too, so `forgoing` empties and `open_items() == 0` —
        // which is the state in which the panel prints its zero-open sentence at all. Driven off
        // the registry to a fixpoint (answering one can make another live), never a hand list.
        for _ in 0..8 {
            for sk in btctax_core::tax::questions::SKIPPABLE_QUESTIONS {
                if !(sk.live)(&ri) {
                    continue;
                }
                match sk.kind {
                    btctax_core::tax::questions::SkippableKind::YesNo => {
                        (sk.set_bool)(&mut ri, false)
                    }
                    btctax_core::tax::questions::SkippableKind::Date => {
                        (sk.set_date)(&mut ri, time::macros::date!(1980 - 01 - 01));
                    }
                    btctax_core::tax::questions::SkippableKind::Choice(opts) => {
                        if let Some(first) = opts.first() {
                            (sk.set_choice)(&mut ri, first);
                        }
                    }
                }
            }
        }
        ri.home_sale.sold_main_home = Some(true);
        ri.home_sale.test1_owned_2_years_and_lived_2_years_of_last_5 = Some(true);
        ri.home_sale.test2_no_exclusion_on_another_home_in_2_years = Some(true);
        ri.home_sale.can_exclude_all_gain = Some(false);
        ri.documents.set(DocumentRow::S1099, Some(false));

        // The gate refuses — read off the gate, never asserted from memory.
        let refusal = btctax_core::tax::return_refuse::screen_inputs(
            &ri,
            &btctax_core::tax::testonly::ty2024_table(),
            &btctax_core::tax::testonly::ty2024_params(),
        )
        .expect("the fixture must be one the commit screen refuses, or this test asserts nothing");

        let st = interview_state_with_params(&ri, &btctax_core::tax::testonly::ty2024_params());
        let lines = panel_lines(&st, "tax year 2024");
        // ★ The fixture must actually reach the zero-open sentence's branch, or the headline
        //   assertion below is vacuous — a forgone benefit alone would suppress the sentence.
        assert_eq!(
            st.blocking.len() + st.forgoing.len() + st.waiting.len() + st.not_computed.len(),
            0,
            "nothing but the refusal may be open, or this test does not reach the sentence: \
             {lines:#?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains("no answer refuses")),
            "the commit gate refuses this return ({:?}) and the panel told the filer nothing \
             refuses: {lines:#?}",
            refusal.reason
        );
        // …and it says WHAT refuses, in the words the gate would have used.
        assert!(
            lines.iter().any(|l| l.contains("REFUSING (")),
            "the refusal has its own heading: {lines:#?}"
        );
        assert!(
            lines.iter().any(|l| l.contains(&refusal.detail)),
            "…carrying the gate's own exit sentence, so the filer reads the same words while \
             authoring as they would at the write:\n  wanted: {}\n  lines: {lines:#?}",
            refusal.detail
        );
        // The commit modal shows forgoing + refusing only — on this return it must not be blank.
        assert!(
            !refusing_lines(&st).is_empty(),
            "the last screen before the write must carry the refusal"
        );

        // ── …and the SAME return without the sale reaches the zero-open sentence, which must
        //    carry its boundary beside it. A surface that asserts completeness it does not have is
        //    the finding; the qualifier is the half that makes the claim true, so it is asserted
        //    here rather than trusted to survive the next edit.
        let mut clean = ri.clone();
        clean.home_sale.sold_main_home = Some(false);
        let st = interview_state_with_params(&clean, &btctax_core::tax::testonly::ty2024_params());
        assert_eq!(st.open_items(), 0, "nothing is open on the clean return");
        let lines = panel_lines(&st, "tax year 2024");
        assert!(
            lines.iter().any(|l| l.contains("no answer refuses")),
            "the zero-open sentence is printed: {lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("a figure in the year's tax table")),
            "…and never on its own: the one tier the panel cannot run must be named beside it, or \
             the sentence claims a completeness it does not have: {lines:#?}"
        );
    }

    /// ★ §4.2 — the panel is printed BEFORE the first question and AFTER the last.
    ///
    /// Mutation: delete either `write_panel` call in `answer_return_inputs` and this reds.
    #[test]
    fn income_answer_prints_the_panel_first_and_last() {
        let mut screen: Vec<u8> = Vec::new();
        let st = btctax_core::tax::interview_state::interview_state(&single());
        write_panel(&mut screen, &st, "before").unwrap();
        let rendered = String::from_utf8(screen).unwrap();
        assert!(
            rendered.contains("The answer panel (before)"),
            "the panel names which end it is: {rendered}"
        );
        assert!(
            rendered.contains("BLOCKING"),
            "a fresh Single return has blocking items: {rendered}"
        );
        assert!(
            !rendered.contains('%') && !rendered.to_lowercase().contains("progress"),
            "R15 — no progress bar, ever: {rendered}"
        );
    }

    /// ★★★ **FR-106's KILL — THE CAVEAT IS PRINTED, AND WHAT IT CLAIMS IS TRUE.**
    ///
    /// The journey walk read *"BLOCKING (36)"* at the start of its session and then met two
    /// questions that were not in that list, opened by its own earlier answers in the same pass.
    /// Correct by design; unannounced.
    ///
    /// ★★ **Both halves, because either alone is decoration.** A test that only greps for the
    ///    sentence passes on a sentence that is FALSE (nothing checks that a count can actually
    ///    grow), and a test that only demonstrates growth passes while the filer is told nothing.
    ///    So this asserts the words are beside the count AND drives a return whose blocking count
    ///    really does rise when one live question is answered — the walk's own experience, in a
    ///    fixture.
    ///
    /// ★ The growth is produced through the registry's own liveness, not by hand: answering
    ///   `sold_main_home` YES is what makes the three §121 home-sale tests live (R8 / T9), so the
    ///   count goes DOWN by the question answered and UP by the three it opened. A registry change
    ///   that removed every such dependency would red the second half — and at that point the
    ///   sentence would deserve to be deleted.
    #[test]
    fn the_blocking_count_says_it_can_grow_and_it_really_can() {
        const CAVEAT: &str = "answering can make it GROW";

        // ── (1) THE WORDS, beside the count they qualify.
        let st = btctax_core::tax::interview_state::interview_state(&single());
        let lines = panel_lines(&st, "before");
        let at = lines
            .iter()
            .position(|l| l.contains("BLOCKING ("))
            .expect("a fresh Single return blocks on something");
        assert!(
            lines[at + 1..]
                .iter()
                .take_while(|l| !l.trim_start().starts_with("REFUSING")
                    && !l.trim_start().starts_with("FORGOING"))
                .any(|l| l.contains(CAVEAT)),
            "★ THE KILL: the BLOCKING count is a snapshot of NOW and the filer is not told, so \
             a number they read as \"how much is left\" grows while they work: {lines:#?}"
        );

        // ── (2) …AND THE CLAIM IS TRUE. One answer, given the way a filer gives it, and the count
        //        this panel prints goes UP.
        let mut ri = single();
        ri.home_sale.sold_main_home = None;
        let before = btctax_core::tax::interview_state::interview_state(&ri)
            .blocking
            .len();
        ri.home_sale.sold_main_home = Some(true);
        let after = btctax_core::tax::interview_state::interview_state(&ri)
            .blocking
            .len();
        assert!(
            after > before,
            "the caveat asserts a behaviour this interview must actually have: answering \
             `sold_main_home` YES opens the three §121 tests, so the blocking count must RISE \
             ({before} -> {after})"
        );
    }

    /// ★ THE no-brick property, registry-DERIVED (§3.5 assertion 3 / r4 I-3 / IMPL r1 I-1). For EVERY
    /// registry entry: on a return where it is live, `income answer` must ASK it. Held by identity today,
    /// but the spec mandated this assertion by name after it went red three revisions running — and a
    /// hand-written per-question test silently omitted the mortgage question (its liveness is the one
    /// non-trivial predicate). Deriving it means a dropped or mis-filtered entry — for ANY question, incl.
    /// the ones steps 6–12 keep adding to this file — fails a named test.
    #[test]
    fn income_answer_asks_every_live_declaration() {
        for q in FORM_QUESTIONS {
            let ri = scenario_for(q.id);
            // ★★ R3 — ONE census row is DELIBERATELY not live yet (`form_1098` → T9: its amount is
            //    collected today by a scalar). A never-live question cannot be exercised by this
            //    property; the skip is DERIVED from the census's own liveness predicate, not from a
            //    name list, so the moment T9 flips `row_is_live` the row re-enters this loop with
            //    no edit here — which is exactly what `form_1098e` did at T5.
            if !(q.live)(&ri) {
                let row = btctax_core::tax::document_census::row_of_question(q.id);
                assert!(
                    row.is_some_and(|row| !btctax_core::tax::document_census::row_is_live(
                        &ri, row
                    )),
                    "{:?} is not live in its own scenario and is not a census row awaiting its \
                     screen (T9)",
                    q.id
                );
                continue;
            }
            assert!(
                (q.live)(&ri),
                "{:?} must be live in its own scenario (test bug otherwise)",
                q.id
            );
            assert!(
                live_questions(&ri)
                    .iter()
                    .any(|a| a.declaration_id() == Some(q.id)),
                "income answer must ask {:?} when it is live — else the screen can refuse for a question \
                 the user can never answer (the near-brick D-8's recovery exists to prevent)",
                q.id
            );
        }
    }

    /// ★★★ **R6 / T8 — the panel PRINTS the line-19 forgo, with its size, under its own heading.**
    ///
    /// The renderer is the last surface between the computation and the filer, and a `NotComputed`
    /// row nothing printed would be a figure with no reader. Both halves are asserted: the sentence
    /// carries the year's own figure with a package, and no figure at all without one.
    #[test]
    fn income_answer_prints_the_line_19_forgo_with_its_size() {
        let mut ri = single();
        ri.header.dependents = vec![btctax_core::tax::return_inputs::Dependent {
            name: "Kid Example".into(),
            ssn: "000-00-1111".into(),
            relationship: "Daughter".into(),
            ..Default::default()
        }];
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        let p = btctax_core::tax::testonly::ty2024_params();

        let mut screen: Vec<u8> = Vec::new();
        write_panel(&mut screen, &panel_state(&ri, Some(&p)), "before").unwrap();
        let with_package = String::from_utf8(screen).unwrap();
        assert!(with_package.contains("NOT COMPUTED"), "{with_package}");
        assert!(with_package.contains("child tax credit"), "{with_package}");
        assert!(
            with_package.contains("1 dependent with a credit box"),
            "{with_package}"
        );
        assert!(
            with_package.contains("2000"),
            "the size is the year's own figure: {with_package}"
        );

        let mut screen: Vec<u8> = Vec::new();
        write_panel(&mut screen, &panel_state(&ri, None), "before").unwrap();
        let no_package = String::from_utf8(screen).unwrap();
        assert!(no_package.contains("NOT COMPUTED"), "{no_package}");
        assert!(
            !no_package.contains("2000"),
            "no package \u{2014} no invented figure: {no_package}"
        );
        assert!(
            no_package.contains("size waiting on"),
            "…and it names what it waits on: {no_package}"
        );

        // A return with no dependents never prints the heading at all.
        let mut plain = single();
        btctax_core::tax::testonly::answer_all_live_declarations(&mut plain);
        let mut screen: Vec<u8> = Vec::new();
        write_panel(&mut screen, &panel_state(&plain, Some(&p)), "before").unwrap();
        assert!(!String::from_utf8(screen).unwrap().contains("NOT COMPUTED"));
    }

    /// ★★★ **THE NO-BRICK PROPERTY FOR THE SKIPPABLE REGISTRY, registry-DERIVED** — the sibling of
    /// `income_answer_asks_every_live_declaration`, and it exists because R7 put the FIRST class-(A)
    /// entry in that registry: an entry whose blank REFUSES and which `income answer` did not ask
    /// would be a brick with no exit at all, which is strictly worse than the declaration case.
    ///
    /// ★ The scenario is derived per entry from the registry's own `live` predicate, so a new
    ///   skippable joins this loop with no edit here.
    #[test]
    fn income_answer_asks_every_live_skippable_including_the_class_a_one() {
        use btctax_core::tax::questions::SkippableKind;
        let mut asked_any_class_a = false;
        for s in SKIPPABLE_QUESTIONS {
            let ri = skippable_scenario_for(s.id);
            if !(s.live)(&ri) {
                continue;
            }
            assert!(
                live_questions(&ri)
                    .iter()
                    .any(|a| matches!(a, Ask::Skippable(e) if e.id == s.id)),
                "income answer must ask {:?} when it is live",
                s.id
            );
            if s.unanswered.is_some() {
                asked_any_class_a = true;
                // A class-(A) skippable BLOCKS while blank, and answering it through its own setter
                // clears the block — the no-brick property, one registry over.
                let st = btctax_core::tax::interview_state::interview_state(&ri);
                assert!(
                    st.blocking.iter().any(|b| b.item
                        == btctax_core::tax::provenance::AnswerKey::Skippable(s.id)),
                    "{:?} declares a refusal, so the panel must list it as BLOCKING",
                    s.id
                );
                let mut answered = ri.clone();
                match s.kind {
                    SkippableKind::Choice(options) => (s.set_choice)(&mut answered, options[0]),
                    SkippableKind::YesNo => (s.set_bool)(&mut answered, true),
                    SkippableKind::Date => panic!("a class-(A) Date skippable needs a case here"),
                }
                let st = btctax_core::tax::interview_state::interview_state(&answered);
                assert!(
                    !st.blocking.iter().any(|b| b.item
                        == btctax_core::tax::provenance::AnswerKey::Skippable(s.id)),
                    "answering {:?} through its own setter must empty the item",
                    s.id
                );
            }
        }
        assert!(
            asked_any_class_a,
            "the loop must have exercised the class-(A) arm, or it passes by finding nothing"
        );
    }

    /// A return on which `id` is live. Derived per entry from the registry's own predicate: each arm
    /// primes exactly the condition that entry's `live` reads.
    fn skippable_scenario_for(id: SkippableId) -> ReturnInputs {
        use btctax_core::tax::return_inputs::{ParentAliveAnswer, ScheduleAInputs};
        let mut r = single();
        match id {
            SkippableId::BlindSpouse | SkippableId::DobSpouse => r = with_spouse(single()),
            SkippableId::DodSpouse | SkippableId::SpouseDiedDuringYear => {
                r = with_spouse(single());
                r.filing_status = FilingStatus::Mfj;
                r.header.spouse_died_during_year = Some(true);
            }
            SkippableId::DodTaxpayer => r.header.taxpayer_died_during_year = Some(true),
            SkippableId::SalesTaxElection => r.schedule_a = Some(ScheduleAInputs::default()),
            SkippableId::FbarFilingRequired => r.foreign_accounts = Some(true),
            SkippableId::ScheduleC1099Required
            | SkippableId::ScheduleCIsSstb
            | SkippableId::ScheduleCIsCooperativePatron => {
                r.schedule_c = Some(Default::default());
            }
            SkippableId::ScheduleC1099Filed => {
                r.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
                    payments_requiring_1099: Some(true),
                    ..Default::default()
                });
            }
            SkippableId::Form8615ParentIdentityUnobtainable => {
                r.header.form8615_condition4_parent_alive = Some(ParentAliveAnswer::CannotKnow);
            }
            // ★★★ R7 / T8 — the class-(A) entry: live iff the filer checked Head of household.
            SkippableId::HohMaritalBasis => r.filing_status = FilingStatus::HoH,
            _ => {}
        }
        r
    }

    /// ★ P9 §2.2 step 7 — the class-(B) SKIPPABLE bool prompts: blindness (taxpayer always; spouse only
    /// with a spouse `Person`) and the §164(b)(5) sales-tax election (only with a Schedule A). Skippable ⇒
    /// a bare Enter leaves `None`, and the forgone-benefit advisory fires (the owner mandate).
    #[test]
    fn income_answer_asks_the_class_b_skippables_when_live() {
        use btctax_core::tax::return_inputs::ScheduleAInputs;
        // The core registry entry for `id` — the source of truth `income answer` now derives from.
        fn reg(id: SkippableId) -> &'static SkippableQuestion {
            SKIPPABLE_QUESTIONS
                .iter()
                .find(|s| s.id == id)
                .expect("id is a registry entry")
        }
        fn has(ri: &ReturnInputs, want: SkippableId) -> bool {
            live_questions(ri)
                .iter()
                .any(|a| matches!(a, Ask::Skippable(s) if s.id == want))
        }
        // Taxpayer blindness is always live; spouse-blind and SALT only when their gate is met.
        assert!(has(&single(), SkippableId::BlindTaxpayer));
        assert!(
            !has(&single(), SkippableId::BlindSpouse),
            "no spouse ⇒ no spouse-blind"
        );
        assert!(
            !has(&single(), SkippableId::SalesTaxElection),
            "no Sch A ⇒ no SALT"
        );

        assert!(has(&with_spouse(single()), SkippableId::BlindSpouse));

        let mut with_a = single();
        with_a.schedule_a = Some(ScheduleAInputs::default());
        assert!(has(&with_a, SkippableId::SalesTaxElection));

        // A bool skippable roundtrips through the CORE registry accessors and is genuinely skippable.
        let mut ri = with_spouse(single());
        assert_eq!((reg(SkippableId::BlindTaxpayer).get_bool)(&ri), None);
        (reg(SkippableId::BlindTaxpayer).set_bool)(&mut ri, true);
        assert_eq!(ri.header.taxpayer.blind, Some(true));
        (reg(SkippableId::BlindSpouse).set_bool)(&mut ri, false);
        assert_eq!(ri.header.spouse.as_ref().unwrap().blind, Some(false));
        assert!(Ask::Skippable(reg(SkippableId::BlindTaxpayer)).is_skippable());
    }

    /// The mandatory declarations are not skippable; the DOBs are. (Anchored to the enum shape, not a value:
    /// every `Skippable` is skippable, every `Declaration` is not.)
    #[test]
    fn only_the_skippables_are_skippable() {
        for ask in live_questions(&with_spouse(single())) {
            assert_eq!(
                ask.is_skippable(),
                ask.declaration_id().is_none(),
                "a declaration must not be skippable; a skippable must not be a declaration"
            );
            // ★ Every skippable `Ask` is a genuine entry of the CORE registry (the source of truth
            // post-move) — the prompt scope IS the `SKIPPABLE_QUESTIONS` scope, by derivation.
            if let Ask::Skippable(s) = ask {
                assert!(
                    SKIPPABLE_QUESTIONS.iter().any(|r| r.id == s.id),
                    "a skippable Ask must come from SKIPPABLE_QUESTIONS, got {:?}",
                    s.id
                );
            }
        }
    }

    /// ★ A bare Enter is an ANSWER only when there is already an answer to keep. With nothing on file it
    /// must NOT resolve — accepting silence is exactly the defect D-8 removed, walking back in through the
    /// prompt.
    #[test]
    fn a_bare_enter_never_invents_an_answer() {
        assert_eq!(parse_yes_no("", None), None, "silence is not an answer");
        assert_eq!(parse_yes_no("", Some(false)), Some(false));
        assert_eq!(parse_yes_no("", Some(true)), Some(true));
        assert_eq!(parse_yes_no("y", None), Some(true));
        assert_eq!(parse_yes_no("N", None), Some(false));
        assert_eq!(parse_yes_no("Yes", None), Some(true));
        assert_eq!(
            parse_yes_no("maybe", None),
            None,
            "garbage is not an answer"
        );
        // ...and garbage must not silently take the stored default either.
        assert_eq!(parse_yes_no("maybe", Some(true)), None);
    }

    #[test]
    fn a_dob_can_be_skipped_or_given() {
        assert_eq!(parse_date("  "), Ok(None));
        assert_eq!(
            parse_date("1960-01-02"),
            Ok(Some(time::macros::date!(1960 - 01 - 02)))
        );
        assert!(parse_date("Jan 2 1960").is_err());
    }
    // ── FR-109: the four kills ────────────────────────────────────────────────────────────────
    //
    // ★★★ Each drives the REAL `answer_return_inputs` from a keystroke script and reads the SCREEN
    //     — never a re-implementation of its loop. That is the T7 seam-review lesson: an emulation
    //     that omits the step under test is green because it never ran it (B1).

    /// The day every FR-109 fixture answers on. One constant so `record_answer`'s `answered_on` is
    /// not a moving part of the comparison.
    const FR109_DAY: time::Date = time::macros::date!(2026 - 02 - 03);

    fn fr109_params() -> FullReturnParams {
        use btctax_core::tax::tables::FullReturnTables;
        btctax_adapters::BundledFullReturnTables::load()
            .full_return_for(2024)
            .cloned()
            .expect("TY2024 params are bundled")
    }

    /// A TY2024 seed with two dependents — so the fixture covers the gates FR-105 measured (15 in
    /// one session) and the return-level declarations (33) in the same run.
    fn fr109_seed() -> ReturnInputs {
        let mut seed = single();
        seed.tax_year = 2024;
        seed.header.dependents = vec![
            btctax_core::tax::return_inputs::Dependent {
                name: "First Kid".into(),
                ssn: "000-00-1111".into(),
                relationship: "Daughter".into(),
                ..Default::default()
            },
            btctax_core::tax::return_inputs::Dependent {
                name: "Second Kid".into(),
                ssn: "000-00-2222".into(),
                relationship: "Son".into(),
                ..Default::default()
            },
        ];
        seed
    }

    /// How many questions a session actually PUT to the filer, read off the screen. Every prompt in
    /// the command — declaration, skippable in each of its three shapes, and dependent gate — ends
    /// with the same `]: ` before the cursor, and nothing else the command prints does.
    ///
    /// ★ It is cross-checked against the derived live set in
    ///   [`re_answer_puts_every_live_question_again`], so a counter that drifted from the prompts it
    ///   counts fails there rather than reporting a comfortable number here.
    fn fr109_prompts(screen: &str) -> usize {
        screen.matches("]: ").count()
    }

    /// One `income answer` session against `vault`, driven by `script`. Returns the screen.
    fn fr109_run(
        vault: &std::path::Path,
        script: &str,
        scope: AskScope,
    ) -> Result<String, CliError> {
        let mut keys = script.as_bytes();
        let mut screen: Vec<u8> = Vec::new();
        answer_return_inputs(
            vault,
            &t7_pp(),
            2024,
            FR109_DAY,
            &mut keys,
            &mut screen,
            AnswerOptions {
                discard_draft: false,
                scope,
            },
        )?;
        Ok(String::from_utf8(screen).unwrap())
    }

    /// A vault whose TY2024 draft has had every live question ANSWERED by the real command, so the
    /// answer log holds a record for each. Returns the vault and how many prompts that first pass
    /// put to the filer.
    fn fr109_answered_vault() -> (tempfile::TempDir, std::path::PathBuf, usize) {
        let (dir, vault) = t7_vault();
        let seed = fr109_seed();
        {
            let mut s = Session::open(&vault, &t7_pp()).unwrap();
            crate::input_form_store::save_draft(&mut s, 2024, &seed).unwrap();
        }
        let (script, _, _) = t7_script(&seed, Some(&fr109_params()));
        let screen = fr109_run(&vault, &script, AskScope::StillNeeded)
            .expect("every live question is scripted");
        let n = fr109_prompts(&screen);
        assert!(
            n >= 40,
            "the fixture must present a REAL session — {n} prompts is not one (FR-105 measured 33 \
             declarations plus 15 gates)"
        );
        (dir, vault, n)
    }

    /// ★★★ **KILL 1 — a session over a fully-answered return asks NOTHING, and says so.**
    ///
    /// The input is EMPTY. Before FR-109 that could only end one way: the command put its first
    /// question, `read_line` returned 0, and it exited *"input ended before every question was
    /// answered — nothing was stored"*. So the empty stdin is the kill — it cannot pass unless the
    /// command genuinely asked nothing.
    ///
    /// ★★ **B1a — the fixture is asserted to present the case.** A fixture whose questions were
    ///    somehow NOT answered would also ask nothing new for the wrong reason, so every live
    ///    question is measured `Given` or `Declined` first, from the log.
    #[test]
    fn a_fully_answered_return_asks_nothing_and_says_so() {
        let (_dir, vault, first) = fr109_answered_vault();
        let params = fr109_params();
        let ri = t7_draft(&vault, 2024);

        // B1a — every live question really is on file, and under the words asked NOW.
        let live = live_questions_with(&ri, Some(&params));
        assert!(!live.is_empty(), "the fixture has no live questions at all");
        for ask in &live {
            let key = answer_key_of(&ri, ask).expect("a live ask has a key");
            let st = answer_status(&ri, &key);
            assert!(
                matches!(st, AnswerStatus::Given | AnswerStatus::Declined),
                "{key:?} reads {st:?} after being answered — the fixture does not present the case"
            );
        }

        let screen = fr109_run(&vault, "", AskScope::StillNeeded)
            .expect("a fully-answered return must not read a single keystroke");
        assert_eq!(
            fr109_prompts(&screen),
            0,
            "a question was put to the filer over a fully-answered return:\n{screen}"
        );
        assert!(
            screen.contains("Nothing to ask:"),
            "a session that asks nothing must SAY so — two panels and silence looks broken:\n\
             {screen}"
        );
        assert!(
            screen.contains("--re-answer"),
            "…and must say how to be asked anyway:\n{screen}"
        );
        // The first pass really did ask; this is the same command on the same return.
        assert!(first >= 40, "the first pass asked {first}");
    }

    /// ★★★ **KILL 2 — `--re-answer` restores the old behaviour exactly: every live question again.**
    ///
    /// The comparison is against the FIRST pass's own prompt count on the same return, so it cannot
    /// drift with the registry: add a question and both numbers move together.
    ///
    /// ★ It also CROSS-CHECKS the screen counter against the derived live set. Without that,
    ///   `fr109_prompts` could be counting something else entirely and every FR-109 assertion would
    ///   be measuring a number nobody has tied to a prompt.
    #[test]
    fn re_answer_puts_every_live_question_again() {
        let (_dir, vault, first) = fr109_answered_vault();
        let params = fr109_params();
        let ri = t7_draft(&vault, 2024);
        let live = live_questions_with(&ri, Some(&params)).len();

        // A bare Enter keeps every answer on file, so one per live question is a complete script.
        let script = "\n".repeat(live + MAX_SWEEPS);
        let screen = fr109_run(&vault, &script, AskScope::Every)
            .expect("Enter keeps every answer that is already on file");
        let asked = fr109_prompts(&screen);
        assert_eq!(
            asked, live,
            "`--re-answer` must put every LIVE question — the screen counter and the derived live \
             set disagree, so one of them is not measuring prompts:\n{screen}"
        );
        assert_eq!(
            asked, first,
            "`--re-answer` must ask what the first pass asked: that IS the behaviour it restores"
        );
        assert!(
            !screen.contains("Nothing to ask:"),
            "a session that asked {asked} questions must not claim it asked none"
        );
        // …and the default, on the same return, asks none of them.
        let default = fr109_run(&vault, "", AskScope::StillNeeded).expect("nothing is due");
        assert_eq!(
            fr109_prompts(&default),
            0,
            "the flag is what changes the behaviour, and without it nothing is asked"
        );
    }

    /// ★★★ **KILL 3 — a `WordingChanged` answer is asked again WITHOUT the flag.**
    ///
    /// [`AnswerStatus::WordingChanged`]'s own doc is *"Treated as UNANSWERED everywhere: blocking
    /// for class (A), forgoing for class (B), and refused by `screen_inputs`."* If the FR-109 skip
    /// treated it as answered, `income answer` would become the one surface in the product that
    /// disagrees — and a filer whose question was reworded could no longer reach it from the
    /// keyboard at all, while the commit gate kept refusing. A brick, and R10.3's whole re-ask rule
    /// dead on arrival.
    ///
    /// The plant re-records ONE live declaration under words the filer was never shown, leaving the
    /// leaf's value untouched — exactly what a reworded registry prompt does to an old record.
    #[test]
    fn a_reworded_question_is_asked_again_without_the_flag() {
        let (_dir, vault, _) = fr109_answered_vault();
        let params = fr109_params();
        let mut ri = t7_draft(&vault, 2024);
        let victim = live_questions_with(&ri, Some(&params))
            .iter()
            .find_map(Ask::declaration_id)
            .expect("the fixture has a live declaration");
        let key = AnswerKey::Question(victim);

        // ── THE PLANT ──
        assert_eq!(
            answer_status(&ri, &key),
            AnswerStatus::Given,
            "the victim must start ANSWERED, or the plant proves nothing"
        );
        let before = (FORM_QUESTIONS
            .iter()
            .find(|q| q.id == victim)
            .expect("a registry question")
            .get)(&ri);
        record_answer(
            &mut ri,
            key.clone(),
            "a sentence the filer was never shown",
            FR109_DAY,
            AnswerState::Given,
        );
        // B1a — the fixture presents the case: stale record, value intact.
        assert_eq!(
            answer_status(&ri, &key),
            AnswerStatus::WordingChanged,
            "the plant did not produce a wording change"
        );
        assert_eq!(
            (FORM_QUESTIONS
                .iter()
                .find(|q| q.id == victim)
                .expect("a registry question")
                .get)(&ri),
            before,
            "the plant must leave the LEAF alone — otherwise it is an unanswered question, not a \
             reworded one"
        );
        {
            let mut s = Session::open(&vault, &t7_pp()).unwrap();
            crate::input_form_store::save_draft(&mut s, 2024, &ri).unwrap();
        }

        // ── THE MEASUREMENT — default scope, NO flag. ──
        let screen = fr109_run(&vault, "\n", AskScope::StillNeeded)
            .expect("Enter keeps the value already on file");
        assert_eq!(
            fr109_prompts(&screen),
            1,
            "exactly the reworded question must be asked, and nothing else:\n{screen}"
        );
        assert!(
            screen.contains(btctax_core::tax::provenance::WORDING_CHANGED_REASON),
            "the panel must tell the filer WHY it is asking again:\n{screen}"
        );
        assert!(
            !screen.contains("Nothing to ask:"),
            "a stale record is not nothing to ask"
        );
        // …and answering it under the current words settles it.
        assert_eq!(
            answer_status(&t7_draft(&vault, 2024), &key),
            AnswerStatus::Given,
            "the re-ask must have re-recorded against the words on the screen"
        );
    }

    /// ★★★ **KILL 4 — an IMPORTED leaf is asked once, and not twice.**
    ///
    /// `income import` writes values and no records, so every leaf reads answered while
    /// `answer_status` reads `NeverAsked` (FR-109's own question, and FR-105's refutation measured
    /// exactly this state). The skip rule asks it, which is what stamps the record — so the second
    /// session skips it. **That convergence is the point:** skipping a recordless leaf outright
    /// would honour the owner's ruling and leave the value with no prompt hash forever, which is
    /// R10.3's re-ask rule permanently unable to fire for anything a filer imported.
    ///
    /// The fixture is built by clearing the log off an answered draft — the imported shape, derived
    /// rather than hand-typed.
    #[test]
    fn an_imported_leaf_is_asked_once_and_not_twice() {
        let (_dir, vault, first) = fr109_answered_vault();
        let params = fr109_params();
        let mut ri = t7_draft(&vault, 2024);
        ri.answer_log.clear();
        ri.answer_log_history.clear();
        {
            let mut s = Session::open(&vault, &t7_pp()).unwrap();
            crate::input_form_store::save_draft(&mut s, 2024, &ri).unwrap();
        }

        // B1a — the fixture presents the case: values everywhere, records nowhere.
        let ri = t7_draft(&vault, 2024);
        assert!(
            ri.answer_log.is_empty(),
            "the imported shape has no records"
        );
        let live = live_questions_with(&ri, Some(&params));
        assert!(!live.is_empty());
        for ask in &live {
            let key = answer_key_of(&ri, ask).expect("a live ask has a key");
            assert_eq!(
                answer_status(&ri, &key),
                AnswerStatus::NeverAsked,
                "{key:?} must read NeverAsked in the imported shape"
            );
        }

        // ── PASS ONE — asked, exactly as a first pass is. ──
        let script = "\n".repeat(live.len() + MAX_SWEEPS);
        let one = fr109_run(&vault, &script, AskScope::StillNeeded)
            .expect("Enter keeps every imported value");
        assert_eq!(
            fr109_prompts(&one),
            first,
            "an imported return must be put through the whole set once — that is what gives its \
             values a prompt hash:\n{one}"
        );

        // ── PASS TWO — nothing, because pass one recorded. ──
        let two = fr109_run(&vault, "", AskScope::StillNeeded)
            .expect("the second pass must not read a keystroke");
        assert_eq!(
            fr109_prompts(&two),
            0,
            "an imported leaf must be asked ONCE, not every session:\n{two}"
        );
        assert!(two.contains("Nothing to ask:"));
    }
    /// ★★★ **KILL 5 — the LEAF conjunct, watched discriminating.**
    ///
    /// [`needs_asking`] does not read the log alone: for a class-(A) ask it also requires the leaf
    /// to hold what the record says was given. A `Given` record standing over an EMPTY class-(A)
    /// leaf is a record of testimony the return does not carry, and skipping there is the one
    /// outcome worse than re-asking — `screen_inputs` refuses the commit, R12 lists the question as
    /// blocking, and the command that exists to fix it declines to ask. A brick.
    ///
    /// The three rows below are the whole rule, and the last two are what stop the conjunct from
    /// simply reding on everything: **fail-closed on the claim, fail-OPEN on the interview.**
    #[test]
    fn a_record_standing_over_an_empty_class_a_leaf_is_still_asked() {
        let params = fr109_params();

        // ── (a) a DECLARATION: `Given` record, empty leaf ⇒ ASK. ──
        let mut ri = fr109_seed();
        let ask = live_questions_with(&ri, Some(&params))
            .into_iter()
            .find(|a| matches!(a, Ask::Declaration(_)))
            .expect("the seed has a live declaration");
        let Ask::Declaration(q) = &ask else {
            unreachable!("filtered above")
        };
        assert!(
            (q.get)(&ri).is_none(),
            "the fixture must present an EMPTY leaf, or it proves nothing"
        );
        let key = AnswerKey::Question(q.id);
        let words = q.prompt_text(&ri).into_owned();
        record_answer(&mut ri, key.clone(), &words, FR109_DAY, AnswerState::Given);
        assert_eq!(
            answer_status(&ri, &key),
            AnswerStatus::Given,
            "the fixture must present a record the log calls ANSWERED"
        );
        assert!(
            needs_asking(&ri, &ask),
            "a `Given` record over an empty class-(A) leaf must still be asked — skipping it is a \
             brick: the commit refuses and the command that fixes it will not ask"
        );

        // ── (b) the NEAR MISS — same record, leaf now filled ⇒ SKIP. Without this the conjunct
        //       reds on everything and FR-109 buys nothing. ──
        (q.set)(&mut ri, q.neutral);
        let words = q.prompt_text(&ri).into_owned();
        record_answer(&mut ri, key.clone(), &words, FR109_DAY, AnswerState::Given);
        assert!(
            !needs_asking(&ri, &ask),
            "an answered leaf with a matching record is exactly what FR-109 skips"
        );

        // ── (c) a SKIPPABLE takes NO leaf test: an empty leaf IS its lawful answer. ──
        let mut ri = fr109_seed();
        let sk_ask = live_questions_with(&ri, Some(&params))
            .into_iter()
            .find(|a| matches!(a, Ask::Skippable(_)))
            .expect("the seed has a live skippable");
        let Ask::Skippable(sk) = &sk_ask else {
            unreachable!("filtered above")
        };
        record_answer(
            &mut ri,
            AnswerKey::Skippable(sk.id),
            sk.prompt,
            FR109_DAY,
            AnswerState::Declined,
        );
        assert!(
            !needs_asking(&ri, &sk_ask),
            "`Declined` is provenance — asked and passed over — and R12 still lists the benefit as \
             forgone, so re-asking it every session is the behaviour FR-109 removes"
        );
    }
}
