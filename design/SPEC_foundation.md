# SPEC — bitcoin_tax (TaxApp), Phase 1: Foundation (v0.5)

- **Status:** DRAFT v0.5 — folds round-1/2/3/4 independent reviews (tax + engineering + architecture). Pending round-5 re-review to green (0 Critical / 0 Important) before the plan gate.
- **History:** v0.1 → v0.2 (R1) → v0.3 (R2) → v0.4 (R3) → v0.5 (R4). Superseded specs in git history; all reviews in `reviews/`.
- **Date:** 2026-06-28
- **Phase:** 1 of 3 — **foundation only**: ingestion → canonical event ledger → lot engine → transfer reconciliation, on an encrypted store. Forms (Phase 2) and the optimizer (Phase 3) get their own spec→plan→implement cycles.

## 0. References (verified at write time, against the local archive)
- Legal report + addendum: `legal/research/REPORT_us_btc_tax_TY2025-2026.md`, `legal/research/ADDENDUM_open_questions_verified.md`.
- Primary sources + manifest: `legal/SOURCES.md` (47 docs; `legal/SHA256SUMS` 47/47); grep-able text in `legal/text/`.
- Reviews folded: `reviews/architecture-review-phase1-foundation-round-1.md`; `reviews/spec-review-phase1-{engineering,tax}-round-{1,2,3}.md`. Fold map: §18.
- Deferred items: `FOLLOWUPS.md`.

## 1. Purpose, scope, non-goals
**Purpose.** A local, offline, single-user desktop app (CLI first) that reconstructs a complete, auditable **per-lot Bitcoin ledger** across all venues + self-custody, so later phases can produce correct tax forms and tax-optimal sell recommendations.

**In scope (Phase 1).** Ingest Coinbase, Gemini, River, Swan exports; normalize to an append-only canonical event log; derive a per-wallet lot ledger as a deterministic projection; model self-custody wallets + assisted transfer reconciliation; reconstruct full history for basis; handle the Jan-1-2025 per-wallet transition; PGP-encrypted vault; a CLI to import/reconcile/inspect/resolve/verify/export.

**Non-goals (Phase 1).** Form generation; the optimizer; non-BTC assets; multi-user; networked operation; GUI.

## 2. Tax positions (TP) — each cited to the archive; engine encodes them as named, swappable rules
| ID | Position | Archived basis | Uncertainty |
|----|----------|----------------|-------------|
| TP1 | BTC is **property**. A **sale or spend** is a realization (gain/loss) event. **Gift and donation are NOT realization events** (TP10). | Notice 2014-21 A-1/A-6; §1001(a)/(c); RevProc 2024-28 §3.11. | Settled. |
| TP2 | **Basis** = USD cost + **acquisition** fees. **Disposition** fees/selling expenses **reduce proceeds**. | Pub 551; §1.1012-1(h)(1)/(h)(2)(ii)(A); §1001(b); Pub 544. | Settled. |
| TP3 | Income-received BTC = **ordinary income at FMV-USD on dominion & control**; that FMV = basis; HP starts next day. Tag mining business-vs-hobby. | §61; Notice 2014-21 A-4/A-8/A-9; RevRul 2023-14, 2019-24. | Settled. |
| TP4 | **Holding period:** day after acquisition → disposition day inclusive; >1 yr = LT. | Pub 544; §1222; §1223 (conditional tacking — TP11). | Settled. |
| TP5 | Default **FIFO**; **specific-ID-ready**. | §1.1012-1(j). | Settled. |
| TP6 | **Per-wallet basis from 2025-01-01**, via **path A reconstruction** (default) or **path B Rev. Proc. 2024-28 safe harbor** (guarded, irrevocable). | RevProc 2024-28 §§3.11/4.01/4.02/5.02; §1.1012-1(j). | Settled (mechanics); deadlines date-sensitive — §7.4. |
| TP7 | **Self-transfers** non-taxable; lots carry basis + HP. | §1001; RevProc 2024-28 §3.11; §1.1012-1(j). | Settled. |
| TP8 | **Self-transfer network fee — DEFAULT (c):** `fee_sat` consumed at zero proceeds (non-taxable); full basis carries. **Config (b):** taxable **mini-disposition** of fee-sats. Extended by analogy to gift/donation network fees (§7.3). | Contrary signal (taxable-exchange only): §1.1012-1(h)(2)/(h)(4). | **Limited guidance.** Default (c) **user-mandated**; do not change default. |
| TP9 | **Wash-sale does not apply** to crypto. | §1091; crypto is property. | Pending legislation — out of scope. |
| TP10 | **Gift out / donation = non-recognition removal:** zero gain/loss; capture per-lot **basis + FMV-at-transfer + ST/LT** (+ donation appraisal flag) for Phase-2. | §1015; §170(e)(1)/(f)(11)(C); CCA 202302012; RevProc 2024-28 §3.11. | Settled (non-recognition). |
| TP11 | **Received-gift dual basis (§1015(a)) + conditional tacking (§1223(2)):** gain-basis = donor carryover (HP tacks); if FMV-at-gift < donor basis, **loss-basis = FMV-at-gift**, **HP from gift date**; sale between = no gain/loss; FMV-at-gift ≥ donor basis → single carryover (no dual basis, HP tacks). **Unknown donor basis →** fallback = FMV **at the donor's acquisition date** (price dataset, when `donor_acquired_at` known; §1.1015-1(a)(3)); if that date is also unknown, **flag for user input** (not `fmv_at_gift`). | §1015(a); §1.1015-1(a)(1)-(3); §1223(2); addendum Q4. | Settled; unknown-basis fallback is a pragmatic simplification — §1.1015-1(a)(3) is in form an IRS-*determination* mechanism (flagged). |

