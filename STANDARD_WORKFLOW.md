# Standard Workflow — spec-and-plan-gated, review-to-green delivery

A reusable, domain-neutral process for taking a non-trivial change from idea to
shipped. The spine: **every written design artifact, from the spec onward, passes
an independent review loop that runs until zero blocking findings remain, and no
work proceeds past a gate while a blocking finding is open.** Extracted from a
working project; stripped of domain specifics so it drops into any AI-coding
tool's instructions file (`CLAUDE.md` / `AGENTS.md` / `.cursorrules` / a system
prompt) or a human runbook.

---

## 0. The one rule everything else serves

**No work proceeds past a gate while a blocking finding is open.**

A *gate* is any point where work would otherwise start building on an artifact
that has not yet passed review — crossing from spec to plan, from plan to code,
from one phase to the next — **or** any transition that is expensive to walk back
(merge, tag, ship). A *blocking finding* is anything a reviewer rates **Critical**
or **Important** (see the rubric in §6). The spec gate, the plan gate, and every
implementation phase are all gated the same way: converge to **0 Critical /
0 Important** *before* crossing.

This is a hard gate, not a guideline. "It's a small/mechanical change" is the
rationalization the rule exists to override.

---

## 1. The phase sequence

```
Brainstorm -> Spec --> [review loop -> green] --> Plan --> [review loop -> green]
   --> Implement (phased, TDD; each phase has its own review loop -> green)
   --> Post-implementation whole-diff review -> green --> Ship
```

Each arrow into "green" is the **review loop** of §2. You do not skip a box
because the work feels simple — the boxes get *shorter* for simple work, never
removed.

### Phase A — Brainstorm (idea -> agreed design)
Collaborative, before any artifact is written. Explore existing context first.
Ask clarifying questions **one at a time** (purpose, constraints, success
criteria). Propose **2–3 approaches with trade-offs and a recommendation**, not a
single take. Present the design in sections scaled to their complexity and get
agreement section by section. Output: an agreed design direction. *Do not write
code or scaffolding here.* (The brainstorm is settled by agreement, not by the
independent review loop; the spec — the first reviewed artifact — backstops it.)

### Phase B — Spec -> review-to-green
Write the design as a self-contained spec document. Then run the **review loop**
(§2) on the spec until 0 Critical / 0 Important. The spec is the contract; a
weak spec is the cheapest place to catch a problem.

### Phase C — Plan -> review-to-green
Turn the green spec into a concrete implementation plan: phases, **each phase's
acceptance criteria (its definition-of-done)**, files touched, test strategy,
sequencing, risks. Run the **same review loop** on the plan until 0 Critical /
0 Important. The plan is gated independently of the spec — a sound spec can still
get an unsound plan.

### Phase D — Implementation (phased, test-first)
Execute the green plan in phases. Per phase:
1. **Write the tests first** — they encode the phase's acceptance criteria (the
   definition-of-done named in the plan).
2. Implement until they pass.
3. Run the **review loop** on that phase's diff until 0 Critical / 0 Important
   *before* starting the next phase.

Those tests stay as the project's **regression net**: §6's full-suite reviews and
every later phase re-run them, so a break introduced downstream is caught — the
tests are an asset kept, not a gate passed once.

Prefer **one implementer carrying the whole plan** over several agents
re-implementing the same thing in parallel — parallel re-implementations diverge
and create a merge/selection problem. Parallelism belongs in *research* and in
*independent* work (separate files/modules/repos), not in redundant builds of one
artifact. Isolate in-progress work (a branch / worktree / sandbox) so a failed
phase is cheap to discard.

**A material change re-enters the process.** If implementation shows the spec or
plan was wrong — scope grows, an assumption breaks — stop and re-enter at the spec
or plan gate (review-to-green) for the changed part. A design change is never
smuggled in as "just another phase."

