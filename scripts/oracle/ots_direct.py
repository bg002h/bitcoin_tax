"""Drive OpenTaxSolver's own binaries directly — oracle #1 for the golden returns.

★ Why this exists, and why it is NOT `tenforty`
-----------------------------------------------
P7.1 originally used `tenforty` (a Python wrapper around OTS) as the first oracle. It turned
out to have TWO input-plumbing defects, both of which OVERSTATE a self-employed filer's tax:

  1. Schedule SE line 8a (the filer's own W-2 social security wages) was never populated, so
     the 12.4% OASDI rate was charged on self-employment earnings the wage base had already
     absorbed. SE tax came out INVARIANT to wages.
  2. The §199A QBI deduction (1040 line 13) was never supplied on its OTS backend, so taxable
     income was overstated by the whole deduction.

Both were reported upstream (mmacpherson/tenforty#278, with a fix in #279). Crucially, **OTS
itself is correct in both cases** — its Schedule SE solver reads and honours `L8a`, its 1040
reads `L13`, and it even ships a Form 8995 solver. The wrapper simply never passed the values.
So we drop the wrapper and drive the engine.

★ A LIMIT on this oracle's independence — say it out loud (Fable P7 r2, Minor)
--------------------------------------------------------------------------
OTS's 1040 is driven from raw inputs and derives everything itself. Its **Form 8995** is not:
this harness hand-computes two of its inputs — `L1_i_c` (the QBI base = Schedule C profit net of
the §164(f) half-SE deduction) and `L12` (§1222(11) net capital gain, increased by qualified
dividends) — because OTS's 8995 reads a 1040 *output* file that carries a taxable income, not
those quantities. OTS still computes the whole 8995 chain and the income limitation from them.

The consequence: on those two inputs OTS cannot independently catch an error — if btctax's notion
of net capital gain were wrong, the same wrong number would be handed to OTS and it would agree.
**PSL Tax-Calculator is the only fully independent witness there** (it derives line 12 from
`p23250`/`p22250`/`e00650`). It agrees. But that is one witness, not two, and the two-oracle claim
is thinner on the QBI path than everywhere else. Deriving `L12` from OTS's own Schedule D output
would close it.

★ Licensing / clean-room posture (SPEC §9, recon 05)
----------------------------------------------------
OTS is GPL-2.0, INCOMPATIBLE with our `MIT OR Unlicense`. It is used **observe-only**: we
execute the shipped binaries and read the numbers they print. We do not read, copy, link,
vendor or distribute its source, and nothing it produces enters btctax's implementation —
only its computed FIGURES land in the golden file, and computed tax figures are FACTS, not
copyrightable expression (the same reasoning already applied to the bundled price data).

★ How a full return is assembled
--------------------------------
OTS's 1040 solver does not compute the subsidiary forms; you feed it their results. It does
carry Schedule D internally (the `D1ad`/`D8ad` proceeds/cost lines) and runs the qualified
dividends & capital gain worksheet itself. The QBI deduction is circular — it is limited by
taxable income, which depends on the deduction — so the 1040 is run twice, exactly as OTS's
own Form 8995 solver expects (it reads a 1040 *output* file to resolve the limitation):

    Schedule SE ─┬─> SE tax (S2_4), half-SE deduction (S1_15)
    Form 8959   ─┴─> additional medicare (S2_11)
                     │
                     ├─> 1040 pass 1 (L13 = 0)        ──> AGI, taxable income before QBI
                     │       │
                     │       ├─> Form 8960 (needs AGI) ──> NIIT
                     │       └─> Form 8995 (reads the pass-1 output) ──> QBI deduction
                     │
                     └─> 1040 pass 2 (L13 = QBI deduction) ──> the filed figures

Set OTS_DIR to an unpacked OpenTaxSolver2024 tree. Nothing here is imported by btctax; this
script runs by hand and its answers are committed (the CI is offline and must stay hermetic).
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

OTS_DIR = Path(os.environ.get("OTS_DIR", "")).expanduser()

# ★ The solver YEAR, separate from OTS_DIR. Two installs coexist here
# (`OpenTaxSolver2024_22.07_linux64` and `OpenTaxSolver2025_23.06_linux64`), and pointing OTS_DIR at
# the 2025 tree while this stayed 2024 failed with a confusing "solver not found" — the tree and the
# year are two facts, so they are two settings.
OTS_YEAR = int(os.environ.get("OTS_YEAR", "2024"))


def _bin(form: str, year: int | None = None) -> Path:
    year = OTS_YEAR if year is None else year
    p = OTS_DIR / "bin" / f"taxsolve_{form}_{year}"
    if not p.exists():
        raise FileNotFoundError(
            f"OTS solver not found: {p}. Set OTS_DIR to an OpenTaxSolver{year} tree, "
            f"or set OTS_YEAR to match the tree OTS_DIR points at."
        )
    return p


def _template(subdir: str, name: str, year: int | None = None) -> str:
    year = OTS_YEAR if year is None else year
    d = OTS_DIR / "tax_form_files" / subdir
    for cand in (f"{name}_{year}_template.txt", f"{name}_template.txt"):
        p = d / cand
        if p.exists():
            return p.read_text()
    raise FileNotFoundError(f"no template for {subdir}/{name} under {d}")


def _capgains_rows(stcg: float, ltcg: float) -> list[str]:
    """Build OTS `CapGains-A/D` transaction rows for a net short- and long-term gain.

    OTS's 1040 does NOT take a net capital gain as a number. Its `D1ad`/`D8ad` aggregate lines
    are read but never reach line 7 — the only path that does is the transaction list its own
    example uses. That is the better fit anyway: OTS decides short versus long from the DATES,
    exactly as Form 8949 does, so this exercises the real Schedule D rather than a shortcut.

    Each transaction is three lines: `-basis  buy-date  {note}`, then `proceeds  sell-date`,
    then an adjustment-code row (`~ ~` = none). The basis is arbitrary — only the difference
    reaches the return — so we pick one large enough that a loss never drives it negative.
    """
    rows: list[str] = []
    basis = 100_000.00

    def txn(gain: float, buy: str, note: str) -> None:
        rows.extend(
            [
                f"        -{basis:.2f}\t{buy}\t{{ {note} }}",
                f"         {basis + gain:.2f}\t6-01-2024",
                "\t~\t~",
                "",
            ]
        )

    if stcg:
        txn(stcg, "2-01-2024", "short-term: held ~4 months")
    if ltcg:
        txn(ltcg, "3-15-2019", "long-term: held ~5 years")
    return rows


def _fill(template: str, values: dict[str, object], capgains: list[str] | None = None) -> str:
    """Substitute values into an OTS template, preserving its line ORDER.

    OTS parses its input files strictly in template order — supply `L8a` before `L5a` and it
    fails with "Found 'L8a' when expecting 'L5a'". So we rewrite the template in place rather
    than emit our own file, which also means every optional line OTS expects stays present.
    """
    remaining = dict(values)
    out: list[str] = []
    for line in template.splitlines():
        m = re.match(r"^\s*([A-Za-z][A-Za-z0-9_#/]*:?)(?=\s|;|\{|$)(.*)$", line, re.S)
        key = m.group(1) if m else None
        if key not in remaining:
            out.append(line)
            continue
        value = remaining.pop(key)
        rest = m.group(2)
        # Keep the line's `{...}` comment. OTS comments may OPEN here and CLOSE on a later
        # line; dropping the opening brace would leave that continuation as stray tokens and
        # derail the parse ("Found 'have' when expecting 'L2'").
        brace = rest.find("{")
        comment = " " + rest[brace:] if brace >= 0 else ""
        # Whether a field ends with ';' or a newline varies per field, and the template is the
        # only authority on which — the 1040 terminates L1a with a newline but L1b with a
        # semicolon. Copy whatever the template does.
        head = rest[:brace] if brace >= 0 else rest
        term = " ;" if ";" in head else ""
        out.append(f"{key} {value}{term}{comment}")
    if remaining:
        raise KeyError(f"keys not found in template: {sorted(remaining)}")

    if capgains:
        # The CapGains-A/D section already carries its own ';' terminator; the transactions go
        # in front of it.
        start = next(i for i, l in enumerate(out) if l.startswith("CapGains-A/D"))
        end = next(i for i in range(start, len(out)) if out[i].strip() == ";")
        out[end:end] = capgains

    return "\n".join(out) + "\n"


def _parse(out_text: str) -> dict[str, float]:
    """Read the `Lxx = value` lines OTS prints. Its own output is the only thing we consume."""
    found: dict[str, float] = {}
    for line in out_text.splitlines():
        m = re.match(r"^\s*([A-Za-z][A-Za-z0-9_]*)\s*=\s*(-?[\d,]+\.?\d*)", line)
        if m:
            found.setdefault(m.group(1), float(m.group(2).replace(",", "")))
    return found


# ── OTS TY2024 known defects (confirmed in source; both fixed in OTS 2025 v23.06) ────────────────
#
# 1. `taxsolve_US_1040_2024.c:270-275` carries the **2023** §55(d)(3) MFS constants — 831,150 /
#    1,084,150 / 63,250 — where i6251 (2024) p.9 gives 875,950 / 1,142,550 / 66,650. The same
#    function uses correct 2024 values for every exemption constant, so one block was updated and its
#    neighbour was not. On our V8 vector OTS returns Form 6251 line 4 = 912,150 against the form's
#    900,950 — and i6251 works that exact case ("$895,950 → enter $900,950").
#
# 2. The 2024 Schedule A applies **no §170(b)(1)(G) 60%-of-AGI ceiling** on cash gifts
#    (`SchedA[11] = charityCC;`), so an over-ceiling gift deflates taxable income and therefore
#    Form 6251 line 1. The 2025 solver caps it. btctax and Tax-Calculator agree against OTS here.
OTS_2024_MFS_KICKER_STALE_THRESHOLD = 831_150.0
OTS_2024_CASH_CEILING_FRACTION = 0.60

# ★★ BOTH DEFECTS ARE 2024-ONLY, AND SCOPING THEM IS WHAT MAKES THE TY2025 PIVOT WORK.
#
# Verified in `taxsolve_US_1040_2025.c`: the MFS §55(d)(3) block carries the correct TY2025 triple
# (900,350 / 1,174,350 / 68,500 — `:271-275`, and 900,350 is also the TY2025 zero-exemption point on
# the archived f6251--2025.pdf), and Schedule A now caps cash gifts at 60% of AGI (`:2424-2429`).
#
# Left year-blind, this predicate would disqualify TY2025's MFS-kicker vectors against a solver that
# handles them CORRECTLY — the entire purpose of moving to TY2025 — while every run printed OK and the
# witness census reported them "not witnessed". The bug would have been invisible and total.
# ── Schedule 1-A: present from TY2025, and SILENTLY SKIPPABLE ─────────────────────────────────────
#
# ★ `taxsolve_US_1040_2025.c:2375-2391` reads either `S1A_2a` **or** `A1`:
#
#       get_parameter( infile, 'l', word, "S1A_2a or A1" );   /* Fix next year. */
#       if (strcmp( word, "S1A_2a" ) == 0) { ... sched_1A( sched1A_L2a ); }
#       if (strcmp( word, "A1" ) == 0)     { ...  }           /* sched_1A NEVER called */
#
# On the `A1` path Schedule 1-A does not run and `L13b` stays 0 — **no error, no warning**. Since
# `_fill` rewrites the solver's own template in place, the `S1A_*` lines are normally carried through
# blank (= 0), which is right for a household with no tips/overtime/vehicle interest. But "normally"
# is not a guarantee: a template revision or a hand-built input file would drop the block and every
# such household would reconcile perfectly against a Schedule 1-A that never executed.
#
# `run_form` therefore ASSERTS the block is present for TY2025+. Fail loudly, not quietly.
OTS_FIRST_YEAR_WITH_SCHEDULE_1A = 2025
OTS_SCHEDULE_1A_SENTINEL = "S1A_2a"

# Every `S1A_*` input the TY2025 template exposes, so a caller can drive Schedule 1-A rather than
# only zero it. Names are OTS's own; values are ours.
OTS_SCHEDULE_1A_KEYS = (
    "S1A_2a", "S1A_2b", "S1A_2c", "S1A_2d",          # Part I MAGI add-backs (PR, 2555 L45/L50, 4563 L15)
    "S1A_4a", "S1A_4b", "S1A_5",                     # Part II qualified tips
    "S1A_14a", "S1A_14b",                            # Part III qualified overtime
    "S1A_22a", "S1A_22aii", "S1A_22aiii",            # Part IV vehicle 1: VIN + interest
    "S1A_22b", "S1A_22bii", "S1A_22biii",            # Part IV vehicle 2
)

OTS_YEARS_WITH_STALE_MFS_KICKER = frozenset({2024})
OTS_YEARS_WITHOUT_CASH_CEILING = frozenset({2024})


def _ots_amt_disqualified(
    status: str,
    h: dict,
    final: dict[str, float],
    ots6251: dict[str, float],
    year: int | None = None,
) -> str | None:
    """Return why OTS cannot witness this household's AMT, or `None` if it can.

    Fail-CLOSED: an unrecognised shape returns a reason rather than silently witnessing. Each
    disqualification is scoped to the solver years that actually carry the defect — a defect fixed
    upstream must stop gating, or the fix buys us nothing.
    """
    year = OTS_YEAR if year is None else year
    if not ots6251:
        return "OTS printed no Form 6251 for this household"
    line4 = ots6251.get("line4")
    if (
        year in OTS_YEARS_WITH_STALE_MFS_KICKER
        and status == "Married/Sep"
        and line4 is not None
        and line4 > OTS_2024_MFS_KICKER_STALE_THRESHOLD
    ):
        return (
            f"OTS {year} §55(d)(3) MFS constants are the 2023 triple (line 4 = {line4:,.0f} is above "
            f"the stale {OTS_2024_MFS_KICKER_STALE_THRESHOLD:,.0f} threshold) — "
            "taxsolve_US_1040_2024.c:270-275"
        )
    cash_gift = float(h.get("charitable_cash", 0) or 0)
    agi = final.get("L11", 0.0)
    # ★★★ `cash_gift > 0` IS LOAD-BEARING, not defensive (phase-4 review M-3).
    #
    # Without it, a household with NO GIFT AT ALL is disqualified the moment AGI goes negative:
    # `0 > 0.60 * -3,000` is `0 > -1,800`, which is TRUE. That is exactly what happened to the new
    # `single_loss_year_taxable_income_at_the_floor` cell — a loss year with no charitable gift on
    # it anywhere — and OTS silently stopped witnessing its AMT while the run printed OK, for a
    # stated reason ("gift 0 exceeds 60% of AGI -3,000") that is absurd on its face.
    #
    # The defect this leg models is "OTS deducts a cash gift the §170(b) ceiling should have capped".
    # No gift, nothing to over-deduct — so the mechanism itself says the guard belongs here.
    if (
        year in OTS_YEARS_WITHOUT_CASH_CEILING
        and cash_gift > 0
        and cash_gift > OTS_2024_CASH_CEILING_FRACTION * agi
    ):
        return (
            f"OTS {year} applies no §170(b) 60%-of-AGI cash ceiling (gift {cash_gift:,.0f} exceeds "
            f"60% of AGI {agi:,.0f}), which deflates Form 6251 line 1"
        )
    return None


def selftest_defect_years() -> None:
    """Assert each OTS defect gates the years that carry it — and ONLY those. Pure; no OTS needed.

    ★ This exists because the year-blind version of `_ots_amt_disqualified` would have silently
    defeated the whole reason for moving to TY2025. It hardcoded OTS 2024's stale 831,150 threshold
    and applied the cash-ceiling leg unconditionally, so a TY2025 MFS-kicker household — handled
    CORRECTLY by `taxsolve_US_1040_2025.c` — would have been reported "not witnessed" while every run
    printed OK. A wrong exclusion is as damaging as a wrong inclusion and far quieter.

    Called from `verify_f6251.py` before any solver runs, so it cannot be skipped.
    """
    mfs_above = {"line4": 943_662.50, "line11": 13_571.50}
    gated = _ots_amt_disqualified("Married/Sep", {}, {"L11": 960_000.0}, mfs_above, year=2024)
    assert gated and "55(d)(3)" in gated, "OTS 2024 MUST be gated on an MFS line 4 above 831,150"
    assert (
        _ots_amt_disqualified("Married/Sep", {}, {"L11": 960_000.0}, mfs_above, year=2025) is None
    ), (
        "OTS 2025 carries the correct §55(d)(3) triple (900,350 / 1,174,350 / 68,500) and must NOT be "
        "gated — gating it here is exactly the failure that makes the TY2025 pivot pointless"
    )

    over_ceiling = {"charitable_cash": 1_000_000.0}
    final = {"L11": 1_500_000.0}
    six = {"line4": 600_000.0, "line11": 0.0}
    assert _ots_amt_disqualified("Married/Joint", over_ceiling, final, six, year=2024), (
        "OTS 2024 applies no §170(b) 60%-of-AGI cash ceiling and MUST be gated"
    )
    assert _ots_amt_disqualified("Married/Joint", over_ceiling, final, six, year=2025) is None, (
        "OTS 2025 caps cash gifts at 60% of AGI (taxsolve_US_1040_2025.c:2424) and must NOT be gated"
    )

    # ★★★ NEGATIVE AGI WITH NO GIFT — the row this selftest did not have (phase-4 review M-3).
    #
    # `single_loss_year_taxable_income_at_the_floor` has AGI of −3,000 and no charitable gift of any
    # kind. Without the `cash_gift > 0` guard, `0 > 0.60 * -3,000` is TRUE and OTS was disqualified
    # from witnessing that household's AMT — quietly, on the FIRST household of a new shape, with a
    # reason ("gift 0 exceeds 60% of AGI -3,000") that reads as nonsense. Both engines compute AMT $0
    # there so nothing was mis-filed, but an instrument went dark and every run printed OK.
    #
    # PLANT TO RE-RUN THE KILL: delete `cash_gift > 0` from `_ots_amt_disqualified` ⇒ this reds.
    loss_year = {"L11": -3_000.0}
    six_zero = {"line4": 0.0, "line11": 0.0}
    for y in (2024, 2025):
        assert (
            _ots_amt_disqualified("Single", {}, loss_year, six_zero, year=y) is None
        ), (
            f"({y}) a household with NO GIFT must not be disqualified by the cash-ceiling leg just "
            "because its AGI is negative — 0 > 0.60 * -3,000 is true and means nothing"
        )
    # …and the leg still bites when there IS a gift over the ceiling, negative AGI or not.
    assert _ots_amt_disqualified(
        "Single", {"charitable_cash": 5_000.0}, loss_year, six_zero, year=2024
    ), "a REAL gift against negative AGI is over any 60% ceiling and OTS 2024 must still be gated"

    # Fail-closed is not year-scoped: no printed form is never a witness, in any year.
    for y in (2024, 2025):
        assert _ots_amt_disqualified("Single", {}, {}, {}, year=y), f"empty 6251 must not witness ({y})"


def _form6251_lines(parsed: dict[str, float]) -> dict[str, float]:
    """Pull OTS's printed Form 6251 lines out of a parsed 1040 output.

    OTS emits them as `AMT_Form_6251_L<n>` (e.g. `AMT_Form_6251_L2a = 29200.00`). Returning them
    keyed by the FORM's own line label — `line1`, `line2a`, `line11`, … — lets a caller diff btctax's
    `Form6251` against an independent engine line by line rather than only at the bottom line.
    **Which** lines get compared is the caller's business: `verify_f6251.py` compares every line both
    engines produce, minus the AMT foreign tax credit (line 8), which our driver feeds to OTS and so
    would be comparing an input to itself. Note OTS does NOT print lines 2c-2t — it stubs them like we
    do — so this cannot witness that region and no caller should imply it does.

    Absent when the filer owes no AMT and OTS skips the routine: returns `{}`, which callers must
    treat as "not witnessed", never as zeros.
    """
    out: dict[str, float] = {}
    for k, v in parsed.items():
        if not k.startswith("AMT_Form_6251_L"):
            continue
        out["line" + k[len("AMT_Form_6251_L") :]] = v
    return out


def run_form(
    form: str,
    subdir: str,
    tname: str,
    values: dict[str, object],
    work: Path,
    capgains: list[str] | None = None,
    year: int | None = None,
) -> tuple[dict[str, float], Path]:
    """Run one OTS solver; return its parsed lines and the path of its output file."""
    template = _template(subdir, tname, year)
    # OTS reads a blank `YourName:` as consuming the NEXT line as its value, which then
    # derails the whole strict-order parse. Every identity field must carry something.
    identity = {"YourName:": "Golden Household", "YourSocSec#:": "000-00-0000"}
    values = {
        **{k: v for k, v in identity.items() if re.search(rf"^{re.escape(k)}", template, re.M)},
        **values,
    }
    src = work / f"{form}.txt"
    filled = _fill(template, values, capgains)
    # ★ Schedule 1-A is skippable in silence (see OTS_SCHEDULE_1A_SENTINEL) — refuse to run without it.
    if (
        form == "US_1040"
        and (OTS_YEAR if year is None else year) >= OTS_FIRST_YEAR_WITH_SCHEDULE_1A
        and not re.search(rf"^\s*{OTS_SCHEDULE_1A_SENTINEL}\b", filled, re.M)
    ):
        raise RuntimeError(
            f"generated OTS input has no {OTS_SCHEDULE_1A_SENTINEL} line, so OTS would take its 'A1' "
            f"branch and never run sched_1A() — Schedule 1-A would be silently 0 with no error. "
            f"Template: {form}"
        )
    src.write_text(filled)
    proc = subprocess.run(
        [str(_bin(form, year)), src.name], cwd=work, capture_output=True, text=True
    )
    out_path = work / f"{form}_out.txt"
    if not out_path.exists():
        raise RuntimeError(f"{form}: no output.\n{proc.stdout}\n{proc.stderr}")
    text = out_path.read_text()
    if "ERROR" in text:
        bad = [ln for ln in text.splitlines() if "ERROR" in ln]
        raise RuntimeError(f"{form}: OTS reported {bad}")
    return _parse(text), out_path


def evaluate(h: dict) -> dict[str, float | None]:
    """Compute one household's federal return by driving OTS end to end."""
    status = h.get("filing_status", "Single")
    w2 = h.get("w2_income", 0)
    se_profit = h.get("self_employment_income", 0)
    stcg = h.get("short_term_capital_gains", 0)
    ltcg = h.get("long_term_capital_gains", 0)

    work = Path(tempfile.mkdtemp(prefix="ots-"))
    try:
        se_tax = half_se = addl_medicare = 0.0
        # C1 cross-foot legs (§6.1 → §6.2) — captured from the SE / 8959 solvers when they run,
        # left None otherwise so §6.4's Option rule leaves them unwitnessed (T8 emits, T9 leaves
        # taxcalc's None). qbi_cap_l12 is likewise None unless a Form 8995 is run.
        se_l10_oasdi = se_l11_medicare = None
        f8959_l7 = f8959_l13 = None
        qbi_cap_l12 = None
        # §1(h) net-capital-gain subterms. The QD-EXCLUSIVE leaf is the §1222(11) long-term gain
        # that survives cross-netting against short-term, floored at zero (r5-N2 — this is NOT the
        # QD-inclusive net_capital_gain); the QBI cap (8995 L12) adds qualified dividends on top
        # (§199A(a)(1)(B) / §1(h)).
        net_ltcg_qd_exclusive = max(0.0, min(float(ltcg), float(ltcg) + float(stcg)))
        net_capital_gain = h.get("qualified_dividends", 0) + net_ltcg_qd_exclusive

        if se_profit:
            # Schedule SE. Line 8a is the filer's OWN social security wages: these fill the
            # OASDI band before self-employment earnings do (§1402(b)(1)). Our fixtures model
            # the wages as belonging to the self-employed person — the same attribution we
            # give Tax-Calculator via e00200p/e00900p, and the one btctax's `se_w2_ss_wages`
            # input carries — so the three engines are answering the same question.
            se, _ = run_form(
                "US_1040_Sched_SE",
                "US_1040_Sched_SE",
                "US_1040_Sched_SE",
                {"L2": se_profit, "L5a": 0, "L8a": w2, "L8b": 0, "L8c": 0},
                work,
            )
            se_tax, half_se = se.get("L12", 0.0), se.get("L13", 0.0)
            se_l10_oasdi = se.get("L10")     # OASDI leg — Sch SE L10 (0 once wages fill the band)
            se_l11_medicare = se.get("L11")  # Medicare leg — Sch SE L11

            f8959, _ = run_form(
                "f8959",
                "Form_8959",
                "Form_8959",
                {"Status": status, "L1": w2, "L8": round(se_profit * 0.9235)},
                work,
            )
            addl_medicare = f8959.get("L18", 0.0)
            f8959_l7 = f8959.get("L7")       # 8959 Part I leg (Additional Medicare on wages)
            f8959_l13 = f8959.get("L13")     # 8959 Part II leg (Additional Medicare on SE income)
        elif w2:
            f8959, _ = run_form(
                "f8959", "Form_8959", "Form_8959", {"Status": status, "L1": w2}, work
            )
            addl_medicare = f8959.get("L18", 0.0)
            f8959_l7 = f8959.get("L7")       # 8959 Part I leg (Additional Medicare on wages)
            f8959_l13 = f8959.get("L13")     # 8959 Part II leg (no SE income → 0)

        # The 1040 carries Schedule D itself, and runs the qualified dividends & capital gain
        # worksheet. Gains go in as 8949-shaped TRANSACTIONS, not as a net number — see
        # `_capgains_rows`.
        base: dict[str, object] = {
            "Status": status,
            "L1a": w2,
            "L2b": h.get("taxable_interest", 0),
            "L3a": h.get("qualified_dividends", 0),
            "L3b": h.get("ordinary_dividends", 0),
            "S1_3": se_profit,
            "S1_15": half_se,
            "S2_4": se_tax,
            "S2_11": addl_medicare,
        }
        if h.get("standard_or_itemized") == "Itemized":
            # OTS applies the $10,000 §164(b)(5) cap itself, on Schedule A line 5e — so the components
            # must go in as themselves. A lump sum in "other deductions" would sail past the cap.
            base["A5a"] = h.get("state_income_tax", 0)   # state & local income tax
            base["A5b"] = h.get("real_estate_tax", 0)    # real estate tax
            base["A8a"] = h.get("mortgage_interest", 0)  # mortgage interest reported on a 1098
            # A11 is cash/check charity — NOT A16 ("other"), which sails past Schedule A's own
            # handling of the gift. OTS 2024 applies no §170(b) 60%-of-AGI ceiling, so a corpus cell
            # must keep the gift under it or lose this oracle (the fate of the 6251 fixture's V2b).
            base["A11"] = h.get("charitable_cash", 0)   # cash gift to a public charity (Sch A 11)
            base["A16"] = h.get("itemized_deductions", 0)  # the lump-sum household's "other"
            base["A18"] = "Yes"

        gains = _capgains_rows(stcg, ltcg)

        # Pass 1: no QBI deduction yet — it is limited BY taxable income, so it cannot be
        # known until the 1040 has produced one.
        p1, p1_out = run_form(
            "US_1040", "US_1040", "US_1040", {**base, "L13": 0}, work, capgains=gains
        )

        qbi_deduction = 0.0
        if se_profit:
            # OTS's own Form 8995 resolves the taxable-income limitation by reading a 1040
            # OUTPUT file — which is exactly the mechanism a wrapper needs and tenforty lacks.
            # QBI is the Schedule C profit NET of the §164(f) half-SE deduction.
            # ★ Line 12 — NET CAPITAL GAIN — must be supplied. §199A(a)(1)(B) caps the deduction at
            # 20% of (taxable income − net capital gain), and OTS's 8995 models it (its template has an
            # `L12` key) but cannot infer it: the 1040 output it reads carries a taxable income, not a
            # §1(h) net capital gain. Leaving it blank silently DROPS the cap.
            #
            # It went unnoticed for a while because every other QBI household in the matrix has no
            # capital gain, so line 12 was zero and OTS agreed by accident. §1222(11): the net capital
            # gain is the long-term gain that survives cross-netting against short-term, floored at
            # zero, plus qualified dividends — computed as `net_capital_gain` at the top of evaluate.
            #
            # ★ `qbi_cap_l12` is OTS single-witness / WEAK (I1): it is DRIVER-HAND-FED here (not
            # derived by OTS from its own Schedule D output), so OTS cannot independently catch an
            # error in it. The §14.2 closure — derive L12 from OTS's Sch-D output — is filed as a
            # follow-up (post-oracle-sweep hardening).
            qbi_cap_l12 = round(net_capital_gain)
            f8995, _ = run_form(
                "f8995",
                "Form_8995",
                "Form_8995",
                {
                    "FileName1040": p1_out.name,
                    "L1_i_a:": "Crypto",
                    "L1_i_c": round(se_profit - half_se),
                    "L12": qbi_cap_l12,
                },
                work,
            )
            qbi_deduction = f8995.get("L15", 0.0)

        # Pass 2: the filed figures. AGI is unchanged (QBI is a below-the-line deduction), so
        # the NIIT computed off pass 1's AGI still stands.
        final = (
            run_form(
                "US_1040",
                "US_1040",
                "US_1040",
                {**base, "L13": qbi_deduction},
                work,
                capgains=gains,
            )[0]
            if qbi_deduction
            else p1
        )

        # ★ §1211 fix (r2-I3 / r3-M3). OTS's own 1040 line 7 is the §1211-LIMITED signed
        # Schedule-D net: a net loss is floored at −3,000 and REDUCES net investment income,
        # exactly as btctax applies it (other_taxes.rs:219-222,308). Feed THAT to Form 8960 L5a.
        # The old `max(ltcg,0)+max(stcg,0)` dropped the limitation entirely, overstating OTS's NII
        # by up to $3,000 on a capped-loss × NIIT-firing cell → OTS's NIIT wrong by construction
        # (T11 would then file a FALSE btctax bug + pin btctax's correct value as a known defect).
        # The same §1211-limited figure widens the 8960 TRIGGER gate below, so a net-STCG-only
        # cell over the threshold no longer gets a false niit = 0 (the old `max(ltcg,0)` never saw
        # short-term gains). L13 stays pass-1 cents-MAGI → NIIT is a paper-level ±cents epsilon.
        sch_d_net = final.get("L7", 0.0)
        niit = 0.0
        investment = (
            h.get("taxable_interest", 0) + h.get("ordinary_dividends", 0) + max(sch_d_net, 0.0)
        )
        if investment:
            f8960, _ = run_form(
                "f8960",
                "Form_8960",
                "Form_8960",
                {
                    "Status": status,
                    "Entity": "Individual",
                    "Sec6013g": "No",
                    "Sec6013h": "No",
                    "Sec1141_10g": "No",
                    "L1": h.get("taxable_interest", 0),
                    "L2": h.get("ordinary_dividends", 0),
                    "L5a": sch_d_net,
                    "L13": p1.get("L11", 0.0),
                },
                work,
            )
            niit = f8960.get("L17", 0.0)

        # ── OTS's TY2024 AMT is only a witness where its two known defects cannot reach. Both were
        #    confirmed by reading its source and both are FIXED in OTS's 2025 release, so they are
        #    not upstream-reportable — but the 2024 line is closed, so we must gate rather than wait.
        #    Reporting `None` (not witnessed) uses the same Option semantics the codebase already
        #    applies to weak witnesses (`qbi_cap_l12`), instead of inventing a defect-pinning path.
        ots6251 = _form6251_lines(final)
        amt_disqualified = _ots_amt_disqualified(status, h, final, ots6251)
        amt_witnessed = None if amt_disqualified else final.get("L17", 0.0)

        salt_capped = None
        if h.get("standard_or_itemized") == "Itemized":
            # OTS's US_1040 solver runs Schedule A internally and PRINTS the §164(b)(5)-capped
            # line 5e as `A5e` (A5a/A5b ride the US_1040 input in the itemized block above), so
            # read OTS's own figure. Fall back to the driver-DERIVED min(5a + 5b, 10000) only if a
            # future template stops printing A5e (r2-N2).
            salt_capped = final.get("A5e")
            if salt_capped is None:
                salt_capped = min(
                    float(h.get("state_income_tax", 0)) + float(h.get("real_estate_tax", 0)),
                    10000.0,
                )

        return {
            "adjusted_gross_income": final.get("L11", 0.0),
            "taxable_income": final.get("L15", 0.0),
            "qbi_deduction": qbi_deduction,
            "income_tax_before_credits": final.get("L16", 0.0),
            "se_tax": se_tax,
            "niit": niit,
            "additional_medicare_tax": addl_medicare,
            # OTS's L24 already carries SE tax and additional medicare (we fed it S2_4/S2_11);
            # it does not carry NIIT, which we add, matching 1040 line 24's Schedule 2 total.
            "total_tax": final.get("L24", 0.0) + niit,
            # ── T8: deeper lines + provenance leaves + C1 cross-foot legs (§6.1 → §6.2) ──
            "deduction_taken": final.get("L12", 0.0),         # 1040 L12 — standard or itemized
            "sch_d_to_l7": sch_d_net,                         # 1040 L7 — SIGNED, §1211-limited
            "salt_capped": salt_capped,                       # Sch A L5e (None when standard)
            "qbi_cap_l12": qbi_cap_l12,                       # 8995 L12 — OTS single-witness/WEAK (I1)
            "qual_div_l3a": h.get("qualified_dividends", 0),  # 1040 L3a provenance leaf
            "net_ltcg_qd_exclusive": net_ltcg_qd_exclusive,   # §1(h) QD-EXCLUSIVE subterm (r5-N2)
            "se_l10_oasdi": se_l10_oasdi,                     # Sch SE L10 (OASDI leg)
            "se_l11_medicare": se_l11_medicare,               # Sch SE L11 (Medicare leg)
            "f8959_l7": f8959_l7,                             # 8959 L7 (Part I leg)
            "f8959_l13": f8959_l13,                           # 8959 L13 (Part II SE leg)
            # ── G-6: the AMT. Oracle 1 computes the WHOLE of Form 6251 and always did; this harness
            #    simply never read it. `taxsolve_US_1040_2024.c:222` is
            #    `form6251_AlternativeMinimumTax(int itemized)`, and it prints every line as
            #    `AMT_Form_6251_L*` before assigning `L[17] = Sched2[3]`.
            #
            #    ★ Line 2a is the branch that matters: OTS codes it
            #    `if (itemized) amtws2a = SchedA[7]; else amtws2a = L[12];` — the standard deduction
            #    added back for a non-itemizer. That is btctax's branch and the form's, and it is the
            #    one PSL Tax-Calculator omits (issue #3108). So on AMT the two oracles SPLIT, which is
            #    diagnostic rather than ambiguous, and OTS is the one that agrees with the IRS PDF.
            "amt": amt_witnessed,                            # 1040 L17; None when OTS is DISQUALIFIED
            "amt_not_witnessed_because": amt_disqualified,    # the reason, or None
            "form6251": ots6251,                              # every printed 6251 line, by number
        }
    finally:
        shutil.rmtree(work, ignore_errors=True)


def version() -> str:
    readme = OTS_DIR / "README"
    for cand in (readme, OTS_DIR / "README.txt"):
        if cand.exists():
            m = re.search(r"OpenTaxSolver.*?(\d+\.\d+)", cand.read_text()[:2000])
            if m:
                return f"OpenTaxSolver 2024 v{m.group(1)}"
    return f"OpenTaxSolver 2024 ({OTS_DIR.name})"


if __name__ == "__main__":  # offline self-check; needs no OTS install
    selftest_defect_years()
    print("ots_direct: defect-year scoping OK")
