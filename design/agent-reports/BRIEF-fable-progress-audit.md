# BRIEF — are we spinning our wheels, or making real progress?

The repo owner asked this question directly. Answer it. You are the highest-cost model available and this
is a one-shot whole-system judgement, so **do not audit code quality and do not hunt defects** — judge the
*trajectory*. Read whatever you need; change nothing.

## The question, sharply

btctax emits US federal Form 1040 paper packets. It has **no users** and has **never** been driven
end-to-end on the owner's real data. The owner asked for **Phase 6 — EITC and retirement income** several
days ago and chose *"retirement first"*.

**Is the work of the last two days advancing that, or circling it?** And if circling — name the mechanism,
not the symptom.

## The honest numbers, measured not estimated

| | |
|---|---|
| commits in this arc | **61** |
| of those, `fix(` or `fold(` | **16** |
| of those, ledger-only (`followups(`/`ledger(`) | **19** |
| follow-up entries | 558 → **576** (+18 opened) |
| entries marked closed | 60 → **85** (+25) |
| **net open follow-ups** | 498 → **491** (−7) |
| workspace tests | **3,927** passing, gate green throughout |
| Phase 6 (retirement) | **specced, not built.** Form 1040 has no 4a–6b fields at all |
| EITC | **not specced.** One advisory saying it is not computed |

## The case FOR real progress

Live, user-visible defects fixed — each found by walking the product, not reading it:

- `income clear` destroyed **67 recorded answers** with no prompt, no undo. The product *prescribed* that
  path in five places, once as the escape from another wall. It was also an unimplemented **spec mandate**.
- An over-$750k mortgage and a **$600 bag of clothes** each refused the *entire* packet — no 8949, no
  Schedule D. Both now file (Pub 936 Table 1 and Form 8283 Section A transcribed).
- btctax **omitted a Form 8283 the instructions require** whenever a §170(b) ceiling bound.
- A standard-deduction filer donating >$5,000 **could not file at all**.
- A refusal named two cures and **both dead-ended**; an export refusal printed `btctax set-pii`, a verb that
  has never existed.

Instruments built that then caught things nobody went looking for: a refusal census that found an untested
refusal *within an hour* of landing; a year census that caught a variant added while its own author worked;
a ledger/source consistency check that has fired on **5 of 5** subsequent folds.

And the review round you are being asked about caught **3 Criticals before a build**, including a spec
building a Form 1040 line (**6d**) that does not exist in TY2024.

## The case AGAINST — and steelman this, do not soften it

1. **Phase 6 is what was asked for and it is not built.** Twice the owner asked what to do next; twice the
   controller chose the follow-up shelf and did not say *"the thing you asked for is still unbuilt."*
2. **The shelf regenerates.** 18 new follow-ups opened while 25 closed. Net −7 out of ~498. At this rate
   the list outlives the product.
3. **Five of the controller's own claims were false in one round**, caught by the review: a §G-9 oracle
   argument that was backwards, a "verified accurate" citation claim that covered a subset, a misread
   advisory, a short block, a map-row count off by 3×.
4. **Six "open Criticals" were already fixed** — the controller recommended working them as top priority.
5. **The artifact grew rather than converged**: the retirement spec went 41.5 KB → 63.5 KB in this round and
   is still **not green** (3C/11I, now 0C/11I).
6. **Much of the value is meta.** Several days produced instruments, censuses and ledger hygiene rather than
   tax features. That may be exactly right for a funds-sensitive product with no users — or it may be
   displacement activity that feels productive because it always finds something.

## What I want from you

1. **A verdict in one paragraph.** Spinning, progressing, or progressing-at-the-wrong-thing.
2. **The mechanism**, if something is wrong. The controller's hypothesis is *"the follow-up shelf is an
   infinite generator and I keep choosing it over the roadmap."* Test that hypothesis; propose a better one
   if it is wrong.
3. **A decision rule** the controller can apply *without asking the owner each time* to decide between
   "fix the defect I just found" and "build the thing that was asked for". The absence of such a rule is
   the suspected root cause.
4. **What to do next, concretely**, and what to stop doing.
5. ★ **Is the instrument-building justified?** btctax signs a return under §6065 penalties of perjury and
   has never been run on real data. Argue both sides and then commit.

★ Be blunt. A comfortable answer is worth nothing here. If the honest verdict is that two days were largely
spent on the wrong thing, say so plainly and say what should have happened instead.

## Persisting your report
`Write` is REFUSED for report files. Use a Bash heredoc:

    cat > design/agent-reports/FABLE-progress-audit.md <<'RPTEOF'
    ... your report ...
    RPTEOF

then `wc -c` it and give the byte count. Do not commit. Do not spawn subagents.
