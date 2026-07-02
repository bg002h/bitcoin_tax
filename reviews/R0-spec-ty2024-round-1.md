# R0 — Architect review: SPEC_ty2024_tables.md (round 1)

**Artifact:** `design/SPEC_ty2024_tables.md`
**Baseline verified against:** HEAD `42ddab876a132fd32fc21a3632763c31e4c96f78` (matches the spec's claimed `42ddab8`).
**Reviewer stance:** independent — every dollar figure re-verified against primary sources fetched
directly by the reviewer (NOT the spec author's extraction).
**Date:** 2026-07-01

**Verdict: NOT GREEN — 1 Critical, 1 Important, 3 Minor, 2 Nit.**
The transcription payload itself (every bracket edge, every LTCG breakpoint, gift/lifetime/wage-base)
is verified correct digit-by-digit against two independent renderings of the primary source. The
blocking findings are in the KAT hand-derivations (§1411 NIIT mishandled in A6d; MAGI derivation
method wrong in A6a–A6c), not in the table data.

---

## 1. Primary sources used (fetched independently by R0)

1. **Rev. Proc. 2023-34 — official IRS PDF**, `https://www.irs.gov/pub/irs-drop/rp-23-34.pdf`,
   downloaded 2026-07-01 (255,987 bytes), extracted with `pdftotext -layout`.
2. **Rev. Proc. 2023-34 — IRB publication of record**, Internal Revenue Bulletin 2023-48,
   `https://www.irs.gov/irb/2023-48_IRB` (HTML), used to cross-check the PDF rendering.
3. **SSA determination — Federal Register**, "Cost-of-Living Increase and Other Determinations for
   2024," FR Doc. 2023-23317, published 2023-10-23,
   `https://www.govinfo.gov/content/pkg/FR-2023-10-23/pdf/2023-23317.pdf` (the Commissioner's legal
   determination under §230 of the Social Security Act behind SSA's 2023-10-12 press release; the
   press release itself is Akamai-blocked to non-browser clients).

### Section-numbering claim — CONFIRMED

The PDF's table of contents and body confirm: **SECTION 2 = "CHANGES"** (Hazardous Substance
Superfund rate under §13601(a)(2)); **SECTION 3 = "2024 ADJUSTED ITEMS"**, with `.01 Tax Rate
Tables … 1(j)(2)(A)-(D)`, `.03 Maximum Capital Gains Rate … 1(h)`, `.41 Unified Credit Against
Estate Tax … 2010`, `.43 Annual Exclusion for Gifts … 2503; 2523`. The spec's mandate to cite
`§3.xx` (not `§2.xx` as in Rev. Proc. 2024-40) is correct.

## 2. Value-by-value re-verification (all CONFIRMED)

### §3.01 ordinary brackets — every edge matches the primary source verbatim

| Rate | MFJ (Table 1) | HoH (Table 2) | Single (Table 3) | MFS (Table 4) |
|------|--------------|---------------|------------------|---------------|
| 10%  | 0 ✓          | 0 ✓           | 0 ✓              | 0 ✓           |
| 12%  | 23,200 ✓     | 16,550 ✓      | 11,600 ✓         | 11,600 ✓      |
| 22%  | 94,300 ✓     | 63,100 ✓      | 47,150 ✓         | 47,150 ✓      |
| 24%  | 201,050 ✓    | 100,500 ✓     | 100,525 ✓        | 100,525 ✓     |
| 32%  | 383,900 ✓    | 191,950 ✓     | 191,950 ✓        | 191,950 ✓     |
| 35%  | 487,450 ✓    | **243,700** ✓ | **243,725** ✓    | 243,725 ✓     |
| 37%  | 731,200 ✓    | 609,350 ✓     | 609,350 ✓        | **365,600** ✓ |

- The HoH-35% transcription trap: primary source reads "Over $243,700 but not over $609,350 —
  $53,977 plus 35% of the excess over $243,700" (HoH) vs "Over $243,725 … $55,678.50 plus 35% of
  the excess over $243,725" (Single/MFS). The spec pins both correctly.
- MFS 37%: "Over $365,600 — $98,334.75 plus 37% of the excess over $365,600" — stated explicitly
  in the Rev. Proc. (and equals MFJ 731,200 ÷ 2). Spec correct.
- Base-tax cross-check: every "the tax is $X plus r%" base amount is arithmetically consistent
  with the bounds above (e.g. Single at 243,725: 39,110.50 + 32%×(243,725−191,950) = 55,678.50 ✓;
  MFS at 365,600: 55,678.50 + 35%×(365,600−243,725) = 98,334.75 ✓). Independent confirmation the
  bounds are transcribed right.
- Table 5 (Estates & Trusts) exists in the Rev. Proc. and is correctly not modeled (no such
  `FilingStatus`; same as TY2025).

### §3.03 LTCG breakpoints — all four rows match verbatim

| Status | max_zero | max_fifteen |
|--------|----------|-------------|
| MFJ/QSS | 94,050 ✓ | 583,750 ✓ |
| MFS | 47,025 ✓ | **291,850** ✓ |
| HoH | 63,000 ✓ | 551,350 ✓ |
| Single ("All Other Individuals") | 47,025 ✓ | 518,900 ✓ |

**MFS max_fifteen = $291,850 CONFIRMED verbatim in both the PDF and the IRB HTML** — NOT $291,875
(583,750/2). The spec's independent-rounding flag is correct and correctly resolved.

### §3.43 / §3.41 / SSA — CONFIRMED

- **§3.43(1):** "For calendar year 2024, the first **$18,000** of gifts to any person … are not
  included in the total amount of taxable gifts under § 2503" ✓.
- **§3.41:** "For an estate of any decedent dying in calendar year 2024, the basic exclusion
  amount is **$13,610,000** … under § 2010" ✓.
- **SS wage base:** Federal Register (FR Doc. 2023-23317, 2023-10-23): "the OASDI contribution and
  benefit base is **$168,600** for 2024" (§230 of the Act; average-wage-indexed from the 1992
  $60,600 base; exceeds the prior $160,200 base for 2023) ✓. The spec's "$160,200 in 2023" and
  "TY2025 = $176,100" context values also confirmed.

