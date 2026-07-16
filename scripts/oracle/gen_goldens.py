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
    cargo build -p btctax-oracle-harness   # the §9 harness the D-2 admission loop drives
    export OTS_DIR=/path/to/OpenTaxSolver2024_22.07_linux64
    .venv/bin/python scripts/oracle/gen_goldens.py \
        > crates/btctax-core/tests/goldens/full_return_goldens.json

★ What the oracles do NOT cover
-------------------------------
They model the CTC/ODC and the EIC; btctax **conservatively omits both** (§3.4 — omitting a
favorable credit only ever OVERSTATES tax, and the advisories say so loudly). So the corpus
(`corpus.py`) carries **no dependents**, which makes `total_tax` directly comparable. The crypto-specific
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
    import corpus
except ImportError:  # pragma: no cover
    sys.exit("scripts/oracle/corpus.py must sit beside this script")

try:
    import pandas as pd
    import taxcalc as tc
except ImportError:  # pragma: no cover
    sys.exit("run inside a venv with `pip install taxcalc pandas`")

# ── The §9 test-only harness binary — the D-2 refusal-free admission gate (SPEC §4/§5.1, plan T10) ──
# Generation is Python and btctax's assembly is Rust, so the D-2 check (does btctax assemble this
# scenario without a refusal?) crosses that boundary through the COMPILED harness, never a Python
# re-implementation of the AMT/QBI screens that could drift (r2-M4). Build it once with
# `cargo build -p btctax-oracle-harness`; this is its debug-profile path.
HARNESS_BIN = Path(__file__).resolve().parents[2] / "target" / "debug" / "btctax-oracle-harness"


# ── The corpus (SPEC §5.1 / plan T10) ─────────────────────────────────────────────────────────────
# The hand-written 12-household list moved to `corpus.py` — where it survives VERBATIM as the 12
# anchors, joined by the 2 bake-time-steered pinned liveness cells and a generated variable-strength
# covering array. `corpus.households()` is the full candidate list; `main()` admits it refusal-free
# (D-2) through the §9 harness before baking.


def _taxcalc_row(n, i):
    """One Tax-Calculator input record from a household's `inputs` dict (the variable mapping the old
    inline builder used — factored out so the D-2 AMT/credit admission probe reuses it verbatim)."""
    return {
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
        # Schedule A, as separate components — the SALT cap can only be exercised if state income tax
        # and real estate tax reach the engine as themselves, not as one lump.
        "e18400": i.get("state_income_tax", 0),           # state & local income tax  (Sch A 5a)
        "e18500": i.get("real_estate_tax", 0),            # real estate tax           (Sch A 5b)
        "e19200": i.get("itemized_deductions", 0)         # interest paid             (Sch A 8a)
        + i.get("mortgage_interest", 0),
        "s006": 1.0,
    }


