#!/usr/bin/env python3
"""The oracle-sweep LIVE, NON-CI divergence sweep (SPEC §5.2 / §9, plan T12).

★ What this is — a discovery mechanism, never a gate
----------------------------------------------------
`make check` is HERMETIC: it holds btctax's filled PDF against the BAKED double-oracle corpus
(`full_return_goldens.json`), offline, deterministically. That corpus is a finite covering array —
it proves the interaction SPACE, but it is a fixed set of points. This sweep is the complement: a
seeded, **threshold-biased** generator that hunts for btctax-vs-oracle divergences on scenarios the
baked corpus does not cover — the rounding/edge cases that hide right on a tax threshold. It runs
BOTH live oracles (OpenTaxSolver + PSL Tax-Calculator) and btctax, diffs the full line set, and
emits a paste-ready divergence report for anything that does not reconcile.

It is **never** part of `make check`: it needs the OTS binaries and the `taxcalc` venv, and it is
non-deterministic across seeds. Run it by hand:

    export OTS_DIR=/path/to/OpenTaxSolver2024_22.07_linux64
    cargo build -p btctax-oracle-harness          # the §9 harness this drives
    .venv/bin/python scripts/oracle/sweep.py --year 2024 --seed 1 --count 50

★ I4 (MANDATORY) — the sweep NEVER re-implements btctax's arithmetic in Python
------------------------------------------------------------------------------
Python's built-in `round()` is BANKER'S rounding (round-half-to-even): it drifts from btctax's
half-UP `round_dollar` on any `.50`, exactly the boundary this sweep biases toward. So the sweep
does NOT re-implement `round_dollar`, the IRS Tax Table, or the QDCGT worksheet. It drives the
compiled §9 harness `--check` mode (`crates/btctax-oracle-harness`) for BOTH the btctax on-paper
values AND the reproduction + per-line classification — all rounding / Table / QDCGT logic stays in
Rust, reached over a JSON stdin/stdout contract. The only comparisons this file makes are exact
integer/string equalities on whole-dollar values the harness already rounded.

★ Divergence lifecycle (SPEC §10) — the sweep DISCOVERS, a human ADJUDICATES
---------------------------------------------------------------------------
Every divergence the sweep surfaces is triaged into exactly one of four causes (§6.4 / T11 step 3):

  (i)   corpus/steering error       → fix the generator here (a draw the domain constraints missed).
  (ii)  a genuine btctax fill/compute bug
                                     → ★ NEWS: report it to the user for adjudication FIRST. It is
                                       NOT auto-fixed and NOT auto-filed. Once adjudicated, file a
                                       `FOLLOWUPS.md` entry (severity + owning phase, STANDARD_WORKFLOW
                                       §4) and PROMOTE the scenario into the baked corpus — promotion
                                       is what creates the `KnownDefect` pin (declared in
                                       `golden_returns.rs`/`golden_packet.rs`, or passed to `--check`
                                       via `--known-defect 1040.line16=<value>@<fu-id>`), so
                                       `make check` stays green with the bug tracked, never silently
                                       tolerated. This sweep DOES NOT fix btctax (frozen) and DOES NOT
                                       auto-file — it prints the report and the triage guidance.
  (iii) oracle-driver/extraction bug → fix `ots_direct.py` / `gen_goldens.py` (never a false btctax pin).
  (iv)  lawful epsilon               → a Σround≠roundΣ / cents-MAGI residual on a `round_leaf`-of-a-
                                       non-leaf line (QBI L15, NIIT L17); §10 triage, never a class.

The harness `--check` already absorbs the LAWFUL §6.4 classes (methodology / per-oracle provenance)
and known-defect pins, so a line it reports `reconciled: false` is by construction NOT one of those —
it is an UNDECLARED divergence needing triage. A clean run reports "0 undeclared divergences".

★ Already-filed known defects (SPEC §10 suppression)
----------------------------------------------------
`KNOWN_DEFECTS` (below) is the sweep-side registry of divergences already filed as follow-ups and
pinned. It is EMPTY today (T11 re-baked the full 104-household corpus GREEN — no btctax bug). When a
bug is filed and its scenario promoted, add an entry so a re-discovery is labelled `KNOWN DEFECT →
<fu-id>` (suppressed, passed to `--check` via `--known-defect`) instead of a fresh alarm. A divergence
that matches no entry is UNDECLARED and must be triaged.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

try:
    import corpus  # the axis definitions + D-2/D-3 domain constraints we reuse
    import gen_goldens  # taxcalc_run + _taxcalc_amt_credits (batched) + the §9 harness driver
    import ots_direct  # the live OTS oracle
except ImportError as e:  # pragma: no cover
    sys.exit(f"sweep.py must sit beside corpus.py / gen_goldens.py / ots_direct.py: {e}")


# ── The tax thresholds this sweep biases toward (SPEC §5.2) ────────────────────────────────────────
# A grid STEPS OVER these edges; a threshold-biased random draw lands ON them, where the printed-chain
# rounding and the Tax-Table $50 bins can hide an off-by-a-dollar bug.
#
# ★★★ FR-164 — THESE USED TO BE MODULE-LEVEL TY2024 LITERALS WITH NO YEAR ANYWHERE IN THE FILE.
# `SALT_CAP = 10_000`, `OASDI_BASE = 168_600`, `STD_DEDUCTION = corpus.STD_DEDUCTION_2024` and
# `QBI_8995_CEILING` are every one of them a TY2024 figure, and `sweep.py` had no `--year`, passed no
# year to either oracle, and named no year in its output. So the answer to *"which tax year did this
# sweep hunt in?"* was four unlabelled constants — and OBBBA moved two of them (the §164(b) cap goes
# $10,000 → $40,000 at TY2025), which would have quietly turned the `salt_cap_edge` theme into a draw
# nowhere near an edge while the run still reported "0 undeclared divergences".
#
# ★★ UNINDEXED STATUTE STAYS A CONSTANT, and says so. Two of the six are fixed dollar amounts in the
# statute rather than inflation-adjusted figures, so keying them by year would invent a variation the
# Code does not have — the honest form is to state which is which.
SCH_B_TRIGGER = 1_500  # §6012 / 1040 Schedule B trigger — a STATUTORY amount, never indexed
# The $200k/$250k Additional-Medicare (§3101(b)(2)) AND NIIT (§1411) MAGI thresholds, by status.
# STATUTORY and explicitly NOT indexed (§1411(b), §3101(b)(2) name the dollar figures outright).
ADDL_MEDICARE_NIIT = {"Single": 200_000, "Married/Joint": 250_000}

STATUSES = ["Single", "Married/Joint"]

# ── The YEAR-DEPENDENT thresholds, keyed by year, refusing an unknown one ──────────────────────────
#
# ★ Modelled on `corpus.salt_for(year)` (the pattern this file was told to copy): a year with no entry
#   RAISES, naming what a new year must supply, instead of silently reusing the last one's figures.
# ★ The §164(b) cap is READ FROM `corpus.SALT_CAP_BY_YEAR` rather than retyped — the corpus already
#   holds it per year, with a straddle guard, and a second copy here is exactly the list that goes
#   stale beneath a set that grew.
_YEAR_THRESHOLDS = {
    2024: {
        # §63(c) standard deduction — the itemizing-wins crossover. ONE definition, in corpus.py.
        "std_deduction": corpus.STD_DEDUCTION_2024,
        # §1402(b)(1)/§3121 OASDI wage base — Sch SE L8a absorbs the band.
        "oasdi_base": 168_600,
        # §199A simple-Form-8995 taxable-income ceiling. ABOVE it btctax REFUSES (Form 8995-A is out
        # of the sweep's domain, D-2), so an SE draw must keep earned income well under it. The
        # compiled harness's refusal screen is the authoritative D-2 gate; this is the generator-side
        # bias that keeps most SE draws admissible instead of wasting oracle runs on refusals.
        "qbi_8995_ceiling": {"Single": 191_950, "Married/Joint": 383_900},
    },
}


def thresholds_for(year: int) -> dict:
    """The threshold figures this sweep biases toward, for `year`. Refuses a year it has none for.

    ★ B1 kill: `--selftest` claim 1. PLANT: give this a fallback to 2024 and it reds.
    """
    try:
        thr = dict(_YEAR_THRESHOLDS[year])
    except KeyError:
        raise KeyError(
            f"no threshold table for TY{year}. Add one to `_YEAR_THRESHOLDS` with THAT year's §63(c) "
            f"standard deduction, §1402(b)(1) OASDI wage base and §199A simple-8995 ceiling. There is "
            f"no fallback: reusing another year's edges makes every threshold-biased theme draw away "
            f"from the edge while the run still reports '0 undeclared divergences'."
        ) from None
    try:
        thr["salt_cap"] = corpus.SALT_CAP_BY_YEAR[year]
    except KeyError:
        raise KeyError(
            f"corpus.SALT_CAP_BY_YEAR has no §164(b) cap for TY{year}; add it there (with its axis) "
            f"rather than typing a second copy here."
        ) from None
    return thr


# ── The sweep-side known-defect registry (SPEC §10 suppression) — EMPTY today ──────────────────────
# Each entry: {"line": "1040.line16", "btctax_value": <int whole dollars>, "fu_id": "<FU-…>",
#              "match": <callable(inputs_dict) -> bool>}. When a scenario matches an entry, the pin is
# routed to the harness `--check` via `--known-defect` (see `_known_defect_arg`), and the harness
# ADJUDICATES it in Rust (I4): a matching pin comes back `reconciled` with class `known-defect` (a
# SUPPRESSED known defect — logged, not undeclared); a STALE pin comes back red and IS reported. Only
# `1040.line16` is supportable (the sole class/stacking line the harness pins). Populate this ONLY after
# a bug is adjudicated, filed, and pinned; EMPTY today because T11 re-baked the corpus green.
KNOWN_DEFECTS: list[dict] = []


# ── The seeded, threshold-biased scenario generator (SPEC §5.2) ────────────────────────────────────
def _near(rng: random.Random, center: int, spread: int) -> int:
    """A draw clustered ON a threshold: `center ± U(0, spread)`, floored at 0. Small `spread` keeps the
    draw tight to the edge (where rounding bugs hide); the caller widens it for coverage variety."""
    return max(0, center + rng.randint(-spread, spread))


def _split_salt(rng: random.Random, total: int) -> tuple[int, int]:
    """Split a SALT total into (state income tax 5a, real estate tax 5b) so the §164(b)(5) cap is
    exercised on the SUM, not a lump (the components must reach the engines separately)."""
    a = rng.randint(0, total)
    return a, total - a


def _spice(rng: random.Random, inp: dict) -> None:
    """Sprinkle a little independent secondary income so a themed scenario also varies OFF its own axis
    (broadens coverage without leaving the domain). Never adds a Schedule-A/SE field (those interact
    with D-2/D-3 constraints and are set only by the themes that own them)."""
    if "taxable_interest" not in inp and rng.random() < 0.35:
        inp["taxable_interest"] = _near(rng, SCH_B_TRIGGER, 1_400)
    if "ordinary_dividends" not in inp and rng.random() < 0.30:
        qd = rng.randint(0, 6_000)
        inp["ordinary_dividends"] = qd + rng.randint(0, 3_000)
        inp["qualified_dividends"] = qd
    if not any(k in inp for k in ("short_term_capital_gains", "long_term_capital_gains")) and rng.random() < 0.30:
        shape = rng.choice(["LT", "ST", "loss"])
        if shape == "LT":
            inp["long_term_capital_gains"] = rng.randint(1_000, 40_000)
        elif shape == "ST":
            inp["short_term_capital_gains"] = rng.randint(1_000, 20_000)
        else:
            inp["short_term_capital_gains"] = -rng.randint(1_000, 25_000)


def _itemize(rng: random.Random, inp: dict, status: str, itemized_total_target: int, salt_total: int,
             thr: dict) -> None:
    """Attach a Schedule A sized so itemizing WINS (D-3): the itemized total STRICTLY exceeds the
    standard deduction. `salt_total` is the pre-cap 5a+5b sum; the mortgage is sized so
    mortgage + min(salt_total, cap) ≈ `itemized_total_target` (kept ≥ STD + $1 by the caller).
    `thr` is the run's year's thresholds (`thresholds_for`) — the §164(b) cap is that year's."""
    state, realest = _split_salt(rng, salt_total)
    capped_salt = min(salt_total, thr["salt_cap"])
    mortgage = max(0, itemized_total_target - capped_salt)
    inp["state_income_tax"] = state
    inp["real_estate_tax"] = realest
    inp["mortgage_interest"] = mortgage
    # Read only by the Python oracles to force their Schedule-A path (ignored by btctax's GoldenInputs).
    inp["standard_or_itemized"] = "Itemized"


