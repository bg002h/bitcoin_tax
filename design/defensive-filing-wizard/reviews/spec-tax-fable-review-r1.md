# SPEC review — Defensive Filing wizard — US-federal-tax-correctness lens, round 1 (Fable)

**Artifact:** `design/defensive-filing-wizard/SPEC.md` (first draft, post arch-r2 "SOUND").
**Reviewer:** Fable, tax lens (independent; first tax-lens pass — the two prior critiques were
architecture-only). All load-bearing citations re-verified against current source on
`feat/conservative-filing` (fold.rs / resolve.rs / transition.rs / pools.rs / cmd/promote.rs /
cmd/tranche.rs / cmd/admin.rs / conservative_promote.rs / tax_tables.rs), not trusted from the SPEC.

## Verdict

**NOT GREEN — 0 Critical / 5 Important / 4 Minor / 2 Nit**

The composition's headline flow is tax-correct (see "Verified sound" below), and the SPEC's central
factual bet — that the sub-project-1 CLI can already fold pseudo numbers into the recorded §6664(c)
`Acknowledgment` — is **confirmed real in source**. The blocking findings are all at the seams the
wizard adds: the export year-set, the pseudo premise, the triage census, the missing over-coverage
state, and the clearance-check's scope.

---

## Important

### I-1 (DFW-D11) — The export year-set under-captures the BG-D9 amendment set: `promote_export_gate(None)` enumerates promoted **disposal**-leg years only

Verified: `cmd/admin.rs:85-98` builds the `None` year-set exclusively from `state.disposals` legs
whose `origin_event_id` is in `promoted_origins`. That enumeration exists to key the **8275
completeness** refusal (BG-D8) — and for that purpose disposal-legs-only is *correct*, because per
BG-D11 the estimate never files on a removal surface, so removal-only years need no 8275. But
DFW-D11 adopts it as **the wizard's whole export set** ("{current year} ∪ {years in which a promoted
disposal leg files}"), while the BG-D9 amendment set is the **fold-diff flagged set over disposal
AND removal legs** — strictly larger. Two under-export classes:

- **Removal-flagged prior years.** Promote an undisposed 2016-window tranche in 2026; the HIFO
  reorder changes which lots a 2025 **donation** drew (`Op::Donate` runs through the same
  method-elected `consume_principal`) → 2025's Schedule-A deduction / Form 8283 rows change with
  **zero** disposal-leg change. The BG-D9 advisory names 2025 (that machinery is removal-aware —
  the converged sub-1 r3 blocker), but the export step's set is `{2026}`: no 2025 1040-X packet.
  If the reorder *lowered* 2025's deduction relative to what was filed, the filed 2025 return now
  overstates a deduction and the wizard's "export the packet" step presented a complete-looking set
  that omits the year needing amendment.
- **Reorder-only prior years.** A promote that re-orders documented lots across years changes a
  prior year's filed 8949 content with **no promoted leg filing in that year at all** — flagged by
  BG-D9, absent from the disposal-leg enumeration.

Root cause is inherited: arch-r2's N-5 *equated* "advisory-flagged prior years" with the
`promote_export_gate(None)` enumeration — an architecture simplification with a tax consequence
(exactly the class this lens exists to catch; the r2 I-4 resolution's claim that the set "keeps the
BG-D9 1040-X packets" is false as pinned).

**Fix:** define the export set as {current year} ∪ {BG-D9 fold-diff flagged years across live
promotes, over disposal AND removal legs} — i.e. enumerate via the `promote_prior_year_advisory`
fold-pair machinery, recomputed from state at export time (which also preserves N-5's real point:
derived, never remembered). Keep `promote_export_gate` for its 8275-completeness purpose only. Add a
KAT: a donation-reordered prior year with no promoted disposal leg is in the export set.

### I-2 (DFW-D6) — The "pseudo-stable candidate signal" premise is false, and the pseudo-off force is scoped too narrowly

Verified: pseudo Phase B (`resolve.rs:1156-1181`) synthesizes `SelfTransferMine { basis: None }` for
every unresolved effective `TransferIn`, and that classification **folds a real lot** (`fold.rs`
~1156, the documented headline taint case) — its sats enter the pool, so a downstream
`dispose short by N` shortfall **can clear under pseudo** (likewise an accept-first `ImportConflict`
adopting a sat-bearing payload). DFW-D6's parenthetical "(pseudo-reconcile does NOT clear
`UncoveredDisposal`)" is factually wrong, and it is load-bearing: it is the SPEC's argument that the
discovery signal needs no pseudo handling. The composition currently survives only by accident of
the `!pseudo_active()` journey gate (TUI) and `would_conflict`'s pseudo-off (`project/mod.rs:119`,
verified) — but the chokepoints also serve the **CLI** drivers, where no journey gate exists, and
the SPEC's pseudo-off requirement is scoped to "the consent/savings computation" only. The DFW-D5
clearance re-projection and the DFW-D7 discovery recompute are left to inherit the stored session
config (`config.rs:38-45` carries `pseudo_reconcile` into every `to_projection()`).

