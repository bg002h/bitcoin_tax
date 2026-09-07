# VERIFICATION ledger — the interview T1 seam review (`2026-09-07-build-interview-T1-review.md`, 1C/1I/4M/3N)

Controller's machine-check at `aeb4db8e`, before the fold.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | C1 | `import_return_inputs` writes the TOML's `answer_log` / `answer_log_history` straight to the row (a second writer), while it normalises `CarryProvenance` a few lines away | `cmd/tax.rs`: the `CarryProvenance` normalisation block exists in `import_return_inputs`; no `answer_log` normalisation beside it | HOLD |
| 2 | I1 | `supersede_stale_prompts` has no production caller | grep: only its definition and tests | HOLD |
| 3 | M2 | the GENERATED fixture has no test that it matches its emitter | the reviewer reproduced the non-idempotence and diagnosed it (the report's D9 diagnosis corrected) — accepted | accepted |
| 4 | M1, M3, M4, N1–N3 | wording / doc / floor claims | read | HOLD |

Disposition: C1 and I1 block — folded next by one opus agent with the Minors/Nits; then ONE sonnet
re-verification (S6); then T2.
