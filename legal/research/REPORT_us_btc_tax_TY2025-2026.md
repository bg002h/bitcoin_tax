# Research Report — US Federal Bitcoin/Crypto Taxation (TY2025–2026)

> **STATUS: RECON ARTIFACT — NOT YET A REVIEWED ARTIFACT.**
> Produced by the `deep-research` workflow (fan-out search → fetch → 3-vote adversarial
> verification → synthesis). Under our [standard workflow](../../STANDARD_WORKFLOW.md) this is
> recon input only. Every legal claim below must be re-verified against the archived primary
> sources in `../primary-sources/` before it hardens into the SPEC. Do **not** treat as legal advice.
>
> - Generated: 2026-06-28 · deep-research run `wf_78e18831-61d` (Task `wz1qm50yu`)
> - Verification: 6 angles · 23 sources fetched · 112 claims extracted · 25 verified · **25 confirmed, 0 killed** · 10 findings after synthesis
> - Scope (user-set): current law TY2025–2026 · dispositions + basis events · legal-reference + implementation-implications layers
> - Raw result JSON: `/tmp/claude-1000/-scratch-code-bitcoin-tax/71ab70cd-f674-4e66-86de-cbc9cc49e1a8/tasks/wz1qm50yu.output`

---

## ⚠️ Coverage gaps — what this report does NOT establish

The verified claim set is concentrated on the **foundational layer** and is solid there. It does
**not** contain confirmed claims for several in-scope topics. These remain **open** and must be
researched/cited separately before the spec relies on them (see Open Questions):

- **Wash-sale rule (§1091)** applicability to crypto + pending legislation — critical for the tax-loss-harvesting optimizer.
- **Rate/limit mechanics**: 0/15/20% long-term brackets, 3.8% NIIT, $3,000/yr capital-loss-vs-ordinary limit + carryforward.
- **Income-event timing**: staking (Rev. Rul. 2023-14 dominion-and-control), mining, airdrops & hard forks (Rev. Rul. 2019-24).
- **Gifts** (carryover/dual basis, holding-period tacking) and **charitable donations** (FMV deduction, qualified-appraisal threshold).
- **De minimis** exemption status for small spends (currently believed none; pending proposals).

The corresponding **primary sources are already archived** (`Notice_2025-07`, `RevRul_2023-14`,
`RevRul_2019-24`, `Pub526`, `Pub561`, `Form_8283`, etc.) — these topics are unverified-by-the-workflow,
not unsourced. A targeted second research/verification pass should close them.

---

## Executive summary

For TY2025–2026 the foundational rules are settled and primary-sourced: **digital assets are PROPERTY,
not currency.** Every disposition — sale for USD, crypto-to-crypto exchange (taxable; §1031 like-kind is
unavailable post-2017), and spending crypto on goods/services or paying a transfer fee in crypto — is a
**realization event** producing capital gain/loss (ordinary only if the asset is inventory/held for sale).
Gain = amount realized (cash + FMV received, **reduced by disposition costs**) − adjusted basis. Basis =
acquisition cost **plus acquisition fees** for purchases, or **FMV-in-USD at receipt** for crypto received
as income. Character turns on a strict **more-than-one-year** holding period, counted from the **day after
acquisition** through the disposition day. **Specific identification** is allowed only with adequate,
contemporaneous records (identify the units **no later than the time of sale**); otherwise **FIFO**
applies, now **per wallet/account** following the 2025 transition (Rev. Proc. 2024-28 safe harbor;
Treas. Reg. §1.1012-1(j)). Reporting flows through **Form 8949** (digital-asset boxes **G/H/I** short-term,
**J/K/L** long-term — **never C/F**) and **Schedule D**, with broker **Form 1099-DA** gross-proceeds
reporting from **Jan 1 2025** and basis reporting from **Jan 1 2026**.

---

## Verified findings (legal reference + implementation implications)

Each finding passed adversarial verification at the stated confidence. "⇒ Impl" = what the app must do.

### 1. Crypto is PROPERTY, not currency  · confidence: HIGH (7 merged claims, all 3-0)
**Rule.** Digital assets/virtual currency (e.g., Bitcoin) are treated as **property** for US federal tax
purposes; general property-transaction principles govern all crypto transactions. Foundational rule of
Notice 2014-21 (A-1), only narrowly modified by Notice 2023-34 (legal-tender phrasing); still controlling
for TY2025–2026, and all 2024–2026 changes (1099-DA, Rev. Proc. 2024-28, TD 10000) build on it.
**Sources.** Notice 2014-21; IRB 2014-16; IRS Digital Asset FAQs; Virtual Currency FAQs; irs.gov/filing/digital-assets; Form 8949 instructions.
**⇒ Impl.** Model every unit/lot as a property holding with an adjusted basis and holding period; no
foreign-currency gain/loss treatment; every disposition computes capital gain/loss against per-lot basis.

