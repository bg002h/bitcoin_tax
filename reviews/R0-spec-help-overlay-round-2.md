# R0 — SPEC_help_overlay.md — round 2

**Artifact:** `design/SPEC_help_overlay.md`
**Baseline verified against:** branch `feat/tui-help-overlay` @ `0b13e2c` (main == `00cfd86`).
**Reviewer role:** independent architect, R0 round-2 verification of the round-1 fold. Read-only; no implementation, no branch switch.
**Bar:** 0 Critical / 0 Important. Ceremony scaled DOWN (small additive UI feature), gate not removed.
**Round 1:** 0C / 1I / 4M / 2N (`reviews/R0-spec-help-overlay-round-1.md`). All findings folded per commit `0b13e2c`.

## Verdict: 0 Critical / 0 Important / 0 Minor / 1 Nit — **R0-GREEN**

Every round-1 finding is folded and verified against current source. The correctness spine
(complete key list, top-level modal gate, no height gate, tui-edit-only scope) is intact and
re-confirmed. The single Important (I1) — the feature's own entry point being undiscoverable — is
resolved: the footer is now mandated to advertise `?: help`, pinned by a dedicated KAT. The one
remaining Nit is a residual, non-blocking citation imprecision inherited from N2 (footer cited as
`154-155`, actual block `156-162`); it points a reader at the footer's own section comment, so it is
not materially wrong and does not affect the bar. **R0-GREEN.**

---

## Fold verification (per round-1 finding)

### [I1] IMPORTANT — footer must advertise `?` — **FOLDED, verified**
Spec §Content, lines 44-48 now state: "the footer **MUST add `?: help`** so the feature's own entry
point is discoverable," while keeping the overlay as the single authored `KEYMAP` const and the footer
as a deliberately-partial hand-written `[EDITOR]` inline hint — explicitly declining to coerce both
through one rendered structure. KAT `footer_advertises_help` (line 70) asserts the rendered footer
contains `?: help`. This is exactly the round-1 fix (mandate the token + assert the rendered footer),
not the weaker "compact subset … SHOULD be derived" wording that permitted omitting `?`.
**Source cross-check:** live footer (`draw_edit.rs:156-162`, string at 159-160) confirms `?` is absent
today and that the footer is the only always-visible key hint — so this line is genuinely the feature's
discoverability entry point. **Resolved.**

### [M1] MINOR — completeness DoD honesty — **FOLDED, verified**
Spec lines 40-43 and 82-83 now frame completeness as "best-effort … a test CANNOT reflect over `match`
arms, so this is a best-effort guard, not a compile-time proof," and mandate a
`// KEEP IN SYNC with KEYMAP overlay` comment "at the handler (`main.rs:366`)."
**Source cross-check:** `main.rs:366` is the `match key.code {` line opening the Browse dispatch — the
correct edit site for the sync comment. Wording now matches what a Rust unit test can actually enforce.
**Resolved.**

### [M2] MINOR — assert `!should_quit` on close (the real precedence guard) — **FOLDED, verified**
Spec lines 61-62: `help_closes_on_esc_q_and_question` now asserts, per its own annotation, "each of
`Esc`/`q`/`?` clears `help_open` **AND** `!app.should_quit` after `Esc`/`q` [R0-M2 — the precedence
guard is the real assertion]." Line 23 restates the mechanism: "the gate pre-empts the Browse quit arm
(`Char('q')|Esc => should_quit`, `main.rs:367`)."
**Source cross-check + hazard confirmation:** `main.rs:367` is the FIRST Browse match arm
(`KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true`). A top-level `if app.help_open { … return; }`
placed alongside the `*_modal.is_some()` gates returns before the Browse `match` runs, so `q`/`Esc` close
help and can never reach 367 to set `should_quit`. The precedence hazard is addressed and the KAT now
pins it (clearing `help_open` alone would pass even a buggy quit-as-side-effect; asserting `!should_quit`
locks in precedence). **Resolved.**

### [M3] MINOR — KAT for `?` while a flow/modal is open — **FOLDED, verified**
Spec lines 65-66: `question_mark_ignored_while_flow_open` [R0-M3] — "open the void flow, press `?`,
assert `!app.help_open` (the `?` is swallowed by the flow/modal gate first)."
**Source cross-check:** `?` opens help only from the deepest Browse arm (`main.rs:362+`); the modal wall
(`main.rs:130-247`) and flow dispatch (`main.rs:249+`) each `return` before the screen arm, so `?` is
naturally inert over an open flow/modal. The KAT pins that intended non-interaction. **Resolved.**

