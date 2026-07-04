# SPEC — Windows + macOS CI (validate the cfg-gated OS code paths)

**Source baseline:** `main` @ `95f1ebc` (branch `feat/cross-platform-ci`). **Review status: DRAFT — awaiting R0.**
**Lineage:** FOLLOWUPS #3 (open follow-up, user-picked 2026-07-04). The store's tri-OS code (file locks,
mlock/VirtualLock, atomic rename, owner-only perms) is `cfg`-gated + compile-checked on Linux but **never
executed** on Windows/macOS. Decisions (user 2026-07-04): the new OS jobs are **required** (block merge — user
sets branch protection); validation = **I push `feat/cross-platform-ci` + watch the matrix via `gh`, iterate
to green, then merge**.

## Goal
Extend `.github/workflows/ci.yml` so the workspace is **built and tested on ubuntu + macos + windows**, proving
the `cfg`-gated OS primitives actually run. Add the line-ending normalization needed for Windows.

## Design
1. **Matrix the `test` job** over `os: [ubuntu-latest, macos-latest, windows-latest]`, `runs-on: ${{ matrix.os }}`,
   `strategy.fail-fast: false` (see all three results, not just the first failure). Each leg runs the existing
   `cargo test --workspace --locked`. Job legs render as `test (ubuntu-latest)` / `(macos-latest)` /
   `(windows-latest)` — the three the user marks required in branch protection.
2. **Keep `clippy` / `fmt` / `msrv` / `pii-scan` Linux-only** — host-independent (lint/format/MSRV-check are
   the same on every OS; `pii-scan` is a bash script). Matrixing them = 3× CI cost for zero added signal.
3. **[★ REQUIRED] add `.gitattributes` = `* text=auto eol=lf`.** No `.gitattributes` exists; on Windows the
   checkout would convert fixtures + sources to CRLF, breaking the adapter CSV-fixture parsers and the exact
   text/snapshot tests (btctax-adapters `tests/*.rs`, ~45 tui-edit snapshot asserts, CLI output compares).
   Forcing LF on checkout everywhere prevents spurious Windows failures. (On the Linux tree everything is
   already LF → no renormalization diff.)
4. **Pin actions** — reuse the existing pinned SHAs (checkout v4, dtolnay/rust-toolchain, Swatinem/rust-cache);
   give `rust-cache` a per-OS key so the three legs don't collide (it keys on the runner OS by default —
   confirm, else set `key: ${{ matrix.os }}`).

## De-risked (already cross-platform — verified in source at baseline)
- **Crypto backend = `crypto-rust`** (pure Rust, no system lib): `btctax-store/Cargo.toml:9`
  (`sequoia-openpgp default-features=false, features=["crypto-rust", …]`) → builds on all three OSes.
- **`fsperms.rs`** has a `#[cfg(not(unix))]` branch for every fn (`open_owner_only` / `restrict_file_to_owner`
  / `mkdir_owner_only`, lines 32/63/82); **`memlock.rs`** has `#[cfg(unix)]`+`#[cfg(windows)]` (29/38/58/64).
- **`groff` is only in the `--pdf` path** (`xtask/src/docs.rs:75`), NOT in any test — `cargo test -p xtask`
  passes without it (5 roff-only KATs). The CI matrix runs `cargo test`, NOT `make docs`, so no groff needed.
- **CLI integration tests** resolve the binary via `env!("CARGO_BIN_EXE_btctax")` (`.exe`-aware, portable);
  **`repo_hygiene.rs`** shells to `git` (pre-installed on all GitHub runners).
- **`--locked`**: `Cargo.lock` resolves the full platform graph (windows-sys / rustix already present).

## Cross-platform unknowns — discoverable ONLY on the CI run (the reason we push + watch)
- Any test asserting a Unix path separator, `/tmp`, or a mode bit NOT behind `cfg(unix)`.
- Windows file-locking / atomic-rename timing (the store's `fs2` + rename-replace paths).
- macOS-specific (ARM `macos-latest`) surprises.
- Residual line-ending assumptions the `.gitattributes` doesn't cover (e.g. a test that hard-codes `\r\n`).
These are why the deliverable is a GREEN matrix RUN, not just the YAML.

## Validation (the "test" for this artifact)
Push `feat/cross-platform-ci` → GitHub hosted macos/windows/ubuntu runners execute the matrix → watch via
`gh run watch` / `gh run view` → for each failure, fix (cfg-gate a Unix assumption, normalize an ending, adjust
a test) on the branch, push, re-watch → repeat until all three legs GREEN. Only then merge.

## Scope / SemVer
CI + `.gitattributes` only. **No production code change** unless a genuine cross-platform test defect surfaces
on the matrix — if so, the fix is in scope (cfg-gate / normalize) and gets called out in the whole-diff. No
crate version bump.

## Plan
- **Task 1** — add `.gitattributes` (`* text=auto eol=lf`); verify `git add --renormalize .` yields no content
  churn on the Linux tree (confirms everything is already LF).
- **Task 2** — matrix the `test` job (fail-fast:false; keep the other jobs Linux); pin/rust-cache key per OS.
- **Task 3 (validation loop)** — push the branch; watch the matrix; iterate to 3-green; then whole-diff review
  + merge. Note the user sets the three `test (<os>)` legs as required checks in branch protection.

## Gotchas
- **`.gitattributes` is load-bearing for Windows** — without it the matrix fails on line endings, masking the
  real signal. Land it in the SAME change as the matrix.
- **`fail-fast: false`** — else a Windows failure cancels the in-progress macOS leg and hides its result.
- **Don't matrix clippy/fmt/msrv/pii-scan** — no OS signal; wasteful + `pii-scan` is bash (would fail on Windows).
- **The deliverable is a green RUN** — the YAML compiling is not proof; the hosted runners are.