def _gen_scenario(rng: random.Random, thr: dict) -> tuple[dict, str]:
    """Draw ONE threshold-biased scenario. Returns (inputs, theme). Honors the domain by construction as
    far as it can (SE⇒low W-2; itemizing-wins); the authoritative D-2/AMT/credit gates run downstream.

    `thr` is the run's year's threshold table — every edge this draw aims at comes from it (FR-164),
    so a sweep can no longer be biased toward one year's thresholds while claiming another's."""
    theme = rng.choice(
        ["sch_b_edge", "salt_cap_edge", "std_crossover", "addl_medicare_edge", "se_oasdi_edge", "niit_edge", "capital_shapes", "broad"]
    )
    status = rng.choice(STATUSES)
    inp: dict = {"filing_status": status}

    if theme == "sch_b_edge":
        inp["w2_income"] = rng.randint(35_000, 95_000)
        inp["taxable_interest"] = _near(rng, SCH_B_TRIGGER, 400)  # tight on the $1,500 trigger

    elif theme == "salt_cap_edge":
        inp["w2_income"] = rng.randint(60_000, 160_000)
        salt_total = _near(rng, thr["salt_cap"], 800)  # tight on the cap (straddles both sides)
        # Mortgage clears STD comfortably so itemizing wins regardless of the cap outcome.
        target = thr["std_deduction"][status] + rng.randint(6_000, 20_000)
        _itemize(rng, inp, status, target, salt_total, thr)

    elif theme == "std_crossover":
        inp["w2_income"] = rng.randint(40_000, 120_000)
        salt_total = rng.randint(2_000, 8_000)
        # Itemized total JUST above the standard deduction (D-3 from the winning side, near the crossover).
        target = thr["std_deduction"][status] + rng.randint(1, 1_500)
        _itemize(rng, inp, status, target, salt_total, thr)

    elif theme == "addl_medicare_edge":
        # Medicare wages (= box 1 here) tight on the $200k/$250k Additional-Medicare threshold (§3101(b)(2)).
        inp["w2_income"] = _near(rng, ADDL_MEDICARE_NIIT[status], 2_500)

    elif theme == "se_oasdi_edge":
        # SE⇒W-2 must stay low for §199A (D-2), so probe the OASDI wage base with MFJ headroom: a W-2 at
        # the year's wage base fills the OASDI band (Sch SE L8a), and a modest SE profit rides on top
        # (8959 Part II).
        status = "Married/Joint"
        inp["filing_status"] = status
        inp["w2_income"] = _near(rng, thr["oasdi_base"], 3_000)
        inp["self_employment_income"] = rng.randint(15_000, 60_000)

    elif theme == "niit_edge":
        # Push MAGI (wages + investment income) tight on the NIIT threshold with real net investment income.
        base = ADDL_MEDICARE_NIIT[status]
        inp["w2_income"] = rng.randint(int(base * 0.55), int(base * 0.85))
        inp["taxable_interest"] = rng.randint(1_000, 8_000)
        qd = rng.randint(2_000, 12_000)
        inp["ordinary_dividends"] = qd + rng.randint(0, 4_000)
        inp["qualified_dividends"] = qd
        inp["long_term_capital_gains"] = rng.randint(10_000, 90_000)

    elif theme == "capital_shapes":
        inp["w2_income"] = rng.randint(45_000, 130_000)
        shape = rng.choice(["LT", "ST", "loss", "both"])
        if shape in ("LT", "both"):
            inp["long_term_capital_gains"] = rng.randint(2_000, 60_000)
        if shape in ("ST", "both"):
            inp["short_term_capital_gains"] = rng.randint(2_000, 30_000)
        if shape == "loss":
            inp["short_term_capital_gains"] = -rng.randint(4_000, 25_000)  # §1211 cap territory

    else:  # broad — a wide draw for coverage
        inp["w2_income"] = rng.randint(20_000, 260_000)
        if rng.random() < 0.4:
            inp["self_employment_income"] = rng.randint(10_000, 70_000)
            inp["w2_income"] = rng.randint(0, 40_000)  # SE ⇒ keep W-2 low (D-2 / §199A)

    _spice(rng, inp)

    # ── Domain guards mirroring corpus.py's constraints (belt-and-suspenders; the harness/taxcalc gates
    #    downstream are authoritative). ────────────────────────────────────────────────────────────────
    # SE present ⇒ keep earned income under the §199A simple-8995 ceiling so btctax does not refuse (D-2).
    if inp.get("self_employment_income", 0) > 0:
        earned = inp.get("w2_income", 0) + inp["self_employment_income"] * 0.9235
        if earned > thr["qbi_8995_ceiling"][status] * 0.85:
            inp["w2_income"] = min(inp.get("w2_income", 0), 40_000)
    # At least one income source (corpus.py's no-all-none rule).
    if not corpus._has_income(inp):
        inp["w2_income"] = inp.get("w2_income", 0) + rng.randint(20_000, 60_000)

    return inp, theme


