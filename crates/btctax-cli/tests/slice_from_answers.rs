//! spec 1099-DA **R6** — the crypto slice files a LIVE year from the stored Form 1099-DA answers
//! when the year's full return cannot compute.
//!
//! ★★★ **Where these run, and why it is not TY2026.** R6's arms are observable only where a year
//! has BOTH a live (basis-reporting) regime AND bundled form templates, and no bundled year has
//! both: TY2026's record is live and bundles zero templates (R6 N-5 — nothing about a printed slice
//! is observable there), TY2025 bundles fifteen templates under a `proceeds`-only regime. So the
//! printing kills run on **TY2025's templates with the LIVE regime injected**
//! (`btctax_cli::testonly::export_irs_pdf_with_regime`), the pattern
//! `btctax-core/tests/kat_broker_reporting.rs` uses throughout. The kills that do NOT need a
//! template — the refusals, the CSV boxes, `report` — run on the real command with the real record.

use btctax_cli::cli::FormArg;
use btctax_cli::{cmd, input_form_store, return_inputs, testonly, Session};
use btctax_core::event::*;
use btctax_core::forms::{BrokerReported, CohortAnswers, InformationReturnRegime};
use btctax_core::identity::*;
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_forms::testonly::*;
use btctax_forms::Form1040Map;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{datetime, offset};

/// The regime a TY2026-and-later year declares: brokers report proceeds AND basis, so the Form
/// 1099-DA question is LIVE.
const LIVE: InformationReturnRegime = InformationReturnRegime::PROCEEDS_AND_BASIS;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn ev(provider: &str, rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange {
            provider: provider.into(),
            account: "default".into(),
        }),
        payload: p,
    }
}

