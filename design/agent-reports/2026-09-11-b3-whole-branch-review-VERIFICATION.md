# Controller ledger — machine-checking the B3 whole-branch review

**Report under check:** `design/agent-reports/2026-09-11-b3-whole-branch-review.md` (persisted verbatim at
`587d7a9c`, 432 lines, 1C/2I/0M/1N). **Nothing was folded before this ledger was written.**

Every measurable claim below was re-derived by the controller from the tree at `587d7a9c`, not read out of
the report. Where a number or a path differs from the report, that is recorded rather than smoothed.

---

## C-1 — the form-engine surface never stamps `ReturnInputs.tax_year`

### The root claim: zero production writes in the editor chain — **CONFIRMED**

A `#[cfg(test)]`-stripped, line-tracking scan of the eight files the report names:

| file | production writes of `tax_year` | production reads |
|---|---|---|
| `btctax-tui-edit/src/main.rs` | — | — |
| `btctax-tui-edit/src/draw_edit.rs` | — | `:3087` |
| `btctax-tui-edit/src/edit/form.rs` | — | `:461` |
| `btctax-tui-edit/src/edit/persist.rs` | — | — |
| `btctax-input-form/src/apply.rs` | — | — |
| `btctax-input-form/src/spec/sections.rs` | — | — |
| `btctax-input-form/src/spec/mod.rs` | — | — |
| `btctax-cli/src/input_form_store.rs` | **`:426`** — see below | `:426` |

★ The one hit is **not a stamp**. `input_form_store.rs:424-429` is `draft_is_disposable`, where
`tax_year: ri.tax_year` appears inside a **comparand struct literal** (`*ri == ReturnInputs { tax_year:
ri.tax_year, filing_status: …, ..Default::default() }`) — it copies the value in order to exclude it from an
equality test. The report's "zero writes" stands; its two named reads are exactly right.

### The ordering defect — **CONFIRMED, and the sibling's fix is verbatim above it**

```
input_form_store.rs:666    if let Some(refusal) = screen_inputs(ri, table, params) {
                   :667        return Ok(CommitOutcome::Refused(refusal)); // fail-closed: writes nothing
                   :668    }
                   :669    mutate_and_save(sess, |conn| {
                   :670        crate::return_inputs::set(conn, year, ri)?;   ← the stamp is HERE, below the screen
```

```
cmd/tax.rs:280    return_inputs::stamp_year(&mut ri, year)?;
          :281    if let Some(refusal) = …::screen_param_free(&ri) {
```

and the comment at `cmd/tax.rs:274-279` is FR-103 in its own words: *"the stamp happened inside
`return_inputs::set` — BELOW this screen. So the screen read the TOML's `0` … and any rule that asks*
*"which tax year is this?" was structurally silent on `income import`, the one command that creates a row."*
**Two commands create a committed row. One was fixed; the other is `input_form_store::commit`.**

### The four asserted premises — **all four verbatim at the cited lines**

| site | the assertion |
|---|---|
| `return_inputs.rs:2104-2106` | *"the storage boundary stamps this from the row key on read and refuses a disagreement on write, so an in-memory value and the row it came from can never diverge"* |
| `questions.rs:920-923` | *"stamped from the storage row key on read, so this predicate reads a year that is true by construction. A year-0 … **fixture** is NOT ≥ 2025"* |
| `return_refuse.rs:2459-2461` | *"a yearless `ReturnInputs` is a test convenience, and refusing it would be an assertion about a year nobody named"* |
| `coverage.rs:696-699` | *"It is set by the command (`--year`) and stamped from the storage row key"* |

`return_inputs.rs:2813` is `tax_year: 0` under the comment *"`0` = NOT STATED. Default() is a test
convenience"*. `apply.rs:141` is `let mut ri = ReturnInputs::default();`. So the surface that materializes
the working return is the one the four comments call a test convenience.

### The blocking consequence's mechanism — **every hop CONFIRMED**

- `questions.rs:1920` — `live: |_ri| true` for `DigitalAssetActivity`. Every filer, every year.
- `questions.rs:1902-1903` — its `prompt` is commented **"★ The STATIC fallback; the words shown quote the
  year (`RENDERED_PROMPTS`)"**, and `questions.rs:102-123` lists it among the five.
- `apply.rs:116-121` — `record_or_forget` hashes `prompt_for_record(&key, ri)`, i.e. the **rendered** text,
  under the comment *"the words are RENDERED FROM THE RETURN, so a question that quotes a value hashes the
  sentence the filer actually saw."*
- `return_refuse.rs:2702-2707` — the `FORM_QUESTIONS` loop turns `AnswerStatus::WordingChanged` into
  `refuse(q.unanswered, WORDING_CHANGED_DETAIL)`.
- `return_refuse.rs:993-997` — that detail reads *"the wording of this question changed since you answered
  — so the answer on file was given to a different question."* **No wording changed; the year was stamped.**
- `tax_tables.rs:100-104` — `by_year.insert(2024, …)` and nothing else, so TY2024 is the only committable
  year. The report's "the only year that can commit" is exact.

### The five consequences — sites CONFIRMED

