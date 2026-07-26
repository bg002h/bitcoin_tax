# Architecture review — Defensive Filing SPEC (r3, Opus lens)

**Artifact reviewed:** `design/defensive-filing-wizard/SPEC.md` (binding decisions DFW-D1..D12),
commit `cc44402` on `feat/defensive-filing-wizard`.
**Stance:** independent SPEC-level architecture re-review after the r2 fold. My job: (a) verify my r2
**Critical (C-1)** and Minors (m-1, m-2) are genuinely resolved by the displacement-predicate rewrite,
and (b) find any NEW defect the fold introduced. Every load-bearing claim below is re-derived from
current source, not carried forward from r2 and not anchored on the SPEC's self-citations.

---

## Verdict

**NOT GREEN** — **1 Critical / 0 Important / 0 Minor / 0 Nit**.

m-1 and m-2 are genuinely resolved (audit below). But **C-1 is NOT resolved**: the r2 fold *re-named*
the over-constrained refusal ("displacement of documented basis" instead of "covers no shortfall")
without actually narrowing it to the true hazard — because the true hazard (a double-count) is **not
ledger-distinguishable** from a legitimate, shipped, attested operation (a mixed-vintage HIFO reorder).
The displacement predicate, as written, **refuses `mixed_vintage_hifo_2018_disposal`** — a promote the
shipped CLI verb allows today and that the entire prior-year-advisory / DFW-D11 export subsystem is
built around. It therefore still (i) removes a shipped capability at the extraction seam, (ii)
**contradicts DFW-D11**, and (iii) rests on a false premise ("promoting a displacing tranche is never
legitimate"). Same class as r2 C-1, re-described, not fixed.

---

## C-1 / m-1 / m-2 resolution notes

- **C-1 — NOT RESOLVED.** See Critical below. The predicate spares the *degenerate* undisposed case
  (nothing draws the tranche) but still refuses the *reorder* case (a real disposal draws the floor),
  which is exactly the legitimate, shipped operation the advisory subsystem exists to warn about, not
  forbid.
- **m-1 (per-event `short_sat`) — RESOLVED.** Verified the multi-per-event reality in source: a single
  `Dispose` event raises a principal short (`fold.rs:709`, `"dispose short by N sat"`) AND, via
  `consume_fee` on the same `eff.id` (`fold.rs:724` → `:388`, `"self-transfer/gift fee short by N sat"`),
  a second `UncoveredDisposal` — two blockers on one `EventId`, distinguishable only by `detail`. DFW-D7
  now pins `short_sat` as the **per-event aggregate** and the DFW-D5.2 clearance target as the `EventId`
  tested **event-level** ("no `UncoveredDisposal` remains on the target event"); DFW-D8's
  "excess above `short_sat`" is the event aggregate. Consistent across DFW-D4 (keys on `short_sat`
  presence) / D5 (EventId target) / D7 / D8. Closed.
- **m-2 (§5 behavior-preserving carve) — RESOLVED for DFW-D6.** §5 now reads "behavior-preserving
  **EXCEPT** the DFW-D6 chokepoint pseudo-off correction … the ONLY intended behavior change." The
  DFW-D6 half is accurate and honestly flagged (verified the latent gap: `promote.rs:396` threads the
  stored `cfg.pseudo_reconcile` into `consent_terms`/advisory/`gift_only`). **But** the same sentence
  appends "C-1's over-coverage refusal must NOT change any shipped promote KAT" — that clause is *false*
  and is a symptom of the unresolved Critical (the displacement refusal is a second, un-acknowledged
  behavior change to the shipped verb). Folded into C-1, not raised separately.

---

## r2-resolution re-verification (still holding, source-checked)

- `hifo_cmp` sorts `usd_basis == 0` **last** (`pools.rs:276-281`), so a promoted `>$0` floor exits the
  sort-last case and reorders ahead of documented lots — the mechanism DFW-D5.3 invokes is real.
- `would_conflict` forces `pseudo_reconcile = false` (`project/mod.rs:119`) — the shadow-projection
  precedent DFW-D6 mirrors is real.
- `promote_prior_year_advisory` folds **promote-present vs promote-excluded** and diffs per-year
  **disposal ∪ removal** leg sets (`conservative.rs:701-758`) — DFW-D11's machinery is real.
- The shipped `promote_tranche` verb has **no shortfall / displacement guard** (`promote.rs:364-488`,
  grep-confirmed); the `ConsentTerm::Unrealized` forward-promote path (`promote.rs:310-326`) and its KAT
  `fully_undisposed_promote_records_an_unrealized_term_not_empty` (`kat_promote.rs:2117`) are the shipped
  BG-D6 guarantee.

---

## Critical

### C-1 (DFW-D5.3) — The displacement predicate is over-broad: it refuses a legitimate mixed-vintage HIFO-reorder promote, which the shipped verb allows and DFW-D11 depends on; it is un-narrowable on a shared hard-refusal chokepoint

DFW-D5.3 now refuses a promote iff "some real disposal/removal in its pool draws the tranche's promoted
`>$0` floor **in place of the documented (non-`EstimatedConservative`) basis it would otherwise draw**,"
and asserts this "fires ONLY on a genuine filing hazard … promoting a displacing tranche is never
legitimate on either [surface]." **That premise is false.** The trigger condition is satisfied by a
routine, shipped, legitimate operation.

**The counterexample is a shipped fixture.** `mixed_vintage_hifo_2018_disposal`
(`kat_promote.rs:1527`) — mirrored on the CLI as `build_promoted_vault` (`promote_cli.rs:54-59`,
comment: *"the amend-to-PAY reorder the advisory warns about"*):

- documented 2017 buy, 60M sat, basis $3,000 (≈ $0.00005/sat);
- a 40M-sat tranche, window 2018-01..03, promoted to a $12,000 floor (≈ $0.0003/sat — higher per-sat);
- a 2018-09-01 sell of exactly 40M sat, proceeds $20,000.

WITHOUT the promote (tranche $0, sorts last) the sell draws 40M of the documented lot → gain $18,000.
WITH the promote the floor sorts ahead → the sell draws the **tranche floor in place of** the documented
basis → gain $8,000; the documented 60M is deferred to a later disposal. **The displacement predicate
fires and refuses this promote.** Yet it is legitimate: if the filer's BG-D5 attestation is true they
genuinely hold 100M sat (60M documented 2017 + 40M attested 2018); drawing the higher-basis tranche
first is ordinary HIFO; total gain across time is unchanged (a *timing* shift, not an understatement);
the 1040-X advisory (`undisposed_promote_that_hifo_reorders_a_prior_year_fires_the_advisory`,
`kat_promote.rs:1680`) exists precisely to **warn about — not forbid —** exactly this reorder. The
shipped CLI verb, having no shortfall guard, records it today.

**This is not narrowable, because the hazard and the legitimate case are ledger-identical.** Let `T` =
tranche live sat, `S` = shortfall re-materialised by removing the tranche entirely:

| case | T | S | required verdict |
|---|---|---|---|
| mixed-vintage reorder (`mixed_vintage`) | 40M | 0 | **allow** |
| full phantom / redundant tranche | 40M | 0 | **refuse** |
| correctly-sized cover | 40M | 40M | allow |
| tax-I-A partial over-coverage | 100M | 40M | refuse (over by 60M) |

The first two rows have **identical `(T,S)` signatures and opposite required verdicts.** Both are "a
40M tranche whose floor a real disposal HIFO-draws ahead of a documented lot." The *only* distinguishing
fact — whether the tranche's coins ARE the documented coins (double-count) or DIFFERENT coins (genuine
extra holdings) — is knowable only from the filer's attestation, never from the ledger, and DFW-D3
forbids persisting a per-tranche target that could link a tranche to the shortfall it was declared
against. So **no ledger-structural predicate on the shared chokepoint can both spare `mixed_vintage`
and catch tax-I-A.** ("Refuse iff removing the tranche leaves no new shortfall" spares tax-I-A's partial
case, re-opening tax-I-A; "refuse iff the floor displaces documented" refuses `mixed_vintage`,
re-opening my C-1.) The r2 fold picked the second and thereby reproduced C-1.

