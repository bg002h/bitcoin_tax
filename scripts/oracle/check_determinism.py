#!/usr/bin/env python3
"""§12 DETERMINISM CHECK (oracle-sweep T13) — OFFLINE, needs OTS + the taxcalc venv.

SPEC §12: *"regenerating the corpus twice yields identical `households` payload — the claim EXCLUDES the
`_provenance.generated` date field, which is pinned or ignored by the determinism check."* The baked path
carries no RNG (the covering array is a fixed construction), so at fixed engine versions the whole file
reproduces byte-for-byte except `generated`. The T11 re-review confirmed this by hand; this script codifies
it as a REPEATABLE check.

WHY IT CANNOT RUN IN `make check`: it runs the external oracles — OpenTaxSolver's own binaries and the PSL
Tax-Calculator — so it needs `OTS_DIR` and the `.venv`, and `make check` is network-free and hermetic. This
is an OFFLINE gate, run by hand (or in a non-hermetic CI lane) whenever the corpus or an engine version
changes.

USAGE
    export OTS_DIR=/path/to/OpenTaxSolver2024_22.07        # OTS install (oracle 1)
    #  (A) a fresh regeneration must match the COMMITTED corpus — determinism AND non-staleness:
    .venv/bin/python scripts/oracle/check_determinism.py
    #  (B) the PURE two-regeneration claim, independent of the committed file (SPEC §12 "twice"):
    .venv/bin/python scripts/oracle/check_determinism.py --twice
    #  (C) hermetic self-test of THIS script's compare/normalize logic (no OTS/venv needed):
    python3 scripts/oracle/check_determinism.py --selftest

EXIT 0 = identical (PASS). Non-zero + a diff = FAIL (a real drift, a stale committed corpus, or — for a
version bump — a legitimate signal to regenerate and RE-REVIEW).
"""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
COMMITTED = REPO / "crates/btctax-core/tests/goldens/full_return_goldens.json"
GEN = HERE / "gen_goldens.py"

# The ONE field SPEC §12 excludes from the determinism claim — the wall-clock generation date.
NONDETERMINISTIC = ("_provenance", "generated")


def regenerate() -> dict:
    """Run gen_goldens.py and parse its stdout JSON (needs OTS_DIR + the taxcalc venv)."""
    proc = subprocess.run(
        [sys.executable, str(GEN)], capture_output=True, text=True
    )
    if proc.returncode != 0:
        sys.exit(f"gen_goldens.py failed (exit {proc.returncode}):\n{proc.stderr}")
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as e:  # pragma: no cover
        sys.exit(f"gen_goldens.py did not emit valid JSON: {e}\n{proc.stdout[:400]}")


def normalize(doc: dict) -> dict:
    """A deep copy with the single non-deterministic field (`_provenance.generated`) dropped."""
    d = copy.deepcopy(doc)
    top, leaf = NONDETERMINISTIC
    d.get(top, {}).pop(leaf, None)
    return d


def first_household_diff(a: dict, b: dict) -> str | None:
    """A readable pointer to the FIRST differing household (the load-bearing payload), or None."""
    ha, hb = a.get("households", []), b.get("households", [])
    if len(ha) != len(hb):
        return f"household COUNT differs: {len(ha)} vs {len(hb)}"
    for i, (x, y) in enumerate(zip(ha, hb)):
        if x != y:
            nx = x.get("name", f"#{i}")
            # Name the first differing key so the failure is actionable, not just "something changed".
            keys = set(x) | set(y)
            drifted = [k for k in sorted(keys) if x.get(k) != y.get(k)]
            return f"household {i} ({nx!r}) differs in: {drifted}"
    return None


def compare(a: dict, b: dict, label_a: str, label_b: str) -> bool:
    """True iff `a` and `b` are identical modulo `_provenance.generated`. Prints a PASS/FAIL verdict."""
    na, nb = normalize(a), normalize(b)

    # The load-bearing §12 claim FIRST: the `households` payload is byte-identical.
    hh_diff = first_household_diff(na, nb)
    if hh_diff is not None:
        print(f"FAIL: the `households` payload is NOT deterministic ({label_a} vs {label_b}).")
        print(f"      {hh_diff}")
        return False

    # `_provenance` (minus `generated`) must also match — a version-string drift here is a legitimate
    # signal that the engines moved and the corpus needs regeneration + re-review (SPEC §11).
    if na.get("_provenance") != nb.get("_provenance"):
        pa, pb = na.get("_provenance", {}), nb.get("_provenance", {})
        drifted = sorted(k for k in set(pa) | set(pb) if pa.get(k) != pb.get(k))
        print(f"FAIL: `_provenance` (excluding `generated`) drifted ({label_a} vs {label_b}): {drifted}")
        print("      If an *_version field moved, regenerate the corpus AND re-review (SPEC §11).")
        return False

    n = len(na.get("households", []))
    print(f"PASS: {label_a} and {label_b} are byte-identical over {n} households "
          f"(excluding `_provenance.generated`) — §12 determinism holds.")
    return True


def selftest() -> int:
    """Hermetic check of the compare/normalize LOGIC (no OTS/venv). Proves two things:

      1. a differing `generated` date is IGNORED (the excluded field), and
      2. a genuine one-cent household drift IS caught.
    """
    committed = json.loads(COMMITTED.read_text())

    # (1) same payload, different `generated` → must be treated as identical.
    other_date = copy.deepcopy(committed)
    other_date["_provenance"]["generated"] = "1999-01-01"
    ok_ignores_date = compare(committed, other_date, "committed", "committed+date")

    # (2) perturb ONE household leaf by a cent → must be caught.
    perturbed = copy.deepcopy(committed)
    if not perturbed["households"]:
        sys.exit("selftest: the committed corpus is empty")
    perturbed["households"][0]["expected_ots"]["taxable_income"] += 0.01
    caught_drift = not compare(committed, perturbed, "committed", "committed+1cent")

    if ok_ignores_date and caught_drift:
        print("SELFTEST PASS: `generated` is excluded AND a real payload drift is caught.")
        return 0
    print("SELFTEST FAIL: the determinism-check logic is broken.")
    return 1


def main() -> int:
    ap = argparse.ArgumentParser(description="§12 corpus determinism check (offline).")
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--twice", action="store_true",
                   help="regenerate the corpus TWICE and compare (pure §12 claim, ignores the committed file)")
    g.add_argument("--selftest", action="store_true",
                   help="hermetic self-test of the compare logic (no OTS/venv needed)")
    args = ap.parse_args()

    if args.selftest:
        return selftest()

    if args.twice:
        a = regenerate()
        b = regenerate()
        return 0 if compare(a, b, "regen#1", "regen#2") else 1

    # Default: a fresh regeneration must match the committed corpus (determinism + non-staleness).
    fresh = regenerate()
    committed = json.loads(COMMITTED.read_text())
    return 0 if compare(fresh, committed, "fresh-regen", "committed") else 1


if __name__ == "__main__":
    sys.exit(main())
