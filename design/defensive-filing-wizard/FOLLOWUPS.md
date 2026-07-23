# Defensive Filing Wizard — FOLLOWUPS

Phase-owned follow-ups discovered during the subagent-driven build (per-task + phase-gate reviews).
Burned down on the **owning phase's** schedule (STANDARD_WORKFLOW §"per-phase, by ownership"), not all-at-end.
Reconciliation is a grep: on entering a phase, sweep this file for that phase's items. Live per-task/gate
state lives in the (gitignored) SDD ledger `.superpowers/sdd/progress.md`.

Legend: **[open]** not started · **[done]** burned down (kept for provenance) · owning phase in **bold**.

## P-C (must close before the P-C green gate)

- **[open] Era→window preset table — real product-authored content.** (Owner: **P-C**, product/user decision.)
  The plan/SPEC (DFW-D9) referenced a "reviewed era→window preset table" that was never authored anywhere in the
  design corpus. Task 8 built the full mechanism (confirm/edit, prefill-precedence, live readout) using clearly-flagged
  **PROVISIONAL** round calendar-year buckets (2009–2011, …, 2021–2024) — see `crates/btctax-core/src/defensive/era.rs`
  module doc. **Filing-neutral** (presets are editable starting suggestions; `plan_declare(Some)` validates the chosen
  window; the filed floor is `filed_basis_for` requiring `Coverage::Full`), so it does NOT gate correctness — but the
  actual windows + labels are a product/copy + date-boundary decision the owner should make (or bless the calendar
  buckets) before ship. *(Closes T8-review Minor-1 by existing: `era.rs` cites this file.)*
- **[open] `declare_flow::nudge_window_start` has no lower bound** (T8-review Minor-2). Can move `window_start` past
  `window_end`/before genesis. Filing-safe (surfaced live as `NoCoverage`; `plan_declare` refuses at confirm). UX polish;
  contained to `declare_flow.rs`. (Owner: **P-C/Task 9** polish.)

## P-C / Task 9 (the promote flow)

- **[done: no split needed] T2-M1 — `Refusal::Coverage` overload.** Confirmed at Task 9, the FIRST (and only)
  `Refusal`-consuming TUI flow: `promote_flow.rs`'s `review()` never branches on a `Refusal` variant — it only maps
  the WHOLE enum through `From<Refusal> for CliError` and displays `.to_string()` (grepped: the sole `Refusal::`
  mention in `btctax-tui-edit/src` is a doc comment, not a match arm). The routing signal the flow uses upstream is
  `journey_view`'s own `TrancheStatus`/`safe_harbor_blocked`, never the `Refusal` enum. No enum split needed — closing
  as YAGNI-confirmed, not merely YAGNI-presumed. (Owner: **P-C/Task 8-9** — DONE.)
- **[done] T2-M2 — phantom-wallet stderr byte-assertion.** Added `phantom_wallet_warning_is_emitted_verbatim_on_a_successful_declare`
  / `phantom_wallet_warning_is_silent_on_a_refused_declare` to `crates/btctax-cli/tests/declare_tranche_cli.rs` (spawns
  the real `btctax` binary — `eprintln!` cannot be intercepted in-process — mirrors `chokepoint_parity.rs`'s subprocess
  convention): pins the shipped phantom-wallet warning is emitted verbatim on an unknown-wallet declare AND silent on a
  refused (non-positive `--amount`) declare. (Owner: **P-C** — DONE.)
- **[done] T4 — `Refusal::Target` parity uncovered.** Added `assert_target_refusal_parity` + three KATs
  (`bg_target_unknown_ref_refusal_is_identical_across_drivers` / `..._voided_tranche_..` / `..._wrong_type_..`) to
  `crates/btctax-cli/tests/chokepoint_parity.rs`: an unknown/voided/wrong-type target is refused byte-identically
  across the CLI verb and `chokepoint::plan_promote`, both mapping through the SAME `From<Refusal>`, and asserted to
  be the `Refusal::Target` variant specifically. (Owner: **P-C/Task 9** — DONE.)

## Task 10 / P-D (the export step)

- **[open] T3-M2 — `apply_export` has no per-year error isolation.** A flagged year with no bundled form template `?`-returns
  and aborts the batch (already-written years stay correct; fails loud; no unattested/pseudo packet escapes). Task 10's
  multi-year driver should decide per-year "2 of 3 exported, year 3 failed" reporting — MAY revise `apply_export`'s return
  type (`Vec<Result<…>>` vs `Result<Vec<…>>`); acceptable, no external consumers (no-users-yet). (Owner: **Task 10**.)
- **[open] T3-M1 — per-year `out_dir/<year>/` subdir is an unbriefed layout contract** (decided + KAT-pinned in P-A). Task 10's
  TUI must surface/read under it. (Owner: **Task 10**, display-only.)

## Copy pass / whole-branch review (ownerless residue — batch to the end)

- **[open] T7-copy** — `defensive_dashboard.rs`: "[optional, SUPPRESSED] promote" reads as *disabled* though core does NOT
  refuse a fee-only promote (DFW-D1 no-second-gate); `[x] export` bracket notation inconsistent with the `'d'`/`'p'`
  quoted-key style.
- **[open] Debug-format rows** (P-B arch N1) — `render_candidate/tranche/pool_short/resolve_first_row` emit `{:?}` on
  `EventId`/`PoolKey`/`BlockerKind` (e.g. `Decision { seq: 1 }`) — ugly for a filer; give them filer-facing formatting.
- **[open] Free-text date/sat entry** (T8-review Nit) — the declare flow edits via nudge (±1d/±1000 sat) + preset-cycling
  (a legitimate DFW-D9 "edit"); free-text entry, if wanted, is a contained `declare_flow.rs` follow-up.
- **[open] Plan-doc drift** — `IMPLEMENTATION_PLAN.md:61` File-Map names `ShortfallCandidate`; the shipped type is `Shortfall`.
  Doc-only; code is internally consistent.

## Done (burned down in their owning phase — provenance)

- **[done] M-new-1** (P-A gate → P-B/Task 6): `promote_changed_years` forces `pseudo_reconcile=false` on an own copy; KAT'd.
- **[done] T3-Nit** (→ Task 6): `journey_view.flagged_years` == the `< current`-filtered export set; KAT'd.
- **[done] T6-Minor1** (→ Task 8): the on-demand tax-Δ readout sources the profile-aware `clamped_promote_year_saving`.
- **[done] T7-entrykey** (→ Task 8): Browse `w` → `EditorScreen::DefensiveFiling` (+ pseudo-refusal); KAT'd.
- **[done] P-B-tax-Minor** (→ Task 8): `Advisory::WouldDisplaceIfPromoted` caveats a displacement-driven gain-Δ; KAT'd.
- **[done] arch-Minor2** (→ Task 8): `residue_latch_status()` guard at `open_defensive_filing`; mutation-verified.
- **[done] arch-Minor1** (→ Task 8): visible cursor marker on the dashboard; KAT'd.