**Consequences (all Critical-grade):**
- **Weakened/removed shipped capability.** The CLI `promote-tranche` verb can record a
  documented+tranche reorder promote today (`build_promoted_vault` is a recordable state; the whole
  void-direction + advisory subsystem is built on it). DFW-D5.3 on the shared chokepoint removes that.
  §5's "behavior-preserving except DFW-D6" is therefore false — there is a **second**, un-acknowledged
  behavior change. (This is the false m-2 clause.)
- **DFW-D5.3 contradicts DFW-D11.** DFW-D11 exports 1040-X packets for prior years a **live promote's
  HIFO reorder** changed (documented disposal/donation/gift legs) — it presupposes such promotes are
  *recorded*. DFW-D5.3 refuses them. If enforced, DFW-D11's reorder-export path and the
  `promote_prior_year_advisory` warning subsystem become largely unreachable.
- **Not well-defined for a plan-writer.** The SPEC claims the predicate "fires ONLY on a genuine filing
  hazard"; it does not. A plan-writer implementing the text literally breaks `mixed_vintage`; one
  implementing the stated *intent* ("double-counted sat") needs a same-coins signal the ledger cannot
  supply. The exact C-1/I-2 ambiguity the fold was meant to close is reopened.

**Fix (this is the r2 option (b) I flagged, now shown to be the only sound one):** **demote the
promote-side over-coverage/displacement check to a derived DASHBOARD ADVISORY** — "this pool now holds
documented coins overlapping this tranche's window; if the tranche is redundant, void + re-declare at
the covered size" — and **leave the shared promote chokepoint's gate set unchanged** (behavior-
preserving, matching §5 and DFW-D11). The filer, the only party who knows whether the coins are the
same, decides; the engine does not hard-refuse an attested, ledger-legitimate promote. Keep the derived
"over-covered by N sat" state (DFW-D5.3's shadow-projection derivation is fine **as a surfaced
advisory**), drop the chokepoint refusal, and add a KAT that a `mixed_vintage`-shaped reorder promote
**still records** (mutation: keep the displacement refusal on the chokepoint → the reorder promote is
wrongly refused → reds). If a hard refusal is truly wanted for the phantom, it requires a persisted
tranche→target linkage — a DFW-D3 reversal that is out of scope and must be a separate binding decision,
not smuggled in via an over-broad predicate.

