# §G-28/B3+B4 review r4 — THE CRITICAL FIX (Opus, 2026-08-03)

Scope: `07c135e` — r3's fix of the two Criticals r2's fold introduced. One question: *is that fix
correct, and did it introduce anything new?*

**VERDICT: 0 Critical / 3 Important / 1 Minor / 1 Nit. Both Criticals CLOSED**, verified by executing
the committed code on 20 vectors — engine delta equals independently form-derived truth **to the cent
on 19/20**, the exception being a pre-existing, overstating approximation.

★ The reviewer also answered the question that mattered most — *is the carryforward now consumed
twice?* — numerically on three vectors rather than by argument. It is not; it cancels algebraically
out of `ordinary_taxable_income`. That was the most likely new defect and it is absent.

**Persisted VERBATIM before folding.** Everything below the rule is the reviewer's own text.

---

**Nothing Critical. 3 Important, 1 Minor, 1 Nit.**

**SEVERITY: Important** — `FOLLOWUPS.md` §G-30/B4 direction paragraph
**CLAIM:** The "broker short-term **LOSS** … OVERSTATING (safe)" branch is false. A broker short-term loss can make the engine **understate** — the paragraph is wrong for a **third** time, on the same entry, in the same way, and this time on the branch it marks safe.
**EVIDENCE:** executed `derive_tax_profile` + `compute_tax_year` (committed 07c135e) from an out-of-tree probe. Single, TY2024, W-2 $200,000, broker **short-term loss $100,000**, crypto **short-term loss $50,000**:

| | engine | truth (independent, from Sch D + §1211(b) + §1(h) + §1411) |
|---|---|---|
| `total_federal_tax_attributable` | **−$720.00** | **$0.00** |
| control: same but broker **LONG**-term loss (which *is* modelled) | $0.00 | $0.00 |

Mechanism: §1211(b)'s $3,000 is one per-return allowance. With the broker ST loss invisible, `without` shows no capital loss and `with` shows the crypto loss consuming the whole $3,000 — so $3,000 of ordinary deduction is attributed to crypto that the filer already had.
**FAILURE:** that filer is told crypto saved $720 it did not save. `optimize` minimizes exactly this number, so it will prefer a lot method that manufactures a crypto capital loss with **no** real benefit. (The GAIN branch also flips: broker ST gain $100k + crypto ST loss $100k → engine −$720 vs truth −$36,526.25, **over**stating by $35,806.) The sign is not a property of the broker leg's sign at all — it is a property of the *interaction*, which is precisely what the paragraph keeps getting wrong.

---

**SEVERITY: Important** — `crates/btctax-core/src/tax/return_1040.rs:1069-1071`
**CLAIM:** The fix removed the code but left the r2 fold's justification comment sitting directly on top of the fixed line, where it now asserts the exact opposite of what the fix does — and instructs the next author to re-commit C-1.
**EVIDENCE:** `git log -L` puts that comment at **cbaca05** — it is the r2 fold's rationale for adding `+ b1099_st + b1099_lt`. 07c135e deleted that term and its own comment block says the opposite in terms; `FOLLOWUPS.md` adds *"Do NOT half-fix it."* Three statements, two contradicting the one that sits on the line.
**FAILURE:** an author reading it restores `+ _b1099_st`. Running that mutation reds only 2 tests, both of which the author would then "correct" to the new numbers — and the profile↔`without` inconsistency that produced *both* Criticals is back.

---

**SEVERITY: Important** — `crates/btctax-core/src/tax/return_1040.rs:1029-1031`
**CLAIM:** The fix's one load-bearing claim — *"the profile now calls `net_1222` with the SAME arguments the engine will use"* — is held by **no test** for 2 of the 6 arguments: the carryforward pair and the status-dependent §1211(b) limit.
**EVIDENCE:** mutation on a `git archive HEAD` copy: dropping both carryforwards and hardcoding the limit at `dec!(3000)` → **2614 passed, 4 failed**, byte-identical to the unmutated baseline (all 4 environmental). No test anywhere calls `derive_tax_profile` with a nonzero carryforward.
**FAILURE:** with the carryforward args dropped, a filer with a $50,000 short-term carryover gets `magi_excluding_crypto` $3,000 too high — this is the **pre-07c135e behaviour**, i.e. this commit silently *fixed* a live wrong AGI for every carryforward filer and pinned nothing. With the limit hardcoded, an MFS filer's AGI is $1,500 too low, and the MFS §1411 threshold is $125,000. Under B1 the answer to *"which test reds?"* is **none**.

---

**SEVERITY: Minor** — the KNOWN APPROXIMATION block
**CLAIM:** The comment names the divergence trigger as `TI < qd + cap_gain_distr`, but the region that actually still diverges is `AGI < deduction` — reachable with **zero** preferential income.
**EVIDENCE:** Single, W-2 $5,000, broker LT loss $200,000, crypto mining $20,000 → engine `$1,808.00`, independent truth `$740.00`. Same shape with only box-2a (predates B4 entirely). Direction **overstating** (safe) in every case measured.
**FAILURE:** none new.

---

**SEVERITY: Nit** — the `$6,704` / `$19,360.25` figures inherited from the r3 reviewer are $114 off: the source used NII $50,000 and omitted the §1211(b) −$3,000 (Form 8960 line 5a). NIIT is $1,786, not $1,900; the true delta is **$19,246.25**, so the fold understated by **$6,590.25**.

---

## Answers to the six deciding questions

1. **Contract — YES, both halves, exactly.** Algebraically `oti = base − ded − qd` (the whole capital term, carryforward included, cancels), so `bottom_without` = the true non-crypto ordinary bottom and `magi_excluding_crypto` = true AGI. Executed on 20 vectors: **engine delta == independent form-derived truth to the cent on 19/20**. Composite MFS case (box-2a $5,000 + broker LT −$20,000 + cf {10,000/4,000}): engine **$13,518.00**, truth **$13,518.00**.
2. **No double-count, no third consumer.** `noncrypto_cap_agi` is read exactly twice and cancels between them; the three profile fields are read only by `compute_tax_year`.
3. **Nothing else used `taxable_income`.**
4. **NOT consumed twice.** Applied once to build AGI (correct — Sch D line 21 → 1040 L7) and once inside each engine netting, and it vanishes algebraically from `ordinary_taxable_income`. Confirmed numerically on three carryforward vectors, all exact. **This was the most likely new defect and it is not present** — but see Important #3: it is also untested.
5. **No.** `.max(0)` still applies once, at the end.
6. **All three tests' constants recompute correctly by hand**, and none is vacuous — five mutations each red at least one. `the_delta_baseline_…`'s `magi == 2,091,000` deliberately pins a value $20,000 below true AGI: that is the §G-30 gap, disclosed in the test's own comment, and pinning it is correct coupling.

## Are the two Criticals actually fixed?

**C-1: CLOSED.** `magi_excluding_crypto` on the r3 vector is now **$197,000** (was $100,000; truth $197,000); on the failure vector `compute_tax_year` returns **$19,400.00** against a truth of **$19,246.25** — the $153.75 residue is §G-30's already-filed ST blindness, not C-1.

**C-2: CLOSED.** W-2 $50,000 + $200,000 broker LT loss → `ordinary_taxable_income` $35,400 (was $200,000), `bottom_without` $32,400 = truth. W-2 $5,000 → `oti` $0. The vacuity the commit self-reported is genuinely repaired.

No repo file was modified; mutations ran on a `git archive HEAD` copy.
