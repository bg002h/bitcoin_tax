# Fold report — interview T2 seam review (C1 / I1 / I2 / I3 / M1–M4)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, from `edf5dde0`. No
subagents, no commits, no pushes. Brief: `design/agent-reports/BRIEF-fold-interview-T2-review.md`.
Contract: `2026-09-07-build-interview-T2-review.md` + its VERIFICATION ledger; `design/SPEC_interview.md`
R2.1/R4/§5.2/§7 T2; `design/forms/README.md`.

**Everything the fold claims is machine-measured below.** `make check` is green (3215 tests run, 12
skipped, clippy clean); `cargo fmt --all` clean; `CARGO_TARGET_DIR=target-clippy cargo clippy
--workspace --all-targets --all-features -- -D warnings` finishes with no warnings.

---

## 1. C1a — the missing editions, archived IN ADDITION (17 documents)

Nothing already archived was removed or changed: `git diff design/forms/MANIFEST.json` is **153
insertions, 0 deletions**, and no pre-existing note, extract or geometry fixture is modified
(`git status --short design/forms` shows only `MANIFEST.json` and `README.md`).

| document | revision READ OFF THE DOCUMENT (extract line) | sha256 (first 16) | bytes |
|---|---|---|---|
| `fw2--2024` | `2024` beside *Form W-2 Wage and Tax Statement* (:71) at :73 — annual | `318af9431e372ef7` | 1,341,097 |
| `iw2w3--2024` | `2024 / General Instructions for / Forms W-2 and W-3` (:1,3-4) — annual | `1eae396d17506e18` | 534,704 |
| `f1099b--2024` | `2024` (:24) beside *Form 1099-B* (:26) — annual | `109f656771e53052` | 594,730 |
| `i1099b--2024` | `2024 / Instructions for Form 1099-B` (:1,3) — annual | `866f36ac32f45e13` | 225,877 |
| `f1098e--2024` | `2024` (:24) beside *Form 1098-E* (:26) — annual | `5ff3e77936678d73` | 482,193 |
| `i1098et--2024` | `2024 / Instructions for Forms / 1098-E and 1098-T` (:1,6-7) — annual | `9469677eb306ea5b` | 158,147 |
| **`f1098--2022`** | **`(Rev. January 2022)`** (:25); footer `Form 1098 (Rev. 1-2022)` (:60) — periodic | `3c77f5a0d5a97bd7` | 510,955 |
| **`i1098--2022`** | **`(Rev. January 2022)`** (:6); footer `(Rev. 01-2022)` (:260) — periodic | `a0de2cd965c5faf6` | 169,482 |
| `fw2--2026` | `2026` beside *W-2 Wage and Tax Statement* (:80) — annual | `61eca7c81f16d396` | 2,150,352 |
| `iw2w3--2026` | `2026 / General Instructions for / Forms W-2 and W-3` (:1,3-4) — annual | `d16b9f506f039f5a` | 527,838 |
| `f1099b--2026` | `2026` (:23) beside *Form 1099-B* (:25) — annual | `31fb17392add5485` | 602,398 |
| `i1099b--2026` | `2026 / Instructions for Form 1099-B` (:1,3) — annual | `81ef22308f8e5f61` | 227,398 |
| `f1098e--2026` | `2026` (:50) beside *Form 1098-E* (:52) — annual | `bffb9b5ec4f06d47` | 495,874 |
| `i1098et--2026` | `2026 / Instructions for Forms / 1098-E and 1098-T` (:1,3-4) — annual | `a06256fece2bfc53` | 155,690 |
| **`f1099g--2026`** | **`(Rev. December 2026)`** (:50); footer `(Rev. 12-2026)` (:70) — periodic | `65a416c52508d8a3` | 542,657 |
| `i1099g--2026` | `(Rev. December 2026)` (:2) — periodic | `e89dac8a60ad285a` | 146,879 |
| `i1098--2026` | `(Rev. December 2026)` (:2) — periodic | `ad68e1d312b85a66` | 154,262 |

