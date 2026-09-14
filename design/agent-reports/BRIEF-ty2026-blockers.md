# BRIEF — make the TY2026 blocker list DERIVABLE, then drive a throwaway TY2026 return

**Owner, 2026-09-13:** *"make a list of all the things that prevent us from completing 2026 return and then
do a throw-away test run where we make up what is knowable yet and see what problems arise."*

**Two stages. This brief is STAGE 1 only.** Stage 2's design depends on what stage 1 finds.

## 0. Why a derived list rather than a written one

A hand-written blocker list is the thing that rots. Today alone, three stale prose claims were corrected —
and one of them (*"only two text cells differ"* on Form 6251) had **mispriced an owner decision**. So the
deliverable is **an instrument, not a document**: something that answers *"what blocks TY2026?"* by reading
the tree, and that is therefore still true next month.

★★ And it sharpens stage 2: the throwaway run's real product becomes **the gap between what this command
predicts and what actually breaks.** Everything in that gap is an unknown unknown — which is what FR-102
(an HSA filer who could not print a page), FR-218 (a duplicate blank page) and this week's five walls all
were. A run that only confirms known blockers has taught nothing.

## 1. What already exists — extend it, do not replace it

`YearReadiness` (`crates/btctax-cli/src`) already carries `declared`, `table`, `params`, `forms_bundled`,
`prices_max_date`. That is the **year-level** half. Find it, read it, and extend from there.

## 2. What the list must additionally derive — every item from the tree, none typed

1. **Form-level state**, from the work list's own generated columns (landed today): which of the 15 stems
   report `the FORM = CHANGED` vs `unchanged` vs `UNWITNESSED`, and the `respelled` / `introduced` /
   `retired` / `meaning-changed` / `UNREAD` counts. ★ Do **not** re-derive these; read what the generator
   prints, and cite the generator.
2. **Archive gaps.** FR-181: neither `f1040--2026` nor `i1040gi--2026` is archived, and the 1040 is the form
   the packet is built around. Derive the set of forms a TY2026 return needs against what
   `design/forms/extract/` actually holds.
3. **The refusals that fire on a TY2026 return.** There are ~128 `RefuseReason` variants; which are
   reachable *because the year is 2026*? ★ This is the one that needs care: a refusal that fires for a
   TY2026 filer is a blocker; one that fires for any year is not year-specific and belongs in a different
   bucket. **Separate them.**
4. **Worksheets with no revision archived** — the new `state_local_refund` module already refuses TY2026
   until the January `i1040gi` lands, with a test that reds the day it does. Find every surface with that
   shape; they are the blockers that will clear themselves.
5. **Owner-owned items**, listed but marked as not-ours: the Form 8283 Section B **qualified appraisal** for
   a Bitcoin gift over $5,000 (an external deadline on the owner), and S1.

## 3. Shape of the deliverable

A subcommand — `xtask ty2026-blockers` or an extension of the readiness surface, your call, argued in the
report — that prints the blockers grouped by **who clears them and when**:

| bucket | meaning |
|---|---|
| **clears itself in January** | waiting only on an IRS document (finals, `i1040gi--2026`) |
| **we must build** | code or transcription, with the form that demands it |
| **we must decide** | an owner ruling or an adjudication |
| **not ours** | the appraisal; anything outside `LONG_RANGE_PLAN §7` |

★ Each row must name its **evidence** — the file, the generated column, or the refusal variant — so a reader
can check it without trusting the list. ★★ And it must distinguish *"blocked"* from *"unmeasured"*: a form
whose axis reads `UNWITNESSED` is **not known to be fine**, and saying so is the whole point (that
distinction was added to the work-list table hours ago; reuse it rather than inventing a second vocabulary).

**B1:** the command must be observed RED on a planted blocker — remove a bundled param, or hide an archived
extract, and watch the list grow the right row. A list that cannot notice a new blocker is a list, not an
instrument.

## 4. Out of scope for stage 1

Do not bundle TY2026 params, do not invent any IRS figure, do not archive anything, do not touch
`crates/btctax-core/src/tax/state_local_refund.rs` or the 8949 emitters (both landed hours ago). Write no
markdown blocker list as the primary artifact — the command is the artifact; your report may summarise
its output.

## 5. Working rules

Own worktree, `CARGO_TARGET_DIR=<worktree>/target-blockers`, never `/tmp` (32 GB tmpfs). **FOREGROUND every
command** (FR-175: two agents stalled this way, the second with the prohibition in its brief).
★ Two distinct concurrency failures: `ld.lld: undefined hidden symbol` from a stale `.rlib` →
`cargo clean -p <crate>`; `signal: 9, SIGKILL` → memory, run nextest and clippy serially. `make check`
excludes `cargo fmt --all --check`; run both. **Do not commit, do not push.** No subagents.
★★ **Ten briefs in this arc were refuted by their implementer, seven of them mine — if you can disprove a
premise, STOP and report it.**

Deliverable: Bash heredoc (**not** `Write`) → `design/agent-reports/REPORT-ty2026-blockers.md`.
