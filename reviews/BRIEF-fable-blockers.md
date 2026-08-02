# Fable brief — strategy for every filing blocker, and a review of the code written for them

**Repo:** `/scratch/code/bitcoin_tax`, branch `main`. Two jobs, in this order. Both matter; the second
is the one that can catch a wrong number on a filed return, so do not let the first crowd it out.

---

## Background you need, in one paragraph

btctax emits a complete US federal Form 1040 packet signed under 26 USC §6065. It began as a Bitcoin
tax ledger and grew a full return. On 2026-08-02 a **filing trial** drove the shipped binary end to end
against the owner's own scenario and three published gold standards, and produced
`design/direction/FILING-TRIAL-2026-08-02.md` — **read it first; it is the source of every blocker
below.** Two blockers were fixed during the trial and are part of what you are reviewing.

Doctrine is in `CLAUDE.md` and is binding. The parts that bear hardest here: **transcribe forms, never
paraphrase** (one field per numbered line, in the form's own numbering, instruction text as the doc
comment; a derived/closed form needs a written equivalence proof plus a KAT); **blank is the normal
case** (assert PROVENANCE, never non-blankness); **an entry is testimony** (a printed 0 the filer never
gave is fabricated); **tests for conformance, reviews for judgment**; and harness rule **B1** — no
checker exists until it has been observed RED on a planted defect.

---

## JOB 1 — Strategy for the blockers

The trial found eleven. Three are closed, two were retracted after testing, six are open:

| id | status | one line |
|---|---|---|
| **B1** | OPEN | `QbiAboveThreshold` refuses any Schedule C above the §199A threshold. Form 8995-A is 40 lines and btctax has at most ONE Schedule C, so only column A is ever used. **The refusal is a missing INPUT** — `QbiInputs` has no W-2-wages and no UBIA field, which are 8995-A lines 4 and 7. |
| **B2** | OPEN | >4 dependents overflow the 1040 grid; the emitter refuses. Form 1040 HAS a *"If more than four dependents … check here"* box + continuation statement. `report` computes 9 dependents happily — only the emitter refuses. |
| **B3** | OPEN | `ScheduleCInputs` has no gross-receipts field. Self-employment income can only arrive as mined Bitcoin. |
| **B4** | OPEN | No 1099-B input. Capital gains can only arrive as BTC disposals. |
| **B5** | OPEN | Income cannot be stated in dollars — you must solve for a satoshi quantity against the bundled FMV. |
| **B7** | OPEN | TY2025 unsupported (this is the existing TY2025 program, spec+plan green, T1 built — a project, not a fix). |
| **B11** | OPEN | Out-of-scope INCOME (Schedule E/F, K-1) is documented but **never asked**, and nothing refuses. A filer with rental income files a clean packet omitting it. |
| B6, B8 | RETRACTED | I filed them from reading, not running; both were wrong. |
| B9, B10 | FIXED | see Job 2. |
| **R1** | OPEN | The CTC advisory never applies the §24(b) phase-out, so it tells a $2M-AGI filer to claim a credit that is provably $0. Changes no number. |
| **R2** | FIXED | An excess-SS credit granted without the "more than one employer" test — a **$3,894 understatement**. See Job 2. |

A prior workflow already produced an ordered build plan; its headline judgments were: do shared
instrument work once (an `Option<Usd>` writer that omits rather than printing `0`; grammar/ratchet
changes; a money-leaf `_` ban); do **B2 before B1** because B2 builds the continuation-statement asset
three later items need; do **B1 before B3** because B3 pushes *more* returns into `QbiAboveThreshold`;
and it argued **against** B5 on the grounds that a filer-stated USD amount is testimony about value and
would corrupt the FMV-from-dataset integrity story.

**What I want from you — judgment, not a restatement:**

1. **Is that ordering right?** Argue it or replace it. In particular: is B2-before-B1 real, or an
   artifact of one agent's framing?
2. **B11 is the one I think is underweighted.** It is not a missing feature — it is the answered-ness
   invariant at the SCOPE boundary, and its failure direction is an understatement. Is the right answer
   a scope questionnaire, a refusal, an advisory, or something else? What is the *minimal* mechanism
   that makes "nobody asked" distinguishable from "the filer has none", given the product cannot ask
   about every out-of-scope item without becoming a questionnaire nobody finishes?
3. **What should NOT be built?** Be specific. B5 has one argument against it already; are there others?
4. **The product identity question, which is genuinely open.** B3 and B4 together turn btctax from a
   Bitcoin tax tool with a 1040 attached into a general 1040 preparer. Is that the right direction, or
   should the scope boundary be made *loud and honest* (B11) instead of *wide*? The owner is filer #1
   and wants to compare TY2025 output to their own real return.
5. **Sequencing against reality:** TY2024 is the only supported year, TY2025 tables are not out, and the
   owner's first real filing would be TY2026 or TY2027.

---

## JOB 2 — Review the code written for B9, B10 and R2

**This is where a wrong answer costs a wrong return.** Scope: `git diff c8c3704~1..HEAD -- crates`.
Four commits: `c8c3704` (B9), `18c9980` (R2 + B10), `95f7f34` (pii allowlist), plus the trial docs.

### The one question

