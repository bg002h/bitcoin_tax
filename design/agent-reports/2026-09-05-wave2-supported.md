# Wave 2 / F9 — `SUPPORTED_YEARS` asserts more than anything checks

**Agent:** wave-2 fixer, package finding **F9**.
**Files touched:** `crates/btctax-forms/src/lib.rs` (doc comment only, no behaviour change) and the new
`crates/btctax-forms/tests/supported_years_cross_product.rs`.
**Result:** the cross-product check is landed, green, and its record says 21 of 37 shipped
(year, form) cells are missing at least one supporting artifact — **54 obligations unmet**.

---

## 1. What was wrong — measured, not described

`crates/btctax-forms/src/lib.rs:68` declares `SUPPORTED_YEARS = &[2017, 2024, 2025]`. Before this
change **no test in the workspace referred to it**: grep returns its own definition, doc comments, and
`sp4.rs`/`kats.rs` iterating it as a loop variable — never as a claim under test. Two directions were
unguarded at once:

- a year could be **listed with nothing behind it** (every fill refuses while the product claims the
  year), and
- a year could be **bundled on disk and never listed** (assets compiled in, unreachable).

Nothing anywhere held the product `(SUPPORTED_YEARS × the forms this build ships)`.

### The matrix, as measured 2026-09-05

3 supported years × 18 bundled form stems = **54 cells**: **37 bundled**, **2 aliased**, **15 absent**.
Eight obligations per bundled cell (`map`, `wired`, `dispatch`, `note`, `manifest`, `extract`,
`geometry`, `census`). **21 cells carry a gap; 54 obligations are unmet.**

| year | shape of the gap | cells | unmet |
|---|---|---|---|
| **2017** | fillable, and **no authority chain at all** — no note, no `MANIFEST.json` entry, no extract, no geometry fixture, no `[census]` | 5 of 5 | 25 |
| **2024** | complete **except Form 8283**, whose bundled asset matches no committed note and no manifest sha | 1 of 17 | 4 |
| **2025** | **10 of 15 committed forms unreachable** (`wired` + `dispatch`); 5 more reachable with no `[census]` | 15 of 15 | 25 |

### ★ Is TY2017 "supported in name only"? — No. It is **supported in fill, unsupported in evidence.**

This is the fact the finding asked for, and it is not the one I expected. TY2017 is genuinely wired:
five forms (`f1040`, `f8283`, `f8949`, `schedule_d`, `schedule_se`), five committed maps, five
`include_bytes!`, and a `for_year(2017)` arm in all five map types plus all five PDF accessors. A
TY2017 export really does produce a filled IRS PDF.

What TY2017 has **none** of is everything that makes a shipped year *checkable*:

- **0** provenance notes — `find design/forms -name '*2017*'` is empty; nothing hash-pins the five
  bundled PDFs to an irs.gov document, so they cannot be re-derived or re-verified after an IRS
  revision. (The only two `2017` strings in `MANIFEST.json` are an SSA COLA determination.)
- **0** committed extracts — every citation assertion in the repo reads `design/forms/extract/`, so
  **no line of the TY2017 forms is cited by anything**.
- **0** geometry fixtures — no line→label join exists for any TY2017 page.
- **0** `[census]` sections — `field_census.rs`'s *"★★★ THE GATE"* (map FQNs ∪ census FQNs == PDF field
  set) has nothing to run against for TY2017, so **no gate has ever asked whether the TY2017 maps
  account for every field on the TY2017 pages.**

So the honest sentence for a reader is: *btctax fills TY2017 forms from assets whose provenance is
unrecorded and whose field coverage no instrument has ever checked.* That is a narrower claim than
"supported in name only" and a worse-shaped one — the product is not refusing, it is emitting.

---

## 2. What I changed

**`crates/btctax-forms/tests/supported_years_cross_product.rs`** (new, 6 tests).

Both axes are derived, never hand-listed:

- **years** — read from `btctax_forms::SUPPORTED_YEARS` itself;
- **forms** — the union of `forms/<year>/*.pdf` stems over every bundled year directory;
- **the join to the archive is by CONTENT** — each bundled PDF is sha256'd and looked up in the
  `# sha256` line of the committed `design/forms/*/<stem>--<year>.pdf.txt` notes.

That last point is the reason no fifth registry hand-list was added. The crate spells Schedule D
`schedule_d` and the archive spells it `f1040sd`; joining by content hash spans the two spellings
without a translation table, so the drift named in the port report (R16 / `CLAUDE.md` §B3 — the
registry existing four times as independent hand-lists) is not made worse by this file. It also means
"an asset no note describes has no provenance" is decided by bytes, not by naming.

