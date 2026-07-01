# SPEC — P2 B-M1: §1411 NIIT net-capital-loss correctness fix

**Source baseline:** `origin/main` @ `c55b91b`.
**Goal:** Fix engine B's §1411 Net Investment Income computation so a net capital loss reduces NII by
only the **§1211(b)-allowed amount (≤ $3,000 / $1,500 MFS)** — not by wiping other capital gains out of
NII. The current code omits this subtraction, which **OVERSTATES** NIIT in net-capital-loss years (the
opposite of the shipped "can only ever understate" flag). Also floor the NIIT base at $0, and correct the
disclosure. Tax-math change in engine B — hand-verified goldens required.

**SemVer:** behavior change to `compute_tax_year`'s NIIT figure in loss years ⇒ **MINOR** (pre-1.0; no
real users). No struct/API change.

## Primary-source determination (verified via web — see `reviews/` determination)
- **§1411(c)(1)(A)(iii)** + **26 CFR §1.1411-4(d)(2)** ("net gain may not be less than zero; losses
  allowable under §1211(b) are permitted to offset gain … subject to §1411") + **§1.1411-4(d)(3)(ii)
  Example 1** (P-stock loss $40k vs Q-stock gain $10k → net gain floored at 0 AND "A may reduce net
  investment income by the $3,000 … allowed for income tax purposes under §1211(b)"): **all dispositions
  NET together**; a net capital loss contributes to NII only the §1211-allowed loss (≤ $3k), NOT the
  other-category gains preserved. **Form 8960 line 5a** = Form 1040 line 7a (the Schedule D §1211-limited
  net) — confirms full netting, and line 5a flows in as a **negative**, reducing NII.
- **NIIT is floored at $0** (never negative/refundable): `3.8% × max(0, min(NII, MAGI − threshold))`.
- **crypto ordinary income** (`crypto_ord` = mining/staking/airdrops/rewards): NOT §1411 NII — business →
  SE income excluded by **§1411(c)(6)**; hobby → "other income" outside the §1411(c)(1)(A) categories.
  EXCLUDING it is correct. The ONLY NII slice in that bucket is crypto-**lending interest**
  (§1411(c)(1)(A)(i)) — excluding it understates, but the model can't yet distinguish it (deferred).

## Current-state (recon @ c55b91b — `crates/btctax-core/src/tax/compute.rs`)
- `nii_with = qd + with.ordinary_gain + with.preferential_gain` (~line 335); `nii_without` symmetric
  (~336). `ordinary_gain`/`preferential_gain` are the net ST/LT gains **clamped ≥ 0** (net_1222 ~167-168).
  In a net-loss year both clamp to 0 → `nii_with = qd` — the §1211-allowed loss is NEVER subtracted.
- `with.loss_deduction` (§1211-limited, ≤ limit) is already computed (net_1222 ~174-178) and already used
  in `bottom_with` (~323) + `crypto_agi`/MAGI (~338) — **MAGI is already correct**; only NII omits it.
- `niit = |nii, magi| { over = max(0, magi−thr); base = min(nii, over); round_cents(base*0.038) }`
  (~343-349) — **no floor at 0 on `base`**, so a negative NII would yield a negative NIIT.
- Disclosure (`render.rs` ~822-825 + compute.rs comments ~214,333): says NII "does not reduce NII by the
  allowed §1211 loss … so it MAY UNDERSTATE NIIT" — **directionally wrong** (it overstates in loss years).

## Design

### D1 — subtract the §1211-allowed loss from NII (both scenarios)
```
nii_with    = qd + with.ordinary_gain    + with.preferential_gain    - with.loss_deduction
nii_without = qd + without.ordinary_gain + without.preferential_gain - without.loss_deduction
```
Rationale: in a GAIN year `loss_deduction == 0` → unchanged (no regression). In a net-LOSS year the
gains are 0 and `loss_deduction` = min(net_loss, limit) → `nii = qd − loss_deduction`, i.e. NII reduced by
exactly the §1211-allowed loss (matching Form 8960 line 5a / Example 1). No extra floor is needed on the
disposition piece (`ordinary_gain`/`preferential_gain ≥ 0`, only the capped `loss_deduction` is negative,
so the disposition contribution ≥ −limit automatically).

### D2 — floor the NIIT base at $0
`base = max(Usd::ZERO, min(nii, over))` in the `niit` closure, so total NII going negative (e.g.
`qd < loss_deduction`) never produces a negative/refundable NIIT. (Latent for the headline golden where
NII_with = $2k > 0, but required for robustness.)

### D3 — correct the disclosure + comments (do NOT keep the wrong-direction claim)
Reword the `render.rs` footer + the compute.rs doc/inline comments to state the ACCURATE position:
"§1411 NII now reduces NII by the §1211(b)-allowed net capital loss (≤ $3k/$1.5k) — matching Form 8960
line 5a / §1.1411-4(d). Crypto ordinary income (mining/staking/airdrops/rewards) is correctly excluded
from NII (SE income per §1411(c)(6), or non-NII 'other income'); the ONLY residual understatement is
crypto-lending **interest** (NII under §1411(c)(1)(A)(i)), which the minimal model cannot yet distinguish
from other `crypto_ord` — a Phase-2 refinement." Remove the "can only ever understate" language.

### Decisions
- **Fix the loss-deduction subtraction (the real, dominant, verified bug); do NOT add `crypto_ord` to
  NII** (excluding mining/staking/airdrops is correct; only the lending-interest slice is NII, and the
  model can't isolate it — defer). This keeps the fix in the verified-correct direction.
- Standalone to the NIIT figure; no change to §1222 netting, loss-limit, carryforward, MAGI, or the
  capital-gains/ordinary tax.

## Plan (TDD)

### Task 1 — NII loss-deduction subtraction + NIIT base floor + goldens
- **Files:** `crates/btctax-core/src/tax/compute.rs` (nii_with/nii_without ~335-336; niit closure ~343-349).
- Implement D1 + D2. Hand-verified golden KATs:
  - **Loss-year (headline):** Single, thr $200k, qd $5,000, `other_net_capital_gain` +$15,000 (LT),
    `magi_excluding_crypto` $290,000, crypto net ST −$80,000, crypto_lt 0, crypto_ord 0, zero
    carryforward. **[R0-M1] `TaxResult` exposes only the DELTA `niit` (= niit_with − niit_without) and
    `total_federal_tax_attributable`** — so ASSERT `r.niit == dec!(-684.00)` (and the corresponding
    `total_federal_tax_attributable`); `nii_with $2,000` / `niit_with $76.00` / `niit_without $760.00` are
    derivation checkpoints in the test comment, NOT TaxResult-field assertions. (Pre-fix the code yields
    `r.niit == −570.00` — assert the NEW −684.00.)
  - **Gain-year regression:** an all-gains year (loss_deduction == 0) → `r.niit` UNCHANGED from pre-fix
    (subtracting 0 is a no-op) — pin an existing gain-year NIIT golden (e.g. `niit_threshold_crossing`
    760.00, `full_worked_example` 2280.00) and confirm it doesn't move.
  - **[R0-M2] NII-negative floor:** choose inputs where the WITHOUT-scenario NIIT is ALSO $0 so the
    observable DELTA `r.niit == dec!(0.00)` truly pins "never negative" (e.g. `magi_excluding_crypto`
    below the threshold in the without-scenario, qd small, a crypto net loss) — otherwise `r.niit` is a
    nonzero delta and doesn't test the floor. The D2 `max(0, …)` floor is what this pins: NII going
    negative must NOT produce a negative NIIT in either scenario.
  - **MFS $1,500 limit:** a loss-year MFS case → NII reduced by $1,500 (not $3,000).
- Verify existing NIIT KATs (gain-year) still pass unchanged.

### Task 2 — corrected disclosure + KAT
- **Files:** `crates/btctax-cli/src/render.rs` (the tax-report footer ~822-825), `crates/btctax-core/src/tax/compute.rs` (doc/inline comments ~214,333).
- Implement D3. KAT: the `Computed` tax-report footer NO LONGER contains "can only ever understate" /
  "does not reduce NII by the allowed §1211 loss"; it DOES state the §1211 loss is now applied + the
  residual crypto-lending-interest understatement caveat.

### Task 3 — whole-diff review (Phase E) + FOLLOWUPS
- Cross-cutting: the fix is in the §1411-verified direction (loss-year NIIT no longer overstated;
  hand-goldens match Form 8960 walk); gain years unchanged (no regression); NIIT never negative; §1222 /
  loss-limit / carryforward / MAGI / capital-gains tax untouched; determinism; exact Decimal.
- FOLLOWUPS: per-`IncomeKind` NII classification (add crypto-lending **interest** to NII; keep
  mining/staking/airdrops excluded) — the residual understatement slice — deferred to a later Phase-2 item.

## Out of scope
- Adding crypto ordinary income (mining/staking/airdrops) to NII (correctly excluded); the per-IncomeKind
  lending-interest NII slice (deferred); any change to §1222/§1211/§1212/MAGI/capital-gains math;
  Form 8960 generation; 2026/2027 tax tables.
