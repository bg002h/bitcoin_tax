# Seam review — interview build T6 (the exchange seam), commit `a79492ee`

Independent adversarial build review, own worktree at `673b0f51` (the T6 code is byte-identical:
`git diff a79492ee..673b0f51 -- crates` is empty). Read-only for the record — every plant was made
here and reverted from a `cp` backup or deleted; nothing committed, no subagents.

## Commands

```
git diff --stat 2fe4ba5f..a79492ee -- crates docs Cargo.lock          # 48 files, +3117/-474
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
cargo nextest run --locked -p btctax-core -E 'binary(zz_review_plant)' --no-capture
cargo nextest run --locked -p btctax-core -E 'test(every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths)'
cargo nextest run --locked -p btctax-cli  -E 'binary(zz_review_slice_da)' --no-capture
cargo nextest run --locked -p btctax-cli  -E 'binary(zz_review_panel_regime)' --no-capture
cargo nextest run --locked -p btctax-cli  -E 'test(the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_the_day_before)'
cargo run -q -p xtask -- stop-list                                    # clean, and RED on a plant
cargo nextest run --locked --no-fail-fast -p btctax-core -p btctax-cli -p btctax-forms \
      -p xtask -p btctax-input-form -p btctax-tui-edit
      → 3045 run: 3038 passed, 7 failed, 8 skipped
      → the 7 are exactly the declared environment set (6 × form_delta + harness_check)
```

## Summary

The core of T6 is sound and the kills are real. On the **full-return** path the printed Digital
Assets box is `ri.digital_asset_activity` and only that; a contradicted `No` refuses naming the
first qualifying event by the instruction's own list (a purchase is never named — measured); an
unwitnessed `Yes` is accepted with the advisory and never refused; the cross-check is in
`screen_compute_dependent` and the T4 tier census **reds** when it is moved into the param-free
body; the Step 0 panel cannot write (its signature takes `&LedgerState`, `&[LedgerEvent]`,
`Option<&ReturnInputs>`); the stop-list extension **reds** on a banned word planted in a rendered
prompt.

Two defects, both outside the window the build's own fixtures look through — every Step 0 fixture is
TY2026, and no fixture exports the R6 arm-(2) slice:

- **C-1** — on the R6 arm-(2) crypto slice (TY2025, the only year this build can print for) the
  Digital Assets box is still decided by the ledger predicate. Measured: stored answer `Some(false)`
  → **Yes** printed; stored answer `None` → **Yes** printed; and no refusal, no hand-mark and no
  advisory on that path. The build report's recorded deviation 2 says *"the answer wins where one
  exists"* — no code on that path reads the answer at all.
- **I-1** — the Step 0 venue-vs-answer list drops the regime gate `screen_broker_reporting` applies,
  so on TY2024 and TY2025 it names every exchange venue as *"not accounted for"* and hands the filer
  to a Form 1099-DA block that is not asked and whose answer would be refused.

Answering the two pointed questions directly:

- **(a) the slice's ledger fallback.** There is no fallback: the slice reads the ledger
  unconditionally, on both arms, whether or not a return exists. That is not what spec 1099-DA R6
  requires — R6 exempts the slice from `screen_inputs` / `screen_compute_dependent` on the stated
  premise that *"neither reaches a figure the slice prints"*, and T6 invalidated that premise by
  making the box an answer. See C-1.
- **(b) a GLOBAL election silencing the standing-order row.** Defensible; no finding. Notice
  2026-20 §4.02(2) requires only that the standing order *"include[] sufficient information to
  identify any digital asset units sold"* and be *"entered into the taxpayer's books and records
  before the units covered by the order are sold"* — it imposes no account scoping. The
  account-by-account requirement is §1012(c)(1) (recited in the Notice's Background at
  `Notice_2026-20.txt:31-34`, `:70-71`), and it binds the *application* of the convention, which
  `resolve_election` (`project/resolve.rs:187-212`) satisfies: a global election is resolved
  per-wallet, tier 2, only where no scoped one is in force. Warning a filer who has made the
  identification would contradict the engine's own `StandingOrder` verdict.

