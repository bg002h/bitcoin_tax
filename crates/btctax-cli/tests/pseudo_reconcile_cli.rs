//! Pseudo-reconcile mode (sub-project 2) — CLI-level KATs.
//!
//! Load-bearing guards:
//!  - [★] the `[PSEUDO]` marker appears on the ON-SCREEN render (report) — including the C1 basis-taint
//!    case (a REAL Sell on a pseudo `$0`-basis lot) — and is PROVABLY ABSENT from every export CSV / form
//!    file (the headline guard: a dedicated bool the writers OMIT, never a `BasisSource` variant);
//!  - [R0-I3] `export-snapshot` REFUSES while any synthetic contributes;
//!  - synthetics are NEVER persisted: after projecting in pseudo mode, `load_all` shows no new events.
mod fixtures;
use btctax_cli::{cmd, render, Session};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::persistence::load_all;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use std::path::PathBuf;
use time::macros::{datetime, offset};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn w() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w()),
        payload: p,
    }
}
fn prices() -> StaticPrices {
    let mut m = BTreeMap::new();
    m.insert(time::macros::date!(2025 - 03 - 01), dec!(100000));
    m.insert(time::macros::date!(2025 - 06 - 01), dec!(100000));
    StaticPrices(m)
}
fn cfg_on() -> ProjectionConfig {
    ProjectionConfig {
        pseudo_reconcile: true,
        ..ProjectionConfig::default()
    }
}

/// The C1 basis-taint fixture: an unknown-basis inbound (→ pseudo `$0` self-transfer lot) consumed by a
/// REAL Sell (→ flagged disposal leg) + the held-lot remainder.
fn taint_events() -> Vec<LedgerEvent> {
    vec![
        imp(
            "in-1",
            datetime!(2025-03-01 12:00 UTC),
            EventPayload::TransferIn(TransferIn {
                sat: 1_000_000,
                src_addr: None,
                txid: None,
            }),
        ),
        imp(
            "sell-1",
            datetime!(2025-06-01 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 400_000, // partial sell → a held-lot remainder ALSO stays pseudo
                usd_proceeds: dec!(500),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ]
}

/// [★] The headline guard: `[PSEUDO]` is on the ON-SCREEN render (held lot + the C1 basis-taint disposal
/// leg) AND is PROVABLY ABSENT from every export CSV / form file.
#[test]
fn pseudo_marker_on_screen_but_absent_from_every_export_file() {
    let evs = taint_events();
    let st = project(&evs, &prices(), &cfg_on());

    // (a) ON-SCREEN: the report carries [PSEUDO] — on the held-lot remainder AND the C1 disposal leg.
    let screen = render::render_report(&st, None);
    assert!(
        screen.contains("[PSEUDO]"),
        "on-screen report MUST flag pseudo rows:\n{screen}"
    );
    // The disposal leg (a REAL Sell on a pseudo $0-basis lot) is itself flagged (C1 basis taint).
    assert!(
        st.disposals[0].legs[0].pseudo,
        "the real Sell on a pseudo $0-basis lot must be flagged (C1)"
    );

    // (b) OUTPUT: write every CSV / form to a temp dir, then grep them all — assert NO pseudo/synthetic
    // marker leaked. This tests the WRITERS directly (bypassing the I3 command-level refusal), because
    // sub-3 will replace the refusal with a typed-attest gate that lets attested exports through — even
    // then the marker must never appear in a file.
    let dir = tempfile::tempdir().unwrap();
    let empty_details: BTreeMap<EventId, btctax_core::DonationDetails> = BTreeMap::new();
    render::write_csv_exports(dir.path(), &st, Some(2025), None, &empty_details).unwrap();

    let mut checked = 0usize;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("csv") {
            continue;
        }
        let body = std::fs::read_to_string(&path).unwrap().to_lowercase();
        assert!(
            !body.contains("pseudo") && !body.contains("synthetic"),
            "export file {:?} LEAKED a pseudo/synthetic marker:\n{body}",
            path.file_name().unwrap()
        );
        checked += 1;
    }
    assert!(
        checked >= 4,
        "expected the lots/disposals/removals/income CSVs to be written"
    );
}

/// Init a vault + append the given events directly (bypassing CSV import), returning `(tempdir, vault)`.
fn make_vault(evs: &[LedgerEvent]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    btctax_core::persistence::append_import_batch(s.conn(), evs).unwrap();
    s.save().unwrap();
    (dir, vault)
}

/// [sub-3, ex-I3] The attestation gate SUPERSEDES sub-2's interim blanket refusal: while pseudo-active,
/// a MISSING attestation refuses the export (nothing written); a CORRECT attestation PERMITS it; turning
/// the mode OFF exports with no attestation at all.
#[test]
fn attest_gate_supersedes_interim_i3_refusal() {
    let (_dir, vault) = make_vault(&taint_events());
    let out = tempfile::tempdir().unwrap();

    // Pseudo ON + no attestation ⇒ refused (AttestationRequired), nothing written.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let err = cmd::admin::export_snapshot(&vault, &pp(), out.path(), Some(2025), None).unwrap_err();
    assert!(
        matches!(err, btctax_cli::CliError::AttestationRequired),
        "expected AttestationRequired (supersedes the interim refusal), got {err:?}"
    );
    assert!(
        !out.path().join("snapshot.sqlite").exists(),
        "a refused export must leave the out_dir untouched"
    );

    // Pseudo ON + the CORRECT attestation ⇒ PERMITTED (this is the whole point of sub-3): the draft is
    // exported ON PURPOSE.
    let ok = cmd::admin::export_snapshot(
        &vault,
        &pp(),
        out.path(),
        Some(2025),
        Some(btctax_cli::ATTEST_PHRASE),
    );
    assert!(
        ok.is_ok(),
        "a correct attestation must PERMIT the pseudo-active export: {ok:?}"
    );

    // Pseudo OFF ⇒ the unknown-basis inbound is a Hard blocker again, but export itself is not gated
    // (it proceeds to write the snapshot) even with NO attestation.
    let out2 = tempfile::tempdir().unwrap();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), false).unwrap();
    let ok = cmd::admin::export_snapshot(&vault, &pp(), out2.path(), Some(2025), None);
    assert!(
        ok.is_ok(),
        "mode-off export must not be gated by the attestation guard: {ok:?}"
    );
}

