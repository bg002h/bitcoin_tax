# SPEC — bitcoin_tax (TaxApp), Phase 1: Foundation (v0.1)

- **Status:** DRAFT — pending self-review, user review, and the independent review-to-green gate (STANDARD_WORKFLOW §2) before the plan.
- **Date:** 2026-06-28
- **Phase:** 1 of 3. This spec covers the **foundation only**: ingestion → canonical event ledger → lot engine → transfer reconciliation, on an encrypted store. **Forms (Phase 2)** and the **goal-driven optimizer (Phase 3)** are out of scope here and get their own spec→plan→implement cycles.

## 0. References (verified at write time)
- Legal report: `legal/research/REPORT_us_btc_tax_TY2025-2026.md`
- Verified addendum (the 5 open questions): `legal/research/ADDENDUM_open_questions_verified.md`
- Primary-source archive + manifest: `legal/SOURCES.md` (47 docs; `legal/SHA256SUMS` verifies 47/47); grep-able text in `legal/text/`.
- Architecture review folded into this spec: `reviews/architecture-review-phase1-foundation-round-1.md` (4 Critical / 8 Important / 4 Minor / 3 Nit — all addressed; see §13 traceability).

## 1. Purpose, scope, non-goals
**Purpose.** A local, offline, single-user desktop application (CLI first) that reconstructs a complete, auditable **per-lot Bitcoin ledger** across all of a US taxpayer's venues and self-custody, so that later phases can produce correct tax forms and tax-optimal sell recommendations.

**In scope (Phase 1).**
- Ingest exchange exports from **Coinbase, Gemini, River, Swan** (≈6 schemas).
- Normalize to an append-only **canonical event log**; derive a per-wallet lot ledger as a deterministic projection.
- Track **self-custody wallets** as first-class locations; **assisted transfer reconciliation** with user classification.
- Reconstruct **full history** for basis; handle the **Jan-1-2025 per-wallet transition** via the Rev. Proc. 2024-28 safe harbor.
- Store everything in a **PGP-encrypted** vault (app-managed key, passphrase-protected).
- A CLI to import, reconcile, inspect holdings/lots/events, resolve FMV, run the 2025 allocation, verify integrity, and export.

**Non-goals (Phase 1).** Form 8949 / Schedule D generation; the optimizer; non-BTC assets; multi-user; networked/online operation; a GUI. (BTC-only is a hard scope decision; non-BTC rows are dropped at ingest with a reported count.)

## 2. Locked tax positions (with citations + uncertainty flags)
The engine encodes these as **named, swappable rules** so a guidance change is a localized edit. Each is backed by the archive.

| ID | Position | Basis | Uncertainty |
|----|----------|-------|-------------|
| TP1 | BTC is **property**; every disposition (sell/spend/gift/donation) is a realization event. | Notice 2014-21; §1001. | Settled. |
| TP2 | **Basis** = USD cost + **acquisition** fees; **disposition** fees reduce proceeds. | Pub 551; FAQ; §1012. | Settled. |
| TP3 | Income-received BTC = **ordinary income at FMV-USD on dominion & control**; FMV becomes the lot's basis; holding period starts next day. | §61; RevRul 2023-14, 2019-24; Notice 2014-21. | Settled. |
| TP4 | **Holding period:** starts day *after* acquisition, includes disposition day; >1yr = long-term. | §1222/§1223; Pub 544. | Settled. |
| TP5 | Default lot method **FIFO**; engine is **specific-ID-ready** (HIFO/LIFO are forms of specific-ID). Spec-ID selection/UX is Phase 3; Phase 1 only guarantees the ledger supports it. | FAQ; §1.1012-1; addendum Q-spec. | Settled. |
| TP6 | **Per-wallet basis** from 2025-01-01; pre-2025 uses an aggregate pool; transition via **Rev. Proc. 2024-28 safe harbor** (one-time allocation). | RevProc 2024-28; §1.1012-1(j). | Settled (user chose safe-harbor path). |
| TP7 | **Self-transfers** (own→own wallet) are non-taxable; lots carry basis + holding period. | FAQ; §1001 realization. | Settled. |
| TP8 | **Network-fee treatment = (c):** on a self-transfer the `fee_sat` is consumed at **zero proceeds** (non-taxable transfer cost, no deduction); the **full original basis** rides the sats that arrive. | Practitioner-common; §1001 (no realization on self-transfer). | **Limited IRS guidance — documented default, swappable.** |
| TP9 | **Wash-sale rule does not apply** to crypto (no loss deferral). Phase-1 relevance: none beyond not implementing deferral. | §1091; addendum Q1. | Pending legislation could change — out of Phase-1 scope. |

