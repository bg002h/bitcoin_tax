# FABLE FINAL DESIGN AUDIT — full-return v1 (whole-corpus, pre-implementation)

**Auditor:** Fable, fresh context (did not see the incremental review rounds). **Date:** 2026-07-12.
**Mandate:** single adversarial whole-design audit of SPEC (GREEN r4) + PLAN (GREEN r2) + recon
(`00`–`05`, `deep/01`–`05`, `fable/00`–`06`) + `FOLLOWUPS.md`, hunting **cross-artifact and emergent**
issues the per-artifact loops structurally could not see. Settled single-artifact findings were not
re-litigated.

**VERDICT: NOT yet sound-to-build as written — 1 Critical / 4 Important.** All five are small,
localized design changes (one new refuse-guard row + input-surface tightenings + one orphaned spec
field); none disturbs the architecture, the frozen-engine seam, the phase structure, or any locked
recon result. Fix the Critical and Importants in the spec (a §4.10/§4.4a/§4.5 edit + one plan-task
addition), then implementation may proceed. The corpus is otherwise remarkably coherent — the full
list of what was checked and found clean is in §5.

---

## 0. What was audited

**Design corpus (read in full):** `SPEC_full_return.md` (520 lines); `IMPLEMENTATION_PLAN_full_return.md`
(219); recon `00-SYNTHESIS`, `01`–`05`, `deep/01`–`05`, `fable/00-SYNTHESIS-FABLE`, `fable/01`–`06`,
`FABLE_RECON.md`; `full-return/FOLLOWUPS.md`.

**Source verified against current code (load-bearing claims):**
`crates/btctax-core/src/tax/types.rs` (TaxProfile contract :34-38, :114-comment lineage),
`compute.rs` (delta assembly :336-416; `nii_with` :359; `crypto_agi` :364-368; NIIT closure :369),
`se.rs` (`se_net_income` :55-62 predicate; `compute_se_tax` :99-173), `tables.rs` (statutory
constants), `event.rs` (`IncomeKind` :33-39 = exactly {Mining, Staking, Interest, Airdrop, Reward};
`Income.business` :61), `conventions.rs` (`round_cents` = half-even :13-24; no `round_dollar` yet),
`btctax-adapters/src/tax_tables.rs` (`ty2024()` :234; bundled years 2017/2024/2025/2026; statutory-vs-
indexed doc :9-11), `btctax-forms/src/schedule_d.rs` (:5-6 L17–22 scope-out), `schedule_se.rs`
($400 floor on `base`; line 12 = ss+medicare; line-9 clamp), `verify.rs` (grid + flat oracles;
Yes/No-pair-only checkbox model), `lib.rs` (`fmt_money` = raw Decimal; SUPPORTED_YEARS).

Every spec/plan/recon code citation I checked resolves correctly against current source (details in §5).

---

## 1. CRITICAL

### C1 — Form 8615 (kiddie tax) is a reachable, silent, tax-UNDERSTATING hole in the fail-closed surface

**Locations:** SPEC §4.2 (`can_be_claimed_as_dependent_{taxpayer,spouse}`, `date_of_birth`),
§4.7 (dependent standard-deduction floor + G21 earned-income derivation — i.e., **dependent filers are
deliberately in scope**), §4.3 (unbounded 1099-INT/DIV `Vec`s), §5 stage 4 (L16 =
`method.rs::qdcgt_line16` unconditionally), §4.10 (no screening row); recon `deep/01`/`fable/02`
(locked the Tax-Table/QDCGT method without its i1040 Line-16 preconditions); `fable/06` (line-walked
the 1040 but never the L16 method preconditions); `fable/04` §3.3 (probed **Form 8814**, the
*parent-side* election, for NIIT only — never Form **8615**, the *child-side* mandatory computation).
Neither "8615" nor "kiddie" appears anywhere in the corpus.

