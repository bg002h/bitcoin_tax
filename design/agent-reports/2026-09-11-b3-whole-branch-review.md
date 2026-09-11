# B3 — the whole-branch review of the interview arc

**One opus reviewer, isolated worktree, range `121c8805..ea8941ec`.** Written 2026-09-11 as the reviewer's
final action, per `BRIEF-b3-whole-branch-review.md` §7.

Scope as dispatched: **INTERACTION across the twelve tasks**, not correctness-per-commit. Nothing below is
a finding a per-task review could have held, and that is stated for each one.

---

## Verdict

**1 Critical / 2 Important / 0 Minor / 1 Nit**

The Critical is a single root cause with five distinct consequences, one of which blocks **every** filer who
authors a return in the tax-inputs form and then tries to print it. It is not in the nine thin-ice commits
the brief pointed at; it is in the form-engine surface built early in the arc, and it is reachable only by
holding three tasks at once (the form engine, the draft store, and the year-scoped question registry). Both
folds in the last nine commits traversed clean.

---

## Findings

### C-1 (Critical) — the form-engine surface never stamps `ReturnInputs.tax_year`, so every year-scoped rule, every rendered prompt, the completeness instrument and the commit gate all run at **year 0**

**Root site:** `crates/btctax-input-form/src/apply.rs:139-146` — `apply` materializes the working return as
`ReturnInputs::default()`, whose `tax_year` is `0`
(`crates/btctax-core/src/tax/return_inputs.rs:2813`). **No production code in the entire editor chain ever
assigns `ri.tax_year`.** Machine-checked: a `#[cfg(test)]`-stripped scan of
`btctax-tui-edit/src/{main,draw_edit}.rs`, `btctax-tui-edit/src/edit/{form,persist}.rs`,
`btctax-input-form/src/apply.rs`, `btctax-input-form/src/spec/{sections,mod}.rs` and
`btctax-cli/src/input_form_store.rs` finds exactly two production *reads* (`edit/form.rs:461`,
`draw_edit.rs:3087`) and **zero writes**.

**The three storage boundaries do not agree, and only one of them stamps:**

| boundary | stamps `tax_year`? | site |
|---|---|---|
| committed `return_inputs` row, read | **yes** | `crates/btctax-cli/src/return_inputs.rs:69-84` (`row_to_inputs`) |
| committed `return_inputs` row, write | **yes**, inside `set` | `crates/btctax-cli/src/return_inputs.rs:132-152` |
| `return_inputs_draft` row, read | **no** | `crates/btctax-cli/src/input_form_store.rs:33-64` (`get_draft_row`) |
| `return_inputs_draft` row, write | **no** | `crates/btctax-cli/src/input_form_store.rs:66-84` / `140-145` (`set_draft_row` / `save_draft`) |
| a freshly materialized working return | **no** (never touched storage) | `apply.rs:139-146` |

**The gate is structurally blind.** `crates/btctax-cli/src/input_form_store.rs:666-673`:

```
if let Some(refusal) = screen_inputs(ri, table, params) {   // ri.tax_year == 0 here
    return Ok(CommitOutcome::Refused(refusal));
}
mutate_and_save(sess, |conn| {
    crate::return_inputs::set(conn, year, ri)?;             // the stamp happens HERE, BELOW the screen
```

**This is FR-103 verbatim, on the other writer.** `crates/btctax-cli/src/return_inputs.rs:104-118` says why
`stamp_year` was extracted as a named function at all:

> *"a caller sometimes needs the stamp to have happened BEFORE the write — and FR-103 is the case that
> proved it. `income import` screened the parsed row with `screen_param_free` and only then called `set`, so
> at screening time `tax_year` was still the TOML's `0` and **every year-scoped rule in the screen was
> silent on the one command that creates a row**."*

`income import` was fixed (`crates/btctax-cli/src/cmd/tax.rs:280-281` calls `stamp_year` then screens).
`input_form_store::commit` — the **other** command that creates a committed row — was not. The brief's
seam 4 named this exactly: *"that is one instance of a class, and the class was never swept."*

