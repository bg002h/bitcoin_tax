# SPEC — bulk-reclassify-outflow (queue item 3, Cycle 5 — the LAST)

**Source baseline:** `main` @ `a241705` (branch `feat/bulk-reclassify-outflow`). **Review status: DRAFT —
awaiting R0 (2-round independent architect loop to 0C/0I before implementation).**
**Lineage:** final cycle of `bulk-reconcile-other-types` (architect-designed 2026-07-03, transcript;
Cycles 1-4 SHIPPED). **User decisions (2026-07-03):** outflow→Sell with FMV as ESTIMATED proceeds
(approved earlier); the estimate **must be flagged PERSISTENTLY and shown on the Disposals tab**
(→ design **A+**, below).

## The feature
The bulk analog of the single `o` reclassify-outflow. Sweep MANY pending outflows
(`state.pending_reconciliation`) → a **`Dispose`** with **auto-FMV as ESTIMATED proceeds**, in one
filtered, per-row-excludable, confirmed, atomic batch. Mirrors the shipped bulk-income/bulk-sti shape.
New TUI key **`O`** (Shift-o; free — pairs with single `o`). New CLI `reconcile bulk-reclassify-outflow`.

## Scope — `Dispose{Sell, Spend}` ONLY [Q2 resolved]
`OutflowClass = Dispose{DisposeKind: Sell|Spend} | GiftOut | Donate{appraisal_required}` (event.rs:109).
This cycle does **Dispose{Sell, Spend} only**; **GiftOut and Donate are DEFERRED**:
- `GiftOut`/`Donate` need a `donee` (event.rs:120-124) — a UNIFORM donee across many real recipients in one
  batch is semantically wrong.
- `Donate.principal_proceeds_or_fmv` is the §170 charitable-deduction FMV, which for Section B (>$5,000,
  forms.rs:356-376) must come from a **qualified appraisal**, NOT a bulk daily-close price — auto-substituting
  market FMV could manufacture an unsupportable deduction. Stronger reason to exclude than UX.
- Sell vs Spend is the SAME `Op::Dispose` shape (resolve.rs:253-260), differing only in the reported
  `DisposeKind` tag — so both are free; the user picks ONE for the batch.

