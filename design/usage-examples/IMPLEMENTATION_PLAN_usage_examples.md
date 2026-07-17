# Usage-examples constellation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.
> Each phase closes with an **independent Fable review to 0 Critical / 0 Important**, persisted verbatim
> under `design/usage-examples/reviews/` before the fold; re-review after every fold. This plan is the
> substrate of `SPEC_usage_examples.md` (r1 GREEN) — read the spec's §§ referenced per task.

**Goal:** Build two CI-gated, groff-rendered btctax usage-example docs (CLI verbatim-I/O + a separate
style-aware TUI text-capture) from synthetic data, whose authoring surfaces UX/workflow bugs.

**Architecture:** A one-line CLI clock-injection seam (`BTCTAX_NOW`) makes decision-bearing stdout
deterministic (P0); a new `xtask examples` generator runs the built binary against synthetic vaults and
commits a whole-file golden gated by a `regen==committed` test (P1); a CI job re-diffs it and runs a
forms-coverage census (P2); a `TestBackend` journey driver captures style-aware TUI frames behind a shared
TUI clock helper (P3); an adversarial workaround-audit files discovered bugs (P4).

**Tech Stack:** Rust (workspace, MSRV 1.88), `clap`, `time` (RFC3339), `ratatui 0.29` `TestBackend`,
`xtask` + `groff -k -man -T pdf`, `cargo test`/`nextest`.

## Global Constraints (from the spec — every task inherits these)

- **The (i)/(ii)/(iii) determinism-prerequisite fence** (SPEC §3.1): a code change is allowed *only if*
  (i) seam-inactive ⇒ byte-identical behavior, (ii) it injects an input never transforms an output, (iii)
  tests pin the inactive-path equivalence. Anything else → FOLLOWUPS, never an inline edit. STOP ledger
  §12 (S1–S5) is in force.
- **Synthetic data only** in any committed/distributed artifact. Vaults are regenerated from committed
  CSVs, never committed (encrypted/gitignored).
- **Harness pins** (SPEC §3.3): `BTCTAX_PASSPHRASE=pw`, `BTCTAX_PRICE_CACHE`→nonexistent path, fixed cwd +
  relative `--vault`/`--out`, explicit `--at`/`--effective-from`, `TZ=UTC LC_ALL=C LANG=C`, scrubbed HOME.
- **Reviews use Fable**, author ≠ reviewer, loop to 0C/0I; persist verbatim before folding.
- **Determinism surfaces (SPEC §3.2):** clock-derived = `verify` election `recorded`, `bulk-void`
  preview, `config --set-forward-method` made-date. **Forbidden as seam-proof:** bulk-resolve-conflict,
  match-self-transfers (CSV-derived).
- **Census authority = the J6 packet manifest only** (`{seq}_{name}.pdf`); never a corpus-wide scan.
- **Version pin:** golden front-matter carries the `btctax-cli` crate version read from
  `crates/btctax-cli/Cargo.toml` (the binary has NO `--version`).
- **Validation:** `make check` (nextest + clippy, ~6s) is the fast gate; CI runs `cargo test --workspace
  --locked`. Green = full suite passes AND 0C/0I.

---

## Phase 0 — the `BTCTAX_NOW` CLI clock seam (gates all goldens)

**Owning FOLLOWUP:** UX-P0-1. **Gate:** green + Fable 0C/0I **before any golden is recorded.**

### Task 0.1: `resolve_now()` seam + banner + hard-error parsing

**Files:**
- Modify: `crates/btctax-cli/src/main.rs` (the `let now = OffsetDateTime::now_utc();` at `:66`, and add a
  private `resolve_now()` fn near `passphrase()` at `:49`).
- Test: `crates/btctax-cli/tests/btctax_now_seam.rs` (new).
- *(Man-page text is NOT touched here — it lives in `crates/xtask/src/docs.rs` hand-authored consts
  (`render_root`, `docs.rs:144-159`), and editing it before its Task 0.2 regen would red
  `gen_docs_is_deterministic`. Task 0.2 owns all man-page changes. — I1)*

**Interfaces:**
- Produces: `fn resolve_now() -> Result<time::OffsetDateTime, CliError>` — reads `BTCTAX_NOW`; unset ⇒
  `OffsetDateTime::now_utc()`; set ⇒ strict RFC3339 parse (hard `CliError::Usage` on malformed/empty/
  non-UTF-8) + an unconditional stderr banner; returns the parsed instant. `run()` calls it in place of
  the direct `now_utc()` read (main.rs:66).
- Consumes: `CliError::Usage(String)` (exists, `lib.rs`), `time::format_description::well_known::Rfc3339`.

- [ ] **Step 1: Write the failing tests** (`crates/btctax-cli/tests/btctax_now_seam.rs`)

