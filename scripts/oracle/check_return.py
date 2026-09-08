#!/usr/bin/env python3
"""★★★ **THE ORACLE PATH for a REAL return** — SPEC_interview.md R13, build task T11.

`sweep.py` and `gen_goldens.py` drive the two engines over the built-in CORPUS. This drives them over
**your own return**: `btctax income project --year N` prints the return as the household description
both engines take (a `GoldenInputs` row, which carries no identity by type), and this script runs
btctax's own §9 harness plus OpenTaxSolver plus PSL Tax-Calculator on that row and diffs the compared
lines. Run it before you export the packet.

    export OTS_DIR=/path/to/OpenTaxSolver2024_22.07_linux64
    cargo build -p btctax-oracle-harness
    btctax income project --year 2024 > /tmp/row.json
    .venv/bin/python scripts/oracle/check_return.py --file /tmp/row.json

Exit 0 = every compared line reconciles (or diverges by exactly its computed excuse). Exit 1 = a
divergence. Exit 2 = the run could not be made (no OTS, a refused return, a stale harness).

★ I4 (MANDATORY) — **this file re-implements NONE of btctax's arithmetic.** Every per-line verdict
  comes from the compiled harness's `--check` mode, which reproduces btctax's §3.1 printing, the Tax
  Table and the QDCGT worksheet in Rust. Python's built-in `round()` is banker's rounding and drifts
  from `round_dollar` on any `.50`. The only comparisons made here are the two credit lines the
  harness does not compare, and they are exact integer equalities.

★★★ **THE TWO EXCUSES, AND WHY THEY ARE COMPUTED RATHER THAN NAMED.** `CLAUDE.md`: *"state the
    mechanism, let it decide, never enumerate the outcomes you happened to see."* Both excuses below
    are read off the oracle's own output for THIS household, so a divergence of any other size is a
    failure even on a line that is expected to diverge:

    * **1040 line 19** — *"Child tax credit or credit for other dependents from Schedule 8812."*
      btctax has no Schedule 8812, so it prints the line BLANK (no cell at all — a blank is no
      testimony, §"An entry is testimony"). Tax-Calculator computes `c07220 + odc`. The expected gap
      is therefore exactly that figure.
    * **1040 line 27** — the earned income credit. btctax computes none; Tax-Calculator's `eitc` is
      the expected gap.

★★★ **AND OPENTAXSOLVER IS NOT A WITNESS ON EITHER.** `taxsolve_US_1040_2024.c` reads `L19`, `L27`
    and `L28` with `GetLine(...)` — they are INPUTS — and parses `Dependents` into `NumDependents` at
    line 1928 without ever using it. Its agreement with btctax on line 19 would be `0 == 0` between
    two engines neither of which computed anything, which is `FOLLOWUPS.md` §G-9 exactly: *a value
    the oracles take as INPUT is never validated by their agreement.* So the census below counts ONE
    witness on those lines and says so, rather than printing two OKs.

★★ **1040 line 24 is deliberately NOT compared against Tax-Calculator.** Its `c09200` is the
   liability AFTER nonrefundable credits, so a household with dependents makes it smaller than
   btctax's line 24 by `c07100` — and it also inherits the Tax-Table-versus-rate-schedule dissent on
   line 16. Two mechanisms in one number is not a check; the harness already declares taxcalc
   unwitnessed on line 24 and cross-foots it against OTS instead.

★ **What the row could NOT carry is reported too.** `GoldenInputs` is the corpus's household model
  and is smaller than the 1040: a filer with medical expenses, student-loan interest or a prior-year
  capital-loss carryforward is described to both engines WITHOUT them. `income project` prints those
  figures under `not_carried`, and they are echoed here — a divergence on such a line is the
  description's, not btctax's, and saying so is what keeps it attributable.

★ **A known non-excuse, stated so it is not mistaken for one.** If the filer never answered the
  §G-9 death question, btctax FORGOES the §63(f) age-65 addition (class (B): silence forgoes) while
  both engines grant it, and 1040 line 12 comes up short by a multiple of the year's addition. That
  is a real, actionable finding for the filer — answer the question and the deduction appears — so it
  is left to FAIL rather than excused.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

try:
    import ots_direct
except ImportError:  # pragma: no cover
    sys.exit("scripts/oracle/ots_direct.py must sit beside this script")

try:
    import gen_goldens
except ImportError:  # pragma: no cover
    sys.exit("scripts/oracle/gen_goldens.py must sit beside this script")

HARNESS_BIN = Path(__file__).resolve().parents[2] / "target" / "debug" / "btctax-oracle-harness"

# Why a compared line has only ONE engine behind it — the MECHANISM in each case, never a bare list.
SINGLE_WITNESS_REASON = {
    "1040.line19": "OpenTaxSolver reads L19 with `GetLine` — it is an INPUT — and parses "
    "`Dependents` into `NumDependents` without ever using it, so it computes no CTC/ODC (§G-9)",
    "1040.line27": "OpenTaxSolver reads L27 with `GetLine` — an INPUT — and computes no EIC (§G-9)",
    "1040.line24": "Tax-Calculator's `c09200` is the liability AFTER nonrefundable credits and also "
    "inherits the line-16 Tax-Table dissent; the harness cross-foots line 24 against OTS instead",
    "8995.line12": "driver-hand-fed to OTS rather than derived by it — a WEAK witness (SPEC §14.2)",
}


def _harness(args: list[str], payload: dict) -> dict:
    if not HARNESS_BIN.exists():
        sys.exit(
            f"the §9 harness is not built ({HARNESS_BIN}). Run: cargo build -p btctax-oracle-harness"
        )
    p = subprocess.run(
        [str(HARNESS_BIN), *args], input=json.dumps(payload), capture_output=True, text=True
    )
    if p.returncode != 0:
        sys.exit(f"oracle_harness {' '.join(args)} failed: {p.stderr.strip()}")
    return json.loads(p.stdout)


def read_row(text: str) -> tuple[dict, list[dict], list[dict]]:
    """Accept either `btctax income project`'s wrapper or a bare `GoldenInputs` row."""
    doc = json.loads(text)
    if isinstance(doc, dict) and "row" in doc:
        return (
            doc["row"],
            doc.get("not_carried", []),
            doc.get("not_carried_from_the_ledger", []),
        )
    return doc, [], []


