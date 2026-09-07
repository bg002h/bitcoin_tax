# Verification ledger — interview T2 seam review (`2026-09-07-build-interview-T2-review.md`, 1C/3I/4M/1N)

Controller machine-checks, run from the main tree at `c5cf40ec` before any fold. Every row below is a
command's output, not a reading of the report.

| # | claim in the report | check run | measured | decision |
|---|---|---|---|---|
| C1 | `SPEC_interview.md:45` names TY2026 as the target | `sed -n 45p` | *"TY2026, the target (`ROADMAP_STATUS.md` §0a)"* | true |
| C1 | nine `--2026` editions exist as finals on `irs-prior` | `curl -sI` × 12 | `fw2 iw2w3 f1099b i1099b f1098e i1098et f1099g i1099g i1098` → 200; `f1099int f1099div f1098` → 404 | true |
| C1 | `f1099g--2026` is *Rev. December 2026*, final, with box 10 *Family leave benefits* and state boxes 11a/11b/12 | fetched; `pdftotext -layout`; grep | `DRAFT AS OF` 0; `(Rev. December 2026)`; `:64 "9 Market gain … 10 Fa[mily leave benefits]"`, `:67 "11a State  11b State identification no."` | true — **the grid moved by an income box** |
| C1 | current `irs-pdf/` bytes equal the archive for INT/DIV/1098 and differ for 1099-G | `curl … | sha256sum` vs the note | `f1099int` SAME, `f1099div` SAME, `f1098` SAME, `f1099g` DIFFERENT (`65a416c5…` live vs `fe46acb4…` archived) | true |
| C1 | the `Collected` rows pin one edition per document regardless of the row's tax year | `grep -o 'doc_box("…", "…"'` | `fw2 2025` ×8, `f1099b 2025` ×4, `f1099int 2024` ×7, `f1099div 2024` ×5, `f1099g 2024` ×2 — the 2024 maps' rows cite the **2025** W-2 | true, and wider than the report says: no row resolves an edition from its year |
| C1 | root cause | `BRIEF-build-interview-T2.md` | the controller's own brief said *"the one in force for TY2025"* — the build followed it exactly | the brief was wrong, not the build |
| M4 | `fw2--2026` lacks the `1141, 1167, and 1179` preamble marker | fetched; `grep -c` | `f1099g--2026` 1, `fw2--2026` **0** | true |
| I1 | the `FilerRecords` arm binds neither booklet-to-form nor sentence-to-line | `sed -n 598,645p line_coverage_check.rs` | the arm checks: non-empty, `starts_with('i')`, `stem != form`, file exists, `normalize(text).contains(sentence)` — nothing else | true (the controller's own re-plant mis-edited the row and did not compile; the mechanism read is decisive) |
| I2 | `box_census::DOCUMENTS` is a hand-list with no join to the archive | `grep -n 'const DOCUMENTS\|MANIFEST\|read_dir'` | `pub const DOCUMENTS: &[DocumentAuthority] = &[` at `:97`; the only `manifest` mention is `CARGO_MANIFEST_DIR` at `:65` | true |
| I3 | no instrument re-fetches a note's URL | `grep -ln irs.gov crates/xtask/src`; `grep ureq\|reqwest\|curl` | `irs.gov` appears only in doc text; `check_isolation.rs` forbids `ureq`/`rustls` in the six TAX crates (xtask is not one) | true; an xtask command shelling out to `curl`, on demand, is inside the isolation rule |
| M1 | doc comment says *"Six of the seven documents are `--2024`"* | `sed -n 86,88p box_census.rs`; the `doc_box` census above | three of the seven forms are `--2024` | true |
| M2 | Form 8995 line 12's quoted sentence names no line | `sed -n 49p f8995--2024.txt` | *"Enter your net capital gain, if any, increased by any qualified dividends"* | true; `Combine` has no operands to range over |
| M3 | the FATCA and *2nd TIN not.* boxes print unlabelled on the 1099-INT | `sed -n '47p;51p' f1099int--2024.txt` | both present, neither carries a label | true |
| N1 | seven xtask tests fail in a PDF-less worktree | (environment; the reviewer's own run) | not re-run | noted for every future reviewer brief |

## What the Critical actually is

The archive is one edition per document, and the interview must serve **three** tax years:
**TY2024** (the simulated real return, FR-64 — the only computable year), **TY2025** (on S1) and
**TY2026** (the target). The TY2024 filer holds a **2024** W-2, 1099-B and 1098-E; the TY2026 filer
holds the 2026 editions and the December 2026 1099-G. Neither is archived. The reviewer's minimal
change (move everything to 2026) fixes TY2026 and breaks TY2024; the fold instead makes the archive
**per revision** and derives *which revision governs which tax year* from the rule the IRS prints on
the documents (annual forms: the edition of that year; periodic *Rev.* forms: the latest revision
whose year ≤ the tax year), so a `Collected` row cites a box and the checker resolves the edition
from the row's own year. That is the spec's own cadence (*"per-revision (periodic)"*) taken
seriously, and it is what makes the 1099-G's box 10 visible to the census in exactly the year it
appears.

## Disposition

- **C1 — FOLD** (shape above; archive the editions in force for TY2024 and TY2026 in addition, never
  instead; census per archived edition; `revision_in_force(stem, year)` derived and pinned by a KAT;
  `DocBox`/`FilerRecords` resolve their edition from the row's year).
- **I1 — FOLD**: bind the booklet to the row's form via `cite_check::FORMS.instructions` and the
  sentence to that booklet's `Line <N>` block, enumerated from the extract; the reviewer's plant is
  the kill.
- **I2 — FOLD**: derive the censused document set from `MANIFEST.json` by stem series (the W /
  1098 / 1099 series `archive_check::irs_stem` already classifies), asserted equal both ways; kill
  by deleting one authority and by archiving one more.
- **I3 — FOLD**: `xtask authority-refresh --check` — `curl` subprocess, on demand, never in
  `make check`; reports hash drift per note and probes `irs-prior/<stem>--<Y+1>` for a newer edition
  of every information return; kill by a mutated note hash.
- **M1–M4 — fold inline** (doc count; 8995 line 12 to `Exception`/`Carry` with its reason; the
  unlabelled-box limit declared and `is_label` widened; a per-edition face-block bound).
- **N1 — process**: every future reviewer brief states that six `form_delta` tests and one
  `harness_check` test fail in a PDF-less worktree with a redirected target dir.

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T2-review.md`.