**The premise is asserted in four independent places and is false on this surface:**

- `crates/btctax-core/src/tax/return_inputs.rs:2104-2108` — *"the storage boundary stamps this from the row
  key on read and refuses a disagreement on write, so an in-memory value and the row it came from can never
  diverge."*
- `crates/btctax-core/src/tax/questions.rs:920-923` — *"`ReturnInputs::tax_year` is stamped from the storage
  row key on read, so this predicate reads a year that is true by construction. A year-0 (never stored,
  never stated) **fixture** is NOT ≥ 2025…"*
- `crates/btctax-core/src/tax/return_refuse.rs:2459-2461` — *"`tax_year == 0` … **a yearless
  `ReturnInputs` is a test convenience**, and refusing it would be an assertion about a year nobody named."*
- `crates/btctax-input-form/src/spec/coverage.rs:696-700` — the census exempts `tax_year` from needing a
  `Field` because *"it is set by the command (`--year`) and stamped from the storage row key"*. The
  exemption reasons about a different surface than the one the census governs.

A year-0 `ReturnInputs` is **not** only a test convenience. It is the normal state of the tax-inputs form's
working return, and of every draft derived from one — and `crates/btctax-cli/src/input_form_store.rs:348-352`
says the draft is the *primary* store: *"R11 makes the draft the **Sep–Dec store for TY2026**: a year with no
`FullReturnParams` cannot commit at all, so the interview lives in the draft for months."*

The tell is already in the tree: `crates/btctax-input-form/src/apply.rs:1625-1628`, the FR-97 fixture, sets
`w.as_mut().expect("materialized").tax_year = 2024;` by hand, with the comment *"the year is set directly
because the tax year is not a form `Field` at all — **the renderer carries it**
(`TaxInputsFormState::fresh`)."* The renderer does carry a `year` field
(`crates/btctax-tui-edit/src/edit/form.rs:177`) — and never puts it on the return.

#### Consequence 1 — a filer who authors ANY return in the tax-inputs form cannot print it (the blocking one)

`QuestionId::DigitalAssetActivity` is `live: |_ri| true` (`questions.rs`, verified) — every filer, every
year — and it is one of the five `RENDERED_PROMPTS` (`crates/btctax-core/src/tax/questions.rs:101-122`),
whose text interpolates `ri.tax_year`.

Concrete scenario, TY2024 (the only year that can commit — `crates/btctax-adapters/src/tax_tables.rs:100-104`
bundles full-return params for 2024 alone):

1. `btctax-tui-edit`, press `T`, year 2024, no committed row and no draft → `Loaded::Fresh` →
   `working = None` (`crates/btctax-tui-edit/src/main.rs:866-869`).
2. Filer chooses a filing status → `apply` materializes `ReturnInputs::default()`, `tax_year == 0`.
3. Filer answers the Digital Assets question. `apply` → `record_or_forget` → `prompt_for_record` →
   `current_prompt` → `digital_asset_prompt(ri)` → **"At any time during 0, did you…"** → that string is
   hashed into `answer_log` (`crates/btctax-input-form/src/apply.rs:99-120`).
4. Filer presses `s`, confirms. `commit` → `screen_inputs(ri, …)` with `tax_year == 0` → `answer_status`
   renders **"during 0"** → hash matches → `Given` → **passes**. `return_inputs::set` then stamps 2024.
   The modal reports `committed 2024 as Single`.
5. `btctax report --year 2024` (or `export-irs-pdf`) → `return_inputs::get` → `tax_year = 2024` →
   `screen_inputs` → `current_prompt` renders **"during 2024"** → hash mismatch → `WordingChanged` →
   `refuse(DigitalAssetActivityUnanswered, WORDING_CHANGED_DETAIL)`
   (`crates/btctax-core/src/tax/return_refuse.rs:2700-2708`).

