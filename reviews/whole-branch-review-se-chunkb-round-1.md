# Whole-branch review — SE Chunk B: Schedule C expenses (advisory-only) — round 1

**Diff:** `60f33c0..5af71ee` (2 commits: spec R0-GREEN + Task-1 implementation), branch `feat/se-chunkb-expenses`.
**Spec:** `design/SPEC_se_chunkB_expenses.md` (R0 GREEN at round 2, `reviews/R0-spec-se-chunkB-round-1.md`).
**Reviewer:** independent whole-diff reviewer (final gate), 2026-07-01.
**Gate state accepted as given:** 678 tests green, clippy/fmt clean, PII clean — not re-run.
**Working tree verified == `5af71ee`** (`git diff 5af71ee` empty); all excerpts below re-checked against
the CURRENT source, not just the diff package.

**Verdict: NOT READY TO MERGE — 0 Critical / 2 Important / 2 Minor / 1 Nit.**
All *behavioral* surfaces verified correct (math, three-way None split, advisory mechanism, engine-B
untouched, regression-by-inspection). Both Importants are **spec-mandated Task-1 tests that were never
written** — the implementation report's "All spec requirements … are implemented" claim is inaccurate on
exactly these two bullets.

---

## 1. The math — re-derived by hand, all goldens confirmed exact

`compute_se_tax` (`crates/btctax-core/src/tax/se.rs:99-121`): `gross_se = se_net_income(state, year)`
(unchanged helper); `net_se = max(0, gross_se − schedule_c_expenses)` via an explicit `n <= ZERO → ZERO`
clamp; `net_se.is_zero() → None`; **then** `base = round_cents(net_se × 0.9235)`. The subtraction is
BEFORE the ×0.9235 factor — the statutory/Schedule-SE order (§1402(a) net, then the §1402(a)(12) deemed
deduction). All downstream logic (SS cap, addl threshold, deductible half) operates on the expensed base,
untouched from Chunk A.

**(a) Headline — mining $100,000, expenses $20,000, no W-2** (`se.rs` KAT `chunkb_headline_expenses_20k_no_w2`):
- net_se = 100,000 − 20,000 = **80,000** ✓
- base = 80,000 × 0.9235 = **73,880.00** (exact) ✓
- ss = 0.124 × min(73,880, 176,100) = 0.124 × 73,880 = **9,161.12** (73,880 × 124/1000 = 9,161,120/1000, exact) ✓
- medicare = 0.029 × 73,880 = **2,142.52** (73,880 × 29/1000 = 2,142,520/1000, exact) ✓
- addl = 0.009 × max(0, 73,880 − 200,000) = **0.00** ✓
- total = 9,161.12 + 2,142.52 = **11,303.64** ✓
- deductible_half = 11,303.64 / 2 = **5,651.82** (exact, no tie) ✓
- Matches every assertion in the KAT and the spec figures.

**(b) Fully expensed — mining $10,000, expenses $15,000** (`chunkb_fully_expensed_mining_10k_expenses_15k_is_none`):
10,000 − 15,000 = −5,000 → clamped to 0 → **`None`** ✓. The companion floor KAT
(`chunkb_expenses_equal_to_gross_is_none`, expenses == gross $50,000) pins the boundary → `None` ✓.

**(c) Composed — expenses $20,000 + w2_ss $150,000 + w2_medicare $150,000** (`chunkb_expenses_w2_combined`):
- net_se = **80,000**; base = **73,880.00** ✓ (the W-2 caps apply to the EXPENSED base — correct
  Schedule SE ordering: line 3 net → line 4a ×92.35% → line 8a/9 cap reduction)
- ss_cap = max(0, 176,100 − 150,000) = 26,100; ss = 0.124 × min(73,880, 26,100) = 0.124 × 26,100 =
  **3,236.40** (26,100 × 124 = 3,236,400, exact) ✓
- medicare = **2,142.52** ✓
- addl_threshold = max(0, 200,000 − 150,000) = 50,000; over = 73,880 − 50,000 = 23,880;
  addl = 0.009 × 23,880 = **214.92** (23,880 × 9 = 214,920, exact) ✓
