#!/usr/bin/env python3
"""Generate the P7 golden-return expectations from TWO INDEPENDENT tax engines.

★ Why this exists
-----------------
Every test btctax had before P7 was **self-referential**: the core printed chains agree with
the fillers, the forms tie out to each other, and the packet is internally consistent. None of
that can catch an *internally consistent wrong number* — a return where every form cross-foots
beautifully and the tax is simply wrong. SPEC §10's Layer 2 is precisely this gap: "synthetic
end-to-end vs an **independent oracle**".

★ Why TWO engines
-----------------
A single oracle can only tell you that you DISAGREE with it. It can never tell you which of you
is wrong, and "we disagree with the only oracle we have, but we're confident" is not a position
to file a tax return from. Two engines of separate lineage turn a stand-off into evidence.

  1. **OpenTaxSolver 2024** — its own C binaries, driven directly (see `ots_direct.py`).
  2. **PSL Tax-Calculator** (CC0) — a completely different lineage, from the Policy Simulation
     Library.

★ Why NOT `tenforty` (which this script used to use)
----------------------------------------------------
P7.1's first oracle was `tenforty`, a Python wrapper around OTS. It has two input-plumbing
defects, both of which OVERSTATE a self-employed filer's tax: it never populates Schedule SE
line 8a (so the 12.4% OASDI rate is charged on earnings the wage base has already absorbed —
SE tax comes out invariant to W-2 wages), and it never supplies the §199A QBI deduction on
1040 line 13. Both were reported upstream (mmacpherson/tenforty#278, fix in #279).

The engine underneath was never at fault: OTS's Schedule SE reads `L8a`, its 1040 reads `L13`,
and it ships a Form 8995 solver. So we dropped the wrapper and kept the engine. Every
divergence the old golden file declared was an artifact of the wrapper, and all of them are
gone: OTS-direct, Tax-Calculator and btctax now agree.

★ Licensing / clean-room posture (SPEC §9, recon 05)
----------------------------------------------------
OTS is GPL-2.0, INCOMPATIBLE with our `MIT OR Unlicense`. Used **observe-only**: we run it and
compare numbers. No source is read, copied, linked, vendored or distributed; only its computed
FIGURES land in the golden file, and computed tax figures are FACTS, not copyrightable
expression. Tax-Calculator is CC0.

★ Why the numbers are BAKED and not computed in CI
--------------------------------------------------
btctax is offline-first with a locked, network-free CI. A suite that pip-installed a Python
engine (or shelled out to a GPL binary) at build time would break that property and make the
suite non-hermetic. So the oracles run HERE, by hand, and their answers are committed:

    python3 -m venv .venv && .venv/bin/pip install taxcalc pandas
    export OTS_DIR=/path/to/OpenTaxSolver2024_22.07_linux64
    .venv/bin/python scripts/oracle/gen_goldens.py \
        > crates/btctax-core/tests/goldens/full_return_goldens.json

★ What the oracles do NOT cover
-------------------------------
They model the CTC/ODC and the EIC; btctax **conservatively omits both** (§3.4 — omitting a
favorable credit only ever OVERSTATES tax, and the advisories say so loudly). So the households
below carry **no dependents**, which makes `total_tax` directly comparable. The crypto-specific
machinery (the §170(e) charitable reduction, the 8949 row construction, basis/lot selection) has
no counterpart in any general engine and is covered by btctax's own hand-worked KATs — but the
*consequences* of that machinery (a capital gain, a Schedule C profit) are exactly what these
oracles check.
"""

import json
import subprocess
import sys
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

try:
    import ots_direct
except ImportError:  # pragma: no cover
    sys.exit("scripts/oracle/ots_direct.py must sit beside this script")

try:
    import pandas as pd
    import taxcalc as tc
except ImportError:  # pragma: no cover
    sys.exit("run inside a venv with `pip install taxcalc pandas`")


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
        "why": "MFJ, standard deduction, a little interest BELOW the $1,500 Schedule B trigger — the "
        "common household, and the discriminating case for whether Schedule B files at all",
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
        "name": "mfj_itemized_salt_over_the_cap",
        "why": "§164(b)(5): state income tax + real estate tax EXCEED the $10,000 SALT cap, and itemizing "
        "still wins — so the cap BINDS and changes the tax. No other golden exercises it: the matrix's "
        "other itemized household carries one lump sum. The SALT figures are IRS ATS Test Scenario 2's "
        "($1,068 state income tax + $10,509 real estate = $11,577, capped to $10,000); the mortgage is "
        "raised so the itemized total clears the $29,200 standard deduction, which Scenario 2's own "
        "numbers do NOT (its Schedule A totals $28,289 — the IRS scenario tests e-file schema, not "
        "whether itemizing wins).",
        "filing_status": "Married/Joint",
        "w2_income": 38_730,  # ATS Scenario 2's two W-2s: $29,513 + $9,217
        "state_income_tax": 1_068,  # Sch A 5a
        "real_estate_tax": 10_509,  # Sch A 5b  ⇒ 5d = 11,577 > the $10,000 cap
        "mortgage_interest": 25_000,  # Sch A 8a
        "standard_or_itemized": "Itemized",
    },
    {
        "name": "single_crypto_business_se",
        "why": "crypto MINING as a trade or business: Schedule C → Schedule SE (deep/02 Ex.2 shape)",
        "filing_status": "Single",
        "w2_income": 40_000,
        "self_employment_income": 60_000,
    },
    {
        "name": "single_miner_qbi_limited_by_net_capital_gain",
        "why": "★ Form 8995 line 12. The §199A deduction is capped at 20% of (taxable income − NET "
        "CAPITAL GAIN), and here that limit BINDS *because of* the gain: 20% × QBI = 11,152, but "
        "20% × (81,161 − 40,000) = 8,232, so the capital gain costs this miner $2,920 of deduction. "
        "Drop the line-12 subtraction and the deduction silently grows to 11,152 — understating tax. "
        "No other golden combines QBI with a capital gain, so nothing else holds line 12 against an "
        "oracle.",
        "filing_status": "Single",
        "self_employment_income": 60_000,
        "long_term_capital_gains": 40_000,
    },
    {
        "name": "mfj_se_over_the_addl_medicare_threshold",
        "why": "SE income pushing the household over $250k — 8959 Part II (the SE leg) engages, and "
        "the W-2 wages have already consumed the OASDI band (Sch SE L8a/L9/L10)",
        "filing_status": "Married/Joint",
        "w2_income": 220_000,
        "self_employment_income": 80_000,
    },
]


