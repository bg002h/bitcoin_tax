# Whole-branch CODE review — US-federal-tax-correctness lens (Approach-B Phase 1a)

**Reviewer:** Opus (tax lens), adversarial re-derivation from first principles. Author ≠ reviewer.
**Range:** dde890a..645bc20 (`feat/conservative-filing-b`), `crates/` only.
**Rubric:** `design/conservative-filing-approach-b/SPEC.md` (BG-D1..D11).

## Verdict

**GREEN — 0 Critical / 0 Important / 3 Minor / 4 Nit**

Critical titles: none
Important titles: none

I traced the actual numbers through the fold at every FoldCtx site, constructed the SPEC's worked
corners by hand and against the KATs, and re-derived the clamp/decomposition algebra. The BG-1 core
guarantee (never manufacture a loss off the estimate) and BG-D11 (estimate never funds a
deduction/carry) hold by construction on every surface I could reach. The suite is green
(kat_promote 53 / kat_conservative / kat_tranche 17 / promote_cli 18 all pass; `clippy -p btctax-core`
clean).

---

## What I verified holds (the load-bearing guarantees)

**BG-D4 clamp (`conservative_promote::clamped_leg_basis`, fold.rs:209).** Re-derived across every
band with `estimate_basis = min(estimate_share, max(net − documented_share, $0))`,
`reported = documented_share + estimate_basis`:
- Sold below floor, pure estimate → `reported = proceeds`, gain $0 (not a loss). ✓
- Crowd-out band `estimate ≤ net < estimate+documented` (floor $12k + $30 documented fee sold at
  $8k) → `reported = $8,000`, gain **$0**, NOT −$30. The `net − documented` bound (not bare `net`)
  is what files this correctly; pinned by `sold_just_above_floor_band_still_files_zero_gain` and the
  end-to-end `relocated_with_fee_then_promoted_sold_below_floor_...`. ✓
- Genuine documented loss (documented ALONE > net, incl. `fee_usd > proceeds`) → estimate → $0,
  `reported = documented > net`, gain < 0 preserved (attribution intact). Pinned. ✓
- `None` arm (non-promoted lot) is the identity. ✓

**Fee-draw evaporation (`consume_fee`, fold.rs:411).** The estimate component of a FIFO-drawn
promoted fee fragment is withheld before `FeeCarry`; only the documented remainder re-homes. Per-sat
floor `filed_basis/tranche_sat` is invariant across relocated fragments (relocated lots carry the
floor per-sat), so the `tranche_sat`-denominated `estimate_share` cancels `gain_basis` exactly →
full evaporation for a pure fragment, documented remainder preserved when a real fee carry exists.
Pinned by `tranche_fee_draw_evaporates_estimate_then_sale_files_zero_loss` (sharp $0-proceeds
discriminator).

**All FOUR FoldCtx sites thread the real promote set.** `fold` (492), `pools_before` (547),
`state_as_of` (605) each set `promotes: res.promotes.clone()`; `universal_snapshot`
(transition.rs:67) takes it as a param and the sole call site (resolve.rs:1499) passes the live
`&promotes`. No fold path sees an empty set. The optimizer's `evaluate_disposal → fold` also inherits
it (pinned by `the_optimizer_sees_the_clamped_promoted_basis_not_a_phantom`).

**BG-D11 single-site decomposition (`make_removal_legs`, fold.rs:290, `net_proceeds_share = $0`).**
A removal leg from a promoted lot files `documented_share` only; estimate evaporates. Every downstream
consumer reads `leg.basis` and inherits by construction — verified by reading source, not trusting the
census:
- fold `claimed_deduction` (Donate arm, fold.rs:1318) — reads leg.basis. ✓
- `crypto_charitable_gifts` (return_1040.rs:535) → `apply_170b` → Schedule A line 12 — reads
  `leg.fmv_at_transfer.min(leg.basis)`. ✓ (the independent second emitter — no re-derivation from
  `lot.usd_basis`; grep found zero other removal-basis re-derivations in `tax/`, `render.rs`,
  `form8283.rs`.)
- Form 8283 `cost_basis` (forms.rs:154/440) → printed (printed.rs) → removals.csv (render.rs:468) —
  all `leg.basis`, ST **and** LT. ✓
- §1015 gift carryover (`make_removal_legs` principal `basis:`, not `rehome_onto_removal_leg`). ✓
The KATs assert the COMPUTED 1040 (`full_return_noncash_12` → Schedule A line 12 == $0), the fold
deduction, and the 8283 column together, plus the LT-donation (deduction = FMV, 8283 column still
documented-only), the non-promoted identity ($9k unchanged), and the evaporate-but-keep-$30-fee
corner. Non-vacuous; they would catch a fold-only patch.

**8275 content (form8275.rs).** Part I amount = `leg.basis` (AS-FILED col (e), never the pre-clamp
floor); the `NO_LOSS_SUFFIX` fires on the `leg.basis == leg.proceeds` clamp-bound heuristic; removal
legs are correctly EXCLUDED from Part I (they take no estimated position); the incomplete flag
(`part_ii.trim().is_empty()`) is read back from the event (raw-vault backstop). RISK_PARAGRAPH names
the base as "the resulting additional tax / underpayment attributable to the misstatement" (not the
disallowed basis), 20/40%, §6664(c)(2), Woods, never "safe harbor". ✓

**Export gate (`promote_export_gate`, admin.rs:78).** Called FIRST (refuse-before-bytes) in
`export_snapshot` (142), `export_irs_pdf` crypto-slice (352), and `export_full_return` (579); the
full-return dispatch routes through the latter. `year: None` scans every year with a promoted disposal
leg. Refuses on incomplete Part II. ✓

