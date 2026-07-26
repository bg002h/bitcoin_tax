# P-C PHASE GATE — architecture lens (Opus) — persisted verbatim before folding

Scope: Tasks 8–9 + `badfae4` Minor-burndown. Base `8074a3e` → head `badfae4` (4 commits).
Reviewer: Opus, architecture lens, 30 tool uses (manually replicated the KAT-G1 scanner).

## Verdict

**NOT GREEN — 0 Critical / 3 Important / 4 Minor / 4 Nit**

The named load-bearing risk (write confinement) is **clean** — verified mechanically, not by eye. The three Importants are elsewhere: one collapses a ★ binding SPEC guarantee on the very write path this phase introduces, one makes every refusal/error the phase produces invisible to the filer, and one leaves the feature's sole entry point undiscoverable.

---

## ★ Named-risk check: WRITE CONFINEMENT (KAT-G1) — PASS

Replicated `kat_g1_mechanized_source_gate`'s scanner exactly (walk `crates/btctax-tui-edit/src`, split at the first `#[cfg(test)]`, strip `//`, apply `everywhere_tokens` + `fs_write_tokens` to all files and `persist_only_tokens` to all files except `edit/persist.rs`) over the shipped tree: **0 violations.**

- `apply_declare(` — exactly one real call site, `edit/persist.rs:443`, inside `persist_declare_tranche`. Every other hit is a doc comment or the gate's own runtime-constructed token.
- `apply_promote(` — exactly one real call site, `edit/persist.rs:471`, inside `persist_promote_tranche`. Same.
- `declare_flow.rs` / `promote_flow.rs` contain **zero** `apply_*` call tokens (each also carries its own belt-and-braces `include_str!` grep guard).
- `persist_only_tokens` gained **both** `"apply_declare("` and `"apply_promote("` (`persist.rs:2030-2031`), the self-check plants **both** (`:2224-2225`) and asserts detection of **both** (`:2257`, `:2261`) — so the gate cannot silently drop either.
- `edit/mod.rs:7-11` guarantee text amended to name both new tokens and both confining wrappers.
- **No `cmd::` leak:** both flows reach the chokepoint via the `btctax_cli` crate-ROOT re-exports (`lib.rs:44-49`, `62-86`). The only `cmd::` occurrences in the two new files are one doc comment (`promote_flow.rs:31`) and test-region fixtures — both invisible to the scanner.

Seam integrity also holds: `defensive/era.rs` is pure core (`conventions::TaxDate` + `time` only, no cli symbol); `declare_preview_saving` takes `&dyn TaxTables` + `Option<&TaxProfile>` (core trait, never `BundledTaxTables`); both flows are collect-and-read drivers with no second gate.

---

## Findings

### Important

**I-1 — the TUI promote driver hands the chokepoint a hardcoded ack, so BG-D6's gate is a tautology on this path**
`crates/btctax-tui-edit/src/main.rs:4420`

```rust
crate::edit::persist::persist_promote_tranche(
    session, *plan,
    Some(btctax_cli::PROMOTE_ACK_PHRASE),   // ← the CONSTANT, not what the filer typed
    now,
)
```

`handle_promote_flow_consent_key` (`main.rs:4346-4381`) compares the typed buffer itself and returns early on mismatch, then `promote_flow_confirm` passes the canonical constant. `require_promote_ack` (`chokepoint/mod.rs:301`) therefore **can never refuse** on the TUI path — the only gating authority is the driver's own compare.

This directly contradicts SPEC DFW-D2 ★ *BG-D6 ack residency*: "Drivers only **collect** the phrase — they NEVER validate it — so BG-D6's enforcement point stays single-sourced in the chokepoint, not scattered across N drivers", and SPEC §5's "a driver cannot append without a correct phrase reaching `apply`". It also makes the shipped doc comment at `main.rs:4324-4325` false: *"the REAL fail-closed gate is `apply_promote`'s own `require_promote_ack` … this pre-check is a UX nicety … not a second gating authority."* As written the pre-check **is** the gating authority. The CLI driver (`cmd/promote.rs`) forwards the user's `--i-acknowledge` value straight through, so the two drivers now differ in *where* the gate lives.

