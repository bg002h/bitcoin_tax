# VERIFICATION ledger — the interview spec's one review (`2026-09-07-spec-interview-review.md`, 1C/14I/9M/4N)

Controller's machine-check at `1312d72b`, before the fold. Most findings are about the spec's own
text (design judgement); the checkable ones:

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | C1 | R2.2's `covered_by` join admits an `Advisory` on an income line and checks only that the variant exists | the spec's R2 text names `covered_by` and lists `Advisory` among the coverage kinds with no direction rule | HOLD |
| 2 | I11 | §7 has no task for the year-N+1 opener R10 part 4 describes | grep of §7 for "N+1 / opener / re-interview" → 0 | HOLD |
| 3 | I13 | T8's kill names TY2025 fixture rows on a map with no dependents-grid cells | `forms/2025/f1040.map.toml` names no dependent cells (0), `forms/2024/f1040.map.toml` does | HOLD |
| 4 | I9 | `prompt_hash` is stored and never read | the spec mentions it in the schema and the kill, with no mismatch rule | HOLD |
| 5 | N4 | off-by-one cites | `year_readiness.rs:27` vs `:28` etc. — accepted as read | HOLD |
| 6 | I1–I8, I10, I12, I14, M1–M9 | design-judgement findings with quoted spec text and minimal changes | read; each cites a spec section that says what the reviewer quotes | HOLD |

Disposition (S6): fold EVERYTHING into `SPEC_interview.md` r2 by one opus agent under
`BRIEF-fold-spec-interview.md`; NO second prose round — C1 tightens a rule (a direction rule for
income lines) and does not change the design's shape. The build's seam reviews are the gate.
