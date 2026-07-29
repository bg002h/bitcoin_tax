# E2 review r1 — mechanical fact-check of the six load-bearing claims

**Reviewer:** independent agent (sonnet tier — mechanical verification per the tiering rule).
**Subject:** commit on `feat/amt-e2-vector-population` ("amt(E2): a population of AMT-owing vectors,
and three routings proved dead").
**Brief:** verify six named factual claims against primary sources; TRUE/FALSE/UNVERIFIABLE with
file:line or command output; no design review, no improvement proposals.

**Persisted VERBATIM before folding**, per `STANDARD_WORKFLOW.md` §2.

---

1. TRUE — `.venv/lib/python3.12/site-packages/taxcalc/calcfunctions.py:2571-2583` builds `c62100` (AGI − adjustments ± preferences) with no MARS-conditional term anywhere in the block; the only `MARS == 3` conditional in the whole AMT function is at `:2590` (`if MARS == 3 and c62100 > AMT_em_pe: line5 = 0.`), which zeros the exemption (`line5`), not `c62100`. `grep -n "MARS == 3"` over the file shows this is the sole MFS branch in the AMT path — confirms the exemption cliff exists but no line-4 AMTI add-back does.

2. TRUE — `~/OpenTaxSolver2024_22.07_linux64/src/taxsolve_US_1040_2024.c:270-275`:
```
270: if ((status == MARRIED_FILING_SEPARAT) && (amtws[4] > 831150.0))
272:   if (amtws[4] > 1084150.0)
273:    amtws[4] = amtws[4] + 63250.0;
275:    amtws[4] = amtws[4] + 0.25 * (amtws[4] - 831150.0);
```
Constants 831150/1084150/63250 match the 2023 (not 2024) §55(d)(3) MFS thresholds; falls inside the claimed 265-280 range.

3. TRUE — `TaxRateFunction`, `taxsolve_US_1040_2024.c:122-137`: for `income < 100000`, bin width `x=50` (except near-zero income), `dx = 0.5*x = 25`, `k = income/x`, `x = x*k + dx` (bin midpoint), `tx = (int)(TaxRateFormula(x,status)+0.5)` (round to whole dollar). Arithmetic: deviation of income from midpoint is `< dx = 25`; top marginal rate is 37%, so schedule-value error `< 25*0.37 = 9.25`; nearest-integer rounding adds `≤ 0.5` (claim's "+1" is a safe over-bound covering this ≤0.5, actual tight bound ≈9.75). `9.25 + 1 = 10.25` — the stated bound holds (and is not violated even by the tighter true bound).

4. TRUE — `609350 + 4*66650 = 875950` (verified via `.venv/bin/python`). `design/amt-form6251/PART_III.md:220-222` quotes the line-4 kicker verbatim ("more than $875,950 … include an additional amount"), and `:231-234` quotes the exemption-worksheet note ("$875,950 if married filing separately, your exemption is zero"); `:236-238` states explicitly: "the collision: MFS's zero-exemption threshold and the MFS kicker start are the same $875,950."

5. TRUE — test `bps()` in `crates/btctax-core/src/tax/form6251.rs:465-484` (Single 47025/518900, Mfj|Qss 94050/583750, Mfs 47025/291850, HoH 63000/551350) matches the production `ltcg` map in `crates/btctax-adapters/src/tax_tables.rs:380-408` exactly (Single 47025/518900 at :383-384, Mfj 94050/583750 at :390-391, HoH 63000/551350 at :397-398, Mfs 47025/291850 at :405-406).

6. TRUE — `git diff HEAD~1 HEAD -- crates/btctax-core/src/tax/fixtures/form6251_vectors.json` shows exactly 2 `-` lines total (the `--- a/...` diff header and one `_note` string) and 1218 `+` lines (the `_note` replacement plus appended vectors). Parsing both revisions with `.venv/bin/python`/`json` confirms V1, V2, V2b, V3, V4, V5, V6, V7, V8, V9, V10 are identical dicts before/after, and the new content is exactly 19 new vectors (V11–V29) appended after V10 — consistent with the 11+19=30 total from the settled KAT/oracle run.

VERDICT: 6 true, 0 false, 0 unverifiable
