# Architecture review r2 — `design/conservative-filing-approach-b/SPEC.md` (Approach B sub-project 1)

**Reviewer:** independent Fable architecture lens, round 2 (adversarial; gate = 0 Critical / 0 Important).
**Artifact:** SPEC.md as revised by the r1 fold (`a9fad77`). Both r1 reviews + `DESIGN_PROVENANCE.md` read.
**Method:** every r1 finding checked for *actual* resolution against the folded text AND current source on
`feat/conservative-filing-b`; every code symbol the fold newly cites was opened and read; an independent
full sweep of every `EventPayload` consumer in product code was performed to adversarially test the §3
payload census's completeness claim.

**Verdict: 0 Critical / 1 Important / 1 Minor / 2 Nit — NOT green (1 Important).**
Five of the six r1 Importants are genuinely resolved with correct code facts; the fold of I-4 traded the
mis-keyed predicate for a predicate that is *structurally unable to fire* in reachable configurations
(`tax_total` is `None` for any year without a bundled table — and the shipped `BundledTaxTables` covers
ONLY 2017/2024/2025/2026 — or without a profile, or whenever ANY unrelated Hard blocker exists), which
also silently zeroes BG-D6's consent Σ terms. One fix stroke closes both. Nothing in the fold undermined
BG-D1.

---

## § Verified resolved (r1 finding → status, with the code fact)

### arch r1 I-1 (clamp goes negative) — RESOLVED
BG-D4 now states `clamp(net_proceeds_share, $0, estimate_share) = min(estimate_share, max(net_share, $0))`
— algebraically correct (estimate basis ∈ [0, estimate_share]; a negative `net` yields estimate basis $0
with the negative gain attributed to documented fee/rounding). Both enumerated corners (`fee_usd >
proceeds` netting in `fold.rs`; cent-scale negative remainder) are named and pinned by a §6 KAT. The
amended invariant wording is sound.

### arch r1 I-2 (decomposition falsified by the TP8(c) fee carry) — RESOLVED
BG-D4 now derives the estimate share from the STORED promote (`filed_basis × leg.sat / tranche_sat`,
keyed via `leg.lot_id.origin_event_id` → the threaded promote set), documented = the remainder,
unclamped. Verified deeper than r1 did: (a) `FeeCarry::rehome_onto_lot` (`fold.rs`) does merge documented
fee into `lot.usd_basis` — the falsifier is real; (b) **the relocation arm preserves `origin_event_id`**
(`Op::SelfTransfer` in `fold.rs`: the relocated `Lot` is built with
`origin_event_id: c.lot_id.origin_event_id.clone()` + a bumped `split_sequence`) — so the keying survives
the exact Exchange→SelfCustody scenario the decomposition exists for; (c) fee carries add basis but never
sats, so a tranche-origin leg's sats are all tranche sats and the pro-ration is exact; (d) `DisposalLeg`
AND `RemovalLeg` both carry `lot_id` + `basis_source` (`state.rs`), so the same decomposition is available
at the BG-D11 donation/gift sites. Documented share ≥ 0 holds (nothing debits a tranche lot's `usd_basis`
below the floor — the §1015 pool-side debit path requires `dual_loss_basis`, which `rehome_onto_lot`
deliberately never promotes on a non-dual lot). The §6 relocated-with-fee-then-promoted KAT pins it.

