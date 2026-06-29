# Review — SPEC_foundation.md v0.3, Engineering (Round 3)

- **Artifact:** `design/SPEC_foundation.md` (v0.3)
- **Reviewer:** independent engineering/spec reviewer, fresh context (adversarial). Round-2 findings verified against v0.3 text; legal cites spot-checked.
- **Date:** 2026-06-28
- **Verdict:** 0 Critical, **2 Important** (both newly introduced by the round-2 folds) + 6 Minor / 4 Nit. Round-2 set genuinely resolved; two-pass determinism model (round-2 Critical) sound and terminating; no regressions.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

---

## Round-2 verification (against v0.3 text)
- **C2-1 (decision identity + two-pass + determinism) — RESOLVED.** §6.2 `decision_seq` → `EventId=f("decision",decision_seq)` (separate namespace, no collision). §7.2 real two-pass: pass 1 resolves decisions in `decision_seq` order onto an effective imported timeline; pass 2 folds canonically; the 2026-rewrites-2022 case handled correctly; total order well-defined, terminating. NFR4 updated.
- **I2-1 (ImportConflict + reject) — PARTIAL.** Variant + Supersede/Reject + fold rules + FR1/FR8 added, **but conflict identity collides — I-NEW-1.**
- **I2-2 (conservation) — RESOLVED.** FR9 `+ Σ removed`; §13 updated; gift/donation fee_sat → fee-sats term.
- **I2-3 (uncovered totality) — RESOLVED.** §7.1 total; §7.3 Totality clause (incl. SupersedeImport below-consumed) → `uncovered_disposal`; §13 KAT + property test.
- **I2-4 (guard trigger) — PARTIAL.** Broadened to Sell/Spend/Transfer/Gift/Donation + incl. extension (verbatim-correct vs §5.02(4)), **but override contradicts FR7 + un-modeled — I-NEW-2.**
- **I2-5 (dual-basis gifts) — RESOLVED.** §6.3 `dual_loss_basis`+`donor_acquired_at`; §7.3 three-zone matches §1015(a)/§1223(2); TP11 + §13 KAT.
- Eng minors/nits + Tax R2 minors/nits — RESOLVED.
No regressions.

## CRITICAL
None. Two-pass model sound, terminating, correctly handles retroactive corrections.

## IMPORTANT

### I-NEW-1. `ImportConflict` has no defined identity and collides with its target's `EventId` → accept/reject lifecycle + conflict idempotency unimplementable.
`ImportConflict` is listed under "Imported events (from adapters)" (§6.4); the only imported `EventId` rule is `f(source, source_ref)` (§6.2). But a conflict carries the **same `source_ref` as its target**, so `f(source,source_ref)` = the **target's `EventId`** — a collision in a log keyed on unique `EventId`s (NFR6). `SupersedeImport{conflict_event}`/`RejectImport{conflict_event}` reference the conflict by `EventId`, and §7.3 says Supersede "replace[s] target payload (same `EventId`)" — so conflict and target must be distinct addressable events, which the scheme can't deliver. Also breaks §13 idempotency ("a changed row → one `ImportConflict`"; re-import of the identical changed row = no-op) and has no disambiguation for a second distinct change. Pin a distinct identity, e.g. `f("conflict", source, source_ref, new_fingerprint)` (makes re-import idempotent; a later distinct change → separate conflict), or model as a sequence-keyed event.

### I-NEW-2. Safe-harbor override path contradicts FR7 and is not modeled as an event → pure projection can't honor it deterministically.
FR7: "If a `SafeHarborAllocation` exists it governs (path B); otherwise path A." §7.4 guard (1): even when an allocation exists, if the ledger shows a 2025 disposition/transfer before its effective date, "warn + require explicit user override … and default to Path A." For the un-overridden preceding-disposition case these give opposite answers (B vs A) — contradiction. The "explicit user override" is unmodeled: no override field/event, no blocker. Since projection is pure-from-events and the guard is phrased at projection level ("If the ledger shows…"), an out-of-band CLI override is invisible on re-derivation → a legitimate override is silently dropped to Path A on every rebuild (NFR4 break). Also a 2025 disposition imported *after* the allocation can retroactively time-bar it, which a command-time check never re-evaluates. Reconcile FR7 vs §7.4; make the time-bar a projection blocker; persist the override as an event/field so the pure projection is deterministic and auditable.

