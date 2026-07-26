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

- **[done] Era→window preset table — real product-authored content.** ★ RE-OWNED from P-C to **P-D/ship** (arch gate
  adjudication): it is a **USER** decision the phase cannot discharge, so leaving it P-C-owned made the gate unclosable
  by construction. Both lenses ruled it non-blocking (presets are seeds; `plan_declare` re-validates the chosen window;
  the filed floor is `filed_basis_for` requiring `Coverage::Full`; `defensive_era.rs` KATs pin the structural properties).
  **Blast radius when the owner decides:** `crates/btctax-core/src/defensive/era.rs` + `crates/btctax-core/tests/defensive_era.rs`
  (pins all five windows verbatim + `ALL_PRESETS.len()`) + the filer-facing preset label (currently `{:?}` → `Y2009To2011`).
  **Three tax-relevant sub-decisions ride with it** (tax gate): (a) which preset is the DEFAULT — that sets the default
  holding-period character (see M-5 below); (b) cycling to a preset later than the short op must not leave an inverted
  window (fixed in code at `47225af`, but re-check against any new table); (c) there is no ≥2025 bucket, so a post-2024
  shortfall's window is reachable only by ±1-day nudges (see the free-text-entry item).
  ★ **CLOSED — the OWNER decided (2026-07-26): the round calendar buckets are RATIFIED as intentional (they make no
  historical/exchange/event claim, so nothing in them can be factually wrong), there is now NO default preset (an
  explicit filer pick is REQUIRED — see M-5), and a `Y2025Onward` bucket was ADDED.** The `PROVISIONAL` /
  "NOT a product-approved artifact" self-disclaimer was stripped from `era.rs` (it would have published to crates.io)
  and replaced with an accurate, non-overclaiming note. Sub-decision (b) survives in its stronger form — an
  inapplicable preset is now REFUSED with a reason rather than silently skipped. Sub-decision (c) is closed by the new
  bucket. `defensive_era.rs`'s `all_presets_end_strictly_before_the_pre2025_pooling_cutover` was REPLACED (not deleted)
  by `no_preset_window_straddles_the_pooling_cutover`, the invariant that actually preserves the property the old guard
  protected. The `{:?}` label Nit stays open under the "Debug-format rows" residue item (the picker now renders each
  bucket's concrete dates beside its name, which carries the substance).
- **[done] tax-M-5 — the default preset seeds a taxpayer-FAVORABLE holding date.** `window_end` IS the lot's acquisition
  date (`resolve.rs:1310`), so defaulting to the oldest bucket (2009-01-03..2011-12-31) makes nearly every disposal
  **long-term** at the preferential rate — while the code justifies oldest-first purely on the basis axis. Not silent
  (window + "(long-term at the short op's date)" render on Edit and Confirm) and there is shipped precedent
  (`conventions::long_term_default_acquired`), but it is a TAX dimension of the era decision and should be chosen
  explicitly. (Owner: **P-D/ship**, rides the era-table decision.)
  ★ **CLOSED — the OWNER required an EXPLICIT PICK (2026-07-26): there is no default preset at all.** On the
  `$0`-declare branch the window's only filing-substantive effect IS the holding period (the basis is `$0` either way),
  so pre-selecting the oldest bucket was the tool answering a filing question in the taxpayer-favorable direction —
  the answered-ness invariant. `DeclareFlowState.preset`/`.window_start` are now `Option`, opening `None`; the flow
  refuses to advance or confirm without a pick (fail-closed at BOTH `review()` and `declare_flow_confirm`); the picker
  mirrors the BG-D5 provenance step (numbered list, "(none yet — press 1-6)", pick preserved across a bounce). The
  DFW-D5 prefill is untouched and independent (`window_end` opens at the before-op day, `wallet` = the source pool).
  KATs (a)-(d) in `declare_flow.rs` + the e2e fail-closed leg in `main.rs::declare_flow_end_to_end_...`; all
  mutation-verified.
- **[done] SPEC line for the provenance step.** The P-C gate added an explicit BG-D5 provenance-selection step to the
  promote flow (tax I-2). No design artifact named a provenance picker either way. CLOSED at the whole-branch fold:
  **SPEC DFW-D12** now carries the step explicitly — an unprompted filer answer (nothing preselects `Purchase`), the
  fail-closed `refuse_non_purchase` gate, why it is BG-D5/§6664(c) substance rather than an inference, and its place in
  DFW-D2's unchanged gate order. (Owner: **P-D/ship**, doc-only — DONE.)

## P-D / whole-branch (deferred at the P-C gate — non-blocking, but sweep before merge)

- **[open — RE-OWNED to post-merge]** ★ **2026-07-26, whole-branch FINAL r2 fold.** The two displacement-caveat
  holes below (the dashboard `[assess]` line and the declare-flow `t` readout — filed here as tax-M-3/tax-M-4,
  and carried as **tax M-4 / M-5** in the r2 tax lens' own numbering) were owned by **P-D/whole-branch**, and
  that phase CLOSES at this gate. They are NOT discharged, so per STANDARD_WORKFLOW ("an item whose owning
  phase has already passed is overdue, not deferred") they are explicitly RE-OWNED to **post-merge / next
  cycle** rather than left pointing at a closed phase. Both lenses ruled them non-blocking: neither changes a
  filed number — each is caveat COPY beside an advisory figure that is already labelled a gain-Δ and "not a tax
  saving". Owner from now on: **post-merge / next cycle.**
- **[open] tax-M-3 — displacement-caveat hole for a correctly-sized cover.** (Owner: **post-merge / next
  cycle** — re-owned above.) `defensive/mod.rs:659-688`:
  `WouldDisplaceIfPromoted` fires only when `covered_sat == 0`; when `covered_sat > 0 && t.sat == covered_sat`, neither it
  nor `OverCovered` fires — yet a HIFO reorder across multi-year disposals still shifts gain between years, so that row's
  per-year delta is a reorder artifact shown as an unqualified saving. Fix: fire on `!promoted && displaces_documented_basis(..)`,
  suppressing only where `OverCovered` already carries displacement copy.
- **[open] tax-M-4 — the declare flow's on-demand tax-Δ carries no displacement caveat.** (Owner: **post-merge
  / next cycle** — re-owned above.) (`declare_flow.rs:293-307` prints
  bare `$delta`/`gain-Δ`). `declare_preview_saving` already builds both folds, so the check is nearly free.
  ★ **PREMISE CORRECTED at the whole-branch fold:** this entry used to justify itself with "…while the dashboard row's
  equivalent number is caveated." That was FALSE (whole-branch tax-M-1) — at the time it was written the dashboard drew
  NO number at all: `TrancheRow.clamped_saving` was computed by `journey_view` and rendered nowhere, so there was no
  caveated sibling to be inconsistent with. As of this fold the dashboard DOES render it (`render_saving_line`, with the
  `WouldDisplaceIfPromoted` caveat above it), so the comparison the entry makes is true GOING FORWARD — but the item's
  real basis is standalone: a bare gain-Δ shown to a filer without its displacement caveat can be read as an unqualified
  saving, wherever it is printed.
- **[done, partial] arch-M-1 — ~35 lines of verbatim duplication between the two confirm tails** (`declare_flow_confirm` vs
  `promote_flow_confirm`) + a third copy of the dashboard refresh in `open_defensive_filing`. The
  **`refresh_defensive_dashboard(app)`** half is DONE (whole-branch fold): extracted in `main.rs` and now the single
  source for both confirm tails AND the export step's own post-re-projection refresh. `EditorApp::open_defensive_filing`
  deliberately keeps its own copy (it takes `&mut self` and must run the DFW-D6 entry gate first). **Still open:** the
  `after_defensive_write(app, status)` half (the save→re-project→status→close-flow tail itself).
- **[done] arch-M-2 — `DeclareFlowState::clearance()` DELETED** (whole-branch fold). It had no non-test caller, and its
  own doc already conceded the real (and only) declare gate is `declare_flow_confirm`'s FRESH `plan_declare` at the
  Confirm-step Enter. Wiring it into the readout would have created a second, drifting gating authority (DFW-D1 forbids
  exactly that) and re-projected per keystroke (DFW-D10 forbids that too), so it was removed rather than wired. Its sole
  test went with it — which also closes **N-r2-4(a)** below. The one other test that called it now calls
  `btctax_cli::plan_declare` directly, i.e. the REAL gate.
- **[done] arch-N-1 — `debug_assert!(open_flow_count() <= 1)` is evaluated before the flow field is set** (so it permitted one
  OTHER flow open); the invariant it names is `== 0`. Tightened to `debug_assert_eq!(.., 0)` at all three sites
  (`main.rs` open_declare_flow / open_promote_flow, `editor.rs` open_defensive_filing) at the whole-branch fold.
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
- **[open] N-r2-4 residue — three r1 Nits neither folded nor filed.** (a) **[done]** tax N-3: the tautological
  `empty_events.len() == 0` assert plus two `let _ = ...` import-keepers in
  `declare_flow::tests::clearance_reflects_plan_declare_and_is_a_pure_read` — closed at the whole-branch fold by
  DELETING that test wholesale alongside the `clearance()` probe it was the sole caller of (see arch-M-2 above).
  (b) tax N-4: the test name
  `defensive_journey.rs::declare_preview_saving_edits_the_window_and_changes_nothing_it_should_not` is misleading —
  the body makes a single call over a fixed window and never edits anything; rename or rewrite to actually exercise
  re-derivation across an edit. (c) the existing "Debug-format rows" item below is now widened to cover the two new
  flow renders.

## Task 10 / P-D (the export step)

- **[done] T3-M2 — `apply_export` has no per-year error isolation.** Fixed: `apply_export`'s return type became
  `Result<ExportOutcomes, CliError>` where `ExportOutcomes = Vec<(i32, Result<IrsPdfReport, CliError>)>` — the outer
  `Result` covers only the "couldn't even reload events/state from `session`" failure; the per-year loop no longer `?`s
  on a single year's failure, so a flagged/current year outside the bundled IRS-form-template set fails LOUD for THAT
  year alone while every other planned year is still attempted (ascending `BTreeSet` order) and already-written years
  stay correct on disk. `defensive_dashboard::render_export_status` (`btctax-tui-edit`) turns the per-year outcome set
  into the "N of M year(s) written … — YEAR failed: reason" `app.status` text. KAT:
  `promote_cli.rs::apply_export_isolates_a_per_year_failure_and_still_writes_the_other_years` (an unsupported year,
  2016, ordered ASCENDING-BEFORE a supported one, 2025 — pins that an EARLY failure does not abort a LATER success).
  (Owner: **Task 10** — DONE.)
- **[done] T3-M1 — per-year `out_dir/<year>/` subdir is an unbriefed layout contract** (decided + KAT-pinned in P-A).
  Fixed: `defensive_dashboard::render_export_status` names EVERY successfully-exported year's own `out_dir/<year>`
  path (not just "done") in the `app.status` NOTICE text; `defensive_dashboard::defensive_export_dir_for` computes the
  shared base directory (mirrors `btctax_tui::export::export_dir_for`'s pure/testable shape, under a DISTINCT
  `btctax-defensive-export-` prefix so a same-second single-year CLI/viewer export can never collide with it). KATs:
  `render_export_status_names_every_year_and_its_own_out_dir_path_on_full_success` +
  `render_export_status_reports_a_partial_success_and_names_the_failing_year_and_reason` (both `defensive_dashboard.rs`)
  + the `main.rs` end-to-end KAT `x_exports_both_a_promoted_2025_leg_and_a_2024_removal_reordered_year_including_form_8275`,
  which reads the real files back off disk under the computed `out_dir/<year>/`. (Owner: **Task 10**, display-only — DONE.)

## P-D / whole-branch FINAL review (both lenses NOT GREEN — folded in one pass before merge)

All four blocking/near-blocking items below landed in the SAME commit; both blockers were in the export step
(Task 10, `0a6cf21`/`9cbd65e`) — the only code on the branch that never faced a two-lens phase gate.

- **[done] tax I-1 (BLOCKING) — the export omitted the year a `$0`-only declare fixed.** `conservative::flagged_years`
  iterated `live_promote_ids` ONLY, so a vault with `DeclareTranche` decisions and no `PromoteTranche` yielded the EMPTY
  set and the wizard's `$0`-branch export planned just `{current_year}`. But a `$0` declare DOES rewrite the shortfall
  year's filed forms (`make_disposal_legs` re-splits the disposal's net proceeds pro-rata across `consumed`, giving the
  uncovered share its own 8949 row with `acquired_at = window_end`, moving the Schedule D short/long split, and clearing
  the Hard `UncoveredDisposal`) — so the CONSERVATIVE branch of the DFW-D3 fork was strictly less complete than the
  aggressive one. Fixed by composing the SHIPPED fold-diff machinery, no new tax logic: `promote_changed_years` was
  generalized to `decision_changed_years` (the criterion is decision-agnostic) and `flagged_years` now unions the
  per-LIVE-DECLARE diff alongside the per-live-promote one — same `< current` retain, same forced-pseudo-off shadow.
  KAT: `promote_cli.rs::flagged_years_includes_the_prior_year_a_zero_dollar_declare_alone_fixed`
  (mutation verified: reverting to the promote-only union yields `{}` → reds). SPEC DFW-D11 amended to state the
  three-part union explicitly. (Owner: **P-D/whole-branch** — DONE.)
- **[done] arch I-1 (BLOCKING) — the export planned from a possibly-STALE snapshot and reported a short packet set as
  full success.** `execute_defensive_export` computed the plan (incl. the year set) from `app.snapshot` while
  `chokepoint::apply_export` re-loads `events`/`state` FRESH from `session` — stale year set, fresh PDF content.
  Reachable whenever a write SAVED but its `build_snapshot` failed (~24 "Saved but re-projection failed" tails, both
  defensive ones included); the "restart to refresh" status could not guard it because the DefensiveFiling key handler
  runs `app.status = None;` before dispatching the very `x` that would act on it. Fixed with the brief's **option (ii)**
  — `execute_defensive_export` RE-PROJECTS immediately before planning, so plan and apply read one image by
  construction; the refreshed snapshot + dashboard are retained. Option (i) (a `snapshot_stale` latch) was built first
  and DISCARDED: the marker turned out to have 24 emitters, so the latch would have needed arming at ~24 sites and
  clearing at ~35, and a stuck flag would refuse valid exports. KAT:
  `main.rs::x_replans_off_a_fresh_projection_so_a_stale_snapshot_cannot_shorten_the_year_set` (mutation verified:
  deleting the re-projection reproduces the exact defect — `"1 of 1 year(s) written"`, 2024 silently absent).
  (Owner: **P-D/whole-branch** — DONE.)
- **[done] tax M-2 — every 2026 filer's `x` reported a FAILURE and left a half-written directory.** `plan_export`
  inserted `current_year` unconditionally while `btctax_forms::SUPPORTED_YEARS = [2017, 2024, 2025]`, so `x` read
  "0 of 1 year(s) written — 2026 failed: unsupported tax year 2026" — and `export_irs_pdf_from_session` had already
  `mkdir`'d the year dir and written `basis_methodology.txt`/`form_8275.txt` before `fill_form_8949` raised. Two fixes:
  (a) `plan_export` partitions the candidate set against `SUPPORTED_YEARS` into `years` / a new
  `ExportPlan::unsupported_years`, rendered as "no bundled IRS templates for `<year>` yet" (with the standing
  amend-by-hand note for a prior year) instead of a failure; (b) `export_irs_pdf_from_session` refuses an unsupported
  year BEFORE `mkdir_out` — mirroring the shipped Form 8275 overflow pre-check — with the byte-identical
  `CliError::FormFill(FormsError::UnsupportedYear(y))`, so the single-year CLI path is unchanged except that it now
  writes ZERO bytes. KATs: `plan_export_holds_an_unsupported_year_out_of_the_fill_set`,
  `render_export_status_reports_an_unsupported_year_as_not_attempted_not_as_a_failure`, plus no-half-write assertions in
  `export_irs_pdf.rs::unsupported_year_is_refused` and the per-year-isolation KAT. (Owner: **P-D/whole-branch** — DONE.)
- **[done] tax M-1 — `TrancheRow.clamped_saving` was computed and drawn NOWHERE.** `journey_view` spent two full
  projections per realized year per unpromoted tranche on it, and `render_tranche_row` never rendered it — so
  `Advisory::WouldDisplaceIfPromoted`'s copy ("any saving/gain-Δ **shown above** would UNDERSTATE the gain…") pointed at
  nothing. Fixed BOTH ways: the figure is now rendered (`render_saving_line`, above the advisories, keeping the BG-D6
  three-flavor discipline — an `Uncomputable` year prints a REALIZED-GAIN delta explicitly labelled "not a tax saving",
  never a bare `$X`), and the caveat copy no longer presupposes a figure ("…on an [assess] line above", since the
  advisory legitimately fires on rows that have no saving years at all). KATs:
  `tranche_row_renders_the_clamped_saving_flavors_without_quoting_a_tax_figure_for_an_uncomputable_year`,
  `the_assess_figure_is_rendered_above_the_advisory_that_caveats_it`. (Owner: **P-D/whole-branch** — DONE.)
- **[done] arch M-1 — `x` silently did nothing when `snapshot`/`session` was `None`.** Both bare `return`s set a status
  now; the screen handler clears `app.status` before dispatch, so a bare return was indistinguishable from a dead key.
  KAT: `x_with_no_loaded_ledger_refuses_with_a_reason_never_a_silent_no_op`. (Owner: **P-D/whole-branch** — DONE.)
- **[done] arch M-2 — `chokepoint::promoted_filing_years` demoted to `pub(crate)`.** It was `pub` solely to serve one
  integration-test assertion, which would have parked it on btctax-cli's v0.10.0 PUBLIC API permanently (much cheaper to
  narrow before the first release than after); its only production caller is in-crate (`cmd/admin.rs`). The
  disposal-legs-only contract is now pinned by an in-crate unit test,
  `chokepoint::tests::promoted_filing_years_enumerates_promoted_disposal_legs_only`. (Owner: **P-D/whole-branch** — DONE.)
