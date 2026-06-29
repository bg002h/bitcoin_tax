# FOLLOWUPS — bitcoin_tax (TaxApp)

Open/!resolved action items (STANDARD_WORKFLOW §4). Each: what · why · status · pointer.

## btctax-core whole-branch fixes (2026-06-29) — both Important findings resolved

- **I-1 — `ReclassifyOutflow → Dispose` on-chain `fee_sat` silently dropped (FIXED).**
  Added `fee_sat: Option<Sat>` to `Op::Dispose`; `OutflowClass::Dispose` arm now passes
  `t.fee_sat`; native `EventPayload::Dispose` passes `None`. Fold arm calls `consume_fee`
  after principal and re-homes carry onto last disposal leg via `rehome_onto_disposal_leg`.
  Fee-sats are consumed; holdings no longer overstated; conservation is honest.
  KATs: `reclassify_dispose_fee_sat_treatment_c_conservation_honest` and
  `reclassify_dispose_fee_sat_treatment_b_mini_disposition` — both pass.

- **I-2 — Path-B seeded-lot `LotId` collision after post-2025 `SelfTransfer` (FIXED).**
  Added `PoolSet::init_split_counter(origin, next)` and called it in `seed_transition`'s
  Path-B arm after pushing seed lots, setting `next_split[allocation_id] = seed.len()`.
  Later `bump_split(allocation_id)` returns `seed_len` (not 0), so relocated fragments get
  fresh unique split sequences.
  KAT: `path_b_seeded_lot_relocation_no_lotid_collision` — all LotIds unique, conservation
  balanced after partial relocation of a seeded lot.

- **Phase-2 refinement note:** The precise fee-sat disposition treatment when a
  `TransferOut` is reclassified as Dispose is a TP8-adjacent Phase-2 refinement (the Phase-1
  TP8 treatment is applied correctly per the existing TreatmentC/B config; any further
  guidance-specific nuance is deferred).

## btctax-adapters (Plan 3) — confirmed real schemas folded into §9.1 (2026-06-29)
- **CROSS-CRATE GAP — inbound `TransferIn` cannot carry cost-basis / acquisition-date (record clearly).**
  Swan `transfers` `deposit` rows carry **`USD Cost Basis` + `Acquisition Date`**, and Coinbase `Receive` /
  Gemini `Credit`(BTC) inbound rows may carry basis context, but core's
  `TransferIn { sat, src_addr?, txid? }` has **no field to hold a cost-basis or acquisition-date**. So at
  ingest every inbound on-chain row becomes a **plain `TransferIn`** and the exchange-supplied basis/date are
  **dropped from the event**. They must be **re-supplied by reconciliation (Plan 4)** — e.g. a
  `ClassifyInbound` decision (`GiftReceived{donor_basis, donor_acquired_at, …}`) or a future
  `ClassifyInbound`-style "external-acquisition" decision that records basis+date for an externally-sourced
  inbound. For a confirmed **self-transfer** the source lot is authoritative anyway (the Swan basis is only
  relevant for externally-sourced coins), so no data is lost there. **Candidate fix (Phase-2):** a
  reconciliation-hints side-table (or extra optional fields on `TransferIn`) so the adapter can persist the
  exchange-provided basis/date as a *hint* the reconciler can accept, instead of re-keying it by hand. —
  OPEN (Plan 4 reconciliation / Phase-2). — adapters §9.1 / plan FOUND GAP.