/// The pure `require_attestation` exact-compare helper [R0-I2]: correct (trimmed) ⇒ Ok;
/// wrong ⇒ AttestationFailed; missing ⇒ AttestationRequired. Exact, trimmed, case-SENSITIVE.
/// (★ fault-inject target — break the exact-compare and this goes RED.)
#[test]
fn require_attestation_is_exact_trimmed_case_sensitive() {
    use btctax_cli::{require_attestation, CliError, ATTEST_PHRASE};
    // Correct — exact.
    assert!(require_attestation(Some(ATTEST_PHRASE)).is_ok());
    // Correct — surrounding whitespace is TRIMMED before comparison.
    assert!(require_attestation(Some("  I attest this is true  ")).is_ok());
    assert!(require_attestation(Some("\tI attest this is true\n")).is_ok());
    // Wrong — case-sensitive (lower-case i) ⇒ FAILED, not required.
    assert!(matches!(
        require_attestation(Some("i attest this is true")),
        Err(CliError::AttestationFailed)
    ));
    // Wrong — trailing junk that survives trim ⇒ FAILED.
    assert!(matches!(
        require_attestation(Some("I attest this is true!!")),
        Err(CliError::AttestationFailed)
    ));
    // Wrong — interior text (a contains()-style bug would let this through) ⇒ FAILED.
    assert!(matches!(
        require_attestation(Some("I do not attest this is true")),
        Err(CliError::AttestationFailed)
    ));
    // Missing ⇒ Required (distinct from Failed) [R0-I1].
    assert!(matches!(
        require_attestation(None),
        Err(CliError::AttestationRequired)
    ));
    // Empty string is a supplied-but-wrong phrase ⇒ FAILED, not Required.
    assert!(matches!(
        require_attestation(Some("")),
        Err(CliError::AttestationFailed)
    ));
}

