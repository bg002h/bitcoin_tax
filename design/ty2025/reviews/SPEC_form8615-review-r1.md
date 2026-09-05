# SPEC_form8615_kiddie_tax.md — independent review r1

**Artifact:** `design/ty2025/SPEC_form8615_kiddie_tax.md` (714 lines), unchanged since `e9126e89`.
**Reviewed against:** `HEAD = 29e47a0b` on `feat/schedule-1a-ty2025`.
**Reviewer:** independent (author ≠ reviewer). Nothing was modified but this file; nothing was committed.

---

## VERDICT

**1 Critical / 4 Important / 5 Minor / 3 Nit — DO NOT BUILD YET.**

The design is right. The gate, the fail-closed ladder, the class-(A)-semantics-in-a-class-(B)-registry
placement and the direction proofs all hold, and I could not find a path by which the specified fix
under-refuses. **The independence trap is navigated correctly everywhere it appears in normative text**
(§2, §4 doc comments, §4.1 prompts, §5.4, §6, §8) — the one soft spot is filer-facing *help* text, and
it is a symptom of the Critical rather than an independent error.

The Critical is that **the spec's sourcing of record is now false.** `design/forms/extract/i8615--2025.txt`
(712 lines) and `design/forms/extract/f8615--2025.txt` (59 lines) exist at HEAD. They were archived by
`29e47a0b`, the commit immediately *after* the spec — so §1's claim was true when written and is false
now. That matters because i8615 is the document that defines *support*, *earned income* and *unearned
income* **for Form 8615 specifically**, and because it states the FR-29 holding outright in one sentence
the spec never quotes. This is a re-sourcing pass, not a redesign.

---

## FINDINGS

### C-1 (Critical) — §1 "Sourcing of record — READ THIS FIRST" is false at HEAD, and the missing document is the one that defines the terms this spec paraphrases

`design/ty2025/SPEC_form8615_kiddie_tax.md:19-20`:

> "There is no separate `i8615` in the archive for this cycle. **`i1040gi` carries the whole test**, in
> the file that already sources Schedule 1-A."

Both statements are now wrong:

```
$ ls design/forms/extract/ | grep 8615
f8615--2025.txt
i8615--2025.txt
$ git log --oneline -1 -- design/forms/extract/i8615--2025.txt
29e47a0b archive(f8615/i8615): the Form 8615 instructions were NEVER ARCHIVED — and they answer the owner's question
$ git merge-base --is-ancestor e9126e89 29e47a0b && echo "extracts landed AFTER the spec"
extracts landed AFTER the spec
```

I record that ordering deliberately: **the spec was accurate when authored.** But the artifact under
review is the one at HEAD, and at HEAD its §1 misdirects every downstream transcription. Concretely,
what i8615 carries that the spec invented or borrowed:

**(a) The sentence that settles FR-29, stated affirmatively.** `design/forms/extract/i8615--2025.txt:65-68`:

> "For these rules, the term “child” includes a legally adopted child and a stepchild. **These rules
> apply whether or not the child is a dependent.** These rules don’t apply if neither of the child’s
> parents were living at the end of the year."

The entire spec — and the corrected doc comment already shipped at
`crates/btctax-core/src/tax/return_refuse.rs:265-272` — *derives* this from dependency's **absence**
from i1040gi's five conditions. The IRS asserts it. In a repo whose standing rule is *"The answer is in
the manual"*, a spec that reasons from an absence when the source states the presence has stopped
reading and started inventing. This sentence belongs verbatim in `RefuseReason::KiddieTax`'s doc comment
and in the condition-3 help, and it is the single best inoculation against FR-29 recurring.

**(b) The definition of *support*, which §4.1 paraphrases.** `i8615--2025.txt:69-73` + `:4-6`:

> "**Support.** Your support includes all amounts spent to provide the child with food, lodging,
> clothing, education, medical and dental care, recreation, transportation, and similar necessities. To
> figure your child’s support, count support provided by you, your child, and others. However, **a
> scholarship received by your child isn’t considered support if your child is a full-time student.**
> For details, see Pub. 501…"

The spec's help (`SPEC_form8615_kiddie_tax.md:217-218`) says instead: *"'Support' is the whole cost of
keeping you for the year"* — and drops the scholarship carve-out entirely. The carve-out is scoped to
**full-time students**, i.e. exactly condition 3(c)'s population.