Tests:

| test | what it holds |
|---|---|
| `the_cross_product_matrix_matches_the_recorded_gaps` | **the gate** — every cell against `KNOWN_GAPS` / `KNOWN_ALIASES` / `BUNDLED_FORMS_PER_YEAR`, exact-match in both directions |
| `supported_years_and_bundled_year_directories_agree` | a listed year with no assets fails; a bundled year nothing lists fails unless recorded |
| `every_bundled_stem_is_probed_or_recorded` | a committed form that reaches `map_resolves`'s `_ => None` arm unrecorded fails (*skipping is not passing*) |
| `every_committed_map_has_its_bundled_pdf` | a map with no page beside it fails |
| `the_unsupported_year_refusal_names_exactly_the_supported_years` | the sentence a filer reads must name exactly `SUPPORTED_YEARS` |
| `the_gate_reds_on_every_planted_defect` | B1 — every verdict watched rejecting its own defect class |

**The record is a ratchet, and it is deliberately not green-by-narrowing.** `KNOWN_GAPS` lists all 21
cells artifact-by-artifact. A gap that grows fails; a gap that *closes* also fails, with instructions
to delete the line.

★ **Removing TY2017 from `SUPPORTED_YEARS` cannot make this green** — verified by planting exactly that
(plant 2 below). Its `KNOWN_GAPS` rows stop matching a cell *and* `forms/2017/` becomes a
bundled-but-unsupported directory. Two tests red, from opposite sides.

**`crates/btctax-forms/src/lib.rs`** — doc comment on `SUPPORTED_YEARS` only. It now names the gate
that holds it and states the three counter-intuitive facts the gate pins (TY2017 unevidenced, TY2025
10-of-15 unreachable, four year-sets governing four entry points). **No change to the constant's
value and no code change.** `diff` against the pre-change file is 18 added doc lines and nothing else.

---

## 3. Which test reds for which planted defect

Nine plants, each run and observed. Plants 1–3 mutate the **real** crate source; 4–5 mutate the test's
own measurement path; 6–9 mutate the record. Every file was restored from a byte-compared backup
(`diff` clean, and the suite is 6/6 green afterwards).

| # | plant | test that RED | message |
|---|---|---|---|
| 1 | `SUPPORTED_YEARS = [2017, 2024, 2025, **2026**]` in `lib.rs` | `supported_years_and_bundled_year_directories_agree` **and** `the_unsupported_year_refusal_names_exactly_the_supported_years` | *"SUPPORTED_YEARS lists [2026], and forms/ has no directory for them"*; *"lists 2026 and the refusal a filer sees does not mention it"* |
| 2 | `SUPPORTED_YEARS = [2024, 2025]` (drop TY2017 — the dodge) | `the_cross_product_matrix_matches_the_recorded_gaps`, `supported_years_and_bundled_year_directories_agree`, `the_unsupported_year_refusal…` | *"KNOWN_GAPS records (2017, \"f1040\") … Removing a year does not discharge its gaps"* (×5) + *"forms/ bundles [2017], which SUPPORTED_YEARS does not list"* |
| 3 | add `include_bytes!("../forms/2025/f6251.pdf")` + its `include_str!` to `lib.rs` | `the_cross_product_matrix…` | *"(2025, f6251) records {dispatch, wired} but measures {dispatch} — newly present [\"wired\"]"* |
| 4 | point `archive_root()` at `design/forms/geometry` | `the_cross_product_matrix…` | *"read 0 note file(s) and parsed 0 sha256 line(s) — the JOIN is broken … Fix the reader, do not record the gaps"* |
| 5 | restrict the note walk to directories containing `2024` | `the_cross_product_matrix…` | 15 × *"(2025, …) newly missing [\"extract\", \"geometry\", \"note\"]"* — and **no 2024 cell moved**, so the join discriminates per cell rather than wholesale |
| 6 | `(2025, "f1040", &[])` — drop one obligation from one row | `the_cross_product_matrix…` | *"records {} but measures {\"census\"} — newly missing [\"census\"]"* |
| 7 | delete the `(2024, "f8283")` row | `the_cross_product_matrix…` | *"is missing {extract, geometry, manifest, note} and NOTHING records it"* |
| 8 | `BUNDLED_FORMS_PER_YEAR` 2024 → 16 | `the_cross_product_matrix…` | *"measured {2017: 5, 2024: 17, 2025: 15}, recorded {… 2024: 16 …} — a form gained or LOST its bundled asset"* |
| 9 | drop `(2017, "f8275")` from `KNOWN_ALIASES` | `the_cross_product_matrix…` | *"bundles NO asset of its own, yet the crate resolves it — another year's map, restamped"* |