Failure scenario: any future loosening of the driver compare (drop the `trim`, add a case-insensitive match, add a "skip ack if re-confirming" shortcut) records a `PromoteTranche` with no correct phrase ever reaching `apply`, and no engine gate catches it. Nothing reds — the existing tests pin the driver's compare (`main.rs` e2e) and, separately, the chokepoint's compare (`promote_flow.rs::wrong_ack_phrase_refuses_fail_closed_and_records_nothing` calls persist directly); **the wiring between them is unpinned**.

Fix: extract the typed buffer in `promote_flow_confirm` (it already `mem::replace`s the `Consent` variant — take `ack` alongside `plan`) and pass `Some(typed.as_str())`. Keep the pre-check as a genuine UX nicety, or drop it entirely and let the chokepoint's own (byte-identical) message drive the inline error — note `require_promote_ack` already trims, so semantics are unchanged either way. Add the SPEC-named KAT: neuter the driver compare and assert the write still refuses.

---

**I-2 — `app.status` is never rendered on the DefensiveFiling screen, so every refusal reason this phase produces is invisible**
`crates/btctax-tui-edit/src/draw_edit.rs:92-145` (`draw_defensive_filing`), `main.rs:486`

`draw_defensive_filing` renders a `Block` plus exactly one `Paragraph` (flow render or dashboard render) and returns. It never threads `app.status`. `app.status` is rendered **only** in the Browse footer (`draw_edit.rs:256`) — and this codebase already has a reviewed rule for exactly this (the "★ I-2" precedent at `draw_edit.rs:274-276` and `:2304`: full-frame overlays MUST thread `app.status` in, *"every in-flow refusal/error/outcome routed there stays VISIBLE"*). The DefensiveFiling screen was read-only in P-B, so the omission was cosmetic; **P-C turns it into a write surface and the omission becomes load-bearing.**

Silently swallowed today:
- `main.rs:4157` — `"declare refused: {err}"`, set immediately before bouncing to the Edit step. This is DFW-D5's mandated *"a refusal with a reason, not a silent append"*. The filer sees the flow re-render, unchanged, with no reason at all.
- `open_declare_flow` (`:3988`) — "that declare candidate is no longer present…", "this shortfall carries no wallet on record…"
- `open_promote_flow` (`:4229`) — "that tranche is no longer a promote candidate…"
- both success statuses, and "Saved but re-projection failed ({e}) — restart to refresh"
- **worst:** `on_persist_error`'s `ResidueLive` arm (`main.rs:697-704`) sets the `"CRITICAL: a save failed and could not be reverted … Quit the editor NOW"` status and calls `close_all_mutation_surfaces()` — which does **not** change `screen`. The filer stays on DefensiveFiling, sees nothing, and `main.rs:486` (`app.status = None;`) destroys the message on the very next keypress. The `rollback_failed` latch survives, so the next mutating opener re-warns — but the immediate CRITICAL notice is provably unreachable.

No test catches this: the e2e tests assert `app.status` (the field), never the render, and `render_declare_flow`/`render_promote_flow`/`render_dashboard` take no status parameter.