- **Swan withdrawals `source_ref` — native-vs-semantic owner question.** The confirmed withdrawals schema
  carries a `Transaction ID` column, but per the owner it is **not a stable per-row id** (the schema-only
  doc shows the column but not values; cf. Swan-trades' present-but-empty `Tag`). The adapter therefore
  treats withdrawals as **id-less** (synthesized `(source, direction, utc_ms, type, sat)` + occurrence_index,
  §6.2). If the withdrawals `Transaction ID` turns out to be stable/unique, switch to a native ref (one-line
  change). — OPEN (owner confirm). — adapters §9.1 / plan Schema-items.
- **Swan `Total/Transaction USD` purchase-cost semantics.** Swan transfers `purchase`→`Acquire` uses
  `Transaction USD` (principal) + `Fee USD` (fee), with `Total USD` as the basis cross-check (`Total ==
  Transaction + Fee`); confirm by fixture once real values are available. — OPEN (confirm). — adapters §9.1.
- **Coinbase internal-move default.** `Exchange/Pro Deposit/Withdrawal` (Coinbase↔Coinbase-Pro) are routed to
  `Unclassified` (likely self-transfers, but user-confirmed via reconciliation rather than auto-`TransferIn`/
  `TransferOut`). Confirm this conservative default is desired. — OPEN (owner confirm). — adapters §9.1.
- **XLSX-float→decimal precision bound; id-less `occurrence_index` file-order fragility** (River, Swan trades,
  Swan withdrawals, Gemini `Credit`/`Debit`) — both already noted; carry forward. **Pin** the resolved
  `csv`/`calamine`/`rust_xlsxwriter` versions + re-verify the `calamine::Data` variant list after first build.
  — OPEN. — plan Notes for Plan 4.

## Deferred to later phases (out of Phase-1 scope by design)
- **Forms generation (Phase 2):** filled IRS 8949 + Schedule D PDFs; §170(e) charitable-deduction computation (FMV vs basis); Form 8283 (>$5k qualified appraisal — §170(f)(11)(C), CCA 202302012); Form 709 routing for gifts. — *Phase 1 captures the metadata (FMV, ST/LT, appraisal-required, donor carryover) so Phase 2 can compute.* — OPEN (Phase 2). — tax-review N1/M-(donation), spec §16.
- **Rate/limit mechanics (Phase 2/3):** 0/15/20% (§1(h)), 3.8% NIIT (§1411), $3,000 loss limit + carryforward (§1211/§1212). — Confirmed safe to defer (downstream of per-lot basis/gain/ST-LT). — OPEN (Phase 2/3). — tax-review "Positions confirmed".
- **Self-employment tax routing (Phase 2):** business-vs-hobby mining → Schedule SE (Notice 2014-21 A-9). — *Phase-1 ledger tags `Income{Mining, business: bool}`; Phase 2 routes.* — OPEN. — tax-review N1.
- **Optimizer (Phase 3):** goal-driven specific-ID/HIFO/LIFO/loss-harvesting, bracket/NIIT-aware. — OPEN. — spec §16.
- **Non-BTC scope:** fork-coin income (e.g., 2017 BCH airdrop, RevRul 2019-24) and non-BTC dispositions are OUT of BTC-only scope and must be handled separately. — Acknowledged, not covered. — OPEN/won't-do-in-foundation. — tax-review M4.

## Deferred — precise Phase-2 tax refinements (Phase-1 over-approximates safely)
- **Qualified-appraisal trigger precision.** Phase 1 flags `Donate.appraisal_required` on FMV>$5k (a safe over-flag). The precise §170(f)(11)(C) trigger is a **claimed deduction > $5,000**, aggregating similar items in a year (§170(f)(11)(F)); for §170(e)-reduced property (≤1-yr / ordinary-income) the deduction is limited to **basis**, so a high-FMV short-term donation with basis ≤ $5k would not trigger an appraisal. Compute the exact trigger in Phase 2. — OPEN (Phase 2). — tax-review R3 N2.
- **§1015(d) gift-tax basis increase.** A donee's basis is bumped by gift tax paid attributable to net appreciation (§1015(d)). Rare for personal BTC gifts (mostly under the annual exclusion); omitted in Phase 1, noted for completeness. — OPEN (won't-do unless needed). — tax-review R3 N3; spec §15.

## btctax-store — whole-branch fix I-1 (owner-only perms) — deferred hardening
- **M-1: `open`/`recover_target` bak-on-corrupt.** `recover_target` restores from `.bak` only when the target is MISSING, not when it is present-but-corrupt. Consider retrying from `.bak` on decrypt/decode failure — but must NOT retry on `WrongPassphrase` (caller error, not corruption). Deferred hardening; overlaps the kill-mid-save fuzz-harness item. — OPEN. — I-1 fix follow-on.
- **M-2: save-path plaintext not zeroized.** The `db_to_bytes`/`encode_blob` `Vec`s produced during `save()` hold plaintext before encryption and are not zeroized on drop. Within the accepted R1 bound (SQLite heap already holds plaintext all session). Future: wrap in `SecretBuf`/zeroize after `encrypt_to`. — OPEN. — I-1 fix follow-on.
- **M-3: Windows owner-only perms — verify under CI.** All four sinks (`vault.key`, `vault.pgp`, `export_snapshot`, `backup_key`) now use the non-Unix ACL-inheritance path (no explicit DACL). Verify under Windows CI that the written files are not world-readable. — OPEN (CI). — I-1 fix follow-on.