### Phase E — Post-implementation whole-diff review (mandatory, non-deferrable)
After the last phase, an **independent, adversarial** review of the **entire
diff as one system** — not a re-run of the per-phase reviews. The earlier reviews
checked *plan correctness*; this one catches *implementation-introduced*
regressions and cross-phase inconsistencies that phase-scoped reviews can't see
(constants that disagree across modules, a layering contract that drifted, a
guarantee the plan promised but the code doesn't deliver). Run the review loop on
it until green. Only then ship.

---

## 2. The review loop (the engine of the whole process)

Every "-> green" above is this loop:

```
1. Dispatch an INDEPENDENT reviewer over the artifact (spec / plan / phase diff /
   whole diff). Independent = a separate agent/model, or at minimum a fresh-
   context adversarial pass — not the author re-reading their own work.
2. PERSIST the reviewer's full output VERBATIM to a reviews directory BEFORE
   doing anything with it (Critical / Important / Minor / Nit sections + precise
   file:line citations). Transcript-only review text is unrecoverable later.
3. If 0 Critical / 0 Important -> GREEN. Exit the loop.
4. Otherwise FOLD the findings into the artifact — revise it so each Critical /
   Important is resolved (fixed, or cleared per "Clearing a blocking finding"
   below).
5. Re-dispatch the reviewer over the revised artifact — back to step 1.
```

(**Solo human, no second reviewer:** you can't get a fresh context by fiat, so
manufacture real separation — take a deliberate delay, then review against the
rubric as adversary-of-record with a written checklist; bring in a second pair of
eyes whenever the stakes justify it. The guarantee is only as strong as this
separation.)

**The loop continues after *every* fold — including the last one.** Folding a
finding can introduce new drift, so a fold is never "done" until a fresh review
confirms it. "Reviewed once -> fixed -> shipped" is insufficient: the fix itself
is unreviewed. This applies to *post-implementation* folds too — a "mechanical"
correction made just before shipping re-enters the loop; it does not get
self-verified and waved through.

### Clearing a blocking finding — two ways, never a third
A Critical/Important is cleared **only** by:
- **Fold** — change the artifact so the finding no longer applies; or
- **Adjudication** — a *second, independent* reviewer agrees the finding does not
  hold. Record both the original finding and the ruling in the persisted review.
  (A second reviewer is the *floor*; for a high-stakes finding, §3's "majority to
  clear" bar raises it.)

**The author may never unilaterally downgrade or dismiss a blocking finding on
their own say-so** — that is the §0 rationalization in another costume. If the
loop will not converge (the same finding survives repeated folds, or two
reviewers deadlock), **escalate to a named tie-breaker** — a designated
maintainer / senior reviewer — whose ruling is recorded. If nothing can clear it,
the change is **parked or abandoned, not shipped.**

---

## 3. When multi-agent / parallel orchestration helps (and when it hurts)

Default to orchestration for substantial work; run trivial edits, version bumps,
and plain Q&A solo. Map parallelism to the phase:

- **Research / recon — fan out.** Parallel investigators, each covering a
  different angle, then synthesize. **Any claim about an external fact (a protocol
  spec, an API contract, a standard, third-party behavior) MUST be verified
  against the authoritative source text — not against the draft you're reviewing.**
  Independent agents reading the same draft will happily reach *false consensus*
  on a plausible-but-wrong fact. The draft is not a source.
- **Design / spec / plan — single author + the review loop.** One coherent
  authorial voice; rigor comes from the independent reviewer, not from
  co-authoring.
- **Implementation — single implementer** (see Phase D). Worktree/branch
  isolation if multiple independent work-streams run at once.
- **Review — independent and, for high-stakes findings, redundant.** For a
  correctness- or safety-critical claim, use several skeptics each prompted to
  *refute* it (or each with a distinct lens), and require a majority to clear it.
  Diversity of attack beats a single reviewer.

If a planned independent review can't be obtained — a reviewer is unavailable, an
agent dispatch fails, a tool is down — **say so explicitly and defer the formal
review**; never silently substitute the author's own inline self-review for the
independent one.

---

## 4. Artifacts & persistence

Keep a stable, greppable set of artifacts so the audit trail survives across
sessions and people:

- **Design docs** — one per stage: a brainstorm/design doc, a spec, an
  implementation plan. (Suggested naming: `BRAINSTORM_*`, `SPEC_*`,
  `IMPLEMENTATION_PLAN_*`.)
- **Reviews directory** — every reviewer's verbatim output, one file per round
  (e.g. `reviews/<topic>-<stage>-round-N.md`). Persisted **before** folding.
- **Follow-up tracker** — a single file of open/resolved action items
  (`FOLLOWUPS.md` or similar). Each entry: what, why, status, and a pointer.

Two persistence disciplines that repeatedly pay off:
- **Persist the review before you fold it.** (See §2 step 2.)
- **Verify citations at write time.** Line numbers and "as of" references decay
  every commit. When a plan/spec cites source, re-check it against the *current*
  source as you write, and record the source revision for future readers.

---

## 5. Roles

Three hats. They can be three agents, three people, or one actor switching
context — but the **author and the reviewer hat must not be worn at the same time
on the same artifact**; the review's value is its independence.

- **Author** — writes the spec and the plan.
- **Reviewer ("architect")** — independent adversarial review against the gate
  rubric; produces the persisted findings.
- **Implementer** — writes and tests the code, executing the green plan.

For *any* artifact, its producer wears the Author hat, and the independence rule
(author ≠ reviewer, at the same time, on the same artifact) applies to code
exactly as it applies to the spec and the plan.

---

## 6. Severity rubric (what gates, what doesn't)

| Severity | Meaning | Gates? |
|---|---|---|
| **Critical** | Wrong result, data loss, safety/security hole, or a guarantee the artifact claims but doesn't deliver. | **Yes — blocks.** |
| **Important** | Real defect, missing case, or unsound assumption that will bite; not catastrophic. | **Yes — blocks.** |
| **Minor** | Quality/clarity/maintainability; safe to ship, worth doing. | No — fix or file. |
| **Nit** | Cosmetic / preference. | No. |

Green = the full validation suite passes **and** **0 Critical / 0 Important**. A
red suite (failing tests / lint / build) is itself a blocking finding. Minors and
Nits are recorded (fixed inline or filed as follow-ups) but do not hold a gate.

Two rubric disciplines:
- **Reviews run against the *whole* validation surface, not a narrow slice.**
  Run the full test/lint/build suite, not just the targets the current phase
  touched — a change in one area ripples into checks outside it, and a
  narrowly-scoped review goes green while a suite-level failure hides.
- **Don't manufacture findings to look thorough, and don't rubber-stamp.** If
  it's green, say so plainly.

---

## 7. After shipping

- **Offer to burn down the follow-ups this change generated.** Right after ship,
  enumerate the newly-filed items + rough effort, and offer to clear them (all or
  a chosen subset) through the same gated process.
- **Flip tracker status in the shipping commit.** Status drifts from reality
  fast; reconcile "open vs done" at decision time, and resolve a follow-up in the
  same commit that ships its fix.
- **A defect found after ship re-enters the workflow.** An escaped defect is a
  new (usually small) change: it gets a short spec/plan and is gated like any
  other. Production pressure is exactly when the gate is most tempting to skip and
  most needed — a hotfix is *scaled down* (§8), not *un-gated*. **Start the fix
  with a test that reproduces the defect and *fails first*** — proving it actually
  exercises the bug — then implement to green. A test that passes against the
  unfixed code is coverage theatre, not a fix.

---

## 8. Scaling the ceremony

The process is **fixed in shape, variable in depth.** A one-paragraph spec, a
single-skeptic review, and a two-phase implementation is a complete, faithful run
of this workflow for a small change. A large change gets a long spec, redundant
multi-lens reviews, and many phases. What you never drop, at any size:

1. an independent review before each gate,
2. the loop that re-reviews after every fold, and
3. the gate itself — no crossing on an open Critical/Important.

The solo-vs-orchestrate threshold is the one from §3: trivial mechanical edits,
version bumps, and conversational answers run solo with no ceremony; everything
substantial runs the full shape above.
