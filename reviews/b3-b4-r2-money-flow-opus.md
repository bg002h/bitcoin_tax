# §G-28/B3+B4 review r2 — the MONEY-FLOW COMPLETENESS lens (Opus, 2026-08-03)

Scope: the same two commits, run in parallel with r1 and given the same settled facts. One question:
*does each new money source reach EVERY place on the return it should — and NOWHERE it should not?*
A new money source is dangerous exactly where it is FORGOTTEN.

**Persisted VERBATIM before folding.** Everything below the rule is the reviewer's own text.

★ Both lenses independently found the same `derive_tax_profile` miss (r1's Important, r2's I-3). Two
independent contexts converging on one defect is the strongest signal this process produces.

---

Traced both chains against current source. Three Important, one Minor. No Critical.

**SEVERITY: Important**
**FILE:** `crates/btctax-core/src/tax/printed.rs:846-861` (`ScheduleDLines::must_file`)
**CLAIM:** B4 added `line1a_*`/`line8a_*` to `ScheduleDLines` but not to `must_file()`, so a return whose only capital activity is 1099-B totals can file with **no Schedule D and no Form 8949 attached at all**.
**TRACE:** the attach gate `btctax-forms/src/packet.rs:147 if sch_d.must_file()` reads a list of ten fields containing `line3_*`, `line6`, `line10_*`, `line13`, `line14`, `line16` and **neither `line1a_*`, `line8a_*`, `line7` nor `line15`**. `line16` is the only term that can carry them, and it is `line7 + line15` — zero exactly when the two characters offset.
**FAILURE:** Single, W-2 $200,000, no crypto, one 1099-B with ST 1,050,000/1,000,000 and LT 400,000/450,000. 1a(h) = +50,000 → line 7 = +50,000; 8a(h) = −50,000 → line 15 = −50,000; line 16 = 0. `must_file()` = false ⇒ no `schedule_d.pdf`. **$1,450,000 of proceeds reported to the IRS on Forms 1099-B appear nowhere in the filed packet.** Same for a `proceeds == basis` wash. No dollar figure is wrong — the defect is a **required schedule omitted from a filed return**, invisible to every value-checking test. Per B1, `must_file()` has never been watched red on a 1099-B-only case.

---

**SEVERITY: Important**
**FILE:** `crates/btctax-forms/src/schedule_d_full.rs:113-185`
**CLAIM:** Lines 1a and 8a are emitted with `push_money`, never `push_money_opt`, so **every** TY2024 return — including pure-crypto ones with no 1099-B — now prints a sworn `0` in six cells that were blank before this commit.
**TRACE:** `needs_1099b` is used **only** to decide whether the map cell is *required*; when false the code still takes `map.line1a.as_ref()`, which is `Some` for TY2024 → the cell is pushed → `push_money` writes unconditionally. Its sibling's doc states: *"A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065 … Prefer this over `push_money` for any line whose instruction states a condition."*
**FAILURE:** Any existing filer with BTC disposals and no broker account. Their Schedule D now prints `0` on line 1a — the one line whose own text is *"However, if you choose to report all these transactions on Form 8949, **leave this line blank**"* — asserting they had Form 1099-B transactions with basis reported to the IRS totaling zero. This is the §G-24 / lines-18-and-19 class the same file fixed 100 lines below, reintroduced. The fixture sets all six to `Usd::ZERO`, so every existing KAT exercises the zero-write path and passes.
**Counter-argument (stated for the caller):** lines 3/6/10/13/14 also print `0` unconditionally, matching the file's convention. What makes 1a/8a different is (a) their instruction text is an explicit conditional entry with a *"leave this line blank"* clause, which is precisely the `push_money_opt` trigger, and (b) the disqualifying predicate is already computed two lines above.

---

