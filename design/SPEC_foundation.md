# SPEC — bitcoin_tax (TaxApp), Phase 1: Foundation (v0.3)

- **Status:** DRAFT v0.3 — folds round-1 (architecture+tax+engineering) and round-2 (tax+engineering) independent reviews. Pending round-3 re-review to green (0 Critical / 0 Important) before the plan gate.
- **History:** v0.1 (initial) → v0.2 (round-1 fold) → v0.3 (round-2 fold). Superseded specs preserved in git history; all reviews in `reviews/`.
- **Date:** 2026-06-28
- **Phase:** 1 of 3 — **foundation only**: ingestion → canonical event ledger → lot engine → transfer reconciliation, on an encrypted store. Forms (Phase 2) and the goal-driven optimizer (Phase 3) get their own spec→plan→implement cycles.

## 0. References (verified at write time, against the local archive)
- Legal report + verified addendum: `legal/research/REPORT_us_btc_tax_TY2025-2026.md`, `legal/research/ADDENDUM_open_questions_verified.md`.
- Primary sources + manifest: `legal/SOURCES.md` (47 docs; `legal/SHA256SUMS` 47/47); grep-able text in `legal/text/`.
- Reviews folded: `reviews/architecture-review-phase1-foundation-round-1.md`, `reviews/spec-review-phase1-{engineering,tax}-round-1.md`, `reviews/spec-review-phase1-{engineering,tax}-round-2.md`. Fold map: §18.
- Deferred items: `FOLLOWUPS.md`.

## 1. Purpose, scope, non-goals
**Purpose.** A local, offline, single-user desktop app (CLI first) that reconstructs a complete, auditable **per-lot Bitcoin ledger** across all venues + self-custody, so later phases can produce correct tax forms and tax-optimal sell recommendations.

**In scope (Phase 1).** Ingest Coinbase, Gemini, River, Swan exports; normalize to an append-only canonical event log; derive a per-wallet lot ledger as a deterministic projection; model self-custody wallets + assisted transfer reconciliation; reconstruct full history for basis; handle the Jan-1-2025 per-wallet transition; PGP-encrypted vault; a CLI to import/reconcile/inspect/resolve/verify/export.

**Non-goals (Phase 1).** Form generation; the optimizer; non-BTC assets; multi-user; networked operation; GUI.

## 2. Tax positions (TP) — each cited to the archive; engine encodes them as named, swappable rules
| ID | Position | Archived basis | Uncertainty |
|----|----------|----------------|-------------|
| TP1 | BTC is **property**. A **sale or spend** is a realization (gain/loss) event. **Gift and donation are NOT realization events** (non-recognition removals — TP10). | Notice 2014-21 A-1/A-6; §1001(a)/(c); RevProc 2024-28 §3.11 ("transfer, other than a sale or disposition"). | Settled. |
| TP2 | **Basis** = USD cost + **acquisition** costs/fees. **Disposition** fees/selling expenses **reduce proceeds**. | Pub 551 ("commissions … transfer fees"); §1.1012-1(h)(1)/(h)(2)(ii)(A); §1001(b); Pub 544 ("Minus: Selling expenses"). | Settled. |
| TP3 | Income-received BTC = **ordinary income at FMV-USD on dominion & control**; that FMV = lot basis; holding period starts next day. Tag mining business-vs-hobby (Phase-2 SE-tax). | §61; Notice 2014-21 A-4/A-8/A-9; RevRul 2023-14; RevRul 2019-24. | Settled. |
| TP4 | **Holding period:** starts the day after acquisition, includes disposition day; >1 yr = long-term. | Pub 544 (worked example); §1222; §1223 (tacking, conditional — TP11). | Settled. |
| TP5 | Default **FIFO**; engine **specific-ID-ready**. | §1.1012-1(j). | Settled. |
| TP6 | **Per-wallet basis from 2025-01-01**, via **path A actual per-wallet reconstruction** (default) or **path B Rev. Proc. 2024-28 safe-harbor allocation** (eligibility/deadline-guarded, irrevocable). | RevProc 2024-28 §§3.11/4.01/4.02/5.02; §1.1012-1(j). | Settled (mechanics); deadlines date-sensitive — §7.4. |
| TP7 | **Self-transfers** (own→own) non-taxable; lots carry basis + holding period. | §1001; RevProc 2024-28 §3.11; §1.1012-1(j). | Settled. |
| TP8 | **Self-transfer network fee — DEFAULT (c):** `fee_sat` consumed at **zero proceeds** (non-taxable transfer cost); full basis carries to arriving sats. **Config (b):** taxable **mini-disposition** of the fee-sats. | Contrary signal (taxable-exchange only): §1.1012-1(h)(2)/(h)(4). | **Limited guidance.** Default (c) **user-mandated**; do not change default. |
| TP9 | **Wash-sale does not apply** to crypto. | §1091 (stock/securities only); crypto is property. | Pending legislation — out of scope. |
| TP10 | **Gift out / charitable donation = non-recognition removal:** lot leaves at **zero gain/loss**; capture per-lot **basis + FMV-at-transfer + ST/LT** (+ donation appraisal flag) for Phase-2 §170(e) deduction and donee carryover. | §1015; §170(e)(1)/(f)(11)(C); CCA 202302012; RevProc 2024-28 §3.11. | Settled (non-recognition); forms = Phase 2. |
| TP11 | **Received-gift dual basis (§1015(a)) + conditional tacking (§1223(2)):** gain-basis = donor carryover (HP tacks); if FMV-at-gift < donor basis, **loss-basis = FMV** and **HP starts at the gift date (no tack)**; sale between the two = no gain/no loss. Unknown donor basis → FMV-basis fallback. | §1015(a); §1.1015-1(a)/(a)(3); §1223(2); addendum Q4. | Settled. |

