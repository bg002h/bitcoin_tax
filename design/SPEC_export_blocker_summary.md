# SPEC — export-snapshot unresolved-Hard-blocker summary (no more silent empty forms)

**Source baseline:** `main` @ `e9a3690` (branch `feat/export-blocker-summary`). **Review status: R0-GREEN (2 rounds; 0C/0I).
Cleared to implement.** Reviews: `reviews/R0-spec-export-blocker-summary-round-{1,2}.md`. Round 1 0C/4I
(redundancy removal + binary-test split + generated-docs), round 2 0C/0I/1M/1N (Message-B "figures" not "forms";
binary KAT uses `Command::output()`). Fixes the real-vault-test finding: `export-snapshot`
silently writes an EMPTY Form 8949 / zero Schedule D when unresolved Hard blockers make years `NotComputable`,
with no warning. User-approved (2026-07-05). btctax-cli only.

## Goal
When `export-snapshot` runs against a projection with **unresolved Hard blockers**, print a warning to stderr —
the exported forms are INFORMATIONAL, not final — mirroring `report`. The export still SUCCEEDS and writes the
files (a disclosure, NOT a refusal); exit 0. A fully-resolved ledger exports with no such warning
(byte-identical to today).

## Key invariant driving the design [R0-I1]
**ANY unresolved Hard blocker gates EVERY year** (`compute_tax_year` short-circuits on the projection-wide
`first_hard_blocker`, compute.rs:237-241/441-450). So there is NO state where Hard blockers exist yet a
requested year still computes. Therefore a per-year `compute_tax_year` check adds ZERO information over
`unresolved_hard > 0` — the round-1 `not_computable` field + its `compute_tax_year` call are DROPPED (they'd
also wrongly fire on `TaxProfileMissing`/`TaxTableMissing` with 0 Hard blockers). The warning is driven purely
by `unresolved_hard > 0` + whether a `--tax-year` was requested.

## Mechanism (I/O-explicit — library returns data, main.rs prints)
- `cmd::admin::export_snapshot` (admin.rs:50 — **the CLI wrapper; NOT the store method vault.rs:263 which stays
  `PathBuf`** [R0-M2]) returns **`ExportReport { path: PathBuf, unresolved_hard: usize }`** (`#[derive(Debug,
  Clone)]` [R0-M3]) instead of a bare `PathBuf`. `unresolved_hard = state.blockers.iter().filter(|b|
  b.kind.severity() == Severity::Hard).count()` (state.rs:74-97/257) — Hard only; Advisory (incl.
  `PseudoReconcileActive`) never warns. No `compute_tax_year` call, no profile/tables dependency added.
- **main.rs `ExportSnapshot` arm** (main.rs:325): print `Exported …` as today, THEN if `unresolved_hard > 0`,
  `eprintln!` to STDERR [R0-I2 — both messages carry the load-bearing "INFORMATIONAL, not final … verify"]:
  - `--tax-year {y}` requested: `"⚠ tax year {y} is NOT COMPUTABLE — {n} unresolved Hard blocker(s) remain; the
    exported Form 8949 / Schedule D are INFORMATIONAL, not final. Run `btctax verify` to resolve them."`
  - no `--tax-year` (full export writes ALL years, cli.rs:94-100 — every year is NOT COMPUTABLE under any Hard
    blocker, so the risk is BROADER): `"⚠ {n} unresolved Hard blocker(s) remain — every affected year is NOT
    COMPUTABLE; the exported figures are INFORMATIONAL, not final. Run `btctax verify`."` [R0-r2-M: "figures"
    not "forms" — no `--tax-year` writes the projection CSVs, not the 8949/Schedule D forms.]
  - `unresolved_hard == 0` → NO warning (stdout byte-identical to today).
- Exit code stays 0 (a warning, not an error). stderr so it never pollutes a redirected stdout.
- **Attest-gate ordering (verified):** `require_attestation` (admin.rs:62-64) runs FIRST, before any byte /
  before the report; a refused pseudo export `?`-returns `Err` at main.rs:325 and never reaches the warning.
  An attested pseudo draft that still carries a real Hard blocker → warns. Correct.