**The emergent problem.** The 2024 i1040 Line-16 instructions make the Tax Table / TCW / QDCGT
worksheet the *default* method with enumerated exceptions. The corpus handles every exception but one:
Schedule D Tax Worksheet (refused via DIV 2b/2c/2d, §4.10 ✓), Form 2555 (structurally absent ✓),
Form 8814 / 4972 / Schedule J (no inputs ✓) — but **Form 8615** is mandatory for a filer with
unearned income > **$2,600** (TY2024, Rev. Proc. 2023-34 §3.16 — the same section the $1,300
dependent floor comes from) who is under 18 (or 18–23 under the student/support rules) with a living
parent. Such a filer's tax is computed at the *parent's* rate — `qdcgt_line16` alone is **wrong, in
the understating direction**.

This is squarely reachable in scope: the spec *invites* dependent filers (the §4.7 dependent floor
and its KAT exist precisely for a claimable filer), captures DOB and the claimable flags, and accepts
arbitrary interest/dividends/capital gains. A 17-year-old with a W-2 and $3,000 of dividends in a
custodial account files a silently wrong return. §3.4's own rule ("any captured-but-unmodeled input
that would *increase tax* … produces `NotComputable`") is violated by its enumerated table.

**Why only cross-artifact:** each artifact is locally green — deep/01+fable/02 locked the *method*
(their mandate excluded filing-population preconditions), fable/06 walked *lines* not method
preconditions, the spec's §4.7 dependent support and §4.10 refuse table were reviewed in different
sections. The hole exists only in the conjunction {dependent filers in scope} ∧ {unearned income in
scope} ∧ {L16 method unconditional}.

**Fix (small, spec-level):** add a §4.10 row — refuse when
`(can_be_claimed_as_dependent_taxpayer || …_spouse)` **and** unearned income (2b + 3b + max(0, L7) +
non-business L8v) > the year's §1(g)/Rev.-Proc. threshold ($2,600 TY2024), *or*, conservatively,
when a DOB-derived age < 24 with unearned income over the threshold (age ≥ 24 can never be 8615).
Add the threshold to the per-year `TaxTable` (§8 — it is indexed, same Rev.-Proc. §3.16 family as the
dependent floor already being bundled). One KAT per §4.10 convention. Optionally an attest input
("not subject to Form 8615") as a follow-on; a refusal is a correct v1 answer, a silent number is not.

---

## 2. IMPORTANT

### I1 — The W-2 box-12 refuse-guard is a blocklist where fail-closed requires an allowlist (codes K, R, T leak)

**Locations:** SPEC §4.1 (`box12: Vec<Box12Entry>` captured verbatim), §4.10 (rows only for
{W}, {A/B/M/N}, {Z}), §3.4 (general rule); recon `fable/06` G9 (source of the enumerated set);
`fable/04` hardening (a) ("enumerate, never a grab-bag" — the same principle, stated for Sch 1).

**Problem.** Box 12 is captured as an open `Vec<(code, amount)>`, but §4.10 refuses only six codes.
At least three *other* codes are captured-but-inert and tax-affecting — exactly the class §3.4
forbids: **K** (20% golden-parachute excise → Sch 2 L17k; silent understatement of total tax),
**R** (Archer MSA employer contributions → Form 8853 mandatory), **T** (adoption benefits → Form 8839
reconciliation; excess is taxable wages the return would omit). §3.4 (the norm) and §4.10 (the
enumeration) disagree on coverage; an implementer building "one KAT per row" ships the leak.

**Why cross-artifact:** G9's list was adopted verbatim into §4.10; the spec review verified the rows
present, not the *complement* of the captured input space against §3.4 — a check that requires holding
§4.1's open capture, §4.10's closed table, and §3.4's rule together.

**Fix:** invert to an allowlist — enumerate the known-inert codes (C, D, E, F, G, H, J, L, P, S, V,
Y, AA, BB, DD, EE — verify the list once at spec time) and **refuse any box-12 code outside it**
(single row: "box-12 code ∉ inert allowlist ⇒ refuse", one KAT with an exotic code). This also
future-proofs against new IRS codes.

