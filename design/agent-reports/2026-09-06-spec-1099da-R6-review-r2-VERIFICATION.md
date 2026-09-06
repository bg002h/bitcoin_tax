# VERIFICATION ledger — the R6 r2 review (`2026-09-06-spec-1099da-R6-review-r2.md`, 11/15 resolved, 4 partial; NEW 1C/4I/2M)

Controller's machine-check at `4ed35a03`, before the fold.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | C-3 | `impl Default for ReturnInputs` sets `filing_status: Single` | `return_inputs.rs:1355` | HOLD |
| 2 | C-3 | `resolve_core` reads `return_inputs::get` BEFORE `tax_profile::get`, so a committed row shadows the profile | `resolve.rs` fn body: step 1 at +9, step 2 at +36 | HOLD |
| 3 | C-3 | the TUI already saves the working return (block included) to the draft table on a params-less year | `tui-edit/main.rs:978 form_save_draft` | HOLD |
| 4 | I-7 | the slice arm resolves maps beyond the two the gate named | `admin.rs:757 Form8275Map::for_year`; the fillers resolve `Form1040Map`/`ScheduleDMap`/`Form8949Map` inside `fill_*` | HOLD |
| 5 | I-9 | `schedule_d.csv` is written and a three-artifact KAT exists | `render.rs:913 write_schedule_d_csv`; `kats.rs:533 schedule_d_totals_match_form8949_and_csv` | HOLD |
| 6 | I-6 | the full return's line pairing is `[I, C]` / `[L, F]` | `printed.rs::schedule_d_lines` (T4) | HOLD (by the T4 commit) |
| 7 | N-1 | stale cites: `admin.rs:467`/`:471` (real 526), `:573-600` (real 642, dispatch 670), `return_refuse.rs:853` (real 989) | grep of both files | HOLD |
| 8 | I-8, M-7, M-8 | wording/buildability claims | read against `map.rs` (`Option<AmountCols>`), `year_readiness.rs:54-100`, `lib.rs:182-185` | HOLD |

Disposition: fold all seven new findings and the four PARTIALs into R6 now; r3 scoped to the fold.
