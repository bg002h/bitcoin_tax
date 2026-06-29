# Review — SPEC_foundation.md v0.5, Engineering (Round 5)

- **Artifact:** `design/SPEC_foundation.md` (v0.5)
- **Reviewer:** independent engineering/spec reviewer, fresh context, adversarial.
- **Date:** 2026-06-28
- **Verdict:** **0 Critical, 0 Important** + 4 Minor / 3 Nit. Both round-4 Importants substantively resolved; all round-4 Minors/Nits resolved. **Sound foundation to proceed to the implementation plan.**
- Persisted per STANDARD_WORKFLOW §2.

---

## Round-4 verification (against v0.5 text)
- **IMPORTANT-1 (time-bar wrong date) — RESOLVED.** `effective_date`→`as_of_date` (fixed 2025-01-01); deadline reference = made-date (`utc_timestamp`). §7.4 guard (1) fires when a first-2025 event is "earlier than the allocation's made-date" — the legally correct §5.02(4) comparison; dead-rule gone. §7.4 ↔ §13 KAT now agree (incl. self-transfer-doesn't-trip).
- **IMPORTANT-2 (Void-vs-irrevocability circularity) — RESOLVED.** §7.2 pass-1 staged 1→4 (apply non-allocation decisions → effective timeline + first-2025 trigger → evaluate allocation effectiveness → adjudicate allocation-targeting Voids). Strict pipeline, no backward dependency → non-circular, deterministic. FR8/§7.3: void-of-effective → `decision_conflicts`; inert IS voidable; inert→effective-flip consequence stated + KAT'd.
- Minors 1–4 and Nits 1–4 (round 4) — all RESOLVED. No regressions in previously-solid items.

## CRITICAL — None.
## IMPORTANT — None. Both round-4 Importants correctly resolved; staged pass-1 and made-date/as_of_date split sound, non-circular, deterministic.

## MINOR (new — non-blocking)
1. **Conservation sub-check (§7.4(3)) needs the pre-2025 pool fold as a prerequisite.** §7.2 step 3 evaluates "+ conservation" in pass 1, but "remaining held sat/pool basis as of 2025-01-01" is folded (pass-2-like) state. Not circular (pre-2025 pool is independent of the allocation and of 2025+ events → computable as a prerequisite), but say so — e.g., "the pre-2025 pool fold is a prerequisite to step-3 conservation."
2. **A conservation/eligibility-failed inert allocation has no distinct blocker — mislabeled `safe_harbor_timebar`.** Inert can mean time-barred (Path A genuinely valid, benign) OR conservation-failed/dealer-ineligible (a real error). Per "nothing silent" (§12) + NFR6, give guard-(3)/(4) failures a distinct blocker (e.g., `safe_harbor_unconservable`), separate from the time-bar advisory.
3. **Time-bar trigger counts only *confirmed* dispositions → an unreconciled 2025 outflow can leave an allocation provisionally Path-B-effective (anti-conservative).** A 2025 `TransferOut` in `pending_reconciliation` doesn't trip prong (a) until classified `Dispose`; meanwhile it's only the advisory `unmatched_outflows`. Treat an unresolved 2025 outflow as a potential trigger or add an "effectiveness provisional pending reconciliation" advisory. Low current-date impact (prong (b) already forces today's non-attested allocations inert).
4. **Date basis/granularity for the made-date comparison + the 2025 boundary unspecified (UTC vs `original_tz`).** Not a determinism break (fields are fixed), but two compliant impls could differ at the boundary; legally-correct basis = tax calendar date (`original_tz`). Specify basis + granularity once; testable.

## NIT (new)
1. No direct KAT for "void an *inert* allocation → Void applies (dropped, stays Path A)." Add to pin both step-4 branches.
2. "effective" still used in three senses (effective timeline / effective allocation / renamed snapshot) — load-bearing collision resolved by the field rename; optional polish.
3. Unextended 2025 return due date referenced but the constant isn't pinned (TY2025 = 2026-04-15). Pin it or route to FOLLOWUPS.

## Verdict
**v0.5 is at 0 Critical / 0 Important.** Round-4 IMPORTANT-1 and IMPORTANT-2 are substantively fixed (live legally-correct made-date rule consistent with the §13 KAT; non-circular deterministic staged pass-1 with void-of-effective → `decision_conflicts`). All round-4 Minors/Nits resolved. The seven new findings are Minor/Nit and non-blocking; most worth folding: Minor-1 (state the pre-2025 pool fold prerequisite) and Minor-2 (distinct blocker for conservation/eligibility failure vs the time-bar advisory). **Sound foundation to proceed to the implementation plan**; fold Minors 1–2 and log the rest to FOLLOWUPS.
