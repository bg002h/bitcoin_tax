# Wave 2 · #14 — `Geometry::pdf_sha256` was pinned by nothing

**Agent:** wave-2 fixer, item #14 (`TY2026_PORT_REPORT.md` §2 row 14 / §"10 · Assert `geometry.pdf_sha256 == MANIFEST.sha256`").
**File owned and touched:** `crates/xtask/src/form_geometry.rs` (only).
**Date:** 2026-09-05.

## 1 · What was wrong (measured)

`Geometry::pdf_sha256` (now `:76`) is written by `extract()` at generation time and its doc comment
states the guarantee outright — *"A changed hash means the IRS REVISED the form — review it, never
regenerate silently."* Nothing read it.

Measured before the change:

| measurement | value |
|---|---|
| committed geometry fixtures (`design/forms/geometry/*.json`) | **47** (the report says 48; the directory holds 47 today) |
| fixtures whose `pdf_sha256` equals the `MANIFEST.json` sha256 of their PDF | **47 / 47** |
| fixtures whose PDF has **no** manifest entry | **0** |
| tests, commands or hooks comparing the two | **0** — `grep -n pdf_sha256` returned the field decl, the write in `extract()`, one truncated `[..8]` print, and a `String::new()` in an unrelated y-flip unit test |

So the pin was correct and unwitnessed. That is the expensive shape here, not a wrong value: 16 of the
47 fixtures are `*-DRAFT`, and the IRS replaces a draft **in place at the same URL** (the TY2026
Schedule 1-A draft was revised 2026-09-04). A revision leaves the filename, the `.pdf.txt` note and
the fixture all looking exactly as they did; the hash is the only thing that changes, and nobody was
looking at it. Every downstream witness — the label census, `label-proof`, the Schedule 1-A
conformance KAT — keeps passing against last year's geometry.

## 2 · What I changed

Two edits, both in `crates/xtask/src/form_geometry.rs`.

**(a) One rule for "which PDF does this fixture observe" — `pdf_rel_for_stem` (`:125`).**
The check needs a fixture stem → manifest path derivation, and `extract()` already had one. Rather
than write a second copy (two truths, and the check would then be verifying its own convention), the
derivation is factored out and `extract()` calls it. It delegates the year to
`label_reader::stem_year`, the same function `label-proof` resolves its PDF with — so a fixture, its
PDF and its manifest entry cannot be resolved three different ways.

This incidentally closes the second site of report item **#4**: `extract()`'s copy ended in
`.unwrap_or("2025")`, a fallback that can never fire (`rsplit` always yields at least one item) while
reading as a deliberate fallback-to-2025 policy. `label_reader.rs`'s own doc comment names
`form_geometry.rs:194` as "still to fix"; it is now gone. Behaviour change for a malformed stem: it
was a confusing *"design/forms/f1040/f1040.pdf not present, re-fetch it"*, it is now a refusal naming
the shape it wanted.

**(b) `mod pdf_sha_tests` (`:487`) — the check, plus its kill.**
A `Finding` enum with five variants and a pure `findings_for(entries, stem, geometry)`, so the
planted-defect tests can hand it defects that must never exist on disk:

- `Drifted` — the pin and the manifest disagree.
- `NoManifestEntry` — the fixture observes a document the repo does not track. **A finding reported
  by name, not a skip.**
- `Ambiguous` — two manifest entries claim one path, so "the" expected hash is undefined (a
  first-match lookup would have silently picked one).
- `NameDisagrees` — the fixture's own `form` field disagrees with its filename; the two identities
  the lookup uses are not the same document.
- `UnusableStem` — the filename is not `<form>--<year>[-DRAFT]`, so no PDF path exists for it.

Nothing is enumerated by hand: the fixture set is `read_dir(design/forms/geometry)` filtered to
`*.json`, and the expected hash is whatever `MANIFEST.json` files under the fixture's own derived PDF
path. Unreadable or unparseable fixtures **panic**; they are not skipped (the sibling cover-sheet
guard shipped with a `continue` there and silently skipped the defect planted to prove it worked).
Vacuity guards: `read_dir` panics on a wrong directory, `!paths.is_empty()` catches a wrong extension
filter, `!entries.is_empty()` catches a manifest that parsed to nothing, and
`assert_eq!(checked, paths.len())` catches a fixture dropped mid-loop.

