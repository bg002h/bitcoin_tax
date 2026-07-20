# Conservative-filing SPEC v2 — architecture lens review r2 — NOT GREEN (2 Important)

_Fable, independent, general-engineering/architecture lens, round 2. All r1 architecture findings resolved;
2 NEW Important gaps the `DeclareTranche` rewrite opened, + minors/nits._

VERDICT: NOT GREEN

Verified positives: the v2 rewrite is real — every r1 architecture finding addressed on its own terms, the new citations check out (`transition.rs:82-85` overwrite loop; `fold.rs:568` acquire-wallet requirement; `pools.rs:272-287` hifo_cmp with `usd_basis == 0` LAST; `pools.rs:255-260` FIFO acquisition-date-ASC confirming D-9's inversion premise; `optimize.rs:~455-486` is_broker/persistability/ForbiddenBroker2027; `render.rs:44` basis_source_tag; `tui-edit form.rs:1756` cycle_basis_source with off-ring `SelfTransferInbound` precedent; `forms.rs:237` how_acquired_from is the 8283 donor field, consumed at `forms.rs:403`), and legs genuinely carry `basis_source` to where P3/P7/D-6 need it (`pools.rs:291-307` Consumed, propagated `fold.rs:210/263`). FOLLOWUPS shipped-box entry confirms D-6's scheme and declares the dependency both ways. What keeps this NOT GREEN: two Important gaps the rewrite opened.

r1-C1: RESOLVED — D-8 Path-A exemption is a contained conditional at the verified overwrite; Path-B refusal prevents `SafeHarborUnconservable`; KAT buildable. (But New-2/New-3.)
r1-I2/I3: RESOLVED — D-6 no longer claims Box E/`box_needs_review` reuse (honest "inherit the corrected logic"); VARIOUS dropped for the window-end date; box-fix dependency declared in header + D-6 + §4 + FOLLOWUPS.
r1-I4/I5: RESOLVED at schema/identity level — `DeclareTranche` homes the window; `EventId::Decision { seq }` a clean identity. The FOLD half under-specified — New-1.
r1-I6: RESOLVED — D-9 advisory buildable, FIFO-inversion KAT pins the dependence, declining to auto-elect is the right v1 call (auto-election would silently answer a tax question — answered-ness).
r1-I7: RESOLVED — no floor field; $0-only uniform; contradiction gone.
r1-min-8/9/10/11: RESOLVED; r1-min-12: PARTIAL (year-scoping specified; surface unnamed — New-4); r1-nit-13: RESOLVED.

## New findings

**1. [Important] D-1/P1/D-8 — "folds like an acquire" over machinery that structurally excludes decision events; the load-bearing dating mechanism is unspecified.** Decision-id events have NO fold-timeline entry today: the pass-2 timeline builder hard-filters to `EventId::Import` (`resolve.rs:1054-1059`, "Non-import events (decisions, conflicts) are skipped — they have no timeline entry"), so `DeclareTranche` would be the first decision payload folded as a primary movement. That requires SPEC-owned design: (a) the tranche's effective fold date must be `window_end` — D-8's "pre-2025 tranches fold into Universal" depends on it (`pool_key` keys on the Eff date, `pools.rs:15-21`; the pass-1 conservation snapshot filters the same timeline by date, `transition.rs:44-48`) — but `LedgerEvent` documents decision timestamps as creation time (`event.rs:348-349`), so either back-date (breaks a convention with events-list/audit implications) or build the Eff with `utc = window_end` diverging from the event timestamp; (b) canonical ordering needs surrogates — `sort_canonical` keys on `(utc, src_priority, src_ref)` (`resolve.rs:1376-1383`) which a Decision id lacks, so determinism (NFR4) for two same-date tranches needs a tie-break; (c) `build_op`'s `_ => Op::Skip` (`resolve.rs:393`) means the arm is NOT compile-forced — an omission silently vanishes the tranche, deserving a named KAT. Fix: amend D-1 to specify the effective-date mechanism (recommend: event keeps creation time; the Eff/timeline entry is dated `window_end` + an explicit ordering surrogate) and note the timeline-builder change as in-scope for P1.

**2. [Important] D-8 fixed one tag-erasing overwrite; a second, unhandled one sits on the exact path P8 recommends.** The self-transfer relocation arm hard-sets `basis_source: CarriedFromTransfer` on every relocated fragment (`fold.rs:812`; sweep confirms transition.rs:83 + fold.rs:812 are the only two overwrite sites). A tranche moved Exchange → SelfCustody — precisely what P8 advises, and `SelfTransfer` is LotSelection-selectable (resolve.rs:1119-1120) — sheds `EstimatedConservative`; the disposal leg then misses P3's dip advisory, P7's MANDATORY disclosure (its own test calls a filed-tranche year without it "a hard gap"), and the D-8 KAT guarantee. Not Critical: `usd_basis` stays $0 and `acquired_at` carries (`fold.rs:808,811`), so tax/term/HIFO are unharmed — only the tag-keyed advisory/disclosure layer degrades. Fix: a D-8-style exemption at the relocation site + a relocated-tranche KAT (precedent: pseudo taint propagates on the same struct, `fold.rs:818`).

**3. [Minor] D-8 specifies the refusal only one direction.** The reverse — a `SafeHarborAllocation` declared into a vault already holding a pre-2025 tranche — is unspecified: normally it hard-blocks (allocation inert → Path A → tag survives via the exemption), but an allocation deliberately conserving over the tranche's $0-basis sats becomes effective and Path B DISCARDS the tranche lot with no trace (`transition.rs:77,88-94`), contradicting the D-8 KAT. One sentence (loud supersede vs block) + scope the KAT to Path-A/no-allocation vaults.

**4. [Minor] P6's surface unnamed (residue of r1-min-12); P7's "first-class export artifact" names no format/location.** One line each.

**5. [Nit] Exhaustive-`BasisSource` sweep lists 3 of 4:** `form.rs:1771` (`basis_source_display`) is a fourth exhaustive match. Compile-forced anyway; add for completeness.

**6. [Nit] `DeclareTranche`'s void/duplicate semantics unstated.** Defaults are sane (`Some(_)` catch-all makes it revocable, `resolve.rs:464-466`; two tranches additive). Add "voidable via `VoidDecisionEvent`" per the recent-decision-variant doc precedent.

Scope/YAGNI: v2 remains a coherent minimal $0-only v1. GREEN is two contained amendments away: specify the decision-fold dating/ordering machinery (New-1) and extend the D-8 exemption to the relocation overwrite (New-2).
