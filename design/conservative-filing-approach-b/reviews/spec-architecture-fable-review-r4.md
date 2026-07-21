# Architecture review r4 — `design/conservative-filing-approach-b/SPEC.md` (Approach B sub-project 1)

**Reviewer:** independent Fable architecture lens, round 4 (adversarial; gate = 0 Critical / 0 Important).
**Artifact:** SPEC.md with the r1+r2+r3 folds applied. Both r3 reviews + the r1/r2 reviews +
`DESIGN_PROVENANCE.md` read. Scope per convergence protocol: the r3 fold's touch surface (the fold-diff
widening to removal legs, the current-year Σ, the BG-D9-ii scope, the two tax nits) re-derived against
source; the r2/r3-settled verifications (§3 census completeness, BG-D1, removal-leg builder, fee
evaporation, deferred void-adjudication mechanism) re-confirmed only where the fold touches them.

**Verdict: GREEN — 0 Critical / 0 Important / 1 Minor / 1 Nit. The gate can close.**
Both r3 findings are genuinely resolved with correct code facts; the widened predicate is buildable from
the existing fold-pair machinery and provably fires on the exact r3 I-1 scenario. The two residual items
are a copy-scoping Minor (the gift-only corner of the new "never $0" clause) and a word-order Nit —
neither gates under §2 severity rules.

---

## § Verified resolved (r3 finding → status, with the code fact)

### r3 I-1 (fold-diff DISPOSAL-scoped while BG-D11 made REMOVAL legs promote-sensitive — the CONVERGED r3 blocker) — RESOLVED
The fold text now keys BG-D9 on the per-year **FILED-CONTENT** set = disposal legs AND removal legs, and
BG-D6's per-year term ranges over both surfaces (removal-flagged year → profile-free deduction-Δ). Every
load-bearing claim verified at source:

- **The raw material exists with the right derives and year keys.** `LedgerState.removals: Vec<Removal>`
  (`state.rs:268`), `Removal { removed_at: TaxDate, legs: Vec<RemovalLeg>, claimed_deduction: Option<Usd>, .. }`
  (`state.rs:202-216`), and BOTH `RemovalLeg` (`state.rs:181`) and `Removal` (`state.rs:201`) derive
  `PartialEq, Eq` — as do `DisposalLeg`/`Disposal` (`state.rs:144, 166`). The per-year removal-leg set diff
  is the same direct set comparison as the disposal side, grouped by `removed_at`/`disposed_at`.
- **`Σ claimed_deduction` is derivable from the fold pair WITHOUT engine B.** The `Op::Donate` arm computes
  `claimed_deduction` from the FINAL persisted legs (LT→FMV; ST→`min(fmv, basis)`, `fold.rs:1231-1240`)
  and stores `Some(..)` on the `Removal` (`fold.rs:1276`); the field's own doc says "Standalone Schedule-A
  figure — does NOT feed engine B / `compute_tax_year`" (`state.rs:209-211`). Profile-, table-, and
  blocker-free by construction — readable straight off the two `LedgerState`s the consent already folds.
- **The widened predicate actually FIRES on a donation-reorder with no disposal change.** Donations and
  gifts draw through the same method-elected path as disposals — `consume_principal` (`fold.rs:1108`
  GiftOut, `fold.rs:1185` Donate) → `applicable_method` (HIFO default, `fold.rs:33-44`) → `pools.consume`
  under `hifo_cmp`, whose `usd_basis == ZERO` arm sorts the tranche last (`pools.rs:276-281`). A promote
  (BG-D1: `usd_cost` rewritten inside `resolve`) moves the tranche out of the special-case → different
  `Consumed` fragments → `make_removal_legs` emits legs differing in at least `lot_id` (also `basis`,
  `acquired_at`; `fold.rs:253-266`) → the per-year removal-leg set differs → the diff fires. The Donate
  arm pushes ONLY `st.removals` (no disposal record), so the disposal set is untouched — exactly the r3
  I-1 scenario, now caught by construction.
