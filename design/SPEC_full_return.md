# SPEC — Full Return (Common W-2 Household, TY2024) — v1

**Status:** ✅ GREEN r4 (opus-authored; folded Fable reviews r1+r2+r3; user decisions D-3/D-5/D-6 of
2026-07-12). **Fable re-review r4 = GREEN — 0 Critical / 0 Important / 5 Minor** (Minors → `FOLLOWUPS.md`).
Reviews: `design/full-return/reviews/SPEC-fable-review-r{1,2,3,4}.md`. → pending user review; plan may proceed.
**Governs:** `STANDARD_WORKFLOW.md` (spec → plan → implement, each review-to-green).
**Grounded in:** `design/full-return/recon/` — opus `00`–`05`, `deep/01`–`05`, Fable `01`–`06` +
`fable/00-SYNTHESIS-FABLE.md`. Verify citations against current source at plan time.

**r4 changelog (Fable r3 fold):** R3-I1 restored the 8959 Part-II inner clamp + both 8960 floors (§5 stage 7);
R3-I2 the complete Schedule B filing trigger incl. ordinary-dividends>$1,500 (§7.1); R3-I3 business-flagged
crypto Interest → refuse-guard (§4.4a/§4.10); R3-I4 scoped the reduce-to-delta invariant (SE MAGI wedge, §5
tail); R3-M1–M10 folded (Schedule C fill fields + §6017 $400 floor + SE box-7 tips + 8960 L7 home + write-back
precedence + phase de-dup + aligned LIMITATIONS lists + SALT election box).

**r3 changelog:**
- **Decisions:** D-6=(c) **Schedule C + SE in v1** (business/self-employment crypto income files; full Form
  8959 Part I+II+V; ½-SE; Schedule SE + Schedule C fillers). D-3=(a) **§904(j) foreign-tax credit** (≤$300/$600
  passive+1099-reported → Sch 3 L1). D-5=(a) **one spec, phased plan**.
- **r2 Importants:** R2-I1 charitable 30%-cap-gain ceiling restored to `min(30%·AGI, 50%·AGI − 50%-tier
  allowed)` (§4.6); R2-I2 Schedule D routing made exhaustive incl. ST-gain/LT-loss and L16=0 (§7.2);
  R2-I3 Schedule B force-files on foreign account/trust + Form 3520 refuse (§4.4/§7); R2-I4 SALT 5a honors the
  §164(b)(5) income-OR-sales either/or (§4.6). r2 Minors folded (§10 KAT-9 arithmetic; M4 G20).

---

## 1. Purpose & scope

Extend `btctax` from "bitcoin capital-gains only" to producing a **complete US federal individual income tax
return** for a **Common W-2 household** (including a crypto miner/staker operating as a business), filling the
official IRS PDFs (print-and-mail), fully offline.

**v1 target:** **TY2024 only.** Offline, vault-encrypted, permissive-licensed **and distributable** (keep the
full legal apparatus — §9). TY2025 (+ full Schedule 1-A) and CTC/Schedule 8812 are **follow-on cycles** (§11).

### 1.1 In scope (v1)

- **Income:** W-2 wages (multi-employer); 1099-INT; 1099-DIV (ordinary + qualified + cap-gain distributions +
  §199A REIT dividends); 1099-G unemployment; the existing **crypto** 8949/Schedule D pipeline (unchanged);
  **crypto ordinary income** — **hobby/other** → Schedule 1 **L8v**, and **business/self-employment** →
  **Schedule C → Sch 1 L3** (both derived from the ledger's `Income.business` flag).
- **Self-employment tax:** Schedule SE + **SE tax** on business crypto income (existing `se.rs`); ½-SE → Sch 1
  L15; the 0.9% Additional Medicare on SE income via Form 8959 Part II.
- **Deductions:** standard vs itemized (**Schedule A**: medical 7.5% floor, SALT $10k/$5k cap with the
  §164(b)(5) income-OR-sales election, home-mortgage interest [Form 1098 line 8a], charitable with §170(b)
  class ceilings + 5-yr class/vintage carryover incl. the existing crypto Form 8283).
- **Schedule 1:** a **minimal enumerated** surface (§4.4) — fail-closed on everything else.
- **QBI:** Form 8995 simplified path for §199A REIT dividends (1099-DIV box 5).
- **Credits (taxpayer's-own-money / simple):** **§904(j) foreign-tax credit** (≤$300/$600, no Form 1116 →
  Sch 3 L1, D-3); **excess-Social-Security credit** (multi-employer, §4.9).
- **Other taxes:** absolute **NIIT (Form 8960)**; **Additional Medicare (Form 8959)** Part I (wages) + Part II
  (SE) + Part V (withholding).
- **Absolute liability:** AGI → deduction → taxable income → tax → credits → other taxes → total → payments →
  refund/owed, on the real 1040 lines.
- **Forms filled:** 1040 (full), Schedule 1, 2, 3, A, B, C, D (extended — §7.2), SE, 8949, 8283 (existing SE/
  8949/8283), **8959, 8960, 8995** (new). Schedule C is new.

### 1.2 Out of scope (v1) — fail-closed, documented in LIMITATIONS (§9.2)

CTC/ODC, education, dependent-care, saver's, energy, adoption credits — **omitted conservatively** (advisory,
not refused; §3.4). AMT computation (screen-only, §4.11); Schedule E/F rental/farm; retirement/IRA/pension/SS
income (1040 4a–6b); foreign tax **> $300/$600 or non-passive** (refuse, D-3); foreign trust (Form 3520 —
refuse, R2-I3); state returns; e-file; **TY2025** and Schedule 1-A; any line needing an unmodeled worksheet.
The fail-closed rule (§3.4) turns each refusal case into `NotComputable`, never a silent wrong number.

