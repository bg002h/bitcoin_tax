# Deep Recon 05 — Open-Source US-Federal Income-Tax Engines: License-Verified Borrow Catalog

**Question:** Of the open-source US individual income-tax engines / libraries / datasets in the
wild, which can `btctax` (**MIT OR Unlicense**) actually *borrow* code or data from — versus
merely *study*?

**Method:** Every license below was verified at write time by reading the project's **actual
`LICENSE` file or GitHub license metadata via `gh api`** (not from memory). Where GitHub's
auto-detector returned `NOASSERTION` / "Other", the raw `LICENSE` text was decoded and read. Last-
activity timestamps are from the repo `pushed_at` field. Verified **2026-07-11**.

**License frame (from the task):**
- **BORROWABLE** = permissive / public-domain: MIT, BSD-2/3, Apache-2.0, ISC, Unlicense, CC0, or
  US-Government public-domain (17 USC §105). Incorporate code/data with attribution.
- **STUDY-ONLY** = copyleft: GPL-2.0/3.0, **AGPL-3.0**, LGPL, MPL. Architecture/algorithm lessons
  only; **never** copy code, comments, constant tables, or data files into our tree.
- **NON-FREE** = proprietary / no-license (all-rights-reserved by default) / service-restricted.

---

## Bottom line up front

