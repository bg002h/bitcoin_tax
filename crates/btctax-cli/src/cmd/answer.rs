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
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_store::Passphrase;
use std::io::Write;
use std::path::Path;

/// A question `income answer` may ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// 1040 "Someone can claim YOU as a dependent" — asked on every return.
    DependentTaxpayer,
    /// 1040 "Someone can claim YOUR SPOUSE as a dependent" — only when a spouse is on the return.
    DependentSpouse,
    /// Schedule B Part III line 7a (foreign accounts) — only when Schedule B files.
    ForeignAccounts,
    /// Schedule B Part III line 8 (foreign trust) — only when Schedule B files.
    ForeignTrust,
    /// §63(c)(6) — only on MFS, where it couples the spouses' std/itemize choice.
    MfsSpouseItemizes,
    /// §63(f) aged addition. SKIPPABLE — see [`Question::is_skippable`].
    DateOfBirthTaxpayer,
    /// §63(f) aged addition for the spouse — only when a spouse is on the return. Skippable.
    DateOfBirthSpouse,
}

impl Question {
    /// The prompt text — phrased as the FORM phrases it. The user is answering a 1040 line, not a struct
    /// field, and the words printed on the form are the ones they can check against their own paperwork.
    pub fn prompt(self) -> &'static str {
        match self {
            Self::DependentTaxpayer => "Can someone claim YOU as a dependent on their return?",
            Self::DependentSpouse => "Can someone claim YOUR SPOUSE as a dependent on their return?",
            Self::ForeignAccounts => {
                "Schedule B line 7a: did you have a financial interest in, or signature authority over, \
                 a FOREIGN financial account?"
            }
            Self::ForeignTrust => {
                "Schedule B line 8: did you receive a distribution from — or were you the grantor of, or \
                 transferor to — a FOREIGN TRUST?"
            }
            Self::MfsSpouseItemizes => {
                "Does your spouse ITEMIZE deductions on their separate return? (§63(c)(6) forces your \
                 choice to match theirs)"
            }
            Self::DateOfBirthTaxpayer => "YOUR date of birth",
            Self::DateOfBirthSpouse => "YOUR SPOUSE's date of birth",
        }
    }

    /// Whether this is a date question (rather than a yes/no).
    pub fn is_date(self) -> bool {
        matches!(self, Self::DateOfBirthTaxpayer | Self::DateOfBirthSpouse)
    }

    /// ★ The DOB prompts are SKIPPABLE; the tri-states are NOT.
    ///
    /// A *mandatory* date-of-birth prompt is a forcing function to INVENT a value — and an invented-old
    /// birthday grants the §63(f) aged addition, which understates tax. `None` is the safe state, and the
    /// prompt must permit it. The tri-states are the exact opposite: their whole purpose is that silence
    /// is not an answer.
    pub fn is_skippable(self) -> bool {
        self.is_date()
    }
}

/// ★ EXACTLY the questions this return needs answered — scoped the same way the REFUSALS are scoped.
///
/// Asking a Single filer about a spouse who does not exist is the prompt-level twin of the refusal-level
/// bug D-8 fixed (a spouse flag demanded of a return that has no spouse). The two scopes must agree, so
/// both read the same predicates: `header.spouse.is_some()`, `schedule_b_files`, `filing_status == Mfs`.
///
/// **Every live question is asked in ONE pass** — including ones already answered, whose current value is
/// offered as the default. Once the screen-before-store gate lands, a blob the screen refuses cannot be
/// stored; so answering only *some* questions would leave the return still-refused and therefore still
/// unstorable, and the user could never answer the rest. Asking everything at once is what prevents that
/// deadlock.
pub fn live_questions(ri: &ReturnInputs) -> Vec<Question> {
    let mut qs = vec![Question::DependentTaxpayer];
    let has_spouse = ri.header.spouse.is_some();
    if has_spouse {
        qs.push(Question::DependentSpouse);
    }
    if btctax_core::tax::return_1040::schedule_b_files(ri) {
        qs.push(Question::ForeignAccounts);
        qs.push(Question::ForeignTrust);
    }
    if ri.filing_status == btctax_core::FilingStatus::Mfs {
        qs.push(Question::MfsSpouseItemizes);
    }
    qs.push(Question::DateOfBirthTaxpayer);
    if has_spouse {
        qs.push(Question::DateOfBirthSpouse);
    }
    qs
}

/// The yes/no currently on file for `q`, so the prompt can offer it as the default (Enter keeps it).
pub fn current_bool(ri: &ReturnInputs, q: Question) -> Option<bool> {
    match q {
        Question::DependentTaxpayer => ri.header.can_be_claimed_as_dependent_taxpayer,
        Question::DependentSpouse => ri.header.can_be_claimed_as_dependent_spouse,
        Question::ForeignAccounts => ri.foreign_accounts,
        Question::ForeignTrust => ri.foreign_trust,
        Question::MfsSpouseItemizes => ri.mfs_spouse_itemizes,
        Question::DateOfBirthTaxpayer | Question::DateOfBirthSpouse => None,
    }
}

/// The date currently on file for `q`.
pub fn current_date(ri: &ReturnInputs, q: Question) -> Option<time::Date> {
    match q {
        Question::DateOfBirthTaxpayer => ri.header.taxpayer.date_of_birth,
        Question::DateOfBirthSpouse => ri.header.spouse.as_ref().and_then(|s| s.date_of_birth),
        _ => None,
    }
}

