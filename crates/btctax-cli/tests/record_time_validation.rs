//! UX-P4-3 — record-time decision validation that MIRRORS the resolver (pseudo-safe).
//!
//! Every single-verb reconcile append refuses, AT RECORD TIME, any decision the resolver would
//! adjudicate as a NEW `DecisionConflict` — duplicate (first-wins), wrong-type, unknown-target, or a
//! non-revocable/unknown `void` — instead of silently recording it and surfacing the error only at the
//! next `verify`. The predicate is `btctax_core::would_conflict` (the real projection, pseudo forced
//! OFF), so record-time == resolver by construction.
//!
//! PRIVACY: synthetic Coinbase fixtures in tempdirs; no user file is read.
use btctax_cli::{cmd, CliError, Session};
use btctax_core::{EventPayload, InboundClass};
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

/// Vault with one Receive → a raw `TransferIn`. Returns (vault, in_ref).
fn vault_with_receive(dir: &Path) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let csv = dir.join("cb.csv");
    std::fs::write(
        &csv,
        format!(
            "{HEADER}\
cb-recv,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn())
            .unwrap()
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    (vault, in_ref)
}

fn self_transfer() -> InboundClass {
    InboundClass::SelfTransferMine {
        basis: None,
        acquired_at: None,
    }
}

fn count<P: Fn(&EventPayload) -> bool>(vault: &Path, pred: P) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .iter()
        .filter(|e| pred(&e.payload))
        .count()
}

/// A second live `ClassifyInbound` on the same TransferIn is a first-wins duplicate the resolver
/// flags — refused at record time, fail-closed (not appended).
#[test]
fn duplicate_classify_inbound_is_refused_at_record_time() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());

    cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now()).unwrap();
    let err = cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now())
        .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)),
        "expected Usage; got {err}"
    );
    assert!(
        err.to_string().to_lowercase().contains("duplicate"),
        "the refusal must name the duplicate conflict: {err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::ClassifyInbound(_))),
        1,
        "the duplicate must NOT be appended (fail-closed)"
    );
}

/// Buy + Send fixture → a pending TransferOut. Returns (vault, out_ref).
fn vault_with_outflow(dir: &Path) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let csv = dir.join("cb.csv");
    std::fs::write(
        &csv,
        format!(
            "{HEADER}\
cb-buy,2025-01-15 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-send,2025-06-20 12:00:00 UTC,Send,BTC,0.03000000,USD,105000.00,,,,,,bc1qdest\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn())
            .unwrap()
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferOut(_)))
            .unwrap()
            .id
            .canonical()
    };
    (vault, out_ref)
}

/// River Income fixture → a raw Income event. Returns (vault, income_ref).
fn vault_with_income(dir: &Path) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let csv = dir.join("river.csv");
    std::fs::write(
        &csv,
        "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
         2025-03-01 12:00:00,,,0.00100000,BTC,,,Income\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let income_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn())
            .unwrap()
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::Income(_) => Some(e.id.canonical()),
                _ => None,
            })
            .unwrap()
    };
    (vault, income_ref)
}

fn dispose_sell() -> btctax_core::OutflowClass {
    btctax_core::OutflowClass::Dispose {
        kind: btctax_core::DisposeKind::Sell,
    }
}
fn usd(s: &str) -> btctax_core::Usd {
    btctax_cli::eventref::parse_usd_arg(s).unwrap()
}

// ── REFUSE cases ──────────────────────────────────────────────────────────────

/// A second live `ReclassifyOutflow` on the same TransferOut → first-wins duplicate, refused.
#[test]
fn duplicate_reclassify_outflow_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_outflow(dir.path());
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        dispose_sell(),
        usd("100"),
        None,
        None,
        now(),
    )
    .unwrap();
    let err = cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        dispose_sell(),
        usd("100"),
        None,
        None,
        now(),
    )
    .unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("duplicate")
            && err.to_string().contains("events list"),
        "{err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::ReclassifyOutflow(_))),
        1
    );
}

/// A second live `ReclassifyIncome` on the same Income → first-wins duplicate, refused.
#[test]
fn duplicate_reclassify_income_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_income(dir.path());
    cmd::reconcile::reclassify_income(&vault, &pp(), &in_ref, true, None, now()).unwrap();
    let err =
        cmd::reconcile::reclassify_income(&vault, &pp(), &in_ref, false, None, now()).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("duplicate"),
        "{err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::ReclassifyIncome(_))),
        1
    );
}

