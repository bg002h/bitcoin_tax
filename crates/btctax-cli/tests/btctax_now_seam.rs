//! P0 seam KATs — `BTCTAX_NOW` must make decision-bearing stdout deterministic without changing behavior
//! when unset. Binary-level (real process) so the seam is proven end-to-end, closing the gap the
//! library-level `now`-injection tests dodge. See SPEC §3.2 (T-P0.1..6) and
//! design/usage-examples/IMPLEMENTATION_PLAN_usage_examples.md Task 0.1/0.2.
use std::path::Path;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_btctax")
}

/// Run `btctax` in `cwd` with the SPEC §3.3 pinned environment; return (code, stdout, stderr).
fn run_in(cwd: &Path, extra_env: &[(&str, &str)], args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(bin());
    c.current_dir(cwd)
        .env("BTCTAX_PASSPHRASE", "pw")
        .env("BTCTAX_PRICE_CACHE", cwd.join("no-such-cache.csv"))
        .env("HOME", cwd) // scrub HOME to a fixed path (§3.3 uniform contract)
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .args(args);
    for (k, v) in extra_env {
        c.env(k, v);
    }
    let out = c.output().expect("spawn btctax");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Build a vault with a forward-method election recorded under a pinned clock; return the tempdir.
/// `config --set-forward-method` records a decision whose made-date defaults to `now` (BTCTAX_NOW),
/// and `verify` prints that `recorded` date — a clock-derived surface (SPEC §3.2).
fn vault_with_election(now: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let (c, _o, e) = run_in(dir.path(), &[], &["--vault", "v.pgp", "init", "--key-backup", "k.asc"]);
    assert_eq!(c, 0, "init failed: {e}");
    let (c, _o, e) = run_in(
        dir.path(),
        &[("BTCTAX_NOW", now)],
        &["--vault", "v.pgp", "config", "--set-forward-method", "hifo"],
    );
    assert_eq!(c, 0, "config failed: {e}");
    dir
}

#[test] // T-P0.3
fn malformed_btctax_now_is_exit_2_naming_the_var() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _out, err) = run_in(
        dir.path(),
        &[("BTCTAX_NOW", "not-a-date")],
        &["--vault", "v.pgp", "init", "--key-backup", "k.asc"],
    );
    assert_eq!(code, 2, "malformed BTCTAX_NOW must exit 2; stderr: {err}");
    assert!(err.contains("BTCTAX_NOW"), "error must name the variable; got: {err}");
}

#[test] // T-P0.3 (empty)
fn empty_btctax_now_is_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _o, err) = run_in(
        dir.path(),
        &[("BTCTAX_NOW", "")],
        &["--vault", "v.pgp", "init", "--key-backup", "k.asc"],
    );
    assert_eq!(code, 2, "empty BTCTAX_NOW must exit 2; stderr: {err}");
    assert!(err.contains("BTCTAX_NOW"), "error must name the variable; got: {err}");
}

#[test] // T-P0.4
fn banner_on_stderr_when_set_never_on_stdout_absent_when_unset() {
    let dir = vault_with_election("2025-05-01T00:00:00Z");
    let (_c, out, err) = run_in(
        dir.path(),
        &[("BTCTAX_NOW", "2025-05-02T00:00:00Z")],
        &["--vault", "v.pgp", "verify"],
    );
    assert!(err.contains("BTCTAX_NOW override active"), "banner must be on stderr; got: {err}");
    assert!(!out.contains("BTCTAX_NOW override active"), "banner must NOT be on stdout");
    // unset: no banner
    let (_c, _out, err2) = run_in(dir.path(), &[], &["--vault", "v.pgp", "verify"]);
    assert!(!err2.contains("BTCTAX_NOW override active"), "no banner when unset; got: {err2}");
}

#[test] // T-P0.2 + T-P0.5
fn recorded_date_roundtrips_through_binary_and_is_twice_run_identical() {
    let dir = vault_with_election("2025-05-01T00:00:00Z");
    // T-P0.2: the pinned made-date surfaces in verify's `recorded`
    let (code, out, _e) = run_in(
        dir.path(),
        &[("BTCTAX_NOW", "2025-05-01T00:00:00Z")],
        &["--vault", "v.pgp", "verify"],
    );
    assert!(out.contains("2025-05-01"), "recorded date must reflect BTCTAX_NOW; got: {out}");
    // T-P0.5: byte-identical stdout AND exit code across two runs
    let (code2, out2, _e2) = run_in(
        dir.path(),
        &[("BTCTAX_NOW", "2025-05-01T00:00:00Z")],
        &["--vault", "v.pgp", "verify"],
    );
    assert_eq!((code, out), (code2, out2), "twice-run must be byte-identical");
}

#[test] // T-P0.1
fn unset_seam_behaves_normally() {
    // With BTCTAX_NOW unset the command still succeeds and prints no banner (inactive path unchanged).
    let dir = vault_with_election("2025-05-01T00:00:00Z"); // building under a pin is fine
    let (code, out, err) = run_in(dir.path(), &[], &["--vault", "v.pgp", "verify"]);
    assert_eq!(code, 0, "verify should succeed; stderr: {err}");
    assert!(!err.contains("override active"));
    assert!(out.contains("recorded 2025-05-01"), "verify still renders the election's recorded date");
}
