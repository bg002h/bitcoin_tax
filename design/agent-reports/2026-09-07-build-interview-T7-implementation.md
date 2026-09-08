# T7 — the dependents gates (R6, the gates half). Implementation report.

Single implementer, shared main tree, branch `main`, dispatched at `8d2f6b83`. **Not committed, not
pushed.** No subagents. Nothing reverted with `git checkout`/`restore`/`stash` — every mutation was
reverted from a `cp` backup under the session scratchpad.

`CONTINUITY.md` and `design/ROADMAP_STATUS.md` are modified in the tree by the CONTROLLER, not by me
(`design/ROADMAP_STATUS.md` gained the *"T7 (the dependents gates) BUILDING"* row while I worked). So
is the untracked `design/agent-reports/2026-09-07-build-interview-T16-reverify.md`. **Left alone.**

---

## 1. The six numbered deliverables

### (1) The gate fields on `Dependent` — LANDED

`crates/btctax-core/src/tax/return_inputs.rs` — **20 new `Option<bool>` leaves**, each
`#[serde(default)]`, each carrying the instruction's own sentence and its line cite as its doc
comment (measured: `sed -n '/pub struct Dependent {/,/^}/p' … | grep -c 'Option<bool>'` = **20**).
That is R14's own count — *"20 `Option<bool>` leaves on `Dependent` in total (17 gates + these 3)"* —
and it matches `DependentGate::ALL.len() == 21` exactly once `DateOfBirth` is counted.

`date_of_birth` is now REQUIRED in R6's sense: it is `DependentGate::DateOfBirth` in the registry,
the walk demands it in Step 1's block, and `screen_inputs` refuses
`DependentGateUnanswered { row, gate: DateOfBirth }` **before any print path**. Observed live during
the build, from `export_irs_pdf`:

> `called Result::unwrap() on an Err value: Usage("the 2024 return is not computable [DependentGateUnanswered { row: 0, gate: QrRelationshipOrMemberOfHousehold }]: … — no forms were written")`

- **`LEAF_SOURCE` entries: NONE, and that is not an omission.** `LEAF_SOURCE` partitions `Usd` /
  `Option<Usd>` leaves only (`provenance.rs:90`); T7 adds no money leaf anywhere on `ReturnInputs`.
  Machine-checked by the existing both-directions KAT
  `every_money_leaf_has_exactly_one_source_and_every_source_prefix_is_live`, still green. The
  two-chain money instrument `packet::tests::every_money_leaf_household` is likewise untouched and
  green. **Deviation D1**, recorded below.
- **Classifier rows: all 20**, through a new `Census::dependent_gate` (see §2).
- **TOML round-trip: green**, and it is the existing emitter round-trip that proves it —
  `fullreturn_oracle::fullreturn_fixture_matches_its_emitter` re-emits
  `kitchen_sink_household()` to TOML and compares byte-for-byte against the committed fixture, which
  now carries all 20 gates + `filer_tin_issued_by_due_date`. The nine-dependent TOML is parsed by
  `nine_dependents_scenario`.

Also on `HouseholdHeader`: **`filer_tin_issued_by_due_date: Option<bool>`** (deliverable 4).

### (2) `DEPENDENT_GATES` in core — LANDED

New module `crates/btctax-core/src/tax/dependent_gates.rs` (~950 lines + ~600 of tests).

**`DEPENDENT_GATES: &[DependentGateQuestion]`** — 21 entries, **total over `DependentGate::ALL`**
(asserted both directions in `every_gate_has_exactly_one_entry_and_every_entry_names_a_real_gate`).
Each entry carries `gate`, `prompt`, `prompt_from_params`, `help`, `cite`, `kind`, `get`/`set`,
`get_date`/`set_date`, `clear`, `unanswered_detail`, `durability`, `claim_path`.

**Liveness is DERIVED FROM THE WALK, not a per-entry predicate.** `DependentGateQuestion::live(ri,
row)` is a method delegating to `walk_dependent(ri, row).demands(self.gate)`. There is therefore *no
second copy of the flowchart* to drift — which is what the standing rule *"no decision keys on a list
you typed beside derived data"* asks for. `walk_dependent` returns a `u32` bitset (21 gates fit
exactly), so a liveness query allocates nothing.

