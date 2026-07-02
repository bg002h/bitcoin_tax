# R0 architect review — SPEC_se_chunkA_w2.md (round 1)

**Artifact:** `design/SPEC_se_chunkA_w2.md` (SE-completion Chunk A: W-2 wage coordination + §164(f)
advisory micro-patch)
**Baseline verified:** HEAD `33b7f26` (matches the spec's stated baseline)
**Reviewer:** R0 (independent; author ≠ reviewer)
**Verdict: NOT GREEN — 0 Critical / 4 Important / 5 Minor / 3 Nit.** Fold and re-review.

---

## 1. Independent legal verification (web, primary sources — NOT taken from the spec)

### 1.1 §1402(b)(1) — Social-Security cap coordination: CONFIRMED
- **Statute** (26 U.S.C. §1402(b)(1), Cornell LII): "self-employment income" excludes "that part of
  the net earnings from self-employment which is in excess of (i) an amount equal to the
  contribution and benefit base (as determined under section 230 of the Social Security Act) …
  minus (ii) the amount of the wages paid to such individual during such taxable year[]".
- **Form mechanism** (2025 Schedule SE, IRS f1040sse.pdf, fetched): line 7 = **$176,100** (2025);
  line 8a = "Total social security wages and tips (**total of boxes 3 and 7** on Form(s) W-2) and
  railroad retirement (tier 1) compensation. If $176,100 or more, **skip lines 8b through 10**";
  line 9 = "Subtract line 8d from line 7. **If zero or less, enter -0-**"; line 10 = "Multiply the
  **smaller of line 6 or line 9** by 12.4%".
- ⇒ `ss = 12.4% × min(base, max(0, ss_wage_base − w2_ss_wages))` is the correct §1402(b)(1) /
  Schedule SE 8a–10 mechanism, including the floor at 0 and the ss=0 outcome when W-2 SS wages ≥
  the base. **The spec's D2 SS formula is correct.** (Box nuance → M2 below.)

### 1.2 §1401(b)(2)(B) — Additional-Medicare threshold coordination: CONFIRMED
- **Statute** (26 U.S.C. §1401(b)(2)(B), Cornell LII): the §1401(b)(2)(A) threshold "shall be
  **reduced (but not below zero)** by the amount of wages taken into account in determining the
  tax imposed under section 3101(b)(2) with respect to the taxpayer" (i.e. FICA **Medicare** wages;
  the Cornell rendering shows the well-known "3121(b)(2)" scrivener's cross-ref — operationally
  §3101(b)(2), as implemented by Form 8959).
- **Form mechanism** (Form 8959 instructions, IRS, fetched): Part I line 1 = "Medicare wages and
  tips from **box 5** of your Form W-2" (totaled on line 4). **Part II**: line 8 = SE income from
  Schedule SE Part I line 6; line 9 = filing-status threshold; line 10 = the line-4 Medicare-wage
  total; line 11 = line 9 − line 10, **-0- if zero or less**; line 12 = line 8 − line 11, -0- if
  zero or less; line 13 = line 12 × 0.9%.
- ⇒ `addl_threshold = max(0, threshold(status) − w2_medicare_wages)`;
  `over = max(0, base − addl_threshold)`; `addl = 0.9% × over`. **The spec's D2 addl formula is
  correct**, including the all-of-base-taxed outcome when Medicare wages ≥ the threshold.
- Statutory thresholds (Single/HoH 200k, MFS 125k, MFJ/QSS 250k) match `tables.rs:167-173`.

### 1.3 Box 3 vs Box 5, independent defaults: SOUND (one Minor)
- Medicare side: **exactly Box 5** (Form 8959 line 1/line 10 — confirmed verbatim). ✓
- SS side: Schedule SE line 8a is **boxes 3 + 7** (SS *tips* ride Box 7, not Box 3) plus RRTA
  tier-1, and 8b/8c add Form 4137/8919 amounts → the "Box 3" help text slightly understates the
  reduction for tipped/RRTA users (→ **M2**).
- Independent fields (no forced pairing) are right: Box 3 is capped at the wage base while Box 5
  is uncapped, so Box 3 ≠ Box 5 is routine for high earners. ✓

