# Review — SPEC_foundation.md v0.4, Tax-Correctness (Round 4)

- **Artifact:** `design/SPEC_foundation.md` (v0.4)
- **Reviewer:** independent tax-correctness reviewer, fresh context; verified against verbatim archived primary text.
- **Date:** 2026-06-28
- **Verdict:** **0 Critical, 0 Important — tax gate HOLDS.** Round-3 minor folds are tax-correct (M1 now in the safe/gain-protective direction). 2 new Minor / 2 Nit (non-blocking). All TP citations resolve to archived sources.
- Persisted per STANDARD_WORKFLOW §2.

---

## Part 1 — Round-3 minor folds verified tax-correct
- **TAX-M1 (unknown-donor-basis fallback date)** — TP11 now uses FMV **at the donor's acquisition date** (§1.1015-1(a)(3) / §1015(a), verified verbatim), not `fmv_at_gift`; distinct `GiftFmvFallback` basis_source; unknown date → `unknown_basis_inbounds`. Reverses the prior understate-gain risk. Correct + conservative.
- **TAX-M2 (config-(b) fee double-count)** — FR9: `Σ on-chain-fee-sats` is the sole conservation home; (b) adds a recognition record, not a second bucket. Correct.
- **TAX-M3/ENG-n4 (gift/donation fee via TP8 analogy)** — TP8 "extended by analogy"; §1.1012-1(h)(2)(ii) addresses only purchase/exchange costs → "limited guidance" label accurate. Correct.
- **TAX-M4 (global-alloc deadline)** — §7.4 now: method *description* before 2025-01-01 (§5.02(5)(a)); completed allocations before later of §5.02(4)(a)/(b) (§5.02(5)(b)). Verified verbatim. Correct.
- **TAX-N1/ENG-m6 (`dual_loss_basis = None` branch)** — §7.3 single carryover + HP tacks; matches §1.1015-1(a)(1) and §1223(2). Boundary (`proceeds == basis`) → zero-gain/loss zone, matching the reg Example. Correct.
- **Time-bar trigger set** — §5.02(4) "earlier of (a) first sale/disposition/transfer on/after 2025-01-01 or (b) due date including extension" matched verbatim; "same type" narrowed to BTC. (See R4-M1 on "transfer" precision.)

## Part 2 — TP citations all resolve to archived sources
TP1–TP11 spot-verified against Notice 2014-21, §1001, RevProc 2024-28 §§3.11/4.01/4.02/5.02, §1.1012-1(h)/(j), §1015(a)/§1.1015-1(a)(1)-(3), §1222/§1223, §61, RevRul 2023-14/2019-24, §170(e)/(f)(11), §1091, CCA 202302012, Pub 544/551. No drift.

## CRITICAL — None.
## IMPORTANT — None. Tax treatment at 0 Critical / 0 Important.

## MINOR (new; non-blocking)
- **R4-M1 — §5.02(4) "transfer" trigger should cite §3.11; a confirmed self-transfer is over-included.** §3.11 defines "transfer" as a conveyance "other than a sale or disposition ... **to another taxpayer**, including a completed gift, donation, contribution, or distribution." (1) This is the precise ground for including Gift/Donation in the §7.4 trigger — §7.4 should cite §3.11, not just §5.02(4). (2) A **self**-transfer (`TransferLink`, own→own) is NOT a §3.11 "transfer" (not "to another taxpayer"), so listing a bare "Transfer" as a deadline trigger over-includes confirmed self-transfers. Severity Minor: direction is conservative (over-fired time-bar → inert allocation → Path A reconstruction, always valid; attestation override available) → no wrong tax number, only possible loss of a legitimate Path-B optimization absent attestation. Recommend: cite §3.11; exclude confirmed `TransferLink` from the trigger (or document the conservative treatment).
- **R4-M2 — §7.4 "before the allocation's effective date" is load-bearing and, read literally, is a dead guard.** `effective_date` is fixed 2025-01-01; any "first 2025 event" is on/after 2025-01-01, so literally the inert-condition never fires → a genuinely time-barred safe harbor could govern (Path B) → wrong basis allocation. The intent text ("re-evaluates when a 2025 disposition is imported later") shows the comparison is against the allocation's *establishment/recording*, not the static field. Minor because intent is sound, but the phrasing controls a tax outcome → restate unambiguously (e.g., "if a first-2025 sale/disposition/to-another-taxpayer-transfer precedes the allocation's recorded establishment, or is present without attestation").

## NIT
- **R4-N1** — TP11 fallback: note §1.1015-1(a)(3) is an IRS-*determination* mechanism (Secretary/district director), not taxpayer self-help; v0.4's "pragmatic simplification (flagged)" is adequate; add a clause so "Settled" isn't read as covering taxpayer self-application.
- **R4-N2** — Deferred items (appraisal trigger precision §170(f)(11)(C)/(F) + basis-limit for §170(e) property; §1015(d)) correctly in FOLLOWUPS with safe Phase-1 over-flagging. Verified §170(f)(11)(C)/(F), §170(e)(1)(A), CCA 202302012. No action.

## Verdict
v0.4's fold of the round-3 tax minors is faithful and introduces no tax error; M1 corrects the prior understate-gain risk. All citations archive-grounded; time-bar trigger/extension/global-method wording match RevProc §5.02(4)/(5) verbatim, with §3.11 confirming gift/donation inclusion. **Tax at 0 Critical / 0 Important.** R4-M1 and R4-M2 (load-bearing wording) recommended for the next fold but do not block the plan gate.