def generate(seed: int, count: int, year: int) -> list[dict]:
    """The K seeded, threshold-biased scenarios (reproducible from `seed`). Each is a
    `{name, why, inputs, theme}` dict; `name` encodes the seed+index so a report is reproducible.

    `year` selects the threshold table every draw is biased toward (FR-164) — required, because the
    edges ARE the sweep and last year's edges are not this year's."""
    thr = thresholds_for(year)
    rng = random.Random(seed)
    out = []
    for i in range(count):
        inp, theme = _gen_scenario(rng, thr)
        out.append(
            {
                "name": f"sweep_s{seed}_i{i:04d}",
                "why": f"live sweep seed {seed} #{i} [{theme}]: threshold-biased draw (SPEC §5.2)",
                "theme": theme,
                "inputs": inp,
            }
        )
    return out


# ── The per-scenario admission + live diff (SPEC §6 / §9) ──────────────────────────────────────────
def _harness_check(household: dict, known_defect: str | None = None) -> dict:
    """Drive the §9 harness `--check`: btctax's on-paper values + the reproduction + per-line
    classification, ALL in Rust (I4). When `known_defect` is given (`1040.line16=<value>@<fu-id>`) the
    harness adjudicates the §10 pin ITSELF — reconciled with class `known-defect` while btctax still
    prints the pinned wrong value, red for a stale pin. Returns the parsed verdict object, or
    `{"malformed": …}` on exit-code 2 (a stdin the harness could not parse); the CALLER decides whether
    that is fatal."""
    args = [str(gen_goldens.HARNESS_BIN), "--check"]
    if known_defect:
        args += ["--known-defect", known_defect]
    proc = subprocess.run(args, input=json.dumps(household), capture_output=True, text=True)
    if proc.returncode == 2:
        return {"malformed": True, "stderr": proc.stderr.strip()}
    if proc.returncode != 0:  # pragma: no cover — a harness crash is a sweep bug, surface it loudly
        raise RuntimeError(f"oracle_harness --check exited {proc.returncode}: {proc.stderr.strip()}")
    return json.loads(proc.stdout)


