# SPEC — the INTERVIEW that fills the return

**Status: DRAFT r1** (Fable, lens 2 of 3). **Written 2026-09-06 at `c5af3f60` (`main`).** Brief:
`design/agent-reports/BRIEF-spec-interview.md`. Inputs, in the order read: the recon
`design/agent-reports/2026-09-07-interview-recon.md` (cited *recon §X*; measured at `36bddc75` — the four
commits since are design documents only, so its code cites resolve unchanged at HEAD), the brainstorm
`design/BRAINSTORM_interview.md` (cited *brainstorm §X*), `design/SPEC_input_surface.md` (superseded, kept
for its answered-ness classes and the three refusal classes), `design/SPEC_input_form.md` (GREEN r5 — the
`FormSpec` seam this extends), `CLAUDE.md`, `STANDARD_WORKFLOW.md` §2 (S6), `design/ROADMAP_STATUS.md` §0a.

**Process (S6):** ONE independent review round → persist verbatim → ledger → fold → build, one opus agent
per task under a persisted brief. Every rule below names its mechanism and its kill; every task is
sized for one agent. Nothing in this document was executed; every claim about the tree cites a
`file:line` that exists at HEAD.

**Where the brainstorm hedged, this spec decides** (each decision says why): the unit is
document + gate (§3 R1); "derived" is a census kill, not a generator (R2); the census is one tri-state
per document type stored as class-(A) declarations (R3); the provenance schema lands first and is
*structural* — sources by struct, dates and prompt hashes per answer — not a per-leaf metadata table
(R10); Step 0 is a panel with parallel authoring and a hard gate at commit (R9); year N+1 opens with
confirmations, never carries (R10); line 19 stays a visible forgo until Schedule 8812 (R6, owner Q2);
the 1099-DA answer stays a keystroke and gets no transcription screen (R9); the CLI twin is `income
answer` + `income import`, not a third renderer (§4.2); `screen_inputs` is **not** refactored to collect
refusals — the four-state panel is derived from the registries instead (R12).

---

## 1. Why this exists

The owner's ask, verbatim: *"Can we design an interview process to elicit income data, deduction
related information, real estate, dependence, etc., as well as exports from Bitcoin exchanges to allow
the interview process to fill out most of the tax return, if not all of it?"*

What exists is an engine with almost no questions in it. Measured (recon §A.0, §G.1):

- **Five surfaces, one interview.** The TUI "tax inputs" form has **16 sections / 98 Fields**
  (`crates/btctax-input-form/src/spec/mod.rs:19-38`; the count is pinned at
  `crates/btctax-input-form/src/spec/coverage.rs:479-485`), and its `commit` returns `NoTables` on every
  year without bundled `FullReturnParams` (`crates/btctax-cli/src/input_form_store.rs:369-376`) — which is
  every year but TY2024 (`crates/btctax-adapters/src/tax_tables.rs:102,108`). **The only year a filer can
  author interactively is a year nobody is filing.** TY2026, the target (`ROADMAP_STATUS.md` §0a), can
  receive a return only through `income import`'s TOML (`crates/btctax-cli/src/cmd/tax.rs:49`), which
  has no template and no import-time screen.
- **Of the 60 Form 1040 entry lines, the interview is the sole source for 6** (1a, 12, 25a, 25c, 26,
  31); 7 are TOML-only (2a, 2b, 3a, 3b, 10, 20, 25b); 2 come from the ledger; 19 are computed; **one is
  never populated (19, the child tax credit)**; **23 cannot be taken at all** (1b–1i, 4a–6c, 27–29,
  35b–d, 36, 38) (recon §G.1).
