# SPEC — bulk-void (queue item 3, Cycle 3) — sweep-void many reconcile decisions at once

**Source baseline:** `main` @ `13cb135` (branch `feat/bulk-void`). **Review status: R0-GREEN (2 rounds;
0 Critical / 0 Important). Reviews: `reviews/R0-spec-bulk-void-round-{1,2}.md` (both flagged decisions
adjudicated: bespoke persist, Tier-B). Cleared to implement.**
**Lineage:** queue item 3 of `bulk-reconcile-other-types` (architect-designed 2026-07-03, user-approved
safety-first sequencing). Cycles 1 (`persist_bulk_decisions` extract) + 2 (bulk-resolve-conflict) SHIPPED;
this is **Cycle 3 — the DANGEROUS one** (void has blast radius + a side-effect + a tax-safety trap).

## The feature
The bulk analog of the single-item void (`v` → `open_void_flow`). Sweep-void MANY reconcile **Decision**
events at once — accidental/wrong classifications, superseded lot selections, stale reclassifications — in
one filtered, per-row-excludable, confirmed, **atomic** batch. New TUI key **`V`** (Shift-v; `v` stays the
single void; `V` is free — confirmed against the current keymap). New CLI `reconcile bulk-void` (two-phase).

## Candidate set — a SINGLE shared predicate [Task 1 extraction; tax-safety-critical]
The single-void filter chain lives INLINE in `open_void_flow` (`main.rs:2733-2770`). Bulk MUST use the
**exact same** predicate — any drift is a tax-safety bug (see §Tax-safety). **Task 1 extracts it** into a
shared, CLI-and-TUI-reachable function and re-points `open_void_flow` (zero-behavior):

```
// btctax-core (new): pure, reads only events + the projected blockers — reachable by CLI (project())
// AND TUI (snapshot). is_revocable_payload MOVES here from btctax-tui-edit/edit/form.rs:896 (pure
// EventPayload match → belongs in core); tui-edit re-imports it (zero-behavior).
pub fn voidable_decisions<'a>(events: &'a [LedgerEvent], blockers: &[Blocker]) -> Vec<&'a LedgerEvent>
```
The predicate, verbatim from the shipped single-void (`main.rs:2733-2770`):
1. `matches!(e.id, EventId::Decision { .. })` — only Decision events.
2. NOT already-voided — `e.id` is not the `target_event_id` of any `VoidDecisionEvent`.
3. `is_revocable_payload(&e.payload)` — excludes `SupersedeImport` / `RejectImport` / `VoidDecisionEvent`
   (so a void is never itself voidable, and Cycle-2's resolve decisions aren't swept here).
4. `!effective_alloc(e)` — **the #7 exclusion.** `effective_alloc` = `e.payload` is
   `SafeHarborAllocation` AND NEITHER `SafeHarborTimebar` NOR `SafeHarborUnconservable` blocker fired on
   `e.id`. Engine evidence [R0-M1 — corrected cite; the `865-921` range in `main.rs:2749`/this spec was
   wrong]: unconservable ⟹ blocker (`resolve.rs:989-994`), timebarred ⟹ blocker (`resolve.rs:997-1002`),
   and voiding an EFFECTIVE allocation → Hard `DecisionConflict` (`resolve.rs:1030-1039`). INERT
   allocations (timebarred OR unconservable) STAY voidable — the void applies cleanly [R0-M2: source
   invariant `resolve.rs:1030-1031`; the "cleanly applies" behavior is pinned by KAT
   `crates/btctax-core/tests/transition.rs:403`, NOT a source line]. **Fix the same stale cite in
   `main.rs:2749` during Task 1.**

Both `Session::bulk_void_plan` (CLI) and `open_bulk_void_flow` (TUI) enumerate candidates via this one
function; `open_void_flow` is re-pointed to it. **No second copy of the predicate exists after Task 1.**

## Persist — `persist_bulk_void` [the delicate part: atomic batch + per-item side-effect]
Bulk-void appends N `VoidDecisionEvent`s AND, for each target that is a `LotSelection`, clears its
optimizer attestation (`optimize_attest::clear(conn, &ls.disposal_event)`) — ALL inside ONE atomic
envelope. This mirrors the shipped single `persist_void` (`persist.rs:248-300`) across N. It is a
**bespoke fn** (NOT a hook bolted onto the shared `persist_bulk_decisions`): keeping the dangerous
side-effect path isolated is safer than threading a closure through the 3-flow shared helper, at the cost
of ~15 mirrored lines (pinned by the mid-batch-rollback KAT, same shape as `persist_bulk_decisions`).
[R0-adjudicated: bespoke — blast-radius isolation.] **[R0-N2]** the bespoke fn carries a lockstep comment
cross-referencing `persist_bulk_decisions` (the mirrored safety skeleton) so a future edit to the shared
invariant is echoed here. **[R0-N1]** if two voided `LotSelection`s target the SAME disposal,
`optimize_attest::clear(disposal)` runs twice — harmless (a pure idempotent DELETE); the precompute may
dedup `disposal_to_clear`, but correctness does not depend on it.

