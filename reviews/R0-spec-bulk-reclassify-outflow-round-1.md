# R0 — SPEC_bulk_reclassify_outflow (round 1)

**Artifact:** `design/SPEC_bulk_reclassify_outflow.md` (Cycle 5, the LAST).
**Branch/base:** `feat/bulk-reclassify-outflow` @ `f6f02b6`; main == `a241705`.
**Reviewer role:** independent architect, read-only spec review vs CURRENT source. No implementation, no branch switch.
**Bar:** 0 Critical / 0 Important.

## Verdict: **0 Critical / 1 Important / 3 Minor / 2 Nit** — NOT green (one Important must be resolved or explicitly accepted-with-KAT in round 2).

The two number-corrupting vectors I was told to attack hardest — **the side-table join key (#1)** and **the leg-basis double-counting (#2)** — are both **VERIFIED CORRECT against source**. No tax number is silently corrupted by this design as written. The one Important finding is a **display-lifecycle gap** (a persisted flag that can go stale after a void), not a tax-math error; the runtime harm is bounded and conservative, but the spec adopts a pattern (optimize_attest) and silently drops half of that pattern's discipline, which should be an explicit, tested decision before implementation.

---

## VERIFIED CORRECT (the load-bearing claims — evidence)

**#1 — Side-table join key is airtight (provenance).** The spec keys `bulk_estimated_proceeds` by `ReclassifyOutflow.transfer_out_event` and claims that IS the eventual `Disposal.event`. Traced end-to-end:
- `outflow_class` map is keyed by `ro.transfer_out_event` (`resolve.rs:610` `let target = &ro.transfer_out_event;` → `resolve.rs:644` `outflow_class.insert(target.clone(), ro.clone())`).
- `build_op` looks it up by the ORIGINAL TransferOut event's own `id` (`resolve.rs:236` `if let Some(ro) = outflow_class.get(id)`) and returns `Op::Dispose{…}` (`resolve.rs:253-259`). The Op therefore rides on the original TransferOut event, NOT the decision.
- The `Eff` carries `id: e.id.clone()` where `e` is the imported TransferOut (`resolve.rs:827-828`).
- The fold pushes `Disposal { event: eff.id.clone(), … }` (`fold.rs:633-634`).
- ⟹ `Disposal.event == eff.id == ro.transfer_out_event` for every bulk-reclassified outflow. The join is sound. Native `EventPayload::Dispose` events (exchange sells) carry different import ids, so they can never collide with a `bulk_estimated` key → no false flags from that direction. The spec's `fold.rs:633` citation is exact.

**#2 — No double-counting of leg basis.** The spec reuses fold-computed `PendingTransfer.legs` (`state.rs:204-210`; `PendingLeg` `state.rs:197-203`) and claims a single chronological fold pass makes `Σ` over multiple candidates disjoint. Verified the mechanism is real:
- `Op::PendingOut` calls `pools.consume_fifo(&key, total_sat)` in the fold (`fold.rs:712`) and stores exactly what it consumed as legs (`fold.rs:720-734`).
- `consume_fifo` → `consume_ordered` → `take_from` **mutates/drains** the pool: `lot.usd_basis -= gain_basis; lot.remaining_sat -= take;` and `lots.retain(|l| l.remaining_sat > 0)` (`pools.rs:227-231, 200`). So the 2nd pending outflow in a wallet sees the 1st's consumption. `Σ legs.usd_basis` over candidates cannot double-count.
- Precedent confirmed: `bulk_link_transfer_plan` already sums `pt.legs.iter().map(|l| l.usd_basis).sum()` over `state.pending_reconciliation` in one `load_events_and_project()` (`session.rs:510, 491, 529`).
- The spec's rejection of `evaluate.rs::CandidateDisposal` is VALID: the existing-event path returns `UnknownExistingDisposal` when the target isn't a honoring disposal (`evaluate.rs:117-122`) — a still-`Op::PendingOut` candidate fails this; the synthetic path runs a fresh whole-ledger `resolve(events,…)` per call (`evaluate.rs:105, 139`), so N candidates = N independent folds that don't see each other → would re-introduce double-counting. Fold-computed legs are the correct, cheap source.

**#3 — #a silent-failure defenses are structurally correct.** `ReclassifyOutflow.principal_proceeds_or_fmv` is `Usd`, NOT `Option` (`event.rs:118`) — confirmed you cannot represent a missing-price row without fabricating a number, and (unlike income's loud Hard `FmvMissing`) a fabricated proceeds gates nothing. Twin defenses confirmed real: (a) the plan row carries a resolved non-Option `fmv: Usd`; (b) the CLI apply re-derives with `let Some(fmv) = fmv_of(...) else continue` — this exact defense-in-depth already ships in the Cycle-4 precedent (`reconcile.rs:349-351`, with the identical "STRUCTURALLY unreachable even if a future caller passed a non-plan-filtered id" comment). Dispatch-from-`plan.included` (spec §CLI/§Gotchas) makes the exclusion non-bypassable.

**#4 — Persist atomicity is sound.** `persist_bulk_void` is exactly the bespoke shape the spec says to mirror: empty-guard → `pre = snapshot()` → per row `append_decision` guarded with `return Err(rollback(session,&pre,e))` (NOT `?`) → per-row side-effect (`optimize_attest::clear`) guarded the same way → single `save_or_rollback` (`persist.rs:557-593`). Whole-DB revert is genuine: `Session::snapshot` → `Vault::snapshot` → `sqlite_io::db_to_bytes(&self.conn)` (whole-DB image, all tables), `restore` → `db_from_bytes` swap (`session.rs:325-334`, `vault.rs:156-166`) — so `bulk_estimated` rows revert for free, same as the documented `optimize_attest` behavior (`persist.rs:242-244`). CLI uses its OWN append-loop with a trailing single `session.save()` (`reconcile.rs:329-366`); on a mid-batch `?` the in-memory session is discarded so neither event rows nor `mark`s reach disk (CLI atomicity is coarse discard-on-error, and both live on the same in-memory conn until `save`). The dependency-cycle reason for not calling the tui-edit helper is legitimate.

**#5 — Scope (Sell/Spend only) is sound.** `OutflowClass::Dispose { kind: DisposeKind } | GiftOut | Donate { appraisal_required }` (`event.rs:109-113`); Sell/Spend are the same `Op::Dispose` shape differing only in the reported tag (`resolve.rs:253-259`). Deferring GiftOut/Donate is well-justified: they need a `donee` (`event.rs:120-124`, non-uniform across a batch), and `Donate` Section-B FMV must be a qualified appraisal, not daily-close — `form_8283` derives `"qualified appraisal"` for `year_agg_deduction > QUALIFIED_APPRAISAL_THRESHOLD` (`forms.rs:366-375`), so auto-substituting market FMV could manufacture an unsupportable deduction. Deferral is acceptable and correctly reasoned.

**#6 — Candidate set + fee correct.** A reclassified TransferOut resolves to `Op::Dispose` (never `Op::PendingOut`), so it never re-enters `pending_reconciliation`; a wallet-less outflow returns before pushing (`fold.rs:699-709`). `fee_usd: None` is right because the on-chain `fee_sat` still flows independently via `resolve.rs:257` (`fee_sat: t.fee_sat`), regardless of `fee_usd`.

**#7 — Residual-imprecision disclosure is ACCURATE.** Preview basis uses `Op::PendingOut`'s FIFO-always (`fold.rs:712`); the real `Op::Dispose` uses `applicable_method(date, ctx)` and honors a named `selection` (`fold.rs:60-62`) → the FIFO-vs-election divergence is real and disclosed. On the fee point the spec is right and this is the subtle part I checked hardest: `Op::PendingOut` consumes `principal + fee_sat` and stores ALL legs (`fold.rs:711-712, 720-728`), while the real `Op::Dispose` consumes principal only (`fold.rs:596`) then re-homes the fee-sat carry onto the last disposal leg under the user's default TreatmentC (`fold.rs:609-624`). ⟹ under (c), `estimated_gain = fmv − Σ(principal+fee) basis` MATCHES the persisted number; only non-default TreatmentB (fee mini-disposition, leg basis unchanged) diverges by the bounded fee-basis amount — exactly what spec line 70 says. The persisted numbers are always the exact fold. Acceptable for a clearly-labeled preview; identical property to the shipped, R0-accepted `bulk_link_transfer_plan`.

**#8 — SemVer.** btctax-core is genuinely unchanged (all reused types verified present and untouched: `ReclassifyOutflow`, `OutflowClass::Dispose`, `fmv_of`, `pending_reconciliation`); the flag lives outside the append-only event log. Side-table (btctax-cli) + `Snapshot` field (btctax-tui) + CLI subcommand + TUI `O` (confirmed FREE — no existing `Char('O')` binding) — same class as the shipped `donation_details`/`optimize_attestation` side-tables. **MINOR** is correct. KAT set pins the ordering hazard, the #a exclusion, the side-table persist/join/revert, and the scope-lock. (Gaps below.)

Side-table pattern confirmed core-free and correctly modeled: `donation_details.rs` is btctax-cli-only, `CREATE TABLE IF NOT EXISTS`, keyed by `EventId::canonical()`, defensive `init_table` on every accessor, `all() -> BTreeMap<EventId,_>`; `init_table` wired in `Session::from_fresh_vault` (`session.rs:299-301`); loaded into `Snapshot` via the single shared `build_snapshot` (`unlock.rs:170-188`, `donation_details` at line 177) which the editor also calls for post-mutation re-projection (`editor.rs:79-80`) — so the user-mandated `[est]` marker WILL refresh in-session, not only after reopen.

---

## FINDINGS

### [I1] IMPORTANT — the persisted flag has no clear-on-void; it can go stale and render a false `[est]`
**Spec location:** §"The estimated flag — design A+" (lines 72-88); §Persist (lines 90-108); §KATs (154-157). The spec explicitly models on `donation_details.rs`/`optimize_attest.rs` but specifies NO void interaction.

**Source evidence.** The pattern the spec cites *defines* clear-on-void as its discipline: `persist_void` clears `optimize_attest` for a voided `LotSelection` (`persist.rs:261, 284`), `persist_bulk_void` does the same in-envelope (`persist.rs:585-589`), and both are pinned by dedicated KATs (`kat_p2f_void_lot_selection_clears_optimize_attest…`, `kat_persist_void_rollback_preserves_optimize_attest_on_failed_save`, `persist.rs:2155, 2322`). The spec's `bulk_estimated` writes a row keyed by `transfer_out_event`, and `ReclassifyOutflow` is voidable (spec line 132: "revocable tier … `ReclassifyOutflow` is voidable"). But `persist_void` only clears side-tables for `LotSelection` targets — a voided `ReclassifyOutflow` touches no side-table today, and the spec adds no `bulk_estimated::clear`.

**Failure sequence.** (1) Bulk-reclassify TransferOut X → Dispose, `bulk_estimated[X]` written. (2) Void the reclassify → X reverts to `Op::PendingOut`, no `Disposal{event:X}`; the flag row survives (orphan, invisible while voided — harmless). (3) Re-reclassify X via single `o` with a REAL, user-entered sale price → new `Disposal{event:X}` → the stale `bulk_estimated[X]` joins → the Disposals tab shows `[est]` on a disposal whose proceeds are now exact. The Compliance "N disposals use estimated FMV proceeds" advisory (spec line 86-87) similarly mis-states if it counts raw side-table rows rather than rows that join to a live disposal.

**Why Important (with honest framing).** This does NOT corrupt a tax number and errs in the conservative direction (over-flags "revisit this," never under-flags an estimate as exact), and it requires an uncommon void+re-reclassify path — so runtime harm is bounded. It is Important because (a) the spec adopts the optimize_attest side-table pattern and silently omits its defining clear-on-void half with NO analysis, (b) this is the program's FIRST persisted flag on a **user-mandated** trust surface (the Disposals tab), and (c) an implementer following the spec literally will reproduce the gap. At a 0C/0I gate this should be an explicit, tested decision, not silence.

**Concrete fix (either is acceptable):**
- **(a) Wire the clear.** Add `bulk_estimated::clear(out_event)` to `persist_void` (the `ReclassifyOutflow`/void-of-target arm) and `persist_bulk_void`, in-envelope, mirroring `optimize_attest::clear` (`persist.rs:284, 586`); add a KAT `bulk_reclassify_outflow_flag_cleared_on_void`. OR
- **(b) Document-accept + test the invariant.** State in the spec that a stale flag is an accepted conservative false-positive, require the Disposals marker and the Compliance count to be computed by **joining against live `state.disposals`** (so an orphaned row renders nothing / isn't counted), and add a KAT `bulk_reclassify_outflow_stale_flag_after_void_does_not_mismark` (void → re-reclassify via single `o` with a real price → assert no `[est]` and count excludes it). Note this (b) is only sufficient if step-3 re-reclassify-via-single-`o` is out of the marker's join scope, which it is NOT under a raw `transfer_out_event` join — so (b) in practice still needs the join to be disposal-provenance-aware or the clear from (a). Prefer (a).

---

### [M1] MINOR — the typed `Session::bulk_estimated()` accessor is not enumerated; build_snapshot must not touch `conn()`
**Spec location:** lines 82-83 ("loaded into `Snapshot` (new field) like `donation_details`").
**Source evidence.** `build_snapshot` is contractually "Uses ONLY the typed read-only methods — never `session.conn()` directly [R0-I1]" (`unlock.rs:168`) and loads via `session.donation_details()` (`unlock.rs:177`), a typed wrapper over the side-table `all()` (`session.rs:369-373`). The spec references the side-table `all()` and the new `Snapshot` field but never calls out the mirror `Session::bulk_estimated()` accessor that `build_snapshot` requires — an implementer could reach for `session.conn()` directly in the TUI and violate the invariant.
**Fix:** add `Session::bulk_estimated() -> Result<BTreeMap<EventId, _>, CliError>` (thin wrapper over `bulk_estimated::all`) to the Task-1 deliverables, and have `build_snapshot` call it (never conn()).

### [M2] MINOR — no KAT pins CLI-side side-table atomicity (mid-batch failure writes NO flag rows)
**Spec location:** §Persist CLI (lines 99-105); §KATs (142-161).
**Source evidence.** The CLI apply writes `bulk_estimated::mark` with a bare `?` before the trailing `session.save()` (spec line 105), relying on discard-on-error (`reconcile.rs:329-366` pattern). The KAT list pins the TUI mid-batch revert (`..side_table_reverts_on_mid_batch_failure`, line 156) but nothing asserts the CLI path leaves NO `bulk_estimated` rows on a mid-batch `?` failure.
**Fix:** add `bulk_reclassify_outflow_cli_mid_batch_failure_writes_nothing` (a failing row k>1 → reopened vault has neither the `ReclassifyOutflow` appends nor any `bulk_estimated` rows).

### [M3] MINOR — the optional stored proceeds/gain snapshot must not override the exact numbers on the Disposals tab
**Spec location:** line 81 ("optionally the estimated proceeds/gain snapshot for display").
**Source evidence.** The Disposals tab renders the fold's exact `leg.proceeds/basis/gain` (`disposals.rs:40-51`). If an implementer reads "snapshot … for display" as "render the stored preview numbers," the tab would show FIFO-preview figures alongside exact ones for other rows — an internal inconsistency, and directly contradicts spec lines 65-70/175 ("the PERSISTED numbers are always exact").
**Fix:** state explicitly that the Disposals tab renders the fold's exact leg numbers and adds ONLY the `[est]` marker; any stored proceeds/gain snapshot is informational (never rendered as the disposal's numbers). If it is never displayed, consider dropping it from the row to avoid the temptation.

### [N1] NIT — plan row `wallet (always Some)` diverges from the precedent's defensive `Option<WalletId>`
**Spec location:** line 112 (`wallet (always Some)`).
**Source evidence.** The mirrored `BulkLinkRow` keeps `source_wallet: Option<WalletId>` with an explicit "[R0-N2] ALWAYS `Some` … `Option` kept defensively" note (`session.rs:44-49`). A non-Option field forces an unwrap in `enrich`. The wallet is also unused for the reclassify itself (only for the source-wallet filter/display).
**Fix:** mirror the precedent (`Option<WalletId>`, defensive), or drop the field if only used for filtering.

### [N2] NIT — a couple of struct citations are span-approximate (verify-at-write-time still satisfied)
**Spec location:** line 54 (`state.rs:198-210` for legs), line 73 (`state.rs:123-150`).
**Source evidence.** `PendingLeg` is `state.rs:197-203` and `PendingTransfer` `204-210` (spec's 198-210 spans both, fine); `DisposalLeg` is `123-140`, `Disposal` `141-150` (spec's 123-150 spans both, fine). No claim is wrong — just tighten the ranges on the next pass.

---

## Note (not a finding): single-`o` vs bulk asymmetry is intentional
The control KAT (`…estimated_flag_persists_and_joins`, line 154-155) asserts a single-`o` Sell is NOT flagged. That is a deliberate, defensible scope choice — a single-`o` reclassify may carry a user-entered REAL proceeds, whereas the bulk path is ALWAYS auto-FMV. Worth one sentence in the spec so it reads as a decision, not an omission. (This same asymmetry is what makes I1's step-3 false-flag possible, which is why I1 prefers the clear-on-void fix.)

## Round-2 gate
Resolve **I1** (wire clear-on-void, or the document-accept path with a disposal-provenance-aware join + KAT). M1-M3 and N1-N2 are cheap spec edits. Re-review after the fold, including the last. The two tax-number vectors are clean; nothing here blocks on the math.
