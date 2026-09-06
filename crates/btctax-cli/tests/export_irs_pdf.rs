//! `export-irs-pdf` CLI KATs: a real ledger fills clean official PDFs (Box I checked, no watermark);
//! a pseudo-reconciled ledger is attestation-gated (refused without the phrase; DRAFT-watermarked
//! with it). Mirrors the export-snapshot gate exactly.

use btctax_cli::cli::FormArg;
use btctax_cli::{cmd, CliError, Session, ATTEST_PHRASE};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::PathBuf;
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

/// A REAL short-term round-trip in 2025: buy 0.01 BTC @ $200, sell it @ $500 (gain $300). No synthetic
/// default ⇒ not pseudo-active.
fn real_events() -> Vec<LedgerEvent> {
    vec![
        ev(
            "buy-1",
            datetime!(2025-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(200),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "sell-1",
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

/// An unknown-basis inbound consumed by a real Sell ⇒ pseudo-active under pseudo mode.
fn pseudo_events() -> Vec<LedgerEvent> {
    vec![
        ev(
            "in-1",
            datetime!(2025-03-01 12:00 UTC),
            EventPayload::TransferIn(TransferIn {
                sat: 1_000_000,
                src_addr: None,
                txid: None,
            }),
        ),
        ev(
            "sell-1",
            datetime!(2025-06-01 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 400_000,
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
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

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// UX-P4-8 (fold I2): an `export-irs-pdf --out` that collides with an existing FILE (so the export
/// directory cannot be created) names the out path — not the bare `io: File exists (os error 17)`
/// this item exists to kill — on the flagship official-PDF export.
#[test]
fn export_irs_pdf_out_collision_names_path() {
    let (_dir, vault) = make_vault(&real_events());
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("collide");
    std::fs::write(&out, b"i am a file, not a directory").unwrap();

    let err = cmd::admin::export_irs_pdf(&vault, &pp(), &out, 2025, &[], None, Default::default())
        .expect_err("an --out that collides with a file must error");
    let msg = err.to_string();
    assert!(
        msg.contains(&out.display().to_string()),
        "names the --out path: {msg}"
    );
}

#[test]
fn real_ledger_fills_clean_official_pdfs() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();

    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .expect("real ledger export must succeed");
    assert!(!report.watermarked, "a real ledger fill is NOT watermarked");

    let f8949 = std::fs::read(out.path().join("f8949.pdf")).unwrap();
    let sd = std::fs::read(out.path().join("schedule_d.pdf")).unwrap();
    assert!(f8949.starts_with(b"%PDF") && sd.starts_with(b"%PDF"));
    assert!(
        !contains(&f8949, b"ESTIMATE, NOT FOR FILING"),
        "real fill must NOT carry the DRAFT watermark"
    );

    // Box I (short-term digital assets) must be checked — NOT Box C.
    use btctax_forms::testonly::*;
    let doc = load(&f8949).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[5]"].id).as_deref(),
        Some("6"),
        "Box I checked for short-term BTC"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[2]"].id),
        None,
        "Box C stays off"
    );
}

/// ★★★ A CONTINUATION STATEMENT RIDING WITH DRAFT FORMS MUST SAY SO.
///
/// Every PDF in a pseudo-reconciled packet is stamped `DRAFT — ESTIMATE, NOT FOR FILING`. A `.txt`
/// cannot carry a diagonal watermark, so without an explicit banner the dependents statement would
/// leave the machine looking like a clean page — and it is the one artifact a filer DETACHES, so it is
/// the most likely of all of them to be separated from the forms that carry the warning.
///
/// Both legs, because a banner that always fires is as wrong as one that never does: a clean ledger's
/// statement must be free of it, or the filer learns to ignore the words.
#[test]
fn a_dependents_statement_is_marked_draft_only_on_a_pseudo_ledger() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::{Dependent, ReturnInputs};
    use btctax_core::tax::types::FilingStatus;

    let nine = |ri: &mut ReturnInputs| {
        ri.header.dependents = (0..9)
            .map(|i| Dependent {
                name: format!("Kid {i}"),
                ssn: format!("1112233{:02}", i),
                relationship: "Child".into(),
                ..Default::default()
            })
            .collect();
    };

    // ── Clean ledger: a statement, and NO draft banner. ──
    let (_d1, clean) = make_vault(&real_events());
    {
        let mut s = Session::open(&clean, &pp()).unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        ri.header.taxpayer = btctax_core::tax::return_inputs::Person {
            first_name: "Pat".into(),
            last_name: "Filer".into(),
            ssn: "123456789".into(),
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        nine(&mut ri);
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }
    let out = tempfile::tempdir().unwrap();
    let rep = cmd::admin::export_irs_pdf(
        &clean,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    assert!(!rep.watermarked, "a real ledger is never watermarked");
    let body = std::fs::read_to_string(out.path().join("dependents_statement.txt"))
        .expect("nine dependents ⇒ a statement");
    assert!(
        !body.contains("NOT FOR FILING"),
        "a clean statement must carry NO draft banner:\n{body}"
    );
    assert!(body.contains("Kid 4"), "the overflow rows are there");

    // ── Pseudo-reconciled ledger: the SAME statement, now banner-first. ──
    //
    // ★★★ THIS LEG EXISTS BECAUSE ITS ABSENCE HAD A WRITTEN EXCUSE, AND THE EXCUSE WAS FALSE. The
    //     comment here used to say a pseudo TY2024 full return was unreachable "because the pseudo
    //     fixtures are TY2025, which has no full-return path". But the watermark predicate is
    //     `state.pseudo_active()`, which counts synthetic legs across the WHOLE LEDGER and is not
    //     year-scoped — while the full-vs-slice dispatch keys purely on `return_inputs::exists(conn,
    //     tax_year)`. So a TY2025 pseudo ledger with TY2024 `return_inputs` watermarks a TY2024
    //     full-return export, and the leg was always writable in ~30 lines against fixtures already
    //     in this file. Review r9 wrote it and watched it red.
    //
    //     ★★ Without it, replacing the call site's `watermarked` argument with `false` left
    //     `make check` at 2568/2568 GREEN — producing a banner-free page listing five dependents'
    //     names and full SSNs beside fourteen pages all shouting that the figures are synthetic. An
    //     untested guard is bad; an untested guard with a committed rationale is worse, because the
    //     rationale stops the next person from trying.
    let (_d2, pseudo) = make_vault(&pseudo_events());
    cmd::reconcile::pseudo_set_mode(&pseudo, &pp(), true).unwrap();
    {
        let mut s = Session::open(&pseudo, &pp()).unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        ri.header.taxpayer = btctax_core::tax::return_inputs::Person {
            first_name: "Pat".into(),
            last_name: "Filer".into(),
            ssn: "123456789".into(),
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        nine(&mut ri);
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }
    let out2 = tempfile::tempdir().unwrap();
    let rep2 = cmd::admin::export_irs_pdf(
        &pseudo,
        &pp(),
        out2.path(),
        2024,
        &[],
        Some(btctax_cli::ATTEST_PHRASE),
        Default::default(),
    )
    .expect("a TY2024 full return on a pseudo ledger exports under attestation");
    assert!(rep2.watermarked, "a pseudo ledger IS watermarked");
    let body2 = std::fs::read_to_string(out2.path().join("dependents_statement.txt")).unwrap();
    assert!(
        body2.starts_with("*** DRAFT — ESTIMATE, NOT FOR FILING ***"),
        "the banner must be the FIRST thing on a page the filer detaches:\n{body2}"
    );

    // ★ And the manifest must TELL them to attach it — a page nobody is told to attach may as well
    //   not have been written. Deleting that line reddened nothing before r9.
    let man = std::fs::read_to_string(out2.path().join("manifest.txt")).unwrap();
    assert!(
        man.contains("dependents_statement.txt") && man.contains("attach"),
        "the manifest must name the statement AND say to attach it:\n{man}"
    );
}
#[test]
fn pseudo_fill_requires_attestation() {
    let (_dir, vault) = make_vault(&pseudo_events());
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let out = tempfile::tempdir().unwrap();

    // No attestation ⇒ refused, nothing written.
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::AttestationRequired),
        "pseudo-active export without attestation must be refused, got {err:?}"
    );
    assert!(
        !out.path().join("f8949.pdf").exists(),
        "a refused export writes no PDF"
    );

    // Wrong phrase ⇒ failed.
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        Some("nope"),
        Default::default(),
    )
    .unwrap_err();
    assert!(matches!(err, CliError::AttestationFailed), "got {err:?}");

    // Correct phrase ⇒ permitted AND watermarked.
    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        Some(ATTEST_PHRASE),
        Default::default(),
    )
    .unwrap();
    assert!(report.watermarked, "a pseudo fill must be watermarked");
    let f8949 = std::fs::read(out.path().join("f8949.pdf")).unwrap();
    assert!(
        contains(&f8949, b"ESTIMATE, NOT FOR FILING"),
        "the pseudo fill must carry the DRAFT watermark"
    );
}

/// Business mining income (SE) + a real disposal (1040/8949) in 2025.
fn se_plus_disposal_events() -> Vec<LedgerEvent> {
    let mut evs = real_events();
    // Mining AFTER the June sell, so the sell unambiguously consumes the $200 buy lot (gain $300)
    // regardless of the configured lot-identification method.
    evs.push(ev(
        "mine-1",
        datetime!(2025-08-01 12:00 UTC),
        EventPayload::Income(Income {
            sat: 200_000_000,
            usd_fmv: Some(dec!(100000)),
            fmv_status: FmvStatus::ExchangeProvided,
            kind: IncomeKind::Mining,
            business: true,
        }),
    ));
    evs
}

#[test]
fn sp2_packet_writes_schedule_se_and_1040_capgains() {
    let (_dir, vault) = make_vault(&se_plus_disposal_events());
    // A stored Single profile enables the §1401 Schedule SE computation.
    cmd::tax::set_profile(
        &vault,
        &pp(),
        2025,
        btctax_core::TaxProfile {
            filing_status: btctax_core::FilingStatus::Single,
            ordinary_taxable_income: dec!(0),
            magi_excluding_crypto: dec!(0),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Default::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        },
        false,
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();

    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    // Full packet written; no donation ⇒ no 8283.
    assert!(
        report.schedule_se_path.is_some(),
        "SE written (business mining ≥ $400)"
    );
    assert!(
        report.form_1040_path.is_some(),
        "1040 written (reportable activity)"
    );
    assert!(report.form_1040_filled_7a, "7a filled (active gain)");
    assert!(report.form_8283_path.is_none(), "no donations ⇒ no 8283");
    assert!(!report.se_below_floor && report.se_addl_medicare.is_none());

    use btctax_forms::testonly::*;
    // Schedule SE line 12 = SS + regular Medicare only ($100k mining, Single, no W-2 → 14,129.55).
    let se = std::fs::read(out.path().join("schedule_se.pdf")).unwrap();
    let doc = load(&se).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        text_value(&doc, idx["topmostSubform[0].Page1[0].f1_21[0]"].id).as_deref(),
        Some("14129.55"),
        "SE line 12 = ss + medicare"
    );

    // Form 1040: DA question = YES; line 7a = Schedule D line 16 (gain $300).
    let f1040 = std::fs::read(out.path().join("form_1040_capgains.pdf")).unwrap();
    let doc = load(&f1040).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_10[0]"].id).as_deref(),
        Some("1"),
        "Digital-Asset question = YES"
    );
    assert_eq!(
        text_value(&doc, idx["topmostSubform[0].Page1[0].f1_70[0]"].id).as_deref(),
        Some("300"),
        "1040 line 7a = Schedule D line 16"
    );
}

#[test]
fn sp2_forms_filter_selects_subset() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    // --forms f8949 ⇒ ONLY Form 8949 (no Schedule D, no 1040 even though there is activity).
    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[FormArg::F8949],
        None,
        Default::default(),
    )
    .unwrap();
    assert!(report.f8949_path.is_some());
    assert!(report.schedule_d_path.is_none(), "Schedule D not selected");
    assert!(report.form_1040_path.is_none(), "1040 not selected");
    assert!(out.path().join("f8949.pdf").exists());
    assert!(!out.path().join("schedule_d.pdf").exists());
}

/// ★★★ spec 1099-DA R6 fold (N-1) KILL — `--forms schedule-d` WITHOUT `f8949` is refused, and
/// nothing is written.
///
/// Since T8 the slice's Schedule D carries per-box lines (1b/2/3, 8b/9/10) whose own captions read
/// "Totals for all transactions reported on Form(s) 8949 with Box … checked". Written alone into an
/// export directory the filer mails, that schedule states totals for a page-set the packet does not
/// contain. R6's uniform `wants()` probing let it through; the fold fails closed.
///
/// Both directions, because a refusal that fired on any narrowing would be no better: `--forms
/// f8949` alone still exports (an 8949 cites no attachment of its own), and `f8949,schedule-d`
/// together — the pairing `docs/examples/examples.md` prints — exports both.
#[test]
fn schedule_d_selected_without_form_8949_is_refused_and_writes_nothing() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("slice");
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        &dir,
        2025,
        &[FormArg::ScheduleD],
        None,
        Default::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.contains("`schedule-d` without `f8949`") && err.contains("No forms were written"),
        "the refusal names the missing form and that nothing was written: {err}"
    );
    assert!(!dir.exists(), "…and the export directory was never created");

    // The pairing the docs print still works.
    let both = out.path().join("both");
    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        &both,
        2025,
        &[FormArg::F8949, FormArg::ScheduleD],
        None,
        Default::default(),
    )
    .expect("f8949 + schedule-d together is exactly the honored narrowing");
    assert!(report.f8949_path.is_some() && report.schedule_d_path.is_some());
}