/// While pseudo-active, the CORRECT attestation writes the files (the draft exports ON PURPOSE).
#[test]
fn export_pseudo_active_correct_attest_writes_files() {
    let (_dir, vault) = make_vault(&taint_events());
    let out = tempfile::tempdir().unwrap();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let sqlite = cmd::admin::export_snapshot(
        &vault,
        &pp(),
        out.path(),
        Some(2025),
        Some(btctax_cli::ATTEST_PHRASE),
    )
    .expect("a correct attestation must permit the export")
    .path;

    assert!(sqlite.exists(), "the snapshot.sqlite must be written");
    // The all-years + year-scoped form CSVs must all be present.
    for name in [
        "lots.csv",
        "disposals.csv",
        "form8949.csv",
        "schedule_d.csv",
    ] {
        assert!(
            out.path().join(name).exists(),
            "{name} must be written after a correct attestation"
        );
    }
}

/// While pseudo-active, a MISSING attestation (`None`) ⇒ AttestationRequired and out_dir untouched.
#[test]
fn export_pseudo_active_missing_attest_refused_out_dir_untouched() {
    let (_dir, vault) = make_vault(&taint_events());
    let out = tempfile::tempdir().unwrap();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let err = cmd::admin::export_snapshot(&vault, &pp(), out.path(), Some(2025), None).unwrap_err();
    assert!(
        matches!(err, btctax_cli::CliError::AttestationRequired),
        "missing attestation ⇒ AttestationRequired, got {err:?}"
    );
    // NOTHING written — checked FIRST, before any bytes.
    assert!(!out.path().join("snapshot.sqlite").exists());
    assert!(!out.path().join("form8949.csv").exists());
    let count = std::fs::read_dir(out.path()).unwrap().count();
    assert_eq!(count, 0, "a refused export must leave out_dir empty");
}

/// While pseudo-active, a WRONG phrase ⇒ AttestationFailed and out_dir untouched — exact/trimmed/
/// case-sensitive at the command boundary. (★ fault-inject target — break the exact-compare ⇒ RED.)
#[test]
fn export_pseudo_active_wrong_phrase_refused() {
    let (_dir, vault) = make_vault(&taint_events());
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    for wrong in [
        "i attest this is true",   // wrong case
        "I attest this is true!!", // trailing junk (survives trim)
        "I attest this is  true",  // interior whitespace differs
        "attest",                  // substring — a contains() bug would pass this
    ] {
        let out = tempfile::tempdir().unwrap();
        let err = cmd::admin::export_snapshot(&vault, &pp(), out.path(), Some(2025), Some(wrong))
            .unwrap_err();
        assert!(
            matches!(err, btctax_cli::CliError::AttestationFailed),
            "wrong phrase {wrong:?} ⇒ AttestationFailed, got {err:?}"
        );
        assert!(
            !out.path().join("snapshot.sqlite").exists(),
            "a refused export ({wrong:?}) must leave the out_dir untouched"
        );
        assert_eq!(
            std::fs::read_dir(out.path()).unwrap().count(),
            0,
            "out_dir must be empty after a refused export ({wrong:?})"
        );
    }
}

/// A fully-real (NOT pseudo-active) ledger exports with NO `--attest` — and even a bogus attest is
/// simply IGNORED [R0-N1]. Same file SET each way (bytes differ — sqlite embeds timestamps).
#[test]
fn export_not_pseudo_active_ignores_attest() {
    // Mode never turned on ⇒ no synthetics ⇒ pseudo_active() is false, even though the unknown-basis
    // inbound is a Hard blocker (export itself is not gated by blockers).
    let (_dir, vault) = make_vault(&taint_events());

    // No attestation → exports fine.
    let out_a = tempfile::tempdir().unwrap();
    cmd::admin::export_snapshot(&vault, &pp(), out_a.path(), Some(2025), None)
        .expect("a fully-real ledger must export with no attestation");
    assert!(out_a.path().join("snapshot.sqlite").exists());
    assert!(out_a.path().join("form8949.csv").exists());

    // A bogus attestation is IGNORED (not validated) when not pseudo-active → still exports.
    let out_b = tempfile::tempdir().unwrap();
    cmd::admin::export_snapshot(&vault, &pp(), out_b.path(), Some(2025), Some("nonsense"))
        .expect("attest is ignored when not pseudo-active");
    assert!(out_b.path().join("snapshot.sqlite").exists());
    assert!(out_b.path().join("form8949.csv").exists());
}

