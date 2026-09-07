# Build report — interview T2: the information returns archived, and the box census that reads them

Implementer: one opus agent, shared main tree, branch `main`, from `d49de0c7`. No subagents, no
commits, no pushes. Brief: `design/agent-reports/BRIEF-build-interview-T2.md`. Contract:
`design/SPEC_interview.md` r2 §7 row T2, R2.2, R4, R5, §5.2, §8, and the fold report's T2 rows
(I2, M6).

---

## 1. The archive — 14 documents, revision READ OFF EACH DOCUMENT'S OWN TEXT

Pattern followed: commit `b60c600c` exactly. PDF fetched to `design/forms/<year>/<stem>.pdf`
(gitignored by `.gitignore:63`), a `.pdf.txt` note in the existing convention with the sha256 and byte
count **measured on the fetched bytes**, `pdftotext` to `design/forms/extract/<stem>.txt`,
`extract-geometry` for the forms, then `authority-manifest --regen` + `authority-manifest`.

| document | sha256 | bytes | extract lines | geometry |
|---|---|---|---|---|
| `fw2--2025` | `6a52ad63693de54220a3326b22c4d0fb34f4c25084366537087312975a55ae96` | 1,343,180 | 494 | 272 boxes / 11 pages |
| `iw2w3--2025` | `bfecbc99720cc8fbedb50f6e698ac1f6fa758dbfa5df775715446b7cc30bae2b` | 523,710 | 4,482 | — (instructions) |
| `f1099int--2024` | `ee7697c9b29374596fd9645b157b7312d121ae3c2f76bebf68908c3fc2b739e6` | 538,774 | 251 | 127 boxes / 7 pages |
| `i1099int--2024` | `6f13ff025d4ab270ac53d9f1b60f7f354b1e558366b9d6bfd49cc60bf57fbc63` | 196,656 | 1,053 | — |
| `f1099div--2024` | `4ea1de804ff1db921753bd6319beeccfe4647299e03d79816e04d852a9cd1435` | 542,425 | 206 | 140 boxes / 6 pages |
| `i1099div--2024` | `af89ec14c288df70092cc2439e0bf215bab9704fc0b70ba3f9bd7d420067e607` | 178,269 | 677 | — |
| `f1099g--2024` | `fe46acb40d53442cca67fba55f79aa7683ab8d351e109096c3d6000f60e6226f` | 523,268 | 159 | 100 boxes / 6 pages |
| `i1099g--2024` | `bd1f344b01190496a6a81ae768fb2ff1df8b1d74300a3631a5b6be17eccb9f3a` | 128,896 | 303 | — |
| `f1099b--2025` | `7e334cea66c2b2be3d88615a56db288a2805cb8705076b3b282a4eabbbd788ad` | 601,719 | 271 | 163 boxes / 7 pages |
| `i1099b--2025` | `55bbe7df46f43f2c305481f303bc9e6518f5874766a2d183f2cb14dc0b91d745` | 232,745 | 1,532 | — |
| `f1098--2025` | `304da9c0f67a46ff04ba831359185dc13dde335199cbf7c413dc879e5410dd87` | 501,143 | 156 | 41 boxes / 4 pages |
| `i1098--2025` | `d1d6647dd6a3bc6fb6132593fcc04deb4f163faa9cecaf867961e28f96cbe51a` | 182,716 | 779 | — |
| `f1098e--2025` | `66287bb7f31bbfb1b28a0fef2c2c60175fe01a0f7890763a483dc29ded180246` | 481,922 | 90 | 21 boxes / 4 pages |
| `i1098et--2025` | `68c0f34312f15295ac882e3b4148c0cc93df1f5cf31de8c8f9793258672bafb1` | 161,690 | 493 | — |

Every URL is `https://www.irs.gov/pub/irs-prior/<stem>.pdf`, recorded as the note's first line.

### The revision, read off the document rather than assumed from the filename

Each note carries a `REVISION, READ OFF THE DOCUMENT'S OWN TEXT` block with the extract line it was
read from.

