# Whole-Branch Review — `btctax-core` (4c1b0f2..3b30f19, 22 commits) — Round 1

- **Reviewer:** independent final whole-branch reviewer (most-capable model); traced multi-event lifecycles end-to-end, two-pass seam, conservation, determinism, spec/legal faithfulness.
- **Date:** 2026-06-29
- **Verdict:** **NOT ready to merge — 0 Critical, 2 Important** (both NEW cross-task findings, not in the per-task lists). Recorded Minors: none merge-blocking. Persisted per STANDARD_WORKFLOW §2.

## Critical — None.
Default/common path correct: FIFO, TP2 fee-reduces-proceeds, TP4 day-after/>1yr, TP8(c) full-basis-carry (never flips), TP10 zero-gain removal, TP11 four-zone dual-basis, Path A/B transition + time-bar. Golden cross-boundary KAT + 3 proptests (256 cases) confirm Σsat/Σbasis conservation through fee'd self-transfers.

## Important (merge-blocking)
### I-1 — `ReclassifyOutflow → Dispose` silently drops the outflow's on-chain `fee_sat`
`resolve.rs` `OutflowClass::Dispose` arm builds `Op::Dispose { sat: t.sat, proceeds, fee_usd, kind }` and never reads `t.fee_sat`; `Op::Dispose` has no `fee_sat` field; the fold arm consumes only `*sat`. Every OTHER outflow path accounts the fee sats (PendingOut consumes sat+fee_sat; SelfTransfer/GiftOut/Donate call `consume_fee`). A `TransferOut{sat:100_000, fee_sat:Some(200)}` reclassified Sell/Spend disposes 100k and leaves 200 fee-sats in the pool → holdings overstated by fee_sat; the fee-sat disposition (spending BTC to pay a miner fee IS a disposition) omitted; the phantom sats land in Σheld so `conservation_report` returns `balanced==true`/`has_uncovered==false` — the verifier itself is masked. Silent (no blocker) → violates §12. Spec §6.4/§7.3 say apply "fee per TP8" to the Dispose variant too. Only reclassify→Dispose KAT uses `fee_sat: None`. **Fix:** carry `t.fee_sat` into `Op::Dispose` and route it through the TP8 `consume_fee` mechanism (under (c) the fee-sat basis rolls into the disposal legs; under (b) a mini-disposition) — mirroring the gift/donation arms; add a fee'd reclassify→Dispose KAT.

### I-2 — Path-B safe-harbor seeded lots collide on `LotId` after a post-2025 self-transfer
Path-B seed lots use `lot_id.split_sequence = i` (allocation index) over `origin = allocation EventId`, pushed via `push_lot`, which does NOT initialize `next_split[allocation_id]`. A later `SelfTransfer` relocation calls `bump_split(origin)` → `or_insert(0)` → 0, colliding with seed lot index 0. (Acquire/Income/Path-A use `new_origin_lot` which pre-increments.) Two distinct lots share one `LotId`. No Phase-1 numeric error (sats/basis/gains/conservation correct, deterministic) but breaks §6.2 LotId-uniqueness → ambiguous for audit/Phase-2. Silent. Reachable via effective Path-B + a 2025+ TransferLink partially relocating a seeded lot. Untested. **Fix:** after seeding Path-B lots, `next_split[allocation_id] = seed.len()`.

## Minor (defer to FOLLOWUPS — none merge-blocking)
ManualFmv ignored for ClassifyInbound::Income (gated basis_pending, no wrong number); gift known-donor_basis but donor_acquired_at=None silently uses gift-date HP (no tacking; spec-faithful); Path-B AllocLot can't carry dual-basis (spec-faithful); unchecked Decimal/i64 in fold arms (recorded Task-4); missing-wallet mislabeled FmvMissing (recorded Task-6); TreatmentB fee shortfall FMV on requested not consumed (recorded Task-11 M2); effective-Path-B+zero-2025-events seed never fires (recorded Task-12); recorded Task-5/9/10/12 minors (all gated/cosmetic).

## Nit
Persistence fingerprint separators (recorded Task-3); make_removal_legs term for onward-gifted dual lots (Phase-2).

## Cross-cutting checks PASSED
Two-pass seam (universal_snapshot reuses fold_event + same canonical sort; stable tax-date partition; acyclic). Determinism NFR4 (no HashMap; total canonical order; finalize sorts; tie-break tests). TP8(c) hard default never flips; Task-11 M1 guard refuses to promote non-dual survivor (proven). FR9 conservation (fee-sats sole home; (b) mini-dispositions excluded; no double-count). Persistence/FR1 idempotency sound.

## Verdict (Round 1)
**Not ready to merge. 0 Critical / 2 Important (I-1, I-2).** Both are new cross-task findings — exactly what the whole-branch gate is for. Fix both, add both to FOLLOWUPS, re-review the fold per §2.

## Round 2 — fix re-review (commit 77efa7e) — BOTH CLOSED, GREEN
Independent re-review of the fix (diff 3b30f19..77efa7e). **I-1 and I-2 FULLY CLOSED; fix sound. 0 Critical / 0 Important / 0 Minor / 3 Nit.**
- **I-1:** `Op::Dispose` now carries `fee_sat`; the reclassify arm passes `t.fee_sat` (native Dispose → `None`, clean no-op). The fold arm consumes the fee-sats then routes them through the existing TP8 `consume_fee` + `rehome_onto_disposal_leg` — under (c) the fee-sat basis rolls onto the last disposal leg (gain reduced: KAT pins net $149.00, basis $60.00, gain $89.00), under (b) a `fee_mini_disposition` record. Fee-sats consumed (holdings no longer overstated), `conservation_report` now HONEST (KAT: `balanced`, 99_800 + 200 == 100_000), no double-count, `rehome_onto_disposal_leg` touches only basis/gain (no dual-basis corruption). Existing disposal KATs unaffected.
- **I-2:** `init_split_counter(origin, seed.len())` after Path-B seeding → a later `bump_split` returns `seed.len()`, beyond the seed range. KAT (effective Path-B + partial self-transfer of a seeded lot) asserts all LotIds unique + conservation balanced.
- Nits (non-blocking): init_split_counter single-origin assumption (safe — one origin per allocation; add an assert); UncoveredDisposal blocker-kind for the unreachable fee-rehome guard; TreatmentB mini-disposition FMV uses the reclassification date not the on-chain fee date (FOLLOWUPS Phase-2).

**Net status: 0 Critical / 0 Important — btctax-core GREEN, ready to merge.**
