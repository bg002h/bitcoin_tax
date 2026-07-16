# SPEC-oracle-sweep — independent Fable architect re-review, r5 (GREEN)

*Persisted VERBATIM (STANDARD_WORKFLOW §2). Reviewer: Fable (independent architect pass, r5 — re-review
after the r4 fold). Reviewed against HEAD `ba65d73`, clean tree, 2026-07-15. **GREEN: 0C/0I.** The 1 Minor
+ 2 Nits below are non-gating and folded post-green.*

---

VERDICT: 0 Critical / 0 Important / 1 Minor / 2 Nit

**Disposition of r4-I1:** **Resolved.** The provenance class is now declared per-oracle O ∈ {OTS, taxcalc} at every required touchpoint (§6.2(b), §6.4 second class bullet, the §5.1 second pinned cell, the §12 liveness line), and the claimed composition is verified true against `method.rs` (details below).
**Disposition of r4-M1:** **Resolved.** §6.2(b) now correctly names the three **leaf** figures `qdcgt_line16` actually takes (`method.rs:74-91`: TI, QD, net-LTCG; remainder and slices derived internally at :83-90), all three demonstrably obtainable from the drivers today, with the fail-closed note stated. One citation-precision Nit survives (r5-N2).
**Disposition of r4-N1:** **Declined soundly.** The under-absorption argument for a mixed methodology+provenance household is correct, and the stacking backstop holds (verified below). N1 invited "record or decline"; declined with accurate rationale.

---

## Verification of the r4 fold

**r4-I1 → the per-oracle composition, checked arm by arm against `method.rs`:**

- **At/above the ceiling, `Table_btctax` is the exact schedule — TRUE.** `worksheet_tax` (`method.rs:47-56`): `amt < TAX_TABLE_CEILING` → Table cell; else `ordinary_tax_on(schedule, amt)` exact, cents carried, one final `round_dollar` (`:90`). The ceiling is inclusive-to-TCW (`:20-21`, confirmed by the `worksheet_tax(s, 100000) == 17,053.00` test at `:228`), so an operand exactly at $100,000 raises no boundary ambiguity. Empirically: taxcalc's exact TI for `mfj_se_over_the_addl_medicare_threshold` is 253,942.992 with L16 47,031.318; btctax's exact formula reproduces OTS's 47,031.31 at OTS's TI 253,942.94 (r4-verified), so at taxcalc's TI (+$0.052 at 24%) it lands ≈47,031.32 — conjunct 1 holds at whole dollars, and the class absorbs exactly the printed-vs-exact cents residual. A real btctax TCW/schedule bug sits inside the very `ordinary_tax_on` conjunct 1 evaluates, fails against O's independent exact figure, and stays red — teeth kept.
- **Below the ceiling (pure — all operands < $100k), the taxcalc class cannot fire — TRUE, and stronger than the spec's stated mechanism.** In general conjunct 1 fails outright (empirically: `single_crypto_business_se`, taxcalc TI 70,008.908 → `Table_btctax` bins to OTS's own 10,459 while round(taxcalc 10,454.96) = 10,455). In the near-bin-midpoint coincidence where conjunct 1 *holds*, conjunct 2 cannot simultaneously hold: printing moves an operand ≤ ~$0.50 while the midpoint sits $25 from each edge, so printed and exact operands share a bin, `Table_btctax(printed) = Table_btctax(exact) = round(O L16)` — the conjuncts are jointly unsatisfiable. No over-absorption path exists.
- **Mixed households (remainder below, TI at/above):** conjunct 1 *can* coincidentally hold (remainder near its midpoint) while the TCW-side cents satisfy conjunct 2 — but then conjunct 1 makes the absorbed diff identically `Table_btctax(printed leaves) − Table_btctax(exact leaves)`, a pure operand-provenance residual, so the firing is *lawful*, not over-absorption. The guarantee (§6.4 "cannot over-absorb") is correct; only the mechanism gloss is overbroad → r5-N1.
- **The one legitimate-residual seam I could construct** is the Table↔TCW regime-crossing straddle (exact TI a few dollars below the ceiling, printed chain crossing it) — measure-epsilon, unreachable in the deterministic corpus, OTS-side already absorbed, taxcalc-side covered by §10 triage → r5-M1, non-gating.
- **The taxcalc pinned cell is constructible and non-jamming.** Above the ceiling conjunct 1 holds by the arm verified above; the flip is steerable by the same checked SE-cents machinery r4 already blessed for the bin-edge cell (the generator holds both engines' exact figures offline; an unsteerable cell fails the bake loudly). In the same household the OTS side either agrees (single-oracle case) or fires its own provenance class (conjunct 1 holds — OTS's TCW at cents, 47,031.31-style, reproduced by btctax's exact formula), so §6.4's stacking rule is satisfiable either way.

**r4-M1:** `qdcgt_line16(schedule, bp, taxable_income, qual_div, net_ltcg)` takes exactly the three leaves; `bottom`/`pref` derive inside (`method.rs:83-85`). Driver-side: `_parse` keeps every OTS `Lxx` at printed cents (`ots_direct.py:164-171`; JSON carries `taxable_income: 253942.94`), L3a is a driver input (`:254`), the §1222(11) LTCG subterm is derived at `:292-294`, and taxcalc's TI-at-cents is baked (253,942.992) with QD as `e00650` (`gen_goldens.py:228`). The fail-closed property is stated in §6.2(b). Accurate.

**r4-N1 decline:** the backstop is real. A both-oracle disagreement forces the OTS conjuncts under stacking (conjunct 1 witnesses btctax's Table semantics against OTS's independently computed figure); a taxcalc-only disagreement means the paper matched OTS's Table figure exactly, which is itself the independent semantics witness. The irreducible residue — an identical OTS+btctax Table bug — is the known two-oracle common-mode limit accepted since r1 (it is why taxcalc is in the design at all), not a defect of this class.

**New-seam sweep (r5 fold only):** per-oracle declaration consistent across §6.2(b)/§6.4/§5.1/§12; the stacking rule composes with the new class (both-oracle above-ceiling cents flips pass per-predicate; a real TCW bug fails both conjunct-1s and stays red); both §12 liveness obligations present, one per pinned cell, neither cell double-loaded. Nothing gating found.

---

## Minor

### r5-M1 — the Table↔TCW regime-crossing straddle leaves a measure-epsilon lawful taxcalc residual unclassed; record its disposition (§10 triage) in one sentence — and do NOT "fix" it by widening the methodology condition

**Anchors:** spec §6.4 (methodology-class condition; "clean by construction"); `method.rs:49-55` (regime choice per operand); §6.2 (printed L15 is cross-footed, so printed−exact can exceed $0.50).

**Failure.** Household with exact TI just below $100,000 whose printed (cross-footed) TI lands at/above it: the paper's L16 is TCW-on-printed. Taxcalc side, if the ≤ rate×δ residual flips a rounded dollar: the methodology class is false (btctax's part-(b) lookup on printed operands consulted no Table), and the taxcalc provenance conjunct 1 is false (`Table_btctax` *bins* the below-ceiling exact operand, taxcalc doesn't) — a lawful $1 residual with no absorbing class. This cannot occur in the baked corpus (deterministic amounts; any bake-time red blocks commit per L-1) and is vanishingly rare per sweep draw (joint window ~10⁻⁵–10⁻⁷; §5.2 doesn't bias toward the Table ceiling); the OTS-side analogue of the same household is *correctly absorbed* by the OTS provenance class (conjunct 1 holds — Table on OTS's below-ceiling operands reproduces OTS's Table-based L16).