def taxcalc_run(households):
    """★ Oracle #2 — PSL Tax-Calculator (CC0), a lineage completely separate from OTS.

    Variables are Tax-Calculator's own (`e00200p` wages, `e00900p` Schedule C net profit, …).
    Note the per-person suffixes: `p` is the primary filer, `s` the spouse. Wages and SE income
    both go to the PRIMARY, i.e. the model is "one person earned both" — which is exactly what
    btctax's `se_w2_ss_wages` input means (the filer's OWN box-3 wages) and what we hand OTS on
    Schedule SE line 8a. All three engines are answering the same question.

    We do NOT take its `combined`/`iitax` totals: those bundle payroll tax on W-2 wages, which
    1040 line 24 does not.
    """
    rows = []
    for n, i in enumerate(households):
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
                # Schedule A, as separate components — the SALT cap can only be exercised if state
                # income tax and real estate tax reach the engine as themselves, not as one lump.
                "e18400": i.get("state_income_tax", 0),           # state & local income tax  (Sch A 5a)
                "e18500": i.get("real_estate_tax", 0),            # real estate tax           (Sch A 5b)
                "e19200": i.get("itemized_deductions", 0)         # interest paid             (Sch A 8a)
                + i.get("mortgage_interest", 0),
                "s006": 1.0,
            }
        )
    recs = tc.Records(
        data=pd.DataFrame(rows), start_year=2024, gfactors=None, weights=None, adjust_ratios=None
    )
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(2024)
    calc.calc_all()
    return [
        {
            "adjusted_gross_income": float(calc.array("c00100")[n]),
            "taxable_income": float(calc.array("c04800")[n]),
            "qbi_deduction": float(calc.array("qbided")[n]),
            "income_tax_before_credits": float(calc.array("taxbc")[n]),
            "se_tax": float(calc.array("setax")[n]),
            "niit": float(calc.array("niit")[n]),
            "additional_medicare_tax": float(calc.array("ptax_amc")[n]),
        }
        for n in range(len(households))
    ]


def main() -> None:
    inputs = [{k: v for k, v in h.items() if k not in ("name", "why")} for h in HOUSEHOLDS]

    ots = [ots_direct.evaluate(h) for h in inputs]
    taxcalc = taxcalc_run(inputs)

    goldens = [
        {
            "name": h["name"],
            "why": h["why"],
            "inputs": args,
            "expected_ots": o,
            "expected_taxcalc": t,
        }
        for h, args, o, t in zip(HOUSEHOLDS, inputs, ots, taxcalc)
    ]

    tc_version = subprocess.run(
        [sys.executable, "-m", "pip", "show", "taxcalc"], capture_output=True, text=True
    ).stdout
    tc_version = next(
        (l.split(": ", 1)[1] for l in tc_version.splitlines() if l.startswith("Version")),
        tc.__version__,
    )

    print(
        json.dumps(
            {
                "_provenance": {
                    "oracle_1": (
                        "OpenTaxSolver 2024 — its own C binaries, driven DIRECTLY (not through the "
                        "`tenforty` wrapper, which drops Schedule SE line 8a and the §199A deduction; "
                        "reported as mmacpherson/tenforty#278, fix in #279). The engine itself was "
                        "never at fault and reproduces these figures to the cent."
                    ),
                    "oracle_1_version": ots_direct.version(),
                    "oracle_2": (
                        "PSL Tax-Calculator (CC0) — a SECOND independent implementation of a completely "
                        "different lineage. Two oracles are worth far more than one: a single oracle can "
                        "only tell you that you DISAGREE, never which of you is wrong."
                    ),
                    "oracle_2_version": tc_version,
                    "tax_year": 2024,
                    "generated": date.today().isoformat(),
                    "generator": "scripts/oracle/gen_goldens.py",
                    "licensing": (
                        "OTS is GPL-2.0 and INCOMPATIBLE with our MIT OR Unlicense. Used OBSERVE-ONLY: "
                        "we run it and compare numbers. No source is read, copied, linked, vendored or "
                        "distributed; only its computed FIGURES land here, and figures are facts, not "
                        "copyrightable expression. Tax-Calculator is CC0."
                    ),
                    "why_baked": (
                        "btctax is offline-first with a network-free CI. Baking the oracles' answers "
                        "keeps the suite hermetic; re-run the generator when the matrix changes."
                    ),
                    "not_covered": (
                        "Both oracles model CTC/ODC and EIC; btctax conservatively OMITS both (§3.4), so "
                        "every household here has NO dependents and total_tax is directly comparable."
                    ),
                },
                "households": goldens,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
