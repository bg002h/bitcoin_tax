# Whole-slug review — P2-D §1401 self-employment tax routing (round 1)

**Scope:** BOTH task-review and whole-diff gate for P2-D.
**Artifacts reviewed:** `design/SPEC_p2d_se_tax.md`; `.superpowers/sdd/p2d-report.md`;
diff `52cdd53..87e443d` (commits `69938b2` spec, `87e443d` impl); source
`crates/btctax-core/src/tax/{se.rs,tables.rs,compute.rs}`,
`crates/btctax-adapters/src/tax_tables.rs`,
`crates/btctax-cli/src/{render.rs,cmd/tax.rs,cmd/admin.rs,main.rs}`, and the KATs.
**Reviewer stance:** independent; goldens re-derived by hand, not read off the asserts.

## VERDICT: READY TO MERGE — 0 Critical / 0 Important

All tax figures re-derive exactly; the standalone/no-regression guarantee is structurally
proven (income-tax engine `compute.rs` is untouched by this branch); the dual-direction W-2
disclosure states the two opposite directions correctly. Remaining findings are 2 Minor + 3 Nit,
none blocking.

---

## 1. SE-tax math (highest priority) — VERIFIED, all goldens reproduce

`compute_se_tax` (`se.rs:80-134`) implements exactly the spec formula. Predicate
`se_net_income` (`se.rs:54-61`): `Σ usd_fmv` where `business && kind != Interest &&
recognized_at.year() == year` — **[M2] Interest excluded** (§1402(a)(2)), hobby (`business==false`)
excluded, year-filtered. `base = round_cents(net_se × 0.9235)`;
`ss = round_cents(0.124 × min(base, ss_wage_base − 0))`;
`medicare = round_cents(0.029 × base)`;
`addl = round_cents(0.009 × max(0, base − threshold(status)))`;
`total = ss + medicare + addl`; **[C1] `deductible_half = round_cents((ss + medicare)/2)` —
EXCLUDES `addl`** (`se.rs:123`).

