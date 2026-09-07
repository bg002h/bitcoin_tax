# Seam review — interview build T4 (the year gate and draft protection, R11)

Reviewer: independent adversarial build reviewer, own worktree, read-only for the record.
Range: `git diff 15c15927..bbcee739`. Worktree reset to `bf26f1e8` (HEAD of `main`) before starting —
it had been left at `2bd04d45`, an ancestor. Every plant below was made in this worktree and
reverted from a `cp` backup; `git status` is clean apart from this file.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

# seam 1 — the shared param-free tier, un-planted
cargo nextest run --locked -p btctax-core -E 'test(param_free_tier)'
    4 tests run: 4 passed, 1251 skipped

# seam 1 — PLANT: gate the census call behind the unanswered tier
#   `if let Some(r) = screen_document_census(ri)` → wrapped in `if tier.unanswered_refuses { … }`
cargo nextest run --locked -p btctax-core -E 'test(param_free_tier)'
    FAIL every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths
    assertion `left == right` failed: `income import` screens on a year with NO package —
      DocumentTypeUnsupported must fire there too
      left: None
     right: Some("DocumentTypeUnsupported")
  (reverted from a cp backup)

# seams 2/3/4/6 — plants appended to crates/btctax-cli/tests/year_gate_t4.rs, then reverted
cargo nextest run --locked -p btctax-cli -E 'binary(year_gate_t4) and test(plant_)' --no-capture --no-fail-fast

# regression surface, un-planted
cargo nextest run --locked -p btctax-cli \
  -E 'binary(year_gate_t4) or binary(slice_from_answers) or binary(tax_report) or binary(tax_profile)'
    102 tests run: 102 passed, 0 skipped
