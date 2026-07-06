# SPEC — official IRS PDF form-fill, sub-project 1 (engine + Form 8949 + Schedule D, TY2025)

**Source baseline:** `main` @ `3117379` (branch TBD `feat/irs-form-fill-sp1`). **Review status: DRAFT — awaiting
R0 (model: user-directed Fable/Opus).** SP1 of task #45 (btctax fills official IRS fillable PDFs). Feasibility
recon: [[irs-form-fill-feasibility]]. **Full-set/3-year is the GOAL; SP1 proves the engine on Form 8949 +
Schedule D for TY2025**, then SP2 (8283/SE/1040 summary) + SP3 (2017/2024) follow.

## Goal (SP1)
A new `btctax export-irs-pdf --tax-year 2025 --out <dir>` command that populates the OFFICIAL IRS fillable PDFs
(Form 8949 + Schedule D) with btctax's already-computed data (the same source as `form8949.csv` /
`schedule_d.csv`) and writes ready-to-review PDFs — offline, deterministic, verified field-by-field.

## Architecture
- **New crate `crates/btctax-forms`** (lib; pure Rust, NO network — the `cargo tree` isolation check still
  holds, `ureq` stays only in btctax-update-prices). Deps: `lopdf` (AcroForm manipulation) + btctax-core types.
  Holds: the fill ENGINE, the per-(form,year) FIELD MAPS, and the bundled official PDFs.
- **Bundled forms (public domain):** `crates/btctax-forms/forms/2025/f8949.pdf`, `.../schedule_d.pdf` (US
  government works — freely redistributable; no attribution). One dir per tax year.
- **btctax-cli** gains the `export-irs-pdf` arm calling `btctax_forms::fill_*`. The computed row/total data comes
  from the SAME projection path that builds `form8949.csv` (reuse it — do NOT recompute).

## Fill mechanism (lopdf)
1. Load the bundled PDF (`lopdf::Document::load_mem`).
2. For each logical cell → set the AcroForm field's `/V` (and `/AS` for checkboxes) per the FIELD MAP.
3. Set the catalog `/AcroForm << /NeedAppearances true >>` so viewers regenerate appearances (SP1 does NOT
   hand-author appearance streams — an SP2+ hardening; **[caveat]** a few viewers ignore NeedAppearances, so
   correctness is verified on the DATA (`/V`), not on rendered pixels — see KATs).
4. **Determinism (NFR4):** strip/pin `/CreationDate`/`/ModDate`/`/ID` so the same (data, form) → byte-identical
   PDF (a golden-file test). No RNG/clock.
5. **Read-back verification [★]:** re-open the written PDF, read every mapped field's `/V`, assert it equals the
   intended value. A mis-mapped field on an OFFICIAL tax form is the headline risk — this catches it.

## Field maps (TY2025 — committed data + verified)
- **Form 8949** (verified fillable, 2pg/229 fields): the grid is REGULAR — each DATA ROW = 8 sequential text
  fields = cols (a)desc (b)acquired (c)sold (d)proceeds (e)cost (f)code (g)adj (h)gain; Page1 row1 = `f1_03..
  f1_10`, +8/row (`f1_11..f1_18`, …), 14 rows/part/page; Page2 = Part II (`f2_*`). Box A/B/C checkbox = `c1_*`,
  D/E/F = `c2_*`; the line-2 totals row = its own 8 fields; name/SSN = `f1_01`/`f1_02`. **The impl EXTRACTS the
  exact 2025 field names** (by annotation position, the pypdf method used in recon) into a committed map
  (`forms/2025/f8949.map.toml` — logical cell → field name) + a golden test.
- **Schedule D** (2pg/68 fields): line 3 (short-term Box-C totals: d/e/h), line 10 (long-term Box-F totals),
  line 16 (net). Committed `schedule_d.map.toml`.
- **Map format:** a per-(form,year) TOML mapping logical positions → field names, loaded by the engine; adding a
  year = add a `forms/<year>/` dir (PDF + maps) + a golden test. NO code change per year.

