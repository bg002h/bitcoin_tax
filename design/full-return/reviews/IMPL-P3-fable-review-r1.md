# IMPL-P3 Fable review r1 — full-return Phase 3 (deductions: §63 std + Schedule A + §170(b) engine + std-vs-itemized)

- **Reviewer:** Fable (independent; author was a different model). Adversarial phase-boundary review gating P3.
- **Scope:** `git diff 4b10e4c..2614a51` (4b10e4c = P2-GREEN-certified). Commits `31310ec`, `2e3e4eb`, `2b1121e`,
  `e2beae6`, `7cb8d48`, `2614a51`. Files: `crates/btctax-core/src/tax/charitable.rs` (new, 309 lines),
  `crates/btctax-core/src/tax/return_1040.rs` (+365/−17), `crates/btctax-core/src/tax/return_refuse.rs` (+53),
  `crates/btctax-cli/src/resolve.rs` (2-line signature update), `design/full-return/FOLLOWUPS.md` (+22).
- **Read first:** plan Phase 3 (`design/IMPLEMENTATION_PLAN_full_return.md:139-153`), SPEC §4.6/§4.7/§5-stage-3/§6,
  recon `design/full-return/recon/deep/04-schedule-a-and-std-deduction-engine.md` §1/§2d/§3/§4, FOLLOWUPS P3 section.
- **Verdict:** **NOT GREEN** — **1 Critical / 2 Important / 4 Minor.**

---

## 0. Suite / lint / frozen-guard evidence (run locally at HEAD `2614a51`)

`cargo test --workspace` — every test binary green; totals across all binaries + doc-tests:
**1,483 passed / 0 failed / 1 ignored**. Representative real result lines (from the captured log):

```
test result: ok. 268 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 41.82s
test result: ok. 163 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 127 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.84s
test result: ok. 116 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.70s
test tax::frozen_guard::tests::frozen_engine_files_are_unchanged ... ok
test tax::charitable::tests::deep04_worked_example_cent_exact ... ok
test tax::return_1040::tests::derive_matches_deep02_example1_to_the_cent ... ok
```

`cargo clippy --workspace --all-targets` — clean:

```
    Checking btctax-tui v0.5.0 (/scratch/code/bitcoin_tax/crates/btctax-tui)
    Checking xtask v0.5.0 (/scratch/code/bitcoin_tax/crates/xtask)
    Checking btctax-tui-edit v0.5.0 (/scratch/code/bitcoin_tax/crates/btctax-tui-edit)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.94s
```

FROZEN files: `git diff 059ec2a..2614a51 -- crates/btctax-core/src/tax/types.rs
crates/btctax-core/src/tax/compute.rs crates/btctax-core/src/tax/se.rs` produced **0 bytes** (byte-identical),
and `frozen_guard::tests::frozen_engine_files_are_unchanged` passes (above).

**Reviewer probes.** I wrote a temporary integration test (`crates/btctax-core/tests/p3_review_probe.rs`,
deleted after the run — working tree back to clean) driving the live `apply_170b` / `standard_deduction` with
adversarial inputs. Result: `4 passed; 0 failed` — i.e. all four suspected failure modes **reproduce on the
shipping code**. The probe values are quoted verbatim in the findings below.

---

## 1. Independent cent-exact re-derivations

### 1.1 deep/04 §3 worked example (charitable engine)

MFJ, AGI $200,000; gifts $5,000 `Cash60` + $70,000 `CapGainProp30`; no carryover-in; year 2024. Worksheet-2
order, re-derived by hand against `apply_170b` (`charitable.rs:96-176`):

| Step | Class | Ceiling | Allowed | Carryover-out |
|---|---|---|---|---|
| 1 | Cash60 | 0.60·200,000 = **120,000.00** | min(5,000, 120,000) = **5,000.00** | — |
| 2 | OrdinaryProp50 | max(0, 100,000 − 5,000) = 95,000.00 | 0 (no gifts) | — |
| 3 | CapGainProp30 | min(0.30·200,000 = **60,000.00**, max(0, 100,000 − 5,000 − 0) = 95,000.00) = **60,000.00** | min(70,000, 60,000) = **60,000.00** | 70,000 − 60,000 = **10,000.00**, class CapGainProp30, origin 2024 |

Line 11 = $5,000.00; line 12 = $60,000.00; line 13 = $0; line 14 = **$65,000.00**; carryover_out =
`[(CapGainProp30, 10,000.00, 2024)]`. Matches the recon table (deep/04 §3) and the engine KAT
(`deep04_worked_example_cent_exact`) to the cent. **Verified.**

