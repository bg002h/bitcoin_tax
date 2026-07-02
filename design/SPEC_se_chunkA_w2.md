# SPEC — SE completion Chunk A: W-2 wage coordination (+ §164(f) advisory text)

**Source baseline:** `main` @ `33b7f26`. Queue item 2, chunk 1 of 3 (A = W-2; C = ReclassifyIncome;
B = Schedule C expenses advisory-only).
**Goal:** Replace P2-D's "$0 W-2 assumed" stub with REAL coordination: `TaxProfile.w2_ss_wages` +
`w2_medicare_wages` (user-supplied, per-year) → the SE **Social-Security cap** is reduced by W-2 SS wages
and the **Additional-Medicare threshold** is reduced by W-2 Medicare wages (§1401(b)(2)(B)). The
dual-direction disclosure becomes an accurate statement. Plus the §164(f) advisory micro-patch (quantified
first-order overstatement; NO profile-edit prescription — see D3/R0-I3). SE stays STANDALONE (engine B
untouched).

**SemVer:** additive `TaxProfile` fields (`#[serde(default)]`, proven back-compat) + a `compute_se_tax`
signature extension + new CLI flags ⇒ **MINOR** (pre-1.0).

## Legal grounding (R0 to web-verify)
- **§1402(b)(1):** self-employment income subject to the SS rate is capped at the wage base MINUS wages
  (W-2 Social-Security wages) received in the year — i.e. `ss_cap = max(0, ss_wage_base − w2_ss_wages)`
  (Schedule SE line 8a/8d coordination).
- **§1401(b)(2)(B):** the Additional-Medicare 0.9% threshold for SE income is REDUCED (not below zero) by
  the taxpayer's W-2 Medicare wages — `addl_threshold = max(0, threshold(status) − w2_medicare_wages)`
  (Form 8959 **Part II** coordination).
- **§164(f)(1):** ½ of SE tax (SS + regular Medicare only, NOT the §1401(b)(2) addl — the P2-D C1 rule,
  unchanged) is an above-the-line deduction — the advisory quantifies the first-order overstatement
  (not auto-coordinated; no profile edit prescribed — R0-I3).

## Current-state (recon @ 33b7f26)
- `se.rs:95`: `let w2_ss_wages = Usd::ZERO;` (the D4 stub — the ss-cap structure already exists);
  `se.rs:111-118`: `over = base − se_addl_medicare_threshold(status)` with NO W-2 reduction.
- `compute_se_tax(state, year, status, table)` (`se.rs:80`) — takes `status`+`table` only; called from
  `cmd/tax.rs:79-88` with `p.filing_status`.
- `TaxProfile` (`types.rs:31-50`): JSON side-table (`tax_profile.rs`); `#[serde(default)]` back-compat
  PROVEN (`other_net_capital_gain`/`carryforward` precedent + the `optional_profile_fields_default_to_zero`
  KAT at types.rs:129-136).
- The dual-direction disclosure (`render.rs:1160-1169`: "Assumes $0 W-2 wages… OVERSTATED… UNDERSTATED");
  the §164(f) informational line (~1154-1159); the standalone note (1171-1177).
- P2-D goldens (all W-2 = 0): Golden-1 $14,129.55/$7,064.78; C1-lock $30,564.30/$14,935.42; wage-base-cap
  $21,836.40; MFS $27,730.00; fractional $1,744.39 — plus the CLI KAT (`tax_report.rs:173`). ALL must stay
  byte-identical (W-2 defaults to $0).

## Design

### D1 — `TaxProfile.w2_ss_wages` + `w2_medicare_wages`
Two new fields, `#[serde(default)] pub w2_ss_wages: Usd` + `#[serde(default)] pub w2_medicare_wages: Usd`
(old stored profiles → $0, preserving current behavior). CLI: optional `--w2-ss-wages` +
`--w2-medicare-wages` on the `tax-profile` subcommand (default $0; reject negative — `CliError::Usage`,
validated on the REAL path, not a test-side copy). They may be set independently (each defaults $0; both
often equal for a simple W-2 job but Box 3 ≠ Box 5 is common — do NOT force pairing). **[R0-M2] Help
text:** "Form W-2 Social Security wages (Box 3 + Box 7 tips; Schedule SE line 8a)" / "Medicare wages
(Box 5; Form 8959 line 1)". **[R0-M4]** `tax-profile --show` displays the two new fields.

