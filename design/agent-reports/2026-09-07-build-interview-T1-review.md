# Review — the ONE seam review of interview build T1 (the provenance schema)

Independent adversarial build reviewer, own worktree, read-only (every plant below was reverted;
`git status --porcelain` is empty at the time of writing and `git log --oneline -1` is `b8d9bd6a`).
Build under review `3142775d`; contract `design/SPEC_interview.md` r2 (R10, R12, §5.6, §7 row T1, §8)
plus the fold report's T1 rows (I9, I10). The implementer's account
(`design/agent-reports/2026-09-07-build-interview-T1-implementation.md`) was read and treated as claims.

**Worktree note.** The worktree handed to me was 229 commits stale (at `2bd04d45`); I fast-forwarded it
to `b8d9bd6a` before starting. Nothing else about the tree was changed.

## Commands run

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo build   --locked --workspace --all-targets                       ok
cargo nextest run --locked -p btctax-core -p btctax-input-form         1296 passed, 0 failed   (= 1230 + 66, matches the report)
cargo nextest run --locked -p btctax-core -E 'test(provenance)'        26 tests
cargo nextest run --locked -p btctax-core -E 'test(answer) or test(prompt) or test(wording)'
cargo nextest run --locked -p btctax-input-form                        66 tests
cargo nextest run --locked -p btctax-cli -E 'test(the_editor_and_income_answer)'
cargo nextest run --locked -p btctax-cli -E 'test(version_2) or test(stale)'
cargo nextest run --locked -p btctax-cli -E 'binary(fullreturn_oracle) or test(examples)'
cargo nextest run --locked -p xtask      -E 'test(examples)'           13 passed
cargo test -p btctax-cli --test fullreturn_oracle -- --ignored emit_fullreturn_fixture   (×2, for idempotency)
```

The workspace suite was NOT re-run (3197 at `3142775d`, machine-verified by the gate, per the brief).

## Summary

The schema itself is good work, and most of what the report claims is true when you go and check it.
Nine of the ten seams hold. The one that does not is the one the report never looked at: **`record_answer`
is not the one writer.** `income import` is a second, unguarded writer of `answer_log` /
`answer_log_history`, and I reproduced both directions — it mints a forged record, and it silently
destroys fourteen genuine ones while the answered *value* survives.

Secondary: `supersede_stale_prompts` — half two of R10.3's mismatch rule — has **zero production callers**,
so the superseded record never reaches history on any real path; the next `record_answer` overwrites it.

---

## Per seam

### Seam 1 — `LEAF_SOURCE` ↔ the coverage KAT. **HOLDS (stronger than documented).**

Planted a new `pub planted_money_leaf: Usd` on `ReturnInputs` with no `LEAF_SOURCE` row. The compiler
bit first (E0027 ×3 in `classifier.rs` / `return_refuse.rs` / `scrub.rs`, E0063 in `scrub_axis.rs`);
after satisfying all four with `_` bindings, the KAT redded:

```
left: Audit { unsourced: ["planted_money_leaf"], unmatched: [], multi: [] }
```

Then changed it to `Option<Usd>` left `None` on `maximal_sentinel` — the case the doc comment calls the
instrument's blind spot. **It is not blind: it redded anyway**, *and* `classifier::tests::
no_option_money_leaf_is_bound_with_underscore` redded alongside it. Both nets fire. (See M4 — the
doc comment's stated limit is wrong, which matters because a wrong stated limit misdirects.)

Both built-in negative tests (stale prefix, double-claimed leaf) pass and are real.

`EXEMPT_PREFIXES` (the input-form coverage census) and `LEAF_SOURCE` are orthogonal lists and neither
hides the other: `int_1099` / `div_1099` / `g_1099` / `b_1099` / `schedule_1a` are exempt from *coverage*
(no `Field` yet) while `LEAF_SOURCE` still claims every money leaf inside them — the KAT's `unsourced`
count is zero over all 106. The two new `answer_log` / `answer_log_history` exemptions carry reasons and
are pinned by the fold's kill #13, which I did not need to re-plant because the fixture change that makes
them live (`coverage.rs:148,168`) is what would break first.

Measured, not taken from the report: **106 money leaves** (planted a `panic!` on `money.len()`). The
report's figure is exact.

### Seam 2 — one writer. **FAILS. See C1.**

The identity kill (`tax_report::the_editor_and_income_answer_write_the_same_answer_record`) is genuine and
compares the two records serialized. Both surfaces derive `now` identically —
`conventions::tax_date(now, UtcOffset::UTC)` is literally `utc.to_offset(tz).date()`
(`conventions.rs:67-69`), the same expression `main.rs:830` uses for the TUI — so there is no clock drift
between them.

I then went looking for a third path. `(q.set)` / `(sk.set_*)` / `field.set` call sites outside `apply`
and `income answer` are all `#[cfg(test)]` (`testonly::answer_all_live_declarations` and the no-brick
test). But `income import` deserializes the **whole** `ReturnInputs`, `answer_log` included, and writes
it. Reproduced — see C1.

