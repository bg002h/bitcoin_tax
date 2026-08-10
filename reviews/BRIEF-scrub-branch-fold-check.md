# BRIEF — scoped fold check on the whole-branch review's findings

**Not a fresh review.** Six passes have run on this branch. This checks ONE diff.

```
git show b1356781            # the whole-branch report (1C / 1I / 4M / 2N)
git diff b1356781..ef98b021  # THE FOLD — the only text under examination
```

## The ONE question, in two parts

For each finding — **C-1** (a malformed EIN upgraded to a well-formed synthetic, so the copy files
where the original refused and claims a $1,546.80 §6413(c) credit), **I-1** (the round trip could not
fail on a lossy emit), and **M-1…M-4, N-1, N-2**:

1. **Did the fold CLOSE it**, or restate it without closing it?
2. **Did the response introduce a NEW defect?**

## ★★★ WHERE TO PRESS HARDEST

Fixing C-1 exposed something bigger than the finding. `screen_inputs` returns the **first** refusal,
and `maximal_sentinel` carried `foreign_trust: Some(true)` — so **all 23 rows of §3.3's matrix were
comparing `Some(ForeignTrust)` to itself**. The entire refusal assertion asserted nothing. Four more
masks surfaced one behind another (a non-public-charity gift class, gibberish box-12 codes, a claimed
IRA deduction).

The fixture is now a return that FILES, and the matrix asserts `screen_inputs(base) == None` as its
first act.

**So: is the sentinel still MAXIMAL after being made fileable?** Every `Option` must still be `Some`,
every `Vec` still ≥2, the literal still exhaustive with no `..`. If making it fileable quietly shrank
it, the derived axis shrinks with it and fields drop out silently — which is the exact failure the
axis exists to prevent, arriving through its own fixture. **Check the derived path set still contains
`w2s[].ein`, `header.ip_pin`, `foreign_country_names`, `b_1099[].payer` and
`schedule_c.business_description`.**

Second: does `synthetic_malformed_ein` actually fail `canonical_ein` for every `n`, and can it
collide with `synthetic_ein`'s range?

## Read these — by path

| path | what |
|---|---|
| `reviews/scrub-branch-prebuild-review.md` | the findings being answered |
| `crates/btctax-core/src/tax/scrub.rs` | `EinMap`, `synthetic_malformed_ein` |
| `crates/btctax-core/src/tax/scrub_axis.rs` | the sentinel + the matrix + the new baseline guard |
| `crates/btctax-cli/tests/scrub_refusal.rs` | the round trip |

## Already machine-checked — do not re-verify

- 2666 tests pass; fmt + clippy clean; man pages regenerated.
- Restoring the C-1 defect reds BOTH the direct test and the matrix (it red neither before).
- Dropping `payments` from the emitter reds the round trip (it survived before).

## FORBIDDEN

Re-auditing anything the diff does not touch. Re-reporting a settled item. Style. Manufacturing a
finding — **"all closed, nothing new" is the expected and useful result.**

## OUTPUT FORMAT

```
VERDICT: <all-closed | needs-changes>

C-1: <closed | partially | reopened | new-defect>  — evidence
I-1: <...>  — evidence
M-1 / M-2 / M-3 / M-4 / N-1 / N-2: <...> — one line each

IS THE SENTINEL STILL MAXIMAL? <yes/no, with the derived path set as evidence>

NEW DEFECTS INTRODUCED BY THE FOLD: <none, or list with severity, where, and the failure vector>

WHAT WOULD MAKE THIS CHECK WRONG: <one sentence>
```
