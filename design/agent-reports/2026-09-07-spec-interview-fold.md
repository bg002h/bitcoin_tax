# FOLD — the interview spec's one review, folded into `design/SPEC_interview.md` (r1 → r2)

**Editor:** single editor, shared `main` tree, branch `main`, from HEAD `d42c70bd`. Prose only: one file
edited (`design/SPEC_interview.md`) plus this report. No code, no other design file, no commit, no
subagent.
**Input:** `design/agent-reports/2026-09-07-spec-interview-review.md` (Fable, 1C/14I/9M/4N) and its
ledger `…-VERIFICATION.md`. **Brief:** `design/agent-reports/BRIEF-fold-spec-interview.md`.
**Ruling in force:** S6 — this is the LAST prose step. No second review follows; every task's own seam
review is the gate. So each fold below is written to be *complete* (the failure scenario cannot recur
under the new text) and *buildable* (an implementer can act on the sentence without asking).

**Result: 28 of 28 folded.** Nothing deferred, nothing dropped. The spec went 869 → 1,310 lines
(+567/−127).

---

## Machine-checks run BEFORE and DURING the fold (not delegated to the build)

| # | claim | check | result |
|---|---|---|---|
| 1 | N4: `year_readiness.rs:28` is off by one | `sed -n '25,30p'` — `:25 pub table`, `:26` doc, **`:27 pub params: bool`** | HOLDS — cite corrected to `:27` |
| 2 | N4: `return_inputs.rs:233` is off by one | `grep -n "pub struct HouseholdHeader"` → **233** | **DOES NOT HOLD** — §5.4's cite was already right; left unchanged (deviation D1) |
| 3 | N4: `sentence()` at `:115` | `sed -n '113,117p' year_readiness.rs` → `pub fn sentence` at 115 | HOLDS — unchanged |
| 4 | I13: TY2025 `f1040` map has no dependents-grid cells | `cat forms/2025/f1040.map.toml` — 17 lines, `line7a` / `da_yes` / `da_no` only | HOLDS |
| 5 | N1: the anchor cluster is five, not six | `grep -n` in `attribute.rs`: `PrivateActivityBondAmt:219`, `UnrecapturedOrSpecialRateGain:223`, `InconsistentDividendSubset:227`, `ForeignTaxOverCeiling:230`, `Form1099BNeedsForm8949:253`; `IraDeductionClaimed:234` is a separate refusal | HOLDS — count set to five |
| 6 | C1: the seven extracts the direction table keys on exist | `ls design/forms/extract/` — `f1040--2025.txt`, `f1040s1/s2/s3--2025.txt`, `f1040sa/sb/sd--2025.txt` all present | HOLDS |
| 7 | C1: every direction key is present verbatim in its extract | `grep -c -i` per key: *Income* 14, *Additional Income* 3, *Adjustments to Income* 3, *Additional Taxes* 3, *Additional Credits and Payments* 1, *Itemized Deductions* 1 | HOLDS |
| 8 | C1: Schedule 2 / B / D part headings are separably captioned | `grep` — S2 prints `Part I Tax` / `Part II Other Taxes`; **Sch B's Part I/II/III headings run into their content with no separable caption**; Sch D's three print cleanly | PARTIAL — drove deviation D3 |
| 9 | the fold did not break a markdown table | pipe-count-per-block scan over all 1,310 lines, blockquote-aware | 0 anomalies |

---

## Per finding — the section edited, and the sentences now in the spec

### C1 (Critical) — the DIRECTION RULE. §3 R2 mechanism 2 + its kill; §7 T3; §8.

R2 mechanism 2's heading is now *"**Every unmodeled line names what tells the filer — and a line whose
omission UNDERSTATES tax may never be covered by an advisory.**"* A new sub-block follows:

> **The DIRECTION RULE — the join is not direction-blind.** *"Announced or refused"* is not the same
> guarantee in both directions: a **deduction or credit** left blank forgoes money lawfully, and a
> sentence saying btctax did not try is an honest cover for it; an **income or additional-tax** line
> left blank is an omission that understates the tax, and no sentence covers that. So each census entry
> carries a **direction**, and the direction is *derived, never typed*

