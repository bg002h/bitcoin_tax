# SPEC — bulk-resolve-conflict (+ persist_bulk_decisions extraction)

**Source baseline:** `main` @ `719e9fe` (all anchors verified at write time).
**Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-bulk-resolve-conflict-round-{1,2}.md` (round 1: 0C/1I — the CLI `ResolveKind` cross-crate
catch; round 2: 0C/0I, 2 cosmetic stale-name Nits swept). Cleared to implement.**
**Design lineage:** architect program design for "bulk reconcile for the OTHER decision types" (queue item
3), user-approved **safety-first** sequencing. This is **Cycle 2** (the extraction folded in as Task 1;
Cycle 1 per the roadmap). Roadmap: memory `bulk-reconcile-other-types-roadmap`.

**Goal.** (Task 1) Extract the safety-critical bulk-persist loop the two shipped bulk flows duplicate
byte-for-byte into ONE `persist_bulk_decisions` helper (zero behavior change). (Task 2) The first new bulk
flow: **bulk-resolve-conflict** — accept OR reject MANY flagged import conflicts in one confirmed batch.
Lowest-risk feature flow (no per-item input, no $), so it proves the generalized skeleton + settles the
non-revocable-batch ceremony early.

---

## SemVer / lockstep
- **btctax-core:** UNCHANGED — reuses existing `SupersedeImport`/`RejectImport` (`event.rs:192/196`). No
  new decision variant → no forward-only serde break.
- **btctax-cli:** MINOR/additive — `Session::bulk_resolve_conflict_plan` + `cmd::reconcile::
  {bulk_resolve_conflict_plan, apply_bulk_accept_conflicts, apply_bulk_reject_conflicts}` + a `Reconcile` clap variant.
- **btctax-tui-edit:** internal refactor (Task 1, zero behavior) + MINOR/additive (Task 2 `C` flow +
  `persist_bulk_resolve_conflict` — a thin wrapper over `persist_bulk_decisions`).
- **Lockstep: NONE** (no `docs/manual/`, no GUI crate).

---

## Grounding (verified at `719e9fe`)
- The two shipped bulk-persist fns to unify: `persist_bulk_link_transfer` (`edit/persist.rs:393`) and
  `persist_bulk_self_transfer_in` (`:442`) — identical EXCEPT the per-row payload; both = pre-snapshot
  empty guard → `snapshot` → loop `if let Err(e) = append_decision(...) { return Err(rollback(session,
  &pre, e.into())) }` → `save_or_rollback`. The **mid-batch rollback** is the safety-critical block.
- Single-item resolve-conflict: `persist_resolve_conflict(session, conflict_event, kind, now)`
  (`edit/persist.rs:525` — `kind` → `SupersedeImport{conflict_event}` / `RejectImport{conflict_event}`);
  CLI `accept_conflict` (`reconcile.rs:179`) / `reject_conflict` (`:195`); the `ImportConflict` blocker
  candidate set + the opener that joins it to `current`/`new` summaries; the **Tier-B non-revocable
  modal** `draw_resolve_conflict_modal` (`draw_edit.rs`, "This decision CANNOT be voided (non-revocable)").
- `SupersedeImport`/`RejectImport` are NOT in `is_revocable_payload` (`form.rs:896`) → non-revocable.
- The bulk two-phase CLI template: `bulk_link_plan` (`reconcile.rs:216`) + `apply_bulk_link_transfer`
  (`:231`); the TUI checklist flow + `bulk_usd_floor_label`/`Frame`. Free Browse key **`C`**.

---

## Task 1 — extract `persist_bulk_decisions` (edit/persist.rs; ZERO behavior change)

```rust
/// The shared bulk-persist loop: refuse empty, snapshot, append EACH payload, and on ANY append error
/// revert the WHOLE batch (append_decision commits per-call), then ONE save. The mid-batch rollback is
/// the safety-critical invariant that distinguishes the TUI atomic path from the CLI's drop-the-session.
pub fn persist_bulk_decisions(session: &mut Session, payloads: Vec<EventPayload>, now: OffsetDateTime,
    empty_label: &str) -> Result<usize, PersistError> {
    // [R0-M2] caller-supplied label preserves each shipped fn's EXACT "nothing selected" string → the
    // re-point is TRULY zero-behavior (bulk-link passes "bulk link: nothing selected", bulk-sti passes
    // "bulk classify-inbound-self-transfer: nothing selected"; bulk-resolve passes its own).
    if payloads.is_empty() { return Err(PersistError::NoChange(CliError::Usage(empty_label.into()))); }
    let n = payloads.len();
    let pre = session.snapshot()?;
    for payload in payloads {
        if let Err(e) = append_decision(session.conn(), payload, now, UtcOffset::UTC, None) {
            return Err(rollback(session, &pre, e.into()));
        }
    }
    save_or_rollback(session, pre)?;
    Ok(n)
}
```
**Re-point** `persist_bulk_link_transfer` + `persist_bulk_self_transfer_in` to build their `Vec<EventPayload>`
then delegate to `persist_bulk_decisions`, **each passing its existing empty-label string** [R0-M2] (so the
re-point is truly byte-for-byte). **Invariant: ZERO behavior change** — the shipped bulk-link +
bulk-sti KATs (strict-prefix, mid-batch-rollback, empty-refuse) MUST stay green unchanged (they are the
pins). No new KAT needed for Task 1 beyond keeping the existing ones green; optionally a direct
`persist_bulk_decisions` unit KAT. (Cycle 3/void will add a side-effect-hook variant for
`optimize_attest::clear`; NOT in this cycle.)

---

## Task 2 — bulk-resolve-conflict

### D1 — the plan (read-only) `Session::bulk_resolve_conflict_plan`
Mirror the shipped `*_plan` helpers. **Candidate set** = `snap.state.blockers` where `kind ==
BlockerKind::ImportConflict` (engine-post-filtered — fires only while UNRESOLVED, so no client-side
exclusion; an accepted/rejected conflict is no longer flagged → structural idempotence). Each blocker
carries the conflict event id; join to the event index to build, per row: `conflict_event`, `date`,
`target`, and the **STRUCTURED** current/new payloads + fingerprint — NOT pre-rendered summary strings
[R0-M1: `import_payload_summary` is a private TUI-binary fn unreachable from `btctax-cli`, and the sibling
`BulkStiRow`/`BulkLinkRow` carry structured data; each FRONT-END renders its own summary (CLI table
formatter; TUI reuses `import_payload_summary`)]. **No $ number** (a conflict resolution recognizes no
gain). **No time/wallet filter** (the conflict set is small; per-row exclude is the precision tool) —
optional frame filter is out of scope.
```rust
pub struct BulkResolveRow {
    pub conflict_event: EventId, pub date: TaxDate, pub target: EventId,
    pub current_payload: EventPayload,  // payload currently at the target (front-end renders "current")
    pub new_payload: EventPayload,      // ImportConflict.new_payload (front-end renders "→ new")
    pub new_fingerprint: String,        // [R0-N1] the 8-char disambiguator (front-end shows it)
}
pub struct BulkResolvePlan { pub rows: Vec<BulkResolveRow> }
pub fn bulk_resolve_conflict_plan(&self) -> Result<BulkResolvePlan, CliError>;
```

### D2 — CLI `bulk-resolve-conflict`
Two-phase (mirror bulk-link): `bulk_resolve_conflict_plan` (read/render the `current → new` table) +
**TWO apply fns** `apply_bulk_accept_conflicts(vault, pp, conflict_events, now)` /
`apply_bulk_reject_conflicts(...)` [R0-I1 — do NOT reference `ResolveKind`; it lives ONLY in
`btctax-tui-edit` and is unreachable from `btctax-cli` (dependency direction is tui-edit→cli). This mirrors
the shipped single-item split `accept_conflict`/`reject_conflict`, `reconcile.rs:179/195`]. Each appends one
`SupersedeImport`/`RejectImport` per event (one save). Dispatch selects the fn from the clap
`--accept`/`--reject` bool. Clap:
```
reconcile bulk-resolve-conflict (--accept | --reject) [--dry-run] [--yes]
```
`--accept`/`--reject` is REQUIRED (a batch-wide accept/reject bool — NO `ResolveKind` in the CLI [R0-r2];
mutually exclusive). Empty candidate set
→ "no unresolved import conflicts" + exit 0. `--dry-run` → preview + stop. `--yes`|interactive `y/N`.
Print "accepted/rejected N import conflicts".

### D3 — TUI `C` flow
New Browse key **`C`**. `open_bulk_resolve_conflict_flow` (latch → snapshot → `ImportConflict` set
non-empty else status "No unresolved import conflicts to bulk-resolve"). Steps:
1. **Accept/Reject toggle** — a batch-wide `ResolveKind` choice (`←/→` or a two-item pick; the shipped
   `ResolveKind`, `form.rs`). This is the ONLY "param". Enter → preview.
2. **Preview checklist (per-row exclude)** — a `TargetList<BulkResolveRow>` all-checked; `Space`/`x`
   toggles; each row `date · target · current → new`. Footer: **checked count** + the chosen action
   (Accept/Reject). Enter → confirm modal.
3. **Confirm modal — Tier B (non-revocable), NOT typed-word.** Reuse the shipped non-revocable warning
   framing ("These decisions CANNOT be voided (non-revocable)"), pluralized, + the checked count + action.
   Explicit Enter/Esc. Enter → `persist_bulk_resolve_conflict`.

**Confirm-strength decision [ARCHITECT/roadmap]:** Tier-B non-revocable warning, NOT the `ATTEST`
typed-word — typed-word is reserved for the UNRECOVERABLE §7.4 batch; a wrong bulk accept/reject is
non-revocable as a *decision* but the OUTCOME is recoverable out-of-band (re-import / `ManualFmv` /
`ClassifyRaw`). Tier-B + per-row preview + count is proportionate (stronger than bulk-link's plain modal,
weaker than typed-word).

**`persist_bulk_resolve_conflict` (edit/persist.rs — thin wrapper over `persist_bulk_decisions`):**
```rust
pub fn persist_bulk_resolve_conflict(session: &mut Session, conflict_events: Vec<EventId>,
    kind: ResolveKind, now: OffsetDateTime) -> Result<usize, PersistError> {
    let payloads = conflict_events.into_iter().map(|conflict_event| match kind {
        ResolveKind::Accept => EventPayload::SupersedeImport(SupersedeImport { conflict_event }),
        ResolveKind::Reject => EventPayload::RejectImport(RejectImport { conflict_event }),
    }).collect();
    persist_bulk_decisions(session, payloads, now, "bulk resolve-conflict: nothing selected")
}
```
Empty selection (all unchecked) → refuse (the shared helper's empty guard). Post-apply status
(`derive_bulk_resolve_status`, re-project): `"{Accepted/Rejected} {N} import conflict(s); {remaining}
unresolved remain."`

