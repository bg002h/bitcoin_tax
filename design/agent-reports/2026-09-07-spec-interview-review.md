# REVIEW — `design/SPEC_interview.md` DRAFT r1 (Fable, lens 3 of 3; the ONE round under owner ruling S6)

**Reviewer:** independent, adversarial, read-only, in its own worktree detached at `127cb75c` (HEAD of
`main`, the spec's own commit). No subagents; nothing edited; no cargo run (every claim below is a
`sed`/`grep` against the tree or a reading of the cited extract).
**Brief:** `design/agent-reports/BRIEF-review-spec-interview.md`. **Read in order:** `CLAUDE.md`; the recon
`design/agent-reports/2026-09-07-interview-recon.md` (cited *recon §X*, not re-measured); the brainstorm
`design/BRAINSTORM_interview.md`; the spec in full (869 lines, §1–§10); then the seams the spec names.

## The one question, answered

**Not yet — but close, and for reasons the spec can fix in one fold.** Built exactly as written, a filer with
a shoebox and four exports would reach a return with **no defaulted amount** (R1/R3/R4 hold: every dollar is a
box on a declared document or a filer's-records figure; every gate is `Option<bool>` the classifier polices)
and with **no defaulted gate** (R14). Where it fails the question is in three places:

1. **"Correct for what it covers or refused with a named exit" is unmet in the understatement direction.**
   The instrument that is supposed to make §2.2's *"never under-filed silently"* mechanical (R2.2, the
   `covered_by` join) accepts an `Advisory` on an income line and checks only that the named variant exists.
   An income line "covered" by a forgone-benefit advisory, or by the residual attestation's *"anything else it
   never asked about"*, is green and silent (**C1**). Three concrete instances the form itself names
   (**I1**, **I3**), and one the documents name (**I2**).
2. **The dependents flowchart is transcribed with two edges the instruction states and the spec drops** — the
   age test on a missing DOB (**I4**) and the born-or-died / temporary-absence exception (**I5**). J-11's own
   newborn is misrouted by the spec's own gate.
3. **"Re-ask only what changed a year later" is a mechanism with a kill and no task** (**I11**), and its two
   data-model guarantees — the prompt hash and the per-row answer key — are stored without a reader (**I9**)
   and keyed by a mutable index (**I10**).

Everything else is Important-by-shape but small: a wrong predicate (I6), an assertion left untested for a
filing status the spec fixes for its neighbour (I7), one liveness key that refuses a standard-deduction filer
(I8), an oracle projection blind to dependents (I12), one task whose kill names a map with no cells (I13),
and an owner question that omits most of the refusal families it exists to surface (I14).

**Cites.** Every `file:line` the spec gives was resolved against `127cb75c`. All of them point at what the spec
says they do; four are off by one line and are listed as Nits (N4). The extract cites (`i1040gi--2025.txt`,
`i1040sca`, `i1040sd`, `i1040sb`, `f1040--2025`) are exact — I checked the ones I doubted and was wrong to doubt
them.

---

## Per lens — what I checked, and the verdict

| lens | checked | verdict |
|---|---|---|
| **Testimony** | every new field in §5 against R14's table; whether any value reaches the page without a keystroke; the row-(7) computation; the 8a/8e sums; the census `Vec` semantics | **Holds for amounts and gates.** Row (7), rows (5)/(6), 8e and the DA box are all printed from the filer's own answers. One coerced answer: R9's cross-check refuses a truthful *No* on a buy-only ledger and a truthful *Yes* on a self-custody one (I6). One row printed without its predicate established (I4). |
| **Derivation** | R2's three instruments against `line_coverage.rs:79-81`, `field_census.rs:170-190`, `coverage.rs:240`; the residual attestation's prompt (`questions.rs:546-563`); the 57 Sch 1 `unmodeled` reasons | **The join is direction-blind** (C1). The gate list is derived from the instructions (good); the *document box* lists are hand-lists (I2). |
| **Documents** | R4's table against `return_inputs.rs:36-187`, the 1040 line instructions, Sch B `:56-66`, the 1099-INT/DIV/W-2 box sets | Boxes collected reach named lines. Boxes not collected are dropped, not censused (I2); one collected box is neither reported nor refused (I3); income the instruction says to report without the document has no door (I1). |
| **Dependents** | R6's table row by row against `i1040gi--2025.txt:1452-1800, 1905-1925, 1943-1970`; `Dependent` (`:223-229`); `GoldenInputs` | Steps 1–5 are transcribed with the instruction's edges, including the Step 2 q.3 taxpayer-joint-return edge I initially misread. Two dropped edges (I4, I5). QSS untested (I7). |
| **Real estate** | R8 against `i1040sca--2025.txt:1058-1155`, `i1040sd:311-354`, `f1040sa.map.toml:99-102`, `return_inputs.rs:280, 613, 647` | 8a/8b/8c/8e and the home-sale table are right. Liveness of the three mortgage declarations regresses (I8). Box 4 (I3). Aggregate ceiling and shared-interest cases (M5). |
| **Exchange side** | R9 against `state.rs:23`, `forms.rs:272-287`, `normalize.rs:63-69`, `return_1040.rs:2468-2475` | The two-store split is clean and the panel is the right shape. The DA predicate exists in core already and R9 restates it wrongly (I6). |
| **Year N+1** | R10 parts 1–4 against `questions.rs:13-35`, `FIELD_PROVENANCE.md:393-406, 440-448`, `cli/return_inputs.rs:24, 57-65`; §7's task list | Schema is structural and has kills. The opener has no task (I11); the hash has no reader (I9); the key is by index (I10). |
| **Oracles** | R13 against `gen_goldens.py:199-232`, `testonly.rs:650-693, 731`; the W-2 arithmetic checks | The projection is the right idea and its inverse KAT is sound for what `GoldenInputs` carries — which excludes dependents, ages, blindness and HoH's qualifying person, so the one line R13 singles out (19) cannot fail (I12). |
| **Journey** | the spec's J-1…J-18 plus a fresh walk with the recon's gap list (below) | Twelve new moments; nine map to findings above, three are Minor. |
| **Buildability** | each T-task's kill against the tree it must run on; §10's freezes against R6's and §4.1's requirements | T8's kill has no map to run on (I13); the N+1 opener is unassigned (I11); per-row liveness needs a named emulation under the §10 seam freeze (M3). Ordering is right — the schema first, the owner's documents next. |

