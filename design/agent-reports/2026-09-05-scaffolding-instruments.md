# Year-pinned instruments — which gates go quiet when the target moves to TY2026

**Lens:** year-pinned instruments. **Date:** 2026-09-05. **Rule:** read-only; no `git`, `cargo`,
`make` or `nextest` was run (five agents share this worktree). Every number below is a measurement
from `grep`/`awk`/`python3` over the committed tree, pasted, not estimated.

---

## Headline

**The instruments that survive a year change are exactly the ones that get their year from a
directory listing; every other gate takes it from a literal, a `const`, or a default argument — and
the three worst do not skip the new year, they silently re-check TY2024 and report success.** The
biggest is `DEFAULT_ROW_YEAR = "2024"`, which binds **273 of 323** line-coverage rows — the direct
enforcement of "transcribe the form, never paraphrase it" — to a TY2024 text layer with no call site
that has to say so.

Not restated: R15 (`field_census.rs` year pin), R16 (`EMITTED_FORMS` omits `f8995a`), R21
(`ots_direct.version()`), R26 (`verify_schedule_1a._rows`) are in `TY2026_PORT_REPORT.md` and stand.
I-4 and I-5 below generalise R16 into a different defect; the rest are new.

---

## Findings

### I-1 — CRITICAL. `DEFAULT_ROW_YEAR` silently binds 273 of 323 coverage rows to a TY2024 extract

`crates/btctax-core/src/tax/line_coverage.rs:55`

```rust
pub const DEFAULT_ROW_YEAR: &str = "2024";
```

`crates/xtask/src/line_coverage_check.rs:560` resolves each row's authority from it:

```rust
let stem = format!("{}--{}", e.form, e.year);
```

Measured, over `crates/btctax-core/src/tax/line_coverage.rs`:

```
non-test `quoting_year(` calls:            1   (line 2993: c.quoting_year("2025"))
c.line(/c.exception( pushes, lines 1-2984: 273
c.line(/c.exception( pushes, 2985-3314:     50   (cover_schedule1a, the TY2025 rows)
c.line(/c.exception( pushes, 3315-EOF:       3   (test fixtures)
```

So **exactly one** production call site names a year, and it names 2025. Every other row's quoted
instruction is verified against `design/forms/extract/{form}--2024.txt`. A TY2026 port that adds
lines and moves quotes — Form 1040 moves 31 printed line bindings and Form 6251 moves 28
(`design/TY2026_WORK_LIST.md`) — passes this check against the TY2024 booklet unless someone
remembers a call that nothing forces.

The path already treats the year as load-bearing; only the *default* is wrong. Note that the same
file's emitted-form test **already derives** from the filesystem
(`line_coverage_check.rs:576-586`: a committed `crates/btctax-forms/forms/{year}/{form}.map.toml`
means "we emit it"). The year axis was simply never given the same treatment.

**Fix:** delete the default — make `year` a required argument to `line()`/`exception()`. The compiler
then names all 323 sites. **MECHANICAL.**

---

### I-2 — IMPORTANT. The label-join check `eprintln!`s its own blind spots, and its floor is global

`crates/xtask/src/label_reader.rs:1033` `every_mapped_line_lands_on_its_own_printed_label` is the
instrument built for R3 — the only one that can catch a renumber. Two structural gaps:

* `:1071-1079` — forms with no geometry fixture go into `unwitnessed` and are reported with
  `eprintln!`, never asserted. nextest hides that on a pass.
* `:1061` — `assert!(checked >= 151, …)` is **one global count across all years**. 2024+2025 already
  satisfy it, so a TY2026 whose geometry was never generated contributes 0 joins and the gate is
  still green.

Measured today — 10 of 37 committed maps are unwitnessed:

```
UNWITNESSED f1040--2017      UNWITNESSED f8283--2024
UNWITNESSED f8283--2017      UNWITNESSED schedule_d--2024
UNWITNESSED f8949--2017      UNWITNESSED schedule_se--2024
UNWITNESSED schedule_d--2017 UNWITNESSED schedule_d--2025
UNWITNESSED schedule_se--2017 UNWITNESSED schedule_se--2025
```

**Four of those have a geometry fixture on disk that the stem builder cannot name.**
`every_map()` at `:1013` builds the stem from the *map* filename:

```rust
let stem = format!("{form}--{y}");
```

giving `schedule_d--2024`, while the committed fixture is
`design/forms/geometry/f1040sd--2024.json`. All four exist:

```
design/forms/geometry/f1040sd--2024.json   f1040sse--2024.json
design/forms/geometry/f1040sd--2025.json   f1040sse--2025.json
```

Counting bindings exactly as `line_bindings()` does (bare `lineN = "FQN"`, comments skipped):

```
2024/schedule_d.map.toml:   3      2024/schedule_se.map.toml:  12
2025/schedule_d.map.toml:   3      2025/schedule_se.map.toml:  12   → 30 joins unreachable
```

**Schedule D is where btctax's capital gain lands**, and no committed test joins any of its lines to
its printed label, in any year, purely because two directories spell the same form differently.

For TY2026 the fixtures are archived as `f6251--2026-DRAFT.json` etc., so *every* 2026 map will land
in the eprintln'd list on the day `forms/2026/` is created.

**Fix:** (a) one committed map-stem ⟷ IRS-stem alias table — **MECHANICAL**; (b) assert
`unwitnessed.is_empty()`, or make the floor per-year — **MECHANICAL**; (c) TY2026 final geometry —
**HUMAN** (needs the final PDF, not the draft).

---

### I-3 — IMPORTANT. "The validated artifact must be the shipped artifact" covers only TY2024, while four years ship

`crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs` — all three tests pin one
year:

```
:47  fn the_shipped_ty2024_table_equals_the_one_every_test_validates()
:50      .table_for(2024)
:211     let shipped = shipped_all.table_for(2024).unwrap();
:257 fn the_shipped_ty2024_params_equal_the_ones_every_test_validates()
:260     .full_return_for(2024)
```

`crates/btctax-adapters/src/tax_tables.rs:73-79` ships four:

```rust
by_year.insert(2017, ty2017());
by_year.insert(2024, ty2024());
by_year.insert(2025, ty2025());
by_year.insert(2026, ty2026());
```

Measured: **260** references to `ty2024_params`/`ty2024_table` across `crates/`; **zero** to any
`ty2025_*`/`ty2026_*` equivalent. `crates/btctax-core/src/tax/testonly.rs` defines exactly two:
`ty2024_params` (`:53`) and `ty2024_table` (`:111`).

There is therefore no validated counterpart for the TY2025 and TY2026 tables the binary loads. The
file's own doc comment names the class it exists to close — *"an assurance surface that does not touch
the artifact it is claimed to assure"* — and that sentence is now true of this file for two of the
four shipped years. Its `★★ Direction matters here` paragraph applies verbatim: there is no oracle
for this, because both engines are handed whatever table we give them.

**Fix:** the equality test itself becomes year-general by iterating `by_year.keys()` —
**MECHANICAL**. But a second, independently transcribed TY2026 table has to exist first, or the
equality is vacuous: **HUMAN** (Rev. Proc. 2025-32 + OBBBA Pub. L. 119-21).

---

### I-4 — IMPORTANT. The authority ratchet is keyed on *form*, not *(form, year)*

`crates/xtask/src/cite_check.rs`. `FormAuthority` carries a year at `:654`:

```rust
pub year: i32,
```

and `authority_coverage_may_only_improve` at `:756` throws it away:

```rust
let archived: BTreeSet<&str> = FORMS
    .iter()
    .filter(|f| !f.extract_stem.is_empty())
    .map(|f| f.form)
    .collect();
```

`FORMS` (`:665-672`) has exactly one entry: `f1040s1a`, `year: 2025`, extract
`schedule_1a_2025`.

`design/TY2026_WORK_LIST.md` measures Schedule 1-A as **REBUILT** — 10 fields survive out of 219, 175
added, 44 removed. After a year bump the ratchet still reads "f1040s1a is archived", stays green, and
`cite-check`'s quotation test keeps verifying the spec and plan against the TY2025 booklet. The
ratchet's own doc comment (`:682-690`) promises it "may only SHRINK"; it cannot express *"we hold the
2025 authority and not the 2026 one"* at all.

**Fix:** key `archived`, `excused` and the emitted side on `(form, year)`. **MECHANICAL** for the
keying; **HUMAN** for archiving each TY2026 authority pair.

---

### I-5 — IMPORTANT. Three hand-lists of "the forms btctax emits", two naming conventions, nothing joining them

| authority | file:line | spelling | contents |
|---|---|---|---|
| `CENSUS_KEYS` | `crates/btctax-forms/tests/common/mod.rs:16` | map (`schedule_d`, `schedule_se`) | 17, incl. `f8995a`, **no** `f1040s1a` |
| `EMITTED_FORMS` | `crates/xtask/src/cite_check.rs:678` | IRS (`f1040sd`, `f1040sse`) | 16, incl. `f1040s1a`, **no** `f8995a` |
| `SUPPORTED_YEARS` | `crates/btctax-forms/src/lib.rs:68` | — | `&[2017, 2024, 2025]` |

`CENSUS_KEYS`'s doc calls itself *"THE ONE AUTHORITY for the set of forms `fill_full_return` can
emit"* — and it is, for one of the two seams. `EMITTED_FORMS`'s doc says it is *"derived from
`btctax-forms`' modules"*; it is hand-typed, and it has already drifted (R16: `f8995a` is emitted at
`crates/btctax-forms/src/packet.rs:194`).

