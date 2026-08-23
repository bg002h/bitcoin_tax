# BRIEF — the PRE-MERGE pass, scoped `main..HEAD`, pointed at THE SEAM

_Harness **B3**: a per-range review is not a branch review, and the LAST review before an
irreversible action is scoped to the whole branch and pointed at **interaction**. Merging to `main`
is that action. This brief exists so the pass spends its budget where no earlier reviewer could
have looked._

---

## THE ONE QUESTION

**Does anything on this branch interact badly across the seam that no earlier review could see?**

Not per-commit correctness. Not a fresh audit. **Interaction.**

## ★★★ THE SEAM — this is the whole point of the brief

Six independent reviews ran on this branch. Every one was **range-scoped**, and their windows do not
add up to the branch:

| review | range | verdict |
|---|---|---|
| phase 1 | `3fc88497..HEAD` (whole branch *at the time*) | folded to green |
| phase 2 | `8f0f982a..HEAD` | folded to green |
| phase 4 | `6cd9e073..HEAD` | folded to green |
| **final** | **first 36 commits** | folded to green |
| widening | `99628341..HEAD` (26 commits) | 2 Important + 1 Minor, folded |
| fold | `02939632..HEAD` | **sound, 0C/0I** |

★★★ **`99628341` is the seam.** The "final" whole-branch review saw the **36 commits before it**.
Everything after — 26 commits, including BOTH owner-authorised widenings, nine review-response folds,
and the fold that answered the widening review — landed **after that window closed**. The widening
review says so itself: *"The persisted whole-branch review covered only the first 34 commits;
everything here landed after it."*

**No reviewer has ever held both sides at once.** That is verbatim the B3 failure mode this repo
already paid for: three reviews, two ranges, 0C/0I each, and the Important lived in the earliest
commit — outside every window — while the fix for it already existed nine commits later in the same
branch, uncarried, because nobody held both.

**34 files were edited on BOTH sides of `99628341`.** That is the search space. The highest-value
ones, by how much behaviour crosses them:

```
crates/btctax-core/src/tax/return_1040.rs      crates/btctax-cli/src/cmd/tax.rs
crates/btctax-core/src/tax/return_refuse.rs    crates/btctax-cli/src/main.rs
crates/btctax-core/src/tax/questions.rs        crates/btctax-cli/src/render.rs
crates/btctax-core/src/tax/advisories.rs       crates/btctax-cli/LIMITATIONS.md
crates/btctax-core/src/tax/classifier.rs       crates/btctax-core/src/tax/capital_loss_carryover.rs
crates/btctax-core/src/tax/return_inputs.rs    crates/btctax-core/src/tax/scrub.rs
```

Ask of each: **did the later window change an assumption the earlier window's code still relies on?**
A predicate widened on one side and consumed on the other. A refusal deleted on one side whose caller
on the other still reasons about it. A field whose provenance semantics moved under an older reader.

## WHAT THE BRANCH IS

`btctax` emits a complete US federal 1040 a human signs under **26 USC §6065 penalties of perjury**.
A wrong number is the worst outcome; an **understatement** is worse than an overstatement. 62 commits,
89 files, +14,727/−958.

Two owner-authorised **widenings** are the risky content, and both make btctax do *more* rather than
refuse:

- **(A)** a taxable-income≤0 year carrying a capital-loss carryforward-IN now **FILES** (a refusal
  variant was deleted). A population that used to be turned away now receives a return.
- **(B)** `--write-carryover` now **ROLLS** the §1212(b) capital-loss carryover into next year's
  inputs, stamped `Computed` — btctax became an **author** of a figure it previously only read.

## ALREADY SETTLED — do not re-derive, do not re-litigate

- **The widenings themselves are the owner's decision.** Do not argue (A) or (B) should not exist.
  Their *mechanics* are in scope; their *existence* is not.
- **The `grounded` predicate's three disjuncts** and the **whole-dollar rounding** of the persisted
  carryover: settled, reviewed twice.
