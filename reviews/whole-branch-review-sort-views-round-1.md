# Whole-diff review (Phase E) — feat/sort-views — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (1837917)..HEAD` — 3 task commits (T1-T3), 19 files, +2182/−132.
Contract: `design/SPEC_sort_views.md` (R0-GREEN, 3 rounds). `btctax-tui` + `btctax-tui-edit`; display-only UI.

## Fault-injection (restored byte-for-byte)
- **[★ sort correctness] CONFIRMED load-bearing.** `stable_sort_by` (sort.rs:106) applies `dir` to the primary
  key (`Dir::Desc => base.reverse()`) then a tie-break that STAYS ascending (the R0-M-2 total-order design).
  **Fault-inject:** dropping the `.reverse()` on `Desc` (so descending wrongly yields ascending) drove
  `sort_by_btc_asc_desc` RED. The direction logic is guarded.
- **[display-only] structurally guaranteed + KAT'd.** Sorting operates on TYPED BORROWED working sets
  (`Vec<&Lot>` / per-leg `Vec<(&Disposal,&DisposalLeg,usize)>` / `Vec<&IncomeRecord>`) then formats [R0-I2] —
  the read-only `snap.state.*` is never mutable in that path (borrow-enforced, so not naturally fault-
  injectable). `sorting_does_not_mutate_events_or_state` passes in BOTH crates (tabs/tests.rs + editor main.rs);
  `edit_flow_targets_are_independent_of_display_order` confirms the R0-I1 safety basis (flows build own pickers).

## Verified by inspection + named KATs
- **Real columns / typed sort** [R0-I3]: per-view comparators over the actual rendered columns (Income has no
  wallet). KATs `sort_by_{btc,usd_basis,proceeds,kind_and_fmv}_asc_desc`, `default_sort_is_<date>_ascending`.
- **Total order** [R0-M-2]: `per_leg_ties_keep_leg_index_order` + `tie_break_is_event_id_and_year_filtered`
  (Disposals per-leg, stable).
- **Keys**: `cursor_moves_h_l_and_arrows`, `s_toggles_direction_on_repeat`, `sort_state_is_per_view`,
  `tax_year_moves_to_bracket_keys`, `holdings_year_keys_are_noop_on_view`.
- **Editor rebind** [R0-I5]: `editor_s_now_sorts`, `editor_shift_s_opens_select_lots`,
  `editor_shift_l_opens_link_transfer`. **All 23 browse rebind tests migrated** (15 `s`→`S` + 8 `l`→`L`) +
  the 2 arrow=year tests (lib.rs, main.rs:9493) — AND the implementer caught 2 MORE viewer arrow=year tests in
  `tabs/tests.rs` that the spec's line-list missed (both migrated + passing). Good catch; no gap remains.
- **Docs** [R0-I4]: hand-authored man pages + footers + editor overlay + README note; the docs.rs:410-415 sync
  test (`?`/`V`/`O`) untouched + green; a groff glyph fix (`\(<->` → "ascending / descending").

## Suite
`cargo test --workspace --locked` 1189 passed / 0 failed (implementer; re-run in progress at merge); clippy -D +
fmt clean. MINOR (new capability + the user-facing `←`/`→`→`[`/`]` year key change + editor `s`/`l`→`S`/`L`).

**SHIP.**