### Seam 3 — `prompt_hash`. **HOLDS as far as it goes; half two is dead. See I1.**

Planted the refusal away (deleted the `answer_status == WordingChanged` check at `return_refuse.rs:1158`):

```
FAIL btctax-core tax::return_refuse::tests::an_answer_hashed_against_earlier_words_refuses_and_a_missing_record_does_not
```

The "an absent record is not a mismatch" half is deliberate, correct for a corpus with no records, and
pinned by the same test. The exit from the refusal exists and works: `live_questions` yields every *live*
declaration regardless of whether it holds a value, so `income answer` re-asks a `WordingChanged`
question and a bare Enter re-stamps it against the new words. Not a brick.

What does **not** happen is the other half of R10.3 — the superseded record reaching
`answer_log_history`. See I1.

### Seam 4 — `Declined`. **HOLDS structurally; no production reader yet.**

`AnswerState` is a two-variant enum with **no** `Default` derive (`provenance.rs:283-286`), so there is no
defaulted answer to launder; `AnswerStatus` is four-valued and `answer_status` matches it exhaustively
with no wildcard arm (`provenance.rs:393-402`). Grepped every `AnswerState::` / `AnswerStatus::` mention
outside the module: the only production reader is `return_refuse.rs:1158`, and it is an `==` against
`WordingChanged`, not a match — so nothing can silently collapse `Declined` into `None`.

Planted `skippable_state` to always return `Given`:

```
assertion `left == right` failed: a skippable the filer passed over is a DECLINED record — asked, and refused
  left: Some(Given)  right: Some(Declined)
```

Caveat recorded as N3: today `Declined` is *written* and read by nothing in production (R12's panel is
T3), which is the same "stored value with no reader" shape `provenance.rs` itself warns about for
`prompt_hash`. Expected at T1; it stops being acceptable if T3 slips.

### Seam 5 — dependent identity. **HOLDS.**

Planted both seam calls away (`retire_dependent_identity` from the Dependents `remove`,
`supersede_dependent_identity` from the `DepSsn` setter):

```
FAIL btctax-input-form spec::sections::broker_block_tests::removing_a_dependent_row_takes_only_that_identitys_answers_and_a_new_ssn_starts_fresh
```

Doing it in the seam's own closures rather than in `apply` is the right call and is the reason a test that
drives `Field::set` directly is covered. The blank-`ssn` bucket collapse is documented at
`retire_dependent_identity` and correctly deferred to T7.

### Seam 6 — `SCHEMA_VERSION` 3. **HOLDS.**

Planted a *silent upgrade* rather than a version change — `version != SCHEMA_VERSION && version != 2` in
both `row_to_inputs` and `input_form_store::load`. Three tests redded:

```
FAIL btctax-cli return_inputs::p9_stale_row_refuses::a_version_2_row_refuses_stale
FAIL btctax-cli input_form_store::tests::a_version_2_wip_draft_is_discarded_with_a_note_and_a_version_2_parked_draft_refuses
FAIL btctax-cli::slice_from_answers the_stale_draft_split_holds_on_the_export_path
```

