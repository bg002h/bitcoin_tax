#!/usr/bin/env python
"""Schedule 1-A (TY2025) — the PER-PART two-oracle census.

★★★ WHY THIS IS PER PART, AND WHY DISQUALIFICATIONS ARE COMPUTED.

Schedule 1-A is four independent deductions on one page, and the oracles do not cover them
uniformly. Printing one "OK" for the schedule would hide that: the standing failure on record is the
MFS AMT case, where two SEPARATELY-disqualified oracles agreed, three vectors had no witness at all,
and the run printed "OK" twice while that was true. Only a per-vector witness COUNT found it.

So this census asks, per part: **how many independent engines actually witnessed it?** And a
disqualification is derived from the defect's own MECHANISM — here, whether the engine models the
provision at all — never from a hand-written list of vector names. Both excuse lists this project
ever keyed by name went stale on the first batch of new vectors.

★★ TWO CENSUSES LIVE HERE, and the second is the one with teeth.

  1. the PER-PART CONSTANTS census — every printed cap, threshold, step and per-step rate against
     Tax-Calculator's own parameters. It passes with 0 divergences and has always passed.
  2. the APPLIED-RESULT census — the STEP ARITHMETIC of lines 10-12 / 18-20 / 27-29, which no
     comparison of constants can reach, because the rounding DIRECTION is not a constant. See the big
     banner comment further down for why a flipped direction is a full $100/$200 step for every filer
     in the band while every constant still agrees.

★ Run: `OTS_DIR=~/OpenTaxSolver2025_23.06_linux64 OTS_YEAR=2025 .venv/bin/python
  scripts/oracle/verify_schedule_1a.py`. Without `OTS_DIR` the applied census still runs and says
  loudly that it rests on ONE oracle — a missing oracle is a gap in coverage, never a pass.
"""

import decimal
import json
import os
import pathlib
import sys

# btctax's own TY2025 table, transcribed from `crates/btctax-core/src/tax/tables.rs`. Kept here as
# literals ON PURPOSE: an oracle check that imported our value would be comparing our input to
# itself — an echo, not a witness.
#
# ★ The Rust side pins the same numbers from the other direction, so a drift on either side reds:
# `tables.rs::schedule_1a_tests::every_constant_matches_its_printed_line` for the constants and
# `tests/schedule_1a_compute.rs::the_step_arithmetic_matches_the_oracle_census` for the applied
# results below. (This comment previously named `schedule_1a_table_matches_the_oracle_census`, a test
# that has never existed in the suite — a citation nobody could follow, and the exact shape of defect
# the cite-check harness exists for. Measured with `grep -rn --include='*.rs'`: zero matches.)
BTCTAX = {
    "tips_cap": 25000,
    "tips_ps_base": 150000,
    "tips_ps_mfj": 300000,
    "tips_step": 1000,
    "tips_per_step": 100,
    "overtime_cap_base": 12500,
    "overtime_cap_mfj": 25000,
    "overtime_ps_base": 150000,
    "overtime_ps_mfj": 300000,
    "overtime_step": 1000,
    "overtime_per_step": 100,
    "qpvli_cap": 10000,
    "qpvli_ps_base": 100000,
    "qpvli_ps_mfj": 200000,
    "qpvli_step": 1000,
    "qpvli_per_step": 200,
    "senior_per_person": 6000,
    "senior_ps_base": 75000,
    "senior_ps_mfj": 150000,
    "senior_rate": 0.06,
}


def _policy():
    import taxcalc

    path = pathlib.Path(taxcalc.__file__).parent / "policy_current_law.json"
    return taxcalc.__version__, json.loads(path.read_text())


def _rows(pol, name, year=2025):
    """The latest rows at or before `year`, or None if the parameter does not exist at all.

    ★ `None` is the DISQUALIFICATION, and it is computed: an engine that has no parameter for a
    provision cannot witness it. That is the whole mechanism for Part IV.
    """
    if name not in pol:
        return None
    cand = [r for r in pol[name]["value"] if r.get("year", 0) <= year]
    if not cand:
        return None
    latest = max(r["year"] for r in cand)
    return [r for r in cand if r["year"] == latest]


def _by_mars(rows):
    return {r.get("MARS", "*"): r["value"] for r in rows} if rows else {}


def _parameter_census(version: str, pol: dict) -> int:
    print(f"── Schedule 1-A (TY2025) per-part witness census · taxcalc {version} ──\n")

    # (part, our-key-prefix, taxcalc parameter names). Absence of a name ⇒ computed disqualification.
    PARTS = [
        ("Part II  — qualified tips (§224)",
         {"TipIncomeDed_c": ("tips_cap", None),
          "TipIncomeDed_ps": ("tips_ps_base", "tips_ps_mfj"),
          "TipIncomeDed_po_step_size": ("tips_step", None),
          "TipIncomeDed_po_rate_per_step": ("tips_rate", None)}),
        ("Part III — qualified overtime (§225)",
         {"OvertimeIncomeDed_c": ("overtime_cap_base", "overtime_cap_mfj"),
          "OvertimeIncomeDed_ps": ("overtime_ps_base", "overtime_ps_mfj"),
          "OvertimeIncomeDed_po_step_size": ("overtime_step", None),
          "OvertimeIncomeDed_po_rate_per_step": ("overtime_rate", None)}),
        ("Part IV  — car loan interest (§163(h)(4))",
         {"AutoLoanInterestDed_c": ("qpvli_cap", None),
          "AutoLoanInterestDed_ps": ("qpvli_ps_base", "qpvli_ps_mfj"),
          "AutoLoanInterestDed_po_step_size": ("qpvli_step", None),
          "AutoLoanInterestDed_po_rate_per_step": ("qpvli_rate", None)}),
        ("Part V   — seniors (§151(d)(5))",
         {"SeniorDed_c": ("senior_per_person", None),
          "SeniorDed_ps": ("senior_ps_base", "senior_ps_mfj"),
          "SeniorDed_prt": ("senior_rate", None)}),
    ]

    bad = 0
    zero_oracle_parts = []
    disqualified: list[str] = []
    for title, params in PARTS:
        print(title)
        missing = [n for n in params if _rows(pol, n) is None]
        if len(missing) == len(params):
            zero_oracle_parts.append(title)
            print(f"    witnesses: 0  — taxcalc models NONE of {sorted(params)}")
            print("    ⇒ ZERO-ORACLE. Adjudicated against the FORM, and the citation is in the code.\n")
            continue
        for name, (base_key, mfj_key) in params.items():
            rows = _rows(pol, name)
            if rows is None:
                print(f"    {name:36} ABSENT — cannot witness")
                continue
            vals = _by_mars(rows)
            # rate-vs-dollars: taxcalc stores a RATE per step, btctax stores DOLLARS per step.
            if name.endswith("po_rate_per_step"):
                step = _by_mars(_rows(pol, name.replace("rate_per_step", "step_size")))["*"]
                ours = BTCTAX[base_key.replace("_rate", "_per_step")]
                theirs = vals["*"] * step
                ok = abs(theirs - ours) < 1e-9
                print(f"    {name:36} {theirs:>10,.0f}/step  vs ours {ours:>10,.0f}  {'OK' if ok else '** DIVERGES **'}")
                bad += 0 if ok else 1
                continue
            base = vals.get("single", vals.get("*"))
            ours_base = BTCTAX[base_key]
            ok = abs(float(base) - ours_base) < 1e-9
            # ★ FOUR decimals, not zero. A first draft printed `SeniorDed_prt` (0.06) as "0 vs 0":
            #   the comparison was right and the DISPLAY could not have shown a divergence, which is
            #   this repo's green-and-blind shape inside the very instrument meant to catch it.
            print(f"    {name:36} base {float(base):>12,.4f}  vs ours {float(ours_base):>12,.4f}  {'OK' if ok else '** DIVERGES **'}")
            bad += 0 if ok else 1
            if mfj_key and "mjoint" in vals:
                mj, ours_mj = float(vals["mjoint"]), BTCTAX[mfj_key]
                ok = abs(mj - ours_mj) < 1e-9
                print(f"    {'':36} MFJ  {mj:>12,.4f}  vs ours {float(ours_mj):>12,.4f}  {'OK' if ok else '** DIVERGES **'}")
                bad += 0 if ok else 1
                # ★ QSS is NOT MFJ. The form says "Married filing jointly—$X. All other filing
                #   statuses—$Y", so a QSS threshold equal to the MFJ one is an engine defect.
                qss = float(vals.get("widow", base))
                if abs(qss - float(base)) > 1e-9:
                    print(f"    {'':36} ** QSS takes {qss:,.0f}, not the base {float(base):,.0f} —")
                    print(f"    {'':36}    ENGINE DEFECT: the form says \"Married filing jointly—"
                          f"{ours_mj:,.0f}. All other filing statuses—{ours_base:,.0f}\", and a")
                    print(f"    {'':36}    qualifying surviving spouse is NOT married filing jointly.")
                    print(f"    {'':36}    ⇒ taxcalc is DISQUALIFIED as a witness for QSS on this part.")
                    disqualified.append(f"{title} · QSS (taxcalc threshold {qss:,.0f} ≠ form {float(base):,.0f})")
        print()

    print("── verdict ──")
    for d in disqualified:
        print(f"  DISQUALIFIED WITNESS: {d}")
    if disqualified:
        print("  ★ Computed from the parameter itself, not read off a list of vector names. Both")
        print("    excuse lists this project ever keyed by NAME went stale on the first new vectors.")
    if zero_oracle_parts:
        for t in zero_oracle_parts:
            print(f"  ZERO-ORACLE: {t}")
        print("  ★ Stated per part rather than hidden behind a schedule-level OK. A part with no")
        print("    witness is not 'passing'; it is unwitnessed, and the form is the authority.")
    print(f"  divergences: {bad}")
    return bad


