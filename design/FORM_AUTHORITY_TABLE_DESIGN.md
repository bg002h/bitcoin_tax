# The year package — design, r2

**Status: r2, reconciled with the Fable plan review** (`design/agent-reports/2026-09-05-fable-plan-review.md`,
findings I1/I2/I3 and the "recommended shape" section; every claim it makes was machine-checked in
`…-fable-plan-review.VERIFICATION.md` before this was written). r1 (`95b1a368`) proposed a Rust
`const FORMS: &[FormYearRow]` with `Revision`/`MapFamily` enums and argued against codegen because
"a generated row cannot carry a judgment". The review's answer is not a refutation but a relocation:
**the judgments go in the `.map.toml` header the human already writes, and codegen touches only the
mechanical binding.** That is r1's own "codegen for the consts, never for the row" — with the row moved
out of Rust. r1's measurements (§1) stand; its shape (§2–§7) is replaced.

## 1. What was measured (unchanged from r1; verified 2026-09-05)

There are **six** year-registries, and they disagree with each other today:

| # | registry | where | rows | derived? | what it governs |
|---|---|---|---|---|---|
| 1 | `include_bytes!` consts + `*_pdf(year)` arms | `pdf.rs` | 27 consts / 18 fns | **hand** | which PDF a year fills |
| 2 | `include_str!` consts + `tyYYYY()` + `for_year(year)` arms | `map.rs` | 27 consts / 17 top-level map structs | **hand** | which map a year parses |
| 3 | `CENSUS_KEYS` | `tests/common/mod.rs` | 17 stems, map spelling, **no `f1040s1a`** | **hand** | which forms the census checks |
| 4 | `SUPPORTED_YEARS` | `lib.rs:86` | `[2017, 2024, 2025]` | **hand** | the refusal message |
| 5 | `BundledFullReturnTables.by_year` | `tax_tables.rs:101` | **2024 only** | **hand** | THE full-return gate |
| 6 | `emitted_form_years()` | `cite_check.rs:797` | walks `forms/` → 37 (form, year) | **derived** ✓ | the authority ratchet |

The filesystem today: 37 `.map.toml` and 37 PDFs under `crates/btctax-forms/forms/{2017,2024,2025}`
(5 / 17 / 15). Every fill already funnels through `Map::for_year(year)` — 17 call sites in
`packet.rs` — so the table replaces the *arms*, not the call sites. Every top-level map struct already
self-declares `form: String, year: i32`. And `Form6251Map` still has a lone `line1: MoneyCell`, so
the committed TY2025 6251 map (`line1a`/`line1b`) cannot parse into any struct in the crate — the
struct is a per-**line-set-revision** artifact wearing a per-form name (§7).

Beyond the forms crate, the year is also spread over: `TRANSITION_DATE` / `TY2025_RETURN_DUE`
(`conventions.rs:17,19`, singular consts), `FORMS_ABSENT_FROM_YEAR` (inside `tests/field_census.rs:100`),
`BundledPrices` (a compiled-in CSV ending **2026-06-03**), 16 attachment-sequence literals in
`packet.rs`, and two `selected_year: 2025` literals in the TUIs. None of those has a year-level home.

## 2. The principle

**Stem is code; year is data; the filesystem is the registry.** A new *form* needs a filler and is
correctly a code change. A new *year* of an existing form must be files under a directory and nothing
else. **Every gate walks the glob; nothing walks a list.** A list is a claim about the filesystem; the
filesystem is the fact — and a row someone forgets is a form outside every gate that walks the list,
reporting nothing (`HARNESS.md` F2, the exact shape `field_census.rs` was fixed out of today).

What a glob cannot produce — instructions stem, page range, versioning, template hash, which schema
parses the map — has the same author and lifecycle as the map file. So it lives **in the map header**,
where it cannot describe a map that does not exist or omit one that does.

## 3. Layer 0 — location is status

```
crates/btctax-forms/forms/<year>/<stem>.pdf          bundled template — FINAL authority only
crates/btctax-forms/forms/<year>/<stem>.map.toml     the ROW (header) + bindings + [identity] + [census]
crates/btctax-forms/forms/<year>/YEAR.toml           the year record (§6)
crates/btctax-forms/forms-provisional/<year>/…       port-machine output against a DRAFT; never globbed
```

A provisional map cannot be loaded *by construction*: the build never sees the directory. Same move
as `-DRAFT` in the archive filename and `irs-dft` in the URL — three signals, one rule, no runtime
flag. This replaces r1's idea of a `provisional = true` field.

## 4. Layer 1 — the row is the map header

```toml
form                = "f6251"           # crate stem (exists today)
year                = 2026              # (exists today)
irs_stem            = "f6251"           # IRS basename; differs only for schedule_d/schedule_se — STEM_ALIASES retires into this
versioning          = "annual"          # or { periodic = "Rev. 10-2024" }: a periodic form aliases a prior revision BY HASH, never by year list
template_sha256     = "…"               # of the bundled PDF; joined BY CONTENT to MANIFEST.json, whose entry must be is_authority()
instructions        = "i6251"           # "" only for a self-instructing form (f8275)
instr_pages         = [101, 110]        # only for i1040gi-hosted schedules; the human records it once
line_set            = "f6251/2025"      # WHICH transcription struct parses this map (§7). constants-only year ⇒ same line_set; renumber ⇒ new one
attachment_sequence = "32"              # read from the extract; today 16 literals in packet.rs
```