### I2 — Schedule C net loss (`expenses` > derived gross) is unhandled and prints an incomplete/wrong return

**Locations:** SPEC §4.4a (`expenses: Usd` unbounded; "Net profit = gross − expenses → Sch 1 L3");
§5 stage 7 (§6017 floor covers only the SE side); frozen `se.rs:110-118` (floors net_se at 0 — SE
side safe); the Schedule C form itself (line 31 loss ⇒ **line 32a/32b at-risk boxes mandatory**);
PLAN P2 task 2 / P6 task 2 (no loss branch anywhere).

**Problem.** Nothing stops `expenses > gross`. The SE path silently floors to zero (correct), but the
income-tax path as specced would put a **negative** number on Sch 1 L3, reducing AGI, while the filled
Schedule C omits the mandatory at-risk answer (line 32) and none of the loss-limitation doctrine
(at-risk §465, hobby §183, excess-business-loss §461(l)) is modeled. That is a deduction the design
cannot substantiate — the taxpayer-favorable direction, which the §3.4 carve-out explicitly does
NOT cover (the carve-out permits favorable *omissions*, never favorable *claims*).

**Why cross-artifact:** the SE recon (deep/02, fable/04) analyzed profitable fixtures; se.rs's
`max(0,·)` makes the SE side look closed; only joining §4.4a's unbounded input to the Schedule-C
*form* obligations and the income-side flow exposes it.

**Fix:** §4.10 row — refuse when `schedule_c.expenses > derived gross business income` ("Schedule C
loss out of scope: at-risk/§183 unmodeled"), + KAT. (v1 keeps `net ≥ 0`; a loss year is a legitimate
follow-on.)

### I3 — `qbi_deduction_override` has no defined semantics and no plan home

**Locations:** SPEC §4.5 (field declared: `qbi_deduction_override: Option<Usd>`), §7.1 ("box5>0/
**override** forces the 8995 map"); PLAN P4 task 1 (implements only the computed path — the word
"override" does not appear anywhere in the plan); recon-04 §5.1 (origin: a pre-D-1 escape hatch from
before auto-compute was decided).

**Problem.** A spec-declared input with (a) no rule for when it wins over the computed Form 8995
L15, (b) no story for how an 8995 *prints* when the override disagrees with the computed lines
(an overridden L15 that ≠ min(L10, L14) is a self-inconsistent attached form — violating the §7.1
"never a line with no [consistent] backing form" closure), (c) an open question whether it can
smuggle a QBI claim past the §4.5 over-threshold/non-REIT **refuse** rules, and (d) no plan task —
this is the one place the spec builds something the plan doesn't, and vice-versa the plan cannot
faithfully implement §4.5 as written.

**Why cross-artifact:** it is a recon-04 vestige (escape hatch designed *before* D-1 chose
auto-compute) that survived into the spec; the spec review saw a plausible field, the plan review
implemented §4.5's compute sentence — only the spec↔plan walk shows the orphan.

**Fix (pick one, pre-coding):** (1) **drop the field for v1** (recommended — the raw
`TaxProfile` path already exists as the global escape hatch, §2); or (2) define: honored only when
≤ computed envelope, still subject to every §4.5 refuse rule, printed 8995 shows *computed* lines and
refuses if `override ≠ computed L15`; + plan task + KAT.

### I4 — (Downgraded after verification — recorded as resolved probe.) None. The fourth candidate
(printed-line rounding vs "to the cent" acceptance) verified as coherent once the KAT-9 P4/P6 split
is read as "cents internally, `round_dollar` at form-line materialization" — kept as observation M4
so the implementer states it explicitly rather than discovering it.

---

## 3. MINOR / OBSERVATIONS (fold into FOLLOWUPS; none gates)

