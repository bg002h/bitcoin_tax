# Whole-diff review (Phase E) — `?` help overlay (btctax-tui-edit)

**Reviewer:** independent (reviewer ≠ author). **Branch:** `feat/tui-help-overlay`.
**Commit under review:** `51206d4` — files `crates/btctax-tui-edit/src/{editor.rs, main.rs, draw_edit.rs}`.
**Contract:** `design/SPEC_help_overlay.md` (R0-GREEN, 2 rounds). **Bar:** 0 Critical / 0 Important.

## Verdict

**0 Critical / 0 Important / 0 Minor / 2 Nit — SHIP.**

Fault injection (the correctness gate) confirmed load-bearing; full suite green (255/255);
scope clean (btctax-tui-edit only, no serde). The two Nits are optional polish, non-blocking.

---

## 1. The modal gate [★ the correctness point] — VERIFIED

`handle_key` (`main.rs:129-140`) places the gate as **section 0**, BEFORE the 20+ `*_modal.is_some()`
short-circuits (from `main.rs:142`), the flow gates (`main.rs:262`), the form gate (`main.rs:347`),
and the Browse screen dispatch (`main.rs:375`, quit arm at `main.rs:380`).

- **(a) non-close keys SWALLOWED.** The gate returns for *every* key when `help_open`; only
  `?`/`Esc`/`q` additionally clear the flag. So `Tab`, `v`, and every flow opener never reach the
  Browse dispatch. Pinned by `help_modal_swallows_action_keys` (Tab = load-bearing probe; `v` secondary).
- **(b) `Esc`/`q` close, do NOT quit.** The gate sets `help_open = false` and `return`s, pre-empting the
  Browse quit arm `Char('q')|Esc => should_quit` (`main.rs:380`). Pinned by
  `help_closes_on_esc_q_and_question`, which asserts `!app.should_quit` after each of `Esc`/`q`/`?`.
- **(c) `?` only settable from Browse dispatch.** `KeyCode::Char('?') => app.help_open = true` appears
  exactly once as a mutator (`main.rs:419`), inside the `EditorScreen::Browse` match — reachable only
  after all modal/flow/form gates returned early. Therefore `help_open` ⇒ no modal/flow/form open, so the
  top-level `return` is safe (never strands an open modal). The only other `Char('?')` in `handle_key` is
  the gate's own close-match at `main.rs:135`.

**Fault injection:** deleted the gate's `return;` (left the flag-reset), ran
`cargo test -p btctax-tui-edit help_modal_swallows_action_keys` →
**FAILED** at `main.rs:8857`: *"Tab while help_open must be SWALLOWED (overlay is modal), not cycle tabs"*
(without the return, Tab falls through to `app.tab = app.tab.next()`). Restored the `return;`; the test
is GREEN again and `git status` is clean. The gate's `return` is confirmed load-bearing.

## 2. Key completeness — VERIFIED (no missing, no invented)

Cross-checked `draw_help_overlay` (`draw_edit.rs:1706-1726`) against every `EditorScreen::Browse` match
arm (`main.rs:380-419`). Every bound action key is present and every listed key is really bound:

- Action: `c o r f v s d l u m i z a A b B C V I O p ?` — all present.
- Bulk labels map to real openers: `b`→bulk-link, `B`→bulk-self-transfer-in, `C`→bulk-resolve-conflict,
  `V`→bulk-void, `I`→bulk-classify-income, `O`→bulk-reclassify-outflow.
- Nav: `Tab`/`Shift-Tab` (BackTab), `←/→`, `j/k`, `↑/↓`, `PgUp/PgDn`, `g/G` — all present.
- `q/Esc` — present ("q/Esc close" + the self-documenting "? · Esc · q to close" hint per R0-N1).
- No invented/unbound key in the overlay.

Best-effort pin `help_lists_every_browse_action_key` asserts the descriptive labels (M1: a test can't
reflect over match arms); the `// KEEP IN SYNC with KEYMAP overlay` comment sits at `main.rs:418` and the
mirror doc-comment at `draw_edit.rs:1695`.

## 3. Footer advertises `?` [R0-I1] — VERIFIED

Footer string (`draw_edit.rs:159-160`) now reads `… g/G: top/bottom   p: profile   ?: help   q/Esc: quit
[EDITOR]`. Contains `?: help`. Pinned by `footer_advertises_help` (rendered against a real snapshot with
the overlay closed, so the footer is the surface under test). ("p: edit tax profile" was shortened to
"p: profile" to make room — cosmetic, in scope, consistent with the overlay's own "p profile".)

## 4. Fits 80×24 — VERIFIED

Overlay is `centered_rect(70, lines.len()+2=18, area)` with `Clear` (`draw_edit.rs:1727-1737`);
`centered_rect` clamps width/height to `area` (`draw_edit.rs:1740-1749`) — no scroll, truncates if too
big. Measured display widths (unicode-aware; `←→↑↓·—` all width-1):
- Box: outer width **70 ≤ 80**, outer height **18 ≤ 24** — fits with margin.
- Longest content line = **67 cols** (`i resolve-conflict … a/A safe-harbor attest/allocate`); inner width
  at rect-70 is 68, so it fits with **1 column** to spare. Title " Help — keyboard shortcuts " = 27.

## 5. KAT coverage (6) — COMPLETE, no gap

`help_opens_on_question_mark`, `help_closes_on_esc_q_and_question` (+`!should_quit`),
`help_modal_swallows_action_keys` (Tab load-bearing), `question_mark_ignored_while_flow_open`,
`footer_advertises_help`, `help_lists_every_browse_action_key`. All 6 present and passing.

## 6. No regression / scope — VERIFIED

- `cargo test -p btctax-tui-edit` → **255 passed / 0 failed**, incl. `q_on_browse_sets_should_quit`,
  `tab_on_browse_cycles_forward`, and all existing modal/flow suites.
- `help_open: bool` is a plain field on `EditorApp` (`editor.rs:79`); the struct has no `#[derive]` /
  serde — pure runtime UI state, no persisted format change.
- Diff touches only the three btctax-tui-edit files; no btctax-core / btctax-cli / btctax-tui change.
  PATCH-class additive UI, matching the spec's scope/SemVer section.

---

## Nits (optional, non-blocking)

- **N1 — "q/Esc close" wording in the App group.** Inside the overlay the App line reads "q/Esc close",
  but the *Browse* binding for `q/Esc` (help closed) is quit-the-app. The dedicated hint line
  "? · Esc · q to close" already self-documents the help-close divergence (R0-N1), so the App group
  arguably reads truer as "q/Esc quit". Purely cosmetic.
- **N2 — 1-column headroom on the longest line.** The 67-col line fits the 68-col inner width with a
  single column to spare; a future word added to that line would truncate on an 80-col terminal. Consider
  widening the box to ~72 (still ≤ 80) if that line grows.

**SHIP.**