---

## 2. Architecture

**Additive.** The crypto **delta engine stays frozen**; a new input layer and a new absolute-liability layer
wrap it.

```
NEW  ReturnInputs (per-year side-table, in vault)  ── line items + PII + payments + carryovers (§4)
       │  derive_tax_profile(&tables, year)  (pure)          FROZEN
       ▼                                                        ▼
     TaxProfile (2 scalars) ───────────────────────────► compute.rs (crypto DELTA engine, unchanged)
       │
NEW  absolute-liability assembly (btctax-core/src/tax/return_1040.rs) ── reuses:
       tax/method.rs (NEW: Tax-Table/TCW + half-up round_dollar + QDCGT)      [deep/01 + F2]
       tax/charitable.rs (NEW: §170(b) class ceilings + carryover aging)      [deep/04 + G8]
       preferential_tax / ordinary_tax_on / net_1222 / se.rs (SE)             [reused as-is]
       absolute Form 8960 + 8959 (Part I/II/V) (NEW, tax/other_taxes.rs)      [deep/02]
       │
NEW/EXTENDED btctax-forms fillers: full 1040 + Sch 1/2/3/A/B/C + Sch D(L17–22) + SE + 8959 + 8960 + 8995
                                   (per-(form,year) TOML maps; geometric read-back; DRAFT/attest gate)
```

**Frozen (unchanged):** `TaxProfile` (`types.rs`), `compute.rs`, the ~80 constructors, what-if,
pseudo-reconcile, existing crypto tests. Old hand-entered scalars remain the **raw-override escape hatch**.

**New modules:** `btctax-core/src/tax/{method,charitable,return_1040,other_taxes,return_inputs}.rs`;
`btctax-cli/src/return_inputs.rs`; `btctax-forms/src/{schedule_c,f8959,f8960,f8995}.rs` + extended
`schedule_d.rs`/full `form1040.rs` + new schedule fillers; new `conventions::round_dollar`.

---

## 3. Cross-cutting conventions

### 3.1 Rounding (resolves BLOCKER G3; I1)

- **New `round_dollar` (`MidpointAwayFromZero` / half-up)** in `conventions.rs`, distinct from `round_cents`
  (`MidpointNearestEven`). IRS "$2.50 → $3" (2024 i1040 **p. 23**; F2 §1 proven mandatory).
- **Global election = round-all-amounts, with cross-footing:** unprinted worksheets (QDCGT, IRA/student-loan,
  §170(b) ceiling, excess-SS, SE) **carry cents**, round once at the value landing on a form line; **printed
  form lines** are `round_dollar`ed at the line and **printed totals sum the already-rounded printed lines**
  (every filed form cross-foots — FFFF/commercial reading). Inputs accepted in cents, rounded at first
  form-line use. KAT-9 uses a **discriminating** fixture (two `.50` components; see §10).

### 3.2 Negative-sign formatting (resolves the other half of G3)

Per-(form,line) sign policy in the map schema: `neg: minus | parens | magnitude` (Sch 1 L8a NOL and Sch D loss
lines are pre-printed parens → magnitude; 1040 L7 leading minus). KAT a loss-year **1040 L7 = −3,000** fill +
read-back (extend the oracle — never verified a negative cell).

### 3.3 Determinism & verification layers

Geometric read-back (`verify.rs`) proves **placement only**. Numeric correctness is the layered plan (§10).
Golden-PDF SHA-256 hashing extends to every new form.

### 3.4 Fail-closed posture — with the conservative-omission carve-out (I2)

A wrong return is worse than a refusal. **Any in-scope line that can't be computed, and any captured-but-
unmodeled input that would *increase tax or change a reported figure*, produces `NotComputable(Blocker)` —
never a silent 0 or plausible wrong number.** **Carve-out:** a **purely taxpayer-favorable** benefit v1
deliberately omits (CTC/ODC, EIC) does **not** refuse — it is **omitted conservatively** (overstates tax
slightly, never understates), with a **loud advisory + LIMITATIONS entry** and a KAT pinning the line to 0 +
advisory. §1.2's out-of-scope items split into this **omission** list (favorable-only) vs the §4.10 **refuse**
list (anything that could make the return wrong).

---

## 4. Data model — `ReturnInputs`

New per-year `return_inputs` side-table (JSON, `year PRIMARY KEY`), mirroring `tax_profile.rs`, in the
encrypted vault. `Usd` = `rust_decimal::Decimal` (cents). `#[serde(default)]` on every optional field.

