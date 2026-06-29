# Review — SPEC_foundation.md v0.2, Engineering (Round 2)

- **Artifact:** `design/SPEC_foundation.md` (v0.2)
- **Reviewer:** independent engineering/spec reviewer, fresh context (adversarial). Round-1 findings checked against actual v0.2 text; legal cites spot-verified against `legal/text/irs-guidance/RevProc_2024-28.txt` and the addendum.
- **Date:** 2026-06-28
- **Verdict:** 0/0 NOT yet met — **1 Critical + 5 Important** (most from the v0.2 fold). Round-1 set largely resolved. All localized.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

---

# Independent Engineering/Spec Review (Round 2) — `design/SPEC_foundation.md` (v0.2)

**Reviewer posture:** adversarial, fresh eyes, greenfield-on-merits. I checked each round-1 engineering finding against the actual v0.2 text (not the §18 fold record), then hunted for fold-introduced defects in the expanded taxonomy, the stable-EventId scheme, the two 2025 paths, and testability.

**Verdict up front:** v0.2 is a large, mostly-faithful improvement — the architecture findings, and engineering C-2/C-3/C-4, I-2/I-3/I-5/I-6/I-7/I-8/I-9 and all minors/nits are **genuinely resolved**. But the C-1 identity fix was applied **only to imported rows** and not extended to the decision/correction events that now reference it everywhere, and the Removal/Disposal split introduced a conservation defect. **v0.2 is NOT yet at 0 Critical / 0 Important.** 1 Critical + 5 Important remain.

