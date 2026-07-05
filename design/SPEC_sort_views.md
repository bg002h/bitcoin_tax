# SPEC — sortable Holdings / Disposals / Income views (viewer + editor)

**Source baseline:** `main` @ `1837917` (branch `feat/sort-views`). **Review status: DRAFT — awaiting R0.**
Interactive column sorting of the row-based views in BOTH `btctax-tui` (viewer) and `btctax-tui-edit` (editor).
Brainstormed + user-approved design (2026-07-05); all key decisions settled below.

## Goal
Let the user re-order the rows of the **Holdings**, **Disposals**, and **Income** views by any displayed column
(date, wallet, USD value, BTC value, term, …), via a column cursor + a sort key, working IDENTICALLY in the
viewer and the editor. Display-only — sorting NEVER mutates ledger data or events. Summary tabs
(Tax/Forms/Compliance) have no per-row data and are out of scope. CLI report is out of scope.

## Finalized key map (top-level browse handler; both apps)
| Keys | Action |
|---|---|
| `j`/`k` + `↑`/`↓` | scroll rows (UNCHANGED) |
| `g`/`G` | top / bottom (UNCHANGED) |
| `h`/`l` + `←`/`→` | move the **column cursor** (highlight the focused column) |
| `s` | **sort** by the focused column — toggles asc↔desc; focusing a NEW column sorts ascending first |
| `[` / `]` | **tax year** prev / next — **MOVED off `←`/`→`** (viewer lib.rs:215/219; editor main.rs:407/411) |

**Editor-only rebinds** (free `s`/`l` for sorting — user-directed):
- `s` (select-lots, main.rs:421) → **`S`**
- `l` (link-transfer, main.rs:423) → **`L`**

Rationale: in the viewer `s`/`l` are free; in the editor they are edit-flow keys, so select-lots/link-transfer
shift to `S`/`L` (verified free in the editor browse handler — R0 must confirm no `S`/`L`/`[`/`]` binding exists
elsewhere). The sort keys (`s`, `h`/`l`, arrows, `[`/`]`) are then identical in both. No dedicated
"force-descending" key — `s` toggling covers both directions (supersedes the earlier `S`-descending idea, since
`S` is now select-lots).

## Sortable columns (the column cursor lands on each; approved)
- **Holdings:** acquisition date · wallet (provider→account) · BTC amount · USD basis · term (short/long)
- **Disposals:** disposal date · wallet · BTC amount · USD proceeds · USD basis · USD gain · term
- **Income:** date · wallet · BTC amount · USD FMV · kind (mining/staking/interest/…)

## Semantics
- **Default sort = by DATE, ascending** (per view). (Supersedes "natural order" — an explicit deterministic
  default, so the views open chronologically.)
- `s`: if the focused column is not the active sort key → sort ascending by it; if it IS → toggle its
  direction. Wallet sorts by (provider, account) lexicographically; term by short<long; numeric columns
  numerically. **Ties break by (date, then a stable unique key: EventId/LotId)** so the order is fully
  deterministic (NFR: no RNG, stable across renders).
- Sort state is **per-view, session-only** (each of the 3 views remembers its own `{sort_col, dir}` +
  `cursor_col`); NOT persisted to disk; reset on reload. Switching tabs preserves each view's state.
- **Display-only invariant** — sorting reorders the DISPLAY rows only. It must not touch `events`,
  `LedgerState`, any `TableState` selection identity semantics, or (editor) any edit/persist path. The editor's
  edit actions operate on the row the cursor/selection points to AFTER sorting (the selected row is identified
  by its stable key, not its index, so a re-sort keeps the right row selected).

## Architecture
- **Shared module `btctax-tui::sort`** (the editor depends on `btctax-tui`, editor.rs:33): a `SortCol` per view
  (or a view-tagged column index), a `Dir {Asc,Desc}`, a `ViewSort {col, dir}` struct, and a pure
  `sort_rows(view, rows: &mut Vec<Row>, sort: ViewSort)` / comparator keyed off the row's typed fields (NOT the
  formatted strings — sort by the underlying `Date`/`Decimal`/`WalletId`/term, so `$1,000` vs `$900` order
  numerically). One comparator table per view; both apps call it.