Fix: give `draw_defensive_filing` a footer/NOTICE line fed from `app.status` (mirror `draw_tax_inputs_form`'s I-2 threading), or pass `app.status.as_deref()` into the three render fns. Add a render-level assertion (the refusal text appears in the rendered lines) so the mutation "stop threading status" reds.

---

**I-3 — Browse `w` was added without updating the KEYMAP overlay it sits directly above**
`crates/btctax-tui-edit/src/main.rs:473-474` vs `draw_edit.rs:2700-2722`

```rust
KeyCode::Char('w') => app.open_defensive_filing(),
// KEEP IN SYNC with KEYMAP overlay (draw_help_overlay). `?` opens the full-keymap help.
```

`draw_help_overlay` lists every other Browse binding (`c/o/r/f/v/S/d/L/u/m/i/z/e/a/A/b/B/C/V/I/O/P/p/T/?`) and does **not** list `w`. Neither does the Browse footer hint (`draw_edit.rs:259-260`). `w` is the **only** entry point to the entire Defensive Filing Wizard, so the feature is undiscoverable in-product; the in-source sync contract was broken by the edit immediately adjacent to it. There is no keymap-sync test — grep confirms `draw_help_overlay` has exactly one caller and zero test references.

This is the recurring whole-surface-sweep failure: `T7-entrykey` was closed `[done]` in FOLLOWUPS with "Browse `w` → `EditorScreen::DefensiveFiling` (+ pseudo-refusal); KAT'd" — the KAT pins the key handler, not the surface sweep.

Fix: add `w defensive-filing wizard` to the `hdr("Reconcile")` (or App) block in `draw_help_overlay`, and add a cheap sync guard — a test asserting every `KeyCode::Char(_)` arm in the Browse match appears in the overlay text, or at minimum that the overlay contains `"w "`.

---

### Minor

**M-1 — ~35 lines of verbatim duplication between the two confirm tails; a third copy of the dashboard refresh**
`main.rs:4160-4203` (`declare_flow_confirm`) vs `main.rs:4425-4468` (`promote_flow_confirm`)

The entire post-write tail — `build_snapshot` → clear the flow field → set `app.snapshot` → set a status → recompute `journey_view` → rebuild `DefensiveDashboardState` → the `Err` re-projection arm → the `on_persist_error` arm — is byte-identical apart from the flow field name and the status string. The `journey_view` + `DefensiveDashboardState::new` construction is additionally a third copy of what `EditorApp::open_defensive_filing` (`editor.rs`) does, and the two confirm-path copies omit that opener's `pseudo_active()` gate (harmless today — a declare/promote cannot turn pseudo on — but it is a divergence that a single helper would eliminate structurally). Fix: extract `fn after_defensive_write(app: &mut EditorApp, status: &str)` and have both confirms call it; have it reuse one `refresh_defensive_dashboard(app)` shared with the opener.

**M-2 — `DeclareFlowState::clearance()` is production-dead, and the confirm path re-implements the same `plan_declare` call**
`crates/btctax-tui-edit/src/edit/declare_flow.rs:168` / `main.rs:4141-4151`

`.clearance(` has exactly two call sites, both in `declare_flow.rs`'s own `#[cfg(test)]`. `declare_flow_confirm` builds the identical nine-argument `btctax_cli::plan_declare(…, Some(target_event), now)` call inline instead of calling it — two sources for the same chokepoint invocation, so a future change to one (say, threading the era clamp or a different target) silently diverges. Its doc also claims the refusal is *"surfaced live rather than discovered only at a final Enter (DFW-D5)"*, which the shipped `render_declare_flow` never does (it renders the cheap trio only — correct per DFW-D10, but the doc overstates). Fix: call `flow.clearance(...)` from `declare_flow_confirm` (one source), and correct the doc to say the clearance runs at confirm.

**M-3 — a persist error discards the filer's authored Part II narrative even when nothing was written**
`main.rs:4470-4473`

```rust
Err(e) => { app.promote_flow = None; app.on_persist_error(e); }
```

`PersistError::NoChange` means the vault is untouched — reachable via `apply_promote`'s internal `would_conflict` (BG-D9) check, which runs after the ack. The filer loses a multi-paragraph Form 8275 narrative with no recovery path (unlike the CLI, which reads Part II from a file). The flow is otherwise scrupulous about preserving authored work (buffer lives outside `step` precisely for this). Fix: on `NoChange`, keep the flow open and bounce to `PartII { error: Some(...) }`; close only on `RolledBack`/`ResidueLive`. (Combines well with I-2 — the message is currently invisible either way.)

**M-4 — a misnamed test gives false confidence on the T6-Minor1 property**
`crates/btctax-tui-edit/src/edit/declare_flow.rs`, `compute_tax_delta_without_a_profile_yields_uncomputable_never_a_bare_dollar`

The name asserts the no-`TaxProfile` → non-dollar guarantee, but the fixture uses `StaticPrices::default()` (no coverage), so `declare_preview_saving` short-circuits to `SavingFlavor::Named` before `profile` is ever consulted — and the assertion matches `Named(_)` accordingly. The real property *is* covered at core level (`defensive_journey.rs::declare_preview_saving_is_uncomputable_without_a_profile`), so this is false-confidence rather than a hole. Fix: rename to `…_without_price_coverage_yields_named`, or give it full coverage + `None` profile so it tests what it claims.

---

### Nit

**N-1** — `debug_assert!(app.open_flow_count() <= 1, …)` at `main.rs:4023` and `:4250` is evaluated *before* the flow field is set, so it permits one *other* flow to be open; the M-4 invariant it names is `== 0`. It mirrors the P-B precedent (`editor.rs:493`), so it's convention-consistent — a one-line tightening at all three sites.

**N-2** — Both flows add filer-facing `{:?}` Debug rendering: `"Declare — covering shortfall {:?}"`, `"wallet: {:?}"`, `"era preset: {:?}"` (renders `Y2009To2011`), `"Promote — tranche {:?}"`. The existing ownerless "Debug-format rows" follow-up names only `render_candidate/tranche/pool_short/resolve_first_row` — extend its scope to the two flow renders.

**N-3** — `declare_flow.rs::clearance_reflects_plan_declare_and_is_a_pure_read` asserts `empty_events.len() == 0` after the call, which is tautological (`clearance` takes `&[LedgerEvent]`), and closes with two `let _ = …` statements whose stated purpose is keeping imports used. Drop both.

**N-4** — `declare_preview_saving_edits_the_window_and_changes_nothing_it_should_not` promises a change-detection property; the body asserts only that a wider fully-covered window still returns `ComputedTax`. Rename or strengthen.

---

## Adjudication 1 — deferring the era-table content to the user

**Sound for closing P-C, with one correction to make.**

Architecturally it binds nothing. `era.rs` is a total, pure `enum → (TaxDate, TaxDate)` table with `ALL_PRESETS` as the single enumeration and `next_preset` derived from it; changing labels, dates, or the bucket count touches one file plus its two KATs, with no type or seam change anywhere. Filing-neutrality is real and checkable: `DeclareFlowState::new`/`cycle_preset` only *seed* the window, the DFW-D5 before-op clamp always governs on conflict, `plan_declare(Some(target))` validates whatever window the filer confirms, and the filed floor comes from `filed_basis_for` requiring `Coverage::Full`. No preset value reaches a filed number. `all_presets_end_strictly_before_the_pre2025_pooling_cutover` pins the one property that *is* structural (the safe-harbor/pooling boundary) and survives any table revision. So this is a product/copy decision, correctly the owner's, correctly tracked, and it does not gate P-C's architecture.

Two corrections to fold before closing:

1. **Remove the self-imposed gate.** `era.rs`'s module doc asserts "per the standing workflow's 'phase-owned follow-up' rule it must be burned down before the P-C gate closes" — but the item is a *user* decision the phase cannot discharge, so as written the phase is gated on something it cannot satisfy. Re-own the FOLLOWUPS entry to **P-D/ship** (owner decision, before ship) and amend the module doc to match. Leaving both texts as-is makes the gate unclosable by construction.
2. **Widen the item's blast radius.** It currently names only `era.rs`. Blessing/changing the table must also update `crates/btctax-core/tests/defensive_era.rs::era_window_maps_every_preset_to_a_concrete_window` (pins all five windows verbatim) and `…_is_a_pure_total_function_over_every_variant` (pins `ALL_PRESETS.len() == 5`), plus the filer-facing preset label (see N-2 — `{:?}` currently prints `Y2009To2011`, which DFW-D9's "copy-level review rigor" clause covers).

