# Whole-diff review (Phase E) — bulk-classify-inbound-self-transfer — round 1

**Branch:** `feat/bulk-classify-inbound-self-transfer` · **Diff:** `main..HEAD` (main `569a5ee`;
spec `9f5c1cd`, Task 1 `1767b3c`, Task 2 `1d91118`) · ~1979 ins / 45 del, 10 files.
**Spec:** `design/SPEC_bulk_classify_inbound_self_transfer.md` (R0-GREEN, 2 rounds).
**Reviewer:** independent, adversarial. Trust-nothing; 3 fault-injections executed + restored.

## Verdict: **0 Critical / 0 Important / 0 Minor / 1 Nit** — SHIP.

The implementation is a faithful mirror of the shipped `bulk-link-transfer` applied to Cycle A's
`InboundClass::SelfTransferMine`. The tax-safety spine (filter-3 already-classified + wallet-less
exclusion), the honest USD floor, the atomic N-append/one-save with mid-batch rollback, and the
reversibility accounting are all present, correct, and pinned by KATs that I confirmed are
non-vacuous via fault injection. `btctax-core` is UNTOUCHED (`git diff main..HEAD -- crates/btctax-core`
empty — verified). All 13 new KATs pass; KAT-G1 green; clippy clean on both changed crates.

---

## Fault-injection results (item 9 — all 3 starred, all RED, all restored byte-for-byte)

| # | Injection | KAT | Result |
|---|-----------|-----|--------|
| I1 | Removed the `already_classified.contains(id)` exclusion in `session.rs::bulk_self_transfer_in_plan` | `bulk_sti_plan_excludes_already_classified_and_walletless` | **RED** — `included` became `[sti-normal, sti-gift4]` (the gift-case-4 already-classified inbound leaked in → a bulk apply would fire a return-blocking Hard `DecisionConflict`). Wallet-less still excluded, so the KAT isolates I1. |
| bulk-I1 | Changed the mid-batch `if let Err(e) { return Err(rollback(...)) }` to a bare `?` in `persist.rs::persist_bulk_self_transfer_in` | `kat_persist_bulk_sti_reverts_mid_batch_append_failure` | **RED** — error surfaced as bare `NoChange(...)` (contract: "vault unchanged") instead of `RolledBack`, leaving append #1 as live phantom residue. |
| M2 | Removed the wallet-less `else { continue }` exclusion in `session.rs` | `bulk_sti_plan_excludes_already_classified_and_walletless` | **RED** — `included` became `[sti-normal, sti-walletless]` (wallet-less TransferIn that creates no lot leaked in). gift-case-4 still excluded, so the KAT isolates M2. |

`git status` clean after each restore; final `git diff --stat` vs HEAD empty.

---

## Item-by-item

**1. [I1 ★★] filter-3 already-classified exclusion — CORRECT.** `session.rs:461-509` builds
`voided` = `VoidDecisionEvent.target_event_id` set, then `already_classified` =
`events.filter(|e| !voided.contains(&e.id)).filter_map(ClassifyInbound → transfer_in_event)`. This is
EXACTLY the spec's `classifyinbounds.filter(|c| !voided.contains(&c.id)).map(|c| c.transfer_in_event)`
— it intersects each ClassifyInbound's OWN decision id against `voided` (decision-id space) and maps
the survivor to its `transfer_in_event` (TransferIn-id space); the disjoint id-spaces are respected
[R0-M-r2-1], NOT a naive "minus VoidDecisionEvent targets". Byte-for-byte mirror of the single-item
`open_classify_inbound_flow` filter 3 (`main.rs:2139-2171`). The KAT fixture genuinely includes a
gift-case-4 `GiftReceived{donor_basis:None,donor_acquired_at:None}` (re-fires `UnknownBasisInbound` —
confirmed at `fold.rs:932`) AND a wallet-less `TransferIn`, and FIRST asserts all three re-fire
`UnknownBasisInbound` (`reconcile.rs:1745-1755`) so the exclusions are meaningful, not vacuous.
Fault-injected RED.

**2. [M2 ★] wallet-less exclusion — CORRECT.** `session.rs:492-494` `let Some(wallet) = ev.wallet
.clone() else { continue }` drops wallet-less inbounds (which `fold.rs:966-978` shows create no lot and
re-fire the blocker). Survivors always carry `wallet: Some(_)`, so the E2E "included → $0-basis lot
created" holds. Fault-injected RED.

**3. [bulk-I1 ★] mid-batch rollback + empty guard — CORRECT.** `persist.rs::persist_bulk_self_transfer_in`
refuses empty BEFORE the snapshot (`NoChange(CliError::Usage(..))`, same shape as
`persist_bulk_link_transfer` [R0-M1]), snapshots, loops appending with
`if let Err(e) { return Err(rollback(session,&pre,e.into())) }` (NOT `?`), single `save_or_rollback`.
`kat_persist_bulk_sti_reverts_mid_batch_append_failure` uses a BEFORE-INSERT trigger firing on the
2nd append's decision_seq (so append #1 is already committed), asserts `RolledBack` + byte-unchanged
log + clean retry. Fault-injected RED. Empty-guard KAT `kat_persist_bulk_sti_refuses_empty` green.

