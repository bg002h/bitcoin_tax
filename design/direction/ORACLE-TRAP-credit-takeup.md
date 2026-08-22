# ORACLE TRAP — Tax-Calculator zeroes the refundable credits by design, and it is POSITION-DEPENDENT

_Found and machine-verified 2026-08-21 while de-risking N2 (EITC/ACTC), **before** any of it was
built. Nothing shipped is contaminated — see "Are we already affected?" — but this arms itself on the
first credit-engaged vector, which N2 necessarily adds._

## The one-line finding

**Tax-Calculator's default settings silently report `EITC = $0` for a household plainly entitled to
it — and would therefore have "corroborated" btctax's current wrong answer.** The zeroing is not tax
law, is not on the form, and depends on the household's **row position in the input array**.

## What was measured (not read)

MFJ, $40,000 wages, two qualifying children, TY2024, taxcalc 6.7.2:

| run | EITC | ACTC | iitax |
|---|---:|---:|---:|
| taxcalc **defaults** | **0.00** | 2,920.00 | −2,920.00 |
| `*_claim_prob_scale = 9e99` | **4,778.18** | 2,920.00 | −7,698.18 |

**The default hides $4,778.18 of EITC** — and it hides it *as a stable, reproducible zero*, which is
what makes it dangerous. It never looks flaky.

Then the same household five times in **one vectorized pass**, defaults:

| row | EITC | ACTC | iitax |
|---:|---:|---:|---:|
| 0 | **0.00** | 2,920.00 | −2,920.00 |
| 1 | 4,778.18 | 2,920.00 | −7,698.18 |
| 2 | 4,778.18 | 2,920.00 | −7,698.18 |
| 3 | 4,778.18 | 2,920.00 | −7,698.18 |
| 4 | 4,778.18 | 2,920.00 | −7,698.18 |

**Identical inputs, identical year, one pass — a $4,778.18 spread driven by nothing but array index.**
`gen_goldens.py:269-273` runs exactly such a vectorized pass over all candidates, so under N2 a
household's oracle answer would depend on corpus ordering.

## The mechanism

`calcfunctions.py:3216-3220` (EITC) and `:4370-4375` (ACTC), both commented by upstream as
**"Not on the form: credit claiming logic"**:

```python
unscaled_prob = max(eitc_claim_prob_min, c59660 / max_amount)
prob = eitc_claim_prob_scale * unscaled_prob
if credit_claim_urn >= prob:
    c59660 = 0.
```

Defaults in 6.7.2 — **the take-up simulation is ON out of the box**:

| parameter | default |
|---|---|
| `eitc_claim_prob_min` | 0.4 |
| `eitc_claim_prob_scale` | **1.03** |
| `actc_claim_prob_min` | 0.0 |
| `actc_claim_prob_scale` | **1.1** |

`credit_claim_urn` is drawn once per record from a **fixed seed** (`records.py:194-195`,
`default_rng(seed=192837465)`), so position 0 always draws `0.759807`:

- EITC: `prob = 1.03 × max(0.4, 4778.18/6960) = 0.70712` → `0.759807 ≥ 0.70712` → **zeroed**
- ACTC: `prob = 1.10 × max(0.0, 2920/3400) = 0.94471` → `0.759807 < 0.94471` → **survives**

★ **The ACTC survived by arithmetic luck, not by design.** Same branch, same urn, different ratio.
Testing only this vector would license the conclusion "ACTC is safe" — the exact
excuse-list-keyed-by-vector-name failure this repo already has a rule against. **Both scales must be
disabled; neither is safe by observation.**

## Why this is a THIRD oracle blindness, not an instance of §G-9

| shape | example | detectable by two-oracle agreement? |
|---|---|---|
| the oracle takes the value as **input** (§G-9) | Schedule A line 8a | no — it echoes us |
| both oracles are **disqualified together** | MFS §55(d)(3) | no — they align while both wrong |
| ★ **the oracle models the value away on purpose** | this | **no — and it looks stable** |

The first two are known here. This one is new and nastier: the oracle *does* compute the figure, then
**discards it for microsimulation realism** (population take-up rates), which is correct for
estimating aggregate revenue and wrong for validating one return. A per-return harness that does not
turn it off is not asking the oracle the question it thinks it is asking.

## Are we already affected? — NO, verified

`corpus.py:36` pins **D-1 no dependents** ("CTC/ODC/EIC omitted"), and `corpus.py:69-70` floors the
"low" wage band **above** the childless-EIC range (~$18.6k Single / ~$25.5k MFJ) *specifically* so
EIC never engages. `gen_goldens.py:56-58` states the same. So no committed golden, and no shipped
figure, is contaminated today.

**The trap arms when N2 inverts D-1** — which N2 must do, since a credits corpus without children
tests nothing.

## REQUIRED PROTOCOL for N2 (binding on the Phase-5 implementer)

1. **Disable BOTH take-up simulations** on every per-return oracle call:
   ```python
   pol.adjust({"eitc_claim_prob_scale": [{"year": YEAR, "value": 9e99}],
               "actc_claim_prob_scale": [{"year": YEAR, "value": 9e99}]})
   ```
   (`.adjust` with the paramtools list form — `implement_reform({YEAR: {...}})` raises
   `ValidationError: Parameter 2024 is not a string` on 6.7.2.)
2. **B1 kill-test — mandatory.** A test that reds when the disable is removed. It must plant the real
   defect (run the credit-engaged vector at **row position 0**, where the seeded urn zeroes it) and
   assert a nonzero EITC. A kill-test at any other row position **passes with the disable removed**
   and is therefore worthless — this is the "seen red once" rule and the position-dependence is
   exactly what makes it easy to get wrong.
3. **Do not key any excuse or expectation to a vector name.** State the mechanism and let it decide,
   per the standing rule.
4. **OTS must be asked independently** for the same households. A single oracle here is not a witness,
   and this finding is precisely why: the second oracle's *default* answer was wrong.
5. **Do not rely on ordering.** Even with both scales disabled, do not let corpus row order become
   load-bearing; assert the disable rather than a position.

## What would make this finding wrong

If `Records` were constructed somewhere in our harness with a `credit_claim_urn` column supplied from
data (overriding the seeded draw), the position-dependence would not apply — I verified our harness
does not set it (no `credit_claim_urn` in `scripts/oracle/`), but an N2 implementer who adds it should
re-check this. And if a future taxcalc changes the defaults to 9e99, the disable becomes a no-op
rather than a correction — assert it, do not assume it.

## Reproduce

Both tables above regenerate from `.venv/bin/python` with taxcalc 6.7.2 in ~5 seconds; the script is
inline in this document's history (see the commit that added it).
