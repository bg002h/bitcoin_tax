# Review brief — r8, `95f7f34..HEAD` on `main`

Five commits, ~650 lines across 23 files. Read the range: `git diff 95f7f34..HEAD -- crates`.

| commit | what | moves ink? |
|---|---|---|
| `03527f7` | §6413(c) EIN canonicalization + a new advisory | **YES** — Schedule 3 line 11 |
| `fa0559b` | R1 — the CTC advisory answers §24(b) | no (advisory text only) |
| `35ebf4b` | B11 — an always-live scope-attestation **REFUSAL** | no, but it can BLOCK every return |
| `cb29fc1` | Schedule D 18/19 left blank instead of `0` | **YES** — a value leaves the page |
| `d7ded00` | `_` forbidden on `Option<Usd>` (a test) | no |

## THE ONE QUESTION

**Does any of this put a wrong mark on a filed return, remove a required one, or refuse a return that
should have filed?**

## ★ Why this review exists, and where to spend the budget

**Every one of these five commits is a FOLD of a prior review's finding.** That is the shape that has
failed twice in this session, both times with a Critical:

- r7 found the r6 fold's guard had **no kill test at all** — deleting it reddened nothing.
- A Fable pass found the §6413(c) fix compared EINs as **trimmed free text**, so one employer spelled
  `11-1111111` and `111111111` counted as two and *restored the exact understatement the fix existed to
  kill*.

So: **do not re-audit the underlying features. Audit the folds.** Assume the findings they answer were
real and correctly diagnosed; ask whether the answers are right.

## Where the risk actually is

### 1 · Schedule D 18/19 — a value LEAVES a filed return (`cb29fc1`)

`schedule_d_full.rs` pushed `Usd::ZERO` into lines 18 and 19 on every `BothGains` return. Both are
conditional entries — *"**If you are required to complete** the 28% Rate Gain Worksheet …, enter the
amount, **if any**"* — and btctax completes neither worksheet (it refuses §1202 / collectibles /
unrecaptured §1250 via `UnrecapturedOrSpecialRateGain`). They are now **blank**.

- **Is the refusal that justifies this actually total?** The argument is "btctax never has 28%-rate or
  unrecaptured-§1250 gain, so the condition is never met." Find a path where it *can* — 1099-DIV box 2b
  / 2c / 2d, a ledger disposal, a carryover, anything. If one exists, the blank is wrong and the old
  zero was accidentally right.
- **Line 20** reads *"Are lines 18 and 19 both zero **or blank** and you are not filing Form 4952?"* —
  the test asserts L20 is still Yes. Verify that is what the emitted PDF does, and that nothing
  downstream (`transcribe::extract_lines`, `no_unmapped_filled`, the oracle harness, the QDCGT routing)
  reads those cells and now sees an absent key where it saw `0`.
- The **ordinal/descent** bookkeeping: `push_p2` incremented `p2_ord`; the new `push_money_opt` calls
  pass `None` for descent. Does skipping two cells leave the descent group sound?
- `push_money_opt` itself (`cells.rs`) — is it correct, and is `None` genuinely indistinguishable on
  paper from "never written"?

### 2 · §6413(c) — the fold of a Critical (`03527f7`)

`canonical_ein` strips hyphens/whitespace and requires exactly nine digits, applied at the screen and
the compute site. A malformed EIN routes to the refusal.

- **Is nine-digits-after-stripping the right canonical form?** What about a leading `+`, unicode
  dashes, or an EIN typed with a space instead of a hyphen? Which of those SHOULD be one employer?
- **`excess_ss_not_creditable` is a new `AbsoluteReturn` field** feeding a new advisory. Is it computed
  on the same footing as the credit — per person, never pooled, and zero when identity is unknown?
- The advisory fires when `eins.len() == 1`. What about **zero** known EINs with an amount over the
  cap? (The claim is the screen refuses first. Verify no path reaches the advisory without the screen.)
- Does the **MFS** case work? The cap is per person.

### 3 · B11 — an always-live refusal (`35ebf4b`)

`other_out_of_scope_income: Option<bool>` — `None` refuses, `Some(true)` refuses, `Some(false)` files.

- **What does it strand?** I verified all three states, plus that a crypto-only year (no `income
  import`) is unaffected. **Not verified: the TUI, the defensive-filing wizard, `what-if`, and any
  stored return authored before this field existed.** An existing vault's TOML has no such key ⇒
  `None` ⇒ refuses. Is that the right migration behaviour, and is the message good enough to act on?
- The question is `live: |_| true`. Is there any filing shape where asking it is *wrong* rather than
  merely redundant?
- 31 tests went red when it landed and were fixed by answering it in fixtures. **Did any of those
  fixtures exist to test something that the answer now masks?**

### 4 · R1 — the CTC phase-out (`fa0559b`)

Transcribed from Schedule 8812 Part I (archived: `design/forms/extract/f1040s8--2024.txt`). btctax
cannot compute lines 4/6 (it does not know dependents' ages), so it computes the **ceiling** of line 8
as `dependents × $2,000` and compares to line 11.

- **Is the ceiling argument sound?** It assumes $500 (ODC) ≤ $2,000 (CTC) so an all-CTC composition
  maximises line 8. Check for a composition that beats it.
- Line 10 rounds the excess **UP** to the next $1,000. Verify against the form's own words.
- Line 3 is modified AGI. Are all four line-2 add-backs the right ones, and are they the right sign?
- **The false-negative direction is the dangerous one**: telling a filer "you get nothing" when they
  are owed money. Find an input where `provably_zero` is true but the credit is not zero.

### 5 · `_` on `Option<Usd>` (`d7ded00`) — cheapest, check last

A test reading both sources. The real sources have zero `Option<Usd>` fields, so it is driven by
planted cases. Can the parser be fooled — a multi-line field declaration, a type alias, `Option<Usd>`
inside a nested struct, a doc comment containing the text?

## Settled — do NOT re-derive

- The findings these commits answer are real: the $3,894 understatement, the CTC misdirection at AGI
  $2,085,000, Pub 559's unaskable rental income, the sworn zeros. All are evidenced in
  `design/direction/FILING-TRIAL-2026-08-02.md`.
- The per-employer-cap construction was already adjudicated correct against Pub 505 Worksheet 3-1.
- Golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47` is unchanged across the whole range; 2559 tests
  pass; all five gates green.
- B1 (Form 8995-A) and B2 (>4 dependents) are NOT in this range and are not in scope.

## Output

`VERDICT: clean` or `VERDICT: <n> Critical / <n> Important`, then per finding:

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: path:line
CLAIM: one sentence.
FAILURE: concrete inputs → the wrong mark on a filed return, or the return wrongly refused.
EVIDENCE: quote the code AND the form text or rule it violates.
```

**Critical** = a wrong or missing mark on a filed return, or a return refused that should file.
**Important** = a real defect, or a guard that cannot catch what it claims to. Do not inflate — a short
clean report is a fine outcome, and three of the five commits are inert on paper.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY on tracked files. You may mutate temporarily to verify a kill **if** you back
up with `cp` to `/tmp` and restore with `cp` — never `git checkout --`. Leave the tree clean and say so.
No commits. No subagents.
