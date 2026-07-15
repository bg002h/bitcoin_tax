# Whole-branch review — plan 3 (TUI), independent Fable pass r1

*Persisted VERBATIM (STANDARD_WORKFLOW §2). Reviewer: Fable. Persisted 2026-07-15 against HEAD `e6b9435`
(base `0e9d8aa`, 11 task commits, ~5200 insertions in `btctax-tui-edit`). The plan-3 whole-branch gate.*

---

VERDICT: 0 Critical / 4 Important

All file:line references are at the head commit e6b9435 unless noted.

## Strengths

The four hard vault-safety properties all hold: zero leaf-name reads, zero persist-seam bypasses, zero `ReturnInputs` constructions, and no path that writes a committed row except through the store's own screen gate. The engine/store/frozen-core diffs are literally empty. The `descent` nav model is genuinely sound — `active_pane` is a single source of truth shared by render and nav, so the render/nav desync class is eliminated by construction. Test discipline is strong (fresh-Session disk read-backs, mutation-checked guards, the self-checking leaf scanner).

## Issues

### Important

**I-1. Commit and park never re-project the Browse snapshot — the Tax/Forms tabs show the pre-mutation liability until restart.**
`crates/btctax-tui-edit/src/main.rs:1215` (`commit_tax_inputs`, `Committed` arm, closes flow at :1251) and `main.rs:1381` (`confirm_park_to_profile`, refreshes only `active_source_label`). Both mutations change the year's resolve outcome — `build_snapshot` (`crates/btctax-tui/src/unlock.rs:171`) computes per-year `profiles`/`refused` via `resolve_all_screened`, whose own P2-C1 comment says "the viewer never shows a different liability … than `report`". Every other mutating flow in this crate re-projects on success (~12 call sites; the directly comparable side-table write, profile save, does at `main.rs:492-506`). Failure scenario: filer commits a full return (`T`→`s`→Enter), lands back on the Tax tab, and reads the OLD tax-profile-derived (or missing) liability for that year — in a tax app, a displayed-wrong-number defect. Same after park (Tax tab keeps the full-return numbers although the profile is now active). **Fix:** in the `Committed` arm and the park `Ok(())` arm, rebuild via the existing block-scoped `build_snapshot` pattern with the "Saved but re-projection failed — restart to refresh" fallback. (Discard and autosave correctly need no re-projection — drafts are invisible to resolve.)

**I-2. Every `app.status` message set while the flow stays open is invisible — the overlay clears the full frame over the footer that renders status.**
`draw_tax_inputs_form` clears the whole area (`draw_edit.rs:1928`), covering the Browse footer that is the only renderer of `app.status` (`draw_edit.rs:196`); every other overlay in the crate is a centered rect that leaves the footer visible. But Tasks 6–8 route these outcomes to `app.status` while the flow remains open: the `t`-dirty park refusal (`main.rs:1293`), the park `Err` refusal (:1417 — comment claims "the refusal is in the status"), `NoTables`, commit save `Err`, "choose a filing status first" (:1183), "no parked draft to discard", discard `Err` (incl. the P2-a state — "the message stays reachable" is false), reinstate messages, and every autosave-failure error from `flush_tax_inputs_draft` (:915). Failure scenarios: a filer on a non-2024 year presses `s`→Enter — the modal closes, nothing visible explains that nothing committed; a park refused over a flushed WIP draft looks identical to a successful park except the small source label. **Fix:** render `app.status` as a line in `draw_tax_inputs_status` (`draw_edit.rs:2170` — the 4-row region has room), or route in-flow outcomes to a flow-local `form.error`/notice the pane already renders.

**I-3. `q`/`Esc` close the flow even when the final flush FAILS, dropping the in-memory edits.**
`main.rs:1013-1037`: both arms call `flush_tax_inputs_draft` (on `Err` it sets status — invisible per I-2 — and leaves `dirty` set), then unconditionally `app.tax_inputs_form = None` (:1021, :1036). The q-arm comment promises "so `q` never loses work" and the legend prints "close (autosaved)" unconditionally. On a `Vault::save` failure (disk full), everything since the last good flush is dropped, destroying the recovery the retry latch was built for — and the crate's own error pattern is "Err → keep form open (buffers intact)". Compounded by I-2: the preceding idle-tick flush failures were never visible, so the filer had no warning. **Fix:** make the close conditional — after the flush, if the flow is still `dirty` (and `working.is_some()`), keep the flow open and surface the error; close only when clean (or on an explicit second press).

