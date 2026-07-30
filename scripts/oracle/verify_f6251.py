#!/usr/bin/env python3
"""Cross-check btctax's Form 6251 against BOTH reference engines, line by line.

    OTS_DIR=~/OpenTaxSolver2024_22.07_linux64 .venv/bin/python scripts/oracle/verify_f6251.py

Not part of `make check` (that is the Rust suite); this is the independent-engine probe the AMT plan
called for. It reads `crates/btctax-core/src/tax/fixtures/form6251_vectors.json` and compares against:

  1. **Tax-Calculator** — `c09600` (AMT) and `c62100` (AMTI). Always available (it is in `.venv`).
  2. **OpenTaxSolver** — EVERY printed Form 6251 line (`AMT_Form_6251_L*`), when `OTS_DIR` is set.
     Skipped with a loud note otherwise, never silently.

★ THIS IS THE ONLY PLACE AMT IS VALIDATED, and it exists because the golden matrix CANNOT do it.
`gen_goldens.py`'s D-2 predicate rejects any household taxcalc sees AMT on, and btctax refuses returns
that must attach Form 6251 — so an AMT-owing household is rejected twice over and the corpus is
structurally AMT-free (measured: 0 of 104 households have nonzero AMT on either oracle). The way
around it is that `assemble_absolute` is infallible: btctax computes the whole form even for returns it
will not file, so these vectors exercise Part I through Part III on returns the corpus can never hold.

★ An earlier version of this docstring said OTS "is not installed here, and even when it is,
`ots_direct.py` never reads line 17". The second half was true and is now fixed; the first half was an
assumption. OTS computes Form 6251 in full (`taxsolve_US_1040_2024.c:222`) and always did.

★ THE STANDARD-DEDUCTION DIVERGENCE, adjudicated against the FORM (not against a second oracle).
On every STANDARD-DEDUCTION vector, taxcalc's AMTI is lower than ours by EXACTLY the standard
deduction. That is a Tax-Calculator defect, not ours (`taxcalc/calcfunctions.py`, the AMTI block):

    if standard > 0.0:
        c62100 = c00100 - e00700 - qbided - standard

It subtracts the standard deduction and never adds it back. Its itemizer branch DOES add back
Schedule A line 7 (`+ c18300`), which is why every itemizing vector agrees. But Form 6251 line 2a says:
"If filing Schedule A (Form 1040), enter the taxes from Schedule A, line 7; **otherwise, enter the
amount from Form 1040 or 1040-SR, line 12**" — i.e. add the standard deduction back. i6251 p.2 repeats
it, §56(b)(1)(D) mandates it, and i6251's own TIP turns on it ("the standard deduction isn't allowed
for the AMT"). The form is the authority; a taxcalc disagreement is adjudicated against the PDF and
never encoded. Direction: taxcalc UNDERSTATES AMT for standard-deduction filers.

★ THAT EXCUSE IS CHECKED, NOT ASSERTED. This used to be a `KNOWN_DIVERGENT = {"V4", "V5"}` name
list: a vector on the list could diverge for ANY reason and still print "KNOWN". `_amti_verdict`
now requires the ΔAMTI to be exactly the standard deduction on a standard-deduction vector and
exactly zero on an itemizing one — so a divergence of the wrong SHAPE is reported as unexpected
even on a vector whose divergence we expect, and the day taxcalc fixes #3108 this reds and tells us.
"""
import json, os, shutil, sys, tempfile, warnings, pathlib

warnings.filterwarnings("ignore")
import pandas as pd, taxcalc as tc  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
# 2024 standard deduction, keyed by the fixture's own filing-status tokens (Rev. Proc. 2023-34
# §2.15). Used to CHECK that the taxcalc divergence has the shape we claim — never to compute.
STANDARD_DEDUCTION_2024 = {
    "single": 14_600.0, "mfj": 29_200.0, "mfs": 14_600.0, "hoh": 21_900.0, "qss": 29_200.0,
}

