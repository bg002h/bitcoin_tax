# The year-package table — design draft

**Status:** DRAFT, 2026-09-05, written on Fable after measuring the registries. To be reconciled against
`design/agent-reports/2026-09-05-fable-plan-review.md` before anything is built. Nothing here is
implemented.

**Goal it serves** (`ROADMAP_STATUS.md` §0): future returns, every year — year N+1 must be a DATA change.

---

## 1. What was measured (not the report's numbers — mine, today)

There are not four year-registries. There are **six**, and they disagree with each other today:

| # | registry | where | rows | derived? | what it governs |
|---|---|---|---|---|---|
| 1 | `include_bytes!` consts + `*_pdf(year)` arms | `pdf.rs` | 27 consts / 18 fns | **hand** | which PDF a year fills |
| 2 | `include_str!` consts + `tyYYYY()` + `for_year(year)` arms | `map.rs` | 27 consts / 17 top-level map structs | **hand** | which map a year parses |
| 3 | `CENSUS_KEYS` | `tests/common/mod.rs` | 17 stems, map spelling, **no `f1040s1a`** | **hand** | which forms the census checks |
| 4 | `SUPPORTED_YEARS` | `lib.rs:86` | `[2017, 2024, 2025]` | **hand** | the refusal message |
| 5 | `BundledFullReturnTables.by_year` | `tax_tables.rs:101` | **2024 only** | **hand** | THE full-return gate |
| 6 | `emitted_form_years()` | `cite_check.rs:802` | walks `forms/` → 37 (form, year) | **derived** ✓ | the authority ratchet |

Only #6 is derived, and it was derived *yesterday* by wave 2. The other five are the "~85 hand-edits
per year". #5 is the one that matters most and is the smallest: **one `insert` line**.

Two facts the port report did not have:

- **Fills already funnel through `for_year`.** `fill_form_8995(…, year)` calls
  `Form8995Map::for_year(year)?`, and `packet.rs` calls 17 fill functions this way. So the map
  lookup is already one chokepoint per form; the table replaces the *arms*, not the call sites.
- **Every top-level map struct already self-declares `form: String, year: i32`.** A parsed map can
  therefore be checked against the row that loaded it — the table and the artifact witness each other.

And the one thing a table cannot fix by itself, which the report named correctly:
`Form6251Map` still has a lone `line1: MoneyCell`, so the committed TY2025 6251 map (`line1a`/`line1b`)
**cannot parse into any struct in the crate**. Ten TY2025 maps are committed and loaded by nothing. A
row that says "2025/f6251 → `Form6251Map`" would be a row pointing at a parse error. **The struct is
a per-year artifact wearing a per-form name.**

---

## 2. The shape — one row per `(form, year)`, and what a row must carry

```rust
pub struct FormYearRow {
    pub form: &'static str,          // IRS stem: "f1040s1a"
    pub year: i32,
    pub crate_stem: &'static str,    // bundled filename: "schedule_d" for f1040sd — the 2-row alias
    pub revision: Revision,          // see §3 — how this year's PDF relates to another year's
    pub instructions: Option<Instr>, // { stem: "i1040gi", pages: Option<(u32,u32)> }
    pub struct_family: MapFamily,    // WHICH Rust map struct this (form, year) parses into — see §4
}
```

The table is keyed on `(form, year)`, matching `FormYear` in `cite_check.rs`, so the existing derived
ratchet consumes it unchanged.

**What is NOT a column:** paths. Every path is a pure function of `(form, year, crate_stem)` —
`forms/{year}/{crate_stem}.pdf`, `forms/{year}/{crate_stem}.map.toml`,
`design/forms/{year}/{form}--{year}.pdf`, `design/forms/geometry/{form}--{year}.json`,
`design/forms/extract/{form}--{year}.txt`. Storing them would be a second copy of the truth.

---

## 3. `Revision` — the 8275 problem, expressed instead of special-cased

Form 8275 is versioned by REVISION (Rev. 10-2024), not tax year: TY2017/2024/2025 all fill the same
PDF. Wave 2 licensed the alias by byte-equality with `F8275_PDF_2024`. Correct, but a special case
inside one struct. The table expresses it:

```rust
pub enum Revision {
    OwnYear,                 // forms/{year}/{stem}.pdf is this year's own document
    SameAs { year: i32 },    // fill THAT year's PDF; the map is that year's map with `year` restamped
}
```

A `SameAs` row is licensed by a **test that hashes both bundled PDFs and asserts equality**, derived
from the table. `| 2026` stops being a one-token edit: adding `(f8275, 2026, SameAs{2024})` reds the
moment `forms/2026/f8275.pdf` is a different document.

---

## 4. `MapFamily` — the real problem, named

The struct is the per-year artifact. Three honest options:

