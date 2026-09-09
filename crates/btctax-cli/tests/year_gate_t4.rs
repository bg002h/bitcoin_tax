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

/// ★★★ **FR-103's END-TO-END KILL — THE JOURNEY WALK'S OWN TOML, THROUGH THE COMMAND IT WAS TYPED
/// INTO.**
///
/// The walk imported a fully detailed `[[schedule_1a.vehicles]]` row onto TY2024 and got **exit 0
/// and no message of any kind**; the figure was stored, `income show` echoed it back, and it reached
/// no line of any form. Every other out-of-scope item in this codebase refuses or advises; this was
/// the one silent exception found.
///
/// ★★★ **AND THIS TEST EXISTS BECAUSE THE UNIT-LEVEL KILL WAS NOT ENOUGH.** The rule was written,
///     the `screen_param_free` KAT passed on both tiers — and the shipped command still imported the
///     walk's TOML in silence. `income import` screened the parsed row BEFORE `return_inputs::set`
///     stamped `tax_year` from the row key (§G-15), so the screen saw `0` ("not stated") and every
///     year-scoped rule in it was structurally blind on the one command that creates a row. Nothing
///     in the unit tier could see that: they hand the screen a `ReturnInputs` whose year is already
///     set. A KILL THAT DRIVES THE COMMAND IS THE ONLY ONE THAT COULD HAVE CAUGHT IT.
///
/// ★★ **Both halves.** The SAME TOML on TY2025 must import and be STORED — a rule that refused every
///    Schedule 1-A block would pass the first half while bricking the year the schedule exists for,
///    which is the population Part IV was written to serve.
#[test]
fn a_schedule_1a_claim_on_a_pre_2025_year_refuses_at_import_and_writes_no_row() {
    const CAR_LOAN: &str = "filing_status = \"Single\"\n\
                            [[schedule_1a.vehicles]]\n\
                            description = \"2025 pickup\"\n\
                            interest_paid = \"1850\"\n\
                            loan_originated_after_2024 = true\n\
                            loan_originated_by_you = true\n\
                            proceeds_used_to_purchase = true\n\
                            personal_use = true\n\
                            secured_by_first_lien = true\n\
                            original_use_starts_with_you = true\n";

    // ── TY2024: the walk's year. There is no Schedule 1-A on it.
    let (dir, vault) = fresh_vault();
    let toml = write_toml(&dir, "carloan.toml", CAR_LOAN);
    let err = cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false, false).expect_err(
        "★ THE KILL: $1,850 of car-loan interest on a TY2024 return reaches no line of any form, \
         and the walk was told nothing at all",
    );
    let msg = err.to_string();
    for needle in [
        "There is no Schedule 1-A on a TY2024 return",
        "Pub. L. 119-21",
        "2025 through 2028",
        "reach NO line",
    ] {
        assert!(
            msg.contains(needle),
            "the refusal must say {needle:?}: {msg}"
        );
    }
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), 2024).unwrap(),
        None,
        "a refused import must leave NO committed row — a stored figure nothing reads is exactly \
         what the filer mistakes for a deduction"
    );

    // ── TY2025: the year the schedule exists for. Same bytes, and they are STORED.
    let (dir2, vault2) = fresh_vault();
    let toml2 = write_toml(&dir2, "carloan.toml", CAR_LOAN);
    cmd::tax::import_return_inputs(&vault2, &pp(), 2025, &toml2, false, false)
        .expect("TY2025 HAS a Schedule 1-A — the same row must import there");
    let stored = cmd::tax::show_return_inputs(&vault2, &pp(), 2025)
        .unwrap()
        .expect("the TY2025 row is stored");
    assert!(
        stored.contains("1850"),
        "…and the figure the filer typed is actually on the row: {stored}"
    );
}

