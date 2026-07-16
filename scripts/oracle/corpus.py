#!/usr/bin/env python3
"""The oracle-sweep BAKED corpus — a deterministic, variable-strength, constrained covering array.

★ Why this file exists (SPEC §5.1 / plan T10)
--------------------------------------------
`gen_goldens.py` used to carry a hand-written list of 12 households. That list is preserved here
VERBATIM as the 12 **anchors** (their `why` prose intact — each pins a specific tax-law obligation).
On top of the anchors this module generates a **covering array** so the double-oracle differential
harness exercises the interaction space, not just 12 points, while staying inside the runtime budget.

★ Variable strength — t=3 on two named triples, t=2 elsewhere (SPEC §5.1)
------------------------------------------------------------------------
Pairwise (t=2) is provably insufficient for SPEC §12's load-bearing lines: the Form 8995 line-12
qualified-dividend term is a **3-way** interaction (SE × LTCG × qualified-dividends, all at once),
and pairwise never guarantees the triple co-occurs. A *global* t=3 over all axes would blow the
~80–120 budget. So this is a **variable-strength** array, built as a UNION (deduplicated, M3):

  * **Block A** — the FULL CARTESIAN product over the axes of the named triple {SE, LTCG, qual-div}
    (× filing status). t=3-complete on that triple *by construction*.
  * **Block B** — the FULL CARTESIAN product over the axes of the named triple
    {itemized, SALT-over-cap, high-income} (× filing status), under the constraints. t=3-complete on
    that triple *by construction*.
  * **Block P** — a deterministic greedy **pairwise (t=2)** covering array over ALL axes, with
    constraints, so every axis-value PAIR co-occurs somewhere ("t=2 elsewhere").

`assert_named_triple_coverage()` (the ~10-line check the plan asks for) proves every named
triple-combination is present in the ADMITTED corpus — it is NOT a generic CA algorithm and does NOT
use `allpairspy`/PICT (pairwise-only, a mis-fit for t=3). No new *runtime* dependency: pure Python.

★ Constraints (SPEC §4 domain invariants D-1/D-2/D-3)
-----------------------------------------------------
  * **SALT-over-cap position ⇒ itemized** (a standard-deduction return has no Schedule A line 5e).
  * **itemized ⇒ itemizing wins** (D-3): the itemized total exceeds the standard deduction, so the
    §63(e) election is not a confound (the mortgage figure is sized to clear both standards).
  * **no all-none row**: at least one income source (the degenerate zero-income return is excluded).
  * **D-1 no dependents**: never varied (CTC/ODC/EIC omitted, app §3.4; `build_golden_return` stamps
    `can_be_claimed_as_dependent_* = Some(false)`).
  * **SE present/over ⇒ W-2 not mid/high**: large combined earned income drives taxable income over
    the §199A simple-8995 threshold ($191,950 Single / $383,900 MFJ, 2024), which btctax REFUSES
    (Form 8995-A is out of the sweep's domain). Keeping SE households' W-2 low/none keeps them
    refusal-free (D-2); the admission loop in `gen_goldens.py` is the backstop that drops + logs any
    straggler, so a missed corner never silently enters the baked file.

Domain: filing status ∈ {Single, MFJ} only (MFS deferred). Refusal-freeness (D-2) and AMT/credit
freeness are ENFORCED by the harness-binary admission loop in `gen_goldens.py`, not here — generation
is Python and assembly is Rust, so the D-2 check crosses that boundary through the §9 harness rather
than a Python re-implementation of the AMT screen that could drift.

★ The two bake-time-steered pinned liveness cells (SPEC §5.1, r3-I2b/r4-I1)
--------------------------------------------------------------------------
Two explicit cells hold the per-oracle L16 **provenance** classes live (§6.4 liveness). Each is
STEERED (its inputs chosen to the cent) and CHECKED at generation time to actually produce the
intended flip — `gen_goldens.py:verify_pinned_cells()` drives the §9 harness `--check` on each and
asserts the flip fires. See `PINNED_CELLS` below for the exact figures and how each was found.

Everything here is DETERMINISTIC (no RNG in the baked path) — regenerating yields an identical
`households` payload (SPEC §12; only `_provenance.generated`, the date, varies).
"""

