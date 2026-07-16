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
    .venv/bin/python scripts/oracle/sweep.py --seed 1 --count 50

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
SCH_B_TRIGGER = 1_500  # taxable interest ≥ $1,500 ⇒ Schedule B files (Part I/II) — 1040 §Interest
SALT_CAP = 10_000  # §164(b)(5): Sch A line 5e = min(5a+5b+5c, $10,000)
OASDI_BASE = 168_600  # 2024 §1402(b)(1)/§3121 OASDI wage base — Sch SE L8a absorbs the band
# The $200k/$250k Additional-Medicare (§3101(b)(2)) AND NIIT (§1411) MAGI thresholds, by status.
ADDL_MEDICARE_NIIT = {"Single": 200_000, "Married/Joint": 250_000}
STD_DEDUCTION = corpus.STD_DEDUCTION_2024  # {"Single": 14_600, "Married/Joint": 29_200} — crossover

STATUSES = ["Single", "Married/Joint"]

# §199A simple-Form-8995 taxable-income ceiling (2024). ABOVE it btctax REFUSES (Form 8995-A is out of
# the sweep's domain, D-2), so an SE draw must keep earned income well under it. The compiled harness's
# refusal screen is the authoritative D-2 gate; this is the generator-side bias that keeps most SE draws
# admissible instead of wasting oracle runs on refusals.
QBI_8995_CEILING = {"Single": 191_950, "Married/Joint": 383_900}


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


def _itemize(rng: random.Random, inp: dict, status: str, itemized_total_target: int, salt_total: int) -> None:
    """Attach a Schedule A sized so itemizing WINS (D-3): the itemized total STRICTLY exceeds the
    standard deduction. `salt_total` is the pre-cap 5a+5b sum; the mortgage is sized so
    mortgage + min(salt_total, cap) ≈ `itemized_total_target` (kept ≥ STD + $1 by the caller)."""
    state, realest = _split_salt(rng, salt_total)
    capped_salt = min(salt_total, SALT_CAP)
    mortgage = max(0, itemized_total_target - capped_salt)
    inp["state_income_tax"] = state
    inp["real_estate_tax"] = realest
    inp["mortgage_interest"] = mortgage
    # Read only by the Python oracles to force their Schedule-A path (ignored by btctax's GoldenInputs).
    inp["standard_or_itemized"] = "Itemized"


def _gen_scenario(rng: random.Random) -> tuple[dict, str]:
    """Draw ONE threshold-biased scenario. Returns (inputs, theme). Honors the domain by construction as
    far as it can (SE⇒low W-2; itemizing-wins); the authoritative D-2/AMT/credit gates run downstream."""
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
        salt_total = _near(rng, SALT_CAP, 800)  # tight on the $10k cap (straddles both sides)
        # Mortgage clears STD comfortably so itemizing wins regardless of the cap outcome.
        target = STD_DEDUCTION[status] + rng.randint(6_000, 20_000)
        _itemize(rng, inp, status, target, salt_total)

    elif theme == "std_crossover":
        inp["w2_income"] = rng.randint(40_000, 120_000)
        salt_total = rng.randint(2_000, 8_000)
        # Itemized total JUST above the standard deduction (D-3 from the winning side, near the crossover).
        target = STD_DEDUCTION[status] + rng.randint(1, 1_500)
        _itemize(rng, inp, status, target, salt_total)

    elif theme == "addl_medicare_edge":
        # Medicare wages (= box 1 here) tight on the $200k/$250k Additional-Medicare threshold (§3101(b)(2)).
        inp["w2_income"] = _near(rng, ADDL_MEDICARE_NIIT[status], 2_500)

    elif theme == "se_oasdi_edge":
        # SE⇒W-2 must stay low for §199A (D-2), so probe the OASDI wage base with MFJ headroom: a W-2 near
        # $168,600 fills the OASDI band (Sch SE L8a), and a modest SE profit rides on top (8959 Part II).
        status = "Married/Joint"
        inp["filing_status"] = status
        inp["w2_income"] = _near(rng, OASDI_BASE, 3_000)
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
        if earned > QBI_8995_CEILING[status] * 0.85:
            inp["w2_income"] = min(inp.get("w2_income", 0), 40_000)
    # At least one income source (corpus.py's no-all-none rule).
    if not corpus._has_income(inp):
        inp["w2_income"] = inp.get("w2_income", 0) + rng.randint(20_000, 60_000)

    return inp, theme


