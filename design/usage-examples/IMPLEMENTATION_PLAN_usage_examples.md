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
- Doc: `crates/btctax-cli/src/cli.rs` root doc-comment (the `btctax.1` DESCRIPTION/ENVIRONMENT source) +
  `crates/xtask/src/docs.rs` `render_root` ENVIRONMENT/FILES section (man-page misuse language).

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
    assert!(out.contains("recorded"), "verify still renders the election");
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
- Test: `crates/btctax-cli/tests/btctax_now_seam.rs` (append T-P0.6).
- Modify: `crates/xtask/src/docs.rs` (`render_root` — add an ENVIRONMENT paragraph for `BTCTAX_NOW`) and
  regenerate `docs/man/btctax.1` via `cargo run -p xtask -- docs`.

**Interfaces:** Consumes the P0 seam. Produces the committed `docs/man/btctax.1` ENVIRONMENT text (gated by
`gen_docs_is_deterministic`).

- [ ] **Step 1: Write the failing integrity KAT** (append to the seam test file)

```rust
#[test] // T-P0.6 — the KAT IS the disclosure: backdating the clock ≤ a (non-broker, pre-2027) sale
        // yields a Contemporaneous classification. Uses a 2025 sale so ForbiddenBroker2027
        // (optimize.rs:476-480) never precedes ContemporaneousNow.
fn backdated_now_yields_contemporaneous_classification() {
    // Build a vault with a 2025 disposition, then `optimize accept` under a BTCTAX_NOW backdated to
    // before the sale; the accepted-selection output must show the contemporaneous (not needs-attestation)
    // wording. Exact vault construction reuses fixtures::coinbase_buy_sell_send via the generator harness;
    // if the CLI path is heavy, assert the property at the library layer on `persistability` with
    // made_date <= sale_date and a non-broker wallet. (Finalize the concrete surface at execution;
    // the property under test is fixed: made <= sale ⇒ ContemporaneousNow.)
    // Placeholder-free requirement: this test MUST assert on real output/return, not compile-only.
}
```

  *Execution note:* pick the cheapest surface that actually exercises the seam→persistability path (library
  KAT on `persistability(made, sale, wallet)` preferred if the CLI accept flow needs a disposition-bearing
  vault). Do NOT land a compile-only stub — a fix isn't done until the mutation dies (memory:
  untested-guard-pattern).

- [ ] **Step 2: Run → FAIL.** **Step 3: Implement/choose the surface. Step 4: Run → PASS.**
- [ ] **Step 5: Add man-page ENVIRONMENT language** in `render_root` (docs.rs), e.g.:
  `BTCTAX_NOW — pins the clock (RFC3339) for reproducible testing/documentation. Backdating a decision
  record does not make an identification contemporaneous under Treas. Reg. §1.1012-1(j).`
- [ ] **Step 6: Regenerate + verify docs** — `cargo run -p xtask -- docs && cargo test -p xtask docs` → PASS.
- [ ] **Step 7: Commit** — `git commit -am "test(cli): BTCTAX_NOW integrity KAT + man-page misuse language (P0)"`.

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
**Interfaces:** Produces `fn generate(bin_dir: &Path) -> String` (returns the whole golden text) and
`struct Journey { name, steps: Vec<Step> }`; `run(cmd)` executes the pinned binary and emits `$ cmd` +
stdout + exit-code fences; `prose(md)` interleaves narration.
- [ ] Step 1: failing test — `generate` over a trivial one-command journey (`--help`) returns text
  containing `` $ btctax --help `` and the captured output; deterministic across two calls.
- [ ] Step 2: run → FAIL. Step 3: implement the capture harness (env pins from Global Constraints;
  `CARGO_BIN_EXE`/`--bin-dir` resolution; capture stdout+exit; version from `btctax-cli/Cargo.toml` into
  front-matter). Step 4: run → PASS. Step 5: commit.

### Task 1.2: the synthetic corpora (SPEC §4.2)
**Files:** Create `crates/xtask/tests/fixtures/examples/{self_transfer,income_missing_fmv,business,multilot,fullreturn_inputs.toml,donation}.csv` (committed synthetic); reuse `btctax-cli/tests/fixtures.rs`
builders where they already produce a CSV.
**Interfaces:** Produces the committed CSV/TOML inputs each journey imports.
- [ ] Author **C-self-transfer**, **C-income-csv**, **C-business**, **C-multilot**, and **C-fullreturn**
  (kitchen-sink ReturnInputs TOML + a donation CSV so Sch A L12 > $500 — SPEC §4.2/§6.1). Each is a
  committed synthetic file. Step: add a `cargo test` asserting each imports without a hard blocker (except
  where the journey deliberately drives a blocker). Commit.

