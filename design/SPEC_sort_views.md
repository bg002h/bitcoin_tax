# SPEC — sortable Holdings / Disposals / Income views (viewer + editor)

**Source baseline:** `main` @ `1837917` (branch `feat/sort-views`). **Review status: R0-GREEN (3 rounds; 0C/0I).
Cleared to implement.** Reviews: `reviews/R0-spec-sort-views-round-{1,2,3}.md`. Round 1 0C/4I (my column/arch
facts), round 2 0C/1I (I5 the 23-test rebind migration), round 3 0C/0I/0M/1N. **R0 confirmed NO data-safety
bug** (edit flows build their own pickers — see Safety). **[R0-N-2] T3 note:** the man-page sync test
(docs.rs:410-415, asserting `?`/`V`/`O`) is NOT touched by the T3 hand-edits — verified stays green.
User-approved design (2026-07-05).

## Goal
Interactive column sorting of the **Holdings**, **Disposals**, **Income** row views in BOTH `btctax-tui`
(viewer) and `btctax-tui-edit` (editor). Display-only — sorting NEVER mutates ledger data/events. Summary tabs
(Tax/Forms/Compliance) + the CLI are out of scope.

## Finalized key map (top-level browse handler; both apps)
| Keys | Action |
|---|---|
| `j`/`k` + `↑`/`↓` | scroll rows (UNCHANGED) · `g`/`G` top/bottom (UNCHANGED) |
| `h`/`l` + `←`/`→` | move the **column cursor** (highlight the focused column) |
| `s` | **sort** by the focused column — toggles asc↔desc; focusing a NEW column sorts ascending first |
| `[` / `]` | **tax year** prev / next — **MOVED off `←`/`→`** (viewer lib.rs:215/219; editor main.rs:407/411) |

**Editor-only rebinds** (free `s`/`l` for sorting — user-directed): `s` (select-lots, main.rs:421) → **`S`**;
`l` (link-transfer, main.rs:423) → **`L`**. R0-confirmed `S`/`L`/`[`/`]` are unbound in the editor; `h` appears
only inside flow contexts (main.rs:7138/8271); `l` IS bound top-level (main.rs:423) and is freed by this rebind
[R0-N-3]. So the top-level rebinds are conflict-free. **[R0-I5] The rebind ALSO breaks 23 browse-level tests
that press top-level `s`/`l` — they must migrate to `S`/`L` (see KATs).** No dedicated force-descending key —
`s` toggling covers both directions.

## [R0-I3] Sortable columns = EXACTLY what each view renders today (verified against source)
The column cursor lands on each RENDERED column; sorting keys off the underlying TYPED field (not the formatted
string).
- **Holdings** (tabs/holdings.rs:51-57, source `snap.state.lots`): `Wallet`(WalletId) · `Acquired`(Date) ·
  `BTC`(amount) · `USD Basis`(Decimal) · `Source`(BasisSource) · `Pending`(bool).
- **Disposals** (tabs/disposals.rs:58-66, source `snap.state.disposals` flattened **per-leg**):
  `Disposed`(Date) · `Acquired`(leg.acquired_at Date) · `BTC` · `Proceeds`(Decimal) · `Basis`(Decimal) ·
  `Gain`(Decimal) · `Term`(short<long) · `Wallet`(WalletId).
- **Income** (tabs/income.rs:41-45, source `snap.state.income_recognized`): `Recognized`(Date) ·
  `Kind`(IncomeKind) · `Business`(bool) · `BTC` · `USD FMV`(Decimal). **[R0-I3] NO wallet column** —
  `IncomeRecord` has no wallet field (state.rs:213-217); do not invent one.

## Semantics
- **Default sort = by the primary DATE column, ascending**, per view: Holdings→`Acquired`, Disposals→`Disposed`
  [R0-M-1] (the disposal date, not Acquired), Income→`Recognized`.
- `s`: focus a new column → sort ascending by it; focus the active sort column → toggle direction. Wallet sorts
  by (provider, account); term short<long; Source/Kind by their enum order; bool false<true; numerics
  numerically; dates chronologically.