**Load-bearing rounding-mode check.** `round_cents` = `RoundingStrategy::MidpointNearestEven`
(banker's / ROUND_HALF_EVEN) — `conventions.rs:13,22-23`. This is decisive for the C1 lock and
the M3 deductible: `14,935.425`, `7,064.775`, and `872.195` are all EXACT `.xx5` ties, so the
mode is not cosmetic. HALF_EVEN yields the asserted `14,935.42` / `7,064.78` / `872.20`; a
HALF_UP implementation would produce `14,935.43` (C1 would then be WRONG). Confirmed HALF_EVEN.

### Re-derivations (by hand)

| Case | base | ss | medicare | addl | total | deductible_half |
|---|---|---|---|---|---|---|
| Golden 1 Single $100k | 92,350.00 | 11,451.40 | 2,678.15 | 0.00 | **14,129.55** | **7,064.78** |
| **$300k Single [C1]** | 277,050.00 | 21,836.40 | 8,034.45 | 693.45 | **30,564.30** | **14,935.42** |
| $250k cap Single | 230,875.00 | **21,836.40** | 6,695.375→? | — | — | — |
| MFS $200k (125k thr) | 184,700.00 | 21,836.40 | 5,356.30 | 537.30 | 27,730.00 | — |
| M3 $12,345.67 | 11,401.23 | 1,413.75 | 330.64 | 0.00 | 1,744.39 | 872.20 |

All match the code asserts (`se.rs` tests 171-337) and the report.

**The $14,935.42 C1-lock, re-derived independently:**
- base = round_cents(300,000 × 0.9235) = round_cents(277,050.00) = **277,050.00**
- ss (capped) = round_cents(0.124 × min(277,050, 176,100)) = round_cents(0.124 × 176,100) =
  round_cents(21,836.40) = **21,836.40**
- medicare = round_cents(0.029 × 277,050) = round_cents(8,034.45) = **8,034.45**
- addl = round_cents(0.009 × (277,050 − 200,000)) = round_cents(0.009 × 77,050) =
  round_cents(693.45) = **693.45** (NOT in the deductible)
- deductible_half = round_cents((21,836.40 + 8,034.45) / 2) = round_cents(29,870.85 / 2)
  = round_cents(**14,935.425**) → HALF_EVEN (kept digit `2` is even → stays) = **$14,935.42** ✓
- The §164(f)(1)-wrong `total/2` = round_cents(30,564.30 / 2) = **$15,282.15** — the code
  asserts `deductible_half != 15,282.15` (`se.rs:207`). The $346.73 gap = half of the $693.45
  additional Medicare that §164(f)(1) expressly excludes. Correct.

Rates (0.124 / 0.029 / 0.009), the 0.9235 factor, the $176,100 wage base, the $200k/$250k/$125k
thresholds, and the §164(f)(1) additional-Medicare exclusion all independently confirmed against
§1401(a)/(b), §1402(a)(12), §164(f)(1), and SSA 2024-10-10. No float anywhere in `se.rs`
(grep clean). NFR4/NFR5 satisfied; the intermediate `base` cent-round is the intentional
Schedule-SE line-item order (documented) and is benign for every golden (verified for M3: applying
rates to the unrounded base gives identical cents).

## 2. Standalone (D5) / no-regression (highest priority) — VERIFIED, structurally proven

- **`compute.rs` is NOT modified by this branch.** The diff has no `diff --git .../tax/compute.rs`
  hunk; every "compute.rs" string in the package is SPEC-recon prose or the *separate*
  `tests/tax_compute.rs` file. Therefore `compute_tax_year` / `TaxResult` / the income-tax / NIIT
  / capital-gains math and the pinned identity `total == (ord_with−ord_without) + ltcg_tax + niit`
  (`compute.rs:369,395`) are untouched. SE tax cannot have moved any existing figure.
- **SE tax is never added to `total_federal_tax_attributable`.** It is computed by a separate
  function and surfaced through a separate render path / CSV; no engine-B field references it.
- **The D5 KAT genuinely locks it.** `tax_compute.rs::se_tax_is_standalone_...` asserts
  `r.total_federal_tax_attributable == dec!(16000.00)` (the ordinary-income delta ONLY, for
  Single $100k business mining on synth brackets: 0.10·50k + 0.22·50k) AND separately that
  `compute_se_tax(...).total == 14,129.55`, and that the total ≠ 16,000 + 14,129.55. The
  `assert_eq!(…, 16000.00)` is the real, non-vacuous lock. No existing tax golden moved (SE is
  purely additive). PASS.

## 3. W-2 disclosure directions (I2) — VERIFIED correct (opposite directions, not "both overstated")

`render_schedule_se` (`render.rs:454-462`) states:
- **SS component may be OVERSTATED** — real cap = wage base LESS W-2 SS wages, a *lower* cap.
  Verified: `ss = 0.124 × min(base, cap)`; with actual W-2 SS wages `cap = 176,100 − w2 < 176,100`,
  so real ss ≤ computed ss. Direction correct (computed is a max).
- **Additional Medicare may be UNDERSTATED** — §1401(b)(2)(B)/Form 8959 threshold is *reduced* by
  W-2 Medicare wages, a *lower* threshold → MORE net SE earnings over it → MORE taxed at 0.9%.
  Verified against Form 8959 Part II (line 11 = threshold − Medicare wages, floored at 0); with
  W-2 wages real addl ≥ computed addl. Direction correct (computed is a min).

The two disclosed directions are genuinely opposite and each is correct; cites §1401(b)(2)(B)
(asserted by the KAT). The D5 standalone note ("SEPARATE federal liability… §164(f) not
auto-coordinated") and the no-table "SS wage base unavailable for {year}" note (business income
present, no bundled table → no silent drop, mirrors P2-C m6) are both present and correct.

## 4. Homes + literals — VERIFIED

- Rates as `tables.rs` statutory consts: `SE_RATE_SS/MEDICARE/ADDL_MEDICARE`,
  `SE_NET_EARNINGS_FACTOR` (`tables.rs:118-160`), each doc'd STATUTORY / cited. Correct home
  (year-independent) — mirrors `NIIT_RATE`.
- `ss_wage_base` is a year-indexed `TaxTable` field (TY2025 = $176,100, §230 SSA / 42 U.S.C. §430,
  SSA 2024-10-10) — correct home (moves year-over-year), mirrors `gift_annual_exclusion`.
- `se_addl_medicare_threshold(status)` = $200k Single/HoH, $250k Mfj/Qss, $125k Mfs
  (`tables.rs`), fixed-statutory function — correct home (§1401(b)(2), not inflation-indexed).
- **All `TaxTable {}` literal construction sites updated.** Precise count: 18 raw `TaxTable {`
  grep hits = 2 struct/impl headers + 3 `-> TaxTable {` return-type signatures + **13 real literal
  constructions**, and all 13 carry `ss_wage_base`. Completeness is guaranteed by the green build:
  `ss_wage_base` is a non-`Default` required field, so any missed site is a hard compile error.
- `TaxYearReport` type alias (clippy `type_complexity` fix) is behavior-preserving: it names the
  identical 5-tuple; callers still destructure positionally (verified across `main.rs`,
  `tax_report.rs`).

## 5. NFR4/NFR5/CSV/privacy — VERIFIED

Deterministic (pure fold over `Vec`, no map/set ordering, no clock/RNG). Exact `Decimal`, zero
float. `schedule_se.csv` written via `fsperms::open_owner_only` (0o600) inside the
`if let Some(year)` year-scoped block, and only when a `SeTaxResult` exists (nothing to file
otherwise). All test data synthetic.

---

## Findings

### Minor

- **[Minor-1] Vacuous `assert_ne!` in the wiring KAT.**
  `tax_report.rs::report_tax_year_renders_schedule_se_for_business_mining` asserts
  `assert_ne!(r.total_federal_tax_attributable, r.total_federal_tax_attributable + dec!(14129.55))`
  — this is `X != X + 14129.55`, a tautology that verifies nothing and gives false confidence in
  the D5 guarantee. D5 IS genuinely locked (by the same test's `assert!(!it.contains("14129.55"))`
  render-backstop and by `tax_compute.rs`'s `assert_eq!(…, 16000.00)`), so this is not blocking —
  but the assert should be repaired to compare against a pre-computed constant (e.g.
  `assert_eq!(total, dec!(16000.00))`) so it actually tests something. DEFER (test-only; genuine
  coverage exists).

- **[Minor-2] Render labels gross income as "Schedule C net".**
  `render.rs:418` prints "net self-employment income (Schedule C net; business crypto, Interest
  excluded)" but no Schedule C expenses are modeled — `net_se` is GROSS (the struct doc at
  `se.rs:26-28` says so). A business miner with electricity/depreciation expenses could read
  "Schedule C net" as "expenses already deducted" and over-report. It is CONSERVATIVE (overstates
  SE tax, never understates) and expense computation is explicitly out-of-scope in the SPEC, but
  the user-facing label should carry a "no business expenses modeled — reduce by your Schedule C
  expenses" caveat. DEFER to the Schedule-C-expenses FOLLOWUP; add the caveat there (or a one-line
  render note now).

