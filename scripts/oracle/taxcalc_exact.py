#!/usr/bin/env python3
"""★★★ THE ONE PLACE that builds a Tax-Calculator run, and the only one that may turn `exact` on.

**The defect this module exists because of (FR-124).** `gen_goldens.py` asked for the stepped
branch by putting `exact: 1` in the Records **input DataFrame**:

    {"RECID": n + 1, "FLPDYR": year, **({"exact": 1} if year in TAXCALC_EXACT_YEARS else {}), ...}

`exact` is a **calculated** variable in taxcalc's `records_variables.json`, not a read variable, so
`tc.Records(data=DataFrame(...))` **silently drops the column** — measured directly on taxcalc 6.8.2:
the array comes back `[0]` after `advance_to_year` and still `[0]` after `calc_all`, while the same
value written through the Calculator comes back `[1]`. So `TAXCALC_EXACT_YEARS = frozenset({2025})`
was inert for a year: every stepped phase-out took taxcalc's **smooth marginal-rate fallback**
instead of the stepped arithmetic a tax form performs, and nothing anywhere said so. That is this
repo's dominant defect shape — *a flag that reads as set, an engine quietly on a different branch,
and everything green.*

**Why the fix is a module and not a line.** By the time FR-124 was written, `verify_schedule_1a.py`
already did it correctly (through the Calculator, and asserting it stuck). Patching `gen_goldens.py`
in place would have left **two implementations of "turn `exact` on" — one right, one that was wrong
for a year** — which is the divergence that produced the defect in the first place, re-armed. So
there is now exactly one construction path, [`build_calculator`], and every engine-2 caller in the
repo goes through it:

| caller | year | `exact` |
|---|---|---|
| `gen_goldens.taxcalc_run` / `taxcalc_credits` / `_taxcalc_amt_credits` | the corpus's (2024) | off — see [`EXACT_OFF_YEARS`] |
| `verify_schedule_1a._taxcalc_applied` | 2025 | **on** |
| `verify_f6251._taxcalc` | 2024 | off |

★★ **The historical defect is now INEXPRESSIBLE through that path**, not merely discouraged:
[`build_calculator`] REFUSES a row dict carrying an `exact` key, naming what the writer meant and
what to do instead. A comment saying "don't do this" is guarded by whoever reads it next; a refusal
is guarded by the interpreter. (`CLAUDE.md`: *"derive the list, or make the compiler hold it"* —
applied to a procedure rather than a list.)

**B1:** `python3 scripts/oracle/taxcalc_exact.py --selftest` watches all of it discriminate, including
the two-directional branch kill that is the only check able to see the original defect: on a
fractional-step vector the stepped and smooth answers differ, so the selftest drives the same vectors
**twice** — once through this module and once through a local reconstruction of the broken path — and
demands the first differ from the smooth value and the second equal it.
"""

import argparse
import sys

# ★★★ THE YEARS `exact` IS **OFF** FOR — and the polarity is the whole point.
#
# The shipped bug was a list of years to turn `exact` ON for, hand-typed as `{2025}`. That list was
# correct on the day it was written and is the exact shape `CLAUDE.md` calls this repo's dominant
# defect class: by 2026-09-12 taxcalc's own parameters put the OBBBA stepped deductions in force for
# **2025, 2026, 2027 and 2028** (measured from `policy_current_law.json`: `TipIncomeDed_c`,
# `OvertimeIncomeDed_c` and `AutoLoanInterestDed_c` are nonzero from 2025 and return to 0 at 2029),
# so an ON-list of `{2025}` was already three years stale and a TY2026 census would have read the
# ±$100/$200 smooth/stepped gap as a **btctax rounding defect**. That is `TY2026_PORT_REPORT.md`'s
# R25, and it is closed here by REVERSING the polarity rather than by extending a list.
#
# `exact == 1` means *"compute as the printed tax forms do"* — taxcalc's own words. Every comparison
# in this repo is against a printed form, so ON is the right default for **every** year, and a year
# bump now gets the form-faithful branch by doing nothing at all.
#
# ★ This set therefore cannot go stale, and that is a property rather than a hope: it names the years
# whose goldens were **already baked** with `exact` off, and it is closed by history. `exact`
# co-governs `AGI`, `MiscDed`, `F2441`, `ChildDepTaxCredit`, `AmOppCreditParts`, `EducationTaxCredit`
# and `CTC_new` (taxcalc `calcfunctions.py`), so switching it on retroactively could move a committed
# `expected_taxcalc` cell. Every generation from this commit forward turns it on, so no future year
# can ever need an entry here — the only way this grows is if someone deliberately bakes a corpus
# with it off, which is a decision taken in a diff with a reason.
EXACT_OFF_YEARS = frozenset({2024})

