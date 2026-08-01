# Review brief — PRE-MERGE, whole branch `feat/no-pen-deferrals`

**Range: `main..HEAD` — 34 commits.** Read it yourself: `git log --oneline main..HEAD`,
`git diff main..HEAD`. Do not ask for it to be pasted. Files you need are on disk; read them.

## The ONE question

**Is this branch safe to merge to `main`?** Concretely: can any change, or any INTERACTION between
changes made days apart, cause a filer to

1. sign a US federal 1040 that **UNDERSTATES tax** (the worst outcome, 26 USC §6065), or
2. be **BLOCKED from filing a return that is actually correct** (a false refusal with no exit), or
3. have **testimony printed that they never gave** (a mark, a "No", or a `0` on a line nobody asked)?

Nothing else gates a merge. Style, naming, and comment density are not findings.

## Why this pass exists, and what it must NOT redo

Four review rounds already ran on this branch. **All four were scoped to RANGES.**

| round | range | lenses | result |
|---|---|---|---|
| r1 | `7bde148..65270db` | Opus + Sonnet | 0C/0I |
| r2 | `afa0ffe..<then-HEAD>` | Opus + Sonnet | 0C/0I |
| pre-publish | `main..d0aad6f` (whole branch, interaction-scoped) | Fable | **1 Important** |
| r3 | `d0aad6f..HEAD~1` | Opus + Sonnet | **4 Important** |

★★★ **The branch's own history says range-scoped review MISSES THINGS.** The Fable round's Important
lived in the branch's earliest commit, outside both earlier windows. That produced harness rule **B3**
(`design/HARNESS.md`): *a per-range review is not a branch review, and a stack of them does not add up
to one.* Then r3 found the same shape AGAIN — an advisory and the deduction it describes, two commits
apart, contradicting each other, because no reviewer had held both at once.

**So: do not re-audit commit-by-commit correctness.** The prior rounds did that competently. Spend
your budget on what a range-scoped reviewer COULD NOT SEE:

- a guarantee established early and quietly broken later
- two features that are each correct alone and wrong together
- a doc/message/advisory that describes behaviour a later commit changed
- one filer walked end-to-end from `main` to `HEAD` — does their outcome move in a defensible
  direction at every step?

## ★ THE HIGHEST-RISK SURFACE: r3's own fixes are UNREVIEWED

Commit `5ab1258` (and `d6ff290` before it) is the newest code on the branch and no reviewer has seen
it. It was produced by the round that found four Important defects in the code it replaced. It:

- **moved** the §G-21 donation-restriction refusal from `screen_compute_dependent` to
  `screen_absolute`, and **re-keyed** it from a ledger aggregate to `ar.deduction_is_itemized`
- **added two parameters** (`state`, `year`) to `screen_absolute`, touching every call site
- **split one predicate into two** — `spouse_63f_boxes_count` (record AND status, decides the
  DEDUCTION) vs `spouse_63f_status_permits` (status only, drives the ADVISORIES)
- **deleted** a `CarryProvenance::Computed` stamp
- added a `section == Form8283Section::B` guard to the 8283 5a/5b/5c writer

Every one of those is a place a fix can be wrong in a new way. Check them hardest.

## Settled facts — do NOT re-derive or re-file these

1. **§G-19a** (§1411 all-in marginal-rate display) — OPEN OWNER DECISION, deliberately unbuilt.
2. **§G-12** (no Form 8275-R) — blocked on an asset unobtainable in this environment.
3. **§G-22** — knowingly partial: only the QBI loss carryforward is asked; other carryforward
   families remain import-only. FILED, not forgotten.
4. **§G-20b** — the advisory list has two unconditional members, with a stated gate for a third.
5. **§G-23** — `CarryProvenance` cannot express "the filer stated ZERO". Filed; direction is safe.
6. **`.pii-patterns` is absent; push and publish are BLOCKED.** Out of scope entirely.
7. **Neither oracle (OpenTaxSolver, Tax-Calculator) can validate a value they are HANDED.** Form 8995
   line 3 is such a value. "This is unvalidated by the oracles" is the known standing condition, not
   a finding. A finding would be that the value is used WRONGLY.
8. All five gates pass at HEAD: `make check` (2536 tests), `cargo fmt --all --check`,
   `cargo +1.88 check --workspace --locked`, `xtask check-isolation`, `scripts/pii-scan-generic.sh`.
   TY2024 golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47`, unchanged across the branch.

## The project's own rules ARE the authority

Read `/scratch/code/bitcoin_tax/CLAUDE.md`. It is short and it is the standard:
transcribe-don't-paraphrase; blank-is-the-normal-case (assert PROVENANCE, never non-blankness); an
entry is TESTIMONY (a `0` on an unasked line fabricates it); the two-oracle limit. Extracted form
text is in `design/forms/extract/`. **Quote the form or the instruction text** when you claim btctax
contradicts it. The IRS PDF is the authority; an oracle is a witness.

## Output format — follow exactly

First line: `VERDICT: <merge | fix-before-merge: X>`

Then zero or more findings, most severe first, each in a fenced block:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence — the defect.
FAILURE: concrete inputs/state → the wrong figure on a signed return, or the wrongly-blocked filer.
EVIDENCE: quote the code AND the authority (form text, instruction text, or the in-repo rule).
```

**Critical** = wrong tax figure, data loss, or an unmet guarantee. **Important** = real defect,
missing case, unsound assumption. **Minor/Nit** = recorded, does not gate. Do not inflate: a clean
result is the expected outcome and "0 Critical / 0 Important" is a successful review.

End with:

`ALSO CHECKED, SOUND:` — specifics, so the next round knows what not to redo.

`WHAT WOULD MAKE THIS REVIEW WRONG:` — the assumption you did not verify by execution. Be honest.
The prior round's version of this section is what let its Important be confirmed rather than trusted.