def taxcalc_run(households):
    """★ Oracle #2 — PSL Tax-Calculator (CC0), a lineage completely separate from OTS.

    Variables are Tax-Calculator's own (`e00200p` wages, `e00900p` Schedule C net profit, …).
    Note the per-person suffixes: `p` is the primary filer, `s` the spouse. Wages and SE income
    both go to the PRIMARY, i.e. the model is "one person earned both" — which is exactly what
    btctax's `se_w2_ss_wages` input means (the filer's OWN box-3 wages) and what we hand OTS on
    Schedule SE line 8a. All three engines are answering the same question.

    For the 1040 line 24 equivalent we take `c09200` — income tax (incl. AMT) after the
    nonrefundable credits, PLUS the Schedule 2 "other taxes" (SE tax, NIIT, Additional
    Medicare), before refundable credits — which for these creditless households IS line 24.
    We do NOT take `combined` (it ADDS the W-2 payroll tax that line 24 excludes), nor `iitax`
    (taxcalc books SE tax and Additional Medicare as *payroll*, so `iitax` OMITS them though
    line 24 includes them). taxcalc exposes only the EXACT totals `setax`/`ptax_amc` — no
    OASDI/Medicare or 8959 Part I/II leg split, and no granular 8995-line-12 variable — so the
    paper-level cross-foot legs (Sch SE L12, 8959 L18) and the 8995 L12 cap are left ABSENT
    here; those lines are OTS-single-witness (SPEC §6.4 `Option` rule).
    """
    rows = [_taxcalc_row(n, i) for n, i in enumerate(households)]
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
            # ── T9 · deeper lines + provenance leaves taxcalc CAN express ──────────────
            # Deduction actually subtracted (Form 1040 L12): taxcalc zeroes `standard`
            # for itemizers and `c04470` for non-itemizers, so this is whichever it took.
            "deduction_taken": (
                float(calc.array("c04470")[n])
                if calc.array("c04470")[n] > 0
                else float(calc.array("standard")[n])
            ),
            "salt_capped": float(calc.array("c18300")[n]),  # Sch A L5e = min(5d, $10k cap)
            "sch_d_to_l7": float(calc.array("c01000")[n]),  # Sch D L21 → 1040 L7 (§1211-limited)
            "total_tax": float(calc.array("c09200")[n]),    # 1040 L24 equiv (see docstring)
            # provenance leaves
            "qual_div_l3a": float(calc.array("e00650")[n]),  # 1040 L3a qualified dividends
            "net_ltcg_qd_exclusive": max(  # §1(h) subterm max(0, min(LTCG, LTCG+STCG)) — QD-EXCLUSIVE (r5-N2)
                0.0,
                min(
                    float(calc.array("p23250")[n]),
                    float(calc.array("p23250")[n]) + float(calc.array("p22250")[n]),
                ),
            ),
            # ★ se_l10_oasdi / se_l11_medicare / f8959_l7 / f8959_l13 / qbi_cap_l12 are
            #   DELIBERATELY ABSENT (⇒ serde(default) None): taxcalc exposes only the EXACT
            #   totals `setax` / `ptax_amc` — no OASDI/Medicare or 8959 Part I/II leg split —
            #   and no granular 8995-L12 variable (its `net_cg` cap is an inline local, never
            #   a records variable). Those paper-level cross-foot legs are OTS-single-witness
            #   (SPEC §6.4 `Option` rule); fabricating a taxcalc value would be a false oracle.
        }
        for n in range(len(households))
    ]


def _harness_default(inputs):
    """Drive the §9 harness in DEFAULT mode: assemble+fill+read-back one scenario. Returns the parsed
    `{"refused": bool, "lines": {...}}` object, or `{"malformed": True}` on exit-code 2 (a stdin the
    harness could not parse as a `GoldenInputs` scenario — distinct from a lawful refusal)."""
    if not HARNESS_BIN.exists():
        sys.exit(
            f"{HARNESS_BIN} not found — build it first: `cargo build -p btctax-oracle-harness` (T7)."
        )
    proc = subprocess.run([str(HARNESS_BIN)], input=json.dumps(inputs), capture_output=True, text=True)
    if proc.returncode == 2:  # malformed input — NOT a refusal (harness contract)
        return {"malformed": True, "stderr": proc.stderr.strip()}
    if proc.returncode != 0:  # pragma: no cover — a harness crash is a generator bug, surface it loudly
        raise RuntimeError(f"oracle_harness exited {proc.returncode}: {proc.stderr.strip()}")
    return json.loads(proc.stdout)


def _taxcalc_amt_credits(inputs_list):
    """One vectorized Tax-Calculator pass over ALL candidates → [(AMT c09600, credits c07100), …].
    The D-2 admission predicate reads these as oracle-2's 1040 L17 (AMT) and L21 (credits)."""
    df = pd.DataFrame([_taxcalc_row(n, i) for n, i in enumerate(inputs_list)])
    recs = tc.Records(data=df, start_year=2024, gfactors=None, weights=None, adjust_ratios=None)
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(2024)
    calc.calc_all()
    return [
        (float(calc.array("c09600")[n]), float(calc.array("c07100")[n]))
        for n in range(len(inputs_list))
    ]