| document | revision as the document prints it | cadence |
|---|---|---|
| `fw2--2025` | `2025`, on the face beside *Form W-2 Wage and Tax Statement* (extract:73) | annual |
| `iw2w3--2025` | `2025 / General Instructions for / Forms W-2 and W-3` (extract:1,3-4) | annual |
| `f1099int--2024` | `(Rev. January 2024)` (extract:24); footer `Form 1099-INT (Rev. 1-2024)` (extract:55) | continuous-use |
| `i1099int--2024` | `(Rev. January 2024)` (extract:7) | continuous-use |
| `f1099div--2024` | `(Rev. January 2024)` (extract:26); footer (extract:56) | continuous-use |
| `i1099div--2024` | `(Rev. January 2024)` (extract:7) | continuous-use |
| **`f1099g--2024`** | **`(Rev. March 2024)`** (extract:25); footer `Form 1099-G (Rev. 3-2024)` (extract:45) | continuous-use |
| **`i1099g--2024`** | **`(Rev. March 2024)`** (extract:6) | continuous-use |
| `f1099b--2025` | `2025`, on the face beside *Form 1099-B* (extract:24) | annual |
| `i1099b--2025` | `2025 / Instructions for Form 1099-B` (extract:1,3) | annual |
| `f1098--2025` | `(Rev. April 2025)` (extract:24); footer `Form 1098 (Rev. 4-2025)` (extract:60) | periodic |
| `i1098--2025` | `(Rev. April 2025)` (extract:2) | periodic |
| `f1098e--2025` | `2025`, on the face beside *Form 1098-E* (extract:24) | annual |
| `i1098et--2025` | `2025 / Instructions for Forms / 1098-E and 1098-T` (extract:1,3-4) | annual |

**Two deviations from the dispatch brief, both decided by the document:**

1. **The 1099-G pair is `Rev. March 2024`, not `Rev. January 2024`.** The brief said all six
   continuous-use 1099s read *"Rev. January 2024"*; `f1099g--2024` and `i1099g--2024` read **March**.
   Recorded in both notes with a ★ saying the document decided it, not the brief.
2. **`i1099int--2024` is the combined 1099-INT / 1099-OID booklet** (*"Instructions for Forms 1099-INT
   and 1099-OID"*, extract:1-2). Recorded in the note; the same shape as the 1098-E's, which the brief
   already named.

The 1098-E's instructions are the combined `i1098et--2025` as the brief measured (`i1098e` does not
exist); the join lives in `box_census::DOCUMENTS` — see §2 for why not in `MANIFEST.json`.

### Round-trip verified against irs.gov

Three notes re-fetched from their own recorded URL and re-hashed, after the notes were written:

    f1098--2025   ROUND-TRIP OK (501143 bytes, 304da9c0…)
    f1099g--2024  ROUND-TRIP OK (523268 bytes, fe46acb4…)
    fw2--2025     ROUND-TRIP OK (1343180 bytes, 6a52ad63…)

### Instructions are extracted with NO flags; forms with `-layout`

