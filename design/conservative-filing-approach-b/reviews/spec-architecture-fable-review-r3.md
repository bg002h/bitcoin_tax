# Architecture review r3 — `design/conservative-filing-approach-b/SPEC.md` (Approach B sub-project 1)

**Reviewer:** independent Fable architecture lens, round 3 (adversarial; gate = 0 Critical / 0 Important).
**Artifact:** SPEC.md as revised by the r2 fold (`558c86f`). Both r2 reviews + r1 reviews + `DESIGN_PROVENANCE.md`
read. Every symbol the r2 fold newly cites was opened and read on `feat/conservative-filing-b`; the four
focus areas (removal-leg decomposition, fee evaporation, fold-diff re-key, deferred void-adjudication) were
each independently re-derived against source, including an adversarial sweep of every `state.removals`
consumer and the full donation/gift draw path.

**Verdict: 0 Critical / 1 Important / 0 Minor / 1 Nit — NOT green (1 Important).**
All four r2 findings are genuinely resolved with correct code facts, and every r2-fold addition is buildable
exactly as written at real symbols. But the two headline r2 folds do not compose: BG-D11's removal-leg
decomposition makes prior-year REMOVAL legs promote-sensitive, while the BG-D9/BG-D6 re-key defines the fold
diff over the per-year DISPOSAL-leg set only — and gifts/donations draw by the ELECTED method (HIFO default),
so a promote-induced reorder can silently rewrite a prior donation-only year's Schedule A/8283 with no
advisory and a consent Σ that reads "no change" through BOTH of its arms. One keying stroke closes it.

---

## § Verified resolved (r2 finding → status, with the code fact)

### r2 I-1 (advisory/consent keyed on `tax_total`, structurally unable to fire — the CONVERGED blocker) — RESOLVED
The re-key is concretely specified in BG-D9 ("any year `< current` whose per-year DISPOSAL-LEG set
(equivalently Σ-gain / 8949 content) differs between the pre- and post-promote fold") and BG-D6 (tax-Δ quoted
only when both folds compute the year; otherwise the gain-Δ with an explicit "tax not computable for Y"
clause; each `Acknowledgment` term flagged computed-tax-Δ vs gain-Δ; a genuinely-all-uncomputable promote
never records a bare $0). Buildability verified against source: the fold pair IS producible by existing
machinery — `overpayment_delta_one` (`conservative.rs`) already does clone-`resolve` → mutate →
`fold(res, prices, config)` → `LedgerState`, the baseline is `project(events, prices, config)`, and
`would_conflict` (`project/mod.rs`) is the exact clone-append-project precedent for a not-yet-recorded
candidate event. The per-year leg sets are directly readable: `LedgerState.disposals`/`.removals`
(`state.rs:267-268`) carry `disposed_at`/`removed_at` + legs, and `DisposalLeg`/`RemovalLeg` both derive
`PartialEq, Eq` (`state.rs:144, 181`) so the set diff is a direct comparison. The predicate is genuinely
profile/table/blocker-independent: `fold` requires none of `compute_tax_year`'s three refusal doors (Hard
blocker anywhere / `tables.table_for(year)` miss / missing profile — `tax/compute.rs` gates (1)-(3);
`BundledTaxTables::load()` still {2017, 2024, 2025, 2026}, `tax_tables.rs:75-78`), so it fires on a leg-diff
in a table-less 2018–2023 year — pinned by the §6 lifecycle KAT ("INCLUDING a table-less/profile-less year").
The r2 defect (`None == None` reads "no change") is dead. The *replacement* predicate has a new, different
blind spot — see new I-1; that is a new finding against the folded text, not a failure of this resolution.

### r2 M-1 (void ordering / deferred adjudication) — RESOLVED
BG-D9-i now states: a promote-void applies unconditionally; a tranche-void DEFERS and is adjudicated against
the FINAL non-voided-promote set, mirroring `allocation_voids`. Verified the mirrored pattern is real and
identical in shape: pass-1a collects `AllocationVoid { void_id, target }` instead of classifying inline
(`resolve.rs:477-483`), and step-3 item (5) adjudicates against the FINAL `effective` set
(`resolve.rs:1356-1382` — effective target → `DecisionConflict`, inert target → void applies + Hard
retraction). Acyclicity holds: promote-liveness depends only on promote-targeted voids, which classify in the
plain `Some(_)` arm (`resolve.rs:484-486`), and void-of-void is non-revocable (`resolve.rs:464-466`), so the
two-stage evaluation cannot cycle and is order-independent. The record-time double-void refusal that made the
inline version a permanent brick exists as described (`cmd/reconcile.rs:284` "already names this target").
The §6 both-voids-either-order KAT is present.

### r2 N-1 (non-interactive path two-way choice) — RESOLVED
BG-D6 picks the flag form: `--i-acknowledge <phrase>` with the computed figures still printed to stdout,
citing the shipped precedent — verified `ATTEST_PHRASE` (`btctax-cli/src/lib.rs:197`) and
`require_attestation` (`lib.rs:208`).

