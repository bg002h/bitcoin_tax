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
use time::macros::{datetime, offset};

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

#[test]
fn real_ledger_fills_clean_official_pdfs() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();

    let report = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[], None)
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

#[test]
fn pseudo_fill_requires_attestation() {
    let (_dir, vault) = make_vault(&pseudo_events());
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let out = tempfile::tempdir().unwrap();

    // No attestation ⇒ refused, nothing written.
    let err = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[], None).unwrap_err();
    assert!(
        matches!(err, CliError::AttestationRequired),
        "pseudo-active export without attestation must be refused, got {err:?}"
    );
    assert!(
        !out.path().join("f8949.pdf").exists(),
        "a refused export writes no PDF"
    );

    // Wrong phrase ⇒ failed.
    let err =
        cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[], Some("nope")).unwrap_err();
    assert!(matches!(err, CliError::AttestationFailed), "got {err:?}");

    // Correct phrase ⇒ permitted AND watermarked.
    let report =
        cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[], Some(ATTEST_PHRASE))
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
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();

    let report = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[], None).unwrap();
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
    let report =
        cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[FormArg::F8949], None)
            .unwrap();
    assert!(report.f8949_path.is_some());
    assert!(report.schedule_d_path.is_none(), "Schedule D not selected");
    assert!(report.form_1040_path.is_none(), "1040 not selected");
    assert!(out.path().join("f8949.pdf").exists());
    assert!(!out.path().join("schedule_d.pdf").exists());
}

#[test]
fn unsupported_year_is_refused() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();
    let err = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2024, &[], None).unwrap_err();
    assert!(
        matches!(
            err,
            CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(2024))
        ),
        "SP1 bundles TY2025 only, got {err:?}"
    );
}