def _known_defect_arg(inputs: dict) -> str | None:
    """The `--known-defect 1040.line16=<value>@<fu-id>` argument for a scenario matching an already-filed
    §10 pin (SPEC §10 suppression), or None. This only SELECTS which pin applies — the harness `--check`
    ADJUDICATES it in Rust (I4). Only `1040.line16` is supportable (the sole class/`stacking_ok` line the
    harness pins); a non-L16 `KNOWN_DEFECTS` entry is a configuration error (its pin lives in the golden
    test at promotion, not on `--check`) and fails loud."""
    for kd in KNOWN_DEFECTS:
        if kd["match"](inputs):
            if kd["line"] != "1040.line16":
                raise RuntimeError(
                    f"KNOWN_DEFECTS entry {kd['fu_id']} is on {kd['line']}, but the sweep can only route a "
                    "1040.line16 pin through --check; a non-L16 pin is declared in the golden test."
                )
            return f"{kd['line']}={kd['btctax_value']}@{kd['fu_id']}"
    return None


GOLDEN_MATRIX = (
    Path(__file__).resolve().parents[2] / "crates/btctax-core/tests/goldens/full_return_goldens.json"
)


def verify_year_matches_corpus(year: int) -> None:
    """★★ FR-164 — `--year` must agree with the BAKED CORPUS's own `_provenance.tax_year`.

    The compiled harness this sweep drives carries btctax's side, and its tax year is fixed at
    `crates/btctax-oracle-harness/src/main.rs`'s `const YEAR` — the same year the corpus was baked
    for. So btctax answers for the corpus's year whatever `--year` says. Left unchecked, `--year 2026`
    would bias every draw at TY2026 edges and score them against a TY2024 btctax and a TY2026
    taxcalc: manufactured divergences, in a tool whose entire output is divergences.

    Pure — no OTS, no harness binary — and therefore run BEFORE either is required, so the operator
    learns the year is wrong instead of being told to install a solver first.

    ★ B1 kill: `--selftest` claim 5. PLANT: delete this call from `run_sweep` and it reds.
    """
    baked_year = json.loads(GOLDEN_MATRIX.read_text())["_provenance"]["tax_year"]
    if int(baked_year) != int(year):
        sys.exit(
            f"--year {year} disagrees with the baked corpus's _provenance.tax_year ({baked_year}). "
            f"btctax's side of this sweep comes from the compiled harness, which is fixed at the "
            f"corpus's year (crates/btctax-oracle-harness/src/main.rs `const YEAR`) — so btctax would "
            f"answer for TY{baked_year} while the draws and oracle 2 answered for TY{year}, and every "
            f"divergence reported would be that mismatch. Run with --year {baked_year}, or port the "
            f"harness and regenerate the corpus first."
        )


