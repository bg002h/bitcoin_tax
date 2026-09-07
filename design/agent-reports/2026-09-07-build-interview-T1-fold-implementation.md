# Fold — the interview T1 seam review (1C / 1I / 4M / 3N)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatch HEAD
`4f99bef4`. Nothing committed or pushed; no subagents; no `git checkout --` / `git restore` on any file
I did not create (every plant below was reverted from a `cp` backup under the session scratchpad).

Source: `design/agent-reports/2026-09-07-build-interview-T1-review.md`, findings from line 194.
Contract: `design/SPEC_interview.md` R10.3 / R12.

**One deviation, in I1** — the read-boundary sweep the brief asked for was *measured to break the
guarantee it serves* and is not landed. §I1 below carries the measurement and the permanent guard that
now pins the decision. Everything else is the review's own "Minimal change".

---

## C1 (Critical) — `income import` is no longer a second writer of the answer log

### The change

| file:line | what |
|---|---|
| `crates/btctax-cli/src/cmd/tax.rs:112-137` | the normalisation block: `ri.answer_log.clear(); ri.answer_log_history.clear();`, placed immediately after the `CarryProvenance` block it is modelled on, with the "normalise, never refuse" reasoning spelled out for the same reason that block spells it out |
| `crates/btctax-cli/src/cmd/tax.rs:130-145` | **added beyond the review's minimal change:** when the file *did* carry records, the discard is **announced** (`note: ignored N answer record(s) in the file …`). The review's own complaint was that the diligence file went *"with no note at all"* while the sibling `Computed` block prints what it preserved. Only fires when `carried > 0`, so an ordinary hand-written TOML import stays silent |
| `crates/btctax-cli/src/cmd/tax.rs:170-176` | the other half — inside `if let Some(existing)`, re-attach the **stored** row's `answer_log` / `answer_log_history`. On a fresh year the block does not run, so the log stays empty, which is the truthful record of a return nobody has been asked about |
| `crates/btctax-cli/tests/scrub_refusal.rs:849-857, 884-903` | `the_scrubbed_toml_round_trips_back_through_import` normalised on **both** sides — but asserting rather than merely dropping: the landed side must be *empty* (the guarantee), the sent side must be *non-empty* (or the first assertion means nothing), then both are cleared and the emitter compared field-for-field as before. Plus the note assertion |

`answer_log` on the TOML side is still *accepted* rather than rejected: `income scrub` emits those keys,
and a refusal would make btctax unable to read a file it emits — the same rule the `Computed` block
above it states.

### The kills, each seen red once

**(a) Forgery** — `tax_report::an_imported_toml_cannot_mint_an_answer_record_or_a_history_entry`. The
TOML mints a record with a *valid* `prompt_hash` of the live `ForeignTrust` wording, `state = "given"`,
`answered_on = [1999, 1]` — the shape that reads as `AnswerStatus::Given` and satisfies R10.3's mismatch
rule on an act that never happened. It asserts the imported **value** still lands (`foreign_trust =
Some(false)`) so the test cannot pass by refusing the import.

Plant: both `clear()`s and both re-attaches commented out.

```
thread 'an_imported_toml_cannot_mint_an_answer_record_or_a_history_entry' panicked at
crates/btctax-cli/tests/tax_report.rs:3127:5:
assertion `left == right` failed: a TOML minted an AnswerRecord: `income import` has become a second
writer of the log, and a forged record with a LIVE prompt hash reads as `Given` — provenance for an
act that never happened
  left: Some(AnswerRecord { answered_on: 1999-01-01,
        prompt_hash: "d3808fb20dbde50c0f8218755531cccfffd9e13a60716ba8c5e172481cbcc43a", state: Given })
 right: None
```

Second plant, isolating the **history** half (`answer_log` normalised, `answer_log_history` not) —
because closing one alone only moves the forgery:

```
thread 'an_imported_toml_cannot_mint_an_answer_record_or_a_history_entry' panicked at
crates/btctax-cli/tests/tax_report.rs:3136:5:
a TOML minted an `answer_log_history` entry — closing `answer_log` alone only moves the forgery into
the history, which is where a superseded record is supposed to be safe:
[(Question(ForeignAccounts), AnswerRecord { answered_on: 1999-01-02,
  prompt_hash: "d3808fb20dbde50c0f8218755531cccfffd9e13a60716ba8c5e172481cbcc43a", state: Given })]
```

