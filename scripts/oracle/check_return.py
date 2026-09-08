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

★★★ **WHAT IS CHECKED IS THE DESCRIPTION, NOT THE PACKET — say it plainly** (T11 seam review C-1).
    Every column below, INCLUDING btctax's own, is computed from the projected row: the harness
    rebuilds a household from it with `build_golden_return`. So this check bounds the DESCRIPTION.
    A figure the row cannot carry is missing from all three columns and produces silence rather than
    a disagreement, which is why `not_carried` is printed at the end and is not decoration.

    And `build_golden_return` is a FIXTURE builder: it sets the tax year, answers every live
    declaration and both Schedule B Part III questions. That is right for the corpus and wrong for a
    filer — a return `btctax report` REFUSES for an unanswered FBAR gate used to come back *"Every
    compared line reconciles"* at exit 0, because the round trip had answered the gate on the
    filer's behalf. So the refusal is taken where the filer's own `ReturnInputs` are in hand:
    `income project` runs the same screens `btctax report` runs and emits a `"refused"` block, and
    this script exits 2 on it. A return that does not file has nothing to reconcile.

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


def cannot_run(message: str) -> None:
    """Exit 2 — *"the run could not be made"*, which is NOT the same as a divergence.

    ★ The docstring has promised `Exit 2 = the run could not be made (no OTS, a refused return, a
      stale harness)` since T11, and every one of these paths used `sys.exit(<str>)`, which exits
      **1** — the divergence code. So a missing OTS install and a real disagreement were
      indistinguishable to any caller, and the refused-return contract could not have been honoured
      even once the refusal was detectable (T11 seam-review fold, C-1).
    """
    print(message, file=sys.stderr)
    sys.exit(2)


try:
    import ots_direct
except ImportError:  # pragma: no cover
    cannot_run("scripts/oracle/ots_direct.py must sit beside this script")

try:
    import gen_goldens
except ImportError:  # pragma: no cover
    cannot_run("scripts/oracle/gen_goldens.py must sit beside this script")

HARNESS_BIN = Path(__file__).resolve().parents[2] / "target" / "debug" / "btctax-oracle-harness"


# ★★★ Why a compared line has only ONE engine behind it — the MECHANISM in each case, never a bare
#     list, and never a default.
#
# ★★ **T11 seam review I-3 — there is no fallback string any more, because the one there was LIED.**
#    `8959.line18`, `8960.line17`, `schedule_se.line12` and `1040sa.line5e` were each reported as
#    *"only one engine models this line"* while Tax-Calculator's own figure sat unread in the same
#    process (measured: `additional_medicare_tax = 1530.0` on a line the census called single-
#    witness). The first three now carry a taxcalc leg in the harness and the fourth got its OTS
#    witness back with the itemize election, so all four are two-witness. A line that reaches the
#    census with one witness and NO named mechanism is now an ERROR: a census that cannot say why
#    cover is missing is worse than none, because knowing where two-oracle cover is genuinely absent
#    is its entire purpose.
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
        cannot_run(
            f"the §9 harness is not built ({HARNESS_BIN}). Run: cargo build -p btctax-oracle-harness"
        )
    p = subprocess.run(
        [str(HARNESS_BIN), *args], input=json.dumps(payload), capture_output=True, text=True
    )
    if p.returncode != 0:
        cannot_run(f"oracle_harness {' '.join(args)} failed: {p.stderr.strip()}")
    return json.loads(p.stdout)


def read_row(text: str) -> dict:
    """Accept either `btctax income project`'s wrapper or a bare `GoldenInputs` row.

    Returns the wrapper's own fields, including `tax_year` (which `--year` must agree with — M-2)
    and `refused` (which makes the run exit 2 — C-1). A bare row carries neither, and says so with
    `tax_year: None` rather than a default that could silently disagree with `--year`.
    """
    doc = json.loads(text)
    if isinstance(doc, dict) and "row" in doc:
        return {
            "row": doc["row"],
            "tax_year": doc.get("tax_year"),
            "refused": doc.get("refused"),
            "not_carried": doc.get("not_carried", []),
            "not_carried_from_the_ledger": doc.get("not_carried_from_the_ledger", []),
        }
    return {
        "row": doc,
        "tax_year": None,
        "refused": None,
        "not_carried": [],
        "not_carried_from_the_ledger": [],
    }


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
        cannot_run(
            "the Python and Rust dependent blocks DISAGREE (python, rust): "
            f"{mismatch}. Fix `gen_goldens.dependent_block` or `GoldenInputs`'s accessors — "
            "they are two derivations of one fact and the credit lines turn on them."
        )


