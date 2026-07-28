# r1 review — Form 6251 Part I exhaustiveness — is "AMTI = taxable income + the line-2 add-back, EXACTLY" true for every input the application accepts?

**Headline:** The §3.1 exactness claim is FALSE for two inputs the app already accepts — the state-refund subtraction (Form 6251 line 2b) and the MFS line-4 kicker — and Tier 1's "no attach when AMT is $0" premise fails in the §904(j) FTC window where line 7 > line 10 with line 11 = $0.

## [Critical] §3.1 (and T2's compute_6251 signature) — WRONG

**Claim:** "After fix/amt-screen-line2, worksheet line 3 IS AMTI for every input v1 accepts: AMTI = taxable_income_L15 + amt_worksheet_line2(...)"

**Authority:** Form 6251 line 2b: "Tax refund from Schedule 1 (Form 1040), line 1 or line 8z" (a parenthesized, i.e. negative, entry). i6251 (2024) p.2, Line 2b: "Include any refund from Schedule 1 (Form 1040), line 1, that is attributable to state or local income taxes... Enter the total as a negative amount." The refund is a reachable input: the app captures it and the screen itself subtracts it — crates/btctax-core/src/tax/amt.rs:95-97 computes line5 = line3 − state_refund_and_8z ("worksheet line 4, subtracted — a state refund is not AMT income"). The plan's formula reproduces worksheet lines 1–3 only and drops lines 4–5, contradicting both the form and the app's own reviewed screen.

**Fix:** AMTI = taxable_income_L15 + line2_addback − state_refund_and_8z (the worksheet's AMTI analog is line 5, not line 3). compute_6251 must take the refund parameter; omitting it overstates AMTI by the refund → Tier 1 wrongly refuses true-$0-AMT filers and Tier 2 files an OVERSTATED Form 6251 (wrong filed number). Note for §2: line 2b is outside Who-Must-File condition 4's "lines 2c through 3" range, so the refund never by itself creates an attach obligation. Add a refund-bearing worked vector to §8 (all six current vectors have refund = 0).

---

## [Critical] §3.1/§3.2 (and T2's KAT list) — INCOMPLETE

**Claim:** The AMTI derivation and exemption arithmetic ("exemption = max(0, base − 0.25 × max(0, AMTI − phaseout_start))") are complete per status; the plan reuses amt_should_file_6251's phaseout arithmetic unchanged.

**Authority:** i6251 (2024) p.9, Line 4—Alternative Minimum Taxable Income: "If your filing status is married filing separately and line 4 is more than $875,950, you must include an additional amount on line 4. If line 4 is $1,142,550 or more, include an additional $66,650. Otherwise, include 25% of the excess of the amount on line 4 over $875,950." (§55(d)(3), last sentence.) MFS is a reachable input: FilingStatus::Mfs exists (crates/btctax-core/src/tax/types.rs:13), AmtParams carries MFS exemption/breakpoint rows (tables.rs), and return_inputs.rs:389 has the MFS spouse-itemizes coupling — MFS returns are accepted, not refused. Neither the plan nor the existing screen (amt.rs:86-126) contains this add-on.

**Fix:** Add the MFS line-4 increase to the AMTI derivation: if MFS and AMTI > $875,950, AMTI += min($66,650, 25% × (AMTI − $875,950)), with both constants in AmtParams per §5. Omission understates AMTI by up to $66,650 → TMT understated by up to ~$18,662 → understated tax on a filed return. Add MFS boundary KATs ($875,950 / $1,142,550) to T2 — the current KAT list and all §8 vectors are MFJ-only. When folding, also verify the existing screening worksheet against the corresponding i1040gi line (the screen the plan reuses lacks the kicker too).

---

## [Important] §1 ("Form 6251 need not be attached when AMT is $0 — the 'Who Must File' test is not met"), T3, T9 — UNSOUND

**Claim:** The attach/skip boundary is AMT > 0: T3 proceeds with no form whenever computed amt == 0; T9 says the form "is skipped when AMT is $0 (Who Must File)"; the plan's tax model is amt = max(0, TMT − regular_tax) with no line 8 or line 10.

**Authority:** i6251 (2024) p.1, Who Must File: "Attach Form 6251 to your return if any of the following statements are true. 1. Form 6251, line 7, is greater than line 10. ..." — the test is line 7 vs line 10, NOT line 11 (AMT) > 0. Form 6251 line 9: "Tentative minimum tax. Subtract line 8 from line 7"; line 10: "Add Form 1040 or 1040-SR, line 16 (minus any tax from Form 4972), and Schedule 2 (Form 1040), line 1z. Subtract from the result Schedule 3 (Form 1040), line 1...". i6251 p.10, Line 8: "If you made an election to claim the foreign tax credit... without filing Form 1116, your AMTFTC is the same as the foreign tax credit on Schedule 3 (Form 1040), line 1." The app accepts a §904(j) FTC up to $300/$600 on Sch 3 L1 (return_1040.rs:921-1257), so line 8 is reachable and nonzero: whenever line 10 < line 7 ≤ line 10 + FTC, line 11 = $0 yet condition 1 is TRUE and the form MUST be attached.

**Fix:** Model lines 8–10 explicitly: amtftc = Sch 3 L1 (§904(j) elector rule), line 9 = line 7 − amtftc, line 10 = 1040 L16 − Sch 3 L1 (v1 has no 4972/Sch 2 L1z/8978). Define compute_6251's `regular_tax` as form line 10 — the plan leaves it undefined, and if an after-credits figure is passed without modeling line 8, AMT is overstated by up to $600 (wrong filed number); with both modeled the bottom line is invariant but the printed Tier 2 lines 8/9/10 are correct. Make the refusal/attach boundary the Who-Must-File conditions (in v1 only condition 1 is reachable; conditions 2–3 need Forms 3800/8834/8911/8801 the app lacks, condition 4 needs lines 2c–3 entries the app can't have): Tier 1 must refuse — not proceed — a return with line 7 > line 10 even when AMT = $0; Tier 2 must attach in that window. Amt6251 needs the amtftc/line-10 fields for the emitter.