```rust
//! P0 seam KATs — BTCTAX_NOW must make decision-bearing stdout deterministic without changing behavior
//! when unset. Binary-level (real process) so the seam is proven end-to-end, closing the gap the
//! library-level `now`-injection tests dodge. See SPEC §3.2 (T-P0.1..6).
use std::process::Command;
use std::path::Path;

fn bin() -> &'static str { env!("CARGO_BIN_EXE_btctax") }

/// Run `btctax` in `cwd` with the SPEC §3.3 pinned environment; return (code, stdout, stderr).
fn run_in(cwd: &Path, extra_env: &[(&str, &str)], args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(bin());
    c.current_dir(cwd)
        .env("BTCTAX_PASSPHRASE", "pw")
        .env("BTCTAX_PRICE_CACHE", cwd.join("no-such-cache.csv"))
        .env("HOME", cwd) // scrub HOME to a fixed path (§3.3 uniform contract — M6)
        .env("TZ", "UTC").env("LC_ALL", "C").env("LANG", "C")
        .args(args);
    for (k, v) in extra_env { c.env(k, v); }
    let out = c.output().expect("spawn btctax");
    (out.status.code().unwrap_or(-1),
     String::from_utf8_lossy(&out.stdout).into_owned(),
     String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Build a vault with a forward-method election recorded under a pinned clock; return the tempdir.
/// `config --set-forward-method` records a decision whose made-date defaults to `now` (BTCTAX_NOW),
/// and `verify` prints that `recorded` date — a clock-derived surface (SPEC §3.2).
fn vault_with_election(now: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let (c, _o, _e) = run_in(dir.path(), &[], &["--vault", "v.pgp", "init", "--key-backup", "k.asc"]);
    assert_eq!(c, 0, "init failed: {_e}");
    let (c, _o, e) = run_in(dir.path(), &[("BTCTAX_NOW", now)],
        &["--vault", "v.pgp", "config", "--set-forward-method", "hifo"]);
    assert_eq!(c, 0, "config failed: {e}");
    dir
}

#[test] // T-P0.3
fn malformed_btctax_now_is_exit_2_naming_the_var() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _out, err) = run_in(dir.path(), &[("BTCTAX_NOW", "not-a-date")],
        &["--vault", "v.pgp", "init", "--key-backup", "k.asc"]);
    assert_eq!(code, 2, "malformed BTCTAX_NOW must exit 2");
    assert!(err.contains("BTCTAX_NOW"), "error must name the variable; got: {err}");
}

#[test] // T-P0.3 (empty)
fn empty_btctax_now_is_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _o, err) = run_in(dir.path(), &[("BTCTAX_NOW", "")],
        &["--vault", "v.pgp", "init", "--key-backup", "k.asc"]);
    assert_eq!(code, 2);
    assert!(err.contains("BTCTAX_NOW"));
}

#[test] // T-P0.4
fn banner_on_stderr_when_set_never_on_stdout_absent_when_unset() {
    let dir = vault_with_election("2025-05-01T00:00:00Z");
    let (_c, out, err) = run_in(dir.path(), &[("BTCTAX_NOW", "2025-05-02T00:00:00Z")],
        &["--vault", "v.pgp", "verify"]);
    assert!(err.contains("BTCTAX_NOW override active"), "banner must be on stderr; got: {err}");
    assert!(!out.contains("BTCTAX_NOW override active"), "banner must NOT be on stdout");
    // unset: no banner
    let (_c, _out, err2) = run_in(dir.path(), &[], &["--vault", "v.pgp", "verify"]);
    assert!(!err2.contains("BTCTAX_NOW override active"), "no banner when unset; got: {err2}");
}

#[test] // T-P0.2 + T-P0.5
fn recorded_date_roundtrips_through_binary_and_is_twice_run_identical() {
    let dir = vault_with_election("2025-05-01T00:00:00Z");
    // T-P0.2: the pinned made-date surfaces in verify's `recorded`
    let (code, out, _e) = run_in(dir.path(), &[("BTCTAX_NOW", "2025-05-01T00:00:00Z")],
        &["--vault", "v.pgp", "verify"]);
    assert!(out.contains("2025-05-01"), "recorded date must reflect BTCTAX_NOW; got: {out}");
    // T-P0.5: byte-identical stdout AND exit code across two runs
    let (code2, out2, _e2) = run_in(dir.path(), &[("BTCTAX_NOW", "2025-05-01T00:00:00Z")],
        &["--vault", "v.pgp", "verify"]);
    assert_eq!((code, out), (code2, out2), "twice-run must be byte-identical");
}

#[test] // T-P0.1
fn unset_seam_behaves_normally() {
    // With BTCTAX_NOW unset the command still succeeds and prints no banner (inactive path unchanged).
    let dir = vault_with_election("2025-05-01T00:00:00Z"); // building under a pin is fine
    let (code, out, err) = run_in(dir.path(), &[], &["--vault", "v.pgp", "verify"]);
    assert_eq!(code, 0);
    assert!(!err.contains("override active"));
    // tighten (M2): "recorded" alone also matches "Lot selections recorded: 0"; assert the election's date
    assert!(out.contains("recorded 2025-05-01"), "verify still renders the election's recorded date");
}
```