Every citation above was machine-checked (a script asserting the named extract line contains the named
text: **0 bad of 42 assertions**). Each got a `.pdf.txt` note in the existing convention, a
`pdftotext -layout` (forms) / no-flags (instructions) extract, and `xtask extract-geometry` for the
eight new forms:

    fw2--2024   272 boxes / 11 pages      fw2--2026    568 / 11
    f1099b--2024 163 / 7                  f1099b--2026 207 / 7
    f1098e--2024  21 / 4                  f1098e--2026  43 / 4
    f1098--2022   62 / 6                  f1099g--2026 148 / 6

### ★★★ Deviation 1 — the Form 1098 revision in force for TY2024 is NOT on `irs-prior`

The brief said *"locate the prior revision on `irs-prior`, e.g. `f1098--2022`"*. Measured 2026-09-07:
`irs-prior/f1098--2022.pdf`, `--2023.pdf` and `--2024.pdf` are **all 404**, and the IRS prior-products
picklist for Form 1098 jumps straight from **2021** to **2025** (parsed from
`apps.irs.gov/app/picklist/list/priorFormPublication.html?value=1098`). Form 1098 was **annual through
2021** and went continuous-use with **Rev. January 2022**, which the IRS has retired from `irs-prior`.

Archiving `f1098--2021` instead would have been a **false** answer: an annual edition governs its own
tax year and nothing later, so it would have made `revision_in_force("f1098", 2024)` return a document
no TY2024 filer holds.

So the Rev. January 2022 pair is archived from the **Internet Archive's capture of the IRS's own
moving URL** `https://www.irs.gov/pub/irs-pdf/f1098.pdf` — the third source
`design/forms/README.md`'s cadence table already contemplates. Corroboration, all measured:

- the fetched bytes print `(Rev. January 2022)` and the footer `Form 1098 (Rev. 1-2022)`;
- the Wayback SHA-1 digest `HXH64KERYKEWLRJCZ5IA24CE6D2KBPG4` is **identical across every capture of
  that IRS URL from 2022-02-08 to 2025-02-26**, and changes only at 2025-03-23 (Rev. April 2025) — so
  the object did not move under the moving URL for three years;
- the base32 SHA-1 of the bytes I archived equals that digest.

Both notes record the URL actually fetched, the reason, and the 404 measurements. `README.md` carries
the same paragraph so a future archiver reads it as a decision rather than a mistake.

### Round-trip, and the archive's currency

`cargo run -p xtask -- authority-refresh --check` (the new I3 command, §7) against the live network:

    authority-refresh: 115 note(s) re-fetched — 115 unchanged, 0 DRIFTED, 0 unreachable
    authority-refresh: probed 14 information-return stem(s) for a newer edition — 0 found
    authority-refresh: OK — every note still matches irs.gov, and no newer edition is served

That is the whole archive, including the two Wayback-sourced notes, verified against irs.gov in one
command — and it is the direct answer to the review's C1: **no newer edition of any information
return is served today.**

---

## 2. C1b — the census is per archived EDITION

`box_census::DocumentAuthority` is now one entry per edition — `stem`, `edition`, `revision_year` (read
off the document), `cadence`, `instructions`, and `preamble_end` (M4). A sibling `BOOKLETS` table
carries the 16 archived booklet editions, so `revision_in_force` answers for an `i…` stem too.

    $ cargo run -q -p xtask -- box-census
      fw2--2024      (iw2w3 instructions):    29 boxes — 18 collected,  2 refuse-if-nonzero,  9 not read
      fw2--2025      (iw2w3 instructions):    29 boxes — 18 collected,  2 refuse-if-nonzero,  9 not read
      fw2--2026      (iw2w3 instructions):    30 boxes — 18 collected,  2 refuse-if-nonzero, 10 not read
      f1099int--2024 (i1099int instructions): 17 boxes —  6 collected,  1 refuse-if-nonzero, 10 not read
      f1099div--2024 (i1099div instructions): 22 boxes —  7 collected,  4 refuse-if-nonzero, 11 not read
      f1099g--2024   (i1099g instructions):   12 boxes —  2 collected,  0 refuse-if-nonzero, 10 not read
      f1099g--2026   (i1099g instructions):   13 boxes —  2 collected,  1 refuse-if-nonzero, 10 not read
      f1099b--2024   (i1099b instructions):   22 boxes —  4 collected,  0 refuse-if-nonzero, 18 not read
      f1099b--2025   (i1099b instructions):   22 boxes —  4 collected,  0 refuse-if-nonzero, 18 not read
      f1099b--2026   (i1099b instructions):   22 boxes —  4 collected,  0 refuse-if-nonzero, 18 not read
      f1098--2022    (i1098 instructions):    11 boxes —  1 collected,  0 refuse-if-nonzero, 10 not read
      f1098--2025    (i1098 instructions):    11 boxes —  1 collected,  0 refuse-if-nonzero, 10 not read
      f1098e--2024   (i1098et instructions):   2 boxes —  1 collected,  0 refuse-if-nonzero,  1 not read
      f1098e--2025   (i1098et instructions):   2 boxes —  1 collected,  0 refuse-if-nonzero,  1 not read
      f1098e--2026   (i1098et instructions):   2 boxes —  1 collected,  0 refuse-if-nonzero,  1 not read
    box-census OK: 246 printed boxes across 15 archived editions of 7 information returns, every one
    decided (246 entries)