---

## C-1 (Critical) — the R6 arm-(2) crypto slice prints the Digital Assets box from the LEDGER, never the filer's answer

**Where.** `crates/btctax-cli/src/cmd/admin.rs:1150-1161` (the arm-(2)/(3) `Form1040Inputs`
construction) and `crates/btctax-forms/src/form1040.rs:132-155` (`fill_form_1040_capgains`).

**What is wrong.** T6 changed the full-return chain (`PrintedInputs.digital_asset_answer` →
`Form1040Lines.digital_asset_answer` → `form1040_full.rs`) but left the crypto-slice chain exactly
as it was:

```rust
// admin.rs:1150 — unchanged by a79492ee; `ri`/`working` is in scope at :783 and never consulted
let da_yes = !rows.is_empty()
    || state.income_recognized.iter().any(|i| i.recognized_at.year() == tax_year)
    || state.removals.iter().any(|r| r.removed_at.year() == tax_year);
```

`grep -n "digital_asset" crates/btctax-cli/src/cmd/admin.rs` returns exactly two hits — line 440
(`hand_marks`) and a test name. The stored answer reaches nothing on this path.

The path is not hypothetical or future. `BundledFullReturnTables::load()` bundles **TY2024 only**
(`btctax-adapters/src/tax_tables.rs:100-104`), so `full_return_for(2025).is_none()`; TY2025 bundles
fifteen templates and its `f1040.map.toml:16-17` carries both `da_yes` and `da_no`. TY2025 with
stored answers **is** R6 arm (2) — and per the standing memory it is the year btctax exists to file.

**Evidence** (plant: `crates/btctax-cli/tests/zz_review_slice_da.rs`, three cases, deleted after the
run; TY2025 templates with the LIVE regime injected, the same pattern `slice_from_answers.rs` uses;
the box read back off the emitted `form_1040_capgains.pdf` through `Form1040Map::ty2025()`):

```
PLANT-SLICE stored answer = Some(false); printed da_yes=true da_no=false
PLANT-SLICE hand_marks = []
PLANT-SLICE stored answer = Some(true);  printed da_yes=true da_no=false
PLANT-SLICE stored answer = None;        printed da_yes=true da_no=false
PLANT-SLICE hand_marks = []
```

So on the arm the product will actually use this filing season:

- a filer who answered **`No`** gets a Form 1040 page 1 with **Yes** checked — the opposite of their
  own testimony, with no refusal (`screen_compute_dependent` is not run on this arm, by R6);
- a filer who has **never been asked** (`None`) gets **Yes** checked — btctax answering a §6065
  declaration for a human, which is the single defect T6 exists to close;
- `hand_marks` is empty in both cases, so the T6 fail-closed backstop
  (`admin.rs:440`, keyed on `digital_asset_answer.is_none()`) never fires here — it reads a
  `PrintedReturn` that only the full-return arm builds. The slice arm also sets
  `advisories: Vec::new()` (`admin.rs:1233`), so the off-ledger `Yes` warning is absent too.

`form1040.rs:117-120` states the product's own position on that cell: *"btctax vouches for exactly
two cells on it (the digital-asset question and line 7a)"*. It is vouching for a cell it holds no
testimony for.

**Mitigation, stated for the fold's grading:** the artifact is watermarked
(`stamp_partial_worksheet_watermark`, `admin.rs:1174`) and the slice note calls it *"a worksheet, not
a return"*. It is nonetheless a printed Form 1040 page 1 the filer transcribes onto the return they
sign.

**Also a report defect.** Recorded deviation 2 reads *"The answer wins where one exists; where none
does, the pre-T6 never-`No` behaviour is kept."* Half of that is true (never `No`); the first half is
not implemented anywhere.

