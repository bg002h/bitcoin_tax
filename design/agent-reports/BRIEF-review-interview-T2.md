# Brief — the ONE seam review of interview build T2 (the archived information returns + the box censuses)

Independent, adversarial BUILD REVIEWER in your own worktree at the commit the controller names.
Read-only: no source edits, no commits, no subagents. `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
before any cargo command; scoped `cargo nextest run --locked -p <crate> -E '<filter>'` only; never the
whole workspace (green at the commit under review — the controller states the count). Network is
available for re-fetching an archived document to a temp path. NOTE: the archived PDFs are gitignored,
so your worktree holds the notes, extracts and geometry fixtures but NOT the PDFs — re-fetch from the
URL in each note when you need bytes. Under the owner's S6 rule this is the ONE review the build gets.

## The one question
Are the fourteen archived documents the AUTHORITIES the spec's document screens will transcribe from
— the right revision for TY2025, the year read off each document rather than assumed, the manifest
hash equal to the fetched bytes, the extract equal to `pdftotext -layout` of those bytes — and do the
new instruments (the `DocBox` caption check in `line-coverage`, the per-document box censuses from the
extracts) actually discriminate: a one-character caption drift reds, a box the return reads that no
screen names reds, a census entry naming a caption the extract does not carry reds?

## The contract
`design/SPEC_interview.md` r2: §7 row T2 (its four kills), R4, R5, R2.2 (`Production::Collected {
from: DocBox | FilerRecords }`), §5.2, §8; the fold report's T2 rows
(`design/agent-reports/2026-09-07-spec-interview-fold.md`). The implementer's account is
`design/agent-reports/2026-09-07-build-interview-T2-implementation.md` — its claims are claims. The
controller's measured revision names are in `design/agent-reports/BRIEF-build-interview-T2.md`.

## Seams to examine, in order
1. **The archive, document by document (14).** For each: the note's URL answers; re-fetch to a temp
   path, `sha256sum` equals the note and the manifest entry; the byte count equals; `pdftotext -layout`
   of the fetched file equals the committed extract byte-for-byte; the geometry fixture's `pdf_sha256`
   equals; the revision date / tax year in the note was READ off the document's text (find the
   masthead or "Rev." line in the extract and compare) — a note that asserts a year the document does
   not print is a finding. `cargo run -q -p xtask -- authority-manifest` says OK.
2. **The revision in force.** For the six `--2024` documents (1099-INT/DIV/G + instructions): is the
   January-2024 revision the one in force for TY2025 (the instructions' own "What's New"/revision
   statement), and does the note say so? For `i1098et`: does the 1098-E row of the manifest join to it?
3. **`Production::Collected { from }`.** Every `Collected` line now carries a `DocBox { stem, box,
   extract_line }` or a `FilerRecords { instruction_line }`; `line-coverage` compares each `DocBox`
   caption against the archived extract. Plant a one-character caption change → red; plant a
   `Collected` with neither → red.
4. **The per-document box censuses.** For each archived form: the box set the census enumerates
   equals the boxes the extract prints (spot-check three forms by reading the extract yourself: W-2
   boxes 1–20 + 12a–12d + 14…; 1099-INT boxes 1–17; 1098 boxes 1–11); plant an omitted box → red;
   plant a census entry with a caption the extract lacks → red. Is the instrument's fixture population
   honest (does it cover every archived document, not a hand-picked three)?
5. **Pinned numbers and goldens.** Every number the report moved, re-measured; `docs/`, the work list
   and the manifest count moved only where listed.
6. **What T2 did NOT do that the spec's T2 row asks** — anything deferred to T5 must be named in the
   report with a reason, never silently missing.

## Severity (STANDARD_WORKFLOW.md): Critical = a wrong authority (wrong revision/year) or an
instrument that cannot red; Important = a missing document, a caption check with a hole, a census
that skips a form; Minor/Nit non-blocking.

## Output — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T2-review.md` in your worktree: header (commands +
summary lines); the 14-row document table (stem · revision read · sha match · extract match · verdict);
per seam findings as `### <ID> (<Severity>) — <claim>` with Where / What is wrong / Evidence / Minimal
change; `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a 3-line summary (counts, the single most
important finding, the report path).