`design/forms/README.md:14-40` — *"`-layout` for a form, plain `pdftotext` for 3-column instruction
pages"* — and every one of the 30 header-carrying `i*` extracts on disk records `pdftotext (no
flags)`. The seven instruction booklets were first extracted with `-layout` and **re-extracted without
it**: on a 3-column page `-layout` interleaves two columns into one line, and every `FilerRecords`
quotation in §3 would then have been unquotable. Measured on `i1099int--2024`: `-layout` produces
*"taxable bond, if you have been notified by the taxpayer that the | if you received them in different
years but they both related to"* on one line; no-flags produces reading-order prose.

### `authority-manifest` — 139 → 153 entries

    authority-manifest: regenerated 153 entries
    authority-manifest: by kind — form 63, guidance 22, instructions 37, publication 6, regulation 6, statute 19
    authority-manifest: 0 document(s) archived under more than one path (pinned 0 …)
    authority-manifest: OK — every entry resolves and every source is listed

### ★★★ The first regen listed only 12 of the 14 — a silent hole in the shape detector

`archive_check::irs_stem` matched `f`/`i`/`p` **plus at least three digits**. The IRS does not number
the wage series that way: `fw2`, `iw2w3`, `fw9`. So `classify("fw2--2025.pdf")` returned `None`,
`authority_manifest::collect_sources` skipped the file, and the regen produced **151** entries for 14
new documents — with `authority-manifest` still printing *"OK — every entry resolves and every source
is listed"*. That is the *archived-but-never-recorded* half the manifest census exists to catch,
defeated one layer below it.

Fixed by a narrow W-series arm (`w` then **at least one digit**), with its kill:

- **kill**: `archive_check::tests::the_w_series_stems_are_primary_sources`, asserting both directions
  (`fw2--2025.pdf`, `fw2--2025.pdf.txt`, `iw2w3--2025.pdf`, `iw2w3--2025.pdf.txt`, `fw9.pdf` classify
  as `irs-stem`; `forward.txt`, `index.html`, `issues.txt`, `fw.pdf`, `iw.pdf` classify as `None`).
- **seen RED** by reverting `irs_stem` to the digits-only body (`cp` backup, planted, observed,
  restored):

      thread 'archive_check::tests::the_w_series_stems_are_primary_sources' panicked at
      crates/xtask/src/archive_check.rs:582:13:
      assertion `left == right` failed: `fw2--2025.pdf` is an IRS primary source (the W-series); a
      `None` here makes it invisible to the manifest census while `authority-manifest` still prints OK
        left: None
       right: Some("irs-stem")

  The one-sided halves were deliberately put in one test: the positive half alone passes a matcher
  that returns `Some` for everything, and the negative half alone passes the old matcher.

---

## 2. The per-document BOX CENSUS — `crates/xtask/src/box_census.rs` (new)

Fold I2: *"a KAT reads the box captions out of T2's archived `fNNNN` extract and requires every caption
to carry exactly one entry … A caption in the extract with no entry reds; an entry naming a caption the
extract does not have reds."*

    box-census OK: 115 printed boxes across 7 archived information returns, every one decided

      fw2--2025      (iw2w3 instructions):    29 boxes — 18 collected, 2 refuse-if-nonzero,  9 not read
      f1099int--2024 (i1099int instructions): 17 boxes —  6 collected, 1 refuse-if-nonzero, 10 not read
      f1099div--2024 (i1099div instructions): 22 boxes —  7 collected, 4 refuse-if-nonzero, 11 not read
      f1099g--2024   (i1099g instructions):   12 boxes —  2 collected, 0 refuse-if-nonzero, 10 not read
      f1099b--2025   (i1099b instructions):   22 boxes —  4 collected, 0 refuse-if-nonzero, 18 not read
      f1098--2025    (i1098 instructions):    11 boxes —  1 collected, 0 refuse-if-nonzero, 10 not read
      f1098e--2025   (i1098et instructions):   2 boxes —  1 collected, 0 refuse-if-nonzero,  1 not read

New operator command `cargo run -q -p xtask -- box-census`, and the module doc carries the enumerator's
four rules with the evidence for each.

### The enumerator is a reading of the documents, not a hand-list

1. **Face block** = the lines strictly between the *Attention* preamble's last sentence (*"See
   Publications 1141, 1167, and 1179 …"*, which occurs **exactly once** in each of the seven extracts —
   measured) and Copy A's first `Cat. No.` footer. That excludes the prose pages, the recipient
   instructions and the repeated Copy B/C/1/2 faces without naming a page number.
2. **Runs** split on two-or-more spaces; a run beginning `<label> <Capital|(>` is a caption, and a run
   that is *only* a label adopts the next run on its line unless that run is itself a label.
3. **`OMB No.` ends a caption** — Paperwork Reduction Act furniture, never a caption, and asserted
   present in all seven extracts (`the_enumerator_markers_are_present_in_every_extract`). Without it,
   Form 1099-G box 1 transcribes as `1 Unemployment compensation OMB No. 1545-0120`, because
   `pdftotext -layout` collapses that column gap to one space.
4. **Contiguity guard** — the numeric labels must run `1..=max` with no gap, lettered labels from `a`.

Rule 2's second clause and rule 4 are not decoration. **The first draft's splitter silently dropped
Form 1099-DIV boxes 3, 4, 5 and 6** — the form prints `3    Nondividend distributions` with the label
in its own column — and returned an 18-label list that looked entirely plausible. The gap is what
showed it.

### Empty captions are recorded, not dropped

The 2025 Form W-2 prints box `9` **shaded with no caption**, and `12b`/`12c`/`12d` as bare labels beside
a vertical *Code* rail. All four carry entries with reasons. Dropping a box because it carries no words
is the "we forgot this box" defect wearing the costume of a clean list.

### What the entries are, and what they are NOT (fold D7)

D7: *"T2 owns the extract, so T2's kill is the unentered-caption red and T5's is the entries
themselves."* So `BoxDecision` is `Collected(field) | RefuseIfNonzero(guard) | NotRead(reason)` with
free text, and it deliberately does **not** yet join to `FieldId` / `RefuseReason` variants — those
variants are T5/T9's, and a join written before they exist would be a hand-list pretending to be a
check. A `NotRead` whose reason begins *"T5:"* / *"T9:"* is a box a later task collects, and the reason
names the line it will reach (1099-INT box 10 → Schedule B line 1; boxes 11–13 → the bond-premium
refusal; W-2 box 13 → `StatutoryEmployeeW2` naming Schedule C line 1; 1099-DIV box 3 →
`not_read("reduces basis")`; every Form 1098 box → T9).

### The kills, and the red text

`the_gate_reds_on_every_planted_defect` (the `field_census.rs:329` shape — `verdict()` is pure, so the
plants need no tree): four plants, each red with its own message —

| plant | refusal must contain |
|---|---|
| a printed box with no entry | `we forgot this box` |
| an entry for a box the extract does not print | `the extract does not print it` |
| a caption changed by one character (*Treasury* → *Treasry*) | `caption does not match the extract` |
| two entries for one box | `more than one entry` |

`the_enumerator_reds_when_it_drops_a_box` plants a gap and an unbounded face block against the
enumerator itself (`not contiguous`, `Copy A never closes`).

**Seen RED on the committed registry, not only the synthetic one** (`cp` backup, planted, observed,
restored):

- deleting the 1099-INT box 10 entry:

      the box census failed for 1 of 7 documents:
      f1099int--2024:
        box 10 ("10 Market discount") is printed on the form and NOTHING decides it — we forgot this
        box, which is invisible on the page and to every value assertion

- removing one character from box 3's caption:

      f1099int--2024:
        box 3's caption does not match the extract: census "3 Interest on U.S. Savings Bonds and
        Treasury obligation" vs printed "3 Interest on U.S. Savings Bonds and Treasury obligations"

Two more tests keep the instrument from passing by finding nothing:
`every_entry_belongs_to_an_archived_document` (no entry can park on a typo'd stem) and
`every_document_has_its_extract_and_its_note` (both halves of the archive obligation on disk for all
seven forms **and** their instructions).

### Deviation: the 1098-E → `i1098et` join lives in `DOCUMENTS`, not `MANIFEST.json`

The brief said *"the `MANIFEST.json` `instructions` join for a 1098-E row is `i1098et`"*. Measured:
`MANIFEST.json` entries carry `path / kind / storage / sha256 / bytes / url / extract` and **no
`instructions` field** — the form→instructions join in this tree is `cite_check::FORMS.instructions`,
whose table is *"every IRS form btctax EMITS"*. btctax emits none of these seven (the brief forbids
bundled templates for them), so adding rows there would be false. The join is therefore
`box_census::DocumentAuthority.instructions`, where `f1098e` → `i1098et`, `fw2` → `iw2w3`, and the rest
follow the identically-numbered convention. `check_collected_from` prints it in its refusals.

---

## 3. `Production::Collected` gains `from` — R2.2 / R5, and T2's two kills

`crates/btctax-core/src/tax/line_coverage.rs`:

```rust
pub enum CollectedFrom {
    DocBox { stem, year, box_label, extract_line },   // the box's caption, verbatim
    FilerRecords { stem, year, instruction_line },    // the instruction's own sentence, verbatim
}
pub enum Production { Collected(CollectedFrom), Carry, Combine, … }
```

with `Production::doc_box(...)` / `Production::filer_records(...)` constructors so a call site stays one
expression. **All 71 `Collected` rows were migrated** (27 `DocBox`, 36 `FilerRecords`, 8 reclassified —
see below); the 6 synthetic rows in the checker's own tests were given a real `FilerRecords`.

### Deviations from the spec's field list, recorded

- **`year` added to both variants.** The spec writes `DocBox { stem, box, extract_line }`. The archive
  is year-keyed and six of the seven documents are `--2024` (continuous-use), so a stem alone cannot
  resolve `design/forms/extract/<stem>--<year>.txt`. The document's year is also **not** the row's
  year: 1040 line 2b is quoted from `f1040--2024` and collected from `f1099int--2024`, while line 1a is
  quoted from `f1040--2024` and collected from `fw2--2025`.
- **`box` → `box_label`.** `box` is a reserved keyword in Rust.
- **`extract_line` holds the LINE OF THE EXTRACT** — the caption text as the extract prints it, not a
  line number. A line number cannot be *"verified verbatim"* (R2's word) and drifts on re-extraction;
  the tree's own quotation convention (`LineCoverage.instruction`) carries text, and this matches it.
- **`FilerRecords` carries `stem`/`year` too**, for the same resolution reason.

### The checker — rule (7) in `line_coverage_check::check_collected_from`

- **`DocBox`**: the (stem, year) must be one of `box_census::DOCUMENTS`; that document must actually
  PRINT the named box (enumerated from its extract, not from the row); and `extract_line` must equal
  the printed caption. **This is a join between two independently-derived readings of the same paper** —
  a 1040 line's claim about a W-2 box, and the W-2's own box grid.
- **`FilerRecords`**: non-empty; the stem must be an `i…` instructions booklet; **the stem may not be
  the row's own form**; the extract must exist; and the sentence must occur verbatim
  (whitespace-normalised) in it.

The *"never the row's own form"* clause is the **anti-tautology guard**, and it exists because of how
this field would otherwise be satisfied under time pressure: the row already carries its form's printed
text in `instruction`, and pasting that back in would type-check, resolve, pass a `contains` and prove
nothing.

### Kill (a) — a one-character caption change reds `line-coverage`

Planted on the committed table (W-2 box 1's caption on 1040 line 1a), observed, restored:

    xtask line-coverage: line-coverage FAILED (1 problem(s)):
      - f1040:1a (Form1040Lines.line1a) quotes fw2--2025 box 1 as "1 Wages, tips, other compensatio",
        but the extract prints "1 Wages, tips, other compensation" — the box caption is the document's
        text, never ours (instructions: iw2w3)

### Kill (b, M6) — a `Collected` line with NEITHER a `DocBox` nor a `FilerRecords`

`CollectedFrom` has exactly two variants, so the "neither" state is **unrepresentable**: the omission is
a compile error at the call site, before any test runs. Planted on Form 6251 line 8, observed,
restored:

    error[E0308]: mismatched types
       --> crates/btctax-core/src/tax/line_coverage.rs:472:9
        |
     82 |     Collected(CollectedFrom),
        |     --------- `Collected` defines an enum variant constructor here, which should be called
    ...
    472 |         Production::Collected,
        |         ^^^^^^^^^^^^^^^^^^^^^ expected `Production`, found enum constructor
        |
        = note:          expected enum `Production`
                found enum constructor `fn(CollectedFrom) -> Production {Production::Collected}`

A third *"computed elsewhere"* variant was considered and **rejected**: it is the residual bucket this
file already records `Production::Exception` becoming in SPEC r1, and it would have absorbed the eight
mis-tagged rows below instead of surfacing them.

### Kill (b′) — the rule watched red in the suite, six plants

Added to `each_rule_rejects_a_table_that_violates_it`, each derived from one clean control row (1040
line 2b ← 1099-INT box 1) so only the source naming differs:

| plant | refusal must contain |
|---|---|
| a `DocBox` caption changed by one character | `but the extract prints` |
| a `DocBox` naming a box the form does not print | `which that form does not print` |
| a `DocBox` naming an unarchived document (`f1099nec`) | `no such information return is archived` |
| a `FilerRecords` quoting the row's own form | `quotes its own form back at itself` |
| a `FilerRecords` sentence that is not in the instructions | `NOT FOUND in that extract` |
| a `FilerRecords` with a blank instruction line | `EMPTY instruction line` |

**Seen RED** by neutering rule (7) (`if let Production::Collected(_from) = … { }`), observed, restored:

    thread 'line_coverage_check::tests::each_rule_rejects_a_table_that_violates_it' panicked at
    crates/xtask/src/line_coverage_check.rs:1680:36:
    planting a DocBox caption changed by one character must RED, and it did not

### ★★★ Adding the field surfaced EIGHT rows tagged `Collected` that nothing collects

This is the finding of the task, and it is the argument for letting the compiler do the review: making
`from` mandatory forced every one of the 71 sites to be re-read, and eight of them could not name a
source because no filer supplies them. Each is now the production its **own quoted sentence** dictates,
and the checker's existing rules (3) and (4) gated the result:

| row | was | now | the sentence that decides it |
|---|---|---|---|
| f6251 line 2a | `Collected` | `Carry` | *"…enter the amount from Form 1040 or 1040-SR, line 12"* |
| f6251 line 2b | `Collected` | `Carry` | *"Tax refund from Schedule 1 (Form 1040), line 1 or line 8z"* |
| f6251 line 4 | `Collected` | `Combine` | *"Combine lines 1 through 3"* |
| f6251 line 9 | `Collected` | `Combine` | *"Subtract line 8 from line 7"* (no clamp) |
| f6251 line 11 | `Collected` | `Clamped(FloorAtZero)` | *"If zero or less, enter -0-"* |
| f1040sc line 28 | `Collected` | `Combine` | *"Add lines 8 through 27b"* |
| f1040sse line 2 | `Collected` | `Carry` | *"Net profit or (loss) from Schedule C, line 31"* |
| f8995 line 12 | `Collected` | `Combine` | *"Enter your net capital gain, if any, increased by any qualified dividends"* |

**No emitted figure can change.** `Production` is census metadata: the only readers are
`xtask line_coverage_check` and one `== Production::Exception` filter in
`btctax-core/src/tax/tables.rs:2053`. Confirmed by grep across the workspace and by the suites below.

### The 27 `DocBox` rows

W-2 → 1040 1a (×2 collectors) and Sch 1-A 14a (box 1), 1040 25a (box 2), Sch SE 8a (box 3), F8959 1
(box 5), F8959 19 (box 6), Sch 1-A 4a (box 7). 1099-INT → 1040 2b (×2, box 1), Sch 1 18 (box 2), 1040
25b (box 4), Sch 3 1 (box 6), 1040 2a (×2, box 8), Sch B row (box 1). 1099-DIV → 1040 3b (×2, box 1a),
1040 3a (×2, box 1b), Sch D 13 (box 2a). 1099-G → Sch 1 7 (box 1), Sch 1 1 (box 2). 1099-B → Sch D
1a(d)/8a(d) (box 1d), 1a(e)/8a(e) (box 1e). 1098 → Sch A 8a (box 1).

Where a line is fed by several documents (1040 25b takes 1099-INT/DIV/G box 4; 1040 2a takes 1099-INT
box 8 and 1099-DIV box 12), the `DocBox` names the box the interview collects it from, and the *complete*
set of boxes reaching a line is the other direction of the join — the box census's own `Collected`
reasons, each of which names the line it reaches.

### The 36 `FilerRecords` quotations were machine-verified before the code was written

A script normalised each cited instructions extract and asserted `contains` on every quotation:
**36 quotes, 0 MISSING**. R5's two named quotations are among them —
Schedule A 5a: *"State and local income taxes **paid in 2024** for a prior year, such as taxes paid
with your 2023 state or local income tax return."* (`i1040sca--2024`), and
1040 line 26: *"Include any overpayment that you **applied to your 2024 estimated tax** from your 2023
return or an amended return (Form 1040-X)."* (`i1040gi--2024`).

---

## 4. Pinned numbers moved

| number | old | new | cause |
|---|---|---|---|
| `authority-manifest` entries | 139 | **153** | +14 archived documents |
| manifest `form` entries | 56 | **63** | the seven information returns (measured from `HEAD:design/forms/MANIFEST.json`) |
| manifest `instructions` entries | 30 | **37** | their seven booklets |
| `design/forms/README.md` archived count | 60 | **98** | +14 mine; the number was **already stale by 24** (84 notes on disk at `d49de0c7`). The measuring command is now written beside it |
| xtask test count | 132 | **139** | +1 `archive_check` W-series kill, +6 `box_census` |
| `line-coverage` money lines / exceptions / unverifiable / not-line-bound | 341 / 24 / 0 / 12 | **341 / 24 / 0 / 12** | unchanged — the 8 reclassifications move between non-`Exception` productions |
| box census | — | **115 boxes / 7 documents** | new instrument |

Nothing else moved: `max_unwitnessed`, the census registers, the label-reader floors, `EXCEPTION_RATCHET`,
`MAX_UNLOCATABLE`, `MAX_UNVERIFIABLE`, `DUPLICATE_SOURCE_GROUPS` are untouched. No template was bundled
under `crates/btctax-forms/forms/`. The spec and the other agent reports were not touched.

---

## 5. Commands, with their summary lines

    $ cargo run -q -p xtask -- authority-manifest --regen
    authority-manifest: regenerated 153 entries

    $ cargo run -q -p xtask -- authority-manifest
    authority-manifest: OK — every entry resolves and every source is listed

    $ cargo run -q -p xtask -- box-census
    box-census OK: 115 printed boxes across 7 archived information returns, every one decided

    $ cargo run -q -p xtask -- line-coverage
    line-coverage OK: 341 money lines across 17 form(s) […], 24 exception(s) (ratchet 24),
    0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)

    $ cargo run -q -p xtask -- archive-check
    archive-check: no primary source outside the 5 accounted-for tree(s)

    $ cargo nextest run --locked -p xtask
    Summary [6.393s] 138 tests run: 138 passed, 1 skipped

    $ cargo nextest run --locked -p btctax-core
    Summary [0.543s] 1230 tests run: 1230 passed, 0 skipped

    $ cargo fmt --all                       # clean
    $ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile … (no warnings)

Clippy found two of my own defects on the first pass and both were fixed rather than allowed:
`needless_borrow` on `&e.form`, and `type_complexity` on the plant table (given a `type Plant` alias,
matching `field_census.rs`'s own resolution of the same lint).

---

## 6. Not done, and why

- **The `[boxes]` entries do not join to `FieldId` / `RefuseReason` variants.** Fold D7 assigns the
  entries themselves to T5; the variants for W-2 box 13, 1099-INT boxes 10–13 and the Form 1098 boxes do
  not exist yet, and a join written against names that do not exist would be a hand-list. Every such box
  carries a `NotRead` whose reason names the task and the line it will reach, so the residue is legible
  rather than implicit.
- **No document screens, no `Form1098` / `Form1098E` structs, no `Production` change to
  `student_loan_interest_paid` or `mortgage_interest_1098`.** T5 and T9.
- **No geometry for the seven instructions booklets** — `extract-geometry` is for AcroForm-bearing
  forms; instruction PDFs carry none, and no committed geometry fixture exists for any `i*` stem.
- **`cite_check::FORMS` gained no rows** and `AUTHORITY_NOT_YET_ARCHIVED` did not move: that table is
  *"every IRS form btctax emits"*, and btctax emits none of these seven.
- **A caption the layout wraps is quoted to its first printed line** (1099-INT box 9 →
  `"9 Specified private activity bond"`, its second line *"interest"* sitting in another column's
  vertical run). The wrap is layout, not text; quoting across it would quote something the page never
  prints contiguously. Named in the module doc as a stated limit rather than left to be discovered.
- **`design/forms/README.md`'s step-2/step-3 paragraphs were not rewritten.** Only the stale archived
  count was corrected; the fourteen documents sit at step 2 (extract committed) and step 3 is the box
  census, which is now the instrument for them — but the paragraph as written is about
  `cite_check`'s ratchet, which these documents do not enter.