/// [R0-M1] The attestation error strings are BUILT from `ATTEST_PHRASE` (no drift): both variants name
/// the exact phrase AND the pseudo-reconciled state.
#[test]
fn attest_strings_contain_phrase() {
    use btctax_cli::{CliError, ATTEST_PHRASE};
    let required = CliError::AttestationRequired.to_string();
    let failed = CliError::AttestationFailed.to_string();
    assert!(
        required.contains(ATTEST_PHRASE),
        "AttestationRequired must name the exact phrase: {required}"
    );
    assert!(
        failed.contains(ATTEST_PHRASE),
        "AttestationFailed must name the exact phrase: {failed}"
    );
    assert!(
        required.to_lowercase().contains("pseudo"),
        "AttestationRequired must name the pseudo-reconciled state: {required}"
    );
    assert!(
        failed.to_lowercase().contains("pseudo"),
        "AttestationFailed must name the pseudo-reconciled state: {failed}"
    );
}

/// The `PseudoReconcileActive` advisory blocker renders in `verify` (automatically, via `{:?}`), and it
/// is ADVISORY (does not add a Hard blocker / change the verify exit).
#[test]
fn verify_shows_pseudo_reconcile_active_advisory() {
    let (_dir, vault) = make_vault(&taint_events());
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    let text = render::render_verify(&report);
    assert!(
        text.contains("PseudoReconcileActive"),
        "verify must surface the PseudoReconcileActive advisory:\n{text}"
    );
}

use btctax_cli::cmd::reconcile::PseudoApproveFilter;
use btctax_core::PseudoKind;

fn unclassified(rf: &str, ts: time::OffsetDateTime) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::Unclassified(Unclassified {
            raw: "weird".into(),
        }),
    )
}
fn now_ts() -> time::OffsetDateTime {
    datetime!(2026-01-15 00:00 UTC)
}

/// [T5] `approve` materializes pseudo defaults as REAL (attested) decisions via the own-loop: after
/// approval the ledger has a NEW real decision, the default is no longer `[PSEUDO]`, and it SURVIVES
/// turning the mode off (it is real now — the whole point).
#[test]
fn approve_materializes_real_decisions_that_survive_mode_off() {
    let inbound = vec![imp(
        "in-1",
        datetime!(2025-03-01 12:00 UTC),
        EventPayload::TransferIn(TransferIn {
            sat: 1_000_000,
            src_addr: None,
            txid: None,
        }),
    )];
    let (_dir, vault) = make_vault(&inbound);
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        load_all(s.conn()).unwrap().len()
    };
    let n = cmd::reconcile::apply_bulk_pseudo_approve(
        &vault,
        &pp(),
        PseudoApproveFilter::default(),
        now_ts(),
    )
    .unwrap();
    assert_eq!(n, 1, "the single unknown-basis inbound default is approved");

    // A NEW real decision was persisted.
    let after = {
        let s = Session::open(&vault, &pp()).unwrap();
        load_all(s.conn()).unwrap().len()
    };
    assert_eq!(
        after,
        before + 1,
        "approve persists exactly one real decision"
    );

    // Re-project (mode still on): the default is now governed by the REAL decision ⇒ NOT pseudo anymore.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (st, _cfg) = s.project().unwrap();
        assert_eq!(st.lots.len(), 1);
        assert!(
            !st.lots[0].pseudo,
            "an approved default is real ⇒ no longer [PSEUDO]"
        );
        assert_eq!(st.pseudo_synthetic_count, 0, "no synthetic remains for it");
    }

    // Turn the mode OFF: the approved (real) decision REMAINS — the inbound is a real $0 self-transfer lot,
    // NOT an UnknownBasisInbound blocker.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), false).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let (st, _cfg) = s.project().unwrap();
    assert!(
        !st.blockers
            .iter()
            .any(|b| b.kind == btctax_core::BlockerKind::UnknownBasisInbound),
        "the approved real decision persists after the mode is turned off"
    );
    assert_eq!(st.lots.len(), 1);
    assert!(!st.lots[0].pseudo);
}

