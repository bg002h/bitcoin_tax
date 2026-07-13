# CONTINUITY — full-return expansion, Phase 6 (PDF fillers)

**Read this first to resume.** Supersedes `CONTINUITY_P4.md` (Phase 4 is closed).

## Where we are

| Phase | State |
|---|---|
| P0–P4 | **CERTIFIED GREEN** (incl. P4.9 carryover write-back) |
| P5 (LIMITATIONS + advisories) | **CERTIFIED GREEN** — Fable r2 `0C/0I` at `b40bdec` |
| **P6 (PDF fillers)** | **IN PROGRESS — all 9 forms + Schedule D full fill; ALL core printed chains done. The 1040 FILLER and packet assembly remain** |
| P7 (golden returns) | not started |

Branch `full-return`. Gates at HEAD: **1624 passing / 0 failed**, clippy
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
| Schedule A | `printed::schedule_a_lines` | `f1040sa.map.toml` | `schedule_a.rs` | `7a8a184` |
| Schedule 1 | `printed::schedule_1_lines` | `f1040s1.map.toml` | `schedule23.rs` | `50462bc` |
| Schedule C | `printed::schedule_c_lines` | `f1040sc.map.toml` | `schedule_c.rs` | `c099e5a` |
| Schedule B | `printed::schedule_b_lines` | `f1040sb.map.toml` | `schedule_b.rs` | `23e5e22` |
| Schedule D (full) | `printed::schedule_d_lines` | `schedule_d.map.toml` (extended) | `schedule_d_full.rs` | `778913e` |
| **Form 1040** | `printed::form_1040_lines` | — | **NOT YET WRITTEN** | `a440811` (chain only) |

All 9 official TY2024 IRS PDFs are already bundled in `crates/btctax-forms/forms/2024/`.

**The components pattern** (use it for Schedule C): a schedule's printed chain needs its LINES, but
`assemble_absolute` only kept totals. So each schedule gets a `*Parts` struct in `return_1040.rs`
(exact cents) exposed on `AbsoluteReturn`, and `printed::*_lines` rounds it at the line. Done for
`ScheduleAParts` and `Schedule1Parts`. Note `charitable::CharitableResult` already carried Schedule A
lines 11/12/13/14 exactly — check for an existing struct before adding a new one.

## Remaining P6 work, in the order I'd do it

**Every core printed chain is DONE** (`tax/printed.rs` + `other_taxes.rs` + `qbi.rs`). All nine
standalone forms fill, and so does the full-return Schedule D. What remains is the 1040 filler and
wiring the packet.

1. **The Form 1040 filler.** `printed::form_1040_lines` already exists and is KAT'd; only the PDF
   fill is missing. `form1040.rs` today writes just the capital-gain cluster + the digital-asset
   question, so this is an extension, not a rewrite. Three known hazards:
   - the **5-way filing-status checkbox group** — SPEC §7.4 says `verify.rs`'s oracle handles only
     Yes/No pairs today and must be extended for it;
   - **per-year map collisions** — SPEC §7.4: `f1_57` is L12 on the 2024 form and L1z on the 2025 one,
     and the filing-status on-states are re-assigned between years. The map is per-(form, year) for
     exactly this reason; do not share a constant;
   - **line 7 is signed with a LEADING MINUS** (SPEC §3.2), unlike Schedule D's paren boxes.
2. **Packet assembly** (`export_irs_pdf`):
   - write the **taxpayer name + SSN header** on every form — none of the nine fillers does yet, so
     the money lines are right but the forms are **not filable as-is** (`p6-form-identity-header`);
   - add the always-on **DRAFT/attest gate** for full-return PDFs (`p5-i1`);
   - **DELETE the P5-C1 refusal** — `CliError::CryptoSliceExportForFullReturnYear`, its guard in
     `cmd/admin.rs`, and the KAT `export_refuses_for_a_full_return_year_p5_c1`. That refusal exists
     ONLY because the crypto-slice export would file an understated Schedule D. `schedule_d_full.rs`
     now fills lines 6/13/14 and all of Part III, so the reason for it is gone. **This is the phase's
     exit condition.**
   - decide how the report surfaces the report-vs-PDF rounding difference
     (`p5-report-vs-pdf-may-differ-by-rounding`) and say it in LIMITATIONS.md.
3. **Fable P6 gate review → fold → re-review to 0C/0I.**
   ⚠️ P5's green was measured at a HEAD that already contained the parked P6 commit `51020d8`. That
   code is inert at the user surface, so the green covers it — **but P6's own gate must NOT treat
   `51020d8` as already reviewed.**

Then **P7**: end-to-end golden returns (synthetic-household matrix, independent-oracle diff, IRS ATS
Scenario 2 partial-line diff).

## Open follow-ups owned by P6

See `FOLLOWUPS.md` — `p5-c1` (replace the refusal with the real export), `p5-i1` (always-on DRAFT
gate), `p5-m1` (report's interior schedule lines), `p6-printed-line-chain`,
`p5-report-vs-pdf-may-differ-by-rounding`, `p5-n5-advisory-line-wrapping`, `p1-ssn-normalization`.
