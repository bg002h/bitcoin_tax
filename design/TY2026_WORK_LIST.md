# TY2026 port work list — computed, not estimated

Generated 2026-09-05 by `xtask form-delta <2025-final> <2026-draft>` over every form with both
revisions archived. **Regenerate when a final lands** — that is the whole point of the tool: the
handoff becomes a diff, not a rebuild from memory.

★ Drafts are EVIDENCE ONLY (`design/ty2025/SPEC.md`). Nothing here is transcribed; these are
counts of what CHANGED, which is exactly what a port must look at and is not a tax figure.

## The headline

**Two of the three most important forms rename NOTHING and move dozens of lines.** `f1040`
keeps all 199 field names and moves **31** printed line bindings; `f6251` keeps all 62 and moves
**28**. A field-name existence check passes cleanly on both. That is the same trap TY2025's Form
6251 sprang — one added field walks everything below it down — and it is why `form-delta`
reports the label axis separately.

**Schedule 1-A was REBUILT, not revised.** 10 fields survive out of 219. That corroborates the
TY2026 port report's Critical R2 from the other side: the draft moves Part V from lines 37-43,
so `Form6251Line1Rule::Y2025 { schedule_1a_l37 }` cannot be reused for 2026.

**Four forms are byte-identical in both axes** — `f1040sb`, `f8949`, `f8959`, `f8960`. Those
ports are mechanical.

| form | common | added | removed | lines that moved | shape |
|---|---|---|---|---|---|
| `f1040` | 199 | 0 | 0 | 31 | port |
| `f1040s1a` | 10 | 175 | 44 | 1 | **REBUILT** |
| `f1040s2` | 44 | 24 | 19 | 12 | port |
| `f1040s3` | 35 | 3 | 2 | 0 | port |
| `f1040sa` | 14 | 33 | 19 | 0 | port |
| `f1040sb` | 72 | 0 | 0 | 0 | unchanged |
| `f1040sc` | 59 | 50 | 46 | 16 | port |
| `f1040sd` | 55 | 0 | 0 | 8 | port |
| `f1040sse` | 25 | 2 | 2 | 1 | port |
| `f6251` | 62 | 0 | 0 | 28 | port |
| `f8949` | 202 | 0 | 0 | 0 | unchanged |
| `f8959` | 26 | 0 | 0 | 0 | unchanged |
| `f8960` | 38 | 0 | 0 | 0 | unchanged |
| `f8995` | 22 | 14 | 11 | 0 | port |

## Not listed

`f8275` and `f8283` have **no TY2026 draft**: the IRS draft URL served a 2024 and a 2025
document respectively, and `scripts/archive_drafts.py` REFUSED both rather than archive a
wrong-year form as TY2026 evidence. Re-run the archiver when they are posted.
