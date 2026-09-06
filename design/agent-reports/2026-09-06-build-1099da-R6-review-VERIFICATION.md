# VERIFICATION ledger — the R6 build seam review (`2026-09-06-build-1099da-R6-review.md`, 1C/2I/4M/1N)

Controller's machine-check at `c3a22a29`, before the fold.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | C-1 | the Form 8283 restriction gate runs only inside `if files_from_answers` (arm 2), while an 8283 is written on both arms | `admin.rs:806 if files_from_answers {` … `:827 donations_had_restrictions == Some(true)`; the 8283 write is outside | HOLD |
| 2 | C-1 | `printed.rs` records the now-broken invariant | `printed.rs:201` "`Some(true)` cannot arrive: the year refuses upstream." | HOLD |
| 3 | C-1 | the fifth cell (committed row, params-less, empty answers) reaches arm (3) with the row unread | `admin.rs:760 files_from_answers = answers_stored && !params_bundled`; arm (1) needs `params_bundled` | HOLD |
| 4 | I-1 | the readiness helper has no templates term | `cmd/tax.rs:474 stored_answers_reach_the_slice` (no `Form8949Map::for_year` probe) | HOLD |
| 5 | I-2 | the pre-2025 T8 kill exercises 2025 boxes | `kats.rs:79-145` every fixture is `mixed_rows()` on 2025 | HOLD |
| 6 | M-2 | `slice_map_gate` is scoped to arm (2) | `admin.rs:783-784` | HOLD |
| 7 | M-1, M-3, M-4, N-1 | wording / partition / the unanswered-restriction row | read | HOLD |

Disposition: C-1, I-1, I-2 block; folded next by one opus agent with the Minors and the Nit (all small),
each with its kill; then a sonnet re-verification; FR-62 closes at 0C/0I.
