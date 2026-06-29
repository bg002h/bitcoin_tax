# Engineering Review — IMPLEMENTATION_PLAN_foundation_03_adapters.md — Round 1

- **Reviewer:** independent senior Rust reviewer; verified vs confirmed real schemas + btctax-core API + parsing/arithmetic correctness.
- **Date:** 2026-06-29
- **Verdict:** **0 Critical / 1 Important (IP-1)** + 7 Minor + 4 Nit. IP-1 folded; net green (IP-1's calamine-API resolution verified at Task-2 build). Persisted per STANDARD_WORKFLOW §2.

## IMPORTANT
### IP-1 — Gemini XLSX `Data::DateTime` path can drop the time component (→ wrong utc_ms)
calamine may return numeric date cells as `Data::DateTime` (not `Data::Float`); `dt.to_string()` could yield a date-only/precision-lost string → wrong `utc_ms` for Gemini Credit/Debit SEMANTIC source_refs (native Trade/Order-ID rows unaffected). The string-cell fixture wouldn't catch it. **FOLDED:** `Data::DateTime` arm now extracts the Excel serial (`dt.as_f64()`) and routes through the identical `Data::Float→parse_timestamp_flex(serial)` path; Task-0 first-build checklist confirms the accessor; Task-6 Gemini fixture adds a `write_number` serial date cell to exercise the real path end-to-end.

## MINOR (folded)
- M-1 Gemini fixture used write_string for dates (real path untested) → write_number row added.
- M-2 `read_xlsx` "no worksheet" used `PriceDataset` error → new `EmptyXlsx` variant.
- M-3 calamine 0.26 `Data` variant list (DateTime/DateTimeIso/DurationIso) → Task-0 build-verify checklist.
- M-4 Coinbase `Order` relied on `_` arm → explicit `"order" => Unclassified` arm.
- M-5 Gemini `Deposit Destination`→`TransferIn.src_addr` is the destination (not sender) addr → doc note for the Plan-4 reconciler.
- M-6 `IncompleteSwanBatch` mis-named (triggers on unrecognized role) → renamed `UnrecognizedSwanRole`.
- M-7 `Decimal::from_scientific` existence in rust_decimal 1.36 → Task-0 build-verify (else drop or_else; from_str parses scientific).

## NIT (folded N-3/N-4)
- N-3 Gemini detection-order comment rationale fixed. N-4 integration test asserts named events / prints offending Unclassified IDs (not a global count). N-1/N-2 observations only.

## Confirmed correct (self-review claims verified)
No `// OPEN` constants remain; all 4 parsers' match arms exhaustive + conservative `_`. Coinbase 3-line preamble skipped (line-3 identity never parsed); Swan transfers/withdrawals 2-line preamble (header row 3); Swan trades/River row 1. `excel_serial_to_utc` epoch 1899-12-30 / anchor 25569 math verified incl. the 1900 leap-year-bug absorption. NFR5: no float money (Decimal::from_str; XLSX floats shortest-round-trip stringified then exact-parsed; fractional-sat guard). §6.2 source_ref: native ids where present, semantic+occurrence_index else (deterministic). All btctax-core import paths valid against lib.rs (EventPayload/EventId::canonical/Source/PriceProvider/conventions). Privacy: synthetic fixtures only.

## Verdict
Engineering-sound at 0 Critical / 0 Important with IP-1 folded. Clears the gate; IP-1's calamine specifics verified at Task-2 implementation/build (per-task review + whole-branch review are the net).
