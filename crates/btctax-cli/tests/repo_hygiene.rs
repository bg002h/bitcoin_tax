//! Repository-hygiene KATs (burndown-3 D4, closing CI N-2).
//!
//! Locks the tracked executable bit on the git-hook scripts. The original mode-644 fail-open
//! (the hooks were tracked non-executable, so `pre-push` silently never ran) was caught only
//! empirically — this test asserts the index mode permanently.
//!
//! **Fail-closed by design:** if `git` is unavailable, the command errors, or either file is
//! missing from the index, the test FAILS — there is deliberately NO skip-if-not-git arm,
//! because the regression this locks was itself a fail-open. The workspace gate always runs
//! inside a real git checkout (locally, in worktrees, and in CI via actions/checkout), so a
//! loud failure on a source-tarball test run is acceptable and intended.

use std::process::Command;

/// `git ls-files -s scripts/pre-push scripts/pii-scan-generic.sh` must list BOTH files with
/// index mode 100755 (tracked executable).
///
/// cwd note (R0-N2): cargo runs integration tests with cwd = the crate manifest dir, and
/// `git ls-files` resolves pathspecs relative to cwd — so the repo root (two `parent()` hops
/// from `btctax-cli`'s manifest dir) is set explicitly as the child's working directory.
#[test]
fn hook_scripts_are_tracked_executable_100755() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root is two parent() hops from crates/btctax-cli");

    let out = Command::new("git")
        .args([
            "ls-files",
            "-s",
            "scripts/pre-push",
            "scripts/pii-scan-generic.sh",
        ])
        .current_dir(repo_root)
        .output()
        .expect("git must be runnable (fail-closed: no skip-if-not-git arm)");
    assert!(
        out.status.success(),
        "git ls-files must succeed (fail-closed); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "both scripts/pre-push and scripts/pii-scan-generic.sh must be tracked in the index \
         (fail-closed: a missing file is a failure, not a skip); got:\n{stdout}"
    );
    for line in &lines {
        assert!(
            line.starts_with("100755"),
            "hook script must be tracked with index mode 100755 (executable) — the mode-644 \
             fail-open regression (CI I-1) must never recur; got: {line}"
        );
    }
}
