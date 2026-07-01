# R0 Architect Review — SPEC_p2_bm1_niit_loss (round 1)

**Artifact:** `design/SPEC_p2_bm1_niit_loss.md`
**Baseline:** HEAD `c55b91b` (verified: `git rev-parse HEAD` → `c55b91bb…`).
**Reviewer role:** independent architect gate. This change alters a TAX FIGURE (NIIT in
net-capital-loss years), so a wrong §1411 rule, a wrong fix direction, a gain-year regression, or a
non-reproducing golden is **Critical**.

**VERDICT: 0 Critical / 0 Important.** 2 Minor, 1 Nit (all non-blocking). **Gate: PASS —
implementation may proceed.**

---

## A. Recon citations verified against current source (HEAD c55b91b)

All line-number citations in the spec are accurate at HEAD:

| Spec claim | Source | Status |
|---|---|---|
| `nii_with = qd + with.ordinary_gain + with.preferential_gain` (~335); `nii_without` symmetric (~336) | `compute.rs:335-336` | ✅ exact |
| `niit` closure `over/base/round_cents` with NO floor on `base` (~343-349) | `compute.rs:343-347` | ✅ exact (`base = if nii < over { nii } else { over }`, no `max(0,…)`) |
| `loss_deduction` = §1211-limited, ≤ limit (~174-178) | `compute.rs:174-178` | ✅ exact |
| `crypto_agi` / MAGI already subtracts `loss_deduction` (~338) | `compute.rs:338-342` | ✅ exact — **MAGI is already correct; only NII omits the subtraction** |
| Disclosure "does not reduce NII … MAY UNDERSTATE NIIT" | `render.rs:822-825` | ✅ exact (verbatim "so it MAY UNDERSTATE NIIT") |
| Wrong-direction inline comment "this can only ever understate NIIT" | `compute.rs:333-334` | ✅ exact; doc comment `compute.rs:214` also present |
| Reported `niit` field = `niit_with − niit_without` (delta) | `compute.rs:372` | ✅ confirmed |
| `loss_limit(Mfs)=1500`, else `3000` | `tables.rs:144-149` | ✅ confirmed (MFS golden sound) |

**Structural finding that underpins the whole fix (verified by reading `net_1222`,
`compute.rs:141-193`):** after §1222 cross-netting, `ordinary_gain`/`preferential_gain` (clamped ≥0)
and `loss_deduction` (>0 only when `net_loss>0`) are **mutually exclusive** — a year is EITHER net-gain
(`loss_deduction==0`, gains ≥0) OR net-loss (gains==0, `loss_deduction` = min(net_loss, limit)). This is
exactly what makes D1 both correct in loss years and a no-op in gain years.

---

## B. INDEPENDENT web verification of the §1411 net-loss rule (re-derived from primary source — did
NOT trust the spec)

### 1. Net-capital-loss NII treatment — CONFIRMED, and the spec's direction is RIGHT.

- **26 CFR §1.1411-4(d)(2)** (fetched, law.cornell.edu, verbatim): *"The calculation of net gain may not
  be less than zero. Losses allowable under section 1211(b) are permitted to offset gain from the
  disposition of assets other than capital assets that are subject to section 1411."* → net gain FLOORED
  at zero, and the §1211(b)-allowed loss is separately permitted.
- **26 CFR §1.1411-4(d)(3)(ii) Example 1** (fetched, verbatim): unmarried individual, **$40,000 capital
  loss** (P stock) + **$10,000 capital gain** (Q stock) → **$30,000 net capital loss**; under §1211(b)
  only **$3,000** is allowed; **"A may reduce net investment income by the $3,000."** The $10,000 gain
  does NOT survive in NII (it is fully offset by the loss); the disposition line contributes **−$3,000**.
