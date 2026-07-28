# r1 review — §3.4 — §53 minimum tax credit / Form 8801 obligation

**Headline:** §3.4 is CONFIRMED correct against the primary authorities: the taxes/standard-deduction add-back is a §56(b)(1) exclusion item under §53(d)(1)(B)(ii)(I), Form 8801 Part I line 15 equals the entire AMT so Part II lines 18 and 21 are $0, no §53 credit accrues, and per i8801's Who Should File the filer is not even directed to complete Form 8801 — Tier 2 may file without creating a next-year obligation, provided the code carries the exclusion-only invariant.

## [CONFIRMED_CORRECT] §3.4 — CORRECT

**Claim:** AMT computed by btctax generates a $0 §53 minimum tax credit and no Form 8801 is ever required, because the only AMT adjustment (the §56(b)(1) taxes/standard-deduction add-back) is an exclusion item and every deferral item is refused or uncapturable.

**Authority:** 26 U.S.C. §53(d)(1)(B)(i): adjusted net minimum tax = net minimum tax "reduced by the amount which would be the net minimum tax for such taxable year if the only adjustments and items of tax preference taken into account were those specified in clause (ii)"; clause (ii) specifies "(I) the adjustments provided for in subsection (b)(1) of section 56, and (II) the items of tax preference described in paragraphs (1), (5), and (7) of section 57(a)". When the ONLY items present ARE specified items, the reduction equals the whole net minimum tax, so the creditable amount is $0. i8801 (2024), Specific Instructions: "The minimum tax credit is allowed only for the AMT caused by deferral items."

**Fix:** None — keep as written. Carry the invariant from the last finding so a future input cannot silently break it, and keep §9's mitigation (Tier 2 refuses if any non-exclusion adjustment ever becomes reachable).

---

## [CONFIRMED_CORRECT] §3.4 (classification) — CORRECT

**Claim:** The §56(b)(1)(A)(ii) taxes add-back and the §56(b)(1)(E) standard-deduction add-back are exclusion items.

**Authority:** §53(d)(1)(B)(ii)(I) specifies the whole of §56(b)(1) — which contains both (A)(ii) taxes and (E) standard deduction — as exclusion-side items. i8801 (2024) Line 2: "Exclusion items are only the following AMT adjustments and preferences: certain itemized deductions …, certain tax-exempt interest, depletion, the section 1202 exclusion, the standard deduction, and any other adjustments related to exclusion items. … Combine lines 2a, 2b, 2c, 2d, 2g, and 2h of your 2023 Form 6251." 2024 Form 6251 line 2a is verbatim "If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; otherwise, enter the amount from Form 1040 or 1040-SR, line 12" — exactly amt_worksheet_line2.

**Fix:** None.

---

## [CONFIRMED_CORRECT] §3.4 (Form 8801 Part I/II walk) — CORRECT

**Claim:** A filer whose only AMT arose from the line-2a add-back computes a zero credit on Form 8801.

**Authority:** Form 8801 (2024): line 1 ("Combine lines 1 and 2e of your 2023 Form 6251") + line 2 (exclusion items = 6251 line 2a here) = line 4 = the filer's full AMTI; lines 5–13 rerun the exemption phase-out and 26/28%/Part-III tax on that identical AMTI, so line 13 = 6251 line 9 (TMT); line 14 = "the amount from your 2023 Form 6251, line 10"; line 15 (net minimum tax on exclusion items) = 13−14 = 6251 line 11 = the ENTIRE AMT. Part II: line 16 = 6251 line 11; line 17 = line 15; line 18 = 16−17 = 0; lines 19–20 = 0; line 21 = 0. i8801: "File Form 8801 only if line 21 is more than zero", and Who Should File bullet 1 requires "An AMT liability and adjustments or preferences other than exclusion items" — unmet, so the form is not even required to be completed. The app's ≤$600 §904(j) FTC cancels symmetrically: i8801 Line 12 — the MTFTCE "is the same as the foreign tax credit on your 2023 Schedule 3 (Form 1040), line 1". (Verified on the 2024 edition, which looks back at TY2023; the 2025 edition that would look back at TY2024 is not yet published, but the structure is statutory and stable.)

