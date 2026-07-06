# SPEC — official IRS PDF form-fill, sub-project 1 (engine + Form 8949 + Schedule D, TY2025)

**Source baseline:** `main` @ `3117379` (branch `feat/irs-form-fill-sp1`). **Review status: R0 round 1 folded
(2C/5I/4M/3N — merged IN-PLACE; surgical; R0 run on Fable). Awaiting R0 round 2.** Review:
`reviews/R0-spec-irs-form-fill-sp1-round-1.md`. SP1 of task #45. Feasibility: [[irs-form-fill-feasibility]].
**Full-set/3-year is the GOAL; SP1 proves the engine on Form 8949 + Schedule D for TY2025.**

## Goal (SP1)
`btctax export-irs-pdf --tax-year 2025 --out <dir>` populates the OFFICIAL IRS fillable PDFs (Form 8949 +
Schedule D) with btctax's already-computed data (reuse the projection's `form_8949()`/`schedule_d()` behind
`write_form8949_csv` — do NOT recompute) and writes ready-to-review PDFs — offline, deterministic, verified.

## [★ R0-C1] These are XFA-hybrid PDFs — the fill MUST drop `/XFA`
Every bundled form's `/AcroForm` carries an `/XFA` packet (a live XML forms layer Acrobat/Reader prefer). A
`/V`-only fill opens **BLANK in Acrobat** (the IRS's recommended viewer). **Verified fix (R0 ran the
experiment):** they are STATIC XFA hybrids with a COMPLETE classic AcroForm layer, so the engine **removes the
`/XFA` key** from `/AcroForm` before saving; the filled `/V`/`/AS` values then persist AND render (poppler
confirmed). KAT `output_has_no_xfa` + a one-time MANUAL Acrobat open of a golden output (documented, not CI).

## [★ R0-C2] TY2025 Form 8949 is the 1099-DA revision — digital assets are Box I / Box L, NOT C / F
Part I now has 6 boxes (A,B,C,G,H,I); **Box C reads "other than digital asset transactions"** — checking it for
BTC is FACTUALLY FALSE. Bitcoin's conservative default is **Box I (short-term)** / **Box L (long-term)**
(on-state `/6` of `c1_1[5]` / `c2_1[5]`). The core `Form8949Box::{C,F}` (forms.rs:114-116) is the OLD taxonomy —
DO NOT reuse it directly; the forms crate maps to the YEAR's digital-asset box via the per-year map (no core
change). Schedule D survives: line 3 text = "Box C **or Box I**", line 10 = "Box F **or Box L**" (confirmed in
the PDF). **[R0-I5]** the CSV `box_needs_review` maps to a loud warning: if any row could belong on a SEPARATE
1099-DA-reported 8949 (Box G/H/J/K), warn — SP1 files all BTC under I/L and says so.

## Architecture
- **New crate `crates/btctax-forms`** (lib; pure Rust, NO network — **[R0-M] add it to
  `xtask/src/check_isolation.rs` TAX_CRATES**; the cargo-tree isolation check stays a `xtask`/CI step, not a
  `#[test]`). Deps: `lopdf` (R0-confirmed the right crate) + btctax-core. Bundled PDFs via `include_bytes!`.
- **Bundled forms (public domain):** `forms/2025/{f8949.pdf,schedule_d.pdf}` (US-gov works). One dir/year.
- **btctax-cli** gains the `export-irs-pdf` arm calling `btctax_forms::*`, fed the projection's form data.

## Fill mechanism (lopdf)
1. `Document::load_mem` the bundled PDF; **remove `/XFA`** from `/AcroForm` [C1].
2. Set each mapped field's `/V` (+ checkbox `/AS` to the box's on-state) per the per-year map.
3. Set `/AcroForm /NeedAppearances true` (SP1 defers appearance-stream generation; correctness is verified on
   `/V`, not pixels — [caveat] noted).
4. **Determinism:** strip/pin `/CreationDate`/`/ModDate` and the trailer `/ID` so (data, form) → byte-identical
   (golden test). No clock/RNG.
5. **[★ R0-I3] Read-back verification — MAP-INDEPENDENT:** the naive "re-read via the same map" is circular
   (swapped columns pass). Verify GEOMETRICALLY: assert each written value sits in a widget whose `/Rect` falls
   in the EXPECTED column-x / row-y band (bands dumped in the R0 review), AND assert NO unmapped field was
   filled. This is the real safety net against a mis-mapped cell.

## Field maps (TY2025 — committed data + golden-verified) [R0-I1 corrected]
- **Form 8949** (2pg; XFA-hybrid AcroForm): each DATA ROW = 8 sequential text fields = cols (a)desc (b)acquired
  (c)sold (d)proceeds (e)cost (f)code (g)adj (h)gain; **11 ROWS per part per page (NOT 14 — that was the 2017
  form)**; Page1 = Part I, Page2 = Part II. **The per-part line-2 TOTALS row = 5 fields `f1_91..f1_95`** (d,e,g,h
  + a spacer — NOT 8). 6 box checkboxes/part. **rows-per-page is PER-YEAR MAP DATA** (else "adding a year =
  data-only" breaks). **[R0-M] map keys are fully-qualified + bracketed** (`topmostSubform[0].Page1[0].f1_03[0]`;
  note Schedule D uses UNPADDED `f1_3` not `f1_03`).
