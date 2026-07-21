//! Conservative-filing Task 8 (CLI wiring) — the VOID-direction BG-D9 prior-year fold-diff advisory
//! reaches the real `btctax reconcile void` verb. Voiding a live `PromoteTranche` reverts a filed floor
//! basis toward `$0`, which HIFO-reorders a PRIOR filed year's disposals (amend-to-PAY). This drives the
//! actual binary (`std::process::Command`) so the wiring — not just the core builder — is exercised: the
//! `Direction::Void` lines must PRINT before the void is recorded.
//!
//! Setup is hand-built via `persistence` (there is no CLI `promote` verb yet — that consent screen is
//! Task 10 — so the promote is appended directly, exactly as `declare_tranche_cli.rs` hand-crafts a raw
//! void). PRIVACY: synthetic values in a tempdir; no user file is read.

use btctax_cli::Session;
use btctax_core::conservative::Coverage;
use btctax_core::event::{
    Acknowledgment, Acquire, BasisSource, DeclareTranche, Dispose, DisposeKind, EventPayload,
    FloorMethod, PromoteTranche,
};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::{append_decision, append_import_batch, load_all};
use btctax_core::LedgerEvent;
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::datetime;
use time::UtcOffset;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026 - 01 - 01 0:00 UTC)
}
/// The single Exchange wallet the documented buy, the tranche, and the sell all share — so a promoted
/// tranche can HIFO-reorder the sell's draw.
fn wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload,
    }
}

/// A vault with a documented 0.6-BTC lot ($5,000/BTC), a 0.4-BTC tranche PROMOTED to a $12,000 floor
/// ($30,000/BTC — higher per-sat, so HIFO draws it FIRST), and a 2018 sell of EXACTLY 0.4 BTC. WITH the
/// promote the sell drains the tranche (gain $8,000); voiding it reverts the tranche to $0 (sorted LAST),
/// so the sell instead drains the documented lot (gain $18,000) — the amend-to-PAY reorder the advisory
/// warns about. Returns (vault, promote decision id).
fn build_promoted_vault(dir: &Path) -> (PathBuf, EventId) {
    let vault = dir.join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();
    let buy = imp(
        "BUY",
        datetime!(2017-01-01 00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 60_000_000,
            usd_cost: dec!(3_000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = imp(
        "SELL",
        datetime!(2018-09-01 00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 40_000_000,
            usd_proceeds: dec!(20_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    append_import_batch(s.conn(), &[buy, sell]).unwrap();

    // DeclareTranche (decision 1) then PromoteTranche targeting it (decision 2).
    let tranche_id = append_decision(
        s.conn(),
        EventPayload::DeclareTranche(DeclareTranche {
            sat: 40_000_000,
            wallet: wallet(),
            window_start: time::macros::date!(2018 - 01 - 01),
            window_end: time::macros::date!(2018 - 03 - 31),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    let promote_id = append_decision(
        s.conn(),
        EventPayload::PromoteTranche(PromoteTranche {
            target: tranche_id,
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(12_000),
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I understand and accept the risk".into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    s.save().unwrap();
    (vault, promote_id)
}

/// Run `btctax --vault <vault> reconcile void <target>`; returns (exit, stdout, stderr).
fn run_void(vault: &Path, target: &str) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let out = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().unwrap())
        .arg("reconcile")
        .arg("void")
        .arg(target)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn decision_count(vault: &Path) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    load_all(s.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .count()
}

/// ★ Task 8 (§6 / arch r1 I-3): `reconcile void` on a live promote PRINTS the `Direction::Void`
/// prior-year advisory (amend-to-PAY) before recording, and still records the void. Dropping the wiring
/// leaves stdout without the 1040-X / additional-tax warning (surfacing mutation).
#[test]
fn voiding_a_promoted_tranche_prints_the_void_direction_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, promote_id) = build_promoted_vault(dir.path());
    let before = decision_count(&vault);

    let (code, stdout, stderr) = run_void(&vault, &promote_id.canonical());
    assert_eq!(code, 0, "the void must succeed; stderr: {stderr}");
    // The VOID-direction lines: the 2018 rewrite, its 1040-X implication, and the amend-to-PAY wording.
    assert!(
        stdout.contains("2018"),
        "the advisory names the affected filed year 2018: {stdout}"
    );
    assert!(
        stdout.contains("1040-X"),
        "the advisory names the Form 1040-X implication: {stdout}"
    );
    assert!(
        stdout.to_lowercase().contains("additional tax"),
        "voiding a promote over a filed floor-year is amend-to-PAY (additional tax): {stdout}"
    );
    // The void was still recorded (the advisory is non-gating).
    assert_eq!(
        decision_count(&vault),
        before + 1,
        "the void decision is still recorded after the warning"
    );
}
