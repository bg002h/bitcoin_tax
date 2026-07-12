# Fable F4 (round 2, VERIFY-ONLY) — Adversarial confirmation of deep/02 (derivation + absolute 8960/8959)

**Agent:** Fable F4 second-pass, adversarial verify-only. **Date:** 2026-07-11.
**Target:** `design/full-return/recon/deep/02-derivation-and-absolute-niit-medicare.md` (the locked
round-2 result) + `04-input-data-model.md` §5.
**Verdict: CONFIRMED — 0 discrepancies.** Every locked claim survived attack; every worked figure
reproduced independently to the cent. Four non-blocking hardening notes (§4).

**Method (independent, not a re-read of deep/02's citations):** re-fetched the PRIMARY 2024-revision
IRS documents fresh this pass (`irs.gov/pub/irs-prior/`: `i8960--2024.pdf`, `f8960--2024.pdf`,
`i8959--2024.pdf`, `f8959--2024.pdf`, `f1040sse--2024.pdf`, `i1040sse--2024.pdf`,
`f1040s2--2024.pdf`; `pdftotext`-extracted, quoted verbatim below — deep/02 transcribed the 2025
forms, so this pass independently proves its "identical line map 2024" claims). Re-read the frozen
code (`crates/btctax-core/src/tax/compute.rs`, `se.rs`, `tables.rs`;
`crates/btctax-adapters/src/tax_tables.rs` `ty2024()`). Recomputed both worked examples and the
reduce-to-delta invariant with an independent exact-Decimal script (ROUND_HALF_EVEN, same as
`round_cents`), not by re-running the engine's own tests.

---

## 1. Item-by-item verdicts

### 1.1 `magi_excluding_crypto = AGI` (deep/02 C1) — **CONFIRMED, could not break it**

2024 Instructions for Form 8960, *Line 13—Modified Adjusted Gross Income (MAGI)*, verbatim from the
2024 PDF: "**If you didn't exclude any amounts from your gross income under section 911 and you
don't own a CFC or PFIC, your MAGI is your AGI as reported on Form 1040 or 1040-SR.**" The 2024
*Line 13—MAGI Worksheet* is 4 lines: (1) AGI; (2) Form 2555 FEIE (line 42) net of line-44 allocable
deductions; (3) "Adjustments for certain CFCs and certain PFICs"; (4) sum → Form 8960 L13.
**Nothing else is ever added.** Attack log — everything I tried to make MAGI ≠ AGI in scope, and why
each fails — is §3. The fail-closed guard (deep/02 §1.6: refuse on any §911/CFC/PFIC input) is
correct and necessary; hardening note §4(a).

### 1.2 Absolute NII must be rebuilt from line items; `nii_with` is NOT it (C2/§4.2) — **CONFIRMED**

Code: `compute.rs:359` — `nii_with = qd + with.ordinary_gain + with.preferential_gain −
with.loss_deduction + interest_nii`. It contains **no** whole-household taxable interest and only
the *qualified* slice of dividends (`qd`), because both are with/without-constant and cancel in the
delta (`compute.rs:405` subtracts). The absolute Form 8960 needs (2024 instructions, verbatim):
- **L1** ← "Form 1040 or 1040-SR, line 2b" — ALL taxable interest;
- **L2** ← "Form 1040 or 1040-SR, line 3b" — ALL ordinary dividends (qualified ⊂ 3b; the
  qualified split is a §1(h) rate concept only — §1411 includes every dividend);
- **L5a** ← "combining … **Form 1040 or 1040-SR, line 7, and Schedule 1 (Form 1040), line 4**" —
  so a net capital loss enters at the §1211-limited figure that reaches L7 (matches the engine's
  `− loss_deduction` term), and crypto Schedule D + box 2a arrive via L7.
The worked $6,000 gap (59,000 vs 53,000 = non-crypto interest 4,000 + non-qualified dividends
2,000) reproduces exactly (§2). The asymmetry claim also verifies: `magi_with` (`compute.rs:368`)
**does** equal absolute L13 because `magi_excluding_crypto = AGI` already carries all
interest/dividends and `crypto_agi` (`compute.rs:364`) is the true crypto AGI delta. Reuse MAGI;
rebuild NII. Exactly as locked.

### 1.3 W-2 owner-tag: box 5 aggregates household-wide, box 3 is per-earner (C4) — **CONFIRMED, the difference is real**

- **Box 5 household:** 2024 i8959 Line 1, verbatim: "If you are filing a joint return, **also
  include your spouse's wages and tips**." Line 19 likewise ("include your spouse's Medicare tax
  withheld"). One Form 8959 per MFJ return; Part II L10 = L4 = household Σ box 5.
- **Box 3 per-earner:** 2024 Schedule SE is titled "Name of person with self-employment income";
  2024 i1040sse verbatim: "**If both spouses have self-employment income, each must file a separate
  Schedule SE.**" Line 8a ("total of boxes 3 and 7 on Form(s) W-2") is that person's own W-2s —
  §1402(b)(1) caps against the individual's wages.
- **Divergence is material:** SE-earner box 3 = 50,000 / spouse = 168,600 → per-earner ss_cap =
  118,600 (SS tax due on min(base, 118,600)); a household sum (218,600) would zero the cap and
  wrongly eliminate the 12.4% portion entirely. The `04` §5 derivation line `w2_ss_wages: Σ(w2.box3
  + w2.box7)` (household sum) is indeed wrong as written; deep/02's owner-tag correction stands.

### 1.4 Sch 2 L4 = SS + regular Medicare only; the 0.9% routes via 8959 → Sch 2 L11 (C5) — **CONFIRMED from both sides**

- **Form side:** 2024 Schedule SE L10 = 12.4% SS, L11 = 2.9% Medicare, **L12 = "Add lines 10 and
  11. Enter here and on Schedule 2 (Form 1040), line 4"** — the 0.9% appears nowhere on Schedule
  SE. 2024 Schedule 2: L4 "Self-employment tax. Attach Schedule SE"; **L11 "Additional Medicare
  Tax. Attach Form 8959"**; L12 "Net investment income tax. Attach Form 8960". 2024 Form 8959 L18:
  "Add lines 7, 13, and 17. Also include this amount on Schedule 2 (Form 1040), line 11."
- **Code side:** `se.rs:160` `total = ss + medicare + addl` — `total` bundles the 0.9%. Feeding
  `total` to Sch 2 L4 while filing Form 8959 double-counts `addl`. The unbundle rule (Sch 2 L4 =
  `ss + medicare`; 8959 L13 = `addl`) is exactly right. Bonus consistency: `se.rs:162`
  `deductible_half = (ss + medicare)/2` = Schedule SE L13 ("Multiply line 12 by 50%") — already
  excludes `addl`, corroborating that `se.rs` itself knows `addl` is not a Schedule-SE item.

### 1.5 Form structure / routing claims — **CONFIRMED against the 2024 forms**

2024 Form 8960 lines 1–17 and 2024 Form 8959 lines 1–24 match deep/02's 2025 transcriptions
line-for-line (thresholds MFJ 250,000 / MFS 125,000 / Single·HoH·QSS 200,000 on 8959 L5/L9/L15;
L8 = "Self-employment income from Schedule SE (Form 1040), Part I, line 6. If you had a loss,
enter -0-"; L24 → "Form 1040 … line 25c"; 8960 L8 = combine 1, 2, 3, 4c, 5d, 6, 7; L17 → "include
on your tax return" per i8960 = Sch 2 L12). The Kathleen/Liam threshold-coordination example is in
the **2024** instructions verbatim too ("The $130,000 of Kathleen's wages reduces Liam's
self-employment income threshold to $120,000"). Employer-withholding trigger confirmed: "Your
employer must withhold Additional Medicare Tax on wages it pays to you in excess of $200,000 for
the calendar year, **regardless of your filing status**" — so Ex-2's Part V L22 = $180 two-stage
treatment (payment on 25c, distinct from the Sch 2 L11 tax) is right.

### 1.6 Statutory/code constants — **CONFIRMED**

`tables.rs`: `NIIT_RATE` 0.038, `niit_threshold` 250k/200k/125k (Qss→Mfj), `SE_RATE_ADDL_MEDICARE`
0.009, `SE_NET_EARNINGS_FACTOR` 0.9235, `se_addl_medicare_threshold` same 250k/200k/125k.
`ty2024()`: MFJ brackets 0/23,200/94,300/201,050/383,900/487,450/731,200 at 10–37%; `max_zero`
94,050, `max_fifteen` 583,750; `ss_wage_base` 168,600. All as cited in deep/02's appendix.

---

## 2. Worked examples — re-derived independently, all figures match to the cent

Recomputed with a standalone exact-Decimal script (independent implementation of the bracket sum,
§1(h) stack, `net_1222` cross-netting, and the NIIT closure; ROUND_HALF_EVEN):

| Figure | deep/02 | re-derived |
|---|---|---|
| Ex 1: AGI / taxable / ordinary | 287,000 / 257,800 / 246,800 | ✓ identical (round-trip identity holds) |
| Ex 1: absolute 8960 L17 | 646.00 | ✓ 0.038 × min(17,000, 37,000) |
| Ex 1: engine delta NIIT | 0.00 | ✓ (with == without) |
| Ex 1: 8959 Part I L7 | 180.00 | ✓ 0.9% × (270,000 − 250,000) |
| Appendix: `ordinary_tax_on(246,800)` / pref / QDCGT | 45,317.00 / 1,650.00 / 46,967.00 | ✓ all three |
| Ex 4.2: `nii_with` / `magi_with` | 53,000 / 329,000 | ✓ |
| Ex 4.2: `niit_with` / `niit_without` / **delta** | 2,014.00 / 418.00 / **1,596.00** | ✓ |
| Ex 4.2: absolute L8 / **L17** | 59,000 / **2,242.00** | ✓ (gap = 6,000 = 4,000 int + 2,000 non-qual div) |
| Ex 2: 8959 L7 / L11 / L13 / **L18** | 270.00 / 0 / 498.69 / **768.69** | ✓ |
| Ex 2: Part V L21 / L22 / L24 | 4,060.00 / 180.00 / 180.00 | ✓ (box-6 inputs internally consistent: 3,370 = 1.45%×220k + 0.9%×20k; 870 = 1.45%×60k) |
| Ex 2 `se.rs`: base / ss / medicare / addl / total | 55,410.00 / 0.00 / 1,606.89 / 498.69 / 2,105.58 | ✓ (`addl` ≡ 8959 Part II L13 exactly) |

**Reduce-to-delta invariant** (deep/02 §4.3.1), probed numerically in four regimes with all
non-crypto inputs zeroed: big ST+LT gains + lending interest (delta = absolute = 6,460.00); pure LT
gain 400k (5,700.00); LT loss + interest + large crypto ordinary income pushing MAGI over the
threshold (76.00); and the negative-NII floor edge (interest 500 < loss_deduction 3,000 → both 0.00
— engine floors inside the closure, the form floors at L12, same answer). **Equality held in all
four.** The companion "absolute ≥ delta" invariant also survives adversarial probing: non-crypto
NII line items (interest, box 1a, box 2a) are all ≥ 0, so absolute NII ≥ `nii_with` while MAGI is
shared, and NIIT is monotone in NII; when crypto losses make the delta negative, absolute ≥ 0 >
delta trivially.

---

## 3. Attack log — attempts to make MAGI ≠ AGI (or break the NII assembly) in scope; all failed

1. **Tax-exempt interest / exempt-interest dividends** (INT box 8, DIV box 12): NOT on the 2024
   MAGI worksheet; also "excluded income" for NII per i8960 definitions. Correctly out of both.
   (8960's MAGI is unusual — unlike the IRA/PTC/Social-Security MAGIs, it adds back *no* domestic
   exclusions.)
2. **Form 8815 savings-bond interest exclusion**: reduces AGI; the 8960 worksheet does NOT add it
   back → MAGI stays = AGI. No break.
3. **Form 8814 (kiddie election)**: the child's income is IN the parent's AGI; i8960 routes the
   line-12 amount to 8960 **line 7** (NII), not to a MAGI adjustment. Not a MAGI break; it IS an
   NII line-item that `ReturnInputs` cannot represent — structurally fail-closed (no 8814 input
   exists). Keep it that way.
4. **Schedule K-1 (Form 1041) box 14 code H**: the one non-obvious MAGI adjuster in the 2024
   instructions ("increase your MAGI on Form 8960, line 13") — it is part of the CFC/PFIC/trust
   adjustment family (worksheet line 3) and structurally unrepresentable (no K-1 input). Covered by
   note §4(a).
5. **§6013(g)/(h) NRA-spouse elections** (checkboxes on the 2024 Form 8960 header): change the NIIT
   filing/threshold treatment. No NRA-spouse input exists in scope; structurally fail-closed.
6. **§404(k) ESOP dividends inside box 1a**: i8960 says back them out on line 7 (negative). Not
   modeling this can only OVERSTATE NII (conservative direction). Rare; note §4(c).

None of these produces MAGI ≠ AGI, or an NII understatement, within the locked v1 scope.

---

## 4. Non-blocking hardening notes (no findings; fold into the spec at leisure)

(a) **Make the §1.6 fail-closed guard structural, and keep it that way.** Today MAGI ≠ AGI is
impossible because `ReturnInputs` simply has no §911/2555, CFC/PFIC, K-1, 8814, or NRA-election
fields. The spec should state that any future free-form Schedule-1 income field (e.g. a generic
"line 8z other income" scalar) must be **enumerated, not free-form** — a signed catch-all would let
a user smuggle a §911 exclusion (Sch 1 line 8d is a *negative* entry) past the guard and silently
break `MAGI = AGI` and the NII assembly (8814/8z NII items). Refuse or enumerate; never a signed
grab-bag.
(b) **8959 L8 multi-Schedule-SE rule.** 2024 i8959: "Combine amounts from this line if you have
multiple Schedules SE." v1 has one crypto ledger → one SE earner, so a single Sch-SE-line-6 feeds
L8; if a second SE earner ever enters scope, L8 sums both spouses' line 6 while each spouse's SS
cap stays per-earner (§1.3). Worth one sentence in the spec's scope statement.
(c) **Form 8960 Part II = 0 is conservative, say so.** Line 9b (state income tax allocable to
investment income) is an optional NII reduction an itemizing household could claim; omitting it
(and the §404(k)/self-charged-interest line-7 negatives) can only overstate NIIT, never understate.
The spec should record this as a deliberate conservative simplification.
(d) **Terminology nit in the F4 brief (not in deep/02):** the engine computes *NIIT* as a crypto
delta, but Additional Medicare is not in `TaxResult` at all — `se.rs.addl` is a standalone
**level** (= absolute 8959 Part II L13 when fed household box 5). deep/02 §4.1 states this
correctly; carry deep/02's wording into the spec, not the brief's shorthand.

**Bottom line: the locked deep/02 result stands unmodified. 0 Critical / 0 Important. No
corrections to escalate.**