def credit_line_verdict(printed: str | None, oracle_value: float) -> tuple[bool, int, int]:
    """The verdict for one credit line: `(ok, actual_gap, expected_gap)`.

    ★★★ **A BLANK AND AN ASSERTED `0` ARE DIFFERENT TESTIMONY, AND THIS IS WHERE THAT MATTERS**
        (T11 seam review I-2). `printed is None` means btctax put NO CELL on line 19 — no testimony
        — and the excuse applies: it has no Schedule 8812, so the WHOLE of the oracle's computed
        credit is the expected gap, and anything else (an off-by-one, a partial credit) fails.
        `printed is not None` means btctax **swore a figure** under §6065, and there is nothing to
        excuse: it must EQUAL the oracle's, and a sworn `0` against an oracle computing $1,000 is a
        divergence like any other.

    ★★★ **This is the half of FR-85 that lives here, and it is not hypothetical.**
        `advisories.rs::ctc_odc_line19` prints `Some(Usd::ZERO)` — a real cell — whenever
        `ctc_provably_zero` fires, and FR-85 records that `CTC_PER_CHILD_SS24H2` is a year-blind
        $2,000 while Pub. L. 119-21 §70104(a)(2) raises the TY2025 figure to $2,200. So when the
        TY2025 package lands, that proof will conclude *"provably zero"* for a household that still
        has credit and line 19 will print a sworn `0` — taxpayer-adverse and invisible on the page.
        This check is the one instrument that can catch it, and until this fold it could not: both
        `None` and `"0"` were collapsed to the integer `0` and got the same verdict. The
        `--selftest` case named for FR-85 is that exact plant, run offline forever.

    ★ Split out of `main` so `selftest()` can watch it discriminate offline. `CLAUDE.md` B1: *"no
      checker exists until it has been observed RED on a planted defect."*
    """
    expected_gap = round(oracle_value)
    if printed is None:
        # No cell: the forgo excuse. The gap must be the whole credit.
        return True, expected_gap, expected_gap
    asserted = int(printed)
    actual_gap = expected_gap - asserted
    return asserted == expected_gap, actual_gap, expected_gap


