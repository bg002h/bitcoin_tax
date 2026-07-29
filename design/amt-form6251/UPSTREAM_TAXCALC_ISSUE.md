# Draft issue for `PSLmodels/Tax-Calculator` — for review before filing

**★ FOLLOW-UP COMMENT POSTED 2026-07-29** —
[#3108 comment 5119300474](https://github.com/PSLmodels/Tax-Calculator/issues/3108#issuecomment-5119300474).
Adds OpenTaxSolver as a genuinely independent second witness on line 2a (OTS: AMTI 2,250,000 / AMT
26,271, matching the form), and **corrects an error in our own filed issue**: we wrote that §55(d)(3)
"also appears not to be modelled." Too broad. taxcalc DOES model it — `AMT_em_pe = 875,950` for 2024
and the `MARS == 3` guard at `calcfunctions.py:2590`, docstring citing the subsection at `:2429`. What
is genuinely missing is the *addition to line 4*; and the implemented cliff is inert, because the
ordinary phaseout already reaches zero at 609,350 + 66,650/0.25 = 875,950, the same number.
**We should have grepped for `AMT_em_pe` before filing.**

Also disclosed against ourselves in that comment: OTS is NOT a witness on the MFS point — its 2024
solver carries the 2023 thresholds and returns a third number (912,150) — so the MFS half rests on the
instructions alone.

**Status:** ★ **FILED 2026-07-29 (05:12Z) as [PSLmodels/Tax-Calculator#3108](https://github.com/PSLmodels/Tax-Calculator/issues/3108)** — open at the time of the v0.14.0 tag. This file is retained as the drafting record and the evidence table; the issue text below is what was posted. Repo confirmed from the installed package's own metadata
(`taxcalc` 6.7.2, Home-page `https://github.com/PSLmodels/Tax-Calculator`, CC0-1.0 — so quoting their
source carries no licence friction).

**Framing note:** posed as a **question**, not a defect report. Their itemizer branch is correct, so the
asymmetry between the two branches is the whole argument — and it may well be a deliberate
microsimulation simplification that deserves a docstring rather than a code change. Precedent for the
register: our tenforty issue #278 / PR #279, where OTS turned out never to have been wrong; the wrapper
was.

---

## Title

`AMTI omits Form 6251 line 2a's standard-deduction add-back when standard > 0 — intentional?`

## Body

Hello — I'm using `taxcalc` as one of two reference engines while implementing Form 6251 line by line,
and I've hit a difference I can't resolve from the code alone. It may be a deliberate simplification
for microsimulation, in which case a note in the docstring would help downstream users; I'd rather ask
than assume.

### What I see

In `taxcalc/calcfunctions.py`, the AMTI block branches on the standard deduction:

```python
# Form 6251 line 1 = Form 1040 line 15 = AGI - (STD or itemized) - QBID.
if standard == 0.0:
    c62100 = (
        c00100 - e00700 - qbided - c04470 +
        c18300 +    # SALT add-back (Form 6251 line 2a)
        ...
    )
if standard > 0.0:
    c62100 = c00100 - e00700 - qbided - standard
```

The **itemizer** branch adds Schedule A's taxes back (`+ c18300`), matching Form 6251 line 2a. The
**standard-deduction** branch subtracts `standard` and does not add anything back — so `c62100` stops at
Form 6251 **line 1** and line 2a is never applied on that path.

Form 6251 (2024) line 2a reads, in full:

> If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; **otherwise, enter the
> amount from Form 1040 or 1040-SR, line 12**

and the instructions (i6251 2024, *Line 2a—Taxes*, p. 2):

> If you aren't filing Schedule A (Form 1040), then enter the **standard deduction amount** that you
> reported on Form 1040 or 1040-SR, line 12.

Form 1040 line 12 is "Standard deduction or itemized deductions", so for a non-itemizer line 2a is the
standard deduction, added back. That is also what §56(b)(1)(D) requires ("The standard deduction under
section 63(c) … shall not be allowed"), and what i6251's own TIP turns on:

> If you owe AMT, you may be able to lower your total tax … by claiming itemized deductions … even if
> your total itemized deductions are less than the standard deduction. This is because **the standard
> deduction isn't allowed for the AMT** …

That advice only makes sense if the standard deduction is disallowed for AMT purposes.

There's also a second, independent IRS construction of the same quantity — the *"Worksheet To See if
You Should Fill in Form 6251"* in the 1040 instructions, which for a non-itemizer says to **skip lines 1
and 2** and take `line 11 − line 13`, i.e. **AGI − QBI deduction**, never subtracting the standard
deduction at all. That agrees with adding it back on Form 6251 itself.

### Minimal repro

MFJ, 2024, both under 65: **$250,000** of wages, **$2,000,000** of net long-term capital gain, standard
deduction, no AMT preference items, no credits.

```python
row = {"RECID": 1, "FLPDYR": 2024, "MARS": 2,
       "e00200": 250_000.0, "e00200p": 250_000.0, "e00200s": 0.0,
       "p23250": 2_000_000.0, "s006": 1.0}
```

| | taxcalc | Form 6251 worked by hand |
|---|---:|---:|
| AMTI (`c62100` / line 4) | 2,220,800 | **2,250,000** |
| AMT (`c09600` / line 11) | 18,331 | **26,271** |

The AMTI gap is exactly the $29,200 MFJ standard deduction. Across a set of eleven test vectors the
pattern is consistent: every **standard-deduction** vector differs by exactly the standard deduction,
and every **itemizing** vector agrees with `taxcalc` to the cent — which is what pointed at the branch.

(One MFS vector differs by $19,600 = $14,600 standard deduction + the $5,000 line-4 MFS addition from
i6251 p.9 — "if line 4 is more than $875,950 … include 25% of the excess" — which also appears not to be
modelled. Happy to open that separately if it's a real gap rather than something I've missed.)

### The question

Is the omission on the `standard > 0.0` path intentional — e.g. because the effect is immaterial in
weighted aggregates — or is it a bug? If intentional, would a comment on that branch be welcome? Users
reading `c09600` per-filer (rather than in aggregate) will otherwise see AMT understated for
standard-deduction filers, which is the direction that matters for anyone using it as a reference
implementation.

Happy to open a PR if a fix is wanted; I didn't want to send one before checking the intent.

---

## Reviewer checklist before filing

- [ ] Confirm the quoted `calcfunctions.py` block still matches the version at HEAD (quoted from 6.7.2).
- [ ] Re-run the repro against the latest release, not just 6.7.2.
- [ ] Search issues once more for prior art (searched 2026-07-28: "AMT standard deduction" → nothing;
      `c62100` → 6 unrelated closed issues).
- [ ] Decide whether to include the MFS line-4 observation here or hold it for a separate issue.
- [ ] Statute cite verified 2026-07-28: **§56(b)(1)(D)**, not the widely-cited (E). P.L. 116-94 struck
      the former medical subparagraph (B) and redesignated (C)–(F) to (B)–(E); today's (E) is "Section
      68 not applicable". Confirmed against uscode.house.gov and Cornell LII.

## Evidence backing this (for our records, not for the issue)

| witness | independent of the others? | says |
|---|---|---|
| Form 6251 line 2a, printed text | authority | enter 1040 line 12 = the standard deduction |
| i6251 p.2, *Line 2a—Taxes* | authority | "enter the standard deduction amount" |
| i6251 TIP | authority | "the standard deduction isn't allowed for the AMT" |
| i1040's Form-6251 worksheet | ✓ different document, different route | non-itemizer base = AGI − QBI |
| §56(b)(1)(D) | ✓ statute | standard deduction "shall not be allowed" |
| Blind derivation A (Opus) | ✓ never saw our code or answer | $26,271.00 |
| Blind derivation B (Fable) | ✓ never saw our code or answer | $26,271.00 |
| btctax `form6251.rs` | — | $26,271.00 |
| **Tax-Calculator 6.7.2** | — | **$18,331.00** |

**Caveat worth remembering:** both blind derivations cited the statute as §56(b)(1)**(E)** — the same
error, because both took it from memory rather than fetching the U.S. Code. Two independent agents
agreeing does **not** corroborate a fact neither of them looked up. Only the document-derived figures
above are genuinely cross-checked.