## Adjudication 2 — the two `badfae4` Minor fixes

**Both correctly scoped. Verified against source, not just the commit message.**

**Consent attestation echo — DISPLAY-ONLY, confirmed.** The added line lives solely in `render_promote_flow`'s `Consent` arm (`promote_flow.rs`, `format!("You attest: {}", btctax_cli::PROVENANCE_TEXT)`), which is the flow's own pure `Vec<String>` builder. The diff touches **no** file under `crates/btctax-cli/src/chokepoint/` — and I confirmed `chokepoint::render_consent` is built purely from `plan.advisory_lines` + `render_consent_terms(&plan.terms, &plan.gift_only_years)` + `plan.post_consent_note`, with no `PROVENANCE_TEXT` anywhere in it, so the fix's stated rationale is factually correct. The recorded artifact is untouched: `provenance_text` is set inside the plan builder (`chokepoint/mod.rs:414`) and `shown_terms` comes from `consent_terms`, neither of which the echo reaches. The standing proof is the parity KAT `tui_promote_records_an_acknowledgment_eq_identical_to_the_cli_driver`, which compares the full recorded `Acknowledgment` across drivers — it would red the instant the echo leaked into the chokepoint. The new KAT additionally pins *adjacency* (echo above the ack prompt), which is the point of the fix. Correct, and it genuinely improves answered-ness: the fixed `ProvenanceKind::Purchase` is now restated at the moment of consent rather than only two screens earlier.

