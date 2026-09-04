# A harness to keep the assistant on track — design

**Written 2026-07-30**, after a session in which doctrine was written down and then violated *the same
day*. The user's diagnosis: *"not doing what you are supposed to lowers your value considerably."*

**Revised 2026-07-30 (r2)** after an independent Fable consult — verdict `needs-changes`, persisted
verbatim at [`reviews/harness-design-fable-r1.md`](../reviews/harness-design-fable-r1.md). The r1 draft
enumerated **five** mechanisms against **five** symptoms. That was the excuse-list mistake this document
itself warns against: an enumeration of the outcomes one session happened to produce. This revision is
organised around the **two classes** the five failures actually belong to.

## The diagnosis, in this project's own vocabulary

The process has the exact defect the code spent this session removing: **held by convention, not
construction.** `CLAUDE.md` and the memory files are *passive context* — read at session start, then
violated 40 tool calls later, mid-flow, while executing rather than reflecting. They are the
answered-ness invariant's twin: a rule with no mechanism is a rule that silently doesn't apply.

★ The project's own standard applies to itself: **a guarantee without a test that reds when it is removed
does not exist.** None of this doctrine has a test.

★★ **And a harness cannot make the assistant think.** It can only make *specific, recurring, enumerable*
omissions loud. So it must be built from **observed failures**, never from good intentions — the same
reason an oracle excuse list is computed from the defect's mechanism rather than from the outcomes
someone happened to see.

## The observed failures — 2026-07-30, one session

| # | what happened | class |
|---|---|---|
| **F1** | Built `design/forms/` as a primary-source archive **without checking whether one existed**. `legal/primary-sources/` already held 16 × 26 USC, 6 × 26 CFR, 11 guidance docs — including forms that overlap directly. | **α** |
| **F2** | Enumerated from a **range or hand-list instead of the source**, three times: a `BTreeSet` from `1..=38` when the label set is 48; a rounding check keyed to a hand-list of 3 parts (deleting a part red nothing); a citation check keyed to 22 hand-picked quotes. | **β** |
| **F3** | **Committed with `make check` RED** — ran the gate and committed in the same command without reading the output. | **α** |
| **F4** | Claimed a checker was working when it was **blind to the case that mattered** — `cite-check` only read `*"…"*` spans, so the rounding-direction table it was meant to protect was never checked. Found only by mutating the document. | **β** |
| **F5** | Truncated a payload between workflow agents, then **reported the artifact as a finding** ("10 of 12 labels omitted"). | **β** |

★ Every one is mechanically detectable. That is what makes a harness possible at all.

## ★★ The two classes — this is the load-bearing structure

Five symptoms, two mechanisms. Building against the symptoms produces a harness that is exactly as long
as one session's bad luck; building against the classes produces one that can fire on a failure nobody
has seen yet.

| | class | failures | what actually went wrong | the cure's shape |
|---|---|---|---|---|
| **α** | **Acted without observing an available fact** | F1, F3 | The fact was present, cheap, and unread. F1 was never *considered and refused* — it was never considered. | **Couple the act to the fact** at the decision point, so proceeding requires the observation rather than recommending it. |
| **β** | **Shipped an instrument never seen discriminating** | F2, F4, F5 | A measuring device was trusted without once being watched to distinguish a true case from a false one. | **Require the kill.** No instrument counts as existing until it has been observed red on a planted defect. |

★ F5 belongs to **β**, which is not obvious: a harness whose truncation manufactures findings is an
*instrument defect*, not a communication slip. The reviewer read the instrument, not the world.

★★ **β is the project's own mutation doctrine turned on its own instruments.** *"A guarantee without a
test that reds when it is removed does not exist"* — applied to checkers, censuses, and review harnesses
rather than to features. And it cannot be satisfied performatively: an honest kill-test for a blind
checker **cannot be written without discovering the blindness**. That is literally how F4 was found.

## ★★ Scope bound — this harness is btctax-ONLY

**User constraint, 2026-07-30.** Claude is used across many projects; this harness is tuned to *this*
repo's observed failures and would be wrong-to-actively-harmful elsewhere (A3 knows about IRS form
stems; A2 assumes `make check`; the seen-red-once rule is calibrated to a codebase with 2450 tests and
mutation-verification). **A harness grown from one project's failure data must not leak into projects
whose failure data nobody has looked at** — that would be F2, false completeness, at the largest
possible scale.

So every artifact lands **inside this repository**, and the check is *mechanical*, not a resolution:

| mechanism | lands in | why that is repo-scoped |
|---|---|---|
| A1, A3 test half | `crates/xtask/` | ordinary workspace code; runs only via this repo's suite |
| A2 hooks, A3/A4 hook scripts | `scripts/` + **repo-local** `git config core.hooksPath` | git config written without `--global`, so it lives in this repo's `.git/config` |
| A3/A4 `PreToolUse` wiring | **`.claude/settings.json` in this repo** | project-scoped by construction — Claude Code loads it only when working in this directory |
| B1, B2 standing rules | **`/scratch/code/bitcoin_tax/CLAUDE.md`** | the project file |

★★ **Three files are explicitly OFF LIMITS for anything in this document**, because each one would
apply the harness to unrelated projects:

- **`~/.claude/settings.json`** — global hooks; would fire in every repo on this machine.
- **`~/.claude/CLAUDE.md`** — global doctrine.
- **`/scratch/code/CLAUDE.md`** — the *cross-project* standard-workflow file covering every project
  under that directory. Its generality is the point; harness specifics do not belong there.

★ If a mechanism here later proves itself general, promoting it is a **separate, deliberate decision**
with its own evidence — never a side effect of building it for btctax.

## The mechanisms

Ordered by **(value × certainty) ÷ cost**, and by dependency — **A1 comes first because without it every
other mechanism here is decoration.**

### Class α — couple the act to the fact

#### A1 — the harness-is-installed gate ★ build first

An `xtask` check inside `make check` that **reds while the hooks are not wired**: `core.hooksPath` unset,
hook file missing, or hook file non-executable.

★★ **This is not hypothetical, and it is why it is first.** `scripts/pre-push` is a reviewed, hardened
PII gate, executable, in this repo since 2026-07-02. `core.hooksPath` is unset; `.git/hooks/` contains
nothing but `*.sample`. **It has never run.** Worse, the install command is written in its own header —

    # Install:  ln -s ../../scripts/pre-push .git/hooks/pre-push
    #       or: git config core.hooksPath scripts

— so this repo already contains a *written-down instruction that was not followed*, which is the very
failure mode the harness exists to answer. Shipping A2 without A1 is F4 on day one: an instrument that
was never seen to fire.

It also genuinely fires on its own: every fresh clone and every mis-configured worktree reds immediately.
**Mutation-verified by unsetting `core.hooksPath` and observing red.**

★ A1 must also cover `scripts/pre-push` itself — the corpse gets buried as part of this work, not left
as an exhibit.

#### A2 — `pre-commit` hook running the gates

Catches **F3** completely and costs almost nothing. `make check` (~6s warm: nextest + clippy in parallel)
plus `cargo fmt --all --check`; refuse the commit on failure. The slower gates (msrv, isolation,
pii-scan) belong in `pre-push`, which is the right granularity anyway since pii-scan reads HEAD.

Git's exit-status coupling is what makes this a real fire rather than a reminder: the commit *cannot*
happen while the gate is red, so "ran it and didn't read the output" stops being expressible.

★ **The route-around is closed with a fact-gate, not an instruction.** A narrow `PreToolUse` deny on
`Bash` matching `git commit --no-verify`. Telling oneself to reserve `--no-verify` for a stated reason is
exactly the passive doctrine that fails; a deny is not. The hook prints what it is protecting rather than
just failing.

#### A3 — the primary-source shape detector (one implementation, two call sites)

Catches **F1**, as a standing invariant rather than a memory. Exactly one directory tree is the declared
authority for IRS primary sources; anything else is either a runtime asset
(`crates/btctax-forms/forms/` — the fillable AcroForm templates the emitter needs) or sits in a
shrink-only excuse ratchet with a reason.

★★ **Classify by SHAPE over a whole-tree walk — never by a hand-list of known directory paths.** IRS
stems (`fNNNN`, `iNNNN`, `pNNNN`), USC and CFR signatures. A path-list version is **F2 committed inside
the harness**: it passes a third archive at a new location, which is precisely the case it exists to
catch. In-repo model for the ratchet: `AUTHORITY_NOT_YET_ARCHIVED` and
`authority_coverage_may_only_improve` (`crates/xtask/src/cite_check.rs:685`, `:750`).

Two call sites, one detector:

| call site | when | effect |
|---|---|---|
| **test** | `make check` | reds on a second archive anywhere in the tree |
| **`PreToolUse` deny on `Write`** | at the decision point | refuses a primary-source-shaped file written outside the declared tree |