- total = 3,236.40 + 2,142.52 + 214.92 = **5,593.84** ✓
- deductible_half = (3,236.40 + 2,142.52)/2 = 5,378.92/2 = **2,689.46** — **EXCLUDES addl**; the
  §164(f)(1) C1 rule survives composition (source: `deductible_half = round_cents((ss + medicare) / 2)`,
  `se.rs:162`, unchanged) ✓

No golden depends on a rounding tie; `round_cents` HALF_EVEN cannot shift any figure. All exact Decimal;
no float anywhere in the new code (render uses `fmt_money` = `format!("{d:.2}")` on `Decimal`; TUI uses
`{:.2}` on `Decimal`). Deterministic (pure functions of inputs).

## 2. Three-way None split [R0-I1] — CORRECT; the false wage-base note is unreachable for a fully-expensed year

`render_schedule_se` (`render.rs:1118-1273`) None-arm at HEAD:

1. `gross_se.is_zero()` → `None` (no section) — unchanged behavior;
2. `else if !table_present` → the "SS wage base unavailable for {year} … NOT computed (no silent drop)"
   note — unchanged text;
3. `else` (gross > 0, table present, result `None` ⇒ net_se == 0) → the NEW line:
   `fully expensed: gross {gross_se} − Schedule C expenses {expenses} ≤ $0 → no §1401 SE tax for {year}.`
   — liability framed as "no tax owed", NOT "couldn't compute", exactly the spec's D3 text.

**Data path confirmed:** `render_schedule_se` has exactly ONE production caller — `cmd/tax.rs:95`. The
caller computes `gross_se = se_net_income(&state, year)` (`cmd/tax.rs:81`), `table_present =
tables.table_for(year).is_some()` (`:83`), and passes `p.schedule_c_expenses` (`:92, :100`) — the R0-M4
carrier exactly as specified. `cmd/admin.rs` computes the CSV figure only (never renders); the TUI has its
own condensed block (silently omits on `None` — accepted under the Chunk-A N-1 deferral per spec).

**Why the false note is impossible for a fully-expensed year:** the wage-base note is emitted in exactly
one place — branch (2), gated on `!table_present`. `table_present` is derived at the sole caller from
`table_for(year).is_some()` *before* the `and_then` that gates `compute_se_tax`. A fully-expensed year by
definition has a bundled table on the compute path (otherwise `compute_se_tax` is never called and the
year lands in branch (2), which is then *literally true* — table genuinely absent, "NOT computed"
accurate; the spec defines this overlap edge as case 2, deterministic). With a table present,
`table_present == true` ⇒ branch (2) unreachable ⇒ only the fully-expensed line can render. The render
golden `fully_expensed_shows_new_line_not_wage_base_note` (`render.rs:2853+`) pins BOTH the positive
assertion (line present, gross `10000.00`, expenses `15000.00`, "no §1401 SE tax", year) AND the negative
assertion (`!contains("SS wage base unavailable")`) — the regression is unimplementable-past.

The `Some`-arm breakout reconstruction `gross_display = r.net_se + schedule_c_expenses` is exact
(`Some` ⇒ no clamp fired). Confirmed rendered figures for goldens (a)/(c) in the render KATs
(`expenses_20k_no_w2_renders_breakout_and_advisory`, `expenses_w2_combined_renders_both`): gross
`100000.00`, expenses `20000.00`, net `80000.00`, plus all five component figures — match §1.

## 3. Advisory mechanism [I3] — CORRECT; no OTI prescription