### arch r1 I-3 (engine-adjudicated void / dangling promote) — RESOLVED (one Minor residue, M-1 below)
BG-D9 now specifies: resolver-inert + `DecisionConflict` for void-of-tranche-with-live-promote (the
void-of-effective-allocation mirror — verified at `resolve.rs` step-3 item (5): effective target →
conflict, inert target → void applies); hard `DecisionConflict` for absent/voided/wrong-type promote
targets (the pass-1d/1e pattern — verified: "target absent or wrong type → Hard DecisionConflict,
EXCLUDED" exists as described); `PromoteTranche` added to `is_revocable_payload` and the
`DeclareTranche`-with-live-promote exclusion in `voidable_decisions` (verified: `DeclareTranche` is
unconditionally in the `matches!` today, `void.rs`; the `effective_alloc` closure is the exact exclusion
shape to mirror, and the predicate's `events + blockers` inputs suffice). `would_conflict`
(`project/mod.rs`) verified as the right record-time surface — it runs the REAL projection twice and
diffs the `DecisionConflict` set, so any resolver-adjudicated conflict surfaces at record time with zero
new code. The `safe_harbor_residue` dangling shape (`session.rs`: the filter drops
`SafeHarborAllocation | DeclareTranche` but would keep a `PromoteTranche`) is real and §3 item 10 covers
it. "Never a dangling target" now holds by construction as claimed.

### arch r1 I-4 (advisory trigger mis-keyed) — PARTIALLY RESOLVED → new I-1 below
The re-key from "already-disposed" to the any-year-`tax_total`-diff does fix the r1 defect (an
undisposed-tranche promote that HIFO-reorders a prior year now diffs non-equal *when both folds are
computable*), and the void-direction symmetry + §6511 + conditional copy are all folded. But the chosen
predicate inherits `compute_tax_year`'s computability preconditions, which the underlying hazard does not
have — see new I-1.

### arch r1 I-5 (Part II narrative home + a real gate) — RESOLVED
The narrative lives ON the event (`part_ii_narrative: String`, BG-D1), captured at promote time, empty/
scaffold-only REFUSED at record time, edited via void + re-promote — option (a) of the r1 fix, cleanly
taken. BG-D8 now names the pseudo-export-block pattern, and that pattern is verified as a REAL refusal:
the fold pushes the `PseudoReconcileActive` blocker (`fold.rs`), `state.pseudo_active()` is enforced
refuse-before-any-bytes at the CLI export surface (`cmd/admin.rs::export_snapshot` →
`require_attestation` checked FIRST) and at the TUI export modal — a projected-state predicate checked
per surface, exactly the shape BG-D8 needs. The rejected `basis_methodology.txt` "gate" is confirmed
unconditional (`render.rs::write_basis_methodology_txt` called unguarded from both export paths). The
1a/1b split stays coherent under ONE ship gate (the 1a content artifact is present-by-construction; the
gate's real bite is the 1b PDF + the surfaces that could omit it — acceptable as a backstop guarantee,
same posture as D-8).

### arch r1 I-6 (payload-side census) — RESOLVED, and independently re-verified complete
§3 items 9–15 all check out against source: `is_revocable_payload` (`void.rs`), `safe_harbor_residue`
(`session.rs`), `build_op`'s `_ => Op::Skip` + the `Some(_)` void classification (`resolve.rs`), the
fingerprint `_ => return None` (`persistence.rs`), the bulk-void summary `other => {other:?}`
(`cli/main.rs`) and the void-flow summary `_ => ("?", …)` (`tui-edit/main.rs`), `would_conflict` +
pass-1d/1e, `Coverage` serde (verified: derives `Debug, Clone, Copy, PartialEq, Eq` only). My independent
sweep of EVERY `EventPayload` consumer in product code found **no additional site a promote reaches
wrongly**: `bulk_resolve_payload_summary` (`cli/main.rs`) and `import_payload_summary`
(`tui-edit/main.rs`) are imported-payload-scoped (a promote is unreachable — a Decision id never carries
an `ImportConflict` payload); `classify_raw_variant_label` is ClassifyRaw-scoped;
`cmd/inspect.rs::events_list` renders only imported rows (a `DeclareTranche` doesn't appear today, so a
promote consistently doesn't either); every other match is a variant-specific `filter_map` whose
catch-all is correct for a promote (`session.rs`, `compliance.rs`, `cmd/reconcile.rs`, `render.rs`);
`edit/persist.rs` / `edit/form.rs` payload matches are test code. The "no compile-forced `EventPayload`
match in product code" claim is TRUE (only serde derives compile-force).

### arch r1 M-1 (stale "$0" copy) — RESOLVED
All five §3-item-5 strings verified in source: `TRANCHE_IS_FINAL_HINT` + the "($0 EstimatedConservative)
is on file" refusal + the phantom-wallet "still files at $0" (`cmd/tranche.rs`), the
`SafeHarborUnconservable` blocker detail (`resolve.rs`), the TUI opener refusal inside
`safe_harbor_residue` (`session.rs`).

### arch r1 M-2 (floor units) — RESOLVED
`filed_basis` pinned as WHOLE-TRANCHE `round_cents(window_min_close_price × sat / SATS_PER_BTC)` on the
payload + a §6 units KAT; matches the source formula in `overpayment_delta_one` exactly (`reference` is
USD/BTC, scaled to whole-lot `usd_cost`).

### arch r1 M-3 (rewrite timing) — RESOLVED
BG-D1 now locates the rewrite INSIDE the resolve timeline build and explicitly demotes the
`overpayment_delta_one` analogy to "right swap, wrong timing" (verified: it mutates `res.timeline` after
`resolve()` returns). Verified the load-bearing ordering: resolve step 3 computes
`universal_snapshot(&timeline, …)` from the timeline that `build_op`/the `DeclareTranche` admit site
built, so an admit-site rewrite IS visible to §7.4 effectiveness. The snapshot-timing KAT pins it.

### arch r1 M-4 (payload typing / non-interactive) — RESOLVED
All payload fields typed; `Acknowledgment` is a struct (phrase + shown-figures snapshot + attested
provenance text/version); `timely_allocation_attested: bool` precedent cited (verified, `event.rs`); the
non-interactive path bounded to refuse-or-`--i-acknowledge <phrase>` (the `require_attestation`/
`ATTEST_PHRASE` precedent exists). See N-1 for the residual either/or.

### arch r1 N-1/N-2/N-3 — all RESOLVED
Coverage serde in §3 item 15; the forward-only vault-compat note + no-fingerprint KAT in BG-D1's tail
(precedent doc comments verified in `event.rs`); the two no-change forms sites stated in §3 item 8
(verified: 8949 col (e) reads `leg.basis`, `how_acquired_from` → `Review` for `EstimatedConservative`,
`forms.rs`).

### tax r1 C-1 → BG-D11 — architecturally BUILDABLE; §3/§6 hooks real
Verified: the §170(e)(1)(A) site computes `min(FMV, leg.basis)` over the FINAL legs (after
`rehome_onto_removal_leg`) in the `Op::Donate` arm (`fold.rs`); `RemovalLeg` carries `lot_id` +
`basis_source`, so the documented-component decomposition is the same promote-set keying as BG-D4 — one
mechanism, two consumers; the gift carveout (`rehome_onto_removal_leg`) is the cited lesser leak;
`forms.rs`'s "$0" §170(e) doc sentence (§3 item 8) exists as described; the §6 KAT (ST-donated promoted
tranche files documented-only) is pinned. Reported-vs-consumed divergence on removals has the §1015
precedent. No architectural obstacle.

### BG-D1 (the core ruling) — NOT undermined by the fold
The fold added payload fields, the in-resolve rewrite location, and the vault-compat note; the
no-new-identity / no-new-`BasisSource` ruling and its by-construction guarantee set (D-8 backstop keyed
on `estimated_conservative_remaining_sat`; tag-keyed Path-A seed exemption; tag-preserving relocation
carve; tag-keyed record-time refusals; `hifo_cmp` exit from the `usd_basis == ZERO` special-case) all
re-verified intact against current source. Phase 1a/1b boundary unbroken; `packet.rs`'s exhaustive
destructure confirmed as the 1b compile-forced hook.

---

## New findings

### I-1 — BG-D9/BG-D6: the any-year-`tax_total`-diff trigger (and the consent Σ) silently degrade to "no change"/"$0" for every year `compute_tax_year` refuses — which, against the shipped table set, is EVERY year 2018–2023, plus the no-profile and any-unrelated-Hard-blocker doors
- **Defect (one sentence):** the folded trigger compares per-year `tax_total` pre/post promote, but
  `tax_total` returns `None` whenever `compute_tax_year` is `NotComputable` — and `None == None` reads as
  "no change" — so the advisory that BG-D9 calls its structural G-3 guarantee cannot fire for any year
  lacking a bundled tax table (`BundledTaxTables::load()` inserts ONLY 2017, 2024, 2025, 2026 —
  `tax_tables.rs`), lacking a stored profile, or coexisting with ANY unrelated Hard blocker anywhere in
  the projection (`compute_tax_year` gate (1), `tax/compute.rs`) — while the year's legs/8949 rows are
  rewritten regardless, because the fold needs none of those preconditions.
- **Concrete failure scenarios (both reachable):** (a) default-config vault with an unresolved 2026
  `ImportConflict` (a Hard blocker): the promote records fine (`would_conflict` diffs only
  `DecisionConflict`), the 2025 HIFO draw reorders exactly as r1 I-4 described, but BOTH folds'
  `tax_total(2025)` are `None` → no advisory, and the BG-D6 consent Σ term for 2025 is $0 (the
  `overpayment_delta_one` convention: uncomputable → `Usd::ZERO`) → the recorded `Acknowledgment`
  snapshots a $0 saving/exposure for a real five-figure position — the exact "bare $0" defect the fold
  forbids for the undisposed case, re-entered through the uncomputable door. (b) a `pre2025_method =
  Hifo` filer (explicitly supported — §A.7 allocations record non-FIFO methods) with 2018–2023 disposals:
  those years have NO bundled table, so their `tax_total` is `None` in every vault forever — the trigger
  is structurally unable to fire for the feature's own stated audience (Mt. Gox/LocalBitcoins-era, old
  filed years). The void direction inherits the same holes.
- **Code facts:** `tax_total` → `None` on `TaxOutcome::NotComputable` (`conservative.rs`);
  `compute_tax_year`'s three refusal doors — any Hard blocker anywhere, `tables.table_for(year)` miss,
  missing profile (`tax/compute.rs`); `BundledTaxTables::load()` = {2017, 2024, 2025, 2026}
  (`crates/btctax-adapters/src/tax_tables.rs`); `overpayment_delta_one` returns `Usd::ZERO` for an
  uncomputable with-scenario year (`conservative.rs`).
- **Fix direction (stays in-spec, one stroke for both surfaces):** define BOTH quantifications on the
  before/after **fold pair** (which the machinery already produces), not on tax alone: the BG-D9 advisory
  fires for any year `< current` whose per-year DISPOSAL-LEG set (equivalently Σ gain / 8949 content)
  differs between the two folds — profile/table/blocker-independent — quoting the tax-Δ when both years
  compute and otherwise the gain-Δ with an explicit "tax uncomputable for Y (no table/profile/blocked)"
  clause; BG-D6's Σ terms for uncomputable years must be surfaced as uncomputable (gain-delta line, or a
  refusal to promote until the year computes), NEVER a silent $0 term — and state that the Σ's
  "years with disposed tranche legs" scope is evaluated on the POST-promote fold (so reorder-created
  tranche years are counted consistently with the advisory shown beside it). Amend the §6 lifecycle KAT
  to pin the uncomputable-year case (advisory fires on a leg-diff in a table-less year).

