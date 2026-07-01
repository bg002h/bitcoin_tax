# SPEC — NII interest slice: crypto-lending interest → §1411 NII

**Source baseline:** `main` @ `43c653e` (post gift-cluster merge — the CLI line refs cite this baseline;
`tax/` was untouched by the gift work). First item of the user-confirmed P2/3 queue (NII slice → SE
completion → TY2024 tables).
**Goal:** Close the known B-M1 residual **NIIT understatement**: crypto-lending **interest** income
(`IncomeKind::Interest`) IS Net Investment Income under **§1411(c)(1)(A)(i)** and must enter the NII base;
mining/staking/airdrops/rewards stay excluded (SE income per §1411(c)(6), or non-NII "other income").
Tax-math change in engine B — hand-verified goldens required.

**SemVer:** behavior change to `compute_tax_year`'s NIIT figure in interest-income years ⇒ **MINOR**
(pre-1.0). No struct/API change (`TaxResult` unchanged).

## Legal grounding (R0 to web-verify)
- **§1411(c)(1)(A)(i):** NII includes gross income from **interest** (unless derived in an ordinary
  trade/business that's neither a §1411 passive activity nor trading — the §1411(c)(2) exception; for a
  single-user crypto-lending account this exception does not apply — lending interest is classic portfolio
  interest). Crypto-lending interest = interest for this purpose (the B-M1 determination, re-verify).
- **Mining/staking/airdrops/rewards:** NOT NII — business income is SE-excluded via §1411(c)(6)
  (SE-covered income is excluded from NII); hobby income is "other income" outside the §1411(c)(1)(A)
  categories. EXCLUDING them remains correct (B-M1-verified).
- **`business` flag irrelevance:** interest is NII per §1411(c)(1)(A)(i) whether or not the lending is
  business-flagged (and interest is already excluded from SE per §1402(a)(2), P2-D) — filter on
  `kind == Interest` only.

## Current-state (recon @ 114d6e0 — `crates/btctax-core/src/tax/compute.rs`)
- `crypto_ord` (297-302): kind-agnostic Σ over `income_recognized` in the year → feeds `bottom_with` (327,
  WITH only) + `crypto_agi`→`magi_with` (348-352). NOT in NII.
- `nii_with`/`nii_without` (344-346): `qd + ordinary_gain + preferential_gain − loss_deduction` per
  scenario — NO income component in either.
- **The WITH/WITHOUT convention (the delta-attribution rule):** `crypto_ord` is in the WITH scenario only
  (`bottom_with` 327 vs `bottom_without` 329; `magi_with` via crypto_agi 352 vs `magi_without` 351 =
  `magi_excluding_crypto`). The WITHOUT scenario = "no crypto at all". Any crypto-sourced NII must
  therefore enter `nii_with` ONLY — adding it to BOTH would cancel in `r.niit = niit_with − niit_without`
  and hide the liability from the crypto-attributable delta.
- `niit` closure (353-364): `3.8% × max(0, min(nii, magi − thr))`, floored at 0 (B-M1). Delta at 389.
- MAGI already includes interest (via `crypto_agi` ⊇ `crypto_ord`) — a NII-only insertion adds NO
  double-count.
- SE (P2-D, `se.rs:58`): interest already excluded from net SE (`kind != Interest`) — consistent; no change.
- Disclosure debt: `render.rs:1020-1028` footer says "The only residual understatement is crypto-lending
  interest … which the minimal model cannot yet isolate — a Phase-2 refinement"; same language in
  `compute.rs` module-doc (217-219) + the inline comment (341-343); KAT
  `report_tax_year_footer_discloses_1211_loss_and_lending_interest_caveat` (`tax_report.rs:212-243`) pins
  it. All must change (the model NOW isolates it).
- Existing NIIT goldens: NONE has `Interest` income in `compute_tax_year`; the B-M1 loss-year goldens
  (−684.00 / −57.00) and `niit_threshold_crossing` (760.00) have no income; `full_worked_example` (2280.00)
  and the double-count guard use **Mining** → all UNMOVED by this fix (regression net).

## Design

### D1 — `interest_nii` into `nii_with` ONLY
In `compute_tax_year` (after the `crypto_ord` sum, before `nii_with`):
```rust
// §1411(c)(1)(A)(i): crypto-lending interest IS NII. Isolated from the rest of crypto_ord
// (mining/staking/airdrops/rewards stay excluded — SE income per §1411(c)(6) or non-NII other income).
// WITH-scenario only, per the crypto_ord attribution convention (the WITHOUT scenario = no crypto):
// adding it to both scenarios would cancel out of the r.niit delta and hide the liability.
let interest_nii: Usd = state.income_recognized.iter()
    .filter(|i| i.recognized_at.year() == year && i.kind == IncomeKind::Interest)
    .map(|i| i.usd_fmv).sum();
```
`nii_with = qd + with.ordinary_gain + with.preferential_gain − with.loss_deduction + interest_nii;`
`nii_without` UNCHANGED. `crypto_agi`/MAGI UNCHANGED (interest already there — no double-count).
Needs `use crate::event::IncomeKind;` (mirror `se.rs`). Filter on `kind` only (business-irrelevant, D-legal).

