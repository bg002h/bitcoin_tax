#!/usr/bin/env python3
"""E2 — generate the AMT-owing vector population, one vector per LIVE Part III routing per status.

    .venv/bin/python design/amt-form6251/gen_e2_vectors.py [--write]

Reads `crates/btctax-core/src/tax/fixtures/form6251_vectors.json`, re-derives every vector already
in it from its INPUTS (a regression guard: the derivation below must reproduce V1-V10 to the cent
or the run aborts), then appends the E2 population.

★ WHY A GENERATOR AND NOT HAND-ENTERED FIGURES. Each vector asserts its own intended routing at
generation time — `expect` names the branch the vector exists to exercise, and a vector whose inputs
drift off that branch fails HERE rather than silently becoming a duplicate of its neighbour. That is
the failure mode a fixture of look-alike high-income households is most prone to: V11 and V14 differ
only in whether Form 6251 line 18 takes the 26% or the 28% side of §55(b)(1), and nothing in the
committed JSON would ever say so.

Every figure is computed by `f6251_reference.py` — the line-by-line transcription of the form — and
then cross-checked against BOTH oracles by `scripts/oracle/verify_f6251.py`. This file is not an
oracle; it is the thing the oracles check.
"""
import argparse, json, pathlib, sys
from decimal import Decimal as D

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from f6251_reference import form6251, qdcgt, STD, EXM, PHASE, BP28, L19  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "crates/btctax-core/src/tax/fixtures/form6251_vectors.json"

# §170(b)(1)(G): cash gifts to public charities are deductible to 60% of AGI. Every E2 vector keeps
# the gift far under it — OTS 2024 applies no such ceiling, and a vector that crosses it loses an
# oracle (that is what disqualified V2b).
CASH_CEILING = D("0.6")
KICK_START = D(875950)  # i6251 p.9, line 4 — and, for MFS, the zero-exemption threshold as well

# ★★★ §164(b)(6) — the SALT cap, and why a vector needs it at all (G-6d).
#
# FOLLOWUPS §G-6d, verbatim: *"no fixture vector has Schedule A line 7 > 0. Every itemizing vector
# deducts a cash gift only, so the fixture drives line 2a's itemizer limb at zero throughout."*
# Form 6251 line 2a is *"Enter the amount from Form 1040 … Schedule A, line 7"* on an ITEMIZING
# return — the taxes add-back, and the one figure on such a filer's Form 6251 that is not just
# taxable income re-stated. With every vector at SALT = 0 that limb was exercised by nothing but a
# unit KAT. §164(b)(6) caps the deduction at $10,000 ($5,000 MFS) for TY2024, so a vector whose
# state and local taxes exceed the cap lands Schedule A line 7 exactly ON it.
SALT_CAP = {"mfj": D(10_000), "mfs": D(5_000), "single": D(10_000), "hoh": D(10_000)}


def derive(st, wages, ltcg, gift, refund, ftc, salt=D(0)):
    """The 1040 figures a vector's inputs imply, then the whole Form 6251.

    `salt` is the filer's state and local taxes BEFORE §164(b)(6); Schedule A line 7 is the capped
    figure, and it is what Form 6251 line 2a adds back on an itemizing return.
    """
    agi = wages + ltcg + refund
    allowed = min(gift, CASH_CEILING * agi)
    sch_a_line7 = min(salt, SALT_CAP[st])
    itemized_total = allowed + sch_a_line7
    itemized = itemized_total > STD[st]
    deduction = itemized_total if itemized else STD[st]
    ti = max(D(0), agi - deduction)
    regular_tax, _ = qdcgt(ti, ltcg, st)
    f = form6251(
        st=st, taxable_income_L15=ti, agi_L11=agi, deduction_L14=deduction,
        # A standard-deduction filer has no Schedule A at all, so line 2a takes the standard
        # deduction instead and this figure is not consulted — but pass zero rather than a stale
        # number, so a mis-set `itemized` cannot quietly add back a line that is not on the return.
        sch_a_line7=sch_a_line7 if itemized else D(0),
        itemized=itemized, refund_sch1_L1=refund,
        net_ltcg=ltcg, qual_div=D(0), regular_tax_L16=regular_tax,
        sch2_L1z=D(0), sch3_L1=ftc,
    )
    derived = {
        "agi_1040_L11": agi, "deduction_1040_L14": deduction, "itemized": itemized,
        "charitable_allowed": allowed, "taxable_income_1040_L15": ti,
        "regular_tax_1040_L16": regular_tax,
        # ★ The CAPPED figure, emitted separately from the raw `state_local_tax` input: the Rust
        #   fixture loader reads THIS (it is Form 6251 line 2a's operand), while `verify_f6251.py`
        #   hands the RAW amount to each oracle and lets the oracle apply its own §164(b)(6) — which
        #   is what makes the cap itself independently witnessed rather than assumed.
        "schedule_a_line7": sch_a_line7 if itemized else D(0),
    }
    return derived, f