def _verify_harness_freshness(year: int) -> None:
    """Build-freshness gate — prove the harness binary is T7-m1-fresh (emits `reproduction_ok`) BEFORE
    spending oracle time. A binary built before T7-m1 lacks the field; defaulting a missing key to a pass
    would silently disable the structural witness in a DISCOVERY tool, so we fail loud and early with a
    rebuild instruction instead. Probes `--check` on the refusal-free floor anchor.

    ★ The `--year` half of this gate is [`verify_year_matches_corpus`], which runs FIRST because it
      needs neither OTS nor the harness binary.
    """
    matrix = json.loads(GOLDEN_MATRIX.read_text())
    probe = next(h for h in matrix["households"] if h["name"] == "single_w2_only_standard")
    chk = _harness_check(probe)
    if chk.get("malformed") or "reproduction_ok" not in chk:
        sys.exit(
            f"{gen_goldens.HARNESS_BIN} is STALE — its --check output carries no `reproduction_ok` "
            "(built before T7-m1). Rebuild it: `cargo build -p btctax-oracle-harness`."
        )


def _report_divergence(scenario: dict, seed: int, index: int, verdict: dict, injected: bool,
                       year: int) -> None:
    """Emit ONE paste-ready divergence report (SPEC §9): the scenario as a household dict, the disagreeing
    line, oracle-1 (OTS) / oracle-2 (taxcalc) / btctax-on-paper, and the seed+index to reproduce."""
    banner = " [INJECTED SELF-TEST]" if injected else ""
    print(f"\n================ DIVERGENCE (seed {seed}, index {index}){banner} ================")
    print(f"  theme: {scenario['theme']}")
    print("  scenario (paste-ready household inputs):")
    print("    " + json.dumps(scenario["inputs"]))
    print(f"  line: {verdict['line']}  ({verdict['label']})")
    print(f"    oracle-1 (OTS):      {verdict.get('ots')}")
    print(f"    oracle-2 (taxcalc):  {verdict.get('taxcalc')}")
    print(f"    btctax-on-paper:     {verdict.get('on_paper')}   (btctax-internal {verdict.get('internal')})")
    print(f"    class: {verdict.get('class')}   reconciled: {verdict.get('reconciled')}")
    print(f"  reproduce: sweep.py --year {year} --seed {seed} --count {index + 1}   "
          f"(scenario index {index})")
    if injected:
        print("  NOTE: this is the --inject-divergence SELF-TEST (an oracle figure was perturbed on purpose)")
        print("        to prove the sweep surfaces a report; it is NOT a real btctax finding.")
        return
    print("  TRIAGE (SPEC §10) — categorize into exactly one cause, then act:")
    print("    (i)  corpus/steering error         → fix the generator draw in sweep.py")
    print("    (ii) GENUINE btctax fill/compute bug → ★ STOP: report to the user for adjudication FIRST;")
    print("         do NOT auto-fix (btctax is frozen) and do NOT auto-file. Once adjudicated: file a")
    print("         FOLLOWUPS.md entry (severity + owning phase) and PROMOTE this scenario into the baked")
    print("         corpus — promotion creates the KnownDefect pin (golden test, or `--check")
    print("         --known-defect 1040.line16=<value>@<fu-id>`) so make check stays green with the bug tracked.")
    print("    (iii) oracle-driver/extraction bug  → fix ots_direct.py / gen_goldens.py (never a false btctax pin)")
    print("    (iv) lawful epsilon                 → §10 triage (Σround≠roundΣ / cents-MAGI residual), never a class")


