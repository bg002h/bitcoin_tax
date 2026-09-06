# What a YEAR PACKAGE is — the per-form checklist that turns "do TY2027" into a list

**Headline: a year package is 9 artifacts per form; TY2025 built 7 of them for 10 forms and 0 of the
last 2, and the two it skipped are the only two the compiler can see — so ten complete, audited,
census-carrying TY2025 maps are loaded by nothing and checked by a gate that is hardcoded to 2024.**

Agent report, 2026-09-05. Lens: *what a year package is*. No git, cargo, make or nextest was run;
everything below is `cat`/`grep`/`.venv/bin/python` over the working tree.

---

## 1. The nine slots, measured

Traced end to end on **Form 8959, TY2025** — the easiest possible port (`design/TY2026_WORK_LIST.md`
calls it byte-identical in both axes across the 2025→2026 boundary), so anything missing here is
missing structurally, not because the form was hard.

| # | slot | artifact for (f8959, 2025) | bytes | committed? |
|---|---|---|---|---|
| 1 | authority PDF | `design/forms/2025/f8959--2025.pdf` | 71 969 | **no** (gitignored) |
| 2 | provenance note | `design/forms/2025/f8959--2025.pdf.txt` | 736 | yes |
| 3 | manifest entry | `design/forms/MANIFEST.json` (31 entries for 2025) | — | yes |
| 4 | form text layer | `design/forms/extract/f8959--2025.txt` | 5 990 | yes |
| 4b | instructions text layer | `design/forms/extract/i8959--2025.txt` | 19 676 | yes |
| 5 | geometry fixture | `design/forms/geometry/f8959--2025.json` (1 063 words, 26 boxes) | 78 190 | yes |
| 6 | bundled template | `crates/btctax-forms/forms/2025/f8959.pdf` | 71 969 | yes |
| 7 | field map + census | `crates/btctax-forms/forms/2025/f8959.map.toml` (137 lines) | 10 616 | yes |
| 8 | **asset binding** | `include_bytes!` const + `f8959_pdf(2025)` arm in `pdf.rs` | — | **ABSENT** |
| 9 | **map binding** | `include_str!` const + `ty2025()` + `Form8959Map::for_year(2025)` arm | — | **ABSENT** |

TY2025 totals on disk: **31** notes (17 forms + 14 instructions), **31** extracts, **15** geometry
fixtures, **15** bundled templates, **15** maps, **31** manifest entries.

## FINDING 1 — 10 of 15 TY2025 templates+maps are orphans: on disk, referenced by no code

```
$ for f in crates/btctax-forms/forms/2025/*.pdf; do grep -q "2025/$(basename $f)" crates/btctax-forms/src/*.rs && echo WIRED $f || echo ORPHAN $f; done
WIRED   f1040.pdf      ORPHAN  f1040s1a.pdf   ORPHAN  f1040s2.pdf    ORPHAN  f1040s3.pdf
ORPHAN  f1040sa.pdf    ORPHAN  f1040sb.pdf    ORPHAN  f1040sc.pdf    ORPHAN  f6251.pdf
WIRED   f8283.pdf      WIRED   f8949.pdf      ORPHAN  f8959.pdf      ORPHAN  f8960.pdf
ORPHAN  f8995.pdf      WIRED   schedule_d.pdf WIRED   schedule_se.pdf
```

`crates/btctax-forms/src/pdf.rs:13-21` holds exactly five `2025/` `include_bytes!`; `map.rs:33-41`
exactly five `2025/` `include_str!`. Slots 1-7 are complete for all fifteen; slots 8-9 exist for five.

## FINDING 2 — the split is *exactly* inverse to census coverage: two generations of "a map"

Counting `[census]` sections in every committed map:

| year | wired maps | census sections | unwired maps | census sections |
|---|---|---|---|---|
| 2025 | f1040, f8283, f8949, schedule_d, schedule_se | **0, 0, 0, 0, 0** | the other ten | **1 each** |
| 2025 | (their sizes) | 10-46 lines | (their sizes) | 137-314 lines |
| 2017 | all five | **0** | — | — |
| 2024 | all seventeen | **1 each** | — | — |