---

## Gotchas (for the reviewer)
- **G1 (Task 1 zero-behavior):** the extraction MUST preserve the pre-snapshot empty guard + the mid-batch
  `rollback` + the single `save_or_rollback` EXACTLY; the shipped bulk-link/bulk-sti KATs are the pins and
  must stay green with no edit. Do NOT change the CLI atomicity path (it drops the session on error;
  `persist_bulk_decisions` is the TUI path only).
- **G2 (non-revocable):** `SupersedeImport`/`RejectImport` are NOT in `is_revocable_payload` — do NOT add
  them; the confirm is Tier-B (non-revocable warning), NOT typed-word, NOT the "each individually voidable"
  reassurance (a wrong accept/reject can't be voided — the modal must say so).
- **G3 (structural idempotence):** candidates = live `ImportConflict` blockers only → a resolved conflict
  is no longer flagged → re-running never double-resolves; excluded rows not appended.
- **G4 (both-side summaries):** the preview resolves `current` from the TARGET's payload and `new` from
  `ImportConflict.new_payload` (target ≠ conflict event) — accept adopts `new_payload`, reject keeps
  `current`.
- **G5:** the mid-batch rollback (via the shared helper) + empty guards at every gate.

## KATs
- **Task 1:** the shipped bulk-link + bulk-sti KATs stay green UNCHANGED after the re-point (the pins);
  optionally `persist_bulk_decisions_reverts_mid_batch` direct unit KAT.
- **btctax-cli:** `bulk_resolve_plan_lists_unresolved_conflicts` (only live `ImportConflict`s; resolved
  ones excluded); `bulk_resolve_cli_accept_adopts_new` / `bulk_resolve_cli_reject_keeps_current` (E2E: the
  targets' blockers clear; accept adopts `new_payload`, reject keeps current); `bulk_resolve_cli_dry_run_
  writes_nothing`; `bulk_resolve_cli_requires_accept_xor_reject`.
- **edit/persist.rs:** `persist_bulk_resolve_strict_prefix` (N SupersedeImport|RejectImport tail-appended);
  `persist_bulk_resolve_reverts_mid_batch` (via the shared helper); `persist_bulk_resolve_refuses_empty`;
  **non-revocable KAT** (a bulk-accepted conflict is NOT in `is_revocable_payload` / a void of it →
  `DecisionConflict`).
- **main.rs:** `bulk_resolve_refuses_when_no_conflicts`; `bulk_resolve_per_row_exclude_drops_row`;
  `bulk_resolve_accept_reject_toggle`; E2E `bulk_resolve_then_conflicts_cleared` (`C` → toggle → exclude
  one → confirm → the included targets' `ImportConflict` blockers clear, the excluded one stays). **KAT-G1**
  stays green.

## Plan (TDD, phased — each: KATs red → implement green → review to 0C/0I)
- **Task 1 — extract `persist_bulk_decisions`** + re-point the 2 shipped fns (zero behavior; existing KATs
  stay green). Land FIRST (the later cycles build on it).
- **Task 2 — bulk-resolve-conflict** (`Session::bulk_resolve_conflict_plan`; two-phase CLI + clap variant;
  the TUI `C` flow + `persist_bulk_resolve_conflict`; the KATs above).
- **Task 3 — whole-diff review (Phase E) + FOLLOWUPS** (record the remaining cycles: void → inbound-income
  → outflow-reclassify).

## Out of scope
- The `optimize_attest::clear` side-effect-hook variant of `persist_bulk_decisions` (Cycle 3 / bulk-void).
- Per-row accept/reject (v1 is a batch-wide toggle; per-row exclude covers the "resolve these differently"
  case by excluding + single-item `i`).
- A time/wallet filter on the conflict set (small set; per-row exclude suffices).
- The other three bulk flows (void, inbound-income, outflow-reclassify) — later cycles.
- **Key-space note:** b/B/C now bind bulk flows; once all 4 land, a bulk SUB-MENU may be the right home —
  flagged for a future cycle, NOT this one.
