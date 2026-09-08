# Brief — fold the T12 seam review (1C / 2I / 2M / 3N)

You are folding an independent seam review into the **main working tree** of `/scratch/code/bitcoin_tax`,
at `HEAD` on branch `main`. You are the only agent editing this tree. **Commit nothing** — the controller
commits through the pre-commit gate when you return. Do not push, do not `git stash`, do not spawn
subagents, and never `git checkout -- <file>` over your own uncommitted work (keep a `cp` backup, then
`touch`).

**This is the last fold of the interview arc.** T1–T11 and T16 are closed at 0C/0I.

Read first, in this order:
1. `design/agent-reports/2026-09-07-build-interview-T12-review.md` — the review, verbatim.
2. `design/agent-reports/2026-09-07-build-interview-T12-review-VERIFICATION.md` — the controller's ledger.
   **C-1, I-1 and I-2 are machine-confirmed at source.** Do not re-litigate. Your job is the fix and its kill.
3. `design/agent-reports/2026-09-07-build-interview-T12-implementation.md` — the build you are correcting.
4. `CLAUDE.md` at the repo root.

## ⚠ FR-90 — `touch` after every plant and every restore

A reverted plant once stayed compiled into the `btctax-core` rlib and produced a phantom red at a clean
HEAD. `find crates -name '*.rs' -exec touch {} +` before the closing gate.

## Validation

Main tree ⇒ `CARGO_TARGET_DIR` unset for tests; `target-clippy` for clippy. Scoped runs while you work;
`make check` once at the end (3529 at HEAD) and `make docs` (must stay clean, no man-page diff).
Instruments at HEAD: `line-coverage` 375/18/31 (ratchet 31)/0/17; `census-join` 274 across 13 maps;
`stop-list` 8+4 sources **+ 1 renderer source**, 91 prompts; `prompt-check` 88; `box-census` 268/19/9.

## C-1 (Critical, fold first) — the panel pane cannot be scrolled to its end

Confirmed. `main.rs:1312-1314` re-clamps `panel_scroll` by **logical** lines on every keypress, after
`draw_edit.rs:2186` has already clamped by **wrapped rows** — so the tighter bound wins and the pane
stops dead. Measured: `logical=39 wrapped=141 view_h=37 renderer_max=104 handler_max=38`; 500 ×
`PageDown` through the real `handle_key` leaves `panel_scroll=38`; rendering at that ceiling shows
`showing 39–75 of 141 lines` with the last line unreachable. **47% of the panel is unreachable**, and the
hidden tail is where `FORGOING`, `NOT COMPUTED` and the `(n answered, m not applicable)` summary live —
while the footer tells the filer to press ↑/↓/PgUp/PgDn.

The renderer's own comment names this defect — *"the FR-63 defect, one surface on"* — two files from the
code that commits it, and the pane's doc states the broken guarantee: *"A line that does not fit is
REACHABLE, never clipped."*

**Fix.** Delete the two-line clamp in the handler and let the renderer's write-back own it — the pattern
the commit modal already uses (`main.rs:1259-1268` only *moves* `m.scroll`; the draw fn clamps and writes
back). The reviewer applied exactly this in their worktree and re-ran the crate: **400 passed, 2
skipped**, no regression.

**Kill — and this is the part that matters.** The existing kill is a **shadow**: it sets
`form.panel_scroll = usize::MAX / 2` directly (`draw_edit.rs:6858`) and never presses a key, so it
exercises the renderer's clamp and not the handler's. `grep PageDown crates/btctax-tui-edit/src/main.rs`
returns **only the handler arm itself** — no test drives it. Reach the tail **through `handle_key`**, so
the guarantee is held where a filer actually exercises it. Then plant the clamp back and paste the red.

★ This is the fourth shadow kill of the arc (T8 I-1, T9 C-1, T10 I-1, now this). The common thread is a
test that reaches past the layer where the defect lives. When you write the kill, enter at the layer the
**filer** enters at.

## I-1 (Important) — the panel says "no answer refuses" on a return the commit gate refuses

Confirmed: `RefuseReason` has **126** variants; `interview_state.rs` constructs exactly **five** for the
`refusing` list (`DependentGateRefused`, `DependentRefusedByQuestion`, `DocumentTypeUnsupported`,
`DocumentDeclaredNotTranscribed`, `DocumentCensusContradicted`). On a return `screen_inputs` refuses for
any of the other 121, the panel asserts the opposite — against a `LIMITATIONS.md` promise **this same
commit ships**.

**Fix.** Decide and state which: either the panel's `refusing` list is **derived** from the same screen
the commit gate runs (preferred — one function, the shape T12 already used for `home_sale_decision`), or
the panel stops making the universal claim and says only what it actually knows. Do not leave a surface
asserting completeness it does not have. Update `LIMITATIONS.md` in the same pass so the promise and the
behaviour agree.

**Kill.** A return refused by a reason outside the five must not print *"no answer refuses"*. Plant the
derivation back to the hand list → red.

## I-2 (Important) — three document families are missing from the undated-rows list

Confirmed: `undated_document_rows` (`provenance.rs:162`) walks `Form 1098-E`, `Form 1099-B`,
`Form 1099-DIV`, `Form 1099-G`, `Form 1099-INT`. The structs carrying `transcribed_on` also include
**`Form1098`** (added by T9), **`Form1099Sa`** and **`Form5498Sa`** (added by T16). So an undated
Form 1098 / 1099-SA / 5498-SA row is silently absent from **both** the §4.4 block and the manifest — two
surfaces that present themselves as complete.

**Fix — derive it, do not extend the list.** This is the exact shape as T11's I-4 (T8 widened the filing
statuses; a hand-typed match did not follow), and the ledger tabulates seven instances across the arc.
Adding three entries fixes today and re-arms tomorrow. Make the walk **total over the structs that carry
`transcribed_on`**, with a compiler-enforced or derived completeness check, so the ninth family cannot be
forgotten.

**Kill.** Add a document family carrying `transcribed_on` and leave the walk alone → red. If you cannot
make it structural, say so plainly and state exactly what the check does and does not cover.

## M-1, M-2, N-1…N-3

- **M-1** the manifest's FORGONE block drops the *NOT COMPUTED* instruction.
- **M-2** below 80 columns the pane's `[p/Esc] close` legend is truncated; the modal wraps its legend and
  the pane does not. Same class as C-1 — a chrome element the filer needs, lost to layout.
- **N-1** a duplicated comment block in `draw_tax_inputs_modal`. **N-2** `r15_stop_list.rs` still says
  "three" in two doc comments (there are now four checks). **N-3** `progress_widgets` scans one file and
  the CLI panel renderer is outside its field of view — a checker whose scope no longer matches its
  subject; fix or state the limit.

## Standing lessons that bind this fold

- **Enter at the layer the filer enters at.** Four shadow kills in twelve tasks all reached past it.
- **A surface that presents a complete list must have one** — that is I-1 and I-2, and T10's I-1 before them.
- **Derive; never type a list beside a set that grows** (the ledger's FR-99 table).
- **Render, never recompute.**

## Report — your FINAL action

Write `design/agent-reports/2026-09-07-build-interview-T12-fold.md`: Commands with real output; one
section per finding (What changed / Where / The kill and its observed red / Anything decided differently
and why); for I-1 and I-2, an explicit statement of what the new check covers **and what it does not**;
a **Deviations** section; the five instrument outputs; `make docs` result; the closing `make check`
summary line. Then return ONLY a 3-line summary plus the path.