def routing(st, f):
    """The branch flags a vector claims to exercise. Names match `expect` keys."""
    return {
        "amt": f[11] > 0,
        "attach": f[7] > f[10],
        # Part III runs only on line 7's middle bullet; every E2 vector has capital gain, so it does.
        "part3": 12 in f,
        # line 23 — "This amount is taxed at 0%". Live only when the regular-tax ordinary bottom
        # (line 20) leaves room under line 19's 0%-band ceiling.
        "l23_pos": f[23] > 0,
        # line 32's skip — "If lines 32 and 12 are the same, skip lines 33 through 37".
        "skip32": f[32] == f[12],
        # which side of §55(b)(1) line 18 takes (the Part III ordinary slice)…
        "l18_26": f[17] <= BP28[st],
        # …and which side line 39 takes (the whole-base ceiling).
        "l39_26": f[12] <= BP28[st],
        # §55(d)(3): "none" = full exemption, "partial" = on the 25% ramp, "full" = phased to zero.
        "phaseout": "none" if f[5] == EXM[st] else ("full" if f[5] == 0 else "partial"),
        "l8_live": f[8] > 0,            # the §904(j) elector's AMT foreign tax credit
        "l2b_live": f["2b"] != 0,       # the state-refund subtraction
        "l2a_live": f["2a"] != 0,       # the standard-deduction add-back (§56(b)(1)(D))
        "l20slice": f[33] > 0,          # a 20% tranche survives lines 32/33
        # ★ lines 20/27 = the QDCGT worksheet's line 5, the REGULAR-tax ordinary bottom. Zero exactly
        #   when the preferential slice reaches or exceeds taxable income — a routing no vector
        #   exercised before V30, and the one where line 21 gets the whole 0% band.
        "l20_zero": 20 in f and f[20] == 0,
        "kicker": st == "mfs" and f[4] > KICK_START,
    }


