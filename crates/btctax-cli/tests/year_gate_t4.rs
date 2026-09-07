//! **T4 — the year gate and draft protection (`SPEC_interview.md` R11).**
//!
//! Six deliverables, each with its kill:
//!
//! 1. the two entry states — *interview-complete* and *return-computable* — stated on a year whose
//!    package has not arrived;
//! 2. confirm-before-discard of a non-trivial WIP draft;
//! 3. `income answer` into a draft-only year;
//! 4. `income import` runs the param-free screens BEFORE it writes;
//! 5. `commit` → `NoTables` writes nothing;
//! 6. the TY2026 slice path is unchanged.
//!
//! ★ **TY2026 is the params-less year R11 is about** and it is the real one: `full_return_for(2026)`
//! is `None` by design (`tax_tables.rs`, FR-47), so every "on a year with no package" kill below
//! runs on the year the product actually files, not on a synthetic one.

use btctax_cli::{cmd, Session};
use btctax_store::Passphrase;
use std::path::PathBuf;

/// The params-less year: bundled, declared `preparing`, no `FullReturnParams`.
const NO_PACKAGE_YEAR: i32 = 2026;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn fresh_vault() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    (dir, vault)
}

fn write_toml(dir: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
    let p = dir.path().join(name);
    std::fs::write(&p, body).unwrap();
    p
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// 4. `income import` runs the param-free screens BEFORE it writes.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **R11's headline kill.** On a params-less year a TOML declaring a document btctax cannot
/// take is refused AT IMPORT, naming the exit, and NOTHING is written — the unscreened committed row
/// at `resolve.rs` precedence 1 is the poison `SPEC_input_surface.md` §3.2 describes, and until T4
/// `income import` created one on every params-less year.
#[test]
fn an_unsupported_census_declaration_refuses_at_import_and_writes_no_committed_row() {
    let (dir, vault) = fresh_vault();
    let toml = write_toml(
        &dir,
        "k1.toml",
        "filing_status = \"Single\"\n[documents]\nk1 = true\n",
    );
    let err = cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a declared Schedule K-1 must refuse at import");
    let msg = err.to_string();
    assert!(
        msg.contains("Schedule K-1"),
        "the refusal names the document: {msg}"
    );
    assert!(
        msg.contains("A preparer is the exit for this year."),
        "the refusal carries the row's own EXIT sentence: {msg}"
    );
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR).unwrap(),
        None,
        "a refused import must leave NO committed row — that row is the poison"
    );
}

/// The other half: the SAME shape with every census row supported imports and is stored. Without
/// this the test above would pass on an import that refuses everything.
#[test]
fn the_same_toml_with_every_census_row_supported_imports() {
    let (dir, vault) = fresh_vault();
    let toml = write_toml(
        &dir,
        "ok.toml",
        "filing_status = \"Single\"\n[documents]\nk1 = false\n",
    );
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect("a supported census answers imports on a params-less year");
    assert!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR)
            .unwrap()
            .is_some(),
        "the row is stored"
    );
}

/// ★★★ **The tier is not "everything": an UNANSWERED declaration must still import.**
///
/// `income import` is the ONLY path that creates a committed row and `income answer` is the ONLY
/// path that answers one, so an import that demanded every answer would make `income answer`
/// unreachable for a filer holding a TOML — a brick, not a screen. The census's own VALUE rules do
/// refuse (above); the registry's UNANSWERED tier does not.
#[test]
fn an_unanswered_declaration_still_imports_so_income_answer_remains_reachable() {
    let (dir, vault) = fresh_vault();
    // Not one declaration answered — the shape a filer's first TOML has.
    let toml = write_toml(&dir, "bare.toml", "filing_status = \"Single\"\n");
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false).expect(
        "an unanswered declaration is what `income answer` exists to fill — it must import",
    );
    assert!(cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR)
        .unwrap()
        .is_some());
}

/// A param-DEPENDENT rule does not fire at import: the §402(g) limit lives in `FullReturnParams`,
/// which this year does not have. It waits for commit, exactly as R11 says.
#[test]
fn a_param_dependent_rule_does_not_fire_at_import() {
    let (dir, vault) = fresh_vault();
    let toml = write_toml(
        &dir,
        "deferral.toml",
        "filing_status = \"Single\"\n\
         [documents]\nw2 = true\n\
         [[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"400000\"\nbox2_fed_withheld = \"0\"\n\
         [[w2s.box12]]\ncode = \"D\"\namount = \"99000\"\n",
    );
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect("§402(g) needs the year's package — it waits for commit");
    assert!(cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR)
        .unwrap()
        .is_some());
}