Plants 4 and 5 are the ones that matter for B1's real question — *which test reds when the checker is
removed?* — because they attack the **measurement**, not the record. Plant 4 proves a blind join fails
loudly instead of silently reporting every asset as unprovenanced; plant 5 proves the join
distinguishes cell from cell.

`the_gate_reds_on_every_planted_defect` additionally holds 16 synthetic plants over the three verdict
functions (unrecorded gap, widened excuse, closed-but-still-recorded, misspelt obligation name,
unrecorded alias, stale alias, gap row naming an unbundled cell, count up, count down, and the
record-is-the-only-way-through cases).

**Independent cross-check.** Before writing any Rust I derived the same matrix twice in Python — once
joining through `MANIFEST.json`, once through the `.pdf.txt` notes. Both produced the identical 21
gapped cells, and the Rust reports the same 21 / 54. The numbers in `KNOWN_GAPS` are measured three
ways, not copied from the test's own output.

---

## 4. Found but NOT fixed

1. **TY2017's missing authority chain (25 obligations).** Fixing it is an archive task — fetch the five
   TY2017 PDFs from `irs-prior`, hash them against the bundled bytes, write notes + manifest entries,
   extract the text layer, generate geometry, write five `[census]` sections. It is recorded per cell,
   not waved through. ★ Note the acquisition may *fail to reproduce*: if an `irs-prior` PDF no longer
   hashes to the bytes we bundle, that is a finding, not a download error.
2. **`(2024, f8283)` is outside the archive.** Its bundled Rev. 12-2023 asset matches **no** committed
   note and no `MANIFEST.json` sha, yet `design/forms/extract/f8283--2024.txt` (13,174 bytes) exists
   and is read by conformance assertions. `design/forms/README.md` explains the intent (an extract
   taken from the bundled runtime asset because the IRS holds no `f8283--2024.pdf`) — but **nothing
   binds that extract to the bytes we ship**, so an asset swap would leave every f8283 citation
   pointing at text from a different revision. It is the one TY2024 cell with no provenance.
3. **TY2025: 10 of 15 committed forms are unreachable** — `f1040s1a`, `f1040s2`, `f1040s3`, `f1040sa`,
   `f1040sb`, `f1040sc`, `f6251`, `f8959`, `f8960`, `f8995`: asset and map committed and 100 % censused,
   never `include_bytes!`'d, `for_year(2025)` refusing. This is the port report's *"REFUSES — wrongly"*
   row, now pinned cell by cell, so wiring one of them turns the record red until it is deleted.
4. **`f1040s1a` has no map type at all.** `forms/2025/f1040s1a.map.toml` is committed and censused, and
   `map.rs` has no `Schedule1AMap` — there is no type to give a `for_year` arm to. Recorded in
   `STEMS_WITH_NO_MAP_TYPE` rather than skipped. This is Schedule 1-A, the form the TY2026 work list
   measures as most changed.
5. **The `UnsupportedYear` literal in `error.rs:11` is still a literal** (*"2017, 2024 and 2025"*). I
   did not edit it — it is another agent's file — but it is now **bound**: adding 2026 to
   `SUPPORTED_YEARS` reds `the_unsupported_year_refusal_names_exactly_the_supported_years` until the
   sentence is fixed. The test reads the rendered `Display` string, so it holds whether the message
   stays a literal or becomes derived.
6. **Four year-sets still govern four entry points** (`BundledTaxTables` {2017,2024,2025,2026}, this
   constant {2017,2024,2025}, maps on disk, `full_return_for` {2024}). This file binds one of them to
   its own assets and to its own refusal message; it does **not** reconcile the four. That is the port
   report's §2.5 finding and is out of scope here.

## 5. Scope notes

- No `git` was run. No subagents were spawned. Only the two owned files were modified; the plants that
  touched them were reverted and `diff`-verified byte-identical.
- Only scoped runs: `cargo nextest run -p btctax-forms --test supported_years_cross_product`. The rest
  of the workspace suite was not run.
- `rustfmt --edition 2021 --check` is clean on both files.