# ── The population ────────────────────────────────────────────────────────────────────────────
# `expect` is asserted, not documented: every key listed must match the computed routing.
#
# R1 exemption phased to ZERO · line 23 = 0 · line 18 on the 28% side   — the deep-AMT case
# R2 exemption phased to ZERO · line 23 > 0                            — the AMT 0% band is live
# R3 exemption on the 25% RAMP                                          — §55(d)(3) partially applied
# R4 exemption phased to ZERO · line 23 = 0 · line 18 on the 26% side   — 26/28 without the line-23 confound
#
# ★ MFS R1/R2/R4 have NO ORACLE AT ALL, and that is arithmetic, not an accident.
#   §55(d)(3) puts the MFS zero-exemption threshold and the Form 6251 line-4 kicker start at the
#   SAME $875,950 (609,350 + 4 x 66,650), so for MFS "exemption gone" and "kicker live" are one
#   condition — a search over the whole box found ZERO MFS returns with a zeroed exemption below
#   the kicker start. And the kicker is precisely where BOTH engines fail: OTS 2024 carries the
#   stale 2023 MFS triple, and Tax-Calculator does not model the line-4 add-back at all (it models
#   the other limb of §55(d)(3), the exemption cliff, via `AMT_em_pe`). Two oracles, one blind
#   spot, exactly aligned.
#
#   So these three are KATs against the transcription and the i6251 p.9 worked example, NOT
#   validated figures, and `verify_f6251.py`'s witness census prints them as unwitnessed every run
#   rather than letting two separate "OK" lines imply otherwise. **V22 — the phase-out ramp, below
#   the kicker start — is the two-oracle MFS vector the Tier-2 gate actually requires.**
SPEC = [
    # id     status    wages     ltcg       gift     refund  ftc  why
    ("V11", "single", 280_000, 700_000,   25_000,      0,     0,
     "R1 single — exemption phased to zero, line 23 = 0, line 18 on the 28% side",
     dict(amt=True, phaseout="full", l23_pos=False, l18_26=False, l39_26=False, skip32=False)),
    ("V12", "single",  55_000, 925_000,   25_000,      0,     0,
     "R2 single — the AMT 0% band (line 23) is live",
     dict(amt=True, phaseout="full", l23_pos=True, l18_26=True, skip32=False)),
    ("V13", "single", 200_000, 625_000,   25_000,      0,     0,
     "R3 single — the exemption is on the 25% phase-out ramp, not zeroed",
     dict(amt=True, phaseout="partial", l23_pos=False, l18_26=True)),
    ("V14", "single", 205_000, 775_000,   25_000,      0,     0,
     "R4 single — exemption zeroed, line 23 = 0, line 18 on the 26% side",
     dict(amt=True, phaseout="full", l23_pos=False, l18_26=True, l39_26=False)),

    ("V15", "hoh",    280_000, 700_000,   25_000,      0,     0,
     "R1 hoh — exemption phased to zero, line 23 = 0, line 18 on the 28% side",
     dict(amt=True, phaseout="full", l23_pos=False, l18_26=False, skip32=False)),
    ("V16", "hoh",     80_000, 900_000,   25_000,      0,     0,
     "R2 hoh — the AMT 0% band (line 23) is live; line 19 is hoh's own $63,000",
     dict(amt=True, phaseout="full", l23_pos=True, l18_26=True, skip32=False)),
    ("V17", "hoh",    200_000, 600_000,   25_000,      0,     0,
     "R3 hoh — the exemption is on the 25% phase-out ramp",
     dict(amt=True, phaseout="partial", l23_pos=False, l18_26=True)),
    ("V18", "hoh",    205_000, 775_000,   25_000,      0,     0,
     "R4 hoh — exemption zeroed, line 23 = 0, line 18 on the 26% side",
     dict(amt=True, phaseout="full", l23_pos=False, l18_26=True)),

    # MFJ's R1 is V6 (a donation CREATES AMT), already in the fixture.
    ("V19", "mfj",    130_000, 1_675_000, 50_000,      0,     0,
     "R2 mfj — the AMT 0% band (line 23) is live",
     dict(amt=True, phaseout="full", l23_pos=True, l18_26=True, skip32=False)),
    ("V20", "mfj",    435_000, 1_025_000, 50_000,      0,     0,
     "R3 mfj — the exemption is on the 25% phase-out ramp",
     dict(amt=True, phaseout="partial", l23_pos=False, l8_live=False)),
    ("V21", "mfj",    280_000, 1_525_000, 50_000,      0,     0,
     "R4 mfj — exemption zeroed, line 23 = 0, line 18 on the 26% side",
     dict(amt=True, phaseout="full", l23_pos=False, l18_26=True)),

    ("V22", "mfs",    215_000, 525_000,   25_000,      0,     0,
     "R3 mfs — the ramp, BELOW the kicker start: the only AMT-owing MFS routing OTS can witness",
     dict(amt=True, phaseout="partial", kicker=False, l23_pos=False)),
    ("V23", "mfs",    205_000, 700_000,   25_000,      0,     0,
     "R1 mfs — exemption zeroed, which for MFS necessarily means the line-4 kicker is live too",
     dict(amt=True, phaseout="full", kicker=True, l23_pos=False, l18_26=False)),
    ("V24", "mfs",     55_000, 850_000,   25_000,      0,     0,
     "R2 mfs — the AMT 0% band is live, with the kicker; line 18 on mfs's own $116,300 side",
     dict(amt=True, phaseout="full", kicker=True, l23_pos=True, l18_26=True)),
    ("V25", "mfs",    130_000, 775_000,   25_000,      0,     0,
     "R4 mfs — exemption zeroed, line 23 = 0, line 18 on the 26% side, kicker live",
     dict(amt=True, phaseout="full", kicker=True, l23_pos=False, l18_26=True)),

    # ── cross-cutting: one input moved against a control vector, so a mutation is diagnosable ──
    ("V26", "mfj",    435_000, 1_025_000, 50_000,      0,   600,
     "R5 mfj — line 8 LIVE with AMT > 0. V20 is the same household with no FTC: the §904(j) "
     "elector's credit must move lines 8/9/10 and leave line 11 untouched",
     dict(amt=True, l8_live=True, attach=True)),
    ("V27", "single", 200_000, 625_000,   25_000,      0,   300,
     "R5 single — line 8 live at the $300 non-MFJ §904(j) de-minimis. Control: V13",
     dict(amt=True, l8_live=True, attach=True)),
    ("V28", "mfj",    420_000, 875_000,        0,      0,     0,
     "R7 mfj — the STANDARD deduction add-back (line 2a) landing on the phase-out ramp, where it "
     "is clawed back at 25% on the margin",
     dict(amt=True, phaseout="partial", l2a_live=True, l23_pos=False)),
    ("V29", "single", 195_000, 650_000,   25_000,  5_000,     0,
     "R8 single — line 2b (the state-refund subtraction) with AMT > 0, so dropping it moves the "
     "AMT itself and not merely AMTI. Control: V13, same $175,000 ordinary bottom",
     dict(amt=True, l2b_live=True, phaseout="partial")),

    # ── G-6d: the itemizer limb of line 2a, at last ────────────────────────────────────────────
    #
    # ★★★ The household FOLLOWUPS §G-6d names. Every vector above deducts a cash gift only, so
    #     Form 6251 line 2a's ITEMIZER limb — "the amount from Schedule A, line 7" — has been zero
    #     throughout, held by one unit KAT and by nothing independent. Here SALT of $25,000 is
    #     capped by §164(b)(6) to $10,000, and that $10,000 IS this filer's entire AMT preference:
    #     the one number on their Form 6251 that is not just taxable income re-stated.
    #
    # ★★ It is also a Part III routing no vector exercises: wages are deliberately BELOW the
    #    $60,000 itemized total, so the long-term gain EXCEEDS taxable income, the QDCGT worksheet's
    #    line 5 is driven to zero, and Form 6251 lines 20 and 27 print $0 — the branch where line
    #    21 ("subtract line 20 from line 19") gets the whole 0% band.
    #
    # ★ The gift stays far under the §170(b) 60%-of-AGI ceiling ($984,000 here), so OTS remains a
    #   witness; the SALT is what is new, not the charitable branch.
    ("V30", "mfj",     40_000, 2_000_000, 50_000,      0,     0,
     "G-6d mfj — SCHEDULE A LINE 7 > 0 at last: $25,000 of state and local taxes capped by "
     "§164(b)(6) to $10,000, which is this filer's whole Form 6251 line 2a add-back. Also the "
     "first vector whose long-term gain EXCEEDS taxable income, so the QDCGT worksheet's line 5 "
     "is zero and lines 20/27 print $0",
     dict(amt=True, attach=True, part3=True, l2a_live=True, l20_zero=True,
          phaseout="full", l23_pos=True, l18_26=True, l20slice=True), 25_000),
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true", help="update the fixture in place")
    args = ap.parse_args()

    doc = json.loads(FIXTURE.read_text())
    existing = {v["id"]: v for v in doc["vectors"]}

    # ── regression guard: this derivation must still reproduce every committed vector ──
    bad = 0
    for v in doc["vectors"]:
        i = v["inputs"]
        # ★ `state_local_tax` is absent from V1-V29, which predate G-6d. Absent means ZERO — those
        #   vectors deduct a cash gift only, which is precisely the gap V30 exists to close — and
        #   the regression guard below still re-derives every one of their committed figures, so a
        #   vector that silently GAINED a SALT it should not have would drift and abort.
        d, f = derive(i["filing_status"], D(i["wages"]), D(i["net_ltcg"]), D(i["cash_gift"]),
                      D(i["state_refund"]), D(i["sch3_line1_ftc"]),
                      D(i.get("state_local_tax", "0")))
        for k, want in v["derived"].items():
            got = d[k]
            same = got == want if isinstance(want, bool) else D(got) == D(want)
            if not same:
                print(f"  DRIFT {v['id']}.{k}: derived {got}, fixture {want}")
                bad += 1
        for k, want in v["form6251"].items():
            n = k[len("line"):]
            got = f.get(int(n) if n.isdigit() else n)
            if got is None or D(got) != D(want):
                print(f"  DRIFT {v['id']}.{k}: derived {got}, fixture {want}")
                bad += 1
    if bad:
        print(f"ABORT: {bad} committed value(s) no longer reproduce from their inputs.")
        return 1
    print(f"regression guard OK — all {len(doc['vectors'])} committed vectors reproduce")

    new = []
    for row in SPEC:
        # ★ A row is 9 fields, or 10 with a trailing state-and-local-tax amount. Any other width is
        #   a typo in the table, and it ABORTS rather than unpacking into the wrong columns — the
        #   failure mode a positional table is prone to, and one that would shift `expect` onto
        #   `why` and silently stop asserting the routing.
        if len(row) == 9:
            vid, st, wages, ltcg, gift, refund, ftc, why, expect = row
            salt = 0
        elif len(row) == 10:
            vid, st, wages, ltcg, gift, refund, ftc, why, expect, salt = row
        else:
            print(f"ABORT: SPEC row {row[0]} has {len(row)} fields; expected 9 or 10")
            return 1
        # ★ Already committed ⇒ SKIP, never overwrite. This used to ABORT, which made the script
        #   single-use: `SPEC` is the population's permanent record, so the moment V11-V29 were
        #   written the generator could never be run again to append a thirtieth. Skipping keeps the
        #   safety it was protecting (a committed vector is never silently rewritten — and the
        #   regression guard above has ALREADY re-derived it from its own inputs and aborted on any
        #   drift) while letting the population grow.
        if vid in existing:
            print(f"  [skip] {vid} already committed — re-derived by the regression guard above")
            continue
        d, f = derive(st, D(wages), D(ltcg), D(gift), D(refund), D(ftc), D(salt))
        got = routing(st, f)
        for k, want in expect.items():
            if got[k] != want:
                print(f"ABORT: {vid} claims {k}={want} but computes {got[k]}\n  {why}")
                return 1
        # A vector whose gift is capped has lost OTS as a witness (V2b's fate) — never silently.
        if D(gift) > CASH_CEILING * d["agi_1040_L11"]:
            print(f"ABORT: {vid}'s gift crosses the §170(b) ceiling and would lose an oracle")
            return 1
        new.append({
            "id": vid,
            "why": why,
            "inputs": {"filing_status": st, "wages": str(wages), "net_ltcg": str(ltcg),
                       "cash_gift": str(gift), "state_refund": str(refund),
                       "sch3_line1_ftc": str(ftc),
                       # The RAW state and local taxes, pre-§164(b)(6). Each oracle applies its own
                       # cap, so the cap is witnessed rather than assumed; the capped figure lives
                       # in `derived.schedule_a_line7`.
                       "state_local_tax": str(salt)},
            "derived": {k: (v if isinstance(v, bool) else str(v)) for k, v in d.items()},
            "form6251": {f"line{k}": str(v) for k, v in f.items()},
            "attach_required_who_must_file_cond1": bool(f[7] > f[10]),
        })
        print(f"  {vid:5} {st:6} AMT={f[11]:>12,.2f}  " +
              " ".join(f"{k}={v}" for k, v in got.items() if v not in (False, "none")))

    doc["vectors"].extend(new)
    doc["_note"] = (
        "V1-V6 were independently derived in review r1 BEFORE any code existed and are reproduced "
        "here to the cent; V2b/V7-V10 are constructed. V11-V29 are the E2 population "
        "(FOLLOWUPS §G-6b): one AMT-owing vector per live Part III routing per filing status, "
        "emitted by design/amt-form6251/gen_e2_vectors.py, which asserts each vector's intended "
        "routing at generation time. Figures are EXACT CENTS (plan §2)."
    )
    if args.write:
        FIXTURE.write_text(json.dumps(doc, indent=2) + "\n")
        print(f"\nwrote {len(new)} vectors -> {FIXTURE.relative_to(ROOT)} "
              f"({len(doc['vectors'])} total)")
    else:
        print(f"\n(dry run: {len(new)} vectors would be appended; pass --write)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
