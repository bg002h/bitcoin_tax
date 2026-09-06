//! **`btctax extension` KATs** (spec `SPEC_form_4868_1040v.md` R2, task T3) — the Form 4868 command:
//! its refusals in order, the pseudo-reconciled gate, the due-date warning, and one end-to-end run
//! that writes a real PDF.
//!
//! Two things worth stating up front, because both are places this command could be silently wrong:
//!
//! 1. **A refusal must write NO bytes**, and that is checked against the DIRECTORY rather than a
//!    hand-list of filenames that would rot. A signed-ready extension application produced by a year
//!    that refuses is the worst outcome available here.
//! 2. **The due date is never a hardcoded month/day.** TY2017's committed `return_due` is
//!    **2018-04-17** (the Emancipation Day shift), so any test that pinned `04-15` would be asserting
//!    a bug. The pure `extension_due_date` is exercised on a TY2017-shaped record precisely because
//!    TY2017 itself cannot reach this command (no full-return tables) — the refusal comes first.
//!
//! The clock arrives as an ARGUMENT (`now`), which is what `main.rs` fills from the `BTCTAX_NOW`
//! seam. Passing it directly keeps these tests off a process-wide environment variable that parallel
//! tests would race on.

use btctax_cli::{cmd, Session, ATTEST_PHRASE};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime, offset};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn wallet() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn ev(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wallet()),
        payload: p,
    }
}

/// A REAL short-term round-trip in 2024 — no synthetic default, so the ledger is not pseudo-active.
fn real_events_2024() -> Vec<LedgerEvent> {
    vec![
        ev(
            "buy-1",
            datetime!(2024-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(200),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "sell-1",
            datetime!(2024-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 1_000_000,
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
}

/// An unknown-basis inbound consumed by a real Sell ⇒ pseudo-active under pseudo mode.
fn pseudo_events_2024() -> Vec<LedgerEvent> {
    vec![
        ev(
            "in-1",
            datetime!(2024-03-01 12:00 UTC),
            EventPayload::TransferIn(TransferIn {
                sat: 1_000_000,
                src_addr: None,
                txid: None,
            }),
        ),
        ev(
            "sell-1",
            datetime!(2024-06-01 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 400_000,
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
}

/// Build a TY2024 full-return vault, letting the caller shape the inputs and the ledger — the same
/// shape `export_irs_pdf.rs`'s `full_return_vault` uses, so both suites exercise one fixture family.
fn full_return_vault(
    evs: &[LedgerEvent],
    shape: impl FnOnce(&mut btctax_core::tax::return_inputs::ReturnInputs),
) -> (tempfile::TempDir, PathBuf, tempfile::TempDir) {
    use btctax_cli::return_inputs;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::append_import_batch(s.conn(), evs).unwrap();
        s.save().unwrap();
    }
    let out = tempfile::tempdir().unwrap();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        ri.header.taxpayer = btctax_core::tax::return_inputs::Person {
            first_name: "Pat".into(),
            last_name: "Roe".into(),
            ssn: "222-33-4444".into(),
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        shape(&mut ri);
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }
    (dir, vault, out)
}

/// A vault with a ledger but NO stored return inputs for any year.
fn bare_vault(evs: &[LedgerEvent]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    btctax_core::persistence::append_import_batch(s.conn(), evs).unwrap();
    s.save().unwrap();
    (dir, vault)
}

/// Every file the command could write, so "no bytes" is checked against the directory itself rather
/// than a hand-list that would rot.
fn wrote_nothing(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(Result::ok).count() == 0)
        .unwrap_or(true)
}

/// A clock well past every TY2024 due date.
fn late() -> time::OffsetDateTime {
    datetime!(2026-02-01 12:00 UTC)
}

fn contains_bytes(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ── The refusals, in the spec's order ────────────────────────────────────────────────────────────

/// ★ KILL — refusal (1)/(2): a year the command cannot compute an ESTIMATE for is REFUSED, with the
/// export's own message, and NO bytes.
///
/// The instructions demand an estimate *"as accurate as you can"* and warn that an unreasonable one
/// makes the extension *"null and void"* — so a year with no stored inputs has no estimate at all,
/// and inventing one would be worse than refusing.
#[test]
fn a_year_with_no_stored_inputs_is_refused_with_the_exports_own_message_and_no_bytes() {
    let (_d, vault) = bare_vault(&real_events_2024());
    let out = tempfile::tempdir().unwrap();

    let err = cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, late())
        .expect_err("a year with no return inputs has no estimate");
    let msg = format!("{err}");
    assert!(
        msg.contains("no return_inputs stored for 2024"),
        "the packet export's own refusal, reused verbatim: {msg}"
    );
    assert!(
        wrote_nothing(out.path()),
        "★ a refused extension must leave --out untouched"
    );

    // …and refusal (1), the year with no full-return tables at all.
    let out2 = tempfile::tempdir().unwrap();
    let err = cmd::admin::extension(&vault, &pp(), out2.path(), 2025, None, false, None, late())
        .expect_err("TY2025 has no full-return tables in this build");
    assert!(
        format!("{err}").contains("no full-return tables for 2025"),
        "{err}"
    );
    assert!(wrote_nothing(out2.path()));
}

/// ★ KILL — refusal (3): a return that fails a fail-closed SCREEN is refused with the export's
/// "not computable … no forms were written" message, and keeps that promise.
#[test]
fn a_screen_refusal_gives_the_exports_message_and_writes_no_bytes() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |ri| {
        // Un-answer a mandatory class-(A) declaration that `answer_all_live_declarations` had set.
        ri.header.can_be_claimed_as_dependent_taxpayer = None;
    });
    let err = cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, late())
        .expect_err("an unanswered mandatory declaration must refuse the extension too");
    let msg = format!("{err}");
    // MEASURED, not assumed: this reds in `screen_inputs`, which names the unanswered declaration —
    // `the 2024 return is not computable [DependentStatusUnanswered]: … — no forms were written`.
    assert!(
        msg.contains("not computable")
            && msg.contains("DependentStatusUnanswered")
            && msg.contains("no forms were written"),
        "the refusal must name itself, the reason and the promise: {msg}"
    );
    assert!(wrote_nothing(out.path()), "…and write no bytes");
}

/// ★ KILL — refusal (4): `--out` already holds a `manifest.txt`, so it is the RETURN's envelope
/// directory. Form 4868 is mailed separately and weeks earlier; an unlabelled copy sitting in the
/// return's envelope is precisely what the form's own page 2 forbids.
#[test]
fn an_out_directory_holding_a_packet_manifest_is_refused() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |_| {});
    std::fs::write(out.path().join("manifest.txt"), b"# staple in this order\n").unwrap();

    let err = cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, late())
        .expect_err("the packet directory is not where the 4868 goes");
    let msg = format!("{err}");
    assert!(
        msg.contains("manifest.txt") && msg.contains("Don't attach a copy of Form 4868"),
        "the refusal must name what it saw and quote the form: {msg}"
    );
    assert!(
        !out.path().join("f4868.pdf").exists(),
        "and write nothing beside the packet"
    );
}

