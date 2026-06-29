# Tax-Correctness Review — IMPLEMENTATION_PLAN_foundation_02_core.md — Round 1

- **Reviewer:** independent US-federal-crypto-tax reviewer; verified vs legal REPORT + ADDENDUM + SPEC TP1–TP11 / §7.3 / §7.4.
- **Date:** 2026-06-28
- **Verdict:** **NOT tax-correct as written — 1 Critical (C1), 0 Important.** TP8 default is correctly (c) and never silently flipped to (b); C1 is a basis-arithmetic bug *within* (c). Persisted per STANDARD_WORKFLOW §2.

## CRITICAL
### C1 — TP8(c) self-transfer (and gift/donation) network fee: basis is DROPPED, not carried (Task 8 + Task 11)
Spec (TP8, §24): DEFAULT (c) = `fee_sat` consumed at zero proceeds (non-taxable); **full basis carries**. User mandate: "non-taxable, basis carries across the self-transfer."
Plan implements (c) in two steps that LEAK basis:
- Task 8 `Op::SelfTransfer`: `consume_fifo(principal)` → relocated lots, basis split pro-rata by sat (Task 4).
- Task 11 (c): consume `fee_sat` FIFO at zero proceeds, recorded in `fee_sats_consumed`, No Disposal.
Worked example (plan KAT): buy 100,000 sat for $60.00; transfer 99,800 principal + 200 fee.
- `consume_fifo(99,800)` → relocated basis = split_pro_rata($60, 99_800, 100_000) = **$59.88**; source remainder 200 sat / $0.12.
- Task 11 (c) consumes the 200 fee-sat at zero proceeds → the **$0.12 basis is permanently destroyed**. Destination lot = 99,800 sat @ **$59.88**, not the mandated **$60.00**.
Why it's a wrong tax result: (1) systematically understates basis → overstates future gain on surviving coins on every fee-bearing self-transfer/gift/donation, accumulating; (2) contradicts spec "full basis carries" and the user mandate — (c) behaves like (b) minus recognition (discards basis with neither recognized loss nor carry); (3) violates the plan's own Σbasis conservation invariant (Task 13). FR9 *sat* conservation still holds (fee_sats absorbs the sat) so it's a basis-only leak invisible to sat tests.
Test gap masking it: the (c) KAT asserts only `holdings_by_wallet[&cold]==99_800` and `disposals.is_empty()` — never the destination lot's `usd_basis`. The Σbasis proptest generators don't synthesize `TransferLink`-confirmed fee'd self-transfers.
**Remediation:** under (c), carry the consumed lots' FULL basis to the destination (consume principal+fee as one relocation; destination sat = principal only; basis = full consumed basis) OR redistribute the fee-sats' pro-rata basis onto the relocated destination lots. Add a KAT asserting destination lot `usd_basis == dec!(60.00)` (+ gift/donation analogue); extend the Σbasis proptest to include fee'd `TransferLink`s. (If "drop fee basis" were ever intended for (c), that needs an explicit spec change + an explicit Σbasis carve-out — currently plan ≠ spec, so blocking.)

## IMPORTANT — None.

## MINOR (mostly acceptable-as-documented; no Phase-1 wrong number)
- M1 Feb-29 holding-period anniversary → Feb-28 fallback (documented convention; rare edge).
- M2 Donation `appraisal_required` decision-supplied, not engine-derived (correctly deferred to Phase-2 forms; ensure flag/FMV/term always preserved).
- M3 FMV-missing disposal emits basis-0/full-proceeds gain gated by hard blocker (by design §7.3; ensure Phase-2 honors `FmvMissing` and never treats basis-0 as final).
- M4 Path-B ProRata effectiveness requires `timely_allocation_attested` (faithful to §7.4; conservative — denies to taxpayer's non-detriment, falls back to valid Path A).
- M5 Path-B conservation guard requires Σbasis == engine-reconstructed Universal pool basis → hard `safe_harbor_unconservable` (per spec; may over-flag users with more complete books than imports; produces a blocker, not a wrong number).

## NIT
- N1 Daily-close FMV vs dominion-and-control instant — adapter/price concern (Plan 3), not core. Core correctly recognizes income at the event `tax_date` FMV.
- N2 `original_tz` fixed `UtcOffset` (no IANA/DST) — near-midnight DST corner case; day-granularity adequate (FOLLOWUPS).

## Confirmations (verified correct vs law + spec)
TP2 basis=cost+acq fee, disposition fee reduces proceeds (pro-rata, remainder-takes-rest), per-lot. TP4 holding period off-by-one correct (disp > one_year_after(acq); same-day ST; on `original_tz` calendar date). TP1/TP3 realization: sale/spend→Disposal; gift/donation→Removal zero gain; income FMV on receipt = basis, HP next day. TP5 FIFO default + LotId specific-ID-ready + partial-disposition splitting. **TP8 default = (c) CONFIRMED, no drift** — ProjectionConfig::default()=TreatmentC with "do not flip" guard; (b) opt-in only; TransferLink never becomes a disposition; only ReclassifyOutflow{as:Dispose} disposes. TP10/TP11 gift/donation: removal w/ basis+FMV+ST/LT+donor/appraisal metadata, zero gain; received-gift dual basis all 4 zones (gain→carryover+tack; loss→FMV-at-gift, HP from gift date no tacking; between→no gain/no loss; unknown→GiftFmvFallback/basis_pending). §7.4 transition faithful (UniversalPool→PerWallet; Path A default vs Path B method-keyed deadline/attestation/conservation/capital-asset/irrevocable/provisional; (c) excluded from trigger, (b) included). TP9 wash sale correctly N/A. Income FMV + business:bool carried for SE routing. De minimis: gain on every disposition, no threshold.

**Bottom line:** Fix C1 (TP8(c) basis carry) + add destination-basis KAT + fee'd-transfer Σbasis proptest → 0 Critical / 0 Important. TP8 default correctly (c), not at risk of a (b)-flip.

## Round 2 (fold re-review) — C1 CLOSED, 0 Critical / 0 Important
Verified against the actual `consume_fee`/`FeeCarry`/`consume_fifo`/`split_pro_rata` code + KATs. C1 closed: under (c) the full $60.00 basis carries (destination 99,800 sat @ $60.00; (c) KAT asserts `usd_basis == dec!(60.00)`); re-homing preserves SAT conservation (fee sats only in `fee_sats_consumed`, never on a lot) and gain-BASIS conservation (a move, net-zero) across single-lot, multi-lot, cross-boundary, and shortfall cases; gift/donation analogue correct (no phantom disposition); (b) opt-in unbroken ($59.88 contrast KAT); TP8 default still (c); Σbasis proptest now exercises fee'd `TransferLink`s. **No new Critical/Important.**
New Minors (→ FOLLOWUPS, non-blocking, taxpayer-conservative cents-scale): **m1** loss-basis (dual-gift) carry dropped when a fee spans a non-dual `relocated.last()` while originating on a dual-basis lot (rare; gain basis fully conserved); **m2** exact-boundary fee attribution rides a different holding period by a few cents; **m3** degenerate `principal==0` fee'd transfer drops the carry with no blocker ("documented, not silently lossy" is self-contradictory — assert principal>0 or raise `uncovered_disposal`). Nit: add a `has_uncovered` guard to the C1 proptest for style parity.