# ══════════════════════════════════════════════════════════════════════════════════════════════════
# THE APPLIED-RESULT CENSUS — the STEP ARITHMETIC, which the constants census above cannot reach
# ══════════════════════════════════════════════════════════════════════════════════════════════════
#
# ★★★ WHY A SECOND CENSUS. Everything above compares CONSTANTS, and every constant agrees today. The
# arithmetic that applies them does not follow from them: lines 11/19/28 all read *divide, round,
# multiply*, and the ROUNDING DIRECTION is per part —
#
#   line 11 (tips)     "decrease the result to the next lower whole number  … decrease 0.05 to 0"
#   line 19 (overtime) "decrease the result to the next lower whole number  … decrease 0.05 to 0"
#   line 28 (car loan) "increase the result to the next higher whole number … increase 0.05 to 1"
#
# A flipped direction is a FULL STEP: $100 on Parts II/III, $200 on Part IV, for every filer in the
# band — with every constant still agreeing and the census above still printing 0 divergences. That is
# the defect this section exists to make impossible, and `StepRounding`'s own doc comment in
# `tables.rs` calls it "the field that must never be shared".
#
# ★★ TY2026 IS WHAT THIS PINS. `SCHEDULE_1A_YEARS` is 2025..=2028 and NOTHING on the form is indexed,
# so the arithmetic checked here is the arithmetic TY2026 files with — independently of
# `FullReturnParams`, which has no TY2025 entry. taxcalc carries the same window (rows at 2025 and
# 2029, nothing between), so this census runs every year of it and the expectations never move. The
# TY2026 DRAFT form RENUMBERS the lines (the tips cap moves from line 7 to line 9, the divide from 11
# to 13) while every constant and both rounding directions are unchanged — measured against
# `design/forms/extract/f1040s1a--2026-DRAFT.txt`. So the arithmetic transfers; only the numbering
# moves, which is why a transcription struct is per line-set revision and this census is not.
#
# ★ Everything below is computed in PYTHON from the form's own words. Never from btctax: an oracle
# check that imported our value would be comparing our input to itself — an echo, not a witness.
# The Rust side pins the identical vectors as literals in
# `schedule_1a_compute.rs::the_step_arithmetic_matches_the_oracle_census`, so a drift on either side
# reds.

# The window `tables.rs::SCHEDULE_1A_YEARS` prints, transcribed (see the echo rule above). The census
# runs every year in it, plus the first year after it, where the four provisions have expired.
WINDOW = range(2025, 2029)

# Schedule 1-A, TY2025, transcribed from the TEXT LAYER of `design/forms/extract/f1040s1a--2025.txt`
# — never from the rendered page (a rendered `12` and `22` differ by a few pixels).
#
# `direction` is the word the form prints, and `instruction` is the sentence it comes from, so a
# reviewer can check the direction against the quote without opening the PDF.
FORM_1A = {
    "II": {
        "title": "Part II  — qualified tips (§224), lines 7-13",
        "cap": {"*": 25000},  # L7 "Enter the smaller of the amount on line 6 or $25,000" — no MFJ figure
        "threshold": {"*": 150000, "mjoint": 300000},  # L9 "$150,000 ($300,000 if married filing jointly)"
        "step": 1000,  # L11 "Divide line 10 by $1,000"
        "per_step": 100,  # L12 "Multiply line 11 by $100"
        "direction": "floor",
        "instruction": "decrease the result to the next lower whole number. (For example, decrease "
                       "1.5 to 1, and decrease 0.05 to 0.)",
        # "If married, you must file jointly to claim this deduction." (Part II Caution)
        "mfs_barred": True,
        "claimed": 40000,  # over the cap on purpose, so the CAP binds and the phase-out is what moves
        "taxcalc": ("tip_income", "tip_income_deduction"),
        "ots_input": {"S1A_4a": None},  # L4a "qualified tips included on Form W-2, box 7"
        "ots_deduction": "S1A_13",
        "ots_ran": "S1A_9",  # the printed threshold: always nonzero when OTS computed this part
    },
    "III": {
        "title": "Part III — qualified overtime (§225), lines 15-21",
        # L15 "the smaller of the amount on line 14c or $12,500 ($25,000 if married filing jointly)" —
        # unlike the tips cap, THIS one doubles.
        "cap": {"*": 12500, "mjoint": 25000},
        "threshold": {"*": 150000, "mjoint": 300000},  # L17
        "step": 1000,  # L19
        "per_step": 100,  # L20
        "direction": "floor",
        "instruction": "decrease the result to the next lower whole number. (For example, decrease "
                       "1.5 to 1, and decrease 0.05 to 0.)",
        "mfs_barred": True,
        "claimed": 30000,
        "taxcalc": ("overtime_income", "overtime_income_deduction"),
        "ots_input": {"S1A_14a": None},
        "ots_deduction": "S1A_21",
        "ots_ran": "S1A_17",
    },
    "IV": {
        "title": "Part IV  — car loan interest (§163(h)(4)), lines 24-30",
        "cap": {"*": 10000},  # L24 "the smaller of the amount on line 23 or $10,000"
        "threshold": {"*": 100000, "mjoint": 200000},  # L26 "$100,000 ($200,000 if married filing jointly)"
        "step": 1000,  # L28 "Divide line 27 by $1,000"
        "per_step": 200,  # L29 "Multiply line 28 by $200"
        "direction": "ceil",
        "instruction": "increase the result to the next higher whole number. (For example, increase "
                       "1.5 to 2, and increase 0.05 to 1.)",
        # ★ Part IV prints NO "must file jointly" caution, so it is ALLOWED for MFS — adjudicated
        #   against the FORM over an oracle that bars it (OTS does; taxcalc does not).
        "mfs_barred": False,
        "claimed": 12000,
        "taxcalc": ("auto_loan_interest", "auto_loan_interest_deduction"),
        "ots_input": {"S1A_22a": "1HGCM82633A004352", "S1A_22aiii": None},
        "ots_deduction": "S1A_30",
        "ots_ran": "S1A_26",
    },
}

