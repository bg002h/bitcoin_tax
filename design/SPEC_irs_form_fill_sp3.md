# SPEC — official IRS PDF form-fill, sub-project 3 (2017 + 2024 field maps, full packet)

**Source baseline:** `main` @ `55f5812` (branch `feat/irs-form-fill-sp3`). **Review status: DRAFT — awaiting R0
(model: Fable — dense per-year forensics; 2017's old forms are the trap-rich case).** SP3 (final) of task #45.
Builds on the shipped SP1+SP2 engine (`btctax-forms`: fill primitive, XFA-drop, per-form geometric oracle,
watermark, attestation, per-year TOML maps). **The engine does not change; SP3 adds per-year DATA (maps +
bundled PDFs) + the per-year structural config the oracle/fills read.**

## Goal (SP3)
Extend `export-irs-pdf` to accept `--tax-year 2017` and `--tax-year 2024` for the full packet (8949, Schedule D,
8283, Schedule SE, 1040-cap-gains), so a historical vault (e.g. the ReadOnly TY2017 demo) fills the REAL
official PDFs of its own year. All 9 forms re-verified fillable XFA hybrids → the engine's XFA-drop applies.

## [★] The per-year facts (recon-verified against the real PDFs)
| form | 2017 | 2024 | (2025 shipped) |
|---|---|---|---|
| **8949 BTC box** | **Box C (ST) / F (LT)** — "not reported on 1099-B" | **Box C / F** | Box I / L |
| **8949 rows/part** | **14** | **14** | 11 |
| **1040 cap-gain line** | **line 13** | **line 7** (not 7a) | line 7a |
| **1040 digital-asset Q** | **NONE** (predates it) | **yes** | yes (7a era) |
| **Schedule SE** | **old short(§A)+long(§B), 70 fields** | unified, 32 fields | unified, 32 |
| **8283** | Rev. applicable to 2017 | Rev. applicable to 2024 | Rev. 12-2025 |

- **[★ box taxonomy]** 2017 + 2024 predate the 1099-DA revision → BTC (self-custody, no 1099-B) files under
  the OLD **Box C (ST) / Box F (LT)**. This IS the core `Form8949Box::{C,F}` (forms.rs:114) that SP1 correctly
  declined for 2025 — for these years it is right. The per-year map's box field + on-state encode it (data-only;
  no core change). Schedule D line 1b/8b or 3/10 wording per year — verify at extraction.
- **[★ rows/part = 14]** (not 11) for both years — rows-per-page is already per-year map DATA (SP1-I1); the
  overflow uses 14. Re-derive the 8-field/row grid + totals-row fields per year (they shift — SP1 saw 2017 ≠
  2025 numbering).
- **[★ 1040 per year]** 2017 → capital gain on **line 13**, and **NO digital-asset question exists** (skip it
  entirely; the map has no DA field). 2024 → **line 7** (not 7a) + the DA question present (SP2's C4 YES-iff-
  activity logic applies, 2024 field ids). The I★1 active/inactive-Schedule-D rule + the loss/§1211 rule apply
  both years.
- **[★ Schedule SE 2017 is the OLD short+long form]** (70 fields; Section A short-method vs Section B
  long-method) — structurally different from the unified 2020+ form the SP2 fill targets. Its line set + the
  oracle's logical-sequence config are 2017-specific. **[decision]** btctax's SE data maps to the LONG form
  (Section B) — enumerate the 2017 §B chain + fill it; the $400 floor + line-12=SS+Medicare (the 0.9% addl went
  to Form 8959 only from 2013, so 2017 already excludes it — confirm the 2017 line arithmetic) still hold.
- **[★ 8283 is REVISION-dated, not year-dated]** — bundle the 8283 REVISION applicable to each filing year
  (2017 filing → the rev current in early 2018; 2024 → the rev current in early 2025), keyed by revision string,
  and confirm the "k Digital assets" property box EXISTS in that revision (it is NEW in a recent revision — a
  2017-era 8283 has NO digital-asset property line → BTC donation uses the closest correct box, likely "Other",
  with a printed note). This is the sharpest 2017 trap.

