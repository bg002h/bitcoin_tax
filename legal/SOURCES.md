# Primary-Source Legal Archive — Manifest (SOURCES.md)

Verbatim US federal tax **primary authority** for the bitcoin_tax (TaxApp) project, archived locally
with provenance so the legal basis of every calculation is **defensible** even if the source sites
reorganize or change. This is the legal-defense index.

- **Retrieved:** 2026-06-28 (UTC). IRS forms/pubs/guidance batch ~20:48Z; statute/regs/FR batch ~21:03Z.
- **Files:** 47 documents, ~15 MB, under `primary-sources/` (44 archived 2026-06-28 + 3 added by the
  open-questions verification pass: §1223, §61, CCA 202302012).
- **All sources are official government hosts** (irs.gov, govinfo.gov, ecfr.gov) — no secondary mirrors
  were needed; every fetch returned HTTP 200.
- **Integrity:** full SHA-256 for every file is in [`SHA256SUMS`](./SHA256SUMS). Re-verify anytime:
  ```
  cd /scratch/code/bitcoin_tax/legal && sha256sum -c SHA256SUMS      # expect: 47 OK
  ```
  Raw fetch log (status/bytes/hash/content-type/url): [`_provenance/fetch_log.tsv`](./_provenance/fetch_log.tsv).
  Re-runnable fetch scripts: [`_scripts/`](./_scripts/).
- **Companion recon report** (NOT a primary source; recon only): [`research/REPORT_us_btc_tax_TY2025-2026.md`](./research/REPORT_us_btc_tax_TY2025-2026.md).
  "Finding N" / "Open Qn" references below point into that report.

> ⚠️ These are statutes/regs/IRS guidance as captured on 2026-06-28 for TY2025–2026. Law changes; re-pull
> and re-hash before relying on them in a later tax year. Not legal advice.

---

## 1. IRS sub-regulatory guidance — `primary-sources/irs-guidance/`
Notices, Revenue Rulings, a Revenue Procedure, and a Chief Counsel memo. URL base `https://www.irs.gov/pub/irs-drop/` (CCA: `irs-wd/`).

| Citation | File | SHA-256 (short) | Relevance to app |
|---|---|---|---|
| **Notice 2014-21** | `Notice_2014-21.pdf` | `5b582aee72dc` | Foundational: crypto = property (Findings 1–3, 6) |
| **Notice 2023-34** | `Notice_2023-34.pdf` | `8710443455d5` | Modifies 2014-21 (legal-tender phrasing) |
| **Rev. Rul. 2019-24** | `RevRul_2019-24.pdf` | `e0ccbbefc302` | Hard forks & airdrops income (Open Q2) |
| **Rev. Rul. 2023-14** | `RevRul_2023-14.pdf` | `e2ef8cdc432b` | Staking rewards — dominion & control timing (Open Q2) |
| **Rev. Proc. 2024-28** | `RevProc_2024-28.pdf` | `5efcc0f0206c` | Per-wallet basis transition + safe harbor (Finding 8) |
| **Notice 2025-7** | `Notice_2025-07.pdf` | `0a2b5478018e` | 2025 transition relief for specific identification (Finding 7) |
| **Notice 2024-56** | `Notice_2024-56.pdf` | `c80173f55281` | Broker transition/penalty relief, 2025 (Finding 9) |
| **Notice 2024-57** | `Notice_2024-57.pdf` | `1bff7c80e6b6` | Broker reporting relief, certain transactions |
| **Notice 2026-20** | `Notice_2026-20.pdf` | `37ff4019430d` | Broker specific-ID transition relief (Finding 7) |
| **CCA 202124008** | `CCA_202124008.pdf` | `334a22f1de94` | Crypto decline-in-value / wash-sale context (Q1, TLH) |
| **CCA 202302012** | `CCA_202302012.pdf` | `42b763510df2` | Crypto charitable donation requires qualified appraisal (Q4) |

## 2. IRS Publications — `primary-sources/irs-publications/`
URL base `https://www.irs.gov/pub/irs-pdf/`.

