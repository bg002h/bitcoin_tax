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

## Layout

    design/forms/MANIFEST.json     every archived PDF with its source URL, sha256 and size
    design/forms/2024/             TY2024 editions (what btctax ships today)
    design/forms/2025/             TY2025 editions (the B3 target)
    design/forms/periodic/         non-annual revisions

`MANIFEST.json` is the provenance record: re-fetching a URL and getting a different sha256 means the IRS
revised the document, which is a change to the authority and must be reviewed, never silently absorbed.

## What is NOT done yet

Archived ≠ extracted ≠ conformance-tested. These PDFs are step 1 of three:

1. **archived** — done here, 57 documents.
2. **extracted** — text layer committed as an in-crate fixture (`xtask extract-schedule-1a` is the model;
   `-layout` for a form, plain `pdftotext -f N -l M` for 3-column instruction pages). Done for
   Schedule 1-A only.
3. **conformance-tested** — label census from the extract, decisions derived from each line's own text.
   Done for Schedule 1-A only.

`cite_check::AUTHORITY_NOT_YET_ARCHIVED` therefore stays as it is until a form reaches step 3. Shrinking
it on step 1 would be exactly the false-completeness this archive exists to prevent.