## Engine reuse (unchanged) + what SP3 touches
- Reuses: everything. The per-form geometric oracle re-derives its bands from EACH year's blank PDF (already
  map-independent), so 2017/2024's shifted coordinates are handled — provided the per-year logical-sequence
  config (SE line order, 8949 columns) is supplied as data.
- SP3 adds: `forms/2017/` + `forms/2024/` dirs (bundled PDFs + maps), the per-year box/row/line config, and the
  per-year branch in each `fill_*` (e.g. 1040 chooses line 13 vs 7 vs 7a + DA-present; SE chooses old-vs-unified
  chain; 8949 chooses C/F vs I/L + 14-vs-11 rows). Prefer a per-year config struct read from the map over
  `if year==` ladders.

## KATs (per year, mirroring SP1/SP2 per-form)
- **★ box taxonomy:** `ty2017_and_2024_bitcoin_use_box_C_and_F` (NOT I/L); `ty2025_still_uses_I_L` (regression).
- **★ per-year 1040:** `ty2017_1040_capital_gain_on_line_13_no_da_question`; `ty2024_1040_line_7_with_da_question`.
- **★ SE 2017 old form:** `ty2017_schedule_se_uses_long_form_section_b`; line-12 + $400 floor hold.
- **rows:** `ty2017_2024_8949_is_14_rows_per_part` (+ overflow at 14).
- **★ per-form geometric read-back + fault-inject** for EACH new (form,year) — swap two map entries ⇒ RED,
  fails closed (the oracle re-derives per year). `no_unmapped_filled` per (form,year).
- **determinism:** golden sha per (form,year); `map_YYYY_matches_bundled_pdf_fieldset` for 2017 + 2024.
- **8283 revision:** `ty2017_8283_has_no_digital_asset_box_uses_other_with_note`; `ty2024_8283_digital_asset_box`.
- **regression:** the full 2025 suite stays green (no per-year branch breaks 2025).

## Scope / SemVer / lockstep
`btctax-forms` (+`forms/2017/` +`forms/2024/` PDFs & maps + per-year config; NO engine-logic change) + the
`export-irs-pdf` per-year dispatch. Bundled PDFs public domain. MINOR (new supported years). Man page + README
(supported years 2017/2024/2025; 2017 has no DA question + old SE + Box C/F). cargo-tree isolation unchanged.

## Plan (TDD)
- **T1 (2024 — closest to 2025)** — bundle 2024 PDFs; extract + commit the 2024 maps (Box C/F, 14 rows, 1040
  line 7 + DA, unified SE, the applicable 8283 rev); per-year config + the 2024 fill branches; the 2024 KATs +
  fault-injects; confirm 2025 regression green.
- **T2 (2017 — the old forms)** — bundle 2017 PDFs; the 2017 maps incl. the **old short+long Schedule SE** + the
  **no-DA-question line-13 1040** + the 2017-era **8283 with no digital-asset box**; the 2017 fill branches;
  the 2017 KATs + fault-injects; end-to-end on the ReadOnly TY2017 vault; man page + README; whole-diff.

## Gotchas
- **[★ box]** 2017/2024 = Box C/F (old); 2025 = I/L — per-year map data; 14 rows/part (not 11).
- **[★ 1040]** 2017 = line 13 + NO DA question; 2024 = line 7 + DA question; 2025 = 7a. Per-year field ids.
- **[★ SE 2017]** old short+long form (70 fields) — map btctax's SE to the LONG form §B; different line set.
- **[★ 8283]** revision-dated (per-revision maps); the "k Digital assets" box is NEW — a 2017-era 8283 lacks it
  (use the closest box + a note).
- **[oracle]** re-derives bands per year (map-independent) — but supply the per-year logical-sequence config.
- **[regression]** keep the 2025 suite green; prefer per-year config data over `if year==` ladders.
- **[safety]** pseudo ⇒ attestation + DRAFT watermark (unchanged); determinism (golden sha) per (form,year).