def cross_check_counts(row: dict) -> None:
    """★★★ The Python dependents block against the RUST one, on every run.

    `gen_goldens.dependent_block` derives `XTOT`/`n24`/`EIC` in Python and
    `GoldenInputs`'s accessors derive them in Rust. Schedule 8812's other-dependent leg is
    `ODC_c * max(0, XTOT - childnum - num)`, so a Python `XTOT` that drifted would move the
    $500-per-dependent credit that the line-19 excuse is measured against — silently, because both
    sides would still print a number. This is the seam that check exists for.
    """
    rust = _harness(["--row-counts"], row)
    py = gen_goldens.dependent_block(row)
    mismatch = {k: (py[k], rust[k]) for k in py if k in rust and py[k] != rust[k]}
    if mismatch:
        sys.exit(
            "the Python and Rust dependent blocks DISAGREE (python, rust): "
            f"{mismatch}. Fix `gen_goldens.dependent_block` or `GoldenInputs`'s accessors — "
            "they are two derivations of one fact and the credit lines turn on them."
        )


def credit_line_verdict(btctax_value: int, oracle_value: float) -> tuple[bool, int, int]:
    """The mechanism-computed excuse for one forgone credit line: `(ok, actual_gap, expected_gap)`.

    btctax prints the line blank (it has no Schedule 8812 and no Schedule EIC), so the WHOLE of the
    oracle's computed credit is the expected gap. Anything else — including an off-by-one — fails.

    ★ Split out of `main` so `selftest()` can watch it discriminate offline. `CLAUDE.md` B1: *"no
      checker exists until it has been observed RED on a planted defect."*
    """
    expected_gap = round(oracle_value)
    actual_gap = round(oracle_value) - btctax_value
    return actual_gap == expected_gap, actual_gap, expected_gap