| Citation | File | SHA-256 (short) | Relevance to app |
|---|---|---|---|
| **Pub. 544** Sales & Other Dispositions of Assets | `Pub544_Sales_and_Other_Dispositions.pdf` | `b8282a22f637` | Gain/loss, holding period, like-kind (Findings 3–4) |
| **Pub. 551** Basis of Assets | `Pub551_Basis_of_Assets.pdf` | `0fbd4f0d37d5` | Cost basis incl. acquisition fees (Finding 5) |
| **Pub. 525** Taxable & Nontaxable Income | `Pub525_Taxable_Nontaxable_Income.pdf` | `39b0fa5e04c5` | Income events / FMV-at-receipt (Finding 6, Open Q2) |
| **Pub. 550** Investment Income & Expenses | `Pub550_Investment_Income_Expenses.pdf` | `83552968a07e` | Capital gains, wash sales, loss limits (Open Q1, Q3) |
| **Pub. 526** Charitable Contributions | `Pub526_Charitable_Contributions.pdf` | `607011d9065a` | Crypto donations FMV deduction (Open Q4) |
| **Pub. 561** Determining Value of Donated Property | `Pub561_Value_of_Donated_Property.pdf` | `c51b97c8d89b` | Qualified-appraisal threshold (Open Q4) |

## 3. IRS Forms & Instructions — `primary-sources/irs-forms/`
URL base `https://www.irs.gov/pub/irs-pdf/`.

| Citation | File | SHA-256 (short) | Relevance to app |
|---|---|---|---|
| **Form 8949** | `Form_8949.pdf` | `274513891e4e` | Disposition reporting form |
| **Instructions for Form 8949** | `Instructions_8949.pdf` | `f077991c83b0` | Digital-asset boxes G–L; adjustment codes (Finding 10) |
| **Schedule D (1040)** | `Schedule_D_1040.pdf` | `90564c8b7e49` | Capital gain/loss summary |
| **Instructions for Schedule D** | `Instructions_Schedule_D.pdf` | `650643a68637` | Loss limitation/carryforward mechanics (Open Q3) |
| **Form 1099-DA** | `Form_1099-DA.pdf` | `e6f397924211` | Broker digital-asset return (Finding 9) |
| **Instructions for Form 1099-DA** | `Instructions_1099-DA.pdf` | `1ae3947a4b57` | What brokers report & when (Finding 9) |
| **Form 8283** Noncash Charitable Contributions | `Form_8283_Noncash_Charitable.pdf` | `389ab1b7c01b` | Donations >$5k appraisal reporting (Open Q4) |

## 4. Statute — Internal Revenue Code (26 U.S.C.) — `primary-sources/statute-irc/`
Official per-section HTML granules, **govinfo USCODE-2024 edition**. URL base
`https://www.govinfo.gov/content/pkg/USCODE-2024-title26/html/`.

| Citation | File | SHA-256 (short) | Relevance to app |
|---|---|---|---|
| **IRC §1** Tax imposed (incl. §1(h) cap-gains rates) | `26USC_s1.html` | `725cb94c9ca6` | 0/15/20% LT brackets (Q2) |
| **IRC §61** Gross income defined | `26USC_s61.html` | `cacb490fcdcc` | Root of income recognition for crypto received (Q3) |
| **IRC §170** Charitable contributions | `26USC_s170.html` | `53cab7897a10` | Donation deduction; §170(f)(11) appraisal (Q4) |
| **IRC §1001** Determination of gain/loss | `26USC_s1001.html` | `8fbbd2b4eed5` | Realization; amount realized (Finding 3; Q5 de minimis = none) |
| **IRC §1011** Adjusted basis for gain/loss | `26USC_s1011.html` | `bfff77717fa2` | Adjusted-basis rule |
| **IRC §1012** Basis = cost (incl. (c) per-account) | `26USC_s1012.html` | `b28d3253e258` | Cost basis; per-account FIFO authority (Findings 5, 8) |
| **IRC §1015** Basis of gifted property | `26USC_s1015.html` | `e5d9d9295e15` | Gift carryover/dual basis (Open Q4) |
| **IRC §1016** Adjustments to basis | `26USC_s1016.html` | `e5c335270b33` | Basis adjustments |
| **IRC §1031** Like-kind exchanges | `26USC_s1031.html` | `23a71e50da47` | Real-property-only; unavailable for crypto (Finding 3) |
| **IRC §1091** Wash sales | `26USC_s1091.html` | `c808ed4bb0b3` | Applies to "securities" (Open Q1) |
| **IRC §1211** Limitation on capital losses | `26USC_s1211.html` | `4d1b4a95620f` | $3,000/yr vs ordinary income (Open Q3) |
| **IRC §1212** Capital loss carrybacks/carryovers | `26USC_s1212.html` | `a913d91341b7` | Indefinite carryforward (Open Q3) |
| **IRC §1221** Capital asset defined | `26USC_s1221.html` | `f2f87653fd91` | Capital vs ordinary character (Finding 2) |
| **IRC §1222** Capital gains/losses terms | `26USC_s1222.html` | `b0e396d95b4d` | ST/LT definitions; holding period (Finding 4) |
| **IRC §1223** Holding period of property | `26USC_s1223.html` | `6612a42ba24e` | Holding-period tacking for gifted lots (Q4) |
| **IRC §1411** Net Investment Income Tax | `26USC_s1411.html` | `57eb2f33a690` | 3.8% NIIT (Q2) |

