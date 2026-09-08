# Seam review — interview build T7 (the dependents gates), `282a8a32`

Reviewer: independent build reviewer, own worktree at `8569b450`. Every plant below was made in this
worktree and reverted from a `cp` backup; `git status --porcelain` is empty at the time of writing.
No commits, no subagents.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

# plants (each reverted immediately afterwards)
cargo nextest run --locked -p btctax-cli  -E 'test(probe_gate_answered_through_the_command)'   --no-capture
cargo nextest run --locked -p btctax-cli  -E 'test(probe_commit_gate_after_the_real_command)'  --no-capture
cargo nextest run --locked -p btctax-cli  -E 'test(probe_bare_enter_accepts)'                  --no-capture
cargo nextest run --locked -p btctax-core -E 'test(review_probe)'                              --no-capture
cargo nextest run --locked -p btctax-core -E 'test(review_probe2)'                             --no-capture
cargo nextest run --locked -p btctax-core --no-fail-fast          # step-4 citizen STOP deleted
cargo nextest run --locked -p btctax-core --no-fail-fast          # age test fed the taxpayer DOB
cargo nextest run --locked -p btctax-input-form --no-fail-fast    # dep_gate_tristate! indices 5/6 swapped

# baseline, unmutated tree
cargo nextest run --locked -p btctax-core -p btctax-input-form --no-fail-fast
  → 1385 tests run: 1385 passed, 0 skipped
```

Primary sources read: `design/forms/extract/i1040gi--2025.txt:1447-1812` (the whole flowchart),
`legal/text/irs-guidance/RevProc_2023-34.txt:615`, `RevProc_2024-40.txt:576`,
`RevProc_2025-32.txt:909`, `design/SPEC_interview.md` R6 / §7 row T7.

## Summary

The flowchart itself is transcribed faithfully — I re-drove every edge of R6's table against the
instruction's own text and found no wrong verdict, no declinable value feeding Step 1, and no print
path a row can reach with an incomplete chain. The defects are all at the **seams around** the walk,
and the worst of them is fatal: **every gate answered through `income answer` is recorded under a
prompt string no reader can reproduce, so the command's own "after" panel reports all fifteen gates
as *answered under earlier words* the instant the filer answers them, and the commit gate refuses
with an exit the filer has just followed.** A dependent cannot be claimed on an incomplete chain;
right now a dependent cannot be claimed at all.

Four of the five Importants are the same shape one layer out: an instrument that would have caught
something did not run the step that breaks it (I-1), a decision `provenance.rs` explicitly deferred
to T7 was met rather than decided (I-2), a rule the codebase already learned once was re-broken on
the one gate it applies to (I-3), and a shipped `--help` sentence now states the opposite of what the
command does (I-4). I-5 is a REFUSE edge of R6's table that no KAT row covers.

---

## C-1 (Critical) — every dependent gate answered through `income answer` is immediately stale, and the return can never be committed

**Where.**
`crates/btctax-cli/src/cmd/answer.rs:747` and `:817`:

```rust
let prompt = format!("[{banner}] {}", gate.prompt_text(&ri, params.as_ref()));
…
record_answer(&mut ri, key, &prompt, now, AnswerState::Given);
```

versus `crates/btctax-core/src/tax/provenance.rs:759-762`, the one resolver of "the words currently
asked":

```rust
AnswerKey::DependentGate { gate, .. } => {
    let q = crate::tax::dependent_gates::entry(*gate);
    (!q.needs_params()).then(|| q.prompt_text(ri, None))
}
```

`record_answer` hashes the **banner-prefixed** string; `current_prompt` returns the **unprefixed**
one. Every reader of R10.3 therefore reports `WordingChanged`:
`return_refuse.rs:1610` (`screen_dependent_gates`), `interview_state.rs:319-330`, and — for the
params-quoting gate, which escapes `current_prompt` — `return_refuse.rs:1601-1608` and
`interview_state.rs:322-324`, which both compare against `g.prompt_text(ri, params)`, also unprefixed.
So **all twenty-one gates** are affected, not just the twenty with a resolver.

`answer.rs:813` is the only production writer of `AnswerKey::DependentGate` (grepped: the TUI and the
input form write the leaf but no record), so the blast radius is exactly `income answer` — the
primary interview surface, and the one every refusal detail tells the filer to run.

**What is wrong.** This is the D-1 defect class that `answer_status`'s own doc comment says was closed
one layer up — *"a refusal firing on a correct answer … the wrong comparand is no longer something a
caller can supply"* — reintroduced at the one new call site, because the banner is display chrome that
was folded into the hashed string.

**Evidence (the plant and the observed output).** Probe planted in
`crates/btctax-cli/tests/open_next_year_t4b.rs`: seed a prior dependent, run the **real**
`answer_return_inputs` through the existing `answer_the_draft` harness (which answers every gate at
its claim path), then read the draft back.

The command's own trailing panel, printed to the filer:

```
── The answer panel (after) ──
  BLOCKING (15) — commit waits on these:
    • What is this person's date of birth? [the wording of this question changed since you answered]
    • Is this person your son, daughter, stepchild, foster child, … ? [the wording of this question changed since you answered]
    • Was this person younger than you (or your spouse if filing jointly) at the end of the tax year? [the wording of this question changed since you answered]
    … (all 15 gates, same reason)