### 2. Character: capital vs ordinary  · confidence: HIGH (2 merged, 3-0)
**Rule.** Capital gain/loss when held as investment; **ordinary** when crypto is inventory or property
held mainly for sale to customers (IRC §1221; Notice 2014-21 A-7).
**Sources.** Notice 2014-21; IRB 2014-16.
**⇒ Impl.** Assume capital-asset treatment for the typical investor target, but expose a
dealer/business-inventory flag that re-routes gains as ordinary (away from 8949/Schedule D).

### 3. Dispositions are realization events; §1031 unavailable  · confidence: HIGH (4 merged; one 2-1)
**Rule.** Taxable dispositions include: selling for USD; crypto-to-crypto exchange (incl. assets
"differing materially in kind or extent"); spending crypto on goods/services; paying a transfer fee in
crypto. Gain/loss = amount realized (cash + FMV received, **reduced by transaction costs allocable to the
disposition**) − adjusted basis. **§1031 like-kind deferral does NOT apply** (real property only,
post-2017). The lone 2-1 vote only flagged that bare "transferring" is over-inclusive — **moving crypto
between your OWN wallets is NOT taxable.**
**Sources.** Notice 2014-21 (A-6); Digital Asset FAQ 64/65/66; Virtual Currency FAQ A-16; irs.gov/filing/digital-assets (IRC §1001(b); Treas. Reg. §1.1001-1(a)).
**⇒ Impl.** Treat crypto-to-crypto swaps and spends as simultaneous **disposition (spent asset) +
acquisition (received asset at FMV)**; capture FMV-in-USD at the instant of each swap/spend; subtract
disposition-side fees from proceeds; never offer §1031; distinguish own-wallet transfers (non-taxable)
from transfer-fee-in-crypto events (the fee is a taxable mini-disposition).

### 4. Holding period: strict >1 year, day-after rule  · confidence: HIGH (3 merged, 3-0)
**Rule.** ≤1 year = short-term; >1 year = long-term. Clock **begins the day after acquisition** and
**ends on (includes) the disposition day**. Edge case: buy Jan 1, sell the following Jan 1 = exactly one
year = **short-term**; long-term requires sale on/after Jan 2 (one year + one day).
**Sources.** Digital Asset FAQ; Virtual Currency FAQ; Form 8949 instructions (IRC §1222/§1223; Pub. 544).
**⇒ Impl.** Holding-period clock uses `acquisition_date + 1` as day 1 and counts the disposition day as
held; long-term = disposition strictly after the one-year anniversary. Optimizer should surface lots
about to cross the one-year boundary.

### 5. Basis of purchased crypto = cost + acquisition fees  · confidence: HIGH (2 merged, 3-0)
**Rule.** Basis = USD paid to acquire **plus** fees/commissions/other acquisition costs to effect the purchase.
**Sources.** Digital Asset FAQ 56; Virtual Currency FAQ A-8 (Treas. Reg. §1.1012-1; Pub. 551).
**⇒ Impl.** Per-lot basis **adds** acquisition-side fees to cost; disposition-side fees instead **reduce**
proceeds (finding 3). The data model must tag each fee as acquisition vs disposition so it lands on the
correct side of the gain/loss computation.

### 6. Crypto received as income → basis = FMV-in-USD at receipt  · confidence: HIGH (3 merged, 3-0)
**Rule.** For crypto received as payment/income, basis = FMV in USD as of date of receipt (and the
holding period for those units starts the day after receipt). Notice 2014-21 A-4 literally covers
goods/services; generalization to mining/staking/airdrops/forks is sound but rests on additional
authorities (Rev. Rul. 2019-24, 2023-14) **not among the confirmed claims** (see gaps).
**Sources.** Notice 2014-21 A-4; IRB 2014-16; Digital Asset FAQ 59.
**⇒ Impl.** Income-event ingestion records (a) FMV-in-USD at receipt as **both** ordinary income and the
new lot's basis, and (b) receipt date as holding-period start. Requires an **FMV/price oracle keyed to
the receipt timestamp.**