- **Form 8960 line 5a instructions** (fetched, IRS i8960): line 5a = combine **Form 1040 line 7a**
  (Schedule D net) + Schedule 1 line 4; it is a signed net that can be **negative** and flows in reducing
  NII. So NII sees the Schedule-D §1211-limited net, as a negative — matching Example 1.

**Conclusion (my own derivation, agrees with the spec):** (a) all dispositions NET together — a big loss
in one category offsets gains in another, and the other gains do NOT survive in NII; (b) a net capital
loss reduces NII by only the §1211(b)-allowed amount (≤ $3,000 / $1,500 MFS); (c) line 5a = the §1211-
limited Schedule-D net, entering as a negative. Therefore the CORRECT `nii` subtracts `loss_deduction`,
and the **current code (which omits it) OVERSTATES NIIT in loss years.** The shipped "can only ever
understate" flag is **directionally wrong**; the spec's fix (subtract `loss_deduction`, reducing NII)
moves in the **verified-correct direction.** ✔

### 2. NIIT floored at $0 — CONFIRMED.

**§1411(a)(1)** (fetched, verbatim summary): tax = 3.8% × the **LESSER** of NII or (MAGI − threshold). A
tax is never negative/refundable, so when NII goes negative (`qd < loss_deduction`) the base must floor
at $0. `base = max(0, min(nii, over))` is correct. (`over` is already clamped ≥0 at `compute.rs:344`, so
only `nii<0` can drive it negative — handled.) ✔

### 3. Crypto ordinary-income NII status — CONFIRMED; spec correctly does NOT add `crypto_ord` to NII.

- **§1411(c)(1)(A)** (fetched, verbatim): the only NII categories are (i) interest/dividends/annuities/
  royalties/rents (non-business), (ii) passive-activity or financial-instrument/commodity-trading income,
  (iii) net gain from disposition of property.
- **§1411(c)(6)** (fetched, verbatim): NII excludes "any item taken into account in determining
  self-employment income … on which a tax is imposed by section 1401(b)."
- Mining/staking/airdrops/rewards are ordinary income that is EITHER SE income (business → Schedule C →
  excluded by §1411(c)(6)) OR non-NII "other income" (hobby → outside all (c)(1)(A) categories). Either
  way **not NII** → keeping them excluded is **correct**. Only crypto-**lending interest** is NII under
  §1411(c)(1)(A)(i); the model can't yet isolate it from `crypto_ord`, so deferring is the right call and
  the sole residual understatement. The spec does **not** wrongly add `crypto_ord` to NII. ✔

Thresholds cross-checked against statute: MFS $125k / Single $200k / MFJ,QSS $250k = `tables.rs`. ✔

---

## C. Headline golden — hand-reproduced independently (PASS)

Inputs: Single, thr $200k; qd $5,000; `other_net_capital_gain` +$15,000 (LT);
`magi_excluding_crypto` $290,000; crypto ST −$80,000; crypto_lt 0; crypto_ord 0; zero carryforward.

- **WITH** `net_1222(-80000, 0, +15000, 0,0, 3000)`: st_net −80000, lt_net +15000 → cross-net (ST loss,
  LT gain), −st_net 80000 > lt_net 15000 → (st2,lt2)=(−65000, 0). ordinary_gain 0, preferential_gain 0,
  net_loss 65000, **loss_deduction 3000**. → **nii_with = 5000 + 0 + 0 − 3000 = $2,000.** ✔
- crypto_agi = (0+0−3000) − (0+15000−0) + 0 = −18000 → **magi_with = 290000 − 18000 = $272,000.**
  over = 72000. base = min(2000,72000)=2000 (floor n/a). **niit_with = 0.038×2000 = $76.00.** ✔
- **WITHOUT** `net_1222(0,0,+15000,…)`: preferential_gain 15000, loss_deduction 0 → nii_without =
  5000+15000 = 20000; magi_without 290000; over 90000; base 20000. **niit_without = $760.00.** ✔