/// An unknown target ref → refused (nothing exists to classify).
#[test]
fn unknown_target_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _in_ref) = vault_with_receive(dir.path());
    let bogus = btctax_core::EventId::import(
        btctax_core::identity::Source::Coinbase,
        btctax_core::identity::SourceRef::new("in|no-such-event"),
    )
    .canonical();
    let err = cmd::reconcile::classify_inbound(&vault, &pp(), &bogus, self_transfer(), now())
        .unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unknown"), "{err}");
}

/// `set-fmv` on a non-Income event (a TransferIn) → wrong-type, refused (existence/type applies even
/// though set-fmv is duplicate-EXEMPT).
#[test]
fn set_fmv_on_non_income_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let err = cmd::reconcile::set_fmv(&vault, &pp(), &in_ref, usd("100"), now()).unwrap_err();
    assert!(
        err.to_string().contains("non-Income") && err.to_string().contains("events list"),
        "{err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::ManualFmv(_))),
        0
    );
}

/// `reclassify-income` on a non-Income event (a TransferIn) → wrong-type, refused.
#[test]
fn reclassify_income_on_non_income_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let err =
        cmd::reconcile::reclassify_income(&vault, &pp(), &in_ref, true, None, now()).unwrap_err();
    assert!(err.to_string().contains("non-Income"), "{err}");
}

/// `void` of a nonexistent decision → refused.
#[test]
fn void_nonexistent_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _in_ref) = vault_with_receive(dir.path());
    let err = cmd::reconcile::void(&vault, &pp(), "decision|99", now()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unknown"), "{err}");
}

/// `void` of an already-voided decision → refused (the resolver is idempotent here; the record-time
/// layer refuses it per spec).
#[test]
fn void_already_voided_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let d1 = cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now())
        .unwrap()
        .canonical();
    cmd::reconcile::void(&vault, &pp(), &d1, now()).unwrap();
    let err = cmd::reconcile::void(&vault, &pp(), &d1, now()).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("already voided"),
        "{err}"
    );
}

// ── ACCEPT cases ──────────────────────────────────────────────────────────────

/// void→re-decide: after voiding the first classify, a second is ACCEPTED (the resolver excludes the
/// voided one, so there is no live duplicate).
#[test]
fn void_then_re_decide_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let d1 = cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now())
        .unwrap()
        .canonical();
    cmd::reconcile::void(&vault, &pp(), &d1, now()).unwrap();
    // re-decide: accepted (no live duplicate).
    cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now())
        .expect("void→re-decide must be accepted");
}

/// First REAL classify over a pseudo-defaulted target is ACCEPTED — the pseudo default is never
/// persisted, so the resolver (pseudo forced OFF in the shadow) sees no live decision to duplicate.
#[test]
fn first_classify_over_pseudo_default_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now())
        .expect("the first real classify of a pseudo-defaulted target must be accepted");
}

/// `set-fmv` is duplicate-EXEMPT: a second set-fmv on the same valid Income target is ACCEPTED
/// (ManualFmv is last-wins; re-pointing an FMV is a sanctioned correction, no conflict).
#[test]
fn second_set_fmv_on_income_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, income_ref) = vault_with_income(dir.path());
    cmd::reconcile::set_fmv(&vault, &pp(), &income_ref, usd("50"), now()).unwrap();
    cmd::reconcile::set_fmv(&vault, &pp(), &income_ref, usd("60"), now())
        .expect("a second set-fmv (last-wins correction) must be accepted");
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::ManualFmv(_))),
        2
    );
}

/// An Income payload for classify-raw (serialized so the JSON is exactly the core serde shape).
fn income_payload() -> EventPayload {
    EventPayload::Income(btctax_core::event::Income {
        sat: 5_000_000,
        usd_fmv: Some(usd("84.00")),
        fmv_status: btctax_core::event::FmvStatus::PriceDataset,
        kind: btctax_core::IncomeKind::Interest,
        business: false,
    })
}

/// [G2-6] `classify-raw` is first-wins: a second classify-raw on the same target is a duplicate,
/// refused. ★ This is the KAT that mutation-proves the `classify_raw` guard wiring — with the guard
/// deleted from `classify_raw`, THIS is the red (review r1 I1: previously a proven survivor).
#[test]
fn duplicate_classify_raw_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let json = serde_json::to_string(&income_payload()).unwrap();
    cmd::reconcile::classify_raw(&vault, &pp(), &in_ref, &json, now()).unwrap();
    let err = cmd::reconcile::classify_raw(&vault, &pp(), &in_ref, &json, now()).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("duplicate")
            && err.to_string().contains("events list"),
        "{err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::ClassifyRaw(_))),
        1
    );
}