**Why this feature exists (user rationale, 2026-07-03) — Spend is the PRIMARY driver, not Sell.** Many
outflows are NOT self-transfers and NOT exchange sells — they are **purchases of goods/services** for which
**no "price" exists at all**. Spending BTC on a good/service is a taxable disposition whose proceeds are the
**FMV of the BTC that left** (= the measure of the good/service's value). So for a `Spend`, FMV-as-proceeds
is not a fallback estimate of an unknown sale price — it is the **correct and only** valuation method. This
is exactly why `Dispose{Spend}` is in scope and why FMV drives proceeds. The persisted "estimated" flag
remains honest for both kinds: the daily-close FMV is still an APPROXIMATION of the exact-moment value (and
for a Sell, additionally a proxy for the unrecorded real sale price) — so the user can later refine it.

## Candidate set — `pending_reconciliation` − missing-price [#a]
`state.pending_reconciliation` (already excludes already-reclassified/linked — `build_op`, resolve.rs:236-266,
resolves a reclassified `TransferOut` to `Op::Dispose`/… so it never re-enters `pending_reconciliation`; and
wallet-less outflows never enter it — fold.rs:699-709 returns before pushing). Then the **Cycle-5 exclusion**:
- **MINUS every row where `fmv_of(&prices, date, principal_sat) == None`** — reported as
  `excluded_missing_price`, NOT silently dropped.

**#a is MORE structurally forced here than in bulk-income.** `ReclassifyOutflow.principal_proceeds_or_fmv`
is **`Usd`, NOT `Option<Usd>`** (event.rs:118) — you cannot even construct a payload for a missing-price row
without FABRICATING a number, and (unlike bulk-income's LOUD Hard `FmvMissing`) a fabricated/`0` proceeds
would be **SILENT** — it gates nothing and misreports gain/loss. So the plan's row type carrying a RESOLVED
`fmv: Usd` (non-Option, resolved before construction) is the load-bearing structural defense (mirrors
`BulkIncomeRow.fmv: Usd`).

## Estimated proceeds + gain [Q3 resolved — reuse the fold-computed leg basis]
- **Proceeds** = `fmv = fmv_of(&prices, date, pt.principal_sat)` (the market value that day — ESTIMATED).
- **Basis** = `Σ pt.legs.usd_basis` — **already computed by the fold**. `Op::PendingOut` (fold.rs:698-734)
  runs `consume_fifo` in the fold's SINGLE real chronological pass and stores the consumed `legs`
  (`lot_id, sat, usd_basis, acquired_at`) on each `PendingTransfer` (state.rs:198-210). Because it is ONE
  pass over the whole ledger with all N candidate `PendingOut`s pending, **`Σ` over multiple entries'
  legs is NEVER double-counted** — an earlier-dated `PendingOut` has already drawn the pool down before a
  later one folds. This resolves the ordering hazard with **no re-fold**. Precedent: `bulk_link_transfer_plan`
  already sums `pt.legs.usd_basis` (session.rs:510).
- **`estimated_gain = round_cents(fmv − Σ pt.legs.usd_basis)`** — per row; the preview shows total
  ESTIMATED **GAIN**, not just Σ FMV.
- **[R0: do NOT use `evaluate.rs::CandidateDisposal`]** — it either returns `UnknownExistingDisposal` for a
  still-`Op::PendingOut` candidate, or (synthetic path) tail-injects the candidate + runs N independent
  whole-ledger folds that don't see each other → reintroduces double-counting. The fold-computed legs are
  the correct, cheap source.
- **Disclosed residual imprecision (NON-blocking; the PERSISTED numbers are always exact — only the
  PREVIEW estimate carries this):** `Op::PendingOut` consumes FIFO always (fold.rs:712); a real post-reclass
  `Op::Dispose` uses `applicable_method` (FIFO unless a HIFO/LIFO election / non-FIFO `pre2025_method` is in
  force). Identical property to the shipped `bulk_link_transfer_plan` `basis_usd` preview (R0-accepted). And
  under the user's default `TreatmentC` fee handling the fee-sat basis re-homes identically; only non-default
  `TreatmentB` diverges by a bounded, tiny amount. Label the preview "ESTIMATED".

## The estimated flag — design A+ (btctax-cli side-table) [Q1, user-chosen: persist + show on Disposals]
No core marker exists (`ReclassifyOutflow`/`FmvStatus`/`DisposalLeg` carry no estimate/FMV-provenance —
event.rs:114-125/8-14, state.rs:123-150, confirmed forms.rs:290,347). Rather than a core-schema change
(threading a field through `Op::Dispose`/`Disposal`/`DisposalLeg`/forms/CSV — 4-6 files, forward-only serde),
use the established **`btctax-cli`-only side-table** pattern (`donation_details.rs`, `optimize_attest.rs`):
- **New `crates/btctax-cli/src/bulk_estimated.rs`** (~90 lines, mirror `donation_details.rs`): idempotent
  `CREATE TABLE IF NOT EXISTS bulk_estimated_proceeds`, keyed by `EventId::canonical()` of the
  **`transfer_out_event`** — which IS the eventual `Disposal.event` (fold.rs:633 pushes
  `Disposal{event: eff.id.clone()}` using the ORIGINAL TransferOut id, not the decision's id). Stores the
  estimate provenance (date marked; optionally the estimated proceeds/gain snapshot for display).
  `init_table` called in `Session::from_fresh_vault` (session.rs:296-304); loaded into `Snapshot` (new field)
  like `donation_details` (app.rs:104-111, unlock.rs:170).
- **Display (user-mandated "shown on Disposals"):** the Disposals tab (disposals.rs:39-54) joins the
  side-table against `Disposal.event` and renders an **`[est]`** marker on flagged rows (+ a legend note).
  The Compliance tab adds a small **advisory count** ("N disposals use estimated FMV proceeds") — closes the
  loop so the estimate is visible at tax-review time.
- **btctax-core stays UNCHANGED** (the flag lives outside the append-only event log entirely; no serde risk).

## Persist — bespoke (the side-table write is a per-row side-effect in the atomic envelope)
Because the side-table `mark` must land in the SAME atomic envelope as the `ReclassifyOutflow` appends (a
mid-batch failure must not leave a decision without its flag, or vice versa), the persist is **bespoke**
(like `persist_bulk_void`, NOT a thin `persist_bulk_decisions` delegate):
- **TUI `persist_bulk_reclassify_outflow`** (edit/persist.rs) — lockstep-commented against
  `persist_bulk_decisions`: empty-guard → `pre = snapshot()` → per row: `append_decision(ReclassifyOutflow…)`
  guarded (`return Err(rollback(session,&pre,e))`, NOT `?`), then `bulk_estimated::mark(conn, out_event, …)`
  guarded the same way → single `save_or_rollback`. Whole-DB restore covers the side-table for free on any
  failure (documented invariant, persist.rs:536-551, same as `optimize_attestation`).
- **CLI `apply_bulk_reclassify_outflow(vault, pp, out_events, kind: DisposeKind, now)`** — its OWN
  append-loop (mirror `apply_bulk_classify_inbound_income`, reconcile.rs:321-367; the CLI CANNOT call the
  tui-edit `persist_bulk_decisions` — dependency cycle, R0-I1 of Cycle 4): per row look up the TransferOut,
  resolve `date`, **`let Some(fmv) = fmv_of(&prices, date, t.sat) else { continue };`** (#a defense-in-depth —
  re-derive, never trust a threaded number), `append_decision(ReclassifyOutflow{ transfer_out_event,
  as_: Dispose{kind}, principal_proceeds_or_fmv: fmv, fee_usd: None, donee: None })`, then
  `bulk_estimated::mark(conn, out_event, …)`, bare `?`-before the single trailing `session.save()`.
- **Uniform params:** `kind: DisposeKind` (Sell/Spend, chosen once); `fee_usd: None` always (the on-chain
  `fee_sat` still flows via resolve.rs:257 `fee_sat: t.fee_sat` regardless — a uniform USD disposition-fee
  across heterogeneous txns is meaningless); `donee: None` (Dispose has none).

## Plan struct (session.rs — mirror BulkIncomePlan)
```
BulkReclassifyOutflowRow { out_event, date, wallet (always Some), principal_sat, fmv: Usd (resolved),
                           basis_usd: Usd, estimated_gain: Usd }
BulkReclassifyOutflowPlan { included: Vec<Row> (sorted by date), excluded_missing_price: usize,
                            total_sat, total_proceeds_usd (Σ fmv), total_basis_usd,
                            total_estimated_gain (Σ estimated_gain) }
Session::bulk_reclassify_outflow_plan(filter: BulkFilter) -> BulkReclassifyOutflowPlan   // REUSE BulkFilter
```
Reuse the existing `BulkFilter { frame, from_wallet }` (session.rs:37-40 — its doc already says "pending
outbound transfers"). Enrich over `pending_reconciliation` exactly as `bulk_link_transfer_plan`
(session.rs:486-562) does.

## CLI + TUI
- CLI `reconcile bulk-reclassify-outflow --kind <sell|spend> [--year/--from/--to] [--wallet <src>]
  (--dry-run xor --yes)`; `--kind` parser rejects `gift`/`donate` (structural scope-lock). Dispatch derives
  `out_events` from `plan.included` (never raw `--ref`). Confirm output prints "ESTIMATED" adjacent to both
  the proceeds and gain totals + the `excluded_missing_price` note.
- TUI `O` → Filter (kind toggle + source-wallet + frame) → `TargetList` per-row-exclude checklist
  (date / BTC / est.proceeds / est.basis / **est.gain**) → confirm modal: **bold "ESTIMATED proceeds" /
  "ESTIMATED gain"** lines (mirror `draw_bulk_income_modal` FMV-warning styling) + excluded-missing-price
  note; **revocable tier** (Tier-A + prominent warning; `ReclassifyOutflow` is voidable → NOT Tier-B/typed).

## Core / SemVer
- **btctax-core: UNCHANGED** (reuses `ReclassifyOutflow`/`OutflowClass::Dispose`/`fmv_of`/`pending_reconciliation`;
  no new variant/field, no serde break, no behavior change).
- Additive: `btctax-cli` side-table (new file + `init_table`), `btctax-tui` `Snapshot` field + 2 tab-render
  lines, CLI subcommand, TUI key `O`. **MINOR** (pre-1.0) — same class as the shipped `donation_details`/
  `optimize_attestation` side-tables; the flag lives OUTSIDE the event log (no forward-only concern). No
  docs/help-overlay mirror (parked convention).

## KATs
- `bulk_reclassify_outflow_plan_lists_pending_outflows` (candidate == `pending_reconciliation`; frame/wallet filters).
- **`bulk_reclassify_outflow_plan_excludes_missing_price`** [#a] + `..apply_never_emits_fabricated_proceeds`
  (defense-in-depth: fed a non-plan id, the apply `let Some(fmv)=… else continue` skips it — never a `0`/fabricated proceeds).
- `bulk_reclassify_outflow_plan_resolves_fmv_as_proceeds` (`row.fmv == fmv_of`; payload `principal_proceeds_or_fmv == row.fmv`).
- **`bulk_reclassify_outflow_plan_estimated_gain_matches_pending_legs_basis`** (2 lots of DIFFERENT bases →
  `estimated_gain == round_cents(fmv − Σ legs.usd_basis)`, provably not a coincidental zero).
- **`bulk_reclassify_outflow_plan_batch_gain_not_double_counted`** [the ordering-hazard pin] — Lot A then Lot
  B (higher basis) in one wallet; two outflows (60k @ d1, 80k @ d2>d1) so the 2nd spills A→B; assert
  `total_estimated_gain == Σ row.estimated_gain` AND row2's A-leg is exactly A's REMAINDER after row1 (never A's original size).
- `bulk_reclassify_outflow_scope_excludes_gift_and_donate` (TUI picker offers only Sell/Spend; CLI `--kind` rejects gift/donate).
- `bulk_reclassify_outflow_apply_uniform_kind`; `..fee_usd_none_but_fee_sat_flows`.
- `bulk_reclassify_outflow_empty_refuses`; `..dry_run_writes_nothing`.
- **Side-table (A+):** `bulk_reclassify_outflow_estimated_flag_persists_and_joins` (after apply+REOPEN the
  side-table row exists per applied `transfer_out_event`, and a control single-item `o` Sell is NOT flagged);
  `..side_table_reverts_on_mid_batch_failure` (mirror `bulk_void_reverts_mid_batch` — a failing row k>1 leaves
  no phantom flag rows either); `..disposals_tab_shows_est_marker` (the flagged Disposal row renders `[est]`).
- E2E `bulk_reclassify_outflow_apply_then_disposals_reflect` (`state.disposals` grows by the included count,
  `pending_reconciliation` shrinks by the same, NO new Hard blocker kind).
- TUI `..refuses_when_no_candidates`, `..per_row_exclude_drops_row`, `..preview_shows_estimated_gain_flagged`
  (modal text contains "ESTIMATED" adjacent to BOTH proceeds and gain).

## Plan (TDD)
- **Task 1 — engine of the cycle (btctax-cli):** `bulk_estimated.rs` side-table + `Session::bulk_reclassify_outflow_plan`
  + CLI `apply_bulk_reclassify_outflow` (own-loop + `mark`) + subcommand. KATs: plan/apply/#a/gain/ordering/side-table.
- **Task 2 — TUI (btctax-tui-edit + btctax-tui):** bespoke `persist_bulk_reclassify_outflow` (atomic + side-table
  in-envelope + rollback) + `O` flow + the **Disposals `[est]` marker + Compliance advisory count** (the
  user-mandated display) + `Snapshot` side-table field/load. TUI KATs.
- **Task 3 — whole-diff review (Phase E)** + full workspace suite + FOLLOWUPS + roadmap (program COMPLETE).

## Gotchas
- **#a is silent here** — a missing-price outflow classified as Sell would fabricate proceeds (no loud blocker).
  The plan's `fmv: Usd` (resolved, non-Option) + the apply's `let Some(fmv) else continue` are the twin defenses.
- **Gain, not just FMV** — the preview shows `Σ(fmv − legs.basis)`, flagged ESTIMATED; never just Σ proceeds.
- **The estimate is a preview only** — the PERSISTED Form-8949 numbers run the ordinary fold (always exact);
  the FIFO-vs-method / fee-treatment residuals affect only the preview estimate. Label it.
- **Side-table key = `transfer_out_event`** (== `Disposal.event`, fold.rs:633), NOT the decision id — else the
  Disposals-tab join misses.
- **Bespoke persist** (not a thin wrapper) BECAUSE of the in-envelope side-table write; lockstep-comment it
  against `persist_bulk_decisions`, mirror `persist_bulk_void`.
- **Dispatch derives from `plan.included`** — the #a exclusion + the estimate must not be bypassable via raw ids.
