# TY2026 port — ORACLE READINESS lens

**Headline: Tax-Calculator already witnesses TY2026 today with real Rev. Proc. 2025-32 statute (22/22 exact against our own primary-source transcription), OpenTaxSolver cannot until ~2027-01-27, and taxcalc carries three confirmed TY2026 parameter defects — one of which (`AMT_em_pe` = 639,200 vs the draft form's $640,200) is adjudicated wrong against the IRS PDF and is not fixed in the latest release.**

Scope: oracle readiness only. Form-authority, AcroForm maps, constants/indexing and code year-seams belong to other lenses; where a measurement here touches them it is flagged and not pursued.
No `git`/`cargo`/`make`/`nextest` was run. All figures below are pasted tool output.

---

## F1 — Oracle 2 (PSL Tax-Calculator) supports TY2026 **today**, with statute rather than extrapolation

`.venv/lib/python3.12/site-packages/taxcalc/policy.py:43-46`

```
    JSON_START_YEAR = 2013  # remains the same unless earlier data added
    LAST_KNOWN_YEAR = 2026  # last year for which indexed param vals are known
    # should increase LAST_KNOWN_YEAR by one every calendar year
    LAST_BUDGET_YEAR = 2036  # default value of last extrapolation year
```

`LAST_KNOWN_YEAR = 2026` is taxcalc's own assertion that 2026 indexed values are **known law, not projected**. Corroborated by row counts in `policy_current_law.json`:

```
rows per year in policy_current_law.json:
  2023: 218   2024: 218   2025: 249   2026: 260   2027: 5   2028: 5   2029: 13   2030: 16
```

260 explicit TY2026 rows; TY2027 has 5 (sunsets only). **Beyond 2026 taxcalc extrapolates** from `growfactors.csv` (`ACPIU` 2027 = 1.025417) — projections, not law.

**And the 2026 values are right.** Cross-checked, cell by cell, against this repo's own primary-source transcription of Rev. Proc. 2025-32 in `design/SPEC_tax_tables_2026.md:22-37` (a genuinely independent transcription — theirs vs ours, same Rev. Proc.):

```
=== ordinary brackets: taxcalc II_brk1..6 (2026) vs SPEC (Rev. Proc. 2025-32) ===
  single     taxcalc [12400, 50400, 105700, 201775, 256225, 640600]   OK
  mjoint     taxcalc [24800, 100800, 211400, 403550, 512450, 768700]  OK
  widow      taxcalc [24800, 100800, 211400, 403550, 512450, 768700]  OK
  headhh     taxcalc [17700, 67450, 105700, 201750, 256200, 640600]   OK
  mseparate  taxcalc [12400, 50400, 105700, 201775, 256225, 384350]   OK
=== LTCG breakpoints: CG_brk1/CG_brk2 (2026) vs SPEC ===
  single (49450, 545500) OK · mjoint (98900, 613700) OK · widow (98900, 613700) OK
  headhh (66200, 579600) OK · mseparate (49450, 306850) OK
  divergences: 0
```

22 of 22 exact, including the HoH 32%/35% split the spec warns about. `SS_Earnings_c` 2026 = **184,500**, matching the spec's SSA figure exactly.

**It runs.** A live `tc.Records(start_year=2026) / advance_to_year(2026) / calc_all()` on four households succeeded (see F7 for the figures). No code change is needed to ask taxcalc about TY2026.

---

## F2 — Oracle 1 (OpenTaxSolver) cannot witness TY2026 before **~2027-01-27**, and nothing can bring that forward

Installed trees (the only two on this box): `~/OpenTaxSolver2024_22.07_linux64`, `~/OpenTaxSolver2025_23.06_linux64`. Live check of upstream:

```
$ curl -sL https://sourceforge.net/projects/opentaxsolver/files/ | grep -oE "OpenTaxSolver20[0-9]{2}_[0-9]+\.[0-9]+" | sort -u | tail
OpenTaxSolver2025_23.06
```

```
$ curl -sL https://opentaxsolver.sourceforge.net/ | sed 's/<[^>]*>//g'
February 20, 2026 ... Updated versions of OTS for the current tax-year (2025) have been posted
OpenTaxSolver for the current Tax Year (2025) can be downloaded from: ...
```

So the installed 23.06 **is** the newest OTS in existence. The release cadence is measured from the two READMEs, not assumed:

| README line | date |
|---|---|
| `~/OpenTaxSolver2024_22.07_linux64/0_README.txt:72` — `v22.00 (1/27/2025) - Preliminary release for Tax-Year 2024.` | 2025-01-27 |
| `~/OpenTaxSolver2025_23.06_linux64/0_README.txt:60` — `v23.00 (1/27/2026) - Preliminary release for Tax-Year 2025.` | 2026-01-27 |

Two data points, the same calendar day. Fix releases then run for two to three months (2024: v22.07 on 4/28/2025; 2025: v23.06 on 3/19/2026 — and v23.01/v23.02 are where Schedule 1-A actually landed).

**Therefore: earliest OTS TY2026 preliminary ≈ 2027-01-27; earliest settled build ≈ 2027-03.** Until then **every TY2026 figure has at most one witness**, which by this repo's own standing rule (`CLAUDE.md`, "Two oracles") is not validation.

---

## F3 — ★ CONFIRMED taxcalc TY2026 defect, adjudicated against the FORM: MFS AMT zero-exemption point is $1,000 low

`policy_current_law.json`, `AMT_em_pe` ("AMT exemption phaseout ending AMT taxable income for Married filing Separately"):

```
    {'year': 2025, 'value': 900350.0}
    {'year': 2026, 'value': 639200.0}
```

Self-consistency against taxcalc's own other parameters (`AMT_em_ps` + `AMT_em` / `AMT_prt`, the identity this repo already asserts in `FOLLOWUPS.md` G-6f):

```
TY2025: AMT_prt=0.25  MFS em=68,500 ps=626,350  ⇒ implied zero-point = 900,350   taxcalc AMT_em_pe = 900,350   CONSISTENT
TY2026: AMT_prt=0.5   MFS em=70,100 ps=500,000  ⇒ implied zero-point = 640,200   taxcalc AMT_em_pe = 639,200   ** INCONSISTENT by 1,000 **
```

**The form settles it.** `design/forms/2026/f6251--2026-DRAFT.pdf`, text layer, Part I line 4:

```
4    Alternative minimum taxable income. Combine lines 1b through 3. (If married filing separately and line 4 is
     more than $640,200, see instructions.)
```

and Part II line 5's printed table:

```
     Single or head of household . . . . . . $ 500,000 . . . . . $ 90,100
     Married filing jointly or qualifying surviving spouse 1,000,000 . . . . . 140,200
     Married filing separately . . . . . .   500,000 . . . . .    70,100
```

The archived PDF is **byte-identical to what the IRS is serving right now** (`sha256 a547fc9d629e1f04bdc30e088214c716667b88cb1b45447c580c0ae33095b5cc` for both `design/forms/2026/f6251--2026-DRAFT.pdf` and a fresh fetch of `irs.gov/pub/irs-dft/f6251--dft.pdf`; `Form 6251 (2026) Created 5/26/26`), so this is the current, unsuperseded draft.

- **Correct value: $640,200. taxcalc: $639,200. Delta −$1,000.**
- **Direction: taxcalc OVERSTATES AMT for MFS** with AMTI in `[639,200, 640,200)` — `calcfunctions.py:2590` (`if MARS == 3 and c62100 > AMT_em_pe: line5 = 0.`) zeroes an exemption worth up to $500 there, so AMT is up to ~$130 too high.
- **taxcalc's six exemption/threshold amounts otherwise match the form exactly** (90,100 / 140,200 / 70,100 and 500,000 / 1,000,000 / 500,000), so this is one bad cell, not a bad reading.
- **Not fixed upstream** — see F6: `AMT_em_pe` 2026 is 639,200 in 6.8.2 too.
- **The form also independently confirms the 50% exemption phase-out rate**: 500,000 + 70,100/0.5 = 640,200 is the only rate that reproduces the printed threshold. `FOLLOWUPS.md:709-712` calls that "inferable ... but inference is what we forbid encoding" — the printed $640,200 now makes it a transcription rather than an inference.
- **Two independent witnesses**, so this clears the repo's "never file upstream on one oracle" bar: the IRS PDF (primary) and taxcalc's own internal arithmetic.

---

## F4 — The QSS car-loan defect persists into TY2026 unchanged, and it is STRUCTURAL, not a 2026 value

The parameter has exactly **one** row, at the JSON start year, non-indexed:

```
--- AutoLoanInterestDed_ps  (indexed=False) ---
    {'year': 2013, 'MARS': 'single', 'value': 100000.0}
    {'year': 2013, 'MARS': 'mjoint', 'value': 200000.0}
    {'year': 2013, 'MARS': 'mseparate', 'value': 100000.0}
    {'year': 2013, 'MARS': 'headhh', 'value': 100000.0}
    {'year': 2013, 'MARS': 'widow', 'value': 200000.0}
```

So it governs **TY2025 through TY2028 identically** (the OBBBA deductions carry a `{'year': 2029, 'value': 0.0}` sunset row). There is no "TY2026 value" to check — asking about 2026 returns this same row. `verify_schedule_1a.py`'s computed disqualification therefore re-derives itself for TY2026 with no edit to its mechanism.

**Re-adjudicated against the TY2026 form**, not carried over: `f1040s1a--dft.pdf` (`Schedule 1-A (Form 1040) 2026 Created 6/16/26`), Part IV:

```
   32     Enter $100,000 ($200,000 if married filing jointly)
```

A qualifying surviving spouse is not married filing jointly. taxcalc gives a QSS $200,000. **taxcalc stays disqualified as a QSS witness for Part IV in TY2026**, and the direction is an **understatement of tax** (it hands a QSS a deduction the form phases out).

Every other TY2026 Schedule 1-A figure is unchanged from TY2025 — verified from the same draft text layer, not inferred from taxcalc: `$25,000` (L9), `$150,000/$300,000` (L11, L23), `$100`/step (L14, L26), `$12,500 ($25,000 if MFJ)` (L21), `$10,000` (L30), `$200`/step (L35), `$75,000 ($150,000 if MFJ)` (L38), `$6,000` (L39/L41), 6% (L40). *(Other lens: the schedule is RENUMBERED for 2026 — Part IV now at 28-36, Part V at 37-43, and Form 6251 line 1a cites Sch 1-A **line 43**, where taxcalc 6.8.2's comment still cites line 37.)*

---

## F5 — Same-class sweep: two more TY2026 anomalies, one of them new

Two computed detectors were run over the whole policy file (no hand lists).

**(a) Every parameter where QSS ≠ single at TY2026** — 19 hits. Fifteen are correct law because the provision's own words include a surviving spouse (§55(d) AMT exemption and phase-out — confirmed verbatim on the 2026 form above; §1(h)/§1(j) brackets; §63(c)(2)(A) and §63(f) standard deduction; §1411(b) NIIT). Four give QSS the **joint-return** amount where the provision says "joint return":

| parameter | QSS gets | reaches a line btctax computes? |
|---|---|---|
| `AutoLoanInterestDed_ps` | 200,000 | **YES** — F4, live disqualification |
| `CTC_ps` | 400,000 | no — btctax does not compute CTC (`crates/btctax-core/src/tax/advisories.rs:306` "CTC/ODC NOT COMPUTED") |
| `PT_qbid_taxinc_gap` | 150,000 | no — `RefuseReason::QbiAboveThreshold` (`crates/btctax-core/src/tax/return_refuse.rs:301`) |
| `ALD_BusinessLosses_c` | 512,000 | no — §461(l) is not modelled (grep for `461` in `crates/btctax-core/src/tax/*.rs` finds only unrelated literals) |

Not adjudicable from local sources (no §24/§199A/§461 under `legal/`), so they are listed as candidates, not findings — but only the first bears on btctax at all.

**(b) Parameters whose CROSS-STATUS equality pattern CHANGES between 2025 and 2026** — 5 hits. `CDCC_po2_step_size`, `CDCC_ps2`, `STD_charity_ded_nonitemizers_max` change pattern only because the provision turns on in 2026 (legitimate). `II_brk4`'s new HoH split (201,750 vs single 201,775) is **real law** — it matches `design/SPEC_tax_tables_2026.md` exactly. The fifth is new:

```
  PT_qbid_taxinc_thd   2025 ('=', 'x2', '=', '=', '=')
                       2026 ('=', 'x2', '?', '=', '=')
      2025 {'single': 197300.0, 'mjoint': 394600.0, 'mseparate': 197300.0, 'headhh': 197300.0, 'widow': 197300.0}
      2026 {'single': 201750.0, 'mjoint': 403500.0, 'mseparate': 201775.0, 'headhh': 201750.0, 'widow': 201750.0}
```

**`PT_qbid_taxinc_thd` gives MFS $201,775 in 2026 where single, HoH and QSS all get $201,750, and MFJ is exactly 2 × 201,750 = 403,500.** $201,775 is precisely `II_brk4`'s single/MFS value for 2026 — a bracket start bleeding into a §199A threshold that was uniform across all non-joint statuses in 2025. High-confidence one-cell transcription contamination; **also unfixed in 6.8.2**. Out of btctax's reach today (it refuses above the QBI threshold) but it would surface in a TY2026 census as an unexplained $25 boundary divergence.

**Also measured — the 11 non-indexed TY2026 statutory changes taxcalc models**, several of which land on btctax's Schedule A / §170 surface:

```
  AMT_prt                    0.25 -> 0.5          [AMT exemption phaseout rate]
  ID_AllTaxes_c              40,000 -> 40,400 (MFS 20,000 -> 20,200)   [SALT cap]
  ID_Charity_frt             0.0 -> 0.005         [0.5%-of-AGI floor on itemized charity]
  ID_reduction_rate          0.0 -> 0.05405405    [OBBBA itemized-deduction reduction, "about equal to 2/37"]
  STD_charity_ded_nonitemizers_max  0 -> 1,000 (2,000 MFJ)  [above-the-line charity for non-itemizers]
  PT_qbid_taxinc_gap         50,000 -> 75,000 (100,000 -> 150,000 MFJ)
  CDCC_po1_rate_max / po1_rate_min / po2_rate_min / po2_step_size / CDCC_ps2   [dependent care — not btctax]
```

`ID_reduction_rate` is taxcalc's own admitted approximation: 2/37 = 0.05405405405405406, its value is 4.05e-9 low, so taxcalc's reduction is fractionally small and its tax fractionally low. Sub-cent at any realistic amount, but it is a tolerance item for an exact-cent census, not something to encode.

---

## F6 — taxcalc 6.8.2 FIXES #3108; we are three releases behind, and the upgrade costs nothing in TY2026 policy

```
$ .venv/bin/pip index versions taxcalc
taxcalc (6.8.2) ... INSTALLED: 6.7.2   LATEST: 6.8.2
```

6.8.2 downloaded to scratch (not installed into `.venv`) and diffed:

```
params ADDED in 6.8.2: []
params REMOVED in 6.8.2: ['RPTC_c', 'RPTC_rt']
params whose EFFECTIVE TY2026 value differs 6.7.2 -> 6.8.2: 0
```

**Zero TY2026 policy differences.** But `calcfunctions.py` changed (300 diff lines), and the AMTI block is rewritten:

```python
    sch1a_amount = (tip_income_deduction + overtime_income_deduction + auto_loan_interest_deduction)
    if standard > 0.0:
        # the standard deduction subtracted on line 1b is added back on
        # line 2a (IRC 56(b)(1)(E)), so it nets out of AMTI entirely
        c62100 = c00100 - e00700 - qbided - sch1a_amount
```

That is exactly the defect `scripts/oracle/verify_f6251.py:26-38` documents as PSLmodels/Tax-Calculator#3108, and `_taxcalc_expected_gaps` (`verify_f6251.py:174-...`) exists to excuse. **It is fixed.** Measured on the same three households, 6.7.2 vs 6.8.2 run in-process:

| household (TY2026) | AMTI 6.7.2 | AMTI 6.8.2 | AMT 6.7.2 | AMT 6.8.2 |
|---|---:|---:|---:|---:|
| single, W-2 120k + LTCG 900k | 1,003,900 | 1,020,000 | 9,444 | **13,630** |
| MFJ, W-2 200k + LTCG 1.5M | 1,667,800 | 1,700,000 | 17,288 | **25,660** |
| MFS, W-2 50k + LTCG 600k | 633,900 | 650,000 | 4,175 | **9,180** |

Consequences, all foreseen by the harness's own design:
- taxcalc becomes a **real AMTI witness for standard-deduction filers** for the first time.
- `_amti_verdict` reds by design — its docstring says "the day taxcalc fixes #3108 this reds and tells us." Expect that, and delete gap #1 and the `STANDARD_DEDUCTION` excuse table rather than widening a tolerance.
- 6.8.2 also folds the Schedule 1-A deductions into AMTI, which the TY2026 form requires (line 1a subtracts Sch 1-A line 43) — so on TY2026 the *old* engine would be wrong twice over.
- SPEC §11 gates golden regeneration on the engine version, so this is a regenerate-and-review event.

---

## F7 — TY2026 is a materially bigger AMT year than TY2025, for exactly btctax's population

Same engine (6.8.2), same households, year varied:

| household | AMT TY2025 | AMT TY2026 |
|---|---:|---:|
| single, W-2 120k + LTCG 900k | 13,333 | 13,630 |
| MFJ, W-2 200k + LTCG 1.5M | 18,556 | **25,660** (+38%) |
| MFS, W-2 50k + LTCG 600k | 0 | **9,180** (0 → owing) |

Mechanism, measured: `AMT_prt` 0.25 → 0.5 and `AMT_em_ps` MFJ 1,252,700 → 1,000,000 (both explicit 2026 rows). A large-LTCG product is the population this hits hardest, and the MFS row is the case with the fewest witnesses (F2, F3).

---

## F8 — The double-oracle harness is still TY2024. There is no TY2025 baseline to port from.

```
crates/btctax-core/tests/goldens/full_return_goldens.json  _provenance:
  oracle_1_version: OpenTaxSolver 2024 (OpenTaxSolver2024_22.07_linux64)
  oracle_2_version: 6.7.2
  tax_year: 2024
  generated: 2026-08-22
```

```
crates/btctax-core/src/tax/fixtures/form6251_vectors.json
  vector count: 31   by year: {2024: 31}   by status: {'mfj': 16, 'mfs': 5, 'single': 6, 'hoh': 4}
```

Corpus filing statuses are **Single and Married/Joint only** (`scripts/oracle/corpus.py`); HoH, MFS and QSS never enter the golden matrix. So TY2026 would be the **second** year-port of the oracle harness, not the first — the TY2025 port has not been done, even though OTS 2025 is installed, final, and correct on the §55(d)(3) MFS rule that TY2024 could not witness (`FOLLOWUPS.md:700-708`).

---

## F9 — Year-seams inside the oracle harness itself (all measured)

| where | what | consequence |
|---|---|---|
| `scripts/oracle/ots_direct.py:658-665` | `version()` returns `f"OpenTaxSolver 2024 v…"` regardless of `OTS_YEAR`. Pasted run against the 2025 tree: `OpenTaxSolver 2024 (OpenTaxSolver2025_23.06_linux64)` | It feeds `gen_goldens.py:511 "oracle_1_version"`, and SPEC §11 gates regeneration on that string — **the version gate cannot see the change it exists to catch.** B1-class: an instrument that cannot discriminate. |
| `scripts/oracle/gen_goldens.py:518` | `"tax_year": 2024` hardcoded; `:506` oracle_1 prose says "OpenTaxSolver 2024" | Baked provenance would misdescribe a TY2026 golden. |
| `scripts/oracle/gen_goldens.py:196` | `TAXCALC_EXACT_YEARS = frozenset({2025})` | **Must include 2026** (and 2027/2028 — the sunset row is at 2029). Without `exact`, `calcfunctions.py:1074/1094/1115` smooths the Schedule 1-A step and diverges by up to $100 (Parts II/III) or $200 (Part IV) at every MAGI that is not a $1,000 multiple. My TY2026 probe ran with `exact` **off**, exactly as a naive year bump would. |
| `scripts/oracle/ots_direct.py:122,129` | sale dates hardcoded `6-01-2024` / `2-01-2024` in `_capgains_rows` | OTS derives short vs long from these dates; a TY2026 return carrying 2024 dates is at best confusing and at worst mis-binned. |
| `scripts/oracle/verify_f6251.py:56-66` | `STANDARD_DEDUCTION` holds 2024 and 2025 only, and raises a *named* KeyError otherwise | Correctly **fail-closed**. Needs a TY2026 row with a primary-source cite. |
| `scripts/oracle/verify_schedule_1a.py:58` | `def _rows(pol, name, year=2025)`, `BTCTAX` literals are TY2025, header prints "(TY2025)" | Needs a year parameter and a TY2026 literal set. Note the fallback semantics ("latest row at or before `year`") are right for step functions but mean a wrong `year` argument prints OK rather than complaining. |
| `scripts/oracle/ots_direct.py:237-238` | `OTS_YEARS_WITH_STALE_MFS_KICKER = frozenset({2024})`, `OTS_YEARS_WITHOUT_CASH_CEILING = frozenset({2024})` | Correctly year-scoped already; TY2026 needs its own read of `taxsolve_US_1040_2026.c` when it exists — the sets must not be widened speculatively. |

---

## F10 — What the IRS has actually published for TY2026, checked live today (2026-09-05)

| document | status |
|---|---|
| `irs.gov/pub/irs-dft/f6251--dft.pdf` | **TY2026**, `Created 5/26/26`, sha256 `a547fc9d…` — byte-identical to `design/forms/2026/f6251--2026-DRAFT.pdf` |
| `irs.gov/pub/irs-dft/f1040s1a--dft.pdf` | **TY2026**, `Created 6/16/26` |
| `irs.gov/pub/irs-dft/f1040--dft.pdf` | still **TY2025** ("1040 U.S. Individual Income Tax Return 2025") — no TY2026 draft 1040 |
| `irs.gov/pub/irs-dft/i6251--dft.pdf` | still **TY2025** (contains `$900,350`) — **no TY2026 Form 6251 instructions** |

This makes `FOLLOWUPS.md:709-714` reason (1) — "the 2026 instructions are unpublished, so the phase-out rate, zero-exemption thresholds and kicker rate/cap are unknown" — **half stale**. The *form* now states the zero-exemption threshold ($640,200) and forces the exemption phase-out rate (50%). The **MFS kicker rate and cap still live only in the instructions and remain unpublished.**

---

# CAN BE DONE TODAY (no IRS dependency)

1. **Upgrade taxcalc 6.7.2 → 6.8.2.** Zero TY2026 policy change (measured), fixes #3108, makes taxcalc a real AMTI witness for standard-deduction filers, and models Schedule 1-A in AMTI as the TY2026 form requires. Expect `verify_f6251.py::_amti_verdict` to red *by design*; the response is to delete expected-gap #1 and the `STANDARD_DEDUCTION` excuse table, not to widen a tolerance. Regenerate + re-review the golden per SPEC §11.
2. **File the `AMT_em_pe` TY2026 defect upstream** (639,200 → 640,200). Two independent witnesses already in hand — the IRS draft form's printed line 4, and taxcalc's own `AMT_em_ps`/`AMT_em`/`AMT_prt` arithmetic — so it clears the repo's "never file on one oracle" bar that #3108 did not. Include that the same identity holds exactly for TY2025.
3. **File or encode `PT_qbid_taxinc_thd` MFS 2026 = 201,775.** Unfixed in latest; encode as a *computed* disqualification (MFS threshold ≠ the other non-joint statuses) rather than a vector-name excuse.
4. **Extend `verify_schedule_1a.py` to TY2026.** The TY2026 draft Schedule 1-A is published and every figure is unchanged (F4) — this is a `year` parameter plus a second `BTCTAX` literal set transcribed from `f1040s1a--dft.pdf`. The QSS disqualification re-derives itself with no mechanism change.
5. **Add 2026 (and 2027, 2028) to `TAXCALC_EXACT_YEARS`** before any TY2026 census runs. This is the single most likely source of a spurious "btctax rounding defect".
6. **Fix `ots_direct.version()`** to report the tree's actual year, and de-hardcode `gen_goldens.py`'s `tax_year` / oracle_1 prose. Until then the engine-version gate is blind to the exact transition it guards. Pair with a planted-defect test per B1.
7. **Build the TY2026 Form 6251 vector set now, marked explicitly single-witness.** taxcalc runs at 2026 today and matches all six printed exemption/threshold amounts on the draft form; the vectors can be authored and pinned against the form, then re-scored against OTS in 2027 without re-authoring. This is the biggest schedule win available: the expensive part is vector construction, not the second oracle.
8. **Add the TY2026 standard deduction to `verify_f6251.py`** from Rev. Proc. 2025-32 §4.02 (the owner already holds that PDF — it is the source for `SPEC_tax_tables_2026.md`). taxcalc's 16,100 / 32,200 / 16,100 / 24,150 / 32,200 gives the cross-check, but the cite must be the Rev. Proc.
9. **Decide the TY2026 itemized/charitable story while there is still time to collect inputs.** taxcalc models all of it — `ID_Charity_frt` 0.5% AGI floor, `ID_reduction_rate` 2/37, `STD_charity_ded_nonitemizers_max` 1,000/2,000, SALT 40,400/20,200 — and all four touch btctax's Schedule A and §170 surface. taxcalc is therefore *available* as a witness for each; whether btctax models them is another lens's question.
10. **Do the TY2025 oracle port first.** OTS 2025 is installed, final and correct on §55(d)(3); the golden corpus and all 31 AMT vectors are still TY2024. A TY2025 two-oracle baseline is the thing TY2026 gets diffed against, and it is fully achievable today. Widen the corpus past Single/MFJ while doing it (HoH, MFS, QSS never enter the golden matrix, and QSS is where the one live taxcalc disqualification lives).

# MUST WAIT — and on exactly what

1. **A second TY2026 witness at all.** Gated on **OpenTaxSolver2026 (v24.00)**, expected ~**2027-01-27** from two measured data points, with a settled build ~2027-03. No substitute exists on this box or upstream. Until then every TY2026 figure is single-witness, and by the repo's own rule that is not validated.
2. **The MFS AMT kicker rate and cap for TY2026.** Gated on **i6251 (2026)** — the `--dft` slot still serves TY2025. The form gives the $640,200 threshold and forces the 50% *exemption* phase-out; it says nothing about the kicker rate, and `FOLLOWUPS.md` G-6f split those two rates precisely because they need not move together.
3. **Naming the lines a TY2026 census compares.** Gated on the **TY2026 Form 1040** — still TY2025 at the draft URL. Form 6251 line 1a/1b and line 2a cite 1040 lines 14, 11b and 12e; those cross-references cannot be verified against a form that does not exist.
4. **Anything pinned to a FINAL TY2026 form.** Drafts are replaced in place and withdrawn; the archived sha256 is the identity of what was reviewed, not a retrievable document.
5. **TY2027 in any form.** taxcalc's `LAST_KNOWN_YEAR` is 2026; past it, values are CPI projections from `growfactors.csv`, not law. The TY2027 Rev. Proc. is expected ~October 2026 — about six weeks out, and it will move `LAST_KNOWN_YEAR` only on the next taxcalc release.
6. **Any upstream fix to the three taxcalc TY2026 defects.** All three are present in the current latest release (6.8.2), so waiting buys nothing; they must be carried as computed disqualifications.