`versioning` is r1's `Revision`, and `line_set` is r1's `MapFamily` — the same two judgments, written
by the human in the file they already author instead of in a Rust table.

**Derived by convention, never stored:** the form extract `design/forms/extract/<irs_stem>--<year>.txt`,
the instructions extract `<instructions>--<year>.txt`, the geometry `design/forms/geometry/<irs_stem>--<year>.json`,
the census (the `[census]` table in the same file). A stored path is a second copy of the truth.

**B1 kills, each planted before its checker ships:** a header missing a required field → parse refusal
(the existing `deny_unknown_fields` structs gain required fields); `template_sha256` ≠ the file → red;
that hash absent from `MANIFEST.json`, or present on an `is_draft()` entry → red; `line_set` naming no
schema → **compile error** via the exhaustive match in §5; `attachment_sequence` ≠ the extract's
"Attachment Sequence No." → red.

## 5. Layer 2 — the binding is a `build.rs`

`crates/btctax-forms/build.rs` globs `forms/<year>/` under `CARGO_MANIFEST_DIR` only and emits
`bundled.rs`:

```rust
pub enum Stem { F1040, F1040s1, F1040s1a, F1040s2, /* … the closed set — CODE */ }
pub fn template(stem: Stem, year: i32) -> Option<&'static [u8]>;   // include_bytes! per file found
pub fn map_text(stem: Stem, year: i32) -> Option<&'static str>;    // include_str!  per file found
pub fn bundled_years() -> &'static [i32];                          // replaces SUPPORTED_YEARS
// println!("cargo:rerun-if-changed=forms");
```

Every `Map::for_year` becomes one line — `map_text(Stem::F6251, y).ok_or(UnsupportedYear(y))?` — then
parse into the struct the header's `line_set` names, through an exhaustive `match` (an unknown
`line_set` cannot compile). The 18 + 17 hand arms and `SUPPORTED_YEARS` cease to exist; `error.rs`'s
"supported years" sentence is built from `bundled_years()`.

**What the build script must never do:** decide anything. It binds files; every judgment is in a
header. A build script that classifies is r1's objection come true.

**The publishing trap** (`crate-publishing-state`: an escaping `include_str!` once shipped a broken
tarball with exit 0): the script reads only under the manifest dir, and a `cargo package --list` gate
asserts every globbed file is in the tarball. Measured today: no `build.rs` exists anywhere in the
workspace, `btctax-forms/Cargo.toml` has no `include`/`exclude`, and `forms/` is tracked — so the
tarball carries it by default and the gate is what keeps that true.

*Weaker alternative* if a build script is refused for the published crate: `forms port --emit-wiring`
writes a committed `bundled.rs`, and `forms wire --check` runs in `make check`. Weaker because drift
becomes a test failure rather than a non-event.

`supported_years_cross_product`'s `wired`/`dispatch` obligations stay as the B1 witness that the script
binds every file on disk (tautological under this layer — that is what a witness is for).

## 6. Layer 3 — the year record, `forms/<year>/YEAR.toml`

```toml
year                = 2026
status              = "preparing"         # preparing | filable — a DECLARATION; full_return_for(year) stays the compute gate
return_due          = 2027-04-15          # replaces TY2025_RETURN_DUE / TRANSITION_DATE
forms_expected      = ["f1040", "f1040s1", "f1040s1a", "f1040s2", "…"]     # runbook step 1's OUTPUT, committed
forms_absent        = { f8275 = "Rev. 10-2024 aliased by hash from 2024" }   # FORMS_ABSENT_FROM_YEAR moves here
tables              = "Rev. Proc. 2025-32; Pub. L. 119-21"                   # citation of record for TaxTable + FullReturnParams
oracles             = { ots = "2026 — not published (expected ~2027-01-27)", taxcalc = ">= 6.8.2" }
prices_through      = 2026-12-31          # BundledPrices::max_date() must reach this before status = filable
information_returns = { f1099da = { proceeds = true, basis = true } }         # routes Form 8949's boxes (C1)
```

This is the home runbook step 1 never had: **the intended set**, so "a form present that was not
expected" and "a form expected that is absent" both become visible. `YearReadiness` (port report D7)
is then *declared* (this file) versus *actual* (glob, tables, params, prices), rendered on every
number-bearing surface and used to build every refusal string.

**B1 kills:** a form in `forms_expected` with no map → red; a map on disk in neither `forms_expected`
nor `forms_absent` → red (the "third state"); `status = "filable"` with `full_return_for(year).is_none()`
→ red; `prices_through` beyond `BundledPrices::max_date()` with `status = "filable"` → red.