def run_sweep(seed: int, count: int, year: int, inject: bool, verbose: bool) -> int:
    """Generate K scenarios, admit (D-2 refusal-free + AMT/credit-free), live-diff each admitted one, and
    report undeclared divergences. Returns the number of UNDECLARED divergences (0 = a clean run).

    `year` is REQUIRED and reaches all three engines: the threshold table the draws aim at, oracle 1's
    solver year, oracle 2's `Records` start year, and the check against the baked corpus's own label
    (FR-164). Nothing here may default it."""
    verify_year_matches_corpus(year)  # cheapest and most likely to be wrong — checked first
    if not os.environ.get("OTS_DIR") or not ots_direct.OTS_DIR.exists():
        sys.exit(f"set OTS_DIR to an unpacked OpenTaxSolver{year} tree (see ots_direct.py).")
    if not gen_goldens.HARNESS_BIN.exists():
        sys.exit(f"{gen_goldens.HARNESS_BIN} not found — build it: `cargo build -p btctax-oracle-harness`.")
    _verify_harness_freshness(year)  # year vs the baked corpus, then T7-m1 freshness — both fail loud

    scenarios = generate(seed, count, year)
    all_inputs = [s["inputs"] for s in scenarios]

    # Batch oracle-2 (taxcalc) once: AMT/credit admission probe + the full expected dict (vectorized —
    # one Calculator pass each, not per-scenario).
    amt_credits = gen_goldens._taxcalc_amt_credits(all_inputs, year)
    taxcalc_full = gen_goldens.taxcalc_run(all_inputs, year)

    admitted = skipped = undeclared = suppressed = 0
    injected_done = False

    for idx, scenario in enumerate(scenarios):
        inputs = scenario["inputs"]
        amt, credits = amt_credits[idx]

        # D-2 / AMT / credit admission (SPEC §4), the authoritative gates:
        if amt or credits:
            skipped += 1
            if verbose:
                print(f"[skip {idx}] taxcalc AMT={amt} credits={credits} — not L24-comparable", file=sys.stderr)
            continue
        hv = gen_goldens._harness_default(inputs)
        if hv.get("malformed"):
            skipped += 1
            if verbose:
                print(f"[skip {idx}] harness rejected shape: {hv['stderr'][:80]}", file=sys.stderr)
            continue
        if hv["refused"]:
            skipped += 1
            if verbose:
                print(f"[skip {idx}] btctax REFUSED (AMT screen / §199A over-threshold / unmodeled) — D-2", file=sys.stderr)
            continue
        lines = hv["lines"]
        if int(lines.get("1040.line17", "0")) or int(lines.get("1040.line21", "0")):
            skipped += 1
            if verbose:
                print(f"[skip {idx}] btctax paper AMT/credit line present — not L24-comparable", file=sys.stderr)
            continue

        # Admitted — run oracle-1 (OTS) live and diff via the §9 harness `--check` (I4: all btctax
        # arithmetic + classification stays in Rust).
        admitted += 1
        expected_ots = ots_direct.evaluate(inputs, year=year)
        expected_taxcalc = taxcalc_full[idx]

        injected = inject and not injected_done
        if injected:
            # SELF-TEST (plan T12 step 2): perturb ONE oracle figure so btctax's on-paper L16 no longer
            # matches it, proving the sweep surfaces a divergence report end-to-end.
            expected_ots = dict(expected_ots)
            expected_ots["income_tax_before_credits"] = expected_ots["income_tax_before_credits"] + 1_234.0
            injected_done = True

        household = {
            "name": scenario["name"],
            "why": scenario["why"],
            "inputs": inputs,
            "expected_ots": expected_ots,
            "expected_taxcalc": expected_taxcalc,
        }
        # Route any already-filed §10 known-defect pin THROUGH the harness so it is adjudicated in Rust
        # (I4) — never on the injected self-test (its perturbation is not a real, filed defect).
        pin = None if injected else _known_defect_arg(inputs)
        chk = _harness_check(household, known_defect=pin)

        # ★ A discovery tool must NEVER count a shape it could not evaluate as clean. A `--check` that
        # malforms or refuses an ALREADY-ADMITTED scenario is inconsistent with the default-mode gate that
        # just admitted it — a harness build/logic bug — and a missing `reproduction_ok` means the binary
        # is stale (T7-m1 not built). All three FAIL LOUD, naming the scenario, rather than silently pass.
        if chk.get("malformed"):
            raise RuntimeError(
                f"oracle_harness --check rejected admitted scenario {scenario['name']!r} "
                f"(inputs {json.dumps(inputs)}): {chk['stderr']}"
            )
        if chk.get("refused"):
            raise RuntimeError(
                f"oracle_harness --check REFUSED admitted scenario {scenario['name']!r} — inconsistent "
                "with the default-mode admission gate that admitted it (a harness build/logic bug)."
            )
        if "reproduction_ok" not in chk:
            raise RuntimeError(
                f"oracle_harness --check emitted no `reproduction_ok` for {scenario['name']!r} — the "
                "binary is STALE (pre-T7-m1). Rebuild: `cargo build -p btctax-oracle-harness`."
            )

        found_here = False
        if not chk["reproduction_ok"]:
            # The Part-1 structural witness failed (T7-m1): btctax's own table_l16 did not reproduce its
            # own regular tax — a real reproduction/Table-semantics signal.
            print(f"\n================ DIVERGENCE (seed {seed}, index {idx}) — reproduction_ok=FALSE ================")
            print(f"  scenario: {json.dumps(inputs)}")
            print("  btctax's own table_l16 did NOT reproduce its filed regular tax — triage as a Table/QDCGT")
            print("  reproduction bug (cause ii/iii). This should be impossible on a correct build.")
            undeclared += 1
            found_here = True

        for v in chk["verdicts"]:
            if v.get("class") == "known-defect":
                # The harness adjudicated an already-filed §10 pin (SPEC §10 suppression): reconciled while
                # btctax still prints the pinned wrong value. Logged, not counted undeclared. (A STALE pin
                # comes back red/`diverge` below and IS reported — the loud escape hatch, never silent.)
                suppressed += 1
                print(f"[known-defect {idx}] {v['line']} suppressed by pin {pin} (harness-adjudicated, Rust)")
                continue
            if v.get("reconciled"):
                continue
            _report_divergence(scenario, seed, idx, v, injected, year)
            found_here = True
            if not injected:
                undeclared += 1

        if verbose and not found_here:
            print(f"[ok {idx}] {scenario['theme']}: reconciled", file=sys.stderr)

    print(
        f"\n[sweep] TY{year} seed={seed} count={count}: {admitted} admitted, {skipped} skipped (out of domain)."
        f" {suppressed} suppressed known-defect(s)."
    )
    if undeclared == 0:
        print("0 undeclared divergences")
    else:
        print(f"{undeclared} UNDECLARED divergence(s) — triage per SPEC §10 above.")
    return undeclared


