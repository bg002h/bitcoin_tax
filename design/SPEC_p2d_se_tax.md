# SPEC — P2-D: Self-employment tax routing (business mining → Schedule SE)

**Source baseline:** `origin/main` @ `52cdd53` (post P2-C).
**Goal:** Compute + surface **§1401 self-employment tax** on business crypto income (mined crypto in a
trade/business = SE income, Notice 2014-21 A-9), reported as a standalone Schedule SE figure alongside the
income-tax/NIIT report. Federal-only; standalone (does NOT fold into `total_federal_tax_attributable` —
see D5).

**SemVer:** additive `TaxTable.ss_wage_base` field + SE constants + `compute_se_tax` + new render/CSV ⇒
**MINOR** (pre-1.0). No change to the existing income-tax / NIIT / capital-gains figures.

## Legal grounding
- **Notice 2014-21 A-9:** crypto mined in a **trade or business** (not as a hobby) is self-employment
  income → Schedule C net income → Schedule SE.
- **§1401(a)/(b) + §1402(a):** SE tax = **12.4% Social Security** (on net SE earnings up to the SS wage
  base, less W-2 SS wages) + **2.9% Medicare** (uncapped) + **0.9% Additional Medicare** (§1401(b)(2),
  on net SE earnings over $200k Single/HoH, $250k MFJ, $125k MFS). Net SE earnings = Schedule C net ×
  **92.35%** (§1402(a), the 1 − 7.65% factor).