- **M1 — Sch 2 L2 double-booked inside the spec.** §5 stage 5 says "L2 AMT" (matches deep/03's
  geometric extraction: 2024 Sch 2 `L1z=f1_11, L2 AMT=f1_12`) while §9.2(ii) cites "excess-APTC/Form
  8962 **(Sch 2 L2)**". Both cannot be L2 on the 2024 revision (APTC repayment lives in the L1a–1z
  block). Fix the §9.2 cite when folding `fr-8962-taxonomy`.
- **M2 — Derived-profile pref>TI clamp.** `derive_tax_profile` (recon-04 §5.1 step 11, PLAN P2 t3)
  can hand the frozen engine `qd + other_net_capital_gain > taxable income` in a low-TI/high-QD year;
  the frozen `preferential_tax` has no L10 = min(L1,L4) cap (that clamp lives in the new
  `qdcgt_line16` only, F2 F-A). The *filed* return is unaffected (absolute path is correct); only the
  delta/planning number degrades. Clamp at derivation or add it to §6's documented approximations.
- **M3 — Capital-loss carryforward is excluded from the R3-M6 write-back set** (charitable + QBI
  only), staying manual with the old M4 advisory. Asymmetric but not wrong; consider adding for
  uniformity in a follow-on.
- **M4 — State where `round_dollar` is applied.** P4's acceptance ("Ex.2 … to the cent") plus §3.1's
  printed-line rounding coexist only if `other_taxes.rs`/`return_1040.rs` carry cents internally and
  round at the form-line boundary (which the KAT-9 P4/P6 dual ownership implies). One sentence in the
  P4 task list prevents churn.
- **M5 — Reduce-to-delta edge below the §6017 floor.** Absolute path zeroes SE/½-SE/8959-L8 when
  base < $400 (spec stage 7); the frozen `se.rs` has no floor (the *filler* applies it,
  `schedule_se.rs:59`). KAT-5's fixtures avoid the band; scope the invariant wording to base ≥ $400.
- **M6 — Excess-SS "employer" identity is a free-text string** (no EIN captured). Two W-2s from the
  same employer would pass the "≥2 employers" test and claim a non-creditable excess. Rare; match on
  the employer string + advisory, or document.
- **M7 — HoH/QSS qualifying-person write-in name** (2024 1040 `f1_18`/`f1_19`) has no input; an HoH
  return prints with that header field blank. Capture or LIMITATIONS-note.
