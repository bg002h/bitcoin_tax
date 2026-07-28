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

## Transcribe IRS forms — never paraphrase them

**Tax forms are designed to be filled out by ordinary people following instructions. If implementing one
feels hard, you are doing it wrong.** A form never asks you to *derive* anything: it says "enter the
amount from", "enter the smaller of", "if X, skip to Y".

**The rule.** When implementing or reviewing an IRS form, schedule, **or worksheet**: one field per
numbered line, named for the line, in the form's own numbering, carrying the official instruction text
verbatim as its doc comment.

```rust
/// L20 — "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax
///        Worksheet ... (as figured for the regular tax)."
pub line20: Usd,
```

A **derived or closed form is allowed only** with (a) a written equivalence proof that names the branch
where it breaks, and (b) a KAT pinning that branch. Absent both, transcribe.

**Why this is a standing rule and not a style note.** Every defect in the 2026-07-27 AMT sequence — the
one shipped in v0.9.0–v0.13.0 and five more found in review — was a line that was never typed in, not a
hard tax question. The shipped bug reduced the AMT screening worksheet to `AGI − QBI` and conflated
Schedule A **line 7** (taxes) with **line 17** (itemized total). Later drafts dropped Form 6251 line 2b
and the MFS line-4 kicker, and spent three review rounds on a Part III question that line 20 answers in
one sentence. Compression always looks like good engineering; it is where the bugs live, because the
dropped term becomes invisible once the lines are gone. Two of these compressions carried confident
equivalence comments that were simply wrong.

**Corollaries.**
- **If the form asks something our input surface cannot answer, collect it.** That is following
  instructions, not scope creep.
- The review gate becomes mechanical: *is every line present, and does each doc comment match the
  official instruction text?*
- The PDF emitter becomes trivial — if the struct is the form, filling it is a field→AcroForm mapping
  with no logic.

An audit of the whole constellation against this rule is open: `FOLLOWUPS.md` §G-5.

## Tests for conformance, reviews for judgment

**Do not review a document to check whether it faithfully describes a form. Write the test.** The form
is a specification you can execute against — encode it, and let the compiler and the suite find the
gaps permanently, on every future change. Prose review finds a defect once; a test finds it forever.

- **Conformance ⇒ test.** "Is every form line present?" "Does each doc comment match the instruction
  text?" "Does line 10 subtract Schedule 3 line 1?" are assertions, not opinions.
- **Let the compiler review.** Adding a field to a struct with N literals `E0063`s all N; adding a
  `RefuseReason` variant reds an exhaustive cross-crate match. That blast radius is free and exact.
  Prefer designs in which an omission does not compile.
- **A guarantee without a test that reds when it is removed does not exist.** Mutation-verify.
- **Judgment ⇒ review, kept scarce.** Worth paying for: adjudicating an ambiguous instruction against
  the primary source; a design decision with an adverse branch that a *passing* test would hide; a
  domain fact that would make a green test meaningless. One round, sharp brief, primary sources, build.
- **Stop reviewing a document once findings become "section X disagrees with section Y."** That is the
  signal the artifact has stopped being where the risk lives. Go execute it.

This does **not** relax `STANDARD_WORKFLOW.md`'s gate — it says match the instrument to the question.
Evidence (the 2026-07-27/28 AMT plan): five review rounds went 5C/12I → 2C/8I → 0C/4I → 0C/4I → 0C/2I,
rounds 3–5 finding mostly "edited §X, forgot §Y" — while the single highest-severity defect, a deleted
Form 6251 line-10 definition, was found in seconds by opening the PDF that all five rounds discussed.
