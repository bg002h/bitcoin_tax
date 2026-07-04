# R0 — SPEC_bulk_reclassify_outflow (round 2)

**Artifact:** `design/SPEC_bulk_reclassify_outflow.md` (Cycle 5, the LAST).
**Branch/base:** `feat/bulk-reclassify-outflow` @ `27e6170`; main == `a241705`.
**Reviewer role:** independent architect, read-only round-2 verification of the folded spec vs CURRENT source.
**Bar:** 0 Critical / 0 Important. **Round-1:** 0C / 1I / 3M / 2N (BLOCKED on I1).

## Verdict: **0 Critical / 0 Important / 0 Minor / 0 Nit — R0-GREEN**

Every round-1 finding resolved against current source. No new drift: the whole branch (`a241705..27e6170`)
touches ONLY the spec + the round-1 review file — zero source files changed.

## I1 (blocker) — clear-on-void — RESOLVED
`bulk_estimated::clear(transfer_out_event)` wired into BOTH `persist_void` and `persist_bulk_void`, keyed by
the voided `ReclassifyOutflow`'s `transfer_out_event`, in-envelope, idempotent, KAT-pinned. All four sub-claims
confirmed:
- (a) `persist_void` clears `optimize_attest` for a `LotSelection` target — the mirror pattern (persist.rs:262–289,
  clear at :284, guarded `return Err(rollback(...))`). `ReclassifyOutflow` is an `EventPayload` variant
  (event.rs:300) carrying `transfer_out_event` (event.rs:116) → the mirror arm is structurally identical.
- (b) A voided `ReclassifyOutflow` returns the outflow to `PendingOut`: pass-1e skips voided decisions
  (resolve.rs:515–517) so it never enters `outflow_class`; `build_op` falls through to `Op::PendingOut`
  (resolve.rs:262–265). The stale-`[est]` orphan scenario is real; the clear is needed.
- (c) `persist_bulk_void` (persist.rs:552–593) is the bespoke bulk analog to extend (lockstep-commented,
  blast-radius-isolated) — the right home for the `ReclassifyOutflow` clear arm.
- (d) Idempotent clear (persist.rs:551 "pure idempotent DELETE") → single-`o` unflagged reclassifies unaffected;
  pinned by the single-`o` control assertion.
KAT `bulk_reclassify_outflow_void_clears_estimated_flag` covers both clear arms + the control.

## M1 — typed `Session::bulk_estimated()` accessor — RESOLVED
Mirrors `donation_details()` (session.rs:369–373); `build_snapshot` loads via it, never `conn()` (unlock.rs:168,177);
`init_table` in `from_fresh_vault` (session.rs:299–301).

## M2 — CLI mid-batch-failure KAT — RESOLVED
`bulk_reclassify_outflow_cli_mid_batch_failure_writes_nothing` present (no appends AND no side-table rows).

## M3 — side-table stores only flag+date; exact numbers rendered — RESOLVED
Disposals tab renders exact `leg.proceeds/basis/gain` (disposals.rs:40–42,49–51) + `[est]` marker; no numbers
stored → nothing can override the exact figures. The round-1 "optionally store the snapshot" is removed.

## N1 — plan row `wallet: Option<WalletId>` — RESOLVED (mirror `BulkLinkRow.source_wallet`, session.rs:47–49).
## N2 — struct cites tightened — RESOLVED (PendingLeg 197–203, PendingTransfer 204–210, exact).

## Spot-checks (no regression)
- **#1 join key**: `fold.rs:633–634` pushes `Disposal{event: eff.id.clone()}` = the original TransferOut id =
  `ReclassifyOutflow.transfer_out_event`; native Dispose sells carry distinct import ids → no collision.
- **#2 gain not double-counted**: `Op::PendingOut` runs `consume_fifo` (fold.rs:712) storing exactly-consumed
  legs (fold.rs:720–734); `consume_fifo→consume_ordered→take_from` DRAINS the pool (pools.rs:227,231,173/200) in
  one chronological pass → `Σ legs.usd_basis` cannot double-count. Precedent `bulk_link_transfer_plan` (session.rs:510).
- **#a**: `principal_proceeds_or_fmv: Usd` non-Option (event.rs:118); plan `fmv: Usd` + apply `let Some(fmv) else continue` are the twin defenses.
- **Bespoke persist atomicity**, **Sell/Spend-only scope**, **single-`o`-not-flagged asymmetry** — all confirmed.

**R0-GREEN.** Clear of the 0C/0I bar; may proceed to Plan.
