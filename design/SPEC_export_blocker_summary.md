# SPEC — export-snapshot unresolved-Hard-blocker summary (no more silent empty forms)

**Source baseline:** `main` @ `e9a3690` (branch `feat/export-blocker-summary`). **Review status: DRAFT —
awaiting R0.** Fixes the real-vault-test finding: `export-snapshot` silently writes an EMPTY Form 8949 / zero
Schedule D when unresolved Hard blockers make a year `NotComputable`, with no warning (`report` says so loudly;
export was mute). User-approved (2026-07-05, "follow your recommendation on blocker summary"). btctax-cli only.

## Goal
When `export-snapshot` runs against a projection with **unresolved Hard blockers**, print a summary to stderr —
which years are `NOT COMPUTABLE` and that the exported forms are INFORMATIONAL, not final — mirroring what
`report` already emits. The export still SUCCEEDS and writes the files (a blocker summary, NOT a refusal); the
user just learns why an 8949 came out empty/partial. A fully-resolved ledger exports with no such warning.

## Current state (verified)
- `cmd::admin::export_snapshot` (admin.rs:7-…) projects `(state, _cfg)`, gates on `pseudo_active()` →
  `require_attestation`, writes the SQLite + CSVs + per-year `se_result`, and returns `PathBuf`. It NEVER
  consults `state`'s Hard blockers. main.rs:325-326 prints only `Exported {path} + CSVs to {dir}`.
- `LedgerState`: `pub blockers: Vec<Blocker>` (state.rs:257); `Blocker { kind: BlockerKind, … }` (state.rs:100);
  `BlockerKind::severity()` → `Severity::Hard|Advisory` (state.rs:74); `has_hard_blockers()` exists
  (inspect.rs:22). `TaxYearNotComputable` is Hard (state.rs:49). `compute_tax_year(year,…)` returns
  `TaxOutcome::NotComputable(TaxYearNotComputable)` when a Hard blocker gates that year (the `report` path).

## Mechanism (I/O-explicit — library returns data, main.rs prints)
- `export_snapshot` returns a small **`ExportReport { path: PathBuf, unresolved_hard: usize, not_computable:
  Option<i32> }`** instead of a bare `PathBuf`:
  - `unresolved_hard` = count of `state.blockers` with `severity()==Hard`.
  - `not_computable` = `Some(year)` iff `tax_year == Some(year)` AND `compute_tax_year(year,…)` is
    `NotComputable` (the exported year's forms are empty/partial). `None` when no `tax_year` was requested or
    the year computes. (Keeping the library I/O-free — no printing inside it; the round-3 attest-gate I2 lesson.)
- **main.rs `ExportSnapshot` arm** (main.rs:325): print `Exported …` as today, THEN, if `unresolved_hard > 0`,
  `eprintln!` a warning to STDERR:
  - year-scoped + not-computable: `"⚠ tax year {y} is NOT COMPUTABLE — {n} unresolved Hard blocker(s) remain;
    the exported Form 8949 / Schedule D are INFORMATIONAL, not final. Run `btctax verify` to resolve them."`
  - otherwise (Hard blockers present but the requested year still computes, or no `--tax-year`):
    `"⚠ {n} unresolved Hard blocker(s) remain; some figures may be incomplete. Run `btctax verify`."`
  - `unresolved_hard == 0` → no warning (clean export, byte-identical output to today).
- Exit code stays 0 (a warning, not an error — the export succeeded). Warning to stderr so it doesn't pollute
  a piped stdout.

## KATs (btctax-cli tests)
- `export_with_hard_blockers_warns_and_still_writes` — a vault with an unresolved Hard blocker (e.g. an income
  FMV-missing) + `--tax-year Y` where Y is not-computable → files ARE written AND stderr contains "NOT
  COMPUTABLE" + the Hard-blocker count + "verify". Exit 0.
- `export_clean_ledger_no_warning` — a fully-resolved ledger → NO blocker warning on stderr (byte-identical to
  today's success output).
- `export_report_counts_only_hard` — `unresolved_hard` counts Hard blockers only (an Advisory-only ledger →
  0 → no warning).
- `export_not_computable_only_when_year_requested_and_blocked` — `not_computable` is `Some(y)` exactly when
  `--tax-year y` is requested AND y is `NotComputable`; `None` otherwise.
- **★ fault-inject target:** suppress the warning (drop the `unresolved_hard > 0` eprintln) ⇒
  `export_with_hard_blockers_warns_and_still_writes` goes RED.

## Scope / SemVer / lockstep
btctax-cli only (`export_snapshot` return type `PathBuf → ExportReport`; the main.rs arm; a warning). No core
change (reuses `state.blockers` / `compute_tax_year`). The only caller of `export_snapshot` is main.rs:325 (+
its tests) — update them. Additive warning behavior → MINOR. Lockstep: the `btctax-export-snapshot.1` man page
+ `--help` doc-comment gain a one-line note that export warns (but does not refuse) on unresolved Hard blockers.

## Plan (TDD)
- **T1** — `ExportReport` + compute `unresolved_hard`/`not_computable` in `export_snapshot`; the main.rs-arm
  stderr warning; the KATs + the ★ fault-inject; update the man page/`--help` note. Whole-diff + full suite +
  FOLLOWUPS (mark the real-vault-test "silent empty forms" finding resolved).

## Gotchas
- **Warn, don't refuse** — the export must still write the files (it's informational disclosure, not a gate);
  exit 0.
- **Hard only** — count `severity()==Hard`; Advisory blockers (normal in a working ledger) must NOT warn.
- **I/O in main.rs, not the library** — `export_snapshot` returns the report; the `eprintln!` lives in the arm
  (keeps the fn deterministic/testable — the attest-gate lesson).
- **Clean ledger unchanged** — 0 Hard blockers ⇒ no new output (don't regress the happy path).
- **stderr** — the warning goes to stderr so it never corrupts a redirected export/stdout.
