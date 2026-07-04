# R0 — SPEC_cross_platform_ci.md — round 1

**Artifact:** `design/SPEC_cross_platform_ci.md` (DRAFT)
**Baseline:** branch `feat/cross-platform-ci` @ `07faf73` (main == `95f1ebc`)
**Reviewer role:** independent architect, read-only. **Bar:** 0 Critical / 0 Important.
**Scope of R0:** find what will make the Windows/macOS matrix FAIL that the spec has not accounted
for — verified against current source.

## Verdict: **0 Critical / 0 Important / 2 Minor / 3 Nit → R0-GREEN**

The plan is sound. Static analysis finds **no test or build construct that WILL fail the
Windows/macOS matrix** beyond what the spec already lists as de-risked or discoverable-on-CI.
Every `std::os::unix` use in the test suite is `#[cfg(unix)]`-gated; the store's OS primitives are
genuinely cross-platform *and* actually execute (not just compile) on the new legs; the native-dep
graph is fully resolved in `Cargo.lock`; `.gitattributes` is correct and will not corrupt the
committed binaries. The Minor/Nit items are de-risk completeness gaps and one watch-item, not
blockers. **No finding gates implementation.**

---

## What I verified clean (evidence)

**Every `std::os::unix` in tests is cfg-gated.** All 24 hits are behind `#[cfg(unix)]` (either an
attribute on the `#[test]` fn or an inner `#[cfg(unix)] { … }` block), so the test *binaries*
compile on Windows:
- `crates/btctax-cli/tests/export.rs:566` → gated by `#[cfg(unix)]` at :563
- `crates/btctax-store/tests/integration.rs:138/163/247` → inner `#[cfg(unix)]` blocks / fn gates
- `crates/btctax-tui/src/export.rs:454` → `#[cfg(unix)]` at :451
- `crates/btctax-tui-edit/src/edit/persist.rs` (7 hits: 2415/2547/2890/3721/3925/4095/4336) — every
  one is inside a `#[cfg(unix)] #[test]` fn
- `crates/btctax-tui-edit/src/main.rs` (8 hits: 9441/9949/11652/13195/14531/16877/18203/18900) —
  every one inside a `#[cfg(unix)] #[test]` fn

**No shell-outs to Unix tools, no build.rs, no Unix-only crates.** Only `Command::new` targets are
`env!("CARGO_BIN_EXE_btctax")` (portable, `.exe`-aware), `git` (`repo_hygiene.rs` — see M-2), and
`groff` (`xtask/src/docs.rs:75`, only in the `--pdf` path, never a test). No `build.rs` anywhere.
Dev-deps are all portable (`tempfile`, `rust_xlsxwriter`, `rust_decimal_macros`, `time`, `proptest`,
`serde_json`); no `nix`/`libc`/`xattr`.

**Store OS primitives execute on the Windows leg (spec GOAL met, not just compile-checked):**
- `memlock` VirtualLock runs via the **ungated** test `exposes_bytes_never_errors`
  (`crates/btctax-store/src/memlock.rs`, `SecretBuf::new` → `#[cfg(windows)] try_mlock` → `VirtualLock`),
  and again through `Vault::create`/`save`. Note it asserts nothing about lock *success*, so a
  `VirtualLock` working-set failure would still pass (returns `false`, prints a warning, no panic).
- `fs2` LockFileEx runs via `lock.rs::second_acquire_refused` (contention → `ERROR_LOCK_VIOLATION(33)`
  → `WouldBlock` → `StoreError::Locked`, mapping documented at `lock.rs:19-20`).