**(c) The definition of *earned income*, which §4.1 borrows from the wrong chart.**
`i8615--2025.txt:139-140`:

> "**Earned income.** Earned income includes wages, tips, and other payments received for personal
> services performed."

…followed by the sole-proprietor/partner 30 %-of-net-profits allowance (`:141-152`) and the
qualified-disability-trust distribution (`:157-`). The spec's help says *"wages, salaries, tips,
professional fees"* — which is **Chart B's** list at `i1040gi--2025.txt:695`, a definition written for
the *filing-requirement* chart, not for §1(g). The two documents genuinely disagree: Chart B counts
*"taxable scholarship and fellowship grants"* as **earned**; i8615 counts *"taxable scholarship and
fellowship grants not reported on Form W-2"* as **unearned** (`i8615--2025.txt:31-32`). Importing one
scope's definition into the other's test is the compression `CLAUDE.md` forbids, and here the two
sources point opposite ways on the same category.

**(d) The unearned-income enumeration §5.2 re-blesses without checking.** `i8615--2025.txt:27-36`:

> "Unearned income is generally all income other than salaries, wages, and other amounts received as pay
> for work actually performed (earned income). It includes taxable interest, dividends, capital gains
> (including capital gain distributions), **rents, royalties, pension and annuity income, taxable
> scholarship and fellowship grants not reported on Form W-2**, unemployment compensation, **alimony**,
> the taxable part of social security and pension payments, and **income (other than earned income)
> received as the beneficiary of a trust**."

`return_1040.rs:990-995`'s component sum omits every bolded category. §5.2 nevertheless states the sum
"can only be **TOO HIGH** ⇒ it can only OVER-refuse" and instructs *"Do not 'fix' that without
preserving the direction."* **The claim survives, but not for the reason given.** It survives because
`QuestionId::OtherOutOfScopeIncome` (`questions.rs:546-563`) names *"a PENSION, ANNUITY or IRA
DISTRIBUTION … SOCIAL SECURITY … rent or royalties, a farm, a partnership, S corporation, estate or
trust (any Schedule K-1) … alimony"* and a `yes` hard-refuses `OtherIncomeOutOfScope`
(`return_refuse.rs:997`). So the omitted categories are unreachable — via a refusal in a different
module that **this spec never cites**. That is a load-bearing dependency held by luck, in the exact
shape §G-9 warns about. It must be written down and pinned, or the next scope widening silently turns
an over-refusal into an under-refusal.

**(e) Two smaller consequences.** i8615's January-1 chart (`:12-24`) *states* what §5.1 offers as its own
"equivalence proof", including the footnote **`*** Don’t use Form 8615 for this child`** for a
January-1-2002 birth — the 24-or-older suppression, printed by the IRS. And i8615 documents the **Form
8814 parental election** (`:33-41`): *"If the parent makes this election, the child won’t have to file a
return or Form 8615."* That is a real exit for the filer R-3 refuses, and §11 does not mention it.

**Fix:** add i8615/f8615 to §1's authority table; requote §4.1's help from `i8615:65-73` and `:139-140`;
add i8615 to `schedule_1a_docs()` in P-1; add the *"whether or not the child is a dependent"* sentence
to R-3's doc comment; state (and test) the `OtherOutOfScopeIncome` dependency behind §5.2's direction
claim; note Form 8814 in §11. None of this changes the gate.

---

### I-1 (Important) — the condition-3 *help* is the one place the spec explains the support test in its own words, and it is the only text that brushes the trap

`SPEC_form8615_kiddie_tax.md:217-219`. Covered in C-1(b)/(c); flagged separately because it is
**filer-facing** and because it sits on the exact distinction this review was told to hunt.

Direction is safe in every branch I could construct — the help under-counts what is earned (drops the
30 % allowance, the disability-trust distribution, scholarships) and over-counts support (drops the
scholarship carve-out), and both errors push a filer toward answering **YES** to condition 3, i.e.
toward refusal. So this is an over-refusal, not an understatement. But that is an accident of which
terms were dropped, not a property of the design, and the spec asserts no direction argument for its
help text at all.

The help's redeeming sentence is correct and should survive verbatim into the build:

> "Income from investments, crypto or a trust is not earned income."

---

### I-2 (Important) — §4.1's index-hazard claim is false, and §9 G9 omits the only file where the hazard lives

`SPEC_form8615_kiddie_tax.md:189-192`:

> "★ Note the `decl_tristate!` INDEX hazard recorded at `questions.rs:99-102` binds `FORM_QUESTIONS`,
> **not** this registry: `skippable_to_field` matches on `SkippableId` by name, so a mid-array insert
> here mis-orders a UI section rather than silently repointing a question."

`crates/btctax-input-form/src/spec/registries.rs:67-95` is `skippable_tristate!`, and every field it
builds reads `SKIPPABLE_QUESTIONS[$idx]` for its `label`, `help`, `live`, `get` **and** `set`, while
taking its `id` from a separate `$fid` argument. Call sites are literal indices:

```
registries.rs:254: skippable_tristate!(0,  FieldId::BlindTaxpayer, …)
…
registries.rs:355: skippable_tristate!(15, FieldId::CharitableCwaObtained, …)
```

A mid-array insert therefore **does** repoint every later question's prompt, liveness and accessors —
the identical hazard, in a second registry. It is not *silent* (the delegation loop at
`crates/btctax-input-form/src/spec/mod.rs:341-360` sets through the registry entry and reads back
through the `Field`, so a mismatch reds), but "not silent" is not what the spec says. `skippable_to_field`
being name-keyed is true and irrelevant: it is not the protection.

