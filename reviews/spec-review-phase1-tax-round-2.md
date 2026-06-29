# Review — SPEC_foundation.md v0.2, Tax-Correctness (Round 2)

- **Artifact:** `design/SPEC_foundation.md` (v0.2)
- **Reviewer:** independent tax-correctness reviewer, fresh context; verified against verbatim archived primary text.
- **Date:** 2026-06-28
- **Verdict:** 0 Critical, **2 Important** (both newly introduced by the v0.2 fold). Round-1 Criticals genuinely resolved; all TP citations now grounded in the archive. Not yet 0/0 — one more fold.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

---

# Independent Tax-Correctness Review — `SPEC_foundation.md` v0.2 (Round 2)

**Reviewer:** independent tax-correctness skeptic, fresh context. **Method:** every position and every round-1 finding checked against the verbatim archived primary text in `legal/text/` and `legal/primary-sources/` — not against the spec's assertions or the round-1 review. **Date:** 2026-06-28.

**Bottom line up front:** Both round-1 Criticals (C1 gift/donation; C2 safe-harbor) are **genuinely and correctly resolved**, not superficially papered over, and every TP-table citation now resolves to an archived source (round-1 M1/M2 fixed — no "FAQ" cites remain). However, the fold that fixed C1/I1 **introduced two new Important defects**: the global sat-conservation invariant was never updated to account for the new non-recognition `Removal`, and the new `GiftReceived` dual-basis lot has no disposal/holding-period algorithm (and an affirmatively-wrong "tacked holding period" assertion). So the spec is **not yet at 0 Critical / 0 Important** — but it is close, and both are localized.

---

## Round-1 finding disposition (verified against source)

| R1 | Status in v0.2 | Verified against archive |
|----|----------------|--------------------------|
| **C1** gift/donation as gain events | **RESOLVED** | TP1/TP10 + `Removal` (§6.3) + `GiftOut`/`Donate` (§6.4) + §7.3 zero-gain. RevProc 2024-28 §3.11 verbatim (lines 360-364): "'Transfer' means the conveyance, other than a sale or disposition, … including a completed gift, donation, contribution, or distribution." §1001(a)/(c), §1015(a) (carryover + dual-basis), §170(e)(1)(A), §170(f)(11)(C), CCA 202302012 all confirmed. |
| **C2** safe-harbor deadline/irrevocability/fallback | **RESOLVED** | §7.4 guards verified verbatim: deadline §5.02(4); global §5.02(5)(a) "before January 1, 2025"; irrevocable §4.02(6); post-2025 exclusion §4.01(2). Path-A reconstruction default; FR8/§6.4 make `SafeHarborAllocation` non-voidable; conservation guard added. |
| **I1** inbound income/gift unrepresentable | **RESOLVED** | `ClassifyInbound` → `Income{kind,fmv,business}` \| `GiftReceived{donor_basis,donor_acquired_at,fmv_at_gift}` (§6.4). *(But see New-I2 — disposal side unspecified.)* |
| **I2** Gemini BTC `Credit` swallows income | **RESOLVED** | §9.1: `Credit`(BTC) → `Unclassified`, "never auto-`TransferIn`." |
| **I3** income/Convert mapping; unknown→Unclassified | **RESOLVED** | §9.1: Coinbase `Convert` BTC-leg → `Dispose{Sell}`/`Acquire` at FMV (Notice 2014-21 A-6); `Order`+unknown → `Unclassified`; reward/income → `Income`. |
| **I4** txid dedup-vs-match collision | **RESOLVED** | §6.2: `source_ref` scoped by `(source,direction)`; txid = within-source dedup + cross-source match signal, "NOT … a global dedup key." |
| **I5** Swan double-count | **RESOLVED** | §9.1: Swan `transfers` → reconcilable `TransferIn`; source-venue lot authoritative; Swan basis only when no internal match. |
| **M1/M2** FAQ cites → archive | **RESOLVED** | No "FAQ" in spec. TP2 cites Pub 551 (line 240), §1001(b), Pub 544 (line 348). |
| **M3** daily-close note | RESOLVED | §9.2 + FOLLOWUPS (RevRul 2023-14 "date and time … dominion and control"). |
| **M4** fork out-of-scope | RESOLVED | FOLLOWUPS; `Income.kind` drops `Fork`. |
| **M5** reclassify drops fee | RESOLVED | `ReclassifyOutflow{… fee_usd?}`. *(see New-Minor)* |
| **M6** prefer actual allocation | RESOLVED | §7.4 Path A default; Path B `ActualPosition` preferred. |
| **N1** mining SE flag | RESOLVED | `Income.business` (Notice 2014-21 A-9). |
| **N2** TP8 contrary cite | RESOLVED | TP8 cites §1.1012-1(h)(2)/(h)(4), scoped "taxable-exchange context only." |

All ten TP citations resolve to archived sources.

---

## CRITICAL
None. Both round-1 Criticals are correctly and substantively resolved.

---

## IMPORTANT (both newly introduced by the v0.2 fold)