- **[open] Stale `app.snapshot` after a failed re-projection — the CLASS, not just the instance.** The whole-branch fold
  closed the one place where it could produce a WRONG FILED ARTIFACT: `execute_defensive_export` now re-projects BEFORE
  planning and REFUSES outright if the ledger will not project (`main.rs:4618-4639`), so the exported year set and the
  packet content always come from the same image — export is immune **by construction**, not by luck. But the class
  remains: `main.rs` carries **26** `"Saved but re-projection failed ({e}) — restart to refresh"` tails (grep the
  literal), each of which leaves `app.snapshot` holding the PRE-write image while the vault on disk has moved on. The
  status tells the filer to restart, but nothing stops them ignoring it and driving another read off the stale
  snapshot (dashboard rows, the declare/promote flows' `plan_*` inputs, `journey_view`). Fix the class, not the
  instance: either invalidate `app.snapshot` (`None`) on a failed re-projection so every reader fails loud instead of
  reading stale, or route all of them through one `after_write` helper that owns the invalidate — which also folds the
  still-open `after_defensive_write` half of arch-M-1 above. **Not a filing-correctness gate today** (no writer
  re-derives a filed number from `app.snapshot` without its own fresh `plan_*`), so this is post-merge-safe.
  (Owner: **post-merge / next cycle.**)

**Explicitly NOT touched (USER decision, handled separately):** the era-preset table content and its default preset
(`crates/btctax-core/src/defensive/era.rs`), including the PROVISIONAL language and the missing ≥2025 bucket — see the
P-D/ship section above. ★ **RESOLVED 2026-07-26** — the owner ruled on all of it (ratified buckets + explicit pick +
`Y2025Onward` + disclaimer stripped); the P-D/ship entries above carry the detail.

## P-D / whole-branch FINAL review **round 2** (arch GREEN "ship it"; tax NOT GREEN 0C/2I — folded in one pass)

Both r2 blockers were **doc-only** and both would have published at v0.10.0. No filed number changed in this
fold: the blockers and tax-M-3 are pure documentation; tax-M-1 makes an export year-set strictly MORE complete
(never less); tax-M-6 is render-only.

- **[done] tax I-1 = arch M-1 (BLOCKING) — a FALSE pooling claim in the SPEC and in a published API doc.** The
  sentence "`pool_key` puts a pre-2025 lot in the Universal pool and a 2025+ lot in its wallet's own pool, so a
  pre-2025 tranche cannot cover a post-2025 disposal in the same wallet" is **FALSE** under Path A (the default,
  i.e. no `SafeHarborAllocation`): `project/transition.rs:93-106` drains every Universal residue lot into
  `PoolKey::Wallet(lot.wallet)` at the cutover and *explicitly preserves* `BasisSource::EstimatedConservative`
  for exactly this case (D-8). It was refuted by a **green test already in the workspace** —
  `kat_tranche.rs::tranche_tag_survives_2025_path_a_seed_and_reaches_a_2025_disposal_leg`, where a 2015-window
  tranche in wallet `w` fully covers a 2025-06-01 sale in the SAME wallet. The claim holds only under Path B,
  which is unreachable here (`guard_tranche_vs_allocation` / `guard_allocation_vs_tranche` make an in-force
  allocation and a pre-2025 tranche mutually exclusive). The owner's ratified DECISION (add `Y2025Onward`) is
  UNCHANGED and correct — only its stated reason was wrong. Replaced with the two SOUND justifications ((a) a
  filer whose coins genuinely are 2025+ must be able to attest truthfully rather than nudging ~150 times; (b) a
  pre-2025 declare permanently forfeits Rev. Proc. 2024-28 eligibility via `pre2025_tranche_exists` →
  `guard_allocation_vs_tranche`) in **all four** places — whole-surface sweep, this repo's own lesson:
  `crates/btctax-core/src/defensive/era.rs` (the docs.rs publish surface),
  `crates/btctax-core/tests/defensive_era.rs`, `SPEC.md` DFW-D9, and the `main.rs` T10 fixture comment.
  (Owner: **P-D/whole-branch** — DONE.)