- **[R0-M-2] Total, deterministic order.** Disposals render PER-LEG and many legs share
  (disposed_at, EventId), so the tie-break is **(sort key, then disposed_at, EventId, leg index)** — a total
  order — applied via a STABLE sort. Holdings/Income tie-break on (sort key, then the item's stable id). No RNG.
- Sort state is **per-view, session-only** (`{sort_col, dir}` + `cursor_col` per view); not persisted; reset on
  reload; each tab keeps its own.
- **[R0-N-1] The `TOTAL` footer is excluded from sorting** — it is a separate `Table::footer`
  (holdings.rs:76 / disposals.rs:82 / income.rs:61), not a member of `rows`, and always stays at the bottom.
- **[R0-N-2]** Holdings is NOT year-scoped (holdings.rs:24,29), so `[`/`]` is a no-op on the Holdings tab
  (year still steps on Disposals/Income) — acceptable; note it in help.

## [R0-I1] Safety — display-only, and why it's safe (the round-1 "stable-key" claim was counterfactual)
R0 traced this: the tab selection (`*_state: TableState`, app.rs:139-141 / editor.rs:96-98) is a bare ratatui
INDEX read ONLY by scroll helpers (main.rs:8771-8778) and cleared by `reset_selections` (main.rs:8885). The
editor's edit flows do NOT resolve their target from it — each flow builds its OWN picker list fresh from
`snap.state.*` (e.g. `open_select_lots_flow` → `TargetList::new` main.rs:3980). **Therefore reordering the
display rows CANNOT retarget an edit.** So: sorting reorders display rows only; it must not mutate
`events`/`LedgerState`. The scroll highlight is index-based; on a re-sort it simply stays at its row index
(cosmetic — no correctness impact). No stable-key selection mechanism is needed or claimed.

## [R0-I2] Architecture — sort TYPED data, then format (NOT `Vec<ratatui::Row>`)
The rendered rows are `Vec<ratatui::Row>` of formatted strings with no readable typed fields, so sorting must
happen BEFORE formatting:
- Each view builds a typed, borrowed working set from the snapshot — Holdings `Vec<&Lot>`; Disposals
  `Vec<(&Disposal, &DisposalLeg, usize /*leg idx*/)>` (flatten); Income `Vec<&IncomeRecord>` — sorts THAT
  by the focused column + dir + tie-break, THEN formats the sorted items into `Row`s (the existing formatting,
  reordered). The read-only `snap.state.*` is never mutated (we sort borrows/indices).
- **Shared `btctax-tui::sort`**: the `Dir {Asc,Desc}` + `ViewSort {col, dir}` + `cursor` types + toggle/step
  helpers (shared, unit-tested). The per-view COMPARATOR (col→typed field) lives in each view module (it knows
  its own row type); a small shared `stable_sort_by(items, cmp, dir, tiebreak)` applies it uniformly.
- **[R0-M-4]** the `render`/`draw` fns for the 3 views gain a `ViewSort` + `cursor` param; BOTH call sites —
  viewer `draw.rs:137-139` and editor `draw_edit.rs:143-163` — pass the app's per-view sort/cursor state.
- **State:** add `{holdings,disposals,income}_sort: ViewSort` + `_cursor: usize` to the viewer `App` (app.rs)
  and editor `Editor` (editor.rs), mirroring the existing `*_state: TableState` fields.
- **Keys:** extend the two top-level browse handlers (viewer lib.rs:205-248; editor main.rs:~403-430) — cursor
  h/l+arrows, `s` sort, `[`/`]` year; editor also select-lots→`S`, link-transfer→`L`. Flow-level Left/Right
  (main.rs:5536+ etc.) are a separate open-flow match context — untouched.

## KATs
- Sort correctness (pure, on the typed working set): `sort_by_<col>_asc_desc` per view (distinct fixture →
  assert typed ORDER after each column + a toggle); `default_sort_is_primary_date_asc`;
  `sort_is_total_and_stable` (esp. Disposals per-leg — equal (key,date,EventId) legs keep leg-index order).
- Keys: `cursor_moves_h_l_and_arrows`; `s_toggles_direction_on_repeat`; `sort_state_is_per_view`;
  `tax_year_moves_to_bracket_keys` (`[`/`]` step year; arrows no longer do); `holdings_year_keys_are_noop`.
- Editor rebind: `editor_S_opens_select_lots`, `editor_L_opens_link_transfer`, `editor_s_now_sorts`.
- **[R0-I1] display-only:** `sorting_does_not_mutate_events_or_state` (events/LedgerState byte-identical
  before/after any sort); `edit_flow_targets_are_independent_of_display_order` (a flow's picker list is built
  from `snap.state.*` regardless of the tab's current sort/cursor).
- **[R0-M-3]** UPDATE the existing arrow-steps-year tests: viewer `lib.rs:776`, editor `main.rs:9493` (they
  assert `←`/`→` change the year — now `[`/`]` do). The plan owns these edits, not just new KATs.
- **[R0-I5] MIGRATE the 23 browse-level rebind tests** — the **15** that press top-level `s` to open select-lots
  (main.rs ~14735-18402, e.g. :14738 "select_lots_flow must open on 's'") and the **8** that press top-level
  `l` for link-transfer (main.rs ~18328-19478, e.g. :18522) → press `S`/`L`. Both are top-level openers (the
  flow key handlers `handle_select_lots_flow_key`:3197 / `handle_link_transfer_flow_key`:4247 bind neither `s`
  nor `l` internally), so all 23 break on the rebind unless migrated. This is a mechanical find-and-replace in T2.

## Scope / SemVer / lockstep
`btctax-tui` (sort module + viewer keys/state/render) + `btctax-tui-edit` (editor keys incl. `S`/`L` rebind +
state/render). MINOR (new capability + a user-facing KEY CHANGE: `←`/`→` no longer step the year — `[`/`]` do;
editor select-lots/link-transfer → `S`/`L`). **[R0-I4] Docs are HAND-AUTHORED, not `make docs`** (xtask
generates only the CLI pages, docs.rs:98/361). Hand-edit: the year hint in `docs/man/btctax-tui.1:68-69` +
`btctax-tui-edit.1:35-36` and the `s`/`l` hints at `btctax-tui-edit.1:63-64`; the viewer footer hint
(draw.rs:150) + editor footer (draw_edit.rs:188) + editor help OVERLAY (draw_edit.rs:1916-1922). (Viewer has NO
overlay — footer only.) Add the key change to README (no CHANGELOG file exists).

## Plan (TDD)
- **T1** — `btctax-tui::sort` (types + `stable_sort_by`) + per-view comparators + the pure sort KATs; wire the
  VIEWER: App state, the 3 typed-working-set + sorted render, key handler (cursor/`s`/`[`/`]`, arrows→year
  removed), header highlight + `▲`/`▼`; viewer key KATs + update `lib.rs:776`.
- **T2** — wire the EDITOR: Editor state, render, key handler INCLUDING select-lots→`S` / link-transfer→`L` +
  the year `[`/`]` move; editor rebind + display-only + edit-independence KATs + update the arrow=year test
(`main.rs:9493`) + **[I5] migrate the 23 browse `s`/`l`→`S`/`L` opener tests** (mechanical find-and-replace).
- **T3** — hand-edit the man-page + footer/overlay hints + README note; whole-diff + full suite.

## Gotchas
- **[I2] sort typed data, not `Vec<Row>`** — build a typed borrowed working set, sort, then format.
- **[I3] use the REAL columns** — Income has NO wallet; don't invent columns.
- **[I1] display-only** — never mutate events/state; edit flows are already independent of display order (KAT).
- **[M-2] Disposals are per-leg** — total order needs the leg-index tie-break + a stable sort.
- **[I4] hand-edit the docs** — TUI man pages aren't generated; viewer has no `?` overlay.
- **[M-3] update the existing arrow=year tests**; **[N-1] footer excluded**; **[N-2] Holdings `[`/`]` no-op**.
- Deterministic; no RNG.
