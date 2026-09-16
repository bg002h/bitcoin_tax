# BRIEF — the ONE independent review round on `design/ty2025/SPEC_retirement_income.md`

Owner ruling S6: a prose artifact gets **one** independent review round, then the build. This is that
round. The spec has never had one — `design/ty2025/reviews/` holds thirteen artifacts, all for Schedule
1-A, Form 6251 and Form 8615. You are reviewing, not implementing: change nothing, write a report.

## The one question to answer

**Would executing this spec produce a correct TY2024 and TY2025 return for a retiree — and where would it
produce a wrong figure, a missing refusal, or a silent assumption?**

The feature is Form 1040 lines **4a/4b** (IRA distributions), **5a/5b** (pensions and annuities) and
**6a/6b** (social security benefits), plus the Social Security Benefits Worksheet and the Simplified
Method. None of it is built: `Form1040Lines` has no 4a–6b fields at all, so this is new construction.

## Already machine-verified — do NOT spend budget here

| | |
|---|---|
| every quotation verbatim in the archived extract | `xtask cite-check` OK, **51 quotations** |
| code citations | **15 of 19 had drifted** and were converted to file+symbol; the remaining 24 line numbers verified accurate at HEAD 2026-09-15 |
| the worksheet is byte-identical across TY2024/TY2025 | S-7, machine-checked |
| the coverage-checker prerequisite | **FR-184 closed** — a quote shared by N lines now needs N printed occurrences, so 4b/5b/6b cannot be graded interchangeably |

## The four owner rulings, all recorded in the spec at their own question

1. **OQ-1** — widen the scope-attestation prompt: ruled yes, **and it was already done** (`1548462af`,
   2026-09-04). Residual risk is *priming*, not silence.
2. **OQ-2** — **both TY2024 and TY2025**, because the owner *"will have at least one of those returns for
   evaluation."*
3. **OQ-3** — **refuse every QCD in v1**; the QCD split is the first widening after the base lands.
4. **OQ-4** — ★★★ *"Zero is the goal when nothing is wrong."* This **rejected** the spec's own
   recommendation to keep four advisories. §9 is rewritten with per-advisory verdicts.

## Where your budget should go — five things tools cannot reach

1. **★★★ THE VALIDATION PROBLEM, and it is the sharpest thing here.** `FOLLOWUPS.md` §G-9: *a value the
   oracles take as INPUT is never validated by their agreement.* Both engines consume Form 1040 line 6b as
   an input, so a green two-oracle sweep on the Social Security worksheet proves **nothing**. The owner's
   real return is therefore not a nice-to-have, it is the only oracle. **Does §10's test plan actually say
   that, and is it sufficient?** And FR-250: the 107-household golden corpus has no retirement income at
   all, so day-one coverage is zero.
2. **★★ DOES §9 NOW COMPLY WITH THE RULING?** The mechanism I asserted is that A-1…A-4 each fired on a
   *condition of the return* (`line 5b > 0`) rather than on whether the advice applies to the filer.
   Check that reasoning, the per-advisory verdicts (ask / ask / prose / owner-decides), and **A-4**, which
   I deliberately left for an owner decision because box 2a genuinely is the higher of two lawful figures.
   Is A-4's framing right, or did I misread it?
3. **★★ THE REFUSALS (§8) — completeness and direction.** Every gap must fail CLOSED. For each refusal:
   can a filer reach it, does it fire on the state it names, and is there an exit that works? Two defects
   of exactly that shape shipped this week (a refusal naming two cures that both dead-ended; a refusal
   printing a verb that never existed). ★ And the reverse: is anything refused that should compute, or
   computed that should refuse?
4. **★ S-6's ASYMMETRY.** The spec claims the pension side may take 1099-R box 2a while the IRA side may
   not, because the instructions differ. That is a domain claim with a funds consequence — **check it
   against the archived `i1040gi` text**, not against the spec's paraphrase.
5. **★ THE INPUT SURFACE (§7).** Four new mandatory questions. Does each one have to exist, and is each
   answerable from the paper the filer holds? A question nobody can answer from their documents is a wall.

## ★★ Review MY amendments hardest — they are the newest text and the least reviewed

I am an author of this spec now, which is why you are reviewing it. Written or rewritten by me on
2026-09-15 and seen by nobody:

- **S-5** — rewritten for FR-183: worksheet line 6 is `Sch 1 L13 + L15 + L18`, stated as a **block rule**
  rather than a list. Both of the original citations were stale. ★ Check the block membership against the
  worksheet's own sentence (*"Schedule 1, lines 11 through 20, and 23 and 25"*), and check I got the
  direction right: I claim omitting a member **overstates** the filer's tax.
- **§9** — rewritten for the OQ-4 ruling, including the claim that `CtcOdcOmitted { provably_zero: true }`
  is a live violation in shipped code.
- **M-6b** — a mutation row I added because M-6 only tested one direction.
- **§1's citation convention** and the four ruling blocks.

★ If any of that is wrong, say so plainly — a correction from you is worth more than agreement.

## Ground rules
- **Change nothing.** No edits to the spec, the code, or `FOLLOWUPS.md`.
- Severity per `STANDARD_WORKFLOW.md`: Critical and Important **block** the build; Minor and Nit are
  recorded. ★ Secret-handling defects are never Critical or Important (owner ruling 2026-08-27).
- **Read the primary sources.** `design/forms/extract/i1040gi--2025.txt` and `f1040--2024.txt` are
  archived; quote them, never the spec's paraphrase of them.
- Do NOT spawn subagents. Do NOT commit.

## Persisting your report
`Write` is REFUSED for report files. Use a Bash heredoc:

    cat > design/agent-reports/REVIEW-retirement-spec-r1.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and give the byte count. Structure: findings by severity, each with the spec section, what
is wrong, the primary source that settles it, and what you would change. A finding I can act on beats a
finding I have to re-derive.
