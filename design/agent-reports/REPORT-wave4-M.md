# REPORT — wave 4, parcel M. FR-225: the false `computable`, the exit that could not exit, and the misclassification.

**Agent:** parcel M implementer. **Base:** `789bb786a`. **Gate:** 3777/3777 tests, clippy 0 warnings,
`cargo fmt --all --check` clean. **Nothing committed, nothing pushed.**

All three defects are fixed, each with a planted-defect kill that was watched red. **Eight plants run;
all eight red on their own assertion; the restore re-measured 1078/1078 green.** Two of the plants
found defects in *my own* fix before the gate did — recorded below, because they are the interesting
part.

---

## 1. Premises — one refinement, no refutation

Everything the brief asserted holds. One phrase is imprecise, and the imprecision understates the
defect rather than overstating it:

> `year_readiness.rs:254,259` computes `interview: complete` from **answered-ness** and then prints
> `return: computable` **from it**

`return: computable` was **not** printed from answered-ness. Line 259 printed it from
`self.return_computable`, which `package_only` sets to `YearReadiness::bundled(year).params` — *does
this BUILD bundle the year's `FullReturnParams`*. So the conflation was **package-capability vs. this
return's verdict**, not interview vs. return. That is worse, not better: the clause was independent of
*everything* about the return, so it would have said `computable` for a TY2024 return refused for any
of the 126 reasons, not only an unresolved acknowledgment.

Verified, unchanged: both `return_1040.rs:3434` (`None`) and `:3466` (`Some(false)`) raise
`CharitableCwaUnresolved`; `interview_state.rs`'s `Declined` arm lists it as forgoing and never
blocking; `form.rs:4742` / `:4767` is the precedent, and this fix extends its guarding to the source.

---

## 2. What was built

### Defect 1 — the false claim. `return: computable` is now the RETURN's verdict, by construction.

- **`btctax-core/src/tax/interview_state.rs`** — new `pub enum ReturnVerdict { NotRun, Computes,
  Refuses { reason, detail } }`, **one type for both readers** (the walk and `EntryStates`), with
  `computes()` as the single predicate that licenses the word. `NotRun` is the `Default` and is a
  statement *about the instrument*: a surface that has not computed the return may not report that it
  computes.
- New entry point `interview_state_with_verdict(ri, params, verdict)`; the existing two pass `NotRun`.
- **`btctax-cli/src/year_readiness.rs`** — `EntryStates::return_computable` becomes
  **`package_computes`** (renamed, because the old name *was* the defect) plus `verdict: ReturnVerdict`
  and `with_return_verdict()`. `lines()` prints `computable` from the verdict **and nothing else**.
- **`btctax-cli/src/cmd/answer.rs`** — new `return_verdict()` composes `report`'s own chain verbatim
  and in its order: `screen_inputs`, then `screen_compute_dependent`, then `assemble_absolute` +
  `screen_absolute`. Every missing input (no params, no table, no year record, unprojectable vault)
  yields `NotRun`, **never `Computes`** — fail closed on the claim, fail open on the interview.

**The agreement is asserted, not reasoned about**, as the brief required.
`year_readiness::tests::the_printed_claim_agrees_with_the_return_verdict` quantifies
`claims_computable() == verdict.computes() && package_computes` over **24 cells** (2 package states x
4 interview states x 3 verdicts), enumerated from `bundled_years()` rather than typed. And the
end-to-end kill asserts the agreement *between the two surfaces on one vault*: it parses the reason out
of `report`'s `NOT COMPUTABLE [...]` and requires `income answer` to name the same one, so a future
divergence reds whatever either surface decides to print.

### Defect 2 — the exit that could not exit. The command the refusal names now asks the question.

Rather than re-word a refusal in a file this parcel does not own, **`btctax income answer` was made to
do what it says**: when the return refuses **and the default scope would ask nothing**, the scope
escalates to `Every` and the filer is told why (*"not your flag: the 2024 return does NOT compute
[CharitableCwaUnresolved] and the answer that refuses is already on file..."*). Every printed copy of
that remedy — including the two in `return_1040.rs` — becomes true at once.

Two boundaries stated in the source rather than papered over:

1. **It asks every question, not the one at fault.** Attributing a computed refusal to one registry
   answer needs a `RefuseReason -> PanelItem` map. The total one is `btctax-input-form`'s `attribute()`
   — which sits in a crate *above* `btctax-cli` and is a **dev-only** dependency there
   (`btctax-cli/Cargo.toml:48`), so production code cannot reach it without a dependency-graph change
   outside this parcel. In the state the escalation fires in, the alternative is asking *nothing*.
2. **It is gated on the brick, not on the refusal** — see section 3.

### Defect 3 — the misclassification.

- The walk's **section 7** takes the computed refusal into `refusing` (deduped by reason, so a
  registry-modelled refusal keeps its panel ITEM), which makes `is_committable()` false and stops
  `Some(false)` being counted as plainly `answered` with nothing said.