#[test]
fn unsupported_year_is_refused() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    // This build bundles TY2024 + TY2025 (TY2026 is record-only); 2023 is refused.
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2023,
        &[],
        None,
        Default::default(),
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(2023))
        ),
        "only 2024/2025 are bundled, got {err:?}"
    );
    // ★ whole-branch tax M-2: the refusal writes ZERO bytes. Before the pre-`mkdir_out` year check, the
    // slice pipeline had already created the directory and written `basis_methodology.txt` +
    // `form_8275.txt` by the time `fill_form_8949` raised `UnsupportedYear` — a half-populated packet
    // beside a reported failure. (Mutation: move the check back below `mkdir_out` → these red.)
    assert!(
        !out.path().join("basis_methodology.txt").exists(),
        "an unsupported year must leave no half-written packet: basis_methodology.txt was written"
    );
    assert!(
        !out.path().join("form_8275.txt").exists(),
        "an unsupported year must leave no half-written packet: form_8275.txt was written"
    );
}

/// A REAL short-term round-trip in 2024: buy 0.01 BTC @ $200, sell it @ $500 (gain $300).
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

#[test]
fn ty2024_real_ledger_fills_box_c_f_and_line7_and_da() {
    // ★ End-to-end SP3a sanity: a 2024 export fills the OFFICIAL 2024 PDFs — clean (no watermark),
    // XFA dropped, Box C checked (NOT Box I), 1040 line 7 = the gain, and the DA question found via
    // the adjacency oracle (c1_5).
    let (_dir, vault) = make_vault(&real_events_2024());
    let out = tempfile::tempdir().unwrap();
    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("2024 real-ledger export must succeed");
    assert!(!report.watermarked);

    use btctax_forms::testonly::*;
    // Form 8949: Box C (short-term) = c1_1[2] on /3; Box I (c1_1[5]) does not exist on 2024.
    let f8949 = std::fs::read(out.path().join("f8949.pdf")).unwrap();
    assert!(f8949.starts_with(b"%PDF"));
    assert!(!contains(&f8949, b"ESTIMATE, NOT FOR FILING"));
    let doc = load(&f8949).unwrap();
    assert!(!pdf_has_xfa(&doc).unwrap(), "XFA must be dropped");
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[2]"].id).as_deref(),
        Some("3"),
        "Box C checked for short-term BTC on the 2024 form"
    );

    // Form 1040: line 7 (Line4a-11 f1_52) = gain $300; DA question (c1_5[0]) = YES.
    let f1040 = std::fs::read(out.path().join("form_1040_capgains.pdf")).unwrap();
    let doc = load(&f1040).unwrap();
    // ★ Carried over from the deleted TY2017 twin (S9, 2026-09-06): XFA must be dropped from the
    // 1040 too, not only from the 8949 asserted above — an XFA-bearing 1040 renders blank in the
    // reader the filer is most likely to open it in.
    assert!(!pdf_has_xfa(&doc).unwrap(), "XFA must be dropped");
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        text_value(
            &doc,
            idx["topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].f1_52[0]"].id
        )
        .as_deref(),
        Some("300"),
        "1040 line 7 = Schedule D line 16"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_5[0]"].id).as_deref(),
        Some("1"),
        "Digital-Asset question = YES (2024 c1_5, adjacency-selected)"
    );
    // ★ Also carried over from the deleted TY2017 twin: the REPORT says the capital-gain line was
    // filled, so a caller that never opens the PDF still learns it.
    assert!(
        report.form_1040_filled_7a,
        "line 7 filled (active gain) — the report must say so"
    );
}

/// ★★ **S9 KILL — `export-irs-pdf --tax-year 2017` is now a REFUSAL, and it names the years that
/// remain.**
///
/// Until 2026-09-06 this year exported a full crypto-slice packet off five bundled TY2017 PDFs; the
/// owner's S9 ruling (`design/ROADMAP_STATUS.md` §0a, `FOLLOWUPS.md` FR-61) deleted that package
/// because it had no archived authority behind it. The user-visible consequence is exactly this
/// refusal, so it is pinned where a filer would meet it — at the command, not at
/// `btctax_forms::SUPPORTED_YEARS`.
///
/// Two halves, and the second is the one that rots: the error must be `UnsupportedYear(2017)`, and
/// its message must name the years this build DOES bundle, so the filer is sent somewhere. The
/// years in that sentence are derived (`bundled::years_sentence`), and
/// `supported_years_cross_product.rs::the_unsupported_year_refusal_names_exactly_the_supported_years`
/// holds the sentence to the constant; here we only assert the filer is told, and that a refusal
/// writes no bytes.
#[test]
fn ty2017_is_refused_by_the_export_and_the_refusal_names_the_bundled_years() {
    let (_dir, vault) = make_vault(&real_events_2024());
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("slice-2017");
    let err = cmd::admin::export_irs_pdf(&vault, &pp(), &out, 2017, &[], None, Default::default())
        .expect_err("TY2017 has no bundled forms since S9 — the export must refuse");
    assert!(
        matches!(
            err,
            CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(2017))
        ),
        "expected UnsupportedYear(2017), got {err:?}"
    );
    let msg = err.to_string();
    for year in btctax_forms::SUPPORTED_YEARS {
        assert!(
            msg.contains(&year.to_string()),
            "the refusal must send the filer to TY{year}, which this build does bundle: {msg}"
        );
    }
    assert!(
        !msg.contains("2017 "),
        "the refusal must not offer 2017 as a destination: {msg}"
    );
    assert!(!out.exists(), "a refused export writes NO bytes");
}

/// ★ THE DISPATCH, direction 1 (P6.5) — a year WITH full-return inputs gets the **full packet**, not the
/// crypto slice. This replaces the P5-C1 refusal: that guard existed only because the slice's Schedule D
/// carries the crypto totals alone (no line 13 for 1099-DIV box-2a distributions, no lines 6/14 for
/// capital-loss carryovers), so on a full-return year it was a complete-LOOKING form with income missing.
/// The full pipeline fills all of them, plus every attachment the forms cite.
///
/// The two paths write NON-OVERLAPPING filenames, so artifacts from two runs can never be collated into a
/// chimera return — asserted in both directions.
#[test]
fn export_dispatches_a_full_return_year_to_the_full_packet() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();

    // TY2024 is the full-return year (v1 has tables for it); give it inputs — WITH an identity, since
    // an unnamed return is not filable (the packet refuses one; see the KAT below).
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("a full-return year exports the full packet");

    assert!(
        out.path().join("00_f1040.pdf").exists(),
        "the full packet writes sequence-prefixed files"
    );
    assert!(
        out.path().join("manifest.txt").exists(),
        "…and the filer's stapling order"
    );
    assert!(!rep.full_return_paths.is_empty());
    // …and NOT the crypto slice's files: the two name-spaces are disjoint by construction.
    assert!(
        !out.path().join("form_1040_capgains.pdf").exists(),
        "the slice's 1040 must never appear beside the full packet"
    );
    assert!(rep.form_1040_path.is_none());

    // ★★ §G-19d — the full return's ADVISORIES ride out on the report, so the EXPORT path surfaces
    // them. `advisories_for` had exactly ONE production caller (`report --tax-year`), which meant a
    // filer who ran only `export-irs-pdf` saw none of them — on the very path that hands them a PDF
    // to sign. Every advisory names something the return OMITS.
    assert!(
        !rep.advisories.is_empty(),
        "a computed full return must carry its advisories out to the export path"
    );

    // ★ The FULL-return 1040 is a complete return and must NOT carry the partial-worksheet
    // watermark. Half of the guarantee in `crypto_slice_1040_is_watermarked_as_a_worksheet`: a
    // watermark applied to every 1040 would be as wrong as one applied to none.
    let f1040 = std::fs::read(out.path().join("00_f1040.pdf")).unwrap();
    assert!(
        !contains_bytes(&f1040, b"NOT A COMPLETE FORM 1040"),
        "the full-return 1040 IS complete — stamping it a worksheet would be a false disclosure"
    );
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// ★★ `form_1040_capgains.pdf` renders as a Form 1040 — masthead, a populated line 7a, a BLANK line
/// 1a — while btctax vouches for exactly two cells on it. Its only caveat used to be a note on
/// stderr, and **the document outlives the terminal**: a filer who opens this file a month later sees
/// a Form 1040. The disclosure must therefore be ON the page.
#[test]
fn crypto_slice_1040_is_watermarked_as_a_worksheet() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    let report = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    assert!(report.form_1040_path.is_some(), "1040 written");

    // ★ …and the CRYPTO SLICE carries none: it computes no full return, so there is nothing to advise
    // ON. An empty list here is a real assertion, not an absent one — it pins that the slice does not
    // borrow the full return's advisories for a return it never computed.
    assert!(
        report.advisories.is_empty(),
        "the crypto slice computes no full return ⇒ no full-return advisories"
    );

    let f1040 = std::fs::read(out.path().join("form_1040_capgains.pdf")).unwrap();
    assert!(
        contains_bytes(&f1040, b"NOT A COMPLETE FORM 1040"),
        "the crypto-slice 1040 must carry the partial-worksheet watermark on the page itself"
    );
    // A REAL (non-pseudo) ledger: the worksheet stamp is present, the DRAFT stamp is not — they are
    // independent disclosures about different things.
    assert!(
        !contains_bytes(&f1040, b"ESTIMATE, NOT FOR FILING"),
        "a real-ledger export is not a DRAFT estimate"
    );
    // The forms btctax DOES vouch for in full are not worksheets and must stay unstamped.
    for name in ["f8949.pdf", "schedule_d.pdf"] {
        let bytes = std::fs::read(out.path().join(name)).unwrap();
        assert!(
            !contains_bytes(&bytes, b"NOT A COMPLETE FORM 1040"),
            "{name} is a complete crypto-slice form — it must not be stamped a worksheet"
        );
    }
}

