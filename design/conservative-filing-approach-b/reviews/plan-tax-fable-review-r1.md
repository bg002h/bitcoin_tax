# Plan review — US-federal-tax-correctness lens (Fable) — ROUND 1

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `ad16e3a` (branch `feat/conservative-filing-b`)
**Against:** `SPEC.md` (GREEN, 0C/0I both lenses, r5) + `reviews/spec-{tax,architecture}-fable-review-r{1..5}.md` + current source (all cited symbols/line numbers verified against the tree: `fold.rs` `make_disposal_legs` :192-202 / `make_removal_legs` :256 / `FeeCarry` :273-317 / `consume_fee` :347-374 / `claimed_deduction` :1231-1240; `conservative.rs` `window_reference` :193 / `overpayment_delta_one` :283-322 / `basis_methodology` :125-168 / `Coverage` :173-177; `return_1040.rs` `crypto_charitable_gifts` :524-553 / `charitable_carryover_out` :1311 / `apply_carryover_writeback` :1454; `compute.rs` :317/:436; `forms.rs` :154/:213/:439; `void.rs`; `resolve.rs` :1085-1114/:1305-1315/:1356-1382/:453-496; `persistence.rs`; `census.rs` `CENSUS_KEYS: [&str; 14]`; `packet.rs` both crates; `FormArg` cli.rs:959).

## Verdict

**NOT green. 0 Critical / 8 Important / 7 Minor / 3 Nit.**

The plan's engine core is faithful: T2's whole-tranche scaling, T4's `clamp(net, $0, estimate_share)` decomposed
from the STORED `filed_basis` (not `lot.usd_basis`), T5's fee-draw evaporation at `consume_fee`, T6's
removal-leg-builder decomposition, and T7's deferred void adjudication each match the spec ruling exactly, and I
verified all six downstream `leg.basis` consumers T6 declines to patch (fold `claimed_deduction` — computed from
the FINAL legs at fold.rs:1231-1240, i.e. after `rehome_onto_removal_leg`; `crypto_charitable_gifts`
short-basis at return_1040.rs:535; `forms.rs` 8949 :154 / SchD :213 / 8283 :439; `printed.rs`/`removals.csv`
downstream of `Form8283Row`) genuinely read `leg.basis`, so the "verified by KAT, not independently patched"
claim is **safe** — the `crypto_charitable_gifts` "reconcile with `claimed_deduction`" doc invariant also
survives because both read the same final legs. The Importants are: two spec-mandated KAT sets the plan *names
but never writes* (I-1), two spec-mandated surfaces with **no owning task** (I-2 drift, I-3 the advisory's
call sites), two BG-D6 consent behaviors the planned `ConsentTerm`/dispatch cannot express (I-4, I-5), a
**plan-introduced** 8275 Part-I defect (I-6), and two tasks whose KATs are vacuous or unconstructible as
specified (I-7, I-8).

---

## Important

### I-1 (Task 3) — The "Mutation to kill" cites a snapshot-timing KAT that no step writes; the ★ BG-D1 timing ruling (and two more §6 by-construction KATs) are pinned by nothing.

- **Defect:** T3 Step 1 writes exactly two KATs (rewrite+tag, D-8 backstop). Its Mutation-to-kill paragraph
  then claims "a snapshot-timing KAT (assert the promoted tranche's 2025-crossing basis carries the floor
  through the Path-A seed) reds it" — that KAT does not exist in any step of any task. Also absent from the
  whole plan: the §6 "a relocated promoted tranche keeps the tag + the floor" KAT and the §6 "STILL fires both
  record-time refusal directions" KAT (a **promoted** tranche must still refuse a `SafeHarborAllocation` at
  record time — cmd/tranche.rs:93-97 / session.rs:694, both tag-keyed but never exercised with a promote on
  file).
- **Tax failure:** the single most important ruling (BG-D1 ★ / arch r1 M-3) — rewrite INSIDE `resolve` so
  pass-1 §7.4 effectiveness + `universal_snapshot` see the floor — becomes unpinned. An implementer who moves
  the rewrite to the `overpayment_delta_one` post-`resolve` timing (the exact wrong precedent the plan itself
  cites as "the how") goes green: step-3 conservation is then adjudicated against a different pre-2025 residue
  than the fold consumes, silently mis-filing the 2025 transition after a promoted-basis HIFO reorder.
