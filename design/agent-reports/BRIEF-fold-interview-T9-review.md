# Brief — fold the T9 seam review (1C / 1I / 2M / 1N)

You are folding an independent seam review into the **main working tree** of `/scratch/code/bitcoin_tax`,
at `HEAD` on branch `main`. You are the only agent editing this tree. **Commit nothing** — the controller
commits through the pre-commit gate when you return. Do not push, do not `git stash`, do not spawn
subagents, and never `git checkout -- <file>` over your own uncommitted work (keep a `cp` backup and
restore from it).

Read first, in this order:
1. `design/agent-reports/2026-09-07-build-interview-T9-review.md` — the review, verbatim.
2. `design/agent-reports/2026-09-07-build-interview-T9-review-VERIFICATION.md` — the controller's ledger.
   **Every finding is already machine-confirmed.** Do not re-litigate whether one is real; the ledger
   records the observed output. Your job is the fix and its kill.
3. `design/agent-reports/2026-09-07-build-interview-T9-implementation.md` — the build you are correcting.
4. `CLAUDE.md` at the repo root.

**A fold is authorship and re-earns the build gate.** Machine-check everything machine-checkable before
you return.

## Validation

Main tree ⇒ leave `CARGO_TARGET_DIR` unset (default `target/`); use `CARGO_TARGET_DIR=target-clippy` for
clippy. Scoped runs while you work; `make check` once at the end (~19 s, 3470 tests at HEAD). Run all
five instruments before returning — at HEAD they are `line-coverage` 375 money lines / 18 forms / 31
exceptions (ratchet 31) / 0 unverifiable / 17 not line-bound; `census-join` 283 across 13 maps;
`stop-list` 8 + 4 sources, 91 prompts; `prompt-check` OK — 88 assertions; `box-census` 268 boxes / 19
editions / 9 returns. Any movement is yours and must be explained.

## C-1 (fold first, Critical) — a standard-deduction filer is asked and refused over a mortgage they do not deduct

Confirmed by the controller on the spec's own J-24 filer (`schedule_a: None`, one Form 1098):

```
LEDGER live(ClaimingMortgageInterestCredit) = true
LEDGER reason(blank shared-interest) = Some("SharedMortgageInterestUnanswered")
LEDGER reason(shared-interest = Some(true)) = Some("SharedMortgageInterest")
```

`design/SPEC_interview.md:1164` (J-24) guarantees the opposite: *"the census row and the three
declarations are not live; nothing is transcribed and **nothing refuses**"*.

The doc comment at `questions.rs:2509-2511` states the premise that made this look safe — *"the 1098
SECTION is already gated on the itemize election"* — and it is **false**: `section_is_live`
(`btctax-tui-edit/src/edit/form.rs:796-809`) has no `Form1098s` arm and falls through `_ => true`.
Reachable with no filer action: `open_next_year` seeds a prior lender onto a year carrying no
`schedule_a` (`grep -c schedule_a crates/btctax-cli/src/open_next_year.rs` → **0**).

**Fix** (the reviewer verified nothing in `btctax-core` depends on the un-gated behaviour):
1. `mortgage_interest_credit_question_live` →
   `ri.schedule_a.as_ref().is_some_and(|a| !ri.form_1098.is_empty() || !a.mortgage_interest_not_on_1098.is_empty())`,
   and **delete the false premise from its doc comment** — replace it with what is actually true.
2. In `screen_param_free`'s Form 1098 loop, skip the shared-interest arms when `ri.schedule_a.is_none()`.
3. **Leave the box-4 refusal ungated.** That refund is income on Schedule 1 line 8z, owed whether or not
   the filer itemizes. Say so in a comment so a later reader does not "fix" it.

**Kill.** The shipped `a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing`
is a shadow — it enumerates only the three OLDER declarations and its fixture pre-answers both new gates
(`other_borrower_paid_interest: Some(false)`, and `ri()` sets `claiming_mortgage_interest_credit =
Some(false)`), so no mutation of either new rule can red it. Make it the kill it claims to be: assert
`!question_is_live(ClaimingMortgageInterestCredit, …)`, and assert `reason(&standard) == None` with the
shared-interest gate at `None` **and** at `Some(true)`, and `claiming_mortgage_interest_credit` at `None`
**and** at `Some(true)`. Then plant each rule's gate back and paste both reds.

