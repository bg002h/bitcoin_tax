# SPEC review — Defensive Filing wizard — US-federal-tax-correctness lens, round 4 (Opus)

**Artifact:** `design/defensive-filing-wizard/SPEC.md` @ commit `93bef99` (post-r3 fold: the lens conflict
adjudicated FOR the architecture lens — DFW-D5.3's hard over-coverage refusal **demoted to a dashboard
advisory**; shared promote gate behavior-preserving; CLI gap → §8 known-limitation). **Reviewer:** Opus,
tax lens, independent, **re-derived** — I do NOT anchor on my r3 GREEN (which rested on the now-removed
displacement REFUSAL). Load-bearing citations re-verified on `feat/defensive-filing-wizard` @ `93bef99`:
`cmd/promote.rs:364-488`, `conservative_promote.rs` (`filed_basis_for:54-70`, `promote_drift_advisory:89`,
`clamped_leg_basis:180-192`, `consent_terms:258-441` incl. `Unrealized:410-427`), `pools.rs` `hifo_cmp:275-286`,
`project/fold.rs` UncoveredDisposal sites (`:388/:710/:833/:878/:1198/:1276`), `project/mod.rs:119`
`would_conflict` pseudo-off, `state.rs` (`basis_source:121/152/188`, `promoted_origins:285`),
`conservative.rs` (`promote_prior_year_advisory`, `method_inversion_advisory:61`, `tranche_dip_advisory:27`),
KATs `kat_promote.rs:1527` (`mixed_vintage`), `:1680`, `:2117` (`fully_undisposed`), `promote_cli.rs:54-59`
(`build_promoted_vault`). I did NOT trust the SPEC's self-citations.

## Verdict

**GREEN — 0 Critical / 0 Important / 1 Minor / 2 Nit**

The advisory demotion is tax-sound. I converge with the controller's adjudication on a **fresh** derivation
(not by deferring to it): my r3 GREEN wrongly assumed the displacement predicate refused *exactly* the harm;
the architecture lens correctly proved that false (`mixed_vintage` is a legitimate, timing-only reorder with
the **identical ledger signature** as the phantom double-count — verified below). A tool that cannot
distinguish hazard from legitimate use on the ledger MUST NOT hard-block in an attested-provenance regime;
loud advisory + mandatory Form 8275 + §6662 is the correct posture. My r3 M-1/M-2/N-1/N-3 all folded. The
residue is one advisory-derivation over-fire (no filed-number effect) + two Nits.

---

## Q1 — Is the advisory demotion tax-sound? YES (re-derived, not anchored)

**The hazard and a legitimate reorder are ledger-identical — verified in source, not asserted.**
`mixed_vintage_hifo_2018_disposal` (`kat_promote.rs:1527`): documented 2017 BUY 60M sat @ $3,000 (≈
$0.00005/sat), a 40M tranche promoted to a $12,000 floor (≈ $0.0003/sat), a 2018-09-01 SELL of exactly 40M @
$20,000. `hifo_cmp` (`pools.rs:275-286`) sorts `usd_basis == 0` **last** and non-zero by per-sat DESC, so the
$0 tranche (without-promote) sorts last → SELL draws documented → gain $18,000; the promoted floor (with-
promote) sorts ahead → SELL draws the tranche floor → gain $8,000, documented deferred. `build_promoted_vault`
(`promote_cli.rs:54-59`) is the same shape and its own comment names it *"the amend-to-PAY reorder the advisory
warns about."* A **phantom double-count** (the 40M tranche IS the documented coins, re-declared) produces the
**identical `(tranche, disposal)` ledger signature** — same lots, same HIFO order, same $8,000. The ONLY
distinguishing fact is whether the filer genuinely holds 100M sat (legit timing shift, total lifetime gain
unchanged) or 60M (double-count → understated gain). That fact lives in the **BG-D5 provenance attestation**,
never in the ledger, and DFW-D3/D8 forbid persisting the per-tranche target that could link them.

**Consequence for my r2 I-A.** My I-A premise — "the tool can and should refuse *exactly* the current-return
harm" — was **wrong**: the displacement predicate does not fire only on harm; it also fires on every
legitimate `mixed_vintage`-class reorder (a false-positive machine, removing a shipped capability). Given the
ledger-indistinguishability, the tool CANNOT prevent only-the-hazard — prevention necessarily false-refuses
legitimate, tax-correct reorders. So there is **no** tax-Critical understatement the tool "could and should
have prevented." The correct posture in an attested regime is exactly the one the SPEC now takes:

- **Loud dashboard advisory** surfacing the concern with the gain figures (answered-ness preserved: not
  silent on the surface the feature owns).
- **Mandatory Form 8275 Part II narrative** — enforced at the chokepoint (`cmd/promote.rs:381-389`, non-empty
  under Reg. §1.6662-4(f)); a knowing, DISCLOSED position, the §6662(d)(2)(B)/§6664(c) footing.
- **§6662 accuracy regime** governs the residual: if the attestation is false (double-count), that is the
  filer's §6662/§6663 exposure on a disclosed position, not a tool defect.