---

## [Important] §2 (out-of-scope list: "§56(a)(6) disposition basis differences") — UNSOUND

**Claim:** "Each is either refused upstream today or an input v1 never captures, and each must stay refused/absent so the AMTI derivation in §3.1 remains exhaustive."

**Authority:** i6251 (2024) p.4, Line 2k—Disposition of Property: item 4 covers "Capital gain or loss (including any carryover that is different for the AMT)"; and "Because the amount of your gains and losses may be different for the AMT, the amount of any capital loss carryover may also be different for the AMT. To figure your AMT capital loss carryover, fill out an AMT Capital Loss Carryover Worksheet." The app accepts a user-declared regular-tax capital-loss carryforward (capital_loss_carryforward_in, crates/btctax-core/src/tax/compute.rs:317). A carryforward imported from prior years prepared OUTSIDE btctax can carry a divergent AMT twin (e.g., prior ISO dispositions — the i6251 example generates a $62,000 AMT carryover against a $0 regular one). This channel is neither refused upstream nor an input v1 never captures, so §2's dichotomy is false for it. (For lots the app itself tracks, AMT basis = regular basis — no ISO/depreciation/PAB input can exist — so current-year 2k is genuinely unreachable; only the imported carryover diverges.)

**Fix:** State the assumption explicitly in §2/§3.1: "the declared capital-loss carryforward is assumed equal for AMT" — and back it with either a class-(A) declaration/attestation (the pattern already used for the mixed-use mortgage) or a refusal when an externally-originated carryforward coincides with an AMT-range return. If the true AMT carryover is smaller than the regular one, AMTI is understated. Add "a carryforward input with a divergent AMT twin" to §9's guard list.

---

## [Important] §2/§3.1 (mortgage interest absent from the out-of-scope list) — INCOMPLETE

**Claim:** The §56(b)(1) taxes/standard-deduction add-back is the only reachable itemized-deduction adjustment; §2's list is exhaustive over Schedule A.

**Authority:** i6251 (2024) p.8, Line 3—Mortgage Interest: "If you deducted home mortgage interest on Schedule A for a dwelling that isn't a principal residence (within the meaning of section 121) or qualified dwelling for AMT, include that deducted interest on line 3... A qualified dwelling for AMT is a house, apartment, condominium, or mobile home not used on a transient basis. A qualified dwelling for AMT doesn't include house boats and recreational vehicles." (§56(b)(1)(C)/§56(e).) The app accepts Schedule A line 8a mortgage interest (mortgage_interest_1098, return_inputs.rs:299) with no dwelling-type question — only the §163(h)(3)(F) buy/build/improve declaration. §2 never mentions mortgage interest.

