# Architecture review r5 — `design/conservative-filing-approach-b/SPEC.md` (Approach B sub-project 1)

**Reviewer:** independent Fable architecture lens, round 5 (re-review-after-fold; gate = 0 Critical / 0 Important).
**Artifact:** SPEC.md @ `52c76c8` (r1–r4 folds applied). Both r4 reviews + all prior reviews +
`DESIGN_PROVENANCE.md` read. Scope per convergence protocol: the r4 fold's touch surface (BG-D9 cascade
bullet, BG-D9 donation/gift quoting sub-bullet, BG-D6 removal-flagged term + cascade note, §3 item 8c, the
two new §6 KATs, the §6 lifecycle word-order) re-derived against source; r2–r4-settled verifications
(§3 census 1–8b, BG-D1, removal-leg builder, fee evaporation, deferred void-adjudication, BG-D9-ii scope)
re-confirmed only where the fold sits adjacent — all intact and untouched.

**Verdict: GREEN — 0 Critical / 0 Important / 0 Minor / 2 Nits. The architecture lens remains GREEN; the gate can close.**
Both r4-fold edits (my M-1/N-1 + the tax lens's I-1) landed correctly, every newly cited symbol is real at
the named site, the cascade fix is naming-plus-conditional-field-reads with zero new machinery, and no
prior-resolved finding is reopened. The two Nits are word-level doc-consistency residue.

---

## § Verified resolved (r4 item → status, with the code fact)

### arch r4 M-1 (gift-only "never $0" unsatisfiable by `Σ claimed_deduction`) — RESOLVED, buildable
BG-D6 now quotes "the profile-free deduction-Δ for donation legs (`Σ claimed_deduction`) **or** the §1015
carryover-basis-Δ for gift legs (`Σ leg.basis`)"; the BG-D9 sub-bullet ("The removal-flagged term
distinguishes DONATION legs from GIFT legs") carries the copy split — gift-flagged → "donee-basis (§1015
carryover) documentation … changes; the donor's Form 1040 is unaffected", **no 1040-X assertion**, Form 709
noted — plus the both-Δs-zero rule (name the changed 8283 dates / donee records, never a bare $0); the §6
"Removal-flag quoting + cross-year cascade" KAT pins the gift-only reorder. Buildability verified to be
exactly the deduction-Δ's shape:
- `RemovalKind::{Gift, Donation}` (`state.rs:177-180`) discriminates the legs; `Removal { kind, removed_at,
  legs, claimed_deduction, .. }` and `RemovalLeg { basis, .. }` (`state.rs:181-216`) both derive
  `PartialEq, Eq`, year-keyed by `removed_at` — so "Σ `leg.basis` over the year's gift removal legs" is a
  filter (`kind == Gift`, `removed_at.year() == Y`) + sum over the same two `LedgerState`s the fold pair
  already yields. Profile-, table-, and blocker-free, identical derivation pattern to `Σ claimed_deduction`.
- The `None`/`Some` split the copy rule rides on is code-true: the `Op::GiftOut` arm pushes
  `claimed_deduction: None` (`fold.rs:1151`); the `Op::Donate` arm pushes `Some(..)` computed from the final
  legs (`fold.rs:1231-1240, 1276`). Gift legs come through the same `make_removal_legs`
  (`fold.rs:1118`; `basis: c.gain_basis`, `fold.rs:256`) the BG-D11 decomposition ruling owns, so the
  carryover-Δ automatically reads the post-BG-D11 documented-only basis — the quoted figure and the filed
  donee-documentation figure cannot diverge. No new mechanism anywhere in the clause.

### arch r4 N-1 (§6 KAT word-order) — RESOLVED
The lifecycle KAT now reads "a **non-voided promote** with an absent/wrong-type target → hard
`DecisionConflict` (arch r4 N-1 word-order)" — qualifier on the promote, matching BG-D9-ii, which is
unchanged (still scoped to non-voided promotes with the arch-r3 both-voids no-spurious-Hard clause intact).

### tax r4 I-1 fold (cross-year carryover cascade) — VERIFIED SOUND from the architecture lens
The fold is the naming clause + census + KAT the finding prescribed, and demands **no new machinery**:

- **Census entries are real symbols at the named sites.** `carryforward_consistency`
  (`tax/compute.rs:436-448`), wired at `cmd/tax.rs:454-466`; `write_back_carryover` (`cmd/tax.rs:485`);
  `apply_carryover_writeback` (`return_1040.rs:1454-1491`, called from `cmd/tax.rs:581`). File
  attributions in §3 item 8c are correct respectively.
- **The 8c prose claims are code-true.** `apply_carryover_writeback`'s own doc says "A computed (or empty)
  existing carryover-in is overwritten silently" (`return_1040.rs:1453`; the guard at `:1459-1470` refuses
  only `CarryProvenance::User` without `--force`) — the spec's "silently overwriting a Computed-provenance
  value" is exact. `carryforward_consistency`'s copy is literally "verify your prior return"
  (`compute.rs:442-444`) — promote-blind as stated — and the CLI wiring fires only when both years resolve
  profiles AND the prior year is `Computed` (`cmd/tax.rs:444-466`), confirming the never-for-2018-2023
  characterization. Engine B's blindness: `let cf = profile.capital_loss_carryforward_in`
  (`compute.rs:317`) — profile-supplied, identical across both folds, so the per-year tax-Δ structurally
  cannot see the cascade; the spec says exactly this.
