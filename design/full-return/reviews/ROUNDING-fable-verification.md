# Independent verification of `ROUNDING_AUTHORITY.md` (Fable)

*Persisted VERBATIM before folding. Author = Opus; reviewer = Fable. This citation has now been wrong
**three times** — the whole reason this file exists.*

**Verdict: 0 Critical / 3 Important / 2 Minor — ALL against the evidence doc, NONE against the code.
P7 stays GREEN. The conclusion (btctax is right; close the follow-up) stands. The case is SUFFICIENT,
not DECISIVE — and the commit message claimed decisive.**

---

The fact-checker's independent read (raw HTML, full-text grep — my WebFetch was truncating 2.2 MB and 6.0 MB pages) **confirms my correction on every point.** My retraction stands, and so do the revised findings.

Four refinements worth folding, one of which is a genuine win you didn't claim:

## 1. The IRM actually *does* support you — for lines 1–23, not line 24

This is better than "strike §4." The IRM establishes that Form 1040 **lines 1–23 are transcribed into dollar-only fields**:

- **EC 369:** *"Verify that dollars and cents were not entered into dollar-only fields."*
- **IRM 3.11.3.14.2:** *"All lines on Form 1040 are edited in dollars only except lines 24 through 38."*

So when the IRS's computer recomputes the subtotals — **lines 9, 11, 15** — it is necessarily summing whole-dollar values. That is real (if inferential) support for §3.1's cross-footing across most of the return. It is exactly the opposite of what §4 currently claims, and it is defensible.

What the IRM does **not** support is the one line you built the section on. **IRM 3.11.3.14.2.28: line 24 is edited "as dollars and cents."** So: rewrite §4 as *"the IRM confirms the whole-dollar regime for lines 1–23 and is silent-to-adverse on line 24"* — and delete the inversion claim.

## 2. The IRM forecloses the exposure in both directions

**IRM 3.12.3.31.15.5:** *"If the taxpayer has computed total tax or total payments in 'dollars and cents,' but has rounded or truncated the balance due or refund, **follow the taxpayer's intent** and adjust total tax or total payments accordingly."*

The IRS's stated posture on rounding-sized deltas is to follow the filer's intent, not to assess. Combined with the **redacted** TPNC 282 tolerance (`≡ ≡ ≡`), this means: you cannot show the inversion, *and* you never needed to show it. The $1 anxiety from r2 is not a live exposure in either direction. Say that plainly instead of claiming a win.

## 3. Two citation hygiene items

- **Sourcing caveat, and it belongs in the doc.** The IRS **gates** MeF schema distribution (Pub 4164 §2.9 — Registered User Portal / SOR). Your XSD text is from public GitHub mirrors. It is corroborated across three independent mirrors spanning 2011→2025, byte-identical on these types — which is strong — but the doc must say where the text came from rather than imply a canonical IRS URL.
- **A trap to avoid:** `schemas.liquid-technologies.com/eFile/3.1/...` is the 2004-era eFile 3.1 library. It has `AmountType` (integer, 11 digits) and **no `USAmountType` at all**. If anyone "verifies" your §2 against that mirror they will conclude you invented the type. Cite a TY-correct mirror.

## 4. EC 334, not 282

The Error Code is **334**; 282 is the *Taxpayer Notice Code*. (Confusingly, "Error Code 282" exists and is the Credit for the Elderly or the Disabled — unrelated.) If §4 survives in any form, get that right.

---

## Where this leaves the argument

Your conclusion is right and the follow-up should close. But the case now rests where the evidence actually is:

1. **MeF (airtight).** Line 24 is `TotalTaxAmt : USAmountType : xsd:integer`, and so are lines 22 and 23. An e-filed return transmitting 8,355 and 8,478 and then reporting 16,832 would be **inconsistent with its own transmitted components**. Reading B is unrepresentable on the IRS's wire format. This is the argument; lead with it.
2. **Direct File (strong).** The IRS's own engine deliberately sums rounded values and says so in a comment, and its expected output is $7 off a true sum to keep the printed form adding up.
3. **The reg (supportive, not dispositive).** Structure and the (b) binary election favour you; the text does not compel you. Stop saying the ambiguity dissolves.
4. **The IRM (partial).** Confirms dollar-only transcription for lines 1–23. Says nothing helpful about line 24, and expressly treats it as a cents line.

That is a sufficient case, honestly stated. It is not the decisive one the commit message claims — and the difference between "sufficient" and "decisive" is the whole reason this citation has now been wrong three times. Fix the doc, then close.
