# Review — SPEC_foundation_v0_1.md, Engineering (Round 1)

- **Artifact:** `design/SPEC_foundation_v0_1.md`
- **Reviewer:** independent engineering/spec reviewer, fresh context (adversarial). Cross-checked tax positions against `legal/research/REPORT_us_btc_tax_TY2025-2026.md`.
- **Date:** 2026-06-28
- **Verdict:** NOT yet sound to proceed to a plan — fold Criticals + re-review first. Architecture right; event-identity scheme unsound; several tax-model errors contradict our own legal research.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2. (Companion: tax-correctness review, separate file.)

---

# Independent Review — `SPEC_foundation_v0_1.md` (bitcoin_tax Phase 1)

**Reviewer posture:** adversarial, greenfield (spec on its merits). I verified the cited artifacts exist (`legal/research/*`, `reviews/*`, `SOURCES.md`, `SHA256SUMS`) and cross-checked the spec's tax positions against the project's **own** legal report `legal/research/REPORT_us_btc_tax_TY2025-2026.md`. Several of the strongest findings below are not just internal-consistency issues — they are places where the spec **contradicts its own verified legal research**.

Overall the architecture (event sourcing, pure projection, crate split, i64 sats / `Decimal` USD, atomic blob + flock) is sound and the §13 traceability is *mostly* real. But three of the prior review's findings are addressed only **superficially** (I7 most seriously), and the spec introduces new correctness holes around event identity and the tax model. It is **not yet a sound foundation to proceed to a plan** without the Critical fixes.

---

## Critical

### C-1. `EventId = content hash` is neither guaranteed-unique nor canonicalized — it corrupts `LotId` and breaks determinism
§6.2 defines `EventId` as "content hash of the normalized event," used as the **unique reference target** and as `LotId = (origin_event_id, split_sequence)`. Two problems, and they are in tension:

- **Collision:** §9 itself admits identical-content rows exist (the semantic dedup key appends `row_index` "to disambiguate collisions"). Two genuinely distinct fills — same source, same ms timestamp, same `sat`, same `usd_cost`, same `fee`, same wallet (routine on Coinbase: one order, multiple fills) — produce the **same content hash → same `EventId` → same `LotId` origin**. `split_sequence` disambiguates splits of *one* lot, not two distinct identical lots. Lot math silently corrupts.
- **Canonicalization:** a content hash is only stable if the encoding is canonical. `rust_decimal` `1.50` vs `1.5` (same value, different scale), optional-field encoding, field order, and timestamp encoding all change the hash. The spec mandates `ROUND_HALF_EVEN` for math but specifies **no canonical encoding for hashing**. Without it, a corrected/cosmetically-different re-export yields a different `EventId` for the "same" event → `LotId` instability → violates NFR4 determinism and the "stable, inspectable, referenceable LotId" that C4 was supposed to deliver.

The spec is ambiguous on whether `source_ref` is part of the hashed content, and **every** resolution has a failure mode (include it → unstable across re-exports because `row_index` shifts; exclude it → identical rows collide). Pin: (1) exactly which fields are hashed, (2) a canonical byte encoding, (3) a uniqueness guarantee across distinct real rows, and (4) how identity survives corrections (see I-1). This is the single most load-bearing under-specification in the spec.

### C-2. Gifts and charitable donations are modeled as gain-realization events — this is tax-wrong and contradicts the project's own legal report
TP1 declares "every disposition (sell/spend/gift/donation) is a realization event," marked **Settled**, and §6.4 models `Dispose{Gift|Donation}` with `usd_proceeds` and per-lot gain/loss (§6.3 Disposal). The project's own `REPORT_us_btc_tax_TY2025-2026.md` says the opposite:

- Gifts: **carryover/dual basis + holding-period tacking (§1015/§1223)** — *no gain recognized by the donor* (report lines 184, 193).
- Charitable donations: **FMV deduction (if >1yr) + qualified appraisal (Form 8283)** — *not a proceeds-minus-basis gain* (report lines 193-194).
- The report further flags these topics as **"unverified-by-the-workflow"** (line 29), so even TP1's "Settled" uncertainty flag is overstated.

