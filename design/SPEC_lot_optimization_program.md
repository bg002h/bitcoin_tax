# SPEC — Lot-Identification & Tax-Optimization Program (Phase-2)

**North-star (user requirement):** let the user **pay the least federal tax the law permits and regulation does not forbid** — by choosing which lots each disposal consumes, manually or via a rate-aware optimizer, **always within the legal identification boundary** (adequate identification made *by the time of sale*; never invented after the fact).

**Status:** design approved in brainstorm (2026-06-29); revised after R0 architect review rounds 1–2 (fold records at end). This document specs **all three** sub-projects before any code; they are then built in dependency order **A → B → C**, each through the full cycle (plan → R0 0C/0I → implement TDD → whole-diff review → ship). Federal only (state tax out of scope, per the app charter). Offline/local/single-user, BTC-only, event-sourced — all existing invariants hold (NFR4 determinism, NFR5 exact arithmetic / no float money, privacy: synthetic fixtures only).

**Build order is forced:** C (optimizer) needs B (rate engine) to be rate-aware; B and C both operate on A's lot selections and on A's *hypothetical-disposal* evaluate entrypoint. A is independently valuable (manual least-tax control) and ships first. **C1/C2/I7 reshape A's object model before it is built** (the dated standing order, the LotId-keyed selection, the synthetic-disposal evaluate path, the Mode-1/attestation contract); C3 binds A to the existing §7.4 transition. These folds are landed in this spec before A is planned.

---

## Legal grounding (verified 2026-06-29; re-verify exact wording against the primary PDFs at each plan's write time, per STANDARD_WORKFLOW §4)

The binding principle for the whole program: **adequate identification must exist no later than the date and time of the sale, disposition, or transfer.** There is **no post-hoc identification** in any year. Choosing lots at tax-filing time for a sale that already executed *without* a contemporaneous identification or a pre-recorded standing order is **undocumented FIFO reported as something else** — a position the regulation forbids, and one this program must never silently produce.

