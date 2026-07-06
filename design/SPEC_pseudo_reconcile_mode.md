# SPEC — pseudo-reconcile mode (sub-project 2 of auto-pseudo-reconcile)

**Source baseline:** `main` @ `514875b` (branch `feat/pseudo-reconcile-mode`). **Review status: R0-GREEN (3 rounds; 0C/0I).
Cleared to implement.** Reviews: `reviews/R0-spec-pseudo-reconcile-mode-round-{1,2,3}.md`. Round 1 2C/5I,
round 2 0C/1I (I2-R), round 3 0C/0I/2M/2N — all folded IN-PLACE (surgical; no append). M2 (Unclassified →
`ClassifyRaw`) folded into the defaults table. Design of record:
`design/BRAINSTORM_auto_pseudo_reconcile.md`; roadmap memory `auto-pseudo-reconcile-roadmap`. **Cross-cutting
decisions settled — do NOT re-brainstorm.** Sub-project 1 (per-exchange method election) is SHIPPED (`514875b`)
and is the method source for pseudo Sells.

## Goal
A reversible **mode** that fills DELIBERATELY-FICTIONAL default decisions at PROJECTION time (never persisted)
to clear the **Hard CLASSIFICATION blockers**, producing a loudly-flagged on-screen picture the user corrects
toward truth. Real decisions always supersede. **Bulk-approve** promotes chosen defaults to real (attested)
decisions; `off` reverts everything.

**What "0 blockers" means [R0-I2 — precise]:** pseudo clears the Hard *classification* kinds it can honestly
default (`UnknownBasisInbound`, `Unclassified` inbound, `ImportConflict`) — AND, since **#41 Part B (REVERSAL of
the original decision below)**, native-`Income` `FmvMissing` **WHEN a local daily-close price exists** (it is
synthesized from that close as a `PseudoKind::PseudoFmv` `ManualFmv` default, flagged `[PSEUDO]`, approve-able).
It does NOT clear, and leaves SURFACED: `UncoveredDisposal` (under-covered real Sells — fabricating acquisitions
would be max-gain fiction), native-`Income` `FmvMissing` **when NO local price exists** (the residual #41 Part C's
online updater addresses), `DecisionConflict` (a collision of REAL decisions — see Mechanism), and
`TaxTableMissing` (a missing-bundle defect). **A tax TOTAL computes only when
0 Hard blockers of ANY kind remain** — `compute_tax_year` returns `NotComputable` on the first Hard blocker
(compute.rs:242,445-450; every excluded kind is Hard, state.rs:71-83). So with any excluded kind present, the
user sees the flagged HOLDINGS/skeleton (and those blockers) but not a tax number until they resolve them; the
"≈zero-tax null-hypothesis" total is assertable only once they're all cleared.

## Mechanism — projection-time default-injection (NOT persisted)
- **Mode flag:** vault setting `pseudo_reconcile: bool` (btctax-cli config, mirroring `pre2025_method`/fee),
  default **false** [N1]. `reconcile pseudo on|off`. Off ⇒ projection byte-identical to today.
- **Injection [R0-I1]:** in `resolve`, when on, at the **map/`Eff` layer** (resolve.rs:102-111) synthesize
  in-memory defaults for each still-unresolved event and record their targets in a `pseudo_ids` set on the
  `Resolution`/`Eff`. **Do NOT mint `EventId::Decision{seq}`** (that `u64` collides the real decision_seq space,
  identity.rs:69) — synthetics are map-layer entries, not seq-bearing events. Real decisions are collected
  FIRST ⇒ an event with any real decision gets NO synthetic default (real supersedes; no conflict, no void).
- **Taint propagation [R0-C1 — the headline correctness point]:** a per-event flag is INSUFFICIENT. A `pseudo`
  bit rides the DATA: `Lot`(incl. relocated lots fold.rs:766-813) → `Consumed`(pools.rs:289-302) →
  `DisposalLeg`/`PendingLeg`/held-lot row (state.rs:124-140). A row is `[PSEUDO]` if its EXISTENCE **or its
  BASIS** traces to any synthetic — e.g. a REAL imported Sell consuming a pseudo $0-basis lot (fold.rs:994-1008)
  MUST render its gain flagged, never as a clean `proceeds − 0` (render.rs:617-639).
- **Determinism (NFR4):** synthetics are a pure function of (events, real-decisions, mode); no `Date::now`/RNG;
  stable order.

## The defaults (only where no real decision exists)
| Blocker / event | Synthetic default |
|---|---|
| `UnknownBasisInbound` inbound | `ClassifyInbound(SelfTransferMine{ basis:$0 })` — never income (assumption 3) |
| `Unclassified` inbound [R0-M2] | `ClassifyRaw` synthetic (resolve.rs:474-492) — NOT `ClassifyInbound` (rejected on a non-`TransferIn` `Unclassified` → `DecisionConflict`, resolve.rs:642-654) |
| `TransferOut` withdrawal (unmatched) [R0-I5] | **leave as `Op::PendingOut`** (fold.rs:698-740 — already non-taxable, no dest fabricated); `approve` writes the concrete self-transfer decision the user confirms |
| Outbound network fee | de-minimis: drop fee sats; basis stays with held coins; NO re-homing |
| `ImportConflict` [R0-C2] | accept-first: synthetic `SupersedeImport` of the first-seen (map-clearable, resolve.rs:430-472) |
| `DecisionConflict` [R0-C2] | **NOT cleared** — a real-decision collision (resolve.rs:630-640); stays SURFACED |
| `TaxProfileMissing` [R0-I2/M6] | CLI-layer PLACEHOLDER profile (single, $0 income/MAGI/qual-div) injected at `report_tax_year` (tax.rs:66-68) — clears `TaxProfileMissing` ONLY (compute.rs:265-272), NOT `TaxYearNotComputable` |
| method for real Sells | sub-1 resolver (scoped election → global → FIFO) |
Sells are taxable disposals from import — pseudo does NOT touch them.

