# Seam review — interview build T4b (the year-N+1 opener)

Brief: `design/agent-reports/BRIEF-review-interview-T4b.md`. Build under review: `44ca7075`
(`git diff 68467dde..44ca7075`). Worktree checked out at `3445c50b` per dispatch. Read-only for the
record: every plant was applied from a `cp` backup and reverted; `git status --short` and
`git diff --stat` are both **empty** at the time of writing, and `binary(open_next_year_t4b)` is
18/18 green on the reverted tree.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo build --locked -p btctax-cli --tests
cargo nextest run --locked -p btctax-cli -E 'binary(open_next_year_t4b)'          # 18 passed
cargo nextest run --locked -p btctax-cli -E 'binary(tax_report) or binary(year_gate_t4) or binary(export_irs_pdf)'
                                                                                  # 120 passed
cargo nextest run --locked --no-fail-fast -p btctax-cli -p btctax-core \
    -E 'binary(open_next_year_t4b) or test(document_census) or test(census) or test(pre_named) or test(answer_row)'
```

Plants applied and reverted (all in `crates/btctax-cli/src/open_next_year.rs` unless noted):

| plant | reds |
|---|---|
| `payments: prior.payments.clone()` | `no_money_leaf_…` — *"payments.estimated_tax_payments = 1500 crossed the year boundary and is not a carryforward"* |
| `answer_log: prior.answer_log.clone()` | `a_durable_fact_is_shown_but_unanswered…` — *"the opener carries no record at all — {Skippable(DobTaxpayer): AnswerRecord { answered_on: 2025-03-01, … }}"* |
| `foreign_accounts: prior.foreign_accounts` | `every_per_year_gate_on_the_seed_is_unanswered` — *"ForeignAccounts carries an answer nobody gave this year"* |
| `restamp_from_prior_return` removed | `nothing_in_the_seed_is_stamped_merely_computed` (all four provenance leaves) **and** `no_money_leaf_…` (*"and it says which RETURN it came off"*) |
| 4 census `set` closures reverted to `ri.documents.X = Some(v)` (`questions.rs`) | `answering_no_to_the_census_removes_the_pre_named_rows` — `DocumentCensusContradicted { kind: Int1099 }` |

Four scratch probe binaries (`zz_review_probe{,2,3}.rs`) were written, run, and **deleted**; their
output is quoted verbatim in the evidence below.

## Summary

The copy-in design is sound and its kills are real — I independently reproduced five of the
seventeen, including the two that carry the design (no money leaf crosses; no `AnswerRecord`
crosses). Seam 2 holds: the carryforwards come off the frozen return through one definition, and
`write_back_carryover`'s own 120 tests are green untouched. Seam 4 holds: the write is `save_draft`,
`coherence_check` runs first and deletes nothing, and the three refusals fire in the documented
order. Seam 6 holds: year N's committed row is byte-identical.

What the build's kills do **not** ask is what happens to the seed **on the next surface the filer
touches**. Every finding below is of that shape. The Critical is that the opener pre-fills a
`Durable` date of birth and `income answer`'s documented *skip* keystroke then converts it into a
**this-year `AnswerRecord { state: Given }`** — a prior-year answer satisfying this year's
provenance, arriving one command after the module header declares that structurally impossible.

The three Importants are each a thing that crosses with **no surface that can answer it**:
`filing_status` and the household header (unnamed anywhere, and the classifier's exemption reason for
`filing_status` is invalidated by this new construction path), the dependent row (no census row, no
`FormQuestion`, no `interview_state` item, no refusal), and the venue key (which flips a documented
"the filer stored 1099-DA answers" predicate on with nothing answered).

## Findings

---

### C-1 (Critical) — a bare Enter on the CARRIED date of birth records it as **Given this year**

**Where.** `crates/btctax-cli/src/open_next_year.rs:254` (`carry_person`'s `date_of_birth: p.date_of_birth`)
meeting `crates/btctax-cli/src/cmd/answer.rs:146-157` (`skippable_state`) and `:382-403` (the
`SkippableKind::Date` prompt) and `:490-493` (the recording site).

**What is wrong.** `skippable_state` decides `Given` vs `Declined` by reading the **value**, not the
keystroke — and it documents that as the rule:

> *"a live skippable that ends the prompt holding a value was **`Given`**; one that ends it holding
> nothing was offered and passed over, which is **`Declined`**."* (`answer.rs:139-145`)

That rule is sound only while a value can reach `header.taxpayer.date_of_birth` **only** by the filer
putting it there this year. **T4b is the first path that breaks the premise.** The seed pre-fills the
DOB from year N, the prompt renders it, and the documented skip keystroke leaves it in place — so the
recording site writes a fresh record, dated by the `BTCTAX_NOW` seam, hashing today's prompt, saying
the filer **gave** the date for TY2025.

This is the exact prohibition in `questions.rs:31-33`:

> *"The prior MAY be displayed, but it still requires the same explicit keystroke as a fresh ask:
> **never Enter-to-accept, never pre-filled**."*

and it falsifies `open_next_year.rs:17-21`'s own claim:

> *"A `Durable` fact (a date of birth) is *displayed* on the seeded draft, and it carries no
> `AnswerRecord` **until the filer confirms it with a fresh `SetField`**."*

No `SetField` occurred. The record is worse than no record — `FIELD_PROVENANCE.md:440-441`,
*"a diligence record that lies is worse than none"* — because `answer_log_history`, the `prompt_hash`
re-ask rule and any future audit read it as an affirmative answer given on that date.

The build's kills #6 and #7 (`a_durable_fact_is_shown_but_unanswered_until_the_filer_confirms_it`)
prove only that the **seed** carries no record. Nothing asks what the next command does with it.

**Evidence.** Probe: open TY2025 from a TY2024 row whose taxpayer DOB is `1980-05-05`, then run
`cmd::answer::answer_return_inputs` with `y` to every declaration and a **bare Enter to every
skippable** (the documented skip). Observed:

```
PROBE5 seeded DOB = Some(1980-05-05), answer_log = {}
PROBE5 prompt as shown: … YOUR date of birth [1980-05-05; Enter to skip]: …
PROBE5 DOB after = Some(1980-05-05)
PROBE5 DobTaxpayer record after a bare ENTER =
    Some(AnswerRecord { answered_on: 2026-02-03,
                        prompt_hash: "33a53d9eab5fcb14aa615b073651e1c2cd06601c3527ff2604ce0f9d0a48b996",
                        state: Given })