- atomic rename-replace runs via `atomic.rs` tests (`write_keeps_prev_in_bak_and_target_never_absent`).
- `open_owner_only` non-unix branch runs via `atomic_write`.
- `atomic.rs:27-28` directory-fsync is `let _ = File::open(dir).and_then(|d| d.sync_all())` — the
  ignored `Result` is exactly what makes it Windows-safe (`File::open` on a directory errors on
  Windows without `FILE_FLAG_BACKUP_SEMANTICS`; here it's a silent no-op, not a failure).

**Native-dep / `--locked` graph is complete.** `windows-sys`, `winapi`, `windows-targets`,
`windows_x86_64_msvc`, `rustix`, `libsqlite3-sys` are all present in `Cargo.lock` (version 4), so
`cargo test --workspace --locked` resolves the full platform graph on Windows/macOS without a lock
update. sequoia is `default-features=false` + `crypto-rust`; **no** `nettle-sys` / `bzip2-sys` /
`openssl-sys` in the lock → no system-crypto/compression C lib. (Confirms spec lines 31–39.)

**Path/TZ/exit-code hazards absent.**
- The two `/tmp` literals are inert: `crates/btctax-tui/src/export.rs:232` feeds the pure
  `export_dir_for` (suffix `.ends_with` check, no FS touch); `crates/btctax-tui/src/tabs/tests.rs:697`
  is a substring render check (`buffer_has(&buf, "test-vault.pgp")`). `PathBuf::from("/test/…")` in
  tui-edit tests are labels that are never opened/canonicalized.
- No `.close().unwrap()` on a `TempDir` (Windows open-file-delete panic — none exists).
- CLI exit-code tests use `.code().expect("… not via signal")`; on Windows `code()` is always `Some`,
  so `.expect()` never trips. No signal-number (`130`/`137`) asserts.
- All timestamps are `OffsetDateTime::now_utc()` — zero `now_local`/local-offset dependence.
- The source-walk gates (`btctax-tui/src/export.rs:692 e10_mechanized_source_gate`,
  `btctax-tui-edit/src/edit/persist.rs:1640`) use `BufRead::lines()` (strips trailing `\r`) +
  `.contains()` token matching + `PathBuf::join` — no separator or line-ending sensitivity.

---

## Findings

### [M-1] MINOR — spec "De-risked" omits that the build compiles vendored C (bundled SQLite)
`rusqlite` is pinned with `features = ["bundled", …]` in all three crates that use it:
`crates/btctax-store/Cargo.toml:10`, `crates/btctax-core/Cargo.toml:16`,
`crates/btctax-cli/Cargo.toml:22`. `bundled` compiles `libsqlite3-sys`'s vendored SQLite C source via
the `cc` crate, so **`cargo build` needs a C compiler on every leg**. This is *not* a WILL-FAIL on the
runners the spec targets — `windows-latest` ships MSVC (`cl.exe`) and `macos-latest` ships clang — so
the matrix stays green. But the spec's "De-risked" section (lines 30–39) lists only pure-Rust crypto
and implies a pure-Rust build; it never states the C-toolchain requirement.

**Fix:** add one line to the De-risked section: "rusqlite is `bundled` → compiles vendored SQLite C
via `cc`; satisfied by the pre-installed MSVC/clang on hosted `windows-latest`/`macos-latest`. (A
container / self-hosted runner without a C compiler would break `cargo build` — hosted runners only.)"

### [M-2] MINOR — `repo_hygiene.rs` asserts Unix mode `100755` un-gated + fail-closed; the top watch-item
`crates/btctax-cli/tests/repo_hygiene.rs:22` (`hook_scripts_are_tracked_executable_100755`) runs
`git ls-files -s scripts/pre-push scripts/pii-scan-generic.sh` and asserts each line
`starts_with("100755")` — with **no `#[cfg(unix)]` gate and a deliberate NO-skip, fail-closed** design
(:7-11, :38). This is the single most Windows-hostile-*looking* test in the suite (it asserts a Unix
executable-bit mode and cannot be skipped). The spec's de-risk of this file (line 38) covers only
"shells to `git` (pre-installed)" and does **not** address the mode-bit assertion.

**Assessment:** I expect it to **PASS** on Windows. `git ls-files -s` reads the mode from the *index*,
which is populated from the committed *tree* object (`100755`); `core.filemode=false` (git-for-Windows
default) prevents git from rewriting that mode from the NTFS filesystem, so the index retains `100755`.
The mode is a property of the git object, not the checkout OS. It already reads `100755` on Linux
(the repo is green), and the same tree yields `100755` on Windows.

**Fix (spec):** add this to the de-risk note so the first Windows run isn't a surprise, with the
explicit contingency: "if `git ls-files -s` unexpectedly reports `100644` on the Windows leg (mode
not preserved), the in-scope fix is a `#[cfg(unix)]` gate on this test — its regression target
(fail-open executable bit) is Unix-semantics-only." No code change needed pre-push; just name it.

