# Brief — fold the interview T2 seam review (C1 / I1 / I2 / I3 / M1–M4)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create (revert a plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E
'<filter>'` (never `cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and
a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D
warnings` before finishing. Every guarantee lands with a kill seen red once; the report quotes the
red. Every pinned number moved: old → new with cause. Network is available for the archive fetches
(`curl -A btctax-archive`); the archived PDFs are gitignored, the notes and extracts are not.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T2-review.md` (read it whole) and the
controller's ledger `2026-09-07-build-interview-T2-review-VERIFICATION.md` — every claim there is
already machine-verified; do not re-derive them. The ledger's **Disposition** section is the fold
list; its "What the Critical actually is" section fixes the SHAPE: the archive becomes
**per revision**, serving **TY2024, TY2025 and TY2026**, and the edition a row cites is **resolved
from the row's own tax year**, never pinned. Spec: `design/SPEC_interview.md` R2.1, R4, §5.2, §7 T2
(cadence *"per-revision (periodic)"*); `design/forms/README.md` (the archive steps); the T2 build
report `2026-09-07-build-interview-T2-implementation.md` (what exists). The IRS rule you derive
from, printed on the 2026 editions: *"the year of the revision date is the first year for which
issuers are to use the form to report amounts"* (annual forms: the edition of that year).

## The fold, in order

1. **C1a — archive the missing editions, in addition, never instead.** For each of the seven
   information returns and its booklet, the edition in force for TY2024 and for TY2026 that is not
   yet archived. Controller-measured on `irs-prior` (HTTP 200, finals): `fw2--2026`, `iw2w3--2026`,
   `f1099b--2026`, `i1099b--2026`, `f1098e--2026`, `i1098et--2026`, `f1099g--2026` (Rev. December
   2026), `i1099g--2026`, `i1098--2026` (Rev. December 2026); `f1099int/f1099div/f1098 --2026` are
   404 and the current `irs-pdf/` bytes hash equal to the archived ones (still in force). For TY2024
   find, the same way, the 2024 editions of the annual documents (`fw2`, `iw2w3`, `f1099b`,
   `i1099b`, `f1098e`, `i1098et` at `--2024`) and the periodic revisions in force for TY2024 where
   the archived one is later (`f1098`/`i1098` Rev. April 2025 is first used for TY2025 — locate the
   prior revision on `irs-prior`, e.g. `f1098--2022`, and archive it; the 1099-INT/DIV Rev. January
   2024 and 1099-G Rev. March 2024 already serve TY2024). Same pattern as T2: `design/forms/<year>/
   <stem>--<year>.pdf` (gitignored) + `.pdf.txt` note with measured sha256 and bytes, `pdftotext
   -layout` (forms) / plain (instructions) to `design/forms/extract/`, `xtask extract-geometry` for
   each form, `authority-manifest --regen` then `authority-manifest` → OK. Read the revision off each
   document and record it in the note. Nothing already archived is removed or changed.
2. **C1b — the census per archived edition.** `box_census::DocumentAuthority` becomes one entry per
   archived EDITION (`stem`, `edition`, the *Rev.*/annual year read from the note, `instructions`);
   `BOXES` carries a census per edition. The December 2026 Form 1099-G's grid has 13 boxes: **box
   10 "Family leave benefits"** is an income box — decide it **`RefuseIfNonzero`** naming Schedule 1
   as the exit (fail closed until T5 decides the line; record it as a T5 item in the report), and
   renumber the state boxes 11a/11b/12. Per-edition face-block bounds (M4): `fw2--2026`'s preamble
   does not end in *"1141, 1167, and 1179"* — read each edition's own preamble end and record it on
   the authority (or an accepted-marker list with a kill), never a silent fallback.
3. **C1c — `revision_in_force(stem, tax_year) -> Option<edition>`**, derived from the archive's own
   notes (annual: `edition == tax_year`; periodic: the archived revision with the greatest revision
   year ≤ `tax_year`), `None` when nothing archived governs that year. A KAT pins the whole table for
   TY2024/2025/2026 × the seven documents (e.g. `fw2`: 2024/2025/2026; `f1099g`: 2024→Rev.3-2024,
   2025→Rev.3-2024, 2026→Rev.12-2026; `f1098`: 2024→the prior revision, 2025/2026→Rev.4-2025).
4. **C1d — rows cite a box, the checker resolves the edition.** `CollectedFrom::DocBox` loses its
   pinned `year` (and its `extract_line`, which lives on the census entry): `DocBox { stem, box }`.
   `line-coverage` resolves the edition from the ROW's form year via (3) and verifies the row's
   quoted caption against that edition's census entry (whitespace-normalised — the reviewer measured
   that the 2026 captions differ only in wrap position). A row whose year has no governing edition
   reds. Kill: the 2024 W-2 rows now verify against `fw2--2024`, and planting a caption that only
   the 2025 edition prints on a 2024 row reds.
5. **I1 — bind `FilerRecords`.** `FilerRecords { instruction_line }` only: the booklet is the row's
   form's own instructions (`cite_check::FORMS.instructions`) at the row's year, and the sentence
   must fall inside that booklet's `Line <N>` block for the row's line, the block enumerated from
   the extract's `Line N.` headings (record any booklet whose headings do not follow that shape and
   how you bound it). The reviewer's plant (Schedule A 5b citing the Form 6251 AMTFTC sentence) is
   the kill — red. Re-point any of the 36 rows this exposes to the sentence that actually says it,
   or re-tag with a reason; list each in the report.
6. **I2 — derive the document set.** `DOCUMENTS` (per edition, now) must equal, both ways, the set
   of `MANIFEST.json` `kind == "form"` entries whose stem `archive_check::irs_stem` classifies into
   the W / 1098 / 1099 series. Kills: delete one authority → red; the reviewer's plant (dropping
   `f1098e`) → red.
7. **I3 — `xtask authority-refresh --check`.** For every `storage: note` manifest entry, `curl`
   (subprocess — no `ureq` in any crate; xtask stays outside `check-isolation`'s TAX_CRATES anyway)
   the note's URL, hash, and report drift per entry; for every information return also probe
   `irs-prior/<stem>--<max archived year + 1>.pdf` and report a 200 as *"a newer edition exists"*.
   On demand only — never in `make check`, never in the suite. Kill: a mutated note hash → red
   (offline-testable by pointing the fetch at a local file, or a `--from-dir` seam).
8. **M1–M3 inline**: the `year` doc comment counts what the tool prints; Form 8995 line 12 →
   `Exception` (or `Carry`) with the reason the sentence names no line; the unlabelled-box limit
   (FATCA, *2nd TIN not.*) declared beside the wrapped-caption limit, `is_label` widened past `f`
   with the contiguity guard still holding.

## Constraints
- Nothing archived by T2 is altered; T2's kills stay red-then-green (re-run them).
- `line-coverage` counts may move only where a row was re-pointed (list each); the ratchets are not
  raised by hand.
- If context runs short: leave the tree compiling, fmt-clean and green on every crate you touched,
  and state precisely which numbered item is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T2-fold.md`: per numbered item what landed, the
documents archived (stem, edition, revision read off the document, sha256 short, bytes), the
`revision_in_force` table as the KAT pins it, the box-10 decision, every re-pointed `FilerRecords`
row, every kill with its red text, every pinned number moved, suite lines per crate. Return only a
4-line summary plus the path.