```

`answered_on: 2026-02-03` is the injected `BTCTAX_NOW`, i.e. a TY2025-session record certifying
TY2024's value. Verified that the **TUI is not affected**: `btctax-input-form/src/apply.rs:76-87`
records only after a successful `SetField`, so this is the `income answer` surface specifically.
`DobSpouse` has the identical shape whenever a spouse crosses.

**Minimal change.** The seed must not put a `Durable` value where a value-reading predicate will
mistake it for this year's testimony. Either
(a) leave `date_of_birth: None` on the seed and carry the prior date into the **prompt** only (which
is literally what `Durability::Durable` says — *shown*, not pre-filled), so a skip records `Declined`
and `is_aged` correctly forgoes; or
(b) make `skippable_state` key off whether **this session** set the value rather than off
`get_date(ri).is_some()`.
(a) is the smaller change and keeps the one writer. Whichever is chosen, the kill is the probe above:
open, bare-Enter the DOB, assert the record is absent or `Declined`.

---

### I-1 (Important) — `filing_status` and the household header cross **unnamed**, and every surface asserts the opposite

**Where.** `open_next_year.rs:259` (`filing_status: prior.filing_status`), `:260-284` (name, SSN,
DOB, mailing address), against `open_next_year.rs:98-99` (`Opened::render`), `cli.rs:569-595` (the
`--help` text), `docs/man/btctax-income-open-next-year.1`, and
`btctax-tui-edit/src/draw_edit.rs:2886-2900` (the TUI offer).

**What is wrong.** Three separate things, one root.

1. **Nothing asks the filer to confirm the filing status this year.** Machine-checked: no
   `FormQuestion` and no `SkippableQuestion` prompt asks it (the only registry strings containing
   *"filing status"* are inside the Form 8615 parent-identity prompt, `questions.rs:2214-2223`), and
   `interview_state()` on the seeded draft carries no item for it. Marital status is determined **on
   the last day of the tax year** (§7703(a)(1)) — it is a per-year determination by statute, and the
   year boundary is exactly when it changes. Carrying **MFJ** across a divorce is the understatement
   direction.

2. **The classifier's exemption reason no longer holds.** `classifier.rs:146-151` exempts
   `filing_status` as `Class::SerdeRequired` with the stated ground:
   *"filing_status has no `#[serde(default)]` — a TOML without it refuses to parse, so no default to
   launder (§2.1)."* That premise is that **every** path onto a `ReturnInputs` forces a human to state
   it, because deserialization refuses without it. `seed()` is the first path that constructs a
   `ReturnInputs` **in Rust**, so the serde requirement never fires and the value comes from year N.
   The one mechanism holding filing status's answered-ness is bypassed, and the exemption's own
   sentence is now false.