def generate(seed: int, count: int) -> list[dict]:
    """The K seeded, threshold-biased scenarios (reproducible from `seed`). Each is a
    `{name, why, inputs, theme}` dict; `name` encodes the seed+index so a report is reproducible."""
    rng = random.Random(seed)
    out = []
    for i in range(count):
        inp, theme = _gen_scenario(rng)
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


def _verify_harness_freshness() -> None:
    """Build-freshness gate — prove the harness binary is T7-m1-fresh (emits `reproduction_ok`) BEFORE
    spending oracle time. A binary built before T7-m1 lacks the field; defaulting a missing key to a pass
    would silently disable the structural witness in a DISCOVERY tool, so we fail loud and early with a
    rebuild instruction instead. Probes `--check` on the refusal-free floor anchor."""
    matrix_path = Path(__file__).resolve().parents[2] / "crates/btctax-core/tests/goldens/full_return_goldens.json"
    matrix = json.loads(matrix_path.read_text())
    probe = next(h for h in matrix["households"] if h["name"] == "single_w2_only_standard")
    chk = _harness_check(probe)
    if chk.get("malformed") or "reproduction_ok" not in chk:
        sys.exit(
            f"{gen_goldens.HARNESS_BIN} is STALE — its --check output carries no `reproduction_ok` "
            "(built before T7-m1). Rebuild it: `cargo build -p btctax-oracle-harness`."
        )


def _report_divergence(scenario: dict, seed: int, index: int, verdict: dict, injected: bool) -> None:
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
    print(f"  reproduce: sweep.py --seed {seed} --count {index + 1}   (scenario index {index})")
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


def run_sweep(seed: int, count: int, inject: bool, verbose: bool) -> int:
    """Generate K scenarios, admit (D-2 refusal-free + AMT/credit-free), live-diff each admitted one, and
    report undeclared divergences. Returns the number of UNDECLARED divergences (0 = a clean run)."""
    if not os.environ.get("OTS_DIR") or not ots_direct.OTS_DIR.exists():
        sys.exit("set OTS_DIR to an unpacked OpenTaxSolver2024 tree (see ots_direct.py).")
    if not gen_goldens.HARNESS_BIN.exists():
        sys.exit(f"{gen_goldens.HARNESS_BIN} not found — build it: `cargo build -p btctax-oracle-harness`.")
    _verify_harness_freshness()  # fail loud NOW if the binary predates T7-m1 (no `reproduction_ok`)

    scenarios = generate(seed, count)
    all_inputs = [s["inputs"] for s in scenarios]

    # Batch oracle-2 (taxcalc) once: AMT/credit admission probe + the full expected dict (vectorized —
    # one Calculator pass each, not per-scenario).
    amt_credits = gen_goldens._taxcalc_amt_credits(all_inputs)
    taxcalc_full = gen_goldens.taxcalc_run(all_inputs)

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
        expected_ots = ots_direct.evaluate(inputs)
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
            _report_divergence(scenario, seed, idx, v, injected)
            found_here = True
            if not injected:
                undeclared += 1

        if verbose and not found_here:
            print(f"[ok {idx}] {scenario['theme']}: reconciled", file=sys.stderr)

    print(
        f"\n[sweep] seed={seed} count={count}: {admitted} admitted, {skipped} skipped (out of domain)."
        f" {suppressed} suppressed known-defect(s)."
    )
    if undeclared == 0:
        print("0 undeclared divergences")
    else:
        print(f"{undeclared} UNDECLARED divergence(s) — triage per SPEC §10 above.")
    return undeclared


def main() -> None:
    ap = argparse.ArgumentParser(description="Live, non-CI, threshold-biased btctax-vs-two-oracle divergence sweep (SPEC §5.2/§9).")
    ap.add_argument("--seed", type=int, required=True, help="deterministic RNG seed (reproducible)")
    ap.add_argument("--count", type=int, required=True, help="number of threshold-biased scenarios to generate")
    ap.add_argument("--verbose", action="store_true", help="log per-scenario admission/skip reasons to stderr")
    ap.add_argument(
        "--inject-divergence",
        action="store_true",
        help="SELF-TEST: perturb one oracle figure on the first admitted scenario to prove the sweep surfaces a report",
    )
    args = ap.parse_args()
    undeclared = run_sweep(args.seed, args.count, args.inject_divergence, args.verbose)
    # A real undeclared divergence is a non-zero exit so a wrapper/CI notices; the injected self-test
    # still returns 0 (its perturbation is not counted as undeclared).
    sys.exit(1 if undeclared else 0)


if __name__ == "__main__":
    main()