- **Authority/code:** SPEC §6 "Snapshot-timing (arch r1 M-3)" bullet; `resolve.rs` step-3 conservation at
  :1300-1315 keys on `snap.estimated_conservative_remaining_sat`/`snap.basis` — `alloc_basis != snap.basis`
  is floor-sensitive.
- **Fix:** add the three KATs to T3 Step 1 (snapshot-timing via a pre-2025 promoted tranche + allocation whose
  conservation outcome differs between floor-visible and floor-blind snapshots; relocation keeps tag+floor;
  a promoted tranche still refuses the allocation in both record-time directions).

### I-2 (no owning task) — BG-D3's direction-aware `verify` drift advisory is missing from the plan entirely.

- **Defect:** BG-D3 (and Global Constraints' own summary: "`verify` flags drift, direction-aware") mandates:
  fold uses the STORED number forever; a later bundled-price-data update is surfaced by `verify` as an
  advisory — direction-aware (stored floor recomputing ABOVE the current reference on a not-yet-filed position
  → "void + re-promote to the corrected lower number" G-4 hint; already-filed → advisory only). §6 pins it:
  "the stored number survives a price-data change (fold uses stored, verify flags drift, direction-aware per
  N-3)." No task implements or tests any of this; the T2→T3 stored-number chain covers only the fold half.
- **Tax failure:** a price-data correction that lowers the window min leaves an unfiled position silently
  carrying an overstated floor — an understatement the filer was never warned to correct; the G-4
  anti-overstatement guardrail the spec traded for dropping the v1 warn-if-above check is simply not built.
- **Fix:** add a task (or a T11 step + KAT): recompute `filed_basis_for` against current prices for each live
  promote, compare to the stored `filed_basis`, emit the direction-aware advisory; KAT with a mutated price
  fixture pinning BOTH directions and that the fold still uses the stored number.

### I-3 (Tasks 8/10 + file map) — `promote_prior_year_advisory` has no caller: the 1040-X/§6511 copy never reaches the filer in the PROMOTE direction, and the VOID direction (BG-D9's "the SAME advisory") is wired nowhere.

- **Defect:** T8 builds the fn (with `Direction::Promote|Void`) and KATs it directly, but no task calls it.
  T10's promote flow consumes only `filed_basis_for`/`consent_terms`/`would_conflict`; no task touches
  `cmd/reconcile.rs` (or the TUI void flow) to fire `Direction::Void` when a `PromoteTranche` (or its target)
  is voided; the File-Structure Map even promises "carryover-cascade naming hooks" in `tax/compute.rs` +
  `cmd/tax.rs` "(T8)" that T8's own Files/steps never touch (implementers see only their own task, so those
  hooks are unowned).
- **Tax failure:** BG-D9's mandate — *"Voiding a promote over a year whose fold diff changes reverts the books
  to `$0` while a filed return still claims the floor — an amend-to-PAY situation"* (tax r1 M-5) — is unmet:
  the filer voids, the books revert, the filed return overstates basis, and nothing fires. In the promote
  direction the consent terms (T9) carry figures but NOT the conditional "if Y was already filed … Form 1040-X
  … §6511" copy — that copy exists only inside an unreachable function.
- **Fix:** T10 Step 3 renders `promote_prior_year_advisory(.., Direction::Promote, ..)` on the consent screen;
  a new step (T8 or T12) wires `Direction::Void` into the void path for a `PromoteTranche` void AND a
  tranche-void-with-live-promote attempt; reconcile the file map's `compute.rs`/`cmd/tax.rs` hook claim
  (either assign the `carryforward_consistency`/`write_back_carryover` promote-aware copy per §3 item 8c to a
  task, or state the decided no-change); add an end-to-end void-direction KAT.

### I-4 (Tasks 9/1) — A removal-flagged year that COMPUTES loses its deduction-Δ: the `ConsentTerm` flavors + T9's computes→`ComputedTax` dispatch re-open the tax r3 I-2 blind spot.

