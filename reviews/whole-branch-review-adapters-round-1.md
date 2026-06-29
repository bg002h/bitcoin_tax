# Whole-Branch Review — `btctax-adapters` (e568dbb..c6efddf, 16 commits) — Round 1

- **Reviewer:** independent final whole-branch reviewer (most-capable model); cross-cutting correctness across all four parsers + shared ingest layer vs §9/FR2/FR3/NFR5/§6.2 + privacy.
- **Date:** 2026-06-29
- **Verdict:** **NOT ready to merge — 0 Critical, 2 Important** (both Gemini-specific, NEW cross-task findings the BTCUSD-only/positive-value per-task fixtures could not surface). Persisted per STANDARD_WORKFLOW §2.

## Headline
Crate well-built: clean layering (read→parse→normalize→match→ingest), real conservative-mapping discipline (no outbound auto-disposed, no inbound zero-basis Acquire, ambiguous→Unclassified), NFR5 honored, detection collision-free, privacy clean. Both Important findings are in the only XLSX source (Gemini), on real-export value shapes that the per-task fixtures never exercised.

## Critical — None.

## Important (merge-blocking)
### I-1 — Gemini `Buy`/`Sell` on BTC-quoted pairs (ETHBTC/BCHBTC) → wrong-direction + zero-basis
`gemini.rs` Buy/Sell arms key on `Type` alone and take `usd_amount = …unwrap_or(ZERO)`. The confirmed `Type` enum is {Buy,Sell,Credit,Debit}, so a crypto↔BTC trade on a BTC-quoted pair is necessarily Buy/Sell AND has a BTC leg → retained by FR2, not dropped. Two silent failures: (a) **direction inversion** — `Symbol=ETHBTC,Type=Buy` (buy ETH WITH BTC) disposes BTC but emits an `Acquire` of |BTC| sats; (b) **zero/garbage basis** — `USD Amount USD` empty for a non-USD pair → usd_cost/proceeds=0 → zero-basis lot (phantom gain) / zero-proceeds disposal. Contradicts FR2 ("crypto↔BTC = BTC disposition/acquisition at FMV") and the never-zero-basis/never-guess rule. §9.1 Gemini mapping doesn't address BTC-quoted pairs (spec gap). Gemini-only (River/Swan are BTC-only venues; Coinbase rows are single-asset with USD Subtotal).
**Fix:** gate `Buy/Sell→Acquire/Dispose` on `Symbol==BTCUSD` (or `USD Amount USD` present); otherwise emit `Unclassified` (conservative — user classifies the crypto-crypto BTC leg); never fall through to usd=ZERO. Update §9.1. Add an ETHBTC fixture.

### I-2 — Gemini USD sign convention unhandled (negative basis/proceeds)
`parse_usd` converts accounting-negative `(1,234.56)`→`-1234.56` and carries a leading minus through; no source `.abs()`-normalizes USD magnitude. If Gemini encodes outflows as negative/parenthesized (its cash columns conventionally do), a BTCUSD Buy gets negative `usd_cost` → core basis = usd_cost+fee goes negative; a Sell's net corrupts if `Fee (USD) USD` is parenthesized. Schema confirmation was schema-only (sign never verified); `parse_usd`'s paren-negative handling signals the author anticipated negatives but never neutralized them. All Gemini fixtures positive → untested. Gemini-specific (Coinbase Total=Subtotal+Fees self-checks positivity; River/Swan positive in practice).
**Fix (privacy-safe):** since `Type` fixes the role (Buy=cost, Sell=proceeds, fee=cost), `.abs()` the parsed USD magnitudes in the Gemini parser (or assert positivity + error). Add a negative/parenthesized-encoded fixture.

