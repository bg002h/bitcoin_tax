# Review — SPEC_foundation.md v0.4, Engineering (Round 4)

- **Artifact:** `design/SPEC_foundation.md` (v0.4)
- **Reviewer:** independent engineering/spec reviewer, fresh context, adversarial. (First round-4 attempt timed out; this is the retry.)
- **Date:** 2026-06-28
- **Verdict:** **0 Critical, 2 Important** + 4 Minor / 4 Nit. Round-3 set genuinely folded, but the I-NEW-2 fix is mechanically present yet substantively defective (wrong comparison date), and the m1 Void-ordering fold contradicts the new irrevocability model. Not yet 0/0.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

---

## Round-3 verification (against v0.4 text)
- **I-NEW-1 (ImportConflict identity) — RESOLVED.** §6.2 `f("conflict",source,source_ref,new_fingerprint)`, separate namespace; §6.4 moved to "System-generated events"; Supersede/Reject keep target `EventId`; re-import idempotent; §13 KAT. (Edge in MINOR-1: multiple concurrent conflicts per target.)
- **I-NEW-2 (safe-harbor override) — PARTIAL → IMPORTANT-1.** FR7 "iff effective" removes the contradiction; override persisted as `pre_disposition_attested`; time-bar is a projection blocker. **But the comparison date is wrong (IMPORTANT-1).**
- **m1 (Void resolved-before-fold) — FOLDED but → IMPORTANT-2.**
- **m2 determinism wording — RESOLVED.** **m3 origin_event_id — RESOLVED.** **m4 Σ in — RESOLVED.** **m5 conservation precondition — RESOLVED.** **m6 dual_loss_basis=None — RESOLVED.** **m7 KATs — MOSTLY** (gap: second *distinct* change to same target). **n1–n4 — RESOLVED.**
No regressions in the solid items.

## CRITICAL — None. Two-pass model sound and terminating.

## IMPORTANT

### IMPORTANT-1. Safe-harbor time-bar compares against `effective_date` (=2025-01-01) instead of the allocation's made/declaration date → the disposition prong is a dead rule, and §7.4 contradicts the §13 KAT.
§7.4 guard (1): allocation inert if the effective timeline shows a first 2025 Sell/Spend/Transfer/Gift/Donation "**before the allocation's effective date** and `pre_disposition_attested==false`." But `effective_date` is fixed to **2025-01-01** (§6.4). A "first 2025 event" is by definition on/after 2025-01-01, so it can never be "before 2025-01-01" → the inert/`safe_harbor_timebar` branch never triggers from the disposition prong. The legally-correct comparison is the allocation **decision's creation time** (`utc_timestamp`, §6.3 "decisions: creation time") vs. the first 2025 disposition — RevProc 2024-28 §5.02(4) bars allocations *established* after the first disposition; `effective_date` (the as-of snapshot) is a different concept. Also contradicts the §13 KAT ("unattested + **prior 2025 disposition** → inert/Path A") — "prior" only makes sense against the made-date. An implementer coding to §7.4's letter ships a no-op safety rule on an irrevocable election. **Fix:** compare against the allocation event's `utc_timestamp` (made-date); disambiguate the overloaded "effective" (rename the field).

### IMPORTANT-2. §7.2's "remove the void's target from the applied set" contradicts FR8/§6.4's "Void does not apply to an effective `SafeHarborAllocation`," and the phasing is infeasible as stated.
§7.2 (m1 fold): "first compute the non-voided decision set … then apply." But FR8/§6.4: a Void "revokes a *revocable* decision (not an effective `SafeHarborAllocation`)." Allocation-effectiveness depends on the time-bar (IMPORTANT-1) → on the first 2025 disposition → which can be produced by a `ReclassifyOutflow` decision. So effectiveness is a function of the *resolved* decision set, which §7.2 only produces *after* the void-subtraction — circular for an allocation-targeting Void. Defects: (a) a literal impl drops an effective (irrevocable) allocation when Void targets it (violates FR8); (b) "Void of an effective allocation" outcome unspecified. **Fix:** state the phasing — resolve all non-allocation-void decisions → build effective timeline → determine first 2025 disposition → evaluate allocation effectiveness → only then adjudicate an allocation-targeting Void (effective → reject Void as `decision_conflicts`; inert → apply). Note the deterministic consequence: reclassifying a 2025 outflow away from `Dispose` can flip an allocation inert→effective, freezing a previously-voided allocation — state the intended outcome.

## MINOR
1. **Multiple concurrent `ImportConflict`s on one target — interaction unspecified.** §6.2 permits N pending conflicts/target; spec never says how two `SupersedeImport`s (on different conflicts of the same target) or a `Supersede`+`Reject` interact. Specify "latest `decision_seq` governs the target payload" vs. flag; add §13 KAT for a *second distinct* change (m7 only covers identical re-import).
2. **Return-due-date deadline prong not operationalized.** §7.4 names two deadlines but only the disposition prong gets a projection rule. The 2025 return due date is an objective date the app knows (unextended ≈2026-04-15 has passed; extended ≈2026-10-15 hasn't); the app can't know if an extension was filed → this prong needs its own attestation/assumption. Specify it.
3. **Unknown-basis (date-unknown) `GiftReceived` vs conservation.** §7.3 "… or a `unknown_basis_inbounds` flag if the date is unknown" can read as "flag instead of creating a lot," but FR9 counts classified `GiftReceived` in `Σ in` → it must still create a sat-bearing, **basis-pending** lot (symmetric with Income-`Missing`). Reword.
4. **`blockers` conflates hard blockers with advisory notes.** §7.1 lumps `pre2025_method_note`/`safe_harbor_timebar` (Path A is valid → ledger usable) with hard blockers (`fmv_missing`/`uncovered_disposal`/`import_conflicts`). Mark which halt downstream vs. advisory.

## NIT
1. Voiding a Void / a `Supersede`/`Reject` — unspecified. State the rule (recommend: voids/supersedes/rejects are not themselves revocable).
2. `ImportConflict` ordering identity — neither folded nor `decision_seq`-keyed; clarify it carries the target's `source_ref` and is not folded in pass 2.
3. `split_sequence` for allocation-seeded lots — state it equals the index into the event's `lots` array (deterministic).
4. "effective" overloaded in §7.4 (governing vs `effective_date` field) — rename the field (e.g., `as_of_date`).

## Genuinely solid (do not regress)
Distinct conflict-identity namespace; two-pass resolve-then-fold + `decision_seq` order; `LotId` origin pinning (all four paths); conservation `Σ in` scoping + no-uncovered precondition; total three-zone dual-basis; persisted attestation as an event field (right shape); time-bar as a pure-projection re-evaluated rule; KAT/property/idempotency/determinism mandates.

## Verdict
**0 Critical, 2 Important, 4 Minor, 4 Nit — not yet 0/0.** IMPORTANT-1 (time-bar compares the wrong date — defective I-NEW-2 fix, contradicts §13) and IMPORTANT-2 (Void-ordering vs irrevocability contradiction, infeasible phasing). Both localized: a date-field substitution + §7.4/§13 reconciliation; one §7.2 phasing paragraph + a stated Void-of-effective outcome. Fold + re-review → v0.4 should reach 0/0.