### Task 1.3: the six journeys (SPEC §5) + the whole-file golden
**Files:** Modify `crates/xtask/src/examples/mod.rs` (journey scripts J1–J6); Create golden
`docs/examples/examples.md`; flip its `.gitignore` untrack in the SAME commit.
**Interfaces:** Consumes Task 1.1 harness + Task 1.2 corpora. Produces `docs/examples/examples.md`.
- [ ] Step 1: encode J1–J6 exactly per SPEC §5 (corrected commands: `init --key-backup`, kebab `--forms`,
  per-journey `BTCTAX_NOW`, J6 donation leg + `income import --file`). Step 2: `cargo run -p xtask --
  examples > docs/examples/examples.md`. Step 3: **write the born-green test** (Task 1.4). Step 4: commit
  golden + test together.

### Task 1.4: the born-green `regen==committed` test + determinism proofs (SPEC §7)
**Files:** Test: `crates/xtask/tests/examples_golden.rs`.
**Interfaces:** Consumes `examples::generate`.
- [ ] `examples_golden_matches_committed` — `generate(bin_dir) == fs::read_to_string("docs/examples/examples.md")`
  byte-for-byte (modeled on `gen_docs_is_deterministic`, docs.rs:352-368). `examples_generate_is_deterministic`
  — two `generate` calls byte-identical. Cross-HOME proof (run under two HOME values). Land in the SAME
  commit as the golden. Run → PASS. Commit.

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
**Files:** Test: `crates/xtask/tests/forms_census.rs` (or a `btctax-forms` test-support module).
**Interfaces:** Consumes `btctax_forms::fill_full_return` (enumeration mechanism per SPEC §6.2:
all-arms-`Some` fixture asserting `count == 14`, cross-checked vs the §6.1 literal).
- [ ] Step 1: `census_key_set_is_exactly_14` — build an all-arms `PrintedForms`, push through
  `fill_full_return`, collect `NamedForm.name`, assert set == the 14 literal keys (§6.1). Step 2:
  `every_census_form_demonstrated_in_j6` — enumerate the J6 packet manifest (`{seq}_{name}.pdf`,
  **J6 only**, exact `{name}`-component match) and assert all 14 present; FAIL loud on any gap. Step 3:
  implement; run → PASS. Commit.

### Task 2.2: subcommand-coverage report (SOFT)
**Files:** `crates/xtask/src/examples/mod.rs` (a `subcommand_coverage_report()` walking `Cli::command()`
like `manpage_covers_every_subcommand`, docs.rs:261).
- [ ] Produce a printed/uploaded report of which subcommands lack a worked example; **non-blocking**.
  Commit.

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
  commit goldens with a `regen==committed` test (mirror Task 1.4). Add a groff render target for the
  separate TUI PDF. Extend the CI `examples` job diff to `docs/examples-tui`. Commit.

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
  releases; no users yet). Update `crates/*/Cargo.toml` versions + `Cargo.lock`.
- [ ] **Tag** `v0.7.0` + **GitHub release** (notes: the BTCTAX_NOW seam + the two usage-example docs +
  the UX-P1-2 findings).
- [ ] **crates.io publish** — requires a user-provided temp token (pause here for it). Publish in
  dependency order; per memory: `cargo publish --workspace` can internal-error at the tail → resume with
  `-p <crate>`; verify the index with `grep -c`. Remind the user to revoke the token after.

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

## Self-review (author, against the spec)
- **Spec coverage:** P0↔§3.2; P1↔§4/§5/§7; P2↔§6/§9; P3↔§3.4/§8; P4↔§10; merge/tag/release↔spec tail. ✓
- **Placeholder scan:** the only deferred concretions are the exhaustive per-journey command strings
  (they ARE the doc content, authored at P1 execution) and the T-P0.6 surface choice (constrained to
  "made ≤ sale ⇒ Contemporaneous", non-broker/pre-2027) — both bounded, neither a vague TODO.
- **Type consistency:** `resolve_now`, `generate`, `capture`, the 14-key census set, and the clock-derived
  surface list are used consistently across tasks and match the spec.