- **[done] tax I-2 (BLOCKING) — the README stated a safety guarantee the engine does not provide.** "the floor …
  will never manufacture a loss, and **it will never exceed basis you can already document**". The first clause
  is TRUE (`clamped_leg_basis`, `conservative_promote.rs:179-192`, bounds the estimate at `net − documented`).
  The second was **UNBACKED**: nothing compares the computed floor to documented basis; `plan_promote` gates only
  on provenance, a non-empty Part II, `Coverage::Full`, consent and the ack. Counter-example: a documented 2013
  buy at ~$100/BTC + a declared 2021-01-01..2024-12-31 window → a floor of order $15,000/BTC, ~150× documented,
  and nothing refuses it — which is precisely why `Advisory::WouldDisplaceIfPromoted`/`NowDisplacing` ship. The
  false clause was deleted and replaced with what IS guaranteed (the estimate never absorbs proceeds the
  documented component needs, so it can never manufacture a loss; and the promote is refused outright for every
  provenance that already has a real basis in law), plus an explicit note that a wide window CAN exceed
  documented basis and that the dashboard says so. (Owner: **P-D/whole-branch** — DONE.)
- **[done] tax M-3 — the rest of the README accuracy cluster (same section).** (1) "the tax delta where it can be
  computed": `journey_view` passes `profile: None`, so `SavingFlavor::ComputedTax` is structurally UNREACHABLE on
  the dashboard — every `[assess]` line is the gain-Δ flavor, explicitly labelled "not a tax saving". Reworded,
  and the real dollar-tax route (the declare flow's on-demand `t`, which sources the stored `TaxProfile`) named
  separately. (2) "which years your **promotion** actually changed": after the declare-side fold the set also
  covers years a `$0` DECLARE changed — the prose reproduced in words the very branch asymmetry the code fix
  removed. (3) "writes one packet per year": `SUPPORTED_YEARS = [2017, 2024, 2025]`, so for the wizard's core
  audience (a lost-records sale in 2018–2023) `x` writes NO packet and reports "no bundled IRS templates … amend
  by hand" — a paragraph now says so. (4) the §6664(c) sentence no longer claims the record "matches the screen
  you agreed to" (the open no-scrolling follow-up means a trailing term can be recorded as shown without being
  rendered); it now claims only what is true — the consent text is recorded verbatim.
  (Owner: **P-D/whole-branch** — DONE.)