/// [T2-I1] The validator reads the resolver's EFFECTIVE `applied` payload, not the raw log: a TransferIn
/// rewritten to Income by a live `ClassifyRaw` IS `reclassify-income`-able (accepted). A naive
/// "raw-log type" validator would false-REFUSE this — the definitional shadow gets it right.
#[test]
fn reclassify_income_on_a_classify_raw_income_target_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let json = serde_json::to_string(&income_payload()).unwrap();
    cmd::reconcile::classify_raw(&vault, &pp(), &in_ref, &json, now()).unwrap();
    cmd::reconcile::reclassify_income(&vault, &pp(), &in_ref, true, None, now())
        .expect("reclassify-income on a ClassifyRaw'd-Income target must be accepted");
}

/// [R3-I1] Accept-governed effective type via a REAL `SupersedeImport` — the OTHER `applied` writer
/// (resolve.rs:513), the channel a naive "enumerate ClassifyRaw only" validator MISSED. Accepting an
/// Income `ImportConflict` makes the target effective-Income: `set-fmv` on it is ACCEPTED, and
/// `classify-inbound` on it is REFUSED wrong-type (effective Income, not the raw log's type). The
/// definitional shadow honors the accept by construction.
#[test]
fn accept_governed_supersede_import_income_is_effective_income() {
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;

    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path()); // a raw TransferIn (id X)

    // Mint an ImportConflict on X: append a competing import with X's OWN id but an Income payload
    // (a different fingerprint → a conflict, not a dedup). This is the accept-governed `SupersedeImport`
    // channel (resolve.rs:513) — the OTHER `applied` writer.
    let x = btctax_cli::eventref::parse_event_id(&in_ref).unwrap();
    let competing = LedgerEvent {
        id: x.clone(),
        utc_timestamp: time::macros::datetime!(2025 - 03 - 01 12:00:00 UTC),
        original_tz: time::UtcOffset::UTC,
        wallet: Some(btctax_core::WalletId::Exchange {
            provider: "coinbase".into(),
            account: "default".into(),
        }),
        payload: income_payload(),
    };
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        append_import_batch(s.conn(), &[competing]).unwrap();
        s.save().unwrap();
    }
    let conflict_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn())
            .unwrap()
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::ImportConflict(_) => Some(e.id.canonical()),
                _ => None,
            })
            .expect("appending a same-id competing import must mint an ImportConflict")
    };
    // Accept the conflict → a real SupersedeImport that makes X effective-Income (resolve.rs:513).
    cmd::reconcile::accept_conflict(&vault, &pp(), &conflict_ref, now()).unwrap();

    // set-fmv on the accept-governed Income target → ACCEPTED (effective type is Income).
    cmd::reconcile::set_fmv(&vault, &pp(), &in_ref, usd("50"), now())
        .expect("set-fmv on an accept-governed Income target must be accepted");

    // classify-inbound on it → REFUSED wrong-type: the EFFECTIVE payload is Income, not a TransferIn.
    let err = cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, self_transfer(), now())
        .unwrap_err();
    assert!(
        err.to_string().contains("non-TransferIn"),
        "classify-inbound on an accept-governed Income target is wrong-type: {err}"
    );
}

/// [R4-M3] …and voiding that `ClassifyRaw` reverts the effective type back to TransferIn, so the same
/// `reclassify-income` is then REFUSED wrong-type. Together with the accept above, this pins that the
/// shadow tracks the resolver's `applied` in BOTH directions.
#[test]
fn voiding_the_classify_raw_reverts_effective_type_and_refuses_reclassify_income() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_receive(dir.path());
    let json = serde_json::to_string(&income_payload()).unwrap();
    let craw = cmd::reconcile::classify_raw(&vault, &pp(), &in_ref, &json, now())
        .unwrap()
        .canonical();
    cmd::reconcile::void(&vault, &pp(), &craw, now()).unwrap();
    let err =
        cmd::reconcile::reclassify_income(&vault, &pp(), &in_ref, true, None, now()).unwrap_err();
    assert!(
        err.to_string().contains("non-Income"),
        "after voiding the ClassifyRaw the target is a raw TransferIn again: {err}"
    );
}