**Minimal change.** Feed the slice's `da_yes` from the working return the arm already resolved at
`admin.rs:783` — `working.as_ref().and_then(|ri| ri.digital_asset_activity)` — and make
`Form1040Inputs.da_yes` an `Option<bool>` so `fill_form_1040_capgains` can check *No* and can print
neither on `None` (the produce/skip decision then needs its own predicate, since today `!da_yes`
means "skip the whole 1040"). If instead the owner rules that the slice worksheet must stay
ledger-derived, then R6's premise sentence needs amending and the slice must say on the page that
this box is btctax's reading of the ledger and not the filer's answer — silence is what makes it
testimony.

---

## I-1 (Important) — the Step 0 venue-vs-answer list ignores the year's Form 1099-DA regime

**Where.** `crates/btctax-cli/src/step0.rs:218-245`; echoed into the TUI commit modal at
`crates/btctax-tui-edit/src/edit/tax_inputs.rs:730-742`.

**What is wrong.** The list claims to share the screen's derivation:

> ★★★ THE SAME KEYS `screen_broker_reporting` DEMANDS. `broker_key_census` over `form_8949` is
> the one derivation … so the panel can never name a venue the screen would not ask about, or stay
> quiet about one it would. — `step0.rs:216-221`

It shares the *key* derivation but drops the *liveness* gate. `screen_broker_reporting`
(`return_refuse.rs:1266`) computes `broker_question_is_live(&rows, regime)` and, when that is false,
**refuses any stored answer** as `RefuseReason::BrokerAnswerUnread` with the sentence *"TY{year}'s
Form 1099-DA regime reports proceeds only (no basis) — the tool would never read this answer, and
testimony it would discard is not kept"*. The panel consults no regime at all.

**Evidence** (plant: `crates/btctax-cli/tests/zz_review_panel_regime.rs`, one exchange round-trip per
year, no stored answers, deleted after the run):

```
YEAR 2023: 8949 rows=1, regime=Err(UnsupportedYear(2023)), broker_question_is_live=None
  VENUE ROW: coinbase has 1 noncovered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file — a venue you disposed on is not accounted for
      handoff: btctax income answer --year 2023  (the Form 1099-DA block)
YEAR 2024: 8949 rows=1, regime=Ok((proceeds=false, basis=false)), broker_question_is_live=Some(false)
  VENUE ROW: coinbase has 1 noncovered row(s) … NO Form 1099-DA answer on file — a venue you disposed on is not accounted for
      handoff: btctax income answer --year 2024  (the Form 1099-DA block)
YEAR 2025: 8949 rows=1, regime=Ok((proceeds=true, basis=false)), broker_question_is_live=Some(false)
  VENUE ROW: coinbase has 1 noncovered row(s) … NO Form 1099-DA answer on file — a venue you disposed on is not accounted for
      handoff: btctax income answer --year 2025  (the Form 1099-DA block)
YEAR 2026: 8949 rows=1, regime=Ok((true, true)), broker_question_is_live=Some(true)
  VENUE ROW: … (correct here)
YEAR 2027: 8949 rows=1, regime=Err(UnsupportedYear(2027)), broker_question_is_live=None
  VENUE ROW: … (fires anyway)
```

Every Step 0 fixture in the build is `const YEAR: i32 = 2026` (`step0_panel.rs:31`), which is the one
year the row is right for. So on the TWO years the product actually serves — TY2024 (the only
full-return year) and TY2025 (the filing year) — every filer with an exchange disposal gets a yellow
Step 0 row on the `income answer` header and on the TUI entry screen telling them a venue *"is not
accounted for"*, pointing at a Form 1099-DA block that is not asked for that year and whose answer,
if supplied, refuses. The commit modal repeats it under

> `VENUES WITH NO FORM 1099-DA ANSWER (commit is not blocked by this; the EXPORT is)`

— and on a non-live year the export is not blocked by it either, so the modal asserts a gate that
does not exist.

**Minimal change.** Thread the year's regime into `step0_panel` (the callers already have it:
`admin.rs` uses `year_readiness::regime_or_refuse`, and `TaxInputsFormState.broker_regime` is already
carried) and build the `venues` list only when
`btctax_core::forms::broker_question_is_live(&rows, regime)`; on a non-live year either omit the list
or state the regime instead of a missing answer. Pair it with a fixture at TY2024 or TY2025 — the
row's absence is the assertion the current pair cannot make, because both halves of it run at 2026.