- **[done] tax M-1 — a still-in-force PROMOTED tranche could be dropped from the export union.**
  `conservative.rs`'s `live_declare_ids` built liveness from the NAIVE `voided_decision_targets` ("some
  `VoidDecisionEvent` names this id"), but `project/resolve.rs:627-638`'s BG-D9 deferred adjudication makes a
  void of a `DeclareTranche` **INERT** when a live promote references it (the tranche stays in force; the void
  raises a `DecisionConflict`). Fixed with `!voided.contains(&e.id) || state.promoted_origins.contains(&e.id)` —
  `promoted_origins` IS the resolver's settled verdict — so the correction can only ADD years, never remove one.
  KATs: the white-box, mutation-verified
  `conservative::tests::live_declare_ids_keeps_a_tranche_whose_void_the_engine_held_inert`, plus the end-to-end
  `promote_cli.rs::flagged_years_keeps_a_promoted_tranche_whose_declare_void_the_engine_made_inert`. ★ Noted in
  both docs: the END-TO-END test is deliberately not the mutation-killer — on that shape the promote half of the
  union coincides, because its own shadow fold removes the promote, which un-defers the very void and drops the
  tranche anyway. That coincidence is a property of the CURRENT resolver, not a guarantee, hence the white-box
  pin. (Owner: **P-D/whole-branch** — DONE.)