**Per-row liveness uses the I-4 emulation, not a widened seam** (R6's decision, §10's freeze): the
form-seam `Field.live` is `|_| true` and the ROW's liveness is expressed by `get` returning `None`
and `set` refusing `NoSuchRow` — the `registries.rs:44-50` pattern, one row deeper.

**Liveness opens a whole STEP at a time**, because the instruction states each step's conditions as a
block and only then asks its question — and because a gate-at-a-time liveness would make
`income answer`'s sweep as deep as the flowchart. That is asserted, not assumed:
`income_answer_asks_the_dependent_gates_and_the_sweep_settles` runs the command's own loop against
the module-level `MAX_SWEEPS`.

`Durability`: `DateOfBirth` is the one `Durable` gate; the other 20 are `PerYear`.

### (3) `screen_inputs` rows × gates, the `interview_state` walk, `live_questions`, `income answer` — LANDED

| surface | where | what |
|---|---|---|
| `screen_inputs` | `return_refuse.rs::screen_dependent_gates` | rows × gates inside the same `tier.unanswered_refuses` gate as the `FORM_QUESTIONS` loop, for the identical reason (`income import` is the only row-creating path). One walk per row, read for both liveness and the STOP. `DependentGateUnanswered { row, gate }` on a live blank; `DependentGateRefused { row, gate }` on a STOP; the R10.3 wording check keyed by `ssn_hash`. |
| `interview_state()` | `interview_state.rs` §3 | replaces the `debug_assert!` placeholder T3 left (D7). `blocking` / `refusing` / **`waiting`** / `answered` / `not_live` all populated. `waiting` gets its **first real occupant**. |
| `live_questions` | `cmd/answer.rs` | new `Ask::DependentGate { gate, row }`; grouped per ROW, after the three declaration groups and before the skippables. Liveness re-checked immediately before each ask (the M-3 rule), keyed by `AnswerKey::DependentGate { ssn_hash, gate }` through `record_answer`. |
| the input form | `spec/sections.rs` | `dep_gate_tristate!` × 20 in `DEPENDENT_FIELDS`, plus `decl_tristate!(52, DeclFilerTinIssuedByDueDate)`. |
| `attribute` | `input-form/attribute.rs` | both new refusals anchor on the gate's own field via a TOTAL `dependent_gate_field` match (no `_` arm). |

★ **`interview_state`'s two entry points now have a production caller.** `interview_state_with_params`
had none before T7; `income answer` now loads `BundledFullReturnTables` and passes the year's package,
so a params-quoting gate is *waiting* on a params-less year and *blocking* on a year that has arrived.

### (4) `filer_tin_issued_by_due_date` — LANDED

`QuestionId::FilerTinIssuedByDueDate` (index 52, appended at the END for the `decl_tristate!`
array-index reason), a `FormQuestion` with `live: |ri| !ri.header.dependents.is_empty()` and
`neutral: true`. Refusal `RefuseReason::FilerTinUnanswered`.