Neither form list carries a year. Neither is joined to the other, or to `crates/btctax-forms/forms/`
on disk. The naming split is *known* and used as a test fixture rather than fixed —
`crates/xtask/src/line_coverage_check.rs:1328`:

> `schedule_d` is the MAP's name; the extract is committed as `f1040sd`. So a row naming it has a map
> and no text layer — which is exactly `unverifiable`…

This is the same field-of-view shape as `CLAUDE.md` §B3: every copy is individually correct and
nothing holds them together. It is also the direct cause of I-2's 30 missing joins.

**Fix:** one derived `(form, year)` inventory built by walking `crates/btctax-forms/forms/` plus a
committed alias table, consumed by all three sites. **MECHANICAL.**

---

### I-6 — IMPORTANT. `Form8275Map::for_year` substitutes another year's map by design, guarded by an enumerated year list

`crates/btctax-forms/src/map.rs:1005-1014`:

```rust
pub fn for_year(year: i32) -> Result<Self, FormsError> {
    match year {
        2017 | 2024 | 2025 => {
            let mut m = Self::ty2024();
            m.year = year;
            Ok(m)
        }
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}
```

Measured accepted-year sets across all 16 map constructors in that file:

```
Form6251Map: 2024        Form8949Map:  2017 2024 2025
Form8959Map: 2024        Form1040Map:  2017 2024 2025
Form8960Map: 2024        Form8283Map:  2017 2024 2025
Form8995AMap: 2024       ScheduleDMap: 2017 2024 2025
Form8995Map: 2024        ScheduleSeMap:2017 2024 2025
Schedule1Map: 2024       Schedule2Map: 2024
Schedule3Map: 2024       ScheduleAMap: 2024
ScheduleBMap: 2024       ScheduleCMap: 2024
Form8275Map:  2017 | 2024 | 2025  ← the only ALIASING arm, not a refusal
```

Fifteen refuse. Form 8275 is a non-year-dated form (Rev. 01-2021), so the aliasing is defensible —
but the *guard is a year list*, so adding `| 2026` is a one-token edit and no instrument contradicts
it. There is no TY2026 evidence to contradict it with: per `design/TY2026_WORK_LIST.md`,
`scripts/archive_drafts.py` **refused** the f8275 draft because the IRS URL served a 2024 document.

Worse, the test that reads as covering this is blind. `crates/btctax-forms/tests/sp4.rs:394-403`:

```rust
fn map_year_matches_bundled_pdf_fieldset_for_every_supported_year() {
    for &year in btctax_forms::SUPPORTED_YEARS {
        let map = Form8275Map::for_year(year).unwrap();
        assert_eq!(map.year, year);
        let set = fieldset(F8275_PDF_2024);       // ← one asset, every year
```

It iterates the year set correctly and then compares every year against the **TY2024 PDF**. A year
whose Form 8275 revision changed cannot be seen.

**Fix:** gate the alias on a committed `pdf_sha256` rather than a year list. **MECHANICAL** to build;
**HUMAN** to confirm the TY2026 revision when it posts.

---

### I-7 — IMPORTANT. The golden corpus labels its own tax year and nothing reads the label

`crates/btctax-core/tests/goldens/full_return_goldens.json` — 106 households, and:

```json
"tax_year": 2024,
"oracle_1_version": "OpenTaxSolver 2024 (OpenTaxSolver2024_22.07_linux64)",
"generated": "2026-08-22"
```

Grep for `tax_year` / `_provenance` in `golden_returns.rs`, `golden_packet.rs` and
`oracle_sweep_readback.rs`: **no code reads either.** All hits are prose in doc comments. The
corpus's own year label is inert, while `golden_returns.rs:38` imports `ty2024_params, ty2024_table`
as the thing it compares against.

The generator carries the year as a **Python default argument** in three places —
`scripts/oracle/gen_goldens.py:199` `_taxcalc_row(n, i, year: int = 2024)`, `:232`
`taxcalc_run(households, year: int = 2024)`, `:321` `_taxcalc_amt_credits(inputs_list, year: int =
2024)` — and writes `"tax_year": 2024` as a literal at `:518`.
`scripts/oracle/ots_direct.py:79` does the same by environment: `OTS_YEAR =
int(os.environ.get("OTS_YEAR", "2024"))`.

So the file whose own doc comment calls it *"the most important test in the repo"* is a TY2024
instrument with no way to say so to the code that consumes it, driven by a generator that defaults to
TY2024 at every call.

`scripts/oracle/sweep.py` has no year axis at all: `:89 OASDI_BASE = 168_600`, `:92 STD_DEDUCTION =
corpus.STD_DEDUCTION_2024`, `:96` the TY2024 §199A ceiling — module-level constants.

**The model to copy is already in the same directory.** `scripts/oracle/corpus.py:123-132`:

```python
def salt_for(year: int) -> dict:
    """The SALT axis for `year`, asserted to actually straddle that year's §164(b) cap."""
    try:
        axis, cap = SALT_BY_YEAR[year], SALT_CAP_BY_YEAR[year]
    except KeyError:
        raise KeyError(f"no SALT axis for TY{year}; add it with that year's §164(b) cap") from None
    ...
    assert under < cap < over, (
```

That refuses an unprepared year **and** asserts the axis still discriminates at that year's
constants, so a cell cannot go silently inert. `verify_f6251.py:75-84 _standard_deduction` raises a
named `KeyError` the same way.

**Fix:** delete the three default arguments and assert `_provenance.tax_year` equals the year the
Rust test computes against — **MECHANICAL**. Regenerating for TY2026 is **HUMAN**, and blocked:
R23 records that installed taxcalc 6.7.2 understates TY2026 AMTI by the whole standard deduction.

---

### I-8 — IMPORTANT. `prompt-check`'s eight filer-facing wording assertions are pinned to TY2025 by literal path

`crates/xtask/src/prompt_check.rs` — every `Clause.extract` is a hardcoded string:

```
:56, :62, :68, :74, :80   "design/forms/extract/i1040gi--2025.txt"
:86, :92, :99             "design/forms/extract/i8615--2025.txt"
```

These are the strings the TUI shows a filer immediately before an answer becomes sworn testimony.
When the TY2026 booklet lands, the check keeps verifying them against TY2025 wording and prints
`xtask prompt-check: OK — {passed} assertions, all verbatim` (`:223`).

**Fix:** parameterise the extract path by target year — **MECHANICAL**; re-adjudicate all eight
clauses against the TY2026 booklet — **HUMAN**.

---

### I-9 — IMPORTANT. The best-designed conformance census in the repo is pinned to the one form that was rebuilt

`crates/xtask/src/schedule_1a_membership.rs:134` and `:157`:

```rust
let printed = printed_labels("f1040s1a--2025").expect("Schedule 1-A geometry");
```

Its design is exactly right — the expected set is not a range and not a hand-list, it is the
adjudication of two witnesses (the printed margin column and the AcroForm geometry) resolving 50
printed labels into 48 entry lines and 2 headings, with the actual set tied to `Schedule1A::leaves`
by an exhaustive destructure.

But `Schedule1A` is one struct with the TY2025 numbering baked in (port report R2), and the TY2026
draft moves Part IV to 28-36 and Part V to 37-43. After a year bump this census keeps adjudicating 48
TY2025 lines against a TY2025 fixture and stays green while the emitted form is wrong.

**Fix:** the struct has to become year-shaped before the census can be. **HUMAN.** Once it is, the
year selection is a parameter.

---

