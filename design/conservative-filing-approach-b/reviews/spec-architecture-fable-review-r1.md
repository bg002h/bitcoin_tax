# Architecture review r1 — `design/conservative-filing-approach-b/SPEC.md` (Approach B sub-project 1)

**Reviewer:** independent Fable architecture lens (adversarial; gate = 0 Critical / 0 Important).
**Artifact:** SPEC.md (DRAFT, design of record post-adjudication). Provenance trail read
(`reviews/DESIGN_PROVENANCE.md`); folded points were verified for resolution, not re-raised.
**Method:** every load-bearing code claim was checked against current source on
`feat/conservative-filing` (resolve.rs, transition.rs, fold.rs, pools.rs, conservative.rs, event.rs,
void.rs, cmd/tranche.rs, session.rs, forms.rs, render.rs, tags.rs, tui-edit form.rs/main.rs,
persistence.rs, tui export.rs, btctax-forms/packet.rs).

**Verdict: 0 Critical / 6 Important / 4 Minor / 3 Nit — NOT green.** The central ruling (BG-D1) is
sound and buildable, and its by-construction claims about the tag-keyed guarantee set are TRUE
against source. The blocking findings are at the edges the spec under-specifies: the BG-D4 clamp
formula and decomposition, BG-D9's engine-side semantics and advisory trigger, the BG-D7/D8
narrative/gate mechanics, and a false "exhaustive-match sweep" premise that leaves the new payload's
whole lifecycle surface to an un-census'd hand sweep.

---

## 0. Claims VERIFIED TRUE (the good news, with the code facts)

These BG-D1 "holds by construction, zero change" claims were checked and are correct:

1. **D-8 backstop.** `resolve.rs:1304` keys `has_tranche_residue` on
   `snap.estimated_conservative_remaining_sat > 0`; `transition.rs:76-80` computes it by filtering
   `l.basis_source == BasisSource::EstimatedConservative` over the pre-2025 Universal residue. A
   promoted lot keeps the tag (no new `BasisSource`), so the deny-effectiveness guarantee holds
   with zero change. Also verified: a promote that (via HIFO re-order, see I-4) causes the tranche
   to be fully consumed pre-2025 turns the backstop off *by design* (arch r4 Nit-2: nothing left to
   discard), and any stale allocation then fails the totals arm (`alloc_basis != snap.basis`,
   `resolve.rs:1306`) because the residue basis changed — no silent-discard path opens.
2. **Path-A seed exemption.** `transition.rs:97` (`if lot.basis_source != EstimatedConservative`)
   is tag-keyed; Path A carries `usd_basis` — so the FLOOR carries through the 2025 seed unchanged.
3. **Relocation carve.** `fold.rs:816-820` keeps the tag on relocation; the floor rides
   `c.gain_basis` → `usd_basis` (`fold.rs:811`). "A relocated promoted tranche keeps the tag + the
   floor" is true.
4. **Record-time refusals + mutual exclusion.** Both directions key on `DeclareTranche` payload
   presence + the voided set (`cmd/tranche.rs:53-76,91-116`; the four allocation sites chokepoint
   through `guard_allocation_vs_tranche`; TUI opener `session.rs:692-699`). A promote does not touch
   the target event → zero change, TRUE (but see M-1 for copy staleness).
5. **HIFO.** `pools.rs:275-283`: `usd_basis == ZERO` sorts LAST; a promoted lot exits that
   special-case and sorts by per-sat filed basis. "HIFO applied to the as-filed basis, not a bug" —
   accurate (but this same fact powers I-4).
6. **§7.4 allocation-void retirement.** `resolve.rs:1356-1382` — no interaction with the promote;
   the retraction is keyed on allocation ids and `SafeHarborUnconservable` only.
7. **The what-if seam.** `conservative.rs:298-313` (`overpayment_delta_one`) is exactly the swap
   the promote persists: `Op::Acquire.usd_cost` rewritten on the Decision-id
   `EstimatedConservative` acquire, `basis_source` untouched. The consent-screen reuse (BG-D6)
   works: pre-promote baseline = tax($0), with = tax(floor). (See M-2/M-3 for two traps in the
   analogy.)