- **The fold pair itself is settled machinery** (re-confirmed unchanged): `overpayment_delta_one`
  (`conservative.rs:284-322`) is the clone-`resolve` → mutate → `fold` → `LedgerState` precedent;
  baseline via `project`. Nothing new is required beyond reading `state.removals` off the two states.
- **The composition with BG-D11 is self-consistent, both ways.** A prior-year donation that ALREADY drew
  the tranche pre-promote carries basis $0; post-promote, BG-D11 files it documented-only ≈ $0 → the leg
  set is unchanged → no advisory — correct, since the filed content genuinely doesn't change. The
  decomposition and the diff predicate cannot contradict each other.
- **Engine B's blindness (why the deduction-Δ must come from the fold pair) re-confirmed:**
  `tax/compute.rs` contains zero charitable/§170 symbols; only the full-return engine prices donations
  (`crypto_charitable_gifts`, `return_1040.rs:524`, `short_basis += leg.fmv_at_transfer.min(leg.basis)`
  at `:535` → `apply_170b` at `:762`; the "crypto donations belong to the absolute return, not the frozen
  delta" comment at `:758-759`). The BG-D9 copy's "must NOT imply the ~$Δ tax figure captures the
  deduction effect" clause matches this fact.
- The §6 KAT now pins the case: "a prior DONATION/GIFT-only year with NO disposal-leg change … quoting
  the deduction-Δ", in both promote and void directions.

### r3 N-1 (BG-D9-ii scope: both-voids end state must not emit a spurious Hard) — RESOLVED
BG-D9-ii now reads "a **non-voided** `PromoteTranche` whose target is absent/wrong-type is a hard
`DecisionConflict` … scoped to non-voided promotes so the both-voids end state … does NOT emit a spurious
permanent Hard", and the §6 both-voids KAT carries the same clause. Consistency with the deferred
adjudication verified end-to-end: promote-void applies unconditionally → promote dead; the tranche-void
is deferred and adjudicated against the FINAL non-voided-promote set (the `allocation_voids` mirror —
pass-1a collection `resolve.rs:477-483`, step-3 item (5) final-set adjudication `resolve.rs:1356-1382`,
shape re-confirmed unchanged) → no live promote → the tranche-void applies; ii ranges over non-voided
promotes → the dead promote validates nothing → end state promote-dead + tranche-voided with no Hard.
Also checked the fold's simultaneous DROP of "voided" from ii's target-state list for a gap: none exists —
a non-voided promote's target can never end voided, because BG-D9-i renders a tranche-void inert (+
`DecisionConflict`) while a promote is live, so "voided target under a live promote" is unrepresentable
at the resolver's end state and {absent, wrong-type} is the complete residual list. ii and i partition
the cases with no overlap and no hole.

---

## § r3-fold coherence checks (the assigned focus areas)

**(b) Current-year Σ — coherent and unambiguous.** One predicate (the per-year filed-content leg-set
diff over disposals ∪ removals from the single fold pair), two filters: the BG-D9 advisory keeps
`< current` (already-filed years need the 1040-X copy); the BG-D6 consent Σ runs the same diff with no
year filter, so the sell-then-promote-before-filing dominant term is quoted. The per-year terms remain
individually flagged (computed-tax-Δ vs gain/deduction-Δ-with-uncomputable-flag) — the heterogeneous-
terms structure the r2 fold settled; the deduction-Δ term slots into the existing gain-Δ arm's shape and
introduces no new blending claim. A year flagged on BOTH surfaces quotes both clauses (the BG-D9 copy
composes them explicitly). No ambiguity found.