This is BG-1's own posture ("surface, never forbid the attested choice") and DFW-D11's warn-not-forbid for
this reorder class. The whole feature already trusts BG-D5 provenance; hard-blocking the promote after
trusting the attestation everywhere else would be incoherent. **Tax-sound. My r2 I-A is adequately addressed.**

## Q2 — r3 Minors/Nits folded (verified line-by-line)

- **M-1 (over-covered advisory = sat-count, both orderings, displacement-independent) — FOLDED.** DFW-D5.3:
  "a live tranche whose sat exceed the sat it actually covers in the without-promote fold — a **sat-count**
  comparison, covering BOTH per-sat orderings (tax-M-1), independent of whether a displacement currently
  manifests" (SPEC:163-166), + the §5 mutation KAT ("reverse-per-sat-ordering over-size shows nothing →
  reds", SPEC:306-307). Closes my r3 reverse-ordering gap. (One over-fire remains — Minor M-1 below.)
- **M-2 (DFW-D6 names the new shadow) — FOLDED.** DFW-D6 now enumerates "the **DFW-D5.3 over-covered /
  drift-advisory with/without folds (tax-r3 M-2)**" among the projections that MUST force
  `pseudo_reconcile=false` (SPEC:188-190). Verified the precedent is real: `would_conflict` forces it at
  `project/mod.rs:119`.
- **N-1 (basis_source COMPOSITION, not leg-set eq) — FOLDED.** DFW-D5.3 drift advisory: "detected by
  `basis_source` COMPOSITION — a documented leg in the without-fold replaced by an `EstimatedConservative`
  floor leg in the with-fold, NOT a bare leg-set inequality" (SPEC:169-171). Correctly discriminates the
  displacement (documented→estimate) from a correctly-sized cover ($0-estimate→floor-estimate on the SAME
  tranche lot, which is NOT a documented-leg replacement). Derivable: legs carry `basis_source`
  (`state.rs:121/152/188`); `promoted_origins` (`:285`) identifies promoted legs; `consent_terms` already
  builds the with/without fold pair (`conservative_promote.rs:271-315`). No new tax logic.
- **N-3 (drift advisory mirrors `promote_drift_advisory`) — FOLDED.** DFW-D5.3: the "recorded promote is now
  displacing" advisory "mirrors the shipped `promote_drift_advisory` (`conservative_promote.rs:89`)"
  (SPEC:168-171). Verified `:89` is a derived, read-the-live-set-from-`resolve`, never-writes informational
  builder — the right structural precedent (the KIND of drift differs — price-data vs displacement — but the
  derivation pattern is the mirror, which is all N-3 asked).
- **N-2 (my r3 CLI gap) → §8 known-limitation.** Addressed (Q3).

## Q3 — §8 CLI-gap known-limitation: tax-defensible? YES

`cmd/promote.rs:364-488` grep-confirmed to have **no shortfall/displacement guard** (gates: BG-D1 live →
BG-D5 provenance → BG-D7 Part II → BG-D3 floor → consent → prior-year advisory → gift-only → render_consent
→ ack → `would_conflict` → append). So a CLI `declare → import-documented → promote` can file understated
gain today. Framing it as a **non-hard-fixable known limitation** is correct: (i) it is **pre-existing sub-1
behavior**, not introduced by this feature (G-3: no new tax logic); (ii) it is **ledger-identical to
`mixed_vintage`**, so the arch adjudication applies to the CLI identically — no hard gate can fix it without
false-refusing legitimate reorders; (iii) the mandatory 8275 + §6662 backstop the disclosed attested
position; (iv) the wizard **adds** a dashboard advisory the shipped CLI never had — a net improvement, never
a regression. An owning follow-up is filed against sub-1 (§8, DFW-D5.3 residual). Correct per workflow. One
near-free improvement is available (Nit N-2 below), non-gating.

## Q4 — NEW tax defects from the r3 fold

One found (Minor M-1). No filed-number Critical/Important. No case where over-coverage becomes *invisible* on
the dashboard: the sat-count advisory fires on BOTH orderings for a genuine over-size, the drift advisory
catches an already-recorded displacement, and the correctly-sized cover correctly stays silent (live 40M =
covered 40M). The only remaining silence is the acknowledged §8 CLI gap.

---

## Minor

### M-1 (DFW-D5.3) — the sat-count over-covered advisory OVER-fires on a legitimately fully-undisposed forward-promote tranche

