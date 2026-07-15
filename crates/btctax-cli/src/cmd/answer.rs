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
use btctax_core::tax::questions::{FormQuestion, QuestionId, FORM_QUESTIONS};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_store::Passphrase;
use std::io::Write;
use std::path::Path;

/// A SKIPPABLE prompt `income answer` asks in addition to the mandatory registry declarations. Skippable
/// means a bare Enter leaves the value `None` — the opposite of a declaration, where silence is refused
/// (D-8). Only the §63(f) dates of birth for now; the class-(B) blind/SALT skippables join in a later step
/// (bundled with the advisories that make skipping them meaningful).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skippable {
    /// §63(f) aged addition. A mandatory DOB prompt would force the user to INVENT a birthday, and an
    /// invented-old one grants the aged addition and understates tax — so `None` must stay reachable.
    DateOfBirthTaxpayer,
    /// §63(f) aged addition for the spouse — only when a spouse `Person` is on the return (a `set_date` on
    /// an absent spouse is silently discarded, so the prompt is gated to match, r3 I-7).
    DateOfBirthSpouse,
}

impl Skippable {
    /// The prompt text — phrased as the FORM phrases it.
    pub fn prompt(self) -> &'static str {
        match self {
            Self::DateOfBirthTaxpayer => "YOUR date of birth",
            Self::DateOfBirthSpouse => "YOUR SPOUSE's date of birth",
        }
    }
    /// The date currently on file (offered as the default; Enter keeps it).
    pub fn current_date(self, ri: &ReturnInputs) -> Option<time::Date> {
        match self {
            Self::DateOfBirthTaxpayer => ri.header.taxpayer.date_of_birth,
            Self::DateOfBirthSpouse => ri.header.spouse.as_ref().and_then(|s| s.date_of_birth),
        }
    }
    /// Record a date-of-birth answer. A spouse DOB on a return with no spouse `Person` is silently
    /// discarded — which is exactly why `live_questions` gates the spouse DOB prompt on `spouse.is_some()`.
    pub fn set_date(self, ri: &mut ReturnInputs, v: time::Date) {
        match self {
            Self::DateOfBirthTaxpayer => ri.header.taxpayer.date_of_birth = Some(v),
            Self::DateOfBirthSpouse => {
                if let Some(sp) = ri.header.spouse.as_mut() {
                    sp.date_of_birth = Some(v);
                }
            }
        }
    }
}

/// One thing `income answer` asks: a MANDATORY declaration (from the [`FORM_QUESTIONS`] registry — a bare
/// Enter with nothing on file is refused, never accepted as an answer) or a SKIPPABLE prompt.
pub enum Ask {
    Declaration(&'static FormQuestion),
    Skippable(Skippable),
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
    asks.push(Ask::Skippable(Skippable::DateOfBirthTaxpayer));
    if ri.header.spouse.is_some() {
        asks.push(Ask::Skippable(Skippable::DateOfBirthSpouse));
    }
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
    time::Date::parse(t, fmt).map(Some).map_err(|e| e.to_string())
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
    let Some(mut ri) = return_inputs::get(s.conn(), year)? else {
        return Err(CliError::Usage(format!(
            "no full-return inputs for tax year {year} — `income answer` fills in the questions on an \
             EXISTING return; create one first with `btctax income import --year {year} --file <toml>`"
        )));
    };

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
                            "input ended before every question was answered — nothing was stored".into(),
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
            // A SKIPPABLE prompt (DOB) — a bare Enter KEEPS whatever is on file (which may be `None`).
            Ask::Skippable(sk) => {
                let cur = sk.current_date(&ri);
                loop {
                    let shown = cur.map_or_else(|| "none".to_string(), |d| d.to_string());
                    write!(out, "{} [{}; Enter to skip]: ", sk.prompt(), shown)?;
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
                            sk.set_date(&mut ri, d);
                            break;
                        }
                        Err(e) => writeln!(out, "  not a date (YYYY-MM-DD): {e}")?,
                    }
                }
            }
        }
    }

    return_inputs::set(s.conn(), year, &ri)?;
    s.save()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        live_questions(ri).iter().filter_map(Ask::declaration_id).collect()
    }
    fn has_spouse_dob(ri: &ReturnInputs) -> bool {
        live_questions(ri)
            .iter()
            .any(|a| matches!(a, Ask::Skippable(Skippable::DateOfBirthSpouse)))
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
                Ask::Skippable(_) => {}                          // skippable by design
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
        assert_eq!(parse_yes_no("maybe", None), None, "garbage is not an answer");
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
