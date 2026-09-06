# Wave 2 — F5: the extract suffix collided with the provenance-note suffix

**Date:** 2026-09-05
**Scope:** `scripts/archive_drafts.py`, `crates/xtask/src/authority_manifest.rs`
**Status:** fixed. Every number below is measured, not estimated.

---

## 1. What was wrong

### 1a. One suffix, two meanings — measured

`authority_manifest::extract_for` resolves a text layer at exactly one path for a
`design/forms/<year>/<stem>.pdf` source:

    design/forms/extract/<stem>.txt

`scripts/archive_drafts.py` ran, per archived draft:

    subprocess.run(["pdftotext", "-layout", str(dest), str(dest) + ".txt"], check=False)

i.e. it wrote its text layer to `design/forms/<year>/<name>.pdf.txt` — the filename
`design/forms/README.md` reserves for the provenance **note**. Measured on the tree as found:

| | 2024 / 2025 | 2026 |
|---|---|---|
| `design/forms/<year>/*.pdf.txt` | 31 files, mean **738 bytes** — URL + sha256 + bytes | 15 files, mean **9,494 bytes** — a `pdftotext` dump |

### 1b. Both consequences, measured

**(i) The manifest under-reported the archive.** All **15** TY2026 entries carried
`"extract": ""` while their text layers existed. Nothing was at
`design/forms/extract/<stem>.txt`, so `extract_for` correctly found nothing and correctly
recorded nothing — *the first year whose text layers were fully extracted read, in
`MANIFEST.json`, as not extracted.*

(The port report says 16. **It is 15.** `f1040` was refused by the archiver's own year check
— the draft URL still serves a TY2025 document — and `f8275`/`f8283` are revision-dated and
refused with that diagnosis. All three refusals are correct; the count in
`TY2026_PORT_REPORT.md` §2 row for F5 is one high.)

**(ii) 15 documents had no recorded URL and no recorded hash, with every instrument green.**
`verify()`'s `Storage::Note` arm checked only that `<path>.txt` **existed** — and a dump
exists exactly as well as a note does. Measured over the 78 note-storage entries: **63 notes
carried a URL on line 1 and the entry's own sha256 verbatim; 15 carried neither.**
`xtask authority-manifest` printed *"OK — every entry resolves and every source is listed"*
throughout.

This was not merely a misplaced file: **the archiver never wrote a provenance note at all.**
The TY2026 URLs survived in `MANIFEST.json` only through the `previous_urls` fallback added
the same day.

---

## 2. What I changed

### 2a. `scripts/archive_drafts.py`

- **Two named destinations, derived not chosen.** `note_path(pdf)` → `<binary>.pdf.txt`;
  `extract_path(pdf)` → `<extract_dir>/<pdf.stem>.txt`, which is the *only* path
  `extract_for` reads. The stem keeps its `-DRAFT` marker, so `line_coverage_check`'s
  `<form>--<year>` lookup is structurally unable to land on a draft's text layer:
  `f6251--2026` is not `f6251--2026-DRAFT`.
- **`record(pdf, url, year, extract_dir=None)`** writes **both** artifacts and raises if they
  resolve to one file. It replaces the single `pdftotext … <dest>.txt` line in `main()`.
- **`note_text()`** emits the same shape the 2024/2025 notes carry (URL line 1, sha256,
  bytes, fetch + verify commands, pointer to the text layer), plus the draft-specific caution
  that the IRS replaces drafts in place.
- **`extract_text()`** emits the same `# GENERATED` header the 64 committed extracts carry,
  with a `# ★ DRAFT — evidence only` line and a working `# Regenerate:` command.
- **`--relayout <year>`** rewrites note + text layer for drafts **already archived**, with no
  network. It touches only `*--<year>-DRAFT.pdf`, so the hand-verified TY2024/TY2025 finals
  are out of reach, and a draft whose binary is absent is **reported by name and fails the
  run** rather than skipped.

### 2b. `crates/xtask/src/authority_manifest.rs` — two new `Problem` variants

- **`ExtractNotRecorded { path, found }`** — the entry records `extract: ""` while
  `extract_for` finds one on disk. This is the inverse of `MissingExtract`, which fires on a
  *name pointing at nothing* and is structurally unable to see a *file pointed at by nothing*.
- **`NoteIsNotAProvenanceNote { path, first_line }`** — a `<source>.txt` whose first non-empty
  line is not an `http` URL, or which records no 64-hex digest. **This is the bar against the
  collision recurring.** It is structural rather than a naming convention: a note vouches for
  bytes it does not contain, and a `pdftotext` dump cannot satisfy it without ceasing to be a
  text layer.

### 2c. The data

`.venv/bin/python scripts/archive_drafts.py --relayout 2026` then
`./target/debug/xtask authority-manifest --regen`.

| | before | after |
|---|---|---|
| files in `design/forms/extract/` | 64 | **79** (64 untouched + 15 new) |
| manifest entries | 128 | **128** (nothing dropped) |
| entries recording a text layer | 91 | **106** |
| TY2026 entries with `extract: ""` | **15** | **0** |
| mean `design/forms/2026/*.pdf.txt` | 9,494 B (dump) | **924 B** (note) |

The 64 pre-existing extracts were not written: `--relayout` globs `*--2026-DRAFT.pdf` only,
and the four non-draft extracts with today's mtime (`f8275r--2025`, `f8615--2025`,
`i8275r--2025`, `i8615--2025`) are 09:00 files from other work; my run was 20:25.

