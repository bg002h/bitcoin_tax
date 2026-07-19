//! Binary-level `Command::Config` dispatch tests (A-M3).
//!
//! These tests drive the actual `btctax config` CLI arm via `std::process::Command` rather than
//! calling the library functions (`cmd::admin::*`) directly. The distinction is load-bearing:
//! the clap dispatch in `main.rs::run()` contains guards (the attest-guard, the apply-all
//! no-early-return fix) and flag-combination logic that library-level tests do not exercise.
//!
//! The `fr9_exit_code.rs` pattern is reused: `CARGO_BIN_EXE_btctax` gives the freshly-built
//! binary path; `BTCTAX_PASSPHRASE` supplies the passphrase without an interactive prompt.
use btctax_cli::cmd;
use btctax_core::event::EventPayload;
use btctax_core::persistence;
use btctax_core::project::FeeTreatment;
use btctax_core::LotMethod;
use btctax_store::Passphrase;
use std::path::Path;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// Run `btctax --vault <vault> config <args...>` via the compiled binary.
/// Returns `(exit_code, stdout_text)`.
fn run_config(vault: &Path, args: &[&str]) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let output = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().expect("vault path is valid UTF-8"))
        .arg("config")
        .args(args)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute successfully");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let code = output
        .status
        .code()
        .expect("btctax process must exit normally (not via signal)");
    (code, stdout)
}

/// A-M3 — apply-all: `config --set-fee-treatment b --set-pre2025-method lifo` via the real binary
/// must apply BOTH flags (the old if/else dispatch silently dropped `--set-fee-treatment` when
/// `--set-pre2025-method` was also provided). This test FAILS if the `Command::Config` arm reverts
/// to the old early-return dispatch.
#[test]
fn config_binary_apply_all_both_flags_take_effect() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Drive the real clap Command::Config arm with both flags in one invocation.
    let (code, stdout) = run_config(
        &vault,
        &["--set-fee-treatment", "b", "--set-pre2025-method", "lifo"],
    );
    assert_eq!(
        code, 0,
        "config --set-fee-treatment b --set-pre2025-method lifo must exit 0; stdout: {stdout}"
    );

    // Read back via the library to verify both mutations landed (not silently dropped).
    let cfg = cmd::admin::show_config(&vault, &pp()).unwrap();
    assert_eq!(
        cfg.fee_treatment,
        FeeTreatment::TreatmentB,
        "fee_treatment must be B after --set-fee-treatment b (not silently dropped by old dispatch)"
    );
    assert_eq!(
        cfg.pre2025_method,
        LotMethod::Lifo,
        "pre2025_method must be Lifo after --set-pre2025-method lifo (not silently dropped)"
    );
}

/// A-M3 — apply-all incl. forward method: `config --set-forward-method hifo --set-fee-treatment b`
/// via the real binary must apply BOTH — the `MethodElection` is appended AND the fee-treatment
/// flag mutates (no early return after the decision append). This test FAILS if the `Command::Config`
/// arm has the old `return Ok(())` early-exit after recording the MethodElection.
#[test]
fn config_binary_set_forward_method_and_fee_treatment_both_apply() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Drive the real clap Command::Config arm with forward-method + fee-treatment together.
    let (code, stdout) = run_config(
        &vault,
        &["--set-forward-method", "hifo", "--set-fee-treatment", "b"],
    );
    assert_eq!(
        code, 0,
        "config --set-forward-method hifo --set-fee-treatment b must exit 0; stdout: {stdout}"
    );

    // Open a single session to check BOTH assertions; close it before the function returns.
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();

    // (1) A MethodElection must have been appended (--set-forward-method not dropped).
    let events = persistence::load_all(s.conn()).unwrap();
    let me = events.iter().find_map(|e| match &e.payload {
        EventPayload::MethodElection(m) => Some(m.clone()),
        _ => None,
    });
    assert!(
        matches!(me, Some(ref m) if m.method == LotMethod::Hifo),
        "a HIFO MethodElection must be persisted via the binary dispatch (--set-forward-method not dropped)"
    );

    // (2) The fee-treatment mutation must also have taken effect (co-flag not dropped by early return).
    let cfg = s.config().unwrap();
    assert_eq!(
        cfg.fee_treatment,
        FeeTreatment::TreatmentB,
        "fee_treatment must be B — co-passed --set-fee-treatment must not be silently dropped by an early return"
    );
}