## 3. Functional requirements
- **FR1 Import.** Accept one+ files; detect source; group multi-file sources (Swan); parse; normalize; assign `source_ref`; validate; **append the batch atomically**. Re-importing unchanged rows is idempotent; a changed row (same `source_ref`, different fingerprint) appends an **`ImportConflict` event** (distinct identity, §6.2) — a blocker resolved by `SupersedeImport` (accept) or `RejectImport` (keep original); re-importing the identical changed row reproduces the same conflict (idempotent), never silent overwrite/drop.
- **FR2 BTC-only filter.** Drop a row **only if it has no BTC leg**. Any BTC leg is retained (a crypto↔BTC trade = BTC disposition/acquisition at FMV; non-BTC leg ignored). **Unknown/ambiguous BTC-side rows → `Unclassified` (blocker), never dropped.** Report dropped (no-BTC) + unclassified counts per file.
- **FR3 FMV resolution.** Prefer export USD; else dataset; else `Missing` (blocker). `Missing` blocks both the income amount and any downstream disposal/removal of the affected lot (§7.3).
- **FR4 Projection.** Deterministically derive, rebuilt from scratch: per-wallet lots, holdings (optionally as-of), `Disposal`s (Sell/Spend: proceeds/basis/gain + ST/LT), `Removal`s (Gift/Donation: basis + FMV + ST/LT, zero gain), recognized income, the reconciliation queue, and all blockers.
- **FR5 Wallets.** Exchange accounts + self-custody wallets as basis pools; create/label self-custody wallets.
- **FR6 Reconciliation.** Propose matches for unclassified outflows ↔ inflows/known wallets (amount±fee, time window, address, **txid match signal**); user confirms `TransferLink` / `ReclassifyOutflow`, classifies inbounds (`ClassifyInbound`), and accepts/rejects `ImportConflict`s. Unmatched outflows + unknown-basis inbounds remain flagged.
- **FR7 2025 basis transition.** Provide `reconstruct-2025` (path A; **default — no election event**; engine reconstructs actual per-wallet positions from history) and `allocate-2025` (path B; emits the `SafeHarborAllocation` election). **Path B governs iff an *effective* `SafeHarborAllocation` exists** (passes the §7.4 guards incl. the time-bar/attestation rule); otherwise path A.
- **FR8 Corrections.** `VoidDecisionEvent` revokes a *revocable* decision; voiding an **effective `SafeHarborAllocation`** (irrevocable) or a non-revocable decision (`SupersedeImport`/`RejectImport`/`VoidDecisionEvent`) → `decision_conflicts` (an inert/time-barred allocation IS voidable). `ImportConflict`s resolve only via `SupersedeImport`/`RejectImport`.
- **FR9 Integrity (`verify`).** Per-wallet holdings vs source running balances (Gemini, Swan); **sat conservation (when no `uncovered_disposal`):** `Σ in == Σ disposed(Sell/Spend) + Σ removed(Gift/Donation) + Σ held + Σ on-chain-fee-sats + Σ pending-reconciliation`, where **`Σ in` counts only externally-sourced acquisitions** (`Acquire`, `Income`, classified `GiftReceived`) — excluding unclassified/unlinked inbounds and self-transfer destination `TransferIn`s (a `TransferLink` relocates lots, it does not create sats) — and **`Σ on-chain-fee-sats` is the sole conservation home for network-fee sats** (a config-(b) mini-disposition adds a *recognition* record, not a second conservation entry). Report drift + all blockers + the pre-2025 filed-method note.
- **FR10 Export.** `export-snapshot` writes the decrypted ledger (SQLite + CSV) — the sole explicit exception to NFR2. `backup-key` exports the passphrase-protected key.

