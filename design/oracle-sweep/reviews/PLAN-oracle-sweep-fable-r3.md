# PLAN-oracle-sweep — independent Fable plan re-review, r3 (GREEN)

*Persisted VERBATIM (STANDARD_WORKFLOW §2). Reviewer: Fable (independent plan review). Reviewed
`design/IMPLEMENTATION_PLAN_oracle_sweep.md` @ HEAD `7f3eb39` (r3 fold of my r2 0C/4I/3M/5Nit) against
GREEN spec r5. All r2-fold claims verified against current source and numerically against the baked
`full_return_goldens.json` (all 12 anchors dumped and compared). Persisted 2026-07-16. **GREEN: 0C/0I.**
The 3 Minors + 2 Nits are non-gating and folded post-green.*

---

VERDICT: 0 Critical / 0 Important / 3 Minor / 2 Nit

**One-line dispositions of the r2 findings:**

- **r2-I1 — FIXED.** The T2 test is now a synthetic-literal flip test (`274.50/499.50 → 775 ≠ round(774.00) = 774`), which is the repo's own worked doctrine verbatim (`other_taxes.rs:80-85` — half-up confirmed there and at `method.rs:6` "round_dollar (IRS half-up)"); it is non-tautological (a `round(Σ)` mis-implementation yields 774 and fails), needs no JSON read, and the read-the-JSON instruction survives only for the `table_l16`/`consulted_table` literals, which ARE baked and exact — re-verified: OTS TI 253,942.94 / L16 47,031.31 (independently re-derived from the 2024 MFJ schedule: 34,337 + 24% × 52,892.94 = 47,031.31 ✓), inputs QD 8,000 / LTCG 25,000 / TI 112,400 (remainder 79,400 < ceiling ✓), taxcalc TI 70,008.908 / L16 10,454.9598 ✓.
- **r2-I2 — FIXED.** The QBI-deduction row is in the C1 table with the printed-8995-chain derivation matching `qbi.rs:195-211` exactly (L15 = min(L10, round(20% × (round(TI_bq) − round(ncg))))), it reuses the existing `qbi_deduction` field (verified present and **required** on both structs, `testonly.rs:403,418` — no new field), the compute-level row is kept in T5's six, and the exact-.50/min-near-tie epsilon is assigned to §10 triage, not a class.
- **r2-I3 — FIXED, and the fix is executable.** `ots_direct.py:341` is confirmed still `L5a = max(ltcg,0) + max(stcg,0)`; btctax's 8960 L5a is confirmed the §1211-limited 1040-L7 figure, negative in a loss year (`other_taxes.rs:219-222` and `:308` "may be NEGATIVE (a §1211-limited loss)"); the T8 fix feeds exactly that quantity, and it IS available at the build point — the 8960 block (`:329-346`) runs after pass-2 `final` (`:311-322`), so `final.get("L7")` (T8's named source for `sch_d_to_l7`) is in scope. L13 stays pass-1 cents-MAGI with the epsilon honestly assigned to §10 triage; the false "printed L17" gloss is gone; T11 cause (iii) names the driver-error route that would otherwise have produced a false btctax KNOWN-DEFECT pin.
- **r2-I4 — FIXED, and the two-level rule is internally consistent.** T5 keeps HEAD's six rows (verified: `golden_returns.rs:249-297` compares `ar.qbi_deduction`/`ar.agi`/`ar.taxable_income`/`ar.se_tax_sch2_l4`/`ar.additional_medicare.additional_medicare_tax`/`ar.niit.tax` against `e.*` AND `t.*` — every field the rule needs exists on both sides today, so "compute-level" is well-defined for AGI/TI). The TI/AGI double comparison is **coherent, not contradictory**: the compute row holds `round_dollar(exact)` equality across engines (as HEAD), the T6 paper row holds the printed cross-foot against the §3.1 reproduction on oracle leaves — different quantities, different guarantees, both green today, and post-T11 the reproduction absorbs the Σround residual the exact rows never see. L16/L24 are correctly NOT forced exact-vs-exact (OTS exact total 49,568.75-class residuals are lawful); L24's compute comparison reads the **printed** chain (`printed.f1040.line24`, built at `:243`, used at `:297` — the r2-M2 fix landed). The interim dip is cured: the six rows need no new fields and pass at T5 (verified below).

