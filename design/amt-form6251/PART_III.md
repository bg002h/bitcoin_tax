# Form 6251 (2024) — line-by-line transcription

**T1 deliverable.** Source: `https://www.irs.gov/pub/irs-prior/f6251--2024.pdf`, stashed at
`crates/btctax-forms/forms/2024/f6251.pdf` (sha256 `7fea4e42…9a`, 103,554 bytes, `/AcroForm` present —
so T6's map has a real target). US Government work, public domain (17 U.S.C. §105).

Transcribed from the form itself, not from memory or from a reviewer's quotation
(`CLAUDE.md`, "Transcribe IRS forms"). **`src` column:** where each input comes from —
`AMT` = refigured for the AMT · `REG` = *as figured for the regular tax* · `—` = self-contained.
**`v1` column:** `✓` implemented · `0` structurally zero (no input surface) · `REF` refused upstream.

---

## Part I — Alternative Minimum Taxable Income

| # | Printed text | src | v1 |
|---|---|---|---|
| 1 | Enter the amount from Form 1040 or 1040-SR, line 15, if more than zero. If Form 1040 or 1040-SR, line 15, is zero, subtract line 14 of Form 1040 or 1040-SR from line 11 of Form 1040 or 1040-SR and enter the result here. **(If less than zero, enter as a negative amount.)** | REG | ✓ |
| 2a | If filing Schedule A (Form 1040), enter the taxes from Schedule A, **line 7**; otherwise, enter the amount from Form 1040 or 1040-SR, line 12 | REG | ✓ |
| 2b | Tax refund from Schedule 1 (Form 1040), line 1 or line 8z | REG | ✓ **negative** |
| 2c | Investment interest expense (difference between regular tax and AMT) | — | 0 |
| 2d | Depletion (difference between regular tax and AMT) | — | 0 |
| 2e | Net operating loss deduction from Schedule 1 (Form 1040), line 8a. Enter as a positive amount | — | 0 |
| 2f | Alternative tax net operating loss deduction | — | 0 |
| 2g | Interest from specified private activity bonds exempt from the regular tax | — | **REF** |
| 2h | Qualified small business stock, see instructions | — | 0 |
| 2i | Exercise of incentive stock options (excess of AMT income over regular tax income) | — | 0 |
| 2j | Estates and trusts (amount from Schedule K-1 (Form 1041), box 12, code A) | — | 0 |
| 2k | Disposition of property (difference between AMT and regular tax gain or loss) | — | **§3 declaration** |
| 2l | Depreciation on assets placed in service after 1986 (difference between regular tax and AMT) | — | 0 |
| 2m | Passive activities (difference between AMT and regular tax income or loss) | — | 0 |
| 2n | Loss limitations (difference between AMT and regular tax income or loss) | — | 0 |
| 2o | Circulation costs (difference between regular tax and AMT) | — | 0 |
| 2p | Long-term contracts (difference between AMT and regular tax income) | — | 0 |
| 2q | Mining costs (difference between regular tax and AMT) | — | 0 |
| 2r | Research and experimental costs (difference between regular tax and AMT) | — | 0 |
| 2s | Income from certain installment sales before January 1, 1987 | — | 0 |
| 2t | Intangible drilling costs preference | — | 0 |
| 3 | Other adjustments, including income-based related adjustments | — | **§3 declaration** |
| 4 | **Alternative minimum taxable income.** Combine lines 1 through 3. **(If married filing separately and line 4 is more than $875,950, see instructions.)** | — | ✓ |

**★ Line 4's parenthetical is the MFS kicker** — the instruction that an earlier draft dropped. i6251 p.9
supplies the rule the form points to: over $875,950 add 25% of the excess; at $1,142,550 or more add a
flat $66,650.

**★ Line 2k / line 3 are §3's two declarations.** 2k covers a capital-loss carryover that differs for the
AMT; line 3's instructions reach mortgage interest on a non-qualified dwelling (§56(b)(1)(C)). Both are
questions the form asks that btctax's input surface cannot answer — hence the ternary.

---

## Part II — Alternative Minimum Tax (AMT)

| # | Printed text | src | v1 |
|---|---|---|---|
| 5 | **Exemption.** IF your filing status is… AND line 4 is not over… THEN enter on line 5…<br>• Single or head of household — $609,350 — **$85,700**<br>• Married filing jointly or qualifying surviving spouse — $1,218,700 — **$133,300**<br>• Married filing separately — $609,350 — **$66,650**<br>*If line 4 is **over** the amount shown above for your filing status, see instructions.* | — | ✓ |
| 6 | Subtract line 5 from line 4. If more than zero, go to line 7. **If zero or less, enter -0- here and on lines 7, 9, and 11, and go to line 10.** | — | ✓ |
| 7 | • If you are filing **Form 2555**, see instructions for the amount to enter.<br>• If you reported capital gain distributions directly on Form 1040 or 1040-SR, line 7; you reported qualified dividends on Form 1040 or 1040-SR, line 3a; **or** you had a gain on **both** lines 15 and 16 of Schedule D (Form 1040) (as refigured for the AMT, if necessary), **complete Part III on the back and enter the amount from line 40 here.**<br>• **All others:** If line 6 is $232,600 or less ($116,300 or less if married filing separately), multiply line 6 by 26% (0.26). Otherwise, multiply line 6 by 28% (0.28) and subtract $4,652 ($2,326 if married filing separately) from the result. | mixed | ✓ (2555 = 0) |
| 8 | Alternative minimum tax foreign tax credit (see instructions) | — | ✓ |
| 9 | **Tentative minimum tax.** Subtract line 8 from line 7 | — | ✓ |
| 10 | Add Form 1040 or 1040-SR, line 16 **(minus any tax from Form 4972)**, and Schedule 2 (Form 1040), line 1z. Subtract from the result Schedule 3 (Form 1040), line 1 **and any negative amount reported on Form 8978, line 14 (treated as a positive number)**. **If zero or less, enter -0-.** If you used **Schedule J** to figure your tax on Form 1040 or 1040-SR, line 16, refigure that tax without using Schedule J before completing this line. See instructions | REG | ✓ (4972/8978/J = 0) |
| 11 | **AMT.** Subtract line 10 from line 9. **If zero or less, enter -0-.** Enter here and on **Schedule 2 (Form 1040), line 2** | — | ✓ |

**★ Line 7 has TWO branches.** Part III runs *only* on the middle bullet's condition. A filer with no
preferential income takes "All others" — the flat 26/28% on line 6. Do not assume Part III always runs.

**★ Line 9, not line 7, is "Tentative minimum tax."** The attach test (Who Must File condition 1)
compares **line 7 > line 10** — *before* the FTC subtraction — which is the mechanism creating the
window where AMT is $0 and the form is still required.

---

## Part III — Tax Computation Using Maximum Capital Gains Rates

*"Complete Part III only if you are required to do so by line 7 or by the Foreign Earned Income Tax
Worksheet in the instructions."*

| # | Printed text | src | v1 |
|---|---|---|---|
| 12 | Enter the amount from Form 6251, line 6. If you are filing Form 2555, enter the amount from line 3 of the worksheet in the instructions for line 7 | — | ✓ |
| 13 | Enter the amount from line 4 of the Qualified Dividends and Capital Gain Tax Worksheet in the Instructions for Form 1040 or the amount from line 13 of the Schedule D Tax Worksheet in the Instructions for Schedule D (Form 1040), whichever applies **(as refigured for the AMT, if necessary)**. See instructions. If you are filing Form 2555, see instructions for the amount to enter | **AMT** | ✓ |
| 14 | Enter the amount from Schedule D (Form 1040), line 19 **(as refigured for the AMT, if necessary)**. See instructions. If you are filing Form 2555, see instructions for the amount to enter | **AMT** | 0 (no §1250) |
| 15 | If you did not complete a Schedule D Tax Worksheet for the regular tax or the AMT, enter the amount from line 13. Otherwise, add lines 13 and 14, and enter the **smaller** of that result or the amount from line 10 of the Schedule D Tax Worksheet (as refigured for the AMT, if necessary). If you are filing Form 2555, see instructions for the amount to enter | AMT | ✓ |
| 16 | **Enter the smaller of line 12 or line 15** | — | ✓ **← the cap** |
| 17 | Subtract line 16 from line 12 | — | ✓ |
| 18 | If line 17 is $232,600 or less ($116,300 or less if married filing separately), multiply line 17 by 26% (0.26). Otherwise, multiply line 17 by 28% (0.28) and subtract $4,652 ($2,326 if married filing separately) from the result | — | ✓ |
| 19 | Enter: • **$94,050** if married filing jointly or qualifying surviving spouse, • **$47,025** if single or married filing separately, or • **$63,000** if head of household. | — | ✓ |
| 20 | Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet or the amount from line 14 of the Schedule D Tax Worksheet, whichever applies **(as figured for the regular tax)**. If you did not complete either worksheet for the regular tax, enter the amount from Form 1040 or 1040-SR, line 15; if zero or less, enter -0-. If you are filing Form 2555, see instructions for the amount to enter | **REG** | ✓ |
| 21 | Subtract line 20 from line 19. If zero or less, enter -0- | — | ✓ |
| 22 | **Enter the smaller of line 12 or line 13** | — | ✓ **← the cap** |
| 23 | Enter the smaller of line 21 or line 22. **This amount is taxed at 0%** | — | ✓ |
| 24 | Subtract line 23 from line 22 | — | ✓ |
| 25 | Enter: • **$518,900** if single, • **$291,850** if married filing separately, • **$583,750** if married filing jointly or qualifying surviving spouse, or • **$551,350** if head of household. | — | ✓ |
| 26 | Enter the amount from line 21 | — | ✓ |
| 27 | Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax Worksheet or the amount from line 21 of the Schedule D Tax Worksheet, whichever applies **(as figured for the regular tax)**. If you did not complete either worksheet for the regular tax, enter the amount from Form 1040 or 1040-SR, line 15; if zero or less, enter -0-. If you are filing Form 2555, see instructions for the amount to enter | **REG** | ✓ |
| 28 | Add line 26 and line 27 | — | ✓ |
| 29 | Subtract line 28 from line 25. If zero or less, enter -0- | — | ✓ |
| 30 | Enter the smaller of line 24 or line 29 | — | ✓ |
| 31 | Multiply line 30 by **15% (0.15)** | — | ✓ |
| 32 | Add lines 23 and 30.<br>**If lines 32 and 12 are the same, skip lines 33 through 37 and go to line 38. Otherwise, go to line 33.** | — | ✓ **← the skip** |
| 33 | Subtract line 32 from **line 22** | — | ✓ |
| 34 | Multiply line 33 by **20% (0.20)**.<br>**If line 14 is zero or blank, skip lines 35 through 37 and go to line 38. Otherwise, go to line 35.** | — | ✓ |
| 35 | Add lines 17, 32, and 33 | — | 0 (L14 = 0) |
| 36 | Subtract line 35 from line 12 | — | 0 |
| 37 | Multiply line 36 by **25% (0.25)** | — | 0 |
| 38 | Add lines 18, 31, 34, and 37 | — | ✓ |
| 39 | If line 12 is $232,600 or less ($116,300 or less if married filing separately), multiply line 12 by 26% (0.26). Otherwise, multiply line 12 by 28% (0.28) and subtract $4,652 ($2,326 if married filing separately) from the result | — | ✓ |
| 40 | **Enter the smaller of line 38 or line 39** here and on line 7. If you are filing Form 2555, do not enter this amount on line 7. Instead, enter it on line 4 of the worksheet in the instructions for line 7 | — | ✓ **← the final min** |

---

## The Part III question, settled by the form

The plan's §3.3 spent two review rounds on "where are the capital-gain bands positioned?" and offered
$75,812.50 vs $55,897.50. **Both were wrong**, because the form does two independent things and the
draft conflated them:

1. **Position — REGULAR side.** Lines **20** and **27** both say *"(as figured for the regular tax)"*.
   Line **13** says *"(as refigured for the AMT, if necessary)"*. So the gain's **amount** is AMT-side;
   its band **positions** are regular-side.
2. **Amount — CAPPED at the taxable excess.** Lines **16** and **22** are both "enter the **smaller** of
   line 12 or …", line **17** floors the 26/28% slice at 0, and line **32** skips the 20% tranche when
   it equals line 12. Line **40** takes the smaller of 38 or 39.

$75,812.50 ignored the caps; $55,897.50 mispositioned the bands. The correct figure for the disputed
vector is **$70,005.00** — and the excess-under-gain case needs no special code, because lines 16/17/22
and the line-32 skip handle it structurally. It does need a KAT: **V2b**.

## ★ Transcription errata — read the TEXT LAYER, not the rendered page

**Line 33 was first transcribed as "Subtract line 32 from line **12**". The form says **line 22**.**
Found not by review but by running the transcription against the seven verified vectors: 1/7 passed.
The single wrong digit taxed the ordinary slice twice — 26/28% at line 18 and again 20% at line 34 —
inflating TMT by $200,000 on V3 and manufacturing AMT on four returns that owe none. With line 33
corrected: **7/7**.

Root cause: I read the rendered page image, where `12` and `22` differ by a few pixels.
**`pdftotext -layout` is the authority, not the picture.** After the fix, every cross-reference in
lines 12–40 was re-verified against the extracted text layer; line 33 was the only error.

## Every constant is on the form

$85,700 / $133,300 / $66,650 · $609,350 / $1,218,700 · $875,950 / $1,142,550 (i6251 p.9) · $232,600 /
$116,300 · $4,652 / $2,326 · $94,050 / $47,025 / $63,000 · $518,900 / $291,850 / $583,750 / $551,350 ·
26% / 28% / 25% / 20% / 15%. **All belong in `AmtParams`** per `CLAUDE.md`'s no-literal rule — and the
only production literal is `btctax-adapters/src/tax_tables.rs:141`.

## Still owed from i6251 (not on the form)

Who Must File's four conditions verbatim · line 4's MFS-kicker text · line 8's AMT-FTC rule for the
§904(j) elector · the Part I line-3 dwelling instruction · Part III's "None of the statements apply ⇒ use
the regular tax amounts for lines 13, 14, and 15". **Fetch `i6251--2024.pdf` next; if it cannot be
fetched, STOP — do not transcribe from memory.**

---

# Addendum — from `i6251--2024.pdf` (Nov 13, 2024, Cat. 64277P)

Fetched and transcribed 2026-07-28; archived at `design/amt-form6251/i6251--2024.pdf`. This closes the
five items PART_III.md listed as "still owed". **Nothing below was written from memory.**

## Who Must File (p.1) — verbatim

> Attach Form 6251 to your return if any of the following statements are true.
> 1. Form 6251, line 7, is greater than line 10.
> 2. You claim any general business credit, and either line 6 (in Part I) of Form 3800 or line 25 of Form 3800 is more than zero.
> 3. You claim the qualified electric vehicle credit (Form 8834), the personal-use part of the alternative fuel vehicle refueling property credit (Form 8911), or the credit for prior year minimum tax (Form 8801).
> 4. The total of Form 6251, lines 2c through 3, is negative and line 7 would be greater than line 10 if you didn't take into account lines 2c through 3.

**Condition 1 is the Tier-1/Tier-2 boundary**, as the plan says. Conditions 2 and 3 are credits v1 has no
input for. **Condition 4 is reachable in principle** — it keys on lines 2c**–3**, which *includes* line
2k and line 3, i.e. §3's two declarations. Under §3's ternary both refuse unless answered AMT-neutral
(⇒ 0), so the total can never be negative and condition 4 stays unreachable. **That is now a stated
consequence of the ternary, not a coincidence — record it in the §5 discharge KAT.**

## Line 2a (p.2) — verbatim

> Enter the amount of all taxes from Schedule A (Form 1040), line 7, **except any generation-skipping
> transfer taxes on income distributions**. If you aren't filing Schedule A (Form 1040), then enter the
> standard deduction amount that you reported on Form 1040 or 1040-SR, line 12.

Confirms the fix at the root of this whole sequence: **Schedule A line 7**, not line 17. The GST carve-out
is new to us — v1 has no GST input, so it is structurally 0, but `amt_worksheet_line2` should say so.

## Line 2b (p.2) — verbatim

> Include any refund from Schedule 1 (Form 1040), line 1, that is attributable to state or local income
> taxes. Also include any refunds received in 2024 and included in income on Schedule 1 (Form 1040), line
> 8z, that are attributable to state or local personal property taxes or general sales taxes; foreign
> income taxes; or state, local, or foreign real property taxes. **Enter the total as a negative amount.**

btctax's `state_refund_taxable` *is* Schedule 1 line 1 ("taxable refunds … of state and local income
taxes"), so it maps exactly. The 8z limb has no v1 input ⇒ 0.

## Line 3 — Mortgage Interest (p.8) — verbatim. **This is §3's declaration, in the IRS's own words.**

> If you deducted home mortgage interest on Schedule A for a dwelling that isn't a principal residence
> (within the meaning of section 121) or qualified dwelling for AMT, include that deducted interest on
> line 3. A qualified dwelling for AMT is a house, apartment, condominium, or mobile home not used on a
> transient basis. A qualified dwelling for AMT doesn't include house boats and recreational vehicles.

Phrase the question so the AMT-neutral answer is `true`: *"Is the dwelling a principal residence or a
house/apartment/condominium/mobile home not used on a transient basis?"* — with houseboats and RVs
called out, since the instruction names them.

## Line 3 — Charitable Contributions of Certain Property (p.8) — ★ CORRECTS THE PLAN

> If you make a charitable contribution of property to which section 170(e) applies and you had a
> different basis for AMT purposes, you may have to make an adjustment. See section 170(e) for details.

The plan (and r1) asserted charitable **can never** diverge, citing the post-1992 repeal of §57(a)(6).
That is right about the *appreciation* preference but **overstated**: a §170(e) gift of property with a
different **AMT basis** is a line-3 adjustment. It stays 0 for btctax only because donated crypto has no
AMT/regular basis divergence (no depreciation, no ISO, no §56(a)(6) event ever touches it) — a *derived*
conclusion, not the blanket one. Restate it that way, and note btctax **does** emit §170(e) crypto
donations (Form 8283), so this is live surface, not hypothetical.

## Line 4 (p.9) — the MFS kicker, verbatim, **with the IRS's own worked example**

> If your filing status is married filing separately and line 4 is more than $875,950, you must include
> an additional amount on line 4. If line 4 is $1,142,550 or more, include an additional $66,650.
> Otherwise, include 25% of the excess of the amount on line 4 over $875,950. **For example, if the
> amount on line 4 is $895,950, enter $900,950 instead—the additional $5,000 is 25% of $20,000 ($895,950
> minus $875,950).**

★ **V8's KAT is written for us:** MFS, line 4 = $895,950 ⇒ line 4 becomes **$900,950**. Plus the two
boundaries: $875,950 (nothing added) and $1,142,550 (flat +$66,650).

## Exemption Worksheet — Line 5 (p.9) — the zero-exemption note

> **Note.** If Form 6251, line 4, is equal to or more than **$952,150** if single or head of household,
> **$1,751,900** if married filing jointly or qualifying surviving spouse, or **$875,950** if married
> filing separately, your exemption is zero. Don't complete this worksheet; instead, enter the amount
> from Form 6251, line 4, on line 6 and go to line 7.

Confirms the plan's $1,751,900 MFJ full-phaseout. **Note the collision: MFS's zero-exemption threshold
and the MFS kicker start are the same $875,950** — so an MFS filer crossing it gets the kicker *and*
loses the exemption at the same point. Worth its own KAT.

The worksheet itself is the arithmetic already in `amt.rs`: exemption − 25% × (AMTI − phase-out start),
floored at 0.

## Line 8 — AMTFTC (p.10) — verbatim, and it settles the ordering

> **Do I need to fill out line 8?** Before figuring your AMTFTC, figure your foreign tax credit for the
> regular tax and complete Schedule 3 (Form 1040), line 1. Next, fill in Form 6251, line 10, as
> instructed. If the amount on line 10 is greater than or equal to the amount on line 7, do the
> following: • Leave line 8 blank and enter -0- on line 11. • See Who Must File, earlier, to find out if
> you must attach Form 6251 to your return. …
> If the amount on line 10 is less than the amount on line 7, figure your AMTFTC and enter it on line 8.

> **Figuring the AMTFTC.** If you made an election to claim the foreign tax credit on your 2024 tax
> return without filing Form 1116, your AMTFTC is the same as the foreign tax credit on Schedule 3
> (Form 1040), line 1.

★ **Line 10 is computed BEFORE line 8**, and line 8 is left blank when line 10 ≥ line 7. So the
implementation order is 7 → 10 → (8, 9, 11), not the printed order. The second paragraph confirms the
§904(j)-elector equality the plan relies on — and btctax's ≤$300/$600 screen *is* the no-Form-1116
election, so `line8 = Schedule 3 line 1` exactly.

## TIP (p.2) — a planning fact worth surfacing

> If you owe AMT, you may be able to lower your total tax (regular tax plus AMT) by claiming itemized
> deductions on Form 1040 or 1040-SR, even if your total itemized deductions are less than the standard
> deduction. This is because the standard deduction isn't allowed for the AMT and, if you claim the
> standard deduction on Form 1040 or 1040-SR, you can't claim itemized deductions for the AMT.

btctax already has `ItemizeElection::ForceItemize` (§63(e)). When AMT is owed, force-itemizing can lower
total tax — a real advisory the engine is positioned to give and currently does not.