/// UX-P4-8 (fold I2): the FULL-RETURN export path (`export_full_return`, dispatched for a
/// full-return year) also names the `--out` path on a collision — the same `mkdir_out` choke point
/// as the crypto-slice path, on a distinct call site.
#[test]
fn export_full_return_out_collision_names_path() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let (_dir, vault) = make_vault(&real_events());
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("collide");
    std::fs::write(&out, b"i am a file, not a directory").unwrap();

    let err = cmd::admin::export_irs_pdf(&vault, &pp(), &out, 2024, &[], None, Default::default())
        .expect_err("a full-return --out that collides with a file must error");
    let msg = err.to_string();
    assert!(
        msg.contains(&out.display().to_string()),
        "names the --out path: {msg}"
    );
}

/// ★★★ B9 — `--forms full-return` on a year with NO full-return inputs must REFUSE, loudly.
///
/// `wants()` is `selected.is_empty() || selected.contains(f)`, so this selection matches no
/// crypto-slice form. Without the guard the export writes an EMPTY directory and exits 0 — which a
/// filer would reasonably read as "there was nothing to file". Silence is the one answer a tax tool
/// may not give here, and it is the failure mode adding the enum variant introduces.
#[test]
fn forms_full_return_on_a_crypto_only_year_refuses_instead_of_writing_nothing() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[FormArg::FullReturn],
        None,
        Default::default(),
    )
    .expect_err("full-return was asked for on a year that has no full-return inputs");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("no full-return inputs") && msg.contains("income import"),
        "the refusal must name the missing inputs AND how to author them: {msg}"
    );
    assert!(
        wrote_nothing(out.path()),
        "a refusal writes no bytes — and an EMPTY export dir with exit 0 is the defect this guards"
    );
}

/// UX-P4-5: a `--forms` SLICE is ignored on a full-return year (honoring part of a jointly-computed
/// 14-form packet is tax-unsound) — the whole packet still writes, and the report FLAGS that the
/// slice was ignored so the caller can warn. With no `--forms`, nothing is ignored.
#[test]
fn forms_slice_ignored_on_full_return_year_is_flagged_and_packet_unchanged() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let (_dir, vault) = make_vault(&real_events());
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    // A --forms slice on a full-return year: ignored, but flagged; the full packet still writes.
    let out = tempfile::tempdir().unwrap();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[FormArg::F8949],
        None,
        Default::default(),
    )
    .expect("full-return export succeeds");
    assert!(
        rep.forms_ignored_full_return,
        "a --forms slice on a full-return year is flagged as ignored"
    );
    assert!(
        !rep.full_return_paths.is_empty(),
        "the full packet still writes despite the ignored slice"
    );

    // ★★★ `--forms full-return` is the ONE selection this path can HONOR, so it is not "ignored" —
    //     the filer asked for exactly what they got. Before the B9 fix clap rejected the value
    //     outright, with a possible-values list that did not contain it and no hint that the right
    //     move is to omit the flag entirely.
    let out_fr = tempfile::tempdir().unwrap();
    let rep_fr = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out_fr.path(),
        2024,
        &[FormArg::FullReturn],
        None,
        Default::default(),
    )
    .expect("--forms full-return on a full-return year succeeds");
    assert!(
        !rep_fr.forms_ignored_full_return,
        "--forms full-return asks for exactly what this path writes — it is HONORED, not ignored"
    );
    assert!(
        !rep_fr.full_return_paths.is_empty(),
        "the full packet writes under --forms full-return"
    );

    // Same year, NO --forms: nothing ignored, and the packet is identical (the slice never changed it).
    let out2 = tempfile::tempdir().unwrap();
    let rep2 = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out2.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("full-return export succeeds");
    assert!(
        !rep2.forms_ignored_full_return,
        "no --forms → nothing was ignored"
    );
    // The path COUNT is unchanged (the process-level KAT below compares the full sorted file-NAME set).
    assert_eq!(
        rep.full_return_paths.len(),
        rep2.full_return_paths.len(),
        "the packet path count is unchanged regardless of --forms (the slice is inert)"
    );
}

/// UX-P4-5 fold r1-I4: the actual STDERR warning fires (process-level) when `--forms` is passed on a
/// full-return year, and NOT otherwise; and the written packet FILE-SET is identical either way (the
/// slice is inert). Pins the user-visible deliverable + the "packet unchanged" contract by name/set.
#[test]
fn forms_slice_on_full_return_year_warns_on_stderr_and_packet_is_identical() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let (_dir, vault) = make_vault(&real_events());
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    let bin = env!("CARGO_BIN_EXE_btctax");
    let run = |out: &std::path::Path, with_forms: bool| -> (String, Vec<String>) {
        let mut args: Vec<String> = vec![
            "--vault".into(),
            vault.to_str().unwrap().into(),
            "export-irs-pdf".into(),
            "--out".into(),
            out.to_str().unwrap().into(),
            "--tax-year".into(),
            "2024".into(),
        ];
        if with_forms {
            args.push("--forms".into());
            args.push("f8949".into());
        }
        let o = std::process::Command::new(bin)
            .args(&args)
            .env("BTCTAX_PASSPHRASE", "pw")
            .output()
            .expect("btctax runs");
        assert!(
            o.status.success(),
            "export must succeed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
        let mut files: Vec<String> = std::fs::read_dir(out)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        (stderr, files)
    };

    let tmp = tempfile::tempdir().unwrap();
    let (stderr_forms, files_forms) = run(&tmp.path().join("with"), true);
    let (stderr_plain, files_plain) = run(&tmp.path().join("without"), false);

    assert!(
        stderr_forms.contains("--forms is ignored on a full-return year"),
        "the warning is emitted with --forms:\n{stderr_forms}"
    );
    assert!(
        !stderr_plain.contains("--forms is ignored"),
        "no warning without --forms:\n{stderr_plain}"
    );
    assert_eq!(
        files_forms, files_plain,
        "the written packet file-set is byte-for-byte identical regardless of --forms"
    );
}

/// ★ THE DISPATCH, direction 2 — a year with NO full-return inputs still gets the crypto slice,
/// unchanged. Deleting the P5-C1 refusal downgraded a type-level impossibility to a branch, so the
/// branch is pinned in BOTH directions.
#[test]
fn export_without_return_inputs_still_gets_the_crypto_slice() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();

    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .expect("a crypto-only year exports the slice");

    assert!(
        rep.full_return_paths.is_empty(),
        "no full packet on this path"
    );
    assert!(
        !out.path().join("00_f1040.pdf").exists(),
        "the full packet's 1040 must never appear on the slice path"
    );
    assert!(
        rep.schedule_d_path.is_some() || rep.f8949_path.is_some(),
        "the slice still produces its own forms"
    );
}

/// ★ An UNNAMED return is not filable — the packet refuses, and writes ZERO bytes.
///
/// This is the compute-vs-packet split the SSN design turns on: the tax math never reads an SSN, so a
/// household that has not entered its PII still gets a REPORT (it can decide whether to file at all).
/// The filable ARTIFACT is what fails closed — no PDF can be produced without an identity.
#[test]
fn a_full_return_without_an_ssn_refuses_and_writes_no_bytes() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(), // no header ⇒ no SSN
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }

    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect_err("an unnamed return must not produce a filable packet");
    assert!(
        format!("{err}").contains("no SSN"),
        "the refusal says what is missing: {err}"
    );
    assert!(
        std::fs::read_dir(out.path())
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "a refused export leaves out_dir EMPTY — never a half-written packet"
    );
}

