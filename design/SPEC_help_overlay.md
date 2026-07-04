# SPEC — `?` help overlay (btctax-tui-edit)

**Source baseline:** `main` @ `00cfd86` (branch `feat/tui-help-overlay`). **Review status: DRAFT — awaiting
R0 (2-round loop to 0C/0I; small feature → ceremony scaled DOWN per §8, not removed).**
**Lineage:** parked item 1 (user-requested; design SETTLED earlier this session — "full keymap, same on every
tab; per-tab dropped as unnecessary since the action keys are global"). User standing-authorized proceeding.

## The feature
A `?` shortcut in the Browse screen opens a **full-keymap help overlay** listing every keyboard shortcut,
grouped. The reconcile action keys are GLOBAL (work from any tab), so the overlay is the **SAME on every
tab** — there is no per-tab content. **Value:** the ~20 action keys (esp. the reconcile + bulk keys) have NO
on-screen hint today — the footer (`draw_edit.rs:154-155`) advertises only navigation + `p` + `q`.

## Behavior
- **Open:** `?` (KeyCode::Char('?'), confirmed FREE) in `EditorScreen::Browse` sets a new `EditorApp.help_open`
  bool → the overlay renders.
- **Modal gate:** while `help_open`, the key dispatch handles ONLY `?`/`Esc`/`q` (all close it) and IGNORES
  every other key (the overlay is on top; a reconcile key like `v` must NOT open its flow). Checked in the
  dispatch alongside the existing `*_modal.is_some()` gates (`main.rs:130-142`).
- **Dismiss:** `?` (toggle) / `Esc` / `q`.
- **Render:** a centered modal (`centered_rect` + `Clear`, drawn AFTER content like the other overlays —
  `draw_edit.rs:414-416` pattern). No height gate — `centered_rect` clamps to the area (the ≥10-row gate is
  the SEPARATE parked column-totals feature, not this one). If content exceeds a tiny terminal, it clamps;
  keep the list compact enough for 80×24.

## Content — grouped, one source of truth
Render from a SINGLE keymap list (const/builder) so the footer and the overlay can NEVER drift:
- **Navigation:** `Tab`/`⇧Tab` switch tab · `←`/`→` change year *(year-scoped tabs)* · `↑`/`↓` or `j`/`k`
  scroll · `PgUp`/`PgDn` page · `g`/`G` top/bottom
- **Reconcile (single):** `c` classify-inbound · `o` reclassify-outflow · `r` reclassify-income · `f` set-fmv ·
  `v` void · `s` select-lots · `d` donation-details · `l` link-transfer · `u` classify-raw ·
  `m` match-self-transfers · `i` resolve-conflict · `z` optimize-accept · `a`/`A` safe-harbor attest/allocate
- **Reconcile (bulk):** `b` bulk-link-transfer · `B` bulk-self-transfer-in · `C` bulk-resolve-conflict ·
  `V` bulk-void · `I` bulk-classify-income · `O` bulk-reclassify-outflow
- **App:** `p` edit tax profile · `?` help · `q`/`Esc` quit
- **[R0: DoD]** every key in the live `EditorScreen::Browse` handler (`main.rs`) appears in the list — a KAT
  cross-checks so a future new flow-key can't silently omit itself. **The existing footer SHOULD be derived
  from (or a documented compact subset of) the same source** so the two never disagree.

## Scope / SemVer
- **btctax-tui-edit ONLY.** New `EditorApp.help_open: bool` (default false), a `?` arm + the modal gate in the
  Browse key handler, a `draw_help_overlay` fn, and the shared keymap source. **No btctax-core / btctax-cli /
  btctax-tui change; no persisted state; no serde.** PATCH-class (additive UI). No docs mirror (the manual's
  CLI reference is unaffected; this is TUI-interactive help).

## KATs (btctax-tui-edit)
- `help_opens_on_question_mark` — `?` in Browse sets `help_open` and the overlay renders (TestBackend: the
  rendered frame contains a help title + a reconcile key absent from the footer, e.g. `V bulk-void`).
- `help_closes_on_esc_q_and_question` — each of `Esc`/`q`/`?` clears `help_open`.
- `help_modal_swallows_action_keys` — while `help_open`, pressing `v` does NOT open the void flow
  (`app.void_flow.is_none()` after) — the overlay is modal.
- **`help_lists_every_browse_action_key`** — the overlay text contains EVERY action key bound in the Browse
  handler (esp. the bulk `C/V/I/O` + reconcile keys the footer omits) — the discoverability guarantee.
- (if the footer is derived) `footer_and_overlay_share_source` — both render from the one keymap const.

## Plan (TDD)
- **Task 1** — `help_open` field + the `?`/modal-gate/dismiss handling + `draw_help_overlay` + the shared
  keymap source (+ derive the footer from it if clean) + the KATs.
- **Task 2** — whole-diff review (Phase E) + full workspace suite + FOLLOWUPS.

## Gotchas
- **Modal gate is the correctness point** — while `help_open`, action keys must be SWALLOWED (not fall
  through to the flow openers), mirroring how `*_modal.is_some()` short-circuits the dispatch.
- **One source of truth** — do not hand-maintain the overlay list separately from the footer; a KAT pins that
  every live Browse key is listed, so an omission fails CI.
- **No height gate here** — that requirement belongs to the parked column-totals feature; this is a clamped modal.
- `?` is Shift-/ → `KeyCode::Char('?')`; confirmed unbound.
