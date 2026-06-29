# Engineering Review — IMPLEMENTATION_PLAN_foundation_04_cli.md — Round 1

- **Reviewer:** independent senior Rust reviewer; traced every API call/field/signature against the live source of the three crates.
- **Date:** 2026-06-29
- **Verdict:** **1 Critical (C1) / 1 Important (I1)** + 3 Minor + 3 Nit. Persisted per STANDARD_WORKFLOW §2.

## CRITICAL
### C1 — `csv::Error` unconvertible to `CliError` → `write_csv_exports` won't compile (Task 0 / 15)
`write_csv_exports` uses `?` on `csv::Writer::from_path`/`write_record` (→ `csv::Error`), but `CliError` has no `Csv` variant and `csv::Error` isn't covered by `Io(#[from] io::Error)` (the From goes the other way), nor by adapters' named-struct `Csv{path,source}` variant. Compile blocker for Task 15 + the gate. **Fix:** add `#[error("csv: {0}")] Csv(#[from] csv::Error)` to `CliError` (+ the Public-interface list) and `use csv;` in render.rs.

## IMPORTANT
### I1 — `safe_harbor_attest` counts VOIDED allocations in its single-allocation guard (Task 13)
Collects all `SafeHarborAllocation` events regardless of voids; the legitimate workflow (allocate → inert → void → re-allocate → attest) yields two allocation events → trips "multiple allocations present" → attest permanently unusable. **Fix:** build the voided-target set from `VoidDecisionEvent`s and exclude voided allocations from the guard (mirrors the engine's pass-1 effectiveness). (Combine with the reconciliation review's I-2 attest fix.)

## MINOR
- M1 `verify` calls `load_all` twice per session (project() + explicit) — FOLLOWUP: a `Session::load_events_and_project()`.
- M2 render + CSV use `{:?}` Debug for enums (BasisSource/DisposeKind/Term/IncomeKind/GiftZone/BlockerKind) → CSV columns tied to Debug; add `Display`/`tag()` (FOLLOWUP before CSV consumers rely on it).
- M3 `rust-version.workspace = true` omitted from btctax-cli Cargo.toml (consistent with the other 3 crates, but MSRV enforcement is then inactive); add it (and ideally to all four).

## NIT
- N1 `let mut session = session;` rebinding in safe_harbor_allocate (works via NLL; simplify or comment).
- N2 attest stamps Void + re-attest with the same `now` (decision_seq distinguishes; correct; comment).
- N3 `_year` unused in report (intentional; filtering in render).

## Positive confirmations (verified vs live source)
`conn()` returns `&Connection` (no conn_mut); no overlapping borrow with `save()`; `append_decision(conn,payload,now,UtcOffset,Option<WalletId>)` all 11 call sites correct; `append_import_batch(&Connection,&[LedgerEvent])`; `ingest_files_bundled(&[PathBuf])`; `export_snapshot` writes snapshot.sqlite only (CSVs are CLI's job); all 16 payload field names match event.rs; LedgerState/Blocker/ConservationReport fields match state.rs/conservation.rs; BlockerKind variants correct; TRANSITION_DATE path; suffixed_key vault.pgp→vault.key; EventId::canonical()/parse round-trip with `|`-containing source_refs; MSRV 1.74 respected (map_or not is_none_or); holdings_by_wallet is BTreeMap (deterministic); FR9 exit code; single vault-open per command; injected `now` clock seam; privacy (temp vaults + synthetic fixtures, no real reads).

## Verdict
1 Critical (C1 compile) + 1 Important (I1 attest voided-count). After those + the reconciliation review's I-1/I-2, the plan is engineering-sound. Architecture solid: API fidelity high, borrow model correct, MSRV/determinism/privacy upheld.