/// A REAL short-term round-trip in **2025**, for the slice arm of
/// [`the_two_pipelines_cannot_overwrite_each_others_files`]: buy 0.02 BTC @ $400, sell @ $900. The
/// source refs differ from every other fixture in this file, so it composes with `real_events_2024()`
/// without colliding on `EventId` and without either year's lots reaching the other.
fn real_events_2025_for_the_slice_arm() -> Vec<LedgerEvent> {
    vec![
        ev(
            "slice-buy-2025",
            datetime!(2025-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 2_000_000,
                usd_cost: dec!(400),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "slice-sell-2025",
            datetime!(2025-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 2_000_000,
                usd_proceeds: dec!(900),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
}

/// ★ **I7 / r2 NEW-I3 — the two pipelines cannot clobber each other, and this KAT FAILS if they can.**
///
/// The r1 version of this test was VACUOUS: its key assertion (`for name in after − before { assert!(
/// !before.contains(name)) }`) is a set-difference tautology, and a colliding write TRUNCATES IN PLACE,
/// so the filename set is unchanged either way — it passed with the fix reverted. Fable caught it, and
/// it is the same false-safety-claim class the finding itself was about.
///
/// This version snapshots every packet file's BYTES before the second pipeline runs and asserts they are
/// untouched afterwards. This fixture's packet contains **Form 8949 and Schedule D** (an Acquire+Dispose
/// ledger, so no SE income and no Schedule SE) — and those are exactly the names the slice also writes.
/// Revert the sequence-prefix and the slice's CENTS `f8949.pdf` / `schedule_d.pdf` overwrite the packet's
/// whole-dollar ones, and this test fails — which is the whole point. It was the explicit condition on
/// deleting the P5-C1 refusal: a cents form inside a whole-dollar return is the chimera the dispatch
/// mitigation exists to prevent. (Schedule SE collides too, on a ledger that has SE income.)
#[test]
fn the_two_pipelines_cannot_overwrite_each_others_files() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;
    use std::collections::BTreeMap;

    // ★ The ledger's crypto activity must be in 2024, so the PACKET actually contains the forms that
    // collide (here: Schedule D + Form 8949). With a crypto-less 2024 the packet is a lone 1040 and the
    // test cannot fail even with the fix reverted — which is precisely how the r1 version was vacuous.
    //
    // ★ And the SLICE year needs its own crypto too, for the same reason: a slice that writes no
    // f8949/schedule_d cannot clobber anything. The slice year was 2017 until S9 dropped that
    // package (2026-09-06); it is TY2025 now, so the ledger carries a round-trip in BOTH years —
    // distinct source refs, distinct lots, so neither year's figures move.
    let mut evs = real_events_2024();
    evs.extend(real_events_2025_for_the_slice_arm());
    let (_dir, vault) = make_vault(&evs);
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    // 1) The full packet (2024 — it HAS a Schedule D and an 8949).
    cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    let snapshot: BTreeMap<String, Vec<u8>> = std::fs::read_dir(out.path())
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    assert!(
        snapshot.len() > 1,
        "the packet wrote several files: {:?}",
        snapshot.keys().collect::<Vec<_>>()
    );

    assert!(
        snapshot.keys().any(|k| k.contains("schedule_d")),
        "the packet must contain the colliding forms, or this test proves nothing: {:?}",
        snapshot.keys().collect::<Vec<_>>()
    );

    // 2) The crypto slice for ANOTHER year, into the SAME directory — the collision scenario.
    cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    // The slice really wrote the colliding names, or step (2) proves nothing about clobbering.
    for name in ["f8949.pdf", "schedule_d.pdf"] {
        assert!(
            out.path().join(name).exists(),
            "the TY2025 slice must have written {name}, or the collision never happened"
        );
    }

    // ★ Every packet file must still be byte-for-byte what the packet wrote.
    for (name, bytes) in &snapshot {
        let now = std::fs::read(out.path().join(name))
            .unwrap_or_else(|_| panic!("the slice DELETED the packet's {name}"));
        assert_eq!(
            &now, bytes,
            "★ the slice OVERWROTE the packet's {name} — a cents form inside a whole-dollar return"
        );
    }
}

/// I-3 (T16 review r1 / D-4): `export-irs-pdf` writes the MANDATORY `basis_methodology.txt` alongside the
/// PDF packet whenever a $0-basis tranche row is filed — the disclosure must ride the flagship
/// filing-ready artifact, not only the CSV paths.
#[test]
fn export_irs_pdf_writes_basis_methodology_when_a_tranche_is_filed() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // A 2025 tranche (in `wallet()`) + a 2025 Sell of it → a $0-basis tranche row filed in 2025.
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        1_000_000,
        wallet(),
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 31),
        datetime!(2026-01-01 0:00 UTC),
    )
    .unwrap();
    let sell = vec![ev(
        "sell-t",
        datetime!(2025-06-15 12:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 1_000_000,
            usd_proceeds: dec!(500),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )];
    let mut s = Session::open(&vault, &pp()).unwrap();
    btctax_core::persistence::append_import_batch(s.conn(), &sell).unwrap();
    s.save().unwrap();
    drop(s); // release the vault lock before the export opens its own session

    let out = tempfile::tempdir().unwrap();
    cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2025,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    let disclosure = out.path().join("basis_methodology.txt");
    assert!(
        disclosure.exists(),
        "the PDF packet must write the mandatory basis_methodology.txt (I-3 / D-4)"
    );
    assert!(
        std::fs::read_to_string(&disclosure)
            .unwrap()
            .contains("Basis methodology disclosure"),
        "the disclosure content is present"
    );
}

// ── PRE-MERGE finding 3 — the filed-PDF path's three fail-closed screens ─────────────────────────
//
// ★★★ These three screens could ALL be deleted and the entire 2536-test suite stayed green. The
// export path is the one that puts INK ON PAPER: a return that refuses in `report` but exports a
// signed-ready PDF is the worst failure this codebase has, and nothing held it.
//
// ★★ It was found because `5ab1258` MOVED the §G-21 refusal into `screen_absolute` and changed its
// signature, touching this exact call site — and the fold's stated assurance was "the compiler
// enumerated every call site". The compiler enumerates a signature CHANGE. It does not enumerate a
// call that is DELETED. Only a test can do that, and there was none.
//
// Each test below asserts BOTH halves: the refusal fires, AND no bytes are written. The comment at
// admin.rs:772 says "A refusal writes NO bytes" — that is the guarantee, so that is the assertion.

/// Every file the exporter could write, so "no bytes" is checked against the directory itself rather
/// than against a hand-list that would rot.
fn wrote_nothing(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(Result::ok).count() == 0)
        .unwrap_or(true)
}

/// Build a TY2024 full-return vault, letting the caller shape the inputs and the ledger.
fn full_return_vault(
    evs: &[LedgerEvent],
    shape: impl FnOnce(&mut btctax_core::tax::return_inputs::ReturnInputs),
) -> (tempfile::TempDir, PathBuf, tempfile::TempDir) {
    use btctax_cli::return_inputs;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;

    let (dir, vault) = make_vault(evs);
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

/// ★ SCREEN 1 — `screen_inputs`. An unanswered mandatory declaration must stop the export.
///
/// Mutation-verified: deleting the `screen_inputs` block reds this. ★ Note the mechanism, because the
/// fold review measured it and an imprecise claim here would be the very thing this file exists to
/// prevent: it reds via a SECOND by-design backstop (`ReturnHeader::build`'s `HeaderError::Unanswered`,
/// packet.rs:381-390) rather than by writing bytes, so `wrote_nothing()` still holds. B1 is satisfied —
/// the test discriminates — but this screen is belt-and-braces, not the sole guard. Screens 2 and 3
/// red literally as documented, with real PDF bytes observed landing.
#[test]
fn the_export_path_refuses_on_an_input_screen_and_writes_no_bytes() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |ri| {
        // Un-answer a mandatory class-(A) declaration that `answer_all_live_declarations` had set.
        ri.header.can_be_claimed_as_dependent_taxpayer = None;
    });
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect_err("an unanswered mandatory declaration must refuse the EXPORT, not just the report");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("not computable") && msg.contains("no forms were written"),
        "the refusal must name itself and promise no bytes: {msg}"
    );
    assert!(
        wrote_nothing(out.path()),
        "★ and it must KEEP that promise — a signed-ready PDF from a return that refuses is the \
         worst outcome in this codebase"
    );
}

/// ★ SCREEN 2 — `screen_compute_dependent`, the ledger-dependent one. A non-crypto NONCASH gift
/// pushes total noncash over $500, requiring an 8283 listing property btctax holds no details for.
/// Mutation-verified: deleting the `screen_compute_dependent` block reds this.
#[test]
fn the_export_path_refuses_on_the_compute_screen_and_writes_no_bytes() {
    use btctax_core::tax::return_inputs::{CharitableClass, CharitableGift, ScheduleAInputs};
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |ri| {
        ri.schedule_a = Some(ScheduleAInputs {
            charitable: vec![CharitableGift {
                class: CharitableClass::CapGainProp30, // NON-crypto noncash — btctax has no rows for it
                amount: dec!(600),                     // over the $500 Schedule A line 12 trigger
            }],
            ..Default::default()
        });
    });
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect_err("an incomplete required Form 8283 must refuse the export");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("not computable") && msg.contains("no forms were written"),
        "{msg}"
    );
    assert!(
        wrote_nothing(out.path()),
        "no bytes on a compute-screen refusal"
    );
}