- `InterviewState::forgo_label_is_contradicted()` — keyed to `Refuses` and deliberately **not** to
  `!refusing.is_empty()` nor to `!ran()`, so the packet manifest and commit modal (which render the
  same lines holding no verdict) are untouched. `forgoing_lines()` then qualifies the heading:
  *"`lawful to skip` assumes a return that FILES, and this one does NOT compute..."*.
- The module header, the `Declined` arm and the `:802` test's doc now all state the boundary: the
  registry is **right** to carry §170(f)(8) as class (B) (its `live` sees neither the ledger nor the
  computed §63(e) election, so a class-(A) declaration would refuse every standard-deduction filer).
  The missing half was never the class — it was that nothing on this path ever ran the screen that
  decides when the class-(B) label stops applying.

### The same class, one surface over — closed rather than left

`report`'s §4.4 interview block is printed **on the refusal branch** ("a return that will not compute is
exactly when a filer needs the panel") and was built by `interview_state_with_params`, which cannot see
`screen_absolute`. So the §170(f)(8) question sat under **FORGOING — lawful to skip** a few lines below
`NOT COMPUTABLE [CharitableCwaUnresolved]`, in one block, on one screen. `render_interview_block` now
takes the verdict, and `cmd/tax.rs` hands in the refusal it already holds (and `Computes` on the arm
where `screen_absolute` returned `None`). Plant **p8** reds the e2e kill on this.

---

## 3. Defects the plants found in my own fix

**(a) The escalation was too wide, and an existing test caught it.** The first version escalated on
*any* refusal. But `screen_inputs` at the commit tier refuses an unanswered or reworded class-(A)
declaration too — so **every ordinary incomplete return** would have been put through the whole
registry, and FR-109's `a_reworded_question_is_asked_again_without_the_flag` (*"the flag is what
changes the behaviour"*) red on it. The condition is now *the return refuses **and** the default scope
would ask nothing* — which is the brick, stated. An unanswered declaration is already `blocking`,
already asked, already printed with its prompt; there is no brick there to break.

**(b) My end-to-end test was a false PASS, and plant p4 is what said so.**
`assert!(screen.contains("REFUSING"))` was satisfied by my *own other fix's* text — the FORGOING
qualification line says *"(see REFUSING above)"*. With section 7 planted away the test stayed green. It
now asserts the REFUSING **heading** (`— an answer already given that stops the return`) plus the
refusal's own §170(f)(8)(A) sentence.

**(c) The first fixture did not reproduce the brick.** It left `charitable_cwa_obtained = None` with no
answer record — which is `AnswerStatus::NeverAsked`, and `needs_asking` asks a `NeverAsked` question
anyway, so the *unfixed* command would have reached it too. The test now **runs `income answer` once**
(all bare Enters, which is how a filer records `Declined`), then machine-checks the brick before
measuring: `live_questions(&ri).iter().all(|a| !needs_asking(&ri, a))`.