---

## Lens answers (condensed)

**L1 (DFW-D2):** unchanged from r2 — contract complete/implementable; ack-inside-`apply` fail-closed;
export trio degenerate; full-driver parity. Sound.
**L2 (DFW-D4/D7):** m-1 resolved — `short_sat` per-event aggregate + event-level clearance is consistent
across D4/D5/D7/D8 and matches the multi-per-event fold reality (`fold.rs:709` + `:388` on one `eff.id`).
**L3 (DFW-D5):** DFW-D5.1/5.2 (prefill, target-parameterised clearance, `None`/`Some` declare carve) and
the didn't-cover pool-state (I-4) remain sound. **DFW-D5.3's displacement refusal is NOT sound — C-1**
(over-broad; refuses `mixed_vintage`; un-narrowable on a shared hard-refusal chokepoint).
**L4 (DFW-D11):** two-set split still clean in itself — but DFW-D11 now **contradicts DFW-D5.3** (D11
exports reorder-promotes D5.3 would refuse). Resolved by demoting D5.3 to an advisory.
**L5/L6 (consistency / new):** the one contradiction is DFW-D5.3 vs DFW-D2-§5 (behavior-preservation) /
DFW-D11 / the shipped advisory subsystem — the same shape as r2 C-1, re-described. Every other shipped
BG-D1..D11 enforcement point stays put. m-1/m-2(DFW-D6 half) genuinely resolved.

---

*End r3 (SPEC, Opus). Verdict: **NOT GREEN — 1C/0I/0m/0n**. m-1 and m-2's DFW-D6 half are resolved; the
single Critical is unchanged in class from r2: DFW-D5.3's promote-chokepoint refusal is over-broad
(re-named, not narrowed), refuses the shipped `mixed_vintage` reorder promote, and contradicts DFW-D11.
Resolvable without an architecture reshape by demoting it to a derived dashboard advisory and leaving
the shared promote gate behavior-preserving. Re-review required after fold.*