def admit(candidates):
    """The D-2 refusal-free + AMT/credit-free ADMISSION loop (SPEC §4). Each candidate is piped
    through the §9 harness binary; a `refused` candidate is REJECTED (never silently kept). Admitted
    only if btctax assembles it AND both oracles report zero AMT and zero L21 credits:

      * btctax (the harness): `refused == false`, and paper **L17 == 0** (AMT/APTC) and **L21 == 0**
        (credits) — the exact L24-cross-foot precondition (`golden_packet.rs:104-119`). The Form 6251
        AMT screen inside `assemble` makes `refused` the OTS-side AMT guard too (same assembled
        return; `ots_direct` is frozen at T8 and surfaces no AMT line of its own).
      * taxcalc (oracle 2): **c09600 == 0** (AMT) and **c07100 == 0** (nonrefundable credits → L21).

    Returns `(admitted, rejected)` where `rejected` is a list of `(name, reason)` — logged, not dropped
    silently.
    """
    amt_credits = _taxcalc_amt_credits([h["inputs"] for h in candidates])
    admitted, rejected = [], []
    for h, (amt, credits) in zip(candidates, amt_credits):
        hv = _harness_default(h["inputs"])
        if hv.get("malformed"):  # pragma: no cover — a generator-side shape bug, not a tax case
            rejected.append((h["name"], f"harness rejected the scenario shape: {hv['stderr'][:100]}"))
            continue
        if hv["refused"]:
            rejected.append((h["name"], "btctax REFUSED (AMT screen / QBI-over-threshold / unmodeled input)"))
            continue
        lines = hv["lines"]
        l17 = int(lines.get("1040.line17", "0"))
        l21 = int(lines.get("1040.line21", "0"))
        if l17 or l21:
            rejected.append((h["name"], f"btctax paper L17={l17} L21={l21} (AMT/credit ⇒ not L24-comparable)"))
            continue
        if amt or credits:
            rejected.append((h["name"], f"taxcalc AMT c09600={amt} credits c07100={credits}"))
            continue
        admitted.append(h)
    return admitted, rejected


def verify_pinned_cells(admitted, ots, taxcalc):
    """CHECK the two §5.1 bake-time-steered liveness cells actually flip their intended L16 class, at
    generation time, by driving the §9 harness `--check` (the SAME `oracle_diff` reproduction the
    golden tests use — never a Python re-implementation of the Tax Table). Raises if a cell fails to
    flip, so a steering that decayed under an engine bump can never bake a dead pinned cell.

    The flip signal is `internal` (btctax's own §3.1 line-16 = `round_dollar(regular_tax)` =
    `table_l16` on btctax's operands) ≠ the oracle's rounded L16 — i.e. the provenance conjunct-2
    holds (btctax's own Table/schedule does NOT reproduce that oracle's L16 on btctax's operands),
    while the oracle reproduces itself (conjunct-1, the Tax-Table/schedule identity below/at the
    ceiling). The bin-edge cell must sit BELOW the $100k ceiling (OTS provenance vs the $50 bins); the
    cents-flip cell must sit AT/ABOVE it (taxcalc provenance vs the exact schedule, methodology off).
    """
    by_name = {h["name"]: (h, o, t) for h, o, t in zip(admitted, ots, taxcalc)}
    for cell in corpus.PINNED_CELLS:
        if cell["name"] not in by_name:  # pragma: no cover — a dropped pinned cell is a hard error
            sys.exit(f"PINNED CELL {cell['name']} was not admitted — cannot hold its class live (D-2 rejected it?).")
        h, o, t = by_name[cell["name"]]
        household = {"name": h["name"], "why": h["why"], "inputs": h["inputs"], "expected_ots": o, "expected_taxcalc": t}
        proc = subprocess.run(
            [str(HARNESS_BIN), "--check"], input=json.dumps(household), capture_output=True, text=True
        )
        if proc.returncode != 0:  # pragma: no cover
            sys.exit(f"oracle_harness --check failed for {cell['name']}: {proc.stderr.strip()}")
        chk = json.loads(proc.stdout)
        repro_ti = float(chk["reproduced_ops"]["ti"])
        l16 = next(v for v in chk["verdicts"] if v["line"] == "1040.line16")
        internal, ots16, tc16 = int(l16["internal"]), int(l16["ots"]), int(l16["taxcalc"])
        below_ceiling = repro_ti < 100_000
        if cell["pins_class"] == "ots_provenance":
            ok = below_ceiling and internal != ots16
            detail = f"below_ceiling={below_ceiling} btctax_L16={internal} OTS_L16={ots16}"
        else:  # taxcalc_provenance
            ok = (not below_ceiling) and internal != tc16
            detail = f"above_ceiling={not below_ceiling} btctax_L16={internal} taxcalc_L16={tc16}"
        if not ok:
            sys.exit(
                f"PINNED CELL {cell['name']} did NOT flip its {cell['pins_class']} class "
                f"(TI={repro_ti}; {detail}) — re-steer its inputs (T10)."
            )
        print(
            f"[pinned] {cell['name']}: {cell['pins_class']} FLIP verified — TI={repro_ti}, {detail}",
            file=sys.stderr,
        )


