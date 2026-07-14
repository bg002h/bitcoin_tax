# Fable Second Pass — Reconciliation (recon COMPLETE)

**Date:** 2026-07-11 · **Pass:** Fable round 2 (verify + deepen + critic), F1–F6. **Status:** recon
complete; ready for the spec. Reports: `fable/01`–`fable/06`. Reconciles against opus `00`–`05` + `deep/01`–`05`.

**Headline:** the computation core (Tax-Table/QDCGT method, absolute NIIT/8959, deductions, field maps) is
**spec-ready and adversarially confirmed to the cent**. The remaining work is **3 BLOCKERS in the
"everything around the core" layer** (Schedule 1 surface; form-set closure; whole-return rounding/sign) plus
12 IMPORTANT gaps — all itemized by F6 and all resolvable *in* the spec.

---

## (a) CONFIRMED — locked findings that survived adversarial re-verification

- **Tax-Table + QDCGT method (`deep/01`) — CONFIRMED [F2].** Half-up rounding *proven* (structural
  whole-region proof + 5 discriminator cells; couldn't be falsified); bin structure, midpoint rule, $100k
  boundary, TCW≡`ordinary_tax_on`, MFS≡Single, all 25 QDCGT lines, and all 3 worked examples re-derived to
  the cent. `line16 = round_dollar(min(L23,L24))` stands.
- **Derivation + absolute 8960/8959 (`deep/02`) — CONFIRMED, 0 discrepancies [F4].** 6 attempts to break
  `MAGI=AGI` failed; NII-rebuild, W2 owner-tag, Sch 2 L4 unbundling, and the reduce-to-delta invariant all
  verified against the 2024 forms + frozen code; both worked examples ($2,242 vs $1,596; 8959 both sides) to
  the cent.
- **TY2024 field maps (`deep/03`) — CONFIRMED, 13/13 spot-check, zero mismatches [F5].**
- **Standard-deduction + Schedule A / §170(b) engine (`deep/04`) — CONFIRMED** (F6 §5; ST-crypto-50%
  correction propagated), *plus* the G8 carryover-aging addition below.

## (b) CORRECTIONS to locked findings (fold into the spec)

| ID | Correction | Affects | Source |
|---|---|---|---|
| **F2 F-A** | QDCGT `pref` input must be **capped: `pref_ws = min(TI, qd+ltcg)`** (worksheet L10). Uncapped overstates line 16 (counterexample: TI 35,400 / QD 50,000 → $0, not $446). One clamp in `qdcgt_line16`. | **v1 (TY2024)** correctness | `fable/02` |
| **F2 F-B** | "`min(L23,L24)` non-binding" is **false** — same-$50-bin cases bind and change rounded line 16. Method already computes the min (safe); fix commentary + add KAT. | v1 | `fable/02` |
| **F2 F-C/D/E** | Rounding rule is 2024 i1040 **p. 23** (not p. 22); table-vs-formula gap ceiling is **$6.00** for TY2024 (not $9.75); worksheet-internal "carry cents, round once" is the decided convention — state it. | doc/minor | `fable/02` |
| **F1** | **SALT MFS phase-down halves LAST**, not both constants: `cap = max(10k, 40k − 0.30·(MAGI−500k)); cap_final = mfs ? cap/2 : cap` (effective 15% slope, $5k floor at MAGI $350k). Opus overstated by up to $7,500 in the $250–350k MFS band. | **TY2025 follow-on** | `fable/01` |
| **F5** | 1040 leaf **`f1_57` = L12 (2024) but L1z (2025)** — cross-year collision; filing-status **on-states re-assigned** (MFJ `/3`→`/2`, MFS `/4`→`/3`, HOH `/2`→`/4`); Sch 1 & Sch A **roots flip**; income-line container `Line4a-11_ReadOrder` gone in 2025; dependents grid transposed. Per-(form,year) maps + geometric read-back are the defense. | TY2025 maps | `fable/05` |
| **F3** | Phase-out rounding is **statutorily asymmetric** (tips/OT floor; car-loan/CTC ceil; senior smooth 6%) → shared helper needs a rounding-mode param; **2025 Form 8995 L11 subtracts 1040 L13b** (Sch 1-A shrinks the QBI limit → order AGI→1-A→8995); **capture DOB + SSN-validity, not booleans**; **ACTC is a Stage-7 credit** (reads Sch 2/Sch 3). | follow-ons + v1 data model | `fable/03` |

## (c) NEWLY RESOLVED (opus open flags closed)

- **TY2025 = GO [F1].** Final 2025 forms exist and were read: Schedule A keeps the 2024 charitable/itemized
  structure (only SALT 5e changed); the three "2026 scare" items (0.5% charitable floor, 90% gambling, new-§68
  cap) are confirmed **statutorily TY2026**, absent from 2025. Every OBBBA dollar figure confirmed against
  enacted Pub. L. 119-21 + final Pub 501 (std ded $15,750/$31,500/$23,625; SALT $40k/$20k; senior $6k; §63(f)
  $1,600/$2,000). Bundle-ready parameter table in `fable/01` §5. LTCG breakpoints/brackets = Rev. Proc.
  2024-40 (only std-deduction + SALT are OBBBA-overridden for 2025).
- **TY2025 field maps [F5]:** all six root FQNs + full leaf skeletons extracted from the *final* PDFs (no
  draft caveat). Schedule 1-A is the known **7th-map TODO**.
- **Follow-on math [F3]:** Schedule 1-A (all parts, phase-outs), QBI/8995, CTC/ACTC/8812 all specced to
  formula grade for the follow-on cycles.

## (d) BLOCKERS + IMPORTANT gaps the SPEC must resolve (from F6, `fable/06`)

**3 BLOCKERS** (spec cannot be written without deciding these):
1. **G1 — Schedule 1 input surface undefined.** `Sch1Income`/`Sch1Adjustments` are named but never
   enumerated; each candidate line hides an unmodeled worksheet (state-refund §111, IRA phase-out, student-loan
   phase-out, HSA→8889). **Resolution:** publish an enumerated v1 Sch 1 line list, each with a policy from
   {full worksheet | attest-already-limited + advisory | **refuse (fail-closed)**}. Recommended v1 minimum:
   L1 (user enters taxable portion + advisory), L7 (1099-G struct), L15/L18 (already derived), L21 (full
   worksheet — small), L20 (worksheet or refuse-when-box-13), L13 refuse; everything else → LIMITATIONS.
2. **G2 — Form-set not closed under "Attach Form X."** Sch 2 L11/L12 and 1040 L13 mandate attaching
   **8959/8960/8995** — none has a filler/map; and the existing Schedule D filler **scopes out lines 17–22
   including the mandatory §1211 line-21 loss line** (`schedule_d.rs:5-6`). **Resolution:** extend the Sch D
   filler to L17–22; add 8959 + 8960 fillers (1-page, same XFA family — a scheduled F5-style extraction);
   decide QBI as add-8995-map or hard-refuse-when-box5>0-without-8995 (never print a non-DRAFT return with an
   unbacked line).
3. **G3 — No whole-return rounding + negative-sign convention.** `deep/01` fixed only QDCGT L25.
   **Resolution:** adopt the IRS **round-all-amounts** election globally (every form-line entry `round_dollar`ed
   at the line; carry cents within a worksheet and round once at the form line; inputs accepted in cents,
   rounded at first form-line use); add a per-(form,line) **sign policy** (`neg: minus|parens|magnitude`) to
   the map schema and KAT a loss-year 1040 L7 = −3,000 fill + read-back.

**12 IMPORTANT** (spec must address — full detail in `fable/06` §3): G4 one `resolve_profile()` +
provenance everywhere (+ `tax-profile set` guard); G5 `W2` struct add box 4/6/19 + owner (box 6 contradicts
the locked 8959 Part V); G6 excess-SS credit (per-person, 2-employer, $10,453.20 cap → Sch 3 L11); G7
**Schedule A must read WITH-crypto AGI** (charitable ceilings move with gains) — absolute path uses with-crypto
AGI, delta path documents its deduction as approximate; G8 **charitable carryover ages even in std-deduction
years** (Reg. §1.170A-10(a)(2)); G9 enumerated refuse-guard table for captured-but-inert inputs (box-12
W/A/B/M/N/Z, box 8/10, INT box 9/DIV box 13); G10 foreign-tax §904(j) ≤$300/$600 no-1116 path (capture INT
box 6 / DIV box 7); G11 Schedule B Part III inputs (foreign-account/trust booleans); G12 Sch A line-5a
composition (double-count trap) + sales-tax election box; G13 AMT screen-or-document decision; G14 fold the
12 new KATs into the test plan; G15 MFS-both-itemize as tri-state driving both the math and the header box.

**~10 MINOR** (G16–G25): direct-deposit/L36/L38, IP PIN, extension payment (Sch 3 L10), `other_withholding`
gate, "Sch D not required" box policy (btctax always files Sch D), derive dependent-filer earned income,
TI≤0 refuse, always-file Sch B, LIMITATIONS entries (EIC/1040-SR/no-state/no-1099-R/AMT), PII/vault security.

---

## Spec-readiness verdict

**Ready to draft.** The core math is locked and confirmed; the corrections in (b) are small and enumerated;
TY2025 is de-risked for the follow-on. The 3 blockers are **design decisions with recommended resolutions in
hand** (F6), not unknowns. The spec's job: resolve G1/G2/G3, address G4–G15, adopt the fail-closed posture
for everything unmodeled, and fold the (b) corrections + the 12 new KATs into the test plan. v1 stays
**TY2024, Common W-2 household, PDF-only, permissive+distributable**, with the full legal apparatus retained.
