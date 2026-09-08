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
use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
use btctax_core::tax::questions::{
    FormQuestion, QuestionId, SkippableKind, SkippableQuestion, FORM_QUESTIONS, SKIPPABLE_QUESTIONS,
};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_store::Passphrase;
use std::io::Write;
use std::path::Path;

/// One thing `income answer` asks: a MANDATORY declaration (from the [`FORM_QUESTIONS`] registry — a bare
/// Enter with nothing on file is refused, never accepted as an answer) or a SKIPPABLE prompt (from the
/// core [`SKIPPABLE_QUESTIONS`] registry — a bare Enter leaves `None`, forgoing the benefit lawfully).
pub enum Ask {
    Declaration(&'static FormQuestion),
    Skippable(&'static SkippableQuestion),
}

impl Ask {
    /// The registry `QuestionId` if this is a declaration — for tests that assert WHICH questions are live.
    pub fn declaration_id(&self) -> Option<QuestionId> {
        match self {
            Ask::Declaration(q) => Some(q.id),
            Ask::Skippable(_) => None,
        }
    }
    /// Whether a bare Enter with nothing on file is a legitimate outcome. True for skippables (DOBs), false
    /// for declarations — silence on a declaration is exactly what D-8 forbids.
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
    asks.extend(
        SKIPPABLE_QUESTIONS
            .iter()
            .filter(|s| (s.live)(ri))
            .map(Ask::Skippable),
    );
    asks
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
    writeln!(out, "\n── The answer panel ({when}) ──")?;
    if st.open_items() == 0 {
        writeln!(
            out,
            "  nothing is open: every live question is answered, nothing is forgone, and no answer \
             refuses."
        )?;
    }
    if !st.blocking.is_empty() {
        writeln!(
            out,
            "  BLOCKING ({}) — commit waits on these:",
            st.blocking.len()
        )?;
        for b in &st.blocking {
            writeln!(out, "    • {} [{}]", b.prompt, b.reason)?;
        }
    }
    if !st.refusing.is_empty() {
        writeln!(
            out,
            "  REFUSING ({}) — an answer already given that stops the return:",
            st.refusing.len()
        )?;
        for r in &st.refusing {
            writeln!(out, "    • {}", r.exit)?;
        }
    }
    if !st.forgoing.is_empty() {
        writeln!(
            out,
            "  FORGOING ({}) — lawful to skip; each one costs YOU, not the Treasury:",
            st.forgoing.len()
        )?;
        for f in &st.forgoing {
            let mark = if f.declined { " (declined)" } else { "" };
            // ★ Plain `$N` rather than `advisories::fmt_usd`, which is `pub(crate)` to core.
            let size = f
                .size
                .map_or_else(String::new, |s| format!(" — up to ${s}"));
            writeln!(out, "    • {}{mark}{size}", f.prompt)?;
        }
    }
    if !st.waiting.is_empty() {
        writeln!(
            out,
            "  WAITING ({}) — these cannot be asked until a year package arrives:",
            st.waiting.len()
        )?;
        for w in &st.waiting {
            writeln!(out, "    • {} [waiting on {}]", w.prompt, w.waiting_on)?;
        }
    }
    writeln!(
        out,
        "  ({} answered, {} not applicable to this return)",
        st.answered, st.not_live
    )?;
    Ok(())
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
    discard_draft: bool,
) -> Result<(), CliError> {
    let mut s = Session::open(vault, pp)?;
    let target = answer_target(&s, year, discard_draft)?;
    let (mut ri, into_draft, coherence) = match target {
        AnswerTarget::Committed { ri, coherence } => (ri, false, coherence),
        AnswerTarget::Draft(ri) => (ri, true, crate::input_form_store::DraftCoherence::Absent),
    };

    // ★ r3 NIT-2 — the questions say "in this tax year" but the registry prompts are `&'static str` and
    // cannot interpolate the year; a one-line banner anchors them so the filer need not hold it in their head.
    writeln!(out, "Answering full-return questions for tax year {year}:")?;
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
    write_panel(
        out,
        &btctax_core::tax::interview_state::interview_state(&ri),
        "before",
    )?;

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
    let mut asked: std::collections::BTreeSet<AnswerKey> = std::collections::BTreeSet::new();
    let key_of = |a: &Ask| match a {
        Ask::Declaration(q) => AnswerKey::Question(q.id),
        Ask::Skippable(sk) => AnswerKey::Skippable(sk.id),
    };
    const MAX_SWEEPS: usize = 8;
    let mut sweeps = 0usize;
    loop {
        let round: Vec<Ask> = live_questions(&ri)
            .into_iter()
            .filter(|a| !asked.contains(&key_of(a)))
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
            };
            if !still_live {
                continue;
            }
            asked.insert(key_of(&ask));
            match ask {
                // A MANDATORY declaration — silence with nothing on file is refused, never accepted (D-8).
                Ask::Declaration(q) => {
                    let cur = (q.get)(&ri);
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

    // ★★★ R12 / §4.2 — THE PANEL, AFTER the last question. Printed BEFORE the write, so a filer
    //     whose session ends in a refusing answer sees it while they are still at the keyboard.
    write_panel(
        out,
        &btctax_core::tax::interview_state::interview_state(&ri),
        "after",
    )?;

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
        for _ in 0..8 {
            let round: Vec<Ask> = live_questions(&ri)
                .into_iter()
                .filter(|a| a.declaration_id().is_none_or(|id| !asked.contains(&id)))
                .collect();
            if round.is_empty() {
                break;
            }
            for ask in round {
                if let Some(id) = ask.declaration_id() {
                    asked.insert(id);
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
                }
            }
        }
        assert!(
            screen_inputs(&ri, table, params).is_none(),
            "answering every LIVE declaration must clear the screen — if it does not, `answer` cannot \
             rescue a bricked year and the whole command is a dead end"
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
            QuestionId::MortgageAllUsedToBuyBuildImprove
            | QuestionId::AmtQualifiedDwelling
            | QuestionId::MortgageWithinDebtLimit => {
                r.schedule_a = Some(ScheduleAInputs {
                    mortgage_interest_1098: dec!(9000),
                    ..Default::default()
                });
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
            // ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. Each of the three paired questions is
            //     live EXACTLY when its census row says `No` — the pairing R3 states — so the
            //     scenario answers that row and nothing else.
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
}