from __future__ import annotations

# ── 2024 standard deductions (for the D-3 "itemizing wins" sizing note) ───────────────────────────
STD_DEDUCTION_2024 = {"Single": 14_600, "Married/Joint": 29_200}

# ── Axis value → amount tables (deterministic; one amount per axis-value) ──────────────────────────
# Filing status ∈ {Single, MFJ}. MFS is deferred (SPEC §2).
STATUS = ["Single", "Married/Joint"]

# W-2 box-1 wages. "low" is floored ABOVE the childless-EIC band (~$18.6k Single / ~$25.5k MFJ) so the
# covering array stays satisfiable while EIC never engages (SPEC §4 r2-M5). "high" clears the $250k
# MFJ NIIT/Add'l-Medicare thresholds — the triple-B "high-income" leg.
W2 = {"none": 0, "low": 42_000, "mid": 105_000, "high": 270_000}

# Taxable interest: none / below the $1,500 Schedule-B trigger / above it.
INTEREST = {"none": 0, "under": 1_200, "over": 2_000}

# Dividends: (ordinary incl. the qualified subset, qualified). "qual" carries BOTH so the QDCGT
# worksheet's qualified-dividend slice is load-bearing.
DIV = {"none": (0, 0), "qual": (6_000, 4_000)}

# Capital-gain SHAPE. "loss" is a §1211(b) net loss capped at −$3,000 against ordinary income. ("both
# slices" = LTCG + qualified dividends is produced by cap=LT × div=qual, so it needs no own value.)
CAP = {
    "none": {},
    "LT": {"long_term_capital_gains": 20_000},
    "ST": {"short_term_capital_gains": 12_000},
    "loss": {"short_term_capital_gains": -18_000},
}

# Schedule C (crypto business) net profit. "over" is a LARGE SE profit; combined with W-2 (mid/high)
# it would cross the §199A simple-8995 threshold, so the SE⇒low/none-W-2 constraint keeps it in domain.
SE = {"none": 0, "present": 55_000, "over": 120_000}

# Itemized Schedule-A components. Mortgage interest is sized so the itemized total clears BOTH standard
# deductions (D-3 "itemizing wins"): under-cap 3+4k SALT + 25k = 32k; over-cap 8+9k→10k cap + 25k = 35k.
MORTGAGE_ITEMIZED = 25_000
SALT = {
    "under": {"state_income_tax": 3_000, "real_estate_tax": 4_000},  # 5d = 7,000 < $10k cap
    "over": {"state_income_tax": 8_000, "real_estate_tax": 9_000},   # 5d = 17,000 → capped to $10k
}


def _build(status, w2, interest, div, cap, se, dedsalt):
    """Assemble one household's `inputs` dict from axis-value LABELS. `dedsalt` is the combined
    deduction/SALT axis ∈ {std, iu (itemized+under-cap), io (itemized+over-cap)} — collapsing the
    deduction and SALT-position axes into one removes the SALT⇒itemized cross-constraint by
    construction (a standard row simply has no SALT fields)."""
    inp = {"filing_status": status}
    if W2[w2]:
        inp["w2_income"] = W2[w2]
    if INTEREST[interest]:
        inp["taxable_interest"] = INTEREST[interest]
    ordinary, qualified = DIV[div]
    if ordinary:
        inp["ordinary_dividends"] = ordinary
        inp["qualified_dividends"] = qualified
    inp.update(CAP[cap])
    if SE[se]:
        inp["self_employment_income"] = SE[se]
    if dedsalt != "std":
        salt = "under" if dedsalt == "iu" else "over"
        inp.update(SALT[salt])
        inp["mortgage_interest"] = MORTGAGE_ITEMIZED
        inp["standard_or_itemized"] = "Itemized"  # read by the Python oracles (not a GoldenInputs field)
    return inp


def _has_income(inp):
    return any(
        k in inp
        for k in (
            "w2_income",
            "taxable_interest",
            "ordinary_dividends",
            "short_term_capital_gains",
            "long_term_capital_gains",
            "self_employment_income",
        )
    )


