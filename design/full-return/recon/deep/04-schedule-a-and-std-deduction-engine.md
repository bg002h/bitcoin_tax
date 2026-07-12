# Deep Recon 04 — Standard-Deduction + Schedule A Engine (TY2024)

**Agent:** OPUS deep-dive recon round 2 (spec-grade). **Scope:** the standard-deduction and
Schedule A itemized-deduction engine for **v1 = TY2024, common W-2 household, absolute
liability**, integrating the already-shipped crypto §170(e) / Form 8283 deduction. **Builds
on:** first-pass `recon/02-computation-worksheets.md` §1–§3 and `recon/04-input-data-model.md`
§5.1. **Primary sources read directly this pass** (via `pdftotext` on the IRS PDFs): Rev. Proc.
2023-34 §3.15 (verbatim), 2024 Instructions for Schedule A (verbatim), Pub. 526 "Limits on
Deductions" + Worksheet 2 ordering, §63/§170(b) of the Code.

---

## ⚠️ CORRECTIONS to the first pass (flagged loudly)

1. **[CORRECTION — ceiling class] Short-term / ordinary-income property to a public charity is
   the 50% limit, NOT 60%.** First-pass `02` §2e wrote "BTC ≤ 1 yr … **60%/50%** ceiling" and
   §2d's table has **no 50%-limit row at all**. **The 60% limit (§170(b)(1)(G)) is CASH-ONLY.**
   Pub. 526 (read verbatim): *"The 60% limit … applies to cash contributions … to 50% limit
   organizations."* Non-cash **ordinary-income property** (which is what short-term crypto
   becomes after §170(e)(1)(A) reduces it to basis) given to a public charity is subject to the
   **50% limit** (§170(b)(1)(A)/(B)), which the first-pass table omitted. **Use the corrected
   6-class table in §2d below.** This is a real dollar-affecting error, not cosmetic: at high
   donation levels 50% vs 60% changes the allowed amount and the carryover.