/// [T5] `approve --kind self-transfer` promotes ONLY the self-transfer defaults; the unclassified-row
/// default stays pending (still `[PSEUDO]`). Deterministic own-loop filter.
#[test]
fn approve_filter_by_kind_promotes_only_that_type() {
    let evs = vec![
        imp(
            "in-1",
            datetime!(2025-03-01 12:00 UTC),
            EventPayload::TransferIn(TransferIn {
                sat: 1_000_000,
                src_addr: None,
                txid: None,
            }),
        ),
        unclassified("u-1", datetime!(2025-03-01 13:00 UTC)),
    ];
    let (_dir, vault) = make_vault(&evs);
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    // Two synthetic defaults present (one self-transfer, one raw).
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (st, _cfg) = s.project().unwrap();
        assert_eq!(st.pseudo_synthetic_count, 2);
    }
    // Approve ONLY the self-transfer kind.
    let n = cmd::reconcile::apply_bulk_pseudo_approve(
        &vault,
        &pp(),
        PseudoApproveFilter {
            kind: Some(PseudoKind::SelfTransferInbound),
            ..Default::default()
        },
        now_ts(),
    )
    .unwrap();
    assert_eq!(n, 1, "only the self-transfer default is approved");

    // The raw (unclassified) default is STILL a pending synthetic.
    let s = Session::open(&vault, &pp()).unwrap();
    let (st, _cfg) = s.project().unwrap();
    assert_eq!(
        st.pseudo_synthetic_count, 1,
        "the unclassified-row default is still pending (not approved)"
    );
}

/// [T5] Revert is TOTAL: turning the mode off with NO approvals reverts the projection to real-only
/// (the Hard blocker returns; no lot; 0 synthetics) — and NO fictional event was ever written.
#[test]
fn revert_is_total_when_nothing_approved() {
    let inbound = vec![imp(
        "in-1",
        datetime!(2025-03-01 12:00 UTC),
        EventPayload::TransferIn(TransferIn {
            sat: 1_000_000,
            src_addr: None,
            txid: None,
        }),
    )];
    let (_dir, vault) = make_vault(&inbound);
    let n_events = {
        let s = Session::open(&vault, &pp()).unwrap();
        load_all(s.conn()).unwrap().len()
    };

    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), false).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (st, _cfg) = s.project().unwrap();
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == btctax_core::BlockerKind::UnknownBasisInbound));
    assert!(st.lots.is_empty());
    assert_eq!(st.pseudo_synthetic_count, 0);
    // Not one fictional event was written across on→off.
    assert_eq!(load_all(s.conn()).unwrap().len(), n_events);
}

/// Synthetics are NEVER persisted by projection: after projecting in pseudo mode, `load_all` shows the
/// SAME events (only `reconcile pseudo approve` writes). The event count is unchanged.
#[test]
fn pseudo_projection_persists_no_events() {
    let (_dir, vault) = make_vault(&taint_events());
    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        load_all(s.conn()).unwrap().len()
    };
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    // Project repeatedly via the session (each call synthesizes in-memory defaults).
    for _ in 0..3 {
        let s = Session::open(&vault, &pp()).unwrap();
        let (_state, _cfg) = s.project().unwrap();
    }
    let after = {
        let s = Session::open(&vault, &pp()).unwrap();
        load_all(s.conn()).unwrap().len()
    };
    assert_eq!(
        before, after,
        "projection in pseudo mode must NEVER append events — only `approve` writes"
    );
}

// ── #41 Part B: pseudo income-FMV default — CLI-level KATs ────────────────────────────────────────

fn now() -> time::OffsetDateTime {
    datetime!(2026-02-01 12:00 UTC)
}