/// ★ KILL — refusal (5): `--pay` negative or with cents. Paying ABOVE line 6 is deliberately NOT
/// refused: line 6 is an estimate, and paying ahead of it to limit interest is the filer's own call.
#[test]
fn a_negative_or_fractional_pay_is_refused_but_paying_above_line_6_is_not() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |_| {});

    let err = cmd::admin::extension(
        &vault,
        &pp(),
        out.path(),
        2024,
        Some(dec!(-1)),
        false,
        None,
        late(),
    )
    .expect_err("a negative payment is not a payment");
    assert!(format!("{err}").contains("--pay must be >= 0"), "{err}");
    assert!(wrote_nothing(out.path()));

    let err = cmd::admin::extension(
        &vault,
        &pp(),
        out.path(),
        2024,
        Some(dec!(100.50)),
        false,
        None,
        late(),
    )
    .expect_err("cents contradict the form's own all-or-nothing rounding rule");
    let msg = format!("{err}");
    assert!(
        msg.contains("WHOLE DOLLARS") && msg.contains("round all"),
        "the refusal quotes the rule it is enforcing: {msg}"
    );
    assert!(wrote_nothing(out.path()));

    // …and a payment far above the balance due is ALLOWED.
    let rep = cmd::admin::extension(
        &vault,
        &pp(),
        out.path(),
        2024,
        Some(dec!(999_999)),
        false,
        None,
        late(),
    )
    .expect("paying more than line 6 is a choice, not an error");
    assert_eq!(rep.lines.line7, Some(dec!(999_999)));
}

// ── The pseudo-reconciled gate (C-2) ─────────────────────────────────────────────────────────────

