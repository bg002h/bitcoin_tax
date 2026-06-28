# Verified Addendum — Closing the 5 Open Questions

> Companion to [`REPORT_us_btc_tax_TY2025-2026.md`](./REPORT_us_btc_tax_TY2025-2026.md). The recon report
> left five in-scope topics unverified. Each is resolved below **by reading the primary-source text now in
> the local archive** (`../primary-sources/`, grep-able copies in `../text/`). Citations point to archived files.
>
> - Verified: 2026-06-28, against the archived statute/regs/guidance (all hashed in `../SHA256SUMS`).
> - Method: direct reading of primary text (not blogs, not the recon draft).
> - ⚠️ Still recon: per [standard workflow](../../STANDARD_WORKFLOW.md) §2 this author-produced addendum
>   passes the **independent review gate before its conclusions enter the SPEC.** Not legal advice.
> - Two caveats recur: **(a)** pending *legislation* is external & time-varying — not verifiable from a
>   static archive (flagged, monitor); **(b)** inflation-adjusted dollar breakpoints come from annual Rev.
>   Procs (not individually archived) but are embedded in the archived Schedule D Tax Worksheet.

---

## Q1 — Wash-sale rule (§1091): does it apply to crypto? **NO (current law).**

**Verified.** §1091(a) disallows a loss only on **"shares of stock or securities"** (and contracts/options
to acquire them): *"In the case of any loss … from any sale … of shares of stock or securities … the
taxpayer has acquired … substantially identical stock or securities … no deduction shall be allowed."*
(`statute-irc/26USC_s1091.html`). Treas. Reg. §1.1091-1 (`regulations-cfr/26CFR_1.1091-1_wash_sales.xml`)
implements it **for securities only**. Crypto is **property, not stock or a security** (Notice 2014-21;
the §1221 capital-asset definition, `26USC_s1221.html`, does not classify crypto as a security).
**⇒ Conclusion:** selling crypto at a loss and repurchasing substantially identical crypto immediately
does **not** trigger wash-sale loss deferral under current law — aggressive tax-loss harvesting is available.
**Pending (not enacted as of 2026-06-28):** repeated proposals would extend §1091 to digital assets.
External/legislative — not verifiable from the archive; monitor.
**⇒ Impl.** TLH optimizer may harvest crypto losses **without** wash-sale deferral now, BUT: (1) make the
wash-sale rule a **config + effective-date flag** that can be switched ON if Congress acts; (2) **track
repurchases within ±30 days regardless**, so the data exists if the rule activates; (3) surface an
economic-substance caution for instantaneous sell/rebuy.

## Q2 — Rate/limit mechanics. **Verified.**

- **LT rates 0/15/20%** — §1(h) taxes "adjusted net capital gain" in 0% / 15% / 20% tiers
  (`26USC_s1.html`). **Breakpoints are inflation-adjusted annually**: the statute is a formula; the actual
  TY2025/2026 dollar thresholds come from the annual Rev. Proc. inflation adjustments (NOT separately
  archived) and are embedded in the **Schedule D Tax Worksheet** in the archived Sch D instructions
  (`irs-forms/Instructions_Schedule_D.pdf`). Short-term gains → ordinary rates (§1(a)–(d)).
- **NIIT 3.8%** — §1411 imposes 3.8% on the lesser of net investment income or MAGI over a **threshold**:
  **$250,000** (MFJ/surviving spouse), **$200,000** (single/HoH), **$125,000** (MFS) — statutory, **not**
  inflation-adjusted (`26USC_s1411.html`).
- **Capital-loss limit** — §1211(b): net capital loss deductible against ordinary income is capped at
  **$3,000 ($1,500 MFS)/yr** (`26USC_s1211.html`).
- **Carryover** — §1212(b): for **non-corporate** taxpayers, unused net capital loss carries **forward
  indefinitely (no carryback)**, retaining ST/LT character (`26USC_s1212.html`). *(The 5-/10-yr carryover
  text is §1212(a), corporate — distinct.)*
**⇒ Impl.** After-tax model needs: (1) holding-period→ST/LT split routing to ordinary vs 0/15/20 brackets;
(2) LTCG breakpoints as a **per-tax-year config table** (source: annual Rev. Proc. / Sch D worksheet) —
**never hardcode**; (3) a NIIT 3.8% layer with the fixed statutory thresholds + MAGI; (4) a **$3,000/$1,500
ordinary-offset cap** and an **indefinite carryforward ledger** that preserves ST/LT character.

## Q3 — Income-event timing & basis. **Verified.**

Root: §61 — *"gross income means all income from whatever source derived"* (`26USC_s61.html`). Crypto
received as income is **ordinary income at FMV-in-USD when received**; that FMV becomes the new lot's
**basis**; holding period starts the **day after** receipt (Notice 2014-21 A-3/A-4 — Finding 6).

- **Mining** — Notice 2014-21 Q-8/A-8: FMV of mined crypto included in gross income at receipt; Q-9/A-9:
  if a **trade/business** (not as employee), net earnings are subject to **self-employment tax**
  (`text/irs-guidance/Notice_2014-21.txt`; Pub 334 referenced).