### ★ Deviation 2 — an entry carries its EDITIONS, rather than being copied per edition

`BoxEntry` gained `editions: &'static [&'static str]`; an entry is in scope for an edition iff it is
listed. 123 entries cover 246 printed boxes (`grep -c '^ *BoxEntry {'` → 123; `DOCUMENTS.len()` → 15;
`BOOKLETS.len()` → 16 — all machine-counted, none hand-counted).

Duplicating each row per edition would have meant ~250 hand-copies of the same decision text, and it
would have **hidden** the thing worth seeing: where an entry's edition list splits, a caption moved
between revisions. Every listed edition still has its caption checked against that edition's own
extract, so nothing is weakened — `every_printed_box_carries_exactly_one_entry` asserts
`total_printed == total_in_scope_entries` (246 == 246) and a floor of 200.

### The four grid changes the per-edition archive exposed

| edition | what moved |
|---|---|
| `f1099g--2026` | **box 10 "Family leave benefits" is NEW** (an income box); state boxes renumber `10a/10b/11 → 11a/11b/12` |
| `fw2--2026` | **box 14 split into `14a Other` and `14b Treasury Tipped Occupation Code(s)`** — a new box, and the one Schedule 1-A line 4a's tips deduction turns on |
| `fw2--2026` | box 13's caption reads `13 employee` (the 2024/2025 editions read `13 Statutory`) — the checkbox header wraps differently |
| `f1099b--2026` | box 7 reads `7 Check if loss is not` (2024/2025: `7 Check if loss is not allowed`) — wrap position |

### The box-10 decision, and a second T5 item

- **`f1099g` box 10 `10 Family leave benefits` → `RefuseIfNonzero`**, per the brief: *"T5: paid family
  leave benefits — an INCOME box the Rev. December 2026 grid added, reportable on **Schedule 1**. No
  field holds it and no line reads it, so > 0 REFUSES until T5 decides the line: an income box with no
  reader understates, and this fails closed instead."* **T5 owns choosing the Schedule 1 line and the
  `RefuseReason` variant.**
- **`fw2` box 14b `14b Treasury Tipped Occupation Code(s)` → `NotRead`** with a T5 reason naming
  Schedule 1-A line 4a: the code says whether box 7's tips came from a qualifying occupation. It is a
  code, not an amount, and no field holds it at T2. **Second T5 item**, recorded here because it is the
  same class as box 10 — a 2026-only box that one pinned edition hid.
- `f1099g` 11a/11b/12 → `NotRead`, each naming its pre-2026 number; `fw2` 14a inherits box 14's reason;
  `fw2` 13 (2026) and `f1099b` 7 (2026) inherit their siblings' decisions.

### M4 — per-edition face-block bounds

`preamble_end` is per authority: 14 of the 15 editions carry `PREAMBLE_1141 = "1141, 1167, and 1179"`
(machine-counted), and `fw2--2026` carries `PREAMBLE_W2_2026 = "See IRS Publication 1223"` — its
preamble was rewritten (`fw2--2026.txt:26-28`). `the_enumerator_markers_are_present_in_every_extract`
now requires each edition's own marker to occur **exactly once**, so this is a reading of the document,
never a fallback.