```rust
pub struct ReturnInputs {
    pub filing_status: FilingStatus,
    pub header: HouseholdHeader,             // §4.2
    pub w2s: Vec<W2>,                         // §4.1
    pub int_1099: Vec<Form1099Int>,
    pub div_1099: Vec<Form1099Div>,
    pub g_1099: Vec<Form1099G>,               // §4.3 unemployment
    pub schedule_c: Option<ScheduleCInputs>,  // §4.4a business crypto (D-6)
    pub schedule_a: Option<ScheduleAInputs>,  // None ⇒ standard deduction
    pub itemize_election: ItemizeElection,
    pub mfs_spouse_itemizes: Option<bool>,    // REQUIRED iff MFS
    pub sch1: Schedule1Inputs,                // §4.4
    pub payments: Payments,                   // §4.8
    pub capital_loss_carryforward_in: Carryforward,
    pub charitable_carryover_in: Vec<CharitableCarryItem>,  // §4.6 class + vintage
    pub qbi: QbiInputs,                        // §4.5
    pub foreign_accounts: Option<bool>,        // Sch B Part III — required if Sch B files (I7)
    pub foreign_trust: Option<bool>,           // Some(true) ⇒ refuse (Form 3520, R2-I3)
    pub foreign_country_names: String,
}
```
**Carryover persistence (R3-M6):** at report time the computed carryover-*out* (charitable, per class/vintage;
and QBI REIT/PTP) is written back as year **(Y+1)'s `*_carryover_in`** on that row — a single mechanism (no
separate staging field). **Precedence:** a computed carryover-in overwrites a prior *computed* value but
**refuses to silently overwrite a user-entered** one (warn + `--force`, mirroring §4.12); every carryover-in
carries **provenance** (computed vs user).

### 4.1 W-2 (G5)

`W2 { owner: Owner (Taxpayer|Spouse; per-earner box3/box4 §4.9 + SE cap §4.4a), employer, box1_wages (→1a),
box2_fed_withheld (→25a), box3_ss_wages, box4_ss_withheld (→excess-SS), box5_medicare_wages (→8959 Part I),
box6_medicare_withheld (→8959 Part V→25c), box7_ss_tips, box17_state_tax_withheld (→Sch A 5a), box19_local_tax
(→5a), box12: Vec<Box12Entry> (refuse-guard §4.10), box13_retirement_plan (gates Sch1 L20 §4.4),
box8_allocated_tips (refuse if >0), box10_dependent_care (refuse if >0) }`.

### 4.2 Header / PII (vault-only; I5)

`HouseholdHeader { taxpayer: Person, spouse: Option<Person>, address…, dependents: Vec<Dependent>,
can_be_claimed_as_dependent_{taxpayer,spouse}: bool, presidential_fund_{taxpayer,spouse}: bool,
ip_pin: Option<String> }`.
`Person { first, last, ssn, ssn_valid_for_employment: bool, date_of_birth: Date, blind: bool, occupation }` —
DOB for age-65 (F3), **`blind` explicit** (I5); both drive §63(f) + the 2024 header checkboxes.
`Dependent { name, ssn, ssn_valid_for_employment, relationship, date_of_birth }` — captured (CTC = conservative
omission). SSNs normalized/masked; `--stdin` entry (security-review item).

### 4.3 1099-INT / 1099-DIV / 1099-G

`Form1099Int { payer, box1_interest, box2_early_withdrawal_penalty (→Sch1 L18), box3_treasury_interest,
box4_fed_withheld (→25b), box8_tax_exempt_interest (→2a; NOT §1411 add-back), box6_foreign_tax (→FTC §4.7a),
box9_private_activity_bond_amt (refuse-guard) }`.
`Form1099Div { payer, box1a_ordinary, box1b_qualified, box2a_capgain_distr, box2b/2c/2d (refuse-guard),
box4_fed_withheld (→25b), box5_section_199a (→QBI §4.5), box7_foreign_tax (→FTC §4.7a),
box12_exempt_interest_dividends (→2a), box13_private_activity_amt (refuse-guard) }`.
`Form1099G { payer, box1_unemployment (→Sch1 L7), box4_fed_withheld (→25b) }`.

### 4.4 Schedule 1 — enumerated minimal surface (BLOCKER G1; C1/I3)

| Sch 1 line | v1 policy | Detail |
|---|---|---|
| L1 state/local refund | user enters taxable portion + advisory | §111 worksheet not modeled |
| **L3 business income (Schedule C)** | **derived** — §4.4a | business crypto net profit (mining/staking as a trade/business) |
| L7 unemployment | derived (Σ G box1) | |
| **L8v digital-asset ordinary income** | derived (Σ non-business `crypto_ord`) | hobby/other-income rewards + non-business lending interest |
| L15 ½-SE-tax | derived (`se.rs`, from Schedule C net) | |
| L18 early-withdrawal penalty | derived (Σ INT box2) | |
| L21 student-loan interest | full worksheet | $2,500 cap; MAGI phase-out $80–95k S / $165–195k MFJ; **MFS ⇒ $0** (§221(e)(2)) |
| L20 IRA deduction | **refuse iff a deduction is claimed** (`ira_contribution > 0`) | box 13 alone does NOT refuse (I3); phase-out worksheet is the follow-on |
| L13 HSA | refuse if present | Form 8889 (also box-12 W guard) |
| L2/L5/L8x-other/L9… | no input; refuse if needed; LIMITATIONS | out of scope |

#### 4.4a Schedule C inputs (D-6)

