# SPEC review — Defensive Filing wizard — US-federal-tax-correctness lens, round 3 (Opus)

**Artifact:** `design/defensive-filing-wizard/SPEC.md` @ commit `cc44402` (post-r2 fold: the two blocking
findings — my r2 tax **I-A** + the arch r2 **C-1** — collapsed into ONE displacement-based DFW-D5.3
predicate). **Reviewer:** Opus, tax lens (independent; re-derived from the current tree, not anchored on
r2). Load-bearing citations re-verified against source on `feat/conservative-filing` @ `cc44402`:
`project/pools.rs`, `project/fold.rs`, `project/resolve.rs`, `project/transition.rs`,
`conservative_promote.rs`, `conservative.rs`, `state.rs`, `cmd/promote.rs`, and the shipped promote KATs
(`kat_promote.rs`, `promote_cli.rs`). I did NOT trust the SPEC's self-citations.

## Verdict

**GREEN — 0 Critical / 0 Important / 2 Minor / 3 Nit**

Both r2 blockers are genuinely resolved by the displacement predicate, and the resolution is grounded in
real source (not the SPEC's self-citations). The predicate is tax-sound, computable, method-robust, and
transition-safe. The residue is UX/completeness only (no filed-number effect).

---

## I-A / C-1 / N-1 resolution note

- **I-A (my r2 Important — partial over-coverage re-mints understated gain) — RESOLVED.** The hazard is a
  CURRENT-return understated gain when a real disposal draws the promoted `>$0` floor **in place of
  documented basis it would otherwise draw**. I re-confirmed the mechanism is REAL: `resolve.rs:1299-1307`
  rewrites the promoted tranche's `Op::Acquire.usd_cost` to the stored `filed_basis` **before** the lot is
  pooled, so the lot enters HIFO ordering at `>$0` — it exits `hifo_cmp`'s `usd_basis==Usd::ZERO` sort-last
  case (`pools.rs:276-280`) and can reorder AHEAD of documented lots. The partial-over KAT (declare 100M →
  later 60M in-pool import before the disposal, floor per-sat > documented): WITHOUT-promote the disposal
  draws 60M documented + 40M tranche(`$0`); WITH-promote it draws 100M tranche(floor), **displacing the 60M
  documented** → the floor is filed on 60M sat the vault documents at a lower basis → understated gain. The
  displacement predicate fires (a non-`EstimatedConservative` leg in the without-fold is replaced by the
  tranche floor in the with-fold), remedy = refuse + void + re-declare (whole-tranche-only forbids clamp —
  §4). This is strictly more precise than my r2 "sat-aware" proposal: it refuses **exactly** when the
  current return would be wrong. (The reverse per-sat ordering — floor < documented — leaves the current
  disposal HONEST and only strands a fictional residue; correctly NOT refused. See M-1 for the advisory
  gap on that case.)

- **arch C-1 (undisposed forward-promote wrongly refused) — RESOLVED, tax-soundly.** The displacement
  predicate does NOT fire on a fully-undisposed tranche: nothing draws it, so no leg in EITHER fold
  references it → no displacement. The shipped `ConsentTerm::Unrealized` path (`conservative_promote.rs:412-427`,
  emitted for `with_state` remaining sat) is preserved on both surfaces. I verified this is not just an
  assertion: **every verb-level promote KAT promotes over an undisposed/empty vault** — `vault_with_tranche`
  seeds no imports (`promote_cli.rs:297`), and `vault_with_promoted_disposal_via_cli` promotes via the verb
  BEFORE appending the sell (`promote_cli.rs:761` then `:772`). The one displacing scenario
  (`build_promoted_vault`, `promote_cli.rs:59-121`: 60M documented BUY@$3k + 40M tranche→$12k floor + 40M
  SELL) is **hand-built via `append_decision`, bypassing the verb**, and used only for void-path tests. So
  DFW-D5.3's chokepoint refusal changes **no shipped promote KAT** — the SPEC §5 claim (line 316-317) is
  TRUE, and "undisposed-vs-displacing" is a clean, consistent partition (a tranche the with-promote fold
  draws in place of documented basis is, by construction, NOT fully-undisposed in that fold).

- **N-1 (my r2 Nit — fee-only promote is a tax no-op) — RESOLVED.** Folded into DFW-D3 (line 94-97) as a
  UX SHOULD (suppress/annotate the promote branch on fee-only coverage). Tax-grounded: `consume_fee` draws
  fee-sats FIFO (`pools.rs:62` → `Fifo`, method-independent), and for a promoted-tranche fee fragment the
  estimate component **evaporates** (`fold.rs:414-419`: `c.gain_basis - estimate_share_of(entry, c.sat)`),
  so promoting a fee-only-covering tranche reduces no gain. Correctly a SHOULD, not a gate.

- **arch m-1 (per-event `short_sat`) — RESOLVED, tax-correct.** Verified a single `Dispose` event emits a
  principal short (`fold.rs:709`) AND, via `consume_fee`, a fee short (`fold.rs:388`) — two
  `UncoveredDisposal` on one `EventId`, differing only in the detail string. DFW-D7's per-event `short_sat`
  aggregate + **event-level** clearance ("no `UncoveredDisposal` remains on the target event", line 197-198)
  is the tax-correct target: it forces BOTH shorts cleared before the target is declared covered, and the
  DFW-D4 classifier keying on `short_sat` presence (never the detail string) stays total.

## Derivability / soundness audit (verified)

- **Computable.** `DisposalLeg` (`state.rs:145-158`) and `RemovalLeg` (`state.rs:182-195`) both carry
  `basis_source: BasisSource` and `lot_id: LotId`, and `LedgerState.promoted_origins` (`state.rs:285`)
  identifies a promoted leg by `lot_id.origin_event_id`. So "a documented (non-`EstimatedConservative`) leg
  in the without-promote fold is replaced by an `EstimatedConservative`-floor leg in the with-promote fold"
  is directly derivable from the leg data — the same with/without-promote fold pair `consent_terms` already
  builds (`conservative_promote.rs:271-273`, over disposal ∪ removal legs `:297-315`).
- **Method-robust.** Under an elected FIFO/LIFO the draw order is basis-INVARIANT, so the with- and
  without-promote folds draw identical lots in identical order — no documented lot is ever displaced by a
  promote, and the predicate correctly stays silent (there is no reorder-hazard to catch). Displacement is
  a HIFO-family phenomenon, and the predicate detects it wherever the actual elected-method fold produces
  it. No false-refuse and no false-pass across methods.
- **Transition-safe (Path-A).** `seed_transition` PathA keeps the tranche's `EstimatedConservative` tag
  and basis/`acquired_at` across the 2025 boundary; documented lots become `ReconstructedPerWallet`
  (`transition.rs:96-106`) — still non-`EstimatedConservative`, so the predicate's documented-vs-estimate
  discrimination holds post-transition.

---

## Minor

### M-1 (DFW-D5.3) — the derived "over-covered by N sat" ADVISORY is displacement-based, so it misses reverse-per-sat-ordering over-sizing

The REFUSAL is correctly displacement-based (it fires only on a current-return harm). But the SPEC also
promises a derived dashboard **advisory** — "over-covered by N sat — void + re-declare" (line 168) — as the
mirror of didn't-cover, "Same shadow-projection; derived." In the **reverse per-sat ordering** (a recovered
documented import whose per-sat basis EXCEEDS the promoted floor), the current disposal draws documented
first, then tranche, in BOTH folds — no displacement — so a displacement-derived advisory shows NOTHING,
yet the tranche is genuinely over-declared and strands a fictional `>$0` residue lot. No filed-number effect
(the current return is honest, and any future disposal that DOES draw the residue in place of documented
basis is itself a displacement the predicate would catch at that promote's evaluation), so this is below
Important — but the filer is never told to clean up the over-declaration.
**Fix:** derive the over-covered ADVISORY from a sat-count comparison (tranche live sat > the sat it
actually covers in the without-promote fold), independent of the displacement REFUSAL. Keep the refusal
displacement-based (only refuse a current-return harm); let the advisory surface all over-sizing.

### M-2 (DFW-D6 × DFW-D5.3) — the pseudo-off enumeration doesn't name the new displacement shadow projection

DFW-D6's binding mandate — "**EVERY** chokepoint shadow projection ... MUST force `pseudo_reconcile =
false`" (line 181-182) — covers the DFW-D5.3 displacement projection, but its illustrative list
(discovery / DFW-D5 clearance / consent-savings) predates the displacement predicate and doesn't name it.
This matters on the CLI verb, which has no journey pseudo-gate (line 183): if a plan-writer implements the
displacement check as a SEPARATE projection and forgets pseudo-off, a `SelfTransferMine{$0}` synthetic lot
(`resolve.rs:~1156`) could mask a real documented displacement → a displacing promote FALSE-PASSES →
understated gain. The mandate's "EVERY" already binds it, so this is a completeness Minor, not a missing
guarantee.
**Fix:** add the DFW-D5.3 displacement shadow to DFW-D6's enumeration, or pin that it reuses
`consent_terms`' (pseudo-off-corrected) with/without-promote fold pair rather than a fresh projection.

---

## Nit

- **N-1 (DFW-D5.3 wording).** The displacement predicate must inspect leg **`basis_source` composition**
  (a documented, non-`EstimatedConservative` leg in the without-fold replaced by an `EstimatedConservative`
  floor leg in the with-fold), NOT a bare leg-set inequality — a correctly-sized cover ALSO changes its
  legs (`$0`→floor on the SAME tranche lot) and a bare inequality would false-refuse it. The clause "in
  place of the documented (non-`EstimatedConservative`) basis it would otherwise draw" mandates this, and
  the "a correctly-sized cover promotes" KAT (line 299) guards it — the plan should state the basis_source
  discrimination explicitly so it is not re-derived as a leg-eq check.
- **N-2 (DFW-D5.3 / §8 framing).** Placing the displacement refusal on the SHARED chokepoint also closes a
  latent **sub-project-1 CLI** understated-gain gap: the shipped `promote_tranche` verb has no
  shortfall/displacement guard (`cmd/promote.rs:364-488` — gates stop at `would_conflict`), so a filer who
  declares → imports documented coins → promotes via the CLI files understated gain **today**. Like the
  DFW-D6 pseudo gap (§8), this could be filed against sub-1; noting it makes the both-surface fix (no
  CLI-vs-dashboard carve) explicit rather than incidental.
- **N-3 (post-promote drift — pre-existing, out of scope).** A promote-time gate cannot police FUTURE
  events: a recorded promote can become displacing when later imports/disposals land, and the actual fold
  files its stored floor regardless. This is a pre-existing property of the shipped promote mechanism (not
  introduced by this fold; the DFW-D5.3 gate strictly improves the promote-time picture). A derived
  dashboard "this recorded promote is now displacing — consider voiding" advisory (mirror of the shipped
  `promote_drift_advisory`, `conservative_promote.rs:89`) would complete it. Non-gating.

---

## Verified sound (do not re-litigate)

- The displacement mechanism is source-real, not just plausible: `resolve.rs:1299-1307` (Acquire.usd_cost
  → floor before pooling) + `pools.rs:276-280` (`usd_basis==0` sort-last) is the load-bearing pair; the
  BG-D4 clamp (`conservative_promote.rs:179-192`) files `min(net−documented, estimate)`, and the
  `filed_basis_for` `Coverage::Full`-only refusal (`:50-69`) is unchanged.
- The refusal on the shared chokepoint is correct on BOTH surfaces (promoting a displacing tranche is never
  legitimate on either — it always files understated gain), so no CLI-vs-dashboard carve is needed;
  contrast DFW-D5.2's declare-side `target_shortfall:None` carve, which is still correct (declare files
  nothing `>$0`).
- The §5 behavior-preservation carve is honest: the ONLY intended shipped-behavior change is the DFW-D6
  pseudo-off correction (a sub-1 bug fix); the displacement refusal changes no shipped promote KAT
  (verified — all verb-level promotes are undisposed).
- DFW-D7 per-event aggregation, DFW-D4 total-by-`short_sat` triage, DFW-D10 three-flavor discipline, and
  DFW-D11 fold-diff export set all remain tax-correct after the fold (unchanged from r2's "verified sound").

*End r3. GREEN: 0 Critical / 0 Important / 2 Minor / 3 Nit. Both r2 blockers (tax I-A + arch C-1) resolved
by one displacement-based predicate that is tax-sound, computable, method-robust, and transition-safe; the
residue is UX/advisory completeness with no filed-number effect.*