---

## Findings

### C1 (Critical) — R2.2's `covered_by` join accepts an *Advisory* on an income line and checks only that the variant exists, so the "announced or refused, never silent" guarantee is unmet in the understatement direction

**Where.** §3 R2 mechanism 2 and its kill (b); §2.2's opening rule; §8 row 3 ("a line btctax cannot take is announced or refused, never silent").

**What is wrong.** The rule says every `unmodeled` entry on the seven forms gains `covered_by = "Advisory::X" | "RefuseReason::Y" | "QuestionId::Z"` and "a KAT joins each against the enum's own source". Two holes, both in the direction that matters:

1. *Direction.* An **income** or **additional-tax** line covered by an `Advisory` is a line whose omission understates tax, "announced" by a sentence that says btctax did not try. That is not "correct for what it covers or refused". `Advisory` is the right cover for a deduction or credit (forgoing money lawfully); it is the wrong cover for Sch 1 Part I, 1040 1b–1h, 4a–6b, Sch 2, and it is the cover the build will reach for, because there is one advisory and no question for most of those 57 Sch 1 lines.
2. *Reach.* A `QuestionId` cover is checked for existence, not for what the filer is told. `covered_by = "QuestionId::OtherOutOfScopeIncome"` is green for jury duty pay (8h), prizes (8i), hobby income (8j), household-employee wages (1b) and Medicaid waiver payments (1d) — none of which the attestation's prompt names (`questions.rs:547-563`; it names 1099-R, SSA-1099, rent, farm, K-1, tips, gambling, alimony, "a business", then *"or anything else it never asked about"*). That last clause is exactly the compound-no the brainstorm rejected (§1.4) and `FIELD_PROVENANCE.md:289-292` forbids: *"a compound 'no' spanning distinct legal predicates fabricates precision the filer never swore to."* A filer with $600 of jury pay reads "anything else it never asked about", answers No, and the join is green.

This is B1's shape: the kill (b) plants a *missing* or *nonexistent* variant and reds; it never plants an *existing* variant that says nothing about the line. The instrument has not been seen discriminating on the case it exists for.

**Evidence.** `crates/btctax-forms/forms/2024/f1040s1.map.toml:87` (alimony), `:95` (gambling), `:96` (COD — *"Income, so the understatement direction; there is no input that could reach it"*), and 54 more `unmodeled` entries (M4 in the recon: 57). `questions.rs:546-563` is the only residual gate. `CLAUDE.md` "blank because nothing populated it — the defect"; the memory `widening-an-exemption-is-never-the-safe-edit`: enumerate YES-conditions.

**Minimal change (R2.2, one paragraph; T3 gains two assertions).**
> Each census entry carries a *direction*, derived from the form part it sits in and asserted by the KAT from the extract's part headings (Sch 1 Part I, 1040 lines 1–8, Sch 2 = **understates if omitted**; Sch 1 Part II, Sch 3, Sch A = **overstates if omitted**). An understating line may be covered only by a `QuestionId` or a `RefuseReason`; an `Advisory` cover on an understating line reds. For a `QuestionId` cover, the KAT asserts the question's `prompt` contains the census entry's line caption keyword (`reason`'s leading quoted phrase, or a `names = "…"` key on the entry) — so the residual attestation's prompt is forced to enumerate every line it covers, and *"anything else it never asked about"* covers nothing. Kill: plant `covered_by = "Advisory::EicOmitted"` on Sch 1 line 2a → red; plant `covered_by = "QuestionId::OtherOutOfScopeIncome"` on line 8h while the prompt does not say "jury duty" → red.

### I1 (Important) — Income the instructions say to report *without* the document has no door: the census `No` closes it, and the 1099-G refund gate sits on a row that may not exist

**Where.** R3 semantics of `Some(false)`; R4's 1099-G row (`itemized_prior_year` on `Form1099G`); §5.2.

**What is wrong.** The document-first rule is right for amounts, but the form names four incomes that exist without their document, and the spec routes each through a census row whose *No* ends the conversation:
- **Wages with no W-2** — `i1040gi--2025.txt:2442-2444`: *"Even if you don't get a Form W-2, you must still report your earnings."*
- **A taxable state refund with no 1099-G** — `:41897-41898`: *"Report any taxable refund you received even if you didn't receive Form 1099-G."* The spec puts the *"did you itemize on your prior-year return?"* gate on the 1099-G row; a filer whose agency issued the 1099-G electronically and never retrieved it answers `g_1099 = No` and Sch 1 L1 is silent.
- **Interest and dividends with no 1099** — Sch B line 1, `i1040sb--2025.txt:56-58`: *"Report on line 1 all of your taxable interest"*; `i1040gi:2521`: *"if you received dividends not reported on Form 1099-DIV"*; seller-financed mortgage interest received (`i1040sb:24, 77`) names the buyer's SSN and address — a Schedule B row from the filer's records.

Each is small money and each is the understatement direction; the residual attestation's catch-all is the only net (see C1).