A gift has **no amount realized** under §1001, so computing `proceeds − basis` produces **phantom gains** in the `Disposal` records (`income_recognized`/disposals are emitted in Phase 1 per FR4). Fix: `Sell|Spend` stay taxable; `Gift|Donation` must remove lots and record FMV + carryover/deduction info but recognize **zero gain to the taxpayer** (routing to 709 / 8283 is Phase 2). The data model bakes the error in now, so it must be corrected at the spec level.

### C-3. The 2025 safe-harbor allocation event loses acquisition dates / holding periods → wrong ST vs LT after 2025 (I7 only superficially addressed)
`SafeHarborAllocation { per_wallet: [{wallet, allocated_sat, allocated_usd_basis}], method }` (§6.4) carries only an **aggregate sat + aggregate basis per wallet**. But the legal report (line 134) requires allocating **unused-basis *lots*** to accounts, and per-wallet lots retain their **original acquisition dates** (tacking, §1223). With only an aggregate basis number per wallet, the engine **cannot determine ST vs LT** for post-2025 disposals of allocated lots → wrong character → wrong tax. The event must allocate lots (each with `acquired_at` and basis), not a blended pair. Additionally, nothing validates conservation (Σ`allocated_sat` == remaining held sat; Σ`allocated_usd_basis` == remaining pool basis at 2025-01-01). §13 maps I7 to "TP6/§7.4 pool modes," which exist structurally, but the **substance** of the allocation is not correctly modeled.

### C-4. The BTC-only filter (FR2) can silently drop the BTC leg of crypto-to-crypto trades — a missing taxable disposition
The report is emphatic: crypto-to-crypto exchange is a **taxable disposition** of the asset given up (lines 73-81; §1031 unavailable). FR2 says "drop non-BTC rows (ETH/BCH/LTC/…) at normalization." A BTC→ETH trade is a **BTC disposition** (proceeds = FMV of BTC); an ETH→BTC trade is a **BTC acquisition** at FMV. If such a row is classified "non-BTC" and dropped, a taxable BTC disposition vanishes — understating gains. The spec never addresses crypto-to-crypto BTC legs; "drop non-BTC rows" as written invites the omission. (Coinbase `Convert`/`Order` → `Unclassified` is a partial mitigation, but the rule must explicitly state that the **BTC side of any trade is retained**, and that "non-BTC" means only rows with no BTC leg.)

---

## Important

### I-1. Corrections change `EventId`, orphaning decision events and lot references; and `import_conflicts` cannot be derived purely from the event log
The conflict rule (§9) resolves a corrected re-export by "supersede via a decision event." But if identity is content-hash (C-1), the corrected event has a **new `EventId`**, so every `TransferLink`/`Reclassify`/`ManualFmv`/`VoidDecisionEvent` that targets the old `EventId`, and every `LotId` derived from it, is **orphaned**. The supersession path's reference-remapping is undefined.

Two further problems collide here:
- **Conflict vs all-or-nothing (m4):** FR1 says import is "all-or-nothing in a single transaction," but §9 says a conflicting row is "surfaced as a conflict blocker for the user to accept or reject." If a corrected file has 499 idempotent rows + 1 changed row, does the whole import abort (all-or-nothing) or partially apply with one blocker? These two rules contradict.
- **Purity violation:** FR4/§7.1 say `blockers` (incl. `import_conflicts`) are derived by the **pure projection from events**. But a *rejected/unaccepted* conflicting row is, by definition, **not appended** — so the conflict cannot be reconstructed from the event log. Either the conflict must be recorded as an event, or `import_conflicts` is an import-time state that breaks the "everything derives from the event log" guarantee (NFR6).

