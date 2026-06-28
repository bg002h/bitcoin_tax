# SPEC — bitcoin_tax (TaxApp), Phase 1: Foundation (v0.2)

- **Status:** DRAFT v0.2 — folds round-1 independent reviews (tax + engineering + architecture). Pending re-review to green (0 Critical / 0 Important) before the plan gate.
- **Supersedes:** `SPEC_foundation_v0_1.md` (removed; preserved in git history). Round-1 reviews persisted in `reviews/`.
- **Date:** 2026-06-28
- **Phase:** 1 of 3 — **foundation only**: ingestion → canonical event ledger → lot engine → transfer reconciliation, on an encrypted store. Forms (Phase 2) and the goal-driven optimizer (Phase 3) are out of scope and get their own spec→plan→implement cycles.

## 0. References (verified at write time, against the local archive)
- Legal report + verified addendum: `legal/research/REPORT_us_btc_tax_TY2025-2026.md`, `legal/research/ADDENDUM_open_questions_verified.md`.
- Primary sources + manifest: `legal/SOURCES.md` (47 docs; `legal/SHA256SUMS` verifies 47/47); grep-able text in `legal/text/`.
- Reviews folded here: `reviews/architecture-review-phase1-foundation-round-1.md`, `reviews/spec-review-phase1-engineering-round-1.md`, `reviews/spec-review-phase1-tax-round-1.md`. Fold map: §18.
- Deferred items: `FOLLOWUPS.md`.

## 1. Purpose, scope, non-goals
**Purpose.** A local, offline, single-user desktop app (CLI first) that reconstructs a complete, auditable **per-lot Bitcoin ledger** across all venues + self-custody, so later phases can produce correct tax forms and tax-optimal sell recommendations.

**In scope (Phase 1).** Ingest Coinbase, Gemini, River, Swan exports (≈6 schemas); normalize to an append-only canonical event log; derive a per-wallet lot ledger as a deterministic projection; model self-custody wallets + assisted transfer reconciliation; reconstruct full history for basis; handle the Jan-1-2025 per-wallet transition; PGP-encrypted vault; a CLI to import/reconcile/inspect/resolve/verify/export.

**Non-goals (Phase 1).** Form generation; the optimizer; non-BTC assets; multi-user; networked operation; GUI. BTC-only is a hard scope decision (see FR2 for the *correct* meaning of "BTC-only").

