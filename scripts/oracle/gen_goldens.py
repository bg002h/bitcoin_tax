#!/usr/bin/env python3
"""Generate the P7 golden-return expectations from an INDEPENDENT tax engine.

★ Why this exists
-----------------
Every test btctax had before P7 was **self-referential**: the core printed chains agree with
the fillers, the forms tie out to each other, and the packet is internally consistent. None of
that can catch an *internally consistent wrong number* — a return where every form cross-foots
beautifully and the tax is simply wrong. SPEC §10's Layer 2 is precisely this gap: "synthetic
end-to-end vs an **independent oracle**".

This script is that oracle. It runs a **separate implementation of the US individual income
tax** (`tenforty`, which wraps Open Tax Solver) over a matrix of synthetic households and
writes the resulting line values into a committed golden file. A Rust test then builds the same
households through btctax's own engine and diffs every line against these numbers.

★ Licensing / clean-room posture (SPEC §9, recon 05)
----------------------------------------------------
OTS is GPL-2.0, which is incompatible with our `MIT OR Unlicense`. We therefore use it
**observe-only**: we RUN it and compare numbers. We do not read, copy, link, vendor or
distribute its source, and nothing it produces enters btctax's implementation — only its
*outputs* land in the golden file, and computed tax figures are FACTS, not copyrightable
expression (the same reasoning already applied to the bundled price data).

★ Why the numbers are BAKED and not computed in CI
--------------------------------------------------
btctax is an offline-first tool with a locked, network-free CI. A test suite that pip-installs
a Python engine at build time would break that property and make the suite non-hermetic. So the
oracle runs HERE, by hand, and its answers are committed. Re-run it when the matrix changes:

    python3 -m venv .venv && .venv/bin/pip install tenforty
    .venv/bin/python scripts/oracle/gen_goldens.py > crates/btctax-core/tests/goldens/full_return_goldens.json

★ What the oracle does NOT cover
--------------------------------
It models the CTC/ODC and the EIC; btctax **conservatively omits both** (§3.4 — omitting a
favorable credit only ever OVERSTATES tax, and the advisories say so loudly). So the households
below carry **no dependents**, which makes `total_tax` directly comparable. The crypto-specific
machinery (the §170(e) charitable reduction, the 8949 row construction, basis/lot selection) has
no counterpart in any general engine and is covered by btctax's own hand-worked KATs — but the
*consequences* of that machinery (a capital gain, a Schedule C profit) are exactly what this
oracle checks.
"""

import json
import subprocess
import sys
from datetime import date

try:
    import tenforty
except ImportError:  # pragma: no cover
    sys.exit("run inside a venv with `pip install tenforty taxcalc`")

try:
    import pandas as pd
    import taxcalc as tc
except ImportError:  # pragma: no cover
    sys.exit("run inside a venv with `pip install tenforty taxcalc`")


