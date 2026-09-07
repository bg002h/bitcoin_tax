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
| **periodic** — revised every few years, "Rev. Month Year" on the face | archived under **the tax year its revision governs**, `{TY}/{stem}--{TY}.pdf.txt`, sourced in this order: `irs-prior/{stem}--{TY}.pdf` when the IRS holds that edition; else the bundled runtime asset (the `extract/f8283--2024.txt` model); else the **current** `irs-pdf/{stem}.pdf`, hashed at fetch time, with the note recording that the URL is a moving one. ★ The third case is real, not theoretical: `irs-prior/f8275r--2025.pdf` is **404** while `irs-pdf/f8275r.pdf` is **200** (checked 2026-09-04), which is the situation §G-12's unblock command lands in | `f8275`, `f8283` |

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
verified by round-trip (fetch `f8995--2025.pdf`, hash it, compare to the note: match; and on
2026-09-04 `f8275--2024`, `i8275--2024`, `f8283--2025`, all three HTTP 200 and hash-exact,
when `periodic/` was retired into the year directories).

    design/forms/MANIFEST.json     every document: source URL, sha256, size  (the provenance record)
    design/forms/2022/*.pdf.txt    the Form 1098 revision in force for TY2022–TY2024 (see below)
    design/forms/2024/*.pdf.txt    TY2024 notes — what btctax ships today
    design/forms/2025/*.pdf.txt    TY2025 notes — the B3 target
    design/forms/2026/*.pdf.txt    TY2026 notes — the target year; `-DRAFT` names are IRS drafts
    design/forms/extract/*.txt     ★ THE COMMITTED TEXT LAYER — what everything actually reads

★★ **A different hash is not a corrupt download — it means the IRS REVISED the document.** That is a
change to the *authority*: review it, never silently absorb it. This is the one thing the manifest exists
to make impossible to miss.

★★★ **And now it has a reader: `cargo run -p xtask -- authority-refresh --check`.** It re-fetches every
`storage: note` entry's own URL, hashes it, and reports drift — plus, for every information return, it
probes `irs-prior/<stem>--<newest archived + 1>.pdf` and reports a 200 as *a newer edition exists*.
Network-gated and **on demand**: it is never in `make check` and never in the suite (what the suite
holds is the pure drift comparison and its planted kill). Before it existed, the archive sat a whole tax
year behind its own target with every instrument printing OK — the seam review's I3.

★ **An edition the IRS no longer serves is archived from the Internet Archive's capture of the IRS's
own URL, and the note says so.** Form 1098 went continuous-use after the 2021 annual edition, and the
revision in force for TY2022–TY2024 is **Rev. January 2022** — which `irs-prior` does not hold
(`f1098--2022/2023/2024.pdf` are all 404 and the IRS picklist jumps 2021 → 2025, both measured
2026-09-07). Those are the bytes the IRS served at its own moving `irs-pdf/f1098.pdf` URL for three
years, recovered from a capture of that URL, with the Wayback digest's constancy recorded in the note.
Archiving the 2021 edition instead would have been a false answer: an ANNUAL edition governs its own
tax year and nothing later.

## What is NOT done yet

Archived ≠ extracted ≠ conformance-tested. These PDFs are step 1 of three:

1. **archived** — done: **115** documents recorded, all in `design/forms/` (measured
   `ls design/forms/*/*.pdf.txt | wc -l`, 2026-09-07), each as a URL note plus its extracted text.
   ★ The older `design/amt-form6251/` holds **no** notes — it was retired as an archive on
   2026-07-30.

   ★★ **This count said 60 and had been stale for some time** — 84 notes were on disk at
   `d49de0c7`, before the interview T2 archive added its 14 (the seven information returns
   `fw2` / `f1099int` / `f1099div` / `f1099g` / `f1099b` / `f1098` / `f1098e` and their
   instructions, `iw2w3` / `i1099int` / `i1099div` / `i1099g` / `i1099b` / `i1098` / `i1098et`).
   A hand-maintained count in prose is exactly the thing `CLAUDE.md` says never to hand-count;
   the command that produces it is now written beside it.
2. **extracted** — text layer committed as an in-crate fixture (`xtask extract-schedule-1a` is the model;
   `-layout` for a form, plain `pdftotext -f N -l M` for 3-column instruction pages). Done for
   Schedule 1-A only.
3. **conformance-tested** — label census from the extract, decisions derived from each line's own text.
   Done for Schedule 1-A only.

`cite_check::AUTHORITY_NOT_YET_ARCHIVED` shrinks when a form reaches **step 2**, not step 1 and not
step 3 — and that is the ratchet's own definition rather than a choice: `archived_form_years()`
counts a `(form, year)` as covered exactly when a `FORMS` row names it AND both committed fixtures
are on disk, so `authority_coverage_may_only_improve` REDS on an excuse for a pair whose extract
exists. Shrinking it on **step 1** would be the false-completeness this archive exists to prevent —
an archived PDF nothing can check against. Step 3 has its own instruments (the label census, the
derive-the-decision-from-the-line tests), and a form sitting at step 2 is honestly described by this
list as extracted-but-not-yet-conformance-tested.

★ Corrected 2026-09-06 (residue sweep 1, item 3), when `f4868` and `f1040v` reached step 2 for both
bundled years: this paragraph said "stays as it is until a form reaches step 3", which the ratchet
makes impossible — adding the fixtures turns the excuse stale and fails the test.