## 2. Tax positions (TP) — each cited to the archive; engine encodes them as named, swappable rules
| ID | Position | Archived basis | Uncertainty |
|----|----------|----------------|-------------|
| TP1 | BTC is **property**. A **sale or spend** is a realization (gain/loss) event. **Gift and donation are NOT realization events** (non-recognition removals — see TP10). | Notice 2014-21 A-1/A-6; §1001(a)/(c) (`26USC_s1001`); RevProc 2024-28 §3.11 (gift/donation = "transfer, other than a sale or disposition"). | Settled. |
| TP2 | **Basis** = USD cost + **acquisition** costs/fees. **Disposition** fees/selling expenses **reduce proceeds** (amount realized). | Pub 551 ("commissions and … transfer fees"); §1.1012-1(h)(2)(i); §1001(b); Pub 544 ("Minus: Selling expenses"). | Settled. |
| TP3 | Income-received BTC = **ordinary income at FMV-USD on dominion & control**; that FMV becomes the lot basis; holding period starts the next day. Tag mining as business-vs-hobby (Phase-2 SE-tax). | §61 (`26USC_s61`); Notice 2014-21 A-4/A-8/A-9; RevRul 2023-14; RevRul 2019-24. | Settled. |
| TP4 | **Holding period:** starts the day *after* acquisition, includes the disposition day; >1 yr = long-term. | Pub 544 (worked example); §1222 (`26USC_s1222`); §1223 tacking (`26USC_s1223`). | Settled. |
| TP5 | Default lot method **FIFO**; engine is **specific-ID-ready** (HIFO/LIFO are forms of specific-ID; selection UX = Phase 3). | §1.1012-1(j) (`26CFR_1.1012-1_basis.xml`): ID "no later than the date and time of the sale," else deemed-earliest per wallet/account. | Settled. |
| TP6 | **Per-wallet basis from 2025-01-01.** Two supported paths to seed it (see §7.4): **(A) actual per-wallet reconstruction** under §1.1012-1(j) (default; uses full history), **(B) Rev. Proc. 2024-28 safe-harbor allocation** — eligibility/deadline-guarded and **irrevocable**. | RevProc 2024-28 §§3.11/4.01/4.02(6)/5.02; §1.1012-1(j). | Settled (mechanics); deadlines are date-sensitive — §7.4. |
| TP7 | **Self-transfers** (own→own) are non-taxable; lots carry basis + holding period. | §1001 (no sale/disposition); RevProc 2024-28 §3.11; §1.1012-1(j). | Settled. |
| TP8 | **Network-fee on a self-transfer — DEFAULT treatment (c):** `fee_sat` consumed at **zero proceeds** (non-taxable transfer cost, no deduction); full basis carries to the sats that arrive. **User-configurable to treatment (b):** taxable **mini-disposition** of the fee-sats (disposed at FMV → gain/loss; principal still carries basis). | Contrary signal (taxable-exchange context only): §1.1012-1(h)(2)/(h)(4). No on-point guidance for a pure self-transfer miner fee. | **Limited guidance.** Default (c) is **user-mandated**; (b) is a config option. Do not change the default. |
| TP9 | **Wash-sale rule does not apply** to crypto. | §1091 (`26USC_s1091`) covers only "stock or securities"; crypto is property. | Pending legislation — out of Phase-1 scope. |
| TP10 | **Gift out** = non-recognition removal: lot leaves at **zero gain/loss**; capture FMV-at-transfer + ST/LT + donor info for the donee's carryover/dual basis (§1015) and tacking (§1223). **Charitable donation** = non-recognition removal: capture FMV + ST/LT + **>$5k qualified-appraisal-required** flag for the Phase-2 §170(e) deduction. | §1015 (`26USC_s1015`); §170(e)(1)/(f)(11)(C) (`26USC_s170`); CCA 202302012; RevProc 2024-28 §3.11. | Settled (non-recognition). Deduction/forms = Phase 2. |

