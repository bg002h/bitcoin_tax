# ADVICE — is "the PLANT is the other half of the kill" a new rule, an amendment to B1a, or already implied?

**Fable, 2026-09-12. Doctrine judgment on FR-154, for the owner. No file edited; nothing audited.**
Inputs: `design/HARNESS.md` (B1 `:279`, B1a `:298`, the two-class structure `:39`, the scope bound
`:58`), `design/agent-reports/REPORT-a5-fr136.md` §5, and `FOLLOWUPS.md` FR-136 / FR-154 / FR-155.

## The call: **2 — amend B1a.** Not a new B1b, and not "already implied".

**One disease, two organs.** A kill test hands the checker two things besides itself: a **fixture**, which
makes the checker's subject *present*, and a **plant**, which makes it *wrong*. B1a names the fixture. A5
found the identical defect in the plant: a value typed beside a set that grows, correct on the day, and
disarmed later by unrelated, correct work. The cure has the same shape in both places — derive it from
the measured set, or declare the premise with a named boundary — and A5's own escape hatch is, in its
words, "the B1a escape hatch". `HARNESS.md` classifies by the cure's shape, not by the symptom (`:41`,
"five symptoms, two mechanisms"). Same cure ⇒ same rule.

**But not option 3**, because a rule is what its question catches, and B1a's question does not reach
the plant. Ask *"what in this fixture makes the checker's subject present at all?"* of `year_record`'s
old kill and it answers cleanly: *the plant's push of `"f8995a"`*. The subject **was** present. The
borrow lives one step further back — in *why* `f8995a` is a defect rather than a fact — and B1a as
written never asks that. Applied faithfully on 2026-09-07 it would have passed the plant that broke on
2026-09-12. Doctrine that passes the case is not doctrine that implies it.

**Why not a sibling B1b.** Two reasons, both from the document's own text.

1. `:8` — the r1 draft "enumerated five mechanisms against five symptoms", and r2 calls that the
   excuse-list mistake. A rule per *site the disease was found in* is that mistake at the next level:
   B1a for the fixture, B1b for the plant, and — A5 already supplied the candidate — B1c for the
   **assertion** (kill 3 was "a mutation the old test passed, because it only checked a substring").
   The class is *every input the kill test constructs*; fixture and plant are its two observed instances.
2. The "expressible when the set is complete" criterion looks new, and it is not a new principle. It is
   the derived-versus-borrowed test made operational for a plant, exactly as "what makes the subject
   present?" is for a fixture. A borrowed plant cannot survive the borrow being repaid; a derived plant
   has nothing to repay. Naming it as a separate rule would hide that it is the same test.

The strongest case for a new rule — the symptom is *opposite* (B1a: silent green; A5: loud red about the
instrument, then inexpressibility) and the trigger is *repo progress* rather than the checker's own
mutation — is real, and it is why the amendment must carry its own paragraph, table and question rather
than a one-line "and the plant too". Different symptom, same class, same cure: amendment.

## Proposed wording — replaces B1a's heading and first paragraph, keeps its existing body, appends