/// ★ SCREEN 3 — `screen_absolute`, which needs the COMPUTED return. This is the screen `5ab1258`
/// moved the §G-21 refusal into, and the one whose deletion the skeptic executed: with it gone the
/// exporter wrote `00_f1040.pdf` plus the SIMPLIFIED Form 8995 — precisely the wrong form once the
/// §199A(e)(2) phase-in applies.
/// Mutation-verified: deleting the `screen_absolute` block reds this.
#[test]
fn an_above_threshold_reit_only_export_files_form_8995a() {
    use btctax_core::tax::return_inputs::Owner;
    use btctax_core::tax::return_inputs::{Form1099Div, W2};
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |ri| {
        // Taxable income before QBI above the TY2024 §199A(e)(2) threshold, WITH REIT dividends ⇒
        // the Form 8995-A phase-in applies and v1 does not model it.
        ri.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(250000),
            box2_fed_withheld: dec!(50000),
            box3_ss_wages: dec!(168600),
            box5_medicare_wages: dec!(250000),
            ..Default::default()
        }];
        ri.div_1099 = vec![Form1099Div {
            box1a_ordinary: dec!(1000),
            box5_section_199a: dec!(1000),
            ..Default::default()
        }];
    });
    // ★★★ §G-28/B1a — THIS EXPORT NOW SUCCEEDS, and writes Form 8995-A. The filer's only §199A item is
    //     REIT dividends, so there is no trade or business for Parts I-III to attach to, and i8995a
    //     sends them straight to Part IV. Until B1a this refused outright.
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("a REIT/PTP-only filer above the threshold files on Form 8995-A Part IV");
    let names: Vec<String> = rep
        .full_return_paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().any(|n| n == "55A_f8995a.pdf"),
        "the packet must carry Form 8995-A: {names:?}"
    );
    // ★★ …and NOT the simplified form. The original skeptic's observation still stands, inverted: with
    //    the wrong form selected, "00_f1040.pdf + the SIMPLIFIED 55_f8995.pdf land here — the wrong
    //    form, on disk, ready to sign." Above the threshold i8995a's "Who Must File" forbids it.
    assert!(
        !names.iter().any(|n| n.contains("f8995.pdf")),
        "the SIMPLIFIED Form 8995 must not be filed above the threshold: {names:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// P2a — the Form 8949 overflow PREFLIGHT (FILING-READINESS-PLAN rank 3).
//
// Form 8949 holds 14 rows per part per page and the full-return path does not paginate. Before this
// preflight, a filer with more disposal legs than that got `error: IRS form fill: 16 rows exceed the
// 14-row capacity of a single Part II page` — exit 2, output directory never created, EVERY form in
// the packet lost — from a message naming no tax year, no remedy, and never saying this is a btctax
// limit rather than the filer's error. Meanwhile `report --tax-year` on the same vault exits 0 and
// prints every figure: the filer has the numbers and cannot get the paper.
//
// ★ The overflow is LOT-COUNT-driven, not dollar-driven. A weekly dollar-cost-averaging buyer holds
// ~52 lots and any meaningful sale draws on more than 14 of them, while a single-lot whale with a $1M
// gain emits one row — so the SMALL end of the dollar axis is the more exposed population.

/// A TY2024 ledger whose single sale draws on `lots` separate long-term lots ⇒ `lots` Form 8949
/// Part II rows (`form_8949` emits one row per disposal LEG, never aggregating). The DCA shape: many
/// small weekly buys in 2022, one sale in 2024.
fn dca_events_2024(lots: usize) -> Vec<LedgerEvent> {
    let mut evs: Vec<LedgerEvent> = (0..lots)
        .map(|i| {
            ev(
                &format!("dca-buy-{i}"),
                datetime!(2022-01-05 12:00 UTC) + time::Duration::days(i as i64 * 7),
                EventPayload::Acquire(Acquire {
                    sat: 100_000,
                    usd_cost: dec!(50),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            )
        })
        .collect();
    evs.push(ev(
        "dca-sell",
        datetime!(2024-06-15 12:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 100_000 * lots as i64,
            usd_proceeds: dec!(100) * rust_decimal::Decimal::from(lots as i64),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    ));
    evs
}

/// ★★ **This test asserted the OPPOSITE until P2b landed, and the inversion is the point.**
///
/// P2a shipped an honest overflow refusal here — correct while the full-return path could not
/// paginate. P2b then made it paginate (`fill_full_return` → `fill_8949_full_with_map`, chunking into
/// ⌈rows/grid⌉ copies with no ceiling), which turned that refusal into a FALSE one: the CLI rejected a
/// packet the filler would have produced, reintroducing the exact total loss — exit 2, zero bytes,
/// every form in the packet gone — that P2b existed to end, for the DCA population P2a itself named as
/// the most exposed.
///
/// ★★★ **B3, demonstrated.** P2a (btctax-cli) and P2b (btctax-forms) were built in PARALLEL worktrees
/// off one base. Each shipped a passing kill-test, and the two asserted contradictory things about
/// this same 16-leg filer: the forms test that it PAGINATES, this one that it REFUSES. Both suites
/// were green simultaneously, because each was scoped to one layer and neither could see the other.
/// A per-range review is not a branch review, and a green suite per lane does not compose into a
/// correct product.
///
/// B1 planted defect: restore the deleted preflight in `export_irs_pdf` and this reds at the
/// `expect()` with `CliError::Usage`.
#[test]
fn a_full_return_with_more_8949_legs_than_a_page_holds_now_files_on_multiple_copies() {
    let (_d, vault, out) = full_return_vault(&dca_events_2024(16), |_ri| {});

    // The premise: this fixture really does exceed one page's grid. Without it the test proves
    // nothing — it would pass on a 1-leg household that never reaches the pagination path at all.
    assert_eq!(
        btctax_forms::Form8949Map::ty2024().rows_per_page,
        14,
        "the TY2024 grid — the capacity 16 legs must exceed"
    );

    cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("★ 16 legs must FILE now that the full-return 8949 paginates (P2b)");

    // ★ The full-return packet writes SEQUENCE-PREFIXED names (`12A_f8949.pdf`) so a slice run and a
    // packet run into one directory cannot interleave; find it by stem rather than pinning the
    // prefix, which is the attachment sequence and not this test's subject.
    let f8949_path = std::fs::read_dir(out.path())
        .expect("out_dir exists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("_f8949.pdf"))
        })
        .expect("the packet must contain a Form 8949, not be lost to a false refusal");
    let f8949 = std::fs::read(&f8949_path).unwrap();
    assert!(f8949.starts_with(b"%PDF"));

    use btctax_forms::testonly::*;
    let doc = load(&f8949).unwrap();
    assert_eq!(
        doc.get_pages().len(),
        4,
        "16 legs ⇒ 2 copies × 2 pages — the 15th and 16th legs have somewhere to print"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// P6 — the §170(d)(1) charitable carryover-out must reach a human on the EXPORT path too
// (FILING-READINESS-PLAN rank 9). `report --tax-year` is not the only way a filer meets their
// return; `export-irs-pdf` is the one that hands them a PDF to sign, and it prints the §170 warnings
// already. A carryover the filer is never told about is a deduction they paid for and forfeit.

/// A TY2024 full-return vault whose gift OVERFLOWS its §170(b) ceiling: $50,000 of wages against a
/// $40,000 cash gift, so the 60%-of-AGI ceiling leaves an excess to carry into 2025.
fn full_return_vault_with_a_gift_over_its_ceiling(
) -> (tempfile::TempDir, PathBuf, tempfile::TempDir) {
    use btctax_core::tax::return_inputs::{
        CharitableClass, CharitableGift, Owner, ScheduleAInputs, W2,
    };
    full_return_vault(&real_events_2024(), |ri| {
        ri.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(50000),
            box2_fed_withheld: dec!(5000),
            box3_ss_wages: dec!(50000),
            box5_medicare_wages: dec!(50000),
            ..Default::default()
        }];
        ri.schedule_a = Some(ScheduleAInputs {
            charitable: vec![CharitableGift {
                class: CharitableClass::Cash60,
                amount: dec!(40000),
            }],
            ..Default::default()
        });
        // ★ P4 (phase 2) gates an itemizing return with any single gift >= $250 on the §170(f)(8)
        //   contemporaneous written acknowledgment. This fixture's subject is the CARRYOVER, not
        //   substantiation, so it answers the question rather than dodging it — a $40,000 gift with
        //   no CWA is a deduction the statute denies, and the fixture must not model an unlawful one.
        ri.charitable_cwa_obtained = Some(true);
    })
}

/// ★ P6, the report struct: the export path must CARRY the carryover out to its caller. Without this
/// the value has no reader on this path at all.
#[test]
fn the_full_return_export_report_carries_the_charitable_carryover_out() {
    let (_d, vault, out) = full_return_vault_with_a_gift_over_its_ceiling();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("the packet exports");
    assert!(
        !rep.charitable_carryover_out.is_empty(),
        "a gift over its §170(b) ceiling must carry its carryover out on the export report"
    );
    assert!(
        rep.charitable_carryover_out
            .iter()
            .all(|c| c.origin_year == 2024 && c.amount > btctax_core::Usd::ZERO),
        "…tagged with the origin year and a real amount: {:?}",
        rep.charitable_carryover_out
    );
}

/// ★ P6, the human: running the real binary, `export-irs-pdf` must NAME the carryover on stderr,
/// beside the other §170 notes. Mutation: delete the block from main.rs and this reds — which is the
/// only thing that pins the value reaches a person rather than a struct field.
#[test]
fn export_irs_pdf_tells_the_filer_about_the_charitable_carryover() {
    let (_d, vault, out) = full_return_vault_with_a_gift_over_its_ceiling();
    let bin = env!("CARGO_BIN_EXE_btctax");
    let res = std::process::Command::new(bin)
        .arg("--vault")
        .arg(&vault)
        .args(["export-irs-pdf", "--tax-year", "2024", "--out"])
        .arg(out.path().join("packet"))
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    let stderr = String::from_utf8_lossy(&res.stderr).into_owned();
    assert_eq!(res.status.code(), Some(0), "the export succeeds: {stderr}");
    assert!(
        stderr.contains("Charitable carryover to 2025"),
        "the export must name the §170(d)(1) carryover on stderr: {stderr}"
    );
    assert!(
        stderr.contains("--write-carryover"),
        "…and how to roll it forward: {stderr}"
    );
}

/// The other half of the B1 pair: 14 legs — exactly the page capacity — still EXPORTS. A preflight
/// that refuses one leg too early would be as wrong as no preflight at all (`>` vs `>=` is the
/// mutation this kills).
#[test]
fn a_full_return_with_exactly_a_full_8949_page_of_legs_still_exports() {
    let (_d, vault, out) = full_return_vault(&dca_events_2024(14), |_ri| {});

    cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("14 legs fit the page exactly and must still file");
    assert!(
        out.path().join("00_f1040.pdf").exists(),
        "the packet writes on the boundary case"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// N4 — the packet's UNSIGNALLED HAND-MARKS (FILING-READINESS-PLAN rank 14).
//
// A no-crypto packet leaves the Digital Asset question unmarked (btctax cannot swear "No" for a
// ledger it was not given) and the line-7 "If not required, check here" box blank (it cannot
// establish that Schedule D is NOT required). Both blanks are CORRECT — "an entry is testimony", and
// btctax must never answer for the filer. What was wrong is that the filer was never TOLD: no
// advisory named the digital-asset question, and manifest.txt was one line of stapling order. A
// filer signed a return with a mandatory question unanswered and no instruction anywhere in the
// product's output.
//
// ★ The fix adds the SIGNAL, never the answer. Owner decision 13 puts it in the manifest — the one
// artifact the filer is told to follow while assembling paper.

/// A TY2024 full-return vault with NO crypto activity at all: a plain wage earner. This is the L1
/// household of the plan's low-end pass, and the population for both mark 1 and mark 2.
fn no_crypto_full_return_vault() -> (tempfile::TempDir, PathBuf, tempfile::TempDir) {
    use btctax_core::tax::return_inputs::{Owner, W2};
    full_return_vault(&[], |ri| {
        ri.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(40000),
            box2_fed_withheld: dec!(3000),
            box3_ss_wages: dec!(40000),
            box5_medicare_wages: dec!(40000),
            ..Default::default()
        }];
    })
}

/// ★ N4. The no-crypto packet's manifest must ENUMERATE the marks btctax deliberately left for the
/// filer — and must not answer any of them.
///
/// Mutation: delete the `hand_marks` block from the manifest and this reds.
#[test]
fn a_no_crypto_packet_names_the_marks_the_filer_must_make_by_hand() {
    let (_d, vault, out) = no_crypto_full_return_vault();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("a plain wage earner's packet exports");
    let manifest = std::fs::read_to_string(out.path().join("manifest.txt")).unwrap();

    assert!(
        manifest.contains("COMPLETE BY HAND"),
        "the manifest must carry a hand-marks section: {manifest}"
    );
    assert!(
        manifest.contains("Digital Asset"),
        "…naming the Digital Asset question, which is mandatory and unanswered: {manifest}"
    );
    assert!(
        manifest.contains("line 7"),
        "…the line-7 \"If not required, check here\" box, blank on a no-Schedule-D packet: {manifest}"
    );
    assert!(
        manifest.contains("penalties of perjury") || manifest.contains("sign"),
        "…and the signature block, which is every filer's: {manifest}"
    );
    // The signal, never the answer.
    assert_eq!(
        rep.hand_marks.len(),
        3,
        "exactly the three marks this packet leaves: {:?}",
        rep.hand_marks
    );
}

/// The other half of the B1 pair. A packet where btctax DID answer the Digital Asset question (there
/// is crypto activity) and DID attach a Schedule D must not tell the filer to make either mark — a
/// list that always says the same thing signals nothing, and instructing a filer to hand-mark a box
/// on a correctly-filed form is worse than silence.
#[test]
fn a_crypto_packet_does_not_list_marks_btctax_already_made() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |_ri| {});
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("the crypto packet exports");
    let manifest = std::fs::read_to_string(out.path().join("manifest.txt")).unwrap();

    assert!(
        !manifest.contains("Digital Asset"),
        "btctax answered the Digital Asset question \"Yes\" — it is not the filer's to make: {manifest}"
    );
    assert!(
        !manifest.contains("line 7"),
        "a Schedule D IS attached, so the line-7 \"if not required\" box is not the filer's: {manifest}"
    );
    // The signature is always theirs, on every return.
    assert_eq!(
        rep.hand_marks.len(),
        1,
        "only the signature block remains: {:?}",
        rep.hand_marks
    );
    assert!(
        manifest.contains("COMPLETE BY HAND"),
        "…and it is still announced: {manifest}"
    );
}

/// ★★★ **P5 / §170(f)(11)(D) — THE MANIFEST MUST NAME THE APPRAISAL, END TO END.**
///
/// Over $500,000 claimed for donated property the qualified appraisal is not a record to keep, it is
/// an ATTACHMENT to the filed return. btctax cannot generate it — only an appraiser can — so the one
/// thing it owes the filer is to put it in the stapling order. A required attachment that appears in
/// no manifest is one nobody attaches, and the packet looks complete without it.
///
/// This runs the REAL `export-irs-pdf` path, not the advisory builder: a long-term BTC donation
/// reclassified through `reclassify_outflow`, a full return that itemizes and claims it, and then
/// `manifest.txt` read back off disk.
///
/// **B1 mutations, each observed RED before the fix landed:**
/// - delete the manifest block from `export_irs_pdf` ⇒ the first assertion reds while the rest of
///   the suite stays green (the advisory alone is stderr, and stderr is not the envelope);
/// - relax the §170(f)(11)(D) threshold to §170(f)(11)(C)'s $5,000 ⇒ the second, small-donation
///   assertion reds, i.e. every $5,001 donor is told to attach an appraisal to the return.
#[test]
fn the_manifest_names_the_qualified_appraisal_over_500k_and_stays_quiet_below_it() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::event::OutflowClass;
    use btctax_core::tax::return_inputs::{Owner, Person, ReturnInputs, ScheduleAInputs, W2};
    use btctax_core::tax::types::FilingStatus;

    // A 2024 long-term donation of `fmv`: buy 1 BTC in 2020, Send it in 2024, reclassify as Donate.
    // The lot is long-term, so §170(e) leaves the deduction at FMV — the claimed amount IS `fmv`.
    let donation_vault = |fmv: &str| {
        let evs = vec![
            ev(
                "buy-lt",
                datetime!(2020-01-05 12:00 UTC),
                EventPayload::Acquire(Acquire {
                    sat: 100_000_000,
                    usd_cost: dec!(10000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            ),
            ev(
                "send-donate",
                datetime!(2024-06-01 12:00 UTC),
                EventPayload::TransferOut(TransferOut {
                    sat: 100_000_000,
                    fee_sat: None,
                    dest_addr: Some("bc1qsyntheticcharity".into()),
                    txid: None,
                }),
            ),
        ];
        let (dir, vault) = make_vault(&evs);
        let out_ref = {
            let s = Session::open(&vault, &pp()).unwrap();
            let (state, _) = s.project().unwrap();
            state.pending_reconciliation[0].event.canonical()
        };
        cmd::reconcile::reclassify_outflow(
            &vault,
            &pp(),
            &out_ref,
            OutflowClass::Donate {
                appraisal_required: false,
            },
            btctax_cli::eventref::parse_usd_arg(fmv).unwrap(),
            None,
            None,
            datetime!(2026-01-01 12:00 UTC),
        )
        .unwrap();
        // A full return that ITEMIZES and claims the property deduction. AGI is large enough that
        // §170(b)'s 30% ceiling lets a real amount onto Schedule A line 12.
        {
            let mut s = Session::open(&vault, &pp()).unwrap();
            let mut ri = ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                donations_had_restrictions: Some(false),
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(ScheduleAInputs {
                    salt_real_estate: dec!(10000),
                    ..Default::default()
                }),
                w2s: vec![W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(3000000),
                    box3_ss_wages: dec!(168600),
                    box5_medicare_wages: dec!(3000000),
                    ..Default::default()
                }],
                ..Default::default()
            };
            ri.header.taxpayer = Person {
                first_name: "Pat".into(),
                last_name: "Filer".into(),
                ssn: "123456789".into(),
                ..Default::default()
            };
            btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
            return_inputs::set(s.conn(), 2024, &ri).unwrap();
            s.save().unwrap();
        }
        (dir, vault)
    };

    // ── OVER the threshold: $700,000 claimed. The manifest must say to attach the appraisal. ──
    let (_d1, big) = donation_vault("700000.00");
    let out1 = tempfile::tempdir().unwrap();
    cmd::admin::export_irs_pdf(
        &big,
        &pp(),
        out1.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("the $700,000-donation full return exports");
    let man = std::fs::read_to_string(out1.path().join("manifest.txt")).unwrap();
    assert!(
        man.contains("qualified appraisal")
            && man.contains("170(f)(11)(D)")
            && man.to_ascii_uppercase().contains("MUST SUPPLY"),
        "the manifest must name the appraisal, cite §170(f)(11)(D), and say btctax cannot make \
         it:\n{man}"
    );
    assert!(
        man.contains("$700000.00"),
        "…and carry the amount that triggered it, so the filer can check it:\n{man}"
    );
    assert!(
        man.contains("1.170A-16(f)(3)"),
        "…and say the duty recurs in every §170(d) carryover year:\n{man}"
    );

    // ── UNDER it: a $20,000 donation. Over §170(f)(11)(C)'s $5,000 (so an appraisal must be
    //    OBTAINED and a Section B 8283 files) but far under §170(f)(11)(D)'s $500,000, so nothing is
    //    ATTACHED and the manifest must not say otherwise. This is the row that separates the two
    //    thresholds — without it, wiring the gate to $5,000 would go unnoticed.
    let (_d2, small) = donation_vault("20000.00");
    let out2 = tempfile::tempdir().unwrap();
    cmd::admin::export_irs_pdf(
        &small,
        &pp(),
        out2.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect("the $20,000-donation full return exports");
    let man2 = std::fs::read_to_string(out2.path().join("manifest.txt")).unwrap();
    assert!(
        !man2.contains("qualified appraisal"),
        "a $20,000 donation owes no ATTACHED appraisal — §170(f)(11)(D) is 'more than \
         $500,000':\n{man2}"
    );
}

/// spec 1099-DA (build review I-2): the crypto-slice arm REFUSES on a LIVE year through the real
/// command — a TY2026 vault with one exchange disposition and no stored return inputs — before any
/// byte: the out directory does not exist afterwards, and the message names the exit.
fn live_2026_events() -> Vec<LedgerEvent> {
    vec![
        ev(
            "buy-2026",
            datetime!(2026-02-01 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(900),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "sell-2026",
            datetime!(2026-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 1_000_000,
                usd_proceeds: dec!(1200),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
}

#[test]
fn a_live_year_refuses_the_crypto_slice_before_any_byte() {
    let (_dir, vault) = make_vault(&live_2026_events());
    let out = tempfile::tempdir().unwrap();
    let out_dir = out.path().join("slice-2026");
    let err =
        cmd::admin::export_irs_pdf(&vault, &pp(), &out_dir, 2026, &[], None, Default::default())
            .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Form 1099-DA answers") && msg.contains("income import"),
        "the slice refusal names the exit: {msg}"
    );
    assert!(!out_dir.exists(), "a refusal writes NO bytes");
}

// ── Form 1040-V, the payment voucher (spec SPEC_form_4868_1040v.md R4, task T4) ─────────────────
//
// The voucher's whole point is that it rides in the envelope WITHOUT being part of the stapled
// return: *"Do not staple or attach this voucher to your payment or return."* Every assertion below
// is about keeping those two facts apart — the file is written BESIDE the packet, and the manifest
// names it in a block BELOW the stapling list.

/// A TY2024 full-return vault that OWES: $250,000 of wages with NO federal withholding, so Form 1040
/// line 37 is large and positive.
fn owing_vault() -> (tempfile::TempDir, PathBuf, tempfile::TempDir) {
    use btctax_core::tax::return_inputs::{Owner, W2};
    full_return_vault(&real_events_2024(), |ri| {
        ri.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(250000),
            box2_fed_withheld: dec!(0),
            box3_ss_wages: dec!(168600),
            box4_ss_withheld: dec!(10453.20),
            box5_medicare_wages: dec!(250000),
            ..Default::default()
        }];
    })
}

/// The stapling list is every `{seq}  {file}` / `  ATT  ` line the manifest prints before any
/// free-form block. Derived from the file rather than a hand-list, so a new packet member is covered.
fn stapling_list_lines(manifest: &str) -> Vec<&str> {
    manifest
        .lines()
        .take_while(|l| !l.contains("ENCLOSE LOOSE"))
        .collect()
}

/// ★★ THE END-TO-END KAT — `--pay-by-check` on a return that owes writes `f1040v.pdf` BESIDE the
/// packet, box 3 equals Form 1040 line 37 read back from the written voucher, and the manifest gains
/// its own block below the stapling list.
#[test]
fn pay_by_check_writes_a_voucher_beside_the_packet_with_line_37_in_box_3() {
    let (_d, vault, out) = owing_vault();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: None,
        },
    )
    .expect("a return that owes, paying by check, gets its voucher");

    let path = rep
        .form_1040v_path
        .clone()
        .expect("the voucher path rides out on the report");
    assert_eq!(path, out.path().join("f1040v.pdf"));
    assert!(path.exists());

    // ★ BESIDE the packet, never IN it: `full_return_paths` is the stapling order.
    assert!(
        !rep.full_return_paths.contains(&path),
        "the voucher must not be listed among the stapled forms"
    );
    assert!(
        !rep.full_return_paths
            .iter()
            .any(|p| p.file_name().is_some_and(|n| n == "f1040v.pdf")),
        "…under any prefix either"
    );

    // ★ Box 3 = Form 1040 line 37, read back OUT OF THE WRITTEN VOUCHER — not from a struct.
    let doc = btctax_forms::testonly::load(&std::fs::read(&path).unwrap()).unwrap();
    let fields = btctax_forms::testonly::collect_fields(&doc).unwrap();
    let map = btctax_forms::testonly::Form1040VMap::ty2024();
    let box3 = fields
        .iter()
        .find(|f| f.fqn == map.box3_amount)
        .and_then(|f| btctax_forms::testonly::text_value(&doc, f.id))
        .expect("box 3 carries a value");
    // The same figure the packet's own 1040 prints on line 37.
    let f1040 = std::fs::read(out.path().join("00_f1040.pdf")).unwrap();
    let f1040_doc = btctax_forms::testonly::load(&f1040).unwrap();
    let f1040_fields = btctax_forms::testonly::collect_fields(&f1040_doc).unwrap();
    let f1040_map = btctax_forms::testonly::Form1040Map::ty2024();
    let line37 = f1040_fields
        .iter()
        .find(|f| {
            match f1040_map
                .line37
                .as_ref()
                .expect("TY2024's 1040 map binds line 37")
            {
                btctax_forms::testonly::MoneyCell::Single(s) => &f.fqn == s,
                btctax_forms::testonly::MoneyCell::Pair(p) => f.fqn == p.dollars_field,
            }
        })
        .and_then(|f| btctax_forms::testonly::text_value(&f1040_doc, f.id))
        .expect("the packet's 1040 prints line 37");
    assert_eq!(
        box3, line37,
        "★ the voucher and the return it accompanies must agree about what is owed"
    );

    // ★ The manifest block: present, verbatim, and SEPARATE from the stapling list.
    let manifest = std::fs::read_to_string(out.path().join("manifest.txt")).unwrap();
    assert!(
        manifest.contains(btctax_cli::cmd::admin::ENCLOSE_LOOSE_LINE),
        "the manifest must carry the enclose-loose line:\n{manifest}"
    );
    let stapled = stapling_list_lines(&manifest);
    assert!(
        !stapled.iter().any(|l| l.contains("f1040v")),
        "★ the voucher must NOT appear in the stapling list — the manifest is what a filer follows \
         when assembling the envelope, and this is the one page the form says not to staple:\n{manifest}"
    );
    assert!(
        stapled.iter().any(|l| l.contains("00_f1040.pdf")),
        "premise: the stapling list is non-empty and really is the list:\n{manifest}"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the voucher carries the filer's SSN");
    }
}

/// ★ KILL — without `--pay-by-check` a return that owes gets a NOTE, not a file. Most filers who owe
/// pay online, and Direct Pay / EFTPS need no voucher — but silence about a balance due is the one
/// answer a tax tool may not give.
#[test]
fn a_return_that_owes_without_the_flag_gets_the_note_and_no_voucher() {
    let (_d, vault, out) = owing_vault();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .unwrap();

    assert!(rep.form_1040v_path.is_none(), "no flag ⇒ no voucher");
    assert!(
        !out.path().join("f1040v.pdf").exists(),
        "…and no file on disk"
    );
    let note = rep.form_1040v_note.expect("a balance due is never silent");
    assert!(
        note.contains("Direct Pay / EFTPS need no voucher; pass --pay-by-check for Form 1040-V"),
        "the note names the alternative and the flag: {note}"
    );
    assert!(
        !note.contains("IGNORED"),
        "no --pay was given, so there is nothing to report as discarded: {note}"
    );

    // ★ `--pay` WITHOUT `--pay-by-check` is discarded — and the note says so. Not a refusal (that
    // would cost the filer every form over one inapplicable flag), but not silent either: they typed
    // an amount they meant to pay, and nothing else on the run would tell them it went nowhere.
    let out2 = tempfile::tempdir().unwrap();
    let rep2 = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out2.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: false,
            pay: Some(dec!(500)),
        },
    )
    .unwrap();
    assert!(rep2.form_1040v_path.is_none());
    let note2 = rep2
        .form_1040v_note
        .expect("a discarded --pay is never silent");
    assert!(
        note2.contains("--pay $500 was IGNORED"),
        "the note must name the flag it dropped: {note2}"
    );
    let manifest = std::fs::read_to_string(out.path().join("manifest.txt")).unwrap();
    assert!(
        !manifest.contains("ENCLOSE LOOSE"),
        "and the manifest gains no block for a page that was not written"
    );
}

