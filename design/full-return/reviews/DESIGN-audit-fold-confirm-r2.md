# FABLE RE-CONFIRMATION — F1/F2/F3 fold (SPEC r6 + PLAN r4)

**Reviewer:** Fable, focused re-confirmation pass. **Date:** 2026-07-12.
**Mandate:** confirm the confirmation review's 3 Importants (`DESIGN-audit-fold-confirm.md` F1/F2/F3) are
genuinely resolved in SPEC r6 + PLAN r4, with no new regression from the six edits.
Gate: 0 Critical / 0 Important ⇒ sound to begin implementation.

**VERDICT: GATE MET — 0 Critical / 0 Important / 7 Minor.**
F1, F2, and F3 are each resolved at every required site, verified against primary sources (the project's own
2024 Schedule 2 extractions, §1(g)/Form 8615, IRS Notice 2023-75). The regression scan over the six edits is
clean: no surviving inversion, no orphan reference, no new understatement path. All seven residual findings
are Minor (bookkeeping/wording; none changes a computed figure or weakens a refuse path).

**THE DESIGN (SPEC r6 + PLAN r4) IS SOUND TO BEGIN IMPLEMENTATION.**

---

## 1. Finding confirmations

### F1 (Schedule 2 revert) — RESOLVED at all five sites

Target structure (2024 Sch 2 Part I): **L1a = excess-APTC → L1z; L2 = AMT; L3 = L1z + L2 → 1040 L17.**

