//! **T4b — the year-N+1 opener (`SPEC_interview.md` R10 part 4, §7 row T4b, §6 J-27).**
//!
//! R10.4 in one sentence: *"Year N+1 opens with year N's identities as fresh tri-state prompts,
//! every box blank, every `PerYear` gate re-asked blank, every `Durable` fact shown and confirmed by
//! the same keystroke a fresh answer takes, and carryforwards arriving as DATA — never a carried
//! amount."*
//!
//! Every kill below is R10.4's, plus the ones §7 row T4b adds. The two that carry the design are:
//!
//! - **no `Usd` leaf of the seeded draft is non-zero except the carryforwards** — asserted by
//!   WALKING the leaves with T1's own type-driven money detector
//!   (`provenance::leaf_walk::nonzero_money_leaves`), never a hand-list, so a money box added to
//!   `ReturnInputs` tomorrow is examined the day it is added; and
//! - **the carryforwards are read off year N's FROZEN RETURN**, planted here as a year-N row whose
//!   stored `capital_loss_carryforward_in` differs from what its return computes OUT.

use btctax_cli::open_next_year::open_next_year;
use btctax_cli::{cmd, input_form_store, return_inputs, Session};
use btctax_core::tax::document_census::DocumentRow;
use btctax_core::tax::provenance::leaf_walk::nonzero_money_leaves;
use btctax_core::tax::return_inputs::{
    CarryProvenance, Dependent, Form1099Int, Owner, Person, ReturnInputs, W2,
};
use btctax_core::FilingStatus;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::PathBuf;

/// The one year v1 has full-return params for — so it is the only year whose FROZEN RETURN exists,
/// and therefore the only `--from` whose carryforward chain can be computed at all.
const FROM: i32 = 2024;
const TO: i32 = 2025;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn fresh_vault() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    (dir, vault)
}

/// A committed year-`FROM` return, shaped by `mutate` and then answered (the census is reconciled to
/// whatever rows `mutate` added, so the row screens clean).
fn vault_with_year_n(mutate: impl FnOnce(&mut ReturnInputs)) -> (tempfile::TempDir, PathBuf) {
    let (dir, vault) = fresh_vault();
    let mut ri = ReturnInputs {
        tax_year: FROM,
        filing_status: FilingStatus::Single,
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "Alex".into(),
        last_name: "Filer".into(),
        ssn: "123456789".into(),
        date_of_birth: Some(time::macros::date!(1980 - 05 - 05)),
        occupation: "Cooper".into(),
        ..Default::default()
    };
    ri.header.address_street = "1 Main St".into();
    ri.header.address_city = "Town".into();
    ri.header.address_state = "TX".into();
    ri.header.address_zip = "77001".into();
    ri.header.ip_pin = Some("123456".into());
    mutate(&mut ri);
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    let mut s = Session::open(&vault, &pp()).unwrap();
    return_inputs::set(s.conn(), FROM, &ri).unwrap();
    s.save().unwrap();
    (dir, vault)
}

/// ★★★ **Year N with MONEY IN IT, everywhere a return can hold money** — wages and their boxes, two
/// interest payers, a dividend payer, a state-refund payer, a Schedule A, Schedule 1 deductions,
/// estimated payments, and a $60,000 long-term loss carried in.
///
/// ★★ **The richness is the test.** *"No `Usd` leaf of the seed is non-zero"* is vacuously true of a
///    year N that had no figures: a plant that copies `payments` forward passed against the first
///    version of this fixture, because the fixture's `payments` were all zero. The floor assertion in
///    [`no_money_leaf_of_the_seed_is_nonzero_except_the_carryforwards`] is what keeps that visible.
///
/// ★ The §1211(b) $3,000 is absorbed against the ordinary income, so the return's carryover-OUT is
///   **$57,000** — a figure that differs from the stored carryover-IN, which is what makes "read off
///   the RETURN" testable at all.
fn a_year_with_money_in_it(ri: &mut ReturnInputs) {
    use btctax_core::tax::return_inputs::{
        Form1099Div, Form1099G, Payments, Schedule1Inputs, ScheduleAInputs,
    };
    ri.w2s = vec![W2 {
        owner: Owner::Taxpayer,
        employer: "Acme Tooling".into(),
        ein: Some("12-3456789".into()),
        box1_wages: dec!(30000),
        box2_fed_withheld: dec!(2400),
        box3_ss_wages: dec!(30000),
        box4_ss_withheld: dec!(1860),
        box5_medicare_wages: dec!(30000),
        box6_medicare_withheld: dec!(435),
        box17_state_tax_withheld: dec!(900),
        ..Default::default()
    }];
    ri.int_1099 = vec![
        Form1099Int {
            payer: "First Bank".into(),
            payer_tin: "11-1111111".into(),
            box1_interest: dec!(120),
            ..Default::default()
        },
        Form1099Int {
            payer: "Second Bank".into(),
            payer_tin: "22-2222222".into(),
            box1_interest: dec!(240),
            ..Default::default()
        },
    ];
    ri.div_1099 = vec![Form1099Div {
        payer: "Index Fund".into(),
        payer_tin: "33-3333333".into(),
        box1a_ordinary: dec!(500),
        box1b_qualified: dec!(400),
        ..Default::default()
    }];
    ri.g_1099 = vec![Form1099G {
        payer: "State of TX".into(),
        payer_tin: "44-4444444".into(),
        box1_unemployment: dec!(1000),
        ..Default::default()
    }];
    ri.schedule_a = Some(ScheduleAInputs {
        medical: dec!(2000),
        salt_real_estate: dec!(3500),
        salt_personal_property: dec!(200),
        investment_interest: dec!(50),
        ..Default::default()
    });
    // ★ No `ira_deduction_claimed`: a claimed IRA deduction is refused in v1 (the active-participant
    //   phase-out worksheet is unmodelled), and this fixture has to COMPUTE.
    ri.sch1 = Schedule1Inputs {
        state_refund_taxable: dec!(310),
        ..Default::default()
    };
    // ★ T5 — the §221 student-loan interest that used to be the `sch1` scalar is now a Form 1098-E
    //   ROW, so this fixture carries the document the figure came off. The opener does NOT seed
    //   loan servicers (it seeds W-2 employers, 1099 payers and venues), which is asserted by
    //   `document_census::tests::every_kind_with_a_section_drops_its_pre_named_rows`.
    ri.form_1098e = vec![btctax_core::tax::return_inputs::Form1098E {
        lender: "Example Student Loan Servicing".into(),
        lender_tin: "10-1010101".into(),
        box1_interest: dec!(600),
        ..Default::default()
    }];
    ri.payments = Payments {
        estimated_tax_payments: dec!(1500),
        extension_payment: dec!(250),
        ..Default::default()
    };
    ri.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
        short: rust_decimal::Decimal::ZERO,
        long: dec!(60000),
    };
}

/// The seeded year-`TO` draft, read back out of the vault.
fn draft(vault: &std::path::Path) -> ReturnInputs {
    let s = Session::open(vault, &pp()).unwrap();
    match input_form_store::load(s.conn(), TO).unwrap().0 {
        input_form_store::Loaded::Draft { ri, .. } => ri,
        other => panic!(
            "the opener writes a DRAFT for {TO}; found {}",
            match other {
                input_form_store::Loaded::Committed(_) => "a committed row",
                _ => "nothing",
            }
        ),
    }
}