## 3. Functional requirements
- **FR1 Import.** Accept one+ files; auto-detect source; group multi-file sources (Swan's 3 files) into one batch; parse; normalize; assign `source_ref`; validate; **append the batch atomically** (all-or-nothing on parse/validation failure). Re-importing unchanged rows is idempotent; a changed row (same `source_ref`, different content fingerprint) appends an **`ImportConflict` event** (a blocker) resolved by `SupersedeImport` (accept) or `RejectImport` (keep original) — never silent overwrite/drop (§9, §6.4).
- **FR2 BTC-only filter.** Drop a row **only if it has no BTC leg**. Any BTC leg is retained: a crypto↔BTC trade is a BTC **disposition** (BTC out) or **acquisition** (BTC in) at FMV; its non-BTC leg is ignored. **Unknown/ambiguous BTC-side rows → `Unclassified` (blocker), never dropped.** Report dropped (no-BTC) and unclassified counts per file.
- **FR3 FMV resolution.** Prefer export USD; else bundled dataset; else `Missing` (blocker). `Missing` FMV blocks **both** the income amount and any downstream disposal/removal consuming the affected lot (§7.3).
- **FR4 Projection.** Deterministically derive, rebuilt from scratch: per-wallet lots, holdings (optionally as-of a date), `Disposal`s (Sell/Spend: per-lot proceeds/basis/gain + ST/LT), `Removal`s (Gift/Donation: per-lot basis + FMV + ST/LT, zero gain), recognized ordinary income, the reconciliation queue, and all open blockers.
- **FR5 Wallets.** Exchange accounts + self-custody wallets as first-class basis pools; create/label self-custody wallets.
- **FR6 Reconciliation.** Propose matches for unclassified outflows ↔ inflows/known wallets (amount±fee, time window, address, **txid match signal**); user confirms a self-transfer (`TransferLink`) or reclassifies (`ReclassifyOutflow`). Classify standalone inbounds (`ClassifyInbound`) as income or received-gift. Unmatched outflows **and** unknown-basis inbounds remain flagged.
- **FR7 2025 basis transition.** Provide `reconstruct-2025` (path A; the default — no election event; engine reconstructs actual per-wallet positions from history) and `allocate-2025` (path B; emits the `SafeHarborAllocation` election) — with §7.4 guards. If a `SafeHarborAllocation` exists it governs (path B); otherwise path A.
- **FR8 Corrections.** `VoidDecisionEvent` revokes a *revocable* decision. An **effective `SafeHarborAllocation` is irrevocable** (cannot be voided/re-done). `ImportConflict`s resolve only via `SupersedeImport`/`RejectImport`.
- **FR9 Integrity (`verify`).** Per-wallet holdings vs source running balances (Gemini, Swan); **sat conservation:** `Σ in == Σ disposed(Sell/Spend) + Σ removed(Gift/Donation) + Σ held + Σ on-chain-fee-sats + Σ pending-reconciliation`; report drift, unknown-basis inbounds, FMV blockers, unclassified rows, import conflicts, uncovered disposals, decision conflicts, and the pre-2025 filed-method note.
- **FR10 Export.** `export-snapshot` writes the **decrypted** ledger (SQLite + CSV) — the *sole, explicit, user-invoked* exception to the no-plaintext rule (NFR2). `backup-key` exports the passphrase-protected key.

## 4. Non-functional requirements
- **NFR1 Local & offline** (bundled price data). **NFR2 Encryption at rest:** the only artifact written automatically is the PGP vault; no plaintext DB except the explicit `export-snapshot` (FR10). **NFR3 Durability:** atomic write + rolling backup (§8). **NFR4 Determinism:** identical inputs → identical ledger, independent of import order, **including the resolution of all decision/correction events** (§6.2, §7.2). **NFR5 Exact arithmetic** (no floats). **NFR6 Auditability:** every derived number traces to events; *all* state — including conflicts — lives as events (the log is the sole source of truth). **NFR7 Single-user safety:** concurrent instances cannot silently clobber (§8).

## 5. Architecture
**Event-sourced core.** Append-only **event log = single source of truth**; all ledger state is a **pure deterministic projection** re-derived from scratch (no caching in Phase 1).

**Cargo workspace** (`MIT OR Unlicense`): `btctax-core` (domain + `PriceProvider` trait + projection; pure, no I/O), `btctax-adapters` (per-source parsers + bundled price dataset), `btctax-store` (PGP-blob ⇄ in-memory SQLite, key/session), `btctax-cli`. (Future: `btctax-forms`, `btctax-optimizer`.)

**Data flow:** `files → adapter(detect→group→preamble→parse→normalize→source_ref→FMV) → atomic append → encrypted event log → pure projection(resolve decisions → effective timeline → canonical fold) → holdings / disposals / removals / income / reconciliation queue / blockers → CLI / verify / export`.

## 6. Domain model
### 6.1 Money & time
- **BTC** = integer **satoshis** (`i64`); never float. **USD** = `rust_decimal::Decimal`. **Rounding** = `ROUND_HALF_EVEN` (`domain::conventions`).
- Time stored UTC + `original_tz`. **Holding-period day-count uses the calendar date in `original_tz`** (the taxpayer's trade date) — authoritative (TP4).

### 6.2 Identity, dedup & ordering
- **`source_ref`** = stable identity of a real-world row, scoped by `(source, direction)` (direction ∈ {in,out,trade,other}). Built from the source's native stable id (Coinbase `ID`, Gemini `Trade ID`, Swan `Transaction ID`); txid is used for within-source cross-file dedup (Swan's 3 files) and as a cross-source reconciliation **match signal — NOT a global dedup key**. For id-less sources (River): canonical `(source, direction, utc_ms, type, sat)`; a last-resort `occurrence_index` disambiguates exact duplicates in one import (fragility documented in FOLLOWUPS; a re-export that edits a constituent field changes the `source_ref` and cannot be auto-superseded — documented limitation).
- **`EventId`** (stable, the universal reference target for `LotId` and all decision targets):
  - **Imported events:** `EventId = f(source, source_ref)` — survives cosmetic re-exports/corrections (a `SupersedeImport` keeps the same `EventId`).
  - **Decision events:** app-generated, so they carry a persisted monotonic **`decision_seq: u64`** assigned at creation; `EventId = f("decision", decision_seq)`. Stable across re-derivation and the basis of the total order among decisions.
- A **canonical content encoding** (fixed field order; `Decimal` normalized to fixed scale; explicit optional/timestamp encoding) defines the **content fingerprint** used only to detect "same `source_ref`, changed content" (§9).
- **`LotId` = (origin_event_id: EventId, split_sequence: u32)**; `split_sequence` assigned deterministically as the projection splits lots. Stable because `EventId` is stable.
- **Total order:** **decisions** are resolved in `decision_seq` order (pass 1, §7.2); **imported effective events** fold in canonical order — `utc_timestamp` → fixed source priority (Swan>Coinbase>Gemini>River; arbitrary-but-stable) → `source_ref` (pass 2). Mandatory determinism tests cover both import-order and decision-order independence.

### 6.3 Entities
- **`Wallet`** (basis pool): `Exchange{provider, account}` | `SelfCustody{label}`. (No `External`.)
- **`LedgerEvent`** (immutable): `EventId`, `utc_timestamp` (decisions use creation time), `original_tz`, `source` (or `Decision`), `source_ref` (or `decision_seq`), `wallet`, `payload`.
- **`Lot`** (derived): `lot_id`, `wallet`, `acquired_at`, `original_sat`, `remaining_sat`, `usd_basis` (gain basis), `basis_source`, and **for received gifts** `dual_loss_basis: Option<Decimal>` (= FMV-at-gift when < donor basis) + `donor_acquired_at: Option` (conditional tacking, TP11). Splits on partial disposal/transfer (basis fields split pro-rata, `ROUND_HALF_EVEN`).
- **`Disposal`** (derived, **Sell/Spend**): consumed lots with per-lot proceeds/basis/gain + ST/LT + `basis_source`; for a dual-basis gift lot, records which basis (gain/loss/none-zone) was used and the resulting HP (TP11).
- **`Removal`** (derived, **Gift/Donation**): consumed lots with per-lot **basis** + FMV-at-transfer + ST/LT + donor/appraisal metadata; **zero recognized gain/loss**.

### 6.4 Event taxonomy
**Imported events (from adapters):**
- `Acquire { sat, usd_cost, fee_usd, basis_source }` — buy/purchased receive (basis = cost + acquisition fee).
- `Income { sat, usd_fmv: Option<Decimal>, fmv_status, kind: Mining|Staking|Interest|Airdrop|Reward, business: bool }` — ordinary income; new lot at FMV.
- `Dispose { sat, usd_proceeds, fee_usd, kind: Sell|Spend }` — realization; net proceeds = usd_proceeds − fee_usd.
- `TransferOut { sat, fee_sat?, dest_addr?, txid? }` / `TransferIn { sat, src_addr?, txid? }` — movement; unclassified until linked/classified.
- `Unclassified { raw }` — a parsed BTC-side row not unambiguously mappable (Coinbase `Order`, ambiguous Gemini BTC `Credit`). Imported, inert, blocker; never guessed/dropped.
- `ImportConflict { target: EventId, new_payload, new_fingerprint }` — appended when a re-import row matches an existing `source_ref` with a different content fingerprint. A blocker until resolved.

**Outflow-classification payloads** (produced by `ReclassifyOutflow`, **not** adapter-emitted): `GiftOut { sat, basis(from lots), fmv_at_transfer, fee_sat? }`, `Donate { …, appraisal_required: bool }`.

**Decision events (app-generated; `decision_seq`/`EventId` per §6.2):**
- `TransferLink { out_event, in_event_or_wallet }` — confirms a non-taxable self-transfer (applies TP8).
- `ReclassifyOutflow { transfer_out_event, as: Dispose{Sell|Spend} | GiftOut | Donate, principal_proceeds_or_fmv, fee_usd? }` — gives a reclassified outflow real proceeds/FMV; the on-chain `fee_sat` is handled per TP8 (c default / b config); for `Dispose{Spend}`, `principal_proceeds_or_fmv` = FMV of goods/services for the principal sats.
- `ClassifyInbound { transfer_in_event, as: Income{kind, fmv, business} | GiftReceived{donor_basis: Option<Decimal>, donor_acquired_at: Option, fmv_at_gift} }` — classifies a standalone inbound (unknown `donor_basis` → FMV-basis fallback, TP11).
- `ManualFmv { event, usd_fmv }`.
- `SafeHarborAllocation { lots: [{wallet, sat, usd_basis, acquired_at}], method: ActualPosition|ProRata, effective_date: 2025-01-01 }` — carries **lots with acquisition dates** (not aggregates); conservation-checked; irrevocable once effective (§7.4).
- `SupersedeImport { conflict_event }` — accepts an `ImportConflict` (applies its `new_payload` to the target, keeping the target's `EventId`).
- `RejectImport { conflict_event }` — rejects an `ImportConflict` (keeps the original payload; clears the blocker).
- `VoidDecisionEvent { target_event_id }` — revokes a *revocable* decision (not an effective `SafeHarborAllocation`).
- `ClassifyRaw { target: Unclassified_event, as: <imported payload> }` — resolves an `Unclassified` row.

- **`FmvStatus`** = `ExchangeProvided | PriceDataset | ManualEntry | Missing`.
- **`BasisSource`** = `ExchangeProvided | ComputedFromCost | FmvAtIncome | CarriedFromTransfer | GiftCarryover | SafeHarborAllocated | ReconstructedPerWallet`. Propagated `Lot → Disposal/Removal`.

## 7. Projection / lot engine
### 7.1 Contract
`project(events) -> LedgerState { lots, holdings_by_wallet, disposals, removals, income_recognized, pending_reconciliation, blockers }` — pure, deterministic, no I/O, **total** (never panics), rebuilt from scratch. `blockers` = `{ fmv_missing, unclassified, import_conflicts, uncovered_disposal, unmatched_outflows, unknown_basis_inbounds, decision_conflicts, pre2025_method_note }`.

### 7.2 Two-pass model (determinism)
- **Pass 1 — resolve decisions onto the imported timeline (in `decision_seq` order):** apply `SupersedeImport`/`RejectImport` to `ImportConflict`s; apply `Void` (drop revoked decisions); apply `ClassifyRaw` (turn `Unclassified`→real payload); apply `TransferLink`/`ReclassifyOutflow`/`ClassifyInbound` to set the **effective treatment** of their target imported events; detect unresolved/contradictory decisions → blockers. Output: an **effective imported timeline**. (This is why a 2026 decision can correctly rewrite a 2022 event's treatment — it mutates the effective timeline, not a 2026 fold slot.)
- **Pass 2 — fold the effective timeline in canonical order** (§6.2) to produce lots/disposals/removals/income/holdings.
- Re-running with shuffled import order **and** shuffled decision-append order yields an identical `LedgerState` (tested).

### 7.3 Fold rules (every variant has a rule)
- `Acquire` → new lot (basis = cost + fee; `basis_source = ComputedFromCost` or `ExchangeProvided`).
- `Income` → if FMV known, new lot at FMV (`FmvAtIncome`) + record ordinary income; if `Missing`, record lot existence, add to `fmv_missing`, and **block both** the income amount and any disposal/removal consuming a lot whose history includes it.
- `Dispose{Sell|Spend}` → consume lots from the wallet pool (default FIFO; spec-ID-ready). **For a `GiftCarryover` (dual-basis) lot apply TP11:** proceeds > gain-basis → gain on carryover basis, HP tacks (`donor_acquired_at`); proceeds < `dual_loss_basis` → loss on FMV basis, HP from gift date; between → zero gain/loss. Emit `Disposal` (net proceeds/basis/gain + ST/LT). Split → new `split_sequence`.
- `GiftOut`/`Donate` → consume lots → `Removal` (per-lot basis + FMV + ST/LT + metadata); **zero recognized gain** (TP10). The on-chain `fee_sat` is consumed per TP8 (default (c): zero proceeds, non-taxable; config (b): mini-disposition) and **counts in the conservation fee-sats term** (FR9).
- `TransferOut` (unclassified) → lots leave into `pending_reconciliation`.
- `TransferLink` → move exact lots to destination carrying basis + `acquired_at`; **TP8 (c) default** (`fee_sat` zero-proceeds) / **(b) config** (fee-sat mini-disposition).
- `ClassifyInbound` → `TransferIn`→`Income` lot (FMV basis) or `GiftReceived` lot (gain-basis = donor carryover or FMV-fallback; `dual_loss_basis` set when FMV<donor basis; `donor_acquired_at` for conditional tacking — TP11).
- `ReclassifyOutflow` → fold the target outflow as the chosen `Dispose{Sell|Spend}` / `GiftOut` / `Donate` with supplied proceeds/FMV; fee per TP8.
- `ImportConflict` → `import_conflicts` blocker until a `SupersedeImport`/`RejectImport` targets it. `SupersedeImport` → replace target payload (same `EventId`). `RejectImport` → keep original; clear blocker.
- `ManualFmv` → set FMV (`ManualEntry`) on the target income/event; clears its `fmv_missing`.
- `ClassifyRaw` → replace an `Unclassified` with the supplied real payload.
- `VoidDecisionEvent` → drop a revocable decision; **two un-voided conflicting decisions on one target → `decision_conflicts` blocker** (no silent "both apply").
- **Totality:** any `Dispose`/`Removal`/`TransferOut`/`SupersedeImport` that cannot be covered by available lots (empty/insufficient pool, sats still `pending`, quantity corrected below already-consumed) → **`uncovered_disposal` blocker** with the shortfall; **never panic, never negative remainder.**

### 7.4 2025 basis transition (TP6) — guarded, with fallback
- Pre-2025: **`UniversalPool`** tracking **lots (with acquisition dates), un-partitioned by wallet**; pre-2025 disposals consume it **FIFO** (legal default; if the taxpayer's *filed* pre-2025 returns used another method → `pre2025_method_note` blocker, not silently assumed).
- At 2025-01-01, seed `PerWalletPool` via:
  - **Path A — actual per-wallet reconstruction (default; most defensible; no election event):** assign each remaining lot to the wallet that actually holds it at 2025-01-01 (from reconciled history), preserving `acquired_at` + basis (`ReconstructedPerWallet`).
  - **Path B — Rev. Proc. 2024-28 safe harbor (`SafeHarborAllocation`):** allocate remaining **lots** (with dates) to wallets (`ActualPosition` preferred; `ProRata` permitted). **Guards:** (1) **eligibility/deadline** — the specific-unit allocation must precede the earlier of **the first 2025 BTC Sell, Spend, Transfer, Gift, or Donation** (any "sale, disposition, or transfer" per §5.02(4)) or **the 2025 return due date including extension**; the global-method variant must predate 2025-01-01. If the ledger shows a 2025 disposition/transfer already occurred before the allocation would be effective, **warn + require explicit user override** (the app can't see the user's own books, which may hold a valid timely allocation) and **default to Path A**. (2) **irrevocable** once effective (FR8). (3) **conservation** — Σsat == remaining held sat; Σbasis == remaining pool basis at 2025-01-01. (4) **capital-asset eligibility (§4.02(1)-(2))** assumed for the personal investor; flagged if dealer.
- 2025+: `PerWalletPool` — FIFO/spec-ID strictly within each wallet; self-transfers carry lots between pools.
- **Pending-at-snapshot:** sats still in `pending_reconciliation` at 2025-01-01 are **excluded from allocation** (neither held nor disposed) and flagged; they enter a wallet pool only once reconciled.

## 8. Encrypted storage & session (`btctax-store`)
- **On disk:** one `vault.pgp` (Sequoia-PGP, encrypted to the app-managed keypair; private key passphrase-protected with the **strongest available S2K** — Argon2 if the pinned Sequoia supports it, else high-work-factor iterated-salted; FOLLOWUPS KDF note). Decrypted layout: `[schema_version:u32][SQLite serialized image]`.
- **Open:** `flock(LOCK_EX|LOCK_NB)` → fail fast if held (NFR7); reap orphan `vault.pgp.tmp`; decrypt → `mlock` plaintext buffer (**best-effort defense-in-depth; warn on failure**; does not fully cover SQLite internal heap / `Decimal`/`String` — R1); `deserialize` into in-memory SQLite.
- **Save:** serialize → prepend `schema_version` → encrypt → write `vault.pgp.tmp` (same dir) → `fsync` → atomic `rename()` over `vault.pgp`, rotating prior to `vault.pgp.bak`.
- **Migration:** `migrate(version, …)` on open; versioning spans the outer layout, the SQLite DDL, and the serde encoding of event payloads (deserialize-and-transform).
- **Session:** unlock once; key + DB in `mlock`ed, `zeroize`-on-drop memory (best-effort); re-lock on exit/timeout.
- **Key lifecycle:** `init` generates the keypair, sets the passphrase, **forces a key-backup step** (loss = unrecoverable). `export-snapshot` is the recovery escape hatch (NFR2 exception).

## 9. Ingestion & adapters (`btctax-adapters`)
**Pipeline:** detect → **group multi-file sources (Swan)** → strip preamble (handle CRLF) → parse → normalize → `source_ref` → validate all → **atomic append**. A row whose `source_ref` exists with a different content fingerprint → append an `ImportConflict` event (resolve via `SupersedeImport`/`RejectImport`); identical content → idempotent no-op.

**`Adapter` trait** (`detect`/`group`/`parse`/`normalize`). Each module doc states, **with a passing test on real fixtures**: `source_ref`/dedup fields, gross-vs-net proceeds, fee placement, and the **unknown-BTC-type → `Unclassified`** rule.

### 9.1 Per-source mapping (from real-sample inspection; tax-relevant invariants tested)
- **Coinbase** (yearly CSV; 3-line preamble; native `ID` = `source_ref`): `Buy`→`Acquire` (basis = `Total` = `Subtotal`+`Fees`); `Sell`→`Dispose{Sell}` (gross = `Subtotal`, fee = `Fees`); `Send`→`TransferOut`; `Receive`/`Exchange Deposit`/`Pro Deposit`→`TransferIn`; `Withdrawal`/`Exchange Withdrawal`/`Pro Withdrawal`→`TransferOut`; **`Convert` BTC-leg → `Dispose{Sell}` (out) or `Acquire` (in) at FMV**; **`Order` + any unrecognized BTC-side type → `Unclassified`**; BTC reward/income types → `Income`. Non-BTC-only rows dropped (FR2).
- **Gemini** (wide XLSX ledger; `Tx Hash`/`Trade ID` for within-source dedup; `BTC Balance` for reconciliation): `Buy`→`Acquire`, `Sell`→`Dispose{Sell}` (`USD Amount`=gross, `Fee (USD)` separate); `Debit`(BTC)→`TransferOut`; **`Credit`(BTC) → `Unclassified`** (income-vs-transfer; never auto-`TransferIn`); `Credit`(USD) = cash deposit (no BTC event).
- **River** (universal CSV; may be CRLF; semantic `source_ref`): `Buy`→`Acquire` (basis = `Sent`+`Fee`; `Sent` excludes fee); `Income`/`Interest`→`Income{kind}` (BTC, no USD → dataset FMV); BTC-sent/`Withdrawal`→`TransferOut`.
- **Swan** (3 files = one batch; `Transaction ID`/txid dedup within Swan): `trades`→`Acquire`; `transfers` (carries `USD Cost Basis`+`Acquisition Date`) → a reconcilable `TransferIn` matched to a tracked source-venue outflow (source lot authoritative + carries; Swan-stated basis used only when no internal match = externally-sourced coins); `withdrawals`→`TransferOut`.

### 9.2 Price dataset
Bundled daily BTC/USD behind `PriceProvider` (trait in core, impl in adapters). **Daily close** = documented FMV convention (approximates the dominion-and-control date/time standard, RevRul 2023-14; FOLLOWUPS M3).

## 10. Reconciliation & decision precedence
Engine surfaces `pending_reconciliation` outflows + `unknown_basis_inbounds`. Reconciler proposes matches (amount±`fee_sat`, time window, address, **txid match signal**); user confirms `TransferLink` / `ReclassifyOutflow` / `ClassifyInbound`, and accepts/rejects `ImportConflict`s. **Precedence:** decisions append-only, resolved in `decision_seq` order (§7.2); a later decision does not implicitly override — `VoidDecisionEvent` first; two un-voided conflicting decisions on one target → `decision_conflicts` blocker.

## 11. CLI (`btctax-cli`)
`init` · `import <files…>` (auto-groups Swan) · `reconcile` (interactive; resolves unmatched transfers, `Unclassified` rows, `ImportConflict`s, and inbound classification; prompts the full typed payload field-by-field) · `wallets` · `holdings [--at DATE]` · `lots [--wallet W]` · `events [--filter]` · `fmv` · `reconstruct-2025` · `allocate-2025` · `verify` · `export-snapshot` · `backup-key`. All behind the session/unlock; mutating commands trigger an atomic save.

## 12. Error handling & integrity
Typed errors (`thiserror`) with file/row/column context; parse failure aborts the batch. **Nothing silent:** no-BTC drops counted; `Unclassified`, `Missing` FMV, unknown-basis inbounds, import conflicts, uncovered disposals, decision conflicts, and unmatched transfers all surface in `blockers`. `verify` runs §FR9. Save atomic; lock contention → clear message; orphan `.tmp` reaped on open.

## 13. Testing & acceptance ("green" = full suite passes + 0 Critical/0 Important)
TDD throughout. Required:
- **Per-adapter** real-fixture tests: `source_ref`, gross/net+fee, preamble/CRLF, no-BTC drop count, each type mapping, Convert BTC-leg retention, Gemini BTC-Credit→Unclassified, unknown-type→Unclassified, Swan 3-file batch dedup + transfer-in non-double-count.
- **Known-answer (hand-computed) tax tests:** buy→1yr+1day→sell = LT; same-day buy/sell = ST; self-transfer fee under (c) basis-conservation **and** under (b) mini-disposition; income lot with FMV; **gift/donation outbound = zero recognized gain** with correct basis/FMV/ST-LT captured; **received-gift dual-basis disposal — all three zones** (sell above donor basis → gain + tacked HP; sell below FMV → loss + HP from gift date; in-between → no gain/loss); a 2025 transition (paths A and B) with mixed-vintage lots verifying post-2025 ST/LT + conservation; an **uncovered disposal → blocker** (not panic/negative).
- **Property tests:** conservation `Σ in == disposed + removed + held + fee-sats + pending`; no negative remainders; Σlot-basis == pool-basis.
- **Determinism:** shuffled import order **and** shuffled decision-append order → identical state. **Idempotency:** re-import incl. **cosmetic variation** (whitespace, `Decimal` scale, CRLF) → no dupes; a changed row → one `ImportConflict`; accept/reject each deterministic.
- **Storage:** atomic-save/crash (vault never partial), concurrency (2nd instance refused), encryption round-trip + wrong-passphrase-fails + mlock-failure-warns, migration identity.
- **Golden end-to-end** over the real sample set with pinned holdings/disposals/removals/income — *guarded by the known-answer tests.*

## 14. Risks & assumptions
- **R1 mlock/zeroize best-effort** (don't fully cover SQLite heap / `Decimal`/`String`); defense-in-depth; docs recommend encrypted/disabled swap.
- **R2 Adapter semantics to confirm by fixture test:** Coinbase `Order`/`Convert`/reward types; Gemini `Credit` income-vs-transfer; River CRLF + Income/Interest row shape. Unresolved → `Unclassified`, never silent.
- **R3** Pin a Sequoia-PGP version (encryption-only) + confirm S2K (Argon2 if available) before first build.
- **A1** Past tax years already filed (no historical forms; pre-2025 method FIFO unless `verify` says otherwise).
- **A2** The four sources are the current venue set; new sources are additive adapters. Externally-sourced inbounds (no internal match) need `ClassifyInbound` for basis.

## 15. Out of scope / future phases
Phase 2 (forms: 8949 + Sch D PDFs; §170(e) deduction; 8283; 709 routing; SE-tax). Phase 3 (optimizer). Non-BTC assets (incl. **fork-coin income/dispositions, e.g. 2017 BCH — explicitly out of BTC-only scope**), GUI, online pricing, multi-user. (See FOLLOWUPS.)

## 16. Suggested implementation order (input to the plan)
1. `btctax-store` safety primitives (atomic write+bak, flock, encrypt/decrypt round-trip, mlock+warn, schema_version+migrate) — test crash/concurrency/round-trip first.
2. `btctax-core` identity & ordering (money/time conventions, `source_ref`/`EventId` for imports + `decision_seq` for decisions, `LotId`, canonical order + two-pass model + determinism tests, event taxonomy).
3. `btctax-core` projection (two-pass resolve+fold; FIFO; holding period + TP11 dual-basis; TP8 (c)+(b); gift/donation non-recognition; FMV gating; totality/uncovered blocker; pool modes + paths A/B) — property + determinism + idempotency + known-answer tests.
4. `btctax-adapters` one source at a time (Swan → Coinbase → Gemini → River), each with fixture tests + the `PriceProvider` dataset.
5. Reconciliation + CLI (`reconcile` incl. conflict/inbound/raw resolution, `verify`, golden end-to-end).

## 17. (reserved)

## 18. Fold record — reviews → fixes
**Round 1 (v0.2):** ENG-C1 identity → §6.2; ENG-C2/TAX-C1 gifts-donations → TP1/TP10/§6.3/§6.4/§7.3; ENG-C3/TAX-C2 safe-harbor → §7.4/FR7/FR8; ENG-C4 crypto-to-crypto → FR2/§9.1; ENG-I1 corrections/purity → §6.2/§9; ENG-I2 pre-2025 → §7.4; ENG-I3 reclassify → §6.4; ENG-I5 decision conflict → §7.3/§10; ENG-I6 HP date → §6.1; ENG-I7 NFR2/export → NFR2/FR10; ENG-I8 FMV gating/pending → FR3/§7.3/§7.4; ENG-I9 KATs → §13; TAX-I1 inbound → `ClassifyInbound`; TAX-I2 Gemini Credit → §9.1; TAX-I3 income/Convert → FR2/§9.1; TAX-I4 txid → §6.2; TAX-I5 Swan → §9.1; minors/nits per §0.
**Round 2 (v0.3):** ENG-C2-1 decision-event identity + two-pass fold → §6.2 (`decision_seq`/EventId, total order) + §7.2 (two-pass) + NFR4 + §13 determinism. ENG-I2-1 import-conflict event + reject → §6.4 (`ImportConflict`/`SupersedeImport`/`RejectImport`) + §7.3 + FR1/FR8. ENG-I2-2 & TAX-New-I1 conservation omits removals → FR9 (`+ Σ removed`) + §13. ENG-I2-3 uncovered disposal → §7.1/§7.3 totality (`uncovered_disposal`) + §13. ENG-I2-4 safe-harbor guard trigger → §7.4 (sale/spend/transfer/gift/donation + incl. extension + warn/override/default-A). ENG-I2-5 & TAX-New-I2 dual-basis gift → TP11 + §6.3 `Lot` dual fields + §7.3 three-zone + §13. Minors: §7.3 full fold enumeration (ManualFmv/ClassifyRaw/ImportConflict); FOLLOWUPS occurrence_index + River-source_ref + daily-close labels; SupersedeImport infeasibility → §7.3 totality; gift/donation fee_sat → §7.3/FR9; ReclassifyOutflow Sell target + proceeds → §6.4; appraisal trigger note → FOLLOWUPS; GiftReceived unknown-basis → TP11/§6.4; Path-B extension/override → §7.4; TP2 pinpoint → §2; reconstruct-vs-allocate → FR7/§7.4; GiftOut/Donate are decision-produced → §6.4; capital-asset eligibility → §7.4; §15 fork note added.