- **M8 — LIMITATIONS list (iii) must be exhaustive at P5**: add non-crypto capital sales
  (1099-B/stocks — the biggest real-world absence for a brokerage household; only box-2a
  distributions are modeled), alimony received, W-2G gambling, Schedule H, 1099-INT boxes 10–13/OID
  (box-10 market discount is an understatement if the user holds one), K-1s. The spec's list is
  illustrative; the shipped doc should be per-line (G24's spirit).
- **M9 — The blanket TI ≤ 0 refuse (G22/§4.10) excludes refund-only low-income filers** (wages below
  the standard deduction, withholding to recover). Correctly fail-closed, but the carryover-worksheet
  rationale only bites when a capital loss exists — a follow-on could narrow the refuse to
  "TI ≤ 0 ∧ capital loss present" and serve that demographic.
- **M10 — §4.8 doc nit:** note that `estimated_tax_payments` (→L26) includes any prior-year
  overpayment applied (recon-04 §4 said it; the spec field doc dropped it).
- **M11 — P6 effort flag (landmine, not defect):** the read-back oracle extensions are the riskiest
  filler work — the 2024 filing-status "group" is 5 independent checkboxes across *two containers at
  two x-positions and three y-rows* with per-year on-states (deep/03), nothing like the existing
  adjacent-pair oracle (`verify.rs::topmost_yes_no_pair`); negative-cell verification must interact
  with the per-line `neg:` sign policy (magnitude-in-parens cells read back as positive text). The
  plan schedules both (P6 t1/t2) — budget accordingly.

---

## 4. FOLLOWUPS.md — no hidden blockers

Walked all 11 items: `fr-schedc-27a` (map detail, P6-time ✓), `fr-se-sscap-clamp` (verified: both
`se.rs:127-134` and `schedule_se.rs:64-73` already clamp — pure spec-text gap ✓), `fr-schb-user-forced`
(plan P2 t4 drops the clause ✓), `fr-8962-taxonomy` (extend with M1 ✓), `fr-profile-diagram-nit` ✓,
`pm-r2-m1`–`m4` (cosmetic, verified against plan text ✓), `spec-s8-kat3-mod25` (plan already
implements the corrected mod-25 assertion; verified TY2017 9,325 / TY2025 11,925 are ≡ 25 mod 50 —
midpoint edges, harmless ✓), `spec-s48-l36` (L36 pinned 0, P4 t6 ✓). **None gates.**

---

## 5. Audited and found COHERENT (explicit no-findings list)

**Cross-artifact fidelity (spec ↔ plan ↔ recon):**
- Full spec→plan walk: every §3–§11 requirement has a plan home (KAT-ownership block covers all 18
  KATs + refuse rows; phases §11 ↔ P0–P7 are 1:1). The **only** orphan found is I3
  (`qbi_deduction_override`). No plan task builds anything the spec doesn't sanction.
- Every Fable-pass correction landed in the spec: F2 F-A pref-cap (§5 stage 4 + KAT-1), F-B
  binding-min (KAT-2), F-C p.23 cite (§3.1), F1 SALT-halve-last (correctly deferred to TY2025,
  KAT-4 labeled), F3 DOB/SSN-validity capture (§4.2), F5 `f1_57` collision + on-state reassignment
  (§7.4, KAT-7/8), deep/04 ST-crypto-50% (§4.6 classes), deep/02 C1–C5 (MAGI=AGI §5; NII rebuild;
  owner tag §4.1; Sch 2 L4 unbundle + 8959 split, KAT-6/12). All six `00-SYNTHESIS` §8 corrections
  propagated. The spec's R3-M5 move of crypto lending interest from 8960 L1 (deep/02's placement) to
  **L7** is a deliberate, documented improvement (it rides Sch 1 L8v, not 1040 2b) — sum-identical.
- Numbers spot-checked across artifacts and against `ty2024()`/`tables.rs`: brackets, breakpoints
  (incl. MFS 291,850), $10,453.20 excess-SS MAX, $300/$600 FTC, 8995 $191,950/$383,900, thresholds
  250k/200k/125k, $2,500/L21 phase-outs, std-deduction family, KAT-1/2/9 fixture arithmetic re-derived.

**Emergent correctness / DAG:** the full §5 pipeline is acyclic and complete for v1 scope
(income→AGI→SchA-on-with-crypto-AGI→std/itemized→QBI→TI→L16→AMT-screen→FTC(L20)/CTC-omit(L19)→
SE/8959/8960(Sch 2)→L24→payments incl. 8959-PartV→settle). Probed interactions all clean: SE↔QBI-L11
(L11 after deduction, before QBI; no cycle in 2024 — the F3 L13b coupling is TY2025-only), FTC↔CTC
ordering (CTC omitted ⇒ CLW-A interaction vacuous; L22 = max(0,·) clamps), charitable-ceiling↔
with-crypto-AGI↔frozen-delta (G7 + §6 dual-report + P4 t8 consistent), the R3-I4 MAGI-binding
inequality (verified against `compute.rs:364` — the delta's `crypto_agi` is gross-`crypto_ord`-based,
no expenses/½-SE, exactly as the spec scopes; KAT-5 NII-binding equality holds because delta-MAGI ≥
absolute-MAGI), student-loan/IRA phase-outs reading the with-crypto pivot, 8959 inner clamp ≡
`se.rs.addl`, 8960 floors ≡ `compute.rs:369`.
- The spec §4.4a SE-eligibility allowlist ({Mining,Staking,Airdrop,Reward}) is **provably identical**
  to `se.rs:59`'s `kind != Interest` blocklist — `IncomeKind` (event.rs:33-39) has exactly five
  variants. Schedule C gross ≡ the frozen SE base's gross; no divergence possible today (a new
  `IncomeKind` variant would desynchronize them — the FROZEN content-pin on se.rs plus the spec's
  allowlist makes that loud).