The spec's *action* — append at the end — is correct and safe. Its *reason* is a false reassurance
sitting where a future maintainer will read it before inserting in the middle. By the spec's own
standard three sections later (§8: *"Leaving the warning in place after the fix would be its own defect —
a doc telling a future maintainer the gate is wrong when it is right"*), that is a defect.

**Compounding:** §9 G9 lists `questions.rs:1490-1495`, `spec/mod.rs:305-325`, and `coverage.rs:435` —
and never names `registries.rs`. Two `FieldId` variants and two `skippable_to_field` arms are
compile-forced (the match at `registries.rs:457-475` is exhaustive over `SkippableId`), but **the two
`skippable_tristate!(16, …)` / `(17, …)` call sites are not** — any index 0–17 compiles. That is the one
piece of this change's bookkeeping that neither the compiler nor G9 catches.

**Fix:** correct the sentence to say the hazard is present but test-caught, and add `registries.rs`
(two `Field`s at indices 16/17, two `FieldId` variants, two match arms) to G9.

---

### I-3 (Important) — G7's clause table cannot pass as written; "normalised" is never defined, and the natural repair is to loosen the checker

`SPEC_form8615_kiddie_tax.md:580-590` requires each clause to appear verbatim (normalised) in **both**
the extract and the prompt. Take the clauses against the prompt the spec itself writes at `:200-204`:

| clause (§9 G7) | in extract? | in the spec's own prompt? |
|---|---|---|
| `"under age 18"` | `i1040gi:3933` has **"Under** age 18" | prompt has "(a) **under** age 18" |
| `"Age 18"` | `:3934` has **"Age** 18" | prompt has "(b) **age** 18 and" |
| `"didn’t have earned income that was more than half of your support"` | `:3935-3936`, across a line break | yes |
| `"A full-time student at least age 19 but under age 24"` | `:3937-3938`, across a line break | prompt has "**a** full-time student…" |
| `"At least one of your parents was alive"` | `:3941-3942`, across a line break | yes |

Three of five clauses differ **in case** from one side or the other, and three span newlines in the
extract. So G7 passes only if normalisation folds case *and* collapses newlines — and the spec says
neither. An implementer who transcribes G7 literally gets a red check on day one, and the cheapest
repair in front of them is to edit the clause list, which guts the instrument. That is the F2/F4 shape
harness rule **B1** exists to prevent.

Note the B1 pairing (`prompt_check_rejects_a_paraphrased_prompt_and_accepts_the_real_one`) still reds
under case-folding, so defining normalisation as case-insensitive + whitespace-collapsing does **not**
make the checker performatively satisfiable. The spec just has to say so.

**Fix:** one sentence in G7 defining normalisation, and a note that clauses are matched case-insensitively
against a whitespace-collapsed extract.

---

### I-4 (Important) — R-3's filer-facing text asserts something §5.4 knows is false for an enumerated population

`SPEC_form8615_kiddie_tax.md:513-518` (R-3's detail):

> "you meet **all five** of Form 8615's conditions for {year}: more than ${threshold} of unearned income,
> **a filing requirement**, … Complete Form 8615 by hand."

§5.4 assumes condition 2 and names two populations for whom it is false — a non-dependent below Chart A
(`i1040gi:634-639`, `$15,750` single under 65) and a blind dependent under 24 (`:705`, `$3,350`). For
those filers the refusal **states as fact** a condition btctax never established, and then instructs
them to complete a form they are not required to file.

The spec pins this branch — but only in a **test assertion message** (G6, `:570-578`). The falsehood the
filer actually reads is unpinned. This is the same shape as the `QbiAboveThreshold` anchor defect
recorded at `crates/btctax-input-form/src/attribute.rs:224-231` — *"an anchor saying the refusal has no
form field is a FALSEHOOD that leaves the filer with nowhere to go. It was one, and a green test pinned
it."*

Aggravating: `KiddieTax` keeps its `NotInForm` anchor (`attribute.rs:221-223`) and is by design a
refusal **no input can clear**. So this population has no exit at all — btctax simply cannot produce
their return.

**Fix (cheap, and independent of OQ-2):** reword R-3 to disclose the assumption — *"btctax assumes you are
required to file a return (Form 8615's condition 2); if your gross income is below the Chart A/Chart B
threshold for your status you may not be, in which case Form 8615 is not required"* — and add the
assertion to G6. This converts a falsehood into a disclosed assumption at zero design cost.

---

### M-1 (Minor) — G1's test name is not a legal Rust identifier

`SPEC_form8615_kiddie_tax.md:537`: `form8615_screens_a_filer_who_is_nobody's_dependent`. Machine-checked:

```
$ rustc --edition 2021 …
error: prefix `form8615_screens_a_filer_who_is_nobody` is unknown
  |
1 | fn form8615_screens_a_filer_who_is_nobody's_dependent() {}
  |    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unknown prefix
  = note: prefixed identifiers and literals are reserved since Rust 2021
```

Suggest `form8615_screens_a_filer_who_is_nobodys_dependent`.

### M-2 (Minor) — §4.2's "Never the reverse" holds only for `Default`, not for a hand-built `ReturnInputs`

`:261-267` argues the `ri.tax_year` / gate-`year` divergence is one-directional because
`ReturnInputs::default()` is `tax_year: 0` (`return_inputs.rs:1081`) and a smaller year yields a smaller
age ⇒ the question stays live. True for `0`. But a hand-built fixture may set **any** `tax_year`, and
`tax_year > year` computes an inflated age ⇒ `provably_24_or_older` true ⇒ the question is **not live**
while the gate demands it — the brick §3.1 exists to avoid. Production-unreachable (the storage boundary
stamps on read at `crates/btctax-cli/src/return_inputs.rs:75` and refuses a disagreement at `:103`), so
this is scoped to hand-built inputs. Narrow the claim, or have G8 assert the invariant rather than the
direction.

### M-3 (Minor) — the condition-4 help states a false rule to the filer

`:229-231`: *"It is asked only when you answered YES to condition 3, because **the two together are what
send you to Form 8615**."* All five conditions do. And condition 4's liveness (`:242-244`) does not
include condition 1, so a filer with zero unearned income is asked it and told this.

### M-4 (Minor) — §7's fixture migration does not bound the date of birth away from the §63(f) age-65 band

`:458-459`: *"give them a date of birth, which both fixes them and exercises §5.1's suppression path."*
A DOB showing 65+ at year end also switches on the §63(f) aged addition (`return_1040.rs:89-91`,
`:110-122`), moving the standard deduction on the four named oracle households
(`single_w2_plus_crypto_ltcg`, `single_qdcgt_both_slices`, `single_short_term_crypto_gain`,
`single_miner_qbi_limited_by_net_capital_gain`). The goldens red loudly, so this is not silent — but
"regenerate the golden" is the tempting repair, and this repo has a starred rule against exactly that.
Say: choose a DOB in `[year − 64, year − 24]`.

### M-5 (Minor) — §4.1's "only permitted departure … three times" understates the departures

`:206-208` claims the sole change is *"at the end of 2025"* → *"at the end of the tax year"*, three
times, and that *"Every other clause is byte-identical."* Against `i1040gi:3932-3940` the prompt also
(a) **hoists** the qualifier to a single leading occurrence rather than three, (b) recases four clause
openings (`You were` → `were you`, `Under` → `under`, `Age` → `age`, `A full-time` → `a full-time`), and
(c) appends a sentence that is not in the form: *"Answer YES if any one of (a), (b) or (c) is true."*
(c) is correct and worth keeping; the claim just needs to admit it.

### N-1 (Nit) — G9 cites one `93`; there are two

`crates/btctax-input-form/src/spec/coverage.rs:429` (`field_count, 93`) and `:434` (`covered.len(), 93`).
Both move to 95.

### N-2 (Nit) — the registry-count test's name and message also need editing

`questions.rs:1490` `fn skippable_registry_is_separate_and_has_five_entries_with_correct_liveness` (already
stale at 16) and the enumerating message at `:1495-1496`. G9 names neither.

### N-3 (Nit) — no QSS vector

§5.3 calls QSS out by name (*"QSS is not a joint return and is therefore not exempted"*), and G4 pins MFJ
only. G8's grid is `{Single, MFJ, MFS}`. Add a QSS row somewhere.

---

## THE INDEPENDENCE TRAP

The brief's trap: *"lawfully independent"* / *"not a dependent"* / *"self-supporting"* is **not** Form
8615's condition-3 test. The test is whether **earned** income was more than half of the filer's
support. A student living entirely on crypto gains is nobody's dependent, is independent in every
ordinary sense, and is still inside §1(g). Every place the spec touches the distinction, and whether it
holds:

| # | site | what it says | holds? |
|---|---|---|---|
| 1 | §2 `:79-84` | *"They are not claimable as anyone's dependent … but Form 8615 asks about **earned** income … and unearned support does not satisfy it."* | **YES** — states the trap explicitly and correctly |
| 2 | §4 doc comment `:157-163` | condition 3 transcribed verbatim from `i1040gi:3932-3940` | **YES** — no independence language |
| 3 | §4 doc comment `:175-177` | condition 4 verbatim | **YES** |
| 4 | §4.1 prompt `:200-204` | carries *"didn’t have earned income that was more than half of your support"* twice | **YES** (see M-5 for wording drift, not meaning drift) |
| 5 | §4.1 **help** `:213-219` | *"the test is whether your EARNED income … covered more than half of it. Income from investments, crypto or a trust is not earned income."* | **YES on the trap** — the crypto sentence is the anti-trap sentence and it is right. **But the definitions of *earned income* and *support* are paraphrased from the wrong document** (C-1/I-1); direction is over-refusal |
| 6 | §4.1 condition-4 help `:229-231` | no independence language | YES on the trap (see M-3) |
| 7 | §5.1 `:273-310` | age arithmetic only | **YES** — nothing about dependency |
| 8 | §5.4 `:346-350` | *"a **non-dependent** below Chart A's threshold"* / *"a **blind dependent** under 24"* | **YES, and this is the near-miss** — dependency is used *only* to pick between Chart A and Chart B, which is exactly what `i1040gi:691-692` says it selects (*"If your parent (or someone else) can claim you as a dependent, use this chart"*). It never leaks into conditions 1/3/4/5 |
| 9 | §6 `:360-363` | *"**The dependency flag is not read.** Its deletion is the fix; because the identifier disappears, no refactor can quietly restore the old behaviour"* | **YES** — and the structural argument is the right one |
| 10 | §6.1 `:390-402` | six-row table, no dependency column | **YES** |
| 11 | §8 R-1 `:479-486` | condition 3 transcribed with the year | **YES** |
| 12 | §8 R-3 `:513-518` | *"the condition-3 age **and earned-income-support** test"* | **YES** — precisely worded (its defect is I-4, about condition **2**, not the trap) |
| 13 | §9 G1 `:537-545` | `…screens_a_filer_who_is_nobody's_dependent`, `can_be_claimed_as_dependent_taxpayer = Some(false)` ⇒ `KiddieTax` | **YES** — the test is the inversion of `return_1040.rs:4473-4475` and pins exactly the trap (see M-1 for the identifier) |
| 14 | §12 OQ-4 `:711-714` | three-leaf alternative: "age band, full-time student, support test" | **YES** |

**Verdict on the trap: it holds in all 14 sites.** No prompt, doc comment, field name, refusal or test
in this spec collapses independence into the §1(g) support test. Site 5 is the only weak one, and its
weakness is a *sourcing* failure (definitions taken from Chart B, whose scope is the filing-requirement
test) rather than a conceptual one — and it errs toward refusing.

★ **The irony worth recording:** the sentence that would make site 5 unfalsifiable — *"These rules apply
whether or not the child is a dependent"* (`i8615--2025.txt:66-67`) — has been sitting in the archive
since the commit after this spec was written, and no site quotes it. The best defence against FR-29
recurring is one line of transcription the spec does not yet know it can do.

---

## WHAT I VERIFIED AND HOW

**Primary sources — every quotation, by line.** `sed -n` over `design/forms/extract/i1040gi--2025.txt`
confirmed the five conditions at `:3927-3944`, the January-1 convention at `:3945-3951`, Chart A Single
at `:634-639` (`$15,750` / `17,750`), Chart B single-dependent "No" at `:699` (`$1,350`) and "Yes" at
`:705` (`$3,350 ($5,350 if 65 or older and blind)`), and Chart C's seven items at `:732-`. Every §1.1
quotation in the spec is byte-accurate. I also read `i1040gi--2024.txt:3559-3581` (the year btctax can
actually file): the condition wording is identical, only the figures and years move — which validates
§4.1's year-free prompt design.

**The Critical.** `ls design/forms/extract/`, then `git log -1` and `git merge-base --is-ancestor` to
establish that `i8615--2025.txt` / `f8615--2025.txt` landed at `29e47a0b`, *after* `e9126e89`, and
`git diff --stat e9126e89 HEAD -- <spec>` (empty) to confirm the spec has not been updated since. Read
i8615 `:4-6`, `:12-24`, `:26-41`, `:43-75`, `:95-180` for the definitions and the dependency sentence.

**The code, every `file:line` the spec cites.** `return_1040.rs:895-1006` (`screen_compute_dependent`,
the `:989` gate, the `:980-988` direction comment, the `:990-995` component sum), `:110-122`
(`reaches_65_on`, and the leap-day reason §5.1 contrasts against), `:2676-2690` (the patron precedent —
real and exactly as described), `:4441-4489` (the FR-29 test; the assertion is at `:4473-4475` as
claimed). `questions.rs:53-54, 99-102, 289, 309-310, 344-345, 800-801, 837-839, 952, 1003-1013,
1490-1495, 1587-1598` — all confirmed. `return_refuse.rs:265-272` (the corrected doc comment),
`:823-826` (the `FORM_QUESTIONS` unanswered loop). `classifier.rs:49-51, 265-281, 498-509`.
`attribute.rs:68-73, 221-223, 427-450`. `tables.rs:469-470`, `tax_tables.rs:134` (`dec!(2600)`),
`:814-820`. `coverage.rs:429/434`, `smoke.rs:164-173, 195-198, 204`. `cite_check.rs:404-416`.
Citation drift is ≤2 lines anywhere I checked and never changes the meaning.

**Attack 2 — do class-(A) semantics survive in `SKIPPABLE_QUESTIONS`?** **Yes, and structurally.** I
traced every path that could write an answer the filer did not give:
- `SkippableQuestion` has **no `neutral` field** — the polarity `answer_all_live_declarations` uses does not exist here.
- `testonly::answer_all_live_declarations` (`testonly.rs:34-40`) iterates `FORM_QUESTIONS` **only**, so §7's "do not add the answers to the shared builders" is enforced by construction, not by discipline.
- `btctax income answer` (`cmd/answer.rs:193-200`): *"A bare Enter KEEPS whatever is on file (may be `None` ⇒ skip); only y/n sets a value"*.
- The input-form `Field::set` (`registries.rs:85-92`) rejects `TriState(None)` with `SetError::WrongKind`; the un-answer path is the separate `clear`, which writes `None`.
- `Durability::PerYear` is enforced by `questions.rs:1587-1598`, which asserts `durable_skips == [DobTaxpayer, DobSpouse]` — a `Durable` new entry reds.
- The mandatory half is a compute-time refusal, exactly as `ScheduleCIsCooperativePatron` / `ScheduleCIsSstb` already do (`attribute.rs:68-73` ↔ `return_1040.rs:2676-2690`).
Nothing in the class-(B) machinery can answer for the filer. §3.1's argument holds.

**Attack 3 — condition 2 assumed TRUE.** Assuming a conjunct true enlarges the refusal set
(`c1∧c2∧c3∧c4∧c5 ⊆ c1∧c3∧c4∧c5`), and condition 2 never appears in the §6 ladder, so it can only
over-refuse. I could construct no under-refusing branch. ★ But I found the decisive argument **against**
a partial fix, which feeds OQ-2 below: Charts A/B/C are a **disjunction** (`i1040gi:733`: *"You must file
a return if **any** of the conditions below apply"*), so computing A and B *without* C would make
condition 2 false where it is true — flipping the direction to under-refusal.

**Attack 4 — the six-row fail-closed table.** Row by row: `DOB≥24` proceeds on a **proof** from the
filer's own volunteered DOB (§5.1's equivalence, and it is the IRS's own — `i8615:20-24` footnote `***`);
`Some(false)` on either condition proceeds on the **filer's answer**; both `None` arms refuse. No row
proceeds through an absent answer. I also checked the two ladder-step-1 proceeds the table does not
cover: `!c1` is computed conservatively high (so under-claimed), and `!c5` reads `ri.filing_status`,
which is a non-`Option` enum defaulting to `Single` (`return_inputs.rs:1091`) — i.e. defaulting to
**not** joint, the fail-closed direction. Both licensed.

**Attack 5 — the 18 newly-failing tests, two spot-checks (not re-run; reasoned from source).**
- `btctax-core::…::business_interest_income_refuses` (`return_1040.rs:4397-4406`): its second half is `state_income(vec![income(IncomeKind::Interest, false, dec!(5000))])` with `single()`, and `single()` (`:4386-4393`) is `testonly::not_a_dependent()` — `Some(false)` dependency, **no DOB**, `FilingStatus::Single`. $5,000 of hobby interest lands in `crypto.nonbusiness_ordinary` ⇒ `unearned = $5,000 > $2,600`; not MFJ, not provably 24+, condition 3 `None` ⇒ R-1. The `assert_eq!(screened(&single(), &hobby), None)` at `:4405` **newly fails.** Confirmed, and §10's diagnosis (*"the `$5,000` hobby interest half"*) is exactly right.
- `btctax-cli::promote_cli::export_full_return_writes_form_8275_txt_by_name` (`promote_cli.rs:1025-1037`): plants `plant_full_return_ri` (`:950-969`), which is `not_a_dependent()` + `answer_all_live_declarations` — Single, **no DOB**, and (per the helper) no skippable answers. The vault carries `t14_sell()` (`:688-699`), a $20,000 disposal draining a declared tranche ⇒ capital gain far over the threshold ⇒ R-1 ⇒ `export_irs_pdf`'s `.expect(...)` **newly fails.** Confirmed.
Both are consistent with §10's "shape of the 15 CLI failures" and with §7's migration remedy.

**Machine checks I ran rather than reasoned about.** `rustc` on the G1 identifier (M-1);
`git ls-tree e9126e89` / `git merge-base --is-ancestor` for the C-1 ordering; `grep -n` for the two `93`
asserts (N-1) and for the fifteen `skippable_tristate!` / `skippable_date!` literal indices (I-2). I did
not hand-count anything a command could count.

---

## WHAT I COULD NOT CHECK

1. **I did not run the suite.** §10's headline numbers (2779 / 2775+4 baseline → 2757+22 patched, and
   the 18-test delta) are taken as given per the brief; I verified two named members by source and both
   hold. The `#[ignore]`d ~104-household corpus (`smoke.rs:204`) is out of the measurement by the spec's
   own admission (P-2) and I did not run it either.
2. **`cite_check`'s actual extraction surface.** I did not read enough of `crates/xtask/src/cite_check.rs`
   to say which spans it scans, so I cannot confirm how much of this spec P-1 would actually cover.
   Given `CLAUDE.md`'s note that it once read only `*"…"*` spans, and given that §4's doc comments and
   §4.1's prompts live in fenced code blocks, P-1 may cover less than §1's wording implies. Worth one
   `grep` before the plan is written; G7 exists precisely because the gap is real.
3. **Whether Chart B's earned-income definition is *authoritative* for §1(g)**, versus i8615's. I
   established that the two documents differ on scholarships and that the spec used the wrong one for
   this test; adjudicating which controls in a contested case is a tax-judgment question (Pub. 501 and
   §1(g)(4)(A)(ii)(I) are the next sources) and is outside a spec review.
4. **The `OtherOutOfScopeIncome` closure of C-1(d) is my reasoning, not a test.** I confirmed the prompt
   names the categories and that a `yes` refuses; I did not verify by execution that *every* i8615
   unearned category is unreachable in a computable btctax return. That is exactly why it should become
   a written, tested dependency of §5.2's direction claim.
5. **No oracle can adjudicate any of this** — the spec's §1 statement to that effect is correct in
   substance and I did not attempt to; `grep 8615 scripts/oracle/*.py` being empty is a claim I did not
   re-run, but it is consistent with everything else I read.

---

## RECOMMENDATIONS ON THE OPEN QUESTIONS

**OQ-2 (the one the brief asked for) — recommendation: DO NOT transcribe Charts A/B/C in this cycle.
Keep condition 2 assumed TRUE, and fix I-4 instead.** Four reasons, in ascending weight:

1. The assumption's direction is over-refusal, which is the side this repo prefers by rule.
2. Both named populations are people **not required to file a return at all** — the smallest possible
   blast radius for an over-refusal.
3. **Chart B keys on dependency** (`i1040gi:691-692`). Reintroducing a read of
   `can_be_claimed_as_dependent_taxpayer` into the Form 8615 path *in the same cycle that deletes it* is
   the most plausible mechanism by which FR-29 recurs. If it is ever done, the read must be scoped to
   condition 2 alone with a test that reds if it reaches conditions 1/3/4/5.
4. ★ **Decisive:** Charts A, B and C are a **disjunction** — `i1040gi:733`, *"You must file a return if
   **any** of the conditions below apply for 2025."* Chart C's seven items include *"net earnings from
   self-employment of at least $400"*, *"wages of $108.28 or more from a church…"*, §965 inclusions,
   Archer/Medicare-Advantage MSA distributions and a transferred clean-vehicle credit — several of which
   btctax cannot answer without new collection. Transcribing A and B while leaving C unmodelled would
   compute condition 2 as **false** for filers whom Chart C requires to file, i.e. it would
   **under-refuse**. A partial transcription is therefore not a smaller version of this improvement; it
   is a direction reversal. If OQ-2 is ever answered "collect it", the C limb must either be collected
   in full or kept assumed-true — and the latter means the assumption never actually goes away.

The cheap half should still be done now, and it is independent of the answer: **fix I-4** so the refused
filer is told what btctax assumed and can check Chart A/B themselves.

**OQ-1 — recommendation: the reason string is enough for now; revisit at the fifth leaf.** `Class::NoTaxDirection`
is documented as *"a lawful silent default"* (`classifier.rs:50-51`), which these leaves are not — but
`qbi_w2_wages` and `qbi_ubia` already stretch it identically (`classifier.rs:498-509`) and their reason
strings carry the real semantics. A new variant is cheap (an exhaustive match reds), so this is a
judgment call, not a defect; the argument for acting now is that the *name* is what a future reader
greps for. If it is added, add it for all four leaves in one pass, not two.

**OQ-3 — accept the split.** A year-free `prompt` with a `format!`ed year in the refusal detail matches
every existing registry entry, and I confirmed the condition wording is year-invariant across
`i1040gi--2024.txt:3559-3576` and `i1040gi--2025.txt:3927-3944` (only the figures and years move). No
change warranted.

**OQ-4 — confirm the single leaf.** The form states one numbered condition with three alternatives and
asks for the disjunction; splitting it would make btctax re-derive what the form writes out. ★ And note
that C-1 strengthens this: `i8615:53-58`'s footnotes to the January-1 chart show the IRS itself resolving
the limbs *together* per birth date. One leaf, as specced.