3. **The report says the opposite.** `Opened::render` opens with *"Every box is blank and every
   question is unanswered"*, `--help` and the man page say *"every box blank, every question
   unanswered"* and *"Nothing about last year's answers comes with them"*, and the TUI offer says
   *"Every box arrives blank and every question unanswered: last year's answer is not testimony for
   this year."* None of the four names `filing_status`, the taxpayer/spouse name and SSN, the two
   DOBs, or the mailing address — all of which cross. A filer who reads any of those has no reason to
   look.

The **carry itself is defensible** and the builder's deviation-3 reasoning is right: the field has no
`None`, so not carrying it asserts **Single**, which is worse. The defect is that it crosses silently
under a positive assertion of blankness, with no confirmation anywhere.

**Evidence.** Probe with year N `filing_status = HoH`:

```
PROBE3 seeded filing_status = HoH
PROBE3 render =
Opened TY2025 from TY2024. Every box is blank and every question is unanswered — what follows is
what TY2024 knew about, for you to confirm:
  · Last year Acme Tooling (EIN 12-3456789) issued you a Form W-2. Did Acme Tooling issue one for 2025?
  · Last year First Bank (TIN 11-1111111) issued you a Form 1099-INT. Did First Bank issue one for 2025?
  …
PROBE3 a registry question asks about filing status: true   # ← the Form 8615 prompt text only
```

(the `true` is the substring match; reading `questions.rs:2214-2223` shows both hits are inside the
Form 8615 parent-identity prompt, not a question that asks the filer's own status). Also observed:
`the_filers_identity_crosses_and_the_per_year_header_facts_do_not:889` pins `seed.filing_status ==
Single` on a fixture whose year N is Single — the assertion cannot distinguish "carried" from
"defaulted", so no existing test sees this.

**Minimal change.** Name what crossed. `Opened` grows a `carried_identity: Vec<String>` (or the
`carried` list gains the header items) so `render`, the TUI offer, `--help` and the man page all say
*"TY2024's filing status (Head of Household), your name, SSN, date of birth and mailing address were
carried — confirm them; everything else is blank"*. Separately, either give filing status a
confirmation surface owned by this build's interview (a `FormQuestion` whose `live` is
`|_| true` would be the registry-shaped answer), or amend `classifier.rs:146-151`'s exemption text so
its stated ground matches reality and file the confirmation as owned by T5/T7 — not deferred past the
interview build.

---

### I-2 (Important) — the seeded **dependent** identity has no answer surface at all

**Where.** `open_next_year.rs:365-378` (`identities_of`'s dependent arm, `census_row: None`,
`answer: None`) and `:276-281` (the seeded `Dependent` literal).

**What is wrong.** `SPEC_interview.md` §7 row T4b requires *"each payer (TIN + name), each dependent
(identity fields) and each venue … **each identity presented as its own tri-state prompt**"*, and
R10.4 the same. For a payer the prompt has a real answer — the kind's census row, whose `None`
blocks and whose `No` now removes the pre-named row (`answer_row`). For a **dependent** there is
nothing: no census row, no `FormQuestion`, no `SkippableQuestion`, no `RefuseReason`, and — machine
-checked — **no `interview_state` item**. The prompt is a string printed once by `Opened::render`
and then gone.

The consequence is not merely a missing prompt. The seeded row **is** the claim: a `Dependent` row on
the return prints the person, their SSN and their relationship in the 1040 Dependents grid
(`dependents_statement.rs:78-87`) — sworn testimony — with nothing this year having asserted it. The
child who aged out, moved out, or is claimed by the other parent rides across the year silently.

