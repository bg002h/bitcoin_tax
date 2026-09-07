# Brief — interview build T2: archive the information returns (evidence, then the box censuses)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore` files you did not
create. Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never `cargo test`, never
`--release`, never the whole workspace — the controller's gate runs `make check`); `cargo fmt --all`
and a clean `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once (plant, observe, revert via a `cp`
backup); the report says how, with the red text. Every pinned number moved: old → new with cause.
Process in force (owner S6): this build gets ONE seam review and ONE re-verification after you.

## The contract
`design/SPEC_interview.md` r2: **§7 row T2** (the task, its kills, its cadence "per revision"),
**R4** (the document screens carry the box caption verbatim from the archived extract), **R5** (lines
collected from the filer's records with no box: `FilerRecords { instruction_line }`), **R2.2**'s
`Production::Collected { from: DocBox | FilerRecords }` shape, **§5.2**, **§8**. The fold report's
T2-tagged "what the build must now prove" rows (`design/agent-reports/2026-09-07-spec-interview-fold.md`:
I2 per-document box censuses; M6). Build AS WRITTEN; the tree's real names win; deviations recorded.

## What T2 delivers
1. **Archive, as AUTHORITIES (finals from `irs-prior`, never drafts), the information returns and
   their instructions** the spec names: `fw2`, `f1099int`, `f1099div`, `f1099g`, `f1099b`, `f1098`,
   `f1098e` and their `iNNNN` (the W-2's instructions are `iw2w3`; each 1099's are `i1099<suffix>`;
   `i1098`, `i1098e`). Revision: the one in force for TY2025 (these are periodic or annual forms —
   read the revision date off each document's own text and record it). Follow the archive pattern of
   commit `b60c600c` exactly: fetch to `design/forms/2025/<stem>--2025.pdf` (gitignored), the
   `.pdf.txt` note with the measured sha256 and byte count in the existing convention, `pdftotext
   -layout` to `design/forms/extract/<stem>--2025.txt`, `cargo run -q -p xtask -- extract-geometry
   <stem>--2025`, then `cargo run -q -p xtask -- authority-manifest --regen` and `authority-manifest`
   (must say OK). The controller measured at dispatch that every one of these URLs answers HTTP 200
   at `https://www.irs.gov/pub/irs-prior/<stem>--2025.pdf`; if a stem's TY2025 revision is not the
   one in force (a periodic form revised earlier), fetch the revision the instructions name and say
   so. The year on each document is READ off its text before it goes in the note (the archiver's
   rule), never assumed.
2. **`Production::Collected` gains `from: DocBox | FilerRecords { instruction_line }`** (R2.2/R5) and
   `line-coverage` checks each `DocBox` caption against the document's extract — a one-character
   caption change reds (kill); a `Collected` line with neither a `DocBox` nor a `FilerRecords` reds
   (kill, M6).
3. **Per-document BOX CENSUSES enumerated from the extracts** (I2): for each archived document, the
   set of boxes the form prints (from the extract's box labels), and the kill that every box the
   return reads is in the screen's field set or recorded unused with a reason — the same shape as the
   form-line census (`crates/btctax-forms/tests/field_census.rs`), built against the T2 extracts.
   The screens themselves are T5; T2 lands the census instrument and its fixture population (a box
   named in an archived extract that no map/screen/census names is red — say how the instrument is
   seen red on a planted omission).
4. **Anything else §7's T2 row lists** — the row wins.

## Constraints
- `design/forms/MANIFEST.json`, the notes, extracts and geometry fixtures are the deliverable; no
  bundled templates under `crates/btctax-forms/forms/` (the interview fills the RETURN, not these
  documents). The census registers and `max_unwitnessed` do not move; the label-reader floors move
  only if a run measures them (and then upward, with the numbers pasted).
- Do not touch the spec or any `design/agent-reports/*.md` other than your report.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T2-implementation.md`: per document the URL, the
revision read off the text, sha256/bytes, extract line count, geometry box count; the `Production`
change and its kills with red text; the box-census instrument and its planted red; pinned numbers
moved; the exact commands with summary lines; anything not done and why. Return ONLY a 3-line
summary (what landed, suite results for the crates you touched, the report path).
