# Review — SPEC_foundation_v0_1.md, Tax-Correctness (Round 1)

- **Artifact:** `design/SPEC_foundation_v0_1.md`
- **Reviewer:** independent tax-correctness reviewer, fresh context. Verified every position against verbatim archived primary text in `legal/text/` and `legal/primary-sources/`.
- **Date:** 2026-06-28
- **Verdict:** NOT yet 0 Critical/0 Important. 2 Critical (gift/donation as gain events; safe-harbor deadline/irrevocability + no fallback), 5 Important (ingest/reconciliation gaps). Money-line capital-gains engine otherwise sound and faithfully sourced.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

---

# Independent Tax-Correctness Review — `SPEC_foundation_v0_1.md` (Phase 1)

**Reviewer role:** independent tax-correctness skeptic. **Method:** every position checked against the verbatim archived primary text in `legal/text/` and `legal/primary-sources/`, not against the spec's or the research artifacts' own assertions. **Date:** 2026-06-28.

**Bottom line up front:** The core capital-gains engine (sells/spends, basis, holding period, FIFO/spec-ID, self-transfers, wash-sale) is well-modeled and faithfully sourced. But there are **two Critical defects** (gift/donation treated as gain-realization events; the Rev. Proc. 2024-28 safe-harbor deadline/irrevocability are unguarded) and a cluster of **Important** omissions in the ingest/reconciliation model that will produce missed income or corrupt basis. The spec is **not yet at 0 Critical / 0 Important** and should take one more fold before the plan gate.

---

## CRITICAL

### C1. TP1 treats **gifts and charitable donations as realization (gain/loss) events** — they are not.
TP1 states "every disposition (sell/spend/**gift/donation**) is a realization event," and §6.4 defines `Dispose{kind: Sell|Spend|Gift|Donation}` as "**taxable**; net proceeds = usd_proceeds − fee_usd," with §7.3 emitting per-lot **gain/loss** for every `Dispose`. This is wrong for gift and donation:

- **§1001(a)/(c)** (`statute-irc/26USC_s1001.html`): gain/loss arises on the "**sale or other disposition**" and is recognized on the "**sale or exchange** of property." A gratuitous transfer is neither.
- **§1015(a)** (`statute-irc/26USC_s1015.html`): the donee takes the donor's **carryover basis** (dual-basis for loss) — a rule that exists *because* the donor recognizes no gain on the gift.
- **§170(e)(1)** (`statute-irc/26USC_s170.html`): a charitable contribution yields a **deduction** (FMV if the asset would yield LTCG, else basis); it is not a gain-recognition event.
- **Rev. Proc. 2024-28 §3.11** (`irs-guidance/RevProc_2024-28.txt`, lines 360-364): the IRS itself defines **"Transfer"** as "the conveyance, **other than a sale or disposition**, … including a completed **gift, donation, contribution**, or distribution." This is direct, on-point authority distinguishing gift/donation from a taxable disposition.
- The project's own `ADDENDUM_open_questions_verified.md` Q4 says donating appreciated >1-yr crypto yields "an FMV deduction **and no capital-gain recognition**."

**Effect:** modeling a gift/donation as a `Dispose` with proceeds produces a fabricated gain (if proceeds = FMV) or a non-deductible "loss" (if proceeds = 0) — either is a wrong tax result that propagates into the Phase-2 8949. **Fix:** make gift/donation **non-recognition removals** (lot leaves at zero gain/loss); capture **FMV-at-transfer** and **ST/LT** so Phase 2 can compute the §170(e) deduction (FMV vs. basis) and the donee carryover/dual basis; flag the >$5,000 qualified-appraisal requirement (§170(f)(11)(C); CCA 202302012) as Phase-2 metadata. Only **Sell** and **Spend** are realization events (Notice 2014-21 A-6).

### C2. TP6 / FR7 ignore the **binding Rev. Proc. 2024-28 safe-harbor deadline and irrevocability**.
TP6 and FR7 present `allocate-2025` → `SafeHarborAllocation` as an unconditional one-time switch, and FR8 allows **voiding/re-doing** it. The Rev. Proc. text imposes hard constraints the spec does not encode:

- **Deadline — specific-unit allocation (§5.02(4),** lines 595-621): must be completed **before the earlier of** (a) "the date and time of the **first sale, disposition, or transfer** … on or after January 1, 2025," or (b) the **due date (incl. extension) of the 2025 return**.
- **Deadline — global allocation (§5.02(5)(a),** lines 630-632): the method must be described in books and records **before January 1, 2025**.
- **Irrevocable (§4.02(6),** lines 467-469): "A taxpayer must treat any allocation under this revenue procedure as **irrevocable** for all purposes of section 1012." This directly conflicts with FR8 `VoidDecisionEvent` re-allocation.
- **Out of scope for post-2025 lots (§4.01(2),** lines 387-395): the safe harbor cannot touch assets acquired/transferred on/after Jan 1 2025; `§1.1012-1(j)` governs those.

**Effect:** Today is 2026-06-28. For any user who made a BTC disposition during 2025 **before** an allocation existed, the specific-unit safe harbor window already closed at that first disposition, and the global-allocation window closed before 2025. Generating a safe-harbor allocation now and "voiding/re-doing" it is an **unsupportable filing position**. The spec offers no eligibility/deadline guard, no irrevocability enforcement, and **no non-safe-harbor fallback** — even though this app's full-history reconstruction uniquely lets it compute *actual* per-wallet basis under `§1.1012-1(j)` without the safe harbor. **Fix:** add a deadline/eligibility check; make the allocation irrevocable once it precedes a 2025 disposition or filing; and provide a true per-wallet-reconstruction path for users who are time-barred (note: the safe harbor remains valid for a no-2025-disposition user who allocates before filing).

---

## IMPORTANT

### I1. Inbound BTC that is **income or a received gift cannot be represented** — basis/income lost.
Income/`Acquire` events only originate from adapters; the decision-event set (`TransferLink`, `Reclassify`, `ManualFmv`, `SafeHarborAllocation`, `ClassifyRaw`, `VoidDecisionEvent`) has **no way to classify a standalone `TransferIn`** as (a) **income received into self-custody** (mining/staking/payment — ordinary income at FMV, §61; Notice 2014-21 A-8; RevRul 2023-14) or (b) a **received gift** (carryover/dual basis + tacked holding period, §1015 / §1223(2)). A `TransferIn` with no matching own-wallet outflow can therefore only be force-linked or left as a perpetual blocker, leaving the resulting lot with **unknown/zero basis** and (for income) **unrecognized ordinary income**. Given the spec claims a "complete … per-lot ledger across all venues and self-custody," this is a real gap. **Fix:** add decision events to classify an inbound as `Income{kind}` (with FMV) or `GiftReceived{donor_basis, donor_acquired_at, fmv_at_gift}`.

### I2. Gemini BTC **"Credit" is auto-mapped to `TransferIn`**, silently dropping interest/reward income.
§9.1 maps a BTC `Credit` → `TransferIn` (a non-taxable movement). A BTC credit can also be **Gemini Earn interest / promo / referral income**, which is ordinary income at FMV (§61; Notice 2014-21 A-3/A-4). Auto-classifying it as a transfer **silently misses ordinary income** and gives the lot wrong basis — and unlike the Coinbase `Order` case, it is *not* surfaced as a blocker. **Fix:** route ambiguous BTC `Credit` rows to `Unclassified`/blocker (or an income-vs-transfer disambiguation), never default-to-transfer.

### I3. No **income/reward mapping for Coinbase, Gemini, or Swan**; crypto-to-crypto **"Convert"** not enumerated.
§9.1 maps income only for **River** (`Income`/`Interest`). Coinbase BTC reward/income rows (e.g., card "Rewards Income," "Inflation Reward," Learn/Earn paid in BTC) and crypto-to-crypto **"Convert"** rows (a taxable BTC **disposition** at FMV — Notice 2014-21 A-6; §1.1012-1(h)(1)(iv)) are not listed. These are recoverable via `Unclassified` → `ClassifyRaw` (the safety net works), but the spec must **guarantee unknown BTC-side types fall to `Unclassified`, never the FR2 non-BTC drop**, and should auto-recognize known income/convert types so correct treatment doesn't depend on the user knowing a "Convert" is a taxable sale. **Fix:** enumerate Coinbase `Convert` (and any income/reward types) explicitly; state the unknown-type → `Unclassified` default.