# The Records columns every caller here supplies as input. Named so the refusal below can say which
# key is the mistake without guessing.
_CALCULATED_NOT_READ = "exact"


def needs_exact(year: int) -> bool:
    """Should this year's run compute as the tax forms do? Every year but a baked-corpus one."""
    return year not in EXACT_OFF_YEARS


def apply_exact(calc, n_rows: int, year: int) -> int:
    """Write `exact` **through the Calculator** — the only route that sticks — and return the value.

    Does NOT assert: `exact` is a calculated variable, so the load-bearing question is what the
    engine held while `calc_all()` ran. [`assert_exact_stuck`] answers that, afterwards.
    """
    import numpy as np

    want = 1 if needs_exact(year) else 0
    calc.array(_CALCULATED_NOT_READ, np.full(n_rows, want, dtype=np.int32))
    return want


def assert_exact_stuck(calc, n_rows: int, want: int) -> None:
    """★ The check that the write took, run AFTER `calc_all()` — i.e. on the value the engine used.

    Two-directional on purpose. `sum == n_rows` alone is satisfied by a run that always writes 1,
    which would silently move a baked TY2024 golden; `sum == 0` alone is satisfied by the original
    defect. Only the equality can tell the two apart.
    """
    got = int(calc.array(_CALCULATED_NOT_READ).sum())
    if got != want * n_rows:
        raise RuntimeError(
            f"taxcalc's `exact` is {got} across {n_rows} row(s) where {want * n_rows} was written "
            f"(want={want} per row). `exact` is a CALCULATED variable: a column named `exact` in the "
            f"Records DataFrame is silently dropped, and every stepped phase-out then takes the "
            f"SMOOTH marginal-rate fallback — up to $100 (Schedule 1-A Parts II/III) or $200 "
            f"(Part IV) per return, reading as a btctax rounding defect. See FR-124."
        )


def build_calculator(rows, year: int, *, policy=None):
    """Records → Calculator → `advance_to_year` → `exact` → `calc_all`. The ONE construction path.

    `rows` is a list of Records **input** dicts. Returns the calculated `tc.Calculator`.

    ★ The refusal below is the point of the function: it makes FR-124's defect unwritable rather
    than merely deprecated.
    """
    import pandas as pd
    import taxcalc as tc

    if not rows:
        # ★ Measured honestly: taxcalc ALSO refuses an empty frame, with
        #   `ValueError: data missing one or more MUST_READ_VARS`. So this guard is not what stands
        #   between us and a vacuous pass — it is what makes the refusal legible at the call site
        #   instead of arriving from `taxcalc/data.py` with no mention of rows.
        raise RuntimeError(
            "build_calculator got no rows — a census over nothing has nothing to witness. "
            "(taxcalc refuses this too, as MUST_READ_VARS; the message is the point.)"
        )
    offenders = [i for i, r in enumerate(rows) if _CALCULATED_NOT_READ in r]
    if offenders:
        raise RuntimeError(
            f"row(s) {offenders[:5]}{'…' if len(offenders) > 5 else ''} carry an "
            f"`{_CALCULATED_NOT_READ}` key. That is FR-124 exactly: `{_CALCULATED_NOT_READ}` is a "
            f"CALCULATED taxcalc variable, so a Records input column of that name is SILENTLY "
            f"DROPPED and the engine stays on its smooth fallback. Delete the key — this function "
            f"already writes `{_CALCULATED_NOT_READ}` through the Calculator for every year outside "
            f"EXACT_OFF_YEARS, and asserts it stuck."
        )
    recs = tc.Records(
        data=pd.DataFrame(rows), start_year=year, gfactors=None, weights=None, adjust_ratios=None
    )
    calc = tc.Calculator(policy=policy if policy is not None else tc.Policy(), records=recs)
    calc.advance_to_year(year)
    want = apply_exact(calc, len(rows), year)
    calc.calc_all()
    assert_exact_stuck(calc, len(rows), want)
    return calc


