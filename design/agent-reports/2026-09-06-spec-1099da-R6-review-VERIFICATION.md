# VERIFICATION ledger — the R6 design review (`2026-09-06-spec-1099da-R6-review.md`, 2C/5I/6M/2N)

Controller's machine-check at `cd7cfbd9`, before the fold.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | C-1 | the slice's Schedule D filler writes only `line3`/`line10`; `ScheduleDTotals` has no box dimension | `schedule_d.rs:78 map.line3`, `:96 map.line10`, nothing else; `forms.rs:480 ScheduleDTotals { st, lt }` | HOLD |
| 2 | C-2 | the input-form store's `commit` returns `NoTables` without params; the dispatch reads the committed table | `input_form_store.rs:262-292` (I-11 gate); `return_inputs.rs:140 SELECT 1 FROM return_inputs` | HOLD |
| 3 | I-1 | `TEMPLATE_YEARS` counts a year with ANY (pdf, map) pair; a partially ported year is a template year | `build.rs:103-109` per-stem pairing, `:181` the year list | HOLD |
| 4 | I-2 | the TUI export's exclusive mkdir precedes the broker screen | `tui/export.rs:184` mkdir; the screen runs inside `write_form_csvs` → `routed_8949_rows` (`render.rs:954`), called after | HOLD |
| 5 | I-3 | `report` returns `Err` on the uncomputable outcome | `cmd/tax.rs`: `ProfileOutcome::Uncomputable { detail } => return Err(CliError::Usage(detail))` | HOLD |
| 6 | I-5 | the 8283 restriction row lives in `screen_absolute`; the slice arm prints 8283 without it | `return_1040.rs:2667`; `admin.rs:888-890` slice 8283 state | HOLD |
| 7 | M-5 | the price-coverage check runs only in the `Filable` arm | `year_readiness.rs`: `Filable =>` … `prices_max_date`; `Slice | Preparing =>` without it | HOLD |
| 8 | I-4, M-1..M-4, M-6, N-1, N-2 | wording / citation claims | read against the spec and `admin.rs:700-720`, `year_readiness.rs::import_note` | HOLD |

Disposition: fold ALL into R6 now (spec edits + two new build tasks T8/T9); one more opus review round
scoped to the folded R6; then the build under FR-62.
