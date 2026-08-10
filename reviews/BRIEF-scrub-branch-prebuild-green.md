# BRIEF — whole-branch review of `feat/income-scrub` (`main..HEAD`)

Harness **B3**: the last review before an irreversible action is scoped to the WHOLE branch and
pointed at **interaction**, not correctness-per-commit. A stack of range-scoped reviews does not add
up to a branch review.

```
git diff main..HEAD          # 24 commits, 50 files, ~6.9k insertions
git log --oneline main..HEAD
```

## The ONE question

**Is `btctax income scrub` safe to ship?** Concretely: can it tell a filer "this file is safe to hand
to a stranger" when it is not — or emit a file that computes or prints a different return than the
original?

## ★★★ WHAT EVERY EARLIER PASS STRUCTURALLY COULD NOT SEE

Five passes ran on this branch. **None of them read the build.**

| pass | what it read | what it could not |
|---|---|---|
| `reviews/scrub-r1-workflow.md` | the ORIGINAL held implementation (`31d5c79`, `2449ee4`) — 19 confirmed + 6 sweep, 2 CRITICALs | anything written since |
| `reviews/scrub-spec-r{1,2,3,4}-review.md` + the r5 fold-diff check | the SPEC, at five successive drafts | **all code** |

So the build — commits `77e50f9b` through `344c1e11`, SPEC §8 steps 2–8 — **has never been reviewed by
anyone**. That is this pass's centre of gravity.

### The two seams, in priority order

1. **★★★ DID THE BUILD CLOSE r1's ACTUAL FINDINGS — or only the ones the spec happened to name?**
   This is the interaction nobody has checked, and it crosses a document boundary: r1 found defects in
   CODE, the spec folded them into PROSE sections, and the build implemented the PROSE. A finding that
   fell out between r1 and the spec is invisible to every pass so far — the spec reviews took the spec
   as the subject, and r1 predates it. **Read `reviews/scrub-r1-workflow.md`'s findings directly
   against the current code**, not against the spec's description of them.

2. **Interaction across the build steps.** Each step was gated and mutation-verified in isolation.
   Ask what only shows when they run together, e.g.: the ledger refusal (step 2) now runs before the
   draft read (step 6) — is the ORDER right, and does either see state the other needs? The marker
   (step 7) rides on the emitted string that the round trip (step 8) parses back — can a filer's own
   edits break that? `scrub_ip_pin` (step 4) and the disclosure test (step 5) both assert on the IP
   PIN — do they agree? Does the derived axis (step 4) still reach every field after step 6 changed
   what scrub replaces?

## Read these — by path

| path | what |
|---|---|
| `design/SPEC_income_scrub.md` | the spec (r6) the build followed |
| `reviews/scrub-r1-workflow.md` | **the code review of the ORIGINAL — seam 1's authority** |
| `crates/btctax-core/src/tax/scrub.rs` | the scrubber |
| `crates/btctax-core/src/tax/scrub_axis.rs` | the derived axis + §3.3's matrix |
| `crates/btctax-cli/src/cmd/tax.rs` | the refusal, the marker, the import guard |
| `crates/btctax-cli/tests/scrub_refusal.rs` | the CLI tests |

## ★ SETTLED across four spec rounds — do not re-derive

`absent → absent`; the ledger is never scrubbed; no perturbing figures; the refusal is a `CliError`
not a `RefuseReason`; the `year-1` disjunct is deliberate over-refusal that secures no live read (two
attempts to name a mechanism for it were both wrong — do not propose a third); committing a scrubbed
return as a fixture is out of scope for v1; the IP PIN is never synthesised when valid.

## ★★ ALREADY MACHINE-CHECKED — do not re-verify

- **2665 tests pass**; `cargo fmt --all --check` and `clippy -D warnings` clean at HEAD.
- **`frozen_engine_files_are_unchanged` passes** — `tax/compute.rs` was reverted byte-exact after the
  build found the spec had mandated an edit to a frozen file.
- **Every §8 step was mutation-verified.** 6 mutations on step 2's predicate, 1 on step 3, 6 on step 4,
  3 on step 5, 1 on step 6, 3 on step 7 — all killed. Do not re-run mutations; **do ask what a
  mutation could not have covered.**
- 42/42 `file:line` citations in the spec resolve, 0 ambiguous.

## Where I already know the ice is thin — say so if you disagree

- **`replace_preserving_emptiness` is applied per-field by hand.** The RULE is derived; its
  APPLICATION is a call at each site. A new replaced string that forgets it would be caught by the
  matrix only if the matrix's axis picks the field up — which it should, by derivation. Verify that
  loop actually closes.
- **The emptiness-class assertion walks string leaves in parallel.** If the two trees diverge in
  SHAPE (an array shortened), `zip` truncates and the tail goes unchecked.
- **`ledger_contribution` returns the FIRST firing disjunct.** Order affects only the message, not the
  verdict — confirm that is still true.

## FORBIDDEN

- Re-auditing tax logic or anything outside `income scrub`'s blast radius.
- Style, prose, naming, doc-comment length.
- Re-reporting a settled item, or re-verifying the machine-checked list.
- "Add more tests" without naming the defect and the mutation that would kill it.

## Severity

**Critical** — wrong tax figure, data loss, security exposure, or **a false safety authorization**.
**Important** — a real defect, missing case, unsound assumption, or a guarantee no test holds.
Both block the ship. Minor/Nit recorded.

## OUTPUT FORMAT

```
VERDICT: <ship | needs-changes | ship-after-fixing-X>

CRITICAL: <n>
IMPORTANT: <n>

For each finding:
  [C|I|M|N] <one-line title>
  WHERE: <file:line>
  FAILURE: <concrete inputs/state → wrong outcome. Name the vector.>
  FIX: <smallest change that closes it>

r1 FINDINGS vs THE BUILD: <seam 1. For each r1 finding: closed | partially | NOT closed | n/a.
Name any that fell out between r1 and the spec.>

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence>
```

If nothing blocks, say so plainly and name the seam you looked hardest at.