8. **BG-D4 tractability precedent (spec doesn't cite it, but it exists and helps):** reporting a
   leg basis different from the consumed basis is ALREADY done — the §1015(a) NoGainNoLoss zone
   reports `basis = proceeds, gain = 0` while Σbasis conservation rides the pool-side pro-rata
   debit (`fold.rs:182-190` comment; `pools.rs:209,228`). The clamp is the same shape.
9. **Forms surface.** `forms.rs:154` reads `leg.basis` into col (e) — a promoted floor flows with
   no forms change; `how_acquired_from` (`forms.rs:279`) keeps `Review`. §3.7's "labels unchanged"
   is right (`tags.rs:40,66`, `render.rs:54`, tui-edit `form.rs:1766,1782`).
10. **BG-D2's code claim** — `event.rs:17-35` has no `Inherited` variant. TRUE.
11. **Phase 1b's hook exists** — `btctax-forms/src/packet.rs:7` documents the exhaustive
    destructure of `fill_full_return`. The 1a/1b split with ONE ship gate is coherent; 1a stands
    alone (content artifact + gate), no hidden dependency found beyond the stated 1b re-keying.
12. **Clean export / no watermark** (BG-D6/D8) is consistent with the standing full-return
    DRAFT-gate policy (attestation/DRAFT stays pseudo-only), and a `PromoteTranche` is a real
    decision — `pseudo_active` counts only synthetics, so no `[PSEUDO]` taint. TRUE.

**Guarantees that do NOT hold by construction** (each is a finding below): the BG-D4
never-understate clamp (new code — the spec knows; but its formula and decomposition are defective:
I-1, I-2); BG-D9 revocability/refusal at the engine + void-flow layer (I-3); the prior-year
integrity of already-computed years (I-4); the 8275 hard-gap gate (I-5); and everything keyed on
the new *payload* variant rather than the tag (I-6).

---

## CRITICAL

None.

## IMPORTANT

### I-1 — BG-D4: the clamp formula goes NEGATIVE in the enumerated fee corner
- **Defect:** `estimate-claimed basis = min(floor_share, net_proceeds_share)` produces a
  **negative** claimed basis whenever the leg's net-proceeds share is negative — a corner the
  parent SPEC itself enumerates as reachable (Invariant KAT corner (a): `fee_usd > proceeds`).
- **Failure scenario:** a promoted tranche sold in a disposal where `fee_usd > proceeds`
  (`fold.rs:634`: `net = round_cents(proceeds - fee_usd)` is negative; pro-rata leg shares of a
  negative net are negative). `min(floor_share, negative)` = negative ⇒ a negative Form 8949
  col (e) and an estimate-attributable gain *above* proceeds — nonsense output, and worse than the
  $0 filing it replaces.
- **Code fact:** `crates/btctax-core/src/project/fold.rs:634` (netting), `:137` (`split_pro_rata`
  shares); parent `design/conservative-filing/SPEC.md §3` Invariant KAT corner (a).
- **Fix direction:** clamp to the interval — `estimate_basis = min(floor_share, max(Usd::ZERO,
  net_share))` — and state it in BG-D4 so the KAT pins the corner.

### I-2 — BG-D4: the stated estimate/documented decomposition is falsified by the TP8(c) relocation carry
- **Defect:** BG-D4 defines the estimate component as "the lot's `usd_basis` share" and the
  documented components as "fee carry applied after" — but the TP8(c) **self-transfer** fee carry
  is re-homed **onto the LOT** (`FeeCarry::rehome_onto_lot`, `fold.rs:291-301`, called at
  `fold.rs:844`), merging documented fee basis INTO the tranche lot's `usd_basis` *before* any
  disposal. After a promote, `lot.usd_basis = floor + documented fee cents`, and "the lot's
  `usd_basis` share" is NOT the estimate.
- **Failure scenario:** the filer follows the product's own P8 recommendation — self-transfer the
  promoted tranche Exchange → SelfCustody with an on-chain fee under the TP8(c) default (the
  tranche lot is the last relocated lot → carries the fee basis). Later below-floor sale: clamping
  the whole `usd_basis` share clamps the documented fee cents away, contradicting BG-D4's own
  "documented components stay UNCLAMPED" and the §6 KAT ("the documented fee corners still reach
  negative — attribution intact"). The v1 disclosure copy (`conservative.rs:158-160`) documents this
  exact lot-merge case ("a `>$0` amount reflects documented on-chain fee basis re-homed onto that
  unit") — the spec's decomposition forgot it.
- **Code fact:** `crates/btctax-core/src/project/fold.rs:291-301` (`rehome_onto_lot` adds to
  `lot.usd_basis`), `:844` (SelfTransfer arm applies it to the relocated lot).
- **Fix direction:** define the estimate share from the **stored promote**, not the lot: per-sat
  floor = `filed_basis / tranche.sat` (both on/under the promote event), estimate share =
  per-sat floor × `leg.sat` (via `leg.lot_id.origin_event_id`); documented share =
  `c.gain_basis − estimate_share` (≥ 0), never clamped. BG-D4's "leg→tranche identity makes this
  tractable" then becomes actually true; as written it directs the builder to the wrong quantity.

### I-3 — BG-D9: dangling-promote semantics and the refusal's home are unspecified (record-time-only ≠ this codebase's own doctrine)
- **Defect:** "Voiding a `DeclareTranche` that has a live promote → REFUSED … never a dangling
  target" is specified as *behavior* with no *mechanism*, and the resolver's behavior for a promote
  whose target is voided/absent/wrong-type is never defined.
- **Why it matters here:** the engine's void-target classification applies a void of ANY
  non-listed decision via the `Some(_)` catch-all (`resolve.rs:484`), so a raw/hand-crafted void of
  the tranche **applies** at the engine layer regardless of a CLI guard — leaving a live
  `PromoteTranche` with a voided target. This codebase's own D-8 doctrine says record-time refusals
  are the friendly layer and the ENGINE invariant is the guarantee (`cmd/tranche.rs:5-6`), and it
  already hardens against hand-crafted vaults (the P9/T15 guard, `resolve.rs:396-405`). An
  internal consumer produces the dangling shape *today*: `session.rs:708-716`
  (`safe_harbor_residue`) filters OUT `DeclareTranche` events but would keep a `PromoteTranche`,
  projecting a promote with no target.
- **Failure scenario:** plan implements the refusal as a CLI-only guard (the natural reading);
  a raw void (or the residue projection above) yields a dangling promote; the resolver's rewrite
  pass finds no target — is that silently inert (the `overpayment_delta_one` `swapped=false`
  pattern), a Hard `DecisionConflict`, or UB? Whatever a given code path happens to do becomes
  load-bearing, unreviewed behavior.
- **Code fact:** `crates/btctax-core/src/project/resolve.rs:459-495` (void classification,
  `Some(_)` applies); `crates/btctax-core/src/void.rs:20-35` (`is_revocable_payload`),
  `:72-88` (the #7 effective-alloc exclusion precedent for "voidable but currently blocked");
  `crates/btctax-core/src/project/mod.rs:107` (`would_conflict` — the existing seam that surfaces a
  resolver-adjudicated conflict at record time); `resolve.rs:1358-1364` (void-of-effective-
  allocation → `DecisionConflict`, the exact precedent shape).
- **Fix direction:** specify the refusal as **resolver-adjudicated** (a void whose target
  `DeclareTranche` has a live promote → the void is inert + `DecisionConflict`, mirroring
  void-of-effective-allocation), which makes "never a dangling target" hold by construction and
  surfaces at record time for free via `would_conflict`; specify a hard `DecisionConflict` +
  promote-excluded for absent/wrong-type/voided targets (the pass-1d/1e validation pattern); and
  add the corresponding `voidable_decisions` exclusion so the bulk-void sweep cannot void a
  promoted tranche's target (today `DeclareTranche` is unconditionally in the candidate set,
  `void.rs:33`).

### I-4 — BG-D9: the prior-year-delta advisory trigger is mis-keyed — HIFO retroactivity is missed
- **Defect:** the advisory is gated on "a promote over an already-DISPOSED tranche" (pre-promote
  state). But under the no-election **HIFO default** (`fold.rs:43` `unwrap_or(LotMethod::Hifo)`),
  promoting an **undisposed** tranche whose per-sat floor exceeds a documented lot's per-sat basis
  re-orders a PRIOR year's draw (`pools.rs:267,275-283` — the promoted lot exits the sort-last
  special-case and now outranks cheaper documented lots), retroactively rewriting that year's legs,
  8949 rows, and computed tax — with NO advisory.
- **Failure scenario:** the feature's exact audience (mixed-vintage early adopter): documented 2016
  lot at $500/BTC + Q4-2017 tranche; 2025 HIFO sale consumed (documented-$60k, then documented-$500)
  while the tranche sat last at $0. 2026: promote the tranche to its ~$12k window floor → the 2025
  sale now draws the tranche instead of the $500 lot → 2025's computed tax silently changes; the
  "already-disposed" trigger (evaluated pre-promote) never fires, so no 1040-X advisory. This
  violates the design's own G-3 ("never made silently") and the advisory's stated purpose.
- **Code fact:** `crates/btctax-core/src/project/fold.rs:33-45` (`applicable_method` → HIFO
  default); `crates/btctax-core/src/project/pools.rs:272-283` (`hifo_cmp`).
- **Fix direction:** key the advisory on the **post-promote projection** — any promoted-tranche leg
  in a year strictly before the promote's record date (or, stronger, a before/after prior-year tax
  diff, which the engine can compute exactly as the consent screen already does) — and define
  "disposed" for the partially-disposed tranche (any leg vs. fully consumed), which the current
  wording leaves ambiguous.

### I-5 — BG-D7/D8: the REQUIRED Part II narrative has no home, and the "presence gate" being mirrored never refuses
- **Defect:** BG-D7 makes a filer-authored Part II facts narrative REQUIRED, but the spec never
  says where it lives (on the `PromoteTranche` event? recorded at consent time? a side artifact?),
  how it is authored/edited, or what its lifecycle is. BG-D8 then claims the hard gap "mirrors
  v1's `basis_methodology.txt` presence gate" — but that gate cannot refuse: the artifact is fully
  auto-generated and unconditionally written on export (`render.rs:871`, `:911`), and the TUI
  merely LISTS it (`tui/export.rs:110-118`); presence holds by construction, so no refusal
  machinery exists there to mirror. An 8275 with REQUIRED user content is the first artifact that
  can be *absent* at export time.
- **Failure scenario:** the plan mirrors the cited gate literally → the 8275 is auto-written with
  an empty/scaffold Part II → Reg §1.6662-4(f) adequacy (the entire legal point of BG-D7, per the
  adjudication) is defeated while the gate reports green. Or the plan invents a refusal surface
  ad hoc, unreviewed.
- **Code fact:** `crates/btctax-cli/src/render.rs:871,911` (unconditional write when `Some`);
  `crates/btctax-tui/src/export.rs:110-118` (listing only). The actual refusing-gate precedent in
  this codebase is the pseudo export block (`fold.rs:396-407`, "export/forms are BLOCKED while
  this is active").
- **Fix direction:** decide and state: (a) the narrative's home — the natural fit is capture at
  promote time on/alongside the event (consistent with BG-D6's "recorded ON the event" consent, and
  it makes the artifact present-by-construction again), with void+re-promote as the edit path; or
  (b) a deferred-authoring model, in which case BG-D8 must name the refusal mechanism (the
  pseudo-gate pattern, not the basis_methodology pattern) and the surfaces it blocks (CSV export,
  `export-irs-pdf`, full-return).

### I-6 — §4 Phase 1a / §3: "the exhaustive-match sweep" does not exist — the payload-side census is missing
- **Defect:** Phase 1a relies on "`PromoteTranche` schema + the exhaustive-match sweep", but there
  is **no exhaustive `EventPayload` match in product code** — every consumer has a catch-all, so a
  new variant compile-forces approximately nothing, and the spec provides no hand census for the
  payload-side surface (§3 enumerates only `== EstimatedConservative` sites). BG-D1 kills the
  tag-side invisibility class; the payload-side invisibility class is fully alive.
- **Concrete silent-miss sites (verified):**
  `void.rs:21` (`is_revocable_payload` `matches!` — unlisted ⇒ the promote is absent from the bulk
  and TUI void candidate lists, degrading BG-D9 revocability to the raw CLI path only);
  `session.rs:708-716` (`safe_harbor_residue` filter keeps a promote while dropping its target —
  feeds I-3's dangling shape);
  `resolve.rs:413` (`_ => Op::Skip` in `build_op`) and `resolve.rs:484` (`Some(_)` void
  classification) — both happen to do the right thing for a promote, but silently;
  `persistence.rs:96` (`_ => return None` fingerprint — correct, needs only the standard KAT);
  `cli/main.rs:2171` (`other => {other:?}` bulk-void summary) and `tui-edit/main.rs:3844`
  (`_ => ("?", …)` void-flow summary) — a promote renders as Debug/"?" in the exact flows BG-D9
  depends on.
- **Failure scenario:** the builder, told the compiler will find the sites, adds the variant, gets
  a clean build, and ships a promote that the void UIs don't list and the void flows render as
  `"?"` — no test red, no compile error, exactly the silent hazard class the spec claims to have
  structurally killed.
- **Fix direction:** replace the "exhaustive-match sweep" phrase with an enumerated **payload-side
  census** (the sites above + the record-time `would_conflict`/`voidable_decisions` items from
  I-3), per the standing whole-surface-sweep rule; optionally note that the only compile-forced
  sites are serde derives and any match the plan itself makes exhaustive.

## MINOR

### M-1 — §3 census: stale "$0" copy in the mutual-exclusion refusals and backstop blocker
`cmd/tranche.rs:95` and `session.rs:693-699` say "a … tranche (**$0** EstimatedConservative) is on
file", `TRANCHE_IS_FINAL_HINT` (`cmd/tranche.rs:29-31`) says "if you have already filed the
tranche's **$0** basis", and the backstop blocker detail (`resolve.rs:1311-1315`) says
"($0 EstimatedConservative) remains". Once a promoted (>$0) tranche exists, all four are factually
wrong user-facing copy. None is in the §3 list. Add them (copy-only; the predicates are correct).

### M-2 — BG-D3/D6: floor units are loose (per-BTC price vs whole-tranche basis)
`window_reference().min` and `overpayment_delta_one`'s `reference` are **USD-per-BTC prices**
(scaled internally at `conservative.rs:309` — the arch/tax I-2 fixture bug was exactly this class),
while the stored `filed_basis: Usd` must be the **whole-tranche** amount the fold emits as
`usd_cost`. BG-D6's "overpayment_delta_one reused with reference = the floor" must pass the price,
not the stored basis. Pin the units on both fields (`filed_basis` = whole-lot, computed
`round_cents(min × sat / SATS_PER_BTC)`) so the plan cannot mix them.

### M-3 — BG-D1: "exactly the `overpayment_delta_one` what-if seam, minus the discard" is a trap if read literally
The what-if mutates the timeline **after** `resolve()` returns (`conservative.rs:298-313`), so
pass-1 §7.4 effectiveness / `universal_snapshot` never see the swapped basis. A promote implemented
that way would let allocation conservation be adjudicated against a **different** pre-2025 residue
than the fold consumes (a promoted-basis HIFO re-order changes which lots remain), re-opening a
checked-vs-folded divergence on the Path-B discard. The normative text ("pass-2 Op-construction")
is correct — the promote map must be applied at/inside the resolve timeline build
(`resolve.rs:1085-1115` admit site or `build_op`), so the step-3 snapshot sees the floor. Demote
the analogy or add one sentence locating the rewrite inside `resolve`.

### M-4 — BG-D5/D6: the consent/attestation payload shape is unspecified
Every other payload field is typed; `acknowledgment: <recorded typed consent, BG-D6>` is not
(typed phrase string? bool + hash of the shown figures? timestamp?), and the spec is silent on the
non-interactive path (a typed acknowledgment in a scripted/non-TTY invocation — refuse, or a
`--yes-i-acknowledge <phrase>` form?). Precedent to cite: `timely_allocation_attested: bool`
(`event.rs:187`). One sentence each removes a plan guess.

## NIT

- **N-1:** `Coverage` (`conservative.rs:173`) has no `Serialize/Deserialize`; storing it on the
  payload forces the derive — list it in the plan.
- **N-2:** the standard forward-only vault-compat note (older binaries fail loudly on the new
  variant; precedent doc on `ReclassifyIncome`/`SelfTransferPassthrough`/`DeclareTranche`,
  `event.rs`) is not restated for `PromoteTranche`. Add the stock doc note + the stock
  no-fingerprint KAT.
- **N-3:** §3 could state the two verified no-change forms sites explicitly (8949 col (e) reads
  `leg.basis`, `forms.rs:154`; Form 8283 `how_acquired_from` stays `Review`, `forms.rs:279`) so the
  plan doesn't re-derive them.

---

## Census check (§3) — result

The `== / != EstimatedConservative` census over product code is **complete**: all non-test sites
are either in §3's list (conservative.rs items 1–4; labels item 7) or verified
hold-by-construction (transition.rs:78,97; fold.rs:816; resolve.rs:1304; forms.rs:279; tags.rs;
render.rs:54; tui-edit form.rs). No missed tag-keyed site found. The missing census is the
**payload-side** one (I-6) plus the copy strings (M-1).

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 6 |
| Minor | 4 |
| Nit | 3 |

Gate: **NOT green** (6 Important). All six are repairable in-spec: two BG-D4 formula/decomposition
corrections (I-1, I-2), two BG-D9 mechanism/trigger specifications (I-3, I-4), the 8275
narrative-home + gate-mechanism decision (I-5), and the payload-side census replacing the
"exhaustive-match sweep" premise (I-6). The core BG-D1 ruling needs no change.
