# RECON — the interview surface btctax already has, per 1040 line

**Agent:** read-only recon. **Repo:** `/scratch/code/bitcoin_tax`, branch `main`, HEAD `36bddc75`.
**Date:** 2026-09-06. **Scope:** what btctax can elicit today, per Form 1040 line and per attached
form line; what is TOML-import-only; what it cannot take at all.

**Nothing was edited.** Every number below is the output of a command recorded beside it.

---

## 0. Measurement commands

| # | command | used for |
|---|---|---|
| M1 | `cargo nextest run --locked -p btctax-input-form -E 'test(coverage)'` | 1 passed — the 98-Field coverage KAT is green |
| M2 | `cargo nextest run --locked -p btctax-forms -E 'binary(field_census)'` | 5 passed — the per-field census gate is green |
| M3 | `cargo nextest run --locked -p btctax-cli -E 'test(input_form_store)'` | 15 passed, incl. `commit_non2024_is_notables_and_writes_nothing` |
| M4 | python text-scan of `crates/btctax-forms/forms/*/*.map.toml` using **the gate's own rule** (`field_census.rs:170-190`: FQN = a quoted chunk containing `[0]` and `.`, split at the `[census]` header) | mapped / censused per form |
| M5 | python regex over `spec/sections.rs` + `spec/registries.rs` for `id: FieldId::` and `<macro>!(<idx>, FieldId::` | Fields per section |
| M6 | python regex over `return_inputs.rs` for `pub struct X { … pub f: … }` | struct field inventories |
| M7 | `grep -c` / regex over `return_refuse.rs`, `state.rs`, `advisories.rs`, `attribute.rs` | enum cardinalities |

---

## A. The interview surface today — five surfaces, and only one of them is an interview

### A.0 Summary

| surface | shape | reach | year gate |
|---|---|---|---|
| **A1** TUI "tax inputs" form (`btctax-tui-edit`, key `T`) | 16 sections, **98 Fields** (M1, M5) | the only *authoring* interview | **commit works on TY2024 only** (M3) |
| **A2** `btctax income answer --year N` | **36** registry questions (17 declarations + 19 skippables), liveness-filtered, one pass | yes/no + dates only — **no money** | any year, but **requires an existing row** created by `income import` |
| **A3** `btctax income import --year N --file X.toml` | the **whole** `ReturnInputs` — **40 top-level fields** (M6) | everything, including the 8 structs the form has no section for | any year; **no `income template` command, no schema doc, no screen at import** |
| **A4** `btctax tax-profile --year N …` | 11 scalar CLI flags | crypto-slice escape hatch only; a stored `ReturnInputs` outranks it | any year with a `TaxTable` |
| **A5** `btctax import <files>` + `btctax reconcile <27 subcommands>` + `btctax config --set-forward-method` | 4 exchange adapters, 22 `EventPayload` variants | the Bitcoin side | any year |

★ **The finding that dominates everything else:** the interactive form's `commit` returns
`CommitOutcome::NoTables` for any year without bundled `FullReturnParams`
(`crates/btctax-cli/src/input_form_store.rs:369-376`), and `full_return_for` is `Some` for
**TY2024 alone** (`crates/btctax-adapters/src/tax_tables.rs:102` inserts only 2024; TY2025 and TY2026
are held fail-closed by `ty2025_full_return_must_stay_fail_closed_until_complete` and
`ty2026_full_return_must_stay_fail_closed`). So **the only year a filer can author a return
interactively is a year nobody is filing.** TY2026 — the owner's target year per
`ROADMAP_STATUS.md` §0a — can only receive a return through `income import`'s TOML.

### A1 — the TUI input form: 16 sections, 98 Fields, in this order

`crates/btctax-input-form/src/spec/mod.rs:19-38` is the render order. Counts measured by M5; the
98 total is pinned by the coverage KAT (`spec/coverage.rs:471-484`, "expected 98 Fields").

