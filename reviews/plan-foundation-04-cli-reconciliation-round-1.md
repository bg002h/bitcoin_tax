# Reconciliation-correctness Review — IMPLEMENTATION_PLAN_foundation_04_cli.md — Round 1

- **Reviewer:** independent reconciliation/tax-mapping reviewer; verified command→decision-event routing vs core's event.rs + resolve.rs/fold.rs/transition.rs.
- **Date:** 2026-06-29
- **Verdict:** **0 Critical / 2 Important (I-1, I-2)** + 2 Minor + 2 Nit. Persisted per STANDARD_WORKFLOW §2.

## Verified CORRECT (every command except safe-harbor)
- `link-transfer`→TransferLink (non-taxable; TP8(c) fee default applied by core, never flippable from CLI).
- `classify-inbound`→ClassifyInbound — both Income (kind/fmv/business→TP3) and GiftReceived (donor_basis/fmv_at_gift/donor_acquired_at→TP11 all 4 §1015(a) cases) routed exactly. **This is the transfer-basis-GAP re-supply — correct.**
- `reclassify-outflow`→ReclassifyOutflow{Dispose/GiftOut/Donate} with correct proceeds/fee_usd split (on-chain fee_sat passes through from the original TransferOut; CLI fee_usd = disposition fee).
- `set-fmv`→ManualFmv; `void`→VoidDecisionEvent; `classify-raw`→ClassifyRaw (is_imported guard); `accept/reject-conflict`→Supersede/RejectImport (conflict_event id, latest-seq precedence).
- **TP8 default stays (c)**; config makes (b) opt-in only; never flipped.
- **FR9 verify**: conservation_report + all blockers (hard vs advisory partition correct) + pending + unknown-basis + safe-harbor status; non-zero exit on hard blockers. Correct.
- **FR10 export**: export-snapshot (plaintext SQLite, NFR2 exception) + CLI CSVs (lots/disposals/removals/income) from engine-computed values, no CLI fabrication, Decimal exact. Correct.
- **O3** (config side-table = projection input, not ledger state): consistent with project(config) param + NFR6 carve-out — sound. **O4** (ProRata actual-position seed): ProRata is attestation-gated/inert otherwise — no silent tax error; acceptable.

## IMPORTANT
### I-1 — `safe-harbor allocate` seeds AllocLots from post-2025-disposal `state.lots`, not the pre-2025 residue (Task 13)
Builds AllocLots from `state.lots` filtered `acquired_at < TRANSITION_DATE`, using `remaining_sat`/`usd_basis` from the FULL projection (post-2025-disposal). The engine's conservation guard compares against `transition::universal_snapshot` (pre-2025-only fold = the 2025-01-01 residue). Once any 2025 disposal consumed pre-2025 lots, `alloc_sat < snap.held_sat` → hard `SafeHarborUnconservable` → Path A forced; Path B unusable via auto-build, no CLI recovery. Fails closed (no silent wrong tax) but breaks the normal workflow. **Fix:** seed AllocLots from a **pre-2025-only projection** (filter events to tax_date < TRANSITION_DATE, project, use those lots) so the allocation equals the 2025-01-01 residue the engine checks against.

### I-2 — `safe-harbor attest` on an already-effective allocation permanently breaks Path B (Task 13)
attest guards `prior.timely_allocation_attested` but NOT effectiveness. Attesting an allocation that's already effective (made-date predates first-2025-disposition, so effective without attestation) → appends Void (→ void-of-effective = DecisionConflict, rejected) + a new effective allocation → two effective → DecisionConflict → Path A, irrecoverable. **Fix:** before attesting, evaluate whether the prior allocation is already effective (time-bar vs first_2025_disposition); reject with a clear message if so.

## MINOR
- M-1 `verify` double-loads events (project() + explicit load_all) — efficiency only.
- M-2 `AllocLot` has no `dual_loss_basis` → a pre-2025 received-gift lot loses §1015(a) dual basis under Path B (spec defines AllocLot without it; faithful to spec). Record in FOLLOWUPS.

## NIT
- N-1 `set_fmv` test targets an Acquire (engine applies ManualFmv only to Income) — strengthen to an Income{Missing} target asserting the blocker clears.
- N-2 attest error message advises voiding the effective allocation (which is irrecoverable) — advise `verify` first.

## Round 2 — fold re-review — CLOSED (0 Critical / 0 Important)
Independent re-review (incl. the engineering round-1 C1/I1) confirms all blocking findings closed:
- **C1 (eng):** `CliError::Csv(#[from] csv::Error)` added (enum + interface list + self-review); no From-collision with Io; render uses `csv::Writer`.
- **I-1:** `safe_harbor_allocate` now seeds from a PRE-2025-ONLY `project()` (import events tax_date<TRANSITION_DATE; decisions/conflicts kept; prior SafeHarborAllocation dropped) → equals the engine's `universal_snapshot` residue → conservation passes. Regression test (allocate after a 2025 Sell consumed a pre-2025 lot → seeds the full residue, no Unconservable) verified.
- **I-2(a)/Eng-I1:** attest excludes voided allocations from the single-allocation guard (voided set from VoidDecisionEvents).
- **I-2(b):** attest re-projects + reads the engine verdict; rejects an already-effective allocation with a log-unmutated `Usage` error advising `verify`; proceeds only when inert purely due to time-bar. No borrow-after-move (NLL/edition 2021). 3 regression tests cover voided-excluded / already-effective-rejected / inert-timebar-proceeds.
- M3 rust-version.workspace added; attest message advises `verify`; FOLLOWUPS records M-2 (AllocLot no dual_loss_basis), M-1 (verify double-load), eng-M2 (Debug-format CSV), N-1, + the display-only stale-timebar nit. No new inconsistency.

**Net: 0 Critical / 0 Important — btctax-cli plan GREEN, ready to implement.**
