# Whole-diff review (Phase E) — feat/bulk-resolve-conflict (Cycle 2) — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review (reviewer ≠ author; the implementer subagent authored the branch). Diff base
`719e9fe`; commits `1202f6d` (spec, R0-GREEN 2 rounds) → `6f9226e` (Task 1) → `b39fb89` (Task 2). Code
diff = `btctax-cli` (session.rs, cmd/reconcile.rs, lib.rs, main.rs, tests/reconcile.rs) + `btctax-tui-edit`
(draw_edit.rs, edit/form.rs, edit/persist.rs, editor.rs, main.rs). No `btctax-core` behavior change.

## Verification + fault-injection (both probes restored the tree byte-for-byte)

**1. Task 1 — `persist_bulk_decisions` extraction (safety-critical) — CORRECT.**
`persist.rs:394` — empty-guard BEFORE any snapshot/append (`Err(NoChange(Usage(empty_label)))`), then
`pre = snapshot()`, then per-payload `append_decision` guarded by
`if let Err(e) = … { return Err(rollback(session, &pre, e.into())) }` (explicitly NOT a bare `?`, with the
reasoning in the comment: a bare `?` at row k>1 leaks appends 1..k-1 as live residue under a bare
NoChange), then ONE `save_or_rollback`. The two re-pointed callers (`persist_bulk_link_transfer`,
`persist_bulk_self_transfer_in`) each build their `Vec<EventPayload>` and pass their EXACT original
empty-label string → zero-behavior re-point.
- **[★ fault-inject]** Replaced the mid-batch `if let Err … rollback` with a bare `?`:
  **3 KATs RED** — `kat_persist_bulk_decisions_reverts_mid_batch` (the shared helper) AND both re-pointed
  callers `kat_persist_bulk_link_reverts_mid_batch_append_failure` /
  `kat_persist_bulk_sti_reverts_mid_batch_append_failure`. The other rollback-on-failed-save KATs stayed
  green (different path). Proves the extraction is load-bearing AND the re-point preserved the invariant
  for both callers. Restored.

**2. Task 2 — bulk-resolve-conflict accept/reject (tax-safety) — CORRECT.**
CLI has TWO apply fns: `apply_bulk_accept_conflicts` → `SupersedeImport { conflict_event }` (adopt the new
import), `apply_bulk_reject_conflicts` → `RejectImport { conflict_event }` (keep current). CLI atomicity:
bare `?` mid-batch returns BEFORE `save` → in-memory session discarded, nothing lands on disk (comment
correctly distinguishes this from the TUI's `persist_bulk_decisions` rollback path).
- **[★ fault-inject]** Wired `apply_bulk_accept_conflicts` to append `RejectImport` instead of
  `SupersedeImport`: `bulk_resolve_cli_accept_adopts_new` went **RED** (panic reconcile.rs:694) while
  `bulk_resolve_cli_reject_keeps_current` stayed green — the accept/reject distinction is load-bearing.
  Restored.

**3. Spec adherence (structural).**
- **NO `ResolveKind` in `btctax-cli`** (grep: only two explanatory comments referencing the R0-I1
  decision) — the dependency-cycle regression that R0 round 1 caught cannot recur.
- Two apply fns behind a clap ArgGroup (`bulk_resolve_cli_requires_accept_xor_reject` pins the xor).
- Candidate set = live `ImportConflict` blockers; reuses `SupersedeImport`/`RejectImport`; NOT added to
  `is_revocable_payload`. TUI: accept/reject toggle → per-row-exclude checklist → Tier-B non-revocable
  confirm (not a typed-word). KATs present: CLI `bulk_resolve_plan_lists_unresolved_conflicts`,
  `..accept_adopts_new`, `..reject_keeps_current`, `..dry_run_writes_nothing`, `..requires_accept_xor_reject`;
  TUI `bulk_resolve_refuses_when_no_conflicts`, `..accept_reject_toggle`, `..per_row_exclude_drops_row`, E2E.

**4. No over-reach / no regression** — diff is cli + tui-edit only; no core change; the 7 shipped bulk pins
(strict-prefix / rollback / empty-refuse) stayed green through the Task-1 re-point.

## Full suite
`cargo test --workspace --locked` + `clippy -D warnings` + `fmt --check` — see ship gate (run at merge time).

**SHIP.**