```

Every record, immediately after being written:

```
DependentGate { ssn_hash: "b899c059…", gate: QcRelationship }    -> state=Given status=WordingChanged
DependentGate { ssn_hash: "b899c059…", gate: DateOfBirth }       -> state=Given status=WordingChanged
… 15 of 15
```

And the commit gate (second probe, `screen_inputs` with the TY2024 package):

```
SCREEN_INPUTS after answering everything => Some(Refusal {
    reason: DependentGateUnanswered { row: 0, gate: DateOfBirth },
    detail: "you answered this question, but the wording of this question changed since you answered
             — … Re-answer it with `btctax income answer` …",
})
```

The exit the refusal offers is the command that produced the state. Re-running it re-records the same
banner-prefixed string, so the loop never terminates: this is a **brick**, not a delay.

**Minimal change.** Keep the banner as display only — hash the registry's words, the way the
`Declaration` arm at `answer.rs:574` already does (`let prompt = q.prompt_text(&ri).into_owned();`):

```rust
let words = gate.prompt_text(&ri, params.as_ref());
let shown = format!("[{banner}] {words}");   // written to `out`
…
record_answer(&mut ri, key, &words, now, AnswerState::Given);
```

Pair it with a kill that reds: the existing sweep KAT extended to call `record_answer` and then assert
`screen_inputs(...).is_none()` (see I-1).

---

## I-1 (Important) — the KAT that names the no-brick property omits the step that breaks it

**Where.** `crates/btctax-cli/src/cmd/answer.rs:1227`,
`income_answer_asks_the_dependent_gates_and_the_sweep_settles`.

**What is wrong.** Its doc comment and its own inline comment say it runs *"the sweep, exactly as
`answer_return_inputs` runs it"*, and its fourth assertion is

```rust
// ── 4. The whole return then screens clean — the no-brick property, with gates. ──
assert!(screen_inputs(&ri, &table, &params).is_none(), "answering every live gate must clear the screen: …");
```

The emulation reproduces `live_questions_with`, the `asked` set, `key_of`, the re-check of liveness and
the `set` — and **not** `record_answer`, which is the only line of the real loop that can make that
assertion false. With no record in the log, `answer_status` returns `NeverAsked`, the staleness branch
is skipped, and the test reports PASS on a return the real command cannot commit. It is green because
it never ran the step it exists to cover — the B1 shape, in the build's own headline instrument.

**Evidence.** The test passes at `282a8a32` (`1315 passed`); the same scenario driven through the real
`answer_return_inputs` refuses, as quoted in C-1. Nothing had to be mutated to show it.

**Minimal change.** Add the recording site to the emulation (or, better, drive
`answer_return_inputs` itself, as `open_next_year_t4b.rs::answer_the_draft` already does) and keep
assertion 4. That single line is the kill for C-1.

---

## I-2 (Important) — two dependent rows with the same SSN hash starve one row's gates: `income answer` ends with a live gate never asked

**Where.** `crates/btctax-cli/src/cmd/answer.rs:175-186` (`key_of`) and `:527` (the `asked` filter);
`crates/btctax-core/src/tax/provenance.rs:592-600` (`dependent_ssn_hash`), whose doc at `:780-784`
says in terms: *"a row with a BLANK `ssn` has no identity, so every blank row shares one bucket …
**Recorded here so T7 decides it rather than meets it**."*

**What is wrong.** T7 met it. The sweep's `asked` set is keyed by `AnswerKey`, i.e. by
`(dependent_ssn_hash(row.ssn), gate)`. Two rows whose SSNs hash the same — both blank, or the same
digits typed twice — collide. Within one round both asks are still in the collected `round`, so the
collision is invisible; **across** rounds it is not: a gate row 0 was asked in round *n* is filtered
out of row 1's round *n+1*, is never put to the filer, stays `None`, and `screen_dependent_gates`
then refuses it forever. It is fail-closed on the claim (nobody is claimed on an incomplete chain)
and fail-open on the interview: the command exits reporting nothing left to ask.

Reachable at authoring: `screen_inputs` has no dependent-SSN gate (`return_refuse.rs:1896-1901`, *"NO
SSN GATE HERE, DELIBERATELY"*) and no uniqueness check anywhere; the identity boundary is
`ReturnHeader::build`, at the packet.

**Evidence.** Core probe replicating the command's sweep with two blank-SSN rows — row 0 a qualifying
child, row 1 a qualifying relative:

```
sweep 1: 8 Step-1 gates for BOTH rows
sweep 2: [(0, LivedWithYouInUs), (1, LivedWithYouInUs)]
sweep 3: [(0, QualifyingChildOfAnotherPerson), (0, CitizenNationalResidentOrCanadaMexico), (0, Married),
          (1, QrRelationshipOrMemberOfHousehold), (1, QualifyingChildOfAnyTaxpayer), (1, GrossIncomeUnderLimit),
          (1, YouProvidedOverHalfSupport), (1, DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies)]