# ── The matrix (SPEC §10 / plan P7): single & MFJ · standard & itemized · ±QD+LTCG ·
#    under/over $100k · multi-W-2 · self-employment (the crypto Schedule C) · a loss year.
HOUSEHOLDS = [
    {
        "name": "single_w2_only_standard",
        "why": "the floor case — one W-2, standard deduction, no crypto at all",
        "filing_status": "Single",
        "w2_income": 62_000,
    },
    {
        "name": "single_w2_plus_crypto_ltcg",
        "why": "the core btctax case: wages + a long-term crypto gain (Sch D → 1040 L7)",
        "filing_status": "Single",
        "w2_income": 62_000,
        "long_term_capital_gains": 20_000,
    },
    {
        "name": "single_qdcgt_both_slices",
        "why": "the QDCGT worksheet with BOTH preferential slices — qualified dividends AND LTCG",
        "filing_status": "Single",
        "w2_income": 90_000,
        "qualified_dividends": 8_000,
        "ordinary_dividends": 10_000,
        "taxable_interest": 2_000,
        "long_term_capital_gains": 25_000,
    },
    {
        "name": "single_short_term_crypto_gain",
        "why": "a SHORT-term crypto gain — ordinary rates, no preferential slice",
        "filing_status": "Single",
        "w2_income": 55_000,
        "short_term_capital_gains": 12_000,
    },
    {
        "name": "single_capital_loss_capped",
        "why": "§1211(b): a big net capital loss is capped at $3,000 against ordinary income",
        "filing_status": "Single",
        "w2_income": 70_000,
        "short_term_capital_gains": -18_000,
    },
    {
        "name": "mfj_two_w2_standard",
        "why": "MFJ, multi-W-2, standard deduction — the common household",
        "filing_status": "Married/Joint",
        "w2_income": 185_000,
        "taxable_interest": 1_200,
    },
    {
        "name": "mfj_itemized_over_100k",
        "why": "MFJ, ITEMIZED (over the $29,200 standard), over $100k — the Schedule A path",
        "filing_status": "Married/Joint",
        "w2_income": 240_000,
        "qualified_dividends": 5_000,
        "ordinary_dividends": 6_000,
        "long_term_capital_gains": 30_000,
        "standard_or_itemized": "Itemized",
        "itemized_deductions": 41_000,
    },
    {
        "name": "mfj_high_income_niit_and_addl_medicare",
        "why": "over the $250k MFJ thresholds — Form 8960 NIIT *and* Form 8959 Additional Medicare",
        "filing_status": "Married/Joint",
        "w2_income": 300_000,
        "taxable_interest": 5_000,
        "ordinary_dividends": 12_000,
        "qualified_dividends": 9_000,
        "long_term_capital_gains": 60_000,
    },
    {
        "name": "single_crypto_business_se",
        "why": "crypto MINING as a trade or business: Schedule C → Schedule SE (deep/02 Ex.2 shape)",
        "filing_status": "Single",
        "w2_income": 40_000,
        "self_employment_income": 60_000,
    },
    {
        "name": "mfj_se_over_the_addl_medicare_threshold",
        "why": "SE income pushing the household over $250k — 8959 Part II (the SE leg) engages",
        "filing_status": "Married/Joint",
        "w2_income": 220_000,
        "self_employment_income": 80_000,
    },
]

OUTPUT_LINES = [
    "federal_adjusted_gross_income",
    "federal_taxable_income",
    "federal_income_tax",
    "federal_se_tax",
    "federal_niit",
    "federal_additional_medicare_tax",
    "federal_total_tax",
]


