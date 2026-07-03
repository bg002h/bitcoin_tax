# R0 spec review — `design/SPEC_self_transfer_passthrough.md` (round 1)

**Artifact:** `design/SPEC_self_transfer_passthrough.md` @ `6709dff` (branch `feat/self-transfer-passthrough`; main == `1dcacad`).
**Reviewer role:** independent adversarial architect (did NOT author). btctax-core-touching, tax-safety-critical.
**Gate:** 0 Critical / 0 Important to proceed to implementation.

## Verdict: **0 Critical / 2 Important / 2 Minor / 2 Nit**

The core design is **sound** on the points that matter most for tax safety:

- **DROP = `Op::Skip` both legs is correct.** `Op::Skip => {}` is a genuine no-op in `fold_event` (`crates/btctax-core/src/project/fold.rs:1220`): no lot, no income record, no disposal/removal, no `pending_reconciliation` push, holdings untouched, and `Op::Skip` contributes nothing to any `FoldStats` accumulator. For a passthrough where the waypoint had no other coins, skipping both legs *clears* the pre-existing `UnknownBasisInbound` (in-leg) and `UncoveredDisposal` (out-leg consuming an empty pool) blockers, so `conservation_report(...).balanced` flips true — exactly invariant 1. Conservation stays balanced regardless of leg symmetry, because neither leg ever contributed to `Σin` (`UnknownInbound` adds no `sigma_in`; `PendingOut` only moves held→pending). ✓
- **Precedence claim (the "Critical if wrong" item) is correct.** The `pending_reconciliation` push happens in the fold's `Op::PendingOut` arm (`fold.rs:729`), and `Op::PendingOut` is the *fallthrough* of `build_op`'s `TransferOut` arm (`resolve.rs:254-258`). A `passthrough_skip.contains(id)` guard at the top of `build_op` returns `Op::Skip` before any `TransferOut`/`TransferIn` branch is reached, so a passthrough out never also lands pending. G-PRECEDENCE holds. ✓
- **False-match candidate sets are airtight.** Candidate outs = `state.pending_reconciliation`, which by construction holds only unmatched `TransferOut`s (a linked/reclassified out is `Op::SelfTransfer`/`Op::Dispose`/… and never enters the queue — confirmed by the `bulk_link_transfer_plan` doc, `session.rs:281`). Candidate ins = `Op::UnknownInbound`, i.e. a `TransferIn` with no `inbound_class`; an already-`Income`/`GiftReceived`/`SelfTransferMine`-classified in projects to a different op and is not a candidate. Invariant 4 is achievable. ✓
- **RELOCATE reuse is real.** Cross-wallet routes to the existing `link_transfer(out, InEvent(in))` (`reconcile.rs:252`) → `Op::SelfTransfer{dest = in.wallet}` (`resolve.rs:219`) → fold relocates source lots with `BasisSource::CarriedFromTransfer` (`fold.rs:779`). No new core. The C1 rationale (a passthrough out's source pool is empty ⇒ `UncoveredDisposal` if you tried to link it) is accurate. ✓
- **Core void works for free.** A `VoidDecisionEvent` targeting a `SelfTransferPassthrough` hits the `Some(_) => voided.insert(...)` catch-all (`resolve.rs:344`), so the passthrough is skipped in pass-1e and both legs re-project (in→`UnknownInbound`, out→`PendingOut`). Invariant 6's core mechanism is correct.
- **Leg fields verified.** `TransferOut { sat, fee_sat: Option<Sat>, dest_addr: Option<String>, txid: Option<String> }` and `TransferIn { sat, src_addr: Option<String>, txid: Option<String> }` (`event.rs:71-82`) — the matcher's criteria are computable, and `PendingTransfer` exposes `principal_sat`/`fee_sat` (`state.rs:205`) so the amount check is feasible. `out.principal_sat == TransferOut.sat` (fee is a separate field), so the spec's mixed use of `out.sat`/`out.principal_sat` is self-consistent. ✓

The two Important findings are hygiene/completeness gaps around **conflicting decisions on a skipped leg** and the **void surface**, not flaws in the happy-path engine behavior. Both are cheap to close at the spec level.

---

### [I1] IMPORTANT — a passthrough leg that is ALSO reclassified silently erases the real disposal/income (no conflict)
`crates/btctax-core/src/project/resolve.rs:96-99` (`build_op` precedence) · spec §C1 "Decision hygiene" (lines 105-106) · Invariant 6 (lines 175-176)

**Defect.** The spec's precedence — `passthrough_skip` checked **before** `outflow_class`/`inbound_class`/`links` — combined with a hygiene section that only handles *same-type* duplicates and *bad targets*, leaves the **cross-decision-type overlap** case silent and dangerous:

1. User confirms `SelfTransferPassthrough{in:A, out:B}` (B enters `passthrough_skip`).
2. Later the user (or a future flow) appends `ReclassifyOutflow{Dispose}` on B — a real sale — *without* first voiding the passthrough. Collection validates B is a `TransferOut`, sees no duplicate `ReclassifyOutflow`, and inserts it into `outflow_class` (`resolve.rs:615-632`). **No conflict blocker is raised.**
3. `build_op` checks `passthrough_skip` first → B → `Op::Skip`. The `Dispose` is silently dropped. **A taxable disposal is hidden with no signal.**

The symmetric in-leg case hides income: `SelfTransferPassthrough{in:A,...}` then `ClassifyInbound{Income}` on A → the income is silently skipped. Both directions are exactly the "hide a taxable event" failure the governing policy exists to prevent — here reachable not by the matcher (which won't propose a reconciled leg) but by the append-only accumulation of decisions in the wrong order.

**Why it gates.** This is a silent-erasure-of-a-disposal path in a tax-safety-critical primitive whose default outcome ("nothing happened") is the maximally dangerous one. The spec is currently *silent* on it — an implementer will just "check `passthrough_skip` first" and ship the silent-win.

**Note / fair calibration.** The codebase already accepts an analogous silent precedence for `TransferLink`-vs-`ReclassifyOutflow` and `TransferLink`-vs-`ClassifyInbound`, explicitly documented as "a *precedence* question, not a bad target; unchanged" (`resolve.rs:617-620`, `:566-568`). So parity with that pattern is a *defensible* resolution — but it must be a **conscious, recorded** decision at R0, not a default.

**Fix (either, but it must be addressed + KAT'd):**
- **(preferred)** At pass-1e collection, raise a Hard `DecisionConflict` (and exclude the passthrough, or the later reclassification) when a passthrough leg overlaps another classification of the same leg — i.e. `in_event ∈ (inbound_class keys ∪ consumed_ins)` or `out_event ∈ (outflow_class keys ∪ links keys)`. Because collection order matters (a `ReclassifyOutflow` appended *after* the passthrough won't see it during its own arm), do the cross-check **after all pass-1e maps are built**, or symmetrically in both collectors. This makes the conflict fail loud.
- **(minimum)** If you choose parity with the existing `TransferLink` precedence, the spec must (a) state the precedence explicitly, (b) explain why silently dropping a later disposal on a passthrough'd out is acceptable, and (c) add a KAT pinning the behavior. Silence is not an option for this primitive.

---

### [I2] IMPORTANT — the void surface is under-specified; the TUI cannot offer a mistaken DROP for voiding
`crates/btctax-tui-edit/src/edit/form.rs:896-910` (`is_revocable_payload`) · `crates/btctax-tui-edit/src/main.rs:2599-2657` (`summarize_void_payload`) · spec §SemVer (39-49), Task 3 (196), Invariant 6 (175-176)

**Defect.** The spec claims "Void re-exposes both legs" (invariant 6) and scopes a first-class TUI flow (Task 3), but never enumerates the two TUI match sites that gate whether a decision is *offered* for voiding:

- `is_revocable_payload` (`form.rs:896`) is a **separate allowlist** from the core void catch-all. It lists every revocable decision type by variant (`TransferLink … SafeHarborAllocation`) and is the pre-filter for the TUI void list (`main.rs:2717`). If `SelfTransferPassthrough` is not added here, the DROP **never appears in the TUI void list** — the user cannot undo a mistaken passthrough through the documented surface.
- `summarize_void_payload` (`main.rs:2599`) has a `_ => ("?", "?", None, false)` catch-all (`:2656`). Any passthrough that reaches the void list would render as **"?"** with no target summary.

The core mechanism works (via the `Some(_)` catch-all in `resolve.rs:344`), and CLI `reconcile void <seq>` (`reconcile.rs:107`, which appends a `VoidDecisionEvent` for any target without a revocability filter) is an existing partial undo path — so this is **not** an undo-impossible Critical. But for a decision whose entire purpose is a reversible non-taxable collapse, leaving the primary TUI surface unable to void it is a real completeness gap that an implementer following the spec's task list literally will ship wrong.

**Why it gates.** Invariant 6 asserts a property the spec's own task breakdown does not wire through the scoped TUI surface, and the load-bearing site (`is_revocable_payload`) is non-obvious (a second allowlist, not the core catch-all) and easy to miss.

**Fix.** In Task 1/3, explicitly require: add `EventPayload::SelfTransferPassthrough(_)` to `is_revocable_payload` (`form.rs:896`) and a real arm to `summarize_void_payload` (`main.rs:2599`); add a TUI KAT that a persisted passthrough appears in the void list and voids to re-expose both legs. If you deliberately descope TUI-void for Cycle B, say so explicitly and name CLI `reconcile void` as the sanctioned undo (and still add the two arms to avoid the "?" render / silent omission).

---

### [M1] MINOR — wrong citation for the `fingerprint` catch-all
spec lines 46-47 and 56 cite `persistence.rs:127` / `_ => None`

**Defect.** The `persistence::fingerprint` catch-all is `_ => return None` at **`crates/btctax-core/src/persistence.rs:96`** (inside `fingerprint`, which fingerprints only the 6 imported payloads). Line **127** is `_ => None` inside an unrelated function, `source_tag` (the source-string matcher). The cited line/pattern points at the wrong function.

**Why it matters.** CLAUDE.md requires citations verified against current source at write time; a wrong anchor into a same-file neighbor is exactly the decay this guards against. The *substance* is correct — a `SelfTransferPassthrough` decision is not an imported payload, so it hits the `_ => return None` catch-all and gets `fingerprint == None` for free — so this does not block on behavior.

**Fix.** Change the citation to `persistence.rs:96` (`_ => return None`).

---

### [M2] MINOR — matcher candidate-in enumeration source is unspecified (no per-event `Op` accessor exists)
spec §C2 lines 115-116 ("candidate ins = `TransferIn` events projecting to `Op::UnknownInbound`")

**Defect.** `Session::project()` returns a `LedgerState` that exposes `pending_reconciliation` (candidate outs — directly usable) and `blockers`, but **not** a per-event `Op`/timeline. There is no public accessor for "which events projected to `Op::UnknownInbound`." The practical source is the `UnknownBasisInbound` blocker set emitted by the fold (`fold.rs:817`, carrying the in-event `EventId`), joined back to the raw `TransferIn` via the event index (the same `index: HashMap<EventId,&LedgerEvent>` pattern `bulk_link_transfer_plan` already uses, `session.rs:298`).

**Why it matters.** Left as-is, an implementer may assume a timeline accessor that doesn't exist, or expose one unnecessarily. Naming the blocker-driven source keeps Task 2 aligned with the established `bulk_link_transfer_plan` helper shape.

**Fix.** In §C2, state that candidate ins are enumerated from the `UnknownBasisInbound` blockers (event id) and hydrated from the event index — mirroring how candidate outs come from `pending_reconciliation`.

---

### [N1] NIT — G-BOTH-ATOMIC's "shortfall" illustration covers only one of two failure modes
spec line 203-204 (G-BOTH-ATOMIC), §C1 (72-75)

Skipping only the in-leg yields `UncoveredDisposal` **only when the waypoint pool lacks other coins**. If the waypoint holds sufficient other lots, the orphaned out (`Op::PendingOut`) silently *mis-consumes real lots* into pending (`fold.rs:712`) — a wrong result *without* a shortfall blocker, arguably worse than the cited failure. The design is fine (one decision skips both legs atomically); only the rationale is incomplete. Consider noting both failure modes so the "one primitive" justification is airtight.

### [N2] NIT — spec should state both-or-nothing on bad target
spec §C1 lines 91-94

The spec says validate in→`TransferIn` and out→`TransferOut`, bad→`DecisionConflict` excluded, and "add BOTH ids" on success. Make explicit that if **either** target is bad the **entire** decision is excluded (neither leg enters `passthrough_skip`) — the natural reading, but worth pinning so an implementer doesn't half-apply a decision with one good leg.

---

## Confirmations (pressure-test items that passed)
1. **DROP `Op::Skip` both legs** — correct; `Op::Skip => {}` no-ops the fold; conservation balanced for symmetric *and* asymmetric confirmed pairs (neither leg touches `Σin`). G-BOTH-ATOMIC's core claim (skip-only-in is broken) holds (see N1 for a completeness nit).
2. **Precedence (Critical-if-wrong)** — correct and complete; a top-of-`build_op` `passthrough_skip` guard wins over the `TransferOut`/`TransferIn` arms, so no leg slips to `PendingOut`/`inbound_class`/`outflow_class`.
3. **False-match safety** — candidate sets (`UnknownInbound` ins, `pending_reconciliation` outs) cannot contain a reconciled leg; ambiguity (>1 match) flagged, never auto-picked; the only residual risk (coincidental equal-amount unreconciled income+sale) is correctly mitigated by mandatory confirm + criteria, not by the candidate sets. Airtight at the set level. (I1 is a *post-confirm accumulation* hazard, orthogonal to the propose-time candidate sets.)
4. **Matcher criteria + defaults** — `tol = max(fee_sat, ceil(0.005·out.sat))`, ±2d, txid bonus — all computable from `PendingTransfer.{principal_sat,fee_sat}` and the raw legs' `txid`; `out.principal_sat == out.sat` so the formula is self-consistent; the only foot-gun (two equal-amount self-transfers in-window) is handled by the ambiguity guard.
5. **RELOCATE reuse** — genuinely the existing `link_transfer` → `Op::SelfTransfer` with `CarriedFromTransfer`; determination rule (same-wallet⇒DROP, cross-tracked-wallet⇒RELOCATE) is correct; out→untracked / in-from-untracked resolve as "no candidate ⇒ no proposal ⇒ manual," and multi-hop chains resolve via sequential single-pair confirms — no gap.
6. **Decision hygiene + SemVer** — additive variant, forward-only serde-fail-loud (parity with `ReclassifyIncome`, `event.rs:214-218`), workspace lockstep, `fingerprint == None` via catch-all (behavior correct; cite fixed in M1), core void via `Some(_)` catch-all (`resolve.rs:344`). Persistence `kind` column is keyed off `EventId` type (`KIND_DECISION`), not payload variant, so no new persistence arm is needed — spec's implicit assumption holds. Gaps: I1 (cross-type overlap) and I2 (TUI void sites).
7. **Scope/decomposition** — Cycle B scoping (primitive + matcher + single-pair confirm; bulk deferred) is coherent. Under-specified items: I2 (void surface), M2 (candidate-in source), N2 (both-or-nothing).

**R0 gate: BLOCKED on I1 + I2.** Both are spec-level and cheap to close (name the cross-type-conflict decision + KAT; name the two TUI void sites + KAT). No Critical found; the engine happy-path design is sound.