**Consent + advisory direction (`consent_terms`, `promote_prior_year_advisory`, `render_term`).**
Promote maps `(new,old)=(with,without)` → dgain<0 → "LOWERS … §6511 refund-limited"; Void maps
`(without,with)` → dgain>0 → "RAISES … additional tax + interest". `tax_sign` = `dtax` when both
folds compute, else `(dgain − dded)` (deduction increase lowers tax) — correct pressure sign. Gift Δ
excluded from the 1040 amend clause and labeled §1015 donee-basis-only. Uncomputable years surface the
gain/deduction-Δ with a "tax not computable" flag (never a silent $0); cascade named; unrealized line
present. These are advisory copy, not filed numbers.

**Lifecycle / snapshot (resolve.rs).** Promote rewrites `Op::Acquire.usd_cost` INSIDE resolve step-2
(1298) so the pass-1 §7.4 snapshot sees the floor; `basis_source` stays `EstimatedConservative` so the
D-8 backstop still denies a SafeHarborAllocation (keyed on tag + `remaining_sat`, transition.rs:80).
Deferred tranche-void adjudication (1243) is order-independent; term-invariance preserved (only
`usd_cost` changes).

---

## Minor (recorded; do not gate)

- **M1 — 8275 Part I no-loss suffix misses the mixed clamp+documented-fee corner
  (form8275.rs:129).** A promoted leg sold BELOW floor with a documented fee-sat carry re-homed onto
  it (fold.rs:738, `rehome_onto_disposal_leg` runs AFTER `make_disposal_legs`) has
  `leg.basis = proceeds + documented_fee ≠ proceeds`, so the `leg.basis == leg.proceeds` heuristic
  does NOT append `NO_LOSS_SUFFIX` even though the estimate WAS clamped and the filed basis sits below
  the pure `min-close × sat` floor. The disclosed AMOUNT is exactly as-filed (matches 8949 col (e)),
  and the direction is taxpayer-ADVERSE (a lower-than-method basis, no penalty exposure), so this is
  a disclosure-narrative completeness gap, not a filed-number defect or an aggressive-position
  mismatch. Fix: base the suffix on `leg.gain < 0 || leg.basis < pre-clamp floor share` rather than
  `== proceeds`, or document the exotic corner. (Requires an unusual multi-lot same-disposal fee draw
  on a below-floor promoted sale.)

- **M2 — all-years CSV snapshot omits the Form 8275 txt artifact (admin.rs:198).**
  `export_snapshot` writes `write_form_8275_txt` only under `if let Some(y) = tax_year`; the
  `tax_year: None` all-years dump exports the promoted disposal rows (with the estimated basis in the
  projection CSVs) but no 8275 file rides alongside. The completeness GATE still fires (an incomplete
  Part II refuses even for `None`), so no inadequately-disclosed position escapes; and filing happens
  per-year, where the 8275 IS written. BG-D8 names "CSV" as a surface, so writing the complete
  disclosure alongside the all-years dump too would close the letter of the rule. Low impact (raw
  projection dump, not a per-year filing packet).

- **M3 — [T2] `filed_basis_for` success arm is a catch-all, not `Coverage::Full`
  (conservative_promote.rs:59).** `Some(wr) if wr.coverage == Coverage::Partial => Err`, then
  `Some(wr) => Ok(Full)`. `Coverage` is 2-variant today so it is correct; a future third
  (`Partial`-family) variant would silently take the Full/filed path (latent guard-defeat). Make it an
  explicit `Coverage::Full` match with the residue erroring. Matches the triage item; agreed Minor.

## Nit (recorded)

- **N1 — [T7] `CONFLICT_HINT` duplicated** (local const in `live_promotes` AND `resolve`, "kept in
  sync" by comment) — single source of truth.
- **N2 — `consume_fee` estimate withholding has no per-fragment $0 floor**
  (fold.rs:411, `c.gain_basis - estimate_share` summed): a cent-scale rounding residue could make a
  promoted fragment's contribution slightly negative, marginally REDUCING the survivor's re-homed
  carry basis. Direction is conservative (more future gain, never a loss), so harmless; a
  `.max($0)` per fragment would be tidier.
- **N3 — [T8] the void-of-inert-tranche / mixed gift+disposal advisory copy** can read
  promote-shaped or self-contradictory. Advisory text only (no filed number); soften the verb /
  attribute the gift portion. Agreed Minor→Nit.
- **N4 — [T11] verify "drift advisories: 0" vs TUI-hidden**, and the "documented on-chain fee basis"
  methodology header on a promote-only return — cosmetic copy consistency.

## Triage adjudication (independent)

Every triage item is code-quality / copy / cosmetic — none reaches Critical/Important on my own
re-derivation. [T2]→M3, [T7]→N1, [T8]×2→N3, [T5]/[T11]×2→N4, [T10]×2 (365-day threshold, double
consent print)→Nit (correctness unaffected), [T6] partial-removal cent-residue: I verified the clamp
absorbs the residue without manufacturing a loss on every band, so a characterization KAT is
nice-to-have, not required. Nothing on the list is under-rated as a hidden Critical/Important.

## KAT integrity

Non-vacuous. The clamp KATs assert `gain == $0` in the crowd-out band (not `< 0`), `gain < 0` only for
the genuine documented loss, `basis == $5,000 / $13,000 / $0` at exact values; the BG-D11 KATs assert
the COMPUTED Schedule A line 12 (`full_return_noncash_12`) alongside the fold deduction and the 8283
column, and include the LT/ST split, the non-promoted identity, and the evaporate-but-keep-$30
decomposition. The fee-evaporation KAT uses a $0-proceeds discriminator that turns the un-evaporated
leak into a REAL reported loss (kills the mutation). No KAT asserts the wrong thing.
