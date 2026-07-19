//! §3.4 TUI clock-seam KAT (I-2) — the viewer's `BTCTAX_NOW` error path exits 2 before raw mode, naming
//! the variable (mirrors the CLI seam + the btctax-tui-edit KAT). No TTY needed: the malformed-value arm
//! exits before `enable_raw_mode`.

use std::process::Command;

fn run_with_btctax_now(now: &str) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_btctax-tui"))
        .env("BTCTAX_NOW", now)
        .output()
        .expect("spawn btctax-tui");
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
