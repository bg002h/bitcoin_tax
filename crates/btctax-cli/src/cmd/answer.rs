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
    let mut asks: Vec<Ask> = FORM_QUESTIONS
        .iter()
        .filter(|q| (q.live)(ri))
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
pub fn answer_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    input: &mut impl std::io::BufRead,
    out: &mut impl Write,
) -> Result<(), CliError> {
    let mut s = Session::open(vault, pp)?;
    // ★ §6.2 (M-1): reconcile the draft BEFORE the committed-row read below — that read early-returns a
    // generic "no inputs" message on an absent row, and a PARKED year has no committed row, so running
    // coherence first is what surfaces the parked-refuse remedy instead of the generic message.
    crate::input_form_store::coherence_clear_or_refuse(s.conn(), year)?;
    let Some(mut ri) = return_inputs::get(s.conn(), year)? else {
        return Err(CliError::Usage(format!(
            "no full-return inputs for tax year {year} — `income answer` fills in the questions on an \
             EXISTING return; create one first with `btctax income import --year {year} --file <toml>`"
        )));
    };

    // ★ r3 NIT-2 — the questions say "in this tax year" but the registry prompts are `&'static str` and
    // cannot interpolate the year; a one-line banner anchors them so the filer need not hold it in their head.
    writeln!(out, "Answering full-return questions for tax year {year}:")?;

    for ask in live_questions(&ri) {
        match ask {
            // A MANDATORY declaration — silence with nothing on file is refused, never accepted (D-8).
            Ask::Declaration(q) => {
                let cur = (q.get)(&ri);
                loop {
                    let shown = match cur {
                        Some(true) => "y/n, currently y",
                        Some(false) => "y/n, currently n",
                        None => "y/n",
                    };
                    write!(out, "{} [{}]: ", q.prompt, shown)?;
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
                    loop {
                        let shown = cur.map_or_else(|| "none".to_string(), |d| d.to_string());
                        write!(out, "{} [{}; Enter to skip]: ", sk.prompt, shown)?;
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
            },
        }
    }

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

    /// A Single filer is asked the FIVE always-live declarations (dependent-taxpayer, both foreign
    /// questions, HSA activity, dual-status) and — skippably — a DOB. Nothing about a spouse who does not
    /// exist, and (post-§2.9) the foreign questions appear even with no Schedule B.
    #[test]
    fn a_single_filer_is_asked_the_always_live_declarations_and_no_spouse_question() {
        assert_eq!(
            declaration_ids(&single()),
            vec![
                QuestionId::DependentTaxpayer,
                QuestionId::ForeignAccounts,
                QuestionId::ForeignTrust,
                QuestionId::HsaActivity,
                QuestionId::DualStatusAlien,
            ]
        );
        assert!(!has_spouse_dob(&single()), "no spouse ⇒ no spouse DOB");
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
        for ask in live_questions(&ri) {
            match ask {
                Ask::Declaration(q) => (q.set)(&mut ri, false), // answer "no"
                Ask::Skippable(_) => {}                         // skippable by design
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
            QuestionId::MortgageAllUsedToBuyBuildImprove => {
                r.schedule_a = Some(ScheduleAInputs {
                    mortgage_interest_1098: dec!(9000),
                    ..Default::default()
                });
            }
            _ => {}
        }
        r
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