/// ★★★ **T9 / R8 — A FORM 1098 BOX 4 REFUND REFUSES AT IMPORT, NAMES LINE 8z, AND WRITES NOTHING.**
///
/// The rule is param-free — it reads a box, not a table — so it belongs in the tier `income import`
/// runs BEFORE it writes. That placement is the whole point: `income import` is the only path that
/// creates a committed row, and a box-4 amount stored unscreened is a figure the tool holds with no
/// reader, sitting at `resolve.rs` precedence 1 until `report` finally says so.
///
/// ★ The exit is real rather than a wall: the refund's home is **Schedule 1 line 8z**, and the
///   refusal names it, so the filer knows what a preparer has to do with the number.
///
/// ★★ **Both halves.** The SAME TOML with box 4 at zero imports and is stored — without that, a
///    rule that refused every Form 1098 would pass the first half while bricking every homeowner.
#[test]
fn a_1098_box_4_refund_refuses_at_import_naming_line_8z_and_writes_no_row() {
    let row = |refund: &str| {
        format!(
            "filing_status = \"Single\"\n\
             [documents]\nform_1098 = true\n\
             [schedule_a]\n\
             [[form_1098]]\n\
             lender = \"Home Savings\"\n\
             box1_interest = \"9000\"\n\
             box4_refund_overpaid_interest = \"{refund}\"\n\
             other_borrower_paid_interest = false\n"
        )
    };

    let (dir, vault) = fresh_vault();
    let toml = write_toml(&dir, "refund.toml", &row("300"));
    let err = cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a box-4 refund must refuse at import");
    let msg = err.to_string();
    assert!(
        msg.contains("line 8z"),
        "the refusal names where the refund goes: {msg}"
    );
    assert!(
        msg.contains("REFUND OF OVERPAID INTEREST"),
        "…and what it is refusing on: {msg}"
    );
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR).unwrap(),
        None,
        "★ THE KILL: a refused import leaves NO committed row"
    );

    // The other half — the same TOML with box 4 at zero imports and is stored.
    let ok = write_toml(&dir, "no-refund.toml", &row("0"));
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &ok, false, false)
        .expect("box 4 = 0 is an ordinary Form 1098 and imports");
    assert!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR)
            .unwrap()
            .is_some(),
        "…and it IS stored"
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
        cmd::answer::AnswerOptions::default(),
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
        cmd::answer::AnswerOptions::default(),
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

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ **FOLD — seam review C-1: the disposable predicate is STRUCTURAL, never a category list.**
//
// The shipped `draft_holdings` enumerated four categories (`answer_log`, census document rows,
// dependents, a Schedule A) and anything outside them made a draft *disposable*, i.e. destroyable on
// a note. `ReturnInputs.broker_reporting` — which `input_form_store.rs` itself calls *"the primary
// authoring path on a params-less year"* — is outside them, and so are `schedule_c`, `schedule_1a`,
// `sch1`, the carryovers, `qbi` and the header identity. A TY2026 draft holding the Form 1099-DA
// answers therefore read as EMPTY and was destroyed, unconfirmed, by all four deleting paths.
//
// The fix compares the draft against the YEAR'S FRESH SEED instead. A field added to `ReturnInputs`
// is protected the day it is added, which is the direction the rest of this build takes everywhere.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The reviewer's plant, as a fixture: a TY2026 draft holding two providers' Form 1099-DA answers
/// and a Schedule C — and NOTHING the four counted categories can see.
fn broker_and_schedule_c_draft() -> btctax_core::tax::return_inputs::ReturnInputs {
    use btctax_core::forms::{BrokerReported, CohortAnswers};
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    for provider in ["coinbase", "gemini"] {
        ri.broker_reporting.0.insert(
            provider.to_string(),
            CohortAnswers {
                covered: Some(BrokerReported::BasisMatches),
                noncovered: Some(BrokerReported::NotReported),
            },
        );
    }
    ri.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
        business_description: "consulting".into(),
        ..Default::default()
    });
    // The four counted categories are all empty — that is the whole point of the fixture.
    let held = btctax_cli::input_form_store::draft_holdings(&ri);
    assert!(
        held.is_empty(),
        "premise: the counted categories see NOTHING here — {held:?}"
    );
    ri
}

fn seed_draft(vault: &std::path::Path, ri: &btctax_core::tax::return_inputs::ReturnInputs) {
    let mut s = Session::open(vault, &pp()).unwrap();
    btctax_cli::input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, ri).unwrap();
}

/// ★★★ **C-1 (a) — `income import` must REFUSE over a broker-answers draft, and it must survive.**
#[test]
fn import_over_a_broker_answers_draft_refuses_and_the_draft_survives_byte_identical() {
    let (dir, vault) = fresh_vault();
    seed_draft(&vault, &broker_and_schedule_c_draft());
    let before = raw_draft_json(&vault, NO_PACKAGE_YEAR);

    let toml = write_toml(&dir, "over-broker.toml", "filing_status = \"Single\"\n");
    let err = cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a draft holding the 1099-DA answers is not superseded on a note");
    assert!(
        err.to_string().contains("--discard-draft"),
        "the refusal names the remedy: {err}"
    );
    assert_eq!(
        raw_draft_json(&vault, NO_PACKAGE_YEAR),
        before,
        "the draft must survive byte-identical"
    );
    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR).unwrap(),
        None
    );
}

