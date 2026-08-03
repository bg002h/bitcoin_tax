# §G-28/B3+B4 review r3 — THE FOLD (Opus, 2026-08-03)

Scope: `git show cbaca05` — the fold of r1+r2 only, not the features. One question: *did the fold
introduce a new defect, or fail to close a finding it claims to close?*

**VERDICT: 2 CRITICAL, 1 Important, 3 Minor, 1 Nit — the fold carried two Criticals it did not have
before.** This is §G-28's own recorded pattern, and the sharpest instance of it yet: BOTH Criticals are
in `derive_tax_profile`, the function the fold was editing to close a finding, and both were proved by
EXECUTING the committed code from an out-of-tree probe rather than by reading it.

**Persisted VERBATIM before folding.** Everything below the rule is the reviewer's own text.

---

**SEVERITY: Critical** — `crates/btctax-core/src/tax/return_1040.rs:1035-1036`
**CLAIM:** The fold adds the **raw signed** 1099-B gains to `income_total`, bypassing both §1222 netting and the §1211(b) $3,000 limit — so a broker capital **loss** is deducted from AGI/MAGI/TI at its full size. Pre-fold this filer was safe (overstated by ≤ $3,000); post-fold the number is wrong by the whole loss, in **both** directions depending on character.

```rust
-    let income_total = wages + taxable_int + ord_div + cap_gain_distr + sch1_income;
+    let income_total =
+        wages + taxable_int + ord_div + cap_gain_distr + b1099_st + b1099_lt + sch1_income;
```
`form_1099b_gains` returns the *signed* `proceeds − basis` per character. Nothing between it and `income_total` applies §1211(b); the only §1211 limit in the delta path is inside `net_1222`, which `derive_tax_profile` never calls.

Measured by calling the committed `derive_tax_profile` directly (Single, TY2024):

| vector | `magi_excluding_crypto` | `ordinary_taxable_income` | truth |
|---|---|---|---|
| W-2 200k, no 1099-B | 200,000 | 185,400 | — |
| W-2 200k, **$100k broker ST loss** | **100,000** | **85,400** | 197,000 / 182,400 |
| W-2 200k, **$100k broker LT loss** | **100,000** | 185,400 | 197,000 / 182,400 |

**FAILURE:** Single, W-2 $260,000, a $100,000 broker short-term loss, $50,000 of crypto lending interest. `compute_tax_year` returns `total_federal_tax_attributable = $12,656.00` (NIIT $380). Correct is **$19,360.25** (NIIT $1,900). **Understates crypto-attributable tax by $6,704** — and `optimize` selects a lot method by minimizing exactly this number. Pre-fold the same filer got $19,400.

---

**SEVERITY: Critical** — `crates/btctax-core/src/tax/return_1040.rs:1077-1078`
**CLAIM:** The new `− b1099_lt` add-back is applied to `taxable_income`, which is **already floored at zero**. When a broker long-term loss drives AGI below the deduction, the clamp discards the negative and the add-back then **manufactures ordinary income out of nothing**.

```rust
    let taxable_income = (agi - deduction).max(Usd::ZERO);      // pre-existing clamp
-   let ordinary_taxable_income = (taxable_income - qual_div - cap_gain_distr).max(Usd::ZERO);
+   let ordinary_taxable_income =
+       (taxable_income - qual_div - cap_gain_distr - b1099_lt).max(Usd::ZERO);
```
Measured on the committed code (Single, W-2 $50,000, `long_term_basis 200,000`):
```
W2 50k, no 1099-B             agi=     50000  ord_ti=     35400
W2 50k, LT broker loss 200k   agi=   -150000  ord_ti=    200000
W2 50k, ST broker loss 200k   agi=   -150000  ord_ti=         0
```
**FAILURE:** True non-crypto ordinary base = $32,400. The engine's `bottom_without` = **$197,000** — $164,600 too high. Every crypto ordinary dollar priced at 32-35% instead of 12%, and the §1(h) stack sits above `max_fifteen`, reporting 20% where it is 0%. **Overstates** by roughly 3×. The mirror ST row is the opposite: `ord_ti = 0` against a true $32,400 — **understates**. Both new; pre-fold both read $35,400.

---

