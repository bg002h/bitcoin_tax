# r1 review — 2024 Form 6251 / i6251 primary-source verification of Part III band positioning (plan §3.3) and the §8 vectors

**Headline:** Part III's capital-gain bands ARE positioned by the regular return (Form 6251 lines 20 and 27, "as figured for the regular tax"), but BOTH of the plan's candidate TMTs are wrong: lines 16/22 cap the preferential slice at the taxable excess, so the disputed vector's TMT is $70,005.00 — and the §8 V2 row is a different vector whose TMT is $113,654.50.

## [CONFIRMED_CORRECT] §3.3 (positioning question) — CORRECT

**Claim:** Part III line 20 pulls a regular-return figure, so the gain's rate bands are positioned by the regular-return ordinary bottom, not the AMT ordinary slice.

**Authority:** Form 6251 (2024) line 20: "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet or the amount from line 14 of the Schedule D Tax Worksheet, whichever applies (as figured for the regular tax). If you did not complete either worksheet for the regular tax, enter the amount from Form 1040 or 1040-SR, line 15; if zero or less, enter -0-." Line 27 (15% band) repeats it: "...line 5 of the Qualified Dividends and Capital Gain Tax Worksheet or ... line 21 of the Schedule D Tax Worksheet ... (as figured for the regular tax)". QDCGW (i1040 2024, line 16 worksheet) line 5 = "Subtract line 4 from line 1. If zero or less, enter -0-" = the REGULAR return's ordinary taxable income. Contrast line 13's parenthetical "(as refigured for the AMT, if necessary)": the gain AMOUNT is AMT-side, the band POSITIONS are regular-side. Line-by-line sources, 12–40: L12 AMT taxable excess (L6); L13 AMT-refigured QDCGW L4 (= regular amounts for every v1 input per i6251 Part III, "None of the statements apply ... Use the regular tax amounts to complete lines 13, 14, and 15"); L14 AMT-refigured Sch D L19 (always 0 in v1); L15 = L13; L16 = smaller of L12/L15 (AMT cap); L17 = L12−L16 (AMT ordinary slice, never negative); L18 = 26/28%−$4,652 on L17; L19 = $94,050; L20 REGULAR ordinary income; L21 = L19−L20 floor 0; L22 = smaller of L12/L13 (AMT cap); L23 = smaller of L21/L22 @0%; L24 = L22−L23; L25 = $583,750; L26 = L21; L27 REGULAR ordinary income; L28 = L26+L27; L29 = L25−L28 floor 0; L30 = smaller of L24/L29; L31 = 15%; L32 = L23+L30 (if = L12, skip 33–37); L33 = L22−L32; L34 = 20%; L35–37 §1250 25% tranche, skipped when L14 = 0; L38 = 18+31+34+37; L39 = 26/28%−$4,652 on ALL of L12; L40 = smaller of L38/L39 → line 7.

**Fix:** Keep the positioning conclusion; T1's PART_III.md should cite lines 20/27's "(as figured for the regular tax)" against line 13's "(as refigured for the AMT)".

---

## [Critical] §3.3 / §8 V2 — WRONG

**Claim:** "...which indicates the bands are positioned by the regular bottom, making $75,812.50 the correct one."