### Two new enumerator rules, both forced by a red

Widening `is_label`'s single-letter arm from `a..=f` to `a..=z` (M3) and admitting two new editions
made the enumerator red twice, and each red is now a rule stated in the module doc with its evidence:

- **Rule 5 — `For calendar year` marks the blank year stub.** `f1098--2022` prints a bare `20` under
  *For calendar year*; the contiguity guard caught it.
- **Rule 6 — a bare LETTERED run is the vertical `Code` rail, never a box.** Every captionless box these
  forms print is numbered (W-2 box 9, 12b–12d), so a run that is only a letter is rail furniture. This
  is what makes the M3 widening safe.

---

## 3. C1c — `revision_in_force(stem, tax_year) -> Option<edition>`

Derived from the rule the IRS prints on the 2026 editions (`f1098e--2026.txt:2-5`, *"Which Revision To
Use for Which Year"*): *"the year of the revision date is the first year for which issuers are to use
the form to report amounts."* Annual: `revision_year == tax_year`. Periodic: the greatest
`revision_year <= tax_year`. `None` otherwise.

`box_census::tests::revision_in_force_pins_the_whole_table` pins every cell:

| stem | TY2024 | TY2025 | TY2026 |
|---|---|---|---|
| `fw2` / `iw2w3` | 2024 | 2025 | 2026 |
| `f1099b` / `i1099b` | 2024 | 2025 | 2026 |
| `f1098e` / `i1098et` | 2024 | 2025 | 2026 |
| `f1099int` / `i1099int` (Rev. 1-2024) | 2024 | 2024 | 2024 |
| `f1099div` / `i1099div` (Rev. 1-2024) | 2024 | 2024 | 2024 |
| `f1099g` / `i1099g` | 2024 *(Rev. 3-2024)* | 2024 | **2026** *(Rev. 12-2026)* |
| `f1098` | **2022** *(Rev. 1-2022)* | 2025 *(Rev. 4-2025)* | 2025 |
| `i1098` | **2022** | 2025 | **2026** *(the booklet was revised when the form was not)* |

plus three `None` cases: `fw2 @ 2023` (an annual edition never governs a neighbouring year),
`f1098 @ 2021` (Rev. January 2022 governs nothing earlier) and `f1099nec @ 2024`.

---

## 4. C1d — rows cite a box, the checker resolves the edition

`CollectedFrom::DocBox` lost `year` **and** `extract_line`: it is now `DocBox { stem, box_label }`.
`line_coverage_check` rule (7) resolves the edition through `revision_in_force(stem, row.year)`,
requires that edition to PRINT the box, and requires the box census's entry for it to quote that
edition's own caption (whitespace-normalised). A row whose year has no governing edition reds.

All **27** `DocBox` rows migrated. The visible effect: the 2024 rows now verify against `fw2--2024`,
`f1099b--2024` and `f1098--2022` (they cited `fw2--2025`, `f1099b--2025` and `f1098--2025` before),
while the TY2025 Schedule 1-A rows still resolve to `fw2--2025`.

---

## 5. I1 — `FilerRecords` bound to the row's own form and line

`CollectedFrom::FilerRecords` is now `{ instruction_line }` only. Both joins are derived:

- **the booklet** = `booklet_for(row.form)` at the row's year — the `design/forms/README.md`
  identically-numbered convention with its two stated exception families (`f1040`/`f1040s1`/`s2`/`s3`/
  `s1a` → `i1040gi`; `f1040sa` → `i1040sca`). The anti-tautology guard is now **structural**: every
  value it returns begins `i`, so a row cannot quote its own printed text back at itself.
- **the line** = the sentence must satisfy one of two clauses, measured over all 36 rows:
  - **(i) block containment — 31 of 36.** The booklet's `Line <N>` blocks are enumerated from the
    extract's own headings (three shapes occur and all three are read: `Line 5b` alone, `Line 4. <text>`,
    `Line 7—<Title>`; ranges like `Lines 5a–5d` expand). A block includes its heading line, and an
    **empty** block extends to the next heading.
  - **(ii) the sentence NAMES the row's line — 5 of 36** (machine-counted by disabling clause (ii) and
    counting the reds: exactly 5).