- **"Name the cascade, don't compute it" holds — the two quoted Δs are conditional field reads of existing
  outputs, not new engines.** (i) `carryforward_out` is a field of the `Computed` outcome
  (`types.rs:112`, populated `compute.rs:407`) that the consent's existing per-year
  `compute_tax_year`-on-both-folds calls already produce — quoting its diff "when both folds compute Y" is
  a field read. (ii) `charitable_carryover_out: Vec<CharitableCarryItem>` (`return_1040.rs:902`, populated
  `:1311`; `CharitableCarryItem` derives `PartialEq, Eq`, `return_inputs.rs:270`) is produced by the
  existing public entry `assemble_absolute` (`return_1040.rs:1035`) over a `LedgerState` — running it over
  the fold pair is a new *call* of existing machinery, correctly conditioned ("when Y's absolute return
  computes") so absent `ReturnInputs`/screen-refusal degrades to named-unquantified, never a demand.
  Nothing in the clause requires flagging Y+1, adding Y+1 to the consent Σ as a computed term, or
  recomputing the downstream amendment — the flagged-year set remains the leg-diff set; the cascade is
  copy + a recorded `Acknowledgment` term. The BG-D6 side ("named-unquantified … never silently absent")
  and the §6 KAT ("quoted where the machinery computes, else named-unquantified — loud, never silent")
  agree with the BG-D9 clause; no contradiction between the three sites.

### Adjacency sweep — nothing reopened
The r4 fold's edits are additive within BG-D9/BG-D6/§3/§6; the r2–r4 settled surfaces they abut are
byte-intact where it matters: BG-D9-ii non-voided scope + deferred tranche-void adjudication; the
fold-diff predicate (leg-SET, disposals ∪ removals, `< current` advisory filter vs unfiltered consent Σ);
§3 items 1–8b incl. the fee-draw evaporation; the BG-D11 one-builder ruling (which the gift carryover-Δ
now *depends on*, consistently); BG-D8/BG-D7. The status header accurately reflects r4 (arch GREEN 0C/0I;
tax 0C/1I, folded) and pends r5. No cited-symbol-that-doesn't-exist found in the fold diff.

---

## New findings

### N-1 (Nit) — BG-D6's Acknowledgment-flavor enumeration predates the cascade term
"The `Acknowledgment` snapshot records each term as computed-tax-Δ **or**
gain/deduction-Δ-with-uncomputable-flag" is now a two-flavor enumeration of a three-flavor reality — the
cascade note (quoted-Δ or named-unquantified) is a third recorded kind, defined two bullets earlier and
pinned by the §6 cascade KAT, so the operative clauses are unambiguous and no implementation can silently
drop it; the enumeration sentence just wasn't extended. One-phrase fix at the fold or in the plan.

### N-2 (Nit) — cascade conditional wording, singular vs pair
"the `charitable_carryover_out` diff when Y's absolute return computes" — a diff needs the absolute return
under BOTH folds, parallel to the sibling clause's "when both folds compute Y". The intent is clear from
the sibling; word-level.

---

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 0 |
| Minor | 0 |
| Nit | 2 |

Gate: **GREEN (0C/0I)**. The r4 fold is faithful and fully buildable at the cited symbols: the gift
carryover-Δ is the same fold-pair filter-and-sum as the deduction-Δ (`RemovalKind` + `RemovalLeg.basis` +
`Eq` derives), the three cascade census symbols exist exactly where §3 8c places them with code-true prose
(`apply_carryover_writeback`'s silent Computed-overwrite included), and the cascade clause is honest
naming over existing outputs (`carryforward_out` field read; `assemble_absolute`'s
`charitable_carryover_out`, conditionally) with no new machinery and no Y+1 flagging demand. Nothing
reopened. The two Nits are enumeration/word-order residue for the ownerless batch or the Phase-1a plan.