**Fix:** None. Optionally note in §3.4 that the conclusion is stronger than "credit = $0": under i8801 Who Should File, the filer is not directed to complete Form 8801 at all.

---

## [CONFIRMED_CORRECT] §3.4 (interactions) — CORRECT

**Claim:** No interaction — capital loss carryforward, charitable carryover, or later-year state-tax refund — can make a deferral component appear.

**Authority:** Capital loss carryforward: an AMT/regular carryover divergence requires an AMT basis difference, and every source (6251 line 2i ISO, 2k disposition-of-property, 2l depreciation) is refused with no AMT-basis input, so 8801 line 28's "(as refigured for the AMT, if necessary)" is a no-op and 6251 line 2k is structurally 0 in every year. Charitable carryover: charitable contributions have no §56/§57 adjustment under current law (the former §57(a)(6) appreciated-property preference was repealed in 1993) — they appear nowhere on 6251 lines 2a–3 or Form 8801. State-tax refund: 2024 Form 6251 line 2b ("Tax refund from Schedule 1 (Form 1040), line 1 or line 8z") is a §56(b)(1)(D) recovery and sits INSIDE i8801 Line 2's exclusion-item combine list ("Combine lines 2a, 2b, 2c, 2d, 2g, and 2h") — it stays in the exclusion bucket forever.

**Fix:** None.

---

## [Minor] §3.4 ("no Form 8801 is ever required") — INCOMPLETE

**Claim:** "AMT computed by btctax generates a $0 credit and no Form 8801 is ever required."

**Authority:** i8801 (2024) Who Should File, bullet 2: complete Form 8801 if you had "A credit carryforward to 2024 (on 2023 Form 8801, line 26)". A user arriving with a pre-btctax MTC carryforward (e.g., a 2022 ISO exercise) meets that bullet independently of anything this plan does — but that obligation exists in v1 today, is not created by Tier 2, and skipping it only forfeits a beneficial credit (overpays; never understates tax).

**Fix:** Scope the sentence to "btctax-computed AMT creates no NEW Form 8801 obligation"; a pre-existing minimum-tax-credit carryforward is an already-unsupported input, unchanged by this plan.

---

## [Minor] §3.1 (flagged from the §3.4 lens) — INCOMPLETE

**Claim:** AMTI = taxable_income_L15 + amt_worksheet_line2 is exhaustive for every input v1 accepts.

**Authority:** 2024 Form 6251 line 2b: "Tax refund from Schedule 1 (Form 1040), line 1 or line 8z" — entered as a NEGATIVE adjustment. If v1 accepts (or ever accepts) a taxable state-tax-refund input for a later year, §3.1's formula omits the 2b subtraction and OVERSTATES AMTI/AMT — conservative direction (wrongly refuses under Tier 1 or overstates a filed L17 under Tier 2), never an understatement, and irrelevant to Form 8801. Moot if no such input exists.

**Fix:** The §3.1 reviewer should confirm whether a Schedule 1 line 1 state-refund input exists; if it does, add line 2b to the AMTI derivation (or refuse refund-bearing itemizer-history returns).

---

## [Minor] §3.4 / §9 (the guard the plan should name) — INCOMPLETE

**Claim:** The plan asks review to confirm §3.4 but does not yet state the code-level invariant that keeps it true.

**Authority:** Follows from §53(d)(1)(B)(ii)(I) and i8801 Line 2: the conclusion holds exactly as long as every AMT adjustment the app makes lives inside Form 6251 lines 2a/2b (§56(b)(1)), i.e., lines 2c–2t and 3 are structurally zero.

**Fix:** Carry this one-line assertion in form6251.rs: `debug_assert_eq!(amti - taxable_income, line2_addback, "every AMT adjustment must be a §56(b)(1) exclusion item (F6251 line 2a/2b); any other adjustment creates a §53 credit / Form 8801 obligation btctax cannot discharge — Tier 2 must refuse instead of filing")` — plus one KAT that recomputes Form 8801 Part I lines 1–15 from each §8 AMT-owed vector and asserts line 15 == Form 6251 line 11 (net minimum tax on exclusion items equals the whole AMT, so 8801 line 18 = 0), wired to §9's source-scan guard over §2's refusal list.

---