**`nudge_window_start` clamp — an input bound, not a gate, confirmed.** The change is confined to `DeclareFlowState::nudge_window_start`: floor at `era_window(ALL_PRESETS[0]).0` (2009-01-03 — reuses the date already governing the oldest preset; no new date invented, no second table) and cap at `self.window_end`. It introduces no refusal, no new predicate, and does not touch `plan_declare` — DFW-D1's "no second gating authority" holds, and the FOLLOWUPS entry's filing-safety reasoning (the unclamped state was already surfaced live as `NoCoverage` and refused at confirm) is correct. Worth recording that the sibling asymmetry is deliberate and documented: `nudge_window_end`'s clamp at the DFW-D5 before-op boundary *is* a filing invariant (the lot must exist in time to cover the short op) and predates this commit; `nudge_window_start`'s is purely UX robustness. The KAT exercises both bounds in one test.

---

## Also verified sound (no finding)

- **Promote reuses the review-time plan; declare re-runs fresh.** Not an inconsistency. SPEC DFW-D2 N-4 blesses reuse under the one-flow invariant + single-threaded loop, with `would_conflict` inside `apply` as the backstop — and for promote it is *required*: recomputing at the final Enter could record `shown_terms` differing from the consent screen the filer actually read, breaking the §6664(c) "the artifact equals what the filer saw" contract. An `Esc` round-trip out of Consent forces a fresh `plan_promote` on the next `Tab`, so a stale plan cannot survive an edit.
- **`declare_preview_saving`'s `compute_tax_year(events, &with_state, …)` / `(events, &without_state, …)` pairing** mirrors the shipped `clamped_saving_for` (`defensive/mod.rs:342-343`) exactly — same convention, not a new mismatch.
- **`displaces_documented_basis` extraction** correctly factors the composition-vs-leg-Vec distinction into one place shared by `now_displacing` and `would_displace_if_promoted`, which is precisely the drift the ★ tax-M negative KAT guards.
- **Dashboard intents cannot mis-target a write.** `handle_defensive_dashboard_key` emits `Declare` only from a `DashRow::Candidate` and `Promote` only from a `DashRow::Tranche` with `status == DeclaredZero`; both openers then re-verify the target against the dashboard's own already-computed `journey_view` rather than re-deriving it.
- **Flow-state discipline** otherwise holds: both fields are in `open_flow_count()` and in `close_all_mutation_surfaces()`; both openers check `residue_latch_status()`; both are dispatched in the flow layer ahead of form/screen dispatch, so `q`/`Esc` never fall through to a quit arm.