- **[done] tax M-6 — the oldest era bucket rendered no holding date and no term.** `declare_flow.rs:411-444`
  rendered the holding date + `({term} at the short op's date)` ONLY in the `Some(Ok(cf))` (`Coverage::Full`)
  arm. The bundled price data starts **2010-07-17**, so `Y2009To2011` (from the 2009-01-03 genesis block) is
  ALWAYS `Coverage::Partial` → the `Err` arm → no holding date, no term — on the oldest bucket, the one a
  lost-records filer is most likely to pick and the one that most reliably makes the disposal LONG-term, while
  the new prompt copy promises the pick "sets … whether the disposal it covers is SHORT- or LONG-term". The
  line was hoisted out of the floor match and now renders whenever `window()` is `Some`. Render-only —
  `window_end` IS the acquisition date either way. KAT (mutation-verified, covers Partial + NoCoverage + the
  still-fail-closed no-pick case): `declare_flow::tests::the_holding_date_and_term_render_even_when_the_floor_is_not_computable`.
  (Owner: **P-D/whole-branch** — DONE.)
- **[done] arch M-2 — `era::next_preset` deleted** (see the residue section below for the full disposition).
- **[done] arch Nit-N1 — `[T; N]` public constants narrowed to `&'static [T]` BEFORE the first publish.**
  `era::ALL_PRESETS: [EraPreset; 6]` and `ProvenanceKind::ALL: [ProvenanceKind; 7]` baked the length into the
  **public type**, so adding a variant would have been a breaking change — and the census drift guard
  (`the_newest_era_preset_reaches_the_newest_filable_tax_year`) actively SCHEDULES the next length change for
  whenever a new filing year is bundled. Both are now slices; call sites/KATs updated. (Owner:
  **P-D/whole-branch** — DONE.)