**I-4. The §9A `!` refusal glyph and the status-line "screen status" segment never shipped, and no follow-up records the drop.**
Plan Task 2 (`design/IMPLEMENTATION_PLAN_input_form_tui.md:104`) commits the glyph set "`✓`/`…`/`!` a screen refusal attributes here (**Task 7 wires `!`**)" and a status line carrying "the screen status"; SPEC §9A (~L588-599) requires both (`!` glyph; "screens clean… / 1 issue: <refusal>"). Task 7's own step list never picked either up: the shipped `section_glyph` (`draw_edit.rs:2483`) returns only `✓`/`…` while its doc comment still says "(Task 7 adds `!` for a screen refusal)", and `draw_tax_inputs_status` has no screen-status segment. FOLLOWUPS.md has no entry. Not data-unsafe — the refusal is surfaced via the focus jump + inline `form.error` — but it is a spec-named feature dropped in the seam between tasks. **Fix:** wire `!` (retain the refused anchor's section from `focus_refusal`; clear on the next successful apply/commit) and a screens-clean/1-issue status segment — or amend §9A and file the follow-up with an owning phase.

### Minor / Nit (recorded, non-gating)

- `TaxInputsModalState.shadows` is production-dead (only tests read it; the summary embeds the warning).
- On persistent flush failure the idle tick retries a full vault re-encrypt every ~100ms.
- `reinstate_parked_full_return` (`main.rs:1346`) labels any `Loaded::Draft` "the parked full return" even if `parked=false` (unreachable in-session under the exclusive lock).
- The legend's "(autosaved)" is printed even on a `None` working copy and after failed flushes; `Esc` mid-row pops a level, not "close" as the legend implies.
- `value_is_answered` treats `Money(0)`/`Bool(false)` as unanswered — a deliberately-zero field pins the glyph at `…` (cosmetic).
- `seed_string` through the 64-byte `FieldBuffer` cap would silently truncate a longer externally-imported Text value on re-commit (v1 fields are short in practice).

## Whole-branch seams

1. **Never-name-a-leaf: HELD.** Independent grep across non-test regions of `main.rs`, `edit/tax_inputs.rs`, `edit/form.rs`, `draw_edit.rs` at e6b9435: zero `ReturnInputs` leaf reads. The only `ri.`/`.filing_status` hits are the pre-existing reconcile flow's `ReclassifyIncome` payload variable (`main.rs:3486,3715`), the profile form's `TaxProfile` fields, and doc comments. Leaf names appear only in `#[cfg(test)]` code, which the constraint permits.
2. **NI-2: HELD end-to-end.** All three `Some(working)` producers are `apply` materialization or store loads (opener :773, reinstate :1346); zero `ReturnInputs{..}`/`::default()` in non-test flow code (grep); on `None`, every mutating key is provably inert except the filing-status cycle; render on `None` shows only the filing-status choice in every state incl. P2-a.
3. **Persist seam: HELD.** Zero `conn(`/`save(`/direct `input_form_store::` tokens in non-test flow code; all writes go through the seven `edit/persist.rs` wrappers; disjoint-field borrows are consistently scoped (and the crate compiles, which enforces them).
4. **Secrets: HELD.** Entry renders bullets keyed on the focused field's `FieldKind::Secret`; focus cannot move mid-edit (edit branch swallows nav keys); `seed_string` never seeds a Secret; failed secret commits keep rendering bullets; `get` yields masked `SecretView`s everywhere (previews included); modal summaries carry counts only; parse/apply error strings are static. No digit path to any `Line`/`Span` found.
5. **Commit/toggle/discard data-safety: HELD at the vault level** (no refused/NoTables write path; park is store-atomic + TUI dirty-gated; `X` double-gated on `parked`; frozen remove address; year frozen at open — no year-poisoning; one-flow/one-modal holds by dispatch construction) — **but broken at the feedback/display level** (I-1, I-2, I-3).
6. **Key routing: HELD.** Priority chain (editing → pending_remove → modal → discard_offered → Esc/q/s/t/X → keymap) is airtight; `pending_remove` and `modal` are mutually unreachable; `x`/`X` distinct; no action can fire mid-edit or mid-modal; `Left`'s missing `descent` reset is unreachable (with `descent` set, `leave_row` always returns true).