def _parser() -> argparse.ArgumentParser:
    """The real CLI, factored out so `selftest` can prove `--year` is required on THIS parser rather
    than on a look-alike built in the test (B1: the thing that decides must be the thing checked)."""
    ap = argparse.ArgumentParser(description="Live, non-CI, threshold-biased btctax-vs-two-oracle divergence sweep (SPEC §5.2/§9).")
    ap.add_argument("--seed", type=int, required=True, help="deterministic RNG seed (reproducible)")
    ap.add_argument("--count", type=int, required=True, help="number of threshold-biased scenarios to generate")
    # ★ FR-164 — REQUIRED, no default. The year picks the threshold table every draw is biased toward,
    #   the OTS solver year and taxcalc's start year; it is checked against the baked corpus's own
    #   `_provenance.tax_year` before any oracle runs.
    ap.add_argument("--year", type=int, required=True,
                    help="tax year: the threshold table to bias draws toward AND the year both oracles "
                         "are asked about. Must match the baked corpus's _provenance.tax_year.")
    ap.add_argument("--verbose", action="store_true", help="log per-scenario admission/skip reasons to stderr")
    ap.add_argument(
        "--inject-divergence",
        action="store_true",
        help="SELF-TEST: perturb one oracle figure on the first admitted scenario to prove the sweep surfaces a report",
    )
    ap.add_argument("--selftest", action="store_true",
                    help="B1: watch the FR-164 year plumbing discriminate; needs no OTS and no harness "
                         "(handled before the parser runs, so it needs no --seed/--count/--year)")
    return ap


def main() -> None:
    args = _parser().parse_args()
    undeclared = run_sweep(args.seed, args.count, args.year, args.inject_divergence, args.verbose)
    # A real undeclared divergence is a non-zero exit so a wrapper/CI notices; the injected self-test
    # still returns 0 (its perturbation is not counted as undeclared).
    sys.exit(1 if undeclared else 0)