**(d) FR-63, committed on the very function whose doc cites it.** My first `NotRun` wording was two
lines, and `docs/examples-tui-walkthrough/j6/01` came back with the second **clipped at the pane's
118th column** (*"..., which run"*) and one section row pushed off the list. The golden caught it. It is
one line now, and `the_entry_lines_fit_the_fixed_pane` holds the budget for every arm. That test then
immediately found a second instance nobody had looked for: a payload-carrying `RefuseReason` renders
`{reason:?}` at **164 columns** — so the entry line now prints the **variant name** (taken from
`Debug`'s first token, not a hand-written 126-entry map), while the payload rides the `detail` that the
panel and `report` print in full. The two surfaces agree on the refusal's *identity*, which is what a
filer matches between them.

---

## 4. The brief's open question: *"remove that gift from the deduction"* has **no CLI verb**

Measured, not assumed. `IncomeCmd` (`cli.rs:470`) is `Import | Show | Project | Scrub | Clear |
Answer`; a grep for a write to `schedule_a.charitable` under `crates/btctax-cli/src/cmd/` finds only a
doc example (`tax.rs:1797`). So a filer told to remove a gift has exactly three paths, none of them a
verb for it:

| path | cost |
|---|---|
| `income import --file <toml>` with the gift deleted | replaces the **whole** return; the filer must still hold the TOML the spec told them to delete |
| `income clear` | deletes every full-return input for the year |
| the tax-inputs TUI, Schedule A charitable section | works, and is the only targeted path — `attribute.rs:266` already anchors two charitable refusals at `SectionId::ScheduleACharitable` |

**I did not give it one**, and the reason is scope rather than difficulty: a `SetCharitableGift` or
`RemoveCharitableGift` verb is a new input surface with its own refusals, its own answer-log provenance
and its own tests — a parcel, not a line. **The refusal's text is in `return_1040.rs`, which this parcel
does not own**, so I could not even re-point it at the TUI. Recommended: either add the verb, or amend
that arm's cure to name the TUI section (the path that exists) instead of an action with no command.

---

## 5. Out-of-ownership edits, reported rather than hidden

The brief gave me three files. Five more had to move, all mechanically forced or the same defect:

| file | why |
|---|---|
| `btctax-cli/tests/year_gate_t4.rs` | where the kills live (it already drives `answer_return_inputs`) |
| `btctax-tui-edit/src/draw_edit.rs` | one identifier: `return_computable` to `package_computes` |
| `btctax-tui-edit/src/main.rs` | `the_tax_inputs_entry_screen_states_both_year_gate_states` asserted the **false** string; updated, with the reason in its doc |
| `docs/examples-tui-walkthrough/j6/01` and `/02` | regenerated (`emit_btctax_tui_edit_walkthrough_goldens`); one line each, nothing displaced |
| `btctax-cli/src/render.rs`, `btctax-cli/src/cmd/tax.rs` | the same false label on `report`'s own panel — see section 2 |

Not touched: `return_refuse.rs` (parcel N), `return_1040.rs`, `questions.rs`.

---

## 6. The eight plants, each red on its own assertion

| # | planted defect | reds |
|---|---|---|
| p1 | `lines()` prints `computable` from `package_computes` (the shipped defect, verbatim) | `the_printed_claim_agrees_with_the_return_verdict`, `a_surface_with_no_verdict_...`, `a_params_less_year_...`, **the e2e kill** |
| p2 | `return_verdict` drops the `screen_absolute` leg | the e2e kill |
| p3 | the scope escalation removed | the e2e kill (the CWA prompt never appears) |
| p4 | section 7 of the walk deleted | `a_computed_refusal_is_listed_as_refusing_...`, the e2e kill |
| p5 | `ReturnVerdict::computes()` true for `NotRun` | `the_default_verdict_neither_claims_computable_...`, `the_printed_claim_agrees_...` |
| p6 | the `NotRun` line stops naming the command | `a_surface_with_no_verdict_states_the_boundary_and_names_the_command` |
| p7 | section 7's dedup guard dropped | `a_computed_refusal_the_walk_already_modelled_is_not_listed_twice` |
| p8 | `report`'s interview block handed `NotRun` | the e2e kill |

Each plant was applied over a byte-identical restore, with every `.rs` file touched after it (FR-90, so
no stale rlib can serve a phantom result), and the loop ended by re-measuring **1078/1078 green**.

**And the paired half is in the kill**, so none of this is a gate that always fires: answering **y** at
the §170(f)(8) prompt clears the refusal from the `after` panel, un-qualifies the FORGOING heading, and
`report` agrees — all asserted on the same vault. The `Some(false)` leg is covered too: an explicit,
recorded *no* refuses identically, which is the half that was not even listed before.

---

## 7. One decision, stated

**`income answer` still exits 0** on a refusing return. It is an authoring command that successfully
stored the filer's answers; the defect FR-225 names is the false *claim*, not the exit code, and making
a successful write exit non-zero would break `--re-answer` and every script around it. The refusal is
now printed three times on that run — the entry line, the before panel and the after panel — and
`report` and `export-irs-pdf` remain the gates that refuse.

---

## 8. Follow-up worth filing (not filed — `FOLLOWUPS.md` is the controller's)

**Attribution.** A computed refusal reaches the panel with `item: None`, so the filer's cursor cannot be
moved to the answer at fault and the escalation must ask everything. The total `RefuseReason -> Anchor`
map already exists (`btctax-input-form/src/attribute.rs`, `_`-free, 126 variants). Making it reachable
from `btctax-core`'s walk — or promoting `btctax-input-form` from a dev-dependency of `btctax-cli` —
would turn "ask all of them" into "ask the one", and would let the panel point at it. That is a
dependency-graph decision, which is why it is a follow-up and not a line in this parcel.
