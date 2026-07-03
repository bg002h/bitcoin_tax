//! self-transfer-passthrough C3 (CLI): the `reconcile match-self-transfers` two-phase dispatch. Drives
//! the real `btctax` binary (`std::process::Command`) to verify: Phase 1 renders the proposed pairs
//! (read-only, writes nothing) + honors `--dry-run`; Phase 2 confirms ONE pair — DROP appends a
//! `SelfTransferPassthrough` (both legs Skip), RELOCATE routes to the existing `link_transfer` (dest
//! holds the coins). NEVER automatic.

use btctax_cli::Session;
use btctax_core::event::{Acquire, BasisSource, TransferIn, TransferOut};
use btctax_core::persistence::{append_import_batch, load_all};
use btctax_core::{EventId, EventPayload, LedgerEvent, Source, SourceRef, WalletId};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::datetime;
use time::UtcOffset;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn wallet_a() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}
fn wallet_b() -> WalletId {
    WalletId::Exchange {
        provider: "river".into(),
        account: "main".into(),
    }
}
fn id_of(r: &str) -> EventId {
    EventId::import(Source::Coinbase, SourceRef::new(r))
}

/// Vault with: an Acquire in A (covers the relocate), a same-wallet DROP pair (in-d/out-d), and a
/// cross-wallet RELOCATE pair (out-r from A / in-r to B).
fn build_vault(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();
    let ti = |sat| {
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        })
    };
    let to = |sat| {
        EventPayload::TransferOut(TransferOut {
            sat,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        })
    };
    let ev = |r: &str, ts, wallet: WalletId, payload| LedgerEvent {
        id: id_of(r),
        utc_timestamp: ts,
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet),
        payload,
    };
    let batch = vec![
        ev(
            "acq",
            datetime!(2025-02-01 12:00:00 UTC),
            wallet_a(),
            EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(50),
                fee_usd: dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        ),
        ev(
            "in-d",
            datetime!(2025-03-01 12:00:00 UTC),
            wallet_a(),
            ti(100_000),
        ),
        ev(
            "out-d",
            datetime!(2025-03-02 12:00:00 UTC),
            wallet_a(),
            to(100_000),
        ),
        ev(
            "out-r",
            datetime!(2025-04-01 12:00:00 UTC),
            wallet_a(),
            to(100_000),
        ),
        ev(
            "in-r",
            datetime!(2025-04-02 12:00:00 UTC),
            wallet_b(),
            ti(100_000),
        ),
    ];
    append_import_batch(s.conn(), &batch).unwrap();
    s.save().unwrap();
    vault
}

/// Run `btctax --vault <vault> reconcile match-self-transfers <args...>`; returns (exit, stdout, stderr).
fn run(vault: &Path, args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let out = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().unwrap())
        .arg("reconcile")
        .arg("match-self-transfers")
        .args(args)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn event_count(vault: &Path) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    load_all(s.conn()).unwrap().len()
}

/// Phase 1: the bare command RENDERS both proposals (DROP + RELOCATE) and writes NOTHING; `--dry-run`
/// is identical. Two-phase preview, never automatic.
#[test]
fn match_self_transfers_preview_renders_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_vault(dir.path());
    let before = event_count(&vault);

    let (code, stdout, stderr) = run(&vault, &[]);
    assert_eq!(code, 0, "preview exits 0; stderr: {stderr}");
    assert!(
        stdout.contains("DROP"),
        "renders the DROP proposal: {stdout}"
    );
    assert!(
        stdout.contains("RELOCATE"),
        "renders the RELOCATE proposal: {stdout}"
    );
    assert_eq!(event_count(&vault), before, "preview writes nothing");

    let (code2, _stdout2, _) = run(&vault, &["--dry-run"]);
    assert_eq!(code2, 0);
    assert_eq!(event_count(&vault), before, "--dry-run writes nothing");
}

/// Phase 2 DROP: `--in <in-d> --out <out-d>` appends ONE SelfTransferPassthrough; both legs Skip.
#[test]
fn match_self_transfers_confirm_drop_skips_both_legs() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_vault(dir.path());

    let (code, stdout, stderr) = run(
        &vault,
        &[
            "--in",
            &id_of("in-d").canonical(),
            "--out",
            &id_of("out-d").canonical(),
        ],
    );
    assert_eq!(code, 0, "confirm DROP exits 0; stderr: {stderr}");
    assert!(
        stdout.contains("dropped self-transfer passthrough"),
        "{stdout}"
    );

    let s = Session::open(&vault, &pp()).unwrap();
    let events = load_all(s.conn()).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::SelfTransferPassthrough(_)))
            .count(),
        1,
        "exactly one SelfTransferPassthrough appended"
    );
    let (state, _) = s.project().unwrap();
    // in-d skipped (no lot from it); out-d skipped (not pending). Only the RELOCATE pair's out-r stays
    // pending, and A's acquire lot is intact.
    assert!(
        !state
            .pending_reconciliation
            .iter()
            .any(|pt| pt.event == id_of("out-d")),
        "out-d is skipped, not pending"
    );
}

/// Phase 2 RELOCATE: `--in <in-r> --out <out-r>` routes to the EXISTING link_transfer (appends a
/// TransferLink), landing the coins in the destination wallet B with carried basis.
#[test]
fn match_self_transfers_confirm_relocate_lands_coins_in_dest() {
    let dir = tempfile::tempdir().unwrap();
    let vault = build_vault(dir.path());

    // Drop the same-wallet pair first so A's acquire lot is free for the relocate (out-d no longer
    // consumes it).
    run(
        &vault,
        &[
            "--in",
            &id_of("in-d").canonical(),
            "--out",
            &id_of("out-d").canonical(),
        ],
    );

    let (code, stdout, stderr) = run(
        &vault,
        &[
            "--in",
            &id_of("in-r").canonical(),
            "--out",
            &id_of("out-r").canonical(),
        ],
    );
    assert_eq!(code, 0, "confirm RELOCATE exits 0; stderr: {stderr}");
    assert!(stdout.contains("relocated self-transfer"), "{stdout}");

    let s = Session::open(&vault, &pp()).unwrap();
    let events = load_all(s.conn()).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::TransferLink(_)))
            .count(),
        1,
        "RELOCATE routes to link_transfer → exactly one TransferLink"
    );
    let (state, _) = s.project().unwrap();
    assert_eq!(
        state.holdings_by_wallet.get(&wallet_b()).copied(),
        Some(100_000),
        "the destination wallet B holds the relocated coins"
    );
    assert_eq!(
        state
            .holdings_by_wallet
            .get(&wallet_a())
            .copied()
            .unwrap_or(0),
        0,
        "the source wallet A holds 0 after relocation"
    );
    // Non-taxable: the relocation recognizes no disposal.
    assert!(
        state.disposals.is_empty(),
        "a self-transfer relocation is non-taxable"
    );
    let leg_basis: btctax_core::Usd = state
        .lots
        .iter()
        .filter(|l| l.wallet == wallet_b())
        .map(|l| l.usd_basis)
        .sum();
    assert_eq!(leg_basis, dec!(50), "carried basis (not $0, not FMV)");
}