### 1.4 §164(f)(1) addl-exclusion (unchanged from P2-D): re-confirmed structurally
2025 Schedule SE line 13 = 50% × line 12, where line 12 = lines 10+11 (SS + regular Medicare
only — Additional Medicare never appears on Schedule SE; it is a Form 8959 item). The spec keeping
`deductible_half = (ss + medicare)/2` is correct.

## 2. Golden re-derivation (independent, by hand; TY2025 base $176,100; Single threshold $200,000)

Mining $100,000 → `base = round_cents(100,000 × 0.9235) = 92,350.00`. Rounding = ROUND_HALF_EVEN
at cents (`round_cents`).

1. **Headline (w2_ss 150,000; w2_medicare 150,000):**
   ss_cap = 176,100 − 150,000 = 26,100; ss = 0.124 × min(92,350, 26,100) = **3,236.40** ✓ (LOWER
   than the un-coordinated 11,451.40 ✓); medicare = 0.029 × 92,350 = **2,678.15** ✓;
   addl_threshold = 200,000 − 150,000 = 50,000; over = 42,350; addl = 0.009 × 42,350 = **381.15** ✓
   (HIGHER than 0 ✓); total = **6,295.70** ✓; deductible_half = (3,236.40 + 2,678.15)/2 =
   2,957.275 → half-even tie → **2,957.28** ✓ (excludes the 381.15 ✓).
2. **SS-above-base (w2_ss 180,000 > 176,100; w2_medicare 0):** ss_cap = max(0, −3,900) = 0 →
   **ss = 0.00** ✓; addl over = max(0, 92,350 − 200,000) = 0 → addl 0 ✓; total = medicare only
   **2,678.15** ✓; deductible_half = 1,339.075 → half-even → **1,339.08** ✓.
3. **Medicare-above-threshold, isolated (w2_ss 0; w2_medicare 250,000):** addl_threshold =
   max(0, −50,000) = 0 → addl = 0.009 × 92,350 = **831.15** ✓; ss **11,451.40** / medicare
   **2,678.15** unchanged ✓; total **14,960.70** ✓; deductible_half = 14,129.55/2 = 7,064.775 →
   **7,064.78 UNCHANGED** ✓ — correctly pins addl-still-excluded.
4. **Regression net:** with both W-2 fields $0 the new formulas degenerate exactly to the current
   code (`ss_cap = 176,100`; threshold un-reduced) → the five se.rs golden figure-sets
   (14,129.55/7,064.78; 30,564.30/14,935.42; 21,836.40; 27,730.00; 1,744.39 — re-checked against
   se.rs:169-283) and the CLI-KAT **figures** (tax_report.rs:171-174, 195) are unchanged. The CLI
   KAT's **text** assertions are NOT unchanged — see **I2**.

**All three new goldens reproduce exactly; every direction claim in the spec is correct.**

## 3. Recon-citation verification against HEAD 33b7f26

| Spec claim | Verified |
|---|---|
| `se.rs:95` `let w2_ss_wages = Usd::ZERO;` stub | ✓ exact |
| `se.rs:111-118` addl `over` with no W-2 reduction | ✓ exact |
| `compute_se_tax(state, year, status, table)` at se.rs:80 | ✓ |
| Caller `cmd/tax.rs:79-88` | ✓ — **but not the ONLY caller: `cmd/admin.rs:58-64` (export) also calls it → I1 (drift)** |
| `TaxProfile` types.rs:31-50; serde-default KAT types.rs:129-136 | ✓ |
| Dual-direction disclosure render.rs:1160-1169; §164(f) line 1154-1159; standalone note 1171-1177 | ✓ |
| CLI KAT tax_report.rs:173 | ✓ (figure at 173; **text assertions at 176-179 → I2**) |
| Five P2-D golden values | ✓ all five re-checked |
| Wage base $176,100 in synthetic (tables.rs:256) AND bundled (adapters/tax_tables.rs:190) | ✓ |
| Chunk-3a negative-validation precedent on the real path | ✓ main.rs:435-446 (`--prior-taxable-gifts`, R0-M3) |

## 4. Findings

### Critical — none

### Important