## Round-1 verification (abbrev.)
C-1 **Partially** (imported rows only — decision events left out → Critical below). C-2/C-3 Yes (core; guard trigger incomplete — Important #4). C-4 Yes. I-1 Partially (SupersedeImport right, but import-conflict event missing — Important #1). I-2 Yes. I-3 Yes. I-5 Yes. I-6 Yes. I-7 Yes. I-8 Yes. I-9 Yes. arch C1/C2/C3 Yes. minors/nits Yes (one dangling citation — Minor #2).

---

## CRITICAL

### C2-1. The C-1 identity/ordering scheme covers only *imported* rows — decision & correction events have no defined `EventId` or fold position, yet every reference targets them. Determinism (NFR4) and the C-1 guarantee are unmet for any real ledger.
§6.2 defines `EventId = f(source, source_ref)` and canonical order — **only for imported source rows**. But §6.3 makes every event a `LedgerEvent` with `EventId`, and the decision layer is keyed on decision-event `EventId`s: `VoidDecisionEvent{target}`, `SupersedeImport{target}`, `ReclassifyOutflow{transfer_out_event}`, `ClassifyInbound{transfer_in_event}`. Voiding a *decision* requires it to have a stable `EventId` — undefined (a decision has no source/source_ref). Worse, the fold (§7.2 "fold in canonical order") is a single forward pass keyed on `utc_timestamp`; but a decision created in 2026 (reclassify of a 2022 TransferOut, or supersede of a 2021 Acquire) **rewrites the treatment of an earlier imported event**, changing 2022+ FIFO/ST-LT. A single canonical-order fold that places the decision at its 2026 timestamp **cannot retroactively** alter 2022 lot math. The projection must be **two-pass** (resolve decisions/supersessions into an effective imported timeline, then fold canonically) — never stated. Pin: (1) decision-event stable `EventId`; (2) two-pass fold model; (3) total order over decisions and decisions-vs-imports. Without this, NFR4/NFR6 fail once any reconciliation/correction exists — i.e., for essentially every real dataset.

---

## IMPORTANT

### I2-1. The "import-conflict event" is referenced everywhere but is not in §6.4, and the accept/**reject** lifecycle is undefined → `import_conflicts` has no backing event (NFR6 hole).
FR1/§9/§7.1/§10/§12 say a changed row "records an import-conflict event" resolved by "accept via `SupersedeImport` or reject." But §6.4 has **no `ImportConflict` variant** and §7.3 has **no fold rule** for it; "reject" is unmodeled. Define the `ImportConflict` event (target `EventId` + new payload + fingerprint), its fold rule, and how accept (`SupersedeImport`) and reject each resolve it deterministically.

### I2-2. The sat-conservation identity (FR9) omits gift/donation **Removals** → spurious `verify` drift for any gifting/donating user.
FR9: `Σ in == Σ disposed + Σ held + Σ fee-sats + Σ pending`. v0.2 split `Removal` (Gift/Donation) out of `Disposal` (Sell/Spend), so gifted/donated sats appear in none of the five terms. Add `Σ removed (gift/donation)` (and decide where gift/donation `fee_sat` lands — Minor #4). §13 property tests inherit the omission. §18's "ENG-n3 conservation → FR9" is overstated.

### I2-3. Projection behavior for a disposal/removal that cannot be covered by available lots is undefined → realistic case with no blocker and a likely negative-remainder/panic.
§7.3 folds `Dispose` by "consume from the wallet pool (FIFO)" with no rule for an empty/insufficient pool. Realistic: a Coinbase `Sell` of coins bought on an un-imported venue; a sale of coins still in `pending_reconciliation`; pre-import-window basis. No `uncovered_disposal`/`insufficient_basis` blocker; the property test asserts "no negative remainders" as if inputs are always feasible. The projection contract must be **total**: an uncovered disposal/removal surfaces a blocker, never panics/negative. (A `SupersedeImport` lowering a quantity below already-consumed sats is the same failure — Minor #3.)

### I2-4. Safe-harbor deadline guard triggers on "first 2025 **disposition**," but Rev. Proc. 2024-28 triggers on "first sale, disposition, **or transfer**" — and "disposition" here = Sell/Spend only. The guard can green-light a time-barred (unsupportable) allocation.
§7.4 guard (1): refuse/flag if posting after "the taxpayer's first 2025 disposition or the 2025 return due date." Archive (`RevProc_2024-28.txt` §5.02(4)): "before the earlier of: (a) the … first **sale, disposition, or transfer** … on or after January 1, 2025." v0.2 defines "disposition" as Sell/Spend only, with self-transfers/gifts/donations separate — so a 2025 self-transfer/gift/donation (the app's core data) won't trip the guard, allowing a specific-unit allocation whose window already closed. Fix: trigger on "first 2025 Sell/Spend **or** Transfer **or** Gift/Donation of BTC." (Also: §5.02(4)(b) is "due date **including extension**.")

### I2-5. Dual-basis received gifts are unrepresentable in the `Lot`/`Disposal` model → loss disposals compute wrong basis + holding period.
`GiftReceived{donor_basis, donor_acquired_at, fmv_at_gift}` captures the inputs, but `Lot` has a **single `usd_basis` and single `acquired_at`**, and §7.3 says only "donor carryover/dual basis + tacked holding period." The addendum Q4 is explicit: a gifted lot needs **two basis figures** (gain = carryover; loss = min(donor_basis, fmv_at_gift)) **and** a *conditional* HP start (gift date, not tacked, when the loss-FMV basis applies). A single-basis `Lot` gets the gain case right, the loss case wrong. Phase 1 computes disposals (FR4) → in scope. Extend `Lot`/`Disposal` to carry dual basis + conditional HP, or explicitly document received-gift loss dispositions as a deferred limitation.

---

## MINOR
1. **§7.3 fold-rule enumeration incomplete:** `ManualFmv` and `ClassifyRaw` (and the missing import-conflict event) have no stated fold rule. Every variant needs fold semantics.
2. **Dangling FOLLOWUPS citation:** §6.2 and §18 ("ENG-m2 occurrence_index fragility → §6.2/FOLLOWUPS") point to a FOLLOWUPS entry that does not exist; §9.2's "FOLLOWUPS M3" daily-close entry exists but is unlabeled. Add/relabel.
3. **`SupersedeImport` can retroactively invalidate dependent state** (lowers an `Acquire` sat below already-consumed). Same remedy as I2-3 (blocker).
4. **Gift/donation `fee_sat` treatment undefined.** `GiftOut`/`Donate` carry `fee_sat?` but §7.3 is silent; the on-chain miner fee leaves the taxpayer (conservation) and may be its own mini-disposition; specify consistently with TP8.
5. **River semantic-`source_ref` corrections can't be superseded.** If `source_ref = (source,direction,utc_ms,type,sat)`, a re-export correcting a constituent field changes the `source_ref` → not detected as "same source_ref, changed content"; old event orphans. Document the limitation (with the occurrence_index note).
6. **`ReclassifyOutflow → Spend` proceeds under-modeled:** one `usd_proceeds_or_fmv` can't separate principal proceeds (FMV of goods) from the fee-sat mini-disposition. Defer ok, but note.
7. **`reconstruct-2025` vs `allocate-2025` selection ambiguous:** what does `reconstruct-2025` emit? Behavior if both a reconstruction and a `SafeHarborAllocation` exist? Clarify Path B = presence of `SafeHarborAllocation`, Path A = absence, and what each command writes.

## NIT
1. **`GiftOut`/`Donate` listed under "Imported events" but no adapter emits them** (every on-chain send → `TransferOut`); they arise only via `ReclassifyOutflow`. Move/annotate as decision-produced.
2. **§13 has no decision-event determinism test.** Add: shuffled decision order → identical `LedgerState`.

---

## Genuinely solid (do not regress)
Stable `source_ref`/`EventId` for imported rows, `LotId=(origin,split_sequence)`, txid as match-signal-not-key, `Removal`/zero-gain, lot-level `SafeHarborAllocation` (dates+conservation+irrevocability+Path-A fallback), FR2 BTC-leg retention, FMV gating of both income and downstream disposals, pending@snapshot exclusion, `decision_conflicts` blocker, atomic-write/flock/mlock/migration with honest R1, and the KAT + cosmetic-idempotency test mandates.

---

## Verdict
**No — v0.2 is not yet at 0 Critical / 0 Important.** 1 Critical (decision/correction-event identity + two-pass fold) + 5 Important (import-conflict event + reject lifecycle; conservation omits Removals; uncovered-disposal undefined; safe-harbor guard misses "transfer" trigger; dual-basis received gifts). All localized; re-review the diff (incl. §18 + FOLLOWUPS citations) after folding.
