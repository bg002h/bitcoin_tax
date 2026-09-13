# REPORT — wave 1 C: FR-203 (publish the `income import` TOML schema) + FR-201 (the interview never asked for estimated tax payments)

Agent: wave-1 C. Worktree `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a0ddf8b5656ba7a34`,
`CARGO_TARGET_DIR=<worktree>/target-c`. **Nothing committed, nothing pushed.** No subagents.
`return_refuse.rs` and `advisories.rs` are **untouched** (verified against the changed-file list).

## Closing gate — `make gate` equivalent, after every plant was restored

```
find crates -name '*.rs' -exec touch {} +
cargo nextest run --workspace --no-fail-fast
     Summary [  30.910s] 3686 tests run: 3686 passed, 12 skipped

CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 4.00s
clippy-exit=0

cargo fmt --all --check
fmt-exit=0
```

---

## FR-203 — `docs/income-import-schema.md`, GENERATED

`cargo run -p xtask -- toml-schema > docs/income-import-schema.md` — new module
`crates/xtask/src/toml_schema.rs` (960 lines incl. tests), new subcommand row + dispatch arm in
`crates/xtask/src/main.rs`, new CI drift-gate step in `.github/workflows/ci.yml` beside the existing
`examples.md` one.

**The derivation.** The key set is `toml::Value::try_from(&ReturnInputs)` over
`btctax_core::tax::scrub_axis::maximal_sentinel()` — every `Option` `Some`, every `Vec` non-empty,
written as an exhaustive `..`-free struct literal, so a field added anywhere under `ReturnInputs` is a
**compile error in `scrub_axis.rs`** before it can be an unpublished key. That is the same serde shape
`parse_return_inputs_toml`'s `serde_ignored` guard enforces, so the document and the guard cannot
disagree about what a key is.

The document publishes, all machine-derived (measured, not typed):

| quantity | value |
|---|---|
| paths published | **365** (330 of them leaves that take a value) |
| keys the PARSER requires | **41**, each found by deleting it from a complete file and re-deserializing |
| `date` leaves | **14**, each found by substituting an ISO date string and re-parsing |
| worked example | `btctax_cli::testonly::J10_FULLRETURN_TOML`, verbatim, asserted byte-for-byte |

Plus the three traps the controller's evidence named (bare key after a `[[table]]`; census-vs-rows
contradiction; unknown keys refused by name), and the two key groups `income import` **normalises away**
rather than refusing (`answer_log*` discarded with a note; every `*_provenance` forced to `user`) — both
annotated per row, derived from the path rather than listed.

### Four things the derivation found that a hand-written document would have shipped wrong

1. **`claiming_mortgage_interest_credit` is a RETURN-LEVEL key, not `schedule_a.…`.** I wrote the
   `schedule_a.` prefix from the field's doc comment. The derived inventory says otherwise
   (`return_inputs.rs:2667` is inside `pub struct ReturnInputs`). This is also *why* the controller's
   trap-1 reproduction happens: written after `[[form_1098]]`, TOML makes it `form_1098.0.…`.
2. **`form_1098[].box1_interest`, not `box1_mortgage_interest`** — same failure mode, same test caught it.
3. **`filing_status` is NOT optional.** The first draft's prose said *"every key is optional to the
   parser: `ReturnInputs` carries `#[serde(default)]` on every field"* — **false**, and a schema that
   tells a filer a required key is optional sends them to a parse error with no way to see why. Replaced
   with the derived 41-key list.