def _se_w2_ok(se_label, w2_label):
    """SE present/over ⇒ W-2 ∈ {none, low} (keeps taxable income under the §199A simple-8995 threshold
    so btctax does not refuse — Form 8995-A is out of the sweep's domain)."""
    return se_label == "none" or w2_label in ("none", "low")


# ── Block A — full cartesian over the named triple {SE, LTCG, qualified-dividends} × status × ctx ──
# t=3-COMPLETE on {SE, LTCG, qual-div} by construction: all 3 (se) × 2 (ltcg) × 2 (qd) = 12 value
# combinations appear (for BOTH filing statuses, AND in two interest contexts, so the load-bearing
# 8995-L12 3-way interaction is exercised broadly). W-2 = "low" keeps every row funded and
# refusal-free (SE ⇒ low/none W-2).
def block_a():
    rows = []
    for status in STATUS:
        for interest in ("none", "over"):  # a Schedule-B-off and a Schedule-B-on context
            for se in ("none", "present", "over"):
                for ltcg in (False, True):
                    for qd in (False, True):
                        inp = _build(
                            status,
                            w2="low",
                            interest=interest,
                            div=("qual" if qd else "none"),
                            cap=("LT" if ltcg else "none"),
                            se=se,
                            dedsalt="std",
                        )
                        who = "mfj" if status != "Single" else "single"
                        rows.append(
                            {
                                "name": f"ca_A_{who}_int-{interest}_se-{se}_ltcg-{int(ltcg)}_qd-{int(qd)}",
                                "why": "t=3 triple {SE,LTCG,qual-div}: "
                                f"SE={se}, LTCG={'yes' if ltcg else 'no'}, qual-div={'yes' if qd else 'no'} "
                                "(Form 8995 L12 qualified-dividend term is the 3-way interaction)",
                                "inputs": inp,
                            }
                        )
    return rows


# ── Block B — full cartesian over {itemized, SALT-over-cap, high-income} × status, with constraints ─
# Feasible (item, salt, high) combos under "SALT-over ⇒ itemized": (std,-,lo) (std,-,hi)
# (item,under,lo) (item,under,hi) (item,over,lo) (item,over,hi) = 6, × 2 status × 2 dividend contexts
# = 24. SE=none (so the high-income leg does not trip the §199A refusal), income = W-2 (mid for
# "not-high", high for "high"). The dividend context exercises the itemize election × the QDCGT.
def block_b():
    rows = []
    for status in STATUS:
        for div in ("none", "qual"):
            for dedsalt in ("std", "iu", "io"):
                for high in (False, True):
                    w2 = "high" if high else "mid"
                    inp = _build(status, w2=w2, interest="none", div=div, cap="none", se="none", dedsalt=dedsalt)
                    item = {"std": "standard", "iu": "itemized(SALT under cap)", "io": "itemized(SALT over cap)"}[dedsalt]
                    who = "mfj" if status != "Single" else "single"
                    rows.append(
                        {
                            "name": f"ca_B_{who}_div-{div}_{dedsalt}_{'hi' if high else 'lo'}",
                            "why": "t=3 triple {itemized,SALT-over-cap,high-income}: "
                            f"{item}, income={'high' if high else 'mid'}, qual-div={'yes' if div == 'qual' else 'no'} "
                            "(the §164(b)(5) cap × the itemize election × the rate bands)",
                            "inputs": inp,
                        }
                    )
    return rows


# ── Block P — deterministic greedy pairwise (t=2) covering array over ALL axes, with constraints ───
# Axes collapsed to labels; (deduction,SALT) is one combined axis to dissolve the SALT⇒itemized
# cross-constraint. The remaining constraints (SE⇒low/none-W-2; at-least-one-income) are enforced as
# feasibility predicates during construction — a standard, deterministic AETG-style greedy (no RNG:
# axes and values iterate in fixed order, ties broken by first-seen). This guarantees every FEASIBLE
# axis-value pair co-occurs in ≥1 row ("t=2 elsewhere").
PAIRWISE_AXES = [
    ("status", ["Single", "Married/Joint"]),
    ("w2", ["none", "low", "mid", "high"]),
    ("interest", ["none", "under", "over"]),
    ("div", ["none", "qual"]),
    ("cap", ["none", "LT", "ST", "loss"]),
    ("se", ["none", "present", "over"]),
    ("dedsalt", ["std", "iu", "io"]),
]