/// ★★ KILL — pseudo-active + no attestation ⇒ REFUSED with ZERO bytes; pseudo-active + the phrase ⇒
/// written, and every page DRAFT-watermarked.
///
/// Both halves in one test, because either alone is satisfiable by a broken implementation: a command
/// that always refused would pass the first, one that never watermarked would pass the second. A form
/// MONEY is attached to may not be the exception to this gate (spec C-2).
#[test]
fn a_pseudo_ledger_is_refused_without_the_phrase_and_watermarked_with_it() {
    let (_d, vault, out) = full_return_vault(&pseudo_events_2024(), |_| {});
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).expect("pseudo mode on");

    let err = cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, late())
        .expect_err("a fictional draft may not print an extension unattested");
    assert!(
        format!("{err}").contains(ATTEST_PHRASE),
        "the refusal must name the phrase: {err}"
    );
    assert!(
        wrote_nothing(out.path()),
        "★ a refused attestation leaves --out untouched"
    );

    let rep = cmd::admin::extension(
        &vault,
        &pp(),
        out.path(),
        2024,
        None,
        false,
        Some(ATTEST_PHRASE),
        late(),
    )
    .expect("with the exact phrase the DRAFT form is filled ON PURPOSE");
    assert!(rep.watermarked, "the report says so…");
    let bytes = std::fs::read(&rep.path).unwrap();
    assert!(
        contains_bytes(&bytes, b"NOT FOR FILING"),
        "★ …and the PAGE says so — read back out of the written PDF, not from the report struct"
    );
}

// ── The due date ────────────────────────────────────────────────────────────────────────────────

/// ★ KILL — §7503: a due date on a Saturday moves to the following Monday, a Sunday to the following
/// Monday, and a weekday stands. The three cases in one test, because a shifter that moved
/// everything, or nothing, passes any one of them alone.
#[test]
fn the_section_7503_shifter_moves_a_weekend_to_monday_and_leaves_a_weekday_alone() {
    use btctax_forms::year_record::section_7503_shift;
    // 2025-06-14 is a Saturday, 2025-06-15 a Sunday, 2025-06-16 a Monday, 2024-06-15 a Saturday.
    assert_eq!(
        section_7503_shift(date!(2025 - 06 - 14)),
        date!(2025 - 06 - 16)
    );
    assert_eq!(
        section_7503_shift(date!(2025 - 06 - 15)),
        date!(2025 - 06 - 16)
    );
    assert_eq!(
        section_7503_shift(date!(2025 - 06 - 16)),
        date!(2025 - 06 - 16)
    );
    assert_eq!(
        section_7503_shift(date!(2024 - 06 - 15)),
        date!(2024 - 06 - 17)
    );
    assert_eq!(
        section_7503_shift(date!(2018 - 06 - 15)),
        date!(2018 - 06 - 15)
    );
}

/// ★★ KILL — a TY2017-shaped record never yields `04-15`, and its out-of-country date is
/// **2018-06-15**, not `return_due + 2 months` = 06-17.
///
/// This is the whole reason `extension_due_date` is a pure function taking the record's date: TY2017
/// cannot reach the command at all (it has no full-return tables, so the refusal comes first), and a
/// rule that only ever ran on TY2024 — whose April date happens NOT to be shifted — would never
/// discriminate. TY2017's committed `return_due` is 2018-04-17 because Emancipation Day moved it;
/// two months later is 06-17, and the real out-of-country date is 06-15.
#[test]
fn a_ty2017_shaped_record_never_prints_04_15_and_its_june_date_is_the_15th() {
    use btctax_cli::cmd::admin::extension_due_date;
    let ty2017_return_due = btctax_forms::year_record::YearRecord::for_year(2017)
        .expect("TY2017 has a committed year record")
        .return_due;
    assert_eq!(
        ty2017_return_due,
        date!(2018 - 04 - 17),
        "premise: the committed record already carries the Emancipation Day shift"
    );

    let plain = extension_due_date(2017, ty2017_return_due, false);
    assert_eq!(plain, date!(2018 - 04 - 17));
    assert_ne!(
        plain,
        date!(2018 - 04 - 15),
        "★ a hardcoded April 15 would be wrong for this year"
    );

    let abroad = extension_due_date(2017, ty2017_return_due, true);
    assert_eq!(
        abroad,
        date!(2018 - 06 - 15),
        "the form names June 15 itself; 2018-06-15 is a Friday, so §7503 leaves it alone"
    );
    assert_ne!(
        abroad,
        date!(2018 - 06 - 17),
        "★ NOT return_due + 2 months — that would carry April's own shift into June"
    );
}

