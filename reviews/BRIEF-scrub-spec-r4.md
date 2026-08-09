# BRIEF — independent review of `design/SPEC_income_scrub.md` (r4)

## The ONE question

**Is this spec safe to build from?** If a competent implementer follows it literally and the branch
then goes green, does `btctax income scrub` ever tell a filer "this file is safe to hand to a stranger"
when it is not — or emit a file that computes or *prints* a different return than the original?

## ★★★ REVIEW THE FOLD, and there is a second question this round

```
git show 970dc98             # the r3 report, verbatim
git diff 970dc98..d9d7b87    # THE FOLD — what changed in response, and nothing else
```

r4 responds to r3. **A fold is authorship and re-earns the gate.** Sections touched: §2.2, §3.2, §3.3,
§5.1, §7, §8, and the status header. Three independent reviewers have read everything else.

### ★★ THE SECOND QUESTION — answer it explicitly, in its own section

**Three rounds have now found the same defect at three depths.** §3.2 states its rule over a
*mechanism*; each round has then found that some *instrument* implementing it was written as a
hand-list one level down. r2 found it in §3.3's field axis and §5's divergence set. r3 found it in r3's
own **replacements** for those. The status header carries the table.

This project's standing rule is: *"Stop reviewing a document once findings become 'section X disagrees
with section Y.' That is the signal the artifact has stopped being where the risk lives. Go execute
it."* And: *"Conformance ⇒ test. Judgment ⇒ review, kept scarce."*

**So: is the remaining risk in this document still PROSE-SHAPED, or has it become TEST-SHAPED?** If
your findings are things a test would catch permanently and a reader would have to catch forever, say
so and say which test. **"Stop reviewing this and go build it" is a legitimate and welcome verdict** —
it is not a failure to find things, and I would rather hear it now than after a fifth round. Equally,
if a real blocking defect remains, that outranks this question entirely: say it and gate the build.

## Where the risk is concentrated

The header names two paragraphs as the ones to check first, because if they are wrong the same defect
returns at a fourth depth:

1. **§3.3's derivation and its blind spot.** The replaced set is now defined as *the paths at which
   `to_value(ri)` and `to_value(scrub_pii(&ri))` differ, over an all-sentinel fixture*, with a
   per-field `assert_ne!` precondition as the stated blind spot. **Is that actually buildable, and is
   the blind spot the only one?** Consider: nested paths, `Option` vs absent, `Vec` element identity,
   a field replaced with a value that varies by index, and whether serde's representation makes any
   replaced field invisible.
2. **§3.3's third verdict** (`no such state`, with the type as the reason) — does it let a genuinely
   forgotten cell hide behind "the type has no such state"?
3. **§5.1's now-stated filter** — "the header identity is announced in one sentence, not enumerated;
   the table enumerates what a recipient would not predict from that sentence." Is the enumeration
   complete under that filter now that `header.ip_pin` is in it? r3 found the IP PIN by asking which
   replaced fields reach a printed cell; ask the same question of anything else.
4. **§3.2's trim-emptiness rule** — is `trim().is_empty()` the complete emptiness predicate across the
   whole reachable surface, or only across the three readers in `screen_inputs`?

## Read these — by path

| path | what |
|---|---|
| `design/SPEC_income_scrub.md` | **the artifact (r4, 558 lines)** |
| `reviews/scrub-spec-r3-review.md` | **r3: 0C/4I/3M/1N — what this fold answers** |
| `reviews/scrub-spec-r2-review.md` | r2: 0C/4I/3M |
| `reviews/scrub-spec-r1-review.md` | r1: 31 raw → 19 blocking |
| `crates/btctax-core/src/tax/scrub.rs` | the held implementation (423 lines) |

## ★ SETTLED — do not re-derive, do not re-report

**r1:** `absent → absent`; the ledger is never scrubbed; no perturbing figures; the refusal is a
`CliError`; dependent DOB dropped; `naics_code` and box-12 codes kept.

**r2 verified and could not break:** §2.2's four disjuncts are complete (Severity has two variants,
every gating read filters on `Hard`, every other state read is year-filtered, the pseudo-placeholder
channel is unreachable from scrub); the marker cannot be silently stripped; §8 step 2 fits
`scrub_return_inputs`'s signature; §6's DOB drop moves no figure.

**r3 verified and could not break:** the `NotDigits(_) → NotDigits('x')` coarsening and §3.3 test 2's
asymmetric comparison are coherent and `'x'` is safe (`Ssn::canonical` checks `NotDigits` *before*
length, so the variant holds at any length); preserving emptiness leaks nothing; §7's out-of-scope call
is safe and is the only branch the scanner can express; §4.3's `--force` scoping is complete.

## ★★ ALREADY MACHINE-CHECKED — do not re-verify

- **33/33 `file:line` citations resolve; 0 out-of-range; 0 ambiguous**, on r4 as committed.
- The gate's first run **failed** with 2 ambiguous (`return_inputs.rs` exists in `btctax-core` *and*
  `btctax-cli`); both are now crate-qualified.
- It also caught a citation the fold had transcribed from **r3's own report**: r3 cited
  `scrub.rs:222-223` for the carryforward pair, which is `:220-221` (`:222-223` is the AMT carryover
  pair). Corrected in place. **Treat r3's other line numbers as verified — I re-resolved them.**
- The full suite (2646 tests) + `cargo fmt --all --check` are green at `d9d7b87`.
- The spec carries no compilable blocks; the `ledger_contributes` block is pseudocode.

## FORBIDDEN

- Re-auditing tax logic, the ledger engine, or anything outside `income scrub`'s blast radius.
- Style, prose, naming, section ordering, document length.
- Re-reporting an r1/r2/r3 finding (see SETTLED) or re-verifying the machine-checked list.
- "Add more tests" without naming the specific defect and the mutation that would kill it.
- Manufacturing a finding to justify the round. **Zero findings is a valid, useful result here.**

## Severity

**Critical** — wrong tax figure, data loss, security exposure, or **a false safety authorization**.
**Important** — a real defect, missing case, unsound assumption, or a guarantee no test could hold.
Both block. Minor/Nit recorded, do not gate.

## OUTPUT FORMAT

```
VERDICT: <ready-to-build | needs-changes | wrong-shape>

CRITICAL: <n>
IMPORTANT: <n>

For each finding:
  [C|I|M|N] <one-line title>
  WHERE: <spec §, and file:line if any>
  FAILURE: <concrete inputs/state → wrong outcome. Name the vector.>
  FIX: <smallest change that closes it>

DID THE FOLD CLOSE r3? <per finding I-1..I-4, M-1..M-3, N: closed | partially | reopened | new-defect>

PROSE-SHAPED OR TEST-SHAPED? <the second question. Name which remaining risks a test would hold
permanently, and give a clear recommendation: another prose round, or go build.>

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence naming the assumption your findings depend on>
```