- **Blast radius [R0-M2]:** `export_snapshot`'s only non-test caller is main.rs:325 (→ `report.path.display()`);
  the return-type change is a COMPILE error at each value-consuming test, never silent. Rewire `tests/export.rs`
  + `tests/pseudo_reconcile_cli.rs` sites that read the path to `.path`; the `{:?}` sites rely on the new
  `Debug`. The store `export_snapshot` (vault.rs:263) + `tests/integration.rs:282` are untouched.

## KATs — split library vs binary [R0-I3]
- **Library** (call `cmd::admin::export_snapshot` directly): `export_report_counts_only_hard`
  (Hard→counted; an Advisory-only ledger — use a REAL Advisory e.g. `SelfTransferInboundZeroBasis` [R0-N2] —
  → `unresolved_hard == 0`); `export_report_path_points_at_snapshot`; `export_still_writes_files_with_blockers`.
- **Binary integration** (drive the built binary via `env!("CARGO_BIN_EXE_btctax")` + `std::process::Command`,
  **using `Command::output()` to CAPTURE stderr** [R0-r2-N] — not the cited sites' `.status()` shape — the
  `tests/fr9_exit_code.rs:53` / `tests/tax_report.rs:728` bin-name pattern):
  `export_with_hard_blockers_warns_on_stderr` — Hard-blocker vault + `--tax-year Y` → stderr contains "NOT
  COMPUTABLE" + the count + "verify"; files exist; **exit 0**. `export_full_no_year_warns_informational` — no
  `--tax-year` → stderr contains "INFORMATIONAL"/"NOT COMPUTABLE". `export_clean_ledger_no_warning` → stderr
  has no ⚠/"NOT COMPUTABLE".
- **★ fault-inject:** delete the `unresolved_hard > 0` `eprintln!` in the main.rs arm ⇒
  `export_with_hard_blockers_warns_on_stderr` (binary KAT) goes RED.

## Scope / SemVer / lockstep
btctax-cli only (`export_snapshot` return `PathBuf → ExportReport`; the main.rs arm; a stderr warning). No core
change. Additive warning → MINOR. **Lockstep [R0-I4]: the man page is GENERATED** (clap_mangen via
xtask/docs.rs, with the `gen_docs_is_deterministic` drift test docs.rs:342-357). Add the "warns (does not
refuse) on unresolved Hard blockers" note to the **`ExportSnapshot` clap doc-comment (cli.rs:92)**, THEN
`cargo run -p xtask -- docs` to regenerate `btctax-export-snapshot.1` (do NOT hand-edit the `.1`). The
`.contains` help test (cli.rs:750) won't break. FOLLOWUPS: mark the real-vault "silent empty forms" finding resolved.

## Plan (TDD)
- **T1** — `ExportReport` (Debug/Clone) + `unresolved_hard` in `export_snapshot`; rewire main.rs:325 + the
  path/`{:?}` test sites; the main.rs-arm stderr warning (both messages); the library + binary KATs + the ★
  fault-inject; the cli.rs:92 doc-comment note + regenerate docs. Whole-diff + full suite + FOLLOWUPS.

## Gotchas
- **[I1] no `compute_tax_year`** — `unresolved_hard > 0` already means every year is not computable; don't add
  the redundant call/dependency.
- **[I2] both messages say "INFORMATIONAL, not final"** — the full-export path is the MORE dangerous one.
- **[I3] the stderr assertion is a BINARY test** — the `eprintln!` is in main.rs; library tests see only the struct.
- **[I4] doc-comment + regenerate**, never hand-edit the generated `.1`.
- **Warn, don't refuse; Hard only; exit 0; stderr; clean ledger unchanged.** [N1] a doc sentence tells
  automation to gate on `btctax verify` (exit 1), since export stays exit 0.
- **[M2] only the CLI `export_snapshot` changes** — the store method stays `PathBuf`.
