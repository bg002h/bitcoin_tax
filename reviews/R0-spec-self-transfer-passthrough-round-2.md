# R0 spec review — `design/SPEC_self_transfer_passthrough.md` (round 2)

**Artifact:** `design/SPEC_self_transfer_passthrough.md` (folded after round 1).
**Reviewer role:** independent adversarial architect (did NOT author).
**Round-1 findings:** 0C / 2I / 2M / 2N. This round verifies the folds + checks for new drift.

## Verdict: **0 Critical / 0 Important / 1 Minor / 1 Nit → R0-GREEN**

Both Important findings are **correctly and completely resolved**. The one residual is a Minor stale-citation leftover from the M1 fold; it does not gate. R0 passes.

---

## [I1] — RESOLVED ✓ (the tax-safety one). Guard is correct AND complete.

Spec §C1 (lines 99-108), Invariant 8 (207-210), G-OVERLAP (249-251), the precedence note (112-113).

The author took the **preferred (loud-conflict) fix**. I traced it end-to-end against source:

**The four sets are exactly right and exhaustive** — they are the complete reconciliation-indicator set for each leg:
- **Out-leg** is reconciled iff `out_event ∈ outflow_class` (→ `Op::GiftOut`/`Donate`/`Dispose`, `resolve.rs:228-253`) **or** `out_event ∈ links.keys()` (→ `Op::SelfTransfer`, `:210-224`). A `link` present in the map always resolves to `Op::SelfTransfer` (the no-wallet fall-through can't occur — collection rejects an unresolvable-wallet link and never inserts it, `:524-534`), so `links.keys()` is a sound "linked out" set. No other decision reconciles a raw `TransferOut` (`LotSelection` only targets an already-honoring op, never a raw out). **Complete.**
- **In-leg** is reconciled iff `in_event ∈ inbound_class` (→ `Op::IncomeInbound`/`GiftReceived`/`SelfTransferInbound`, `:263-292`) **or** `in_event ∈ consumed_ins` (→ `Op::Skip` via a link, `:260-262`). `ManualFmv` cannot target a `TransferIn` (it validates an `Income` target, `:468-486`), so nothing else reconciles an in. **Complete.**

**The "after all pass-1e maps are built" timing is necessary and correct.** The collection loop iterates decisions in `decision_seq` order; a `SelfTransferPassthrough` may be appended *before* the conflicting `ReclassifyOutflow`/`ClassifyInbound`. A guard inside either collector's own arm would miss the opposite ordering. Running it once, after the loop populates `links`/`consumed_ins`/`inbound_class`/`outflow_class`/`passthrough_skip`, catches the overlap regardless of append order. ✓

**Excluding the passthrough genuinely makes the taxable event survive** — traced:
- Overlap on the out: guard removes B from `passthrough_skip` → `build_op(B)` skips the (now-missing) `passthrough_skip` check → falls to `outflow_class.get(B)` → **`Op::Dispose`**; the fold records the disposal, computes gain, consumes lots. The disposal is RECOGNIZED, not silently `Op::Skip`'d. ✓
- Overlap on the in: guard removes A → `build_op(A)` → `inbound_class.get(A)` → **`Op::IncomeInbound`** → income lot at FMV + IncomeRecord. Income RECOGNIZED. ✓
- The *non-overlapping* leg of an excluded passthrough correctly reverts to its natural op (`UnknownInbound` / `PendingOut`) — a safe "return to unreconciled," never a silent drop.

**No self-inflicted precedence hole.** Because the guard guarantees any id left in `passthrough_skip` has no competing classification, checking `passthrough_skip` first in `build_op` is now provably safe (the spec's added note at 112-113 states exactly this). The I1 fix and the G-PRECEDENCE requirement are mutually consistent. Invariant 8 + the both-directions KAT pin it.

This is the load-bearing tax-safety property, and it is now safe-by-construction. Fully resolved.

## [I2] — RESOLVED ✓. Void surface specified.

§C1 (125-131), Invariant 9 (211-213), Task 1 (232-235), the TUI KAT (224-227). The spec now mandates adding `EventPayload::SelfTransferPassthrough(_)` to `is_revocable_payload` (`form.rs:896`, the separate allowlist that pre-filters the void list at `main.rs:2717` — anchor re-verified) **and** a real `summarize_void_payload` arm (`main.rs:2599`, else the `_ => "?"` catch-all), plus a TUI void KAT, with CLI `reconcile void` noted as the always-available undo. Both load-bearing sites named; the "void re-exposes both legs" invariant is now wired through the scoped TUI surface. Resolved.

## [M2] / [N1] / [N2] — RESOLVED ✓.
- **M2:** candidate-ins enumerated from the `UnknownBasisInbound` blocker set (`fold.rs:817`) joined via the event index (`session.rs:298`) — accurate; no per-event `Op` accessor is assumed. ✓
- **N1:** G-BOTH-ATOMIC now states both failure modes (loud `UncoveredDisposal` when the waypoint is empty; silent mis-consume of real lots at `fold.rs:712` when it holds others). Anchor verified. ✓
- **N2:** both-or-nothing on bad target stated (95-96, Invariant 6). ✓

---

### [M1-residual] MINOR — one stale copy of the wrong fingerprint citation survived the fold
`design/SPEC_self_transfer_passthrough.md:59`

The M1 fix corrected the SemVer bullet (line 47 → `persistence.rs:96`, `_ => return None`) and the hygiene line (120), but the **Grounding** block at line 59 still reads:
`persistence::fingerprint catch-all _ => None (persistence.rs:127)`.
That is the exact defect M1 named (`:127` is `source_tag`, not `fingerprint`), and it now directly contradicts the corrected line 47 in the same document. The Grounding block is the authoritative anchor list, so the stale copy is the worst one to leave. Non-blocking, but fix line 59 to `_ => return None` / `persistence.rs:96` for internal consistency.

### [N3] NIT — duplicate-`SelfTransferPassthrough` key is under-specified for a two-target decision
§C1 line 97 ("duplicate/first-wins")

"Duplicate/first-wins" mirrors the single-target `ClassifyInbound`, but a passthrough carries TWO target ids. Define the dup condition as **"either leg already claimed"** — i.e. `in_event ∈ passthrough_skip` OR `out_event ∈ passthrough_skip` → first-wins + `DecisionConflict` — mirroring how `TransferLink` guards BOTH `links.contains_key(out)` (`resolve.rs:507`) and `consumed_ins.contains(in)` (`:516`). Not a safety hole (worst case is a safe over-conflict, never a silent taxable-event erasure), so it doesn't gate — but pin the key so an implementer doesn't only check one leg.

---

## New-drift check: none.
The I1 guard introduces no new hazard — I checked its interaction with voids (voided passthroughs are skipped before collection, `resolve.rs:502`, so the guard never sees them), with the non-overlapping-leg revert (safe), and with the `DecisionConflict` target (raised on the passthrough decision id, pointing the user at the right thing to void). Conservation, precedence, false-match candidate sets, and RELOCATE reuse are unchanged from round 1 and remain sound.

**R0 gate: GREEN (0C / 0I).** Recommend sweeping the [M1-residual] line-59 citation and pinning [N3] during Task 1, but neither blocks. Cleared for implementation.
