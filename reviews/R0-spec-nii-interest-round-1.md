# R0 architect review — SPEC_nii_interest_slice.md — round 1

**Artifact:** `design/SPEC_nii_interest_slice.md`
**Reviewer role:** independent architect (author ≠ reviewer)
**Gate:** mandatory pre-implementation R0. Critical bar = wrong inclusion rule, wrong scenario
attribution (the delta), MAGI double-count, or a non-reproducing golden.
**Verdict:** **0 Critical / 0 Important.** GREEN — cleared for implementation. 2 Minor + 2 Nit (non-blocking).

---

## 1. Recon-citation verification (against CURRENT checkout)

Branch: `feat/gift-chunk3b` (HEAD ahead of `main`). `git diff main -- crates/btctax-core/src/tax/`
is **EMPTY** — the entire `tax/` tree (incl. `compute.rs`) is byte-identical to `main`. The spec's
core assumption holds.

| Spec citation | Verified | Notes |
|---|---|---|
| `compute.rs` crypto_ord 297-302 | ✅ exact | kind-agnostic Σ `usd_fmv` over in-year income; Interest ⊆ crypto_ord |
| `compute.rs` bottom_with/without 326-329 | ✅ exact | crypto_ord in `bottom_with` ONLY (327) |
| `compute.rs` nii_with/without 344-346 | ✅ exact | no income component in either scenario today |
| `compute.rs` crypto_agi/magi 348-352 | ✅ exact | `crypto_agi = … + crypto_ord` (350); `magi_with = magi_without + crypto_agi` |
| `compute.rs` niit closure 353-364 | ✅ exact | `3.8% × max(0, min(nii, magi−thr))`; NIIT_RATE = `dec!(0.038)` (tables.rs:133); `round_cents` conventions.rs:22 |
| `compute.rs` delta 389 | ✅ exact | `niit: niit_with − niit_without` ("crypto-attributable … DELTA") |
| `compute.rs` module-doc 217-219 / inline 341-343 | ✅ exact | both carry the "cannot yet isolate" residual language |
| `event.rs` IncomeKind | ✅ | enum at 29-35; `Interest` variant present (line 32) |
| `se.rs:58` interest-excluded SE filter | ✅ exact | `i.business && i.kind != IncomeKind::Interest && …` |
| `render.rs` disclosure | ⚠️ position | text found at **1026-1027 on branch** (spec cites ~1026-1027) — but **1006-1007 on main@114d6e0** |
| `tax_report.rs` KAT 212-243 | ⚠️ position | fn at **212 on branch**, **208 on main@114d6e0** |
| goldens in `tax_compute.rs` | ✅ | −684.00 (435), −57.00 (507), 760.00 (347), 2280.00 (385), double-count guard niit 0.00 (233) |

Import note in spec (D1: "Needs `use crate::event::IncomeKind;`") is **accurate** — `compute.rs:13`
imports only `LedgerEvent`, not `IncomeKind` (unlike `se.rs:13` which imports it). Add the import.

`IncomeRecord.usd_fmv` is `Usd` (not `Option`); `.map(|i| i.usd_fmv).sum()` mirrors crypto_ord — correct.

**Drift (Minor, M1):** the gift branch modifies `render.rs` (+205) and `tax_report.rs` (+172). The
spec's line citations for those two CLI files are **branch / post-merge-main positions**, not
`main@114d6e0` positions (disclosure moved 1006→1026; KAT 208→212). The header label "Source
baseline: main @ 114d6e0" is therefore imprecise for the CLI files, though the spec discloses it
"targets the post-merge main." The **anchor text is byte-identical** on both refs (the sentence to
replace, the KAT fn name), so the substance is unambiguous. No correctness risk provided
implementation lands on/after the gift merge.

---

## 2. Independent web verification (did NOT trust the spec)

**§1411(c)(1)(A)(i) — interest IS gross NII.** Confirmed verbatim (Cornell LII 26 USC §1411, IRS
NIIT Q&A): NII includes "gross income from interest, dividends, annuities, royalties, and rents,
**other than such income which is derived in the ordinary course of a trade or business not described
in paragraph (2)**." Interest is squarely inside category (A)(i).