- **State:** add `{holdings,disposals,income}_sort: ViewSort` + `_cursor: usize` to the viewer `App` (app.rs)
  and the editor `Editor` (editor.rs) — mirrors the existing per-view `*_state: TableState` fields. Default
  `ViewSort { col: <date>, dir: Asc }`.
- **Render:** each view's draw fn builds its row vec from the snapshot (as today), then applies `sort_rows`
  before rendering; the focused/sort columns get a header highlight + `▲`/`▼` indicator.
- **Keys:** extend the two top-level browse handlers (viewer lib.rs:205-248; editor main.rs:~395-430) —
  cursor h/l+arrows, `s` sort, `[`/`]` year; editor also moves select-lots→`S`, link-transfer→`L`. Flow-level
  Left/Right handlers (editor main.rs:5536+ etc.) are inside open flows/modals — untouched (different match
  context; the cursor/year keys only act in top-level browse with no flow open).

## KATs
- `sort_by_<col>_asc_desc` per view (a fixture with distinct dates/wallets/USD/BTC → assert row ORDER after
  each sort key + a toggle); `sort_is_stable_deterministic` (equal-key rows keep (date, id) order across
  re-sorts); `default_sort_is_date_ascending`.
- `cursor_moves_h_l_and_arrows`; `s_toggles_direction_on_repeat`; `sort_state_is_per_view` (each view
  independent); `tax_year_moves_to_bracket_keys` (`[`/`]` change year; arrows no longer do).
- **Editor rebind:** `editor_S_opens_select_lots` + `editor_L_opens_link_transfer` (the moved keys still work);
  `editor_s_now_sorts_not_select_lots`.
- **★ display-only invariant:** `sorting_does_not_mutate_events_or_state` (event/LedgerState byte-identical
  before/after any sort); `editor_edit_after_sort_targets_correct_row` (select-lots on the cursor row after a
  re-sort hits the intended disposal by stable key, not stale index).

## Scope / SemVer / lockstep
`btctax-tui` (sort module + viewer keys/state/render) **+ `btctax-tui-edit` (editor keys incl. the S/L rebind +
state/render). MINOR (new capability + a user-facing KEY CHANGE: `←`/`→` no longer step the tax year — `[`/`]`
do; editor select-lots/link-transfer move to `S`/`L`). Lockstep: regen `btctax-tui.1` + `btctax-tui-edit.1`
key references (`make docs`) + the in-app `?` help overlay(s); add a CHANGELOG/README note for the key change.

## Plan (TDD)
- **T1** — `btctax-tui::sort` module + comparators + the sort KATs (pure, no UI); wire the VIEWER (App state,
  the 3 draw fns, key handler: cursor/s/`[`/`]`, arrows→year removed) + viewer KATs + indicators.
- **T2** — wire the EDITOR (Editor state, draw fns, key handler) INCLUDING the select-lots→`S` /
  link-transfer→`L` rebinds + the year `[`/`]` move; the editor rebind + display-only + edit-after-sort KATs.
- **T3** — `make docs` (both man pages) + `?` overlays + CHANGELOG/README key-change note; whole-diff + full suite.

## Gotchas
- **[★] Display-only** — sorting must not mutate `events`/`LedgerState`/edit paths (KAT); the editor's selected
  row is tracked by STABLE KEY so a re-sort doesn't retarget an edit to the wrong row.
- **Editor rebind conflict-check** — confirm `S`, `L`, `[`, `]` are unused in the editor before assigning (R0);
  the moved select-lots/link-transfer must keep working under the new keys.
- **Year keys only at top-level browse** — the many flow-level Left/Right handlers (in open modals/flows) are
  a different match context and stay as-is; only the top-level browse Left/Right become the column cursor.
- **Sort the typed values, not formatted strings** — `Decimal`/`Date`/`WalletId`/term, so numbers/dates order
  correctly (not lexically).
- **User-facing key change** — `←`/`→` no longer step the year; document it (help overlay + man + CHANGELOG) so
  it isn't a silent surprise.
- **Deterministic** — stable tie-break by (date, id); no RNG.