**I1 — Recon missed the second `compute_se_tax` call site: the export path
(`crates/btctax-cli/src/cmd/admin.rs:58-64`) — spec must direct it, or `schedule_se.csv` can
silently disagree with the report.**
`export_snapshot` computes an `SeTaxResult` for `schedule_se.csv` (render.rs:722-723, 732-754) via
`.and_then(|p| tables.table_for(y).map(|t| (p.filing_status, t)))` — it destructures ONLY
`filing_status` from the profile. D2 says "Caller (`cmd/tax.rs`) passes …" (singular). The
signature change will force a compile error at admin.rs, but with no spec direction the natural
local fix is `Usd::ZERO, Usd::ZERO` — producing an exported CSV whose figures (e.g. 14,129.55)
contradict the report (6,295.70) for the same vault/year. A wrong-figure artifact is exactly what
this gate exists to block.
**Fix:** (a) amend the Current-state recon (two call sites); (b) D2: admin.rs passes
`p.w2_ss_wages`/`p.w2_medicare_wages` (the full profile is already in scope there); (c) Task 1 test
net: add an export-path pin that `schedule_se.csv` carries the coordinated figures when the year's
profile has W-2 set (or at minimum a stated invariant + test that CSV figures == report figures).

**I2 — Internal contradiction: "ALL must stay byte-identical … + the CLI KAT" vs D3's disclosure
replacement, which BREAKS the existing text assertions.**
`tax_report.rs:176-179` asserts `se.contains("OVERSTATED") && se.contains("UNDERSTATED")`, and
`render.rs:2355-2367` (`business_mining_year_renders_full_section`) asserts the same dual-direction
strings. D3 deletes that text in both modes, so these tests fail — the CLI KAT cannot "stay
byte-identical" as the recon (spec lines 34-36) demands. The Task-1 regression bullet says "figures
unchanged", but the spec never instructs updating the two text assertions.
**Fix:** scope "byte-identical" to the FIGURES; explicitly list the two text-assertion updates
(render.rs:2341-2384 and tax_report.rs:176-179) and require the R0-N1 semantic-assertion pattern
already used at tax_report.rs:210-214: assert the NEW phrase present AND the OLD phrase
("OVERSTATED"/"UNDERSTATED") ABSENT, in each mode (set / unset).

**I3 — §164(f) advisory prescription is the wrong mechanism: reducing `ordinary_taxable_income`
by `deductible_half` does NOT remove the stated overstatement, because OTI feeds BOTH legs of the
delta.**
The report's "income-tax total above" is `total_federal_tax_attributable` = T(b+c) − T(b)
(with − without). The true attributable delta including §164(f) is T(b+c−d) − T(b) (the deduction
exists only in the with-crypto world). The advisory's edit (b → b−d) yields T(b+c−d) − T(b−d):
the correction achieved is only d×(marg_with − marg_without) — **zero when crypto doesn't change
the bracket**, partial otherwise. Worked example on the existing CLI-KAT fixture (Single, OTI
40,000, mining 100,000, d = 7,064.78, TY2025): unadjusted delta 21,885.50; true delta
T(132,935.22) − T(40,000) = 24,751.45 − 4,561.50 = 20,189.95 → overstatement **1,695.55 =
0.24 × d** (so the spec's "overstates … by your marginal ordinary rate applied to it" IS the right
first-order statement, using the WITH-crypto marginal rate — the one `MarginalRates.ordinary`
reports). But after following the advisory: T(132,935.22) − T(32,935.22) = 24,751.45 − 3,713.73 =
21,037.72 — **still overstated by 847.77 (= 0.12 × d)**. The prescription both fails its promise
and quietly corrupts the without-crypto baseline (a world with no SE income gets a §164(f)
deduction), contradicting the spec's own deferral rationale ("circular + identity").
**Fix:** keep the quantified overstatement sentence (first-order, marginal-ordinary-rate × $X —
correct); DROP the "reduce your `ordinary_taxable_income` by this $X" instruction. State instead
that the profile cannot express the correction (OTI feeds both the with- and without-crypto legs)
and that the user should take the §164(f) deduction on their actual return / in their overall
liability calc, treating the app's delta as overstated by ≈ marginal-rate × $X.

