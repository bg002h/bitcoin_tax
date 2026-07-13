# CONTINUITY — full-return expansion, Phase 6 (PDF fillers)

**Read this first to resume.** Supersedes `CONTINUITY_P4.md` (Phase 4 is closed).

## Where we are

| Phase | State |
|---|---|
| P0–P4 | **CERTIFIED GREEN** (incl. P4.9 carryover write-back) |
| P5 (LIMITATIONS + advisories) | **CERTIFIED GREEN** — Fable r2 `0C/0I` at `b40bdec` |
| **P6 (PDF fillers)** | **IN PROGRESS — 5 of 9 forms fill** |
| P7 (golden returns) | not started |

Branch `full-return`. Gates at HEAD: **1599 passing / 0 failed**, clippy
(`--workspace --all-targets --locked -D warnings`) 0, fmt clean, xtask docs 5/5, FROZEN files 0 bytes.

## The operating contract (unchanged)

- opus implements test-first; **Fable reviews to green** (0 Critical / 0 Important). Persist each
  review verbatim *before* folding; re-review after every fold, including the last.
- **Gates are hard.** Do not start the next phase while a gate is open. (This was violated once in
  P5 — P6 was begun in the shared tree while the P5 gate was live, which contaminated the reviewer's
  measurements. Don't repeat it.)
- **FROZEN files** — `tax/{types,compute,se}.rs`. Never edit. Verify:
  `git diff 059ec2a..HEAD -- crates/btctax-core/src/tax/{types,compute,se}.rs` = 0 bytes.
- Use **CI's exact commands**, not per-crate shortcuts:
  `cargo test --workspace --locked` · `cargo clippy --workspace --all-targets --locked -- -D warnings`
  · `cargo fmt --all -- --check` · `cargo test -p xtask` (docs staleness).

## ★ The two things that will bite you

### 1. The printed-line chain (SPEC §3.1) — this shapes ALL remaining work

Printed form lines are **`round_dollar`ed AT THE LINE**, and a printed **total sums the
already-rounded lines above it**, so every filed form cross-foots. This is deliberately **NOT**
`round_dollar(exact_total)` — with two `.50` components the two differ by a dollar
(SPEC §10 KAT-9, locked by `kat9_printed_lines_round_then_cross_foot`).

**`btctax-forms` does ZERO tax arithmetic.** Each form's chain is derived in **core** and the filler
transcribes it cell-for-cell. The patterns to copy:

- `tax/other_taxes.rs` → `Form8959Lines` / `form_8959_lines`, `Form8960Lines` / `form_8960_lines`
- `tax/qbi.rs` → `Form8995Lines` / `form_8995_lines`
- `tax/printed.rs` → `Schedule2Lines` / `Schedule3Lines` (the module for the numbered schedules)

**The chains COMPOSE on the printed lines.** Schedule 2 line 11 is Form 8959's **printed** line 18 —
not a re-rounding of the exact figure. Otherwise a schedule disagrees with its own attachment by a
dollar and the return does not tie out. Locked by
`schedule_2_line11_takes_the_printed_8959_line_18_not_the_rounded_total`.

**Known, intended consequence:** the whole-dollar PDF can differ from the exact-cents report by a few
dollars. Filed under `p5-report-vs-pdf-may-differ-by-rounding → P6` in FOLLOWUPS — P6 must decide how
the report surfaces this, and LIMITATIONS.md must say it.

### 2. Field maps are NEVER guessable — dump them

`cargo run -p xtask -- dump-fields <pdf>` lists a PDF's AcroForm FQNs in reading order with geometry
(built for exactly this). Correlate every line against `pdftotext -layout <pdf>` output — the
`f1_N = line + k` offset breaks on every form with lettered sub-lines. **Never extrapolate a suffix.**

The geometric read-back (`verify_flat`) bands each widget's **x-CENTER** against a code-side column
cluster and checks **ordinal-y descent** per page. It is the map-independent oracle: a mis-mapped cell
FAILS CLOSED. Fault-inject both legs (cross-column and same-column swap) in every form's KATs.

## Done (commits)

| Form | Core chain | Map | Filler | Commit |
|---|---|---|---|---|
| Form 8959 | `other_taxes::form_8959_lines` | `f8959.map.toml` | `form8959.rs` | `51020d8` |
| Form 8960 | `other_taxes::form_8960_lines` | `f8960.map.toml` | `form8960.rs` | `683e83b` |
| Form 8995 | `qbi::form_8995_lines` | `f8995.map.toml` | `form8995.rs` | `683e83b` |
| Schedule 2 | `printed::schedule_2_lines` | `f1040s2.map.toml` | `schedule23.rs` | `010af35` |
| Schedule 3 | `printed::schedule_3_lines` | `f1040s3.map.toml` | `schedule23.rs` | `010af35` |

All 9 official TY2024 IRS PDFs are already bundled in `crates/btctax-forms/forms/2024/`.

## Remaining P6 work, in the order I'd do it

1. **Schedule A** — the next step, and it needs a small refactor first:
   `return_1040::schedule_a_deduction` returns only a TOTAL; the printed chain needs the lines.
   The pieces already exist — `charitable::CharitableResult` **already carries Schedule A lines
   11/12/13/14 exactly** (`allowed_cash` / `allowed_noncash` / `allowed_carryover` / `allowed`).
   Plan: add a `ScheduleAParts` struct (medical L1, AGI L2, SALT 5a/5b/5c/5d/5e + the cap, mortgage
   8a, the four charitable lines, total L17), expose it on `AbsoluteReturn`, have
   `schedule_a_deduction` sum it. **This also closes `p5-m1`** (the report can then print interior
   lines). Map hazards already dumped: line 8b is `f1_19`, **not** `f1_17` (the only non-sequential
   suffix); line 2 (`f1_4`) sits in its OWN x-cluster at [331.2, 402.5] — neither MID nor AMOUNT, so
   the oracle needs a third column; line 8d (`f1_21`) is a ReadOnly "Reserved" widget — never write it.
2. **Schedule 1** — income (L1 state refund, L3 Schedule C, L8v crypto ordinary) + adjustments
   (L15 half-SE, L18 early-withdrawal, L21 student-loan). Needs the same components-exposure treatment.
   Map hazards: line 22 (`f2_14`) is a ReadOnly "Reserved" widget that CONSUMES a suffix number; lines
   8a/8d/8s are `( )` boxes (positive magnitudes); `f1_06`/`f2_10`/`f2_11` are date/SSN-comb fields
   sitting in the money x-band — writing money there prints garbage.
3. **Schedule B** — repeating payer tables (Part I = **14** rows, Part II = **15** — the asymmetry is
   real). Row 1 of BOTH parts has a different parent subform (`Line1_ReadOrder` / `ReadOrderControl`),
   so generating row FQNs by string interpolation produces two wrong names. Its amount column is
   **[489.6, 576]**, NOT the [504, 576] every other form uses. Part III lines 7a/8 are Yes/No pairs
   with identical on-states (`"1"`/`"2"`) — only y-geometry + name disambiguate; **7b is FREE TEXT,
   not a Yes/No pair**.
4. **Schedule C** — crypto business income/expenses.
5. **Schedule D lines 17–22** — extend `schedule_d.rs` (all four §7.2 routing paths; KAT-10; the
   negative-cell read-back the oracle has never verified).
6. **The full 1040** — extend `form1040.rs` from the capital-gains cluster to every line.
7. **Wire it together**: `export_irs_pdf` emits the full packet; add the always-on **DRAFT/attest
   gate** for full-return PDFs (`p5-i1`); and **DELETE the P5-C1 refusal**
   (`CliError::CryptoSliceExportForFullReturnYear` + its guard in `cmd/admin.rs` + the KAT
   `export_refuses_for_a_full_return_year_p5_c1`) — that refusal exists ONLY because the crypto-slice
   export would file an understated Schedule D, which is precisely what this phase fixes.
8. **Fable P6 gate review → fold → re-review to 0C/0I.**
   ⚠️ The reviewer noted P5's green was measured at a HEAD that already contained the parked P6 commit
   `51020d8`. That code is inert at the user surface, so the green covers it — **but P6's own gate must
   NOT treat `51020d8` as already reviewed.**

Then **P7**: end-to-end golden returns (synthetic-household matrix, independent-oracle diff, IRS ATS
Scenario 2 partial-line diff).

## Open follow-ups owned by P6

See `FOLLOWUPS.md` — `p5-c1` (replace the refusal with the real export), `p5-i1` (always-on DRAFT
gate), `p5-m1` (report's interior schedule lines), `p6-printed-line-chain`,
`p5-report-vs-pdf-may-differ-by-rounding`, `p5-n5-advisory-line-wrapping`, `p1-ssn-normalization`.