def _row_feasible(assign, full):
    if not _se_w2_ok(assign.get("se", "none"), assign.get("w2", "none")):
        return False
    if full:
        inp = _build(
            assign["status"], assign["w2"], assign["interest"], assign["div"], assign["cap"], assign["se"], assign["dedsalt"]
        )
        if not _has_income(inp):
            return False
    return True


def _all_feasible_pairs():
    pairs = set()
    n = len(PAIRWISE_AXES)
    for i in range(n):
        ai, vi = PAIRWISE_AXES[i]
        for j in range(i + 1, n):
            aj, vj = PAIRWISE_AXES[j]
            for a in vi:
                for b in vj:
                    if _row_feasible({ai: a, aj: b}, full=False):
                        pairs.add((ai, a, aj, b))
    return pairs


def _pairs_of(assign):
    keys = [a for a, _ in PAIRWISE_AXES]
    out = set()
    for i in range(len(keys)):
        for j in range(i + 1, len(keys)):
            out.add((keys[i], assign[keys[i]], keys[j], assign[keys[j]]))
    return out


def block_p():
    remaining = _all_feasible_pairs()
    rows = []
    guard = 0
    while remaining:
        guard += 1
        if guard > 10_000:  # pragma: no cover — deterministic termination guard
            raise RuntimeError("pairwise construction did not converge")
        # Seed the row with a still-uncovered pair (first in a fixed sort order → deterministic).
        seed = sorted(remaining)[0]
        assign = {seed[0]: seed[1], seed[2]: seed[3]}
        for axis, values in PAIRWISE_AXES:
            if axis in assign:
                continue
            best_val, best_gain = None, -1
            for v in values:  # fixed order ⇒ deterministic tie-break (first-seen wins on a tie)
                if not _row_feasible({**assign, axis: v}, full=False):
                    continue
                # gain = still-uncovered pairs this value forms with the axes already assigned.
                gain = sum(
                    1
                    for (a1, x1, a2, x2) in remaining
                    if (a1 == axis and x1 == v and assign.get(a2) == x2)
                    or (a2 == axis and x2 == v and assign.get(a1) == x1)
                )
                if gain > best_gain:
                    best_val, best_gain = v, gain
            if best_val is None:  # no feasible value (an SE⇒W-2 dead-end) → a safe in-domain default
                best_val = "none" if axis in ("w2", "interest", "cap", "se") else values[0]
            assign[axis] = best_val
        if not _row_feasible(assign, full=True):
            # A fully-assigned infeasible row (all-none): force a minimal income and retry feasibility.
            assign["w2"] = "low"
        covered = _pairs_of(assign)
        newly = covered & remaining
        if not newly:
            # Safety: the seed pair must be covered; drop it to guarantee progress.
            remaining.discard(seed)
            continue
        remaining -= covered
        inp = _build(assign["status"], assign["w2"], assign["interest"], assign["div"], assign["cap"], assign["se"], assign["dedsalt"])
        rows.append(
            {
                "name": f"ca_P_{len(rows):02d}",
                "why": "pairwise (t=2) cell: "
                + ", ".join(f"{a}={assign[a]}" for a, _ in PAIRWISE_AXES),
                "inputs": inp,
            }
        )
    return rows


