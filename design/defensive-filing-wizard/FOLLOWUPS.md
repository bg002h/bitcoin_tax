# Defensive Filing Wizard — FOLLOWUPS

Phase-owned follow-ups discovered during the subagent-driven build (per-task + phase-gate reviews).
Burned down on the **owning phase's** schedule (STANDARD_WORKFLOW §"per-phase, by ownership"), not all-at-end.
Reconciliation is a grep: on entering a phase, sweep this file for that phase's items. Live per-task/gate
state lives in the (gitignored) SDD ledger `.superpowers/sdd/progress.md`.

Legend: **[open]** not started · **[done]** burned down (kept for provenance) · owning phase in **bold**.

## P-C (must close before the P-C green gate)

*(The era→window preset-table item was RE-OWNED to **P-D/ship** at the P-C gate — see that section. It is a USER
decision the phase cannot discharge, so leaving it here made the gate unclosable by construction. Nothing else
P-C-owned remains open.)*

- **[done] `declare_flow::nudge_window_start` has no lower bound** (T8-review Minor-2). Was able to move
  `window_start` past `window_end`/before genesis. Filing-safe (surfaced live as `NoCoverage`; `plan_declare` refuses
  at confirm), so this was a UX-robustness fix, not a correctness gate. Clamped `nudge_window_start` to
  `window_start <= window_end` and to never precede Bitcoin's genesis block (`era_window(ALL_PRESETS[0]).0`,
  2009-01-03 — the SAME floor already governing the oldest era preset; no new date invented). KAT:
  `declare_flow::tests::nudging_window_start_never_crosses_past_window_end_or_before_genesis`. (Owner: **P-C/Task 9**
  polish — DONE.)

## P-C / Task 9 (the promote flow)

- **[done] T9-review Minor-1 — the Consent step didn't echo the purchase attestation.** The Part II
  authoring screen shows `PROVENANCE_TEXT` ("By continuing, you attest: ...") but the Consent screen —
  where the filer types the TypedWord ack — did not restate it, even though `render_consent(&plan)`
  (the shipped chokepoint text) never includes `PROVENANCE_TEXT` (it's built purely from
  `plan.advisory_lines`/`plan.terms`/`plan.gift_only_years`/`plan.post_consent_note`). Added a one-line
  echo ("You attest: {PROVENANCE_TEXT}") right above the ack prompt in `render_promote_flow`'s `Consent`
  arm — DISPLAY-only, in the flow's own render, never touching the chokepoint `render_consent` or the
  recorded `Acknowledgment`/`shown_terms`. KAT:
  `promote_flow::tests::consent_step_renders_the_purchase_attestation_echo_above_the_ack_prompt`.
  (Owner: **P-C/Task 9** — DONE.)
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

## P-D / ship (re-owned + newly filed at the P-C gate, 2026-07-26)

- **[open] Era→window preset table — real product-authored content.** ★ RE-OWNED from P-C to **P-D/ship** (arch gate
  adjudication): it is a **USER** decision the phase cannot discharge, so leaving it P-C-owned made the gate unclosable
  by construction. Both lenses ruled it non-blocking (presets are seeds; `plan_declare` re-validates the chosen window;
  the filed floor is `filed_basis_for` requiring `Coverage::Full`; `defensive_era.rs` KATs pin the structural properties).
  **Blast radius when the owner decides:** `crates/btctax-core/src/defensive/era.rs` + `crates/btctax-core/tests/defensive_era.rs`
  (pins all five windows verbatim + `ALL_PRESETS.len()`) + the filer-facing preset label (currently `{:?}` → `Y2009To2011`).
  **Three tax-relevant sub-decisions ride with it** (tax gate): (a) which preset is the DEFAULT — that sets the default
  holding-period character (see M-5 below); (b) cycling to a preset later than the short op must not leave an inverted
  window (fixed in code at `47225af`, but re-check against any new table); (c) there is no ≥2025 bucket, so a post-2024
  shortfall's window is reachable only by ±1-day nudges (see the free-text-entry item).
- **[open] tax-M-5 — the default preset seeds a taxpayer-FAVORABLE holding date.** `window_end` IS the lot's acquisition
  date (`resolve.rs:1310`), so defaulting to the oldest bucket (2009-01-03..2011-12-31) makes nearly every disposal
  **long-term** at the preferential rate — while the code justifies oldest-first purely on the basis axis. Not silent
  (window + "(long-term at the short op's date)" render on Edit and Confirm) and there is shipped precedent
  (`conventions::long_term_default_acquired`), but it is a TAX dimension of the era decision and should be chosen
  explicitly. (Owner: **P-D/ship**, rides the era-table decision.)
- **[open] SPEC line for the provenance step.** The P-C gate added an explicit BG-D5 provenance-selection step to the
  promote flow (tax I-2). No design artifact names a provenance picker either way — SPEC §3/DFW-D2 should gain a line so
  the step is spec-anchored rather than review-anchored. (Owner: **P-D/ship**, doc-only.)

## P-D / whole-branch (deferred at the P-C gate — non-blocking, but sweep before merge)

