# FR-97 — the TUI writes a dependent-gate answer with NO `AnswerRecord`

**Date:** 2026-09-07 · **Tree:** `/scratch/code/bitcoin_tax`, branch `main`, base HEAD `9e82a792`
**Scope:** FR-97 only, plus the widened kill. Nothing committed; nothing pushed.

---

## 1. What was wrong, in one paragraph

`btctax-input-form`'s `apply.rs::answer_key_for(id: FieldId)` mapped a field to `AnswerKey::Question`
or `AnswerKey::Skippable` and returned `None` for every `DepGate*` field — and for `DepDob`, which is
the twenty-first gate (`DependentGate::DateOfBirth`). So `Edit::SetField` on a dependent gate set the
leaf and recorded **nothing**, while `income answer` recorded the identical answer with its date and
its prompt hash. The key needs the row's SSN, which a bare `FieldId` cannot supply — hence a seam
change rather than an inline fix.

Measured, not asserted (plant 1 below): on one dependent row answered identically on both surfaces,
`income answer` wrote **15** `DependentGate` records and the editor wrote **0** — and T1's own
"identical records" kill stayed **GREEN** through it, which is the FR-97 entry's claim restated as a
measurement.

---

## 2. What changed, and where

| file | change |
|---|---|
| `crates/btctax-input-form/src/spec/registries.rs` | **+`gate_to_field(DependentGate) -> FieldId`** — the total, `_`-free map, MOVED here from `attribute.rs` so there is one copy, beside `question_to_field` / `skippable_to_field`. **+`field_to_dependent_gate(FieldId) -> Option<DependentGate>`**, DERIVED as its inverse over `DependentGate::ALL` — not a second hand-written match. |
| `crates/btctax-input-form/src/attribute.rs` | the private duplicate `dependent_gate_field` is now a one-line call into `spec::gate_to_field`; its §7 anchor behaviour is unchanged. |
| `crates/btctax-input-form/src/spec/mod.rs` | re-exports `gate_to_field` / `field_to_dependent_gate`. |
| `crates/btctax-input-form/src/apply.rs` | `answer_key_for(id, ri, addr)` — now `pub`, takes the row address and the return, and derives the gate arm from `field_to_dependent_gate` + `dependent_ssn_hash(row.ssn)`. New `prompt_for_record` (the words this surface showed) and `record_or_forget` (provenance follows the leaf). All three call sites — `SetField`, `ClearField`'s `clear` path, `ClearField`'s `empty` path — updated. |
| `crates/btctax-input-form/src/spec/coverage.rs` | comment only: the fixture's stated reason *"nothing in the form spec writes a dependent-gate record"* is retired and replaced with the operative one (the coverage KAT drives each `Field`'s own `set`, never `apply`, so the answer log — EXEMPT by prefix — is written one layer up). |
| `crates/btctax-cli/tests/tax_report.rs` | **+ the widened T1 kill**, `the_editor_and_income_answer_write_the_same_dependent_gate_records`. |
| `FOLLOWUPS.md` | FR-97 marked **CLOSED** with what closed it; **FR-100** and **FR-101** filed (§6). |

Three design decisions worth naming, all recorded in the code:

1. **`DepDob` is in scope.** It is `DependentGate::DateOfBirth`, `income answer` records it through
   the same site, and the derived inverse makes *excluding* it the awkward option. It is also always
   demanded — the Step 1 block demands it for every row before any other gate.
2. **The hashed comparand is the gate registry's own words** (`entry(gate).prompt_text(ri, None)`),
   not `Field::label` and not `current_prompt` (which returns `None` for the one params-quoting gate
   by design). The registry's words are what `current_prompt` resolves and what `income answer`
   writes, so a record written in the editor reads `Given`, not `WordingChanged`, the instant it is
   written — the C-1 brick, avoided from the other side. Empirically forced: the first draft asserted
   `prompt_hash(Field::label)` and went red on `DateOfBirth` (see FR-100).
3. **Provenance follows the leaf.** The twenty tri-state gates cannot be set to "unanswered"
   (`Field::set` refuses `TriState(None)`), but `DepDob` accepts `Date(None)`; a `Given` record for a
   date that is not on the row would be testimony nobody gave, so that set forgets the record instead.

