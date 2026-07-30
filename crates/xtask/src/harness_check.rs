//! A1 — **the harness-is-installed gate.** (`design/HARNESS.md` r2, class α.)
//!
//! ★★ **Why this exists, and why it is the FIRST thing built.** `scripts/pre-push` is a reviewed,
//! hardened PII gate — executable, committed 2026-07-02, and carrying its own install command in its
//! header. It has **never run**: `core.hooksPath` was unset and `.git/hooks/` held nothing but
//! `*.sample`. A hook that is written but not wired is not a weak gate, it is a **zero** gate that
//! looks like a gate — which is F4 (*"claimed a checker was working while it was blind"*) relocated
//! from the checker to its installation.
//!
//! So every other mechanism in the harness depends on this one. Shipping a `pre-commit` hook without
//! a test that reds while it is unwired would reproduce, on day one, the exact failure the harness was
//! built to answer.
//!
//! **What "wired" means here.** Not "a file with the right name exists" — an empty or foreign
//! `pre-commit` would satisfy that and run no gate at all. The installed hook must (a) resolve through
//! whichever install route the developer chose, (b) be **executable**, and (c) contain the
//! [`HARNESS_SENTINEL`], which is the shape-check that says *this is our hook* rather than *some file
//! is present*. Both documented install routes are honoured, because `git rev-parse --git-path hooks`
//! resolves `core.hooksPath` for us:
//!
//! ```text
//! git config core.hooksPath scripts                    # whole directory
//! ln -s ../../scripts/pre-commit .git/hooks/pre-commit # per hook
//! ```
//!
//! ★ **Shape, not path** — the same rule A3 states. The sentinel travels with the content, so a copied
//! hook, a symlinked hook, and a `core.hooksPath` hook all verify identically, and a *renamed* script
//! does not silently disable the gate.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The marker every btctax-harness hook carries in its text.
///
/// ★ Checked by *content*, so it survives copying, symlinking and `core.hooksPath` alike. A presence
/// check on the filename alone would pass on an empty file, which is the failure this gate exists to
/// make impossible.
pub const HARNESS_SENTINEL: &str = "btctax-harness-hook";

/// A hook the harness requires to be wired, and what it protects.
pub struct RequiredHook {
    /// Git hook name, i.e. the filename git will execute.
    pub name: &'static str,
    /// The tracked script that must be installed under that name.
    pub tracked: &'static str,
    /// The observed failure this hook closes — quoted in the failure message, because a gate that
    /// cannot say what it is protecting gets bypassed on reflex.
    pub protects: &'static str,
}

/// ★ Both hooks, together. `pre-push` is included even though it predates the harness: leaving it
/// out would leave the corpse on display — a hook this very module cites as the reason it exists,
/// still unwired.
pub const REQUIRED_HOOKS: &[RequiredHook] = &[
    RequiredHook {
        name: "pre-commit",
        tracked: "scripts/pre-commit",
        protects: "F3 — `make check` + `cargo fmt --check` before every commit. Git's exit status is \
                   what makes this real: with the hook wired, \"ran the gate and never read the \
                   output\" stops being expressible.",
    },
    RequiredHook {
        name: "pre-push",
        tracked: "scripts/pre-push",
        protects: "the PII gate over the pushed range (owner-specific + generic shape). Committed \
                   2026-07-02 and never once executed — the precedent that motivates this whole check.",
    },
];

/// Why a required hook is not satisfying the gate.
#[derive(Debug, PartialEq, Eq)]
pub enum Defect {
    /// Nothing resolves at the hook path.
    Missing,
    /// Present, but git will not execute it.
    NotExecutable,
    /// Present and executable, but it is not our hook — no [`HARNESS_SENTINEL`] in its text.
    /// ★ This is the case a filename check would wave through.
    Foreign,
}

impl Defect {
    fn describe(&self) -> &'static str {
        match self {
            Defect::Missing => "not installed",
            Defect::NotExecutable => "installed but NOT EXECUTABLE — git silently skips it",
            Defect::Foreign => {
                "installed but does not carry the harness sentinel — it is not the tracked hook"
            }
        }
    }
}