`row_to_inputs` is the one read boundary and `all()` shares it (`all_refuses_a_stale_row_identically_to_get`);
grepped every `inputs_json` reader and found no path around it. Nothing silently upgrades a v2 row.

### Seam 7 — generated fixtures and goldens. **HOLDS except the examples fixture. See M2.**

`git diff --stat 976a0b63 3142775d -- docs/man/ crates/xtask/` is empty: the report's "man pages: NO
change" and "census registers: NOT moved" are both true. `xtask::examples::tests::
examples_golden_matches_committed` and its twelve siblings pass. `fullreturn_oracle::
fullreturn_fixture_is_the_kitchen_sink_oracle` passes both with the committed fixture and with the
regenerated one — i.e. it cannot detect the drift. See M2.

### Seam 8 — the kills. **Five of five re-verified red.** (See the per-seam sections above.)

I planted, ran and reverted: the one-writer/`Declined` identity, the prompt-hash rule, the `Declined`
third value, the dependent identity key, and the v2 refusal — plus the `LEAF_SOURCE` completeness kill in
two forms. Every one redded with a message that names the guarantee. I did not re-plant kills 1–4, 13, 14
and 15; the report's quoted red text for those is consistent with the code I read.

### Seam 9 — `payer_tin: ""` / `transcribed_on: null`. **Not testimony. But see M1.**

`""` is not testimony by omission: no printed line on any form reads a 1099 payer TIN, the classifier
records it as a scalar `String` under the `_` rule with a reason, the four 1099 vectors are already
`EXEMPT_PREFIXES` in the coverage census (so no `Field` is owed), and the scrub's `map_payer_tin`
deliberately preserves emptiness *"so the scrubbed copy cannot differ from the original in whether an
emptiness check fires"* — which is the reader-consistency the question asks for, written before there is
a reader. The `income show` JSON prints it beside `"ein": null`, and nothing acts on either.

What is wrong is smaller and is M1: the type cannot express *"the document prints no TIN"*, while its own
sibling on the same struct (`transcribed_on: Option<Date>`) and the same-class W-2 identifier
(`ein: Option<String>`) both can.

### Seam 10 — D9, the generated examples fixture. **Reproduced; the report's diagnosis is wrong. See M2.**

Ran the header's command twice and diffed. **The emitter IS idempotent** — run 2 is byte-identical to
run 1. What is stale is the *committed file*, and by more than the report says: 15 insertions / 10
deletions, of which T1's share is `answer_log_history = []`, `[answer_log]` and `payer_tin = ""` ×3, and
the rest is **five keys added to the schema before T1** —

```
+answer_log_history = []          +b_1099 = []            +[broker_reporting]
+[schedule_1a] vehicles = []      +schedule_a.investment_interest = "0"
+schedule_c.other_gross_receipts = "0"      (plus filing_form_4952 reordered)
```

— and the deletion of the three hand-added `DELETING THIS LINE REFUSES THE RETURN` comment blocks.

---

## Findings

### C1 (Critical) — `record_answer` is not the one writer: `income import` both FORGES answer records and SILENTLY DESTROYS them

**Where.** `crates/btctax-cli/src/cmd/tax.rs:73` (`parse_return_inputs_toml` deserializes the whole
`ReturnInputs`), `:102-111` (the `CarryProvenance` normalisation block), `:121` (the preservation block)
— against `crates/btctax-core/src/tax/provenance.rs:347` (*"THE ONE WRITER of `answer_log`"*) and
`crates/btctax-core/src/tax/return_inputs.rs:1311-1338`.

**What is wrong.** `answer_log` and `answer_log_history` are `#[serde(default)]` fields on
`ReturnInputs`, and `income import` is a whole-blob upsert with no guard on either. Two consequences,
both reproduced:

- **(a) Forgery.** A hand-written TOML can mint an `AnswerRecord` btctax never observed — including one
  whose `prompt_hash` matches the current registry wording, which then reads as `AnswerStatus::Given` and
  satisfies R10.3's mismatch rule. This is the *exact* class the same function was hardened against
  twenty-five lines earlier, in a comment that states the principle: *"`Computed` IS BTCTAX'S SIGNATURE,
  AND THE IMPORT SURFACE MUST NOT BE ABLE TO SIGN IT."* The answer log is btctax's signature about the
  **asking** — when the filer was asked, and in what words — and the import surface can sign it.
- **(b) Silent destruction, which re-opens the laundering path.** A routine re-import that does not carry
  `[answer_log]` wipes every record. The answered *values* survive (they are in the TOML), so the return
  keeps standing as testimony while every trace of when and under which words it was answered is gone —
  and because *"an absent record is not a mismatch"* (`return_refuse.rs:1155-1157`, deliberately),
  `screen_inputs`' wording-change refusal is thereafter permanently disarmed for that return. The same
  function goes to the trouble of preserving `Computed` carryovers across this upsert **and printing a
  note naming what it preserved**; the diligence file is destroyed with no note at all.

**Evidence** (a temporary test appended to `crates/btctax-cli/tests/tax_report.rs`, run, then reverted):

```
PROBE-A forged record present = Some(AnswerRecord { answered_on: 1999-01-01,
    prompt_hash: "d3808fb20dbde50c0f8218755531cccfffd9e13a60716ba8c5e172481cbcc43a", state: Given })
PROBE RESULT: forged_answered_on=1999-01-01 ; before_reimport=14 ; after_reimport=0 ;
    value_survived=Some(false)
```

`14 → 0` are genuine records written by `income answer` at `BTCTAX_NOW = 2026-09-01`, destroyed by an
import of a six-line TOML. The forged record carries a 1999 date and a *valid* hash of the live
`ForeignTrust` prompt.

**Minimal change.** One block in `import_return_inputs`, beside the `CarryProvenance` block that already
does exactly this for the other unforgeable stamp: discard whatever the TOML says about `answer_log` /
`answer_log_history`, then re-attach the stored row's (empty on a fresh year). That closes (a) and (b)
together. It must be `answer_log_history` too, or history becomes the forging surface instead. Note the
interaction with `scrub_refusal::the_scrubbed_toml_round_trips_back_through_import`: the scrubbed TOML
carries a re-keyed log, so that test's comparison needs the same normalisation applied to both sides (the
same shape the `CarryProvenance` block's FR-18 note already describes). The alternative — refusing an
import that carries `[answer_log]` — would make btctax unable to read a file it emits, which that comment
explicitly rules out.

### I1 (Important) — `supersede_stale_prompts` has NO production caller, so R10.3's "the old answer is kept as history" is unmet on every real path

**Where.** `crates/btctax-core/src/tax/provenance.rs:433`; `record_answer` at `:347`.

**What is wrong.** R10.3 is explicit and has two halves: *"**The old answer is kept as history and never
as the current answer:** the superseded `AnswerRecord` moves to `answer_log_history`, append-only, which
nothing reads as an answer, and the re-answer writes a fresh record."* Half one is delivered by
`answer_status` + the `screen_inputs` refusal. Half two is delivered by `supersede_stale_prompts` — which
is called from nowhere but its own tests.

The consequence is not cosmetic. When the filer takes the exit and re-answers, `record_answer`'s
`ri.answer_log.insert(key, …)` **overwrites** the stale record. It is not moved to history; it is
destroyed. So on the one path R11's Sep–Dec calendar makes ordinary — a prompt edited in a November fold
under an answer given in September — `answer_log_history` never receives a prompt-superseded record at
all, and the diligence file loses the fact that an earlier answer was ever given. (`answer_log_history`
*is* written in production, but only by `supersede_dependent_identity` via `sections.rs:548` — the
changed-`ssn` case.) The T1 kill passes because it calls `supersede_stale_prompts` directly; that is the
"green and blind instrument" shape — a function fully tested and never run.

**Evidence.**

