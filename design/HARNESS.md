# A harness to keep the assistant on track — design

**Written 2026-07-30**, after a session in which doctrine was written down and then violated *the same
day*. The user's diagnosis: *"not doing what you are supposed to lowers your value considerably."*

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
| **F1** | Built `design/forms/` as a primary-source archive **without checking whether one existed**. `legal/primary-sources/` already held 16 × 26 USC, 6 × 26 CFR, 11 guidance docs — including forms that overlap directly. | *concluded from not having looked* |
| **F2** | Enumerated from a **range or hand-list instead of the source**, three times: a `BTreeSet` from `1..=38` when the label set is 48; a rounding check keyed to a hand-list of 3 parts (deleting a part red nothing); a citation check keyed to 22 hand-picked quotes. | *false completeness* |
| **F3** | **Committed with `make check` RED** — ran the gate and committed in the same command without reading the output. | *gate skipped* |
| **F4** | Claimed a checker was working when it was **blind to the case that mattered** — `cite-check` only read `*"…"*` spans, so the rounding-direction table it was meant to protect was never checked. Found only by mutating the document. | *unverified guard* |
| **F5** | Truncated a payload between workflow agents, then **reported the artifact as a finding** ("10 of 12 labels omitted"). | *harness artifact as signal* |

★ Every one is mechanically detectable. That is what makes a harness possible at all.

## The harness

Ordered by **(value × certainty) ÷ cost**. Each must itself be mutation-verified — a hook that does not
fire is decoration, and shipping decoration is F4 again.

### H1 — `pre-commit` git hook running the gates ★ build first

Catches **F3** completely and costs almost nothing. `make check` + `cargo fmt --all --check`; refuse the
commit on failure. The remaining three gates (msrv, isolation, pii-scan) are slower — run those in
`pre-push`, which is the right granularity anyway since pii-scan reads HEAD.

★ It must NOT be bypassable by habit. `--no-verify` should be reserved for a stated reason, and the hook
should print what it is protecting rather than just failing.

### H2 — a test that forbids TWO primary-source archives

Catches **F1** as a standing invariant rather than a memory. Assert that exactly one directory tree is the
declared authority for IRS primary sources, and that anything under another path is either a runtime asset
(`crates/btctax-forms/forms/` — the fillable AcroForm templates the emitter needs) or listed in a shrinking
excuse list with a reason. **Reds today**, which is correct: the reconciliation is real work and is
recorded in `CONTINUITY.md`.

### H3 — `PreToolUse` hook on `Write` for new top-level paths

The only mechanism that fires **at the decision point**, which is where F1 actually happened. When a
`Write` creates a file under a top-level directory that does not yet exist, interrupt with: *"new
top-level path — has a search for an existing one been run this session?"* It cannot verify the search
happened; it can force the question at the moment it is answerable. ★ Deliberately narrow: firing on
every write would be noise, and a noisy hook gets disabled, which is worse than no hook.

### H4 — a lint for enumeration-from-a-literal

Catches **F2**, the highest-value class and the hardest to mechanise. Heuristic, so it must be advisory,
not blocking: flag a literal array or `N..=M` range within a few lines of `expected`, `all_`, `_count`, or
`assert_eq!(…len()`, and ask whether the set is derived from the source. ★ An advisory that fires often
will be ignored — tune it against the three real instances above before shipping it, and if it cannot be
made precise, **prefer H2's shape instead**: encode the specific invariant per case.

### H5 — a workflow-script lint

Catches **F5**. Flag `.slice(` / `.substring(` applied to anything passed into an `agent()` prompt.
Truncating a payload silently converts "not sent" into "not found", and a reviewer will faithfully report
the artifact as a defect.

## What this deliberately does NOT attempt

- **No "be more careful" instructions.** They are what already failed; adding more of them is the null
  action wearing a costume.
- **No blocking gate on judgement calls.** H1-H2 gate *facts*; H3 only *asks*. Anything that tries to
  gate a judgement will be routed around, and routing around a gate teaches that gates are routable.
- **No self-verification scaffolding.** Global `CLAUDE.md` forbids it, and rightly: it produces
  over-verification with no quality gain. These are all *inline mechanical* checks, not "review your work"
  prompts.

## Honest limits

H1, H2 and H5 are near-certain. H3 depends on hook support and on staying narrow enough not to be muted.
**H4 is the one that matters most and the one least likely to work** — F2 is a reasoning failure with a
syntactic shadow, and the shadow is faint. If H4 cannot be made precise, the fallback is not a better
lint: it is to encode each instance as its own invariant, which is what H2 does and what the label-reader
design already specifies (*enumerate from the extract, never from a range*).

★★ The measure of this harness is not that it exists. It is whether a future session **fails a gate it
would otherwise have walked past.** Until that happens at least once, treat it as unproven.