4. **Dates: both spellings parse, and my "trap" framing was backwards.** I wrote that a quoted
   `"2012-04-15"` is *refused* and only `time`'s compact `[2012, 106]` is accepted — because that is what
   SERIALIZATION emits (`toml`'s serializer is not human-readable; its deserializer is). The kill-test
   reported the opposite: **both spellings deserialize.** The document now tells the filer to write
   `"2012-04-15"` and explains that `income scrub`'s `[year, ordinal]` output also reads back.

Each of those four is now held by a test rather than by care. In particular
`every_key_named_in_the_prose_exists_in_the_derived_inventory` checks the **hand-written prose** against
the derivation, which is the half that was wrong twice.

### A fifth: the maximal fixture is deliberately NOT maximal in one place, and it cost two keys

`scrub_axis.rs:620` sets `broker_reporting: Default::default(), // nothing answered (spec 1099-DA T1)` —
correct for the scrub axis, wrong for a key inventory. The first generation therefore published
`broker_reporting` as a bare table and **silently omitted `broker_reporting.<provider>.covered` and
`.noncovered`** — two keys `income import` honours, that `income import --help` devotes a paragraph to,
and that a TY2026 return refuses without.

Fixed in the generator (`augmented_sentinel`, which leaves `scrub_axis` alone — changing the maximal
sentinel would move the derived scrub axis, a different instrument's business), and the *class* is now
loud rather than fixed one instance at a time: **`assert_no_empty_containers` fails the generation on any
empty table or array anywhere in the emitted value, naming the path.** An empty container instantiates no
child, so every key beneath it goes unpublished while the document still looks complete — the worst
possible failure for a schema, and exactly the blind spot `scrub_axis`'s own module header is about.

### And a sixth, found by adding the count check: the document disagreed with itself

The header announced `inv.len()` while the table skipped each date's `[]` element row, so the first
version said **"379 paths" above a table of 365 rows**. Both now come off one `rows` vector, and
`the_announced_path_count_is_the_number_of_rows_printed` reds if they are ever computed from two places.

### B1 — kills observed RED

**(a) The staleness gate.** Appended one fake row to the committed document:

```
$ printf '\n| `payments.a_key_nobody_added` | string |  |\n' >> docs/income-import-schema.md
$ cargo nextest run -p xtask -E 'test(the_committed_schema_matches_a_fresh_generation)'
        FAIL [   0.027s] (1/1) xtask::bin/xtask toml_schema::tests::the_committed_schema_matches_a_fresh_generation
    thread '…' panicked at crates/xtask/src/toml_schema.rs:670:9:
    assertion `left == right` failed: docs/income-import-schema.md is STALE; regenerate with `cargo run -p xtask -- toml-schema > docs/income-import-schema.md`
     Summary [   0.028s] 1 test run: 0 passed, 1 failed, 244 skipped
```

Then regenerated — green, and byte-identical to the pre-plant copy:

```
$ cargo run -q -p xtask -- toml-schema > docs/income-import-schema.md
$ cargo nextest run -p xtask -E 'test(the_committed_schema_matches_a_fresh_generation)'
        PASS [   0.033s] (1/1) xtask::bin/xtask toml_schema::tests::the_committed_schema_matches_a_fresh_generation
     Summary [   0.034s] 1 test run: 1 passed, 244 skipped
$ diff -q docs/income-import-schema.md <pre-plant copy>
regenerated == original (the edit was the only difference)
```

**(b) The prose-key checker**, planted with the exact wrong path I originally wrote:

```
$ cargo nextest run -p xtask -E 'test(every_key_named_in_the_prose_exists_in_the_derived_inventory)'
        FAIL [   0.025s] (1/1) xtask::bin/xtask toml_schema::tests::every_key_named_in_the_prose_exists_in_the_derived_inventory
    thread '…' panicked at crates/xtask/src/toml_schema.rs:876:9:
    the generated document's PROSE names 1 key(s) the derived inventory does not publish: [
        "schedule_a.claiming_mortgage_interest_credit",
    ★ Either the key was renamed, or the prose was written from a doc comment instead of from the derivation …
     Summary [   0.026s] 1 test run: 0 passed, 1 failed, 244 skipped
```

**(c) The count checker** (header computed from a set the table does not print):

```
$ cargo nextest run -p xtask -E 'test(the_announced_path_count_is_the_number_of_rows_printed)'
        FAIL [   0.024s] (1/1) xtask::bin/xtask toml_schema::tests::the_announced_path_count_is_the_number_of_rows_printed
    assertion `left == right` failed: the document announces 379 paths and prints 365 rows — one of the two counts is computed from something other than what is published
     Summary [   0.025s] 1 test run: 0 passed, 1 failed, 245 skipped
```

All restored; the 7 `toml_schema` tests are green in the closing gate.

### What the document does NOT cover, stated in it and in the module header

- **semantics** (what a figure means — the field's doc comment and the worked example do that);
- **completeness** (`screen_inputs` decides that, refuses with the form's own words, and reports at a
  different moment — the document points at it rather than duplicating it);
- **two open-keyed maps** (`broker_reporting`, `answer_log`) whose child keys are filer data: published
  with a `<placeholder>` segment, each declaration asserted against the emitted value so a rename reds;
- the **ISO-date probe's own blind spot** (paths under a `<placeholder>` segment, and nested
  arrays-of-arrays): both are under `answer_log*`, which the import discards, **and the generator panics
  if such a leaf ever appears outside that subtree**.

---

## FR-201 — the interview now asks for the payments no document reports

The controller's narrowing was correct in every particular: the field exists, is routed, is printed, and
is reachable from the input form. **The 46-prompt `income answer` interview asked for none of it.** The
live journey is not hypothetical: `open_next_year::seed` deliberately carries *"every single dollar
except the carryforwards"* forward as blank, so a filer who files year N with btctax and opens year N+1
walks the whole interview on a return whose payment boxes are all zero.

### What was built

`crates/btctax-core/src/tax/questions.rs` — a third registry beside `FORM_QUESTIONS` and
`SKIPPABLE_QUESTIONS`:

- `MoneyId` (`EstimatedTaxPayments`, `ExtensionPayment`, `OtherWithholding`) with `ALL` pinned by an
  exhaustive `match` in a test, mirroring `SkippableId::ALL`;
- `MoneyQuestion { id, prompt, help, line, live, get, set }` and `MONEY_QUESTIONS`;
- **`payments_are_all_accounted_for(&Payments)` — a `..`-free destructure**, so a fourth payment leaf is
  an **E0027 in the file that decides what the interview asks**, not a figure the interview silently
  stops collecting. Same shape as `classifier::classify_payments`.

`crates/btctax-cli/src/cmd/answer.rs` — a fourth `Ask::Money(&'static MoneyQuestion)` variant, appended
to `live_questions_with` **derived from the registry** with each entry's own `live`, asked last, with the
current figure shown and a bare Enter keeping it. A negative is refused by `parse_nonneg_usd_arg` (the
same guard the CLI's basis/FMV flags use) and the prompt is put again.

`crates/btctax-cli/src/cli.rs` — `income answer`'s clap doc amended (its old text promised *only*
fail-loud questions), and `income import`'s doc now names `docs/income-import-schema.md`. Both man pages
regenerated.

### ALL THREE, not just FR-201's one — by derivation

`Payments` has exactly three leaves; all three are the same class (a payment the filer made that **no
document in btctax's census reports**, whose omission overstates the tax by its own amount), and the
interview asked for none of them. Hand-picking one of three is the defect shape `CLAUDE.md`'s
highest-yield rule is about, so the destructure accounts for all three and the registry asks all three.
This is a **superset of FR-201 as filed**, delivered by derivation rather than by widening scope by hand
— flagged explicitly because it is a scope call.

The extension payment already had half a fix: `render.rs:5697` notices an extension filed with no
`payments.extension_payment` and names the surfaces that can record it — *"`income import`'s
`payments.extension_payment`, the TUI input form's Payments section"*. The interview was absent from that
list because the interview could not do it.

### The design changed mid-build, because two tests refuted the first version

My first version gave money asks **no** `AnswerRecord`, reasoning that a fourth `AnswerKey` variant was
too wide a change for a Minor, and documented that as a stated limit. **Two existing tests reported it as
a regression, not a limit:** with no record, `needs_asking` can never skip these three, so `income
answer` asked them on *every* session and FR-109's *"Nothing to ask"* sentence — a deliberately built
property with its own KAT — became unreachable.

I then measured the blast radius instead of assuming it: **`AnswerKey::Money(MoneyId)` cost four small
arms.** `provenance.rs`'s `Display`, `FromStr` and `current_prompt`; `scrub.rs`'s rekey already has an
`other =>` catch-all; `input-form/apply.rs`'s `prompt_for_record` already has a `_ =>`; and every other
`AnswerKey` site in the workspace — `interview_state.rs` (21 uses), both TUI crates, the input-form spec
— **constructs** rather than matches. **`return_refuse.rs` only constructs `AnswerKey` too, so the
forbidden file is not reached.**

And it buys the thing this repo actually cares about. The state follows the value, exactly as
`skippable_state`'s does: a figure entered is `Given`, a bare Enter over `0` is **`Declined` — *asked,
and no payment claimed***. So a `0` on Form 1040 line 26 now carries **provenance**, which is not the
same blank as *nobody asked*. Measured: the `income answer` answer_log went **38 → 41 records**, all
three `Declined` on a fixture that presses Enter.

**Still open, stated in the source:** the other two writers of these leaves (`income import` and the
input form) record nothing, because `record_answer` writes only when btctax itself put the question. An
imported `0` is still an unprovenanced blank; `needs_asking` reads it as `NeverAsked` and the interview
asks once, which converges — the same shape FR-109 already documents for an imported declaration.

### B1 — the kill observed RED

Planted the question's absence (deleted the `asks.extend(MONEY_QUESTIONS…)` block in
`live_questions_with`):

```
$ cargo nextest run -p btctax-cli -E 'test(the_interview_asks_for_every_payment_figure) or test(a_typed_estimated_tax_payment_reaches_the_stored_return) or test(a_negative_payment_is_refused_and_the_prompt_is_put_again)'
      left: 0
     right: 8000
     Summary [   0.154s] 3 tests run: 0 passed, 3 failed, 843 skipped
        FAIL [   0.004s] (1/3) btctax-cli cmd::answer::tests::the_interview_asks_for_every_payment_figure
        FAIL [   0.152s] (2/3) btctax-cli cmd::answer::tests::a_negative_payment_is_refused_and_the_prompt_is_put_again
        FAIL [   0.152s] (3/3) btctax-cli cmd::answer::tests::a_typed_estimated_tax_payment_reaches_the_stored_return
```

`left: 0, right: 8000` is literally the defect: the filer paid $8,000 of estimates and the stored return
carries zero. Restored:

```
$ cargo nextest run -p btctax-cli -E '<the same three>'
        PASS [   0.006s] (1/3) btctax-cli cmd::answer::tests::the_interview_asks_for_every_payment_figure
        PASS [   0.238s] (2/3) btctax-cli cmd::answer::tests::a_negative_payment_is_refused_and_the_prompt_is_put_again
        PASS [   0.247s] (3/3) btctax-cli cmd::answer::tests::a_typed_estimated_tax_payment_reaches_the_stored_return
     Summary [   0.248s] 3 tests run: 3 passed, 843 skipped
```

The second of those three drives `answer_return_inputs` itself with a scripted keyboard, types `8000` at
the line-26 prompt, and reads the value back out of the vault — so it also reds on a keystroke landing on
the wrong leaf, or a value parsed and dropped. The third types `-8000` *then* `8000` at the same prompt,
so the run only completes if the negative was refused and the prompt re-asked.

### Two latent defects the compiler blast radius exposed in EXISTING tests

Adding the `Ask` variant reded six exhaustive matches across five files (the free, exact blast radius the
doctrine wants). Two of them were not mere mechanical widenings:

1. **`year_gate_t4.rs` built its keystroke script as a COUNT PER CLASS plus an ORDER ASSUMPTION** —
   `"n\n".repeat(declarations) + "\n".repeat(rest)`, with the comment *"a bare Enter passes over each
   skippable, which `live_questions` appends after the declarations"*. A third shape appended **after**
   the skippables broke it silently: money counted as `!is_skippable()`, so the script sent `n` at a
   dollar prompt, the prompt refused it and re-asked, and the run died on *"input ended before every
   question was answered"*. Replaced with `keystrokes_for(&asks)` — one keystroke per ask, positionally,
   over an exhaustive `match`, so a fifth shape is a compile error rather than a mis-aligned script.
2. **`only_the_skippables_are_skippable` asserted a bijection that was never true of the type** —
   `is_skippable() == declaration_id().is_none()` — only of the fixture, which carries no dependent row.
   A dependent gate has always been neither a declaration nor skippable. Restated per variant over an
   exhaustive `match`.

`tax_report.rs`'s answer-record tripwire (`38`) also fired exactly as designed; updated to `41` with the
reason appended to its own history comment.

---

## Files

**New:** `crates/xtask/src/toml_schema.rs`, `docs/income-import-schema.md`.

**Modified:** `.github/workflows/ci.yml`, `crates/btctax-cli/src/cli.rs`,
`crates/btctax-cli/src/cmd/answer.rs`, `crates/btctax-cli/tests/open_next_year_t4b.rs`,
`crates/btctax-cli/tests/step0_panel.rs`, `crates/btctax-cli/tests/tax_report.rs`,
`crates/btctax-cli/tests/year_gate_t4.rs`, `crates/btctax-core/src/tax/provenance.rs`,
`crates/btctax-core/src/tax/questions.rs`, `crates/xtask/src/main.rs`,
`docs/man/btctax-income-answer.1`, `docs/man/btctax-income-import.1`.

**Tests added (12):** 7 in `xtask::toml_schema::tests`, 3 in `btctax-cli cmd::answer::tests`
(FR-201), 2 in `btctax-core tax::questions::tests` (`every_money_id_is_in_all_…`,
`every_payments_leaf_is_asked_and_each_accessor_reaches_its_own_leaf`).

---

## Follow-ups for the controller to file (NOT done here — out of my ownership)

1. **The refusal messages still do not name the schema document.** Every *"use `income import`
   instead"* remedy lives in `return_refuse.rs` / `advisories.rs`, which I was told not to edit. The
   document now exists and `income import --help` points at it; the refusals should too. **This is the
   last mile of FR-203** — without it a filer meeting a wall still has to find the doc themselves.
2. **`income import` and the input form write the three payment leaves with no `AnswerRecord`**, so an
   imported or editor-entered `0` remains an unprovenanced blank. It converges via one interview pass;
   worth closing properly with the answered-ness work.
3. **`income answer` cannot CREATE a return** (*"Requires an existing return for the year"*), so the
   "interview-only filer" of FR-201 is reached through `open_next_year` or the TUI form, never through
   `income answer` alone. Not a defect — but the phrase is imprecise, and FR-203's *"no documented way in
   at all"* is now answered for the import path only.
4. **Dates in the import TOML.** `income scrub` emits `[year, ordinal]` while a filer will type
   `"YYYY-MM-DD"`; both work, but the asymmetry is surprising and is now documented rather than fixed.
   A follow-up could make serialization human-readable so a scrubbed file reads the way a filer writes.