**I4 — Caller-wiring transposition risk is untested: no profile→compute path test with
ASYMMETRIC W-2 values asserting a coordinated figure.**
Both new params are bare `Usd`; a swapped `(w2_medicare, w2_ss)` at either call site
(cmd/tax.rs or admin.rs) type-checks. The headline unit golden uses EQUAL values
(150,000/150,000 — swap-blind); the two asymmetric goldens are core-unit tests that bypass the
callers; the specified CLI tests assert disclosure TEXT and the Usage error, not coordinated
figures. So a transposed caller passes the entire specified test net while shipping wrong figures.
**Fix (either):** (a) add one CLI-path KAT with w2_ss = 150,000 / w2_medicare = 0 asserting the
report contains ss 3,236.40 AND addl 0.00 (a swap yields 11,451.40 / 381.15 — fails loudly); or
(b) make the two params a named struct (e.g. `W2Wages { ss, medicare }`), which kills the
transposition class at the type level. (a) is the smaller diff; (b) is the stronger invariant.

### Minor

**M1 — D3 renders `${w2_ss}` / `${w2_medicare}` but `render_schedule_se` has no access to them.**
Current signature is `(year, Option<&SeTaxResult>, business_income_present)`; `SeTaxResult`
carries no W-2 fields. The spec must choose the plumbing: extend `render_schedule_se`'s params, or
echo the two inputs on `SeTaxResult` (the latter also updates the render-test `golden1()`
constructor at render.rs:2327-2337 — compile-forced). Unspecified → ad-hoc divergence at
implementation time.

**M2 — Help-text box mapping is slightly narrow on the SS side.** Schedule SE line 8a is W-2
**boxes 3 + 7** (SS tips) plus RRTA tier-1; 8b/8c add Form 4137/8919 wages. Say "W-2 Box 3 + Box 7
(Social Security wages and tips; include RRTA tier-1 compensation)" or equivalent, else tipped/RRTA
users understate the reduction and OVERPAY the SS component's cap. (Box 5 for the Medicare side is
exactly right.)

**M3 — Coordinated-mode disclosure text can print a negative cap, and "set" is undefined.**
"SS cap = wage base − ${w2_ss}" is negative when w2_ss > the base — print the FLOORED cap (or
append "floored at $0"). Also define "set" for the mode switch (e.g. either field `> 0`), since an
explicit `--w2-ss-wages 0` is indistinguishable from unset.

**M4 — `tax-profile --show` printout omitted.** main.rs:625-641 prints every profile field; the
spec adds two fields + set-flags but never says to extend the show output (set-then-show would
hide them). Add both lines (and a KAT contains-assertion if show is tested).

**M5 — Core precondition on negative W-2 params.** CLI validation covers the CLI only; a direct
core caller passing a NEGATIVE `w2_ss_wages` INFLATES the cap past the statutory wage base
(`ss_cap = base − (neg) > wage_base`). Document the ≥ 0 precondition on `compute_se_tax` (or
`debug_assert!`/clamp), mirroring how the floor already guards the opposite direction.

### Nit

**N1 —** Spec line 20: the SE coordination is Form 8959 **Part II** (lines 8–13); Part I is
Medicare *wages*. Fix the citation.
**N2 —** Doc-comment sweep: se.rs:32-34 ("W-2 SS wages assumed $0 — D4"), se.rs:72 (same),
render.rs:1100 ("dual-direction W-2 disclosure") go stale; add to Task 1.
**N3 —** TDD sequencing: the signature change makes the first "red" a compile error, not a figure
failure. Sequence explicitly: (1) extend signature, callers pass `Usd::ZERO` literals → full
regression net green; (2) add the three W-2 goldens → genuine figure-level red (headline yields
11,451.40 / addl 0.00 against expected 3,236.40 / 381.15); (3) implement coordination → green.

## 5. Dimensions evaluated (no finding)

- **Option A explicit params vs profile-passing:** agreed — keeps `se.rs` decoupled from
  `TaxProfile` and unit-testable; the only real cost is the transposition risk, addressed in I4.
- **Independent field defaults:** correct (§1.3 above); forcing pairing would be wrong.
- **Serde back-compat:** the `#[serde(default)]` pattern is proven (types.rs:43-49 precedent +
  KAT at 129-136); extending that KAT is the right pin; `tax_profile_serde_round_trips`'s struct
  literal updates compile-forced.
- **Negative-flag validation on the REAL path:** matches the Chunk-3a precedent
  (main.rs:435-446, `is_sign_negative()`, `CliError::Usage`; zero accepted) — correctly specified.