def taxcalc_run(households):
    """★ The SECOND oracle — PSL Tax-Calculator (CC0), a completely separate lineage from OTS.

    Two independent engines are worth far more than one. A single oracle can only tell you that you
    DISAGREE with it; it cannot tell you which of you is wrong. When P7.1's first oracle produced a
    self-employment tax that was INVARIANT to W-2 wages — flat at $11,304 whether wages were $0 or
    $300,000 — the form text said btctax was right, but "we disagree with the only oracle we have"
    is not a position to file a tax return from. This engine broke the tie: it reproduces btctax's
    §1402(b)(1) behaviour exactly, including the discriminating middle case ($100,000 of wages ⇒
    $10,649, where the band is partly but not fully consumed).

    Variables are Tax-Calculator's own (`e00200p` wages, `e00900p` Schedule C net profit, …); the
    outputs we take are the ones whose definitions are unambiguous across engines. We do NOT take its
    `combined`/`iitax` totals: those bundle payroll tax on W-2 wages, which 1040 line 24 does not.
    """
    rows = []
    for n, h in enumerate(households):
        i = h
        rows.append(
            {
                "RECID": n + 1,
                "FLPDYR": 2024,
                "MARS": 2 if i.get("filing_status") == "Married/Joint" else 1,
                "e00200": i.get("w2_income", 0),
                "e00200p": i.get("w2_income", 0),
                "e00200s": 0,
                "e00300": i.get("taxable_interest", 0),          # taxable interest
                "e00600": i.get("ordinary_dividends", 0),        # ordinary dividends (incl. qualified)
                "e00650": i.get("qualified_dividends", 0),       # the qualified subset
                "p22250": i.get("short_term_capital_gains", 0),  # short-term gain/loss
                "p23250": i.get("long_term_capital_gains", 0),   # long-term gain/loss
                "e00900": i.get("self_employment_income", 0),    # Schedule C net profit
                "e00900p": i.get("self_employment_income", 0),
                "e00900s": 0,
                "e19200": i.get("itemized_deductions", 0),       # interest paid ⇒ forces itemizing
                "s006": 1.0,
            }
        )
    df = pd.DataFrame(rows)
    recs = tc.Records(
        data=df, start_year=2024, gfactors=None, weights=None, adjust_ratios=None
    )
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(2024)
    calc.calc_all()
    return [
        {
            "adjusted_gross_income": float(calc.array("c00100")[n]),
            "taxable_income": float(calc.array("c04800")[n]),
            "income_tax_before_credits": float(calc.array("taxbc")[n]),
            "se_tax": float(calc.array("setax")[n]),
            "niit": float(calc.array("niit")[n]),
            "additional_medicare_tax": float(calc.array("ptax_amc")[n]),
        }
        for n in range(len(households))
    ]


def main() -> None:
    inputs = [{k: v for k, v in h.items() if k not in ("name", "why")} for h in HOUSEHOLDS]
    second = taxcalc_run(inputs)

    goldens = []
    for h, args, tc_out in zip(HOUSEHOLDS, inputs, second):
        r = tenforty.evaluate_return(year=2024, **args)
        goldens.append(
            {
                "name": h["name"],
                "why": h["why"],
                "inputs": args,
                "expected": {k: getattr(r, k) for k in OUTPUT_LINES},
                "expected_taxcalc": tc_out,
            }
        )

    version = subprocess.run(
        [sys.executable, "-m", "pip", "show", "tenforty"],
        capture_output=True,
        text=True,
    ).stdout
    version = next(
        (l.split(": ", 1)[1] for l in version.splitlines() if l.startswith("Version")),
        "unknown",
    )

    print(
        json.dumps(
            {
                "_provenance": {
                    "oracle": "tenforty (wraps Open Tax Solver) — an INDEPENDENT implementation",
                    "oracle_version": version,
                    "oracle_2": (
                        "PSL Tax-Calculator (CC0) — a SECOND independent implementation, of a "
                        "completely different lineage. Two oracles are worth far more than one: a "
                        "single oracle can only tell you that you DISAGREE, never which of you is "
                        "wrong. This one broke the tie on the SE-tax divergence, reproducing "
                        "btctax's §1402(b)(1) behaviour exactly."
                    ),
                    "oracle_2_version": tc.__version__,
                    "tax_year": 2024,
                    "generated": date.today().isoformat(),
                    "generator": "scripts/oracle/gen_goldens.py",
                    "licensing": (
                        "OTS is GPL-2.0 and INCOMPATIBLE with our MIT OR Unlicense. Used "
                        "OBSERVE-ONLY: we run it and compare numbers. No source is read, "
                        "copied, linked, vendored or distributed; only its computed FIGURES "
                        "land here, and figures are facts, not copyrightable expression."
                    ),
                    "why_baked": (
                        "btctax is offline-first with a network-free CI. Baking the oracle's "
                        "answers keeps the suite hermetic; re-run the generator when the matrix "
                        "changes."
                    ),
                    "not_covered": (
                        "The oracle models CTC/ODC and EIC; btctax conservatively OMITS both "
                        "(§3.4), so every household here has NO dependents and total_tax is "
                        "directly comparable."
                    ),
                },
                "households": goldens,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