/// A native `Income{Missing}` on a PRICED date renders its recognized daily-close FMV FLAGGED `[PSEUDO]`
/// on the ON-SCREEN report (the primary tax-output guard surface) — and the value is the daily close.
#[test]
fn pseudo_income_fmv_flagged_on_render() {
    let evs = vec![imp(
        "inc-1",
        datetime!(2025-03-01 13:00 UTC),
        EventPayload::Income(Income {
            sat: 500_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Mining,
            business: false,
        }),
    )];
    let st = project(&evs, &prices(), &cfg_on());
    assert_eq!(st.income_recognized.len(), 1);
    assert!(st.income_recognized[0].pseudo);
    let screen = render::render_report(&st, None);
    // The income line (`… sat = <fmv> USD …`) carries the synthesized value AND the [PSEUDO] flag.
    let income_line = screen
        .lines()
        .find(|l| l.contains("sat =") && l.contains("USD"))
        .unwrap_or_else(|| panic!("income line missing:\n{screen}"));
    assert!(
        income_line.contains("500"),
        "income line shows the synthesized daily-close FMV:\n{income_line}"
    );
    assert!(
        income_line.contains("[PSEUDO]"),
        "the pseudo income FMV line MUST be flagged:\n{income_line}"
    );
}

/// [M5 gate] A committed batch of 27 native `Income{Missing}` events (bundled-covered dates) is Hard
/// `FmvMissing` under real data ALONE; turning pseudo mode ON clears ALL 27 (recognized at the daily
/// close, each flagged pseudo) — the "27 clear under pseudo" contract Part B delivers.
#[test]
fn income_fmv_27_clear_under_pseudo() {
    let (_dir, vault) = make_vault(&fixtures::income_fmv_missing_batch(27));

    // Baseline (A alone): 27 FmvMissing, none recognized.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert_eq!(
            state
                .blockers
                .iter()
                .filter(|b| b.kind == btctax_core::BlockerKind::FmvMissing)
                .count(),
            27
        );
        assert_eq!(state.income_recognized.len(), 0);
    }

    // Pseudo ON → all 27 clear.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(
        state
            .blockers
            .iter()
            .filter(|b| b.kind == btctax_core::BlockerKind::FmvMissing)
            .count(),
        0,
        "pseudo cleared every income FmvMissing (all dates are bundled-covered)"
    );
    assert_eq!(
        state.income_recognized.len(),
        27,
        "all 27 recognized at the daily close"
    );
    assert!(
        state.income_recognized.iter().all(|r| r.pseudo),
        "every synthesized income row is flagged pseudo"
    );
}

/// While pseudo-active from a synthesized income FMV, `export-snapshot` is GATED (a missing attestation
/// refuses) — the estimate never silently reaches an export.
#[test]
fn pseudo_income_fmv_export_gated() {
    let (_dir, vault) = make_vault(&fixtures::income_fmv_missing_batch(3));
    let out = tempfile::tempdir().unwrap();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let err = cmd::admin::export_snapshot(&vault, &pp(), out.path(), Some(2025), None).unwrap_err();
    assert!(
        matches!(err, btctax_cli::CliError::AttestationRequired),
        "a pseudo-active income estimate must gate the export, got {err:?}"
    );
    assert!(
        !out.path().join("snapshot.sqlite").exists(),
        "a refused export writes nothing"
    );
}

/// `reconcile pseudo approve --kind fmv` promotes the income-FMV default to a REAL `ManualFmv` decision;
/// the next projection resolves it via the real path (recognized, NO longer `[PSEUDO]`).
#[test]
fn approve_promotes_pseudo_fmv_to_manualfmv() {
    let (_dir, vault) = make_vault(&fixtures::income_fmv_missing_batch(1));
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let filter = cmd::reconcile::PseudoApproveFilter {
        kind: Some(btctax_core::PseudoKind::PseudoFmv),
        ..Default::default()
    };
    let n = cmd::reconcile::apply_bulk_pseudo_approve(&vault, &pp(), filter, now()).unwrap();
    assert_eq!(n, 1, "the single income-FMV default is promoted");

    let s = Session::open(&vault, &pp()).unwrap();
    let has_manual_fmv = load_all(s.conn())
        .unwrap()
        .iter()
        .any(|e| matches!(e.payload, EventPayload::ManualFmv(_)));
    assert!(has_manual_fmv, "a real ManualFmv decision was persisted");

    // Re-project (pseudo still ON in config): the income resolves via the REAL ManualFmv → recognized,
    // and NO synthetic is injected for it (no longer pseudo).
    let (state, _) = s.project().unwrap();
    assert_eq!(state.income_recognized.len(), 1);
    assert!(
        !state.income_recognized[0].pseudo,
        "an approved (now real) income FMV is no longer flagged pseudo"
    );
}