## 3. Functional requirements
- **FR1 Import.** Accept one or more export files; auto-detect source; parse; normalize to canonical events; assign dedup keys; validate; append **all-or-nothing** in a single transaction. Re-importing the same or a corrected re-export is **idempotent** (no duplicates).
- **FR2 BTC-only filter.** Drop non-BTC rows (ETH/BCH/LTC/…) at normalization; report the dropped count per file.
- **FR3 FMV resolution.** For events lacking USD, set FMV from the bundled price dataset; if still unavailable, mark `Missing` and surface as a blocker. Track provenance via `FmvStatus`.
- **FR4 Projection.** Derive, deterministically and re-buildable from scratch: per-wallet lots, holdings (optionally as-of a date), disposals with per-lot gain/loss + ST/LT, recognized ordinary income, the reconciliation queue, and all open blockers (missing FMV, unclassified rows, import conflicts).
- **FR5 Wallets.** Model exchange accounts and self-custody wallets as first-class locations (basis pools). Allow creating/labeling self-custody wallets.
- **FR6 Reconciliation.** Propose matches for unclassified outflows ↔ inflows/known wallets (amount±fee, time window, address/txid); user confirms a self-transfer (→ `TransferLink`) or classifies as spend/gift/donation. Unmatched outflows remain flagged in `PendingReconciliation`.
- **FR7 2025 allocation.** Provide an `allocate-2025` action producing the `SafeHarborAllocation` decision event that switches the engine from `UniversalPool` to `PerWalletPool`.
- **FR8 Corrections.** Allow voiding/superseding a prior decision event (`VoidDecisionEvent`) without mutating history.
- **FR9 Integrity (`verify`).** Cross-check reconstructed per-wallet holdings against source running balances where available (Gemini, Swan); run global sat-conservation (in == out + held + fees); report drift and all open blockers.
- **FR10 Export.** `export-snapshot` writes the decrypted ledger (SQLite + CSV) for backup/inspection; `backup-key` exports the (passphrase-protected) key.

## 4. Non-functional requirements
- **NFR1 Local & offline.** No network calls in normal operation; bundled price data. (Online price lookup is explicitly out of scope.)
- **NFR2 Encryption at rest.** Only artifact on disk is the PGP-encrypted vault; no plaintext DB file ever written (§8).
- **NFR3 Durability.** No save can corrupt or lose the vault (atomic write + rolling backup, §8).
- **NFR4 Determinism.** Identical inputs → identical ledger, independent of import order (§7.2).
- **NFR5 Exact arithmetic.** No floating point for money (§6.1).
- **NFR6 Auditability.** Every derived number is traceable to source events; reviews and tax positions are documented; the event log is the source of truth.
- **NFR7 Single-user safety.** Concurrent instances cannot silently clobber (§8).

## 5. Architecture
**Event-sourced core.** An append-only **event log is the single source of truth**; all ledger state is a **pure deterministic projection** folded from events and re-derived from scratch (no caching in Phase 1 — YAGNI).

**Cargo workspace:**
- `btctax-core` — domain types, the `PriceProvider` trait, and the projection engine. **Pure, no I/O.** The most heavily tested crate.
- `btctax-adapters` — one parser per source + the bundled price dataset implementing `PriceProvider`.
- `btctax-store` — PGP-encrypted-blob ⇄ in-memory SQLite, key/session/passphrase lifecycle.
- `btctax-cli` — command surface wiring the above.
- *(future: `btctax-forms`, `btctax-optimizer`.)*

**End-to-end data flow:** `export files → adapter (detect→preamble→parse→normalize→dedup→FMV) → all-or-nothing append → encrypted event log → pure projection (canonical order, fold rules, pool mode) → holdings / disposals / reconciliation queue / FMV blockers → CLI / verify / export`.