`ScheduleCInputs { owner: Owner, business_description: String, naics_code: String (default "999999", line B),
accounting_method: Method (default Cash, line F), expenses: Usd }`. **Gross business income = Σ ledger
`crypto_ord` where `Income.business == true` AND kind ∈ {Mining, Staking, Airdrop, Reward}** (the SE-eligible
kinds; derived, `event.rs:61`). **Business-flagged `Interest` income ⇒ refuse (§4.10, R3-I3):** it is excluded
from SE by §1402(a)(2) yet not sheltered from NIIT, so it has no clean v1 home; refusing keeps the standalone
SE delta report untouched. `expenses` (single scalar) → Schedule C **line 27a (other expenses) → line 28
total** (the Part V itemization is left blank). Net profit = gross − expenses → **Sch 1 L3**; and Schedule SE
Part I (SS cap keyed to `owner`'s own box3 + box7 tips, deep/02 C4 / R3-M2). v1 supports **one** Schedule C
(one SE earner); ≥2 SE earners → refuse (F4 hardening (b)). **If the ledger has business income but
`schedule_c == None` ⇒ fail loud** (owner + description unknowable; G15 pattern, R3-M10). Non-business crypto
ordinary income → L8v.

### 4.5 QBI inputs

`QbiInputs { reit_ptp_carryforward_in: Usd, qbi_deduction_override: Option<Usd> }`. §199A REIT dividends =
Σ `div.box5`. Compute Form 8995 (F3 §2) when TI-before-QBI ≤ $191,950/$383,900 (TY2024); above, or if actual
non-REIT QBI asserted, **refuse** (8995-A unbuilt). box5 ⊂ ordinary dividends → stays in ordinary stack.
(Note: crypto Schedule C is *not* §199A QBI in v1 — refuse if the user asserts a QBI deduction on it; a
follow-on can add the QBI-on-Sch-C path.) Carryforward-out persists per §4.

### 4.6 Schedule A inputs — classified charitable + SALT either/or (G12; I4; R2-I1/R2-I4)

```rust
pub struct ScheduleAInputs {
    pub medical: Usd,
    pub salt_use_sales_tax: bool,                 // §164(b)(5) election (R2-I4)
    pub salt_sales_tax_amount: Usd,               // used IFF salt_use_sales_tax
    pub salt_state_estimated_payments: Usd,       // income-tax path only
    pub salt_prior_year_balance_paid: Usd,        // income-tax path only
    pub salt_real_estate: Usd,                    // 5b (always)
    pub salt_personal_property: Usd,              // 5c (always)
    pub mortgage_interest_1098: Usd,              // 8a only; 8b/points refuse-or-advise (M8); $750k/$1M advisory
    pub charitable: Vec<CharitableGift>,          // classified
}
```
**SALT 5a (R2-I4 — §164(b)(5) EITHER/OR):**
- `salt_use_sales_tax == true`  → `5a = salt_sales_tax_amount` **only** (income-tax withholding is excluded).
- `salt_use_sales_tax == false` → `5a = Σ w2.box17 + Σ w2.box19 + salt_state_estimated_payments +
  salt_prior_year_balance_paid`. Never both.
The filler **checks the 5a sales-tax election box** (deep/03 `c1_1`) iff `salt_use_sales_tax` (R3-M9). A
nonzero `salt_sales_tax_amount` with the election **off** ⇒ **fail-loud** (refuse — a silent drop would hide an
input error).

**Charitable (classified — deep/04 6-class; ST-crypto = 50%, not 60%):**
```rust
pub enum CharitableClass { Cash60, Cash30, CapGainProp30, CapGainProp20, OrdinaryProp50, OrdinaryProp30 }
pub struct CharitableGift { class: CharitableClass, amount: Usd }
pub struct CharitableCarryItem { class: CharitableClass, amount: Usd, origin_year: i32 }
```
Crypto donations flow from the **ledger's computed §170(e) deduction** (LT → `CapGainProp30`; ST →
`OrdinaryProp50`) — never re-typed. `tax/charitable.rs` applies §170(b) ceilings in statutory order and
**(R2-I1)** limits the **30%-cap-gain class** to `min(30%·AGI, 50%·AGI − (allowed 60%/50%-tier contributions
this year, INCLUDING allowed ordinary-income-property amounts))` — the full deep/04 §5 / :190 formula, not
`50%·AGI − cash`. Carryover consumed **oldest-vintage-first**, expires at 5 years (§170(d)(1)), and **ages even
in standard-deduction years** (Reg. §1.170A-10(a)(2); G8).

### 4.7 Std-vs-itemized (deep/04 §3; M3)

`deduction = max(std_total, scheduleA_L17)`. `ForceItemize` honors §63(e). **MFS coupling (§63(c)(6)):**
`mfs_spouse_itemizes` required iff MFS (`Option<bool>`; `None` ⇒ fail-loud, G15); `Some(true)` ⇒ other spouse
std = $0 **and** the filler checks the **2024 combined header checkbox** (`c1_8`, deep/03; the 12b/12c split is
TY2025, M3). Std deduction = basic + §63(f) aged (DOB) + blind (§4.2) + dependent floor (dependent's earned
income = Σ box1 + Schedule C net − ½SE, G21).

### 4.7a §904(j) foreign-tax credit (D-3)