### D2 — `compute_se_tax` W-2 params (explicit — Option A)
`compute_se_tax(state, year, status, table, w2_ss_wages: Usd, w2_medicare_wages: Usd)` — explicit params,
NOT the whole profile (keeps SE decoupled from TaxProfile layout; unit-testable). Changes in the body:
- `se.rs:95` stub → the param; ss stays `12.4% × min(base, max(0, ss_wage_base − w2_ss_wages))` (the
  max(0,·) matters: W-2 SS wages above the base → cap 0 → ss 0).
- addl: `addl_threshold = max(0, se_addl_medicare_threshold(status) − w2_medicare_wages)`; `over =
  max(0, base − addl_threshold)` (W-2 Medicare above the threshold → all base taxed 0.9%).
- `deductible_half = (ss + medicare)/2` UNCHANGED (still excludes addl — P2-D C1).
**[R0-I1] BOTH call sites** pass the profile's W-2 fields: `cmd/tax.rs` (~79-88, the report) AND
`cmd/admin.rs` (~58-64, the **export path** that writes schedule_se.csv — missed by the recon; if it
defaulted to ZERO the CSV would contradict the report). Add an export-path pin: the schedule_se.csv
figures EQUAL the report figures for the same W-2-bearing profile.
**[R0-I4] Transposition guard:** the two params are both bare `Usd` — a swapped `(medicare, ss)` at either
call site would pass symmetric tests. Mandate ONE ASYMMETRIC-W-2 CLI-path KAT: w2_ss $150,000 /
w2_medicare $0 → ss == $3,236.40 AND addl == $0.00 (a transposition flips both). Name the params
unambiguously (`w2_ss_wages`, `w2_medicare_wages`).
**[R0-M5]** Document (doc-comment) the ≥0 precondition on both params (the CLI validates; core assumes).

### D3 — disclosure + §164(f) advisory (render)
- Replace the dual-direction "$0 assumed" block: when either W-2 field is > $0 → "W-2 coordination
  applied: SS cap = max(0, wage base − ${w2_ss}) (Box 3+7); Additional-Medicare threshold reduced (not
  below 0) by ${w2_medicare} (Box 5, §1401(b)(2)(B)/Form 8959 Part II)." **[R0-M3(a)] floored wording —
  never render a negative cap.**; when both are $0 → keep a short "assumes $0 W-2 wages (set --w2-ss-wages/
  --w2-medicare-wages on the tax profile if you had a wage job)" note (accurate either way — no more
  over/understated hedging when the figures are real).
- **§164(f) advisory micro-patch [R0-I3 — quantify, do NOT prescribe an OTI edit]:** after the
  deductible-half line: "The §164(f) deduction (${deductible_half}) is NOT auto-coordinated into the
  income-tax total above — to first order, that total overstates your combined tax by your marginal
  ordinary rate applied to ${deductible_half}. The tax profile cannot express this deduction directly
  (reducing `ordinary_taxable_income` would shift BOTH legs of the crypto-attributable delta and only
  correct the bracket differential, not the level) — coordinate it on your actual return."
  Rationale: the earlier "reduce your OTI by $X" prescription was wrong-mechanism (R0 worked example:
  it corrects only the bracket-differential slice of the overstatement, and corrects NOTHING when the
  bracket is unchanged). Text-only; no KAT figures move.
- **[R0-M1] Disclosure plumbing:** `render_schedule_se` needs the two W-2 values to print them — pass
  them (or the profile slice) through to the renderer alongside the SeTaxResult.

### Decisions
- Explicit params over profile-passing (decoupling); independent defaults (Box 3 ≠ Box 5 legitimate);
  full §164(f) auto-coordination stays DEFERRED (circular + identity — the recon's analysis; advisory
  instead); engine B untouched (SE standalone).

## Plan (TDD)