The expenses advisory (`render.rs:1146-1157`) matches the spec's D3 text: quantifies the first-order
overstatement as "your marginal ordinary rate applied to {expenses}" (with the actual dollar amount
substituted), states the profile cannot express it, defers the engine-side coordination, "coordinate it on
your actual return". The only mention of an `ordinary_taxable_income` edit is the anti-prescription
rationale ("would shift both legs of the crypto-attributable delta") — the correct mechanism per
`compute.rs:335-338` (crypto_ord enters the WITH leg only). Grep across all crates for
"reduce your ordinary" / "set --ordinary" finds only the two NEGATIVE test assertions (`render.rs:2705,
2709`). The old "not modeled / your actual SE tax is lower" caveat is fully removed; the $0-expenses path
carries the new short note "(Schedule C) no Schedule C expenses supplied (--schedule-c-expenses)";
`!contains("not modeled")` is asserted at both the render-unit and CLI-integration levels.

## 4. Three-surface parity + regression + engine-B

- **All three call sites** pass the profile figure: `cmd/tax.rs:92`, `cmd/admin.rs:70`
  (`p.schedule_c_expenses`), `tui/tabs/tax.rs:95+104` (`profile.map(…).unwrap_or_default()` — $0 with no
  profile, per D2). Grep confirms **no production `compute_se_tax` call passes a literal ZERO** for
  expenses — every ZERO literal is in test code.
- **compute.rs untouched:** `git diff --stat 60f33c0..5af71ee -- crates/btctax-core/src/tax/compute.rs`
  is EMPTY. Engine B cannot read the field (it never sees it).
- **Regression:** I swept every test hunk in the diff — the only changes to pre-existing tests are
  (i) `schedule_c_expenses: dec!(0)` field additions, (ii) `Usd::ZERO`/`dec!(0)` seventh-arg additions,
  and (iii) the spec-mandated caveat-text assertion swap ("not modeled" → "no Schedule C expenses
  supplied"). **No numeric golden figure was edited anywhere** — all P2-D + Chunk-A figure-sets are
  byte-identical, plus the explicit `chunkb_regression_zero_expenses_byte_identical_to_golden1` KAT.
- **serde back-compat:** `optional_profile_fields_default_to_zero` (`types.rs`) extended — old/minimal
  JSON → `schedule_c_expenses == ZERO` ✓; round-trip KAT updated ✓.
- **CLI hygiene:** negative `--schedule-c-expenses` → `CliError::Usage` on the REAL path
  (`main.rs:751-760`, inside the non-`--show` set branch, mirroring the W-2 guards at `:738-749`);
  `--show` prints the field (`main.rs:670+`); help text matches D1. CSV-skip comment updated
  (`render.rs:339-344`) to name the fully-expensed reason [N4] ✓; `schedule_se.csv` keeps its shape with
  `net_se_earnings` = expensed net [M3] ✓.

## 5. Findings

### I1 (Important) — Spec-mandated engine-B invariance KAT was never written

Spec Task 1, golden list: "**Engine-B invariance:** `compute_tax_year` figures IDENTICAL with expenses 0
vs $20,000 (crypto_ord gross, untouched) — the advisory, not the engine, carries the difference." R0
round 1 §6 explicitly endorsed it as "an honest future-lock in the established pattern"
(`reclassify_income.rs:302` `engine_b_invariance_business_only_flip` is the Chunk-C exemplar;
`tax_compute.rs:252+` the P2-D one).

**The test does not exist.** A workspace-wide grep shows every `schedule_c_expenses` occurrence in every
test file is zero-valued — no test anywhere constructs a `TaxProfile` with nonzero expenses, and therefore
no test runs `compute_tax_year` under nonzero expenses at all. Today's behavior is provably correct
(`compute.rs` diff empty), so this is not Critical — but the lock against *future* accidental coupling is
exactly what the spec ordered and what this workflow's gates exist to enforce, and the implementation
report claims full spec compliance. **Fix:** one test (pattern: `reclassify_income.rs:302`) computing
`compute_tax_year` twice with profiles differing only in `schedule_c_expenses` (0 vs 20,000) and asserting
the full `TaxResult` figure-set identical.

### I2 (Important) — Spec-mandated expensed-profile parity guard absent; zero end-to-end coverage of expenses > 0

Spec Task 1, CLI/render bullet: "export/TUI parity (the asymmetric-style guard: **an expensed profile
renders the same figures in report + CSV**)." The Chunk-A analog exists
(`tax_report.rs:340+` `chunk_a_export_parity_asymmetric_w2`, `w2_ss_wages: dec!(150000)`); Chunk B has no
analog — the same grep shows **no integration test stores a nonzero-expenses profile** via `set_profile`
and drives `report_tax_year` / `export_snapshot`. Consequently the whole
`TaxProfile.schedule_c_expenses → compute_se_tax → render/CSV` wiring is pinned only at the unit level
(se.rs + render.rs KATs) plus inspection. Partial mitigations: the type system rules out Usd/bool
transpositions at the render call; a gross/expenses transposition at `cmd/tax.rs` would trip the existing
$0-note assertion in `report_tax_year_renders_schedule_se_for_business_mining`. Still, this is a
spec-mandated guard in the chunk's own named pattern ("the Chunk-A parity lesson + the asymmetric-fixture
guard"), absent. **Fix:** one CLI-path test with an expensed profile (e.g., mining $100k, expenses $20k)
asserting the report shows 9,161.12/2,142.52/11,303.64/5,651.82 and `schedule_se.csv` carries
`net_se_earnings = 80000` with the same components; ideally a fully-expensed sibling asserting the report
contains "fully expensed" (and not the wage-base note) and that `schedule_se.csv` is not written — which
also closes M2 below.

