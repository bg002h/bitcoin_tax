# Architecture review — Defensive Filing SPEC (r4, Opus lens)

**Artifact reviewed:** `design/defensive-filing-wizard/SPEC.md` (binding decisions DFW-D1..D12),
commit `93bef99` on `feat/defensive-filing-wizard` (HEAD).
**Stance:** independent SPEC-level architecture re-review after the r3 fold. My job: (a) verify my r3
**Critical (C-1)** — the un-narrowable over-coverage hard-refusal on the shared promote chokepoint — is
genuinely resolved by the demotion to a derived dashboard advisory, and (b) find any NEW defect the fold
introduced. Every load-bearing claim below is re-derived from current source, not carried forward from r3
and not anchored on the SPEC's self-citations.

---

## Verdict

**GREEN** — **0 Critical / 0 Important / 0 Minor / 2 Nit**.

C-1 is resolved. The r3 fold applied exactly the option my r3 review identified as the only sound one:
**demote the over-coverage/displacement check to a derived dashboard advisory and leave the shared
promote chokepoint's gate set unchanged.** Re-derived against source, the demotion introduces no new
gating authority, no persisted state, and no moved/weakened guarantee; the two DFW-D5.3↔DFW-D11 /
§5-behavior-preservation contradictions that constituted r3 C-1 are both gone. The advisory is genuinely
derivable from shipped state and mirrors a shipped derived-advisory pattern. Only two documentation Nits
remain (a stale "refusal" fossil in §5; a "mirrors `promote_drift_advisory`" phrasing that is
pattern-deep, not semantic) — neither gates.

---

## C-1 resolution note (verified against source)

**C-1 — RESOLVED.** The r3 Critical was that a ledger-structural over-coverage/displacement predicate on
the *shared* promote chokepoint necessarily false-blocks the legitimate, shipped `mixed_vintage` HIFO
reorder (the hazard and a legitimate reorder have identical `(T,S)` signatures; only the filer's
attestation distinguishes them, and DFW-D3 forbids persisting the per-tranche target that could link
them). The fold removes the chokepoint refusal entirely. Source confirms the three legs of the fix:

1. **Shared gate is genuinely behavior-preserving.** `cmd/promote.rs::promote_tranche`
   (`promote.rs:364-488`, full re-read) has **no** shortfall / over-coverage / displacement guard: the
   pipeline is resolve-live → BG-D5 provenance (`:381`) → BG-D7 Part II (`:386`) → BG-D3
   `filed_basis_for` (`:397`) → BG-D6 `consent_terms` (`:410`) → T8 advisory (`:433`) → gift-only
   classify (`:449`) → consent render (`:453`) → `require_promote_ack` (`:458`) → `would_conflict`
   (`:477`) → append. DFW-D5.3 adds nothing to it. So the SPEC's "the shared promote chokepoint stays
   behavior-preserving — NO over-coverage guard is added" is **TRUE against the code**, and §5's "changes
   no shipped promote KAT" is TRUE.
2. **The C-1 counterexample still promotes.** `mixed_vintage_hifo_2018_disposal` (`kat_promote.rs:1527`;
   CLI mirror `build_promoted_vault`, `promote_cli.rs:59`) — a documented 2017 60M-sat lot + a promoted
   40M-sat $12,000-floor tranche + a 2018 40M-sat sell whose HIFO draw the floor reorders — is a
   recordable state today (the shipped verb has no guard) and remains so; the demoted check does not
   touch it. The advisory-fires KAT `undisposed_promote_that_hifo_reorders_a_prior_year_fires_the_advisory`
   (`kat_promote.rs:1680`) — the whole warn-not-forbid subsystem — stays reachable. DFW-D5.3 and §5 now
   both name this KAT as the proof-of-unchanged-promote.
3. **No moved/weakened guarantee.** The `ConsentTerm::Unrealized` forward-promote path
   (`render_term` `promote.rs:310-326`; `consent_terms` `conservative_promote.rs:410-427`) is untouched;
   correctly-sized covers and `mixed_vintage` reorders all promote unchanged. The only intended behavior
   change remains the DFW-D6 pseudo-off correction (a sub-1 bug fix), correctly and singly flagged.

