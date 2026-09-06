# VERIFICATION ledger — the R6 r3 review (`2026-09-06-spec-1099da-R6-review-r3.md`, 7/11 resolved, 4 partial; NEW 1C/4I/3M/1N)

Controller's machine-check at `5f03b965`, before the fold.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | C-4 | `input_form_store::load` reads the DRAFT first (a draft shadows the committed row); the fold's accessor said committed-over-draft | `input_form_store.rs:180-216`: draft row first, then the committed tail | HOLD |
| 2 | I-10 | the stale split and the `parked` flag live in `load` (§6.3) and the fold's accessor named neither | `:183-203` (`StaleParkedDraft`, discard + `StaleNote`, `parked`) | HOLD |
| 3 | I-11 | with no `tax_profile`, a draft-only vault's `report` is `NotComputable [TaxProfileMissing]`, exit 1 | `compute.rs` (r2 ledger #2) and `main.rs` exit rule | HOLD |
| 4 | I-12 | `Form8949Part` has no `Ord` | `forms.rs:22 #[derive(Debug, Clone, Copy, PartialEq, Eq)]` | HOLD |
| 5 | I-13 | `kats.rs::schedule_d_totals_match_form8949_and_csv` reads no CSV; `btctax-forms` has no CSV writer | the KAT names "csv" in its identifier/comments only; `write_schedule_d_csv` is in `btctax-cli/src/render.rs` | HOLD |
| 6 | M-11 | `wants()` can put `Form1040Map` out of reach under `--forms` | `admin.rs:550 wants`, `:926 fill_form_1040_capgains` | HOLD |
| 7 | M-9, M-10, N-3 | call sites / refusal sentences / the stamp | read | HOLD |

Disposition: fold all; r4 scoped to this fold's design cells (precedence, stale/parked, the three sub-states); build-level wording (I-12, I-13, M-9) is folded but its proof is the build.
