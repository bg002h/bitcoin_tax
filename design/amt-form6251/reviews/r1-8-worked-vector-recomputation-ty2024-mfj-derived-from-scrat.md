# r1 review — §8 worked-vector recomputation — TY2024 MFJ, derived from scratch against f6251--2024, i6251--2024, and i1040gi--2024 (fetched)

**Headline:** Five of the six §8 vectors verify exactly against the 2024 primary documents, but V5 is wrong — the correct AMT is $26,271.00, and the plan's "≈$28,000" reproduces precisely the AMT-slice Part III stacking the plan itself disfavors — and the §1 peak-exposure figure ($24,615), the §3.3 worked example ($75,812.50/$55,897.50, irreproducible), and V1's balance-due assumption all need correction before the vectors become the oracle.

## [Critical] §8 V5 (and §1) — WRONG

**Claim:** V5 (wages 250,000, LTCG 2,000,000, standard): AMT "≈28,000"; §1: "at $250,000 of wages and a $2M gain the AMT is about $28,000"

**Authority:** Form 6251 (2024) line 20: "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet … (as figured for the regular tax)"; line 18: 26% up to $232,600, else 28% minus $4,652; i1040gi Sch Y-1: 196,669.50 + 37% over 731,200

**Fix:** Correct AMT = 26,271.00. Full derivation: AGI 2,250,000; standard 29,200 (no itemized deductions exist); TI 2,220,800; ordinary TI 220,800; gain split 0%/15%/20% = 0 / 362,950 / 1,637,050; regular tax 420,929.50; AMTI 2,250,000; exemption 0 (AMTI ≥ 1,751,900); taxable excess 2,250,000; L17 = 250,000 → L18 = 65,348; TMT 447,200.50; AMT 26,271.00. The plan's ≈28,000 reproduces EXACTLY the AMT-slice stacking (TMT 448,660.50, AMT 27,731.00) — the reading §3.3 itself deems wrong. As written, the archetypal-user KAT would bake the disfavored Part III positioning into the oracle and overstate tax by 1,460. Replace ≈28,000 with 26,271.00 (conditional on T1 confirming the line-20 reading, which the printed form supports).

---

## [Important] §8 V1 note — INCOMPLETE

**Claim:** "balance due $83,225.50" against "$300,000 withheld"

**Authority:** Form 8959 line 24 → Form 1040 line 25c (Additional Medicare Tax withholding is credited as federal income tax withholding); total tax = 364,675.50 + 19,000 + 6,750 = 390,425.50

**Fix:** 83,225.50 is correct ONLY if payments = 300,000 (W-2 box 2) PLUS the mandatory employer Additional-Medicare withholding of 7,200 (0.9% × (1,000,000 − 200,000), Form 8959 L24 → 1040 L25c), i.e. 307,200 total. If "$300,000 withheld" means total payments, balance due = 90,425.50. The KAT must state the withholding composition (box 2 = 300,000; box 6 = 21,700 of which 7,200 is Additional Medicare) or the oracle is ambiguous by exactly 7,200.

---

## [Important] §3.3 / T1 — UNSOUND

**Claim:** "I computed two different answers for the same taxpayer ($1M wages / $10M gain / $1M donation): $75,812.50 … and $55,897.50" — a "$19,915 spread on one return"; T1 must "Resolve the $75,812.50 vs $55,897.50 question"

**Authority:** QDCG wksht line 5 = 0 for that return (TI 10,000,000 = gain 10,000,000; gift 1,000,000 < 60%×11,000,000, fully deductible); Form 6251: L12 = 10,000,000, L16 = 10,000,000, L17 = 0 — the gain stacks from 0 under BOTH candidate readings

**Fix:** Neither figure is reproducible. For that taxpayer ordinary taxable income is $0 in both systems, both stackings coincide, TMT = regular tax = 1,956,705.00, AMT = 0. There is no spread on that return. T1's acceptance criterion targets phantom numbers; re-anchor it on the vectors where the two readings actually diverge: V2 (TMT 113,654.50 line-20/regular-position vs 106,989.50 AMT-slice) and V5 (AMT 26,271.00 vs 27,731.00).

---

## [Important] §1 — WRONG

**Claim:** "AMT plateaus and peaks near $24,615 at ~$384,000 of ordinary taxable income" ("Exposure is bounded")