/// A-M3 — attest-guard: `config --attest-pre2025-method` WITHOUT `--set-pre2025-method` must
/// produce a non-zero exit code (CliError::Usage → exit 2 in main.rs). This test FAILS if the
/// guard `if attest_pre2025_method && set_pre2025_method.is_none()` in `Command::Config` is
/// removed — the binary would silently no-op instead of returning an error.
#[test]
fn config_binary_attest_without_set_pre2025_method_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Attest flag without --set-pre2025-method must fail.
    let (code, _) = run_config(&vault, &["--attest-pre2025-method"]);
    assert_ne!(
        code, 0,
        "config --attest-pre2025-method without --set-pre2025-method must exit non-zero (Usage error)"
    );
}

/// §A.5(a) T3 — `config --set-forward-method <m> --exchange exchange:PROVIDER:ACCOUNT` appends a
/// SCOPED `MethodElection` (wallet in the PAYLOAD) when the account is known; and rejects LOUDLY an
/// unknown account, a `self:` scope, and an orphan `--exchange` (no `--set-forward-method`).
#[test]
fn config_set_forward_method_exchange_scoped() {
    use btctax_core::event::{Acquire, BasisSource};
    use btctax_core::identity::{Source, SourceRef, WalletId};
    use btctax_core::persistence::append_import_batch;
    use btctax_core::{EventId, LedgerEvent};
    use time::macros::datetime;
    use time::UtcOffset;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Seed a KNOWN exchange account (coinbase:main) via a synthetic Acquire import.
    let acquire = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("cb-buy-1")),
        utc_timestamp: datetime!(2025-02-01 00:00:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(WalletId::Exchange {
            provider: "coinbase".into(),
            account: "main".into(),
        }),
        payload: EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: rust_decimal_macros::dec!(50.00),
            fee_usd: rust_decimal_macros::dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };
    {
        let mut s = btctax_cli::Session::open(&vault, &pp()).unwrap();
        append_import_batch(s.conn(), &[acquire]).unwrap();
        s.save().unwrap();
    }

    // (1) Known account → success; a SCOPED MethodElection is persisted (wallet in the payload).
    let (code, stdout) = run_config(
        &vault,
        &[
            "--set-forward-method",
            "hifo",
            "--exchange",
            "exchange:coinbase:main",
        ],
    );
    assert_eq!(
        code, 0,
        "scoped election on a known account must exit 0; stdout: {stdout}"
    );
    {
        let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
        let events = persistence::load_all(s.conn()).unwrap();
        let me = events
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::MethodElection(m) => Some(m.clone()),
                _ => None,
            })
            .expect("a MethodElection must be persisted");
        assert_eq!(me.method, LotMethod::Hifo);
        assert_eq!(
            me.wallet,
            Some(WalletId::Exchange {
                provider: "coinbase".into(),
                account: "main".into()
            }),
            "the per-account scope must ride in the MethodElection PAYLOAD"
        );
    }

    // (2) Unknown account → rejected LOUDLY (a typo can't create a dead election).
    let (code_unknown, _) = run_config(
        &vault,
        &[
            "--set-forward-method",
            "hifo",
            "--exchange",
            "exchange:bogus:acct",
        ],
    );
    assert_ne!(
        code_unknown, 0,
        "an unknown exchange account must be rejected (non-zero exit)"
    );

    // (3) self:LABEL scope → rejected (only exchange accounts are electable).
    let (code_self, _) = run_config(
        &vault,
        &["--set-forward-method", "hifo", "--exchange", "self:cold"],
    );
    assert_ne!(
        code_self, 0,
        "a self:LABEL scope must be rejected (only exchange accounts are electable)"
    );

    // (4) Orphan --exchange (no --set-forward-method) → rejected (mirror the attest-guard).
    let (code_orphan, _) = run_config(&vault, &["--exchange", "exchange:coinbase:main"]);
    assert_ne!(
        code_orphan, 0,
        "--exchange without --set-forward-method must exit non-zero"
    );

    // Exactly ONE MethodElection remains (the rejected attempts appended nothing).
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    let events = persistence::load_all(s.conn()).unwrap();
    let n = events
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::MethodElection(_)))
        .count();
    assert_eq!(
        n, 1,
        "rejected scoped-election attempts must append nothing"
    );
}

