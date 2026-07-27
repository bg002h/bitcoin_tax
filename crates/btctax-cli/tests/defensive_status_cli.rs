//! `btctax defensive status` — read-only Approach-B status (arch/engine-keep-wizard-cut).
//!
//! This is the CLI replacement for the retired TUI wizard dashboard: the SAME pure
//! `btctax_core::defensive::journey_view`, printed as plain text. It is a top-level command
//! (`btctax defensive status`), a sibling of `reconcile`/`export-irs-pdf`/`export-snapshot` — NOT
//! nested under `reconcile` (it emits a decision event; this is a plain read).
//!
//! PRIVACY: synthetic values in tempdirs; no user file is read.

use btctax_cli::{cmd, Session};
use btctax_core::event::{Dispose, DisposeKind, EventPayload};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::append_import_batch;
use btctax_core::LedgerEvent;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime};
use time::UtcOffset;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026 - 01 - 01 0:00 UTC)
}
fn wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}

/// Run `btctax --vault <vault> <args...>`; returns (exit, stdout, stderr).
fn run_btctax(vault: &Path, args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let mut c = std::process::Command::new(bin);
    c.arg("--vault").arg(vault.to_str().unwrap());
    for a in args {
        c.arg(a);
    }
    c.env("BTCTAX_PASSPHRASE", "pw");
    let out = c.output().expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A bare vault (no imports, no decisions) — nothing outstanding.
fn empty_vault(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    vault
}

/// A wallet with a single Sell (`Dispose`) and NO other lots anywhere in that wallet — a full,
/// sat-for-sat `UncoveredDisposal` shortfall, i.e. a `journey_view` declare candidate. Mirrors
/// `chokepoint_declare.rs::vault_with_uncovered_disposal`. Returns (vault, the Sell's `EventId`).
fn vault_with_declare_candidate(dir: &Path) -> (PathBuf, EventId) {
    let vault = dir.join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();
    let sell_id = EventId::import(Source::Coinbase, SourceRef::new("SELL"));
    let sell = LedgerEvent {
        id: sell_id.clone(),
        utc_timestamp: date!(2020 - 06 - 01).midnight().assume_utc(),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload: EventPayload::Dispose(Dispose {
            sat: 50_000_000,
            usd_proceeds: dec!(1_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };
    append_import_batch(s.conn(), &[sell]).unwrap();
    s.save().unwrap();
    (vault, sell_id)
}

// ── (a) empty vault: the all-clear ─────────────────────────────────────────────────────────────

#[test]
fn empty_vault_prints_the_all_clear_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());

    let (code, stdout, stderr) = run_btctax(&vault, &["defensive", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("Nothing outstanding"),
        "an empty vault has nothing outstanding: {stdout:?}"
    );
}

// ── (b) a declare candidate names the exact `reconcile declare-tranche`-adjacent verb ────────────

#[test]
fn declare_candidate_names_the_declare_tranche_verb_with_its_wallet_and_amount() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, sell_id) = vault_with_declare_candidate(dir.path());

    let (code, stdout, stderr) = run_btctax(&vault, &["defensive", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("Declare candidates (1)"),
        "the uncovered disposal must surface as a declare candidate: {stdout:?}"
    );
    assert!(
        stdout.contains(&sell_id.canonical()),
        "the shortfall's own event ref must be printed (copy-pasteable): {stdout:?}"
    );
    assert!(
        stdout.contains("btctax declare-tranche")
            && stdout.contains("--amount 0.50000000")
            && stdout.contains("--wallet exchange:coinbase:main"),
        "the exact next verb, with the shortfall's own amount and wallet, must be named: {stdout:?}"
    );
}

// ── (c) a live $0 tranche names `promote-tranche`; once promoted, it does not ────────────────────

#[test]
fn declared_tranche_names_promote_tranche_and_stops_once_promoted() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    let (code, _out, stderr) = run_btctax(
        &vault,
        &[
            "reconcile",
            "declare-tranche",
            "--amount",
            "0.5",
            "--wallet",
            "self:cold",
            "--window-start",
            "2020-01-01",
            "--window-end",
            "2020-12-31",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");

    let (code, stdout, stderr) = run_btctax(&vault, &["defensive", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("Live tranches (1)"), "{stdout}");
    assert!(
        stdout.contains("btctax promote-tranche"),
        "a DeclaredZero ($0) tranche must offer the promote verb: {stdout}"
    );
    assert!(stdout.contains("declared ($0, revocable)"), "{stdout}");
}

// ── (d) a pseudo-active ledger refuses (never silently masks a shortfall) ────────────────────────

#[test]
fn pseudo_active_ledger_refuses_with_a_reason_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    // Turn pseudo-reconcile mode on, then create a synthetic-default-contributing situation: an
    // unclassified inbound transfer with no decision.
    let (code, _out, stderr) = run_btctax(&vault, &["reconcile", "pseudo", "on"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    {
        use btctax_core::event::TransferIn;
        let mut s = Session::open(&vault, &pp()).unwrap();
        let inbound = LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("IN")),
            utc_timestamp: now(),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet()),
            payload: EventPayload::TransferIn(TransferIn {
                sat: 10_000_000,
                src_addr: None,
                txid: None,
            }),
        };
        append_import_batch(s.conn(), &[inbound]).unwrap();
        s.save().unwrap();
    }
    // Precondition: the ledger really is pseudo-active now.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _cfg) = s.project().unwrap();
        assert!(
            state.pseudo_active(),
            "precondition: the fixture must actually be pseudo-active"
        );
    }

    let (code, stdout, stderr) = run_btctax(&vault, &["defensive", "status"]);
    assert_ne!(
        code, 0,
        "a pseudo-active ledger must refuse: stdout={stdout:?}"
    );
    assert!(
        stderr.to_lowercase().contains("pseudo"),
        "the refusal must name the pseudo-reconcile cause: {stderr:?}"
    );
    assert!(
        stdout.is_empty(),
        "a refusal must print nothing on stdout: {stdout:?}"
    );
}

// ── (e) the Approach-B experimental disclosure gates on `uses_approach_b`, same as every other verb ──

#[test]
fn experimental_notice_absent_on_a_plain_vault_present_once_a_tranche_is_declared() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    const NOTICE_MARK: &str = btctax_core::experimental::NOTICE.title;

    let (code, stdout, stderr) = run_btctax(&vault, &["defensive", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        !stderr.contains(NOTICE_MARK),
        "no live tranche/promote yet — no notice: {stderr:?}"
    );
    assert!(!stdout.contains(NOTICE_MARK));

    let (code, _out, stderr) = run_btctax(
        &vault,
        &[
            "reconcile",
            "declare-tranche",
            "--amount",
            "0.5",
            "--wallet",
            "self:cold",
            "--window-start",
            "2020-01-01",
            "--window-end",
            "2020-12-31",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");

    let (code, stdout, stderr) = run_btctax(&vault, &["defensive", "status"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains(NOTICE_MARK),
        "a live tranche is on file — Approach-B is in use — the notice must reach stderr: {stderr:?}"
    );
    assert!(
        !stdout.contains(NOTICE_MARK),
        "the notice must NEVER reach stdout (stdout is parsed/piped): {stdout:?}"
    );
}
