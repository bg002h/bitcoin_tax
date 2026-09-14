# BRIEF — FR-200(a): btctax refuses an over-$750k mortgage because it never transcribed the worksheet

**The owner's instruction, verbatim: *"Fix the real problem which is the software bug itself."*** This
is not a policy question about partial printing, and it is not a hard tax question. It is a worksheet
nobody typed in. Work in your own worktree. Do **not** spawn subagents. Do **not** commit.

## The defect

An over-$750k mortgage makes btctax refuse **the entire packet** — including the Form 8949 and
Schedule D, which is the part this product exists to produce. Two refusals are involved
(`crates/btctax-core/src/tax/return_refuse.rs`):

- `MortgageDebtLimitUnanswered` (~`:1018`) — the §163(h)(3)(B) declaration is `None`.
- `MortgageOverDebtLimit` (~`:1028`) — the filer answered **adversely**: "one of the debt limits bites".

`MortgageOverDebtLimit`'s own doc admits the cause: *"i1040sca's Limits on home mortgage interest block
states four limits and btctax models only the mixed-use one."* So the product asks the filer whether a
limit bites and then refuses if they say yes, because it cannot do the arithmetic.

## Facts already SETTLED by the controller — do not re-derive

1. **`i1040sca` does NOT contain the arithmetic.** It states the four limits and says *"see Pub. 936 to
   figure your deduction"* **four times** (`design/forms/extract/i1040sca--2024.txt:877` onward).
2. **Pub 936 is now ARCHIVED, by me, for this task** (commit `3bec18e52`):
   - `legal/text/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.txt` — the TEXT LAYER, and
     **transcribe from this, never from a rendered page** (`CLAUDE.md`).
   - **Table 1** — *"Worksheet To Figure Your Qualified Loan Limit and Deductible Home Mortgage
     Interest for the Current Year"* — begins at **line 899**. Part I is lines 1–11, Part II is line 12
     onward. There is a **"Table 1 Instructions"** section later in the document; it carries the
     official per-line text you need for the doc comments.
   - sha256 `a4ef802b92ddd40ce2d00331b5012834dd274fd6ce74624586a1ea7dc31c32de`, revision *"For use in
     preparing 2025 Returns"*.
3. **The four limits** are: grandfathered debt; home acquisition debt incurred after 12 Oct 1987 and
   before 16 Dec 2017 capped at **$1,000,000 ($500,000 MFS)**; debt after 15 Dec 2017 capped at
   **$750,000 ($375,000 MFS)** and **reduced by** the pre-2017 qualifying debt; plus the fair-market-value
   limit. btctax models only the mixed-use one today.

## What to build

**Transcribe Table 1, do not derive it.** `CLAUDE.md`: one field per numbered line, named for the line,
in the form's own numbering, carrying the official instruction text verbatim as its doc comment. A
closed form is allowed **only** with a written equivalence proof naming the branch where it breaks plus
a KAT pinning that branch — and here you do not need one, because the worksheet is 15 lines of "enter
the smaller of" and "divide line 11 by line 12".

★ Transcribe the **branches** too, not only the lines. Part I has two "if … go to line 7 / go to Part II
line 12" jumps and Part II has a "stop here, all of your interest is deductible" exit. Those are lines
of the form, and a dropped branch is exactly the compression `CLAUDE.md` says the bugs live in.

**Collect what the worksheet asks for.** `CLAUDE.md`'s corollary: *"If the form asks something our input
surface cannot answer, collect it. That is following instructions, not scope creep."* The worksheet needs
average balances per origination bucket, interest paid on those loans, and the FMV limit's inputs. Add
them to the input surface properly — and per *"an entry is testimony"*, a missing figure is **not** zero:
if the filer has not supplied a balance, the return must still refuse rather than assume.

**Then retire the adverse refusal.** Once the limit computes, `MortgageOverDebtLimit` should no longer
fire for a filer who simply has a large mortgage — that is the bug. ★ But **do not widen anything into
silence**: keep a fail-closed refusal for the cases you genuinely do not model (the FMV limit, the
pre-Oct-1987 grandfathering exception, the Apr-2018 transition rule) and say in the refusal which limit
is unmodelled. *"Widening an exemption is never the safe edit."* State clearly in your report which of
the four limits now compute and which still refuse.

## ★★ VALIDATION — the oracles CANNOT check this, so read this section twice

Both oracles consume Schedule A **line 8a as an INPUT**. That is `FOLLOWUPS.md` §G-9's limit: *a value
the oracles take as input is never validated by their agreement.* So a green two-oracle sweep proves
**nothing** here, and you must not cite one as evidence.

The validation has to be **KATs from the publication's own worked examples**. Pub 936 contains many
(`grep -n "Example" ` the extract — there are examples at lines 241, 566, 603, 779, 792 and more, plus
the Table 1 Instructions). Find the ones that exercise Table 1 with figures, transcribe them as test
vectors with the line-by-line expected values, and cite the publication line number for each. ★ A vector
whose expectation you computed with your own implementation is worth nothing (FR-230) — the expected
numbers must come from the publication.

## Required — B1, seen-red-once
Every new checker lands **paired with a test that plants the exact defect and is observed RED**, then
green. ★ Per FR-235 do not plant in the checker's own vocabulary. ★ And per `CLAUDE.md`, mutate a line:
change one "smaller of" to "larger of", or drop a branch, and show which vector reds — that is the test
that the transcription is load-bearing rather than decorative.

Answer in your report, one sentence each: **"which test reds when this is reverted?"** with pasted red
output.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
★ **Never background a gate command and end your turn** (FR-175). ★ If you touch a clap doc comment,
`docs/man/**` is generated from it — regenerate or `gen_docs_is_deterministic` reds.

## Persisting your report
`Write` is REFUSED for report files. Use a Bash heredoc:

    cat > design/agent-reports/REPORT-fr200a-mortgage-limit.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and give the byte count in your summary. If refused, return it inline and say at the top
it is the only copy — never silently trim.

## Your tree, and what is NOT yours
Your worktree branches from an OLDER base than main (possibly ~20 commits behind). **Pub 936 may not be
in it** — if `legal/text/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.txt` is missing, read
it from `/scratch/code/bitcoin_tax/legal/text/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.txt`.
Do not rebase and do not try to catch up.

Do not touch: `crates/btctax-oracle-harness/**`, `crates/xtask/**`, `design/forms/extract/**` (archived
IRS text), `legal/**` (archived authority — read it, never edit it), any existing
`design/agent-reports/REPORT-*`, or `FOLLOWUPS.md` (put recommendations in your report).
★ **Non-crypto noncash charitable gifts / Form 8283 are a SEPARATE task — do not start on them.**
