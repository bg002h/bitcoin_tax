# SPEC — bundled tax tables for TY2026

**Source baseline:** `main` @ `f97adac` (branch `feat/tax-tables-2026`). **Review status: DRAFT — awaiting R0.**
Adds the TY2026 bundled `TaxTable` (the deferred "2026/2027 backfill" FOLLOWUP; 2027 is NOT possible — IRS
publishes 2027 figures in fall 2026, after our data horizon). Data-add only; no logic change. **Every figure
below was verified against the PRIMARY source (Rev. Proc. 2025-32 PDF + SSA) by reading the actual tables.**

## Goal
Add `ty2026()` to `crates/btctax-adapters/src/tax_tables.rs` (mirroring `ty2025()`, lines 225-341), wire it into
`BundledTaxTables::table_for` (currently dispatches 2024/2025), and add per-figure KATs (mirroring the TY2025
KATs) that pin every 2026 value to its cite. Arms report/export/optimize/SE for TY2026 (else `TaxTableMissing`).

## PRIMARY-SOURCE figures (Rev. Proc. 2025-32, I.R.B. 2025-45; SSA 2025-10-24; OBBBA Pub. L. 119-21)

### Ordinary brackets — §4.01 Tables 1-4 (lower bound of each rate; `br(lower, rate)`)
| Status | 10% | 12% | 22% | 24% | 32% | 35% | 37% |
|--------|----:|----:|----:|----:|----:|----:|----:|
| **Single** (Tbl 3, §1(j)(2)(C)) | 0 | 12,400 | 50,400 | 105,700 | 201,775 | 256,225 | 640,600 |
| **MFJ/QSS** (Tbl 1, §1(j)(2)(A)) | 0 | 24,800 | 100,800 | 211,400 | 403,550 | 512,450 | 768,700 |
| **HoH** (Tbl 2, §1(j)(2)(B)) | 0 | 17,700 | 67,450 | 105,700 | **201,750** | **256,200** | 640,600 |
| **MFS** (Tbl 4, §1(j)(2)(D)) | 0 | 12,400 | 50,400 | 105,700 | 201,775 | 256,225 | **384,350** |

⚠️ **HoH 32%/35% differ from Single** ($201,750/$256,200 NOT $201,775/$256,225) — do NOT copy Single's top
bands to HoH. **MFS matches Single for 10-35% but 37% starts at $384,350** (= ½ of MFJ $768,700).

### §1(h) LTCG breakpoints — §4.03 (max_zero = top of 0% band; max_fifteen = top of 15% band)
| Status | max_zero | max_fifteen |
|--------|---------:|------------:|
| **Single** (All Other Individuals) | 49,450 | 545,500 |
| **MFJ/QSS** | 98,900 | 613,700 |
| **HoH** | 66,200 | 579,600 |
| **MFS** | 49,450 | 306,850 |

### Ancillary scalars
- `gift_annual_exclusion` = **$19,000** — §2503(b), Rev. Proc. 2025-32 **§4.42(1)** ("the first $19,000 of
  gifts to any person…"; unchanged from TY2025).
- `ss_wage_base` = **$184,500** — §230 (42 U.S.C. §430), SSA announced **2025-10-24** (up from $176,100).
- `gift_lifetime_exclusion` = **$15,000,000** — §2010(c)(3) basic exclusion for decedents dying in CY2026, set
  by **OBBBA Pub. L. 119-21 §70106** (Rev. Proc. 2025-32 §2.14 confirms; a flat statutory figure, first
  inflation-indexed in 2027 — NOT a §1(f) inflation item this year).

### `source` string (mirror ty2025's format)
`"Rev. Proc. 2025-32 §4.01/§4.03 + §4.42 (TY2026); §2010(c)(3) basic exclusion $15,000,000 per OBBBA Pub. L. 119-21 §70106; SS wage base $184,500 per SSA 2025-10-24"`

## Mechanism
- `ty2026()` — copy `ty2025()`'s shape; substitute the figures above. QSS aliases MFJ (no separate entry, via
  `TaxTable::key`). `year: 2026`.
- `table_for` — add the `2026 => Some(&self.ty2026)` arm (mirror how 2024/2025 are stored/dispatched; check
  whether tables are lazily built or stored in a field and follow the existing pattern exactly).
- **Do NOT touch** the year-independent STATUTORY constants (NIIT_RATE, SE rates, niit_threshold,
  se_addl_medicare_threshold, loss_limit, QUALIFIED_APPRAISAL_THRESHOLD) — they are not per-year (I4) and 2026
  does not change them.

## KATs (mirror the ty2025 tests)
- `ty2026_single_ordinary_brackets_match_rev_proc_2025_32`; `ty2026_mfj_...`; `ty2026_hoh_...` (**assert the
  201,750/256,200 that differ from Single**); `ty2026_mfs_37_pct_starts_at_384350`.
- `ty2026_ltcg_breakpoints_all_statuses` (all 4 statuses, both breakpoints).
- `ty2026_gift_annual_exclusion_is_19000`; `ty2026_ss_wage_base_is_184500`; `ty2026_lifetime_exclusion_is_15M`.
- `ty2026_table_is_available` (table_for(2026).is_some()); `ty2025_and_2024_unchanged` (no regression — a
  spot-check that the older tables still return their values).
- `statutory_values_are_not_in_the_table_and_constant_across_years` — extend/confirm it still holds for 2026.
- **★ fault-inject target**: `ty2026_hoh_...` — swapping HoH's 32% start to Single's $201,775 must go RED.

## Scope / SemVer / lockstep
`btctax-adapters` only (one new fn + one dispatch arm + tests). No API change. Additive data → PATCH. No CLI/doc
surface (the man pages don't enumerate years). Update FOLLOWUPS "2026/2027 tax-table backfill" → 2026 DONE, 2027
deferred (data not yet published).

## Plan (TDD)
- **T1** — write the failing KATs with the exact figures above; add `ty2026()` + the `table_for` arm; green.
  Whole-diff (re-verify a sample against the cited Rev. Proc.) + full suite + FOLLOWUPS.

## Gotchas
- **HoH ≠ Single at 32%/35%** ($201,750/$256,200) — the one easy-to-miss transcription trap; KAT + fault-inject it.
- **MFS 37% = $384,350** (½ MFJ), lower bands = Single.
- **Lifetime exclusion is a flat OBBBA $15,000,000**, not an inflation-indexed §1(f) item — cite OBBBA, not a §4.xx.
- **No 2027** — figures unpublished; don't fabricate.
- Exact `dec!()` integers; never a float (NFR5).