### I-10 — MINOR. Hand-listed year loops across the KAT surface

| site | pin |
|---|---|
| `crates/btctax-forms/tests/kats.rs:391, 452, 474, 487` | `for year in [2017, 2024, 2025]` ×4 |
| `crates/btctax-forms/tests/kats.rs:382-389` | on-state expectation with a **wildcard year arm**: `(_, true) => "1"`, `(_, false) => "2"` |
| `crates/btctax-forms/tests/sp3b.rs:127` | `for year in [2017, 2024]` |
| `crates/btctax-forms/tests/full_return_forms.rs:523` | `for year in [2017, 2023, 2025]` — a *refusal* pin, enumerated rather than derived |
| `crates/btctax-forms/tests/f6251_map.rs:15-16` | `include_str!("../forms/2024/f6251.map.toml")` + `include_str!(".../f6251--2024.txt")` |
| `crates/btctax-forms/tests/f8995a_map.rs:9-10` | same shape, TY2024 only |
| `crates/btctax-forms/tests/field_census_slice.rs:65-70, 116` | the whole slice is `ty2024_params/table` + literal `2024` |
| `crates/xtask/src/examples.rs` | every J1-J6 journey's argv hardcodes `--tax-year 2024`/`2025`; the goldens are byte-pinned |

The 6251 pair matters most: `design/TY2026_WORK_LIST.md` measures f6251 as keeping **all 62 field
names** while moving **28 printed line bindings**, and its transcription-conformance test is TY2024
only.

The wildcard in `kats.rs` is the same shape as the two `_ =>` year arms fixed today in
`f1040_clusters`/`se_clusters`, but in a test: a revision that changed the Schedule D Yes/No
on-states would be asserted against `"1"`/`"2"` by default — except that 2026 is not in any of the
four loops, so it is never asked.

---

## What already derives its year, and what makes the difference

The dividing line is not test-vs-xtask, and it is not care — several of the pinned instruments above
are among the most carefully written files in the repo. It is **whether the instrument's year comes
from an artifact it can enumerate.**

Year-general today, and how:

* `crates/btctax-forms/tests/map_pdf_conformance.rs:22-50` — `every_committed_map()` does
  `read_dir(forms/)` and takes the year from the directory name.
* `crates/xtask/src/label_reader.rs:988-1015` — `every_map()`, the same walk. (Its *stem* is still
  wrong; see I-2. The year axis is right.)
* `crates/xtask/src/line_coverage_check.rs:576-586` — derives "do we emit this form?" from
  `crates/btctax-forms/forms/{year}/{form}.map.toml` existing, rather than a list.
* `crates/btctax-forms/tests/census.rs:421` — `SUPPORTED_YEARS.iter().max()` as "newest filable
  year", coupling the era preset to it. This is the model hardcode: it **reds the moment
  `SUPPORTED_YEARS` gains 2026.**
* `scripts/oracle/corpus.py:123-132` — refuses an unknown year *and* asserts the axis still
  straddles that year's cap.
* `crates/btctax-forms/tests/map_pdf_conformance.rs:186-200` — `a_year_with_no_bundled_6251_map_
  refuses_instead_of_reusing_another_years_geometry` hardcodes `[2023, 2025, 2026]`, which is a
  **tripwire** pin: wiring a 2026 map turns it red rather than quiet. That is the good direction for
  an enumerated year list.

And the asymmetry that makes the rest dangerous: `field_census.rs:105`'s `let year = 2024;` and
`DEFAULT_ROW_YEAR` do not *skip* the new year. They check the old year again, find it correct, and
report success — `CLAUDE.md`'s dominant defect shape with the year as its axis.

**The keystone the year package is missing** is a single derived `(year, form)` inventory: one
function walking `crates/btctax-forms/forms/` and `design/forms/`, applying a committed map-stem ⟷
IRS-stem alias table, returning the set — with every site above taking its year set from it and
asserting the set is non-empty for the target year. The B1 kill test writes itself: create
`forms/2026/` holding one map, and assert each instrument's checked-count rises.

---

## MECHANICAL — a machine can do it

1. **Delete `DEFAULT_ROW_YEAR`**; make `year` a required parameter of `Coverage::line`/`exception`.
   The compiler names all 323 call sites. (I-1)