Add a second kill for the reachable path: the `open_next_year` seed on a standard-deduction year must
refuse nothing.

★ This is the same defect shape as T8's I-1 one task ago — a rule typed beside the thing that already
defines liveness, with a doc comment asserting the conjunct is redundant. When you write the fix, prefer
a shape where the conjunct cannot be forgotten by the next rule rather than three sites that each
remember it.

**Settle the spec in the same pass.** R8 currently says *both* that the document is top-level (*"it
arrives whether or not the filer itemizes"*) and that *"the `form_1098` census row and the 1098 section
are live iff `schedule_a.is_some()`"*, and the build followed the first. Gating the two rules satisfies
R8's rationale and J-24 while keeping the document top-level. Pick that reading, correct R8 so it says
one thing, and note the change.

## I-1 (Important) — line 8b prints *"See attached"* for a statement nothing produces, asks for, or names

Confirmed: `grep -c 'See attached\|SEE_ATTACHED'` → `schedule_a.rs` **4**, `cmd/admin.rs` **0**. And
TY2025's `line8b_payee` has exactly **one** cell (`forms/2025/f1040sa.map.toml:134`), so **two**
recipients overflow.

The instruction's sentence is *"identify the person by **attaching a statement to your paper return** and
printing 'See attached'"* (`i1040sca--2025.txt:1126-1131`). btctax prints the second half and does
nothing about the first, so the filer signs under §6065 a Schedule A asserting an attachment they were
never told to write.

**Fix.** Add a `hand_marks` entry conditioned on the overflow actually having occurred — the same
"conditioned on the mark being blank in THIS packet" rule that function's own doc comment states —
naming line 8b and listing the recipients whose identities did not fit. Decide and state whether an
advisory or transcription warning should fire too.

**Kill.** Drive `fill_schedule_a` with `len(payee) > len(map.line8b_payee)` on **both** years, asserting
the printed `"See attached"` and the manifest item. Plant: remove the `hand_marks` entry → red. The
branch has never been observed doing anything (B1), so this is its first sighting.

## M-1, M-2, N-1 — fold as cleanups

- **M-1** `transcription_warnings.rs:346` guards on `ri.form_1098.is_empty()`, not the election, so the
  ceiling warning fires for a standard-deduction filer and points at `MortgageWithinDebtLimit` — a
  question they are never asked. Add `|| ri.schedule_a.is_none()`, and give the three
  `the_acquisition_debt_warning_*` fixtures a `schedule_a: Some(ScheduleAInputs::default())` (the
  reviewer confirmed those three are the only reds).
- **M-2** the home-sale table crosses 10 of 24 combinations while the report and brief describe a full
  cross. Either lift the 1099-S state into the loop (24 rows, still instant) or correct the sentence in
  the test's doc comment **and** the build report. State which.
- **N-1** `transcription_warnings.rs:364` interpolates `${total}` / `${limit}` raw, so the filer reads
  `$900000`. Use the module's `money()` (`:312`); thousands separators if `money()` provides them.

## Standing lessons that bind this fold

- **A rule typed beside the thing that defines liveness will drift** — that is C-1, and it was T8's I-1.
- **A kill CALLS the instrument, and a fixture that pre-answers the gate is a shadow.**
- **An entry is testimony** — a printed mark the filer never made is the defect class, not a cosmetic one.
- **Blank is the normal case**; assert provenance, never population.
- **Never hand-count what a tool can count.**

## Report — your FINAL action

Write `design/agent-reports/2026-09-07-build-interview-T9-fold.md`: Commands with real output; one
section per finding (What changed / Where / The kill and its observed red / Anything decided differently
and why); the spec change you made for C-1; a **Deviations** section; the five instrument outputs; the
closing `make check` summary line. Then return ONLY a 3-line summary plus the path.