## 6. Domain model
### 6.1 Money & time
- **BTC** = integer **satoshis** (`i64`); never float.
- **USD** = `rust_decimal::Decimal`; intermediate math exact.
- **Rounding** = `ROUND_HALF_EVEN`, defined once in `domain::conventions` and used at every USD computation site.
- **Time** = UTC `time::OffsetDateTime`; original timezone preserved on the event.

### 6.2 Identifiers & ordering
- `EventId` = content hash of the normalized event — the immutable internal identity used as a reference target (LotId origin, decision-event targets). Distinct from `source_ref`, which is the **dedup key** used to detect re-imports of the same real-world row (§9). Identity for "is this the same event we already have?" is `source_ref`; `EventId` is what other events point at.
- `LotId = (origin_event_id: EventId, split_sequence: u32)`; `split_sequence` assigned deterministically by the projection as lots split — a function of event history, never random.
- **Canonical event order** (a named function in `btctax-core`, documented as tax-affecting): primary `utc_timestamp`, secondary fixed **source priority** (Swan > Coinbase > Gemini > River), tertiary `source_ref` lexicographic. An order-independence test is mandatory.

### 6.3 Entities
- **`Wallet`** (a basis pool): `Exchange { provider, account }` | `SelfCustody { label }`. *(No `External` variant — unreconciled outflows are a projection state, not a location.)*
- **`LedgerEvent`** (immutable): `id`, `utc_timestamp`, `original_tz`, `source`, `source_ref` (dedup key), `wallet`, `payload`.
- **`Lot`** (derived): `lot_id`, `wallet`, `acquired_at`, `original_sat`, `remaining_sat`, `usd_basis`, `basis_source`. Splits on partial disposal/transfer.
- **`Disposal`** (derived): the `Dispose` event mapped to the specific consumed lots, each with proceeds, basis, gain/loss, ST/LT, and propagated `basis_source` (for the future 8949 basis code).

### 6.4 Event taxonomy
**Imported events** (from exports):
- `Acquire { sat, usd_cost, fee_usd, basis_source }` — buy / purchased receive. Lot basis = `usd_cost + fee_usd`.
- `Income { sat, usd_fmv: Option<Decimal>, fmv_status: FmvStatus, kind: Mining|Staking|Interest|Airdrop|Fork }` — ordinary income; new lot at FMV.
- `Dispose { sat, usd_proceeds, fee_usd, kind: Sell|Spend|Gift|Donation }` — taxable; net proceeds = `usd_proceeds − fee_usd`.
- `TransferOut { sat, fee_sat: Option<i64>, dest_addr?, txid? }` / `TransferIn { sat, src_addr?, txid? }` — movement; unclassified until linked.
- `Unclassified { raw }` — a parsed row that does not map unambiguously to a payload (e.g., Coinbase `Order`). Imported (so the file is not rejected) but inert: it holds no lots and is surfaced as a blocker until resolved. *Never guessed into a real payload.*

**Decision events** (user inputs; append-only):
- `TransferLink { out_event, in_event_or_wallet }` — confirms a non-taxable self-transfer.
- `Reclassify { transfer_out_event, as: Dispose{Spend|Gift|Donation} }`.
- `ManualFmv { event, usd_fmv }`.
- `SafeHarborAllocation { per_wallet: [{wallet, allocated_sat, allocated_usd_basis}], method: ProRata|SpecificLots }` (dated 2025-01-01).
- `ClassifyRaw { target: Unclassified_event, as: <imported payload> }` — resolves an `Unclassified` row into a real event.
- `VoidDecisionEvent { target_event_id }` — nullifies a prior decision before folding (precedence rule, §10).

- **`FmvStatus`** = `ExchangeProvided | PriceDataset | ManualEntry | Missing`.
- **`BasisSource`** = `ExchangeProvided` (e.g., Swan transfers) | `ComputedFromCost` (buy: cost+fee) | `FmvAtIncome` (income lot) | `CarriedFromTransfer` (self-transfer) | `SafeHarborAllocated` (2025 allocation). Captured at lot creation and propagated `Lot → Disposal` for the future Form 8949 basis code (Phase 2).

