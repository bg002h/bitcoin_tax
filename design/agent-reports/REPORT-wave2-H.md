# REPORT — wave 2, parcel H

FR-212/213/214 (four stale claims) + FR-166/167 (two surviving year defaults). Owned files only:
`design/ROADMAP_STATUS.md`, `design/SPEC_1099da_broker_reporting.md`, `scripts/oracle/verify_schedule_1a.py`,
`scripts/oracle/check_return.py`. No premise disproved — all five FOLLOWUPS entries checked out against
current source.

## FR-212 — `ROADMAP_STATUS.md:63`, "only two text cells differ" on Form 6251

Ran the cited tool myself (not just re-quoted FOLLOWUPS.md's table):
`CARGO_TARGET_DIR=<worktree>/target-h cargo run -p xtask -- form-delta f6251--2025 f6251--2026-DRAFT`.
Verbatim: *"8 of 59 COMPARED surviving line number(s) now print a MATERIALLY DIFFERENT caption"* —
lines **1a, 4, 5, 7, 18, 19, 25, 39**. Cross-checked line 5 directly against both extracted text
layers (`design/forms/extract/f6251--2025.txt:53-55` vs `f6251--2026-DRAFT.txt:91-93`):
Single/HoH $626,350→$500,000, MFJ/QSS $1,252,700→$1,000,000, MFS $626,350→$500,000 — matches the
brief exactly. Confirmed the reassuring half too: `crates/btctax-adapters/src/tax_tables.rs:311-312`
already carries `phaseout_start_single_hoh_mfs: dec!(500000)` / `phaseout_start_mfj_qss:
dec!(1000000)` — no figure is wrong today.

Also found and fixed in passing: the row's own citation `TY2026_WORK_LIST.md:41` had **already gone
stale** — that document was regenerated 2026-09-13 ("three axes were added and the numbers moved on
every row"), and line 41 is now a prose bullet, not the f6251 row (currently line 120). Same shape as
FR-152 (an absolute line-number anchor into a generated file). Replaced with a citation to "the
`f6251` row of `TY2026_WORK_LIST.md`" (no absolute line number) plus the tool command.

Edited: `ROADMAP_STATUS.md:63`'s table cell — now states 8 (not 2), lists all eight lines, explains
line 5's taxpayer-adverse direction, and states the tax_tables.rs figures are already correct (wrong
price, not a wrong number).

## FR-213 — `SPEC_1099da_broker_reporting.md:358`, "both forms unchanged"

Verified against `TY2026_WORK_LIST.md`'s rows: `f1040sd` (Schedule D) genuinely prints `unchanged`
(0 added/removed/respelled/moved, 0 line-caption changes, 0 line numbers unread — its 4 unread
BOXES sit beside no printed line on either revision, a different axis). `f8949` prints
`UNWITNESSED` with caption axis "0 of 2" line numbers compared (both unread). So the spec's claim
was true for Schedule D and false for Form 8949 — restated as **unwitnessed**, not merely wrong,
per the distinction FR-211 added a column for. Edited the "What R6 does not change" paragraph
accordingly; the row-count claim ("the port is two rows") is unaffected and preserved.

## FR-214 — 1040 line-numbering rests on one indirect witness

Confirmed: no `f1040--2026*` or `i1040gi--2026*` file exists anywhere under `design/forms/`
(`find` returned nothing) — the 1040 itself is `NO DRAFT`. Confirmed the citation independently:
`f6251--2025.txt` line 65 cites *"Form 1040 or 1040-SR, line 7"*; `f6251--2026-DRAFT.txt` line 103
cites *"...line 7a"* — the only sighting anywhere in the tree of the 1040 renumbering line 7. Added
a new bullet to `ROADMAP_STATUS.md` §4 (after the existing archival-status list, before "Nothing is
transcribed from a draft") recording this as one indirect witness and naming the action (archive the
1040 draft/final and read it directly) rather than resolving it from the citing form.

## FR-166 — `check_return.py:307`, yearless projection silently scored as TY2024

Refactored the M-2 year-resolution logic out of `main()` into `_resolve_year(cli_year,
wrapper_year)`, which now `cannot_run()`s (exit 2, naming `btctax income project --year`) when
BOTH are `None`, instead of defaulting to 2024. The pre-existing "`--year` contradicts tax_year"
refusal is preserved verbatim. Added a `selftest()` kill (case 10): `_resolve_year(None, None)`
raises `SystemExit(2)`; `_resolve_year(None, 2025)`, `(2025, None)`, `(2025, 2025)` all resolve to
2025; `(2025, 2024)` still refuses the contradiction case.

B1, pasted from an actual CLI run (`.venv/bin/python scripts/oracle/check_return.py`, stdin
`{"filing_status": "Single"}`):
- **No year at all** → refused: `"the projection carries no tax_year and --year was not given, so
  there is no year to drive OpenTaxSolver, Tax-Calculator or btctax's own harness on — silently
  assuming one used to fabricate a divergence (FR-166). Re-run `btctax income project --year <N>`
  ..."`, exit code **2**.
- **`--year 2025` given** → the year check no longer fires; the run proceeds to the next real gate
  (`"the §9 harness is not built ... cargo build -p btctax-oracle-harness"`, exit 2 for an
  unrelated, pre-existing reason — the harness binary isn't built in this worktree). Confirms the
  fix's refusal is specific to the missing-year case, not a blanket refusal.

`--selftest` passes end-to-end after the change.

## FR-167 — `verify_schedule_1a.py:85`, `_rows(pol, name, year=2025)`

Measured reliance first, as instructed: `grep -n "_rows("` found 4 call sites — 3 inside
`_parameter_census` (lines 141, 148, 155) omitted `year` and relied on the default; 1 inside
`_tc_value` (line 541, the applied census) already passed `year` explicitly. Checked for external
callers: `_rows` is module-private (leading underscore); `taxcalc_exact.py` imports the module
(`import verify_schedule_1a as v`) but calls only `v._policy`, `v._vectors`, `v._discriminating`,
`v._pins_the_arithmetic`, `v._taxcalc_predicted`, `v._taxcalc_applied` — never `v._rows`. So all
reliance on the default was internal and localized to `_parameter_census`, which is TY2025-only by
its own banner ("Schedule 1-A (TY2025) per-part witness census" — Schedule 1-A did not exist before
TY2025).

Fix: `year` is now a required positional parameter (no default); `_parameter_census` sets a local
`year = 2025` (matching its own explicit, permanent scope) and passes it to all three call sites.

B1:
- **No year**: `v._rows(pol, 'TipIncomeDed_c')` → `TypeError: _rows() missing 1 required
  positional argument: 'year'`.
- **With year**: `v._rows(pol, 'TipIncomeDed_c', 2025)` → `[{'year': 2025, 'value': 25000.0}]`.

Ran the full script after the fix (`OTS_DIR` unset, so one-oracle mode as documented):
`OK: 0 unexpected divergence(s) across both censuses.`, exit 0. The parameter census's own printed
values (e.g. `TipIncomeDed_c base 25,000.0000 vs ours 25,000.0000 OK`) are unchanged from before the
fix — confirms the refactor is behavior-preserving for the one year it's ever called with.

## Build gate

No Rust code was touched (all four owned files are Markdown or Python), so `make check` / `cargo
fmt --all --check` do not apply to this parcel's diff. `cargo run -p xtask -- form-delta` was
compiled and run once for FR-212's citation (`CARGO_TARGET_DIR=<worktree>/target-h`, foreground,
~2m29s cold build, exit 0) — read-only use of existing code, no xtask changes made. Both Python
files were syntax-checked (`ast.parse`) and their respective self-tests / full runs executed in the
foreground with `.venv/bin/python` (bare `python3` lacks taxcalc/pandas per project convention).

## Scope discipline

Touched exactly the four owned files (`git status --short` confirms). No edits to `return_refuse.rs`,
`witness_text`, `f1040s1.map.toml`/`f1040s3.map.toml`, or any file another wave-2 parcel owns.