/// ★ KILL — `--pay-by-check` on a return that owes NOTHING writes no voucher and says why. A voucher
/// for $0 is not a payment.
#[test]
fn pay_by_check_on_a_refund_return_is_a_note_not_a_voucher() {
    let (_d, vault, out) = full_return_vault(&real_events_2024(), |_| {});
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: None,
        },
    )
    .unwrap();
    assert!(rep.form_1040v_path.is_none());
    assert!(!out.path().join("f1040v.pdf").exists());
    let note = rep
        .form_1040v_note
        .expect("asking for a voucher on a refund return deserves an answer");
    assert!(
        note.contains("owes nothing"),
        "the note says why there is none: {note}"
    );
}

/// ★ KILL — `--pay` above line 37, or negative, or with cents, is REFUSED.
///
/// The ceiling is the deliberate asymmetry with `btctax extension --pay`, which has none: the
/// voucher pays a COMPUTED balance, so more than it is a slip; Form 4868 line 7 pays against an
/// ESTIMATE, which a filer may overshoot on purpose to limit interest. The refusal says so.
#[test]
fn a_partial_payment_above_line_37_or_negative_or_fractional_is_refused() {
    let (_d, vault, out) = owing_vault();
    let call = |pay: btctax_core::Usd| {
        cmd::admin::export_irs_pdf(
            &vault,
            &pp(),
            out.path(),
            2024,
            &[],
            None,
            btctax_cli::cmd::admin::VoucherChoice {
                pay_by_check: true,
                pay: Some(pay),
            },
        )
    };

    let err = call(dec!(99_999_999)).expect_err("above line 37 must refuse");
    let msg = err.to_string();
    assert!(
        msg.contains("more than the") && msg.contains("btctax extension --pay"),
        "the refusal names the ceiling AND the flag that has none: {msg}"
    );

    assert!(call(dec!(-1))
        .expect_err("a negative payment is not a payment")
        .to_string()
        .contains("--pay must be >= 0"));
    assert!(call(dec!(100.25))
        .expect_err("cents disagree with a whole-dollar return")
        .to_string()
        .contains("WHOLE DOLLARS"));

    // …and a PARTIAL payment below line 37 is allowed, with a note about the interest that runs.
    let rep = call(dec!(1000)).expect("a partial payment is a legitimate choice");
    assert!(rep.form_1040v_path.is_some());
    let note = rep
        .form_1040v_note
        .expect("a partial payment is never silent");
    assert!(
        note.contains("PARTIAL payment of $1000") && note.contains("interest"),
        "the note names what is left unpaid: {note}"
    );
}