**Fix:** (a) correct the claim; (b) require **every** chokepoint shadow projection — discovery
signal, DFW-D5 clearance check, consent/savings — to force `pseudo_reconcile = false`, exactly as
`would_conflict` does; (c) extend the DFW-D6 KAT beyond consent/savings to the clearance/discovery
computations.

Confirmed for the record (supports §8's filing): the sub-1 latent gap is REAL —
`cmd/promote.rs:396` takes `session.config()?.to_projection()` with the stored `pseudo_reconcile`,
and `consent_terms` (promote.rs:218-223), `promote_prior_year_advisory`, and
`gift_only_flagged_years` all project with it; no pseudo force-off exists anywhere in `promote.rs`.
With pseudo stored ON, today's CLI records an `Acknowledgment` whose figures fold synthetic
defaults. The SPEC's fix shape is right: forcing pseudo off makes pseudo-papered years surface as
Hard-blocked → `TaxYearNotComputable` → the BG-D6 three-flavor discipline records gain-Δ/
named-unquantified honestly — the correct consent artifact.

### I-3 (DFW-D4 / §2) — The triage census doesn't match the engine's real `UncoveredDisposal` emitter set; the audience's most common shape is unassigned

Verified emitter census (`fold.rs`): **sat shortfalls** = "dispose short" (:710), "pending out
short" (:831), "self transfer short" (:876), "gift out short" (:1196), "donate short" (:1274), and
"self-transfer/gift fee short" (:388, the FIFO fee draw); **without-wallet** = dispose (:691),
pending-out (:819), self-transfer (:864), **gift-out (:1177), donate (:1255)**, plus the degenerate
"fee carry has no surviving disposal leg" (:742). Against that:

1. DFW-D4.1's exclusion list names only "dispose/pending-out/self-transfer without wallet" — it
   misses **gift-out and donate without wallet** (and the degenerate fee-carry case). Spec-driven
   implementation would leave those two falling through toward cover-candidates they can never be
   (no sat quantity; DFW-D5's clearance check would refuse, but the triage — the decision DFW-D4
   exists to make — mis-routes first).
2. §2 scopes the feature to BTC "**sold or given away**", but **self-transfer-short** is arguably
   the MOST common audience shape: the Mt. Gox filer withdrew coins to self-custody, then sold —
   the sat shortfall lands on the *transfer*, not the sale. Covering it with a tranche is
   tax-correct (the relocated lot keeps the `EstimatedConservative` tag via the shipped relocation
   carve), but the SPEC never assigns the class, and DFW-D5's prefill is disposal-worded ("before
   the earliest short **disposal**", "the **disposal's** wallet") — for a transfer-short the anchor
   is the transfer date and the **source** wallet. Pending-out-short and fee-short are likewise
   unassigned.

**Fix:** enumerate the full emitter census in DFW-D4; assign each class cover vs fix (sat shortfalls
of all six kinds are coverable; all without-wallet variants are data-fixes); word DFW-D5's prefill
per-class (anchor event date / source-pool wallet); KATs pin a self-transfer-short candidate and
zero candidates for gift/donate-without-wallet.

### I-4 (DFW-D4/D5) — No redundant-tranche (over-coverage) state: declare-then-classify still re-mints the double-count through the temporal gap, and the phantom is silently promotable

DFW-D4.2 orders remedies only for acquisition-shaped blockers **open at declare time**, and its
"same pool/timeframe" linkage is a heuristic (a cross-wallet `UnmatchedOutflows` later reclassified
as a self-transfer INTO the shortfall wallet escapes it). Scenario: declare a tranche that clears a
shortfall (clearance check passes, honestly); a later import/classify supplies the real acquisition
→ the pool is over-covered and the tranche is now a **phantom $0 lot**. Nothing surfaces this:
DFW-D5.3 covers only the *didn't-cover* direction, and the shortfall row simply disappears, leaving
a healthy-looking tranche row. The phantom is then **promotable** — promote requires no live
shortfall (verified: the verb gates on target/coverage/attestation/consent only) — and a promoted
phantom's >$0 per-sat basis exits `hifo_cmp`'s `usd_basis==0` sort-last case (`pools.rs:275-287`)
and is drawn FIRST on the next real disposal: **understated gain filed on double-counted coins**,
behind a provenance attestation the filer gave for coins the vault meanwhile accounts for twice.
At $0 the phantom merely overstates future gain (conservative but wrong); promoted, it understates
— the direction BG-1 exists to prevent.

**Fix:** a first-class, derived "tranche redundant" state, the mirror of DFW-D5.3: re-project
**without** the tranche; if its targeted shortfall (or any shortfall it clears) no longer
materializes, surface "this tranche no longer covers anything real — void it" routing on the
dashboard, and add a promote-chokepoint advisory/refusal-grade check when the target tranche
currently covers no shortfall. Same shadow-projection machinery as DFW-D5.2; derived state; no new
tax logic. KAT: declare → clears; later classify supplies the coins; dashboard renders the
redundant state and the promote plan surfaces it.

### I-5 (DFW-D5 vs DFW-D8/§5) — The unconditional clearance check contradicts the behavior-preserving chokepoint claim; its CLI scope is undefined

Verified: the shipped declare verb gates on input validation + `guard_tranche_vs_allocation` only
(`cmd/tranche.rs:125-175`), and DFW-D8 says the wizard's declare "matches the shipped verb". But
DFW-D5 makes the **declare chokepoint** refuse a candidate that would not clear "the targeted
shortfall", while DFW-D2 makes both the CLI verb and the dashboard thin drivers over that ONE
chokepoint and §5's last bullet asserts "the chokepoint extraction is behavior-preserving". As
written this is a fork: either (a) the CLI freeform declare — no targeted shortfall; e.g. declaring
before the sale is imported, or covering a future disposal of no-record coins, both legitimate
shipped v1 flows — starts refusing (a behavior break that blocks a legitimate $0-conservative
filing path), or (b) the clearance check lives only in the dashboard driver — a second gating
authority, the exact I-2 violation DFW-D1 forbids.

**Fix:** parameterize the chokepoint `plan` with `Option<target shortfall>`: the clearance
refusal binds iff a target is designated (dashboard candidates always designate one; the CLI
freeform path passes `None` and keeps shipped semantics — at $0 a non-covering declare files
nothing wrong). State this scoping in DFW-D5 and pin both branches with KATs.

---

## Minor

- **M-1 (DFW-D9 vs DFW-D10) — live-readout contradiction.** D9 mandates "clamped saving as the
  filer edits"; D10 limits the live readout to floor/coverage/holding-date with tax-Δ "on demand,
  never per keystroke". State which binds (D10, per its own rationale) and require the on-demand
  saving to **invalidate on any window edit** (blank/"stale — recompute"), so a $ computed for a
  previous window is never displayed against the current floor.
- **M-2 (DFW-D10) — flavor condition compressed.** "Computed-tax-Δ where tables exist
  (2017/2024/2025/2026)" understates BG-D6's condition: tax-Δ requires **both folds to compute the
  year** — table (verified `tax_tables.rs:73-80`: exactly 2017/2024/2025/2026 ship) AND stored
  `TaxProfile` AND no Hard blocker. A literal reading re-admits the bare-$0 through the
  no-profile/blocked doors. Tighten the wording to "where both folds compute the year".
- **M-3 (DFW-D9/D5) — preset↔prefill precedence + attestation substance.** An era preset's
  `window_end` can conflict with DFW-D5's before-the-disposal prefill — state which governs (and
  note the term consequence: `window_end` IS the lot's holding-period start, verified
  `resolve.rs:1310` "effective date = window_end"; an early preset end makes the covering leg
  long-term even at $0 basis). Require the preset-confirm copy to say the window must reflect the
  filer's OWN knowledge of when they bought — the attested window is the substance of the BG-D5
  attestation and the Cohan/§6664(c) footing, and must never read as tool-sourced. (Both flows end
  behind the clearance check and the typed attestation, hence Minor.)