---

## 3. The kill — widened, not copied

The instruction was to make coverage **derived**, and not to add twenty entries to a list. There are
two new checks and neither carries a list of gates:

**(a) The coverage partition** — `apply.rs::the_fields_that_record_an_answer_are_exactly_the_three_registries_images`.
It asserts set EQUALITY between

* the image of the three registry→form maps over the registries' own totality
  (`FORM_QUESTIONS` → `question_to_field`, `SKIPPABLE_QUESTIONS` → `skippable_to_field`,
  `DependentGate::ALL` → `gate_to_field`), and
* every `Field` in `form_spec()` for which `answer_key_for` yields a key,

in **both directions**, and then requires each gate's key to name the row's own `ssn_hash` (which a
set comparison cannot see, and which also makes the map's injectivity load-bearing and checked).

**(b) The cross-surface byte comparison** — `tax_report.rs::the_editor_and_income_answer_write_the_same_dependent_gate_records`,
T1's central kill widened to the gates. It drives the real `income answer` over a vault with one
dependent row, then answers the same row through `apply(Edit::SetField)` at the same `now`, and
compares every `DependentGate` record serialized, byte for byte. Nothing is hand-listed: the gate set
is whatever the keyboard recorded, each answer is read back off the CLI's own row, and liveness comes
from `walk_dependent`'s demands (answering one gate is what makes the next live).

Three more tests hold the parts a set comparison cannot reach:
`every_demanded_dependent_gate_answered_through_apply_records_the_registrys_words`,
`the_gate_fields_draw_the_words_they_hash_except_the_one_named_date_leaf`,
`emptying_a_dependents_date_of_birth_forgets_its_record_rather_than_dating_a_blank`,
`the_params_quoting_gate_records_the_fallback_it_drew_and_is_re_asked_once_the_figure_lands`.

### B1 — every kill watched RED on a planted defect

Each plant was made, `find crates -name '*.rs' -exec touch {} +` run, the tests run, then the file
restored from a `cp` backup and touched again (FR-90). No `git checkout --`, `restore` or `stash` was
used at any point.

| # | plant | observed |
|---|---|---|
| 1 | `answer_key_for`'s gate arm returns `None` (the original FR-97 defect) | 4 of 5 input-form tests RED; coverage names all **21** fields (`left: ["DepDob", "DepGateCitizenNationalOrResidentAlien", … 21 entries]`, `right: []`). Cross-surface kill RED: `left:` 15 `DependentGate` keys, `right: []`. **T1's original kill PASSED throughout** — the blind spot, measured. |
| 2 | a hand-list that covers the gates it was written beside and misses the one added later (`if matches!(id, FieldId::DepDob) { return None }`) | coverage RED naming exactly `left: ["DepDob"]`; 3 of 5 RED. |
| 3 | C-1's shape — the row banner folded into the hashed comparand | `…records_the_registrys_words` RED (`QcRelationship: the record must hash the gate REGISTRY's words`); params-boundary test RED; cross-surface kill RED (`QcRelationship: the two writers produced DIFFERENT records…`). |
| 4 | `dep_gate_tristate!`'s `label` switched to `unanswered_detail` (the pane draws one sentence, the record hashes another) | `the_gate_fields_draw_the_words_they_hash_…` RED, listing all **20** yes/no gates. |
| 5 | `record_or_forget`'s early return removed (provenance stops following the leaf) | `emptying_a_dependents_date_of_birth_…` RED. |
| 6 | a **twenty-second** gate added to `DependentGate` | does not compile: `error[E0004]: non-exhaustive patterns: …DependentGate::PlantedTwentySecondGate not covered --> crates/btctax-input-form/src/spec/registries.rs:936:11`. |
| 7 | a **collision** in `gate_to_field` (`LivedWithYouInUs → DepGateLivedWithYouOverHalfYear`), which is the one way a derived inverse can silently lie | coverage RED: `left: Some(DependentGate { …, gate: LivedWithYouOverHalfYear })` / `right: Some(DependentGate { …, gate: LivedWithYouInUs })`. |

Plants 6 + 7 together are the answer to *"does it red when a twenty-first gate appears?"*: a new gate
is a **compile error** until it is placed in the one total map, and once placed it reaches
`answer_key_for` for free — while a map that placed it **wrongly** (onto an existing field) reds in
(a). Restoration verified by `md5sum` against the backups: all four touched files byte-identical to
their pre-plant state, and `git status` shows `sections.rs` and `provenance.rs` unmodified.

### What the widened check covers — and what it does NOT

**Covers.** Every `FieldId` in `form_spec()` is accounted for: it either yields an `AnswerKey`, or it
is not in any registry's image — and the two sets are compared, so neither a field that records
without an owner nor a registry field that records nothing can pass. All 21 dependent gates key on
the row's salted SSN. Every gate the flowchart demands, answered through `apply`, carries a record
with the seam's date, `state: Given`, and the registry's own prompt hash; and `answer_status` reads
`Given` for each. On a real qualifying-child row, all 15 records the keyboard wrote are byte-identical
to the editor's.

**Does not cover.**

1. **`DependentGate::GrossIncomeUnderLimit` is NOT byte-identical across the two surfaces, by
   design.** Its prompt quotes the year's §152(d)(1)(B) figure; the form seam holds no
   `FullReturnParams`, so the editor draws and hashes FR-83's figureless fallback while `income
   answer` (which holds the package) hashes the rendered question. The consequence is asserted rather
   than glossed: `the_params_quoting_gate_records_the_fallback_it_drew_…` shows the record carries the
   fallback's hash, that `interview_state_with_params` then lists the gate as **blocking** with
   `WORDING_CHANGED_REASON`, and that `Edit::ClearField` withdraws it. That is strictly better than
   what it replaces (no record at all ⇒ the gate counted as answered under words nobody was shown),
   but it is a boundary, not a closure — filed as **FR-101**.
2. **`DepDob` draws a caption, not the registry's question.** The record hashes the registry's words
   (it must). The divergence is allowed by exactly one test, derived by `GateKind` rather than by
   name, and a *second* one reds — filed as **FR-100**.
3. The coverage partition says nothing about whether a field that records records the *right* key or
   the *right* words; that is (b) and the three supporting tests.
4. Nothing here checks the **TUI keystroke layer** above `apply` (`edit/tax_inputs.rs`), only the
   seam every renderer must go through. The cross-surface kill enters at `Edit::SetField`, which is
   what the brief specified and what a keystroke produces.

---

## 4. Are the two surfaces identical now? Shown, not asserted

`the_editor_and_income_answer_write_the_same_dependent_gate_records` compares
`serde_json::to_string(record)` for every gate key, and its first assertion compares the two key
lists. Under plant 1 the CLI list had 15 entries and the editor list was empty; at HEAD-with-fix both
lists are equal and every record matches byte for byte:

```
cargo nextest run --locked -p btctax-cli -E 'test(the_editor_and_income_answer_write_the_same_dependent_gate_records)'
        PASS [   0.213s] (1/1) btctax-cli::tax_report the_editor_and_income_answer_write_the_same_dependent_gate_records
     Summary [   0.213s] 1 test run: 1 passed, 825 skipped
```

The 15 gates compared on that fixture (derived, printed by the plant-1 failure): `QcRelationship,
ProvidedOverHalfOwnSupport, FilingJointReturn, QualifyingChildOfAnotherPerson,
CitizenNationalResidentOrCanadaMexico, Married, TinIssuedByDueDate, CitizenNationalOrResidentAlien,
SsnsValidForEmploymentIssuedByDueDate, YoungerThanYouOrSpouse, LivedWithYouOverHalfYear,
LivedWithYouInUs, FullTimeStudent, PermanentlyAndTotallyDisabled, DateOfBirth`.

---

## 5. Commands, with their real output

```
$ cargo nextest run --locked -p btctax-input-form
     Summary [   0.096s] 80 tests run: 80 passed, 0 skipped

$ cargo nextest run --locked -p btctax-tui-edit
     Summary [   2.100s] 402 tests run: 402 passed, 2 skipped

$ cargo nextest run --locked -p btctax-cli
     Summary [   6.700s] 825 tests run: 825 passed, 1 skipped

$ cargo fmt --all
$ CARGO_TARGET_DIR=target-clippy cargo clippy -p btctax-input-form -p btctax-cli \
      --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 5.37s          (no warnings)
```

### The five instruments

```
$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
f1040s3:5 f1040sa:21 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12 f8959:17
f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)