**Fix (one sentence, §6.4 or §10).** Note that the regime-crossing straddle's taxcalc residual is out-of-class by design and falls to §10 triage if the sweep ever surfaces one. **Explicitly do not** extend the methodology condition to "Table consulted on exact *or* printed operands": in this same household that widened class would also absorb a *real* btctax TCW bug (whose printed-operand evaluation is TCW, uncertified by either conjunct) — the condition-only status quo keeps that red, and a TCW bug is independently caught by every ordinary above-ceiling cell §5.1 mandates.

## Nit

### r5-N1 — §6.4's below-ceiling gloss is stated too universally

"Below the ceiling the taxcalc conjunct 1 *fails* … the methodology class … is the sole taxcalc absorber there" is true for pure below-ceiling households (and there the conjuncts are jointly unsatisfiable — stronger than stated), but a *mixed* household with its remainder near a bin midpoint can satisfy conjunct 1 while the TCW-side cents satisfy conjunct 2, so the provenance class can lawfully fire "below the ceiling in part," and it is then not the *sole* absorber. The normative predicate and the "cannot over-absorb" guarantee are correct as written; only the explanatory mechanism overreaches. Wording only.

### r5-N2 — the `ots_direct.py:292-294` citation names a QD-inclusive value; the QDCGT leaf is its QD-exclusive subterm

The value at `:292-294` is the 8995-L12 term — `qualified_dividends + max(0, min(ltcg, ltcg+stcg))`. `qdcgt_line16`'s `net_ltcg` parameter is the §1(h) term *without* QD (QD enters separately as L3a; `method.rs:66-68, 83`). The needed subterm is right there, but the plan's driver extension must expose `max(0, min(ltcg, ltcg+stcg))` alone — wiring the L12 value would double-count QD (caught loudly by the anchors, e.g. `single_miner_qbi…`'s QD 5,000, but cheaper to prevent with one clause). Provenance note: my own r4-M1 text carried the same imprecise citation; the fold copied it faithfully.

---

## Strengths (brief)

The r4-I1 fold is exact — the per-oracle predicate lands at all four touchpoints with the composition argument verified true against `method.rs` arm by arm, and the second pinned cell is constructible with the already-vetted steering machinery. Every citation in the folded text checks against current source.

**GREEN: 0 Critical / 0 Important.** The r4 fold is verified correct against source; the one Minor and two Nits are recorded with dispositions and do not hold the gate. **The spec is ready for an implementation plan** (fold r5-M1's one sentence and the two Nit clauses there or during planning — none blocks).