`ftc_raw = Σ(int.box6 + div.box7)`. Claim directly on **Sch 3 L1 (no Form 1116)** iff all foreign tax is
**passive-category income, 1099-reported, and `ftc_raw ≤ $300 ($600 MFJ)`**; else **refuse** (Form 1116 out of
scope). Nonrefundable, capped by tax liability via Sch 3 L8 → 1040 L20. v1 assumes 1099-sourced foreign tax is
passive (advisory: "if any is non-passive, do not use this path").

### 4.8 Payments (M4)

`Payments { estimated_tax_payments (→L26), extension_payment (→Sch 3 L10, G18), other_withholding (→25c; docs
warn it needs an in-scope income line, G19) }`. **25a**=Σ box2; **25b**=Σ(INT+DIV+G box4); **25c**=8959 L24 +
other_withholding. **L31 = Sch 3 L15** (sums L10 extension + L11 excess-SS), not L11 alone.

### 4.9 Excess-Social-Security credit (G6; per-employer clamp M1)

Per person: `credit_p = max(0, Σ_employers min(box4ᵢ, MAX) − MAX)`, **MAX = 6.2%×$168,600 = $10,453.20**
(TY2024). **Refuse** if any single employer's box4 > MAX (not creditable). Requires **≥2 employers** for that
person; **per person** (never pooled); → Sch 3 L11 → 1040 L31. RRTA out of scope.

### 4.10 Refuse-guard table (G9) — normative, one KAT per row

| Input present | v1 action | Why |
|---|---|---|
| ≥2 self-employment/business crypto earners | refuse | v1 supports one Schedule C (§4.4a) |
| **business-flagged crypto `Interest` income** | refuse | R3-I3: excluded from SE (§1402(a)(2)) but not NIIT-sheltered — no clean v1 home |
| QBI deduction asserted on Schedule C income | refuse | §199A-on-Sch-C is a follow-on (§4.5) |
| W-2 box 12 **W** | refuse | Form 8889 mandatory |
| W-2 box 12 **A/B/M/N** | refuse | Sch 2 L13 |
| W-2 box 12 **Z** | refuse | Sch 2 L17h |
| W-2 **box 8** allocated tips | refuse | Form 4137 |
| W-2 **box 10** dependent-care | refuse | Form 2441 |
| INT **box 9** / DIV **box 13** (PAB AMT pref) | refuse | AMT preference (§4.11) |
| DIV **box 2b/2c/2d** (§1250/§1202/28%) | refuse | Schedule D Tax Worksheet |
| foreign tax > $300/$600 or non-passive | refuse | Form 1116 (D-3) |
| `foreign_trust == Some(true)` | refuse | Form 3520 (R2-I3) |
| single-employer excess SS withholding | refuse | not creditable (§4.9) |
| Sch 1 L13 / L20-with-deduction | refuse | unmodeled worksheet |
| taxable income ≤ 0 | refuse | carryover-worksheet edge (G22) |

### 4.11 AMT screen (G13; M7)

Sch 2 L1 = 0, **not silently**: implement the 2024 "Worksheet To See if You Should Fill in Form 6251" as a
**refuse-trigger**; plus refuse on visible AMT-preference inputs (INT box 9 / DIV box 13). KAT the screen.
LIMITATIONS: "AMT computation out of scope; screened."

### 4.12 Precedence (G4; D-4)

**One resolver** `resolve_profile(year) -> (TaxProfile, Provenance)` for every consumer (report/TUI/optimize/
what-if/export). Order: `ReturnInputs` → stored `TaxProfile` → pseudo-placeholder → `TaxProfileMissing`.
**Provenance printed on every output.** `tax-profile set` **warns + requires `--force`** when `ReturnInputs`
exists (D-4).

---

## 5. Computation pipeline (topological)

