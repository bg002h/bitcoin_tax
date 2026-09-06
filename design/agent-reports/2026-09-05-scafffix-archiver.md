# scafffix — the archiver archived the WRONG FORM

**Agent:** scafffix-archiver. **Date:** 2026-09-05. **Files owned and touched:**
`scripts/archive_drafts.py`, files under `design/forms/2026/`, and (per the brief's explicit
authorisation) one fixture under `design/forms/geometry/`.

**Status: FIXED.** The year reader now reads the form's own declared year from two independent
places that must agree; three wrongly-archived artifacts are deleted; the checker ships with a
`--self-test` whose 13 planted defects were each watched going RED.

**One thing found and NOT fixed is more serious than the bug I was sent to fix** — see §6, the
`Storage::Note` blindness. It is not in a file I own.

---

## 1. What was wrong (measured)

### 1.1 The defect as briefed — confirmed

`scripts/archive_drafts.py:85` read

```python
years = [int(y) for y in re.findall(r"\b20(?:1[5-9]|2[0-9])\b", out)]   # out = pages 1-3
return max(years) if years else None
```

Every Form 1040 carries line 36, *"Amount of line 34 you want applied to your 2026 estimated
tax"*, so a TY2025 Form 1040 contains the token `2026` and `max()` returned 2026.

Measured on the archived file before deletion:

| reader | result |
|---|---|
| masthead (25.2pt year box, first form page) | **2025** |
| footer stamp `Form 1040 (YYYY)` ×3 | **2025** |
| shipped `printed_year()` (`max` over pages 1-3) | **2026** — passed the check |

The only `2026` in the whole document was line 36. `sha256 d57f1ee32a37…` — and the live IRS draft
URL still serves exactly those bytes today (verified by refetch, §3).

### 1.2 It is the ONLY form on which `max()` could fail that way

I did not take this on trust. I ran both new readers over **every one of the 82 PDFs** committed
under `design/forms/{2024,2025,2026}/`. Exactly one file disagreed with its own filename:
`design/forms/2026/f1040--2026-DRAFT.pdf`. The other 15 TY2026 drafts declare 2026 in both places.

### 1.3 A second defect the brief did not name: the wrong REFUSAL REASON

`f8275` and `f8283` were being bucketed as *"the draft at that URL prints None, not 2026"*. That
diagnosis is wrong. Both are **revision-dated, not annual**:

```
f8275--2024.pdf   Form 8275   (Rev. October 2024)     Disclosure Statement
f8283--2025.pdf   Form 8283   (Rev. December 2025)    Noncash Charitable Contributions
f8275r--2025.pdf  Form 8275-R (Rev. November 2024)    Regulation Disclosure Statement
```

They carry no tax-year masthead at all, so there is no year to check and never will be. A refusal
that reports the wrong reason costs someone an afternoon — the module's own docstring already says
so about a different case. Now reported as `revision-dated, not annual: (Rev. October 2024) — this
form has no tax year to check`.

---

## 2. What I changed

`scripts/archive_drafts.py` (147 → 548 lines). The year is now read from the two places where the
form declares its **own** identity, and they must agree with each other **and** with the year asked
for. A year in a line's prose is never read.

### 2.1 The masthead reader — structural, not a heuristic

The masthead year is the token that is simultaneously:

* **a BARE year** — every prose and revision-date occurrence carries adjacent punctuation and
  tokenises as `2025,` `2025.` `2024)`, so `^20\d\d$` excludes them *by construction*, not by luck
  (verified: the `(Rev. October 2024)` token is literally the word `2024)`);
* **in the top fifth of the page** — measured maximum over all 82 PDFs: yMin **49pt** of 792
  (6.2%); the window is 20%, deliberately far looser than the measurement;
* **larger than the page's median word height** — measured margin **2.2×–3.2×** (form mastheads
  25.2pt against a 9.3–10.5pt median; instruction booklets 31.3–45.4pt against 14.3pt).

Of the survivors the tallest wins; **two different years tied at the top REFUSE rather than pick.**
Three independent reasons select the same token, so no single one is load-bearing alone.

### 2.2 The footer stamp reader