## Guard (sub-2's half; the export/forms typed-attest gate is sub-3)
- **On-screen `[PSEUDO]`** on report/TUI rows (render.rs:211-227,252) + a `PseudoReconcileActive` ADVISORY
  blocker in `verify` (renders automatically via `{:?}`), driven by a **dedicated `pseudo` bool** [R0-I4] that
  the CSV/form writers OMIT — NOT a `BasisSource::Pseudo` variant (`lots.csv` writes `basis_source_tag`,
  render.rs:596 → would LEAK "PSEUDO" into the export and fail the grep-KAT).
- **[R0-I3 — interim export guard, until sub-3 ships]:** `export_snapshot` (admin.rs:45-85) consumes the
  "pseudo-active" signal and REFUSES while any synthetic contributes — no fictional 8949/Schedule D leaves the
  machine unguarded. (Sub-3 replaces this with the "I attest this is true" typed gate.)
- Expose a queryable count of contributing synthetics (for the banner + I3 + sub-3).

## Bulk-approve + revert
- `reconcile pseudo approve` (+ TUI flow + a filter by default-type/wallet/year): materialize selected
  synthetics as REAL decisions via the **btctax-cli `apply_bulk_*` OWN-LOOP** [R0-M4] (NOT tui-edit's
  `persist_bulk_decisions` — dep cycle, Cargo.toml:19): empty-guard + mid-batch rollback + single save,
  deterministic order [N2]. After approval they're real/attested → no longer `[PSEUDO]`.
- `reconcile pseudo off` — clears the flag; projection reverts to real-only instantly + totally (0 fictional
  events were ever written). Already-approved decisions REMAIN (they're real now — the point).

## Tax-safety invariants (fault-inject each)
- Mode OFF ⇒ projection byte-identical (KAT diffs a fixture snapshot).
- REAL decision on an event ⇒ NO synthetic for it (fault-inject: break precedence → real ignored → RED).
- **[★] `[PSEUDO]` appears on-screen (report/TUI) and is PROVABLY ABSENT from every export CSV / form** — the
  headline guard; a KAT greps the export for any pseudo/synthetic marker → asserts NONE, while the on-screen
  render carries it. Includes the C1 basis-taint case (a real Sell on a pseudo lot is flagged on-screen).
- Synthetics NOT persisted (KAT: after projecting in pseudo mode, `load_all` shows no new events; only `approve`
  writes).
- Determinism: two pseudo projections byte-identical.

## Scope / SemVer / lockstep
btctax-core (resolve map-injection + `pseudo` bit threading + accept-first `SupersedeImport`) + btctax-cli
(`reconcile pseudo on/off/approve`, config flag, placeholder-profile CLI injection, render `[PSEUDO]`, export
refusal) + btctax-tui-edit (banner + approve flow). Mode-off byte-identical → PATCH-class behavior; MINOR for
the new subcommands. Lockstep: `make docs`, `?`-overlay, doc-comments. **No GUI schema_mirror** (no GUI crate).

## Plan (TDD)
- **T1** — mode flag + `reconcile pseudo on/off`; mode-off-identical KAT.
- **T2** — resolve map-injection + `pseudo` bit threaded through fold to disposals/lots/legs (incl. C1 basis
  taint via relocated lots); real-supersedes + not-persisted + determinism KATs.
- **T3** — the defaults (self-transfer/$0, leave-pending outflow, accept-first ImportConflict, de-minimis fee)
  + CLI placeholder profile; per-default KATs; the "0 Hard classification blockers; tax TOTAL only at 0 Hard
  total" end-to-end KAT (I2-precise).
- **T4** — render `[PSEUDO]` (dedicated bool, writers omit) + `PseudoReconcileActive` advisory + the ★
  on-screen-yes / output-no + basis-taint fault-inject KAT + the I3 export-refusal KAT.
- **T5** — bulk-approve (`apply_bulk_*` own-loop) + revert; approve-materializes-real + revert-is-total KATs.
- **T6** — btctax-tui-edit banner + approve flow; `make docs`; whole-diff + full suite + FOLLOWUPS.

## Gotchas
- **Synthetics NEVER persisted by projection** — only `approve` writes (else ledger corruption). Not-persisted KAT.
- **[★] flags on-screen, clean output** — the load-bearing guard; the fault-inject KAT (incl. C1 basis taint) is mandatory.
- **Real supersedes pseudo** — collect real first; an event with a real decision gets no default.
- **Sells are NOT self-transfers** — only `TransferOut` withdrawals default (to leave-pending); imported `Dispose(Sell)` stays taxable (sub-1 method).
- **`DecisionConflict` + `UncoveredDisposal` + `TaxTableMissing` are NOT cleared** — surfaced; a tax TOTAL needs them resolved by the user. (native-Income `FmvMissing` is now cleared by #41 Part B WHEN a local daily-close price exists — flagged `[PSEUDO]`; it stays surfaced only when no price is available.)
- **No `EventId::Decision{seq}` minting in injection** (real-seq collision) — map-layer only; seq-minting in `approve`.
- **Marker via a dedicated bool the writers omit** — never a `BasisSource` variant (export leak).