# ── The 12 ANCHORS — verbatim from the former `gen_goldens.py` HOUSEHOLDS (their `why` preserved) ──
ANCHORS = [
    {
        "name": "single_w2_only_standard",
        "why": "the floor case — one W-2, standard deduction, no crypto at all",
        "inputs": {"filing_status": "Single", "w2_income": 62_000},
    },
    {
        "name": "single_w2_plus_crypto_ltcg",
        "why": "the core btctax case: wages + a long-term crypto gain (Sch D → 1040 L7)",
        "inputs": {"filing_status": "Single", "w2_income": 62_000, "long_term_capital_gains": 20_000},
    },
    {
        "name": "single_qdcgt_both_slices",
        "why": "the QDCGT worksheet with BOTH preferential slices — qualified dividends AND LTCG",
        "inputs": {
            "filing_status": "Single",
            "w2_income": 90_000,
            "qualified_dividends": 8_000,
            "ordinary_dividends": 10_000,
            "taxable_interest": 2_000,
            "long_term_capital_gains": 25_000,
        },
    },
    {
        "name": "single_short_term_crypto_gain",
        "why": "a SHORT-term crypto gain — ordinary rates, no preferential slice",
        "inputs": {"filing_status": "Single", "w2_income": 55_000, "short_term_capital_gains": 12_000},
    },
    {
        "name": "single_capital_loss_capped",
        "why": "§1211(b): a big net capital loss is capped at $3,000 against ordinary income",
        "inputs": {"filing_status": "Single", "w2_income": 70_000, "short_term_capital_gains": -18_000},
    },
    {
        "name": "mfj_two_w2_standard",
        "why": "MFJ, standard deduction, a little interest BELOW the $1,500 Schedule B trigger — the "
        "common household, and the discriminating case for whether Schedule B files at all",
        "inputs": {"filing_status": "Married/Joint", "w2_income": 185_000, "taxable_interest": 1_200},
    },
    {
        "name": "mfj_itemized_over_100k",
        "why": "MFJ, ITEMIZED (over the $29,200 standard), over $100k — the Schedule A path",
        "inputs": {
            "filing_status": "Married/Joint",
            "w2_income": 240_000,
            "qualified_dividends": 5_000,
            "ordinary_dividends": 6_000,
            "long_term_capital_gains": 30_000,
            "standard_or_itemized": "Itemized",
            "itemized_deductions": 41_000,
        },
    },
    {
        "name": "mfj_high_income_niit_and_addl_medicare",
        "why": "over the $250k MFJ thresholds — Form 8960 NIIT *and* Form 8959 Additional Medicare",
        "inputs": {
            "filing_status": "Married/Joint",
            "w2_income": 300_000,
            "taxable_interest": 5_000,
            "ordinary_dividends": 12_000,
            "qualified_dividends": 9_000,
            "long_term_capital_gains": 60_000,
        },
    },
    {
        "name": "mfj_itemized_salt_over_the_cap",
        "why": "§164(b)(5): state income tax + real estate tax EXCEED the $10,000 SALT cap, and itemizing "
        "still wins — so the cap BINDS and changes the tax. No other golden exercises it: the matrix's "
        "other itemized household carries one lump sum. The SALT figures are IRS ATS Test Scenario 2's "
        "($1,068 state income tax + $10,509 real estate = $11,577, capped to $10,000); the mortgage is "
        "raised so the itemized total clears the $29,200 standard deduction, which Scenario 2's own "
        "numbers do NOT (its Schedule A totals $28,289 — the IRS scenario tests e-file schema, not "
        "whether itemizing wins).",
        "inputs": {
            "filing_status": "Married/Joint",
            "w2_income": 38_730,  # ATS Scenario 2's two W-2s: $29,513 + $9,217
            "state_income_tax": 1_068,  # Sch A 5a
            "real_estate_tax": 10_509,  # Sch A 5b  ⇒ 5d = 11,577 > the $10,000 cap
            "mortgage_interest": 25_000,  # Sch A 8a
            "standard_or_itemized": "Itemized",
        },
    },
    {
        "name": "single_crypto_business_se",
        "why": "crypto MINING as a trade or business: Schedule C → Schedule SE (deep/02 Ex.2 shape)",
        "inputs": {"filing_status": "Single", "w2_income": 40_000, "self_employment_income": 60_000},
    },
    {
        "name": "single_miner_qbi_limited_by_net_capital_gain",
        "why": "★ Form 8995 line 12. The §199A deduction is capped at 20% of (taxable income − NET "
        "CAPITAL GAIN), and here that limit BINDS *because of* the gain: 20% × QBI = 11,152, but "
        "20% × (81,161 − 40,000) = 8,232, so the capital gain costs this miner $2,920 of deduction. "
        "Drop the line-12 subtraction and the deduction silently grows to 11,152 — understating tax. "
        "No other golden combines QBI with a capital gain, so nothing else holds line 12 against an "
        "oracle.",
        "inputs": {
            "filing_status": "Single",
            "self_employment_income": 60_000,
            "long_term_capital_gains": 40_000,
            # Form 8995 line 12 is "net capital gain" INCREASED BY qualified dividends. Without these, the
            # qualified-dividend term of that line is zero on every household that has a Form 8995 at all —
            # drop it from the code and nothing goes red. $5,000 makes it load-bearing.
            "qualified_dividends": 5_000,
            "ordinary_dividends": 5_000,
        },
    },
    {
        "name": "mfj_se_over_the_addl_medicare_threshold",
        "why": "SE income pushing the household over $250k — 8959 Part II (the SE leg) engages, and "
        "the W-2 wages have already consumed the OASDI band (Sch SE L8a/L9/L10)",
        "inputs": {"filing_status": "Married/Joint", "w2_income": 220_000, "self_employment_income": 80_000},
    },
]