# ── B1: the kills ─────────────────────────────────────────────────────────────────────────────────
def _smooth_path_calculator(rows, year: int):
    """FR-124's defect, RECONSTRUCTED here so the kill below has something to compare against.

    Deliberately local to the selftest: production offers no "turn exact off" override, because an
    override is a hole, and this is the shape the shipped code had for a year — `exact` handed to
    `tc.Records` as an input column, where it is dropped without a word.
    """
    import pandas as pd
    import taxcalc as tc

    broken = [{**r, _CALCULATED_NOT_READ: 1} for r in rows]
    recs = tc.Records(
        data=pd.DataFrame(broken), start_year=year, gfactors=None, weights=None, adjust_ratios=None
    )
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(year)
    calc.calc_all()
    return calc


def selftest() -> int:
    """Watch every claim in this module discriminate. Needs taxcalc; needs no OTS and no vault."""
    import decimal
    import pathlib

    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    try:
        import taxcalc as tc
    except ImportError:
        sys.exit(
            "selftest needs taxcalc — run it with the repo venv: "
            ".venv/bin/python scripts/oracle/taxcalc_exact.py --selftest"
        )

    tol = decimal.Decimal("0.005")

    # (1) The polarity. 2024's goldens are baked with `exact` off; every other year computes as the
    #     forms do, INCLUDING the years a hand-written ON-list had already gone stale on.
    assert not needs_exact(2024), "the baked TY2024 corpus must keep `exact` off"
    for y in (2025, 2026, 2027, 2028, 2029, 2030):
        assert needs_exact(y), f"TY{y} must compute as the forms do (R25: an ON-list went stale here)"

    row = {"RECID": 1, "FLPDYR": 2025, "MARS": 1, "e00200": 100000.0, "e00200p": 100000.0,
           "e00200s": 0.0, "age_head": 70, "age_spouse": 40, "s006": 1.0}

    # (2) ★ IT STUCK — the first kill, and the one the shipped code would fail. Both directions, so
    #     the assertion cannot be satisfied by a run that always writes the same value.
    calc = build_calculator([{**row, "FLPDYR": 2025}], 2025)
    assert int(calc.array("exact").sum()) == 1, calc.array("exact")
    calc = build_calculator([{**row, "FLPDYR": 2024}], 2024)
    assert int(calc.array("exact").sum()) == 0, calc.array("exact")

    # (3) ★★ THE PLANT: the defect as it shipped. A Records input column named `exact` is refused by
    #     name, so FR-124 cannot be re-typed through the authoritative path.
    try:
        build_calculator([{**row, "exact": 1}], 2025)
    except RuntimeError as e:
        assert "FR-124" in str(e) and "SILENTLY" in str(e), str(e)
    else:
        raise AssertionError("an `exact` input column was ACCEPTED — FR-124 is re-armed")

    # (4) Anti-vacuity: a run over no rows checks nothing and must not report clean.
    try:
        build_calculator([], 2025)
    except RuntimeError as e:
        assert "nothing to witness" in str(e), str(e)
    else:
        raise AssertionError("an empty run was accepted — it would report OK having checked nothing")

    # (5) ★★★ THE BRANCH KILL, two-directional. "The flag is set" is what the broken code also looked
    #     like; the symptom was a SMOOTH value where a stepped one belongs. `verify_schedule_1a`
    #     already expresses that comparison in `_smooth_fallback`, so it is reused, not restated.
    import verify_schedule_1a as v  # noqa: E402 — after the sys.path insert above

    pol = v._policy()[1]
    # ★ Three filters, every one of them the census's own — nothing hand-listed here. A vector can
    #   tell the branches apart only if its excess is a fractional number of steps
    #   (`_discriminating`), if the figure has not already collapsed to the floor at zero
    #   (`_pins_the_arithmetic`), and if taxcalc is not DISQUALIFIED on it — FR-125/FR-126 mean some
    #   vectors get taxcalc's wrong threshold, where the engine never enters the phase-out at all and
    #   "smooth vs stepped" is not the question being asked.
    vectors = [x for x in v._vectors()
               if v._discriminating(x) and v._pins_the_arithmetic(x)
               and not v._taxcalc_predicted(pol, x, 2025)[1]]
    assert len(vectors) >= 12, (
        f"only {len(vectors)} vector(s) tell the stepped branch from the smooth one — the kill below "
        f"would pass by comparing nothing. `_discriminating`, `_pins_the_arithmetic` and taxcalc's "
        f"own disqualification are the filters; a suite that lost them measures the floor at zero, "
        f"not the arithmetic"
    )
    stepped = v._taxcalc_applied(pol, vectors, 2025)
    smoothed = _smooth_path_calculator(
        [{"RECID": n + 1, "FLPDYR": 2025, "MARS": x["mars"], "e00200": float(x["magi"]),
          "e00200p": float(x["magi"]), "e00200s": 0.0,
          "age_head": 70 if x["seniors"] >= 1 else 40,
          "age_spouse": 70 if x["seniors"] >= 2 else 40,
          "tip_income": 0.0, "overtime_income": 0.0, "auto_loan_interest": 0.0, "s006": 1.0,
          v.FORM_1A[x["part"]]["taxcalc"][0]: float(v.FORM_1A[x["part"]]["claimed"])}
         for n, x in enumerate(vectors)],
        2025,
    )
    moved = 0
    for n, vec in enumerate(vectors):
        want_smooth = v._smooth_fallback(vec)
        got_broken = decimal.Decimal(
            str(round(float(smoothed.array(v.FORM_1A[vec["part"]]["taxcalc"][1])[n]), 2))
        )
        # The broken path MUST return the smooth value — otherwise this comparison proves nothing
        # about the branch and the "stepped" leg below could be passing for any other reason.
        assert abs(got_broken - want_smooth) < tol, (
            f"{vec['part']}/{vec['status']}/{vec['offset']}: the reconstructed broken path returned "
            f"{got_broken}, not the smooth {want_smooth} — this kill has stopped measuring the branch"
        )
        assert abs(stepped[n] - want_smooth) >= tol, (
            f"{vec['part']}/{vec['status']}/{vec['offset']}: taxcalc returned the SMOOTH value "
            f"{stepped[n]} through build_calculator — the stepped branch never ran, which is FR-124"
        )
        moved += 1
    assert moved == len(vectors), (moved, len(vectors))

    print(
        f"taxcalc_exact: `exact` sticks in both directions, an input column is refused by name, and "
        f"{moved} fractional-step vector(s) move off the smooth fallback (taxcalc {tc.__version__}) "
        f"— B1 kills OK"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--selftest", action="store_true",
                    help="B1 kills: watch `exact` stick, watch an input column be refused, and "
                         "watch the stepped branch move off the smooth fallback")
    args = ap.parse_args()
    if args.selftest:
        return selftest()
    ap.error("nothing to do — this module is imported by the oracle scripts; try --selftest")
    return 2  # pragma: no cover — argparse.error exits


if __name__ == "__main__":
    sys.exit(main())
