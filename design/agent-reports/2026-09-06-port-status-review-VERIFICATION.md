# VERIFICATION ledger — port-status verification (2026-09-06-port-status-review.md)

Controller: Claude Fable 5.1, 2026-09-06, tree after `2d87db37`.

| # | claim | check | verdict |
|---|---|---|---|
| 1 | the printer hardcodes NO DRAFT for any tag; the checker matches only that literal; `NO FINAL` appears nowhere | `grep -c 'NO FINAL' crates/xtask/src/form_delta.rs` → 0 (mine, by construction) | **TRUE** |
| 2 | the shape column is dropped by `parse_work_list_row` and the test | the test's `.map(\|(form, cells, _, _)\| (form, cells))` (mine) | **TRUE** |
| 3 | the excused rows' prose is unheld; a literal paste would destroy it | the test compares excused rows by form only (mine); the doc's `f8275` row carries the periodic-alias prose | **TRUE** |
| 4 | `emitting_surface` degrades silently where the old scan panicked | `.into_iter().flatten()` / `let Ok(..) else continue` (mine) | **TRUE** |
| 5 | both sides missing prints only NO DRAFT in the cell column | the `cell` expression (mine) | **TRUE** |
| 6 | the doc's "Until `forms port-status <year>` exists" sentence is stale | `grep -n 'Until' design/TY2026_WORK_LIST.md` | **TRUE** |
| 7 | the catch-all usage omits `form-delta` and `port-status` | `main.rs` `_ =>` arm | **TRUE** |
| 8 | the trailing plant re-parses the printer's text and never exercises the printer | the plant (mine) | **TRUE** |

**Verdict: 8/8 TRUE.**