# ★ OTS's disqualifications are COMPUTED, never listed by vector name.
# `ots_direct._ots_amt_disqualified` is the fail-closed predicate — an unrecognised shape returns a
# reason rather than silently witnessing — and it reaches both OTS 2024 defects: the stale 2023
# §55(d)(3) MFS constants (taxsolve_US_1040_2024.c:270-275) and the missing §170(b) 60%-of-AGI cash
# ceiling. Both are FIXED in OTS 2025, so neither is upstream-reportable; the 2024 line is closed, so
# we gate instead of waiting.
#
# ★ A NAME LIST WOULD HAVE GONE STALE HERE, and not hypothetically. It carried exactly {V8, V2b}.
# The E2 population then added V23/V24/V25 — MFS with the exemption phased to zero — and for MFS that
# is the SAME condition as the line-4 kicker, because §55(d)(3) puts the zero-exemption threshold and
# the kicker start at the identical $875,950 (609,350 + 4 x 66,650). All three land in OTS's
# stale-constant region. The predicate gates them; a hand-kept set would have diffed against a
# witness already known to be wrong and printed "OK".


# ── OTS's Tax-Table quantization, as a MECHANISM rather than a list of vector names ──
#
# `taxsolve_US_1040_2024.c:126` (`TaxRateFunction`): below $100,000 OTS quantizes income to the Tax
# Table's own bin, takes the tax at the bin MIDPOINT, and rounds to a whole dollar —
#
#     if (income < 100000.0) { ... x = 50.0; dx = 0.5*x; k = income/x;
#                              tx = (int)(TaxRateFormula(x*k + dx, status) + 0.5); }
#
# — while btctax applies the exact §1(j) schedule. Neither is wrong: the IRS publishes both, and
# below the ceiling the Tax Table IS the authority. Form 6251 line 10 is `1040 L16 - Sch 3 L1`, so
# the L16 difference lands there unchanged, and line 11 inherits it.
#
# ★ COMPUTED, because a name list under-covered. It held exactly one entry, ("V10","line10"). The
# E2 population then added V12/V16/V19 — the line-23-live vectors, whose whole design is a SMALL
# ordinary bottom under a large gain, which is precisely the region below the Tax Table ceiling.
# The mechanism predicts them; a name list would have called three of them defects.
OTS_TAX_TABLE_CEILING = 100_000.0   # taxsolve_US_1040_2024.c:126
OTS_TAX_TABLE_BIN = 50.0            # the widest bin, used for everything above $3,000
OTS_TOP_MARGINAL_RATE = 0.37        # §1(j)(2) — the most a half-bin of income can be worth
# Half a bin at the top rate, plus OTS's whole-dollar rounding, plus ours.
OTS_TAX_TABLE_TOLERANCE = OTS_TAX_TABLE_BIN / 2 * OTS_TOP_MARGINAL_RATE + 1.0


def _ots_tax_table_methodology(v, line: str, delta: float) -> str | None:
    """Why this line's difference is the Tax Table and not a defect — or None if it is not."""
    # Only the two lines the regular tax reaches: line 10 carries 1040 L16, line 11 subtracts it.
    if line not in ("line10", "line11"):
        return None
    # Form 6251 line 20 is the QDCGT worksheet's ordinary bottom "as figured for the regular tax" —
    # exactly the amount OTS runs through `TaxRateFunction`.
    ordinary_bottom = float(v["form6251"]["line20"])
    if ordinary_bottom >= OTS_TAX_TABLE_CEILING:
        return None
    if abs(delta) > OTS_TAX_TABLE_TOLERANCE:
        return None
    return (f"Tax-Table vs exact schedule on the {ordinary_bottom:,.0f} ordinary slice "
            f"(under the {OTS_TAX_TABLE_CEILING:,.0f} ceiling; delta {delta:+,.0f})")