## [★] 14-row overflow (Box C/F must be itemized — no totals-only shortcut)
Form 8949 holds 14 rows per part per page. SP1's data may exceed it (the ReadOnly TY2017 demo = 26 ST + 28 LT).
Handle by **cloning the 8949 page** (lopdf page duplication) ⌈rows/14⌉ times per part, filling each copy with
its 14-row slice; the per-part line-2 TOTALS go on the LAST page of that part (the intermediate pages leave
totals blank). Output = one multi-page `f8949.pdf`. Schedule D carries the grand totals (unaffected by paging).

## [★ safety] Pseudo/estimate guard — a filled OFFICIAL form is more dangerous than a CSV
- **Attestation-gated:** when the projection is `pseudo_active()`, `export-irs-pdf` requires the same
  `I attest this is true` attestation as `export-snapshot` (reuse `require_attestation`).
- **DRAFT watermark:** when pseudo-active, stamp a diagonal `DRAFT — ESTIMATE, NOT FOR FILING` watermark on
  EVERY page (an lopdf content-stream overlay) so a printed pseudo-filled form can NEVER be mistaken for a real
  return. A fully-real ledger fills clean (no watermark). This is the load-bearing safety difference from the CSV.

## KATs (btctax-forms + btctax-cli)
- **★ read-back:** `fill_then_readback_matches` (fill 8949 + Sch D from a fixture → re-read every mapped `/V`
  == expected, incl. the box checkbox `/AS`). The fault-inject target: corrupt one map entry ⇒ RED.
- **★ overflow:** `overflow_paginates_and_totals_on_last_page` (>14 rows/part → ⌈n/14⌉ pages, correct row
  slices, totals only on the last).
- `schedule_d_totals_match_form8949` (Sch D line 3/10/16 == the 8949 part sums == the CSV).
- **determinism:** `fill_is_byte_deterministic` (golden PDF; no clock/RNG); `field_map_2025_covers_all_cells`
  (every logical cell has a field + the golden 2025 map matches the bundled PDF's field set).
- **safety:** `pseudo_active_fill_requires_attestation`; `pseudo_active_fill_is_watermarked`;
  `real_ledger_fill_is_clean`.
- **isolation:** `btctax_forms_has_no_network_dep` (cargo-tree — lopdf only; ureq absent).

## Scope / SemVer / lockstep
New `btctax-forms` crate (+ `lopdf` + bundled TY2025 public-domain PDFs) + btctax-cli `export-irs-pdf` arm. No
core change (reuses the form data). Workspace grows +1 crate. MINOR (new command/crate) → next release bump. The
new `export-irs-pdf` man page (clap doc-comment → regenerate). README note (fills official PDFs; DRAFT watermark
on estimates; public-domain forms). The cargo-tree network-isolation check must still pass (lopdf ≠ network).

## Plan (TDD)
- **T1 (engine)** — `btctax-forms`: `lopdf` fill primitive (set `/V`/`/AS` + NeedAppearances + strip dates) +
  the read-back verifier + the TOML map loader; extract + commit the TY2025 8949 + Sch D maps + bundled PDFs;
  the read-back + determinism + map-coverage KATs.
- **T2 (command)** — the `export-irs-pdf --tax-year 2025 --out` arm wired to the projection's form data; the
  14-row overflow pagination; the pseudo attestation gate + DRAFT watermark; the overflow + Sch-D-totals +
  safety + isolation KATs; man page + README. Whole-diff + full suite + FOLLOWUPS.

## Gotchas
- **[★] read-back-verify every field** — a mis-mapped cell = a wrong official form; the golden map + read-back
  are the safety net.
- **[★] pseudo → attestation + DRAFT watermark** — never emit a clean-looking official form from an estimate.
- **[overflow] Box C/F must itemize** — clone pages; totals on the last page of each part.
- **[per-year] maps are DATA** — a committed TOML + golden PDF per year; adding a year is data-only.
- **[determinism] strip PDF dates/ID** — golden-file byte-stability.
- **[NeedAppearances caveat] correctness is on `/V`, not pixels** — a viewer that ignores it still holds the
  right data; SP2 may add appearance-stream generation.
- **[isolation] lopdf is pure Rust** — no network; the tax-binary isolation check still passes.