- **The information returns are structs with no screen.** `Form1099Int` / `Form1099Div` / `Form1099G` /
  `Form1099B` (`crates/btctax-core/src/tax/return_inputs.rs:85-187`) are box-named — *"box1_interest //
  → 1040 2b / Sch B"* — and have **zero** form Fields; the coverage KAT exempts them by prefix
  (`coverage.rs:327`). A W-2 filer with a savings account cannot use the form at all (recon §G.2 #4).
- **Dependents are collected for identity and never for entitlement.** `Dependent { name, ssn,
  relationship, date_of_birth }` (`return_inputs.rs:224-228`); nothing asks a §152 test (recon §C.2);
  the CTC/ODC row boxes are mapped and *"deliberately NOT checked"*
  (`crates/btctax-forms/src/form1040_full.rs:539`). And since TY2025 the form itself prints three of the
  tests as checkboxes on page 1 — rows (5) lived-with-you (a)/(b), (6) student/disabled, (7) credits
  (`design/forms/extract/f1040--2025.txt:38-52`).
- **Real estate is one hand-typed number.** `mortgage_interest_1098: Usd` (`return_inputs.rs:613`);
  Schedule A 8b/8c/15/16 are censused `unmodeled` (`crates/btctax-forms/forms/2024/f1040sa.map.toml:99-102`);
  no 1098, no §121, no Schedule E (recon §D).
- **The exchange side is complete and separate:** four adapters, 27 `reconcile` subcommands, 23
  `BlockerKind`s (`crates/btctax-core/src/state.rs:23`), the 1099-DA answers already a seeded section
  (`spec/mod.rs:33`). What is missing is the *seam*: nothing tells the return interview what the ledger
  still needs.
- **71 `RefuseReason`s, 17 of them anchored `NotInForm`** (recon §E.3; `crates/btctax-input-form/src/attribute.rs:254-265`)
  — seventeen refusals the form cannot let a filer answer. Their notes are this spec's to-do list.

The repo has already written the inversion and not built it: *"the FORMS derive the INTERVIEW"*
(`design/forms/FIELD_PROVENANCE.md:78`), with its two named blockers — `screen_inputs` returns the
**first** refusal only (`:105-118`), and nothing links field → question (`:96-103`).

---

## 2. Scope

### 2.1 IN (v1) — set by the S2 rule, inside it by 1040 lines unlocked

v1 is everything the brainstorm ranked 1–7 (brainstorm §10), which is: *every line the Form 1040 and its
Schedules 1/2/3/A/B/D carry that needs no new form template*, plus the provenance schema that must land
before the first TY2026 answer is stored.

| # | item | class | rules |
|---|---|---|---|
| 1 | provenance schema — sources by struct, `answered_on` + prompt hash per answer, `Declined`, payer identity | schema | R10 |
| 2 | interview-complete vs return-computable at entry; the Step 0 ledger/owner-action panel | process + α | R9, R11 |
| 3 | the document census (tri-state per type) + 1099-INT / DIV / B / G / 1098-E sections + trailer (35b–d, phone, spouse IP PIN, foreign address) | α | R3, R4 |
| 4 | dependents: the TY2025+ seven-row grid, Steps 1–5 as per-dependent gates, the filer-TIN gate, HoH gates; row (7) computed; line 19 a visible forgo | β | R6, R7 |
| 5 | the four-state answer panel — every blocking and forgoing item at once | engine | R12 |
| 6 | Form 1098 as a document; Schedule A 8b / 8c collected; the 8396 gate; the home-sale gates; the rental / K-1 census refusals | α / β | R8 |
| 7 | `ReturnInputs → oracle row` projection + a pre-export two-oracle run | harness | R13 |

**Conditional on S2** (built only when the owner's real 2026 shoebox says so — owner Q1): Schedule C
Part II as a transcription struct + a 1099-NEC/MISC/K screen (T13); 1099-R / SSA-1099 screens with
their two worksheets (T14; `design/ty2025/SPEC_retirement_income.md:1-5` is DRAFT r1, parked by D-C
`design/OWNER_DECISIONS_2026-09-04.md:87`).

### 2.2 OUT — every excluded family, with the sentence a filer who has it is told

**Rule: a filer who holds an excluded document is never under-filed silently.** Each family below is a
typed row of the document census (R3) or an existing declaration; answering *one or more* **refuses**
with the sentence shown, naming the exit. The refusal is `RefusalKind::UNSUPPORTED` in
`SPEC_input_surface.md` §3.5's vocabulary: the data is true, btctax's scope is short, nothing is stored.

| excluded family | 1040 / schedule lines it would reach | refusal sentence (the exit) |
|---|---|---|
| **Form 1099-R** (IRA, pension, annuity) | 4a/4b, 5a/5b; Sch 1 L20; Sch 2 L8 | *"btctax cannot take a Form 1099-R for this year: Form 1040 lines 4a–5b and the Simplified Method are not built (T14, on S2). File with a preparer, or wait for T14."* |
| **Form SSA-1099 / RRB-1099** | 6a/6b/6c | *"btctax cannot take Social Security or railroad retirement benefits: the Social Security Benefits Worksheet is not built (T14, on S2)."* |
| **Form 1099-NEC / 1099-MISC / 1099-K** | Sch 1 L3 (via Sch C), Sch 2 L4, Sch 1 L15; Sch 1-A Parts II/III | *"btctax cannot take non-employee compensation this year: Schedule C Part II and the 1099-NEC/MISC/K screen are not built (T13, on S2). A crypto-only Schedule C still fills from the ledger."* (matches `attribute.rs:262,265`) |
| **Schedule K-1** (any) | Sch 1 L5; Sch D L5/L12; Sch 3 L6l | *"btctax cannot take a Schedule K-1: partnership, S-corporation, estate and trust items reach nothing. A preparer is the exit for this year."* |
| **rental real estate / royalties (Schedule E)** | Sch 1 L5; Form 8960 L4a | *"btctax has no Schedule E. A rental or royalty is a preparer's return for this year."* |
| **Form 1099-S / a home sale you cannot fully exclude** | Form 8949 code H; Sch D | *"btctax does not compute a home sale: Form 8949 code H and the Pub. 523 worksheet are the exit."* (R8) |
| **Form 1099-OID** | 2a/2b via Sch B | *"btctax takes Form 1099-INT only; a 1099-OID's boxes differ (box 1 OID, box 8/11 tax-exempt). Enter with a preparer or wait for its screen."* |
| **Form W-2G** (gambling) | Sch 1 L8b; Sch A L16; 25c | *"btctax cannot take gambling winnings or losses."* |
| **Form 1099-C** (canceled debt) | Sch 1 L8c | *"btctax cannot take canceled debt (Form 982 is not built)."* |
| **Form 1095-A** (Marketplace) | Sch 2 L1a; Sch 3 L9 | *"btctax cannot reconcile the premium tax credit (Form 8962 is not built)."* |
| **Form 1098-T** (education) | Sch 3 L3; 1040 L29 | *"btctax cannot take education expenses (Form 8863 is not built)."* |
| **HSA / IRA contributions** | Sch 1 L13 / L20 | existing: `HsaActivityUnsupported`, `IraDeductionClaimed` (recon §B.7) — unchanged |
| **the child tax credit amount** (Schedule 8812) | 1040 L19, L28 | **not a refusal — a visible forgo** (R6): the row-(7) boxes are printed from the answers; line 19 stays blank with `Advisory::CtcOdcOmitted` (`crates/btctax-core/src/tax/advisories.rs:53`) sized in the panel. Owner Q2 |
| EIC, 2441, 8880, 5695, 8936, 8396, 8801, direct-deposit-to-IRA, Form 2210, Form 8888, line 36 | 27, Sch 3 L2/4/5a-b/6b/6f/6g/6m, 35a, 36, 38 | existing `unmodeled(advisory)` census entries and `EicOmitted` / `OtherCreditsOmitted` (`advisories.rs:59,169`) — unchanged; **the 8396 gate is new** (R8) and refuses on yes |
| a foreign-country reading (Pub. 501 / 519 / 523 branches) | — | every "see *X*, later" that leaves a flowchart for a multi-page rule refuses naming the rule (R6, R8) |

**Where "most of the return" ends.** After v1, the interview is the sole or joint source for every
Form 1040 line except 1b–1i (eight rare lines, all `X`), 4a–6c (T14), 27–29 (credits the form sends to
another form) and 38 (Form 2210). Against the recon's 60-line census that moves the interview from 6
sole-source lines to roughly 20, and "cannot be taken at all" from 23 lines to about 14 (brainstorm §3).
For the owner's return under D-B (`OWNER_DECISIONS_2026-09-04.md:61`) — no 1099-R, SSA-1099, rental or
K-1 — the stop is exactly where the shoebox stops. **The exit at the stop is always a named refusal.**

### 2.3 Not this spec

The crypto engine, the 1099-DA rules R1–R6, the packet, the oracles' code, the frozen files, the
`reconcile` command set (§10). A web renderer (the seam is data — `seam.rs:205-207,301-303` — and a second
consumer drops in later). E-file, state returns.

---

## 3. The rules

### R1 — The unit is two-kinded: a DOCUMENT for amounts, a GATE for conditions. Never a line, never a life event.

**Mechanism.** Every money line on Form 1040 page 1 names a document — *"box 1a of Form(s) 1099-DIV"*
(`design/forms/extract/i1040gi--2025.txt:2611-2618`), *"box 8 of Form 1099-INT"* (`:2448-2459`); every
condition is a skip instruction or a flowchart edge — *"Yes. Go to Step 2. No. Go to Step 4."*
(`:1525-1529`). So the interview is (i) one **document screen** per information-return type the return
accepts, whose fields are the document's own boxes with the box captions verbatim; plus (ii) the union of
the two registries that exist (`FORM_QUESTIONS` 17 at `crates/btctax-core/src/tax/questions.rs:286`,
`SKIPPABLE_QUESTIONS` 19 at `:985`) with the new gates this spec adds (census rows, dependents, HoH, home
sale, 8396, digital assets); plus (iii) the **filer's-records screens** for the amounts the form
attributes to no document (line 26's *"estimated federal income tax payments you made"* `:4261-4268`;
Schedule A 5b from a tax bill `i1040sca--2025.txt:693-700`). A life event is at most a help index — a
documentation layer mapping "sold a home" to three gates and one census row — never a data unit
(brainstorm §1.4: a compound "no" *"fabricates precision the filer never swore to"*,
`FIELD_PROVENANCE.md:289-291`).

**Kill.** No `Field` of kind `Money` exists outside a document section or a filer's-records section:
the `LEAF_SOURCE` KAT (R10) reds on a `Usd` leaf whose prefix names no source. No gate takes a dollar:
every registry entry is `Option<bool>` / `Option<Date>` (the `decl_tristate!` / `skippable_*` shapes,
`crates/btctax-input-form/src/spec/registries.rs:35-137`); a `Money` field wired through a registry macro
does not compile.

### R2 — "Derived from the form" means transcribed by a human and PROVED complete by a census. No generator.

**Mechanism.** The instructions are prose flowcharts; nothing mechanical turns *"Who didn't provide over
half of their own support for 2025 (see Pub. 501)"* into a predicate (brainstorm §1.1). So a human
transcribes each gate and each document from the text layer, and three instruments prove nothing was
skipped:

1. **Box captions are checked verbatim.** `Production::Collected`
   (`crates/btctax-core/src/tax/line_coverage.rs:79-81`) already exists for printed lines; `xtask
   line-coverage` checks each printed line's quotation against `design/forms/extract/`
   (`crates/xtask/src/main.rs:211-220`, `crates/xtask/src/line_coverage_check.rs:1-12`). This spec adds
   the document side: every `Collected` line names the **document box** it is collected from
   (`from: DocBox { stem, box, extract_line }`), and the checker verifies the box caption — the
   `Field.help` — verbatim against that document's archived extract. The information returns are
   **not archived at HEAD** (`design/forms/extract/` holds the 1040 family, 8283/8949/6251/8995/8615/8275
   and Schedule 8812 for 2024 only); T2 archives `f1099int`, `f1099div`, `f1099g`, `f1099b`, `f1098`,
   `f1098e`, `fw2` and their `iNNNN` per `design/forms/README.md:14-25` (periodic cadence, note + text
   layer, no PDF committed).
2. **Every unmodeled line names what tells the filer.** The `[census]` sections
   (`crates/btctax-forms/tests/field_census.rs:167-177`, text-scanned) carry `rule = "unmodeled"` with a
   free-text `reason`. On the forms the interview reaches (f1040, f1040s1/s2/s3, f1040sa, f1040sb,
   schedule_d) every `unmodeled` entry gains `covered_by = "Advisory::X" | "RefuseReason::Y" |
   "QuestionId::Z"`, and a KAT joins each against the enum's own source (the `classifier.rs` technique of
   reading both sources) — a line btctax cannot take is thereby either **announced** or **refused**,
   never silent. This is §2.2's table made mechanical.
3. **Every gate is reachable and every Field is read.** The coverage KAT
   (`coverage.rs:240`, `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt`) stays the
   forward direction; the reverse — no `FieldId` exists that no printed chain, screen rule, or census
   entry reads — is the `LEAF_SOURCE` join (R10).

**Kill (B1, seen red first).** Plant: (a) change one document `Field.help` by one character → `line-coverage`
red; (b) delete `covered_by` from one `unmodeled` entry, or point it at a variant that does not exist →
the census join red; (c) add a `Usd` leaf to a struct with no `LEAF_SOURCE` prefix → red. Each planted in
the kill test *before* the checker's first green, in the `field_census.rs:329` style
(`the_gate_reds_on_every_planted_defect`).

### R3 — The document census: one TRI-STATE per document type, stored as class-(A) declarations. "Not yet received" is unanswered, never "none".

**Mechanism.** `ReturnInputs.documents: DocumentCensus` — one `Option<bool>` per document type in §5.1,
each a `FormQuestion` in `FORM_QUESTIONS` (so `screen_inputs` refuses on a live `None`,
`income answer` asks it, the Declarations adapter renders it — zero new plumbing). The prompt is the
form's own attribution: *"Did you receive one or more Form 1099-INT (each payer should send you one —
Form 1040 line 2b instructions)?"* Semantics:

- `None` — not asked / not yet received. **Blocks commit** (class A) and is listed in the panel (R12).
  A broker that has not mailed by February is *unanswered*, not *none* (J-3).
- `Some(false)` — none. The section is non-live; the type's `Vec` **must be empty** — a `No` with rows
  present is `RefuseReason::DocumentCensusContradicted { kind }` (INVALID), and `apply` refuses the
  `SetField(No)` while rows exist (the renderer removes rows first, with a payload-confirm; never a
  silent delete — the `DeleteSection(ScheduleA)` I-10 precedent, `SPEC_input_form.md` §5.1).
- `Some(true)` — one or more. The section is live; **zero rows refuses** `DocumentDeclaredNotTranscribed
  { kind }` (UNANSWERED class): a declared document with no transcription is exactly *"nothing ever
  populated it"*.
- For an **unsupported** type (§2.2) `Some(true)` refuses UNSUPPORTED with the exit sentence; the row
  exists so the filer is shown the category and can answer it — *"a filer cannot answer no to a category
  they were never shown"* (brainstorm §4.2).

This changes one thing about the W-2: today `w2s: Vec<W2>` empty means either *no W-2* or *never asked*
— the answered-ness trap at the money level. The `w2` census row makes them distinct. TY2024 fixtures
gain `documents.w2 = Some(true)` (the compiler-and-KAT blast radius, by design).

The compound scope attestation `OtherOutOfScopeIncome` (`questions.rs:546-560`) is **kept unchanged** as
the residual: the census rows are additive, a filer with a 1099-R now refuses twice (row and attestation),
and *widening an exemption is never the safe edit*. The 1099-DA gets **no** census row: the per-provider
`BrokerReporting` answers (`crates/btctax-core/src/forms.rs:272-287,339`) are already a finer census
(`NotReported` is "no 1099-DA for these rows"), seeded from the ledger (`spec/mod.rs:33,44-49`).

