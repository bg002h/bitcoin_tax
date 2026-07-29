#!/usr/bin/env python3
"""Cross-check btctax's Form 6251 against BOTH reference engines, line by line.

    OTS_DIR=~/OpenTaxSolver2024_22.07_linux64 .venv/bin/python scripts/oracle/verify_f6251.py

Not part of `make check` (that is the Rust suite); this is the independent-engine probe the AMT plan
called for. It reads `crates/btctax-core/src/tax/fixtures/form6251_vectors.json` and compares against:

  1. **Tax-Calculator** — `c09600` (AMT) and `c62100` (AMTI). Always available (it is in `.venv`).
  2. **OpenTaxSolver** — EVERY printed Form 6251 line (`AMT_Form_6251_L*`), when `OTS_DIR` is set.
     Skipped with a loud note otherwise, never silently.

★ THIS IS THE ONLY PLACE AMT IS VALIDATED, and it exists because the golden matrix CANNOT do it.
`gen_goldens.py`'s D-2 predicate rejects any household taxcalc sees AMT on, and btctax refuses returns
that must attach Form 6251 — so an AMT-owing household is rejected twice over and the corpus is
structurally AMT-free (measured: 0 of 104 households have nonzero AMT on either oracle). The way
around it is that `assemble_absolute` is infallible: btctax computes the whole form even for returns it
will not file, so these vectors exercise Part I through Part III on returns the corpus can never hold.

★ An earlier version of this docstring said OTS "is not installed here, and even when it is,
`ots_direct.py` never reads line 17". The second half was true and is now fixed; the first half was an
assumption. OTS computes Form 6251 in full (`taxsolve_US_1040_2024.c:222`) and always did.

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
import json, os, shutil, sys, tempfile, warnings, pathlib

warnings.filterwarnings("ignore")
import pandas as pd, taxcalc as tc  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
KNOWN_DIVERGENT = {"V4", "V5"}  # standard-deduction + AMT owed; see the module docstring

# OTS's TY2024 solver cannot witness these — its own defects reach them. Both are FIXED in OTS 2025,
# so neither is upstream-reportable; the 2024 line is closed, so we gate instead of waiting.
#   V8  — MFS with line 4 above the stale 2023 §55(d)(3) threshold (taxsolve_US_1040_2024.c:270-275).
#   V2b — a cash gift above the §170(b) 60%-of-AGI ceiling, which OTS's 2024 Schedule A never applies.
OTS_CANNOT_WITNESS = {
    "V8": "OTS 2024 carries the 2023 \u00a755(d)(3) MFS constants (831,150/1,084,150/63,250)",
    "V2b": "OTS 2024 applies no \u00a7170(b) 60%-of-AGI cash ceiling",
}
# Per-(vector, line) divergences that are METHODOLOGY, not error — the same Tax-Table-vs-schedule class
# the Rust harness already models for 1040 L16 (`verdict_l16`'s provenance predicate).
#   V10 — MFJ, TI 520,800, of which only 20,800 is ORDINARY. The QDCGT worksheet figures the tax on
#         that ordinary slice, and 20,800 is under the $100,000 Tax-Table ceiling, so OTS charges the
#         table's $50-bin rate while btctax applies the exact §1(j) schedule. Form 6251 line 10 is
#         `L16 - Sch 3 L1`, so the L16 difference lands here unchanged. Neither is "wrong": the IRS
#         publishes both, and the Tax Table IS the authority below the ceiling.
OTS_METHODOLOGY_DIFF = {
    ("V10", "line10"): "Tax-Table vs exact schedule on the $20,800 ordinary slice (under the $100k ceiling)",
}

# The lines worth comparing: Part I in full, plus the Part II figures that decide Who Must File.
OTS_COMPARED_LINES = ["line1", "line2a", "line2b", "line4", "line5", "line6", "line7", "line10", "line11"]


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
    print(f"\n  {'FAIL' if bad else 'OK'}: {bad} unexpected divergence(s); "
          f"{len(KNOWN_DIVERGENT)} known and adjudicated against the form.")

    bad += _ots_pass(vectors)
    return 1 if bad else 0


def _ots_pass(vectors) -> int:
    """Compare EVERY witnessed Form 6251 line against OpenTaxSolver. Returns the unexpected-diff count.

    Absent `OTS_DIR` this prints a loud SKIP and returns 0 — a missing oracle is a gap in coverage, not
    a pass, and saying so is the whole point of the two-oracle standard.
    """
    print("\n── oracle 1 · OpenTaxSolver, every printed Form 6251 line ──")
    if not os.environ.get("OTS_DIR"):
        print("  SKIPPED — OTS_DIR is unset, so AMT rests on ONE oracle for this run.")
        print("  Install: https://sourceforge.net/projects/opentaxsolver/files/OTS_2024/")
        return 0

    sys.path.insert(0, str(ROOT / "scripts" / "oracle"))
    import ots_direct as o  # noqa: E402 — deliberately late; needs OTS_DIR at import time

    status_token = {"mfs": "Married/Sep", "mfj": "Married/Joint"}
    print(f"  {'vec':5} {'line':8} {'OTS':>14} {'btctax':>14}   verdict")
    bad = compared = 0
    for v in vectors:
        vid, i, d, want = v["id"], v["inputs"], v["derived"], v["form6251"]
        if vid in OTS_CANNOT_WITNESS:
            print(f"  {vid:5} {'—':8} {'not witnessed':>14} {'':>14}   {OTS_CANNOT_WITNESS[vid]}")
            continue
        work = pathlib.Path(tempfile.mkdtemp(prefix="ots-f6251-"))
        try:
            vals = {
                "Status": status_token[i["filing_status"]],
                "L1a": float(i["wages"]), "L2b": 0, "L3a": 0, "L3b": 0,
                "S1_1": float(i["state_refund"]), "S1_3": 0, "S1_15": 0,
                "S2_4": 0, "S2_11": 0, "S3_1": float(i["sch3_line1_ftc"]),
                # ★ Form 6251 line 8 is an INPUT to OTS, not something it derives: i6251 has the filer
                #   compute the AMT foreign tax credit on a separate AMT Form 1116. btctax carries the
                #   §904(j) de-minimis election through unchanged, so the AMT FTC equals the regular
                #   one. Omitting this left OTS's line 9 high by exactly the credit.
                "AMTws8": float(i["sch3_line1_ftc"]),
            }
            if d["itemized"]:
                # A11 is cash/check charity — NOT A16 ("other"), which sails past Schedule A's own
                # handling of the gift. The vectors' only itemized component is the cash gift.
                vals |= {"A5a": 0, "A5b": 0, "A8a": 0, "A11": float(i["cash_gift"])}
            parsed, _ = o.run_form("US_1040", "US_1040", "US_1040", vals, work,
                                   capgains=o._capgains_rows(0.0, float(i["net_ltcg"])))
            got = o._form6251_lines(parsed)
        finally:
            shutil.rmtree(work, ignore_errors=True)
        if not got:
            print(f"  {vid:5} {'—':8} {'no 6251 printed':>14} {'':>14}   not witnessed")
            continue
        for ln in OTS_COMPARED_LINES:
            if ln not in got or ln not in want:
                continue
            a, b = round(float(got[ln])), round(float(want[ln]))
            compared += 1
            if abs(a - b) <= 1:
                continue
            why = OTS_METHODOLOGY_DIFF.get((vid, ln))
            if why:
                print(f"  {vid:5} {ln:8} {a:>14,} {b:>14,}   METHODOLOGY \u2014 {why}")
            else:
                bad += 1
                print(f"  {vid:5} {ln:8} {a:>14,} {b:>14,}   \u2605 UNEXPECTED")
    print(f"\n  {'FAIL' if bad else 'OK'}: {compared} lines compared against OTS, {bad} unexpected, "
          f"{len(OTS_METHODOLOGY_DIFF)} methodology; {len(OTS_CANNOT_WITNESS)} vector(s) not witnessed.")
    return bad


if __name__ == "__main__":
    sys.exit(main())
