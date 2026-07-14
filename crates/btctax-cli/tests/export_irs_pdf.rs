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
        false,
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
    // This build bundles TY2017 + TY2024 + TY2025; 2023 is refused.
    let err = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2023, &[], None).unwrap_err();
    assert!(
        matches!(
            err,
            CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(2023))
        ),
        "only 2017/2024/2025 are bundled, got {err:?}"
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
    let report = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2024, &[], None)
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
}

/// A REAL short-term round-trip in 2017: buy 0.01 BTC @ $200, sell it @ $500 (gain $300).
fn real_events_2017() -> Vec<LedgerEvent> {
    vec![
        ev(
            "buy-1",
            datetime!(2017-01-05 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(200),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "sell-1",
            datetime!(2017-06-15 12:00 UTC),
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
fn ty2017_real_ledger_fills_box_c_f_and_line13_no_da() {
    // ★ End-to-end SP3b: a 2017 export fills the OFFICIAL 2017 PDFs — clean, XFA dropped, Box C checked
    // (NOT Box I), the 1040 capital gain on LINE 13 (dollars f1-_51 + cents f1_52), and NO Digital-Asset
    // question anywhere.
    let (_dir, vault) = make_vault(&real_events_2017());
    let out = tempfile::tempdir().unwrap();
    let report = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2017, &[], None)
        .expect("2017 real-ledger export must succeed");
    assert!(!report.watermarked);

    use btctax_forms::testonly::*;
    // Form 8949: Box C (short-term) = c1_1[2] on /3; XFA dropped.
    let f8949 = std::fs::read(out.path().join("f8949.pdf")).unwrap();
    assert!(f8949.starts_with(b"%PDF"));
    let doc = load(&f8949).unwrap();
    assert!(!pdf_has_xfa(&doc).unwrap(), "XFA must be dropped");
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[2]"].id).as_deref(),
        Some("3"),
        "Box C checked for short-term BTC on the 2017 form"
    );

    // Form 1040: capital gain on LINE 13 (dollars f1-_51 = 300, cents f1_52 = 00); NO DA question.
    let f1040 = std::fs::read(out.path().join("form_1040_capgains.pdf")).unwrap();
    let doc = load(&f1040).unwrap();
    assert!(!pdf_has_xfa(&doc).unwrap());
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        text_value(&doc, idx["topmostSubform[0].Page1[0].f1-_51[0]"].id).as_deref(),
        Some("300"),
        "1040 line 13 dollars = Schedule D line 16"
    );
    assert_eq!(
        text_value(&doc, idx["topmostSubform[0].Page1[0].f1_52[0]"].id).as_deref(),
        Some("00"),
        "1040 line 13 cents"
    );
    // No Digital-Asset {/1,/2} pair is ANSWERED on the 2017 1040 (the form has no such question).
    assert!(report.form_1040_filled_7a, "line 13 filled (active gain)");
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    let rep = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2024, &[], None)
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
}

/// ★ THE DISPATCH, direction 2 — a year with NO full-return inputs still gets the crypto slice,
/// unchanged. Deleting the P5-C1 refusal downgraded a type-level impossibility to a branch, so the
/// branch is pinned in BOTH directions.
#[test]
fn export_without_return_inputs_still_gets_the_crypto_slice() {
    let (_dir, vault) = make_vault(&real_events());
    let out = tempfile::tempdir().unwrap();

    let rep = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2025, &[], None)
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
            &ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(), // no header ⇒ no SSN
                ..Default::default()
            },
        )
        .unwrap();
        s.save().unwrap();
    }

    let err = cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2024, &[], None)
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
    let (_dir, vault) = make_vault(&real_events_2024());
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
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    // 1) The full packet (2024 — it HAS a Schedule D and an 8949).
    cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2024, &[], None).unwrap();
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
    cmd::admin::export_irs_pdf(&vault, &pp(), out.path(), 2017, &[], None).unwrap();

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
