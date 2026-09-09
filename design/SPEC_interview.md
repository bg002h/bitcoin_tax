# SPEC — the INTERVIEW that fills the return

**Status: r2 — the one review folded.** (DRAFT r1 written 2026-09-06 at `c5af3f60` by Fable, lens 2 of 3;
r2 folds `design/agent-reports/2026-09-07-spec-interview-review.md`, 1C/14I/9M/4N, under
`design/agent-reports/BRIEF-fold-spec-interview.md`; the fold record is
`design/agent-reports/2026-09-07-spec-interview-fold.md`.) **Under owner ruling S6 this is the LAST prose
step: no second review round follows — each task below is built with its kill red first and its own seam
review is the gate.** Brief: `design/agent-reports/BRIEF-spec-interview.md`. Inputs, in the order read: the recon
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
refusals — the answer panel is derived from the registries instead (R12).

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
| 1 | provenance schema — sources by struct, `answered_on` + prompt hash per answer (and the re-ask a hash mismatch forces), `Declined`, payer identity, the answer key by identity; the year-N+1 opener | schema | R10 |
| 2 | interview-complete vs return-computable at entry; the Step 0 ledger/owner-action panel | process + α | R9, R11 |
| 3 | the document census (tri-state per type) + 1099-INT / DIV / B / G / 1098-E sections + trailer (35b–d, phone, spouse IP PIN, foreign address) | α | R3, R4 |
| 4 | dependents: the TY2025+ seven-row grid, Steps 1–5 as per-dependent gates, the filer-TIN gate, HoH and QSS gates; row (7) computed; line 19 a visible forgo | β | R6, R7 |
| 5 | the answer panel — every blocking, forgoing, refusing and waiting item at once | engine | R12 |
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
| **Form 1099-NEC / 1099-MISC / 1099-K** | Sch 1 L3 (via Sch C), Sch 2 L4, Sch 1 L15; **Sch 1 L8z** (1099-MISC box 3); Sch 1-A Parts II/III | *"btctax cannot take a Form 1099-NEC, 1099-MISC or 1099-K this year. Box 1 / non-employee compensation is Schedule C income and Schedule C Part II is not built; **1099-MISC box 3 — prizes, awards, research-study pay — is Schedule 1 line 8z income**, which is not built either (T13, on S2). A crypto-only Schedule C still fills from the ledger."* (matches `attribute.rs:262,265`) |
| **Schedule K-1** (any) | Sch 1 L5; Sch D L5/L12; Sch 3 L6l | *"btctax cannot take a Schedule K-1: partnership, S-corporation, estate and trust items reach nothing. A preparer is the exit for this year."* |
| **rental real estate / royalties (Schedule E)** | Sch 1 L5; Form 8960 L4a | *"btctax has no Schedule E. A rental or royalty is a preparer's return for this year."* |
| **Form 1099-S / a sale of real property you cannot fully exclude** | Form 8949 (code H for a main home); Sch D | *"btctax does not compute a sale of real property. Form 8949 and Schedule D are the exit for a vacant lot, an inherited house or a rental; if it was your main home, Form 8949 code H and the Pub. 523 worksheet."* (R8) |
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
every registry entry is `Option<bool>` / `Option<Date>` / `Option<Enum>` — the last **only** where the
instruction itself states named alternatives rather than a yes/no, which is R7's `HohMaritalBasis` and
nothing else in v1 (a single `Option<bool>` over three distinct legal predicates is the compound answer
this rule exists to forbid) — (the `decl_tristate!` / `skippable_*` shapes,
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
   the document side: every `Collected` line names where it is collected from —
   `from: DocBox { stem, box, extract_line } | FilerRecords { instruction_line }`, the second because
   R5's lines (line 26, Schedule A 5b/5c, 8b/8c, the charitable rows) have no issuing third party and so
   no box — and the checker verifies the quotation verbatim against the archived extract: the box caption
   for a `DocBox`, the instruction's own sentence for a `FilerRecords`. The information returns are
   **not archived at HEAD** (`design/forms/extract/` holds the 1040 family, 8283/8949/6251/8995/8615/8275
   and Schedule 8812 for 2024 only); T2 archives `f1099int`, `f1099div`, `f1099g`, `f1099b`, `f1098`,
   `f1098e`, `fw2` and their `iNNNN` per `design/forms/README.md:14-25` (periodic cadence, note + text
   layer, no PDF committed).
2. **Every unmodeled line names what tells the filer — and a line whose omission UNDERSTATES tax may
   never be covered by an advisory.** The `[census]` sections
   (`crates/btctax-forms/tests/field_census.rs:167-177`, text-scanned) carry `rule = "unmodeled"` with a
   free-text `reason`. On the forms the interview reaches (f1040, f1040s1/s2/s3, f1040sa, f1040sb,
   schedule_d) every `unmodeled` entry gains `covered_by = "Advisory::X" | "RefuseReason::Y" |
   "QuestionId::Z"`, and a KAT joins each against the enum's own source (the `classifier.rs` technique of
   reading both sources) — a line btctax cannot take is thereby either **announced** or **refused**,
   never silent. This is §2.2's table made mechanical.

   **The DIRECTION RULE — the join is not direction-blind.** *"Announced or refused"* is not the same
   guarantee in both directions: a **deduction or credit** left blank forgoes money lawfully, and a
   sentence saying btctax did not try is an honest cover for it; an **income or additional-tax** line
   left blank is an omission that understates the tax, and no sentence covers that. So each census entry
   carries a **direction**, and the direction is *derived, never typed*:

   > A `[direction]` table per form maps that form's own **part headings** to `Understates` or
   > `Overstates`, and a KAT asserts every key **verbatim against the form's extract** in
   > `design/forms/extract/` before using it, so the table is a *reading of the form*, not a hand-list,
   > and a re-parted revision reds rather than drifting. Where a form's parts are not separably captioned
   > in the text layer, the key is the **form's own title** and the direction is whole-form. Verified
   > present in the TY2025 extracts at the time of writing:
   >
   > | form (extract) | key(s) asserted | direction |
   > |---|---|---|
   > | `f1040--2025.txt` | *Income* (the lines 1–9 block) | `Understates` |
   > | `f1040s1--2025.txt` | *Additional Income* (Part I) | `Understates` |
   > | `f1040s1--2025.txt` | *Adjustments to Income* (Part II) | `Overstates` |
   > | `f1040s2--2025.txt` | *Tax* (Part I) and *Other Taxes* (Part II) | `Understates` (both) |
   > | `f1040s3--2025.txt` | *Additional Credits and Payments* | `Overstates` |
   > | `f1040sa--2025.txt` | *Itemized Deductions* | `Overstates` |
   > | `f1040sd--2025.txt` | each of the three part headings as the extract prints them | `Understates` |
   > | `f1040sb--2025.txt` | the form title (its Part I/II/III headings run into their content in the text layer, so there is no separable caption to key on) | `Understates` |
   >
   > An entry the table cannot place — a key the extract no longer carries, a line outside every mapped
   > part — **reds**; it is never defaulted to `Overstates`. (Schedule B Part III's foreign-account and
   > trust questions carry no dollar; they are gates, decided by R3's classes, not by direction.)

   Then the join has two rules, not one:
   - **An `Understates` entry may be covered only by a `QuestionId` or a `RefuseReason`. An `Advisory`
     cover on an `Understates` entry reds.** This is the rule the build would otherwise break by default:
     there is one advisory and no question for most of the 57 `unmodeled` Schedule 1 entries
     (`crates/btctax-forms/forms/2024/f1040s1.map.toml:87` alimony, `:95` gambling, `:96` cancellation of
     debt — *"Income, so the understatement direction; there is no input that could reach it"*), and an
     advisory is exactly what a builder reaches for.
   - **A `QuestionId` cover is checked for REACH, not existence.** The KAT asserts the covering question's
     `prompt` text contains the entry's own line caption keyword — the leading quoted phrase of `reason`,
     or an explicit `names = "…"` key on the entry when the caption and the prompt's wording differ. A
     variant that exists is not a cover; a question the filer can answer *No* to without ever reading the
     line's name is not a cover. In particular the residual scope attestation
     (`crates/btctax-core/src/tax/questions.rs:546-563`) covers **exactly the lines its prompt enumerates**
     and its trailing *"or anything else it never asked about"* covers **nothing** — that clause is the
     compound "no" over distinct legal predicates that brainstorm §1.4 rejected and
     `FIELD_PROVENANCE.md:289-292` forbids (*"fabricates precision the filer never swore to"*). Every line
     claimed for it must therefore be **named in the words the filer reads**, which is an edit to the
     prompt, not to a TOML key: today's prompt names 1099-R, SSA-1099, rent, farm, K-1, tips, gambling,
     alimony and *"a business"*, so household-employee wages (1b), Medicaid waiver payments (1d), jury duty
     pay (8h), prizes and awards (8i), hobby income (8j) and each remaining Schedule 1 Part I line it is
     asked to cover are added to it or are covered some other way.
   - **A `RefuseReason` cover is checked as today** (the variant exists), because a refusal stops the
     return and nothing is filed silently.

   `Overstates` entries keep the existing rule: an `Advisory`, a `RefuseReason` or a `QuestionId` cover,
   existence-checked. The asymmetry is the point — this is the direction in which a blank is a false
   statement rather than a forgone benefit.
3. **Every gate is reachable and every Field is read.** The coverage KAT
   (`coverage.rs:240`, `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt`) stays the
   forward direction; the reverse — no `FieldId` exists that no printed chain, screen rule, or census
   entry reads — is the `LEAF_SOURCE` join (R10).

**Kill (B1, seen red first).** Plant: (a) change one document `Field.help` by one character → `line-coverage`
red; (b) delete `covered_by` from one `unmodeled` entry, or point it at a variant that does not exist →
the census join red; **(b2)** plant `covered_by = "Advisory::EicOmitted"` on Schedule 1 line 2a — an
`Understates` entry — → red, and the same advisory on a Schedule 1 Part II entry stays green (the
instrument is watched *discriminating*, not merely refusing); **(b3)** plant
`covered_by = "QuestionId::OtherOutOfScopeIncome"` on Schedule 1 line 8h while the attestation's prompt
does not contain *"jury duty"* → red, then add the words to the prompt → green; **(b4)** delete one part
heading from the `[direction]` table → every entry in that part is unplaceable and reds (it does not fall
back to `Overstates`); (c) add a `Usd` leaf to a struct with no `LEAF_SOURCE` prefix → red. Each planted in
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

**A census `No` never closes income the instructions say to report WITHOUT the document.** The
document-first rule is right for amounts and wrong as a stop: the form names incomes that exist with no
information return behind them, and each is small money in the understatement direction, where the
residual attestation is the only net (C1). So the four supported income rows `w2`, `int_1099`,
`div_1099` and `g_1099` each carry a **paired class-(A) `FormQuestion`, live iff that row is
`Some(false)`**, phrased from the instruction that says so:

- *"Did you receive wages, salary or tips from an employer who issued no Form W-2?"* —
  *"Even if you don't get a Form W-2, you must still report your earnings"*
  (`i1040gi--2025.txt:2442-2444`). `Yes` refuses `WagesWithoutW2`, naming Form 1040 line 1a.
- *"Did you receive taxable interest or dividends for which no Form 1099-INT or 1099-DIV was issued — a
  bank paying under $10, a seller-financed mortgage you hold, a nominee distribution?"* — Schedule B line
  1 *"Report on line 1 all of your taxable interest"* (`i1040sb--2025.txt:56-58`); 1040 line 3b *"if you
  received dividends not reported on Form 1099-DIV"* (`i1040gi--2025.txt:2521`); seller-financed mortgage
  interest, which Schedule B asks for with the buyer's SSN and address (`i1040sb--2025.txt:24, 77`).
  `Yes` opens a `FilerRecords` repeating row `schedule_b_filer_records` (§5.1) — `{ payer_name,
  payer_ssn, payer_address, amount, kind: Interest | Dividend }`, exactly what Schedule B lines 1 and 5
  take — and `Yes` with no row refuses `DocumentDeclaredNotTranscribed`-style as UNANSWERED.
- *"Did you receive a refund, credit or offset of state or local income taxes in 2026?"* — *"Report any
  taxable refund you received even if you didn't receive Form 1099-G"* (`i1040gi--2025.txt:41897-41898`).
  `Yes` refuses naming the **State and Local Income Tax Refund Worksheet**, the same exit 1099-G box 2
  takes (R4).

And because a gate may not ride on a row that might not exist, **`itemized_prior_year` moves off the
`Form1099G` row onto `ReturnInputs`** as a return-level `FormQuestion`, live iff the refund question is
`Some(true)` **or** any 1099-G box 2 > 0. On a year btctax filed year N−1, a `screen_compute_dependent`
rule cross-checks it against year N−1's committed `itemize_election`, the way R9 cross-checks the
digital-assets box.

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
class (A) with its `QuestionId` (`crates/btctax-core/src/tax/classifier.rs:5-9` — no `..`, no `_`);
(f) **each paired question is live exactly when its row is `Some(false)` and blocks there** — `w2 =
Some(false)` with `w2_wages_without_w2 = None` refuses; `w2 = Some(true)` never asks it; a `Yes` on the
wage question refuses naming line 1a, a `Yes` on the refund question refuses naming the State and Local
Income Tax Refund Worksheet, and a `Yes` on the interest/dividend question with no
`schedule_b_filer_records` row refuses while one row passes; (g) `itemized_prior_year` is live on a
return with **no** 1099-G row when the refund question is `Some(true)`.

### R4 — Document screens: the document's boxes, verbatim, each reaching a named line. The tool checks arithmetic the document guarantees and never fills a line from the check.

**Mechanism.** One repeating section per supported document type; per row the payer identity plus the
boxes. Reach, from the 1040 text (all cites `design/forms/extract/`):

| document | boxes collected (struct today) | reaches | cite |
|---|---|---|---|
| **W-2** (exists, 14 Fields + box 12) | 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 17, 19 (`return_inputs.rs:36-80`) | 1a, 25a, 8959, Sch A 5a, §6413(c) | `i1040gi--2025.txt:2184` |
| **1099-INT** | payer, **payer TIN (new)**, 1, 2, 3, 4, 6, 8, 9 (`:85-99`) | 2b (1+3, via Sch B 1), Sch 1 L18 (2), 25b (4), §904(j) FTC (6), 2a (8), refuse (9) | 2a `:2448-2459`; 2b `:2485-2492`; Sch B 1 `i1040sb--2025.txt:58-60`; box 9 `:163-164`; 25b `i1040gi--2025.txt:4225-4241` |
| **1099-DIV** | payer, TIN, 1a, 1b, 2a, 2b, 2c, 2d, 4, 5, 7, 12, 13 (`:104-126`) | 3b (1a), 3a (1b), Sch D 13 (2a), refuse (2b/2c/2d/13), 25b (4), 8995 (5), FTC (7), 2a (12) | 3a `:2508-2516`; 3b `:2611-2618`; box 12 `i1040sb--2025.txt:163` |
| **1099-B** | payer, TIN, ST/LT proceeds and basis, `basis_reported_and_no_adjustments` (`:156-187`) | Sch D 1a/8a; the gate refuses `Form1099BNeedsForm8949` (`attribute.rs:254`) | `return_inputs.rs:140-152` quotes Sch D 1a |
| **1099-G** | payer, TIN, 1, 4 (`:131-135`); **box 2 (new)**; the *"did you itemize on your prior-year return?"* gate is **return-level**, not a row field (R3/I1) | Sch 1 L7 (1), 25b (4); **box 2: `itemized_prior_year = No` ⇒ Sch 1 L1 blank-by-decision; `Yes` ⇒ refuse** naming the State and Local Income Tax Refund Worksheet until it is transcribed (owner Q1 decides whether that worksheet joins T5); the same refusal is reached with no 1099-G at all, through the refund question (R3/I1) | L7 `i1040gi--2025.txt:42005`; L1 `:41894-41897` |
| **1098** (new, R8) | lender, TIN, 1, 2, 3, 4, 5, 6, 7/8 property, 10 | Sch A 8a (1 + 6), the §163(h)(3)(B) ceiling check (2, 3), 8d if a final reinstates it (5) | `i1040sca--2025.txt:1058-1068` |
| **1098-E** (new) | lender, TIN, 1 | Sch 1 L21 (replaces `sch1.student_loan_interest_paid`, `return_inputs.rs:689`) | Sch 1 L21 chain `printed.rs:483-484` |

**Every supported document carries a `[boxes]` census beside its `Field`s, ENUMERATED from the archived
extract — the table above is a reading list, never the authority.** The same rule that governs form lines
governs document boxes (*"enumerate the line set FROM the form's extracted text, never from a range or a
hand-written list"*, `CLAUDE.md`): a KAT reads the box captions out of T2's archived `fNNNN` extract and
requires **every caption to carry exactly one entry** — `collected(FieldId)`,
`refuse_if_nonzero(RefuseReason)` (1099-INT box 9 today), or `not_read(reason)` carrying the
instruction's own pointer. A caption in the extract with no entry **reds**; an entry naming a caption the
extract does not have **reds**. Without it the box lists are hand-lists and a box the return needs is
*dropped* rather than *recorded* — which is the blank-because-nothing-populated-it defect one level below
the form line. Three the 1040 text already points at, decided as the form decides them:

- **1099-INT box 10** (market discount) → `collected` → Schedule B line 1 and the 2b sum
  (`i1040sb--2025.txt:59-61`, *"Also include any accrued market discount that is includible in income"*;
  1040 line 2b, `i1040gi--2025.txt:2493-2497`). Income, so `Understates` under C1's direction rule.
- **1099-INT boxes 11–13** (bond premium) → `refuse_if_nonzero(AmortizableBondPremiumNotComputed)`,
  naming the amortizable-bond-premium adjustment and Pub. 550, until it is transcribed. A reduction, so
  refusing rather than dropping is the conservative direction *and* the honest one.
- **W-2 box 13 *Statutory employee*** → `collected: Bool`; `Some(true)` refuses `StatutoryEmployeeW2`
  naming **Schedule C line 1** — a checked box 13 sends box 1 to Schedule C, not to 1040 line 1a, so
  today's `W2` (`return_inputs.rs:36-80`), which has no box 13, would file the wages on the wrong line.
- **1099-DIV box 3** (nondividend distributions) → `not_read("reduces basis, does not reach a line this
  year; Pub. 550")` — a box on the paper that now has a recorded reason instead of no entry.

Box captions come from the document's extract (T2), one `Field` per box named for the box
(`Box1Interest`, …), the caption verbatim as `help` (the transcription rule, `CLAUDE.md`; the struct
*is* the form). Checks the tool may run and **display** (brainstorm §7): W-2 box 4 ≈ 6.2% × box 3, box
6 ≈ 1.45% × box 5, box 3 ≤ the wage base — **warnings**, typo detectors; the filer corrects the box, the
tool never writes it. The **payer TIN** is the cross-year identity (R10); a document row's every box is
testimony by construction, because the row exists only because the filer declared the document (R3) —
a 1099-INT with box 2 = 0 is the *document's* zero, so no `Option<Usd>` is needed inside a document row.
Two consequences of that, both cheap: a row whose **every income box is zero** is most likely a row the
filer began and did not finish — a payer issues a 1099-INT at $10 or more — so it raises the same kind of
**warning** the W-2 arithmetic checks raise (*"every income box on this 1099-INT is zero; a payer issues
one at $10 or more — check the row"*) and writes nothing; and a row that arrives by TOML with
`transcribed_on = None` is not silently tidied away — the packet manifest prints it as *transcribed
without a date* (§4.4), because an absent date is a fact about the evidence, not the absence of a fact.

**Kill.** Coverage KAT: `EXEMPT_PREFIXES` (`coverage.rs:327`) loses `int_1099`, `div_1099`, `g_1099`,
`b_1099`, `sch1.student_loan_interest_paid`; the pinned Field count rises from 98 and the exempt count
is asserted **≤ its new value** (a ratchet — a task may only shrink it). Attribution: the **five** `NotInForm`
anchors in this cluster (`PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`,
`InconsistentDividendSubset`, `ForeignTaxOverCeiling`, `Form1099BNeedsForm8949`) become `Field`/`Section`
anchors and a test asserts the `NotInForm` count fell by exactly **five**. `IraDeductionClaimed`
(`attribute.rs:234`) is **not** among them: HSA and IRA contributions stay refused unchanged (§2.2), so
its anchor stays `NotInForm`. The W-2 arithmetic warning fires on a planted box-1-in-box-3 fixture and writes
nothing; the all-zero
income-box warning fires on a 1099-INT row with every income box 0 and writes nothing. The `[boxes]`
census is complete for all seven documents against T2's extracts: deleting one entry reds, adding an
entry for a caption the extract does not carry reds, a W-2 with box 13 checked refuses
`StatutoryEmployeeW2` naming Schedule C line 1, and a 1099-INT with box 11 > 0 refuses
`AmortizableBondPremiumNotComputed`.

### R5 — Amounts the form attributes to the filer's own records are collected as `FilerRecords`, and the difference is recorded.

**Mechanism.** Line 26 (`i1040gi--2025.txt:4261-4268`), line 31's extension payment, Schedule A 5b/5c
(`i1040sca--2025.txt:693-700`), 8b/8c (R8), the charitable rows, the dependents' facts: these have no
issuing third party. They live in structs whose `LEAF_SOURCE` entry (R10) is `Source::FilerRecords`,
and their help text carries the instruction's own timing words — 5a's *"paid in 2026"* (J-16) — and line
26's help carries the clause that no other source can supply: *"Include any overpayment that you applied
to your 2026 estimated tax from your 2025 return"* (`i1040gi--2025.txt:4266-4268`). The owner's TY2025
return was filed outside this project, so if the help does not say it, an applied overpayment is simply
missing from line 26 and nothing in the tool can notice (J-31). A
figure with a document behind it and a figure from memory are different evidence in an examination;
the source is recorded so the packet's manifest can say which is which.

**Kill.** `LEAF_SOURCE` completeness (R10). The 5a help text contains the instruction's *paid in*
wording and line 26's contains the *applied to your 2026 estimated tax* clause, both checked by
`line-coverage` like any other quotation; each is a `FilerRecords { instruction_line }` production, so a
`Collected` line with neither a box nor an instruction line reds (R2 mechanism 1).

### R6 — Dependents: the TY2025+ grid transcribed, Steps 1–5 as per-dependent gates with the instruction's own edges, row (7) computed, line 19 a visible forgo.

**Mechanism.** Per dependent row (`Dependent`, `return_inputs.rs:224-228`), in addition to the four
identity fields, the gates below — every one `Option<bool>` (class A: live ⇒ must be answered), phrased
as the instruction phrases it, with the edge the instruction gives. They form a new per-row registry
`DEPENDENT_GATES: &[DependentGate]` in core (same shape as `FormQuestion` — prompt, `live(&ReturnInputs,
row)`, get/set over `&Dependent`, the refusal, `Durability`), so that `screen_inputs` loops rows × gates
(refusing `DependentGateUnanswered { row, gate }` on a live `None`), `live_questions` enumerates them
(`crates/btctax-cli/src/cmd/answer.rs:52`), and the input form adapts them into the `Dependents` section
per row.

**Per-row liveness uses the I-4 emulation that already exists, not a widened seam.** `Field.live` is
`fn(&ReturnInputs) -> bool` (`seam.rs:266`) and §10 freezes the seam types, so a gate whose liveness
depends on *its own row* — a Step 4 gate is live for a row only when that row's Step 1 answered No —
is rendered by the pattern already in `registries.rs:44-50`: `live: |_| true` with the row's `get`
returning absent when the gate is not live for that row, which the renderer already treats as hidden.
`DEPENDENT_GATES` keeps its own `live(&ReturnInputs, row)`, which is all `screen_inputs` and
`interview_state` need, because they loop the rows themselves. Widening the seam to
`fn(&ReturnInputs, &RowAddr) -> bool` — 98 mechanical closure edits — is **decided against**: it breaks
§10's freeze and buys no behaviour the emulation does not already give.

| step | gate (prompt as the instruction phrases it) | edge | cite `i1040gi--2025.txt` |
|---|---|---|---|
| 1 | `qc_relationship` — *son, daughter, stepchild, foster child, brother, sister, stepbrother, stepsister, half brother, half sister, or a descendant of any of them?* | No ⇒ Step 4 | `:1463-1466` |
| 1 | **age test — COMPUTED** from `date_of_birth` (**required on every row**), `full_time_student`, `permanently_and_totally_disabled` (row 6) and the gate below | `date_of_birth = None` ⇒ **`DependentGateUnanswered { row, gate: DateOfBirth }`**, blocking — Step 1 has no *unknown* edge, so the row is not printed until the test can be evaluated | `:1487-1499` |
| 1 | `younger_than_you_or_spouse` — *younger than you (or your spouse if filing jointly)?* — a **per-row gate**, not a computation over the taxpayer's DOB | part of the age test; `None` while live blocks | `:1489, :1493` |
| 1 | `provided_over_half_own_support` — *did this person provide over half of their own support?* | Yes ⇒ Step 4 | `:1502` |
| 1 | `filing_joint_return`; if Yes, `joint_return_only_to_claim_refund` | joint and not refund-only ⇒ Step 4 | `:1506-1508` |
| 1 | `lived_with_you_over_half_year` — **row (5)(a)**, whose `help` carries `:1905-1913` **verbatim**: the *Exception to time lived with you* — temporary absences (school, vacation, business, medical care, military service), a child **born or died in the year**, a child adopted or placed in the year | No ⇒ Step 4 — but the filer answers the instruction's condition **with its exception in front of them**, so a child born 2026-11-01 whose home was the filer's for more than half the time the child was alive answers `Yes` and row (5)(a) prints | `:1512-1516`, `:1905-1925` |
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
| 4 | `gross_income_under_limit` — the figure is a `FullReturnParams` value (TY2025: $5,200) **quoted in the prompt**, so the gate is **not live until the year's parameters exist**: on a params-less year the panel lists it as *waiting for the TY2026 package* (an R11 state, R12's row), never as a blocking item, because a prompt that cannot state the figure asks the filer to derive it | No ⇒ REFUSE; disabled + No ⇒ REFUSE naming *Exception to gross income test* | `:1690-1691` |
| 4 | `you_provided_over_half_support` | No ⇒ REFUSE | `:1694-1696` |
| 4/1 | `divorced_separated_multiple_support_or_kidnapped_rule_applies` | Yes ⇒ **REFUSE** naming the three rules (`:1823, :1949`) | `:1696` |
| 4 | citizen / married / joint / could-you-be-claimed — the Step 2 gates reused | as Step 2 | `:1700-1735` |
| 5 | return-level `filer_tin_issued_by_due_date` — *did you, and your spouse if joint, have an SSN or ITIN issued on or before the due date?* — a `FormQuestion` live iff any dependent row exists | No ⇒ no ODC box | `:1743-1750` |
| 5 | qualifying relative's TIN / citizen / married — the Step 3.1 and Step 2 gates reused | as above | `:1765-1790` |

**`date_of_birth` is REQUIRED on a dependent row, and the age test is a Step 1 condition, not a credit
condition.** `None` is `DependentGateUnanswered { row, gate: DateOfBirth }` and blocks (class A): a birth
date is on the SSN application and the birth certificate — a fact, not a decision. The r1 text (*"DOB
absent ⇒ the test is unknown ⇒ no credit column, class-(B) forgo"*) is **withdrawn**: the age test sits
inside Step 1 (`:1487-1499`), which ends *"Yes. Go to Step 2. No. Go to Step 4."* (`:1525-1529`) with no
*unknown* edge, so an unevaluable age test leaves neither §152(c) nor §152(d) established, and printing
the person in the Dependents section — *"the dependents you claim"* (`:1470-1472`) — is testimony that
§152 is satisfied on a chain that was never completed. For the same reason *"younger than you (or your
spouse if filing jointly)"* is asked as its own per-row gate rather than computed from the taxpayer's
DOB: that DOB is a class-(B) **skippable** the filer may lawfully `Decline` (`questions.rs:939-941`,
`Durable`), and no Step 1 predicate may depend on a declinable value. The age brackets (<19 / <24 / any)
compute from the row's own `date_of_birth` and `tax_year`.

**The *Exception to time lived with you* is part of the condition, not a branch out of the flowchart.**
`:1512-1516` says *"If the child didn't live with you for the required time, see Exception to time lived
with you, later"*, and `:1905-1925` states the exception fully in one paragraph — *"Temporary absences …
such as school, vacation, business, medical care, military service … count as time the person lived with
you"*; *"If the person meets all other requirements to be your qualifying child but was born or died in
2025, the person is considered to have lived with you for more than half of 2025 if your home was this
person's home for more than half the time the person was alive."* That is a definition the filer applies,
which is the brainstorm's own rule for a *"see X, later"* pointer. Dropping it is **not** conservative:
a child born in November answers the bare gate *No*, reaches Step 4, passes the qualifying-relative
tests — a newborn has no gross income and the filer provided all support — and prints **row (7) Credit
for other dependents**, a checked box that is wrong on the form's own terms, an under-claim of the larger
credit, and not even a visible forgo. A college student away at school for nine months is the same
misroute. Only *Kidnapped child* (`:1910`) and *Children of divorced or separated parents* (`:1824`)
leave the flowchart for a multi-page rule, and both are already REFUSE branches through
`divorced_separated_multiple_support_or_kidnapped_rule_applies`.

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
rule. **The two edges r1 dropped are rows of that table:** a row with `date_of_birth = None` refuses
`DependentGateUnanswered { gate: DateOfBirth }` and prints nothing, while a `Single` return whose
taxpayer-DOB **skippable** is `Declined` still resolves Step 1 for a row with
`younger_than_you_or_spouse = Some(true)`; and a fixture child with `date_of_birth = 2026-11-01` and
`lived_with_you_over_half_year = Some(true)` lands on the **CTC** edge, not the ODC edge, with
`line-coverage` checking that row (5)(a)'s `help` quotes `:1905-1913` verbatim.
`gross_income_under_limit` is not live on a params-less fixture and is listed as *waiting for the year
package*; with `FullReturnParams` inserted it becomes live and blocking, and its prompt contains the
year's figure. The classifier reds until each new `Dependent` leaf is classified. A `Single` return with no
dependents asks no dependent gate and not `filer_tin_issued_by_due_date` (liveness). The emitter KAT:
on a TY2025 fixture, the six row-(5)/(6) checkboxes and two row-(7) boxes per dependent are filled from
the answers; on TY2024 the emitter output is byte-identical to today.

### R7 — Head of household AND Qualifying surviving spouse are assertions; choosing either asks the instruction's own tests.

**Mechanism.** `FilingStatusArg::Hoh` and `FilingStatusArg::Qss` are both offered with *"no test at all"*
(recon §C.3; `crates/btctax-core/src/tax/types.rs:15`, `crates/btctax-cli/src/cli.rs:1176`). Each choice
is an assertion about the filer's household, and each unlocks money — HoH a wider bracket and standard
deduction, QSS the **joint** rates and the joint standard deduction — so each asks the instruction's
tests. Fixing one and leaving the other in the identical state is what r1 did.

**HoH — the marital test is an ENUM of the instruction's own states, not one compound `Option<bool>`.**
`:1149-1163` states three distinct legal predicates (legally separated under a decree; married but lived
apart *and you meet the other rules under* **Married persons who live apart**; a nonresident-alien spouse
with no election), and the second is itself five conditions (`:1247-1268`: lived apart the last 6 months;
a separate return; paid over half the cost of keeping up the home; the home was your child's main home
for more than half the year; you can claim the child). A single *Yes* spanning them is exactly the
compound answer R1 forbids — *"a compound 'no' spanning distinct legal predicates fabricates precision the
filer never swore to"* (`FIELD_PROVENANCE.md:289-292`). So:
`hoh_marital_basis: Option<HohMaritalBasis { NotMarried, LegallySeparatedByDecree, MarriedLivedApart,
NraSpouseNoElection }>`, live iff `filing_status == Hoh` (`return_inputs.rs:936`).
`MarriedLivedApart` **refuses** naming *Married persons who live apart* (`:1247`) until that rule's five
conditions are transcribed as their own five `Option<bool>` gates — T8 may transcribe them in the same
task, at which point the variant stops refusing and asks the five instead.
`NraSpouseNoElection` refuses naming *Nonresident aliens and dual-status aliens* (existing declaration
territory). Two `FormQuestion`s then follow as in r1: `hoh_qualifying_person` (*Test 1 or Test 2
applies*, `:1164-1200`) and `hoh_paid_over_half_cost_of_keeping_up_home` (`:1164,1172`); any `No` on
either refuses `HohTestNotMet` — *"choose another filing status"*. The name entry for a non-dependent
qualifying child (`:1196-1200`) is a `Text` field live iff HoH and no dependent row is the qualifying
person — collected, printed in the entry space.

**QSS — five `FormQuestion`s live iff `filing_status == Qss`**, each phrased from `:1288-1316`:
(1) your spouse died in one of the **two years preceding the tax year** and you did not remarry before
the end of the tax year (`:1295-1297` — the window is *derived from `tax_year`*, never a literal pair of
years, because each revision shifts it); (2) you have a child or stepchild — **not** a foster child —
whom you can claim as a dependent, or could but for the three reasons the instruction names; (3) that
child lived in your home all year; (4) you paid over half the cost of keeping up that home; (5) you could
have filed a joint return the year your spouse died. Any `None` refuses `QssTestUnanswered`; any `No`
refuses `QssTestNotMet` — *"choose another filing status"*.

**Not an R7 gate, for the avoidance of a false lead.** The TY2025 page-1 checkbox *"Check if your filing
status is MFS or HOH and you lived apart from your spouse for the last 6 months …"*
(`f1040--2025.txt:54-56`) is the **EIC special rule for separated spouses**
(`i1040gi--2025.txt:4866-4876`) — class (B) under `EicOmitted`, a census cell, not a filing-status test.

**Kill.** `Single`/`Mfj` ask none of them. `Hoh` with `hoh_marital_basis = None`, or with either test
`None`, refuses; `hoh_marital_basis = MarriedLivedApart` refuses with *Married persons who live apart* in
the message and `NraSpouseNoElection` with *Nonresident aliens*; `Hoh` with `Some(false)` on a test
refuses with the exit. `Qss` with any of the five `None` refuses `QssTestUnanswered`, with any
`Some(false)` refuses `QssTestNotMet` naming the test, and with all five `Some(true)` computes at the
joint rates. The no-brick property (`answer.rs:412`) covers every one of them.

### R8 — Real estate: a document, four gates, two refusals.

**Mechanism.**

- **Form 1098 is a document screen** (`form_1098: Vec<Form1098>`, top-level — it arrives whether or
  not the filer itemizes; what the itemize election governs is its **liveness**, below, not its home).
  Boxes: lender, lender TIN, **1** interest → Schedule A 8a with **6** points
  (`i1040sca--2025.txt:1058-1061` — *"mortgage interest and points reported to you on Form 1098"*);
  **2** outstanding principal and **3** origination date → the §163(h)(3)(B) ceiling is now **checked
  with a figure**, over the **sum of box 2 across every 1098 row** — the limit is on aggregate
  acquisition debt, so two 1098s at $500,000 each are over it while each row alone is silent — against
  $750,000 / $1,000,000 by whether box 3 precedes 2017-12-16, **halved for MFS** ($375,000 / $500,000),
  all `FullReturnParams` values, displayed as a **warning** beside the existing `MortgageWithinDebtLimit`
  declaration (`return_inputs.rs:647`), which stays the filer's testimony; a per-row gate
  `other_borrower_paid_interest` — *"Did someone other than your spouse also pay interest on this loan?"*
  — refuses `SharedMortgageInterest` on `Some(true)`, because a co-borrower deducts *"only your share"*
  (`i1040sca--2025.txt:1071-1080`) and 8a sums box 1 in full; **4** refund of overpaid interest → `> 0`
  **refuses** `MortgageInterestRefundNotComputed`, *"Form 1098 box 4 is income on Schedule 1 line 8z to
  the extent the interest reduced your tax in an earlier year (Pub. 936, Refund of home mortgage
  interest); btctax does not compute it"* — the instruction is explicit that the refund is **not** netted
  against the deduction (*"don't reduce your deduction by the refund. Instead, see the instructions for
  Schedule 1 (Form 1040), line 8z"*, `:1067-1070`), so a figure the tool already holds may not reach a
  blank 8z with a census note: that is a typed number with no reader in the understatement direction and
  C1's direction rule reds it. Same shape as box 9 on the 1099-INT; **5** mortgage insurance → collected
  against the reserved 8d (`:1153-1155`; if the TY2026 final reinstates the line — the draft Schedule A is
  REBUILT, `design/TY2026_WORK_LIST.md:36` — the field is already there); **7/8** the property address;
  **10** other (informational, shown beside 5b).
  `ScheduleAInputs.mortgage_interest_1098` (`:613`) is **removed**; 8a = Σ(box 1 + box 6) over the rows.
  **Liveness is keyed on the itemize election, not on the document's presence** — and the election gates
  the **consequences of a row, never the section**: ★ *(corrected in the T9 fold; this paragraph
  previously said both that the document is top-level and that the 1098 SECTION is live iff
  `schedule_a.is_some()`, and the build followed the first)*. The **section stays top-level and is
  offered to every filer**, because the document arrives whether or not they itemize and `open_next_year`
  seeds a prior lender onto a year that has elected nothing yet. What carries `schedule_a.is_some()` is
  the `form_1098` **census row**, and **every rule that reads a row for a Schedule A line 8 purpose**: the
  `MortgageAllUsed` / `AmtQualifiedDwelling` / `MortgageWithinDebtLimit` declarations, the **Form 8396
  gate**, the **`other_borrower_paid_interest` refusal**, and the **§163(h)(3)(B) ceiling warning**. All
  of them read the conjunct through the single accessor `ReturnInputs::form_1098_deducted()` rather than
  re-typing it, so the next such rule cannot forget it. **Box 4 is the one deliberate exception**: a
  refund of overpaid interest is income on Schedule 1 line 8z, owed whether or not the filer itemizes, so
  its refusal reads every transcribed row. This is what journey **J-24** asserts — a standard-deduction
  filer holding a $900k 2019 1098 is asked nothing and refused nothing. Today's condition is
  `.is_some_and(|a| a.mortgage_interest_1098 > Usd::ZERO)` (`questions.rs:280`), which already requires a
  Schedule A; dropping that half would make an always-live census row force every homeowner to transcribe
  a 1098, then refuse a **standard-deduction** filer on a truthful `MortgageWithinDebtLimit = Some(false)`
  for a $900,000 2019 loan (`return_inputs.rs:644-647`) — a refusal over a deduction they are not
  claiming, and a return they cannot file. `AmtQualifiedDwelling` is Form 6251 line 3, which itself reads
  *"If you deducted home mortgage interest on Schedule A"*; and a standard-deduction filer's 1098 rows
  would be a `Field` set nothing reads, which R2 mechanism 3's reverse join reds. Compile errors enumerate
  every reader.
- **Schedule A 8b** — interest paid to a recipient who gave no 1098: a repeating
  `mortgage_interest_not_on_1098: Vec<{ recipient_name, recipient_tin, recipient_address, amount }>`
  on `ScheduleAInputs`, because the instruction demands the recipient's *"name, identifying number, and
  address on the dotted lines"* (`i1040sca--2025.txt:1109-1116`). ★ *(Cell pairing corrected in the T9
  fold; this sentence previously paired `f1_17`/`f1_19` for 8b and `f1_18`/`f1_20` for 8c, and both
  years were re-measured with `xtask dump-fields` against it.)* On **TY2024**, 8b is `f1_19` (the
  AMOUNT) with `f1_17` and `f1_18` as its **two dotted description lines**, both wide free-text cells at
  x = [115.2, 396.0] below the amount; **8c is `f1_20`**, which has **no description cell on either
  form**. On **TY2025** the IRS merged the two dotted rows into ONE 24pt box,
  `Line8b_ReadOrder[0].f1_16`, so a return with **two** recipients already overflows — and an overflow
  prints the instruction's own escape, *"See attached"* (`:1126-1131`), which makes the **statement a
  packet-manifest hand mark**: btctax cannot write it, and a page that asserts an attachment nobody was
  told to write is testimony the filer never gave. **8c** — points not on a 1098: one `Usd`, with the
  instruction's *"generally deductible over the life of the loan"* (`:1137-1139`) in its help.
  8e = 8a + 8b + 8c (the printed chain, today `= 8a`).
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
silent at $700,000; **two rows at $500,000 each warn while one row at $500,000 does not** (the limit is
aggregate); an MFS fixture warns at $400,000 where the same Single fixture does not; the declaration
still refuses on `None`. `other_borrower_paid_interest = Some(true)` refuses `SharedMortgageInterest`.
**A fixture 1098 with box 4 = $1 refuses `MortgageInterestRefundNotComputed` with *Schedule 1 line 8z* in
the message; the same fixture with box 4 = 0 does not.** A **standard-deduction** fixture holding a 1098
with box 2 = $900,000 asks none of the three declarations, transcribes no 1098 row (the census row is not
live) and does not refuse; the same fixture with a `ScheduleAInputs` present asks them and refuses on
`MortgageWithinDebtLimit = Some(false)`. The 8396 gate refuses on Yes. The home-sale
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
  rule (`crates/btctax-core/src/tax/return_1040.rs:907`, the tier that sees the ledger) cross-checks it
  against **the predicate that already exists**, `digital_asset_activity(state, year)`
  (`return_1040.rs:2468-2475`: any disposal ∨ income recognized ∨ removal in the year — the instruction's
  own list, with the reasoning in its doc comment). Not *"the ledger has events"*: the instruction
  (`i1040gi--2025.txt:1346-1362` and the bullets after it) checks *Yes* for **receipts as a reward, an
  award or a payment, and for dispositions** — **not** for buying digital assets with real currency and
  **not** for transfers between the filer's own wallets. So:
  - `digital_asset_activity(state, year)` ∧ `No` ⇒ **refuse** `DigitalAssetAnswerContradictsLedger`,
    naming the **first qualifying event** (date, venue, kind) so the filer can check it.
  - `!digital_asset_activity(state, year)` ∧ `Yes` ⇒ **accept and WARN**, never refuse:
    *"the ledger shows no receipt or disposition in 2026; if you had off-ledger activity — a self-custody
    receipt, a payment in kind — it is not on this return: Schedule 1 line 8v and Form 8949 come only from
    the ledger."* The ledger is **not complete by construction** — no self-custody wallet is importable at
    all (recon §E.1) — so a filer paid in BTC to their own wallet has no export to import, and refusing
    their truthful *Yes* leaves *No* as the only way through the gate: a false answer the tool coerced
    into sworn testimony.
  - The two agreeing cells print. A **buy-only year** is one of them: a filer who bought monthly on Swan
    and sold nothing has a ledger full of `Acquire` events, no `digital_asset_activity`, and the correct
    answer `No`, which the tool prints.
  Today the box is never "No" (recon §B.1); a holder with no 2026 receipts or disposals can now answer it.
- **Venue/account granularity is documented, not asked**: the account segment is hardcoded `default`
  (`crates/btctax-adapters/src/normalize.rs:63-69`); asking "how many accounts at X?" would collect an
  answer nothing reads.

**Kill.** The DA table as a test, **five rows**: activity ∧ `No` refuses and the message names the first
qualifying event; activity ∧ `Yes` prints `Yes`; no-activity ∧ `No` prints `No`; no-activity ∧ `Yes`
prints `Yes` **with the off-ledger warning and no refusal**; and a ledger holding only `Acquire` events
and linked self-transfers, answered `No`, prints `No` — a purchase is not a Yes-forcing event. The
standing-order warning fires on a fixture ledger with a
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
   Question(QuestionId) | Skippable(SkippableId) | DependentGate { ssn_hash: String, gate }` (a stable
   string form for serde) and `AnswerRecord { answered_on: Date, prompt_hash: String, state: Given |
   Declined }`. Written by **one** core function `record_answer(ri, key, prompt, now)` that both `apply`
   (`crates/btctax-input-form/src/apply.rs:28`) and `income answer` call; `now` is the `BTCTAX_NOW`
   seam (`crates/btctax-cli/src/main.rs:64-76`). A skippable **skipped on purpose** records `Declined`;
   `None` with no record is *never asked* — the distinction `FIELD_PROVENANCE.md:395-398` says the
   record cannot make today. Forbidden, per `:400-403`: progress, position, "what remains", superseded
   values, half-typed tokens.

   **The dependent key is an IDENTITY, never a row index.** `Dependent` rows are a `Vec` with `add` and
   `remove` through the seam (`seam.rs:295-297`), so `DependentGate { row, gate }` would move one child's
   `answered_on` and `prompt_hash` onto another the moment row 0 is deleted — and *"a diligence record
   that lies is worse than none"* (`FIELD_PROVENANCE.md:440-441`). The key is `ssn_hash`, a **salted hash
   of the row's `ssn`** and never the digits (the secret rules); `remove` on the Dependents section
   deletes that identity's entries; a row whose `ssn` changes starts a fresh record and the old entries
   move to the history below. Part 4's opener already confirms year N's identities by TIN, so the model
   holds the right key and now uses it in both places. (The *refusal* `DependentGateUnanswered { row,
   gate }` keeps the row index: it points the filer at a position on screen, and is not stored.)

   **A `prompt_hash` mismatch RE-ASKS — the hash has exactly one reader, and this is it.** The hash exists
   so *"which words were asked"* is on record (`FIELD_PROVENANCE.md:393-395`), and the only use that
   matters is detecting that the words changed — the case the Sep–Dec calendar (R11) makes likely: a
   prompt edited in a November fold, under an answer given in September on the same year's draft. So:
   `interview_state` treats an `AnswerRecord` whose `prompt_hash` ≠ `hash(the current prompt)` as
   **unanswered** — blocking for class (A), forgoing for class (B) — with the reason *"the wording of
   this question changed since you answered"*, and `screen_inputs` refuses such a class-(A) record as
   UNANSWERED. **The old answer is kept as history and never as the current answer:** the superseded
   `AnswerRecord` moves to `answer_log_history: Vec<(AnswerKey, AnswerRecord)>`, append-only, which
   nothing reads as an answer, and the re-answer writes a fresh record. A stored value with no reader is
   not a guarantee.
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
Change one prompt's text in a fixture registry → that answer reappears in `blocking` (class A) or
`forgoing` (class B) with the changed-wording reason, and the superseded record is in
`answer_log_history`; restore the text → it does not reappear and no history entry is added. Answer gates
on two dependent rows, delete row 0, and row 1's records are unchanged while row 0's are gone; change row
1's `ssn` and its records move to history. The N+1 opener: a fixture year N with two payers yields exactly
two identity prompts, both `None`, and the opened screen's boxes are all default. Grep-KAT: no field named
`progress`, `remaining`, `position` on `ReturnInputs`.

### R11 — The year gate: interview-complete is a state the tool reaches and reports on a year whose parameters have not arrived; commit still waits for them. A long-lived draft is protected.

**Mechanism.** `commit` keeps I-11 (`input_form_store.rs:369-376`): a params-less year writes nothing,
because `screen_inputs` needs `&TaxTable + &FullReturnParams` (`return_refuse.rs:1034`) and an
unscreened row at precedence 1 (`crates/btctax-cli/src/resolve.rs:72-84`) is the poison
`SPEC_input_surface.md` §3.2 describes. What changes:

- **Two states at entry**, both derivable without params: *interview-complete* — `interview_state(ri)`
  (R12) has no blocking item; *return-computable* — `YearReadiness.params`
  (`crates/btctax-cli/src/year_readiness.rs:27`), already rendered by `sentence()` (`:115`). The entry
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
- **`income import` runs the param-free screens BEFORE it writes.** Today it writes a committed,
  unscreened row on a params-less year (recon §A3), so a TOML carrying `documents.k1 = true` is stored
  and poisons the year until `report`. R3's three census invariants, the unsupported-row refusals and
  `NegativeAmount` need neither a `TaxTable` nor `FullReturnParams`: they run **at import** and refuse
  there, with nothing written. The param-dependent rules (§402(g), SALT, excess SS, the §152(d) figure)
  still wait for commit, as above.
- The slice-filing path for TY2026 (spec 1099-DA R6, `design/SPEC_1099da_broker_reporting.md:278`) is
  unchanged: a committed row via `income import`, or the TUI `commit` once `FullReturnParams` TY2026 is
  inserted (the *filable* flip, `crates/btctax-forms/forms/2026/YEAR.toml`).

**Kill.** On a params-less year, `income import` of a TOML with `documents.k1 = true` refuses at import,
naming the exit, and writes no committed row (the same TOML with every census row supported imports).
Also: `commit` → `NoTables`, writes nothing (exists); `save_draft` persists
across `Session::open` (exists, I-7); `income import` over a draft with one answered census row and no
`--force` refuses and the draft survives; with `--force` it is deleted and a note names what was lost;
`income answer` on a draft-only year writes the draft and `return_inputs::get` still returns `None`;
`interview_state` on a TY2026 fixture with every gate answered reports complete while
`YearReadiness.params` is false.

### R12 — The answer panel is derived from the registries, not from `screen_inputs`. Class (B) is never flattened into class (A), and a `Declined` benefit is still forgone.

**Mechanism.** `SPEC_input_form.md` §7 forbids refactoring `screen_inputs` to collect all refusals
(its early-return tiers are semantic) — kept. The panel needs none of it: `live_questions` already
enumerates every live question across both classes (`FIELD_PROVENANCE.md:108-110` — *"the primitive
exists"*). A core function `interview_state(ri) -> InterviewState { blocking: Vec<Blocking>, forgoing:
Vec<Forgo>, refusing: Vec<Refusing>, waiting: Vec<Waiting>, answered: usize, not_live: usize }` walks
**three** registries — `FORM_QUESTIONS` (which is where the census rows live: R3 makes each census row a
`FORM_QUESTIONS` entry, so it is one walk, not two), `SKIPPABLE_QUESTIONS`, and `DEPENDENT_GATES` × rows
— plus the declared-document/rows invariant (R3):

| state | listed as |
|---|---|
| not live | counted, silent |
| live, answered, prompt unchanged | counted |
| live, answered, **`prompt_hash` ≠ the current prompt** | **blocking** (class A) / **forgoing** (class B), reason *"the wording of this question changed since you answered"* (R10.3) |
| live, unanswered, **class (A)** | **blocking** — the question, its anchor, and what it accounts for |
| live, unanswered, **class (B)** | **forgoing** — the benefit, its size where computable (the §63(f) add-on from params; line 19 per R6; blank on a params-less year) |
| live, **answered, and the answer refuses** — a census row `Some(true)` on an unsupported type, `BasisDiffers` on a broker row, any `Yes` whose gate carries a refusal | **refusing** — the row, its `RefuseReason` and its exit sentence, **derived from the `FormQuestion`'s own refusal**, so the filer sees it while authoring instead of meeting it at commit (J-32) |
| live, class (B), **`Declined`** | **forgoing, marked *(declined)***, with the size — declining is provenance (asked, refused); the benefit is still forgone, and dropping it from the list exactly when the forgo becomes final is backwards. Only `Given` removes an item |
| live but **unstatable** — a gate whose prompt must quote a `FullReturnParams` figure the year does not yet have (R6's `gross_income_under_limit`, M7) | **waiting for the year package** — named with the package it waits on, never blocking; it becomes blocking the moment params arrive |

No progress bar (`FIELD_PROVENANCE.md:123-125`). The TUI renders it as a pane; `income answer` prints
it before the first question and after the last; the commit modal prints the forgoing **and refusing**
lists (J-12, J-17, J-32). The no-brick property (`answer.rs:412`) extends: answering every blocking item through its
own setter empties `blocking`, and on a params-bearing year `screen_inputs` then reports no
UNANSWERED-class refusal.

**Kill.** A fixture with N unanswered live gates across the three registries and the document invariant
lists exactly N blocking items in one call. A live skippable with `Declined` is listed in `forgoing`
marked *(declined)* with its size and is **never** in `blocking`; answering it `Given` removes it. A
census row `Some(true)` on an unsupported type appears in `refusing` with its exit sentence **before**
commit, and the extended no-brick test asserts `refusing` is empty before `screen_inputs` passes. An
`AnswerRecord` whose `prompt_hash` no longer matches appears in `blocking` (class A) or `forgoing` (class
B) with the changed-wording reason. On a params-less TY2026 fixture `gross_income_under_limit` is in
`waiting`, not `blocking`; with params inserted it moves to `blocking`. The extended no-brick test. A
snapshot with the forgo size present when params exist and absent when they do not.

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

**`GoldenInputs` gains the dependents block both oracles take, or the line-19 excuse is a check that
cannot fail.** `GoldenInputs` (`testonly.rs:650-693`) today carries filing status, wages, interest,
dividends, gains, SE income, four Schedule A components and a cash gift — **no dependents, no ages, no
blindness** — and `gen_goldens.py:199-232` projects the same. So `project_to_golden` could not carry a
dependent, Tax-Calculator's `n24` would stay 0, `oracle_line19` would be 0, btctax's line 19 is
blank-as-0, and *"the CTC excuse is `oracle_line19 − 0`"* would be `0 − 0` on **every** return, including
one with three qualifying children; the same blindness covers EIC (line 27), the §63(f) aged and blind
add-ons, and HoH's qualifying person. So it gains: `n24` (children under 17 on the CTC edge), `nu18`,
`n1820`, `n21` (age bands computed from each row's `date_of_birth` and `tax_year`), `age_head` /
`age_spouse` (from the DOB skippables when `Given`, absent when `Declined`), `blind_head` /
`blind_spouse`, and the EIC-qualifying-child count — all projected from the T7/T8 answers — and
`gen_goldens.py`'s row and the OTS template's dependent lines carry them likewise. The line-19 excuse is
then **the oracle's computed CTC**, and a diff of any other size fails.

**Kill.** `project_to_golden(build_golden_return(g).0) == g` on every golden household in
`crates/btctax-core/tests/goldens/` (projection inverse). `ORACLE_INVISIBLE` — every `Usd` leaf either
projects or is listed (declarations, 1099-DA answers, every `Option<bool>`) — asserted complete inside
the KAT. The projection inverse now covers a golden household with two dependents (ages, blindness and
the CTC edge round-trip). On a fixture with one CTC-edge child, `oracle_line19 > 0` and the excuse
**equals it exactly**; a diff of any other size on line 19 fails; and deleting the dependents block from
the projection reds the inverse rather than passing silently.

### R14 — Answered-ness is structural: every new field joins a classifier class by construction.

**Mechanism.** The classifier destructures every struct with no `..` and forbids `_` on `bool` /
`Option<bool>` / `Option<Usd>` / defaulted-enum leaves (`classifier.rs:5-9,16-19`). Each new field's
class:

| new field(s) | class | mechanism |
|---|---|---|
| `DocumentCensus.*`, the three paired document-less income questions (`w2_wages_without_w2`, `interest_or_dividends_without_1099`, `state_refund_without_1099g`), return-level `itemized_prior_year`, HoH's two tests, QSS ×5, home-sale ×4, `claiming_mortgage_interest_credit`, `digital_asset_activity`, `filer_tin_issued_by_due_date`, `form_1098[].other_borrower_paid_interest` | **(A) declaration** | `FORM_QUESTIONS` entries; `screen_inputs` refuses on live `None` |
| `header.hoh_marital_basis: Option<HohMaritalBasis>` | **(A) declaration**, an `Option<Enum>` (R1's one exception) | a `FormQuestion` whose answer is one of the instruction's four named states; the classifier forbids `_` on it, so a new variant reds every match |
| `Dependent` gates ×17 — §5.3's sixteen plus `lived_with_you_over_half_year` | **(A) declaration, per row** | `DEPENDENT_GATES`; `screen_inputs` loops rows; the key in `answer_log` is the row's `ssn_hash`, not its index (R10.3) |
| `Dependent.full_time_student`, `.permanently_and_totally_disabled`, `.lived_with_you_in_us` | **(A)**, printed as checkboxes — `Some(false)` prints unchecked | same — **20 `Option<bool>` leaves on `Dependent` in total** (17 gates + these 3), plus `date_of_birth`, which is now **required**: `None` blocks (R6) rather than forgoing |
| `answer_log[..].state`, `answer_log_history[..]` | not leaves the classifier forbids (no default); history is append-only and is never read as an answer | — |
| `schedule_b_filer_records[].*` | money / text from the filer's records (R3/I1) | `LEAF_SOURCE` = `FilerRecords` |
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
 1  ReturnOptions       filing status (+ the HoH marital-basis enum and its two tests, or the five QSS
                        tests, R7); itemize election                          (exists + T8)
 2  Taxpayer / Spouse / Address (+ foreign country/province/postal; phone; spouse IP PIN)   (exists + T10)
 3  Dependents          rows (1)–(4) + DOB (required); rows (5)(6) gates with the row-(5)(a) exception
                        in the help; Steps 1–5 gates; row (7) shown computed (R6)
 4  DocumentCensus      one tri-state per type (R3) — the income block opens here; a `No` on w2 /
                        int_1099 / div_1099 / g_1099 opens its paired document-less income question
 5  W2s / W2Box12                                                              (exists)
 6  Int1099 · Div1099 · B1099 · G1099 · Form1098E                              (R4)
 7  ScheduleA           SALT, medical, investment; Form1098 rows; 8b rows; 8c; charitable; the 8396 gate (R8)
 8  HomeSale            the three gates (R8)
 9  Payments            (exists)
10  Carryforwards · QbiLimitation                                              (exists)
11  BrokerReporting     seeded from the ledger                                 (exists)
12  Declarations · IncomeExclusions · Skippables                               (exists; the new FormQuestions land in Declarations by the adapter)
13  DirectDeposit       35b–d (T10)
    The answer panel (R12) is a pane visible from every section; `s` commit shows it in the modal.
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
document with no table refuses at **import** where the rule is param-free (R11) and at commit otherwise —
never at parse.

### 4.4 `report`'s rendering

`report --tax-year N` prints, after the existing chains: the document census (type → declared / rows),
the answer panel, the home-sale decision and its answers, row (7) per dependent, and the
`LEAF_SOURCE` provenance of every collected figure (document with payer TIN masked / filer's records),
naming any document row that carries no `transcribed_on` as *transcribed without a date* rather than
omitting it (M8/R4).
The packet's `manifest.txt` "COMPLETE BY HAND" block (`crates/btctax-cli/src/cmd/admin.rs:494-500`)
gains the forgoing list, each `Declined` item marked *(declined)*, so the filer sees it while assembling
paper (J-15).

---

## 5. Data model

All on `ReturnInputs` (`return_inputs.rs:916`). `V` = repeating (`Vec`), `S` = singleton, `O` =
optional singleton. Every `Usd` shown is `Money ≥ 0` in the form. Provenance is by struct (R10).

### 5.1 `documents: DocumentCensus` (S, new) — one `Option<bool>` per row, each a `FormQuestion`

`w2`, `int_1099`, `div_1099`, `b_1099`, `g_1099`, `form_1098`, `form_1098e` (supported);
`r_1099`, `ssa_1099`, `nec_misc_k_1099`, `k1`, `schedule_e_rental`, `s_1099`, `oid_1099`, `w2g`, `c_1099`,
`a_1095`, `t_1098` (refuse on `Some(true)`, §2.2). Coverage fixture: every row `Some(false)` except the
supported rows that carry a fixture row.

**Row liveness.** Every row is live except `form_1098`, which is live **iff `schedule_a.is_some()`** (R8
/I8) — a standard-deduction filer is never made to transcribe a 1098 and never refused over its debt
limit.

**Paired with the four supported income rows** (R3, the document-less income door), on `ReturnInputs`:
`w2_wages_without_w2`, `interest_or_dividends_without_1099`, `state_refund_without_1099g` — three
`Option<bool>` `FormQuestion`s, each **live iff its census row is `Some(false)`** — plus
`itemized_prior_year: Option<bool>`, moved here off `Form1099G` (live iff
`state_refund_without_1099g == Some(true)` **or** any `g_1099[].box2_state_refund > 0`), and
`schedule_b_filer_records: Vec<ScheduleBRecord { payer_name: String, payer_ssn: String, payer_address:
String, amount: Usd, kind: Interest | Dividend }>` (V, `Source::FilerRecords`, live iff
`interest_or_dividends_without_1099 == Some(true)`, and non-empty is then required). `payer_ssn` and
`payer_address` are the seller-financed-mortgage case Schedule B asks for by name
(`i1040sb--2025.txt:24, 77`) and are empty otherwise.

### 5.2 Documents (V) — the box fields carry the box caption verbatim from the archived extract (T2)

| struct | new fields | reaches | fixture must carry |
|---|---|---|---|
| `Form1099Int` (`:85`) | `payer_tin`, `transcribed_on` | as R4 | one row, box 9 = 0 |
| `Form1099Div` (`:104`) | `payer_tin`, `transcribed_on` | as R4 | one row, boxes 2b/2c/2d/13 = 0 |
| `Form1099B` (`:156`) | `payer_tin`, `transcribed_on` | Sch D 1a/8a | one row, gate `Some(true)` |
| `Form1099G` (`:131`) | `payer_tin`, `transcribed_on`, `box2_state_refund: Usd` — **`itemized_prior_year` is NOT a row field**: it is return-level (§5.1), because the question outlives the document | Sch 1 L7; box 2 per R4 | one row, with return-level `itemized_prior_year = Some(false)` |
| `Form1098` (new) | `lender`, `lender_tin`, `transcribed_on`, `box1_interest`, `box2_outstanding_principal`, `box3_origination_date: Option<Date>`, `box4_refund_overpaid_interest`, `box5_mortgage_insurance`, `box6_points`, `box7_property_address_same_as_payer: bool`, `box8_property_address: String`, `box10_other`, `other_borrower_paid_interest: Option<bool>` (M5) | Sch A 8a; the **aggregate** ceiling check across rows; box 4 > 0 refuses (I3); 8d if reinstated | one row, box 2 under the ceiling, box 4 = 0, `other_borrower_paid_interest = Some(false)` |
| `Form1098E` (new) | `lender`, `lender_tin`, `transcribed_on`, `box1_interest` | Sch 1 L21 | one row |

`ScheduleAInputs.mortgage_interest_1098` and `Schedule1Inputs.student_loan_interest_paid` are removed
(`:613`, `:689`).

The field list above is what the **struct** carries; the authority for *which boxes exist* is each
document's `[boxes]` census, enumerated from the T2 extract (R4), which also decides 1099-INT box 10
(`collected`), boxes 11–13 (`refuse_if_nonzero`), W-2 box 13 (`collected: Bool`, refuses when checked)
and 1099-DIV box 3 (`not_read`). A caption in the extract with no entry reds, so this table cannot go
stale silently.

### 5.3 `Dependent` (V, `:224`) — the grid and the gates

Identity (exists): `name`, `ssn`, `relationship: String` (printed in column (4) as written — the
relationship *class* is the Step 1 / Step 4 gate, so no enum migration), `date_of_birth` — which is now
**required**: `None` is `DependentGateUnanswered { row, gate: DateOfBirth }` and blocks (R6/I4), not a
class-(B) forgo. `ssn` is additionally the row's **identity key** for `answer_log` (a salted hash of it,
R10.3). Row (5)/(6):
`lived_with_you_over_half_year`, `lived_with_you_in_us`, `full_time_student`,
`permanently_and_totally_disabled`. Gates (R6, all `Option<bool>`, `#[serde(default)]`):
`qc_relationship`, `provided_over_half_own_support`, `filing_joint_return`,
`joint_return_only_to_claim_refund`, `qualifying_child_of_another_person`,
`citizen_national_resident_or_canada_mexico`, `married`, `tin_issued_by_due_date`,
`citizen_national_or_resident_alien`, `ssns_valid_for_employment_issued_by_due_date`,
`qr_relationship_or_member_of_household`, `qualifying_child_of_any_taxpayer`,
`gross_income_under_limit`, `you_provided_over_half_support`,
`divorced_separated_multiple_support_or_kidnapped_rule_applies`, `younger_than_you_or_spouse` (R6/I4 —
asked, never computed from the taxpayer's declinable DOB skippable). **Sixteen gates**, plus the four
row-(5)/(6) fields = **20 `Option<bool>` leaves** (R14). Computed, not stored: the age brackets,
under-17, row (7). Fixture: one dependent at the CTC edge (all gates answered, DOB 2015, both credit
inputs Yes), and a second fixture at the **born-in-year** edge (DOB 2026-11-01,
`lived_with_you_over_half_year = Some(true)` under the exception in the help) which must land on CTC, not
ODC.

### 5.4 `HouseholdHeader` (S, `:233`) additions

`hoh_marital_basis: Option<HohMaritalBasis { NotMarried, LegallySeparatedByDecree, MarriedLivedApart,
NraSpouseNoElection }>` (R7/I7 — an enum of the instruction's four states, **not** one compound
`Option<bool>`; the last two refuse naming their rule), `hoh_qualifying_person`,
`hoh_paid_over_half_cost_of_keeping_up_home` (R7, `Option<bool>`, live on `Hoh`);
`qualifying_child_name: String` (live on `Hoh` **or `Qss`** — ★ corrected 2026-09-07 by the T8 seam
review's I-3: the form's own sentence is one entry space for three statuses, *"If you checked the MFS
box, enter the name of your spouse. If you checked the **HOH or QSS** box, enter the child's name if
the qualifying person is a child but not your dependent"* (`f1040--2024.txt:28-29`), and QSS condition
2 (`i1040gi--2025.txt:1298-1306`) is exactly the household it serves. T8 built it as
`hoh_qualifying_child_name`, live on `HoH` alone, with **no reader on any year**; the `hoh_` prefix was
part of the mistake and is gone); the five QSS gates
`qss_spouse_died_in_window_and_not_remarried`, `qss_child_you_can_claim`,
`qss_child_lived_in_your_home_all_year`, `qss_paid_over_half_cost_of_keeping_up_home`,
`qss_could_have_filed_jointly_in_year_of_death` (`Option<bool>`, live iff `filing_status == Qss`, R7/I7);
`filer_tin_issued_by_due_date: Option<bool>`
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

`answer_log: BTreeMap<AnswerKey, AnswerRecord>`, keyed `Question(QuestionId) |
Skippable(SkippableId) | DependentGate { ssn_hash, gate }` (identity, never a row index — R10.3/I10);
`answer_log_history: Vec<(AnswerKey, AnswerRecord)>` (append-only; holds every record superseded by a
`prompt_hash` mismatch or a changed `ssn`, and is never read as an answer — R10.3/I9);
`CarryProvenance::ComputedFromPriorReturn { year }`; `digital_asset_activity: Option<bool>`.
`SCHEMA_VERSION = 3`.

### 5.7 The coverage KAT's fixture (`coverage.rs:88`, `maximal_fixture`)

Carries one row of every `Vec` above with every `Option` `Some`, every census row answered (and, on the
variant whose supported rows are `Some(false)`, each paired document-less income question answered), one
dependent at the CTC edge with a `date_of_birth`, HoH answered on an `Hoh` fixture variant (marital basis
`NotMarried` plus the two tests) and the five QSS gates on a `Qss` variant, one
`schedule_b_filer_records` row, the home sale on its blank branch, `direct_deposit` present. `EXEMPT_PREFIXES` after v1: `capital_loss_carryforward_in`,
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
| J-11 | a child born in 2026 with no SSN; a 22-year-old student | the grid + Steps 1–5; **row (5)(a)'s help carries the *Exception to time lived with you* verbatim, so the newborn answers Yes and lands on CTC, not ODC**; the DOB is required, so neither row prints on an unevaluable age test; the TIN gate says *issued before the due date including extensions* — the extension calendar helps (say so in the help) | R/Doc | R6 |
| J-12 | line 19 never populated | the forgo is visible with its size before filing, in the panel and the commit modal | W | R6, R12 |
| J-13 | a seller-financed second loan | 8b rows with the recipient's name/TIN/address; the 1098 screen | — | R8 |
| J-14 | sold the main home | three gates + the 1099-S row → blank-by-decision or a refusal naming Pub. 523 | R | R8 |
| J-15 | a rental / K-1 | typed census rows that refuse naming the form | R | R3 |
| J-16 | state estimates paid in Jan 2027 typed into 2026 SALT | 5a's help carries *paid in 2026* | Doc | R5 |
| J-17 | commit hits the first refusal, fixes, hits the next | the answer panel, every state at once | — | R12 |
| J-18 | skips the oracle | `income project` + `check_return.py` before export; disagreements adjudicated against the form | — | R13 |

The r2 fold's own walk, re-run with the recon's gap list — twelve further moments, each now a
requirement of the rule beside it:

| J | moment | requirement | class | rule |
|---|---|---|---|---|
| J-19 | $6 of credit-union interest, no 1099-INT; answers the census *No* | the paired *"interest or dividends for which no 1099 was issued"* question opens a Schedule B filer's-records row | R | R3 |
| J-20 | reads box 10 ($120 market discount) and box 11 ($40 bond premium) on the 1099-INT | box 10 is `collected` into Schedule B line 1; box 11 refuses `AmortizableBondPremiumNotComputed`; neither is dropped | R | R4 |
| J-21 | 1098 box 4 shows a $300 refund of overpaid interest | box 4 > 0 refuses naming Schedule 1 line 8z — never shown-and-dropped | R | R8 |
| J-22 | child born 2026-11-01; *"lived with you more than half of 2026?"* reads literally *no* | the exception is in the gate's own help, so the filer answers the instruction's condition, not a fragment of it | Doc | R6 |
| J-23 | bought BTC monthly on Swan, sold nothing; answers the DA question *No* | accepted and printed *No* — a purchase is not a Yes-forcing event, and the tool never coerces the answer | — | R9 |
| J-24 | takes the standard deduction and holds a $900k 2019 1098 | the census row and the three declarations are not live; nothing is transcribed and nothing refuses | — | R8 |
| J-25 | answered the foreign-accounts gate in September; a November fold reworded it | the answer returns to `blocking` with *"the wording of this question changed since you answered"*; the old record is history | W | R10 |
| J-26 | removes dependent row 1 (moved out); row 2 becomes row 1 | the diligence records follow the identity, not the index | — | R10 |
| J-27 | February 2028, opens TY2027 | `open_next_year` seeds identities and gates blank, amounts only as `ComputedFromPriorReturn` carryforwards | — | R10, T4b |
| J-28 | runs `check_return.py` with two children; line 19 matches | the match is a real comparison — the projection carries `n24` and the excuse equals the oracle's CTC | — | R13 |
| J-29 | is on a Marketplace plan and holds a 1095-A | §9 Q1 put the whole §2.2 table in front of the owner in September, so this is a decision, not a February discovery | R | §9 Q1 |
| J-30 | widowed in 2025; picks *Qualifying surviving spouse* | five gates, phrased from the instruction; any `None` or `No` refuses | R | R7 |
| J-31 | applied the 2025 overpayment to 2026 estimates; types only the four vouchers | line 26's help carries the *"include any overpayment you applied"* clause | Doc | R5 |
| J-32 | answers `k1 = Yes` at the census, sees nothing blocking, commits | the panel's **refusing** list shows it with its exit while authoring — the J-17 round trip does not return | W | R12 |

Not requirements: box-14 entries; January spill rows; the Q4 payment date; the appraisal timing beyond
the existing owner-action row; a corrected 1099 arriving after transcription (re-type the row); the
spouse's separate 1099-INT on MFJ (Schedule B does not distinguish owners); an amended prior-year return. **The walk is re-run live with the owner on the S1 rehearsal documents
before T12 closes** (owner Q3) — a live walk diverges where a solo one cannot.

---

## 7. Build plan

Ordered so the owner's own return (S2) is fillable earliest: the schema first because it cannot be
back-filled (and T4b immediately after it, because the year-N+1 opener is the every-year machinery the
strategy ranks above any single year and it reads only what T1 defines); then what a W-2-plus-crypto
filer with interest and dividends needs on TY2026 in Sep–Dec (census, year gate, 1099 sections, the
exchange seam); then dependents and real estate (each conditional on Q1 in effect, but built regardless
— they are page-1 and Schedule A lines of the form the owner files); then the trailer, the oracle path,
the panel. Each task: one opus agent under a
persisted brief; TDD; the kill red before green; **once** = code that survives every year, **per-year**
= data a new revision re-earns.

| T | task | once / per-year | kill (the test that reds when the guarantee is removed) |
|---|---|---|---|
| **T1** | **Provenance schema** (R10): `LEAF_SOURCE` + KAT; `payer_tin` / `transcribed_on`; `answer_log` keyed by identity (`DependentGate { ssn_hash, gate }`) + `answer_log_history` + `record_answer` (one writer); the **`prompt_hash` mismatch rule** (re-ask, old record to history); `Declined`; `CarryProvenance::ComputedFromPriorReturn`; `SCHEMA_VERSION` 3 with the refuse / discard / parked-refuse split | once | `LEAF_SOURCE` both directions; identical records via `apply` and `income answer`; `Declined` vs absent; v2 row refuses, v2 WIP discards with note, v2 parked refuses; **change one fixture prompt's text → its answer reappears unanswered and the old record is in history, restore it → it does not**; **answer two dependent rows' gates, delete row 0 → row 1's records are unchanged and row 0's are gone** |
| **T2** | **Archive the information returns** — `f1099int`, `f1099div`, `f1099g`, `f1099b`, `f1098`, `f1098e`, `fw2` + their `iNNNN`: `MANIFEST.json` entries, `.pdf.txt` notes, `extract/*.txt` (`design/forms/README.md:14-40`); `Production::Collected` gains `from: DocBox` and `line-coverage` checks the box caption against the document's extract | per-revision (periodic) | a one-character caption change reds `line-coverage`; a `Collected` line with **neither a `DocBox` nor a `FilerRecords { instruction_line }`** reds (M6 — R5's lines have no box); **a box caption present in an archived extract with no `[boxes]` census entry reds, and an entry naming a caption the extract does not carry reds**; the archive round-trip hash matches the note |
| **T3** | **Document census + interview state** (R3, R12 core): `DocumentCensus` as `FORM_QUESTIONS`; the three census rules; the unsupported rows with §2.2's sentences; `covered_by` on every `unmodeled` entry of the seven forms; **the `[direction]` table derived from each form's part headings and asserted verbatim against `design/forms/extract/`**; the join KAT with the direction rule and the prompt-names-the-line reach check; **the residual attestation's prompt extended to name every line it is claimed to cover**; `interview_state()` with all seven states (blocking / forgoing / refusing / waiting / declined-forgoing / hash-mismatch / counted); `income answer` prints the panel | once (the `covered_by` and `[direction]` entries per-year) | R3's seven kills; the census join red on a planted bad variant, **red on `covered_by = "Advisory::EicOmitted"` on Schedule 1 line 2a while the same advisory on a Part II line stays green, red on `QuestionId::OtherOutOfScopeIncome` covering Schedule 1 line 8h while the prompt lacks *"jury duty"*, and red when a part heading is deleted from the direction table**; N unanswered ⇒ N listed; `Declined` forgoes and never blocks; `refusing` non-empty before `screen_inputs` fails; the extended no-brick test |
| **T4** | **Year gate + draft protection** (R11): entry states; confirm-before-discard of a non-trivial WIP draft; `income answer` into a draft-only year; **`income import` runs the param-free screens before it writes** (M9) | once | R11's seven kills, including: `income import` of a TOML with `documents.k1 = true` refuses at import on a params-less year and writes no committed row |
| **T4b** | **The year-N+1 opener** (R10 part 4 — the every-year machinery the strategy ranks above any single year, and r1 had a mechanism and a kill for it with no task): `open_next_year(conn, year) -> Draft` in `btctax-cli`, reading year N's committed row and seeding year N+1's draft with each payer (TIN + name), each dependent (identity fields) and each venue as **pre-named rows with every box blank and every `PerYear` gate `None`**, each identity presented as its own tri-state prompt; every `Durable` fact (a DOB) displayed and confirmed by a `SetField` that writes a **fresh** `AnswerRecord`; year N's **computed** capital-loss and charitable carryforwards written with `CarryProvenance::ComputedFromPriorReturn { year: N }`, read from year N's committed **return** (the carryforward-out chain), never from year N's inputs; surfaced as `btctax income open-next-year --from N` and as the TUI's entry action | once | R10.4's kill (a fixture year N with two payers yields exactly two identity prompts, both `None`, and the opened screen's boxes are all default), **plus**: no `Usd` leaf of the seeded draft is non-zero except the two carryforwards and each carries `ComputedFromPriorReturn { year: N }`; every seeded `PerYear` gate is `None`; every seeded `Durable` fact has **no** `AnswerRecord` until the filer confirms it (a carried confirmation would be a prior-year answer satisfying this year's provenance) |
| **T5** | **1099-INT / DIV / B / G / 1098-E sections** (R4): Fields with T2 captions; the per-document `[boxes]` census against T2's extracts, with box 10 collected, boxes 11–13 refusing, W-2 box 13 and 1099-DIV box 3 decided (I2); 1099-G box 2 + the **return-level** `itemized_prior_year`; the three **document-less income questions** and the Schedule B filer's-records rows (R3/I1); `student_loan_interest_paid` replaced; `EXEMPT_PREFIXES` shrinks; the five anchors re-attributed; W-2 arithmetic warnings + the all-zero-row warning | once (captions per-revision) | coverage KAT count and ratchet; `NotInForm` count falls by **five**; the warning fixtures; the box-2 / refund path refuses naming the worksheet; each paired question live exactly on its row's `Some(false)`, `Yes` on the wage and refund questions refusing, `Yes` on the interest question requiring a Schedule B row; the `[boxes]` census complete, a deleted entry reds, box 13 checked refuses, box 11 > 0 refuses |
| **T6** | **The exchange seam** (R9): Step 0 panel function + TUI pane; `digital_asset_activity` + the cross-check against the **existing** `digital_asset_activity(state, year)` predicate (`return_1040.rs:2468-2475`), refusing a contradicted `No` and **warning, never refusing**, an unwitnessed `Yes`; the standing-order warning; venue-vs-answer listing | once | the **five-row** DA table (a buy-only ledger answered `No` prints `No`; an empty-ledger `Yes` prints `Yes` with the off-ledger warning and no refusal); the standing-order fixture pair; the unnamed-venue fixture; the grep-KAT on registry prompts |
| **T7** | **Dependents gates** (R6 gates): `Dependent` fields including `younger_than_you_or_spouse` and the **required** `date_of_birth`; row (5)(a)'s help carrying `:1905-1913` verbatim; `DEPENDENT_GATES` registry with the I-4 per-row liveness emulation (M3, no seam change); `screen_inputs` rows × gates; `live_questions` + the Dependents section per row; the `filer_tin` question; the §152(d) figure in `FullReturnParams` (TY2024/25/26) and `gross_income_under_limit`'s params-gated liveness | once (the figure per-year) | every gate `None` ⇒ refuses; **`date_of_birth = None` refuses and the row is not printed**; a `Declined` taxpayer-DOB skippable still resolves Step 1; every REFUSE edge names its rule; classifier compiles only when classified; `Single`-no-dependents asks nothing; `gross_income_under_limit` waits, not blocks, on a params-less year |
| **T8** | **Row (7), HoH/QSS, the TY2025+ grid emitter and its MAP** (R6 computed, R7): the age / under-17 / row-(7) computation; `form1040_full.rs` fills rows (5)–(7) on TY2025+; **T8 maps the TY2025 `f1040` dependents grid itself** — rows (1)–(7) × four dependents plus the *more than four dependents* box (`f1040--2025.txt:38-52`, read through the label reader) into `crates/btctax-forms/forms/2025/f1040.map.toml`, which today is 17 lines of capital-gains cells only (`line7a`, `da_yes`, `da_no`), so r1's kill had no map to run on; the rest of the form stays in the `UNCENSUSED` register; `hoh_marital_basis` + the two HoH tests + `hoh_qualifying_child_name`, the five QSS gates, and (optionally, in the same task) the five *Married persons who live apart* conditions; the line-19 forgo sized in the panel | once + per-year map cells (TY2026 ports the cells after finals) | the flowchart truth table **including the born-in-year row landing on CTC**; TY2024 emitter byte-identical; TY2025 fixture rows filled — now runnable — with the `UNCENSUSED` register's `f1040` count falling by **exactly** the cells mapped; HoH kills incl. `MarriedLivedApart` refusing with the rule's name; QSS `None`/`No` kills; the forgo size present/absent |
| **T9** | **Real estate** (R8): `Form1098` section replacing `mortgage_interest_1098` — the section itself **top-level**, with the `form_1098` census row and **every line-8 consequence of a row** keyed on `schedule_a.is_some()` through `form_1098_deducted()` (I8; box 4 the one exception); 8b rows + 8c + the TY2024 map cells; 8e chain; the **aggregate, status-adjusted** ceiling warning + `other_borrower_paid_interest` (M5); **box 4 > 0 refusing to Schedule 1 line 8z** (I3); the 8396 gate; `HomeSale` gates | once (map cells per-year) | 8e sum; the sweep reconciles; the ceiling fixture set (one row under, two rows summing over, MFS at half); `other_borrower_paid_interest = Yes` refuses; **box 4 = $1 refuses and box 4 = 0 does not**; **a standard-deduction fixture with a $900k 1098 asks nothing and refuses nothing — with the shared-interest gate and the 8396 gate each at BLANK and at a truthful *yes*, and differentially: adding the rows changes neither the live-question set nor the refusal — while the same fixture with a Schedule A refuses on each in turn**; **the opener's own seed onto a standard-deduction year refuses nothing**; 8396 refuses; the home-sale table crossed over all 24 combinations; empty `recipient_tin` refuses; **more 8b recipients than the year's dotted lines prints "See attached" on BOTH years and the packet manifest names the statement** |
| **T10** | **Trailer** (§5.4): direct deposit 35b–d + map + emitter + `RefundByPaperCheck` conditional; phone; spouse IP PIN; foreign address | once (map cells per-year) | the routing validator; the advisory silent when a deposit is given; spouse-PIN asymmetry (`get` never returns digits); foreign block printed on a fixture |
| **T11** | **Oracle path** (R13): `GoldenInputs` gains the dependents block (`n24`, `nu18`, `n1820`, `n21`, `age_head`/`age_spouse`, `blind_head`/`blind_spouse`, the EIC count) and `gen_goldens.py` + the OTS template carry it (I12); `project_to_golden`; `income project`; `check_return.py`; `ORACLE_INVISIBLE` | once | the projection inverse over every golden **and over a household with two dependents**; the invisible list complete; on a one-CTC-child fixture `oracle_line19 > 0` and the excuse equals it exactly, any other size failing; deleting the dependents block reds the inverse |
| **T12** | **The panel in the TUI + docs** (R12 render): the pane, the commit modal's forgoing **and refusing** lists, the manifest block (with `(declined)` marks and undated document rows named); man pages (`docs/man/btctax-income-answer.1` and siblings via `make docs`); `LIMITATIONS.md` | once | snapshots (N blocking; forgo sizes); `make docs` clean; the manifest block present on a fixture packet |
| **T13** | *(iff S2 confirms a Schedule C)* Schedule C Part II as a transcription struct (lines 8–27, `i1040sc`) + the 1099-NEC / MISC / K screen; the `nec_misc_k_1099` census row stops refusing | once | line-coverage on every Part II line; the census row opens the section; the two `attribute.rs:262,265` anchors re-attributed |
| **T14** | *(iff S2 confirms a 1099-R or SSA-1099)* the two screens + the Simplified Method and Social Security Benefits worksheets — from `design/ty2025/SPEC_retirement_income.md` after its own one round | once | per that spec |
| **T15** | *(post-v1, first γ)* Schedule 8812 — line 19 / 28 computed from the T7/T8 answers; needs the TY2026 final | per-year form | the forgo disappears; both oracles reconcile line 19 |
| **T16** | **Form 8889 (HSA)** — *owner ruling 2026-09-07 (FR-76): the one form on the owner's filed TY2024 return that btctax refuses; transcribed as a task, scheduled after T6 and ahead of T7.* Archive `f8889`/`i8889` (TY2024, TY2025), `f1099sa`/`i1099sa` and `f5498sa` (the HSA information returns, per revision); Parts I–III line by line in the form's numbering with the instruction text as the doc comment; the documents as R4 screens (1099-SA boxes, 5498-SA boxes, W-2 box 12 code W read from the W-2 section); the reach — Schedule 1 line 13 (`f1040s1--2024.txt:62`), Schedule 1 line 8f *Income from Form 8889* (`:30`), Schedule 2 lines 17c/17d (`f1040s2--2024.txt:84,87`); the `hsa_activity` declaration's `Some(true)` stops refusing and opens the section; the two oracles gain the HSA deduction (Tax-Calculator `e03290`; OTS's HSA line) | once (map cells per-year) | line-coverage on every 8889 line; the `[boxes]` censuses for 1099-SA / 5498-SA complete; the golden corpus gains an HSA household reconciling on both oracles; a contribution over the year's limit refuses naming Part I's line; the owner's filed TY2024 Form 8889 as the third witness, compared locally |

**Thirteen unconditional tasks (T1–T12 plus T4b), two conditional on S2, one post-v1.** Every task lands with its kill red
first (B1), one seam review scoped to the task's `main..HEAD` interaction (B3), a fold, and one
re-verification (S6).

---

## 8. Consolidated kills

| guarantee | the test that reds when it is removed |
|---|---|
| no `Money` field outside a document or filer's-records struct; every `Usd` leaf has one source | `LEAF_SOURCE` completeness, both directions (T1) |
| every document box caption is the form's own text | `line-coverage` `DocBox` check (T2) |
| a line btctax cannot take is announced or refused, never silent | the `covered_by` join over every `unmodeled` census entry (T3) |
| **a line whose omission UNDERSTATES tax is never covered by an advisory, and a question covers only the lines its prompt NAMES** | the direction rule in that join: an `Advisory` on an `Understates` entry reds while the same advisory on an `Overstates` entry stays green; a `QuestionId` cover whose prompt lacks the line's caption keyword reds; a missing part heading makes its entries unplaceable and reds (T3) |
| the direction of a line is read off the form, not typed **per map** — the block's caption and line range are asserted verbatim against the extract, and the caption's SIGN is a reading pinned ONCE in code (`DIRECTION_OF_CAPTION`, each row citing the form's own total sentence as evidence); no map may state a direction (T3 fold, seam review I1) | the `[direction]` table's headings asserted verbatim against `design/forms/extract/`; the caption ↔ reading join asserted both ways; a `direction` key in any map fails to parse (T3) |
| a document box the return needs is recorded, never dropped | the per-document `[boxes]` census enumerated from T2's extract: an unentered caption reds, an entry for a caption the extract lacks reds (T2, T5) |
| income the instructions say to report without its document has a door | each supported income row's `Some(false)` makes its paired question live and blocking; `Yes` refuses or opens a filer's-records row (T3, T5) |
| a figure the tool holds in the understatement direction reaches a line or a refusal | 1098 box 4 = $1 refuses naming Schedule 1 line 8z; box 4 = 0 does not (T9) |
| a document type is asked, and "none" is an answer, not an absence | census `None` refuses; declared-with-no-rows refuses; none-with-rows refuses (T3) |
| an excluded family refuses with its exit | every unsupported row's `Some(true)` message contains the exit (T3) |
| every blocking and forgoing item is visible at once; class (B) is never flattened | N-in-one-call; `Declined` is listed *(declined)* in `forgoing` and never in `blocking`; a refusing answer is listed before commit; a params-gated prompt waits rather than blocks; the extended no-brick test (T3) |
| an answer given under earlier words never stands under later ones | change a fixture prompt's text → the answer returns unanswered with the changed-wording reason and the old record is history; restore it → it does not (T1) |
| a diligence record never describes the wrong person | `answer_log` keyed by `ssn_hash`: delete row 0 and row 1's records are unchanged, row 0's are gone (T1) |
| year N+1 opens with confirmations and never a carried amount | the seeded draft has no non-zero `Usd` but the two carryforwards, each carrying `ComputedFromPriorReturn`, and no `AnswerRecord` on a durable fact until it is confirmed (T4b) |
| a long-lived draft is not lost to a note | import over a non-trivial draft refuses without `--force` (T4) |
| the draft never poisons a year | `resolve.rs` never reads it (exists, re-pinned) (T4) |
| the form covers every in-scope leaf and the exempt set only shrinks | the coverage KAT + the ratchet (T5, every later task) |
| the form can answer what it refuses | `NotInForm` count falls by exactly the re-attributed number (T5) |
| the DA box is never a `No` the ledger contradicts, and a truthful `Yes` is never refused | the five-row DA table, including the buy-only ledger answered `No` (T6) |
| a venue without a standing order is warned before its 1099-DA row | the fixture pair (T6) |
| every §152 edge lands where the instruction says, including the ones the instruction states as exceptions | the truth table per edge, with the born-in-year row landing on CTC and `line-coverage` checking row (5)(a)'s help quotation (T7/T8) |
| no dependent row is printed on a chain that was never completed | `date_of_birth = None` refuses; no Step 1 predicate reads a declinable skippable (T7) |
| an unanswered dependent gate blocks | rows × gates `None` ⇒ refuse (T7) |
| row (7) is computed, never asked | no `FieldId` for a row-(7) box; the emitter reads the computation (T8) |
| TY2024 is untouched by the grid work | byte-identical emitter output (T8) |
| HoH and QSS are assertions, each asked as the instruction's own predicates | `Hoh` with `hoh_marital_basis = None` or either test `None` refuses, `MarriedLivedApart` refuses naming its rule; `Qss` with any of the five `None` refuses (T8) |
| a standard-deduction filer is never refused over a deduction they are not claiming | the 1098 census row and the three declarations are not live without a Schedule A (T9) |
| 8e = 8a + 8b + 8c; the ceiling is checked with a figure | the sum fixture; the warning pair (T9) |
| a home sale is blank only on the instruction's own branch | the 8-branch table (T9) |
| secrets never cross the seam | spouse-PIN asymmetry (T10) |
| a real return reaches both oracles; every figure projects or is listed | the projection inverse; `ORACLE_INVISIBLE` (T11) |
| the line-19 excuse is a comparison that can fail | the projection carries the dependents block, so a one-CTC-child fixture yields `oracle_line19 > 0` and the excuse equals it exactly (T11) |
| answered-ness is structural for every new field | the classifier compiles only when classified; the `_` KAT (every task) |
| no forbidden shape returns | the R15 grep-KATs (T3, T6) |
| every checker was seen red | each instrument's planted-defect test, in the `field_census.rs:329` style, committed before its first green |

---

## 9. Open questions for the owner (four)

1. **S2, sharpened to documents — §2.2's WHOLE table, one yes/no per row, plus six.** Q1 is not a
   sample of the excluded families; it is all of them, because each one is a *"cannot file with btctax
   this year"* and the only bad time to learn that is February 2027 on the extension (J-29). For 2026, do
   you hold or expect any of: **Form 1099-R** (IRA, pension, annuity) · **Form SSA-1099 / RRB-1099** ·
   **Form 1099-NEC / 1099-MISC / 1099-K** · **Schedule K-1** (any) · **rental real estate or royalties
   (Schedule E)** · **Form 1099-S or a sale of real property** · **Form 1099-OID** · **Form W-2G** ·
   **Form 1099-C** (canceled debt) · **Form 1095-A** (Marketplace — ★ **CORRECTED 2026-09-07 by the
   owner: *"I have no self employment income / Or business."*** The premise here was P0's
   self-employment claim, and it is WRONG for this filer: the filed TY2024 return carries no Schedule C
   and no Schedule SE. The *"Marketplace enrolment is likely"* inference that rested on it is withdrawn.
   The refusal itself stands unchanged — a 1095-A still refuses the **whole** return because Form 8962
   is not built and no task builds it — but it is no longer a *probable* row for this filer, and the
   corroborating signals point the other way: Form 8959 with no Schedule SE means wages, and Form 8889
   means an HSA, which means an HDHP, which is usually employer-sponsored. **Still ask it** — employer
   coverage is an inference, not an answer, and this is the one row that stops the entire return) · **Form 1098-T** (education) · **an HSA (a W-2 box 12 code W) or a
   deductible IRA contribution**? And from the supported side: a **Form 1098** (mortgage)? **Dependents**
   — how many, any full-time student aged 19–23, any born in 2026, and are you filing as **HoH** or
   **QSS**? A **1099-G** with a state refund, or a taxable state refund with no 1099-G — and did you
   itemize on the 2025 return? A **Schedule C** beyond the ledger's? — *Each row has exactly one
   consequence, decided now: 1098 and dependents change nothing (T7–T9 are built regardless, being the
   form's page 1 and Schedule A); "itemized 2025 + a refund" adds the State and Local Income Tax Refund
   Worksheet to T5; Schedule C or 1099-NEC/MISC/K ⇒ T13; 1099-R or SSA-1099 ⇒ T14; and **every other yes
   is either a named refusal you accept now — a preparer for 2026 — or a task you add now.** D-B's
   assumption (`OWNER_DECISIONS_2026-09-04.md:61-63`: *"every income TYPE on the owner's real return is a
   member of P0's set … and nothing else"*) was ruled by a delegate; this is where you say it against the
   full list.*
2. **Line 19 in v1.** — **ANSWERED 2026-09-07: accept the forgo; T15 stays post-v1.** *"My income is
   well above that limit. Defer it until later."* The §24(b) phaseout is **$400,000 MFJ / $200,000 all
   other statuses**, reducing $50 per $1,000 above it (`i1040s8--2024.txt:100-101`), and the $500 ODC
   phases out on the same schedule — so at the owner's income the child tax credit and the credit for
   other dependents are **both zero on the merits**, and the forgo costs nothing. ★ Note what this means
   on the page: `ctc_provably_zero` fires, so line 19 prints a sworn `0` rather than a blank — which is
   the CORRECT testimony here (the filer genuinely claims no credit), not a forgo. **FR-85 stays open
   regardless**: the stale $2,000 ceiling is a correctness defect at the margin (where
   `2 × 2,000 ≤ L11 < 2 × 2,200`), and its pin is now derived over the bundle so it reds when TY2025's
   package lands. It simply does not bite this filer's own return. *(original question follows)* —
   Accept the visible forgo of the child tax credit until Schedule 8812 is transcribed from the TY2026
   final (T15), or pull T15 into v1 after finals? — *With dependents, up to the statutory maximum per
   child is on the extension deadline; without, the question is moot.*
3. **Is the S1 rehearsal the interview's first live walk?** — **ANSWERED 2026-09-07: yes.** *"We will
   plan to use the interview to simulate a real tax return."* The simulated return is the lived journey;
   the first computable target is TY2024 (see `ROADMAP_STATUS.md` §0a), TY2025 on the S1 ruling, TY2026
   with the January 2027 package or the R6 slice. *(original question follows)*  — *Recommended yes: the TY2025 documents in
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