### 7. Specific identification vs FIFO default  · confidence: HIGH (3 merged, 3-0)
**Rule.** Specific identification is allowed only if you can identify the units and substantiate basis —
by unique digital identifier, or records showing (1) date/time acquired, (2) basis & FMV at acquisition,
(3) date/time disposed, (4) FMV/proceeds at disposition — and the identification is made **no later than
the date/time of sale**. Absent adequate ID, units are **FIFO** (earliest first), applied **per
account/wallet** (Treas. Reg. §1.1012-1(j)(3)(i); IRC §1012(c)(1)), for dispositions on/after Jan 1 2025.
For broker-custodied units after 12/31/2025 the ID must be specified **to the broker** (FAQ 85), with
transition relief under Notices 2025-7 / 2026-20. **HIFO/LIFO are NOT standalone methods** — permissible
only as a form of specific identification meeting these substantiation/timing rules.
**Sources.** Virtual Currency FAQ Q40/Q41; Digital Asset FAQ 82/85.
**⇒ Impl.** The lot-selection optimizer (FIFO/HIFO/LIFO/specific-ID/TLH) must implement **spec-ID as the
engine for any non-FIFO strategy** and generate a contemporaneous, timestamped identification record
at-or-before each sale; fall back to **per-account FIFO** when records are inadequate; for broker accounts
capture the standing-order/broker-default and emit the spec-ID instruction to the broker; persist all four
required data points per lot for substantiation.

### 8. Per-wallet/per-account basis from Jan 1 2025 (Rev. Proc. 2024-28 safe harbor)  · confidence: HIGH (3-0)
**Rule.** Effective Jan 1 2025, basis is tracked **per wallet/account** (end of universal/aggregate
pooling). Rev. Proc. 2024-28 (§§1, 5.01) provides a safe harbor under IRC §1012(c)(1) to **allocate unused
basis** to remaining units held within each wallet/account as of Jan 1 2025, based on the taxpayer's
records; paired with Treas. Reg. §1.1012-1(h)/(j) (TD 10000) for on/after-2025 activity.
**Sources.** Rev. Proc. 2024-28; irs.gov/filing/digital-assets.
**⇒ Impl.** The per-account ledger must (1) snapshot every wallet/account's holdings as of Jan 1 2025;
(2) perform and **persist a documented safe-harbor allocation** of pre-2025 unused-basis lots to specific
accounts; (3) thereafter **forbid cross-account pooling** — FIFO and spec-ID operate strictly within each
account; (4) track lots by account identity so own-account transfers carry basis/holding period to the
destination ledger.

### 9. Form 1099-DA broker reporting timeline  · confidence: HIGH (3-0)
**Rule.** Brokers report on new **Form 1099-DA**: **gross proceeds** for transactions on/after **Jan 1
2025**, **basis** for certain transactions on/after **Jan 1 2026**. The 2025 obligation applies to
**custodial** brokers (exchanges, hosted wallets, kiosks, PDAPs), not non-custodial/DeFi; penalty relief
for good-faith 2025 failures under Notice 2024-56.
**Sources.** irs.gov/filing/digital-assets (refined by 1099-DA instructions / broker-reporting FAQs).
**⇒ Impl.** The importer ingests 1099-DA where issued and reconciles broker-reported proceeds (2025+) and
basis (2026+) against the app's own per-lot ledger, **flagging mismatches**; handles the 2025 gap (proceeds
but no basis → drives 8949 box selection); continues self-tracking basis for non-custodial/DeFi activity.

### 10. Reporting on Form 8949 + Schedule D (digital-asset boxes)  · confidence: HIGH (24: 3-0; reporting-half of 10: 2-1)
**Rule.** 2025 Form 8949 uses digital-asset boxes: short-term **G** (basis reported to IRS), **H** (basis
not reported), **I** (no 1099-DA); long-term **J** (basis reported), **K** (basis not reported), **L** (no
1099-DA). **Boxes C and F must NOT be used for digital assets** — use **I** (ST) or **L** (LT) when no
1099-DA was received. Totals roll to Schedule D.
**Sources.** Form 8949 instructions (2025); irs.gov/filing/digital-assets.
**⇒ Impl.** Form generation must (1) split each disposition ST vs LT per the holding-period clock; (2)
route each transaction to the correct box from {1099-DA received?, basis reported on it?} → G/J (basis
reported), H/K (basis not reported), I/L (no 1099-DA); (3) never emit C/F for digital assets; (4) support
adjustment codes (e.g., basis corrections) and roll totals into Schedule D; (5) drive the Form 1040
digital-asset question to **Yes** when any disposition occurred.

---

## Caveats (from the workflow)

- **Source quality:** every confirmed finding rests on **primary IRS authority** (Notice 2014-21 / IRB
  2014-16, IRS Digital Asset & Virtual Currency FAQs, irs.gov/filing/digital-assets, Form 8949
  instructions, Rev. Proc. 2024-28) plus underlying IRC/Treas. Reg.; 24/25 claims passed 3-0; the single
  2-1 (claim 10) failed only on the over-inclusive word "transferring" (own-wallet transfers are
  non-taxable), corrected in finding 3. No blog/marketing sources were relied on.