**Numeric verification of the T5 green gate (all 12 anchors, from the baked JSON):** all six kept lines agree at §3.1 whole dollars on all 12 households; the L16 dispositions reproduce (the 5 Table entries at `:95-144` + `single_crypto_business_se` at `:192-202` all satisfy `consulted_table`; `mfj_se` needs no class — table_l16 on both oracles' TI rounds to 47,031 = btctax; the miner's L24 16,833 = 8,355 + 8,478 dissolves). The `checked==3`/`round(exact se_tax)` shapes at `golden_packet.rs:161-185` and the form-set map at `:300-350` are exactly as the plan cites; `tests/common/mod.rs` exists with `#![allow(dead_code)]` consumed by kats/overflow/sp3/sp3b; `serde_json` is genuinely absent from `btctax-forms/Cargo.toml` (T7's bin-crate reasoning holds); `SCHEDULE_D_MAP_2024` is exported via `testonly` (`lib.rs:377-382`) with `line21` mapped and marked ★ PAREN.

**r2-M1/M2/M3, N1–N5 — all landed and verified against source** (capital leg = §1211-netted per `printed.rs:533-538` ✓; four-cause T11 triage ✓; `other_taxes.rs:320` cite ✓; salt driver-derived note with `US_1040`-carried A5a/5b ✓ (`ots_direct.py:261-268`); owning-phase at every filing site ✓; `printed.rs:518-542` cite ✓; `Some(&ops)` ✓).

---

## Minor

### r3-M1 — "agree cent-exact across all three engines today" is factually overclaimed for TI/AGI/QBI; the agreement HEAD relies on is at §3.1 whole dollars

**Touches:** the C1 table's two-level block (¶ "keep their exact-vs-BOTH-oracles"), T5 Produces/step 2, T11 step 3 (iv).

Dumped from the baked JSON: `se_tax` is bit-equal OTS-vs-taxcalc on all 12; `niit` bit-equal; `additional_medicare` cent-exact (one 1e-13 float echo). But **TI/AGI/QBI differ at the cents level on all four QBI households** — e.g. `single_crypto_business_se` TI 70,008.94 (OTS) vs 70,008.908 (taxcalc), QBI 11,152.20 vs 11,152.227; `mfj_se…` TI 253,942.94 vs 253,942.992 (5.2¢) — because OTS line-rounds its 8995 chain (as btctax does) while taxcalc doesn't. The comparison that passes is HEAD's `round_dollar`-both-sides, and the plan's operative instruction ("as HEAD, `golden_returns.rs:249-297`") is therefore **correct**; only the rationale sentence is false, and an executor who takes "cent-exact" literally and asserts raw-cents equality goes red at T5 step 3 on `single_crypto_business_se` (self-correcting at the task's own boundary, hence Minor). **Fix:** say "agree at §3.1 whole dollars today (bit-equal on SE tax/NIIT; within ~5¢ on TI/AGI/QBI where the 8995 chain line-rounds) — keep HEAD's `round_dollar`-both-sides rows verbatim," and extend T11 cause (iv)'s parenthetical to note the compute-level TI/AGI rows inherit the QBI-chain epsilon by subtraction (a corpus cell straddling a .50 boundary within those cents is a cause-(iv) triage, not a class).

### r3-M2 — the pre-T11 shape of the rows whose reproduction consumes not-yet-baked leaves is unstated; T5 step 3's "dissolves" and T6 step 3's "headline lines match OTS as before" quietly assume the HEAD-shaped fallback

**Touches:** T5 step 2 (L24 row), T6 step 2 (SE L12 / 8959 L18 / TI rows), the global gate-on-`Some` constraint.