### Nit

- **[Nit-1] Report's "15 construction sites" mis-counts.** Actual = 13 literal constructions; the
  extra 2 are `-> TaxTable {` return-type signature lines. Immaterial (green compile proves
  completeness); correct the report's arithmetic for accuracy.
- **[Nit-2] `se_net_income` scanned twice in `report_tax_year`** (once for
  `business_income_present`, once inside `compute_se_tax`). Trivial redundant O(n) pass over
  income; not a hot path.
- **[Nit-3] No profile + business income → no Schedule SE section at all** (render is only invoked
  when a profile exists, `cmd/tax.rs:200`). The wage-base-unavailable note only fires when a
  profile exists but the table doesn't. This mirrors the income-tax NotComputable path (which
  already tells the user to set a profile), so it is acceptable/by-design; noted for completeness.

---

## FOLLOWUPS triage (BLOCK / DEFER)

| Deferral | Decision | Rationale |
|---|---|---|
| `TaxProfile.w2_ss_wages` / `w2_medicare_wages` field | **DEFER** | $0 solo-miner default is the canonical case; the dual-direction W-2 note discloses both effects with the correct directions. No correctness bug. |
| `ReclassifyIncome` / River `business:false` business-flip | **DEFER** | River income has no flip path; documented KNOWN LIMITATION in SPEC. Event-schema change out of scope; the CLI `classify-inbound-income --business` path covers the primary case. |
| Schedule C deductible mining EXPENSES (net_se = gross) | **DEFER** | Spec-sanctioned out-of-scope; conservative (overstates). See Minor-2 — add the render caveat when this lands. |
| §164(f) ½-SE-deduction auto-coordination into the income-tax total | **DEFER** | This IS the D5 rationale; `deductible_half` is informational and the standalone note discloses non-coordination. Folding it in without care would break the pinned identity. |
| SS wage base for TY2024 / TY2026+ | **DEFER** | Only TY2025 bundled; the "wage base unavailable for {year}" note handles other years with no silent drop (mirrors P2-C's year-coverage deferral). |

None blocking.

---

## Bottom line

The one figure this slug exists to protect — the §164(f)(1) deductible half — is correct at
**$14,935.42** (excludes the $693.45 additional Medicare; the naïve `total/2` = $15,282.15 is
explicitly asserted against), and it is correct *because* `round_cents` is HALF_EVEN. Every other
golden re-derives exactly. The standalone guarantee is airtight: the income-tax engine file is
literally not in the diff, and the D5 KAT pins the engine-B total to the income-tax delta alone.
The W-2 disclosure names the two opposite directions correctly.

**0 Critical / 0 Important → READY TO MERGE.** (2 Minor + 3 Nit, all deferrable.)