The `information_returns` slot is the review's C1: TY2026 is the first year brokers report **basis** on
Form 1099-DA (TD 10000, effective 2026-01-01), and the TY2025 maps box every 8949 row I/L ("not
reported to you on Form 1099-DA") unconditionally. The year record says what regime the year is under;
the filer's answer (`none | proceeds | basis` per disposition source) is input-surface work filed
separately. This design gives it a home, not an implementation.

## 7. Layer 4 — the label table is the join, and the `*Map` struct is per line-set revision

`LineCoverage { form, year, line, field, instruction }` already exists and is already re-verified
against the year's extract by `Coverage::quoting(year)` (`line_coverage.rs:193`). It is the join
between a map cell and a printed line; it is not a new artifact.

**The struct's fate, called (review I2).** A transcription struct is per **line-set revision**, not
per year: `Form6251Map` (`line1..line40`, 39 `(&map.lineN, f.lineN)` pairs in `form6251.rs`) serves
2025 and 2026 alike because 6251 did not renumber between them, and it broke at 2024→2025 (1a/1b).
So: keep line-named transcription structs, **one per line-set revision, selected by `line_set`**;
expose semantic accessors for cross-year consumers (`AbsoluteReturn` is already the model); and for a
**renumber-only** revision, generate the new struct's skeleton and the filler's `(map cell, printed
field)` pairing from the label table rather than hand-writing them. For a **rebuilt part** the machine
stops and a person transcribes under the `CLAUDE.md` rule.

The `CLAUDE.md` clause is amended to say what it always meant: *fields are named for the line **within
a transcription struct**; cross-year quantities are semantic.* `schedule_1a.rs` was built under it and
reviewed green; the port report's three-axis table listing it as the bad example was a doctrine
reversal presented as synthesis, and is withdrawn.

## 8. The gates — all walking the glob, each with its kill

map ⊆ PDF fields (exists) · census ∪ map == PDF (exists; register) · `template_sha256` == file == an
`is_authority()` manifest entry · `geometry.pdf_sha256 == template_sha256` · doc comment ⊆ that line's
extract text (17/17 on f8959) · `for_year(y)` Ok ⇔ file on disk (tautological under §5; kept as
witness) · `forms_expected` == present ∪ absent-with-reason · every `Stem` has an arm in
`fill_full_return`'s exhaustive `PrintedForms` destructure (`packet.rs:60-83`, "★ NO `..`", exists).

## 9. What the hand-lists become

| today | after |
|---|---|
| `SUPPORTED_YEARS` | `bundled_years()` — emitted from the glob |
| `*_pdf(year)` × 18 | `template(stem, year)` |
| `for_year` × 17 | `map_text(stem, year)` + one exhaustive `match` on `line_set` |
| `CENSUS_KEYS` | the glob — and it gains `f1040s1a`, which the hand-list lost |
| `STEM_ALIASES` (2 rows) | the `irs_stem` header field |
| `FORMS_ABSENT_FROM_YEAR` | `YEAR.toml` `forms_absent` |
| `TY2025_RETURN_DUE`, `TRANSITION_DATE` | `YEAR.toml` `return_due` |
| `selected_year: 2025` × 2 | derived from `YearReadiness` |
| 16 attachment-sequence literals | the header field, checked against the extract |
| `BundledFullReturnTables` 2024-only | **untouched** — it is the compute gate; `YEAR.toml` `status` declares, it decides |

## 10. Sequencing — proof before switch

1. **Header parse + glob-derived row set + the two-way test**, consuming nothing. Add the header fields
   to the 37 existing maps; parse them with required fields; assert `glob == emitted_form_years()`
   both ways; plant each §4 kill. **Red on any disagreement is the deliverable.**
2. **`build.rs` beside the old arms**, with a test that `template`/`map_text` agree byte-for-byte with
   every existing `include_*` const, and the `cargo package --list` gate.
3. **Switch `packet.rs` fills over one at a time**; delete the 18 + 17 arms and `SUPPORTED_YEARS`. The
   compiler names every remaining reader.
4. **`YEAR.toml` for 2017/2024/2025** with `YearReadiness` and its kills; move the four literals in.
5. **`line_set`: add `f6251/2025`** (the 1a/1b struct) and wire the ten orphaned TY2025 maps. **This is
   where the TY2025 6251 map first parses.**
6. Only then TY2026 rows — `forms-provisional/2026/` today, promoted file-by-file as finals land.

## 11. What this does NOT do

- It does not close runbook steps 16/17/19/20/21/24. It **locates** them: every judgment lives in a
  header field, a `[census]` block or `YEAR.toml`, beside the thing it decided.
- It does not touch compute. `full_return_for(2026)` stays `None` until TY2026 `FullReturnParams` are
  transcribed — a NOW item with its own follow-up, since Rev. Proc. 2025-32 §2.10 is in the tree.
- It does not build the Schedule 1-A emitter (`Schedule1AMap` / `fill_schedule_1a`: count 0 today) or
  the 1099-DA input surface. It gives each a slot.
- It does not decide whether TY2017 ships. That is the owner's open call.