The filer is told *"the wording of this question changed since you answered — so the answer on file was
given to a different question"* (`return_refuse.rs:993-997`). **No wording changed.** The year was stamped.
Nothing on the printed page is wrong, because every downstream screen re-runs on the stamped row — the
defect is that the **commit gate cannot fail** on this family of rules and then reports success, and that the
refusal a filer does meet states a false reason.

It is recoverable on TY2024 (re-opening the editor now loads `Loaded::Committed`, which *is* stamped, so
re-answering fixes it). On a draft-only year it is **not**: the draft is never stamped, so re-answering in
the editor re-records the year-0 hash and the trap re-arms on every future commit.

#### Consequence 2 — the completeness instrument reports a false `interview: complete`

`crates/btctax-tui-edit/src/edit/form.rs:436` → `EntryStates::with_interview(self.working.as_ref())` →
`crates/btctax-cli/src/year_readiness.rs:224-231` → `interview_state(ri)` on the **unstamped** working
return, rendered as `"interview: complete"` / `"interview: N question(s) still to answer"`
(`year_readiness.rs:254-259`). A question whose liveness is year-scoped is not live at year 0, so it is not
counted, so the screen says complete. `QuestionId::HasIncomeExclusion` is `live: |ri| ri.tax_year >= 2025`
(`questions.rs:924`) with `unanswered: RefuseReason::IncomeExclusionUnanswered` — a class-(A) declaration
whose own detail says an unasked exclusion *"UNDERSTATES modified AGI … and understates the tax"*. On a
TY2025+ draft the editor never asks it and says the interview is complete.

#### Consequence 3 — the §152 dependent walk runs at year 0, so the wrong branch of the flowchart is live

`crates/btctax-core/src/tax/dependent_gates.rs:314` (`age_test(d, ri.tax_year)`) and `:402`
(`under_17_at_year_end(w.d, w.ri.tax_year)`). Liveness of the gates is the **walk's**
(`return_refuse.rs`, `screen_dependent_gates`), so at year 0 every dependent passes the §152(c)(3) age test
by a negative age and is walked down the qualifying-**child** branch. The qualifying-**relative** gates
(gross-income limit, support) are therefore not live, not asked, not answered — and become live the instant
the row is stamped. Same fail-closed-downstream shape as consequence 1, same false-passing gate.

#### Consequence 4 — a warning designed to be shown *at the moment of testimony* is silent on every year

`crates/btctax-tui-edit/src/edit/form.rs:461`:
`FullReturnTables::full_return_for(&fr, ri.tax_year).map(|p| p.acquisition_debt_ceiling)`. At `tax_year == 0`
this is always `None`, so the §163(h)(3)(B) aggregate acquisition-debt warning never renders in the editor —
including on TY2024, where the package exists. The surrounding comment says the check *"waits exactly as the
box-3 wage-base check does"* on a params-**less** year; it in fact waits on every year. R8's design is that
the ceiling warning is displayed *beside* `MortgageWithinDebtLimit`, the declaration it exists to inform, so
the filer swears that declaration without it.

#### Consequence 5 — a 60-year-old is asked the kiddie-tax questions

`crates/btctax-core/src/tax/questions.rs:3348-3351`, `:3385-3389`, `:3441-3448` gate three Form 8615
skippables on `!provably_24_or_older(ri, ri.tax_year)`. `provably_24_or_older`
(`crates/btctax-core/src/tax/return_1040.rs:1339-1344`) compares
`considered_age_at_year_end(dob, year) >= 24`; at `year == 0` a 1980 date of birth yields a negative age, so
the predicate is `false` for everyone and all three are live in the editor. The comment at `:3345-3347` says
the conjunct exists *"to keep the interview from asking a 60-year-old about full-time student status."*
Class-(B), so it does not block — but it is the predicate failing at exactly the job it was written for.