/// A `NegativeAmount` — the integrity gate R11 names — refuses at import too, and writes nothing.
#[test]
fn a_negative_amount_refuses_at_import_and_writes_no_committed_row() {
    let (dir, vault) = fresh_vault();
    let toml = write_toml(
        &dir,
        "neg.toml",
        "filing_status = \"Single\"\n\
         [documents]\nw2 = true\n\
         [[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"-1\"\nbox2_fed_withheld = \"0\"\n",
    );
    let err = cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a negative form-box magnitude is a corrupt import");
    assert!(err.to_string().contains("negative"), "{err}");
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR).unwrap(),
        None
    );
}

/// A declared document with nothing transcribed is *"nothing ever populated it"* — and it refuses at
/// import, because the TOML is the transcription surface and `income answer` cannot add a row.
#[test]
fn a_declared_document_with_no_rows_refuses_at_import() {
    let (dir, vault) = fresh_vault();
    let toml = write_toml(
        &dir,
        "declared.toml",
        "filing_status = \"Single\"\n[documents]\nw2 = true\n",
    );
    let err = cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a declared W-2 with no row must refuse");
    assert!(
        err.to_string().contains("none is \ntranscribed")
            || err.to_string().contains("none is transcribed"),
        "{err}"
    );
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR).unwrap(),
        None
    );
}

/// The screen runs on a params-BEARING year too — one body, one tier list, both callers (TY2024 is
/// the only year with a package in this build).
#[test]
fn the_import_screen_runs_on_a_params_bearing_year_as_well() {
    let (dir, vault) = fresh_vault();
    let toml = write_toml(
        &dir,
        "k1-2024.toml",
        "filing_status = \"Single\"\n[documents]\nk1 = true\n",
    );
    let err = cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false)
        .expect_err("the same declaration refuses on a year that HAS its package");
    assert!(err.to_string().contains("Schedule K-1"), "{err}");
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), 2024).unwrap(),
        None
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// 5. `commit` on a params-less year writes nothing.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ Re-asserted with the DRAFT and the COMMITTED row both measured: `NoTables` must leave the
/// committed row absent AND the draft intact (a commit that consumed the draft while writing
/// nothing would lose the filer's work).
#[test]
fn commit_on_a_params_less_year_writes_no_row_and_keeps_the_draft() {
    use btctax_cli::input_form_store::{self, CommitOutcome};
    let (_dir, vault) = fresh_vault();
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);

    let mut s = Session::open(&vault, &pp()).unwrap();
    input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, &ri).unwrap();
    let outcome = input_form_store::commit(&mut s, NO_PACKAGE_YEAR, &ri, None, None).unwrap();
    assert!(
        matches!(outcome, CommitOutcome::NoTables),
        "a params-less year cannot commit"
    );
    assert!(
        btctax_cli::return_inputs::get(s.conn(), NO_PACKAGE_YEAR)
            .unwrap()
            .is_none(),
        "NoTables writes NO committed row — an unscreened row at precedence 1 is the poison"
    );
    assert!(
        input_form_store::draft_exists(s.conn(), NO_PACKAGE_YEAR).unwrap(),
        "and it does not consume the draft"
    );
}

