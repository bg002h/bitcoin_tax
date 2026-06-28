# Review — Phase-1 Foundation Architecture (Round 1)

- **Artifact reviewed:** Phase-1 foundation design (brainstorm stage) — event-sourced core, crate split, domain model, encrypted store. (Not yet a written spec; design captured in the brainstorm conversation.)
- **Reviewer:** independent `feature-dev:code-architect` agent, fresh context (adversarial review).
- **Date:** 2026-06-28.
- **Verdict (summary):** Sound foundation; no structural rethink needed. But 4 Critical + several Important findings must be resolved before code (data-loss or wrong-tax-result risks).
- Persisted verbatim below before folding, per STANDARD_WORKFLOW §2.

---

# Phase-1 Architecture Review: bitcoin_tax

---

## Preamble: Framing

Before the findings: the core architecture choices are defensible. Event sourcing is a genuine fit here (reclassifications, the 2025 safe-harbor allocation, and bug-fix re-derivations all need it), the crate decomposition is clean, and the technology choices (i64 satoshis, rust_decimal for USD, Sequoia-PGP, SQLite) are appropriate. This is a foundation worth building on — but four findings below would produce data loss or wrong tax results if shipped as-is.

---

## Critical Findings

### C1 — Non-atomic blob write is a data-loss landmine

The design says "serialize + re-encrypt on save." The PGP-encrypted blob is the **only copy** of the user's data. If a write is interrupted — power loss, SIGKILL, disk-full — the output file is truncated or partially overwritten. There is no WAL, no redundancy. The result is unrecoverable data loss.

This is not a theoretical risk: disk-full during an encrypt-and-write of a growing blob is routine.

**Recommendation:** Write to a sibling temp file in the same directory (same filesystem as the target, so rename is atomic), then call `rename()` (POSIX-atomic). Additionally, keep one rolling `.bak` of the previous successful write. Both are straightforward. Neither requires crypto changes.

---

### C2 — Plaintext data in swap negates the PGP guarantee