/// ★ KILL (I-7) — `--pay-by-check` on a crypto-slice year REFUSES, naming the reason. That year
/// computes no Form 1040 at all, so there is no line 37 for box 3 to carry, and a voucher whose
/// amount btctax invented would tell the Service the filer is paying a figure no return supports.
#[test]
fn pay_by_check_on_a_crypto_slice_year_refuses_naming_the_reason() {
    let (_dir, vault) = make_vault(&real_events_2024());
    let out = tempfile::tempdir().unwrap();
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: None,
        },
    )
    .expect_err("there is no line 37 on a crypto-slice year");
    let msg = err.to_string();
    assert!(
        msg.contains("there is no Form 1040 line 37 for 2024")
            && msg.contains("Form 1040-V accompanies a full return")
            && msg.contains("income import"),
        "the refusal names the reason and the exit: {msg}"
    );
    assert!(
        wrote_nothing(out.path()),
        "★ and it refuses BEFORE any byte — the slice is not written either"
    );
}

/// ★★★ KILL (seam review I-1) — **every `--pay` refusal is PRE-BYTE: `--out` is untouched.**
///
/// The refusals themselves were already tested; what was NOT tested is *when* they fire. They ran
/// inside `write_payment_voucher`, i.e. after every packet PDF was on disk and before `manifest.txt`
/// was written — so a refused `--pay 100.25` left the return's forms in `--out` with no manifest
/// beside them. `btctax extension` decides "is this the return's envelope directory?" by the
/// presence of `manifest.txt` ALONE, so it read that directory as empty ground and wrote `f4868.pdf`
/// into it: the one outcome that guard exists to prevent, reached by an ordinary filer typo.
///
/// A FRESH `--out` per refusal, because `wrote_nothing` cannot distinguish "wrote nothing" from
/// "wrote nothing new". The four cases are the whole `--pay` match; the second half of the test
/// proves the same directory still accepts a legitimate run, so a command that refused everything
/// could not pass.
#[test]
fn every_pay_refusal_is_pre_byte_and_leaves_the_out_directory_untouched() {
    let (_d, vault, _out) = owing_vault();
    let refuse = |pay: btctax_core::Usd, needle: &str| {
        let dir = tempfile::tempdir().unwrap();
        let err = cmd::admin::export_irs_pdf(
            &vault,
            &pp(),
            dir.path(),
            2024,
            &[],
            None,
            btctax_cli::cmd::admin::VoucherChoice {
                pay_by_check: true,
                pay: Some(pay),
            },
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
        assert!(
            matches!(err, btctax_cli::CliError::Usage(_)),
            "a flag error is a USAGE error, not a FormFill one: {err:?}"
        );
        // ★ THE POINT: not merely "no manifest" but NOTHING — the directory is as the filer left it.
        assert!(
            wrote_nothing(dir.path()),
            "★ a refused --pay must leave --out untouched; found {:?}",
            std::fs::read_dir(dir.path())
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.file_name())
                .collect::<Vec<_>>()
        );
    };

    refuse(dec!(-1), "--pay must be >= 0");
    refuse(dec!(100.25), "WHOLE DOLLARS");
    refuse(dec!(99_999_999), "is more than the");
    // ★ M-2: `--pay 0 --pay-by-check` on a return that OWES. It used to reach `fill_form_1040v` and
    //   surface that filler's own refusal — "a return that owes nothing needs no Form 1040-V", a
    //   sentence describing the wrong state, in a class that does not read as a flag error.
    refuse(dec!(0), "--pay $0 writes no voucher");

    // …and the identical call with a legitimate amount WRITES, so the test cannot be passed by a
    // command that refuses everything.
    let ok_dir = tempfile::tempdir().unwrap();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        ok_dir.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: Some(dec!(1000)),
        },
    )
    .expect("a whole-dollar partial payment below line 37 is legitimate");
    assert!(rep.form_1040v_path.is_some());
    assert!(ok_dir.path().join("manifest.txt").exists());
}

