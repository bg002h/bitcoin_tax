# BRIEF — FR-200(b): a $600 bag of clothes refuses the whole packet, because Form 8283 Section A was never built

**The owner's instruction, verbatim: *"Fix the real problem which is the software bug itself."*** And the
owner's standing ruling on scope: *"The correct question isn't whether it applies to me but whether we
want to support the tax scenarios or not. The answer is yes, support all."* Work in your own worktree.
Do **not** spawn subagents. Do **not** commit.

## The defect

A non-crypto **noncash** charitable gift — a thrift-store donation of used clothing — makes btctax
refuse **the entire packet**, including the Form 8949 and Schedule D that are the reason this product
exists. The refusal is `RefuseReason::NonCryptoNoncashGift`
(`crates/btctax-core/src/tax/return_refuse.rs`, ~`:984`), and its own doc states the cause:

> *"Those amounts reach Schedule A line 12, but btctax holds no property details for them (no
> description, no acquisition date, no appraiser), so it can produce no 8283 rows — the packet would
> attach a Form 8283 that UNDER-REPORTS its own property list."*

The reasoning is sound and the refusal was the right stopgap. It is not the right end state: an
incomplete attachment is a §170(f)(11) risk, so the fix is to **hold the property details**, not to
loosen the refusal.

## Facts already SETTLED by the controller — do not re-derive

1. **Only Section B exists today.** `crates/btctax-forms/src/form8283.rs` and
   `crates/btctax-core/src/donation.rs` implement `section_b` only — the >$5,000 appraised path that
   crypto gifts take. **`Section A` appears nowhere in the source.** So this is new construction on an
   existing emitter, not a repair.
2. **The form is archived** — `design/forms/extract/f8283--2024.txt` and `--2025.txt`, instructions
   `i8283--2024.txt`/`--2025.txt`. **Transcribe from the text layer, never a rendered page.**
3. **Section A's structure, read off `f8283--2024.txt:26` onward:** *"Donated Property of $5,000 or Less
   and Publicly Traded Securities"*, four lettered rows **A–D** with *"If you need more space, attach a
   statement"*, and nine columns:
   `(a)` donee name and address · `(b)` vehicle box + VIN · `(c)` description **and condition** ·
   `(d)` date of the contribution · `(e)` date acquired by donor (mo., yr.) · `(f)` how acquired by donor ·
   `(g)` donor's cost or adjusted basis · `(h)` fair market value · `(i)` method used to determine the FMV.
4. **★★ The form prints a per-item carve-out, and it is the thing most likely to be got wrong:**
   *"Note: If the amount you claimed as a deduction for an item is $500 or less, you do not have to
   complete columns (e), (f), and (g)."* That is **per item**, while the $500 that triggers filing Form
   8283 at all is the **total** of noncash contributions. Two different $500s. A $600 bag is over the
   per-item line and needs (e), (f) and (g); four $200 bags totalling $800 must file the form but need
   not complete those columns. Model both thresholds distinctly, and say in your report how each is
   decided.
5. **Pub 561, *Determining the Value of Donated Property*, is already archived** —
   `legal/text/irs-publications/Pub561_Value_of_Donated_Property.txt`. That is the authority for column
   `(i)`. Pub 526 (*Charitable Contributions*) is archived too.

## What to build

**Transcribe Section A**, one field per column in the form's own lettering, official column text as the
doc comment, per `CLAUDE.md`. **Collect the property details** the columns require — *"If the form asks
something our input surface cannot answer, collect it. That is following instructions, not scope
creep."*

★ **"Optional" is not "blank", and this is the trap.** When the per-item carve-out applies, columns
(e)(f)(g) need not be *completed* — but per `CLAUDE.md`'s *"blank is the normal case"*, every line still
needs a determinate **provenance**: "not required, item ≤ $500" is a recorded decision; "nobody ever
collected it" is the defect. They are identical on the printed page and must not be identical in the
type system.

★ **The overflow.** Four rows print; more require an attached statement. There is in-tree precedent for
this shape — Form 8949's 14-row overflow — so follow it rather than inventing one, and **never silently
drop row five**. A gift that does not fit must either continue onto another copy/statement or refuse; it
must not vanish.

**Then retire the refusal for the case that now works**, and keep it fail-closed for what does not.
★ Do not widen it into silence: a gift whose details are missing, a vehicle (Form 1098-C territory),
intellectual property, inventory, or anything over $5,000 that is not publicly traded securities must
still refuse **and name which condition it is**. State in your report exactly which cases now file and
which still refuse.

## ★★ VALIDATION — the oracles cannot check this either

Both oracles consume Schedule A **line 12 as an INPUT** (`FOLLOWUPS.md` §G-9: *a value the oracles take
as input is never validated by their agreement*). So a green two-oracle sweep proves nothing about
Section A, and you must not cite one as evidence. Validate against **i8283 and Pub 561**, with expected
values quoted from those documents and their line numbers cited. ★ A vector whose expectation you
computed with your own implementation is worth nothing (FR-230).

## Required — B1, seen-red-once
Every new checker lands **paired with a test that plants the exact defect and is observed RED**, then
green. ★ Per FR-235 do not plant in the checker's own vocabulary. Mutate the transcription — swap a
column, drop the per-item note's threshold, delete a row from the overflow — and show which vector reds.
★ And the one I most want to see: **a fifth gift must not disappear.**

Answer in your report, one sentence each: **"which test reds when this is reverted?"** with pasted red
output.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
★ **Never background a gate command and end your turn** (FR-175). ★ If you touch a clap doc comment,
`docs/man/**` is generated from it — regenerate or `gen_docs_is_deterministic` reds. ★ Adding a
`RefuseReason` variant reds `xtask blockers`'s pinned census count; that is the instrument working —
report it, and leave `crates/xtask/**` to the controller.

## Persisting your report
`Write` is REFUSED for report files. Use a Bash heredoc:

    cat > design/agent-reports/REPORT-fr200b-form8283-section-a.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and give the byte count in your summary. If refused, return it inline and say at the top
it is the only copy — never silently trim.

## Your tree, and what is NOT yours
Your worktree branches from an OLDER base than main (~25 commits behind). Recent work may be absent —
including `crates/btctax-core/src/tax/pub936_table1.rs` (FR-200a, the mortgage half, folded already).
Do not rebase and do not try to catch up. If a file named in this brief is missing, read it from
`/scratch/code/bitcoin_tax/<path>`.

Do not touch: `crates/btctax-oracle-harness/**`, `crates/xtask/**`, `design/forms/extract/**` and
`legal/**` (archived authority — read, never edit), any existing `design/agent-reports/REPORT-*`, or
`FOLLOWUPS.md` (put recommendations in your report).