| option | year N+1 costs | omission compiles? | verdict |
|---|---|---|---|
| A. one struct per form, `Option` every line that ever moved | 0 struct edits if lines only appear/vanish; a struct edit when a line SPLITS | no — an `Option` line nobody sets is a silent blank | **reject**: it is §G-11's laundered blank, in the type |
| B. one struct per (form, revision), enum-dispatched | a new struct when the form's line set changes; nothing otherwise | **yes** — `E0004` on every match | **recommend** |
| C. untyped `BTreeMap<String, MoneyCell>` keyed by line label | 0 | no — a missing line is a runtime `None` | **reject**: throws away the compiler review this repo depends on |

**B, concretely.** `MapFamily` is an enum of the struct *shapes* that exist:

```rust
pub enum MapFamily { F6251_Rev2024, F6251_Rev2025 /* 1a/1b split */, F8949, ScheduleD, … }
```

A row names its family. `for_year` becomes one generic function over the table: find the row, parse
the map into the family's struct, assert the parsed `form`/`year` match the row. Adding TY2026: if
the form's lines did not change, the row reuses the existing family and **no Rust changes**. If they
did (Schedule 1-A: 10 of 219 fields survive), a new family variant is added — and that is a code
change **because the form genuinely changed**, which is the irreducible human step 19 of the runbook.
The table makes the boundary explicit: *data change when the IRS moved a box; code change when the
IRS moved a line.*

---

## 5. Where the table lives — and why not a `const`

A `const FORMS: &[FormYearRow]` is still a hand-list. It only stops being one if something derives the
*expected* set from the filesystem and asserts the table equals it **both ways** (a row with no
`forms/{year}/{crate_stem}.pdf` is a lie; a bundled PDF with no row is unreachable). That check is
`supported_years_cross_product.rs`'s job, generalised: **the filesystem is the authority, the table is
the index, and the test proves index == authority.**

`include_bytes!`/`include_str!` need compile-time literal paths, so the consts cannot be table-driven at
runtime. Two honest homes:

| home | what a new year costs | risk |
|---|---|---|
| **`build.rs` codegen** — walk `forms/`, emit the consts + a `match (form, year)` into `OUT_DIR` | **0 edits**: drop a directory in | a build script is code nobody reviews; must itself be tested against a planted tree |
| **hand `const` table + filesystem ⇔ table test** | 1 row per (form, year) | the row is written by a human, checked by a machine |

**Recommendation: the `const` table with the two-way test, NOT codegen — for now.** Codegen makes the
row invisible, and the row is where a human records `revision` and `struct_family`, which are
judgments (runbook steps 17 and 19). A generated row cannot carry a judgment. Revisit codegen only for
the consts (the mechanical half), never for the row.

---

## 6. What the five hand-lists become

| today | after |
|---|---|
| `SUPPORTED_YEARS` | `FORMS.iter().map(|r| r.year).collect()` — derived, one place |
| `*_pdf(year)` × 18 | one `pdf_for(form, year)` reading the table; the 27 consts stay (generated or not) |
| `for_year` × 17 | one generic `map_for(form, year)` dispatching on `MapFamily` |
| `CENSUS_KEYS` | `FORMS` filtered by year — and it gains `f1040s1a`, which the hand-list lost |
| `BundledFullReturnTables` 2024-only | **untouched by this design** — it is the compute gate and stays fail-closed until TY2026 params are transcribed |

---

## 7. Sequencing — proof before switch

1. Land `FORMS` as a `const` with **every existing (form, year) on disk** — 37 rows — plus the two-way
   filesystem test. Nothing consumes it yet. **Red on any disagreement is the deliverable.**
2. Land `map_for`/`pdf_for` beside the old paths, with a test that they agree on all 37 for every
   existing arm. Switch `packet.rs` fills over one at a time; each switch reds if behaviour changes.
3. Delete the 18 + 17 hand arms and `SUPPORTED_YEARS`. The compiler names every remaining reader.
4. Add `F6251_Rev2025` (the 1a/1b family) and wire the ten orphaned TY2025 maps. **This is where the
   TY2025 6251 map first parses.**
5. Only then: TY2026 rows, as the drafts become finals.

## 8. What this does NOT do, stated so it is not overclaimed

- It does not close runbook steps 16/17/19/20/21/24. It **locates** them: every judgment lives in a
  row (`revision`, `struct_family`) or a `[census]` block, and the machine checks everything else.
- It does not touch compute (`tax_tables.rs`, `BundledFullReturnTables`). The compute gate stays
  fail-closed until TY2026 `FullReturnParams` are transcribed from Rev. Proc. 2025-32 — a separate,
  already-archived-source task.
- It does not make the Schedule 1-A emitter exist. There is **no** `Schedule1AMap` or
  `fill_schedule_1a` anywhere in `btctax-forms` today (count: 0). That is P2's missing 17th form.
