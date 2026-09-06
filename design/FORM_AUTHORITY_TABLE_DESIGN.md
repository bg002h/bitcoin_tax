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
authority           = "not-yet-archived: <reason>"   # OPTIONAL. The ONLY excuse the MANIFEST join accepts; six rows today (all five TY2017 + f8283/2024)
extract_override    = "…"               # OPTIONAL. Only while a second extract root exists (f1040s1a/2025); see §9
instructions        = "i6251"           # fNNNN → iNNNN with the IRS's aliases; every bundled form has one (f8275 → i8275), so "" is not used — step-1 review P7
instr_pages         = [101, 110]        # only for i1040gi-hosted schedules; the human records it once
line_set            = "f6251/2025"      # the LINE-SET REVISION this map is a transcription of (§7): constants-only year ⇒ same line_set; renumber ⇒ new one
attachment_sequence = "32"              # read from the extract; today 16 literals in packet.rs. ABSENT on the 1040 itself, which carries no sequence number
```

`versioning` is r1's `Revision`, and `line_set` is r1's `MapFamily` — the same two judgments, written
by the human in the file they already author instead of in a Rust table.

**`line_set` names a revision, and the match is many-to-one** (fold review F5). Today five structs —
`Form1040Map`, `Form8949Map`, `Form8283Map`, `ScheduleDMap`, `ScheduleSeMap` — each absorb three
renumbered revisions (2017/2024/2025) with `Option` + `#[serde(default)]` (`Form1040Map`'s own doc:
`line7a` is "line 7a for 2025, line 7 for 2024, line 13 for 2017"). So those **15** maps (5 structs × 3 years) get **per-year
`line_set`s** (`f1040/2017`, `f1040/2024`, `f1040/2025`) that all resolve to one struct in §5's match,
and splitting a shared struct into per-revision structs is later work, filed per form. The rule is:
the header records what the document IS (its revision); the match records what parses it today.
Deciding the other way — one `line_set` per struct — would erase the renumber the field exists to
name.

**Derived by convention, never stored:** the form extract `design/forms/extract/<irs_stem>--<year>.txt`,
the instructions extract `<instructions>--<year>.txt`, the geometry `design/forms/geometry/<irs_stem>--<year>.json`,
the census (the `[census]` table in the same file). A stored path is a second copy of the truth.

