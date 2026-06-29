# Review — SPEC_foundation.md v0.3, Tax-Correctness (Round 3)

- **Artifact:** `design/SPEC_foundation.md` (v0.3)
- **Reviewer:** independent tax-correctness reviewer, fresh context; every claim verified against verbatim archived primary text.
- **Date:** 2026-06-28
- **Verdict:** **0 Critical, 0 Important** — tax-correctness gate CLEARED. Round-2 Importants (conservation; dual-basis/tacking) genuinely resolved and archive-grounded. 4 Minor / 3 Nit refinements remain (non-blocking); M1 errs toward understating gain — worth folding.
- Persisted per STANDARD_WORKFLOW §2.

---

## Round-2 findings: verified resolved against the archive
**New-I1 (conservation omits Removals) — RESOLVED.** FR9 = `Σ in == Σ disposed(Sell/Spend) + Σ removed(Gift/Donation) + Σ held + Σ on-chain-fee-sats + Σ pending`; §13 conservation test mirrors it. Preserves the C1 distinction (removals not folded into disposed).

**New-I2 (dual-basis gift disposal + wrong unconditional tacking) — RESOLVED, logic correct.** Verified line-by-line vs source:
- Gain-basis = donor carryover, HP tacks — §1.1015-1(a)(1), §1223(2). Spec §7.3 correct.
- Loss-basis = FMV-at-gift, HP from gift date (no tack) — §1.1015-1(a)(1); §1223(2) doesn't tack when basis differs. Spec §7.3 correct (fixes the round-2 unconditional-tacking error).
- Middle zone = no gain/no loss — §1.1015-1(a)(2) Example. Spec §7.3 correct.
- Unknown donor basis → FMV fallback — §1015(a)/§1.1015-1(a)(3). Present (but wrong FMV date — M1).
Data model: §6.3 `Lot.dual_loss_basis: Option<Decimal>` + `donor_acquired_at: Option`; pro-rata basis split; `Disposal` records zone + HP; §13 KAT covers all three zones. Confirmed.

**Round-2 minors — verified:** Removal captures basis (§6.3/FR4/§7.3, symmetric with Disposal) ✓; gift/donation `fee_sat` per TP8 + conservation fee-sats term ✓ (see M2); appraisal trigger now a bare flag, computation deferred (Nit N2) ✓; Path-B deadline incl. extension + full trigger set verified verbatim vs RevProc §5.02(4) ✓; TP2 pinpoint §1.1012-1(h)(1)/(h)(2)(ii)(A) verified ✓. All TP citations re-verified; no drift.

## CRITICAL — None.
## IMPORTANT — None. Tax treatment at 0 Critical / 0 Important.

## MINOR (new; non-blocking)
- **M1 — Unknown-donor-basis fallback uses the wrong FMV date; errs toward understating gain.** §1015(a)/§1.1015-1(a)(3) set the fallback at FMV "as of the date... the property was acquired by such donor," but the `GiftReceived` payload only carries `fmv_at_gift`. For appreciated BTC, `fmv_at_gift` ≫ FMV-at-donor-acquisition → overstates basis → understates gain (dangerous direction). Also: §1.1015-1(a)(3) is an IRS-determination provision (a fallback is already a pragmatic simplification), and the lot's `basis_source` would mislabel as `GiftCarryover`. Recommend: when `donor_acquired_at` known, compute fallback from the price dataset at that date; else flag for user input/conservative basis; add a distinct basis_source; drop "Settled" on this sub-rule until pinned.
- **M2 — Config-(b) fee mini-disposition can double-count in conservation.** §7.3 says fee_sat counts in the FR9 fee-sats term even under (b); but a (b) mini-disposition emitted as `Dispose{Spend}` would also fall under `Σ disposed`, double-counting. State that `Σ on-chain-fee-sats` is the *sole* conservation home for fee sats and the (b) toggle affects only *recognition*. (Mostly engineering; FR9/NFR6 invariant.)
- **M3 — Gift/donation network-fee treatment borrows TP8 without separate grounding.** TP8 is scoped to self-transfer fees; §7.3 reuses (c)/(b) for gift/donation fees with no separate citation. Immaterial in dollars; TP8's limited-guidance framing reasonably extends, but say so explicitly.
- **M4 — Global-allocation (ProRata) deadline over-stated as "predate 2025-01-01."** Per §5.02(5): only the *method description* must predate 2025-01-01 (§5.02(5)(a)); completed allocations are due before the later of §5.02(4)(a)/(b). Spec is over-restrictive but conservative (over-block → Path A) → no wrong tax result; tighten wording.

## NIT
- **N1 — `dual_loss_basis = None` path unstated** (FMV-at-gift ≥ donor basis → ordinary carryover, both gain/loss use `usd_basis`, HP tacks). Implied; state it so the engine doesn't trip on a `None` comparison.
- **N2 — Appraisal trigger precision** keyed to claimed deduction >$5k (aggregated, §170(f)(11)(F)); for §170(e)-reduced property the deduction = basis. FMV-keyed Phase-1 flag only over-flags (safe); record precise Phase-2 trigger.
- **N3 — §1015(d) gift-tax basis increase not modeled** (rare for personal BTC under annual exclusion); note the omission.

## Verdict
Money-line engine sound and faithfully grounded. Both round-2 Importants genuinely resolved; every named round-2 minor resolved (verified vs §1015(a), §1223(2), §1.1015-1, §5.02(4), §3.11, §1.1012-1(h), §170(f)(11)(C), CCA 202302012). No Critical/Important introduced by the fold; the broadened safe-harbor guard, the two-pass reclassification model, and gift/donation fee handling are tax-correct as written. **Tax at 0 Critical / 0 Important — gate cleared.** Remaining 4 Minor / 3 Nit; M1 most substantive (pin the unknown-basis fallback FMV date). None blocks the plan gate.