- [ ] **Step 2: Run to verify they fail** — `cargo test -p btctax-cli --test btctax_now_seam` → FAIL
  (malformed BTCTAX_NOW currently ignored → wrong exit code / no banner).

- [ ] **Step 3: Implement `resolve_now()` and call it at main.rs:66**

```rust
// near the top of main.rs with the other `use`s:
use time::format_description::well_known::Rfc3339;

// replace `let now = OffsetDateTime::now_utc();` (main.rs:66) with:
    let now = resolve_now()?;

// add beside `passphrase()`:
/// Resolve "now". `BTCTAX_NOW` (RFC3339) pins the clock for reproducible docs/testing; unset ⇒ the
/// system clock (behavior unchanged). Malformed / empty / non-UTF-8 ⇒ a hard usage error (exit 2) — a
/// typo must never silently yield wall-clock nondeterminism nor a wrong made-date. When active, an
/// unconditional stderr banner discloses that decision timestamps are simulated. Backdating BTCTAX_NOW
/// does NOT make an identification contemporaneous under Treas. Reg. §1.1012-1(j) — the vault's recorded
/// timestamp is self-reported, not evidence of the fact. (SPEC §3.2 R-P0.1..6.)
fn resolve_now() -> Result<OffsetDateTime, CliError> {
    match std::env::var_os("BTCTAX_NOW") {
        None => Ok(OffsetDateTime::now_utc()),
        Some(os) => {
            let s = os.to_str().ok_or_else(|| {
                CliError::Usage("BTCTAX_NOW is set but not valid UTF-8".into())
            })?;
            let parsed = OffsetDateTime::parse(s, &Rfc3339).map_err(|e| {
                CliError::Usage(format!(
                    "BTCTAX_NOW is set but not a valid RFC3339 timestamp ({s:?}): {e}. \
                     Expected e.g. 2026-02-01T12:00:00Z."
                ))
            })?;
            eprintln!("warning: BTCTAX_NOW override active — decision timestamps are simulated");
            Ok(parsed)
        }
    }
}
```

- [ ] **Step 4: Run the seam tests** — `cargo test -p btctax-cli --test btctax_now_seam` → PASS.
- [ ] **Step 5: Run the whole CLI suite to prove no regression (fence iii)** — `make check` → PASS.
- [ ] **Step 6: Commit** — `git add -A && git commit -m "feat(cli): BTCTAX_NOW clock-injection seam (P0, SPEC §3.2)"`.

### Task 0.2: integrity KAT + man-page misuse language

**Files:**
- Test: `crates/btctax-cli/tests/btctax_now_seam.rs` (append T-P0.6; add `mod fixtures;` — reuse
  `crates/btctax-cli/tests/fixtures.rs`, precedent `end_to_end.rs:1`).