def main() -> None:
    candidates = corpus.households()
    admitted, rejected = admit(candidates)

    # A rejection is never a silent drop — log every one (name + reason) to stderr.
    for name, reason in rejected:
        print(f"[reject] {name}: {reason}", file=sys.stderr)
    print(
        f"[corpus] {len(candidates)} candidates → {len(admitted)} admitted, {len(rejected)} rejected "
        f"(D-2 refusal/AMT/credit-free).",
        file=sys.stderr,
    )

    # The two pinned cells MUST admit — a dropped pinned cell cannot hold its L16 class live, so that
    # is a hard error. An anchor that D-2 rejects is a LOUD warning, not a hard error: D-2 (refusal-
    # free, the plan's "AMT-triggering scenarios OUT") is authoritative over the "12 anchors" wish, and
    # a rejected anchor is a legitimate REJECTION (logged above), never a silent keep. (Today
    # `mfj_high_income_niit_and_addl_medicare` trips btctax's conservative Form 6251 AMT *screening*
    # worksheet — actual AMT is $0 on both oracles — so the harness refuses it and it drops out.)
    admitted_names = {h["name"] for h in admitted}
    for cell in corpus.PINNED_CELLS:
        if cell["name"] not in admitted_names:  # pragma: no cover
            sys.exit(f"pinned cell {cell['name']!r} was REJECTED by admission — it cannot hold its class live.")
    dropped_anchors = [a["name"] for a in corpus.ANCHORS if a["name"] not in admitted_names]
    if dropped_anchors:
        print(
            f"[warn] {len(dropped_anchors)} anchor(s) dropped by D-2 admission (refusal-free / AMT-out): "
            f"{dropped_anchors} — the baked anchor set is {len(corpus.ANCHORS) - len(dropped_anchors)}, "
            "not 12; reconcile the golden-test anchor list at bake time (T11).",
            file=sys.stderr,
        )

    # t=3 completeness on the two named triples, proven over the ADMITTED corpus (§5.1).
    corpus.assert_named_triple_coverage(admitted)

    inputs = [h["inputs"] for h in admitted]
    ots = [ots_direct.evaluate(i) for i in inputs]
    taxcalc = taxcalc_run(inputs)

    # The two §5.1 pinned cells must ACTUALLY flip their L16 provenance classes (checked, not assumed).
    verify_pinned_cells(admitted, ots, taxcalc)

    goldens = [
        {
            "name": h["name"],
            "why": h["why"],
            "inputs": args,
            "expected_ots": o,
            "expected_taxcalc": t,
        }
        for h, args, o, t in zip(admitted, inputs, ots, taxcalc)
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
                    "corpus": (
                        "SPEC §5.1 variable-strength constrained COVERING ARRAY (scripts/oracle/corpus.py): "
                        "the 12 hand-written anchors (verbatim) + 2 bake-time-steered pinned liveness cells "
                        "+ a generated array — full cartesian on the named triples {SE,LTCG,qual-div} and "
                        "{itemized,SALT-over-cap,high-income} (t=3 by construction), pairwise (t=2) "
                        "elsewhere, under the D-1/D-2/D-3 constraints. Every candidate is ADMITTED "
                        "refusal-free and AMT/credit-free through the §9 oracle_harness binary before it is "
                        "baked, so a scenario btctax would refuse never enters this file."
                    ),
                    "regeneration": (
                        "★ ENGINE-VERSION-GATED (SPEC §11). These figures are computed by the EXTERNAL "
                        "engines pinned in oracle_1_version / oracle_2_version. Re-run the generator — and "
                        "re-review — whenever EITHER version changes: a bump can shift a figure by a cent "
                        "and move a $50 Tax-Table bin edge or a half-dollar rounding boundary, and the two "
                        "§5.1 pinned liveness cells sit deliberately on exactly such edges. Recipe: "
                        "`export OTS_DIR=…; .venv/bin/python scripts/oracle/gen_goldens.py > "
                        "crates/btctax-core/tests/goldens/full_return_goldens.json` (requires the T7 harness "
                        "binary built: `cargo build -p btctax-oracle-harness`)."
                    ),
                    "determinism": (
                        "Every field below is DETERMINISTIC and reproduces byte-for-byte at fixed engine "
                        "versions (no RNG in the baked path — the covering array is a fixed construction) "
                        "EXCEPT `generated` (this date), which is the ONLY non-deterministic field and is "
                        "excluded from the §12 determinism claim."
                    ),
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
