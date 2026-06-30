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
