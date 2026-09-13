# BRIEF — Schedule A for TY2026: scope the 17→18 move, the §68 gate, and two new worksheets

**Tier:** opus. **READ-ONLY** — change no source file. Own worktree. No subagents.
**Why now:** the owner stated 2026-09-13 that **their own return itemizes**, so Schedule A is on the
critical path for the first filed return — and the port report already flags this area **DORMANT /
IMPORTANT** with the gate and both worksheets **unmodelled**.

## 0. The one question

> What exactly must be transcribed for TY2026 Schedule A, and which existing code names a line number that
> has moved underneath it?

## 1. The facts to start from — verify each, do not assume

From `design/TY2026_PORT_REPORT.md` (rows 20 and the "new" row, `:135`, `:454`, `:623`):
- the itemized **total moves from line 17 to line 18**, behind a new §68-style gate:
  *"Is 1040 line 11b minus 13a and 13b more than $384,350?"*;
- the former free-text **line 16 is enumerated 17a–17k/17z**;
- **charitable moves to a new Charitable Contribution Limitation Worksheet at line 13**;
- an **Itemized Deductions Worksheet** appears;
- `printed.rs::ScheduleALines.line17` and `AbsoluteReturn.itemized_deduction` (`return_1040.rs:1875`) both
  still name **line 17**;
- ★ `:623` records a **draft-internal discrepancy**: one draft says *"line 13a"* while the TY2026 Schedule A
  draft line 18 subtracts *"lines 13a and 13b"*. **Adjudicate this against the extracted text and say which
  the document actually prints.**

**Transcribe from the TEXT LAYER** (`design/forms/extract/*.txt`), never a rendered page. That rule exists
because a rendered `12` and `22` differ by a few pixels, and reading Form 6251 line 33 off the image once
produced *"subtract line 32 from line 12"* where the form says **line 22** — a $200,000 error on one vector.

## 2. What to produce

1. **The TY2026 Schedule A line set, from the extract**, every line with its printed caption verbatim.
2. **A move table**: for each TY2025 line, where it went in TY2026 — same / moved / retired / ★ **reused for a
   different quantity**. The last is the dangerous class: the Schedule 1-A 37→43 collision refilled line 37
   with MAGI, so reusing the old cross-reference substituted a six-figure income for a ≤$6,000 deduction and
   overstated the AMT base by ≈MAGI, taxpayer-adverse. **Look for that shape here and say plainly whether it
   occurs.**
3. **Every site in `crates/` that names a Schedule A line number**, and whether that number still means the
   same thing in TY2026. Derive this with grep; do not hand-list from memory.
4. **The two worksheets**: are they separate documents to archive, or printed on the schedule itself? What
   inputs do they need that btctax's input surface **cannot currently answer**? ★ `CLAUDE.md`: *if the form
   asks something our input surface cannot answer, collect it — that is following instructions, not scope
   creep.*
5. **The §68-style gate's threshold**, quoted, with the line it reads and whether `FullReturnParams` has a
   field for it.

## 3. Out of scope

Write no code and propose no implementation beyond naming what must be transcribed and collected. Do not
touch `crates/`. Do not re-derive `printed.rs`'s field count (measured 2026-09-13: **149** lineNN fields
across 10 structs — FR-159 records that the port report's 129 is stale). Nothing about TY2025 as a filed
year. No state returns (§7.2).

## 4. Stop-and-report

Six briefs in this arc were refuted by their implementer, three of them mine, all today. If the TY2026
Schedule A extract is absent, or a draft is not archived, or the port report's claims do not match the text —
**stop and report that**, because a scoping document built on a mis-transcribed line is worse than none.

## 5. Working rules

Own worktree. Run everything in the **FOREGROUND** (FR-175). Capture output once, grep it. **Do not commit,
do not push.**

## 6. Deliverable

Final action: Bash heredoc (**not** the `Write` tool) to:

    design/agent-reports/RECON-schedule-a-ty2026.md

Return a short summary plus that path.