(I-1, I-2 are NEW — not in the recorded per-task list — exactly what BTCUSD-only positive fixtures couldn't surface.)

## Minor
- M-1 Gemini real `Data::DateTime` timestamp path unit-untested (tests only the `Data::Float` write_number path; real Gemini date cells carry a number-format → calamine returns `Data::DateTime`). Add a `write_datetime` fixture. (Task-2 M-2.)
- M-2 §13 type→event matrix test gaps (Coinbase Withdrawal/Exchange-Wd/Pro arms/unknown; River Income{Reward}; Gemini ETH/BCH-drop + non-BTCUSD + Sell-kind pin). Closing the Gemini half de-risks I-1/I-2.
- M-3 no explicit re-import reproducibility test for semantic source_refs (logic is deterministic — verified; HashMap used only for counting, drained by fixed adapter Vec).
- M-4 bundled price dataset is a 6-row test stub — FR3 returns Missing for real income dates until full daily-close history is bundled (pre-production task, not a code defect).
- M-5 Swan deposit→TransferIn drops USD Cost Basis+Acquisition Date (documented GAP; harmless for self-transfers; reconciliation re-supplies for external deposits).

## Nit
Documented basis cross-checks (Coinbase Total=Subtotal+Fees; Swan Total USD) not implemented (#[allow(dead_code)] consts); Unclassified always Direction::Trade (cosmetic); price-dataset dup-date last-wins; parse_btc_to_sat("")→Ok(0) untested; Gemini Credit src_addr=Gemini's own deposit addr (documented); US-locale MM/DD tried for all sources (latent landmine).

## Cross-cutting checks PASSED (1–9)
Mapping consistency/conservatism (except I-1); FR2 uniform (parsed_rows=events+dropped); FR3 (only River income; export→dataset→Missing; Missing→sat-bearing; never fabricated); §6.2 source_ref deterministic + collision-free; NFR5 no float money; preamble/identity-row safety (AND-matched signatures; PII rows never data); privacy clean (synthetic fixtures, no real reads, no PII); composition well-formed for core + deterministic aggregation; spec faithful except the §9.1 Gemini BTC-quote gap.

## Verdict (Round 1)
**Not ready to merge. 0 Critical / 2 Important (I-1, I-2)** — both Gemini, both cheap+conservative+privacy-safe fixes. The other three parsers + shared layers are merge-ready. Close I-1/I-2 WITH the Gemini test-coverage gaps (M-2 Gemini half), re-review per §2.

## Round 2 — fix re-review (commit bcd29ac) — BOTH CLOSED, GREEN
Independent re-review of the fix (diff c6efddf..bcd29ac). **I-1 and I-2 FULLY CLOSED; no new defect; NO Gemini coverage lost. 0 Critical / 0 Important / 3 Nit.**
- **I-1:** Gemini `Buy`/`Sell` now gate on `Symbol=="BTCUSD"` (case-insensitive) + a USD-present safety net; a BTC-quoted pair (ETHBTC) with a BTC leg → `Unclassified` (never zero-basis Acquire, never usd=ZERO). KAT `gemini_btcquoted_pair_buy_is_unclassified` panics if it sees an Acquire.
- **I-2:** Gemini USD magnitudes (`usd_cost`/`usd_proceeds`/`fee_usd`) abs-normalized in the Gemini parser (shared `parse_usd` unchanged); KAT `gemini_negative_usd_normalized_to_positive` pins -1000→1000, (900)→900.
- **No new defect:** normal positive BTCUSD Buy/Sell unaffected (`.abs()` no-op); Credit/Debit/`_`/FR2 arms unchanged.
- **No lost coverage:** the diff replaced the Buy/Sell match blocks inside one test with strictly-stronger assertions (incl. the newly-pinned `DisposeKind::Sell`) + added 3 new tests; ALL pre-existing Gemini assertions retained. (The earlier "139" was a prior over-count; authoritative workspace = 133 passed / 0 failed, clippy/fmt clean — Gemini suite GREW.)
- §9.1 + FOLLOWUPS updated (BTCUSD gate / BTC-quoted→Unclassified / abs-normalization + Phase-2 crypto↔BTC-pair FMV note). Nits: `has_usd` safety-net doc; no mixed-case-Symbol test; one test's "ETH-only" label cosmetic.

**Net status: 0 Critical / 0 Important — btctax-adapters GREEN, ready to merge.**