Every TY2025 map that carries a field census is unwired; every wired TY2025 map carries none. The
five wired ones are crypto-slice maps that predate the census discipline (`f1040.map.toml` for 2025
is **10 lines**; the 2024 one is 293). So "TY2025 is partly done" is not a gradient — it is two
different definitions of the artifact, and the complete definition is the one not plugged in.

## FINDING 3 — the census gate is hardcoded to 2024, so the ten TY2025 censuses are checked by nothing

    crates/btctax-forms/tests/field_census.rs:105        let year = 2024;
    crates/btctax-forms/tests/field_census.rs:193            .join("2024")

`census_accounts_for_every_field` is documented as *"★★★ THE GATE"* and asserts
`(map FQNs) ∪ (census FQNs) == (the PDF's AcroForm FQNs)`. It runs that assertion on TY2024 only.
Add TY2026 and the gate re-checks 2024 and reports green — **this is the lens's own failure shape in
the instrument rather than in the product**: an unprepared year silently gets last year's answer,
and the answer is *"pass"*.

Contrast `crates/btctax-forms/tests/map_pdf_conformance.rs:21-40`, added after the TY2025 map audit,
which walks `crates/btctax-forms/forms/` on the filesystem and covers a new map the moment it is
committed. That file is the model; `field_census.rs` has not been converted to it.

## FINDING 4 — the `*Map` STRUCT is a per-YEAR artifact wearing a per-FORM name. This is the single thing that makes a year a code change.

TY2025 split Form 6251 line 1 into 1a/1b. Machine-diffing the struct against the committed map:

```
struct line-fields: 41
missing from 2025 toml (=> parse ERROR):                    ['line1']
2025 toml line keys the struct has no field for (silently discarded): ['line1a', 'line1b']
```

`crates/btctax-forms/src/map.rs:121  pub line1: MoneyCell,` versus
`crates/btctax-forms/forms/2025/f6251.map.toml:72,75  line1a = …, line1b = …`. There is no
`#[serde(deny_unknown_fields)]` anywhere in `map.rs`. This was IMPORTANT-1 of
`design/agent-reports/2026-09-05-ty2025-map-AUDIT.md` and is **still open**; the audit's other half
*was* folded — `crates/btctax-forms/src/packet.rs:181` now reads `Form6251Map::for_year(year)?`.

The general statement: a `.map.toml` is data, but its **key set is schema**, and the schema is Rust.
`f1040s1a` (219 fields, 10 surviving into TY2026) has no `Schedule1AMap` type at all. So for any form
whose line set changes, "add a year" is a code change by construction — and the failure is quiet in
one direction (unknown keys discarded) and loud only when a *required* key disappears.

## FINDING 5 — three text-layer conventions, and two of them share one filename shape

    design/forms/2025/f8959--2025.pdf.txt          736 bytes   URL + sha256 + bytes  (a PROVENANCE NOTE)
    design/forms/2026/f8959--2026-DRAFT.pdf.txt   9 494 bytes  "Caution: DRAFT—NOT FOR FILING…"  (a TEXT LAYER)

`scripts/archive_drafts.py:127` writes `pdftotext -layout <dest> <dest>.txt`, colliding with the
note convention `design/forms/README.md` defines. The consequence is measurable: all sixteen TY2026
manifest entries carry `"extract": ""`, because `authority_manifest.rs:543-544` resolves an extract
only at `design/forms/extract/<stem>.txt`. A year whose text layer exists reads as *not extracted*.
The third convention is `crates/btctax-core/src/tax/fixtures/{stem}_form.txt` (2 files, Schedule 1-A
only), which is what `cite_check` actually reads.

## FINDING 6 — nothing regenerates `design/forms/extract/`, and nothing hashes it

The 64 committed text layers are what every conformance instrument reads
(`line_coverage_check.rs:565`, `capital_loss_carryover_check.rs:31`, `prompt_check.rs:56-99`,
`label_reader.rs:648`). The only committed `pdftotext` callers are:

* `crates/xtask/src/form_geometry.rs:205` — geometry fixtures only;
* `crates/xtask/src/cite_check.rs:534` — driven by `FORMS`, which has **one** entry;
* `scripts/archive_drafts.py:127` — drafts only.