**4. Honest floor + fmv_of — CORRECT.** Plan sums `total_usd_fmv_floor` = Σ of the `Some` `usd_fmv`
only + `missing_price_count`. CLI `render_bulk_sti_preview` (`main.rs`) renders exact `$X` when
`missing==0` else `≥ $X (N unavailable)`; TUI reuses `bulk_usd_floor_label`. USD is
`btctax_core::price::fmv_of(&prices, date, sat)` [G4] in both plan and preview — not hand-rolled. No
blank / false-exact. `bulk_sti_plan_fmv_floor_when_price_missing` pins it.

**5. Reuse faithful + non-taxable — CORRECT.** Both surfaces append
`ClassifyInbound{SelfTransferMine{None,None}}` per row. `fold.rs:996-1007` confirms this projects to a
$0-basis lot with `basis_pending: false` (computable → never gated); the advisory soft
`SelfTransferInboundZeroBasis` blocker (the deliberate over-tax surface) is NOT a return-blocker and is
correctly excluded from the "remaining" count. CLI apply is one-session/N-append/one-save (session
dropped on error before save → nothing on disk); RELOCATE/other paths untouched.
`bulk_sti_cli_apply_is_atomic_single_save` asserts exactly N `SelfTransferMine{None,None}`, cleared
blockers, and `usd_basis==0` lots. The E2E `bulk_sti_then_void` pins void → `UnknownInbound` re-exposed
AND the inbound reappears in a FRESH `plan.included` [R0-M-r2-1], with no `DecisionConflict`.

**6. KAT-G1 cleanliness — GREEN.** Opener enumerates candidates + filter choices from `snap`
directly; only the priced preview routes through `Session::bulk_self_transfer_in_plan`; the batch
append lives in `edit/persist.rs`. `kat_g1_mechanized_source_gate` passes (no forbidden
`conn(`/`save(`/`append_`/`cmd::` token in tui-edit non-test source).

**7. No existing-test regression — VERIFIED ADDITIVE (added 13, removed 0).** `git diff --numstat`:
`tests/reconcile.rs` +366/-0 (purely additive block at :1559), `edit/persist.rs` +207/-0 (new tests
appended), `main.rs` +790/-36. The ONLY removed `fn` is `derive_bulk_link_status`, confirmed
BYTE-IDENTICAL between `main` and `HEAD` (a pure re-position ahead of the inserted
`handle_bulk_sti_*` fns — `diff` of the extracted function == identical). All other deletions are
`use {...}` import-list reformatting in main.rs/draw_edit.rs/editor.rs/reconcile.rs/lib.rs. No `#[test]`
removed, no assertion deleted/loosened. The 832-vs-992 count gap is a counting-method artifact
(controller sums every "N passed" line incl. doctests), NOT a deletion. New tests: 6 CLI + 3 persist
+ 4 main = 13.

**8. CLI/TUI wiring, empty guards, clap, key `B` — CORRECT.** Clap variant uses
`year conflicts_with_all=[from,to]`, `from requires to`, `to requires from` → year XOR from/to
enforced; dispatch match has the defensive catch-all `_ => Usage`. Free Browse key `B` bound
(`main.rs:353`) alongside `b`; `close_all_mutation_surfaces` now clears `bulk_sti_flow` +
`bulk_sti_modal` (`main.rs:582-583`) — the "reset_flows doesn't exist" note is correct and handled.
Empty guards at every gate: empty plan → opener status/no-open; empty filter result → stay on Filter
with error; all-unchecked → `open_bulk_sti_modal` refuses ("Nothing selected"); persist refuses empty.
The absent dedicated `Frame::Range` CLI KAT is acceptable (identical `in_frame` logic to the
KAT-covered bulk-link; range is exercised by the shared `Frame` type). The opener's candidate
predicate matches the plan's exactly, so "opener opens but plan(All/Any) empty" cannot occur.

**9.** See fault-injection table above (I1, bulk-I1, M2).

**10. Dead code / clippy / doc — CLEAN.** `cargo clippy -p btctax-cli -p btctax-tui-edit
--all-targets` emits zero warnings. Doc comments are thorough and cite the R0 findings they discharge.

---

## [N1] Nit — confirm-modal label alignment
`draw_edit.rs::draw_bulk_sti_modal`: the `Σ USD → $0 basis :` line's colon does not column-align with
`deposits    :` / `Σ BTC       :` above it (and the `Σ` multibyte chars already make space-padded
alignment approximate). Purely cosmetic; does not gate. Optional: pad to a common colon column.

---

## Ship gate
0 Critical / 0 Important. **Cleared to ship.** (Task 3 also records FOLLOWUPS queue item 3 — bulk for
the other decision types — per the spec plan; not part of this diff's correctness gate.)