**(b) The fourteen survivors** — `tax_report::a_re_import_keeps_every_answer_record_already_on_the_row`.
A return, then the full interview at the keyboard, then a re-import of the same six-line TOML. The
count is asserted `== 14` (the review's measurement, machine-confirmed here) *before* the survival
assertion, so a survival test over an empty log cannot pass for the wrong reason.

```
thread 'a_re_import_keeps_every_answer_record_already_on_the_row' panicked at
crates/btctax-cli/tests/tax_report.rs:3206:5:
assertion `left == right` failed: a re-import destroyed the answer log (14 record(s) → 0): the values
survive, so the return still stands as testimony while the record of the asking is gone
  left: {}
 right: {Question(DependentTaxpayer): …, Question(ForeignAccounts): …, Question(ForeignTrust): …,
         Question(HsaActivity): …, Question(DualStatusAlien): …, Question(OtherOutOfScopeIncome): …,
         Question(FilingForm4952): …, Skippable(BlindTaxpayer): …, Skippable(DobTaxpayer): …,
         Skippable(TaxpayerDiedDuringYear): …, Skippable(DonationsHadRestrictions): …,
         Skippable(CharitableCwaObtained): …, Skippable(Form8615Condition3AgeSupport): …,
         Skippable(Form8615Condition4ParentAlive): …}
```

**(c) A third net, unplanned** — the round-trip test's new landed-side assertion reds on the same plant:

```
thread 'the_scrubbed_toml_round_trips_back_through_import' panicked at
crates/btctax-cli/tests/scrub_refusal.rs:879:5:
`income import` must not carry the file's answer log onto a fresh vault — the recipient was never
asked these questions; got 2 record(s) and 1 history entr(ies)
```

**(d) The note** — plant `if carried > 0 { eprintln!(…) }` away:

```
thread 'the_scrubbed_toml_round_trips_back_through_import' panicked at
crates/btctax-cli/tests/scrub_refusal.rs:853:5:
an import that discards the file's answer log must SAY so — the recipient otherwise cannot tell a
return nobody was asked about from one that was:
```

---

## I1 (Important) — the guarantee moved into the one writer; the read-boundary sweep is **refused**, with the measurement

### The change

| file:line | what |
|---|---|
| `crates/btctax-core/src/tax/provenance.rs:347-393` | `record_answer` now historises the record it is about to replace **when that record's `prompt_hash` differs from the one it is writing**. Re-answering under *unchanged* words is a correction, not a supersession, and does not grow history |
| `crates/btctax-core/src/tax/provenance.rs` (deleted) | `supersede_stale_prompts` is **removed**. With `record_answer` carrying the rule, the sweep was the thing the finding actually named: a `pub fn` fully tested and never run. Deleting it removes the blind instrument rather than documenting it. Grep-confirmed no other reference in `crates/`, `docs/` or the spec — only this fold's report and the review itself mention the name |
| `crates/btctax-core/src/tax/provenance.rs:408-411` | `current_prompt`'s doc no longer links the deleted function |
| `crates/btctax-core/src/tax/provenance.rs` (tests) | `changing_the_words_re_asks_and_historises_restoring_them_does_neither` → `changing_the_words_re_asks_and_the_re_answer_itself_historises_the_old_record`, rewritten so **nothing but `record_answer` is called**. New `D2` constant so a re-answer's date is distinguishable from the record it superseded |

### Why the read-boundary sweep is not landed — deviation, measured not argued

R10.3's two sentences are ordered, and the order is the design:

> *"…and `screen_inputs` refuses such a class-(A) record as UNANSWERED. **The old answer is kept as
> history and never as the current answer:** the superseded `AnswerRecord` moves to
> `answer_log_history` … **and the re-answer writes a fresh record**."*

The refusal comes **first** and needs the mismatched record still in `answer_log` at read time; R12's
table (`SPEC_interview.md:844`) lists exactly that record as **blocking**. A sweep at
`return_inputs::row_to_inputs` moves it to history *before* any reader sees it, `answer_status` then
reports `NeverAsked`, and `return_refuse.rs`'s deliberate *"an absent record is not a mismatch"* lets a
September answer stand silently under November's words with no re-ask — i.e. it disarms the refusal the
sweep exists to serve, and does so with the whole suite green.

Measured with the sweep inlined at `row_to_inputs` (temporary probe, since removed):

```
thread 'probe_read_boundary_sweep' panicked at crates/btctax-cli/tests/tax_report.rs:4214:5:
assertion `left == right` failed: PROBE: a September answer under November words must still read as
WordingChanged after `row_to_inputs` — this is what `screen_inputs` refuses on (return_refuse.rs:1158).
It is now NeverAsked, and `foreign_trust` = Some(false) still stands as testimony with NO re-ask.
  left: NeverAsked
 right: WordingChanged
```

The same probe passes without the sweep. The review offered the two options as alternatives
(*"Either (i) … or (ii) …"*); (ii) meets R10.3's half two on every real path, and (i) costs half one.
The reasoning is written into `record_answer`'s doc comment so the next reader cannot re-derive the
sweep from the review's text.

### The kills

**Second half, in the one-writer test** — `provenance::changing_the_words_re_asks_and_the_re_answer_
itself_historises_the_old_record`. The history entry appears with **no sweep called anywhere**; it also
asserts the stale record *stays in the log* until it is re-answered, which is what the refusal needs.

Plant: the historisation removed from `record_answer`.

```
thread 'tax::provenance::tests::changing_the_words_re_asks_and_the_re_answer_itself_historises_the_old_record'
panicked at crates/btctax-core/src/tax/provenance.rs:893:9:
assertion `left == right` failed: the superseded record must be KEPT — history is where it goes, and
nothing else wrote it
  left: 0   right: 1
```

Opposite plant (`if superseded.prompt_hash != hash` → `if true`), so the "unchanged wording" half is
seen discriminating too:

```
panicked at crates/btctax-core/src/tax/provenance.rs:926:9:
changing an answer under UNCHANGED wording must not grow the history — only the words changing
supersedes (R10.3), and a writer that appended every time would turn the append-only file into a
keystroke log
```

**End to end, on a real surface** — `tax_report::re_answering_at_the_keyboard_moves_the_stale_record_
into_history_by_itself`. The ordinary R11 case: a September record hashed against words no longer
asked, then `income answer` at the keyboard in November. Nothing in the test touches
`answer_log_history`, and no sweep exists to call.

```
thread 're_answering_at_the_keyboard_moves_the_stale_record_into_history_by_itself' panicked at
crates/btctax-cli/tests/tax_report.rs:3288:5:
assertion `left == right` failed: the September record was DESTROYED rather than kept: `record_answer`
overwrote it, so the diligence file lost the fact that an earlier answer was ever given — R10.3's
half two, unmet
  left: []
 right: [(2026-09-01, "7e2ad9451b470ff25836d92ee811a379e8215181ca5b34d2e9fa5380c02daa0c")]
```

**The deviation's own guard** — `tax_report::a_record_whose_words_changed_still_reads_as_wording_
changed_after_a_load`, added so the refused option cannot be re-added to a green suite. Plant: the
sweep inlined at `row_to_inputs`.

```
thread 'a_record_whose_words_changed_still_reads_as_wording_changed_after_a_load' panicked at
crates/btctax-cli/tests/tax_report.rs:3370:5:
assertion `left == right` failed: the read boundary superseded the record on load: it now reads
NeverAsked, so `screen_inputs` sees no mismatch and `foreign_trust` = Some(false) stands as testimony
under words the filer was never shown. Supersession belongs at the RE-ANSWER (`record_answer`), never
at a load.
  left: NeverAsked   right: WordingChanged
```

---

## M1 (Minor) — recorded, nothing changed

`payer_tin: String` cannot express *"the document prints no TIN"* while its siblings
(`transcribed_on: Option<Date>`, `W2::ein: Option<String>`) can. **No code was touched.** R10.2
specifies `String`, nothing reads a validity class off it today, and the decision belongs with the T5
document screen that would add the emptiness check. The controller carries it into the T5 brief's
residue alongside the existing `w2s[].transcribed_on` (D3) item.

## M2 (Minor) — the fixture now matches its emitter

| file:line | what |
|---|---|
| `crates/btctax-cli/tests/fullreturn_oracle.rs:19-42` | the fixture's three hand-added `DELETING THIS LINE REFUSES THE RETURN` notes moved beside `const FIXTURE`, each keeping its statute cite, its `RefuseReason` and its "and `false` refuses it as …" half |
| `crates/btctax-cli/tests/fullreturn_oracle.rs:69-84` | the emitter extracted from the `#[ignore]` test body into `emit_fullreturn_fixture_text()`, so a gate can run it in memory |
| `crates/btctax-cli/tests/fullreturn_oracle.rs:86-108` | **`fullreturn_fixture_matches_its_emitter`** — a byte comparison, which is the only thing that makes *"do not hand-edit"* true |
| `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` | regenerated with its own documented command |

The review's D9 correction is confirmed: the emitter **is** idempotent; the committed file had drifted.
Regeneration diff — **15 insertions / 10 deletions**, exactly as the review predicted, every line
listed:

```
+answer_log_history = []          +b_1099 = []
+[answer_log]                     +[broker_reporting]
+payer_tin = ""   (div_1099)      +payer_tin = ""   (g_1099)      +payer_tin = ""   (int_1099)
+[schedule_1a] / +vehicles = []   +schedule_a.investment_interest = "0"
+schedule_c.other_gross_receipts = "0"
 filing_form_4952 = false   — moved (alphabetical order after `excluded_puerto_rico_income`)
-  the 3-line §170(f)(8) CharitableCwaUnresolved comment block
-  the 3-line Schedule D line 20 / Form4952DeclarationUnanswered comment block
-  the 3-line §163(h)(3)(B) MortgageDebtLimitUnanswered comment block
```

**No other golden moved.** `xtask::examples::tests::examples_golden_matches_committed` and its twelve
siblings pass unchanged against `docs/examples/examples.md`, and
`btctax_tui_edit_walkthrough_goldens_match_committed` is green — the emitter's key order did not
change, only its key *set*.

**Kills.** A one-byte value edit (`medical = "2000"` → `"2001"`) reds both tests. The discriminating
case is a hand-added **comment**, which the pre-existing oracle test cannot see:

```
PASS  fullreturn_oracle::fullreturn_fixture_is_the_kitchen_sink_oracle
FAIL  fullreturn_oracle::fullreturn_fixture_matches_its_emitter
  assertion `left == right` failed: the committed fullreturn_inputs.toml is not what its emitter
  produces — either it was hand-edited (the banner forbids it; put the annotation beside `FIXTURE` in
  this file instead) or the schema moved under it. Regenerate with `cargo test -p btctax-cli --test
  fullreturn_oracle -- --ignored emit_fullreturn_fixture`.
```

The fixture was restored from a `cp` backup after each plant (never `git checkout --`), and the
restore was verified with `cmp`.

## M3 (Minor) — the two corrupted doc comments restored

`crates/btctax-tui-edit/src/edit/tax_inputs.rs:11` → `apply(SetField)`;
`:262` → `apply(&mut form.working, Edit::…, now)`. The `now`-parameter rewrite had put
`time::macros::date!(2026 - 09 - 01)` into prose, telling the next reader the TUI stamps a fixed date
where the code passes `form.now` from the `BTCTAX_NOW` seam. Grep confirms the literal now appears in
that file only inside test bodies.

## M4 (Minor) — the money detector's stated limits replaced with the real ones

`crates/btctax-core/src/tax/provenance.rs:583-606`. The claim that a new `Option<Usd>` left `None`
*"rejects both probes and is therefore reported as not money"* is gone; it is replaced by a note that
the case **is** detected (`walk` emits the `null`, `set_at` overwrites it, the `Option<Usd>`
deserializer classifies it), and by the two real blind spots, each with its mechanism:

1. a field removed from the serialized JSON by `skip_serializing_if` (verified in use at
   `crates/btctax-core/src/forms.rs:293,295`), which `walk` never emits;
2. the element leaves of an **empty `Vec`** — `walk` descends an array only when some element is an
   object or array, so `[]` is one leaf at the vec's own path and every money box on the element type
   goes unwalked. This is why `maximal_sentinel` realizes two rows of every `Vec`.

The (correct) note that `classifier::no_option_money_leaf_is_bound_with_underscore` is the second net
is kept.

## N1 — the fixture-shrink floor

`crates/btctax-core/src/tax/provenance.rs:665-672`: `money.len() > 50` → `>= 100`. Measured, not taken
from the review — temporarily raising the floor to `1000` printed the real count:

```
the maximal fixture realized only 106 money leaves — it has stopped being maximal, and a shrunken
fixture makes this KAT vacuous
```

## N2 — `answer.rs`'s enforcement claim re-worded to what the code does

`crates/btctax-cli/src/cmd/answer.rs:307-319`. The old comment claimed *"the three skippable shapes and
the declaration all pass through this line"*. They do not: the declaration records inside its own
branch (its record is always `Given` and its loop cannot exit without a value), and the site is an
`if let Ask::Skippable(sk)`, which a third `Ask` variant would escape silently. The replacement says
what the placement *does* buy (one recording site for the three skippable shapes, state read back off
`ri`) and names what it does not — with the honest note that the exhaustive `match` above is the real
net, and that it is a compile error forcing someone to look at this line, not a guarantee they will add
an arm.

## N3 — `Declined`'s missing production reader, dated

`crates/btctax-core/src/tax/provenance.rs:281-296`. A note on `AnswerState` that `Declined` is written
by `income answer` and read by nothing in production today, that its reader is **T3**'s
`interview_state()` (which lists a `Declined` class-(B) item in `forgoing` marked *(declined)* and never
in `blocking`) rendered by **T12** — verified against `SPEC_interview.md:1191` and `:1201` — and that it
stops being acceptable if T3 slips. Same *"a stored value with no reader is not a guarantee"* caveat the
module already states for `prompt_hash`.

---

## Validation

`cargo fmt --all` clean. Clippy clean:

```
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 1.51s
```

The workspace clippy run also type-checks every crate against the removal of
`supersede_stale_prompts` — nothing outside `provenance.rs` referenced it.

Tests, over the five crates this fold touches (the whole workspace was **not** run; the controller's
gate runs `make check`):

```
$ cargo nextest run --locked -p btctax-core -p btctax-input-form
     Summary [   0.546s] 1296 tests run: 1296 passed, 0 skipped

$ cargo nextest run --locked -p btctax-cli
     Summary [   6.431s] 721 tests run: 721 passed, 1 skipped

$ cargo nextest run --locked -p btctax-core -p btctax-input-form -p btctax-cli -p btctax-tui-edit -p xtask
     Summary [  13.818s] 2530 tests run: 2530 passed, 4 skipped
```

`1296` in core + input-form is byte-for-byte the review's own baseline for those two crates. The five
crates go **2525 → 2530**: five new tests, all in `btctax-cli` —
`fullreturn_fixture_matches_its_emitter`, `an_imported_toml_cannot_mint_an_answer_record_or_a_history_
entry`, `a_re_import_keeps_every_answer_record_already_on_the_row`,
`re_answering_at_the_keyboard_moves_the_stale_record_into_history_by_itself`,
`a_record_whose_words_changed_still_reads_as_wording_changed_after_a_load`. Core is net zero (one test
renamed and rewritten; no test deleted). The workspace figure should therefore read **3202** against the
3197 at dispatch HEAD.

## Diff surface

```
 crates/btctax-cli/src/cmd/answer.rs                |  15 +-
 crates/btctax-cli/src/cmd/tax.rs                   |  49 ++++
 .../tests/fixtures/examples/fullreturn_inputs.toml |  25 +-
 crates/btctax-cli/tests/fullreturn_oracle.rs       |  77 ++++-
 crates/btctax-cli/tests/scrub_refusal.rs           |  35 ++-
 crates/btctax-cli/tests/tax_report.rs              | 322 +++++++++++++++++++++
 crates/btctax-core/src/tax/provenance.rs           | 200 ++++++++-----
 crates/btctax-tui-edit/src/edit/tax_inputs.rs      |   4 +-
 8 files changed, 627 insertions(+), 100 deletions(-)
```

No spec file and no `design/agent-reports/*.md` other than this one was touched. `git status
--porcelain` lists only the eight files above.

## What a re-review should look at first

1. **The I1 deviation** — is refusing the read-boundary sweep right? The measurement is above and the
   guard is `a_record_whose_words_changed_still_reads_as_wording_changed_after_a_load`. The question is
   whether R10.3's *"kept as history"* is discharged by supersession-at-re-answer alone.
2. **Deleting `supersede_stale_prompts`** — a `pub` API removal in `btctax-core`, justified by "a
   function with no production caller is the finding". If T3 wants a bulk view, it will need one that
   does not run at a load.
3. **The C1 note's wording** — it is filer-facing text on a path a recipient of a scrubbed file walks.
4. **The `== 14` assertion** in `a_re_import_keeps_every_answer_record_already_on_the_row`: deliberate
   (a survival test over an empty log passes for the wrong reason), and it will red informatively when
   a registry grows.