## 7. Projection / lot engine
### 7.1 Contract
`project(ordered_events) -> LedgerState { lots, holdings_by_wallet, disposals, income_recognized, pending_reconciliation, blockers }` — pure, deterministic, no I/O, rebuilt from scratch. `income_recognized` captures ordinary-income amounts (for Phase-2 reporting); `blockers` aggregates open items: `fmv_missing`, `unclassified`, `import_conflicts`, and unmatched transfers.

### 7.2 Determinism
Events folded in canonical order (§6.2). Re-importing in any order yields identical `LedgerState` (tested).

### 7.3 Fold rules
- `Acquire` → new lot (basis = cost + acquisition fee; `basis_source` carried).
- `Income` → if FMV known, new lot at FMV (ordinary income recorded); if `Missing`, record the lot's existence but add to `fmv_blockers` and **block gain computation** for any disposal consuming a lot whose history includes an unresolved-FMV income event.
- `Dispose{…}` → consume from the event's wallet pool by method (Phase 1 default **FIFO**; structure supports specific-ID selection later) → emit `Disposal` (per-lot net proceeds/basis/gain + ST/LT via TP4); split lots → new `split_sequence`.
- `TransferOut` (unclassified) → lots leave the wallet into `PendingReconciliation`.
- `TransferLink` → move the exact lots to the destination wallet carrying basis + `acquired_at`; **fee treatment (c) / TP8**: consume `fee_sat` at zero proceeds (non-taxable transfer cost), carry full basis to remaining sats.
- `Reclassify` → fold the referenced `TransferOut` as a `Dispose`.
- `VoidDecisionEvent` → drop the targeted decision before folding.

### 7.4 Pool modes & the 2025 transition (TP6)
- Pre-2025: `UniversalPool` — one aggregate BTC basis pool (sufficient because past years are locked/filed; we only need the correct aggregate carried forward).
- `SafeHarborAllocation` @ 2025-01-01 distributes remaining basis into per-wallet pools.
- 2025+: `PerWalletPool` — FIFO/specific-ID operate strictly within each wallet; self-transfers carry lots between pools.

## 8. Encrypted storage & session (`btctax-store`)
- **On disk:** one `vault.pgp` (Sequoia-PGP, encrypted to the app-managed keypair; private key passphrase-protected, stored separately and itself passphrase-encrypted). Decrypted layout: `[schema_version: u32][SQLite serialized image]`.
- **Open:** `flock(LOCK_EX|LOCK_NB)` on the vault → **fail fast** if held (NFR7/C3) → decrypt → `mlock` the plaintext buffer (best-effort; **warn loudly** if it fails — see §15 R1) → `deserialize` into in-memory SQLite. No plaintext `.db` on disk (NFR2).
- **Save:** serialize → prepend `schema_version` → encrypt → write `vault.pgp.tmp` (same dir/filesystem) → `fsync` → atomic `rename()` over `vault.pgp`, rotating the prior to `vault.pgp.bak` (NFR3/C1).
- **Migration:** `migrate(version, bytes)` invoked on open; ships with the identity migration (I8).
- **Session:** unlock once per session; the unlocked DB + key live in `mlock`ed, `zeroize`-on-drop memory; re-lock on exit/timeout.
- **Key lifecycle:** `init` generates the keypair, sets the passphrase, and **forces a key-backup step**, documenting that key/passphrase loss = unrecoverable data loss. `export-snapshot` is the recovery escape hatch.

## 9. Ingestion & adapters (`btctax-adapters`)
**Pipeline (per file):** detect source (filename + header signature) → strip preamble → parse rows (handle CRLF) → normalize → assign dedup key → validate **all** → append **all-or-nothing** (m4).

**`Adapter` trait:** `detect`, `parse → Vec<RawRecord>`, `normalize → Vec<LedgerEvent>`. Each adapter's **module doc must state, with a passing test on real fixtures**: its dedup-key fields, gross-vs-net proceeds, and fee placement (I2/I5).

**Dedup key (`source_ref`):** prefer on-chain **txid**; else the source's native row id; else semantic `(source, utc_ms, type, sat[, row_index])`. **Conflict rule:** a re-imported row whose `source_ref` matches an existing event but whose **content differs** (a corrected re-export) is **not** silently ignored or silently overwritten — it is surfaced as a **conflict blocker** for the user to accept (supersede via a decision event) or reject. Identical content → idempotent no-op.