## 3. Functional requirements
- **FR1 Import.** Accept one or more files; auto-detect source; group multi-file sources (Swan's 3 files) into one batch; parse; normalize; assign `source_ref`; validate; append **all parsed events for the batch atomically** (all-or-nothing on parse/validation failure). Re-importing the same or a corrected re-export is **idempotent** for unchanged rows; a changed row (same `source_ref`, different content) records an **import-conflict event** requiring user accept (supersede) or reject — never silent overwrite/drop (§9).
- **FR2 BTC-only filter (corrected).** Drop a row **only if it has no BTC leg**. Any row with a BTC leg is retained: a crypto↔BTC trade is a BTC **disposition** (BTC out) or **acquisition** (BTC in) at FMV; its non-BTC leg is ignored. **Unknown/ambiguous BTC-side row types are routed to `Unclassified` (a blocker), never dropped.** Report dropped (no-BTC) and unclassified counts per file.
- **FR3 FMV resolution.** Prefer export-provided USD; else bundled price dataset; else `Missing` (a blocker). Track `FmvStatus`. `Missing` FMV blocks **both** the income amount and any downstream disposal that consumes the affected lot (§7.3).
- **FR4 Projection.** Deterministically derive, rebuilt from scratch: per-wallet lots, holdings (optionally as-of a date), disposals (per-lot proceeds/basis/gain + ST/LT) for **Sell/Spend**, non-recognition removals (Gift/Donation) with FMV+ST/LT metadata, recognized ordinary income, the reconciliation queue, and all open blockers.
- **FR5 Wallets.** Model exchange accounts + self-custody wallets as first-class basis pools; create/label self-custody wallets.
- **FR6 Reconciliation.** Propose matches for unclassified outflows ↔ inflows/known wallets (amount±fee, time window, address, **txid as a match signal**); user confirms a self-transfer (`TransferLink`) or reclassifies (spend/gift/donation). Classify standalone inbounds (`ClassifyInbound`) as income or received-gift. Unmatched outflows **and** unknown-basis inbounds remain flagged.
- **FR7 2025 basis transition.** Provide both: `reconstruct-2025` (path A, default) and `allocate-2025` (path B, safe harbor) — with the eligibility/deadline/irrevocability guards of §7.4.
- **FR8 Corrections.** `VoidDecisionEvent` may revoke a *revocable* decision. An **effective `SafeHarborAllocation` is irrevocable** and cannot be voided/re-done (§7.4).
- **FR9 Integrity (`verify`).** Per-wallet holdings vs source running balances (Gemini, Swan); global sat conservation: `Σ in == Σ disposed + Σ held + Σ fee-sats + Σ pending-reconciliation`; report drift, unknown-basis inbounds, FMV blockers, unclassified rows, import conflicts, un-voided decision conflicts, and any pre-2025 filed-method reconciliation note.
- **FR10 Export.** `export-snapshot` writes the **decrypted** ledger (SQLite + CSV) for backup/inspection — the *sole, explicit, user-invoked* exception to the no-plaintext rule (NFR2). `backup-key` exports the passphrase-protected key.

## 4. Non-functional requirements
- **NFR1 Local & offline.** No network in normal operation; bundled price data.
- **NFR2 Encryption at rest.** The only on-disk artifact written **automatically/implicitly** is the PGP-encrypted vault; no plaintext DB is ever written except by the explicit user-invoked `export-snapshot` (FR10).
- **NFR3 Durability.** No save can corrupt/lose the vault (atomic write + rolling backup, §8).
- **NFR4 Determinism.** Identical inputs → identical ledger, independent of import order (§6.2).
- **NFR5 Exact arithmetic.** No floats for money (§6.1).
- **NFR6 Auditability.** Every derived number traces to events; the event log is the sole source of truth (all state, incl. conflicts, lives as events).
- **NFR7 Single-user safety.** Concurrent instances cannot silently clobber (§8).

## 5. Architecture
**Event-sourced core.** Append-only **event log = single source of truth**; all ledger state is a **pure deterministic projection** re-derived from scratch (no caching in Phase 1).

**Cargo workspace** (license `MIT OR Unlicense`): `btctax-core` (domain + `PriceProvider` trait + projection; pure, no I/O), `btctax-adapters` (per-source parsers + bundled price dataset), `btctax-store` (PGP-blob ⇄ in-memory SQLite, key/session), `btctax-cli`. (Future: `btctax-forms`, `btctax-optimizer`.)

**Data flow:** `files → adapter(detect→group→preamble→parse→normalize→source_ref→FMV) → atomic append → encrypted event log → pure projection(canonical order, fold rules, pool mode) → holdings / disposals / income / reconciliation queue / blockers → CLI / verify / export`.

## 6. Domain model
### 6.1 Money & time
- **BTC** = integer **satoshis** (`i64`); never float. **USD** = `rust_decimal::Decimal`. **Rounding** = `ROUND_HALF_EVEN`, defined once in `domain::conventions`.
- **Time** stored UTC + `original_tz`. **Holding-period day-count uses the calendar date in `original_tz`** (the taxpayer's trade date), so a late-evening local trade is not shifted across midnight by UTC (TP4). Documented as the authoritative date rule.

### 6.2 Identity, dedup & ordering
- **`source_ref`** = the **stable identity of a real-world row, scoped by `(source, direction)`** (direction ∈ {in,out,trade,other}). Built from the source's native stable id where present (Coinbase `ID`, Gemini `Trade ID`, Swan `Transaction ID`); on-chain **txid is used for within-source cross-file dedup (Swan's 3 files) and as a cross-source reconciliation *match signal* — NOT as a global dedup key** (the same txid legitimately appears on both legs of a cross-venue transfer). For sources without native ids (River), `source_ref` = canonical `(source, direction, utc_ms, type, sat)`; a last-resort `occurrence_index` disambiguates exact duplicates within one import (documented fragility, FOLLOWUPS).
- **`EventId`** = stable function of `(source, source_ref)` — **not** a content hash — so it survives cosmetic re-exports and corrections (a correction keeps the same `EventId`; see §9 supersession). A **canonical content encoding** (fixed field order; `Decimal` normalized to a fixed scale; explicit optional/timestamp encoding) is defined for the **content fingerprint** used only to detect "same `source_ref`, changed content" conflicts (§9).
- **`LotId` = (origin_event_id: EventId, split_sequence: u32)**; `split_sequence` assigned deterministically as the projection splits lots — a function of event history, never random. Stable across re-derivation because `EventId` is stable.
- **Canonical event order** (named, documented, tax-affecting): `utc_timestamp` → fixed **source priority** (Swan>Coinbase>Gemini>River; arbitrary-but-stable) → `source_ref` lexicographic. Mandatory order-independence test.

### 6.3 Entities
- **`Wallet`** (basis pool): `Exchange{provider, account}` | `SelfCustody{label}`. (No `External` — unreconciled flows are projection state.)
- **`LedgerEvent`** (immutable): `EventId`, `utc_timestamp`, `original_tz`, `source`, `source_ref`, `wallet`, `payload`.
- **`Lot`** (derived): `lot_id`, `wallet`, `acquired_at`, `original_sat`, `remaining_sat`, `usd_basis`, `basis_source`; splits on partial disposal/transfer.
- **`Disposal`** (derived, **Sell/Spend only**): consumed lots with per-lot proceeds/basis/gain + ST/LT + propagated `basis_source`.
- **`Removal`** (derived, **Gift/Donation**): consumed lots with FMV-at-transfer + ST/LT + donor/appraisal metadata; **zero recognized gain/loss**.

### 6.4 Event taxonomy
**Imported events:**
- `Acquire { sat, usd_cost, fee_usd, basis_source }` — buy/purchased receive. Basis = cost + acquisition fee.
- `Income { sat, usd_fmv: Option<Decimal>, fmv_status, kind: Mining|Staking|Interest|Airdrop|Reward, business: bool }` — ordinary income; new lot at FMV.
- `Dispose { sat, usd_proceeds, fee_usd, kind: Sell|Spend }` — realization; net proceeds = usd_proceeds − fee_usd.
- `GiftOut { sat, fmv_at_transfer, fee_sat? }` / `Donate { sat, fmv_at_transfer, fee_sat?, appraisal_required: bool }` — **non-recognition removals** (TP10).
- `TransferOut { sat, fee_sat?, dest_addr?, txid? }` / `TransferIn { sat, src_addr?, txid? }` — movement; unclassified until linked/classified.
- `Unclassified { raw }` — a parsed BTC-side row that doesn't map unambiguously (e.g., Coinbase `Order`, ambiguous Gemini BTC `Credit`). Imported, inert, surfaced as a blocker; never guessed, never dropped.

**Decision events (append-only):**
- `TransferLink { out_event, in_event_or_wallet }` — confirms a non-taxable self-transfer (applies TP8).
- `ReclassifyOutflow { transfer_out_event, as: Dispose{Spend}|GiftOut|Donate, usd_proceeds_or_fmv, fee_usd? }` — gives a reclassified outflow real proceeds/FMV + fee semantics (the on-chain `fee_sat` of a spend is part of the disposed quantity).
- `ClassifyInbound { transfer_in_event, as: Income{kind, fmv, business} | GiftReceived{donor_basis, donor_acquired_at, fmv_at_gift} }` — classifies a standalone inbound (fixes unknown-basis / unrecognized-income).
- `ManualFmv { event, usd_fmv }`.
- `SafeHarborAllocation { lots: [{wallet, sat, usd_basis, acquired_at}], method: ActualPosition|ProRata, effective_date: 2025-01-01 }` — carries **lots with acquisition dates** (not aggregates); conservation-checked; irrevocable once effective (§7.4).
- `SupersedeImport { target_event, corrected_payload }` — accepts a corrected re-export row (keeps the `EventId`; records the change as an event for purity).
- `VoidDecisionEvent { target_event_id }` — revokes a *revocable* decision (not an effective `SafeHarborAllocation`).
- `ClassifyRaw { target: Unclassified_event, as: <imported payload> }` — resolves an `Unclassified` row.

- **`FmvStatus`** = `ExchangeProvided | PriceDataset | ManualEntry | Missing`.
- **`BasisSource`** = `ExchangeProvided | ComputedFromCost | FmvAtIncome | CarriedFromTransfer | GiftCarryover | SafeHarborAllocated | ReconstructedPerWallet`. Propagated `Lot → Disposal/Removal`.

## 7. Projection / lot engine
### 7.1 Contract
`project(ordered_events) -> LedgerState { lots, holdings_by_wallet, disposals, removals, income_recognized, pending_reconciliation, blockers }` — pure, deterministic, no I/O, rebuilt from scratch. `blockers` = `{ fmv_missing, unclassified, import_conflicts, unmatched_outflows, unknown_basis_inbounds, decision_conflicts, pre2025_method_note }`.

### 7.2 Determinism
Fold in canonical order (§6.2); re-import in any order → identical `LedgerState` (tested).

### 7.3 Fold rules
- `Acquire` → new lot (basis = cost + fee; `basis_source`).
- `Income` → if FMV known, new lot at FMV + record ordinary income; if `Missing`, record lot existence, add to `fmv_missing`, and **block both** the income amount and any disposal/removal consuming a lot whose history includes that unresolved income.
- `Dispose{Sell|Spend}` → consume from the wallet pool (default FIFO; spec-ID-ready) → `Disposal` (net proceeds/basis/gain + ST/LT). Split → new `split_sequence`.
- `GiftOut`/`Donate` → consume lots → `Removal` with FMV + ST/LT + metadata; **zero recognized gain** (TP10).
- `TransferOut` (unclassified) → lots leave into `pending_reconciliation`.
- `TransferLink` → move exact lots to destination carrying basis + `acquired_at`; **TP8 default (c)**: `fee_sat` consumed at zero proceeds, full basis to remainder; **config (b)**: fee-sats are a mini-disposition (FMV proceeds, gain/loss), principal still carries.
- `ClassifyInbound` → turn a `TransferIn` into an `Income` lot (FMV basis) or a `GiftReceived` lot (donor carryover/dual basis + tacked holding period).
- `ReclassifyOutflow` → fold the outflow as the chosen realization/removal with supplied proceeds/FMV + fee.
- `SupersedeImport` → replace the targeted event's payload (same `EventId`; references intact).
- `VoidDecisionEvent` → drop a revocable decision; **two un-voided conflicting decisions on the same target → `decision_conflicts` blocker** (no silent "both apply").

### 7.4 2025 basis transition (TP6) — guarded, with fallback
- Pre-2025: **`UniversalPool`** that **tracks lots (with acquisition dates), un-partitioned by wallet**; pre-2025 disposals consume it **FIFO** (legal default). If the taxpayer's *filed* pre-2025 returns used another method, emit a `pre2025_method_note` blocker for reconciliation (don't silently assume).
- At 2025-01-01, seed `PerWalletPool` by one of:
  - **Path A — actual per-wallet reconstruction (default, most defensible):** assign each remaining lot to the wallet that actually holds it at 2025-01-01 (derivable from the reconciled transfer history), preserving `acquired_at` and basis. `basis_source = ReconstructedPerWallet`.
  - **Path B — Rev. Proc. 2024-28 safe harbor:** `SafeHarborAllocation` of remaining **lots** to wallets (`ActualPosition` preferred; `ProRata` permitted), preserving acquisition dates. **Guards:** (1) **eligibility/deadline** — refuse/flag if it would post *after* the earlier of the taxpayer's first 2025 disposition or the 2025 return due date (global-method variant required pre-2025); (2) **irrevocable** once effective (FR8 cannot void it); (3) **conservation** — Σsat == remaining held sat and Σbasis == remaining pool basis at 2025-01-01.
