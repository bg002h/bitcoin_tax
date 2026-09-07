# Seam review — interview build T2 (the archived information returns + the box censuses)

Independent adversarial BUILD REVIEWER, own worktree at `35ac41b9` (build under review `1c8a7301`).
Read-only: no source edits committed, no subagents. Every plant below was made in this worktree,
observed, and reverted from a `cp` backup; `git status --short` is empty at the time of writing and
`git rev-parse --short HEAD` is `35ac41b9`.

## Commands

    export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
    cargo run -q -p xtask -- authority-manifest      # OK — 153 entries; form 63, guidance 22,
                                                    #   instructions 37, publication 6, regulation 6, statute 19
    cargo run -q -p xtask -- box-census              # OK: 115 printed boxes across 7 archived
                                                    #   information returns, every one decided
    cargo run -q -p xtask -- line-coverage           # OK: 341 money lines / 17 forms, 24 exceptions
                                                    #   (ratchet 24), 0 unverifiable, 12 not line-bound
    cargo nextest run --locked -p xtask --no-fail-fast
                                                    # 138 run: 131 passed, 7 failed, 1 skipped
                                                    #   — all 7 are worktree artifacts, see N1

    # 14 documents re-fetched from each note's own URL and re-hashed (network), then
    # `pdftotext -layout` (forms) / `pdftotext` no-flags (instructions) diffed against the
    # committed extract with `cmp`.

## Summary

The archive itself is **sound as built**: 14/14 documents re-fetch to the byte, 14/14 note hashes
and byte counts match the live bytes, 14/14 extracts are byte-identical to `pdftotext` of those
bytes, 7/7 geometry fixtures carry the true `pdf_sha256`, all 14 are in `MANIFEST.json` with correct
`kind`, and every one of the 14 revision claims is present at the extract line the note cites — the
1099-G pair's *Rev. March 2024* deviation from the dispatch brief is real and correctly recorded.
The two new instruments discriminate on every class the report claims: a one-character caption drift,
a box the extract does not print, a box nothing decides, a `DocBox` naming an unarchived document,
a `DocBox` naming an unprinted box, and a `FilerRecords` sentence not in the extract all red, with
the messages quoted in the report.

What the earlier rounds could not see, and this pass did, is **which paper the archive is** and
**what the instruments cannot see**: the archive is pinned one tax year behind the spec's stated
target, and one of the two new checks is a join in name only.

## The 14 documents

| stem | revision READ off the document (extract line verified) | sha vs live | bytes | extract byte-exact | manifest | verdict |
|---|---|---|---|---|---|---|
| `fw2--2025` | `2025` beside *Form W-2 Wage and Tax Statement* (:73) | MATCH | 1,343,180 | YES (`-layout`) | form | OK |
| `iw2w3--2025` | `2025 / General Instructions for / Forms W-2 and W-3` (:1,3-4) | MATCH | 523,710 | YES (no flags) | instructions | OK |
| `f1099int--2024` | `(Rev. January 2024)` (:24); footer `(Rev. 1-2024)` (:55) | MATCH | 538,774 | YES (`-layout`) | form | OK |
| `i1099int--2024` | `(Rev. January 2024)` (:7) — the combined 1099-INT/OID booklet | MATCH | 196,656 | YES (no flags) | instructions | OK |
| `f1099div--2024` | `(Rev. January 2024)` (:26); footer (:56) | MATCH | 542,425 | YES (`-layout`) | form | OK |
| `i1099div--2024` | `(Rev. January 2024)` (:7) | MATCH | 178,269 | YES (no flags) | instructions | OK |
| `f1099g--2024` | **`(Rev. March 2024)`** (:25); footer `(Rev. 3-2024)` (:45) | MATCH | 523,268 | YES (`-layout`) | form | **stale for TY2026 — C1** |
| `i1099g--2024` | **`(Rev. March 2024)`** (:6) | MATCH | 128,896 | YES (no flags) | instructions | **stale for TY2026 — C1** |
| `f1099b--2025` | `2025` beside *Form 1099-B* (:24) | MATCH | 601,719 | YES (`-layout`) | form | **superseded — C1** |
| `i1099b--2025` | `2025 / Instructions for Form 1099-B` (:1,3) | MATCH | 232,745 | YES (no flags) | instructions | **superseded — C1** |
| `f1098--2025` | `(Rev. April 2025)` (:24); footer `(Rev. 4-2025)` (:60) | MATCH | 501,143 | YES (`-layout`) | form | OK (still current) |
| `i1098--2025` | `(Rev. April 2025)` (:2) | MATCH | 182,716 | YES (no flags) | instructions | **superseded — C1** |
| `f1098e--2025` | `2025` beside *Form 1098-E* (:24) | MATCH | 481,922 | YES (`-layout`) | form | **superseded — C1** |
| `i1098et--2025` | `2025 / Instructions for Forms / 1098-E and 1098-T` (:1,3-4) | MATCH | 161,690 | YES (no flags) | instructions | **superseded — C1** |

