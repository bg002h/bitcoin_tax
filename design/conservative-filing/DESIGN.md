# Conservative / Defensive Filing — design of record (brainstorm output)

**Status:** brainstorm complete; awaiting user review → formal SPEC → independent review → plan.
**Date:** 2026-07-20. **Approach chosen:** C (primitives first) now, aiming for B (guided wizard) soon.

---

## 1. Persona & goal

A high-earner with 10+ years of Bitcoin, **poor/impossible records**, who wants the tax
equivalent of "leave me alone" and will **overpay for peace**. Least effort that still produces a
return that **keeps offensive tax evaluators away** in an adversarial system. (Concretely: the busy
physician who used BTC from a decade ago.)

## 2. Core principle (the whole feature in one line)

**Sell what you can prove; hold what you can't** — and where basis is unprovable, report
**conservatively** (max gain) so there is nothing for an examiner to adjust upward.

Two hard guardrails:
- **Conservative reporting ≠ omission.** Every disposal (including private/P2P sales with no
  1099-DA) is reported. Overpaying is the defense; hiding income is evasion. The tool never helps
  omit a taxable event.
- **Any basis > $0 is a position you must defend; $0 is the only unassailable one.** So $0 is the
  default filed basis for unprovable coins; anything less punitive is an explicit, warned choice.

## 3. The mental model

1. **Two buckets.** *Documented lots* (imported, real basis) and *no-records tranches* (declared by
   acquisition ERA — "~5 BTC held since before 2016" — with a conservative basis).
2. **Matching is steered (emergent).** The HIFO default already keys on gain-basis DESC and sorts
   `$0`/basis-pending lots LAST (`pools.rs:272-274`), so sales draw documented (higher-basis) lots
   first and the no-records tranches only when everything documented is exhausted. Gains compute
   from *provable* basis; the no-records coins stay untouched.
3. **Warn at the boundary.** Before a disposal's matched legs reach into a no-records tranche, stop
   the filer: "this reaches your pre-2016 tranche — basis $0/estimated, gain $X — continue?"
4. **Custody-aware compliance (narrowly scoped).** For a **2027+ sale of a COVERED, broker-held**
   coin being specific-ID'd, warn that own-books ID is insufficient — must select lots *at the
   exchange* or it defaults to FIFO. Do NOT fire for self-custody, noncovered transfer-ins, or
   private sales (own records control there in every year).
5. **Nudge structure.** Actively suggest keeping the oldest/no-records coins in **self-custody**,
   where own-books specific-ID never expires (the safest configuration).
6. **Filing-ready + documented.** Output is real 8949/Schedule D **plus a methodology disclosure**
   ("basis for pre-20XX holdings unreconstructable; treated conservatively as $0, long-term"). The
   disclosure *is* a core part of the audit defense (good-faith, reasonable, consistent method → no
   understatement to penalize).

## 4. The floor's role (resolved)

