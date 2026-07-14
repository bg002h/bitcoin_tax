# Recon 04 — Offline Structured-Input DATA MODEL (non-crypto inputs)

**Agent:** recon 4 of 5 (FIRST-PASS; Fable sweeps later). **Scope:** "Common W-2
household" federal 1040 — W-2 wages, 1099-INT/DIV, Schedule A (std-vs-itemized),
Schedule B, Schedule 1 basics, wrapping the existing crypto engine. Manual/structured
entry only (offline CLI + TOML), no OCR, no import parsing, PDF output only.

Citations to IRS.gov are box-definition-level; uncertainty is flagged inline. Tax-law
amounts (standard deduction, SALT cap) are year-indexed and belong in the existing
`btctax-adapters` per-year tables, **not** in the input model — see §5.

---

## 0. Established codebase facts (verified against current source, 2026-07-11)

The data model must slot into these existing shapes:

- **`TaxProfile`** — `crates/btctax-core/src/tax/types.rs:31-69`. Serde struct, per-year,
  stored as JSON in the `tax_profile` side-table (`crates/btctax-cli/src/tax_profile.rs`,
  keyed `year INTEGER PRIMARY KEY`). Optional fields use `#[serde(default)]` so older
  stored profiles deserialize. `Usd` is `rust_decimal::Decimal` (`conventions.rs:8`).
- **The two opaque scalars** the engine consumes:
  - `ordinary_taxable_income` — already-post-deduction ordinary slice, **excluding** the
    preferential (QD + net LTCG) slice, which the §1(h) stack sits *on top of*.
  - `magi_excluding_crypto` — MUST already include QD + non-crypto cap gains (§1411 contract,
    `cli.rs:223-234`).
- **How they feed the engine** (`crates/btctax-core/src/tax/compute.rs:336-368`):
  `bottom_with = ordinary_taxable_income + crypto_ord + net_ST − loss_deduction`; the
  preferential stack = `qualified_dividends_and_other_pref_income + other_net_capital_gain`;
  `magi_with = magi_excluding_crypto + crypto_agi_delta`. **The engine is a crypto-DELTA
  engine and never sees line items** — it only needs these derived scalars.
- **~80 `TaxProfile { … }` construction sites** across the workspace (grep: 8 in
  `btctax-cli/src`, the rest tests + TUI). The three non-test *runtime* builders:
  1. `placeholder_tax_profile()` — `cmd/tax.rs:16` (pseudo-reconcile: Single, all $0).
  2. `adhoc_profile()` — `cmd/whatif.rs:36` (what-if `--income`/`--magi` ad-hoc).
  3. the `Command::TaxProfile` handler — `main.rs:775` (CLI-flag → struct).
  Plus TUI mirrors (`whatif_panel.rs:387`, `edit/form.rs:197`, `export.rs:217`).
- **PDF layer** already exists: `crates/btctax-forms/` fills official IRS PDFs
  (`form1040.rs` currently fills ONLY line 7a cap-gain + the digital-asset question;
  everything else on the 1040 is blank). A full-return build extends this filler — it does
  not replace it.

**Design consequence (drives §5):** because the engine's contract is *derived scalars*, the
cleanest model is **additive** — a new rich `ReturnInputs` side-table that *derives* a
`TaxProfile`, leaving the engine, all 80 constructors, what-if, and pseudo-reconcile
untouched. `TaxProfile` itself becomes the "raw override / escape hatch." Detail in §5.

---

## 1. Box inventory + in-scope marking

Legend: **CALC** = feeds the common-household tax computation; **PDF** = captured only to
fill the official PDF / Schedule A/B enumeration; **OUT** = ignorable for first pass (flag).

### 1.1 Form W-2 (Wage and Tax Statement)