/// ★★★ **C-1 (b) — `income clear` must REFUSE over the same draft.**
#[test]
fn income_clear_over_a_broker_answers_draft_refuses_and_the_draft_survives_byte_identical() {
    let (_dir, vault) = fresh_vault();
    seed_draft(&vault, &broker_and_schedule_c_draft());
    let before = raw_draft_json(&vault, NO_PACKAGE_YEAR);

    let err = cmd::tax::clear_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, false)
        .expect_err("`income clear` must not destroy the 1099-DA answers on a note");
    assert!(err.to_string().contains("--discard-draft"), "{err}");
    assert_eq!(raw_draft_json(&vault, NO_PACKAGE_YEAR), before);
}

/// ★★★ **C-1 (c) — `load`'s §6.3 stale-WIP discard must REFUSE over the same draft.**
#[test]
fn a_stale_broker_answers_draft_is_refused_not_discarded() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = fresh_vault();
    write_raw_draft(
        &vault,
        NO_PACKAGE_YEAR,
        &broker_and_schedule_c_draft(),
        1,
        false,
    );
    let before = raw_draft_json(&vault, NO_PACKAGE_YEAR);

    let s = Session::open(&vault, &pp()).unwrap();
    match input_form_store::load(s.conn(), NO_PACKAGE_YEAR) {
        Err(e) => assert!(
            e.to_string().contains("NOT discarded"),
            "the refusal says the draft was kept: {e}"
        ),
        Ok(_) => panic!("the §6.3 stale-WIP discard destroyed a draft holding the 1099-DA answers"),
    }
    assert!(input_form_store::draft_exists(s.conn(), NO_PACKAGE_YEAR).unwrap());
    drop(s);
    assert_eq!(raw_draft_json(&vault, NO_PACKAGE_YEAR), before);
}

/// **C-1 (d) — `report --write-carryover` shares the predicate, asserted rather than assumed.**
///
/// It reaches `coherence_clear_or_refuse` on **year N+1**, so the same draft on year N+1 raises the
/// same refusal. Driving the whole write-back needs a computable year N and a committed year N+1
/// row; what is load-bearing here is the PREDICATE, so this asserts the shared entry point directly
/// on the year the command targets.
#[test]
fn the_carryover_write_back_shares_the_same_predicate_on_year_n_plus_1() {
    use btctax_cli::input_form_store;
    let (_dir, vault) = fresh_vault();
    seed_draft(&vault, &broker_and_schedule_c_draft());
    let s = Session::open(&vault, &pp()).unwrap();
    // `write_back_carryover(.., year = 2025, .., discard_draft = false)` calls exactly this.
    let err = input_form_store::coherence_clear_or_refuse(s.conn(), NO_PACKAGE_YEAR, false)
        .expect_err("the write-back's own guard must refuse the same draft");
    assert!(
        matches!(
            err,
            btctax_cli::CliError::NonTrivialDraftBlocksWrite {
                year: NO_PACKAGE_YEAR,
                ..
            }
        ),
        "{err:?}"
    );
    assert!(input_form_store::draft_exists(s.conn(), NO_PACKAGE_YEAR).unwrap());
}