### [N-1] NIT — `.gitattributes` correctness confirmed; rationale is imprecise; optional binary marker
`* text=auto eol=lf` is **correct and safe** (answers spec Q2):
- The only committed binaries are 25 IRS/FedReg **PDFs under `legal/`**, all NUL-bearing →
  `text=auto` auto-detects them as binary and leaves them untouched. No corruption. (`legal/text/*.txt`
  and `legal/_provenance/fetch_log.tsv` are genuine text and are not read by any test.)
- **Zero** tracked text files contain real CR bytes (`git grep -lIP '\r'` → none), so **no
  intentional-CRLF fixture is normalized away.**
- The adapter CSV fixtures' `\r\n` (e.g. `crates/btctax-adapters/tests/coinbase.rs:11+`,
  `river.rs:9`, and many CLI fixtures) are **Rust escape sequences in `.rs` source** (bytes `\ r \ n`),
  *not* CR bytes — `eol=lf` does not touch them.

The spec's rationale ("checkout would convert fixtures to CRLF, breaking the adapter CSV-fixture
parsers", line 22-23) misidentifies the exposure. The real CRLF exposure is **inline multi-line
string literals** in snapshot/output-compare tests, the `include_str!` price dataset
(`crates/btctax-adapters/src/price.rs:10` → `../data/btc_usd_daily_close.csv`, currently LF), and the
**xtask byte-compare KAT** `gen_docs_is_deterministic` (`crates/xtask/src/docs.rs:341-356`, `std::fs::read`
of committed `.1` man pages vs in-memory LF roff — omitted from the spec's affected-test list). The
universal `*` glob covers all of these, so the **decision is right**; only the enumerated rationale is
under-inclusive. Optional hardening: add `*.pdf binary` (belt-and-suspenders; not required since NUL
auto-detection already protects them).

### [N-2] NIT — resolve the rust-cache open question: no per-OS key needed (but harmless)
Spec line 27-28 leaves open whether `Swatinem/rust-cache` keys on OS by default. It does: rust-cache v2
folds `rustc -vV` — whose `host:` triple differs per leg (`x86_64-unknown-linux-gnu` /
`x86_64-pc-windows-msvc` / `aarch64-apple-darwin`) — into the cache key, so the three legs **cannot
collide** even with the shared job name `test`. The planned `key: ${{ matrix.os }}` is fine and
harmless (belt-and-suspenders), just not strictly required. `fail-fast: false` is correct (independent
per-OS validation; you want all three results).

### [N-3] NIT — (positive) the spec's core GOAL is actually achieved, worth stating in the spec
The spec frames the cfg(windows) paths as "never executed" and the matrix as the fix. Confirmed the
matrix *does* execute them (memlock VirtualLock, fs2 LockFileEx, atomic rename, open_owner_only — see
"verified clean" above), so the deliverable genuinely proves-they-run rather than merely compile-checks.
Suggest adding one sentence to the Goal so the success criterion is explicit: "these primitives are
exercised by existing ungated tests (`memlock::exposes_bytes_never_errors`, `lock::second_acquire_refused`,
`atomic::*`), so a green Windows leg proves execution, not just compilation."

---

## Bottom line
The spec's plan produces a **green matrix** on the targeted hosted runners as far as static analysis
can determine — no un-accounted WILL-FAIL-on-windows/macos test or build construct exists. The one
test that *looks* like a Windows landmine (`repo_hygiene.rs` `100755`, M-2) is assessed to pass because
it reads the git index mode, not the filesystem; it is named as the top watch-item with a one-line
contingency fix. **0 Critical / 0 Important → R0-GREEN.** Fold M-1/M-2/N-1/N-2/N-3 into the spec
(de-risk completeness + resolved open questions), then proceed to plan.
