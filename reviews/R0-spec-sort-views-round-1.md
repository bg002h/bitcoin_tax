# R0 — SPEC_sort_views.md — round 1 (independent architect)

**Artifact:** `design/SPEC_sort_views.md` (DRAFT).
**Baseline:** branch `feat/sort-views` @ `7dd5f9a`; `main` @ `1837917`. Repo `/scratch/code/bitcoin_tax`.
**Reviewer role:** independent architect (author ≠ reviewer). Read-only; no implementation.
**Bar:** 0 Critical / 0 Important.

## Verdict: **0 Critical / 4 Important / 4 Minor / 2 Nit — CHANGES REQUIRED (not GREEN).**

Headline: the **★ selection-retargeting** worry does **not** manifest as a data-safety bug — the editor's
edit flows do **not** resolve their target from the tab's `TableState` selection; each flow opens its **own**
picker built fresh from the snapshot. So there is **no Critical**. *But* the spec's flagship safety claim
(the selected row is "identified by its stable key, not its index") is **counterfactual** — the tab selection
is index-based and used only for scroll — and its KAT tests a mechanism the code does not have. Together with
three other factual defects (an infeasible comparator signature, wrong column enumerations, and a wrong docs
lockstep instruction), the spec is not yet implementable as written.

---

## ★ Finding on the highest-risk item (#1): the good news and the real defect

**The data-safety catastrophe the prompt worried about does NOT exist.** I traced how the editor resolves an
edit target:

- The tab selection states `holdings_state` / `disposals_state` / `income_state` (editor `EditorApp`,
  `editor.rs:96-98`) are `ratatui::TableState` and are read **only** by the scroll helpers via
  `active_state()` (`main.rs:8771-8778`: `scroll_up/down`, `page_up/down`, `go_top/bottom`) and cleared by
  `reset_selections()` (`main.rs:8885-8889`). Grep of every `holdings_state|disposals_state|income_state`
  reference in `main.rs` returns **only** those two sites.
- **No** edit-flow opener reads the tab selection. `open_select_lots_flow` (`main.rs:3799`) builds its **own**
  list from `snap.state.disposals` / `snap.state.removals` / raw events (`main.rs:3890-3970`), sorts it by
  date, and stores it as `TargetList::new(items)` inside `SelectLotsFlowState` (`main.rs:3980-3983`). The user
  then picks the target **inside the flow's picker**; every `.selected()` in `main.rs` (lines 865, 1447, 1881,
  …, 3546, 4291, 4369, …) operates on a **flow-local** list (`f.list`, `f.out_list`,
  `f.preview.table_state`), never on `app.*_state`.

**Conclusion:** reordering the display rows in a tab cannot retarget an edit, because the edit target is never
derived from the display-row order or the tab selection. **No Critical.**