def selftest() -> None:
    """Offline B1 kill — the excuse must ACCEPT the exact gap and REFUSE every other size.

    Needs no OTS, no taxcalc and no vault: `python3 scripts/oracle/check_return.py --selftest`.
    """
    ok, gap, expected = credit_line_verdict(0, 2500.0)
    assert ok and gap == 2500 and expected == 2500, (ok, gap, expected)
    # A btctax that printed the credit off by one dollar — the plant the kill exists for.
    ok, gap, expected = credit_line_verdict(1, 2500.0)
    assert not ok, "an off-by-one on line 19 was excused"
    assert (gap, expected) == (2499, 2500), (gap, expected)
    # And a btctax that printed the WHOLE credit (a Schedule 8812 that landed): also a divergence,
    # because the excuse is "btctax forgoes it", not "line 19 may be anything".
    ok, gap, expected = credit_line_verdict(2500, 2500.0)
    assert not ok, "a btctax that printed the full credit was excused"
    assert (gap, expected) == (0, 2500), (gap, expected)
    # A household with no credit: the gap is zero and so is the expectation.
    ok, gap, expected = credit_line_verdict(0, 0.0)
    assert ok and (gap, expected) == (0, 0), (ok, gap, expected)
    print("check_return: the credit-line excuse discriminates (B1 kill OK)")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--file", help="the projection JSON (default: stdin)")
    ap.add_argument("--year", type=int, default=2024, help="tax year (default 2024)")
    ap.add_argument(
        "--selftest",
        action="store_true",
        help="offline B1 kill: watch the credit-line excuse refuse an off-by-one, then exit",
    )
    args = ap.parse_args()
    if args.selftest:
        selftest()
        return

    text = Path(args.file).read_text() if args.file else sys.stdin.read()
    row, not_carried, not_carried_ledger = read_row(text)

    cross_check_counts(row)

    # ── btctax, off the printed page ──────────────────────────────────────────────────────────────
    default = _harness([], row)
    if default.get("refused"):
        sys.exit(
            "btctax REFUSES this return, so there is nothing to compare. Run `btctax report "
            f"--tax-year {args.year}` to see which screen fired."
        )
    lines = default["lines"]

    # ── The two oracles ───────────────────────────────────────────────────────────────────────────
    try:
        ots = ots_direct.evaluate(row)
    except Exception as e:  # noqa: BLE001 — the message must name the remedy
        sys.exit(f"OpenTaxSolver could not be driven: {e}\nIs OTS_DIR set to an install directory?")
    taxcalc = gen_goldens.taxcalc_run([row], args.year)[0]
    credits = gen_goldens.taxcalc_credits([row], args.year)[0]

    # ── The per-line verdicts, computed in RUST (never re-implemented here) ───────────────────────
    household = {
        "name": "projected-return",
        "why": f"btctax income project --year {args.year}",
        "inputs": row,
        "expected_ots": ots,
        "expected_taxcalc": taxcalc,
    }
    check = _harness(["--check"], household)
    if check.get("refused"):
        sys.exit("btctax REFUSES this return under --check; nothing to compare.")

    failures: list[str] = []
    # ★★★ THE WITNESS CENSUS, per LINE rather than per row. `CLAUDE.md`: *"Two disqualified oracles
    #     can align"* — three MFS vectors once owed AMT with no witness at all while two oracle
    #     sections each printed OK. A single-oracle row is not a second opinion, so the count is kept
    #     over the set of ENGINES that spoke about each line, and the single-witness lines are named
    #     at the end rather than blending into a wall of OKs.
    witnesses: dict[str, set[str]] = {}

    def witness(line: str, who: str) -> None:
        witnesses.setdefault(line, set()).add(who)

    print(f"── btctax vs two independent engines · tax year {args.year} ──")
    print(f"   OpenTaxSolver: {ots_direct.version()}")
    print(f"   Tax-Calculator: {gen_goldens.tc.__version__}\n")
    print(f"{'line (label)':38s} {'btctax':>10s} {'oracle':>10s}  verdict")

    for v in check["verdicts"]:
        label = v.get("label", v["line"])
        # `verdict_both` rows carry BOTH engines; a single-oracle row puts its figure in `ots` and
        # says which engine it came from in the label (`[OTS]` / `[taxcalc]`).
        if v.get("ots") is not None and v.get("taxcalc") is not None:
            witness(v["line"], "OTS")
            witness(v["line"], "taxcalc")
            oracle = f"{v['ots']}/{v['taxcalc']}"
        elif v.get("ots") is not None:
            witness(v["line"], "taxcalc" if "[taxcalc]" in label else "OTS")
            oracle = str(v["ots"])
        else:
            oracle = "none"
        ok = v["reconciled"]
        note = v.get("class", "")
        print(
            f"{label:38s} {str(v.get('on_paper')):>10s} {oracle:>10s}  "
            f"{'OK' if ok else 'DIVERGES'}  {note}"
        )
        if not ok:
            failures.append(f"{label}: {note}")

    # ── The two lines the harness does not compare, with excuses computed from the mechanism ──────
    for key, label, oracle_value, mechanism in (
        (
            "1040.line19",
            "CTC/ODC (L19)",
            credits["ctc_odc"],
            "btctax has no Schedule 8812, so line 19 is blank; the gap is taxcalc's own "
            "`c07220 + odc`",
        ),
        (
            "1040.line27",
            "EIC (L27)",
            credits["eitc"],
            "btctax computes no earned income credit; the gap is taxcalc's own `eitc`",
        ),
    ):
        # A line btctax does not print has NO cell — that is the blank, not a zero it asserted.
        printed = lines.get(key)
        btctax_value = int(printed) if printed is not None else 0
        ok, actual_gap, expected_gap = credit_line_verdict(btctax_value, oracle_value)
        witness(key, "taxcalc")  # deliberately NOT OTS — see the module docstring
        print(
            f"{label + ' [taxcalc]':38s} {('blank' if printed is None else printed):>10s} "
            f"{round(oracle_value):>10d}  "
            f"{'OK (excused)' if ok else 'DIVERGES'}  gap={actual_gap} expected={expected_gap}"
        )
        if not ok:
            failures.append(
                f"{label}: btctax {btctax_value}, taxcalc {round(oracle_value)} — the gap is "
                f"{actual_gap} and the mechanism predicts exactly {expected_gap} ({mechanism})"
            )

    two = [l for l, w in witnesses.items() if len(w) >= 2]
    one = sorted(l for l, w in witnesses.items() if len(w) < 2)
    print(
        f"\n── witness census ── {len(two)} of {len(witnesses)} compared lines have TWO independent "
        "engines behind them."
    )
    if one:
        print("   single-witness lines (a divergence here is ambiguous, never diagnostic):")
        for line in one:
            who = ", ".join(sorted(witnesses[line]))
            why = SINGLE_WITNESS_REASON.get(line, "only one engine models this line")
            print(f"     {line:22s} witness: {who:8s} — {why}")

    if not_carried or not_carried_ledger:
        print(
            "\n── figures on your return the engines' description CANNOT express ──\n"
            "   A divergence on a line these touch is the description's, not btctax's."
        )
        for nc in not_carried:
            print(f"   {nc['leaf']:48s} {nc['amount']:>14s}  {nc['because']}")
            print(f"      {nc['note']}")
        for nc in not_carried_ledger:
            print(f"   {nc['line']:48s} {nc['amount']:>14s}  FromTheLedger")
            print(f"      {nc['note']}")

    if failures:
        print("\nDIVERGENCES:")
        for f in failures:
            print(f"  · {f}")
        print(
            "\nAdjudicate against the FORM, never against an engine, and never encode an excuse by "
            "name (CLAUDE.md)."
        )
        sys.exit(1)
    print("\nEvery compared line reconciles.")


if __name__ == "__main__":
    main()