Source: [IRS General Instructions for Forms W-2/W-3](https://www.irs.gov/instructions/iw2w3);
box-12 codes per [IRS box-12 code list](https://www.irs.gov/instructions/iw2w3).

| Box | Name | Flows to | Status |
|----|------|----------|--------|
| 1 | Wages, tips, other comp | 1040 line 1a (Σ all W-2s) | **CALC** (core) |
| 2 | Federal income tax withheld | 1040 line 25a (Σ) | **CALC** (payments) |
| 3 | Social Security wages | already modeled → `w2_ss_wages` (SE cap) | **CALC** (existing) |
| 4 | SS tax withheld | excess-SS credit iff multi-employer over cap | OUT (edge case, flag) |
| 5 | Medicare wages | already modeled → `w2_medicare_wages` (Form 8959) | **CALC** (existing) |
| 6 | Medicare tax withheld | Form 8959 reconciliation | OUT (first pass, flag) |
| 7 | SS tips | part of SS wage base (Sch SE 8a) | CALC (fold into box 3+7) |
| 8 | Allocated tips | adds to line 1 wages | OUT (rare, flag) |
| 9 | (blank) | — | OUT |
| 10 | Dependent care benefits | Form 2441; >$5k taxable → wages | OUT (first pass) |
| 11 | Nonqualified plans | line 1 inclusion | OUT (first pass) |
| 12 | Coded amounts (a–d; code letter + $) | see below | **PDF** capture list; **no CALC** first pass |
| 13 | Checkboxes (statutory emp / retirement plan / 3rd-party sick) | "Retirement plan" → IRA-deduction phase-out | OUT (first pass, flag) |
| 14 | Other (freeform) | some → Sch A; mostly informational | OUT (first pass) |
| 15 | State + employer state ID | (no state return) | PDF only |
| 16 | State wages | state return (out of scope) | OUT (federal) |
| 17 | State income tax withheld | **Schedule A line 5a (SALT)** | **CALC** iff itemizing |
| 18 | Local wages | state/local (out of scope) | OUT |
| 19 | Local income tax | Schedule A line 5a (SALT) | CALC (minor, iff itemizing) |
| 20 | Locality name | — | PDF only |

**Box 12 codes** ([def list](https://www.irs.gov/instructions/iw2w3)): for a common W-2
household the frequent codes (D/E/G/S = elective deferrals, DD = health-coverage cost,
W = HSA, AA/BB = Roth deferrals) are **already excluded from box 1 and do NOT feed the
common-household computation**. Capture the whole `Vec<(code, amount)>` for PDF fidelity and
future credits (8880 saver's credit, 8889 HSA), but first-pass CALC uses none of them. Codes
A/B/M/N (uncollected SS/Medicare) and Z (§409A) *add* tax on Schedule 2 — rare, flag OUT.

### 1.2 Form 1099-INT

Source: [IRS Instructions for Forms 1099-INT/OID](https://www.irs.gov/instructions/i1099int).

| Box | Name | Flows to | Status |
|----|------|----------|--------|
| 1 | Interest income | 1040 line 2b / Sch B Part I | **CALC** |
| 2 | Early withdrawal penalty | Schedule 1 line 18 (adjustment) | **CALC** (adjustment) |
| 3 | Interest on US savings bonds / Treasury obligations | taxable interest, line 2b | **CALC** (fold into taxable interest) |
| 4 | Federal income tax withheld (backup) | 1040 line 25b (Σ) | **CALC** (payments) |
| 5 | Investment expenses | post-TCJA nondeductible | OUT |
| 6 | Foreign tax paid | Sch 3 line 1 / Form 1116 FTC | OUT (first pass, flag) |
| 7 | Foreign country | — | OUT |
| 8 | Tax-exempt interest | 1040 line 2a | **PDF/CALC-adjacent** (see note) |
| 9 | Specified private activity bond interest | AMT (Form 6251) | OUT (AMT out of scope, flag) |
| 10–13 | Market discount / bond premium (×3) | interest basis adjustments | OUT (first pass, flag) |
| 14 | Tax-exempt CUSIP | — | OUT |
| 15–17 | State info | (no state return) | OUT |

**Box 8 note:** tax-exempt interest lands on 1040 line 2a and is required for a faithful PDF,
but for the §1411 NIIT `magi_excluding_crypto` it is **NOT** added back (muni interest is
excluded from NIIT MAGI). *Uncertain / flag:* it IS a MAGI add-back for other thresholds
(e.g. IRA, IL/SS taxation) that are out of first-pass scope. Capture it, exclude from the
NIIT MAGI derivation.

### 1.3 Form 1099-DIV

Source: [IRS Instructions for Form 1099-DIV](https://www.irs.gov/instructions/i1099div).

| Box | Name | Flows to | Status |
|----|------|----------|--------|
| 1a | Total ordinary dividends | 1040 line 3b / Sch B Part II | **CALC** |
| 1b | Qualified dividends (subset of 1a) | 1040 line 3a → preferential stack | **CALC** (→ `qualified_dividends`) |
| 2a | Total capital gain distributions | Sch D line 13 (LT-character) | **CALC** (→ `other_net_capital_gain`) |
| 2b | Unrecaptured §1250 gain | 25% worksheet | OUT (Sch D 17-22 already out of scope) |
| 2c | Section 1202 gain | — | OUT |
| 2d | Collectibles (28%) gain | 28% worksheet | OUT (out of scope) |
| 2e/2f | §897 ordinary / cap gain (FIRPTA) | — | OUT |
| 3 | Nondividend distributions | return of capital (non-taxable, basis ↓) | OUT (capture optional) |
| 4 | Federal income tax withheld (backup) | 1040 line 25b (Σ) | **CALC** (payments) |
| 5 | Section 199A dividends (REIT) | QBI 20% deduction (Form 8995) | CALC-adjacent (**flag QBI**; capture) |
| 6 | Investment expenses | nondeductible | OUT |
| 7 | Foreign tax paid | Sch 3 / FTC | OUT (first pass, flag) |
| 8 | Foreign country | — | OUT |
| 9/10 | Cash / noncash liquidation | — | OUT |
| 11 | FATCA filing requirement | — | OUT |
| 12 | Exempt-interest dividends | 1040 line 2a (muni-fund) | PDF (feeds line 2a like INT box 8) |
| 13 | Specified priv. activity bond int. dividends | AMT | OUT |
| 14–16 | State info | — | OUT |

**Key CALC interactions:** 1a *includes* 1b (do not double-count — see §5 derivation).
Box 2a is LT-character capital gain that flows into the **same** `other_net_capital_gain`
channel the engine already nets against crypto Schedule D — a real coupling point to test.
Box 5 (§199A) triggers a QBI deduction that reduces taxable income; QBI is genuinely
in-scope for a REIT-holding household but the §199A computation is nontrivial — **flag for a
scope decision** (capture the box now; compute QBI in a later chunk or via a manual
`qbi_deduction` override).

---

## 2. Multi-source aggregation (collection schema)

A household has multiple employers and multiple payers; Schedule B enumerates each payer by
name. The model holds a **`Vec` per form type per year**. Each element carries a `payer`
label (employer / bank / brokerage name) for the Schedule B / W-2 enumeration and PDF.

```rust
// crates/btctax-core/src/tax/return_inputs.rs  (NEW module)
use crate::conventions::Usd;
use serde::{Deserialize, Serialize};

/// One W-2. Only CALC/PDF-relevant boxes are typed; the rest are captured opaquely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct W2 {
    pub employer: String,           // for enumeration / PDF (not on 1040 body but on W-2)
    pub box1_wages: Usd,            // line 1a
    pub box2_fed_withheld: Usd,     // line 25a
    #[serde(default)] pub box3_ss_wages: Usd,        // → w2_ss_wages (SE cap)
    #[serde(default)] pub box5_medicare_wages: Usd,  // → w2_medicare_wages (8959)
    #[serde(default)] pub box7_ss_tips: Usd,
    #[serde(default)] pub box17_state_tax_withheld: Usd, // → Sch A SALT
    /// Box 12 coded amounts, captured verbatim (code letter → dollars). No first-pass CALC.
    #[serde(default)] pub box12: Vec<Box12Entry>,
    /// Box 13 checkboxes (retirement_plan affects future IRA logic; captured now).
    #[serde(default)] pub retirement_plan: bool,
    #[serde(default)] pub statutory_employee: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Box12Entry { pub code: String, pub amount: Usd } // e.g. ("DD","18000.00")

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Form1099Int {
    pub payer: String,
    pub box1_interest: Usd,             // line 2b
    #[serde(default)] pub box2_early_withdrawal_penalty: Usd, // Sch 1 line 18
    #[serde(default)] pub box3_treasury_interest: Usd,        // line 2b (folded)
    #[serde(default)] pub box4_fed_withheld: Usd,             // line 25b
    #[serde(default)] pub box8_tax_exempt_interest: Usd,      // line 2a (PDF; NOT NIIT MAGI)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Form1099Div {
    pub payer: String,
    pub box1a_ordinary_dividends: Usd,     // line 3b (INCLUDES 1b)
    #[serde(default)] pub box1b_qualified_dividends: Usd, // line 3a (pref stack)
    #[serde(default)] pub box2a_capital_gain_distr: Usd,  // → other_net_capital_gain
    #[serde(default)] pub box4_fed_withheld: Usd,         // line 25b
    #[serde(default)] pub box5_section_199a_dividends: Usd, // QBI (flag)
    #[serde(default)] pub box12_exempt_interest_dividends: Usd, // line 2a (PDF)
}
```

Rationale: only-`Default` + `#[serde(default)]` on every optional box keeps the JSON small
and forward/backward compatible (same discipline as `TaxProfile`). Typing boxes individually
(not a generic map) gives the derivation and PDF-filler compile-time field access.

---

## 3. Personal / header info (PII → encrypted vault side-table)

**This is entirely new to the app** — `btctax` today holds *no* identity data (it is a
crypto ledger). Names, SSNs, address, and dependents are the highest-sensitivity data in the
product and MUST live inside the existing encrypted `vault.pgp` (never a plaintext side file;
the only plaintext exception is the explicit `export-snapshot` / `export-irs-pdf`, which
already warns and writes owner-only outside any repo).

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Person {
    pub first_name: String,
    pub last_name: String,
    pub ssn: String,              // 9 digits; store normalized, render masked in TUI
    #[serde(default)] pub occupation: String, // 1040 signature block (optional)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Dependent {
    pub name: String,
    pub ssn: String,
    pub relationship: String,     // "son", "daughter", "parent", …
    pub ctc_eligible: bool,       // child tax credit (qualifying child < 17)
    #[serde(default)] pub odc_eligible: bool, // $500 credit for other dependents
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HouseholdHeader {
    pub taxpayer: Person,
    #[serde(default)] pub spouse: Option<Person>,   // Some(..) for MFJ/MFS
    pub address_street: String,
    pub address_city: String,
    pub address_state: String,
    pub address_zip: String,
    #[serde(default)] pub dependents: Vec<Dependent>,
    #[serde(default)] pub presidential_fund_taxpayer: bool, // header checkbox
    #[serde(default)] pub presidential_fund_spouse: bool,
}
```

**Storage-shape decision (recommend):** fold `HouseholdHeader` **into the same per-year
`ReturnInputs` blob** (§5) rather than a separate un-keyed table. Filing status, address,
and dependents legitimately change year to year and the 1040 is a per-year document, so a
year-keyed blob is the faithful model and matches the `tax_profile` side-table exactly.
*(Alternative considered: a separate identity table with a "copy-forward last year" helper to
avoid re-typing SSNs. Recommend the per-year blob + a `income copy-from --year` CLI helper —
keeps one storage discipline, avoids a second table.)* Flag: SSN validation/masking is a
security-review item for the implement phase.

---

## 4. Withholding & payments (the refund/owed bottom line)

To produce the 1040 payments section (lines 25a/25b/26) and the final refund-or-owed figure,
the model needs withholding (already per-form above) plus estimated payments:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Payments {
    /// 1040 line 26 — Σ of the four quarterly estimates + any prior-year overpayment applied.
    #[serde(default)] pub estimated_tax_payments: Usd,
    /// Line 25c — other withholding not on a W-2/1099 (e.g. W-2G, 1099-R). Optional escape hatch.
    #[serde(default)] pub other_withholding: Usd,
}
```

W-2 box 2 and 1099 box 4 are **summed at derivation time** from the `Vec`s (line 25a = Σ W-2
box 2; line 25b = Σ 1099-INT box 4 + Σ 1099-DIV box 4), so they are not duplicated here.
Refund/owed = total tax (crypto engine delta + non-crypto stack) − (25a+25b+26). **Note:** the
*current* engine reports only a crypto *delta*, not total liability — computing an absolute
refund/owed is a new capability that pairs with this model (call it out to the plan owner; it
is the reason the derivation in §5 must produce a full ordinary/pref stack, not just a delta).

---

## 5. THE key design question — replacing the two opaque scalars

### 5.1 Recommendation: additive `ReturnInputs` that *derives* `TaxProfile` (do NOT rewrite `TaxProfile`)

Rewriting `TaxProfile` to hold line items would touch **~80 construction sites** and every
engine test — the documented "single biggest breaking change." It is also unnecessary,
because the engine only ever needs the derived scalars. Recommended shape:

```rust
/// The full-return household inputs for one tax year. Persisted as JSON in a NEW
/// `return_inputs` side-table (year PRIMARY KEY), mirroring `tax_profile` exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReturnInputs {
    pub filing_status: FilingStatus,
    pub header: HouseholdHeader,
    #[serde(default)] pub w2s: Vec<W2>,
    #[serde(default)] pub int_1099: Vec<Form1099Int>,
    #[serde(default)] pub div_1099: Vec<Form1099Div>,
    #[serde(default)] pub schedule_a: Option<ScheduleA>, // None ⇒ take standard deduction
    #[serde(default)] pub sch1_income: Sch1Income,       // basic Schedule 1 Part I
    #[serde(default)] pub sch1_adjustments: Sch1Adjustments, // Part II
    #[serde(default)] pub payments: Payments,
    #[serde(default)] pub capital_loss_carryforward_in: Carryforward, // reuse core type
    #[serde(default)] pub qbi_deduction_override: Usd,   // §199A escape hatch (flag)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScheduleA {
    #[serde(default)] pub medical: Usd,          // deductible portion (> 7.5% AGI floor is derived)
    #[serde(default)] pub salt_income_or_sales: Usd, // Σ W-2 box17 + est. — capped at year limit
    #[serde(default)] pub salt_property: Usd,
    #[serde(default)] pub mortgage_interest: Usd,
    #[serde(default)] pub charitable_cash: Usd,
    #[serde(default)] pub charitable_noncash: Usd, // NOTE: crypto donations already flow via the ledger
}
```

**Derivation** — a pure function on `ReturnInputs` that returns the engine's `TaxProfile`.
It needs the year's tax table (standard-deduction / SALT-cap amounts are year-indexed and
already live in `btctax-adapters`, so pass `&tables`):

```rust
impl ReturnInputs {
    pub fn derive_tax_profile(&self, tables: &impl TaxTables, year: i32) -> TaxProfile { … }
}
```

with logic (crypto excluded throughout — the engine adds crypto on top):

1. `wages       = Σ w2.box1`                                  → 1040 1a
2. `taxable_int = Σ int.box1 + Σ int.box3`                    → 1040 2b
3. `ord_div     = Σ div.box1a`   (1a already includes 1b)     → 1040 3b
4. `qual_div    = Σ div.box1b`                                → 1040 3a (preferential)
5. `cap_gain_distr = Σ div.box2a`                             → LT-character (`other_net_capital_gain`)
6. `sch1_add    = sch1_income totals` ; `adjustments = sch1_adjustments (+ Σ int.box2 penalty)`
7. `AGI            = wages + taxable_int + ord_div + sch1_add − adjustments`
   *(non-crypto AGI; crypto AGI delta is added by the engine)*
8. `deduction      = max(standard_deduction(status, year), schedule_a_total(AGI))`
9. `qbi            = qbi_from_box5(...) or qbi_deduction_override` (flag)
10. `taxable_income = max(0, AGI − deduction − qbi)`
11. **`ordinary_taxable_income = max(0, taxable_income − qual_div − cap_gain_distr)`**
    *(strip the preferential slice — it re-enters via the pref fields below)*
12. `TaxProfile {
        filing_status,
        ordinary_taxable_income,                                  // step 11
        magi_excluding_crypto: AGI,                               // step 7 (incl QD + cap gain — §1411 contract)
        qualified_dividends_and_other_pref_income: qual_div,      // step 4
        other_net_capital_gain: cap_gain_distr,                   // step 5
        capital_loss_carryforward_in,
        w2_ss_wages:       Σ (w2.box3 + w2.box7),                 // existing SE-cap channel
        w2_medicare_wages: Σ w2.box5,                             // existing 8959 channel
        schedule_c_expenses: sch1 business expenses (if modeled), // existing channel
    }`

This reuses **every** existing engine field with its exact current meaning — the engine,
`compute.rs`, and all tests are byte-for-byte unaffected. Only the *source* of the scalars
changes (derived vs. hand-entered).

**Subtleties to test (real bugs live here):**
- Box 1a **includes** 1b — use 1a for the ordinary total, 1b only for the preferential split
  (step 11 subtracts qual_div once). Double-subtracting is the obvious bug.
- Capital gain distributions (box 2a) share the `other_net_capital_gain` channel with crypto
  Schedule D netting — the coupling must be regression-tested.
- `magi_excluding_crypto = AGI` is correct **only** because tax-exempt interest is *not* a
  §1411 add-back; keep box 8 / box 12 (exempt-int) out of AGI. Flag for Fable.
- Standard-vs-itemized picks the max; SALT is capped at the year's limit (2024 = $10k;
  2025+ raised under OBBBA per the tables — do NOT hardcode).

### 5.2 Backward-compat / migration — the escape hatch already exists

The "raw taxable-income override" the task asks about is **literally the current
`TaxProfile`**. Keep both entry paths:

- **Full-return path:** user populates `ReturnInputs` (side-table `return_inputs`) →
  `derive_tax_profile()` → engine. The 1040 PDF filler reads `ReturnInputs` directly for line
  items + payments; the engine reads the derived `TaxProfile`.
- **Raw / crypto-only / what-if path:** unchanged. `placeholder_tax_profile()`,
  `adhoc_profile()`, and `tax-profile set …` still build a `TaxProfile` with hand-entered
  scalars. No migration, no breakage.

**Resolution order at `report --tax-year` / `report_tax_year` (`cmd/tax.rs:88`):**
1. a stored `ReturnInputs` for the year → `derive_tax_profile()` (full return); else
2. a stored `TaxProfile` (raw override — today's behavior); else
3. pseudo-reconcile placeholder (if the mode is on); else
4. `TaxProfileMissing` blocker (unchanged).

This makes the change **purely additive**: no existing constructor, test, or the pseudo /
what-if builders change. `ReturnInputs` and its side-table are new files; `TaxProfile` and
`compute.rs` are frozen.

### 5.3 The one real migration risk (biggest)

**Two coexisting sources of truth for the same year.** If a year has BOTH a hand-entered
`TaxProfile` AND a `ReturnInputs`, the engine would silently prefer one and the user could be
filing off stale scalars. Mitigations to specify: (a) a strict precedence rule (above) that
is *loud* — `report`/TUI must show WHICH source produced the numbers; (b) a "profile is
derived, do not hand-edit" advisory when a `ReturnInputs` exists; (c) consider making
`tax-profile set` refuse (or warn hard) when a `ReturnInputs` already exists for that year, to
prevent divergence. This is the item most likely to produce a *wrong number presented as
authoritative* (the app's cardinal sin, `types.rs:114`) and must be nailed in the spec.

---

## 6. CLI / entry surface (offline, matches existing conventions)

Two complementary surfaces, both offline and consistent with the existing flag style
(`--long-kebab`, `Option<String>` money args parsed via `eventref::parse_usd_arg`, per-year
`--year`, JSON side-table persistence):

### 6.1 Incremental subcommands (mirrors `reconcile` / `optimize` subcommand trees)

```
btctax income add-w2       --year 2025 --employer "ACME" --box1 82000 --box2 9100 \
                           --box3 82000 --box5 82000 [--box17 3200] [--box12 DD=18000,W=4000]
btctax income add-1099-int --year 2025 --payer "Ally Bank" --box1 1240 [--box4 0] [--box8 0]
btctax income add-1099-div --year 2025 --payer "Vanguard" --box1a 3400 --box1b 3100 \
                           --box2a 900 [--box4 0] [--box5 0]
btctax income list         --year 2025            # enumerate stored W-2s / 1099s (Schedule B preview)
btctax income remove-w2    --year 2025 --index 0  # by-index removal (append/remove, like reconcile)
btctax income copy-from    --year 2025 --from 2024 # copy header/dependents forward (avoid re-typing SSNs)

btctax deductions set      --year 2025 [--standard | --itemize] \
                           [--mortgage-interest N] [--salt-income N] [--salt-property N] \
                           [--charitable-cash N] [--charitable-noncash N] [--medical N]

btctax dependents add      --year 2025 --name "…" --ssn "…" --relationship son --ctc
btctax household set        --year 2025 --taxpayer-name "…" --taxpayer-ssn "…" \
                           --filing-status mfj --spouse-name "…" --spouse-ssn "…" \
                           --address "…" --city "…" --state "…" --zip "…" [--presidential-fund]

btctax payments set        --year 2025 --estimated 6000 [--other-withholding 0]
```

Each mutating subcommand opens the vault, edits the `ReturnInputs` for `--year`, and saves —
identical to how `tax_profile::set` works today. `income list` is the Schedule B enumeration
preview. SSN entry echoes masked; a `--stdin` variant lets SSNs avoid the shell history.

### 6.2 TOML import (bulk, offline — a *file*, not a network import)

For a household with many forms, a single offline TOML mirrors `ReturnInputs` 1:1 and is the
ergonomic bulk entry path. **This is structured manual entry (the user types/pastes the TOML),
not OCR or broker-file parsing** — squarely in scope.

```toml
# btctax income import --year 2025 household.toml
filing_status = "mfj"

[header]
taxpayer = { first_name = "Pat", last_name = "Doe", ssn = "123-45-6789" }
spouse   = { first_name = "Sam", last_name = "Doe", ssn = "987-65-4321" }
address_street = "1 Main St"; address_city = "Austin"; address_state = "TX"; address_zip = "78701"
presidential_fund_taxpayer = true

[[header.dependents]]
name = "Kid Doe"; ssn = "111-22-3333"; relationship = "daughter"; ctc_eligible = true

[[w2s]]
employer = "ACME"; box1_wages = "82000"; box2_fed_withheld = "9100"
box3_ss_wages = "82000"; box5_medicare_wages = "82000"; box17_state_tax_withheld = "3200"
box12 = [ { code = "DD", amount = "18000" }, { code = "W", amount = "4000" } ]

[[int_1099]]
payer = "Ally Bank"; box1_interest = "1240"

[[div_1099]]
payer = "Vanguard"; box1a_ordinary_dividends = "3400"; box1b_qualified_dividends = "3100"; box2a_capital_gain_distr = "900"

[schedule_a]
mortgage_interest = "11200"; salt_income_or_sales = "3200"; salt_property = "6800"; charitable_cash = "2500"

[payments]
estimated_tax_payments = "6000"
```

Recommend shipping **both**: subcommands for incremental edits + TOML for bulk. The TOML
deserializes straight into `ReturnInputs` (serde already derives it), so the import command is
a thin `toml::from_str` + `return_inputs::set`. `income show --year --toml` round-trips it back
out for editing.

---

## 7. Open questions / uncertainty flags (for Fable + spec)

1. **QBI / §199A (1099-DIV box 5):** genuinely in-scope for a REIT-holding household but the
   §199A computation (Form 8995 / 8995-A, taxable-income thresholds) is nontrivial. First-pass
   recommendation: capture box 5 + a `qbi_deduction_override`, defer the auto-computation.
2. **Absolute liability vs. delta:** the current engine reports a crypto *delta*, not total
   tax. A refund/owed bottom line (§4) needs the full ordinary+pref+credits stack. Confirm the
   plan owner intends this new capability (recon 1–3/5 may own the "1040 assembly" layer).
3. **Child Tax Credit / Credit for Other Dependents:** dependents are captured (§3) but the CTC
   computation (phase-out at MAGI thresholds, $2,000/$500) is a *credit*, not modeled here.
   Flag: is CTC in first-pass scope, or capture-only?
4. **Tax-exempt interest MAGI treatment:** confirmed NOT a §1411 NIIT add-back; flag that it
   IS an add-back for other (out-of-scope) thresholds so a later chunk doesn't mis-wire it.
5. **AMT (1099-INT box 9, 1099-DIV box 13):** out of first-pass scope; capture-only for PDF.
6. **Excess Social Security credit (W-2 box 4, multi-employer):** common enough in a
   two-earner household to eventually matter; OUT first pass, flag.
7. **SALT cap year-indexing:** 2024 = $10k, 2025+ raised (OBBBA). MUST come from the
   `btctax-adapters` per-year tables, never hardcoded in the input model.

---

## Appendix — files this model touches (implement-phase map)

- **NEW** `crates/btctax-core/src/tax/return_inputs.rs` — `ReturnInputs`, `W2`, `Form1099Int`,
  `Form1099Div`, `HouseholdHeader`, `Person`, `Dependent`, `ScheduleA`, `Payments`,
  `derive_tax_profile()`. Re-export from `btctax-core` lib.
- **NEW** `crates/btctax-cli/src/return_inputs.rs` — the `return_inputs` side-table
  (init/get/set/all), copy-paste discipline from `tax_profile.rs`.
- **EDIT** `crates/btctax-cli/src/cli.rs` — new `Income` / `Deductions` / `Dependents` /
  `Household` / `Payments` subcommand trees + `FilingStatusArg` reuse.
- **EDIT** `crates/btctax-cli/src/cmd/tax.rs` (`report_tax_year`) — resolution order (§5.2).
- **EDIT** `crates/btctax-forms/src/form1040.rs` (+ new schedule_a/schedule_b fillers) — read
  `ReturnInputs` for the income/deduction/payments lines the crypto-only path leaves blank.
- **FROZEN** `crates/btctax-core/src/tax/types.rs` (`TaxProfile`) + `compute.rs` — unchanged;
  the whole point of the additive design.
```