### r2 N-2 (§3 item-13 false lead) — RESOLVED
The parenthetical landed in §3 item 13. Verified both textually-identical catch-alls:
`bulk_void_payload_summary` (`cli/main.rs:2171` — renders decision payloads incl. `DeclareTranche`; needs the
promote arm) vs `bulk_resolve_payload_summary` (`cli/main.rs:2083` — renders imported payloads only; a
promote is unreachable; correctly needs none).

---

## § r2-fold additions — buildability confirmed (the four focus areas)

**(a) Removal-leg-builder decomposition (BG-D11) — SOUND, one site genuinely reaches every consumer.**
`make_removal_legs` (`fold.rs:225`) builds each `RemovalLeg` with `lot_id: c.lot_id.clone()`,
`basis: c.gain_basis`, `basis_source: c.basis_source` from `Consumed` fragments that carry `lot_id` + `sat` +
`gain_basis` (`pools.rs:291-306`) — so the promote-set keying via `leg.lot_id.origin_event_id` (and the
per-fragment `filed_basis × c.sat / tranche_sat` estimate share) is available at exactly this site, same
mechanism as BG-D4. Every downstream consumer reads leg-side values, never the pool: the fold's
`claimed_deduction` is computed "from the FINAL persisted legs" after both the builder and
`rehome_onto_removal_leg` (`fold.rs:1223-1240`); `crypto_charitable_gifts` does
`short_basis += leg.fmv_at_transfer.min(leg.basis)` per removal leg (`return_1040.rs:535`) → `apply_170b` →
Schedule A; `Form8283Row.cost_basis = leg.basis` (`forms.rs:439`) → `printed.rs:82/155` →
`btctax-forms/src/form8283.rs:365/410` → `removals.csv` (`render.rs:1114-1130`). No separate patch needed —
the claim "all consumers inherit by construction" is TRUE. The evaporation cannot break conservation: the FR9
report is SAT-only (`conservation.rs:38-62`), and the basis-side reported≠consumed precedent
(`NoGainNoLoss`, `fold.rs:154-188`) exists as cited. `compliance.rs:198` reads removals for dates/wallets
only — unaffected.

**(b) Fee-sat evaporation (BG-D4 / `consume_fee`) — IMPLEMENTABLE at the cited site with the same keying.**
`consume_fee` (`fold.rs:323`) receives `Vec<Consumed>` from the FIFO-pinned `consume_fifo`
(`pools.rs:59-65`); the TreatmentC arm sums `gain_basis` across fragments (`fold.rs:348-357`) — the exact
place to decompose per fragment on `c.lot_id.origin_event_id` ∈ promote set and withhold the estimate share
from the `FeeCarry`. TreatmentB needs nothing: its mini-disposition routes fee-sats through
`make_disposal_legs` (`fold.rs:358-372`), so BG-D4's disposal-leg decomposition + clamp apply by
construction. The three re-home sites are as enumerated (`rehome_onto_lot` :291, `rehome_onto_removal_leg`
:305, `rehome_onto_disposal_leg` :313). The spec's worked corner is arithmetically exact ($12k × 10,000/10⁸
= $1.20), and evaporation is self-consistent: burning f fee-sats at per-sat floor p leaves the lot at
p×(N−f), so the per-sat floor — and hence the later leg decomposition's `estimate_share = usd_basis_share`,
documented = 0 — is exactly preserved. The composition with (a) is clean: an evaporating carry re-homes
documented-only basis, so `rehome_onto_removal_leg` cannot re-poison a decomposed leg.

**(c) Fold-diff machinery — real** (see r2 I-1 resolution above).

**(d) Deferred void-adjudication — consistent and acyclic** (see r2 M-1 resolution above). Also re-verified
the surrounding BG-D9 payload facts still hold: `is_revocable_payload` contains `DeclareTranche`
unconditionally and lacks `PromoteTranche` (`void.rs:20-35`), and the `effective_alloc` closure
(`void.rs:72-81`) is the exact exclusion shape BG-D9-iii mirrors, with `events + blockers` sufficing.

Also confirmed on the r2-fold-touched census additions: `Coverage` still derives only
`Debug, Clone, Copy, PartialEq, Eq` (`conservative.rs:173` — §3 item 15 stands);
`how_acquired_from(leg.basis_source)` (`forms.rs:269, 436`); `timely_allocation_attested: bool`
(`event.rs:187`); the BG-D8 pseudo-export-block precedent (`state.rs::pseudo_active`,
`cmd/admin.rs:80` checked first).

---

## New findings

