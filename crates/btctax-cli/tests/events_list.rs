//! UX-P4-11 `btctax events list` — ref discoverability for the `reconcile` verbs.
//!
//! Asserts: (1) the decidable universe + per-event decision status (undecided → decidable; a live
//! decision → `decision|N`); (2) the UX-P4-11 trap is closed — a ref shown by `events list`, pasted
//! verbatim, is ACCEPTED by a reconcile verb; (3) a pseudo-defaulted event lists as DECIDABLE (its
//! synthetic default is never persisted, so it must not read as decided).
//!
//! PRIVACY: synthetic Coinbase fixtures in tempdirs; no user file is read.
use btctax_cli::{cmd, eventref, Session};
use btctax_core::{EventPayload, InboundClass, TransferTarget};
use btctax_store::Passphrase;
use std::path::Path;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    time::macros::datetime!(2026 - 01 - 01 0:00 UTC)
}

const HEADER: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n";

/// Vault: a Buy (Acquire — NOT decidable), a Send (TransferOut — decidable), and a Receive
/// (TransferIn — decidable).
fn vault_buy_send_recv(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let csv = dir.join("cb.csv");
    std::fs::write(
        &csv,
        format!(
            "{HEADER}\
cb-buy,2025-01-15 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-send,2025-06-20 12:00:00 UTC,Send,BTC,0.03000000,USD,105000.00,,,,,,bc1qdest\r\n\
cb-recv,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    vault
}

/// (1) The decidable universe + decision status, and stable refs across a decision.
#[test]
fn events_list_reports_decidable_events_and_decision_status() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_buy_send_recv(dir.path());

    let rows = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let kinds: Vec<&str> = rows.iter().map(|r| r.kind).collect();
    assert!(
        kinds.contains(&"transfer-in") && kinds.contains(&"transfer-out"),
        "the Send + Receive must be listed as decidable; got kinds {kinds:?}"
    );
    assert!(
        !kinds.contains(&"acquire"),
        "a Buy (Acquire) is fully determined — NOT decidable; got kinds {kinds:?}"
    );
    assert!(
        rows.iter().all(|r| r.decision_ref.is_none()),
        "nothing is decided yet: {:?}",
        rows.iter().map(|r| &r.decision_ref).collect::<Vec<_>>()
    );

    // Decide the inbound (self-transfer). Its ref must be the one `events list` showed.
    let in_ref = rows
        .iter()
        .find(|r| r.kind == "transfer-in")
        .unwrap()
        .reff
        .clone();
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        },
        now(),
    )
    .unwrap();

    let rows2 = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let ti = rows2.iter().find(|r| r.kind == "transfer-in").unwrap();
    assert_eq!(ti.reff, in_ref, "the ref is stable across the decision");
    assert!(
        ti.decision_ref
            .as_deref()
            .is_some_and(|d| d.starts_with("decision|")),
        "the decided inbound must now carry its decision ref; got {:?}",
        ti.decision_ref
    );
    // The still-undecided outbound stays decidable.
    let to = rows2.iter().find(|r| r.kind == "transfer-out").unwrap();
    assert!(to.decision_ref.is_none(), "the Send is still decidable");
}