/// ★ The per-YEAR half of I-11: another year's tables must not commit this year either.
#[test]
fn commit_with_another_years_tables_writes_nothing() {
    use btctax_adapters::tax_tables::{BundledFullReturnTables, BundledTaxTables};
    use btctax_cli::input_form_store::{self, CommitOutcome};
    use btctax_core::tax::tables::{FullReturnTables, TaxTables};
    let (_dir, vault) = fresh_vault();
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);

    let tables = BundledTaxTables::load();
    let full = BundledFullReturnTables::load();
    let t2024 = tables.table_for(2024).expect("TY2024 has a table");
    let p2024 = full.full_return_for(2024).expect("TY2024 has params");

    let mut s = Session::open(&vault, &pp()).unwrap();
    let outcome =
        input_form_store::commit(&mut s, NO_PACKAGE_YEAR, &ri, Some(t2024), Some(p2024)).unwrap();
    assert!(matches!(outcome, CommitOutcome::NoTables));
    assert!(btctax_cli::return_inputs::get(s.conn(), NO_PACKAGE_YEAR)
        .unwrap()
        .is_none());
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// 1. The two entry states.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **R11's kill: a TY2026 fixture with every gate answered reports interview-COMPLETE while
///     `YearReadiness.params` is false.** The two states are independent, and a screen that reads
///     one off the other is the defect R11 exists to fix.
#[test]
fn a_params_less_year_can_be_interview_complete_while_the_return_is_not_computable() {
    use btctax_cli::year_readiness::{EntryStates, YearReadiness};
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);

    assert!(
        !YearReadiness::bundled(NO_PACKAGE_YEAR).params,
        "premise: TY{NO_PACKAGE_YEAR} has no FullReturnParams in this build"
    );
    let st = EntryStates::for_year(NO_PACKAGE_YEAR, Some(&ri));
    assert_eq!(
        st.interview_complete,
        Some(true),
        "every live declaration is answered, so the INTERVIEW is complete"
    );
    assert!(
        !st.return_computable,
        "and the RETURN is still not computable — that is the point"
    );
    let sentence = st.sentence();
    assert!(
        sentence.contains(
            "authoring and saving work; computing and committing wait for the TY2026 package"
        ),
        "R11's own words: {sentence}"
    );

    // The paired half: a year WITH its package says so, and drops the waiting clause.
    let st24 = EntryStates::for_year(2024, Some(&ri));
    assert!(st24.return_computable);
    assert!(
        !st24.sentence().contains("wait for the TY2024 package"),
        "a year whose package is here must not tell the filer to wait: {}",
        st24.sentence()
    );

    // A year the filer has not started asserts NEITHER state of a return that does not exist.
    let none = EntryStates::for_year(NO_PACKAGE_YEAR, None);
    assert_eq!(none.interview_complete, None);
    assert!(none.sentence().contains("interview: not started"));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// 2. Confirm-before-discard of a non-trivial WIP draft.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A draft holding an interview: one answered census row (with the answer RECORDED, which is the
/// part that cannot be re-created by re-typing).
fn draft_holding_an_interview() -> btctax_core::tax::return_inputs::ReturnInputs {
    use btctax_core::tax::document_census::DocumentRow;
    use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
    use btctax_core::tax::questions::FORM_QUESTIONS;
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    let q = FORM_QUESTIONS
        .iter()
        .find(|q| q.id == DocumentRow::K1.question_id())
        .expect("the K-1 census row is a registry question");
    ri.documents.set(DocumentRow::K1, Some(false));
    record_answer(
        &mut ri,
        AnswerKey::Question(q.id),
        q.prompt,
        time::macros::date!(2026 - 09 - 01),
        AnswerState::Given,
    );
    ri
}

/// ★★★ **R11's kill: `income import` over a draft holding an interview REFUSES without
///     `--discard-draft`, and the draft survives BYTE-IDENTICAL.**
///
/// §6.2's rule superseded such a draft with a one-line note on stderr. On a year that cannot commit
/// at all, that note destroyed the only store the interview had.
#[test]
fn import_over_a_draft_holding_an_interview_refuses_and_the_draft_survives_byte_identical() {
    use btctax_cli::input_form_store;
    let (dir, vault) = fresh_vault();
    let draft = draft_holding_an_interview();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, &draft).unwrap();
    }
    let before = raw_draft_json(&vault, NO_PACKAGE_YEAR);

    let toml = write_toml(&dir, "over.toml", "filing_status = \"Single\"\n");
    let err = cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a draft holding an interview is not superseded on a note");
    let msg = err.to_string();
    assert!(
        msg.contains("1 recorded answer(s)"),
        "the refusal NAMES what the draft holds: {msg}"
    );
    assert!(
        msg.contains("--discard-draft"),
        "and names the remedy: {msg}"
    );
    assert_eq!(
        raw_draft_json(&vault, NO_PACKAGE_YEAR),
        before,
        "the draft must survive byte-identical"
    );
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR).unwrap(),
        None,
        "and nothing was committed"
    );
}