### M1 (Minor) — Negative `--schedule-c-expenses` rejection untested

The guard is correctly on the real path (`main.rs:756-760`), matching Chunk-A precedent (the W-2 negative
guards are also untested), so pattern-parity holds and this does not block. But the repo HAS an
established real-binary pattern for exactly this
(`tax_report.rs:690` `report_negative_prior_taxable_gifts_rejected_without_tax_year`, driving
`CARGO_BIN_EXE_btctax` and asserting exit 2). A one-test backfill covering all three negative profile
flags would close the family; acceptable as a FOLLOWUPS line.

### M2 (Minor) — Fully-expensed report path integration-untested (unit-level golden only)

The spec's "render-level golden" letter is satisfied (`fully_expensed_shows_new_line_not_wage_base_note`
carries both the positive and negative assertions), and §2 above verifies the caller wiring by inspection.
But no test drives a fully-expensed year through `report_tax_year` end-to-end. Fold into I2's fix (the
fully-expensed sibling case).

### N1 (Nit) — None-arm branch (3) trusts the caller contract

A contract-violating call (`result == None`, `gross_se > 0`, `table_present`, `expenses == 0` — impossible
from `compute_se_tax`) would render "Schedule C expenses 0.00 ≤ $0". Unreachable from the sole production
caller; the doc comment states the contract. Optional `debug_assert!(schedule_c_expenses >= gross_se)` in
the branch if desired; no action required.

## 6. Process note (non-blocking, required before ship)

Task 2's FOLLOWUPS updates are not yet in the diff (expected — they ride this review phase): SE cluster
COMPLETE (A + C + B); engine-B gross-vs-net `crypto_ord` coordination deferred; **the §6017 $400 filing
floor [R0-N1]**; the TUI fully-expensed silent-omit under the Chunk-A N-1 family; next queue item =
TY2024 tables. These must land (with the I1/I2 fixes) before the branch is finished.

## 7. Verdict

**0 Critical / 2 Important (I1, I2) / 2 Minor (M1, M2) / 1 Nit — NOT ready to merge.**

Everything the diff *does* is right: the §1402(a) ordering, the max(0,·) floor, all five compute goldens
re-derived exact, the W-2 caps composing against the expensed base, the deductible half still excluding
addl, the three-way None split with the false wage-base note structurally unreachable for a fully-expensed
year, the non-prescriptive advisory, engine B byte-untouched, and a genuinely clean regression surface.
What the diff *omits* is two tests its own spec ordered. Both fixes are small, additive, and cannot move
any behavioral surface; after they land (plus the Task-2 FOLLOWUPS lines), a light round-2 confirmation of
just the new tests suffices — no full re-review.