/// ★★★ **C-1 (e) — THE STRUCTURAL KILL, stated without naming any category.**
///
/// A draft differing from the year's fresh seed in exactly ONE uncounted scalar is non-disposable.
/// This is what makes the predicate fail CLOSED: it is written so that a field added to
/// `ReturnInputs` tomorrow is protected the day it is added, and it names no category list — so it
/// cannot be satisfied by extending one.
#[test]
fn a_draft_differing_from_the_seed_in_one_uncounted_field_is_not_disposable() {
    use btctax_cli::input_form_store::draft_is_disposable;
    use btctax_core::tax::return_inputs::ReturnInputs;

    let seed = ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Mfj,
        ..Default::default()
    };
    assert!(
        draft_is_disposable(&seed),
        "the year's fresh seed — a tax year and a filing status — is not work"
    );

    // One scalar, in none of the four counted categories.
    let mut one_field = seed.clone();
    one_field.sch1.state_refund_taxable = rust_decimal_macros::dec!(1);
    assert!(
        !draft_is_disposable(&one_field),
        "a draft that differs from the seed AT ALL holds work"
    );
    assert!(
        btctax_cli::input_form_store::draft_holdings(&one_field).is_empty(),
        "and the counted categories still see nothing — which is exactly why the DECISION must not \
         read them"
    );

    // The message must not then tell the filer "nothing".
    let clause = btctax_cli::input_form_store::describe_draft(&one_field);
    assert!(
        !clause.contains("nothing"),
        "a non-disposable draft is never described as holding nothing: {clause}"
    );
    assert!(
        clause.contains("work not otherwise itemised"),
        "the fallback clause names the unitemised work: {clause}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// FOLD — seam review M-3: a read-only surface CONTINUES on an unreadable draft; a writer refuses.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **M-3.** `load` fails closed on a stale draft holding work, which is right for a writer and
/// wrong for a reader: it would turn *"your draft is unreadable"* into *"you cannot look at this
/// year at all"* on `report`, `income show-broker-answers`, `export-irs-pdf`'s broker path and
/// `income scrub` — a new class of hard failure, reachable the first time `SCHEMA_VERSION` moves.
///
/// The read seam skips the draft (KEEPING it), falls through to the committed row, and returns the
/// note. The write seam is unchanged.
#[test]
fn a_read_only_surface_continues_past_an_unreadable_draft_while_a_writer_still_refuses() {
    use btctax_cli::input_form_store;
    let (dir, vault) = fresh_vault();

    // A committed row stands behind a stale draft this build cannot read.
    let committed = {
        let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
            tax_year: NO_PACKAGE_YEAR,
            filing_status: btctax_core::FilingStatus::Single,
            ..Default::default()
        };
        ri.sch1.state_refund_taxable = rust_decimal_macros::dec!(7);
        ri
    };
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(s.conn(), NO_PACKAGE_YEAR, &committed).unwrap();
        s.save().unwrap();
    }
    write_raw_draft(
        &vault,
        NO_PACKAGE_YEAR,
        &broker_and_schedule_c_draft(),
        1,
        false,
    );
    let before = raw_draft_json(&vault, NO_PACKAGE_YEAR);

    // READ: continues, on the committed row, with the note — and the note says KEPT.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (working, note) = input_form_store::working_return(s.conn(), NO_PACKAGE_YEAR)
            .expect("a reader continues");
        assert_eq!(
            working.as_ref().map(|r| r.sch1.state_refund_taxable),
            Some(rust_decimal_macros::dec!(7)),
            "the committed row behind the skipped draft is what a reader sees"
        );
        let note = note.expect("and it is told the draft was skipped");
        assert!(note.kept_holdings.is_some(), "{note}");
        assert!(
            note.to_string().contains("KEPT") && note.to_string().contains("not deleted"),
            "{note}"
        );
        // The projection every read-only broker surface goes through resolves too.
        input_form_store::broker_answers(s.conn(), NO_PACKAGE_YEAR)
            .expect("`income show-broker-answers` / `report` / the export broker path all resolve");
    }

    // WRITE: still refuses, and the draft is still there, byte-identical.
    let toml = write_toml(&dir, "m3.toml", "filing_status = \"Single\"\n");
    cmd::tax::import_return_inputs(&vault, &pp(), NO_PACKAGE_YEAR, &toml, false, false)
        .expect_err("a WRITER still fails closed on a draft it cannot read");
    assert_eq!(raw_draft_json(&vault, NO_PACKAGE_YEAR), before);
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// FOLD — seam review N-1. (M-2's grep-KAT lives beside the tier's own fixtures, in
// `btctax-core::tax::return_refuse::param_free_tier`.)
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **N-1 — the two new refusal messages read as sentences.** These are what a filer sees at the
/// moment months of work is at stake, and on the TUI discard screen the first is also the payload of
/// the confirmation.
#[test]
fn the_two_new_refusal_messages_carry_no_collapsed_line_breaks() {
    for e in [
        btctax_cli::CliError::NonTrivialDraftBlocksWrite {
            year: NO_PACKAGE_YEAR,
            holdings: "3 recorded answer(s)".into(),
        },
        btctax_cli::CliError::StaleDraftHoldsInterview {
            year: NO_PACKAGE_YEAR,
            found: 1,
            expected: 3,
            holdings: "3 recorded answer(s)".into(),
        },
    ] {
        let msg = e.to_string();
        assert!(
            !msg.contains("  "),
            "a missing `\\` continuation collapses into a run of spaces: {msg:?}"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ SEAM REVIEW M-3's KILL (ii) — A QUESTION THAT DIES MID-ROUND IS NOT ASKED
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// **It drives the REAL `answer_return_inputs` from a keystroke script and reads the SCREEN**, not a
/// re-implementation of its loop. That distinction is I-1's whole finding one file over: a test that
/// re-walks the sweep's own predicates stays green when the sweep stops re-checking them.
///
/// The state: the census says no Form 1099-G, so `StateRefundWithout1099g` is live and answered
/// `Some(true)` in the draft — which makes the §111(a) gate `ItemizedPriorYear` live. Both are
/// therefore in the SAME round's snapshot, the refund question first. The filer then answers the
/// refund question `n` at the keyboard. The gate is dead from that instant, and asking it anyway
/// would put a question to the filer that nothing on the return is asking, and `record_answer`
/// would store the reply.
#[test]
fn a_question_that_dies_earlier_in_the_same_round_is_never_put_to_the_filer() {
    let (_dir, vault) = fresh_vault();
    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        tax_year: NO_PACKAGE_YEAR,
        filing_status: btctax_core::FilingStatus::Single,
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    // The 1099-G row is `No`, so the refund door is live; `Some(true)` there makes the §111(a) gate
    // live too — and the gate is left UNANSWERED, which is what would be asked.
    ri.documents.set(
        btctax_core::tax::document_census::DocumentRow::G1099,
        Some(false),
    );
    ri.state_refund_without_1099g = Some(true);
    ri.itemized_prior_year = None;
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::input_form_store::save_draft(&mut s, NO_PACKAGE_YEAR, &ri).unwrap();
    }

    // THE PREMISE — both questions are in one round's snapshot, the refund question first.
    let asks = btctax_cli::cmd::answer::live_questions(&ri);
    let ids: Vec<btctax_core::tax::questions::QuestionId> = asks
        .iter()
        .filter_map(btctax_cli::cmd::answer::Ask::declaration_id)
        .collect();
    let refund = ids
        .iter()
        .position(|id| *id == btctax_core::tax::questions::QuestionId::StateRefundWithout1099g)
        .expect("the refund door is live");
    let gate = ids
        .iter()
        .position(|id| *id == btctax_core::tax::questions::QuestionId::ItemizedPriorYear)
        .expect("the premise: the §111(a) gate is in THIS round's snapshot");
    assert!(
        refund < gate,
        "the premise: the answer that kills the gate is asked BEFORE it"
    );

    // Every declaration `n` (the census is all-No on a return with no rows, which is coherent), a
    // bare Enter for each skippable. The refund question's `n` is what kills the gate.
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
        cmd::answer::AnswerOptions::default(),
    )
    .expect("a draft-only year is answerable");
    let screen = String::from_utf8(screen).unwrap();

    // The gate's own prompt, from the registry — never a paraphrase.
    let gate_prompt = btctax_core::tax::questions::FORM_QUESTIONS
        .iter()
        .find(|q| q.id == btctax_core::tax::questions::QuestionId::ItemizedPriorYear)
        .expect("the gate is a registry question")
        .prompt_text(&ri)
        .into_owned();
    // ★ The ASK line, not the bare prompt: the "before" panel lists every live question by its own
    //   words too, and matching that would make this assertion about the panel rather than the
    //   sweep. An ask is `"{prompt} [y/n…]: "`.
    let asked_line = format!("{gate_prompt} [y/n");
    assert!(
        screen.contains(&format!(
            "{} [y/n",
            btctax_core::tax::questions::FORM_QUESTIONS
                .iter()
                .find(|q| q.id == btctax_core::tax::questions::QuestionId::StateRefundWithout1099g)
                .expect("the refund question is a registry question")
                .prompt_text(&ri)
        )),
        "the premise: the refund question WAS put to the filer, and this is how an ask looks"
    );
    assert!(
        !screen.contains(&asked_line),
        "★ THE KILL: the §111(a) gate died the moment the refund question was answered `n` earlier \
         in this very round — putting it to the filer anyway records an answer to a question \
         nothing on the return is asking.\n\nscreen:\n{screen}"
    );

    // …and the answer log carries no record of it, which is the half that outlives the session.
    let s = Session::open(&vault, &pp()).unwrap();
    let (loaded, _) = btctax_cli::input_form_store::load(s.conn(), NO_PACKAGE_YEAR).unwrap();
    let stored = match loaded {
        btctax_cli::input_form_store::Loaded::Draft { ri, .. } => ri,
        _ => panic!("the draft is the working return on a params-less year"),
    };
    assert_eq!(
        stored.state_refund_without_1099g,
        Some(false),
        "the premise: the keyboard `n` really landed on the refund question"
    );
    assert_eq!(
        stored.itemized_prior_year, None,
        "★ THE KILL: nothing may be STORED for a question that was never lawfully asked"
    );
}