**Fix:** Close the gap explicitly. The 8a-only input (1098-reported interest) makes the divergent population narrow — boat loans rarely generate a 1098 and line 8b is not an input — but some RV/houseboat lenders do issue 1098s, and a transient-use dwelling with a 1098 also diverges; when hit, the missed line-3 add-back UNDERSTATES tax. Either add a qualified-dwelling declaration alongside the existing mixed-use one (None ⇒ refuse, matching the app's declaration pattern) or document in §2 precisely why 8a-only is argued to exclude the case, and add "a mortgage input without the qualified-dwelling declaration / any future 8b input" to §9's guard list.

---

## [Minor] §3.1 — INCOMPLETE

**Claim:** Line 1 of the derivation is taxable_income_L15 (floored at $0 by the 1040).

**Authority:** Form 6251 (2024) line 1: "Enter the amount from Form 1040 or 1040-SR, line 15, if more than zero. If Form 1040 or 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from line 11 of Form 1040 or 1040-SR and enter the result here. (If less than zero, enter as a negative amount.)"

**Fix:** Record the zero-TI negative rule in §3.1/PART_III.md. It cannot change any v1 outcome — when L15 = 0, AMTI ≤ the add-back ≤ $29,200 under either reading, far below every exemption, so AMT = 0 and no attach condition triggers — but the derivation should mirror the form it claims to implement.

---

## [Nit] §3.2 — INCOMPLETE

**Claim:** taxable_excess = AMTI − exemption // Form 6251 line 6

**Authority:** Form 6251 (2024) line 6: "Subtract line 5 from line 4. If more than zero, go to line 7. If zero or less, enter -0- here and on lines 7, 9, and 11, and go to line 10."

**Fix:** Floor taxable_excess at 0 (the downstream max(0, TMT − regular_tax) hides the sign error, but line 6 as printed in Tier 2 must be -0-, and Part III line 12 consumes it).

---

## [CONFIRMED_CORRECT] §3.1 (§199A sentence) — CORRECT

**Claim:** "The §199A deduction is allowed for AMT (§199A(f)(2)), so it stays subtracted."

**Authority:** IRC §199A(f)(2) (Coordination with minimum tax): qualified business income is determined "without regard to any adjustments under sections 56 through 59" — i.e., the deduction is identical for AMT. Form 6251 (2024) confirms structurally: line 1 starts from 1040 line 15, which is already net of the line-13 QBI deduction, and no Part I line adds it back. Reachable via box5_section_199a REIT dividends (return_inputs.rs:100) and handled correctly.

**Fix:** None.

---

## [CONFIRMED_CORRECT] §3.1 (charitable exhaustiveness) — CORRECT

**Claim:** The charitable deduction can never differ between the regular tax and AMT for v1 inputs.

**Authority:** Three channels all null: (1) the §57(a)(6) appreciated-property preference is repealed (pre-1993). (2) i6251 (2024) p.9, Related Adjustments, triggers only on "an entry on line 2c because you deducted investment interest allocable to an interest in a trade or business, or on line 2d, 2h, 2i, or 2k through 2t, or... line 3 from pre-1987 depreciation, pollution control facilities, or tax shelter farm activities" — lines 2a/2b do NOT trigger it — and even then only for items "based on a limit of income other than adjusted gross income (AGI) or modified AGI"; the §170(b) charitable limit is AGI-based, so it is never refigured. (3) i6251 p.8, Charitable Contributions of Certain Property, requires "a different basis for AMT purposes," which no v1 input can create. The plan's repeal citation is right; adding the Related-Adjustments quote would make the argument airtight.

**Fix:** Optionally add the Related-Adjustments citation to §3.1's rationale when folding.

---

## [CONFIRMED_CORRECT] §2 (the rest of Part I, lines 2c–2t and line 3) — CORRECT

**Claim:** Every other Form 6251 Part I adjustment/preference is refused upstream or has no input.

**Authority:** Walked line-by-line against i6251 (2024): 2c (§4952 refused), 2d, 2e/2f (NOL/ATNOL refused), 2g (i6251 p.3 confirms the exact 1099-INT box 9 / 1099-DIV box 13 sourcing §2 cites), 2h (§1202), 2i (ISO), 2j (K-1 box 12 code A), 2l (depreciation) — all refused per the settled list; 2m/2n require passive/at-risk activities that cannot arise (Schedule C losses refused, no Sch E/F/4835 input); 2o/2q/2r (circulation, mineral-mining §616/617, R&E) have no input; 2p/2s/2t refused per §2. Line 3's other items (pre-1987 depreciation, pollution control, tax-shelter farm, Form 8990, biofuel credit, net qualified disaster loss) have no input; the 2024 form has no medical adjustment line (TCJA aligned the 7.5% floor). The exactness claim fails ONLY on the channels in the findings above; with those folded (refund term, MFS kicker, lines 8–10, the carryover attestation, the dwelling declaration), the derivation is exhaustive for v1's input surface, and §9's guard should name the break conditions: any Sch 1 line 8z input, any Sch A line 8b or non-declared-dwelling mortgage, any FTC beyond the §904(j) election, any passive/at-risk activity, any externally-imported carryforward, and any MFS path that bypasses the line-4 kicker.

**Fix:** Fold the §9 guard list as stated.

---