/// ★★★ KILL (seam review I-1, the CONSEQUENCE) — after a refused `--pay`, `btctax extension` writing
/// into that same directory must not be walking into a half-written packet.
///
/// This is the defect stated as the filer sees it rather than as a directory listing: the refusal
/// and the extension guard are in different commands, and the seam between them is what broke. With
/// the refusal pre-byte the directory is genuinely empty, so the extension is written on clean
/// ground — and the assertion that makes it a kill is that NO packet form is in there beside it.
#[test]
fn a_refused_pay_does_not_leave_a_packet_the_extension_guard_reads_as_empty_ground() {
    let (_d, vault, out) = owing_vault();
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: Some(dec!(100.25)),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("WHOLE DOLLARS"), "{err}");

    let rep = cmd::admin::extension(
        &vault,
        &pp(),
        out.path(),
        2024,
        None,
        false,
        None,
        time::macros::datetime!(2026-02-01 12:00 UTC),
    )
    .expect("the directory is clean, so the extension is legitimately written here");
    assert!(rep.path.exists());

    let left: Vec<String> = std::fs::read_dir(out.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        left,
        vec!["f4868.pdf".to_string()],
        "★ the extension application is ALONE — a return form beside it is the defect"
    );
}

/// ★★★ KILL (seam review I-2) — the MIRROR of `extension`'s refusal (4): a `--out` that already holds
/// `f4868.pdf` refuses the return packet, before any byte, on BOTH pipelines.
///
/// The guard was one-directional and the unguarded direction is the one that happens FIRST in time:
/// `extension` in April, `export-irs-pdf` in October, one `--out` for the year. The packet was
/// written AROUND the extension application with no refusal, no warning, and a manifest that never
/// mentions it — so a filer collating the directory by filename would attach the one page whose own
/// page 2 says *"Don't attach a copy of Form 4868 to your return."*
#[test]
fn a_directory_already_holding_an_f4868_refuses_the_return_packet_on_both_pipelines() {
    let seed = |dir: &std::path::Path| {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("f4868.pdf"),
            b"%PDF-1.7
",
        )
        .unwrap();
    };
    let check = |err: btctax_cli::CliError, dir: &std::path::Path| {
        let msg = err.to_string();
        assert!(
            msg.contains("f4868.pdf") && msg.contains("Don't attach a copy of Form 4868"),
            "the refusal names what it saw and quotes the form: {msg}"
        );
        let left: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            left,
            vec!["f4868.pdf".to_string()],
            "★ no packet file appeared — the extension application is untouched"
        );
    };

    // (a) the FULL-RETURN pipeline.
    let (_d, vault, out) = owing_vault();
    seed(out.path());
    let err = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: true,
            pay: None,
        },
    )
    .expect_err("the extension's directory is not where the return goes");
    check(err, out.path());

    // (b) the CRYPTO-SLICE pipeline — same guard, same message, above the dispatch so it cannot
    //     drift between the two.
    let (_dir2, vault2) = make_vault(&real_events_2024());
    let out2 = tempfile::tempdir().unwrap();
    seed(out2.path());
    let err = cmd::admin::export_irs_pdf(
        &vault2,
        &pp(),
        out2.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .expect_err("the slice may not be written on top of an extension either");
    check(err, out2.path());
}

/// ★ KILL (seam review M-1) — `--pay` WITHOUT `--pay-by-check` is noted on the CRYPTO-SLICE arm too.
///
/// The full-return path already said so ("--pay $N was IGNORED"), but that clause is built inside
/// `write_payment_voucher`, which the slice arm never reaches — so on a crypto-slice year the amount
/// the filer typed vanished with no message at all. Same filer slip, same class, same answer: a
/// note, never a refusal.
#[test]
fn a_pay_without_the_flag_is_noted_on_the_crypto_slice_arm_too() {
    let (_dir, vault) = make_vault(&real_events_2024());
    let out = tempfile::tempdir().unwrap();
    let rep = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out.path(),
        2024,
        &[],
        None,
        btctax_cli::cmd::admin::VoucherChoice {
            pay_by_check: false,
            pay: Some(dec!(500)),
        },
    )
    .expect("an inapplicable flag may not cost the filer every form");
    assert!(
        rep.full_return_manifest.is_none(),
        "premise: this is the CRYPTO-SLICE arm"
    );
    assert!(rep.form_1040v_path.is_none(), "no voucher on a slice year");
    let note = rep
        .form_1040v_note
        .expect("a discarded --pay is never silent, on either pipeline");
    assert!(
        note.contains("--pay $500 was IGNORED") && note.contains("no full-return inputs"),
        "the note names the flag it dropped and why: {note}"
    );

    // ★★ …and it REACHES THE FILER. The note was printed inside the full-return arm of `main.rs`,
    //    which the slice dispatch never enters — so a note computed here and read by nobody would
    //    satisfy every assertion above while changing nothing on the filer's screen.
    let out_cli = tempfile::tempdir().unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_btctax"))
        .args([
            "--vault",
            vault.to_str().unwrap(),
            "export-irs-pdf",
            "--out",
            out_cli.path().to_str().unwrap(),
            "--tax-year",
            "2024",
            "--pay",
            "500",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("the btctax binary must execute");
    let stderr =
        String::from_utf8_lossy(&run.stdout).into_owned() + &String::from_utf8_lossy(&run.stderr);
    assert!(
        run.status.success(),
        "the export itself still succeeds: {stderr}"
    );
    assert!(
        stderr.contains("--pay $500 was IGNORED"),
        "★ the note must reach the filer's screen on the slice arm, not just the report struct: \
         {stderr}"
    );

    // …and a run with no --pay at all is SILENT here: the note is about the flag, not about the year.
    let out2 = tempfile::tempdir().unwrap();
    let quiet = cmd::admin::export_irs_pdf(
        &vault,
        &pp(),
        out2.path(),
        2024,
        &[],
        None,
        Default::default(),
    )
    .unwrap();
    assert!(
        quiet.form_1040v_note.is_none(),
        "no --pay ⇒ nothing to report as discarded: {:?}",
        quiet.form_1040v_note
    );
}

/// ★ KILL — a pseudo-reconciled voucher is attestation-gated and DRAFT-watermarked, exactly like the
/// packet it rides with (spec C-2). A page with a CHEQUE attached may not be the exception.
#[test]
fn a_pseudo_voucher_is_gated_and_watermarked_like_the_packet() {
    use btctax_core::tax::return_inputs::{Owner, ReturnInputs, W2};
    use btctax_core::tax::types::FilingStatus;

    let (_d2, pseudo) = make_vault(&pseudo_events());
    cmd::reconcile::pseudo_set_mode(&pseudo, &pp(), true).unwrap();
    {
        let mut s = Session::open(&pseudo, &pp()).unwrap();
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
        ri.w2s = vec![W2 {
            owner: Owner::Taxpayer,
            employer: "ACME".into(),
            box1_wages: dec!(250000),
            box2_fed_withheld: dec!(0),
            box3_ss_wages: dec!(168600),
            box4_ss_withheld: dec!(10453.20),
            box5_medicare_wages: dec!(250000),
            ..Default::default()
        }];
        btctax_cli::return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }
    let out = tempfile::tempdir().unwrap();
    let voucher = btctax_cli::cmd::admin::VoucherChoice {
        pay_by_check: true,
        pay: None,
    };

    let err = cmd::admin::export_irs_pdf(&pseudo, &pp(), out.path(), 2024, &[], None, voucher)
        .expect_err("a fictional draft may not print a payment voucher unattested");
    assert!(err.to_string().contains(ATTEST_PHRASE), "{err}");
    assert!(
        !out.path().join("f1040v.pdf").exists(),
        "a refused attestation writes no voucher"
    );

    let out2 = tempfile::tempdir().unwrap();
    let rep = cmd::admin::export_irs_pdf(
        &pseudo,
        &pp(),
        out2.path(),
        2024,
        &[],
        Some(ATTEST_PHRASE),
        voucher,
    )
    .expect("with the phrase the DRAFT packet + voucher are written ON PURPOSE");
    let path = rep.form_1040v_path.expect("the voucher is written");
    assert!(
        contains(&std::fs::read(&path).unwrap(), b"NOT FOR FILING"),
        "★ the VOUCHER page carries the DRAFT watermark, not just the packet"
    );
}

/// ★★ KILL (structural, spec R4) — **no code path hands either new stem to `FiledPacket::stapled`.**
///
/// `stapled` takes any `Vec<NamedForm>`, so this is not held by the type system — the spec says so
/// plainly ("held by test — it is not structural"). The population is the SOURCE: every file that
/// can reach the packet constructor is scanned for a mention of the two stems beside it.
#[test]
fn no_code_path_pushes_the_4868_or_the_voucher_into_the_stapled_packet() {
    let packet_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../btctax-forms/src/packet.rs"
    ))
    .expect("the packet source is readable from the test");

    // `fill_full_return` is the ONLY producer of the `Vec<NamedForm>` that reaches `stapled`, and it
    // destructures `PrintedForms` with no `..` — so a form can only get in there by being pushed by
    // name inside this file. Neither stem is.
    //
    // ★ (seam review M-5) WHAT THIS LEG CANNOT SEE, stated so a future reader does not mistake it
    //   for the whole gate: it is a STRING-LITERAL grep over everything after the `fn` header (not
    //   the function body), so a push built from `Stem::F1040v.file_stem()` would evade it, and a
    //   future `#[cfg(test)]` case in `packet.rs` naming either stem would red it spuriously. The
    //   guarantee is really held by
    //   `pay_by_check_writes_a_voucher_beside_the_packet_with_line_37_in_box_3`, which reads the
    //   MANIFEST btctax actually wrote and asserts the voucher is absent from the stapling list —
    //   an outcome check that no spelling of the push can slip past. This leg is the cheap early
    //   warning beside it.
    let body = packet_src
        .split("pub fn fill_full_return")
        .nth(1)
        .expect("fill_full_return is in packet.rs");
    for stem in ["f4868", "f1040v"] {
        assert!(
            !body.contains(&format!("\"{stem}\"")),
            "★ {stem} must never be pushed into the packet: the manifest's stapling order is what a \
             filer follows, and both forms say in their own words not to attach them"
        );
    }
    // …and the packet's own sequence table gives each of them `None`, by an explicit arm.
    assert_eq!(btctax_forms::attachment_sequence("f4868", 2024), None);
    assert_eq!(btctax_forms::attachment_sequence("f1040v", 2024), None);
    assert_eq!(btctax_forms::attachment_sequence("f4868", 2025), None);
    assert_eq!(btctax_forms::attachment_sequence("f1040v", 2025), None);
}