/// The other half: with the flag the discard happens and the note names what was lost.
#[test]
fn import_with_discard_draft_deletes_it_and_names_what_was_lost() {
    use btctax_cli::input_form_store;
    let (dir, vault) = fresh_vault();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, &draft_holding_an_interview())
            .unwrap();
    }
    // The note itself goes to stderr; its WORDING is pinned by
    // `input_form_store::tests::the_confirmed_discard_note_names_what_was_lost`, so what this test
    // owns is the OUTCOME: the discard happened and the write went through.
    let toml = write_toml(&dir, "forced.toml", "filing_status = \"Single\"\n");
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, true)
        .expect("--discard-draft is the confirmation");
    {
        let s = Session::open(&vault, &pp()).unwrap();
        assert!(
            !input_form_store::draft_exists(s.conn(), NO_PACKAGE_YEAR).unwrap(),
            "the confirmed discard deletes the draft"
        );
    }
    assert!(cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR)
        .unwrap()
        .is_some());
}

/// A draft holding NOTHING an interview put there keeps today's behaviour: superseded silently.
#[test]
fn a_disposable_draft_is_still_superseded_without_a_flag() {
    use btctax_cli::input_form_store;
    let (dir, vault) = fresh_vault();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let ri = btctax_core::tax::return_inputs::ReturnInputs {
            tax_year: NO_PACKAGE_YEAR,
            filing_status: btctax_core::FilingStatus::Mfj, // non-default, but no interview in it
            ..Default::default()
        };
        input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, &ri).unwrap();
    }
    let toml = write_toml(&dir, "plain.toml", "filing_status = \"Single\"\n");
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect("a disposable WIP draft is superseded, as before T4");
    let s = Session::open(&vault, &pp()).unwrap();
    assert!(!input_form_store::draft_exists(s.conn(), NO_PACKAGE_YEAR).unwrap());
}

/// ★★★ **The §6.3 stale-WIP DISCARD is refused for a draft holding an interview.**
///
/// §6.3 discards a stale-version WIP draft silently *"because it is regenerable"*. An interview is
/// not, so `load` fails closed — and the error names the holdings, which is the payload the TUI's
/// confirm screen shows.
#[test]
fn a_stale_wip_draft_holding_an_interview_is_refused_not_discarded() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = fresh_vault();
    write_raw_draft(
        &vault,
        NO_PACKAGE_YEAR,
        &draft_holding_an_interview(),
        1,
        false,
    );
    let before = raw_draft_json(&vault, NO_PACKAGE_YEAR);

    let s = Session::open(&vault, &pp()).unwrap();
    let msg = match input_form_store::load(s.conn(), NO_PACKAGE_YEAR) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("a stale draft holding an interview must not be discarded"),
    };
    assert!(msg.contains("1 recorded answer(s)"), "{msg}");
    assert!(
        input_form_store::draft_exists(s.conn(), NO_PACKAGE_YEAR).unwrap(),
        "and it is still there"
    );
    drop(s);
    assert_eq!(raw_draft_json(&vault, NO_PACKAGE_YEAR), before);
}