fn open(vault: &std::path::Path, discard_draft: bool) -> btctax_cli::open_next_year::Opened {
    let mut s = Session::open(vault, &pp()).unwrap();
    open_next_year(&mut s, FROM, discard_draft).unwrap()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// R10.4's own kill — the identity prompts, and the blank screen behind them
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **R10.4's kill, verbatim: a fixture year N with two payers yields exactly two identity
///     prompts, both `None`, and the opened screen's boxes are all default.**
#[test]
fn two_payers_yield_exactly_two_identity_prompts_both_unanswered_and_every_box_default() {
    let (_dir, vault) = vault_with_year_n(|ri| {
        ri.int_1099 = vec![
            Form1099Int {
                payer: "First Bank".into(),
                payer_tin: "11-1111111".into(),
                box1_interest: dec!(120),
                ..Default::default()
            },
            Form1099Int {
                payer: "Second Bank".into(),
                payer_tin: "22-2222222".into(),
                box1_interest: dec!(240),
                ..Default::default()
            },
        ];
    });
    let opened = open(&vault, false);
    assert_eq!(
        opened.identities.len(),
        2,
        "two payers, two prompts: {:#?}",
        opened.identities
    );
    for (id, who) in opened.identities.iter().zip(["First Bank", "Second Bank"]) {
        assert!(
            id.prompt.contains(who) && id.prompt.contains("Form 1099-INT"),
            "the prompt names the payer and the document in the form's words: {}",
            id.prompt
        );
        assert!(
            id.prompt.contains(&TO.to_string()),
            "and asks about the NEW year: {}",
            id.prompt
        );
        assert_eq!(
            id.answer, None,
            "nothing answers for the filer — the census row is unanswered on the seed"
        );
    }
    let seed = draft(&vault);
    assert_eq!(
        seed.documents.int_1099, None,
        "the census row is re-asked from blank"
    );
    assert_eq!(
        seed.int_1099
            .iter()
            .map(|r| (r.payer.as_str(), r.payer_tin.as_str(), r.box1_interest))
            .collect::<Vec<_>>(),
        vec![
            ("First Bank", "11-1111111", rust_decimal::Decimal::ZERO),
            ("Second Bank", "22-2222222", rust_decimal::Decimal::ZERO),
        ],
        "each row is PRE-NAMED with every box blank"
    );
}

/// ★★★ **No `Usd` leaf of the seeded draft is non-zero except the carryforwards — walked with T1's
///     own money detector, never a hand-list of fields.**
///
/// The prefixes on the right are the four carryovers R10.4 names, and they are the SPEC's list, not
/// this test's: everything else about the seed is asserted to be zero without naming it, so a money
/// box added to `ReturnInputs` tomorrow is covered the day the seed touches it.
#[test]
fn no_money_leaf_of_the_seed_is_nonzero_except_the_carryforwards() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    // ★★★ **THE VACUITY GUARD.** "The seed carries no figure" says nothing about a year N that had
    //     none. Measured at 21 non-zero money leaves on this fixture when it was written; the floor
    //     is just below it, so a fixture that quietly loses its figures fails HERE, saying so,
    //     rather than passing the test it makes meaningless. (A plant that copied `payments`
    //     forward passed against the first version of this fixture, whose `payments` were zero.)
    let prior_nonzero = {
        let s = Session::open(&vault, &pp()).unwrap();
        nonzero_money_leaves(&return_inputs::get(s.conn(), FROM).unwrap().unwrap())
    };
    assert!(
        prior_nonzero.len() >= 20,
        "year N realized only {} non-zero money leaves — too few for \"the seed carries none of \
         them\" to mean anything: {prior_nonzero:#?}",
        prior_nonzero.len()
    );
    let opened = open(&vault, false);
    assert_eq!(
        opened.not_carried, None,
        "premise: year N's chain must COMPUTE, or the carryforward half of this test is vacuous"
    );
    let seed = draft(&vault);
    let nonzero = nonzero_money_leaves(&seed);
    const CARRYFORWARDS: [&str; 4] = [
        "capital_loss_carryforward_in",
        "charitable_carryover_in",
        "qbi.reit_ptp_carryforward_in",
        "qbi.qbi_carryforward_in",
    ];
    for (path, amount) in &nonzero {
        assert!(
            CARRYFORWARDS.iter().any(|p| path.starts_with(p)),
            "{path} = {amount} crossed the year boundary and is not a carryforward — R10.4: NEVER a \
             carried amount. Every non-zero leaf: {nonzero:#?}"
        );
    }
    assert_eq!(
        nonzero.get("capital_loss_carryforward_in.long").copied(),
        Some(dec!(57000)),
        "the §1212(b) roll IS carried (else this test passes on an opener that carries nothing): \
         {nonzero:#?}"
    );
    assert_eq!(
        seed.capital_loss_carryforward_in_provenance,
        CarryProvenance::ComputedFromPriorReturn { year: FROM },
        "and it says which RETURN it came off"
    );
    assert_eq!(
        seed.charitable_carryover_in_provenance,
        CarryProvenance::ComputedFromPriorReturn { year: FROM },
        "§G-23: a computed ZERO is not an unasked one"
    );
    assert_eq!(
        seed.qbi.reit_ptp_carryforward_in_provenance,
        CarryProvenance::ComputedFromPriorReturn { year: FROM }
    );
    assert_eq!(
        seed.qbi.qbi_carryforward_in_provenance,
        CarryProvenance::ComputedFromPriorReturn { year: FROM }
    );
}

/// ★ **The re-stamp is COMPLETE, and this asks the question structurally.** `restamp_from_prior_return`
/// names four provenance fields plus the per-item one; a fifth added tomorrow would be left saying
/// `Computed` — *"this year's write-back derived it"* — on a figure that crossed a year. So: no leaf
/// of the seeded draft is the string `"computed"`.
#[test]
fn nothing_in_the_seed_is_stamped_merely_computed() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    open(&vault, false);
    let json = serde_json::to_value(draft(&vault)).unwrap();
    fn walk(v: &serde_json::Value, at: &str, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, c) in m {
                    walk(c, &format!("{at}.{k}"), out);
                }
            }
            serde_json::Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    walk(c, &format!("{at}[{i}]"), out);
                }
            }
            serde_json::Value::String(s) if s == "computed" => out.push(at.to_string()),
            _ => {}
        }
    }
    let mut bare = Vec::new();
    walk(&json, "", &mut bare);
    assert!(
        bare.is_empty(),
        "these carried figures are stamped `Computed` (this year's write-back) instead of \
         `ComputedFromPriorReturn` (it crossed a year): {bare:?}"
    );
}

/// ★★★ **Every `PerYear` gate is re-asked BLANK** — walked from the registries, so a question added
///     tomorrow is checked without an edit here.
#[test]
fn every_per_year_gate_on_the_seed_is_unanswered() {
    use btctax_core::tax::questions::{
        Durability, SkippableKind, FORM_QUESTIONS, SKIPPABLE_QUESTIONS,
    };
    let (_dir, vault) = vault_with_year_n(|ri| {
        a_year_with_money_in_it(ri);
        ri.header.spouse = None;
    });
    open(&vault, false);
    let seed = draft(&vault);
    for q in FORM_QUESTIONS {
        assert_eq!(
            (q.get)(&seed),
            None,
            "{:?} carries an answer nobody gave this year (every DECLARATION is PerYear)",
            q.id
        );
    }
    for sk in SKIPPABLE_QUESTIONS {
        if sk.durability == Durability::Durable {
            continue; // the dates of birth — shown, and covered by the test below
        }
        let answered = match sk.kind {
            SkippableKind::Date => (sk.get_date)(&seed).is_some(),
            SkippableKind::YesNo => (sk.get_bool)(&seed).is_some(),
            SkippableKind::Choice(_) => (sk.get_choice)(&seed).is_some(),
        };
        assert!(!answered, "{:?} carries a prior year's answer", sk.id);
    }
}

/// ★★★ **C-1 — THE SEAM REVIEW'S PROBE: a `Durable` date of birth is SHOWN AS A HINT and NEVER
///     PRE-FILLED, so the documented SKIP keystroke cannot become this year's testimony.**
///
/// The build pre-filled it. `income answer`'s `skippable_state` decides `Given` vs `Declined` by
/// reading the VALUE, so a bare Enter — the documented skip — left year N's date in place and wrote
/// `AnswerRecord { answered_on: <this session>, state: Given }`: a prior-year answer satisfying this
/// year's provenance, one command after the module header declared that structurally impossible.
/// `Durability::Durable` says it in words: *"never Enter-to-accept, never pre-filled"*.
#[test]
fn a_bare_enter_on_the_shown_date_of_birth_never_records_it_as_given_this_year() {
    use btctax_core::tax::provenance::{AnswerKey, AnswerState};
    use btctax_core::tax::questions::SkippableId;
    let (_dir, vault) = vault_with_year_n(|ri| {
        a_year_with_money_in_it(ri);
        // Year N ANSWERED its own date of birth — a record exists on year N.
        btctax_core::tax::provenance::record_answer(
            ri,
            AnswerKey::Skippable(SkippableId::DobTaxpayer),
            "YOUR date of birth",
            time::macros::date!(2025 - 03 - 01),
            AnswerState::Given,
        );
    });
    open(&vault, false);
    let seed = draft(&vault);
    assert_eq!(
        seed.header.taxpayer.date_of_birth, None,
        "the durable fact is NOT pre-filled"
    );
    assert_eq!(seed.opened_from, Some(FROM), "the year is carried instead");
    assert!(seed.answer_log.is_empty(), "and no record crosses");

    // The filer does the documented SKIP: a bare Enter on every skippable.
    let screen = answer_the_draft(&vault, |_| String::from("\n"));
    assert!(
        screen.contains("TY2024's return gave 1980-05-05 — type it to confirm"),
        "the prior date is SHOWN in the prompt: {screen}"
    );
    let after = draft(&vault);
    assert_eq!(
        after.header.taxpayer.date_of_birth, None,
        "a skip leaves it unanswered — the §63(f) addition is lawfully forgone"
    );
    match after
        .answer_log
        .get(&AnswerKey::Skippable(SkippableId::DobTaxpayer))
    {
        None => {}
        Some(rec) => assert_eq!(
            rec.state,
            AnswerState::Declined,
            "a skip may record DECLINED (asked, passed over) and NEVER `Given`: {rec:?}"
        ),
    }
}