sweep 4: [(0, TinIssuedByDueDate), (0, CitizenNationalOrResidentAlien)]
sweep 5: [(0, SsnsValidForEmploymentIssuedByDueDate)]
sweep 6: nothing left to ask — the interview ENDS here
row 0 final verdict = ChildTaxCredit
row 1 final verdict = Unanswered(CitizenNationalResidentOrCanadaMexico)
```

Row 1's Step 4 questions 2 and 3 became live in sweep 4 and were filtered by row 0's sweep-3 keys.
Separately, that a blank-SSN row gets that far at all:

```
blank-SSN single row screen => None
hash(row0)=2e3139be… hash(row1)=2e3139be… equal=true
retiring the BLANK identity of row 1 removed 1 record(s); row 0's record still present = false
```

**Minimal change.** Decide the deferred question rather than re-file it. Either (a) refuse a dependent
row with a blank or duplicated SSN in `screen_dependent_gates`, before any gate is demanded — the row
is a claim, and R6 already makes the identity fields mandatory — or (b) key the sweep's `asked` set by
`(row, gate)` (it is per-session and never stored, so the index is safe there) while leaving the
*stored* `answer_log` key as the identity. (a) is the one that also closes the
`retire_dependent_identity` cross-deletion above.

---

## I-3 (Important) — the seeded dependent date of birth is pre-filled, and a bare Enter writes a fresh `Given`: the C-1 rule broken on T7's one `Durable` gate

**Where.** `crates/btctax-cli/src/open_next_year.rs:444` (`date_of_birth: d.date_of_birth`) together
with `crates/btctax-cli/src/cmd/answer.rs:767-773`:

```rust
Ok(None) => match cur {
    Some(_) => break,            // a bare Enter KEEPS what is on file
    None => writeln!(out, "  a dependent row needs a date of birth: …")?,
},
```

…followed unconditionally by `record_answer(…, AnswerState::Given)` at `:817`.

**What is wrong.** `DEPENDENT_GATES` declares `DateOfBirth` as `Durability::Durable`, and
`questions.rs:32-35` defines that as: *"The prior MAY be displayed, but it still requires the same
explicit keystroke as a fresh ask: **never Enter-to-accept, never pre-filled**."* Both halves are
violated. This is precisely the C-1 defect recorded ten lines above the seed
(`open_next_year.rs:397-410`): *"pre-filling it broke that in one command … a prior-year answer
satisfying this year's provenance, which is the one thing R10 exists to prevent — and 'a diligence
record that lies is worse than none'."* The build report argues (§4) that C-1's harm was specific to a
skippable, where Enter means *decline*. That disposes of half of it. The other half — a `Given`
record dated **this** year for a value the filer never typed — is identical, and here the value is
load-bearing: the age test computed from it decides §152(c) vs §152(d), i.e. CTC vs ODC.

**Evidence.** Probe: seed a prior dependent with DOB 2015-04-01, drive the real command with a **bare
Enter** at the date prompt, read the draft back.

```
row date_of_birth after a BARE ENTER = Some(2015-04-01)
answer_log[DateOfBirth] = Some(AnswerRecord { answered_on: 2026-02-03, prompt_hash: "17b894f1…", state: Given })
```

The prompt itself shows the pre-fill: `[Sam Filer] What is this person's date of birth? [YYYY-MM-DD;
currently 2015-04-01]:`.

**Minimal change.** The one-line flip the build report itself names: drop `date_of_birth` from
`open_next_year::seed`'s `Dependent` literal, and show it as a typed hint the way `carry_person`
already does for the taxpayer (`opened_from` + the "type it to confirm" path). The gate then blocks
until the filer types it, which is what `Durable` means. Update
`a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not`'s DOB assertion accordingly.

---

## I-4 (Important) — `income open-next-year`'s shipped help says the opposite of what FR-70 now does

**Where.** `crates/btctax-cli/src/cli.rs:582-584` (single-sourced into `--help` and the man page) and
`docs/man/btctax-income-open-next-year.1:13`:

> "Dependents and exchanges are named as questions but **no row is created: an unconfirmed dependent
> is absent, never claimed**."

and, six lines earlier at `cli.rs:573-577`:

> "WHAT COMES WITH THEM, **and nothing else**: your filing status, your name and SSN (and your
> spouse's), your mailing address, each employer and payer by name and EIN/TIN with every box blank,
> and last year's computed carryforwards."

**What is wrong.** FR-70 reverses the first sentence outright — a row **is** created now, carrying the
prior year's name, SSN, relationship and date of birth — and falsifies the closed list in the second.
Neither `cli.rs` nor `docs/man/` is in `git diff 8d2f6b83..282a8a32`. The claim that changed is
exactly the safety property the filer would be reasoning about: a filer who reads it believes last
year's dependent did not carry over and will not go looking for a row to delete. Every other
user-facing surface T7 touched (the module doc at `open_next_year.rs:368`, the T4b test, the seed
comment) was updated; the two that ship to the user were not.

**Evidence.** `grep -n "no row is created" crates/btctax-cli/src/cli.rs
docs/man/btctax-income-open-next-year.1` → both hit at `282a8a32`; `git diff --stat 8d2f6b83..282a8a32`
lists neither file.

**Minimal change.** Rewrite the two sentences to state the new behaviour — the person crosses with
every §152 gate blank, and the row blocks until this year's flowchart is answered or the row is
removed — and regenerate the man page (`make docs`).

---

## I-5 (Important) — the Step 4 question 2 STOP can be deleted and the whole core suite stays green

**Where.** `crates/btctax-core/src/tax/dependent_gates.rs:414-422` — the citizen arm inside `step4`,
with its own distinct rule string (*"covers an adopted **person** only"*, versus Step 2's *"adopted
**child** only"*).

**What is wrong.** R6's table names this edge — *"Step 4 | citizen / married / joint /
could-you-be-claimed — the Step 2 gates reused | as Step 2"* — and the instruction states it as its
own question (`i1040gi--2025.txt:1765-1772`). `the_flowchart_truth_table`'s citizen row starts from
`at_the_ctc_edge`, so it exercises `step2`'s arm; `the_step_four_truth_table` has rows for the Step 4
relationship, qualifying-child-of-any-taxpayer, gross income, support, the three rules, married and
could-you-be-claimed — but none for citizenship. `every_refuse_edge_names_its_rule_in_the_screen_detail`
likewise perturbs a CTC-edge row. Deleting the arm claims a non-citizen, non-resident qualifying
relative as a dependent — a wrong result on the form's own terms — with nothing red.

Two smaller cells of the same table are also uncovered: Step 4 question 4 (`filing_status == Mfj` on
the relative path ⇒ claimed, on to Step 5; verified `CreditForOtherDependents` by probe) and Step 5
question 1 answered *No* on the relative path (only the Step-3-derived path has a row).

**Evidence.** Plant — the whole `if w.no(G::CitizenNationalResidentOrCanadaMexico) { … }` block in
`step4` replaced by a comment:

```
Summary [0.547s] 1315 tests run: 1315 passed, 0 skipped
```

Reverted. For contrast, the seam-2 plant below reds 7 tests, so the suite is not simply insensitive.

**Minimal change.** Add a row to `the_step_four_truth_table` for the citizen STOP, asserting the gate
and a fragment of its own rule string (which differs from Step 2's, so the assertion also pins that the
Step 4 arm is the one that ran); optionally two more rows for Step 4 q4 and Step 5 q1 on that path.

---

## M-1 (Minor) — the "could you be claimed" refusal is attributed to a gate on the row, so the input form anchors the filer on the wrong control

`dependent_gates.rs:315-320` and `:438-443` build the refusal as
`refused(G::QcRelationship, …)` / `refused(G::QrRelationshipOrMemberOfHousehold, …)` when the answer
that stopped the flowchart is the **return-level** `can_be_claimed_as_dependent_taxpayer`.
`attribute.rs:84-87` then anchors `DependentGateRefused { gate }` on `DepGateQcRelationship`. A filer
who follows the anchor and flips the relationship gate routes the row to Step 4 and is refused again
with the same exit. The refusal *text* is correct (it names Step 2 q4 / Step 4 q5 and the exit
sentence, and `every_refuse_edge_names_its_rule_in_the_screen_detail` covers it), which is why this is
Minor rather than blocking. The clean shape already exists next door:
`DependentVerdict::WaitingOnQuestion(QuestionId::DependentTaxpayer)` — a `RefusedByQuestion` sibling
would let `attribute` return `decl(QuestionId::DependentTaxpayer)`.

## M-2 (Minor) — the gate refusal detail carries two 22-space runs

`return_refuse.rs:1627` is a single-line string literal containing
`…send you to is {}.` + 22 spaces + `Remove the row, … flowchart there` + 22 spaces + `` (`btctax income answer`). ``
The filer reads those spaces. `cargo fmt` does not touch string literals, so nothing catches it. Fix:
break it into `\`-continued lines.

## M-3 (Minor, secret-handling class — non-gating) — `income scrub` now emits every dependent's exact date of birth

D5 is *defensible*: §3.2 requires a replaced field to preserve every property a screen reads, T7 gave
the field three readers, and `scrub_person` keeps the taxpayer's DOB on the same reasoning. The man
page (`docs/man/btctax-income-scrub.1:13`) already promises "dates of birth and death … survive
unchanged", so nothing shipped is now false. But the module's own established pattern for a field
that is *both* load-bearing and identifying is `synthetic_ssn_like` — **replace, preserving the read
property** — and a synthetic date preserving `considered_age_at_year_end` exactly would satisfy §3.2
without a child's real birth date leaving the vault in a file stamped shareable. Answering the brief's
question (a) directly: the file `income scrub` writes carries a dependent DOB it did not carry before;
nothing else downstream changed, because `scrub_pii`'s only consumer is `cmd/tax.rs:412`.
Per the 2026-08-27 owner ruling this is a follow-up, not a gate.

## M-4 (Minor) — a flowchart STOP is gated behind `unanswered_refuses`, so `income import` stores a plainly-refusing dependent

`return_refuse.rs:1963-1975` puts the whole of `screen_dependent_gates` inside
`if tier.unanswered_refuses`, whose stated rationale is that `income import` is the only row-creating
path. That rationale covers `DependentGateUnanswered`; it does not cover `DependentGateRefused`, which
is a **stated** answer that stops the flowchart and is not made unreachable by refusing at import (the
filer removes the row or changes the answer). The file's own comment two paragraphs up draws exactly
this line for the census: *"The census's own VALUE rules are NOT in that tier and DO refuse at
import."* As built, a TOML saying the dependent is married and filing jointly imports cleanly and
poisons the year until `report` — the shape T4 introduced the tier split to avoid.

## M-5 (Minor) — the §152(d) figures are typed a second time, in the gate's `help`

`dependent_gates.rs`'s `GrossIncomeUnderLimit` help ends *"$5,050 for 2024, $5,200 for 2025, $5,300
for 2026"* — a hand-typed copy of `FullReturnParams::qualifying_relative_gross_income_limit`, in the
one module whose header states the rule against a list typed beside derived data. `prompt_from_params`
already renders the year's figure into the prompt; the help should not carry a table that can drift
from the params it duplicates. (All three values verified correct against
`RevProc_2023-34.txt:615` / `RevProc_2024-40.txt:576` / `RevProc_2025-32.txt:909` and
`i1040gi--2025.txt:1690`.)

## N-1 (Nit) — no truth-table row pins the under-17 January-1 boundary

Answering the brief's question (b): the boundary is **correct**. Probe over
`under_17_at_year_end`/`walk_dependent` at TY2025:

```
born 2009-01-01: considered_age=17  under17=false  verdict=CreditForOtherDependents
born 2009-01-02: considered_age=16  under17=true   verdict=ChildTaxCredit
born 2008-12-31: considered_age=17  under17=false  verdict=CreditForOtherDependents
born 2009-12-31: considered_age=16  under17=true   verdict=ChildTaxCredit
```

The convention comes from `return_1040.rs:1138`, whose own KAT pins the instruction's three January-1
examples. The truth table's under-17 row uses DOB 2007-06-01, nowhere near the edge, so the *gate's*
use of the convention is only transitively covered. A Jan-1/Jan-2 pair as a truth-table row would cost
two lines.

## N-2 (Nit) — `Census::dependent_gate` ignores its leaf, so a swapped leaf↔gate pairing would not red

`classifier.rs:92-99` takes `_leaf: &Option<bool>` and pushes only the gate, and
`the_classifiers_gate_rows_line_up_with_the_registry` compares **sets**. Mis-pairing two calls in
`classify_dependent` (e.g. `c.dependent_gate(married, G::FilingJointReturn)`) leaves the set identical
and reds nothing. Nothing user-visible moves today (the census only counts), and the analogous risk in
`sections.rs` *is* covered — I swapped `dep_gate_tristate!` indices 5 and 6 and
`every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` red immediately. Recording it because
the two registries look alike and only one is protected.

## N-3 (Nit) — `dep_gate_tristate!`'s `clear` does not check liveness while `set` does

`sections.rs`: `set` returns `SetError::NoSuchRow` when the gate is not live for the row; `clear`
writes `None` unconditionally. Harmless (clearing a non-live gate is a no-op in effect), but the
asymmetry is un-commented and invites the reverse mistake later.

---

## Seams checked clean

1. **The truth table, re-driven.** I read `i1040gi--2025.txt:1447-1812` in full and walked every edge
   of R6's table against the code: Step 1's five conditions and both joint-return limbs, the CAUTION,
   Step 2 q1–q4, Step 3 q1–q4 including the computed under-17 test, Step 4 q1's four conditions plus
   the three-rules gate, Step 4 q3/q4/q5, Step 5 q1–q3, and the born-in-year child. Question ORDER
   matches the instruction's in both `step2` and `step4`. Every verdict the KAT asserts is the
   instruction's. The only cells not covered by a KAT row are in I-5 and N-1; no cell produces a wrong
   outcome.
2. **Nothing declinable feeds Step 1.** `age_test(d: &Dependent, tax_year: i32)`
   (`dependent_gates.rs:525`) has no `&ReturnInputs` in scope, so it *structurally* cannot read the
   taxpayer's DOB; `under_17_at_year_end` is the same. `could_you_be_claimed` reads only
   `filing_status` and the return-level declaration. Plant: rewrite the Step 1 predicate so its
   "younger than you" limb compares the row's DOB against `ri.header.taxpayer.date_of_birth`. Result —
   **7 red**, including the named guard:
   `a_declined_taxpayer_date_of_birth_still_resolves_step_one`, `the_flowchart_truth_table`,
   `a_child_born_in_november_reaches_the_child_tax_credit`, `every_live_gate_that_is_blank_refuses_by_name`,
   `every_gate_at_its_claim_path_answer_reaches_the_ctc_edge`,
   `every_refuse_edge_names_its_rule_in_the_screen_detail`,
   `a_malformed_ssn_computes_but_the_packet_still_refuses_it`. Reverted.
3. **The required DOB, and no print path.** `date_of_birth = None` ⇒
   `Some(DependentGateUnanswered { row: 0, gate: DateOfBirth })` through `screen_inputs` (probe).
   The single production screen chokepoint is `resolve.rs:96`; the only other non-test callers are
   `input_form_store.rs:666` and `cmd/tax.rs`. `dependents_statement` and the emitter take a packet
   `ReturnHeader`, which is built after that screen — I confirmed this by type error when trying to
   call the statement emitter on a raw `ReturnInputs`. No row reaches paper on an incomplete chain.
4. **Per-row liveness and the answer key.** Liveness has exactly one definition
   (`DependentGateQuestion::live` → `walk_dependent(...).demands`), so there is no second flowchart to
   drift. `set` on a non-live gate returns `NoSuchRow` and `get` returns `None` (the I-4 emulation, as
   R6 specifies). The hand-typed `dep_gate_tristate!` index ↔ `FieldId` pairing IS protected: swapping
   indices 5 and 6 red `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt`. Keys are the
   row's salted SSN hash everywhere (`answer.rs:178`, `:813`; `return_refuse.rs:1596`;
   `interview_state.rs:296`, `:347`) — never the index. The degenerate blank/duplicate-SSN case is
   I-2.
5. **Waiting versus blocking.** The panel lists `gross_income_under_limit` as *waiting on the year
   package* when `params.is_none()`, and `screen_dependent_gates` can never disagree with it, because
   the only params-less caller (`screen_param_free`) sets `unanswered_refuses: false` and so never
   reaches the gate loop at all. The figure and its cites are right in all three years:
   $5,050 / `RevProc_2023-34.txt:615` §3.24; $5,200 / `RevProc_2024-40.txt:576` §2.24 and
   `i1040gi--2025.txt:1690`; $5,300 / `RevProc_2025-32.txt:909` §4.23 — each read from the file, each
   under a heading I resolved rather than assumed.
6. **The opener (FR-70).** Verified in the tree: `seed` writes name/ssn/relationship/date_of_birth and
   twenty explicit `None`s with no `..Default::default()` tail; nothing copies `answer_log`; the
   seeded row is blocking through `interview_state` (my own probe printed 15 gate items); the T4b test
   was rewritten with its reason recorded, not deleted; `filer_tin_issued_by_due_date` is
   `live: |ri| !ri.header.dependents.is_empty()` and seeds as `None` (it is a header tri-state, which
   the seed drops). The DOB half of the seed is I-3.

**Also verified, on the build report's other claims:** D1 (no money leaf ⇒ no `LEAF_SOURCE` entry) —
correct, `LEAF_SOURCE` partitions `Usd`/`Option<Usd>` only. D2 — correct, `PARAMS_GATED_PROMPTS` is
`&[(QuestionId, &str)]` and could not hold a `DependentGate`. D3 — correct, `8d2f6b83` has
`dependents: Vec::new()`. D4 — correct, `:1743-1747` is Step 5 q1's own sentence; `:1765-1790` is the
Step 4 q2 / Step 5 q3 block. D6 — the age and under-17 tests are indeed unavoidable in T7 (liveness
depends on them) and change no emitted output. D7 — the `tax_year == 0` assertion is right and the
failure it prevents is real. D8/D9/D10/D11 — as described. Follow-up 1 is I-2 (understated: it is not
only a shared key space but an unaskable gate); follow-up 2 (the `tax_tables.rs` TY2026 §2.14/§2.10
cites) is genuinely pre-existing and the new §4.23 cite is accurate; follow-up 3 is real and is the
same figureless label M-5 touches.

Counts: C=1 I=5 M=5 N=3