The r3 M-1 fold derives the over-covered advisory as "live tranche sat > the sat it actually covers in the
without-promote fold" (SPEC:163-164). For a **fully-undisposed** tranche the "sat it covers" = **0** (nothing
draws it; removing it creates no new shortfall). So `live 40M > 0` → the advisory **fires** on every
fully-undisposed tranche — including the shipped `Unrealized` forward-promote (`consent_terms:410-427`, KAT
`fully_undisposed_promote_records_an_unrealized_term_not_empty:2117`) and the `mixed_vintage` extra-holdings
tranche. Its remedy copy — *"void + re-declare at the covered size"* — is **wrong advice** for genuine
forward-holdings (covered size = 0; the coins are legitimately held, to be disposed later), and contradicts
DFW-D3's "a `$0`/forward tranche is never incomplete." The over-covered STATE is meant to be the *mirror of
didn't-cover*, which is gated on a **live shortfall existing** (SPEC:143-146); the over-covered phrasing
dropped that guard, so it degenerates to "any tranche with undisposed sat." No filed-number effect (advisory
only; the copy self-hedges "if genuinely your no-records coins, promoting is fine"), hence Minor not
Important — but a plan-writer implementing the text literally emits misleading advice on a shipped capability.
**Fix:** scope the over-covered advisory to a **shortfall-residual** context (mirror didn't-cover): fire only
when the tranche covers a real shortfall whose sat a documented import now partly supplies — i.e., `live sat >
the residual shortfall the tranche is still needed for`, NOT merely `> sat-consumed`. A fully-undisposed
forward tranche is the **Unrealized** state, not "over-covered." Add a KAT: a fully-undisposed forward-promote
tranche renders **NO** over-covered advisory (alongside the existing over-sized-fires KAT, SPEC:306).

## Nit

- **N-1 (§5 stale wording).** §5 still reads "C-1's over-coverage **refusal** must NOT change any shipped
  promote KAT — the undisposed-still-promotes KAT proves it" (SPEC:324-325). Post-demotion there **is no
  refusal**; it is an advisory (a dashboard derivation cannot change a promote KAT at all). The conclusion is
  correct and now stronger, but the premise word "refusal" is stale (the arch-r3 m-2 wording that this fold
  was meant to correct). Reword: "the demoted over-coverage ADVISORY changes no shipped promote KAT — the
  shared gate is behavior-preserving; the undisposed-still-promotes and `mixed_vintage` KATs prove the gate
  unchanged." Doc-consistency; no design effect. (Architecture lens likely also catches this.)

- **N-2 (§8 — near-free CLI advisory).** The over-covered/displacement advisory is derived from the
  with/without fold pair that the **shared chokepoint already computes** (`consent_terms`), and the CLI verb
  **already prints** derived advisory lines (`promote_prior_year_advisory`, `cmd/promote.rs:440-442`). So a
  **non-blocking** displacement advisory line could be surfaced at the chokepoint (byte-identical on both
  surfaces per DFW-D2) at ~zero cost and with **no hard gate / no persisted target / no removed capability**,
  closing the §8 CLI gap for CLI users, not only dashboard users. Its absence is disclosed (mandatory 8275)
  so this does NOT gate — but the SPEC scopes the advisory to "dashboard" when the chokepoint is shared and
  the derivation is free. Consider surfacing it at the chokepoint; at minimum record it as the concrete,
  non-hard-fixable improvement the §8 follow-up owns (vs. "8275/§6662 only").

---

## Verified sound (do not re-litigate)

- Demotion resolves the DFW-D5.3-vs-DFW-D11 contradiction (arch-r3 C-1): with no refusal, DFW-D11's
  reorder-export path and the `promote_prior_year_advisory` subsystem stay reachable (the promote records).
- §5 "no shipped promote KAT changes" is now TRUE and stronger — the shared gate is behavior-preserving; the
  ONLY intended shipped-behavior change remains the DFW-D6 pseudo-off correction (the sub-1 latent
  `Acknowledgment` bug fix; `cmd/promote.rs:396`/`398` threads stored `cfg` into `consent_terms`/advisory).
- DFW-D6's "EVERY shadow projection forces `pseudo_reconcile=false`" binds the new displacement/over-covered
  projections (SPEC:188-190); the CLI verb has no journey pseudo-gate, so the chokepoint-level force is
  load-bearing — correctly mandated.
- `clamped_leg_basis:180-192` (`min(net−documented, estimate)`) and `filed_basis_for` `Coverage::Full`-only
  refusal (`:65-70`) unchanged; the fee-evaporation no-op (DFW-D3 fee-only suppress) unchanged.
- DFW-D7 per-event `short_sat` aggregate (dispose short `fold.rs:710` + `consume_fee` fee short `:388` on one
  `EventId`) + event-level clearance, DFW-D4 total-by-`short_sat` triage, DFW-D10 three-flavor discipline,
  DFW-D11 fold-diff export set — all remain tax-correct (unchanged from r3's verified-sound).

*End r4. GREEN: 0 Critical / 0 Important / 1 Minor / 2 Nit. The advisory demotion is tax-sound on a fresh
derivation: the hazard and `mixed_vintage` are ledger-identical (source-verified), so no hard gate can spare
one and catch the other; loud advisory + mandatory 8275 + §6662 is the correct attested-provenance posture,
and my r2 I-A is adequately addressed. All r3 Minors/Nits folded. The one new Minor is an advisory
over-fire on fully-undisposed forward tranches (no filed-number effect); two Nits are doc-consistency and a
near-free CLI-advisory improvement.*