So the 64 files were produced ad hoc and there is no command to reproduce them. Worse, the manifest
`Entry` (`authority_manifest.rs:91-107`) hashes the **PDF** and merely names the extract path;
`verify()` at line 290 only checks the extract *exists*. The artifact every check reads is the one
artifact in the archive with no integrity pin — a hand edit to
`design/forms/extract/f6251--2025.txt` moves every citation assertion with it, silently.

## FINDING 7 — the geometry fixture's sha is right and unchecked

```
geometry fixtures: 48    sha MATCHES manifest: 48    MISMATCH: 0    no manifest entry: 0
```

`form_geometry.rs:67 pub pdf_sha256: String` is written at generation time; grepping every source
file, nothing ever compares it to `MANIFEST.json`. Correct today, unguarded tomorrow — and the whole
point of the manifest (`README.md`: *"A different hash means the IRS REVISED the document"*) is that
a revision must not be absorbed silently. A stale geometry fixture is exactly how it would be.

## FINDING 8 — the right registry already exists and holds one row

    crates/xtask/src/cite_check.rs:651-662
      pub struct FormAuthority { form, year, instructions, instr_pages, extract_stem }
    crates/xtask/src/cite_check.rs:666-672
      pub const FORMS: &[FormAuthority] = &[FormAuthority { form: "f1040s1a", year: 2025, … }];

`FormAuthority` is keyed on **(form, year)** — precisely the year-package key — and its doc comment
already states the recipe verbatim: *"adding a form is a table entry plus a transcription, never a
bespoke project."* It has one entry. Meanwhile `AUTHORITY_NOT_YET_ARCHIVED` (line 691) lists 15
forms as unarchived while the matrix shows all 15 hold a note **and** an extract for 2024 and 2025.
The ratchet measures registry membership, not archive presence, so growing the archive never shrinks
it. That is honest per the README's step-1/2/3 language — but it means the list cannot be used to
answer *"is TY2027 ready?"*.

## FINDING 9 — "supported year" does not require a year package. TY2017 is a hollow one.

    crates/btctax-forms/src/lib.rs:68   pub const SUPPORTED_YEARS: &[i32] = &[2017, 2024, 2025];

TY2017 has 5 bundled templates and 5 maps, and **0** notes, **0** manifest entries, **0** extracts,
**0** geometry fixtures, **0** census sections. It is a fully wired, fillable, shipped year with no
primary source in the repo. Nothing tests (SUPPORTED_YEARS × forms) for completeness; the only
`SUPPORTED_YEARS` loop in any test is `crates/btctax-forms/tests/sp4.rs:397`, and it covers Form 8275
alone — the one form deliberately aliased to a single asset for every year.