**B1 kills, each planted before its checker ships:** a header missing a required field → parse refusal
(the existing `deny_unknown_fields` structs gain the required fields AND the two optional ones above —
an unknown key stays a refusal); `template_sha256` ≠ the file → red; that hash absent from
`MANIFEST.json`, or present on an `is_draft()` entry → red **unless** the header carries
`authority = "not-yet-archived: …"` (six rows today; the field is the excuse, and the ratchet is that
the count may only shrink); `line_set` naming neither a schema nor `Unwired` → **compile error** via the
exhaustive match in §5 (`Unwired` is the explicit arm for the ten TY2025 maps until step 5 wires them —
§10); `attachment_sequence` ≠ the extract's "Attachment Sequence No." → red, **except** on a form the
IRS prints no sequence number for (the 1040 itself).

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
parse into the struct the header's `line_set` names — or `Unwired`, which returns
`UnsupportedYear(year)` (§10 step 3) — through an exhaustive `match` (an unknown `line_set` cannot
compile). The 18 + 17 hand arms and `SUPPORTED_YEARS` cease to exist; `error.rs`'s
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
status              = "preparing"         # preparing | slice | filable — a DECLARATION; full_return_for(year) stays the compute gate. `slice` = the crypto-slice packet only (TY2017)
return_due          = 2027-04-15          # replaces TY2025_RETURN_DUE. ★ NOT TRANSITION_DATE (step-4 amendment): that is the Rev. Proc. 2024-28 per-wallet snapshot date (§7.4), regulatory and one-time, ~90 readers in core's funds-safety logic — it stays in `conventions`
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
extract text (17/17 on f8959) · `for_year(y)` Ok ⇔ file on disk **and** its `line_set` wired (kept as
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
| `TY2025_RETURN_DUE` | `YEAR.toml` `return_due` — DECLARED there; its one code reader (`btctax_core::project::resolve`) cannot see the forms crate, so `tests/year_record.rs` holds the declaration equal to the constant until the reader relocates |
| `TRANSITION_DATE` | **stays in `conventions`** — not a year fact (the Rev. Proc. 2024-28 snapshot date, §7.4); listed here so the omission is deliberate, not forgotten |
| `selected_year: 2025` × 2 | derived from `YearReadiness` |
| 16 attachment-sequence literals | the header field, checked against the extract (the 1040 has none). Step 1 collapsed the literals into one `packet::attachment_sequence(stem, year)` held to every row by test; it retires into the row at step 3 |
| `cite_check.rs::FORMS` (`FormAuthority { form, year, instructions, instr_pages, extract_stem }`, one row) | `instructions` / `instr_pages` header fields. ★ Its `extract_stem` points at a SECOND extract root, `crates/btctax-core/src/tax/fixtures/` (`schedule_1a_2025_form.txt`, `schedule_1a_2025_instructions.txt`), which §4's derive-by-convention rule cannot express. Decision (corrected, fold review r2 G3): the two fixtures are a SECOND EXTRACTION of files already under the convention — `f1040s1a--2025.txt` (11,153 B vs the fixture's 11,443 B) and pages 101–110 of `i1040gi--2025.txt` (the fixture is a 52,672 B slice; the booklet extract is 616,274 B). **Nothing moves**: a `mv` would clobber the booklet extract every other i1040gi-hosted schedule's gate reads, and pointing `tables.rs:1351,1365` at `design/` would make two escaping `include_str!`s — §5's publishing trap. The instructions fixture is regenerated at test time from the booklet extract using the header's `instr_pages`; the form fixture is replaced by `f1040s1a--2025.txt` **once a test asserts every `FORMS` quotation still resolves against it** (the two extractions differ by 290 B). Both are then deleted; until then the header carries `extract_override = "…"` and the ratchet below keeps its row (fold review F2) |
| `cite_check.rs::AUTHORITY_NOT_YET_ARCHIVED` (shrink-only, `(form, years)`, 36 of 37 pairs excused today) | This is a DIFFERENT "archived" from the manifest join: it means "no `FormAuthority` row + extract for cite-check", and it retires as map headers gain `instructions`/extract coverage. The MANIFEST join (`template_sha256`) is the other notion and reds today on exactly **6 of 37** templates — all five TY2017 and `forms/2024/f8283.pdf` (measured by sha256 join, fold review F7) — so the header gets `authority = "not-yet-archived: <reason>"` for those six, and the join kill treats that field as the excuse. Two of the six ride on the open TY2017 decision |
| `BundledFullReturnTables` 2024-only | **untouched** — it is the compute gate; `YEAR.toml` `status` declares, it decides |

## 10. Sequencing — proof before switch

1. ✅ **DONE `6267b6b1` (2026-09-05)** — 37 rows written by script (values computed, never typed);
   `MapRow` + `Versioning`; nine fields on all 17 structs; the four kills each observed red on a
   planted tempdir copy; the two-way test through `irs_stem` in xtask; 3016 tests. **The first kill
   found a live defect before it existed:** Form 8283 Rev. 12-2025 prints Attachment Sequence No.
   **36**, `packet.rs` pushed `"155"` for every year — the TY2025 packet stapled it last. Fixed as
   one `attachment_sequence(stem, year)` held to every row. Independent phase review: pending.
   **Header parse + glob-derived row set + the two-way test**, consuming nothing. Add the header fields
   to the 37 existing maps (`attachment_sequence` absent on the three `f1040` maps; per-year
   `line_set`s on the 15 maps served by the five shared structs, §4; the ten unwired TY2025 maps — `f6251`, `f8959`, `f8960`, `f8995`, `f1040s1a`, `f1040s2`, `f1040s3`, `f1040sa`, `f1040sb`, `f1040sc` — carry a `line_set` with no schema behind it until step 5); parse them with required
   fields; assert **`{ (irs_stem, year) from the glob's headers } == emitted_form_years()`** both ways
   — `emitted_form_years()` keys on the IRS basename (`cite_check.rs:853`), so the comparison goes
   through the new `irs_stem` field or it reds on `schedule_d`/`schedule_se` × 3 years before any
   defect exists (fold review F4). Plant the four kills that are self-contained here: missing required
   field → parse refusal; `template_sha256` ≠ file; the manifest join — **expected red on exactly 6
   rows** (five TY2017 + `forms/2024/f8283.pdf`) until their `authority = "not-yet-archived: …"`
   header is written, which is the excuse slot (F7); `attachment_sequence` ≠ the extract's
   "Attachment Sequence No." (tolerating the 1040, F8). The fifth kill — `line_set` naming neither a schema nor
   `Unwired` → compile error — needs §5's match and is planted at step 3 (F6). **Red on any disagreement is the
   deliverable.**
