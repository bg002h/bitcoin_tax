# BRIEF — retirement income (1040 lines 4a–6b): what the spec covers, and what it costs

**Tier:** opus. **READ-ONLY** — change no source file. Own worktree. No subagents.
**Owner ask, 2026-09-13:** Phase 6, retirement first. **Stated honestly up front:** the owner's own return
has **no** retirement distributions, and `LONG_RANGE_PLAN_filing.md` §7.5 puts Phase 6 *"off the critical
path entirely"*. So this is **breadth for future filers**, not the path to the first filed return. The owner
chose it knowing that. Do not re-argue the priority; scope the work.

## 0. The one question

> `design/ty2025/SPEC_retirement_income.md` exists for lines 4a–6b. **Is it still right, what does it not
> cover, and what would building it cost?**

## 1. Measured before dispatch — do not re-derive

| probe | core | cli | forms |
|---|---|---|---|
| `1099-R` | 12 | 4 | 1 |
| `pension` | 8 | 0 | 0 |
| `annuity` | 7 | 0 | 0 |
| `social_security` | 23 | 0 | 0 |
| **`f8606`** | **0** | **0** | **0** |
| **`RMD`** | **0** | **0** | **0** |

`RefuseReason::IraDeductionClaimed` exists — an IRA **deduction** is a recorded boundary. Nothing names
1099-R **income**. ★ So resolve, with evidence: is retirement income **refused**, **partially modelled**, or
**silently zero**? A silently-zero distribution **understates tax**, which is the worse direction.

## 2. What to produce

1. **The spec's coverage, line by line**, against the TY2026 1040 extract: 4a/4b (IRA distributions),
   5a/5b (pensions and annuities), 6a/6b/6c (social security). Taxable vs gross is the whole subtlety —
   say which lines the spec actually computes.
2. **The forms it drags in**, and whether each is archived: Form 8606 (nondeductible basis / conversions),
   the Simplified Method Worksheet, the Social Security Benefits Worksheet, Form 5329 (the RMD penalty),
   Form 4972. For each: archived? mapped? refused? nothing?
3. ★ **The TY2026 transfer justification.** The spec lives in `design/ty2025/`, and the owner ruled
   2026-09-11 that **no TY2025 return will ever be filed with this software** — *"we only care about 2025 to
   the extent that it helps us with 2026 and beyond."* So state what transfers to TY2026 unchanged, what
   must be re-read from the TY2026 documents, and what is TY2025-only and should be dropped.
4. **The input-surface delta**: which questions the interview must newly ask. Be exact — a question that
   cannot be answered from documents the filer holds is a design problem, not a line item.
5. **A build estimate in tasks**, each independently gateable, with the one that must come first.

## 3. Out of scope

Write no code. Do not build. Do not touch `crates/`. Do not re-run the field provenance census. No EITC —
that is a separate decision pending the scenario census. Nothing about state returns (§7.2) or e-file (§7.1).

## 4. Stop-and-report

Six briefs in this arc were refuted by their implementer, three of them mine, all today. If
`SPEC_retirement_income.md` does not exist at that path, or does not cover 4a–6b, or the probe counts above
are wrong — **stop and report it** rather than scoping a document you had to guess at.

## 5. Working rules

Own worktree. **FOREGROUND** for every command (FR-175). Capture once, grep. **Do not commit, do not push.**

## 6. Deliverable

Final action: Bash heredoc (**not** the `Write` tool) to:

    design/agent-reports/RECON-retirement-income.md

Return a short summary plus that path.