### D2 — disclosure + comments (the "cannot yet isolate" language goes away)
- `render.rs` footer (~1026-1027): replace the residual-understatement sentence with: "crypto-lending
  interest income (§1411(c)(1)(A)(i)) is INCLUDED in NII; mining/staking/airdrops/rewards remain excluded
  (SE income per §1411(c)(6) or non-NII other income)."
- `compute.rs` module-doc (217-219) + inline comment (341-343): same correction.
- The `tax_report.rs:212-243` KAT: rename + re-point the positive assertion (interest IS in NII; the
  "cannot yet isolate" string is GONE); keep the B-M1 negative assertions (no "can only ever understate" /
  "MAY UNDERSTATE" / "does not reduce NII").

### Decisions
- **WITH-only insertion** — the established crypto_ord attribution convention; a both-scenario insertion
  cancels out of the delta (wrong: the owner's lending interest is crypto-attributable NII).
- **Kind-based filter (`Interest` only)**, business-agnostic; the §1411(c)(2) trade-or-business exception
  disclosed as inapplicable to portfolio-style crypto lending (no new modeling).
- NO change to: `TaxResult`, the `total == ord_delta + ltcg_tax + niit` identity (niit is still the delta),
  §1222/§1211/MAGI/SE, the bottom stacks.

## Plan (TDD)

### Task 1 — interest_nii + disclosure + goldens (single task)
- **Files:** `crates/btctax-core/src/tax/compute.rs` (D1 + comments), `crates/btctax-cli/src/render.rs`
  (the footer), `crates/btctax-core/tests/tax_compute.rs` (new goldens), `crates/btctax-cli/tests/
  tax_report.rs` (the disclosure KAT).
- Hand-verified goldens (TY2025, Single, thr $200k; assert EXACT):
  - **Interest+NIIT (headline, exercises the min-cap):** ordinary_taxable_income $150,000,
    magi_excluding_crypto $195,000, qd $0, no disposals/other gains, Interest income $20,000 →
    `interest_nii` 20,000; `nii_with` 20,000; `magi_with` 215,000 → over 15,000 → `niit_with` = 3.8% ×
    min(20,000, 15,000) = **$570.00**; WITHOUT: magi 195,000 < 200,000 → niit_without 0 → **`r.niit ==
    570.00`** (also pins the min(nii, over) cap: nii > over). **[R0-N2] Pin the ABSOLUTE total too:**
    ord_delta = tax(170,000) − tax(150,000) = $4,400.00 (TY2025 Single) → `total_federal_tax_attributable
    == 4,970.00` (= 4,400 + 0 + 570), not just the identity.
  - **Mixed Mining+Interest (the exclusion-boundary lock):** Mining $30,000 + Interest $10,000,
    magi_excluding_crypto $200,000 → crypto_ord 40,000, `interest_nii` 10,000; `magi_with` 240,000 → over
    40,000; `nii_with` 10,000 → niit_with = 3.8% × 10,000 = $380.00; niit_without 0 (magi_without =
    200,000, over 0) → **`r.niit == 380.00`**. (If mining wrongly entered NII: 3.8% × 40,000 = $1,520.00 —
    the golden fails. This is the boundary lock.)
  - **Regression net (must NOT move):** `niit_loss_year_reduces_nii_by_1211_allowed_loss` (−684.00),
    `niit_loss_year_mfs_1500_limit` (−57.00), `niit_threshold_crossing` (760.00), `full_worked_example`
    (2280.00, Mining-only), the double-count guard — all unchanged (no Interest in their fixtures).
  - **Disclosure KAT [R0-N1 — SEMANTIC, not substring-survivable]:** `contains("crypto-lending interest")`
    alone would pass on BOTH old and new text — assert the NEW phrase (e.g. `"is INCLUDED in NII"` /
    the exact new wording) AND `!contains("cannot yet isolate")`; the B-M1 negatives retained.

### Task 2 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: the WITH-only attribution (delta = the interest's NIIT, not cancelled); no MAGI
  double-count; mining/staking/airdrop/reward still excluded (the mixed golden); the B-M1 goldens byte-
  identical; SE untouched; the identity holds; exact Decimal; determinism; comments/disclosure accurate.
- FOLLOWUPS: mark the B-M1 "per-IncomeKind NII" deferral RESOLVED; note the residual §1411(c)(2)
  trade-or-business-exception nuance (not modeled, disclosed); next queue item = SE-tax completion.

## Out of scope
- The §1411(c)(2) trade-or-business exception modeling (disclosed as inapplicable); Form 8960 generation;
  any change to MAGI/§1222/§1211/SE/the identity; staking-as-NII (stays excluded); 2026/2027 tables.
