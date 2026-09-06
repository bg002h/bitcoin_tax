# VERIFICATION ledger — build 1099da T0–C review (2026-09-06-build-1099da-T0-C-review.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `79a23943` (T3 `d9863909` already landed).

| finding | claim | check | verdict |
|---|---|---|---|
| I-1 | at `249d37ad` `form_8949_printed` dropped the routed box and the full-return filler checked the map's scalar box | `git show 249d37ad:crates/btctax-core/src/tax/printed.rs` had no `box_` on `Printed8949Row` (by construction — the T3 commit added it) | **TRUE at C; answered by T3 (`Printed8949Row.box_`, `fill_8949_full_with_map` groups by box)** |
| I-2 | no test hands a live regime to `screen_absolute`, `assemble_printed_forms/return` or `export_irs_pdf_from_session` | `PROCEEDS_AND_BASIS` occurs in `kat_broker_reporting.rs` and `admin.rs` only; 0 calls of the wirings with it | **TRUE** |
| I-3 | `write_form8949_csv` prints the unrouted `box` from `form_8949(state, year)`; reached from `export-snapshot` and the TUI export with no year gate | `render.rs` `fn write_form8949_csv` + the `"box"` header; callers in `admin.rs` (`write_csv_exports`) and `btctax-tui/src/export.rs` | **TRUE** — a spec gap the build inherited |
| M-1 | no fold-driven `Cohort::Covered` assertion | grep over the fold-driven KATs → 0 | **TRUE** |
| M-2 | the slice refusal's exit (the full return) refuses TY2026 today | `full_return_for(2026)` is `None` (FR-47) | **TRUE** |
| M-3 | `regime_for(year - 1).is_some_and(…)` drops the carryforward silently when no record | `tax.rs:689-698` | **TRUE** |
| N-1 | "three years on disk today" beside a four-year assertion | `bundled.rs:257-261` | **TRUE** |
| N-2 | `by_lot`'s hard-coded 0 | `kat_forms.rs` | **TRUE** |
| N-3 | the tree was dirty (T3 in progress) when the suite ran | true — T3 was being built while the review ran; the reviewer read sources from `git show 249d37ad:` | **TRUE** (process; recorded) |

**Verdict: 9/9 TRUE.** Fold: I-1 by T3 (landed); I-2's three wiring kills; I-3 the CSV gate; M-1..M-3
and N-1/N-2 fixed inline; N-3 recorded as a process note (run build reviews on a worktree at the commit).