- **[done] arch N5 — `declare_flow_confirm`'s doc cited `flow.clearance`,** deleted in the previous wave.
  Corrected to name the real gate (`btctax_cli::plan_declare`, re-run fresh at the confirm tail) and to record
  why the probe went. (Owner: **P-D/whole-branch** — DONE.)
- **[open] arch M-3 — two PUBLIC `render_consent` functions on btctax-cli's about-to-be-published API.**
  `cmd/promote.rs:186` (`render_consent(terms, gift_only_years) -> String`) and `chokepoint/mod.rs:438`
  (`render_consent(plan: &PromotePlan) -> String`). Same name, same crate, different signatures and different
  scopes — a caller can reach the narrower one and miss the advisory/post-consent material the chokepoint copy
  carries, which is exactly the "second consent surface" shape DFW-D1 exists to prevent. Not a defect today (the
  wizard and the CLI both go through the chokepoint one). Rename or narrow one before/at the first publish.
  (Owner: **post-merge / next cycle**, ideally BEFORE the v0.10.0 publish.)
- **[open] arch M-5 — the DECLARE path can plan off a stale image, the same class the export fix closed by
  construction.** `execute_defensive_export` now re-projects before planning (r1 arch I-1), but the declare and
  promote flows still read `app.snapshot` for their `plan_*` inputs, and `main.rs` carries **26**
  `"Saved but re-projection failed ({e}) — restart to refresh"` tails (grep the literal), each leaving
  `app.snapshot` on the PRE-write image. Not a filing-correctness gate today — both confirm tails re-run their
  own FRESH `plan_declare`/`plan_promote` against `session` at the Enter, so no filed number is derived from the
  stale image — but the SHOWN readout (floor/coverage/tax-Δ, dashboard rows) can lag. Same remedy as the
  standing class item: invalidate `app.snapshot` on a failed re-projection, or route every write through one
  `after_write` helper that owns the invalidate (which also folds the open `after_defensive_write` half of
  arch-M-1). Duplicate-safe: this is the same root cause as "Stale `app.snapshot` after a failed re-projection —
  the CLASS" above; kept as a separate line only because r2 named the declare path specifically.
  (Owner: **post-merge / next cycle.**)
