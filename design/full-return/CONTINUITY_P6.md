# CONTINUITY — full-return expansion, Phase 6 (PDF fillers)

**Read this first to resume.** Supersedes `CONTINUITY_P4.md` (Phase 4 is closed).

## Where we are

| Phase | State |
|---|---|
| P0–P4 | **CERTIFIED GREEN** (incl. P4.9 carryover write-back) |
| P5 (LIMITATIONS + advisories) | **CERTIFIED GREEN** — Fable r2 `0C/0I` at `b40bdec` |
| **P6 (PDF fillers)** | **P6.1–P6.5 DONE — the PACKET FILES. `export-irs-pdf` emits a complete, filable full-return packet; the P5-C1 refusal is DELETED. Only the P6.6 GATE REVIEW remains (Fable, to 0C/0I).** |
| P7 (golden returns) | not started |

Branch `full-return`. Gates at HEAD: **1676 passing / 0 failed**, clippy
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

## Remaining P6 work — PACKET ASSEMBLY ONLY

**Every form fills.** All nine new forms, the full-return Schedule D, and the full 1040, each with its
core printed chain, its verified map, its geometric read-back, and fault-injection KATs. What remains
is wiring them into a filable packet.

### ★ The order below is the ARCHITECT'S, and it supersedes the earlier identity-first sketch

`reviews/ARCH-P6-fable-packet-assembly.md` (Fable, persisted verbatim at `6d40159`) re-ordered the
phase and found a **gating defect we had missed**. Both SPEC amendments it called for have LANDED
(`fdaa86b`). The live order — do them in this order, the numbering is load-bearing:

| Step | Work |
|---|---|
| **P6.1** | **core** `PrintedReturn` + `assemble_printed_return` + `ReturnHeader` + `Ssn::canonical` + `screen_inputs` refuse + tie-out KATs · *hoist `Form8959Lines::must_file()` to core while here* |
| **P6.2** | identity map fragments + shared writer; the 1040 header block **including the §63(f) aged/blind checkboxes** |
| **P6.3** | `fill_full_return` (all-or-nothing) + dispatch in `export_irs_pdf` + report renders the PRINTED figures + `p5-n5` wrapping + LIMITATIONS |
| **P6.5** | delete `CryptoSliceExportForFullReturnYear` + its KAT; replace with the two dispatch KATs |
| **P6.6** | Fable P6 gate review → fold → re-review to 0C/0I |

**There is no P6.4.** The always-on DRAFT/attest gate (`p5-i1`) is **CLOSED by the SPEC §9 amendment**,
not by code: the user decided the full-return packet exports **clean and filable with no attestation**;
DRAFT/attest stays pseudo-only. Do not build a gate. Likewise `p6-schedule-b-overflow-…` is closed —
the fail-closed refusal is now SPEC'd behavior (§7.4 as amended), not a deviation to declare.

**Start with core, NOT the identity fill** — the identity fill *consumes* `ReturnHeader`, so doing maps
first means inventing the header's data shape twice.

### ★★ GATING, found by the architect: the §63(f) aged/blind checkboxes

Core folds the age-65/blind additions into printed 1040 **L12**, but `f1040.map.toml` has **NO
age/blind checkboxes**. A filed 1040 claiming a nonstandard standard deduction with **zero** boxes
checked fails the IRS's own arithmetic cross-check — the checkbox count is how the Service validates
it. Same class as P5-C1: a form internally inconsistent with itself. **Same tier as name/SSN.**

So the phase's exit condition is restated: the packet is filable **AND every figure on it is internally
and mutually consistent** (cross-foots, ties to its attachments, checkbox-consistent with L12) — "every
money line is right" was quietly narrower than "the return is right".

### 1. The identity header (`p6-form-identity-header`) — P6.2, and it is bigger than name+SSN

**No filler writes the taxpayer name or SSN.** The money lines are right, but the forms are **not
filable**: an unnamed Schedule C is not a return. Do NOT delete the P5-C1 refusal until this is done,
or the export would emit unnamed forms, which is worse than refusing.

