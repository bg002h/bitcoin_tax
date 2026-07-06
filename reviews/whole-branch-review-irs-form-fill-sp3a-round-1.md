# Whole-diff review (Phase E) — feat/irs-form-fill-sp3 STAGE SP3a (T0 engine + T1 2024) — round 1

**Verdict: 0 Critical / 0 Important — SHIP (SP3a).**

Independent Phase-E review of stage SP3a. Diff `main (55f5812)..9643bc6` (the spec commits + the SP3a impl).
Contract: `design/SPEC_irs_form_fill_sp3.md` (R0-GREEN, 3 rounds; round 1 on Fable caught 2017-not-computable +
the 2024 DA-pair fail-closed + the false 8283 recon). SP3a = the per-year engine changes + the full TY2024
packet; **SP3b (2017) is deferred** (a separate branch/merge).

## Fault-injection of the ★ safety net (2024 map restored byte-for-byte)
- **[★ 2024 geometric read-back] CONFIRMED fail-closed.** My independent fault-inject: swapping the 2024 8949
  Row-1 (d)/(e) columns drove the read-back RED — `Geometry("… f1_7: x-center 370.4 not in column 3 band
  (273.6, 337.65) (mis-mapped column)")`, 4 KATs failed, no bytes written. The per-form oracle re-derives its
  bands from the 2024 blank PDF (map-independent). The implementer's suite adds the SE cross/same-column, the
  1040 Yes/No, and the 8283 cross-column fault-injects — all RED.

## Verified by named KAT + inspection (my own runs)
- **[★ R0-C2 — the fix + the regression it risked] DA-question oracle by horizontal adjacency.**
  `da_pair_selected_by_adjacency_not_topmost` PASS — the 2024 filing-status row (dx≈266pt) is skipped, the real
  DA pair `c1_5` (dx≈36pt) selected. **`ty2025_da_still_correct` PASS** — the shared `verify.rs` change does NOT
  regress 2025 (its golden 1040 SHA unchanged). This was the sharpest risk; it's clean.
- **[box] 2024 = Box C/F on `/3`** (`c1_1[2]`/`c2_1[2]`), NOT I/L — the pre-1099-DA taxonomy (this IS the core
  `Form8949Box::{C,F}`); 14 rows/part. Confirmed in the map + the end-to-end output.
- **[1040] cap-gain on line 7** (not 7a); Schedule D line 16 = `f2_01` (2024-specific, caught at extraction);
  8283 Rev. 12-2023 "k Digital assets"; Schedule SE field-identical to 2025 (wage base $168,600 threaded).
- **[per-year engine, no `if year==`]** `Map::for_year`, `SUPPORTED_YEARS=[2024,2025]`, the per-year 8949
  `table_token` (2024 `Table_Line1` vs 2025 `Table_Line1_Part`), `Form1040Map.da_present` (the 2017 no-DA
  extension point, always true here) — the scaffolding SP3b extends.
- **[determinism]** golden sha per 2024 form; end-to-end (pypdf on generated output): XFA=0, NeedAppearances,
  Box C/F=`/3`, DA "Yes"=`/1` with filing-status `/Off`, line 7=45500.50. Pseudo attestation + DRAFT watermark
  reused.

## Suite + isolation
btctax-forms: **64 KATs pass** (18 SP3 + 27 SP2/2025-regression + 14 + 5). Full workspace `cargo test --locked`
= 0 failed (implementer; my independent close-out re-running). clippy -D + fmt clean; `check-isolation` OK
(btctax-forms still no network). Man page + README list 2024. MINOR (new supported year + per-year engine).

## SP3b (2017) — deferred, NOT built (tracked)
The 2017 stage still needs: the `MoneyPair{dollars,cents}` cell (+ 2-dp formatter + `merge_copies` overflow),
the TY2017 `TaxTable` (Rev. Proc. 2016-55 + $127,200) + its full-schedule KAT, per-year QOF-optional, the
`TablePartI` Schedule D token, the pre-filled-exempt set, per-year `SEC_A_CAP`/`SEC_B_CAP` (5/4), and making
the 1040 `da_yes`/`da_no` `Option` (the `da_present=false` guard-skip is already wired). The 2017 PDFs (incl.
8283 Rev. 12-2014 "j Other") are not bundled.

**SHIP SP3a — TY2024 full packet fills correctly (Box C/F, line 7, DA-adjacency), the per-form oracle fails
closed, and 2025 does not regress. SP3b (2017) follows as its own branch.**
