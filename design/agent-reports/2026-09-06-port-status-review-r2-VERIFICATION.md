# VERIFICATION ledger — port-status re-verification r2 (2026-09-06-port-status-review-r2.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `e39ef155`.

| # | claim | check | verdict |
|---|---|---|---|
| N1 | `claims_no_draft`'s NO FINAL acceptance has no plant in either direction | the plant list (mine): NO DRAFT plants only | **TRUE** |
| N2 | the #8 plant compares `partial.keys()` against `surface` instead of `shrunk` | the assertion (mine) | **TRUE** |
| N3 | the variable name undersells what it matches | by reading | **TRUE** |
| gate | 3/3; NO FINAL emits on `port-status 2025 2026` (18/18 rows, 0 NO DRAFT) | the reviewer's run | **accepted** |

**Verdict: 3/3 TRUE.**
