//! §3.4 TUI clock-seam KATs (I-2) — a malformed/empty `BTCTAX_NOW` must exit 2 BEFORE raw mode, naming the
//! variable, mirroring the CLI seam (`btctax-cli/tests/btctax_now_seam.rs`). The error path exits before
//! `enable_raw_mode`, so no TTY is needed and `Command::output` (piped) is safe. This pins the env half of
//! the seam that the goldens (which inject `Clock::Pinned` directly) never exercise.

use std::process::Command;

fn run_with_btctax_now(now: &str) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_btctax-tui-edit"))
        .env("BTCTAX_NOW", now)
        .output()
        .expect("spawn btctax-tui-edit");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn malformed_btctax_now_exits_2_naming_the_var() {
    let (code, err) = run_with_btctax_now("not-a-date");
    assert_eq!(code, 2, "malformed BTCTAX_NOW must exit 2; stderr: {err}");
    assert!(
        err.contains("BTCTAX_NOW"),
        "the error must name the variable; got: {err}"
    );
}

#[test]
fn empty_btctax_now_exits_2() {
    let (code, err) = run_with_btctax_now("");
    assert_eq!(code, 2, "empty BTCTAX_NOW must exit 2; stderr: {err}");
}
