# SPEC — self-transfer completion, Cycle B: matched in/out pairs

**Source baseline:** `main` @ `1dcacad` (post Cycle A; all anchors verified at write time).
**Review status: R0 round 1 folded (0C / 2I / 2M / 2N — GATE BLOCKED then folded); awaiting R0 round 2.
Review: `reviews/R0-spec-self-transfer-passthrough-round-1.md`.**
**Design lineage:** brainstorm with the user (2026-07-03) → architect design Part B (grounded, re-verified
at `1dcacad`). Second half of the "self-transfer completion" program (Cycle A = inbound self-transfer-in,
SHIPPED). Governed by the user-mandated policy in memory `self-transfer-completion-policy`.

**Goal.** Identify and confirm that an inbound leg + an outbound leg are the two sides of ONE self-transfer,
and reconcile the pair as a non-taxable self-transfer — with the user CONFIRMING every match (never
automatic). Two physical cases, resolved to two representations:
- **RELOCATE** (cross-wallet, destination tracked — e.g. Coinbase→River both imported): the coins must
  ARRIVE in the destination wallet with carried basis. This is EXACTLY the existing `TransferLink` out→in
  (`Op::SelfTransfer` relocates the source lot, `BasisSource::CarriedFromTransfer`). **No new core.**
- **DROP** (passthrough — coins in and out of a tracked waypoint, both counterparties external, e.g. the
  user's 50 BTC in+out of Coinbase): both legs net to zero, no lot, no tax. The existing primitives can't
  express this (see C1 rationale) → a **NEW `SelfTransferPassthrough` decision** that maps BOTH legs to
  `Op::Skip`.

**The determination rule.** Same tracked wallet on both legs, ordered in-then-out, counterparties
external ⇒ **DROP**. Different tracked wallets (out from X, in to Y, X≠Y) ⇒ **RELOCATE**. The user's
confirm choice IS the determination (the matcher pre-selects from wallet topology; the user can override).

---

## User-mandated policy (REQUIREMENTS — memory `self-transfer-completion-policy`; do not re-litigate)
- **Confirmed, NEVER automatic.** The matcher PROPOSES; nothing is written until the user confirms a pair.
  A coincidental amount match of a real income-in + real sale-out must NOT be auto-collapsed (it would
  hide TWO taxable events).
- **False-match safety** = the matcher only ever considers UNRECONCILED legs (an already-classified
  income or already-`ReclassifyOutflow`'d sale is not a candidate) + surfaces ambiguity (>1 match) rather
  than silently picking.
- **Non-taxable, outside FIFO** — both DROP and RELOCATE produce zero gain/loss and select no lots.
- **RELOCATE must land the coins in the tracked destination** with carried basis (else its future sale
  computes the wrong gain).

---

## SemVer / lockstep
- **btctax-core:** additive — new `EventPayload::SelfTransferPassthrough` + `SelfTransferPassthrough`
  struct + a `build_op` precedence branch. Consumers in-workspace → workspace lockstep rebuild.
- **Serialized vault (forward-only):** a vault CONTAINING a `SelfTransferPassthrough` decision fails to
  load on a pre-feature binary (serde unknown-variant) — the identical accepted trade-off documented for
  `ReclassifyIncome` (`event.rs:215-292`). Existing vaults load unchanged.
- **Fingerprint:** a decision → `fingerprint == None`; the `persistence::fingerprint` catch-all
  (`_ => return None`, `persistence.rs:96` [R0-M1] — NOT :127, which is `source_tag`) covers it for free.
  Add a KAT mirroring `reclassify_income_decision_has_no_fingerprint`.
- **Exhaustive `EventPayload` match sites needing a new arm** (compile-forced): render/persist/`fingerprint`
  paths + the TUI `is_revocable_payload` (I2). Grep every `match … EventPayload` / `EventPayload::` site.
- **No mirror:** no `docs/manual/` and no GUI crate (verified).
- **Session read-helper** (the matcher) is additive/flag-free → no lockstep.

---

## Grounding (verified at `1dcacad`)
- Decision variants: `EventPayload::{SafeHarborAllocation, RejectImport, ReclassifyIncome, …}`
  (`event.rs:281-292`); the `ReclassifyIncome` old-binary-fails-loud doc (`event.rs:215-218`) is the
  template. `persistence::fingerprint` catch-all `_ => None` (`persistence.rs:127`).
- `build_op` returns `Op::Skip` for `consumed_ins` (`resolve.rs:260-262`) and the `_ => Op::Skip` fallback
  (`:298`). The pass-1e decision collection: `TransferLink` (`resolve.rs:506`), `ClassifyInbound` (`:544`),
  `ReclassifyIncome` (`:650`) — the new decision mirrors these (validate targets, first-wins, build a set).
- Candidate sources: an unclassified `TransferIn` projects to `Op::UnknownInbound` (`resolve.rs:294`); an
  unmatched `TransferOut` lands in `state.pending_reconciliation` (`fold.rs:729`).
- Leg fields for matching: `TransferOut { sat, fee_sat: Option<Sat>, dest_addr, txid: Option<String> }`,
  `TransferIn { sat, src_addr, txid: Option<String> }` (`event.rs`).
- Reuse: the existing `link_transfer` out→in (RELOCATE); the `bulk_link_transfer_plan` read-helper pattern
  (`session.rs`) for the matcher; `persist_link_transfer`/the classify-inbound TUI + confirm-modal
  machinery for the confirm flows.

---

## C1 — core: the `SelfTransferPassthrough` DROP primitive

**Why a new primitive** (the genuinely-hard point, resolved by the architect):
- `TransferLink{out, InEvent(in)}` RELOCATES the OUT's source lots to the IN's wallet — but a passthrough
  OUT's source pool is empty (the IN, if skipped, created no lot) → `UncoveredDisposal` shortfall
  (`fold.rs`). Broken.
- Cycle A `SelfTransferMine` on the in-leg creates a lot, then the out-leg as `PendingOut` consumes it
  into `pending_reconciliation` — an advisory limbo, not a clean drop, and it records no "passthrough".
- A placeholder `self:external` relocation leaves a phantom holding — rejected by the "leave the tracked
  wallets entirely" requirement.
- **Resolution:** a decision that maps BOTH legs to `Op::Skip`.

```rust
// event.rs — new decision (follow the ReclassifyIncome old-binary-fails-loud doc pattern)
pub struct SelfTransferPassthrough {
    pub in_event: EventId,   // the TransferIn leg (coins arriving at the waypoint)
    pub out_event: EventId,  // the TransferOut leg (coins leaving the waypoint)
}
// EventPayload::SelfTransferPassthrough(SelfTransferPassthrough)
```

**Collection (pass-1e, mirror `ClassifyInbound` `resolve.rs:544-579`):** validate `in_event` resolves to
a `TransferIn` and `out_event` to a `TransferOut` — **BOTH targets valid or the WHOLE decision is
excluded** [R0-N2] (bad target → Hard `DecisionConflict`; neither leg enters `passthrough_skip`);
duplicate/first-wins; add **BOTH** ids to a `passthrough_skip: BTreeSet<EventId>`.

**[R0-I1 — MANDATORY cross-type overlap guard] a passthrough leg must be UNRECONCILED on both legs.**
After ALL pass-1e maps are built (a passthrough may be appended BEFORE the conflicting classification, so
the check cannot live inside either collector's own arm), EXCLUDE any `SelfTransferPassthrough` whose
`out_event ∈ (outflow_class ∪ link out-events)` OR `in_event ∈ (inbound_class ∪ consumed_ins)`, raising a
Hard `DecisionConflict` on the passthrough (remove BOTH its ids from `passthrough_skip`). **Rationale:** the
passthrough LOSES so the taxable classification WINS — otherwise a `SelfTransferPassthrough{out:B}` followed
by a real `ReclassifyOutflow{Dispose}` on B (no void) would `Op::Skip` B and **silently erase a taxable
disposal** (symmetric for `ClassifyInbound{Income}` on the in-leg). This is the exact "hide a taxable event"
failure the governing policy forbids; the guard makes the primitive safe-by-construction. First-wins is NOT
used across types here — the taxable event always wins, with a loud conflict. KAT both directions.

**`build_op` precedence:** check `passthrough_skip` **BEFORE** the `consumed_ins` / `inbound_class` /
`outflow_class` / `PendingOut` branches, returning `Op::Skip` for either leg. (So a passthrough-skipped
`TransferOut` never ALSO lands in `pending_reconciliation` — Gotcha G-PRECEDENCE. Safe because the I1 guard
guarantees a leg in `passthrough_skip` has no competing classification.)

**Conservation (trivially clean):** both legs contribute ZERO to `sigma_in/held/disposed/removed/pending`
and `fee_sats`. A passthrough is symmetric (in.sat ≈ out.sat) so skipping both leaves holdings + the FR9
identity balanced. Full audit trail (the append-only, voidable decision records WHY two imported
movements were dropped). Outside FIFO by construction (neither leg is a disposition or method-honoring).

**Decision hygiene:** `fingerprint == None` (catch-all `_ => return None`, `persistence.rs:96` [R0-M1]);
bad target → Hard `DecisionConflict` (excluded); duplicate → first-wins + conflict; cross-type overlap →
Hard `DecisionConflict` (I1). **Void re-exposes both legs** (in → `UnknownInbound`, out → `PendingOut`) —
core void works via the `VoidDecisionEvent` catch-all (`resolve.rs:344`).

**[R0-I2 — void surface, MANDATORY] the TUI must be able to void a mistaken DROP.** `is_revocable_payload`
(`form.rs:896`) is a SEPARATE allowlist that pre-filters the TUI void list (`main.rs:2717`), and
`summarize_void_payload` (`main.rs:2599`) has a `_ => "?"` catch-all. Task 1/3 MUST: add
`EventPayload::SelfTransferPassthrough(_)` to `is_revocable_payload` AND a real arm to
`summarize_void_payload` (else the DROP never appears in the TUI void list / renders as "?"), + a TUI KAT
that a persisted passthrough appears in the void list and voids to re-expose both legs. (CLI `reconcile
void <seq>` is the always-available undo; this closes the TUI surface.)

---

## C2 — the matcher (read-only proposal; NEVER automatic)

A new read-only `Session::self_transfer_match_plan(...) -> Result<Vec<MatchProposal>, CliError>`, modeled
on `bulk_link_transfer_plan`/`safe_harbor_residue` (appends/persists NOTHING). It pairs **only
unreconciled legs** — the primary false-match safety:
- **candidate ins** = `TransferIn` events projecting to `Op::UnknownInbound` (no `InboundClass` yet).
  **[R0-M2]** `Session::project()` exposes no per-event `Op` accessor, so ENUMERATE these from the
  `UnknownBasisInbound` blocker set (`fold.rs:817`, carrying the in-event id) joined to the raw `TransferIn`
  via the event index — the pattern `bulk_link_transfer_plan` already uses (`session.rs:298`).
- **candidate outs** = `state.pending_reconciliation` entries (already `Op::PendingOut`; directly usable).

**Match criteria (ALL required; a pair failing any is not proposed):**
1. **Amount within a fee tolerance:** `|in.sat − out.principal_sat| ≤ tol`, `tol` accommodating the
   on-chain network fee (dest receives out.principal minus fee; `out.fee_sat`). Exact = strong; a small
   positive `out − in` gap is fee-consistent.
2. **Time window:** legs within N days (exchange timestamp drift). Cross-wallet: in on/after out.
   Passthrough: in BEFORE out (receive precedes withdraw).
3. **One-in / one-out:** if >1 candidate in OR out matches the same amount/window → **`ambiguous`**,
   surfaced flagged, NEVER silently picked.
4. **`txid` corroboration (bonus, decisive for RELOCATE):** `in.txid == out.txid` ⇒ literally the same
   on-chain tx ⇒ near-certain cross-wallet self-transfer. Passthrough legs are two DIFFERENT txs (arrive
   in one, leave in another), so a txid non-match is itself a passthrough signal.

**Suggested defaults (tunable — settle exact values at R0/impl):** amount tolerance `tol = max(out.fee_sat
unwrap_or 0, ceil(0.005 × out.sat))` (fee-consistent, ≤ 0.5% slack); time window `±2 days`. A `txid`
EXACT match relaxes the amount check (same tx ⇒ trust it) but NOT the one-in/one-out ambiguity guard.
These are conservative on purpose (under-propose rather than over-propose).

**`MatchProposal`** carries per pair: both legs' dates/wallets/sats, USD value (`price::fmv_of`), the
current status of each leg ("this in is an unknown-basis blocker; this out is a pending outflow"), the
**suggested action** (DROP vs RELOCATE, from wallet topology — same-wallet ⇒ DROP, cross-tracked-wallet ⇒
RELOCATE), and `ambiguous`. This context is the second false-match safety: the user sees exactly which two
real events they are collapsing before confirming.

Tolerance/window are conservative heuristics — a false NEGATIVE (an unproposed real self-transfer) is
cheap (reconcile manually); a false POSITIVE silently applied is a compliance error, so the design errs
toward under-proposing + always requiring confirmation.

---

## C3 — the confirm flow (single-pair; NEVER auto)

- **CLI (two-phase, mirror bulk-link):** `reconcile match-self-transfers` renders the proposed pairs
  (each with its two `EventId`s + suggested action + `ambiguous` flag); the user confirms a pair with an
  explicit action → dispatch to the EXISTING `link_transfer(out, InEvent(in))` (RELOCATE) OR a new thin
  `apply_self_transfer_passthrough(in_ref, out_ref)` that appends one `SelfTransferPassthrough` (DROP).
  Ambiguous pairs require explicit `in`+`out` refs (no auto-pick).
- **TUI (proposal-list flow):** mirror the classify-inbound list + confirm-modal — list proposed pairs →
  select → the modal shows BOTH legs + the DROP/RELOCATE choice → Enter persists (via
  `persist_link_transfer` for relocate, or a new `persist_self_transfer_passthrough` snapshot/
  `save_or_rollback` for drop). **Never auto-applied.**

---

## Invariants (KAT-pinned)
1. **DROP correctness:** `SelfTransferPassthrough{in,out}` on a same-wallet passthrough → both legs
   `Op::Skip`; NO lot anywhere; holdings unchanged; `income_recognized`/`disposals`/`removals` empty for
   both; `conservation_report(...).balanced`.
2. **RELOCATE correctness (the Req condition):** cross-wallet via `TransferLink{out, InEvent(in)}` →
   destination holds the coins, `usd_basis == source basis`, `basis_source == CarriedFromTransfer`, source
   holds 0; a later destination sale computes gain off the carried basis (NOT $0, NOT FMV).
3. **Confirmed-not-automatic:** the matcher persists NOTHING — running it leaves the event log
   byte-identical.
4. **False-match safety:** an already-`Income`-classified in + already-`Dispose`-classified out is NOT
   proposed (neither is a candidate). An ambiguous 1-in/2-out collision is surfaced `ambiguous`, never
   auto-picked.
5. **Precedence:** the `passthrough_skip` check precedes `PendingOut`/`inbound_class` in `build_op` → a
   passthrough-skipped `TransferOut` never also lands in `pending_reconciliation`.
6. **Decision hygiene:** bad target (either leg) → Hard `DecisionConflict`, WHOLE decision excluded;
   duplicate → first-wins; Void re-exposes both legs (in → `UnknownInbound`, out → `PendingOut`);
   `fingerprint == None`.
7. **Outside FIFO:** a `LotSelection` targeting either passthrough-skipped leg → `LotSelectionInvalid`.
8. **[I1] cross-type overlap → the taxable event survives:** `SelfTransferPassthrough{out:B}` THEN
   `ReclassifyOutflow{Dispose}` on B (no void) → Hard `DecisionConflict`, the passthrough is EXCLUDED, B
   projects as the `Dispose` (the disposal is RECOGNIZED, not silently skipped). Symmetric for
   `ClassifyInbound{Income}` on the in-leg. **KAT both directions** — the load-bearing tax-safety invariant.
9. **[I2] void surface:** a persisted `SelfTransferPassthrough` appears in the TUI void list
   (`is_revocable_payload` includes it; `summarize_void_payload` renders it, not "?") and voids to
   re-expose both legs.

## KATs
- **btctax-core:** invariants 1/5/6/7; **invariant 8 [R0-I1] BOTH directions** (passthrough+Dispose-on-out
  → conflict + disposal recognized; passthrough+Income-on-in → conflict + income recognized); serde
  round-trip + old-binary-fails-loud; the `DecisionConflict` bad-target (both legs) + first-wins;
  void-re-exposes-both.
- **btctax-cli:** the matcher `self_transfer_match_plan` proposes the right pairs (amount/window/one-in-one-
  out/txid; DROP vs RELOCATE suggestion by wallet topology); false-match KAT (already-classified legs not
  proposed); ambiguity flagged; `match-self-transfers` two-phase render + `--dry-run`; `apply_self_transfer_
  passthrough` appends one decision; the relocate path routes to `link_transfer`.
- **btctax-tui-edit:** the proposal-list flow lists pairs, the modal DROP/RELOCATE choice, persist
  strict-prefix (drop appends `SelfTransferPassthrough`; relocate appends `TransferLink`), cancel/save-
  error; **invariant 9 [R0-I2] void surface** (a persisted passthrough appears in the void list + voids to
  re-expose both legs — `is_revocable_payload` + `summarize_void_payload` arms); E2E: a same-wallet
  passthrough pair → confirm DROP → both legs Skip; a cross-wallet pair → confirm RELOCATE → destination
  holds the coins.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)
- **Task 1 — core `SelfTransferPassthrough`** (`EventPayload` variant + collection + `passthrough_skip`
  + the **[I1] cross-type overlap guard** + `build_op` precedence + `Op::Skip` both legs + hygiene + the
  **[I2] void arms** (`is_revocable_payload` + `summarize_void_payload`); invariants 1/5/6/7/8/9). The
  heart + the two tax-safety invariants.
- **Task 2 — the matcher + CLI** (`Session::self_transfer_match_plan` read helper + `match-self-transfers`
  two-phase CLI + `apply_self_transfer_passthrough`; matcher + false-match + ambiguity KATs).
- **Task 3 — TUI proposal-list confirm flow** (list + modal DROP/RELOCATE + `persist_self_transfer_
  passthrough`; TUI KATs + the two E2Es).
- **Task 4 — whole-diff review (Phase E) + FOLLOWUPS** (record bulk-confirm-matches as a later slice).

## Gotchas (for the reviewer)
- **G-PRECEDENCE:** `passthrough_skip` MUST be checked before `PendingOut`/`inbound_class`/`outflow_class`
  in `build_op` — else a passthrough out also lands in `pending_reconciliation` (double-counted).
- **G-BOTH-ATOMIC:** one `SelfTransferPassthrough` decision skips BOTH legs. Skipping only the in has TWO
  failure modes [R0-N1]: if the waypoint has no other coins → `UncoveredDisposal` shortfall (loud); if it
  DOES hold other lots → the orphaned `PendingOut` silently MIS-CONSUMES real lots (`fold.rs:712`) — worse,
  no shortfall. Both prove the pair must be ONE atomic primitive.
- **G-OVERLAP [R0-I1]:** a passthrough leg that also carries a reclassification/link/inbound-class is a Hard
  `DecisionConflict` (the passthrough loses); NEVER let a passthrough `Op::Skip` a leg that has a competing
  taxable classification — that silently erases a disposal/income.
- **G-FALSE-MATCH:** the matcher considers ONLY unreconciled legs + flags ambiguity + never auto-applies —
  the whole point (a wrong auto-collapse hides two taxable events).
- **G-RELOCATE-REUSE:** the cross-wallet case is the EXISTING `link_transfer` — do NOT reinvent it; the
  confirm flow just routes to it. Only DROP is new core.
- **G-SYMMETRY:** DROP assumes a symmetric pair (in.sat ≈ out.sat within fee tol); the matcher enforces
  it, but the primitive itself trusts the confirmed pair (garbage-in if a caller passes a mismatched
  pair — acceptable, it's user-confirmed, but note it).

## Out of scope (later)
- **Bulk-confirm** of many proposed matches at once (after single-pair ships) — mirrors the bulk-link
  all-or-nothing/one-save pattern.
- Auto-application without confirmation (explicitly rejected by policy).
- Any change to the RELOCATE (`TransferLink`) path beyond routing to it.