★★★ **It paid for itself on its first run — 2026-07-30.** `CONTINUITY.md` recorded **two** archives.
The walk found **four**: `design/amt-form6251/` (18 files, byte-identical duplicates of
`design/forms/2025/`) and `legal/text/` (25 files, a text-extract layer of `legal/primary-sources/`
with 100% overlap and zero unique documents) had **never been named anywhere**. A hand-list of known
archive paths would have found neither — it would have listed the two that were already remembered,
which is the whole reason the rule is *shape, not path*.

★ And the recorded "two" was itself **F2**: a count written from memory rather than from a walk, inside
the note warning against exactly that. The number is now measured and pinned, so step ③'s progress is a
test result rather than a claim.

**The r1 draft said this test should simply RED today.** It does not, and the change is deliberate:
with A2 wired, a permanently-red suite would block *every commit* until the reconciliation landed, and
a gate that stands red forever is not a gate — it is noise that gets muted, which is the failure this
document is written against. Instead the four trees sit in a **shrink-only ratchet** with a reason
each (`AUTHORITY_NOT_YET_ARCHIVED` is the in-repo model): a **fifth** archive reds immediately, and
retiring one reds the stale entry so the list must tighten. The reconciliation stays real work, is
still recorded as ③ in `CONTINUITY.md` §0, and is now *tickled* rather than remembered.

★★ **UPDATE 2026-09-04 — the tickle fired twice, then discharged.** It reset once (2026-08-13 →
2026-08-28) and fired again on 2026-08-29, holding every commit and reddening `main`'s CI. That
second firing was answered by doing the work rather than by a third date: both residual duplicate
groups were resolved (`DUPLICATE_SOURCE_GROUPS` 7 → 0) and the constant, its test and the `run()`
branch were retired **with their subject**. The guard that remains is stronger than the date it
replaced — a pin at 0 reds on any duplicate immediately, with nothing to renew — and the RESET LOG
is kept verbatim in `archive_check.rs` so both extensions stay readable without git history. The r1
instinct ("this should simply RED today") was right about the *pressure* and wrong about the
*mechanism*: what worked was a deadline that forced a decision, not a permanent red.

★ The r1 draft had the hook fire on **new top-level paths**. That **provably would not have caught F1**:
`design/forms/` is depth-2 under a `design/` dating to 2026-06-28. The trigger sees no new top-level path
and walks straight past the incident it was designed for.

#### A4 — the new-directory ask

The only mechanism that merely *asks*, and the narrowest. Fires on any new **directory** creation at
**any depth** — rare enough not to become noise, and a noisy hook gets muted, which is worse than no hook.

Two things make this more than passive context repeated at a louder volume. First, a `PreToolUse` exit-2
block **feeds its stderr back into the model's loop** at the moment the decision is being made — for a
failure of *inattention* (F1 was never considered and refused) that is materially different from a rule
read 40 tool calls earlier. Second, **the message quotes the trigger-relevant memory line verbatim**
rather than saying "be careful".

★ This is the answer to the consult's scope-question (c): **keep memory as principles**, but wire the two
or three trigger-shaped ones into hook messages, so doctrine surfaces at the moment it is answerable.
The memory system is not mis-shaped; it is merely mis-*delivered*.

#### ★★★ A4's first observed failure was A4's own — 2026-07-30

A4 shipped watching `Write`/`Edit` only. **Within the hour, its own author created
`design/forms/2026/` with `mkdir -p` in Bash, and the ask never fired.** It surfaced during an audit
of *"is every element actually in effect?"*: `.git/btctax-harness/acked-dirs` was **absent**, so the
ask had never once run in anger.

★★ The damning part is that **this document predicted that exact route-around, in writing**, three
paragraphs up — *"a wall would just be routed around with `mkdir -p`"* — and the prediction changed
nothing. **A documented hole is still a hole.** That is the same shape as the failure the whole
harness answers: doctrine written down, then not applied, by the person who wrote it.

**Closed** by extending the Bash hook (`scripts/hooks/deny-bypass.sh`) with the same one-shot ask,
sharing `on-write.sh`'s ack file so a directory is asked about once across both routes. Narrow on
purpose — `mkdir` only, inside the repo only, non-existent directories only, never anything git
ignores — because a noisy hook gets muted. The ALLOW cases are pinned in the kill test as the
*specification of that narrowness*, and the detector is mutation-verified.

★ **This is the harness's own rule working:** grow it from **observed** failures, never anticipated
ones. The anticipation was already in this file and was worth nothing; the observation was worth a
mechanism. Note also what the audit did *not* find — A1, A2, A3 had all fired for real. The measure
is whether a session fails a gate it would otherwise walk past, and here the harness failed its
author twice: once by blocking `git config --unset core.hooksPath`, once by exposing this gap.