# ★ EVERY line OTS prints, minus the echoes — not a hand-picked subset.
#
# An earlier draft compared 9 lines (Part I/II only) while its own docstring claimed a "field-for-field"
# diff. That was the confident-comment-covering-the-gap pattern this project keeps getting burned by,
# and worse, it discarded the one region that most needs an outside witness: **Part III's interior is
# checked against `f6251_reference.py`, a same-team twin derived from the same reading of
# `PART_III.md`** — so a shared mis-reading passes both. The historical line-33 bug (`line22 - line32`
# mis-transcribed as `line12 - line32`, worth $200,000 on one vector) is exactly that class.
#
# OTS prints Part III too, and always did: on V4 it emits lines 12, 13, 15-20, 22, 24, 25, 27, 28, 33,
# 34, 38, 39, 40 — and its line 33 independently confirms the line-22 subtraction. The witness was in
# output we were already parsing and throwing away.
#
# EXCLUDED, and only these: line 8 is the AMT foreign tax credit, which our driver FEEDS to OTS as
# `AMTws8`. Comparing it would be comparing our input to itself — an echo, not a witness.
OTS_ECHOED_LINES = {"line8"}


def _ots_lines_to_compare(got: dict, want: dict) -> list[str]:
    """Every line BOTH engines produced, minus the echoes. Order is the form's, for readable output."""
    def key(name: str):
        body = name[len("line"):]
        num = int("".join(c for c in body if c.isdigit()) or 0)
        return (num, body)
    return sorted((set(got) & set(want)) - OTS_ECHOED_LINES, key=key)


# Tax-Calculator's `MARS`, from its own documentation: 1 single, 2 joint, 3 separate, 4 household
# head, 5 widow(er). ★ This was `3 if mfs else 2` — every Single and HoH vector would have been
# scored as MFJ, which is the exact shape of the Rust-side `_ => Mfj` the E2 population also caught.
TAXCALC_MARS = {"single": 1, "mfj": 2, "mfs": 3, "hoh": 4, "qss": 5}


def _taxcalc_expected_gaps(v) -> list[tuple[float, str]]:
    """The AMTI shortfalls Tax-Calculator's KNOWN defects predict for this vector, each with a size.

    Both are omissions of a Form 6251 Part I term, both adjudicated against the PDF, and both make
    taxcalc UNDERSTATE AMTI (and so AMT). Naming the size is the point: it turns "this vector is
    allowed to disagree" into "this vector must disagree by exactly this much, for this reason".

    1. **Line 2a, the standard-deduction add-back** — PSLmodels/Tax-Calculator#3108, open.
       `calcfunctions.py` computes `c62100 = c00100 - e00700 - qbided - standard` and never adds the
       standard deduction back. Size: the filer's standard deduction.

    2. **Line 4's parenthetical, the §55(d)(3) MFS AMTI add-back** — verified by reading
       `calcfunctions.py` (taxcalc 6.7.2), not assumed. Its `c62100` block has no MFS branch at all.
       What it DOES model is the other limb of §55(d)(3): `if MARS == 3 and c62100 > AMT_em_pe:
       line5 = 0.` zeroes the MFS exemption at the cliff. The exemption limb and the AMTI-increase
       limb are different rules, and only the first is implemented.
       ★ Stated precisely because we have been imprecise here before: an earlier upstream comment of
       ours said §55(d)(3) "appears not to be modelled", which was wrong — `AMT_em_pe` models it.
       The accurate claim is narrower and survives the grep: the EXEMPTION CLIFF is modelled, the
       AMTI ADD-BACK is not. OTS implements the add-back (which is why it is our witness for it),
       just with the stale 2023 constants. Size: taken from the vector itself, as the gap between
       Form 6251 line 4 and the sum of lines 1 through 3 — no constant restated here.
    """
    f, st = v["form6251"], v["inputs"]["filing_status"]
    gaps: list[tuple[float, str]] = []
    if not v["derived"]["itemized"]:
        gaps.append((STANDARD_DEDUCTION_2024[st],
                     f"line 2a's {STANDARD_DEDUCTION_2024[st]:,.0f} standard-deduction add-back (#3108)"))
    pre_kicker = sum(float(f[k]) for k in ("line1", "line2a", "line2b", "line3"))
    kicker = float(f["line4"]) - pre_kicker
    if abs(kicker) > 0.005:
        gaps.append((kicker,
                     f"line 4's {kicker:,.2f} §55(d)(3) MFS AMTI add-back (taxcalc models the "
                     "exemption cliff, not this)"))
    return gaps


