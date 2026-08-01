# §G-11 — ARCHITECT CONSULT BRIEF

You are being asked for **architecture judgment before any code exists.** Nothing is built. No spec is
written. The brainstorm is one commit old and is explicitly not a decision.

## Read these two, in order

1. `/scratch/code/bitcoin_tax/design/no-testimony/BRAINSTORM.md` — the shape, the number, the seven
   mechanisms, my recommended phasing, and the four owner decisions.
2. `/scratch/code/bitcoin_tax/design/no-testimony/MAP-survey.md` — the evidence: 221 money rows
   classified across 16 printed structs + Forms 6251/8959/8960/8995, against the extracted form text.

Then `/scratch/code/bitcoin_tax/CLAUDE.md` — **"an entry is testimony"** and **"blank is the normal
case"** are the authority. And `FOLLOWUPS.md` §G-11 for the original filing.

## The problem, in four sentences

A blank line and a printed `0` are different speech acts on a return signed under 26 USC §6065: a blank
says nothing, a `0` swears the amount **is** zero. btctax has no representation for "not stated" in its
money path, so **64 of the 168 money quantities that reach a PDF can print a zero the filer was never
asked for** — fabricated testimony under someone else's signature. The filed follow-up proposed growing
the emitter's money type; the survey found **62 of the 64 are manufactured upstream**, in
`return_inputs.rs`, `return_1040.rs` and the printed-struct constructors, so that fix would address two.
btctax has exactly three lawful moves per line — **collect** the testimony, **refuse** the return, or
leave **genuinely blank** — and must never silently choose silence and present it as the filer's.

## THE ONE QUESTION

**What is the right architecture here — and is there a shape in which an omission CANNOT COMPILE,
rather than one where 64 sites are individually plumbed and the 65th is forgotten?**

That is the question I most want answered, because the repo's own doctrine says *"prefer designs in
which an omission does not compile"* and I do not know whether that is reachable for this defect. My
recommended phasing (BRAINSTORM §7) treats it as phase 3 of 4. **Tell me if that ordering is wrong, or
if the whole framing is.**

Three sub-questions I consider load-bearing. Answer them only insofar as they serve the question above:

- **Is "not stated" a property of the VALUE or of the LINE?** Both precedents exist in-repo, on exactly
  one line each: `Form8283Row.claimed_deduction` is `Option<Usd>` end-to-end (value); `form8995.rs:255`
  gates by map-cell identity (line). They imply different type surgery and different failure modes.
- **What does a total do with a not-stated operand?** Schedule 1 line 9 sums 21 operands of which 20
  have no field at all. Propagate · treat as zero and print · refuse · decide per line.
- **What is the B1 kill-test?** Every fix here is a *suppression*, and a suppression that over-fires is
  invisible — a missing cell looks exactly like the common correct case on a mostly-blank return.
  Neither oracle sees it (both take these values as INPUT). No golden moves (~62 of 64 overstate tax,
  and the goldens encode today's behaviour). The negative test must plant **both** a zero that should
  not print **and** a blank on one of the 24 lines where the form instructs `-0-`. **What single
  instrument reds on both?** If there is no such instrument, say so plainly — by this repo's B1 rule a
  checker that has never been watched go red does not exist, and a program with no gate should not
  start.

## ★ "Do not build it" is a first-class answer

The repo's most valuable architect consult to date returned exactly that for §G-14 (shred/tombstone),
and it is recorded as such. If the right answer is *build something much smaller*, *build a different
thing*, or *this is not worth the blast radius given btctax has never had a user*, say so and say why.
I would rather spend this consult learning that than get a well-argued plan for the wrong thing.

## Settled — do NOT re-derive these

1. **The counts.** 64 sites / 168 emitted quantities; band 51–64 on two counting calls; the seven
   mechanisms and their tallies; the 24 form-instructed zeros; the layer split. Five readers produced
   these against the form text and I spot-verified the load-bearing ones by hand. Take them as given.
2. **The hard scope boundary**, from §G-11 and non-negotiable: btctax must never build a heuristic that
   flags an omission as suspicious, or any feature opining on whether a blank is lawful. **Both
   directions are software adjudicating intent**, which is out of scope. Do not propose anything that
   crosses it, and flag it if my phasing does.
3. **The four owner decisions** (BRAINSTORM §8) are the owner's and are not yours to settle: whether a
   reconciled ledger is testimony; blank-vs-refuse where silence asserts; whether supplied-then-zeroed
   is in scope; sequencing against Form 6251 Tier 2. **You may say which of them a good architecture
   would make cheap to change later** — that is architecture, and is welcome.
4. Tax correctness is not in question here and there is nothing to re-audit. This is a representability
   problem.

## Context that should inform the shape

- **The mature in-repo pattern is `Option<T>` surviving to the writer** — Schedule B 7a/FBAR/8,
  Schedule C I/J, 8283 5a–5c, all built this week, each carrying a comment recording that
  `unwrap_or(false)` there had been a live fabricated-testimony bug. Same defect, one type over.
- **`classifier.rs:17` explicitly permits bare `_` on every `Usd` leaf** — so the census built to make
  answered-ness structural is blind to all 64 by design. Extending it forces human classification on
  ~200 leaves.
- **`CarryProvenance` is the in-repo sibling pattern**: provenance riding alongside a value rather than
  changing its type. Built, advisory-wired, deliberately not printed.
- btctax has **never had a user**; v0.15.0 is prepared but unpublished. Back-compat is not sacred.

## Output format

1. `ANSWER:` — the architecture, in under 200 words. If it is "do not build", say that here.
2. `WHY THIS SHAPE` — the reasoning, including what you rejected and why.
3. `THE GATE` — your answer to the B1 kill-test question, or an explicit "there isn't one, and here is
   what that means".
4. `PHASING` — if you keep a phased build, give the order and say what makes each phase independently
   green. If you reject my four phases, say which and why.
5. `WHAT WOULD MAKE THIS ANSWER WRONG` — the assumption you did not verify.

Be concrete about types and module boundaries; name files. Read code where it matters — you have a
shell. **READ-ONLY: no edits, no commits.** Do not spawn subagents.