1. **The single best *borrowable* asset in the entire ecosystem is
   [PSLmodels/Tax-Calculator](https://github.com/PSLmodels/Tax-Calculator)'s parameter file
   `taxcalc/policy_current_law.json`, and it is CC0 1.0 / US public domain.** GitHub mis-detects
   it as "NOASSERTION"; the actual `LICENSE` reads *"The Tax-Calculator project is in the public
   domain within the United States. Additionally, we waive copyright and related rights in the work
   worldwide through the CC0 1.0 Universal public domain dedication."* That file is a 635 KB,
   year-indexed, filing-status-aware structured dataset containing **exactly** the parameters we
   hand-transcribe: ordinary brackets (`II_brk1..7`), standard deduction (`STD`, `STD_Aged`),
   LTCG/qualified-dividend breakpoints and rates (`CG_brk1..3`, `CG_rt1..4`), NIIT (`NIIT_thd`,
   `NIIT_rt`), and Additional Medicare Tax (`AMEDT_ec`, `AMEDT_rt`). **This is the parameter-data
   cross-check source** — use it to independently second-source our `tax_tables.rs`.
2. **Every full *engine* worth learning from is copyleft.** OpenTaxSolver (GPL-2.0), HabuTax
   (GPL-2.0), py1040 (GPL-2.0), ustaxlib (GPL-3.0), UsTaxes (AGPL-3.0), filedcom/opentax
   (AGPL-3.0), and the entire PolicyEngine/OpenFisca stack (AGPL-3.0) are all **STUDY-ONLY**. None
   of their code, comments, or data tables may enter our tree.
3. **The prior holds.** Public-domain US-government sources (IRS forms/instructions/Rev. Procs. +
   IRS SOI + IRS ATS fixtures) plus the CC0 Tax-Calculator parameters are our safest and richest
   borrows. **Clean-room re-derivation from IRS instructions stays the primary path for logic.**
4. **No mature Rust US-1040 crate exists.** crates.io / GitHub Rust search turns up only a
   German-focused `income-tax` crate, VAT tools, and one abandoned 0-star 1040-ES calculator. We
   are not reinventing over a mature Rust wheel — there isn't one.
5. **One trap to name explicitly: [tenforty](https://github.com/mmacpherson/tenforty) is MIT on
   the label but GPL-encumbered in substance** — its MIT license covers only the Python/Cython
   glue; it **vendors the GPL-2.0 OpenTaxSolver release tarballs** (`OpenTaxSolver2018..2025_*.tgz`)
   and OTS-derived `otslib/*.cpp`. Do **not** treat its tax-computation code as borrowable. It is,
   however, an excellent **black-box numeric oracle** for verification (running it and comparing
   numbers is not copying).

---

## Master catalog (license-verified)

| Project (repo) | Lang | Scope (forms / years / fed-state) | Last active | License (verified) | Verdict |
|---|---|---|---|---|---|
| [PSLmodels/Tax-Calculator](https://github.com/PSLmodels/Tax-Calculator) | Python | Federal individual income + payroll **microsim**; parameters 1960s→present; not a form-filler | 2026-07-11 (active) | **CC0 1.0 / US public domain** (read from `LICENSE`) | **BORROWABLE** — the crown jewel; see §Data |
| [OpenTaxSolver](https://opentaxsolver.sourceforge.net/) | C | 1040 + Sch 1-3, A-D, 6251, 8949, 8889; several states; per-year | active (yearly) | **GPL-2.0** | **STUDY-ONLY** (already known) |
| [habutax/habutax](https://github.com/habutax/habutax) | Python | 1040 + Sch 1/3 + several forms; field-DAG solver; fills PDFs | 2024-02-25 | **GPL-2.0** | **STUDY-ONLY** (already known) |
| [b-k/py1040](https://github.com/b-k/py1040) | Python | Form 1040 personal calculator (Ben Klemens); 338★ | 2026-04-13 | **GPL-2.0** | **STUDY-ONLY** |
| [rsesek/ustaxlib](https://github.com/rsesek/ustaxlib) | TypeScript | `core` form-DAG framework + `fed2019` forms; well-designed | 2025-01-08 | **GPL-3.0** | **STUDY-ONLY** |
| [ustaxes/UsTaxes](https://github.com/ustaxes/UsTaxes) | TypeScript | **Web/desktop app**; fills Federal 1040 + several states; 1.6k★ | 2026-07-03 (active) | **AGPL-3.0** | **STUDY-ONLY** + it's an *app*, not a library |
| [filedcom/opentax](https://github.com/filedcom/opentax) | TypeScript | Single-binary CLI federal engine, Form 1040 TY2025; "AI-built" | 2026-05-11 | **AGPL-3.0** | **STUDY-ONLY** |
| [PolicyEngine/policyengine-us](https://github.com/PolicyEngine/policyengine-us) | Python | Federal + state tax-benefit microsim (ex-OpenFisca-US) | 2026-07-10 (active) | **AGPL-3.0** | **STUDY-ONLY** |
| [PolicyEngine/policyengine-core](https://github.com/PolicyEngine/policyengine-core) | Python | Microsim engine (fork of OpenFisca-Core) | 2026-07-09 | **AGPL-3.0** | **STUDY-ONLY** |
| [openfisca/openfisca-core](https://github.com/openfisca/openfisca-core) | Python | Upstream microsim framework | 2026-04-22 | **AGPL-3.0** | **STUDY-ONLY** |
| [PolicyEngine/policyengine-taxsim](https://github.com/PolicyEngine/policyengine-taxsim) | Python | TAXSIM-format emulator **wrapping** policyengine-us | 2026-07-08 | **MIT** (wrapper) — but imports AGPL `policyengine-us` | **STUDY-ONLY in practice**; usable as oracle |
| [PolicyEngine/policyengine-us-data](https://github.com/PolicyEngine/policyengine-us-data) | Python | Microdata generation | 2026-07-02 | **no license (null)** | **NON-FREE** (all-rights-reserved) |
| [mmacpherson/tenforty](https://github.com/mmacpherson/tenforty) | Python/C++ | Federal + 11 states, TY2018-2025; pip-installable | 2026-07-06 (active) | **MIT wrapper over vendored GPL-2.0 OTS** | **STUDY-ONLY code / great ORACLE** — see §Trap |
| [kddnewton/taxes](https://github.com/kddnewton/taxes) | TypeScript | Bracket/marginal-rate calculator only | 2026-03-25 (**archived**) | **MIT** | BORROWABLE but low-value (brackets only) |
| [AustinWise/TaxStuff](https://github.com/AustinWise/TaxStuff) | C# | "IRS tax return calculator" | 2026-03-22 | **no license (null)** | **NON-FREE** (all-rights-reserved) |
| [NBER TAXSIM 35](https://taxsim.nber.org/) | Fortran (unreleased) | Federal + state liabilities; the research standard | active (service) | **source not released**; service to NBER associates/gov | **NON-FREE** — service oracle only |
| [TaxFoundation/data](https://github.com/TaxFoundation/data) | data | Datasets behind TF publications (incl. historical brackets) | — | **no license (null)** | **NON-FREE** compilation — use IRS primary instead |
| [IRS SOI / forms / ATS scenarios](https://www.irs.gov/) | data/PDF | Official forms, instructions, Rev. Procs., SOI data, ATS returns | annual | **US-Gov public domain (17 USC §105)** | **BORROWABLE** — primary source (see recon 05) |

---

## The two BORROWABLE finds that matter

### 1. Tax-Calculator parameter data — CC0 — the parameter-data cross-check (HIGH VALUE)

**License, quoted from the repo `LICENSE`:**
> "The Tax-Calculator project is in the public domain within the United States. Additionally, we
> waive copyright and related rights in the work worldwide through the **CC0 1.0 Universal** public
> domain dedication."

(GitHub's API reports `NOASSERTION`/"Other" only because its detector doesn't recognize this custom
CC0 statement — a concrete example of why we read the file rather than trust the badge.)

**What's in it (verified by grepping `taxcalc/policy_current_law.json`, 635 KB):** each parameter
is a structured object with per-year values and, for bracketed items, a value per filing status.
The keys that map onto our hand-transcribed tables:

| Our `tax_tables.rs` concept | Tax-Calculator parameter | 
|---|---|
| Ordinary brackets / rates | `II_brk1`..`II_brk7`, `II_rt1`..`II_rt7` |
| Standard deduction (+ aged/blind add'l) | `STD`, `STD_Aged` |
| LTCG / qualified-dividend breakpoints + rates (0/15/20) | `CG_brk1`..`CG_brk3`, `CG_rt1`..`CG_rt4` |
| Net Investment Income Tax (3.8%) | `NIIT_thd`, `NIIT_rt` |
| Additional Medicare Tax (0.9%) | `AMEDT_ec`, `AMEDT_rt` |
| Personal exemption | `II_em` |

**Why it's the right cross-check and not a blind import:**
- **Independence.** It's produced independently of our clean-room IRS transcription, so diffing our
  TY2024/2025 values against it is a genuine second source — catches a fat-fingered breakpoint.
- **CC0 means we *may* even vendor it**, not just observe it — a small script can extract the
  TY2024/2025 slice of these parameters into a fixture the test suite diffs `tax_tables.rs` against.
  With attribution, this is clean under both MIT and Unlicense.
- **Caveats to honor:** (a) It is a *microsimulation* parameter set — some values are inflation-
  **uprated projections** for out-years, and its "current law" reflects legislation as of its
  release, so pin a specific release/commit and treat IRS Rev. Proc. figures as authoritative on
  any disagreement. (b) It carries the *parameters*, **not** the IRS **Tax Table $50-bin structure**
  or whole-dollar rounding — those still come from the IRS instructions (recon 05 Layer 0). (c) It
  is **not** a form-filler and has **no PDF field maps**.

### 2. IRS public-domain sources — the primary borrow (already established, reaffirmed)

Confirmed in `recon/05-prior-art-verification.md`: IRS forms/instructions/Rev. Procs., **IRS SOI**
data, and the **IRS MeF ATS filled 1040 scenarios** are US-Government works, public domain under
**17 USC §105**. These remain our richest and safest borrow for both **logic derivation** (Tax
Table, QDCGT worksheet, §170(b) ceilings, whole-dollar rounding) and **golden fixtures**. Nothing
found in this deep pass displaces them.

### 3. kddnewton/taxes — MIT, but thin

Genuinely permissive (**MIT**) and thus technically borrowable, but scope is only bracket/marginal-
rate math with hard-coded year data; **archived** since 2026-03. Nothing here we can't re-derive
more reliably from the IRS in an afternoon. Catalog completeness only; not a real borrow.

---

## The trap: tenforty (MIT label, GPL substance)

`mmacpherson/tenforty` advertises **MIT** (`LICENSE.txt` is a genuine MIT text) and is 71.9% C++.
Reading the repo tree shows why the label misleads:
- It **vendors OpenTaxSolver release tarballs**: `ots/ots-releases/OpenTaxSolver2018_16.06_linux64.tgz`
  … `OpenTaxSolver2025_23.06_linux64.tgz`.
- Its computation code is OTS-derived: `src/tenforty/otslib/ots.cpp`, `ots_2018_CA_540.cpp`,
  `ots_2018_MA_1.cpp`, etc., generated from OTS via `ots.template.pyx` / `amalgamate.py`.
- The README states it "relies on the Open Tax Solver project for the underlying tax computation
  logic … wrapping its functionality into a Python library."

**Consequence:** OpenTaxSolver is **GPL-2.0**. The MIT license covers only tenforty's own glue; the
distributed *whole* is a derivative of GPL-2.0 code. **We must not treat any tenforty tax-
computation code as MIT/borrowable.** (Whether tenforty's own MIT-over-GPL packaging is itself
GPL-compliant is tenforty's problem, not a license we can rely on.)

**But it is the best *oracle* in the ecosystem:** pip-installable, federal + 11 states, TY2018-2025,
scriptable. Observing its numeric output to cross-check our returns (recon 05 Layer 2) is **not**
copying and keeps our clean-room intact. Pair it with the CC0 Tax-Calculator parameters and it
becomes a strong two-oracle verification harness.

---

## High-value asset checklist — honest scorecard

| Asset type sought | Best permissive / PD option found | Verdict |
|---|---|---|
| **1. Tax-parameter data** (brackets, std ded, LTCG breakpoints, NIIT/Medicare thresholds, phase-outs) | **Tax-Calculator `policy_current_law.json` — CC0** | ✅ **Strong win.** Vendor/cross-check freely. |
| **2. IRS PDF AcroForm field maps** | None permissive. UsTaxes/HabuTax/opentax have maps but are AGPL/GPL. | ❌ **Gap.** Extract field names directly from the **public-domain IRS PDFs** (btctax already does this); do not copy a copyleft project's map. |
| **3. Test / golden input→1040 fixtures** | **IRS MeF ATS scenarios (PD)** + **IRS instruction worked examples (PD)**; tenforty/PolicyEngine as generated oracles | ✅ for IRS PD fixtures; ⚠️ generated fixtures fine if produced by *observing* an oracle, not copying its data |
| **4. Worksheet logic (QDCGT, Sch A, 8960/8959) in portable/permissive form** | None permissive. All engines are copyleft. | ❌ **Gap.** Clean-room from IRS instructions (recon 05); cross-check the *result* against Tax-Calculator/tenforty. |
| **5. Rust crate (1040/IRS)** | None mature (only German `income-tax`, VAT crates, one abandoned 1040-ES) | ❌ **None.** Greenfield in Rust; that's fine. |

---

## Ranked top borrow candidates

1. **Tax-Calculator `policy_current_law.json` (CC0)** — vendor a TY2024/2025 parameter fixture and
   diff `tax_tables.rs` against it in CI. Highest-value, lowest-risk borrow. *(Data borrow.)*
2. **IRS public-domain corpus (17 USC §105)** — forms, instructions, Rev. Procs., SOI, ATS filled
   returns. Primary source for both **logic** (Tax Table, QDCGT, §170(b)) and **golden fixtures**.
   *(Data + logic-derivation borrow.)*
3. **tenforty + PolicyEngine as black-box oracles (observe-only)** — a scriptable, multi-engine
   numeric-cross-check harness for Layer 2. *(Verification borrow — not a code/data borrow.)*
4. **kddnewton/taxes (MIT)** — legal to copy, but so thin it's not worth it. *(Completeness only.)*

Everything else (OTS, HabuTax, py1040, ustaxlib, UsTaxes, filedcom/opentax, all of PolicyEngine/
OpenFisca) is **STUDY-ONLY** — read for *architecture and algorithm ideas*, never for code or data.
The two most instructive *designs* remain HabuTax's field-level DAG + "fail loudly" and ustaxlib's
form-framework (both already flagged copyleft in recon 05).

---

## Bottom-line recommendation

**The task's prior holds, with one concrete upgrade.** Full-engine borrowing is a dead end — the
entire mature engine ecosystem is GPL/AGPL, so **clean-room re-derivation from IRS instructions
stays the primary path for all computation logic**, exactly as recon 05 concluded. The IRS public-
domain corpus (instructions + SOI + ATS fixtures) remains our safest and richest borrow.

The upgrade this pass adds: **there is one genuinely borrowable structured dataset —
Tax-Calculator's CC0 `policy_current_law.json` — and it lines up parameter-for-parameter with our
hand-transcribed tables.** Adopt it as the independent **cross-check** (or even a vendored fixture,
with attribution) for brackets, standard deduction, LTCG breakpoints, and NIIT/Medicare thresholds.
Combined with the **IRS instructions (logic)**, **IRS ATS/worked-example fixtures (goldens)**, and
**tenforty/PolicyEngine as observe-only oracles**, we have a complete, license-clean verification
story without importing a single line of copyleft code.

**Gaps to accept honestly:** no permissive PDF field maps (extract from the IRS PDFs ourselves), no
permissive worksheet-logic to lift (clean-room it), and no mature Rust crate (greenfield). None of
these is a blocker; each is already the plan.

---

### Sources (verified 2026-07-11)

- Tax-Calculator `LICENSE` (CC0 text, read via `gh api`) — https://github.com/PSLmodels/Tax-Calculator/blob/master/LICENSE
- Tax-Calculator params — https://github.com/PSLmodels/Tax-Calculator/blob/master/taxcalc/policy_current_law.json
- tenforty (MIT wrapper; vendored OTS tarballs + `otslib/*.cpp`) — https://github.com/mmacpherson/tenforty
- habutax (GPL-2.0) — https://github.com/habutax/habutax · py1040 (GPL-2.0) — https://github.com/b-k/py1040
- ustaxlib (GPL-3.0) — https://github.com/rsesek/ustaxlib · UsTaxes (AGPL-3.0) — https://github.com/ustaxes/UsTaxes
- filedcom/opentax (AGPL-3.0) — https://github.com/filedcom/opentax
- policyengine-us (AGPL-3.0) — https://github.com/PolicyEngine/policyengine-us · policyengine-core (AGPL-3.0) — https://github.com/PolicyEngine/policyengine-core · openfisca-core (AGPL-3.0) — https://github.com/openfisca/openfisca-core
- policyengine-taxsim (MIT wrapper over AGPL) — https://github.com/PolicyEngine/policyengine-taxsim
- kddnewton/taxes (MIT, archived) — https://github.com/kddnewton/taxes
- AustinWise/TaxStuff (no license) — https://github.com/AustinWise/TaxStuff
- TaxFoundation/data (no license) — https://github.com/TaxFoundation/data
- NBER TAXSIM (source not released; service) — https://www.nber.org/research/data/taxsim
- Related codebase recon: `design/full-return/recon/05-prior-art-verification.md` (OTS/HabuTax GPL, IRS ATS/PD fixtures, Layer-0 Tax-Table method)