**§1411(c)(2) — the exception's scope.** "A trade or business is described in this paragraph if such
trade or business is—(A) a passive activity (§469) …, or (B) a trade or business of trading in
financial instruments or commodities." So the (A)(i) carve-out removes interest from NII **only** when
it is derived in an *active, non-passive, non-trading* trade or business. Retail crypto lending
(BlockFi/Celsius/Aave-style) is **portfolio interest — not derived in any trade or business** — so the
carve-out does not apply and the interest stays in NII. The spec's phrasing ("does not apply to
non-business portfolio lending") is correct.

**§1411(c)(6) — SE exclusion.** Confirmed verbatim: "Net investment income shall not include any item
taken into account in determining self-employment income … on which a tax is imposed by section
1401(b)." This is exactly why business mining/staking (SE income) is excluded from NII — the B-M1
determination re-confirmed. Hobby mining/staking = "other income" outside the (c)(1)(A) categories →
also not NII. **Exclusions stay correct.**

**Crypto-lending interest treatment.** Confirmed (multiple 2025/2026 crypto-tax guides): lending
interest is ordinary income at FMV on receipt **and** high-MAGI earners "may additionally owe the 3.8%
NIIT on this interest income." This corroborates both (a) crypto-lending interest = NII for a retail
lender, and (b) that it is subject to *both* ordinary income tax and NIIT (two distinct taxes — see §4,
not a double-count).

**Business-flag irrelevance (rated).** Sound for the target use case. Interest is NII whether or not
`business` is set, because (i) portfolio interest is NII regardless, and (ii) interest is already
excluded from SE (`se.rs:58`), so §1411(c)(6) can never pull it out of NII. **Edge (Minor, M2):** if a
taxpayer genuinely ran an *active, non-passive, non-trading money-lending trade or business*, the
(A)(i) carve-out would **exclude** that interest from NII — the always-include rule would then
*overstate* NIIT. This is atypical for a retail crypto lender, the direction is conservative (never
understates), and the spec explicitly scopes the §1411(c)(2) exception out as disclosed-inapplicable.
Not blocking. Suggest a one-line code comment pointing at the disclosed nuance.

---

## 3. WITH-only attribution (highest priority) — CORRECT

**Convention verified from source.** `crypto_ord` appears in `bottom_with` (327) only — never
`bottom_without` (329) — and in `magi_with` via `crypto_agi` (350) only — never `magi_without` (351 =
`profile.magi_excluding_crypto`). The WITHOUT scenario is literally "no crypto at all." `r.niit =
niit_with − niit_without` (389) is the crypto-attributable **delta**. Adding `interest_nii` to
`nii_with` ONLY is exactly consistent with this convention.

**Both-scenario insertion would hide the liability.** In the high-MAGI regime (both scenarios over
threshold, cap not binding differently), adding `interest_nii` to both `nii_with` and `nii_without`
cancels in the subtraction → `r.niit` loses the interest's NIIT. Worked counter-example: `qd`=0, no
gains, `magi_excl`=300k, interest=10k → both-insertion gives niit_with = niit_without = 3.8%×10k = 380
→ delta **0** (liability erased). WITH-only gives 380. The spec's rationale is sound.