/// Record a yes/no answer.
pub fn set_bool(ri: &mut ReturnInputs, q: Question, v: bool) {
    match q {
        Question::DependentTaxpayer => ri.header.can_be_claimed_as_dependent_taxpayer = Some(v),
        Question::DependentSpouse => ri.header.can_be_claimed_as_dependent_spouse = Some(v),
        Question::ForeignAccounts => ri.foreign_accounts = Some(v),
        Question::ForeignTrust => ri.foreign_trust = Some(v),
        Question::MfsSpouseItemizes => ri.mfs_spouse_itemizes = Some(v),
        Question::DateOfBirthTaxpayer | Question::DateOfBirthSpouse => {
            debug_assert!(false, "{q:?} is a date, not a yes/no")
        }
    }
}

/// Record a date-of-birth answer.
pub fn set_date(ri: &mut ReturnInputs, q: Question, v: time::Date) {
    match q {
        Question::DateOfBirthTaxpayer => ri.header.taxpayer.date_of_birth = Some(v),
        Question::DateOfBirthSpouse => {
            if let Some(sp) = ri.header.spouse.as_mut() {
                sp.date_of_birth = Some(v);
            }
        }
        _ => debug_assert!(false, "{q:?} is a yes/no, not a date"),
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

    for q in live_questions(&ri) {
        if q.is_date() {
            let cur = current_date(&ri, q);
            loop {
                let shown = cur.map_or_else(|| "none".to_string(), |d| d.to_string());
                write!(out, "{} [{}; Enter to skip]: ", q.prompt(), shown)?;
                out.flush()?;
                let mut line = String::new();
                if input.read_line(&mut line)? == 0 {
                    return Err(CliError::Usage(
                        "input ended before every question was answered — nothing was stored".into(),
                    ));
                }
                match parse_date(&line) {
                    // A bare Enter KEEPS whatever is on file (which may be `None`). It never CLEARS a date
                    // the user already gave us — "skip" must not be a destructive default.
                    Ok(None) => break,
                    Ok(Some(d)) => {
                        set_date(&mut ri, q, d);
                        break;
                    }
                    Err(e) => writeln!(out, "  not a date (YYYY-MM-DD): {e}")?,
                }
            }
        } else {
            let cur = current_bool(&ri, q);
            loop {
                let shown = match cur {
                    Some(true) => "y/n, currently y",
                    Some(false) => "y/n, currently n",
                    None => "y/n",
                };
                write!(out, "{} [{}]: ", q.prompt(), shown)?;
                out.flush()?;
                let mut line = String::new();
                if input.read_line(&mut line)? == 0 {
                    return Err(CliError::Usage(
                        "input ended before every question was answered — nothing was stored".into(),
                    ));
                }
                match parse_yes_no(&line, cur) {
                    Some(v) => {
                        set_bool(&mut ri, q, v);
                        break;
                    }
                    // ★ No default and no answer ⇒ ASK AGAIN. This is the one place the whole feature
                    // turns on: accepting silence here would reintroduce D-8 through the front door.
                    None => writeln!(out, "  please answer y or n")?,
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

    /// A Single filer with no Schedule B is asked TWO things: the dependent flag, and (skippably) a DOB.
    /// Nothing about a spouse who does not exist.
    #[test]
    fn a_single_filer_is_not_asked_about_a_spouse() {
        assert_eq!(
            live_questions(&single()),
            vec![Question::DependentTaxpayer, Question::DateOfBirthTaxpayer]
        );
    }

    /// ★ The prompt scope must track the REFUSAL scope. A spouse question asked of a spouse-less return is
    /// the prompt-level twin of the refusal-level bug D-8 fixed.
    #[test]
    fn spouse_questions_appear_exactly_when_a_spouse_does() {
        let qs = live_questions(&with_spouse(single()));
        assert!(qs.contains(&Question::DependentSpouse));
        assert!(qs.contains(&Question::DateOfBirthSpouse));
        for q in live_questions(&single()) {
            assert!(
                !matches!(q, Question::DependentSpouse | Question::DateOfBirthSpouse),
                "no spouse ⇒ no spouse question, got {q:?}"
            );
        }
    }

    /// Schedule-B Part III is asked only when Schedule B actually files — the same predicate the refusal
    /// uses, so `answer` can always clear the refusal it is there to clear.
    #[test]
    fn schedule_b_questions_appear_exactly_when_schedule_b_files() {
        assert!(!live_questions(&single()).contains(&Question::ForeignAccounts));
        let mut files = single();
        files.int_1099.push(Form1099Int {
            box1_interest: dec!(2000), // > the $1,500 Schedule-B threshold
            ..Default::default()
        });
        let qs = live_questions(&files);
        assert!(qs.contains(&Question::ForeignAccounts));
        assert!(qs.contains(&Question::ForeignTrust));
    }

    #[test]
    fn mfs_is_asked_whether_the_spouse_itemizes() {
        let mut mfs = single();
        mfs.filing_status = FilingStatus::Mfs;
        assert!(live_questions(&mfs).contains(&Question::MfsSpouseItemizes));
        assert!(!live_questions(&single()).contains(&Question::MfsSpouseItemizes));
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
        for q in live_questions(&ri) {
            if q.is_date() {
                continue; // skippable by design
            }
            set_bool(&mut ri, q, false);
        }
        assert!(
            screen_inputs(&ri, table, params).is_none(),
            "answering every LIVE question must clear the screen — if it does not, `answer` cannot \
             rescue a bricked year and the whole command is a dead end"
        );
    }

    /// The tri-states are not skippable; the DOBs are.
    #[test]
    fn only_the_dobs_are_skippable() {
        for q in live_questions(&with_spouse(single())) {
            assert_eq!(q.is_skippable(), q.is_date(), "{q:?}");
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