### M-1 — BG-D9-i: the adjudication set for "has a live promote" is unstated — an inline implementation is order-dependent and permanently bricks a hand-crafted vault
- **Defect:** BG-D9-i refuses a tranche-void when a "live" promote exists, but does not say the predicate
  is evaluated against the FINAL live-promote set (i.e. deferred, like `allocation_voids` → step-3
  adjudication), rather than inline during the pass-1a void-classification loop with the incrementally
  built `voided` set.
- **Failure scenario:** a hand-crafted vault holds void-of-tranche (seq N) then void-of-promote (seq
  N+1). Inline: on every projection the tranche-void classifies first → inert + Hard `DecisionConflict`
  (the promote not yet voided), then the promote-void applies — end state: promote dead, tranche
  un-voidable, a permanent Hard gating every year, and NO clearing move, because the record-time
  double-void refusal ("a live `VoidDecisionEvent` already names this target", `cmd/reconcile.rs`) blocks
  re-issuing the tranche-void. Deferred: both apply cleanly (promote dead ⇒ no live promote ⇒ the
  tranche-void applies), which is the correct fixpoint. Product paths are safe either way
  (`would_conflict` refuses recording a tranche-void while a promote is live, so the product can only
  produce the void-promote-first order), so this is hand-crafted-vault hardening — the same class as the
  P9/T15 guard, and the §6 KAT list already claims "a RAW/hand-crafted void … cannot dangle the target".