### I4. **`txid` as dedup key collides the two legs of a cross-venue self-transfer.**
§6.2/§9 set `source_ref` to "prefer on-chain **txid**," and §9's conflict rule flags "same `source_ref`, different content" as a conflict blocker — **but the same on-chain txid appears on both the send (venue A `TransferOut`) and the receive (venue B `TransferIn`)**. As written, every cross-venue self-transfer either (a) collides into a spurious "import conflict" blocker, or (b) is silently deduped, losing one leg and corrupting per-wallet holdings and reconciliation — which is the app's central feature. Meanwhile §10 *wants* the shared txid as a reconciliation **match** signal. These two uses are incompatible for a bare-txid key. **Fix:** scope `source_ref` by `(source[, direction])`; use txid for **within-source** cross-file dedup (Swan's 3 files) and as a **cross-source match** signal, not as a global dedup key.

### I5. Swan **"transfers" authoritative-basis lots** can double-count BTC also imported from another venue.
§9.1 has Swan `transfers` create authoritative-basis lots (`ExchangeProvided`). If the same BTC was bought on a tracked venue (e.g., a Coinbase `Buy` + `Send`) and lands at Swan, the Swan transfer-in lot and the Coinbase lot describe the **same coins**, but cross-dedup is only "within Swan, by txid" (m2). Without cross-venue linkage, you get **double-counted quantity and basis**. This interacts with I4. **Fix:** specify whether a Swan transfer-in is a reconcilable `TransferIn` (preferred) or a lot-creating `Acquire`, and how it is matched against the source-venue outflow to avoid duplication and basis conflicts (which basis wins — carried vs. Swan-stated).

---

## MINOR

- **M1. "FAQ" citations are not in the archive.** TP2, TP5, TP7 cite "FAQ" (IRS Digital-Asset/Virtual-Currency FAQs), which is **not** among the 47 archived sources (`SOURCES.md`). Substance is fine — each is fully supported by archived primary text: acquisition fees in basis = **Pub 551** (lines 239-240, "commissions and recording or transfer fees"); FIFO-default/spec-ID = **Treas. Reg. §1.1012-1(j)** (`26CFR_1.1012-1_basis.xml`: adequate ID "no later than the date and time of the sale," deemed-earliest units per wallet/account, (j)(3) broker custody); self-transfer non-taxable = **§1001** + RevProc 2024-28 §3.11. Per the standard-workflow citation rule, **re-cite to the archived reg/statute/pub**.
- **M2. TP2's "disposition fees reduce proceeds" half is mis-cited.** It cites Pub 551/§1012 (the *acquisition*-basis authorities). The proceeds-reduction rule is **§1001(b)** and **Pub 544** ("Minus: Selling expenses," line 348) — cited at TP4 but not TP2.
- **M3. Daily-close FMV vs. "dominion-and-control" timestamp.** §9.2 uses daily close, but TP3's authorities key FMV to the **date and time** of receipt (RevRul 2023-14: value "at the date and time at which it is reduced to … dominion and control," lines 168-170). Daily close is a defensible documented convention but is an approximation; keep it explicitly flagged as the chosen method.
- **M4. The `Income{… Fork}` kind is effectively unreachable in a BTC-only ledger, and BCH-style fork income is silently out of scope.** Under RevRul 2019-24 (lines 107-127), a hard fork + airdrop of a *new* coin (e.g., BCH in 2017) is ordinary income at FMV — but the new coin is non-BTC and is dropped at ingest (FR2). Bitcoin forks never pay *more BTC*, so `Fork` income can't fire. The spec should **explicitly acknowledge** that fork-coin income (and later fork-coin dispositions) are out of BTC-only scope and must be handled separately, rather than implying coverage via the `Fork` kind.
- **M5. `Reclassify(TransferOut → Dispose)` drops `fee_sat`.** `TransferOut` carries `fee_sat` (BTC miner fee) but `Dispose` carries only `fee_usd`. For an on-chain **spend**, the disposed quantity should include the fee sats (all leave the taxpayer's control) with proceeds = FMV of goods. The mapping loses this. Edge case, but specify it.
- **M6. Safe-harbor `ProRata` discards the app's actual per-wallet lot knowledge.** A pro-rata `SafeHarborAllocation` is a permitted "reasonable allocation," but this app can reconstruct *actual* Jan-1-2025 per-wallet lot positions; a quantity-pro-rata basis spread can misstate which lots feed per-wallet FIFO later. Prefer `SpecificLots`/actual-position allocation where records support it.

## NIT

- **N1. Mining "trade or business" SE-tax flag not captured.** Notice 2014-21 A-9 (lines 161-165) subjects business mining to self-employment tax. Phase 1 correctly records the ordinary-income *amount*, but the foundation should tag business-vs-hobby mining so Phase 2 can route Schedule SE. Pure Phase-2 concern.
- **N2. TP8 treatment (c) is honestly flagged but worth strengthening.** §1.1012-1(h)(2)/(h)(4) (`26CFR_1.1012-1_basis.xml`: "TP pays the transaction fees using 2 units of digital asset C … must allocate the digital asset transaction costs ($2) to the disposition") shows the IRS treats **fees paid in crypto as having disposition consequences** — in the *taxable-exchange* context. There is no on-point guidance for a pure **self-transfer** miner fee, so the spec's zero-proceeds default is defensible; the disclosure ("limited guidance, swappable") is appropriate. Keep, and cite this reg as the contrary signal in the rule's doc.

---

## Positions I confirmed correct against the archive (no finding)

- **TP1 (property; sell/spend = realization)** — Notice 2014-21 A-1 (line 80), A-6; §1001(a)/(c). *(Only gift/donation are wrong — C1.)*
- **TP2 (acquisition fees → basis; disposition fees → reduce proceeds)** — Pub 551 (lines 239-240); §1.1012-1(h)(2)(i) ("cash … plus any allocable digital asset transaction costs"); Pub 544 selling expenses (line 348); §1001(b).
- **TP3 (ordinary income at FMV on dominion & control; FMV = basis; HP starts next day)** — §61; Notice 2014-21 A-4 (lines 106-108), A-8; RevRul 2023-14 (lines 193-217); RevRul 2019-24 (lines 77-127, incl. "no income from a hard fork alone if no new crypto received").
- **TP4 (HP: day after acquisition, includes disposition day, >1yr = LT)** — Pub 544 (lines 3872-3879, with worked example); §1222 ("held for more than 1 year"). Corroborated by Instructions 8949 ST/LT digital-asset boxes G–L.
- **TP5 (FIFO default; spec-ID-ready; HIFO/LIFO = forms of spec-ID)** — §1.1012-1(j): adequate identification "no later than the date and time of the sale," else deemed-earliest units per wallet/account; (j)(3) broker custody. **The spec's `§1.1012-1(j)` citation is accurate** (basis rules are in (h); identification/FIFO in (j)).
- **TP6 (per-wallet from 2025-01-01; pre-2025 aggregate; safe-harbor allocation)** — Rev. Proc. 2024-28 + §1.1012-1(j). *Mechanics sound; deadline/irrevocability missing — C2.*
- **TP7 (self-transfers non-taxable; lots carry basis + HP)** — §1001 (no sale/disposition); RevProc 2024-28 §3.11; §1.1012-1(j) (transfers carry basis/holding period).
- **TP9 (wash sale inapplicable)** — §1091 (`26USC_s1091.html`) applies only to "shares of stock or securities"; crypto is property (Notice 2014-21). Deferring loss-deferral logic is safe because the rule does not apply.
- **Deferral of 0/15/20 + NIIT + $3k loss-limit/carryforward to Phase 2/3 is SAFE.** These are downstream of the foundation (§1(h), §1411, §1211/§1212 per addendum Q2); none affect per-lot basis, gain, ST/LT split, or ordinary-income recognition, which Phase 1 captures. **No de minimis** is correctly *not* baked in (§1001(c): "the entire amount of the gain or loss … shall be recognized"; addendum Q5) — dust dispositions are correctly computed via exact sats with no threshold.

---

## Verdict

The spec's treatment of the **money-line capital-gains engine is sound and faithfully grounded** in the archived primary sources — basis (cost+fee / proceeds−fee), holding period, FIFO/specific-ID, self-transfer carryover, and wash-sale inapplicability are all correctly stated and citable, and the deferral of rate/limit mechanics to later phases is safe.

However, the spec contains **two Critical tax defects** — (C1) gift/donation modeled as gain-realization events, contradicting §1001/§1015/§170 and the IRS's own §3.11 "transfer" definition; and (C2) the Rev. Proc. 2024-28 safe harbor implemented with no deadline, eligibility, or irrevocability guard and no fallback, which invites an unsupportable filing position given the mid-2026 build date — plus a cluster of **Important** ingest/reconciliation gaps (inbound income/gift basis unrepresentable; Gemini "Credit" silently swallowing income; missing income/Convert mappings; the txid dedup-vs-match collision; Swan double-count risk).

Under the standard-workflow rule (proceed only at **0 Critical / 0 Important**), the tax treatment is **not yet sound enough to advance to the implementation plan.** It is close: C1 and the Important items are localized edits to the event/decision taxonomy and adapter mappings, and C2 needs a deadline/eligibility guard plus a true per-wallet-reconstruction fallback (which the architecture already supports). One more author-fix-then-re-review fold should clear the gate.