**(d) New-cite sweep of the r3 fold — all real.** Beyond the symbols above: the tax-r3-N-2 fallback
("latest bundled close + its date") is directly buildable — `BundledPrices::max_date()` already exists
(`btctax-adapters/src/price.rs:26`; `PriceProvider` itself is `usd_per_btc` only, `price.rs:5-8`, and
the spec's "or state …" alternative needs nothing). §3 item 8's wiring note ("`crypto_charitable_gifts`
recomputes a prior year's Schedule A from the rewritten legs") matches `crypto_charitable_gifts(state,
year)` reading `state.removals` per year (`return_1040.rs:524-546`). The tax-r3-N-1 clarification (leg-SET
diff, not Σ-gain) is the operative predicate everywhere the spec states it. No cited-symbol-that-doesn't-
exist found in the fold.

---

## New findings

### M-1 (Minor) — BG-D6/BG-D9: the "never $0" clause is unsatisfiable by `Σ claimed_deduction` for a GIFT-only reordered year; quote the carryover-Δ or the filed-content delta there
- **Defect:** BG-D6 says a removal-flagged year "quotes the profile-free deduction-Δ (`Σ claimed_deduction`
  from the fold pair), **never $0**" — but a Gift `Removal` stores `claimed_deduction: None`
  (`state.rs:212`; `fold.rs:1151`), so a promote that HIFO-reorders a prior GIFT-only year (same
  `consume_principal` draw, `fold.rs:1108`) fires the widened predicate correctly while its deduction-Δ is
  identically $0 — the named quantity cannot honor "never $0" for that case, and a KAT written to the
  clause as-is is unbuildable for gifts. The BG-D9 copy has the same corner: the ~$D clause "appears
  whenever a removal leg diffs", so a gift-only year prints "~$G $0 / ~$D $0" plus an inapposite 1040-X
  line, while the REAL change is the §1015 carryover basis on the gift legs (`RemovalLeg.basis`,
  changed by both the reorder and BG-D11's documented-only decomposition; filed surface = removals.csv /
  donee-facing documentation).
- **Why Minor, not Important:** the trigger fires (the year is flagged — no silent rewrite, which was the
  r3 harm), and the donor's own 1040 genuinely does not change on a gift reorder (no gain, no deduction),
  so the $0 figures are truthful and no filed result is wrong; the defect is an internal over-claim plus
  under-descriptive copy for one sub-case, and BG-D6's uncomputable-year clause already lists the
  **"filed-content delta"** as a third arm — the fix is scoping, not machinery.
- **In-spec fix (one clause):** scope "never $0" to DONATION-flagged years; for a GIFT-flagged year quote
  the §1015 carryover-Δ (`Σ leg.basis` over the year's gift removal legs from the fold pair — same
  profile-free derivation) or the filed-content-delta arm, with copy that names the carryover (not a
  deduction) and drops/conditions the 1040-X line for a gift-only diff. Optionally pin with one KAT line
  (gift-only reorder → advisory fires quoting carryover-Δ).

### N-1 (Nit) — §6 lifecycle KAT word-order
"a promote with a **non-voided** absent/wrong-type target → hard `DecisionConflict`" hangs the qualifier
on the target; BG-D9-ii correctly scopes it to the promote ("a **non-voided** `PromoteTranche` whose
target is absent/wrong-type"). The immediately preceding clause in the same §6 bullet states the correct
scope, so the KAT is internally disambiguated — pure word-order: "a **non-voided promote** with an
absent/wrong-type target".

---

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 0 |
| Minor | 1 |
| Nit | 1 |

Gate: **GREEN (0C/0I)**. The r3 fold holds: the removal-leg widening is buildable at the exact symbols
cited (`state.removals` + `removed_at` + `Eq` derives + the stored, engine-B-free `claimed_deduction`),
fires on the donation-reorder-no-disposal-change scenario by construction, composes cleanly with BG-D11
(an already-tranche-drawn removal produces no false diff), and the current-year Σ and the BG-D9-ii scope
are coherent with the settled machinery. M-1 (gift-only "never $0" scoping) and N-1 (KAT word-order) are
recorded per §2 — fix inline at the fold or file with the Phase-1a owner; neither holds the gate.