## btctax-store (Plan 1) — deferred hardening (non-blocking; plan is review-green)
- **Password zeroization (Task-3).** Resolved: `sequoia-openpgp::crypto::Password` wraps `Encrypted`, which stores the plaintext in a `Protected` buffer. The `Protected` type implements `Drop` with `memsec::memzero` — the ciphertext (encrypted plaintext) IS zeroized on drop. The `salt` field in `Encrypted` is NOT explicitly zeroized, but it is a key-derivation salt, not the actual secret. Confirmed — Password zeroizes (Protected buffer). — RESOLVED (2026-06-28). — Task-3.
- **OS-crash mid-first-create residual.** A `kill -9`/power-loss between the `vault.key` write and the first `vault.pgp` rename leaves `vault.key` present + `vault.pgp`/`.bak` absent → `create`→`AlreadyExists`, `open`→`Io(NotFound)`; manual key deletion needed (no committed data lost). In-process failures are cleaned up. Add an OS-level kill-mid-save fuzz harness and/or treat "key present, pgp+bak absent" as a half-created vault to auto-repair. — OPEN. — plan-review R3 M2.
- **Lock file persists after a failed/`AlreadyExists` create** (lock-first; conventional flock pattern, lock files are never unlinked). Harmless. — WONTFIX/ack. — plan-review R3 N1.
- **Sequoia/S2K pin (R3) — CONFIRMED by Task-0 spike:** sequoia-openpgp `1.21` resolved to `1.22.0`; backend `crypto-nettle`. Spike confirmed secret-key S2K = `Iterated { hash: SHA256, hash_bytes: 65011712 }` (i.e. `0x3E00000`, max OpenPGP work factor, ~354 ms) — no Argon2 in 1.22, strongest available = high-work-factor iterated-salted SHA-256, satisfying spec §8. Both primary key and subkey carry this S2K. Revisit if a future Sequoia exposes Argon2 or a public S2K-work-factor setter. — RESOLVED/confirmed (2026-06-28). — plan-review R2/R3 + Task-0.
- **nettle 4.0 system incompatibility (CONCERN, non-blocking for now):** The dev machine has system nettle 4.0, but `nettle-sys-2.3.2` + `nettle-7.5.0` require nettle 3.x API (functions removed/renamed, SHA3 init symbols gone, digest callback arity changed). Build workaround: extracted cached `nettle-3.10.2-1.1-x86_64_v3.pkg.tar.zst` from pacman cache to `/tmp/nettle-3.10.2/`, set `PKG_CONFIG_PATH=/tmp/nettle-3.10.2/pkgconfig-custom LD_LIBRARY_PATH=/tmp/nettle-3.10.2/usr/lib` when running cargo. This workaround is session-local and NOT committed. Future task: either (a) wait for a new `nettle`/`nettle-sys` crate supporting nettle 4.0, (b) install nettle 3.x system-wide, or (c) switch to `crypto-rust` backend (pure Rust, no system lib dependency) for CI portability. Per task-0-brief, no silent backend switch; this is an explicit concern. — OPEN. — Task-0 report.
- **Two on-disk artifacts** (`vault.pgp` + `vault.key`) and the vault is **encrypted but not signed** — documented deviations from §8's single-artifact wording (NFR2 still holds; `vault.key` is S2K-encrypted). Sign-on-save is a future option. — ack. — plan-review R1 M2/M8.

## btctax-store — cross-platform + crypto-rust (user decisions 2026-06-28)
- **Target OS = Linux + macOS + Windows (NFR8).** Store crate abstracts OS primitives: single-instance lock via `fs2` (flock/LockFileEx); secret-memory lock via `rustix` mlock (Unix) / `windows-sys` VirtualLock (Windows); atomic save via `std::fs::rename` (POSIX atomic / Windows MoveFileEx-replace, with the fsync'd `.bak` as the safety net). Spec NFR8 + §8 + plan Tasks 0/4/5/6 updated. — RESOLVED (decision). — user OS choice.
- **Crypto backend = `crypto-rust` (pure Rust)** — supersedes the earlier `crypto-nettle` choice. Reasons: (a) the dev box's nettle 4.0 is incompatible with `nettle-sys` (the Task-0 hack is no longer needed/used); (b) NFR8 cross-platform (Windows can't use nettle). `crypto-rust` needs no system crypto lib on any OS. **Security trade-off accepted:** Sequoia labels RustCrypto variable-time / "not recommended for general use"; acceptable for local at-rest single-user encryption (no network/oracle exposure). `allow-variable-time-crypto` enabled. The Task-0 nettle-4.0 concern above is **SUPERSEDED** by this switch. — RESOLVED (decision). — user backend choice.
- **Cross-platform validation:** Linux is the dev/test OS; Windows/macOS code paths are `cfg`-gated and compile-checked but executed under per-OS CI (set up later). — OPEN (CI). — NFR8.
- **crypto-rust builds clean (no system crypto lib, nettle hack unused):** `cargo build -p btctax-store` + `cargo test --test smoke` pass with `features = ["crypto-rust", "allow-variable-time-crypto", "allow-experimental-crypto"]` and no `PKG_CONFIG_PATH`/`LD_LIBRARY_PATH` set; S2K = `Iterated{SHA256, hash_bytes=65011712}` confirmed unchanged under crypto-rust. `allow-experimental-crypto` is required (sequoia-openpgp build script gates RustCrypto behind it). — RESOLVED (2026-06-28). — Task-0 crypto-rust switch.
- **File-lock crate: `fs2` 0.4 (dormant ~2017) vs `fd-lock` (maintained).** We use `fs2::try_lock_exclusive`; on Windows it relies on Rust ≥1.64 mapping `ERROR_LOCK_VIOLATION(33)`→`WouldBlock` (MSRV 1.74 satisfies). `fd-lock 2.x` normalizes this explicitly and is maintained — preferred swap if Windows CI shows any mapping issue or if the dormant dep becomes a supply-chain concern. — OPEN (monitor; swap candidate). — plan-review delta M-1.