**Why no per-task review could have seen it.** The defect requires holding three tasks simultaneously: the
form engine's NI-2 materialization (which correctly refuses to invent anything, including a year), the draft
store's read boundary (which was written before `tax_year` existed and was never revisited when §G-15 added
it), and the year-scoped liveness/rendered-prompt registry (whose own doc comments assert the stamp as an
invariant). Each is locally correct and locally reviewed. Two independent instruments are structurally blind
to this leaf: the routing partition
(`crates/btctax-core/tests/oracle_projection.rs:605-612`) documents that it does **not** cover *"integers and
opaque codes (no alternative is derivable from the type)"*, and the form-coverage census exempts `tax_year`
deliberately (`coverage.rs:696-700`). This is B3's precedent exactly: the fix exists in the branch
(`stamp_year`, written for FR-103) and nobody carried it to the sibling writer, because no reviewer held both
writers at once.

---

### I-1 (Important) — the declaration registry draws the STATIC prompt and records a hash of the RENDERED one; the standard and its derived pin exist in the branch for the sibling registry and were never carried across

`crates/btctax-input-form/src/spec/registries.rs:38-44` — `decl_tristate!` sets
`label: FORM_QUESTIONS[$idx].prompt`, the **static** string. The TUI draws `f.label` verbatim
(`crates/btctax-tui-edit/src/draw_edit.rs:2946-2955`). But `apply` records
`prompt_for_record` → `current_prompt` → `FormQuestion::prompt_text(ri)`
(`crates/btctax-input-form/src/apply.rs:86-93`; `crates/btctax-core/src/tax/provenance.rs:1034-1045`;
`crates/btctax-core/src/tax/questions.rs:88-95`), which for a member of `RENDERED_PROMPTS` returns the
**rendered** string.

All five `RENDERED_PROMPTS` questions have `Field`s in the editor
(`registries.rs:288, 305, 309, 315` and `:727-752`), and all five statics differ materially from their
rendered forms for **every** year — verified by reading both:

| question | drawn (static) | recorded (rendered) |
|---|---|---|
| `DigitalAssetActivity` (`live: \|_ri\| true`) | "At any time during **this tax year**, did you…" | "At any time during **2024**, did you…" |
| `FilingStatusConfirmed` | "Is **the filing status carried from last year's return**…" | "Your **TY2024** return filed as **Head of household (HOH)**. Is **Head of household (HOH)**…" |
| `StateRefundWithout1099g` | "…state or local income taxes **this year**?" | "…state or local income taxes **in 2024**?" |
| `ItemizedPriorYear` | "…on your **PRIOR-YEAR** federal return…" | "…on your **2023** federal return…" |
| `QssSpouseDiedInWindowAndNotRemarried` | "…die in **one of the two years before this tax year**…" | "…die in **2022 or 2023**…" |

This is independent of C-1: with the year stamped correctly the hashes still match the screen's comparand, so
nothing refuses — the defect is that the diligence record names a sentence the filer was never shown. The
`FilingStatusConfirmed` row is the sharpest: the log claims the filer confirmed a **specifically named**
filing status on a **specifically named** prior-year return, neither of which appears in the sentence they
saw.

**The standard is written down in this branch, with a derived check, one registry over.**
`crates/btctax-input-form/src/apply.rs:1856-1896`,
`the_gate_fields_draw_the_words_they_hash_except_the_one_named_date_leaf`, walks `DependentGate::ALL` and
asserts:

> *"a yes/no gate that draws one sentence and hashes another is C-1 again: the filer's answer would read as
> given under words they never saw"*

…allowing exactly one exception, filed as **FR-100**, and reddening on a second. Nothing walks
`QuestionId::ALL` for the same property, and five declarations violate it today. That is FR-99's shape with
the set being *"registries whose `Field` label must equal the words it hashes"*: the pin was written when
there was one, and a second arrived beneath it.

**Why no per-task review could have seen it.** The gate pin was authored in the FR-97 fold; the rendered
prompts were authored in T5/T6/T8 across three earlier tasks. Neither review held both registries, and the
gate pin's own doc comment reasons carefully about `DepDob` and `GrossIncomeUnderLimit` — it simply never
looked sideways at `decl_tristate!`.

