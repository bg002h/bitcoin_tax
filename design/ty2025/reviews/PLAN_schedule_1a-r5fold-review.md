# Schedule 1-A PLAN — review of the **r5 FOLD** (Opus)

**Date:** 2026-09-05 · **Artifacts:** `0819fca5` (the r5 fold of
`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`, 186 insertions / 23 deletions, one file) and
`ff839ce7` (the code half of its I-2) · **Answers:**
[`PLAN_schedule_1a-r4fold-review.md`](./PLAN_schedule_1a-r4fold-review.md) (0C/4I/4M/1Nit).

**Scope honoured:** the r5 fold's answers, and the plan text they touched. SPEC r3 not re-reviewed;
nothing outside this plan and `ff839ce7` re-audited; no new scope, forms or checkers proposed.

---

## VERDICT

**0 Critical / 2 Important / 4 Minor / 1 Nit — DO NOT BUILD YET.**

**All four r4 Importants are genuinely closed**, and I tried hard to break each one against the primary
source rather than against the fold's summary of it. I-1's per-part completion-source table survives the
test the fold itself did not run (below). I-4 really does shut the pre-2025-vehicle laundering path.
I-3's conjunction is right and its backstop is real and mutation-tested. `ff839ce7` is the correct
missing third of I-2 and needs no fourth piece. **The fold widened nothing** — no task, no form, no
checker.

Both Importants are on **one seam**, and it is the seam the r5 fold itself opened. r4-M-1 said the KAT
could not see `label_reader` from `btctax-core`; the fold answered by moving the KAT to `crates/xtask/`
— and thereby moved it away from the **other** two things the same task requires it to read, both of
which are `#[cfg(test)]` and in-crate. The result is a T2 deliverable that, as specified, cannot compile
its own second half and cannot red on a worksheet omission:

- **I-1** — CORRECTION 2 says *"Use it: `printed_line(<the field's label>)`"*, and `printed_line` is a
  **private `#[cfg(test)]` function** in `btctax-core`. It does not exist when `btctax-core` is compiled
  as `xtask`'s dependency. Half 2 of four — one of the two the plan calls *"what make it a conformance
  check"* — is unreachable from where r5 just put the KAT, and so is its B1 kill-test (item 6 half 2).
- **I-2** — the KAT's expected set has a mechanical source for the 48 **form** labels and **none for the
  four worksheets**, which census F-4 in this same task measured as the blindness to avoid. `label_reader`
  structurally cannot supply them.

Both fixes are one paragraph and need no new code: **split the KAT along the crate line** — the
membership/label half in `xtask` (which needs `label_reader`), the per-line quotation half in
`btctax-core`'s existing `schedule_1a_conformance` module (which already has `printed_line` and **both**
in-crate fixtures, form and instructions). Everything required already exists in the tree.

---

## FINDINGS

### I-1 — Important. r5 item 5 moved the KAT to a crate where `printed_line` does not exist, disabling half 2 of four and its B1 kill

**What the fold says.** §T2 item 5, new in r5: *"**The conformance KAT lives in `crates/xtask/`, not in
`btctax-core`** (r5 M-1 — *say which*)."* It keeps CORRECTION 2 unchanged: *"★
`crates/btctax-core/src/tax/tables.rs:1360-1379` already has `printed_line(label)`, built by **T1 in this
same plan** … **Use it**: `printed_line(<the field's label>)` ∋ the quoted instruction,
whitespace-normalized."* And item 6 names the B1 planted defect for that half: *"**per-line quotation** —
move line 28's *"increase the result to the next higher whole number"* onto line 11."*

**What is actually true.** Measured in `crates/btctax-core/src/tax/tables.rs`:

```
1350  #[cfg(test)]
1351  const SCHEDULE_1A_FORM_TEXT: &str = include_str!("fixtures/schedule_1a_2025_form.txt");
1352
1353  #[cfg(test)]
1354  mod schedule_1a_conformance {
...
1360      fn printed_line(label: &str) -> Option<String> {
```

Three independent blocks, any one of which is sufficient:

1. `mod schedule_1a_conformance` is **`#[cfg(test)]`**. When `btctax-core` is built as a *dependency* of
   `xtask`'s test binary, `cfg(test)` is false **for `btctax-core`**, so the module and everything in it
   are not compiled at all. This is not privacy — the item does not exist.
2. `fn printed_line` is **not `pub`**, and neither is the module.
3. Its only data source, `SCHEDULE_1A_FORM_TEXT`, is likewise `#[cfg(test)]`.

`grep -rn "pub fn printed_line" crates/ --include=*.rs` → **no match**; `printed_line` occurs exactly
once as a definition, in that test module.

So the KAT as r5 places it fails to build on its second half, and the implementer meets `E0433` on first
compile with three improvisations available, all of which the plan elsewhere forbids: re-parse the form
text inside `xtask` (CORRECTION 2's own cited doc says *"the fix is not more citation checking"*), copy
the fixture out of the crate (the escaping-`include_str!` trap **item 5 itself invokes** as a reason for
the move), or drop half 2 — in which case item 6's planted defect has nothing to run against and the
line-28-onto-line-11 swap that *inverts the rounding for Parts II/III* passes.

★ This is r4-M-1 in mirror image, created by M-1's own answer. M-1 was *"the KAT cannot see
`label_reader` from `btctax-core`"*; the fix relocated the KAT and did not re-check what it could still
see from the new address. The fold's justification makes the miss visible in its own sentence: it moves
the KAT to keep *"`label_reader`'s repo-root fixture loading out of `btctax-core`, whose tests
deliberately use in-crate fixtures"* — and the in-crate fixture it is describing is `printed_line`'s.

**This is a T2 decision, which is why it is Important and not Minor:** item 5 is the plan's single
statement of where the KAT lives, and the KAT is T2's deliverable and (per item 7) the *only* guard on
the `Production` requirement for the whole of B3. It is not Critical — it fails loudly at compile time
rather than shipping a false green.

**Smallest fix.** Say the KAT is **two tests on either side of the crate line**, and why: membership /
label-set / provenance in `crates/xtask/` (it needs `label_reader`, and `xtask` can see
`btctax_core::tax::schedule_1a::*`, so B-I1's `(label, got.lineN)` tuple form is unaffected); the
**per-line quotation** half in `btctax-core`'s existing `#[cfg(test)] mod schedule_1a_conformance`,
beside `printed_line` and the fixtures it already reads. Both halves keep their own planted defect.
(The alternative — export `printed_line` and its fixture as non-`cfg(test)` `pub` — ships the fixture in
the crate tarball, which the const's own doc comment argues against; prefer the split.)

---

### I-2 — Important. The KAT's expected set has no mechanical source for the four worksheets — F-4's measured blindness, required three times and never given a mechanism

**What the plan says.** The requirement appears three times: §0 line 128, *"Asserted by a
closed-at-both-ends KAT (T2), plus all four worksheets"*; the §T2 heading, *"48 line labels (52 leaves)
**+ four worksheets**"*; and census **F-4** at §T2 lines 246-250 —

> ★★ **THE EXPECTED SET COMES FROM BOTH EXTRACTS, NOT THE FORM ALONE** (census **F-4** — a hole in this
> task's own approach, found by measurement). `grep -c "Keep for Your Records"` on the **form** extract
> is **0**: the four worksheets exist only in the *instructions* extract. So a label census driven off
> the form fixture … **could never red on a worksheet omission.**

The only **mechanism** the plan states is CORRECTION 3 — *"The expected set comes from `label_reader`,
not a fresh parse … It resolves this with two witnesses and asserts 50 / 48 / `["4","22"]`. Assert
against that."* — which r5 item 5 restates and builds on: *"CORRECTION 3 drives the expected set from
`label_reader`."*

**What is actually true.** `label_reader` cannot enumerate the worksheets, structurally and not merely
for want of a fixture. Measured:

| check | result |
|---|---|
| `grep -c "Keep for Your Records" design/forms/extract/f1040s1a--2025.txt` | **0** (F-4 re-confirmed) |
| same, on `i1040gi--2025.txt` | **12** |
| `ls design/forms/geometry/` | `f1040--2024`, `f1040s1a--2025`, `f1040sa--2024`, `f1040sa--2025`, `f6251--2025` — **no `i1040gi--2025.json`** |
| `strings design/forms/2025/i1040gi--2025.pdf \| grep -c AcroForm` | **0** (the form PDF: **2**) |

`label_reader`'s whole design rests on two witnesses, and its own header doc says the asymmetry *is* the
point: **W2 reads AcroForm field geometry** and answers the question the text layer cannot. An
instructions booklet has no AcroForm, so W2 is empty by construction; and W1's discriminator — *"the
label column is the one that accounts for the most printed lines"* — is a whole-document heuristic that
over a 100-plus-page booklet finds body text, not a worksheet's row labels. There is no committed
geometry for it and no way to produce a meaningful one.

So the plan requires the worksheets in the expected set, states the one mechanism that cannot reach
them, and leaves no third source — while CLAUDE.md forbids the obvious escape (*"enumerate the expected
line set **from the form's extracted text**, never from a range or a hand-written list"*). The
consequence is the one F-4 already measured: **dropping a worksheet, or a worksheet line, passes green.**
That is not a marginal class. The four worksheets are where r1's **C-1** net-income ceiling lives — the
plan's own highest-value finding, the one it records the author getting wrong *in the understating
direction* — and §T2 insists they are *"their own transcribed types, not `min()` calls in the emitter."*
Omitting one is the compression defect this project keeps shipping, and per item 7 nothing else is
watching during B3.

★ **The fix needs no new infrastructure — the source is already committed.**
`crates/btctax-core/src/tax/fixtures/schedule_1a_2025_instructions.txt` exists in-crate (used today by
`cite_check.rs:414`) and contains exactly the four worksheets, each anchored by a title line ending
`— Keep for Your Records` at `:427`, `:556`, `:1049`, `:1066`. That is a mechanical enumeration anchor
of the same kind CORRECTION 3 uses for the form.

**Smallest fix.** One paragraph in §T2 reconciling CORRECTION 3 with F-4: the expected set is the
**union** of (a) `label_reader`'s adjudicated 48 form labels and (b) the worksheet rows enumerated from
`schedule_1a_2025_instructions.txt` by its four `— Keep for Your Records` anchors, closed at both ends
like the form half. ★ Note this lands the worksheet half in `btctax-core` too, which is the same
conclusion I-1 reaches independently — both fixtures and `printed_line` are already there.

---

### M-1 — Minor. §T2 item 8 instructs T2 to add two capabilities that landed in `ff839ce7` before the plan was read

Item 8 ends *"Add a year-carrying path"* and *"Widen `exception` the same way."* Both are done at HEAD:
`Coverage::quoting_year` (`crates/btctax-core/src/tax/line_coverage.rs:144`) and
`exception(_value: impl Into<Option<Usd>>)` (`:186`). The fold said so deliberately — *"the **code** fix
is the controller's, deliberately not this fold's"* — so this is ordering, not error. Restate item 8 as
*use* rather than *add*, or it reads as open work at the top of the task.

### M-2 — Minor. Nothing tells T2 to **call** `quoting_year`, and the setter is sticky-forward with no reset

The capability now exists; the plan never says to invoke it. Without
`c.quoting_year("2025")` before the Schedule 1-A rows, every row still carries `DEFAULT_ROW_YEAR`
(`"2024"`), resolves to stem `f1040s1a--2024`, and takes the hard-error branch the fold correctly
describes — deferred to B4, since item 7 records that the checker sees no Schedule 1-A rows during B3.

★ And the sharper half: the setter is **per-row and forward-sticky with no reset**, inside a ~2,900-line
`all()`. Its doc reasons carefully about the backward direction (*"a setter that retroactively rewrote
earlier rows would silently re-attribute quotations"*) and not at all about the forward one. Any 2024
form's rows pushed **after** the Schedule 1-A block silently become 2025 — and `design/forms/extract/`
holds 2025 extracts for `f1040`, `f1040s2`, `f1040s3`, `f1040sa`, `f1040sb`, `f1040sc`, `f1040sd`,
`f1040sse`, `f6251`, `f8283`, `f8949`, `f8959`, `f8960`, `f8995`, so those rows would be checked against
a **real but wrong-year** document, and every sentence unchanged between the two years would still pass.
Minor because no caller exists yet and `make check` is green; worth one sentence in item 8 (place the
Schedule 1-A rows last, or restore `DEFAULT_ROW_YEAR` after them) because the failure mode is a silent
pass, not a red.

### M-3 — Minor. I-4's fix carries requirement **1** to the prior loan and leaves requirement **3** behind, which the fold's own verification names

The r5 fold's own I-4 row states: *"requirement 1 sits at `:44447-44448` and requirement 3 at
`:44450-44452`, and a refinance satisfies **neither** on its own terms."* The remedy sentence then covers
one of the two: *"★ And one sentence in the prompt stating that **for a refinance, requirement 1 binds
the PRIOR loan**."*

Requirement 3 — *"The proceeds from your loan were used to purchase an APV"* — is collected per vehicle
as a YES-condition defaulting to NO (§T3 lines 413-417, 427-429). A refinancing filer answering honestly
answers **NO** (their proceeds repaid a loan), the vehicle drops out, and the whole *Refinanced loan*
carve-in that r4 C-I3 and r5 I-4 exist to build becomes unreachable. A filer answering on the loose
reading the fold itself predicts (*"as the filer reads it the loan really is on the car they
purchased"*) passes a gate that then decides nothing. Both exits are wrong — the same structure the fold
uses to condemn a part-scoped Part I predicate.

**Minor, not Important,** on two grounds the plan itself sets: the direction is fail-closed (it
overstates tax), and §T3 line 498 already grades the exactly-analogous death-exception omission
*"omitting it fails closed, so it is a Minor."* Fix is one clause beside the requirement-1 sentence: for
a refinance, requirements 1 **and 3** are tested against the prior loan, and the new loan's own surviving
condition is the first lien the paragraph restates.

### M-4 — Minor. The line-5 conjunction still refuses the Schedule-C household whose tips are entirely W-2

T3a line 5 now refuses on *claim gate `Some(true)` **AND** `schedule_c.is_some()`*, which is exactly what
r4-I-3 prescribed and I am not re-litigating. Residue: a mining household — btctax's core case, and the
reason `schedule_c.is_some()` alone was rejected — who *also* has a tipped W-2 job is refused, though
their line 5 is as determinately blank as the W-2-only waiter's. Fail-closed, so it buys something the
cited `questions.rs` gate did not; but the plan already collects per-part declarations, and one more
(*"did you receive qualified tips in the course of that trade or business?"*) would decide it without a
new input surface. Record it in the row so B4 does not rediscover it.

### N-1 — Nit. The r5 header says *"Four consecutive rounds"*; the branch record says five

Plan line 47. Cosmetic, and the fold's substantive point is unaffected.

---

## WHAT I VERIFIED AND HOW

**The four r4 Importants — each tested against the primary source, and each closed.**

- **I-1 (completion sources).** Read `design/forms/extract/f1040s1a--2025.txt` in full (113 lines). Every
  address in the fold's new per-part table resolves and says what the table claims: Part I `:15-22`
  (**no Caution printed**), Part II `:24-26`, Part III `:53-55`, Part IV `:73-74`, Part V `:97-98`
  (SSN + joint filing, **no birth date**), Part VI none. `born before January 2, 1961` is on 36a/36b at
  `:104,:107`. Lines 8, 16, 25 and 31 each read *"Enter the amount from line 3"* (`:45`, `:62`, `:88`,
  `:99`), so the fold's Part I blast-radius argument holds. Instructions: `i1040gi--2025.txt:43275-43285`
  is Part I's line-scoped predicate; `:44609-44617` is the *"Fill out Schedule 1-A, Part V, only if:"*
  block whose first bullet is the birth date.
  ★ **And I ran the test the fold did not:** if Part V's Caution omits a condition the instructions
  carry, do II/III/IV do the same? `grep -n "Fill out Schedule 1-A"` returns four blocks — `:43370`
  (II), `:44020` (III), `:44416` (IV), `:44609` (V). Read all four. Part II's bullets sit at
  `:43332-43338` (displaced above the heading by the two-column layout): *received qualified tips in
  2025* and *valid SSN* — both in the form's Caution, which additionally carries the listed-occupation
  and joint-filing bars. Part III's `:44023-44030` are the same pair, again both in the Caution. Part
  IV's block is one sentence equivalent to its Caution. **No completion condition is lost by sourcing
  II/III/IV from the Caution.** The fold's table is right for the reason it gives and also for the reason
  it did not check.
- **I-4 (the refinance precondition).** `i1040gi--2025.txt:44445-44458` are the five requirements, `:44447-44448`
  req 1 and `:44450-44452` req 3, `:44486-44494` the *Refinanced loan* paragraph opening *"If your prior
  loan that had QPVLI is later refinanced"*, `:44459-44470` the change-in-obligor exception whose quoted
  clause *"If a loan met requirements 1 through 5 at the time it was originated by a previous obligor"*
  is verbatim at `:44460-44461`. The new second YES-condition **does** close the named path: a 2023
  vehicle's loan fails req 1 at origination, so the prior-loan condition is NO. The death-exception
  disjunct is **not** a widening — its own quoted text carries the reqs-1-to-5 precondition, and it
  exists because req 2 (*originated by you*) is what the death case fails. The *"testing the new loan's
  origination date is vacuous"* claim is **true conditional on the new condition** (prior origination
  > 2024-12-31 and refinancing > prior origination ⇒ new loan > 2024-12-31), so it cannot be used to
  drop a gate that would otherwise bind. See M-3 for the one requirement left behind.
- **I-3 (the conjunction).** Form `:27-38` are tips *received as an employee*, `:39-42` tips *received in
  the course of a trade or business* — the two independently-claimable things, as the fold states.
  `return_inputs.rs:693-704` is the information-return list, and the fold's own correction that `b_1099`
  had joined it is right. The named backstop is real **and mutation-tested**, which I checked rather than
  assumed: `return_refuse.rs:995` refuses `Some(true)`, and
  `the_scope_attestation_refuses_unanswered_and_affirmed_alike` (`:1961-1981`) pins all three legs —
  `None` → `OtherIncomeUnanswered`, `Some(true)` → `OtherIncomeOutOfScope`, `Some(false)` → proceeds.
  `questions.rs:598` is `live: |_| true` with `neutral: false`. The refusal message enumerates *tips* and
  *an uncaptured business* by name, so it really does cover line 5's 1099-K / 1099-MISC-box-3 half for a
  filer with no Schedule C — the fold's 14b split is sound for the same reason.
- **I-2 (the code half, `ff839ce7`).** Traced the consumer end to end. The checker builds
  `stem = "{form}--{year}"` and reads `design/forms/extract/{stem}.txt`, falling back to
  `crates/btctax-forms/forms/{year}/{form}.map.toml` and otherwise hard-erroring
  (`line_coverage_check.rs:523-560`). `f1040s1a--2025.txt` exists, so a row that has called
  `quoting_year("2025")` resolves and its quotation is checked against the right document — the residue
  is closed. **There is no third missing piece in `line_coverage`:** the census records a line's
  *production*, never its value, so the `exception` widening is pure type-plumbing to let an
  `Option<Usd>` leaf be consumed without `.unwrap_or`, and `Production::Exception` carries a reason with
  no value involvement anywhere in the checker. The missing piece is not in the code at all — it is the
  instruction to *call* it (M-2). Also checked the blast radius of the tuple-struct change: `Coverage(`
  is constructed only at `line_coverage.rs:131,135`; `xtask` touches `LineCoverage` literals
  (`line_coverage_check.rs:1351,1368`), not `Coverage`.

**Scope — nothing widened.** `git show 0819fca5` is 15 hunks in **one** file. No new `T`-numbered task,
no new form, no new checker, no new artifact. Item 8 adds work the r4 review explicitly prescribed
(*"Add both to §T2's task list"*); item 5 answers M-1's *"Say which"*; M-4 is answered in place, which
the review offered as one of its two options (*"Add the item to §T2 **or** answer it in T3a"*). The one
thing that reads like new scope — *"file the unreported-tips half alongside the 1099 surface"* — is a
follow-up entry explicitly deferred out of B3.

**Claims spot-checked rather than accepted.** `printed.rs:459` is `round_dollar(p.half_se_15)`;
`tables.rs:1360` is `fn printed_line`; `return_1040.rs:1790` is
`let schedule_1a_additional = Usd::ZERO;` and `:1809` the `ti_before_qbi` line; `return_refuse.rs:1182`
is `if w2.box8_allocated_tips > Usd::ZERO`. The 4b row's load-bearing premise — *btctax emits no Form
4137 on any return* — checks out: every reference is a map entry recording it **unmodeled**
(`schedule_se.map.toml:51`, `f1040s2.map.toml:107`, `f8959.map.toml:65`), and no 2025 or 2024 map emits
it. Line 4c takes *"the larger of line 4a or line 4b"* (form `:36-38`), so a forced `-0-` on 4b is
fail-closed, as the row says. The 48-label arithmetic re-derived off the extract: 7+12+10+10+8+1 = 48,
and 52 leaves once line 22's three columns land.

**Checked and NOT raised as findings.** The line-22 arity decision (§T2 lines 238-244) is still stated
as a choice for the implementer, but it names both options, both tax directions, and requires a KAT
pinning the choice — that is an adjudicated decision point, not a gap, and r5 did not touch it. Part V's
part-scoped predicate is correct (the instructions skip the whole part, and line 37's per-person gate is
separately modelled via `Person::date_of_death` / `reaches_65_on`). Item 6's four planted defects are
each plantable, including the compile-time ones — that is the pattern `ff839ce7`'s own
`exception_records_a_line_the_form_says_to_skip` uses.

**Taken as given per the brief, not re-derived:** `make check` 2778/12/0 and `fmt` clean; cite-check
45/45; `EXPECTED_LEAF_PATHS` = 93; Part V's Caution content; the two B1 tests in `ff839ce7` being
mutation-verified.

## WHAT I COULD NOT CHECK

- **I did not run the suite, and did not build the KAT.** I-1 is a static claim about Rust's `cfg(test)`
  visibility across a crate boundary plus three measured facts (`#[cfg(test)]` on the module, no `pub`
  on the fn, no `pub fn printed_line` anywhere). I did not construct a throwaway `xtask` test that fails
  to compile, which would be the direct demonstration; if T2 wants one, that is the check.
- **The instructions PDF's structure** — I established it carries no AcroForm by `strings | grep -c
  AcroForm` (0, against 2 for the form) rather than by parsing it. That is sufficient for I-2's claim
  that `label_reader`'s box witness cannot exist there, but I did not attempt to run `label_reader`
  against it, since no geometry fixture exists to run it on.
- **Whether the four worksheets' rows can be enumerated cleanly from the `— Keep for Your Records`
  anchors.** I confirmed the four anchors exist in the in-crate instructions fixture at `:427`, `:556`,
  `:1049`, `:1066`; I did not write the parser, and the row-label shape inside each worksheet may need a
  different discriminator than the form's margin column. That is T2's work, and it is the work I-2 says
  the plan must name.
- **B4's emitter behaviour**, `T7`'s oracle census (needs `OTS_DIR` and the `.venv`), and the SPEC —
  all out of scope and untouched by this fold.
- **`cite-check`'s own blind spot** (`FOLLOWUPS.md` FR-21) still means the 45/45 result is weaker
  evidence for quotations written as plain `"…"` than for blockquotes. I hand-verified the fold's
  load-bearing new quotations — the refinance paragraph, the death-exception clause, Part V's
  *"Fill out … only if"* block, and Part I's predicate — against `i1040gi--2025.txt` directly.