```
0  Header: filing status, DOB→age, blind, can-be-claimed, dependents, digital-asset
1  Income → L9:
     1a=Σ box1; 2a=Σ(int.box8+div.box12); 2b=Σ(int.box1+box3) [Sch B per §7.1 trigger]
     3a=Σ box1b; 3b=Σ box1a; 7=crypto Sch D (+Σ box2a via Sch D L13)
     8 = Sch 1 L10: L1(attest) + L3(Schedule C net) + L7(Σ G box1) + L8v(Σ non-business crypto_ord) + …
     L9 = 1a+2b+3b+7+8   (all crypto ordinary income has a printed home → cross-foots)
2  Adjustments → L10, AGI L11: ½-SE(L15, from Sch C) + early-withdrawal(L18) + student-loan(L21 WS) → Sch1 L26
     AGI (L11) = with-crypto AGI  ★ pivot (Sch A, 8960 MAGI, phase-outs read THIS; G7)
3  Deduction: 3a Sch A on WITH-CRYPTO AGI (G7): medical max(0,med−7.5%·AGI); SALT (§4.6 either/or, cap
     $10k/$5k); mortgage(8a); charitable (tax/charitable.rs class ceilings incl. R2-I1 30%-cap; vintage
     carryover; age even if std wins, G8).  3b deduction=max(std,SchA L17) [MFS §4.7] → L12.
     3c QBI(8995) → L13; L14=L12+L13; L15=AGI−L14 (refuse if ≤0)
4  L16 = method.rs::qdcgt_line16(TI=L15, qd=3a, ltcg=net_1222 preferential_gain):
     pref_ws = min(TI, qd+ltcg) ★F2 F-A; L22/L24 Table(<100k)/TCW(≥100k) each on its own amount;
     line16 = round_dollar(min(L23,L24)) ★min load-bearing (F2 F-B)
5  Sch 2 Part I (L2 AMT screen=0) → Sch 2 L3 → 1040 L17; L18=L16+L17
6  Nonrefundable credits: FTC(§4.7a)→Sch 3 L1; Sch 3 L8→1040 L20; L19=0 CTC/ODC omitted-conservatively
     (+advisory, §3.4); L21=L19+L20; L22=max(0,L18−L21)
7  Sch 2 Part II (other taxes) → L23:
     Schedule SE (SE tax): SE base = 0.9235 × Sch C net. **§6017 $400 floor (R3-M3): base < $400 ⇒ SE tax=0,
        NO Schedule SE filed, NO ½-SE, 8959 L8=0.** Else Sch SE L10 = 12.4%×min(base, SS_base − owner's own
        (box3+box7 tips)) [R3-M2]; L11 = 2.9%×base; L12 = L10+L11 → Sch 2 L4  ★ 0.9% NOT here (deep/02 C5) —
        routes to 8959 Part II
     Form 8959: Part I = 0.9%·max(0, Σbox5 − thr);  Part II = 0.9%·max(0, SE − **max(0, thr − Σbox5)**)
        [R3-I1 inner clamp; ≡ se.rs.addl ≡ 8959 L11–L13];  L18 = Part I+II → Sch 2 L11;
        Part V: L22 = max(0, Σbox6 − 1.45%·Σbox5); L24 = L22 (RRTA L23=0) → 1040 L25c [R3-M2 cite L24]
     Form 8960 (absolute): NII = L1(=1040 2b interest) + L2(=1040 3b dividends) + L5a(=§1211-limited figure
        reaching 1040 L7) + **L7 (crypto lending interest, carried on Sch 1 L8v — a line-7 modification, since
        it is NOT on 1040 2b; R3-M5)**;  ALL lending interest is in NII (business-flagged Interest is refused,
        §4.10/R3-I3);  Schedule C business income (mining/staking as a trade/business) is EXCLUDED (§1411
        active-business);  MAGI=AGI (fail-closed §911/CFC/PFIC);
        **L17 = 3.8%·max(0, min(max(0, NII), max(0, MAGI − thr)))** [R3-I1 floors; ≡ compute.rs:369; deep/02 §2.4] → Sch 2 L12
     Sch 2 L21 → 1040 L23; L24 = L22+L23 (TOTAL TAX)
8  Payments → L33: 25a Σbox2; 25b Σ(INT/DIV/G box4); 25c 8959 L24 + other_wh; 26 estimated;
     Sch 3 L10 extension + L11 excess-SS → Sch 3 L15 → 1040 L31; L33 = total payments
9  Settle: L34=max(0,L33−L24) → L35a refund (−L36 applied-to-2025, G16) / else L37=L24−L33 owed;
     L38 blank (IRS figures penalty); direct-deposit 35b–d omitted (paper check, LIMITATIONS); "Sch D not
     required" box (1040 L7) always unchecked (btctax always files Sch D, M4/G20)
```

DAG, no cycle. **Reduce-to-delta invariant (scoped, R3-I4):** with all non-crypto inputs 0, **8959 collapses
to the engine's crypto-delta exactly** (Part II reads the same `se.rs` base both sides; Part I=0 without wages).
**8960 collapses exactly for regimes without SE income, and for SE regimes only when the NII arm binds** — with
SE income the *absolute* MAGI additionally reflects the ½-SE deduction and Schedule C expenses (net, not the
engine's gross `crypto_ord`, `compute.rs:364`), which the frozen delta cannot see, so in a **MAGI-binding SE
regime absolute NIIT < delta** (a documented divergence, cf. §6 — the absolute side is the correct Form 8960;
the delta is the approximate crypto-attribution). KAT-5's SE fixture is NII-binding (deep/02 Ex.2 qualifies);
KAT-5b pins the documented inequality for a MAGI-binding SE fixture. Neither mis-fix is allowed at plan time
(stripping ½-SE from absolute MAGI is tax-wrong; teaching the frozen engine about expenses breaks the freeze).

---

## 6. Delta vs absolute (G7)

Both numbers reported, **labeled as different questions** (deep/02's $2,242 absolute-NIIT vs $1,596 crypto-
delta). The delta path's deduction is fixed at derivation time (frozen engine can't re-branch std-vs-itemized),
so `absolute_with − absolute_without ≠ delta` when a deduction is AGI-sensitive; the report documents the delta
deduction as **approximate** and never reconciles the two to the dollar.

---

## 7. Form-set closure & PDF fillers (BLOCKER G2; I8/R2-I2)

### 7.1 Fill set (v1)

Full **1040**, **Sch 1/2/3/A/B/C**, **Sch D** (extended §7.2), **SE, 8949, 8283** (existing), **8959, 8960,
8995** (new); **Schedule C** (new). Every mandatory "Attach Form X" for an in-scope figure has its form; a
non-DRAFT return never shows a line with no backing form. QBI: box5>0/override forces the 8995 map.

