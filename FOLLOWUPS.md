# FOLLOWUPS — bitcoin_tax (TaxApp)

Open/!resolved action items (STANDARD_WORKFLOW §4). Each: what · why · status · pointer.

## Deferred to later phases (out of Phase-1 scope by design)
- **Forms generation (Phase 2):** filled IRS 8949 + Schedule D PDFs; §170(e) charitable-deduction computation (FMV vs basis); Form 8283 (>$5k qualified appraisal — §170(f)(11)(C), CCA 202302012); Form 709 routing for gifts. — *Phase 1 captures the metadata (FMV, ST/LT, appraisal-required, donor carryover) so Phase 2 can compute.* — OPEN (Phase 2). — tax-review N1/M-(donation), spec §16.
- **Rate/limit mechanics (Phase 2/3):** 0/15/20% (§1(h)), 3.8% NIIT (§1411), $3,000 loss limit + carryforward (§1211/§1212). — Confirmed safe to defer (downstream of per-lot basis/gain/ST-LT). — OPEN (Phase 2/3). — tax-review "Positions confirmed".
- **Self-employment tax routing (Phase 2):** business-vs-hobby mining → Schedule SE (Notice 2014-21 A-9). — *Phase-1 ledger tags `Income{Mining, business: bool}`; Phase 2 routes.* — OPEN. — tax-review N1.
- **Optimizer (Phase 3):** goal-driven specific-ID/HIFO/LIFO/loss-harvesting, bracket/NIIT-aware. — OPEN. — spec §16.
- **Non-BTC scope:** fork-coin income (e.g., 2017 BCH airdrop, RevRul 2019-24) and non-BTC dispositions are OUT of BTC-only scope and must be handled separately. — Acknowledged, not covered. — OPEN/won't-do-in-foundation. — tax-review M4.

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
