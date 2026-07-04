# Whole-diff review (Phase E) — feat/bulk-void (Cycle 3) — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review (reviewer ≠ author; the implementer subagent authored the branch). Diff
`822af7f..5ff7efd` (Task 1 `c776767` extract; Task 2 `5ff7efd` bulk-void). Contract:
`design/SPEC_bulk_void.md` (R0-GREEN, 2 rounds). 14 files: btctax-core (void.rs, lib.rs, tests/voidable.rs),
btctax-cli (session.rs, cmd/reconcile.rs, main.rs, lib.rs, tests/{reconcile,optimize_accept}.rs),
btctax-tui-edit (main.rs, draw_edit.rs, edit/{form,persist}.rs, editor.rs).

## Verification + fault-injection (all probes restored the tree byte-for-byte)

**1. Task 1 — predicate extraction (zero-behavior) — CORRECT.**
`btctax-core/src/void.rs`: `voidable_decisions(events, blockers)` is byte-equivalent to the shipped inline
`open_void_flow` filter — `EventId::Decision` ∧ not-voided ∧ `is_revocable_payload` ∧ `!effective_alloc`,
with `effective_alloc = SafeHarborAllocation ∧ ¬SafeHarborTimebar ∧ ¬SafeHarborUnconservable` blocker on
its id. `is_revocable_payload` moved to core (pure `EventPayload` match); `open_void_flow` re-pointed. No
second copy of the predicate remains. The stale `resolve.rs:865-921` cite in `main.rs:2749` was corrected.
The shipped single-void KATs stayed GREEN unchanged (implementer-verified on the isolated Task-1 state;
re-confirmed by the full suite here).

**2. [★ #7 tax-safety — the whole ballgame] fault-inject CONFIRMED.** Removing the `!effective_alloc`
filter from `voidable_decisions` drove BOTH `bulk_void_excludes_effective_allocation` (core predicate,
tests/voidable.rs) AND `bulk_void_plan_omits_effective_allocation` (CLI plan, tests/reconcile.rs) RED,
while `..includes_inert_allocation` stayed green. The exclusion is load-bearing at BOTH the predicate and
the CLI-plan layer — voiding an effective allocation (→ Hard `DecisionConflict`, a year-gating damaging
no-op) is impossible through either path. Restored.

**3. CLI-layer #7 defense — CORRECT.** `Reconcile::BulkVoid` (main.rs:1294) derives `targets` from
`bulk_void_plan().rows` (line 1325), NEVER raw `--ref` ids (the `BulkVoid` subcommand exposes only
`--dry-run`/`--yes`), with an explicit "effective allocation omitted by the plan can never reach
apply_bulk_void" comment. Matches the bulk-resolve dispatch pattern.

**4. Persist atomicity + side-effect — CORRECT.** `persist_bulk_void` (edit/persist.rs:529): empty-guard
before any mutation → snapshot → per-item append `VoidDecisionEvent` guarded by
`return Err(rollback(session, &pre, e.into()))` (NOT bare `?`) → per-`LotSelection`
`optimize_attest::clear` guarded the same way → single `save_or_rollback`; lockstep comment cross-refs
`persist_bulk_decisions`. The CLI `apply_bulk_void` mirrors it (bare `?`-before-`save` = in-memory
discard).
- **[★ fault-inject] save-failure rollback CONFIRMED** — bypassing `save_or_rollback` (bare `session.save()`)
  drove `kat_bulk_void_reverts_mid_batch` (which injects a real save failure via chmod `0o500`) RED. The
  whole batch (void rows + side-table clears) reverts on save failure. The mid-batch append/clear rollback
  arms are defensive-airtight (unreachable in practice — same class as shipped `persist_void`'s clear arm).
- **[★ fault-inject] attestation clear CONFIRMED** — removing the CLI `apply_bulk_void`
  `optimize_attest::clear` drove `bulk_void_clears_attestation_for_lotselection` (optimize_accept.rs) RED.
  Voiding a `LotSelection` clears its optimizer attestation atomically.

**5. TUI `V` flow + KAT coverage.** `V` (Shift-v) → `open_bulk_void_flow` → `TargetList` per-row-exclude
checklist (`summarize_void_payload`) → Tier-B blast-radius modal (red border, NOT typed-word; states N
voids cannot be undone + LotSelection re-expose count). Pinned: `bulk_void_refuses_when_no_candidates`,
`bulk_void_per_row_exclude_drops_row`, `bulk_void_tier_b_not_typed_word`, E2E `bulk_void_then_blockers_reexposed`,
`bulk_void_cli_reexposes_inbound_blockers`, `bulk_void_dry_run_writes_nothing`, `kat_bulk_void_empty_refuses`.

**6. No over-reach / no regression** — core change is relocation + one pure fn (no new `EventPayload`
variant, no serde break); additive cli + tui-edit; no `btctax-core` behavior change.

## Full suite
`cargo test --workspace --locked` + `clippy -D warnings` + `fmt --check` — ship gate (see merge).

**SHIP.**