## btctax-core (Plan 2) — review-green; deferred Minors to address at implementation
- **TP8(c) loss-basis cross-lot edge (tax m1).** When a fee spans lots and `relocated.last()`/last removal-leg is non-dual-basis but the fee originates on a dual-basis received-gift lot, the carry's `loss_basis` fragment is dropped. Effect: future loss-zone basis understated by fee-cents (taxpayer-conservative); gain basis fully conserved. — OPEN (Task 11). — core tax-review R2 m1.
- **TP8 fee exact-boundary holding-period attribution (tax m2).** When principal consumes exactly to a lot boundary, the fee basis (from the next, later-acquired lot) rides the earlier relocated lot's holding period by a few cents. De-minimis; total basis conserved. — OPEN (Task 11). — core tax-review R2 m2.
- **Degenerate `principal==0` fee'd transfer (tax m3).** Carry is silently dropped (no relocated lot/leg) with no blocker — unreachable for real TransferLink/gift (principal>0). At implementation: assert principal>0 or raise `uncovered_disposal` instead of dropping. — OPEN (Task 11). — core tax-review R2 m3.
- **2025-transition seed timezone straddle (eng Minor).** The boundary seed fires on the UTC-sorted timeline while pool routing + `universal_snapshot` use the tax-date; a sub-day offset straddling 2025-01-01 (e.g. a +12:00 post-2025 event vs a −05:00 pre-2025 event) can fold a pre-2025-tax-date event after the seed → fails safe (`uncovered_disposal` or stranded lot), but `universal_snapshot` won't match the real fold's pre-seed residue. At implementation (Task 12): partition the timeline at the **tax-date** boundary (or seed lazily on first wallet route) + add a reversed-offset KAT. — OPEN (Task 12). — core eng-review R2 Minor.
- **`allocation_voids` declaration (eng Nit).** Referenced (pass-1 step 1a, deferred from Task 7) with `.target`/`.void_id` but its struct/collection isn't formally declared in the plan; declare it explicitly at implementation. — OPEN (Task 7/12). — core eng-review R2 Nit.

## Standing notes / decisions (informational)
- **PGP KDF tradeoff (user-mandated PGP retained).** Engineering review suggested age / XChaCha20-Poly1305+Argon2id as simpler with a stronger KDF; **declined — PGP is a hard user requirement.** Mitigation: protect the app-managed private key with the strongest S2K the chosen Sequoia version supports (Argon2 S2K if available, else high-work-factor iterated-salted S2K). — RESOLVED (decision) / monitor. — eng-review YAGNI, spec §8/§15.
- **TP8 self-transfer fee = treatment (c) default, config-switchable to (b) mini-disposition.** User-mandated default; do not flip. Contrary signal: §1.1012-1(h)(2)/(h)(4) (fees-in-crypto have disposition consequences in the *taxable-exchange* context; no on-point guidance for a pure self-transfer). — RESOLVED (decision). — spec TP8, memory `self-transfer-fee-treatment-c`.
- **Daily-close FMV is an approximation** of the "date and time of dominion & control" standard (RevRul 2023-14). Documented convention; revisit if higher precision is needed. — RESOLVED (decision) / monitor. — spec §9.2, tax-review M3.
- **Pre-2025 lot method = FIFO (legal default).** If the taxpayer's *filed* pre-2025 returns used a different method, the reconstructed carryforward basis must be reconciled — surfaced as a `verify` note/blocker, not silently assumed. — OPEN (runtime reconciliation). — spec §7.4, eng-review I-2.
- **Source-priority tiebreak (Swan>Coinbase>Gemini>River)** is arbitrary-but-stable for same-instant cross-source FIFO ties; documented as such. — RESOLVED (decision). — spec §6.2, eng-review n-2.
- **Id-less-source `source_ref` fragility (River).** For sources without native ids, `source_ref = (source, direction, utc_ms, type, sat)` with a last-resort `occurrence_index` for exact duplicates in one import. Two known limitations: (a) `occurrence_index` shifts if a corrected re-export inserts an earlier row; (b) a re-export that edits a *constituent* field (e.g., `sat`) changes the `source_ref`, so it is NOT detected as a "same source_ref, changed content" conflict and cannot be auto-`SupersedeImport`-ed (old event orphans, new appears). — OPEN (documented limitation; prefer time-resolution / native ids where possible). — spec §6.2, eng-review round-2 m-2/m-5.
- **Daily-close FMV (labeled M3)** — see the "Daily-close FMV is an approximation" note above; flagged as the chosen convention vs the date-and-time dominion-and-control standard. — RESOLVED (decision). — tax-review M3.