def selftest() -> int:
    """B1 — watch the FR-164 year plumbing discriminate. Pure: no OTS, no harness binary, no network.

        .venv/bin/python scripts/oracle/sweep.py --selftest

    Four claims, each with the plant that reds it:

      1. `thresholds_for` REFUSES a year it has no table for. ★ PLANT: add a fallback to TY2024 and
         claim 1 reds.
      2. The §164(b) cap it returns is `corpus.SALT_CAP_BY_YEAR`'s, not a second copy. ★ PLANT: type
         `"salt_cap": 10_000` into `_YEAR_THRESHOLDS[2024]` and change the corpus value — claim 2 reds.
      3. The draws are actually BIASED BY the table: two different threshold tables over one seed
         produce different scenarios. ★ PLANT: revert `_gen_scenario` to module-level constants and
         claim 3 reds (the year would stop reaching the draw).
      4. `--year` is REQUIRED — argparse refuses a run without it. ★ PLANT: give it a default and
         claim 4 reds.
    """
    bad = 0

    # (1) an unknown year is refused
    try:
        thresholds_for(1999)
    except KeyError as e:
        if "no threshold table for TY1999" not in str(e):
            print(f"  FAIL: wrong refusal for an unknown year: {e}"); bad += 1
        else:
            print("  thresholds_for REFUSES a year it has no table for: OK")
    else:
        print("  FAIL: thresholds_for(1999) returned a table — a fallback is back (FR-164)"); bad += 1

    # (2) the SALT cap is the corpus's, not a copy
    thr = thresholds_for(2024)
    if thr["salt_cap"] != corpus.SALT_CAP_BY_YEAR[2024]:
        print("  FAIL: the §164(b) cap is not corpus.SALT_CAP_BY_YEAR's"); bad += 1
    else:
        print(f"  the §164(b) cap is read from corpus.SALT_CAP_BY_YEAR ({thr['salt_cap']:,}): OK")

    # (3) the table actually steers the draws — same seed, different thresholds, different scenarios
    hypothetical = {
        "std_deduction": {"Single": 31_500, "Married/Joint": 63_000},
        "oasdi_base": 184_500,
        "qbi_8995_ceiling": {"Single": 201_000, "Married/Joint": 402_000},
        "salt_cap": 40_000,
    }
    rng_a, rng_b = random.Random(7), random.Random(7)
    a = [_gen_scenario(rng_a, thr) for _ in range(40)]
    b = [_gen_scenario(rng_b, hypothetical) for _ in range(40)]
    if a == b:
        print("  FAIL: 40 draws are IDENTICAL under two different threshold tables — the year does "
              "not reach the draw, so every threshold-biased theme is biased at the wrong edge"); bad += 1
    else:
        differing = sum(1 for x, y in zip(a, b) if x != y)
        print(f"  the threshold table steers the draws ({differing}/40 scenarios differ): OK")

    # (4) --year is required — on the REAL parser, not a look-alike
    import contextlib
    import io
    err = io.StringIO()
    with contextlib.redirect_stderr(err):
        try:
            _parser().parse_args(["--seed", "1", "--count", "1"])
            refused = None
        except SystemExit:
            refused = "--year" in err.getvalue()
    if refused is None:
        print("  FAIL: the real parser accepted a run with NO --year (FR-164: it must be required)")
        bad += 1
    elif not refused:
        print("  FAIL: the parser refused, but not for a missing --year"); bad += 1
    else:
        print("  --year is REQUIRED on the real parser (a run without it is refused): OK")
    # And a run WITH it parses, so claim 4 is not passing merely because everything is refused.
    ok = _parser().parse_args(["--seed", "1", "--count", "1", "--year", "2024"])
    if ok.year != 2024:
        print("  FAIL: --year did not reach args.year"); bad += 1

    # (5) a --year that disagrees with the baked corpus is refused before any oracle runs
    baked = json.loads(GOLDEN_MATRIX.read_text())["_provenance"]["tax_year"]
    try:
        verify_year_matches_corpus(int(baked) + 2)
    except SystemExit as e:
        if "disagrees with the baked corpus" not in str(e):
            print(f"  FAIL: wrong refusal for a mismatched --year: {e}"); bad += 1
        else:
            print(f"  --year {int(baked) + 2} against a TY{baked} corpus is REFUSED: OK")
    else:
        print("  FAIL: a --year disagreeing with the baked corpus was accepted"); bad += 1
    try:
        verify_year_matches_corpus(int(baked))
    except SystemExit as e:
        print(f"  FAIL: the corpus's OWN year was refused ({e}) — the check cannot pass anything"); bad += 1
    else:
        print(f"  --year {baked} (the corpus's own) is accepted: OK")

    print("sweep: FR-164 year plumbing " + ("FAILED" if bad else "OK"))
    return 1 if bad else 0


if __name__ == "__main__":
    # ★ Checked before the parser is built: --selftest must run without --seed/--count/--year, which
    #   are (deliberately) required for a real sweep.
    if "--selftest" in sys.argv[1:]:
        sys.exit(selftest())
    main()
