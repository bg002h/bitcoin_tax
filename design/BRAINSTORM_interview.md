# BRAINSTORM — the interview that fills the return

**Status:** BRAINSTORM r1 (Fable, lens 1 of 3). **Written 2026-09-06 at `bf58fcdb` (`main`).**
**Brief:** `design/agent-reports/BRIEF-brainstorm-interview.md`. **Measured map:**
`design/agent-reports/2026-09-07-interview-recon.md` (cited below as *recon §X*; nothing in it was
re-measured here). Read-only; nothing else in the repo was edited.

**The owner's ask, verbatim:** *"Can we design an interview process to elicit income data, deduction
related information, real estate, dependence, etc., as well as exports from Bitcoin exchanges to allow
the interview process to fill out most of the tax return, if not all of it?"*

---

## 0. The answer in one paragraph

Yes — and most of it is already built as an engine with no questions in it. The recommended shape is
**document-first, form-derived, one journey over two question sets**: the interview opens with a
*document census* (which information returns did you receive — one tri-state per document type), each
declared document opens a *transcription screen* whose fields are the document's own boxes, and only
then come the form's *gate questions* — the two registries that exist today plus the section-level
gates transcribed from the instructions' own "if X, skip to Y". Every amount on the return is copied
from a document the filer holds (or from the filer's own records, recorded as such); no question ever
asks for a dollar figure the form attributes to a document. The exchange side stays a separate question
set with a separate store, because it answers about *events* and the return answers about *a year*;
the interview's first screen is the ledger's status and its crypto section is seeded from the ledger,
exactly as `BrokerReporting` already is (recon §A1 row 13). The generator that makes this "derived" is
a **conformance check, not a code generator**: a human transcribes each gate and each document from the
form text, and a KAT proves every censused field traces to a document field, a gate, a computation or a
recorded refusal — the join `FIELD_PROVENANCE.md` §4–§5 specified and nobody built (recon §G.3).

What v1 covers is set by the S2 rule — the owner's real 2026 return decides — and inside that rule the
scope is *every line the Form 1040 and its Schedules 1/2/3/A/B/D carry that needs no new form
template*, plus the year-N+1 provenance schema, which must land before the first TY2026 answer is
stored because it cannot be back-filled (`FIELD_PROVENANCE.md` §6f).

---

## 1. Lens 1 — the interview as a DERIVED artifact

### 1.1 What "derived" can honestly mean here

The tempting reading — a program reads `i1040gi` and emits questions — does not survive contact with
the text layer. The label reader needed three separately-discovered layout facts to enumerate *line
numbers* correctly (`design/forms/LABEL_READER.md` ⑤, facts 1–3), and the instructions are prose:
Step 1 of *Who Qualifies as Your Dependent* is a flowchart rendered as paragraphs with "Yes. Go to Step
2. / No. Go to Step 4." edges (`design/forms/extract/i1040gi--2025.txt:1452-1560`). Nothing mechanical
turns that into a `FormQuestion` without a human deciding what the predicate is.

So the honest meaning of *derived* is **derivable and checked**: a human transcribes, a machine proves
nothing was skipped. The repo already does exactly this for money lines — `field_census.rs` reads every
FQN by text scan *"so the census must see every FQN the file names, including any a future typed struct
forgets to model"* (recon §F.4) — and it does it for declarations (`QuestionId::ALL` is *"the anchor the
completeness test iterates"*, `questions.rs:93-94`). What does not exist is the join between them.

### 1.2 The pipeline, with what exists and what does not

| stage | artifact | exists? |
|---|---|---|
| form text | `design/forms/extract/{f,i}NNNN--YYYY.txt` (`pdftotext -layout`) | yes — 2024, 2025, 2026-DRAFT |
| line set | `xtask label-census <stem>` → labels; the `[census]` table per `.map.toml` keyed by AcroForm FQN | yes — TY2024 complete, 1,362 fields, 0 unaccounted (recon §B.6) |
| per-field rule | `computed` / `collected(input)` / `asked(QuestionId)` / `not-ours` / `unmodeled(advisory)` / `artifact` (`FIELD_PROVENANCE.md` §6c) | the six-way is specified; today's census carries `unmodeled` / `artifact` and the map carries the fill — `collected` vs `computed` and `asked(Q)` are **not** recorded |
| question graph | `FORM_QUESTIONS` (17) + `SKIPPABLE_QUESTIONS` (19), each with the only copy of its liveness predicate | yes, for 36 gates (recon §A2) |
| section-level gates | the instruction's "if you did not X, skip lines a–b" | **no** — `FIELD_PROVENANCE.md` §4: *"no question exists for whole form sections"* |
| document screens | `W2` (14 Fields + box 12 rows) | yes for the W-2 only; `Form1099Int/Div/G/B` are structs with box-named fields and **zero** form Fields (recon §G.1) |
| the join | field → question / question → fields | **no** (recon §G.3) |
| the renderer | `FormSpec` sections → TUI; `income answer`; the serde `Edit` wire | yes (`SPEC_input_form.md` §3–§5) |

The generator, then, is three additions: (i) the census learns `collected(FieldId)` and
`asked(QuestionId)`; (ii) the registries gain section-level gates; (iii) a KAT asserts *every censused
`asked(Q)` names a registry entry, every `collected(F)` names a `FieldId`, and every `FieldId` is
referenced by at least one census entry or an explicit exemption*. That last clause is the inverse of
the coverage KAT that already polices the 98 Fields (recon §A1) — a Field nothing prints is as much a
defect as a line nothing fills.

### 1.3 Where it breaks, and what each break is

| the line's instruction references… | what it is | how the interview handles it |
|---|---|---|
| **a worksheet** (Social Security Benefits Worksheet, Simplified Method, Sch D's Test 1/Test 2) | a transcription struct in its own right — `CLAUDE.md` says *"form, schedule, or worksheet"* | its inputs are documents (SSA-1099 box 5) and gates; the worksheet is a compute node, not an interview node |
| **another form** (Sch 8812, 8863, 2441, 5695, 8889) | a new template: map + census + emitter + oracle lines + year-package row | a cost unit (lens 3, class γ); until built, the gate that leads to it is a refusal or a class-(B) forgo with an advisory |
| **the filer's documents** (W-2 box 12 codes, 1099 boxes) | the document IS the unit; the W-2's back lists the codes | a (code, amount) row that maps or refuses — exactly today's `W2Box12` / `UnsupportedBox12Code` |
| **nothing but the filer** (occupation, estimated payments, "did you live apart from your spouse for the last 6 months") | the filer's own records | `collected` with source `FilerRecords` — a first-class source beside `Document{kind, payer}` |
| **Pub. 501 / Pub. 523 / Pub. 519** ("see Pub. 501 for details and examples") | the form has stopped and handed to a publication | the gate is asked; on the branch that enters the publication, **refuse with the exit named** — a compound reading of a publication is not a transcription |

### 1.4 The unit: a LINE, a DOCUMENT, or a LIFE EVENT?

**Neither one — the form uses two, and the interview should use the form's.**

- A **line** is too fine. Provenance is per field, questions are per gate, the link is many-to-one
  (`FIELD_PROVENANCE.md` §6c: *"Per-field questions are never needed"*). One "did you pay qualified
  passenger vehicle loan interest?" accounts for nine Schedule 1-A lines.
- A **life event** ("I sold my house") is too coarse and *not in the form's vocabulary*. The event spans
  Schedule D's Tests 1 and 2, a Form 1099-S, Form 8949 code H, and a prorated Schedule A line 5b — four
  gates with four legal predicates. A compound "no" to the event *"fabricates precision the filer never
  swore to"* (§6c, verbatim). It is also unbounded: the set of life events is whatever the filer's
  life contains; the set of gates is whatever the form contains.
- A **document** is the form's own unit for *amounts*: every money line on 1040 page 1 names one —
  *"Form(s) W-2, box 1"*, *"Form 1099-INT"*, *"Form SSA-1099"*. And a **gate** is the form's own unit
  for *conditions*: the instruction's skip logic.

So the unit is **two-kinded: DOCUMENT for amounts, GATE for conditions**, both read off the form. A
life event is at most a *help index* ("sold a home? → these three gates and this document") — a
documentation layer, never a data unit.

**So the spec should say:** the interview is the set of document screens (one per information-return
type the return accepts) plus the union of the two registries plus the section-level gates transcribed
from the instructions, and a KAT enumerates that set FROM the map census — every censused field names a
document field, a gate, a computation or a recorded refusal, and no `FieldId` exists that no census
entry reads. The generator is a conformance check; no code is generated from prose.

---

## 2. Lens 2 — document-first vs question-first

| | (a) type each document's boxes | (b) answer questions | (c) census → document screens → gates |
|---|---|---|---|
| **"an entry is testimony"** | best for amounts: the filer copies a number from a paper a third party issued; provenance is the document | worst for amounts: "how much interest did you receive?" invites an estimate from memory — weaker testimony under §6065 than a copied box | (a)'s amounts + (b)'s gates, with the census itself a tri-state so "none" is an answer and not an absence |
| **what it cannot do** | produce gates (no document carries "are you legally blind?" or "did all of the loan buy, build or improve the home?") | produce amounts honestly | — |
| **the oracles** | validates best: Tax-Calculator's variables ARE document boxes (`e00200` = W-2 box 1, `e00300` = 1099-INT box 1+3, `e18500` = Sch A 5b — `gen_goldens.py:199-232`) | gates are inputs the oracles take as given — the §G-9 limit (`two-oracle-model-and-venv`): never validated by their agreement | amounts validated by the oracles; gates validated by the transcription KAT against the form text |
| **the IRS's own design** | matches it: the form says "enter the amount from box 1" | matches it for the flowcharts (Step 1–4) | matches both halves |

**The codebase is already (c) in its types and (b) in its screens.** `Form1099Int` is
`payer, box1_interest, box2_early_withdrawal_penalty, box3_treasury_interest, box4_fed_withheld,
box6_foreign_tax, box8_tax_exempt_interest, box9_private_activity_bond_amt` (`return_inputs.rs:85-99`)
— box-named, with the destination line in each comment. The struct is a document screen with no
screen. The W-2 section is the one document that got one. The TOML fixture is a document-first record
already (`[[div_1099]] box1a_ordinary = "3000"` …, `fullreturn_inputs.toml:40-52`).

**The one trap in (c).** The census question ("which of these did you receive?") is itself
answered-ness by convention if it is a checklist whose unchecked state means *none*. It must be one
`TriState` per document type — *not asked / none / one or more* — under the §5.4 rule that *no renderer
may default-display `None` as "No"* (`SPEC_input_form.md` §5.4). A "not asked" row blocks commit as a
class-(A) declaration would; "one or more" opens the repeating section. That is the whole difference
between a document census and a settings page.

**A second rule that falls out.** Some lines the form attributes to the *filer's records*, not to a
document — line 26 *"2026 estimated tax payments and amount applied from 2025 return"*, Schedule A
line 5b from a property-tax bill, line 31's extension payment. Those are `collected` with source
`FilerRecords`, asked in the payments/deductions screens, and the difference is recorded, because a
figure with a document behind it and a figure from memory are different evidence in an examination.

**So the spec should say:** the interview runs census → document screens → gates, in the form's own
page order; every document type btctax accepts is a repeating section whose fields are the document's
boxes with the box captions verbatim; every money `Field` carries a `source` of `Document{kind,
payer_index}` or `FilerRecords`; no gate question ever takes a dollar amount.

---

## 3. Lens 3 — the gap map, ranked by 1040 lines unlocked

Cost classes: **α** = a `FormSpec` section over a struct that already exists (fields are TOML-only
today); **β** = a new struct + section + a printed-line change + map lines on a form btctax already
bundles; **γ** = a new form template (map, census, emitter, oracle lines, year-package row — and for
TY2026, a final that does not exist until Nov 2026 – Jan 2027).

| rank | gap (recon cite) | 1040 lines unlocked | cost | note |
|---|---|---|---|---|
| 1 | **interview-complete ≠ return-computable.** `commit` → `NoTables` on any year without `FullReturnParams`; only TY2024 has them (recon §A.0 ★, §G.2 #1) | all of TY2026 | process + α | the draft table already works on any year, and R6 already prints the crypto slice from a draft on a params-less year (`SPEC_1099da_broker_reporting.md` R6). What is missing is the *state*: the interview must be able to finish — every live gate answered, every declared document transcribed — on a year whose parameters arrive in January. Two states, shown at entry |
| 2 | **1099-INT / DIV / G / B sections** — 29 fields, four structs, zero Fields (recon §G.2 #4) | 2a, 2b, 3a, 3b, 7, 8, 10, 20, 21, 25b — **10** | α | six of the 17 `NotInForm` refusals live here (recon §E.3) |
| 3 | **dependents: entitlement** (recon §C) — and, new since the recon, **the TY2025+ grid itself** (§4.1 below) | 19, 28, the row boxes, HoH, Sch 3 L2 | β for the grid + Step 1–4; γ for Sch 8812 | the grid rows are a category-6 defect on the face of the owner's filed year (§4.1) |
| 4 | **retirement / Social Security** — 1099-R, SSA-1099, two worksheets (recon §G.2 #3) | 4a, 4b, 5a, 5b, 6a, 6b, 6c — **7**, + Sch 1 L20, Sch 2 L8 | β + worksheets | no new template: all seven lines are on the 1040. `design/ty2025/SPEC_retirement_income.md` is DRAFT r1, parked by D-C; joins v1 iff S2 says a 1099-R or SSA-1099 exists |
| 5 | **1099-NEC / MISC / K + Schedule C Part II** (recon §G.2 #5): 17 of 105 Sch C fields mapped, `expenses` one flat number | Sch 1 L3 → 8; Sch 2 L4; Sch 1 L15 → 10; Sch 3 L6b; Sch 1-A II/III | β | ★ P0 (`OWNER_DECISIONS_2026-09-04.md` D-B) puts *self-employment income* in the owner's set. A Schedule C printing line 28 with lines 8–27 blank is not "blank because the inputs say so" — it is a total with no lines behind it. If S2 confirms Sch C, Part II is one transcription struct of twenty money lines and is on the owner's path |
| 6 | **Schedule A 8b / 8c / 15 / 16 + Form 1098 as a document** (recon §B.5, §D.2) | Sch A only | α / β | a seller-financed loan (8b) needs the recipient's name and TIN; 1098 box 2 answers §163(h)(3)(B) with a figure where today a declaration stands in |
| 7 | **trailer**: direct deposit 35b–d, phone, spouse IP PIN, foreign address (recon §B.1 header/trailer) | 35b–d, 36 | α | the owner hits 35b–d on the first refund; today the refund is a paper check by advisory |
| 8 | HSA (8889), IRA deduction, student-loan interest section, education (8863), 2441, 8880, 5695 | Sch 1 L13/20/21; Sch 3 L2/3/4/5 | L21 α; the rest γ each | only on S2 evidence |
| 9 | **Schedule E, K-1, §121 / 1099-S / 8949 code H** | Sch 1 L5; Sch D 5/12; 8949 | γ | **refusal is the right v1** (lens 4) |

**Where "most of the return" stops for v1.** After ranks 1, 2, 3 (its β half), 6 and 7, the interview
is the sole or joint source for every Form 1040 line except 1b–1i (eight rare lines, all `X`), 4a–6c
(rank 4), 27–29 (γ credits) and 38 (Form 2210). Against recon §G.1's 60-line census that moves the
interview from 6 sole-source lines to roughly 20, and "cannot be taken at all" from 23 lines to ~14 —
of which seven are rank 4 and three are credits the form itself sends to another form. For the
*owner's* return under D-B's assumptions (no 1099-R, SSA-1099, rental or K-1), the stop is exactly
where the owner's shoebox stops, which is the S2 rule doing its job.

**So the spec should say:** v1 = ranks 1, 2, 3β, 6, 7 unconditionally; rank 5 iff S2 confirms a
Schedule C; rank 4 iff S2 confirms a 1099-R/SSA-1099; every γ item is a refusal with a kill until its
final is bundled; rank 9 is a refusal by design.

---

## 4. Lens 4 — dependents and real estate, specifically

### 4.1 Dependents — the form asks more than btctax collects, and since TY2025 it asks it on page 1

The recon measured TY2024: four columns plus two credit checkboxes per row, `Dependent { name, ssn,
relationship, date_of_birth }` (recon §C.1). **The TY2025 form redesigned the grid.** From the text
layer, `design/forms/extract/f1040--2025.txt:38-52`, verbatim row captions:

> (1) First name · (2) Last name · (3) SSN · (4) Relationship ·
> **(5) Check if lived with you more than half of 2025 — (a) Yes (b) And in the U.S.** ·
> **(6) Check if — Full-time student / Permanently and totally disabled** ·
> **(7) Credits — Child tax credit / Credit for other dependents**

Rows (5) and (6) are two of the §152 tests the recon listed as *"nothing in the codebase asks"* (recon
§C.2) — now printed as checkboxes on the return itself. The TY2025 `f1040` map is uncensused (196
fields in the `UNCENSUSED` register, recon §B.6), so the current struct's silence on rows (5)–(7) is
not yet a red test; it is category 6, *nothing ever decided*, on the first page of the first year the
owner files. This is the single most concrete thing the interview must add, and it is a transcription.

**What the instructions actually ask** (`i1040gi--2025.txt:1452-1560`, *Who Qualifies as Your
Dependent*): a flowchart in four Steps. Step 1's qualifying-child conditions, verbatim in shape:

> *A qualifying child is your… Son, daughter, stepchild, foster child, brother, sister, stepbrother,
> stepsister, half brother, half sister, or a descendant of any of them* **AND** *was… Under age 19 at
> the end of 2025 and younger than you … or Under age 24 … a full-time student … or Any age and
> permanently and totally disabled* **AND** *Who didn't provide over half of their own support for 2025
> (see Pub. 501)* **AND** *Who isn't filing a joint return for 2025 …* **AND** *Who lived with you for
> more than half of 2025 … check the "Yes" box (box (a)) on row (5)* … then *"1. Do you have a child who
> meets the conditions to be your qualifying child? Yes. Go to Step 2. No. Go to Step 4."*

Step 2 continues: citizen/national/resident-alien/Canada-Mexico → *"No. STOP — You can't claim this
child as a dependent"*; married → *"see Married person, later"*; joint return → … The edges are the
form's. Line 19's own instruction adds the SSN rule (`i1040gi--2025.txt:4148-4165`): *"you must have a
valid SSN, which means it must be valid for employment and issued before the due date of your return
(including extensions)"*, and hands the computation to Schedule 8812.

| per dependent | kind | source |
|---|---|---|
| relationship (from the form's list, not free text) | gate — `Enum` of the instruction's list | filer |
| age at year end / younger than you | computed from DOB (durable) | DOB is the one document-ish fact: birth certificate |
| full-time student / permanently and totally disabled — row (6) | gate ×2 | filer |
| provided over half of own support | gate | filer; "see Pub. 501" on the hard cases → refusal exit |
| filing a joint return / only for a refund | gate | filer |
| lived with you more than half the year, and in the U.S. — row (5)(a)(b) | gate ×2 | filer; *"Exception to time lived with you, later"* → refusal exit |
| citizen / national / resident alien / Canada / Mexico | gate | filer |
| married | gate | filer; yes → *"Married person, later"* → refusal exit |
| SSN valid for employment, issued before the due date incl. extensions | gate | the SSN card |
| **could this child be the qualifying child of any other person?** | gate | yes → *"Qualifying child of more than one person"* tie-breaker → **refusal**, exit Pub. 501 |
| divorced/separated parents (Form 8332) | gate | yes → **refusal** |
| CTC vs ODC — row (7) | **computed** from the above (age < 17 with a valid SSN ⇒ CTC column) and shown to the filer as the form's checkbox | — |

What is a question: every Step condition — they are yes/no with edges. What is a document: the SSN
card and the birth date. What is a refusal: every "see X, later" that leaves the flowchart for a
multi-page rule (tie-breaker, divorced parents, married dependent, the residency exception, multiple
support agreements). The interview asks the gate and refuses on the branch, naming the publication.

**Line 19.** Filling row (7) from the answers is honest and required by the form. Computing line 19
needs Schedule 8812 (γ) — until then the line stays blank **with the class-(B) state visible**
(`Advisory::CtcOdcOmitted` exists today, recon §B.1 line 19), which is a lawful forgo. What is not
lawful is today's state on a TY2025+ form: rows (5)–(7) blank because nothing asked.

**Head of household.** `FilingStatusArg::Hoh` is offered with *"no test at all"* (recon §C.3). The
instructions' HoH test is two gates — a qualifying person, and *"paid more than half the cost of keeping
up a home"* — both class (A): choosing HoH is an assertion, so an unanswered gate refuses.

### 4.2 Real estate — a document, four gates, and two refusals

**What exists** (recon §D.1): 5b real-estate tax, 5c personal-property tax, 8a mortgage interest *on a
1098*, the line-8 mixed-use box, the §163(h)(3)(B) ceiling as a declaration, the AMT-dwelling
declaration.

**Form 1098 as a document screen** replaces one hand-typed number with the lender's boxes: box 1
(interest — 8a), box 2 (outstanding principal — answers the §163(h)(3)(B) ceiling *with a figure*, where
today `MortgageWithinDebtLimit` asks the filer to assert it), box 3 (origination date — the
pre-12/16/2017 grandfathering the ceiling instruction turns on), box 6 (points), box 10 (real-estate
taxes when the lender pays them). ★ The TY2026 Schedule A draft is **REBUILT** (33 added, 19 removed —
`design/TY2026_WORK_LIST.md`); if the final restores a mortgage-insurance line where TY2024 reserved 8d
(recon §D.2), 1098 box 5 is its source, and a document screen picks that up as one field where a
question-first design would need a new question.

**Schedule A 8b and 8c** (`i1040sca--2025.txt:1100-1150`): interest not on a 1098 wants the recipient's
name, address and TIN — collected fields, α. **The 8396 subtraction** (`:1084-1097`) — *"subtract the
amount shown on Form 8396, line 3"* — is a gate ("are you claiming the mortgage interest credit?") whose
yes is a refusal until Form 8396 is a template.

**Sale of a main home.** The Schedule D instructions state the gate and the tests
(`i1040sd--2025.txt:313-352`), verbatim:

> *Report the sale or exchange of your main home on Form 8949 if: • You can't exclude all of your gain
> from income, or • You received a Form 1099-S for the sale or exchange.* … *Test 1 — During the 5-year
> period ending on the date you sold or exchanged your home, you owned it for 2 years or more … and
> lived in it as your main home for 2 years or more.* *Test 2 — You haven't excluded gain on the sale or
> exchange of another main home during the 2-year period …*

So the home sale is **four gates and one refusal**: sold a main home this year? → no ⇒ nothing (blank,
provenance recorded). Yes → Test 1, Test 2, received a 1099-S?, gain within the exclusion? (the
exclusion computation — adjusted basis, selling expenses, reduced exclusion, nonqualified use — is a
Pub. 523 worksheet, γ). Both tests met, no 1099-S, gain within the limit ⇒ the instruction's own
answer is *"you may not need to report"* — blank by decision, with the four answers on record. Any
other branch ⇒ **refuse**: *"btctax does not compute a home sale; Form 8949 code H and the Pub. 523
worksheet are the exit."* That refusal is correct, names its exit, and costs one struct of five
`Option<bool>`.

**Rentals, royalties, K-1.** Schedule E has no template, no struct, no line (recon §D.2). The right v1
is a **refusal at the document census**: the census row "Schedule K-1 / rental property" answered *one
or more* refuses with the exit (a preparer, for that year). Today the same fact is limb (a) of the
compound scope attestation (`questions.rs:548-560`); splitting it into typed census rows follows the
*enumerate the YES-conditions* rule — a filer cannot answer no to a category they were never shown, and
a typed row tells the tool *which* category, so the refusal can name the form.

**So the spec should say:** the dependents section is the TY2025+ grid transcribed (seven rows) plus
Step 1–4 as per-dependent gate questions with the instruction's own edges, an SSN-validity gate, and two
HoH gates live on `Hoh`; row (7) is computed from the answers; line 19 stays a visible class-(B) forgo
until Schedule 8812 is bundled. Real estate is a Form 1098 document screen, 8b/8c collected, the 8396
gate, the home-sale gate with its two tests → blank-by-decision or a refusal naming Pub. 523, and a
rental/K-1 census row that refuses.

---

## 5. Lens 5 — the exchange exports as interview input

The exports produce the 8949 and Schedule D (recon §A5, §E). What the interview must still ask, and
why software cannot answer it, is recon §E.2's table; the additions and the decision are below.

| still to ask | owner of the truth | where it belongs |
|---|---|---|
| Form 1099-DA per (provider, covered/noncovered) — `NotReported / ProceedsOnly / BasisMatches`; `BasisDiffers`/`Mixed` refuse (recon §E.2) | the filer, holding the broker's paper against btctax's column (e) | **return** — already the `BrokerReporting` section, rows seeded from the ledger |
| venue naming | the adapter — and it cannot: the account segment is hardcoded `default`, one provider = one wallet (recon §E.1) | **documentation only** until an export carries an account id; asking "how many accounts at X?" would collect an answer nothing can use |
| self-custody | the filer's wallets; no self-custody import exists, self-custody appears only as a counterparty (recon §E.1) | **ledger** — `link-transfer`, `classify-inbound-self-transfer`; the interview shows the count of unmatched outflows and hands off |
| gifts / donations | the filer (classification is ledger); the donee, the CWA, restrictions, the appraiser are return facts (Form 8283 §170(f)(8)/(11)) | **split**: `reclassify-outflow` + `set-donation-details` are ledger; `CharitableCwaObtained` / `DonationsHadRestrictions` are return skippables and already are |
| income classification (`Income{Reward|Interest}` → Sch 1 8v) | the filer, per event | **ledger** |
| the Notice 2026-20 standing order (T7) — per venue, before the first 2026 custodial disposal, no later than 2026-09-30 (`ROADMAP_STATUS.md` §0a T7) | the filer's books; a `MethodElection` event with `--exchange` scope | **ledger** — and in February 2027 it is too late to record for 2026 sales, so the interview can only **warn**: *"no standing order recorded for venue X before its first 2026 disposal — expect box 1g to reflect the broker's default; a `BasisDiffers` answer refuses"* |
| the 1040 Digital Assets question | the ledger when it has events; **the filer when it does not** — a holder with no 2026 receipts or disposals answers *No*, and the recon records that btctax *"never answered 'No'"* (§B.1) | **return** — a class-(A) declaration live when the ledger has no events for the year; the ledger answers it when it does |

**One interview or two?** Two question sets, one journey — argued from who owns the truth and from the
answered-ness classes:

- The ledger's questions are about **events**: *this outbound and that inbound are the same coins*. Their
  truth is in the filer's wallets, they are answered once per event, and they are **durable forever** —
  a 2024 transfer is still a transfer in 2030. Their gate is `BlockerKind` (23 variants) and it blocks
  *computation of the ledger*, not the return.
- The return's questions are about **a tax year**: *in this tax year, did…*. Every class-(A) declaration
  is `Durability::PerYear` by construction (`questions.rs:540-543`: *"every class-(A) DECLARATION
  asserts about a TAX YEAR … so none is durable"*). Their gate is `RefuseReason` (71) and it blocks
  *the return*.

Different durability, different scope, different store (the event ledger vs the per-year `ReturnInputs`
row), different failure modes (an inconsistency vs missing testimony). Merging the stores would put
per-event durable facts into a per-year row and re-ask them annually, or carry them silently — one of
which is waste and the other the §G-15 defect. So: **the interview never re-asks a ledger question.**
Its Step 0 is a *status panel* — blockers by kind, unresolved conflicts, venues with 2026 disposals and
no standing order, exports imported per venue — with the hand-off to `reconcile`; its crypto section is
seeded from the ledger (the `BrokerReporting` pattern) and asks only return-side questions.

**So the spec should say:** two registries, two stores, one sequenced journey. Step 0 of the interview
is the ledger status and hands off; the return interview seeds its per-venue rows from the ledger and
asks the 1099-DA answers, the Form 8283 declarations, and the Digital Assets declaration only when the
ledger has no events for the year. Venue/account granularity is documented as a limit, not asked.

---

## 6. Lens 6 — provenance and the year-N+1 re-interview

**What the data model has today:** per-year `ReturnInputs` rows (`tax_year` on the struct,
`return_inputs::get(conn, year)`); `Durability::{PerYear, Durable}` on both registries with the rule
*"a prior-year answer must NEVER silently satisfy this year's provenance … the lawful shape is a
confirmation, not a carry"* (`questions.rs:13-32`); `CarryProvenance` on the two carryforward vectors
(`return_inputs.rs:993, 1054`); the W-2's `EIN` as a durable employer identity. **What it lacks:** an
`answered_on`, a prompt hash, a `Declined` state distinct from never-asked (`FIELD_PROVENANCE.md` §6f —
*"a class-(B) skip and a never-asked question are both `None`"*), any source on an amount, and any
cross-year identity for a 1099 payer (`payer: String`, free text).

**The year-N+1 interview, walked.** Year N's record holds: the document census (which types, which
payers by TIN/EIN), every gate's answer with its date and the words asked, the carryforwards computed
by year N's return. Year N+1 opens with:

1. *"Last year you had a W-2 from EIN 12-3456789 (Acme). Did Acme issue you a 2027 W-2?"* — a fresh
   tri-state; **yes** opens a W-2 screen with the employer identity pre-named and **every box blank**.
   Durable identity, per-year amounts. Never a carried figure.
2. The same per payer for each 1099 type, and *"any payer not listed?"* — the census re-asked with last
   year's list as prompts, not as answers.
3. Every `PerYear` gate re-asked blank (blindness, foreign accounts, car-loan interest, the scope
   attestation). Every `Durable` fact (DOB, a dependent's SSN) shown and **confirmed by the same
   keystroke a fresh answer takes** (`questions.rs:29-32`).
4. Dependents: the prior year's rows as identities to confirm, with row (5)/(6) re-asked — a child who
   lived with you in 2026 may not in 2027, and age is recomputed.
5. Carryforwards **flow as data with provenance** — `CarryProvenance` gains
   `ComputedFromPriorReturn { year, hash }` beside `user`; a filer re-typing a carryforward the tool
   computed is the one place a carry is *worse* than the confirmation.
6. After a §G-14 shred, N+1 asks blank and says why before the shred, not after
   (`FIELD_PROVENANCE.md` §6d.3 ★).

**What the model needs, in one row shape** (§6f's, extended): per `(tax_year, FieldId+addr |
QuestionId)`: `value`, `answered_on`, `prompt_hash`, `source: Document{kind, payer_tin} |
FilerRecords | Ledger | ComputedFromPriorReturn{year}`, state `Given | Declined | Shredded{..}`. Per
document: `(kind, payer identity, tax_year)`. Nothing else — no progress, no "what remains", no
superseded values (§6f's forbidden list). ★ **Sequencing:** `answered_on` and the prompt hash *"cannot
be back-filled"* (§6f) — the schema must land **before the first TY2026 answer is stored**, or every
TY2026 answer's future tombstone carries a fabricated date. That puts the provenance schema ahead of
every section in lens 3, not after.

**So the spec should say:** every amount carries a `source`, every answer carries `answered_on` and
the prompt hash, `Declined` is a state; year N+1 opens with year N's document identities and durable
facts as confirmations that take a fresh keystroke each, re-asks every `PerYear` gate blank, and
receives carryforwards as computed data with provenance; the schema lands first.

---

## 7. Lens 7 — validation through the two oracles

**How the oracles are fed today.** Not from `ReturnInputs`. `gen_goldens.py` and the harness binary
take a *household dict* (`w2_income`, `taxable_interest`, `real_estate_tax`, `mortgage_interest` …):
Rust builds `ReturnInputs` from it (`testonly.rs:731 build_golden_return`, *"the oracle's `w2_income`
is a W-2's box 1 (and its box 3 / box 5"*), Python builds the Tax-Calculator row from the same dict
(`gen_goldens.py:199-232`), and OTS is driven by template fill (`ots_direct.py:135`). So today's oracle
input is an abstraction *upstream* of `ReturnInputs`, and a real return has no path to either engine.

**What the interview needs:** the reverse projection, `ReturnInputs → oracle row`. Because the
document-first fields are box-named and Tax-Calculator's variables are box-named, it is a table:
`e00200 = Σ w2s.box1_wages`, `e00300 = Σ int_1099.(box1+box3)`, `e00600/e00650 = Σ div_1099.box1a/1b`,
`e18400/e18500 = Sch A 5a/5b`, `e19200 = 8a`, `e19800 = cash gifts`, `p22250/p23250` from the ledger's
Schedule D. Its KAT: **every `Money` field either maps to an oracle variable or is listed as
oracle-invisible** — the §G-9 class (declarations, 1099-DA answers, every `Option<bool>`), which is
exactly the set the transcription KAT validates instead. The oracle run is local (OTS binary, taxcalc in
`.venv`) and the projection carries no identity, so a real return can go through it before export
without leaving the machine.

**Where an answer can be checked against a document — and the limit.**

| check | kind | why it is not a fill |
|---|---|---|
| W-2 box 4 ≈ 6.2% × box 3, box 6 ≈ 1.45% × box 5, box 3 ≤ the wage base | **warning** — arithmetic the W-2 itself obeys; a mismatch is a typo detector | the filer corrects the box; the tool never writes it |
| 1099-DA box 1d (proceeds) and box 1g (basis) vs btctax's column (d)/(e) per provider | **shown beside the question** | today the filer compares by eye against paper (recon §E.2). A 1099-DA *transcription screen* would let the tool compute and display the diff — the `BasisMatches`/`BasisDiffers` answer stays the filer's keystroke |
| 1099-INT box 1 vs Schedule B line 1 | identity — the same number | — |
| Σ 1099 box 4 + W-2 box 2 vs line 25 | computed | — |
| estimated payments vs a payment record | the filer's records | `FilerRecords` source; nothing to compare against |

The principle: *a document screen lets the tool check arithmetic the document guarantees and compare a
broker's figure against its own, and never lets it fill a line from the comparison.*

**So the spec should say:** a `ReturnInputs → oracle row` projection, one box-named table for
Tax-Calculator and the OTS template, with a KAT that every `Money` field maps or is listed
oracle-invisible; the real return runs the two-oracle harness before `export-irs-pdf`, disagreements
adjudicated against the form; document-internal arithmetic and broker-vs-ledger comparisons are
warnings displayed at the screen, never fills.

---

## 8. Lens 8 — the journey, February 2027 (solo walk)

**In hand:** a shoebox — one or two W-2s, a 1099-INT, perhaps a 1099-DIV/B, a Form 1098, the
property-tax bill, dependents' SSN cards and birth dates, estimated-payment confirmations, charitable
receipts (a BTC gift → Form 8283 Section B, appraisal), possibly a 1099-NEC, a 1098-E, a car-loan
interest statement; **four Form 1099-DAs** (the first basis-reporting year, arriving ~2027-02-16 per
the strategy review); four exchange exports (Coinbase CSV, Gemini XLSX, River CSV, Swan's three CSVs);
and the TY2025 return filed outside the project. Classification key: **R** refusal · **W** warning ·
**D** default · **N** not our concern · **Doc** documentation only. A divergence earns a requirement
only where the wrong outcome is worse than silence; those are numbered **J-n**.

| # | step — what they do | what the tool does today | what ELSE they might do → outcome | class |
|---|---|---|---|---|
| 0 | opens the TUI, `T`, year 2026 | fresh → filing status; 98 fields; `commit` → `NoTables` (recon §A.0) | fills everything in one sitting, hits `NoTables` at the end — the year's parameters arrive with the January package | **W → J-1**: at entry, show the year's state — *authoring and saving work; computing and committing wait for the TY2026 package (expected Jan 2027)*; interview-complete is a state the tool can reach and report before then |
| 1 | picks MFJ | accepted | picks **HoH** — accepted, nothing asked (recon §C.3) | **R → J-2**: two HoH gates, class (A) |
| 2 | *document census* (new) | — | marks 1099-B "none" because the broker has not mailed yet (Feb 15) | **D → J-3**: the census is tri-state; *not yet received* is **unanswered**, never *none*; unanswered blocks commit and is listed in the four-state panel |
| 2b | | | the ledger shows four venues with 2026 disposals; three 1099-DAs are in hand | **W → J-4**: expected-1099-DA rows seeded from the ledger; fewer answered than venues is a warning naming the venue |
| 3 | W-2 screen, types boxes | 14 Fields | types box 1 into box 3 | **W → J-5**: W-2 arithmetic warnings (lens 7) |
| 3b | | | box 12 code W (HSA) | **R** (exists: `HsaActivity` refuses) — the refusal must name the exit, which today is "not this tool, this year" |
| 3c | | | box 14 entries, state boxes 15–20 | **N** (17/19 already collected for SALT; the rest is state) |
| 4 | 1099-INT / DIV screens | **no section** — leave the TUI, hand-write TOML with no template, no import-time screen (recon §A3) | gives up, or types the interest into the wrong place | **worse than silence → J-6**: lens 3 rank 2 |
| 4b | 1099-DIV box 7 foreign tax $700 | `ForeignTaxOverCeiling` refuses (Form 1116) | — | **R** (exists, names the ceiling) |
| 5 | `btctax import` ×4 | four adapters, Swan as one batch | forgets River; imports a Coinbase file whose rows spill into Jan 2027 | **W → J-7**: Step 0 lists venues with 2026 events vs venues with a 1099-DA; a venue on a 1099-DA with no import is named. The spill is **N** (year filter is the ledger's) |
| 6 | `reconcile` | 27 subcommands; blockers (23 kinds) | Swan transfers-in lost their basis at ingest (recon §E.1 ★) → `UncoveredDisposal` / `FmvMissing` | **ledger, not the interview's** — but **J-8**: Step 0 shows the blocker census so the filer does not discover it at commit |
| 6b | | | no standing order was recorded before the first 2026 custodial sale (T7 was due 2026-09-30) | **W → J-9**: per-venue warning; the consequence (`BasisDiffers` refuses; per-lot import is the exit) is stated now, not at the 1099-DA row |
| 7 | 1099-DA rows per venue | seeded; `BasisDiffers`/`Mixed` refuse | box 1g differs because the venue used FIFO and the ledger HIFO | **R** (exists) — **J-10**: the refusal names `select-lots` / `import-selections` as the exit, and a 1099-DA screen (lens 7) shows the diff instead of asking the filer to compute it |
| 8 | dependents | four fields; rows (5)–(7) of the TY2025+ grid do not exist (§4.1) | a child born in 2026 with no SSN yet; a 22-year-old full-time student | **worse than silence → J-11**: the seven-row grid + Step 1–4; the SSN gate says *issued before the due date including extensions* — the extension-by-default calendar helps here, say so (**Doc**) |
| 8b | | line 19 never populated; `CtcOdcOmitted` fires | — | **W → J-12**: the class-(B) forgo must be visible with its size *before* filing (`FIELD_PROVENANCE.md` §5 ★) |
| 9 | Schedule A: 1098, tax bill, the BTC gift | 8a one number; 5b; 8283 skippables | a seller-financed second loan (8b) — silently absent | **worse than silence → J-13**: 8b/8c collected (rank 6); the 1098 screen |
| 9b | | Section B / appraisal above $5,000 is an owner action before filing | forgets the appraisal | **W** (exists as a refusal path via `set-donation-details`; the Step 0 panel lists it as *owner action, dated*) |
| 9c | | standard deduction wins → deletes Schedule A → `itemize_election` reset | — | **D** (exists, I-10) |
| 10 | "sold your main home?" (new) | nothing exists (recon §D.2) | yes, received a 1099-S | **R → J-14**: the four gates → blank-by-decision or a refusal naming Pub. 523 / code H |
| 10b | rental / K-1 census row (new) | limb (a) of the scope attestation refuses | — | **R → J-15**: typed census rows so the refusal names the form |
| 11 | payments | three Fields | types the 2027-01-15 Q4 payment (correct — it is a 2026 estimate); types state estimates paid in Jan 2027 into 2026's SALT (wrong — cash basis) | first **N**; second **W/Doc → J-16**: the 5a help text carries the instruction's *paid in 2026* wording |
| 11b | | extension payment | in February the 04-15 payment has not happened | **Doc**: the packet is exported after the extension payment is known; the draft/commit split already allows it |
| 12 | scope attestation | one compound prompt, four limbs (`questions.rs:548-560`) | a 1099-R → yes → refuse | **R** (exists). With typed census rows the attestation becomes the *residual* — keep it (widening YES-conditions is the safe edit) |
| 13 | commit | `screen_inputs` returns the **first** refusal (`return_refuse.rs:1034`, `Option<Refusal>`) | answers one, re-commits, hits the next — N round-trips (recon §G.3) | **worse than silence → J-17**: the four-state panel — not live / answered / (A) unanswered blocks / (B) unanswered forgoes — all at once |
| 14 | oracle run (new) | no path from a real return to either engine (lens 7) | skips it | **J-18**: the projection + a pre-export run; a disagreement is adjudicated against the form, never encoded |
| 15 | export, print, sign, Form 4868 + payment 04-15, mail by 10-15 | packet + manifest; 4868/1040-V built (FR-49) | forgets to sign | **Doc/W**: the census's `not-ours: filer-by-hand` carries a *"must be completed before filing"* duty (`FIELD_PROVENANCE.md` §6c 4a); the export report lists the by-hand cells |

**Divergences that earn nothing:** the box-14 entries; the January spill rows; the Q4 payment date;
the appraisal timing beyond the existing owner-action row. **Every J-n above is a spec requirement**,
and eleven of the eighteen are *missing things at moments* — a step that is silent today — which is the
class a correctness review of `SPEC_input_form.md` could not have reached.

**So the spec should say:** J-1 … J-18 as numbered requirements, each with the step that produced it,
and the walk is re-run live with the owner on the TY2025 rehearsal documents (S1) before the TY2026
spec closes — that walk will diverge in ways this one did not.

---

## 9. Lens 9 — what to STOP / not build

| seductive thing | why it fails the rule | what to do instead |
|---|---|---|
| **An extractor (OCR / an LLM) that reads the shoebox and fills boxes** | a value from a guess about a scan is fabricated testimony at scale, and the oracles take it as input — the one class nothing validates (§G-9) | if a scan is ever shown, it is displayed *beside* the box; the filer types. Not v1 |
| **"Smart defaults" / carrying last year's amounts** | *"a prior-year answer must NEVER silently satisfy this year's provenance"* (`questions.rs:21-25`); a carried amount is the answered-ness defect one year removed | confirmations with fresh keystrokes; identities carry, amounts never (lens 6) |
| **A schema-driven generic form builder** (JSON schema → UI) | the vetoed `serde_json::Value` path reflection — *"it reintroduces the stringly-typed null-vs-absent laundering P9 exists to abolish"* (`SPEC_input_form.md` §4) — and a *"third representation to keep in sync"* (`SPEC_input_surface.md` §4); it also hides the form's numbering, which is the review gate | the hand-written `FormSpec` with box-named Fields and the coverage KAT; the form's numbering *is* the schema |
| **A code generator that emits questions from instruction prose** | lens 1.1: the label reader needed three layout facts for *digits*; prose gates need a human predicate; a generator would produce confident wrong questions with no red test | transcribe; prove with the census KAT |
| **A progress bar** | flattens class (B) into class (A) — *"skipping the car-loan question is not an error, it is a decision to leave a deduction on the table"* (`FIELD_PROVENANCE.md` §5 ★) | the four-state panel, with the forgone amount where computable |
| **A life-event wizard as the data model** | not the form's vocabulary; a compound "no" fabricates precision (§6c); unbounded | a help index from events to gates and documents — documentation only |
| **Persisting progress / "what remains" / WIP blobs for fresh authoring** | *"a second copy of the liveness predicate"* (§6f); a draft *"is an EDIT TRANSACTION, not a save file"* | commit per atomic decision; derive the rest |
| **Building Schedule E / K-1 / 8863 / 2441 / 5695 ahead of S2** | forms outside the owner's set are *"refusals with kills instead of transcriptions"* (S2, `ROADMAP_STATUS.md` §0a); each is a γ on a calendar that has no slack | refusal rows that name the exit |
| **Re-asking ledger questions in the return interview** | two owners of truth, two durabilities (lens 5) | Step 0 status + hand-off |
| **A 1099-DA adapter that auto-answers the broker questions** | the answer is a comparison the filer swears to | a 1099-DA transcription screen may compute and *show* the diff; the answer is a keystroke |
| **A web front-end before the TUI journey has been walked once** | the seam exists (`seam.rs`, serde, *"the web wire"*); the value is in the questions, not the renderer | walk S1 in the TUI; the web renderer is a second consumer of a proven `FormSpec` |
| **e-file, state returns** | closed by IRS rule / out of scope (brief) | — |

**So the spec should say:** the interview fills nothing the filer did not type or the ledger did not
compute; no answer is pre-filled from any prior year; no representation of the form exists outside the
Rust structs and the map census; the only automation is arithmetic the document itself guarantees and
comparison the tool can show.

---

## 10. Ranked candidate scope for v1

**Caveat first (S2):** the owner's real 2026 return defines "done". Every conditional row below flips on
the S2 answer; nothing conditional is built ahead of it.

| # | item | class | why this rank |
|---|---|---|---|
| 1 | **Provenance schema** — `source` on amounts, `answered_on` + prompt hash on answers, `Declined` state, payer identity on documents (lens 6) | schema | cannot be back-filled (§6f); every-year machinery, which outranks any single year (`ROADMAP_STATUS.md` §0) |
| 2 | **Interview-complete vs return-computable**, shown at entry; Step 0 ledger/owner-action status panel (J-1, J-4, J-7, J-8, J-9) | process + α | unblocks authoring TY2026 in Sep–Dec 2026, which is the only slack the calendar has |
| 3 | **Document census** (tri-state per type) + **1099-INT / DIV / G / B sections** + trailer fields (35b–d, phone, spouse IP PIN, foreign address) (J-3, J-6) | α | ten 1040 lines for the cost of sections over structs that exist |
| 4 | **Dependents: the TY2025+ seven-row grid + Step 1–4 gates + SSN gate + HoH gates**; row (7) computed; line 19 a visible forgo (J-2, J-11, J-12) | β | a category-6 defect on page 1 of the owner's year; dependents are in P0 |
| 5 | **The four-state answer panel** — `screen_inputs` reports all, not the first (J-17) | engine | the recon's own diagnosis of the round-trip problem (§G.3) |
| 6 | **Form 1098 screen; Sch A 8b/8c; the 8396 gate; the home-sale gate + tests; rental/K-1 census refusal** (J-13, J-14, J-15) | α/β | real estate as the owner named it; two refusals that name their exit |
| 7 | **`ReturnInputs → oracle row` projection + pre-export two-oracle run** (J-18) | harness | the real return has no path to either engine today |
| 8 | **Schedule C Part II as a transcription struct + a 1099-NEC/MISC/K screen** — **iff S2 confirms Schedule C** | β | P0 says SE income; a line 28 with no lines 8–27 behind it is not a blank-by-inputs |
| 9 | **1099-R / SSA-1099 screens + the two worksheets** — **iff S2 confirms one** | β | spec DRAFT r1 exists; no new template |
| 10 | **Schedule 8812** (line 19/28) | γ | first γ, because it is the one with the dependents grid already feeding it |
| — | 8863 / 2441 / 8880 / 5695 / 8889; Schedule E; K-1; §121 computation | γ / refusal | on S2 evidence only; Schedule E and K-1 stay refusals |

Ranks 1–7 are v1. Items 8–9 join v1 on S2. Item 10 is the first post-v1 form.

---

## 11. Open questions for the owner (six; each with the answer that changes the design)

1. **S2, sharpened to documents.** For 2026: any Form 1099-R or SSA-1099? A Schedule C with expenses
   beyond one line, or a 1099-NEC/MISC/K? A Form 1098? Dependents — how many, any full-time student aged
   19–23, any born in 2026? A home sale? A K-1 or rental? — *Each yes moves exactly one row of §10 into
   or out of v1; a rental or K-1 means "preparer for that year", not "build it".*
2. **Hard gate or warning at Step 0?** Should the return interview refuse to open its crypto section
   until the ledger has zero blockers, or allow authoring in parallel with the blocker census shown? —
   *Hard gate is simpler and fail-closed; parallel is what the Sep–Dec calendar needs if the exports
   arrive late. Recommendation: parallel with the panel, hard gate at commit.*
3. **Retention and confirmations.** Should year N+1 open with year N's document identities and durable
   facts as confirmations (lens 6), and how long are answers retained after filing before shred? —
   *"Blank re-ask every year" removes the payer-identity and prompt-hash requirements from rank 1;
   confirmations keep them. The strategy's north star (every year) argues for confirmations.*
4. **Line 19 in v1.** Accept a visible class-(B) forgo of the child tax credit until Schedule 8812 is
   transcribed (γ, needs the TY2026 final), or make 8812 rank 8? — *The owner's dependents decide the
   money at stake; up to $2,000 a child on the extension deadline may be worth one γ.*
5. **Form 1099-DA as a transcription screen.** Type the broker's boxes (1d, 1g, box 2) so the tool
   shows the diff against columns (d)/(e), or keep today's compare-by-eye answer? — *A screen adds one
   struct and makes J-10 a computed warning; it also lets proceeds reach the oracle projection.*
6. **Is the S1 rehearsal the interview's first live walk?** — *Recommended yes: the TY2025 documents in
   Sep–Dec 2026 are the only way to find the missing-at-moments (lens 8) before the ten-week window
   that also holds finals, OTS 2026 and the first 1099-DAs. If S1 stays declined, the first walk is the
   owner's own February 2027 return and every J-n found then lands on the extension.*

---

## 12. What the recon measured — the table this document relied on

| measurement | value | recon cite |
|---|---|---|
| TUI form: sections / Fields | 16 / 98 (coverage KAT pins 98) | §A1, M1, M5 |
| registry questions (`income answer`) | 17 declarations + 19 skippables = 36 | §A2, M7 |
| years on which `commit` succeeds | TY2024 only (`NoTables` otherwise) | §A.0 ★, M3 |
| `income import` screens at import | `screen_inputs` appears 0 times in `import_return_inputs` | §A3 |
| `ReturnInputs` top-level fields | 40 | §A3, M6 |
| import-only structs / fields | `int_1099` 8, `div_1099` 12, `g_1099` 3, `b_1099` 6; `schedule_1a` 23 leaves | §G.1 |
| Form 1040 TY2024 lines by source | I 6 · T 7 · L 2 · T+L 1 · C 19 · never-populated 1 · X 23 · A 1 (of 60) | §G.1 |
| TY2024 packet fields | 819 mapped + 543 censused = 1,362; 512 `unmodeled`, 31 `artifact`; 0 unaccounted | §B.6, §G.1, M2/M4 |
| TY2025 uncensused | 5 maps / 310 fields (`f1040` 196) | §B.6 |
| TY2026 | no maps; `forms_expected = []` | §B.6 |
| `Dependent` fields | 4 (name, ssn, relationship, DOB); no §152 test asked | §C.1–C.2 |
| `RefuseReason` / `NotInForm` anchors | 71 / 17 | §E.3 |
| `BlockerKind` / `Advisory` / `EventPayload` | 23 / 21 / 22 | §E.3 |
| `reconcile` subcommands / adapters | 27 / 4 | §A5, §E |
| `BrokerReported` variants | 5 (2 refuse) | §E.2 |
| the missing join | field→question / question→field: absent; `screen_inputs` returns the first refusal only | §G.3 |

**Measured here, not in the recon** (the recon was TY2024-scoped): the TY2025 Form 1040 dependents
grid carries rows (5) lived-with-you (a)/(b), (6) student/disabled, (7) credits —
`design/forms/extract/f1040--2025.txt:38-52`; the dependents flowchart is at
`design/forms/extract/i1040gi--2025.txt:1452-1560` and line 19 at `:4148`; the Schedule D home-sale
gate and tests at `design/forms/extract/i1040sd--2025.txt:313-352`; the oracle harness takes a
household dict, not `ReturnInputs` (`scripts/oracle/gen_goldens.py:199-232`,
`crates/btctax-core/src/tax/testonly.rs:731`); the scope attestation's four limbs at
`crates/btctax-core/src/tax/questions.rs:548-560`.