★ **Known residual limits, stated rather than implied.** A3's and A4's *hook* halves see the Write
tool and `mkdir`; a file `cp`'d or redirected into a new tree from Bash still evades them. The
backstop is that A3 has a **test** half that walks the whole tree on every `make check`, so a
Bash-created archive is caught late rather than never. **A4 has no test counterpart** — for A4, "late"
does not exist, which is precisely why its coverage had to be widened rather than merely documented.

#### A5 — the tree watcher (`PostToolUse`), and why it beats both hook halves

**Owner's suggestion, 2026-07-30, and it is a better idea than what it supplements.** A3's hook half
watches the `Write` tool; A4's Bash half watches `mkdir`. Both **enumerate ways a file can appear** —
`cp`, `mv`, `tar -x`, `curl -o`, `>` redirection, `unzip`, `rsync`, a script three levels down. That
list is unbounded, and every entry on it is one somebody happened to think of. **It is the excuse-list
mistake, committed inside the harness that warns about it.**

`scripts/hooks/post-tree-watch.sh` watches the **tree** instead of the **command**: a sorted listing
after every Bash call, diffed against the previous one, with the shape detector run over whatever is
new. It cannot be routed around by choosing a different tool, because it does not care which tool ran
— only what is now on disk. Same rule the oracle excuse lists follow: *state the mechanism, let it
decide, never enumerate the outcomes you happened to see.*

| | |
|---|---|
| **Cost** | ~8-21 ms (1833 files, build dirs pruned). Measured, because a slow `PostToolUse` hook is one that gets disabled. |
| **Why a walk, not `git status`** | `git status` is 3 ms but **blind to a new file matching a gitignore rule**, and a detector whose correctness depends on `.gitignore` is not a detector. |
| **What it cannot do** | Undo. It runs *after* the tool. But it turns "caught at the next `make check`" into "caught within one tool call". |
| **Deliberately not reported** | New *directories*. After-the-fact directory reports are noise, and noise gets a hook muted. |

★★ **This closes the `cp` limit recorded above.** A3's coverage is now mechanism-independent, and the
Write/`mkdir` hooks are demoted to what they are uniquely good for: firing *before* the act, at the
decision point, which no `PostToolUse` hook can do.

#### ★★★ Two false positives in two real uses — the verdict on shell parsing

