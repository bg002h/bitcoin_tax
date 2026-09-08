# Ledger — controller machine-check of the T12 seam review

Every measurable claim the controller re-ran from `2026-09-07-build-interview-T12-review.md`
(persisted verbatim at `e6a0fdc5`). Report counts: `C=1 I=2 M=2 N=3`.

Baseline: `make check` at `833ce3f1` — **3529 passed, 12 skipped, exit 0**, after a full `.rs` touch
per FR-90. `make docs` exit 0, no man-page diff. The reviewer's worktree carried no change but the
report; all plants reverted and touched.

| # | Claim | Check | Result |
|---|---|---|---|
| **C-1** | The panel pane cannot be scrolled to its end; the handler re-clamps by LOGICAL lines | source, both clamps | **CONFIRMED** |
| **I-1** | The panel asserts *"no answer refuses"* on a return the commit gate refuses | variant count | **CONFIRMED** |
| **I-2** | Three document families carrying `transcribed_on` are missing from the undated list | set comparison | **CONFIRMED** |
| M-1, M-2, N-1…N-3 | — | accepted, not re-run | none gates |

---

## C-1 — CONFIRMED, and the renderer's own comment names the defect the handler commits

- **The handler** (`crates/btctax-tui-edit/src/main.rs:1312-1314`), on every keypress:
  ```rust
  let last = form.panel_lines().len().saturating_sub(1);
  form.panel_scroll = form.panel_scroll.min(last);
  ```
  `panel_lines()` returns **logical** lines.
- **The renderer** (`crates/btctax-tui-edit/src/draw_edit.rs:2186`):
  `form.panel_scroll = form.panel_scroll.min(total.saturating_sub(view_h));` where `total` counts
  **wrapped rows** — and its comment says, two files away from the offending code:

  > *"Clamping there would bound the cursor by the number of LOGICAL lines, and a panel whose prompts
  > each wrap to twenty rows would then stop scrolling a fifth of the way down with the rest
  > unreachable: **the FR-63 defect, one surface on**."*

The handler does exactly that, and its clamp runs **after** the renderer's write-back, so the tighter
bound wins. The reviewer measured `logical=39 wrapped=141 view_h=37 renderer_max=104 handler_max=38`,
drove 500 × `PageDown` through the real `handle_key` and got `panel_scroll=38`, and rendered at that
ceiling to find `showing 39–75 of 141 lines` with the last line unreachable. **47% of the panel** is
unreachable by any keystroke, and the tail is where `FORGOING`, `NOT COMPUTED` and the
`(n answered, m not applicable)` summary live — while the footer keeps telling the filer to press
↑/↓/PgUp/PgDn.

**Why the kill could not see it — a shadow, the fourth of this arc.** Kill #2 sets
`form.panel_scroll = usize::MAX / 2` directly (`draw_edit.rs:6858`) and never presses a key, so it
exercises the renderer's clamp and not the handler's. The `p`-toggle test presses keys but never
scrolls. `grep PageDown crates/btctax-tui-edit/src/main.rs` returns **only the handler arm itself** —
no test drives it. The pane's own doc comment states the guarantee that is broken: *"★★ A line that
does not fit is REACHABLE, never clipped (the FR-63 rule)."*

**Severity sustained at Critical.** It is a shipped surface that tells the filer to scroll, cannot,
and hides the part of the panel that lists what the return is forgoing.

## I-1 — CONFIRMED

`RefuseReason` has **126** variants (measured over the enum body). `interview_state.rs` constructs
exactly **five** distinct ones for the `refusing` list — `DependentGateRefused`,
`DependentRefusedByQuestion`, `DocumentTypeUnsupported`, `DocumentDeclaredNotTranscribed`,
`DocumentCensusContradicted`. So on a return that `screen_inputs` refuses for any of the other 121
reasons, the panel prints *"no answer refuses"* — against a `LIMITATIONS.md` promise that T12 itself
ships in this commit.

## I-2 — CONFIRMED, and it is the T11 I-4 shape again

`undated_document_rows` (`provenance.rs:162`) walks five families — `Form 1098-E`, `Form 1099-B`,
`Form 1099-DIV`, `Form 1099-G`, `Form 1099-INT`. The structs that actually carry `transcribed_on`
include **`Form1098`** (added by T9), **`Form1099Sa`** and **`Form5498Sa`** (added by T16). So an
undated Form 1098 / 1099-SA / 5498-SA row is silently absent from **both** the §4.4 block and the
manifest — two surfaces that present themselves as complete lists.

The hand list did not follow the structs that grew under it. That is the same failure as T11's I-4,
where T8 widened the filing-status set and a hand-typed match did not follow.

---

## ★★ The dominant defect class of the whole arc — filed as FR-99 for the owner

Twelve tasks in, one shape accounts for more blocking findings than any other: **a hand-written list
standing beside a set that grows.**

| task | finding | the list | the set that grew past it |
|---|---|---|---|
| T8 | I-2 | a fixture literal hardcoded to `year: 2024` | the bundled year packages |
| T9 | C-1 | three new rules each re-typing a liveness conjunct | the rules reading a 1098 row |
| T10 | I-1 | `CARRIED_IDENTITY`'s phrases | the leaves `seed` carries |
| T11 | I-4 | a `match` on two filing statuses | the five T8 had just built |
| T11 | C-2/I-1 | a completeness partition over `Usd` leaves | facts that *route* money |
| **T12** | **I-1** | five `RefuseReason` variants | **126** |
| **T12** | **I-2** | five document families | the eight carrying `transcribed_on` |

Each was written correctly for the set as it stood. None was wrong when written. The repo already
states the rule for *data* — *"no decision keys on a list you typed beside derived data"* — and it is
plainly the highest-yield rule it has. What FR-99 proposes is making it a **structural** requirement
rather than a review reminder: a list of this shape either derives from the set, or carries a
compiler-enforced totality check (the `_`-free match, the `ALL` const with an exhaustive-match guard)
that reds when the set grows. Adopting doctrine is the owner's call, so it is **filed, not actioned**.

Related: **FR-88** (a derived checker paired with a hand-written *fixture* — the same disease on the
test side, three consecutive tasks) and **FR-90** (a gate reporting something other than what it
measured).

## Disposition

**1C / 2I blocking.** C-1, I-1, I-2 fold, plus M-1, M-2 and N-1…N-3. The reviewer independently
confirmed all four T12-owned follow-ups genuinely close — reproducing FR-87's kill and two of FR-86's
six verbatim **with no checker changed** — and that the commit-modal layout fix holds at all seven
terminal sizes down to 40×10. Fold brief: `BRIEF-fold-interview-T12-review.md`.
