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

def _bin(form: str) -> Path:
    p = OTS_DIR / "bin" / f"taxsolve_{form}_2024"
    if not p.exists():
        raise FileNotFoundError(f"OTS solver not found: {p}. Set OTS_DIR.")
    return p


def _template(subdir: str, name: str) -> str:
    d = OTS_DIR / "tax_form_files" / subdir
    for cand in (f"{name}_2024_template.txt", f"{name}_template.txt"):
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


def run_form(
    form: str,
    subdir: str,
    tname: str,
    values: dict[str, object],
    work: Path,
    capgains: list[str] | None = None,
) -> tuple[dict[str, float], Path]:
    """Run one OTS solver; return its parsed lines and the path of its output file."""
    template = _template(subdir, tname)
    # OTS reads a blank `YourName:` as consuming the NEXT line as its value, which then
    # derails the whole strict-order parse. Every identity field must carry something.
    identity = {"YourName:": "Golden Household", "YourSocSec#:": "000-00-0000"}
    values = {
        **{k: v for k, v in identity.items() if re.search(rf"^{re.escape(k)}", template, re.M)},
        **values,
    }
    src = work / f"{form}.txt"
    src.write_text(_fill(template, values, capgains))
    proc = subprocess.run(
        [str(_bin(form)), src.name], cwd=work, capture_output=True, text=True
    )
    out_path = work / f"{form}_out.txt"
    if not out_path.exists():
        raise RuntimeError(f"{form}: no output.\n{proc.stdout}\n{proc.stderr}")
    text = out_path.read_text()
    if "ERROR" in text:
        bad = [ln for ln in text.splitlines() if "ERROR" in ln]
        raise RuntimeError(f"{form}: OTS reported {bad}")
    return _parse(text), out_path


def evaluate(h: dict) -> dict[str, float]:
    """Compute one household's federal return by driving OTS end to end."""
    status = h.get("filing_status", "Single")
    w2 = h.get("w2_income", 0)
    se_profit = h.get("self_employment_income", 0)
    stcg = h.get("short_term_capital_gains", 0)
    ltcg = h.get("long_term_capital_gains", 0)

    work = Path(tempfile.mkdtemp(prefix="ots-"))
    try:
        se_tax = half_se = addl_medicare = 0.0

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

            f8959, _ = run_form(
                "f8959",
                "Form_8959",
                "Form_8959",
                {"Status": status, "L1": w2, "L8": round(se_profit * 0.9235)},
                work,
            )
            addl_medicare = f8959.get("L18", 0.0)
        elif w2:
            f8959, _ = run_form(
                "f8959", "Form_8959", "Form_8959", {"Status": status, "L1": w2}, work
            )
            addl_medicare = f8959.get("L18", 0.0)

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
            # zero, plus qualified dividends.
            net_capital_gain = h.get("qualified_dividends", 0) + max(
                0.0, min(float(ltcg), float(ltcg) + float(stcg))
            )
            f8995, _ = run_form(
                "f8995",
                "Form_8995",
                "Form_8995",
                {
                    "FileName1040": p1_out.name,
                    "L1_i_a:": "Crypto",
                    "L1_i_c": round(se_profit - half_se),
                    "L12": round(net_capital_gain),
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

        niit = 0.0
        investment = (
            h.get("taxable_interest", 0) + h.get("ordinary_dividends", 0) + max(ltcg, 0)
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
                    "L5a": max(ltcg, 0) + max(stcg, 0),
                    "L13": p1.get("L11", 0.0),
                },
                work,
            )
            niit = f8960.get("L17", 0.0)

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
