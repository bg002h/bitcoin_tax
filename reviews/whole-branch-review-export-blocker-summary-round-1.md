# Whole-diff review (Phase E) — feat/export-blocker-summary — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (e9a3690)..HEAD` — 1 impl commit (`391ff8a`), 10 files, +910/−9.
Contract: `design/SPEC_export_blocker_summary.md` (R0-GREEN, 2 rounds). btctax-cli only.

## Fault-injection (restored byte-for-byte)
- **[★ blocker warning] CONFIRMED load-bearing (two independent breaks).** `export_snapshot` returns
  `ExportReport { path, unresolved_hard }` where `unresolved_hard = blockers.filter(kind.severity()==Hard).count()`
  (admin.rs:113-116, no `compute_tax_year` — R0-I1); the main.rs arm `eprintln!`s the NOT-COMPUTABLE / hard-count
  warning to stderr when `>0`. **My fault-inject:** flipping the filter `Hard → Advisory` (count misses the real
  Hard blockers) drove `export_with_hard_blockers_warns_on_stderr` RED ("stderr must flag the year NOT
  COMPUTABLE; got: <empty>"). (The implementer independently confirmed deleting the `eprintln!` block also goes
  RED.) The count → warning path is guarded end-to-end.

## Verified by inspection + named KATs
- **Warn, don't refuse; exit 0; stderr** — files still written; stdout byte-identical on a clean ledger
  (`export_clean_ledger_no_warning`). **Both messages** carry "INFORMATIONAL, not final … verify" [R0-I2];
  no-`--tax-year` says "figures" not "forms" (projection CSVs, not forms) [R0-r2-M].
- **Hard-only** — `export_report_counts_only_hard` (a real Advisory `SelfTransferInboundZeroBasis` → 0; Hard
  `UnknownBasisInbound` → counted) [R0-N2]. `export_report_path_points_at_snapshot`;
  `export_still_writes_files_with_blockers`.
- **Test split** [R0-I3]: library KATs assert the struct; **binary** KATs
  (`export_with_hard_blockers_warns_on_stderr`, `export_full_no_year_warns_informational`,
  `export_clean_ledger_no_warning`) drive `CARGO_BIN_EXE_btctax` via `Command::output()` to capture stderr.
- **Blast radius** [R0-M2]: only the CLI `export_snapshot` changed → `ExportReport`; the store method
  (vault.rs:263) stays `PathBuf`. Value-consuming callers rewired to `.path` (main.rs, export.rs:26,
  pseudo_reconcile_cli.rs:226); `{:?}` sites use the new `Debug`.
- **Docs** [R0-I4]: the note lives in the `ExportSnapshot` clap doc-comment (cli.rs:92); `.1` regenerated via
  xtask; `gen_docs_is_deterministic` stays green (single-source respected — no hand-edit).
- Attest-gate ordering intact (require_attestation `?`-returns before the report is built).

## Suite
`cargo test --workspace --locked` 0 failed (re-run at merge; the implementer's "709" is a single-invocation
count — my summed re-run across all binaries confirms the real total). clippy -D + fmt clean. MINOR (additive
warning + `ExportReport` internal return-type change).

**SHIP — resolves the real-vault "silent empty forms" finding.**
