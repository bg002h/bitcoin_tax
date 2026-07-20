# Conservative-filing SPEC v3 — architecture lens review r3 — NOT GREEN (1 Important)

_Fable, independent, architecture lens, round 3, at f9a4257. Both r2 Importants RESOLVED + verified; 1 NEW
Important (a third ordering the D-8 fix didn't cover) + 2 Minor + 2 Nit._

VERDICT: NOT GREEN

**r2-New-1: RESOLVED** — D-1a is buildable and sufficient. (a) Eff-dated-at-`window_end` is threadable: `Eff.utc`'s only consumers are `Eff::date()` (`resolve.rs:118`) and `sort_canonical` (`resolve.rs:1378`); nothing assumes `Eff.utc == event.utc_timestamp`; event creation-time meaning (`event.rs:348`) untouched. ONE admit-and-date covers BOTH passes: the pass-1 conservation snapshot consumes the same `timeline` (`universal_snapshot(&timeline,…)`, `resolve.rs:1227`) filtering by `e.date() < TRANSITION_DATE` (`transition.rs:44`), and `pool_key` keys on the fold date (`pools.rs:15`) — window_end-dated Eff lands in `PoolKey::Universal` in both passes, no second mechanism. (b) `(window_end, decision_seq)` sound + available (`decisions: Vec<(u64,&LedgerEvent)>` at `resolve.rs:484`, before the timeline build; seq also on `Eff.id`). (c) `_ => Op::Skip` at `resolve.rs:393`; required-arm KAT is the right pin. (d) `Some(_)` catch-all at `resolve.rs:464`. No double-count: every other decision pass is payload-specific (Void 442, supersede/reject 1b, MethodElection 1095, LotSelection 1139, SafeHarborAllocation 1205); DeclareTranche matches none; `honoring_principal` returns None for it (`resolve.rs:1365`).

**r2-New-2: RESOLVED** — `fold.rs:812` is the relocation overwrite (`Op::SelfTransfer` arm, `fold.rs:775-820`, hard-sets `CarriedFromTransfer`); exemption contained + expressible (pseudo-taint precedent `fold.rs:818`; usd_basis/acquired_at carry `fold.rs:808,811`). Sweep re-run: production tag-REPLACE sites are exactly `transition.rs:83` + `fold.rs:812`; `pools.rs:220` copies; `fold.rs:589/721/910/1059`, `resolve.rs:965/1270` construct fresh; `optimize.rs:1538/1569`, `return_1040.rs:2519/2927`, `pools.rs:333` are `#[cfg(test)]`. No third site.

**r2-New-3: RESOLVED as written** — the reverse record-time refusal is aimed correctly at the `transition.rs:77,88-94` discard — but the fold relocated the hand-wave into the inert-allocation window; see New-1.

**r2-New-4/5/6: RESOLVED** — P6 surface named + real (`report --tax-year` `cli.rs:48`, + TUI Tax tab); P7 export named (`basis_methodology.txt`); all 4 `BasisSource` match sites incl. `form.rs:1771`; D-1a(d) voidability + additive duplicates.

## New findings

**1. [Important] D-8/P1 — the coexistence refusal is scoped to an EFFECTIVE allocation, so the allocation-first-while-INERT order reaches the exact silent discard New-3 was written to block.** D-8's forward bullet and P1's test pin "refuses under an EFFECTIVE Path-B allocation." But effectiveness is recomputed per-projection from the timeline (`resolve.rs:1227-1245`) and the tranche is IN that timeline. Sequence: record an allocation whose totals don't match the residue → `SafeHarborUnconservable` → inert, Path A (loud). Then declare a pre-2025 tranche — *permitted*, no allocation effective. The tranche's own sats complete the allocation's sat total while leaving basis unchanged ($0), so the next projection flips the allocation EFFECTIVE: the unconservable blocker vanishes (`resolve.rs:1237-1245` now conserves), Path B discards the Universal remainder — tranche included — with no trace (`transition.rs:77,88-94`), and the allocation's listed basis re-spreads over sats that include the unprovable coins → a `>$0` filed basis on undocumented coins, no `EstimatedConservative` leg survives, P7's MANDATORY disclosure never fires. Silent, and it contradicts D-8's own KAT ("both directions … refused"). Fix (one line): scope the refusal to ANY in-force (non-voided) `SafeHarborAllocation`, effective OR inert (align the P1 test), OR add the invariant-grade projection-time guard — Path-B effectiveness refuses (loud → inert → Path A, tag survives) whenever the pre-2025 residue contains an `EstimatedConservative` lot.

**2. [Minor] Voided-tranche fold has no pin.** D-1a(d) declares voidability, but the timeline admit is a NEW consumer of `voided` outside the established `if voided.contains(&d.id) { continue }` loops (`resolve.rs:1092,1136,1202`) — omit the check and a voided tranche keeps filing; no listed KAT catches it (D-1a-c pins the silent-vanish; the silent-survive is unpinned). Add "a voided DeclareTranche folds nothing" to P1/§6.

**3. [Minor] Stale artifact header** — title/Status still read "v2 DRAFT … 3 Critical + 8 Important" (`SPEC.md:1-4`) while the body folds r2. Bump to v3 and correct the fold-provenance claim.

**4. [Nit] `(window_end, decision_seq)` encoding.** If routed through `sort_canonical`'s `src_ref` (string-ish), lexicographic compare misorders seq 2 vs 10. Compare numerically (seq is on `Eff.id`).

**5. [Nit] Σ-conservation.** The new fold arm must bump `stats.sigma_in` like `Op::Acquire` (`fold.rs:596`, FR9) or conservation KATs go red; one word in P1.

Scope: the v3 fold is real work — D-1a is a genuinely sufficient decision-fold spec (the hard r2 finding), and the two-site D-8 exemption is verified complete. GREEN is one contained amendment away: close the inert-allocation ordering (New-1) plus the two Minors.