## Resolved in SPEC v0.2 (folded round-1 reviews)
See the spec's "Fold record (v0.2)" section for the 1:1 mapping of each Critical/Important to its fix. Round-1 reviews: `reviews/spec-review-phase1-tax-round-1.md`, `reviews/spec-review-phase1-engineering-round-1.md`, `reviews/architecture-review-phase1-foundation-round-1.md`.

- **N-2 (export_snapshot silently overwrites snapshot.sqlite):** Current behaviour matches the brief (no mention of rotation); future improvement: timestamped filenames (e.g. `snapshot-20260628T120000Z.sqlite`) to avoid clobbering a previous export. **Windows owner-only perms** for both `export_snapshot` and `backup_key` rely on user-profile directory ACL inheritance (no explicit DACL set); verify under Windows CI that the written files are not world-readable.

## btctax-adapters plan — deferred Minors (review-green; 2026-06-29)

Non-blocking items raised during the round-1 review of `btctax-adapters` (IP-1 and all code-level Minors folded inline into the plan on 2026-06-29). These are deferred observations for implementation time or later phases.

- **River `Income`→`IncomeKind::Reward` documentation + `business: false` immutability (tax M1/M2).** River's `Income` tag maps to `IncomeKind::Reward` (non-business yield/reward); `business: false` is hard-coded at ingest. At implementation, add a module-doc note that `business: false` is immutable at the adapter layer — the Plan-4 reconciler cannot flip it without a re-import. If the owner's River income is business income (e.g., from professional mining operations), the `IncomeKind` / `business` mapping must be confirmed before implementing the River parser. — OPEN (confirm at River-parser implementation). — adapters tax-review M1/M2.
- **Swan zero-sat-withdrawal defensive counter (tax Nit).** The Swan withdrawals arm currently increments `dropped_no_btc` for a `sat == 0` row (defensive guard; Swan is BTC-only). At implementation, consider whether a zero-sat Swan withdrawal should be counted under a separate `skipped_zero_sat` field rather than the FR2 `dropped_no_btc` counter, since the two cases are semantically different. — OPEN (implementation note). — adapters tax-review Nit.
- **Coinbase internal-move = Unclassified decision (tax-review endorsed).** `Order` + `Exchange/Pro Deposit/Withdrawal` → `Unclassified` is the correct conservative default. The tax reviewer explicitly endorsed keeping this (over auto-routing to `TransferIn`/`TransferOut`), since these Coinbase↔Coinbase-Pro internal moves require user confirmation via reconciliation. No change to the plan; noted here so Plan-4 docs know the decision is reviewed and intentional. — RESOLVED (decision retained; no action needed). — adapters tax-review.
- **Swan withdrawals `Transaction ID` stability — treated id-less; confirm later.** The withdrawals file carries a `Transaction ID` column but the adapter treats it as non-stable (semantic `source_ref`). If confirmed stable/unique, switch to native ref (one-line change in `Swan::normalize` withdrawals arm). Cross-referenced with the existing schema-items entry above. — OPEN (owner confirm). — adapters plan Schema-items / tax-review Nit.

## btctax-core (Task 0) — dependency versions pinned for reproducibility
- btctax-core pinned `rust_decimal` 1.42.1 / `rust_decimal_macros` 1.40.0 (independent Cargo entries; `dec!` literals binary-compatible with the 1.42 `Decimal`) / `time` 0.3.51 — R3 pin record.