2. ✅ **DONE `68b86b8e` (2026-09-05)** — `build.rs` (binds only; refuses a non-year directory and an
   unpaired file, both observed red), `bundled.rs` with the hand-written `Stem` (18, incl. `F1040s1a`
   with no filler yet), `template`/`map_text`/`BUNDLED_YEARS`/`BUNDLED`; byte-agreement with all 54
   old consts; the `cargo package --list` gate in xtask with its kill. 3022 tests.
   **`build.rs` beside the old arms**, with a test that `template`/`map_text` agree byte-for-byte with
   every existing `include_*` const, and the `cargo package --list` gate.
3. ✅ **DONE `bc6dce35` (2026-09-05)** — `LineSet` (37) / `Schema` (17 + `Unwired`) / the exhaustive
   `schema()` match; every `for_year` and `*_pdf` body reads the glob + row; 54 consts and the
   `SUPPORTED_YEARS` hand-list deleted (the name is now `BUNDLED_YEARS`, derived); the periodic alias
   is `bundled::periodic_template`, bundled years only, licensed by hash; 3023 tests. Call sites did
   not move — the arms went, not the names.
   **Switch `packet.rs` fills over one at a time**; delete the 18 + 17 arms and `SUPPORTED_YEARS`. The
   compiler names every remaining reader. The `line_set → struct` match lands here with an explicit
   **`Unwired`** arm for the ten TY2025 maps named in step 1 — the fifth kill is "a `line_set` that is
   neither a schema nor `Unwired`", so wiring one at step 5 is a deletion from that arm and forgetting
   one still cannot compile (fold review r2 G2).
4. ✅ **DONE (2026-09-05)** — `forms/<year>/YEAR.toml` ×3 (expected sets computed from disk; absences
   with reasons), bound by `build.rs` (a year directory without one is a build error), `YearRecord`
   with `partition_problems` (expected ∪ absent == `Stem::ALL`) and `glob_problems` (expected == on
   disk), `YearReadiness` in `btctax-cli` (declared vs table/params/forms/prices, with the
   filable-without-params and prices-short kills), `default_year()` from the glob for both TUIs,
   `FORMS_ABSENT_FROM_YEAR` and `CENSUS_KEYS` retired into the record and `Stem::ALL`.
   **`YEAR.toml` for 2017/2024/2025** with `YearReadiness` and its kills; move the four literals in.
5. ◐ **EIGHT OF TEN WIRED (2026-09-05)** — `f1040s2/s3/sa/sb/sc`, `f8959`, `f8960`, `f8995` for
   TY2025: each verified (map ⊆ PDF fields; the line→printed-label join, derived over every map;
   `[census]` present; 2024→2025 deltas were field renames with no line moved) and measured to parse
   into its 2024 struct — a constants-only revision of the same line set, so the arm is the 2024
   struct and NO new Rust. `tests/line_set_wiring.rs` pins wired ⇔ parses over the ten (plant: un-wire
   one → two tests red). **Two remain `Unwired`, deliberately:** `f6251/2025` (line 1 → 1a/1b, a
   REBUILD: a new transcription struct, to be written once for TY2026 which shares the layout —
   TY2025 is paused) and `f1040s1a/2025` (no struct exists: the Schedule 1-A emitter is P2's missing
   17th form, a NOW item for TY2026).
   **`line_set`: add `f6251/2025`** (the 1a/1b struct) and wire the ten orphaned TY2025 maps. **This is
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
