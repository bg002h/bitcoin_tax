# bitcoin_tax (TaxApp)

## Standard workflow — authoritative

All non-trivial work on this application follows our standard workflow, defined
in [`STANDARD_WORKFLOW.md`](./STANDARD_WORKFLOW.md). Read it before starting any
feature, fix, or design work. It is the contract, not a suggestion.

The spine, in one line: **every written design artifact — from the spec onward —
passes an independent review loop that runs until 0 Critical / 0 Important
findings remain, and no work proceeds past a gate while a blocking finding is
open.**

Operating reminders (full detail in `STANDARD_WORKFLOW.md`):

- **Phase order:** Brainstorm → Spec → Plan → Implement (phased, TDD) →
  whole-diff review → Ship. Each "→ green" is the §2 review loop.
- **Gates are hard.** "It's a small/mechanical change" is the rationalization the
  rule exists to override. Ceremony scales *down* for small work; it is never
  removed (§8).
- **Independent review.** Author ≠ reviewer on the same artifact at the same time.
  Persist every reviewer's output verbatim **before** folding it. Re-review after
  every fold — including the last.
- **Artifacts:** `BRAINSTORM_*`, `SPEC_*`, `IMPLEMENTATION_PLAN_*`, a `reviews/`
  directory, and `FOLLOWUPS.md`. Verify citations against current source at write
  time.
- **Green** = the full validation suite passes **and** 0 Critical / 0 Important.