### [M4] MINOR — backend size / truncation vs scroll — **FOLDED, verified**
Spec lines 57-58: "All KATs render on a ≥`100×40` `TestBackend` [R0-M4] — `centered_rect` TRUNCATES (no
scroll); the authored list must ALSO fit `80×24` or a small terminal truncates it."
**Source cross-check:** `centered_rect` (`draw_edit.rs:1690-1699`) returns `width: width.min(area.width)`,
`height: height.min(area.height)` with `saturating_sub` offsets — pure clamping, and a `Paragraph` has no
scroll offset, so over-tall content is truncated, never scrolled. The reasoning is exactly right: assert
specific keys (`V bulk-void`, "every action key") on a backend tall enough not to truncate, and require
the authored list to fit 80×24 so the real small-terminal render stays intact. **Resolved.**

### [N1] NIT — `q`-closes-help divergence called out — **FOLDED, verified**
Spec lines 84-85: "`q` closes help [R0-N1] — a friendly divergence from other overlays (whose footers say
`q: swallowed`); say so in the overlay's own footer hint so it is self-documenting."
**Source cross-check:** confirmed the established convention — flow/overlay footers read
`… Esc: close   q: swallowed` (`draw_edit.rs:633, 984, 1279, …`), where `q` is a no-op. The divergence is
now explicitly intentional and the spec directs the help overlay's own footer to self-document it.
**Resolved.**

### [N2] NIT — citation drift — **PARTIALLY FOLDED; residual is immaterial (see Nit below)**
The `centered_rect + Clear` "drawn after content" pattern is now cited as `draw_edit.rs:414-416`, which is
exactly the `centered_rect` (414) → `Clear` (416) lines inside `draw_profile_form` (fn at 411) — correct.
The modal-gate cite (`main.rs:130-247`) and the quit-arm cite (`main.rs:367`) both verify precisely. The
footer cite at spec line 13 was NOT retargeted and still reads `draw_edit.rs:154-155` (see Nit N2r).

---

## No-regression spot-checks (all pass)

- **Modal gate shape unchanged.** Spec lines 19-23 still specify a top-level `if app.help_open { … return; }`
  placed alongside the existing `*_modal.is_some()` short-circuits. Source confirms that wall is real and
  contiguous: `main.rs:130-247` is 20+ `if app.<x>_modal.is_some() { handle_<x>_modal_key(app,key); return; }`
  guards (mutation → … → match-self-transfers at 243-247), ahead of flow dispatch (249+) and the screen arm.
  The gate design is unchanged and sound.
- **Key list still matches the live handler.** Re-read `main.rs:366-405`: all 32 Browse bindings
  (`q/Esc, Tab, BackTab, Up/k, Down/j, PgUp, PgDn, g, G, Left, Right, p, c, o, r, f, v, s, d, l, u, a, A,
  b, B, I, C, V, O, m, i, z`) are present in the spec overlay content (§Content lines 32-39). No invented
  keys; labels match the `open_*_flow` targets. Round-1 completeness table still holds.
- **Scope still tui-edit-only, no core/serde.** Spec §Scope (lines 51-54) unchanged: new
  `EditorApp.help_open: bool` (default false), a `?` arm + modal gate in the Browse handler, a
  `draw_help_overlay` fn, the shared keymap source; "No btctax-core / btctax-cli / btctax-tui change; no
  persisted state; no serde." Confirmed `help_open` does not yet exist in source (grep: 0 hits) — correct
  for a spec-only phase. PATCH-class additive.

---

## Findings

### [N2r] NIT (non-blocking) — residual footer citation imprecision
**Spec location:** line 13 — "the footer (`draw_edit.rs:154-155`) advertises only navigation + `p` + `q`."
**Source evidence:** the footer keybinding block is `draw_edit.rs:156-162` (the `let footer_text = if … else { "…" }`,
with the visible string at 159-160). Lines 154-155 are the blank line + the `// ── Footer: status or
keybindings ──` section comment immediately above.
**Assessment:** round-1 N2 recommended retargeting this to `156-162`; the author folded the `centered_rect`
half of N2 (now correct) but left the footer cite at `154-155`. Because 154-155 is the footer's own named
section comment sitting directly above the block, a reader lands in exactly the right region — this is not
materially wrong and does not gate. Optional one-token fix (`154-155` → `156-162`) if the author touches
the spec again; otherwise harmless. **Does not affect the verdict.**

---

## Path to GREEN
Nothing outstanding at the bar. 0 Critical / 0 Important; the four Minors and the substantive Nit (N1) are
all folded and verified against current source; the correctness spine is re-confirmed intact. The lone
residual Nit (N2r) is an immaterial citation imprecision. **R0-GREEN — cleared to proceed to Plan/Implement.**