Primary-source verification: the project's own extraction `recon/deep/03-ty2024-field-maps.md:222` gives the
leaf spine "L1z=`f1_11`, **L2 AMT**=`f1_12`, L3→1040 L17=`f1_13`" (extracted via `qpdf --json` from
`f1040s2--2024.pdf`, deep/03:10-13); `recon/01-form-graph.md:46` reads the same ("AMT **L2** / excess APTC
repayment **L1**"); and the prior confirmation pass fetched the IRS PDF directly (2026-07-12, FINAL) with the
identical layout. Three independent reads agree with the r6 text.

Site-by-site (all verified in current SPEC r6):

- **§5 stage 5 (:365):** "Sch 2 Part I: **L1a = excess-APTC** (refuse if any, §4.10) → L1z; **L2 = AMT**
  (screen=0, §4.11); L3 = L1z + L2 → 1040 L17; L18=L16+L17" — orientation AND sum formula (L3 = L1z + L2)
  both correct. ✓
- **§4.11 (:334):** "Sch 2 **L2 (AMT)** = 0, **not silently**" — the explicit screen-zero now lands in the
  correct cell (`f1_12`), so P6 will not print a $0 into 1a (which would have visually asserted a Form-8962
  reconciliation). ✓
- **§4.10 APTC row (:318):** "Sch 2 **L1a** repayment (would understate)". ✓
- **§9.2(ii) (:469):** "excess-APTC/Form 8962 (Sch 2 **L1a**)" — the original r4 erratum finally corrected. ✓
- **§10 note (:507-509):** retracts the false erratum and restores the project record — "as recon-01 §2 /
  deep/03 correctly read (verified against `f1040s2--2024.pdf`) … An earlier 'erratum' that inverted it to
  L1=AMT was itself wrong and is retracted." recon-01 is no longer branded wrong. ✓

Whole-corpus grep for surviving inversions: the only "L1=AMT" strings are the **r5 historical changelog**
(:16-17, an accurate record of what r5 did, immediately superseded by the r6 changelog at :7-11 which brands
it a mis-fix) and the retraction note itself (:509). Neither is normative. The plan never states a Part I
orientation (verified — its only Sch 2 reference is Part II L4, plan:164). ✓

### F2 (kiddie tax) — RESOLVED; formula verified conservative; phase move consistent

- **§4.10 row (:316):** "unearned = gross income − earned income [wages + Sch C net] → includes interest,
  dividends, capital gains, hobby-crypto L8v, unemployment L7, taxable refunds L1; **compute-dependent —
  screened in P2** after income assembly, not P1". ✓ The complement form structurally includes every in-scope
  income line — the F2 leak classes (L8v hobby staking, L7 unemployment, L1 refunds, custodial capital gains)
  are all inside by construction.
- **§1(g) correctness check** (§1(g)(4): unearned = the AGI portion not attributable to §911(d)(2) earned
  income): statutory unearned = L9 − (½SE + L18 + L21 adjustments) − (wages + SchCnet − ½SE)
  = L9 − wages − SchCnet − L18 − L21. The spec's operational formula (gross[L9] − wages − SchCnet) exceeds
  this by (L18 + L21) ≥ 0 and the ½SE terms cancel exactly — i.e. the spec **never understates** unearned
  income; any divergence over-triggers the refuse (fail-closed, refusal is a correct answer). It also has no
  dependency on the ½-SE wiring (P4), so the P2 placement is self-contained. ✓
- **Plan P1 t4 (:93-99):** the input-screenable KAT list no longer contains the kiddie row; it is explicitly
  routed "**Form 8615 kiddie-tax → P2 (C1/F2) … needs assembled income — KAT-19**". ✓
- **Plan P2 t2 (:116-118):** owns the screen + KAT-19 — "after income assembly compute unearned = gross −
  earned[wages + Sch C net]; claimable dependent + unearned > $2,600 ⇒ refuse". ✓
- **Ordering:** unearned needs 1040 L7 (P2 t1 "Sch D L7 reuse") and the Sch 1 lines + Sch C net (P2 t2
  itself); the refuse fires in P2, structurally **before** L16 (`qdcgt_line16`, P3 t4). No ordering hazard. ✓
- Threshold $2,600 (TY2024) re-confirmed (Rev. Proc. 2023-34; = 2× the $1,300 dependent floor already
  bundled); §8 (:456-457) bundles it as indexed `TaxTable` data. ✓

### F3 (§402(g) excess deferral) — RESOLVED via the pre-authorized blunt variant; $23,000 verified

- **§4.10 row (:321):** "**Σ box-12 elective deferrals (D/E/F/G/S) across employers > §402(g) $23,000**
  (TY2024) ⇒ refuse — excess deferral is taxable on 1040 **1h** — the allowlist codes are inert only *up to*
  the §402(g) limit". ✓ Coherent with the allowlist row (:320): admission first, then the amount condition on
  the deferral subset; the headline leak (job-change dual-401(k), 2×$13k code D) is closed.
- **Limit verified:** IRS Notice 2023-75 — TY2024 §402(g)(1) elective-deferral limit = **$23,000** (catch-up
  $7,500; SIMPLE $16,000). The 1040 destination is right: 2024 Form 1040 instructions, line 1h ("Other Earned
  Income") is where excess elective deferrals are reported — a line v1 does not compute, hence refuse. ✓
- **§8 (:457):** "§402(g) elective-deferral limit $23,000 (TY2024, indexed — F3)" bundled in `TaxTable`. ✓
- **Plan P1 t4 (:93-96):** `screen(&ReturnInputs, &TaxTable)` "[needs the year table for excess-SS MAX +
  §402(g) limit]"; input-screenable KAT "**Σ box-12 D/E/F/G/S > §402(g) $23,000 ⇒ refuse (F3)**". Correctly
  input-screenable (pure W-2 data). ✓
- The fold chose the flat-$23,000 variant the prior confirmation explicitly pre-authorized ("a blunter
  refuse-over-$23,000-flat … is also acceptable fail-closed, at the cost of refusing legitimate 50+ maxers");
  over-refusals (50+ catch-up, 401(k)+457(b) dual max, folded SIMPLE) are the accepted fail-closed cost.
  Residual wording-level deltas vs the reviewer's formula → Minors 4–5 below (none reopens the leak class).

## 2. Regression scan (six edits) — CLEAN

- **Sch 2 internal agreement:** §5 stage 5 (Part I) and stage 7 (Part II: L4 SE, L11 8959, L12 8960, L13
  box-12 uncollected, L17h/L17k in the allowlist rationale, L21 → 1040 L23) verified against deep/03's leaf
  map — all Part II references untouched and correct; the §4.10 rows and KAT-14 reference the AMT screen with
  no orientation dependency. ✓
- **Stale-string grep** over spec+plan: no live "L1 = AMT"/"L2 = excess-APTC" (changelog/retraction only); no
  "Σ int+div+capgain" anywhere; `qbi_deduction_override`/`qbi_override` only in the drop documentation
  (§4.5:243) and changelogs. ✓
- **Kiddie P2 placement:** consistent across §4.10 row / §10 KAT-19 / plan KAT routing / plan P2 t2; no
  ordering cycle (needs stage-1/2 income, precedes stage-4 tax). The row's formula deliberately avoids ½SE,
  removing any P4 dependency. ✓
- **Refuse-surface monotonicity:** all r5 rows are retained verbatim; r6 only widened the kiddie predicate
  (complement ⊇ the old Σ), added a new refuse row (402(g)), and corrected cites — no previously-verified
  refuse/omit path is weakened. ✓
- **§3.4 carve-out:** the 402(g) row guards a tax-increasing configuration (1h income) — correctly a refuse,
  not a favorable omission. ✓

## 3. Minor findings (none gates; fold opportunistically / at P1)

1. **m-r2-1** — FOLLOWUPS `fr-8962-taxonomy` still cites "(Sch 2 L2)"; stale after the F1 revert — update to
   L1a when that item is folded (`design/full-return/FOLLOWUPS.md`).
2. **m-r2-2** — plan:8 "**Implements:** … (GREEN r4)" is stale (spec is DRAFT r6, pending this gate) — the
   prior pass's m5, still unfixed; update on the GREEN stamp.
3. **m-r2-3** — plan KAT-ownership index (:42-43) doesn't reflect the P2-owned compute-dependent refuse KATs
   (KAT-19 kiddie, Sch C net<0); its "compute-dependent: TI≤0, excess-SS" enumeration is stale (same class as
   open `pm-r2-m1`). Task text is unambiguous; index-only fix.
4. **m-r2-4** — the 402(g) row says "across employers" but drops the prior review's "**per person**"
   qualifier (§402(g) is per-individual; cf. §4.9's explicit "per person (never pooled)"). A household-pooled
   misreading would over-refuse the modal dual-earner MFJ (each legitimately at $23k). Fail-closed direction
   + the statutory cite disambiguate → Minor; add the two words at P1.
5. **m-r2-5** — the Σ omits code **H** (501(c)(18)(D), present in the prior review's fix formula) and Roth
   **AA/BB** (designated Roth deferrals count toward the §402(g) aggregate; a pre-tax+Roth mix over $23k can
   leave a pre-tax excess allocable to 1h). Residual populations are tiny (code H is near-nonexistent; the
   Roth-mix excess is taxpayer-allocable) and the prior review's own accepted formula also excluded AA/BB.
   Cheapest hardening: include H/AA/BB in the Σ (only over-refuses genuinely-over-limit filers — safe). Vet
   at P1 KAT time.
6. **m-r2-6** — label collision: the confirmation-review tags F2/F3 now coexist with the recon-doc tags
   F2/F3 in the same sections (§8:457 has "indexed — F3" [confirmation] adjacent to "(F2)" [recon fable/02]).
   Context disambiguates; consider distinct tags (e.g. CF2/CF3) at next touch.
7. **m-r2-7** — the folded-in SIMPLE limit ($16,000 < $23,000 flat) leaves a two-SIMPLE $16k–$23k window
   unrefused; single-plan excess is administrator-prevented (the prior review's own argument), two concurrent
   SIMPLEs is rare. Record alongside m-r2-5 in FOLLOWUPS.

## 4. Disposition

| # | Item | Status |
|---|---|---|
| F1 | 2024 Sch 2 Part I revert (L1a=APTC→L1z, L2=AMT, L3=L1z+L2) at §5/§4.11/§4.10/§9.2/§10 | **RESOLVED** — verified vs deep/03 + recon-01 + the IRS PDF fetch |
| F2 | Kiddie unearned = gross − earned (complement, never-understates) + P1→P2 move + KAT-19 | **RESOLVED** — formula proven conservative vs §1(g)(4); phases consistent |
| F3 | §402(g) $23,000 refuse row + §8 bundling + P1 input-screen + KAT | **RESOLVED** — via the pre-authorized blunt variant; limit verified (Notice 2023-75) |
| m-r2-1…7 | bookkeeping / wording residuals | Minor — fold at P1 or opportunistically |

**Gate: MET — 0 Critical / 0 Important / 7 Minor.**
**The design (SPEC_full_return.md r6 + IMPLEMENTATION_PLAN_full_return.md r4) is SOUND TO BEGIN
IMPLEMENTATION.** Recommended before the GREEN stamp (non-blocking): the two-word m-r2-4 fix and the m-r2-2
status-line update; transcribe m-r2-1/3/5/6/7 into `design/full-return/FOLLOWUPS.md`.