---

### I-2 (Important) — shipped filer-facing text added in this range tells the filer to run `report` to verify the Form 8949 box scope assumption; `report` prints nothing

`crates/btctax-cli/LIMITATIONS.md:425-427`, added at `52b348c2` (the FR-111 fold), inside the TY2024 Box C/F
paragraph:

> *"btctax will tell you when this is live: **`report`** prints an advisory naming how many of your
> dispositions occurred on an exchange and so *may* carry broker reporting."*

The advisory is `cmd::admin::broker_reporting_advisory`
(`crates/btctax-cli/src/cmd/admin.rs:715-735`). It has **exactly one** caller in the workspace:
`crates/btctax-cli/src/main.rs:1089-1093`, which is inside `Command::ExportIrsPdf`
(`main.rs:850`). `Command::Report` (`main.rs:123`) does not call it. `report`'s only broker-related block is
`render_broker_answers` (`crates/btctax-cli/src/render.rs:1949-1960`), which returns `None` unless
`regime.basis` is true or an answer is stored — and for TY2024 the regime reports neither proceeds nor basis,
so on the one year btctax files, `report` says nothing at all about exchange dispositions.

This matters because the document itself frames it as the verification step for a scope assumption it has just
asked the filer to check (*"Verify it fits you"*). A filer who follows the instruction sees nothing and
reasonably concludes the assumption holds. `LIMITATIONS.md` is `include_str!`'d into the binary
(`crates/btctax-cli/src/main.rs:582`, `btctax limitations`) and single-sourced into the man page, so this is
product surface, not a design doc.

The substance is right and only the command name is wrong — `broker_reporting_advisory(2024, Regime::NONE, 1)`
does return `Some(..)` (pinned at `admin.rs:2470`), and its text says what the document says it says.

**Why no per-task review could have seen it.** The sentence was written in the fold that existed to correct
false sentences in this very file, and its truth depends on a dispatch site three crates and one command away
from the paragraph. The FR-111 fold commit message lists the quotes it verified verbatim against source; this
claim is not a quote, so it fell outside that check.

---

## Refuted premises

**Nothing in the brief was refuted.** Everything I relied on, I measured:

| brief claim | measured | result |
|---|---|---|
| range `121c8805..HEAD`, 51 commits / 142 files / +32,236 | `git log --oneline \| wc -l` = **52**; `git diff --stat` = **143 files, +32,416/−982** | the stated delta is exactly `ea8941ec`, the brief file itself, as the dispatch prompt said. Verified, not refuted. |
| `crates/` 78 files / +17,657; `design/` 48 / +12,361 | `crates/` **78 / +17,657** ✔; `design/` **49 / +12,541** | `design/` differs by the brief file. Verified. |
| `RefuseReason` has **127** variants | brace-matched extraction of the enum body: **127** (127 distinct) | ✔ |
| `52558813` added `Field.label_source` with no `Default` and no `_` arm | `seam.rs:699-710` (`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`, no `Default`); `r15_stop_list.rs:461-465` is `_`-free with the reason stated | ✔ |

**One framing in the brief turned out to be only half right, and it is worth recording rather than scoring.**
§2 says *"Where the ice is thinnest — start here"*, pointing at the nine commits after `4fa1e723`. I started
there and traversed both folds (see Seams 5 and 1 below); both are clean, and the `label_source` blast radius
has exactly one reader. The Critical is in the **earlier, fully per-task-reviewed** region. That is not a
defect in the brief — it is B3's own precedent reproduced: *"Every round was thorough inside its window; the
defect lived outside every window."* Thin-ice-by-recency is a proxy for risk, and on this branch the better
proxy was **a fact that must hold across three tasks and is asserted in each of them.**

**Doubted but not measured** (labelled as such, per §7):