`fw2--2025` / `iw2w3--2025` are also superseded (see C1); the table marks the two whose *grid* or
*booklet* changed most sharply, and C1 lists all nine.

---

### C1 (Critical) — the archive is pinned one tax year behind the spec's target, and the 1099-G grid moved: the TY2026 Form 1099-G prints a **box 10 "Family leave benefits"** that the census does not enumerate

**Where.** `design/forms/2025/*.pdf.txt`, `design/forms/2024/*.pdf.txt`,
`crates/xtask/src/box_census.rs:97-133` (`DOCUMENTS`), `crates/xtask/src/box_census.rs:310-333`
(the `f1099g` entries), and `crates/btctax-core/src/tax/line_coverage.rs` (every `doc_box` `year`).

**What is wrong.** The dispatch brief said *"the one in force for TY2025"* and the build followed it
exactly — but the contract is the spec, and `design/SPEC_interview.md:45` states the target
outright: *"**TY2026, the target** (`ROADMAP_STATUS.md` §0a)"*, with §7's ordering rationale
repeating it (*"what a W-2-plus-crypto filer with interest and dividends needs on **TY2026** in
Sep–Dec"*). The T2 row's cadence is *"per-revision (periodic)"*, so the year is not fixed by the
spec — it is fixed by which paper the filer will hold.

The IRS prints the mapping rule itself, on the 2026 editions (`f1098e--2026`, `f1099g--2026`, page 1,
measured):

> **Which Revision To Use for Which Year.** We issue information returns up to a year in advance of
> when issuers will first file them. For all forms that we do not issue annually (such as Form 1040),
> **the year of the revision date is the first year for which issuers are to use the form to report
> amounts.** … we are developing a December 2026 revision of Form 1099-NEC, to use first to report
> amounts for calendar year 2026 with the first filings with the IRS beginning in January 2027.

Measured against `irs-prior` at review time, **9 of the 14 archived documents are superseded for
TY2026**, and every replacement is a FINAL (0 occurrences of *"DRAFT AS OF"* in each):

| archived | in force for TY2026 | status |
|---|---|---|
| `fw2--2025` | `fw2--2026` (200) | superseded |
| `iw2w3--2025` | `iw2w3--2026` (200) | superseded |
| `f1099b--2025` | `f1099b--2026` (200) | superseded |
| `i1099b--2025` | `i1099b--2026` (200) | superseded |
| `f1098e--2025` | `f1098e--2026` (200) | superseded |
| `i1098et--2025` | `i1098et--2026` (200) | superseded |
| `f1099g--2024` (Rev. 3-2024) | `f1099g--2026` — **Rev. December 2026** | superseded, **grid changed** |
| `i1099g--2024` (Rev. 3-2024) | `i1099g--2026` — **Rev. December 2026** | superseded |
| `i1098--2025` (Rev. 4-2025) | `i1098--2026` — **Rev. December 2026** | superseded |
| `f1099int--2024`, `i1099int--2024`, `f1099div--2024`, `i1099div--2024` | unchanged (`--2026` 404; `irs-pdf/` still serves the identical bytes — hashes equal) | **correct** |
| `f1098--2025` | unchanged (`f1098--2026` 404; `irs-pdf/f1098.pdf` hashes identically to the archived bytes) | **correct** |

**Why it is Critical rather than a scheduling note.** For eight of the nine the caption text is
layout-equivalent (I diffed all 115 pinned captions against the 2026 editions: the only misses on
`fw2`/`f1099b`/`f1098e` are wrap positions — *"13 Statutory"*, *"14 Other"*, *"7 Check if loss is not
allowed"*). For the **1099-G the box grid itself moved**, and it moved by adding an **income** box:

    Rev. March 2024 (archived, 12 boxes):
      9 Market gain
      10a State   10b State identification no.   11 State income tax withheld

    Rev. December 2026 (in force for TY2026, 13 boxes):
      9 Market gain            10 Family leave benefits
      11a State   11b State identification no.   12 State income tax withheld

So on the paper a TY2026 filer receives, **box 10 is paid family leave benefits** — taxable income
reportable on Schedule 1 — and the state boxes have renumbered `10a/10b/11 → 11a/11b/12`. Against the
archived extract `xtask box-census` prints *"OK: 115 printed boxes … every one decided"* while an
income box on the real document has no entry at all. That is exactly the defect the census exists to
catch (`CLAUDE.md`: *"blank because nothing populated it … invisible on the printed page, invisible
to both oracles"*), reached one layer below the instrument by pinning the wrong revision — and it is
not self-correcting, because nothing re-checks the archive (see I3).

**Evidence.**

    $ curl -sI https://www.irs.gov/pub/irs-prior/f1099g--2026.pdf   # 200   (also i1099g--2026, i1098--2026,
    $ curl -sI https://www.irs.gov/pub/irs-prior/f1099g--2025.pdf   # 404    fw2--2026, iw2w3--2026,
                                                                   #        f1099b--2026, i1099b--2026,
                                                                   #        f1098e--2026, i1098et--2026 → 200)
    $ pdftotext -layout f1099g--2026.pdf - | grep -o '(Rev\. December 2026)'
    (Rev. December 2026)
    $ pdftotext -layout f1099g--2026.pdf - | sed -n '.../face block/...'
     9 Market gain                        10 Family leave benefits
     11a State    11b State identification no. 12 State income tax withheld

    $ sha256sum irs-pdf/f1099int.pdf irs-pdf/f1099div.pdf irs-pdf/f1098.pdf   # equal to the archived bytes
    $ sha256sum irs-pdf/f1099g.pdf   65a416c5…  ≠  archived fe46acb4…          # a newer revision is live

**Minimal change.** Re-archive the nine superseded documents at their TY2026 names/revisions
(the same fetch → note → `pdftotext` → `extract-geometry` → `authority-manifest --regen` pattern),
move `box_census::DOCUMENTS` and every `doc_box` `year` to them, and let `box-census` red on the
1099-G until box 10 carries a decision and the state boxes are renumbered. If the owner instead rules
that T2 is deliberately anchored on TY2025 (matching the `--2025` 1040-family extracts the spec cites
throughout), that ruling belongs **in the notes and in the `DocumentAuthority::year` doc**, with the
TY2026 refresh filed as a T5-owned follow-up naming the 1099-G box-10 gap by name — because the
current text asserts only *"the revision in force for TY2025"* and says nothing about the target year
diverging from it.

---

### I1 (Important) — `FilerRecords` is a join in name only: any sentence from any archived `i*` booklet passes, from any form

**Where.** `crates/xtask/src/line_coverage_check.rs:601-642` (the `FilerRecords` arm of
`check_collected_from`), and the doc claim at
`crates/btctax-core/src/tax/line_coverage.rs` (`CollectedFrom`, *"the named INSTRUCTIONS document is
archived and carries `instruction_line` verbatim"*).

**What is wrong.** The `DocBox` half is a real join: document → box label → caption, all three checked
against a grid enumerated independently from the document's own extract. The `FilerRecords` half
checks only that (a) the stem starts with `i`, (b) `design/forms/extract/<stem>--<year>.txt` exists,
and (c) the sentence occurs somewhere in that file after whitespace normalisation. **Nothing binds
the booklet to the row's form, and nothing binds the sentence to the row's line.** The booklets are
300–4,500 lines each (`i1040gi--2025` is ~40,000), so the field is satisfiable by any sentence in the
archive — and 36 of the 71 `Collected` rows are `FilerRecords`, i.e. the unbound half is the majority
of the rows this check was added for.

The report presents the *"never the row's own form"* clause as the guard against this
(*"the anti-tautology guard … pasting that back in would type-check, resolve, pass a `contains` and
prove nothing"*). That clause tests `stem == form`, which only fires when the row's own form is named
— it does not fire when a *different* form's booklet is named, which is the larger and easier hole.

**Evidence — planted on the committed table, observed, reverted.** Schedule A line 5b (state and local
**real estate taxes**) re-pointed at the Form 6251 AMT-foreign-tax-credit sentence:

    - Production::filer_records("i1040sca", "2024",
    -     "Enter on line 5b the state and local taxes you paid on real estate you own that wasn't used for business")
    + Production::filer_records("i6251", "2024",
    +     "The AMTFTC is a credit that you can claim against the AMT.")

    $ cargo run -q -p xtask -- line-coverage
    line-coverage OK: 341 money lines across 17 form(s) [...], 24 exception(s) (ratchet 24),
    0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)

Green. A Schedule A row now cites a Form 6251 sentence about a credit it has nothing to do with, and
the check that exists to say *"where does this figure come from"* accepts it.

The nearest real row shows the hole is already occupied rather than hypothetical: Form 8995-A line 29
(*"Qualified REIT dividends and PTP (loss) carryforward from **prior years**"*) cites
*"Any negative amount will be carried forward to the next year."* — the **second sentence of the
`i8995a--2024` paragraph headed "Line 28."** (`design/forms/extract/i8995a--2024.txt:869-870`), about
the carryforward *out*. It is defensible on a charitable reading; the point is that no instrument can
tell that reading from the planted one above.

**Minimal change.** Bind the quotation the way `DocBox` binds the caption: require the cited booklet
to be the row's form's own instructions (the `cite_check::FORMS.instructions` join already holds that
mapping for every emitted form), and require the sentence to fall inside that booklet's `Line <N>`
block for the row's line — the booklets print `Line 5b.` / `Line 28.` headings, so the block is
enumerable from the extract rather than hand-listed. Land it with the plant above as its kill.

---

### I2 (Important) — the census's DOCUMENT set is a hand-written list with no join to the archive: dropping a whole archived form passes green

**Where.** `crates/xtask/src/box_census.rs:97-133` (`DOCUMENTS`) and
`crates/xtask/src/box_census.rs:688-726` (`every_printed_box_carries_exactly_one_entry`).

**What is wrong.** The module doc quotes the rule it is built to satisfy — *"a conformance KAT must
enumerate the expected line set FROM the form's extracted text, never from a range or a hand-written
list"* — and satisfies it **for boxes**. One level up, the set of *documents* censused is a hand-written
`const DOCUMENTS`, and nothing joins it to `MANIFEST.json`, to `design/forms/extract/`, or to
`archive_check`. The suite's only defences against a shrinking census are `total == BOXES.len()`
(which falls in step when both are edited) and `total >= 100` (a floor 15 boxes below the current
115 (`box_census.rs:715-720`)). So an archived information return can be dropped from the census, or a newly archived one
(1099-NEC, 1099-R, 1099-MISC, 1099-DA — the 1099-DA is already specced in
`design/SPEC_1099da_broker_reporting.md`) can simply never be added, in silence. This is the brief's
*"a census that skips a form"*, and it is also the shape `CLAUDE.md` records getting wrong twice in
one sitting.

**Evidence — planted, observed, reverted.** Removed the `f1098e` `DocumentAuthority` and its two
`BoxEntry` rows (nothing else):

    $ cargo run -q -p xtask -- box-census
      f1099b--2025 (i1099b instructions): 22 boxes — 4 collected, 0 refuse-if-nonzero, 18 not read
      f1098--2025  (i1098 instructions):  11 boxes — 1 collected, 0 refuse-if-nonzero, 10 not read
    box-census OK: 113 printed boxes across 6 archived information returns, every one decided

    $ cargo nextest run --locked -p xtask -E 'test(box_census)'
    Summary [0.003s] 6 tests run: 6 passed, 133 skipped

Form 1098-E — the whole student-loan-interest document — vanished from the census and both the
operator command and all six census tests reported success.

**Minimal change.** Derive the document set instead of listing it: enumerate the archived information
returns from `MANIFEST.json` (`kind == "form"` whose extract carries the `PREAMBLE_END` face-block
marker is already a sufficient discriminator — it is present in exactly the seven and measured once
each), and assert `DOCUMENTS` equals that set in both directions. Kill: delete one `DocumentAuthority`
and watch it red; add an eighth information return to the archive and watch it red until censused.

---

### I3 (Important) — the T2 row's round-trip kill is a one-off manual check of 3 of 14 documents, and §6 does not name the gap

**Where.** `design/SPEC_interview.md:1190` (the T2 row's kill list, last clause: *"the archive
round-trip hash matches the note"*); the implementer's report §1 *"Round-trip verified against
irs.gov — Three notes re-fetched"*; and §6 *"Not done, and why"*, which does not mention it.

**What is wrong.** The spec states the round-trip as a **kill** — this repo's defined term for *"the
test that reds when the guarantee is removed"* — and the T2 cadence is *"per-revision (periodic)"*.
What landed is a manual re-fetch of **3 of 14** documents performed once, with no test and no
scheduled re-check. `authority_manifest::tests::every_manifest_entry_resolves_and_hashes_true` covers
the *local* copy only (and, in a fresh worktree where the gitignored PDFs are absent, covers nothing
for these 14 — it passed here with zero PDFs on disk). Nothing anywhere compares a note's hash to what
irs.gov serves today.

This is what makes C1 permanent rather than transient: three of the archived documents were revised
before this build landed and six annual editions were superseded, and no instrument in the repo can
say so. The notes carry the right instinct — *"★ A DIFFERENT hash does not mean a corrupt download —
it means the IRS REVISED this document"* — with no reader.

I discharged the fact myself for this review: **14/14 re-fetch to the byte and hash true right now**,
so the archive is internally honest. The finding is the missing instrument, not a wrong hash.

**Minimal change.** Either (a) an `xtask authority-refresh --check` that re-fetches every
`storage: note` entry's URL and reports hash drift, network-gated and run on demand rather than in the
suite (with a kill that plants a mutated note hash and watches it red); or (b) if that is T5's or a
follow-up's, say so in §6 with the reason, and file it — the spec lists it as a T2 kill, so it may not
go silently missing.

---

### M1 (Minor) — `DocumentAuthority::year`'s doc comment miscounts the documents it documents

`crates/xtask/src/box_census.rs:86-88`: *"**Six of the seven documents are `--2024`**: the
1099-INT/DIV/G family is continuous-use…"*. `DOCUMENTS` holds seven **forms**, of which **three** are
`--2024` (`f1099int`, `f1099div`, `f1099g`); the other four are `--2025`. The "six" is the count of
continuous-use *documents* (three forms + three booklets) out of the fourteen archived — a different
denominator. `box-census`'s own output prints the correct split. A hand-count in prose contradicting
the tool's output, in the file whose doc quotes *"never hand-count what a tool can count"*.

### M2 (Minor) — Form 8995 line 12's retag to `Combine` rests on a sentence that names no lines, and nothing gates it

Seven of the eight retags check out against the line's own quoted instruction text: 6251 2a/2b
(*"enter the amount from … line 12"*, *"from Schedule 1 (Form 1040), line 1 or line 8z"*) → `Carry`;
6251 4 (*"Combine lines 1 through 3"*), 6251 9 (*"Subtract line 8 from line 7"*, no clamp — the enum's
stated trigger is the absence of a clamp, not the verb) and Sch C 28 (*"Add lines 8 through 27b"*) →
`Combine`; 6251 11 (*"…If zero or less, enter -0-"*) → `Clamped(FloorAtZero)`; Sch SE 2 (*"Net profit
or (loss) from Schedule C, line 31"*) → `Carry`. The eighth, Form 8995 line 12 — *"Enter your net
capital gain, if any, increased by any qualified dividends"* (`f8995--2024.txt:49`) — names **no line
of any form**; its operands are Form 1040 line 3a and the Schedule D net capital gain, so `Carry` (or
`Exception` with a reason) fits the row better than `Combine`, whose documented blankness rule
(*"blank iff every operand is blank"*) has no operands to range over here. Rule (4)
(`line_coverage_check.rs:817-824`) only rejects a `Combine` whose quote contains `-0-`, so nothing in
the suite can hold this either way. No emitted figure changes — `Production` has two readers, the
checker and one `== Production::Exception` filter — so this is Minor; it is filed because the commit
message and the report both claim *"each re-tagged to the production its own quoted sentence
dictates"*, and for this row the sentence dictates nothing.

### M3 (Minor) — the enumerator sees only LABELLED boxes, and that limit is not declared

`printed_boxes` keys on `is_label` (a digit run, or `a`–`f`), so a printed box carrying no label is
outside its reach: the 1099-INT's **FATCA filing requirement** checkbox and the *2nd TIN not.* box
both print on the face (`f1099int--2024.txt:47`, `:51`) and appear in no census. Neither reaches a
1040 line, so nothing is wrong today — but the module doc declares exactly one limit (the wrapped
caption) and this one belongs beside it, because the census's whole claim is *"every box the form
PRINTS"*. Related: `is_label` accepts single letters only through `f`, so a future form lettering past
`f` would be invisible and the letter-contiguity guard (which builds its expected set from the
*largest letter found*) would not notice.

### M4 (Minor) — the face-block bound is already absent from the next W-2 revision

`PREAMBLE_END = "1141, 1167, and 1179"` is present exactly once in each of the seven archived extracts
(verified: 1 occurrence each, 7 total). It is present in `f1099g--2026`, `f1099b--2026` and
`f1098e--2026` — but **absent from `fw2--2026`**, whose preamble was rewritten to *"See IRS Publication
1141 … See IRS Publication 1223 …"*. The refresh C1 asks for will therefore hard-fail the enumerator on
the W-2. It fails *loudly* (`"the preamble marker … occurs 0 times, expected exactly 1"`), which is the
right behaviour and why this is Minor rather than blocking — but the marker should be recorded as
revision-fragile beside the rule that uses it, so the next archiver reads it as expected work rather
than as a broken tool.

### N1 (Nit) — 7 `xtask` tests fail in any PDF-less worktree; not a T2 regression

`cargo nextest run --locked -p xtask --no-fail-fast` here: **138 run, 131 passed, 7 failed, 1 skipped**.
All seven are environment, not code: six `form_delta` tests fail with `no PDF found for
f6251--2026-DRAFT` and friends (the archived PDFs are gitignored, so a reviewer's worktree has zero of
them — `ls design/forms/*/*.pdf | wc -l` → 0), and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fails because
`CARGO_TARGET_DIR` is redirected to `target-review` and the hook looks for its binary at the default
path. Every `box_census`, `line_coverage_check`, `archive_check` and `authority_manifest` test passes.
Recorded so a future reviewer in a worktree does not read them as regressions — and because the review
brief's own setup instructions (`export CARGO_TARGET_DIR=…`) deterministically produce one of them.

---

## Seams checked clean (no finding)

- **Seam 1, the archive.** 14/14 URLs answer; 14/14 live sha256 equal the note *and* the manifest
  entry; 14/14 byte counts equal; 14/14 extracts `cmp`-identical to `pdftotext` of the fetched bytes
  (forms with `-layout`, instructions with no flags, matching `design/forms/README.md`); 7/7 geometry
  fixtures carry the true `pdf_sha256` with the box/page counts the report states (272/11, 163/7,
  41/4, 21/4, 127/7, 140/6, 100/6); `authority-manifest` OK at 153 entries with the kind split the
  report prints. Every revision claim is at the extract line cited, including the two recorded
  deviations (1099-G = March 2024; `i1099int` = the combined 1099-INT/OID booklet).
- **Seam 2, the `i1098et` join.** `box_census::DocumentAuthority { stem: "f1098e", instructions:
  "i1098et" }` resolves and is printed in `box-census`'s per-document line and in rule (7)'s refusals;
  the report's reason for not putting it in `MANIFEST.json` (no `instructions` field exists there;
  `cite_check::FORMS` is *"every IRS form btctax EMITS"* and btctax emits none of these) is verified
  correct — `MANIFEST.json` entries carry exactly `path/kind/storage/sha256/bytes/url/extract`.
- **Seam 3, the `DocBox` check.** Four plants on the committed table, each red with the right message:
  a one-character caption drift (*"but the extract prints"*), a box the form does not print (*"which
  that form does not print"*), an unarchived document (*"no such information return is archived"*),
  and a `FilerRecords` sentence absent from the extract (*"NOT FOUND in that extract"*). The
  "neither" state is genuinely unrepresentable — `CollectedFrom` has exactly two variants — so M6's
  kill is structural, as claimed.
- **Seam 4, the box enumeration.** Spot-read three extracts myself: Form 1098 prints 1–11 (census 11),
  Form 1099-INT prints 1–17 (census 17), Form W-2 prints a–f, 1–11, 12a–12d, 13–20 (census 29). Three
  plants red correctly: a deleted entry (*"we forgot this box"*), an entry for an unprinted box
  (*"the extract does not print it"*), and a one-character drift (*"caption does not match the
  extract"*). The empty-caption entries (W-2 box 9, 12b–12d) are the right call.
- **Seam 5, pinned numbers.** Re-measured: manifest 153 (form 63 / instructions 37 / guidance 22 /
  publication 6 / regulation 6 / statute 19) ✓; `ls design/forms/*/*.pdf.txt | wc -l` → **98**, matching
  the corrected README ✓; xtask 139 total (138 run + 1 skipped) ✓; `line-coverage` 341/24/0/12 unmoved
  ✓; no file added under `crates/btctax-forms/forms/` ✓; no ratchet constant touched in the diff ✓.
- **Seam 6, the `archive_check` W-series fix.** The finding is real and the narrow arm is right:
  `the_w_series_stems_are_primary_sources` passes here, and the arm is `w` + at least one digit, so
  `fw.pdf` / `iw.pdf` / `index.html` still classify `None`. §6's other deferrals (no `FieldId` join,
  no document screens, no instructions geometry, no `cite_check::FORMS` rows, the wrapped-caption
  limit, the untouched README step-2/3 paragraphs) are each named with a reason that checks out.

---

Counts: C=1 I=3 M=4 N=1