### 9.1 Per-source mapping (from real sample inspection)
- **Coinbase** (yearly CSV; 3-line preamble): dedup = native `ID`. `Buy`→`Acquire` (basis = `Total` incl. fees; = `Subtotal`+`Fees`). `Sell`→`Dispose{Sell}` (gross proceeds = `Subtotal`; fee = `Fees`). `Send`→`TransferOut` (dest = Recipient Address); `Receive`/`Exchange Deposit`/`Pro Deposit`→`TransferIn`; `Withdrawal`/`Exchange Withdrawal`/`Pro Withdrawal`→`TransferOut`. **`Order`→ flagged-needs-classification** (ambiguous: negative BTC, $0 fee — likely Pro trade/conversion; must not be guessed). Non-BTC assets (ETH/LTC/…) dropped (FR2).
- **Gemini** (wide multi-asset XLSX→parsed; running balances): dedup = `Tx Hash` (on-chain) else `Trade ID`. `Buy`→`Acquire`, `Sell`→`Dispose{Sell}` (`USD Amount`=gross, `Fee (USD)` separate). `Debit`(BTC)→`TransferOut`; `Credit` is **mixed** → USD credit = USD deposit (cash only, no BTC event) / BTC credit = `TransferIn`. Use `BTC Balance` for reconciliation (FR9). Non-BTC columns ignored.
- **River** (universal CSV; may be CRLF): no native id → semantic dedup. `Buy`→`Acquire` (basis = `Sent`+`Fee`; `Sent` is principal **excluding** fee). `Income`/`Interest`→`Income{kind}` (BTC received, **no USD** → FMV from dataset). `Withdrawal`/BTC-sent rows→`TransferOut`.
- **Swan** (3 files/account: trades, transfers, withdrawals): dedup = `Transaction ID` (txid); **all three files ingest as one batch, cross-deduped by txid** so an on-chain send isn't double-counted (m2). `trades`→`Acquire`. `transfers` supplies authoritative `USD Cost Basis` + `Acquisition Date` → `basis_source = ExchangeProvided`. `withdrawals`→`TransferOut`.

### 9.2 Price dataset
Bundled daily BTC/USD history behind `PriceProvider` (trait in `btctax-core`, impl in adapters). Daily close is the consistent FMV method; lookups keyed by event date.

## 10. Reconciliation & decision precedence
- The engine surfaces `PendingReconciliation` outflows. The reconciler proposes candidate matches (amount±`fee_sat`, configurable time window, matching address/txid) against `TransferIn` events and known wallets. The user confirms a self-transfer (`TransferLink`) or classifies (`Reclassify` → spend/gift/donation). Unmatched stay flagged.
- **Precedence:** decision events are append-only; conflicts are resolved by explicit `VoidDecisionEvent` (a later decision does not implicitly override — it must void first). This keeps the projection deterministic and auditable (I6).

## 11. CLI surface (`btctax-cli`)
`init` · `import <files…>` · `reconcile` · `wallets` · `holdings [--at DATE]` · `lots [--wallet W]` · `events [--filter]` · `fmv` (list/resolve missing) · `allocate-2025` · `verify` · `export-snapshot` · `backup-key`. All run inside the session/unlock; mutating commands trigger an atomic save.

## 12. Error handling & integrity
- Typed errors (`thiserror`) with row/column/file context. Parse failure aborts the whole file (all-or-nothing).
- **Nothing silent:** BTC-only drops are counted/reported; `Missing` FMV, `Unclassified` rows (e.g., Coinbase `Order`), import conflicts (§9), and unmatched transfers all surface as **actionable blockers** via `LedgerState.blockers`.
- `verify`: per-wallet holdings vs source running balances (Gemini/Swan) + global sat conservation; reports drift and all open blockers.
- Save is atomic; lock contention yields a clear "another instance is running" message.