---

## M-1 (Minor) — the standing-order row fires outside Notice 2026-20's relief period

`step0.rs:269-303`. `first_custodial` is filtered by tax year only, so the row — printed by
`income answer` under the heading `STANDING ORDERS (Notice 2026-20 §4.02(2))` — is emitted for any
year. Measured above: it fires for 2023, 2024 and 2027. The Notice's relief period is
2025-01-01 → 2026-12-31 (`Notice_2026-20.txt:299-300`) and §5 says taxpayers *"may not rely on the
temporary relief … after the relief period ends"*. For a pre-2025 year the row is doubly inapt: the
product's own `method_election_is_forward` refuses any election effective before `TRANSITION_DATE`
(2025-01-01, `project/resolve.rs:1474-1489`), so the named exit cannot be taken at all. The substance
(no dated election ⇒ the broker's default) survives; the authority cited does not.

**Minimal change.** Gate the row (or at least the §4.02(2) citation and the `STANDING_ORDER_CONSEQUENCE`
sentence) on the relief period, and hand a pre-2025 year to the `Pre2025MethodNote` exit it already
has.

## M-2 (Minor) — a standing order recorded the DAY OF the sale, after it, silences the warning

Seam 4 asked which way the boundary falls. Measured: **silent**. Plant — the committed fixture pair's
half (b) with the election *made* at `2026-03-10 18:00 UTC` and effective `2026-03-10`, six hours
after the 12:00 sale — and `the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_the_day_before`
still **passes**. That is not §4.02(2)'s rule: (2) says *"entered into the taxpayer's books and
records **before** the units covered by the order are sold"*, while (1) is the one that says *"no
later than the date and time"*. `standing_order_in_force` (`compliance.rs:243-251`) inherits
`resolve_election`'s `effective_from <= date` on a day-granular `TaxDate`, so the last day of the
window fails open.

The panel mirroring the engine is deliberate and I am not proposing the panel diverge from it. The
root is the pre-existing `<=` — which also decides the FILED BASIS through `disposal_compliance` —
and that belongs in a follow-up, not in T6. What T6 can do at no cost is stop the doc comment and the
fixture pair claiming a *"before"* they do not test: add a same-day case and say which way it falls.

## M-3 (Minor) — the new granularity negative test scans only the STATIC prompts

`crates/btctax-cli/tests/step0_panel.rs:685-708`. It walks `FORM_QUESTIONS` and
`SKIPPABLE_QUESTIONS` prompts for *"how many accounts"* / *"which account"* and never renders
`RENDERED_PROMPTS` — reintroducing, in the same commit, exactly the blindness the T6 stop-list
extension was written to end (`r15_stop_list.rs:277-289`: *"the words a filer is actually SHOWN were
never read … 'it happened to be clean' is not the same fact as 'it is checked'"*). No rendered prompt
contains those phrases today; the argument for checking is the builder's own.

## M-4 (Minor) — a contradicted `No` does not refuse at TUI commit, and the panel cannot show it

`input_form_store::commit` (`crates/btctax-cli/src/input_form_store.rs:638-662`) runs `screen_inputs`
only; `screen_compute_dependent` is never called there, and `interview_state(&ri)` takes no ledger,
so neither the commit gate nor R12's panel can see the contradiction. A TY2024 filer can therefore
commit `digital_asset_activity = Some(false)` against a ledger full of disposals and first meet the
refusal at `report` / `export-irs-pdf`. Defensible as a tier boundary (the commit site holds no
`state`), and it is not claimed anywhere in the build report or the brief — recorded so the deferral
is a decision rather than an oversight. If it is to stand, the `income answer` Step 0 print is the
natural place to surface it, since that surface *does* hold the ledger.

## N-1 (Nit) — `disposal_compliance` lost its doc comment to the new helper

`crates/btctax-core/src/project/compliance.rs:180-206`. The extracted `voided_set` was inserted
between `disposal_compliance`'s doc block and its signature, so ~25 lines describing
`disposal_compliance` (*"this function (which iterates only `state.disposals` / `state.removals`)"*,
*"Output is sorted by `disposal`"*, the NFR4 determinism note) now document a four-line `BTreeSet`
builder, and `disposal_compliance` (`:253`) has no doc comment at all.

## N-2 (Nit) — two internal inconsistencies in the build report

(a) The five-row table's row 1 describes *"a 2026-06-15 Coinbase disposal"* while the fixture
(`kat_digital_asset_question.rs:91`) and the report's own quoted payload are `2024-06-15`.
(b) *"3,358 tests"* is the exact sum of the ten crates listed and omits the rest of the workspace;
the controller measured 3363 / 12 skipped. Neither figure is wrong — the report's number is a
sub-total presented as a total.

## N-3 (Nit) — two off-by-a-few instruction citations

`questions.rs:141-152` cites the carve-outs as `i1040gi--2025.txt:1385-1394`; the sentence it quotes
(*"The following actions or transactions in 2025, alone, generally don't require you to check
'Yes'"*) is at `:1382-1384`, and the bullets run `:1385-1391` + `:1395-1396`. `questions.rs:313-314`
and `return_refuse.rs:616` cite the mandatory-answer sentence as `:1398-1400`; it ends on `:1401`.
`return_inputs.rs:1605` (`:1352-1357`) and the `f1040--2025.txt:36-37` cites are exact.

## N-4 (Nit) — a KAT comment that misdescribes its own assertion

`kat_digital_asset_question.rs:390-401`: the variable is `ri2015`, the comment says *"with the year
moved off both events"*, and the code sets `ri2015.tax_year = YEAR` (already `YEAR`) and asserts on
`LedgerState::default()` — i.e. it re-measures row 3 rather than a year move. The assertion is true;
the label is not.

---

## Seams checked clean

1. **The five-row table, re-driven.** Ran the KAT (`the_digital_asset_answer_table_holds_in_all_five_cells`
   PASS) and re-drove the seam independently with a fold-built ledger. Plant: an `Acquire` on
   2024-01-05 followed by a `Dispose` on 2024-09-09, answered `No` →
   `DigitalAssetAnswerContradictsLedger { date: "2024-09-09", venue: "exchange:coinbase:default",
   kind: "a disposition" }`, and the detail names all three plus both carve-outs. **The purchase is
   never nameable**: `first_digital_asset_event` (`return_1040.rs:2578-2634`) ranges only over
   disposals ∨ income ∨ removals, so no ordering accident can point the filer at a buy.
   `digital_asset_activity = None` blocks commit and appears in `interview_state.blocking`
   (`step0_panel.rs:401`), and `screen_inputs` refuses it with `DigitalAssetActivityUnanswered`
   naming `btctax income answer`.
2. **The predicate is the instruction's list.** Plant: income-only (a mining receipt, no disposal)
   → refuses, `{ date: "2024-07-07", venue: "swan", kind: "digital assets received as income" }`;
   buys + a `TransferLink`ed self-transfer, built by the real fold → no refusal, box prints `No`
   (KAT row 5, whose premise is asserted). Year boundary is the event's own date: disposals on
   2023-12-31 and 2025-01-01 do **not** contradict a 2024 `No` (plant, PASS). Unmatched outflows
   fold to `PendingOut`, not `Disposal` (`project/resolve.rs:43-47`), so an unreconciled
   self-transfer cannot refuse a truthful `No` — which matters, because R9 promises authoring in
   parallel with an unresolved ledger.
3. **Tier placement.** The rule is the first block of `screen_compute_dependent`
   (`return_1040.rs:954-990`) and is absent from `screen_inputs_tiered`. Plant: a
   `DigitalAssetAnswerContradictsLedger` block added at the top of `screen_inputs_tiered` →
   `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` **FAILS**
   (`return_refuse.rs:6450`), with `DigitalAssetAnswerContradictsLedger` present in the
   source-derived census and absent from the fixture table. Reverted; census green.
   `income import` runs `screen_param_free` only (`cmd/tax.rs:272`), so a TOML stating
   `digital_asset_activity = false` imports on any ledger, and the compute paths
   (`resolve_and_screen`, `screen_full_return`) both call `screen_compute_dependent` on the same
   `ReturnInputs`.
4. **The panel writes nothing.** `step0_panel(&LedgerState, &[LedgerEvent], Option<&ReturnInputs>, i32)
   -> Step0Panel` — every input is a shared reference, so a write is a compile error rather than a
   test. No plant is meaningful and none was needed. Both call sites are reads:
   `cmd/answer.rs:373-388` (`s.project()` + `load_all` + `write_step0`, no `save`) and
   `tui-edit/src/main.rs:836-846` (over `app.snapshot`). No `save_draft`, `return_inputs::set`,
   `record_answer` or `BrokerReporting` insertion anywhere in `step0.rs`. The standing-order pair
   passes as committed, and part (c) of it independently confirms the engine calls the day-before
   case `ComplianceStatus::StandingOrder`. (Caveats: I-1 on the venue list, M-1/M-2 on the
   standing-order row.)
5. **The printed box (full-return path).** `assemble_absolute` sets `digital_asset_answer:
   ri.digital_asset_activity` (`return_1040.rs:2537`), `packet.rs:718` passes it through, and
   `form1040_full.rs:364-390` writes `da_yes` on `Some(true)`, `da_no` on `Some(false)`, neither on
   `None`, failing loud on a half-populated map.
   `the_1040_digital_asset_box_prints_the_filers_answer_including_no` asserts the **on-states**
   (`1` / `2`), not merely that something was written. `None` cannot reach that emitter:
   `screen_full_return` (`admin.rs:1516-1522`) runs `screen_inputs` (which refuses `None` via the
   registry loop) then `screen_compute_dependent`, before any byte. `admin.rs:440`'s hand-mark is
   now keyed on `is_none()` and so is silent on an answered return — the pre-T6 version fired on
   every no-crypto packet. (The slice path is C-1.)
6. **No `reconcile` question in a registry.** `xtask stop-list` → *"8 btctax-input-form sources, 4
   state-bearing sources and 64 registry prompts scanned; no forbidden shape"*. Plant: the word
   `lot` inserted into `digital_asset_prompt` → **RED**, *"RENDERED_PROMPTS DigitalAssetActivity:
   says \"lot\""*, i.e. the extension is genuinely reading the rendered surface, not merely
   producing it. The anti-vacuity assertion at `r15_stop_list.rs:330-345` holds the count. Step 0's
   rows are `Step0Row` strings, not `FormQuestion`s. T4b's opener seeds the answer `None`
   (`step0_panel.rs:514`), and `neutral` has no production consumer — `cmd/answer.rs:1103` is inside
   `#[cfg(test)]`.

**Other build-report claims verified.** Deviation 1 (global election) — see (b) above, defensible.
Deviation 3 (`reconcile_digital_asset_activity`, one-directional) — matches the code
(`tax/testonly.rs:101-106`) and is test-only. Deviation 4 (LIMITATIONS.md → T12) — accurate; the note
exists as `step0::VENUE_GRANULARITY_NOTE` and is printed by `income answer`. The
`normalize.rs:63-69` citation is exact (`exchange_wallet` with its doc comment). Collateral fix 1
(the stop list) — verified red above. Collateral fix 2 (the clipped modal tail) — `wrap_to` +
`lines.len()`-derived height at `draw_edit.rs:2174-2200`, with `step0_lines()` single-sourcing the
status block's height at `:2022-2027`. Follow-up *"`hand_marks`' Digital Assets entry is now
unreachable in production"* — true on the full-return path; **not** the reassurance it reads as,
because the slice path never builds a `PrintedReturn` at all (C-1).

**Suite.** 3038 passed / 8 skipped / 7 failed across the six crates the diff touches; the 7 are
exactly the declared PDF-less-worktree set (`form_delta` × 6 +
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`). No T6 test
fails.

Counts: C=1 I=1 M=4 N=4