- **[open] tax-M-3 — displacement-caveat hole for a correctly-sized cover.** `defensive/mod.rs:659-688`:
  `WouldDisplaceIfPromoted` fires only when `covered_sat == 0`; when `covered_sat > 0 && t.sat == covered_sat`, neither it
  nor `OverCovered` fires — yet a HIFO reorder across multi-year disposals still shifts gain between years, so that row's
  per-year delta is a reorder artifact shown as an unqualified saving. Fix: fire on `!promoted && displaces_documented_basis(..)`,
  suppressing only where `OverCovered` already carries displacement copy.
- **[open] tax-M-4 — the declare flow's on-demand tax-Δ carries no displacement caveat** (`declare_flow.rs:293-307` prints
  bare `$delta`/`gain-Δ`), while the dashboard row's equivalent number is caveated. `declare_preview_saving` already builds
  both folds, so the check is nearly free.
- **[open] arch-M-1 — ~35 lines of verbatim duplication between the two confirm tails** (`declare_flow_confirm` vs
  `promote_flow_confirm`) + a third copy of the dashboard refresh in `open_defensive_filing`. Extract
  `after_defensive_write(app, status)` + a shared `refresh_defensive_dashboard(app)`.
- **[open] arch-M-2 — wire `DeclareFlowState::clearance()` into the readout** (it has no non-test caller; the doc was
  corrected at `47225af` to say the clearance runs at confirm). Wiring it would also let the flow surface the REAL refusal
  instead of predicting one.
- **[open] arch-N-1 — `debug_assert!(open_flow_count() <= 1)` is evaluated before the flow field is set** (so it permits one
  OTHER flow open); the invariant it names is `== 0`. One-line tightening at three sites.
- **[open] Browse footer does not list `w`** (the overlay does, and `?` points at the overlay). Adding it pushed `?: help`
  off the 120-col footer and golden tests caught it; deliberately reverted with an in-source rationale. Revisit only if the
  footer is ever reflowed.
- **[open] tax Minor 2 — flow renders have no scrolling; the NOTICE rect makes the promote Consent surface 3 rows
  shorter.** `draw_edit.rs:162-174` + `DEFENSIVE_NOTICE_LINES = 3`: one `Paragraph` with `Wrap`, no `.scroll(...)`, so
  content beyond the rect is not drawn — pre-existing, but the P-C gate r2 fix wave makes it 3 rows worse on the narrow
  path (a `RolledBack` sets a status, the flow bounces to `PartII`, the filer Tabs back). Tax stake:
  `Acknowledgment.shown_terms` records terms as SHOWN; on a short terminal a trailing term can be recorded as shown
  without being rendered. Fix: scroll support or an "N more lines" indicator.
- **[open] arch N-r2-1 — `Esc` at the promote flow's PartII step cancels the WHOLE flow rather than stepping back to
  Provenance** (`handle_promote_flow_part_ii_key`), while `Esc` at Consent steps back one. Harmless (PartII is only
  reachable via an attested `Purchase`) — worth a doc line only.
- **[open] arch N-r2-2 — the NOTICE's ~230-char CRITICAL status clips below ~77 columns.** Graceful (never a panic),
  and "Quit the editor NOW" survives in the first ~120 chars, so this is cosmetic.
- **[open] arch N-r2-3 — both I-2 render KATs (`draw_edit.rs:7068` + sibling) exercise the DASHBOARD surface only.**
  Add one case with `promote_flow` open (cheap insurance against a regression narrower than the dashboard).
- **[open] N-r2-4 residue — three r1 Nits neither folded nor filed.** (a) tax N-3: the tautological
  `empty_events.len() == 0` assert plus two `let _ = ...` import-keepers in
  `declare_flow::tests::clearance_reflects_plan_declare_and_is_a_pure_read` (`declare_flow.rs:736-756`) —
  `empty_events` is a shared `&[LedgerEvent]`, so the borrow checker already guarantees no mutation; the assert and
  the import-keepers test nothing. (b) tax N-4: the test name
  `defensive_journey.rs::declare_preview_saving_edits_the_window_and_changes_nothing_it_should_not` is misleading —
  the body makes a single call over a fixed window and never edits anything; rename or rewrite to actually exercise
  re-derivation across an edit. (c) the existing "Debug-format rows" item below is now widened to cover the two new
  flow renders.

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
- **[open] Debug-format rows** (P-B arch N1; widened at the P-C gate r2 / N-r2-4(c)) —
  `render_candidate/tranche/pool_short/resolve_first_row` emit `{:?}` on `EventId`/`PoolKey`/`BlockerKind` (e.g.
  `Decision { seq: 1 }`) — ugly for a filer; give them filer-facing formatting. Same class in the two P-C flow
  renders: `render_declare_flow`/`render_promote_flow` add `{:?}` on shortfall/wallet/era-preset/tranche (e.g.
  "shortfall Decision { seq: 1 }", `wallet: SelfCustody { label: "..." }`, `era preset: Y2009To2011`, "tranche
  Decision { seq: 1 }").
- **[open] tax Nit 2** — the Provenance screen states which answer passes the gate BEFORE the filer answers ("Only a
  PURCHASE can be promoted…" renders above the picker) — honest disclosure of the rule (same wording
  `refuse_non_purchase` uses), but mildly leading on a screen whose purpose is an unprompted filer answer. Consider
  moving that sentence into the refusal path.
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