### I-1 — BG-D9/BG-D6: the fold-diff is keyed on DISPOSAL legs only, but the fold pair also rewrites prior-year REMOVAL legs — the very surface BG-D11 (same fold) made promote-sensitive — so a donation-only prior year rewrites silently and BOTH consent arms structurally read "no change"
- **Defect (one sentence):** the re-keyed trigger fires on "any year whose per-year DISPOSAL-LEG set
  (equivalently Σ-gain / 8949 content) differs", but gifts/donations draw by the ELECTED method — HIFO
  default (`consume_principal`, `fold.rs:52-64` → `applicable_method`; NOT the fee draw's pinned FIFO) — so
  a promote-induced HIFO reorder can change which lots a PRIOR year's donation/gift consumed, rewriting that
  year's Schedule A line 12 / Form 8283 / `claimed_deduction` / §1015 gift carryover with ZERO change to its
  disposal-leg set (a removal recognizes no gain, TP10, and emits no 8949 row), leaving the advisory silent
  and the BG-D6 Σ omitting the year.
- **Both quantification arms are blind, not just the trigger:** (i) the gain-Δ fallback is $0 for a
  removal-only year (removals recognize no gain — there is no "Σ-gain" to diff); (ii) the tax-Δ arm cannot
  see it even when the year computes, because `compute_tax_year` EXCLUDES crypto donations BY DESIGN — the
  frozen crypto-attributable delta uses only the NON-crypto Schedule A ("crypto donations belong to the
  absolute return, not the frozen delta", `return_1040.rs:758-762`; `tax/compute.rs` contains no §170/
  charitable term) — only the full-return engine (`crypto_charitable_gifts` → `apply_170b`,
  `return_1040.rs:1176-1177`) prices them. So the recorded `Acknowledgment` snapshots "no change" for a real
  prior-year rewrite — the same class of silent-$0 poisoning the r2 converged blocker forbade, re-entered
  through the removal door.
- **Concrete failure scenario (reachable, five figures):** documented lot @ $10k/BTC; a $0 tranche declared;
  2024 ST donation of 1 BTC — HIFO draws the documented lot ($10k outranks $0; `hifo_cmp`'s
  `usd_basis == ZERO` special-case sorts the tranche last, `pools.rs:275-287`) → deduction
  `min(FMV, $10k)`. 2026: promote the tranche to a $12k floor → the tranche exits the special-case and now
  outranks the $10k lot → the re-fold draws the tranche for the 2024 donation → BG-D11 files its removal leg
  documented-only ≈ $0 → the already-filed 2024 deduction silently drops by ~$10k. No 2024 disposal leg
  changed; no advisory; consent Σ omits 2024; the tax-Δ arm couldn't have priced it anyway. The VOID
  direction (amend-to-pay) inherits the same hole. Note the causal irony: this surface only became
  promote-sensitive because the SAME r2 fold ruled the removal-leg decomposition (BG-D11) — the two r2 fixes
  are individually correct and jointly under-keyed.
- **Fix direction (one keying stroke, stays inside the existing decision structure):** re-key the BG-D9
  fold-diff to the per-year DISPOSAL-LEG **and REMOVAL-LEG** sets (equivalently: the year's filed content —
  8949 **and** Schedule A/8283/`claimed_deduction`) between the two folds — both leg types already derive
  `PartialEq`/`Eq`, so this is the same set comparison over `state.removals` as over `state.disposals`.
  BG-D6: for a year flagged by a removal-leg diff, the per-year term quotes the **deduction-Δ**
  (Σ `claimed_deduction` / Schedule-A-input delta from the fold pair — profile/table-independent), never a
  $0 or gain-Δ-only line, with an explicit note that the computed tax-Δ does not include crypto-donation
  changes (`compute_tax_year` excludes them; only the full-return reprices them — so the copy must not imply
  the quoted tax-Δ captures the deduction effect). Amend the §6 lifecycle KAT to pin the removal case: the
  advisory fires on a promote that reorders a prior DONATION-only year (no disposal-leg change), in both
  directions.

## Nit

- **N-1 (BG-D9-ii):** "a `PromoteTranche` whose target is absent/voided/wrong-type is a hard
  `DecisionConflict`" should say it applies to **non-voided** promotes only. In the both-voids end state
  (promote dead + tranche voided) a naive validation over ALL promotes would emit a spurious permanent Hard —
  contradicting the §6 both-voids KAT's "never a bricked ledger" convergence. The KAT pins the correct
  behavior, so this is a one-qualifier clarification, not a design hole.

---

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 1 |
| Minor | 0 |
| Nit | 1 |

Gate: **NOT green** (1 Important). The r2 fold is faithful and every resolution is real — the fold-diff
re-key, the one-site removal decomposition, the fee evaporation, and the deferred adjudication are all
buildable at the exact symbols cited, and the machinery claims survive adversarial re-derivation. The single
blocker is a composition gap between the fold's own two headline fixes: the advisory/consent fold-diff must
range over removal legs as well as disposal legs, and the consent must quote a deduction-Δ for
removal-flagged years because the tax-Δ arm is structurally blind to crypto charitable changes. The fix is
one keying amendment in BG-D9 + BG-D6 + one §6 KAT line.