**Authority:** Form 6251 line 16: "Enter the smaller of line 12 or line 15"; line 22: "Enter the smaller of line 21 or line 22... " preceded by line 22 = "smaller of line 12 or line 13"; and the line-32 rule: "If lines 32 and 12 are the same, skip lines 33 through 37 and go to line 38." Recomputed disputed vector (MFJ 2024; wages 1,000,000; LTCG 500,000 — the "$1M gift limited to $900,000 by the 60% ceiling" fixes AGI at 1,500,000, so the §3.3 label "$10M gain" is wrong; deduction 900,000; TI 600,000): regular tax = 87,918.50 (12,106.00 ordinary + 72,562.50 @15% + 3,250.00 @20%). 6251: L4 AMTI = 600,000 (no add-back; itemizer, Sch A L7 = 0); L5 exemption = 133,300 (AMTI < 1,218,700); L6 = 466,700. Part III: L12 466,700; L13 500,000; L15 500,000; L16 466,700; L17 0; L18 0; L19 94,050; L20 100,000; L21 0; L22 466,700; L23 0; L24 466,700; L25 583,750; L26 0; L27 100,000; L28 100,000; L29 483,750; L30 466,700; L31 70,005.00; L32 466,700 = L12 → skip 33–37; L38 70,005.00; L39 126,024.00; L40 = TMT = 70,005.00; AMT = max(0, 70,005.00 − 87,918.50) = 0. The plan's $75,812.50 is the regular QDCGW preferential tax on the FULL 500,000 gain — it ignores the L16/L22 excess caps and overstates TMT by 5,807.50; the $55,897.50 alternative repositions the bands at the AMT slice and understates by 14,107.50. Both candidates are wrong; the question was a false dichotomy.

**Fix:** §3.3 must state: bands positioned regular-side AND preferential slice capped at min(taxable excess, net capital gain) with the ordinary slice floored at 0 and the L32=L12 skip. The correct TMT for the disputed (TI 600,000) vector is $70,005.00. Encoding $75,812.50 as a KAT would ship a rule that overstates the filed Schedule 2 line 2 whenever taxable excess < net capital gain and AMT > 0, and over-refuses under Tier 1.

---

## [Important] §3.3 ↔ §8 V2 (vector identity) — WRONG

**Claim:** V2's blank TMT "is the vector whose value depends on the §3.3 resolution ($75,812.50 vs $55,897.50)"; §3.3 labels the disputed taxpayer "$1M wages / $10M gain / $1M donation".

**Authority:** Arithmetic against the form: the two candidate figures arise only from the TI-600,000 vector (donation 1,000,000 capped at 900,000 = 60% × AGI 1,500,000). The §8 V2 row as printed (donation 750,000, TI 750,000, regular tax 129,397.50 — regular tax verified: 46,085.00 ordinary + 50,062.50 @15% + 33,250.00 @20%) is a different vector: L4 AMTI 750,000; L5 133,300; L6 616,700; L12 616,700; L13/15 500,000; L16 500,000; L17 116,700; L18 30,342.00 (26%, ≤ 232,600); L20 250,000; L21 0; L22 500,000; L24 500,000; L27/28 250,000; L29 333,750; L30 333,750; L31 50,062.50; L32 333,750 ≠ L12; L33 166,250; L34 33,250.00; L38 113,654.50; L39 168,024.00; L40 = TMT = 113,654.50; AMT = 0. And the literal "$1M wages / $10M gain / $1M donation" taxpayer (TI 10,000,000) has TMT = 1,956,705.00 — matching neither candidate. The plan conflates three vectors.

**Fix:** Fill §8 V2's blank with TMT = $113,654.50 (AMT $0 stands). Correct §3.3's vector label to wages 1,000,000 / LTCG 500,000 / donation 1,000,000→900,000 (TI 600,000), and record that vector's true figures (regular tax 87,918.50, TMT 70,005.00, AMT 0) as its own KAT — it is the excess-smaller-than-gain exemplar and must be pinned.

---

## [Important] §3.3 last paragraph (upper-bound rejection) — UNSOUND

**Claim:** The regular-position-stack upper bound "must not be adopted: the bound is only valid while the add-back is smaller than the exemption, and it fails exactly where the margin is thinnest — at $1M wages / $10M gain the exemption is $0 while the add-back is $29,200."