### Task 1 — fields + params + coordination + disclosure + goldens
- **Files:** `crates/btctax-core/src/tax/{types.rs,se.rs}`, `crates/btctax-cli/src/{main.rs,cmd/tax.rs,
  render.rs}`, tests in se.rs + `types.rs` (serde) + `tax_report.rs` (CLI).
- Hand-verified goldens (TY2025: wage base $176,100; Single addl threshold $200k; base for mining
  $100,000 = $92,350; assert EXACT; each must FAIL red pre-fix where W-2 > 0):
  - **Both-directions headline:** Single, mining $100,000, w2_ss $150,000, w2_medicare $150,000 →
    ss_cap 26,100 → ss = 12.4% × 26,100 = **$3,236.40** (was 11,451.40 — LOWER); addl_threshold 50,000 →
    over 42,350 → addl = **$381.15** (was 0 — HIGHER); medicare $2,678.15; total **$6,295.70**;
    deductible_half = (3,236.40 + 2,678.15)/2 = 5,914.55/2 = **$2,957.28** (half-even; EXCLUDES the 381.15).
  - **W-2 SS above the base:** w2_ss $180,000 (> 176,100), w2_medicare 0 → ss_cap 0 → **ss = $0.00**;
    addl 0 (threshold un-reduced, base < 200k); total = medicare only **$2,678.15**; deductible_half
    **$1,339.08** (2,678.15/2 = 1,339.075 half-even).
  - **W-2 Medicare above the threshold (isolated):** w2_ss 0, w2_medicare $250,000 Single →
    addl_threshold 0 → addl = 0.9% × 92,350 = **$831.15**; ss $11,451.40 + medicare $2,678.15 unchanged →
    total **$14,960.70**; deductible_half **$7,064.78** (unchanged from P2-D — ss+medicare untouched;
    pins that addl STILL doesn't enter the deductible).
  - **Regression net [R0-I2 — FIGURES byte-identical; TEXT assertions updated]:** all five P2-D se.rs
    golden FIGURE-sets unchanged (14,129.55 / 30,564.30 / 21,836.40 / 27,730.00 / 1,744.39) and the CLI
    KAT's figures unchanged — BUT the D3 disclosure rewrite deletes the OVERSTATED/UNDERSTATED text that
    `tax_report.rs:~176-179` and the `render.rs` schedule-se tests currently assert. UPDATE those text
    assertions (the R0-N1 pattern): the NEW phrasing present AND the old "may be OVERSTATED"/"may be
    UNDERSTATED" hedging absent. Do not claim the tests are untouched.
  - **[R0-I4] Asymmetric-W-2 transposition guard (CLI path):** profile w2_ss $150,000 / w2_medicare $0 →
    the rendered Schedule SE shows ss $3,236.40 AND addl $0.00 (a swapped param order flips both).
  - **[R0-I1/M6] Export-path parity (ASYMMETRIC fixture):** with the I4 asymmetric profile (w2_ss
    $150,000 / w2_medicare $0 — NOT the symmetric headline profile, so an admin-only transposition also
    fails), schedule_se.csv figures == the report figures (the cmd/admin.rs call site passes the same
    W-2 fields).
  - **Serde back-compat:** a profile JSON WITHOUT the two new fields deserializes with both $0 (extend the
    `optional_profile_fields_default_to_zero` KAT).
  - **CLI:** negative `--w2-ss-wages` → Usage error on the real path; the disclosure shows the coordinated
    text when set, the $0 note when not; the §164(f) advisory line present.

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: both coordination formulas correct (§1402(b)(1) cap; §1401(b)(2)(B) threshold; both
  floored at 0); the deductible still excludes addl; the P2-D regression net byte-identical; serde
  back-compat; engine B untouched (assert a tax golden unmoved); the disclosure accurate in both modes;
  exact Decimal; determinism.
- FOLLOWUPS: Chunk C (ReclassifyIncome) next, then Chunk B (expenses, advisory-only); full §164(f)
  auto-coordination remains deferred (documented rationale).

## Out of scope
- Schedule C expenses (Chunk B); ReclassifyIncome (Chunk C); §164(f) auto-coordination into the income-tax
  total (deferred — advisory only); engine-B/ordinary-income changes; Form 8959/Schedule SE PDF; validating
  W-2 box consistency; 2026/2027 tables.