- **Engine B untouched:** `compute_tax_year` never reads the new fields; Task 2's
  "assert a tax golden unmoved" pin is right (the existing tax_report.rs:193-197 21,885.50
  assertion already serves).
- **Scope/right-sizing:** Chunk A is properly thin; the out-of-scope list is coherent; deferring
  full §164(f) auto-coordination is CORRECT (I3's analysis independently re-confirms the
  circularity — the profile cannot express a with-leg-only deduction).
- **SemVer:** additive-but-breaking (struct literal + fn signature) ⇒ 0.x MINOR bump is the right
  call pre-1.0.

## 6. Verdict

**NOT GREEN: 0 Critical / 4 Important (I1 export-path call site + CSV divergence, I2 byte-identical
vs disclosure-replacement contradiction, I3 §164(f) prescription mechanism, I4 asymmetric
caller-wiring pin) / 5 Minor / 3 Nit.** The two coordination formulas and all three new goldens are
independently confirmed correct — the blocking findings are spec-completeness and advisory-accuracy
issues, all with concrete fixes above. Fold and submit for round 2.

## Sources (web verification)

- 26 U.S.C. §1402 (Cornell LII): https://www.law.cornell.edu/uscode/text/26/1402
- 26 U.S.C. §1401 (Cornell LII): https://www.law.cornell.edu/uscode/text/26/1401
- Form 8959 instructions (IRS): https://www.irs.gov/instructions/i8959
- Schedule SE instructions (IRS): https://www.irs.gov/instructions/i1040sse
- 2025 Schedule SE form (IRS PDF, lines 7–13 read directly): https://www.irs.gov/pub/irs-pdf/f1040sse.pdf

---

# Round 2 — re-review (post-fold)

**Date:** 2026-07-01. **Baseline re-confirmed:** HEAD `33b7f26` (unchanged since round 1; spec +
this review remain the only untracked additions). Spot-re-verified against source: `cmd/admin.rs`
export call site still destructures only `filing_status` (I1's premise); `tax_report.rs:176-179`
OVERSTATED/UNDERSTATED assertions and the R0-N1 semantic-pattern precedent (~210-214) as cited.
The coordination formulas and all three W-2 goldens were web-confirmed and hand re-derived in
round 1 — not re-litigated.

## R2.1 Fold verification — the four Importants

**I1 — CLOSED.** D2 now mandates BOTH call sites (`cmd/tax.rs` ~79-88 report AND `cmd/admin.rs`
~58-64 export) pass the profile's `w2_ss_wages`/`w2_medicare_wages`, names the CSV-contradicts-
report failure mode explicitly, and Task 1 adds the export-parity pin (schedule_se.csv figures ==
report figures for a W-2-bearing profile). That is fix (b)+(c) as prescribed. Fix (a) — amending
the Current-state recon bullet — was folded *in-place in D2* ("missed by the recon") rather than
by editing the recon line, which still reads as if `cmd/tax.rs` were the sole caller; D2
supersedes it explicitly, so this is a wording residue only (→ N5). One test-strength residue
on the parity fixture (→ **M6**, new, Minor).

**I2 — CLOSED.** The Task-1 regression bullet now scopes byte-identical to the five se.rs golden
FIGURE-sets + the CLI KAT's *figures*, explicitly lists the two text-assertion updates
(`tax_report.rs:~176-179` and the render.rs schedule-se tests), mandates the R0-N1 semantic
pattern (NEW phrasing present AND the old "may be OVERSTATED"/"may be UNDERSTATED" absent), and
states outright "Do not claim the tests are untouched." No remaining claim that those tests are
untouched: the Current-state sentence ("ALL must stay byte-identical") attaches to the listed
dollar figures and cites `tax_report.rs:173` — the figure line, not 176-179 — and the Task-1
[R0-I2] bullet unambiguously governs. Internally consistent.

**I3 — CLOSED (in the operative section).** D3's advisory now (a) quantifies the first-order
overstatement as marginal-ordinary-rate × `${deductible_half}` — correct, and in report context
"your marginal ordinary rate" is `MarginalRates.ordinary` (the with-crypto rate), which is the
right one; (b) explicitly does NOT prescribe an OTI edit; (c) states the profile cannot express
the deduction, with the mechanism-accurate parenthetical: reducing OTI shifts BOTH legs of the
delta and corrects only the bracket differential, not the level — this matches the round-1
identity ([T(b+c)−T(b)] − [T(b+c−d)−T(b−d)] ≈ d×(marg_with − marg_without)) and the worked
example; (d) directs coordination to the actual return. Mechanism-accurate. **However, two
pre-fold phrasings survive OUTSIDE D3** and contradict it (→ **M7**, new, Minor): the Goal
paragraph still says `concrete "reduce your ordinary_taxable_income by $X" wording`, and the
Legal-grounding §164(f) bullet still says "the advisory quantifies the user's manual profile
adjustment". Neither produces code (D3 specifies the renderer text verbatim, with the rationale
naming the old wording as wrong-mechanism), so the artifact implemented from D3 is correct —
but the sweep was incomplete.