## FINDING 10 — the refusal ratchet is a hand-list of years covering 2 of 17 forms

    crates/btctax-forms/tests/map_pdf_conformance.rs:193   for unmapped in [2023, 2025, 2026] {

Only `Form6251Map` and `Form8995AMap` are asserted to refuse. For the other fifteen, a year is
neither required to refuse nor required to resolve. The pairing itself is currently safe by
construction — `*_pdf(year)` and `*Map::for_year(year)` are independent hard matches, so a
half-finished year fails closed on whichever half is missing — but nothing *tests* that the two
year-sets agree, and a single future `_ => Ok(latest)` in either function would pair one year's map
with another year's PDF with no test to notice.

## FINDING 11 — the per-year hand-edit cost, counted

    for_year fns in map.rs:      17
    *_pdf(year) fns in pdf.rs:   17
    CENSUS_KEYS:                 17   (crates/btctax-forms/tests/common/mod.rs:16)

Five hand-edits per form (`include_bytes!` const, `*_pdf` arm, `include_str!` const, `tyYYYY()`,
`for_year` arm) × 17 forms = **85 code edits** for a complete year, plus `SUPPORTED_YEARS`, plus the
year literal in `field_census.rs`. The form registry exists **four times over** as independent
hand-lists — `pdf.rs` consts, `map.rs` consts, `CENSUS_KEYS`, `cite_check::EMITTED_FORMS` — and none
of them is derived from another.

## FINDING 12 — form→instructions is mechanical for 12 of 17, and a human lookup for 5

```
$ ls design/forms/2025/f*.pdf.txt | wc -l   → 17     $ ls design/forms/2025/i*.pdf.txt | wc -l → 14
forms with no identically-numbered instructions note:
  f1040, f1040s1a, f1040s2, f1040s3   (their instructions are a SECTION of i1040gi)
  f1040sa                             (its instructions are i1040sca, not i1040sa)
```

`instr_pages: Some((101, 110))` for Schedule 1-A was read by a person out of a 110-page booklet.
That number cannot be derived from `(stem, year)`.

---

# The runbook, in dependency order

Each step is tagged **M** (a machine can do it from `(stem, year)` plus committed artifacts) or
**H** (someone must read the form). Steps marked **M\*** are mechanical *by nature* and manual
*today* — no committed command does them.

| # | step | tag | today |
|---|---|---|---|
| 1 | Decide the year's form list: does this stem exist for this year, under this name? | **H** | `f1040s1a--2024` correctly does not exist (Pub. L. 119-21); `f8275`→`f8275r` for 2025 |
| 2 | Fetch the authority PDF from `irs-prior/{stem}--{year}.pdf` | **M\*** | no script; `archive_drafts.py` covers drafts only |
| 3 | Write the `.pdf.txt` provenance note (URL, sha256, bytes) | **M\*** | hand-written; format fixed by `design/forms/README.md` |
| 4 | Add/refresh the `MANIFEST.json` entry | **M** | `xtask authority-manifest --regenerate` reads it off the tree |
| 5 | Name the instructions document, and its page range if it is a section | **H** | 5 of 17 need it; `instr_pages` is unreachable from the stem |
| 6 | Fetch + note + manifest the instructions | **M\*** | same as 2-4 |
| 7 | Extract both text layers (`-layout` for a form, plain for 3-column instructions) | **M\*** | rule stated at `cite_check.rs:484`; only wired for `FORMS` (1 row) |
| 8 | Generate the geometry fixture | **M** | `xtask extract-geometry <stem>` |
| 9 | Copy the authority PDF to `crates/btctax-forms/forms/{year}/{crate_stem}.pdf` | **M** | needs a 2-row alias table (`f1040sd`→`schedule_d`, `f1040sse`→`schedule_se`) |
| 10 | Dump the field inventory | **M** | `xtask dump-fields` |
| 11 | Diff against the prior year on BOTH axes (names, line bindings) | **M** | `xtask form-delta <old> <new>` |
| 12 | Enumerate the printed line set **from the extract** | **M** | `xtask label-census <stem>` |
| 13 | Join line → field with both witnesses | **M** | `label_reader.rs`; agreement is machine-checkable |
| 14 | Adjudicate wherever the witnesses disagree | **H** | f8959 lines 5/9/15: box at the bottom of a 3-row block; witness 1 alone is off by one, silently |
| 15 | Decide whether a moved line is the *same* line, or a new one | **H** | 6251 `line1`→`line1a`/`line1b`; Schedule 1-A rebuilt (10 of 219 survive) |
| 16 | Amend the `*Map` struct if the line set changed | **H** (edit) / **M** (detect) | detection is a key-set diff — see Finding 4's script |
| 17 | Transcribe each mapped line's instruction text verbatim as its doc comment | **H** | the *check* (comment ⊆ extract) is mechanical: 17/17 for f8959 |
| 18 | Write the `[census]`: `rule` + `reason` for every unmapped field | **H** | "encodes no decision" vs "we forgot it" is a judgment about the return |
| 19 | Emit the code bindings (2 consts, 1 ctor, 2 match arms) | **M\*** | 85 hand-edits for a full year |
| 20 | Run the gates: map⊆PDF, census union, doc⊆extract, geometry-sha=manifest-sha | **M** | 1 of 4 is year-generic today |
| 21 | Decide whether a changed line is in scope, or the year must refuse | **H** | no delta tool reports a line whose *meaning* changed while its number did not |

**Two structural changes turn most of column "today" into data.** (a) Make the year package a
**table** — one `FormAuthority`-shaped row per (stem, year) carrying crate-stem, instructions stem,
page range, and the census/geometry paths — and derive `pdf.rs`, `map.rs`, `CENSUS_KEYS` and
`EMITTED_FORMS` from it instead of maintaining four hand-lists. (b) Make every gate walk that table
rather than a hardcoded literal, starting with `field_census.rs:105`. Neither removes a single
**H** row — and that is the honest ceiling: **8 of 21 steps require someone to read the form**, and
they are load-bearing, because steps 14, 15 and 21 are precisely where this repo's year-port defects
have actually come from.

---

# MECHANICAL — a machine can do it

1. Fetch the authority PDF; the URL is a pure function of `(stem, year)` (`irs-prior/{stem}--{year}.pdf`).
2. Write the `.pdf.txt` provenance note — URL, sha256, byte count, all read off the fetched bytes.
3. Add/refresh the `MANIFEST.json` entry (`xtask authority-manifest --regenerate` already does).
4. Extract both text layers, with `-layout` for the form and plain for 3-column instruction pages.
5. **Hash the extract into the manifest** — currently absent; the artifact everything reads is unpinned.
6. Generate the geometry fixture (`xtask extract-geometry`).
7. **Assert `geometry.pdf_sha256 == MANIFEST.sha256`** — 48/48 hold today and no test says so.
8. Copy the authority PDF to the bundled template path (given a 2-row crate-stem alias table).
9. Dump the AcroForm field inventory (`xtask dump-fields`).
10. Compute the prior-year delta on both axes (`xtask form-delta`) — names *and* line bindings.
11. Enumerate the printed line set from the extract (`xtask label-census`).
12. Propose the line→field join from the two witnesses, and flag every disagreement for a human.
13. Diff the `.map.toml` key set against the `*Map` struct field set, both directions — the check that
    would have caught `line1` vs `line1a`/`line1b` at commit time rather than at wire-up time.
14. Check every mapped line's doc comment is a substring of its own line's extracted text.
15. Emit the five code bindings per form (consts, ctor, two match arms).
16. Run map⊆PDF (`map_pdf_conformance.rs`, already year-generic) and the census union
    (`field_census.rs`, **needs de-pinning from `let year = 2024`**).
17. Assert the year-sets of `*_pdf(year)` and `*Map::for_year(year)` are identical, per form.
18. Assert completeness: for every year in `SUPPORTED_YEARS` × every form in the registry, every slot
    is present or explicitly recorded absent with a reason — the check that would flag TY2017's five
    source-less maps.

# HUMAN — someone must read a form

1. **Does this form exist for this year, and under what stem?** A missing year is sometimes correct
   (`f1040s1a--2024`) and sometimes a rename (`f8275`→`f8275r`). Only the IRS page distinguishes them.
2. **Which instructions document defines it, and which pages.** Mechanical for 12 of 17 stems;
   `f1040sa`→`i1040sca` and the four `i1040gi`-hosted schedules are a lookup, and the page range
   (`Some((101, 110))`) is a human reading of a 110-page booklet.
3. **Adjudicating witness disagreement.** Form 8959 lines 5, 9 and 15 sit at the bottom of a
   three-row filing-status block; the "nearest label above" witness is off by one on exactly those
   three, and the failure is silent and plausible.
4. **Deciding whether a moved line is the same line.** `line1` → `line1a`/`line1b` is a split, not a
   rename; TY2026 Schedule 1-A keeps 10 of 219 fields, so almost every "match" is a judgment call.
5. **Amending the `*Map` struct** when the line set changes, and choosing whether the old key becomes
   an alias, a new field, or a refusal.
6. **Transcribing each line's instruction text** verbatim as the doc comment. A machine can check
   containment; only a person can choose the sentence and the line it belongs to.
7. **The census disposition.** `rule` + `reason` for every unmapped field — separating *"this line
   encodes no decision"* from *"we forgot this line"* is a claim about the return, not about the PDF.
8. **Whether a line's meaning changed while its number did not**, and whether that puts the year out
   of scope. No delta tool reports this axis, and it is the one that silently produces a wrong number.