### TaxTable field coverage — COMPLETE

`TaxTable` (`crates/btctax-core/src/tax/tables.rs:53–82`) has exactly seven fields: `year`,
`source`, `ordinary`, `ltcg`, `gift_annual_exclusion`, `ss_wage_base`, `gift_lifetime_exclusion`.
The spec supplies a verified TY2024 value for every one (year=2024, source cite string, 4×7
ordinary brackets, 4 LTCG pairs, 18,000, 168,600, 13,610,000). No placeholder risk.

### Statutory-constant exclusion — CONFIRMED

The spec's "confirmed NOT in TaxTable" list matches `tables.rs` exactly: `NIIT_RATE` 0.038 (l.133),
`SE_RATE_SS` 0.124 (l.139), `SE_RATE_MEDICARE` 0.029 (l.144), `SE_RATE_ADDL_MEDICARE` 0.009
(l.151), `SE_NET_EARNINGS_FACTOR` 0.9235 (l.157), `se_addl_medicare_threshold` (l.167),
`QUALIFIED_APPRAISAL_THRESHOLD` 5000 (l.179), `niit_threshold` (l.190), `loss_limit` (l.204).
None appear in `TaxTable`; none change under this spec.

### Structure citations vs HEAD 42ddab8 — CONFIRMED (with one Nit, N1)

- `tax_tables.rs:1` module docstring, `:49–57` `load()` + TY2026 placeholder, `:53` single
  `insert(2025, ty2025())` ✓. Struct comment text is at lines 39–40 (spec says 38–41 — N1).
- `compute.rs:258–264` missing-table branch, `TaxTableMissing`, detail `"no bundled tax table for
  {year}"` ✓.