- **Staking** — Rev. Rul. 2023-14: a cash-method taxpayer includes the FMV of staking rewards in gross
  income **in the taxable year the taxpayer gains "dominion and control"** (ability to sell/transfer);
  that FMV = income and basis (`text/irs-guidance/RevRul_2023-14.txt`).
- **Hard forks / airdrops** — Rev. Rul. 2019-24: ordinary gross income under §61 when the taxpayer receives
  new crypto from an airdrop following a hard fork and has **dominion and control** (recorded on ledger &
  transferable); **no income from a hard fork alone** if no new crypto is received; basis = FMV at receipt
  (`text/irs-guidance/RevRul_2019-24.txt`).
**⇒ Impl.** Income-event ingestion stamps each receipt with: event type (mining/staking/airdrop/fork/
interest); **FMV-in-USD at the dominion-and-control moment** (price oracle keyed to that timestamp);
recognize ordinary income; set new-lot basis = that FMV; holding-period start = day after. Capture the
**availability/"dominion-and-control" timestamp** (not protocol-emission time) as the taxable moment. Flag
mining-as-business for SE tax (Schedule SE — adjacent regime).

## Q4 — Gifts & charitable donations. **Verified.**

- **Gift basis (§1015(a), `26USC_s1015.html`):** **carryover** — donee's basis = donor's adjusted basis.
  **Dual-basis rule:** if donor's adjusted basis **>** FMV at the date of gift, then **for determining
  loss** the donee's basis = that lower FMV (gain basis = carryover; sale price between the two = no
  gain/no loss).
- **Holding-period tacking (§1223(2), `26USC_s1223.html`):** when the donee takes the donor's
  (carryover) basis, the donee **tacks** the donor's holding period. *(Where the dual-basis FMV applies for
  a loss, the holding period generally runs from the gift date — nuance.)*
- **Charitable deduction (§170, `26USC_s170.html`):** §170(e)(1) reduces the deduction by any gain that
  would **not** be long-term capital gain — so **FMV deduction only for crypto held >1 year**
  ("capital gain property"); crypto held **≤1 year** (or ordinary-income property) → deduction limited to
  **basis**. Donating appreciated >1-yr crypto yields an FMV deduction **and** no capital-gain recognition.
- **Substantiation/appraisal:** §170(f)(11)(C) — property donations **>$5,000** require a **qualified
  appraisal** + Form 8283 Section B (`irs-forms/Form_8283_Noncash_Charitable.pdf`; Pub 526/561); **>$500,000**
  must attach the appraisal. **CCA 202302012** (`text/irs-guidance/CCA_202302012.txt`): crypto donations
  **require a qualified appraisal** — the "readily-valued/publicly-traded securities" exception of
  §170(f)(11)(A) does **not** apply because crypto is **not a security**.
**⇒ Impl.** Support: (1) **gift-in** — store donor's adjusted basis **and** donor's acquisition date
(carryover + tacking) **and** FMV at gift (dual-basis loss path) → **two basis figures** per gifted lot;
(2) **gift-out / donation** as non-sale dispositions; (3) charitable-deduction calc = FMV if lot held
>1 yr else basis; surface the **>$5,000 qualified-appraisal requirement** + Form 8283; **do not** treat
crypto as appraisal-exempt. (Gift tax / Form 709 is an adjacent regime — out of core scope.)

## Q5 — De minimis exemption for small spends? **NONE (current law).**

**Verified.** No de minimis exemption exists. §1001(c) (`26USC_s1001.html`): *"the entire amount of the
gain or loss … on the sale or exchange of property shall be recognized."* Notice 2014-21 created none.
**Every** disposition — even buying coffee — computes gain/loss vs basis.
**Pending (not enacted as of 2026-06-28):** the Virtual Currency Tax Fairness Act and similar bills have
proposed a per-transaction de minimis (commonly ~$200). External/legislative — not verifiable from the
archive; monitor.
**⇒ Impl.** Compute gain/loss on **every** disposition regardless of size; **do not** bake in a threshold.
But architect a **configurable, effective-dated de-minimis rule (off by default)** so small spends can be
excluded if/when enacted — and retain per-transaction records to apply it.

---

## Archive gaps this pass closed
Added to the archive so every citation above is locally backed (re-hashed; `SHA256SUMS` now 47/47 OK):
- **26 USC §1223** (holding-period tacking) — `statute-irc/26USC_s1223.html`
- **26 USC §61** (gross income) — `statute-irc/26USC_s61.html`
- **CCA 202302012** (crypto charitable appraisal) — `irs-guidance/CCA_202302012.pdf`

## Residual items (not blocking; for SPEC-time)
- **Pending legislation** (wash-sale extension; de minimis): monitor; both designed as config flags above.
- **TY2025/2026 inflation-adjusted LTCG breakpoints & standard amounts:** use the archived Sch D Tax
  Worksheet, or add the specific annual Rev. Proc. inflation-adjustment docs to the archive.
- **Adjacent regimes touched but out of core capital-gains scope:** gift tax (Form 709), self-employment
  tax (Schedule SE) — revisit only if the app's scope expands.
