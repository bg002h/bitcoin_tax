# BRIEF — independent review of `design/SPEC_income_scrub.md` (r3)

## The ONE question

**Is this spec safe to build from?** If a competent implementer follows it literally and the branch
then goes green, does `btctax income scrub` ever tell a filer "this file is safe to hand to a stranger"
when it is not — or emit a file that computes or *prints* a different return than the original?

## ★★★ REVIEW THE FOLD. That is the text nobody has read.

r3 is a fold responding to r2. **A fold is authorship and re-earns the gate** — review-response edits
are where transcription defects come from, and they are the one part of this document no reviewer has
ever seen.

```
git show c098169        # the r2 report, verbatim — what was found
git diff c098169..4b49c8c   # THE FOLD — exactly what changed in response, and nothing else
```

**Spend your budget on that diff.** The sections it touches are §2.2, §3.2, §3.3, §4.3, §5.1, §7, §8.
Everything else in the document has now been read by two independent reviewers.

The specific question on the fold: **did each response actually close its finding, or does it restate
the finding in the imperative and leave the defect reachable?** And separately — did any response
introduce a *new* defect? Three of the four r2 Importants were fixed by replacing a hand-list with a
derivation; check that each derivation is genuinely computable by an implementer and does not silently
require the same hand-list one level down.

## What the command is

`btctax income scrub` emits a copy of a real person's complete tax return — identity replaced, every
figure intact — and tells them it is safe to send to a stranger. **The product is the authorization,
not the file.** A false safety claim is unauditable after the fact and persists in every installed copy
forever. The spec is deliberately written to refuse more than it scrubs. The implementation exists on
this branch, was held back from every release, and this spec is the rework order for it.

## Read these — by path, not pasted

| path | what |
|---|---|
| `design/SPEC_income_scrub.md` | **the artifact (r3, 436 lines)** |
| `reviews/scrub-spec-r2-review.md` | **r2: 0C/4I/3M — the findings this fold answers** |
| `reviews/scrub-spec-r1-review.md` | r1: 31 raw → 19 blocking, folded into r2 |
| `reviews/scrub-r1-workflow.md` | code review of the held implementation, 2 CRITICALs |
| `crates/btctax-core/src/tax/scrub.rs` | the held implementation (423 lines) |

## ★ SETTLED — do not re-derive, do not re-report

**From r1:** `absent → absent` stands; scrubbing the ledger is permanently rejected; no perturbing or
rounding of figures; the refusal is a `CliError` not a `RefuseReason`; dependent DOB is dropped;
`naics_code` and box-12 codes are kept.

**From r2 — it checked these and could NOT break them. Do not re-spend budget here:**
- §2.2's four disjuncts are complete. `Severity` has exactly two variants and every gating read filters
  on `Hard`; every other `state` read on the report/export path is year-filtered; the pseudo-placeholder
  channel is unreachable from scrub; carryforward-out is covered by two existing disjuncts.
- The provenance marker cannot be silently stripped by any in-repo path.
- §8 step 2 is compatible with `scrub_return_inputs`'s current signature.
- §6's dependent-DOB drop moves no figure (no reader of a dependent's DOB exists).
- The IP PIN carve-out is coherent; its only open edge was the `NotDigits` payload, now folded.

## ★★ ALREADY MACHINE-CHECKED — do not re-verify

Run before this dispatch, on r3 as committed:

- **20/20 `file:line` citations resolve; 0 out-of-range; 0 ambiguous.** The gate's first run caught two
  citations matching files in more than one crate (`packet.rs`, `testonly.rs` exist in `btctax-core`
  *and* `btctax-forms`/`-cli`); both are now crate-qualified.
- The spec carries no compilable blocks — the `ledger_contributes` block is pseudocode.
- The full suite (2646 tests) + `cargo fmt --all --check` are green at `4b49c8c`.
- §5.1's divergence table was **derived against source, not transcribed from r2**, and it differs from
  r2's list in two rows: `w2s[].employer` and `g_1099[].payer` reach no printed cell and no message
  (verified: zero hits for `.employer` in `btctax-forms`/`printed.rs`; no reader of `g_1099` payer
  anywhere outside `scrub.rs`). **If you think either belongs in the divergence set, say so and name
  the reader** — that would be a real finding.

## Where the risk now is

1. **§3.2's `NotDigits(_) → NotDigits('x')` coarsening + §3.3 test 2's asymmetric comparison.** Is the
   pair actually coherent — does the stated comparison hold every other leg while permitting this one?
   Is `'x'` a safe choice (it is itself a non-digit, so the variant survives — confirm)?
2. **§3.2's `"" → ""` emptiness rule.** Does preserving emptiness leak anything, and does it cover
   every reader of an emptiness predicate, or only the broker one r2 found?
3. **§3.3's derived field axis.** "Every field `scrub_pii` replaces" — is that mechanically
   determinable by a test, or does it need a hand-list to enumerate, reintroducing the defect it fixes?
4. **§5.1's derivation.** "Reaches a printed cell or a user-facing message" — is that the right
   boundary, and is the enumeration complete under it?
5. **§7's out-of-scope call.** Is declaring "commit a scrubbed return as a fixture" out of scope for v1
   actually safe, or does something in the spec still depend on that workflow?
6. **§8's re-sequencing.** Old steps 3+5 merged. Does the new order leave any step depending on
   something a later step creates?

## FORBIDDEN

- Re-auditing tax logic, the ledger engine, or anything outside `income scrub`'s blast radius.
- Style, prose, naming, section ordering. "§X disagrees with §Y" is a lookup, not a finding.
- Re-reporting an r1 or r2 finding (see SETTLED), or re-verifying the machine-checked list.
- "Add more tests" without naming the specific defect and the mutation that would kill it.
- Generic self-verification scaffolding.

## Severity

**Critical** — wrong tax figure, data loss, security exposure, or **a false safety authorization**.
**Important** — a real defect, missing case, unsound assumption, or a guarantee no test could hold.
Both block. Minor/Nit are recorded and do not gate.

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

DID THE FOLD CLOSE r2? <per finding I-1..I-4, M-1..M-3: closed | partially | reopened | new-defect>

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence naming the assumption your findings depend on>
```

If nothing blocks, say so plainly and name the section you looked hardest at. **A clean result closes
the loop** — this is the third review of this document, and the gate is 0C/0I, not zero findings. Do
not manufacture findings to look thorough; do not soften a Critical because the document is careful.
