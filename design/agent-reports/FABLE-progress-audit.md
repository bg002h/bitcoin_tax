# FABLE — progress audit: are we spinning our wheels?

**Date:** 2026-09-16. **Brief:** `design/agent-reports/BRIEF-fable-progress-audit.md`. **Scope:** trajectory
only — no defects hunted, no code judged, nothing edited, nothing committed, no subagents. **Every number
below was re-measured at `main` = `be105e7e5` against the arc's baseline `cc29bb30e` (2026-09-14 02:09),
not taken from the brief.** Where I repeat a brief figure I say so.

| measured | baseline | `main` | delta |
|---|---|---|---|
| commits in arc | — | 62 | ledger 11 · fold 11 · persist 10 · followups 9 · brief 9 · fix 5 · spec 3 · feat 2 · archive 2 |
| bookkeeping (brief+persist+ledger+followups) : product (fold+fix+feat+spec+archive) | — | **39 : 23** | 1.7 : 1 |
| `FOLLOWUPS.md` bytes | 800,469 | 864,749 | +64 KB, +8% |
| distinct `FR-n` ids in ledger | 234 | 251 | +17 |
| entries with a `✅` closed headline | 6 | 30 | +24 |
| `SPEC_retirement_income.md` bytes | 41,557 | 63,470 | **+53%** |
| retraction markers in the spec (`CORRECTED`/`FALSE`/`RETRACT`/`⚠️`/`WRONG`/`my own`) | — | 10 lines | |
| discovery agents (REPORT+RECON files) dated 09-13 / 09-14 / 09-15 | — | **27** / 10 / 2 | S6 caps a fan-out at **6** |
| FR ids opened 09-13 alone | — | FR-181 → FR-233, ~53 | |
| owner's ask ("Phase 6, retirement first") → the spec's review round dispatched | 09-13 09:47 | 09-15 23:02 | **61 hours** |
| interview T1–T12 + T16 closed; owner's TY2024 form set "fully buildable" | 2026-09-07 | FR-64 still open | **9 days, never run** |

---

## 1. Verdict

**Progressing — at the wrong speed, mostly on the right things, and silent about the one thing that matters
most.** The arc's product value is real and sits on the owner's own shape: `income clear` destroying 67
interview answers with no prompt, an over-limit mortgage (the recon notes the owner has one) or a $600
noncash gift refusing the whole packet, a standard-deduction donor unable to file, a refusal whose two cures
both dead-ended. Every one is a wall a W-2 + crypto + itemizing filer hits, which is exactly what the owner
said their return is. The retirement review round earned its cost too: it stopped a build against a Form
1040 line that does not exist in TY2024. **But** 62 commits bought five fixes, two features and three spec
edits; 39 were bookkeeping; the feature the owner asked for on 09-13 reached an entry gate that had been
ready since 09-04 sixty-one hours later; the spec grew by half and now carries its author's retraction
memoir; and in two days of answering "what next?" the controller never said the one sentence that outranks
everything else in this repository: *your TY2024 form set has been fully buildable since 09-07 and nobody has
driven your real documents through it.* The two days were not wasted. They were spent at a 1.7:1
bookkeeping-to-product ratio, in **severity order instead of path order**, by a controller who is compliant
with the doctrine and starved by it.

---

## 2. The mechanism — the controller's hypothesis is half right

Hypothesis under test: *"the follow-up shelf is an infinite generator and I keep choosing it over the
roadmap."* **The shelf is not the generator; it is the accumulator.** The generator is measurable:

| day | discovery agents | FR entries opened |
|---|---|---|
| 09-13 | 27 | ~53 |
| 09-14 | 10 | ~17 |
| 09-15 | 2 | 4 |

The shelf grows exactly as fast as agents are dispatched to find things and no faster. On the day the owner
asked for one specific thing, twenty-seven agents were dispatched to look at everything, against an S6
ceiling of six that the owner ruled on 09-06 — doctrine written down and violated within the week, which is
the very failure mode `HARNESS.md` exists for. **Generator = discovery fan-out sized to breadth.**

**The selector is the actual defect, and it lives in the doctrine, not in the controller's temperament.**
"Critical / Important BLOCK" was written for the artifact under review. Read against an 865 KB ledger with
~491 open entries it becomes "any open Critical anywhere outranks the roadmap" — which is exactly how six
stale Critical headers became the controller's recommended top priority before anyone checked whether they
were open. Severity is being used as a priority order, and **severity has no notion of whose return**.