- **Defect:** T9 Step 3: "if `compute_tax_year` computes both folds → `ConsentTerm::ComputedTax{year, delta}`;
  else → `Uncomputable{gain_delta, deduction_delta}`". But the spec (BG-D6, verbatim): "the deduction-Δ *must*
  use the fold-pair figure **even when the year computes**, because engine B's `compute_tax_year` excludes
  crypto donations by design (tax r3 I-2)", and BG-D9: "The tax-Δ figure must NOT be implied to capture the
  deduction effect." T1's `ComputedTax { year, delta_usd }` has no deduction field, so a computing year with
  BOTH a disposal reorder and a donation reorder records a tax-Δ that silently omits (and implies it includes)
  the Schedule-A change; a donation-ONLY computing year yields engine-B tax-Δ = $0, which T9's own "never emit
  `ComputedTax{delta:0}`" rule then forces into `Uncomputable` — recording *"tax not computable for year Y (no
  bundled table / no tax profile / blocked)"* for a year that computes fine: a false statement inside the
  §6664(c) good-faith artifact. No planned KAT exercises a computing removal-flagged year (T9's three KATs are
  disposal/no-profile only).
- **Fix:** give the computed flavor a deduction slot (e.g. `ComputedTax { year, delta_usd, deduction_delta_usd:
  Option<Usd> }` with copy that the tax-Δ excludes the deduction effect), route removal-flagged computing years
  through it, and add a KAT: 2024 (table ships) donation-reorder + profile → the term carries the fold-pair
  deduction-Δ and is NOT labeled uncomputable.

### I-5 (Tasks 9/1/10) — The BG-D6 unrealized line (undisposed sats) and its no-current-price fallback are unimplemented, and the planned `undisposed_promote_records_no_bare_zero` KAT cannot pass under the planned implementation.

- **Defect:** BG-D6 mandates, for sats not yet disposed: *"saving and exposure accrue at disposal; at today's
  price the floor would reduce reported gain by ~$X (hypothetical, not a filed figure)"* — never a bare $0 —
  with the tax r3 N-2 fallback (latest bundled close + date, or "no current price data — the floor itself,
  $`filed_basis`, is the maximum gain reduction"). §6 pins it ("NON-ZERO/unrealized-labeled for an undisposed
  tranche — tax r1 I-2"). T9 Step 3 ranges ONLY over fold-diff-flagged years; a fully-undisposed promote flags
  no year (no filed content changes), so `consent_terms` returns EMPTY — yet the KAT asserts
  `!terms.is_empty()`. Neither T9 nor T10 implements the unrealized line; `ConsentTerm` (T1) has no flavor to
  record it in `Acknowledgment.shown_terms`.
- **Tax failure:** the recorded consent for the feature's headline flow (promote a five-figure latent position
  before selling) is a bare nothing — the exact "$0 saving / $0 exposure is FALSE in both directions" defect
  tax r1 I-2 killed.
- **Fix:** add an `Unrealized { sat, hypothetical_reduction: Option<Usd>, as_of: Option<TaxDate> }` flavor (or
  equivalent), implement the today-price computation + the two-stage fallback in T9, render it in T10, and
  make the KAT assert the hypothetical label and the no-price fallback branch.

### I-6 (Task 13) — 8275 Part I "amount = the filed floor" misstates the return: the spec's item is the AS-FILED 8949 col (e) amount, and a REMOVAL leg (post-BG-D11) files documented-only, so a floor-amount 8283 item discloses a position the return does not take.

- **Defect:** T13 Step 3: "Part I = one item per promoted disposal/**removal** leg (form '8949'/'8283', …,
  **amount = the filed floor**)". BG-D7 (verbatim): "Part I auto-filled (**item = Form 8949 col (e)**;
  form/line/description/amount)". Two deviations the plan introduces: (a) on a CLAMPED disposal leg the filed
  col (e) amount is the clamped basis (= net proceeds), not the floor — disclosing the floor while filing less
  recreates the disclosed-method≠filed-amount examiner mismatch tax r1 M-4 exists to kill (the clamp sentence
  T13 adds explains a limitation; it does not license a wrong amount column); (b) after T6, a promoted
  removal leg files the DOCUMENTED component on 8283/Schedule A — an 8275 item "form 8283, amount = floor"
  affirmatively discloses an estimated-basis deduction position that BG-D11 guarantees is never taken (and an
  LT-donation-only year would generate an 8275 for a return with no estimated position at all). No KAT pins
  the amount column (the copy KATs check penalty text only), so this ships green.
