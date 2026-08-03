# §G-28/B3+B4 review r1 — the FROZEN-BOUNDARY lens (Opus, 2026-08-03)

Scope: commits `5808ffe` (B3) and `05e0736` (B4). One question: *both features route AROUND frozen
engine files rather than editing them — are those routings exactly equivalent on every input, including
the edges?* Told not to object to the technique (CLAUDE.md permits a derived form with a written proof
plus a KAT), only to judge whether the proofs are TRUE.

**Persisted VERBATIM before folding.** Everything below the rule is the reviewer's own text.

---

## Findings

**SEVERITY: Important**
**FILE:LINE** `crates/btctax-core/src/tax/return_1040.rs:1057` (and `:991`, `:1055`)
**CLAIM:** B4's data reaches only the absolute path; `derive_tax_profile`'s `other_net_capital_gain` was **complete** before B4 and is now **incomplete**, so the crypto-delta engine prices crypto gain against a baseline missing the entire broker position — the exact analogue of the settled §G-30, but **unfiled and in the understating direction**, and (unlike §G-30) the LT half needs no frozen-file edit.

**EVIDENCE:** the frozen consumer, `compute.rs:319-368`:
```rust
let with = net_1222(crypto_st, crypto_lt, profile.other_net_capital_gain, cf.short, cf.long, limit);
let without = net_1222(Usd::ZERO, Usd::ZERO, profile.other_net_capital_gain, cf.short, cf.long, limit);
...
let pref_with = preferential_tax(&bp, bottom_with, qd + with.preferential_gain).tax;
let magi_without = profile.magi_excluding_crypto;
```
and the non-frozen producer, `return_1040.rs:991`/`1057`:
```rust
let income_total = wages + taxable_int + ord_div + cap_gain_distr + sch1_income;   // ri.b_1099 never read
...
other_net_capital_gain: cap_gain_distr,
```
Pre-B4, 1099-DIV box 2a was the *only* non-crypto capital-gain channel in `ReturnInputs`, and `cap_gain_distr` carried exactly it. B4 added a second LT channel (`b_lt`) and a first ST channel (`b_st`) and threaded neither. `net_1222`'s third argument is documented as LT-character, so `b_lt` belongs there verbatim; `derive_tax_profile` is in `return_1040.rs`, which `frozen_guard.rs` does **not** pin.

Worked, Single, TY2024, no wages: $2,000,000 1099-B long-term gain + $100,000 crypto long-term gain.
- Honest: TI ≈ $2,085,400; the crypto slice stacks between $2.0M and $2.1M → 20% = **$20,000**, plus §1411 at 3.8% = **$3,800**. Crypto-attributable ≈ **$23,800**.
- What the engine computes: `other_net_capital_gain = 0`, `magi_excluding_crypto = 0` → $47,025 in the 0% band and $52,975 at 15% = **$7,946.25**; `magi_with = 100,000 < 200,000` → NIIT delta **$0**.

**FAILURE:** `btctax report --tax-year`, the TUI tax tab, `optimize` and `what-if` report ≈ **$7,946** where the true marginal figure is ≈ **$23,800** — **understates by ≈ $15,850**. Same mechanism corrupts `TaxResult.carryforward_out`. And the optimizer *selects a lot method* by minimizing this number, so a wrong marginal rate can select the wrong method. **No filed line on the 1040/Schedule D is wrong** — `assemble_absolute` never reads `TaxProfile`. Direction: **understates** (the opposite of §G-30's stated safe direction). The B4 call-site comment claims only that "the securities gain is added to the same character it belongs to and nothing inside is touched" — true, but it names **no branch where the substitution breaks**, which is the half of CLAUDE.md's derived-form licence that is missing.

---

**SEVERITY: Minor**
**FILE:LINE** `crates/btctax-core/src/tax/se.rs:78-80`
**CLAIM:** the frozen function's own doc instructs a future author to do precisely the edit B3's proof names as fatal.
**EVIDENCE:** `/// - schedule_c_expenses: … **Must be ≥ 0** — the CLI validates; this function assumes the precondition holds.` The full-return caller now passes `−$80,000` on every filer whose receipts exceed expenses. `frozen_guard`'s doctrine explicitly *permits* a deliberate frozen-file change, so a hardening pass adding `schedule_c_expenses.max(Usd::ZERO)` is exactly what this doc licenses.
**FAILURE:** with that clamp, KAT case (a) becomes `net_se = 40,000` instead of `120,000` → **understates SE tax by ≈ $10,400**. It *is* pinned — `non_ledger_receipts_reach_schedule_se_exactly` reds — so this is a documentation contradiction inviting a rejected edit, not a live defect.

---

**SEVERITY: Minor**
**FILE:LINE** `crates/btctax-core/src/tax/return_1040.rs:3911`, `:4055`, `:4074`
**CLAIM:** every B4 KAT runs on an **empty ledger**, so the claim "the broker totals join **the crypto nets**" is never observed with a nonzero crypto net.
**EVIDENCE:** all three B4 tests use `LedgerState::default()` ⇒ `sd.st.gain == sd.lt.gain == 0`. Mutating `capital_net` to drop the crypto addend — `net_1222(b_st, b_lt, …)` — survives all three. The untested combination is a 1099-B loss netting against a crypto gain **of the same character**, i.e. the one arrangement B4's own sentence describes.
**FAILURE:** none currently. The gap is in the *witness*, not the arithmetic.

---

**SEVERITY: Nit**
**FILE:LINE** `crates/btctax-core/src/tax/return_1040.rs:1438,1441`
**CLAIM:** `st_1099b_gain_1ah` / `lt_1099b_gain_8ah` are exact-cent column-(h) figures read by **nothing outside tests** — `schedule_d_lines` recomputes `line1a_h = line1a_d − line1a_e` from the rounded cells. Two authorities for one box, one of them dead.

---

## Verdicts

**Is the B3 equivalence exact? — YES.** `schedule_c_expenses` occurs 7× in `se.rs`: 5 doc comments, the parameter, and **one** arithmetic use. No clamp, sign test, or short-circuit touches the argument — the `max(0)` is on `n`, after the subtraction. `crypto.business_se_gross` is literally `se_net_income(state, year)` — the same function, same `state`, same `year`, that `compute_se_tax` calls — so `net_se ≡ line 1 − line 28` identically, not approximately. Negative-argument, zero-ledger, cap/threshold, rounding and `None`/`SE_6017_FLOOR` cases all verified.

**Is the B4 equivalence exact? — YES, within `capital_net`.** `net_1222` adds arguments 2 and 3 together with no distinction anywhere, so adding `b_lt` to arg 2 is bit-identical to arg 3, and both additions happen strictly **before** the cross-character match — which is Schedule D's own order. Carryforward character is preserved. Nothing assumes a non-negative gain.

The one thing B4's routing does **not** do is reach the second consumer — see the Important above.