2. **[CORRECTION — citation precision] The standard-deduction cite is Rev. Proc. 2023-34
   §3.15, with three numbered subsections**, not the bare "§.15" the first pass used. Verified
   verbatim: **§3.15(1)** basic amounts, **§3.15(2)** dependent limitation, **§3.15(3)**
   aged/blind. (Matches the adapters' existing `§3.01/§3.03/§3.43/§3.41` numbering scheme.)

3. **[REFINEMENT — struct] recon-04 §5.1's flat `charitable_cash` / `charitable_noncash`
   `ScheduleA` fields are INSUFFICIENT for the §170(b) engine.** "Noncash" spans two different
   ceilings (LT capital-gain property = 30%; ordinary-income/basis property = 50%), and the
   5-year carryover must retain its ceiling **class**. The input model must carry ceiling-class
   buckets, and carryover-in must be per-class. See §5.

Everything else in first-pass `02` §1–§3 verified correct (amounts, 7.5% medical floor, SALT
$10k/$5k, mortgage $750k/$1M tiers, the max(std, itemized) rule, MFS coupling, §63(e) election).

---

## 1. Standard Deduction — full computation (§63)

### 1.1 Verified TY2024 amounts (Rev. Proc. 2023-34 §3.15, read verbatim)

**§3.15(1) — basic, §63(c)(2):**

| Filing status | TY2024 basic |
|---|---|
| Married filing jointly / Surviving spouse (§1(j)(2)(A)) | **$29,200** |
| Head of household (§1(j)(2)(B)) | **$21,900** |
| Single / unmarried (§1(j)(2)(C)) | **$14,600** |
| Married filing separately (§1(j)(2)(D)) | **$14,600** |

**§3.15(3) — additional, aged and/or blind, §63(f)** (verbatim: *"the additional standard
deduction amount under §63(f) for the aged or the blind is **$1,550**. The additional standard
deduction amount is increased to **$1,950** if the individual is also unmarried and not a
surviving spouse."*):

| Taxpayer | TY2024 per box |
|---|---|
| Married (MFJ/MFS/QSS) | **$1,550** |
| Unmarried & not a surviving spouse (Single/HoH) | **$1,950** |

**§3.15(2) — dependent limitation, §63(c)(5)** (verbatim: *"the standard deduction … for an
individual who may be claimed as a dependent … cannot exceed the greater of (1) **$1,300**, or
(2) the sum of **$450** and the individual's earned income."*).

> **Home for the amounts:** add to the per-year `TaxTable` (`btctax-adapters/src/tax_tables.rs`)
> beside `gift_annual_exclusion`/`ss_wage_base`. The basic amounts, the §63(f) $1,550/$1,950, and
> the $1,300 dependent floor are **all inflation-indexed** (they moved 2024→2025: $1,550→$1,600,
> $1,950→$2,000, $1,300→$1,350) so they belong in the indexed table, **not** in `core/tax/tables.rs`
> (which holds only statutory-fixed values). The `+$450` add-on has been constant but keep it in the
> table for uniformity. Recommended new `TaxTable` fields (all `BTreeMap<FilingStatus,Usd>` or scalars):
> `std_basic: BTreeMap<FilingStatus, Usd>`, `std_addl_aged_blind_married: Usd`,
> `std_addl_aged_blind_unmarried: Usd`, `dependent_std_floor: Usd`, `dependent_std_earned_addon: Usd`.

### 1.2 Checkbox inputs the algorithm consumes (Form 1040 page 1)

- **Age/Blindness block:** "You: Were **born before January 2, 1960** [ ]  Are **blind** [ ]" and
  the same two boxes for "Spouse". Born-before-Jan-2-1960 = age 65 by TY2024 year-end (someone
  born 1/1/1960 is treated as 65 on 12/31/2024). → up to **2 boxes/person**, up to **4 on a joint
  return**. Each checked box adds one §63(f) amount.
- **Dependency trigger:** "Someone can claim: **You as a dependent** [ ]  **Your spouse as a
  dependent** [ ]" → activates the §63(c)(5) limited basic amount.
- **MFS coupling box:** "Spouse itemizes on a separate return or you were a dual-status alien" [ ]
  → §63(c)(6), see §4.

Model these as booleans on `Person`/`HouseholdHeader`: `born_before_jan2_1960: bool`,
`blind: bool` (taxpayer + spouse), `can_be_claimed_as_dependent: bool`,
`spouse_itemizes_separate: bool`.

### 1.3 The exact algorithm

```
fn standard_deduction(status, table, taxpayer, spouse_opt) -> Usd:
    # 1. BASIC (§63(c)(2)), with the §63(c)(5) dependent cap applied to the basic portion ONLY
    let regular = table.std_basic[status]
    let basic =
        if taxpayer.can_be_claimed_as_dependent:
            min( regular,
                 max( table.dependent_std_floor,               # $1,300
                      taxpayer.earned_income + table.dependent_std_earned_addon ) )  # +$450
        else:
            regular

    # 2. ADDITIONAL aged/blind (§63(f)) — added ON TOP, and NOT limited by §63(c)(5)
    let per_box =
        if status in {Mfj, Mfs, Qss}: table.std_addl_aged_blind_married      # $1,550
        else:                          table.std_addl_aged_blind_unmarried    # $1,950
    let boxes = count( taxpayer.born_before_jan2_1960, taxpayer.blind )
              + if status == Mfj: count( spouse.born_before_jan2_1960, spouse.blind ) else 0
    # (MFS: a taxpayer may claim the spouse's boxes only if the spouse has no gross income and
    #  isn't a dependent — rare; model as taxpayer-only for MFS unless that flag is set.)

    return basic + per_box * boxes
```

**Two correctness invariants to KAT-lock:**
- **A dependent who is also aged/blind STILL adds the §63(f) amount.** §63(c)(5) caps only the
  *basic* deduction; §63(c)(1) = basic + additional, and the additional is untouched. This matches
  the IRS "Standard Deduction Worksheet for Dependents" (1040 instructions), which computes the
  limited basic, then adds `boxes × $1,550/$1,950`. KAT: dependent, blind, $0 earned →
  `min(regular, max(1300, 450)) + 1950 = 1300 + 1950 = $3,250` (Single).
- **`earned_income + $450` can exceed the regular amount → the `min(regular, …)` binds.** KAT:
  dependent Single with $20,000 earned → `min(14600, max(1300, 20450)) = 14600`.

---

## 2. Schedule A engine — TY2024, per line (2024 Instructions for Schedule A, read verbatim)

Ordering constraint (all AGI-dependent lines): **Schedule A cannot be finalized until AGI is
known** (medical 7.5% floor + all §170(b) ceilings key off AGI). Sits after the AGI assembly.

### 2a. Medical & dental (lines 1–4)
Verbatim: *"expenses that exceeds **7.5%** of the amount of your adjusted gross income on Form
1040 … line 11."* (§213(a), permanent.)
`line_4 = max(0, medical_expenses − dec!(0.075) * AGI)`.

### 2b. Taxes you paid / SALT (lines 5a–5e)
Verbatim: *"**$10,000 ($5,000 if married filing separately)**."*
- 5a = state/local **income OR general sales** tax (elect one; box on 5a if sales)
- 5b = state/local **real-estate** tax
- 5c = state/local **personal-property** tax
- 5d = `5a + 5b + 5c`
- 5e = `min(cap, 5d)` where `cap = $5,000 if MFS else $10,000` (§164(b)(6), TCJA, TY2024).

`salt = min( mfs ? dec!(5000) : dec!(10000), salt_income_or_sales + salt_real_estate + salt_personal_property )`.
(TY2025 forks to the OBBBA $40k/$20k cap with the 30%-over-$500k-MAGI phase-down — out of v1 scope,
year-keyed in the table; see first-pass `02` §2b.)

### 2c. Home-mortgage interest (lines 8a–8e, 9, 10) — USER INPUT
- 8a = home-mortgage interest **and points reported on Form 1098**
- 8b = interest **not** reported on 1098; 8c = points not on 1098; 8d = reserved; 8e = `8a+8b+8c`
- 9 = investment interest (Form 4952); 10 = `8e + 9`.

Verbatim acquisition-debt limits (§163(h)): post-12/15/2017 debt **$750,000 ($375,000 MFS)**;
grandfathered 10/14/1987–12/15/2017 debt **$1,000,000 ($500,000 MFS)**; on/before 10/13/1987
uncapped. **Model deductible interest as a direct user input (the Form 1098 box-1 number).** The
tool **surfaces the $750k/$1M limit as an ADVISORY** but does **not** auto-prorate: the Pub. 936
"average of first and last balance" worksheet (needed to prorate interest when the average
balance exceeds the limit) is **explicitly out of auto-scope** — flag it and require the user to
enter the already-limited deductible interest, mirroring how the crypto engine takes user FMV.

### 2d. Charitable (lines 11–14) — the §170(b) ceiling engine

Schedule A line structure (verbatim): **11** gifts by cash/check, **12** other than by cash/check
(property), **13** carryover from prior year, **14** total (`11+12+13`). The Schedule A
instructions only print the *trigger* percentages and defer the ceilings to Pub. 526.

**Corrected ceiling classes (§170(b), Pub. 526 — supersedes first-pass `02` §2d/§2e):**

| Gift kind → donee | AGI ceiling | Code cite | Bitcoin mapping |
|---|---|---|---|
| **Cash** → 50%-org (public charity) | **60%** | §170(b)(1)(G) | (non-crypto cash) |
| **Ordinary-income property** (incl. **short-term** cap-asset reduced to basis) → 50%-org | **50%** | §170(b)(1)(A) | **BTC ≤ 1 yr** (deduct = `min(fmv,basis)`) |
| **Capital-gain property** (LT appreciated, FMV) → 50%-org | **30%** | §170(b)(1)(C)(i) | **BTC > 1 yr** (deduct = FMV) |
| Cash / ordinary-income property → **non-50%-org** (private foundation, etc.) | **30%** | §170(b)(1)(B) | (rare; capture-only v1) |
| **Capital-gain property** → non-50%-org | **20%** | §170(b)(1)(D)(i) | LT BTC to a private foundation → *also* reduced to basis (§170(e)(1)(B)(ii); crypto ≠ qualified stock) |

**Statutory ordering against AGI (Pub. 526 Worksheet 2, verified):** categories consume AGI room
in this fixed order (each later class is limited by AGI room the earlier classes leave):

1. 100% — qualified conservation (farmers/ranchers) — *not in scope*
2. **60% — cash to 50%-orgs**
3. **50% — ordinary-income property to 50%-orgs**
4. **30% — capital-gain property to 50%-orgs  *AND*  "for the use of" / to 30%-limit orgs**
5. **20% — capital-gain property to non-50%-orgs**

**The load-bearing interaction (must be in the spec):** the 30% capital-gain-property class is
capped at the **lesser of** (a) 30% × AGI, **or** (b) 50% × AGI − (cash + ordinary-income amounts
already allowed this year). I.e. the 30% class also has to fit under the overall 50%-of-AGI room.

**Carryover (§170(d)(1)):** the excess of each class over its ceiling **carries forward up to 5
years, retaining its ceiling class/character** (a 30% carryover stays a 30% item next year). Pub.
526 verbatim: *"you can carry over any excess to the next 5 tax years … you can't carry over any
amount longer than 5 years."* → carryover state must be **tagged by class**, not a flat scalar.

**Advisory to surface (out of auto-scope):** §170(b)(1)(C)(iii) lets a taxpayer **elect** to
reduce *all* capital-gain property to basis and use the **50%** limit instead of 30% (useful when
30% would strand a large carryover). The 2024 Schedule A recordkeeping instructions reference it
("How you figured your deduction if you chose to reduce your deduction for gifts of capital gain
property"). Surface as an advisory; do not auto-elect.

### 2e. How the crypto Form 8283 amount plugs in

`btctax` already computes, per donation, `Removal.claimed_deduction` =
`Σ legs ( term==LongTerm ? fmv_at_transfer : min(fmv_at_transfer, basis) )` — the **exact
§170(e)(1)(A) amount BEFORE §170(b) limits** (`SPEC_p2a_170e_deduction.md` D1; the SPEC itself
flags §170(b) limits + carryover as the deferred [R0-I2] follow-up — **this engine is that
follow-up**). Form 8283 Section A/B is already driven off the same amount
(`SPEC_p2c_form8283_709.md`). Integration:

- **Do NOT re-enter crypto donations** as Schedule A input. The ledger supplies them, already
  §170(e)-reduced and **already carrying the holding period that determines the ceiling class**:
  - crypto donation legs with `term == LongTerm` → **capital-gain property, 30% class**, at FMV;
  - crypto donation legs `ShortTerm` → **ordinary-income property, 50% class**, at basis
    (`min(fmv,basis)`).
  Both assume a **public-charity donee** — the exact assumption the 8283 SPEC already documents
  (a private-foundation donee reduces LT crypto to basis at the 20% class; retained as a caveat).
- Sum the year's crypto `claimed_deduction`, partitioned by class, and feed those two subtotals
  into the same §170(b) ceiling engine as the non-crypto property gifts. The engine's line-12
  output (noncash) + line-11 (cash) + line-13 (carryover-in) → line 14.

---

## 3. Worked example — MFJ, AGI $200,000, $5,000 cash + $70,000 LT crypto

Donee = public charity throughout. Cash → 60% class; LT crypto (FMV $70,000) → 30% capital-gain
class. Apply the Worksheet-2 ordering:

| Step | Class | Ceiling | Allowed this year | Running |
|---|---|---|---|---|
| 1 | Cash (60%) | 60% × 200,000 = **$120,000** | min(5,000, 120,000) = **$5,000** | $5,000 |
| 2 | Ord-income property (50%) | — | none | — |
| 3 | LT cap-gain crypto (30%) | min( 30%×200k=**$60,000**, 50%×200k − cash = 100,000 − 5,000 = $95,000 ) = **$60,000** | min(70,000, 60,000) = **$60,000** | $65,000 |

- **Schedule A line 11** (cash) = $5,000; **line 12** (noncash crypto) = $60,000;
  **line 14** current-year charitable = **$65,000**.
- **Carryover to next year** = $70,000 − $60,000 = **$10,000**, tagged **30% capital-gain-property
  class** (usable years 2 through 6; enters next year's line 13 in that class).

(Total $65,000 = 32.5% of AGI — comfortably under the 60% overall cash limit; the binding
constraint is the 30% capital-gain ceiling, exactly as the task states.)

---

## 4. Standard-vs-itemized decision

- **Rule:** `deduction = max( standard_deduction(status), schedule_a_line_17 )`, then
  `taxable_income = max(0, AGI − deduction − qbi)`. (Line 17 = sum of the itemized categories;
  line 18 is the elect-anyway checkbox.)
- **Compute-order placement:** the branch sits **after AGI** (every AGI-dependent Schedule A line
  needs AGI first) and **before taxable income** — i.e. stage "AGI → **[std-vs-itemized]** →
  taxable income → QDCGT line 16" in recon-01's 8-stage order. It does not touch the §1(h)
  preferential stack (which sits on top of taxable income).
- **§63(c)(6) MFS coupling — model as a hard return flag:** if one spouse itemizes on a separate
  return, the other's **standard deduction is $0** (both itemize or both take standard). The engine
  must **not** silently `max()` for an MFS filer whose spouse itemized — consume the
  `spouse_itemizes_separate` boolean (§1.2) and force `standard_deduction = 0` when set. Fail loud
  if the flag is unknown for MFS.
- **§63(e) election:** honor the taxpayer's right to elect itemized **even when smaller** (e.g. to
  match a state return). Model an explicit `force_itemize: bool` that overrides the `max()`.

---

## 5. Input-struct recommendation (refines recon-04 §5.1)

**Verdict:** take **pre-classified, per-ceiling-class** charitable inputs for the *non-crypto*
gifts; let the **crypto ledger supply the 8283 detail** (never re-typed) with its holding period →
ceiling class. The engine, not the input struct, owns the ceiling/carryover math.

```rust
// btctax-core/src/tax/return_inputs.rs — replaces recon-04's flat charitable fields
#[derive(…, Default)]
pub struct ScheduleAInput {
    #[serde(default)] pub medical: Usd,                    // gross; 7.5% floor derived
    #[serde(default)] pub salt_income_or_sales: Usd,       // 5a
    #[serde(default)] pub salt_real_estate: Usd,           // 5b
    #[serde(default)] pub salt_personal_property: Usd,     // 5c
    #[serde(default)] pub mortgage_interest: Usd,          // 8a/8b already-limited (user input)
    #[serde(default)] pub investment_interest: Usd,        // 9 (opt)
    // charitable — NON-CRYPTO gifts, pre-classified by §170(b) ceiling class:
    #[serde(default)] pub gift_cash_60: Usd,               // cash → public charity (60%)
    #[serde(default)] pub gift_ordinary_50: Usd,           // ord-income/basis property → 50%-org
    #[serde(default)] pub gift_capgain_30: Usd,            // LT appreciated property (FMV) → 50%-org
    #[serde(default)] pub gift_cash_30org: Usd,            // cash/ord → non-50%-org (rare)
    #[serde(default)] pub gift_capgain_20: Usd,            // cap-gain property → non-50%-org (rare)
    #[serde(default)] pub force_itemize: bool,             // §63(e) / line 18
    // crypto charitable is NOT here — derived from state.removals (ledger)
}

/// Prior-year charitable carryover, tagged by ceiling class + vintage (5-yr, §170(d)).
#[derive(…, Default)]
pub struct CharitableCarryoverIn {
    pub items: Vec<CarryoverItem>,   // { class: CeilingClass, amount: Usd, origin_year: i32 }
}
```

- **Why not flat `charitable_noncash`:** it collapses the 30% (LT capgain) and 50% (ordinary)
  classes, which have different ceilings and different carryover characters — the engine cannot
  reconstruct the class after the fact. This is the recon-04 §5.1 refinement.
- **Crypto flows in from the ledger** at derive time: sum `Removal{Donation}.claimed_deduction`
  for the year, split by leg `term` → add LT sum to the 30% bucket, ST sum to the 50% bucket
  (public-charity donee assumed, per the 8283 SPEC). No double-entry, single source of truth.
- **Where the engine lives:** a new **pure** module `btctax-core/src/tax/charitable.rs` (or a
  `schedule_a.rs` that hosts it) — signature roughly
  `fn charitable_deduction(agi, buckets: ClassifiedGifts, carryover_in: CharitableCarryoverIn)
   -> (line12_line11_totals, line13_used, next_year_carryover: CharitableCarryoverIn)`.
  It is AGI-driven, class-ordered (§2d ordering), emits the 5-year class-tagged carryover, and is
  called by the Schedule A aggregator, which is called by the std-vs-itemized branch. Keep it out
  of `compute.rs` (the frozen crypto-delta engine) — this is new absolute-return assembly, per the
  synthesis architecture. Carryover-out must be persisted per year so next year's line 13 reads it.

---

## Primary sources (read directly this pass)

- **Rev. Proc. 2023-34 §3.15(1)/(2)/(3)** — std deduction basic / dependent / aged-blind
  (`bradfordtaxinstitute.com/Endnotes/Rev_Proc_2023_34.pdf`, extracted verbatim via `pdftotext`).
- **2024 Instructions for Schedule A** — medical 7.5% (line 4/AGI = 1040 L11), SALT $10k/$5k
  (line 5e), mortgage $750k/$1M tiers (line 8), charitable lines 11–14 + carryover
  (`irs.gov/pub/irs-prior/i1040sca--2024.pdf`, extracted verbatim via `pdftotext`).
- **Pub. 526 "Limits on Deductions" + Worksheet 2** — 60% cash-only; 50% ordinary property; 30%
  cap-gain-to-50%-org / for-the-use-of; 20% cap-gain-to-non-50%-org; the 5-order ordering; 5-year
  class-preserving carryover (`irs.gov/publications/p526`).
- **IRC** §63(c)(2)/(c)(5)/(c)(6)/(e)/(f); §170(a)/(b)(1)(A)/(B)/(C)/(D)/(G)/(d)(1)/(e)(1);
  §164(b)(6); §163(h); §213(a) — as cited inline.
- **In-repo:** `SPEC_p2a_170e_deduction.md` (claimed_deduction = exact §170(e) pre-limit amount;
  §170(b) explicitly deferred to this engine), `SPEC_p2c_form8283_709.md` (Section A/B off the
  same amount; public-charity donee assumption), `crates/btctax-core/src/tax/tables.rs` (statutory
  vs indexed separation — std-deduction amounts are INDEXED → per-year `TaxTable`).
```