**Authority:** Form 6251 line 39: 28% − $4,652 on taxable excess; i1040gi TCW Section B: 37% − 73,874.50; peak sits at the 24%→32% bracket kink, $383,900. AMT gap = 0.28×(ordTI + add-back) − 4,652 − T(ordTI)

**Fix:** Two errors. (a) The zero-add-back peak is 24,619.00 at exactly 383,900 (24,615 is the value at 384,000, past the kink). (b) It is not a bound: the peak grows by 0.28 × add-back. A standard-deduction filer peaks at 32,795.00 (add-back 29,200), a SALT-capped itemizer at 27,419.00. The plan's own V5 (26,271.00) already exceeds the claimed 24,615 peak — an internal contradiction. State the peak as add-back-dependent, max ≈ 32,795 for v1's input space, or the figure will seed a wrong sanity bound.

---

## [Minor] §1 — INCOMPLETE

**Claim:** "AMT is owed when the exemption is fully phased out (AMTI ≥ $1,751,900 MFJ) and ordinary taxable income is below $769,139"

**Authority:** 0.28w − 4,652 = 0.37w − 73,874.50 → w = 769,138.89 (i1040gi TCW Section B subtraction 73,874.50; Form 6251 line 39 subtraction 4,652)

**Fix:** As a sufficient condition the sentence is true, and 769,139 is verified — but only as the zero-add-back (itemizer, $0 SALT) crossover. "The crossover" is add-back-dependent: 0.09w = 69,222.50 + 0.28×add-back → 800,250 for a SALT-capped itemizer, 859,983 for a standard-deduction filer (note V3/V4, both standard, actually straddle 859,983, not 769,139; a standard filer at ordinary TI 800,000 owes ~5,398 of AMT despite being above 769,139). State the condition.

---

## [Minor] T1 — INCOMPLETE

**Claim:** "Encode the resolution as three KATs against the worked vectors in §8 … a failing test per band"

**Authority:** Form 6251 lines 20–21: L21 = max(0, 94,050 − L20); every §8 vector has regular ordinary TI ≥ 220,800 > 94,050, so L21 = 0 in all six

**Fix:** Only V2 and V5 are sensitive to the §3.3 resolution at all, and neither exercises the 0% band (L23 = 0 for all six). "A failing test per band" cannot be satisfied from §8 as-is; add a low-wage vector (e.g., wages ≈ 120,000 + multi-million gain, ordinary TI < 94,050) so the 0%-band positioning is pinned.

---

## [Minor] T2 (compute_6251 signature) — INCOMPLETE

**Claim:** compute_6251(…, regular_tax, …) — regular tax passed as a single opaque figure

**Authority:** Form 6251 line 10: "Add Form 1040 … line 16 … and Schedule 2 … line 1z. Subtract from the result Schedule 3 (Form 1040), line 1 …"; i6251 line 8 ("Do I need to fill out line 8?")

**Fix:** v1 admits a §904(j) foreign tax credit up to $600 MFJ (only larger FTCs are refused). Line 10 subtracts Schedule 3 line 1, and line 8 carries the AMTFTC; under the simplified limitation the two cancel and line 11 is unchanged, but the Tier-2 printed lines 8/9/10 would be wrong if the signature ignores FTC. Either thread the FTC through or document that v1's FTC input is $0-only. (Adjacent to my §8 lane; flagging for the Part I/II reviewer.)

---

## [CONFIRMED_CORRECT] §8 V1 — CORRECT

**Claim:** TI 1,415,000; regular 364,675.50; TMT 327,965.00; AMT 0; NIIT 19,000; AddMed 6,750

**Authority:** Form 6251 lines 2a (Sch A line 7 = 0, no taxes), 5 (exemption 133,300 − 0.25×196,300 = 84,225), 18 (0.28×830,775 − 4,652 = 227,965); i1040gi Sch Y-1

**Fix:** No change. AGI 1,500,000; gift 85,000 < 60% ceiling 900,000, itemizing wins (85,000 > 29,200); ordinary TI 915,000; gain all-20% (100,000); AMTI 1,415,000; taxable excess 1,330,775; TMT 327,965.00 (identical under both Part III readings — insensitive); AMT 0. NIIT = 3.8%×min(500,000; 1,250,000) = 19,000; AddMed = 0.9%×750,000 = 6,750. Balance due: see the Important finding.