## MINOR
1. **Pass-1 `Void` ordering under-specified.** A `VoidDecisionEvent` always has higher `decision_seq` than its target, so a literal single forward pass would apply then drop. State: compute the non-voided decision set first, then apply the remainder in `decision_seq` order.
2. **"Shuffled decision-append order → identical state" ambiguous.** `decision_seq` is the causal order; the property that holds is invariance to storage/load order with `(decision_seq, payload)` fixed. Specify the test that way (re-assigning `decision_seq` legitimately changes results).
3. **`origin_event_id` for non-`Acquire` lots unpinned.** Pin the origin `EventId` for `Income` lots, `GiftReceived` lots (decision-produced), Path-B `SafeHarborAllocation`-seeded lots, and Path-A reconstructed lots — for `LotId` stability.
4. **FR9 LHS `Σ in` undefined w.r.t. unclassified/unlinked inbounds + self-transfer destinations.** `Σ in` must count only externally-sourced acquisitions (Acquire/Income/classified-GiftReceived), excluding unclassified/unlinked inbounds; confirm `TransferLink` consumes the destination `TransferIn` (so it's not double-counted and leaves `unknown_basis_inbounds`).
5. **Uncovered-disposal vs conservation precondition.** When `uncovered_disposal` fires, `Σ disposed` can exceed available sats; condition the §13 conservation test on "no `uncovered_disposal`," or track the shortfall as a term.
6. **`dual_loss_basis = None` branch unstated** (FMV-at-gift ≥ donor basis → single-basis carryover, HP tacks). Add it so the three-zone rule is total.
7. **Testability gaps:** no KAT for the C2-1 motivating case (late `ReclassifyOutflow`/`SupersedeImport` rewriting an earlier tax year + determinism), `decision_conflicts`, `Void` round-trip determinism, re-conflicting the same target twice, or the safe-harbor time-bar/override. Add these.

## NIT
1. `ClassifyRaw` should state it preserves the target's `EventId` (symmetric with Supersede).
2. "basis fields split pro-rata" (§6.3) should explicitly include `dual_loss_basis` (`donor_acquired_at` is a date, doesn't split).
3. Define "effective" `SafeHarborAllocation` (FR8/§7.4 hinge on it): is a guard-failed/defaulted allocation voidable or inert?
4. TP8(b) applied to a gift's `fee_sat` → taxable mini-disposition on a non-recognition transfer — defensible but odd; a one-line note helps. Default (c) sane.

## Genuinely solid (do not regress)
Two-pass resolve-then-fold + `decision_seq` total order; stable import `EventId`/`source_ref` + decision `EventId` separate namespace; `LotId=(origin,split_seq)`; conservation `+ Σ removed`; dual-basis `Lot` + three-zone (§1015(a)/§1223(2)); total projection w/ `uncovered_disposal`; broadened+extension-aware safe-harbor trigger (verbatim vs §5.02(4)); txid-as-match-signal; pending@snapshot exclusion; KAT/property/idempotency/determinism mandates.

## Verdict
**0 Critical, 2 Important** (I-NEW-1 ImportConflict identity; I-NEW-2 safe-harbor override) + 6 Minor / 4 Nit. Every round-2 finding genuinely resolved, two-pass determinism sound. Both new Importants are localized (one identity rule; one FR7/§7.4 reconciliation + override event). Fold + re-review the diff → v0.3 should reach 0/0; otherwise a sound foundation for the plan.