The global constraint says leg-/leaf-gated assertions are inert until T11 — but T5 step 3 claims the miner's L24 divergence "**dissolves** — the L24 `sum_round` reproduction equals btctax's printed L24 from OTS's own components," which is only true *at T5* if the reproduction runs pre-bake with the **per-line totals** as components (`round_leaf(se_tax)`, `round_leaf(addl)`, `round_leaf(niit)` + reproduced L16 — exactly `golden_packet.rs:120-123`'s proven-green formula; the legs upgrade it at T11), and T6 step 3's "headline lines match OTS as before" likewise requires the TI row (whose reproduction needs the unbaked `deduction_taken`) and the SE-L12 row (unbaked legs) to keep their HEAD shapes until their operands are `Some`. A literal gate-on-`Some` executor instead lands on inert rows — still green at every boundary (so this does not gate), but it contradicts step 3's stated dispositions and silently drops live L24/L15/SE-L12 paper coverage for five task boundaries. **Fix:** one sentence in T5/T6: "pre-T11, a reproduction whose component leaves are `None` falls back to `round_leaf` of that component's baked per-line total (the HEAD shape, proven green on the anchors since no anchor flips); the leg form takes over when the leaves bake" — making "dissolves at T5" literally true. (This refines, not re-opens, my r2-I4 disposition: the compute-level rows remain the primary dip cure.)

### r3-M3 — the OTS driver's 8960 *trigger* gate omits short-term gains, so a corpus cell whose only investment income is a net STCG yields a false OTS `niit = 0`

**Touches:** T8 (the same block the r2-I3 fix edits); T11 step 3.

`ots_direct.py:325-328` gates the 8960 run on `investment = taxable_interest + ordinary_dividends + max(ltcg, 0)` — STCG never triggers it. A corpus cell with a net short-term gain, no interest/dividends/LTCG, and MAGI over the threshold (possible under t=2, not guaranteed) gets OTS `niit = 0` while btctax (`form_8960` includes the full §1211 net) and taxcalc (`p22250` feeds `niit`) both compute a real tax → a T11 red correctly routed to cause (iii) but costing a bake cycle. Since T8 already edits the adjacent line, widen the gate now: `investment = interest + dividends + max(sch_d_net, 0)` (the same §1211-limited figure the L5a fix feeds). While there, T8 step 2's eyeball should include one capped-loss × NIIT-firing scenario to confirm OTS's `f8960` binary **accepts a negative L5a** (the r2-I3 fix feeds one; `_parse` reads negatives fine, the solver's behavior is the open question).

---

## Nit

- **r3-N1** (T4 step 1): the 2024 f1040 map keys the line-7 cell **`line7a`**, not `line7` (`f1040.map.toml:11-13` — "the struct field is named `line7a` for cross-year uniformity"). Step 3 already says to confirm the keys; make the snippet match and save the round-trip. (`line21` on schedule_d is exact as written.)
- **r3-N2** (T3 Interfaces): `stacking_ok`'s first parameter is named `paper` but T5 consumes it at the compute level — name it `figure`/`btctax_value` or note the dual use, so the T5 executor doesn't second-guess which chain to pass.

---

**GREEN — ready to execute.** 0 Critical / 0 Important. The r2 fold is sound end-to-end: all four Importants landed exactly as specified, the two-level rule is coherent against both the spec (§6.2(b) "compute-level … §7" / §7 "compute structs vs both oracles") and the code, the T5 green gate is numerically re-verified on all 12 anchors, and the three Minors are wording/one-line-driver refinements that no task boundary depends on (fold them without re-gating, or carry r3-M2/M3 as T5/T8-owned follow-ups).

## Strengths

The fold is precise where it mattered most: the two-level rule resolves the witness-regression without over-extending exact-vs-exact to the lines where it would be wrong, and the r2-I3 driver fix names the exact source quantity (`final.get("L7")`) that the 8960 build point can actually reach. Three rounds in, every cross-task name, field, and cite still reconciles — including the ones added in this fold.