- 2025+: `PerWalletPool` — FIFO/spec-ID strictly within each wallet; self-transfers carry lots between pools.
- **Pending-at-snapshot:** sats still in `pending_reconciliation` at 2025-01-01 are **excluded from allocation** (neither held nor disposed) and flagged; they enter a wallet pool only once reconciled.

## 8. Encrypted storage & session (`btctax-store`)
- **On disk:** one `vault.pgp` (Sequoia-PGP, encrypted to the app-managed keypair; private key passphrase-protected with the **strongest available S2K** — Argon2 if the pinned Sequoia supports it, else high-work-factor iterated-salted; see FOLLOWUPS KDF note). Decrypted layout: `[schema_version:u32][SQLite serialized image]`.
- **Open:** `flock(LOCK_EX|LOCK_NB)` → fail fast if held (NFR7); reap any orphan `vault.pgp.tmp`; decrypt → `mlock` the plaintext buffer (**best-effort defense-in-depth; warn if it fails** — does not fully cover SQLite internal heap/Decimal/String, see R1); `deserialize` into in-memory SQLite.
- **Save:** serialize → prepend `schema_version` → encrypt → write `vault.pgp.tmp` (same dir) → `fsync` → atomic `rename()` over `vault.pgp`, rotating prior to `vault.pgp.bak`.
- **Migration:** `migrate(version, …)` on open; versioning spans the **outer layout, the SQLite DDL, and the serde encoding of event payloads** (deserialize-and-transform, not raw-byte patching).
- **Session:** unlock once; key + DB held in `mlock`ed, `zeroize`-on-drop memory (best-effort); re-lock on exit/timeout.
- **Key lifecycle:** `init` generates the keypair, sets the passphrase, **forces a key-backup step** (key/passphrase loss = unrecoverable). `export-snapshot` is the recovery escape hatch (NFR2 exception).