**Authority:** The rejection is the right engineering decision, but the stated reason is false. The stack (26/28% on max(0, excess − gain) + the regular return's preferential tax on the full gain) is a TRUE upper bound on line 40 for every v1-reachable input, unconditionally: the true form uses the same regular-side band positions (lines 20/27), taxes only min(L12, L13) ≤ full gain at preferential rates (lines 16/22, and preferential tax is monotone in the amount), and line 40's "smaller of line 38 or line 39" can only lower it further. Validity never hinges on add-back vs exemption. At the plan's own cited point ($1M wages / $10M gain, standard deduction = V3) excess 11,000,000 ≥ gain 10,000,000, so the bound is EXACT (2,275,348.00) and clears correctly against 2,285,321.50 — it does not fail there. The same misconception — treating the uncapped stack as the exact answer rather than an upper bound — is what produced the $75,812.50 error.

**Fix:** Keep the rejection, restate the reason: Tier 2 must fill lines 12–40 exactly anyway, T3's refusal message names the exact dollar amount, and the §8 KATs pin exact TMTs — a second, approximate code path buys nothing and adds risk. Delete the "only valid while the add-back is smaller than the exemption" sentence and the false V3 failure claim.

---

## [Important] T2 / T7 / §3 (lines 8–10, FTC) — INCOMPLETE

**Claim:** amt = max(0, TMT − regular_tax); compute_6251 takes regular_tax with no FTC input; Form6251Lines/emitter fill Parts I–III with no line-8/line-10 handling.

**Authority:** v1 accepts foreign tax credits up to the §904(j) $300/$600 no-Form-1116 ceiling (only amounts above it are refused). i6251 (2024), Line 8: "If you made an election to claim the foreign tax credit on your 2024 tax return without filing Form 1116, your AMTFTC is the same as the foreign tax credit on Schedule 3 (Form 1040), line 1. Enter that amount on Form 6251, line 8." Form 6251 line 10: "Add Form 1040 or 1040-SR, line 16 (minus any tax from Form 4972), and Schedule 2 (Form 1040), line 1z. Subtract from the result Schedule 3 (Form 1040), line 1..."; line 9 TMT = line 7 − line 8; line 11 = line 9 − line 10. For a §904(j) filer the FTC cancels in line 11 (AMT and the Tier-1 refuse/proceed decision are unchanged), but Tier 2's PRINTED lines 8, 9, and 10 would each be wrong by the FTC amount if the plan's formula is filled as-is: line 8 blank, line 9 and line 10 each overstated.

**Fix:** Add the Schedule 3 line 1 (FTC) amount as an input to compute_6251/Amt6251; fill line 8 = that amount, line 9 = line 7 − line 8, line 10 = 1040 L16 − Sch 3 L1 (Sch 2 line 1z is structurally $0 in v1). Note in the plan that the net AMT is invariant to it, so Tier 1 needs no behavior change — only the printed chain does. Also honor i6251's rule that when line 10 ≥ line 7, line 8 is left blank and line 11 = 0.

---

## [CONFIRMED_CORRECT] orchestrator sub-question: taxable excess < net capital gain — CORRECT

**Claim:** (Not raised by the plan) What Part III does when the AMT taxable excess is smaller than the net capital gain.

**Authority:** The form handles it structurally, with no special case: line 16 "Enter the smaller of line 12 or line 15" caps the preferential slice at the excess, so line 17 (= L12 − L16) is exactly 0 — never negative; line 18 = 0; line 22 "smaller of line 12 or line 13" applies the same cap inside the band arithmetic so only min(excess, gain) flows through the 0/15/20 bands; and when the whole excess fits in the 0/15 bands, line 32 = line 12 triggers "If lines 32 and 12 are the same, skip lines 33 through 37 and go to line 38" — the 20% tranche never engages even though the regular return taxed part of the same gain at 20%. This occurs in the DISPUTED (TI-600,000, donation-900K) vector — excess 466,700 < gain 500,000, all 466,700 taxed at 15%, TMT 70,005.00 — but NOT in the §8 V2 row as printed (excess 616,700 > gain 500,000).

**Fix:** T1's PART_III.md and the KAT set must include the TI-600,000 vector explicitly as the excess<gain case; the plan currently has no vector exercising the L16 cap or the L32=L12 skip.

---

## [CONFIRMED_CORRECT] §8 V1, V3, V4, V6 (and V1's pins) — CORRECT

**Claim:** V1: reg 364,675.50 / TMT 327,965.00 / AMT 0; V3: 2,285,321.50 / 2,275,348.00 / AMT 0, margin 9,973.50; V4: AMT 15,818.50; V6: 2,203,625.50 / 2,205,348.00 / AMT 1,722.50; V1 NIIT 19,000, Additional Medicare 6,750.

**Authority:** Recomputed each to the cent against Form 6251 lines 1–11 and Part III as walked above. V1: AMTI 1,415,000, exemption 84,225 (= 133,300 − 25% × 196,300), excess 1,330,775, L17 830,775 → L18 227,965.00, L29 0, L33 500,000 → L34 100,000, L38 = 327,965.00 ✓. V3: AMTI 11,000,000, exemption 0, L17 1,000,000 → 275,348.00, L33 10,000,000 → 2,000,000, TMT 2,275,348.00 ✓, margin 9,973.50 ✓. V4: TMT 2,191,348.00 − reg 2,175,529.50 = 15,818.50 ✓. V6: L17 750,000 → 205,348.00 + 2,000,000 = 2,205,348.00; AMT 1,722.50 ✓. NIIT = 3.8% × 500,000 = 19,000 ✓; AddlMed = 0.9% × 750,000 = 6,750 ✓. Also confirmed: 2024 Schedule 2 line 2 = "Alternative minimum tax. Attach Form 6251", line 3 = "Add lines 1z and 2 → 1040 line 17" — the plan's Sch 2 L2→L3→L17 plumbing and Form 6251 line 11's own "Enter here and on Schedule 2 (Form 1040), line 2" agree. §3.2's exemption arithmetic and the 1,751,900/232,600 figures check out; §3.1's exhaustiveness holds for Part I (line 2b state-refund income has no v1 input; every other 2b–3 line maps to a §2 refusal).

**Fix:** None — these four vectors are safe to use verbatim as KATs.

---

## [Minor] §8 V5 and §1 (exposure bound) — WRONG

**Claim:** V5 AMT "≈28,000" / §1 "at $250,000 of wages and a $2M gain the AMT is about $28,000" and "AMT plateaus and peaks near $24,615 at ~$384,000 of ordinary taxable income".

**Authority:** Exact V5 (recomputed per the form): regular tax 420,929.50 (39,077.00 + 54,442.50 + 327,410.00); TMT 447,200.50 (L17 250,000 → 65,348.00; L29 362,950 → 54,442.50; L33 1,637,050 → 327,410.00); AMT = 26,271.00. And the "$24,615 peak" is the zero-add-back (itemizer, no SALT) case only: for a standard-deduction filer AMT(w) = 0.28w − 4,652 − regtax(w − 29,200), which peaks at ≈ $32,795 at wages ≈ 413,100 (ordinary TI 383,900). The plan's own V5 (26,271) already exceeds its claimed 24,615 plateau — internally inconsistent. §1's crossover rule ("AMT owed when fully phased out AND ordinary TI < $769,139") is correct as a sufficiency (verified: 0.28t − 4,652 = regtax(t) at t = 769,139), but the bound quoted next to it is understated.

**Fix:** Fill V5 with the exact 26,271.00 (§8 says vectors are used verbatim as KATs — "≈28,000" cannot be one), and restate §1's exposure bound as ≈$32,795 for standard-deduction filers ($24,615 is the no-add-back floor case).

---

## [Minor] §3.2 — INCOMPLETE

**Claim:** taxable_excess = AMTI − exemption // Form 6251 line 6

**Authority:** Form 6251 line 6: "Subtract line 5 from line 4. If more than zero, go to line 7. If zero or less, enter -0- here and on lines 7, 9, and 11, and go to line 10." The plan's formula omits the zero floor; a below-exemption AMTI would yield a negative excess and a negative TMT instead of 0.

**Fix:** taxable_excess = max(0, AMTI − exemption), with lines 7/9/11 forced to 0 in that branch.

---