**I4 — CLOSED.** The asymmetric CLI-path KAT genuinely catches a transposition — re-derived:
- **Correct wiring** (w2_ss 150,000 / w2_medicare 0): ss_cap = max(0, 176,100 − 150,000) =
  26,100 → ss = 12.4% × min(92,350, 26,100) = **3,236.40**; addl_threshold = max(0, 200,000 − 0)
  = 200,000 → over = max(0, 92,350 − 200,000) = 0 → addl = **0.00**.
- **Transposed** (effective w2_ss 0 / w2_medicare 150,000): ss = 12.4% × min(92,350,
  max(0, 176,100 − 0)) = 12.4% × 92,350 = **11,451.40** ≠ 3,236.40; addl_threshold =
  max(0, 200,000 − 150,000) = 50,000 → over = 42,350 → addl = 0.9% × 42,350 = **381.15** ≠ 0.00.
- BOTH asserted figures flip, so the KAT (which requires ss 3,236.40 AND addl 0.00) fails loudly
  on a swap in either direction. Param names `w2_ss_wages`/`w2_medicare_wages` are unambiguous;
  the ≥0 doc-precondition (M5) covers the bare-Usd hygiene concern. This closes the `cmd/tax.rs`
  path fully; the `cmd/admin.rs` path is closed by the I1 parity pin *iff* the parity fixture is
  asymmetric — see M6.

## R2.2 Fold verification — Minors

- **M1 folded.** D3 [R0-M1]: the two W-2 values are passed through to `render_schedule_se`
  alongside the `SeTaxResult` ("or the profile slice"). This selects the extend-params family and
  kills the ad-hoc-divergence risk; the residual choice (two `Usd` vs a slice) is cosmetic. ✓
- **M2 folded.** Help text now "Box 3 + Box 7 tips; Schedule SE line 8a" / "Box 5; Form 8959
  line 1" — the Box 7 tips inclusion was the substance; RRTA omission is acceptable (the line-8a
  cite carries it). Note "Form 8959 line 1" is Part I and is CORRECT for the *wages* flag. ✓
- **M4 folded.** D1 [R0-M4]: `tax-profile --show` displays both fields. ✓
- **M5 folded.** D2 [R0-M5]: doc-comment ≥0 precondition on both params; CLI validates, core
  assumes. ✓
- **M3 partially folded.** Part 2 (define "set") is resolved structurally: the two D3 modes are
  now exhaustive and complementary ("when either W-2 field is set" ↔ "when both are $0"), which
  defines set as >$0 and makes an explicit `--w2-ss-wages 0` equivalent to unset — fine. Part 1
  is NOT folded: the coordinated-mode template still prints "SS cap = wage base − ${w2_ss}",
  an expression that evaluates negative when w2_ss > the base — and the SS-above-base case
  ($180,000 > $176,100) is IN the golden set. Carries as **M3(a)** (residual, Minor): print the
  floored cap or append "(floored at $0)".
- **Nothing contradictory** remains from the M3 fold itself; the D3 mode-switch and the CLI test
  bullet ("coordinated text when set, the $0 note when not") agree.

## R2.3 New/residual findings (round 2)

### Critical — none. Important — none.

### Minor