# ── The two bake-time-steered PINNED LIVENESS CELLS (SPEC §5.1, r3-I2b/r4-I1) ──────────────────────
# Each holds a per-oracle L16 PROVENANCE class live (§6.4). Both were found by an offline scan (see
# the task report) and are CHECKED at generation time by `gen_goldens.py:verify_pinned_cells()`, which
# drives the §9 harness `--check` and asserts the flip actually fires with the pinned OTS/taxcalc
# versions. The figures below are the verified offline result.
PINNED_CELLS = [
    {
        # OTS provenance class — a BELOW-ceiling household whose L16 Tax-Table operand sits on a $50 bin
        # edge: btctax's own taxable income 32,950.16 falls in bin [32,950 → 33,000) (midpoint 32,975 →
        # $3,725) while OTS's taxable income 32,949.73 falls in the ADJACENT bin [32,900 → 32,950)
        # (midpoint 32,925 → $3,719). btctax's $50-Tax-Table lookup reproduces OTS's L16 on OTS's own
        # taxable income (provenance conjunct-1) but NOT on btctax's (conjunct-2) → the OTS provenance
        # class fires. taxcalc's below-ceiling schedule-vs-Table dissent is absorbed by the methodology
        # class. The single Schedule-C profit is steered to the cent so the half-SE rounding lands the
        # two engines' taxable incomes across the $32,950 bin boundary.
        "name": "pinned_ots_provenance_bin_edge",
        "why": "★ §5.1 PINNED CELL — holds the OTS L16 PROVENANCE class live. An L16 Tax-Table operand "
        "steered onto a $50 bin edge: btctax TI 32,950.16 (bin→$3,725) vs OTS TI 32,949.73 (bin→$3,719); "
        "btctax's Table reproduces OTS's L16 on OTS's operand but not on its own → the class fires "
        "(taxcalc absorbed by the methodology class below the ceiling).",
        "inputs": {"filing_status": "Single", "self_employment_income": 60_028.00},
        "pins_class": "ots_provenance",
    },
    {
        # taxcalc provenance class — an ABOVE-ceiling, high-TI cents household. Above $100k the Tax Table
        # is gone and btctax uses the exact rate schedule, so the METHODOLOGY class cannot fire; the only
        # lawful absorber of a btctax-vs-taxcalc L16 dissent is the taxcalc PROVENANCE class. Steered so
        # the printed-chain vs exact-cents residual flips a rounded dollar: btctax's schedule tax on its
        # exact taxable income 253,943.72 is $47,031.4976 → rounds to $47,031, while taxcalc's exact L16
        # 47,031.5075 rounds to $47,032. btctax's schedule reproduces taxcalc's L16 on taxcalc's own
        # taxable income (conjunct-1) but not on btctax's (conjunct-2) → the taxcalc provenance class
        # fires. The Schedule-C profit is steered to the cent onto this rate×δ half-dollar boundary.
        "name": "pinned_taxcalc_provenance_cents_flip",
        "why": "★ §5.1 PINNED CELL — holds the taxcalc L16 PROVENANCE class live. High-TI, ABOVE the "
        "$100k Tax-Table ceiling (methodology class off): btctax's schedule tax on exact TI 253,943.72 "
        "= 47,031.4976 → $47,031, taxcalc's exact L16 47,031.5075 → $47,032; the rate×δ printed-vs-exact "
        "residual flips a rounded dollar → the taxcalc provenance class fires.",
        "inputs": {
            "filing_status": "Married/Joint",
            "w2_income": 220_000,
            "self_employment_income": 80_001.00,
        },
        "pins_class": "taxcalc_provenance",
    },
]


