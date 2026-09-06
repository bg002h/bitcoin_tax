# scaffolding fix — `DEFAULT_ROW_YEAR` bound 279 of 329 coverage rows to TY2024

**Agent:** scafffix-coverage · **Date:** 2026-09-05 · **File owned:**
`crates/btctax-core/src/tax/line_coverage.rs`
**Status:** FIXED in the file I own, with the residual (and larger) part measured and left for a
decision — see §5.

---

## 1. What was wrong — measured, not inferred

`line_coverage.rs` is the census that makes *"transcribe, never paraphrase"* enforceable: every
printed money line carries the form's own sentence, and `xtask line-coverage` resolves the authority
for that sentence as

```rust
// crates/xtask/src/line_coverage_check.rs:560
let stem = format!("{}--{}", e.form, e.year);   // → design/forms/extract/<form>--<year>.txt
```

so a row's `year` decides **which booklet the quote is checked against**.

| measurement | value | how |
|---|---|---|
| rows in the committed table | **329** | `xtask line-coverage` (`f1040:45 f1040s1:10 f1040s1a:50 f1040s2:6 f1040s3:5 f1040sa:19 f1040sb:5 f1040sc:7 f1040sd:19 f1040sse:22 f6251:41 f8949:12 f8959:17 f8960:15 f8995:16 f8995a:39 i1040gi:1`) |
| rows carrying `DEFAULT_ROW_YEAR` = `"2024"` | **279** | dumped every row of `all()` to TSV from a temporary test, counted by year: 279 × 2024, 50 × 2025 |
| `cover_*` collectors in the file | **24** | `grep -c 'pub fn cover_'` |
| collectors that **named** a year | **1** (`cover_schedule1a`, via `quoting_year("2025")`) | `grep -n 'quoting_year'` |
| collectors that inherited 2024 **by silence** | **23** | the other 23 `Coverage::default()` sites |