---

## [CONFIRMED_CORRECT] §8 V2 — CORRECT

**Claim:** TI 750,000; regular 129,397.50; TMT blank pending T1; AMT 0

**Authority:** Form 6251 line 5 (AMTI 750,000 < 1,218,700 → full 133,300 exemption); line 20 = QDCG wksht line 5 = 250,000; QDCG wksht lines 6/13 (94,050 / 583,750)

**Fix:** All stated figures confirmed; gift 750,000 < 60% ceiling 900,000 (fully deductible, correctly). Ordinary TI 250,000; gain split 0/333,750/166,250; taxable excess 616,700; L17 = 116,700 → 26% = 30,342. TMT both ways as requested: 113,654.50 under the line-20-as-printed (regular-position) reading; 106,989.50 under the AMT-slice reading. AMT = 0 either way (regular 129,397.50 exceeds both), so the row's AMT 0 is robust to the T1 outcome.

---

## [CONFIRMED_CORRECT] §8 V3 — CORRECT

**Claim:** TI 10,970,800; regular 2,285,321.50; TMT 2,275,348.00; AMT 0 (margin 9,973.50)

**Authority:** i6251 Exemption Worksheet note (exemption zero at AMTI ≥ 1,751,900); Form 6251 line 18: 0.28×1,000,000 − 4,652 = 275,348; gain all-20% = 2,000,000

**Fix:** No change — exact to the cent, including the 9,973.50 canary margin. Ordinary TI 970,800; AMTI 11,000,000; taxable excess 11,000,000; insensitive to the Part III reading (ordinary > 583,750 in both systems).

---

## [CONFIRMED_CORRECT] §8 V4 — CORRECT

**Claim:** TI 10,670,800; AMT 15,818.50 (regular tax and TMT left blank)

**Authority:** i1040gi Sch Y-1: 111,357 + 35% over 487,450; Form 6251 line 18: 0.28×700,000 − 4,652 = 191,348

**Fix:** AMT 15,818.50 confirmed. Fill the blanks: regular tax 2,175,529.50 (T(670,800) = 175,529.50 + 20%×10,000,000); AMTI 10,700,000; exemption 0; taxable excess 10,700,000; TMT 2,191,348.00. Insensitive to the Part III reading.

---

## [CONFIRMED_CORRECT] §8 V6 — CORRECT

**Claim:** TI 10,750,000; regular 2,203,625.50; TMT 2,205,348.00; AMT 1,722.50

**Authority:** Form 6251 line 2a (Sch A line 7 = 0 → add-back 0 for the itemizer); line 18: 0.28×750,000 − 4,652 = 205,348

**Fix:** No change — exact. AGI 11,000,000; gift 250,000 ≪ 60% ceiling 6,600,000; itemizing wins; ordinary TI 750,000 in BOTH systems (donation is AMT-allowed, no add-back), gain all-20%; AMTI 10,750,000; exemption 0; taxable excess 10,750,000. The donation-triggers-AMT mechanism (deduction lowers regular tax while AMTI falls dollar-for-dollar but TMT falls only 28¢) is real and correctly captured.

---

## [CONFIRMED_CORRECT] §1 / §3.2 — CORRECT

**Claim:** Exemption fully phased out at "AMTI ≥ $1,751,900 MFJ"

**Authority:** i6251 (2024) Exemption Worksheet note, quoted: "If Form 6251, line 4, is equal to or more than … $1,751,900 if married filing jointly or qualifying surviving spouse … your exemption is zero."

**Fix:** No change; also equals 1,218,700 + 133,300/0.25 from the worksheet's own arithmetic.

---

## [CONFIRMED_CORRECT] §8 (all vectors) — CORRECT

**Claim:** Deduction handling: 60%-of-AGI cash charitable ceiling and itemize-vs-standard choice

**Authority:** §170(b)(1)(G) (60% of contribution base for cash gifts); 2024 MFJ standard deduction $29,200

**Fix:** No mishandling found in any vector: ceilings 900,000 / 900,000 / 6,600,000 never bind (85,000; 750,000; 250,000); itemizing correctly wins V1/V2/V6 and standard correctly applies V3/V4/V5; the AMT add-back is correctly 0 for the no-SALT itemizers and 29,200 for standard takers.

---

