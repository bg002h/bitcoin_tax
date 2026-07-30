# The form archive — primary sources, hash-pinned

**Every form btctax emits, with the IRS instructions that define it.** This is the authority. An oracle
is a witness; our inference is nothing. See `CLAUDE.md` and `FOLLOWUPS.md` §G-11.

## Why this exists

btctax emitted 16 forms while holding the defining PDF for 4, and nothing made that visible. For an
unarchived form the conformance machinery does not *fail* — `xtask cite-check`, the
derive-the-decision-from-the-line assertions and the label census all **pass by finding nothing**.

## The acquisition scheme is MECHANICAL — that is the point

Adding a form, or a new tax year of an existing one, is a manifest entry. Two cadences, and the
distinction is structural rather than incidental:

| cadence | where it lives | example |
|---|---|---|
| **annual** — one revision per tax year | `https://www.irs.gov/pub/irs-prior/{stem}--{year}.pdf` | `f6251--2025.pdf` |
| **periodic** — revised every few years, "Rev. Month Year" on the face | `https://www.irs.gov/pub/irs-pdf/{stem}.pdf` (no year edition exists) | `f8275`, `f8283` |

Instructions follow the **identically-numbered** convention: form `fNNNN` has instructions `iNNNN`
(`f6251`→`i6251`, `f1040sa`→`i1040sca`), with `i1040gi` carrying the 1040-family schedules that get no
standalone booklet (Schedule 1-A, Schedules 2 and 3).

★ A missing year is sometimes the *correct* answer and must not be treated as a fetch failure:
**`f1040s1a--2024.pdf` does not exist because Schedule 1-A was created by Pub. L. 119-21 for TY2025.**

## ★ The PDFs are NOT committed — the notes and the text layer are

IRS forms are public documents, so committing 18 MB of them buys nothing and roughly doubled the repo.
Instead, beside where each PDF belongs sits a **`<name>.pdf.txt` note** carrying its irs.gov URL, sha256
and size, and the **committed text layer** lives in `design/forms/extract/`.

That is what makes it work: **the conformance tests read the extract, so they need no PDF and no
network.** The PDF is only needed to *re-extract*, and the note is sufficient to reproduce it —
verified by round-trip (fetch `f8995--2025.pdf`, hash it, compare to the note: match).

    design/forms/MANIFEST.json     every document: source URL, sha256, size  (the provenance record)
    design/forms/2024/*.pdf.txt    TY2024 notes — what btctax ships today
    design/forms/2025/*.pdf.txt    TY2025 notes — the B3 target
    design/forms/periodic/*.pdf.txt  non-annual revisions
    design/forms/extract/*.txt     ★ THE COMMITTED TEXT LAYER — what everything actually reads

★★ **A different hash is not a corrupt download — it means the IRS REVISED the document.** That is a
change to the *authority*: review it, never silently absorb it. This is the one thing the manifest exists
to make impossible to miss.

## What is NOT done yet

Archived ≠ extracted ≠ conformance-tested. These PDFs are step 1 of three:

1. **archived** — done: 66 documents recorded (57 here + the 9 in the older `design/amt-form6251/`),
   each as a URL note plus its extracted text.
2. **extracted** — text layer committed as an in-crate fixture (`xtask extract-schedule-1a` is the model;
   `-layout` for a form, plain `pdftotext -f N -l M` for 3-column instruction pages). Done for
   Schedule 1-A only.
3. **conformance-tested** — label census from the extract, decisions derived from each line's own text.
   Done for Schedule 1-A only.

`cite_check::AUTHORITY_NOT_YET_ARCHIVED` therefore stays as it is until a form reaches step 3. Shrinking
it on step 1 would be exactly the false-completeness this archive exists to prevent.