## 13. Review traceability (architecture review round 1)
All findings folded: **C1** §8 atomic write+bak · **C2** §8 mlock+warn (R1) · **C3** §8 flock · **C4** §6.2 LotId · **I1** §6.2 ordering+test · **I2** §9 dedup+per-adapter docs · **I3** §6.4/§7.3 Option FMV+FmvStatus+gating · **I4** TP8/§7.3 treatment (c) · **I5** §9 per-adapter gross/net invariant+test · **I6** §6.4/§10 VoidDecisionEvent · **I7** TP6/§7.4 pool modes+safe harbor · **I8** §8 schema_version+migrate · **m1** §6.1 ROUND_HALF_EVEN · **m2** §9.1 Swan batch dedup · **m3** §6.3 no External/PendingReconciliation · **m4** §9 all-or-nothing import · **n1** §6.3 basis_source propagation · **n2** §9.2 PriceProvider in core · **n3** §15 R3 Sequoia version.

## 14. Testing & acceptance ("green" = full suite passes + 0 Critical/0 Important)
TDD throughout. Required tests:
- **Per-adapter** on real (redacted) fixtures: dedup key, gross/net + fee, preamble/CRLF, BTC-only drop count, each event-type mapping; Swan 3-file cross-dedup; Coinbase `Order` → flagged.
- **Property tests** (lot math): basis conservation, no negative remainders, Σ lot basis == pool basis, rounding sums stable.
- **Determinism**: shuffled import order → identical `LedgerState` (NFR4).
- **Idempotency**: re-import same/corrected file → no duplicates.
- **Atomic save / crash**: interrupted save leaves the vault as the previous-or-new full image, never partial.
- **Concurrency**: a second instance is refused.
- **Encryption**: encrypt→decrypt round-trip equality; wrong passphrase fails cleanly; mlock-failure warns.
- **FMV gating**: a disposal of a lot with an unresolved-FMV income ancestor is blocked, not silently zero-cost.
- **Golden end-to-end**: full real sample set → pinned snapshot of holdings + disposals.

## 15. Risks & assumptions
- **R1 (C2) mlock is best-effort.** It needs `RLIMIT_MEMLOCK`/privileges; full swap protection isn't guaranteed. Treated as defense-in-depth; the app warns and the setup docs recommend encrypted/disabled swap. *Honest limitation, not an absolute guarantee.*
- **R2 Adapter semantics to confirm by test (not yet 100% certain):** Coinbase `Order` rows' true meaning; River file CRLF + exact Income/Interest row shape; Gemini `Credit` USD-vs-BTC sub-typing edge cases. Each becomes a per-adapter fixture test (§14); unresolved cases surface as blockers, never silent.
- **R3 (n3)** Pin a specific Sequoia-PGP version (encryption-only; no keyserver/async) before first build.
- **A1** Past tax years were already filed (TP6 / tax-year scope decision) — Phase 1 produces no historical forms.
- **A2** The four sources above are the complete venue set for now; new sources = new adapters (the trait makes this additive).

## 16. Out of scope / future phases
Phase 2 (forms: filled IRS 8949 + Schedule D PDFs, current+future years). Phase 3 (goal-driven optimizer: specific-ID selection, HIFO/LIFO, loss harvesting, bracket/NIIT-aware after-tax modeling). Non-BTC assets, GUI/web UI, online pricing, multi-user.

## 17. Suggested implementation order (input to the plan)
A hint for `writing-plans`; the plan owns the final phasing. Build order favors testing each layer before the next depends on it:
1. **`btctax-store` safety primitives first:** atomic write + `.bak` (C1), `flock` (C3), encrypt/decrypt round-trip, `mlock`+warn (C2), `schema_version`+`migrate` (I8). These gate everything; test crash/concurrency/round-trip explicitly.
2. **`btctax-core` identity & ordering:** money/time conventions (§6.1), `EventId`/`LotId` (C4), canonical ordering + order-independence test (I1), event taxonomy. No fold logic yet.
3. **`btctax-core` projection:** fold rules, FIFO, holding period, fee treatment (c), FMV gating, pool modes + safe harbor — with property + determinism + idempotency tests.
4. **`btctax-adapters`:** one source at a time (Swan first — it carries authoritative basis; then Coinbase, Gemini, River), each with its fixture tests (dedup/gross-net/fee/BTC-filter) and the `PriceProvider` dataset.
5. **Reconciliation + CLI:** matching, decision events, the command surface, `verify`, golden end-to-end over the real sample set.