The builder's own follow-up says *"the seeded row is simply pending"*. That is the claim I checked,
and it is not accurate — it is pending nowhere the tool can see. What is true is that T7's
`DEPENDENT_GATES` will close it (and the `Dependent { .. }` literal with no `..Default::default()`
tail is a real forcing function — deviation 6 is good work). But an item whose owning phase is a
*later* task is not the same as an identity that, today, is carried with no way to answer it, under
a spec row that requires one.

**Evidence.** Probe: year N claims `Sam Filer (daughter, DOB 2015-04-01)`; open TY2025; dump
`interview_state(&seed)`:

```
PROBE2 dependents on the seed =
    [Dependent { name: "Sam Filer", ssn: "987654321", relationship: "daughter",
                 date_of_birth: Some(2015-04-01) }]
PROBE2 interview_state = InterviewState {
    blocking: [ 24 × item: Question(…) ],   # census rows + header declarations — none is the dependent
    forgoing: [ 4 × item: Skippable(…) ],   # blindness, death, 8283 restrictions, CWA
    refusing: [], waiting: [], answered: 1, not_live: 25 }
grep -i "sam filer|dependentgate|987654321"  → no match anywhere in the state
```

Corroborated statically: `classifier.rs:428-430` `classify_dependent` — *"No classifiable leaves"*;
`return_refuse.rs` has no dependent-row refusal (`DependentStatusUnanswered` /
`DependentSpouseStatusUnanswered` are about the **taxpayer's own** dependency status).

**Minimal change.** Either (a) do not seed the dependent rows — carry them into the printed prompt
only, exactly as R10.4's *"shown"* allows, so an unconfirmed dependent is absent rather than claimed;
or (b) if they are seeded, give the class a blocking answer this build owns, so `None` refuses the
way an unanswered census row does. (a) is a two-line change to `seed` and matches the
"copy-in fails closed" direction the module is built on. Either way, the kill is: seed a dependent,
assert `interview_state` names it (or that the seed does not carry it).

---

### I-3 (Important) — the seeded **venue key** creates 1099-DA answered-ness the filer never gave

**Where.** `open_next_year.rs:331-338` — `broker_reporting: BrokerReporting(prior.broker_reporting.0
.keys().map(|p| (p.clone(), Default::default())).collect())`.

**What is wrong.** The field's own contract, `return_inputs.rs:1021-1024`:

> *"Absent = unanswered: **answered-ness lives in the key set, never in a sentinel value**."*

and `forms.rs:289-291`: *"An absent provider … is UNANSWERED (spec 1099-DA R1: answered-ness lives in
the key set)."* The seed inserts the provider **key** with an empty `CohortAnswers`, which is exactly
the sentinel the contract forbids. `open_next_year.rs:240`'s claim that *"answered-ness lives in the
slot"* contradicts both of the above.