- Modify: `crates/btctax-cli/tests/fixtures.rs` — add `coinbase_two_lot_tax_saving(dir)` (**I9**), the
  exact shape of the existing test helper `write_tax_saving_csv` (`optimize_run.rs:85-98`): a Coinbase CSV
  with an LT lot (1 BTC @ $30k, 2023-01-01), an ST lot (1 BTC @ $80k, 2025-01-02), and a 1-BTC Sell @ $50k
  on 2025-06-01. FIFO picks the LT lot (a gain); HIFO picks the ST lot (a loss) — so the optimizer proposes
  a **changed** selection (not a no-change skip), which is what makes `optimize accept` reach the
  persistability gate. (Also J5's C-multilot corpus, Task 1.2 — DRY.)
- Modify: `crates/xtask/src/docs.rs` — the hand-authored root man-page consts (`ROOT_DESCRIPTION` /
  add a `ROOT_ENVIRONMENT`; `render_root` at `docs.rs:144-159`); regenerate `docs/man/btctax.1` via
  `cargo run -p xtask -- docs` **in this same commit** (so `gen_docs_is_deterministic` stays green — I1).

**Interfaces:** Consumes the P0 seam + `fixtures::coinbase_two_lot_tax_saving`. Produces the committed
`docs/man/btctax.1` ENVIRONMENT text (gated by `gen_docs_is_deterministic`, `docs.rs:353`).

- [ ] **Step 1: Write the born-passing disclosure KAT** — a **real** CLI-level test (not a stub). T-P0.6
  is a *characterization* KAT: the property (made-date ≤ sale ⇒ contemporaneous) already exists after the
  Task 0.1 seam + pre-existing core logic, so there is no "Run → FAIL" step; instead it is disclosed by a
  born-passing assertion whose negation is checked by a manual mutation (Step 3).

```rust
#[path = "fixtures.rs"] mod fixtures; // reuse the synthetic Coinbase builders (end_to_end.rs precedent)

#[test] // T-P0.6 — the KAT IS the disclosure that BTCTAX_NOW can move the attestation classification.
        // The Coinbase wallet IS a broker (WalletId::Exchange, optimize.rs:451-453); the KAT relies on
        // SPEC §3.2's "pre-2027 sale date" arm — the 2025-06-01 sale means ForbiddenBroker2027 (year>=2027,
        // optimize.rs:476-478) never fires, so the made<=sale lever governs: backdated made-date (<= sale)
        // ⇒ ContemporaneousNow; postdated (> sale) ⇒ NeedsAttestation. A CHANGED-row fixture is required
        // (I9) — a single-lot vault skips as "already optimal" identically in both runs.
fn backdated_vs_postdated_now_moves_the_attestation_classification() {
    // build a two-lot changed-selection vault (HIFO != FIFO), record a selection under a pinned clock
    fn accept_under(now: &str) -> String {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path();
        let (c, _o, e) = run_in(cwd, &[], &["--vault", "v.pgp", "init", "--key-backup", "k.asc"]);
        assert_eq!(c, 0, "init: {e}");
        let csv = fixtures::coinbase_two_lot_tax_saving(cwd); // I9: LT+ST lots, 2025-06-01 sell
        let (c, _o, e) = run_in(cwd, &[], &["--vault", "v.pgp", "import", csv.to_str().unwrap()]);
        assert_eq!(c, 0, "import: {e}");
        // `optimize accept --tax-year 2025` recomputes the year internally (accept_with_tables →
        // optimize_year, cmd/optimize.rs:198-208) — no prior `optimize run` and no --disposal needed
        // (Optimize::Accept.disposal is Option, cli.rs:315-326). Its made-date defaults to BTCTAX_NOW;
        // the rendered persistability label is the observable.
        let (_c, out, _e) = run_in(cwd, &[("BTCTAX_NOW", now)],
            &["--vault", "v.pgp", "optimize", "accept", "--tax-year", "2025"]);
        out
    }
    let back = accept_under("2025-01-01T00:00:00Z"); // <= 2025-06-01 sale ⇒ ContemporaneousNow
    let post = accept_under("2026-06-01T00:00:00Z"); // >  2025-06-01 sale ⇒ NeedsAttestation
    assert_ne!(back, post,
        "backdated vs postdated BTCTAX_NOW must change the attestation classification wording;\n\
         backdated:\n{back}\npostdated:\n{post}");
}
```

- [ ] **Step 2: Run → PASS** (born-passing disclosure KAT) — `cargo test -p btctax-cli --test btctax_now_seam` → PASS.
- [ ] **Step 3: Mutation-check** — temporarily invert the assert to `assert_eq!(back, post, …)`; run →
  RED (proving the two classifications genuinely differ, i.e. the seam reaches persistability); revert.
  *(Verified at plan time: `optimize accept --tax-year 2025` needs no `--disposal` and no prior `optimize
  run` — `accept_with_tables` recomputes via `optimize_year`, cmd/optimize.rs:198-208. If the observed
  output is identical, the fixture isn't a changed-row scenario — re-check `coinbase_two_lot_tax_saving`
  against the `write_tax_saving_csv` shape, do NOT relax the assert.)*
- [ ] **Step 4: Add man-page ENVIRONMENT language** — in `docs.rs` add/extend a root const (e.g.
  `ROOT_ENVIRONMENT`) wired into `render_root`:
  `BTCTAX_NOW — pins the clock (RFC3339) for reproducible testing and documentation. Backdating a decision
  record does not make an identification contemporaneous under Treas. Reg. §1.1012-1(j).`
  (and note `BTCTAX_PASSPHRASE` / `BTCTAX_PRICE_CACHE` alongside if not already present).
- [ ] **Step 5: Regenerate + verify docs (same commit)** — `cargo run -p xtask -- docs && cargo test -p xtask docs` → PASS.
- [ ] **Step 6: Commit** — `git commit -am "test(cli): BTCTAX_NOW integrity KAT + man-page misuse language (P0)"`.

### Task 0.3: P0 review gate
- [ ] Dispatch an independent **Fable** review of the P0 diff (seam + tests + man text) against SPEC §3.2
  + the fence; persist to `reviews/p0-fable-review.md`; fold to 0C/0I; re-review. **No golden may be
  recorded until this is green.** Update FOLLOWUPS UX-P0-1 → resolved.

---

## Phase 1 — CLI examples generator, corpora, golden (SPEC §4, §5, §7)

**Owning FOLLOWUP:** UX-P1-1, UX-P1-2 (reconcile at phase entry). **Gate:** the golden is born-green
in-tree via a `regen==committed` test in the SAME commit; determinism proofs pass; Fable 0C/0I.

### Task 1.1: `xtask examples` scaffold + the run/show capture helpers
**Files:** Create `crates/xtask/src/examples/mod.rs` (+ wire into `crates/xtask/src/main.rs` subcommand
dispatch and `crates/xtask/src/docs.rs` neighbors); Test: `crates/xtask/src/examples/mod.rs` `#[cfg(test)]`.
**New deps (N2):** add `tempfile` to `crates/xtask/Cargo.toml` `[dev-dependencies]` (generator vaults). No
`btctax-core` dep needed here (census option 1 is built as a `btctax-forms` test-support module — Task 2.1).
**Interfaces:** Produces `fn generate(bin: &Path) -> String` (returns the whole golden text) and
`struct Journey { name, steps: Vec<Step> }`; `run(cmd)` executes the pinned binary and emits `$ cmd` +
stdout + exit-code fences; **`run_with_stderr(cmd, label)` emits `$ cmd` + stdout + a labelled `stderr:`
fence** (I5 — export journeys emit the NO-AUTHORISATION notice, `main.rs:625-634`, and the R-P0.4 banner);
`prose(md)` interleaves narration.
- **Binary resolution (I3 — `CARGO_BIN_EXE_btctax` is NOT set for xtask):** `generate` takes an explicit
  built-binary path; the xtask subcommand + the Task 1.4 test resolve it by running
  `Command::new(env!("CARGO")).args(["build","-p","btctax-cli","--bin","btctax"])` (nested-cargo-in-test,
  the trybuild pattern) then using `{CARGO_TARGET_DIR or ./target}/debug/btctax` — so the golden is never
  compared against a **stale** binary (which would false-green the drift gate).
- [ ] Step 1: failing test — `generate(&built_btctax())` over a trivial one-command journey (`--help`)
  returns text containing `` $ btctax --help `` and the captured output; deterministic across two calls;
  a `run_with_stderr` step emits a `stderr:` fence.
- [ ] Step 2: run → FAIL. Step 3: implement the capture harness (env pins from Global Constraints incl.
  scrubbed HOME; the nested-cargo binary resolution above; capture stdout+exit + optional labelled stderr;
  **front-matter** = the pinned-env convention + an honest one-line "captures use `BTCTAX_PASSPHRASE`
  where a real user is prompted" sentence + the `btctax-cli` crate version from
  `crates/btctax-cli/Cargo.toml`). Step 4: run → PASS. Step 5: commit.

### Task 1.2: the synthetic corpora (SPEC §4.2)
**Files:** Create committed synthetic inputs under `crates/xtask/tests/fixtures/examples/`:
`self_transfer.csv`, `income_missing_fmv.csv`, `business.csv`, `multilot.csv`, `donation.csv`, and
`fullreturn_inputs.toml` (N1 — the TOML is a separate path, not a `.csv`). Reuse `btctax-cli/tests/
fixtures.rs` builders where they already produce a CSV.
**Interfaces:** Produces the committed CSV/TOML inputs each journey imports.
- [ ] Author **C-self-transfer**, **C-income-csv**, **C-business**, **C-multilot**, and **C-fullreturn**
  (kitchen-sink ReturnInputs TOML + a donation CSV so Sch A L12 > $500 — SPEC §4.2/§6.1). Each is a
  committed synthetic file. Step: add a `cargo test` asserting each imports without a hard blocker (except
  where the journey deliberately drives a blocker).
- [ ] **Oracle-consistency test (I6; home crate = `btctax-cli`, M7):** the TOML parser
  (`parse_return_inputs_toml`, `cmd/tax.rs:114`) is private to btctax-cli and xtask must not gain a
  btctax-core dep, so this test lives as a **btctax-cli integration test** (`crates/btctax-cli/tests/`),
  which already depends on btctax-core (reaching `btctax_core::tax::testonly::kitchen_sink_household()`,
  `testonly.rs:165`) and can drive the binary via `CARGO_BIN_EXE_btctax`. It imports the committed
  `fullreturn_inputs.toml` (`income import --year 2024 --file …`) then reads it back (`income show --year
  2024`) and asserts the captured `ReturnInputs` equals `kitchen_sink_household().0`. **Primary path:
  `toml::from_str::<ReturnInputs>(committed_toml) == kitchen_sink_household().0`** (`ReturnInputs` derives
  `Deserialize + PartialEq + Eq`, `return_inputs.rs:370`). **Do NOT compare the `income show` JSON against
  the raw vector (M8)** — `show` masks PII (`mask_pii`, `cmd/tax.rs:149-162`) while the vector has real
  SSNs, so that comparison is born-RED; a show-vs-show fallback (if ever needed) must serialize the vector
  via `toml::Value::try_from`, not `toml::to_string` (ValueAfterTable, `cmd/tax.rs:177`). Also assert the
  `business.csv` amounts against the same vector. So the doc's "the non-donation figures remain the oracle
  vector" claim cannot silently drift from a typo. Commit.

### Task 1.3: the six journeys (SPEC §5) + the whole-file golden
**Files:** Modify `crates/xtask/src/examples/mod.rs` (journey scripts J1–J6); Create golden
`docs/examples/examples.md`. *(M1: no `.gitignore` change — nothing ignores `docs/examples/`; only
`docs/pdf/` is ignored. Verify with `git check-ignore docs/examples/examples.md` → exit 1.)*
**Interfaces:** Consumes Task 1.1 harness + Task 1.2 corpora. Produces `docs/examples/examples.md`.
- [ ] Step 1: encode J1–J6 exactly per SPEC §5 (corrected commands: `init --key-backup`, kebab `--forms`,
  per-journey `BTCTAX_NOW`, J6 donation leg + `income import --file`). Step 2: `cargo run -p xtask --
  examples > docs/examples/examples.md`. Step 3: **write the born-green test** (Task 1.4). Step 4: commit
  golden + test together.

### Task 1.4: the born-green `regen==committed` test + determinism proofs (SPEC §7)
**Files:** Test: `crates/xtask/tests/examples_golden.rs`.
**Interfaces:** Consumes `examples::generate` + the nested-cargo binary resolution (Task 1.1, I3).
- **`#[cfg(unix)]`-gate these tests (I4).** Journey stdout embeds joined paths via `.display()`
  (`export-irs-pdf`, `main.rs:609-648`): `./irs/f8949.pdf` on Unix vs `./irs\f8949.pdf` on Windows, so a
  byte-exact golden **cannot** pass the windows-latest leg of `cargo test --workspace --locked`. Gate the
  regen/determinism tests (and Task 1.1's binary-spawning unit test) `#[cfg(unix)]` with a comment naming
  the path-separator divergence; the ubuntu CI `examples` job + the unix test legs are the gate.
- [ ] `examples_golden_matches_committed` — `generate(&built_btctax()) == fs::read_to_string("docs/examples/examples.md")`
  byte-for-byte (modeled on the committed-match half of `gen_docs_is_deterministic`, docs.rs:352-368).
  `examples_generate_is_deterministic` — two `generate` calls byte-identical. **Cross-HOME proof** (two
  HOME values → identical). **Price-cache proof (M4, SPEC §7):** regen with `BTCTAX_PRICE_CACHE` pointing
  at a *present* dummy cache vs. absent → identical (prices come from the bundled CSV; a stray cache must
  not bleed in). Land in the SAME commit as the golden. Run → PASS. Commit.

### Task 1.5: groff render target (SPEC §7)
**Files:** Modify `Makefile` (add `examples` target: wrap the golden verbatim blocks in roff `.nf/.fi`,
`groff -k -man -T pdf` → `docs/pdf/btctax-examples.pdf`, git-ignored).
- [ ] Add the target; prove it builds (`%PDF` magic check, like `write_pdfs`). Commit.

### Task 1.6: P1 review gate
- [ ] Reconcile FOLLOWUPS (UX-P1-1 capture conventions; UX-P1-2 the pre-existing doc bugs — file/own here,
  do NOT inline-edit product wording; fence class). Independent **Fable** review of the P1 diff →
  `reviews/p1-fable-review.md` → fold to 0C/0I → re-review.

---

## Phase 2 — CI gate + forms-census + subcommand report (SPEC §6, §9)

**Gate:** born-green (golden already gated in-tree from P1); a perturb-one-byte→RED proof observed; Fable 0C/0I.

### Task 2.1: forms-coverage census (HARD)
**Files:** Test: a **`btctax-forms` test-support module** (`crates/btctax-forms/tests/census.rs`) — keeps
the `PrintedReturn`/`ReturnHeader`/`FilingStatus`/`PrintedForms` construction in the crate that owns those
types (N2 — avoids an xtask→btctax-core dep).
**Interfaces:** Consumes `btctax_forms::fill_full_return(&PrintedReturn, year)` (N3 — the fn takes a
`&PrintedReturn`, NOT a bare `PrintedForms`; the fixture wraps an all-arms `PrintedForms` in a
`PrintedReturn` with a synthetic `ReturnHeader` + `FilingStatus`). Enumeration per SPEC §6.2.
- [ ] Step 1: `census_key_set_is_exactly_14` — build a `PrintedReturn` whose `forms: PrintedForms{…}` has
  every optional arm `Some` **and satisfies the three non-Option gates** (`sch_d.must_file`, `f8959`
  internal `must_file`, `f8283` filler `Some` — SPEC §6.2 note), push through `fill_full_return`, collect
  `NamedForm.name`, assert the set == the 14 literal keys (§6.1) — a shortfall reds here, not silently.
- [ ] Step 2: `every_census_form_demonstrated_in_j6` — **source the J6 manifest by parsing the committed
  golden's J6 "Full-return packet" stdout block** (`export-irs-pdf` prints each `{seq}_{name}.pdf` line +
  `manifest.txt`, `main.rs:640-648`) — no binary run needed (M3). Match on exact `{name}`-component
  equality (**J6 only**; never a corpus-wide scan — 3 slice stems collide). Assert all 14 present; FAIL
  loud on any gap. Step 3: implement; run → PASS. **Stage (no commit — the P2 atom commits in Task 2.3, I7).**

### Task 2.2: subcommand-coverage report (SOFT)
**Files:** `crates/xtask/src/examples/mod.rs` (a `subcommand_coverage_report()` walking `Cli::command()`
like `manpage_covers_every_subcommand`, docs.rs:261).
- [ ] Produce a printed/uploaded report of which subcommands lack a worked example; **non-blocking**.
  **Stage (no commit — the P2 atom commits in Task 2.3, I7).**

### Task 2.3: the CI `examples` job (SPEC §9)
**Files:** Modify `.github/workflows/ci.yml` (new `examples` job sibling to `test`).
- [ ] Job: build the binary → `cargo run -p xtask -- examples > docs/examples/examples.md` under the pinned
  env → `git diff --exit-code docs/examples` → run the forms-census (hard) + print the subcommand report →
  prove the PDF builds. Advisory (not yet a required check). Step: **perturb-one-byte proof** — locally
  edit one golden byte, run the diff step, observe RED, revert; record the observation in the commit msg.
  Land CI job + census + report in ONE commit (SPEC §9 rescoped atomicity). Commit.

### Task 2.4: P2 review gate
- [ ] Independent **Fable** review → `reviews/p2-fable-review.md` → fold to 0C/0I → re-review.

---

## Phase 3 — TUI style-aware capture + TUI clock seam (SPEC §3.4, §8)

**Owning FOLLOWUP:** UX-P3-1. **Gate:** TUI clock helper lands before any TUI golden; determinism proof;
Fable 0C/0I.

### Task 3.1: shared TUI clock helper (the TUI's own §3.1-fenced seam)
**Files:** Create `crates/btctax-tui/src/clock.rs` (env-injected `now()` reading `BTCTAX_NOW`, same
semantics as the CLI seam — hard error on malformed, banner on active); Modify the ~24+ production
`now_utc()` sites in `btctax-tui`/`btctax-tui-edit` (incl. `lib.rs:247,256`, `export.rs:30` via its
caller, `tui-edit/src/main.rs:2609`, and the decision-timestamp stamps) to route through it.
**Interfaces:** Produces `fn now() -> Result<OffsetDateTime, TuiError>` (or the crate's error type).
- [ ] Step 1: failing test — a `TestBackend` render of the what-if panel + the export-confirm modal under
  a pinned `BTCTAX_NOW` is byte-identical across two runs (today it embeds wall-clock). Step 2: run →
  FAIL. Step 3: implement the helper; re-point every production read (verify the count against §14 gap 4).
  Step 4: run → PASS + `make check`. Commit.

### Task 3.2: `TestBackend` style-aware capture harness (SPEC §8)
**Files:** Create `crates/btctax-tui/tests/tui_capture.rs` (+ a small `capture` helper module).
**Interfaces:** Produces `fn capture(buf: &Buffer) -> CapturedFrame` serializing per cell
`(symbol, fg, bg, modifier)` (+ decide `underline_color`/`skip`, §14 gap 7) as a glyph grid + a compact
style overlay.
- [ ] Step 1: failing test — capture a known tab render; assert the glyph grid + a specific styled cell
  (e.g. a selected row's `bg`) serialize deterministically. Step 2: run → FAIL. Step 3: implement the
  serializer (read `Cell.symbol()` + pub `fg`/`bg`/`modifier`). Step 4: run → PASS. Commit.

### Task 3.3: TUI journeys + goldens
**Files:** Create `docs/examples-tui/*.txt` goldens; Modify `tui_capture.rs` to drive the journeys via the
existing `handle_key(app, press(...))`/`type_str` harness (esp. the edit reconcile flow — the primary
bug-hunt surface).
- [ ] Drive the tabs + a `btctax-tui-edit` reconcile flow under pinned `BTCTAX_NOW`+`BTCTAX_PASSPHRASE`;
  commit goldens with a `regen==committed` test (mirror Task 1.4). **If any captured frame renders a
  filesystem path (e.g. the export-confirm modal's dir), apply the same `#[cfg(unix)]` gate as I4** (path
  separators diverge on Windows). Add a groff render target for the separate TUI PDF. Extend the CI
  `examples` job diff to `docs/examples-tui`. Commit.

### Task 3.4: P3 review gate
- [ ] Independent **Fable** review → `reviews/p3-fable-review.md` → fold to 0C/0I → re-review. Resolve
  UX-P3-1.

---

## Phase 4 — regen + ship + workaround-audit (SPEC §10)

**Gate:** whole-diff Fable review; audit filed.

### Task 4.1: the adversarial workaround-audit
**Files:** Create `design/usage-examples/reviews/tutorial-workaround-audit.md`.
- [ ] Drive the full assembled surface skeptically (esp. TUI-edit reconcile). Catalog every route-around;
  classify **bug-to-file / harness-artifact / intentional**; file each real bug to `FOLLOWUPS.md` with
  severity + owning phase. Keep standing behavioral assertions (refusals) live + gated. Commit.

### Task 4.2: whole-diff review + promote CI to required (SPEC §9)
- [ ] Regenerate all goldens; run the full suite. Independent **Fable** whole-diff review of
  `feat/usage-examples` vs `main` → `reviews/whole-branch-fable-review.md` → fold to 0C/0I → re-review.
- [ ] Promote the CI `examples` job to a required check (GitHub settings — user-actioned per §14 gap 1;
  add the fail-safe wedge-guard if a paths filter is later added).

---

## Merge / tag / release

- [ ] **Merge** `feat/usage-examples` → `main` (only after whole-diff review is green).
- [ ] **Version bump** to **v0.7.0** across the workspace (the seam is additive; lockstep per prior
  releases; no users yet). Update `crates/*/Cargo.toml` versions + `Cargo.lock`. **The bump reds
  `regen==committed` by design** (the golden front-matter carries the `btctax-cli` version, SPEC §7) —
  **regenerate both goldens (`docs/examples/`, `docs/examples-tui/`) in the SAME commit as the bump** (M5).
- [ ] **Tag** `v0.7.0` + **GitHub release** (notes: the BTCTAX_NOW seam + the two usage-example docs +
  the UX-P1-2 findings).
- [ ] **Attach the release PDFs (I8, SPEC §2 "built in CI, release-attached"):** build `make examples` +
  the TUI PDF target, then `gh release upload v0.7.0 docs/pdf/btctax-examples.pdf docs/pdf/btctax-tui-examples.pdf`.
- [ ] **crates.io publish** — the user holds a valid temp token (confirmed 2026-07-16), so **no pause**;
  publish in dependency order. Per memory ([[crate-publishing-state]]): `cargo publish --workspace` can
  internal-error at the tail → resume with `-p <crate>`; publishing already-published crates aborts;
  verify the index with `grep -c` (not `grep|head`). **Remind the user to revoke the token after** the
  publish is confirmed.

---

## Review cadence (each names a persisted artifact under `reviews/`)

| Gate | Artifact | Reviewer |
|---|---|---|
| Spec | `spec-r0-fable-review.md`, `spec-r1-fable-rereview.md` (GREEN) | Fable |
| Plan | `plan-r0-fable-review.md` | Fable |
| P0 | `p0-fable-review.md` | Fable |
| P1 | `p1-fable-review.md` | Fable |
| P2 | `p2-fable-review.md` | Fable |
| P3 | `p3-fable-review.md` | Fable |
| Whole branch | `whole-branch-fable-review.md` | Fable |

## Status
**r2 GREEN (2026-07-16) — Fable r2 re-review 0C/0I, ready to execute** (`reviews/plan-r2-fable-rereview.md`);
its one non-gating Minor (M8: use TOML-parse-vs-vector, not PII-masked `income show`) folded into Task 1.2.

**r2 (2026-07-16) — folded the Fable r1 re-review** (`reviews/plan-r1-fable-rereview.md`, 0C/1I/1Mi):
**I9** — T-P0.6's fixture was single-lot (`coinbase_buy_sell_send`), so `optimize accept` skipped as
"already optimal" identically in both runs (born-RED). Swapped to a new `coinbase_two_lot_tax_saving`
builder (the `write_tax_saving_csv` LT+ST shape, HIFO≠FIFO) and corrected the comment (Coinbase IS a
broker → the KAT relies on the pre-2027-sale arm, not "non-broker"). **M7** — named the oracle-equality
test's home crate (btctax-cli integration test, through the binary). Awaiting r2 re-review.

**r1 (2026-07-16) — folded the independent Fable r0 review** (`reviews/plan-r0-fable-review.md`,
0C/8I/6Mi/3N): all 8 Important + Minors/Nits addressed. Key folds: T-P0.6 rewritten from an empty-body
stub into a real backdated/postdated `optimize accept` KAT with a mutation-check (I2); xtask binary
resolution pinned to nested-`cargo build` (I3); golden tests `#[cfg(unix)]`-gated for the Windows
path-separator divergence (I4); stderr-mode + front-matter added (I5); C-fullreturn oracle-equality test
(I6); P2 one-commit atom + release PDF-attach + regen-on-bump (I7/I8/M5).

## Self-review (author, against the spec)
- **Spec coverage:** P0↔§3.2; P1↔§4/§5/§7; P2↔§6/§9; P3↔§3.4/§8; P4↔§10; merge/tag/release↔spec tail. ✓
- **Placeholder scan:** the only deferred concretion is the exhaustive per-journey command strings (they
  ARE the doc content, authored at P1 execution); T-P0.6 is now a concrete KAT (I2 fold), no vague TODO.
- **Type consistency:** `resolve_now`, `generate`, `capture`, the 14-key census set, and the clock-derived
  surface list are used consistently across tasks and match the spec.