**Schedule B filing trigger (R3-I2 — normative, single site):** Schedule B files when **taxable interest >
$1,500** *or* **ordinary dividends > $1,500** *or* `foreign_accounts == Some(true)` (Part III trigger (b))
*or* user-forced; `foreign_trust == Some(true)` **refuses** before filing (trigger (c) → Form 3520, §4.10).
When Schedule B files, **Part III 7a/8 must be answered** (`foreign_accounts`/`foreign_trust` tri-state —
fail-loud if `None`), with the FinCEN Notice 2020-2 crypto advisory (§9). Below-threshold interest/dividends
still land on 1040 2b/3b when Schedule B is not required.

### 7.2 Schedule D routing — EXHAUSTIVE (mandatory; R2-I2)

Extend `schedule_d.rs` to L17–22 (`schedule_d.rs:5-6` scope-out removed). L16 = L7 + L15:
- **L16 > 0 and L15 > 0 (both gains):** L17 = **Yes**; L18 = L19 = 0 (28%/§1250 refuse-guard); **L20 = Yes** →
  QDCGT. L21/L22 not completed.
- **L16 > 0 and L15 ≤ 0 (ST-gain / LT-loss — common crypto year):** L17 = **No** → skip 18–21 → **L22**:
  Yes iff 3a > 0 → QDCGT; else Tax Table/TCW.
- **L16 < 0 (net loss):** skip 17–20; **L21 = −min(|L16|, $3,000 / $1,500 MFS)**; **L22**: Yes iff 3a > 0.
- **L16 = 0:** 1040 L7 = 0; skip 17–21; **L22**: Yes iff 3a > 0.
All QDCGT-bound cases feed `method.rs`. KAT all four paths (gain-both, ST-gain/LT-loss, loss, zero).

### 7.3 New fillers (Schedule C, 8959, 8960, 8995)

Schedule C (2 pages, but v1 uses Part I income + Part II expenses only — mostly blank), 8959/8960/8995
(1-page). All XFA-hybrid `f1_N[0]`/`c1_N[0]` — a scheduled `deep/03`-style extraction (root FQN + leaf map +
geometric read-back). Bundle PDFs public-domain.

### 7.4 Map hazards (F5)

Per-(form,year) maps mandatory: `f1_57` = L12(2024)/L1z(2025) collides; filing-status on-states re-assigned →
maps carry `(FQN, on_state)` per status per year; roots flip in 2025. Extend the read-back oracle to the
**5-way filing-status checkbox group** (`verify.rs` today only Yes/No pairs) and **negative cells** (§3.2).
Sch B >14 interest / >15 dividend overflow reuses the 8949 continuation pattern.

---

## 8. Tax-year data (btctax-adapters `TaxTable`)

Add **standard-deduction** basic + §63(f) aged/blind + dependent floor (TY2024 $14,600/$29,200/$21,900;
$1,550/$1,950 per box; floor $1,300/+$450). SALT cap statutory-constant TY2024 ($10k/$5k). Excess-SS MAX =
6.2%×`ss_wage_base`. FTC ceiling $300/$600. **No per-year Tax-Table data** (F2). Add a **per-year assertion**
in `method.rs`: no bracket edge < $100k inside a $50 bin (< $3,000 inside a $25 bin).

---

## 9. Legal / positioning (retained; we distribute)

DRAFT watermark + attestation forced on every full return; mechanical (UPL); Paid Preparer/PTIN blank. **§9.2
LIMITATIONS doc** (versioned; man page + `--help` + shipped file): supported forms/lines/years, plus three
lists aligned to §3.4's omission-vs-refuse split (R3-M8): **(i) favorable-only OMISSIONS** (advisory, overstate
tax at worst) — CTC/ODC, EIC, education/dependent-care/saver's/energy/adoption credits, direct-deposit;
**(ii) REFUSALS** (§4.10) — incl. AMT-screen trigger, excess-APTC/Form 8962 (Sch 2 L2), foreign trust (3520),
foreign tax > cap, the box-12/8/10 guards, business-flagged Interest, Sch 1 L13/L20-with-deduction;
**(iii) unrepresentable / documented-out** (no input; would refuse if captured) — 1099-R/SSA/pension income,
state returns, e-file, non-crypto Schedule C/E/F. Plus the **conservative simplification** (Form 8960 Part II
state-tax reduction omitted — only overstates NIIT, M11) and two **advisories**: the **FBAR/FinCEN** advisory
(Notice 2020-2 — crypto-only foreign exchange accounts currently outside FBAR; never auto-answer Sch B Part
III) and the **charitable-donee** advisory (the ledger auto-classes crypto gifts assuming a **public-charity
(50%-org)** donee → 30%/50% ceilings; a private-foundation donee is the 20%/basis class — deep/04, R3-M4).
Fail closed (§3.4). "Not tax advice." Permissive + clean-room; CC0 PSL Tax-Calculator params as CI cross-check;
tenforty/PolicyEngine observe-only oracles.

---

## 10. Test plan

**Layers:** L0 method → L1 per-worksheet KATs → L2 synthetic end-to-end vs an independent oracle → L3
IRS-authored fixtures → L4 golden-PDF SHA-256. Placement read-back separate.