# Part V is NOT in FORM_1A because it has NO STEP AT ALL. §151(d)(5)(C) / line 34 is a flat 6%, and
# `StairStepPhaseOut`'s own comment states the trap: "giving it a fake step is how a smooth phase-out
# acquires a stair". It is censused separately, with vectors chosen so that a planted stair diverges.
FORM_1A_PART_V = {
    "title": "Part V   — seniors (§151(d)(5)), lines 31-37",
    "per_person": 6000,  # L33's jump constant and L35's minuend
    "threshold": {"*": 75000, "mjoint": 150000},  # L32 "$75,000 ($150,000 if married filing jointly)"
    "rate": decimal.Decimal("0.06"),  # L34 "Multiply line 33 by 6% (0.06)"
    "mfs_barred": True,
    "ots_deduction": "S1A_37",
    "ots_ran": "S1A_32",
}

# our name → (taxcalc MARS code, taxcalc policy MARS label, OTS status token).
# ★ The OTS tokens are matched by `strncasecmp` in taxsolve_US_1040_2025.c; a token OTS does not
#   recognise yields ALL ZEROS rather than an error, which reads exactly like a broken install.
STATUSES = {
    "single": (1, "single", "Single"),
    "mfj": (2, "mjoint", "Married/Joint"),
    "mfs": (3, "mseparate", "Married/Sep"),
    "hoh": (4, "headhh", "Head_of_House"),
    "qss": (5, "widow", "Widow(er)"),
}


def _pick(table: dict, mars_label: str):
    """A form figure for one filing status: the MFJ parenthetical if the line prints one, else base."""
    return table.get(mars_label, table["*"])


def _round_dollar(v: decimal.Decimal) -> decimal.Decimal:
    """The IRS whole-dollar convention — "drop under 50¢, round 50–99¢ up" (i1040 p.23), i.e. HALF UP.

    Used for line 34 only, which is its own printed dollar line. Exact decimal arithmetic on purpose:
    0.06 is not representable in binary, and the case that matters is exactly the half-dollar.
    """
    return v.quantize(decimal.Decimal(1), rounding=decimal.ROUND_HALF_UP)