```
pub fn persist_bulk_void(session, targets: Vec<VoidTarget>, now, empty_label) -> Result<usize, PersistError>
//   VoidTarget { target_event_id, disposal_to_clear: Option<EventId> }  ← disposal_to_clear precomputed
//   ONCE by the caller from the snapshot (LotSelection targets → ls.disposal_event); avoids N× load_all.
```
Exact skeleton (invariants transplanted from `persist_bulk_decisions` + `persist_void`):
1. `if targets.is_empty() { return Err(NoChange(Usage(empty_label))) }` — refuse empty BEFORE any mutation.
2. `let pre = session.snapshot()?;`
3. For each `t`: `append_decision(VoidDecisionEvent { t.target_event_id })`, guarded — on Err
   `return Err(rollback(session, &pre, e.into()))` (NOT `?`). Then, if `Some(disposal) = t.disposal_to_clear`:
   `if let Err(e) = optimize_attest::clear(session.conn(), &disposal) { return Err(rollback(session, &pre, e)) }`
   — the clear is INSIDE the envelope; a mid-batch append OR clear failure reverts the WHOLE batch (both
   void rows AND side-table clears — whole-DB restore covers the side-table for free, per persist_void [M1]).
4. `save_or_rollback(session, pre)?; Ok(n)` — ONE save; on failure the whole batch reverts.

## Tax-safety — the 3 hard points, focused on #7
- **#7 (the trap):** voiding an **effective** `SafeHarborAllocation` writes a permanent `VoidDecisionEvent`
  the engine rejects with **Hard `DecisionConflict`** (a damaging, unrecoverable no-op that gates the whole
  tax year). The shared predicate's `!effective_alloc` filter is the ONLY defense — bulk MUST use it (that's
  why Task 1 extracts, not copies). A dedicated KAT proves an effective allocation is NEVER a bulk candidate.
- Void events are themselves NON-revocable (`is_revocable_payload(VoidDecisionEvent) == false`) — a bulk-void
  cannot be un-done by another void; the ONLY recovery is re-applying the original decision. The confirm
  copy says exactly this (no false "you can undo this" reassurance).
- `DecisionConflict` is Hard + gates the year — the bulk op must never create one. (bulk-void writes no FMV,
  so points (a)/(b) about auto-FMV do NOT apply to this cycle.)