The design claims PGP encryption protects data at rest. The decrypted SQLite lives in process heap and SQLite's internal `mmap` pages. On Linux with swap enabled, those pages can be written to disk as plaintext by the kernel — completely outside the process's control. A laptop stolen mid-session (or even post-session if the kernel hasn't reclaimed the pages) exposes all data.

This is especially acute here because the stated threat model is a **local-only, encrypted-at-rest** design. The encryption claim is the entire security story.

**Recommendation:** `mlock` the decrypted memory before populating it. In Rust with rusqlite you can call `sqlite3_config(SQLITE_CONFIG_HEAP, ...)` or lock the region returned by `sqlite3_serialize`. If mlock is unavailable or fails, the app must warn loudly. At minimum, document this in user-facing setup instructions and recommend encrypted swap or disabling swap. Do not let the security marketing outrun the implementation.

---

### C3 — No protection against concurrent instances causing silent data loss

Nothing in the design prevents two CLI invocations from running simultaneously. Both read the blob into memory, both modify, and the last to call save wins. The other session's work is silently discarded. On a single-user desktop this is easy to trigger: a background process holds a session while the user opens a second terminal.

**Recommendation:** Take an exclusive advisory lock (`flock(LOCK_EX | LOCK_NB)`) on the blob file immediately after opening it. If the lock fails, exit with a clear "another instance is running" error. Release the lock on clean exit. This is three lines of code and eliminates the hazard entirely.

---

### C4 — Lot ID scheme for split lots is undefined; specific-ID will break without it

The design identifies lots by their "origin event." A single origin event (say, a 100,000-sat buy) can be split into multiple residual lots by a partial sell and a partial transfer before the tax year in question. Each residual lot has the same origin event. Without a sub-identifier, the projection engine cannot distinguish them, and any future disposal event that tries to reference "the remaining 35,000-sat stub from that buy" is ambiguous or non-deterministic.

The design explicitly states the ledger must support specific identification. Specific-ID requires the user to name the exact lot at time of sale. That requires stable, inspectable, referenceable lot IDs that survive re-derivation from scratch.

**Recommendation:** Define Lot ID as `(origin_event_id: EventId, split_sequence: u32)` where `split_sequence` is incremented deterministically as the projection engine processes events in canonical order. The split sequence must be a product of the event history, never random. Specify this scheme explicitly before any implementation — it affects the Disposal event schema, the TransferLink event schema, and the future specific-ID UI.

---

## Important Findings

### I1 — Event ordering and FIFO determinism are underspecified

The dataset includes hundreds of DCA buys, many on the same day. Coinbase exports have per-day timestamp resolution for some records. FIFO lot selection depends on a total ordering of events. If two events share a timestamp and the tie-breaker is undefined or depends on import order, then re-importing events in a different order (e.g., importing year 2022 after year 2023) produces a different lot assignment and different computed gains. The tool would produce different tax results depending on the import sequence — a correctness failure.

**Recommendation:** Define canonical event ordering: primary = UTC timestamp, secondary = source priority (a fixed enum: e.g., Swan > Coinbase > Gemini > River for cross-source ties on the same instant), tertiary = source_ref lexicographic order. Encode this ordering as a named function in `btctax-core`, document it as a tax-affecting design decision, and add a test that verifies the same result regardless of import order.

---

### I2 — Dedup key design is underspecified for sources without stable IDs

"source + record id/hash" is not a usable specification for sources where rows have no native ID. Coinbase and Gemini rows have no stable primary key. Hashing the full row content is fragile: Coinbase re-exports the same period with minor formatting differences (whitespace, decimal precision). A full-row hash changes; the event appears as a new import and the original appears as an orphan.

This breaks the idempotent re-import guarantee, which is foundational. If users can't safely re-import a corrected export, the entire reconciliation workflow breaks down.

**Recommendation:** For each adapter, specify exactly which fields form the dedup key. Prefer on-chain txid when present (it is globally stable). For rows without txid, define a semantic key: `(source_name, utc_timestamp_ms, event_type_normalized, amount_sat)`. Document the collision scenario (two buys of the identical amount at the identical millisecond from the same source) and handle it explicitly (append a row-index suffix to distinguish them). This specification belongs in the adapter module doc, not left to the implementer.

---

### I3 — Income events with missing FMV cannot be stored; model must allow Option

`Income { usd_fmv: Decimal }` with a non-optional FMV will cause a parse/normalization failure for every income/interest event that lacks a USD value — and the data reality section explicitly notes these exist. These events cannot be deferred: they represent ordinary income with a tax basis of FMV at receipt (IRC §61), and without FMV they are incomplete but must still be ingested and flagged.

If the parser simply drops these events, the holding history is wrong and the resulting basis is wrong.

**Recommendation:** Change to `usd_fmv: Option<Decimal>`. Add a `fmv_status: FmvStatus` field to the event (variants: `ExchangeProvided | PriceDataset | ManualEntry | Missing`). Events with `FmvStatus::Missing` must surface as user-action-required blockers that gate gain computation — the projection engine should refuse to compute disposals when any prior income event in the affected lot's history has unresolved FMV.

---

### I4 — Transfer fee basis treatment is unspecified; three defensible options exist

`TransferOut { fee_sat: Option<i64> }` captures the mining fee, but the design says nothing about how the fee affects the transferred lot(s). The choices are: (a) fee reduces the cost basis of the transferred sats, (b) fee is a separate micro-disposal (not correct for self-transfers under current IRS guidance), (c) fee reduces the number of sats reaching the destination but the cost basis of those sats is unchanged. Treatment (c) is the most common practitioner position: the fee sats are "lost" in transit, not deductible and not a realization event for self-transfers.

Without picking one, different implementations within the same codebase will produce inconsistent results.

**Recommendation:** Commit to treatment (c) in the domain spec. Document the rationale. The projection engine must: (1) split the lot such that fee_sat worth of sats are "consumed" with zero proceeds (marking them as a non-taxable transfer cost), and (2) carry the full original basis to the remaining sats at the destination. This must be tested with an explicit scenario before implementation proceeds.

---

### I5 — Adapter normalization of gross vs net proceeds is a silent correctness risk

Gemini exports include maker/taker fees as a separate column but may embed them differently depending on the trade side. Coinbase exports gross proceeds with fee as a separate field. If an adapter misidentifies "net proceeds" as "gross proceeds" and then also subtracts the fee column, the proceeds are understated and the gain is understated. This is a per-source correctness invariant that must be verified per adapter.

The design says "normalize raw rows → canonical events" but doesn't flag this as a critical correctness checkpoint.

**Recommendation:** Require each adapter to have inline documentation of: (1) whether the source reports gross or net proceeds, (2) whether fees are embedded or separate, (3) a unit test with a known example that verifies the normalization produces the correct `{usd_proceeds_gross, fee_usd}` pair.

---

### I6 — Competing and voiding decision events have no defined precedence

The design allows reclassifying a TransferLink after the fact via new events. But if the user creates a TransferLink linking TransferOut A to TransferIn B, and later creates another event reclassifying TransferOut A as a `Dispose{Spend}`, both events exist in the append-only log. The projection engine must resolve this conflict. Without a defined rule, the result is non-deterministic or will depend on event log ordering in a way that is not transparent.

**Recommendation:** Define one of: (a) "last decision event by insertion order wins" — simple, but requires the engine to process all decision events and allow later ones to override; or (b) introduce `VoidDecisionEvent { target_event_id: EventId }` for explicit revocation. Option (b) is cleaner for audit purposes. Whichever is chosen, specify it now — this is a structural choice that affects the entire projection engine design.

---

### I7 — Pre-2025 projection semantics are ambiguous

Rev. Proc. 2024-28 safe harbor exists for taxpayers who did not track per-wallet before 2025. This tool has complete transfer history and can track per-wallet from the start. But if there are unmatched transfers (cold-storage sends with no corresponding import), some lots are stranded in an unknown location before 2025, and the per-wallet tracking is incomplete anyway.

The design says "2025 safe-harbor allocation as a decision event" but doesn't specify: what triggers the need for this event, what data it carries, or how the projection engine switches from "pre-2025 pooled" to "post-2025 per-wallet" semantics.

**Recommendation:** Define two explicit projection modes in the engine: `UniversalPool` (pre-2025 where per-wallet tracking is incomplete) and `PerWalletPool` (2025+). The Jan-1-2025 `SafeHarborAllocation` decision event takes the form `{wallet_id, allocated_sat, allocated_usd_basis, allocation_method: ProRata | SpecificLots}` and causes the projection to switch modes. Specify the data model for this event before implementation — it is a unique, one-time event type that the tax authority has defined a specific procedure for.

---

### I8 — No schema/serialization versioning in the blob

The encrypted blob will be written today and read in 2028. If any `LedgerEvent` payload variant changes (renamed field, added variant, changed type), old blobs will fail to deserialize. The design has no version header.

**Recommendation:** Include `schema_version: u32` as the first field of the decrypted payload (before the SQLite bytes). Implement a migration stub (`fn migrate(version: u32, blob: &[u8]) -> Result<Vec<u8>>`) from day one, even if the only implemented migration is "v0 is current, return as-is." The cost of adding this retroactively after the first schema change is much higher.

---

## Minor Findings

### m1 — Rounding convention unspecified

Proportional basis allocation during lot splits (`60,000 sat of a 100,000 sat lot carrying $4,723.17 basis`) requires rounding USD amounts. Over many splits across many years, the rounding strategy accumulates error and affects gain/loss totals. IRS doesn't mandate a specific method, but consistency across the application is required for audit defensibility.

**Recommendation:** Standardize on `ROUND_HALF_EVEN` (banker's rounding) throughout `btctax-core`. Add it to a `domain::conventions` module as a named constant so it cannot silently diverge across computation sites.

---

### m2 — Swan three-file correlation is unaddressed

Swan provides three separate files per account: trades, transfers, withdrawals. An on-chain withdrawal may appear in both the transfers file (as a send) and the withdrawals file (as a BTC exit). Without cross-file deduplication within a single Swan import batch, the same event gets double-ingested as two `TransferOut` events.

**Recommendation:** The Swan adapter must treat all three files as a single import batch, use on-chain txid as the primary dedup key across files, and emit a single canonical event per on-chain event.

---

### m3 — `External{unknown}` wallet is YAGNI and architecturally muddying

An `External` wallet accumulates unreconciled lots with no meaningful semantics — it is simultaneously "might be a spend," "might be a gift," and "might be a self-transfer to cold storage." Placing lots here pretends they have a location when they don't. It also conflates the holding-location concept with the reconciliation-status concept.

**Recommendation:** Remove `External` from the `Wallet` enum. Unreconciled `TransferOut` events should leave the lots in a `PendingReconciliation` state tracked by the projection engine, surfaced as a required user action. The wallet enum should only represent known, confirmed holding locations.

---

### m4 — Partial import atomicity depends entirely on dedup key correctness

A parse failure at row 400 of a 500-row file leaves 400 events in the log. On re-import, correct dedup keys allow clean completion. But if finding I2 is not resolved first, re-import may create duplicates for some rows and miss others, with no error surfaced.

**Recommendation:** Implement import as a transaction: parse the full file into a candidate event list, validate all events, then append all-or-nothing. This removes the dependency on dedup key correctness for recovery from partial imports.

---

## Nit Findings

### n1 — `basis_source` must propagate through the full data path

`Acquire { basis_source }` captures provenance but the design doesn't confirm this flows to Lot and then to the Disposal record. IRS Form 8949 requires a basis reporting code. If `basis_source` is present at ingestion but dropped during projection, it must be reconstructed later — which is harder than carrying it through from the start.

### n2 — Bundled price dataset interface belongs in core, not adapters

The price dataset is bundled in `btctax-adapters`, but FMV lookups are needed by the projection engine in `btctax-core` (e.g., for income events with missing FMV resolved against the dataset). Define a `PriceProvider` trait in `btctax-core`; implement it with the bundled dataset in `btctax-adapters`. This preserves the core's purity and makes the price source swappable in tests.

### n3 — `time` crate version and `Sequoia-PGP` async compatibility

Sequoia 1.x has significant API churn and some components require async runtimes. Confirm the specific Sequoia version and API surface (encryption-only, no keyserver interaction needed here) before the first compilation cycle.

---

## Summary: Missing Pieces vs YAGNI

**Missing and needed for a sound Phase-1 foundation:**
- Lot ID scheme (C4)
- Event canonical ordering definition (I1)
- Per-source dedup key specifications (I2)
- `FmvStatus` on income events (I3)
- Transfer fee basis treatment decision (I4)
- Decision event precedence / void mechanism (I6)
- Blob schema version header (I8)

**YAGNI — cut from Phase 1:**
- `External{unknown}` wallet (m3) — replace with `PendingReconciliation` state
- Projection caching — re-derive from scratch; add caching only when benchmarks show it's needed

---

## Overall Verdict

This is a sound foundation. The core architecture choices — event sourcing, pure projection engine, per-wallet basis tracking, the crate decomposition, and the technology selections — are all correct for this problem. The design does not need a structural rethink.

The blocking issues are implementation-level design gaps, not architectural errors. None of them require changing the event-sourcing model, the crate boundaries, or the storage strategy. They do require resolving before writing code, because C1/C2/C3 produce data loss and C4/I1/I6 produce wrong tax results.

**The three highest-leverage changes before speccing:**

1. **Atomic write + file lock (C1, C3):** Without these, every other design decision is irrelevant. These are two-hour implementation tasks; do them first and test them explicitly.

2. **Lot ID scheme + canonical event ordering (C4, I1):** These two are inseparable. Together they determine whether the projection engine produces stable, deterministic, auditable results. They must be fully specified as named types and documented algorithms before any `btctax-core` code is written.

3. **Per-source dedup key specification (I2):** The idempotent re-import guarantee is the cornerstone of the "safe to correct and re-ingest" workflow. It must be specified per adapter before any adapter is implemented — it cannot be left to each adapter author's judgment.
