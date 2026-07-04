# Whole-diff review (Phase E) — feat/cross-platform-ci — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review. Diff `main (95f1ebc)..18b21b5`. Contract: `design/SPEC_cross_platform_ci.md`
(R0-GREEN). 8 files, +331/−7. **The deliverable is a green matrix RUN — achieved: run 28707743830 is
all-green on `test (ubuntu-latest)`, `test (macos-latest)`, AND `test (windows-latest)`, plus the Linux-only
clippy/fmt/msrv/pii-scan.**

## The CI change (the spec's goal)
- **`ci.yml`** — `test` job matrixed over `[ubuntu, macos, windows]-latest`, `fail-fast: false`, running the
  unchanged `cargo test --workspace --locked`; other jobs stay Linux-only. Validated by the green run itself
  (`actionlint` clean; the three `test (<os>)` legs are the required checks the user sets in branch protection).
- **`.gitattributes`** — `* text=auto eol=lf`; `git add --renormalize .` produced 0 content churn (tree is
  already LF); tracked PDFs auto-detected binary. Prevented Windows CRLF-breakage of the adapter/snapshot tests.

## The three real bugs the matrix caught (invisible on any single dev machine — the whole point)
1. **[gitignore] `docs/man/btctax-export-snapshot.1` was silently un-committed** — `.gitignore`'s
   `*-snapshot.*` (guarding decrypted vault exports, NFR2) over-matched the man page, so the xtask docs KATs
   failed on any clean checkout (passed locally only because the file existed untracked). Fix: `!docs/man/*.1`
   negation + commit the page. Verified it's the ONLY wrongly-ignored file (`find … | comm` vs `git ls-files`).
   Man pages are generated from the CLI structure → no user data → safe to un-ignore.
2. **[★ stack overflow] `btctax` crashed on Windows** (`STATUS_STACK_OVERFLOW`, 0xC00000FD, empty stderr) in
   the classify-inbound-self-transfer flow — Windows' 1 MiB main-thread stack vs Linux/macOS 8 MiB, exceeded
   by large fold stack frames (a 2-event vault overflows → frame size, not recursion depth; no unbounded
   self-recursion found in the engine). Fix: run the CLI on a 64 MiB worker thread (the rustc/cargo
   RUST_MIN_STACK approach). **Root-caused + reproduced on Linux** (`ulimit -s 256` on the subprocess main
   thread + `RUST_MIN_STACK=8M` on the harness thread → pre-fix 3/3 FAIL, post-fix 3/3 pass). `main.rs`:
   `worker.join().unwrap_or(ExitCode::from(2))` preserves the exit-code contract; a worker panic surfaces
   exit 2 (its default hook already printed). Full Linux suite 1095/0 — no regression.
3. **[★ lock semantics] Windows lock-violation not recognized as contention** —
   `lock::tests::second_acquire_refused` failed because `acquire()` mapped contention to `StoreError::Locked`
   only via `ErrorKind::WouldBlock`, but `LockFileEx(LOCKFILE_FAIL_IMMEDIATELY)` refuses the 2nd lock with
   `ERROR_LOCK_VIOLATION (33)`, which current stable std does NOT normalize to `WouldBlock` (the prior code
   comment's assumption — disproven by the runner). Fix: `is_contention()` also matches raw codes 32/33 under
   `#[cfg(windows)]` (they mean unrelated errors on Unix, hence the gate). Linux path unchanged; **Windows
   leg now green**. The test prints the actual `Result` on failure — any future lock-semantics surprise is visible.

## Scrutiny of the 2 product-code changes
- **`main.rs`** — behavior-preserving for the normal path (same match on `run()`'s result, now on a worker
  thread). `Cli::parse()` still `process::exit`s on bad args (terminates all threads — fine). No new dep.
- **`lock.rs`** — the `cfg(windows)` raw-code branch is correctly gated (`raw_os_error()` exists on all
  platforms; the branch is absent on Unix so no cross-platform mismap). `#[derive(Debug)]` on `VaultLock` is
  additive (for the diagnostic message). Linux `WouldBlock` path unchanged.

## SemVer
Docs/CI + two localized cross-platform correctness fixes; no API surface change beyond additive
`VaultLock: Debug`. PATCH-class. No crate version bump.

**SHIP.**