**Kill.** (a) A fresh return with `documents.int_1099 = None` refuses `DocumentCensusUnanswered`; (b)
`Some(true)` + empty `int_1099` refuses; (c) `Some(false)` + one row refuses, and `apply(SetField(No))`
over a row is `ApplyError`; (d) every unsupported row's `Some(true)` refuses with a message containing
the exit's form number; (e) the classifier compiles only once every `DocumentCensus` leaf is classified
class (A) with its `QuestionId` (`crates/btctax-core/src/tax/classifier.rs:5-9` — no `..`, no `_`).

### R4 — Document screens: the document's boxes, verbatim, each reaching a named line. The tool checks arithmetic the document guarantees and never fills a line from the check.

**Mechanism.** One repeating section per supported document type; per row the payer identity plus the
boxes. Reach, from the 1040 text (all cites `design/forms/extract/`):

| document | boxes collected (struct today) | reaches | cite |
|---|---|---|---|
| **W-2** (exists, 14 Fields + box 12) | 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 17, 19 (`return_inputs.rs:36-80`) | 1a, 25a, 8959, Sch A 5a, §6413(c) | `i1040gi--2025.txt:2184` |
| **1099-INT** | payer, **payer TIN (new)**, 1, 2, 3, 4, 6, 8, 9 (`:85-99`) | 2b (1+3, via Sch B 1), Sch 1 L18 (2), 25b (4), §904(j) FTC (6), 2a (8), refuse (9) | 2a `:2448-2459`; 2b `:2485-2492`; Sch B 1 `i1040sb--2025.txt:58-60`; box 9 `:163-164`; 25b `i1040gi--2025.txt:4225-4241` |
| **1099-DIV** | payer, TIN, 1a, 1b, 2a, 2b, 2c, 2d, 4, 5, 7, 12, 13 (`:104-126`) | 3b (1a), 3a (1b), Sch D 13 (2a), refuse (2b/2c/2d/13), 25b (4), 8995 (5), FTC (7), 2a (12) | 3a `:2508-2516`; 3b `:2611-2618`; box 12 `i1040sb--2025.txt:163` |
| **1099-B** | payer, TIN, ST/LT proceeds and basis, `basis_reported_and_no_adjustments` (`:156-187`) | Sch D 1a/8a; the gate refuses `Form1099BNeedsForm8949` (`attribute.rs:254`) | `return_inputs.rs:140-152` quotes Sch D 1a |
| **1099-G** | payer, TIN, 1, 4 (`:131-135`); **box 2 (new)** + the gate *"did you itemize on your prior-year return?"* | Sch 1 L7 (1), 25b (4); **box 2: `No` ⇒ Sch 1 L1 blank-by-decision; `Yes` ⇒ refuse** naming the State and Local Income Tax Refund Worksheet until it is transcribed (owner Q1 decides whether that worksheet joins T5) | L7 `i1040gi--2025.txt:42005`; L1 `:41894-41897` |
| **1098** (new, R8) | lender, TIN, 1, 2, 3, 4, 5, 6, 7/8 property, 10 | Sch A 8a (1 + 6), the §163(h)(3)(B) ceiling check (2, 3), 8d if a final reinstates it (5) | `i1040sca--2025.txt:1058-1068` |
| **1098-E** (new) | lender, TIN, 1 | Sch 1 L21 (replaces `sch1.student_loan_interest_paid`, `return_inputs.rs:689`) | Sch 1 L21 chain `printed.rs:483-484` |

Box captions come from the document's extract (T2), one `Field` per box named for the box
(`Box1Interest`, …), the caption verbatim as `help` (the transcription rule, `CLAUDE.md`; the struct
*is* the form). Checks the tool may run and **display** (brainstorm §7): W-2 box 4 ≈ 6.2% × box 3, box
6 ≈ 1.45% × box 5, box 3 ≤ the wage base — **warnings**, typo detectors; the filer corrects the box, the
tool never writes it. The **payer TIN** is the cross-year identity (R10); a document row's every box is
testimony by construction, because the row exists only because the filer declared the document (R3) —
a 1099-INT with box 2 = 0 is the *document's* zero, so no `Option<Usd>` is needed inside a document row.

**Kill.** Coverage KAT: `EXEMPT_PREFIXES` (`coverage.rs:327`) loses `int_1099`, `div_1099`, `g_1099`,
`b_1099`, `sch1.student_loan_interest_paid`; the pinned Field count rises from 98 and the exempt count
is asserted **≤ its new value** (a ratchet — a task may only shrink it). Attribution: the six `NotInForm`
anchors in this cluster (`PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`,
`InconsistentDividendSubset`, `ForeignTaxOverCeiling`, `Form1099BNeedsForm8949`, `IraDeductionClaimed`
is not — see R8) become `Field`/`Section` anchors and a test asserts the `NotInForm` count fell by
exactly that number. The W-2 arithmetic warning fires on a planted box-1-in-box-3 fixture and writes
nothing.

### R5 — Amounts the form attributes to the filer's own records are collected as `FilerRecords`, and the difference is recorded.

**Mechanism.** Line 26 (`i1040gi--2025.txt:4261-4268`), line 31's extension payment, Schedule A 5b/5c
(`i1040sca--2025.txt:693-700`), 8b/8c (R8), the charitable rows, the dependents' facts: these have no
issuing third party. They live in structs whose `LEAF_SOURCE` entry (R10) is `Source::FilerRecords`,
and their help text carries the instruction's own timing words — 5a's *"paid in 2026"* (J-16). A
figure with a document behind it and a figure from memory are different evidence in an examination;
the source is recorded so the packet's manifest can say which is which.

**Kill.** `LEAF_SOURCE` completeness (R10). The 5a help text contains the instruction's *paid in*
wording, checked by `line-coverage` like any other quotation.

### R6 — Dependents: the TY2025+ grid transcribed, Steps 1–5 as per-dependent gates with the instruction's own edges, row (7) computed, line 19 a visible forgo.

**Mechanism.** Per dependent row (`Dependent`, `return_inputs.rs:224-228`), in addition to the four
identity fields, the gates below — every one `Option<bool>` (class A: live ⇒ must be answered), phrased
as the instruction phrases it, with the edge the instruction gives. They form a new per-row registry
`DEPENDENT_GATES: &[DependentGate]` in core (same shape as `FormQuestion` — prompt, `live(&ReturnInputs,
row)`, get/set over `&Dependent`, the refusal, `Durability`), so that `screen_inputs` loops rows × gates
(refusing `DependentGateUnanswered { row, gate }` on a live `None`), `live_questions` enumerates them
(`crates/btctax-cli/src/cmd/answer.rs:52`), and the input form adapts them into the `Dependents` section
per row.