def _amti_verdict(v, their_amti: float) -> tuple[str, bool]:
    """Adjudicate taxcalc's AMTI against ours. Returns (note, is_unexpected).

    Checks the SIZE, not the vector's name: a vector that diverges by anything other than the sum of
    its predicted gaps is a real finding even if it was expected to diverge, and a vector with no
    predicted gap must agree exactly.
    """
    delta = float(v["form6251"]["line4"]) - their_amti
    gaps = _taxcalc_expected_gaps(v)
    expected = sum(g for g, _ in gaps)
    if abs(delta - expected) >= 1.0:
        return f"★ AMTI off by {delta:,.2f}, predicted {expected:,.2f}", True
    if not gaps:
        return "AMTI agrees", False
    return "AMTI short by " + " + ".join(why for _, why in gaps), False


def main() -> int:
    vectors = json.loads(
        (ROOT / "crates/btctax-core/src/tax/fixtures/form6251_vectors.json").read_text()
    )["vectors"]
    rows = []
    for n, v in enumerate(vectors):
        i = v["inputs"]
        rows.append({
            "RECID": n + 1, "FLPDYR": 2024,
            "MARS": TAXCALC_MARS[i["filing_status"]],
            "e00200": float(i["wages"]), "e00200p": float(i["wages"]), "e00200s": 0.0,
            "p23250": float(i["net_ltcg"]),
            "e19800": float(i["cash_gift"]),   # cash charitable — absent from gen_goldens' builder
            # ★ e00700 = "taxable refunds of state and local income taxes" = Schedule 1 line 1, which
            #   is exactly Form 6251 line 2b's source, and the variable taxcalc's own AMTI block
            #   subtracts. This was `e00300` (taxable INTEREST): same effect on AGI, so nothing
            #   visibly broke, but taxcalc's AMTI never saw the refund and its ΔAMTI on a line-2b
            #   vector was noise. V29 owes AMT with line 2b live and would have diverged on it.
            "e00700": float(i["state_refund"]),
            "e07300": float(i["sch3_line1_ftc"]),
            "s006": 1.0,
        })
    recs = tc.Records(data=pd.DataFrame(rows), start_year=2024, gfactors=None,
                      weights=None, adjust_ratios=None)
    calc = tc.Calculator(policy=tc.Policy(), records=recs)
    calc.advance_to_year(2024)
    calc.calc_all()
    amt, amti = calc.array("c09600"), calc.array("c62100")

    print(f"{'vec':5}{'st':7}{'btctax AMT':>13}{'taxcalc':>12}  verdict")
    bad = known = 0
    for n, v in enumerate(vectors):
        mine, theirs = float(v["form6251"]["line11"]), float(amt[n])
        amti_note, amti_bad = _amti_verdict(v, float(amti[n]))
        # The AMT itself must agree UNLESS one of taxcalc's known Part I omissions reaches this
        # vector. Both legs are checked, and neither excuses the other: a licensed vector still has
        # to be short by exactly the predicted AMTI, and an unlicensed one has to agree outright.
        if abs(mine - theirs) < 1.0:
            verdict = f"AGREE · {amti_note}"
        elif _taxcalc_expected_gaps(v):
            verdict = f"KNOWN · {amti_note}"
            known += 1
        else:
            verdict, bad = f"★ UNEXPECTED DIVERGENCE · {amti_note}", bad + 1
        if amti_bad:
            bad += 1
        print(f"{v['id']:5}{v['inputs']['filing_status']:7}{mine:>13,.2f}{theirs:>12,.2f}  {verdict}")
    print(f"\n  {'FAIL' if bad else 'OK'}: {bad} unexpected divergence(s); "
          f"{known} known and adjudicated against the form.")
    taxcalc_disqualified = {
        v["id"]: "; ".join(why for _, why in _taxcalc_expected_gaps(v))
        for v in vectors if _taxcalc_expected_gaps(v)
    }

    ots_bad, ots_disqualified, ots_ran = _ots_pass(vectors)
    bad += ots_bad
    bad += _witness_census(vectors, taxcalc_disqualified, ots_disqualified, ots_ran)
    return 1 if bad else 0


