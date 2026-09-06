# Year-seam inventory — every year-dependent lookup, and what an unprepared year gets

**Headline:** The forms layer refuses an unprepared year almost everywhere (34 of 34 `*_pdf`/`for_year`
sites return `Err(UnsupportedYear)`), so the danger is not silent substitution at the lookup — it is that
**"adding a year" is 4 code edits per form and 0 data edits, which is why 20 of the 32 TY2025 assets
already committed to `crates/btctax-forms/forms/2025/` are unreachable from any code path.** Three live
class-(c) sites remain, and the instrument built to read the TY2026 drafts (`xtask label-proof`) fails on
**0 of 16** of them.

Scope: `crates/*/src/`, non-test code (everything before each file's first `#[cfg(test)]`). Method: a
year-literal scan (541 non-test hits), a wildcard-arm scan over every `match … year`, and a
silent-substitution scan (`unwrap_or*`, `.or_else`, `range(..=y).next_back()`).

---

## THE TABLE

Class **(a) REFUSES** — `None`, `Err`, or a panic naming the fix. **(b) ABSENT** — not a parameter, so
pinned to one year. **(c) FALLS BACK** — silently returns another year's data.

| symbol | file:line | class | what an unprepared year gets today |
|---|---|---|---|
| 17 × `*_pdf(year)` | `crates/btctax-forms/src/pdf.rs:73,83,93,103,111,119,128,135,143,151,159,167,175,183,191,205,213` | **a** | `Err(FormsError::UnsupportedYear(year))`. 11 of the 17 accept **2024 only**; 5 accept 2017/2024/2025; `f8275_pdf` aliases one Rev. 10-2024 asset to 2017\|2024\|2025 by design |
| 17 × `Map::for_year(year)` | `crates/btctax-forms/src/map.rs:186,380,714,880,1005,1149,1222,1311,1449,1525,1595,1652,1747,1828,1914,2003,2077` | **a** | `Err(UnsupportedYear)`. 12 of 17 accept **2024 only** (Form 6251, 8959, 8960, 8995, 8995-A, Sch 1/2/3/A/B/C) |
| `SUPPORTED_YEARS` | `crates/btctax-forms/src/lib.rs:68` = `&[2017, 2024, 2025]` | **a** | coarse gate — true for 2025 even though 12 of 17 maps refuse 2025. Used as a pre-write gate at `crates/btctax-cli/src/cmd/admin.rs:678` |
| `f1040_clusters(year)` | `crates/btctax-forms/src/form1040.rs:34` | **a** | `panic!("no geometry recorded for TY{other} …")` — fixed today |
| `se_clusters(year)` | `crates/btctax-forms/src/schedule_se.rs:37` | **a** | `panic!("no geometry recorded for TY{other} …")` — fixed today |
| **`sec_clusters(year, section)`** | **`crates/btctax-forms/src/form8283.rs:70-77`** | **c (latent)** | **`(_, Section::A) => SEC_A_CLUSTERS_2023`.** The third geometry wildcard; the two above were enumerated today, this one was not |
| `BundledTaxTables::table_for` | `crates/btctax-adapters/src/tax_tables.rs:84` | **a** | `None`. Bundled: **2017, 2024, 2025, 2026** (`load()`, l.75-78) |
| `BundledFullReturnTables::full_return_for` | `crates/btctax-adapters/src/tax_tables.rs:107` | **a** | `None`. Bundled: **2024 only** (l.101) |
| `compute_tax_year` table gate | `crates/btctax-core/src/tax/compute.rs:262` | **a** | `TaxOutcome::NotComputable(BlockerKind::TaxTableMissing)` |
| full-return gates | `cmd/tax.rs:475,512,819`; `cmd/admin.rs:965`; `session.rs:526,569`; `resolve.rs:264` (all `crates/btctax-cli/src/`) | **a** | `(Some(params), Some(table)) else` → `CliError::Usage`, **no bytes written** |
| `schedule_1a_params(year)` | `crates/btctax-core/src/tax/tables.rs:1088` | **a** | `None` outside `2025..=2028` (statutory sunset) → `Schedule1A::compute` returns `None` (`schedule_1a.rs:1319`) |
| gift advisory | `crates/btctax-cli/src/render.rs:2103` | **a** | explicit *"gift annual-exclusion table unavailable for {year}"* note, never a substituted table |
| prior-year carryover authority | `crates/btctax-cli/src/cmd/tax.rs:658-666` | **a** | triple-`Some` match; a missing/refused prior year falls back to the **flat §1211 rule**, documented in place |
| `row_to_inputs` / `set` | `crates/btctax-cli/src/return_inputs.rs:75, 104` | **a** | the row key stamps `tax_year` on read; a write whose `tax_year` disagrees with the key is **refused** |
| **`DEFAULT_ROW_YEAR`** | **`crates/btctax-core/src/tax/line_coverage.rs:55`** = `"2024"` | **b + c** | 24 `cover_*` fns, **0 take a year**; 23 sit at the default and claim the TY2024 extract |
| `capital_loss_carryover` consts | `crates/btctax-core/src/tax/capital_loss_carryover.rs:74-169` | **b** | 8 `pub const &'static str` hardcoding "2024"/"2025"; `SOURCE_EXTRACT` pinned to `i1040sd--2025.txt` |
| `TRANSITION_DATE`, `TY2025_RETURN_DUE` | `crates/btctax-core/src/conventions.rs:17,19` | **b** | one date each, no year parameter |
| `EraPreset::Y2025Onward` | `crates/btctax-core/src/defensive/era.rs:112` | **b** | window ends `2025-12-31`; **has a tripwire** (`crates/btctax-forms/tests/census.rs:418`) that reds when `SUPPORTED_YEARS` gains 2026 |
| supported-set refusal strings | `cmd/tax.rs:822,896`; `cmd/admin.rs:970`; `btctax-forms/src/form1040_full.rs:57`; `schedule_d_full.rs:77` | **b** | 5 messages name **"TY2024"** as a literal; only `full_return_for` derives it |
| `prompt_check::FORMS.extract` | `crates/xtask/src/prompt_check.rs:56,62,68,74,80,86,92,99` | **b** | 8 pins to `i1040gi--2025.txt` / `i8615--2025.txt` |
| **`label_reader::proof` year** | **`crates/xtask/src/label_reader.rs:456`** | **c (live)** | `rsplit("--").next().unwrap_or("2025")` with **no `.trim_end_matches("-DRAFT")`** → wrong directory for every draft |
| `form_geometry::extract` year | `crates/xtask/src/form_geometry.rs:191-195` | **c (dead)** | same `unwrap_or("2025")`; `rsplit` always yields `Some`, so the literal is unreachable — but it *does* strip `-DRAFT` |
| `FormAuthority.year` | `crates/xtask/src/cite_check.rs:654, 667` | **a (by design)** | the **only** place in the workspace where the year is a struct FIELD rather than a match arm. 1 entry of 16 emitted forms |

Checked and **clean** (no fallback found): `btctax-store/src/` has no year concept at all; `synthetic_table`
(`tables.rs:583`) is `#[cfg(test)]`; `whatif.rs:318` and `optimize.rs:1427` degrade to `None`; every
indexed dollar figure outside `tax_tables.rs` is statutory (NIIT §1411 thresholds `tables.rs:180-182`,
§1211 limit `l.242`, §170 appraisal thresholds `l.190/215`, §221(b)(1) $2,500 cap `return_1040.rs:1279`,
Sch B $1,500 `return_1040.rs:3479`) — a `dec!(≥1000)` scan over all non-test source found no indexed
value living outside a per-year table.

---

## FINDINGS

### F1 (Critical for the target) — the TY2025 data landed and nothing happened: 20 of 32 assets unreachable

```
== TY2025 ==
  UNREFERENCED: f1040s1a.map.toml   f1040s1a.pdf
  UNREFERENCED: f1040s2.map.toml    f1040s2.pdf
  UNREFERENCED: f1040s3.map.toml    f1040s3.pdf
  UNREFERENCED: f1040sa.map.toml    f1040sa.pdf
  UNREFERENCED: f1040sb.map.toml    f1040sb.pdf
  UNREFERENCED: f1040sc.map.toml    f1040sc.pdf
  UNREFERENCED: f6251.map.toml      f6251.pdf
  UNREFERENCED: f8959.map.toml      f8959.pdf
  UNREFERENCED: f8960.map.toml      f8960.pdf
  UNREFERENCED: f8995.map.toml      f8995.pdf
```
(TY2017 and TY2024: zero unreferenced.) Ten TY2025 forms are archived, mapped, committed — and no code
path can reach them, because reaching them needs `include_bytes!`/`include_str!` consts and match arms
that were never written.

`map.rs:3-4` states the opposite as fact:

> *"The maps are DATA (TOML committed next to the bundled PDFs), not code — 'adding a year' is a
> `forms/<year>/` directory (PDF + maps), **never a code change**."*

**Measured cost of adding TY2026 for the 16 emitted forms**, on today's structure: 16 `include_bytes!`
consts + 16 `include_str!` consts + 16 arms across the 17 `*_pdf` fns + 16 arms across the 17 `for_year`
fns + 1 `SUPPORTED_YEARS` edit + 3 geometry cluster fns ≈ **68 hand edits**, every one of which is the
same edit, and any one of which can be forgotten silently — which is exactly what TY2025 demonstrates.
This single fact is the strongest argument for the year-package lens: the refusals are excellent, and they
are refusing because nobody typed the 68 lines.

### F2 (Important) — `xtask label-proof` cannot open a single TY2026 draft, and misdiagnoses why

```
$ ./target/debug/xtask label-proof f6251--2026-DRAFT --out …/p.pdf
xtask label-proof: /scratch/code/bitcoin_tax/design/forms/2026-DRAFT/f6251--2026-DRAFT.pdf not present
  (gitignored; re-fetch from its .pdf.txt note): No such file or directory (os error 2)

$ ./target/debug/xtask label-proof f6251--2025 --out …/p25.pdf
label-proof: wrote --out
  62 boxes filled with their assigned label; 2 printed `?` (BOX-WITH-NO-LABEL)

$ for f in design/forms/2026/*-DRAFT.pdf; do … done
2026 drafts: label-proof OK=0 FAILED=16
```

The file is present, at `design/forms/2026/f6251--2026-DRAFT.pdf`. The two sibling year derivations
differ by one call:

```rust
// crates/xtask/src/form_geometry.rs:191-195   (correct)
let year = stem.rsplit("--").next().unwrap_or("2025").trim_end_matches("-DRAFT");

// crates/xtask/src/label_reader.rs:456        (defective)
let year = stem.rsplit("--").next().unwrap_or("2025");
```

Severity is about *which* instrument: `label-proof` is the human-in-the-loop geometry check — its own
success banner reads *"★ Open it. Every box should show the line it belongs to… a defect the machines
could not see."* It is the one instrument that exists to catch what `label-census` cannot, `label-census`
works on drafts, and so the gap is invisible in a green run. And the error text sends the operator to
re-fetch a PDF that is already on disk. B1's question — *"which test reds when this checker is
removed?"* — has no answer here: nothing exercises `proof` on a `-DRAFT` stem.

`unwrap_or("2025")` is dead in **both** files: `"f1040".rsplit("--").next()` is `Some("f1040")`, never
`None`. It reads as a deliberate fallback-to-2025 and is not one, in the two places most likely to be
copied when a new stem-consuming command is added.

### F3 (Important) — the third geometry wildcard survived today's fix, and goes live the moment TY2026 is added

```rust
// crates/btctax-forms/src/form8283.rs:70-77
fn sec_clusters(year: i32, section: Form8283Section) -> &'static [(f32, f32)] {
    match (year, section) {
        (2017, Form8283Section::A) => SEC_A_CLUSTERS_2017,
        (2017, Form8283Section::B) => SEC_B_CLUSTERS_2017,
        (_, Form8283Section::A) => SEC_A_CLUSTERS_2023,   // ← any other year
        (_, Form8283Section::B) => SEC_B_CLUSTERS_2023,   // ← any other year
    }
}
```

Reached at `form8283.rs:560` as `sec_clusters(map.year, section)`, and the clusters are the oracle that
`verify_flat` reads back against — so a wrong cluster set does not fail the read-back, it *redefines*
it. Today it is unreachable: `Form8283Map::for_year` (`map.rs:880`) and `f8283_pdf` (`pdf.rs:191`) refuse
2026 first. Its refusal is **borrowed from a different function**, which is precisely the property that
made `f1040_clusters` and `se_clusters` safe right up until they were not. Adding `2026 => …` to those
two enumerations — the first thing a TY2026 year package does — silently hands the Rev. 12-2026 form the
**Rev. 12-2023** column geometry. The doc at `form8283.rs:26` already says the clusters are *"per form
revision"*.

### F4 (Important) — the line-provenance census is pinned to one year with no API to hold two

`line_coverage.rs` is 24 `cover_*` functions carrying the transcribed instruction sentence for every
printed line. **Measured: 24 functions, 0 with a year parameter, 1 call to `quoting_year`** (`l.2993`,
`cover_schedule1a`, added today). Everything else quotes `DEFAULT_ROW_YEAR = "2024"` (`l.55`), and
`xtask/src/line_coverage_check.rs:1382,1399` resolves the extract from that same constant.

The type carries a year per *table instance* (`Coverage.1`), not per *tax year requested*, so there is no
shape in which TY2024 and TY2026 provenance coexist — a TY2026 table means either forking all 24
functions or threading a year through all 24 signatures. And the failure mode when a new form's author
forgets `quoting_year` is class (c): the row claims a TY2024 quotation and the checker validates against
`--2024.txt`. The constant's own doc records that this already happened once ("*until 2026-09-05 there was
**no API** by which it could*").

### F5 (Minor) — the supported-year set is stated five times and derived once

`cmd/tax.rs:822` *"needs a supported tax year (TY2024)"*, `cmd/tax.rs:896` *"supports full returns for
TY2024 only"*, `cmd/admin.rs:970` *"(TY2024)"*, `form1040_full.rs:57` and `schedule_d_full.rs:77` *"v1 is
TY2024-only."* None reads `BundledFullReturnTables`. This is the same shape as the six stale refusal
descriptions folded in `3d01b5e3` — the refusal fires correctly and then names the wrong year.

### F6 (Minor) — the filer is shown "2024"/"2025" in worksheet quotations that cannot move

`capital_loss_carryover.rs:74-169` holds the whole Capital Loss Carryover Worksheet as `pub const`
strings — *"Enter the amount from your **2024** Form 1040 … line 15"*, *"Short-term capital loss carryover
for **2025**"*, *"filing separate returns for **2025**"* — and `questions.rs:733,762` quotes them verbatim
into two filer-facing prompts. For TY2026 every one reads a year off by one. The worksheet is genuinely
year-relative ("from prior to current"), so the fix is a two-year format, not a second copy.

### F7 (observation, not a defect) — two uncoupled year predicates for overlapping subjects

`questions.rs:537` is `live: |ri| ri.tax_year >= 2025` (open-ended); `schedule_1a_params` is
`2025..=2028` (`tables.rs:1089`). After the Schedule 1-A sunset the question stays live with no schedule
behind it. This is defensible — `HasIncomeExclusion` also feeds the §164(b) SALT phase-down, which does
not sunset with Schedule 1-A — but the two ranges are written independently with nothing coupling them,
and the second consumer is the only reason the wider one is right.

### F8 (positive) — the pattern the scaffolding needs already exists, at 1/16 coverage

`crates/xtask/src/cite_check.rs:651-672`:

```rust
pub struct FormAuthority { pub form: &'static str, pub year: i32,
                           pub instructions: &'static str,
                           pub instr_pages: Option<(u32,u32)>,
                           pub extract_stem: &'static str }
pub const FORMS: &[FormAuthority] = &[FormAuthority { form: "f1040s1a", year: 2025, … }];
```

This is the only year-as-a-field table in the workspace, it already drives path construction
(`cite_check.rs:500,521`), and it already carries an honest ratchet (`AUTHORITY_NOT_YET_ARCHIVED`, 15 of
16 forms). `pdf.rs` + `map.rs` are the same registry written 34 times as match arms. `defensive/era.rs`
shows the other half working: a literal that cannot be year-parameterised, held by a test
(`census.rs:418`) that reds when `SUPPORTED_YEARS` moves. Between them the constellation already contains
both halves of the answer — a registry keyed by `(form, year)`, plus a tripwire for each literal that
genuinely cannot be.

---

## MECHANICAL (a machine can do it)

1. **Add `.trim_end_matches("-DRAFT")` at `label_reader.rs:456`**, with a B1 negative test that runs
   `proof` on a `-DRAFT` stem and reds today (0/16 → 16/16 is the measurement).
2. **Delete both `unwrap_or("2025")`** (`label_reader.rs:456`, `form_geometry.rs:194`) — `rsplit` cannot
   yield `None`, so they are unreachable literals reading as policy. Factor the one correct derivation
   into a shared `year_dir_of(stem)` so a third copy cannot diverge.
3. **Enumerate `sec_clusters`** (`form8283.rs:70`) exactly as `f1040_clusters`/`se_clusters` were today:
   `2017 => …, 2024 | 2025 => …, other => panic!(…)`. One `git grep '_ =>'` over every
   `fn *(year: i32) -> &'static` closes the class.
4. **A test that every bundled asset is reachable.** Walk `crates/btctax-forms/forms/*/`, assert each file
   appears in an `include_bytes!`/`include_str!`. It reds today with the 20 TY2025 files above, and it is
   the check that would have made F1 visible when the data landed.
5. **A per-(form, year) support census.** `SUPPORTED_YEARS` says 3 years; 12 of 17 maps accept only 2024.
   Emit the 17×N matrix from the `for_year` arms so "what does TY2026 still need" is a command.
6. **Derive the five "(TY2024)" strings** from `BundledFullReturnTables`, then a test asserting no
   user-facing string in `crates/*/src/` contains a `TY20\d\d` literal that is not read from a table.
7. **Thread a year through the 24 `cover_*` signatures** (or make `Coverage` year-keyed) so the
   compiler — not a reviewer — finds the 23 call sites when TY2026 provenance is added.
8. **Two-year-format the `capital_loss_carryover` consts** as `fn(prior: i32, current: i32) -> String`,
   with the existing `xtask capital-loss-carryover-check` re-pointed at the year's own extract.

## HUMAN (someone must read a form)

1. **Does the Rev. 12-2026 Form 8283 keep the Rev. 12-2023 column x-positions?** F3 is only a *latent*
   defect if the answer is yes; if the columns moved, the wildcard is a live wrong-column write the
   read-back cannot see. Requires opening the draft and comparing against `SEC_{A,B}_CLUSTERS_2023`.
2. **Which of the 10 unreachable TY2025 maps are still correct?** They were built against the TY2025
   forms; before wiring them up, someone must confirm each map's field names still match the archived
   PDF — the maps have never been executed.
3. **The TY2026 line-binding moves.** `TY2026_WORK_LIST.md` records 31 (1040) and 28 (6251) moved printed
   line bindings with no field renames — no machine can tell a *moved* binding from a *kept* one without
   a person reading both revisions, and F2 currently blocks the instrument built for that reading.
4. **Does the Capital Loss Carryover Worksheet's wording change for TY2026?** F6's fix presumes only the
   years move. That is a transcription question against `i1040sd--2026`, not an inference.
5. **The `EraPreset::Y2025Onward` extension.** The tripwire will red; a person must decide whether the
   new bucket is `Y2025To2026` or a re-cut, since the pooling cutover gives 2025 its own meaning.