def selftest() -> None:
    """Offline B1 kill — the credit-line verdict must tell a BLANK from an asserted figure.

    Needs no OTS, no taxcalc and no vault: `python3 scripts/oracle/check_return.py --selftest`.
    """
    # 1. The blank — no cell on line 19 — is excused by the whole of the oracle's credit.
    ok, gap, expected = credit_line_verdict(None, 2500.0)
    assert ok and gap == 2500 and expected == 2500, (ok, gap, expected)
    # 2. A btctax that PRINTED a figure is sworn to it, so it must equal the oracle's. Off by one:
    ok, gap, expected = credit_line_verdict("2499", 2500.0)
    assert not ok, "an off-by-one on line 19 was excused"
    assert (gap, expected) == (1, 2500), (gap, expected)
    # 3. …and a btctax that printed the WHOLE credit (a Schedule 8812 that landed) RECONCILES. This
    #    case used to be asserted the other way round, which meant the day Schedule 8812 lands the
    #    check would have failed on a correct return (T11 seam review I-2).
    ok, gap, expected = credit_line_verdict("2500", 2500.0)
    assert ok, "a btctax that printed the correct full credit was called a divergence"
    assert (gap, expected) == (0, 2500), (gap, expected)
    # 4. ★★★ **THE FR-85 PLANT, run offline forever.** `ctc_odc_line19` prints `Some(Usd::ZERO)`
    #    whenever `ctc_provably_zero` fires, and FR-85 records that its per-child ceiling is a
    #    year-blind $2,000 against TY2025's $2,200 (Pub. L. 119-21 §70104(a)(2)) — so the proof will
    #    conclude "provably zero" for a household that still has credit and line 19 will print a
    #    SWORN `0`. Before this fold the verdict collapsed `None` and `"0"` to the integer 0 and
    #    excused it: the reviewer planted the understated ceiling, watched the btctax column move
    #    `blank` → `0` against an oracle computing $1,000, and the verdict did not budge.
    ok, gap, expected = credit_line_verdict("0", 1000.0)
    assert not ok, "a SWORN zero on line 19 was read as the forgone blank (FR-85)"
    assert (gap, expected) == (1000, 1000), (gap, expected)
    # 5. A household with no credit at all: blank, and nothing expected.
    ok, gap, expected = credit_line_verdict(None, 0.0)
    assert ok and (gap, expected) == (0, 0), (ok, gap, expected)
    # 6. …and a printed `0` on a household with no credit is TRUE testimony, so it reconciles.
    ok, gap, expected = credit_line_verdict("0", 0.0)
    assert ok and (gap, expected) == (0, 0), (ok, gap, expected)
    # 7. ★ C-1 — a refused return must never reconcile. The wrapper's own block is what says so.
    refused = read_row(json.dumps({"tax_year": 2024, "row": {}, "refused": {"screen": "X"}}))
    assert refused["refused"], "the wrapper's `refused` block was dropped by `read_row`"
    assert read_row(json.dumps({"tax_year": 2024, "row": {}}))["refused"] is None
    # 8. ★ M-2 — a bare row carries no year, so `--year` cannot silently contradict one.
    assert read_row(json.dumps({"filing_status": "Single"}))["tax_year"] is None
    assert read_row(json.dumps({"tax_year": 2025, "row": {}}))["tax_year"] == 2025
    # 9. ★ I-3 — every line the census can call single-witness has a named mechanism, and the
    #    census refuses an unnamed one rather than printing the old false default.
    assert set(SINGLE_WITNESS_REASON) == {
        "1040.line19",
        "1040.line24",
        "1040.line27",
        "8995.line12",
    }, SINGLE_WITNESS_REASON.keys()
    print("check_return: blank-vs-sworn, refusal, year and witness checks discriminate (B1 kill OK)")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--file", help="the projection JSON (default: stdin)")
    ap.add_argument(
        "--year",
        type=int,
        help="tax year; DEFAULTS to the projection's own `tax_year` and REFUSES to contradict it "
        "(a 2024 projection driven with --year 2025 put Tax-Calculator on 2025 law while btctax "
        "and OTS stayed on 2024, and fabricated a divergence on 1040 line 15)",
    )
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
    doc = read_row(text)
    row = doc["row"]
    not_carried = doc["not_carried"]
    not_carried_ledger = doc["not_carried_from_the_ledger"]

    # ── M-2: the year is the PROJECTION's, and `--year` may not silently contradict it ────────────
    wrapper_year = doc["tax_year"]
    if args.year is None:
        year = wrapper_year if wrapper_year is not None else 2024
    elif wrapper_year is not None and args.year != wrapper_year:
        cannot_run(
            f"--year {args.year} contradicts the projection's own tax_year {wrapper_year}. The "
            "engines would be driven on different law from btctax and each other — Tax-Calculator "
            "would use --year while OpenTaxSolver 2024 and btctax's harness stay on 2024 — and the "
            f"divergence would be fabricated. Re-run `btctax income project --year {args.year}`, "
            "or drop --year."
        )
    else:
        year = args.year

    # ── C-1: a return btctax will not COMPUTE has nothing to reconcile ───────────────────────────
    # `income project` runs the same screens `btctax report` runs, on the filer's OWN inputs. It has
    # to happen there: the harness rebuilds a household from the ROW with `build_golden_return`,
    # which answers every live declaration and both Schedule B Part III questions on the filer's
    # behalf — so the refusal is answered away in the round trip and this script would otherwise
    # print "Every compared line reconciles" for a return that does not file.
    if doc["refused"]:
        r = doc["refused"]
        cannot_run(
            f"btctax REFUSES this return — the {r.get('screen')} screen fired, so there is nothing "
            f"to compare:\n\n  {r.get('detail')}\n\nAnswer it and re-run `btctax income project "
            f"--year {year}`."
        )

    cross_check_counts(row)

    # ── btctax, off the printed page ──────────────────────────────────────────────────────────────
    default = _harness([], row)
    if default.get("refused"):
        cannot_run(
            "the projected household is out of the harness's domain (an AMT screen, an unmodeled "
            f"input, or a form that will not fill), so there is nothing to compare. Run `btctax "
            f"report --tax-year {year}` to see which screen fired."
        )
    lines = default["lines"]

    # ── The two oracles ───────────────────────────────────────────────────────────────────────────
    try:
        ots = ots_direct.evaluate(row)
    except Exception as e:  # noqa: BLE001 — the message must name the remedy
        cannot_run(
            f"OpenTaxSolver could not be driven: {e}\nIs OTS_DIR set to an install directory?"
        )
    taxcalc = gen_goldens.taxcalc_run([row], year)[0]
    credits = gen_goldens.taxcalc_credits([row], year)[0]

    # ── The per-line verdicts, computed in RUST (never re-implemented here) ───────────────────────
    household = {
        "name": "projected-return",
        "why": f"btctax income project --year {year}",
        "inputs": row,
        "expected_ots": ots,
        "expected_taxcalc": taxcalc,
    }
    check = _harness(["--check"], household)
    if check.get("refused"):
        cannot_run("btctax REFUSES this return under --check; nothing to compare.")

    failures: list[str] = []
    # ★★★ THE WITNESS CENSUS, per LINE rather than per row. `CLAUDE.md`: *"Two disqualified oracles
    #     can align"* — three MFS vectors once owed AMT with no witness at all while two oracle
    #     sections each printed OK. A single-oracle row is not a second opinion, so the count is kept
    #     over the set of ENGINES that spoke about each line, and the single-witness lines are named
    #     at the end rather than blending into a wall of OKs.
    witnesses: dict[str, set[str]] = {}

    def witness(line: str, who: str) -> None:
        witnesses.setdefault(line, set()).add(who)

    print(f"── btctax vs two independent engines · tax year {year} ──")
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
            # ★ N-1/I-3 — the ENGINE comes off the verdict itself. It used to be recovered by
            #   looking for the substring "[taxcalc]" in the human label, because every
            #   single-engine row was built by a function hardwired to OTS (which also made a
            #   taxcalc row print `class: agree-ots`). A census that reads a display string is one
            #   relabelling away from miscounting the thing it exists to count.
            witness(v["line"], v.get("engine", "OTS"))
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
        # ★★★ A line btctax does not print has NO cell — that is the blank, not a zero it asserted,
        #     and the WHOLE `Optional` goes into the verdict so the two can be told apart (I-2).
        #     `advisories.rs::ctc_odc_line19` really does print `Some(Usd::ZERO)` when
        #     `ctc_provably_zero` fires, which is the FR-85 hazard this now catches.
        printed = lines.get(key)
        ok, actual_gap, expected_gap = credit_line_verdict(printed, oracle_value)
        witness(key, "taxcalc")  # deliberately NOT OTS — see the module docstring
        verdict_text = ("OK (excused)" if printed is None else "OK (asserted)") if ok else "DIVERGES"
        print(
            f"{label + ' [taxcalc]':38s} {('blank' if printed is None else printed):>10s} "
            f"{round(oracle_value):>10d}  "
            f"{verdict_text}  gap={actual_gap} expected={expected_gap}"
        )
        if not ok:
            failures.append(
                f"{label}: btctax SWORE {printed}, taxcalc computed {round(oracle_value)} — a "
                f"printed cell is testimony, not a forgone line, so it must EQUAL the oracle's "
                f"figure; the gap is {actual_gap} ({mechanism})"
            )

    two = [l for l, w in witnesses.items() if len(w) >= 2]
    one = sorted(l for l, w in witnesses.items() if len(w) < 2)
    print(
        f"\n── witness census ── {len(two)} of {len(witnesses)} compared lines have TWO independent "
        "engines behind them."
    )
    unnamed: list[str] = []
    if one:
        print("   single-witness lines (a divergence here is ambiguous, never diagnostic):")
        for line in one:
            who = ", ".join(sorted(witnesses[line]))
            why = SINGLE_WITNESS_REASON.get(line)
            if why is None:
                unnamed.append(f"{line} (witness: {who})")
                why = "*** NO MECHANISM ON FILE — see below ***"
            print(f"     {line:22s} witness: {who:8s} — {why}")
    # ★★★ I-3 — an UNNAMED single-witness line is an error, not a shrug. The string that used to
    #     stand here ("only one engine models this line") was false for four lines whose second
    #     engine's figure was already in this process, and it was the census's default — so a line
    #     that quietly LOST a witness (as Schedule A line 5e did while no projected row could tell
    #     OTS to itemize) passed as "modelled by one engine". Knowing where two-oracle cover is
    #     genuinely absent is the census's entire purpose; a default that guesses destroys it.
    if unnamed:
        failures.append(
            "the witness census has no mechanism on file for "
            + ", ".join(unnamed)
            + ". Either the line lost a witness it used to have (find out why — that is a real "
            "finding), or the other engine does model it and the harness is not asking. Do not add "
            "a default string: add the mechanism to SINGLE_WITNESS_REASON, or the missing leg."
        )

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