/// UX-P4-12(c): `config` (show) echoes the forward-method standing order that
/// `config --set-forward-method` records — previously readable only in `verify`'s Standing-orders
/// block. A fresh vault names the Fifo default; once an order exists, config shows it with its status.
#[test]
fn config_shows_forward_method_standing_order() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let (code, fresh) = run_config(&vault, &[]);
    assert_eq!(code, 0, "config show exits 0; stdout: {fresh}");
    assert!(
        fresh.contains("forward_method: FIFO (vault-wide default)"),
        "a fresh vault names the FIFO vault-wide default:\n{fresh}"
    );

    // Record a forward-looking standing order (far-future effective date ⇒ deterministically in force).
    let (sc, _) = run_config(
        &vault,
        &[
            "--set-forward-method",
            "hifo",
            "--effective-from",
            "2099-01-01",
        ],
    );
    assert_eq!(sc, 0, "recording a standing order must exit 0");

    let (_c, after) = run_config(&vault, &[]);
    assert!(
        after.contains("forward_method: HIFO (vault-wide standing order, effective 2099-01-01)"),
        "config echoes the recorded vault-wide forward method:\n{after}"
    );
    assert!(
        !after.contains("forward_method: FIFO (vault-wide default)"),
        "the vault-wide default line is replaced once a GLOBAL order is in force:\n{after}"
    );
}

/// UX-P4-12(c) fold r1-I2: a PER-ACCOUNT (scoped) standing order governs ONLY its exchange account —
/// config must NOT report it as the vault-wide `forward_method:`, must still name the FIFO vault-wide
/// default, and must attribute the scoped order to its account.
#[test]
fn config_scoped_forward_method_is_not_reported_vault_wide() {
    use btctax_core::event::{Acquire, BasisSource, EventPayload};
    use btctax_core::identity::{Source, SourceRef, WalletId};
    use btctax_core::persistence::append_import_batch;
    use btctax_core::{EventId, LedgerEvent};
    use time::macros::datetime;
    use time::UtcOffset;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let acquire = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("cb-buy-scope")),
        utc_timestamp: datetime!(2025-02-01 00:00:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(WalletId::Exchange {
            provider: "coinbase".into(),
            account: "main".into(),
        }),
        payload: EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: rust_decimal_macros::dec!(50.00),
            fee_usd: rust_decimal_macros::dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };
    {
        let mut s = btctax_cli::Session::open(&vault, &pp()).unwrap();
        append_import_batch(s.conn(), &[acquire]).unwrap();
        s.save().unwrap();
    }

    let (sc, set_out) = run_config(
        &vault,
        &[
            "--set-forward-method",
            "hifo",
            "--exchange",
            "exchange:coinbase:main",
            "--effective-from",
            "2099-01-01",
        ],
    );
    assert_eq!(sc, 0, "scoped election on a known account must exit 0");
    // fold r1-M1(a): the scoped-set confirmation uses the human method label, not raw Debug `Hifo`.
    assert!(
        set_out.contains("attests HIFO") && !set_out.contains("Hifo"),
        "the set confirmation reads human, not Debug:\n{set_out}"
    );

    let (_c, out) = run_config(&vault, &[]);
    assert!(
        out.contains("forward_method: FIFO (vault-wide default)"),
        "the vault-wide method stays FIFO — a scoped order does not change it:\n{out}"
    );
    assert!(
        out.contains("forward_method for exchange:coinbase:main: HIFO"),
        "the scoped order is attributed to its account, not reported vault-wide:\n{out}"
    );
}

/// UX-P4-12(e): `config` (show) uses human labels, not raw Debug enum-variant names — `TreatmentC`
/// (→ the TP8 (c) description) and `Hifo` (→ `HIFO`) must not leak on screen.
#[test]
fn config_show_uses_human_labels_not_debug_enum_names() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let (code, out) = run_config(&vault, &[]);
    assert_eq!(code, 0, "config show exits 0; stdout: {out}");
    assert!(
        out.contains("fee_treatment: non-taxable, basis carries (TP8 c)"),
        "fee treatment reads human:\n{out}"
    );
    assert!(
        out.contains("pre2025_method: HIFO"),
        "lot method reads human:\n{out}"
    );
    assert!(
        !out.contains("TreatmentC") && !out.contains("Hifo") && !out.contains("Fifo"),
        "no raw Debug variant names leak on screen:\n{out}"
    );
}