### 1.2 R2-I1 two-term 30%-cap

`charitable.rs:135`: `cgp30_ceiling = min(30%·AGI, nonneg(50%·AGI − allowed_cash_tier − allowed_ord_tier))`,
where both tiers include their **carryover-allowed** amounts — exactly SPEC §4.6's "allowed 60%/50%-tier
contributions this year, INCLUDING allowed ordinary-income-property amounts" and the deep/04 §5/:190 formula
(not the naive `50%·AGI − cash`). KAT `thirty_percent_class_capped_by_overall_fifty_room` pins
min($30k, $100k·50% − $40k) = $10k. **Verified** (for the 50%-org classes; the non-50%-org side is C1 below).

### 1.3 Carryover mechanics

- **Current-first, then carryover:** `allocate_class` (`charitable.rs:56-91`) allows current-year gifts to the
  ceiling, then walks carryover-in — matches Pub 526 ("deduct carryover only after current-year contributions
  in that category"). KAT `carryover_in_consumed_after_current_year` ($50k current + $20k 2022 carryover at a
  $60k ceiling → line 13 = $10k, out = $10k@2022). **Verified.**
- **Oldest-vintage-first:** `carry_for` sorts by `origin_year` (`charitable.rs:113-119`, stable sort). KAT
  `oldest_vintage_consumed_first` ($10k@2020 fully used before $10k@2023). **Verified.**
- **5-year expiry boundary:** `is_expired = year − origin > 5` (`charitable.rs:41-43`). §170(d)(1) gives the 5
  succeeding years: origin 2019 → usable through 2024 (`year−origin == 5` usable); origin 2018 → expired in
  2024 (`== 6`). KAT `expired_carryover_is_dropped` pins both sides of the boundary. **No off-by-one.**
- **Vintage preservation + G8 std-year aging:** unused carryover-in survives with its original `origin_year`;
  the engine is callable in a std year (KAT `kat13_carryover_ages_across_a_std_year`). Engine-level **verified**
  (derive-level wiring gap → M2).

### 1.4 Standard deduction

- **Age-65 boundary** (`is_aged`, `return_1040.rs:36-41`): cutoff = Jan 1 of `year−64` (TY2024 → 1960-01-01),
  `d <= cutoff`. Matches Pub 501 "born before January 2, 1960". KAT `aged_boundary_and_none_dob` pins
  1960-01-01 → $16,550 and 1960-01-02 → $14,600. **Verified.**
- **`None` DOB** → not-aged. Direction check: not-aged ⇒ smaller std ⇒ higher tax ⇒ **conservative/fail-closed**
  — the right default (never grant an unsubstantiated §63(f) box), and it honors the `p1-r1-m3-dob-option-pin`
  prohibition (never a silent synthetic birthdate). Fail-loud refusal would be over-strong: the forfeited
  benefit is taxpayer-adverse, never IRS-adverse, and the P6 header checkbox derived from the same `None` stays
  unchecked, so the filed return is internally consistent. Ruling: **correct as built**; add a P5 advisory (M4).
- **Dependent floor** (`return_1040.rs:77-84`): `min(basic, max($1,300, earned + $450))`, aged/blind added ON
  TOP (recon §1.3's two KAT-locked invariants both pinned: `dependent_floor` asserts $1,300 / $5,450 /
  $14,600-cap / $3,250 dependent+blind). **Verified** (earned-income input completeness → M3; spouse flag → I1).