/// One unwired hook.
#[derive(Debug)]
pub struct Finding {
    pub hook: &'static str,
    pub tracked: &'static str,
    pub protects: &'static str,
    pub defect: Defect,
}

/// Is `path` executable by anyone? On Windows, git treats any present hook as runnable, so the
/// executable bit is not a meaningful signal there.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

/// ★★ **The pure core.** Given a hooks directory, which required hooks are not wired?
///
/// Taking `hooks_dir` as a parameter rather than discovering it is what lets the tests below plant
/// each defect in a tempdir and *watch the check go red* — B1's seen-red-once rule applied to A1
/// itself. A gate whose failure path has never been observed is exactly the thing this file exists
/// to forbid.
pub fn audit(hooks_dir: &Path) -> Vec<Finding> {
    REQUIRED_HOOKS
        .iter()
        .filter_map(|req| {
            let path = hooks_dir.join(req.name);
            // `symlink_metadata` would see the link itself; we want the target, since a dangling
            // symlink is `Missing` from git's point of view too.
            let defect = if !path.exists() {
                Some(Defect::Missing)
            } else if !is_executable(&path) {
                Some(Defect::NotExecutable)
            } else if !std::fs::read_to_string(&path)
                .map(|t| t.contains(HARNESS_SENTINEL))
                .unwrap_or(false)
            {
                Some(Defect::Foreign)
            } else {
                None
            };
            defect.map(|defect| Finding {
                hook: req.name,
                tracked: req.tracked,
                protects: req.protects,
                defect,
            })
        })
        .collect()
}

/// The hooks directory git will actually use, honouring `core.hooksPath` and worktrees.
///
/// ★ `git rev-parse --git-path hooks` resolves `core.hooksPath` itself (verified: it prints `scripts`
/// under `-c core.hooksPath=scripts`). Parsing the config by hand would be a second, divergent
/// implementation of a rule git already owns.
pub fn hooks_dir(repo_root: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None; // not a git work tree — see `should_check`
    }
    let rel = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if rel.is_empty() {
        return None;
    }
    let p = PathBuf::from(&rel);
    Some(if p.is_absolute() {
        p
    } else {
        repo_root.join(p)
    })
}

/// Whether this environment is one where the gate is meaningful at all.
///
/// ★★ **Stated plainly, because it is the one escape hatch in A1 and hiding it would be the very
/// dishonesty the harness is against.** The gate asks: *does committing from this working tree run
/// the gates?* Two environments cannot answer it:
///
/// - **CI** — nothing commits there, and CI runs the gates *natively* (`cargo test --workspace`,
///   fmt, msrv, isolation, pii-scan as separate jobs). Requiring a commit hook in a machine that
///   never commits would assert nothing, and a permanently-red CI is not a gate, it is noise that
///   gets muted — which is strictly worse than no gate.
/// - **Not a git work tree** — a packaged or vendored source tree has no hooks to wire.
///
/// **Honest limit:** `CI=1` in a local shell mutes this check. That is a real bypass and it is named
/// here rather than buried; it is accepted because the alternative (no CI skip) makes the check red
/// on every CI run forever, and a gate everyone has learned to ignore protects nothing.
pub fn should_check(hooks: Option<&PathBuf>) -> bool {
    if std::env::var_os("CI").is_some() {
        return false;
    }
    hooks.is_some()
}

/// Render findings as the message a developer should see — including how to fix it.
pub fn report(findings: &[Finding]) -> String {
    let mut s = String::from(
        "HARNESS NOT INSTALLED — the git hooks that run this project's gates are not wired up.\n\n",
    );
    for f in findings {
        s.push_str(&format!(
            "  {} — {}\n      tracked:  {}\n      protects: {}\n",
            f.hook,
            f.defect.describe(),
            f.tracked,
            f.protects
        ));
    }
    s.push_str(
        "\n  Fix (either route works; the check honours both):\n\
         \n      git config core.hooksPath scripts\n\
         \n  or, per hook:\n\
         \n      ln -s ../../scripts/pre-commit .git/hooks/pre-commit\n      \
         ln -s ../../scripts/pre-push   .git/hooks/pre-push\n\
         \n  ★ This is not bookkeeping. `scripts/pre-push` sat in this repo, reviewed and executable,\n  \
         from 2026-07-02 and never ran once, because writing a hook and wiring a hook are two acts and\n  \
         only the first one got done. That is what this check exists to make impossible.\n",
    );
    s
}