**SEVERITY: Important**
**FILE:** `crates/btctax-core/src/tax/return_1040.rs:1052-1062` (`derive_tax_profile`)
**CLAIM:** B4's 1099-B gains reach **nothing** on the crypto-delta path — and unlike §G-30, the long-term half needs no frozen-file change, because `TaxProfile::other_net_capital_gain` already exists for exactly this quantity.
**FAILURE:** the filing-trial filer: $2,000,000 of long-term stock gain plus crypto LTCG. The reported §1(h) marginal rate comes out 0% or 15% where it is really 20%, and `magi_excluding_crypto` misses $2M so the §1411 threshold test flips. Direction: **UNDERSTATES** crypto-attributable tax and the headline all-in LTCG reserve rate — the unsafe direction. Secondary: the M4 `carryforward_consistency` advisory compares the prior year's delta-engine `carryforward_out` (blind to b_1099) against this year's `capital_loss_carryforward_in`, so a filer who correctly enters a real broker-loss carryforward is told it *"does not match"*.

---

**SEVERITY: Minor**
**FILE:** `FOLLOWUPS.md:1285-1300` (§G-30)
**CLAIM:** §G-30 states the delta engine's B3 blindness in **one** direction (overstating SE tax); the same omission also runs the other way and that is not recorded.
**TRACE:** `derive_tax_profile` omits the non-ledger Schedule C net from `income_total` and therefore from `ordinary_taxable_income` and `magi_excluding_crypto`, not only from `schedule_c_expenses`.
**FAILURE:** $85,000 of consulting invisible ⇒ the crypto ordinary slice is priced from a bracket bottom $85,000 too low and the §1411 MAGI test can read under-threshold when the filer is over. Direction: **UNDERSTATES** — opposite to the "safe direction" §G-30 asserts.

**No Critical findings.** No further Minor or Nit.

## B3 — every consumer REACHED or CORRECTLY ABSENT

Schedule C 1/3/5/7/28/29/31, Schedule 1 line 3, 1040 line 8, Schedule SE net earnings (**exact**),
§1401(a) SS cap, §1401(b) Medicare, Form 8959 Part II, §164(f) → AGI, §199A `business_qbi`, Form 8995 /
8995-A I–IV, NIIT via AGI (NII **correctly absent** — §1411(c)(6)), §1211 limit (correctly
status-only), itemized-vs-standard, AMT, dependent standard deduction, §221 phase-out — all REACHED.
`crypto_ord`, `business_se_gross`, the crypto-slice export — all CORRECTLY ABSENT. Carryforward OUT:
N/A and correct (B3 moves no capital-loss quantity). Crypto-attributable delta: correctly absent per
§G-30, but see the Minor on its stated direction.

## B4 — three MISSES

Schedule D 1a/8a/7/15/16, Part III routing and §1211(b) line 21, 1040 line 7, §1222 within-character
netting, the QDCGT worksheet, `net_ltcg`, `qbi_net_capital_gain`, NIIT, AMT — all REACHED. Form 8949
rows and `schedule_d(state, year)` — CORRECTLY ABSENT (the whole point of line 1a).

**MISSED:** the Schedule D attachment decision (I-1); blank-vs-zero on 1a/8a (I-2); the
crypto-attributable delta (I-3).

**Carryforward OUT — correct by design, with a caveat.** `CapNet::st_carry`/`lt_carry` are computed
*with* B4 included, but `apply_carryover_writeback` deliberately writes none (the §1212(b) Carryover
Worksheet is unmodelled). The mitigation holds: `capital_loss_carryforward_in_provenance` stays `User`,
so next year's `BenefitCarryoversNotStated` advisory fires. Direction if ignored: next year's tax
**overstated**. B4 does not create this gap — a crypto loss already reached it — but it widens it
substantially, and it interacts with I-3's false "does not match" advisory.

**Verified clean, no finding:** the B3→Schedule SE equivalence is exact; the B4 refusal gate is
correctly framed as the YES-condition and is amount-gated; negative-amount screening covers all five
new money leaves; the loss screen nets both halves; both fields are censused with written reasons.