$ cargo run -q -p xtask -- census-join
census join: 274 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -q -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91
registry prompts scanned; no forbidden shape

$ cargo run -q -p xtask -- prompt-check
xtask prompt-check: OK — 88 assertions, all verbatim

$ cargo run -q -p xtask -- box-census
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)
```

All five match the briefed baselines exactly (375/18/31 (ratchet 31)/0/17 · 274 across 13 · 8+4+6 and
91 · 88 · 268/19/9). `cite-check` also run, unbriefed: **OK — 51 quotations, all verbatim**, 31
excused, 0 unaccounted.

### `make docs`

```
$ make docs
… wrote /scratch/code/bitcoin_tax/docs/pdf/btctax.pdf
$ git status --short          # after make docs
 M FOLLOWUPS.md
 M crates/btctax-cli/tests/tax_report.rs
 M crates/btctax-input-form/src/apply.rs
 M crates/btctax-input-form/src/attribute.rs
 M crates/btctax-input-form/src/spec/coverage.rs
 M crates/btctax-input-form/src/spec/mod.rs
 M crates/btctax-input-form/src/spec/registries.rs
```

**No man-page diff** — `docs/man/*.1` unchanged; the working tree holds only the seven files this
task edited.

### The closing gate

```
$ find crates -name '*.rs' -exec touch {} +      # FR-90
$ make check
     Summary [  20.942s] 3543 tests run: 3543 passed, 12 skipped
```

Clippy ran concurrently and emitted nothing; no `make check: FAILED` line. **3537 → 3543 is +6, and
the six are exactly the tests added here** (five in `apply.rs`, one in `tax_report.rs`). Skips
unchanged at 12.

---

## 6. Deviations, and defects found outside FR-97

**Deviation 1 — the fix covers 21 gates, not the 20 the entry names.** `DepDob` carries
`DependentGate::DateOfBirth`, `income answer` records it at the same site, and the derived inverse
covers it automatically; excluding it would have meant writing the exception the brief forbids.

**Deviation 2 — the total gate map MOVED rather than being written again.** T7 had already written it
as `attribute.rs::dependent_gate_field`. A second copy would have been the exact FR-99 shape, so it is
now `spec::gate_to_field` with `attribute.rs` delegating; §7 attribution behaviour is unchanged and
its tests pass.

**Deviation 3 — `answer_key_for` is now `pub`.** The coverage partition has to ask the seam *"what
does this field record?"* over every field, and liveness makes that unreachable through `apply` alone
(a gate the walk does not demand refuses the set). The behavioural kills still enter at
`Edit::SetField`.

Two defects found outside FR-97, filed and **not** fixed:

* **FR-100 — `DepDob` draws a caption where the gate registry asks a question.** The twenty tri-state
  gate `Field`s take `label` from the registry (so the pane draws the sentence the record hashes);
  `DepDob` predates T7's registry and draws *"Date of birth"* against the registry's *"What is this
  person's date of birth?"*. Hashing the caption instead is not an option (it would make the editor's
  own answer read `WordingChanged` forever), so this is a renderer wording item. Owning phase:
  ownerless residue (renderer wording). Pinned by a test that allows exactly this one and reds on a
  second.
* **FR-101 — the form seam holds no `FullReturnParams`, so the params-quoting gate is drawn (and now
  recorded) with its figureless fallback even on a year whose package IS bundled.** FR-83 closed the
  params-less half of this two-surface split; this is the params-having half, and the label half is
  pre-existing (`grep -rn "prompt_text" crates/btctax-tui-edit/src` → no matches, so the pane can only
  draw the static prompt). FR-97 makes the consequence visible rather than silent. Closing it changes
  `apply`'s signature and the `Field` label contract, which §10 freezes — a cycle, not an edit.
  Owning phase: whichever cycle next touches the form seam's signature.

Nothing else was touched: no whole-branch review was started, no other guard refactored, and
FR-88 / FR-90 / FR-99 were left as filed doctrine items for the owner.