The `mkdir` ask (A4's Bash half) cried wolf **twice, on its first two real invocations**, both while
building A5:

1. `mkdir -p $S` — `shlex` does not expand variables, so the literal `$S` was read as a repo-relative
   path and the hook asked about a directory that actually lives in `/tmp`.
2. `... "$S"; run() { … }` — `shlex.split` leaves `;` **attached** to the preceding word, so the
   separator was never seen, the scan ran past it, and `run()` from a shell function definition was
   read as a directory name.

★★ **The lesson is not "fix the regex".** It is that *statically parsing arbitrary shell to predict
filesystem effects is the wrong instrument* — which is precisely why A5 exists. A4's parser is kept
only for the one thing A5 structurally cannot provide (the **pre**-hoc ask) and is now deliberately
timid: `punctuation_chars` so separators really separate, a strict path filter, and **an unresolvable
target fails OPEN**. Failing open is safe *because* A5 observes the resulting tree regardless.

★ A hook that cries wolf gets muted, and a muted hook protects nothing — so the ALLOW cases are pinned
in the kill test as **the specification of that narrowness**, not as padding.

### Class β — require the kill

#### B1 — seen-red-once, as a standing rule

**No checker counts as existing until it has been observed red on a planted defect.** Every new census,
conformance check, citation check, or review harness lands **paired** with a negative test that plants
the exact defect it exists to catch and asserts red.

In-repo model: `a_paraphrase_is_rejected_and_the_real_sentence_is_accepted`
(`crates/xtask/src/cite_check.rs:796`).

★★ **This replaces the r1 draft's H4 lint, which is dropped.** A lint for "enumeration from a literal"
would only look like rigour: Rust test code is legitimately full of literal arrays and ranges, so an
advisory firing on those is ignored within a week — and an ignored advisory is decoration that also
teaches that harness output is ignorable. B1 covers F2 **and** F4 as one class, cannot be satisfied
performatively, and produces real future failures rather than the appearance of them.

★ Where a specific enumeration invariant *can* be stated exactly — *"enumerate from the extract, never
from a range"* — encode it as its own invariant in A3's shape, which is what the label-reader design
already specifies. The general lint is the thing that does not work; the specific invariant does.

#### B2 — pass-by-path payloads

**Inter-agent payloads move as file paths the receiver reads — never as inlined content.** Truncation
then has no operator to apply: there is nothing to `.slice()` because the payload is a path.

★★ **This replaces the r1 draft's H5 lint, which is dropped as having no target.** Verified: `.slice(`
and `.substring(` occur in exactly two committed places in this repo — `CONTINUITY.md:67` and this
document's own r1 text — i.e. only in the *description of the proposed lint*. Zero occurrences in code.
The truncating code was **ephemeral orchestration**, which a lint over committed files can never see.
B2 removes the class instead of shadowing one symptom of it, and it belongs in the standing workflow
briefs (`CONTINUITY.md`-style), where the orchestration is actually authored.

#### B3 — a per-range review is not a branch review

**The LAST review before an irreversible action must be scoped to the WHOLE branch and pointed at
INTERACTION — not at correctness-per-commit.** A stack of range-scoped reviews does not add up to a
branch review, however diligent each one was.

★★★ **Observed 2026-07-31, and it is the only class-β mechanism here with a defect to its name rather
than a near-miss.** Three independent reviews ran on `feat/no-pen-deferrals`. r1 covered
`7bde148..65270db`; r2 covered `afa0ffe..HEAD`. Both returned 0 Critical / 0 Important. The pre-publish
Fable pass — the first scoped to `main..HEAD` — found an **Important**: Schedule B's FBAR sub-question
pair printed ungated, so a stored answer orphaned by a 7a Yes→No correction put a checked FinCEN-114
box beside a checked "No", under §6065.

It lived in `b94508d`, **the earliest commit on the branch, which precedes BOTH earlier review ranges.**
Every round was thorough inside its window and the defect lived outside every window.

★★ **And the fix already existed in the branch.** Nine commits later, Schedule C line J received exactly
the missing gate, with the reasoning written out in its own comment — *"an answer to J without a Yes on
I is not a mark the form has a place for."* Nobody carried it back, because **no reviewer ever held both
commits at once.** That is the failure mode: not ignorance, but a field of view.

**The rule.** When a branch spans more than one sitting:
- each round may be range-scoped (it is cheaper and the briefs stay sharp);
- **but the final pass takes `main..HEAD`**, and its brief must say so and must name interaction as the
  target — advisories that now fire together, questions whose class changed across commits, one
  concrete filer walked end to end.
- ★ Tell it explicitly what the earlier rounds covered, so it spends its budget on the seams rather
  than re-deriving their findings.

★ **The measure, as for every mechanism here:** it is not that a final pass happens. It is whether it is
scoped to something the earlier passes could not see. A `main..HEAD` review that merely re-reads the
last commit is the same blind spot with a wider `git diff`.

## What this deliberately does NOT attempt

- **No "be more careful" instructions.** They are what already failed; adding more of them is the null
  action wearing a costume.
- **No blocking gate on judgement calls.** A1-A3 gate *facts*; A4 only *asks*. Anything that tries to
  gate a judgement will be routed around, and routing around a gate teaches that gates are routable.
- **No self-verification scaffolding.** Global `CLAUDE.md` forbids it, and rightly: it produces
  over-verification with no quality gain. These are all *inline mechanical* checks, not "review your
  work" prompts.
- **No session-shaping — no checkpoint cadence, no new required opening action.** The consult's answer to
  scope-question (d) is **no**: the required opening already exists (`CONTINUITY.md` §0, and the consult
  that produced this revision is evidence it is followed), and a checkpoint cadence *is* the forbidden
  self-verification scaffolding. The only session-shaping worth having is the decision-point hooks in A3
  and A4.

## Honest limits

**A1 and A2 are near-certain.** A3's test half is certain; its hook half depends on `PreToolUse`
behaving as documented and on the shape detector being precise enough not to fire on ordinary writes.
A4 is the one that could still be muted, and it is deliberately the only one whose loss costs nothing
structural.

**The class-β rules are process, not code**, and that is their weakness: B1 and B2 have no compiler.
B1's saving grace is that it is checkable at review time by a single question with a factual answer —
*"which test reds when this checker is removed?"* — and B2's is that a path-passing brief makes the
violating form awkward rather than merely discouraged.

★★ **The measure of this harness is not that it exists.** It is whether a future session **fails a gate
it would otherwise have walked past.** Until that happens at least once, treat it as unproven.

★★ **And the assumption everything here rests on**, named by the consult so it can be checked rather than
forgotten: *all of this assumes the failures recur in mechanically recognisable form.* If the next
session's violations are new **classes** rather than new instances of α and β, every gate here holds
green while the new failure walks past. The only defence is the rule above — treat the harness as
unproven, and **grow it only from observed failures, never from anticipated ones.**