**M6 (new) — Pin the export-parity fixture to an ASYMMETRIC W-2 profile.** The I4 KAT covers the
`cmd/tax.rs` path only; an `admin.rs`-only transposition is caught by the I1 parity pin *only if*
the parity profile has w2_ss ≠ w2_medicare (with a symmetric fixture — e.g. the 150k/150k
headline profile, the natural reuse candidate — swapped params produce identical figures and
parity passes while the CSV ships wrong figures for asymmetric users). The spec says only "a
W-2-bearing profile". One-line fix: reuse the I4 fixture (w2_ss $150,000 / w2_medicare $0) for
the export-parity KAT. Rated Minor, not Important: the mandated test exists and D2's wiring
directive is unambiguous — this is test-strength under-specification (defense-in-depth), not a
wrong specified behavior; round 1's I4 fix (a), which the spec folded exactly, had the same
scope.

**M7 (new) — Two stale pre-I3 phrasings contradict the folded D3.** (1) Goal paragraph:
`concrete "reduce your ordinary_taxable_income by $X" wording`; (2) Legal-grounding §164(f)
bullet: "the advisory quantifies the user's manual profile adjustment". Both restate the
prescription D3 explicitly rejects as wrong-mechanism. No behavioral consequence (D3 specifies
the renderer text verbatim and self-describes the supersession), but the document contradicts
itself on a previously-blocking axis. Two-line fix: Goal → "quantified-overstatement advisory
(no OTI-edit prescription — R0-I3)"; grounding bullet → "the advisory quantifies the first-order
overstatement (marginal rate × the deductible half); the profile cannot express the deduction."

**M3(a) (residual, from round-1 M3 part 1)** — floored-cap wording in the coordinated-mode
disclosure; see R2.2.

### Nit

**N1 (residual, unfolded)** — Spec Legal-grounding line ~20 still cites "Form 8959 **Part I**
coordination" for the threshold reduction; the SE coordination is **Part II** (lines 8–13).
(D1's "Form 8959 line 1" for the wages flag is correct and should NOT change.)
**N2 (residual, unfolded)** — the stale doc-comment sweep (se.rs:32-34, se.rs:72, render.rs:1100)
is still unmentioned in Task 1; Task 2's whole-diff review is the backstop.
**N3 (residual, softened)** — explicit red-sequencing not spelled out, but Task 1's "each must
FAIL red pre-fix where W-2 > 0" pins the figure-level-red requirement, which was the substance.
**N4 (new)** — the CLI test bullet pins the negative-value Usage error for `--w2-ss-wages` only;
D1 mandates rejection on both flags. Say "each new flag" (a negative w2_medicare reaching core
would INFLATE the addl threshold — M5's documented precondition, CLI-enforced only).
**N5 (new)** — the Current-state recon bullet still names `cmd/tax.rs:79-88` as the caller
(singular); D2 corrects it in-place ("missed by the recon"), so harmless, but amending the recon
line was fix (a) as written.

## R2.4 Dimensions re-checked (no finding)

- **Internal consistency** (beyond M7/N5 wording residue): D1/D2/D3/Task-1 agree on fields,
  params, both call sites, disclosure modes, and the test net; the figure-scoped byte-identical
  language is now coherent end-to-end.
- **Right-sizing:** unchanged from round 1 — Chunk A stays thin; the folds added tests and text,
  no new machinery; out-of-scope list still coherent; SemVer MINOR call still right.
- **TDD genuineness:** with the signature extended and callers passing the profile fields but
  coordination unimplemented, all three W-2 goldens and the I4 KAT fail on FIGURES (headline
  yields 11,451.40/0.00 vs expected 3,236.40/381.15), and the $0-default regression net stays
  green throughout — genuine red, not compile-error-only red.
- **Goldens:** headline deductible_half tie re-checked (5,914.55/2 = 2,957.275 → half-even →
  2,957.28 ✓); no figure in the spec moved between rounds.

## R2.5 Verdict

**GREEN — 0 Critical / 0 Important / 3 Minor (M6, M7, M3(a)) / 5 Nit (N1–N5).**
All four round-1 Importants are genuinely closed; no new Critical or Important findings. The
spec is ready to implement. Recommended (non-blocking, four one-line edits, none touching
design, formulas, or golden figures): fold M6 (asymmetric parity fixture), M7 (two stale I3-era
phrases), M3(a) (floored-cap wording), and N1 (Part I → Part II) before implementation, with a
delta-confirm; alternatively ticket them to FOLLOWUPS. N2/N4 can ride to the Task-2 whole-diff
review; N3/N5 need no action.