/// One short-term round-trip on `provider`, in 2025: buy 0.01 BTC @ $200, sell @ $500.
fn round_trip(provider: &str) -> Vec<LedgerEvent> {
    vec![
        ev(
            provider,
            &format!("buy-{provider}"),
            datetime!(2025-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(200),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            provider,
            &format!("sell-{provider}"),
            datetime!(2025-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 1_000_000,
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
}

/// One provider (`cb`). Every lot is acquired in 2025, so every key is **noncovered**
/// (`COVERED_ACQUISITION_START` is 2026-01-01).
fn one_provider() -> Vec<LedgerEvent> {
    round_trip("cb")
}

/// Two providers, so two keys — the fixture behind the **G + I mix**: `cb` answered
/// `basis_matches` (→ Box G) and `gem` answered `not_reported` (→ Box I).
fn two_providers() -> Vec<LedgerEvent> {
    let mut e = round_trip("cb");
    e.extend(round_trip("gem"));
    e
}

fn make_vault(evs: &[LedgerEvent]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    btctax_core::persistence::append_import_batch(s.conn(), evs).unwrap();
    s.save().unwrap();
    (dir, vault)
}

/// A `ReturnInputs` carrying nothing but the Form 1099-DA answers — the shape the TUI's 1099-DA
/// block produces on a params-less year, where no full return will ever be derived from it.
fn answers(pairs: &[(&str, BrokerReported)]) -> ReturnInputs {
    let mut ri = ReturnInputs::default();
    for (provider, a) in pairs {
        ri.broker_reporting.0.insert(
            (*provider).to_string(),
            CohortAnswers {
                covered: None,
                noncovered: Some(*a),
            },
        );
    }
    ri
}

/// Store `ri` in the **DRAFT** table (the TUI input form's row — invisible to `resolve.rs`).
fn save_draft(vault: &Path, year: i32, ri: &ReturnInputs) {
    let mut s = Session::open(vault, &pp()).unwrap();
    input_form_store::save_draft(&mut s, year, ri).unwrap();
}

/// Store `ri` in the **COMMITTED** `return_inputs` row (what `income import` writes).
fn save_committed(vault: &Path, year: i32, ri: &ReturnInputs) {
    let mut s = Session::open(vault, &pp()).unwrap();
    return_inputs::set(s.conn(), year, ri).unwrap();
    s.save().unwrap();
}

/// Write a draft row at an ARBITRARY schema version and parked flag — the §6.3 stale states, which
/// `save_draft` (always current, flag-preserving) cannot produce.
fn save_raw_draft(vault: &Path, year: i32, ri: &ReturnInputs, version: i64, parked: bool) {
    let mut s = Session::open(vault, &pp()).unwrap();
    input_form_store::init_draft_table(s.conn()).unwrap();
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

fn wrote_nothing(dir: &Path) -> bool {
    !dir.exists()
        || std::fs::read_dir(dir)
            .map(|d| d.count() == 0)
            .unwrap_or(true)
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// Arm (2) — the slice PRINTS from the stored answers
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ THE HEADLINE KILL — TY2025 has templates and no `FullReturnParams`, and a filer with stored
/// Form 1099-DA answers now gets the crypto slice. **Both storage paths**: the committed row that
/// `income import` writes, and the DRAFT-only vault the TUI input form leaves behind.
///
/// Before R6 the committed-row half refused outright ("no full-return tables for 2025") because
/// `return_inputs::exists` alone sent it down the full-return arm.
#[test]
fn ty2025_with_stored_answers_prints_the_slice_from_either_row() {
    for draft_only in [false, true] {
        let (_d, vault) = make_vault(&one_provider());
        let ri = answers(&[("cb", BrokerReported::BasisMatches)]);
        if draft_only {
            save_draft(&vault, 2025, &ri);
        } else {
            save_committed(&vault, 2025, &ri);
        }
        {
            let s = Session::open(&vault, &pp()).unwrap();
            assert_eq!(
                return_inputs::exists(s.conn(), 2025).unwrap(),
                !draft_only,
                "premise: the draft-only vault has NO committed row"
            );
            // ★ T9 — and the read-only surfaces see it either way. The viewer's `Snapshot` unions
            //   the committed years with the DRAFT table's, so a year whose answers live only in
            //   the draft (the primary authoring path on a params-less year) is not invisible to
            //   the Forms tab and the TUI export.
            assert!(
                s.broker_reporting_answers().unwrap().contains_key(&2025),
                "draft_only={draft_only}: the year's answers reach the Snapshot"
            );
        }
        let out = tempfile::tempdir().unwrap();
        let dir = out.path().join("slice");
        let rep = testonly::export_irs_pdf_with_regime(
            &vault,
            &pp(),
            &dir,
            2025,
            &[],
            None,
            Default::default(),
            LIVE,
        )
        .unwrap_or_else(|e| panic!("draft_only={draft_only}: the slice must print — {e}"));
        assert!(dir.join("f8949.pdf").exists(), "draft_only={draft_only}");
        assert!(
            dir.join("schedule_d.pdf").exists(),
            "draft_only={draft_only}"
        );
        assert!(
            rep.full_return_paths.is_empty(),
            "premise: this is the SLICE arm, not the full return"
        );
        // ★ R6 (M-6) — the note says WHAT this packet is, and it rides on the report.
        let note = rep
            .slice_attachment_note
            .expect("arm (2) carries the attachment-set note");
        for needle in [
            "full-return parameters are not bundled",
            "ATTACHMENT SET",
            "form_1040_capgains.pdf` is a worksheet, not a return",
        ] {
            assert!(note.contains(needle), "the note says {needle:?}: {note}");
        }
    }
}

/// ★★★ THE T8 KILL, END TO END — a live-regime `basis_matches` slice puts the **G** total on
/// Schedule D line **1b** and leaves line **3 BLANK**, and the Form 8949 behind it is boxed G.
///
/// Line 3 reads *"Totals … with Box C or Box I checked"* — "not reported to the IRS". A
/// broker-reported total printed there is a filed page swearing the opposite of the 1099-DA the IRS
/// already holds.
#[test]
fn a_basis_matches_answer_lands_on_schedule_d_line_1b_not_line_3() {
    let (_d, vault) = make_vault(&one_provider());
    save_draft(
        &vault,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect("the slice prints");

    let map = ScheduleDMap::ty2025();
    let sd = load(&std::fs::read(dir.join("schedule_d.pdf")).unwrap()).unwrap();
    let sidx = index(&collect_fields(&sd).unwrap());
    assert_eq!(
        text_value(
            &sd,
            sidx[map.line1b.as_ref().unwrap().proceeds_d.as_str()].id
        )
        .as_deref(),
        Some("500"),
        "the Box G page-set's proceeds belong on line 1b"
    );
    assert_eq!(
        text_value(&sd, sidx[map.line3.proceeds_d.as_str()].id),
        None,
        "line 3 is \"Box C or Box I\" — with no I rows it must be BLANK"
    );

    // …and the page-set it totals is boxed G.
    let f = load(&std::fs::read(dir.join("f8949.pdf")).unwrap()).unwrap();
    let fidx = index(&collect_fields(&f).unwrap());
    let m8949 = Form8949Map::ty2025();
    let st = m8949.parts.iter().find(|p| p.term == "short").unwrap();
    assert!(
        checkbox_on(&f, fidx[st.boxes["G"].field.as_str()].id).is_some(),
        "the Form 8949 page behind line 1b is boxed G"
    );
    assert!(
        checkbox_on(&f, fidx[st.boxes["I"].field.as_str()].id).is_none(),
        "…and NOT the not-reported box"
    );
}

/// ★★★ THE THREE-ARTIFACT CROSS-CHECK (T8, B1) — on a **G + I** mix, each box group's
/// `schedule_d.csv` row equals that box's Schedule D PDF line AND that box's Form 8949 page-set
/// total; the two groups cross-foot to the Part I total.
///
/// It lives here, in `btctax-cli`, because this is where the CSV is WRITTEN — the forms-side KAT
/// that used to be called `schedule_d_totals_match_form8949_and_csv` never opened one.
#[test]
fn the_csv_the_schedule_d_and_the_8949_carry_the_same_per_box_partition() {
    let (_d, vault) = make_vault(&two_providers());
    save_draft(
        &vault,
        2025,
        &answers(&[
            ("cb", BrokerReported::BasisMatches), // → Box G
            ("gem", BrokerReported::NotReported), // → Box I
        ]),
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect("the slice prints");
    // The CSVs come from `write_form_csvs` — the writer that HOLDS the routed rows, and the one
    // both `export --csv` and the TUI export go through. The regime is its own argument here, so
    // the LIVE injection is the same fact injected the same way as on the PDF side.
    let csv_dir = out.path().join("csv");
    let answers_map = {
        let s = Session::open(&vault, &pp()).unwrap();
        input_form_store::broker_answers(s.conn(), 2025)
            .unwrap()
            .expect("the draft holds the answers")
    };
    let state = {
        let s = Session::open(&vault, &pp()).unwrap();
        s.project().unwrap().0
    };
    btctax_cli::render::write_form_csvs(
        &csv_dir,
        &state,
        2025,
        None,
        &std::collections::BTreeMap::new(),
        LIVE,
        Some(&answers_map),
    )
    .unwrap();

    // ── artifact 1: schedule_d.csv, one row per (part, box) ──
    let csv = std::fs::read_to_string(csv_dir.join("schedule_d.csv")).unwrap();
    let mut rows: Vec<Vec<&str>> = csv
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(',').collect())
        .collect();
    rows.sort_by_key(|r| r[1].to_string());
    assert_eq!(
        rows.iter().map(|r| r[1]).collect::<Vec<_>>(),
        vec!["G", "I"],
        "one CSV row per box group, named by box: {csv}"
    );

    // ── artifact 2: the Schedule D PDF's own lines ──
    let map = ScheduleDMap::ty2025();
    let sd = load(&std::fs::read(dir.join("schedule_d.pdf")).unwrap()).unwrap();
    let sidx = index(&collect_fields(&sd).unwrap());
    let line = |fqn: &str| text_value(&sd, sidx[fqn].id).unwrap_or_default();
    assert_eq!(
        line(&map.line1b.as_ref().unwrap().proceeds_d),
        rows[0][2],
        "line 1b (d) == the CSV's Box G proceeds"
    );
    assert_eq!(
        line(&map.line3.proceeds_d),
        rows[1][2],
        "line 3 (d) == the CSV's Box I proceeds"
    );

    // ── artifact 3: the Form 8949 page-sets ──
    let f = load(&std::fs::read(dir.join("f8949.pdf")).unwrap()).unwrap();
    let mut page_totals: Vec<String> = collect_fields(&f)
        .unwrap()
        .iter()
        .filter(|fl| fl.fqn.ends_with("Page1[0].f1_91[0]"))
        .filter_map(|fl| text_value(&f, fl.id))
        .collect();
    page_totals.sort();
    let mut want = vec![rows[0][2].to_string(), rows[1][2].to_string()];
    want.sort();
    assert_eq!(
        page_totals, want,
        "each 8949 page-set totals its own box, and those are the CSV's two rows"
    );

    // ── and the part total cross-foots ──
    let sum: rust_decimal::Decimal = rows
        .iter()
        .map(|r| r[2].parse::<rust_decimal::Decimal>().unwrap())
        .sum();
    assert_eq!(sum, dec!(1000), "1b(d) + 3(d) = the Part I proceeds total");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// Arm (2)'s gates — every one refuses BEFORE a byte
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ An UNANSWERED key on arm (2) refuses in the **slice's own sentence**, carrying the screen's
/// reason and detail — and never "the return is not computable", which is false here: the slice
/// computes, and one Form 1099-DA answer is missing (R6 M-1).
#[test]
fn an_unanswered_key_refuses_in_the_slices_own_sentence_and_writes_nothing() {
    let (_d, vault) = make_vault(&two_providers());
    // `cb` is answered; `gem` has rows and no answer.
    save_draft(
        &vault,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("an unanswered key must refuse")
    .to_string();
    assert!(
        msg.contains("BrokerReportingUnanswered") && msg.contains("gem"),
        "the slice sentence carries the screen's REASON and names the key: {msg}"
    );
    assert!(
        msg.contains("crypto slice cannot choose a Form 8949 box"),
        "…in the SLICE's own words: {msg}"
    );
    assert!(
        !msg.contains("not computable"),
        "…and never claims the return is not computable: {msg}"
    );
    assert!(wrote_nothing(&dir), "a refusal writes NO bytes");
}

/// ★ A stored answer NO ROW READS refuses too — testimony the tool would silently discard is not
/// kept (`BrokerAnswerUnread`).
#[test]
fn a_stored_answer_no_row_reads_refuses_before_any_byte() {
    let (_d, vault) = make_vault(&one_provider());
    save_draft(
        &vault,
        2025,
        &answers(&[
            ("cb", BrokerReported::BasisMatches),
            ("kraken", BrokerReported::ProceedsOnly), // no TY2025 row falls under this key
        ]),
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("an unread answer must refuse")
    .to_string();
    assert!(
        msg.contains("BrokerAnswerUnread") && msg.contains("kraken"),
        "{msg}"
    );
    assert!(wrote_nothing(&dir), "a refusal writes NO bytes");
}

/// ★ `--pay-by-check` on arm (2) refuses, and the refusal names the REAL reason — the year's
/// full-return parameters are not bundled — never "see `income import`", which a filer on this arm
/// has already done (R6 M-10). Same for `--forms full-return`.
#[test]
fn the_voucher_and_full_return_refusals_are_reworded_on_arm_two() {
    let (_d, vault) = make_vault(&one_provider());
    save_draft(
        &vault,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
    );
    let out = tempfile::tempdir().unwrap();

    let dir = out.path().join("pay");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: None,
        },
        LIVE,
    )
    .expect_err("no Form 1040 line 37 on the slice")
    .to_string();
    assert!(
        msg.contains("there is no Form 1040 line 37 for 2025")
            && msg.contains("not bundled in this build")
            && msg.contains("Authoring inputs will not change that"),
        "the voucher refusal names the real reason: {msg}"
    );
    assert!(!msg.contains("see `income import`"), "{msg}");
    assert!(wrote_nothing(&dir), "a refusal writes NO bytes");

    // ★ the same class, the same reason clause: a `--pay` given WITHOUT `--pay-by-check` is a NOTE
    //   (refusing the whole packet over an inapplicable flag would cost the filer every form), and
    //   on arm (2) it must not claim there are no inputs either.
    let dir_note = out.path().join("note");
    let rep = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir_note,
        2025,
        &[],
        None,
        cmd::admin::VoucherChoice {
            pay_by_check: false,
            pay: Some(dec!(500)),
        },
        LIVE,
    )
    .expect("an inapplicable flag may not cost the filer every form");
    let note = rep
        .form_1040v_note
        .expect("a discarded --pay is never silent");
    assert!(
        note.contains("--pay $500 was IGNORED")
            && note.contains("not bundled in this build")
            && !note.contains("has no full-return inputs"),
        "the note names the real reason on arm (2): {note}"
    );

    let dir2 = out.path().join("fr");
    let msg2 = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir2,
        2025,
        &[FormArg::FullReturn],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("there is no full return for 2025")
    .to_string();
    assert!(
        msg2.contains("not bundled in this build") && !msg2.contains("Author them first"),
        "the --forms refusal names the real reason: {msg2}"
    );
    assert!(wrote_nothing(&dir2), "a refusal writes NO bytes");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// T9 — WHICH row the answers come from
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **THE DRAFT SHADOWS THE COMMITTED ROW, on every surface that files (I-15).** A committed row
/// answering `basis_matches` and a draft answering `not_reported` must file Box **I** (the draft
/// wins); swap the two and it must file Box **G**. Asserted on the CLI PDF export, on
/// `export --csv`, and on the answers the viewer's `Snapshot` carries (which is what the TUI export
/// files from).
#[test]
fn a_draft_shadows_the_committed_row_on_every_surface() {
    for (committed, draft, want_box) in [
        (
            BrokerReported::BasisMatches,
            BrokerReported::NotReported,
            "I",
        ),
        (
            BrokerReported::NotReported,
            BrokerReported::BasisMatches,
            "G",
        ),
    ] {
        let (_d, vault) = make_vault(&one_provider());
        save_committed(&vault, 2025, &answers(&[("cb", committed)]));
        save_draft(&vault, 2025, &answers(&[("cb", draft)]));

        // 1. the CLI PDF export
        let out = tempfile::tempdir().unwrap();
        let dir = out.path().join("slice");
        testonly::export_irs_pdf_with_regime(
            &vault,
            &pp(),
            &dir,
            2025,
            &[],
            None,
            Default::default(),
            LIVE,
        )
        .expect("the slice prints from the DRAFT");
        let f = load(&std::fs::read(dir.join("f8949.pdf")).unwrap()).unwrap();
        let fidx = index(&collect_fields(&f).unwrap());
        let m = Form8949Map::ty2025();
        let st = m.parts.iter().find(|p| p.term == "short").unwrap();
        assert!(
            checkbox_on(&f, fidx[st.boxes[want_box].field.as_str()].id).is_some(),
            "the PDF files Box {want_box} (draft {draft:?} over committed {committed:?})"
        );

        // 2. `export --csv` (which reads the answers through the same T9 accessor)
        let csv_dir = out.path().join("csv");
        cmd::admin::export_snapshot(&vault, &pp(), &csv_dir, Some(2025), None).unwrap();
        // TY2025's own record is proceeds-only, so the CSV writer does not route — assert the
        // ACCESSOR instead, which is the fact under test and the one the TUI export reads.
        let s = Session::open(&vault, &pp()).unwrap();
        assert_eq!(
            input_form_store::broker_answers(s.conn(), 2025)
                .unwrap()
                .and_then(|b| b.answer("cb", btctax_core::forms::Cohort::Noncovered)),
            Some(draft),
            "`export --csv` and the TUI export resolve the DRAFT's answer"
        );
        assert_eq!(
            s.broker_reporting_answers()
                .unwrap()
                .get(&2025)
                .and_then(|b| b.answer("cb", btctax_core::forms::Cohort::Noncovered)),
            Some(draft),
            "…and so does the viewer's Snapshot"
        );
        assert!(csv_dir.join("form8949.csv").exists());
    }
}

/// ★ A **PARKED** draft carries no live answers — the filer switched that return back to a raw
/// tax-profile, so its testimony is WITHDRAWN. Arm (2)'s predicate must not see it, and the export
/// falls to arm (3) rather than filing withdrawn testimony.
#[test]
fn a_parked_draft_is_not_the_working_return_and_the_export_falls_to_arm_three() {
    let (_d, vault) = make_vault(&one_provider());
    save_raw_draft(
        &vault,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
        return_inputs::SCHEMA_VERSION,
        true, // parked
    );
    {
        let s = Session::open(&vault, &pp()).unwrap();
        assert!(
            input_form_store::broker_answers(s.conn(), 2025)
                .unwrap()
                .is_none(),
            "the accessor returns None for a parked draft"
        );
    }
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("arm (3): a live year with no live answers refuses")
    .to_string();
    assert!(
        msg.contains("needs the Form 1099-DA answers"),
        "the arm-(3) refusal: {msg}"
    );
    assert!(wrote_nothing(&dir));
}

/// ★ The §6.3 STALE SPLIT holds unchanged through the accessor: a stale **parked** draft REFUSES
/// before any byte (it may hold carryover that exists only there); a stale **WIP** draft is
/// discarded and the export takes arm (3), surfacing the note.
#[test]
fn the_stale_draft_split_holds_on_the_export_path() {
    let stale = return_inputs::SCHEMA_VERSION - 1;

    // parked + stale → refuse, out_dir absent.
    let (_d, vault) = make_vault(&one_provider());
    save_raw_draft(
        &vault,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
        stale,
        true,
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let err = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("a stale PARKED draft fails closed");
    assert!(
        matches!(err, btctax_cli::CliError::StaleParkedDraft { .. }),
        "got {err:?}"
    );
    assert!(wrote_nothing(&dir), "and it writes NO bytes");

    // WIP + stale → discarded; no answers remain, so arm (3).
    let (_d2, vault2) = make_vault(&one_provider());
    save_raw_draft(
        &vault2,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
        stale,
        false,
    );
    let dir2 = out.path().join("slice2");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault2,
        &pp(),
        &dir2,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("the stale WIP draft is discarded ⇒ arm (3)")
    .to_string();
    assert!(msg.contains("needs the Form 1099-DA answers"), "{msg}");

    // ★ …and when a COMMITTED row stands behind the discarded draft, the export files from it and
    //   SURFACES the note (M-14): the filer must be told the answers they last typed were skipped,
    //   before they read the file list as confirmation of them.
    let (_d3, vault3) = make_vault(&one_provider());
    save_committed(
        &vault3,
        2025,
        &answers(&[("cb", BrokerReported::NotReported)]),
    );
    save_raw_draft(
        &vault3,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
        stale,
        false,
    );
    let dir3 = out.path().join("slice3");
    let rep = testonly::export_irs_pdf_with_regime(
        &vault3,
        &pp(),
        &dir3,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect("the committed row files once the stale draft is discarded");
    let note = rep
        .stale_draft_note
        .expect("the export surfaces the StaleNote the way `scrub` does");
    assert!(
        note.contains("2025") && note.contains("Nothing was deleted"),
        "{note}"
    );
}

/// ★ THE KILL THAT REDS ANY CREATE-A-ROW DESIGN — a TUI-saved 1099-DA block leaves the vault with
/// NO committed row, and a stored `tax_profile` for the year still resolves as `StoredProfile`.
/// Persisting a `ReturnInputs::default()` to make the export work would put `filing_status: Single`
/// — testimony the filer never gave — at the top of the §4.12 precedence ladder and silently
/// replace their profile.
#[test]
fn saving_the_answers_creates_no_row_and_a_stored_profile_still_wins() {
    let (_d, vault) = make_vault(&one_provider());
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::tax_profile::set(
            s.conn(),
            2025,
            &btctax_core::TaxProfile {
                filing_status: btctax_core::FilingStatus::Mfj,
                ordinary_taxable_income: dec!(0),
                magi_excluding_crypto: dec!(0),
                qualified_dividends_and_other_pref_income: dec!(0),
                other_net_capital_gain: dec!(0),
                capital_loss_carryforward_in: Default::default(),
                w2_ss_wages: dec!(0),
                w2_medicare_wages: dec!(0),
                schedule_c_expenses: dec!(0),
            },
        )
        .unwrap();
        s.save().unwrap();
    }
    save_draft(
        &vault,
        2025,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
    );

    let s = Session::open(&vault, &pp()).unwrap();
    assert!(
        !return_inputs::exists(s.conn(), 2025).unwrap(),
        "the draft created NO committed row"
    );
    let (state, _cfg) = s.project().unwrap();
    let tables = btctax_adapters::BundledTaxTables::load();
    let outcome = btctax_cli::resolve::resolve_and_screen(s.conn(), &state, 2025, false, None, {
        use btctax_core::TaxTables;
        tables.table_for(2025)
    })
    .unwrap();
    match outcome {
        btctax_cli::resolve::ProfileOutcome::Ready {
            provenance,
            profile,
        } => {
            assert_eq!(provenance, btctax_cli::resolve::Provenance::StoredProfile);
            assert_eq!(
                profile.unwrap().filing_status,
                btctax_core::FilingStatus::Mfj,
                "the filer's own profile, not a laundered default"
            );
        }
        btctax_cli::resolve::ProfileOutcome::Uncomputable { detail } => {
            panic!("the stored profile must still resolve: {detail}")
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// Arm (1) and arm (3) are unchanged
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ TY2024 (parameters bundled) + inputs → the FULL packet still. R6 widened arm (2), it did not
/// narrow arm (1): a year with inputs AND parameters is the full return, always.
#[test]
fn ty2024_with_inputs_still_exports_the_full_return() {
    let events = vec![
        ev(
            "cb",
            "buy-24",
            datetime!(2024-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(200),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "cb",
            "sell-24",
            datetime!(2024-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 1_000_000,
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ];
    let (_d, vault) = make_vault(&events);
    let mut ri = ReturnInputs {
        filing_status: btctax_core::FilingStatus::Single,
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
    // ★ R9 / T6 — the Digital Assets answer, off this vault's own ledger (it disposes crypto).
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().expect("the fixture ledger projects");
        btctax_core::tax::testonly::reconcile_digital_asset_activity(&mut ri, &state, 2024);
    }
    save_committed(&vault, 2024, &ri);

    let out = tempfile::tempdir().unwrap();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("a year with inputs AND parameters is the full return");
    assert!(!rep.full_return_paths.is_empty(), "the full packet");
    assert!(
        rep.slice_attachment_note.is_none(),
        "a full return is not an attachment set"
    );
}

/// ★ THE FOURTH DISPATCH CELL (M-13) — answers stored in a DRAFT, the year's parameters BUNDLED,
/// and no committed row: that is arm (3), and its refusal names the exit that state actually has —
/// **commit** the return, not "answer them".
#[test]
fn params_bundled_with_a_draft_only_return_names_committing_as_the_exit() {
    let events = vec![
        ev(
            "cb",
            "buy-24",
            datetime!(2024-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(200),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "cb",
            "sell-24",
            datetime!(2024-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 1_000_000,
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ];
    let (_d, vault) = make_vault(&events);
    save_draft(
        &vault,
        2024,
        &answers(&[("cb", BrokerReported::BasisMatches)]),
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2024,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("arm (3): the parameters are bundled, so a COMMIT is the exit")
    .to_string();
    assert!(
        msg.contains("commit the return in the TUI input form")
            && msg.contains("then export the full return"),
        "the refusal names committing, not answering: {msg}"
    );
    assert!(wrote_nothing(&dir));
}

/// The 2025 donation fixture behind the Form 8283 gates: one provider's 2025 round-trip plus a 2020
/// long-term acquisition sent out in 2025, which the caller reclassifies as a `Donate` at its own
/// claimed amount (over $5,000 ⇒ Section B, under ⇒ Section A).
fn donation_events() -> Vec<LedgerEvent> {
    let mut evs = one_provider();
    evs.push(ev(
        "cb",
        "buy-lt",
        datetime!(2020-01-05 12:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(10000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    ));
    evs.push(ev(
        "cb",
        "send-donate",
        datetime!(2025-06-01 12:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000_000,
            fee_sat: None,
            dest_addr: Some("bc1qsyntheticcharity".into()),
            txid: None,
        }),
    ));
    evs
}

/// A FRESH vault holding [`donation_events`] with the 2025 outflow reclassified as a `Donate`
/// claiming `claimed` dollars. Fresh per cell on purpose: a draft row written for one cell of the
/// C-1 sweep would shadow the committed row of the next (§6.1 precedence), and the sweep would then
/// silently exercise arm (2) four times.
fn donation_vault(claimed: &str) -> (tempfile::TempDir, PathBuf) {
    let (d, vault) = make_vault(&donation_events());
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        btctax_core::event::OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg(claimed).unwrap(),
        None,
        None,
        datetime!(2026-01-01 12:00 UTC),
    )
    .unwrap();
    (d, vault)
}

/// ★ THE FORM 8283 RESTRICTION ROW, in its SLICE form — a declared restriction refuses the export
/// and no `form_8283.pdf` is written; the `false` twin exports one. Both halves, because a gate
/// that refused every donation year would pass the first assertion alone.
///
/// It reads the SAME `ReturnInputs` the answers came from (the T9 resolution), never
/// `return_inputs::get` — this vault has no committed row at all, so an implementation that reached
/// for one would see `None`, find no declared restriction, and print the overstated 8283.
#[test]
fn a_declared_donation_restriction_refuses_the_slice_and_writes_no_8283() {
    for restricted in [true, false] {
        // ★★★ spec 1099-DA R6 fold (C-1) — THE 2×2 SWEEP. The gate reads the year's WORKING
        //     return, never the answers, so it must fire on every cell of
        //     {draft row, committed row} × {answers stored, no answers}. The `no answers` column
        //     lands on arm (3): R6 narrowed the dispatch to `params_bundled && return_inputs::exists`,
        //     which let a COMMITTED row on a params-less year fall through with the declared
        //     restriction PRESENT and UNREAD, and `form_8283.pdf` — written from the same
        //     `rows_8283` on both arms — printed the gift at full fair market value.
        //     (committed, no answers) is the cell that printed it on the build as it stood; an empty
        //     `[broker_reporting]` is the NORMAL shape of a TY2025 import, since the 1099-DA
        //     question is not live for a proceeds-only year.
        for committed in [false, true] {
            for with_answers in [false, true] {
                let (_d, vault) = donation_vault("20000.00");
                let mut ri = if with_answers {
                    answers(&[("cb", BrokerReported::BasisMatches)])
                } else {
                    ReturnInputs::default()
                };
                ri.donations_had_restrictions = Some(restricted);
                if committed {
                    save_committed(&vault, 2025, &ri);
                } else {
                    save_draft(&vault, 2025, &ri);
                }
                let cell = format!("committed={committed} with_answers={with_answers}");

                let out = tempfile::tempdir().unwrap();
                let dir = out.path().join("slice");
                // ★ The PRODUCTION path, on TY2025's REAL declared regime (proceeds only), not the
                //   injected LIVE one: that is what makes the `no answers` column land on arm (3)
                //   and RUN — `broker_question_is_live` is false for a proceeds-only year, so R1's
                //   refusal does not fire and the 8283 gate is the only thing between the filer and
                //   an overstated form. It is also the shape of the defect: an empty
                //   `[broker_reporting]` is the normal TY2025 import.
                let r = cmd::admin::export_irs_pdf(
                    &vault,
                    &pp(),
                    &dir,
                    2025,
                    &[],
                    None,
                    Default::default(),
                );
                if restricted {
                    let msg = r
                        .expect_err("a declared restriction must refuse")
                        .to_string();
                    assert!(
                        msg.contains("1.170A-7") && msg.contains("overstates the gift"),
                        "{cell}: the refusal names the regulation and the harm: {msg}"
                    );
                    assert!(
                        !dir.join("form_8283.pdf").exists() && wrote_nothing(&dir),
                        "{cell}: …and no Form 8283 (nor anything else) is written"
                    );
                } else if with_answers {
                    // ★ Arm (2) on a year whose 1099-DA question is not live: R1 refuses the
                    //   UNREAD answers. What this half proves is WHICH refusal — not the §1.170A-7
                    //   one. A hoisted gate that fired on every donation year would be
                    //   indistinguishable from the correct one in the `restricted` half alone.
                    let msg = r
                        .expect_err(
                            "stored answers on a proceeds-only year are unread — R1 refuses",
                        )
                        .to_string();
                    assert!(
                        !msg.contains("1.170A-7"),
                        "{cell}: the `Some(false)` twin is never stopped by the restriction gate: {msg}"
                    );
                    assert!(wrote_nothing(&dir), "{cell}: …and nothing is written");
                } else {
                    // ★★★ THE CONTROL CELL, and the one C-1 lived in: arm (3), nothing refuses, and
                    //     the Form 8283 PRINTS. Flip `donations_had_restrictions` to `Some(true)`
                    //     here — the `restricted` half above — and the very same path must refuse.
                    r.unwrap_or_else(|e| {
                        panic!("{cell}: no restriction ⇒ the slice exports — {e}")
                    });
                    assert!(
                        dir.join("form_8283.pdf").exists(),
                        "{cell}: the `Some(false)` twin DOES print the 8283 — a gate that refused \
                         everything would pass the restricted half on its own"
                    );
                }
            }
        }
    }
}

/// ★★★ spec 1099-DA R6 fold (M-3) KILL — the gate's SECOND row on the slice: an UNANSWERED
/// restriction question refuses when, and only when, the year files a **Section B** Form 8283.
///
/// The full return has carried both rows since §G-21 (`return_1040::assemble_absolute`); R6
/// specified and built only the first on the slice. The decision now lives in one place
/// (`return_refuse::donation_restriction_gate`), and the slice supplies the premises it can see: an
/// 8283 attaches iff the year emits one, and the section is the one the printed rows carry
/// (`forms::form_8283` splits on the §170(f)(11)(C) year aggregate over $5,000).
///
/// Both halves are the test: a Section **A**-sized gift still prints with the question unanswered,
/// because below $5,000 lines 5a/5b/5c are never posed and silence forgoes nothing.
#[test]
fn an_unanswered_restriction_refuses_a_section_b_8283_and_prints_a_section_a_one() {
    for (claimed, section_b) in [("20000.00", true), ("2000.00", false)] {
        let (_d, vault) = donation_vault(claimed);

        let mut ri = answers(&[("cb", BrokerReported::BasisMatches)]);
        ri.donations_had_restrictions = None; // never asked, never answered
        save_draft(&vault, 2025, &ri);

        let out = tempfile::tempdir().unwrap();
        let dir = out.path().join("slice");
        let r = testonly::export_irs_pdf_with_regime(
            &vault,
            &pp(),
            &dir,
            2025,
            &[],
            None,
            Default::default(),
            LIVE,
        );
        if section_b {
            let msg = r
                .expect_err("an unanswered Section B restriction question must refuse")
                .to_string();
            assert!(
                msg.contains("SECTION B") && msg.contains("1.170A-7"),
                "the refusal names the section and the regulation: {msg}"
            );
            assert!(
                !dir.join("form_8283.pdf").exists() && wrote_nothing(&dir),
                "…and nothing is written"
            );
        } else {
            r.expect("a Section A gift asks no restriction question ⇒ the slice exports");
            assert!(
                dir.join("form_8283.pdf").exists(),
                "…and the Section A Form 8283 prints, with 5a/5b/5c blank"
            );
        }
    }
}

/// ★ I-11 IS UNMOVED — a params-less year still cannot COMMIT a full return: `commit` returns
/// `NoTables`, writes no committed row, and leaves the draft intact. R6 widened what the EXPORT
/// does with a draft; it changed nothing about what a commit may create.
#[test]
fn a_params_less_year_still_refuses_to_commit_and_keeps_the_draft() {
    let (_d, vault) = make_vault(&one_provider());
    let ri = answers(&[("cb", BrokerReported::BasisMatches)]);
    save_draft(&vault, 2026, &ri);
    let mut s = Session::open(&vault, &pp()).unwrap();
    let tables = btctax_adapters::BundledTaxTables::load();
    let fr = btctax_adapters::BundledFullReturnTables::load();
    let outcome = {
        use btctax_core::tax::tables::FullReturnTables;
        use btctax_core::TaxTables;
        input_form_store::commit(
            &mut s,
            2026,
            &ri,
            tables.table_for(2026),
            fr.full_return_for(2026),
        )
        .unwrap()
    };
    assert!(
        matches!(outcome, input_form_store::CommitOutcome::NoTables),
        "TY2026 has no FullReturnParams — the commit writes nothing"
    );
    assert!(
        !return_inputs::exists(s.conn(), 2026).unwrap(),
        "no committed row was created"
    );
    assert!(
        input_form_store::draft_exists(s.conn(), 2026).unwrap(),
        "and the draft is left intact"
    );
}

/// ★ `report` in state (2a) — the answers live in the DRAFT row only. WITH a stored `tax_profile`
/// the crypto-delta report computes and exits 0; WITHOUT one it is `NotComputable
/// [TaxProfileMissing]` and exits 1 exactly as before. **Both print the Form 1099-DA answers
/// block** (it is built after the resolve), and the not-computable line gains the clause saying the
/// slice still prints.
#[test]
fn report_in_state_2a_keeps_its_exit_code_and_gains_the_slice_clause() {
    for with_profile in [true, false] {
        let (_d, vault) = make_vault(&one_provider());
        if with_profile {
            let mut s = Session::open(&vault, &pp()).unwrap();
            btctax_cli::tax_profile::set(
                s.conn(),
                2025,
                &btctax_core::TaxProfile {
                    filing_status: btctax_core::FilingStatus::Single,
                    ordinary_taxable_income: dec!(50000),
                    magi_excluding_crypto: dec!(50000),
                    qualified_dividends_and_other_pref_income: dec!(0),
                    other_net_capital_gain: dec!(0),
                    capital_loss_carryforward_in: Default::default(),
                    w2_ss_wages: dec!(0),
                    w2_medicare_wages: dec!(0),
                    schedule_c_expenses: dec!(0),
                },
            )
            .unwrap();
            s.save().unwrap();
        }
        save_draft(
            &vault,
            2025,
            &answers(&[("cb", BrokerReported::NotReported)]),
        );

        let rep = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0))
            .expect("state (2a) never turns `report` into an error");
        // ★ The answers block prints in BOTH sub-states, and it reads the DRAFT (there is no
        //   committed row at all — a `return_inputs::get` implementation would print nothing).
        let block = rep
            .broker_answers
            .expect("the Form 1099-DA answers block prints from the draft");
        assert!(block.contains("cb"), "it names the key: {block}");
        assert!(
            rep.slice_prints_from_answers,
            "TY2025 has answers and no bundled parameters ⇒ the slice still prints"
        );
        let rendered = btctax_cli::render::render_tax_outcome(
            2025,
            &rep.outcome,
            rep.advisory.as_deref(),
            rep.pseudo_contributed,
            rep.slice_prints_from_answers,
        );
        match (&rep.outcome, with_profile) {
            (btctax_core::TaxOutcome::Computed(_), true) => {
                assert!(
                    !rendered.contains("NOT COMPUTABLE"),
                    "with a stored profile the delta report computes (exit 0):\n{rendered}"
                );
            }
            (btctax_core::TaxOutcome::NotComputable(b), false) => {
                assert_eq!(b.kind, btctax_core::BlockerKind::TaxProfileMissing);
                assert!(
                    rendered.contains(
                        "`export-irs-pdf --tax-year 2025` still prints the crypto slice from these \
                         answers"
                    ),
                    "the NOT-COMPUTABLE line must not tell the filer the year is dead:\n{rendered}"
                );
            }
            (o, p) => panic!("with_profile={p}: unexpected outcome {o:?}"),
        }
    }
}

/// ★ …and the clause is NOT printed when it would be false: no answers stored ⇒ nothing to file a
/// slice from. (Without this the flag could be hardcoded `true` and the test above would pass.)
#[test]
fn the_slice_clause_is_absent_when_no_answers_are_stored() {
    let (_d, vault) = make_vault(&one_provider());
    let rep = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    assert!(
        !rep.slice_prints_from_answers,
        "no stored answers ⇒ no slice clause"
    );
    let rendered = btctax_cli::render::render_tax_outcome(
        2025,
        &rep.outcome,
        rep.advisory.as_deref(),
        rep.pseudo_contributed,
        rep.slice_prints_from_answers,
    );
    assert!(
        !rendered.contains("still prints the crypto slice"),
        "{rendered}"
    );
}

/// ★ State (2b) — a COMMITTED row on a params-less year — is unchanged: the outcome is still
/// uncomputable and the inputs are kept, and BOTH sentences a filer meets there now name the slice.
#[test]
fn state_2b_is_unchanged_and_both_sentences_name_the_slice() {
    let sentence = btctax_cli::year_readiness::uncomputable_sentence(2025, true);
    assert!(
        sentence.contains("The inputs are KEPT")
            && sentence.contains("`export-irs-pdf --tax-year 2025` still prints the crypto slice"),
        "the uncomputable sentence keeps the inputs AND names the slice: {sentence}"
    );
    let note =
        btctax_cli::year_readiness::import_note(2025, true).expect("TY2025 has no parameters");
    assert!(
        note.contains("`export-irs-pdf --tax-year 2025` still prints the crypto slice"),
        "the import note names the slice: {note}"
    );
    assert!(
        !note.contains("will refuse"),
        "…and no longer says `report` will refuse when a tax-profile is stored: {note}"
    );
    // A params-BUNDLED year has nothing to say — the full return computes there.
    assert!(btctax_cli::year_readiness::import_note(2024, true).is_none());
}

/// ★★★ spec 1099-DA R6 fold (I-1 + M-4) KILL — the slice clause is conditioned on the predicate the
/// EXPORT applies, not on `full_return_for(year).is_none()` alone.
///
/// Three cells, and the first is the one that was wrong on the headline year: **TY2026** has no
/// bundled `Form8949Map`/`ScheduleDMap` at all, so `export-irs-pdf --tax-year 2026` refuses and
/// writes nothing — while both sentences promised it "still prints the crypto slice from the stored
/// answers". The third cell is M-4: with NO answers stored there is nothing to file a slice from,
/// and neither sentence carried even that term.
#[test]
fn the_slice_clause_is_conditioned_on_the_years_templates_and_on_answers_being_stored() {
    const CLAUSE: &str = "still prints the crypto slice";
    // TY2026 + answers — no templates ⇒ the sentence must NOT promise the slice.
    let s = btctax_cli::year_readiness::uncomputable_sentence(2026, true);
    assert!(
        !s.contains(CLAUSE),
        "TY2026 bundles no Form 8949 / Schedule D map — the export refuses, so the sentence must \
         not promise a slice: {s}"
    );
    let n = btctax_cli::year_readiness::import_note(2026, true).expect("TY2026 has no parameters");
    assert!(
        !n.contains(CLAUSE),
        "…and neither does the import note: {n}"
    );
    // TY2025 + answers — templates bundled, answers stored ⇒ it DOES promise the slice. (Without
    // this half, a `slice_can_print` that returned `false` for every year would pass the others.)
    assert!(btctax_cli::year_readiness::uncomputable_sentence(2025, true).contains(CLAUSE));
    assert!(btctax_cli::year_readiness::import_note(2025, true)
        .unwrap()
        .contains(CLAUSE));
    // TY2025 + NO answers — nothing to file a slice FROM (M-4).
    let s = btctax_cli::year_readiness::uncomputable_sentence(2025, false);
    assert!(
        !s.contains(CLAUSE),
        "no stored answers ⇒ no \"from the stored answers\" promise: {s}"
    );
    let n = btctax_cli::year_readiness::import_note(2025, false).unwrap();
    assert!(
        !n.contains(CLAUSE),
        "…and neither does the import note: {n}"
    );
    // …and the sentences keep everything else they said in every cell.
    for (year, answers) in [(2025, true), (2025, false), (2026, true), (2026, false)] {
        let s = btctax_cli::year_readiness::uncomputable_sentence(year, answers);
        assert!(
            s.contains("The inputs are KEPT") && s.contains(&format!("income clear --year {year}")),
            "TY{year} answers={answers}: {s}"
        );
    }
}

/// ★★★ spec 1099-DA R6 fold (I-1) KILL, at the `report` surface — the same divergence, one layer up.
/// TY2026 holds answers and no bundled parameters, so R6's two-term predicate said the slice prints;
/// the export refuses for want of templates. `stored_answers_reach_the_slice` now carries the third
/// term, so `report` no longer prints a clause the export contradicts.
#[test]
fn report_does_not_promise_the_slice_on_a_year_with_no_bundled_templates() {
    let (_d, vault) = make_vault(&one_provider());
    save_draft(
        &vault,
        2026,
        &answers(&[("cb", BrokerReported::NotReported)]),
    );
    let rep = cmd::tax::report_tax_year(&vault, &pp(), 2026, dec!(0))
        .expect("a params-less year never turns `report` into an error");
    assert!(
        !rep.slice_prints_from_answers,
        "TY2026 bundles no Form 8949 / Schedule D map — the slice cannot print, whatever is stored"
    );
    let rendered = btctax_cli::render::render_tax_outcome(
        2026,
        &rep.outcome,
        rep.advisory.as_deref(),
        rep.pseudo_contributed,
        rep.slice_prints_from_answers,
    );
    assert!(
        !rendered.contains("still prints the crypto slice"),
        "{rendered}"
    );
    // …and the answers themselves are still read and shown (the clause is what changed, not the block).
    assert!(
        rep.broker_answers.is_some_and(|b| b.contains("cb")),
        "the Form 1099-DA answers block still prints from the draft"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ THE DIGITAL ASSETS BOX ON THE SLICE'S FORM 1040 (T6 seam review C-1)
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The two Digital Assets checkboxes as the emitted `form_1040_capgains.pdf` actually carries them:
/// `(yes_on, no_on)`, read back through `Form1040Map::ty2025()` — the map the fill wrote from, so a
/// Yes/No swap in the map cannot make this test agree with itself.
fn da_boxes(dir: &Path) -> (Option<String>, Option<String>) {
    let map = Form1040Map::ty2025();
    let doc = load(&std::fs::read(dir.join("form_1040_capgains.pdf")).unwrap()).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    let yes = map
        .da_yes
        .as_ref()
        .expect("TY2025's 1040 carries the Yes box");
    let no = map
        .da_no
        .as_ref()
        .expect("TY2025's 1040 carries the No box");
    (
        checkbox_on(&doc, idx[yes.field.as_str()].id),
        checkbox_on(&doc, idx[no.field.as_str()].id),
    )
}

/// A `ReturnInputs` carrying the Form 1099-DA answers AND an explicit Digital Assets answer.
fn answers_with_da(pairs: &[(&str, BrokerReported)], da: Option<bool>) -> ReturnInputs {
    let mut ri = answers(pairs);
    ri.digital_asset_activity = da;
    ri
}

/// ★★★ **THE C-1 KILL — the slice's Form 1040 page 1 prints the FILER'S ANSWER, or nothing.**
///
/// Until 2026-09-07 this page's Digital Assets box was decided by a LEDGER PREDICATE
/// (`!rows.is_empty() || income || removals`) on every arm, so on TY2025 — the only year this build
/// can print for, and R6 arm (2) — a filer who had answered **No** got a page with **Yes** checked,
/// and a filer who had never been asked got one too. That is btctax swearing a §6065 declaration for
/// a human, on a page the filer transcribes onto the return they sign.
///
/// Both observable states are measured here off the EMITTED BYTES, not off the inputs:
/// `Some(true)` → the Yes box alone; `None` → **neither**, plus the hand mark that names the blank.
/// (`Some(false)` on a ledger with a disposal is the REFUSAL below — it can never print here,
/// because the page is produced only when the ledger has activity, and activity is what contradicts
/// a `No`.)
#[test]
fn the_slice_prints_the_digital_asset_answer_and_never_the_ledger() {
    for (answer, want_yes, want_no, want_marks) in [
        (Some(true), Some("1"), None, 0usize),
        (None, None, None, 1usize),
    ] {
        let (_d, vault) = make_vault(&one_provider());
        save_draft(
            &vault,
            2025,
            &answers_with_da(&[("cb", BrokerReported::BasisMatches)], answer),
        );
        let out = tempfile::tempdir().unwrap();
        let dir = out.path().join("slice");
        let rep = testonly::export_irs_pdf_with_regime(
            &vault,
            &pp(),
            &dir,
            2025,
            &[],
            None,
            Default::default(),
            LIVE,
        )
        .unwrap_or_else(|e| panic!("answer {answer:?}: the slice must print — {e}"));
        assert!(
            rep.form_1040_path.is_some(),
            "premise: this ledger HAS reportable activity, so the page is produced whatever the \
             answer is — produce/skip is the ledger's question and the box is the filer's"
        );
        let (yes, no) = da_boxes(&dir);
        assert_eq!(
            yes.as_deref(),
            want_yes,
            "answer {answer:?}: the Yes box on the emitted PDF"
        );
        assert_eq!(
            no.as_deref(),
            want_no,
            "answer {answer:?}: the No box on the emitted PDF"
        );
        assert_eq!(
            rep.hand_marks.len(),
            want_marks,
            "answer {answer:?}: an UNANSWERED box is named and an answered one is not — a mark \
             that always fires signals nothing: {:?}",
            rep.hand_marks
        );
        if want_marks == 1 {
            assert!(
                rep.hand_marks[0].contains("Digital Asset question")
                    && rep.hand_marks[0].contains("MANDATORY"),
                "{:?}",
                rep.hand_marks
            );
        }
    }
}

/// ★★★ **THE CROSS-CHECK RUNS ON THE SLICE, BEFORE ANY BYTE** (spec 1099-DA R6 as amended
/// 2026-09-07: of `screen_compute_dependent` the slice runs exactly this one rule).
///
/// A `No` this ledger contradicts REFUSES and names the first qualifying event — the same
/// `RefuseReason::DigitalAssetAnswerContradictsLedger` the full return raises, from the same
/// function, so a filer's slice and their full return cannot disagree about their own answer.
#[test]
fn a_contradicted_no_refuses_the_slice_and_writes_nothing() {
    let (_d, vault) = make_vault(&one_provider());
    save_draft(
        &vault,
        2025,
        &answers_with_da(&[("cb", BrokerReported::BasisMatches)], Some(false)),
    );
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let msg = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect_err("a `No` the ledger contradicts must refuse")
    .to_string();
    for needle in [
        "DigitalAssetAnswerContradictsLedger",
        "2025-06-15",
        "cb",
        "a disposition",
        "No forms were written",
    ] {
        assert!(msg.contains(needle), "the refusal names {needle:?}: {msg}");
    }
    assert!(wrote_nothing(&dir), "…and it refused BEFORE any byte");
}

/// ★★★ **THE MIRROR, ON THE SLICE: an unwitnessed `Yes` PRINTS, with the off-ledger warning.**
///
/// The asymmetry is the whole rule (R9): the ledger is not complete by construction — no
/// self-custody wallet is importable — so refusing a truthful `Yes` would leave `No` as the only way
/// through the gate. The slice's report carried `advisories: Vec::new()`, which made the asymmetry
/// ARM-DEPENDENT: the refusal reached this path and its mirror did not.
#[test]
fn an_off_ledger_yes_prints_the_slice_with_the_advisory() {
    // A 2024 round-trip: TY2025 has NO qualifying event, so a `Yes` for 2025 is off-ledger.
    let mut events = round_trip("cb");
    for e in &mut events {
        e.utc_timestamp -= time::Duration::days(365);
    }
    let (_d, vault) = make_vault(&events);
    save_draft(&vault, 2025, &answers_with_da(&[], Some(true)));
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            btctax_core::form_8949(&state, 2025).is_empty(),
            "premise: nothing happened in 2025 on this ledger"
        );
    }
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let rep = testonly::export_irs_pdf_with_regime(
        &vault,
        &pp(),
        &dir,
        2025,
        &[],
        None,
        Default::default(),
        LIVE,
    )
    .expect("an off-ledger `Yes` is ACCEPTED — it may never refuse");
    assert!(
        rep.advisories.iter().any(|a| matches!(
            a,
            btctax_core::tax::advisories::Advisory::DigitalAssetYesNotOnLedger { year: 2025 }
        )),
        "…and it is WARNED: {:?}",
        rep.advisories
    );
}