---

## 3. Which test reds for which planted defect

Every one of these was **run** and **observed red**, then reverted.

| # | plant | test that reds |
|---|---|---|
| 1 | `extract_for` also accepts `<rel>.txt` (the note path) | `authority_manifest::tests::a_text_layer_on_the_note_path_resolves_to_no_extract` **and** `…::every_problem_class_is_caught` |
| 2 | delete the `Problem::ExtractNotRecorded` push from `verify()` | `authority_manifest::tests::every_problem_class_is_caught` (plant 6 inside it) |
| 3 | disable the note-shape predicate — existence alone is a note again | `authority_manifest::tests::every_problem_class_is_caught` (plant 7 inside it) |
| 4 | **data plant:** blank `extract` on one TY2026 entry in `MANIFEST.json` | `…::no_manifest_entry_hides_a_text_layer_that_exists_on_disk`, `…::every_archived_draft_records_a_text_layer_that_still_says_draft`, `…::every_manifest_entry_resolves_and_hashes_true` — **and** `xtask authority-manifest` exits non-zero naming the file |
| 5 | `extract_path` returns the note path | `archive_drafts.py --self-test` → *"record refused to write — the provenance note and the text layer resolve to ONE file"* |
| 6 | **the shipped defect itself:** `record` writes the `pdftotext` dump onto the note path | `archive_drafts.py --self-test`, 4 rows: note does not start with the fetch URL / records no sha256 / is 14,485 bytes / contains the DRAFT cover sheet |

Non-vacuity is asserted where it can hide a pass:
`no_manifest_entry_hides_a_text_layer_that_exists_on_disk` fails if fewer than 50 entries
record a text layer (106 do); `every_archived_draft_records_a_text_layer_that_still_says_draft`
fails if the manifest holds no drafts; the collision kill fails outright if no archived draft
PDF exists to drive it. In plant 6 the *note* rows red while the extract rows stay green,
which is the discrimination being witnessed rather than a blanket failure.

**Seen red on the live tree, before the data fix** — the strongest evidence here:
`xtask authority-manifest` reported **15 problems**, one per colliding file, and the archiver
self-test reported 30 rows (a missing text layer and a non-note for each of the 15). Both are
green now.

---

## 4. Verification

| command | result |
|---|---|
| `cargo nextest run -p xtask` | **109 passed**, 1 skipped |
| `.venv/bin/python scripts/archive_drafts.py --self-test` | **PASSED** (7 + 8 + 2 + 3 + 6 rows + 46-form corpus audit) |
| `./target/debug/xtask authority-manifest` | `OK — every entry resolves and every source is listed` |
| `CARGO_TARGET_DIR=target-clippy cargo clippy -p xtask --all-targets -- -D warnings` | clean (one finding found and fixed: `unnecessary_lazy_evaluations`) |
| `cargo fmt -p xtask -- --check` | clean |

`design/forms/2026/f6251--2026-DRAFT.pdf` now reads:

    "url": "https://www.irs.gov/pub/irs-dft/f6251--dft.pdf",
    "extract": "design/forms/extract/f6251--2026-DRAFT.txt"

---

## 5. Found but NOT fixed

1. **★ There is no `sha_coverage_may_only_improve` ratchet, and the sha has the same hole the
   URL had.** `regen` at `authority_manifest.rs:886` falls back to `Err(_) => (note_sha, 0)`
   when the binary is absent — i.e. it reads the digest **out of the note**. Before this fix
   the 15 TY2026 "notes" contained no digest, so on any tree without the gitignored PDFs (a
   fresh clone, CI) a regen would have written `sha256: ""` and `bytes: 0` for all 15 while
   `regen_would_drop` — keyed on paths — saw nothing. This is the exact shape the file's own
   `a_regen_preserves_urls_it_cannot_rederive` comment describes for URLs, one field over, and
   the guard for it does not exist. It is moot for TY2026 now that the notes carry digests;
   the *guard* is still absent. (Claim from reading the code path, not run — I could not
   remove the PDFs in a shared worktree.)

2. **The 79 committed extracts still have no integrity pin.** The manifest hashes the PDF;
   `verify()` now checks the extract is *recorded* and *exists*, but never that its bytes are
   what was extracted. A hand edit to an extract silently moves every citation assertion that
   reads it. This is package F6 / port report §2.2 and is unchanged by this work.

3. **`design/forms/README.md` is stale and does not state the rule now enforced.** It lists
   only the 2024 and 2025 note directories, says *"archived — done: **60** documents"* against
   a measured 128 manifest entries / 78 note-storage, and nowhere says that
   `<source>.pdf.txt` is a note and only `design/forms/extract/<stem>.txt` is a text layer.
   The invariant is enforced in code now; the document a human reads still does not carry it.
   Not my file to edit.

4. **No TY2026 *instructions* draft is archived.** `STEMS` in `archive_drafts.py` holds 18
   stems and **zero** begin with `i`. The transcription rule wants `iNNNN` beside every
   `fNNNN`, and `i1040gi--2026` / `i6251--2026` are exactly the two the IRS publishes last.
   Adding them is a change to `STEMS`, not to the machinery — deliberately left, because the
   `--DRAFT` naming and the year check would need a review pass for instruction booklets
   (they carry no masthead year box in the same place as a form).

5. **`TY2026_PORT_REPORT.md` §2's F5 row says 16 entries.** Measured: 15. Not corrected — the
   report is not my file.