- **Schedule D** (2pg): **[R0-I4] fill lines 3 AND 7 (ST) and 10 AND 15 (LT) — 7/15 are pure arithmetic**, else
  16 = 7+15 with 7/15 blank is self-inconsistent. Line 16 net. **Answer the QOF Yes/No** (No for SP1). Line 21
  (loss limit) applies on a net loss. **Scope OUT 17-22** (28%-rate/unrecaptured-1250/QDI worksheet) with a
  printed notice on the output.
- **Map format:** per-(form,year) TOML (logical cell → fully-qualified field name + on-states); adding a year =
  a `forms/<year>/` dir (PDF + maps + golden) — data-only.

## [★ R0-I2] 14→11-row overflow (Box I/L must be itemized — confirmed)
>11 rows/part ⇒ ⌈rows/11⌉ page copies per part. **The ISO 32000 same-name trap: duplicated AcroForm fields
share ONE `/V`.** So the engine DEEP-COPIES each page AND RENAMES its fields per copy (independent form
instances), filling each with its 11-row slice. **Per-copy totals** (the line-2 text says "Enter each total
here" — NOT last-page-only); Schedule D aggregates the grand totals. (Demo TY2017-shaped data: 26 ST + 28 LT ⇒
3 + 3 pages at 11/page.)

## [★ safety] Pseudo/estimate guard
Pseudo-active ⇒ **require the `I attest this is true` attestation** (reuse `require_attestation`) AND **stamp a
diagonal `DRAFT — ESTIMATE, NOT FOR FILING` watermark on every page** (an lopdf content-stream overlay with its
OWN embedded font/vector resources [R0-M]). R0 confirmed watermark+attestation is the right bar (consistent with
the shipped pseudo-export policy; strictly safer than the already-permitted CSV) — NOT a full refusal. Real
ledger fills clean.

## KATs
- **★ read-back (geometric, map-independent) [I3]:** `filled_values_land_in_expected_geometry` + `no_unmapped_field_filled`. Fault-inject: swap two map columns ⇒ RED.
- **★ XFA [C1]:** `output_has_no_xfa`; `filled_value_persists_after_xfa_drop`.
- **★ box [C2]:** `ty2025_bitcoin_uses_box_I_and_L` (NOT C/F); `schedule_d_line3_10_accept_I_L`.
- **overflow [I2]:** `overflow_renames_fields_per_copy_no_shared_value`; `each_copy_has_its_own_totals`; `eleven_rows_per_page`.
- `schedule_d_fills_3_7_10_15_16_and_qof`; `schedule_d_totals_match_form8949_and_csv`.
- **determinism:** `fill_is_byte_deterministic` (golden); `map_2025_matches_bundled_pdf_fieldset` + `rows_per_page_is_map_data`.
- **safety:** `pseudo_fill_requires_attestation`; `pseudo_fill_is_watermarked`; `real_fill_is_clean`.
- **isolation:** btctax-forms in `check_isolation` TAX_CRATES (xtask step).

## Scope / SemVer / lockstep
New `btctax-forms` crate (+ `lopdf` + bundled TY2025 public-domain PDFs) + btctax-cli `export-irs-pdf`. No core
change (map to the year's box in the forms crate; reuse `form_8949()`/`schedule_d()`). MINOR + new crate → next
release bump. New man page; README (fills official PDFs; Box I/L for digital assets; DRAFT watermark on
estimates; drops the XFA layer; scope-outs). cargo-tree isolation must still pass (lopdf pure Rust).

## Plan (TDD)
- **T1 (engine)** — `btctax-forms`: lopdf primitive (drop `/XFA`; set `/V`/`/AS`; NeedAppearances; strip
  dates/ID) + the GEOMETRIC read-back verifier + the TOML map loader; extract + commit the TY2025 8949 + Sch D
  maps (11 rows/part, totals `f1_91..95`, Box I/L on-states) + bundled PDFs; the read-back/XFA/box/determinism/
  map-coverage KATs.
- **T2 (command)** — `export-irs-pdf --tax-year 2025 --out` wired to the form data; the 11-row overflow
  (rename-per-copy, per-copy totals); Schedule D 3/7/10/15/16 + QOF + scope-out notice; the pseudo attestation
  gate + DRAFT watermark; the box G/H/J/K warning; safety/isolation KATs; man page + README; whole-diff.

## Gotchas
- **[C1] drop `/XFA`** or Acrobat shows blank; **[C2] Box I/L for BTC**, not C/F (1099-DA revision).
- **[I1] 11 rows/part, totals = 5 fields (`f1_91..95`)**; rows-per-page is per-year DATA.
- **[I2] rename fields per page copy** (shared-name `/V` trap); **per-copy totals**.
- **[I3] geometric, map-independent read-back** + no-unmapped-filled (the map is what we distrust).
- **[I4] fill Sch D 7/15 + QOF; scope-out 17-22 with a printed notice.**
- **[I5] warn on possible Box G/H/J/K (separate 1099-DA 8949) rows.**
- **[★ safety] pseudo ⇒ attestation + DRAFT watermark** (own font resources); real ⇒ clean.
- **[det] strip dates/ID; golden test.** **[iso] btctax-forms in check_isolation TAX_CRATES.**