- **SS wage base (§230 SSA, inflation/wage-indexed, year-dependent):** TY2025 = **$176,100** (SSA
  2024-10-10). Belongs in the year-indexed `TaxTable` (like P2-C's `gift_annual_exclusion`), NOT a fixed
  constant.
- **§164(f):** one-half of SE tax is an above-the-line deduction (reduces AGI) — the coordination reason
  SE tax is reported standalone here (see D5).

## Current-state (recon @ 52cdd53)
- `IncomeRecord{event, recognized_at, sat, usd_fmv, kind, business}` in `state.income_recognized`
  (`state.rs:178-219`); `IncomeKind{Mining,Staking,Interest,Airdrop,Reward}` + `business: bool`. `business`
  is set: hard-coded `false` in the River adapter (`river.rs:148-179` — immutable post-ingest), or `true`
  via CLI `reconcile classify-inbound-income --business` / `ClassifyRaw`. Business-mining SE predicate:
  `business == true` (primarily `kind==Mining`).
- `compute_tax_year` sums ALL income into `crypto_ord` (`compute.rs:297-302`) undifferentiated by
  `business`; **SE tax is computed NOWHERE** (no SE constants/wage-base anywhere). `crypto_ord` is
  correctly excluded from NII (B-M1) — consistent with SE income.
- `TaxProfile` (`types.rs:31-50`) has NO W-2 / SS-wages field → the SS-cap coordination can't subtract
  W-2 SS wages (see D4).
- `TaxTable.gift_annual_exclusion` (P2-C, `tables.rs`) is the year-indexed-field precedent; statutory
  consts (`NIIT_RATE`) + `niit_threshold(status)` are the fixed-statutory precedent.
- Output pattern: year-scoped CSVs + `render_tax_outcome`/`render_schedule_d` text (P2-B/P2-C).

## Design

### D1 — SE constants + year-indexed SS wage base
- `tables.rs` constants (statutory, §1401/§1402): `SE_RATE_SS = dec!(0.124)`, `SE_RATE_MEDICARE =
  dec!(0.029)`, `SE_RATE_ADDL_MEDICARE = dec!(0.009)`, `SE_NET_EARNINGS_FACTOR = dec!(0.9235)`; +
  `fn se_addl_medicare_threshold(status) -> Usd` ($200k Single/HoH, $250k MFJ/Qss, $125k MFS).
- `TaxTable.ss_wage_base: Usd` (year-indexed; TY2025 = `dec!(176100)`, SSA 2024-10-10) — add the field +
  update `synthetic_table` + `BundledTaxTables::ty2025()`.

### D2 — `compute_se_tax`
`compute_se_tax(state, year, status, table) -> Option<SeTaxResult>`:
- `net_se = Σ usd_fmv` over `income_recognized` where `business == true && kind != IncomeKind::Interest
  && recognized_at.year() == year`. **[R0-M2] Exclude `Interest`** — §1402(a)(2) excludes interest from
  net SE earnings (it's investment income, consistent with B-M1 treating crypto-lending interest as NII,
  not SE); Mining/Staking/Airdrop/Reward with `business==true` are included. (Filter on `business`, the
  user's SE assertion; primarily Mining. If `net_se == 0` → `None`.)
- `base = round_cents(net_se * SE_NET_EARNINGS_FACTOR)` (×92.35%).
- `ss = round_cents(SE_RATE_SS * min(base, max(0, ss_wage_base - w2_ss_wages)))` with **`w2_ss_wages = 0`**
  (D4 — no profile field; solo-miner default).
- `medicare = round_cents(SE_RATE_MEDICARE * base)`.
- `addl = round_cents(SE_RATE_ADDL_MEDICARE * max(0, base - se_addl_medicare_threshold(status)))`.
- `total = ss + medicare + addl`; **[R0-C1] `deductible_half = round_cents((ss + medicare) / 2)`** —
  §164(f)(1) EXPRESSLY EXCLUDES the §1401(b)(2) Additional Medicare Tax from the ½-SE-tax deduction (the
  0.9% is a Form 8959 item, Schedule SE line 13 = SS + regular Medicare only). Do NOT include `addl` in
  the deductible half (§164(f), informational).
- `SeTaxResult { net_se, base, ss, medicare, addl, total, deductible_half }`. Return `None` if the year
  has no bundled table (ss_wage_base unavailable) — but if there IS business income and no table, the
  caller emits a "table unavailable" note (mirror P2-C's m6, no silent drop).
- Deterministic; exact Decimal; end-only `round_cents`.

### D3 — render + CSV + wiring
- `render_schedule_se(year, Option<&SeTaxResult>, table_available) -> Option<String>`: show net SE
  earnings, the 92.35% base, SS/Medicare/Additional-Medicare components, total SE tax, and the §164(f)
  deductible half; + the D4 W-2 disclosure + the D5 standalone note. `None` when no business income.
  Table-missing-with-business-income → a "SS wage base unavailable for {year}" note.
- `schedule_se.csv` (0o600, year-scoped) in `write_csv_exports`'s `if let Some(year)` block: columns
  `net_se_earnings, se_base_9235, ss_component, medicare_component, additional_medicare_component,
  total_se_tax, deductible_half`.
- Wire into `report_tax_year` (`cmd/tax.rs`) + `main.rs` (the `--tax-year` path), alongside
  `render_schedule_d`.

### D4 — W-2 SS-wage coordination (default $0 + disclose)
`TaxProfile` has no W-2 wages field, so the computation assumes **w2 wages = $0** (correct for a solo
miner with no wage job — the canonical case). **[R0-I1/I2] The $0 assumption affects TWO things that move
in OPPOSITE directions — disclose both, with the correct direction** in `render_schedule_se`: "assumes $0
W-2 wages. If you had a wage job: (1) the 12.4% Social Security component here may be **OVERSTATED** — its
cap is the wage base LESS your W-2 Social-Security wages (a lower cap → less SS); AND (2) the 0.9%
Additional-Medicare component here may be **UNDERSTATED** — the §1401(b)(2)(B)/Form 8959 threshold is
REDUCED by your W-2 Medicare wages (a lower threshold → MORE income taxed at 0.9%). Adjust each
accordingly." A `TaxProfile.w2_ss_wages`/`w2_medicare_wages` field is a deferred FOLLOWUP.

### D5 — standalone (do NOT fold into `total_federal_tax_attributable`)
SE tax is reported as a SEPARATE §1401 figure, NOT added to `TaxResult.total_federal_tax_attributable`.
Reasons: (a) §164(f) ½-SE deduction reduces AGI → folding SE in without coordinating the deduction would
misstate the income-tax portion (circular; `ordinary_taxable_income` is user-supplied post-deduction);
(b) it would break the pinned identity `total == ord_delta + ltcg_tax + niit`; (c) precedent — every
Phase-2 form/figure (§170 deduction, 8283, 709) is standalone. `render_schedule_se` states: "This §1401
SE tax is a SEPARATE federal liability, not included in the income-tax + NIIT total above; the §164(f)
one-half-SE-tax deduction is not auto-coordinated into that total."

### Decisions
- **SE tax COMPUTED** (feasible, decimal-exact) but **standalone-reported** (D5).
- **Filter on `business == true`** (the user's SE assertion, primarily Mining) — kind-agnostic; disclose
  that non-mining business income (business staking/reward) SE-status is less settled — verify.
- **River `business:false` immutability = KNOWN LIMITATION (deferred).** SE routing computes for income
  with `business == true`; River `Income{Reward}` is hard-coded `business:false` and has no flip path — a
  River business-miner must re-import with a patched adapter OR await a new `ReclassifyIncome`/flip
  decision (deferred FOLLOWUP; event-schema change out of scope here).

## Plan (TDD)

### Task 1 — SE constants + `ss_wage_base` + `SeTaxResult` + `compute_se_tax` + goldens
- **Files:** `crates/btctax-core/src/tax/tables.rs` (consts + `se_addl_medicare_threshold` + `ss_wage_base`
  field + `synthetic_table`), `crates/btctax-adapters/src/tax_tables.rs` (TY2025 `ss_wage_base` $176,100),
  `crates/btctax-core/src/tax/compute.rs` or a new `se.rs` (SeTaxResult + `compute_se_tax`). **[R0-M1]
  adding the non-`Default` `ss_wage_base` field breaks EVERY `TaxTable { .. }` literal — grep
  `TaxTable {` across the workspace (~12 sites: ~11 test files + `render.rs`) and update ALL, not just
  the 2 named here.**
- Hand-verified golden KATs (re-derive by hand): **Golden 1 — Single, business mining $100,000, no W-2:**
  base = 100,000 × 0.9235 = $92,350; ss = 12.4% × min(92,350, 176,100) = $11,451.40; medicare = 2.9% ×
  92,350 = $2,678.15; addl = 0.9% × max(0, 92,350 − 200,000) = $0; total = **$14,129.55**;
  `deductible_half = round_cents((ss+medicare)/2)` = $7,064.78 (addl=0 here). **Wage-base cap:** $250,000
  → base $230,875 > $176,100 → ss = 12.4% × 176,100 = $21,836.40. **[R0-C1] Additional-Medicare golden —
  $300,000 Single:** base $277,050 → ss $21,836.40 + medicare $8,034.45 + addl 0.9%×(277,050−200,000) =
  $693.45 → total **$30,564.30**; `deductible_half = (21,836.40 + 8,034.45)/2 = **$14,935.42**` (EXCLUDES
  the $693.45 addl — this is the C1 lock; the wrong (total/2) would give $15,282.15). **MFS threshold**
  $125k. **[R0-M2] a business `Interest` income → NOT included in net_se** (excluded per §1402(a)(2)); a
  business `Mining` income IS. **[R0-M3] a fractional-base case** (e.g. mining $12,345.67) to genuinely
  exercise `round_cents`. **No business income → `None`**; **hobby (business=false) mining excluded**.
  Independently confirm the rates + $176,100 + 92.35% + the §164(f)(1) addl-exclusion.

### Task 2 — render_schedule_se + schedule_se.csv + wiring
- **Files:** `crates/btctax-cli/src/render.rs`, `crates/btctax-cli/src/cmd/tax.rs` (report_tax_year), `crates/btctax-cli/src/main.rs`.
- KATs: business-mining year → the Schedule SE text shows the components + total + deductible half + the
  W-2 disclosure (D4) + the standalone note (D5); no-business-income year → no SE section; the
  schedule_se.csv columns + values; **standalone: `TaxResult.total_federal_tax_attributable` is UNCHANGED
  by SE tax** (assert SE tax is NOT added to it); a year with business income but no bundled table → the
  "wage base unavailable" note (no silent drop).

### Task 3 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: SE-tax math hand-verified + web-confirmed; STANDALONE (income-tax/NIIT/capital-gains
  figures + the `total == ord_delta + ltcg_tax + niit` identity UNCHANGED); business filter correct
  (hobby excluded); W-2 $0 assumption disclosed; determinism; exact Decimal; CSV 0o600; privacy.
- FOLLOWUPS: `TaxProfile.w2_ss_wages` field (SS-cap coordination for employed miners); a
  `ReclassifyIncome`/business-flip decision (River `business:false` immutability); Schedule C deductible
  mining EXPENSES (net SE earnings currently = gross income, no expense deduction modeled); §164(f)
  ½-SE-deduction auto-coordination into the income-tax total; SS wage base for TY2024/2026+.

## Out of scope
- Folding SE tax into `total_federal_tax_attributable` / §164(f) auto-coordination; W-2 SS-wages profile
  field; Schedule C expense deductions (net SE = gross mining income); a business-flip decision for River
  income; filled-PDF Schedule SE; 2026/2027 tables; the church-employee/optional-method §1402 wrinkles.