- Five "TY2025 only" comment sites all exist as cited (`tax_tables.rs:1`, `:39`, `:49`,
  `optimize.rs:162`, `optimize_accept.rs:83`); a repo-wide sweep for `TY2025` found no additional
  "only-2025" claim sites (but see M1 on the module docstring's Source-citation block).
- Existing-test claims: `missing_year_returns_none` uses 2099 ✓ (`tax_tables.rs:281–283`);
  `refusal_and_missing_table_end_to_end` uses 2099 ✓ (`kat_rate_engine.rs:381–390`);
  `carryforward_mismatch_advisory_rendered` uses 2026 ✓ (`tax_report.rs:575–631`);
  `method_election.rs` `synth_2024()` is a local `OneTable` double independent of
  `BundledTaxTables` ✓ (`method_election.rs:440–466`). Task 2 item 6's six TY2025 test names all
  exist verbatim ✓. No test asserts `table_for(2024).is_none()` ✓ — but the inventory is
  incomplete (M2).

### KAT hand-derivations — re-derived by R0

- **A6a Single** (re-derived): tax(47,150)=5,426.00 (matches the Rev. Proc.'s own base amount);
  tax(48,150)=5,646.00; ord Δ = **$220.00** ✓.
- **A6b MFJ** (re-derived): tax(200,000)=34,106.00; tax(202,000)=34,565.00; ord Δ = **$459.00** ✓.
- **A6c HoH** (re-derived): tax(63,000)=7,229.00; tax(63,500)=7,329.00; ord Δ = **$100.00** ✓.
- **A6d MFS** ordinary component (re-derived): tax(365,000)=98,124.75; tax(366,000)=98,482.75;
  ord Δ = **$358.00** ✓ — **but the NIIT leg is wrong; see C1.**
- **A7 LTCG** (re-derived against `preferential_tax`, compute.rs:53–95): bottom 40,000, pref
  10,000; at_0 = 7,025, at_15 = 2,975; tax = **$446.25**; MAGI_with = 40,000+10,000 = 50,000
  (correctly derived as magi_excl + crypto_agi here) < 200,000 → NIIT 0; total **$446.25** ✓.
- KAT-A1 index/value pairs, A2, A3 (incl. QSS→MFJ mapping via `TaxTable::key`), A4, A5: all values
  verified ✓.

### Scope — CONFIRMED

No engine change (compute path untouched; adding the table only changes `load()` data). TY2026
placeholder stays as-is; 2026/2027 remain blocked; `carryforward_mismatch_advisory_rendered`
depends on 2026 staying unbundled and is preserved. SemVer PATCH claim is sound (additive data,
no public API change).

---

## 3. Findings

### C1 (Critical) — KAT-A6d expected total omits the $38.00 NIIT; the supporting rationale is a false statement of both engine semantics and §1411

Spec §Plan/KAT-A6d claims:

> NIIT: MAGI_with = 366,000 > $125,000 MFS threshold; nii_with = 0 (no LT/income) → NIIT = 0.
> (ST ordinary gain is already in ordinary income; NIIT base is NII, not ST gains as ordinary.)
> total = **$358.00**.

Both halves are wrong:

- **Engine:** `compute.rs:352–353` — `nii_with = qd + with.ordinary_gain + with.preferential_gain
  - with.loss_deduction + interest_nii`. `with.ordinary_gain` is the surviving net **short-term**
  gain: ST gains ARE in the engine's NII. The module contract says so explicitly (compute.rs:214:
  "NII is `QD + surviving net capital gains (ST+LT)` …").
- **Law:** §1411(c)(1)(A)(iii) / Form 8960 line 5a — net gain from disposition of property is NII
  regardless of holding period. Short-term capital gains are taxed at ordinary *rates* but are not
  thereby excluded from NII.

Correct A6d derivation: crypto_agi = 1,000 → MAGI_with = 366,000 + 1,000 = 367,000 > 125,000;
nii_with = 1,000; NIIT = 3.8% × min(1,000, 242,000) = **$38.00**; niit_without = 0.
**total = 358.00 + 38.00 = $396.00** (pinned identity: total = ord Δ + ltcg_tax + niit).

As written, the KAT would fail red against a *correct* implementation — inverting TDD semantics
for a data-only change and inviting the implementer to "fix" inputs or doubt the (correct,
untouchable) engine. Worse, the false parenthetical would propagate into test comments as a
purported statement of §1411 law.

**Fix (exact):** in A6d set expected `total = $396.00`; add `assert_eq!(r.niit, dec!(38.00))` and
(recommended) `assert_eq!(r.total_federal_tax_attributable, dec!(396.00))` with the derivation
above; delete the parenthetical and replace with: "ST gain is NII (§1411(c)(1)(A)(iii); engine:
`nii_with` includes `with.ordinary_gain`, compute.rs); MFS threshold $125,000 exceeded → NIIT
3.8% × 1,000 = $38.00." Note: there is no MFS input near the 365,600 edge that avoids NIIT (the
$125,000 statutory threshold is far below the edge), so the KAT must carry the NIIT leg.