### I-2. Pre-2025 lot-selection method is unspecified, yet it determines the 2025 starting basis; and `UniversalPool` granularity is ambiguous
§7.4 specifies FIFO/spec-ID "within each wallet" only for **2025+**. It says **nothing** about the lot-selection method for pre-2025 disposals against the `UniversalPool` — yet the remaining aggregate basis carried into the safe-harbor allocation is *entirely a function of that method*. FIFO is the legal default (report line 44), so assuming it is defensible, but: (a) the spec never states the assumption, and (b) if the taxpayer's **filed returns** used a different method historically, the reconstructed carryforward basis — and therefore all future gains — is wrong. Also: "one aggregate BTC basis pool" reads as a single blended number, which is incompatible with per-lot FIFO and per-lot FMV gating (§7.3); clarify that `UniversalPool` tracks lots but un-partitioned by wallet.

### I-3. `Reclassify` (TransferOut → Dispose{Spend|Gift|Donation}) cannot produce a computable disposal
`Reclassify { transfer_out_event, as: Dispose{...} }` (§6.4) carries **no proceeds/FMV and no fee handling**. The original `TransferOut` has no USD and only `fee_sat`. A reclassified spend needs `usd_proceeds` = FMV of BTC at the event (PriceProvider lookup or manual entry), and a defined treatment of the on-chain `fee_sat` — which, for a spend (not a self-transfer), the report treats as its **own taxable mini-disposition** (lines 82-83), *different* from the self-transfer treatment (c). As specified, a reclassified disposal cannot be computed.