**The real defect (→ Important, see I1):** the spec asserts a *reason* that is false. SPEC lines 49 and 94
claim "the selected row is identified by its stable key, not its index, so a re-sort keeps the right row
selected." There is **no** stable-key selection anywhere; the tab selection is a bare row **index**
(`TableState::selected() -> Option<usize>`) and is scroll-only. The spec must state the **true** reason
sorting is safe (edit targets come from per-flow pickers, independent of the tab's sort/selection) and reframe
the "keep selection" item and its KAT (which currently test a mechanism that does not exist).

---

## Important

### I1 — The flagship "stable-key selection" safety claim is counterfactual; the edit-after-sort KAT tests a non-existent mechanism
- **Where:** SPEC lines 48-49 ("It must not touch … any `TableState` selection identity semantics … the
  selected row is identified by its stable key, not its index"), line 76-77 KAT
  `editor_edit_after_sort_targets_correct_row` ("select-lots on the cursor row after a re-sort hits the
  intended disposal by stable key, not stale index"), and gotcha line 94.
- **Evidence:** selection is index-based (`ratatui::TableState`, `editor.rs:96-98`) and scroll-only
  (`main.rs:8771-8778`, `8885-8889`). `open_select_lots_flow` does not read `disposals_state`; it opens its own
  picker (`main.rs:3799`, `3980-3983`). There is no "cursor row" that select-lots acts on.
- **Why it matters:** an approved spec whose central safety argument is factually wrong will mislead
  implementation — someone may try to build "stable-key selection tracking" (wasted work on a non-problem), or
  write the KAT against a code path that does not exist (it would be vacuous or test the wrong thing).
- **Fix:** rewrite the display-only invariant to state the real safety basis — *edit targets are chosen inside
  each flow's own picker (`main.rs:3980`), built fresh from the snapshot; the tab `TableState` is scroll-only
  (`main.rs:8771-8778`) and is never an edit target, so display sorting cannot retarget an edit.* Then decide
  and specify the **scroll-highlight** behavior across a re-sort explicitly: either (a) re-find the previously
  highlighted logical row by a stable key after sorting so the highlight follows it (new behavior — code today
  keeps the raw index), or (b) accept that the index-anchored highlight stays at position N. Replace the
  `editor_edit_after_sort_targets_correct_row` KAT with (i) the display-invariant KAT
  (`sorting_does_not_mutate_events_or_state`, keep) and (ii) a KAT for whichever highlight behavior is chosen.

### I2 — `sort_rows(rows: &mut Vec<Row>, …)` is infeasible; it contradicts "sort the typed values"
- **Where:** SPEC lines 52-56 / 98-99: a pure `sort_rows(view, rows: &mut Vec<Row>, sort)` "comparator keyed
  off the row's typed fields (NOT the formatted strings)."
- **Evidence:** the view row vecs are `Vec<ratatui::widgets::Row>` whose cells are **formatted strings** —
  `disposals.rs:58-67` (`Cell::from(format!("{:.2}", leg.proceeds))` etc.), `holdings.rs:51-58`,
  `income.rs:41-47`. A `ratatui::Row` exposes **no** accessor to read cell text back and carries **no** typed
  fields. You cannot "key off typed fields" when the argument is a `Vec<Row>` of strings — the two clauses of
  the spec contradict each other.
- **Why it matters:** the module as specified cannot be built. Numeric/date columns would either be
  un-sortable or sort lexically ("$1,000" < "$900"), which is exactly the failure the spec says to avoid.
- **Fix:** sort a **typed** intermediate. The typed source is available on the shared `Snapshot`
  (`snap.state.lots`, `snap.state.disposals[].legs`, `snap.state.income_recognized` — `state.rs`), so the
  comparator should order a `Vec<TypedRowKey>` (Date/Decimal/WalletId/term/…) and the draw fn builds `Row`s in
  that order — not `sort_rows(&mut Vec<Row>)`. Correct the module signature and the KATs accordingly.

### I3 — The "approved" sortable-column lists do not match the columns actually rendered (all 3 views); two listed columns have no backing data
- **Where:** SPEC lines 33-35.
- **Evidence (rendered columns are authoritative — the editor reuses the same `render` fns via
  `draw_edit.rs:143-163`):**
  - **Holdings** rendered header = `Wallet, Acquired, BTC, USD Basis, Source, Pending` (`holdings.rs:86-93`).
    Spec lists a **`term`** column that **does not exist** in Holdings, and **omits** the real `Source`
    (`basis_source_tag`) and `Pending` columns.
  - **Disposals** rendered header = `Disposed, Acquired, BTC, Proceeds, Basis, Gain, Term, Wallet`
    (`disposals.rs:94-103`). Spec **omits** the **`Acquired`** column and reorders wallet.
  - **Income** rendered header = `Recognized, Kind, Business, BTC, USD FMV` (`income.rs:70-76`). Spec lists a
    **`wallet`** column that **does not exist** — and `IncomeRecognized` has **no wallet field at all**
    (`state.rs:213-217`: `recognized_at, sat, usd_fmv, kind, business`), so it is not merely un-rendered but
    **un-derivable**. Spec omits the real `Business` column.
- **Why it matters:** the spec says "the column cursor lands on each" listed column. As written the cursor
  would target columns that are not shown (and, for Income wallet, cannot be) and skip columns that are shown.
  This is the concrete per-view contract the implementer and KAT authors work from.
- **Fix:** re-enumerate each view's sortable set against the real headers above; for any rendered column
  deemed non-sortable, say so explicitly (cursor skips it) rather than inventing/omitting columns. Drop Income
  `wallet` and Holdings `term`; decide sort semantics for `Source`/`Pending` (Holdings), `Business` (Income),
  and `Acquired` (Disposals) or exclude them by name.

### I4 — Docs lockstep instruction is factually wrong: the TUI man pages are HAND-AUTHORED, not `make docs`-generated; and the viewer has no `?` overlay
- **Where:** SPEC lines 82-83 / 90: "regen `btctax-tui.1` + `btctax-tui-edit.1` key references (`make docs`) +
  the in-app `?` help overlay(s)."
- **Evidence:**
  - `make docs` **does not** regenerate the TUI pages — they are hand-authored: `xtask/src/docs.rs:98`
    ("Does NOT include the hand-authored TUI pages (`btctax-tui.1` / `btctax-tui-edit.1`)") and `docs.rs:361`
    ("hand-authored TUI pages document their tab set + keys"). Both pages hard-code the year binding that must
    change: `btctax-tui.1:68-69` (`.B \(<- / \(->` → "Previous / next tax year …") and
    `btctax-tui-edit.1:35-36` (`.B \(<- / \(->` → "Change tax year"), plus `btctax-tui-edit.1:63-64`
    (`.B s` → select-lots) and the `l` → link-transfer entry below it. These must be **hand-edited**; running
    `make docs` alone would ship stale docs.
  - There is a sync test `xtask/src/docs.rs:409-415` asserting the edit page lists specific action keys
    (`?`, `V`, `O`) as "a copy of the `?` overlay" — keep the man page and overlay consistent (adding `S`/`L`
    and the sort keys does not break it, but note the coupling).
  - The **viewer has no `?` overlay**. Its only key-hint surface is a persistent footer: `draw.rs:150`
    ("`←/→: change year`"). The editor has **both** a footer (`draw_edit.rs:188`, "`←/→: change year`") **and**
    the `?` overlay (`draw_edit.rs:1916` "`←/→ change year`", `:1921` "`s select-lots`", `:1922`
    "`l link-transfer`"). No CHANGELOG file exists in the repo (whole-tree search) and there is no tui README.
- **Why it matters:** docs lockstep is a named gate; an instruction that points at the wrong tool (`make docs`)
  and the wrong surface (a viewer `?` overlay that doesn't exist) will leave the user-facing key-change
  under-documented — exactly the "silent surprise" the spec elsewhere says to avoid (lines 101-102).
- **Fix:** enumerate the real surfaces to hand-edit: viewer footer `draw.rs:150`; editor footer
  `draw_edit.rs:188` + `?` overlay `draw_edit.rs:1916-1922`; man pages `btctax-tui.1:68-69` and
  `btctax-tui-edit.1:35-36` (year) + `:63-64`/`l`-entry (S/L rebind), edited **by hand** (not `make docs`).
  Replace "CHANGELOG/README note" with the surface that actually exists (README-less repo; use the man pages +
  overlay/footers, or create a CHANGELOG deliberately). Note the `docs.rs:409` sync test.

---

## Minor

### M-1 — "Default sort = by DATE" is ambiguous for Disposals (two date columns)
Disposals render **two** dates: `Disposed` and `Acquired` (`disposals.rs:94-95`). SPEC line 38 ("default =
by DATE, ascending") and gotcha "all rows have a single unambiguous date" are not true for this view. Name the
default (presumably `Disposed`/`disposed_at`, matching the existing per-flow ordering at `main.rs:3970`).

### M-2 — Per-leg row granularity makes the "(date, EventId/LotId)" tie-break non-unique / under-specified
The Disposals view is rendered **per leg** (`disposals.rs:51` `for leg in &disposal.legs`) — many rows share
the **same** `EventId` and `disposed_at`. So the spec's tie-break "(date, then a stable unique key:
EventId/LotId)" (line 43) is **not a total order** for this view, and `sort_is_stable_deterministic` (line 70)
cannot be guaranteed by that key alone. Either add a per-leg discriminator (e.g. `(EventId, leg_index)` or the
leg's `(acquired_at, wallet)`), or state that determinism relies on a **stable** sort over a deterministic
input order — and make the KAT assert the actual guarantee.

### M-3 — Existing regression tests assert arrow=year and WILL break; the plan adds new KATs but doesn't own the removals
The plan (lines 71-72, 88-89) adds `tax_year_moves_to_bracket_keys` but does not mention the **existing** tests
that will fail once `←/→` become the column cursor: viewer `left_right_changes_selected_year`
(`lib.rs:776`) and editor `left_right_on_browse_changes_selected_year` (`main.rs:~9494`). Call these out as
must-update/replace, not just "add new."

### M-4 — The shared `render()` signature must change; both call sites are a named touch-point
SPEC line 60-61 says "each view's draw fn builds its row vec … then applies `sort_rows`," but the draw fn is
**shared** (`tabs::{holdings,disposals,income}::render`, called by the viewer's `draw.rs` and the editor's
`draw_edit.rs:143-163`) and today takes `(frame, area, snap, year, table_state)` — it has **no** access to the
`ViewSort`/cursor state, which the spec puts on `App`/`Editor`. The signature must be extended to thread the
sort/cursor through, and **both** call sites updated. Name `draw_edit.rs` as a touch-point (it is a `pub` API
change consumed by the editor crate).

---

## Nit

### N-1 — TOTAL footer is structurally separate; note it's excluded from sort
Each view's TOTAL row is a `Table::footer(total_row)` (`disposals.rs:130-134`, `holdings.rs:111-115`,
`income.rs:97-101`), not part of `rows`, so it is naturally excluded from sorting. Worth a one-line note so an
implementer doesn't fold it into the sortable vec.

### N-2 — Holdings is not year-scoped; `[`/`]` is a no-op there (as `←/→` is today)
`holdings.rs:24,29,32` — Holdings ignores `year`. Fine and unchanged, but the spec could note that the tax-year
keys have no effect on the Holdings view to preempt a "bug?" question.

---

## Confirmed sound (no action)
- **Editor depends on `btctax-tui`** — `btctax-tui-edit/Cargo.toml:18` (`btctax-tui = { path = "../btctax-tui" }`)
  and `editor.rs:33` `use btctax_tui::…`. A shared `btctax-tui::sort` module is usable by both. ✔
- **Viewer and editor render identical columns** — both call the same `tabs::*::render`
  (`draw_edit.rs:143-163`), so **one** comparator table serves both. ✔
- **Rebind targets are free.** `Char('S')`, `Char('L')`, `Char('[')`, `Char(']')` have **no** binding anywhere
  in the editor (grep). `Char('h')`/`Char('l')` appear only inside **flow** handlers
  (`handle_bulk_resolve_choose_key` `main.rs:7138`, `handle_rc_choose_key` `main.rs:8271`) — a different match
  context, untouched by top-level rebinding. The viewer binds **none** of `s l h S L [ ]` (grep of
  `lib.rs`). ✔
- **Flow-level `Left`/`Right` are a separate context.** Top-level year keys live at `main.rs:407/411`;
  flow-level `Left`/`Right` live in per-flow `handle_*` fns (`main.rs:5536, 5879, 6261, 6775, 7638, 8002,
  8271`). Moving the top-level browse arrows to a column cursor does not touch them. ✔ (viewer year =
  `lib.rs:215/219`; editor year = `main.rs:407/411` — as cited.)

## Re-review note
This is round 1 on the DRAFT. Per the standard workflow, fold each finding, persist the fold, and re-review
(including after the last fold) until 0 Critical / 0 Important. I1–I4 are blocking.