```
$ grep -rn 'supersede_stale_prompts' --include=*.rs .
crates/btctax-core/src/tax/provenance.rs:408:  (doc comment)
crates/btctax-core/src/tax/provenance.rs:433:  pub fn supersede_stale_prompts(...)
crates/btctax-core/src/tax/provenance.rs:878,891,897:  (its own tests)
```

**Minimal change.** Either (i) call the sweep at the one read boundary — `return_inputs::row_to_inputs`
already stamps `tax_year` there and is documented as *"the ONE read boundary"*, and the sweep is
idempotent by construction so a per-load call cannot grow the history — or (ii) make `record_answer`
itself historise a record it is about to replace whose `prompt_hash` differs from the one it is writing,
which localises the guarantee to the one writer. Either way the kill needs a second half asserting the
history entry appears **without** a direct call to the sweep, or the same blindness returns.

### M1 (Minor) — `payer_tin: String` cannot express "the document prints no TIN"; its two siblings can

**Where.** `crates/btctax-core/src/tax/return_inputs.rs:90, 116, 149, 183` vs `:94, 119, 152, 186`
(`transcribed_on: Option<Date>`) and `:58` (`W2::ein: Option<String>`).

**What is wrong.** Three identifiers of one class on the same JSON output, two absence encodings. The doc
comment resolves the ambiguity by decree — *"empty means 'not transcribed', never 'the payer has none'"* —
which is a promise the type does not keep; it declares one of the two readings illegal rather than making
it unrepresentable. It is harmless today because nothing reads a validity class off it (correctly recorded
as `NO_READER` **with the reason** in the §3.3 scrub matrix), and R10.2 does specify `String`, so the build
followed the spec. It becomes a decision the moment T5's document screen adds an emptiness check.

**Minimal change.** None now; carry it to T5 alongside the existing `w2s[].transcribed_on` (D3) item, and
decide `Option<String>` there with the screen that would read it.

### M2 (Minor) — a `GENERATED — do not hand-edit` fixture with a documented regeneration command and no test that anything matches

**Where.** `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml:1-6`.

**What is wrong.** The header asserts a property nothing checks, and the property is false. Its sibling
generated artefact (`docs/examples/examples.md`) has exactly the missing test —
`xtask::examples::tests::examples_golden_matches_committed`. This one has none, and
`fullreturn_fixture_is_the_kitchen_sink_oracle` passes identically with the committed file and with the
regenerated one, so it cannot see the drift.

The report's D9 diagnosis is wrong in a way worth correcting: the **emitter is idempotent** (two
consecutive regenerations are byte-identical); the *committed file* has drifted, by three hand-added
comment blocks **and** five keys the schema gained before T1 (`b_1099`, `broker_reporting`,
`schedule_1a.vehicles`, `schedule_a.investment_interest`, `schedule_c.other_gross_receipts`). D9's
decision not to regenerate is right — regenerating destroys the three `DELETING THIS LINE REFUSES THE
RETURN` warnings — but it leaves a file that the next reader who follows the header's own instruction
will silently strip.

**Minimal change.** Move the three notes into `fullreturn_oracle.rs` beside the fixture path, regenerate,
and add the missing `#[test] fn fullreturn_fixture_matches_its_emitter()`. Or, if the file is to stay
hand-annotated, delete the `GENERATED — do not hand-edit` line, because it is currently false.

### M3 (Minor) — two doc comments were corrupted by a mechanical rewrite and now document a hardcoded date where the code has a seam

**Where.** `crates/btctax-tui-edit/src/edit/tax_inputs.rs:11` and `:262`.

**What is wrong.** The `now`-parameter rewrite hit prose:

```
//!   ... a second `Enter` commits (`parse` → `apply(SetField, time::macros::date!(2026 - 09 - 01))`); `Esc` cancels.
// Every shape edit goes through `apply(&mut form.working, Edit::…, time::macros::date!(2026 - 09 - 01))` (via `apply_edit`) ...
```

The production code passes `form.now` (`apply_edit`, `:554`), read once from the `BTCTAX_NOW` seam. Both
comments now tell the next reader the TUI stamps a fixed 2026-09-01 — the precise misreading D2 was
written to prevent.

