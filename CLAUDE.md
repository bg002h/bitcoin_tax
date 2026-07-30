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

**Transcribe from the TEXT LAYER (`pdftotext -layout`), never from the rendered page.** A rendered `12`
and `22` differ by a few pixels. Transcribing Form 6251 line 33 from the image produced "Subtract line 32
from line 12" where the form says **line 22** — which taxed the ordinary slice twice and inflated the
tentative minimum tax by $200,000 on one vector. No review would have caught it; running the
transcription against seven independently-verified vectors caught it in seconds (1/7 → 7/7 after the
one-character fix). **Then re-verify every cross-reference against the extracted text.**

```rust
/// L20 — "Enter the amount from line 5 of the Qualified Dividends and Capital Gain Tax
///        Worksheet ... (as figured for the regular tax)."
pub line20: Usd,
```

A **derived or closed form is allowed only** with (a) a written equivalence proof that names the branch
where it breaks, and (b) a KAT pinning that branch. Absent both, transcribe.

**Why this is a standing rule and not a style note.** Every defect in the 2026-07-27 AMT sequence — the
one shipped in v0.6.0–v0.13.0 and five more found in review — was a line that was never typed in, not a
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
- **★★ But "present" is not "populated" — most fields on a tax return are blank, intentionally.** A
  filer with no tips, no overtime, no car loan and no senior leaves most of Schedule 1-A empty and that
  is the *correct* return. So the gate is never "does every line carry a value"; a conformance test that
  demanded population would push toward inventing entries, which is worse than the gap it closes.

  The real invariant is that **every line has a determinate PROVENANCE** — collected from the filer,
  computed from named lines, a constant the form prints, or a refusal — because two blanks look
  identical on the printed page and are not the same thing:

  | blank because | verdict |
  |---|---|
  | the inputs say so (no tips ⇒ line 4a empty) | **correct**, and the common case |
  | nothing ever populated it — never collected, never asked, never modelled | **the defect** |

  Only the second is a bug, and it is invisible in the emitted PDF, invisible to both oracles, and
  invisible to any test that checks the *value*. It is the [answered-ness invariant](#) again: a
  hardcoded zero and a computed zero are indistinguishable on the page but not in the type system, which
  is why answered-ness must be **structural**. The standing example is an **ISO exercise printed as $0**
  on Form 6251 line 2i — the dominant real AMT trigger post-TCJA, laundered as a blank.

  So a conformance KAT must (a) enumerate the expected line set **from the form's extracted text**, never
  from a range or a hand-written list, and (b) require every line to be *accounted for* — mapped to a
  field or decision, or explicitly recorded as carrying none **with a reason**. A checker that cannot
  distinguish *"this line encodes no decision"* from *"we forgot this line"* is not a conformance check.
  ★ Both halves have been got wrong here in one sitting: a `BTreeSet` built from `1..=38` (the label set
  is 48), and a direction check keyed to a hand-list of three parts that reds on nothing when a part is
  dropped.
- The PDF emitter becomes trivial — if the struct is the form, filling it is a field→AcroForm mapping
  with no logic.

An audit of the whole constellation against this rule is open: `FOLLOWUPS.md` §G-5.

## Two oracles, and the `.venv`

Tax figures are validated against **two independent engines**, never one: **OpenTaxSolver** (oracle 1,
`scripts/oracle/ots_direct.py`, GPL-2.0 and observe-only, needs `OTS_DIR`) and **Tax-Calculator**
(oracle 2, `scripts/oracle/gen_goldens.py`). The sweep asserts every admitted household reconciles on
every compared line against **both**. A single oracle disagreeing is ambiguous; two splitting is
diagnostic. **Never file an upstream defect, or call a figure validated, on one oracle.**

**The Python stack lives in the repo's `.venv`** — `.venv/bin/python` has taxcalc and pandas; bare
`python3` has neither and always will not. Try `.venv/bin/python` before concluding a Python tool is
unavailable.

**A disagreement is adjudicated against the FORM, never encoded.** The IRS PDF is the authority; an
oracle is a witness. Precedent: tenforty issue #278 / PR #279 — OTS was never wrong, the wrapper was.

**Both oracles ARE asked for AMT (since 2026-07-29).** `scripts/oracle/verify_f6251.py` diffs every
Form 6251 line both engines print, over 30 vectors spanning all four filing statuses. See
`FOLLOWUPS.md` §G-6 — still OPEN as the Tier-2 blocker (E4/E5/E6 remain), and
`design/amt-form6251/CONTINUITY_E4.md` is the resume point.

**★ An excuse list keyed by VECTOR NAME is a liability in that harness.** Both of the ones it had went
stale on the first batch of new vectors: OTS's `{V8, V2b}` missed three MFS vectors it is equally
blind to, and the Tax-Table methodology list `{("V10","line10")}` missed six lines. Both are now
COMPUTED from the defect's own mechanism, and taxcalc's names each omission's exact **size** — so a
divergence of the wrong shape is unexpected even on a vector expected to diverge. Same rule as the
form: state the mechanism, let it decide, never enumerate the outcomes you happened to see.

**★ Two disqualified oracles can align.** For MFS, §55(d)(3) puts the zero-exemption threshold and
the Form 6251 line-4 kicker start at the identical $875,950 — and that is precisely where OTS carries
stale constants AND taxcalc omits the add-back. Three vectors owe AMT with *no* witness. Two oracle
sections each printing "OK" hid it; a **witness census** that counts independent witnesses per vector
now surfaces it, and fails the run if any filing status loses its last two-oracle AMT-owing vector.

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