- **[open] ~22 remaining stale-snapshot tails.** Subsumed by the CLASS item above (26 literal sites today); no
  separate work item — burn them down with the `after_write` helper. (Owner: **post-merge / next cycle.**)

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
- **[done] `era::next_preset` is production-dead — DELETED** (whole-branch FINAL r2 fold, arch M-2). The owner's
  explicit-pick decision replaced the Declare flow's Tab-cycling with a numbered picker, so nothing in production
  called it. It had been retained on the rationale "already-published API" — which was **FALSE**: `defensive::era`
  is ABSENT from `main`, so it has never been published, and narrowing the surface before the first release is far
  cheaper than after. Deleted with both of its KATs (`era.rs::next_preset_cycles_and_wraps_to_first`,
  `defensive_era.rs::next_preset_cycles_oldest_to_newest_then_wraps`), matching exactly how `clearance()` was
  handled. (Owner: **P-D/whole-branch** — DONE.)
- **[open] tax Nit 2** — the Provenance screen states which answer passes the gate BEFORE the filer answers ("Only a
  PURCHASE can be promoted…" renders above the picker) — honest disclosure of the rule (same wording
  `refuse_non_purchase` uses), but mildly leading on a screen whose purpose is an unprompted filer answer. Consider
  moving that sentence into the refusal path.
- **[open] Free-text date/sat entry** (T8-review Nit) — the declare flow edits via nudge (±1d/±1000 sat) on top of the
  numbered era pick (`1-6` — the Tab-cycling this entry originally named was replaced by the owner's explicit-pick
  decision); free-text entry, if wanted, is a contained `declare_flow.rs` follow-up.
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