## 9. Ingestion & adapters (`btctax-adapters`)
**Pipeline:** detect source → **group multi-file sources (Swan) into one batch** → strip preamble (handle CRLF) → parse → normalize → `source_ref` → validate all → **atomic append**. A row whose `source_ref` exists with a different content fingerprint → an **import-conflict** recorded as an event (accept via `SupersedeImport` or reject); identical content → idempotent no-op.

**`Adapter` trait** (`detect`/`group`/`parse`/`normalize`). Each adapter's module doc states, **with a passing test on real fixtures**: `source_ref`/dedup fields, gross-vs-net proceeds, fee placement, and the **unknown-BTC-type → `Unclassified`** rule.

### 9.1 Per-source mapping (from real-sample inspection; tax-relevant invariants tested)
- **Coinbase** (yearly CSV; 3-line preamble; native `ID` = `source_ref`): `Buy`→`Acquire` (basis = `Total` incl. fees = `Subtotal`+`Fees`); `Sell`→`Dispose{Sell}` (gross proceeds = `Subtotal`, fee = `Fees`); `Send`→`TransferOut`; `Receive`/`Exchange Deposit`/`Pro Deposit`→`TransferIn`; `Withdrawal`/`Exchange Withdrawal`/`Pro Withdrawal`→`TransferOut`. **`Convert`→ the BTC leg is `Dispose{Sell}` (BTC out) or `Acquire` (BTC in) at FMV**; **`Order` and any unrecognized BTC-side type → `Unclassified`**. Reward/income-in-BTC types → `Income`. Non-BTC-only rows dropped (FR2).
- **Gemini** (wide XLSX ledger; `Tx Hash`/`Trade ID` for within-source dedup; `BTC Balance` for reconciliation): `Buy`→`Acquire`, `Sell`→`Dispose{Sell}` (`USD Amount`=gross, `Fee (USD)` separate); `Debit`(BTC)→`TransferOut`; **`Credit`(BTC) is ambiguous → `Unclassified` (income-vs-transfer disambiguation), never auto-`TransferIn`**; `Credit`(USD) = cash deposit (no BTC event).
- **River** (universal CSV; may be CRLF; no native id → semantic `source_ref`): `Buy`→`Acquire` (basis = `Sent`+`Fee`; `Sent` excludes fee); `Income`/`Interest`→`Income{kind}` (BTC, no USD → FMV from dataset); BTC-sent/`Withdrawal`→`TransferOut`.
- **Swan** (3 files = one batch; `Transaction ID`/txid dedup *within Swan*): `trades`→`Acquire`. `transfers` (carries `USD Cost Basis`+`Acquisition Date`) → **a reconcilable `TransferIn`** that, when matched to a tracked source-venue outflow, **avoids double-counting** (the source lot is authoritative and carries; Swan's stated basis is used only when no matching internal outflow exists, i.e., externally-sourced coins). `withdrawals`→`TransferOut`.

### 9.2 Price dataset
Bundled daily BTC/USD behind `PriceProvider` (trait in core, impl in adapters). **Daily close** is the documented FMV convention (an approximation of the dominion-and-control date/time standard; FOLLOWUPS M3).

## 10. Reconciliation & decision precedence
Engine surfaces `pending_reconciliation` outflows and `unknown_basis_inbounds`. The reconciler proposes matches (amount±`fee_sat`, time window, address, **txid match signal**); the user confirms `TransferLink`, `ReclassifyOutflow`, or `ClassifyInbound`. **Precedence:** decisions are append-only; a later decision does not implicitly override — the user must `VoidDecisionEvent` first. Two un-voided conflicting decisions on one target → a `decision_conflicts` blocker (deterministic, not "both apply").

## 11. CLI (`btctax-cli`)
`init` · `import <files…>` (auto-groups Swan) · `reconcile` (interactive; for `Unclassified`/inbound classification, prompts the full typed payload field-by-field) · `wallets` · `holdings [--at DATE]` · `lots [--wallet W]` · `events [--filter]` · `fmv` · `reconstruct-2025` · `allocate-2025` · `verify` · `export-snapshot` · `backup-key`. All behind the session/unlock; mutating commands trigger an atomic save.

## 12. Error handling & integrity
Typed errors (`thiserror`) with file/row/column context; parse failure aborts the batch (all-or-nothing). **Nothing silent:** no-BTC drops counted; `Unclassified`, `Missing` FMV, unknown-basis inbounds, import conflicts, decision conflicts, and unmatched transfers all surface in `blockers`. `verify` runs the §FR9 checks. Save is atomic; lock contention → clear message; orphan `.tmp` reaped on open.

## 13. Testing & acceptance ("green" = full suite passes + 0 Critical/0 Important)
TDD throughout. Required:
- **Per-adapter** real-fixture tests: `source_ref`, gross/net+fee, preamble/CRLF, no-BTC drop count, each type mapping, Convert BTC-leg retention, Gemini BTC-Credit→Unclassified, unknown-type→Unclassified, Swan 3-file batch dedup + transfer-in non-double-count.
- **Known-answer (hand-computed) tax tests:** buy→1yr+1day→sell = LT; same-day buy/sell = ST; self-transfer with fee under (c) basis-conservation **and** under (b) mini-disposition; income lot with FMV; **gift/donation = zero recognized gain** with correct FMV/ST-LT captured; a 2025 transition (paths A and B) with mixed-vintage lots verifying post-2025 ST/LT and conservation.
- **Property tests:** basis conservation, no negative remainders, Σlot-basis == pool-basis.
- **Determinism:** shuffled import order → identical state. **Idempotency:** re-import incl. **cosmetic variation** (whitespace, `Decimal` scale, CRLF) → no dupes; a changed row → one import-conflict.
- **Storage:** atomic-save/crash (vault never partial), concurrency (2nd instance refused), encryption round-trip + wrong-passphrase-fails + mlock-failure-warns, migration identity.
- **Golden end-to-end** over the real sample set with pinned holdings/disposals/income — *guarded by the known-answer tests so the snapshot can't freeze wrong numbers.*

## 14. Risks & assumptions
- **R1 mlock/zeroize are best-effort.** They don't fully cover SQLite's internal heap or `String`/`Decimal` buffers; framed as defense-in-depth; docs recommend encrypted/disabled swap.
- **R2 Adapter semantics to confirm by fixture test:** Coinbase `Order`/`Convert`/reward types; Gemini `Credit` income-vs-transfer; River CRLF + Income/Interest row shape. Unresolved cases → `Unclassified` blocker, never silent.
- **R3** Pin a specific Sequoia-PGP version (encryption-only) + confirm S2K (Argon2 if available) before first build.
- **A1** Past tax years were already filed (no historical forms in Phase 1); pre-2025 method assumed FIFO unless `verify` reconciliation says otherwise.
- **A2** The four sources are the current venue set; new sources are additive adapters. Externally-sourced inbounds (no internal match) require `ClassifyInbound` for basis.

## 15. Out of scope / future phases
Phase 2 (forms: 8949 + Sch D PDFs; §170(e) deduction; 8283; 709 routing; SE-tax). Phase 3 (optimizer). Non-BTC assets, GUI, online pricing, multi-user. (See FOLLOWUPS.)

## 16. Suggested implementation order (input to the plan)
1. `btctax-store` safety primitives (atomic write+bak, flock, encrypt/decrypt round-trip, mlock+warn, schema_version+migrate) — test crash/concurrency/round-trip first.
2. `btctax-core` identity & ordering (money/time conventions, `source_ref`/`EventId`/`LotId`, canonical order + order-independence test, event taxonomy).
3. `btctax-core` projection (fold rules, FIFO, holding period, TP8 (c)+(b), gift/donation non-recognition, FMV gating, pool modes + paths A/B) — property + determinism + idempotency + known-answer tests.
4. `btctax-adapters` one source at a time (Swan → Coinbase → Gemini → River), each with fixture tests + the `PriceProvider` dataset.
5. Reconciliation + CLI (`reconcile`, classification flows, `verify`, golden end-to-end).

## 17. (reserved)

## 18. Fold record (v0.2) — round-1 reviews → fixes
**Critical:** ENG-C1/identity → §6.2 (`source_ref` stable identity, `EventId` not content-hash, canonical fingerprint, txid not a global key) + I4. ENG-C2 & TAX-C1/gifts-donations → TP1/TP10, §6.3 `Removal`, §6.4 `GiftOut`/`Donate`, §7.3 zero-gain. ENG-C3 & TAX-C2/safe-harbor → §7.4 (lot-level allocation, eligibility/deadline/irrevocability guards, conservation, **path-A reconstruction fallback**), FR7/FR8. ENG-C4/crypto-to-crypto → FR2 + §9.1 (BTC leg retained; Convert).
**Important:** ENG-I1 corrections/purity → §6.2 `EventId` stability + §9 `SupersedeImport` (event-recorded). ENG-I2 pre-2025 method → §7.4 FIFO + note. ENG-I3 & TAX-(M5) Reclassify proceeds/fee → `ReclassifyOutflow`. ENG-I5 decision conflict → §7.3/§10 `decision_conflicts`. ENG-I6 HP date → §6.1 original-tz date. ENG-I7 NFR2 vs export → NFR2/FR10 wording. ENG-I8 FMV gating + pending@snapshot → FR3/§7.3/§7.4. ENG-I9 known-answer tests → §13. TAX-I1 inbound income/gift → `ClassifyInbound`. TAX-I2 Gemini Credit → §9.1 `Unclassified`. TAX-I3 income/Convert mapping + unknown→Unclassified → FR2/§9.1. TAX-I4 txid dedup-vs-match → §6.2. TAX-I5 Swan double-count → §9.1.
**Minor/Nit:** TAX-M1/M2 re-cite to archive → §2 table. TAX-M3 daily-close note → §9.2/FOLLOWUPS. TAX-M4 fork out-of-scope → FOLLOWUPS/§15. TAX-M6 prefer actual allocation → §7.4. ENG-m1 mlock wording → §8/R1. ENG-m2 occurrence_index fragility → §6.2/FOLLOWUPS. ENG-m3 intra-day order → §6.2. ENG-m4 migration levels → §8. ENG-m5 orphan inbound basis → FR6/FR9. ENG-m6 Swan CLI grouping → FR1/§9/§11. ENG-m7 ClassifyRaw ergonomics → §11. ENG-m8 tmp cleanup → §8/§12. ENG-n2 arbitrary tiebreak → §6.2. ENG-n3 conservation incl. pending → FR9. TAX-N1 mining SE flag → `Income.business` + FOLLOWUPS. TAX-N2 TP8 contrary cite → TP8.