## 4. Non-functional requirements
- **NFR1 Local & offline.** **NFR2 Encryption at rest:** only the PGP vault is written automatically; no plaintext DB except the explicit `export-snapshot`. **NFR3 Durability:** atomic write + rolling backup. **NFR4 Determinism:** identical event *set* → identical ledger, invariant to storage/load order (with each event's `(source_ref|decision_seq, payload)` fixed), including resolution of all decision/correction events. **NFR5 Exact arithmetic.** **NFR6 Auditability:** all state — including conflicts and the safe-harbor attestation — lives as events; the log is the sole source of truth. **NFR7 Single-user safety.**

## 5. Architecture
**Event-sourced core.** Append-only **event log = single source of truth**; all state is a **pure deterministic projection** re-derived from scratch (no caching in Phase 1).

**Cargo workspace** (`MIT OR Unlicense`): `btctax-core` (domain + `PriceProvider` trait + projection; pure, no I/O), `btctax-adapters` (parsers + bundled price dataset), `btctax-store` (PGP-blob ⇄ in-memory SQLite, key/session), `btctax-cli`. (Future: `btctax-forms`, `btctax-optimizer`.)

**Data flow:** `files → adapter(detect→group→preamble→parse→normalize→source_ref→FMV) → atomic append → encrypted event log → pure projection(resolve decisions → effective timeline → canonical fold) → holdings / disposals / removals / income / reconciliation queue / blockers → CLI / verify / export`.

## 6. Domain model
### 6.1 Money & time
- **BTC** = integer **satoshis** (`i64`). **USD** = `rust_decimal::Decimal`. **Rounding** = `ROUND_HALF_EVEN` (`domain::conventions`).
- Time UTC + `original_tz`. **Holding-period day-count uses the calendar date in `original_tz`** (authoritative, TP4).

### 6.2 Identity, dedup & ordering
- **`source_ref`** = stable real-world-row identity scoped by `(source, direction)`. Native id where present (Coinbase `ID`, Gemini `Trade ID`, Swan `Transaction ID`); txid for within-source dedup + cross-source **match signal — NOT a global dedup key**. Id-less sources (River): `(source, direction, utc_ms, type, sat)` + last-resort `occurrence_index` (fragility in FOLLOWUPS).
- **`EventId`** (universal reference target):
  - **Imported events:** `f(source, source_ref)` — survives cosmetic re-exports/corrections.
  - **`ImportConflict` (system-generated):** `f("conflict", source, source_ref, new_fingerprint)` — **distinct from its target's `EventId`**; re-importing the identical changed row reproduces the same conflict `EventId` (idempotent); a different change to the same target forms a separate conflict. It carries the target's `source_ref` but is **not folded in pass 2** (consumed only as a blocker, §7.3).
  - **Decision events (app-generated):** persisted monotonic **`decision_seq: u64`**; `EventId = f("decision", decision_seq)`.
- **Canonical content encoding** (fixed field order; `Decimal` fixed scale; explicit optional/timestamp) → the **content fingerprint** used only for conflict detection (§9).
- **`LotId` = (origin_event_id: EventId, split_sequence: u32)**. `origin_event_id` = the `EventId` of the event that created the underlying holding: `Acquire`/`Income` → that event; `GiftReceived` → the inbound `TransferIn` event (the `ClassifyInbound` decision sets treatment, not identity); **Path-B `SafeHarborAllocation`-seeded lots → the allocation's `EventId`**; Path-A relocates existing lots (origin unchanged). `split_sequence` assigned deterministically as lots split; stable because `EventId` is stable. Path-B allocation-seeded lots are fresh (not splits): their `split_sequence` = the index into `SafeHarborAllocation.lots` (deterministic; fixed payload order).
- **Total order:** decisions resolved in `decision_seq` order (pass 1, §7.2); imported effective events fold in canonical order — `utc_timestamp` → fixed source priority (Swan>Coinbase>Gemini>River; arbitrary-but-stable) → `source_ref` (pass 2).

### 6.3 Entities
- **`Wallet`** (basis pool): `Exchange{provider, account}` | `SelfCustody{label}`.
- **`LedgerEvent`** (immutable): `EventId`, `utc_timestamp` (decisions: creation time), `original_tz`, `source` (or `Decision`/`System`), `source_ref` (or `decision_seq`), `wallet`, `payload`.
- **`Lot`** (derived): `lot_id`, `wallet`, `acquired_at`, `original_sat`, `remaining_sat`, `usd_basis` (gain basis), `basis_source`, and **for received gifts** `dual_loss_basis: Option<Decimal>` + `donor_acquired_at: Option`. On split, **basis fields (`usd_basis`, `dual_loss_basis`) split pro-rata (`ROUND_HALF_EVEN`)**; `donor_acquired_at`/`acquired_at` are dates and do not split.
- **`Disposal`** (Sell/Spend): consumed lots w/ per-lot proceeds/basis/gain + ST/LT + `basis_source`; for a dual-basis gift lot records which zone (gain/loss/none) + resulting HP (TP11).
- **`Removal`** (Gift/Donation): consumed lots w/ per-lot **basis** + FMV-at-transfer + ST/LT + donor/appraisal metadata; **zero recognized gain/loss**.

### 6.4 Event taxonomy
**Imported events (from adapters):**
- `Acquire { sat, usd_cost, fee_usd, basis_source }`.
- `Income { sat, usd_fmv: Option<Decimal>, fmv_status, kind: Mining|Staking|Interest|Airdrop|Reward, business: bool }`.
- `Dispose { sat, usd_proceeds, fee_usd, kind: Sell|Spend }`.
- `TransferOut { sat, fee_sat?, dest_addr?, txid? }` / `TransferIn { sat, src_addr?, txid? }`.
- `Unclassified { raw }` — non-mappable BTC-side row; inert blocker.

**System-generated events:**
- `ImportConflict { target: EventId, new_payload, new_fingerprint }` (identity per §6.2); a blocker until resolved.

**Outflow-classification payloads** (produced by `ReclassifyOutflow`, not adapter-emitted): `GiftOut { sat, basis(from lots), fmv_at_transfer, fee_sat? }`, `Donate { …, appraisal_required: bool }`.

**Decision events (app-generated; `decision_seq`/`EventId` per §6.2):**
- `TransferLink { out_event, in_event_or_wallet }` — confirms a non-taxable self-transfer; **consumes the destination `TransferIn`** (removes it from `unknown_basis_inbounds`).
- `ReclassifyOutflow { transfer_out_event, as: Dispose{Sell|Spend} | GiftOut | Donate, principal_proceeds_or_fmv, fee_usd? }` — fee per TP8.
- `ClassifyInbound { transfer_in_event, as: Income{kind,fmv,business} | GiftReceived{donor_basis: Option<Decimal>, donor_acquired_at: Option, fmv_at_gift} }`.
- `ManualFmv { event, usd_fmv }`.
- `SafeHarborAllocation { lots: [{wallet, sat, usd_basis, acquired_at}], as_of_date: 2025-01-01, method: ActualPosition|ProRata, timely_allocation_attested: bool }` — lots with dates; conservation-checked. The allocation's **made-date is the event's `utc_timestamp`** (creation time), distinct from `as_of_date` (the fixed 2025-01-01 snapshot the basis is allocated *as of*). `timely_allocation_attested` persists the user's attestation that they established a valid/timely allocation in their own books **before the §5.02(4) deadline** (covering both the first-2025-disposition and return-due-date prongs, §7.4). Irrevocable once **effective** (§7.4).
- `SupersedeImport { conflict_event }` — accepts an `ImportConflict` (applies `new_payload` to the target, keeping the **target's** `EventId`).
- `RejectImport { conflict_event }` — keeps the original; clears the blocker.
- `VoidDecisionEvent { target_event_id }` — revokes a *revocable* decision. **Not revocable:** an **effective** `SafeHarborAllocation` (§7.4), and `SupersedeImport`/`RejectImport`/`VoidDecisionEvent` themselves. Voiding a non-revocable target → `decision_conflicts` (no effect on the projection).
- `ClassifyRaw { target: Unclassified_event, as: <imported payload> }` — resolves an `Unclassified` row; **preserves the target's `EventId`** (so a later `ManualFmv` can target it).

- **`FmvStatus`** = `ExchangeProvided | PriceDataset | ManualEntry | Missing`.
- **`BasisSource`** = `ExchangeProvided | ComputedFromCost | FmvAtIncome | CarriedFromTransfer | GiftCarryover | GiftFmvFallback | SafeHarborAllocated | ReconstructedPerWallet`. Propagated `Lot → Disposal/Removal`.

## 7. Projection / lot engine
### 7.1 Contract
`project(events) -> LedgerState { lots, holdings_by_wallet, disposals, removals, income_recognized, pending_reconciliation, blockers }` — pure, deterministic, no I/O, **total** (never panics). `blockers` carry a severity: **hard** (gate downstream tax computation for the affected lots/period) = `fmv_missing`, `uncovered_disposal`, `import_conflicts`, `decision_conflicts`, `unknown_basis_inbounds`, `unclassified`; **advisory** (ledger still usable; a valid fallback or info note) = `safe_harbor_timebar` (Path A is a valid election), `unmatched_outflows` (lots sit in `pending_reconciliation`), `pre2025_method_note`.

### 7.2 Two-pass model (determinism)
- **Pass 1 — resolve decisions onto the imported timeline (staged, deterministic):**
  1. **Non-allocation decisions.** Build the applied set by removing any decision targeted by a `VoidDecisionEvent`, **except** a `SafeHarborAllocation` (deferred to step 4) and non-revocable targets (those Voids → `decision_conflicts`). Apply the remaining **non-`SafeHarborAllocation`** decisions in `decision_seq` order: `SupersedeImport`/`RejectImport` resolve `ImportConflict`s (multiple targeting conflicts of the *same* import event → latest `decision_seq` governs the payload); `ClassifyRaw`→real payload; `TransferLink`/`ReclassifyOutflow`/`ClassifyInbound` set the **effective treatment** of their target imported events; contradictory decisions on one target → `decision_conflicts`.
  2. **Build the effective imported timeline** and determine the **first 2025 BTC Sell/Spend/Gift/Donation/§3.11-transfer** (the made-date reference for the time-bar, §7.4).
  3. **Evaluate each `SafeHarborAllocation`'s effectiveness** (time-bar vs its made-date + attestation + conservation, §7.4).
  4. **Adjudicate allocation-targeting `VoidDecisionEvent`s:** effective allocation (irrevocable) → reject the Void (`decision_conflicts`); inert allocation → the Void applies.
  Output: an **effective imported timeline** + the resolved 2025-transition mode. (A 2026 decision thus correctly rewrites a 2022 event's effective treatment and folds at the 2022 date. **Deterministic consequence:** a `ReclassifyOutflow` that moves a 2025 outflow off `Dispose` can flip an allocation inert→effective, freezing a previously-voided allocation — intended and deterministic.)
- **Pass 2 — fold the effective timeline in canonical order** (§6.2) → lots/disposals/removals/income/holdings.
- **Determinism test:** any storage/load permutation with each event's `(source_ref|decision_seq, payload)` held fixed → identical `LedgerState`.

### 7.3 Fold rules (every variant has a rule)
- `Acquire` → new lot (basis = cost + fee).
- `Income` → if FMV known, new lot at FMV + record income; if `Missing`, record lot + `fmv_missing` and **block both** the income amount and any disposal/removal consuming a lot whose history includes it.
- `Dispose{Sell|Spend}` → consume from wallet pool (FIFO; spec-ID-ready). **Dual-basis gift lot (TP11):** if `dual_loss_basis = None` → single carryover basis, HP tacks; else proceeds > gain-basis → gain on carryover, HP tacks; proceeds < `dual_loss_basis` → loss on FMV basis, HP from gift date; between → zero gain/loss. Emit `Disposal`.
- `GiftOut`/`Donate` → consume lots → `Removal` (per-lot basis + FMV + ST/LT + metadata); **zero recognized gain** (TP10). The on-chain `fee_sat` is consumed per **TP8** — default (c) zero-proceeds; (TP8 is scoped to self-transfers and **extended by analogy** to gift/donation network fees, limited guidance). Fee-sats count **only** in the FR9 `Σ on-chain-fee-sats` term; config (b) adds a *recognition* record (mini-disposition gain/loss) without a second conservation entry.
- `TransferOut` (unclassified) → lots leave into `pending_reconciliation`.
- `TransferLink` → move exact lots carrying basis + `acquired_at`; TP8 (c) default / (b) config (same fee-sats conservation rule as above).
- `ClassifyInbound` → `Income` lot (FMV basis) or `GiftReceived` lot (gain-basis = donor carryover; `dual_loss_basis` when FMV-at-gift < donor basis; unknown donor basis → FMV-at-`donor_acquired_at` fallback (`GiftFmvFallback`); if `donor_acquired_at` is also unknown, **still create the sat-bearing lot with basis pending** and raise `unknown_basis_inbounds` — symmetric with Income-`Missing`: the lot exists so sat-conservation holds; only its basis and derived gain are gated — TP11).
- `ReclassifyOutflow` → fold the target outflow as the chosen `Dispose{Sell|Spend}` / `GiftOut` / `Donate` with supplied proceeds/FMV; fee per TP8.
- `ImportConflict` → `import_conflicts` blocker until `SupersedeImport` (replace target payload, same target `EventId`) or `RejectImport` (keep original) resolves it; with multiple conflicts on one target, the latest-`decision_seq` Supersede/Reject governs the payload (§7.2, precedence — not a `decision_conflicts`).
- `ManualFmv` → set FMV (`ManualEntry`) on the target; clears its `fmv_missing`.
- `ClassifyRaw` → replace an `Unclassified` with the supplied payload (target `EventId` preserved).
- `VoidDecisionEvent` → (handled in pass 1, staged §7.2) drops a revocable decision; targeting an **effective** `SafeHarborAllocation` or any non-revocable decision → `decision_conflicts`; two un-voided conflicting decisions on one target → `decision_conflicts`.
- **Totality:** any `Dispose`/`Removal`/`TransferOut`/`SupersedeImport` that cannot be covered by available lots (empty/insufficient pool; sats still `pending`; quantity corrected below already-consumed) → **`uncovered_disposal` blocker** with the shortfall; **never panic, never negative remainder.**

### 7.4 2025 basis transition (TP6) — guarded, with fallback
- Pre-2025: **`UniversalPool`** tracking **lots (with dates), un-partitioned by wallet**; pre-2025 disposals consume **FIFO** (legal default; deviation from the taxpayer's filed method → `pre2025_method_note`).
- At 2025-01-01, seed `PerWalletPool` via:
  - **Path A — actual per-wallet reconstruction (default; no election event):** assign each remaining lot to the wallet that holds it at 2025-01-01 (from reconciled history), preserving `acquired_at` + basis (`ReconstructedPerWallet`).
  - **Path B — Rev. Proc. 2024-28 safe harbor (`SafeHarborAllocation`):** allocate remaining **lots** (with dates) to wallets (`ActualPosition` preferred; `ProRata` permitted).
- **Effectiveness & the time-bar (a *projection* rule — re-evaluated deterministically on every rebuild, so a later-imported 2025 disposition automatically re-tests it):** a `SafeHarborAllocation` is **effective** iff it passes all guards; otherwise it is **inert** → projection uses **Path A** and raises the advisory `safe_harbor_timebar`. Guards:
  - **(1) Deadline — compared against the allocation's *made-date* (= its event `utc_timestamp`), NOT `as_of_date`.** Per RevProc §5.02(4) a specific-unit allocation must be established before the earlier of **(a)** the **first 2025 BTC Sell, Spend, Gift, Donation, or §3.11 transfer-to-another-taxpayer** (per §3.11 a confirmed own-wallet **self-transfer is NOT a "transfer"** and does not trip this prong), or **(b)** the 2025 return due date **including extension**. The engine fires the time-bar when the effective timeline shows such a first-2025 event dated **earlier than the allocation's made-date** (prong a), **or** the made-date is after the **unextended** 2025 return due date the app knows (prong b) — **unless `timely_allocation_attested == true`** (the app cannot see the user's own books or whether an extension was filed; the persisted, auditable attestation satisfies both prongs). The global-method *description* must predate 2025-01-01 (§5.02(5)(a)); completed global allocations are due before the later of §5.02(4)(a)/(b) (§5.02(5)(b)).
  - **(2) Irrevocable** once effective (a Void targeting it → `decision_conflicts`, §7.2).
  - **(3) Conservation** — Σsat == remaining held sat; Σbasis == remaining pool basis as of 2025-01-01.
  - **(4) Capital-asset eligibility (§4.02(1)-(2))** assumed for a personal investor; flagged if dealer.
- 2025+: `PerWalletPool` — FIFO/spec-ID within each wallet; self-transfers carry lots between pools.
- **Pending-at-snapshot:** sats still in `pending_reconciliation` at 2025-01-01 are excluded from allocation (flagged); they enter a pool once reconciled.

## 8. Encrypted storage & session (`btctax-store`)
- **On disk:** one `vault.pgp` (Sequoia-PGP; private key passphrase-protected with the **strongest available S2K** — Argon2 if supported, else high-work-factor iterated-salted; FOLLOWUPS). Decrypted layout: `[schema_version:u32][SQLite serialized image]`.
- **Open:** `flock(LOCK_EX|LOCK_NB)` → fail fast (NFR7); reap orphan `vault.pgp.tmp`; decrypt → `mlock` (best-effort; warn on failure; doesn't fully cover SQLite heap/`Decimal`/`String` — R1); `deserialize` into in-memory SQLite.
- **Save:** serialize → prepend version → encrypt → `vault.pgp.tmp` → `fsync` → atomic `rename()`; rotate prior to `vault.pgp.bak`.
- **Migration:** `migrate(version, …)` spans outer layout + SQLite DDL + event-payload serde.
- **Session:** unlock once; key+DB in `mlock`ed, `zeroize`-on-drop memory (best-effort); re-lock on exit/timeout.
- **Key lifecycle:** `init` generates keypair, sets passphrase, **forces a key-backup step**. `export-snapshot` = recovery escape hatch (NFR2 exception).

## 9. Ingestion & adapters (`btctax-adapters`)
**Pipeline:** detect → group (Swan) → strip preamble (CRLF) → parse → normalize → `source_ref` → validate → atomic append. A row with an existing `source_ref` + different fingerprint → append `ImportConflict` (resolve via Supersede/Reject); identical → no-op.

**`Adapter` trait** (`detect`/`group`/`parse`/`normalize`); each module doc states (with a fixture test): `source_ref`/dedup, gross-vs-net proceeds, fee placement, unknown-type → `Unclassified`.

### 9.1 Per-source mapping (real-sample-verified; invariants tested)
- **Coinbase** (yearly CSV; 3-line preamble; native `ID`): `Buy`→`Acquire` (basis = `Total` = `Subtotal`+`Fees`); `Sell`→`Dispose{Sell}` (gross=`Subtotal`, fee=`Fees`); `Send`→`TransferOut`; `Receive`/`Exchange Deposit`/`Pro Deposit`→`TransferIn`; `Withdrawal`/`Exchange/Pro Withdrawal`→`TransferOut`; **`Convert` BTC-leg → `Dispose{Sell}`/`Acquire` at FMV**; **`Order` + unknown → `Unclassified`**; reward/income → `Income`. No-BTC rows dropped.
- **Gemini** (XLSX ledger; `Tx Hash`/`Trade ID`; `BTC Balance` for reconciliation): `Buy`→`Acquire`, `Sell`→`Dispose{Sell}` (`USD Amount`=gross, `Fee (USD)` separate); `Debit`(BTC)→`TransferOut`; **`Credit`(BTC) → `Unclassified`**; `Credit`(USD) = cash deposit.
- **River** (universal CSV; CRLF; semantic `source_ref`): `Buy`→`Acquire` (basis=`Sent`+`Fee`); `Income`/`Interest`→`Income{kind}` (no USD → dataset FMV); BTC-sent/`Withdrawal`→`TransferOut`.
- **Swan** (3 files = one batch; `Transaction ID`/txid within Swan): `trades`→`Acquire`; `transfers` (carries `USD Cost Basis`+`Acquisition Date`) → reconcilable `TransferIn` matched to a source-venue outflow (source lot authoritative; Swan basis only for externally-sourced coins); `withdrawals`→`TransferOut`.

### 9.2 Price dataset
Bundled daily BTC/USD behind `PriceProvider` (trait in core). **Daily close** = documented FMV convention (approximates the dominion-and-control date/time standard; FOLLOWUPS M3).

## 10. Reconciliation & decision precedence
Engine surfaces `pending_reconciliation` + `unknown_basis_inbounds`. Reconciler proposes matches (amount±`fee_sat`, time window, address, **txid match signal**); user confirms `TransferLink`/`ReclassifyOutflow`/`ClassifyInbound` and accepts/rejects `ImportConflict`s. **Precedence:** append-only, resolved in `decision_seq` order; `VoidDecisionEvent` first; two un-voided conflicting decisions → `decision_conflicts`.

## 11. CLI (`btctax-cli`)
`init` · `import <files…>` (auto-groups Swan) · `reconcile` (resolves unmatched transfers, `Unclassified`, `ImportConflict`s, inbound classification; prompts the full typed payload) · `wallets` · `holdings [--at DATE]` · `lots [--wallet]` · `events [--filter]` · `fmv` · `reconstruct-2025` · `allocate-2025` · `verify` · `export-snapshot` · `backup-key`. All behind the session/unlock; mutating commands atomic-save.

## 12. Error handling & integrity
Typed errors (`thiserror`) with file/row/column context; parse failure aborts the batch. **Nothing silent:** all blockers surfaced (§7.1). `verify` runs §FR9. Save atomic; lock contention → clear message; orphan `.tmp` reaped on open.

## 13. Testing & acceptance ("green" = full suite passes + 0 Critical/0 Important)
TDD. Required:
- **Per-adapter** real-fixture tests: `source_ref`, gross/net+fee, preamble/CRLF, no-BTC drop count, type mappings, Convert BTC-leg retention, Gemini BTC-Credit→Unclassified, unknown→Unclassified, Swan 3-file dedup + transfer-in non-double-count.
- **Known-answer tax tests:** buy→1yr+1day→sell=LT; same-day=ST; self-transfer fee under (c) **and** (b); income lot w/ FMV; **gift/donation outbound = zero gain** w/ correct basis/FMV/ST-LT; **received-gift dual-basis — all four cases** (no dual basis; gain w/ tack; loss w/ HP-from-gift-date; middle zero); 2025 transition paths A & B w/ mixed vintages (post-2025 ST/LT + conservation); **uncovered disposal → blocker**; **safe-harbor time-bar** — (i) unattested allocation whose **made-date** is after a first 2025 disposition → inert/Path A + `safe_harbor_timebar`; (ii) unattested made-date after the unextended 2025 return due date → inert; (iii) `timely_allocation_attested` → Path B; (iv) a confirmed self-transfer dated before the made-date does **not** trip the bar.
- **Determinism & corrections:** a late `ReclassifyOutflow`/`SupersedeImport` retroactively rewriting an **earlier tax year's** lot math (+ deterministic); `decision_conflicts` (two un-voided conflicting decisions → blocker); **voiding an effective `SafeHarborAllocation` → `decision_conflicts`** (not dropped); a `ReclassifyOutflow` flipping an allocation **inert→effective** is deterministic; a **second distinct** change to the same import target → a separate `ImportConflict` (latest-`decision_seq` Supersede governs); `Void` round-trip determinism; storage/load permutation with `(decision_seq|source_ref, payload)` fixed → identical state.
- **Property tests:** conservation `Σ in == disposed + removed + held + fee-sats + pending` **(conditioned on no `uncovered_disposal`)**; no negative remainders; Σlot-basis == pool-basis.
- **Idempotency:** re-import incl. cosmetic variation (whitespace, `Decimal` scale, CRLF) → no dupes; a changed row → **one** `ImportConflict`; re-importing the identical changed row → no new conflict; accept/reject deterministic.
- **Storage:** atomic-save/crash, concurrency refusal, encryption round-trip + wrong-passphrase + mlock-warn, migration identity.
- **Golden end-to-end** over the real sample set (pinned holdings/disposals/removals/income) — guarded by the KATs.

## 14. Risks & assumptions
- **R1 mlock/zeroize best-effort** (don't fully cover SQLite heap/`Decimal`/`String`); defense-in-depth; docs recommend encrypted/disabled swap.
- **R2 Adapter semantics to confirm by fixture test:** Coinbase `Order`/`Convert`/reward; Gemini `Credit`; River CRLF + Income/Interest shape. Unresolved → `Unclassified`.
- **R3** Pin a Sequoia-PGP version + S2K (Argon2 if available) before first build.
- **A1** Past tax years filed (no historical forms; pre-2025 FIFO unless `verify` says otherwise).
- **A2** Four sources are the current venue set; externally-sourced inbounds need `ClassifyInbound`.

## 15. Out of scope / future phases
Phase 2 (forms: 8949 + Sch D PDFs; §170(e) deduction; 8283; 709; SE-tax). Phase 3 (optimizer). Non-BTC assets (incl. fork-coin income/dispositions, e.g. 2017 BCH — explicitly out of BTC-only scope), GUI, online pricing, multi-user, §1015(d) gift-tax basis bump. (See FOLLOWUPS.)

## 16. Suggested implementation order (input to the plan)
1. `btctax-store` safety primitives (atomic write+bak, flock, encrypt/decrypt round-trip, mlock+warn, schema_version+migrate) — crash/concurrency/round-trip first.
2. `btctax-core` identity & ordering (money/time conventions, `source_ref`/`EventId` for imports + conflict + `decision_seq`, `LotId` + origin pinning, canonical order + two-pass model + determinism tests, event taxonomy).
3. `btctax-core` projection (two-pass resolve+fold; FIFO; HP + TP11 dual-basis; TP8 (c)+(b); gift/donation non-recognition; FMV gating; totality; pool modes + paths A/B + time-bar) — property + determinism + idempotency + KATs.
4. `btctax-adapters` one source at a time (Swan → Coinbase → Gemini → River), each w/ fixtures + `PriceProvider`.
5. Reconciliation + CLI + `verify` + golden end-to-end.

## 17. (reserved)

## 18. Fold record — reviews → fixes
**R1 (v0.2):** ENG-C1 identity → §6.2; ENG-C2/TAX-C1 gifts → TP1/TP10/§6.3/§6.4/§7.3; ENG-C3/TAX-C2 safe-harbor → §7.4/FR7/FR8; ENG-C4 crypto-to-crypto → FR2/§9.1; ENG-I1 purity → §6.2/§9; ENG-I2 pre-2025 → §7.4; ENG-I3 reclassify → §6.4; ENG-I5 decision conflict → §7.3/§10; ENG-I6 HP date → §6.1; ENG-I7 NFR2/export → NFR2/FR10; ENG-I8 FMV gating → FR3/§7.3; ENG-I9 KATs → §13; TAX-I1 inbound → ClassifyInbound; TAX-I2 Gemini Credit → §9.1; TAX-I3 income/Convert → FR2/§9.1; TAX-I4 txid → §6.2; TAX-I5 Swan → §9.1.
**R2 (v0.3):** ENG-C2-1 decision identity + two-pass → §6.2/§7.2/NFR4; ENG-I2-1 import-conflict + reject → §6.4/§7.3/FR1/FR8; ENG-I2-2 & TAX-New-I1 conservation removed → FR9/§13; ENG-I2-3 uncovered totality → §7.1/§7.3/§13; ENG-I2-4 guard trigger → §7.4; ENG-I2-5 & TAX-New-I2 dual-basis → TP11/§6.3/§7.3/§13; + minors.
**R3 (v0.4):** ENG-I-NEW-1 ImportConflict identity collision → §6.2 (`f("conflict",…)`) + §6.4 (system-generated). ENG-I-NEW-2 safe-harbor override contradiction → FR7/FR8/§7.1 (`safe_harbor_timebar`)/§7.4 (effective-allocation + projection-level time-bar + persisted `pre_disposition_attested`). ENG-m1 Void ordering → §7.2. ENG-m2 determinism-test wording → §7.2/§13/NFR4. ENG-m3 origin_event_id for non-Acquire lots → §6.2. ENG-m4 `Σ in` definition → FR9. ENG-m5 conservation precondition → §13. ENG-m6 & TAX-N1 dual_loss_basis None branch → §7.3. ENG-m7 testability → §13. ENG-n1 ClassifyRaw EventId → §6.4. ENG-n2 dual_loss_basis split → §6.3. ENG-n3 "effective" defined → §7.4. ENG-n4 & TAX-M3 TP8-on-gift-fee note → TP8/§7.3. TAX-M1 unknown-basis FMV date → TP11. TAX-M2 fee double-count → FR9/§7.3. TAX-M4 global-alloc deadline wording → §7.4. TAX-N2 appraisal trigger precision → FOLLOWUPS. TAX-N3 §1015(d) → §15/FOLLOWUPS.
**R4 (v0.5):** ENG-IMPORTANT-1 & TAX-R4-M2 time-bar compares wrong date → §6.4 (`effective_date`→`as_of_date`; made-date = event `utc_timestamp`) + §7.4 (compare made-date) + §13. ENG-IMPORTANT-2 Void-vs-irrevocability circularity → §7.2 (staged pass-1) + §7.3/FR8 (void-of-effective → `decision_conflicts`). ENG-MINOR-1 multiple conflicts/target → §7.2/§7.3 (latest `decision_seq`). ENG-MINOR-2 return-due-date prong → §7.4. ENG-MINOR-3 unknown-basis gift still creates lot → §7.3. ENG-MINOR-4 blocker hard/advisory severity → §7.1. ENG-n1 void-of-void/supersede not revocable → §6.4/FR8. ENG-n2 ImportConflict not folded → §6.2. ENG-n3 allocation-seeded `split_sequence` = lots index → §6.2. ENG-n4/N4 "effective" overload → §6.4 (`as_of_date` rename). TAX-R4-M1 §3.11 transfer cite + self-transfer excluded from trigger → §7.4. TAX-R4-N1 §1.1015-1(a)(3) IRS-determination note → TP11.