def _steps(excess: int, step: int, direction: str) -> int:
    """Lines 11 / 19 / 28. The direction is the form's printed word, never a shared default."""
    if direction == "floor":
        return excess // step
    if direction == "ceil":
        return -(-excess // step)
    raise ValueError(f"unknown rounding direction {direction!r}")


def _exhaustion_excess(cap: int, step: int, per_step: int, direction: str) -> int:
    """The excess at which the deduction FIRST reaches zero — and it is per-direction.

    A flooring part exhausts exactly at (cap / per_step) × step. A CEILING part exhausts one dollar
    PAST the last full step, because any portion of a step counts as a whole one: Part IV with the
    $10,000 cap exhausts at $49,001, not $50,000 — at an excess of $49,000 line 28 is 49, line 29 is
    $9,800 and line 30 still stands at $200.
    """
    full = cap // per_step
    return full * step if direction == "floor" else (full - 1) * step + 1


def _stepped_offsets(cap: int, step: int, per_step: int, direction: str) -> list[tuple[str, int]]:
    """The MAGI offsets above the threshold that straddle this part's arithmetic — DERIVED.

    ★ Derived from the parameters, not typed: a hand-written list of MAGIs is the shape this project
    keeps getting burned by, because a later change to the cap or the step moves every boundary and
    the list does not know. Two of the six are the FORM'S OWN worked examples (0.05 of a step and
    1.5 steps), which is why they are the two that tell the directions apart.
    """
    exhaust = _exhaustion_excess(cap, step, per_step, direction)
    return [
        ("at the threshold (the form's JUMP)", 0),
        ("+$1 over", 1),
        (f"+0.05 step (${step // 20:,})", step // 20),
        (f"+1.5 steps (${step + step // 2:,})", step + step // 2),
        ("$1 before exhaustion", exhaust - 1),
        ("at exhaustion (deduction first 0)", exhaust),
    ]


def _smooth_offsets(per_person: int, rate: decimal.Decimal, step: int) -> list[tuple[str, int]]:
    """Part V's offsets — chosen so a PLANTED STAIR cannot hide, and derived from the rate.

    A fake stair of `step`/(rate × step) has the same average slope as the true 6% line and agrees
    with it at every multiple of `step`. So the probes that matter are the ones INSIDE a step, plus
    the half-dollar case where line 34's whole-dollar rounding is the only thing that moves.
    """
    exhaust = int(per_person / rate)  # 6,000 / 0.06 = 100,000
    half = step // 2
    return [
        ("at the threshold (jump writes $6,000)", 0),
        ("+$1 over", 1),
        ("+$25 (0.06 × excess on a half-dollar)", 25),
        ("+$50 (0.06 × excess a whole dollar)", 50),
        (f"+half a ${step:,} step (a stair reads 0)", half),
        (f"+1.5 × ${step:,} (a stair reads 1)", step + half),
        ("$1 before exhaustion", exhaust - 1),
        ("at exhaustion (line 35 first 0)", exhaust),
    ]


def _vectors() -> list[dict]:
    """Every boundary vector, DERIVED from the parameters × the filing statuses × the offsets.

    Nothing here is hand-typed: the MAGIs come from each status's own printed threshold plus the
    derived offsets, and the claimed amount is deliberately over the cap so the CAP binds and the
    phase-out is the only thing that moves.
    """
    out: list[dict] = []
    for part, f in FORM_1A.items():
        for st, (mars, label, token) in STATUSES.items():
            cap = _pick(f["cap"], label)
            thr = _pick(f["threshold"], label)
            assert f["claimed"] > cap, (
                f"Part {part}: claimed {f['claimed']:,} must exceed the {cap:,} cap or the cap stops "
                f"binding and these vectors measure the CLAIM instead of the phase-out"
            )
            for name, off in _stepped_offsets(cap, f["step"], f["per_step"], f["direction"]):
                out.append({
                    "part": part, "status": st, "mars": mars, "label": label, "token": token,
                    "offset": name, "magi": thr + off, "excess": off, "cap": cap, "threshold": thr,
                    "seniors": 0,
                })
    v = FORM_1A_PART_V
    step = FORM_1A["II"]["step"]  # the $1,000 a planted stair would plausibly use — derived, not typed
    for st, (mars, label, token) in STATUSES.items():
        thr = _pick(v["threshold"], label)
        # L36b — "If you are married filing jointly, your spouse … enter the amount from line 35", so
        # two seniors is reachable for MFJ only. Derived from the line, not from a list of cases.
        for seniors in ((1, 2) if st == "mfj" else (1,)):
            for name, off in _smooth_offsets(v["per_person"], v["rate"], step):
                out.append({
                    "part": "V", "status": st, "mars": mars, "label": label, "token": token,
                    "offset": name, "magi": thr + off, "excess": off, "cap": v["per_person"],
                    "threshold": thr, "seniors": seniors,
                })
    return out


def _deduction(
    vec: dict,
    *,
    cap: decimal.Decimal,
    threshold: decimal.Decimal,
    barred: bool,
    direction: str | None = None,
    step: int | None = None,
    per_step: decimal.Decimal | None = None,
    rate: decimal.Decimal | None = None,
    round_line_34: bool = True,
) -> decimal.Decimal:
    """The form's arithmetic for one part — parameterised so an ENGINE's own figures can be substituted.

    ★★★ ONE IMPLEMENTATION, THREE CALLERS: the FORM (its printed constants, its printed rounding
    direction, its printed dollar line 34), Tax-Calculator (its policy parameters and its UNROUNDED
    line 34) and OpenTaxSolver (its floored line 28, its $300 line 29, its unrounded line 34).

    ★★ And this shape was found by WRITING THE KILL, not by design. The first draft computed the
    form's expectation and each engine's prediction in separate functions, so a planted fake stair in
    Part V left every engine prediction correct, the run stayed GREEN, and only the witness census went
    to nonsense — a planted defect this census could not see. Two implementations of one arithmetic are
    two chances to be wrong, and only one of them was being mutated.

    ★ The comparison surface is the AMOUNT that reaches line 38, which is 0 both when the part is
    barred and when the phase-out exhausts it. Blank-versus-zero is a real distinction and it is NOT
    this census's job: the `Option<Usd>` leaves in `schedule_1a.rs` hold it, because "blank because the
    inputs say so" and "blank because nothing populated it" are identical on the printed page.
    """
    if barred:
        return decimal.Decimal(0)
    excess = decimal.Decimal(vec["magi"]) - threshold
    if vec["part"] == "V":
        if excess <= 0:  # L33 "If zero or less, enter $6,000 on line 35" — a NONZERO constant
            per_person = cap
        else:
            l34 = rate * excess  # L34 "Multiply line 33 by 6% (0.06)"
            if round_line_34:  # L34 is its own printed dollar line, so the form rounds it
                l34 = _round_dollar(l34)
            per_person = max(decimal.Decimal(0), cap - l34)  # L35
        return per_person * vec["seniors"]  # L36a + L36b = L37
    if excess <= 0:  # "If zero or less, enter the amount from line N on line M" — the JUMP
        return cap
    reduction = _steps(int(excess), step, direction) * per_step
    return max(decimal.Decimal(0), cap - reduction)


def _form_expected(vec: dict) -> decimal.Decimal:
    """What the FORM says reaches line 38 from this part, from the form's own printed figures."""
    if vec["part"] == "V":
        v = FORM_1A_PART_V
        return _deduction(
            vec,
            cap=decimal.Decimal(v["per_person"]),
            threshold=decimal.Decimal(vec["threshold"]),
            barred=v["mfs_barred"] and vec["status"] == "mfs",
            rate=v["rate"],
        )
    f = FORM_1A[vec["part"]]
    return _deduction(
        vec,
        cap=decimal.Decimal(vec["cap"]),
        threshold=decimal.Decimal(vec["threshold"]),
        barred=f["mfs_barred"] and vec["status"] == "mfs",
        direction=f["direction"],
        step=f["step"],
        per_step=decimal.Decimal(f["per_step"]),
    )


def _smooth_fallback(vec: dict) -> decimal.Decimal:
    """What a SMOOTHED (unstepped) phase-out would give — the value that must NOT appear.

    ★★ This is the instrument's own guard, and it is here because the failure it catches is real and
    in this repo: Tax-Calculator's stepped branch runs only when its `exact` variable is 1, and
    `exact` is a CALCULATED variable — passing `exact: 1` in the Records DataFrame is silently
    ignored, which leaves the engine on its smooth marginal-rate fallback. A census that compared a
    smooth number against a stepped expectation would diverge everywhere; worse, one that compared
    two smooth numbers would agree everywhere and witness nothing about the rounding direction.
    """
    if vec["part"] == "V":
        return decimal.Decimal(-1)  # Part V has no stepped branch at all; nothing to distinguish
    f = FORM_1A[vec["part"]]
    cap, excess = decimal.Decimal(vec["cap"]), vec["excess"]
    if excess <= 0:
        return cap
    rate = decimal.Decimal(f["per_step"]) / f["step"]
    return max(decimal.Decimal(0), cap - rate * excess)


# ── Tax-Calculator: the parameters it actually holds, and what they PREDICT ────────────────────────
TC_PARAMS = {
    "II": {"cap": "TipIncomeDed_c", "ps": "TipIncomeDed_ps",
           "step": "TipIncomeDed_po_step_size", "rate": "TipIncomeDed_po_rate_per_step"},
    "III": {"cap": "OvertimeIncomeDed_c", "ps": "OvertimeIncomeDed_ps",
            "step": "OvertimeIncomeDed_po_step_size", "rate": "OvertimeIncomeDed_po_rate_per_step"},
    "IV": {"cap": "AutoLoanInterestDed_c", "ps": "AutoLoanInterestDed_ps",
           "step": "AutoLoanInterestDed_po_step_size", "rate": "AutoLoanInterestDed_po_rate_per_step"},
    "V": {"cap": "SeniorDed_c", "ps": "SeniorDed_ps", "rate": "SeniorDed_prt"},
}


def _tc_value(pol, name: str, label: str, year: int):
    """One taxcalc parameter for one filing status, or None when the engine has no such parameter."""
    rows = _rows(pol, name, year)
    if rows is None:
        return None
    vals = _by_mars(rows)
    return vals.get(label, vals.get("*"))


def _taxcalc_predicted(pol, vec: dict, year: int) -> tuple[decimal.Decimal | None, list[str]]:
    """What taxcalc's OWN parameters predict for this vector, and why they differ from the form.

    ★★★ COMPUTED FROM THE PARAMETER, NOT FROM A LIST OF VECTOR NAMES. Both excuse lists this project
    ever keyed by name went stale on the first batch of new vectors. This reads the engine's own
    figure for THIS status, recomputes the form's arithmetic with it, and reports the mismatch — so it
    reaches any future parameter defect, not only the QSS one it was written against, and it stops
    excusing the moment upstream fixes it.
    """
    p, why = TC_PARAMS[vec["part"]], []
    tc_cap = _tc_value(pol, p["cap"], vec["label"], year)
    tc_thr = _tc_value(pol, p["ps"], vec["label"], year)
    if tc_cap is None or tc_thr is None:
        return None, [f"taxcalc has no parameter for Part {vec['part']} in TY{year}"]
    # ★★ WHETHER A MECHANISM DISQUALIFIES THIS VECTOR IS NOT DECIDED HERE. This function names every
    #    mechanism it finds and returns the number they predict; the caller compares that PREDICTION
    #    against the form's figure, and only a prediction that actually differs counts as a
    #    disqualification. So a defect that cannot move this vector's answer — a wrong parameter on a
    #    part this status is barred from, a wrong per-step rate where the phase-out never runs — leaves
    #    the vector WITNESSED, which is the truth: the engine does produce the form's number there.
    #    Counting such a vector as unwitnessed reads like caution and is really an understatement, and
    #    an understated witness census hides the figures that genuinely have nothing behind them.
    if vec["part"] == "V":
        barred = vec["status"] == "mfs"  # calcfunctions.py: `if SeniorDed_c > 0. and MARS != 3`
        tc_rate = decimal.Decimal(str(_tc_value(pol, p["rate"], vec["label"], year)))
        if abs(float(tc_cap) - vec["cap"]) > 1e-9:
            why.append(f"{p['cap']} is {float(tc_cap):,.0f}, the form prints {vec['cap']:,}")
        if abs(float(tc_thr) - vec["threshold"]) > 1e-9:
            why.append(f"{p['ps']}[{vec['label']}] is {float(tc_thr):,.0f}, the form prints "
                       f"{vec['threshold']:,}")
        if tc_rate != FORM_1A_PART_V["rate"]:
            why.append(f"{p['rate']} is {tc_rate}, the form prints {FORM_1A_PART_V['rate']}")
        # ★ taxcalc does NOT round line 34: `po_amount = excess_agi * SeniorDed_prt` goes straight into
        #   `max(0., SeniorDed_c - po_amount)`. Line 34 is a printed dollar line, so the form rounds it.
        #   The gap is at most 50¢ per person and it is real — see the witness census, where BOTH
        #   engines carry the same omission and the printed figure ends up with no witness at all.
        excess = decimal.Decimal(vec["magi"]) - decimal.Decimal(str(tc_thr))
        if excess > 0 and tc_rate * excess != _round_dollar(tc_rate * excess):
            why.append(f"taxcalc's line 34 is the unrounded {tc_rate * excess} where the form's "
                       f"printed dollar line is {_round_dollar(tc_rate * excess)} (calcfunctions.py "
                       f"`po_amount = excess_agi * SeniorDed_prt`)")
        return _deduction(
            vec,
            cap=decimal.Decimal(str(tc_cap)),
            threshold=decimal.Decimal(str(tc_thr)),
            barred=barred,
            rate=tc_rate,
            round_line_34=False,
        ), why
    tc_step = decimal.Decimal(str(_tc_value(pol, p["step"], vec["label"], year)))
    tc_rate = decimal.Decimal(str(_tc_value(pol, p["rate"], vec["label"], year)))
    tc_per_step = tc_rate * tc_step  # taxcalc stores a RATE per step; the form prints DOLLARS
    f = FORM_1A[vec["part"]]
    barred = f["mfs_barred"] and vec["status"] == "mfs"
    if abs(float(tc_cap) - vec["cap"]) > 1e-9:
        why.append(f"{p['cap']}[{vec['label']}] is {float(tc_cap):,.0f}, the form prints {vec['cap']:,}")
    if abs(float(tc_thr) - vec["threshold"]) > 1e-9:
        why.append(f"{p['ps']}[{vec['label']}] is {float(tc_thr):,.0f}, the form prints "
                   f"{vec['threshold']:,} — the form says \"Married filing jointly—{_pick(f['threshold'], 'mjoint'):,}. "
                   f"All other filing statuses—{f['threshold']['*']:,}\"")
    if tc_per_step != f["per_step"] or tc_step != f["step"]:
        why.append(f"{p['step']}/{p['rate']} give ${tc_per_step:,.0f} per ${tc_step:,.0f}, the form "
                   f"prints ${f['per_step']:,} per ${f['step']:,}")
    # `calcfunctions.py` bars Parts II/III for `MARS == 3` and allows Part IV, exactly as the form does.
    return _deduction(
        vec,
        cap=min(decimal.Decimal(f["claimed"]), decimal.Decimal(str(tc_cap))),
        threshold=decimal.Decimal(str(tc_thr)),
        barred=f["mfs_barred"] and vec["status"] == "mfs",
        direction=f["direction"],
        step=int(tc_step),
        per_step=tc_per_step,
    ), why


# ★ NOTE ON WHAT IS DELIBERATELY *NOT* EXCUSED ABOVE: `_taxcalc_predicted` recomputes with taxcalc's
#   own CONSTANTS but always with the FORM'S rounding direction. The direction is the thing under
#   test, so an engine that flipped one must come out as an UNEXPECTED divergence rather than a
#   predicted one. Excusing it would be the green-and-blind shape inside the instrument built to
#   catch it.

# ── OpenTaxSolver 2025: its Schedule 1-A defects, MEASURED and re-verified every run ───────────────
#
# `~/OpenTaxSolver2025_23.06_linux64/src/taxsolve_US_1040_2025.c`, `sched_1A()` at :1783. Each flag
# below is a reading of that source AND is re-verified on every vector by reproducing OTS's own
# printed output exactly — so the day OTS fixes one, the reproduction fails and this run says so
# instead of going on excusing a defect that is gone. (Same design as `verify_f6251.py`'s sized gaps:
# "this vector must disagree by exactly this much, for this reason".)
#
#   :1888  `j = sched1A_L[27] / 1000.0;`   with `int j` — C truncation, i.e. FLOOR.  Line 28 prints
#          "increase the result to the next higher whole number", so Part IV is floored where the form
#          ceils. Parts II/III (:1845, :1870) floor, which is what lines 11 and 19 print. ✔
#   :1895  `sched1A_L[29] = 300.0 * sched1A_L[28];` — line 29 prints "Multiply line 28 by $200".
#   :1823  `if (status != MARRIED_FILING_SEPARAT)` wraps Parts II THROUGH V, so Part IV is barred for
#          MFS. Only Parts II/III/V print the "you must file jointly" caution.
#   :1909  `sched1A_L[34] = 0.06 * sched1A_L[33];` — no whole-dollar rounding of a printed dollar line.
#   :1894  prints `S1A_20` a second time in place of `S1A_29`, so OTS never emits line 29 at all. Not
#          a computation defect; it is why the per-step rate is checked by REPRODUCING line 30 rather
#          than by reading line 29.
OTS_2025_PART_IV_TRUNCATES = True
OTS_2025_PART_IV_PER_STEP = 300
OTS_2025_LINE_34_UNROUNDED = True
OTS_STEP_LINE = {"II": "S1A_11", "III": "S1A_19", "IV": "S1A_28"}


def _ots_predicted(vec: dict, parsed: dict) -> tuple[decimal.Decimal, list[str], list[str]]:
    """(what OTS's own mechanisms predict, why it differs from the form, hard failures).

    The third list is for things no mechanism explains — a printed threshold that is not the form's,
    or a printed step count the prediction did not reproduce. Those are findings, not excuses.
    """
    part = vec["part"]
    spec = FORM_1A_PART_V if part == "V" else FORM_1A[part]
    why: list[str] = []
    fail: list[str] = []
    if spec["ots_ran"] not in parsed:
        # `showline_wlabelnz` suppresses zeros, so absence of the printed THRESHOLD (never zero when
        # the part runs) is how "OTS did not compute this part" is told apart from "it computed 0".
        if _form_expected(vec) != 0:
            why.append("OTS computes Parts II-V only inside `if (status != MARRIED_FILING_SEPARAT)` "
                       "(:1823), so it never runs Part IV for MFS; the form's caution is on Parts "
                       "II/III/V only, so the FORM allows this deduction")
        return decimal.Decimal(0), why, fail
    printed_threshold = float(parsed[spec["ots_ran"]])
    if abs(printed_threshold - vec["threshold"]) > 0.005:
        fail.append(f"OTS printed threshold {printed_threshold:,.0f}, the form prints "
                    f"{vec['threshold']:,}")
    excess = vec["excess"]
    if part == "V":
        v = FORM_1A_PART_V
        exact = v["rate"] * excess
        if excess > 0 and OTS_2025_LINE_34_UNROUNDED and exact != _round_dollar(exact):
            why.append(f"OTS's line 34 is the unrounded {exact} where the form's printed dollar "
                       f"line is {_round_dollar(exact)} (:1909)")
        return _deduction(
            vec,
            cap=decimal.Decimal(v["per_person"]),
            threshold=decimal.Decimal(vec["threshold"]),
            barred=False,  # the part RAN (its printed threshold is above); MFS never reaches here
            rate=v["rate"],
            round_line_34=not OTS_2025_LINE_34_UNROUNDED,
        ), why, fail
    f, cap = FORM_1A[part], decimal.Decimal(vec["cap"])
    direction = "floor" if (part == "IV" and OTS_2025_PART_IV_TRUNCATES) else f["direction"]
    per_step = OTS_2025_PART_IV_PER_STEP if part == "IV" else f["per_step"]
    if direction != f["direction"]:
        why.append(f"OTS FLOORS line 28 (`int j`, :1888) where the form CEILS (the sentence is "
                   f"printed in this part's header above)")
    if per_step != f["per_step"]:
        why.append(f"OTS multiplies line 28 by ${per_step}, not the ${f['per_step']} line 29 prints "
                   f"(:1895)")
    if excess > 0:
        steps = _steps(excess, f["step"], direction)
        got_steps = parsed.get(OTS_STEP_LINE[part])
        if got_steps is not None and int(got_steps) != steps:
            fail.append(f"OTS printed {OTS_STEP_LINE[part]} = {int(got_steps)}; the mechanism above "
                        f"predicts {steps}")
    return _deduction(
        vec,
        cap=cap,
        threshold=decimal.Decimal(vec["threshold"]),
        barred=False,  # the part RAN; the not-run case returned above
        direction=direction,
        step=f["step"],
        per_step=decimal.Decimal(per_step),
    ), why, fail


# ── The passes ────────────────────────────────────────────────────────────────────────────────────
def _taxcalc_applied(pol, vectors, year: int):
    """Drive Tax-Calculator over every vector for `year`. Returns {index: value} or None if it cannot."""
    import numpy as np
    import pandas as pd
    import taxcalc as tc

    rows = []
    for n, v in enumerate(vectors):
        row = {"RECID": n + 1, "FLPDYR": year, "MARS": v["mars"],
               "e00200": float(v["magi"]), "e00200p": float(v["magi"]), "e00200s": 0.0,
               "age_head": 70 if v["seniors"] >= 1 else 40,
               "age_spouse": 70 if v["seniors"] >= 2 else 40,
               "tip_income": 0.0, "overtime_income": 0.0, "auto_loan_interest": 0.0, "s006": 1.0}
        if v["part"] != "V":
            row[FORM_1A[v["part"]]["taxcalc"][0]] = float(FORM_1A[v["part"]]["claimed"])
        rows.append(row)
    recs = tc.Records(data=pd.DataFrame(rows), start_year=year, gfactors=None, weights=None,
                      adjust_ratios=None)
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(year)
    # ★★★ `exact` is a CALCULATED variable, so a column named `exact` in the Records DataFrame is
    #     SILENTLY DROPPED — measured: it comes back all zeros and the engine stays on its smooth
    #     marginal-rate fallback, which is not the arithmetic any tax form performs. It has to be
    #     written through the Calculator. `_smooth_fallback` is the check that this actually took.
    calc.array("exact", np.ones(len(rows), dtype=np.int32))
    calc.calc_all()
    if int(calc.array("exact").sum()) != len(rows):
        raise RuntimeError("taxcalc's `exact` did not stick; every vector would use the SMOOTH branch")
    agi = calc.array("c00100")
    out = {}
    for n, v in enumerate(vectors):
        # The vector is only AT its boundary if the engine's MAGI is the MAGI we asked for. taxcalc
        # uses AGI as the Schedule 1-A line 3 proxy (calcfunctions.py: `magi = c00100`), as do we.
        if abs(float(agi[n]) - v["magi"]) > 0.005:
            raise RuntimeError(f"taxcalc AGI {agi[n]:,.2f} != the vector's MAGI {v['magi']:,} — this "
                               f"vector is not on the boundary it claims to be on")
        col = ("senior_deduction" if v["part"] == "V" else FORM_1A[v["part"]]["taxcalc"][1])
        out[n] = decimal.Decimal(str(round(float(calc.array(col)[n]), 2)))
    return out


def _ots_applied(vectors, year: int):
    """Drive OpenTaxSolver over every vector. Returns ({index: (value, why, fail)}, note)."""
    import shutil
    import tempfile

    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    import ots_direct as o  # noqa: E402 — deliberately late; needs OTS_DIR at import time

    solver = o.OTS_DIR / "bin" / f"taxsolve_US_1040_{year}"
    if not solver.exists():
        return None, (f"no OTS solver for TY{year} ({solver}), so OTS cannot witness this year — a "
                      f"YEAR-scoped disqualification, computed from the install rather than assumed")
    out = {}
    for n, v in enumerate(vectors):
        vals = {"Status": v["token"], "S1A_2a": 0, "L1a": float(v["magi"]),
                "You_65+Over?": "Y" if v["seniors"] >= 1 else "N",
                "Spouse_65+Over?": "Y" if v["seniors"] >= 2 else "N"}
        if v["part"] != "V":
            spec = dict(FORM_1A[v["part"]]["ots_input"])
            for k in spec:
                if spec[k] is None:
                    spec[k] = float(FORM_1A[v["part"]]["claimed"])
            vals |= spec
        work = pathlib.Path(tempfile.mkdtemp(prefix="ots-1a-"))
        try:
            parsed, _ = o.run_form("US_1040", "US_1040", "US_1040", vals, work, year=year)
        finally:
            shutil.rmtree(work, ignore_errors=True)
        pred, why, fail = _ots_predicted(v, parsed)
        if abs(float(parsed.get("S1A_1", -1)) - v["magi"]) > 0.005:
            fail.append(f"OTS line 1 is {parsed.get('S1A_1')}, not the vector's MAGI {v['magi']:,}")
        spec = FORM_1A_PART_V if v["part"] == "V" else FORM_1A[v["part"]]
        got = decimal.Decimal(str(parsed.get(spec["ots_deduction"], 0.0)))
        out[n] = (got, pred, why, fail)
    return out, None


def _transcription_tie() -> None:
    """`FORM_1A` and `BTCTAX` are two hand transcriptions of the same numbers; tie them together.

    ★ This proves only that the two dicts in THIS file agree — it is not a witness, and it is labelled
    so nobody mistakes it for one. Its value is narrow and real: the census above compares `BTCTAX`
    against the engines, the census below computes from `FORM_1A`, and a typo in either would make the
    two halves quietly disagree about what the form says.
    """
    same = [
        ("tips_cap", FORM_1A["II"]["cap"]["*"]), ("tips_ps_base", FORM_1A["II"]["threshold"]["*"]),
        ("tips_ps_mfj", FORM_1A["II"]["threshold"]["mjoint"]), ("tips_step", FORM_1A["II"]["step"]),
        ("tips_per_step", FORM_1A["II"]["per_step"]),
        ("overtime_cap_base", FORM_1A["III"]["cap"]["*"]),
        ("overtime_cap_mfj", FORM_1A["III"]["cap"]["mjoint"]),
        ("overtime_ps_base", FORM_1A["III"]["threshold"]["*"]),
        ("overtime_ps_mfj", FORM_1A["III"]["threshold"]["mjoint"]),
        ("overtime_step", FORM_1A["III"]["step"]), ("overtime_per_step", FORM_1A["III"]["per_step"]),
        ("qpvli_cap", FORM_1A["IV"]["cap"]["*"]), ("qpvli_ps_base", FORM_1A["IV"]["threshold"]["*"]),
        ("qpvli_ps_mfj", FORM_1A["IV"]["threshold"]["mjoint"]),
        ("qpvli_step", FORM_1A["IV"]["step"]), ("qpvli_per_step", FORM_1A["IV"]["per_step"]),
        ("senior_per_person", FORM_1A_PART_V["per_person"]),
        ("senior_ps_base", FORM_1A_PART_V["threshold"]["*"]),
        ("senior_ps_mfj", FORM_1A_PART_V["threshold"]["mjoint"]),
    ]
    for key, form_value in same:
        assert BTCTAX[key] == form_value, f"transcriptions disagree on {key}: {BTCTAX[key]} vs {form_value}"
    assert decimal.Decimal(str(BTCTAX["senior_rate"])) == FORM_1A_PART_V["rate"]


def _discriminating(vec: dict) -> bool:
    """Does this vector tell FLOOR and CEIL apart? True exactly when the excess is not a whole step.

    ★ Reported as a number, because it is the one thing that makes the rest of this census mean
    anything: a suite of vectors that all sit on step boundaries would agree under either direction
    and witness nothing about the field `StepRounding`'s doc comment calls "the field that must never
    be shared".
    """
    if vec["part"] == "V" or vec["excess"] <= 0:
        return False
    return vec["excess"] % FORM_1A[vec["part"]]["step"] != 0


def _pins_the_arithmetic(vec: dict) -> bool:
    """Does this vector exercise the PHASE-OUT itself, rather than the jump or the floor at zero?

    ★★ This is what the gate below is allowed to count, and the distinction is not pedantic — it was
    measured. The gate's first form asked only for "some witnessed vector in this part", and a planted
    mis-transcribed cap could not make it fire: `max(0, cap − reduction)` collapses to zero at
    exhaustion no matter how wrong the cap is, so the exhausted vector stays "witnessed" while every
    figure that depends on the arithmetic has stopped being witnessed. A gate nobody can watch go red
    is the instrument this repo distrusts most, so it was narrowed until it could be watched.
    """
    if vec["excess"] <= 0 or _form_expected(vec) <= 0:
        # ★ The `> 0` is the second half of the same lesson, and it took a second planted cap to find:
        #   at exhaustion the excess IS fractional, so "discriminating" was true, and both sides still
        #   read zero because `max(0, …)` swallows any error in the subtrahend. Requiring a live figure
        #   is what makes the gate answer the question it is asked.
        return False
    if vec["part"] == "V":
        return True  # inside the band, before line 35 bottoms out
    return _discriminating(vec)  # a fractional step, where the rounding direction decides


def _applied_census(pol, version: str) -> int:
    vectors = _vectors()
    _transcription_tie()
    years = list(WINDOW)
    print("\n" + "═" * 98)
    print(f"THE APPLIED-RESULT CENSUS — {len(vectors)} boundary vectors × TY{years[0]}-{years[-1]}")
    print("═" * 98)
    print(f"  {sum(_discriminating(v) for v in vectors)} of {len(vectors)} vectors sit at a FRACTIONAL "
          f"step, so they tell floor from ceil apart.")
    print(f"  vectors per part: " + ", ".join(
        f"{p}={sum(1 for v in vectors if v['part'] == p)}" for p in ("II", "III", "IV", "V")))

    try:
        tc_by_year = {y: _taxcalc_applied(pol, vectors, y) for y in years}
    except Exception as e:  # noqa: BLE001
        print(f"\n  INCONCLUSIVE — Tax-Calculator {version} could not run these vectors ({e}).")
        return 1

    ots_note, ots = None, None
    if not os.environ.get("OTS_DIR"):
        ots_note = ("OTS_DIR is unset, so every vector below rests on ONE oracle this run. Install: "
                    "https://sourceforge.net/projects/opentaxsolver/files/OTS_2025/")
    else:
        ots_by_year = {}
        for y in years:
            got, note = _ots_applied(vectors, y)
            if got is None:
                print(f"\n  OTS TY{y}: {note}")
            else:
                ots_by_year[y] = got
        ots = ots_by_year

    detail = years[0]
    bad = 0
    witnesses: dict[tuple[str, str], list[int]] = {}
    print(f"\n── per-part detail, TY{detail} (the other years are summarised below) ──")
    for part in ("II", "III", "IV", "V"):
        spec = FORM_1A_PART_V if part == "V" else FORM_1A[part]
        print(f"\n{spec['title']}")
        if part != "V":
            print(f"    line {'11' if part == 'II' else '19' if part == 'III' else '28'}: "
                  f"\"{spec['instruction']}\" ⇒ {spec['direction'].upper()}")
        print(f"    {'status':7}{'MAGI':>12}{'sen':>4}{'form':>11}{'taxcalc':>11}{'OTS':>11}  offset / verdict")
        for n, v in enumerate(vectors):
            if v["part"] != part:
                continue
            want = _form_expected(v)
            got_tc = tc_by_year[detail][n]
            pred_tc, why_tc = _taxcalc_predicted(pol, v, detail)
            notes = []
            # ★★★ TWO SEPARATE QUESTIONS, AND NEITHER EXCUSES THE OTHER.
            #   (a) did the engine do what its own code and parameters predict? If not, that is a
            #       finding — its behaviour is outside everything we know about it.
            #   (b) does that prediction differ from the FORM? If so this vector is disqualified, and
            #       the mechanism must SAY WHY. A prediction that differs from the form with no
            #       mechanism to explain it is the dangerous case: our list of mechanisms is
            #       incomplete, so it fails the run rather than being filed as a disqualification.
            tc_ok = pred_tc is not None and abs(got_tc - pred_tc) < decimal.Decimal("0.005")
            if not tc_ok:
                bad += 1
                notes.append(f"★ UNEXPECTED taxcalc: predicted {pred_tc}, got {got_tc}")
            tc_differs = pred_tc is None or abs(pred_tc - want) >= decimal.Decimal("0.005")
            if tc_differs and not why_tc:
                bad += 1
                notes.append(f"★ UNEXPLAINED: taxcalc's own figures predict {pred_tc} where the form "
                             f"gives {want}, and no known mechanism accounts for it")
            elif tc_differs:
                notes.append("taxcalc DISQUALIFIED — " + "; ".join(why_tc))
            tc_witness = tc_ok and not tc_differs
            # ★ The smooth-branch guard: on a fractional-step vector the stepped and smoothed answers
            #   differ, so an engine sitting on its smooth fallback is caught here rather than read as
            #   a defect in the form's arithmetic.
            if _discriminating(v) and abs(got_tc - _smooth_fallback(v)) < decimal.Decimal("0.005"):
                bad += 1
                notes.append("★ taxcalc returned the SMOOTH value — the stepped branch never ran")
            ots_cell, ots_witness = "  —", False
            if ots is not None and detail in ots:
                got_ots, pred_ots, why_ots, fail_ots = ots[detail][n]
                ots_cell = f"{got_ots:,.2f}"
                if fail_ots:
                    bad += len(fail_ots)
                    notes.extend("★ " + m for m in fail_ots)
                ots_ok = abs(got_ots - pred_ots) < decimal.Decimal("0.005")
                if not ots_ok:
                    bad += 1
                    notes.append(f"★ UNEXPECTED OTS: predicted {pred_ots}, got {got_ots}")
                ots_differs = abs(pred_ots - want) >= decimal.Decimal("0.005")
                if ots_differs and not why_ots:
                    bad += 1
                    notes.append(f"★ UNEXPLAINED: OTS's own mechanisms predict {pred_ots} where the "
                                 f"form gives {want}, and none of them accounts for it")
                elif ots_differs:
                    notes.append("OTS DISQUALIFIED — " + "; ".join(why_ots))
                ots_witness = ots_ok and not ots_differs and not fail_ots
            witnesses.setdefault((part, v["status"]), []).append(
                (int(tc_witness) + int(ots_witness), _pins_the_arithmetic(v))
            )
            mark = "" if not notes else "  " + " | ".join(notes)
            print(f"    {v['status']:7}{v['magi']:>12,}{v['seniors']:>4}{want:>11,.2f}"
                  f"{got_tc:>11,.2f}{ots_cell:>11}  {v['offset']}{mark}")

    print(f"\n── the other years in SCHEDULE_1A_YEARS — nothing on this form is indexed ──")
    for y in years:
        if y == detail:
            continue
        diffs = sum(1 for n in range(len(vectors))
                    if abs(tc_by_year[y][n] - tc_by_year[detail][n]) >= decimal.Decimal("0.005"))
        tag = "identical to TY{}".format(detail) if diffs == 0 else f"★ {diffs} vector(s) MOVED"
        bad += diffs
        print(f"  TY{y}: taxcalc reproduces all {len(vectors)} vectors — {tag}")
    print(f"  ★ TY{years[0]}-{years[-1]} is `tables.rs::SCHEDULE_1A_YEARS`. btctax computes Schedule "
          f"1-A for TY2026 TODAY, with no `FullReturnParams` involved, so this is the arithmetic")
    print(f"    TY2026 files with — which is why it is pinned now rather than in a TY2026 cycle.")

    # The sunset, from the other side of the window: the four provisions expire after the last year.
    after = years[-1] + 1
    try:
        sunset = _taxcalc_applied(pol, vectors, after)
        live = sum(1 for n in range(len(vectors)) if sunset[n] != 0)
        ok = "OK" if live == 0 else f"★ {live} vector(s) still deduct"
        bad += live
        print(f"  TY{after}: every vector deducts 0.00 — {ok}. §§224(f)/225(f)/163(h)(4)(F)/151(d)(5)(D) "
              f"expire after TY{years[-1]}, so taxcalc independently witnesses btctax's `None`.")
    except Exception as e:  # noqa: BLE001
        print(f"  TY{after}: INCONCLUSIVE ({e})")

    bad += _applied_witness_census(vectors, witnesses, ots_note, ots is not None and detail in (ots or {}))
    return bad


def _applied_witness_census(vectors, witnesses, ots_note, ots_ran: bool) -> int:
    """How many INDEPENDENT engines witnessed each part × status — and the parts with none.

    ★★ WHY THIS IS NOT A SCHEDULE-LEVEL "OK". Two separately-disqualified oracles can agree, and they
    do here: Part IV's stepped region has taxcalc giving QSS the MFJ threshold AND OTS both flooring a
    ceiling line and multiplying by $300. On a QSS car-loan vector in the band, NOTHING witnesses the
    figure — the form is the only authority — and a schedule-level OK would have hidden exactly that,
    which is the MFS-AMT failure this project already has on record.
    """
    print("\n── witness census, per part × filing status ──")
    if ots_note:
        print(f"  SKIPPED OTS — {ots_note}")
    unwitnessed = []
    for (part, status), counts in sorted(witnesses.items()):
        zero = sum(1 for c, _ in counts if c == 0)
        two = sum(1 for c, _ in counts if c >= 2)
        mark = "★" if zero else " "
        print(f"  {mark} Part {part:4}{status:7} {len(counts):>3} vectors: {two} with TWO independent "
              f"witnesses, {zero} with NONE")
        if zero:
            unwitnessed.append((part, status, zero, len(counts)))
    if not ots_ran:
        print("  ★ One oracle only, so no vector can have two witnesses this run.")
    for part, status, zero, total in unwitnessed:
        print(f"  ★ UNWITNESSED: Part {part} / {status} — {zero} of {total} vectors have no engine "
              f"behind them; adjudicated against the FORM, whose text is quoted in the code.")
    # ★ THE INSTRUMENT'S OWN GATE. A part with no witnessed vector anywhere is a part this census
    #   reports on without measuring, which is the green-and-blind shape it exists to prevent.
    blind = [
        p
        for p in ("II", "III", "IV", "V")
        if not any(
            c > 0 and pins for (part, _), cs in witnesses.items() if part == p for c, pins in cs
        )
    ]
    if blind:
        print(f"\n  FAIL: Part(s) {', '.join(blind)} have NO witnessed vector that exercises the "
              f"phase-out — this census would be reporting on arithmetic no engine confirmed.")
        return 1
    pinned = sum(1 for cs in witnesses.values() for c, pins in cs if pins and c > 0)
    total_pinned = sum(1 for cs in witnesses.values() for _, pins in cs if pins)
    print(f"\n  {pinned} of {total_pinned} phase-out-exercising vectors have at least one witness; "
          f"every part has one, so no part is reported on unmeasured.")
    # ★ Broken down BY PART, because the aggregate is the one number a reader would otherwise have to
    #   reason out — and a reasoned count is how a wrong figure gets written down (this file's own
    #   follow-up entry quoted the total for a per-part claim before this line existed).
    gaps = {}
    for (part, status), cs in witnesses.items():
        n = sum(1 for c, pins in cs if pins and c == 0)
        if n:
            gaps[part] = gaps.get(part, 0) + n
    if gaps:
        print("  unwitnessed, by part: " + ", ".join(f"{p}={n}" for p, n in sorted(gaps.items())))
    return 0


def main() -> int:
    try:
        version, pol = _policy()
    except Exception as e:  # noqa: BLE001
        print(f"INCONCLUSIVE — taxcalc unavailable ({e}); no vector can have a witness this run.")
        return 0
    bad = _parameter_census(version, pol)
    bad += _applied_census(pol, version)
    print(f"\n  {'FAIL' if bad else 'OK'}: {bad} unexpected divergence(s) across both censuses.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
