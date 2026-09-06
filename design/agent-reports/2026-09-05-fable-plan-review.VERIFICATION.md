# Controller's machine-check ledger — `2026-09-05-fable-plan-review.md` (1C/8I/4M)

Written by the controller (not the reviewer) BEFORE folding anything, so the checks survive a context
loss and so `git diff` between this commit and the fold shows exactly what changed in response.
Every row is a command that was run on 2026-09-05 at HEAD `3f279d64`; nothing here is taken from
the report's own text. Verdicts: **HOLDS** (the cited fact is as stated), **HOLDS+** (holds, and the
check found more than the report claimed), **PARTIAL** (holds with a stated correction), **NOT LOCATED**
(the wording was not found; the finding's substance does not depend on it).

| finding | claim checked | check | verdict |
|---|---|---|---|
| C1 | TY2025 8949 maps set the box unconditionally to I | `forms/2025/f8949.map.toml` `box_on = "6"`; `design/forms/extract/f8949--2025.txt:33` = "(I) Short-term digital asset transactions not reported to you on Form 1099-DA" | HOLDS |
| C1 | `box_needs_review` reaches only an advisory, and the full-return export hardcodes it away | `rows_possibly_broker_reported` (`btctax-forms/src/lib.rs:435`) is its only reader; `admin.rs:862` computes it on the slice path; `admin.rs:1220 broker_reported_rows: 0` on the full-return path; `broker_reporting_advisory` (`:452`) returns `None` on 0 | HOLDS |
| C1 | brokers report basis from 2026-01-01 (TD 10000) | `legal/text/…/TD_10000` extract lines 3041/5050; `legal/research/REPORT_us_btc_tax_TY2025-2026.md:139` §9 "confidence HIGH (3-0)", box routing G/H/I J/K/L at `:151-157` | HOLDS |
| C1 fix size | "one line" | `Printed8949Row` (`printed.rs:37`) carries NO `box_needs_review`; `Printed8949` is built once at `printed.rs:110` from the real `Form8949Row`s (`packet.rs:579`). Fix = one derived `usize` on `Printed8949` + read it at `admin.rs:1220`. Two literal sites (`printed.rs:110`, test `:2724`) E0063. `printed.rs:2241 box_needs_review: false` is a TEST helper, not a second hardcode | PARTIAL — a field, not a line |
| I1 | `emitted_form_years()` is deliberately a function, not a const | `cite_check.rs:797` "the reason this is a function and not a `const`" | HOLDS |
| I1 | `STEM_ALIASES` is a two-row total alias table | `cite_check.rs:770` `[("schedule_d","f1040sd"),("schedule_se","f1040sse")]` | HOLDS |
| I1 | cross-product test joins the archive BY sha256 | `tests/supported_years_cross_product.rs:26-27,68-69` | HOLDS |
| I1 | a const table is 17 rows/year | `find forms -name '*.map.toml'` = 37 (2017: 5, 2024: 17, 2025: 15); 37 PDFs | HOLDS |
| I2 | `Form6251Map` is paired cell-by-cell with the printed struct | `form6251.rs` 39 `(&map.lineN, f.lineN)` pairs, last at `:191`; `map.rs:240 let Self {` exhaustive destructure in `money_cells` | HOLDS |
| I2 | the port report lists `schedule_1a.rs (56)` as the BAD example of the quantity axis | `TY2026_PORT_REPORT.md:298` | HOLDS |
| I3 | `TRANSITION_DATE`/`TY2025_RETURN_DUE` are singular consts; `FORMS_ABSENT_FROM_YEAR` lives in a test file; `SUPPORTED_YEARS` is a literal | `conventions.rs:17,19`; `tests/field_census.rs:100`; `btctax-forms/src/lib.rs:86` | HOLDS |
| I3/M3 | `BundledPrices::max_date()` exists; dataset must reach 12-31 of the filed year | `price.rs:26`; `data/btc_usd_daily_close.csv` last row **2026-06-03** | HOLDS |
| I4 | Rev. Proc. 2025-32 §2.10 publishes TY2026 §55(d) exemption amounts; §.07 records OBBBA §70107's §55(d)(4) change | `RevProc_2025-32.txt:56` ".10 Exemption Amounts for Alternative Minimum Tax"; `:220-228` §70107 / §55(d)(4)(B) | HOLDS |
| I4 | `ty2026()` is already transcribed from Rev. Proc. 2025-32 and KAT-pinned | `tax_tables.rs:592 fn ty2026() -> TaxTable`; `:696` source string; KATs `:1082,1098,1113,1126` | HOLDS — for the BRACKET table only |
| I4 | the fail-closed gate's reason 1 says the rate/thresholds are "unknown" | `tax_tables.rs:926-931` verbatim | HOLDS — reason 1 is stale; reasons 2 (1a/1b restructure) and 3 (no OTS 2026) still hold |
| I4 | `FullReturnParams`/`AmtParams` TY2026 exist | `AmtParams {` appears ONCE, at `:148` inside the TY2024 params; `full_return_for(2026)` = `None` by decision | the review's minimal change ("encode `AmtParams` TY2026 now") is real, unstarted work |
| I5 | the `{2024}` OTS excuse set "until someone reads `taxsolve_US_1040_2026.c`" | `design/TY2026_PORT_REPORT.md:489` (NOT in `scripts/oracle/`) | HOLDS — location corrected |
| I5 | "rule 13" | no `rule 13` string anywhere in `scripts/oracle/` | NOT LOCATED — the two-oracle rule is `CLAUDE.md` doctrine regardless |
| I6 | `Coverage::quoting(year)` exists and its doc says a carried-forward sentence REDS | `line_coverage.rs:193-195` | HOLDS |
| I7 | `CommitOutcome::NoTables` exists in the TUI | `tui-edit/src/edit/persist.rs:127`, `main.rs:1284` | HOLDS |
| I7 | the CLI prescribes `income clear` on the exit-2 path | `btctax-cli/src/lib.rs:129`, `resolve.rs:217` | HOLDS |
| I7 | `selected_year: 2025` is a literal | `tui-edit/src/editor.rs:298`, `tui/src/app.rs:193` — TWO sites | HOLDS+ |
| I8 | Schedule 1 is emitted, bundled for 2024 only, has no `--2025` authority, and has a 2026 draft; so the work list has no row | `packet.rs:104 "f1040s1"`; `forms/2024/f1040s1.*` only; `design/forms/2025/` 0 matches; `MANIFEST.json` 2 matches for `f1040s1--2026-DRAFT` | HOLDS |
| I8 | (controller's own) | `comm` of bundled-2025 stems vs work-list rows: **`f8283`** also has no row (prose only, `TY2026_WORK_LIST.md:39`); `schedule_d`/`schedule_se` appear under `f1040sd`/`f1040sse` — a naming seam, not an omission | HOLDS+ — a second instance of the same shape |
| M1 | `f8275` is revision-dated, not year-dated | `design/forms/extract/f8275--2024.txt:78` "Form 8275 (Rev. 10-2024)" | HOLDS |
| M1 | the fetch refusal reads "year printed on the document ≠ year requested" | not found in `crates/xtask/src` or `scripts/`; no `fetch` module in xtask | NOT LOCATED |
| M2 | `ROADMAP_STATUS.md` §3 still commits to TY2025 after 2026-10-15 | `:147` | HOLDS |
| Layer 1 | attachment sequence is a per-form literal in `packet.rs` | 16 `Some("NN")` literals (`:186 "55"`, `:207 "71"`, `:210 "72"`, `:218 "92"`, `:223 "155"`, …) | HOLDS |
| Layer 2 | `fill_full_return` destructures `PrintedForms` exhaustively | `packet.rs:60-83`, "★ NO `..`", 17 members | HOLDS |
| Layer 2 | publishing trap: a `build.rs` must read only under the manifest dir and the tarball must carry `forms/` | no `build.rs` anywhere in the workspace today; `btctax-forms/Cargo.toml` has no `include`/`exclude`; `forms/2025/*` not gitignored | feasible; the `cargo package --list` gate the review asks for is the right kill |

## What the ledger settles for the fold

1. **C1 is real and the fix is bounded**: add `possibly_broker_reported: usize` to `Printed8949`
   (derived at construction from `box_needs_review`), read it at `admin.rs:1220`, and plant the kill —
   a full-return export with one exchange-wallet disposition must print the advisory. Then the
   year-record slot (`information_returns`) and the filer's `broker_reported` answer are design work,
   not this fold.
2. **My draft's §5 ("a generated row cannot carry a judgment") is answered, not refuted**: the review
   puts the judgments (`versioning`, `line_set`, `instructions`) in the `.map.toml` HEADER the human
   already writes, and leaves `build.rs` only the mechanical binding. That is my own "codegen for the
   consts, never for the row" — with the row moved out of Rust. Adopt it. `Revision` → `versioning`;
   `MapFamily` → `line_set`; the Rust `const FORMS` is retired.
3. **I4 changes the calendar, not just a sentence**: `AmtParams`/`FullReturnParams` TY2026 is a
   NOW item with a primary source in the tree; only reasons 2 and 3 keep the gate closed.
4. **I5's partition (NOW / AFTER FINALS / AFTER OTS 2026) and Form 4868-as-default** go into
   `ROADMAP_STATUS.md` §3, replacing the stale TY2025 commitment (M2 closes with it).
5. **I8 needs an instrument, not a re-run**: the work list must enumerate from the emitting surface
   and print `NO PRIOR SIDE` / `NO DRAFT` / `NO FINAL` per cell. Two forms fell out today (f1040s1,
   f8283).
