# Review brief — r3, branch `feat/no-pen-deferrals`

**Range under review: `d0aad6f..HEAD` (9 commits).** Read it yourself:
`git log --oneline d0aad6f..HEAD` and `git diff d0aad6f..HEAD`. Do not ask for it to be pasted.

## The ONE question

**Can any change in this range cause a filer to sign a return that UNDERSTATES tax — or to be
blocked from filing a return that is actually correct?** Everything else is secondary.

This is a US federal 1040 signed under penalty of perjury (26 USC §6065). Understatement is the
worst outcome. A *false refusal* (btctax blocks a filer whose return was fine) is the second-worst
and is a live risk in this range, because it adds a new refusal.

## Scope discipline — read this before you start

- **Do NOT re-audit the whole branch, the whole repo, or anything outside the range.** Three prior
  rounds covered `main..d0aad6f`. Their findings are folded and closed.
- **Do NOT re-derive the settled facts listed below.** They are decided. Relitigating them is the
  single most likely way to waste this review.
- **Do NOT file style, naming, comment-density, or doc-prose findings** unless the prose states
  something FALSE about behavior.
- If you find nothing blocking, **say so plainly**. "0 Critical / 0 Important" is a successful
  outcome and is the expected one. Do not manufacture severity to justify the round.

## What the earlier rounds covered (so you spend your budget on the SEAMS)

| round | range | lenses | result |
|---|---|---|---|
| r1 | `7bde148..65270db` | Opus (tax) + Sonnet (instrument) | 0C/0I, 2 Minors fixed, 4 filed |
| r2 | `afa0ffe..HEAD-at-the-time` | Opus (tax) + Sonnet (instrument) | 0C/0I, 1 Nit fixed |
| pre-publish | `main..d0aad6f`, whole branch, INTERACTION-scoped | Fable | **1 Important**, 1 Minor, 1 Nit — all folded |

★★★ **The pre-publish round's Important lived in the branch's EARLIEST commit, outside both earlier
review windows.** That produced harness rule **B3** (`design/HARNESS.md`): *a per-range review is not
a branch review; a stack of them does not add up to one.* So while your RANGE is the 9 new commits,
you are explicitly asked to check how they INTERACT with what came before — named seams below.

## Settled facts — do NOT re-derive these

1. **`§G-19a`** (whether to display the §1411 all-in marginal rate) is an OPEN OWNER DECISION,
   deliberately not built. Not a finding.
2. **`§G-12`** (no Form 8275-R) is blocked on an asset that cannot be obtained in this environment.
   Deliberately not built. Not a finding.
3. **`§G-22`** is knowingly partial: only the QBI loss carryforward is asked. Other carryforward
   families remain import-only, and that is FILED, not forgotten.
4. **`§G-20b`** — the advisory list now has two unconditional members. Known, filed, with a stated
   gate ("a third means the surface is the problem"). Not a finding.
5. `.pii-patterns` is absent and push/publish are BLOCKED on it. Out of scope.
6. The TY2024 golden matrix md5 is `c4e1853ed82d113ca5cd97ffd8abbf47` and is unchanged by this
   range. All five gates pass at HEAD: `make check`, `cargo fmt --all --check`,
   `cargo +1.88 check --workspace --locked`, `xtask check-isolation`, `scripts/pii-scan-generic.sh`.
7. **Neither tax oracle (OpenTaxSolver, Tax-Calculator) can validate a carryforward INPUT.** A value
   the oracles are HANDED is never validated by their agreement. Form 8995 line 3 is such a value.
   Saying "this is unvalidated by the oracles" is not a finding — it is the known standing condition.
   A finding would be that the value is used WRONGLY.

## The named seams — where the risk actually is

**S1 — `fd9c15f`, the MFS spouse's §63(f) aged/blind boxes. THE HIGHEST-RISK CHANGE ON THE BRANCH.**
It is the *only* change where an answer can **only reduce tax**. i1040gi permits the spouse's boxes
on MFS "if your spouse had no income, isn't filing a return, and can't be claimed as a dependent on
another person's return". `HouseholdHeader` gained `spouse_had_no_income` and
`spouse_not_filing_a_return`; the third condition already existed. The design intent is that the
gate **FAILS CLOSED**: all three must be affirmatively answered in the claiming direction, and ANY
unanswered or adverse one forgoes. **Verify that intent actually holds in the code, on every path.**
Specifically: is there any input state — including a hand-edited `income import` TOML, a stale answer
under changed liveness, or a filing-status change after answering — that grants a box the filer is
not entitled to? Is the MFJ path unchanged?

**S2 — `d6ff290`, the §G-21 donation-restriction refusal. A NEW WAY TO BLOCK A RETURN.**
A return-level universal ("did any donation have strings attached?") gates Form 8283 Section B lines
5a/5b/5c. Unanswered refuses; `Some(true)` refuses; `Some(false)` files. The question is a
*skippable* offered unconditionally, but the binding gate is keyed on
`year_donation_deduction(state, year) > $5,000`. **Two questions:** (a) can a filer who is entitled
to file be permanently blocked — i.e. is the refusal always ESCAPABLE by answering, and is the
escape route the message names actually the one that works? (b) Is the $5,000 scope right — does any
year that prints a Section B page escape the gate, or any year that does not print one get caught?

**S3 — interaction between `fd9c15f`/`2dc8b07` and the pre-existing aged/blind machinery.**
`questions::spouse_63f_boxes_count` was made ONE shared predicate, consumed both by
`AgedBlindBoxes::for_return` (which decides the deduction) and by the liveness of
`SpouseDiedDuringYear`/`DodSpouse` (which decide whether questions are asked). A shared predicate
with two consumers is exactly the shape that produces "the question is asked but the answer is
discarded" — which is the bug §G-20 existed to fix. **Did fixing it create the mirror defect: an
answer COUNTED that was never asked?**

**S4 — interaction across the whole branch on the ADVISORY surface.** Six advisories were added
across several commits days apart. Do any two double-count, contradict, or fire on a return where
the thing they describe did not happen? Does any advisory claim a box was forgone when it was in
fact claimed (or vice versa) after `fd9c15f` changed when boxes are claimable?

**S5 — `64df404`, Form 8995 line 3.** This fixed a hole that is LIVE IN PUBLISHED v0.14.0 (the
absence INFLATED the deduction and UNDERSTATED tax). Confirm the fix's direction is right and the
write-back cannot produce a negative or double-count a carryforward across years.

## Output format — follow exactly

Start with one line: `VERDICT: <clean | fix-before-merge: X>`

Then zero or more findings, each in its own fenced block, most severe first:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence — the defect.
FAILURE: concrete inputs/state → the wrong output on a signed return, or the wrongly-blocked filer.
EVIDENCE: quote the code and the authority (form text, instruction text, or the in-repo rule).
```

Severity means: **Critical** = wrong tax figure, data loss, or an unmet guarantee.
**Important** = real defect, missing case, or unsound assumption. **Minor/Nit** = recorded, does not
gate. Do not inflate.

End with two labelled sections:

`ALSO CHECKED, SOUND:` — what you verified that was fine. This is how the next round knows what not
to redo. Be specific about seams S1–S5.

`WHAT WOULD MAKE THIS REVIEW WRONG:` — the assumption you did not verify by execution. Be honest;
the prior round's version of this section is what let its Important be confirmed rather than trusted.