Beyond the nine schedules' two fields each, the 1040 header must also carry the **aged/blind
checkboxes** (gating, above) and the **dependents rows** (data already in `Dependent`). Watch the
semantics: "Name(s) shown on return" is the **joint** name line on MFJ for the schedules, but Schedule C
wants the **proprietor only** with that person's SSN — a naive shared writer puts joint names on
Schedule C. That is why `ReturnHeader` is derived ONCE in core (P6.1) and the fillers only transcribe.

The nine schedules take two fields each (name, SSN); dump each with
`cargo run -p xtask -- dump-fields <pdf>` — they are the first two text fields on page 1 (e.g. Form
8959: `f1_1` / `f1_2`; Schedule B: `f1_01` / `f1_02` — note the zero-padding differs per form).

**The 1040's header is a full identity block, not two fields** (dumped, TY2024):

| field | FQN |
|---|---|
| taxpayer first name | `topmostSubform[0].Page1[0].f1_04[0]` |
| taxpayer last name | `topmostSubform[0].Page1[0].f1_05[0]` |
| taxpayer SSN | `topmostSubform[0].Page1[0].f1_06[0]` |
| spouse first name | `topmostSubform[0].Page1[0].f1_07[0]` |
| spouse last name | `topmostSubform[0].Page1[0].f1_08[0]` |
| spouse SSN | `topmostSubform[0].Page1[0].f1_09[0]` |
| address | `…Address_ReadOrder[0].f1_10[0]` … `f1_12[0]` |

Check the SSN cells for `/MaxLen` (11) and comb flags before writing — a formatted `123-45-6789` is
exactly 11 characters. Use `FlatPlacement::free` (geometry-exempt, still in the no-unmapped set), as
`form8283.rs` already does for its identity fields.

### 2. Packet assembly is a CORE function (P6.1), and the CLI keeps only I/O

Build every chain from one `AbsoluteReturn`, in dependency order — the upstream chains are arguments
to the downstream ones, which is what makes the packet tie out:

```
f8959 = other_taxes::form_8959_lines(...)     f8960 = other_taxes::form_8960_lines(...)
f8995 = qbi::form_8995_lines(...)             sch_d = printed::schedule_d_lines(&ar)
sch_a = printed::schedule_a_lines(&ar)        sch_b = printed::schedule_b_lines(&ri)
sch_c = printed::schedule_c_lines(&ar)        sch_1 = printed::schedule_1_lines(&ar)
sch_2 = printed::schedule_2_lines(&ar, &f8959, f8960.as_ref())
sch_3 = printed::schedule_3_lines(&ar)
f1040 = printed::form_1040_lines(&ar, sch_b, sch_1, sch_a, &sch_d, sch_2, sch_3, &f8959, f8995, ...)
```

Each `*_lines` returns `Option` when its form is not required — emit only the `Some` ones.

**That wiring lives in `assemble_printed_return` in CORE, not in the CLI** — "Schedule 2 L11 = the
printed 8959 L18" is tax semantics, exactly what `btctax-forms` is forbidden to know, and the CLI is
the one place the composition KATs cannot reach. `btctax-forms` gets `fill_full_return(&PrintedReturn)`,
which must be **all-or-nothing**: if any member filler refuses, ZERO bytes hit disk (a 1040 citing a
Schedule B that is not attached is a wrong return — partial emission is a fail-OPEN).

Three anti-drift mechanisms, all already in the house style: (1) re-point the existing composition KATs
*through* `assemble_printed_return` so the tested wiring is the shipped wiring, plus tie-out KATs on
`PrintedReturn` itself (`f1040.line23 == sch_2.line21`, `f1040.line8 == sch_1.line10`, …); (2)
`fill_full_return` destructures `PrintedReturn` with **no `..`** (the `p1-r3-m1` precedent), so a form
without a filler is a compile error; (3) a cross-PDF byte oracle — fill the packet, read the cell TEXT
back, assert 1040 L23's text equals Schedule 2 L21's text.

### 3. The report must print the PRINTED figures (P6.3)