/// The paired half: a stale WIP draft holding NOTHING is still discarded with a note, as §6.3 says.
#[test]
fn a_stale_wip_draft_holding_nothing_is_still_discarded_with_a_note() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = fresh_vault();
    let ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Mfj,
        ..Default::default()
    };
    write_raw_draft(&vault, NO_PACKAGE_YEAR, &ri, 1, false);
    let s = Session::open(&vault, &pp()).unwrap();
    let (_loaded, note) = input_form_store::load(s.conn(), NO_PACKAGE_YEAR)
        .expect("a regenerable stale draft is discarded, not refused");
    assert!(
        note.is_some(),
        "and the caller is given the note to surface"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// 3. `income answer` into a draft-only year.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **R11's kill: on a year with a draft and no committed row, `income answer` reads the draft,
///     writes the draft, and `return_inputs::get` still returns `None`.**
///
/// Before T4 it refused outright ("create one first with `income import`") — which on a params-less
/// year, where the draft is the only store there is, meant the questions could not be answered at
/// all through the CLI.
#[test]
fn income_answer_on_a_draft_only_year_writes_the_draft_and_commits_nothing() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = fresh_vault();
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    // Every live declaration answered EXCEPT one, which this run answers from the keyboard.
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    ri.foreign_trust = None;
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, &ri).unwrap();
    }

    // ★ The script is DERIVED from the registry, never a magic count — the count is exactly what
    //   broke `tax_report`'s script when the interview grew. "n" answers every declaration (this
    //   return carries no document rows, so a "no" census is coherent); a bare Enter passes over
    //   each skippable, which `live_questions` appends after the declarations.
    let asks = btctax_cli::cmd::answer::live_questions(&ri);
    let declarations = asks.iter().filter(|a| !a.is_skippable()).count();
    let script = "n\n".repeat(declarations) + &"\n".repeat(asks.len() - declarations);
    let mut keystrokes = script.as_bytes();
    let mut screen: Vec<u8> = Vec::new();
    cmd::answer::answer_return_inputs(
        &vault,
        &pp(),
        NO_PACKAGE_YEAR,
        time::macros::date!(2026 - 09 - 02),
        &mut keystrokes,
        &mut screen,
        false,
    )
    .expect("a draft-only year is answerable");
    let screen = String::from_utf8(screen).unwrap();
    assert!(
        screen.contains("answering the 2026 DRAFT"),
        "the filer is told WHERE the answers go:\n{screen}"
    );
    assert!(
        screen.contains(
            "authoring and saving work; computing and committing wait for the TY2026 package"
        ),
        "and the panel header states the year gate in R11's words:\n{screen}"
    );

    let s = Session::open(&vault, &pp()).unwrap();
    assert!(
        btctax_cli::return_inputs::get(s.conn(), NO_PACKAGE_YEAR)
            .unwrap()
            .is_none(),
        "R11: it writes the DRAFT, never `return_inputs::set` — the year has no committed row"
    );
    let (loaded, _) = input_form_store::load(s.conn(), NO_PACKAGE_YEAR).unwrap();
    let back = match loaded {
        input_form_store::Loaded::Draft { ri, .. } => ri,
        _ => panic!("the draft must still be the working return"),
    };
    assert_eq!(
        back.foreign_trust,
        Some(false),
        "the answer given at the keyboard is in the draft"
    );
    assert!(
        !back.answer_log.is_empty(),
        "and `record_answer` recorded it — the draft's answers carry provenance too"
    );
}

/// It still refuses when the year has NEITHER a committed row nor a draft (R11: *"It still refuses
/// when neither exists"*).
#[test]
fn income_answer_still_refuses_a_year_with_neither_a_row_nor_a_draft() {
    let (_dir, vault) = fresh_vault();
    let mut keystrokes: &[u8] = b"";
    let mut screen: Vec<u8> = Vec::new();
    let err = cmd::answer::answer_return_inputs(
        &vault,
        &pp(),
        NO_PACKAGE_YEAR,
        time::macros::date!(2026 - 09 - 02),
        &mut keystrokes,
        &mut screen,
        false,
    )
    .expect_err("nothing to answer");
    assert!(
        err.to_string()
            .contains("no full-return inputs and no draft"),
        "{err}"
    );
}

// ── helpers that reach the draft table directly ─────────────────────────────────────────────────

/// The raw stored draft JSON for `year` — the byte-identity check the survival kill needs.
fn raw_draft_json(vault: &std::path::Path, year: i32) -> String {
    let s = Session::open(vault, &pp()).unwrap();
    btctax_cli::input_form_store::init_draft_table(s.conn()).unwrap();
    s.conn()
        .query_row(
            "SELECT inputs_json FROM return_inputs_draft WHERE year=?1",
            [year],
            |r| r.get::<_, String>(0),
        )
        .expect("a draft row exists")
}

/// Write a draft row at an ARBITRARY schema version — the §6.3 stale state, which `save_draft`
/// (always current) cannot produce.
fn write_raw_draft(
    vault: &std::path::Path,
    year: i32,
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
    version: i64,
    parked: bool,
) {
    let mut s = Session::open(vault, &pp()).unwrap();
    btctax_cli::input_form_store::init_draft_table(s.conn()).unwrap();
    let json = serde_json::to_string(ri).unwrap();
    s.conn()
        .execute(
            "INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) \
             VALUES(?1,?2,?3,?4) ON CONFLICT(year) DO UPDATE SET inputs_json=?2, schema_version=?3, parked=?4",
            rusqlite::params![year, json, version, parked as i64],
        )
        .unwrap();
    s.save().unwrap();
}