- **Fix:** amount = `leg.basis` as filed per promoted 8949 leg (clamped where the clamp bound); drop removal
  legs from Part I (or, if disclosed at all, describe the documented-only treatment with the documented
  amount); add a KAT asserting the Part I amount equals the filed 8949 basis on a clamped fixture.

### I-7 (Task 8) — The KAT set under-pins three spec-mandated behaviors of the advisory itself: the gift-year NO-1040-X rule, the both-Δs-zero flagged year, and the §170(d) cascade direction.

- **Defect / mis-targeted KATs:** (a) BG-D9/§6 (tax r4 M-1): a GIFT-flagged year gets the "donee-basis
  documentation changes; the donor's Form 1040 is unaffected" copy and **NO 1040-X assertion** — T8's gift KAT
  asserts `contains("donee-basis") && !contains("$0 / $0")` but never asserts the ABSENCE of "1040-X", so an
  implementation that appends the 1040-X clause to every flagged year (falsely telling a donor to amend for a
  gift) passes. (b) BG-D9: "When BOTH Δs are `$0` for a flagged year … name the changed filed content (8283
  acquisition dates / donee-basis records) rather than a bare `$0`" — absent from T8 Step 3's copy spec AND
  from the KATs (§6 mandates the KAT: "a both-Δs-zero flagged year names the changed 8283 dates/donee records
  instead of $0"). (c) §6: "the §170(d) `write_back_carryover` direction is likewise named" — no KAT names
  §170(d)/Schedule-A-carryover on a donation-deduction-changing fixture (the loss-stealing KAT pins §1212(b)
  only). (d) No KAT exercises `Direction::Void` (§6: "in BOTH the promote and void directions") — pairs with
  I-3's unwired surface.
- **Fix:** extend T8 Step 3 with the both-zero copy rule; extend the KATs: gift KAT adds
  `!contains("1040-X")`; add a both-Δs-zero fixture; add a §170(d)-naming assert on a donation-Δ fixture; add
  one `Direction::Void` KAT (amend-to-pay copy).

### I-8 (Task 14) — Both BG-D8 gate KATs are defective as specified: the refusal state is unconstructible in Phase 1a, and the success-path assertion is vacuous (`|| basis_methodology.txt`).

- **Defect:** (a) The refusal KAT's setup is "simulate the artifact being unavailable (e.g. a promoted leg
  whose disclosure can't assemble)" — but by T13's own iff-KAT, `disclosure_8275` is `Some` whenever a promoted
  leg is filed, and the Part II narrative is present-by-construction (T10's record-time refusal), so no
  product path produces the refusing state and the plan names no mechanism (the honest one is the
  hand-crafted-vault class the spec uses for BG-D9: raw-append a `PromoteTranche` with an empty/scaffold
  `part_ii_narrative`, bypassing the CLI). "Simulate" unspecified is exactly how the gate gets implemented as
  dead code with a mocked test. (b) The success KAT asserts
  `out.join("form_8275.txt").exists() || out.join("basis_methodology.txt").exists()` — `basis_methodology.txt`
  is written unconditionally on every tranche export (render.rs:871/911, the always-written pattern BG-D8
  explicitly rejects as a gate), so the disjunction passes even when the 8275 content is never emitted: the
  1a packet could merge gate-green with NO 8275 content alongside a promoted leg — the mandatory-disclosure
  artifact (Reg §1.6662-4(f), BG-D7) silently absent.
- **Fix:** specify the raw-vault empty-narrative mechanism for the refusal KAT (and define
  `disclosure_8275`'s "incomplete" predicate = empty/scaffold Part II, so the gate has a real content
  condition in 1a); make the success KAT assert the 8275 artifact by its own name, no disjunction.

---

## Minor

### M-1 (Task 8) — Baseline-construction wording says "with the promote's **target** excluded"; the correct exclusion is the **PromoteTranche event**. Excluding the target (the `DeclareTranche`) deletes the lot, turns its disposals into `UncoveredDisposal` garbage, and diffs every tranche-touching year. The KATs would red this at implementation time, but fix the sentence — it is the plan's only description of the baseline.

### M-2 (Task 7) — "In step 3 … adjudicate `tranche_voids`" is the wrong location for an APPLYING tranche-void: the step-2 admit branch consults `voided` (resolve.rs:1086-1088), so a void adjudicated after the timeline build cannot take effect without a post-hoc timeline mutation (and step-3's `universal_snapshot` would count a voided tranche's residue). Since promote-liveness depends only on promote-targeted voids (all applied unconditionally in pass-1a), adjudicate at the END of pass-1a against the final promote set — still deferred/order-independent, satisfying arch r2 M-1. Also add the plain §6 "void → reverts to `$0`" KAT (void the promote alone; assert lot basis back to $0 with the tag intact) — `both_voids_either_order` only covers the tranche-also-voided end state.

### M-3 (Task 6) — §6 mandates the 8283 `cost_basis` column documented-only "for ST **and** LT donations"; T6's LT KAT asserts only the FMV deduction. Add `form_8283_cost_basis` (and, cheaply, the `removals.csv` cell) to the LT fixture — the LT column-honesty half of BG-D11 is otherwise unpinned.

### M-4 (Task 10) — The BG-D5 refusal KAT exercises only `ProvenanceKind::Gift`; §6 says "incl. a mined/earned/airdrop/fork filer". Iterate every non-Purchase variant and pin the closed-enumeration copy (tax r1 M-6) + the "model the real acquisition" pointer.

### M-5 (Tasks 10/11) — No KAT pins (a) the CONSENT-screen copy (§6's Copy bullet covers "the 8275/**consent** copy" — never "safe harbor", underpayment base; T13 pins the 8275 only) or (b) the P6 promote-funnel line quoting the CLAMPED delta (§3 item 2 / tax r1 I-3's second surface — an unclamped funnel advertises a saving the promote cannot deliver). Add a consent-render copy KAT in T10 and a funnel-figure KAT in T11.

### M-6 (plan-wide) — §3 items 6, 7, and 11 have no owning task: the parent SPEC D-7 re-scope + parent Invariant-KAT wording amendment (BG-D4's attribution sentence), the `event.rs` `DeclareTranche`/`EstimatedConservative` doc comments ("$0 ONLY (no floor)" at event.rs:214-220 is false post-promote), the now-false `forms.rs` "$0" doc sentence (T6 marks forms.rs "verify-only, NOT patched", which reads as forbidding the mandated doc fix — the no-patch rule is for the *basis consumers*, not the comment), and item 11's explicit `build_op`/void-classification arms for `PromoteTranche` (currently silent catch-alls). Assign them (T11's whole-surface grep is the natural home; T3/T7 for item 11).

### M-7 (Tasks 15/16) — `SUPPORTED_YEARS` underspecified for the re-pointed gate: with only a 2024 map, T16's PDF-keyed BG-D8 gate hard-refuses ANY promoted-leg export for a 2017/2025/2026 packet year — fail-closed, but it bricks the legitimate prior-year 1040-X flow the feature exists for. Form 8275 is revision-dated, not tax-year-versioned, so one blank can map every supported year; mandate coverage of every year the crypto slice can carry a promoted leg (or specify the refusal copy that tells the filer why).

---

## Nit

### N-1 (Tasks 1/10) — The consent phrase is "I understand and accept the risk" in T1's fixture but T10's KATs pass `Some(ATTEST_PHRASE)` ("I attest this is true"). Pick one; the recorded `Acknowledgment.phrase` should be the consent phrase, not the pseudo-attest phrase.

### N-2 (Task 10) — BG-D6's non-TTY rule requires the computed figures still printed to stdout on the `--i-acknowledge` path, and the consent copy list omits "plus interest" (BG-D10 copy). Neither is pinned.

### N-3 (Task 14) — The gate's year scope for the non-year-scoped CSV/snapshot export (`export_snapshot` has no single `year`) is unstated; specify "any year with a promoted filed leg in the exported range".

---

*Reviewer note: I re-verified the T4/T5/T6 decomposition chain against `pools.rs::Consumed` (:291-307) and the
three `FeeCarry` re-home sites — decomposing at `consume_fee`'s summation covers all three destinations, and a
tranche lot can never enter the dual-basis arm (`rehome_onto_lot` never promotes `dual_loss_basis` None→Some;
a `DeclareTranche` lot is born non-dual), so T4's non-dual-arm-only clamp placement is correct. The
`Disposal`/`Removal` `PartialEq, Eq` derives T8 assumes exist (state.rs:166/:201). The cent-scale possibility
of a −$0.01 documented removal-leg share is spec-blessed (BG-D4's documented/rounding carve) and conservative;
not filed as a finding.*