The r3 contradictions are closed: **DFW-D5.3 no longer contradicts DFW-D11** (D11 exports reorder-promotes;
those promotes now proceed rather than being refused), and the false §5/DFW-D2 "second behavior change"
clause is gone (there is no second behavior change — the advisory is a derived read-only surface). The r3
false premise ("promoting a displacing tranche is never legitimate") is explicitly retracted: DFW-D5.3 now
states the hazard is "ledger-identical to a legitimate vintage reorder … Only the filer's provenance
attestation distinguishes them," which is exactly the ledger-identity argument that made the hard gate
unsound. Resolved.

---

## Verify-2: the advisory seam is architecturally clean (source-checked)

- **Derived, in `journey_view`, no chokepoint gate.** `journey_view`/`JourneyView` do not yet exist in
  source (grep-empty) — this is new derived core work under DFW-D1(a) (pure, KAT-able, no session). The
  advisory is non-gating; the shared chokepoint stays the single gating authority (DFW-D1 I-2 preserved).
  No second gating authority is introduced.
- **No persisted state (DFW-D3).** Both advisories (over-covered sat-count; "recorded promote is now
  displacing") are derived from a with/without shadow fold + `basis_source` composition — nothing is
  written. This is precisely why the hard gate was impossible and the advisory is not: the advisory needs
  no per-tranche target linkage (it is a heuristic surface the filer adjudicates), so DFW-D3 is not
  reversed. Consistent.
- **Genuinely derivable from shipped state.** `BasisSource::EstimatedConservative` (`event.rs:36`) is the
  promoted-tranche tag that "SURVIVES a PromoteTranche" and reaches every disposal leg; `DisposalLeg` /
  `RemovalLeg` carry `basis_source` (`state.rs:152,188`). So "a documented leg in the without-fold
  replaced by an `EstimatedConservative` floor leg in the with-fold" is computable from real leg fields,
  and the sat-count comparison is trivial. The tax-N-1 refinement ("basis_source COMPOSITION, NOT a bare
  leg-set inequality") is well-founded — `state.rs:284` documents that two `>$0` `EstimatedConservative`
  legs are "indistinguishable from the leg alone," so a bare leg-set diff would be ambiguous; keying on the
  documented→estimate replacement is the correct signal.
- **Mirrors a shipped derived-advisory pattern.** `promote_drift_advisory` (`conservative_promote.rs:89`)
  is the precedent: reads the live-promote set from `resolve`, recomputes, emits `Vec<String>`, writes
  nothing, leaves the fold unchanged. The new advisories share that shape. The DFW-D6 requirement that the
  advisory's with/without folds force `pseudo_reconcile = false` (mirroring `would_conflict`,
  `project/mod.rs:119`) is correctly imposed. Clean.

## Verify-3: consistency after the fold

- **No DFW-D# now contradictory.** DFW-D5.3↔DFW-D11 reconciled (above). DFW-D6 (`:188`) correctly names
  "the DFW-D5.3 over-covered / drift-advisory" folds among the pseudo-off shadows — reflecting the
  advisory framing, not a refusal. DFW-D3's advisory-rows family (method-inversion / tranche-dip) and the
  new over-covered/displacing advisories sit in the same derived-dashboard-advisory class with no tension.
- **§8 known-limitation framing is sound.** The "CLI displacement gap" is correctly characterized as a
  **pre-existing sub-1 property**, not introduced here: source confirms the shipped `promote_tranche` has
  no displacement guard (verified full function), so a CLI declare→import→promote can already file
  understated gain today. The SPEC does not claim to fix it (it is ledger-identical to `mixed_vintage`, so
  unfixable by any hard gate without a DFW-D3 reversal that is out of scope); it surfaces it via the
  dashboard advisory for dashboard users and rests the CLI path on the mandatory Form 8275 + §6662 regime.
  That is the architecturally honest disposition — a genuine sub-1 limitation of the attested-provenance
  model, recorded not gated. The tax lens (which wanted the gate) was adjudicated on the decisive
  architecture point and reached GREEN at r3; no cross-lens re-open from this side.
- **Phasing coherent.** P-A (chokepoint extraction + DFW-D6 pseudo fix) is unaffected. The derived
  advisory lands naturally in P-B (`journey_view` + dashboard fork rows), leaning on the same shadow-fold
  machinery as the P-C clearance check; the sub-2b escape hatch covers any ballooning. No phase is
  invalidated by the fold.

## Verify-4: new defects introduced by the r3 fold

None Critical/Important. The demotion strictly *removes* a gate; it adds a derived read-only surface that
cannot silently answer for the filer (it surfaces a concern and leaves the choice — it improves the G-1/
answered-ness posture rather than threatening it). tax-I-A partial over-coverage (T=100M, S=40M) is still
**detected** by the sat-count advisory (tranche exceeds covered sat by 60M, both per-sat orderings per
tax-M-1) — only the block is dropped, and the block was the unsound part. The KAT mutation (derive from
displacement-only → the reverse-ordering over-size shows nothing → reds) correctly pins the sat-count
derivation over a displacement-only one.

---

## Findings

### Nit N-1 (§5 / DFW-D5.3) — stale "refusal" fossil

§5's last bullet (`SPEC.md:324-325`) reads "C-1's over-coverage **refusal** must NOT change any shipped
promote KAT — the undisposed-still-promotes KAT proves it." After the demotion there is no over-coverage
refusal; the noun is a fossil from the r2/r3 framing. The *claim* is true and the cited KAT proves it, so
this is cosmetic. **Fix:** reword to "the over-coverage **advisory** (DFW-D5.3, now non-gating) changes no
shipped promote KAT — `mixed_vintage`/undisposed/correctly-sized all still promote."

### Nit N-2 (DFW-D5.3) — "mirrors `promote_drift_advisory`" is pattern-deep, not semantic

The displacing advisory "mirrors the shipped `promote_drift_advisory`" (`SPEC.md:169`). The mirror is
**architectural-shape** only (derived, read-only, recompute-from-state, `Vec<String>`): `promote_drift_
advisory` detects **price-data drift** (stored floor vs today's recompute), whereas the new advisory
detects **basis_source composition** displacement — different signals. The SPEC already spells out the
composition detection at `:168-171`, so a careful plan-writer will not reuse the price-recompute logic;
but the bare "mirrors" could be over-read as semantic reuse. **Fix (optional):** qualify as "mirrors the
**derived-advisory pattern** of `promote_drift_advisory` (read-only, recompute-from-state), with its own
basis_source-composition signal."

---

## Lens answers (condensed)

**L1 (DFW-D2):** unchanged; contract complete/implementable; ack-inside-`apply` fail-closed; export trio
degenerate; full-driver parity. Sound.
**L2 (DFW-D4/D7):** `short_sat` per-event aggregate + event-level clearance unchanged and consistent.
**L3 (DFW-D5):** DFW-D5.1/5.2 sound; **DFW-D5.3's demotion to a derived dashboard advisory is sound and
resolves C-1** — behavior-preserving shared gate, derivable advisory, no persisted state, no second gate.
**L4 (DFW-D11):** two-set split clean and **no longer contradicts DFW-D5.3** (reorder-promotes proceed).
**L5/L6 (consistency / new):** no DFW-D# contradiction remains; §8 known-limitation is a sound pre-existing
sub-1 framing; the fold introduces no new gating authority or persisted state. Two doc Nits only.

---

*End r4 (SPEC, Opus). Verdict: **GREEN — 0C/0I/0m/2n**. C-1 resolved: the over-coverage check is demoted
to a derived, non-gating dashboard advisory and the shared promote chokepoint is verified behavior-
preserving against source (`promote.rs:364-488` has no guard; `mixed_vintage_hifo_2018_disposal` still
promotes; §5's "changes no shipped promote KAT" is TRUE). The advisory is derivable from `basis_source`
composition + sat-count and mirrors the shipped `promote_drift_advisory` pattern. No new architectural
defect; the two remaining items are documentation Nits.*