`Form 1040 (2025)`, `Schedule SE (Form 1040) 2026`, `Schedule 1-A (Form 1040) (2026)` — the IRS
prints it on every page. Measured over all 82 PDFs it yields **exactly one (designation, year) pair
per document, with zero strays**; prose references such as `Schedule K-1 (Form 1041)` and
`Schedule A (Form 8936), Part II` carry no year and cannot match. All occurrences must agree.

### 2.3 The adjudicator returns a REASON CODE

`adjudicate()` returns `(accepted, code, detail)` with codes `ok` / `no-masthead` /
`stamps-disagree` / `no-stamp` / `masthead-vs-footer` / `wrong-year`. The kill table asserts the
**code**, not merely accept/reject — see §4.2 for why that mattered.

### 2.4 `first_form_page` is derived, not assumed

`"NOT FOR FILING"` appears on the draft cover and on no form page (measured: page 1 yes, pages 2-3
no). A final served from a non-draft path answers 1. If every page looks like a cover, the caller
refuses by name.

---

## 3. Re-run over all 18 stems, live against irs.gov

```
archived 15  ·  refused 3  ·  refused-not-a-draft 0  ·  absent 0
```

**Genuinely TY2026 (15):** `f1040s1 f1040s1a f1040s2 f1040s3 f1040sa f1040sb f1040sc f1040sd
f1040sse f6251 f8949 f8959 f8960 f8995 f8995a` — each with masthead **and** footer stamp reading
2026.

**Refused (3):**

| stem | reason |
|---|---|
| `f1040` | the document declares **2025**, not 2026 — the IRS draft path still serves the TY2025 form |
| `f8275` | revision-dated, not annual: `(Rev. October 2024)` |
| `f8283` | revision-dated, not annual: `(Rev. December 2025)` |

**The 16 files already on disk are byte-identical to today's live fetch** (sha256 compared for all
16), so the 15 good archives needed no rewrite and I did not churn them.

**All 15 remaining geometry fixtures verify** against their PDFs' sha256.

---

## 4. Which test reds for which planted defect

`.venv/bin/python scripts/archive_drafts.py --self-test` — four tables, all enumerated from the
filesystem or from the mechanism, never from a hand-list:

```
  reader kills ...                                    ok — 7 rows
  decision kills ...                                  ok — 8 rows
  end-to-end kill on a real document ...              ok — 2 rows
  negative fixtures (temp dir, never the archive) ... ok — 3 rows
  corpus audit (design/forms/*/f*.pdf) ...            ok — 46 declared their own year,
                                                      3 excused as revision-dated:
                                                      f8275--2024, f8275r--2025, f8283--2025
```

### 4.1 The plant table — 15 planted, **13 RED**

| # | planted defect | result | which row reds |
|---|---|---|---|
| **A** | **reader reverts to `max()` over pages 1-3 — the shipped defect** | **RED** | corpus audit: `design/forms/2025/f1040--2025.pdf: the masthead says 2026 but the footer stamp says 2025` (+ `f1040--2024` and 3 more) |
| B | combiner ignores a masthead/footer contradiction | RED | decision kill `masthead matches the year asked for but the footer stamp contradicts it` |
| C | combiner accepts a document with no masthead year | RED | decision kills `no masthead year at all`, `ambiguous masthead` |
| D | reader drops the type-SIZE guard | RED | reader kill `SIZE guard: a body-sized bare year at the top is not a year box` |
| E | reader drops the TOP-OF-PAGE guard | RED | reader kill `TOP-OF-PAGE guard` |
| F | reader drops the BARE-year anchor | RED | reader kill `BARE-year guard` (raises `ValueError: invalid literal for int(): '2025,'`) |
| G | reader picks a tied-ambiguous masthead | RED | reader kill `two different years tied at the top size REFUSE rather than pick` |
| H | classifier stops reporting PROBLEMs | RED | negative fixture `a TY2025 Form 1040 named --2099 classified as 'declares'` |
| I | footer stamps may disagree with each other | RED | decision kill `footer stamps disagree with each other` |
| J | classifier excuses everything (silent skip) | RED | negative fixtures 1 and 3 |
| K | corpus glob narrowed to one year (false completeness) | RED | `corpus audit found only 15 form PDFs — the glob is not finding them` |
| L | excuse widened to "any unreadable masthead" | RED | negative fixture `an unreadable PDF classified as 'excused' — 'I could not check this' was treated as 'this is fine'` |
| Q | `first_form_page` returns the DRAFT COVER | RED | corpus audit, 15 TY2026 files |
| O | e2e kill passes silently when its witness is absent | *green* | see §4.3 |
| P | negative fixtures pass silently when sources absent | *green* | see §4.3 |