- **Reported niit (delta) = 76.00 − 760.00 = −$684.00.** ✔ (Pre-fix: nii_with=$5,000 → niit_with=$190
  → delta −$570; the fix reduces the WITH-scenario level $190→$76, i.e. it FIXES an overstatement.)

Every figure the spec lists reproduces exactly from the independently-verified rule.

## D. No gain-year regression — CONFIRMED

Both shipped NIIT goldens are pure gain-year cases (no capital losses ⇒ `loss_deduction==0` in both
scenarios ⇒ D1's `− loss_deduction` is a no-op):
- `tax_compute.rs:238 niit_threshold_crossing` (LT gain +30k) → `r.niit == 760.00` unchanged. ✔
- `tax_compute.rs:276 full_worked_example…` (ST +10k, LT +50k, mining +10k) → `r.niit == 2280.00`
  unchanged (re-derived: NII_with 80k, over 270k → 3040; NII_without 20k, over 200k → 760; Δ 2280). ✔

The NII-negative floor (D2) is inert for all gain years (`nii ≥ qd ≥ 0`); MFS uses `loss_limit(Mfs)=1500`
automatically. Both extra goldens (negative-floor, MFS-$1,500) are sound.

## E. Scope — CONFIRMED bounded

Fix touches only `nii_with`/`nii_without` (D1), the `niit`-closure base floor (D2), and the disclosure/
comments (D3). §1222 netting, §1211/§1212 loss-limit + carryforward, MAGI (`crypto_agi` already subtracts
`loss_deduction`), and the §1 ordinary / §1(h) preferential tax are all untouched. D3 correctly removes
the wrong-direction "can only ever understate" language and states the accurate position (§1211 loss now
applied; crypto ordinary income correctly excluded; residual = crypto-lending-interest slice). Tasks are
right-sized and TDD (failing goldens first). No struct/API change; SemVer MINOR is appropriate.

---

## Findings

### Minor
- **M-1 (test-target clarity).** The headline golden lists internal locals (`nii_with`, `niit_with`,
  `niit_without`) that are **not** fields on `TaxResult`. The only public, assertable output is
  `r.niit` (the delta) and `r.total_federal_tax_attributable`. The KAT must assert
  `r.niit == dec!(-684.00)` (and the total), treating nii_with/niit_with/niit_without as hand-derivation
  checkpoints, not assertions. Recommend the plan state this explicitly so no one tries to assert a
  non-existent field. (Non-blocking — the delta is fully assertable and reproduces.)
- **M-2 (assert the level, not just the delta, for the negative-floor case).** The NII-negative golden
  proves the floor via `r.niit`, which is a DELTA (`niit_with − niit_without`); with qd=$1,000 and MAGI
  over threshold, `niit_without ≈ $38`, so `r.niit` would be `≈ −$38`, NOT `$0`. The spec's phrasing
  "`niit_with == $0.00`" refers to the internal WITH-scenario level, which isn't exposed. To actually
  pin "never negative," choose inputs where `niit_without == 0` too (e.g. `magi_excluding_crypto` below
  threshold in the WITHOUT scenario, or qd such that the without-scenario NII/over is 0) so the observable
  `r.niit == dec!(0.00)`; OR assert on a scenario where the WITH level dominates. Otherwise the golden as
  literally worded ("niit_with == $0.00") isn't directly checkable. (Non-blocking; a test-construction
  detail for the plan — the underlying D2 floor is correct.)

### Nit
- **N-1.** D3 should also refresh the `compute.rs:214` doc comment ("NII is `QD + surviving net capital
  gains (ST+LT)`") to mention the §1211 subtraction, not just the `:333` inline comment. The spec's Task
  2 already says "doc/inline comments ~214,333," so this is just a reminder to hit both.

None of the above blocks the gate. The §1411 rule is correct, the fix direction is correct (fixes an
overstatement), no gain-year golden moves, and the headline golden reproduces exactly.