/// The other half: TYPING the date is a fresh answer, dated by the clock seam and hashing the words
/// asked today. Without this, the test above passes on a prompt nobody can answer.
#[test]
fn typing_the_shown_date_of_birth_writes_a_fresh_record_dated_by_the_seam() {
    use btctax_core::tax::provenance::{prompt_hash, AnswerKey, AnswerState};
    use btctax_core::tax::questions::{SkippableId, SKIPPABLE_QUESTIONS};
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    open(&vault, false);
    answer_the_draft(&vault, |sk| {
        if sk == SkippableId::DobTaxpayer {
            "1980-05-05\n".to_string()
        } else {
            "\n".to_string()
        }
    });
    let after = draft(&vault);
    assert_eq!(
        after.header.taxpayer.date_of_birth,
        Some(time::macros::date!(1980 - 05 - 05)),
        "typing it is how a durable fact is confirmed"
    );
    let rec = after
        .answer_log
        .get(&AnswerKey::Skippable(SkippableId::DobTaxpayer))
        .expect("confirming writes a FRESH record");
    assert_eq!(
        rec.answered_on,
        time::macros::date!(2026 - 02 - 03),
        "dated by the BTCTAX_NOW seam this run was given, not by year N"
    );
    let sk = SKIPPABLE_QUESTIONS
        .iter()
        .find(|s| s.id == SkippableId::DobTaxpayer)
        .unwrap();
    assert_eq!(
        rec.prompt_hash,
        prompt_hash(sk.prompt),
        "hashing the words asked TODAY"
    );
    assert_eq!(rec.state, AnswerState::Given);
}

/// The hint is a fact about an OPENED year: a year the filer started themselves has no prior row to
/// read and says nothing about one.
#[test]
fn the_date_of_birth_hint_appears_only_on_an_opened_year() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    // A TY2025 draft the filer started themselves — no `opened_from`.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        input_form_store::save_draft(
            &mut s,
            TO,
            &ReturnInputs {
                tax_year: TO,
                filing_status: FilingStatus::Single,
                foreign_trust: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    }
    let screen = answer_the_draft(&vault, |_| String::from("\n"));
    assert!(
        !screen.contains("type it to confirm"),
        "no prior year, no hint: {screen}"
    );
}

/// Drive `income answer` over the year-`TO` draft, scripting every declaration `n` and each skippable
/// by `skippable`. Returns what the filer saw.
///
/// ★ The script is DERIVED from `live_questions`, never a magic count — the count is exactly what
/// ★★★ **THE SCRIPT, DERIVED BY SIMULATING `income answer`'s OWN SWEEP.**
///
/// One pass over `live_questions` is not enough since T5: R3's document-less income door makes a
/// question live BECAUSE another was answered "no", so the command sweeps until the set stops
/// growing and a script taken from one snapshot runs out mid-run (*"input ended before every
/// question was answered"*). Declarations answer `n`; skippables take whatever `skippable` says.
fn sweep_script(
    ri: &ReturnInputs,
    skippable: impl Fn(btctax_core::tax::questions::SkippableId) -> String,
) -> String {
    use btctax_cli::cmd::answer::Ask;
    let mut ri = ri.clone();
    let mut decl: std::collections::BTreeSet<btctax_core::tax::questions::QuestionId> =
        std::collections::BTreeSet::new();
    let mut skip: std::collections::BTreeSet<btctax_core::tax::questions::SkippableId> =
        std::collections::BTreeSet::new();
    let mut gates: std::collections::BTreeSet<(
        btctax_core::tax::provenance::DependentGate,
        usize,
    )> = std::collections::BTreeSet::new();
    let mut script = String::new();
    for _ in 0..8 {
        let round: Vec<Ask> = cmd::answer::live_questions(&ri)
            .into_iter()
            .filter(|a| match a {
                Ask::Declaration(q) => !decl.contains(&q.id),
                Ask::Skippable(sk) => !skip.contains(&sk.id),
                // ★★★ T7 / R6 — the per-row §152 gates join the sweep too.
                Ask::DependentGate { gate, row } => !gates.contains(&(gate.gate, *row)),
            })
            .collect();
        if round.is_empty() {
            break;
        }
        for a in round {
            match a {
                Ask::Declaration(q) => {
                    decl.insert(q.id);
                    script.push_str("n\n");
                    (q.set)(&mut ri, false);
                }
                Ask::Skippable(sk) => {
                    skip.insert(sk.id);
                    script.push_str(&skippable(sk.id));
                }
                // ★★★ T7 / R6 — a seeded dependent row is BLOCKING through its gates (FR-70), so a
                //     script that skipped them would end the run mid-interview. Answered at the
                //     registry's own declared claim-path polarity.
                Ask::DependentGate { gate, row } => {
                    use btctax_core::tax::dependent_gates::GateKind;
                    gates.insert((gate.gate, row));
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
    script
}

/// Drive `income answer` over the year-`TO` draft with an arbitrary keystroke script, returning the
/// command's own result AND what the filer saw. The screen is captured either way, because a
/// refusing run is exactly what one of the tests below is about.
fn answer_the_draft_with(
    vault: &std::path::Path,
    script: &str,
) -> (Result<(), btctax_cli::CliError>, String) {
    let mut keys = script.as_bytes();
    let mut screen = Vec::new();
    let r = cmd::answer::answer_return_inputs(
        vault,
        &pp(),
        TO,
        time::macros::date!(2026 - 02 - 03),
        &mut keys,
        &mut screen,
        false,
    );
    (r, String::from_utf8(screen).unwrap())
}

///   broke `tax_report`'s script when the interview grew.
fn answer_the_draft(
    vault: &std::path::Path,
    skippable: impl Fn(btctax_core::tax::questions::SkippableId) -> String,
) -> String {
    let ri = draft(vault);
    let script = sweep_script(&ri, &skippable);
    let mut keys = script.as_bytes();
    let mut screen = Vec::new();
    cmd::answer::answer_return_inputs(
        vault,
        &pp(),
        TO,
        time::macros::date!(2026 - 02 - 03),
        &mut keys,
        &mut screen,
        false,
    )
    .expect("the seeded draft is answerable");
    String::from_utf8(screen).unwrap()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The carryforwards come off the RETURN, not off year N's inputs
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **The planted defect R10.4 names: year N's stored `capital_loss_carryforward_in` is $60,000
///     and its RETURN computes $57,000 out. The seed must take the RETURN's figure.**
///
/// Reading the inputs instead would carry the filer's own prior entry forward untouched — the loss
/// would never be absorbed, and the §1211(b) allowance would be claimed again every year.
#[test]
fn the_carryforward_is_read_off_the_frozen_return_and_not_off_year_ns_inputs() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    let stored_in = {
        let s = Session::open(&vault, &pp()).unwrap();
        return_inputs::get(s.conn(), FROM).unwrap().unwrap()
    };
    assert_eq!(
        stored_in.capital_loss_carryforward_in.long,
        dec!(60000),
        "premise: year N's INPUTS say 60,000"
    );
    open(&vault, false);
    let seed = draft(&vault);
    assert_eq!(
        seed.capital_loss_carryforward_in.long,
        dec!(57000),
        "the seed takes the RETURN's carryover-OUT, which absorbed the §1211(b) $3,000"
    );
    assert_ne!(
        seed.capital_loss_carryforward_in.long, stored_in.capital_loss_carryforward_in.long,
        "and the two really do differ, or this test cannot tell them apart"
    );
}

/// The chain's own gates still hold: on a year with no package there is no frozen return to read,
/// so NOTHING is stamped and the opener says so. A `{0,0}` stamped `ComputedFromPriorReturn` there
/// would silence next year's `BenefitCarryoversNotStated` about a carryover the filer may have.
#[test]
fn a_year_with_no_package_carries_no_figure_and_the_opener_says_why() {
    let (dir, vault) = fresh_vault();
    // 2026 has no `FullReturnParams` by design, and `income import` still stores a row there (T4).
    let toml = dir.path().join("y2026.toml");
    std::fs::write(&toml, "filing_status = \"Single\"\n").unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2026, &toml, false, false).unwrap();
    let opened = {
        let mut s = Session::open(&vault, &pp()).unwrap();
        open_next_year(&mut s, 2026, false).unwrap()
    };
    let why = opened
        .not_carried
        .expect("no package for 2026 ⇒ no carryforward is stamped");
    assert!(
        why.contains("no full-return tables for 2026"),
        "and the filer is told which year could not be computed: {why}"
    );
    assert!(
        !why.starts_with("usage:"),
        "and it does not read as though the FILER mistyped the command: {why}"
    );
    assert!(opened.carried.is_empty());
    assert_eq!(
        opened.not_stamped, None,
        "the capital-loss note is about a GROUNDING failure, not about a missing package — the \
         chain never ran here, and saying both would name the same gap twice"
    );
    let s = Session::open(&vault, &pp()).unwrap();
    let seed = match input_form_store::load(s.conn(), 2027).unwrap().0 {
        input_form_store::Loaded::Draft { ri, .. } => ri,
        _ => panic!("2027 is still seeded — the identities do not depend on the chain"),
    };
    assert_eq!(
        seed.capital_loss_carryforward_in_provenance,
        CarryProvenance::User,
        "an unasked carryover stays unasked; nothing is stamped"
    );
}

/// ★★★ **T9's rule, inherited: the one carryover that is NOT stamped is NAMED.**
///
/// The §1212(b) roll is the one GATED write — `capital_loss_roll_is_grounded` refuses to stamp a
/// figure btctax cannot vouch for, and on a year that was never asked about a carryover and produced
/// none, nothing is written. Omitting it silently would leave the filer reading four carryovers into
/// a list of three, which is exactly the drift `write_back_carryover`'s summary was fixed for.
#[test]
fn a_capital_loss_roll_the_gate_skipped_is_named_and_not_silently_omitted() {
    // No carried loss and no disposals ⇒ nothing grounds the roll.
    let (_dir, vault) = vault_with_year_n(|ri| {
        ri.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "Acme Tooling".into(),
            ein: Some("12-3456789".into()),
            box1_wages: dec!(30000),
            box3_ss_wages: dec!(30000),
            box5_medicare_wages: dec!(30000),
            ..Default::default()
        }];
    });
    let opened = open(&vault, false);
    assert_eq!(opened.not_carried, None, "premise: the chain DID compute");
    assert!(
        !opened
            .carried
            .iter()
            .any(|c| c.contains("capital-loss carryover")),
        "the gate skipped the write, so the summary must not claim it: {:?}",
        opened.carried
    );
    let note = opened
        .not_stamped
        .as_deref()
        .expect("and the skip is NAMED, not omitted");
    assert!(
        note.contains("never asked about one") && note.contains("never asked"),
        "naming what was not written and why: {note}"
    );
    assert!(
        opened.render().contains("NOT the capital-loss carryover"),
        "and the filer sees it: {}",
        opened.render()
    );
    assert_eq!(
        draft(&vault).capital_loss_carryforward_in_provenance,
        CarryProvenance::User,
        "nothing is stamped, so next year's advisory stays live about it"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The three refusals, and where the write lands
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The opener writes the DRAFT. `return_inputs::get(N+1)` is still `None` — nothing reaches
/// `resolve.rs` precedence 1, exactly as R11 requires of every write on a params-less year.
#[test]
fn the_open_writes_a_draft_and_no_committed_row() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    open(&vault, false);
    let s = Session::open(&vault, &pp()).unwrap();
    assert!(
        return_inputs::get(s.conn(), TO).unwrap().is_none(),
        "the opener never calls `return_inputs::set`"
    );
    assert!(input_form_store::draft_exists(s.conn(), TO).unwrap());
}

/// ★★★ **Nothing about year N changes** — retention is the filer's decision, not this command's.
///
/// ★ M-3 — all THREE of year N's stores, not just the committed row: the draft table and its
///   `parked` flag are separate rows the opener has no business touching, and a kill named
///   *"nothing about year N changes"* that reads one of the three does not hold what it is named
///   for. (The answer log lives inside the committed `ReturnInputs`, so it rides on the first.)
#[test]
fn nothing_about_year_n_changes() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    // Year N also holds a PARKED draft — the state with the most to lose.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        input_form_store::save_draft(&mut s, FROM, &draft_holding_an_interview()).unwrap();
    }
    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        (
            return_inputs::get(s.conn(), FROM).unwrap().unwrap(),
            match input_form_store::load(s.conn(), FROM).unwrap().0 {
                input_form_store::Loaded::Draft { ri, parked } => Some((ri, parked)),
                _ => None,
            },
        )
    };
    assert!(before.1.is_some(), "premise: year N has a draft to protect");
    open(&vault, false);
    let s = Session::open(&vault, &pp()).unwrap();
    let after = (
        return_inputs::get(s.conn(), FROM).unwrap().unwrap(),
        match input_form_store::load(s.conn(), FROM).unwrap().0 {
            input_form_store::Loaded::Draft { ri, parked } => Some((ri, parked)),
            _ => None,
        },
    );
    assert_eq!(
        after, before,
        "year N is READ: its committed row, its draft and that draft's parked flag are untouched"
    );
}

