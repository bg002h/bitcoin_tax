#!/usr/bin/env python3
"""Cross-check btctax's Form 6251 against Tax-Calculator on the T1 vector fixture.

    .venv/bin/python scripts/oracle/verify_f6251.py

Not part of `make check` (that is the Rust suite); this is the independent-engine probe the AMT plan
called for. It reads `crates/btctax-core/src/tax/fixtures/form6251_vectors.json` — emitted by the
line-by-line transcription in
`f6251_reference.py` — and compares line 11 against Tax-Calculator's `c09600`.

⚠ THIS IS A ONE-ORACLE PROBE. This repo's standard is TWO oracles, and the second cannot arbitrate
AMT: OpenTaxSolver is not installed here, and even when it is, `ots_direct.py` extracts only L16 and
L24+NIIT — it never reads the 1040's **line 17**, where AMT lands (its one `L17` is Form 8960's). So
nothing below is confirmed to this repo's usual standard. Treat the divergence as PROVISIONAL and do
not file it upstream until a second engine has been asked.

★ TWO KNOWN DIVERGENCES, adjudicated against the FORM (not against a second oracle). V4 and V5 are the only STANDARD-DEDUCTION vectors that owe AMT,
and on both, taxcalc's AMTI is lower than ours by EXACTLY the $29,200 standard deduction. That is a
Tax-Calculator defect, not ours (`taxcalc/calcfunctions.py`, the AMTI block):

    if standard > 0.0:
        c62100 = c00100 - e00700 - qbided - standard

It subtracts the standard deduction and never adds it back. Its itemizer branch DOES add back
Schedule A line 7 (`+ c18300`), which is why every itemizing vector agrees. But Form 6251 line 2a says:
"If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; **otherwise, enter the
amount from Form 1040 or 1040-SR, line 12**" — i.e. add the standard deduction back. i6251 p.2 repeats
it, §56(b)(1)(D) mandates it, and i6251's own TIP turns on it ("the standard deduction isn't allowed
for the AMT"). The form is the authority; a taxcalc disagreement is adjudicated against the PDF and
never encoded. Direction: taxcalc UNDERSTATES AMT for standard-deduction filers.
"""
import json, sys, warnings, pathlib

warnings.filterwarnings("ignore")
import pandas as pd, taxcalc as tc  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
KNOWN_DIVERGENT = {"V4", "V5"}  # standard-deduction + AMT owed; see the module docstring


def main() -> int:
    vectors = json.loads(
        (ROOT / "crates/btctax-core/src/tax/fixtures/form6251_vectors.json").read_text()
    )["vectors"]
    rows = []
    for n, v in enumerate(vectors):
        i = v["inputs"]
        rows.append({
            "RECID": n + 1, "FLPDYR": 2024,
            "MARS": 3 if i["filing_status"] == "mfs" else 2,
            "e00200": float(i["wages"]), "e00200p": float(i["wages"]), "e00200s": 0.0,
            "p23250": float(i["net_ltcg"]),
            "e19800": float(i["cash_gift"]),   # cash charitable — absent from gen_goldens' builder
            "e00300": float(i["state_refund"]),
            "e07300": float(i["sch3_line1_ftc"]),
            "s006": 1.0,
        })
    recs = tc.Records(data=pd.DataFrame(rows), start_year=2024, gfactors=None,
                      weights=None, adjust_ratios=None)
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(2024)
    calc.calc_all()
    amt, amti = calc.array("c09600"), calc.array("c62100")

    print(f"{'vec':5}{'btctax AMT':>13}{'taxcalc':>12}{'ΔAMTI':>12}  verdict")
    bad = 0
    for n, v in enumerate(vectors):
        mine, theirs = float(v["form6251"]["line11"]), float(amt[n])
        d_amti = float(v["form6251"]["line4"]) - float(amti[n])
        if abs(mine - theirs) < 1.0:
            verdict = "AGREE"
        elif v["id"] in KNOWN_DIVERGENT:
            verdict = f"KNOWN (taxcalc omits the std-ded add-back: ΔAMTI={d_amti:,.0f})"
        else:
            verdict, bad = "★ UNEXPECTED DIVERGENCE", bad + 1
        print(f"{v['id']:5}{mine:>13,.2f}{theirs:>12,.2f}{d_amti:>12,.0f}  {verdict}")
    print(f"\n{'FAIL' if bad else 'OK'}: {bad} unexpected divergence(s); "
          f"{len(KNOWN_DIVERGENT)} known and adjudicated against the form.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