### New-I1. The global sat-conservation invariant (FR9) omits gifted/donated sats — every gifting user will show spurious drift.
FR9's check is `Σ in == Σ disposed + Σ held + Σ fee-sats + Σ pending-reconciliation`. But the fold split **`Removal` (Gift/Donation)** from **`Disposal` (Sell/Spend only)** — §6.3, FR4, §7.1 treat removals as a separate output, and "disposed" = Sell/Spend only. So sats leaving via `GiftOut`/`Donate` appear on **neither** side: not "disposed," not "held," not "fee-sats," not "pending." For any ledger with a gift/donation, `verify` reports false `Σ in > RHS`. The likely "fix under pressure" — lumping removals into `Σ disposed` — would conflate non-recognition removals with realizations, undoing C1. The invariant needs an explicit `+ Σ removed (gift/donation)` term. Core integrity guarantee (NFR6/FR9) wrong as written. *(Fix: add removed term to FR9 and the §13 conservation property test.)*

### New-I2. Disposal of a `GiftReceived` (dual-basis) lot is unspecified, and §7.3 asserts an unconditionally-wrong "tacked holding period."
The I1 fix added `GiftReceived{donor_basis, donor_acquired_at, fmv_at_gift}` lots, and §7.3 says they get "donor carryover/dual basis + tacked holding period." Phase 1 **does compute disposals** (§7.3 `Dispose` → `Disposal` with ST/LT). But the fold rule says nothing about gain/loss or holding period for a dual-basis lot, and blanket "tacked holding period" is wrong for the loss case. Verified:
- **§1015(a) / §1.1015-1(a):** when FMV-at-gift < donor's adjusted basis, the **loss basis is FMV**, not carryover. Reg Example: donor basis $100k, FMV $90k, sale at $95k → **neither gain nor loss** (middle zone).
- **§1223(2):** holding period tacks only if the property has, "for the purpose of determining gain or loss, the same basis" as in the donor's hands. When FMV loss-basis applies, basis ≠ donor's carryover → **holding period does not tack** (starts at gift date).

As written, a donee who received depreciated BTC and later sells it gets the wrong basis, a fabricated loss, and wrong ST/LT. Data captured, but algorithm + known-answer test missing (§13 tests only the *outbound* zero-gain). *(Fix: specify §1015(a) dual-basis + §1223(2) conditional-tacking in the `Dispose` fold when consuming a `GiftCarryover` lot; add a known-answer test for all three zones — sell above donor basis, below FMV, in between.)*

---

## MINOR

- **`Removal` should state it captures per-lot BASIS, not just FMV+ST/LT.** §6.3 foregrounds "FMV-at-transfer + ST/LT." Phase-2 §170(e) needs FMV-vs-basis; §1015 donee carryover needs donor basis. Make basis-capture explicit and symmetric with `Disposal`.
- **`GiftOut`/`Donate` `fee_sat` handling unspecified.** §7.3 removal fold doesn't say whether on-chain fee-sats are consumed (they leave the taxpayer → must appear in conservation) or how valued. Specify consistently with TP8 (c)/(b).
- **`ReclassifyOutflow`→`Dispose{Spend}` proceeds allocation ambiguous.** Is the goods' FMV the proceeds for principal-only (fee-sats at zero) or principal+fee? Gain differs. Tie to TP8 (c)/(b).
- **`Donate.appraisal_required` keyed to ">$5k FMV" over-approximates §170(f)(11)(C).** Statutory trigger is a **claimed deduction > $5,000** (= basis for §170(e)-reduced property), aggregating similar items (§170(f)(11)(F)). Safe over-flag for Phase 1; note precise Phase-2 trigger.
- **`GiftReceived` with unknown donor basis not modeled.** §1.1015-1(a)(3) supplies an FMV-fallback basis when the donee can't obtain donor facts. Allow unknown→FMV-basis.
- **Path-B deadline guard imprecisions.** §7.4 guard (1) says "2025 return due date" but §5.02(4)(b) is "due date **including extension**" — omitting it over-blocks a validly-extended taxpayer. "refuse/flag" ambiguous (hard block vs warn-and-allow); clarify, since Path B is irrevocable and the user may hold a valid pre-deadline allocation in their own records the app can't see.
- **`ReclassifyOutflow` target omits `Dispose{Sell}`.** A P2P/OTC on-chain sale is a `Sell`, not `Spend`. Math identical (both `Dispose`) so harmless to the result, but could mislabel real sales.
- **TP2 pinpoint cite.** "§1.1012-1(h)(2)(i)" is the *definition* of digital-asset transaction costs; the basis-inclusion rule is (h)(1)/(h)(2)(ii)(A). Substance supported; tighten subparagraph.

## NIT
- **Safe-harbor capital-asset eligibility (§4.02(1)-(2)) assumed** — fine for a personal investor, but unstated.
- **§18 fold-record drift.** Maps M4 (fork) to "FOLLOWUPS/§15," but §15 doesn't mention forks; only FOLLOWUPS does.
- **Pre-2025 "legal default" FIFO phrased a touch strong** (no single mandated pre-2025 method); `pre2025_method_note` blocker correctly hedges.

---

## Verdict
The money-line engine remains sound and faithfully grounded, and **all round-1 tax findings are genuinely resolved** (verified against §3.11/§4.01(2)/§4.02(6)/§5.02 and §1001/§1015/§170/CCA 202302012); TP table fully archive-grounded.

The fold introduced **2 Important** defects: (New-I1) conservation invariant omits `Removal`; (New-I2) `GiftReceived` dual-basis disposal unspecified + wrong unconditional-tacking (§1015(a)/§1223(2)).

**NOT yet at 0 Critical / 0 Important** (0 Critical, 2 Important). Both localized — one equation term + its test, and one fold-rule + a three-zone known-answer test. One more fold + re-review should clear the gate.