### I1 (Important) — A6a–A6c MAGI_with derivations use the wrong formula (the same error that produced C1)

The engine computes `magi_with = profile.magi_excluding_crypto + crypto_agi`
(compute.rs:357–361). The spec instead asserts `MAGI_with = MAGI excl.` in three derivations:

- A6a: claims MAGI_with = 48,150; engine gives 48,150 + 1,000 = **49,150**.
- A6b: claims MAGI_with = 202,000; engine gives 202,000 + 2,000 = **204,000**.
- A6c: claims MAGI_with = 63,500; engine gives 63,500 + 500 = **64,000**.

All three remain far below their §1411 thresholds, so the asserted totals ($220.00 / $459.00 /
$100.00) are still correct — but the hand-derivations are the verification substrate of this
spec, the stated method is wrong, and applying it to A6d produced the Critical. Additionally the
chosen `MAGI excl.` values *embed the crypto gain* (e.g. A6a: 48,150 = OTI 47,150 + the $1,000
crypto gain), which contradicts the field's meaning (`magi_excluding_crypto`) and deviates from
the established KAT convention (`kat_rate_engine.rs` uses `magi = OTI`, e.g. `mfs_profile(270000,
270000)` in KAT 7).

**Fix (exact):** set `magi_excluding_crypto = OTI` in all four A6 fixtures (47,150 / 200,000 /
63,000 / 365,000), and rewrite every derivation line as
`MAGI_with = magi_excl + crypto_agi = OTI + gain` (48,150 / 202,000 / 63,500 / 366,000 — all
still below threshold for A6a–c; A6d per C1). A7 already derives MAGI_with correctly; leave it.

### M1 (Minor) — D3 site 1 under-specified: the module docstring's "# Source citation" block must gain the TY2024 cites