**Deviation D4:** the brief said *"prompt from `:1765-1790`"*. `:1765-1790` is the Step 4/5
citizen-and-married block. Step 5 question 1's own sentence is at **`i1040gi--2025.txt:1743-1747`**,
which is what the prompt transcribes (and the spec's R6 table cites `:1743-1750` for that row).

### (5) The §152(d) figure in `FullReturnParams` — LANDED

`FullReturnParams::qualifying_relative_gross_income_limit: Usd`. **Both sources cited and both
machine-verified against files in this repo:**

| year | figure | Rev. Proc. | in-repo line | flowchart | in-repo line |
|---|---|---|---|---|---|
| TY2024 | **$5,050** | Rev. Proc. 2023-34 **§3.24** | `legal/text/irs-guidance/RevProc_2023-34.txt:615` | *"Who had gross income of less than $5,050 in 2024"* | `design/forms/extract/i1040gi--2024.txt:1700` |
| TY2025 | **$5,200** | Rev. Proc. 2024-40 **§2.24** | `…/RevProc_2024-40.txt:576` | *"…less than $5,200 in 2025"* | `…/i1040gi--2025.txt:1690` |
| TY2026 | **$5,300** | Rev. Proc. 2025-32 **§4.23** | `…/RevProc_2025-32.txt:908-910` | (2026 booklet not issued) | — |

Section numbers resolved by locating each `SECTION n` heading above the subsection, not by memory
(2023-34 §3 = *2024 Adjusted Items*, 2024-40 §2 = *2025*, 2025-32 §4 = *2026 Adjusted Items*).
★ The adjacent TY2026 constants in `tax_tables.rs` cite `§2.14`/`§2.10` for figures that are in
SECTION 4 of the same document; that is **pre-existing** and I did not touch it, but the new cite is
accurate and the discrepancy is worth a follow-up.

Shipped in `ty2024_full_return()` and `ty2026_full_return()` (adapters) and in all five in-crate
`FullReturnParams` test fixtures. `full_return_for(2026)` stays `None`, so TY2026 is still the
params-less year the *waiting* fixture needs. TY2025 has no `FullReturnParams` constructor at all
today — unchanged by T7.

The gate's prompt QUOTES the figure and the year:
`"Did this person have gross income of less than $5200 in 2025? (Form 1040 instructions, Step 4: …)"`.

### (6) FR-70 — the opener seeds each prior dependent — LANDED

`open_next_year::seed` now maps `prior.header.dependents` into rows carrying **name, ssn,
relationship, date_of_birth** and **every one of the twenty gates `None`**, spelled out with **no
`..Default::default()` tail** so a gate added later must be decided there or fail to compile.

**Deviation D3:** the brief's settled fact said *"the `Dependent {..}` literal in
`open_next_year::seed` was written with no `..Default` tail so it fails to compile when you add
fields; that is the intended forcing function."* At `8d2f6b83` there was **no literal at all** —
`dependents: Vec::new()` (I-2 had removed them). So the forcing function did not exist; it exists
now, written that way deliberately.

The prompt line is unchanged (`identities_of` still names every prior dependent, still masks the
SSN). The T4b test that pinned *"not seeded"* is rewritten, not deleted:
`a_dependent_and_a_venue_are_prompted_and_never_seeded` →
`a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not`, with the reason recorded in its
doc comment. The venue half of I-3 is unchanged and still asserted.

---

## 2. The gate table as built

Registry order is the flowchart's. `cite` is `design/forms/extract/…`.

| # | gate | prompt cite | edge | refusal / rule named |
|---|---|---|---|---|
| 1 | `DateOfBirth` (`Date`) | `i1040gi--2025:1487-1499, :3945-3951` | Step 1's age test is computed from it | `DependentGateUnanswered{DateOfBirth}` — Step 1 has no *unknown* edge |
| 2 | `qc_relationship` | `:1463-1466` | **No ⇒ Step 4** | — |
| 3 | `younger_than_you_or_spouse` | `:1488-1489, :1491-1492` | part of the age test | — (blocks while blank) |
| 4 | `full_time_student` | `:1491-1494, :1933-1939`; `f1040--2025:48-49` | age limb 2; row (6) box | — |
| 5 | `permanently_and_totally_disabled` | `:1496-1498, :1958-1962`; `f1040--2025:48-50` | age limb 3; row (6) box | — |
| 6 | `provided_over_half_own_support` | `:1502` | **Yes ⇒ Step 4** | — |
| 7 | `filing_joint_return` | `:1506-1508` | opens gate 8 | — |
| 8 | `joint_return_only_to_claim_refund` | `:1506-1508` | joint **and not** refund-only ⇒ Step 4 | — |
| 9 | `lived_with_you_over_half_year` (row 5a) | `:1512-1516, :1905-1913`; `f1040--2025:45-46` | **No ⇒ Step 4**; the *Exception to time lived with you* is quoted VERBATIM in `help` | — |
| 10 | `lived_with_you_in_us` (row 5b) | `f1040--2025:47` | printed; live under a checked (5)(a) | — |
| 11 | `qualifying_child_of_another_person` | `:1519-1521, :1967` | **Yes ⇒ REFUSE** | *Qualifying child of more than one person* (`:1967`) + Pub. 501 |
| 12 | `citizen_national_resident_or_canada_mexico` | `:1540-1543` | **No ⇒ REFUSE** | *"You can't claim this person as a dependent."* + *Exception to citizen test* (`:1889-1896`), Pub. 519 |
| 13 | `married` | `:1548, :1945` | **Yes ⇒ REFUSE** | *Married person* (`:1945`) |
| 14 | `tin_issued_by_due_date` | `:1639-1643` | No ⇒ **no credit box** (forgo) | — |
| 15 | `citizen_national_or_resident_alien` | `:1594-1597` | No ⇒ **no credit box** | — |
| 16 | `ssns_valid_for_employment_issued_by_due_date` | `:1619-1622` | **Yes ⇒ CTC; No ⇒ Step 5 ⇒ ODC** | — |
| 17 | `qr_relationship_or_member_of_household` | `:1662-1679` | **No ⇒ REFUSE** | the Step 4 list, incl. *lived with you all year as a member of your household* |
| 18 | `qualifying_child_of_any_taxpayer` | `:1683-1686` | **Yes ⇒ REFUSE** | *"Who wasn't a qualifying child (see Step 1) of any taxpayer"* + Pub. 501 |
| 19 | `gross_income_under_limit` | `:1690-1691` | **No ⇒ REFUSE**; prompt quotes the year's figure ⇒ **waiting** without params | *Exception to gross income test* (`:1897`) |
| 20 | `you_provided_over_half_support` | `:1695-1696` | **No ⇒ REFUSE** | the three rules (`:1823`, `:1949`, `:1940`) |
| 21 | `divorced_separated_multiple_support_or_kidnapped_rule_applies` | `:1695-1697, :1823, :1940, :1949` | **Yes ⇒ REFUSE** | all three, each with its own line cite |
| — | `filer_tin_issued_by_due_date` (return-level) | `:1743-1747` | No ⇒ no ODC box | — |

Step 2 q3 / Step 4 q4 (*are you filing a joint return?*) is computed from `filing_status`; Step 2 q4 /
Step 4 q5 (*could you be claimed?*) reads the existing `DependentTaxpayer` declaration and yields
`DependentVerdict::WaitingOnQuestion(QuestionId::DependentTaxpayer)` while blank — that question has
its own registry refusal, which fires first.

**The verdict enum T8 reads** (`pub enum DependentVerdict`): `NoRow`, `Unanswered(DependentGate)`,
`WaitingOnQuestion(QuestionId)`, `Refused(DependentRefusal { gate, exit, rule })`, `ChildTaxCredit`,
`CreditForOtherDependents`, `NoCreditBox`.

**`claim_path: Option<bool>`** — a declared polarity column, the exact shape and rationale of
`FormQuestion::neutral` (*"declared per question rather than inferred, because polarity used to live
as a hard-coded `matches!` in `testonly.rs`"*). It is **checked, not asserted**:
`every_gate_at_its_claim_path_answer_reaches_the_ctc_edge` builds a row from that column alone and
requires `ChildTaxCredit`.

---

## 3. The truth-table KAT's rows

Two tests. Each starts from a known-good row and perturbs exactly ONE answer, so every assertion is
attributable to its own perturbation.

**`the_flowchart_truth_table`** (16 rows, from the CTC edge):

| row | expected |
|---|---|
| Step 1 relationship No | ⇒ Step 4 (`Unanswered(QrRelationship…)`) |
| age test fails (not younger) | ⇒ Step 4 |
| provided over half own support Yes | ⇒ Step 4 |
| joint return and NOT refund-only | ⇒ Step 4 |
| joint return ONLY to claim a refund | still `ChildTaxCredit` |
| did not live with you over half the year | ⇒ Step 4 |
| the Step 1 CAUTION | `Refused` naming *Qualifying child of more than one person* |
| Step 2 q1 citizenship No | `Refused`, *You can't claim this person as a dependent* |
| Step 2 q2 married Yes | `Refused` naming *Married person* |
| Step 2 q4 could-you-be-claimed Yes | `Refused`, *You can't claim any dependents* |
| Step 2 q3 filing a joint return (MFJ) | claimed ⇒ `ChildTaxCredit` |
| Step 3 q1 no TIN by the due date | `NoCreditBox` |
| Step 3 q2 narrower citizenship No | `NoCreditBox` |
| Step 3 q3 NOT under 17 | `CreditForOtherDependents` |
| Step 3 q4 SSNs not valid for employment | ⇒ Step 5 ⇒ `CreditForOtherDependents` |
| Step 5 q1 FILER has no TIN by the due date | `NoCreditBox` |

**`the_step_four_truth_table`** (a grandparent claimed under §152(d); 1 baseline + 7 refusal rows + 2
forgo rows): Step 4 relationship No; a qualifying child of any taxpayer; gross income over the limit;
support not provided; a divorced/multiple-support/kidnapped rule applies; Step 4 q3 married; Step 4
q5 could-you-be-claimed — each asserted on **the gate that stopped it AND a fragment of the rule the
refusal names**. Then Step 5 q2/q3 on the relative path ⇒ `NoCreditBox` (a forgo, never a refusal).
It also asserts the relative path never demands the CTC's own SSN question.

Plus the **two edges r1 dropped**:
- `a_child_born_in_november_reaches_the_child_tax_credit` — DOB 2026-11-01 with row (5)(a) *Yes*
  reaches `ChildTaxCredit`, **and the misroute is shown to be real**: the same child answering the
  bare question literally reaches `CreditForOtherDependents`.
- `a_declined_taxpayer_date_of_birth_still_resolves_step_one` — records
  `AnswerState::Declined` on `SkippableId::DobTaxpayer` through the real `record_answer`, and Step 1
  still resolves from the row's own gate.

---

## 4. FR-70 — identity vs. declaration, per `Dependent` field

| field | class | crosses? | why |
|---|---|---|---|
| `name` | identity | **yes** | who this is; the same class as a payer's name |
| `ssn` | identity | **yes** | who this is — and it is the `answer_log` key, so it MUST cross or the diligence log has no anchor |
| `relationship` | identity | **yes** | a fact about the person; printed in column (4) as written |
| `date_of_birth` | identity | **yes** | the one `Durability::Durable` gate: a birth date cannot change, and it is keyed to the SAME person by the SAME SSN. `income answer` still puts it to the filer as a live class-(A) question with the value shown |
| the 20 §152 gates | **declaration** | **no — all `None`** | every one asserts about THIS tax year (*"did this person live with you for more than half of 2027?"*), every one is `Durability::PerYear` |
| `filer_tin_issued_by_due_date` | declaration | **no** | a header tri-state; the seed already drops every header tri-state |
| `answer_log` | — | **no** | nothing in `seed` copies it, so no prior-year record can satisfy this year's provenance |

★ **The one call I want on the record.** I considered NOT seeding `date_of_birth`, on the C-1
precedent (*"`Durable` … never Enter-to-accept, never pre-filled"*, which removed the taxpayer's DOB
from the seed). I seeded it, per the brief and per the spec's own seed table. The C-1 harm was
specific to a **skippable**, where a bare Enter means *decline* and `skippable_state` was converting
that keystroke into a false `Given`; a class-(A) date gate has no decline, the value is shown in the
prompt, and the row's SSN identity guarantees it is the same person's immutable date. The residual is
a provenance nicety (a filer may Enter past a value they did not type), not a correctness risk.
**Flipping it is a one-line change** in `open_next_year::seed` plus one assertion in
`a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not`.

---

## 5. Every kill, with its RED

Each mutation was applied, the suite run, the red quoted, then reverted from a `cp` backup.

**M1 — the screen loop removed** (`screen_dependent_gates`'s result discarded):
> `every_live_gate_that_is_blank_refuses_by_name`: *a blank live DateOfBirth must refuse with its own reason — left: None, right: Some(DependentGateUnanswered { row: 0, gate: DateOfBirth })*
> `every_refuse_edge_names_its_rule_in_the_screen_detail`: *the refusal must name "Qualifying child of more than one person" — got ""*

**M2 — one `claim_path` flipped** (`ProvidedOverHalfOwnSupport` `false`→`true`):
> `every_gate_at_its_claim_path_answer_reaches_the_ctc_edge`: *left: CreditForOtherDependents, right: ChildTaxCredit*

**M3 — the params-quoting prompt ungated** (`prompt_from_params: None`):
> `the_params_quoting_gate_waits_then_blocks_and_quotes_the_figure`: *on a params-less year the gate WAITS: []*
> `the_gross_income_limit_matches_the_shipped_params`: *the prompt quotes the year's figure: Did this person have gross income of less than the §152(d)(1)(B) limit for the tax year? …*

**M4 — the filer-TIN question made always-live**:
> `a_return_with_no_dependents_asks_nothing`: *Step 5 is reached only through a dependent, so the question is not live*

**M5 — one classifier row dropped** (`c.dependent_gate(married, G::Married)`):
> `the_classifiers_gate_rows_line_up_with_the_registry`: *every `Option<bool>` gate in the registry is classified, and nothing else is* — the diff names `Married` as the missing member.

**M6 — Step 1's routing broken** (`if false && is_qualifying_child`):
> `every_live_gate_that_is_blank_refuses_by_name`: *QualifyingChildOfAnotherPerson must be LIVE in its own scenario, or this row asserts nothing*
> `the_flowchart_truth_table`: *Step 1 relationship No ⇒ Step 4 — left: CreditForOtherDependents, right: Unanswered(QrRelationshipOrMemberOfHousehold)*
> `a_child_born_in_november_reaches_the_child_tax_credit`: *left: CreditForOtherDependents, right: ChildTaxCredit*
> plus `every_gate_at_its_claim_path…` and `every_refuse_edge_names_its_rule…`

**M7 — R12's `waiting` arm removed from `interview_state`**:
> `the_params_quoting_gate_waits_then_blocks_and_quotes_the_figure`: *on a params-less year the gate WAITS: []*

**M8 — the opener carries two gate ANSWERS across the year**:
> `a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not`: *every §152 gate must cross BLANK — a prior year's answer is not testimony for this one: [QcRelationship, LivedWithYouOverHalfYear]*

**M9 — the answer key changed from identity to row index** (`ssn_hash: format!("row{row}")`):
> `income_answer_asks_the_dependent_gates_and_the_sweep_settles`: *row 0's QcRelationship is live and was never asked — a gate nobody asks is a gate nobody can answer*

**M10 — the row-(5)(a) `help` mutated (three ways), inside the B1 pairing itself.**
`the_row_five_a_help_quotes_the_exception_and_a_mutated_help_reds` plants (a) a one-word paraphrase
(*"count as"* → *"may count as"*), (b) a TRUNCATION dropping the born-or-died sentence, (c) the empty
help — and asserts the rule `exception_is_quoted(cited_span, help)` reds on each and greens on the
shipped text. The comparand is **read from `i1040gi--2025.txt` lines 1905–1913 at test time**, never
pasted, and `the_row_five_a_cite_names_the_span_this_check_reads` pins the gate's own `cite` to those
same lines so the check cannot silently read a different paragraph.

**M11 — a paraphrase the new checker caught in MY OWN build.** The first version of
`JointReturnOnlyToClaimRefund`'s `help` paraphrased the instruction. `xtask prompt-check` red on the
first run:
> `gate clause 6 (JointReturnOnlyToClaimRefund Help) is NOT in the string the filer reads: "only to claim a refund of withheld income tax or estimated tax paid"`

Fixed by transcribing the sentence. **This is the instrument discriminating on a real defect, not a
planted one.**

**Kills that need no mutation because the compiler is the instrument:** adding 20 fields to
`Dependent` E0027'd `classify_dependent` and `scrub_dependent` and E0063'd every literal; adding
three `RefuseReason` variants E0004'd `input-form/attribute.rs`; adding a `QuestionId` E0004'd
`registries.rs::question_to_field`; adding an `Ask` variant E0004'd four sweep loops across three
crates. Every one of those was observed.

**Other kills landed, all green:**
`every_gate_has_exactly_one_entry_and_every_entry_names_a_real_gate` (registry ⇔ enum, both
directions; the Date gate is exactly the one with no `claim_path`);
`deleting_row_zero_leaves_row_ones_gate_records_intact` (T1's identity key re-asserted through the
REAL gates); `a_gate_answered_under_earlier_words_is_refused_as_unanswered` (R10.3 now reaches the
gates through `current_prompt`); `the_gross_income_gate_is_not_asked_without_the_years_package`;
`a_paraphrased_gate_prompt_is_rejected` (*"on or before the due date"* → *"by the due date"* — a
difference of a day).

---

## 6. Every deviation

| # | deviation | why |
|---|---|---|
| **D1** | No `LEAF_SOURCE` entry for anything T7 added | `LEAF_SOURCE` partitions **money** leaves; T7 adds none. Machine-checked by the existing both-directions KAT, still green. |
| **D2** | `gross_income_under_limit` is **not** in `PARAMS_GATED_PROMPTS` | That table is `&[(QuestionId, &str)]` and the gate is a `DependentGate`, not a `QuestionId` — it could not go there. The dependence is instead **derived** from the registry entry's own `prompt_from_params`, which is stronger than a second hand-keyed list. `PARAMS_GATED_PROMPTS` stays, still empty, with its planted-occupant test unchanged. |
| **D3** | `open_next_year::seed` had **no** `Dependent {..}` literal at HEAD | The brief's settled fact was stale (I-2 had left `dependents: Vec::new()`). The tail-less literal exists now and is the forcing function going forward. |
| **D4** | `filer_tin_issued_by_due_date`'s prompt cites `:1743-1747`, not `:1765-1790` | `:1765-1790` is the Step 4/5 citizen-and-married block; Step 5 q1's own sentence is at `:1743-1747`. |
| **D5** | **`scrub` now KEEPS a dependent's `date_of_birth`**, reversing §6's drop; the `scrub_axis` matrix row for it is removed | §6 dropped it on the reason *"nothing reads a dependent's DOB — btctax does not compute the CTC"*. T7 gave it three readers (the Step 1 age test, Step 3's under-17 question, and `DependentGateUnanswered{DateOfBirth}`), so §3.2's rule — a replaced field must preserve every property a SCREEN reads — flips the decision. Dropping it would make the scrubbed copy REFUSE where the filer's own return passes. The derived axis then rejected the now-stale matrix row, which is the mechanism working in the **opposite** direction from its first occasion; both directions are recorded in the comment. |
| **D6** | The **age test and the under-17 test are computed in T7**, not T8 | Gate liveness (Step 2 vs Step 4) and the truth table cannot exist without them; §7 lists them under T8. T8 keeps the emitter, the TY2025 map, and all printing — nothing about what any year emits changed. |
| **D7** | `answer_all_dependent_gates` **asserts** on `tax_year == 0` rather than deriving a DOB from it | §G-15 defines `0` as *not stated* and `return_inputs::set` stamps the STORE KEY over it, so a fixture in that state would get a date ten years before year zero and the age test — run against the stamped year — would place a 2034-year-old on the qualifying-**relative** branch. Silent, and it moves the verdict. **Nine fixtures across five files** gained an explicit `tax_year`. |
| **D8** | `MAX_SWEEPS` hoisted from a fn-local `const` to a module-level `pub const` | so the sweep-settles kill names the REAL bound instead of carrying a second copy of `8`. |
| **D9** | `income answer` now calls `interview_state_with_params` where the year has a package | The two entry points existed for exactly this split and had **no production caller** before T7. |
| **D10** | The row-(5)(a) verbatim check lives in **`xtask prompt-check`**, not in `line_coverage` | `line_coverage` is a **money-line** census (`c.line(amount: Usd, …)`) and row (5)(a) is a checkbox. `prompt_check` is the existing verbatim-prompt instrument, already paired with a B1 kill, and it lives in `xtask` — which is `publish = false` and may read the repo tree, the reason `line_coverage.rs` states for putting the text half there. |
| **D11** | `Census` gained `dependent_gates: Vec<DependentGate>` beside `declarations` | A dependent gate's key space is `(row, DependentGate)`, not `QuestionId`; `Census::declaration` cannot express it. |

**Synthetic identifiers:** every SSN I introduced is `000-00-1111` / `000-00-2222` / `000-00-3333` —
area **000** *and* group **00**, structurally impossible on two counts. `scripts/pii-scan-generic.sh`
runs clean. No EIN added.

---

## 7. Every pinned number moved

| pin | old → new | cause |
|---|---|---|
| `QuestionId::ALL.len()` / `FORM_QUESTIONS.len()` | 52 → **53** | `FilerTinIssuedByDueDate` |
| `decl_count` (Decl\* fields carrying a question) | 30 → **31** | same |
| `DECLARATIONS.fields.len()` | 31 → **32** | same (+ `foreign_country_names`) |
| `form_spec()` total `Field`s | 218 → **239** | 20 `DepGate*` + 1 `DeclFilerTin…` |
| `covered.len()` (distinct covered in-scope leaves) | 217 → **238** | same |
| `scrub_axis` derived matrix | `header.dependents[].date_of_birth` **row removed** | D5 — scrub no longer replaces it |
| `docs/examples/examples.md` | regenerated | `income show` prints the new `Dependent` leaves + `filer_tin_issued_by_due_date` |
| `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` | regenerated via the committed `--ignored emit_fullreturn_fixture` | the oracle vector gained the gate answers |
| `nine_dependents_amt_inputs.toml` | 9 rows gained a DOB + 13 gate answers each; header gained `filer_tin_issued_by_due_date = true` | given *the answers a real filer would give* (all nine at the CTC edge), so **no computed figure moved** — AMT $9,077 and total tax $492,035 are unchanged, as `the_nine_dependent_amt_return_files_a_complete_packet` asserts |
| `prompt-check` assertions | 34 → **54** | 16 gate clauses × 2 + the exception span + the cite pin |

---

## 8. Suite lines, per crate (final, `cargo nextest run --locked -p <crate> --no-fail-fast`)

```
btctax-core            1315 tests run: 1315 passed, 0 skipped     (was 1301 — +13 dependent_gates, +1 classifier)
btctax-cli              800 tests run:  800 passed, 1 skipped     (was 798 — +2 in cmd::answer)
btctax-forms            359 tests run:  359 passed, 4 skipped
btctax-adapters         103 tests run:  103 passed, 0 skipped
btctax-input-form        70 tests run:   70 passed, 0 skipped
btctax-store             45 tests run:   45 passed, 0 skipped
btctax-tui              160 tests run:  160 passed, 2 skipped
btctax-tui-edit         392 tests run:  392 passed, 2 skipped
btctax-oracle-harness     5 tests run:    5 passed, 1 skipped
btctax-update-prices      5 tests run:    5 passed, 1 skipped
xtask                   162 tests run:  162 passed, 1 skipped     (was 159 — +3 in prompt_check)
```

`cargo fmt --all` — clean.
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
— **clean** (one `type_complexity` fixed by naming a `type Case`).

Repo-level checks, all OK:
```
scripts/pii-scan-generic.sh   pii-scan: clean (HEAD).
xtask cite-check              OK — 51 quotations, all verbatim
xtask prompt-check            OK — 54 assertions, all verbatim
xtask line-coverage           OK: 373 money lines across 18 form(s), 31 exception(s) (ratchet 31)
xtask harness-check           OK — 2 hook(s) wired
xtask stop-list               76 registry prompts scanned; no forbidden shape
xtask box-census              OK: 268 printed boxes … every one decided
xtask census-join             290 unmodeled entries … every one placed
xtask archive-check           no primary source outside the 5 accounted-for tree(s)
xtask authority-manifest      OK — every entry resolves and every source is listed
```

---

## 9. What is NOT mine and stayed untouched

Row (7)'s printing, rows (5)/(6)'s printing, the TY2025 `f1040` dependents-grid map and emitter,
HoH/QSS (T8); Schedule 8812 and line 19 (T15 — `ctc_odc_line19` is byte-identical, and line 19 stays
a visible forgo). **`form1040_full.rs` and `dependents_statement.rs` are unmodified.** TY2024's
emitted output is unchanged — `btctax-forms` is green with no golden regenerated.

## 10. Follow-ups worth filing (not filed — `FOLLOWUPS.md` untouched)

1. **A blank-SSN dependent row has no distinct identity.** `dependent_ssn_hash("")` is a valid hash,
   so two rows with empty SSNs share one `answer_log` key space. Pre-existing (T1's design); T7 makes
   it reachable because the gates now write there. The screen's SSN gate is at the packet boundary,
   not at authoring.
2. **`tax_tables.rs`'s TY2026 doc comment cites Rev. Proc. 2025-32 `§2.14`/`§2.10`** for figures that
   are in **SECTION 4** (*2026 Adjusted Items*) of that document. Pre-existing; the new `§4.23` cite
   is accurate and now sits beside them.
3. **The form seam shows `gross_income_under_limit`'s figureless prompt on a params-less year.**
   `Field.live` has no package, so the TUI renders the fallback label while the R12 panel correctly
   says *waiting*. The fallback names the missing package, so it is honest, but it is a second
   wording of the same gate.
