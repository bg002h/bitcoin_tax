# REPORT — S3 — FR-147

**Finding:** FR-147 (rehearsal F13) — `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
cannot pass under the `CARGO_TARGET_DIR` override the review workflow mandates.

**File owned/touched:** `crates/xtask/src/harness_check.rs` only. No other file was touched.

## 1. Mechanism, as measured

The FOLLOWUPS/brief text names the helper `ensure_xtask` and `design/HARNESS.md`'s rehearsal report
names it `ensure_xtask_binary` — the source has only the latter (`crates/xtask/src/harness_check.rs`,
in `mod tests`). This is a naming slip in the entry text, not a refutation of the mechanism: the
mechanism itself is exactly as F13 describes.

`ensure_xtask_binary()` (pre-fix):

```rust
fn ensure_xtask_binary() {
    let root = repo_root();
    if root.join("target/debug/xtask").exists() || root.join("target/release/xtask").exists() {
        return;
    }
    let st = Command::new(env!("CARGO"))
        .args(["build", "-p", "xtask"])
        .current_dir(&root)
        .status()
        .expect("spawn cargo build -p xtask");
    assert!(st.success(), "could not build xtask, which this test's ALLOW cases require");
    assert!(
        root.join("target/debug/xtask").exists() || root.join("target/release/xtask").exists(),
        "cargo build -p xtask succeeded but left no binary where on-write.sh looks — ..."
    );
}
```

`repo_root()` is fixed via `CARGO_MANIFEST_DIR`'s grandparent, so the two `exists()` checks always mean
the literal `<repo>/target/{debug,release}/xtask` — the same hardcoded path `scripts/hooks/on-write.sh`
itself looks at (`XTASK="$ROOT/target/debug/xtask"`, no override — verified by reading that script; it
is out of this agent's file ownership and was not touched). The `cargo build -p xtask` spawn does **not**
clear `CARGO_TARGET_DIR`, so under the review workflow's mandated override it inherits the ambient value
and builds into `$CARGO_TARGET_DIR/debug/xtask` instead — a location the two `exists()` checks never
look at. The build genuinely succeeds; the assertion right after it fails because it is checking the
wrong directory for a binary that is sitting one directory over.

**Panic, pasted, from a real run** (`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-s3`, clean
default `target/`, pre-fix code):

```
thread 'harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory' (1542646) panicked at crates/xtask/src/harness_check.rs:665:9:
cargo build -p xtask succeeded but left no binary where on-write.sh looks — if the target dir moved, the HOOK's lookup needs updating too, not just this test
```

No refutation: F13's description matches the measured mechanism exactly (module for the helper's exact
name, noted above).

## 2. What changed, and where

One line added to `ensure_xtask_binary()` in `crates/xtask/src/harness_check.rs`, plus a doc-comment
explaining why:

```rust
let st = Command::new(env!("CARGO"))
    .args(["build", "-p", "xtask"])
    .current_dir(&root)
    .env_remove("CARGO_TARGET_DIR")   // <-- added
    .status()
    .expect("spawn cargo build -p xtask");
```

This clears `CARGO_TARGET_DIR` for only this one child `cargo build` invocation, so it always lands the
binary at `<repo>/target/debug/xtask` — the same fixed path `on-write.sh` reads, regardless of what the
ambient `CARGO_TARGET_DIR` is set to in the parent test process. `on-write.sh` was read but not touched:
it is not this agent's file, and it does not need to change — the fix makes the test's own build honor
the same fixed path the hook already hardcodes.

## 3. The three demonstrations

**(a) Failing before, under the override** (clean `target/`, unfixed code,
`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-s3`):

```
thread 'harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory' (1542646) panicked at crates/xtask/src/harness_check.rs:665:9:
cargo build -p xtask succeeded but left no binary where on-write.sh looks — if the target dir moved, the HOOK's lookup needs updating too, not just this test
```
Full-suite summary in that same run: `186 tests run: 179 passed, 7 failed, 1 skipped` (6 `form_delta` +
this one).

**(b) Passing after, under the same override** (clean `target/`, fixed code, same
`CARGO_TARGET_DIR`):

```
PASS [  18.730s] (186/186) xtask::bin/xtask harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory
Summary [  18.905s] 186 tests run: 180 passed, 6 failed, 1 skipped
```
The 6 remaining failures are all `form_delta::tests::*`, e.g.:
```
thread 'form_delta::tests::the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals' panicked at crates/xtask/src/form_delta.rs:1075:14:
the TY2026 draft is archived and its geometry extracted: "no PDF found for f6251--2026-DRAFT"
```
Confirmed environmental: `design/forms/**/*.pdf` is gitignored (`.gitignore:63`) and this isolated
worktree holds zero PDFs under `design/forms/2026/`.

**(c) Still passing without the override** (clean `target/`, fixed code, `CARGO_TARGET_DIR` unset):

```
Starting 11 tests across 1 binary (176 tests skipped)
PASS [...] harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory
Summary [  29.549s] 11 tests run: 11 passed, 176 skipped
```
All 11 `harness_check` tests pass, default path unaffected.

## 4. Skip? No.

No test was skipped, marked `#[ignore]`, or given an early return under an env-var branch. The fix makes
the test's own precondition-building step target the same fixed path the hook reads, in both the
overridden and default cases — the test runs its full body and asserts for real in both.

## 5. Gate numbers

| state | full `xtask` suite (`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-s3`, clean `target/`) |
|---|---|
| before (this agent's fix) | **7 failed** (186 run, 179 passed, 7 failed, 1 skipped) — `harness_check::…the_write_hook_denies_new_archives_and_asks_once_per_new_directory` + 6 `form_delta::*` |
| after (this agent's fix) | **6 failed** (186 run, 180 passed, 6 failed, 1 skipped) — all `form_delta::*`, all traced to the gitignored `design/forms/**/*.pdf` corpus this isolated worktree does not hold |

`cargo fmt --package xtask -- --check` and `cargo clippy -p xtask --all-targets -- -D warnings` both
exit 0 on the fixed file (checked under the `CARGO_TARGET_DIR` override).

## 6. Refuted premises

None on the mechanism. One naming imprecision in the entry text: the brief says the helper is called
`ensure_xtask`; the actual name in source is `ensure_xtask_binary`. Purely cosmetic — the described
mechanism (inherits `CARGO_TARGET_DIR`, asserts the default path) is exactly what was found and fixed.

## 7. Diff scope

Only `crates/xtask/src/harness_check.rs` was touched (verified via `git status --porcelain` /
`git diff`): one functional line (`.env_remove("CARGO_TARGET_DIR")`) plus its explanatory doc comment.
No other file in the repo was modified. No commit was made (per the plan, the coordinator commits and
marks `FOLLOWUPS.md` at integration).
