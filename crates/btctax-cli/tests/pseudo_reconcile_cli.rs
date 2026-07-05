//! Pseudo-reconcile mode (sub-project 2) — CLI-level KATs.
//!
//! Load-bearing guards:
//!  - [★] the `[PSEUDO]` marker appears on the ON-SCREEN render (report) — including the C1 basis-taint
//!    case (a REAL Sell on a pseudo `$0`-basis lot) — and is PROVABLY ABSENT from every export CSV / form
//!    file (the headline guard: a dedicated bool the writers OMIT, never a `BasisSource` variant);
//!  - [R0-I3] `export-snapshot` REFUSES while any synthetic contributes;
//!  - synthetics are NEVER persisted: after projecting in pseudo mode, `load_all` shows no new events.
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

/// [R0-I3] `export-snapshot` REFUSES while pseudo synthetics contribute; OFF ⇒ it proceeds.
#[test]
fn export_snapshot_refused_while_pseudo_active() {
    let (_dir, vault) = make_vault(&taint_events());
    let out = tempfile::tempdir().unwrap();

    // Pseudo ON ⇒ the projection has synthetics ⇒ export is refused, nothing written.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let err = cmd::admin::export_snapshot(&vault, &pp(), out.path(), Some(2025)).unwrap_err();
    assert!(
        matches!(err, btctax_cli::CliError::PseudoActiveExport(n) if n > 0),
        "expected PseudoActiveExport, got {err:?}"
    );
    assert!(
        !out.path().join("snapshot.sqlite").exists(),
        "a refused export must leave the out_dir untouched"
    );

    // Pseudo OFF ⇒ the unknown-basis inbound is a Hard blocker again, but export itself is not refused
    // by the pseudo guard (it proceeds to write the snapshot).
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), false).unwrap();
    let ok = cmd::admin::export_snapshot(&vault, &pp(), out.path(), Some(2025));
    assert!(
        ok.is_ok(),
        "mode-off export must not be refused by the pseudo guard: {ok:?}"
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
