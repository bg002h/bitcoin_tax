# R0 — SPEC_help_overlay.md — round 1

**Artifact:** `design/SPEC_help_overlay.md`
**Baseline verified against:** branch `feat/tui-help-overlay` @ `00d05df` (spec claims `main` @ `00cfd86`).
**Reviewer role:** independent architect, R0 round 1. Read-only; no implementation.
**Bar:** 0 Critical / 0 Important. Ceremony scaled DOWN (small additive UI feature), gate not removed.

## Verdict: 0 Critical / 1 Important / 4 Minor / 2 Nit

The two things that matter are both sound: the **key list is complete and accurate** (every live
Browse binding is in the overlay, nothing invented — #1 verified below), and the **modal-gate design
is correct** (top-level bool gate mirroring the `*_modal.is_some()` short-circuits, closing takes
precedence over the Browse quit arm — #2 verified). The one Important is that the feature's *entry
point* (`?`) is left undiscoverable: the spec never commits the always-visible footer to advertise
`?: help`, which undercuts the whole discoverability rationale. The Minors are test-strength /
mechanism-honesty items. Not GREEN this round on account of I1; everything else is cheap.

---

## #1 — Key completeness: VERIFIED (no finding)

Live `EditorScreen::Browse` handler is `main.rs:366-405`. Full binding set, cross-checked against the
spec overlay content (`SPEC` lines 28-35):

| Live key (main.rs) | Action | In spec overlay? |
|---|---|---|
| `q`/`Esc` (367) | quit | ✓ App |
| `Tab` (368) / `BackTab` (369) | next/prev tab | ✓ Nav (`Tab`/`⇧Tab`) |
| `Up`/`k` (370), `Down`/`j` (371) | scroll | ✓ Nav |
| `PageUp` (372) / `PageDown` (373) | page | ✓ Nav (`PgUp`/`PgDn`) |
| `g` (374) / `G` (375) | top/bottom | ✓ Nav |
| `Left` (376) / `Right` (380) | year ∓1 | ✓ Nav (`←`/`→`) |
| `p` (384) | profile form | ✓ App |
| `c` (385) | classify-inbound | ✓ single |
| `o` (386) | reclassify-outflow | ✓ single |
| `r` (387) | reclassify-income | ✓ single |
| `f` (388) | set-fmv | ✓ single |
| `v` (389) | void | ✓ single |
| `s` (390) | select-lots | ✓ single |
| `d` (391) | donation-details | ✓ single |
| `l` (392) | link-transfer | ✓ single |
| `u` (393) | classify-raw | ✓ single |
| `a` (394) | safe-harbor attest | ✓ single (`a`/`A`) |
| `A` (395) | safe-harbor allocate | ✓ single |
| `b` (396) | bulk-link-transfer | ✓ bulk |
| `B` (397) | bulk-self-transfer-in | ✓ bulk |
| `I` (398) | bulk-classify-income | ✓ bulk |
| `C` (399) | bulk-resolve-conflict | ✓ bulk |
| `V` (400) | bulk-void | ✓ bulk |
| `O` (401) | bulk-reclassify-outflow | ✓ bulk |
| `m` (402) | match-self-transfers | ✓ single |
| `i` (403) | resolve-conflict | ✓ single |
| `z` (404) | optimize-accept | ✓ single |

All 32 live bindings appear; every label matches the live `open_*_flow` target (verified `i`→
`open_resolve_conflict_flow` vs `C`→`open_bulk_resolve_conflict_flow`, `m`→`open_match_self_transfers_flow`,
`B`→`open_bulk_self_transfer_in_flow` — no mislabels). No invented keys in the spec. `?` (`KeyCode::Char('?')`)
is confirmed **unbound** anywhere in the crate (`grep "Char('?')"` → 0 hits), so it is genuinely free.
**#1 clean.**

## #2 — Modal gate: VERIFIED sound (no finding)

The `*_modal.is_some()` short-circuit pattern the spec mirrors is real: `main.rs:130-247` is a wall of
`if app.<x>_modal.is_some() { handle_<x>_modal_key(app, key); return; }` guards ahead of the flow (249-331),
form (334-337), and screen (340-408) dispatch. Adding `if app.help_open { … ; return; }` in that same
region is the correct shape.

**Correct placement + why it's safe.** `?` *opens* help only from the Browse screen arm
(`SPEC` line 15; deepest dispatch level, `main.rs:362`), which is reached only when no modal/flow/form is
open. Therefore `help_open == true` ⇒ no modal/flow/form is open, and a top-level gate placed anywhere at
or above the screen dispatch is behaviorally equivalent and cannot swallow a modal's keys. Recommend the
gate sit at the **top of `handle_key`** (right after the `KeyEventKind::Press` check, before line 130),
exactly alongside the modal gates:

```rust
if app.help_open {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => app.help_open = false,
        _ => {} // swallow everything else — overlay is modal
    }
    return;
}
```

**Ordering hazard (confirmed, and this placement avoids it).** In the Browse `match`, the FIRST arm is
`KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true` (`main.rs:367`). So closing-help MUST intercept
`q`/`Esc` *before* the screen match runs — which the top-level gate does (it `return`s). The anti-pattern to
avoid: adding help-close handling *inside* the Browse arm without an early return, where `q`/`Esc` would hit
the quit arm at 367 first and quit instead of closing. The spec's stated placement ("alongside the existing
`*_modal.is_some()` gates", line 19) is the right one and dodges this. `v` (and every other action key) is
swallowed by the `_ => {}` arm, so `v` cannot open the void flow while help is up — matching the DoD.
**#2 clean** (see M2 for the KAT that should *pin* the q/Esc-precedence).

## #4 — No height gate: VERIFIED (no finding)

`centered_rect` (`draw_edit.rs:1690-1699`) clamps: `width: width.min(area.width)`,
`height: height.min(area.height)`, with `saturating_sub` for the offsets. So a `centered_rect + Clear`
overlay (the `draw_profile_form` pattern, `draw_edit.rs:411-416`) can never overflow the area; the ≥10-row
gate correctly does not belong here. Spec's claim is accurate. (Caveat folded into M4: clamping *truncates*
content — it does not scroll — so the authored list must fit the smallest asserted terminal.)

## #5 — Scope / SemVer: VERIFIED (no finding)

`EditorApp` (`editor.rs:57`) has **no** serde derive (plain runtime struct); adding `help_open: bool`
(default false) is a pure UI-state field with no persistence. The feature touches only the Browse key
handler + a new `draw_help_overlay` fn + the shared keymap, all within `btctax-tui-edit`. No
core/cli/tui-viewer change, no serde, no on-disk state. PATCH-class additive. Spec's scope statement is
accurate.

---

## Findings

### [I1] IMPORTANT — the footer never commits to advertising `?`, so the discoverability feature is itself undiscoverable
**Spec location:** lines 11-12 (value: "the ~20 action keys … have NO on-screen hint today — the footer …
advertises only navigation + `p` + `q`"), lines 36-38 (footer "SHOULD be derived from (or a documented
compact subset of) the same source"), KATs 46-54.
**Source evidence:** live footer string `draw_edit.rs:159-160`:
`"Tab/Shift-Tab: switch tab   ←/→: change year   ↑/↓ j/k: scroll   PgUp/PgDn: page   g/G: top/bottom   p: edit tax profile   q/Esc: quit   [EDITOR]"`
— confirms `?` is absent today. The footer is the ONLY always-visible key hint.
**Problem:** the feature exists so users can *discover* the hidden action keys, but `?` itself is a hidden
key. The spec's footer treatment is permissive ("SHOULD be derived … *or a documented compact subset*") and
never states that `?: help` must appear in the footer; the proposed `footer_and_overlay_share_source` KAT
only checks a shared source, not that the rendered footer contains `?`. A "compact subset" could omit `?`
and still pass. Net: a user who doesn't already know `?` works will never learn the overlay exists.
**Fix (trivial, but must be mandated):** require the footer to advertise `?: help` (add the token to the
one hand-written string, e.g. `… p: edit tax profile   ?: help   q/Esc: quit   [EDITOR]`), and add a KAT
`footer_advertises_help` asserting the rendered Browse footer contains `?`. This is the single
highest-value line in the diff; the spec must pin it rather than leave it to the "compact subset" author's
discretion.

### [M1] MINOR — the DoD claim "a KAT … so a future new flow-key can't silently omit itself" overstates what a Rust test can enforce
**Spec location:** lines 36-38 and 64-65 ("a KAT pins that every live Browse key is listed, so an omission
fails CI").
**Source evidence:** the Browse bindings are `match key.code` arms (`main.rs:366-405`) mapping chars to
`open_*_flow(app)` calls. A unit test cannot reflect over `match` arms at runtime, and a `(char,label)`
const cannot drive that dispatch (each arm calls a distinct fn). So the only realizable KAT asserts the
overlay contains a **hand-maintained expected set** of today's keys — which catches accidental *removal
from the overlay* but does NOT automatically catch a genuinely new handler arm that the author forgets to
add to both the overlay and the expected list.
**Fix:** soften the DoD wording to what's true ("the KAT pins every key bound *today*; adding a new Browse
key requires updating the `KEYMAP` const and the KAT"), and add a `// KEEP IN SYNC with KEYMAP` comment at
the top of the Browse `match` (`main.rs:366`) so the coupling is visible at the edit site. The
today-coverage guarantee (`help_lists_every_browse_action_key`) is fully achievable and worth keeping.

### [M2] MINOR — `help_closes_on_esc_q_and_question` must also assert `!should_quit`, since that is the actual guard for the #2 precedence hazard
**Spec location:** line 49 ("each of `Esc`/`q`/`?` clears `help_open`").
**Source evidence:** `q`/`Esc` are the first Browse arm → quit (`main.rs:367`); the whole point of the gate
is that closing-help pre-empts the quit.
**Fix:** the KAT should assert, after `q` (and after `Esc`), BOTH `!app.help_open` AND `!app.should_quit`.
Clearing `help_open` alone would still pass even if a mis-placed implementation quit as a side effect;
pinning `!should_quit` is what actually locks in "closing takes precedence."

### [M3] MINOR — missing KAT for `?` while a flow/modal is already open (the #6 case)
**Spec location:** KAT list 46-54 (no such case).
**Source evidence:** `?` open is screen-level (`main.rs:362+`); a modal/flow gate (`main.rs:130-331`)
`return`s first, so `?` is *naturally* inert while a modal/flow is open — good behavior, but untested.
**Fix:** add `question_mark_ignored_while_flow_open` — open e.g. the void flow, press `?`, assert
`app.help_open == false` (help cannot open on top of a modal/flow). Cheap and pins the intended
non-interaction.

### [M4] MINOR — KATs that assert specific overlay keys must use a backend tall enough that clamping doesn't truncate them, and the list must fit 80×24
**Spec location:** line 24 ("keep the list compact enough for 80×24"; "If content exceeds a tiny terminal,
it clamps"), KAT 48 (asserts `V bulk-void` present), KAT 52 ("EVERY action key").
**Source evidence:** `centered_rect` clamps by *truncation*, not scrolling (`draw_edit.rs:1696-1697`); there
is no scroll offset for a Paragraph. Existing rich-overlay KATs already use `TestBackend::new(100,40)` /
`(120,40)` (e.g. `main.rs:9947, 11000`), while some render tests use `(80,24)` / `(80,10)`
(`main.rs:8725, 8758`). A ~20-line grouped overlay near the bottom of an 80×24 area could truncate
`V bulk-void` and silently fail/mislead the assertion.
**Fix:** state that the key-completeness KATs render on a backend at least as tall as the overlay (reuse
the repo's `100×40`/`120×40` convention), and that the author must verify the authored list fits within
80×24 so the general small-terminal render remains intact.

### [N1] NIT — `q` closes help here, but every other overlay treats `q` as "swallowed"
**Spec location:** line 20 ("Dismiss: `?` (toggle) / `Esc` / `q`").
**Source evidence:** the flow/modal footers read `… Esc: close   q: swallowed` (`draw_edit.rs:633, 984,
1279, 1470`) — in those overlays `q` is a no-op and only `Esc`/`Enter` dismiss. Making `q` *close* the help
overlay is a reasonable, friendlier choice for a read-only help panel, but it is a deliberate divergence
from the established convention.
**Fix:** none required — just call it out in the spec (one line) so the divergence is intentional and the
help overlay's own footer/hint says `Esc/q/?: close` rather than `q: swallowed`, avoiding user confusion.

### [N2] NIT — footer/overlay citation line numbers are slightly off
**Spec location:** lines 12 & 27 cite the footer as `draw_edit.rs:154-155`; line 22 cites the
"drawn AFTER content" overlay pattern as `draw_edit.rs:414-416`.
**Source evidence:** the footer keybinding string is `draw_edit.rs:156-162` (the actual text at 159-160);
154-155 is the blank line + section comment. The "overlays drawn after content" dispatch block begins
`draw_edit.rs:166`; 414-416 is specifically the `centered_rect + Clear` lines inside `draw_profile_form`.
**Fix:** retarget the citations (footer → 156-162; after-content dispatch → 166+; centered_rect+Clear
pattern → 411-416). Harmless, but citations decay and this is R0.

---

## Answers to the review questions

- **#3 (one source of truth) — recommendation:** do **not** force the footer and overlay to render from a
  single rendered data structure. The drift that actually matters is *handler → overlay* (a new action key
  missing from help), best-effort pinned by `help_lists_every_browse_action_key` (with M1's honesty caveat).
  The footer is a deliberately *partial*, inline, centered, `[EDITOR]`-tagged one-liner; the overlay is a
  grouped multi-line panel. Coercing both through one list buys little and adds a formatting abstraction
  worth more than the risk it removes. Make the **overlay** the single authored `KEYMAP` const; keep the
  footer hand-written but (per I1) add `?: help`. Replace the proposed `footer_and_overlay_share_source`
  KAT with `footer_advertises_help` (assert the footer contains `?`). This is lower-risk and matches the
  "ceremony scaled down" posture.
- **#6 (KAT coverage):** opens-on-`?`, closes-on-Esc/q/`?`, modal-swallows-action-keys, and
  lists-every-key are all present. Gaps: M2 (assert `!should_quit` on close), M3 (`?` ignored over an open
  flow/modal), I1 (footer advertises `?`), M4 (backend size for key-completeness assertions).

## Path to GREEN (round 2)
Fold **I1** (mandate footer `?: help` + its KAT). The four Minors and two Nits are cheap and should be
folded in the same pass (M1 wording+comment, M2 assert `!should_quit`, M3 add the ignored-over-flow KAT,
M4 backend-size note, N1 convention note, N2 citation fixes). No Critical, no architectural rework — the
correctness spine (#1 completeness, #2 modal gate, #4 no-height-gate, #5 scope) is sound as written.