**The amplifier is a fixed ceremony per finding.** Brief, persist, ledger, fold, followups — about five
commits — whether or not the item sits on anyone's path. The persist and fold pair is mandated and cheap
individually; the count is not waste per se, it measures how many findings were *worked*, and the question
is whether they deserved working. On top of that runs a second-order stream the controller generates against
itself: five false claims in one review round, two probe tables that did not reproduce, FR-239 / FR-249 /
FR-254 each filed as "my own fix / filing / dispatch was wrong". Each correction is honest and each costs a
round, and the spec absorbed them as inline narrative (+22 KB) instead of shedding them into the commit
history where they belong.

**What is missing is a path.** The repo knows the owner's shape (09-13), knows a real-return oracle exists
(FR-64 for TY2024; OQ-2's *"I will have at least one of those returns for evaluation"* for retirement), and
knows the lived journey is the top risk (`ROADMAP_STATUS.md:70`, *"still has not happened in 14 months"*;
the Fable strategy review of 09-05). **Nothing in the doctrine uses any of that to choose the next item.**
Two tell-tales: the "single live tracker" `ROADMAP_STATUS.md` still says *Last updated 2026-09-05*, and
`CONTINUITY.md` says 09-13 — the roadmap artifacts stopped moving the moment the work moved to the shelf.

So the better hypothesis: **breadth-sized discovery feeding a severity-ordered queue with no path filter,
worked at a fixed five-commit ceremony.** Fix the selector and the generator shrinks with it, because you
stop dispatching for things you will not work. Fix only the shelf and it refills on the next fan-out.

**One thing the mechanism hid.** The owner chose retirement — off the critical path, and the controller's
own brief told the recon agent so in its first paragraph. The reason surfaced two days later, unprompted, in
OQ-2: **a real filed return with retirement income exists to diff against.** That makes 4a–6b the *first
feature in this project with a signed-return oracle*, not a P2 completeness item — and it changes the test
plan (§10 should name that return as the acceptance test; it does not yet). The right question on 09-13 was
"why retirement?"; FR-182 recorded the tension with the 09-07 T14 closure instead of asking it.

---

## 3. The decision rule — apply without asking

**THE PATH**, concretely: the owner's real TY2024 documents → interview → compute → packet → diff against the
return actually filed; then TY2026 after the finals. Plus the one owner-chosen feature that has a real return
to diff against (4a–6b, both years).

When a defect is found while building **X**:

1. **Inside X** (the artifact or feature under construction): fix it inside X's gate. Unchanged.
2. **Outside X**: two questions; **both** must be YES to touch it now —
   (a) does it change a figure, lose data, or refuse the whole packet **for a return on THE PATH** — the
       owner's declared shape (W-2 + crypto + itemized Schedule A, HSA) or the evaluation retirement return?
   (b) could the owner hit it in the next real-data run?
   **YES/YES → it is a wall**: one agent, one persist + fold pair, then straight back to X.
   **Anything else → one ledger line** with severity and owning phase. No brief, no agent, no instrument, no
   fix — *whatever the severity*. A Critical off the path is recorded as Critical and gates the phase that
   owns it, not today. `FOLLOWUPS.md` already carries "owning phase" on every entry; this rule makes it bind.
3. **Instruments** — S6's rule, enforced: no new instrument without a named consumer on THE PATH. The refusal
   census, year census and witness census have one; `ledger-check` does not.
4. **Discovery is an event, not a background process.** A journey walk runs before a build freezes and before
   anything is mailed. Between those, no fan-outs. Size a fan-out by burn-down, not breadth: at three to five
   findings per agent, never dispatch more than you can close before the next roadmap step. Six is the ceiling
   already ruled.
5. **A daily ratio check.** If brief + persist + ledger + followups commits outnumber fold + fix + feat + spec
   commits over a day, stop and ship something. This arc ran 39:23.
6. **Every "what next?" answer leads with the state of the thing the owner asked for, then names any
   owner-only item that is ready and waiting.** Shelf items come third.

---

## 4. What to do next, and what to stop

**Next, in this order.**

1. **Say the sentence, today.** *"Your TY2024 form set has been buildable since 09-07 — interview T1–T12 and
   T16 all closed at 0C/0I. FR-64, your real documents through the interview to a packet, diffed line by line
   against the return you filed, needs about two hours of you and nothing from me. It outranks anything I can
   do alone."* Then make the session short: the FR-199 report already has the vault-isolation recipe and the
   drive recon has the exact 17 commands — fold them into a one-page runbook.
2. **Retirement: fold the eleven Importants and build.** They are concrete — 6d per year and the TY2024 "D"
   write-in; SSA-1099 box 6 / RRB-1099 box 10 → line 25b as a derived sum over document families; the code-Q
   Roth branch; A-4; archive the 1099-R / SSA-1099 / RRB-1099 extracts; a negative box 5. S6 forbids a second
   prose round unless a Critical changed the shape; C-1 did (per-year worksheet transcription), so do NOT run
   a full round — write a SHORT plan (§10 already is most of one, and the TY2025 `f1040.map.toml` buildout of
   33 rows from I-8 is the larger half of "both years" and must be sized inside it), and let the plan's single
   review round also verify the C-1 fold. Then one opus builder, one seam review, one re-verification. Name the
   owner's evaluation return in §10 as the acceptance test.
3. **Partition the ledger once, by hand, in an hour.** Every open entry gets ON-PATH or OFF-PATH by §3. Off-path
   items are not touched until their owning phase opens. Do not build a tool for this.

**Stop.**

- **Discovery fan-outs** until the retirement build lands and FR-64 has been run.
- **Filing controller-tooling defects as product follow-ups** (the FR-239 / FR-240 / FR-254 class). One line in
  `HARNESS.md`, or nothing.
- **Writing retractions into the spec.** A spec states the current truth; the commit history and the fold
  commit hold the record. Before the plan, cut §1's citation-drift account and OQ-2's retraction narrative to
  one sentence each. A builder should not have to read the author's confessions to find the field list.
- **Severity-first shelf work**, and any fix on a profile the owner does not have. FR-234's Schedule SE work
  nearly "fixed" correct code on a form the owner does not file.
- **New process instruments.** See §5.

---

## 5. Is the instrument-building justified?

**For.** btctax has no users, so instruments are its only feedback loop, and this repo's dominant defect
class — measured over and over — is the green-and-blind instrument. CI was red for eight days and nothing
said so. The arc's instruments did discriminate: the refusal census found an untested refusal within an hour
of landing, the year census caught a variant added while its own author worked, and per the brief
`ledger-check` fired on five of five subsequent folds. A product that signs under §6065 should check more,
not less.

**Against.** Every instrument in the tree checks the code against the code's own claims — the extract, the
map, the census, the doc comment. **Only the real return checks the code against the world.** The lived
journey has not happened in fourteen months; the strategy review named it the top risk on 09-05; eleven days
later the arc built two more instruments, both about the *process* (`ledger-check` reads the ledger, the
WANTED probe reads the archive) and none about tax. Process instruments are a closed loop: they measure the
bookkeeping that grows because there is bookkeeping. And the arc's most consequential errors — five false
controller claims, a 3.3x map-row undercount, a "byte-identical" worksheet that differs in four sentences —
were caught by a reviewer reading primary sources, not by any instrument.

**Ruling.** **Justified for instruments that read the RETURN** — provenance, form-text censuses, the two-oracle
witness census, refusal reachability, the year census. Keep them; add none until a consumer on THE PATH names
it (S6 already says this; enforce it). **Not justified for instruments that read the PROCESS** — `ledger-check`,
the r15 masks, brief-staleness guards. Freeze the class; what exists may stay. And **no instrument of either
class substitutes for the real-data run**: a §6065 signature is a claim about the world, every instrument here
is a claim about the code, and the one instrument with a different oracle costs two hours of the owner's time
and has been ready for nine days.

---

## 6. What should have happened instead, 09-13 → 09-16

**09-13**, on hearing "W-2 + crypto + itemized; retirement first": ask *why retirement*. Dispatch two agents,
not twenty-seven — the drive walk on the owner's shape, and the retirement spec's review round that had been
parked as Phase 6's entry gate since 09-04. Tell the owner FR-64 is ready. **09-14**: fix the walls the drive
walk found on the owner's shape — FR-199, FR-200, FR-225; that part happened and was right — fold the
retirement review, write the plan. **09-15**: build retirement. **09-16**: seam review. The owner runs FR-64
whenever they have two hours, and the first real diff in this project's history lands this week.

Instead the review round ran at 23:02 on 09-15, the build has not started, and FR-64 has not been mentioned.