D3 says only "update … module docstring" (site 1 = line 1). But `tax_tables.rs:12–18` ("TY2025
values are encoded verbatim from: Rev. Proc. 2024-40 …") and line 24 ("the TY2025 indexed values
are exactly Rev. Proc. 2024-40") are inside that docstring. Updating only line 1 leaves the
docstring implying all bundled values derive from Rev. Proc. 2024-40 — wrong once TY2024 lands.
**Fix:** state explicitly in D3 that the docstring update includes (a) adding a Rev. Proc. 2023-34
§3.01/§3.03/§3.41/§3.43 + SSA 2023-10-12 citation block for TY2024, and (b) keeping the OBBBA note
scoped to TY2025.

### M2 (Minor) — "Existing tests unaffected" inventory is incomplete (two year-2024 call sites missing)

- `crates/btctax-tui/src/tabs/tests.rs::tax_tab_year_change_updates_figures` (~lines 926–958):
  navigates Left to year 2024 with no 2024 profile and asserts "NOT COMPUTABLE". The TUI uses
  `BundledTaxTables` (`app.rs:108`, `unlock.rs:118`), so after the backfill the blocker KIND
  rendered at 2024 flips from `TaxTableMissing` to `TaxProfileMissing` (refusal precedence (2)→(3),
  compute.rs). The test still passes — it asserts only the "NOT COMPUTABLE" string and absence of
  "1500.00" — but this is a real behavioral shift at year 2024 that the completeness-claiming
  inventory missed.
- `crates/btctax-cli/tests/optimize_run.rs:385` `optimize_run_pre2025_is_usage_error`: calls
  `optimize::run(…, 2024, …)` and expects an error — unaffected, because the pre-2025 guard fires
  before any table lookup, but it belongs in the inventory as a checked-and-cleared 2024 site.

**Fix:** add both to Current state §"Existing tests unaffected", with the TaxTableMissing→
TaxProfileMissing note for the TUI test.

### M3 (Minor) — "All six fields" lists seven

Out of scope §"TaxTable struct fields" says "All six fields" then enumerates seven (`year`,
`source`, `ordinary`, `ltcg`, `gift_annual_exclusion`, `ss_wage_base`, `gift_lifetime_exclusion`).
**Fix:** "All seven fields". (Also reword "are already present and carry the TY2024 values" →
"are already present and receive the TY2024 values from this spec".)

### N1 (Nit) — struct-comment line cite off by one

Spec cites `tax_tables.rs:38–41` for the struct comment; the "Currently contains **TY2025** only"
text is at lines 39–40 (doc comment spans 37–42). Harmless; correct on next touch.

### N2 (Nit) — record the primary-source rendering discrepancy in the 32% rows

In the official PDF (`rp-23-34.pdf`), the 32% rows of Tables 2–4 print "…plus 32% of the excess
over **$191,150**" while the bound column reads "Over **$191,950**". The IRB publication of
record (IRB 2023-48 HTML) reads $191,950 in BOTH columns, and the base amounts confirm it
(37,417 = 15,469 + 24%×(191,950−100,500); 39,110.50 = 17,168.50 + 24%×(191,950−100,525)). The
spec's $191,950 is correct. **Fix:** add one sentence to §Legal grounding noting the PDF
discrepancy and its IRB/arithmetic resolution, so future digit-by-digit verifiers don't stall on
it.

---

## 4. Disposition

| # | Severity | One-line | Blocking |
|---|----------|----------|----------|
| C1 | Critical | A6d total $358.00 omits $38.00 NIIT; false §1411/engine claim | YES |
| I1 | Important | A6a–c MAGI_with derivation formula wrong (results survive by margin) | YES |
| M1 | Minor | D3 module-docstring update must include the Source-citation block | no |
| M2 | Minor | Test inventory misses TUI year-2024 test + optimize_run 2024 site | no |
| M3 | Minor | "six fields" vs seven enumerated | no |
| N1 | Nit | Struct-comment line cite 38–41 vs 39–40 | no |
| N2 | Nit | Note the PDF's 32%-row $191,150 rendering typo + resolution | no |

Gate result: **blocked** pending C1 + I1 fixes and re-review. The table data itself needs no
change — all transcribed constants are confirmed correct against two renderings of Rev. Proc.
2023-34 and the Federal Register SSA determination.

---

# Round 2 — re-review (post-fold)

**Artifact re-read in full:** `design/SPEC_ty2024_tables.md` (all seven round-1 findings folded).
**Date:** 2026-07-01. Same reviewer; every folded figure and citation re-derived/re-checked
against source, not trusted from the fold.

## Fold-by-fold verification

### C1 — CLOSED

A6d (spec lines ~381–397) now reads: ord Δ **$358.00** + NIIT **$38.00** → total **$396.00**,
with `assert_eq!(r.niit, dec!(38.00))` and
`assert_eq!(r.total_federal_tax_attributable, dec!(396.00))`.

Re-derived once more, end-to-end against the engine:
- Ordinary: tax(365,000) = 55,678.50 + 35%×(365,000−243,725) = 98,124.75;
  tax(366,000) = 98,334.75 + 37%×400 = 98,482.75; Δ = 358.00 ✓ (both cumulative cross-checks in
  the spec match, and the 55,678.50/98,334.75 bases match the Rev. Proc.'s printed amounts).
- NIIT: `with` = net_1222(1,000, 0, …) → ordinary_gain = 1,000; crypto_agi = 1,000;
  MAGI_with = 365,000 + 1,000 = 366,000 > 125,000; nii_with = 1,000; over = 241,000;
  base = min(1,000, 241,000) = 1,000; niit_with = 38.00; niit_without = 0; delta = 38.00 ✓.
- Identity: 358.00 + 0 + 38.00 = 396.00 ✓.

The false parenthetical is gone. The replacement "NII convention [R0-C1]" block quotes the
engine formula **verbatim** (checked character-for-character against `compute.rs:352–353`) and
the module contract at `compute.rs:214`, and states the correct §1411(c)(1)(A)(iii)/Form 8960
line 5a rule. The closing note (no MFS input near the 365,600 edge avoids NIIT) is correct.

### I1 — CLOSED

The new "Fixture convention [R0-I1]" block pins `magi_excluding_crypto = OTI` for all four A6
fixtures (47,150 / 200,000 / 63,000 / 365,000) and cites the engine formula
(`compute.rs:357–361` — verified) and the `kat_rate_engine.rs` KAT-7 precedent
(`mfs_profile(270000, 270000)` — verified). All four derivations now use
`MAGI_with = OTI + gain` with explicit margins and `niit == 0` asserts on A6a–c. Re-verified
each end-to-end:

| KAT | MAGI_with | Threshold (margin) | Cumulative cross-check | ord Δ | NIIT | Total |
|-----|-----------|--------------------|------------------------|-------|------|-------|
| A6a Single | 47,150+1,000 = 48,150 | 200,000 (151,850 ✓) | 5,426.00 → 5,646.00 ✓ | 220.00 ✓ | 0 ✓ | **$220.00** ✓ |
| A6b MFJ | 200,000+2,000 = 202,000 | 250,000 (48,000 ✓) | 34,106.00 → 34,565.00 ✓ | 459.00 ✓ | 0 ✓ | **$459.00** ✓ |
| A6c HoH | 63,000+500 = 63,500 | 200,000 (136,500 ✓) | 7,229.00 → 7,329.00 ✓ | 100.00 ✓ | 0 ✓ | **$100.00** ✓ |
| A6d MFS | 365,000+1,000 = 366,000 | 125,000 (exceeded) | 98,124.75 → 98,482.75 ✓ | 358.00 ✓ | 38.00 ✓ | **$396.00** ✓ |

All spec-stated cumulative bases match the Rev. Proc.'s own printed base-tax amounts
(5,426 / 10,852→34,337 / 1,655→7,241 / 55,678.50→98,334.75). A7 unchanged and still correct
($446.25; its MAGI derivation was already right in round 1).

### M1–M3, N1–N2 — CLOSED

- **M1:** D3 now carries the [R0-M1] block mandating (a) a TY2024 citation block in the module
  docstring's "# Source citation" section (Rev. Proc. 2023-34 §3.01/§3.03/§3.43/§3.41 + SSA
  2023-10-12) and (b) the OBBBA note kept scoped to TY2025. ✓
- **M2:** both inventory entries added under "Existing tests unaffected" ([R0-M2]: the TUI
  `tax_tab_year_change_updates_figures` blocker-kind flip TaxTableMissing→TaxProfileMissing,
  still-passing; `optimize_run_pre2025_is_usage_error` cleared — pre-2025 guard precedes table
  lookup), plus new Task 2 item 11 re-verifying the flip at whole-diff time. ✓
- **M3:** "All seven fields … receive the TY2024 values from this spec." ✓
- **N1:** struct-comment cite corrected to lines 39–40 (doc comment spans 37–42), in both the
  Current-state bullet and comment-site list item 2. ✓
- **N2:** [R0-N2] block added under the §3.01 tables noting the PDF's "$191,150" 32%-row
  rendering typo and its IRB-2023-48/arithmetic resolution. Both quoted base-tax computations
  re-verified: 15,469 + 24%×91,450 = 37,417 ✓; 17,168.50 + 24%×91,425 = 39,110.50 ✓. ✓

## New-findings sweep over the folded text

Checked every citation and figure introduced by the folds: the quoted `nii_with` formula
(verbatim match to `compute.rs:352–353`), the `compute.rs:214` contract quote, the
`compute.rs:357–361` MAGI formula, the KAT-7 fixture cite, all four margins, all eight
cumulative tax values, the N2 arithmetic, and Task 2 item 11's claims. No errors introduced.
The §3.01/§3.03/§3.43/§3.41/SSA constant payload is byte-identical to the round-1 digit-verified
text (the only insertion into that region is the [R0-N2] note). Internally consistent throughout
(fixture convention ↔ derivations ↔ asserts ↔ pinned identity `total = ord Δ + ltcg_tax + niit`).

Residual observation (below Nit threshold, no action required): Current state cites the
missing-table branch as `compute.rs:258–261`; the `else` block actually spans 258–264. The
anchor line (258) is exact and the claim's substance is correct.

## Round-2 disposition

| # | Round-1 severity | Status |
|---|------------------|--------|
| C1 | Critical | CLOSED — verified |
| I1 | Important | CLOSED — verified |
| M1 | Minor | CLOSED — verified |
| M2 | Minor | CLOSED — verified |
| M3 | Minor | CLOSED — verified |
| N1 | Nit | CLOSED — verified |
| N2 | Nit | CLOSED — verified |

**Gate result: 0 Critical / 0 Important / 0 Minor / 0 Nit open — R0 GREEN. The spec may proceed
to implementation.** Task 2 items 1–11 (including the R0 re-verification duties at whole-diff
time) remain binding on the implementation review.