## Confirm — Tier-B (non-revocable) + blast-radius [consistent with bulk-resolve-conflict]
Bulk-void is non-revocable AND high-blast-radius → **Tier-B** (red border, prominent warning, **NOT** a
typed-word — Tier-C is reserved for the §7.4 unrecoverable attest batch). The single `v` is Tier-A; the
bulk sweep is upgraded to Tier-B to match its sibling bulk-resolve-conflict (also non-revocable) and the
blast radius. The modal states: **N** decisions will be voided; **these voids CANNOT themselves be undone**
(re-apply the original decision to restore); and the blast radius — how many are `LotSelection` voids that
**re-expose disposals to the default method + clear their optimizer attestation**. [R0: Tier-A vs Tier-B —
I chose Tier-B; the roadmap's early note said Tier-A.]

## Per-row preview
The checklist renders each candidate via `summarize_void_payload` (`main.rs:2641`) — payload tag + what the
void UNDOES + the inner target — so the user sees exactly what each row reverts before excluding any.
Reuse the shipped `TargetList` per-row-exclude widget (as bulk-link / bulk-sti / bulk-resolve do).

## CLI — two-phase (mirrors the shipped bulk commands)
- `reconcile bulk-void --dry-run` (Phase 1): `Session::bulk_void_plan` lists the voidable decisions (the
  shared predicate) with their `summarize`-style tags; prints the count; writes NOTHING.
- `reconcile bulk-void --yes` (Phase 2): `apply_bulk_void(vault, pp, targets, now)` in
  `btctax-cli/cmd/reconcile.rs` — opens the session, appends N `VoidDecisionEvent`s + per-`LotSelection`
  `optimize_attest::clear`, single `save`. CLI atomicity = bare `?` before `save` (in-memory discard);
  the TUI path uses `persist_bulk_void`'s explicit rollback. Returns the count voided.
- **[R0-M3 — the ONLY CLI-layer #7 defense]** `apply_bulk_void`'s `targets` MUST be exactly the
  `bulk_void_plan` rows (predicate-filtered), re-derived from the vault inside the dispatch — NEVER raw
  `--ref` ids from the user. The single CLI `void` does NO `effective_alloc` check, so a raw-id bulk path
  would let a caller void an effective allocation → Hard `DecisionConflict`. The dispatch re-runs the plan
  and passes its ids (mirror the bulk-resolve dispatch, `btctax-cli/src/main.rs:1267-1268`); a KAT feeds an effective
  allocation's id and asserts the plan omits it so apply never sees it.
- No `--accept/--reject` flags (void is single-valued); a `--dry-run` xor `--yes` guard as usual.

## Core / SemVer
- **btctax-core:** RELOCATION only — `is_revocable_payload` moves in from tui-edit + new pure
  `voidable_decisions(events, blockers)`. Reuses the existing `VoidDecisionEvent`. **No new `EventPayload`
  variant → no forward-only serde break; no behavior change** [R0-N3: the `optimize_attest` side-table is
  `btctax-cli`, not core — the per-`LotSelection` clear is a cli/tui-edit concern, not a core change] (the
  projection already handles `VoidDecisionEvent`). Additive cli + tui-edit. No `docs/manual`/GUI mirror
  beyond the new `bulk-void` subcommand's reference row + the `V` key in the (parked) help overlay.

## KATs
- `voidable_decisions_matches_single_void_flow` — the extracted predicate returns EXACTLY what
  `open_void_flow` listed (Task-1 zero-behavior; re-point proof).
- **`bulk_void_excludes_effective_allocation`** [#7 tax-safety] — an effective `SafeHarborAllocation` (no
  timebar/unconservable blocker) is NOT a candidate; **`bulk_void_includes_inert_allocation`** — a
  timebarred OR unconservable allocation IS.
- `bulk_void_clears_attestation_for_lotselection` — voiding a `LotSelection` target clears its
  `optimize_attestation` row, inside the batch.
- **`bulk_void_reverts_mid_batch`** [safety] — a failing append OR a failing `optimize_attest::clear` at
  row k>1 reverts the WHOLE batch: no phantom void rows AND no phantom-cleared attestations (snapshot ==
  post-state).
- `bulk_void_empty_refuses` (NoChange + exact empty-label; no snapshot/append); `bulk_void_dry_run_writes_nothing`.
- TUI: `bulk_void_refuses_when_no_candidates`, `bulk_void_per_row_exclude_drops_row`,
  `bulk_void_tier_b_not_typed_word` (modal is Tier-B, red, non-revocable copy, NOT typed-word), E2E
  `bulk_void_then_blockers_reexposed` (voiding classify-inbound decisions re-emits their inbound blockers).

## Plan (TDD, phased)
- **Task 1 — extract the voidable-candidate predicate** to `btctax-core` (`voidable_decisions` + move
  `is_revocable_payload`); re-point `open_void_flow` (zero-behavior; pinned by
  `voidable_decisions_matches_single_void_flow` + the shipped single-void KATs staying green UNCHANGED).
- **Task 2 — bulk-void:** `Session::bulk_void_plan` (shared predicate) + `persist_bulk_void` (atomic +
  per-`LotSelection` side-effect + mid-batch rollback) + CLI `apply_bulk_void` two-phase + TUI `V` flow
  (filter → `TargetList` checklist w/ `summarize_void_payload` → Tier-B blast-radius modal → `persist_bulk_void`).
- **Task 3 — whole-diff review (Phase E)** + full workspace suite + FOLLOWUPS.

## Gotchas
- **#7 is the whole ballgame** — bulk MUST NOT include an effective `SafeHarborAllocation` (→ Hard
  `DecisionConflict`, permanent damaging no-op). Share the predicate; never copy it. INERT allocations stay
  voidable (don't over-exclude).
- **The side-effect is inside the envelope** — `optimize_attest::clear` per `LotSelection` void must be in
  the SAME atomic batch + covered by the mid-batch rollback (transplant persist_void's airtight arm; a bare
  `?` on the committed append/clear leaks residue that piggy-backs a later save).
- **Precompute `disposal_to_clear` ONCE** from the snapshot (not N× `load_all` inside persist).
- **Void copy tells the truth** — voids are non-revocable; recovery = re-apply the decision. No false undo promise.
- Void is BTC-quantity-neutral / writes no FMV — the auto-FMV tax-safety points (a)/(b) don't apply here.