**Minimal change.** Restore the two comments to `apply(SetField)` / `apply(&mut form.working, Edit::…, now)`.

### M4 (Minor) — the money detector's stated "honest limit" is factually wrong, and the real limit is elsewhere

**Where.** `crates/btctax-core/src/tax/provenance.rs:583-587` (the `money_leaves` doc comment) and the
report's §1.1.

**What is wrong.** The doc says a new `Option<Usd>` left `None` on `maximal_sentinel` *"serializes as
`null`, which rejects both probes and is therefore reported as not money"*. Measured: it is **detected**.
`walk` emits the `null` as a leaf and `set_at` replaces the whole value with the probe string, so the
`Option<Usd>` deserializer classifies it correctly — I planted exactly that field and the KAT redded on
it. The actual blind spots are different and unstated: a leaf that never appears in the serialized JSON
(`skip_serializing_if`, which `forms.rs:293-295` already uses elsewhere) and an empty `Vec` in the
fixture, whose element leaves are never walked.

A *wrong* stated limit is worse than none, because it is the sentence a future reader will rely on when
deciding whether a second net is needed — and here it points at the safe case while the unsafe ones go
unnamed.

**Minimal change.** Replace the paragraph with the two real limits, and keep the (correct) note that the
classifier's `no_option_money_leaf_is_bound_with_underscore` is the second net.

### N1 (Nit) — the fixture-shrink floor is `> 50` against a measured 106

`provenance.rs:645`. A fixture that lost half its realized money leaves would still pass the guard whose
message is *"it has stopped being maximal"*. The `unmatched` half of the audit catches a shrink that
empties a whole prefix, so this is partial, not absent. Pin it to the measured count, or to
`>= 100`.

### N2 (Nit) — `answer.rs:307-310`'s claim of structural enforcement overstates what the code does

The comment says the record is *"Placed here, after the `Ask` match, so no branch can be added that asks
without recording: the three skippable shapes and the declaration all pass through this line."* The
declaration does **not** pass through that line — it records inside its own branch at `:194` — and the
recording site is an `if let Ask::Skippable(sk)`, which a new `Ask` variant escapes silently (the `match`
above it is exhaustive and would red, but nothing forces the new arm to record). Same class as the
classifier's own carefully-stated `_` limit, minus the honesty.

### N3 (Nit) — `AnswerState::Declined` is written and read by nothing in production

`income answer` writes it (`answer.rs:313`); the only production reader of the answer types is
`return_refuse.rs:1158`, an `==` against `WordingChanged`, which treats `Declined`, `Given` and
`NeverAsked` alike. R12's panel is T3 and T1's contract says so, so this is expected — recorded only
because *"a stored value with no reader is not a guarantee"* is the rule `provenance.rs:290-292` invokes
for `prompt_hash`, and it applies here too until T3 lands.

---

## What I did not examine

- The full workspace suite (3197 at `3142775d`) — machine-verified by the gate per the brief; I ran only
  the scoped sets listed above (1296 in core + input-form, plus targeted `btctax-cli` / `xtask` filters).
- `cargo fmt` / `clippy` — reported clean by the implementer; not re-run.
- Kills 1–4, 13, 14 and 15 were not re-planted (I re-verified 5, 6, 7/8, 9, 10, 11, 12 and both
  `LEAF_SOURCE` directions). The code paths for 13–15 were read.
- The oracle path, the PII scrub's non-`answer_log` matrix rows, and `mask_pii`'s `payer_ssn` gap (the
  report's §6.5 item, owned by T5) — out of T1's scope.
- R12 / `interview_state` (T3), the `DEPENDENT_GATES` registry (T7), the N+1 opener (T4b): not built here,
  and I did not review their absence beyond confirming D4/D5/D6 name the owning task.
- Whether `answered_on` dates in a scrubbed export reveal filer activity — a secret-handling question,
  which under `STANDARD_WORKFLOW.md` is non-gating; not pursued.

Counts: C=1 I=1 M=4 N=3