## 5. Treasury Regulations (26 CFR) — `primary-sources/regulations-cfr/`
Official **eCFR** snapshot **2025-12-01**, Title 26 part 1. URL base
`https://www.ecfr.gov/api/versioner/v1/full/2025-12-01/title-26.xml?part=1&section=`.

| Citation | File | SHA-256 (short) | Relevance to app |
|---|---|---|---|
| **Treas. Reg. §1.1012-1** Basis of property | `26CFR_1.1012-1_basis.xml` | `909e34e133d9` | (c) adequate identification; **(j) digital-asset per-account rules** — confirmed present (Findings 5, 7, 8) |
| **Treas. Reg. §1.6045-1** Broker returns | `26CFR_1.6045-1_broker_reporting.xml` | `aac2a2a49bbc` | Digital-asset broker reporting (Findings 9, 10) |
| **Treas. Reg. §1.1091-1** Wash sales | `26CFR_1.1091-1_wash_sales.xml` | `e7afa0231879` | Wash-sale mechanics (Open Q1) |
| **Treas. Reg. §1.1031(a)-1** Like-kind | `26CFR_1.1031a-1_like_kind.xml` | `04726f4eebed` | Like-kind scope (Finding 3) |
| **Treas. Reg. §1.1015-1** Gift basis | `26CFR_1.1015-1_gift_basis.xml` | `064bc0f56df4` | Dual-basis rule (Open Q4) |
| **Treas. Reg. §1.170A-13** Charitable recordkeeping | `26CFR_1.170A-13_charitable_records.xml` | `d437cd7de7f2` | Substantiation/appraisal (Open Q4) |

## 6. Federal Register — `primary-sources/federal-register/`

| Citation | File | SHA-256 (short) | Relevance to app |
|---|---|---|---|
| **TD 10000**, 89 FR 56480 (2024-07-09), RIN 1545-BP71 — final digital-asset broker regs (104 pp.) | `TD_10000_89FR56480_broker_regs.pdf` | `af59384ad232` | Source of §1.6045-1 & §1.1012-1(j); preamble explains intent (Findings 8–10) |

---

## Coverage notes & known gaps
- The archive **covers all topics in scope**. The five topics the recon report left unverified are now
  **verified against this archive** in [`research/ADDENDUM_open_questions_verified.md`](./research/ADDENDUM_open_questions_verified.md)
  (wash-sale §1091 does NOT apply; rate/NIIT/loss-limit mechanics; income-event timing; gifts/charitable;
  de minimis = none). That addendum still passes the SPEC's independent-review gate before its conclusions are relied on.
- **De minimis:** there is currently **no** primary source creating a de-minimis exemption for crypto spends;
  the archive's gain/loss authorities (§1001, Notice 2014-21) govern every disposition regardless of size.
  Any future de-minimis rule would be new authority to add here.
- **Not archived (deliberate):** TD 10021 (Dec-2024 DeFi-broker rule) — repealed by Congress under the CRA in
  2025; out of scope for a centralized-exchange app. Add only if DeFi support is ever in scope.
