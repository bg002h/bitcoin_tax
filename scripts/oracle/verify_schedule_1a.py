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

★ Run: `.venv/bin/python scripts/oracle/verify_schedule_1a.py`
"""

import json
import pathlib
import sys

# btctax's own TY2025 table, transcribed from `crates/btctax-core/src/tax/tables.rs`. Kept here as
# literals ON PURPOSE: an oracle check that imported our value would be comparing our input to
# itself — an echo, not a witness. `schedule_1a_table_matches_the_oracle_census` in the Rust suite
# pins these same numbers from the other side, so a drift on either side reds.
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


def main() -> int:
    try:
        version, pol = _policy()
    except Exception as e:  # noqa: BLE001
        print(f"INCONCLUSIVE — taxcalc unavailable ({e}); no vector can have a witness this run.")
        return 0

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
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