- **QSS:** basic via `Qss → Mfj` ($29,200) and the **married** $1,550 box rate
  (`uses_married_aged_blind_rate` includes Qss — Rev. Proc. 2023-34 §3.15(3): $1,950 only if "unmarried and
  **not a surviving spouse**"). KAT `qss_uses_married_basic_and_aged_blind_rate` = $30,750. **Verified.**
- **MFJ both-spouses boxes:** taxpayer + spouse each contribute up to 2 boxes on MFJ only. KAT pins
  $29,200 + 2·$1,550 = $32,300. MFS spouse-boxes conservatively never counted — matches recon §1.3's
  parenthetical ("model as taxpayer-only for MFS unless that flag is set"; no such flag exists) and is
  documented at the fn doc. Overstates tax only. **Acceptable.**
- **TY2024 params** (`btctax-adapters/src/tax_tables.rs:118-131`): 14,600/29,200/21,900; 1,550/1,950;
  1,300/+450; SALT 10,000. **Verified against Rev. Proc. 2023-34.**

### 1.5 Schedule A + std-vs-itemized

- **Medical:** `max(0, medical − 7.5%·AGI)` (`return_1040.rs:134`); KAT pins $10,000 at AGI $100k → $2,500.
- **SALT either/or** (`salt_line_5a`, `return_1040.rs:107-118`): election ON → sales-tax amount ONLY (KAT shows
  $9,999 of estimated payments ignored); OFF → Σ box17 + Σ box19 + estimated + prior-year balance. Cap $10k,
  **$5k MFS** (`salt_cap / 2`; KAT pins $20k real-estate → $5,000). Election-ON-with-income-tax-facts is
  legitimately silent (the §164(b)(5) election *is* "instead of"); the reverse direction fails loud (below).
- **R3-M9 fail-loud:** `SaltSalesTaxWithoutElection` (`return_refuse.rs:318-328`) refuses
  `salt_sales_tax_amount > 0 && !salt_use_sales_tax`; KAT `salt_sales_tax_without_election_refuses`. **Verified.**
- **G15:** `MfsSpouseItemizeUnknown` (`return_refuse.rs:330-339`) refuses MFS + `None`; KAT present. The screen
  runs **before** `derive_tax_profile` in the single resolver ladder (`btctax-cli/src/resolve.rs:95-107`), so a
  `None` tri-state can never reach `choose_deduction` silently. **Verified.**
- **choose_deduction** (`return_1040.rs:153-168`): `max(std, itemized)` on Auto; ForceItemize → itemized
  (§63(e)); MFS + `mfs_spouse_itemizes == Some(true)` → std = $0 **before** the match, so: Auto picks
  `max(0, itemized)` (correct — their std *is* zero) and ForceItemize is unaffected (correct — electing to
  itemize is what §63(c)(6) wants). The suspected ForceItemize/MFS-$0 interaction bug is **not present**. KATs
  `derive_uses_max_of_std_and_itemized`, `force_itemize_uses_schedule_a_even_when_smaller`,
  `mfs_spouse_itemizes_forces_zero_std` all pin taxable income end-to-end through `derive_tax_profile`.
- **deep/02 Ex.1 unchanged:** `derive_matches_deep02_example1_to_the_cent` passes with identical asserts
  (246,800 / 287,000) — the Ex.1 household has no Schedule A and no DOB/blind/dependent flags, so the full-std
  path reduces to the P2 basic std. **Verified cent-exact, no drift.**

---

## 2. Findings

### C1 (Critical) — non-50%-org ceilings omit the statutory cross-class terms; the recorded "conservative" claim is FALSE — the engine silently UNDERSTATES tax

`charitable.rs:140` gives `Cash30` a flat `30%·AGI` ceiling and `charitable.rs:150-156` caps `CapGainProp20`
at `min(20%·AGI, 30%·AGI − cash30 − ord30)`. Both omit the cross-terms the statute makes mandatory:

- **§170(b)(1)(B)(ii)** — the non-50%-org 30% classes are capped at the lesser of 30%·AGI or **50%·AGI minus
  the contributions allowed to 50%-orgs** (the Cash60/OrdinaryProp50/CapGainProp30 tiers). The impl never
  subtracts the 50%-org tiers.
- **§170(b)(1)(D)(i)(II)** — CapGainProp20 is capped by **30%·AGI minus the §170(b)(1)(C) capital-gain
  contributions (the CapGainProp30 class)**, not by the non-50%-org cash/ordinary usage the impl subtracts.

deep/04 §2d itself states the ordering rule the impl violates: *"each later class is limited by AGI room the
earlier classes leave"*, and its class table cites §170(b)(1)(B)/(D) for exactly these rows. This was **not**
settled by any earlier gate — SPEC §4.6 specifies only the 50%-org-side R2-I1 formula; the own-% shortcut is a
P3 implementation invention.

**Live reproduction (reviewer probe, real output — all against the shipping engine):**

```
test probe_cash30_ignores_50pct_org_room ... ok    // AGI $100k, $50k Cash60 + $30k Cash30:
                                                   //   law allows $50,000; ENGINE ALLOWS $80,000 (+$30,000)
test probe_cgp20_ignores_cgp30_usage ... ok        // AGI $100k, $30k CGP30 + $20k CGP20:
                                                   //   law allows $30,000; ENGINE ALLOWS $50,000 (+$20,000)
```

Concrete failure scenario: an itemizing donor gives $50k cash to a church and $30k cash to a veterans
organization / private foundation (a `Cash30` input SPEC §4.6 explicitly captures). Schedule A line 14 is
overstated by $30,000 → taxable income understated by $30,000 → **tax understated ~$7,200 at 24%** — silent,
no refuse row, no advisory (`grep Cash30\|OrdinaryProp30\|CapGainProp20 return_refuse.rs` → nothing). The
totals can reach 90%·AGI (60% + 30%) where the law caps at 60%/50%.

The FOLLOWUPS deferral `p3-non50org-charitable-special-limit` records this as "bounded + **conservative**".
The conservative half is factually wrong — this is the fail-open direction — so the deferral cannot stand as
justification at this gate (the workflow's fail-closed posture is precisely "never silently understate tax").

**Fix (either closes the finding):**
(a) implement the two missing subtraction terms — same shape as the already-shipped R2-I1 line
(`cash30`/`ord30` ceiling gains `.min(nonneg(pct(0.50) − allowed_cash_tier − allowed_ord_tier − cgp30_allowed))`,
`cgp20` subtracts the CGP30 tier per §170(b)(1)(D)(i)(II)); or
(b) **refuse** (fail-loud) when any gift or carryover-in carries a non-50%-org class, keeping the classes
capture-only until the precise Worksheet-2 interaction ships. Add KATs pinning both probe scenarios either way.

### I1 (Important) — `can_be_claimed_as_dependent_spouse` is captured but never consumed: MFJ dependent-spouse returns get the full basic std (understates tax)

`return_inputs.rs:167` defines the flag; a workspace grep shows **zero consumers** — `standard_deduction`
keys the §63(c)(5) floor solely on `can_be_claimed_as_dependent_taxpayer` (`return_1040.rs:77`), and the P2
kiddie screen (`return_1040.rs:305`) likewise ignores the spouse flag. The 1040 Standard Deduction Worksheet
for Dependents triggers on "Someone can claim: **you** as a dependent [or] **your spouse** as a dependent" —
on MFJ the spouse box activates the limited basic amount.

**Live reproduction (reviewer probe):**

```
test probe_dependent_spouse_flag_unconsumed ... ok // MFJ, spouse claimable-as-dependent, $0 earned:
                                                   //   ENGINE std = $29,200 (full basic); worksheet limits it
```

Failure scenario: MFJ return with `can_be_claimed_as_dependent_spouse = true` → std overstated by up to
~$27,900 → tax understated. Mitigation on severity: the legally-consistent input space is narrow (the
joint-return test means a claimable spouse usually implies a refund-only return), and the recon/spec share the
gap (deep/04 §1.2 lists the checkbox as consumed input, but §1.3's pseudocode and SPEC §4.7 drop it — an
upstream erratum this review also flags). But the flag is capturable via `income import` **today** and is
silently ignored where it changes the tax number — the same input-error-hiding shape R3-M9 was written to
refuse. **Fix:** consume it (extend the §63(c)(5) trigger to taxpayer-OR-spouse on MFJ, MFJ earned income =
household Σ) **or** refuse when the flag is set; record the spec/recon erratum either way.

### I2 (Important) — `p2-pref-over-ti-clamp` was SCHEDULED → P3 "with the full deduction stack"; P3 shipped the deduction stack without it and without re-recording

FOLLOWUPS (`p2-pref-over-ti-clamp`): "Exact fix … lands in **P3 with the full deduction stack**". The strip
site's own comment (`return_1040.rs:411-416`) still promises "lands in P3 with the full deduction stack". P3
delivered the full deduction stack (this diff) with **no clamp and no re-scheduling entry** — the P3 deferrals
commit (`2614a51`) records other deviations but is silent on this one. The underlying number is
conservative-only (reconstructed TI ≥ true TI ⇒ delta can only overstate — twice-reviewed Minor), so this is
not a wrong-refund hazard; but Schedule A makes the `TI < qd + cap_gain_distr` region *more* reachable (larger
deductions eat the ordinary base first), and a P3-gate review that let a P3-scheduled correctness item vanish
without a record would break the workflow's deviation-traceability spine. **Fix:** implement the min-cap now,
or update both the FOLLOWUPS entry and the code comment with an explicit re-schedule (P4) + justification.

### M1 (Minor) — negative AGI produces negative ceilings, negative "allowed", and a corrupt inflated carryover_out

No ceiling in `apply_170b` clamps the base `pct(...)` terms at zero (only the subtraction terms are
`nonneg`-wrapped), and `allocate_class` lets `current_allowed = min(total, ceiling)` go negative — including
for **empty** classes. Probe (real output):

```
test probe_negative_agi_corrupts_carryover ... ok  // AGI −$10k, $5k Cash60:
                                                   //   allowed_cash = −$9,000.00 (Cash60 −6k + EMPTY Cash30 −3k)
                                                   //   carryover_out = $11,000.00 — MORE than the $5k gift
```

Reachable: negative non-crypto AGI survives the screens (Sch C losses refuse, but an early-withdrawal penalty
exceeding a small income does not). Harmless **in P3** — taxable income floors at `max(0, agi − deduction)`
with `agi < 0`, and `carryover_out` is discarded — but the inflated carryover is a live corruption hazard for
the P4 write-back, and the negative-allowed Schedule A line would render nonsense in P6. Same for the medical
floor (`medical − 7.5%·negative` inflates the deduction). **Fix before P4 wires `carryover_out`:** clamp
AGI (or every ceiling) at zero at the top of `apply_170b` + a negative-AGI KAT; note it inside the
`p3-carryover-writeback-P4` entry so P4 cannot wire the write-back without it.

### M2 (Minor) — G8 std-year aging is engine-supported but the derive never runs the engine when `schedule_a` is `None`

`return_1040.rs:402-404` calls `apply_170b` under `ri.schedule_a.as_ref().map_or(...)` — a filer with
`charitable_carryover_in` but no Schedule A block never runs the engine, so nothing ages/reduces (and the
carryover contributes nothing). Harmless in P3 (carryover_out discarded; `charitable.rs`'s module doc already
states the caller "always writes carryover_out" — aspirational until P4), but the P4 wiring must hoist the
engine call out of the `schedule_a` map or G8 (Reg. §1.170A-10(a)(2)) silently fails for std-deduction years.
KAT-13's std-year scenario is currently pinned only at engine level, not through the derive. Record in the
`p3-carryover-writeback-P4` entry.

### M3 (Minor) — dependent-floor earned income = household wages only; the G21 formula (− Sch C net − ½SE) completion is undocumented

SPEC §4.7/G21: dependent earned income = "Σ box1 + Schedule C net − ½SE". The derive passes `wages` only
(`return_1040.rs:401`). Direction is right for now: including non-crypto Sch C net **without** the P4-deferred
½-SE would overstate earned → overstate the floor → understate tax, so wages-only is the conservative interim,
and the fn doc documents the delta-vs-absolute split. But no FOLLOWUPS entry says the G21 completion lands
with P4's ½-SE — add one line (a dependent with a Schedule C currently gets a conservatively low floor with no
recorded reason).

### M4 (Minor) — `None` DOB silently forfeits the §63(f) benefit; surface a P5 advisory

Direction verified correct (conservative — see §1.4). Recommend the P5 advisories work add "DOB not on file —
if 65+, you are forfeiting $1,550/$1,950 per box" so the conservative default is visible rather than silent.
Non-blocking.

---

## 3. Deferral / design-Q assessment

### 3.1 `p3-carryover-writeback-P4` — **ACCEPTABLE** (with two riders)

Nothing persists: the derive-side `carryover_out` is computed and discarded, so no wrong carryover can reach
storage or a later year — genuinely non-fail-open. The *real* filed carryover needs the crypto-donation excess,
which needs the absolute Schedule A (a P4 piece by the certified `p2-absolute-assembly-deferred-to-P4`
precedent), so persisting the non-crypto figure in P3 would persist a number known to be wrong for anyone with
crypto donations — worse than deferring. P3 need not persist anything. **Riders:** (i) the entry's claim that
"`apply_170b` already computes `carryover_out` correctly" is overstated — C1 (non-50%-org classes) and M1
(negative AGI inflation) must be fixed before P4 trusts it; (ii) P4 must hoist the engine call for G8 std-year
aging (M2). Fold both riders into the entry text.

### 3.2 `p3-l16-absolute-P4` — **ACCEPTABLE**

L16 is an absolute-return line; its only consumers (the §6 dual report, the P6 1040 filler) are P4/P6. The
frozen-DELTA path P3 ships is complete without it — the derived `TaxProfile` feeds `compute_tax_year`, whose
own QDCGT computes the delta; no code path prints a wrong or stubbed L16 (nothing fail-open). Building L16 in
P3 would be consumer-less stub code, the exact shape the P2 gate already certified as a valid deferral. The
plan's P3 acceptance line "L16 golden vs method.rs" moves with it; P4's acceptance must inherit that KAT plus
the plan's "Schedule A reads with-crypto AGI" KAT (G7 — absolute side).

### 3.3 `p3-crypto-donation-delta-integration` (open design Q) — **derive-side exclusion is CORRECT**; ruling + direction analysis

The question: the derive excludes crypto donations from the deduction while the frozen engine adds crypto
INCOME on top — is the delta wrong?

**(a) Donation exclusion can only OVERSTATE the reported tax (conservative).** `apply_170b`'s allowed total is
monotone nondecreasing in gifts: adding crypto `CapGainProp30` gifts only raises that class's allowed; adding
crypto `OrdinaryProp50` gifts raises the ordinary tier by δ while shrinking the cgp30 ceiling by at most δ
(the R2-I1 subtraction), so the total never falls. Hence derived-deduction (no crypto gifts) ≤ true-deduction
(with them) ⇒ derived TI ≥ true TI ⇒ the composed number never understates. The std-vs-itemized flip case
(crypto donation would have tipped `max()` to itemizing) is the same inequality — std ≤ true itemized total.
The existing §170(e) advisory remains the user-visible signal of the forfeited planning benefit.

**(b) Non-crypto AGI for the derived Schedule A is architecturally forced, not a P3 choice to relitigate.**
`TaxProfile` scalars are contractually "EXCLUDING app-computed crypto" (the frozen seam); a with-crypto AGI
inside the derived deduction would contaminate `tax(base)` so it no longer equals the no-crypto counterfactual
— the delta's baseline definition. SPEC §6 says it plainly: "the delta path's deduction is **fixed at
derivation time**… the report documents the delta deduction as approximate."

**(c) The one residual anti-conservative channel is the medical floor — known, documented, P4-mitigated.**
With medical inputs + crypto gains, the true return's 7.5% floor at with-crypto AGI shrinks the medical
deduction; the frozen delta cannot re-shrink a derivation-fixed deduction, so the delta UNDERSTATES the true
crypto-attributable tax there (e.g. wages $50k / medical $20k / crypto +$100k: derived medical $16,250 vs true
$8,750 — $7,500 of deduction the delta never claws back). This is exactly SPEC §6's documented AGI-sensitive
approximation; it is not new in P3 (a P2-era hand-entered profile had identical fixed-deduction semantics),
and the charitable ceiling's AGI-sensitivity points the safe way (lower AGI ⇒ lower ceiling ⇒ overstated tax).
**Ruling:** exclusion stands; keep the FOLLOWUP open for P4, where the crypto donations MUST enter the
**absolute** Schedule A (ledger §170(e) classes at with-crypto AGI, G7), and require P4's
`absolute_with − absolute_without ≠ delta` KAT (plan P4 task 8) to use a **medical-floor** fixture so the one
anti-conservative direction is the one pinned.

---

## 4. Plan-acceptance cross-check (Phase 3)

| Plan acceptance item | Status |
|---|---|
| Charitable worked example (deep/04 §3) to the cent | **PASS** (§1.1; KAT `deep04_worked_example_cent_exact`) |
| KAT-17 same-year ST+LT crypto donation ceiling | Engine-level PASS (`kat17_same_year_short_and_long_crypto`); ledger-fed supply deferred with §3.3 |
| KAT-13 std-year carryover | Engine-level PASS (`kat13_carryover_ages_across_a_std_year`); derive-level wiring → M2 |
| Carryover write-back round-trips two years | DEFERRED → P4 (`p3-carryover-writeback-P4`) — accepted §3.1 |
| L16 golden vs `method.rs` | DEFERRED → P4 (`p3-l16-absolute-P4`) — accepted §3.2 |
| deep/02 Ex.1 unchanged cent-exact | **PASS** (`derive_matches_deep02_example1_to_the_cent`, asserts identical) |
| FROZEN guard green | **PASS** (0-byte diff vs `059ec2a` + guard test) |

## 5. Verdict

**NOT GREEN — 1 Critical / 2 Important / 4 Minor.** The 50%-org spine of the charitable engine, the full §63
standard deduction, Schedule A, the std-vs-itemized choice, both new refuse rows, and all three deferral
postures verify clean — but C1 is a silent tax-understatement on a v1-capturable input with a falsely-justified
deferral, and I1/I2 are fail-closed/traceability breaches at the exact gate this review exists to hold. Fix
C1 + I1 + I2 (fold-or-refuse; re-record), then re-review.