/// ★★ KILL — the past-due WARNING, and the pair that makes it mean something: the same run on the
/// same clock warns without `--out-of-country` and does NOT warn with it.
///
/// A warning that never fired, and one that always fired, both pass a single-sided test. May 1 2025
/// is past TY2024's April 15 due date and before the out-of-country June date, which is exactly the
/// window where the two answers differ.
#[test]
fn on_may_1_a_ty2024_extension_is_late_unless_line_8_is_checked() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |_| {});
    let may_1 = datetime!(2025-05-01 12:00 UTC);

    let home =
        cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, may_1).unwrap();
    assert_eq!(
        home.due,
        date!(2025 - 04 - 15),
        "TY2024's committed due date"
    );
    assert!(home.past_due, "May 1 is past April 15");

    let out2 = tempfile::tempdir().unwrap();
    let abroad =
        cmd::admin::extension(&vault, &pp(), out2.path(), 2024, None, true, None, may_1).unwrap();
    assert_eq!(
        abroad.due,
        date!(2025 - 06 - 16),
        "June 15 2025 is a Sunday, so §7503 moves it to Monday the 16th"
    );
    assert!(
        !abroad.past_due,
        "★ an out-of-country filer on May 1 is NOT late — and the un-checked run above proves the \
         warning can still fire"
    );
    assert!(abroad.lines.line8, "line 8 is checked on the paper too");
}

// ── End to end ──────────────────────────────────────────────────────────────────────────────────

/// ★★ The end-to-end KAT: a TY2024 vault with a recorded extension payment prints a REAL Form 4868,
/// owner-only, with the recorded payment as line 7's default and the past-due warning raised under a
/// pinned clock.
#[test]
fn a_ty2024_run_writes_a_real_4868_owner_only_with_the_recorded_payment_on_line_7() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |ri| {
        ri.payments.extension_payment = dec!(500);
    });

    let rep = cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, late())
        .expect("a computable TY2024 year prints its extension");

    assert_eq!(rep.path, out.path().join("f4868.pdf"));
    let bytes = std::fs::read(&rep.path).unwrap();
    assert!(bytes.starts_with(b"%PDF"), "a real PDF was written");
    assert!(!rep.watermarked, "a real ledger prints CLEAN");

    // The recorded payment (Schedule 3 line 10) is the default for line 7, and the report says so —
    // spec I-6: the field records a payment, not a filing, so recording first is never a refusal.
    assert_eq!(rep.recorded_extension_payment, Some(dec!(500)));
    assert_eq!(rep.lines.line7, Some(dec!(500)));

    // ★ The clock is past 2025-04-15, so the warning is raised — printed, never a refusal.
    assert!(rep.past_due);

    // Owner-only, like every other export.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&rep.path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the filled 4868 carries the filer's SSN");
    }

    // The rendered report shows the lines and closes with the form's own sentence.
    let text = btctax_cli::render::render_extension(&rep);
    assert!(text.contains("line 4"), "{text}");
    assert!(
        text.contains("line 7  amount you're paying              $500"),
        "{text}"
    );
    assert!(
        text.contains(
            "$500 is already recorded on the return as paid with the extension; --pay \
                       overrides it"
        ),
        "the I-6 note, verbatim: {text}"
    );
    assert!(
        text.contains("Don't attach a copy of Form 4868 to your return."),
        "the form's own closing sentence: {text}"
    );
    assert!(text.contains("PASSED"), "the due-date warning: {text}");
}

/// ★ A line the form leaves BLANK is rendered `(blank)`, never `0`. On a return signed under 26 USC
/// §6065 those are different assertions, and a filer checking the printout against the page must see
/// the same thing on both.
#[test]
fn the_report_renders_a_blank_line_as_blank_and_never_as_zero() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |ri| {
        // No withholding, no estimated payments, no extension payment ⇒ 1040 line 33 = 0 ⇒ line 5
        // has nothing to print, and the form gives it no `-0-` clause.
        ri.payments = Default::default();
    });
    let rep =
        cmd::admin::extension(&vault, &pp(), out.path(), 2024, None, false, None, late()).unwrap();
    assert_eq!(
        rep.lines.line5, None,
        "premise: this return made no payments"
    );

    let text = btctax_cli::render::render_extension(&rep);
    assert!(
        text.contains("line 5  total payments (excl. Sch 3 L10)  (blank)"),
        "a blank line 5 must render as (blank): {text}"
    );
    assert!(
        text.contains("line 8  out of the country                (blank)"),
        "and so must an unchecked line 8: {text}"
    );
}