/// ★ M-1 — the opener's normal output is a non-trivial year-N+1 draft, which permanently REFUSES
///   `report --tax-year N --write-carryover`, whose refusal then prescribes `--discard-draft`. A
///   filer who follows that chain destroys the year they just opened, so the report says so at the
///   moment the draft is created.
#[test]
fn the_report_says_the_write_carryover_is_no_longer_needed() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    let opened = open(&vault, false);
    let rendered = opened.render();
    assert!(
        rendered.contains("--write-carryover` is not needed and will refuse")
            && rendered.contains("Do not pass `--discard-draft` to it"),
        "the report warns about the chain that would destroy the opened year: {rendered}"
    );
    // And the refusal it is warning about really does fire.
    let err = cmd::tax::write_back_carryover(&vault, &pp(), FROM, false, false)
        .expect_err("the seeded draft blocks the write-back");
    assert!(matches!(
        err,
        btctax_cli::CliError::NonTrivialDraftBlocksWrite { year, .. } if year == TO
    ));
}

/// There must be a year N to open from.
#[test]
fn a_year_n_with_no_committed_row_refuses() {
    let (_dir, vault) = fresh_vault();
    let mut s = Session::open(&vault, &pp()).unwrap();
    let err = open_next_year(&mut s, FROM, false).expect_err("there is nothing to open from");
    let msg = err.to_string();
    assert!(
        msg.contains("no full-return inputs for 2024") && msg.contains("income import"),
        "the refusal names the missing year and the exit: {msg}"
    );
    assert!(!input_form_store::draft_exists(s.conn(), TO).unwrap());
}

/// And year N+1 must not already exist: the opener starts a year, it does not reset one.
#[test]
fn a_committed_row_on_the_year_being_opened_refuses_and_writes_nothing() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let existing = btctax_core::tax::testonly::answered(ReturnInputs {
            tax_year: TO,
            filing_status: FilingStatus::Single,
            ..Default::default()
        });
        return_inputs::set(s.conn(), TO, &existing).unwrap();
        s.save().unwrap();
    }
    let mut s = Session::open(&vault, &pp()).unwrap();
    let err = open_next_year(&mut s, FROM, false).expect_err("2025 already exists");
    let msg = err.to_string();
    assert!(
        msg.contains("already has a stored full return") && msg.contains("income clear"),
        "the refusal says why and names the exit: {msg}"
    );
    assert!(
        !input_form_store::draft_exists(s.conn(), TO).unwrap(),
        "and NOTHING was written"
    );
}

/// ★★★ **A PARKED draft on the year being opened refuses, and `--discard-draft` does not open it.**
///
/// A parked draft is the sole copy of a screened return (C-1): the committed row it came from is
/// gone. Seeding over it would destroy the only copy, and `--discard-draft` — the confirmation for a
/// work-in-progress draft — is deliberately not a licence to do that.
#[test]
fn a_parked_draft_on_the_year_being_opened_refuses_even_with_discard_draft() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let existing = btctax_core::tax::testonly::answered(ReturnInputs {
            tax_year: TO,
            filing_status: FilingStatus::Single,
            ..Default::default()
        });
        return_inputs::set(s.conn(), TO, &existing).unwrap();
        s.save().unwrap();
        // Park it: the row becomes a parked draft and the committed row is deleted.
        input_form_store::park_to_profile(&mut s, TO).unwrap();
    }
    for discard in [false, true] {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let err = open_next_year(&mut s, FROM, discard)
            .expect_err("a parked return is never seeded over");
        assert!(
            matches!(err, btctax_cli::CliError::ParkedDraftBlocksWrite { year } if year == TO),
            "the parked refusal, with or without --discard-draft: {err}"
        );
    }
    // And the parked draft is still there, still parked.
    let s = Session::open(&vault, &pp()).unwrap();
    assert!(input_form_store::draft_exists(s.conn(), TO).unwrap());
}

/// A draft holding an interview: one answered census row, with the answer RECORDED — the part that
/// cannot be re-created by re-typing.
fn draft_holding_an_interview() -> ReturnInputs {
    use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
    use btctax_core::tax::questions::FORM_QUESTIONS;
    let mut ri = ReturnInputs {
        tax_year: TO,
        filing_status: FilingStatus::Single,
        ..Default::default()
    };
    let q = FORM_QUESTIONS
        .iter()
        .find(|q| q.id == DocumentRow::K1.question_id())
        .unwrap();
    (q.set)(&mut ri, false);
    record_answer(
        &mut ri,
        AnswerKey::Question(q.id),
        q.prompt,
        time::macros::date!(2025 - 09 - 01),
        AnswerState::Given,
    );
    ri
}

