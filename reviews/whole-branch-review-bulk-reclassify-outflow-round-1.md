# Whole-diff review (Phase E) — feat/bulk-reclassify-outflow (Cycle 5, the LAST) — round 1

**Verdict: 0 Critical / 1 Important (FOLDED) / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review (reviewer ≠ author). Diff `b110523..HEAD` (Task 1 `050b077` CLI engine + side-table;
Task 2 `c9484b8` TUI + display + clear-on-void) + this review's fold. Contract:
`design/SPEC_bulk_reclassify_outflow.md` (R0-GREEN, 2 rounds). 19 files. btctax-core UNCHANGED.

## The Important finding — FOUND + FOLDED
### [I1-CLI] IMPORTANT — clear-on-void was applied to the TUI paths ONLY, not the CLI void paths
The implementer flagged it: the R0-I1 clear-on-void fix was wired into `persist_void` + `persist_bulk_void`
(tui-edit) but NOT into the CLI `cmd::reconcile::void` (single) or `apply_bulk_void` (bulk). Both CLI void
paths already clear `optimize_attest` for `LotSelection` targets (reconcile.rs:144, 546) — they have the
mechanism — but were not extended for `ReclassifyOutflow` targets. So `btctax void <a-bulk-reclassify>`
would leave a stale `bulk_estimated[X]` row → the EXACT stale-`[est]` gap I1 closed, reopened via the CLI.
Same invariant, incompletely applied. **FOLDED (this review):** added the parallel
`ReclassifyOutflow → bulk_estimated::clear(transfer_out_event)` arm to BOTH `void` (mirrors its LotSelection
arm) and `apply_bulk_void` (via a `reclass_map` from `load_all`), inside the same batch, idempotent. New KAT
`bulk_reclassify_outflow_cli_void_clears_estimated_flag` (CLI bulk-reclassify → CLI void → flag gone).
**[★ fault-inject]** removing the single-`void` clear drove the KAT RED. Restored.

## Verification + fault-injection (all probes restored byte-for-byte)

**1. [★ #a — the whole cycle] the `fmv_of==None` exclusion — CONFIRMED.** `bulk_reclassify_outflow_plan`
(session.rs) `None => { excluded_missing_price += 1; continue; }`; `included` carries a RESOLVED `fmv: Usd`.
**Fault-inject:** rewrote the `None` arm to fabricate `$0` (include) → `bulk_reclassify_outflow_plan_excludes_missing_price`
RED. The silent-fabricated-proceeds vector is closed at the plan; the CLI apply's `let Some(fmv) else continue`
is the defense-in-depth twin. Restored.

**2. [★ side-table join key] `[est]` lands on the right disposal — CONFIRMED.** `bulk_estimated::mark` is
called with `out_event` = the `transfer_out_event` (reconcile.rs:449, persist.rs:674), which IS the eventual
`Disposal.event` (fold.rs:633 pushes `Disposal{event: <original TransferOut id>}`). R0 verified the id
provenance; the impl uses the correct key. Native exchange Dispose sells carry distinct import ids → no
false-flag collision.

**3. [★ estimated-gain, not double-counted] — CONFIRMED.** `estimated_gain = round_cents(fmv − Σ pt.legs.usd_basis)`
reusing the fold-computed `PendingTransfer.legs` (one chronological `consume_fifo` pass → sequential draw-down,
no double-count; R0-verified against `pools.rs` drain). Pinned by `..estimated_gain_matches_pending_legs_basis`
(2 lots, different bases) and `..batch_gain_not_double_counted` (row2's A-leg = A's remainder after row1).

**4. [★ clear-on-void, TUI] — CONFIRMED.** `persist_void` + `persist_bulk_void` clear `bulk_estimated` for a
`ReclassifyOutflow` target, in-envelope, guarded (persist.rs:296, 605). **Fault-inject:** neutering
`persist_void`'s `ReclassifyOutflow` detection drove `kat_bulk_reclassify_void_clears_estimated_flag` RED. Restored.

**5. side-table stores ONLY the flag** (mirror `donation_details`); the Disposals tab renders EXACT fold
figures + an `[est]` marker (disposals.rs), Compliance shows an advisory count. Loaded via the typed
`Session::bulk_estimated()` accessor in `build_snapshot`. No stored number overrides the exact figures.

**6. Persist atomicity + CLI own-loop** — TUI `persist_bulk_reclassify_outflow` is bespoke (mirror
`persist_bulk_void`: guarded append + `mark` in-envelope + single `save_or_rollback`); CLI
`apply_bulk_reclassify_outflow` is an own-loop (no tui-edit helper). `_side_table_reverts_on_mid_batch_failure`
+ `_cli_mid_batch_failure_writes_nothing` pin both.

**7. Scope / SemVer** — `Dispose{Sell,Spend}` only; `--kind` rejects gift/donate (`_scope_excludes_gift_and_donate`).
btctax-core UNCHANGED (side-table + tui display only); MINOR, same class as `donation_details`. Shipped
single-void/reclassify + bulk-void KATs stayed green through the clear-on-void extension.

## Full suite
`cargo test --workspace --locked` + `clippy -D warnings` + `fmt --check` — ship gate (see merge).

**SHIP.**