- **Fix direction:** one sentence in BG-D9-i: the void-of-tranche is adjudicated against the FINAL
  non-voided-promote set (promote-voids apply unconditionally first; tranche-voids defer, mirroring
  `allocation_voids`' deferred step-3 adjudication — note promote-liveness depends only on
  promote-targeted voids, so the two-stage evaluation is acyclic and order-independent). Add the
  both-voids-either-order KAT.

## NIT

- **N-1 (BG-D6):** "either refuses or requires an explicit `--i-acknowledge <phrase>`" still leaves the
  plan a two-way choice on the non-interactive path. Both options satisfy the guarantee (the recorded
  snapshot is computed server-side either way), but pick one — the shipped `require_attestation`/
  `ATTEST_PHRASE` precedent suggests the flag form with the figures still printed to stdout.
- **N-2 (§3 item 13):** `cli/main.rs` contains a SECOND, textually identical `other => format!("{other:?}")`
  catch-all (`bulk_resolve_payload_summary`) that correctly needs NO promote arm (it renders imported
  conflict payloads only — a promote is unreachable). One parenthetical in item 13 saves the implementer
  a false lead when grepping.

---

## Census check (r2) — result

Independent full sweep of `EventPayload` consumers across `btctax-core`, `btctax-cli`, `btctax-tui`,
`btctax-tui-edit`, `btctax-adapters` product code: §3's payload-side census (items 9–15) is **complete**
— every site not listed is either variant-specific (catch-all correct for a promote), imported-payload-
scoped (unreachable), or test code. The tag-side census (items 1–8) re-verified: all five item-5 copy
strings, the item-1 `basis_methodology` sentence, the item-2 nudge/funnel site, the item-3/4 advisory
sites, and the item-8 `forms.rs` sentence exist in current source as described.

## Summary

| Severity | Count |
|---|---|
| Critical | 0 |
| Important | 1 |
| Minor | 1 |
| Nit | 2 |

Gate: **NOT green** (1 Important). The r1 fold is high quality — every resolution is real, not gestured
at, and the payload census survives an adversarial re-derivation. The one blocker is a precondition
inversion in the I-4 fix: the advisory/consent quantifications assume per-year tax computability that the
underlying leg-rewrite hazard does not require, and the shipped table set (2017/2024/2025/2026 only)
makes the gap structural rather than exotic. The fix is a predicate re-key (fold-diff, not tax-diff) plus
loud-uncomputable copy — fully inside the existing BG-D6/BG-D9 decision structure.