**Plant A is the one that matters**: the exact shipped reader, killed by committed evidence.
`design/forms/2025/f1040--2025.pdf` carries the same line 36, so the original check would have been
caught by this self-test the day it was written.

### 4.2 Two of my own kill rows were themselves blind — found by planting, not by reasoning

This is the finding I would most want a reviewer to see, because it is the B1 disease **inside the
kill table**:

* The row named *"masthead disagrees with footer stamp"* used masthead=2025 / want=2026. Deleting
  the contradiction check left `wrong-year` to refuse it, and **the row stayed GREEN**. It had
  never once exercised the mechanism it was named after. Fixed by adding a row where the masthead
  *matches* the year asked for, so only the contradiction check can refuse.
* The row *"footer stamps disagree with each other"* refused via `next(iter(set))` — an arbitrary
  hash-order pick — so it went green or red depending on which element came out first.

Both are now fixed by asserting the **reason code**, and both re-verified RED under plants B and I.
Neither would have been found by reading the code.

### 4.3 The two plants that stayed green, stated plainly

O and P mutate `if not pdfs: return [...]` / `if not src.is_file(): return [...]` — branches taken
only when the witness PDFs are **absent**, which they are not. They are fail-closed guards for a
situation that does not currently obtain, not checks with a live discrimination. I am not claiming
them as witnessed. The reachable behaviour they guard (a check that cannot run must FAIL, never
pass) is exercised by plants H, J and L on the same principle.

A related honest limit: the corpus audit is a **regression check over a clean corpus**, so mutating
its reporting path cannot be witnessed against the archive alone. That is exactly why
`_negative_fixture_kill` exists — it drives the *same* `classify_form_pdf` implementation with
deliberately broken files in a `tempfile.TemporaryDirectory()`. Nothing is ever written into
`design/forms/`, because manufacturing a mislabelled document inside the authority archive is the
thing this script exists to prevent.

---

## 5. Files deleted, and why

| file | bytes | why |
|---|---|---|
| `design/forms/2026/f1040--2026-DRAFT.pdf` | 307,729 | It is the **TY2025** Form 1040. Masthead *"1040 U.S. Individual Income Tax Return 2025"*, footer *"Form 1040 (2025)"* ×3. |
| `design/forms/2026/f1040--2026-DRAFT.pdf.txt` | 26,927 | Its text layer — the committed half of the pair (the `.pdf` is gitignored). A mislabelled authority document is worse than a missing one. |
| `design/forms/geometry/f1040--2026-DRAFT.json` | 186,293 | Geometry fixture pinned to `pdf_sha256 d57f1ee32a37…`, i.e. faithfully derived from the wrong document. |

Backed up (not committed) to
`…/scratchpad/deleted-f1040-2026/` in case the controller wants the bytes.

**Nothing else was deleted.** All 15 other TY2026 archives and their fixtures verify.

### 5.1 ★ Consequence the controller must act on — `MANIFEST.json`

`design/forms/MANIFEST.json` entry **[63]** still names the deleted document:

```json
{"path": "design/forms/2026/f1040--2026-DRAFT.pdf", "kind": "form", "storage": "note",
 "sha256": "d57f1ee32a374ec628906a346cd6de1213fdc169d83f8d4eaf48912f41b02e6d", ...}
```

`MANIFEST.json` is **not a file I own**, so I did not edit it. Measured consequence, before and
after my deletion:

```
before:  authority-manifest: OK — every entry resolves and every source is listed
after:   xtask authority-manifest: AUTHORITY MANIFEST — 1 problem(s):
           design/forms/2026/f1040--2026-DRAFT.pdf — gitignored, and its `.txt` provenance
           note is missing; nothing records where it came from
```

That red is **correct** — the manifest claims a TY2026 Form 1040 that does not and should not
exist. The entry must be removed (or `xtask authority-manifest --regen` run) by whoever owns that
file. Until then `make check` will carry one red from my change.

---

## 6. Found, NOT fixed — and this one is worse than the bug I was sent for

### 6.1 `Storage::Note` verification never reads the note (instrument fails OPEN)

`crates/xtask/src/authority_manifest.rs:283-289`:

```rust
Storage::Note => {
    // The binary is deliberately absent; the NOTE is the artifact that must survive.
    let note = root.join(format!("{}.txt", e.path));
    if !note.is_file() {
        out.push(Problem::MissingNote(e.path.clone()));
    }
}
```

The check is **`note.is_file()` and nothing else.** It never opens the note, never looks for a URL,
never looks for a sha256. Any file at that path passes.

### 6.2 …and `archive_drafts.py` writes something that is not a note

The convention in `design/forms/2024/` and `design/forms/2025/` is that `<name>.pdf.txt` is a
**737-byte provenance note**:

```
https://www.irs.gov/pub/irs-prior/f1040--2025.pdf

# f1040--2025.pdf — IRS primary source, NOT committed (publicly available; keeps the repo small).
# sha256  3d31c226df0d189ced80e039d01cf0f8820c1019681a0f0ca6264de277b7e982
# bytes   220237
```

…and the committed **text layer** lives separately in `design/forms/extract/<name>.txt`.

`archive_drafts.py:127` instead does `pdftotext -layout <dest> <dest>.txt`, so every file in
`design/forms/2026/` is a **full text layer occupying the note's filename slot** — 230,890 bytes
across 16 files, line 1 of each reading *"Note: The draft you are looking for begins on the next
page."* There is **no URL and no `sha256:` line anywhere in the 2026 directory**, and
`design/forms/extract/` contains **zero** 2026 entries.

The two defects compose: the manifest guard is satisfied by mere existence, so 16 entries have no
recoverable provenance and nothing reports it. In a fresh clone or CI — where the gitignored PDFs
are absent — those notes cannot restore anything.

**Why I did not fix it.** The `.pdf.txt` files are being consumed *as the text layer* by other
agents' in-flight work (`design/agent-reports/2026-09-05-scaffolding-compute.md` reads
`design/forms/2026/f1040--2026-DRAFT.pdf.txt` directly), and the fix spans `MANIFEST.json`,
`design/forms/extract/` and `authority_manifest.rs` — three files I do not own. Changing the
output format mid-flight would break work in progress. **This needs an owner and its own task.**

### 6.3 Two supported forms have no year-scoped draft at all

`f8275` and `f8283` are emitted by btctax but are revision-dated, so the `{stem}--{TY}-DRAFT`
naming scheme cannot express them. `design/forms/README.md` already describes a periodic cadence
for these under `irs-prior`/`irs-pdf`; the draft archiver has no corresponding path. Refusing them
is correct today, but a TY2026 port will need their current revisions from somewhere.

### 6.4 Not re-verified by me

`design/TY2026_PORT_REPORT.md` §2 row 7 and
`design/agent-reports/2026-09-05-scaffolding-compute.md` C-1 both describe this defect correctly,
but the compute lens's downstream conclusions built on the f1040 comparison (the work-list row
`| f1040 | 199 | 0 | 0 | 31 | port |`) rest on a TY2025-vs-TY2025 diff. **The TY2026 Form 1040's
line numbering remains unknown**, because the IRS has not published that draft. Anything scheduled
against it is scheduled against nothing.

---

## 7. Reproduce

```
.venv/bin/python scripts/archive_drafts.py --self-test        # 5 tables, exit 0
.venv/bin/python scripts/archive_drafts.py 2026 --dry-run     # 15 ok, 3 refused (network)
./target/debug/xtask authority-manifest                       # 1 problem — the dangling entry, §5.1
```