- **M-4 (DFW-D3/D8) — "revocable" copy must carry the BG-D9-iii carve.** A `DeclareTranche` with a
  live promote is not voidable (engine-adjudicated `DecisionConflict`); the dashboard's declare-row
  copy must not render an unconditional revocability claim.

## Nit

- **N-1 (DFW-D5/§2):** if the declare flow lets the filer edit `sat` above the prefilled
  `short_sat`, the excess is exactly the "manual I-hold-N-BTC" model §2 excludes, entering through a
  side door (v1-legitimate, files nothing wrong at $0; a confirm-note suffices).
- **N-2 (DFW-D3):** consider surfacing the shipped `method_inversion_advisory` /
  `tranche_dip_advisory` lines on the journey's tranche rows — under an elected FIFO the tranche's
  $0/floor basis lands on earlier disposals than the shortfall row implies (coverage is
  method-invariant, basis allocation is not; both advisories are shipped and state-derived).

---

## Verified sound (do not re-litigate next round)

- **The headline flow files the right number.** Declared tranche → `Eff` at
  `window_end.midnight()`, decisions sort after same-instant imports (`resolve.rs:1308-1316`,
  citation accurate); wallet routing incl. Path-A re-home to `lot.wallet`
  (`transition.rs:89-104`) matches DFW-D5's parenthetical; under HIFO the $0 lot sorts last
  (`pools.rs:276-281`) and absorbs exactly the shortfall residue; promoted, the pass-2 in-resolve
  rewrite changes `usd_cost` only (term-invariance comment verified) and the BG-D4 clamp keys the
  estimate share via the promote set threaded to `make_disposal_legs` — correct basis, correct
  window_end-derived holding period, no double-count (absent I-4's phantom).
- **DFW-D8 is tax-correct:** a $0 declare claims no estimate — no 8275 duty attaches, revocable,
  plain confirmation; matches the shipped verb's gates.
- **DFW-D12 is genuinely BG-D6-required, not just hygiene:** sequential promotes change each
  other's consent figures (each saving is measured against the fold that includes prior recorded
  promotes), so a bulk consent quote is ill-defined per-tranche — one-at-a-time is the only shape
  under which the recorded per-event `Acknowledgment` figures are well-defined.
- **DFW-D10's clamped-only rule** correctly forbids the unclamped `overpayment_delta` what-if as a
  promote quote (the sub-1 tax-r1 I-3 hazard); 2018–2023 are uncomputable-forever as stated;
  display caching is consent-safe because the recorded figures come from the chokepoint plan at
  promote time (DFW-D2 staleness clause), never the dashboard cache.
- **DFW-D6's fix shape files the right consent artifact** (pseudo-off + three-flavor absorption),
  and the sub-1 latent gap it files back is confirmed real in source (see I-2).
- **`promote.rs:451-458` citation** (consent printed before the ack gate) — accurate.
- **No-new-tax-logic claim holds** modulo the findings above: every filed number flows through
  shipped primitives; the new logic is gates/refusals/derived views (and the DFW-D6 chokepoint fix
  is honestly classified as a sub-1 defect repair, not wizard behavior).

*End r1. NOT GREEN: 0 Critical / 5 Important / 4 Minor / 2 Nit. The five Importants are all
seam-level and fixable in SPEC text; none undermines the object choice or the chokepoint
architecture.*