/// ★★★ **T4's rule binds the opener's write: a WIP draft holding an interview is never destroyed on
///     a note, and `--discard-draft` is the same confirmation it is everywhere else.**
#[test]
fn a_non_trivial_draft_on_the_year_being_opened_refuses_and_survives() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        input_form_store::save_draft(&mut s, TO, &draft_holding_an_interview()).unwrap();
    }
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let err = open_next_year(&mut s, FROM, false)
            .expect_err("a draft holding an interview is not superseded on a note");
        let msg = err.to_string();
        assert!(
            msg.contains("recorded answer(s)") && msg.contains("--discard-draft"),
            "the refusal names what the draft holds and the confirmation: {msg}"
        );
    }
    assert_eq!(
        draft(&vault),
        draft_holding_an_interview(),
        "the draft survives untouched"
    );

    // With the flag, the open proceeds and replaces it.
    let opened = open(&vault, true);
    assert_eq!(opened.to, TO);
    assert!(
        draft(&vault).answer_log.is_empty(),
        "the seeded draft carries no record — the discarded interview's records are gone with it"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// A `No` to the census removes the PRE-NAMED rows the opener seeded
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **R10.4: *"a No removes the pre-named row"* — end to end, through `income answer`.**
///
/// Without it the opener manufactures a brick: it pre-names rows the filer never typed, the filer
/// truthfully answers *"no W-2 this year"*, and `screen_inputs` then refuses the year for a
/// contradiction (`Some(false)` beside transcribed rows) that `income answer` offers no way to
/// clear.
#[test]
fn answering_no_to_the_census_removes_the_pre_named_rows() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    open(&vault, false);
    let seed = draft(&vault);
    assert_eq!(
        seed.w2s.len(),
        1,
        "premise: the opener pre-named one employer"
    );

    let script = sweep_script(&seed, |_| "\n".to_string());
    let mut keys = script.as_bytes();
    let mut screen = Vec::new();
    cmd::answer::answer_return_inputs(
        &vault,
        &pp(),
        TO,
        time::macros::date!(2026 - 02 - 03),
        &mut keys,
        &mut screen,
        false,
    )
    .expect("answering the seeded draft");

    let after = draft(&vault);
    assert_eq!(after.documents.w2, Some(false));
    assert!(
        after.w2s.is_empty(),
        "the pre-named employer goes with the answer: {:?}",
        after.w2s
    );
    // ★ Not "the year screens clean" — this filer answered `n` to everything, including the AMT
    //   carryover question, which refuses on a `No` by design. The claim is narrower and is the one
    //   the pre-named row could break: no CONTRADICTION refusal, the state that used to be
    //   unreachable from `income answer`.
    let refusal = btctax_core::tax::return_refuse::screen_inputs(
        &after,
        &btctax_core::tax::testonly::ty2024_table(),
        &btctax_core::tax::testonly::ty2024_params(),
    );
    assert!(
        !matches!(
            refusal.as_ref().map(|r| &r.reason),
            Some(btctax_core::tax::return_refuse::RefuseReason::DocumentCensusContradicted { .. })
        ),
        "the year is not left refusing on a contradiction the filer cannot reach: {refusal:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The rest of the identity list: dependents and venues
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **FR-70 (T7) CLOSES I-2, AND I-3 STANDS: the dependent IDENTITY crosses; the venue KEY does not.**
///
/// I-2 removed the seeded dependent row for ONE reason, recorded in `open_next_year::seed`: *"there
/// is nothing on this year's return that can answer for it — no census row, no `FormQuestion`, no
/// `RefuseReason`, and no `interview_state` item"*, so the child who aged out rode across silently.
/// **T7 built all four.** A seeded row is now BLOCKING through every §152 gate the flowchart demands
/// of it (asserted below through `interview_state`), and `screen_inputs` refuses until the filer
/// answers this year's Step 1 for that child or deletes the row.
///
/// The venue is NOT the same shape and is unchanged: a venue KEY *is* answered-ness —
/// `broker_reporting`'s contract is *"absent = unanswered: answered-ness lives in the key set, never
/// in a sentinel value"*, and three call sites read presence in the key set as *"the filer stored
/// 1099-DA answers"*. There is no blank state to seed.
///
/// ★ This test used to assert `dependents.is_empty()`. That assertion was the RECORD of a missing
///   surface, not a property of the design, and it is replaced here rather than deleted so the diff
///   shows exactly what changed and why.
#[test]
fn a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not() {
    let (_dir, vault) = vault_with_year_n(|ri| {
        ri.header.dependents = vec![Dependent {
            name: "Sam Filer".into(),
            ssn: "987654321".into(),
            relationship: "daughter".into(),
            date_of_birth: Some(time::macros::date!(2015 - 04 - 01)),
            ..Default::default()
        }];
        ri.broker_reporting
            .0
            .insert("coinbase".into(), Default::default());
    });
    let opened = open(&vault, false);
    let rendered = opened.render();
    assert!(
        rendered.contains("Sam Filer (daughter)") && rendered.contains("dependent for 2025"),
        "the dependent is ASKED about by name: {rendered}"
    );
    assert!(
        rendered.contains("coinbase"),
        "and so is the venue: {rendered}"
    );
    assert!(
        !rendered.contains("987654321") && !rendered.contains("987-65-4321"),
        "a dependent's SSN is never printed: {rendered}"
    );

    let seed = draft(&vault);
    // ── FR-70: the PERSON crosses. ──
    assert_eq!(seed.header.dependents.len(), 1, "the identity is seeded");
    let d = &seed.header.dependents[0];
    assert_eq!(
        (d.name.as_str(), d.ssn.as_str(), d.relationship.as_str()),
        ("Sam Filer", "987654321", "daughter"),
        "name, SSN and relationship are IDENTITY — who this is, not a claim about the year"
    );
    // ★★★ **SEAM REVIEW I-3 — the one `Durable` gate is SHOWN, NEVER PRE-FILLED.** It used to
    //     cross, and that broke `Durability::Durable`'s own definition in both halves: `income
    //     answer`'s date gate takes a bare Enter as *keep what is on file* and then records
    //     `AnswerState::Given`, so the filer ended up with a record dated THIS year for a value
    //     they never typed. The value is load-bearing — Step 1's age test decides CTC versus ODC —
    //     so the seed leaves it blank and the prompt carries year N's date as a hint.
    assert_eq!(
        d.date_of_birth, None,
        "the Durable date of birth does not cross: it is displayed beside this year's prompt and \
         confirmed by the same keystroke a fresh answer takes"
    );
    // ── ...and the CLAIM does not: every one of the twenty §152 gates is blank. ──
    let blank: Vec<_> = btctax_core::tax::dependent_gates::DEPENDENT_GATES
        .iter()
        .filter(|g| (g.get)(d).is_some() || (g.get_date)(d).is_some())
        .map(|g| g.gate)
        .collect();
    assert!(
        blank.is_empty(),
        "every §152 gate must cross BLANK — a prior year's answer is not testimony for this one: \
         {blank:?}"
    );
    // ── ...and the row is therefore BLOCKING, which is what I-2 said did not exist. ──
    let st = btctax_core::tax::interview_state::interview_state(&seed);
    let gate_items = st
        .blocking
        .iter()
        .filter(|b| {
            matches!(
                b.item,
                btctax_core::tax::provenance::AnswerKey::DependentGate { .. }
            )
        })
        .count();
    assert!(
        gate_items > 0,
        "FR-70 — the seeded row must be BLOCKING through its gates, or it rides across silently \
         exactly as I-2 described: {:?}",
        st.blocking
    );
    // ── The venue: unchanged. ──
    assert!(
        seed.broker_reporting.0.is_empty(),
        "I-3 — no venue key, so nothing reads answered-ness the filer never gave: {:?}",
        seed.broker_reporting.0
    );
}

/// ★★★ **SEAM REVIEW I-3's KILL — A BARE ENTER DOES NOT CONFIRM A DEPENDENT'S DATE OF BIRTH.**
///
/// `DateOfBirth` is `Durability::Durable`: *"the prior MAY be displayed, but it still requires the
/// same explicit keystroke as a fresh ask: **never Enter-to-accept, never pre-filled**"*. Before this
/// fold the opener seeded the value, so `income answer`'s date gate found something on file, took a
/// bare Enter as *keep it*, and wrote `AnswerRecord { state: Given }` dated THIS year for a date the
/// filer never typed. The value decides Step 1's age test, i.e. the child tax credit versus the
/// credit for other dependents.
///
/// Both halves are asserted: year N's date is SHOWN (so the filer is not sent to look it up), and
/// skipping it confirms NOTHING — no value, no record, and the gate still blocking.
#[test]
fn a_bare_enter_does_not_confirm_a_seeded_dependents_date_of_birth() {
    use btctax_core::tax::provenance::{dependent_ssn_hash, AnswerKey, DependentGate};
    let (_dir, vault) = vault_with_year_n(|ri| {
        a_year_with_money_in_it(ri);
        ri.header.dependents = vec![Dependent {
            name: "Sam Filer".into(),
            ssn: "987654321".into(),
            relationship: "daughter".into(),
            date_of_birth: Some(time::macros::date!(2015 - 04 - 01)),
            ..Default::default()
        }];
    });
    open(&vault, false);
    let seeded = draft(&vault);
    assert_eq!(
        seeded.header.dependents[0].date_of_birth, None,
        "the premise: the seed does not pre-fill it"
    );

    // The script a diligent filer would run — with the dependent's date REPLACED by a bare Enter.
    let full = sweep_script(&seeded, |_| String::from("\n"));
    let typed = format!("{}\n", time::macros::date!(2015 - 06 - 01));
    let dob_line = format!("{}\n", seeded.tax_year - 10);
    let dob_line = full
        .lines()
        .find(|l| l.starts_with(&dob_line[..4]))
        .map(|l| format!("{l}\n"))
        .expect("the sweep script types a date for the dependent's DOB gate");
    let skipped = full.replacen(&dob_line, "\n", 1);
    assert_ne!(skipped, full, "the premise: one keystroke was replaced");

    let (result, screen) = answer_the_draft_with(&vault, &skipped);
    assert!(
        screen.contains("TY2024's return gave 2015-04-01 — type it to confirm"),
        "year N's date is SHOWN beside the gate's own prompt: {screen}"
    );
    assert!(
        result.is_err(),
        "a class-(A) date has no lawful decline — the gate re-asks rather than accepting silence"
    );

    // NOTHING was confirmed: no value, no record, and the gate is still blocking.
    let after = draft(&vault);
    assert_eq!(
        after.header.dependents[0].date_of_birth, None,
        "a bare Enter must not leave a date the filer never typed"
    );
    let key = AnswerKey::DependentGate {
        ssn_hash: dependent_ssn_hash("987654321"),
        gate: DependentGate::DateOfBirth,
    };
    assert!(
        !after.answer_log.contains_key(&key),
        "…and no `Given` record dated this year for a value nobody entered: {:?}",
        after.answer_log.get(&key)
    );
    let st = btctax_core::tax::interview_state::interview_state(&after);
    assert!(
        st.blocking.iter().any(|b| b.item == key),
        "the gate is still BLOCKING, which is what `Durable` means: {:?}",
        st.blocking
    );

    // The other half: TYPING it is a fresh answer, and then it stands.
    let (ok, _) = answer_the_draft_with(&vault, &full.replacen(&dob_line, &typed, 1));
    ok.expect("a typed date is answerable");
    let confirmed = draft(&vault);
    assert_eq!(
        confirmed.header.dependents[0].date_of_birth,
        Some(time::macros::date!(2015 - 06 - 01)),
        "typing it is how a Durable fact is confirmed — including a CORRECTION of year N's date"
    );
    assert!(
        confirmed.answer_log.contains_key(&key),
        "and THAT writes the record"
    );
}

/// ★★★ **I-3's consequence, at the predicate that reads it.** `resolve.rs`'s `answers_stored` — the
/// same key-set test `admin.rs` selects the export's arm with, and `admin.rs` resolves through the
/// DRAFT — must be FALSE on a seeded year, or the year-readiness sentence claims the crypto slice
/// prints *"from the stored answers"* on a year holding none: the exact sentence R6 fold M-4 fixed.
#[test]
fn the_seeded_year_does_not_claim_stored_broker_answers() {
    let (_dir, vault) = vault_with_year_n(|ri| {
        ri.broker_reporting
            .0
            .insert("coinbase".into(), Default::default());
    });
    open(&vault, false);
    let seed = draft(&vault);
    let answers_stored = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::input_form_store::broker_answers(s.conn(), TO)
            .unwrap()
            .is_some_and(|b| !b.0.is_empty())
    };
    assert!(
        !answers_stored,
        "the opener must not make the year look as though the filer answered a 1099-DA question"
    );
    let sentence = btctax_cli::year_readiness::EntryStates::for_year(TO, Some(&seed)).sentence();
    assert!(
        !sentence.contains("from the stored answers"),
        "and the year-readiness sentence does not claim them: {sentence}"
    );
}

/// The IP PIN does not cross. The IRS issues a new one every filing season, so last year's is not
/// merely unconfirmed — it is wrong, and a plausible wrong one prints on a return.
#[test]
fn the_ip_pin_does_not_cross_the_year_boundary() {
    let (_dir, vault) = vault_with_year_n(a_year_with_money_in_it);
    open(&vault, false);
    assert_eq!(draft(&vault).header.ip_pin, None);
}

/// ★★★ **I-1 — identity crosses, and the test can TELL that it did.**
///
/// The old assertion pinned `seed.filing_status == Single` on a fixture whose year N was Single: it
/// could not distinguish *carried* from *defaulted*, which is why nothing saw the status crossing
/// silently. **Head of household** is the fixture now, because `Default` gives Single.
#[test]
fn the_filers_identity_crosses_and_the_per_year_header_facts_do_not() {
    let (_dir, vault) = vault_with_year_n(|ri| {
        a_year_with_money_in_it(ri);
        ri.filing_status = FilingStatus::HoH;
    });
    open(&vault, false);
    let seed = draft(&vault);
    assert_eq!(
        seed.filing_status,
        FilingStatus::HoH,
        "the status is CARRIED — and `Default` would have said Single, so this can tell"
    );
    assert_eq!(seed.header.taxpayer.first_name, "Alex");
    assert_eq!(seed.header.taxpayer.ssn, "123456789");
    assert_eq!(seed.header.address_city, "Town");
    assert_eq!(
        seed.header.taxpayer.occupation, "",
        "an occupation is a statement about the year, and is re-asked"
    );
    assert_eq!(seed.header.can_be_claimed_as_dependent_taxpayer, None);
    assert_eq!(seed.tax_year, TO);
    assert_eq!(seed.opened_from, Some(FROM));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// I-1 — what crossed is NAMED, and the carried filing status has a surface
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **Every leaf the seed writes is claimed by a phrase the report prints.**
///
/// The finding was four surfaces asserting *"every box is blank and every question is unanswered"*
/// while five fields crossed. This walks the seed against a blank return for the same year — with
/// T1's own machinery, not a hand-list — and requires each differing leaf to be named. A field added
/// to `seed` tomorrow fails HERE unless the report grows a phrase for it.
#[test]
fn every_leaf_the_seed_carries_is_named_in_the_report() {
    use btctax_core::tax::provenance::leaf_walk;
    let (_dir, vault) = vault_with_year_n(|ri| {
        a_year_with_money_in_it(ri);
        ri.filing_status = FilingStatus::HoH;
        ri.header.spouse = None;
    });
    let opened = open(&vault, false);
    let seed = draft(&vault);
    let blank = serde_json::to_value(ReturnInputs {
        tax_year: TO,
        ..Default::default()
    })
    .unwrap();
    let doc = serde_json::to_value(&seed).unwrap();
    let mut leaves = Vec::new();
    leaf_walk::walk(&doc, "", &mut leaves);
    let changed: Vec<String> = leaves
        .into_iter()
        .filter(|path| leaf_walk::at(&blank, path) != leaf_walk::at(&doc, path))
        .collect();
    assert!(
        changed.len() >= 8,
        "the fixture must actually carry things, or this test is vacuous: {changed:?}"
    );
    // The phrases the report prints, and the leaf prefixes each stands for. This list is the TEST's,
    // deliberately: it is what the reader of `render` would understand, checked against what the
    // code did.
    const NAMED: &[(&str, &[&str])] = &[
        ("filing status", &["filing_status"]),
        ("name and SSN", &["header.taxpayer", "header.spouse"]),
        ("mailing address", &["header.address_"]),
        (
            "employer and payer",
            &["w2s", "int_1099", "div_1099", "g_1099", "b_1099"],
        ),
        (
            "carryforwards",
            &[
                "capital_loss_carryforward_in",
                "charitable_carryover_in",
                "qbi.",
            ],
        ),
        ("opened from", &["opened_from", "tax_year"]),
    ];
    let rendered = opened.render();
    for leaf in &changed {
        let hit = NAMED
            .iter()
            .find(|(_, prefixes)| prefixes.iter().any(|p| leaf.starts_with(p)));
        let (phrase, _) = hit.unwrap_or_else(|| {
            panic!(
                "the seed writes `{leaf}`, and no phrase in the opener's report names it — a filer \
                 reading \"everything else is blank\" would have no reason to look. Report:\n{rendered}"
            )
        });
        assert!(
            opened.carried_identity.iter().any(|c| c.contains(phrase)),
            "`{leaf}` is covered by \"{phrase}\", which the report does not print: {:?}",
            opened.carried_identity
        );
    }
    assert!(
        rendered.contains("Carried from TY2024 — CONFIRM each:")
            && rendered.contains("Everything else is blank"),
        "and the blankness claim is BOUNDED by its exceptions: {rendered}"
    );
}

/// ★★★ **The carried filing status has a surface: a class-(A) declaration, live only on an opened
///     year, whose prompt QUOTES the status.**
#[test]
fn the_carried_filing_status_is_confirmed_by_its_own_question() {
    use btctax_core::tax::questions::{QuestionId, FORM_QUESTIONS};
    let (_dir, vault) = vault_with_year_n(|ri| {
        a_year_with_money_in_it(ri);
        ri.filing_status = FilingStatus::HoH;
    });
    open(&vault, false);
    let seed = draft(&vault);
    let q = FORM_QUESTIONS
        .iter()
        .find(|q| q.id == QuestionId::FilingStatusConfirmed)
        .expect("the registry owns it");
    assert!((q.live)(&seed), "live on an opened year");
    assert_eq!(
        (q.get)(&seed),
        None,
        "and unanswered — nothing answers for the filer"
    );
    let prompt = q.prompt_text(&seed);
    assert!(
        prompt.contains("Head of household (HOH)")
            && prompt.contains("TY2024")
            && prompt.contains("TY2025"),
        "the prompt quotes the status in the FORM's words and both years: {prompt}"
    );
    assert!(
        prompt.contains("last day of the tax year"),
        "and cites why it is a per-year question: {prompt}"
    );

    // It BLOCKS while unanswered, exactly as the census rows do.
    let st = btctax_core::tax::interview_state::interview_state(&seed);
    assert!(
        st.blocking
            .iter()
            .any(|b| b.prompt.contains("Head of household")),
        "an unanswered confirmation blocks: {:?}",
        st.blocking.iter().map(|b| &b.prompt).collect::<Vec<_>>()
    );

    // A year the filer started themselves is NOT asked it.
    let own = ReturnInputs {
        tax_year: TO,
        filing_status: FilingStatus::HoH,
        ..Default::default()
    };
    assert!(!(q.live)(&own), "no opener, no question");
}

/// A `No` REFUSES and names where the status is changed — the return would otherwise compute every
/// bracket and threshold on last year's status.
#[test]
fn answering_no_to_the_filing_status_confirmation_refuses_and_names_the_exit() {
    use btctax_core::tax::return_refuse::{screen_inputs, RefuseReason};
    let mut ri = btctax_core::tax::testonly::answered(ReturnInputs {
        tax_year: 2024,
        filing_status: FilingStatus::HoH,
        opened_from: Some(2023),
        ..Default::default()
    });
    let table = btctax_core::tax::testonly::ty2024_table();
    let params = btctax_core::tax::testonly::ty2024_params();
    assert_eq!(
        ri.filing_status_confirmed,
        Some(true),
        "premise: the neutral answer is YES, and it screens clean"
    );
    assert!(screen_inputs(&ri, &table, &params).is_none());
    ri.filing_status_confirmed = Some(false);
    let refusal = screen_inputs(&ri, &table, &params).expect("a NO must refuse");
    assert_eq!(refusal.reason, RefuseReason::FilingStatusChanged);
    assert!(
        refusal.detail.contains("Household section") && refusal.detail.contains("income import"),
        "the refusal names where the status is changed: {}",
        refusal.detail
    );
}

/// ★★★ **The re-ask comes FREE from the prompt hash.** The confirmation quotes the status, so
/// changing the status changes the words asked — and R10.3's mismatch rule returns the answer to
/// unanswered with no code of its own.
#[test]
fn changing_the_filing_status_re_asks_the_confirmation() {
    use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
    use btctax_core::tax::questions::{QuestionId, FORM_QUESTIONS};
    let mut ri = ReturnInputs {
        tax_year: 2025,
        filing_status: FilingStatus::HoH,
        opened_from: Some(2024),
        ..Default::default()
    };
    let q = FORM_QUESTIONS
        .iter()
        .find(|q| q.id == QuestionId::FilingStatusConfirmed)
        .unwrap();
    (q.set)(&mut ri, true);
    let prompt = q.prompt_text(&ri).into_owned();
    record_answer(
        &mut ri,
        AnswerKey::Question(q.id),
        &prompt,
        time::macros::date!(2026 - 02 - 03),
        AnswerState::Given,
    );
    let named = |ri: &ReturnInputs| {
        btctax_core::tax::interview_state::interview_state(ri)
            .blocking
            .iter()
            .any(|b| b.item == AnswerKey::Question(QuestionId::FilingStatusConfirmed))
    };
    assert!(
        !named(&ri),
        "answered under today's words — it does not block"
    );
    ri.filing_status = FilingStatus::Single;
    assert!(
        named(&ri),
        "the status changed, so the words changed, so the answer no longer stands"
    );
    assert_eq!(
        (q.get)(&ri),
        Some(true),
        "the stored answer is untouched — it is the READING that changed (R10.3)"
    );
}

/// `opened_from` survives the TOML round trip, so a return exported and re-imported keeps the fact
/// that its identities were carried — and with it the confirmation and the hint.
#[test]
fn opened_from_round_trips_through_income_import() {
    let (dir, vault) = fresh_vault();
    let toml = dir.path().join("opened.toml");
    std::fs::write(
        &toml,
        "filing_status = \"HoH\"\nopened_from = 2025\nfiling_status_confirmed = true\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2026, &toml, false, false).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let stored = return_inputs::get(s.conn(), 2026).unwrap().unwrap();
    assert_eq!(stored.opened_from, Some(2025));
    assert_eq!(stored.filing_status_confirmed, Some(true));
}

/// ★ N-2 — `--from` is range-checked BEFORE the arithmetic. `i32::MAX + 1` used to panic under
///   `debug_assertions`, ahead of every refusal the command has.
#[test]
fn an_out_of_range_from_year_refuses_instead_of_overflowing() {
    let (_dir, vault) = fresh_vault();
    let mut s = Session::open(&vault, &pp()).unwrap();
    for year in [i32::MAX, 1900, 3000] {
        let err = open_next_year(&mut s, year, false).expect_err("out of range");
        assert!(
            err.to_string()
                .contains("not a tax year this build can open from"),
            "{year}: {err}"
        );
    }
}

/// ★★★ **T9 / R8 — THE FORM 1098 LENDER IS A PAYER IDENTITY, SEEDED WITH EVERY BOX BLANK.**
///
/// R10.4's sentence is as true of a Form 1098 as of a Form 1099-INT, and more so: a lender sends one
/// for every year the loan is outstanding, so *"Last year Home Savings (TIN 00-0000000) issued you a
/// Form 1098. Did they issue one for 2025?"* is a real prompt with a real answer.
///
/// What must NOT carry is any box — and on this document three of them are testimony a carried value
/// would fabricate: **box 2**, whose SUM drives the §163(h)(3)(B) ceiling warning; **box 3**, the
/// origination date that decides WHICH ceiling; and the **shared-interest gate**, whose carried
/// `Some(false)` would answer *"nobody else paid interest on that mortgage"* for a year the filer has
/// not looked at.
///
/// ★ Both halves, because only the pair is the guarantee: the identity IS carried, and everything
/// else is `Default` — compared against the row's own identity-only seed rather than a hand-list of
/// boxes, which is `row_is_pre_named`'s own rule, so a box added tomorrow is covered the day it is
/// added.
#[test]
fn a_form_1098_lender_is_seeded_as_an_identity_with_every_box_blank() {
    use btctax_core::tax::return_inputs::{Form1098, ScheduleAInputs};
    let (_dir, vault) = vault_with_year_n(|ri| {
        ri.schedule_a = Some(ScheduleAInputs {
            mortgage_all_used_to_buy_build_improve: Some(true),
            mortgage_within_debt_limit: Some(true),
            mortgage_dwelling_is_amt_qualified: Some(true),
            ..Default::default()
        });
        ri.documents.set(DocumentRow::Form1098, Some(true));
        ri.form_1098 = vec![Form1098 {
            lender: "Home Savings".into(),
            lender_tin: "00-0000000".into(),
            box1_interest: dec!(22000),
            box2_outstanding_principal: dec!(400000),
            box3_origination_date: Some(time::macros::date!(2019 - 06 - 01)),
            box6_points: dec!(500),
            box8_property_address: "1 Main St".into(),
            other_borrower_paid_interest: Some(false),
            ..Default::default()
        }];
    });
    let opened = open(&vault, false);
    let id = opened
        .identities
        .iter()
        .find(|i| i.prompt.contains("Form 1098") && !i.prompt.contains("Form 1098-E"))
        .unwrap_or_else(|| panic!("no identity prompt: {:#?}", opened.identities));
    assert!(
        id.prompt.contains("Home Savings") && id.prompt.contains("00-0000000"),
        "the prompt names the lender and its TIN: {}",
        id.prompt
    );
    assert_eq!(id.answer, None, "nothing answers for the filer");

    let seed = draft(&vault);
    assert_eq!(seed.form_1098.len(), 1, "the lender identity is carried");
    assert_eq!(
        seed.form_1098[0],
        Form1098 {
            lender: "Home Savings".into(),
            lender_tin: "00-0000000".into(),
            ..Default::default()
        },
        "★ every BOX is blank — including box 2's balance, box 3's origination date and the \
         shared-interest gate, each of which would be testimony about a year the filer has not \
         looked at. Compared against the identity-only seed, not a box list."
    );
    assert_eq!(
        seed.documents.form_1098, None,
        "…and the census row itself is UNANSWERED: the seed carries an identity, never an answer"
    );
}

/// ★★★ **THE OPENER'S OWN SEED, ON A YEAR THAT TAKES THE STANDARD DEDUCTION, REFUSES NOTHING**
/// (the T9 seam review's C-1 — the reachable path).
///
/// A filer who itemized last year and takes the standard deduction this one does not have to do
/// anything to reach this: `open_next_year` carries the prior lender forward as an identity, and it
/// carries **no `schedule_a`** (`grep -c schedule_a crates/btctax-cli/src/open_next_year.rs` → 0).
/// So the opener itself manufactures the exact shape T9 shipped broken — a Form 1098 row with a
/// blank shared-interest gate on a return with no line 8a — and the filer, having answered nothing,
/// was told that *"btctax adds box 1 to Schedule A line 8a IN FULL"* on a row whose box 1 is $0.
///
/// ★ This is the SECOND kill for C-1 and it is deliberately not a unit fixture: the first
/// (`a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing`) builds the
/// shape by hand, this one takes whatever the shipped opener actually produces. A change to `seed`
/// that started carrying a `schedule_a`, or a new box seeded non-blank, is visible here and nowhere
/// else.
///
/// Mutation: read `ri.form_1098` instead of `ri.form_1098_deducted()` in the shared-interest loop
/// (or in `mortgage_interest_credit_question_live`) and this reds.
#[test]
fn the_opener_seeds_a_1098_onto_a_standard_deduction_year_and_nothing_refuses() {
    use btctax_core::tax::questions::{question_is_live, QuestionId};
    use btctax_core::tax::return_inputs::{Form1098, ScheduleAInputs};
    let (_dir, vault) = vault_with_year_n(|ri| {
        // Year N itemized — which is the only reason there is a lender to carry forward at all.
        ri.schedule_a = Some(ScheduleAInputs {
            mortgage_all_used_to_buy_build_improve: Some(true),
            mortgage_within_debt_limit: Some(true),
            mortgage_dwelling_is_amt_qualified: Some(true),
            ..Default::default()
        });
        ri.documents.set(DocumentRow::Form1098, Some(true));
        ri.form_1098 = vec![Form1098 {
            lender: "Home Savings".into(),
            lender_tin: "00-0000000".into(),
            box1_interest: dec!(22000),
            box2_outstanding_principal: dec!(900000),
            box3_origination_date: Some(time::macros::date!(2019 - 06 - 01)),
            other_borrower_paid_interest: Some(false),
            ..Default::default()
        }];
    });
    open(&vault, false);
    let seed = draft(&vault);

    // The premises, both measured off the shipped opener rather than assumed.
    assert!(
        seed.schedule_a.is_none(),
        "the premise: the opener carries no Schedule A — year N+1 has not elected anything yet"
    );
    assert_eq!(
        seed.form_1098.len(),
        1,
        "the premise: it DOES carry the lender identity"
    );
    assert_eq!(
        seed.form_1098[0].other_borrower_paid_interest, None,
        "the premise: the row's shared-interest gate is blank — the seed answers nothing"
    );

    for q in [
        QuestionId::MortgageAllUsedToBuyBuildImprove,
        QuestionId::AmtQualifiedDwelling,
        QuestionId::MortgageWithinDebtLimit,
        QuestionId::ClaimingMortgageInterestCredit,
    ] {
        assert!(
            !question_is_live(q, &seed),
            "★ THE KILL: {q:?} must not be asked of a filer with no Schedule A"
        );
    }
    assert_eq!(
        btctax_core::tax::return_refuse::screen_param_free(&seed).map(|r| r.reason),
        None,
        "★ THE KILL: the seeded row refuses nothing — a filer who has done NOTHING but open the \
         year cannot be blocked over a Schedule A line 8a their return does not have"
    );
}

/// ★★★ **T16 / FR-76 — THE HSA TRUSTEE IS A PAYER IDENTITY, SEEDED WITH EVERY BOX BLANK.**
///
/// R10.4's sentence is as true of a Form 1099-SA as of a Form 1099-INT: an HSA trustee that reported
/// a distribution last year will report one again whenever money leaves the account, so *"Last year
/// Vanguard (TIN 55-5555555) issued you a Form 1099-SA. Did they issue one for 2025?"* is a real
/// prompt with a real answer. What must NOT carry is any box — and on this document the box that
/// matters most is **box 5**, the HSA / Archer MSA / MA MSA checkbox, because a carried `Hsa` would
/// be testimony about paper the filer has not yet seen.
///
/// ★ Both halves are asserted, because only the pair is the guarantee: the identity IS carried
/// (name and TIN), and everything else is `Default` — compared against the row's own identity-only
/// seed rather than against a hand-list of boxes, which is `row_is_pre_named`'s own rule.
#[test]
fn an_hsa_trustee_is_seeded_as_an_identity_with_every_box_blank() {
    use btctax_core::tax::return_inputs::{Form1099Sa, Form5498Sa, SaAccountType};
    let (_dir, vault) = vault_with_year_n(|ri| {
        ri.sch1.hsa_activity = Some(true);
        ri.documents.set(DocumentRow::Sa1099, Some(true));
        ri.sa_1099 = vec![Form1099Sa {
            payer: "Vanguard HSA".into(),
            payer_tin: "55-5555555".into(),
            box1_gross_distribution: dec!(1800),
            box2_earnings_on_excess: dec!(0),
            box3_distribution_code: "1".into(),
            box5_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        }];
        ri.documents.set(DocumentRow::Sa5498, Some(true));
        ri.sa_5498 = vec![Form5498Sa {
            trustee: "Vanguard HSA".into(),
            trustee_tin: "55-5555555".into(),
            box2_total_contributions: dec!(4150),
            box5_fair_market_value: dec!(22000),
            box6_account_type: Some(SaAccountType::Hsa),
            ..Default::default()
        }];
    });
    let opened = open(&vault, false);
    for doc in ["Form 1099-SA", "Form 5498-SA"] {
        let id = opened
            .identities
            .iter()
            .find(|i| i.prompt.contains(doc))
            .unwrap_or_else(|| panic!("no identity prompt for {doc}: {:#?}", opened.identities));
        assert!(
            id.prompt.contains("Vanguard HSA") && id.prompt.contains("55-5555555"),
            "the prompt names the trustee and its TIN: {}",
            id.prompt
        );
        assert_eq!(id.answer, None, "{doc}: nothing answers for the filer");
    }

    let seed = draft(&vault);
    assert_eq!(seed.sa_1099.len(), 1, "the trustee identity is carried");
    assert_eq!(seed.sa_5498.len(), 1);
    assert_eq!(
        seed.sa_1099[0],
        Form1099Sa {
            payer: "Vanguard HSA".into(),
            payer_tin: "55-5555555".into(),
            ..Default::default()
        },
        "★ every BOX is blank — including box 5, whose carried value would be testimony about paper \
         the filer has not seen. Compared against the identity-only seed, not a box list."
    );
    assert_eq!(
        seed.sa_5498[0],
        Form5498Sa {
            trustee: "Vanguard HSA".into(),
            trustee_tin: "55-5555555".into(),
            ..Default::default()
        }
    );
    assert_eq!(
        (seed.documents.sa_1099, seed.documents.sa_5498),
        (None, None),
        "both census rows are re-asked from blank"
    );
    assert_eq!(
        seed.sch1.hsa_activity, None,
        "and so is the §223 trigger declaration — a PerYear gate, re-asked blank"
    );
    assert_eq!(
        seed.hsa,
        btctax_core::tax::return_inputs::HsaInputs::default(),
        "★ Form 8889's own seven answers and six figures carry NOTHING: last year's coverage, age, \
         Medicare enrolment and contributions are facts about last year"
    );
}
