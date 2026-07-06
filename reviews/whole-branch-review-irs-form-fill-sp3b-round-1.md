# Whole-diff review (Phase E) — feat/irs-form-fill-sp3b STAGE SP3b (T2 = 2017) — round 1

**Verdict: 0 Critical / 0 Important — SHIP (SP3b). This closes SP3 and task #45.**

Independent Phase-E review of stage SP3b. Diff `main (1c48ff8)..c7e2401` — 3 commits (part-1 tax table `9c6ce2a`
+ part-2 packet `5ea2a8a` + docs `c7e2401`; part-2 was a fresh implementer after part-1's session-limit stop).
Contract: `design/SPEC_irs_form_fill_sp3.md` (R0-GREEN, 3 rounds). SP3b = the TY2017 full packet (the old forms
+ the new engine pieces the DRAFT wrongly called "data-only").

## Fault-injection of the ★ safety net (2017 SE map restored byte-for-byte)
- **[★ the NEW MoneyPair oracle] CONFIRMED fail-closed.** My independent fault-inject: swapping the 2017
  Schedule SE `line2` ↔ `line12` MoneyPair definitions (far apart in y) drove the read-back RED — `Geometry(
  "ordinal-y descent broken: f2_39 (y 282) is not strictly above f2_9 (y 534) — mis-mapped row/line")`, 5 KATs
  failed, no bytes written. The dollars/cents pair is verified at the dollars-field geometry; a mis-mapped pair
  cannot escape. (Earlier I also confirmed the 2024 8949 column-swap RED.) The implementer's own
  `fault_injected_2017_{1040,8283}_moneypair_swap_is_red` + SE cross/same-column injects all RED.

## Verified by named KAT + inspection (my own runs)
- **[★ TY2017 tax table — the tax-critical part, cross-checked vs Rev. Proc. 2016-55] correct.** The committed
  Single/MFJ/HoH/MFS ordinary edges (9325/37950/91900/191650/416700/418400 …) + §1(h) LTCG breakpoints
  (37950/418400 …) + **`ss_wage_base = $127,200`** match the primary source; `ty2017_table_matches_rev_proc_2016_55`
  PASSES; NIIT correctly EXCLUDED (the year-independent `niit_threshold()` fn, never a `TaxTable` field).
- **All 7 named 2017 KATs pass:** `ty2017_and_2024_bitcoin_use_box_c_f`, `money_pair_splits_dollars_and_cents`,
  `ty2017_1040_line13_no_da_question`, `ty2017_schedule_d_has_no_qof`, `ty2017_schedule_se_long_form_section_b`,
  `ty2017_se_prefilled_constants_are_exempt`, `ty2017_8283_rev_2014_uses_j_other_with_note`.
- **[box] 2017 = Box C/F on `/3`** (not I/L); 14 rows/part. **[1040] line 13** (glitched `f1-_51`), **NO DA
  question** (income-only 2017 ⇒ skip). **[SE] §B long form**, line 12 = SS+Medicare (0.9% correctly off-form
  for 2017), $400 floor, factory constants (line 7 $127,200 / line 14) pre-filled-exempt. **[8283] Rev.12-2014
  "j Other" = `p1-cb4[8]`/`/9`** + a printed note (no digital-asset box exists), 5/4 rows, 26 cent fields,
  overflow. **[Schedule D] no-QOF, `TablePartI` token** (per-year, no underscore).
- **[engine, no `if year==`]** `MoneyPair`/`MoneyCell` + `fmt_money_pair` (2-dp zero-pad); per-year
  `SCHED_D_TABLE_TOKEN`, QOF-`Option`, 1040 DA-`Option` gated on `da_present`, pre-filled-exempt set, 8283 caps
  from `map.rows.len()`; `SUPPORTED_YEARS` += 2017. cents field renamed as a unit by `merge_copies`.
- **[determinism/e2e]** golden sha per 2017 form; the implementer's CLI end-to-end (pypdf on generated output):
  XFA=0, NeedAppearances, Box C/F=`/3`, line 13 = `45500`/`50` (dollars+cents correct), "j Other"=`/9`, no DA
  question. (ReadOnly held raw CSVs not a vault → a synthetic 2017 fixture, as the spec permits.)

## Suite + isolation + regression
btctax-forms: **84 KATs pass** (20 SP3b/2017 + 18 SP3a/2024 + 27 SP2/2025 + 14 + 5) — **2017 + 2024 + 2025 all
green**. Full workspace `cargo test --locked` = **1316 passed / 0 failed** (implementer; my close-out
re-running). clippy -D + fmt clean; `check-isolation` OK (btctax-forms still no network — MoneyPair/lopdf pure
Rust). Man page + README list 2017. MINOR (new supported year + new tax table + MoneyPair engine).

**SHIP SP3b — the TY2017 full packet fills correctly (Box C/F, line-13 no-DA, §B SE, "j Other" 8283, dollars+
cents), the MoneyPair oracle fails closed, the 2017 tax table is primary-source-correct, and 2024/2025 do not
regress. This completes SP3 and task #45 — the full IRS packet across 2017 / 2024 / 2025.**