**Fail-closed surface:** constructed attacks that all correctly refuse or stay conservative:
DIV 2b/2c/2d, INT box 9/DIV 13, box-12 W/A/B/M/N/Z, boxes 8/10, foreign tax > cap, foreign trust,
≥2 SE earners, business-flagged Interest, single-employer excess-SS, L13/L20-with-deduction, TI≤0,
sales-tax-amount-with-election-off, business-income-without-Sch-C, MFS tri-state, Sch B Part III
tri-state, §911/CFC/PFIC (structural), 8814/K-1/NRA-election (structural), APTC (structural,
documented), 1099-R/SSA (structural, documented). The three leaks found are C1/I1/I2 above.

**Frozen-engine seam:** no phase quietly needs a frozen edit. All new modules call frozen primitives
(`net_1222`, `preferential_tax`, `ordinary_tax_on`, `se.rs`, `NIIT_RATE`, thresholds) through public
surface; content-pin ≠ call-ban; `TaxProfile.schedule_c_expenses`/`w2_*` fields already exist for the
derivation; nothing needs `compute.rs` internals (`nii_with` correctly rebuilt, not read). The
I4-plan split (absolute with-crypto AGI vs frozen non-crypto scalars) is the correct reading of
`types.rs:34-38`. The delta/absolute two-number story is consistently told in §5-tail, §6, P4 t8,
and KAT-5/5b.

**Form layer:** §7.2's four-path Sch D routing is exhaustive over sign(L16)×sign(L15)×QD; the
tax number never depends on the printed L17–L22 branch (routing is presentational; L21 = the §1211
figure already computed) so P6 placement is safe. Form-set closure (8995/8959/8960/C fillers + the
schedule_d.rs:5-6 scope-out removal) is fully planned. Per-(form,year) maps + root/on-state hazards
match F5's extraction; Sch B 14/15 overflow reuses the shipped continuation path.

**Scope coherence:** a real crypto W-2 household inside the documented envelope (multi-W-2, INT/DIV
incl. QD/2a/box-5/foreign-tax≤cap, unemployment, crypto trades + hobby + business mining w/ SE,
donations, itemized-vs-std, student loan, estimated payments, excess-SS, refund/owed) files
end-to-end with no dangling dependency. The documented exclusions (CTC advisory-omit, retirement/SSA,
non-crypto 1099-B — see M8, HSA, state) refuse or conservatively omit rather than mis-file.

---

## 6. Disposition

| # | Finding | Rank | Fix site |
|---|---|---|---|
| C1 | Form 8615 kiddie-tax silent understatement | **Critical** | SPEC §4.10 row + §8 threshold + KAT; PLAN P1/P3 |
| I1 | Box-12 blocklist → allowlist (K/R/T leak) | Important | SPEC §4.10; PLAN P1 t4 |
| I2 | Schedule C loss unhandled (at-risk box, negative L3) | Important | SPEC §4.4a/§4.10 row + KAT; PLAN P1/P2 |
| I3 | `qbi_deduction_override` orphaned/undefined | Important | SPEC §4.5/§7.1 (drop or define); PLAN P4 |
| M1–M11 | as listed | Minor/Obs | FOLLOWUPS.md |

All four blocking items are input-surface/refuse-guard edits — no change to the architecture (§2),
the computation pipeline (§5), the frozen seam, the form plan (§7), or the phase structure (§11).
After folding C1 + I1–I3 (a spec r5 + a one-line plan touch, re-reviewed per the workflow),
**the design is sound to begin implementation.**