I also extended the `pdf_sha256` doc comment to name the test that now enforces it, so the answer to
*"which test reds when this checker is removed?"* is in the field's own documentation.

## 3 · Which test reds for which planted defect

Run: `cargo nextest run -p xtask -E 'test(pdf_sha_tests)'` → **4 tests, 4 passed** (0.016s).

**Live plant on a real committed fixture** — `design/forms/geometry/f8275--2024.json`, one character
of `pdf_sha256` (`…5fa164` → `…5fa169`), backed up and restored, `sha256sum -c` OK afterwards:

`form_geometry::pdf_sha_tests::every_committed_geometry_fixture_matches_the_manifest` **FAILED** (the assert now at
`form_geometry.rs:677`, post-`rustfmt`; the run itself panicked at `:668` before the file was
formatted):

```
1 of 47 committed geometry fixtures are not observations of the document MANIFEST.json points at:
  f8275--2024: pdf_sha256 9b4b…fa169 but MANIFEST.json says 9b4b…fa164 for
  `design/forms/2024/f8275--2024.pdf` — the IRS REVISED the form (drafts are replaced in place) or
  the fixture was edited. REVIEW the revision against the ledger; do not regenerate silently.
```

**Permanent in-repo kills** (B1 — the checker lands paired with the defect it exists to catch):

| test | planted defect | control in the same test |
|---|---|---|
| `a_recorded_hash_that_drifts_from_the_manifest_is_rejected` | last character of the recorded hash flipped; **and** a draft fixture carrying the *2025 final's* real hash, which a path-blind lookup would have flattered | the truthful fixture must yield `vec![]` |
| `a_fixture_with_no_manifest_entry_is_named_not_skipped` | a fixture (`f8283--2026-DRAFT`) with no manifest entry — asserts the finding **and** that its message names the fixture; **and** a duplicated manifest path with differing hashes | — |
| `a_fixture_that_misidentifies_itself_is_a_finding` | `form: "f6251--2025"` inside `f6251--2026-DRAFT.json`; **and** a filename that is not a form stem | — |

The sample manifest in those tests is real serialised JSON deserialised through
`authority_manifest::Entry`, so they exercise the same parse the live manifest goes through rather
than a hand-built struct.

Also re-ran the pre-existing tests in the file (`cover_sheet_tests`, `the_bbox_parser…`,
`the_y_flip…`) — **7 passed, 0 failed**. `rustfmt --edition 2021 --check` on the file is clean.

## 4 · Found, NOT fixed

1. **The report's count is 48; the directory holds 47.** `find design/forms/geometry -type f` = 47,
   all `*.json`, no subdirectories. Either a fixture was removed since the recon or the 48 was the
   manifest's form-entry count minus something. Not reconciled — I cannot run `git`.
2. **The inverse direction is still unwitnessed.** My check is fixture → manifest. Nothing asserts
   manifest → fixture. Measured: **50** `kind: "form"` manifest entries, **47** fixtures, and three
   form entries have **no** geometry fixture at all: `Form_1099-DA`, `f8275r--2025`, `f8615--2025`.
   Whether each is a deliberate omission or a gap is exactly the "blank because the inputs say so vs.
   blank because nothing populated it" distinction, and it needs a decision (and, for a real fixture,
   the gitignored PDF) rather than a test I can write inside my file.
3. **Nothing checks the manifest hash against the PDF on disk from *this* test.**
   `authority_manifest::verify` covers that for committed storage; all 47 fixtures' PDFs are
   `storage: note` and gitignored, so CI compares fixture ↔ manifest only. That is the right
   boundary, but it means a manifest note and a fixture could be regenerated together from a
   *revised* PDF and this test would stay green — the guard against that is the review the failure
   message demands, not the test.
4. **The check is test-only.** There is no `xtask geometry-sha-check` command, because exposing one
   requires editing `crates/xtask/src/main.rs`, which I do not own. There is in-repo precedent for
   this shape (`capital_loss_carryover_check`, `schedule_1a_membership` are both `#[cfg(test)]` for
   the same reason: they answer a yes/no question, and the answer belongs in the suite).