- The brief's five instrument counts (`line-coverage` 375/18/31/0/17, `census-join` 274, `r15` 8+4+6/91,
  `prompt-check` 90, `box-census` 268/19/9). I did not run `make gate` or any `xtask` instrument — the brief
  said I need not, and a worktree run is not comparable. I read what each instrument traverses instead.
- Whether `C-1`'s consequence 3 (the §152 walk at year 0) can be driven to a *wrong printed figure* rather
  than a downstream refusal. Every money path I traced takes the year as a function argument, not from
  `ri.tax_year` — `return_1040.rs:2436-2439` and `:2650` say so explicitly and record that *"a first draft of
  this wiring used `ri.tax_year` and a mutation caught it"* — so I believe the answer is no. I did not prove
  it exhaustively.

---

## Seams traversed

### Seam 1 — the chain from answer to printed page

Followed keystroke → record → screen → row → page, naming each hop.

**Answer → record.** `draw_edit.rs:2946-2955` (what is drawn: `f.label`) →
`registries.rs:38-44` (`decl_tristate!`, label = the static `prompt`) →
`apply.rs:154-165` (`apply_to`, `SetField`) → `apply.rs:99-120` (`record_or_forget`) →
`apply.rs:86-93` (`prompt_for_record`) → `provenance.rs:1034-1045` (`current_prompt`) →
`questions.rs:88-95` (`prompt_text`) → `questions.rs:101-122` (`RENDERED_PROMPTS`). **Found I-1 here:** the
drawn string and the hashed string are different functions of the same question, for five questions.

**Record → screen.** `provenance.rs:995-1022` (`answer_status` / `answer_status_against`) →
`return_refuse.rs:2649-2708` (the `SKIPPABLE_QUESTIONS` and `FORM_QUESTIONS` loops, the two readers of the
hash on the refusal path). Checked that `WordingChanged` really refuses rather than warns: it does, with
`WORDING_CHANGED_DETAIL` (`return_refuse.rs:991-997`), which names a working exit.