| step | gate (prompt as the instruction phrases it) | edge | cite `i1040gi--2025.txt` |
|---|---|---|---|
| 1 | `qc_relationship` — *son, daughter, stepchild, foster child, brother, sister, stepbrother, stepsister, half brother, half sister, or a descendant of any of them?* | No ⇒ Step 4 | `:1463-1466` |
| 1 | **age test — COMPUTED** from `date_of_birth` (now required for the credit path), `full_time_student`, `permanently_and_totally_disabled` (row 6), and the taxpayer's/spouse's DOB (*"younger than you"*) | DOB absent ⇒ the test is unknown ⇒ no credit column, class-(B) forgo with the advisory naming the missing DOB | `:1487-1499` |
| 1 | `provided_over_half_own_support` — *did this person provide over half of their own support?* | Yes ⇒ Step 4 | `:1502` |
| 1 | `filing_joint_return`; if Yes, `joint_return_only_to_claim_refund` | joint and not refund-only ⇒ Step 4 | `:1506-1508` |
| 1 | `lived_with_you_over_half_year` — **row (5)(a)** | No ⇒ Step 4 (the *Exception to time lived with you*, `:1905`, is never applied — conservative) | `:1512-1516` |
| 1 | `lived_with_you_in_us` — **row (5)(b)** | printed | `f1040--2025.txt:44-46` |
| 1 | `qualifying_child_of_another_person` — the CAUTION | Yes ⇒ **REFUSE** naming *Qualifying child of more than one person* (`:1967`) and Pub. 501 | `:1519-1521` |
| 2 | `citizen_national_resident_or_canada_mexico` | No ⇒ **REFUSE** *"You can't claim this child as a dependent"* — remove the row | `:1540-1547` |
| 2 | `married` | Yes ⇒ **REFUSE** naming *Married person* (`:1945`) | `:1548-1553` |
| 2 | *filing a joint return* — computed from `filing_status` | — | `:1559-1568` |
| 2 | *could you be claimed* — the existing return-level `DependentTaxpayer` declaration (`return_inputs.rs:262`) | `Some(true)` ⇒ **REFUSE** *"You can't claim any dependents"* | `:1571-1589` |
| 3 | `tin_issued_by_due_date` — *SSN, ITIN, or ATIN issued on or before the due date (including extensions)?* | No ⇒ no credit box (forgo, not refusal) | `:1639-1656` |
| 3 | `citizen_national_or_resident_alien` (narrower than Step 2's) | No ⇒ no credit box | `:1594-1604` |
| 3 | *under age 17 at the end of the year* — computed from DOB | No ⇒ **ODC box** | `:1607-1616` |
| 3 | `ssns_valid_for_employment_issued_by_due_date` — *you, your spouse if joint, and this child* | Yes ⇒ **CTC box**; No ⇒ Step 5 ⇒ ODC box | `:1619-1631` |
| 4 | `qr_relationship_or_member_of_household` (the Step 4 list, or *lived with you all year as a member of your household*) | No ⇒ **REFUSE** — not a dependent; remove the row | `:1662-1680` |
| 4 | `qualifying_child_of_any_taxpayer` | Yes ⇒ REFUSE (not a qualifying relative) | `:1683-1687` |
| 4 | `gross_income_under_limit` — the figure is a `FullReturnParams` value (TY2025: $5,200) | No ⇒ REFUSE; disabled + No ⇒ REFUSE naming *Exception to gross income test* | `:1690-1691` |
| 4 | `you_provided_over_half_support` | No ⇒ REFUSE | `:1694-1696` |
| 4/1 | `divorced_separated_multiple_support_or_kidnapped_rule_applies` | Yes ⇒ **REFUSE** naming the three rules (`:1823, :1949`) | `:1696` |
| 4 | citizen / married / joint / could-you-be-claimed — the Step 2 gates reused | as Step 2 | `:1700-1735` |
| 5 | return-level `filer_tin_issued_by_due_date` — *did you, and your spouse if joint, have an SSN or ITIN issued on or before the due date?* — a `FormQuestion` live iff any dependent row exists | No ⇒ no ODC box | `:1743-1750` |
| 5 | qualifying relative's TIN / citizen / married — the Step 3.1 and Step 2 gates reused | as above | `:1765-1790` |

**Row (7) is computed**, never asked: CTC when the Step 1–3 chain passes; ODC when the chain reaches
*"Credit for other dependents"*; both blank otherwise. **Rows (5) and (6) print from the gates.** The
emitter checks the row boxes from the computation (replacing `form1040_full.rs:539`'s deliberate blank)
on the TY2025+ grid; TY2024's grid has no such rows and is unchanged.

**Line 19 stays a forgo, visibly.** `ctc_odc_line19` (`advisories.rs:757-760`) already prints `Some(0)`
only where §24(b) provably zeroes it and `None` otherwise with `CtcOdcOmitted` (`printed.rs:588-596`).
Computing the amount is Schedule 8812 (γ, T15). Until then the panel (R12) lists *"child tax credit not
computed — n children with a credit box; up to $X each (params)"* with the size from the year's
parameters, and blank on a params-less year. Printing a checked row-(7) box beside a blank line 19 is
what the form instructs on the box and says nothing false on the line (the credit is claimed on 8812,
which is not attached); the advisory tells the filer what they forgo (owner Q2 decides whether 8812
joins v1).

**Kill.** A truth-table KAT per flowchart edge: for each row in the table above, a fixture dependent at
that edge yields the instruction's outcome (Step 4, refusal-with-that-exit, CTC box, ODC box, no box).
Every gate: `None` while live ⇒ `DependentGateUnanswered`; every REFUSE row's message contains the named
rule. The classifier reds until each new `Dependent` leaf is classified. A `Single` return with no
dependents asks no dependent gate and not `filer_tin_issued_by_due_date` (liveness). The emitter KAT:
on a TY2025 fixture, the six row-(5)/(6) checkboxes and two row-(7) boxes per dependent are filled from
the answers; on TY2024 the emitter output is byte-identical to today.

### R7 — Head of household is an assertion; choosing it asks the instruction's two tests.

**Mechanism.** `FilingStatusArg::Hoh` is offered with *"no test at all"* (recon §C.3). Three
`FormQuestion`s live iff `filing_status == Hoh` (`return_inputs.rs:936`):
`hoh_unmarried_or_considered_unmarried` (the three bullets, `i1040gi--2025.txt:1143-1163` — a married
filer answering No refuses naming *Married persons who live apart*, `:1247`); `hoh_qualifying_person`
(*Test 1 or Test 2 applies*, `:1164-1200`); `hoh_paid_over_half_cost_of_keeping_up_home` (`:1164,1172`).
Any `No` on the second or third refuses `HohTestNotMet` — *"choose another filing status"*. The name
entry for a non-dependent qualifying child (`:1196-1200`) is a `Text` field live iff HoH and no dependent
row is the qualifying person — collected, printed in the entry space.

**Kill.** `Single`/`Mfj` never ask; `Hoh` with any `None` refuses; `Hoh` with `Some(false)` on a test
refuses with the exit; the no-brick property (`answer.rs:412`) covers the three.

### R8 — Real estate: a document, four gates, two refusals.

**Mechanism.**

- **Form 1098 is a document screen** (`form_1098: Vec<Form1098>`, top-level — it arrives whether or
  not the filer itemizes). Boxes: lender, lender TIN, **1** interest → Schedule A 8a with **6** points
  (`i1040sca--2025.txt:1058-1061` — *"mortgage interest and points reported to you on Form 1098"*);
  **2** outstanding principal and **3** origination date → the §163(h)(3)(B) ceiling is now **checked
  with a figure** ($750,000 / $1,000,000 by whether box 3 precedes 2017-12-16, both `FullReturnParams`
  values) and displayed as a **warning** beside the existing `MortgageWithinDebtLimit` declaration
  (`return_inputs.rs:647`), which stays the filer's testimony; **4** refund of overpaid interest → shown
  with the instruction's pointer to Sch 1 L8z (`:1069-1072`; not computed — the census entry names it);
  **5** mortgage insurance → collected against the reserved 8d (`:1153-1155`; if the TY2026 final
  reinstates the line — the draft Schedule A is REBUILT, `design/TY2026_WORK_LIST.md:36` — the field is
  already there); **7/8** the property address; **10** other (informational, shown beside 5b).
  `ScheduleAInputs.mortgage_interest_1098` (`:613`) is **removed**; 8a = Σ(box 1 + box 6) over the rows;
  the `MortgageAllUsed` / `AmtQualifiedDwelling` / `MortgageWithinDebtLimit` liveness keys on
  `!form_1098.is_empty()` instead of `> 0`. Compile errors enumerate every reader.
- **Schedule A 8b** — interest paid to a recipient who gave no 1098: a repeating
  `mortgage_interest_not_on_1098: Vec<{ recipient_name, recipient_tin, recipient_address, amount }>`
  on `ScheduleAInputs`, because the instruction demands the recipient's *"name, identifying number, and
  address on the dotted lines"* (`i1040sca--2025.txt:1109-1116`) — the four TY2024 map cells
  `f1_17`/`f1_19` become mapped (`f1040sa.map.toml:99-100`). **8c** — points not on a 1098: one `Usd`
  (`f1_18`/`f1_20`, `:101-102`), with the instruction's *"generally deductible over the life of the
  loan"* (`:1137-1139`) in its help. 8e = 8a + 8b + 8c (the printed chain, today `= 8a`).
- **The Form 8396 gate** — *"If you are claiming the mortgage interest credit … subtract the amount
  shown on Form 8396, line 3"* (`:1091-1096`): `FormQuestion` `claiming_mortgage_interest_credit`, live
  iff any 1098 or 8b row; `Some(true)` refuses `MortgageInterestCreditUnsupported` (Form 8396 is not
  bundled).
- **Sale of a main home** — three gates and the 1099-S census row, transcribed from the Schedule D
  instructions (`design/forms/extract/i1040sd--2025.txt:313-347`): `sold_main_home` (always live);
  if Yes: `test1_owned_2_years_and_lived_2_years_of_last_5` (`:335-343`),
  `test2_no_exclusion_on_another_home_in_2_years` (`:344-348`), `can_exclude_all_gain` (*"You can't
  exclude all of your gain from income"*, `:321-322`). **All three Yes and the 1099-S row `No`** ⇒
  the instruction's own answer, *"You may not need to report the sale"* (`:315-316`): **blank by
  decision**, the four answers on record. **Any other branch** ⇒ `HomeSaleNotComputed`, *"btctax does
  not compute a home sale; Form 8949 code H and the Pub. 523 worksheet are the exit."* No amount is ever
  asked.
- **Rentals, royalties, K-1** — census rows (R3) that refuse with §2.2's sentences. No struct, no line.

**Kill.** 8e = 8a + 8b + 8c on a fixture with one of each; the two-oracle sweep still reconciles
(`e19200` absorbs 8b/8c — R13). The ceiling warning fires on box 2 = $900,000 with box 3 in 2019 and is
silent at $700,000; the declaration still refuses on `None`. The 8396 gate refuses on Yes. The home-sale
branch table: 8 combinations of the three gates × the census row — exactly one (Y,Y,Y,No) is blank, the
rest refuse with *Pub. 523* in the message. `NonForm1098Interest` with an empty `recipient_tin` refuses
(the $50-penalty clause, `:1117-1119`).

### R9 — The exchange side: two question sets, two stores, one journey. The interview never re-asks a ledger question.

**Mechanism.** The ledger's questions are about **events** (*this outbound and that inbound are the
same coins*), durable forever, gated by `BlockerKind` (23, `state.rs:23`), answered in `reconcile` (27
subcommands, recon §E.2). The return's questions are about **a tax year**, `Durability::PerYear` by
construction (`questions.rs:540-543`), gated by `RefuseReason`. Different durability, different scope,
different store — so:

- **Step 0 of the interview is a STATUS PANEL, not a question set**, computed by a `btctax-cli`
  function over the held session: blockers by kind; unresolved conflicts; imports per venue; venues
  with dispositions in the year versus providers with a `BrokerReporting` answer (J-4, J-7); **venues
  with a custodial disposition in the year and no `MethodElection` with `--exchange` scope effective
  before it** — the Notice 2026-20 §4.02(2) standing order (T7, `ROADMAP_STATUS.md` §0a; the text at
  `legal/text/irs-guidance/Notice_2026-20.txt`) — with the consequence stated now: *"expect box 1g to
  reflect the broker's default; a `BasisDiffers` answer refuses; `select-lots` / `import-selections` is
  the exit"* (J-9, J-10); and the owner actions with dates (the Section B appraisal, J-9b). Every row
  hands off to the `reconcile` command that answers it. **Authoring proceeds in parallel with an
  unresolved ledger; commit and export stay hard-gated** (decided — the Sep–Dec calendar needs parallel
  authoring; the existing gates already refuse at commit/export).
- **The crypto section asks only return-side questions**, seeded from the ledger exactly as
  `BrokerReporting` is today (`spec/mod.rs:33,44-49`): the 1099-DA answers per provider (kept as a
  keystroke — `BasisMatches` / `BasisDiffers` is a comparison the filer swears to; **no 1099-DA
  transcription screen in v1**, decided: R6 is green and the diff display would add a struct for a
  warning the panel can already state), the Form 8283 declarations (`DonationsHadRestrictions`,
  `CharitableCwaObtained` — exist).
- **The Digital Assets question** (`i1040gi--2025.txt:1346-1362`) becomes a class-(A) `FormQuestion`
  `digital_asset_activity`, always live, PerYear — *"At any time during the year, did you (a) receive
  … or (b) sell, exchange, or otherwise dispose of a digital asset?"* — and a `screen_compute_dependent`
  rule (`crates/btctax-core/src/tax/return_1040.rs:907`, the tier that sees the ledger) cross-checks it:
  ledger has events ∧ `No` ⇒ refuse `DigitalAssetAnswerContradictsLedger`; ledger empty ∧ `Yes` ⇒ refuse
  *"import your exchange exports"*; the two agreeing cells print. Today the box is never "No" (recon
  §B.1); a holder with no 2026 receipts or disposals can now answer it.
- **Venue/account granularity is documented, not asked**: the account segment is hardcoded `default`
  (`crates/btctax-adapters/src/normalize.rs:63-69`); asking "how many accounts at X?" would collect an
  answer nothing reads.

**Kill.** The DA four-cell table as a test. The standing-order warning fires on a fixture ledger with a
2026 Coinbase disposal and no scoped election, and is silent with one effective the day before. A
venue present in the year's 8949 rows and absent from the answers is named in the panel. No
`reconcile` question appears in any registry (grep-KAT: no `FormQuestion` prompt contains *transfer*,
*lot*, *FMV*).

### R10 — Provenance is STRUCTURAL: the source of an amount is the struct it lives in; every answer carries its date and the words asked; `Declined` is a state; year N+1 opens with confirmations and never a carried figure.

**Mechanism.** Four parts, all on `ReturnInputs` (one blob, one row, one `SCHEMA_VERSION` — bumped
2 → 3, `crates/btctax-cli/src/return_inputs.rs:24`; older rows refuse-and-reimport per `:59-63`; a WIP
draft discards, a parked one refuses, per `input_form_store.rs:182-192`).

1. **Source by struct.** A static core table `LEAF_SOURCE: &[(&str, Source)]` maps every serde leaf
   prefix to `Source::Document(DocumentKind) | FilerRecords | Ledger | Answer | Computed`. A KAT
   (sibling of the coverage walk, `coverage.rs:42-68`) asserts every `Usd` / `Option<Usd>` leaf of a
   maximal fixture matches exactly one prefix, and every prefix matches at least one leaf (the stale-
   exemption discipline, `coverage.rs:415-429`). No per-leaf metadata: a box on a declared document is
   testimony because the row exists (R3/R4).
2. **Identity per document.** `payer_tin: String` on every information-return struct (`#[serde(default)]`),
   `ein` already on the W-2 (`return_inputs.rs:58`). `transcribed_on: Option<Date>` per row.
3. **Per answer.** `ReturnInputs.answer_log: BTreeMap<AnswerKey, AnswerRecord>`, where `AnswerKey =
   Question(QuestionId) | Skippable(SkippableId) | DependentGate { row, gate }` (a stable string form
   for serde) and `AnswerRecord { answered_on: Date, prompt_hash: String, state: Given | Declined }`.
   Written by **one** core function `record_answer(ri, key, prompt, now)` that both `apply`
   (`crates/btctax-input-form/src/apply.rs:28`) and `income answer` call; `now` is the `BTCTAX_NOW`
   seam (`crates/btctax-cli/src/main.rs:64-76`). A skippable **skipped on purpose** records `Declined`;
   `None` with no record is *never asked* — the distinction `FIELD_PROVENANCE.md:395-398` says the
   record cannot make today. Forbidden, per `:400-403`: progress, position, "what remains", superseded
   values, half-typed tokens.
4. **Year N+1.** Opens with year N's **identities** (each payer by TIN, each dependent, each venue) as
   fresh tri-state prompts — *"Last year Acme (EIN 12-3456789) issued you a W-2. Did Acme issue one for
   2027?"* — Yes opens a screen with the identity pre-named and **every box blank**; every `PerYear`
   gate re-asked blank; every `Durable` fact (a DOB, `questions.rs:940`) shown and confirmed by the same
   keystroke a fresh answer takes (`:29-32`); carryforwards arrive as data — `CarryProvenance`
   (`return_inputs.rs:575`) gains `ComputedFromPriorReturn { year }` beside `User`/`Computed` (and
   closes §G-23's "stated zero", `FOLLOWUPS.md:1998`). **Never a carried amount** — *"a prior-year answer
   must NEVER silently satisfy this year's provenance"* (`questions.rs:21-25`). Retention after filing is
   the filer's decision, not a window this spec picks (`FIELD_PROVENANCE.md:443-446`); shred is §G-14,
   not built here.

**Kill.** `LEAF_SOURCE` completeness both directions. `record_answer` via `apply` and via `income
answer` produce byte-identical `AnswerRecord`s for the same answer at the same `BTCTAX_NOW`. A skipped
skippable yields `Declined`; an untouched one yields no record. A version-2 committed row refuses with
`StaleReturnInputs`; a version-2 WIP draft is discarded with a note; a version-2 parked draft refuses.
The N+1 opener: a fixture year N with two payers yields exactly two identity prompts, both `None`, and
the opened screen's boxes are all default. Grep-KAT: no field named `progress`, `remaining`, `position`
on `ReturnInputs`.

### R11 — The year gate: interview-complete is a state the tool reaches and reports on a year whose parameters have not arrived; commit still waits for them. A long-lived draft is protected.

**Mechanism.** `commit` keeps I-11 (`input_form_store.rs:369-376`): a params-less year writes nothing,
because `screen_inputs` needs `&TaxTable + &FullReturnParams` (`return_refuse.rs:1034`) and an
unscreened row at precedence 1 (`crates/btctax-cli/src/resolve.rs:72-84`) is the poison
`SPEC_input_surface.md` §3.2 describes. What changes:

- **Two states at entry**, both derivable without params: *interview-complete* — `interview_state(ri)`
  (R12) has no blocking item; *return-computable* — `YearReadiness.params`
  (`crates/btctax-cli/src/year_readiness.rs:28`), already rendered by `sentence()` (`:115`). The entry
  screen says which of the two the year has, and for TY2026: *"authoring and saving work; computing and
  committing wait for the TY2026 package (expected Jan 2027)"* (J-1). The param-dependent rules
  (§402(g), SALT, excess SS, the §152(d) gross-income figure) run at commit.
- **The draft is the Sep–Dec store for TY2026** (`save_draft` works on any year, `:140`). The
  coherence rule (`SPEC_input_form.md` §6.2; `input_form_store.rs:298-314`) today *notes* and deletes a
  non-default WIP draft on any committed-row write. A draft holding an interview is not disposable:
  **a WIP draft with any `answer_log` entry or any document row is discarded only on confirmation**
  (`--force` on the CLI; a payload-confirm in the TUI), never on a note.
- **`income answer` answers into the draft when the year has a draft and no committed row.** Today it
  refuses on a missing row (`answer.rs:117-121,131-137`) to avoid materializing a near-empty blob at
  precedence 1 — the draft is invisible to `resolve.rs` (`input_form_store.rs:1-2`), so writing the
  draft carries none of that hazard. It still refuses when neither exists.
- The slice-filing path for TY2026 (spec 1099-DA R6, `design/SPEC_1099da_broker_reporting.md:278`) is
  unchanged: a committed row via `income import`, or the TUI `commit` once `FullReturnParams` TY2026 is
  inserted (the *filable* flip, `crates/btctax-forms/forms/2026/YEAR.toml`).

**Kill.** On a params-less year: `commit` → `NoTables`, writes nothing (exists); `save_draft` persists
across `Session::open` (exists, I-7); `income import` over a draft with one answered census row and no
`--force` refuses and the draft survives; with `--force` it is deleted and a note names what was lost;
`income answer` on a draft-only year writes the draft and `return_inputs::get` still returns `None`;
`interview_state` on a TY2026 fixture with every gate answered reports complete while
`YearReadiness.params` is false.

### R12 — The four-state panel is derived from the registries, not from `screen_inputs`. Class (B) is never flattened into class (A).

**Mechanism.** `SPEC_input_form.md` §7 forbids refactoring `screen_inputs` to collect all refusals
(its early-return tiers are semantic) — kept. The panel needs none of it: `live_questions` already
enumerates every live question across both classes (`FIELD_PROVENANCE.md:108-110` — *"the primitive
exists"*). A core function `interview_state(ri) -> InterviewState { blocking: Vec<Blocking>, forgoing:
Vec<Forgo>, answered: usize, not_live: usize }` walks `FORM_QUESTIONS`, `SKIPPABLE_QUESTIONS`, the
census rows, `DEPENDENT_GATES` × rows, and the declared-document/rows invariant (R3):

| state | listed as |
|---|---|
| not live | counted, silent |
| live, answered | counted |
| live, unanswered, **class (A)** | **blocking** — the question, its anchor, and what it accounts for |
| live, unanswered, **class (B)** | **forgoing** — the benefit, its size where computable (the §63(f) add-on from params; line 19 per R6; blank on a params-less year) |

No progress bar (`FIELD_PROVENANCE.md:123-125`). The TUI renders it as a pane; `income answer` prints
it before the first question and after the last; the commit modal prints the forgoing list
(J-12, J-17). The no-brick property (`answer.rs:412`) extends: answering every blocking item through its
own setter empties `blocking`, and on a params-bearing year `screen_inputs` then reports no
UNANSWERED-class refusal.

**Kill.** A fixture with N unanswered live gates across all four registries lists exactly N blocking
items in one call. A live skippable with `Declined` is neither blocking nor forgoing. The extended
no-brick test. A snapshot with the forgo size present when params exist and absent when they do not.

### R13 — The oracle path: a box-named projection from `ReturnInputs` to the oracle row, run locally before export; a disagreement is adjudicated against the form, never encoded.

**Mechanism.** The two oracles are fed a *household dict* today, not `ReturnInputs`
(`scripts/oracle/gen_goldens.py:199-232` builds the Tax-Calculator row; `crates/btctax-core/src/tax/testonly.rs:731`
builds `ReturnInputs` from the same `GoldenInputs`, `:650`; OTS is a template fill,
`scripts/oracle/ots_direct.py:135`). A real return has no path to either engine. This spec adds the
reverse: `project_to_golden(ri: &ReturnInputs) -> GoldenInputs` in core (box-named both sides:
`e00200 = Σ w2s.box1`, `e00300 = Σ int_1099.(box1+box3)`, `e00600/e00650 = Σ div_1099.box1a/1b`,
`e18400/e18500` = Sch A 5a/5b, `e19200` = 8a + 8b + 8c, `e19800` = cash gifts, `e02300` = Σ g_1099.box1,
`p22250/p23250` from the ledger's Schedule D), a CLI `btctax income project --year N` that prints it
(no identity — the projection carries no PII by type), and `scripts/oracle/check_return.py` that runs
the harness (`scripts/oracle/sweep.py:18`, `cargo build -p btctax-oracle-harness`) and both oracles on
it and diffs the compared lines. **Excuses are computed from mechanism** (`CLAUDE.md`): line 19 is
expected to differ by exactly the oracle's CTC while 8812 is unbuilt; the §G-9 limit stands — gates
are inputs the oracles take as given and are validated by R2's transcription checks instead.

**Kill.** `project_to_golden(build_golden_return(g).0) == g` on every golden household in
`crates/btctax-core/tests/goldens/` (projection inverse). `ORACLE_INVISIBLE` — every `Usd` leaf either
projects or is listed (declarations, 1099-DA answers, every `Option<bool>`) — asserted complete inside
the KAT. The CTC excuse is `oracle_line19 - 0` and a diff of any other size on line 19 fails.

### R14 — Answered-ness is structural: every new field joins a classifier class by construction.

**Mechanism.** The classifier destructures every struct with no `..` and forbids `_` on `bool` /
`Option<bool>` / `Option<Usd>` / defaulted-enum leaves (`classifier.rs:5-9,16-19`). Each new field's
class:

| new field(s) | class | mechanism |
|---|---|---|
| `DocumentCensus.*`, HoH ×3, home-sale ×4, `claiming_mortgage_interest_credit`, `digital_asset_activity`, `filer_tin_issued_by_due_date`, `g_1099[].itemized_prior_year` | **(A) declaration** | `FORM_QUESTIONS` entries; `screen_inputs` refuses on live `None` |
| `Dependent` gates ×18 | **(A) declaration, per row** | `DEPENDENT_GATES`; `screen_inputs` loops rows |
| `Dependent.full_time_student`, `.permanently_and_totally_disabled`, `.lived_with_you_in_us` | **(A)**, printed as checkboxes — `Some(false)` prints unchecked | same |
| `answer_log[..].state` | not a leaf the classifier forbids (no default) | — |
| `form_1098[].*`, `mortgage_interest_not_on_1098[].*`, `points_not_on_1098`, `form_1098e[].*`, trailer text | money / text on declared documents or filer's records | `LEAF_SOURCE` (R10) |
| `direct_deposit: Option<DirectDeposit>` | **(B)** — absence forgoes direct deposit, advised by `RefundByPaperCheck` (`advisories.rs:172`), which now fires only when absent | exempt-with-reason |

**Kill.** The classifier compiles; `no_option_money_leaf_is_bound_with_underscore` (`classifier.rs:28,913`)
stays green; a planted `_` on any new `Option<bool>` fails the build.

### R15 — What never happens (the stop list, as requirements)

No OCR or model reads the shoebox. No amount is pre-filled from any prior year. No representation of
the form exists outside the Rust structs and the map census (no JSON schema, no generator). No progress
bar; no persisted "what remains". No life-event wizard as a data model. No `reconcile` question in a
return registry. No 1099-DA auto-answer. No web renderer before the TUI journey has been walked once
(brainstorm §9). **Kill:** grep-KATs over `crates/` for the forbidden shapes named in each sentence
(`serde_json::Value` reflection in `btctax-input-form`; `progress`/`remaining` fields; a registry
prompt containing *transfer*/*lot*/*FMV*).

---

## 4. Surfaces

### 4.1 The TUI flow (`btctax-tui-edit` "tax inputs" mode, `crates/btctax-tui-edit/src/edit/{form.rs,persist.rs,tax_inputs.rs}`)

The section order is the 1040's page order with the document screens where the form reads them.
`form_spec()` (`spec/mod.rs:19-38`) becomes:

```
Step 0  Status panel (R9) — ledger blockers, venues vs answers, standing orders, owner actions,
        YearReadiness sentence + interview-complete / return-computable (R11)
 1  ReturnOptions       filing status; itemize election                       (exists)
 2  Taxpayer / Spouse / Address (+ foreign country/province/postal; phone; spouse IP PIN)   (exists + T10)
 3  Dependents          rows (1)–(4) + DOB; rows (5)(6) gates; Steps 1–5 gates; row (7) shown computed (R6)
 4  DocumentCensus      one tri-state per type (R3) — the income block opens here
 5  W2s / W2Box12                                                              (exists)
 6  Int1099 · Div1099 · B1099 · G1099 · Form1098E                              (R4)
 7  ScheduleA           SALT, medical, investment; Form1098 rows; 8b rows; 8c; charitable; the 8396 gate (R8)
 8  HomeSale            the three gates (R8)
 9  Payments            (exists)
10  Carryforwards · QbiLimitation                                              (exists)
11  BrokerReporting     seeded from the ledger                                 (exists)
12  Declarations · IncomeExclusions · Skippables                               (exists; the new FormQuestions land in Declarations by the adapter)
13  DirectDeposit       35b–d (T10)
    The four-state panel (R12) is a pane visible from every section; `s` commit shows it in the modal.
```

Keys and renderer contract are `SPEC_input_form.md` §9A unchanged; `TriState` never displays `None` as
"No" (§5.4). A census row set to `No` over existing rows is refused by `apply` and the renderer offers
*remove N rows and answer No* with a payload-confirm.

### 4.2 The CLI twin — `income answer` extended, `income import` for documents; no `income interview`

Decided: a third renderer would duplicate the TUI. `income answer` (`cli.rs:555`; `answer.rs:52`)
already walks every live registry entry in one pass; with the census rows, HoH, home-sale, 8396, DA and
`filer_tin` gates in `FORM_QUESTIONS` and the dependent gates in `DEPENDENT_GATES` it asks them with no
change to its loop. It gains: the panel printed first and last (R12); answering into the draft on a
draft-only year (R11); `Declined` recorded on a skipped skippable (R10). Documents enter by `income
import` (the TOML wire, §4.3) or the TUI. New: `income project --year N` (R13). The stale help text
naming `set-pii` (`cli.rs:552`) is corrected to name the TUI.

### 4.3 The TOML wire — every new struct, with the `#[serde(default)]` discipline per field

The Rust structs are the schema (`SPEC_input_surface.md` §4). Rule: **`#[serde(default)]` on every new
field whose absence is a lawful state** (`Option<bool>` = never asked; `Vec` = none declared *only
because the census row is the answered-ness carrier*; `String` identity); **no default** on a field
whose absence is not a state (`Form1098.box1_interest` — a 1098 without box 1 is a mistyped row;
`NonForm1098Interest.amount`; `DirectDeposit.{routing, account, kind}`). The TOML for a declared
document with no table refuses at commit (R3), not at parse.

### 4.4 `report`'s rendering

`report --tax-year N` prints, after the existing chains: the document census (type → declared / rows),
the four-state panel, the home-sale decision and its answers, row (7) per dependent, and the
`LEAF_SOURCE` provenance of every collected figure (document with payer TIN masked / filer's records).
The packet's `manifest.txt` "COMPLETE BY HAND" block (`crates/btctax-cli/src/cmd/admin.rs:494-500`)
gains the forgoing list (R12) so the filer sees it while assembling paper (J-15).

---

## 5. Data model

All on `ReturnInputs` (`return_inputs.rs:916`). `V` = repeating (`Vec`), `S` = singleton, `O` =
optional singleton. Every `Usd` shown is `Money ≥ 0` in the form. Provenance is by struct (R10).

### 5.1 `documents: DocumentCensus` (S, new) — one `Option<bool>` per row, each a `FormQuestion`

`w2`, `int_1099`, `div_1099`, `b_1099`, `g_1099`, `form_1098`, `form_1098e` (supported);
`r_1099`, `ssa_1099`, `nec_misc_k_1099`, `k1`, `schedule_e_rental`, `s_1099`, `oid_1099`, `w2g`, `c_1099`,
`a_1095`, `t_1098` (refuse on `Some(true)`, §2.2). Coverage fixture: every row `Some(false)` except the
supported rows that carry a fixture row.

### 5.2 Documents (V) — the box fields carry the box caption verbatim from the archived extract (T2)

| struct | new fields | reaches | fixture must carry |
|---|---|---|---|
| `Form1099Int` (`:85`) | `payer_tin`, `transcribed_on` | as R4 | one row, box 9 = 0 |
| `Form1099Div` (`:104`) | `payer_tin`, `transcribed_on` | as R4 | one row, boxes 2b/2c/2d/13 = 0 |
| `Form1099B` (`:156`) | `payer_tin`, `transcribed_on` | Sch D 1a/8a | one row, gate `Some(true)` |
| `Form1099G` (`:131`) | `payer_tin`, `transcribed_on`, `box2_state_refund: Usd`, `itemized_prior_year: Option<bool>` | Sch 1 L7; box 2 per R4 | one row, `itemized_prior_year = Some(false)` |
| `Form1098` (new) | `lender`, `lender_tin`, `transcribed_on`, `box1_interest`, `box2_outstanding_principal`, `box3_origination_date: Option<Date>`, `box4_refund_overpaid_interest`, `box5_mortgage_insurance`, `box6_points`, `box7_property_address_same_as_payer: bool`, `box8_property_address: String`, `box10_other` | Sch A 8a; the ceiling check; 8d if reinstated | one row, box 2 under the ceiling |
| `Form1098E` (new) | `lender`, `lender_tin`, `transcribed_on`, `box1_interest` | Sch 1 L21 | one row |

`ScheduleAInputs.mortgage_interest_1098` and `Schedule1Inputs.student_loan_interest_paid` are removed
(`:613`, `:689`).

### 5.3 `Dependent` (V, `:224`) — the grid and the gates

Identity (exists): `name`, `ssn`, `relationship: String` (printed in column (4) as written — the
relationship *class* is the Step 1 / Step 4 gate, so no enum migration), `date_of_birth`. Row (5)/(6):
`lived_with_you_over_half_year`, `lived_with_you_in_us`, `full_time_student`,
`permanently_and_totally_disabled`. Gates (R6, all `Option<bool>`, `#[serde(default)]`):
`qc_relationship`, `provided_over_half_own_support`, `filing_joint_return`,
`joint_return_only_to_claim_refund`, `qualifying_child_of_another_person`,
`citizen_national_resident_or_canada_mexico`, `married`, `tin_issued_by_due_date`,
`citizen_national_or_resident_alien`, `ssns_valid_for_employment_issued_by_due_date`,
`qr_relationship_or_member_of_household`, `qualifying_child_of_any_taxpayer`,
`gross_income_under_limit`, `you_provided_over_half_support`,
`divorced_separated_multiple_support_or_kidnapped_rule_applies`. Computed, not stored: the age test,
under-17, row (7). Fixture: one dependent at the CTC edge (all gates answered, DOB 2015, both credit
inputs Yes).

### 5.4 `HouseholdHeader` (S, `:233`) additions

`hoh_unmarried_or_considered_unmarried`, `hoh_qualifying_person`,
`hoh_paid_over_half_cost_of_keeping_up_home` (R7, `Option<bool>`, live on `Hoh`);
`hoh_qualifying_child_name: String` (live on `Hoh`); `filer_tin_issued_by_due_date: Option<bool>`
(live iff any dependent); `spouse_ip_pin: Option<String>` (secret, asymmetric per `seam.rs:220-234`);
`phone: String`; `foreign_country`, `foreign_province`, `foreign_postal_code: String` (live iff
`foreign_country` non-empty; printed in the header's foreign block, which the recon records as
`X` — `FIELD_PROVENANCE.md` §6a); `direct_deposit: Option<DirectDeposit { routing: String, kind:
Checking | Savings, account: String }>` (O; 35b–d, `i1040gi--2025.txt:23963-24000`; the routing rule
*"nine digits … first two digits 01 through 12 or 21 through 32"* is the parse validator).

### 5.5 `ScheduleAInputs` (O, `:594`) additions and `home_sale: HomeSale` (S, new)

`mortgage_interest_not_on_1098: Vec<NonForm1098Interest { recipient_name, recipient_tin,
recipient_address, amount }>`, `points_not_on_1098: Usd`; `claiming_mortgage_interest_credit:
Option<bool>` lives on `ReturnInputs` (a `FormQuestion`, live iff any 1098 or 8b row).
`HomeSale { sold_main_home, test1_owned_2_years_and_lived_2_years_of_last_5,
test2_no_exclusion_on_another_home_in_2_years, can_exclude_all_gain: Option<bool> }`.

### 5.6 Provenance (R10)

`answer_log: BTreeMap<AnswerKey, AnswerRecord>`; `CarryProvenance::ComputedFromPriorReturn { year }`;
`digital_asset_activity: Option<bool>`. `SCHEMA_VERSION = 3`.

### 5.7 The coverage KAT's fixture (`coverage.rs:88`, `maximal_fixture`)

Carries one row of every `Vec` above with every `Option` `Some`, every census row answered, one
dependent at the CTC edge, HoH answered on an `Hoh` fixture variant, the home sale on its blank branch,
`direct_deposit` present. `EXEMPT_PREFIXES` after v1: `capital_loss_carryforward_in`,
`charitable_carryover_in`, `schedule_1a`, `tax_year`, `answer_log`, the four `*_provenance` leaves,
`header.spouse_had_no_income`, `header.spouse_not_filing_a_return`, `schedule_c.*` (until T13),
`sch1.state_refund_taxable` (until its worksheet) — asserted as a ratchet that may only shrink.

---

## 6. The journey — February 2027, as requirements

The brainstorm's walk (§8) restated; each divergence with its class. **R** refusal · **W** warning ·
**D** default · **N** not our concern · **Doc** documentation. A divergence is a requirement only where
the wrong outcome is worse than silence.

| J | moment | requirement | class | rule |
|---|---|---|---|---|
| J-1 | opens the TUI on 2026 | entry shows interview-complete vs return-computable; `NoTables` is never the first time the filer learns it | W | R11 |
| J-2 | picks HoH | the three HoH gates, class (A) | R | R7 |
| J-3 | marks 1099-B *none* because the broker has not mailed | the census is tri-state; *not yet received* stays unanswered and blocks | D | R3 |
| J-4 | four venues with dispositions, three 1099-DAs in hand | the panel names the venue with no answer | W | R9 |
| J-5 | types box 1 into box 3 | W-2 arithmetic warnings; the tool writes nothing | W | R4 |
| J-6 | has a 1099-INT / DIV | a document section; never TOML | — | R4 |
| J-7 | forgets River | the panel lists venues with events vs venues with an answer; a venue on a 1099-DA with no import is named | W | R9 |
| J-8 | Swan transfers-in lost their basis at ingest | the panel shows the blocker census before commit, hands off to `reconcile` | ledger | R9 |
| J-9 | no standing order before the first 2026 custodial sale | per-venue warning with the consequence and the exit | W | R9 |
| J-10 | box 1g differs (FIFO vs HIFO) | `BasisDiffers` refuses (exists); the message names `select-lots` / `import-selections` | R | R9 |
| J-11 | a child born in 2026 with no SSN; a 22-year-old student | the grid + Steps 1–5; the TIN gate says *issued before the due date including extensions* — the extension calendar helps (say so in the help) | R/Doc | R6 |
| J-12 | line 19 never populated | the forgo is visible with its size before filing, in the panel and the commit modal | W | R6, R12 |
| J-13 | a seller-financed second loan | 8b rows with the recipient's name/TIN/address; the 1098 screen | — | R8 |
| J-14 | sold the main home | three gates + the 1099-S row → blank-by-decision or a refusal naming Pub. 523 | R | R8 |
| J-15 | a rental / K-1 | typed census rows that refuse naming the form | R | R3 |
| J-16 | state estimates paid in Jan 2027 typed into 2026 SALT | 5a's help carries *paid in 2026* | Doc | R5 |
| J-17 | commit hits the first refusal, fixes, hits the next | the four-state panel, all at once | — | R12 |
| J-18 | skips the oracle | `income project` + `check_return.py` before export; disagreements adjudicated against the form | — | R13 |

Not requirements: box-14 entries; January spill rows; the Q4 payment date; the appraisal timing beyond
the existing owner-action row. **The walk is re-run live with the owner on the S1 rehearsal documents
before T12 closes** (owner Q3) — a live walk diverges where a solo one cannot.

---

## 7. Build plan

Ordered so the owner's own return (S2) is fillable earliest: the schema first because it cannot be
back-filled; then what a W-2-plus-crypto filer with interest and dividends needs on TY2026 in
Sep–Dec (census, year gate, 1099 sections, the exchange seam); then dependents and real estate (each
conditional on Q1 in effect, but built regardless — they are page-1 and Schedule A lines of the form
the owner files); then the trailer, the oracle path, the panel. Each task: one opus agent under a
persisted brief; TDD; the kill red before green; **once** = code that survives every year, **per-year**
= data a new revision re-earns.

| T | task | once / per-year | kill (the test that reds when the guarantee is removed) |
|---|---|---|---|
| **T1** | **Provenance schema** (R10): `LEAF_SOURCE` + KAT; `payer_tin` / `transcribed_on`; `answer_log` + `record_answer` (one writer); `Declined`; `CarryProvenance::ComputedFromPriorReturn`; `SCHEMA_VERSION` 3 with the refuse / discard / parked-refuse split | once | `LEAF_SOURCE` both directions; identical records via `apply` and `income answer`; `Declined` vs absent; v2 row refuses, v2 WIP discards with note, v2 parked refuses |
| **T2** | **Archive the information returns** — `f1099int`, `f1099div`, `f1099g`, `f1099b`, `f1098`, `f1098e`, `fw2` + their `iNNNN`: `MANIFEST.json` entries, `.pdf.txt` notes, `extract/*.txt` (`design/forms/README.md:14-40`); `Production::Collected` gains `from: DocBox` and `line-coverage` checks the box caption against the document's extract | per-revision (periodic) | a one-character caption change reds `line-coverage`; a `Collected` line with no `DocBox` reds; the archive round-trip hash matches the note |
| **T3** | **Document census + interview state** (R3, R12 core): `DocumentCensus` as `FORM_QUESTIONS`; the three census rules; the unsupported rows with §2.2's sentences; `covered_by` on every `unmodeled` entry of the seven forms + the join KAT; `interview_state()`; `income answer` prints the panel | once (the `covered_by` entries per-year) | R3's five kills; the census join red on a planted bad variant; N unanswered ⇒ N listed; the extended no-brick test |
| **T4** | **Year gate + draft protection** (R11): entry states; confirm-before-discard of a non-trivial WIP draft; `income answer` into a draft-only year | once | R11's six kills |
| **T5** | **1099-INT / DIV / B / G / 1098-E sections** (R4): Fields with T2 captions; 1099-G box 2 + its gate; `student_loan_interest_paid` replaced; `EXEMPT_PREFIXES` shrinks; the six anchors re-attributed; W-2 arithmetic warnings | once (captions per-revision) | coverage KAT count and ratchet; `NotInForm` count falls by six; the warning fixture; the box-2 gate's `Yes` refuses naming the worksheet |
| **T6** | **The exchange seam** (R9): Step 0 panel function + TUI pane; `digital_asset_activity` + the ledger cross-check; the standing-order warning; venue-vs-answer listing | once | the DA four-cell table; the standing-order fixture pair; the unnamed-venue fixture; the grep-KAT on registry prompts |
| **T7** | **Dependents gates** (R6 gates): `Dependent` fields; `DEPENDENT_GATES` registry; `screen_inputs` rows × gates; `live_questions` + the Dependents section per row; the `filer_tin` question; the §152(d) figure in `FullReturnParams` (TY2024/25/26) | once (the figure per-year) | every gate `None` ⇒ refuses; every REFUSE edge names its rule; classifier compiles only when classified; `Single`-no-dependents asks nothing |
| **T8** | **Row (7), HoH, the TY2025+ grid emitter** (R6 computed, R7): the age / under-17 / row-(7) computation; `form1040_full.rs` fills rows (5)–(7) on TY2025+; HoH gates + `hoh_qualifying_child_name`; the line-19 forgo sized in the panel | once (map cells per-year: TY2025 has none for the grid — `crates/btctax-forms/forms/2025/f1040.map.toml:1` is capital-gains only — TY2026 after finals) | the flowchart truth table; TY2024 emitter byte-identical; TY2025 fixture rows filled; HoH kills; the forgo size present/absent |
| **T9** | **Real estate** (R8): `Form1098` section replacing `mortgage_interest_1098`; 8b rows + 8c + the TY2024 map cells; 8e chain; the ceiling warning; the 8396 gate; `HomeSale` gates | once (map cells per-year) | 8e sum; the sweep reconciles; the ceiling fixture pair; 8396 refuses; the 8-branch home-sale table; empty `recipient_tin` refuses |
| **T10** | **Trailer** (§5.4): direct deposit 35b–d + map + emitter + `RefundByPaperCheck` conditional; phone; spouse IP PIN; foreign address | once (map cells per-year) | the routing validator; the advisory silent when a deposit is given; spouse-PIN asymmetry (`get` never returns digits); foreign block printed on a fixture |
| **T11** | **Oracle path** (R13): `project_to_golden`; `income project`; `check_return.py`; `ORACLE_INVISIBLE` | once | the projection inverse over every golden; the invisible list complete; the CTC excuse exact |
| **T12** | **The panel in the TUI + docs** (R12 render): the pane, the commit modal's forgoing list, the manifest block; man pages (`docs/man/btctax-income-answer.1` and siblings via `make docs`); `LIMITATIONS.md` | once | snapshots (N blocking; forgo sizes); `make docs` clean; the manifest block present on a fixture packet |
| **T13** | *(iff S2 confirms a Schedule C)* Schedule C Part II as a transcription struct (lines 8–27, `i1040sc`) + the 1099-NEC / MISC / K screen; the `nec_misc_k_1099` census row stops refusing | once | line-coverage on every Part II line; the census row opens the section; the two `attribute.rs:262,265` anchors re-attributed |
| **T14** | *(iff S2 confirms a 1099-R or SSA-1099)* the two screens + the Simplified Method and Social Security Benefits worksheets — from `design/ty2025/SPEC_retirement_income.md` after its own one round | once | per that spec |
| **T15** | *(post-v1, first γ)* Schedule 8812 — line 19 / 28 computed from the T7/T8 answers; needs the TY2026 final | per-year form | the forgo disappears; both oracles reconcile line 19 |

**Twelve unconditional tasks, two conditional on S2, one post-v1.** Every task lands with its kill red
first (B1), one seam review scoped to the task's `main..HEAD` interaction (B3), a fold, and one
re-verification (S6).

---

## 8. Consolidated kills

| guarantee | the test that reds when it is removed |
|---|---|
| no `Money` field outside a document or filer's-records struct; every `Usd` leaf has one source | `LEAF_SOURCE` completeness, both directions (T1) |
| every document box caption is the form's own text | `line-coverage` `DocBox` check (T2) |
| a line btctax cannot take is announced or refused, never silent | the `covered_by` join over every `unmodeled` census entry (T3) |
| a document type is asked, and "none" is an answer, not an absence | census `None` refuses; declared-with-no-rows refuses; none-with-rows refuses (T3) |
| an excluded family refuses with its exit | every unsupported row's `Some(true)` message contains the exit (T3) |
| every blocking and forgoing item is visible at once; class (B) is never flattened | N-in-one-call; `Declined` neither blocks nor forgoes; the extended no-brick test (T3) |
| a long-lived draft is not lost to a note | import over a non-trivial draft refuses without `--force` (T4) |
| the draft never poisons a year | `resolve.rs` never reads it (exists, re-pinned) (T4) |
| the form covers every in-scope leaf and the exempt set only shrinks | the coverage KAT + the ratchet (T5, every later task) |
| the form can answer what it refuses | `NotInForm` count falls by exactly the re-attributed number (T5) |
| the DA box is never a "No" the ledger contradicts, nor a "Yes" with an empty ledger | the four-cell table (T6) |
| a venue without a standing order is warned before its 1099-DA row | the fixture pair (T6) |
| every §152 edge lands where the instruction says | the truth table per edge (T7/T8) |
| an unanswered dependent gate blocks | rows × gates `None` ⇒ refuse (T7) |
| row (7) is computed, never asked | no `FieldId` for a row-(7) box; the emitter reads the computation (T8) |
| TY2024 is untouched by the grid work | byte-identical emitter output (T8) |
| HoH is an assertion | `Hoh` with any `None` refuses (T8) |
| 8e = 8a + 8b + 8c; the ceiling is checked with a figure | the sum fixture; the warning pair (T9) |
| a home sale is blank only on the instruction's own branch | the 8-branch table (T9) |
| secrets never cross the seam | spouse-PIN asymmetry (T10) |
| a real return reaches both oracles; every figure projects or is listed | the projection inverse; `ORACLE_INVISIBLE` (T11) |
| answered-ness is structural for every new field | the classifier compiles only when classified; the `_` KAT (every task) |
| no forbidden shape returns | the R15 grep-KATs (T3, T6) |
| every checker was seen red | each instrument's planted-defect test, in the `field_census.rs:329` style, committed before its first green |

---

## 9. Open questions for the owner (four)

1. **S2, sharpened to documents.** For 2026: a Form 1098 (mortgage)? Dependents — how many, any
   full-time student aged 19–23, any born in 2026, HoH? A 1099-G with a state refund, and did you
   itemize on the 2025 return? A Schedule C beyond the ledger's, or a 1099-NEC/MISC/K? A 1099-R or
   SSA-1099? A home sale, K-1 or rental? — *Each yes moves one row: 1098/dependents change nothing (T8/T9
   are built regardless, being the form's page 1 and Schedule A); "itemized 2025 + a refund" adds the
   State and Local Income Tax Refund Worksheet to T5; Schedule C / 1099-NEC ⇒ T13; 1099-R / SSA ⇒ T14; a
   K-1 or rental means a preparer for 2026, not a build.*
2. **Line 19 in v1.** Accept the visible forgo of the child tax credit until Schedule 8812 is
   transcribed from the TY2026 final (T15), or pull T15 into v1 after finals? — *With dependents, up to
   the statutory maximum per child is on the extension deadline; without, the question is moot.*
3. **Is the S1 rehearsal the interview's first live walk?** — *Recommended yes: the TY2025 documents in
   Sep–Dec 2026 are the only way to find the missing-at-moments before the ten-week window that also
   holds finals, OTS 2026 and the first 1099-DAs. If S1 stays declined, the first walk is your own
   February 2027 return and every J-n found then lands on the extension.*
4. **Direct deposit or paper check?** — *T10 builds 35b–d either way (α); if you will always take a
   check, the routing/account fields are never entered and `RefundByPaperCheck` keeps firing — say so
   and T10 drops to the phone / spouse-PIN / foreign-address trio.*

Decided without asking (the brainstorm's Q2, Q3, Q5): Step 0 is a panel with parallel authoring and a
hard gate at commit; year N+1 opens with confirmations and retention is the filer's; no 1099-DA
transcription screen.

---

## 10. What this spec does NOT change

- **The crypto engine** — lots, methods, the 8949/Schedule D chains, the four adapters, the 27
  `reconcile` subcommands, `BlockerKind`. The interview reads the ledger (Step 0, seeding) and never
  writes it.
- **The 1099-DA rules** — `SPEC_1099da_broker_reporting.md` R1–R6, `BrokerReported`, the routing, the
  R6 slice path on a params-less year.
- **The packet** — attachment sequence, `manifest.txt` (one appended block), Form 4868 / 1040-V, the
  by-hand marks; the emitter changes are confined to filling cells that exist (rows (5)–(7); 8b/8c;
  35b–d; the foreign block).
- **The oracles** — OTS and Tax-Calculator, `gen_goldens.py`, `sweep.py`, the two-oracle rule and its
  §G-9 limit; R13 adds a projection *beside* them.
- **The frozen files** `tax/{types,compute,se}.rs`; `screen_inputs`' first-refusal contract and its
  tiers; `resolve.rs` precedence; the draft table's C-1 semantics; the `FormSpec` seam types
  (`seam.rs`) — new `SectionId` / `FieldId` variants only.
- **I-11.** A params-less year still cannot commit. What changes is that the interview can finish
  there, and the filer is told so at the door.