- **§1.1012-1(j)(3) — specific-ID standard, timing, FIFO default, standing order, per-wallet scope.**
  - **Broker-custodied units, (j)(3)(ii):** adequate ID is made if, *no later than the date and time of the sale/disposition/transfer,* the taxpayer specifies to the custodial broker the particular units — by a transaction-level instruction **or a standing order on file with the broker.** Timing is *by the time of sale*, not post-hoc.
  - **(j)(3)(i) FIFO default:** absent adequate ID, units are sold in order of earliest acquisition among same-asset units held by that broker. **FIFO is the default.**
  - **Self-custody / non-broker units (this app's core case):** the **base regulation** requires the taxpayer to identify on its own **books and records** the particular units *no later than the date and time of the sale/disposition/transfer*; absent that, **FIFO** among units not held by a broker. Records must establish the unit is removed from the wallet. **This base rule is not subject to any post-hoc relief — ever.**
  - **Per-wallet / wallet-by-wallet scope:** Treasury reads §1012(c)(1) as requiring **wallet-by-wallet** application; the universal method is no longer permissible for 2025+. The engine's `PoolKey::Wallet` post-transition is correct.
  - **Effective date:** §1.1012-1(h)/(j) apply to acquisitions/dispositions **on or after 2025-01-01** (= `TRANSITION_DATE`).
  - A **standing order is valid adequate identification only if recorded before the units it covers are sold** (so HIFO/LIFO as a standing instruction is permitted *only* for disposals on/after the order's recorded date — see A.1/`MethodElection`).
- **Notice 2025-07 — 2025 own-books transition relief (broker-custodied units only).** During 2025-01-01 → 2025-12-31 the taxpayer may make adequate ID **on the taxpayer's own books and records** instead of specifying to the broker, by either (a) identifying the particular units **no later than the date and time of the sale**, or (b) a **standing order recorded on the taxpayer's books before the covered units are sold.** The relief changes **who** you tell (own books vs broker), **not when** (still by time of sale). **It does NOT authorize post-hoc identification.**
- **Notice 2026-20 — extends the relief through 2026-12-31; resolves 1099-DA mismatch; sets the 2027 boundary.** Issued ~2026-03 (directly on point). It (i) **extends the same own-books-by-time-of-sale relief to 2025-01-01 → 2026-12-31** for broker-held units; (ii) keeps the **same timing** (ID by time of sale, or a standing order recorded before the covered units are sold — **still no post-hoc**); (iii) adds **§4.05**: the units sold are those identified in the taxpayer's books and records **regardless of whether the broker's 1099-DA matches** (useful for the app's own-books posture); (iv) **after 2026-12-31**, the permanent (j)(3)(ii) governs — **broker-held units require specifying to the broker** (own-books-only is insufficient), while **self-custody continues** under the base own-books-by-time-of-sale rule.
- **Net timing rule the program enforces:**
  - **Self-custody, all years:** own-books ID by time of sale (base reg). No relief needed, none ever required.
  - **Broker-held, 2025–2026:** own-books ID by time of sale *or* pre-recorded own-books standing order (Notices 2025-07 & 2026-20 — *who*, not *when*).
  - **Broker-held, 2027+:** must specify to the **broker** by time of sale (transaction instruction or broker-side standing order). An own-books-only record is **not** sufficient.
- **Pre-2025 method is a declaration of historical fact** (what your *filed* returns used, under the 2019 VC FAQ A39–A40 universal regime, recited by Rev. Proc. 2024-28) — **NOT** a retroactive election. You cannot switch a closed year's method. Enforced by attestation, not by silent recomputation.
- **Rev. Proc. 2024-28** — the per-wallet basis allocation safe harbor (this app's Path A/B), effective for dispositions on/after 2025-01-01; "reasonable allocation" §5.02 (specific-unit vs global; conservation: same number of units, same asset).
- **Rate authorities (B):** §1(h) LT 0/15/20% (LT/QD stack on top of ordinary income for breakpoint placement); §1411 NIIT 3.8% on `min(NII, MAGI − threshold)` with **statutory, non-indexed** thresholds ($250k MFJ/QSS, $200k Single/HoH, $125k MFS); ST gains at ordinary rates; §1211/§1212 **$3,000 ($1,500 MFS) loss limit (statutory, non-indexed)** + indefinite character-preserving carryforward; §1222 ST/LT netting order.
- **Wash sale (§1091) does NOT apply to crypto** (not a "stock or security"); no statute has passed (recurring Greenbook/legislative proposals only). Loss harvesting is currently unconstrained. *Monitor.* (Form 1099-DA box 1i reports disallowances only for assets that are in fact securities — not a change to crypto generally.)

---

## Sub-project A — Lot-identification substrate

**Goal.** Control which lots a disposal consumes — by a **dated forward standing order** and/or **per-disposal specific-ID**, on both sides of the 2025 boundary, **never producing a position the identification regs forbid.** Pre-2025: reconstruct the carryforward to match filed returns (Universal pool, attested method). Post-2025: current-year identification (per-wallet pools), with a recorded-before-the-sale standing order and contemporaneous selections.

### A.1 The two method levers (note: they are **different kinds of object** — see N2)

1. **`pre2025_method: Fifo|Lifo|Hifo` — an attested historical-fact declaration** (a config / `cli_config` side-table; default `Fifo`). It states the method the taxpayer's *filed* pre-2025 returns actually used and drives Universal-pool consumption ordering for **pre-2025** disposals. It carries an attestation ("matches my filed pre-2025 returns") and a `pre2025_method_attested` flag. It is correctly a side-table because it records a fixed past fact, not a forward decision. Set via `config --set-pre2025-method <m>` (with attestation). **Its interaction with the safe-harbor allocation is governed by A.7 (C3) — it is *not* freely mutable once an allocation is effective.**

2. **The forward standing order — an event-sourced, dated decision** (replaces the old mutable `lot_method` config flag; see C2). New decision event:

   ```
   EventPayload::MethodElection { effective_from: TaxDate, method: LotMethod }
   ```

   It is the §1.1012-1(j)(3)(ii) standing order, recorded **on a specific date** (the decision's made-date, from the existing `Decision{seq}` + `utc_timestamp`/`original_tz`). The fold applies an election to per-wallet disposals **on/after `effective_from`**; **disposals before any election use FIFO** (the regulatory default). Multiple elections over time are honored by `effective_from` (latest-in-force at each disposal; ties broken by `decision_seq`). `effective_from` may not precede the election's own made-date (you cannot back-date a standing order; a hard `MethodElectionBackdated` blocker rejects it) — this is what makes the order *recorded before the sale*, and gives `verify` a truthful "standing order recorded YYYY-MM-DD, effective YYYY-MM-DD: HIFO" line. **Resolve-pass edges (R2-M4):** (i) `effective_from` must also be **≥ `TRANSITION_DATE` (2025-01-01)** — the forward standing order is a **post-2025-pools** instrument (it governs only `PoolKey::Wallet` disposals); pre-2025 disposals are **always** governed by the attested `pre2025_method`. An election with `effective_from < TRANSITION_DATE` is **rejected** (hard blocker, same `MethodElectionBackdated` family) so it can never be read as reaching closed pre-2025 years. (ii) A **voided** `MethodElection` is **excluded** from the in-force computation (via the existing `voided` set, resolve.rs:269-303). Set via `config --set-forward-method <m> [--effective-from <date>]` (defaulting `effective_from` to the made-date), which **appends a `MethodElection` decision** — it does **not** mutate a flag. Voidable via `void` like any decision.

`LotMethod` generalizes from the current FIFO-only enum to `Fifo|Lifo|Hifo`.

### A.2 Per-disposal specific-ID (keyed on full `LotId`, see I1)

New decision event:

```
EventPayload::LotSelection {
    disposal_event: EventId,
    lots: Vec<LotPick { lot: LotId, sat: Sat }>,   // LotId = (origin_event_id, split_sequence)
}
```

It overrides the standing order for that one disposal. It selects on the **full `LotId`**, not on `acquire_ref` (an origin event can split into multiple non-fungible fragments — distinct `split_sequence` via self-transfer relocation/`bump_split`, across the 2025 boundary, in different wallets, or twice in the same wallet — and under TP8(c) fee re-home (`rehome_onto_lot` adds fee-sat basis onto `relocated.last()`) two fragments of one origin can differ in per-sat basis by cents; `acquire_ref` cannot deterministically choose among them → would violate NFR4/NFR5). `LotId` is the existing stable, `Ord`-ered lot identity. Works pre- or post-2025. Emitted two ways:
- `reconcile select-lots <disposal-eventref> --from <lot-id>:<sat> [--from …]` (lot-ids surfaced by `verify`/`report`).
- `reconcile import-selections <file.csv>` — rows `disposal_ref,origin_event_id,split_sequence,sat` → a batch of `LotSelection` decisions (one per disposal, grouping its rows). CSV header-validated; synthetic-only in tests. **`LotId` parsing (R2-M4)** must round-trip **all three `EventId` `origin_event_id` variants** via their canonical string form — `Import{source,source_ref}`, `Decision{seq}`, and `Conflict{source,source_ref,fingerprint}` (Path-B safe-harbor seed lots use the allocation's **`Decision`** EventId as origin, resolve.rs:574; conflict-origin lots can also arise) — together with the `split_sequence`.

### A.3 Engine integration — which consume sites honor method/selection (see I2, I3)

`pools.rs` consume generalizes `consume_fifo` → `consume(method: LotMethod, selection: Option<&[LotPick]>)`. There are **six** consume sites; method/selection is honored at four, FIFO is pinned at two:

| Site (fold.rs) | Op | Honors method + `LotSelection`? |
|---|---|---|
| `Dispose` principal | `Op::Dispose` | **YES** |
| `GiftOut` | `Op::GiftOut` | **YES** (changes remaining basis + donee carryover) |
| `Donate` | `Op::Donate` | **YES** |
| `SelfTransfer` principal | `Op::SelfTransfer` | **YES** (lot choice changes future per-wallet HIFO/gains; is itself a "transfer" under (j); lets the optimizer pre-position lots) |
| `PendingOut` | `Op::PendingOut` | **NO — stays FIFO** (provisional; resolved later) |
| fee leg | `consume_fee` | **NO — stays FIFO** (de minimis; consumed after principal) |

`select-lots` may target any **method-honoring** op (Dispose/GiftOut/Donate/SelfTransfer); targeting `PendingOut` or a fee leg is rejected with `LotSelectionInvalid`.

Ordering, when no `LotSelection` applies, is by method, each a **total order** (NFR4). **FIFO is acquisition-date order, not insertion order** — this is a **deliberate, material adoption of acquisition-date FIFO that replaces the foundation's insertion-order FIFO** at all consume sites, *including for relocated and Path-B-seeded lots*. It is **not merely a tiebreak rule** (see the deliberate-correctness note below):
- **FIFO** = `acquired_at` asc, tie → `lot_id` asc — earliest *acquisition* (§1.1012-1(j)(3)(i)). A self-transfer-relocated fragment retains its **original** `acquired_at` (TP7/TP8(c) — a self-transfer is **not** a new acquisition), so under FIFO it is consumed in **acquisition order**, NOT in the order it was pushed into the destination pool.
- **LIFO** = `acquired_at` desc, tie → `lot_id` desc.
- **HIFO** = **gain basis (`usd_basis`) per sat** desc, tie → oldest `acquired_at` first, tie → `lot_id` asc.

> **Deliberate-correctness note (R0-plan C1).** Adopting acquisition-date FIFO is a **behavior change** — to relocated/seeded-lot consumption **and** to the safe-harbor conservation residue `snap.basis` — **not a no-op**. The foundation today consumes relocated lots in **insertion (push) order**, a **latent §1012 deviation** (a relocated lot legally retains its acquisition date; insertion-order FIFO can therefore consume the wrong lot, changing reported basis/term and the Universal residue against which a Rev. Proc. 2024-28 allocation is conservation-checked). The change is landed deliberately by A's plan: RED→GREEN divergence KATs (self-transfer relocation under FIFO/LIFO/HIFO; Path-B non-`acquired_at` seeding; pre-2025-relocation `universal_snapshot` residue) plus a re-verification of every existing self-transfer/Path-B/safe-harbor fixture under the new order. No real users exist yet (the foundation just shipped); recorded in `FOLLOWUPS.md`.

**HIFO basis key (M1, refined per R2-M1):** the HIFO standing order ranks **every** lot by **gain basis (`usd_basis`) per sat**, descending — ties → oldest `acquired_at` first, then `lot_id` asc (a **total order**, NFR4). **Loss-basis (`dual_loss_basis`) does NOT affect the standing-order ordering**: a **dual-basis gift lot** is keyed on its `usd_basis` (gain basis) regardless of the disposal's gain/loss zone, so the HIFO key is **deterministic and reproducible without consulting the disposal's proceeds/zone**. A **basis-pending** lot (`usd_basis = 0`) sorts **last**. These are documented **standing-order simplifications**: C's scored optimum (A.6 evaluate path) **is** zone-aware (it accounts for `dual_loss_basis` in the loss zone) and may legitimately differ from the manual HIFO order. Pinned + KAT'd.

**Applicable method per pool:**
- `PoolKey::Universal` (pre-2025 disposals) → `pre2025_method`.
- `PoolKey::Wallet` (post-2025 disposals) → the `MethodElection` **in force at that disposal's tax date** (latest `effective_from ≤ disposal date`), else **FIFO**.

Because `universal_snapshot` and the CLI pre-2025-only projection fold through this same path, `pre2025_method` is the **conservation baseline upstream of safe-harbor** — but that coupling is exactly the C3 hazard, governed by A.7.

### A.4 Validation (hard, never silently mis-applied) — conservation covers **principal only** (see I3)

A `LotSelection` is valid iff:
- (a) **conservation of principal** — `Σ picked sat == the disposal's principal sat` (the `Op`'s principal `sat`, **excluding** any on-chain `fee_sat`). The on-chain `fee_sat` of a reclassified `TransferOut`→`Dispose` is **not** part of the selection; it continues to consume **FIFO from the post-selection remainder** (deterministic). A fee-bearing reclassified disposal under a selection gets a dedicated KAT.
- (b) each `LotId` resolves to a lot that exists with ≥ the picked `sat` **remaining at that disposal's point in the fold**.
- (c) **post-2025 only:** every picked lot is in the **same wallet** as the disposal (no cross-account ID, §1.1012-1(j)); the pre-2025 Universal pool has no wallet constraint.

Any violation of (a)–(c) → hard `LotSelectionInvalid` blocker (carrying the disposal id + reason). HIFO/LIFO orderings are conservative by construction.

**Resolve-pass edges (R2-M4):** a **voided** `LotSelection` is **excluded** (existing `voided` set). **Two (non-voided) `LotSelection` decisions targeting the same `disposal_event`** → a hard **`DecisionConflict`** blocker (mirroring the existing duplicate-decision pattern, e.g. duplicate `ReclassifyOutflow` at resolve.rs:459-468) — not a `LotSelectionInvalid`.

### A.5 Compliance model — the **compliant binding levers** (see C1)

The only ways to bind a non-FIFO result that the regulation supports are:
- **(a) a dated forward standing order** (`MethodElection`, A.1) — applies to per-wallet disposals on/after `effective_from`;
- **(b) a contemporaneous `select-lots`** — a `LotSelection` whose **made-date is at/before the disposal's date-and-time of sale** (recorded before the disposal executes; filing status is irrelevant to this test); and
- **(c) a Mode-2 consultation *before* selling** (C.3) — decide the lots, then place the order/instruction and execute.

Adequate ID must exist **by the time of sale** (self-custody = base reg, **no relief ever**; broker = own-books *who-not-when* relief through 2026 per Notices 2025-07 & 2026-20; broker-held **2027+** requires broker communication). **There is no compliant post-hoc selection.**

**Per-disposal compliance status** is computed and surfaced (a `DisposalCompliance` projection over each method-honoring disposal):
- `StandingOrder { effective_from }` — an in-force own-books `MethodElection` covers it (broker-held 2027+: only if the user has separately attested the standing order is **on file with the broker**, since own-books is insufficient then);
- `Contemporaneous` — a `LotSelection` whose **made-date is at/before the disposal's date-and-time of sale** (the canonical test; not a filing-status proxy);
- `AttestedRecording` — a Mode-1-persisted selection backed by the narrow contemporaneous-ID attestation (A.6 / C.2);
- `NonCompliant` — no supported basis exists → **FIFO is the defensible filing position**, and the optimizer must say so.

**`WalletId` → custody mapping (R2-M5)** — the compliance envelope is driven by the disposal's wallet (verified against identity.rs:110-113): **`WalletId::Exchange { provider, account }` = broker-custodied** (subject to the **2027+ broker-communication** rule — own-books alone is then insufficient, so a post-2026 broker-held disposal needs the separately-attested broker-side standing order to be `StandingOrder`); **`WalletId::SelfCustody { label }` = self-custody** (own-books, **all years**, no relief ever needed). This mapping makes the `DisposalCompliance` projection unambiguous.

`verify` reports: the declared `pre2025_method` (+ whether attested); the **standing-order history** (each `MethodElection`'s recorded date + `effective_from` + method); count of `LotSelection`s; any `LotSelectionInvalid`/`MethodElectionBackdated`; and the **per-disposal compliance status** (M6). The existing `Pre2025MethodNote` advisory **still hard-codes the literal "FIFO"** (fold.rs:38: "pre-2025 lots reconstructed under FIFO (the legal default, §7.4)…") and does **not** yet reflect the declared `pre2025_method`. **Updating `Pre2025MethodNote` to render the actually-declared method is a live Sub-project-A task — not yet done** (the prior "already in the burndown commit" claim was stale); the advisory must name the declared method, never a hard-coded "FIFO" (R2-N1).

### A.6 Evaluate entrypoint (forward-compat with C; supports hypothetical disposals — see I7)

A exposes one internal, **side-effect-free** evaluate entrypoint that both the CLI and C call. It accepts an **arbitrary candidate disposal** — either an existing-ledger disposal *or* a **synthetic `Eff`** appended to the canonical timeline — plus a candidate selection set, runs it through the **same `consume`/validation/scoring path** (the proven `universal_snapshot` throwaway-fold pattern: clone, append, fold, discard), and returns the resulting lots/gains/ST-LT split. This is the substrate for both Mode-1 (score existing-ledger selections) and Mode-2 (score a not-in-ledger sale). Because no price exists for a **future** date, the entrypoint **requires `--proceeds`** when no price is available for the candidate's date (Mode-2 future dates); `--fmv` is only usable when a dataset price exists.

### A.7 `pre2025_method` ↔ effective `SafeHarborAllocation` (see C3)

`snap.basis` (Σ remaining basis in the Universal pool) is **method-dependent**: FIFO vs LIFO/HIFO consume *different* pre-2025 lots and leave *different* remaining basis (the sat total is invariant; **basis is not**). The safe-harbor conservation guard checks `alloc_basis == snap.basis`. An allocation attested against the **FIFO** residue would **fail** conservation if `pre2025_method` later flips — and an **effective** allocation is **irrevocable** (§7.4), so the user could be stranded or coerced into rewriting an irrevocable basis. Resolution:

1. **Bind the method to the allocation (storage + method-aware snapshot — R2-M2).** Add an **immutable `pre2025_method` field to the `SafeHarborAllocation` event payload** (today `SafeHarborAllocation` at event.rs:156-161 carries only `lots` / `as_of_date` / `method: AllocMethod` / `timely_allocation_attested` — note its `method` there is the *allocation* method `ActualPosition|ProRata`, **not** the lot-consumption method, so a new field is required). It records the lot-consumption method (`Fifo|Lifo|Hifo`) **captured at attestation** from the `pre2025_method` side-table when the allocation decision is appended, and is **serde-`default`ed** (→ `Fifo`) for backward-compat with already-persisted allocations. Make **`universal_snapshot` method-aware**: today the conservation residue is computed **once** with the live config (resolve.rs:520; transition.rs sums the post-consume `usd_basis`), implicitly FIFO — it must instead consume the pre-2025 Universal pool under the **allocation's recorded `pre2025_method`** so the conserved residue is stable regardless of any later config change. Each candidate allocation's conservation is checked against the residue computed under **that allocation's own recorded method** (never the live config). Because the precedence rule (A.7.2) keeps every recorded method aligned with the single pre-allocation declaration, and ≤1 allocation is ever effective (multiple → `DecisionConflict`, resolve.rs:601-609), this **collapses to one snapshot in any clean state** — so it need not be recomputed per candidate once effectiveness is settled. (Legally correct: Rev. Proc. 2024-28 requires the allocation to reflect the residue under the taxpayer's *actual historical* method.)
2. **Precedence/ordering rule.** `pre2025_method` must be declared **before** any allocation is attested. A later `pre2025_method` change that would break an effective allocation's conservation is a **material change that re-enters the gate** (STANDARD_WORKFLOW §1 "material change").
3. **Dedicated blocker.** The conflict fires precisely when the **live `pre2025_method` config differs from the effective allocation's recorded `pre2025_method`** — surfacing a hard, explanatory `Pre2025MethodConflictsAllocation` blocker (naming the allocation + the recorded vs attempted method) — **never** the generic `SafeHarborUnconservable` (which would misread a method change as bad data). The escape hatch is to **revert the live `pre2025_method` to the recorded one** (the irrevocable allocation correctly *pins* the method); the user is never coerced into rewriting an irrevocable allocation.
4. **KATs.** The promised composition KAT (pre-2025 LIFO residue → Path B conserves) **plus** a method-change-vs-effective-allocation conflict KAT.

### A — data model & artifacts
Generalized `LotMethod` (`Fifo|Lifo|Hifo`); `cli_config` keys `pre2025_method` + `pre2025_method_attested` (side-table; the old mutable `lot_method` flag is **removed** in favor of the event below); `EventPayload::MethodElection { effective_from, method }` (dated decision); `EventPayload::LotSelection { disposal_event, lots: Vec<LotPick{ lot: LotId, sat }> }`; an **immutable, serde-`default`ed `pre2025_method` field added to the `SafeHarborAllocation` payload** (captured at attestation; R2-M2) and a **method-aware `universal_snapshot`** (consumes the pre-2025 residue under the allocation's recorded method, replacing the single live-config snapshot at resolve.rs:520); `BlockerKind::{LotSelectionInvalid, MethodElectionBackdated, Pre2025MethodConflictsAllocation}`; `DisposalCompliance` projection; **`Pre2025MethodNote` rendered with the declared `pre2025_method`** (currently hard-codes "FIFO", fold.rs:38; R2-N1); `select-lots` + `import-selections` reconcile subcommands; CSV schema (`disposal_ref,origin_event_id,split_sequence,sat`) with `LotId` parsing for all three `EventId` origin variants; the A.6 evaluate entrypoint. **Tests:** ordering KATs (LIFO/HIFO pre + post, total-order tiebreaks; HIFO keys on gain-basis-per-sat, loss-basis does not reorder); `MethodElection` applies on/after `effective_from`, FIFO before, latest-in-force, backdate rejected, **`effective_from < TRANSITION_DATE` rejected**, **voided election excluded**; selection override + principal-conservation/per-wallet-violation blockers; **duplicate `LotSelection` for one disposal → `DecisionConflict`**; **voided `LotSelection` excluded**; **CSV round-trips Import/Decision/Conflict `origin_event_id` variants**; fee-bearing reclassified disposal under a selection (fee FIFO from remainder); dual-basis/basis-pending HIFO ordering; per-disposal compliance status (each variant incl. broker-held 2027+ non-compliant); **`Pre2025MethodNote` renders the declared method (not hard-coded "FIFO")**; safe-harbor composition (pre-2025 LIFO residue → Path B conserves, **method-aware snapshot**) **and** method-change-vs-effective-allocation conflict (`Pre2025MethodConflictsAllocation` when live config ≠ recorded method); A.6 evaluate with a synthetic disposal; serde backward-compat (incl. the defaulted allocation `pre2025_method`) + `fingerprint = None` for the new decisions (N3).

---

## Sub-project B — Rate / NIIT / loss-limit engine

**Goal.** Compute the **incremental federal tax attributable to the crypto activity** for a tax year (see I5), given the user's surrounding tax context — so C can compare options and `report` can show tax owed. **B is not a 1040 engine**; it computes a ceteris-paribus delta on a minimal profile.

### B.1 Inputs — minimal per-year tax profile (new config, per tax year)
- `filing_status: Single|MFJ|MFS|HoH|QSS`
- `ordinary_taxable_income: Usd` — the taxpayer's ordinary taxable income **excluding ALL app-computed crypto items** (both capital gains **and** crypto ordinary income placed on the stack — mining/staking/etc.). This is the "stack base" for ST gains and for §1(h) bracket placement. **Excluding crypto ordinary income here is mandatory** to avoid double-counting it (B.3 adds it back on the stack).
- `magi_excluding_crypto: Usd` (or the components needed) — to apply the NIIT threshold.
- `qualified_dividends_and_other_pref_income: Usd` (see I9) — qualified dividends and other §1(h) preferential-rate income **excluding app-computed crypto LT**. Under §1(h) these **share the 0/15/20 bracket space** with net LT gain and stack with it for breakpoint placement; omitting them mis-applies the 0%/15%/20% breakpoints to crypto LT.
- (optional) `other_net_capital_gain: Usd`, `capital_loss_carryforward_in: { short: Usd, long: Usd }` — for non-crypto cap activity / prior-year carryforwards.
- Stored per tax year (a `tax_profile` side-table keyed by year), set via a `tax-profile` command. Missing profile for a year being computed → hard blocker (don't guess).

### B.2 Bundled bracket/threshold tables — classified by how each item changes year-over-year (see I4)
Reference data, per tax year (like the price dataset). **Year-over-year changes are driven by enacted law, not merely CPI** — each year's table must be **sourced to the enacted authority for that year**, dated and source-noted:
- **Inflation-indexed (Rev. Proc.):** §1(h) LT 0/15/20% breakpoints; ordinary brackets; standard deduction (if used).
- **Fixed by statute (NOT inflation-indexed):** §1411 NIIT thresholds ($250k MFJ/QSS, $200k Single/HoH, $125k MFS); the §1211 **$3,000 / $1,500 MFS** loss limit.
- **Structural changes** (e.g., post-OBBBA law) are sourced to the **year's enacted law**, not a CPI bump on the prior year.

A year with no bundled table → hard blocker (`TaxTableMissing`). **KAT:** assert the NIIT threshold (and $3k/$1.5k limit) are **constant across years** while indexed items move.

### B.3 Computation (exact `Decimal`; no float)
- Net the year's crypto disposals into ST and LT, applying §1222 netting (ST losses vs ST gains, LT losses vs LT gains, then cross-net), incorporating `carryforward_in`.
- **ST net gain** → ordinary marginal rates *stacked on* `ordinary_taxable_income`.
- **LT net gain** → 0/15/20% via §1(h) bracket placement, stacked on top of ordinary income **and** of `qualified_dividends_and_other_pref_income` (LT/QD share the preferential bracket space).
- **NIIT** → 3.8% × `min(net investment income (incl. these gains), MAGI − threshold)`; threshold is **fixed by statute** (B.2).
- **Crypto ordinary income** (mining/staking/etc., already FMV-valued at receipt) is added on the ordinary stack here (it was **excluded** from `ordinary_taxable_income` per B.1 — added exactly once). Not lot-selectable.
- **Loss limit** → if net capital loss, deduct up to **$3,000 ($1,500 MFS)** against ordinary income; carry the remainder forward. **§1212(b) ordering (M3):** the **net short-term loss absorbs the $3,000 ordinary offset first**, then net long-term; the carryforward is split ST/LT character-preserving → `carryforward_out`.
- **Objective quantity = incremental delta (I5):** `total_federal_tax_attributable := tax(profile WITH app-computed crypto items) − tax(profile WITHOUT them)`, computed **ceteris-paribus** on the minimal profile. This is explicitly **not** the user's full 1040 liability and does **not** capture AGI-driven second-order effects outside the model (SS taxability, IRMAA, AMT, QBI, phaseouts) — labeled as such on output.
- **Output:** `TaxResult { st_net, lt_net, ordinary_from_crypto, ltcg_tax, niit, loss_deduction, carryforward_out, total_federal_tax_attributable (delta), marginal_rates }`.

### B.4 Refusal on unresolved hard blockers (see I6)
B **emits no `TaxResult`** for a year whose disposals are touched by **any** unresolved blocker of **`Hard` severity** — the gate keys on the **`BlockerKind::severity() == Severity::Hard`** classifier (state.rs:36-48), **not** an enumerated subset, so future hard kinds are auto-covered. Today that set is `FmvMissing`, `UncoveredDisposal`, `ImportConflict`, `DecisionConflict`, `UnknownBasisInbound`, `Unclassified`, `SafeHarborUnconservable`, plus the new `LotSelectionInvalid` / `MethodElectionBackdated` / `Pre2025MethodConflictsAllocation` (all added at `Hard`). Instead it returns a hard `TaxYearNotComputable` blocker (or an explicit "incomplete" result) — a wrong number must never be presented as authoritative. Missing profile/table remain their own hard blockers.

### B.5 Standalone value
Usable without C as a "tax owed / what-if" calculator surfaced in `report` (e.g., `report --tax-year 2025` shows the `TaxResult`, or the `TaxYearNotComputable` reason).

### B — data model & artifacts
`tax_profile` side-table + `tax-profile` command; bundled per-year tax tables + loader (`TaxTableMissing`); `TaxResult` type + the netting/stacking/NIIT/loss-limit/delta computation; `TaxYearNotComputable` hard blocker; `report` tax surfacing. **Tests:** worked-example KATs per filing status (LT bracket-crossing 0→15→20, incl. one **pushed across 15→20 by qualified dividends**; NIIT threshold crossing; ST stacking; $3k loss limit + §1212(b) ST-first carryforward; §1222 netting), each a hand-verified golden number; **NIIT threshold / $3k limit constant across years**; double-count guard (crypto ordinary income added once); incremental-delta correctness; `TaxYearNotComputable` when a year has hard blockers; `carryforward_in` ↔ prior-year `carryforward_out` consistency check/warn (M4); missing-profile/missing-table blockers; determinism.

---

## Sub-project C — Rate-aware optimizer (depends on A + B)

**Goal.** Compute the lot selection that **minimizes the incremental federal tax** for given disposals, **within the legal identification boundary**, and support pre-trade planning. C **assigns lots to disposals** (specific identification); it does **not** decide whether to sell (investment planning, out of scope) and uses **no hold/sell recommendation language.**

**C.1 Scope.** Holistic **single-year**, carryforward-linked: optimize a year's disposals together (so §1222 netting, the $3k limit, and §1(h)/NIIT bracket-fill are all accounted for — greedy-per-disposal can be strictly wrong; see resolved Q#2), linking years only via mechanical carryforward. C **refuses to optimize** a year that B reports as `TaxYearNotComputable` (I6).

**C.2 Mode 1 — post-hoc analysis (what-if by **default**, non-binding; persistence is narrowly gated).** Over the disposals already in the ledger for a tax year, compute the tax-minimizing `LotSelection` set (via A's mechanism, scored by B). The output is a **what-if proposal by default**: the selections + the **incremental tax delta vs the current filing position** (FIFO, or the in-force standing order), framed as *"this is the tax **if** you had identified thus."* **Nothing is filed or bound by running it.**

Persisting a `LotSelection` that would change a **past** disposal's filed result is permitted **only** when **both** hold:
1. the user makes an **accurate, narrow attestation** that a **genuine contemporaneous identification or a pre-recorded standing order actually existed at/before the time of that sale** matching those units — i.e., the app is **recording a real contemporaneous identification, not inventing one after the fact**; the app **must not auto-attest on `optimize accept`** and **must refuse to invite a false attestation**; and
2. it is in the **permitted envelope**: own-books relief covers **2025–2026** for broker-held units (and the base reg covers self-custody in all years), but it is **never** permitted for **2027+ broker-held** units (own-books is insufficient then; the only compliant lever is a broker-side standing order/instruction the app cannot retroactively manufacture).

When persistence is not permitted (no genuine contemporaneous basis, or a 2027+ broker-held disposal), the disposal's defensible filing position is **FIFO**, the proposal is **read-only what-if**, and C says so. `optimize accept` confers `Contemporaneous` (and persists without attestation) **only** for a disposal whose `LotSelection` decision's **made-date is at/before that disposal's date-and-time of sale** — the **A.5 `Contemporaneous` test** (recorded at/before the time of sale; mechanically checkable, the mirror of `MethodElectionBackdated`), **not** a filing-status trigger. **Being "current / not-yet-filed" never by itself confers contemporaneous status** (a disposal can be unfiled yet already executed — return due next April for a sale three weeks ago — and that is exactly the post-hoc position C1 forbids). Any **already-executed** disposal (made-date *after* its time of sale) must instead route through the narrow contemporaneous-ID **attestation gate** (→ `AttestedRecording`, envelope-checked per (1)/(2) above) or be marked **`NonCompliant`** and left read-only what-if (FIFO is its defensible filing position). Consequently, Mode-1 auto-accept over disposals already in the ledger is essentially never `Contemporaneous` — that is C1 working as designed. Persisted selections are revocable via `void`. C **surfaces the per-disposal compliance status** (A.5: `StandingOrder` / `Contemporaneous` / `AttestedRecording` vs `NonCompliant`). (`optimize run --tax-year <y>` → proposal; `optimize accept` → apply, time-of-sale-, attestation- and envelope-gated.)

**C.3 Mode 2 — pre-trade consultation (read-only what-if; the compliant forward path).** For a **hypothetical** disposal **not** in the ledger: `optimize consult --sell <sat> [--wallet <w>] [--at <date>] [--proceeds <usd>|--fmv]` → the tax-minimizing lots to sell, the resulting ST/LT split + incremental federal tax (via B with the year's profile), and **timing insight** (e.g., "X BTC of the best selection is short-term until <date>; selling on/after then would be taxed as LT, a ≈ $Y difference"). Runs through A.6's synthetic-disposal evaluate path; **requires `--proceeds` when no price exists for `--at`** (future dates). **No ledger mutation, no decision events.** This is the lever that lets the user *decide the lots before selling* and then place the standing order / contemporaneous instruction — the compliant way to bind the result. Boundary: tax decision-support only (tax consequences of a contemplated sale); **not** investment advice, no hold/sell recommendation.

**C.4 Algorithm (design note for C's own plan).** Minimize `TaxResult.total_federal_tax_attributable` (the I5 delta) over feasible per-disposal selections, subject to A.4's legal constraints. Start with a defensible, deterministic approach (per-disposal candidate generation favoring highest-effective-basis / LT-preferring lots as a **candidate generator**, then a **holistic** pass accounting for §1222 netting + the $3k limit + bracket/NIIT fill — greedy alone is unsafe). Whether a heuristic suffices or an exact method (DP/ILP) is warranted is decided in C's plan with optimality KATs. **Determinism/exactness (M5) is a C-plan acceptance criterion:** `Decimal`/`i64` money only, **`BTreeMap`/sorted iteration only** (no `HashMap` iteration), any DP/ILP table integer/`Decimal`-keyed — **no float anywhere.**

**C.5 Wash sale.** Crypto is currently exempt from §1091 → harvesting unconstrained; the optimizer may freely select loss lots. Documented + monitored.

**C — data model & artifacts:** `optimize run/accept/consult` commands; a proposal/what-if report type carrying per-disposal compliance status + delta; reuse `LotSelection` (Mode 1) — **no new event type**; persistence gated by the narrow attestation + permitted-envelope check. **Tests:** Mode-1 optimality KATs (HIFO-beats-FIFO; ST/LT tradeoff where naive-HIFO loses to a LT pick; loss-harvest within the $3k limit; per-wallet constraint respected); propose-doesn't-mutate / accept-mutates; **accept auto-confers `Contemporaneous` only when the selection made-date ≤ the disposal's time of sale**; **an already-executed disposal (made-date after its sale) is NOT auto-persisted** — it routes to the attestation gate or `NonCompliant` (no post-hoc-by-default); **accept refuses without the narrow attestation**; **accept refuses for a 2027+ broker-held disposal**; per-disposal compliance status reported; refuses to optimize a `TaxYearNotComputable` year; Mode-2 what-if + ST→LT timing insight + `--proceeds`-required-for-future; consultation never writes events; determinism.

---

## Cross-cutting

- **Determinism (NFR4):** every ordering/tiebreak fully specified (total orders in A.3); optimizer deterministic (`BTreeMap`/sorted iteration, no float); no `Date::now`/random in core.
- **Exact arithmetic (NFR5):** all money `Decimal`, sats `i64`; no float anywhere (incl. bracket/NIIT math).
- **Event-sourcing:** A's `LotSelection` and `MethodElection` are appended, voidable **decisions**; `pre2025_method`/`tax_profile`/tax tables are projection-input side-tables (not ledger state); C Mode-1 produces decisions (gated), Mode-2 produces nothing.
- **Compliance is load-bearing, not cosmetic:** no artifact, command, or doc may describe **post-hoc** selection as compliant. Adequate ID is **by the time of sale** in every year. The compliant binding levers are A.5(a/b/c).
- **Privacy:** synthetic fixtures + temp vaults only; no real reads; no PII; bundled tax tables are public reference data.
- **Safe-harbor interaction:** `pre2025_method` sets the Universal residue = the safe-harbor conservation baseline, **bound to the allocation** per A.7 — via the immutable `pre2025_method` carrier on the `SafeHarborAllocation` payload and a **method-aware `universal_snapshot`** (R2-M2); a composition KAT proves a non-FIFO filer's Path B conserves, and a conflict KAT proves a post-allocation method change (live config ≠ recorded method) is caught by `Pre2025MethodConflictsAllocation`.
- **Backward-compat (N3):** new `EventPayload`/`BlockerKind` variants are additive (serde-default); `LotSelection`/`MethodElection` decisions carry `fingerprint = None` (consistent with `SafeHarborAllocation`; persistence.rs returns `None` for non-imported) — load-bearing for NFR4; an explicit KAT confirms it. No event-fingerprint change.
- **Pre-2025 specific-ID vs closed years (M7):** `select-lots` works pre-2025 (Universal pool) but is governed by the pre-2025 regime (2019 FAQ A39–A40 / universal) and must match the taxpayer's **filed** result. A pre-2025 `LotSelection` that contradicts the attested `pre2025_method` for a **closed** year is a **restatement**, not a free optimization — surfaced as such, not offered as a costless lever.
- **Naming (N2):** the two levers are deliberately **asymmetric** — `pre2025_method` is an **attested historical fact (side-table)**; the forward standing order is a **dated decision (`MethodElection`)**. The spec keeps the `pre2025_method` name (used by C2/C3) but never describes the pair as a symmetric "two knobs."

---

## Resolved questions (the six original Open Questions, answered inline)

1. **Minimal tax-profile sufficiency.** Sufficient for *correct* (not merely precise) §1(h)/NIIT marginal placement for the stated objective, **given** (a) `ordinary_taxable_income` excludes **all** app-computed crypto items incl. crypto ordinary income (I5/B.1), and (b) the `qualified_dividends_and_other_pref_income` field places the 0/15/20 breakpoints correctly (I9/B.1). It is **not** a whole-return marginal model (AGI phaseouts, SS, IRMAA, AMT, QBI out of scope) → output is the **incremental, ceteris-paribus delta**, labeled as such (B.3).
2. **Greedy vs holistic.** **Holistic is strictly required; greedy can be strictly wrong** (coupling via §1(h) breakpoints, the $3k limit + §1212 carryforward, §1222 cross-netting — e.g., greedy "highest basis" realizing a large ST gain when a marginally lower-basis LT lot is taxed at 15% loses; greedy over-harvesting losses beyond the usable $3k wastes high-basis lots). Greedy is acceptable only as a **candidate generator** feeding the holistic scorer (C.4).
3. **`pre2025_method` vs an attested SafeHarborAllocation.** Real defect → **resolved by A.7 (C3):** bind the method-in-force to the allocation, enforce ordering (method declared before allocation; later change = material re-entry), and emit the dedicated `Pre2025MethodConflictsAllocation` blocker (never the generic `SafeHarborUnconservable`), with a conflict KAT.
4. **Adequate-ID timing vs post-hoc Mode-1.** **Decisively against the old premise:** Notices 2025-07 **and 2026-20** relieve *who you tell* (own books vs broker), **not when** (still by time of sale, or a pre-recorded standing order) — **no post-hoc in any year.** Self-custody has **no relief** (base reg, all years); broker-held own-books relief runs **through 2026-12-31**, and **2027+** needs broker communication. Resolved by C1 (Mode-1 reframed to what-if + narrowly-attested, envelope-gated persistence; per-disposal compliance status) and C2 (dated `MethodElection`). Notice 2026-20 added to Legal grounding.
5. **Bundled per-year tables — acceptable dependency?** **Yes** (same model as the price dataset; offline/deterministic/public), **with I4's correction:** do **not** inflation-adjust fixed-by-statute items (NIIT thresholds, $3k limit); source each year to **enacted law**, dated/attributed; `TaxTableMissing` is the safety. Maintenance burden, not a design flaw.
6. **Scope-creep check.** Contained: (a) B is defined as an **incremental delta** on a minimal profile, not a 1040 engine (I5); (b) C.3's timing insight stays a **tax-consequence-of-a-contemplated-sale** with **no hold/sell recommendation** language. Boundary stated in Cross-cutting + C.1/C.3.

---

## Fold record (R0 round 1)

R0 = `reviews/R0-lot-optimization-program-round-1.md` (2026-06-29). Each Critical/Important/Minor/Nit → its resolution in this revision. Engine facts re-verified against current source at fold time: `LotId { origin_event_id, split_sequence }` (identity.rs:117); six `consume_fifo` sites = `consume_fee` + `Op::{Dispose, PendingOut, SelfTransfer, GiftOut, Donate}` (fold.rs); TP8(c) fee re-home onto `relocated.last()` (fold.rs `rehome_onto_lot`); `LotMethod` was FIFO-only / `ProjectionConfig.lot_method` was a mutable flag (project/mod.rs); `TaxDate = Date` (conventions.rs).

**Criticals**
- **C1 — compliance model / post-hoc / Notice 2026-20.** Legal grounding rewritten around "adequate ID by the time of sale; no post-hoc in any year"; **Notice 2026-20 added** (extends own-books relief through 2026-12-31; §4.05 books-control-over-mismatched-1099-DA; 2027+ broker communication). Net timing rule (self-custody base reg no-relief / broker who-not-when through 2026 / broker 2027+) stated. Compliant binding levers = A.5(a) dated standing order, (b) contemporaneous `select-lots`, (c) Mode-2-before-selling. **Mode 1 demoted** to what-if-by-default (non-binding) with narrow-attestation-+-permitted-envelope-gated persistence per the user decision (C.2). Per-disposal `DisposalCompliance` status surfaced (A.5, C.2). All "post-hoc is permissible" language removed.
- **C2 — dated forward standing order.** Mutable `lot_method` side-table **removed**; replaced by event-sourced `EventPayload::MethodElection { effective_from: TaxDate, method }` (A.1) applied to per-wallet disposals **on/after** `effective_from`, **FIFO before any election**, latest-in-force by `effective_from` (tie `decision_seq`), back-dating rejected (`MethodElectionBackdated`). `verify` reports recorded + effective dates (A.5/M6). `pre2025_method` kept as an attested side-table fact.
- **C3 — `pre2025_method` ↔ effective allocation.** New A.7: method-in-force **bound to the allocation**; conservation computed against the recorded method; ordering rule (method before allocation; later change = material re-entry); dedicated `Pre2025MethodConflictsAllocation` blocker; composition + conflict KATs.

**Importants**
- **I1 — LotId key.** `LotPick { lot: LotId, sat }` keys on full `LotId` (origin + split_sequence), not `acquire_ref`; CSV gains `origin_event_id,split_sequence`; rationale (non-fungible fragments under TP8(c)) in A.2.
- **I2 — consume sites.** All six enumerated (A.3 table): Dispose/GiftOut/Donate/SelfTransfer **honor** method+selection; PendingOut + fee legs **stay FIFO**; `select-lots` may target only method-honoring ops.
- **I3 — fee conservation.** A.4(a) conserves **principal only** (`Op` principal `sat`); on-chain `fee_sat` consumes **FIFO from the post-selection remainder**; fee-bearing-reclassified-disposal KAT.
- **I4 — table indexing.** B.2 classifies items: indexed (§1(h) breakpoints, ordinary brackets, std deduction) vs **fixed by statute** (NIIT thresholds, $3k/$1.5k limit); structural changes sourced to enacted law; KAT asserts NIIT threshold/$3k constant across years.
- **I5 — incremental delta + no double-count.** B.3 objective `= tax(with crypto) − tax(without)`, ceteris-paribus; `ordinary_taxable_income` excludes **all** app-computed crypto items incl. crypto ordinary income (B.1), which is added back exactly once on the stack (B.3).
- **I6 — refuse on hard blockers.** B emits `TaxYearNotComputable` (no number) when a year's disposals carry unresolved hard blockers; C refuses to optimize such a year (B.4, C.1); KATs.
- **I7 — synthetic-disposal evaluate entrypoint.** A.6 evaluate accepts an arbitrary candidate disposal (synthetic `Eff`) through the same `consume`/validation/scoring path; **requires `--proceeds`** when no price exists for the date (Mode-2 future); built into A before C.
- **I9 — qualified dividends.** `qualified_dividends_and_other_pref_income` added to B.1, stacked with crypto LT for §1(h) breakpoint placement; 15→20 breakpoint KAT.

**Minors / Nits**
- **M1** HIFO basis key for dual-basis (loss-zone → `dual_loss_basis`) / basis-pending (`usd_basis=0` sorts last) lots — documented standing-order simplification + KAT (A.3).
- **M2** FIFO/LIFO/HIFO pinned as **total orders** in A.3. **[R0-plan C1 update (2026-06-29):** FIFO is **acquisition-date order**, a **deliberate material adoption replacing insertion-order FIFO** for relocated/seeded lots — *not* a mere tiebreak; it corrects a latent §1012 foundation deviation. See A.3's deliberate-correctness note + `FOLLOWUPS.md`.]
- **M3** §1212(b) ST-loss-first $3k absorption ordering pinned + KAT (B.3).
- **M4** `carryforward_in` ↔ prior-year `carryforward_out` consistency check/warn (B — tests).
- **M5** Optimizer determinism/exactness (Decimal/i64, BTreeMap/sorted, integer-keyed DP/ILP, no float) as a C-plan acceptance criterion (C.4).
- **M6** `verify` reports the standing order's recorded/effective dates + per-disposal compliance status (A.5).
- **M7** Pre-2025 `select-lots` vs closed years: a pre-2025 selection contradicting the attested `pre2025_method` for a closed year is a restatement, not a free lever (Cross-cutting).
- **N1** `Pre2025MethodNote` reflects the **declared** method (already in burndown commit) — kept consistent, never hard-coded FIFO (A.5).
- **N2** Asymmetry of the two levers made explicit (attested side-table fact vs dated decision); name `pre2025_method` retained (Cross-cutting).
- **N3** `LotSelection`/`MethodElection` carry `fingerprint = None`; explicit KAT (Cross-cutting + A tests).

## Fold record (R0 round 2)

R0 round 2 = the "Round 2 — fold re-review" section of `reviews/R0-lot-optimization-program-round-1.md` (2026-06-29): C1/C2/C3 + I1–I9 confirmed closed; **1 new Important, 5 Minor, 1 Nit** raised against the reshaped surface. All folded here. Engine facts re-verified against current source at this fold's write time: `SafeHarborAllocation` payload has **no** lot-method field (event.rs:156-161 — its `method` is `AllocMethod` ActualPosition/ProRata, the *allocation* method); `universal_snapshot` computed **once** with live config (resolve.rs:520); `BlockerKind::severity()` Hard set incl. `DecisionConflict`/`Unclassified`/`SafeHarborUnconservable` (state.rs:36-48); `EventId` variants Import/Conflict/Decision (identity.rs:56-105), Path-B seed lots use the allocation `Decision` id as origin (resolve.rs:574); `WalletId::Exchange{provider,account}` / `SelfCustody{label}` (identity.rs:110-113); duplicate-decision-conflict pattern (resolve.rs:459-468); `voided` set (resolve.rs:269-303); `Pre2025MethodNote` still hard-codes "FIFO" (fold.rs:38).

**Important**
- **R2-I1 — close the re-opened post-hoc hole in C.2 (Mode 1).** Default `accept`/persist is now gated on the **A.5 `Contemporaneous` test** (selection made-date **at/before the disposal's date-and-time of sale**), the mirror of `MethodElectionBackdated` — **not** on filing status. The "current / not-yet-filed" framing is removed as the gate: it never by itself confers contemporaneous status. An already-executed disposal (made-date after its time of sale) routes through the narrow contemporaneous-ID **attestation gate** (→ `AttestedRecording`, envelope-checked) or is marked **`NonCompliant`** (read-only what-if; FIFO is its defensible position). Mode-1 auto-accept over already-in-ledger disposals is therefore essentially never `Contemporaneous` (C1 by design). C tests + the `optimize accept` gating updated (C.2).

**Minors / Nit**
- **R2-M2 — C3 made implementable.** Added an **immutable, serde-`default`ed `pre2025_method` field to the `SafeHarborAllocation` payload** (captured at attestation) and made **`universal_snapshot` method-aware** (computes the pre-2025 residue under the allocation's recorded method, replacing the single live-config FIFO snapshot at resolve.rs:520). `Pre2025MethodConflictsAllocation` fires when the **live `pre2025_method` config ≠ the effective allocation's recorded method**; snapshot collapses to one (precedence rule → ≤1 effective allocation) (A.7.1/A.7.3, A artifacts, Cross-cutting).
- **R2-M1 — HIFO dual-basis key pinned.** HIFO sorts by **gain basis (`usd_basis`) per sat**, ties → oldest-first (then `lot_id`); **loss-basis (`dual_loss_basis`) does not affect the standing-order ordering** (deterministic, no proceeds/zone needed); basis-pending sorts last. C's evaluate path remains zone-aware (A.3).
- **R2-M3 — I6 refusal gates on Hard severity generally.** `TaxYearNotComputable` now keys on `BlockerKind::severity() == Hard` (not an enumerated subset), auto-covering future hard kinds (B.4).
- **R2-M4 — resolve-pass edges pinned.** `MethodElection.effective_from` must be **≥ TRANSITION_DATE** (forward-only; pre-2025 uses `pre2025_method`) else rejected; **voided** `MethodElection`/`LotSelection` excluded; **two `LotSelection`s for one disposal → `DecisionConflict`** (mirrors duplicate-`ReclassifyOutflow`); CSV `LotId` parsing round-trips all three `EventId` origin variants + `split_sequence` (A.1, A.2, A.4, A tests).
- **R2-M5 — `WalletId` → custody mapping pinned.** `Exchange{provider,account}` = broker-custodied (2027+ broker-communication rule); `SelfCustody{label}` = self-custody (own-books, all years). Verified against identity.rs:110-113 (A.5).
- **R2-N1 — stale `Pre2025MethodNote` claim corrected.** The advisory still hard-codes "FIFO" (fold.rs:38); "update `Pre2025MethodNote` to the declared method" is now a **live Sub-project-A task (not done)**, not a closed item (A.5, A artifacts).