- **File $0 by default** (unassailable) for no-records tranches.
- **Always compute + show the documented price-floor** as an informational what-if ("reconstructing
  this tranche could save ~$X") from the app's bundled daily BTC-USD price data — so overpayment is
  a *conscious* choice, never silent.
- **Promoting the floor to an actually-filed position is an explicit, warned opt-in** ("this claims
  basis > $0 and adds audit surface"), with a matching methodology disclosure generated for it.

## 5. Tax grounding (verified this session; sources in-thread)

- Per-wallet basis tracking (Treas. Reg. §1.1012-1(j)) — dispositions **on/after Jan 1, 2025**.
- Broker 1099-DA: **gross proceeds** reporting from 2025; **basis** reporting from **2026**.
- Own-books specific-ID relief for broker-held units runs **through 2026-12-31** (Notice 2025-7,
  **extended by Notice 2026-20**). **2027+:** covered broker-held coins need identification *to the
  broker* or default to FIFO. **Self-custody:** own-books works in every year, forever.
- **Transfers between a filer's own accounts are non-taxable** and never reset basis or holding
  period. Only the sale is taxable; gain = proceeds − *actual* basis; long-term if held >1 yr from
  original acquisition (custody hops are irrelevant).
- **§1014 step-up at death** zeroes the embedded gain on coins held to death — the ultimate deferral
  for low-basis no-records coins.
- Post-2026 the app's role sharpens: it is the **own-books system of record for basis**, filling in
  the basis brokers won't report (noncovered transfer-ins, private sales) on the 8949.

## 6. Scope

**In scope (v1, Approach C primitives):** see §7. **Bounded by construction:** the conservative-
basis surface only covers the **old, pre-1099-DA-era** coins; anything acquired 2026+ has records
(real basis, tracked normally) — the no-records tranches are a **finite, shrinking** historical
artifact, not an ongoing concern.

**Non-goals (v1):** the full guided wizard (that's **B**, next); ProRata auto cross-wallet split
(minimum-honest note only — separate feature); AMT computation (screen-only stays); non-BTC assets.

## 7. Decomposition — Approach C (primitives first, each independently TDD'd)

| # | Primitive | New vs existing | Notes |
|---|-----------|-----------------|-------|
| P1 | **No-records vintage tranche** — declare N BTC in era E at conservative basis (default $0), producing tranche lots tagged with a new `BasisSource::EstimatedConservative` (name TBD) | NEW (core data type) | era → long-term holding period + the floor-price window |
| P2 | **Steer toward documented lots** | EMERGENT under HIFO ($0 sorted last, `pools.rs:272`) | mostly free; confirm with tranche lots + make HIFO the posture default |
| P3 | **Dip-into-no-records warning** — advisory when a disposal's matched legs include an `EstimatedConservative` tranche lot; surfaces tranche basis + resulting gain | NEW advisory | builds on existing blocker/advisory surface |
| P4 | **Custody-aware compliance warning (scoped)** — refine `ForbiddenBroker2027` surfacing to fire only for covered broker-held specific-ID; never self-custody / noncovered / private | REFINE existing (`optimize.rs:453`) | needs covered/noncovered modeling — verify against final broker regs before wording |
| P5 | **Documented price-floor engine** — era low from bundled price data; informational what-if by default; opt-in to file | NEW calc on existing price data | |
| P6 | **Overpayment-delta** — tax difference $0 vs floor for a tranche | NEW calc | drives the "could save ~$X" line |
| P7 | **Methodology disclosure output** — generated statement matching the positions taken | NEW output | core audit-defense deliverable |
| P8 | **Self-custody nudge** — advisory to hold oldest/no-records coins in self-custody | NEW advisory | |

**Approach B (soon):** a guided "Defensive Filing" wizard composing P1–P8 (reconstruct → declare
tranches → choose $0/floor → review overpayment + custody nudges → forms + disclosure).

## 8. Open questions (to settle in the formal SPEC)

- **Tranche representation:** new `EventPayload` variant vs a special `Acquire` + `BasisSource`?
  (Leaning: a declared-acquire with the new `BasisSource` + an era field.)
- **Floor definition:** era-low close vs first-documentable price — and the exact "acquisition
  window" the filer asserts (drives both the floor and the holding-period date).
- **Covered/noncovered modeling (P4):** how much of the broker transfer-statement machinery to
  model vs. a simpler "is this a covered broker lot?" flag; verify against the final regs.
- **Disclosure format (P7):** free-form statement, a Form 8275 disclosure, or an attached memo — and
  whether it's filing-attachable or advisory-only.
- **Interaction with pseudo-mode:** this is FILING-READY (not the estimate-only pseudo path); confirm
  the two don't collide (a conservative tranche is a real, filed position, not a synthetic estimate).

## 9. Engine grounding (confirmed 2026-07-20, HEAD de5e4a1)

- HIFO sorts `$0`/basis-pending lots LAST → steering emergent (`pools.rs:250-274`).
- `BasisSource` today: ExchangeProvided / ComputedFromCost / FmvAtIncome / CarriedFromTransfer /
  GiftCarryover / GiftFmvFallback / SafeHarborAllocated / ReconstructedPerWallet / SelfTransferInbound
  — **none** is "estimated-conservative"; P1 adds one (`event.rs:17`).
- Conservative-$0 machinery exists for inbound self-transfers (`UnknownBasisInbound` → `$0`,
  non-taxable) — precedent for the tranche pattern (`resolve.rs`).
- Custody kinds: `WalletId::{Exchange, SelfCustody}` (`identity.rs:110`); custody→envelope logic
  already present (`optimize.rs:453`).
- Bundled daily BTC-USD price data (~5,800 rows) available for the floor engine (P5).