### ★ Deviation 3 — the booklet whose headings do not follow the shape, and how it is bound

The brief asked for this to be recorded. **`i1040gi` is a three-column booklet extracted with no
`pdftotext` flags, and its headings interleave with their bodies across columns.** Measured at
`i1040gi--2024.txt:3962-3975`: `Line 25` / `Federal Income Tax` / `Withheld`, then `Line 26`, then
`Line 25a—Form(s) W-2`, and only then line 26's own body. At `:43326-43350` it is worse: the `Line 10`
heading is followed by line **11**'s body, and line 10's body appears after the `Line 12` heading.

Block containment is therefore not derivable from that extract for every line. Two mechanisms were
tried against all 36 rows and **rejected on measurement** rather than taste: merging adjacent heading
clusters broke 4 rows that bound correctly before (`i8995a` heads every block with a sentence, so
consecutive headings are normal there), and extending a block to the next *higher-numbered* heading
would have made `i8995a`'s `Line 4` block swallow four other lines. What landed is the empty-block
extension (which fixes `f1040:26` honestly) plus clause (ii) for the rest.

### The five rows clause (ii) binds, and the four rows re-pointed

| row | before | after | why |
|---|---|---|---|
| `f1040sd:6` | *"Use this worksheet to figure your capital loss carryovers from 2023 to 2024"* | *"Short-term capital loss carryover for 2024. Subtract line 7 from line 5. If zero or less, enter -0-. If more than zero, also enter this amount on **Schedule D, line 6**"* | the old quote named no line and `i1040sd` prints no `Line 6` heading; the new one is the worksheet line that feeds Schedule D line 6 |
| `f1040sd:14` | the same sentence, twice | *"Long-term capital loss carryover for 2024. … also enter this amount on **Schedule D, line 14**"* | the two rows quoted one sentence; each now quotes its own |
| `f8995a:29` | *"Any negative amount will be carried forward to the next year."* | *"If the sum of **lines 28 and 29** result in a loss (negative number), the loss must be carried forward to next year."* | the old quote is the **Line 28** paragraph's second sentence — the review's own example of the unbound half; `i8995a` prints no `Line 29` heading, and this is the sentence that names line 29 |
| `f1040s3:10` | *"…enter the amount of the payment or any amount you paid with Form 4868."* | the same, extended with *"If you paid a fee when making your payment, don't include on **line 10** the fee you were charged."* | the sentence now names line 10 |

The fifth clause-(ii) row is `f1040s1a:2a`, unchanged: its sentence already says *"complete **lines 2a
through 2e** in Part I of Schedule 1-A"*.

**No other row moved**, and `line-coverage`'s counts are unchanged: **341 money lines / 24 exceptions
(ratchet 24) / 0 unverifiable / 12 not line-bound** — identical to the pre-fold figures. No ratchet was
touched.

---

## 6. I2 — the censused document set is derived from the archive

`archive_check::is_information_return_stem` is new and public: strip the `f`/`i` prefix, then the
remainder is `w`+digit (the wage series) or starts `1098`/`1099`. It is a reading of how the IRS
numbers the series, not a list of the seven we hold — a hand-list would have to be edited to admit the
1099-NEC, 1099-R or 1099-DA.

`box_census::archived_information_returns(root)` reads `MANIFEST.json` and returns the `(stem, edition)`
sets for `kind: form` and `kind: instructions` in that series; `check_document_set` asserts
`DOCUMENTS` and `BOOKLETS` equal them **in both directions**, and is called by the operator command
`xtask box-census` as well as by the suite (so it is live code, not a test-only helper).

---

## 7. I3 — `xtask authority-refresh --check`