Three call sites read presence in the key set as *"the filer stored answers"*:
`cmd/admin.rs:757-760` (`answers_stored` → `files_from_answers`, which selects the export's arm),
`resolve.rs:184-186`, and `cmd/tax.rs:282`. `admin.rs` resolves through `working_return`, so the
**draft shadows the committed row** and the seeded key is what it sees. The resulting sentence is the
precise defect `year_readiness.rs:333-336` records as *fixed* by R6 fold M-4 — *"the sentence
asserted 'from the stored answers' unconditionally, on years holding none"* — reintroduced by the
opener.

The seeded key buys nothing: `identities_of:379` builds the venue prompt from **`prior`**, not from
the seed, and `screen_broker_reporting` derives `keys_with_rows` from the ledger's 8949 rows. No
wrong tax figure results (the arm-2 screen still refuses an unanswered cohort with rows), but a
sentence stating a fact about the filer's own testimony is false.

**Evidence.** Probe: year N answers nothing but holds the key `"coinbase"`; open TY2025:

```
PROBE1 answers_stored (admin.rs:757 / resolve.rs:184 predicate) = true
PROBE1 slots actually answered = {"coinbase": CohortAnswers { covered: None, noncovered: None }}
PROBE1 sentence AS BUILT  = … The inputs are KEPT and will compute when the year's package is
    bundled. `export-irs-pdf --tax-year 2025` still prints the crypto slice from the stored answers.
    To fall back to a raw `tax-profile` …
PROBE1 sentence IF HONEST = … The inputs are KEPT and will compute when the year's package is
    bundled. To fall back to a raw `tax-profile` …
```

`dependents_and_venues_are_prompts_too_and_no_ssn_is_printed:860-868` pins the seeded key as
*intended*, so the existing suite locks the defect in.

**Minimal change.** Drop the `broker_reporting` arm from `seed` entirely — the prompt already comes
from `prior`. Kill: after opening from a year with a venue, assert
`draft.broker_reporting.0.is_empty()`.

---

### M-1 (Minor) — `--discard-draft` after the opener prints a destruction that did not happen

**Where.** `input_form_store.rs:565-570` (`coherence_clear`'s `ConfirmedDiscard` arm) reached from
`write_back_carryover`. Pre-existing code, but T4b makes the precondition routine: the opener's
normal output is a non-trivial year-N+1 draft, which permanently refuses `report --tax-year N
--write-carryover`, whose refusal prescribes `--discard-draft`.

**Evidence.** Probe: open TY2025, then run `write_back_carryover(FROM, force=false, …)`:

```
PROBE4 write-carryover (no --discard-draft) =>
    Err(NonTrivialDraftBlocksWrite { year: 2025, holdings: "1 Form W-2 row(s), 1 Form 1099-INT row(s)" })
PROBE4 draft survives the refusal: true
note: discarded the 2025 work-in-progress draft as requested; it held 1 Form W-2 row(s), 1 Form 1099-INT row(s).
PROBE4 write-carryover (--discard-draft) =>
    Err(Usage("year 2025 has no full-return inputs yet — the carryover is written onto that row, …"))
PROBE4 draft survives --discard-draft + failure: true
PROBE4 draft byte-identical after both: true
```

**Not a Critical:** the draft is byte-identical after both calls, because `coherence_clear` deletes
only in memory and `s.save()` is never reached. What is wrong is the note: the filer is told their
draft **was** discarded, and it was not. **Minimal change:** the opener's report should say that
year N's `--write-carryover` is now unnecessary (the carryforwards are already on the draft) and will
refuse; and/or `coherence_clear` should defer its note until the write has actually saved.

### M-2 (Minor) — `payer_of`'s `_ =>` wildcard

`open_next_year.rs:421` closes the `match row` with `_ => Default::default()`, so a kind that gains a
transcription section tomorrow yields a nameless prompt (*"Last year  issued you a Form 1099-R…"*)
and is silently not seeded. There **is** a compensating kill, but it lives in another crate:
`document_census.rs::every_kind_with_a_section_drops_its_pre_named_rows` panics with *"gained a
section — give it a pre-named seed here"*. Make the arm exhaustive so the compiler, not a test in
`btctax-core`, names the omission.

### M-3 (Minor) — the retention kill compares only year N's committed row

`nothing_about_year_n_changes:608-622` reads `return_inputs::get(conn, FROM)` before and after. The
brief's seam 6 asks for the committed row, the **draft table** and the answer log. The answer log is
inside the committed `ReturnInputs` and so is covered; year N's draft row and parked flag are not.
No live defect — the opener touches neither — but the kill does not hold what it is named for.

### M-4 (Minor) — the census-delegation kill only catches the first offending kind

Planting non-delegating `set` closures on **four** kinds at once
(`Int1099`, `Div1099`, `G1099`, `B1099`) reds exactly one test, on `Int1099` only, because
`screen_inputs` short-circuits at the first `DocumentCensusContradicted`. A future kind whose `set`
skips `answer_row` is caught only if it is the alphabetically-first offender in a fixture that
realizes it. `every_kind_with_a_section_drops_its_pre_named_rows` does not close this: it calls
`answer_row` directly, never `(q.set)`.

### N-1 (Nit) — `Opened::render`'s placeholder hack

`open_next_year.rs:104-105` pushes a literal `"  (TY{from} carried no payer…"` and then runs
`s.replace("{from}", …)` over the whole accumulated string. It is correct today only because the
header line's `{from}` was already interpolated. A `format!` would remove the coupling.

### N-2 (Nit) — `--from` is unvalidated

`open_next_year.rs:154` computes `let to = from + 1` before any check; `--from 2147483647` overflows
(a panic under `debug_assertions`, a wrap in release). Harmless in practice — the wrap then refuses
with *"no full-return inputs for 2147483647"* — but the arithmetic precedes every refusal.

---

## Seams checked clean

- **Seam 1 — the seed's leaves.** The copy-in construction means the risk surface is exactly the
  eleven named fields, and I walked each. Verified by plant that a copied money box reds
  (`no_money_leaf_…`, via `provenance::leaf_walk::nonzero_money_leaves`, with the ≥20-leaf vacuity
  guard measuring 21) and that a copied `PerYear` gate reds (`every_per_year_gate_on_the_seed_is_
  unanswered`, walked from `FORM_QUESTIONS` + `SKIPPABLE_QUESTIONS`, not a hand-list). All eighteen
  census `FormQuestion::set` closures delegate to `answer_row` — `grep -c "answer_row("` on the diff
  is **18** and `grep "ri.documents.[a-z0-9_]* = Some(v)"` returns **nothing**. `answer_log` and
  `answer_log_history` are empty on the seed. The venue slots are `CohortAnswers { None, None }`
  (the *keys* are I-3). `documents.*` are all `None` — covered by the registry walk, since every
  census row is a `FormQuestion`.
- **Seam 2 — the carryforward chain.** `roll_carryover_onto` is genuinely one definition; the
  destination-as-closure preserves refusal order; `apply_carryover_writeback` writes **only** the
  four carryover limbs (read at `return_1040.rs:3489-3524`) and nothing else crosses through it. The
  QBI pair is handled identically and named in `describe_carried`. The ungrounded capital-loss roll
  is correctly never stamped and is *named* rather than omitted (`not_stamped`). The
  `screen_inputs` before/after re-load in `write_back_carryover` is behaviour-preserving (its `else`
  arm is unreachable because the roll refuses without both). `report --write-carryover` is unmoved:
  `binary(tax_report) or binary(year_gate_t4) or binary(export_irs_pdf)` = **120 passed, 0 skipped**.
- **Seam 2b — the wrong chain.** The seed reads year N's **frozen return**, not its inputs:
  `the_carryforward_is_read_off_the_frozen_return_and_not_off_year_ns_inputs` passes on a fixture
  whose stored carryover-in ($60,000 long) differs from the return's carryover-out ($57,000 after the
  §1211(b) $3,000), and `no_money_leaf_…` pins `capital_loss_carryforward_in.long == 57000`.
- **Seam 3 — payer identities.** The payer half is right: the prompt names the payer and its
  EIN/TIN, the answer is the kind's census row, `None` blocks, and a `No` removes the pre-named rows
  through one writer. `row_is_pre_named` is a comparison against the row's own identity-only seed
  (not a box list) and fails **closed** — a row marked only with `transcribed_on` survives. A
  dependent's SSN is not printed. (The dependent and venue halves are I-2 and I-3.)
- **Seam 4 — T4's rules.** `coherence_check` first and deletes nothing; a parked draft refuses even
  with `--discard-draft`; a non-trivial WIP draft refuses and survives; year N without a committed
  row refuses; year N+1 with a committed row refuses and writes nothing; the write is `save_draft`
  and `return_inputs::get(N+1)` is `None` after opening, so `resolve.rs` never sees the seed. A
  second `open-next-year` **refuses** (the seeded draft is non-trivial) — conservative and correct;
  its only cost is M-1.
- **Seam 5 — the TUI.** The offer requires all three facts and says which is false otherwise;
  `handle_open_next_year_key` swallows every other key so `q` cannot quit through it; `Enter` never
  discards; the payload-confirm is **required** (`X` acts only when `blocked_by_draft.is_some()`) and
  names what would be lost. The `n` binding is held by `kat_keymap_overlay_lists_every_browse_char_
  binding`, and `edit/persist.rs` keeps `Session::conn()`/`save()` inside the KAT-G1 module.
- **Seam 6 — retention.** Year N's committed row (answer log included) is byte-identical; nothing is
  deleted or shredded. See M-3 for the kill's narrower scope.
- **Deviations 1, 4, 5, 6, 7** verified as described and are improvements, not regressions —
  particularly 6 (the tail-less `Dependent` literal as a T7 forcing function) and 7 (`leaf_walk`
  promoted so one detector serves both crates). Deviations 2 and 3 are I-1.

Counts: C=1 I=3 M=4 N=2
