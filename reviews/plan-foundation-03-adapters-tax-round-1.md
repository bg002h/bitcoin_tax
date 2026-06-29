# Tax-mapping Review — IMPLEMENTATION_PLAN_foundation_03_adapters.md (§9.1) — Round 1

- **Reviewer:** independent US-crypto-tax reviewer; verified the type→event mapping vs TP1–TP11 + confirmed real schemas + legal REPORT/ADDENDUM.
- **Date:** 2026-06-29
- **Verdict:** **Tax-correct + conservative — 0 Critical / 0 Important** (2 Minor, 1 Nit → FOLLOWUPS). Persisted per STANDARD_WORKFLOW §2.

## Confirmed correct
- **Acquisition** (Coinbase Buy: Subtotal+Fees=Total; Gemini Buy: USD Amount+Fee; River/Swan-trades: Sent+Fee; Swan purchase: Transaction USD+Fee USD=Total USD) → `Acquire`, basis = cost + acquisition fee (TP2). All column choices correct.
- **Disposition** (Coinbase/Gemini Sell) → `Dispose{Sell}` with GROSS proceeds + separate fee (lot engine nets per TP2). Correct.
- **Outbound BTC** (Coinbase Send/Withdrawal; River Withdrawal; Swan withdrawals; Gemini Debit) → `TransferOut`→pending_reconciliation. **CRITICAL CHECK PASSED: zero outbound auto-disposed / no fabricated gain** (TP7/TP8). User/reconciliation classifies.
- **Inbound BTC** (Coinbase Receive; Gemini Credit; Swan deposit) → `TransferIn` unknown-basis. **CRITICAL CHECK PASSED: no zero-basis Acquire / no phantom gain.**
- **Income** (River Income→Income{Reward}; Interest→Income{Interest}) → FMV-at-receipt (TP3), FR3-resolved, sat-bearing even on FMV-Missing. Correct.
- **Ambiguous → Unclassified** (Coinbase Order, Exchange/Pro Deposit/Withdrawal; Swan monthly_fee/prepaid_fee; Swan trades BTC-on-sent edge): no taxable treatment ever guessed. Correct.
- **FR2 BTC-only:** only no-BTC-leg rows dropped; ambiguous BTC-side → Unclassified.
- **Swan deposit basis gap** (USD Cost Basis/Acquisition Date → no TransferIn home) explicitly flagged → reconciliation (Plan 4), not silently wrong.

## Minor (→ FOLLOWUPS)
- M1 River `Income`→`IncomeKind::Reward`: safe catch-all; what River's "Income" tag covers undocumented (refine for SE-tax labeling if needed). No wrong tax outcome.
- M2 `business:false` hardcoded on River income, immutable after ingest (Income isn't ClassifyRaw-able). Only matters for Phase-2 SE-tax. No wrong tax outcome.

## Nit
- N1 Swan zero-sat withdrawal → dropped_no_btc counter (defensive; Swan is BTC-only).

## Owner-decision: Coinbase Exchange/Pro Deposit/Withdrawal — Unclassified vs TransferIn/Out
Both paths yield the same non-taxable self-transfer outcome (no legal risk differential). **Recommendation: KEEP Unclassified** — these are internal Coinbase↔Coinbase-Pro book-entries (no txid/address/on-chain event), so TransferIn/Out's on-chain matching is a poor fit; Unclassified is more accurate + forward-safe; direction-awareness (In/Out) already coded enables a future CLI batch-classify. (Endorsed; → FOLLOWUPS, not a blocker.)

## Verdict
§9.1 mapping is tax-correct and conservative. 0 Critical / 0 Important — clears the gate.