| # | claim | check |
|---|---|---|
| 2 | false `interview: complete` | `edit/form.rs:436` passes `self.working.as_ref()` (unstamped) → `year_readiness.rs:228` → `interview_state(ri)`; the strings are at `year_readiness.rs:254-256`. `questions.rs:924` is `live: \|ri\| ri.tax_year >= 2025`. ✔ |
| 3 | §152 walk at year 0 | `dependent_gates.rs:314, 402` take `ri.tax_year`. ✔ |
| 4 | the debt-ceiling warning never renders | `edit/form.rs:461` is the `full_return_for(&fr, ri.tax_year)` read. ✔ |
| 5 | kiddie-tax questions asked of a 60-year-old | `questions.rs:3348/3385/3441` + `return_1040.rs:1339-1344`. ✔ |

★ The report's own "tell" is real: `apply.rs:1623-1628`, the FR-97 fixture, sets `tax_year = 2024` by hand
with the comment *"the year is set directly because the tax year is not a form `Field` at all — the renderer
carries it"*, and `edit/form.rs:177` is that renderer's `pub year: i32`. The fixture compensates by hand for
what production never does.

★ One citation resolved differently than written: the report's seam-1 table says `interview_state.rs:549`
in a list of `btctax-cli` paths. The file is **`crates/btctax-core/src/tax/interview_state.rs`**, and `:549`
is `if let Some(r) = …::screen_param_free(ri)` — a real unstamped screen call on the editor's per-frame
path. The claim is right; only the crate is ambiguous in the write-up.

## I-1 — the registry draws the static prompt and hashes the rendered one — **CONFIRMED**

- `registries.rs:40-44` — `decl_tristate!` builds `Field { label_source: LabelSource::Authored, label:
  FORM_QUESTIONS[$idx].prompt, … }`. The **static** string.
- `draw_edit.rs:2946-2954` — the TUI formats `"  {}  [{}]{}"` with `f.label`. Drawn verbatim.
- `apply.rs:87-93` + `:116-121` — the hash is of `prompt_for_record` → `current_prompt` → the rendered text.
- The pin exists one registry over: `apply.rs:1856-1885`,
  `the_gate_fields_draw_the_words_they_hash_except_the_one_named_date_leaf`, walks
  `DependentGate::ALL.iter().copied()` and asserts no `GateKind::YesNo` gate differs, with the message *"a
  yes/no gate that draws one sentence and hashes another is C-1 again: the filer's answer would read as given
  under words they never saw."*
- **Nothing walks `QuestionId::ALL` for that property.** All nine `QuestionId::ALL` walks in the workspace
  were read: `spec/mod.rs:252` is a `field_to_question`/`question_to_field` **round-trip**;
  `questions.rs:3599/3675/3683` are ordering and length pins; `classifier.rs:1586`, `provenance.rs:789`,
  `return_refuse.rs:4122` are unrelated. FR-99's shape confirmed: the pin was written when the set had one
  member, and a second arrived beneath it.

## I-2 — shipped text names the wrong command — **CONFIRMED**

- `LIMITATIONS.md:425-427` (in the range, added at `52b348c2`): *"btctax will tell you when this is live:
  `report` prints an advisory naming how many of your dispositions occurred on an exchange…"*
- `broker_reporting_advisory` has **exactly one production caller**: `main.rs:1089`. The nearest preceding
  `Command::` arm is **`Command::ExportIrsPdf` at `main.rs:850`** (`Command::Report` opens at `:123`), so the
  advisory belongs to `export-irs-pdf`, not `report`.
- `report`'s only broker block, `render_broker_answers` (`render.rs:1950`), gates on
  `regime.is_some_and(|r| r.basis)` at `:1960` — false for TY2024.
- `main.rs:582` is `Command::Limitations => print!("{}", include_str!("../LIMITATIONS.md"))`. It is shipped
  filer-facing text, as the brief said.
- The report's own caveat holds: the substance is right, only the command name is wrong.

## N-1 — the broken intra-doc links — **CONFIRMED, and confirmed pre-existing**

`return_1040.rs:2016` and `return_refuse.rs:289` both link
`Advisory::ExcessSsSingleEmployerNotCreditable`; the variant is `ExcessSsNotCreditable`
(`advisories.rs:153`). `git log -L2016,2016:…/return_1040.rs` attributes the line to `03527f73`, and
`git merge-base --is-ancestor 03527f73 121c8805` **succeeds** — so it predates the range, exactly as the
report says.

## Seam-3's derivation — spot-checked

`SingleEmployerExcessSs` is named in production only as the enum variant (`return_refuse.rs:297`) and in
`attribute.rs:288`'s **exhaustive mapping arm**; its two other occurrences (`attribute.rs:589`, `:970`) are
tests. Not constructed in production, and `return_refuse.rs:284-296` states why. The report's "126 of 127
constructed, one stated boundary" is consistent with the sample.

---

## Verdict on the round

**The report's counts stand: 1 Critical / 2 Important / 0 Minor / 1 Nit.** No claim was found overstated;
one crate path in a seam table is ambiguous and one flagged `tax_year` occurrence is a comparand rather than
a write, neither of which changes a finding.

The round met its own success condition. The Critical is in the **earlier, fully per-task-reviewed** region
of the branch, not in the nine recent commits the brief pointed at — which is B3's documented precedent
reproduced rather than a miss, and the reviewer said so itself under *Refuted premises* instead of scoring
the point. The seams-traversed section names hops, derives its writer and refusal sets from the source, and
reports what it did **not** measure.

**Not yet done:** nothing is folded. The empirical kill — a test that reds on the year-0 commit path before
the fix — is the fold's obligation under harness rule B1 (*no checker exists until it has been observed RED
on a planted defect*), and it is the one thing this ledger could not establish by reading.
