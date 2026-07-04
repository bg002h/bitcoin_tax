# SPEC — `?` help overlay (btctax-tui-edit)

**Source baseline:** `main` @ `00cfd86` (branch `feat/tui-help-overlay`). **Review status: R0-GREEN (2 rounds; 0 Critical / 0 Important). Reviews:
`reviews/R0-spec-help-overlay-round-{1,2}.md` (round 1: 0C/1I — footer must advertise `?`; round 2:
0C/0I/0M/1N). Correctness spine [complete key list, top-level modal gate, no height gate, scope] verified.
Cleared to implement.**
**Lineage:** parked item 1 (user-requested; design SETTLED earlier this session — "full keymap, same on every
tab; per-tab dropped as unnecessary since the action keys are global"). User standing-authorized proceeding.

## The feature
A `?` shortcut in the Browse screen opens a **full-keymap help overlay** listing every keyboard shortcut,
grouped. The reconcile action keys are GLOBAL (work from any tab), so the overlay is the **SAME on every
tab** — there is no per-tab content. **Value:** the ~20 action keys (esp. the reconcile + bulk keys) have NO
on-screen hint today — the footer (`draw_edit.rs:156-162`, text 159-160) advertises only nav + `p` + `q`.

## Behavior
- **Open:** `?` (KeyCode::Char('?'), confirmed FREE) in `EditorScreen::Browse` sets a new `EditorApp.help_open`
  bool → the overlay renders.
- **Modal gate [R0-#2 — the correctness point]:** a top-level `if app.help_open { … return; }` at the START
  of `handle_key`, alongside the existing `*_modal.is_some()` short-circuits (`main.rs:130-247`). While open,
  `?`/`Esc`/`q` CLOSE it and every other key is IGNORED (a reconcile key like `v` must NOT open its flow).
  Because `?` opens help ONLY from the deepest Browse dispatch, `help_open` implies no modal/flow is open, so
  the top-level gate is safe. **`Esc`/`q` while help_open must set `help_open = false` and NOT `should_quit`**
  — the gate pre-empts the Browse quit arm (`Char('q')|Esc => should_quit`, `main.rs:367`).
- **Dismiss:** `?` (toggle) / `Esc` / `q` — all clear `help_open`, none quits.
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
- **[R0-M1] completeness (best-effort):** every action key bound in the live `EditorScreen::Browse` handler
  appears in the overlay's single authored `KEYMAP` const. A KAT pins TODAY's keys against the const — a
  test CANNOT reflect over `match` arms, so this is a best-effort guard, not a compile-time proof; a
  `// KEEP IN SYNC with KEYMAP overlay` comment sits at the handler (`main.rs:366`).
- **[R0-I1 + R0-#3] the footer MUST advertise `?`.** The overlay is the single authored `KEYMAP` const; the
  footer stays a deliberately-partial hand-written inline `[EDITOR]` hint — do NOT coerce both through one
  rendered structure (the drift that matters is handler→overlay, best-effort-pinned above; footer↔overlay
  coercion adds a formatting abstraction worth more than the risk it removes). But the footer **MUST add
  `?: help`** so the feature's own entry point is discoverable. KAT `footer_advertises_help` pins it.

## Scope / SemVer
- **btctax-tui-edit ONLY.** New `EditorApp.help_open: bool` (default false), a `?` arm + the modal gate in the
  Browse key handler, a `draw_help_overlay` fn, and the shared keymap source. **No btctax-core / btctax-cli /
  btctax-tui change; no persisted state; no serde.** PATCH-class (additive UI). No docs mirror (the manual's
  CLI reference is unaffected; this is TUI-interactive help).

## KATs (btctax-tui-edit)
- **All KATs render on a ≥`100×40` `TestBackend`** [R0-M4] — `centered_rect` TRUNCATES (no scroll); the
  authored list must ALSO fit `80×24` or a small terminal truncates it.
- `help_opens_on_question_mark` — `?` in Browse sets `help_open`; the rendered frame contains a help title +
  a reconcile key absent from the footer (e.g. `V bulk-void`).
- `help_closes_on_esc_q_and_question` — each of `Esc`/`q`/`?` clears `help_open` **AND** `!app.should_quit`
  after `Esc`/`q` [R0-M2 — the precedence guard is the real assertion].
- `help_modal_swallows_action_keys` — while `help_open`, pressing `v` does NOT open the void flow
  (`app.void_flow.is_none()` after) — the overlay is modal.
- `question_mark_ignored_while_flow_open` [R0-M3] — open the void flow, press `?`, assert `!app.help_open`
  (the `?` is swallowed by the flow/modal gate first).
- **`help_lists_every_browse_action_key`** — the overlay contains every action key in the current Browse
  handler (esp. the bulk `C/V/I/O` + reconcile keys the footer omits) — the discoverability guarantee
  (best-effort per M1).
- **`footer_advertises_help`** [R0-I1] — the rendered footer contains `?: help`.

## Plan (TDD)
- **Task 1** — `help_open` field + the `?`/modal-gate/dismiss handling + `draw_help_overlay` + the shared
  keymap source (+ derive the footer from it if clean) + the KATs.
- **Task 2** — whole-diff review (Phase E) + full workspace suite + FOLLOWUPS.

## Gotchas
- **Modal gate is the correctness point** — while `help_open`, action keys must be SWALLOWED (not fall
  through to the flow openers), mirroring how `*_modal.is_some()` short-circuits the dispatch; `Esc`/`q`
  close help and must NOT quit (the gate pre-empts the Browse quit arm).
- **Overlay = the authored `KEYMAP` const** [R0-M1] — a KAT pins today's Browse keys against it (best-effort;
  a test can't reflect over `match` arms), with a `// KEEP IN SYNC with KEYMAP overlay` comment at the
  handler. The footer stays hand-written but MUST include `?: help` — do NOT coerce both through one structure.
- **`q` closes help** [R0-N1] — a friendly divergence from other overlays (whose footers say `q: swallowed`);
  say so in the overlay's own footer hint so it is self-documenting.
- **No height gate here** — that requirement belongs to the parked column-totals feature; this is a clamped modal.
- `?` is Shift-/ → `KeyCode::Char('?')`; confirmed unbound.