**No misattribution edges.**
- *min(nii, over) cap:* in the WITH scenario the interest raises **both** sides — `nii_with` via
  `interest_nii`, and `over_with` via `magi_with ⊇ crypto_agi ⊇ crypto_ord ⊇ interest`. The pairing is
  internally consistent (matches Form 8960's `3.8% × min(NII, MAGI−thr)`); no case where NII reflects
  the interest but MAGI does not.
- *$0 floor:* `usd_fmv ≥ 0`, so `interest_nii ≥ 0` and can only raise `nii_with` — never pushes it
  negative, never interacts adversely with the floor.
- *§1211 loss interaction:* `nii_with = qd + gains − loss_deduction + interest_nii` correctly nets the
  §1211-allowed capital loss against **total** NII (incl. interest), matching Form 8960 line-1
  interest + line-5a net loss. E.g. loss_deduction 3,000 + interest 10,000 → NII 7,000 (correct).

WITH-only is the correct attribution for the crypto-attributable delta.

---

## 4. No MAGI double-count — CONFIRMED

`crypto_agi` (348-350) already adds `crypto_ord`, and `crypto_ord` (297-302) is kind-agnostic so it
already includes Interest → `magi_with` (352) is already correct. D1 touches the **NII base only**
(`interest_nii` into `nii_with`); it does **not** re-add interest to MAGI. No double-count, no
missing-MAGI path.

Distinct concern ruled out: interest appearing in `bottom_with` (ordinary stack, via crypto_ord)
**and** `nii_with` (via interest_nii) is **not** a double-count — those are two separate taxes
(Chapter 1 ordinary income tax and Chapter 2A §1411 NIIT), both of which correctly apply to interest.
Web-corroborated (§2).

---

## 5. Golden re-derivations (by hand, from the verified rules)

**(a) Headline — Single, thr $200k, ord $150k, magi_excl $195k, qd 0, no disposals, Interest $20k.**
- crypto_ord = 20,000; **interest_nii = 20,000**; with/without gains all 0, loss_deduction 0.
- nii_with = 0+0+0−0+**20,000** = **20,000**; nii_without = 0.
- crypto_agi = (0)−(0)+20,000 = 20,000; magi_without = 195,000; **magi_with = 215,000**.
- niit_with: over = 215,000−200,000 = **15,000**; min(20,000, 15,000) = **15,000** (nii > over — the
  cap binds on `over`); base 15,000 → 3.8%×15,000 = **$570.00**.
- niit_without: 195,000 < 200,000 → over 0 → **$0.00**.
- **r.niit = 570.00 − 0 = $570.00.** ✅ matches.
- Identity: bottom_with = 150k+20k = 170k; ord_delta = tax(170k)−tax(150k) = (5,000+26,400)−(5,000+22,000)
  = **4,400.00**; ltcg_tax 0; total = 4,400+0+570 = **4,970.00** = ord_delta + 0 + 570.00. ✅

**(b) Mixed — Mining $30k + Interest $10k, magi_excl $200k, no disposals.**
- crypto_ord = 40,000; **interest_nii = 10,000** (Interest only — Mining excluded).
- nii_with = 0+0+0−0+**10,000** = **10,000**; nii_without = 0.
- crypto_agi = 0 + 40,000 = 40,000; **magi_with = 240,000**.
- niit_with: over = 240,000−200,000 = **40,000**; min(10,000, 40,000) = **10,000** (nii < over — the
  cap binds on `nii`); base 10,000 → 3.8%×10,000 = **$380.00**.
- niit_without: magi_without = 200,000; closure uses `if magi > thr` → 200,000 **not** > 200,000 → over
  0 → **$0.00**.
- **r.niit = 380.00 − 0 = $380.00.** ✅ matches.
- **Boundary lock:** if Mining wrongly entered NII, nii_with = 40,000 → min(40,000, 40,000) = 40,000 →
  3.8%×40,000 = **$1,520.00** → golden fails. Correct exclusion lock.

Both reproduce **exactly**; the two goldens jointly exercise both branches of `min(nii, over)`
(over-bound in (a), nii-bound in (b)). Both **fail pre-change** (today `nii_with` has no interest term
→ r.niit = 0 ≠ 570/380) → genuine TDD red→green.

---

## 6. Regression net — all UNMOVED (fixtures verified, none has Interest)

| Golden | Fixture income | Interest? | Value |
|---|---|---|---|
| `double_count_guard_…` | Mining $10k (event.rs `IncomeKind::Mining`) | none | niit 0.00 (magi 70k<200k) |
| `niit_threshold_crossing` | `income = vec![]` | none | 760.00 |
| `full_worked_example_…` | `income_rec()` → **`IncomeKind::Mining`** (helper line 167) | none | 2280.00 |
| `niit_loss_year_reduces_nii_by_1211_allowed_loss` | `income = vec![]` | none | −684.00 |
| `niit_loss_year_mfs_1500_limit` | `income = vec![]` | none | −57.00 |
| (bonus) `niit_base_floored_at_zero_when_nii_negative` | `income = vec![]` | none | 0.00 |

`income_rec` (tax_compute.rs:161-170) hard-codes `kind: IncomeKind::Mining` — so `full_worked_example`
(the only regression golden with income) carries **no Interest** and its `interest_nii` = 0 → NII
unchanged. All five (plus the floor golden) are byte-identical post-change. ✅

---

## 7. Disclosure (D2) — covered and accurate

Three "cannot yet isolate" sites confirmed present and all in the plan: `render.rs` 1026-1027,
`compute.rs` module-doc 217-219, inline comment 341-343. The pinned KAT
`report_tax_year_footer_discloses_1211_loss_and_lending_interest_caveat` (tax_report.rs:212-243) is
covered.

The new text ("crypto-lending interest income (§1411(c)(1)(A)(i)) is INCLUDED in NII;
mining/staking/airdrops/rewards remain excluded — SE income per §1411(c)(6) or non-NII other income")
is **legally accurate** per §2. The B-M1 negative assertions to retain — `!contains("can only ever
understate")` (224), `!contains("MAY UNDERSTATE")` (228), `!contains("does not reduce NII")` (232) —
and the §1211 positive `contains("reduces NII by the §1211(b)-allowed net capital loss")` (237, from
render.rs 1023-1025 which D2 does **not** touch) all remain valid. ✅

**Nit (N1):** the existing assertion `rendered.contains("crypto-lending interest")` (241) stays
literally true under the new footer ("crypto-lending interest income … is INCLUDED in NII"), so a
naive add-only edit would leave it passing **for the wrong reason** (its comment says "flags the
residual … understatement"). The spec already says "rename + re-point"; ensure the re-pointed positive
assertion targets the *new* semantics (e.g. assert `contains("INCLUDED in NII")` **and**
`!contains("cannot yet isolate")`) so the KAT genuinely distinguishes old→new.

---

## 8. Scope / right-sizing / TDD genuineness

- **Right-sized:** one implementation task (interest_nii + disclosure + goldens across 4 files) + a
  whole-diff review task. Ceremony scaled down, not removed (STANDARD_WORKFLOW §8). Appropriate for a
  ~4-line engine change.
- **TDD genuine:** both new goldens are hand-derived with exact expected values and both fail on the
  current code (r.niit would be 0). Red→green is real, not retrofit.
- **No struct/API change:** `TaxResult` untouched; the `total == ord_delta + ltcg_tax + niit` identity
  preserved (niit remains the delta). SemVer MINOR (pre-1.0 behavior change) is correct.

---

## 9. Findings ledger

**Critical (0):** none.

**Important (0):** none.

**Minor:**
- **M1 — citation baseline mislabel (CLI files).** `render.rs`/`tax_report.rs` line citations are
  branch/post-merge positions, not `main@114d6e0` (disclosure 1006→1026, KAT 208→212). *Fix:* label
  the two CLI citations as post-merge/branch positions **or** state that implementation is gated on the
  gift-chunk3b merge. Anchor text is identical on both refs; no correctness impact. (`tax/` — the
  substantive change — is byte-identical to main and needs no such caveat.)
- **M2 — disclosed §1411(c)(2) T-or-B edge.** The kind-only, business-agnostic rule would *overstate*
  NII if interest were derived in an active, non-passive, non-trading money-lending trade/business
  (the (A)(i) carve-out). Atypical for retail; conservative direction; already scoped out and
  disclosed. *Fix (optional):* a one-line code comment beside `interest_nii` pointing at the disclosed
  nuance.

**Nit:**
- **N1 — KAT re-point must be semantic, not additive** (see §7): assert the new "INCLUDED in NII" text
  and `!contains("cannot yet isolate")`, since `contains("crypto-lending interest")` alone survives the
  change.
- **N2 — headline golden absolute total.** The spec pins `r.niit` (570.00) and the identity but not the
  absolute `total_federal_tax_attributable` (4,970.00). Optionally pin it too for a stronger lock
  (matches the other worked-example goldens' style).

---

## 10. Gate decision

**0 Critical / 0 Important → R0 GREEN.** The inclusion rule (Interest→NII), the WITH-only scenario
attribution, the no-double-count MAGI reasoning, and both hand-goldens are all verified correct and
reproduce exactly; the regression net is intact. The 2 Minor + 2 Nit are non-blocking polish. Cleared
to proceed to implementation (land on/after the gift-chunk3b merge per M1).