**KATs:** 1) QDCGT pref cap (TI 35,400/QD 50,000 ⇒ $0); 2) binding-min same-bin (L5 58,000/QD 10 ⇒ 7,819);
3) per-year bin/edge assertion; 4) SALT-MFS-halve-last (TY2025 follow-on, labeled); 5) reduce-to-delta (4
regimes, SE fixture **NII-binding**); **5b) documented `absolute NIIT < delta` inequality for a MAGI-binding
SE fixture (R3-I4);** 6) SE Sch 2 L4 unbundle + 8959 Part I+II + Part V floor; 7) cross-year
map-collision negative test; 8) filing-status on-state per year + 5-way checkbox oracle; 9) **rounding
cross-foot** — discriminating fixture: 8959 Part I L7 = 271.50 + Part II L13 = 499.50 → **printed 272 + 500 =
772** (not `round(771.00)`), proving printed-line rounding + cross-foot; 10) loss-year 1040 L7 = −3,000 + Sch D
L17–22 **all four paths** (I8/R2-I2); 11) excess-SS two-employer + per-employer clamp + MFS-both-itemize
golden; 12) L25c composition; 13) G8 std-year-between-two-itemized-years carryover; 14) AMT-screen refuse-
trigger; 15) L8v hobby vs L3 Schedule-C business split + cross-foot; 16) **§904(j) FTC** ≤cap credit + >cap
refuse; 17) **charitable 30%-cap-gain ceiling with same-year ST+LT crypto donations** (R2-I1: exercises the
`50%·AGI − (cash + ordinary allowed)` term); **18) Schedule B filing trigger (R3-I2)** — a $2,000-dividends /
$100-interest household files Schedule B Part II+III, and a ≤$1,500 household with `foreign_accounts=Some(true)`
files Part III; one KAT per refuse-guard row (§4.10). Golden end-to-end: ≥1
synthetic household per branch (single/MFJ; std/itemized; ±QD+LTCG; under/over $100k; multi-W-2; REIT box5;
crypto hobby income; **crypto Schedule-C business income + SE** [= deep/02 Ex.2, $60k mining]). **Fixture
caveat (M6):** IRS ATS Scenario 2 pulls in out-of-scope forms → **partial-line diff** or a v1-envelope
synthetic golden.

*(Erratum, no spec change: recon-01 §2 shows Sch 2 L1/L2 swapped vs the 2024 form; this spec's §5 stage 5
uses the correct "L2 AMT → Sch 2 L3.")*

---

## 11. Build phases (one plan; D-5)

Refuse-guard table (§4.10) lands in phase 1. Each phase = TDD + review-to-green.

0. **Conventions & method** — `round_dollar`; `tax/method.rs` (Table/TCW + QDCGT incl. pref-cap + binding-min +
   §3.1 cross-foot); per-year bin assertion. (KATs 1–3, 9.)
1. **`ReturnInputs` + side-table + CLI/TOML** (§4) — additive; `resolve_profile` + provenance; refuse-guards.
2. **`derive_tax_profile` + `return_1040` income→AGI** (§5 stages 1–2 incl. Schedule C net + SE ½-adjustment;
   std-basic only here) → L11.
3. **Std deduction (full) + `tax/charitable.rs`** (class ceilings incl. R2-I1 + vintage carryover + G8 aging +
   write-back) + Schedule A (§4.6 SALT either/or, with-crypto AGI) + std-vs-itemized → L12–L16.
4. **Credits + other taxes** — FTC §904(j) (§4.7a); absolute NIIT (8960); Additional Medicare 8959 Part I+II+V;
   SE tax (Sch SE, unbundled) + excess-SS; AMT screen; QBI/8995. (R3-M7: the Schedule 1 income lines incl. L8v
   and Schedule C net are computed in **phase 2**; phase 3 carries QBI as a **0-stub** completed here.)
5. **LIMITATIONS doc** (§9.2) + conservative-omission advisories.
6. **PDF fillers** — Sch D L17–22 (§7.2); Schedule C, 8959/8960/8995 maps (§7.3); full 1040 + Sch 1/2/3/A/B +
   SE; per-year maps + on-state/negative read-back extensions (§7.4); DRAFT/attest gate; golden hashes.
7. **End-to-end golden returns + ATS fixture** (§10 L2/L3).

## 12. Decisions (RESOLVED 2026-07-12)

- **D-1 QBI:** include Form 8995 for REIT dividends (index funds). *Resolved: include.*
- **D-2 Schedule 1 minimum:** §4.4 set (L1 attest; L3/L7/L8v/L15/L18 derived; L21 worksheet; refuse L13,
  L20-with-deduction). *Resolved as specced.*
- **D-3 Foreign-tax credit:** **implement §904(j)** ≤$300/$600 passive+1099-reported → Sch 3 L1; refuse above
  (§4.7a). *Resolved (user).*
- **D-4 Precedence:** `tax-profile set` warn + `--force`. *Resolved: warn+force.*
- **D-5 Scope structure:** **one spec, phased plan** (§11). *Resolved (user).*
- **D-6 Crypto ordinary income:** **include Schedule C + SE** — business/SE crypto files (Schedule C → Sch 1
  L3 + Schedule SE + full 8959); hobby crypto → L8v. *Resolved (user).*