The clinching case is L37: "amount you owe" is an instruction to write a check, and a tool that says
$12,345.67 in the terminal and $12,347 on the filed form has produced **two authoritative answers**.
So the absolute block of the report renders whole-dollar printed-chain figures identical to the PDF,
cell for cell; the crypto-DELTA block stays exact cents (it is not a filed figure). This collapses
`p5-m1` + `p5-report-vs-pdf-may-differ-by-rounding` into one piece of work.

### 4. ★ DELETE THE P5-C1 REFUSAL (P6.5) — the phase's exit condition

Remove `CliError::CryptoSliceExportForFullReturnYear`, its guard in `cmd/admin.rs::export_irs_pdf`,
and the KAT `export_refuses_for_a_full_return_year_p5_c1`. That refusal exists ONLY because the
crypto-slice export would file an understated Schedule D (no line 13, no lines 6/14).
`schedule_d_full.rs` now fills all three and all of Part III, so its reason is gone.

⚠️ **What the deletion costs, per the architect:** today the refusal is a *hard* guarantee that the
crypto slice can never run on a full-return year. Deleting it downgrades a type-level impossibility to
an `if` in `export_irs_pdf`. So: put the dispatch in **one** function (has `ReturnInputs` → full packet,
else → slice), pin it with KATs in **both** directions, and give the two packets **non-overlapping
filenames** (the slice writes `form_1040_capgains.pdf`; keep the full packet as `f1040.pdf`,
`f1040s1.pdf`, … + a manifest) so two runs' artifacts can never be collated into a chimera return.

**Keep the two Schedule D fillers separate** (the architect confirmed this call): the slice prints
exact CENTS and the full chain prints whole DOLLARS — they are under different rounding regimes, and a
future "harmonization" must never happen. A crypto-only filer may legitimately file in cents.

### 5. LIMITATIONS.md + the report

Update the "computed vs. filled" section — it currently says NO full-return PDF exists. Per §3 above,
choosing printed figures for the report means there is no user-visible rounding divergence left to
disclose: only a one-line footnote ("whole-dollar figures per the rounding election; internal
computation carries cents") plus the LIMITATIONS entry.

### 6. Fable P6 gate review → fold → re-review to 0C/0I

⚠️ P5's green was measured at a HEAD that already contained the parked P6 commit `51020d8`. That code
is inert at the user surface, so the green covers it — **but P6's own gate must NOT treat `51020d8`
as already reviewed.**

Nothing left to *declare* as a deviation: both former deviations (Sch B overflow, the DRAFT gate) are
now SPEC'd (`fdaa86b`), which is why the amendments landed BEFORE this review — the reviewer certifies
a spec we intend to keep rather than a declared exception.

Then **P7**: end-to-end golden returns (synthetic-household matrix, independent-oracle diff, IRS ATS
Scenario 2 partial-line diff). **Build three of its pieces DURING P6, while the maps are fresh:** a
line-keyed `extract_lines(bytes, &Map)` inverse transcriber (it is trivial today and it powers both the
Q2 cross-PDF tie-out KAT and P7's partial-line diff), the kitchen-sink household fixture (P6's packet
KAT needs one anyway — put it in core's `testonly`), and packet-level determinism + a manifest in IRS
**Attachment Sequence No.** order (which also hands the filer their stapling order).

## Open follow-ups owned by P6

See `FOLLOWUPS.md` — `p5-c1` (replace the refusal with the real export), `p5-m1` (report's interior
schedule lines), `p6-printed-line-chain`, `p5-report-vs-pdf-may-differ-by-rounding`,
`p5-n5-advisory-line-wrapping`, `p1-ssn-normalization`, **`p6-aged-blind-checkboxes-missing` (GATING)**,
`p6-form-identity-header`, `p6-schedule-b-capacity-error-variant` (nit → P6.3),
`p6-form8959-must-file-belongs-in-core` (minor → P6.1).

**CLOSED by amendment, not by code:** `p5-i1` (no always-on DRAFT gate — the packet exports clean) and
`p6-schedule-b-overflow-refuses-instead-of-paginating` (the refusal IS the spec now).
