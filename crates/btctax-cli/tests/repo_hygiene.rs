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

/// ★ No `include_str!` / `include_bytes!` in a PUBLISHED crate may reach outside its own crate root.
///
/// Such a path is **invisible to every gate we run**. `cargo publish`'s verification step builds
/// lib+bins only, so an include inside `#[cfg(test)]` never resolves during packaging — the publish
/// SUCCEEDS and uploads a tarball whose tests cannot compile. Nothing in `make check` or CI runs
/// `cargo package` at all.
///
/// This nearly shipped in v0.14.0: `form6251.rs` did
/// `include_str!("../../../../design/amt-form6251/vectors.json")`, and `cargo package --list` carried
/// **zero** files under `design/` — the 11 AMT vector KATs, the entire subject of that release, would
/// have gone dark for anyone building from crates.io. Caught by a pre-release review, not by a test.
/// Now by a test.
#[test]
fn no_published_crate_includes_a_file_outside_its_own_root() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf();

    let tracked = std::process::Command::new("git")
        .args(["ls-files", "crates/*/src/**/*.rs", "crates/*/src/*.rs"])
        .current_dir(&repo)
        .output()
        .expect("git ls-files");
    let files: Vec<&str> = std::str::from_utf8(&tracked.stdout)
        .expect("utf8")
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    assert!(
        files.len() > 50,
        "expected the whole source tree; got {}",
        files.len()
    );

    // `xtask` and `btctax-oracle-harness` are `publish = false`; an escaping include is harmless there.
    const UNPUBLISHED: [&str; 2] = ["crates/xtask/", "crates/btctax-oracle-harness/"];

    let mut offenders = Vec::new();
    for rel in files {
        if UNPUBLISHED.iter().any(|u| rel.starts_with(u)) {
            continue;
        }
        let text = std::fs::read_to_string(repo.join(rel)).expect(rel);
        for (i, line) in text.lines().enumerate() {
            let Some(rest) = line
                .split_once("include_str!")
                .or_else(|| line.split_once("include_bytes!"))
                .map(|(_, r)| r)
            else {
                continue;
            };
            // The argument is the first string literal after the macro name.
            let Some(arg) = rest.split('"').nth(1) else {
                continue;
            };
            if arg.contains("../") && arg.matches("../").count() >= 3 {
                offenders.push(format!("{rel}:{}: {}", i + 1, arg));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a published crate includes a file outside its own root — `cargo package` will silently drop \
         it and ship a broken tarball. Move the file under the crate's own `src/`:\n{}",
        offenders.join("\n")
    );
}

/// ★ Every intra-workspace dependency must carry an EXACT `version` equal to the workspace's own.
///
/// **What this actually catches — measured, not assumed.** The obvious hazard ("a bump misses a pin, so
/// the publish ships a stale dependency") turns out NOT to be one for an exact requirement: lowering
/// `btctax-core` to `version = "0.13.0"` while the path crate is `0.14.0` makes **cargo itself** refuse
/// to resolve the workspace — *"failed to select a version for the requirement `btctax-core = ^0.13.0`;
/// candidate versions found which didn't match: 0.14.0"*. Any build catches that. This test is
/// redundant there, and the first draft of this comment claimed otherwise.
///
/// The two cases it does catch, both verified by mutation:
///  1. **A LOOSE requirement** — `version = ">=0.13"` resolves happily against the local path crate, so
///     the workspace builds green and the publish succeeds, but the *published* manifest lets a consumer
///     resolve `btctax-cli 0.14.0` against `btctax-core 0.13.0`. On this release that means re-shipping
///     the AMT false-refusal bug v0.14.0 fixed. Silent at every stage. This is the real reason to have
///     the test.
///  2. **A `path` dep with no `version` at all** — unpublishable; `cargo publish` fails, but late, after
///     earlier crates in the dependency order are already permanently on crates.io.
///
/// Also asserts each package's own version matches, so the whole workspace moves in lockstep.
#[test]
fn every_intra_workspace_dependency_pins_the_current_version() {
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ dir");
    let want = env!("CARGO_PKG_VERSION"); // every crate in this workspace shares one version

    let mut manifests: Vec<_> = std::fs::read_dir(crates_dir)
        .expect("read crates/")
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("Cargo.toml"))
        .filter(|p| p.is_file())
        .collect();
    manifests.sort();
    assert!(
        manifests.len() >= 10,
        "expected the whole workspace; found {} manifests",
        manifests.len()
    );

    let mut problems = Vec::new();
    for m in &manifests {
        let text = std::fs::read_to_string(m).expect("read manifest");
        let name = m.display().to_string();

        // The crate's own `version = "..."`, which must also be the shared one.
        if let Some(own) = text
            .lines()
            .find(|l| l.trim_start().starts_with("version = \""))
            .and_then(|l| l.split('"').nth(1))
        {
            if own != want {
                problems.push(format!("{name}: package version {own} != {want}"));
            }
        }

        // Every `btctax-* = { path = ..., version = "..." }` line.
        for (i, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("btctax") || !trimmed.contains("path =") {
                continue;
            }
            match trimmed.split("version = \"").nth(1).and_then(|r| r.split('"').next()) {
                // A path dep with NO version cannot be published at all.
                None => problems.push(format!(
                    "{name}:{}: intra-workspace dep has `path` but no `version` — unpublishable: {trimmed}",
                    i + 1
                )),
                Some(v) if v != want => problems.push(format!(
                    "{name}:{}: pins {v}, workspace is {want} — a publish would ship a STALE dependency: {trimmed}",
                    i + 1
                )),
                Some(_) => {}
            }
        }
    }
    assert!(
        problems.is_empty(),
        "intra-workspace version pins are out of lockstep with {want}:\n{}",
        problems.join("\n")
    );
}