**SEVERITY: Important** — `crates/btctax-forms/src/schedule_d_full.rs:117-124`
**CLAIM:** `(None, None)` is correct for the blank case, but it makes the `needs_1099b == true` branch **unreachable in the entire test suite**, so the six 1a/8a cells' column and row geometry is now verified by nothing.
**EVIDENCE:** the only `ScheduleDLines` fixture in `btctax-forms` hard-sets all six to `Usd::ZERO`; `b_1099` appears in no crate below `btctax-core`, so no packet-level test reaches the emitter with broker totals. Pre-fold, `push_money` pushed a `FlatPlacement` unconditionally even for a zero value, so `verify_flat` checked page membership, the x-clusters and the y-descent for all six on every Schedule D fill. Post-fold no placement is ever produced. `schedule_d_2024_field_names()` omits `line1a`/`line8a`, so the map-vs-PDF test does not cover them either.
**FAILURE:** swap `proceeds_d` and `cost_e` in the map. The census union is unchanged, no placement is emitted, **zero tests red** — and the filer prints $1,050,000 of proceeds in the cost column of a return signed under §6065. Under B1, the answer to "which test reds?" for these six cells is now *none*.

---

**SEVERITY: Minor** — `FOLLOWUPS.md` §G-30/B4 paragraph
**CLAIM:** The direction claim added by this fold is **false in both branches**, and it is false *because of* the edit made in the same commit — the second time this entry has stated a direction from a mechanism rather than from the code.
**EVIDENCE:** it says a broker short-term gain/loss "cannot enter the delta engine" and is "invisible". After `:1035` neither is invisible: both reach `income_total`, hence `ordinary_taxable_income` and `magi_excluding_crypto`. Only the *netting* is missing. Measured: a broker ST **gain** is fully in the ordinary base; a broker ST **loss** is over-deducted by `loss − 3,000` — **understating**, the exact opposite of what the paragraph asserts.
**FAILURE:** the next author adds a short-term `other` argument in the frozen-exception commit, which does not touch `income_total`, so the Critical survives the fix its own follow-up scheduled.

---

**SEVERITY: Minor** — `return_1040.rs` `a_return_with_no_1099b_leaves_lines_1a_and_8a_blank`
**CLAIM:** vacuous with respect to its name: named for the blank-vs-zero distinction, it asserts a **zero**.
**EVIDENCE:** the body asserts `st_1099b_proceeds_1ad == Usd::ZERO` (×4) over a fold across an EMPTY vector — an arithmetic identity. Restoring the whole r2-I2 defect leaves it green.
**FAILURE:** none live; the emitter test holds it. The cost is a false witness — a `0` and a blank are exactly what the name says it distinguishes and exactly what it cannot, which is §G-24's own thesis.

---

**SEVERITY: Minor** — `return_1040.rs` `the_delta_baseline_sees_non_crypto_capital_gain_and_receipts`
**CLAIM:** the two inequality assertions hide something: neither can red on a **double-count**, and the load-bearing "short-term stays IN the ordinary slice" claim is not pinned.
**EVIDENCE:** `magi >= dec!(2105000)` passes at exact equality — tight against an omission, blind to an addend counted twice (2,125,000 also passes). `ordinary_taxable_income` is 90,400 against an assertion of `> 0 && < 2,000,000`; the mutation that breaks the comment's own claim gives 70,400 — still green. `cap_gain_distr` is zero in the fixture, so `other_net_capital_gain == 2,000,000` cannot distinguish `cap_gain_distr + b1099_lt` from `b1099_lt` alone.
**FAILURE:** none live. Three equalities would close it.

---

**SEVERITY: Nit** — `full_return_forms.rs:1110-1113` — the new test was inserted between an existing doc comment and its test, so the paragraph now documents the wrong function and `schedule_d_full_fills_the_lines_the_crypto_slice_omits` is left undocumented.

---

## Status of the four Importants the fold claims to close

1. **`derive_tax_profile` blind to `b_1099`: PARTIALLY CLOSED.** Closed for gains — but the fix introduces two Criticals on the **loss** side that the pre-fold code did not have.
2. **`must_file()` omits 1a/8a: CLOSED.** Deleting the six reds `offsetting_1099b_totals_still_require_a_schedule_d`; no false positive is reachable since `line1a_h ≡ line1a_d − line1a_e`.
3. **1a/8a print a sworn `0`: CLOSED at the emitter, with a coverage regression** (the Important above). The core-side test named for the property is vacuous.
4. **§G-30's direction: PARTIALLY CLOSED.** The B3 half is honestly recorded and genuinely fixed; the newly-written B4 paragraph repeats the identical error on a different mechanism.

**Also closed, non-blocking:** the `se.rs` clamp hazard is now named at the call site with its $10,400 cost, and the GROSS-receipts justification is **factually true of current source**; the empty-ledger witness gap is closed and reds on the named mutation; the two-authorities Nit is recorded.

Evidence for the two Criticals came from executing the committed `derive_tax_profile` and
`compute_tax_year` from an out-of-tree probe crate; no repo file was modified.
