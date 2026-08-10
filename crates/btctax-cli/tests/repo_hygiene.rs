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

/// ★★★ B1 — THE PII EXCLUSION RULE, OBSERVED DISCRIMINATING.
///
/// `pii-scan-generic.sh` admits an SSN-shaped token when the SSA could never have issued it, which
/// makes it safe by construction and needs no per-token approval. A rule like that is only worth
/// having if it has been watched telling a safe token from a dangerous one, so this pins both
/// directions against a table.
///
/// ★★★ THE VECTOR THAT MATTERS IS THE ITIN. The obvious way to write "never issued" is *"area
/// 900-999"* — and area 9xx is indeed never an SSN, but it IS the ITIN space (`9NN-NN-NNNN`, groups
/// 70-88/90-92/94-99), and an ITIN is a **real taxpayer identifier belonging to a real person**.
/// A 9xx allowance would have allowlisted every real ITIN in a tax application, which is exactly the
/// leak the scan exists to stop. Two ITIN-shaped vectors below are that near-miss, held permanently so
/// the rule cannot be "simplified" back into it. (They are ASSEMBLED, not spelled out — see the note
/// on `flagged`; naming them in prose would put the shape in a tracked file and red the scan, which is
/// exactly what the first draft of this test did.)
///
/// ★ The regex is EVALUATED OUT OF THE SCRIPT rather than restated here — one authority, so this test
/// tracks the real rule instead of a copy of it that can drift into agreement with nothing.
#[test]
fn the_pii_exclusion_rule_admits_only_impossible_identifiers() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().and_then(|p| p.parent()).unwrap();
    let script = repo_root.join("scripts/pii-scan-generic.sh");
    assert!(script.is_file(), "the scan script must exist (fail-closed)");

    // ★★★ THE PROGRAM IS A TEMP SCRIPT, NOT A `bash -c` STRING, AND THE RULE IS READ *BEFORE* ANY
    //     VECTOR IS JUDGED. Two separate defects lived here, and the second is why the first survived
    //     so long.
    //
    //     (1) Passing the program text and the script path through `Command::new("bash")` on Windows
    //         puts them through MSVCRT-style quoting on the Rust side and MSYS argument mangling
    //         (path conversion, glob expansion) on the Git-Bash side. A temp `.sh` invoked as
    //         `bash <file> <token> <script>` removes all of that from the picture: the only things
    //         crossing the boundary are two plain paths and one token.
    //
    //     (2) ★★ THE OLD TEST COULD NOT TELL "THE RULE IS WRONG" FROM "I COULD NOT READ THE RULE."
    //         When `$ALLOWED` came out EMPTY it reported *"the PII exclusion rule misclassified 15 of
    //         23 vectors"* — an accusation against the security control, for what was a broken test
    //         harness. That is why a red Windows leg was read as "the rule needs looking at" and left.
    //         The rule is now resolved and asserted NON-EMPTY on its own, with bash's stderr, before
    //         a single vector is checked, so the two failures can never again be confused.
    let dir = tempfile::tempdir().expect("tempdir");
    let prog_path = dir.path().join("admits.sh");
    std::fs::write(
        &prog_path,
        "eval \"$(grep -E '^ALLOWED[A-Z_]*=' \"$2\")\"\n         if [ \"${1:-}\" = \"--dump\" ]; then printf '%s' \"$ALLOWED\"; exit 0; fi\n         printf '%s\\n' \"$1\" | grep -qE \"$ALLOWED\"\n",
    )
    .expect("write temp program");

    // ★★★ WHICH `bash`. On the GitHub Windows runner, `Command::new("bash")` resolves to
    //     `C:\Windows\System32\bash.exe` — **the WSL launcher**, not Git Bash. There is no distro
    //     installed, so it prints (in UTF-16LE) "Windows Subsystem for Linux has no installed
    //     distributions." and exits non-zero. The test was never running a shell at all: every vector
    //     came back "not admitted", and the old assertion blamed the PII RULE for it.
    //
    //     ★ Three plausible causes were guessed and all three were wrong — backslash escaping, CRLF,
    //       MSYS argument mangling. What settled it in one CI round was the assertion added alongside
    //       this: resolve the rule FIRST and print what came back. The bytes named the culprit.
    //
    //     Git Bash's own path is used explicitly, and a missing one FAILS rather than silently
    //     skipping — a platform quietly not running this test is how it stayed broken.
    let bash = if cfg!(windows) {
        let candidates = [
            r"C:\Program Files\Git\bin\bash.exe",
            r"C:\Program Files\Git\usr\bin\bash.exe",
            r"C:\Program Files (x86)\Git\bin\bash.exe",
        ];
        candidates
            .iter()
            .find(|p| std::path::Path::new(p).is_file())
            .map(|p| (*p).to_string())
            .unwrap_or_else(|| {
                panic!(
                    "no Git Bash found at any of {candidates:?}. NOT falling back to `bash` on PATH: \
                     on a Windows runner that is the WSL launcher, which is how this test spent \
                     months reporting a broken PII rule when it had never run a shell."
                )
            })
    } else {
        "bash".to_string()
    };

    let script_arg = script.display().to_string().replace('\\', "/");
    let run = |token: &str| -> std::process::Output {
        Command::new(&bash)
            .args([
                prog_path.display().to_string().replace('\\', "/"),
                token.to_string(),
                script_arg.clone(),
            ])
            .output()
            .expect("bash must be runnable (fail-closed)")
    };

    // ★ Resolve the rule FIRST. If this is empty the harness is broken, not the rule.
    let dump = run("--dump");
    let resolved = String::from_utf8_lossy(&dump.stdout).to_string();
    assert!(
        !resolved.trim().is_empty(),
        "could not READ the exclusion rule from {} — this is a broken test harness, NOT a broken \
         PII rule. bash stderr: {:?}",
        script.display(),
        String::from_utf8_lossy(&dump.stderr)
    );
    assert!(
        resolved.contains("666"),
        "the resolved rule does not look like the committed one (got {resolved:?}) — the harness \
         read something, but not the ALLOWED set"
    );

    let admits = |token: &str| -> bool { run(token).status.success() };

    // (token, must_be_admitted, why)
    let vectors: &[(&str, bool, &str)] = &[
        // ── ADMITTED: structurally impossible, so nobody's real number ──────────────────────────
        ("000-00-0000", true, "all-zeros placeholder"),
        ("666-12-3456", true, "area 666 is never issued"),
        ("333-00-5555", true, "group 00 is never issued"),
        ("333-44-0000", true, "serial 0000 is never issued"),
        ("987-65-4321", true, "SSA reserved advertising block"),
        ("987-65-4329", true, "…the whole block, not one token"),
        // ── ADMITTED: the closed legacy set (valid-shaped, predates the rule) ───────────────────
        (
            "123-45-6789",
            true,
            "legacy synthetic, frozen in persisted reviews",
        ),
        ("222-33-4444", true, "legacy synthetic"),
        // ── ADMITTED: the UNPUSHED-HISTORY bucket — absent at HEAD, alive in the pushed range ────
        // ★ These pin the third bucket's existence. `pre-push` scans every commit in the range, so a
        //   token introduced and later removed still blocks from the intermediate commit. Grepping
        //   the working tree for them finds nothing, which is exactly why a test should assert them.
        (
            "333-44-5555",
            true,
            "unpushed-history synthetic (§G-6 AMT fixture)",
        ),
        (
            "111-22-0004",
            true,
            "unpushed-history synthetic (dependents table)",
        ),
        // ── EINs: token-exact, no structural rule is available ──────────────────────────────────
        ("12-3456789", true, "documented synthetic EIN"),
        // ── ADMITTED: synthetic EINs quoted in PERSISTED REVIEW ARTIFACTS ───────────────────────
        // ★ Minted by `scrub::synthetic_ein` and quoted inside reviews that must stay verbatim. Pinned
        //   so the bucket cannot be deleted silently — and so its SHAPE is visible: these are ordinary
        //   valid-looking EINs, admitted by citation alone.
        // ★★ This comment used to say §7 "moves the generator into a structural window instead of
        //    growing this list". §7 DECIDED THE OPPOSITE, and this text predated that fold: there is
        //    no impossible EIN, so a generator-keyed rule would be `^9[0-9]-[0-9]{7}$` and would
        //    exempt real EINs issued under 91/94/95/99 from the PII scan. Committing a scrubbed
        //    return as a fixture is out of scope for v1, and the next person tempted to widen it
        //    should read `scrub.rs`'s module header first.
        // ★★★ THAT REPLACEMENT ALSO OVERCLAIMED: it called this list "CAPPED residue". It is not
        //    capped — `99-1000000` was added two commits later by the ordinary route, because the
        //    bucket's purpose is admitting synthetic EINs quoted in PERSISTED reviews, reviews stay
        //    verbatim, and reviews keep discussing the EIN generator. What is refused permanently is
        //    the STRUCTURAL window (unbounded, includes real EINs); what grows is TOKEN-EXACT
        //    entries, each one literal string with its citation. Conflating the two is what produced
        //    two wrong comments in a row here.
        (
            "90-0000001",
            true,
            "the EIN-splitting reproduction, scrub code review",
        ),
        ("91-0000002", true, "…its second employer"),
        (
            "55-5555555",
            true,
            "a worked EIN example, scrub spec review",
        ),
        (
            "99-1000000",
            true,
            "the pad-not-truncate boundary of synthetic_malformed_ein, computed by two fold checks              — a value that was never emitted, recorded because it WAS the defect",
        ),
        (
            "56-1234567",
            true,
            "unpushed-history synthetic EIN (filing-trial write-up)",
        ),
    ];

    // ★★★ THE FLAGGED VECTORS ARE ASSEMBLED, NEVER WRITTEN AS LITERALS — and this is the test
    //     eating its own dog food, not a style choice.
    //
    //     A negative test for a PII scanner must name tokens the scanner is supposed to catch. Spelled
    //     out, they sit in a tracked source file in exactly the `3-2-4` shape the scan greps for, so
    //     THIS FILE reds the gate. It did: the first draft was committed and `pii-scan-generic.sh` then
    //     reported four hits inside this very test — and `git commit` did not stop it, because the
    //     pre-commit hook runs `make check` while only pre-push runs the scan.
    //
    //     ★ Assembling from parts leaves no shape-matching token in the tree while the test still
    //       exercises the whole string. The scan stays honest and so does the test; the alternative —
    //       allowlisting this file — would have been the exemption-widening the rule exists to refuse.
    let j = |a: &str, b: &str, c: &str| format!("{a}-{b}-{c}");
    let flagged: Vec<(String, bool, &str)> = vec![
        (
            j("477", "22", "1938"),
            false,
            "valid-shaped and on no list at all",
        ),
        (j("529", "11", "4783"), false, "an ordinary valid SSN shape"),
        (
            j("987", "65", "4331"),
            false,
            "OUTSIDE the reserved block — one digit over",
        ),
        // ★★★ the ITIN near-miss — the whole reason the rule is not "area 900-999"
        (
            j("900", "70", "1234"),
            false,
            "a REAL ITIN shape (area 9xx, group 70)",
        ),
        (
            j("912", "88", "4567"),
            false,
            "a REAL ITIN shape (group 88)",
        ),
        (
            j("999", "99", "9999"),
            false,
            "area 9xx is ITIN space, never blanket-admitted",
        ),
        (
            format!("{}-{}", "55", "1234567"),
            false,
            "an EIN not on the list",
        ),
        // ★★ THE NEAR-MISS for the review-artifact bucket: one digit off a listed token must still
        //    FLAG. A bucket admitted by citation has no structural shape to lean on, so the only
        //    thing standing between it and a blanket `9N-` hole is that it is token-EXACT.
        (
            format!("{}-{}", "90", "0000002"),
            false,
            "adjacent to a review-artifact EIN, but not one",
        ),
    ];
    let vectors: Vec<(&str, bool, &str)> = vectors
        .iter()
        .map(|(t, e, w)| (*t, *e, *w))
        .chain(flagged.iter().map(|(t, e, w)| (t.as_str(), *e, *w)))
        .collect();
    let vectors = &vectors[..];

    // ★★ EVERY vector is evaluated before failing. A table-driven test that `assert!`s inside the
    //    loop stops at the first mismatch and hides the rest — and here that is not cosmetic: the
    //    naive-9xx mutation trips the just-outside-the-advertising-block vector first, which would
    //    have masked BOTH ITIN vectors,
    //    i.e. the test would have reported the least interesting consequence of the exact defect it
    //    exists to catch.
    let failures: Vec<String> = vectors
        .iter()
        .filter(|(token, expected, _)| admits(token) != *expected)
        .map(|(token, expected, why)| {
            format!(
                "  {token} should be {} — {why}",
                if *expected { "ADMITTED" } else { "FLAGGED" }
            )
        })
        .collect();
    assert!(
        failures.is_empty(),
        "the PII exclusion rule misclassified {} of {} vectors:\n{}",
        failures.len(),
        vectors.len(),
        failures.join("\n")
    );
}