def _inputs_key(inp):
    """Canonical dedup key for an `inputs` dict (order-independent; excludes the oracle-only
    `standard_or_itemized` hint, which does not change btctax's assembled return)."""
    return tuple(sorted((k, v) for k, v in inp.items() if k != "standard_or_itemized"))


def households():
    """The full candidate corpus: the 12 anchors (verbatim, first) + the 2 pinned liveness cells +
    the generated covering array (Block A ∪ Block B ∪ Block P), DEDUPLICATED by inputs (M3).

    Anchors and pinned cells are kept whenever they appear; a generated row that duplicates one of
    them (or another generated row) is dropped. Returns a list of `{name, why, inputs, ...}` dicts.
    Admission (D-2 refusal-free + AMT/credit-free) is applied downstream in `gen_goldens.py`.
    """
    out = []
    seen = set()
    for h in ANCHORS + PINNED_CELLS + block_a() + block_b() + block_p():
        key = _inputs_key(h["inputs"])
        if key in seen:
            continue
        seen.add(key)
        out.append(h)
    return out


# ── The ~10-line coverage assertion the plan asks for (runs on the ADMITTED corpus) ───────────────
def _triple_a_cell(inp):
    se = "over" if inp.get("self_employment_income", 0) >= SE["over"] else (
        "present" if inp.get("self_employment_income", 0) > 0 else "none"
    )
    ltcg = inp.get("long_term_capital_gains", 0) > 0
    qd = inp.get("qualified_dividends", 0) > 0
    return (se, ltcg, qd)


def _triple_b_cell(inp):
    itemized = inp.get("standard_or_itemized") == "Itemized" or any(
        inp.get(k, 0) for k in ("state_income_tax", "real_estate_tax", "mortgage_interest", "itemized_deductions")
    )
    salt_over = (inp.get("state_income_tax", 0) + inp.get("real_estate_tax", 0)) > 10_000
    high = inp.get("w2_income", 0) >= W2["high"] or inp.get("self_employment_income", 0) >= W2["high"]
    salt = ("over" if salt_over else "under") if itemized else "na"
    return (itemized, salt, high)


def assert_named_triple_coverage(admitted):
    """Prove t=3 completeness on the two named triples over the ADMITTED corpus (the ~10-line check)."""
    a = {_triple_a_cell(h["inputs"]) for h in admitted}
    want_a = {(se, ltcg, qd) for se in ("none", "present", "over") for ltcg in (False, True) for qd in (False, True)}
    missing_a = want_a - a
    assert not missing_a, f"triple {{SE,LTCG,qual-div}} not fully covered; missing: {sorted(missing_a)}"
    b = {_triple_b_cell(h["inputs"]) for h in admitted}
    want_b = {(False, "na", False), (False, "na", True), (True, "under", False), (True, "under", True), (True, "over", False), (True, "over", True)}
    missing_b = want_b - b
    assert not missing_b, f"triple {{itemized,SALT-over,high}} not fully covered; missing: {sorted(missing_b)}"
    return len(want_a), len(want_b)


if __name__ == "__main__":  # a quick offline sanity dump (no oracles)
    hs = households()
    print(f"candidates: {len(hs)} (anchors {len(ANCHORS)} + pinned {len(PINNED_CELLS)} + generated {len(hs) - len(ANCHORS) - len(PINNED_CELLS)})")
    print(f"  block A {len(block_a())}, block B {len(block_b())}, block P {len(block_p())}")
    na, nb = assert_named_triple_coverage(hs)
    print(f"named-triple coverage OK on candidates: triple-A {na} combos, triple-B {nb} combos")
