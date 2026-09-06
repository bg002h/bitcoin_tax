# VERIFICATION ledger — spec-1099da r5 verification (2026-09-06-spec-1099da-review-r5.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `2d87db37`.

| item | claim | check | verdict |
|---|---|---|---|
| NEW-1 | 15 full `DisposalLeg` literals / 26 `..base_leg()` sites; the r4 ledger's "27" double-counted a doc-comment mention | `grep -rn '\.\.base_leg()'` excluding comment lines → 26 (re-run); the literal count is the reviewer's brace-depth script — the raw grep (49 non-definition hits) cannot separate literals from struct-update sites | **TRUE** (26 re-run; 15 accepted) |
| NEW-2 | T1's kill list lacks the third slice direction | the T1 text (mine) | **TRUE** |
| NEW-3 | T6 lacks the advisory clause | the T6 text (mine) | **TRUE** |
| gate | I-new-1's quotes byte-exact; build may proceed | the reviewer's byte checks against the CFR XML and the instructions | **accepted** |

**Verdict: 0C/0I — the spec is GREEN at r6. Minors and Nits fixed inline; no re-dispatch (Minor/Nit
do not hold the gate, and the r5 result was clean).**