/// Review r1 I1: a `TransferLink --to-event` decides BOTH legs — the outbound AND the inbound it
/// relocates onto — so the in-leg must list as `[decided: decision|N]` with the LINK's ref, not
/// `[decidable]`. (★ fault-inject: drop the TransferLink in-leg arm from the reverse-map → RED.)
#[test]
fn events_list_transfer_link_decides_both_legs() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // Buy 0.05 (covers the send), Send 0.05, Receive 0.05 (matched pair for a clean relocate).
    let csv = dir.path().join("cb.csv");
    std::fs::write(
        &csv,
        format!(
            "{HEADER}\
cb-buy,2025-01-15 12:00:00 UTC,Buy,BTC,0.05000000,USD,84000.00,4200.00,4250.00,50.00,,,\r\n\
cb-send,2025-06-20 12:00:00 UTC,Send,BTC,0.05000000,USD,105000.00,,,,,,bc1qdest\r\n\
cb-recv,2025-06-21 12:00:00 UTC,Receive,BTC,0.05000000,USD,105000.00,,,,,bc1qsender,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();

    let rows = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let out_ref = rows
        .iter()
        .find(|r| r.kind == "transfer-out")
        .unwrap()
        .reff
        .clone();
    let in_ref = rows
        .iter()
        .find(|r| r.kind == "transfer-in")
        .unwrap()
        .reff
        .clone();

    let in_id = eventref::parse_event_id(&in_ref).unwrap();
    cmd::reconcile::link_transfer(
        &vault,
        &pp(),
        &out_ref,
        TransferTarget::InEvent(in_id),
        now(),
    )
    .unwrap();

    let rows2 = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let ti = rows2.iter().find(|r| r.kind == "transfer-in").unwrap();
    let to = rows2.iter().find(|r| r.kind == "transfer-out").unwrap();
    assert!(
        ti.decision_ref
            .as_deref()
            .is_some_and(|d| d.starts_with("decision|")),
        "the linked IN-leg must be decided (not decidable); got {:?}",
        ti.decision_ref
    );
    assert!(
        to.decision_ref.is_some(),
        "the out-leg is decided by the link"
    );
    assert_eq!(
        ti.decision_ref, to.decision_ref,
        "both legs carry the SAME link decision ref"
    );
}

/// Review r1 I2: the void→re-decide remedy loop. Voiding a decision returns its target to
/// `[decidable]`; re-deciding shows the NEW decision ref. (★ fault-inject: break the `voided`
/// filter → the row stays `[decided]` after the void → RED.)
#[test]
fn events_list_void_returns_to_decidable_then_redecide() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_buy_send_recv(dir.path());

    let in_ref = cmd::inspect::events_list(&vault, &pp())
        .unwrap()
        .iter()
        .find(|r| r.kind == "transfer-in")
        .unwrap()
        .reff
        .clone();

    // Decide → decided (decision|1).
    let d1 = cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        },
        now(),
    )
    .unwrap();
    let d1_ref = d1.canonical();
    let ti = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let ti = ti.iter().find(|r| r.kind == "transfer-in").unwrap();
    assert_eq!(
        ti.decision_ref.as_deref(),
        Some(d1_ref.as_str()),
        "decided by d1"
    );

    // Void d1 → back to decidable.
    cmd::reconcile::void(&vault, &pp(), &d1_ref, now()).unwrap();
    let rows = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let ti = rows.iter().find(|r| r.kind == "transfer-in").unwrap();
    assert!(
        ti.decision_ref.is_none(),
        "a voided decision must return the row to decidable; got {:?}",
        ti.decision_ref
    );

    // Re-decide → the NEW decision ref (not the voided d1).
    let d2 = cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        },
        now(),
    )
    .unwrap();
    let rows = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let ti = rows.iter().find(|r| r.kind == "transfer-in").unwrap();
    assert_eq!(
        ti.decision_ref.as_deref(),
        Some(d2.canonical().as_str()),
        "re-decide must show the survivor, not the voided d1"
    );
    assert_ne!(ti.decision_ref.as_deref(), Some(d1_ref.as_str()));
}

/// (2) The UX-P4-11 trap: a ref printed by the REAL `events list` binary, pasted verbatim, is
/// ACCEPTED by `reclassify-outflow` (exit 0).
#[test]
fn a_listed_ref_pastes_verbatim_into_reclassify_outflow() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_buy_send_recv(dir.path());
    let bin = env!("CARGO_BIN_EXE_btctax");

    let out = std::process::Command::new(bin)
        .args(["--vault", vault.to_str().unwrap(), "events", "list"])
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "events list must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The ref is the first whitespace token on the transfer-out row.
    let ref_paste = stdout
        .lines()
        .find(|l| l.contains("transfer-out"))
        .and_then(|l| l.split_whitespace().next())
        .expect("a transfer-out row with a ref")
        .to_owned();

    let re = std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().unwrap(),
            "reconcile",
            "reclassify-outflow",
            &ref_paste,
            "--as-kind",
            "sell",
            "--amount",
            "3100",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .unwrap();
    assert_eq!(
        re.status.code(),
        Some(0),
        "a listed ref must be accepted verbatim by reclassify-outflow; stderr: {}",
        String::from_utf8_lossy(&re.stderr)
    );
}

/// (3) A pseudo-defaulted event lists as DECIDABLE — even with pseudo mode ON, the synthetic default
/// is never persisted, so no decision ref appears.
#[test]
fn pseudo_defaulted_event_lists_as_decidable() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // A lone Receive → an unknown-basis TransferIn (the pseudo self-transfer default target).
    let csv = dir.path().join("recv.csv");
    std::fs::write(
        &csv,
        format!(
            "{HEADER}\
cb-recv,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();

    // Enable pseudo mode (would synthesize a $0 self-transfer default for the inbound).
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let rows = cmd::inspect::events_list(&vault, &pp()).unwrap();
    let ti = rows
        .iter()
        .find(|r| r.kind == "transfer-in")
        .expect("the unknown-basis inbound must be listed");
    assert!(
        ti.decision_ref.is_none(),
        "a pseudo-defaulted event must list as DECIDABLE (no persisted decision); got {:?}",
        ti.decision_ref
    );

    // And no synthetic decision was persisted (the row universe is drawn from the real log).
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::ClassifyInbound(_))),
        "pseudo mode must persist no ClassifyInbound"
    );
}