**Screen → row.** Enumerated every production screen call site and asked what `ri.tax_year` was at each:
`resolve.rs:96` (from `return_inputs::get`, stamped ✔), `cmd/tax.rs:281` (stamped one line above ✔),
`cmd/tax.rs:528/534` (from resolve ✔), `cmd/admin.rs:1772-1784` (from resolve ✔),
`interview_state.rs:549` (**unstamped** on the editor's per-frame call), and
`input_form_store.rs:666` (**unstamped** — C-1).

**Row → page (the FR-102 hop).** Read the fix (`4b4a6c26`) and then swept its class: the defect was one
shared column constant across a plan containing indented sub-lines. Derived the current state — every plan
now declares a column **per line** (`schedule23.rs:74-83` for Schedule 2, `:195-203` for Schedule 1,
`schedule_a.rs:95-115`) — and grepped every `fill_*_with_map` in `btctax-forms/src` for conditional
`continue`s that would leave a line's geometry unchecked. **`schedule23.rs:88-94` is the only such site in
the crate** (two gates: `part_i`, and `hsa` over indices 5..=7), both now fixture-driven. The class is closed
rather than the instance. Clean.

**Page → read-back.** `verify_flat` compares the map's declared column against the blank form's field
geometry, and the FR-102 commit records the deliberate decision not to derive the column from the field
(*"would make the check agree with itself and catch nothing"*). Checked and agreed; no finding.

### Seam 2 — multiple writers of one structure

**Derived the writer set rather than taking the brief's four.** `grep return_inputs::set|delete` over
non-test production code yields exactly five writers of the committed row and two of the draft:

| writer | site | stamps the year before screening? | records provenance? |
|---|---|---|---|
| `income import` | `cmd/tax.rs:296` (screen at `:281`) | **yes** — `stamp_year` at `:280` | no, by design (FR-109 ask-once) |
| `income answer`, committed | `cmd/answer.rs:1253` | n/a (loaded via `get`, stamped) | yes — `record_answer` |
| `income answer`, draft | `cmd/answer.rs:1247` (`save_draft`) | **no** — inherits the draft's year | yes, but hashed at the draft's year |
| `report --write-carryover` (year N+1) | `cmd/tax.rs:1295` | yes — `seed` sets `tax_year: to` (`open_next_year.rs:473`) | no, by design (`open_next_year.rs:19-20`) |
| the TUI editor's commit | `input_form_store.rs:670` (screen at `:666`) | **NO** — C-1 | via `apply` |
| the TUI editor's autosave | `input_form_store.rs:142` (`set_draft_row`) | **NO** — C-1 | via `apply` |
| `open_next_year` seed | `input_form_store.rs:264` (`save_draft`) | yes (`seed` sets it) | no, by design |

Checked the provenance side for disagreement (FR-97's shape): the two surfaces that record both go through
`record_answer` and resolve their words through the same registry — `apply.rs:86-93` and
`cmd/answer.rs:925/1194/1211` — and `answer_key_for` (`apply.rs:54-67`) derives *which* fields record from the
three existing registry maps rather than from a second list. That half is sound. The disagreement is the
**year**, not the record, and it is C-1.

### Seam 3 — refusals as a system

**Derived reachability rather than reading the variants.** Extracted all 127 `RefuseReason` names by
brace-matching the enum body, then searched every `#[cfg(test)]`-stripped production `.rs` in `crates/` for
each name's construction. Result: **126 of 127 are constructed in production. One is not —
`SingleEmployerExcessSs` — and it documents exactly why** (`return_refuse.rs:284-296`: *"NO LONGER RAISED …
Kept as a variant so the exhaustive cross-crate matches stay honest and any persisted value still maps"*), so
it is a stated boundary, not FR-87's shape.

**Exits.** `crates/btctax-input-form/src/attribute.rs:44` maps every reason to an in-form `Anchor`, in an
`_`-free exhaustive match (so a new variant is a compile error), with `Anchor::NotInForm { note }` for the
reasons whose cure is not a field — and `attribute.rs:850-870` pins the count of those by reading the
function's own source, with each increment named and justified. Sound.

**Contradiction.** The one contradictory pair I found is C-1's: `screen_inputs` at the commit gate says a
return is clean and the identical `screen_inputs` on the stamped row refuses it, with a detail that states a
false reason. That is the answer to seam 3's question, and it came from seam 4's mechanism.

### Seam 4 — the year boundary

This is where the Critical is. Derived the reader set: a `#[cfg(test)]`-stripped scan of every
`tax/*.rs` module for `\.tax_year` gives the complete list of behaviours that change when the year is wrong —
five rendered prompts (`questions.rs:126-227`), one liveness predicate (`:924`), three Form 8615 liveness
conjuncts (`:3350/3387/3443`), the Schedule 1-A wrong-year refusal (`return_refuse.rs:2462`), the §152 age
test and the CTC under-17 test (`dependent_gates.rs:314, 402`), and the params-quoting gate prompt
(`dependent_gates.rs:763`). Then traced each of the three storage boundaries and the fresh-materialization
path to see which of them can hand year 0 to those readers. Four of five can. `return_1040.rs:2436-2439` and
`:2650` are the two sites that deliberately do **not** read `ri.tax_year`, and both carry the reasoning —
which is what convinced me the money path is safe and the damage is confined to gating, asking, rendering and
warning.

### Seam 5 — instruments vs the surfaces they claim to cover

- **`r15 stop-list`, after the FR-114 widening.** Read `r15_stop_list.rs:405-470` in full. The label scan is
  derived from the typed registry (`form_spec()`), the split is an `_`-free match on `label_source`, and the
  two residues are stated in source: `Field.help` is not scanned, and `box-census`'s caption join is a
  **one-directional** containment (`box_census.rs:1391` builds `format!("{} {}", f.label, f.help)`, compared
  at `:1572`), so a ledger question appended to a `doc_*!` label is caught by neither. That is FR-99 option 3
  satisfied rather than a hidden gap. FR-117 (the part-authored label) is filed. No finding.
- **`Field.label_source`'s blast radius — the seam question the brief said the re-verification did not ask.**
  Asked it: `grep label_source|LabelSource` over `crates/` returns declarations in `seam.rs`,
  `spec/registries.rs`, `spec/sections.rs`, and exactly **one reader**, `xtask/src/r15_stop_list.rs`. The TUI
  (`draw_edit.rs`), the census join (`box_census.rs`), `prompt_check.rs` and `label_reader.rs` never touch it,
  so nothing is widened, broken or silently reclassified. Clean.
- **`verdict_reach`.** Read it (`xtask/src/verdict_reach.rs:1-60`). Its stated limit is honest and exact:
  reach is **textual**, it proves a name is mentioned on the filing path and not that the value is branched on.
  It also names why the `FILING_PATH` list must stay short. No finding.
- **The oracle routing partition (T11).** Read `oracle_projection.rs:595-705`. Derived (walks leaves, not a
  list), covers both fixture families, carries an anti-vacuity floor (`probed > 200`), and states its three
  blind spots — including *"integers and opaque codes"*, which is precisely why `tax_year` was invisible to
  it. Its own kill test stands beside it. No finding; the blind spot is disclosed, and C-1 lives in it.
- **`LIMITATIONS.md` as product surface (the brief's other thin-ice item).** Checked the FR-111 fold's three
  substantive claims against source rather than against the commit message: the year-aware box pair
  (`forms.rs:429-438`, `DIGITAL_ASSET_8949_FIRST_YEAR = 2025`) ✔; the 1099-DA routing
  `NotReported→I/L`, `ProceedsOnly→H/K`, `BasisMatches→G/J` with `Mixed`/`BasisDiffers`/unanswered as errors
  (`forms.rs:139-182`) ✔; "TY2024 only" (`btctax-adapters/src/tax_tables.rs:100-104` inserts 2024 alone) ✔.
  **Found I-2** on the fourth claim, the one that is an instruction rather than a quote.

---

## Out of scope / already filed

- **N-1 (Nit, pre-existing and OUTSIDE the range).** Two doc comments link
  `[crate::tax::advisories::Advisory::ExcessSsSingleEmployerNotCreditable]` —
  `crates/btctax-core/src/tax/return_1040.rs:2016` and
  `crates/btctax-core/src/tax/return_refuse.rs:289`. No such variant exists; the advisory is
  `Advisory::ExcessSsNotCreditable` (`crates/btctax-core/src/tax/advisories.rs:153`). Both are broken
  intra-doc links. `git diff 121c8805..HEAD` shows neither line as an addition, so this predates the range and
  is recorded only so it is not lost.
- **FR-110** (aggregate 1099-B is the permanent ceiling) — an owner decision, not a gap. I found no place
  where the product fails to disclose it; the new `LIMITATIONS.md` REFUSALS entry states it plainly, including
  the §1091 reasoning.
- **FR-117** (a part-authored label rides the caption's exemption) — filed, inert, and the residue is already
  stated in `r15_stop_list.rs`'s own doc comment. Re-verified as inert: the exemption's unit is the field
  while the thing exempted is a span of text, exactly as filed.
- **FR-100 / FR-101** (the dependent-gate label/hash divergence and the params-less gate prompt) — filed, and
  pinned by a derived test. I-1 is the **sibling registry**, which that pin does not reach and which no
  follow-up covers.
- **FR-95** (a money leaf reaching the oracle row at the wrong value) — filed; named as a stated residue of
  the routing partition.
- Per-task correctness T1–T12/T16, the kill replants, the two journey walks, the cross-task pattern sweep,
  the 8949 box truth table and the R15 kill-test: not re-audited, per brief §4.
- I did not run the suite, `make gate`, `cargo fmt`, or any `xtask` instrument. No suite red is reported,
  because I ran none — and per the brief a worktree result would not have been comparable anyway.