> ##### B1a — the FIXTURE and the PLANT are the other half of the checker (amended 2026-09-07, owner-approved, FR-88; widened to the plant 2026-09-12, FR-154)
>
> **A kill test supplies the checker two things besides itself, and both must be constructed from the
> measured set — never borrowed from whatever the tree happens to hold today.** The **fixture** makes the
> checker's subject *present*; the **plant** makes it *wrong*. A hand-written value in either place,
> standing beside a derived walk, is the `1..=38` trap one level down: the checker is correct, and
> something the checker does not know silently decides what it may see.
>
> *[existing text from "Why this is an amendment and not a new rule" through "A stated boundary is
> reviewable; a silent one is the defect." stays verbatim — it is the fixture half.]*
>
> **The plant half — widened 2026-09-12 (FR-136, FR-154).** The fixture half has one symptom: silent
> green. The plant half has the opposite one, and it is worse. A plant that names a real artefact
> *because it happens to be absent* is correct on the day it is written and is un-planted by the first
> correct, unrelated commit that fills the absence — which, in a repo with a backlog, is the next thing
> anyone works on. It does not go green. It goes **red about the instrument**, at the moment a port is
> being shipped, and the cheapest discharge is to delete the test. And on a **complete** set there is
> no absence left to borrow, so the plant cannot be written at all.
>
> | plant | what it borrowed | what un-planted it |
> |---|---|---|
> | `year_record`, "expected but not bundled" | `f8995a`, absent from TY2025's bundle | supplying `f8995a/2025` — the port itself |
> | `year_record`, "bundled but not declared" | `f1040s1`, absent from TY2025's declaration | supplying `f1040s1/2025` — the same port |
> | `form_delta`, three work-list plants | `f8995a` with no 2017 side and no 2026 final | archiving one TY2026 final — what January does |
>
> ★ Every one was correct when written, and the `f8995a` plants had **already been moved once** (the
> committed comment records tag 2025 → 2017 on 2026-09-06, after `f8995a--2025` was archived).
> Relocating a borrowed plant to a fresh absence is the r1 excuse-list move: it survives exactly until
> the next gap is filled, and the gaps are the backlog.
>
> **Two questions, one per half.** The fixture's: *"what in this fixture makes the checker's subject
> present at all?"* The plant's: **"what makes this plant a defect rather than a fact — and would it
> still be one on the day the set is complete?"** A borrowed-absence plant answers the first cleanly and
> fails the second, which is why the first alone did not catch it.
>
> **The forms that hold, for the plant.** *Derive the victim*: remove a member of the list the build
> measured, so every member is planted and no member's absence is assumed — `year_record` now plants
> every stem in `present_for(year)` three ways and appends a synthetic COMPLETE year that no bundled
> year can supply. Or *declare the premise with synthetic material*: a synthetic stem against an
> injected oracle whose contents the test states in one line (`form_delta::check_work_list_against(..,
> archived)`). ★ A declared premise incurs one debt — the **real** oracle must then be shown to
> discriminate, by a guard that is itself derived (`form_delta`'s is true for every `(stem, year)` in
> `BUNDLED` and false for a stem no archive can hold, and it is the only test that reds when `pdf_for`
> stops resolving bundled templates).
>
> ★★ **The boundary, so this cannot be over-applied.** Inject the oracle the checker *consults*; never
> the computation the checker *is*. A plant that must borrow a real artefact's real content
> (`form_delta`'s `f1040` and `f1040s1` numeric rows, FR-155) keeps the borrow, asserts it with a message
> naming what will falsify it, and is **listed as a debt** — because freeing it by injecting `compute`
> would leave a kill that kills a mock. Where the fixture half's escape hatch is a discharge, the plant
> half's is only a deferral: it expires on the day the set completes, and the entry must say so.

Two notes on the text. First, the in-repo model line under B1 (`:285`) could gain a second model —
`year_record`'s removal walk — for the derived-plant form; optional. Second, FR-154 says the first rescue
was **nine** days earlier and A5 dates it 2026-09-06, which is **six** days before 09-12. The argument
does not turn on the number, so the wording above pins the date, not the interval; whichever is right,
do not let a from-memory count into the document that warns against them (`:157`).

## The cost — proportionate, and it can be stated exactly

**What it obliges.** One more sentence of thought per kill test: the second question. Mechanically, it
prefers *plant by removal from the measured set* over *plant by naming an absence* — and that form is
usually **cheaper**, not dearer: it is a loop over the set, and it plants N times instead of once
(`year_record` went from one plant to 177 for about the same code). The genuinely expensive form,
injecting the oracle, is needed only when the checker consults external state (a filesystem, an
archive), and it is a one-time refactor per such checker — A5 paid it once for `form_delta`.

**When it costs nothing.** A checker with no derived set has no set to complete and the rule does not
apply. It is process, not a hook, so there is nothing to mute (`:394` — "the class-β rules are process,
not code, and that is their weakness"; the same weakness B1 and B1a already carry).

**Earned under the scope bound (`:58`)?** Yes, on the document's own standard: grown from observed
failure, not anticipation. Five plants in two files, one shape; a same-shape rescue that did not hold;
and it bites on the port machine's single most frequent action, at the moment of shipping, with
deletion as the cheapest discharge. That last property is what makes it worth a rule rather than a
follow-up: a defect whose easiest fix is to remove the instrument is the class this harness exists for.

**The hidden cost, named so it can be watched.** Over-applied, the rule manufactures kill tests that
pass against injected fakes and never touch the real oracle — the FR-155 boundary crossed. The
"real oracle must discriminate" debt is the whole mitigation, and it is a debt someone can skip. That
is the cost to watch, not the extra sentence.

## The nine-day recurrence — evidence, but not for the reason FR-154 gives

FR-154's argument is "a shape that recurs after being fixed is doctrine, not a bug". Taken alone that
proves too much: any bug whose first fix was a patch recurs, and the 09-06 rescue *was* a patch — it
moved the literal to a different absence, the r1 excuse-list move. Recurrence after a patch is what
patches do.

What makes it evidence is **who broke it the second time: nobody.** A port rehearsal — correct,
unrelated work — un-planted it. That is the exact signature `CLAUDE.md`'s "derive the list" rule names
(*"not one of these was a mistake when written … the defect is introduced by a later, unrelated edit"*),
and class membership is what earns an amendment under `:23` ("built from observed failures"). And it is
not coincidence that both hits were `f8995a`: an absent thing in a repo with a port machine is, by
definition, the thing about to be filled. A plant keyed to an absence is keyed to the most volatile fact
in the repo. That is selection, not chance.

So: you are right that it is the strongest **empirical** argument. The strongest argument overall is the
completeness one — on a complete year the plant cannot be written, which proves no relocation can ever
fix it — and the recurrence is that proof observed in the wild. They are one argument from two sides.

## The crux — fixture vs plant: one disease

The distinction is real and it is exactly *where* in the kill the borrow sits: the fixture borrows a
state that makes the subject **absent** (T9's pre-answered gates, T10's unpopulated leaves), and the
plant borrows a state that makes a **non-defect look like one** (`f8995a` absent today). Mirror images:
one borrows a favourable accident and goes silently green; the other borrows an unfavourable accident
and goes loudly red later. But "borrowed from the tree's current state instead of constructed from the
measured set" is the single cause, "derive it, or declare it with a named boundary" is the single cure,
and the document's structural commitment (`:39`) is to name classes by cure. One disease. B1a absorbs
it, with its own question — because a class-level rule with only the fixture's question is a rule that
has already been shown to pass the plant.

## What would falsify this recommendation

- **"Amend, don't add" is wrong if** a borrowed-absence plant turns up in a kill for a checker that walks
  *no* derived set — a checker of a single fact. Then "typed beside a set that grows" is not the frame,
  and the plant needs its own name. I predict it cannot: an absence presupposes a set it is absent from.
- **"Amend, don't leave as-is" is wrong if** someone shows that B1a's existing question, asked honestly
  of `year_record`'s old kill, yields *"the absence of `f8995a`"* rather than *"the plant's push"*. Then
  B1a already reached it and only its application failed. I read the honest answer as the push; the
  borrow is in the plant's premise, one question further than B1a goes.
- **The cost judgment is wrong if**, within the port machine's first few ports, the amended rule
  produces a kill whose injected oracle passes while the real one is broken — the FR-155 boundary
  crossed and the discriminate-debt skipped. Then narrow the rule to plant-by-removal only and drop the
  injected-oracle form.