/// Repo root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

/// `cargo run -p xtask -- harness-check`
pub fn run() -> Result<(), String> {
    let root = repo_root();
    let hooks = hooks_dir(&root);
    if !should_check(hooks.as_ref()) {
        println!("harness-check: skipped (CI, or not a git work tree)");
        return Ok(());
    }
    let findings = audit(hooks.as_ref().expect("should_check requires Some"));
    if findings.is_empty() {
        println!(
            "harness-check: OK — {} hook(s) wired at {}",
            REQUIRED_HOOKS.len(),
            hooks.expect("checked").display()
        );
        return Ok(());
    }
    Err(report(&findings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a hooks dir in a tempdir with the given (name, contents, executable) hooks.
    fn plant(hooks: &[(&str, &str, bool)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, body, exec) in hooks {
            let p = dir.path().join(name);
            fs::write(&p, body).expect("write hook");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = if *exec { 0o755 } else { 0o644 };
                fs::set_permissions(&p, fs::Permissions::from_mode(mode)).expect("chmod");
            }
            #[cfg(not(unix))]
            let _ = exec;
        }
        dir
    }

    fn good_body() -> String {
        format!("#!/usr/bin/env bash\n# {HARNESS_SENTINEL}: pre-commit\nexit 0\n")
    }

    /// ★★ **THE KILL — B1 applied to A1.** Each defect is planted and the check is *observed red*.
    /// Without this, `audit` returning an empty vec unconditionally would be indistinguishable from a
    /// correctly-wired repo, and A1 would be the very decoration it was written to prevent.
    #[test]
    fn every_unwired_shape_is_caught() {
        // 1. Nothing installed at all — the state this repo was actually in.
        let dir = plant(&[]);
        let f = audit(dir.path());
        assert_eq!(
            f.len(),
            REQUIRED_HOOKS.len(),
            "all required hooks must be reported missing"
        );
        assert!(f.iter().all(|f| f.defect == Defect::Missing));

        // 2. Present and correct in content, but not executable — git skips it SILENTLY, which is
        //    the nastiest of the three because everything looks installed.
        let dir = plant(&[
            ("pre-commit", &good_body(), false),
            ("pre-push", &good_body(), true),
        ]);
        let f = audit(dir.path());
        #[cfg(unix)]
        {
            assert_eq!(f.len(), 1);
            assert_eq!(f[0].hook, "pre-commit");
            assert_eq!(f[0].defect, Defect::NotExecutable);
        }

        // 3. Executable, right filename, but NOT our hook — the case a filename-presence check waves
        //    through. An unrelated or emptied pre-commit runs no gate.
        let dir = plant(&[
            ("pre-commit", "#!/bin/sh\nexit 0\n", true),
            ("pre-push", &good_body(), true),
        ]);
        let f = audit(dir.path());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].hook, "pre-commit");
        assert_eq!(
            f[0].defect,
            Defect::Foreign,
            "a foreign hook must not satisfy the gate"
        );
    }

    /// The positive half: a correctly wired pair is accepted. Without this the check could be red for
    /// everything and the test above would still pass.
    #[test]
    fn a_correctly_wired_pair_is_accepted() {
        let dir = plant(&[
            ("pre-commit", &good_body(), true),
            ("pre-push", &good_body(), true),
        ]);
        assert!(
            audit(dir.path()).is_empty(),
            "a wired pair must satisfy the gate"
        );
    }

    /// ★ The sentinel must be present in the TRACKED scripts, or the live check below can never pass
    /// no matter how the developer installs them. This is the half that reds if someone edits a hook
    /// script and drops the marker.
    #[test]
    fn the_tracked_hook_scripts_carry_the_sentinel() {
        for req in REQUIRED_HOOKS {
            let p = repo_root().join(req.tracked);
            let body = fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("tracked hook missing: {} ({e})", p.display()));
            assert!(
                body.contains(HARNESS_SENTINEL),
                "{} does not carry the sentinel `{HARNESS_SENTINEL}` — once installed it would be \
                 reported Foreign, and the gate could never go green",
                req.tracked
            );
        }
    }

    /// ★★ **The resolution kill: `hooks_dir` must follow `core.hooksPath`, and `audit` must red on a
    /// repo that has not installed the harness.**
    ///
    /// This runs against a THROWAWAY git repo rather than by toggling this repo's own
    /// `core.hooksPath`, and that is not squeamishness — it is the harness working. The obvious way
    /// to mutation-verify A1 is `git config --unset core.hooksPath`, run the test, set it back; when
    /// that was attempted on 2026-07-30 the A2 deny **blocked it**, correctly, as a command that makes
    /// the gates stop existing. A temp repo proves the same property, touches no real config, and —
    /// unlike a manual toggle — is permanent.
    #[test]
    fn hooks_dir_follows_core_hookspath_and_audit_reds_on_an_uninstalled_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .is_ok_and(|o| o.status.success());
            assert!(ok, "git {args:?} failed");
        };
        git(&["init", "--quiet"]);

        // 1. A fresh repo has no harness — this is the state THIS repo was in for four weeks, and it
        //    must be loud rather than silent.
        let hooks = hooks_dir(root).expect("a git repo resolves a hooks dir");
        let findings = audit(&hooks);
        assert_eq!(
            findings.len(),
            REQUIRED_HOOKS.len(),
            "a fresh clone must report every required hook unwired — that is the whole mechanism"
        );

        // 2. `core.hooksPath` is followed, so the documented directory-install route verifies. If
        //    `hooks_dir` ignored the config, a correctly-installed repo would red forever and the gate
        //    would be muted within a day.
        fs::create_dir_all(root.join("myhooks")).expect("mkdir");
        git(&["config", "core.hooksPath", "myhooks"]);
        let hooks = hooks_dir(root).expect("hooks dir");
        assert!(
            hooks.ends_with("myhooks"),
            "hooks_dir must honour core.hooksPath, got {}",
            hooks.display()
        );

        // 3. Install both hooks there and the gate goes green — the red→green transition, end to end.
        for req in REQUIRED_HOOKS {
            let p = hooks.join(req.name);
            fs::write(&p, good_body()).expect("write");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
            }
        }
        assert!(
            audit(&hooks).is_empty(),
            "a repo with both hooks wired under core.hooksPath must satisfy the gate"
        );
    }

    /// ★★ **THE KILL for A2's bypass deny** (`scripts/hooks/deny-bypass.sh`).
    ///
    /// The deny is the mechanism that stops the commit gate from being routed around, so it is exactly
    /// the kind of instrument B1 forbids trusting unobserved. Both directions are pinned, and the
    /// ALLOW half is not padding: a hook that cries wolf gets muted, and a muted hook is worse than no
    /// hook at all. `git commit -m "… --no-verify …"` is the case that a substring match would fail —
    /// which is why the script tokenises with shell semantics instead.
    #[cfg(unix)]
    #[test]
    fn the_bypass_deny_blocks_real_bypasses_and_only_those() {
        use std::io::Write;
        use std::process::Stdio;

        let script = repo_root().join("scripts/hooks/deny-bypass.sh");
        assert!(script.exists(), "{} missing", script.display());

        // The hook parses its stdin JSON with jq and tokenises with python3; without them it cannot
        // work at all, so their absence is a broken harness rather than a reason to skip quietly.
        for tool in ["jq", "python3"] {
            assert!(
                Command::new(tool)
                    .arg("--version")
                    .output()
                    .is_ok_and(|o| o.status.success()),
                "`{tool}` is required by scripts/hooks/deny-bypass.sh but is not on PATH"
            );
        }

        // (command, must_block)
        let cases: &[(&str, bool)] = &[
            // Real bypasses — every one of these makes the gates stop existing.
            (r#"git commit --no-verify -m "wip""#, true),
            (r#"git commit -nm "wip""#, true),
            ("git commit -n", true),
            ("git push --no-verify", true),
            ("make check && git commit --no-verify -m x", true), // hidden behind a separator
            ("git config --unset core.hooksPath", true),         // uninstall the harness
            // Must NOT fire. The first is the false positive a substring match would produce.
            (
                r#"git commit -m "document the --no-verify escape hatch""#,
                false,
            ),
            (r#"git commit -m "wip""#, false),
            (r#"git commit -am "wip""#, false), // -am contains no `n`
            ("git status", false),
            ("git log -n 5", false), // `log` is not a gated verb
            ("ls -n /tmp", false),   // not git at all
            ("grep -n pattern file.txt", false),
        ];

        for (cmd, must_block) in cases {
            let payload = format!(
                r#"{{"tool_name":"Bash","tool_input":{{"command":{}}}}}"#,
                serde_json_string(cmd)
            );
            let mut child = Command::new(&script)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn deny-bypass.sh");
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(payload.as_bytes())
                .expect("write payload");
            let status = child.wait().expect("wait");
            let blocked = status.code() == Some(2);
            assert_eq!(
                blocked,
                *must_block,
                "deny-bypass.sh got `{cmd}`: expected {}, got exit {:?}",
                if *must_block {
                    "BLOCK (2)"
                } else {
                    "ALLOW (0)"
                },
                status.code()
            );
        }
    }

    /// Minimal JSON string encoder — xtask has no serde dependency and this needs only quotes and
    /// backslashes handled for the test payloads above.
    #[cfg(unix)]
    fn serde_json_string(s: &str) -> String {
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    /// ★★ **THE KILL for A3+A4's Write hook** (`scripts/hooks/on-write.sh`).
    ///
    /// A3 denies a primary source written outside every accounted-for tree; A4 asks once per new
    /// directory. Both directions are pinned, including the three that would quietly break it: a
    /// legitimate write INTO an archive must pass, a path outside the repo must be ignored entirely,
    /// and A4's second attempt must proceed (it is a speed bump, not a wall — a wall gets routed
    /// around with `mkdir -p`).
    #[cfg(unix)]
    #[test]
    fn the_write_hook_denies_new_archives_and_asks_once_per_new_directory() {
        use std::io::Write;
        use std::process::Stdio;

        let script = repo_root().join("scripts/hooks/on-write.sh");
        assert!(script.exists(), "{} missing", script.display());
        // ★ Isolated ack dir — see the mkdir test: a shared one-shot file makes these tests pass
        // once and fail forever after, and race each other.
        let state = tempfile::tempdir().expect("tempdir");

        let fire = |path: &str| -> Option<i32> {
            let payload = format!(
                r#"{{"tool_name":"Write","tool_input":{{"file_path":{}}}}}"#,
                serde_json_string(path)
            );
            let mut child = Command::new(&script)
                .current_dir(repo_root())
                .env("BTCTAX_HARNESS_STATE", state.path())
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn on-write.sh");
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(payload.as_bytes())
                .expect("write");
            child.wait().expect("wait").code()
        };

        // A3 — a new archive is denied wherever it is put.
        for p in [
            "docs/authority/f6251--2025.pdf",
            "notes/26USC_s55.html",
            "research/Notice_2014-21.pdf",
        ] {
            assert_eq!(fire(p), Some(2), "A3 must deny a primary source at `{p}`");
        }

        // A3 — writes into accounted-for trees, ordinary source files, and paths OUTSIDE the repo
        // must pass. The last one matters: this hook must never reach into another project.
        for p in [
            "design/forms/2025/f6251--2025.pdf",
            "crates/btctax-forms/forms/2025/f8949.pdf",
            "legal/primary-sources/statute-irc/26USC_s55.html",
            "crates/xtask/src/main.rs",
            "CONTINUITY.md",
            "/tmp/somewhere-else/f1040--2025.pdf",
        ] {
            assert_eq!(fire(p), Some(0), "`{p}` must be allowed");
        }

        // A4 — first write into a new directory asks; the retry proceeds.
        let probe = "zz-harness-selftest-dir/notes.md";
        assert_eq!(
            fire(probe),
            Some(2),
            "A4 must ASK on the first write into a new directory"
        );
        assert_eq!(
            fire(probe),
            Some(0),
            "A4 must PROCEED on the retry — a speed bump, not a wall"
        );
    }

    /// ★★★ **THE KILL for A4's Bash half — the hole its own author walked through.**
    ///
    /// A4 was built watching `Write`/`Edit` only. Within the hour, `design/forms/2026/` was created
    /// with `mkdir -p` in Bash and the ask never fired — `.git/btctax-harness/acked-dirs` was still
    /// absent, which is how the hole was found. `design/HARNESS.md` had **predicted this exact
    /// route-around in writing**, and the prediction changed nothing: a documented hole is a hole.
    ///
    /// Narrow by design, because a noisy hook gets muted — the ALLOW cases below are the specification
    /// of that narrowness, not padding.
    #[cfg(unix)]
    #[test]
    fn the_mkdir_ask_fires_for_new_repo_directories_only() {
        use std::io::Write;
        use std::process::Stdio;

        let script = repo_root().join("scripts/hooks/deny-bypass.sh");
        // ★ An isolated state dir: the ask is one-shot, so sharing the repo's ack file would make
        // this pass once and fail forever after, and race the Write-hook test besides.
        let state = tempfile::tempdir().expect("tempdir");

        let fire = |cmd: &str| -> Option<i32> {
            let payload = format!(
                r#"{{"tool_name":"Bash","tool_input":{{"command":{}}}}}"#,
                serde_json_string(cmd)
            );
            let mut child = Command::new(&script)
                .current_dir(repo_root())
                .env("BTCTAX_HARNESS_STATE", state.path())
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn deny-bypass.sh");
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(payload.as_bytes())
                .expect("write");
            child.wait().expect("wait").code()
        };

        // Asks once, then proceeds — a speed bump, not a wall.
        assert_eq!(
            fire("mkdir -p zz-probe-alpha"),
            Some(2),
            "a new repo directory must ASK"
        );
        assert_eq!(
            fire("mkdir -p zz-probe-alpha"),
            Some(0),
            "the retry must PROCEED"
        );
        assert_eq!(
            fire("mkdir zz-probe-beta/nested"),
            Some(2),
            "nested new dirs count too"
        );

        // The narrowness, spelled out. Each of these firing would make the hook noise.
        for cmd in [
            "mkdir -p /tmp/anything", // outside the repo — another project's business
            "mkdir -p target/foo",    // gitignored build output
            "mkdir -p crates",        // already exists
            "ls -la",
            "git status",
        ] {
            assert_eq!(fire(cmd), Some(0), "`{cmd}` must NOT ask");
        }

        // ★ And the addition must not have broken the deny it shares a script with.
        assert_eq!(fire("git commit --no-verify -m x"), Some(2));
        assert_eq!(fire(r#"git commit -m "mentions --no-verify""#), Some(0));
    }

    /// ★★★ **THE KILL for the tree watcher — A3's coverage made mechanism-independent.**
    ///
    /// `on-write.sh` watches the `Write` tool and `deny-bypass.sh` watches `mkdir`. Both enumerate
    /// *ways a file can appear*, which is unbounded: `cp`, `mv`, `tar -x`, `curl -o`, `>`, `unzip`,
    /// a script three levels down. `post-tree-watch.sh` watches the **tree** instead, so it cannot be
    /// routed around by choosing a different tool — it does not care which tool ran, only what is now
    /// on disk.
    ///
    /// ★ Run against a THROWAWAY repo. Creating a stray primary source in the real tree would race
    /// `archive_check::this_repo_has_no_unaccounted_primary_source`, and a test that can red an
    /// unrelated test is not a test.
    #[cfg(unix)]
    #[test]
    fn the_tree_watcher_catches_a_primary_source_however_it_arrives() {
        let script = repo_root().join("scripts/hooks/post-tree-watch.sh");
        assert!(script.exists(), "{} missing", script.display());
        let xtask = repo_root().join("target/debug/xtask");
        if !xtask.is_file() {
            return; // no debug binary in this profile; the live gate still covers the repo
        }

        let repo = tempfile::tempdir().expect("tempdir");
        let state = tempfile::tempdir().expect("tempdir");
        let r = repo.path();
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(r)
                .status()
                .is_ok_and(|s| s.success()),
            "git init"
        );

        let watch = || -> Option<i32> {
            Command::new(&script)
                .current_dir(r)
                .env("BTCTAX_HARNESS_STATE", state.path())
                .env("BTCTAX_XTASK", &xtask)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .ok()?
                .code()
        };

        assert_eq!(watch(), Some(0), "cold start seeds the cache silently");
        assert_eq!(watch(), Some(0), "no change is silent");

        // ★ THE CASE THE WRITE HOOK CANNOT SEE: a file that simply appears on disk.
        fs::create_dir_all(r.join("zz-stray")).expect("mkdir");
        fs::write(r.join("zz-stray/f6251--2025.pdf"), "x").expect("write");
        assert_eq!(
            watch(),
            Some(2),
            "a form PDF appearing in a new tree must be CAUGHT"
        );
        assert_eq!(
            watch(),
            Some(0),
            "reported ONCE, not on every subsequent call"
        );

        // The statute too — the rung that is law.
        fs::write(r.join("zz-stray/26USC_s61.html"), "x").expect("write");
        assert_eq!(
            watch(),
            Some(2),
            "a statute appearing in a new tree must be CAUGHT"
        );

        // Must stay silent on ordinary files, or it becomes noise and gets muted.
        fs::write(r.join("zz-stray/notes.md"), "x").expect("write");
        fs::write(r.join("zz-stray/Cargo.toml"), "x").expect("write");
        assert_eq!(watch(), Some(0), "ordinary files must not fire");

        // And silent when the file lands in an accounted-for tree.
        fs::create_dir_all(r.join("design/forms/2026")).expect("mkdir");
        fs::write(r.join("design/forms/2026/f6251--2026.pdf"), "x").expect("write");
        assert_eq!(watch(), Some(0), "an accounted-for tree must not fire");
    }

    /// ★★ **A1's principle, extended to the Claude-side hooks.** A2's bypass deny and A3/A4's Write
    /// hook are only real if `.claude/settings.json` actually references them and they are
    /// executable — otherwise they are `scripts/pre-push` all over again: reviewed, committed, never
    /// run. This is the same "written but not wired" failure at a different layer.
    #[test]
    fn the_claude_hooks_are_wired_and_executable() {
        let root = repo_root();
        let settings_path = root.join(".claude/settings.json");
        let settings = fs::read_to_string(&settings_path).unwrap_or_else(|e| {
            panic!(
                "{} missing ({e}) — the PreToolUse hooks are not wired",
                settings_path.display()
            )
        });
        for script in [
            "scripts/hooks/deny-bypass.sh",
            "scripts/hooks/on-write.sh",
            "scripts/hooks/post-tree-watch.sh",
        ] {
            assert!(
                settings.contains(script),
                ".claude/settings.json does not reference `{script}` — the hook exists but nothing \
                 invokes it, which is exactly the scripts/pre-push failure this harness was built for"
            );
            let p = root.join(script);
            assert!(
                p.exists(),
                "{script} is referenced by settings.json but does not exist"
            );
            assert!(
                is_executable(&p),
                "{script} is not executable — it would never run"
            );
        }
        // ★★ btctax-only, per the 2026-07-30 user constraint: the wiring must be THIS repo's
        // settings file. A global path here would apply this repo's failure data to every project.
        assert!(
            settings.contains("$CLAUDE_PROJECT_DIR"),
            "hook commands must be project-relative ($CLAUDE_PROJECT_DIR), never absolute or global"
        );
    }

    /// ★★ **THE LIVE GATE.** This working tree must actually have the hooks wired.
    ///
    /// This is the test that reds until someone runs the install command. It is *supposed* to fail on
    /// a fresh clone — that is the whole mechanism — and it is skipped only where the question is
    /// meaningless (see [`should_check`]).
    #[test]
    fn this_working_tree_has_the_harness_installed() {
        let root = repo_root();
        let hooks = hooks_dir(&root);
        if !should_check(hooks.as_ref()) {
            return;
        }
        let findings = audit(hooks.as_ref().expect("should_check requires Some"));
        assert!(findings.is_empty(), "{}", report(&findings));
    }
}