New module `crates/xtask/src/authority_refresh.rs`, wired into `main.rs`. `curl` subprocess (no `ureq`
in any crate; xtask is outside `check-isolation`'s TAX_CRATES anyway). **On demand only — never in
`make check`, never in the suite** (`make check` was re-run after adding it: 3215 tests, no network).

It answers the two shapes a stale archive has:

1. **drift** — every `storage: note` entry's own URL is re-fetched and hashed against the note;
2. **a newer edition** — for every information-return stem, `irs-prior/<stem>--<newest archived + 1>.pdf`
   is probed and a 200 reported as *"a newer edition exists"*. The starting year is read out of the
   manifest, never hand-listed (`the_probe_starts_from_the_newest_archived_edition` pins `fw2`→2026,
   `f1099g`→2026, `f1099int`→2024, `f1098`→2025, and that `f6251` is not probed).

`--from-dir <dir>` reads `<dir>/<basename>` instead of fetching, so the command runs with no network.

---

## 8. M1–M3

- **M1** — the `DocumentAuthority::year` doc comment that miscounted (*"Six of the seven documents are
  `--2024`"*) is gone with the field. Every count now in the module doc is machine-produced: 15 form
  editions, 16 booklet editions, 123 entries, 246 printed boxes, 14-of-15 preamble markers.
- **M2** — Form 8995 line 12 retagged `Combine` → **`Carry`**, with the reason in a comment beside it:
  `Combine`'s documented blankness rule (*"blank iff every operand is blank"*) has no operands to range
  over, because the line's own sentence names no line of any form. `Carry` needs no `reason`, so the
  exception ratchet is untouched (24 → 24).
- **M3** — the unlabelled-box limit (the 1099-INT's *FATCA filing requirement* checkbox at
  `f1099int--2024.txt:47` and *2nd TIN not.* at `:51`, both verified present and unlabelled) is declared
  in the module doc beside the wrapped-caption limit. `is_label`'s single-letter arm is widened from
  `a..=f` to `a..=z`, and the letter-contiguity guard still holds because of enumerator rule 6.
- **M4** — see §2.
- **N1** — noted for future briefs: in a PDF-less worktree with `CARGO_TARGET_DIR` redirected, six
  `form_delta` tests and one `harness_check` test fail as environment, not code.

---

## 9. Every kill, seen RED once (plants made with `cp` backups, observed, restored)

| # | what was planted | the red |
|---|---|---|
| A | `revision_in_force` ignores the tax year | `assertion left == right failed: fw2 @ TY2024` / `left: Some("2026")` / `right: Some("2024")` |
| B | the `f1098e--2025` authority deleted (**the reviewer's own I2 plant**) | `DOCUMENTS/BOOKLETS must equal the archived series: "f1098e--2025 is archived as a form but nothing censuses it — an archived information return outside the census is invisible to every box assertion"` |
| C | the `FilerRecords` line binding gutted back to *"anywhere in the booklet"* | `planting a FilerRecords sentence from another LINE of the right booklet must RED, and it did not` |
| D | the `DocBox` edition pinned to the newest archived instead of resolved from the row's year | `planting a DocBox naming a box only a LATER edition prints must RED, and it did not` |
| E | `authority_refresh::compare` stops distinguishing drift | `a revised document must RED, and it did not: Same` |
| F | `fw2--2026`'s per-edition preamble replaced by the single hard-coded marker (**M4**) | test: `fw2--2026: its recorded preamble marker "1141, 1167, and 1179" does not occur exactly once; the face block is being guessed` — and `xtask box-census: fw2--2026: the preamble marker … occurs 0 times, expected exactly 1` |
| G | enumerator rule 5 removed | `xtask box-census: f1098--2022: the box numbers this enumerator found are not contiguous: [12, 13, 14, 15, 16, 17, 18, 19] missing from 1..=20` |
| H | enumerator rule 6 removed | `xtask box-census: fw2--2024: the lettered boxes are not contiguous from 'a': found {'a','b','c','d','e','f','o'}, expected {'a'..'o'} — the reader is dropping boxes` |
| I | **the brief's C1d kill** — the `fw2` box-1 entry split so edition `2024` carries a one-character-drifted caption and `2025`/`2026` keep the true one | `f1040:1a (Form1040Lines.line1a) is Collected from fw2--2024 box 1, whose census entry quotes "1 Wages, tips, other compensatio", but the extract prints "1 Wages, tips, other compensation" — the box caption is the document's text, never ours (instructions: iw2w3)` — **and only the two 2024 rows red; the TY2025 Schedule 1-A row citing the same box stayed green** |

In-suite plants that ship with the fold (each asserts its own red, and C/D above prove the assertions
bite): the six `box_census::verdict` plants, the three enumerator plants + the new year-stub/`Code`-rail
test, the six rule-(7) source plants (now including *"a DocBox naming a box only a LATER edition
prints"* and the reviewer's Schedule A 5b ← Form 6251 AMTFTC sentence), the `FilerRecords` control row
that must PASS, and the three `authority_refresh` tests.

**T2's own kills re-run and still green**: `archive_check::the_w_series_stems_are_primary_sources`,
`box_census::the_gate_reds_on_every_planted_defect`,
`line_coverage_check::each_rule_rejects_a_table_that_violates_it`, and the `CollectedFrom` two-variant
compile kill (structural — unchanged).

---

## 10. Pinned numbers moved

| number | old | new | cause |
|---|---|---|---|
| `authority-manifest` entries | 153 | **170** | +17 archived documents |
| manifest `form` entries | 63 | **71** | +8 form editions |
| manifest `instructions` entries | 37 | **46** | +9 booklet editions |
| `design/forms/README.md` archived count | 98 | **115** | measured `ls design/forms/*/*.pdf.txt \| wc -l` |
| `box-census` | 115 boxes / 7 documents | **246 printed boxes / 15 editions of 7 information returns (123 entries)** | the archive is per revision |
| xtask tests | 139 | **145** (144 run + 1 skipped) | +3 `box_census`, +3 `authority_refresh` |
| `line-coverage` money lines / exceptions / unverifiable / not-line-bound | 341 / 24 / 0 / 12 | **341 / 24 / 0 / 12** | unchanged; four rows re-pointed, none re-classified |

Untouched: `EXCEPTION_RATCHET`, `MAX_UNLOCATABLE`, `MAX_UNVERIFIABLE`, `DUPLICATE_SOURCE_GROUPS`,
`max_unwitnessed`, the label-reader floors, `cite_check::AUTHORITY_NOT_YET_ARCHIVED`. No template was
added under `crates/btctax-forms/forms/`. The spec and the other agent reports were not touched.

---

## 11. Suite lines

    $ cargo nextest run --locked -p xtask --no-fail-fast
      Summary [7.138s] 144 tests run: 144 passed, 1 skipped
    $ cargo nextest run --locked -p btctax-core
      Summary [0.525s] 1230 tests run: 1230 passed, 0 skipped
    $ make check
      Summary [17.783s] 3215 tests run: 3215 passed, 12 skipped        (clippy stage clean)
    $ cargo fmt --all                                                  # clean
    $ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
      Finished `dev` profile — no warnings
    $ cargo run -q -p xtask -- line-coverage
      line-coverage OK: 341 money lines across 17 form(s) …, 24 exception(s) (ratchet 24),
      0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)
    $ cargo run -q -p xtask -- box-census
      box-census OK: 246 printed boxes across 15 archived editions of 7 information returns,
      every one decided (246 entries)
    $ cargo run -q -p xtask -- authority-manifest
      authority-manifest: OK — every entry resolves and every source is listed
    $ cargo run -q -p xtask -- cite-check
      cite-check: OK — 51 quotations, all verbatim.
    $ cargo run -q -p xtask -- archive-check
      archive-check: 3 accounted-for archive(s) — hybrid, decided 2026-07-30 …

Two clippy findings of my own were fixed rather than allowed: `type_complexity` on the
`archived_information_returns` return (given a `pub type ArchivedEditions`) and on the
`revision_in_force` KAT table (given a local `type Row`), and `manual_pattern_char_comparison` on the
heading-label splitter.

---

## 12. Nothing left unfinished

All eight numbered items landed. Two items are handed to **T5** by name, as the brief directs: the
Form 1099-G box 10 *Family leave benefits* Schedule 1 line and its `RefuseReason` variant, and the Form
W-2 box 14b *Treasury Tipped Occupation Code(s)* join to Schedule 1-A line 4a.