The derivation, and where the table lives, are pinned to the extracts (the brief's requirement that the
table be derived from the form's part headings and cited into `design/forms/extract/`):

> A `[direction]` table per form maps that form's own **part headings** to `Understates` or
> `Overstates`, and a KAT asserts every key **verbatim against the form's extract** in
> `design/forms/extract/` before using it, so the table is a *reading of the form*, not a hand-list,
> and a re-parted revision reds rather than drifting. Where a form's parts are not separably captioned
> in the text layer, the key is the **form's own title** and the direction is whole-form.

followed by the eight-row key table (verified by check 7 above) and:

> An entry the table cannot place — a key the extract no longer carries, a line outside every mapped
> part — **reds**; it is never defaulted to `Overstates`.

The two join rules, unwatered:

> - **An `Understates` entry may be covered only by a `QuestionId` or a `RefuseReason`. An `Advisory`
>   cover on an `Understates` entry reds.** This is the rule the build would otherwise break by default:
>   there is one advisory and no question for most of the 57 `unmodeled` Schedule 1 entries […] and an
>   advisory is exactly what a builder reaches for.
> - **A `QuestionId` cover is checked for REACH, not existence.** The KAT asserts the covering question's
>   `prompt` text contains the entry's own line caption keyword — the leading quoted phrase of `reason`,
>   or an explicit `names = "…"` key on the entry when the caption and the prompt's wording differ. A
>   variant that exists is not a cover; a question the filer can answer *No* to without ever reading the
>   line's name is not a cover.

and the consequence for the residual attestation, spelled out so the build cannot dodge it:

> In particular the residual scope attestation (`questions.rs:546-563`) covers **exactly the lines its
> prompt enumerates** and its trailing *"or anything else it never asked about"* covers **nothing** […]
> Every line claimed for it must therefore be **named in the words the filer reads**, which is an edit to
> the prompt, not to a TOML key: today's prompt names 1099-R, SSA-1099, rent, farm, K-1, tips, gambling,
> alimony and *"a business"*, so household-employee wages (1b), Medicaid waiver payments (1d), jury duty
> pay (8h), prizes and awards (8i), hobby income (8j) and each remaining Schedule 1 Part I line it is
> asked to cover are added to it or are covered some other way.

R2's kill gained the review's two plants plus two more that make the instrument *discriminate* rather
than merely refuse (B1's actual test):

> **(b2)** plant `covered_by = "Advisory::EicOmitted"` on Schedule 1 line 2a — an `Understates` entry —
> → red, and the same advisory on a Schedule 1 Part II entry stays green (the instrument is watched
> *discriminating*, not merely refusing); **(b3)** plant `covered_by = "QuestionId::OtherOutOfScopeIncome"`
> on Schedule 1 line 8h while the attestation's prompt does not contain *"jury duty"* → red, then add
> the words to the prompt → green; **(b4)** delete one part heading from the `[direction]` table → every
> entry in that part is unplaceable and reds (it does not fall back to `Overstates`).

§8 gained four rows for it (understatement direction, derived direction, box census, income door) and T3
gained the direction table, the reach join, the attestation's prompt extension and all four plants.

### I1 (Important) — a door for document-less income. §3 R3 (+ its kill); R4's 1099-G row; §5.1; §5.2; T5.

R3 now carries a headed block:

> **A census `No` never closes income the instructions say to report WITHOUT the document.** […] So the
> four supported income rows `w2`, `int_1099`, `div_1099` and `g_1099` each carry a **paired class-(A)
> `FormQuestion`, live iff that row is `Some(false)`**, phrased from the instruction that says so

with the three prompts verbatim from `:2442-2444`, `i1040sb:56-58` / `i1040gi:2521` / `i1040sb:24, 77`,
and `:41897-41898`; `Yes` on the wage and refund questions refuses (line 1a; the State and Local Income
Tax Refund Worksheet), `Yes` on the interest question opens `schedule_b_filer_records`. And:

> And because a gate may not ride on a row that might not exist, **`itemized_prior_year` moves off the
> `Form1099G` row onto `ReturnInputs`** as a return-level `FormQuestion`, live iff the refund question is
> `Some(true)` **or** any 1099-G box 2 > 0. On a year btctax filed year N−1, a `screen_compute_dependent`
> rule cross-checks it against year N−1's committed `itemize_election`, the way R9 cross-checks the
> digital-assets box.

R3's kill grew to seven items, (f) and (g) covering liveness, the three `Yes` outcomes and the
no-1099-G case. §5.1 names the three questions, the moved `itemized_prior_year` and the
`ScheduleBRecord` struct; §5.2's `Form1099G` row says the gate is **not** a row field.

### I2 (Important) — per-document box censuses. §3 R4 (+ kill); §5.2; T2 and T5 kills.

R4 gained a headed paragraph:

> **Every supported document carries a `[boxes]` census beside its `Field`s, ENUMERATED from the archived
> extract — the table above is a reading list, never the authority.** […] a KAT reads the box captions out
> of T2's archived `fNNNN` extract and requires **every caption to carry exactly one entry** —
> `collected(FieldId)`, `refuse_if_nonzero(RefuseReason)` (1099-INT box 9 today), or `not_read(reason)`
> carrying the instruction's own pointer. A caption in the extract with no entry **reds**; an entry naming
> a caption the extract does not have **reds**.

with the four decisions the review named (box 10 collected; boxes 11–13 `refuse_if_nonzero`; W-2 box 13
`collected: Bool` refusing to Schedule C line 1; 1099-DIV box 3 `not_read`). §5.2 closes with:

> The field list above is what the **struct** carries; the authority for *which boxes exist* is each
> document's `[boxes]` census, enumerated from the T2 extract (R4) […] A caption in the extract with no
> entry reds, so this table cannot go stale silently.

### I3 (Important) — 1098 box 4 → Schedule 1 line 8z or a refusal. §3 R8 (+ kill); §5.2; T9.

R8's box-4 clause is replaced:

> **4** refund of overpaid interest → `> 0` **refuses** `MortgageInterestRefundNotComputed`, *"Form 1098
> box 4 is income on Schedule 1 line 8z to the extent the interest reduced your tax in an earlier year
> (Pub. 936, Refund of home mortgage interest); btctax does not compute it"* — the instruction is explicit
> that the refund is **not** netted against the deduction (*"don't reduce your deduction by the refund.
> Instead, see the instructions for Schedule 1 (Form 1040), line 8z"*, `:1067-1070`), so a figure the tool
> already holds may not reach a blank 8z with a census note: that is a typed number with no reader in the
> understatement direction and C1's direction rule reds it.

Kill: *"A fixture 1098 with box 4 = $1 refuses `MortgageInterestRefundNotComputed` with *Schedule 1 line
8z* in the message; the same fixture with box 4 = 0 does not."*

### I4 (Important) — the age test on a missing DOB. §3 R6 table + prose + kill; §5.3; §5.4/R14 counts; T7.

The age row's edge is now:

> `date_of_birth = None` ⇒ **`DependentGateUnanswered { row, gate: DateOfBirth }`**, blocking — Step 1 has
> no *unknown* edge, so the row is not printed until the test can be evaluated

and a new row carries the second half as its own gate (`younger_than_you_or_spouse`, *"a **per-row
gate**, not a computation over the taxpayer's DOB"*). The prose withdraws r1's edge by name:

> The r1 text (*"DOB absent ⇒ the test is unknown ⇒ no credit column, class-(B) forgo"*) is **withdrawn**:
> the age test sits inside Step 1 (`:1487-1499`), which ends *"Yes. Go to Step 2. No. Go to Step 4."*
> (`:1525-1529`) with no *unknown* edge […] and printing the person in the Dependents section — *"the
> dependents you claim"* (`:1470-1472`) — is testimony that §152 is satisfied on a chain that was never
> completed. For the same reason *"younger than you (or your spouse if filing jointly)"* is asked as its
> own per-row gate rather than computed from the taxpayer's DOB: that DOB is a class-(B) **skippable** the
> filer may lawfully `Decline` […] and no Step 1 predicate may depend on a declinable value.

### I5 (Important) — the *Exception to time lived with you*, restored verbatim. §3 R6 row (5)(a) + prose + kill; §5.3 fixture; §6 J-11/J-22; T7.

Row (5)(a) now reads *"whose `help` carries `:1905-1913` **verbatim**: the *Exception to time lived with
you* — temporary absences (school, vacation, business, medical care, military service), a child **born or
died in the year**, a child adopted or placed in the year"*, with the edge *"the filer answers the
instruction's condition **with its exception in front of them**, so a child born 2026-11-01 whose home was
the filer's for more than half the time the child was alive answers `Yes` and row (5)(a) prints."* The
prose quotes both instruction sentences verbatim and states the misroute:

> Dropping it is **not** conservative: a child born in November answers the bare gate *No*, reaches Step 4,
> passes the qualifying-relative tests — a newborn has no gross income and the filer provided all support —
> and prints **row (7) Credit for other dependents**, a checked box that is wrong on the form's own terms,
> an under-claim of the larger credit, and not even a visible forgo.

Only *Kidnapped child* and *Children of divorced or separated parents* stay REFUSE branches. §5.3 gains a
second fixture at the born-in-year edge that *"must land on CTC, not ODC."*

### I6 (Important) — the Digital-Assets predicate, both cells corrected. §3 R9 (+ kill); §6 J-23; §8; T6.

R9's cross-check now runs against the existing predicate, with the instruction's own scope stated:

> Not *"the ledger has events"*: the instruction (`i1040gi--2025.txt:1346-1362` and the bullets after it)
> checks *Yes* for **receipts as a reward, an award or a payment, and for dispositions** — **not** for
> buying digital assets with real currency and **not** for transfers between the filer's own wallets.

Cell 1 refuses naming the first qualifying event. Cell 2:

> `!digital_asset_activity(state, year)` ∧ `Yes` ⇒ **accept and WARN**, never refuse […] The ledger is
> **not complete by construction** — no self-custody wallet is importable at all (recon §E.1) — so a filer
> paid in BTC to their own wallet has no export to import, and refusing their truthful *Yes* leaves *No* as
> the only way through the gate: a false answer the tool coerced into sworn testimony.

and the buy-only year is named as a printing cell. The kill is now five rows, T6 and §8 follow.

### I7 (Important) — HoH's compound gate split; QSS given its five tests. §3 R7 (retitled, rewritten); §2.1; §4.1; §5.4; R1's kill; R14; §8; T8.

R7 is retitled *"Head of household AND Qualifying surviving spouse are assertions; choosing either asks
the instruction's own tests."* The marital test becomes the instruction's own states:

> `hoh_marital_basis: Option<HohMaritalBasis { NotMarried, LegallySeparatedByDecree, MarriedLivedApart,
> NraSpouseNoElection }>` […] `MarriedLivedApart` **refuses** naming *Married persons who live apart*
> (`:1247`) until that rule's five conditions are transcribed as their own five `Option<bool>` gates — T8
> may transcribe them in the same task, at which point the variant stops refusing and asks the five instead.

QSS gets five gates phrased from `:1288-1316`, with the window derived rather than hardcoded:

> (1) your spouse died in one of the **two years preceding the tax year** and you did not remarry before
> the end of the tax year (`:1295-1297` — the window is *derived from `tax_year`*, never a literal pair of
> years, because each revision shifts it)

The MFS/HOH page-1 checkbox is recorded as the EIC separated-spouses rule and explicitly *not* an R7 gate.
R1's kill widened to admit `Option<Enum>` **only** for this case, and R14 classifies it.

### I8 (Important) — the 1098 liveness keyed to Schedule A. §3 R8 (+ kill); §5.1; §6 J-24; §8; T9.

> **Liveness is keyed on the itemize election, not on the document's presence:** the `form_1098` census
> row and the 1098 section are live iff `schedule_a.is_some()`, and the `MortgageAllUsed` /
> `AmtQualifiedDwelling` / `MortgageWithinDebtLimit` declarations key on
> `schedule_a.is_some() && !form_1098.is_empty()`.

with the reason (today's `questions.rs:280` already requires a Schedule A; dropping that half refuses a
standard-deduction filer on a truthful `Some(false)`; Form 6251 line 3 reads *"If you deducted home
mortgage interest on Schedule A"*; and the orphan rows would red R2 mechanism 3's reverse join).

### I9 (Important) — a `prompt_hash` mismatch re-asks. §3 R10 part 3 (+ kill); R12 table; §5.6; T1; §8.

> **A `prompt_hash` mismatch RE-ASKS — the hash has exactly one reader, and this is it.** […]
> `interview_state` treats an `AnswerRecord` whose `prompt_hash` ≠ `hash(the current prompt)` as
> **unanswered** — blocking for class (A), forgoing for class (B) — with the reason *"the wording of this
> question changed since you answered"*, and `screen_inputs` refuses such a class-(A) record as UNANSWERED.
> **The old answer is kept as history and never as the current answer:** the superseded `AnswerRecord`
> moves to `answer_log_history: Vec<(AnswerKey, AnswerRecord)>`, append-only, which nothing reads as an
> answer, and the re-answer writes a fresh record.

### I10 (Important) — the diligence key is an identity. §3 R10 part 3; §5.6; T1; §8.

> **The dependent key is an IDENTITY, never a row index.** […] The key is `ssn_hash`, a **salted hash of
> the row's `ssn`** and never the digits (the secret rules); `remove` on the Dependents section deletes
> that identity's entries; a row whose `ssn` changes starts a fresh record and the old entries move to the
> history below. […] (The *refusal* `DependentGateUnanswered { row, gate }` keeps the row index: it points
> the filer at a position on screen, and is not stored.)

### I11 (Important) — the year-N+1 opener becomes a task. §7 (new T4b + intro); §8; §2.1.

T4b is a full row: `open_next_year(conn, year) -> Draft` in `btctax-cli`, seeding identities as tri-state
prompts with *"every box blank and every `PerYear` gate `None`"*, durable facts confirmed by a `SetField`
that writes a **fresh** `AnswerRecord`, carryforwards written `ComputedFromPriorReturn { year: N }` and
*"read from year N's committed **return** […] never from year N's inputs"*, surfaced as
`btctax income open-next-year --from N` and as the TUI's entry action. `once`. Its kill is R10.4's plus
three: no non-zero `Usd` but the two carryforwards; every seeded `PerYear` gate `None`; and *"every seeded
`Durable` fact has **no** `AnswerRecord` until the filer confirms it (a carried confirmation would be a
prior-year answer satisfying this year's provenance)."* §7's count line is now *"Thirteen unconditional
tasks (T1–T12 plus T4b)"*.

### I12 (Important) — the oracle projection carries dependents. §3 R13 (+ kill); T11; §8; §6 J-28.

> **`GoldenInputs` gains the dependents block both oracles take, or the line-19 excuse is a check that
> cannot fail.** […] So it gains: `n24` (children under 17 on the CTC edge), `nu18`, `n1820`, `n21` (age
> bands computed from each row's `date_of_birth` and `tax_year`), `age_head` / `age_spouse` (from the DOB
> skippables when `Given`, absent when `Declined`), `blind_head` / `blind_spouse`, and the
> EIC-qualifying-child count […] The line-19 excuse is then **the oracle's computed CTC**, and a diff of
> any other size fails.

Kill: the inverse over a two-dependent golden; `oracle_line19 > 0` on a one-CTC-child fixture with the
excuse equal to it exactly; *"deleting the dependents block from the projection reds the inverse rather
than passing silently."*

### I13 (Important) — T8 maps the grid it fills. §7 T8.

T8 now says it *"maps the TY2025 `f1040` dependents grid itself — rows (1)–(7) × four dependents plus the
*more than four dependents* box (`f1040--2025.txt:38-52`, read through the label reader) into
`crates/btctax-forms/forms/2025/f1040.map.toml`, which today is 17 lines of capital-gains cells only
(`line7a`, `da_yes`, `da_no`), so r1's kill had no map to run on"*, with the `UNCENSUSED` register's
`f1040` count asserted to fall by **exactly** the cells mapped. The once/per-year cell became *"once +
per-year map cells (TY2026 ports the cells after finals)"*.

### I14 (Important) — §9 Q1 becomes §2.2's whole table.

> Q1 is not a sample of the excluded families; it is all of them, because each one is a *"cannot file with
> btctax this year"* and the only bad time to learn that is February 2027 on the extension (J-29).

Twelve excluded families listed one by one (1095-A carrying the note that P0's self-employment income
makes Marketplace enrolment likely and that no task builds Form 8962), plus the six supported-side
questions with HoH/QSS and the no-1099-G refund case added, and the consequence rule:

> **every other yes is either a named refusal you accept now — a preparer for 2026 — or a task you add
> now.** D-B's assumption (`OWNER_DECISIONS_2026-09-04.md:61-63` […]) was ruled by a delegate; this is
> where you say it against the full list.

### M1 — the panel's *refusing* state. R12 table + render + kill; §4.4; T12; the panel's name.

New row: *"live, **answered, and the answer refuses** […] **refusing** — the row, its `RefuseReason` and its
exit sentence, **derived from the `FormQuestion`'s own refusal**, so the filer sees it while authoring
instead of meeting it at commit (J-32)."* The extended no-brick test asserts `refusing` is empty before
`screen_inputs` passes.

### M2 — a `Declined` skippable stays on the forgoing list.

> live, class (B), **`Declined`** → **forgoing, marked *(declined)***, with the size — declining is
> provenance (asked, refused); the benefit is still forgone, and dropping it from the list exactly when
> the forgo becomes final is backwards. Only `Given` removes an item

R12's kill was inverted accordingly (it previously asserted the defect), and §4.4's manifest prints the
marks.

### M3 — the per-row liveness seam, decided. R6; T7.

> **Per-row liveness uses the I-4 emulation that already exists, not a widened seam.** […] Widening the
> seam to `fn(&ReturnInputs, &RowAddr) -> bool` — 98 mechanical closure edits — is **decided against**: it
> breaks §10's freeze and buys no behaviour the emulation does not already give.

### M4 — the two refusal sentences. §2.2.

1099-S row retitled *"a sale of real property you cannot fully exclude"*, sentence now *"Form 8949 and
Schedule D are the exit for a vacant lot, an inherited house or a rental; if it was your main home, Form
8949 code H and the Pub. 523 worksheet."* The 1099-NEC/MISC/K row's *lines* column gains **Sch 1 L8z** and
its sentence names *"1099-MISC box 3 — prizes, awards, research-study pay — is Schedule 1 line 8z income"*.

### M5 — the aggregate §163(h)(3) ceiling and the co-borrower. R8 (+ kill); §5.2; T9.

> the §163(h)(3)(B) ceiling is now **checked with a figure**, over the **sum of box 2 across every 1098
> row** — the limit is on aggregate acquisition debt, so two 1098s at $500,000 each are over it while each
> row alone is silent — against $750,000 / $1,000,000 by whether box 3 precedes 2017-12-16, **halved for
> MFS** ($375,000 / $500,000) […] a per-row gate `other_borrower_paid_interest` […] refuses
> `SharedMortgageInterest` on `Some(true)`, because a co-borrower deducts *"only your share"*.

### M6 — T2's kill vs R5. R2 mechanism 1; T2 kill; R5 kill.

`from: DocBox { stem, box, extract_line } | FilerRecords { instruction_line }`, *"the second because R5's
lines (line 26, Schedule A 5b/5c, 8b/8c, the charitable rows) have no issuing third party and so no box"*.
T2's kill: *"a `Collected` line with **neither a `DocBox` nor a `FilerRecords { instruction_line }`** reds."*

### M7 — the params-less prompt. R6 Step 4 row + kill; R12 table + kill; T7.

The gate is *"**not live until the year's parameters exist**: on a params-less year the panel lists it as
*waiting for the TY2026 package* […] because a prompt that cannot state the figure asks the filer to
derive it."* R12 gained a **waiting** state and `InterviewState` a `waiting: Vec<Waiting>` field.

### M8 — the two provenance holes. R4 (+ kill); §4.4.

> a row whose **every income box is zero** is most likely a row the filer began and did not finish — a
> payer issues a 1099-INT at $10 or more — so it raises the same kind of **warning** […] and writes
> nothing; and a row that arrives by TOML with `transcribed_on = None` is not silently tidied away — the
> packet manifest prints it as *transcribed without a date* (§4.4), because an absent date is a fact about
> the evidence, not the absence of a fact.

### M9 — the two journey moments. R5 (+ kill); R11 (+ kill); §4.3; T4.

(a) Line 26's help carries *"Include any overpayment that you applied to your 2026 estimated tax from your
2025 return"* (`:4266-4268`), *"The owner's TY2025 return was filed outside this project, so if the help
does not say it, an applied overpayment is simply missing from line 26 and nothing in the tool can notice
(J-31)"*, checked by `line-coverage`. (b) A new R11 bullet:

> **`income import` runs the param-free screens BEFORE it writes.** […] R3's three census invariants, the
> unsupported-row refusals and `NegativeAmount` need neither a `TaxTable` nor `FullReturnParams`: they run
> **at import** and refuse there, with nothing written.

§4.3's wire rule now reads *"refuses at **import** where the rule is param-free (R11) and at commit
otherwise — never at parse."*

### N1 — five anchors, and the wrong pointer. R4 kill; T5 kill.

> the **five** `NotInForm` anchors in this cluster (…) become `Field`/`Section` anchors and a test asserts
> the `NotInForm` count fell by exactly **five**. `IraDeductionClaimed` (`attribute.rs:234`) is **not**
> among them: HSA and IRA contributions stay refused unchanged (§2.2), so its anchor stays `NotInForm`.

The dangling *"see R8"* is replaced with §2.2, which is where the decision actually lives. T5's *"falls by
six"* → *"falls by five"*.

### N2 — the gate counts. R14; §5.3.

R14 row now reads *"`Dependent` gates ×17 — §5.3's sixteen plus `lived_with_you_over_half_year`"*, and the
checkbox row carries *"**20 `Option<bool>` leaves on `Dependent` in total** (17 gates + these 3), plus
`date_of_birth`, which is now **required**"*. §5.3 states the same arithmetic.

### N3 — three registries, one walk. R12 + kill.

> walks **three** registries — `FORM_QUESTIONS` (which is where the census rows live: R3 makes each census
> row a `FORM_QUESTIONS` entry, so it is one walk, not two), `SKIPPABLE_QUESTIONS`, and `DEPENDENT_GATES`
> × rows — plus the declared-document/rows invariant (R3)

### N4 — the cites. R11.

`year_readiness.rs:28` → `:27` (verified). `:115` and `answer.rs:117-121,131-137` unchanged (both correct).
`return_inputs.rs:233` unchanged — see deviation D1.

---

## Deviations from the review's minimal change, with reasons

**D1 — N4's second cite does not hold; §5.4 was left alone.** The review lists
`return_inputs.rs:233 → :232`. `grep -n "pub struct HouseholdHeader" crates/btctax-core/src/tax/return_inputs.rs`
returns **233**, so §5.4's cite is already right and "correcting" it would introduce the error. The ledger
recorded N4 as *"accepted as read"*, not measured; this is the measurement. The other three N4 items were
verified and the one real off-by-one was fixed.

**D2 — C1's understating span written as the *Income* part (lines 1–9), not "lines 1–8".** The review said
1–8, the brief said 1–9. Since the direction is *derived from part headings*, the honest statement is the
part, and both readings sit inside the one heading `Income`. No entry changes direction either way.

**D3 — the direction table admits a whole-form key where the extract has no separable part caption.** The
review's wording assumed every part heading is a clean string. It is not: `grep` on `f1040sb--2025.txt`
shows Part I/II/III headings running into their content (`Part I   1  List name of payer. If any interest
is from a seller-financed mortgage and the…`), so a strict per-part key would make every Schedule B entry
unplaceable and red the build on day one. The table therefore allows a **form-title key with a whole-form
direction** for that case, and I verified each key I list is present in its TY2025 extract (check 7).
Schedule 2 is keyed on its two printed part headings (*Tax*, *Other Taxes*) rather than the form's title
*Additional Taxes*, for the same reason — that is what the extract prints. The failure mode C1 exists to
close is unaffected: an unplaceable entry still reds and is never defaulted to `Overstates`.

**D4 — the panel is renamed from "the four-state panel" to "the answer panel" at six sites.** M1 and M2 and
M7 take it to seven states. Leaving a name that contradicts its own table is exactly the "§X disagrees with
§Y" defect the next reviewer would file, and there is no next reviewer.

**D5 — I11's task is numbered T4b, not appended as T13.** The brief asked for a T-number; renumbering
T5–T15 would invalidate every cross-reference in §2.1, §7 and §8, and T4b's position (immediately after the
schema it reads) is the ordering §7's intro now states. `T13`/`T14`/`T15` keep their meanings.

**D6 — I9's history has a named home.** The review's minimal change only said "treated as unanswered"; the
brief added "the old answer is kept as history, never as the current answer". A guarantee needs a field, so
`answer_log_history: Vec<(AnswerKey, AnswerRecord)>` is declared in R10.3 and §5.6, append-only and never
read as an answer. Without it, "kept as history" had nowhere to live and would have been dropped at build
time.

**D7 — I1's paired questions are built in T5, I2's box census in T5 with T2's kill.** The review said "T2
and T5 each gain one kill" for I2 and did not assign I1. T3 lands the census rows (the liveness predicate)
and lands first; the paired questions and the Schedule B filer's-records rows are income-screen work, so
they sit with the other document screens in T5. T2 owns the extract, so T2's kill is the
*unentered-caption* red and T5's is the entries themselves.

**D8 — §6 gained twelve journey rows, not M9's two.** §6 is titled *"the journey as requirements"*, and ten
of the review's new moments (J-19…J-32) are now requirements of rules this fold changed. Leaving them out
while changing their rules would reproduce the "edited §X, forgot §Y" shape; adding them costs one table
and makes every folded finding traceable to the moment that found it.

**D9 — R1's kill widened to admit `Option<Enum>`.** I7 replaces a compound `Option<bool>` with
`HohMaritalBasis`, which R1's kill as written (*"every registry entry is `Option<bool>` / `Option<Date>`"*)
would have forbidden. The widening is narrow and states its own limit: *"the last **only** where the
instruction itself states named alternatives rather than a yes/no, which is R7's `HohMaritalBasis` and
nothing else in v1 (a single `Option<bool>` over three distinct legal predicates is the compound answer
this rule exists to forbid)."*

**No finding was folded partially, and none was declined.** No Minor conflicted with a mandatory fold.

---

## What the build must now prove — one kill per blocking fold

| fold | the kill an implementer writes | task |
|---|---|---|
| **C1** | `covered_by = "Advisory::EicOmitted"` on Schedule 1 line 2a **reds**; the same advisory on a Schedule 1 Part II entry stays **green** | T3 |
| **C1** | `covered_by = "QuestionId::OtherOutOfScopeIncome"` on Schedule 1 line 8h **reds** while the attestation's prompt lacks *"jury duty"*, and goes green when the words are added to the prompt | T3 |
| **C1** | deleting one part heading from the `[direction]` table makes every entry in that part unplaceable and **reds** — no fallback to `Overstates` | T3 |
| **C1** | every `[direction]` key resolves verbatim in `design/forms/extract/` (assert before use) | T3 |
| **I1** | each paired question is live **exactly** when its census row is `Some(false)`; `Yes` on the wage question refuses naming line 1a; `Yes` on the refund question refuses naming the State and Local Income Tax Refund Worksheet; `Yes` on the interest question with no `schedule_b_filer_records` row refuses, with one row passes; `itemized_prior_year` is live on a return with **no** 1099-G row | T5 |
| **I2** | a box caption in an archived extract with no `[boxes]` entry **reds**; an entry naming a caption the extract lacks **reds**; W-2 box 13 checked refuses `StatutoryEmployeeW2` naming Schedule C line 1; 1099-INT box 11 > 0 refuses | T2 / T5 |
| **I3** | a fixture 1098 with box 4 = $1 refuses with *Schedule 1 line 8z* in the message; box 4 = 0 does not | T9 |
| **I4** | a dependent row with `date_of_birth = None` refuses `DependentGateUnanswered { gate: DateOfBirth }` and prints nothing; a `Single` return whose taxpayer-DOB skippable is `Declined` still resolves Step 1 for a row with `younger_than_you_or_spouse = Some(true)` | T7 |
| **I5** | a fixture child with DOB `2026-11-01` and `lived_with_you_over_half_year = Some(true)` lands on the **CTC** edge, not ODC; `line-coverage` checks row (5)(a)'s `help` quotes `:1905-1913` verbatim | T7 / T8 |
| **I6** | five-row DA table: activity ∧ `No` refuses naming the first event; no-activity ∧ `Yes` prints `Yes` with the warning and **no refusal**; an `Acquire`-and-self-transfers-only ledger answered `No` prints `No` | T6 |
| **I7** | `Hoh` with `hoh_marital_basis = None` refuses; `MarriedLivedApart` refuses with *Married persons who live apart* in the message; `Qss` with any of the five `None` refuses `QssTestUnanswered`, any `Some(false)` refuses `QssTestNotMet`; `Single`/`Mfj` ask none | T8 |
| **I8** | a standard-deduction fixture holding a $900k 2019 1098 asks none of the three declarations, transcribes no 1098 row, and does not refuse; the same fixture with a `ScheduleAInputs` asks them and refuses on `MortgageWithinDebtLimit = Some(false)` | T9 |
| **I9** | changing one fixture prompt's text makes its answer reappear in `blocking` (class A) / `forgoing` (class B) with the changed-wording reason, with the superseded record in `answer_log_history`; restoring the text un-does it and adds no history entry | T1 |
| **I10** | answer gates on two dependent rows, delete row 0 → row 1's records unchanged, row 0's gone; change row 1's `ssn` → its records move to history | T1 |
| **I11** | R10.4's opener kill, **plus**: no `Usd` leaf of the seeded draft is non-zero except the two carryforwards and each carries `ComputedFromPriorReturn { year: N }`; every seeded `PerYear` gate is `None`; no seeded `Durable` fact has an `AnswerRecord` until the filer confirms it | T4b |
| **I12** | the projection inverse over a golden with two dependents; a one-CTC-child fixture yields `oracle_line19 > 0` with the excuse equal to it **exactly**, any other size failing; deleting the dependents block reds the inverse | T11 |
| **I13** | the TY2025 fixture's dependent rows fill from the now-mapped grid, and the `UNCENSUSED` register's `f1040` count falls by **exactly** the number of cells mapped | T8 |
| **I14** | *(owner decision, not a test)* Q1 answered against the full §2.2 list before T13/T14 scope is fixed | §9 |

Minors and Nits carry kills too, each already written into its rule: the `Declined`-stays-forgoing and
answered-refusing assertions and the params-gated *waiting* state (T3, R12's kill); the aggregate/MFS
ceiling fixture set and `other_borrower_paid_interest` (T9); the `FilerRecords { instruction_line }` red
(T2); the all-zero-row warning and the undated-row manifest line (T5, T12); line 26's help quotation (T5's
`line-coverage`); and `income import`'s param-free refusal on a `documents.k1 = true` TOML (T4).

---

**Status after this fold:** `design/SPEC_interview.md` is r2 and, per S6, closed to further prose review.
Thirteen unconditional tasks (T1–T12 plus T4b), two conditional on owner Q1's answer, one post-v1. Four
owner questions remain open in §9, Q1 now carrying §2.2's whole table.