| # | `SectionId` | kind | n | fields | where the answer lands |
|---|---|---|---|---|---|
| 1 | `ReturnOptions` | singleton | 2 | `FilingStatus`, `ItemizeElection` | 1040 filing-status boxes; §63(e) election |
| 2 | `Taxpayer` | singleton | 6 | first/last/ssn/occupation/presidential-fund/**IP PIN** | 1040 header + p2 signature block |
| 3 | `Spouse` | optional-singleton | 5 | same minus IP PIN | 1040 header |
| 4 | `Address` | singleton | 4 | street/city/state/zip | 1040 header (**domestic only** — see §B) |
| 5 | `Dependents` | repeating | 4 | name/ssn/relationship/**DOB** | 1040 dependents table rows 1–4 + overflow |
| 6 | `W2s` | repeating | 14 | owner, employer, **EIN**, boxes 1,2,3,4,5,6,7,8,10,17,19 | 1040 1a/25a; 8959; Sch A 5a; §6413(c) |
| 7 | `W2Box12` | repeating (nested) | 2 | code, amount | §402(g) screen; refusals |
| 8 | `ScheduleA` | optional-singleton | 12 | medical, SALT ×5, mortgage-1098, investment interest, **Form 8960 line 9b**, + 3 registry tri-states (sales-tax election, mixed-use mortgage, §163(h)(3)(B) debt limit) | Sch A 1/5a-5c/8a/9; 8960 9b |
| 9 | `ScheduleACharitable` | repeating (nested) | 2 | class (6-variant enum), amount | Sch A 11/12; Form 8283 |
| 10 | `Payments` | singleton | 3 | estimated, extension, other withholding | 1040 26; Sch 3 10; 1040 25c |
| 11 | `Carryforwards` | singleton | 2 | QBI REIT/PTP carryforward-in, QBI carryforward-in | Form 8995 lines 3/7 |
| 12 | `QbiLimitation` | singleton | 2 | W-2 wages, UBIA | Form 8995-A lines 4/7 |
| 13 | `BrokerReporting` | repeating, **rows seeded from the ledger** | 2 | covered / noncovered (5-variant enum each) | Form 8949 box routing (G/H/I, J/K/L) |
| 14 | `Declarations` | singleton | 16 | 15 tri-state declarations + Schedule B 7b country text | fail-loud boxes; Form 6251 2k/2l/3 |
| 15 | `IncomeExclusions` | singleton | 4 | PR income, 2555 L45, 2555 L50, 4563 L15 | Schedule 1-A Part I / SALT worksheet |
| 16 | `Skippables` | singleton | 18 | blind ×2, DOB ×2, DOD ×2, FBAR, death gates ×2, Sch C 1099 pair, donation restrictions, SSTB, co-op patron, §170(f)(8) CWA, Form 8615 trio | §63(f); Sch B; Sch C I/J; 8283 5a-c; 8995-A |
| | | | **98** | | |

**Order note:** the render order is *structural* (identity → income → deductions → payments →
declarations), not a walk of the 1040. Nothing sequences a filer through a form.

### A2 — `income answer`: 36 questions, and it cannot create a return

`crates/btctax-cli/src/cmd/answer.rs:52-68` — `live_questions(ri)` = `FORM_QUESTIONS` filtered by
`live`, then `SKIPPABLE_QUESTIONS` filtered by `live`. Cardinalities measured (M7):
**`FORM_QUESTIONS` = 17**, **`SKIPPABLE_QUESTIONS` = 19** (`crates/btctax-core/src/tax/questions.rs:286`,
`:985`).

It refuses on a year with no row — `answer.rs:116-119`: *"Refuses on a year with no row: only
`income import` creates one. Answering questions about a return that does not exist would
materialize a near-empty blob, which then takes PRECEDENCE over the user's `tax-profile`."*

### A3 — `income import`: the TOML is the wire, and it is unscreened

`crates/btctax-cli/src/cmd/tax.rs:49-209` (`import_return_inputs`). Measured: **`screen_inputs`
appears zero times in that function** — the fail-closed screen runs later at
`resolve.rs:96`, `input_form_store.rs:377`, `admin.rs:1488`. This is exactly the defect
`SPEC_input_surface.md` §1 recorded in 2026-07: *"`screen_inputs` … appears **zero times** in
`cmd/tax.rs`. It runs only later, at `report` and `export-irs-pdf`."* Still true.

`btctax income template` and `btctax set-pii` were **specced and never built** — `IncomeCmd` has
exactly five variants (`cli.rs:464`): `Import`, `Show`, `Scrub`, `Clear`, `Answer`. The `income
answer` help text still points at `set-pii` (`cli.rs:553`), which does not exist.

★ **Partial correction to `SPEC_input_surface.md` §1's "no example":** there *is* now a committed,
**generated** worked example — `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`,
170 lines, serialized from `btctax_core::tax::testonly::kitchen_sink_household()` and pinned against
the oracle vector, carrying inline comments on the fail-loud lines (*"★ DELETING THIS LINE REFUSES
THE RETURN (CharitableCwaUnresolved)"*). But it lives at a **test-fixture path**, and the published
walkthrough (`docs/examples/examples.md:488-493`, `docs/examples-tui-walkthrough/j6/00-setup.console.md:41`)
names `fullreturn.toml` without showing its contents. So a real filer still has no discoverable
starting file, and there is still nothing that *validates* one at import time.

### A4 — `tax-profile`

11 flags (`cli.rs:325-390`): filing status, ordinary taxable income, MAGI, qualified dividends,
other net capital gain, ST/LT carryforward, W-2 SS wages, W-2 Medicare wages, Schedule C expenses,
`--show`, `--force`. It feeds the **crypto slice only** — it never produces a 1040.

### A5 — the exchange side

`btctax import <files>` auto-detects one of four adapters; `btctax reconcile` has **27**
subcommands (`cli.rs`, `Reconcile` enum, M7). Full detail in §E.

---

## B. Line-level coverage map

**Provenance key.** **I** = interview (a TUI form `Field` or an `income answer` registry question) ·
**T** = TOML-import only (a `ReturnInputs` struct with no form section) · **L** = ledger (exchange
import + reconcile) · **C** = computed from other printed lines · **K** = constant the form prints ·
**X** = censused `rule = "unmodeled"` — **btctax cannot take it at all** · **A** = censused
`rule = "artifact"` (reserved cell / another party's block).

Sources: `crates/btctax-core/src/tax/printed.rs` (the printed chains and their doc comments) and the
`[census]` sections of `crates/btctax-forms/forms/2024/*.map.toml`.

### B.1 Form 1040 (TY2024), every line

| line | what | prov. | from |
|---|---|---|---|
| 1a | Σ W-2 box 1 | **I** | `W2s.Box1Wages` |
| 1b | household employee wages | **X** | no household employment modelled |
| 1c | tip income not on 1a | **X** | *"btctax … never asks whether the filer received tips withheld from the employer … in the understatement direction"* |
| 1d | Medicaid waiver payments | **X** | |
| 1e | taxable dependent-care benefits (2441) | **X** | W-2 box 10 > 0 **refuses** (`DependentCareBenefit`) |
| 1f | adoption benefits (8839) | **X** | box 12 code T is refused by the allowlist |
| 1g | Form 8919 wages | **X** | reaches btctax as SE income instead (overstates) |
| 1h | other earned income | **X** | |
| 1i | combat-pay election | **X** | |
| 1z | add 1a–1h | **C** | `= 1a` |
| 2a | tax-exempt interest | **T** | `int_1099[].box8` + `div_1099[].box12` |
| 2b | taxable interest | **T** | `int_1099[].box1 + box3` via Schedule B line 4 |
| 3a | qualified dividends | **T** | `div_1099[].box1b` |
| 3b | ordinary dividends | **T** | `div_1099[].box1a` via Schedule B line 6 |
| 4a/4b | **IRA distributions** | **X** | *"btctax models no IRA"* |
| 5a/5b | **pensions and annuities** | **X** | *"not modelled"* |
| 6a/6b/6c | **Social Security benefits** | **X** | *"collects no SSA-1099 and runs no §86 provisional-income worksheet"* |
| 7 | capital gain/(loss) | **L** + **T** | Schedule D 16 / −(D 21). Crypto from the ledger; `b_1099[]` and `div_1099[].box2a` by TOML |
| 8 | Schedule 1 line 10 | **T**/**L** | see B.2 |
| 9 | total income | **C** | sums printed 1z, 2b, 3b, 7, 8 |
| 10 | Schedule 1 line 26 | **C**/**T** | see B.2 |
| 11 | AGI | **C** | |
| 12 | deduction claimed | **I** or **C** | Schedule A line 17 (I) else §63 standard (C, keyed to filing status + DOB/blind skippables) |
| 13 | §199A QBI | **C**+**I**+**T** | Form 8995 L15 / 8995-A; `QbiW2Wages`/`QbiUbia`/carryforwards are **I**; `div_1099[].box5` is **T** |
| 14–15 | add / taxable income | **C** | |
| 16 | tax | **C** | Tax Table / TCW / QDCGT. The three alternative-computation boxes (8814/4972/write-in) are **X** |
| 17 | Schedule 2 line 3 | **C**+**I** | Form 6251 AMT; its three declarations (2k/2l/3) are **I** |
| 18 | add 16 + 17 | **C** | |
| **19** | **CTC / credit for other dependents** | **never populated** | `Option<Usd>` and btctax never fills it. The four dependent-row CTC/ODC checkboxes ARE mapped and are **deliberately never checked** — `crates/btctax-forms/src/form1040_full.rs:539`: *"row.ctc / row.odc are deliberately NOT checked — v1 omits the credit (L19 = 0)."* `Advisory::CtcOdcOmitted` discloses it |
| 20 | Schedule 3 line 8 | **T** or blank | = FTC only (`int_1099.box6` / `div_1099.box7`); `Advisory::OtherCreditsOmitted` fires unconditionally |
| 21–22 | add / subtract | **C** | |
| 23 | Schedule 2 line 21 | **C**+**I** | SE tax + 8959 + 8960; `Nii8960Line9b` is **I** |
| 24 | **total tax** | **C** | |
| 25a | Σ W-2 box 2 | **I** | |
| 25b | Σ 1099 box 4 | **T** | `int_1099`/`div_1099`/`g_1099` box 4 |
| 25c | 8959 L24 + other withholding | **C**+**I** | `Payments.other_withholding` is **I** |
| 25d | add | **C** | |
| 26 | estimated tax payments | **I** | `Payments.estimated_tax_payments` |
| 27 | **EIC** | **X** | *"btctax computes no EIC"* |
| 28 | **additional CTC (8812)** | **X** | *"btctax emits no Schedule 8812"* |
| 29 | **American opportunity credit (8863)** | **X** | *"no education expenses collected"* |
| 30 | reserved for future use | **A** | |
| 31 | Schedule 3 line 15 | **I**+**C** | extension payment (**I**) + §6413(c) excess SS (**C**) |
| 32–33 | totals | **C** | lines 27–30 blank |
| 34 | overpayment refunded | **C** | |
| 35a box | Form 8888 attached | **X** | |
| 35b/35c/35d | **direct-deposit routing / type / account** | **X** | *"btctax collects no bank details"*; refund arrives as a paper check (`Advisory::RefundByPaperCheck`) |
| 36 | apply overpayment to next year | **X** | no such election |
| 37 | amount you owe | **C** | |
| 38 | **estimated tax penalty (§6654)** | **X** | *"btctax emits no Form 2210"* |
| DA question | digital-asset yes/no | **L** | never answered "No" |

**1040 header/trailer, non-numbered:** filing status ✅**I**; names/SSNs/address/occupations/IP PIN
✅**I**; presidential fund ✅**I**; claimed-as-dependent boxes ✅**I**; aged/blind boxes ✅**I** (via
DOB + blind skippables); dependents rows 1–4 ✅**I**; more-than-4 box **C**.
**X**: fiscal-year row, **foreign country/province/postal code** (*"a filer abroad cannot have their
address printed"* — `FIELD_PROVENANCE.md` §6a), §6013(g)/(h) NRA-spouse election, **spouse's IP
PIN** (*"ReturnInputs captures the taxpayer's but not the spouse's"*), **filer phone**, **filer
email**, third-party designee block (5 fields), line-7 "Schedule D not required" box.
**A**: the whole paid-preparer block (7 fields).

### B.2 Schedule 1 (Additional Income and Adjustments)

`printed.rs:466-486` — **ten** printed lines out of 69 fields:

| printed | what | prov. |
|---|---|---|
| 1 | taxable state/local refund | **T** (`sch1.state_refund_taxable`) |
| 3 | business income (Schedule C net) | **L** (crypto) + **T** (`schedule_c.*`) |
| 7 | unemployment compensation | **T** (`g_1099[].box1`) |
| 8v | digital assets received as ordinary income | **L** |
| 9, 10 | totals | **C** |
| 15 | §164(f) half of SE tax | **C** |
| 18 | early-withdrawal penalty | **T** (`int_1099[].box2`) |
| 21 | §221 student-loan interest | **T** (`sch1.student_loan_interest_paid`) |
| 26 | total adjustments | **C** |

**57 censused `unmodeled` fields (M4).** Named in the census, all **X**: 2a/2b **alimony received**,
4 Form 4797, **5 rental real estate, royalties, partnerships, S corps, trusts (Schedule E)**, 6
Schedule F, 8a NOL, **8b gambling**, 8c cancellation of debt (1099-C), 8d/24j Form 2555, 8e Archer
MSA, **8f HSA (Form 8889)**, 8g Alaska PFD, 8h jury duty, 8i prizes, 8j hobby income, **8k stock
options**, 8l personal-property rental, 8m Olympic medals, 8n/8o subpart F / GILTI, 8p §461(l), 8q
ABLE, 8r scholarships, 8s Medicaid waiver, 8t NQDC, 8u incarcerated wages, 8z write-in, **11
educator expenses**, 12 Form 2106, **13 HSA deduction**, 14 Form 3903, **16 SEP/SIMPLE/qualified
plans**, **17 self-employed health insurance**, 19a-c **alimony paid**, **20 IRA deduction**, 23
Archer MSA, 24a–24k, 24z, 25.

### B.3 Schedule 2 (Additional Taxes)

Printed: 2, 3, 4, 11, 12, 21 (`printed.rs:369-396`). **52 censused `unmodeled`** — includes **1a
excess advance premium tax credit (Form 8962 / 1095-A)**, 5 Form 4137 unreported tips, 6 Form 8919,
**8 Form 5329 (IRA/HSA additional tax)**, **9 Schedule H household employment**, 10 first-time
homebuyer repayment, 13 W-2 box-12 A/B/M/N (fail-closed), 16 low-income-housing recapture, 17a–17z.

### B.4 Schedule 3 (Credits and Payments)

Printed: 1 (FTC), 8, 10 (extension payment), 11 (excess SS), 15. **31 `unmodeled` + 1 `artifact`.**
Every one of these is a credit a typical filer might have, and every one is **X**:
**2 child & dependent care (2441)**, **3 education credits (8863)**, **4 saver's credit (8880)**,
**5a/5b residential clean energy + energy-efficient home improvement (5695)**, 6a general business,
**6b prior-year minimum tax (8801)** — *"the one line on this form adjacent to modelled behaviour"*,
6c adoption, 6d elderly/disabled (Sch R), **6f/6m clean vehicle (8936)**, **6g mortgage interest
credit (8396)**, 6h DC homebuyer, 6i/6j/6k, 6l Form 8978, 6z write-in, 7, **9 net premium tax
credit (8962)**, 12 fuels, 13a–13z.

### B.5 Schedule A (Itemized Deductions)

Printed: 1, 2, 3, 4, 5a, 5b, 5c, 5d, 5e, 7, 8a, 8e, 9, 10, 11, 12, 13, 14, 17 + the 5a-is-sales-tax,
line-8 mixed-use and line-18 elects-smaller boxes. **12 `unmodeled` + 1 `artifact`:**

| line | what | prov. |
|---|---|---|
| 6 | "Other taxes" write-in (3 fields) | **X** |
| 8b | **home mortgage interest NOT reported on Form 1098** (2 fields) | **X** — *"btctax collects `mortgage_interest_1098` only"* |
| 8c | **points not reported on Form 1098** (2 fields) | **X** |
| 8d | reserved (the pre-2022 PMI line) | **A** |
| 15 | **casualty and theft losses (Form 4684)** | **X** |
| 16 | "Other" write-in (4 fields) | **X** — this is where **gambling losses** would go |

### B.6 Schedule B / C / D / SE and the attached forms

| form | mapped FQNs | censused | `unmodeled` | `artifact` | notes |
|---|---|---|---|---|---|
| f1040 | 87 | 54 | 46 | 8 | |
| f1040s1 | 12 | 57 | 56 | 1 | |
| f1040s2 | 8 | 52 | 52 | 0 | |
| f1040s3 | 7 | 32 | 31 | 1 | |
| f1040sa | 24 | 13 | 12 | 1 | |
| f1040sb | 70 | 2 | 2 | 0 | interest/dividend rows are **T** |
| f1040sc | 17 | 88 | **88** | 0 | only A, B, F, I, J, 1, 3, 5, 7, 28, 29, 31 are mapped — Part II (lines 8–27), Part III COGS, Part IV vehicle and Part V other-expenses are **all X**; `ScheduleCInputs.expenses` is one flat number |
| f1040v | 12 | 3 | 3 | 0 | |
| f4868 | 12 | 5 | 5 | 0 | |
| f6251 | 43 | 18 | 18 | 0 | |
| f8275 | 59 | 36 | 24 | 12 | |
| f8283 | 64 | 53 | 49 | 4 | donee/appraiser declarations are `artifact` |
| f8949 | 238 | 6 | 4 | 2 | |
| f8959 | 19 | 7 | 7 | 0 | |
| f8960 | 17 | 21 | 21 | 0 | lines 3 (pensions), 4a (rents/royalties/Sch E) are X |
| f8995 | 21 | 12 | 12 | 0 | |
| f8995a | 46 | 65 | 65 | 0 | |
| schedule_d | 49 | 6 | 6 | 0 | 4 (6252/4684/6781/8824), 5 & 12 (K-1), 11 (4797/2439) are X |
| schedule_se | 14 | 13 | 11 | 2 | |
| **TY2024 TOTAL** | **819** | **543** | **512** | **31** | 1,362 fields, 0 unaccounted (M2, M4) |

TY2025 (M4): 595 mapped, 250 censused, and **five maps carry no `[census]` at all** — the
`UNCENSUSED` register in `field_census.rs:80-92` pins them at **5 entries / 310 fields**
(`f1040` 196, `f8283` 63, `f8949` 12, `schedule_d` 24, `schedule_se` 15). TY2026 has **no maps** and
`forms_expected = []`.

★ The 496-unaccounted figure in `design/forms/FIELD_PROVENANCE.md` §6 (TY2024, 15 forms, 1,158
fields) has been **fully burned down** for TY2024: 19 forms, 1,362 fields, 0 unaccounted (M2/M4).
The register's remaining debt is entirely TY2025.

### B.7 The explicit "cannot take at all" checklist (the brief's list)

| a typical individual return needs | btctax today |
|---|---|
| **1099-R / pensions / annuities** | **cannot take** — 1040 4a/4b, 5a/5b are `unmodeled`; no struct exists |
| **Social Security (SSA-1099)** | **cannot take** — 1040 6a/6b `unmodeled`; no §86 worksheet |
| **1099-NEC / 1099-MISC** | **cannot take** — no struct. Named as the blocker for Schedule 1-A lines 5 and 14b (`attribute.rs:261-266`: *"needs a 1099-NEC / 1099-MISC / 1099-K input surface, which btctax does not have"*) |
| **Schedule K-1** | **cannot take** — Sch 1 L5, Sch D L5/L12, Sch 3 L6l all `unmodeled` |
| **Schedule E (rental real estate)** | **cannot take** — Sch 1 L5 `unmodeled`; Form 8960 L4a `unmodeled`; no form template bundled |
| **sale of a home / §121 exclusion** | **cannot take** — no Form 4797/8949-real-property path; Sch 2 17b (mortgage-subsidy recapture) says *"btctax models no home sale"* |
| **HSA (Form 8889) / IRA contributions** | **cannot take a figure.** There IS a *declaration* (`DeclHsaActivity`, `QuestionId::HsaActivity`) and an `sch1.ira_deduction_claimed` money field, but both are **refusals**: `HsaActivityUnsupported`, `IraDeductionClaimed` |
| **education credits (8863) / AOTC** | **cannot take** — Sch 3 L3, 1040 L29 `unmodeled` |
| **child tax credit / ODC** | **cannot take** — 1040 L19 never populated; the row checkboxes never checked; Sch 8812 not emitted; 1040 L28 `unmodeled` |
| **estimated payments** | ✅ **I** — `Payments.estimated_tax_payments` → 1040 L26 |
| **state tax refund (1099-G box 2)** | **T only** — `sch1.state_refund_taxable`, a bare number with no worksheet |
| **unemployment (1099-G box 1)** | **T only** — `g_1099[]` |
| **alimony (paid or received)** | **cannot take** — Sch 1 2a/2b/19a-c `unmodeled` |
| **student-loan interest** | **T only** — `sch1.student_loan_interest_paid`; a bare number, phase-out computed |
| **Schedule A real-estate tax** | ✅ **I** — `SaSaltRealEstate` → Sch A 5b |
| **Schedule A mortgage interest** | ✅ **I** *only if on a Form 1098* — `SaMortgage1098` → 8a. **8b (no-1098 interest) and 8c (points) are X** |
| **casualty loss** | **cannot take** — Sch A L15 `unmodeled` |
| **gambling** | **cannot take** — income Sch 1 8b `unmodeled`; losses Sch A L16 write-in `unmodeled` |
| **foreign tax credit** | **T only** — computed from `int_1099.box6` / `div_1099.box7`; above the §904(j) de-minimis ceiling the return **refuses** (`ForeignTaxOverCeiling`) |
| **EV / energy credits** | **cannot take** — Sch 3 5a/5b/6f/6m `unmodeled` |
| **Form 8949 for NON-crypto securities** | **T only, and it refuses.** `b_1099: Vec<Form1099B>` exists with 6 fields, but `basis_reported_and_no_adjustments` must be `Some(true)` or the return refuses `Form1099BNeedsForm8949` (`attribute.rs:253`: *"Form 1099-B rows are entered via TOML import"*). So only the "basis reported, no adjustments" case is fileable |
| **1099-B broker input generally** | **T only** — aggregate ST/LT proceeds+basis, no per-lot rows |

---

## C. Dependents

### C.1 What the header collects — four fields, nothing else

`crates/btctax-core/src/tax/return_inputs.rs:224-229` (M6):

```rust
pub struct Dependent {
    pub name: String,
    pub ssn: String,
    pub relationship: String,       // free text
    pub date_of_birth: Option<Date>,
}
```

All four are TUI form Fields (`DEPENDENT_FIELDS`, M5) and repeat per row. Rows beyond 4 go to an
overflow statement and check the "more than four dependents" box
(`form1040_full.rs:533-534`).

### C.2 What it does NOT ask

Nothing in the codebase asks any of the §152 tests. Measured: `grep -ri "qualifying child\|qualifying
relative\|months.*residen\|gross income test\|support test"` over `crates/btctax-core/src/tax/`
returns nothing that collects an answer.

| missing answer | statutory home | what it gates |
|---|---|---|
| qualifying child vs qualifying relative | §152(c)/(d) | which credit column, if any |
| **months lived with the taxpayer** | §152(c)(1)(B) | CTC eligibility, HoH status |
| **who provided over half the support** | §152(c)(1)(D), §152(d)(1)(C) | dependency at all |
| citizenship / residency | §152(b)(3) | dependency |
| **CTC vs ODC** — the two per-row checkboxes | §24(h)(2), §24(h)(4) | 1040 L19 and the row boxes |
| dependent's own filing status | §152(b)(2) | dependency |
| whether the child has an SSN valid for employment | §24(h)(7) | CTC vs ODC |

### C.3 Which lines the missing answers block

- **1040 line 19** — CTC/ODC. `Option<Usd>`, never `Some` from btctax. `printed.rs:589-596`
  records that a hardcoded `Usd::ZERO` here *"overstated their tax by up to $2,000 a child"*.
- **1040 dependents-table CTC/ODC checkboxes** (8 mapped fields across 4 rows) — mapped, and
  `form1040_full.rs:539` deliberately never checks them.
- **1040 line 28** — additional child tax credit. `unmodeled`.
- **1040 line 27** — EIC. `unmodeled` (`Advisory::EicOmitted`).
- **Schedule 3 line 2** — child and dependent care credit. `unmodeled`.
- **Head-of-household filing status** is *offered* (`FilingStatusArg::Hoh`) with **no test at all**
  behind it — nothing asks whether the filer maintained a household for a qualifying person.

★ The dependent surface is therefore **complete for identification and empty for entitlement**:
btctax can print who they are, and can grant no credit that depends on them.

---

## D. Real estate

### D.1 What exists

| item | field | line |
|---|---|---|
| real-estate taxes paid | `ScheduleAInputs.salt_real_estate` → `SaSaltRealEstate` (**I**) | Sch A 5b |
| personal-property tax | `salt_personal_property` → `SaSaltPersonalProp` (**I**) | Sch A 5c |
| home mortgage interest **on a Form 1098** | `mortgage_interest_1098` → `SaMortgage1098` (**I**) | Sch A 8a |
| the §163(h)(3) mixed-use declaration | `mortgage_all_used_to_buy_build_improve` → `SaMortgageAllUsed` (**I**) | Sch A line-8 box |
| the §163(h)(3)(B) acquisition-debt-ceiling declaration | `mortgage_within_debt_limit` → `SaMortgageWithinDebtLimit` (**I**) | refuses `MortgageOverDebtLimit` |
| the AMT-qualified-dwelling declaration | `mortgage_dwelling_is_amt_qualified` → `DeclAmtQualifiedDwelling` (**I**) | Form 6251 line 3 |

### D.2 What does not exist

| missing | evidence |
|---|---|
| **Schedule E** — rental real estate, royalties, partnerships, S corps, trusts | no template in any `forms/<year>/`; Sch 1 L5 `unmodeled`; Form 8960 L4a `unmodeled` |
| **Form 1098 import** | no struct. `mortgage_interest_1098` is one hand-typed number; no lender, no address, no points box, no outstanding-principal box (§163(h)(3)(B) is answered by a *declaration*, not by a figure) |
| **mortgage interest not on a 1098** (Sch A 8b) | `unmodeled` — *"btctax collects `mortgage_interest_1098` only, i.e. interest that IS on a 1098"* |
| **points** (Sch A 8c) | `unmodeled` — *"Points not reported to you on Form 1098 — description. Not collected."* |
| **PMI / mortgage insurance premiums** | Sch A 8d is `artifact` — the IRS reserved the line for TY2024, so there is nothing to collect *for that year*; a TY2026 revision reinstating it would need a new field |
| **sale of a principal residence / §121 exclusion** | nothing. No 1099-S struct, no Form 4797, no §121 worksheet. Sch 2 17b's census says *"btctax models no home sale"* |
| **mortgage interest credit (Form 8396)** | Sch 3 6g `unmodeled` |
| **first-time homebuyer credit repayment (Form 5405)** | Sch 2 L10 `unmodeled` |
| **residential energy credits (Form 5695)** | Sch 3 5a/5b `unmodeled` |
| **property depreciation** | only a flat `ScheduleCInputs.expenses`; Form 6251 line 2l is answered by a *declaration* (`DeclAmtDepreciationSame`), not a schedule |

---

## E. The exchange side

### E.1 Per adapter

Four `Source` variants (`crates/btctax-core/src/identity.rs:8-13`). Wallets are named
`exchange:<provider>:default` — `crates/btctax-adapters/src/normalize.rs:64-69`, and the account
segment is **hardcoded to `"default"`**, so one provider = one wallet regardless of how many accounts
the filer holds there.

| adapter | file | format | detection | what it yields |
|---|---|---|---|---|
| **Coinbase** | `sources/coinbase.rs` | yearly CSV, 3-line preamble | header tokens `ID` + `Transaction Type` + `Quantity Transacted` | `Buy`→`Acquire` (basis = Subtotal + Fees); `Sell`→`Dispose`; `Send`/`Withdrawal`→`TransferOut` (dest = Recipient Address); `Receive`→`TransferIn` (src = Sender Address); `Order`, Coinbase↔Pro transfers **and every unknown/future type** → `Unclassified` |
| **Gemini** | `sources/gemini.rs` | XLSX ledger, Excel serial dates | `Trade ID`/`Order ID` present | trades→`Acquire`/`Dispose`; `Credit`(BTC)→`TransferIn`; `Debit`(BTC)→`TransferOut`; `Tx Hash` is the txid match signal; **`Specification` containing "credit card reward" → `Income`** (matched on the phrase, not the full string). ★ documented caveat: `TransferIn.src_addr` for a Gemini Credit holds Gemini's *deposit* address, not the sender |
| **River** | `sources/river.rs` | universal CSV, CRLF, **no ids** (semantic `source_ref`) | `Sent Amount` + `Received Amount` + `Tag` | `Buy`→`Acquire`; **`Income`→`Income{Reward}`**; **`Interest`→`Income{Interest}`** (no USD on the row ⇒ dataset FMV); `Withdrawal`→`TransferOut`; unknown `Tag`→`Unclassified` |
| **Swan** | `sources/swan.rs` | **three CSVs = one batch** (trades / transfers / withdrawals), routed by header signature | per-role signature | trades→`Acquire`; transfers `purchase`→`Acquire`, `deposit`→`TransferIn`, fees→`Unclassified`; withdrawals→`TransferOut`. ★ **FOUND GAP recorded in the module doc**: a transfers row carries `USD Cost Basis` **and** `Acquisition Date` and `TransferIn` has no field for either — *"They are dropped at ingest and must be re-supplied by reconciliation"* |

None of the four carries an on-chain txid column except Gemini's `Tx Hash`; **no self-custody
wallet is importable at all** — self-custody appears only as the counterparty of an exchange
transfer, and must be reconciled by hand.

### E.2 What the filer must still declare by hand

`btctax reconcile` has **27** subcommands (M7). The declarations they exist to collect:

| declaration | command | why software cannot answer it |
|---|---|---|
| this outbound and that inbound are the **same coins moving between my own wallets** | `link-transfer`, `bulk-link-transfer`, `match-self-transfers` | two exchanges, two ledgers, no shared identifier |
| an inbound is **income / a gift received / my own transfer** | `classify-inbound-income`, `classify-inbound-gift`, `classify-inbound-self-transfer` (+ 2 bulk) | `InboundClass` has 3 variants; the adapter can only see "coins arrived" |
| an outflow is a **sale / spend / gift out / donation** | `reclassify-outflow`, `bulk-reclassify-outflow` | `OutflowClass` × `DisposeKind` = 4 outcomes, all tax-different |
| FMV when the export carries none | `set-fmv` | |
| which lots a disposal consumed (specific identification) | `select-lots`, `import-selections`, `optimize accept` | §1.1012-1(j) |
| the **standing order** under Notice 2026-20 §4.02(2) | `btctax config --set-forward-method hifo --exchange … --effective-from …` | **owner action T7, dated: before the next 2026 sale on a custodial venue** |
| donation details (donee, appraiser, restrictions) | `set-donation-details` + the `DonationsHadRestrictions` / `CharitableCwaObtained` skippables | Form 8283 §170(f)(8)/(11) |
| what an `Unclassified` raw row actually was | `classify-raw`, `void`, `accept-conflict` / `reject-conflict` | |
| **the Form 1099-DA answers** | `BrokerReporting` section (rows seeded from the ledger) or `[broker_reporting.<provider>]` in the TOML | requires the filer to hold the broker's paper and compare box 1g against btctax's column (e) |

**1099-DA answer shape:** `BrokerReported` has 5 variants (`forms.rs:272-287`) —
`NotReported`, `ProceedsOnly`, `BasisMatches` route to Form 8949 boxes I/L, H/K, G/J respectively;
`BasisDiffers` and `Mixed` **refuse**. Two slots per provider (`covered` / `noncovered`), one row per
provider the year's 8949 rows carry. An answer for a key no row reads is refused as *unread*.

### E.3 Question-type cardinalities (ledger vs return)

| enum | n | what it covers |
|---|---|---|
| `BlockerKind` (`state.rs:23`) | **23** | the **ledger** side — `FmvMissing`, `UncoveredDisposal`, `Unclassified`, `UnmatchedOutflows`, `LotSelectionInvalid`, `MethodElectionBackdated`, `IdentificationDefaulted`, … |
| `RefuseReason` (`return_refuse.rs:36`) | **71** | the **return** side (both `screen_inputs` and `screen_absolute`) |
| `Advisory` (`advisories.rs:43`) | **21** | forgone-benefit disclosures |
| `EventPayload` (`event.rs`) | **22** | 6 imported + 16 decision/reconciliation events |

Of the 71 `RefuseReason`s, `attribute.rs` maps them to form locations over **63 match arms** with
**17 `Anchor::NotInForm`** anchors (M7) — i.e. **17 refusals the interactive form cannot let a filer
answer.** Their notes name the gap precisely, and they are the interview's to-do list:
`PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`, `InconsistentDividendSubset`,
`ForeignTaxOverCeiling` (the 1099-INT/DIV boxes), `IraDeductionClaimed`, `ScheduleCLoss`,
`ScheduleCNoBusinessDescription`, `BusinessIncomeWithoutScheduleC`, `BusinessInterestIncome`
(Schedule C), `Form1099BNeedsForm8949` (1099-B), `Schedule1aTipsFromTradeOrBusiness` /
`Schedule1aOvertimeFromTradeOrBusiness` (**the 1099-NEC/MISC/K gap**), `NonPublicCharityContribution`
(carryover-in), `SpouseOwnerWithoutJointReturn` (Schedule C owner), `KiddieTax`,
`AmtScreenTriggered`, `NegativeAmount`.

---

## F. Rules the interview must obey (quoted, with cites)

1. **Never default an answer.**
   `design/SPEC_input_surface.md` §5 D-2: *"★ **My completeness machinery structurally revoked the
   codebase's fail-loud guarantee, for exactly the fields it was built for. Visibility with a
   pre-filled answer is a guess wearing documentation's clothes.**"*
   And the membership criterion it states: *"A field may be exempt from KAT C's completeness check
   ONLY IF it satisfies one of: (a) its absence FAILS LOUD; or (b) its absence is CONSERVATIVE *and*
   ADVISED; or (c) it is a SECRET (D-6)."*
   Enforced in code by `answer.rs:169-171`: *"★ No default and no answer ⇒ ASK AGAIN. Accepting
   silence here would reintroduce D-8 through the front door."*

2. **Blank by inputs vs blank by omission — only the second is a bug.**
   `CLAUDE.md`: *"★★ But "present" is not "populated" — most fields on a tax return are blank,
   intentionally. … The real invariant is that **every line has a determinate PROVENANCE**"*, with
   the table `the inputs say so ⇒ correct` vs `nothing ever populated it ⇒ the defect`.
   And: *"The standing example is an **ISO exercise printed as $0** on Form 6251 line 2i — the
   dominant real AMT trigger post-TCJA, laundered as a blank."*

3. **An entry is testimony; a recorded "no" is provenance for us, never testimony on the return.**
   `design/forms/FIELD_PROVENANCE.md` §3: *"Answering "no" to an interview question **does not put
   testimony on the form.** The line stays **blank**, and we must never print `0` to show that the
   filer answered. The answer lives in our records; it makes the omission *informed and auditable*;
   the form says nothing, because nothing is what the filer is entitled to say."*
   ★ And its blocker: *"**§G-11 blocks the honest version of this**: `fmt_money(Usd) -> String`
   cannot express blank, so even a correctly-recorded "no" renders as `0`."*

4. **Enumerate the line set FROM the form, never a range or a hand-list.**
   `CLAUDE.md`: *"a conformance KAT must (a) enumerate the expected line set **from the form's
   extracted text**, never from a range or a hand-written list, and (b) require every line to be
   *accounted for*"*. The mechanism that does it: `field_census.rs:170-190` reads FQNs by text scan
   — *"Text-scanned rather than deserialized on purpose: the census must see **every** FQN the file
   names, including any a future typed struct forgets to model."*

5. **Transcribe, don't paraphrase.**
   `CLAUDE.md`: *"When implementing or reviewing an IRS form, schedule, **or worksheet**: one field
   per numbered line, named for the line, in the form's own numbering, carrying the official
   instruction text verbatim as its doc comment."* And: *"**If the form asks something our input
   surface cannot answer, collect it.** That is following instructions, not scope creep."*

6. **The `income import` TOML is the wire; the Rust structs are the schema.**
   `SPEC_input_surface.md` §4: *"**The Rust structs already ARE the schema, and the money-bearing
   ones are compiler-audited.** … An external schema would be a **third representation** to keep in
   sync."* ⚠️ with the recorded exception: *"But NOT the header. The destructure wildcards it …
   so `HouseholdHeader` / `Person` / `Dependent` get **no compiler forcing**."*
   The seam types are serde-derived and explicitly *"the web wire"* (`seam.rs:169`), so a browser
   interview drops in without new plumbing.

7. **A stored-but-refused row poisons the whole year.**
   `SPEC_input_surface.md` §3.2: *"**So a refused row does not merely fail to help. It takes the year
   down** — the crypto-only report and the hand-entered profile both stop working. … **Storing what
   the engine has already judged unusable is not a kindness. It is a trap.**"* Any interview must
   therefore keep a **draft** separate from a **commit** — which the existing form already does
   (`input_form_store.rs`: draft table, `save_draft` anytime, `commit` fail-closed).

8. **Answered-ness must be structural, not conventional.**
   `classifier.rs:1-32`: *"Every struct is destructured with **NO `..`**, so a newly-added field is a
   `pattern does not mention field` COMPILE ERROR until a human edits this file."* And the honest
   limit: *"The compiler forces "a human must EDIT the classifier", NOT "classified it correctly.""*

9. **Secret handling never blocks a gate.**
   `/scratch/code/CLAUDE.md`: *"A failure to handle secret material secretly … is **never Critical
   and never Important**. Log it as a follow-up when discovered, for future optimization, and let the
   gate close."* The existing surface already treats secrets structurally anyway
   (`SecretView::set_masked` rejects a "mask" carrying a 5+ digit run — `seam.rs:194-203`).

10. **No checker exists until it has been seen RED (B1).**
    `CLAUDE.md`: *"Every new census, conformance check, citation check, lint, or review harness lands
    **paired** with a negative test that plants the exact defect it exists to catch and asserts
    red."* The field census already complies —
    `field_census.rs::the_gate_reds_on_every_planted_defect` (M2).

---

## G. Gap summary

### G.1 Counts

**Form 1040 (TY2024): 59 numbered/lettered entry lines plus the Digital-Asset question = 60**,
each assigned its most-upstream non-computed source from the §B.1 table (a pure sum is **C**):

| class | n | share | lines |
|---|---|---|---|
| **I** — an interview field is the source | **6** | 10% | 1a, 12, 25a, 25c, 26, 31 |
| **T** — TOML import only | **7** | 12% | 2a, 2b, 3a, 3b, 10, 20, 25b |
| **L** — ledger | **2** | 3% | 7, DA question |
| **T + L mixed** | **1** | 2% | 8 (Sch 1 L10: state refund **T**, unemployment **T**, Sch C **T**+**L**, 8v **L**) |
| **C** — computed from other printed lines | **19** | 32% | 1z, 9, 11, 13, 14, 15, 16, 17, 18, 21, 22, 23, 24, 25d, 32, 33, 34, 35a, 37 |
| **never populated** | **1** | 2% | **19** (CTC/ODC) |
| **X** — cannot be taken at all | **23** | 38% | 1b–1i (8), 4a, 4b, 5a, 5b, 6a, 6b, 6c (7), 27, 28, 29 (3), 35b, 35c, 35d (3), 36, 38 |
| **A** — artifact | **1** | 2% | 30 (reserved) |

★ The **C** column is not "free": lines 12→16→18→22→24 sit on the interview's Schedule A and the
standard-deduction table, and 23 sits on the ledger's SE tax. What the table shows is that the
interview is the *sole* source for only six lines, while 24 of 60 (X + never-populated) are
unreachable from any surface btctax has.

At **field** granularity across the whole TY2024 packet (M4, `tomllib` over the `[census]` tables):
**819 mapped + 543 censused = 1,362**, of which **512 `unmodeled` and 31 `artifact`** — i.e.
**40% of every box on every form btctax bundles for TY2024 carries a recorded decision NOT to fill
it.** That share is honest, not alarming — much of it is preparer blocks, write-in lines and
out-of-scope forms — but §B.7 is the part that is not.

**Struct-level:** of `ReturnInputs`'s 40 top-level fields, the form covers 98 leaves across 16
sections and the coverage KAT lists the rest as EXEMPT. The **import-only** surface is:
`int_1099` (8 fields/row), `div_1099` (12), `g_1099` (3), `b_1099` (6), `schedule_c` (6 of 12 leaves),
`sch1` (3 of 4), `schedule_1a` (**3 + 5 + 4 + 11 = 23 leaves**, all of OBBBA's tips / overtime /
car-loan / senior deductions), the two capital-loss and charitable carryover vectors, and 4
provenance leaves.

**Interview reach per surface:** 98 form Fields · 36 registry questions · 17 refusals with **no**
in-form remedy.

### G.2 The five biggest gaps for a typical W-2-plus-crypto filer, ranked by 1040 lines unlocked

| # | gap | 1040 lines it unlocks | why it ranks here |
|---|---|---|---|
| **1** | **No interview for any year but TY2024.** `commit` → `NoTables` on every year without `FullReturnParams` (M3), and only TY2024 has them | **all of them, for TY2026** | The target filing year cannot be authored interactively at all. Everything else in this list is moot until this is answered — a filer's only path to a TY2026 return today is hand-writing a TOML with no template and no import-time screen |
| **2** | **Dependents collect identity but no entitlement** (§C) | 19, 27, 28, the 8 CTC/ODC row boxes, Sch 3 L2 — plus HoH filing status, which has no test behind it | A married filer with two children is overtaxed by up to $4,000 and the return prints a blank the IRS reads as "no credit claimed". `printed.rs:589-596` records the exact shape of the earlier version of this bug |
| **3** | **Retirement and Social Security income is unrepresentable** (1099-R, SSA-1099) | 4a, 4b, 5a, 5b, 6a, 6b, 6c — **7 lines**, plus Sch 1 L20 and Sch 2 L8 | This is the single largest contiguous block of `unmodeled` income on page 1, and it is in the **understatement** direction. Any filer over ~59½, or with any IRA distribution, cannot file |
| **4** | **The information returns are TOML-only** — 1099-INT, 1099-DIV, 1099-G, 1099-B (8+12+3+6 = 29 fields across 4 structs, **zero** form Fields) | 2a, 2b, 3a, 3b, **7**, 8 (via Sch 1 L7), 10 (via Sch 1 L18), 20, 21, 25b — **10 lines** | A W-2-plus-crypto filer with a savings account and a brokerage cannot use the interactive form at all; they must hand-edit TOML for their interest and dividends. Six of the 17 `NotInForm` refusals are in this cluster |
| **5** | **No 1099-NEC / 1099-MISC / 1099-K surface, and Schedule C is one flat expense number** | Sch 1 L3 → 1040 L8; Sch 2 L4; Sch 1 L15 → 1040 L10; Sch 3 L6b; **and all of Schedule 1-A Parts II/III** (tips + overtime, TY2025+) | `attribute.rs:261-266` names this by hand: *"needs a 1099-NEC / 1099-MISC / 1099-K input surface, which btctax does not have."* Schedule C maps 17 of its 105 fields (12 numbered/lettered lines); Part II lines 8–27 do not exist as inputs |

**Runners-up, in the same direction:** HSA (declaration exists, refuses), IRA deduction (field
exists, refuses), Schedule E (nothing), §121 home sale (nothing), education credits (nothing),
Schedule A 8b/8c/15/16 (no-1098 mortgage interest, points, casualty, gambling losses), direct-deposit
refund details (paper check only), foreign address (a filer abroad cannot print their address).

### G.3 One structural observation for the brainstorm

The repo has already written down the right inversion and has not built it.
`design/forms/FIELD_PROVENANCE.md` §4 is titled **"Invert it: the FORMS derive the INTERVIEW"**, and
§5 names the two things that block it:

> *"**But `return_refuse.rs::screen_inputs` returns `Option<Refusal>` — the FIRST unanswered live
> declaration only.** At ~21 questions that is tolerable; at section-level questions across 16 forms
> it becomes *answer one, re-run, hit the next* — **N unanswered ⇒ N round-trips**, and the filer
> never sees how much is left."*

and

> *"★★ **It is one surface, readable from either end.** Each unanswered question names the fields it
> would account for; each unaccounted field names the question that would explain it."*

Both halves already exist in fragments: `attribute.rs` maps refusal → form location (63 arms), and
the `[census]` sections map field → the decision not to fill it (543 records for TY2024). What does
not exist is the join, and the four-state visibility model §5 specifies (not live / answered / live+
unanswered class A **blocks** / live+unanswered class B **forgoes money, lawfully**).

---

## Appendix — files a spec author will need

| file | why |
|---|---|
| `crates/btctax-input-form/src/seam.rs` | `SectionId` (16), `FieldId` (98), `FieldKind`, `Edit`, `Anchor` — serde-derived, explicitly the web wire |
| `crates/btctax-input-form/src/spec/sections.rs` (1,756 lines) | the 13 concrete sections |
| `crates/btctax-input-form/src/spec/registries.rs` (559) | the 2 synthetic registry-driven sections + the FieldId↔QuestionId maps |
| `crates/btctax-input-form/src/spec/coverage.rs` (731) | the drift-proof coverage KAT; `EXEMPT_PREFIXES` / `EXEMPT_LEAVES` name what is import-only |
| `crates/btctax-input-form/src/attribute.rs` | refusal → form location; the 17 `NotInForm` notes are the gap list |
| `crates/btctax-core/src/tax/return_inputs.rs` (1,473) | `ReturnInputs` and its 21 nested structs |
| `crates/btctax-core/src/tax/questions.rs` (1,879) | `FORM_QUESTIONS` (17) + `SKIPPABLE_QUESTIONS` (19), each with prompt, liveness, durability, refusal |
| `crates/btctax-core/src/tax/classifier.rs` (1,000) | the compile-forcing answered-ness census |
| `crates/btctax-core/src/tax/return_refuse.rs` (3,772) | 71 `RefuseReason`s — every one is a question the interview must be able to answer |
| `crates/btctax-core/src/tax/printed.rs` (3,387) | the printed line chains, with each line's source in its doc comment |
| `crates/btctax-forms/forms/2024/*.map.toml` | 819 mapped + 543 censused FQNs — the authoritative "what we fill / what we deliberately do not" |
| `crates/btctax-forms/tests/field_census.rs` | the gate, its `UNCENSUSED` register (5 entries / 310 fields, TY2025) and its planted-defect kill |
| `crates/btctax-cli/src/input_form_store.rs` | draft vs commit, the TY2024-only gate, the park/coherence rules |
| `design/forms/FIELD_PROVENANCE.md` | the provenance taxonomy, the "forms derive the interview" inversion, the deletable-decline-record design |
| `design/SPEC_input_form.md` §5.8 | the v1 field inventory and its explicit exemptions |
| `design/SPEC_input_surface.md` §3, §5 D-2 | ABSENT≠PARTIAL, the poisoned-year rule, the ASK-THE-USER class and its membership criterion |
| `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` | the only committed worked `income import` file (170 lines, generated, oracle-pinned) — the de facto schema example |
| `crates/btctax-adapters/src/tax_tables.rs:96-108`, `:1028`, `:1146` | why only TY2024 can commit a return today |
