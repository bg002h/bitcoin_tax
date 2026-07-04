# SPEC — pseudo-reconcile mode (sub-project 2 of auto-pseudo-reconcile)

**Source baseline:** `main` @ `514875b` (branch `feat/pseudo-reconcile-mode`). **Review status: DRAFT — awaiting
R0 (2 rounds to 0C/0I).** Design of record: `design/BRAINSTORM_auto_pseudo_reconcile.md`; roadmap memory
`auto-pseudo-reconcile-roadmap`. **All cross-cutting decisions are settled with the user — do NOT
re-brainstorm.** Sub-project 1 (per-exchange method election) is SHIPPED (`514875b`) and is the method source
for pseudo Sells.

## Goal
A reversible **mode** that takes an unreconciled ledger from N blockers → 0 by filling DELIBERATELY-FICTIONAL,
reasonable **default decisions** at PROJECTION time (never written to the ledger), producing a loudly-flagged
on-screen tax ESTIMATE the user then corrects toward truth. Real decisions always supersede. A **bulk-approve**
promotes chosen defaults to real (attested) decisions. `off` reverts everything.

## Mechanism — a projection-time default-injection pass (NOT persisted)
- **Mode flag:** a vault setting `pseudo_reconcile: bool` (btctax-cli config side-table or a `Config` field,
  mirroring `pre2025_method`/fee-treatment). `reconcile pseudo on|off`. Off = today's behavior, byte-identical.
- **Injection:** in `resolve` (after collecting REAL decisions), when the mode is on, for each event still
  UNRESOLVED (would raise a classification blocker) synthesize an in-memory **synthetic decision** tagged
  `PSEUDO`, per the §"defaults" table. Fold consumes real + synthetic decisions identically; each resulting
  disposal/lot/row carries a `pseudo_origin: bool` (or a set of synthetic decision-ids) so render can flag it.
  Real decisions are collected FIRST → an event with any real decision gets NO synthetic default (real
  supersedes; no conflict, no void).
- **Determinism (NFR4):** synthetic decisions get deterministic ids/order derived from the target event id (no
  `Date::now`/RNG); the injection is a pure function of (events, real-decisions, mode).

## The defaults (only where no real decision exists) — from the settled brainstorm
| Blocker / event | Synthetic default |
|---|---|
| `UnknownBasisInbound` (inbound, unknown basis) | `ClassifyInbound(SelfTransferMine{ basis:$0 })` |
| `Unclassified` inbound | same — zero-basis self-transfer (never income; assumption 3) |
| `TransferOut` withdrawal (unmatched/unclassified) | non-taxable self-transfer (no Sell/Gift/Spend). A `Dispose(Sell)` from import stays a taxable disposal — pseudo does NOT touch it (its gain uses sub-1's attested method, default FIFO). |
| Outbound network fee | de-minimis: drop the fee sats; basis stays with held coins; NO re-homing (TP8-c intent). |
| `DecisionConflict` / `ImportConflict` | **[aggressive scope]** accept-first (a synthetic accept of the first-seen import/decision). |
| `TaxProfileMissing` / `TaxYearNotComputable` | **[aggressive scope]** a synthetic PLACEHOLDER profile (filing `single`, $0 ordinary income / MAGI / qualified dividends) so `report --tax-year` computes an estimate. |
| method for pseudo Sells | sub-1's resolver (scoped election → global → FIFO). |
Net: all movement non-taxable, ~zero tax → an obviously-fictional "null-hypothesis" ledger.

## Guard (this sub-project's half; the export/forms attest gate is sub-project 3)
- **On-screen `[PSEUDO]` flags + a banner** wherever pseudo defaults contribute: `verify` (a `PseudoReconcileActive`
  ADVISORY blocker + per-row markers), `report`, and the TUI. **NEVER in any output file** (export CSVs / forms
  stay clean — sub-3 gates their production behind the typed attestation).
- Expose a queryable **"is any pseudo default contributing?"** signal (count of synthetic decisions in the
  projection) for sub-3's gate + the banner.

## Bulk-approve + revert
- `reconcile pseudo approve` (+ a TUI flow, + a filter e.g. by default-type/wallet/year): materialize the
  selected synthetic defaults as REAL decision events (reuse the bulk-reconcile append machinery:
  empty-guard + mid-batch rollback + single save). After approval they are real/attested → no longer `PSEUDO`.
- `reconcile pseudo off` — clears the mode flag; the projection reverts to real-only (0 fictional events were
  ever written, so revert is instant + total). Already-approved (materialized) decisions REMAIN (they're real
  now — that's the point).

## Tax-safety invariants (fault-inject each)
- Mode OFF ⇒ projection byte-identical to today (no synthetic decisions; a KAT diffs a fixture snapshot).
- A REAL decision on an event ⇒ NO synthetic default for it (real supersedes; fault-inject: break the
  precedence → the real decision is ignored → RED).
- **[★] pseudo flags appear on-screen (verify/report) and are ABSENT from every export/form file** — the
  headline guard. Fault-inject: a KAT that greps the export CSVs/forms for any `PSEUDO`/synthetic marker and
  asserts NONE, while the on-screen render DOES carry it.
- Synthetic decisions are NOT persisted (a KAT: after projecting in pseudo mode, `load_all` shows no new
  events; only `approve` writes events).
- Determinism: two projections in pseudo mode are byte-identical.

## Scope / SemVer / lockstep
btctax-core (resolve injection + `pseudo_origin` threading + the placeholder-profile/accept-first defaults) +
btctax-cli (`reconcile pseudo on/off/approve`, config flag, render `[PSEUDO]`) + btctax-tui-edit (banner +
approve flow). Additive; mode-off is byte-identical (PATCH-class behavior-preservation, but MINOR for the new
subcommands). Lockstep: `make docs`, `?`-overlay, doc-comments. NO GUI schema_mirror (no GUI crate).

## Plan (TDD)
- **T1** — mode flag (config) + `reconcile pseudo on/off`; mode-off-is-identical KAT.
- **T2** — resolve injection pass + `pseudo_origin` threading through fold to disposals/lots; real-supersedes +
  not-persisted + determinism KATs.
- **T3** — the defaults (self-transfer/$0, accept-first, placeholder profile, de-minimis fee); per-default KATs;
  the null-hypothesis "≈zero tax, 0 classification-blockers" end-to-end KAT.
- **T4** — render `[PSEUDO]` on verify/report + the `PseudoReconcileActive` advisory + the ★ on-screen-yes /
  output-no fault-inject KAT.
- **T5** — bulk-approve (CLI + reuse bulk machinery) + revert; approve-materializes-real + revert-is-total KATs.
- **T6** — btctax-tui-edit banner + approve flow; `make docs`; whole-diff review + full suite + FOLLOWUPS.

## Gotchas
- **Synthetic decisions must NEVER be persisted by projection** — only `approve` writes. A projection that
  writes events would corrupt the ledger. Pin with a not-persisted KAT.
- **[★] flags on-screen, clean output** — the load-bearing guard; the fault-inject KAT is mandatory.
- **Real supersedes pseudo** — collect real decisions first; an event with a real decision gets no default.
- **Sells are NOT self-transfers** — only `TransferOut` withdrawals default to self-transfer; imported
  `Dispose(Sell)` stays taxable (uses sub-1's method).
- **Aggressive scope is fiction** — accept-first + placeholder profile are guesses; they MUST be flagged +
  gated by sub-3 before any output.