def _witness_census(vectors, taxcalc_disqualified, ots_disqualified, ots_ran) -> int:
    """How many INDEPENDENT engines actually witnessed each AMT-owing vector, and the Tier-2 gate.

    ★ WHY THIS SECTION EXISTS. The two oracle passes above each report their own disqualifications,
    and read separately they conceal the case that matters most: a vector BOTH engines are
    disqualified on. That is not hypothetical — V23/V24/V25 are MFS with the exemption phased to
    zero, which for MFS is the same condition as Form 6251 line 4's kicker, and the kicker is
    exactly what OTS 2024 gets wrong (stale 2023 constants) AND what taxcalc does not model at all.
    Two oracles, one blind spot, perfectly aligned. Nothing witnesses those three, and the run
    printed "OK" twice while that was true.

    ★ AND THE GATE. `FOLLOWUPS §G-6b` / `CONTINUITY`: *btctax may not ATTACH a Form 6251 for any
    filing status that has no AMT-owing vector agreed by two oracles.* That is a checkable rule, so
    it is checked here rather than asserted in prose — a filing status that loses its last
    two-oracle AMT-owing vector fails this run.
    """
    print("\n── witness census · the Tier-2 gate (FOLLOWUPS §G-6b) ──")
    if not ots_ran:
        print("  INCONCLUSIVE — OTS did not run, so no vector can have two witnesses this run.")
        return 0
    by_status: dict[str, list[str]] = {}
    orphans = []
    for v in vectors:
        if float(v["form6251"]["line11"]) <= 0:
            continue  # the gate is about vectors that OWE AMT
        vid, st = v["id"], v["inputs"]["filing_status"]
        witnesses = [n for n, dq in (("taxcalc", taxcalc_disqualified.get(vid)),
                                     ("OTS", ots_disqualified.get(vid))) if not dq]
        if len(witnesses) >= 2:
            by_status.setdefault(st, []).append(vid)
        elif not witnesses:
            orphans.append((vid, st))
    for st in sorted({v["inputs"]["filing_status"] for v in vectors}):
        ids = by_status.get(st, [])
        mark = "OK  " if ids else "FAIL"
        print(f"  {mark} {st:7} {len(ids)} AMT-owing vector(s) agreed by BOTH oracles"
              f"{': ' + ' '.join(ids) if ids else ''}")
    for vid, st in orphans:
        print(f"  ★ {vid} ({st}) owes AMT and NO engine can witness it — "
              f"taxcalc: {taxcalc_disqualified.get(vid)} | OTS: {ots_disqualified.get(vid)}")
    ungated = [st for st in {v["inputs"]["filing_status"] for v in vectors} if not by_status.get(st)]
    if ungated:
        print(f"\n  FAIL: {', '.join(sorted(ungated))} may not ATTACH Form 6251 — no AMT-owing "
              "vector agreed by two oracles.")
        return 1
    print(f"\n  OK: every filing status in the fixture clears the Tier-2 attach gate; "
          f"{len(orphans)} unwitnessed AMT-owing vector(s) are recorded above, not counted.")
    return 0