- **B-1 and B-2** from the widening review were each reproduced as printed observations before being
  fixed, and the fold answering them was independently reviewed `sound`.
- **FR-17** (a `Computed` stamp has no retraction path) is a **filed, owner-visible acceptance** with
  owning phase = TY2025 full-return support. Do not re-file it. Only report it if you can show it is
  reachable and harmful in **v1 today**.
- **FR-18/FR-19/FR-20/FR-21** are filed non-gating Minors. Do not re-file them.
- **Neither oracle witnesses a carryover level** — one takes it as an input, the other emits none
  (§G-9). **Never propose an oracle check on it.** This is a permanent limit, not a gap.
- **Phase 5 (EITC/ACTC) is DEFERRED**, scoped as FR-16. Not a finding.

## ALREADY MACHINE-VERIFIED — spend NO budget here

The **full CI surface** is green on `37004b99`, not just the fast local gate:

```
clippy (1.98, --locked, --all-targets, -D warnings)   ✓
test — ubuntu / macos / windows                       ✓   2766 passed, 12 skipped
msrv 1.88 (cargo check --workspace --locked)          ✓
fmt --all -- --check                                  ✓
pii-scan                                              ✓
net-isolation (no HTTP client in the tax crates)      ✓
examples drift gate (regenerated == committed)        ✓
```

`main..HEAD` is a clean fast-forward; `main` has not moved (`3fc88497`, local == remote).

Reporting a compile error, a lint, a formatting nit, or "add a test" for something already covered is
a **wasted finding**.

## WHERE THE JUDGMENT IS — what tools cannot reach

1. **The deleted refusal (A).** A refusal variant was removed so a population could file. Does any
   *surviving* code still assume that refusal fires — a comment, a guard, a test premise, a downstream
   consumer that was correct only because that gate existed? The repo's own standing lesson: *a
   refusal a compute path was built to rely on is LOAD-BEARING; before relaxing one, grep for the code
   whose correctness comment names it.* Was that done for **every** caller, across the seam?
2. **btctax as author (B).** `Computed` now means "btctax derived this". Three surfaces act on it
   (`m4_authority` goes silent, `BenefitCarryoversNotStated` stops asking, the `--force` guard stops
   protecting). Is there any path where a figure gets that signature without a derivation behind it,
   or loses it while a reader still assumes it?
3. **The newly-admitted household.** (A) admits a low-income, TI≤0 filer. Are the credits, advisories
   and printed lines correct **for that population specifically**? It is constructed to be exactly the
   filer the branch never had before.
4. **Blank vs zero.** House rule: every entry is testimony; a printed `0` on an unasked line
   fabricates testimony under someone else's signature. Did anything across the seam start printing a
   figure where silence was lawful?

## FORBIDDEN

Re-auditing the tax engine wholesale. Re-litigating the widenings' existence. Proposing an oracle
check on a carryover. Re-filing FR-17..FR-21. Style, naming, and "consider also…" softening — if
something is wrong, say it plainly.

## OUTPUT

**Write your report to `reviews/filing-readiness-premerge-review.md` as your FINAL action**, then
return only a short summary plus that path. If the harness denies the write, do **not** retry and do
**not** summarise — return the report **verbatim** so it can be persisted byte-exact, and say which
happened.

```
VERDICT: <merge | merge-after-fixing-X | do-not-merge>

## SCOPE
What you read and executed.

## SEAM FINDINGS
The ones that required holding both sides of `99628341` at once. Per finding: severity, file:line,
what is ASSERTED, what is ACTUALLY TRUE, the concrete failing case, the smallest fix.

## OTHER FINDINGS
By severity. "None." is a fine answer.

## WHAT WOULD MAKE THIS REVIEW WRONG
One sentence naming the assumption the verdict depends on.
```

★ **The measure of this pass is not that it happened.** It is whether it found something the six
earlier rounds **structurally could not have seen**. If the seam is clean, say so plainly — a clean
result closes the loop and the branch merges.