2. **Commit a map-stem ⟷ IRS-stem alias table** (`schedule_d` ⟷ `f1040sd`, `schedule_se` ⟷
   `f1040sse`) and use it in `label_reader::every_map()`. Recovers 30 line→label joins that exist
   today and are invisible. (I-2, I-5)
3. **Assert `unwitnessed.is_empty()`** in `every_mapped_line_lands_on_its_own_printed_label`, or make
   the `checked >= 151` floor per-year. Either kills the "whole year contributes zero joins" pass.
   (I-2)
4. **Iterate `BundledTaxTables::load().by_year.keys()`** in
   `shipped_tables_are_the_validated_tables.rs` instead of `table_for(2024)` — so a shipped year with
   no validated counterpart is a compile-or-test failure rather than a silence. (I-3)
5. **Key `cite_check`'s ratchet on `(form, year)`**, not `form`. (I-4)
6. **Build one derived `(form, year)` inventory** from `crates/btctax-forms/forms/` and retire
   `CENSUS_KEYS` / `EMITTED_FORMS` / the year half of `SUPPORTED_YEARS` onto it. Closes R16 by
   construction rather than by adding one string. (I-5)
7. **Replace `Form8275Map`'s year alias list with a `pdf_sha256` check** against the committed
   asset. (I-6)
8. **Delete the three `year: int = 2024` defaults** in `gen_goldens.py` and the `OTS_YEAR` env
   default; assert `_provenance.tax_year` equals the year `golden_returns.rs` computes. (I-7)
9. **Parameterise `prompt_check`'s `Clause.extract` paths by year.** (I-8)
10. **Replace the four `for year in [2017, 2024, 2025]` loops** in `kats.rs` with
    `btctax_forms::SUPPORTED_YEARS`, and delete the `(_, true) => "1"` wildcard in favour of a
    per-year on-state table that refuses an unlisted year. (I-10)
11. **Give `sweep.py` a year axis** (`OASDI_BASE`, `STD_DEDUCTION`, the §199A ceiling) on the
    `corpus.salt_for` pattern — refuse an unknown year, and assert each axis still discriminates at
    that year's constants. (I-7)
12. **Extend `field_census.rs`'s three year literals** (`:105`, `:193`, `:261`) to the derived year
    set — but see HUMAN 1: the 2025 census sections must exist first. (R15)

## HUMAN — someone must read a form

1. **Write the `[census]` sections for the five TY2025 maps that have none** — measured:
   `2025/f1040.map.toml`, `2025/f8283.map.toml`, `2025/f8949.map.toml`, `2025/schedule_d.map.toml`,
   `2025/schedule_se.map.toml` carry 0 `[census]` sections; the other 10 carry 1 each. Until these
   exist, making `field_census.rs` year-general turns a green gate red on missing work rather than on
   a defect, and R15's measured 330 unaccounted TY2025 fields is the size of the reading.
2. **Transcribe a TY2026 `testonly::ty2026_table()` / `ty2026_params()`** independently of the
   shipped table — from Rev. Proc. 2025-32 and OBBBA Pub. L. 119-21, not by copying
   `tax_tables.rs::ty2026()`. A copied table makes I-3's equality vacuous.
3. **Archive and extract each TY2026 authority pair** (`fNNNN` + `iNNNN`) so I-4's `(form, year)`
   ratchet has something to shrink onto. Blocked on IRS finals for f8275 and f8283, whose 2026 drafts
   the archiver correctly refused.
4. **Re-adjudicate `prompt_check`'s eight clauses** against the TY2026 `i1040gi` / `i8615`.
5. **Decide whether `Schedule1A` becomes year-shaped or year-parameterised** before the membership
   census can move off `f1040s1a--2025` — the TY2026 draft rebuilt the form (10 of 219 fields
   survive) and moved Part V to lines 37-43.
6. **Confirm the TY2026 Form 8275 revision** is still Rev. 01-2021 before any `| 2026` reaches
   `Form8275Map::for_year`.
7. **Regenerate the golden corpus for TY2026** — needs a taxcalc release whose TY2026 AMTI is not
   understated by the whole standard deduction (R23), and an OTS 2026 tree.
8. **Generate TY2026 geometry fixtures from the FINAL forms**, named `{stem}--2026.json`, so the
   label-join check can reach them. The `--2026-DRAFT` fixtures must not be renamed into that slot;
   drafts are evidence only.