```

## Summary

The one question — *can a filer's work be lost or poisoned across the year gate* — splits four ways,
and three of the four are clean.

**(a) unscreened committed row on a params-less year:** closed. `income import` runs
`screen_param_free` before the coherence clear, before `return_inputs::set`, and before the FR-48
note; on refusal nothing is saved. Verified on TY2026 for three refusal shapes (`documents.k1 =
true`, a `documents.w2 = false` beside a `[[w2s]]` row, and a negative `box1_wages`): each refused
with the row's own §2.2 sentence, left `return_inputs::get == None`, and left a pre-existing draft
**byte-identical**; the corrected TOML then imported. The param-dependent half also holds — a
§402(g) excess imports on TY2024 and `commit` refuses it.

**(c) an answer that leaks or vanishes:** closed. `income answer` on a draft-only year writes the
draft and never `return_inputs::set`; `resolve.rs` contains no reference to `input_form_store` or the
draft table at all; the second `income answer` on the same year updated all 31 records in place
(`answer_log` is a `BTreeMap`, so a duplicate key is structurally impossible) and refreshed
`answered_on` to the second run's date with an empty history; and the draft's 30 recorded answers
reached the committed row unchanged through `commit` on a params-bearing year.

**(d) a rule split between the two tiers:** closed, and the mechanism is the right one. There is one
body (`screen_inputs_tiered`) under two `ScreenTier`s, so no second list exists to drift; the tier
census is parsed out of `include_str!("return_refuse.rs")` (31 param-free + 3 package-gated
fixtures, 34 `add(` calls, matching the report). I planted a defect the builder did not — gating the
`screen_document_census` call behind `tier.unanswered_refuses`, i.e. silently removing the three
census VALUE rules from the import tier, which is exactly the poison T4 exists to stop — and the
census red is quoted above. B1 satisfied independently.

**(b) a non-trivial draft deleted without confirmation: NOT closed.** `draft_holdings` counts four
things, and one of them is derived from the census while the *category list itself* is hand-written.
A TY2026 draft holding the Form 1099-DA broker answers — which this very file calls *"the primary
authoring path on a params-less year"* — plus a Schedule C reports `is_empty() == true` and is
destroyed, unconfirmed, by all four deleting paths. That is the flagship deliverable failing on the
flagship year.

Findings: 1 Critical, 0 Important, 3 Minor, 1 Nit.

---

## C-1 (Critical) — `draft_holdings` reports "nothing" for a TY2026 draft holding the 1099-DA broker answers, and all four deleting paths then destroy it unconfirmed

**Where.**
`crates/btctax-cli/src/input_form_store.rs:309-372` (`DraftHoldings` / `draft_holdings`), consumed by
`coherence_check` at `:414`, and by `load`'s §6.3 stale-WIP discard at `:212`. Through those two, the
four paths are `income import` (`cmd/tax.rs:172`), `income clear` (`cmd/tax.rs:444`),
`report --write-carryover` on year N+1 (`cmd/tax.rs:976`), and `input_form_store::load`.

**What is wrong.**
`draft_holdings` is a hand-written list of four categories:

```rust
DraftHoldings {
    answers: ri.answer_log.len(),
    documents: DocumentRow::ALL.iter().filter_map(…),   // derived from the census ✓
    dependents: ri.header.dependents.len(),
    schedule_a: ri.schedule_a.is_some(),
}
```

The *documents* half is genuinely derived — the doc comment is right that "a document type that gains
a section is counted here the day it gains one". But the four **categories** are a hand-list, and the
predicate's failure direction is open, not closed: anything not on the list makes the draft
`Disposable`, i.e. destroyable on a note.

`ri.broker_reporting` is not on the list. It is also not an `answer_log` writer: `BrokerCovered` /
`BrokerNoncovered` are real `FieldId`s edited in the tax-inputs form
(`btctax-input-form/src/spec/sections.rs:1567+`, `seam.rs:190-194`), but `field_to_question` and
`field_to_skippable` (`spec/registries.rs:545`, `:647`) do not map them, so `apply.rs:83-87` records
nothing for them. Neither are `schedule_c`, `schedule_1a`, `sch1`, the carryovers, `qbi`, or the
`header` identity fields.

So the one thing `input_form_store.rs:106` itself identifies as living in a params-less year's draft —
*"a year whose answers live ONLY in the draft (the primary authoring path on a params-less year)"* —
is the one thing the new protection does not see. `load`'s new comment says the discard is "narrowed
to the drafts §6.3 was written about"; for a broker-answers draft it is not narrowed at all.

Note that `coherence_clear`'s `Disposable` arm (`:437-447`) already carries a *wider* notion of
non-trivial — `d.ri != ReturnInputs::default()` — and uses it only to decide whether to print a note
before deleting. T4 introduced a strictly **narrower** predicate for the refusal than the one already
in the file for the warning.

**Evidence — the plant and the observed output.**

Plant (appended to `crates/btctax-cli/tests/year_gate_t4.rs`, reverted afterwards): a TY2026 draft
carrying two providers' `CohortAnswers` and a Schedule C, saved with `save_draft`, then
`import_return_inputs(..., force=false, discard_draft=false)`.

```
PLANT draft_holdings = DraftHoldings { answers: 0, documents: [], dependents: 0, schedule_a: false }  is_empty=true
note: TY2026 — preparing (0 forms; TaxTable yes; full-return params no; 1099-DA proceeds+basis) — these inputs are stored now; …
note: superseding a work-in-progress draft for 2026 with this write.
PLANT import result = Ok(())
PLANT draft survived = false

thread '…' panicked at crates/btctax-cli/tests/year_gate_t4.rs:691:5:
a TY2026 draft holding the 1099-DA broker answers and a Schedule C was DESTROYED by an unconfirmed `income import`
```

The same draft through the other two reachable paths:

```
PLANT income clear = Ok(false)
PLANT draft survived income clear = false
thread 'plant_income_clear_destroys_a_broker_only_draft_unconfirmed' panicked at …:670:5:
`income clear` destroyed a draft holding the 1099-DA answers

PLANT stale load discarded (Ok) = true
thread 'plant_stale_load_discards_a_broker_only_draft' panicked at …:686:5:
the §6.3 stale-WIP discard destroyed a draft holding the 1099-DA answers
```

`report --write-carryover` shares `coherence_clear_or_refuse` on `year + 1` and therefore the same
predicate.

None of the T4 tests can see this: every draft fixture in `year_gate_t4.rs`
(`draft_holding_an_interview`) and in `input_form_store::tests` builds its non-disposable draft out
of an `answer_log` record, a `w2s` row, a dependent or a Schedule A — the four categories the
predicate already knows. `draft_holdings_counts_answers_documents_dependents_and_a_schedule_a`'s
part (d) loop asserts the derivation *within* the document category only.

**Minimal change.**
Make the predicate fail closed rather than enumerate. The smallest correct version reuses the test
already in the file two functions down:

```rust
pub fn draft_is_disposable(ri: &ReturnInputs) -> bool {
    draft_holdings(ri).is_empty() && *ri == ReturnInputs::default()
}
```

and keys `coherence_check` / `load` on that, keeping `DraftHoldings::describe()` purely for the
message (with a fallback clause — e.g. `"work not otherwise itemised"` — when the difference is in an
uncounted field). Then a field added to `ReturnInputs` is protected the day it is added, which is the
direction the rest of this build takes everywhere else. Note this reclassifies
`a_disposable_draft_is_still_superseded_without_a_flag`'s fixture (`filing_status: Mfj` on an
otherwise-default return) as non-disposable; if that case must stay silent, compare against
`ReturnInputs { filing_status: ri.filing_status, tax_year: ri.tax_year, ..Default::default() }`
rather than re-opening the category list. Adding `broker_reporting` (and the rest) to
`DraftHoldings` by hand closes today's instance and leaves the class open.

---

## M-1 (Minor) — the TUI's discard-only screen tells the filer a WIP interview draft is "parked"

**Where.** `crates/btctax-tui-edit/src/draw_edit.rs:2230`, `:2252`, `:2256`; the routing is
`main.rs:908-911`.

**What is wrong.** `open_tax_inputs_form` now routes `CliError::StaleDraftHoldsInterview` into the
P2-a discard-only screen alongside `StaleParkedDraft`, which is the right call — it is what makes the
new refusal discardable in-app. But the screen's chrome is still hard-coded for the parked case:
heading `"Stale parked draft for {year}"`, block title `" Tax inputs — stale parked draft "`, and
prompt `"Press X to discard the parked draft, Esc to back out."` A stale WIP draft holding an
interview is not parked, and "parked" has a specific meaning in this product (a return the filer
*withdrew*). The payload itself is correct — `form.error` is rendered and carries the `holdings`
clause — so the confirmation is real, not advisory, and `X` acts only on the screen that shows the
payload (`main.rs:1116-1123`; there is no second modal, by design).

**Evidence.** Read of the render fn and the routing arm; the error text a filer sees on that screen
is quoted under N-1 below.

**Minimal change.** Derive the three strings from which error opened the screen (the state already
distinguishes them at the routing site), or neutralise them to "draft this build cannot open", which
is the wording `discard_parked_now` already uses for the success status.

## M-2 (Minor) — a refusal now raised at import prescribes `income answer`, which cannot be reached on a year the import just refused to create

**Where.** `crates/btctax-core/src/tax/return_refuse.rs:1587-1593`
(`RefuseReason::SalesTaxElectionWithoutAmount`), reached at import via `screen_param_free`.

**What is wrong.** The rule is in the param-free tier (it has a fixture in `param_free_fixtures`), so
T4 moved it from commit-time to import-time. Its detail ends:

> "…enter the amount and re-run `btctax income import`, or run `btctax income answer` to turn the
> election off and deduct income taxes"

On a params-less year with no committed row, the second exit does not exist — `income answer` refuses
with *"no full-return inputs and no draft for tax year {year}"*. This is a narrow instance of exactly
the reasoning the builder's deviation 4 gets right for the unanswered tier. It is Minor rather than
Important only because the *first* exit is named first and does work: the filer holds the TOML.
I checked the whole tier for this shape (`grep 'income answer'` over the two screen bodies) — this is
the only occurrence; `ScheduleBForeignCountryMissing` explicitly names `income import` and says why.

**Minimal change.** Drop the `income answer` clause, or condition the sentence on the caller.

## M-3 (Minor) — `working_return`'s new hard error propagates into read-only surfaces that previously continued

**Where.** `crates/btctax-cli/src/input_form_store.rs:212-219` (the new `Err` from `load`), reaching
`working_return` (`:263`) and `broker_answers` (`:286`).

**What is wrong.** Before T4, a stale-version WIP draft was discarded and the caller continued with a
note. Now, if it holds an interview, `load` returns `Err`. `Session::broker_reporting_answers`
(`session.rs:611`) swallows it with `if let Ok(...)`, but `cmd/tax.rs:575`, `cmd/tax.rs:646` and
`cmd/admin.rs:212` / `:755` propagate with `?`, so `income show-broker-answers`, `report` and the
`export-irs-pdf` broker path will hard-fail on such a year rather than render. This is the
fail-closed direction and it destroys nothing — and the error text names the remedy — but it is a new
class of hard failure on read-only commands, reachable the first time `SCHEMA_VERSION` moves.

**Minimal change.** Nothing is required for correctness; if it is worth closing, have the read-only
projections (`working_return`/`broker_answers`) map `StaleDraftHoldsInterview` to
`(None, Some(note))` the way they already handle `StaleNote`, and leave the hard refusal to the
write paths.

## N-1 (Nit) — the two new `CliError` messages carry collapsed line breaks

**Where.** `crates/btctax-cli/src/lib.rs:184-187` (`NonTrivialDraftBlocksWrite`) and `:194-203`
(`StaleDraftHoldsInterview`).

Both `#[error(…)]` literals are single lines with runs of ~10 spaces where a `\` continuation is
missing, unlike `ParkedDraftBlocksWrite` immediately above them, which does it correctly. Rendered:

```
year 2026's draft is schema v1 but this build expects v2, and it holds          3 recorded answer(s),
2 Form W-2 row(s) — an upgrade changed the input format, so this build cannot read it. It was
NOT discarded. …
```

This is the sentence a filer reads at the moment their months of work is at stake, and on the TUI
discard screen it is also the payload of the confirmation.

---

## Seams checked clean

1. **The shared param-free tier.** Enumerated from the code, not the report: one body
   `screen_inputs_tiered` (`return_refuse.rs:1250`) with two `ScreenTier` entry points; the only
   tier-variable surface is two `if let Some(…) = tier.package {` blocks and one
   `if tier.unanswered_refuses {`. `screen_document_census` is called *outside* the unanswered gate
   (`:1367`), so the three census VALUE rules do run at import — confirmed behaviourally. The census
   KAT reds on a planted move of the census call inside the gate (output above). 31 param-free + 3
   package fixtures; 34 `add(` calls in the source. The unanswered-gate deviation is argued
   correctly and its two kills are real.
2. **Import writes nothing on refusal.** Three refusal shapes on TY2026 each left the draft
   byte-identical and `return_inputs::get == None`; the FR-48 "these inputs are stored now" note did
   **not** print on any refusing run (the screen sits above it, as documented); the corrected TOML
   imported. On TY2024 a §402(g) excess imported and `commit` returned `Refused`.
3. **Draft protection versus coherence** — the path enumeration is complete and correct: the five
   production `delete_draft` callers are `load`'s stale discard (`:212`), `coherence_clear`'s two arms
   (`:449`, `:454`), `commit` (`:554`, which writes the committed row in the same
   `mutate_and_save`), and `discard_blocked_draft` (`:663`). The sixth writer the builder volunteered,
   `park_to_profile` (`:594`), is indeed fail-closed via its `parked_flag(...) == Some(false)` gate —
   verified by reading, it refuses *any* WIP draft, trivial or not. `coherence_check`/`coherence_clear`
   really are split (the check deletes nothing), and every `Err` path in `write_back_carryover` after
   its up-front clear returns before `s.save()`, so an in-memory delete never reaches disk on a
   refusal. The TUI's confirm is required, not advisory. **The predicate feeding all of it is C-1.**
4. **`income answer` into a draft-only year.** Verified end to end, including the two things the
   report did not claim: a second run updates the same 31 keys in place with a refreshed
   `answered_on` and an empty `answer_log_history`, and the draft's 30 records survive into the
   committed row through `commit`. `answer_target` preserves the M-1 ordering with an explicit
   `parked_flag` check ahead of the committed-row read. Both-absent still refuses with the widened
   `answer.rs` message.
5. **The two entry states.** `EntryStates::for_year` is `interview_state(ri).blocking.is_empty()` ×
   `YearReadiness::bundled(year).params` — genuinely independent, and the TUI recomputes only the
   interview half per frame. The three other places "authorable" is decided agree with R11:
   `open_tax_inputs_form` (`main.rs:800`) has no year gate at all, `import_note`
   (`year_readiness.rs:355`) stores-with-a-note rather than refusing, and `uncomputable_sentence`
   keeps the inputs. Both recorded deviations here (no "expected Jan 2027"; 1-or-3 lines rather than
   one sentence) are sound — no bundled record carries a package date, and the FR-63 rule applies.
6. **The year+1 write.** `cmd/tax.rs:1024` at `15c15927` is
   `return_inputs::set(s.conn(), year + 1, &updated)` inside `write_back_carryover` — the builder's
   identification is correct. `--discard-draft` is correctly scoped to year N+1 and correctly kept
   separate from `--force`. The TY2026 slice path is untouched: `slice_from_answers` 20/20 green
   inside a 102/102 run alongside `year_gate_t4`, `tax_report` and `tax_profile`.

Counts: C=1 I=0 M=3 N=1