def _ots_pass(vectors) -> tuple[int, dict[str, str], bool]:
    """Compare every line BOTH engines produce (minus echoes) against OpenTaxSolver.

    Returns (unexpected-diff count, {vector id: why OTS cannot witness it}, did-OTS-run).

    Absent `OTS_DIR` this prints a loud SKIP and returns 0 — a missing oracle is a gap in coverage, not
    a pass, and saying so is the whole point of the two-oracle standard.
    """
    print("\n── oracle 1 · OpenTaxSolver, every printed Form 6251 line ──")
    if not os.environ.get("OTS_DIR"):
        print("  SKIPPED — OTS_DIR is unset, so AMT rests on ONE oracle for this run.")
        print("  Install: https://sourceforge.net/projects/opentaxsolver/files/OTS_2024/")
        return 0, {}, False

    sys.path.insert(0, str(ROOT / "scripts" / "oracle"))
    import ots_direct as o  # noqa: E402 — deliberately late; needs OTS_DIR at import time

    # ★ Runs BEFORE any solver: each OTS defect must gate the years that carry it and only those.
    #   A year-blind predicate would report TY2025's MFS-kicker vectors "not witnessed" against a
    #   solver that handles them correctly, and this run would still print OK.
    o.selftest_defect_years()

    # OTS's own status vocabulary, matched by `strncasecmp` in taxsolve_US_1040_2024.c:1874-1878.
    # ★ A token OTS does not recognise yields ALL ZEROS rather than an error, which reads exactly
    #   like a broken install — hence a dict that KeyErrors on an unknown status rather than a
    #   default that would quietly score a Single filer as something else.
    status_token = {"single": "Single", "mfj": "Married/Joint", "mfs": "Married/Sep",
                    "hoh": "Head_of_House", "qss": "Widow(er)"}
    print(f"  {'vec':5} {'line':8} {'OTS':>14} {'btctax':>14}   verdict")
    bad = compared = methodology = 0
    disqualified: dict[str, str] = {}
    for v in vectors:
        vid, i, d, want = v["id"], v["inputs"], v["derived"], v["form6251"]
        work = pathlib.Path(tempfile.mkdtemp(prefix="ots-f6251-"))
        try:
            vals = {
                "Status": status_token[i["filing_status"]],
                "L1a": float(i["wages"]), "L2b": 0, "L3a": 0, "L3b": 0,
                "S1_1": float(i["state_refund"]), "S1_3": 0, "S1_15": 0,
                "S2_4": 0, "S2_11": 0, "S3_1": float(i["sch3_line1_ftc"]),
                # ★ Form 6251 line 8 is an INPUT to OTS, not something it derives: i6251 has the filer
                #   compute the AMT foreign tax credit on a separate AMT Form 1116. btctax carries the
                #   §904(j) de-minimis election through unchanged, so the AMT FTC equals the regular
                #   one. Omitting this left OTS's line 9 high by exactly the credit.
                "AMTws8": float(i["sch3_line1_ftc"]),
            }
            if d["itemized"]:
                # A11 is cash/check charity — NOT A16 ("other"), which sails past Schedule A's own
                # handling of the gift. The vectors' only itemized component is the cash gift.
                vals |= {"A5a": 0, "A5b": 0, "A8a": 0, "A11": float(i["cash_gift"])}
            parsed, _ = o.run_form("US_1040", "US_1040", "US_1040", vals, work,
                                   capgains=o._capgains_rows(0.0, float(i["net_ltcg"])))
            got = o._form6251_lines(parsed)
        finally:
            shutil.rmtree(work, ignore_errors=True)
        # ★ COMPUTED, fail-closed. `_ots_amt_disqualified` also returns a reason when OTS printed no
        #   Form 6251 at all, so the "not witnessed" case and the "witnessed by a solver with a known
        #   defect in this region" case go through one predicate instead of a dict of vector names.
        why_not = o._ots_amt_disqualified(
            vals["Status"], {"charitable_cash": float(i["cash_gift"])}, parsed, got
        )
        if why_not:
            disqualified[vid] = why_not
            print(f"  {vid:5} {'—':8} {'not witnessed':>14} {'':>14}   {why_not}")
            continue
        for ln in _ots_lines_to_compare(got, want):
            a, b = round(float(got[ln])), round(float(want[ln]))
            compared += 1
            if abs(a - b) <= 1:
                continue
            why = _ots_tax_table_methodology(v, ln, a - b)
            if why:
                methodology += 1
                print(f"  {vid:5} {ln:8} {a:>14,} {b:>14,}   METHODOLOGY \u2014 {why}")
            else:
                bad += 1
                print(f"  {vid:5} {ln:8} {a:>14,} {b:>14,}   \u2605 UNEXPECTED")
    print(f"\n  {'FAIL' if bad else 'OK'}: {compared} lines compared against OTS, {bad} unexpected, "
          f"{methodology} methodology; {len(disqualified)} vector(s) not witnessed.")
    return bad, disqualified, True


if __name__ == "__main__":
    sys.exit(main())