**Minimal change (R3, one bullet; R4's 1099-G row, one sentence).**
> The census rows `w2`, `int_1099`, `div_1099`, `g_1099` each carry a paired class-(A) `FormQuestion` live iff the row is `Some(false)`, phrased from the instruction: *"Did you receive wages, salary or tips from an employer who issued no Form W-2?"* (`:2442`); *"Did you receive taxable interest or dividends for which no Form 1099-INT / 1099-DIV was issued (a bank under $10, a seller-financed mortgage you hold, a nominee)?"*; *"Did you receive a refund, credit or offset of state or local income taxes in 2026?"* (`:41897`). `Yes` on the wage or refund question refuses naming line 1a / the State and Local Income Tax Refund Worksheet; `Yes` on the interest/dividend question opens a `FilerRecords` Schedule B row `{payer, amount}` (Sch B lines 1/5 take exactly that). `itemized_prior_year` moves to `ReturnInputs` as a return-level `FormQuestion`, live iff the refund question or a 1099-G box 2 > 0 — and on a year btctax filed year N−1, `screen_compute_dependent` cross-checks it against year N−1's committed `itemize_election` the way R9 cross-checks the DA box.

### I2 (Important) — The document screens' box lists are hand-lists; no per-document box census is enumerated from the T2 extract, so a box the return needs is dropped rather than recorded

**Where.** R4's table ("boxes collected"); §5.2; T2/T5.

**What is wrong.** R2 makes the *form* side mechanical (the `[census]` accounts for every AcroForm FQN) and then R4 lists the *document* boxes by hand. The lists omit boxes the 1040 text itself points at:
- **1099-INT box 10 (market discount) and boxes 11–13 (bond premium)** — Sch B line 1 (`i1040sb:59-61`): *"Also include any accrued market discount that is includible in income"*; 1040 line 2b (`i1040gi:2493-2497`): *"original issue discount or market discount … and adjustments for amortizable bond premium."* Box 10 is income (understates if dropped); box 11 is a reduction (overstates).
- **W-2 box 13 "Statutory employee"** — checked, box 1 goes to Schedule C line 1, not 1040 line 1a. Today's `W2` (`return_inputs.rs:36-80`) has no box 13 and the spec keeps it "exists".
- **1099-DIV box 3 (nondividend distributions)** — a basis reduction, informational, but a box on the paper with no place to record it.

The brief's document lens asks whether a box the return does not use is "recorded as such rather than dropped". It is dropped. The same rule that governs form lines (*"enumerate the line set FROM the form's extracted text, never from a range or a hand-written list"*, `CLAUDE.md`) governs document boxes; T2 archives the extract that makes it possible and T5 does not use it.

**Minimal change (R4, one paragraph; T2 and T5 each gain one kill).**
> Every supported document has a `[boxes]` census beside its `Field`s, enumerated by a KAT **from the archived `fNNNN` extract's box captions** (T2), each box either `collected(FieldId)`, `refuse_if_nonzero(RefuseReason)` (box 9 today), or `not_read(reason)` with the instruction's pointer; a caption in the extract with no entry reds. Then decide the three above as the form says: 1099-INT box 10 `collected` → Sch B line 1 (add to the 2b sum); boxes 11–13 `refuse_if_nonzero` naming the ABP adjustment (Pub. 550) until transcribed; W-2 box 13 statutory-employee `collected: Bool` → `Some(true)` refuses `StatutoryEmployeeW2` naming Schedule C line 1; 1099-DIV box 3 `not_read("reduces basis; Pub. 550")`.

### I3 (Important) — Form 1098 box 4 (refund of overpaid interest) is collected, shown, and neither reported nor refused; it is Schedule 1 line 8z income

**Where.** R8 first bullet: *"**4** refund of overpaid interest → shown with the instruction's pointer to Sch 1 L8z (`:1069-1072`; not computed — the census entry names it)"*; §5.2 `Form1098.box4_refund_overpaid_interest`.

**What is wrong.** `i1040sca--2025.txt:1067-1070`: *"If your Form 1098 shows any refund of overpaid interest, don't reduce your deduction by the refund. Instead, see the instructions for Schedule 1 (Form 1040), line 8z."* The refund is income in the year received to the extent the interest reduced tax in an earlier year. The spec collects the figure — so the tool *knows* — and the outcome is a blank 8z with a census note. That is a typed number with no reader in the understatement direction; C1's direction rule would red it, and the spec text must change with it.

**Minimal change (R8, replace the box-4 clause).**
> **4** refund of overpaid interest → `> 0` refuses `MortgageInterestRefundNotComputed`, *"Form 1098 box 4 is income on Schedule 1 line 8z to the extent the interest reduced your tax in an earlier year (Pub. 936, 'Refund of home mortgage interest'); btctax does not compute it"* — the same shape as box 9 on the 1099-INT. Kill: a fixture 1098 with box 4 = $1 refuses; with box 4 = 0 it does not.

### I4 (Important) — R6 prints a dependent row when the age test cannot be evaluated: DOB absent, or the taxpayer's DOB declined, leaves neither §152(c) nor §152(d) established

**Where.** R6 table, Step 1 age row: *"DOB absent ⇒ the test is unknown ⇒ no credit column, class-(B) forgo with the advisory naming the missing DOB"*; §5.3 (`date_of_birth` stays `Option<Date>`); R6's kill.

**What is wrong.** The age test is a **Step 1 condition of being a qualifying child** (`i1040gi--2025.txt:1487-1499`), not a credit condition. Step 1 ends *"Do you have a child who meets the conditions…? Yes → Step 2. No → Step 4."* There is no *unknown* edge. With DOB absent the spec neither sends the row to Step 4 (gross-income and support tests) nor refuses; it prints the person in the Dependents section — testimony that §152 is satisfied — on a chain that was never completed. Listing a dependent is what the instruction calls *"the dependents you claim"* (`:1470-1472`). Second half: *"younger than you (or your spouse if filing jointly)"* is computed from the taxpayer's DOB, which is a class-(B) **skippable** that R10 lets the filer `Decline` — so the spec computes a Step 1 predicate from a value the filer may lawfully withhold.

**Evidence.** `Dependent.date_of_birth: Option<Date>` (`return_inputs.rs:228`); `SKIPPABLE_QUESTIONS` DOB entries (`questions.rs:939-941`, Durable); R6's truth-table kill enumerates "each row in the table above" and the table has no row for the absent-DOB edge.

**Minimal change (R6, two sentences; §5.3 one field).**
> `date_of_birth` is **required on every dependent row** — `None` is `DependentGateUnanswered { row, gate: DateOfBirth }` (a birth date is on the SSN application and the birth certificate; it is a fact, not a decision). The instruction's *"younger than you (or your spouse if filing jointly)"* is asked as its own per-row gate `younger_than_you_or_spouse: Option<bool>` (`:1489, :1493`), so no Step 1 predicate depends on a declinable skippable; the age brackets (<19 / <24 / any) compute from the row's DOB and `tax_year`. Kill: a row with `date_of_birth = None` refuses; a Single return whose DOB skippable is `Declined` still resolves Step 1 for a row with `younger_than_you_or_spouse = Some(true)`.

### I5 (Important) — R6 strips the *Exception to time lived with you* from row (5)(a), so J-11's newborn is routed to Step 4 and printed with the ODC box, not the CTC box

**Where.** R6 table, row (5)(a): *"No ⇒ Step 4 (the *Exception to time lived with you*, `:1905`, is never applied — conservative)"*; §6 J-11.

**What is wrong.** The exception is part of the instruction's own definition of the condition, printed in the same flowchart box (`:1512-1516`: *"If the child didn't live with you for the required time, see Exception to time lived with you, later"*), and `:1905-1925` states it as a rule the filer applies: *"Temporary absences … such as school, vacation, business, medical care, military service … count as time the person lived with you"*; *"If the person meets all other requirements to be your qualifying child but was born or died in 2025, the person is considered to have lived with you for more than half of 2025 if your home was this person's home for more than half the time the person was alive."* A child born in November 2026 — the case J-11 names — answers the spec's literal gate *No*, goes to Step 4, passes the qualifying-relative tests (a newborn has no gross income and the filer provided all support), and prints **row (7) "Credit for other dependents"** — a checked box that is wrong on its face (the child qualifies for CTC), a $1,500 under-claim, and not a visible forgo: nothing in the panel says so. A college student away at school for nine months is the same misroute. "Never applied — conservative" is not conservative here; it fabricates an ODC where the form says CTC.

The right reading of *"see Exception … later"* is the brainstorm's own rule for "see X, later": it is a **definition the filer applies**, not a Pub. 501 branch — the instruction states it fully in one paragraph. Only the kidnapped-child and divorced-parents pointers leave the flowchart.

**Minimal change (R6 row (5)(a), replace the parenthetical).**
> The gate's `help` carries `:1905-1913` verbatim (temporary absences; born or died in the year; adopted or placed in the year), so the filer answers the instruction's condition with its exception in front of them; the `Yes` prints row (5)(a). *Kidnapped child* (`:1910`) and *Children of divorced or separated parents* (`:1824`) stay REFUSE branches (they already are, via `divorced_separated_multiple_support_or_kidnapped_rule_applies`). Kill: the truth table gains the born-in-year row — a fixture child with DOB 2026-11-01, `lived_with_you_over_half_year = Some(true)`, lands on the CTC edge; `line-coverage` checks the help quotation.

### I6 (Important) — R9's Digital-Assets cross-check states the wrong predicate in both refuse cells: a buy-only ledger forces a *Yes* the instruction does not require, and a self-custody filer's truthful *Yes* is refused with an exit that does not exist

**Where.** R9 third bullet: *"ledger has events ∧ `No` ⇒ refuse …; ledger empty ∧ `Yes` ⇒ refuse 'import your exchange exports'"*; its kill ("the DA four-cell table").

**What is wrong.** The instruction (`i1040gi--2025.txt:1356-1362` and the bullets that follow) checks *Yes* for **receipts as reward/award/payment and dispositions** — not for purchases with real currency or transfers between the filer's own wallets. A filer who bought BTC on Swan every month and sold nothing has a ledger full of events and the correct answer *No*; the spec's first cell refuses that answer and the filer changes it to *Yes* to proceed — a coerced answer written as testimony. The second cell refuses a *Yes* on an empty ledger and names *"import your exchange exports"* as the exit — but a filer paid in BTC to a self-custody wallet has no export (recon §E.1: *"no self-custody wallet is importable at all"*), so the only way through the gate is to answer *No*, which is false.

The correct predicate already exists: `return_1040.rs:2468-2475` `digital_asset_activity(state, year)` = any disposal ∨ income recognized ∨ removal in the year — exactly the instruction's list, and the doc comment there records the reasoning (*"a 'No' it cannot vouch for is worse than leaving the question to the filer"*). R9 restates it as "has events" and loses the distinction.

**Minimal change (R9, replace the two refuse cells).**
> `digital_asset_activity(state, year)` (the existing predicate) ∧ `No` ⇒ refuse `DigitalAssetAnswerContradictsLedger` naming the first qualifying event. `!digital_asset_activity` ∧ `Yes` ⇒ **accept and warn** (*"the ledger shows no receipt or disposition in 2026; if you had off-ledger activity — a self-custody receipt, a payment in kind — it is not on this return: Schedule 1 line 8v and Form 8949 come only from the ledger"*), never refuse — the ledger is not complete by construction. Kill: the table gains a fifth row — a ledger of `Acquire` and linked self-transfers only, answer `No`, prints *No*.

### I7 (Important) — Filing-status assertions: HoH's "considered unmarried" is one compound gate over three legal predicates (one with five sub-conditions), and Qualifying Surviving Spouse is offered with no test at all

**Where.** R7; §5.4 (`hoh_unmarried_or_considered_unmarried`); `FilingStatus::Qss` (`crates/btctax-core/src/tax/types.rs:15`), `FilingStatusArg::Qss` (`cli.rs:1176`).

**What is wrong.**
1. `hoh_unmarried_or_considered_unmarried` compounds `:1149-1163`'s three bullets: legally separated under a decree; *married but lived apart … and you meet the other rules under Married persons who live apart*; NRA spouse without the election. The second bullet is itself five conditions (`:1247-1268`: lived apart the last 6 months; separate return; paid over half the cost of the home; the home was the main home of your child for more than half the year; you can claim the child). A married filer's *Yes* to the compound is exactly the compound-yes the spec's own R1 forbids (*"fabricates precision"*, `FIELD_PROVENANCE.md:289-292`), and none of the five sub-conditions is asked.
2. **QSS** (`i1040gi--2025.txt:1288-1316`) has five conditions — *"Your spouse died in 2023 or 2024 and you didn't remarry before the end of 2025"* (`:1295-1297`; the TY2026 revision shifts the window by one year); a child or stepchild (not a foster child) you can claim as a dependent (or could but for three named reasons); the child lived in your home all year; you paid over half the cost of keeping up the home; you could have filed jointly the year your spouse died — and unlocks joint rates and the joint standard deduction. R7 fixes HoH's *"no test at all"* (recon §C.3) and leaves QSS in the identical state. Choosing QSS is an assertion.

For the avoidance of a false lead: the new TY2025 page-1 checkbox *"Check if your filing status is MFS or HOH and you lived apart from your spouse for the last 6 months …"* (`f1040--2025.txt:54-56`) is the **EIC special rule for separated spouses** (`i1040gi:4866-4876`), class (B) under `EicOmitted`; it is a census cell, not an R7 gate.

**Minimal change (R7, replace the first question; add a QSS paragraph).**
> `hoh_marital_basis: Enum { NotMarried, LegallySeparatedByDecree, MarriedLivedApart, NraSpouseNoElection }` (the instruction's own four states); `MarriedLivedApart` refuses naming *Married persons who live apart* (`:1247`) until its five conditions are transcribed as gates (they are five `Option<bool>`; T8 may do it in the same task); `NraSpouseNoElection` refuses naming *Nonresident aliens and dual-status aliens* (existing declaration territory). **QSS:** five `FormQuestion`s live iff `filing_status == Qss`, phrased from `:1295-1310`; any `None` refuses `QssTestUnanswered`; any `No` refuses `QssTestNotMet` — *"choose another filing status"*. Kill: `Qss` with any `None` refuses; `Single` asks none; the enum's `MarriedLivedApart` refuses with the rule's name in the message.

### I8 (Important) — R8 keys the three mortgage declarations on the top-level `form_1098` regardless of Schedule A, so a standard-deduction filer with a post-2017 loan over $750,000 is refused `MortgageOverDebtLimit` for a deduction they are not claiming

**Where.** R8 first bullet: *"the `MortgageAllUsed` / `AmtQualifiedDwelling` / `MortgageWithinDebtLimit` liveness keys on `!form_1098.is_empty()` instead of `> 0`"*; §5.1 (`form_1098` census row always live); §5.2 (`form_1098` top-level).

**What is wrong.** Today the three are live only inside an itemizing return: `questions.rs:280` `.is_some_and(|a| a.mortgage_interest_1098 > Usd::ZERO)` — `schedule_a` must exist. The spec moves the 1098 to the top level ("it arrives whether or not the filer itemizes") and drops the Schedule A condition. Then: the always-live census row makes every homeowner transcribe the 1098; the declarations go live; `MortgageWithinDebtLimit = Some(false)` — the truthful answer for a $900k 2019 loan — refuses (`return_inputs.rs:644-647`), and the filer who takes the standard deduction cannot file. `AmtQualifiedDwelling` is Form 6251 line 3, which reads *"If you deducted home mortgage interest on Schedule A"* — the same condition. And a standard-deduction filer's 1098 rows are then a `Field` set nothing reads, which R2.3's reverse join is supposed to red.

**Minimal change (R8, one sentence; §5.1 one liveness).**
> `form_1098` stays top-level, but the census row and the section are live iff `schedule_a.is_some()` (the itemize election is the filer's, as today), and the three declarations key on `schedule_a.is_some() && !form_1098.is_empty()`. Kill: a standard-deduction fixture with a 1098 at $900k box 2 does not ask `MortgageWithinDebtLimit` and does not refuse; the same fixture with a Schedule A does.

### I9 (Important) — R10's `prompt_hash` is stored and never read: the spec gives no rule for a mismatch, so an answer given under earlier words stands under later ones

**Where.** R10 part 3 (`AnswerRecord { answered_on, prompt_hash, state }`); R12 (`interview_state`); §8 (no kill mentions a mismatch).

**What is wrong.** The hash exists so that *"which words were asked"* is on record (`FIELD_PROVENANCE.md:393-395`). Its only use that matters is to detect that the words changed. Nothing in the spec compares it to the current prompt — not `interview_state`, not `screen_inputs`, not the N+1 opener (which re-asks every `PerYear` gate blank anyway). So the very case it exists for — a prompt edited in a fold in November under an answer given in September on the same year's draft (the Sep–Dec calendar R11 designs for) — is silent. A value with no reader is not a guarantee (memory: *a-figure-with-no-reader*).

**Minimal change (R10 part 3, one sentence; R12 one row; T1 one kill).**
> `interview_state` treats an `AnswerRecord` whose `prompt_hash` ≠ `hash(current prompt)` as **unanswered** (blocking for class (A), forgoing for class (B)) with the reason *"the wording of this question changed since you answered"*, and `screen_inputs` refuses such a class-(A) record as `UNANSWERED`. Kill: change one prompt's text in a fixture registry → the answer reappears in `blocking`; restore it → it does not.

### I10 (Important) — `AnswerKey::DependentGate { row, gate }` keys the diligence record by row index, so deleting or reordering a dependent moves one person's `answered_on` onto another

**Where.** R10 part 3; §5.6.

**What is wrong.** `Dependent` rows are a `Vec` with `add`/`remove` through the seam (`seam.rs:295-297`). Delete row 0 and every entry keyed `{ row: 1, .. }` now describes row 0 — a different child — with the original date and hash intact. `FIELD_PROVENANCE.md:440-441`: *"a diligence record that lies is worse than none."* The N+1 opener (R10.4) then confirms year N's *identities* by SSN, so the model already has the right key and does not use it here.

**Minimal change (R10 part 3, one clause).**
> `AnswerKey::DependentGate { ssn_hash: String, gate }` — keyed by the row's identity (a salted hash of `ssn`, never the digits, per the secret rules), not its index; `remove` on the Dependents section deletes that identity's entries; a row whose `ssn` changes starts a fresh record. Kill: answer gates on two rows, delete row 0, and assert row 1's records are unchanged and row 0's are gone.

### I11 (Important) — The year-N+1 opener (R10 part 4) has a mechanism and a kill and no task in §7

**Where.** R10 part 4 and its kill (*"The N+1 opener: a fixture year N with two payers yields exactly two identity prompts…"*); §7 T1 (lists the schema pieces only); §8 (no row for the opener).

**What is wrong.** The brief's one question has two halves; the second — *"re-ask only what changed a year later"* — is R10.4: year N's identities as fresh tri-state prompts, durable facts confirmed by keystroke, `PerYear` gates blank, carryforwards as `ComputedFromPriorReturn { year }` data. T1 builds `CarryProvenance::ComputedFromPriorReturn` and the log; nothing builds the function that reads year N and seeds year N+1's draft, the TUI/CLI surface that presents the confirmations, or the flow that carries a *computed* carryforward from year N's committed return into year N+1's inputs (which also needs to name where year N's computed carryforward is read from). Twelve unconditional tasks, and the every-year machinery the strategy ranks above any single year (`ROADMAP_STATUS.md` §0) is in none of them.

**Minimal change (§7, one row; §8 one row).**
> **T4b — Year N+1 opener** (R10.4): `open_next_year(conn, n_plus_1) -> Draft` in `btctax-cli` — reads year N's committed row, seeds the draft with each payer (TIN + name) and each dependent (identity fields) as pre-named rows with every box blank and every gate `None`, each identity as a census-row prompt; durable facts (DOB) displayed and confirmed by a `SetField` that writes a fresh `AnswerRecord`; year N's computed capital-loss and charitable carryforwards written with `ComputedFromPriorReturn { year: N }`. Kill: R10.4's, plus: no `Usd` leaf of the seeded draft is non-zero except the two carryforwards, and each carries the computed provenance.

### I12 (Important) — R13's projection carries no dependents, ages or blindness, so the line-19 excuse `oracle_line19 − 0` is `0 − 0` on every return and the pre-export oracle run is blind to CTC, EIC and the §63(f) boxes

**Where.** R13 mechanism (the box-named table) and kill (*"The CTC excuse is `oracle_line19 - 0` and a diff of any other size on line 19 fails"*); T11.

**What is wrong.** `GoldenInputs` (`testonly.rs:650-693`) has filing status, wages, interest, dividends, gains, SE income, four Schedule A components and a cash gift — **no dependents, no ages, no blindness**. `gen_goldens.py:199-232` projects the same. So `project_to_golden` cannot carry a dependent, Tax-Calculator's `n24` stays 0, `oracle_line19` is 0, btctax's line 19 is blank-as-0, and the "excuse computed from mechanism" passes on a return with three qualifying children. R13's own sentence — *"line 19 is expected to differ by exactly the oracle's CTC while 8812 is unbuilt"* — presupposes an oracle that sees the children. As written the one line R13 singles out is a check that cannot fail; the same blindness covers EIC (27), the aged/blind add-ons and HoH's qualifying person.

**Minimal change (R13 table, one row; `GoldenInputs`, T11).**
> `GoldenInputs` gains the dependents block the oracles take: `n24` (children under 17 with the CTC edge), `nu18`, `n1820`, `n21` (age bands from DOB), `age_head`/`age_spouse` (from the DOB skippables when given), `blind_head`/`blind_spouse`, and `EIC` count — projected from the T7/T8 answers; the OTS template's dependent lines likewise. The line-19 excuse is then the oracle's computed CTC and a diff of any other size fails. Kill: the projection inverse now covers a golden with two children; a fixture with one CTC-edge child yields `oracle_line19 > 0` and the excuse equals it exactly.

### I13 (Important) — T8's kill "TY2025 fixture rows filled" names a map with no dependents-grid cells

**Where.** §7 T8 (the task's own parenthetical: *"TY2025 has none for the grid — `crates/btctax-forms/forms/2025/f1040.map.toml:1` is capital-gains only — TY2026 after finals"*) and its kill column.

**What is wrong.** `forms/2025/f1040.map.toml` is 17 lines: `line7a`, `da_yes`, `da_no` (`:1-17`). There is no cell for rows (1)–(7) of any dependent, and the TY2025 `f1040` sits in the `UNCENSUSED` register at 196 fields (recon §B.6). The TY2026 final does not exist until January. So the emitter that fills rows (5)–(7) has no form to fill in the build window, and the kill that proves it cannot run. A task whose kill names a fixture that has no map cannot be built as written.

**Minimal change (T8, one clause).**
> T8 includes mapping the TY2025 `f1040` dependents grid — rows (1)–(7) × 4 dependents plus the more-than-four box (`f1040--2025.txt:38-52`, read through the label reader) — into `forms/2025/f1040.map.toml`, leaving the rest of the form in the `UNCENSUSED` register unchanged (the register's count for `f1040` falls by exactly the cells mapped, asserted). The TY2026 map ports the cells after finals. Kill as written, now runnable.

### I14 (Important) — §9 Q1 asks the owner about six document families and omits most of §2.2's refusal families, so the answer that decides whether v1 fills the owner's return is under-asked

**Where.** §9 Q1; §2.2's table; `OWNER_DECISIONS_2026-09-04.md:61-63` (D-B's assumption list).

**What is wrong.** Q1 names: 1098, dependents, 1099-G/itemized 2025, Schedule C/1099-NEC, 1099-R/SSA-1099, home sale/K-1/rental. §2.2 refuses, in addition: **Form 1095-A** (a self-employed owner — P0 says SE income — is a likely Marketplace enrollee; a 1095-A refuses the whole return and no task builds 8962), **HSA** (W-2 box 12 code W refuses today), **IRA contributions**, **1098-T**, **W-2G**, **1099-C**, **1099-OID**, **1099-S** for any real property. Each is a *"cannot file with btctax this year"* for the owner, discovered — under the current text — in February 2027 on the extension (memory: *the-lived-journey-scheduled-last*). D-B's sentence (*"every income TYPE on the owner's real return is a member of P0's set … and nothing else"*) was ruled on by a delegate; Q1 is the spec's chance to make the owner say it against the full list.

**Minimal change (§9 Q1, replace the list).**
> Q1 is §2.2's table, one yes/no per row, plus the six Q1 already asks. A *yes* on a T13/T14 row moves that task into v1; a *yes* on any other row is a named refusal the owner accepts now (preparer for 2026) or a task the owner adds now — never a discovery in February.

---

### M1 (Minor) — The four-state panel has no "answered, refusing" state, so a `k1 = Yes` is invisible until commit and J-17's round trip returns

**Where.** R12's table; R3's fourth bullet. **What.** A census row answered `Some(true)` on an unsupported type, or `BasisDiffers` on a broker row, is *answered* — the panel counts it and says nothing; `screen_inputs` refuses it at commit. Every `FormQuestion` carries its refusal, so the panel can derive it. **Change.** A fifth row: *live, answered, refuses* — listed as **refusing** with the exit sentence; the extended no-brick test asserts `refusing` is empty before `screen_inputs` passes.

### M2 (Minor) — A `Declined` skippable disappears from the forgoing list exactly when the forgo becomes final

**Where.** R12 kill: *"A live skippable with `Declined` is neither blocking nor forgoing"*; §4.4 (the manifest's forgoing block). **What.** Declining the DOB skippable at 66 forgoes the §63(f) add-on; the panel and the manifest stop saying so. `Declined` is provenance (asked, refused) — the benefit is still forgone. **Change.** `Declined` items stay in `forgoing` marked *(declined)* with the size; only `Given` removes an item.

### M3 (Minor) — Per-row gate liveness has no seam: `Field.live` takes `&ReturnInputs` only, and §10 freezes the seam

**Where.** R6 (`live(&ReturnInputs, row)` on `DependentGate`); `seam.rs:266` `pub live: fn(&ReturnInputs) -> bool`; §10 (*"new `SectionId` / `FieldId` variants only"*). **What.** A Step 4 gate is live for a row only when that row's Step 1 said No; `Field.live` cannot see the row. Two honest builds exist and the spec names neither: (a) the I-4 emulation already in `registries.rs:44-50` — `live: |_| true` and `get` returning `None` (absent) when the row-gate is not live, which the renderer already treats as hidden; (b) widen `live` to `fn(&ReturnInputs, &RowAddr) -> bool` (98 mechanical closure edits). **Change.** Say which; (a) keeps §10 intact and costs nothing.

### M4 (Minor) — Two refusal sentences name the wrong exit

**Where.** §2.2 rows *1099-S* and *1099-NEC/MISC/K*. **What.** A 1099-S for a vacant lot, an inherited house or a rental is not a Pub. 523 case; the sentence should say *"Form 8949 / Schedule D for real property; Pub. 523 if it was your main home"*. A 1099-MISC box 3 (prizes, awards, research-study pay) is Schedule 1 line 8z income, not *"non-employee compensation"* and not Schedule C; the row's sentence should name 8z for box 3 and Schedule C for box 1/NEC.

### M5 (Minor) — The 1098 ceiling warning is per row; the §163(h)(3) limit is on aggregate acquisition debt (halved for MFS), and the *more than one borrower* case is unasked

**Where.** R8 first bullet and its kill (one 1098). **What.** Two 1098s at $500k each are silent per row and over the limit together; MFS's limit is $375k/$500k. `i1040sca--2025.txt:1071-1080`: a co-borrower who is not the spouse deducts *"only your share"*; the screen sums box 1 in full. **Change.** Warn on Σ box 2 across rows against the status-adjusted limit; add a per-row gate `other_borrower_paid_interest` (Yes ⇒ refuse naming *More than one borrower*).

### M6 (Minor) — T2's kill contradicts R5: "a `Collected` line with no `DocBox` reds", but R5's lines are collected from the filer's records

**Where.** T2 kill column; R2 mechanism 1 (`from: DocBox { … }`); R5. **Change.** `from: DocBox { .. } | FilerRecords { instruction_line }`; the kill reds on a `Collected` line with neither.

### M7 (Minor) — `gross_income_under_limit` asks the filer to compare against a figure the prompt cannot state on a params-less year

**Where.** R6 Step 4 row (*"the figure is a `FullReturnParams` value"*); R11. **What.** In Sep–Dec 2026 TY2026 has no params, so the prompt cannot carry the $5,200-equivalent, and the filer is asked to derive. **Change.** The gate is not live until the year's params exist; the panel lists it as *waiting for the TY2026 package* (an R11 state, not a blocking item).

### M8 (Minor) — Two small provenance holes on document rows

**Where.** R4 (*"no `Option<Usd>` is needed inside a document row"*); §5.2 `transcribed_on: Option<Date>`. **What.** A row with a payer and every income box zero is most likely a row the filer forgot to finish (a 1099-INT is issued at ≥ $10) — a typo detector like the W-2 checks; and a TOML row arrives with `transcribed_on = None`, which the manifest should print as *transcribed without a date*, not omit. **Change.** Warn on an all-zero income-box row; the manifest prints the absence.

### M9 (Minor) — Two journey moments with a cheap fix

**Where.** R5 (line 26's help); R11/§4.3 (`income import`). **What.** (a) `i1040gi--2025.txt:4266-4268`: line 26 *"Include any overpayment that you applied to your 2026 estimated tax from your 2025 return"* — the owner's TY2025 was filed outside the project; R5 names 5a's timing words only. (b) `income import` still writes an unscreened committed row on a params-less year (recon §A3, unchanged); a TOML with `documents.k1 = true` is stored and poisons the year at `report`. R3's three census invariants need no params. **Change.** (a) line 26's help carries the clause, checked by `line-coverage`; (b) `income import` runs the param-free screens (R3's invariants, `NegativeAmount`) before writing.

---

### N1 (Nit) — R4's kill names "the six `NotInForm` anchors in this cluster" and lists five; "`IraDeductionClaimed` is not — see R8" points at an R8 that does not mention it. T5's kill "falls by six" will red on first run at five.
### N2 (Nit) — R14 says "`Dependent` gates ×18"; §5.3 lists 15 gates + 4 row-(5)/(6) fields = 19 `Option<bool>` leaves.
### N3 (Nit) — R12 lists "the census rows" beside `FORM_QUESTIONS` as a fourth registry; R3 makes them `FORM_QUESTIONS` entries, so they are one walk, not two.
### N4 (Nit) — Off-by-one cites: `year_readiness.rs:28` → `:27`; `return_inputs.rs:233` → `:232`; `answer.rs:117-121` is the doc comment, the refusal is `:131-137` (the spec cites both, fine); everything else resolves exactly.

---

## The journey — February 2027, re-walked with the recon's gap list

Beyond the spec's J-1…J-18 (each of which I checked against the rule it cites — they hold as stated). New
moments, classified; a divergence is listed only where the wrong outcome is worse than silence.

| # | the owner… | today / as specced | class | finding |
|---|---|---|---|---|
| J-19 | has $6 of credit-union interest, no 1099-INT; answers the census *No* | Sch B line 1 silent; only the attestation's *"anything else"* stands between them and an omission | **R** | I1 |
| J-20 | reads box 11 (bond premium $40) and box 10 (market discount $120) on the 1099-INT | no field; typed nowhere | **R / not-read** | I2 |
| J-21 | 1098 box 4 shows a $300 refund of overpaid interest | shown with a pointer; 8z blank | **R** | I3 |
| J-22 | child born 2026-11-01; *"lived with you more than half of 2026?"* — literally no | Step 4 → ODC box checked | **Doc (the help text)** | I5 |
| J-23 | bought BTC monthly on Swan, sold nothing; answers the DA question *No* | refused; changes it to *Yes* to proceed | **R (wrong)** | I6 |
| J-24 | takes the standard deduction; the census made them transcribe a $900k 2019 1098 | `MortgageWithinDebtLimit = No` → refused | **R (wrong)** | I8 |
| J-25 | answered the foreign-accounts gate in September; a fold reworded it in November | the September answer stands under November's words | **W** | I9 |
| J-26 | removes dependent row 1 (moved out); row 2 becomes row 1 | row 1's `answered_on` now describes another child | — | I10 |
| J-27 | (February 2028) opens TY2027 | nothing seeds from 2026 — the opener was never assigned | — | I11 |
| J-28 | runs `check_return.py`; line 19 matches; they have two children | the match is 0 = 0 | — | I12 |
| J-29 | is on a Marketplace plan and holds a 1095-A | refused, first learned now | **R (late)** | I14 |
| J-30 | widowed in 2025; picks *Qualifying surviving spouse* | accepted, nothing asked | **R** | I7 |
| J-31 | applied the 2025 overpayment to 2026 estimates; types only the four vouchers | line 26 short by the applied amount | **Doc** | M9 |
| J-32 | answers `k1 = Yes` at the census; the panel shows nothing blocking; commits | refused at commit — the J-17 round trip, once | **W** | M1 |

Divergences that earn nothing: a corrected 1099 arriving after transcription (re-type the row; not our
concern); the spouse's separate 1099-INT on MFJ (Schedule B does not distinguish owners); an amended prior-year
return (out of scope).

---

## Where the spec departs from the brainstorm, and whether the departure is justified

| departure | verdict |
|---|---|
| No 1099-DA transcription screen (brainstorm Q5 → spec R9 "decided") | Justified: the answer is a comparison the filer swears to; the panel states the consequence; a struct for a warning is not v1. |
| `screen_inputs` not refactored; the panel derived from registries (R12) | Justified and better: it keeps the frozen first-refusal contract and uses the primitive `FIELD_PROVENANCE.md:107-108` names. |
| Provenance by struct, not per-leaf metadata (R10) | Justified — but it makes I9/I10 the whole of what the per-answer log must get right. |
| Line 19 a visible forgo until 8812 (R6, owner Q2) | Justified with I12 folded; without it the "visible size" has no oracle behind it. |
| Step 0 parallel authoring, hard gate at commit (R9, brainstorm Q2) | Justified; the Sep–Dec calendar needs it and the existing gates hold. |
| Steps 1–5 vs the brainstorm's "Step 1–4" | The spec is right: Step 5 exists (`:1737-1790`). |
| The residual attestation kept unchanged (R3) | Half right: keeping it is correct; C1 makes its prompt the derived enumeration it must become. |

---

## What I did not examine

The `reconcile` command set and the four adapters (§10, unchanged); `SPEC_1099da_broker_reporting.md` R1–R6
beyond the R6 cite; the TY2025 map internals beyond `f1040.map.toml`; compile-level shape of any struct
(the build's job per the brief); the Python oracle scripts beyond the cited lines; secret handling (never
blocking, by ruling); `SPEC_input_form.md` §9A's key contract; the web renderer (not this spec); Schedule C
Part II and the retirement spec (T13/T14, conditional on S2). I did not run cargo.

Counts: C=1 I=14 M=9 N=4