- **Time-sensitivity:** per-wallet regime and 1099-DA gross-proceeds reporting effective **Jan 1 2025**;
  1099-DA **basis** reporting begins **Jan 1 2026**; broker spec-ID/transition relief runs through Notices
  2025-7 / 2026-20; the Rev. Proc. 2024-28 safe-harbor allocation is a **one-time Jan 1 2025 snapshot**.
- **HIFO/LIFO** are not standalone methods — only forms of specific identification meeting the
  substantiation/timing rules.
- Several FAQ citations predate the 2024 final regs (per-account framework) and are **reinforced, not
  overridden**, by them.

## Open questions (must close before they enter the spec)

> ✅ **RESOLVED 2026-06-28** — all five verified against the primary-source archive in
> [`ADDENDUM_open_questions_verified.md`](./ADDENDUM_open_questions_verified.md). Summaries: (1) §1091 wash-sale
> does NOT apply to crypto under current law; (2) rate/NIIT/$3k-loss-limit mechanics confirmed (§1(h)/§1411/
> §1211/§1212); (3) income-event timing = FMV at dominion-and-control (RevRul 2023-14, 2019-24, Notice 2014-21);
> (4) gifts = carryover/dual basis + tacking (§1015/§1223), charitable = FMV if >1yr held + qualified appraisal
> >$5k (§170, CCA 202302012); (5) no de minimis (§1001). The addendum still passes the SPEC's independent-review gate.

1. Does the **wash-sale rule (§1091)** currently apply to crypto (property, not a "security"), and what is
   the status of pending legislation extending it to digital assets? (Critical for the TLH optimizer.)
2. Precise **income-recognition timing** + FMV/basis/holding-period-start for **staking** (Rev. Rul.
   2023-14 dominion-and-control), **mining**, **airdrops**, and **hard forks** (Rev. Rul. 2019-24).
3. **Rate/limit mechanics**: 0/15/20% LT brackets, 3.8% NIIT threshold, $3,000/yr net capital-loss limit
   vs ordinary income with indefinite carryforward — for the optimizer's after-tax modeling.
4. **Gifts**: carryover/dual-basis rules + holding-period tacking; **charitable donations**: FMV deduction
   + qualified-appraisal threshold (Form 8283 / Pub. 526 / Pub. 561).
5. **De minimis** exemption for small spends as of TY2025–2026, and status of pending proposals.

## Sources cited by the workflow

**Primary (IRS / Federal Register):**
- Notice 2014-21 — https://www.irs.gov/pub/irs-drop/n-14-21.pdf · IRB 2014-16 — https://www.irs.gov/irb/2014-16_IRB
- Digital Asset FAQs — https://www.irs.gov/individuals/international-taxpayers/frequently-asked-questions-on-digital-asset-transactions
- Virtual Currency FAQs — https://www.irs.gov/individuals/international-taxpayers/frequently-asked-questions-on-virtual-currency-transactions
- Digital Assets hub — https://www.irs.gov/filing/digital-assets
- Form 8949 instructions — https://www.irs.gov/instructions/i8949 · (PDF) https://www.irs.gov/pub/irs-pdf/i8949.pdf
- Rev. Proc. 2024-28 — https://www.irs.gov/pub/irs-drop/rp-24-28.pdf
- Notice 2025-7 — https://www.irs.gov/pub/irs-drop/n-25-07.pdf · **Notice 2026-20 — https://www.irs.gov/pub/irs-drop/n-26-20.pdf** (broker spec-ID transition relief — NOT yet archived)
- TD 10000 (Federal Register) — https://www.federalregister.gov/documents/2024/07/09/2024-14004/...
- Form 1099-DA instructions — https://www.irs.gov/instructions/i1099da · About 1099-DA — https://www.irs.gov/forms-pubs/about-form-1099-da
- Final broker regs newsroom — https://www.irs.gov/newsroom/final-regulations-and-related-irs-guidance-for-reporting-by-brokers-on-sales-and-exchanges-of-digital-assets
- Broker-reporting FAQs — https://www.irs.gov/filing/frequently-asked-questions-about-broker-reporting
- Rev. Rul. 2023-14 (staking) — https://www.irs.gov/pub/irs-drop/rr-23-14.pdf · Rev. Rul. 2019-24 (forks/airdrops) — https://www.irs.gov/pub/irs-drop/rr-19-24.pdf
- PLR/CCA 202124008 — https://www.irs.gov/pub/irs-wd/202124008.pdf (cited under wash-sale/TLH; NOT yet archived)

**Secondary (leads only, not relied on):** The Tax Adviser (×2), RSM (×2), CoinDesk (pending-legislation lead).