**Does the new §6413(c) logic put a correct number on Schedule 3 line 11 in every case, and does the
B9 guard ever refuse an export it should have written?**

### R2 / B10 — what changed and what to attack

i1040gi, Schedule 3 line 11, states two conditions:

> *"If you, or your spouse if filing a joint return, had **more than one employer** for 2024 and total
> wages of more than $168,600, too much social security or tier 1 railroad retirement (RRTA) tax may
> have been withheld. You can take a credit on this line for the amount withheld in excess of
> $10,453.20. But if **any one employer** withheld more than $10,453.20, you can't claim the excess on
> your return. The employer should adjust the tax for you."*

btctax enforced only the second and justified the omission with a comment that was false — *"a
single-employer person nets 0, so the 'requires ≥ 2 employers' rule falls out naturally"*. It does not:
one employer may issue several W-2s to one person. A probe credited **$3,894** to a filer entitled to
$0, turning an $1,085 liability into a $2,809 refund.

The fix (`crates/btctax-core/src/tax/return_1040.rs::excess_social_security`,
`return_refuse.rs`, `return_inputs.rs::W2`, `btctax-input-form`):

- `W2.ein: Option<String>`, collected in the TUI as `FieldId::W2Ein`.
- Credit requires **≥ 2 distinct EINs** for that person.
- Each **employer** contributes at most the cap before the aggregate is compared to it.
- An EIN is demanded **only when it decides something** — at or under the cap, no question is asked.
- Over the cap with any EIN missing → new refusal `ExcessSsEmployerUnknown`.
- The old `SingleEmployerExcessSs` refusal is **no longer raised** (B10): the instruction says *"you
  can't claim the excess"*, not *"you can't file"*.

**Attack these specifically:**

- **Is "each employer contributes at most the cap" the right reading of the second sentence?** I chose
  it as the conservative construction. Check it against i1040gi and Pub 505. If the correct reading is
  "no credit at all when any one employer is over", say so — that is a different number.
- **Tier 1 RRTA.** The instruction covers *"social security **or tier 1 railroad retirement (RRTA)**
  tax"*. Does btctax model RRTA at all? If not, is silence here a gap of the same class as B11?
- **The `None` fallback inside `excess_social_security` returns `Usd::ZERO`** on the theory that the
  screen already refused. Is that reachable by any path that bypasses the screen — the crypto slice, the
  TUI, `derive_tax_profile`, the what-if engine?
- **Whitespace/format collisions**: `"12-3456789"` vs `"123456789"` vs `" 12-3456789 "` are three
  distinct EINs to a `BTreeSet<&str>` after `trim()`. Two spellings of one employer would restore the
  understatement. Should the EIN be canonicalised, and is there an existing canonicaliser (`Ssn::canonical`) to model on?
- **MFS.** The cap is per person; does anything here assume MFJ?
- **Does the new refusal strand anyone?** A filer over the cap who genuinely cannot obtain an EIN now
  cannot file at all. Is that the right trade, given B10's whole point was that refusing was wrong?
- **The tests.** `excess_social_security_per_person_not_pooled` and
  `excess_ss_refuses_only_when_employer_identity_is_unknown`. The OLD test asserted *"two employers each
  $6,000"* on fixtures carrying **no employer identity at all** — it believed a fact it never stated,
  which is how the understatement shipped. Do the new tests actually red on the defects they name?
  Which mutation kills each?

### B9 — the smaller one

`--forms full-return` became a `FormArg` variant. `wants()` is `selected.is_empty() ||
selected.contains(f)`, so on a **crypto-only** year the new value matches no slice form and would have
written an **empty export directory with exit 0**. A guard refuses instead. Check: is the guard
correctly placed relative to the full-return dispatch, does it fire on a mixed `--forms
full-return,f8949`, and does `forms_ignored_full_return` now report honestly?

---

## Settled — do NOT re-derive

- The trial's arithmetic is verified: the owner's scenario and the three gold standards are checked line
  by line in the trial document, including two **exact matches** (TaxCalcBench MFJ excess-SS; Pub 560
  Schedule SE, where line 12 prints 26,262 — the sum of the *rounded* parts — not 26,263).
- B6 and B8 are retracted with evidence. Do not re-file them.
- Golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47` is unchanged across every commit in scope; 2553
  tests pass; all five gates green.
- `.pii-patterns` is owner-authored and off limits. Push and publish are blocked. Do not propose them.

---

## Output

Two clearly separated sections.

**`## STRATEGY`** — the ordering, with reasons; what not to build; your answer on B11 and on the product
identity question. Prose, tight, opinionated. A recommendation, not a survey.

**`## REVIEW`** — `VERDICT: clean` or `VERDICT: <n> Critical / <n> Important`, then:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: concrete inputs → the wrong figure on a filed return.
EVIDENCE: quote the code AND the instruction text or rule it violates.
```

**Critical** = a wrong or missing number on a filed return. **Important** = a real defect, or a guard
that cannot catch what it claims to. A short clean report is a fine outcome for ~200 lines.

End with `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY on tracked files. You may mutate temporarily to verify a kill **if** you back
up with `cp` to `/tmp` and restore with `cp` — never `git checkout --`. Leave the tree clean and say so.
No commits. No subagents.