### I-4. TP8 (self-transfer fee = non-taxable, treatment (c)) contradicts the project's own legal report
The report explicitly distinguishes "own-wallet transfers (non-taxable)" from "**transfer-fee-in-crypto events (the fee is a taxable mini-disposition)**" (lines 38, 74-75, 82-83). TP8/§7.3 lock treatment (c) — fee_sat consumed at **zero proceeds, non-taxable** — i.e., the opposite. The architecture review recommended (c) on practitioner grounds, but it did so **without** the legal finding in view, and the spec folded (c) without reconciling the conflict. It is labeled "swappable," which limits data-model risk, but the **default contradicts cited primary authority**. Reconcile explicitly (and confirm against the user's intended filing position); arguably the report's mini-disposition treatment should be the default or a first-class supported rule.

### I-5. Decision-event conflict is undefined when the user does not void first (I6 only half-addressed)
§10: "a later decision does not implicitly override — it must void first." This defines the *intended* workflow but not the engine's behavior in the **illegal state** it permits: a `TransferLink A→B` and a `Reclassify A→Spend` both present, neither voided. Append-only logs cannot prevent this. The projection must define what happens (most cleanly: an un-voided conflicting decision set is itself a **blocker**), otherwise the "deterministic and auditable" claim (I6) is unmet. As written, both decisions "apply," which is a contradiction state with no resolution.

### I-6. Holding-period date is ambiguous (UTC vs `original_tz`) and can flip ST/LT at the one-year boundary
TP4/report line 87 make ST/LT a **date** comparison ("strictly after the one-year anniversary"). §6.1 computes in UTC but preserves `original_tz`. A late-evening local purchase can be a different *calendar date* in UTC, shifting the anniversary by a day and flipping ST↔LT at the boundary. The spec must state which date is authoritative for holding-period day-counting. For a money-critical domain this is a real correctness ambiguity.

### I-7. NFR2 ("no plaintext DB file *ever* written") directly contradicts FR10 `export-snapshot`
NFR2/§8 state the only on-disk artifact is the encrypted vault and "no plaintext DB file ever written." FR10/§8 define `export-snapshot` as writing the **decrypted ledger (SQLite + CSV)** to disk. As written this is a flat contradiction. Reword NFR2 to "no plaintext DB written *automatically/implicitly*; explicit user-invoked export is the sole, documented exception."

### I-8. FMV-missing gating is incomplete and its interaction with pre-2025/pending is undefined
§7.3 blocks *disposal* gain computation when a lot's history has an unresolved-FMV income event, but an `Income` with `Missing` FMV also has an **unknowable ordinary-income amount** — `income_recognized` must be gated/blocked too, not just downstream disposals. Separately, the interaction of `PendingReconciliation` sats with the **2025 snapshot** is undefined: if an outflow is still unreconciled at 2025-01-01, are its sats in the universal pool, held, or excluded? The safe harbor allocates *held* units, so this matters for conservation.

### I-9. Acceptance tests pin behavior, not correctness — no known-answer tests for a money-critical domain
§14's golden end-to-end "pins a snapshot of holdings + disposals," and the property tests check **conservation** (Σ basis, no negative remainders) — but nothing checks that a specific gain or ST/LT **value** is *correct*. A golden snapshot will happily freeze wrong numbers (e.g., the C-2 phantom gift gains). Add a small set of **hand-computed known-answer scenarios**: buy→1yr+1day→sell (LT); same-day buy/sell (ST); self-transfer with fee (basis conservation under TP8); income lot with FMV; a 2025 allocation with mixed-vintage lots checking post-allocation ST/LT (C-3). Also: the idempotency test fixtures must include **cosmetic re-export variation** (whitespace, decimal scale, CRLF) to actually exercise the C-1 canonicalization requirement.

---

## Minor

- **m-1. mlock/zeroize scope is narrower than the real plaintext footprint.** §8 mlocks "the plaintext buffer," but projection structures (`Vec<LedgerEvent>`, lots, `String`/`Decimal` fields) and SQLite's internal working heap are normal heap and can swap; `String`/`Decimal`/SQLite buffers also can't be reliably zeroized. R1 honestly frames mlock as best-effort, which mostly covers this, but §8's wording ("the unlocked DB + key live in mlocked, zeroize-on-drop memory") overstates. Either mlock the global SQLite allocator arena or state the protection is partial.
- **m-2. `row_index` in the semantic dedup key is fragile across re-exports.** A corrected export that inserts an earlier row shifts every subsequent `row_index`, breaking idempotency for collision rows. It's the best available tiebreak, but note the limitation and prefer time-resolution improvements where possible.
- **m-3. Intra-day ordering relies on `source_ref` lexicographic.** With Coinbase per-day timestamp resolution (I1), many same-day buys fall to `source_ref` lexicographic order, which (for opaque IDs) need not reflect true sequence — deterministic but possibly mis-ordered for FIFO and for the LT boundary. Acknowledge the limitation.
- **m-4. Migration model is muddy.** §8 versions the outer `[schema_version][SQLite image]` bytes, but schema evolution actually lives at three levels: the outer layout, SQLite DDL, and the serde encoding of `LedgerEvent` payloads inside SQLite rows. `migrate(version, bytes)` operating on raw bytes can't evolve enum variants without deserializing. I8's header exists, but *what is versioned and how migrations transform* needs pinning.
- **m-5. Orphan/unknown-basis `TransferIn` unhandled.** A deposit with no matching internal `TransferOut` (BTC acquired outside the tracked venues) has **unknown basis**. Reconciliation targets unclassified *outflows*; unmatched *inflows* (basis holes) aren't clearly surfaced. A2 assumes the four venues are complete, but the first funding event and external receipts are plausible; `verify` sat-conservation may catch it, but make it an explicit blocker.
- **m-6. Swan 3-file batching needs CLI-level grouping.** §9.1 requires the three Swan files to ingest "as one batch, cross-deduped by txid," which means `import <files…>` must group files by detected source before processing. Not stated at the CLI/§11 level.
- **m-7. `ClassifyRaw` ergonomics undefined.** Resolving an `Unclassified` row requires the user to supply a full typed imported payload (`as: <imported payload>`). How a complete `Acquire`/`Dispose` payload is entered on the CLI is unspecified.
- **m-8. Orphan `.tmp` cleanup after a crash mid-save** is unspecified (atomic rename protects `vault.pgp`, but a leftover `vault.pgp.tmp` should be reaped on next open).

## Nit
- **n-1.** §8 "private key passphrase-protected … and itself passphrase-encrypted" is redundant/confusing wording.
- **n-2.** The source-priority tiebreak (Swan > Coinbase > Gemini > River) is **tax-affecting and arbitrary**; document that it's arbitrary-but-stable (or give a rationale), since it can change FIFO lot consumption on same-instant cross-source ties.
- **n-3.** The conservation identity "in == out + held + fees" (FR9) should explicitly include `PendingReconciliation` sats, which are neither held nor disposed.
- **n-4.** Process: `FOLLOWUPS.md` (a required workflow artifact per CLAUDE.md) does not yet exist; not a spec defect, but the open items above are its natural first entries.

---

## YAGNI / over-engineering to cut
- **Reconsider PGP/Sequoia for a single-user local vault.** It brings asymmetric-key indirection (data→pubkey, privkey→passphrase), key-lifecycle/`backup-key` ceremony, **async/API churn** (acknowledged in n3/R3), and OpenPGP's **weak S2K** as the passphrase KDF. For "encrypt one blob with a passphrase," a passphrase-derived AEAD (`age`, or XChaCha20-Poly1305 + **Argon2id**) is simpler, fewer deps, no async, and a *stronger* KDF. No multi-recipient requirement exists in scope. This is the highest-value simplification. [NOTE: PGP is a USER-MANDATED requirement — surface the KDF/security concern to the user; do not unilaterally swap.]

---

## §13 traceability verification (claimed 1:1)
**Genuinely addressed:** C1, C2 (with m-1 caveat), C3, I1 (with m-3 caveat), I2 (with I-1 caveat), I3 (with I-8 caveat), I4 (mechanism present; **but TP8 default conflicts with own research — I-4**), I5, I8 (with m-4 caveat), m1, m2, m3, m4 (with I-1 conflict-vs-atomicity caveat), n1, n2, n3.
**Superficially addressed (substance missing):**
- **I7 (Critical gap):** pool modes exist, but the safe-harbor event loses acquisition dates/holding periods (**C-3**) and the pre-2025 method/granularity is unspecified (**I-2**).
- **I6 (partial):** void mechanism defined, but the un-voided-conflict state is undefined (**I-5**).
- **C4 (partial):** `LotId` shape defined, but `EventId` uniqueness/canonicalization that it depends on is unsound (**C-1**).

---

## Verdict
**Not yet a sound foundation to proceed to a plan — proceed only after the Criticals are folded and re-reviewed.** The architecture is right and most of round-1 was correctly absorbed, but the spec has (a) an unsound event-identity scheme that undermines the very determinism/LotId guarantees it claims, and (b) tax-model errors that **contradict the project's own verified legal research** and would produce wrong numbers. These are spec-level, not mere implementation details.

## Three highest-leverage fixes
1. **Pin event identity (C-1, I-1).** Specify exactly which fields `EventId` hashes, a canonical encoding (incl. `Decimal` scale), a uniqueness guarantee across distinct identical-content rows, and how identity + all references (`LotId`, decision-event targets) survive corrections/supersession. Everything deterministic rests on this.
2. **Fix the tax model against your own legal report (C-2, C-4, I-4, I-3).** Gifts/donations are not donor gain events; the BTC leg of crypto-to-crypto trades must be retained as BTC dispositions/acquisitions; reconcile the TP8 transfer-fee default with the report's mini-disposition position; give `Reclassify` real proceeds/FMV + fee semantics.
3. **Make the 2025 transition correct (C-3, I-2).** Model the safe-harbor allocation as a mapping of **lots (with acquisition dates + basis)** to wallets with conservation checks, and explicitly specify (and assume-document) the pre-2025 lot method that produces the carried basis — with a hand-computed known-answer test (I-9) proving post-2025 ST/LT is right.
