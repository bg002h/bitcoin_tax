# CONTINUITY — full-return expansion, Phase 6 (PDF fillers)

**Read this first to resume.** Supersedes `CONTINUITY_P4.md` (Phase 4 is closed).

## Where we are

| Phase | State |
|---|---|
| P0–P4 | **CERTIFIED GREEN** (incl. P4.9 carryover write-back) |
| P5 (LIMITATIONS + advisories) | **CERTIFIED GREEN** — Fable r2 `0C/0I` at `b40bdec` |
| **P6 (PDF fillers)** | ✅ **CERTIFIED GREEN — the GATE IS CLOSED.** Fable r3 `0C / 0I` at `8a56158` (r1 0C/9I → fold → r2 0C/3I → fold → r3 0C/0I). `export-irs-pdf` emits a complete, filable full-return packet; the P5-C1 refusal is DELETED. |
| P7 (golden returns) | not started |

Branch `full-return`. Gates at HEAD: **1685 passing / 0 failed**, clippy
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
| **Form 1040 (full)** | `printed::form_1040_lines` | `f1040.map.toml` (extended) | `form1040_full.rs` | `0eb3138` |

All 9 official TY2024 IRS PDFs are already bundled in `crates/btctax-forms/forms/2024/`.

**The components pattern** (use it for Schedule C): a schedule's printed chain needs its LINES, but
`assemble_absolute` only kept totals. So each schedule gets a `*Parts` struct in `return_1040.rs`
(exact cents) exposed on `AbsoluteReturn`, and `printed::*_lines` rounds it at the line. Done for
`ScheduleAParts` and `Schedule1Parts`. Note `charitable::CharitableResult` already carried Schedule A
lines 11/12/13/14 exactly — check for an existing struct before adding a new one.

## P6 is DONE — what shipped, and what the gate caught

`export-irs-pdf` now emits a **complete, filable full-return packet**: Form 1040 + Schedules 1/2/3/A/B/C/D/SE
+ Forms 8949/8959/8960/8995/8283, **all-or-nothing** (any refusal ⇒ zero bytes), in IRS **Attachment Sequence
No.** order with a `manifest.txt` (the filer's stapling order). The two pipelines write **disjoint filenames**
(the packet is sequence-prefixed: `00_f1040.pdf` … `12A_f8949.pdf`), pinned by a byte-level no-overwrite KAT.

**The three reviews are the record** (`reviews/IMPL-P6-fable-review-r{1,2,3}.md`, persisted verbatim before
each fold). What the gate caught, and why it was worth three rounds:

- **The extension payment was DROPPED from the filed return** — Schedule 3 had no line 10, so a filer who had
  already paid with their extension would be told, on the filed form, to pay it a second time. (Found by the
  architect's sweep, not by any test.)
- **The packet was INCOMPLETE**: SPEC §7 lists Form 8949, Schedule SE and Form 8283, and none was in
  `PrintedReturn` — while the existing fillers print CENTS, which would have put the filed Schedule D a dollar
  from the 8949 it CITES as its source.
- **A form contradicting its own arithmetic, three times**: the §63(f) aged/blind boxes, the dependent-claim
  box, the MFS-itemize box — each a captured input that reached the ARITHMETIC but never the FORM.
- **1040 L16 vs Table(L15)**: the Tax Table is a $50-tread STEP function, so computing the tax on the exact TI
  and rounding put the filed L16 a whole bin away from what anyone gets by looking up the filed L15.
- **A regression the first fold introduced**: rewiring Schedule A L2 to the printed L11 dropped the negative-AGI
  clamp, and the filed form deducted more medical than was paid.
- **Two of my own KATs were VACUOUS** and claimed guarantees they did not deliver. Fault-inject the
  load-bearing ones — a green suite is not evidence that a test bites.

## ★ The lesson to carry into P7

**A captured input that reaches the arithmetic but never the form is the recurring defect of this project.**
The systematic cure is the **form-citation audit**: walk every citation printed on the form itself ("Attach…",
"from Schedule X, line N", "Totals for all transactions reported on…") and confirm each is satisfied by a
packet member + a tie-out KAT, refused, or documented. That audit — Fable's own idea — is what caught the
attachment family; the closed list is in `reviews/IMPL-P6-fable-review-r1.md`. Re-run it whenever a form
changes.

## Open follow-ups owned by P6

See `FOLLOWUPS.md` — `p5-c1` (replace the refusal with the real export), `p5-m1` (report's interior
schedule lines), `p6-printed-line-chain`, `p5-report-vs-pdf-may-differ-by-rounding`,
`p5-n5-advisory-line-wrapping`, `p1-ssn-normalization`, **`p6-aged-blind-checkboxes-missing` (GATING)**,
`p6-form-identity-header`, `p6-schedule-b-capacity-error-variant` (nit → P6.3),
`p6-form8959-must-file-belongs-in-core` (minor → P6.1).

**CLOSED by amendment, not by code:** `p5-i1` (no always-on DRAFT gate — the packet exports clean) and
`p6-schedule-b-overflow-refuses-instead-of-paginating` (the refusal IS the spec now).
