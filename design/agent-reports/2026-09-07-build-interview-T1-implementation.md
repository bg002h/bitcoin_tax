# Build report — interview T1: the provenance schema

Single implementer, shared main tree, branch `main`, dispatched at `976a0b63`. No subagents, no commit,
no push. Contract: `design/SPEC_interview.md` r2 (R10, R12's inputs, §5/§5.6, §7 row T1, §8) plus the
fold report's T1-tagged rows (I9, I10).

**Suite at dispatch: 3175 passed. Suite now: 3197 passed, 12 skipped, 0 failed** (`cargo nextest run
--locked --workspace`). +22 = exactly the 22 tests added below. `cargo fmt --all --check` clean;
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
clean.

---

## 1. What landed, per T1 item

### 1.1 `LEAF_SOURCE` — the per-leaf provenance table, with the both-directions KAT

New module `crates/btctax-core/src/tax/provenance.rs` (registered `tax/mod.rs`).

| thing | where |
|---|---|
| `DocumentKind` | `provenance.rs:42` |
| `Source { Document(_) \| FilerRecords \| Ledger \| Answer \| Computed }` | `provenance.rs:53` |
| `LEAF_SOURCE: &[(&str, Source)]` — 18 prefixes | `provenance.rs:78` |
| `source_of_leaf` / `prefix_matches` | `provenance.rs:118`, `:128` |

**The money-leaf set is detected BY TYPE, not by a hand-list and not by a value.** For every serde leaf
of the maximal fixture, the KAT writes `"1234.56"` and then `"zzz"` into that leaf and re-deserializes
the whole blob: a leaf that **accepts the decimal and rejects the non-numeric** is a `Usd` /
`Option<Usd>`; a `String` accepts both; a `bool` / `Date` / enum rejects both. It is `Decimal`'s own
deserializer doing the classifying, so a newly-added money field is caught with nobody having remembered
to list it. **Measured: 106 money leaves**, all sourced, no prefix stale, none double-claimed.

Its honest limit is stated in the doc comment rather than hidden: it can only classify a leaf the
fixture **realizes**. A new plain `Usd` defaults to `0` and is realized; a new `Option<Usd>` left `None`
serializes as `null`, rejects both probes and reads as *not money* — for which the second net is the
classifier, which forbids `_` on an `Option<Usd>` leaf.

The fixture is `scrub_axis::maximal_sentinel()` — the repo's existing exhaustive-struct-literal maximal
fixture (a new field anywhere is an `E0063` *there*), rather than a second fixture that could rot.

### 1.2 Document identity on document rows

`payer_tin: String` + `transcribed_on: Option<Date>`, both `#[serde(default)]`, on `Form1099Int`
(`return_inputs.rs:90,94`), `Form1099Div` (`:116,119`), `Form1099G` (`:149,152`), `Form1099B`
(`:183,186`) — exactly §5.2's table. (W-2 not touched: see Deviation D3.)

**The compiler routed this into the PII scrub, which is where the real finding was.** `scrub_pii`'s
`..`-free destructures went red, and the answer is not `_`: a payer TIN is the same class of identifier
as a W-2 EIN, so it now maps through the **same `EinMap`** — preserving both the partition and the
validity class — via `map_payer_tin` (`scrub.rs:812`), which leaves a trim-empty TIN exactly as it was
so a scrubbed copy cannot differ from the original in whether an emptiness check fires. Four new rows in
the §3.3 scrub matrix (`scrub_axis.rs`, `int_1099[].payer_tin` and siblings), each with `absent` and
`empty` exercised and `malformed` recorded as `NO_READER` **with the reason**: nothing reads a validity
class off `payer_tin` until T5's document screens, and a `Fixture` cell would compare `None == None` —
the "exercised and vacuous" state that module exists to prevent. `transcribed_on` is KEPT by the scrub
(a provenance tag, not a person) and that is recorded as a decision at each site.

### 1.3 The answer log, keyed by identity, with ONE writer

| thing | where |
|---|---|
| `DependentGate` — 21 identities (§5.3's 16 gates + the 4 row-(5)/(6) facts + `DateOfBirth`) | `provenance.rs:139` |
| `AnswerKey { Question \| Skippable \| DependentGate { ssn_hash, gate } }` + its string wire form | `provenance.rs:201` |
| `AnswerState { Given, Declined }`, `AnswerRecord`, `AnswerLogHistory` | `provenance.rs:287,297,306` |
| `prompt_hash`, `dependent_ssn_hash` | `provenance.rs:310`, `:325` |
| **`record_answer` — the one writer** | `provenance.rs:347` |
| `forget_answer` (un-answer ⇒ never-asked) | `provenance.rs:370` |
| `ReturnInputs.answer_log` / `.answer_log_history` | `return_inputs.rs:1327`, `:1338` |
| reached from the editor: `answer_key_for` + the `apply` call | `apply.rs:37`, `:84` (clear at `:107`, `:122`) |
| reached from the keyboard: `income answer` | `answer.rs:194` (declaration), `:313` (skippable) |

`answer_key_for` is derived from the two registry maps that already exist (`field_to_question` /
`field_to_skippable`), so "the fields that record" **is** "the fields that delegate to a registry
question", by identity rather than by a second list that can drift.

`dependent_ssn_hash` normalises punctuation (`111-22-3333` and `111223333` are one child) and is
domain-separated SHA-256; the doc states plainly that this is **domain separation, not
confidentiality** — the row's `ssn` sits in the same encrypted blob — and that what it buys is that the
*key*, the thing that reaches a panel or a manifest line, is never the nine digits.

### 1.4 The `prompt_hash` mismatch rule

- `answer_status` (`provenance.rs:393`) returns `NeverAsked | Given | Declined | WordingChanged`.
- `WORDING_CHANGED_REASON` (`:390`) — one constant, so the panel, the refusal and the test cannot drift.
- `supersede_stale_prompts` (`:433`) moves every mismatched record **out of** `answer_log` and **into**
  `answer_log_history`; idempotent by construction, so a sweep on every load cannot grow the history.
- `screen_inputs` refuses a live class-(A) record whose hash disagrees, with `WORDING_CHANGED_DETAIL`
  (`return_refuse.rs:468`, applied at `:1158-1161`) — which names the reason **and the exit**.
- **An ABSENT record is not a mismatch.** Only a record that exists and disagrees refuses. Treating "no
  record" as unanswered would refuse every return in the corpus and would assert a provenance nobody
  ever collected; the kill pins that half explicitly.

### 1.5 Dependent identity maintenance, at the SEAM

`retire_dependent_identity` (`provenance.rs:471`, called from the Dependents section's `remove`,
`sections.rs:627`) and `supersede_dependent_identity` (`provenance.rs:485`, called from the `DepSsn`
setter, `sections.rs:548`). Done in the seam's own closures rather than in `apply`, so every caller of
the form engine gets it — the TUI, a future web renderer, and any test driving `Field::set` directly.

### 1.6 `CarryProvenance::ComputedFromPriorReturn { year }`

`return_inputs.rs:612`. `Copy` preserved; `Default` still `User`; round-trips.

### 1.7 `SCHEMA_VERSION` 3 with the refuse / discard / parked-refuse split

`crates/btctax-cli/src/return_inputs.rs:35`, **2 → 3**. The doc records *why* a bump when every new
field is `#[serde(default)]`: a v2 blob would load cleanly **and wrongly** — every declaration has a
value and no `AnswerRecord`, which the model reads as *"answered, but nobody knows when or in what
words"*, and R10.3's mismatch rule then has nothing to compare. The three-way split already existed and
is now killed at v2 specifically (§2, kills 8–10).

---

## 2. Kills — every one seen RED on a planted defect (B1)

Planted, observed, reverted from a `cp` backup. Verbatim red text.

| # | guarantee | planted defect | the test that redded, and its message |
|---|---|---|---|
| 1 | every `Usd` leaf has a source | deleted `("w2s", …)` from `LEAF_SOURCE` | `provenance::tests::every_money_leaf_has_exactly_one_source_and_every_source_prefix_is_live` — `left: Audit { unsourced: ["w2s[0].box10_dependent_care", "w2s[0].box12[0].amount", … 26 leaves …], unmatched: [], multi: [] }` |
| 2 | the money detector discriminates | dropped the `!accepts(p, "zzz")` half | `provenance::tests::the_money_detector_separates_usd_from_the_leaves_that_look_like_it` — `w2s[0].employer is NOT a money leaf but was detected as one` |
| 3 | a stale prefix reds | in-test mutation adding an unmatched prefix | `a_prefix_that_matches_no_money_leaf_reds_the_kat` (built-in negative test) |
| 4 | the table is a partition | in-test mutation adding `schedule_a.medical` beside `schedule_a` | `two_prefixes_claiming_the_same_leaf_red_the_kat` (built-in negative test) |
| 5 | the old answer never stays current | `supersede_stale_prompts` stopped moving the record | `changing_the_words_re_asks_and_historises_restoring_them_does_neither` — `the superseded record must NOT stay current` |
| 6 | `screen_inputs` refuses a changed-wording answer | deleted the `answer_status` check | `return_refuse::tests::an_answer_hashed_against_earlier_words_refuses_and_a_missing_record_does_not` — `assertion left == right failed: a record hashed against EARLIER words must refuse as UNANSWERED / left: None / right: Some(ScheduleBPart3Unanswered)` |
| 7 | removing a row takes only that identity's records | dropped `retire_dependent_identity` from the seam's `remove` | `sections::…::removing_a_dependent_row_takes_only_that_identitys_answers_and_a_new_ssn_starts_fresh` — `the removed row's records must go with it` |
| 8 | a changed `ssn` supersedes | dropped `supersede_dependent_identity` from the `DepSsn` setter | same test — `the old identity's records must stop standing as this row's answers` |
| 9 | the two writers agree | made `apply` stamp `now.next_day()` | `tax_report::the_editor_and_income_answer_write_the_same_answer_record` — `the two writers produced DIFFERENT records for the same answer at the same BTCTAX_NOW … left: {"answered_on":[2026,244],…} right: {"answered_on":[2026,245],…}` |
| 10 | `Declined` ≠ `Given` | `skippable_state` always returned `Given` | same test — `a skippable the filer passed over is a DECLINED record — asked, and refused / left: Some(Given) / right: Some(Declined)` |
| 11 | a v2 committed row refuses | `SCHEMA_VERSION` back to 2 | `return_inputs::p9_stale_row_refuses::a_version_2_row_refuses_stale` — `a v2 row (the pre-interview provenance schema) must refuse as stale` |
| 12 | a v2 WIP draft discards with a note / a v2 parked draft refuses | same | `input_form_store::tests::a_version_2_wip_draft_is_discarded_with_a_note_and_a_version_2_parked_draft_refuses` — `assertion failed: matches!(loaded, Loaded::Fresh)` |
| 13 | the answer log is exempt from coverage **deliberately** | deleted the `answer_log_history` exemption | `coverage::every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` — `these IN-SCOPE leaves are covered by NO Field and are NOT in EXEMPT …: ["answer_log_history[0][0]", "answer_log_history[0][1].answered_on", …]` |
| 14 | the scrub re-keys dependent identities | dropped `rekey_dependent_answers` | `scrub::tests::a_dependent_gate_key_is_rekeyed_so_no_original_ssn_hash_survives` — `the scrubbed copy still carries the ORIGINAL identity hash for 111-22-3333 — over a nine-digit space that IS the SSN` |
| 15 | the scrub maps a payer TIN | dropped `map_payer_tin` from the 1099-INT loop | `scrub_axis::matrix::every_replaced_field_preserves_its_class_in_every_representable_state` — `§3.3's matrix has a row for int_1099[].payer_tin, which is NOT in the derived axis — either the fixture stopped being maximal or scrub stopped replacing this field` |

**22 tests added** (1230 core, 66 input-form, 716 cli, 382 tui-edit — all green).

### 2.1 An instrument caught this build's own fixture, mid-work

The scrub-axis baseline control (*"the maximal sentinel must be a FILEABLE return — a refusing baseline
masks every cell of this matrix"*) went red with `Some(ScheduleBPart3Unanswered)` the moment the sentinel
gained an `answer_log` entry, because the entry hashed an invented string and §1.4's new refusal fired
on it. Fixed by hashing the **registry's own words** (`scrub_axis.rs`, `current_prompt(&key)`), and the
same fix applied pre-emptively to the coverage KAT's fixture. Recording it because it is the strongest
evidence in this build that both the new rule and the old control work.

---

## 3. Deviations — every one, with its reason

**D1 — `record_answer` takes five arguments, not R10.3's four.** The same rule requires `Declined` to be
recordable (*"a skippable skipped on purpose records `Declined`"*), which `record_answer(ri, key, prompt,
now)` cannot express. `state` is the fifth parameter rather than a second writer function: **one writer**
is the guarantee, four arguments were the sketch. Documented at the function.

**D2 — `apply` gains a `now: Date` parameter.** R10.3 requires `apply` to call `record_answer`, whose
`now` is the `BTCTAX_NOW` seam; `apply(w, e)` had no clock. Signature is now `apply(w, e, now)`; 101 call
sites updated (74 in `apply.rs`'s own tests, 27 in `btctax-tui-edit`). The TUI's production path reads it
**once at open** from `app.clock` into a new `TaxInputsFormState.now` (`edit/form.rs`, set in
`open_tax_inputs_form`), never from a wall clock inside the edit loop — a pinned clock must pin the answer
log too. `TaxInputsFormState::fresh(year)` → `fresh(year, now)` (test-only helper; no production caller).

**D3 — W-2 gains neither `payer_tin` nor `transcribed_on`.** R10.2 says *"`transcribed_on: Option<Date>`
per row"*, but §5.2's table — the authority for **which** structs — omits the W-2 entirely, and the W-2
already carries `ein` as its identity. Adding `transcribed_on` there would put an uncovered leaf into the
input-form coverage census (the W-2 *is* an in-scope form section, unlike the 1099 vectors), forcing
either a new filer-facing `Field` (T5 owns the document screens) or a fresh exemption — a widening of a
ratchet the spec says may only shrink. **Flagged for T5**: when the W-2 document screen lands, decide
`w2s[].transcribed_on` with its `Field`.

**D4 — `digital_asset_activity: Option<bool>` NOT added**, though §5.6 lists it. §7's T1 row does not, and
the brief makes that row authoritative. R14 classifies it class **(A)** with a `FORM_QUESTIONS` entry,
which is T6's (`the exchange seam`) — and adding the bare leaf now would either force a classifier
exemption T6 must undo, or, with its registry entry, make `screen_inputs` refuse every return whose value
is `None`. Owned by **T6**.

**D5 — the `Dependent` gate FIELDS are not added, only the gate IDENTITIES.** §5.3's 20 `Option<bool>`
leaves belong to T7. T1 adds `DependentGate` (21 variants, transcribed from §5.3 + R14) because the
answer-log **key** is the thing that cannot be back-filled. The T1 kills drive gate records through
`record_answer` directly, which is legitimate — `record_answer` is the writer under test.

**D6 — `supersede_stale_prompts` skips `DependentGate` keys.** `current_prompt` has no prompt for them:
the `DEPENDENT_GATES` registry is T7's. Rather than invent a hash, the sweep leaves those records standing
and `current_prompt` documents it. Owned by **T7**.

**D7 — `answer_log_history` needed its own `EXEMPT_PREFIXES` entry**, not just §5.7's `answer_log`. The
census's prefix matcher requires `.` or `[` after a prefix, so `answer_log` does not cover
`answer_log_history`. Both entries carry the reason (provenance is ours, not the filer's — there is no
`Field` that could cover a record and no prompt that could ask for one). Kill #13 pins it.

**D8 — a `ClearField` deletes the record; it does not write `Declined`.** §5.6 enumerates what history
holds (records superseded by a changed prompt or a changed `ssn`) and a clear is neither; and `Declined`
is *"asked and passed over"*, which a class-(A) declaration has no lawful version of. Un-answering returns
the question to **never asked**, matching the leaf going to `None`.

**D9 — the GENERATED examples fixture was NOT regenerated.** Reason, and it is a finding rather than a
shortcut: `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` is **not idempotent under its
own emitter today**. Running the header's command
(`cargo test -p btctax-cli --test fullreturn_oracle -- --ignored emit_fullreturn_fixture`) produced
15 insertions / 10 deletions, of which the T1-caused part is only `+answer_log_history = []`,
`+[answer_log]`, and `+payer_tin = ""` ×3 — the rest **deletes three hand-added comment blocks** (the
§170(f)(8), Schedule D line 20 and CWA notes, each warning *"DELETING THIS LINE REFUSES THE RETURN"*) and
adds/reorders `b_1099 = []`, `[broker_reporting]`, `filing_form_4952`. A prior hand-edit of a file marked
`GENERATED — do not hand-edit` has been surviving because `toml::from_str` ignores comments. Regenerating
is behaviour-neutral but destroys that documentation for reasons unrelated to T1, and **no regeneration is
required**: every new field is `#[serde(default)]` and the kitchen sink leaves them at their defaults, so
`fullreturn_oracle::fullreturn_fixture_is_the_kitchen_sink_oracle` is green as committed. The change was
made, inspected, and reverted. **Filed as a follow-up below.**

**D10 — `btctax-cli` gains a DEV-only dependency on `btctax-input-form`.** Kill #9/#10 must drive *both*
writers in one process; `btctax-cli` previously could not reach `apply`. No cycle (`btctax-input-form`
depends on `btctax-core` alone) and no runtime dependency. `Cargo.lock` gains one line.

---

## 4. Files touched

Core: `tax/provenance.rs` (**new**, 1033 lines incl. tests), `tax/mod.rs`, `tax/return_inputs.rs`,
`tax/classifier.rs`, `tax/return_refuse.rs`, `tax/questions.rs`, `tax/scrub.rs`, `tax/scrub_axis.rs`,
`tax/return_1040.rs` (6 fixture literals). Input-form: `apply.rs`, `spec/sections.rs`, `spec/coverage.rs`.
CLI: `Cargo.toml`, `src/return_inputs.rs`, `src/input_form_store.rs`, `src/cmd/answer.rs`, `src/main.rs`,
`tests/tax_report.rs`, `tests/tax_profile.rs`. TUI-edit: `edit/form.rs`, `edit/tax_inputs.rs`,
`draw_edit.rs`, `main.rs`. Docs: `docs/examples/examples.md`,
`design/usage-examples/SPEC_post_v070_product_cycle.md`. Plus `Cargo.lock`.

### 4.1 Golden / fixture diffs

**`docs/examples/examples.md` — 9 insertions, 1 deletion**, the direct and only consequence of the schema
(the `income show` JSON block gains `"payer_tin": ""` and `"transcribed_on": null` on the three 1099 rows,
and `"answer_log": {}` / `"answer_log_history": []` at the tail). Regenerated with
`cargo run -p xtask -- examples > docs/examples/examples.md`;
`xtask::examples::tests::examples_golden_matches_committed` green.

**Man pages: NO change.** `make docs-man` regenerated them and `git status docs/` shows only
`examples.md` — no CLI surface moved.

**Census registers and `max_unwitnessed`: NOT moved.** `git diff --stat crates/xtask/` is empty.

**Two non-code enumerations updated, both because an in-repo tripwire demanded it:**
- `crates/btctax-cli/tests/tax_profile.rs` — `provenance.rs` added to the audited `serde_json::Value`
  output sites, with the audit written out (every `Value` dies inside the KAT; the function returns
  `BTreeSet<String>`; key order cannot reach persisted or fingerprinted bytes). The test's own failure
  message instructs this.
- `design/usage-examples/SPEC_post_v070_product_cycle.md` §9.6 — the matching sentence, because that
  enumeration is what the tripwire is pinned to and a silent divergence is exactly what it guards.
  (Not `SPEC_interview.md`, which was not touched.)

### 4.2 Pinned numbers moved

| number | old | new | cause |
|---|---|---|---|
| `crates/btctax-cli/src/return_inputs.rs:35` `SCHEMA_VERSION` | 2 | 3 | R10 / §5.6, the provenance schema |
| `tax_report.rs` recovered-row assertion | literal `2` | `return_inputs::SCHEMA_VERSION` | follows the bump; the property is *"the recovered row is current"*, which is relative by nature. Test renamed `clear_then_import_recovers_a_stale_row_to_v2` → `..._to_the_current_schema_version`, since a name asserting a literal goes stale silently. The v2 **refusal** kills stay pinned to the literal 2 on purpose — those are about one specific predecessor. |

No other pinned count moved: `QuestionId::ALL.len() == 17`, `FORM_QUESTIONS.len() == 17`,
`SKIPPABLE_QUESTIONS.len() == 19` and the coverage counts are all unchanged.

---

## 5. Commands run

```
cargo nextest run --locked --workspace                     3197 passed, 12 skipped, 0 failed
cargo nextest run --locked -p btctax-core                  1230 passed, 0 skipped
cargo nextest run --locked -p btctax-input-form              66 passed, 0 skipped
cargo nextest run --locked -p btctax-cli                    716 passed, 1 skipped
cargo nextest run --locked -p btctax-tui-edit               382 passed, 2 skipped
cargo fmt --all --check                                    clean
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace \
  --all-targets --all-features -- -D warnings              clean
make docs-man                                              no diff
cargo run -p xtask -- examples > docs/examples/examples.md  9 +, 1 -
```

---

## 6. Not done, and why — carry these forward

1. **D9's fixture drift** — `fullreturn_inputs.toml` is a `GENERATED` file that has been hand-edited with
   three comment blocks its emitter destroys. Either teach the emitter to carry per-key annotations, or
   move the notes into `fullreturn_oracle.rs` beside the fixture. Left untouched; **not** T1's to decide.
2. **`w2s[].transcribed_on`** (D3) — for **T5**, with its `Field`.
3. **`digital_asset_activity`** (D4) — for **T6**.
4. **The `Dependent` gate fields** (D5) and **dependent-gate prompt hashing** (D6) — for **T7**. Note also
   that a dependent row with a **blank `ssn`** has no identity, so every blank row shares one hash bucket;
   `retire_dependent_identity`'s doc records it and T7 should decide it rather than meet it.
5. **`ScheduleBRecord.payer_ssn`** (§5.1, T5) is a real individual's SSN, unlike `payer_tin`. `mask_pii`
   (`cmd/tax.rs:255`) masks the taxpayer/spouse/dependent SSNs and the IP PIN and nothing else — T5 must
   extend it, or `income show` will print a seller-financer's SSN in full.
6. **`interview_state` / the panel** — R12's rendering of `AnswerStatus::WordingChanged` as *blocking*
   (class A) / *forgoing* (class B) with `WORDING_CHANGED_REASON` is **T3**. T1 ships the primitive and the
   refusal; the panel consumes them.