(The brief's 273/323 was a slightly earlier snapshot of the same table; the shape is identical.)

**Why that is the scaffolding defect and not a style point.** The product fails closed on an
unenumerated year, but this instrument does not follow the year at all: 279 rows re-assert the TY2024
booklet no matter which tax year is being ported, and the run prints `line-coverage OK`.

**It is already wrong for a year we ship.** btctax emits TY2025 maps for 13 forms
(`crates/btctax-forms/forms/2025/`). Of the 279 defaulted rows, 230 are on forms that have a
committed `--2025` extract. Checking each defaulted quote against **its own form's TY2025 text**,
using the checker's own normalisation (whitespace collapse + lone `{`/`}` dropped):

| result | rows |
|---|---|
| quote present verbatim in the TY2025 extract | 195 |
| **quote ABSENT from the TY2025 extract** | **34** |
| empty quote (the ratcheted `(none)` row, `SeTaxResult.addl`) | 1 |
| form has no `--2025` extract (`f8995a` 39, `f1040s1` 10) | 49 |

The 34 by form: `f1040sd` 13, `f1040` 9, `f6251` 6, `f1040sa` 4, `f1040sse` 1, `f1040s2` 1 —
e.g. Form 6251's four indexed thresholds (`$232,600` / `$94,050` / `$518,900` / `$875,950`),
1040 line 26's *"**2024** estimated tax payments and amount applied from **2023** return"*, and
Schedule D's nine `1a/3/8a/10` column totals. Nothing reds, because no row ever asks the TY2025
file a question.

---

## 2. What I changed

All in `crates/btctax-core/src/tax/line_coverage.rs`. **No row's year value changed**, so the
checker's output is byte-for-byte what it was (`329 money lines … 24 exception(s) (ratchet 24),
0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)`) before and after.

1. **`Coverage::quoting(year)` — the year is now a constructor argument**, and all 24 `cover_*`
   collectors name theirs (23 × `"2024"`, `cover_schedule1a` × `"2025"`). `cover_schedule1a`'s
   now-redundant `quoting_year("2025")` call is gone; the setter itself stays, because a collector
   spanning two extracts mid-way is legitimate and its B1 test still holds it.
2. **The collector's year became a decision, not a string**: private `enum RowYear { Decided(&str),
   Undecided }`, plus a sticky `bool` recording *"a row was pushed before any year was decided"*.
   `quoting_year` re-dates the rows that follow it and can never repair the ones already pushed, so
   a collector that ends `Decided` is **not** a collector whose rows were all decided — the mark is
   what tells those two apart.
3. **`fn dated(c: Coverage) -> Vec<LineCoverage>` is the only path into the table**, and it *panics*
   on either state, naming the form and the row count. `all()` now builds a bare `Vec` and takes
   every form through it — including the three **nested** collectors (`cover_form6251line1`, and
   `cover_schedulebrow` × 2), whose rows arrive pre-stamped and would otherwise have passed the gate
   invisibly.
4. `Default` survives **only** as scaffolding for `xtask`'s synthetic single-row test collectors
   (31 sites in a file I do not own); it no longer names a year, and a `cover_*` that uses it cannot
   reach the table. `DEFAULT_ROW_YEAR` survives for the same reason, re-documented with the
   measurement above.
5. Three new tests (§3), the two existing ones updated to the new constructor.

**The point of the fix, in one sentence:** naming a year sends every one of that form's quotes at
that year's extract, so a sentence carried forward from the old booklet **reds** — which is exactly
what the port needs and what a defaulted year cannot do.

---

## 3. Which test reds for which planted defect

Every plant below was applied to the committed code, run, and observed. Restores were `cp` from a
backup (never `git checkout`), and the final file was verified identical to the pre-plant copy.

| # | planted defect | RED test | observed failure |
|---|---|---|---|
| 1 | `cover_form6251` reverts to `Coverage::default()` (the exact 23-collector defect) | `every_row_in_the_table_names_the_extract_it_is_quoted_from` | panic: *"the collector for `"f6251"` (41 row(s)) never named the year its quotes are read from…"* |
| 1b | same, on `cover_schedulealines` | same | panic naming `"f1040sa"` (19 rows) |
| 2 | delete the sticky mark `self.2 \|= self.1 == RowYear::Undecided;` in `line()` | `a_row_pushed_before_the_year_was_decided_is_refused` | FAILED — a push-then-decide collector is no longer refused |
| 3 | delete the `RowYear::Undecided` assert in `dated()` | `a_collector_that_never_named_its_year_is_refused` | FAILED (the surviving taint assert fires with the other message, so the `should_panic(expected=…)` no longer matches) |
| 4 | **nested** collector `cover_form6251line1` reverts to `Coverage::default()` | `every_row_in_the_table_names_the_extract_it_is_quoted_from` | panic naming `"f6251"` (1 row) — this is the hole closed by routing nested extends through `dated` |
| 5 | **the product-level plant:** `cover_form6251` declares `quoting("2025")` while keeping its TY2024 sentences | `xtask line-coverage` (rebuilt, run, restored) | `line-coverage FAILED (5 problem(s))` — f6251 lines 4, 18, 19, 25, 39 *"quotes text NOT FOUND in f6251--2025.txt"*. (The 6th divergence, line 1, lives in `cover_form6251line1`, whose year the plant did not touch.) |

Plant 5 is the one that matters: it is the direct evidence that the year is now load-bearing at the
checker rather than decorative, and it doubles as a live measurement of the TY2024→TY2025 delta.

Green after restore: `cargo nextest run -p btctax-core -E 'test(/line_coverage/) or
test(/schedule_1a/)'` → 31 passed; `cargo nextest run -p xtask -E 'test(/line_coverage/)'` → 4
passed; `cargo clippy -p btctax-core --all-targets` → no warnings; `rustfmt --check` clean;
`xtask line-coverage` and `xtask cite-check` OK.

---

## 4. Exact edits needed in files I do not own

Neither is required for green — the tree builds and every gate passes as committed.

**(a) `crates/xtask/src/line_coverage_check.rs` — to delete the default outright.** 33 mechanical
edits, all inside `#[cfg(test)] mod tests`:

```
sed -i 's/Coverage::default()/Coverage::quoting("2024")/g' crates/xtask/src/line_coverage_check.rs   # 31 sites
# :1382 and :1399 —  year: btctax_core::tax::line_coverage::DEFAULT_ROW_YEAR,  →  year: "2024",
```

Then, in `line_coverage.rs`, the whole runtime apparatus can go: delete `impl Default for Coverage`,
`DEFAULT_ROW_YEAR`, `enum RowYear`, the `.2` field and both asserts in `dated()`, making
`Coverage(pub Vec<LineCoverage>, &'static str)` with `quoting()` as its **only** constructor. The
guarantee becomes *"an omission does not compile"* and the two `should_panic` tests are retired with
it. **I did not do this because it breaks `xtask`'s build for the seven agents sharing this
worktree** — the runtime gate exists precisely and only because `Default` had to survive.

**(b) `crates/btctax-core/src/tax/tables.rs:2040` — a comment gone stale (comment only, no code).**

```
// The rows are quoted from the 2025 extract, not 2024 — the `quoting_year` call is what makes
```
→ …the `Coverage::quoting("2025")` **constructor** is what makes that true. The test's assertion
(`r.year == "2025" && r.form == "f1040s1a"`) is unaffected and passes.

---

## 5. What I did NOT fix — and what it needs

**★ The row set still has no per-`(form, year)` concept, and that is the larger half of this
finding.** `all()` takes no year; each `cover_*` transcribes exactly one booklet. Making the year
*decided* means a wrong year now reds — it does not mean the table covers the years we ship.

Concretely, and measured: `crates/btctax-forms/forms/2025/` holds **15** maps. One (`f1040s1a`) is
covered at its own year; one (`f8283`) has no coverage rows in any year; the other **13 form-years we
emit are covered only by rows that quote the TY2024 booklet** — `f1040`, `f1040s2`, `f1040s3`,
`f1040sa`, `f1040sb`, `f1040sc`, `f6251`, `f8949`, `f8959`, `f8960`, `f8995`, `schedule_d`
(`f1040sd`), `schedule_se` (`f1040sse`). `xtask line-coverage` validates the rows that
exist and has no notion of a `(form, year)` pair that *should* exist, so its OK line is silent about
them. That is the same "skipping is not passing" shape as the 32 silent-fallback sites in the port
report, one level up.

Closing it is a design decision, not an edit:

- **(i) Second transcription per form-year.** 195 of the 230 checkable defaulted quotes are verbatim
  in the TY2025 text, so the *marginal* work for TY2025 is the 34 that are not — but the table
  doubles (≈230 new rows), and the exception / not-line-bound ratchets (`24` and `12`, both in
  `line_coverage_check.rs`) move with it. TY2026 has **no** extracts on disk, so its rows cannot be
  written yet at all.
- **(ii) `cover_*(l, year)` with a `match year` per line.** Keeps one row per line per year and makes
  a missing year a compile error, at the cost of a `match` on every one of the 329 sites.
- **(iii) Declare the scope and enforce it.** Keep coverage at one transcribed year per form, and
  make `xtask line-coverage` derive the emitted inventory from
  `crates/btctax-forms/forms/<year>/<form>.map.toml` and **fail** on any emitted `(form, year)` that
  is neither covered nor recorded-with-a-reason. This is the cheapest instrument that stops the
  silence, and it is the same derived `(form, year)` inventory the port report's **I-5 / R16** already
  wants for `EMITTED_FORMS`. It belongs in `xtask`, which can read the tree; `btctax-core` cannot.

My recommendation is **(iii) first** — it converts today's 13 uncovered form-years from invisible to
red — then (i) for whichever year we actually file.

Two smaller things left alone, both pre-existing and both already ratcheted by the checker: the one
empty-instruction `(none)` row (`SeTaxResult.addl`), and `f8995a` / `f1040s1` having no `--2025`
extract (consistent with `f8995a` being TY2024-only and Schedule 1 being unwired for TY2025).
