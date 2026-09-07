# T6 — the exchange seam (R9). Implementation report.

**Built by** the single implementer for interview build task T6, in the shared main tree
`/scratch/code/bitcoin_tax` on `main`, from HEAD `2fe4ba5f`. Brief:
`design/agent-reports/BRIEF-build-interview-T6.md`. Nothing committed or pushed; `CONTINUITY.md` and
`design/ROADMAP_STATUS.md` (modified by the controller, not by this task) were left alone.

**Status: all five numbered deliverables landed.** Every kill was seen RED on a planted defect and
the plant reverted from a `cp` backup. Whole tree green, `cargo fmt --all --check` clean, and
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
clean.

---

## 1. The Step 0 status panel

**New module `crates/btctax-cli/src/step0.rs`** — a `btctax-cli` function over the held session
(`step0_panel(state, events, ri, year) -> Step0Panel`), no new store, writes nothing.

Five lists, each derived from the thing it reports on rather than re-implemented:

| list | derivation |
|---|---|
| ledger blockers, by kind | `state.blockers` grouped by `BlockerKind`, with `kind.severity()`; the exit comes from `blocker_handoff`, an **exhaustive `match` with no wildcard** so a new `BlockerKind` is a compile error rather than a row that silently loses its exit. Unresolved **import and decision conflicts are two of these kinds** — deliberately not a second list, because a second derivation of the same rows is a second thing to keep in step. |
| imports per venue | walks `EventId::Import { source, .. }` over the events — derived from the events, never a typed venue list |
| venues vs Form 1099-DA answers | `broker_key_census(form_8949(state, year))` — **the same keys `screen_broker_reporting` demands answers for** — joined against `ri.broker_reporting.answer(..)` |
| standing orders (Notice 2026-20 §4.02(2)) | per Exchange wallet's **first** custodial disposition of the year, `btctax_core::standing_order_in_force(..)`, which calls the **shared `resolve_election`** the fold uses to pick the filed basis |
| owner actions with dates | removals in the year with `appraisal_required` (§170(f)(11)(C); CCA 202302012), named with the removal's own date |

**New core function** `btctax_core::standing_order_in_force(events, wallet, date) -> Option<TaxDate>`
(`crates/btctax-core/src/project/compliance.rs`), plus a factored-out `voided_set(events)` so
`disposal_compliance` and the new accessor cannot disagree about which elections are still live.

**Rendered in three places, all off one derivation:**

- `income answer` prints it **before the census** (`cmd/answer.rs`) — document-first order untouched,
  because Step 0 is status, not a question. A projection failure is a note, never a refusal (R9:
  authoring proceeds with an unresolved ledger).
- the TUI tax-inputs **entry screen** — `TaxInputsFormState.step0`, cached at open from the same
  snapshot `broker_census` is read from, rendered by `Step0Panel::tui_lines()` in the status block.
- the **commit modal** — see item 4.

**Nothing gates.** No refusal was added here; commit and export stay hard-gated by the gates that
already exist (`resolve_full_return`'s `TaxYearNotComputable` and the `screen_*` chain), and their
tests stay green.

### The panel's row list, as printed on a fixture

Fixture: Coinbase buy+sell 2026, River buy+sell 2026, one unclassified Swan inbound. Verbatim
`write_step0` output:

```
── Step 0: your ledger, for 2026 ──
  LEDGER BLOCKERS (1):
    • 1 × UnknownBasisInbound (HARD — commit and export refuse while it stands)
      → btctax reconcile classify-inbound-income | classify-inbound-gift | classify-inbound-self-transfer <event>
  IMPORTS BY VENUE (3):
    • coinbase: 2 imported event(s)
    • river: 2 imported event(s)
    • swan: 1 imported event(s)
  VENUES vs FORM 1099-DA ANSWERS (2):
    • coinbase has 1 covered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file — a venue you disposed on is not accounted for
      → btctax income answer --year 2026  (the Form 1099-DA block)
    • river has 1 covered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file — a venue you disposed on is not accounted for
      → btctax income answer --year 2026  (the Form 1099-DA block)
  STANDING ORDERS (Notice 2026-20 §4.02(2)) (2):
    • exchange:coinbase:default: your first custodial disposition of 2026 is dated 2026-03-10, and no standing order (a dated method election) was in force for it — expect box 1g to reflect the broker's default; a `basis_differs` answer refuses; `btctax reconcile select-lots` / `import-selections` is the exit
      → btctax config --set-forward-method <hifo|fifo|lifo> --exchange exchange:coinbase:default (forward only — §1.1012-1(j) allows no post-hoc identification, so this cannot cover the 2026-03-10 sale)
    • exchange:river:default: your first custodial disposition of 2026 is dated 2026-04-11, and no standing order (a dated method election) was in force for it — expect box 1g to reflect the broker's default; a `basis_differs` answer refuses; `btctax reconcile select-lots` / `import-selections` is the exit
      → btctax config --set-forward-method <hifo|fifo|lifo> --exchange exchange:river:default (forward only — §1.1012-1(j) allows no post-hoc identification, so this cannot cover the 2026-04-11 sale)
  (Step 0 is STATUS, not a question — authoring works with an unresolved ledger; committing and exporting do not.)
```

The TUI's compact form for the same panel (wrapped at 112 columns, three named rows then an explicit
overflow line — nothing is hidden silently):

```
Step 0 (2026): 1 blocker kind(s) · 2 venue(s) needing a Form 1099-DA answer · 2 venue(s) with no standing
    order · 0 owner action(s). `btctax verify` and `btctax income answer` print them in full.
• coinbase has 1 covered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file — a venue you
      disposed on is not accounted for
• river has 1 covered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file — a venue you
      disposed on is not accounted for
• exchange:coinbase:default: your first custodial disposition of 2026 is dated 2026-03-10, and no standing
      order (a dated method election) was in force for it — expect box 1g to reflect the broker's default; a
      `basis_differs` answer refuses; `btctax reconcile select-lots` / `import-selections` is the exit
  …and 1 more — `btctax income answer` lists every one.
```

---

## 2. `digital_asset_activity: Option<bool>` — the class-(A) question and its cross-check

- **`ReturnInputs.digital_asset_activity: Option<bool>`**, `#[serde(default)]`, `Default` = `None`.
- **`QuestionId::DigitalAssetActivity`** appended at the end of the enum, of `QuestionId::ALL` and of
  `FORM_QUESTIONS` (index 40) — the `decl_tristate!` array-index discipline preserved.
- **Always live** (`live: |_ri| true`), `Durability::PerYear`, `neutral: false`.
- **Prompt is RENDERED** (`RENDERED_PROMPTS` + `digital_asset_prompt`) so it quotes the return's own
  year, as the form's sentence does. Transcribed from `f1040--2025.txt:36-37` with the
  instruction's two carve-outs quoted after it (`i1040gi--2025.txt:1385-1394`), because a btctax
  ledger is full of exactly those two states (buys, wallet-to-wallet moves) and a filer reading their
  ledger without them in front of them would be pushed toward a wrong *Yes*.
- **`RefuseReason::DigitalAssetActivityUnanswered`** (raised by the registry loop in
  `screen_inputs_tiered`'s `unanswered_refuses` tier) and
  **`RefuseReason::DigitalAssetAnswerContradictsLedger { date, venue, kind }`**.
- **Classifier row** `c.declaration(digital_asset_activity, QuestionId::DigitalAssetActivity)`.
- **`FieldId::DeclDigitalAssetActivity`** + `decl_tristate!(40, …)` in the `Declarations` section;
  the `QuestionId ↔ FieldId` maps stay total; `coverage.rs`'s pinned leaf→FieldId map gains
  `"digital_asset_activity" → DeclDigitalAssetActivity`.
- **`attribute.rs`** anchors both new reasons on the declaration (neither is `NotInForm` — the form
  can reach both, and the contradicted one's second exit is named in its own text).
- **The compute-dependent rule** is the FIRST rule in `screen_compute_dependent`
  (`return_1040.rs`), with a new `first_digital_asset_event(state, year) -> (date, venue, kind)`
  helper derived from `digital_asset_activity`'s own three disjuncts and ordered `(date, event id)`.
- **The off-ledger warning** is `Advisory::DigitalAssetYesNotOnLedger { year }`, fired from
  `advisories()` on `answer == Some(true) ∧ !digital_asset_activity(state, year)` — reading the
  **same predicate** the refusal reads, so the two directions cannot disagree about which cell of the
  table a return is in.

### The printed box is the ANSWER

- `PrintedInputs.digital_asset_activity: bool` → **`digital_asset_answer: Option<bool>`**, fed from
  `ri.digital_asset_activity`.
- `Form1040Lines.digital_asset_yes: bool` → **`digital_asset_answer: Option<bool>`**.
- `form1040_full.rs` writes `da_yes` on `Some(true)`, **`da_no` on `Some(false)`**, and neither on
  `None`. A year whose 1040 carries no digital-asset question at all (2017 — the map omits both)
  prints neither whatever the answer says; a map with one box and not the other fails loud.
- `map.rs`'s `da_no` doc comment — *"never checked by btctax"* — corrected; it is checked now.
- `admin.rs`'s `hand_marks` is conditioned on `digital_asset_answer.is_none()` rather than on the box
  being unchecked. It fired on **every** no-crypto packet before T6, telling a plain wage earner to
  hand-mark a mandatory question the tool had never asked; it is now a fail-closed backstop that says
  nothing on a return that answered.

### The five-row table's outputs

`crates/btctax-core/tests/kat_digital_asset_question.rs::the_digital_asset_answer_table_holds_in_all_five_cells`

| # | ledger | answer | measured outcome |
|---|---|---|---|
| 1 | a 2026-06-15 Coinbase disposal | `No` | **REFUSE** `DigitalAssetAnswerContradictsLedger { date: "2024-06-15", venue: "exchange:coinbase:default", kind: "a disposition" }`; the message contains all three |
| 2 | same | `Yes` | no refusal; printed box = `Some(true)`; **no** off-ledger advisory |
| 3 | empty | `No` | no refusal; printed box = **`Some(false)`** |
| 4 | empty | `Yes` | **no refusal**; printed box = `Some(true)`; `Advisory::DigitalAssetYesNotOnLedger` present, and its message names *8v* and *8949* |
| 5 | two `Acquire` events + a `TransferLink`ed self-transfer, **built by the real fold** | `No` | no refusal; printed box = `Some(false)` |

Row 5's premise is asserted, not assumed: the fold is driven and the test fails unless
`disposals`/`income_recognized`/`removals` are all empty **and** `lots` is non-empty — so the row
measures btctax's fold against the instruction's carve-outs rather than asserting a hand-built empty
state against itself. Row 5's mirror (`Yes` on that ledger) is also measured: warns, does not refuse.

A second test pins that the named event is the **earliest** one: with income recognized on 2024-03-02
before the June disposal, the payload is `{ date: "2024-03-02", venue: "swan", kind: "digital assets
received as income" }`.

---

## 3. The standing-order warning (Notice 2026-20 §4.02(2))

Implemented as item 1's fourth list, keyed on each Exchange venue's **first** custodial disposition
of the year — the moment §4.02(2) asks about (*"entered into the taxpayer's books and records
**before** the units covered by the order are sold"*). The consequence and the exit are stated now,
in R9's own words, and the handoff says *forward only*, because §1.1012-1(j) allows no post-hoc
identification and an election can never be back-dated (`MethodElectionBackdated`).

Fixture pair (`step0_panel.rs`): a 2026 Coinbase disposal with **no** scoped election fires exactly
one row naming the venue, the date, *box 1g*, *basis_differs* and *select-lots*; the same ledger with
an election **effective the day before** is silent. The test additionally asserts the engine itself
calls that second disposal `ComplianceStatus::StandingOrder`, so the panel's silence is the engine's
verdict rather than a coincidence.

---

## 4. Venue-vs-answer listing in the panel and the commit modal

`commit_summary_with_step0(ri, shadows, &form.step0)` appends the panel's unanswered-venue rows and
its standing-order rows to the TUI commit modal's payload summary — the **same** `step0_panel` rows
the entry screen and `income answer` print, never a second derivation.

**A defect found and fixed while wiring it:** `draw_tax_inputs_modal` sized its box from the
**logical** line count while its `Paragraph` word-wraps, so any summary line longer than the modal
was drawn on more rows than were reserved and the tail fell off the bottom. Harmless while the
summary was three short lines; the moment the venue listing rode along, the whole standing-order
block was clipped. Fixed by pre-wrapping (`wrap_to`) so `lines.len()` is exact and the `Wrap` is a
no-op. The same class was fixed in the status block, which **clips** rather than wraps: `tui_lines()`
hard-wraps at 112 columns and the layout height is computed from the same function.

---

## 5. Venue / account granularity — documented, not asked

`btctax_cli::step0::VENUE_GRANULARITY_NOTE`, printed beside the standing-order list in
`income answer`:

> btctax models ONE account per venue (`exchange:<venue>:default`). §1012(c)(1) applies the basis
> conventions account by account, so if you hold more than one account at a venue, the standing order
> above is recorded for the venue and not per account — check that your broker applied the same
> method to each.

The account segment is hardcoded `default` at `crates/btctax-adapters/src/normalize.rs:63-69`
(verified: `exchange_wallet` occupies exactly those lines). **No field, no question was added**, and
a test asserts the negative half a printed sentence cannot prove: no registry prompt asks *"how many
accounts"* / *"which account"*.

---

## Kills, each seen RED once (verbatim)

Plants were applied to the working tree and reverted from `cp` backups; no `git checkout`/`restore`/
`stash` was used.

**P1 — delete the DA cross-check from `screen_compute_dependent`:**
```
thread 'the_digital_asset_answer_table_holds_in_all_five_cells' panicked at
crates/btctax-core/tests/kat_digital_asset_question.rs:269:50:
a `No` the ledger contradicts must refuse
```

**P2 — make the mirror refuse (the symmetric cross-check R9 forbids):**
```
assertion `left == right` failed: ★★★ an unwitnessed `Yes` may NEVER refuse: the ledger is not
complete by construction, and refusing would leave `No` as the only way through the gate
  left: Some((DigitalAssetAnswerContradictsLedger { date: "", venue: "", kind: "planted symmetric refusal" }, "planted"))
 right: None
```

**P3 — the printed box goes back to the ledger predicate (the pre-T6 defect):**
```
assertion `left == right` failed: a filer with no receipts and no disposals must be able to PRINT "No"
  left: None
 right: Some(false)
```

**P4 — delete the off-ledger warning:**
```
thread 'the_digital_asset_answer_table_holds_in_all_five_cells' panicked at …:321:10:
an unwitnessed `Yes` is accepted WITH the off-ledger warning
```

**P5 — the emitter goes back to "Yes or nothing"** (`btctax-forms`, the NEXT SURFACE):
```
assertion `left == right` failed: ★★★ a `No` answer checks the NO box. Before T6 this box was never
written by btctax, so a filer with no digital-asset activity signed a return with a MANDATORY question blank
  left: None
 right: Some("2")
```

**P6 — the standing-order row never fires:**
```
assertion `left == right` failed: one custodial venue with no standing order: []
  left: 0
 right: 1
```

**P7 — the standing-order row always fires (no discrimination):**
```
a standing order recorded the day BEFORE the sale IS the §4.02(2) identification — warning here would
contradict the engine's own `StandingOrder` verdict: [Step0Row { what: "exchange:coinbase:default: your
first custodial disposition of 2026 is dated 2026-03-10, and no standing order …" }]
```

**P8 — a venue with rows and no answer is silently dropped:**
```
assertion `left == right` failed: []
  left: 0
 right: 2
```

**P9 — the RENDERED prompts stop being scanned by the R15 stop list** (the pre-T6 blindness):
```
0 of 4 RENDERED prompts were scanned — the words a filer is SHOWN are the ones this check exists to
read, and a static-only scan reports success over a region it cannot see
```

**P10 — a banned word planted inside a RENDERED prompt** (invisible to the pre-T6 scan):
```
R15/R9: a return-registry prompt asks a LEDGER question — those belong to `reconcile`, and the
interview never re-asks one:
  RENDERED_PROMPTS DigitalAssetActivity: says "lot"
```

**P11 — the commit modal sizes itself from unwrapped lines again:**
```
★★★ the LAST line of the listing must be drawn — a box sized from unwrapped lines clips exactly this:
… │ • river has 4 noncovered row(s) on this year's Form 8949 and NO │ │Form 1099-DA answer on file — a
venue you │ │disposed on is not accounted for │ … └──────┘   (SENTINEL_TAIL absent)
```

**P12 — the Step 0 lines stop being wrapped for the pane:**
```
a Step 0 line was CLIPPED by the pane — the panel said it and the filer cannot read it.
  wanted: • river has 4 noncovered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file
          — a venue you disposed on is not accounted for
  screen: … │ • river has 4 noncovered row(s) on this year's Form 8949 and NO Form 1099-DA answer on file —│
          │ • exchange:river:default: your first custodial disposition of 2024 is dated 2024-05-01, and n│
```

**P13 — `income answer` stops printing Step 0:**
```
thread 'income_answer_prints_step_0_before_the_first_census_question' panicked at
crates/btctax-cli/tests/step0_panel.rs:640:10:
`income answer` prints the Step 0 ledger panel
```

### The remaining brief kills, all landed and green

| kill | where |
|---|---|
| the five-row DA table, as ONE test | `btctax-core/tests/kat_digital_asset_question.rs` |
| the standing-order fixture pair | `btctax-cli/tests/step0_panel.rs` |
| the unnamed-venue fixture | `btctax-cli/tests/step0_panel.rs` (both directions: named when unanswered, listed-as-answered when answered) |
| the grep-KAT on registry prompts, **extended** to the rendered set | `xtask/src/r15_stop_list.rs` (+ its own anti-vacuity assertion, P9/P10) |
| `digital_asset_activity = None` blocks commit and is listed by `interview_state` | `btctax-cli/tests/step0_panel.rs` |
| the answer survives an `income import` round-trip (stated both ways; omitted ⇒ `None`, never a defaulted `No`) | `btctax-cli/tests/step0_panel.rs` |
| T4b's opener seeds it `None` (year N answered `Yes` ⇒ year N+1 is blank) | `btctax-cli/tests/step0_panel.rs` |
| every blocker row carries a `reconcile` exit | `btctax-cli/tests/step0_panel.rs` |
| the emitter join: `Some(true)`→`da_yes`, `Some(false)`→`da_no`, `None`→neither, **on-states asserted** | `btctax-forms/tests/full_return_forms.rs` |
| Step 0 rendered on the TUI entry screen with nothing clipped | `btctax-tui-edit/src/draw_edit.rs` |
| the commit modal draws the whole listing including its last line | `btctax-tui-edit/src/draw_edit.rs` |
| venue/account granularity is documented, not asked | `btctax-cli/tests/step0_panel.rs` |

---

## Deviations from the brief, recorded

1. **The standing-order row honours a GLOBAL election too.** R9 says *"no `MethodElection` with
   `--exchange` scope effective before it"*. `standing_order_in_force` calls the shared
   `resolve_election`, whose tier 2 also honours a global election — so a filer with a global standing
   order recorded before the sale is **not** warned. Reason: they have made the §4.02(2)
   identification, the fold already treats that disposal as `StandingOrder`, and warning them would
   contradict the engine's own verdict while naming no action they have not taken (an election cannot
   be back-dated). Using a narrower predicate here would also be a **third** copy of a rule that
   already has two callers.

2. **The crypto-slice worksheet's box falls back to the ledger predicate when there is no stored
   return.** On the full-return path the printed box is the answer, full stop (`None` cannot reach a
   filed return — `screen_inputs` refuses it). The `admin.rs` slice arm can run with **no**
   `ReturnInputs` at all, and making the box `None` there would regress a correctly-checked *Yes* to
   a blank on a filer who has authored nothing. The answer wins where one exists; where none does,
   the pre-T6 never-`No` behaviour is kept. That path emits a watermarked worksheet, not a return.

3. **`FormQuestion::neutral` for the DA question is `false`, and a fixture reconciler was added
   rather than changing it.** `answer_all_live_declarations` cannot see the ledger, so it answers the
   neutral `No` — which on a crypto fixture is precisely the contradiction the new rule refuses. The
   fix mirrors the existing `reconcile_document_census`: a new
   `testonly::reconcile_digital_asset_activity(ri, state, year)`, **one direction only**
   (`None`/`Some(false)` → `Some(true)` when the ledger witnesses), so a fixture's deliberate
   `Some(true)` survives. That one-directionality is not cosmetic — `extension.rs`'s pseudo-reconcile
   fixture stores its return **before** `pseudo_set_mode` is switched on, so the projection the
   helper can see holds no disposal while the projection the export computes does; a two-way flip
   re-answered it `No` and the export refused on the fixture instead of on the attestation gate.

4. **No `LIMITATIONS.md` / man-page edit.** T12 owns the docs surface (`SPEC_interview.md` §7 row
   T12). The granularity sentence lives in `step0.rs` as `VENUE_GRANULARITY_NOTE` and is printed by
   `income answer`; T12 should carry it into `LIMITATIONS.md`.

5. **`docs/examples/examples.md` and the three j6 TUI walkthrough goldens were regenerated** by their
   own committed generators (`cargo run -p xtask -- examples`,
   `emit_btctax_tui_edit_walkthrough_goldens`). The only content change to `examples.md` is
   `"digital_asset_activity": true` in the J6 `income show` dump; the TUI goldens gained the Step 0
   lines and the modal's venue listing. Both regenerations are re-asserted by their `regen ==
   committed` tests, and both diffs were read before being accepted (a golden cannot validate its own
   regeneration).

---

## Pinned numbers moved

| pin | old → new | cause |
|---|---|---|
| `questions.rs::every_question_id_is_in_all_in_order_and_has_exactly_one_entry` — `QuestionId::ALL.len()` and `FORM_QUESTIONS.len()` | **40 → 41** | `QuestionId::DigitalAssetActivity` |
| `spec/mod.rs::declarations_section_delegates_…` — `decl_count` | **20 → 21** | the new `Decl*` field |
| `spec/mod.rs::declarations_section_delegates_…` — `decls.fields.len()` | **21 → 22** | 21 declarations + `foreign_country_names` |
| `spec/coverage.rs` — `field_count` | **182 → 183** | the new `Field` |
| `spec/coverage.rs` — `covered.len()` | **181 → 182** | the new distinctly-covered leaf |
| `tax_report.rs::a_re_import_keeps_every_answer_record_already_on_the_row` — `before.len()` | **34 → 35** | the interview writes one more `AnswerRecord` |
| `export_irs_pdf.rs::a_no_crypto_packet_names_the_marks_…` — `rep.hand_marks.len()` | **3 → 2** | the Digital Assets hand-mark is gone: the question is answered and the box prints |

---

## Fixture edits forced by the new class-(A) declaration

The new always-live declaration and its ledger cross-check reached 53 CLI tests, 48 core tests and
one core-fixture chain. Every edit is a fixture stating an answer its own ledger supports, never a
relaxation of the rule:

- `btctax-core`: `return_refuse::tests::ri()` and the `dividend_subset_inconsistency_refuses`
  `answered()` closure answer it `Some(false)` (those fixtures carry no ledger at all);
  `testonly::kitchen_sink_household` and `testonly::build_golden_return` call the new
  `reconcile_digital_asset_activity` against **their own** ledgers.
- `btctax-cli`: one shared helper per test file (`full_return_vault`, `answered_for`,
  `give_full_return_inputs`, …) reconciles from `s.project()`; three TOML fixtures state the answer
  (`fullreturn_inputs.toml` regenerated by its emitter, `nine_dependents_amt_inputs.toml`,
  `J10_FULLRETURN_TOML`); `extension.rs`'s pseudo fixture answers `Some(true)` explicitly, with the
  reason recorded inline.
- `btctax-forms`: two `Form1040Lines` literals move to `digital_asset_answer: Some(true)`.

---

## Validation

`cargo nextest run --locked --no-fail-fast -p <crate>`, one crate at a time (never `cargo test`,
never `--release`, never a whole-workspace run):

```
btctax-core             Summary [ 0.546s] 1281 tests run: 1281 passed, 0 skipped
btctax-cli              Summary [ 6.588s]  791 tests run:  791 passed, 1 skipped
btctax-input-form       Summary [ 0.045s]   70 tests run:   70 passed, 0 skipped
btctax-forms            Summary [ 5.553s]  355 tests run:  355 passed, 4 skipped
btctax-tui-edit         Summary [ 2.116s]  391 tests run:  391 passed, 2 skipped
btctax-tui              Summary [ 1.607s]  160 tests run:  160 passed, 2 skipped
xtask                   Summary [ 6.373s]  157 tests run:  157 passed, 1 skipped
btctax-oracle-harness   Summary [ 3.156s]    5 tests run:    5 passed, 1 skipped
btctax-adapters         Summary [ 0.025s]  103 tests run:  103 passed, 0 skipped
btctax-store            Summary [ 0.988s]   45 tests run:   45 passed, 0 skipped
```

3,358 tests, 0 failures. `cargo fmt --all --check` → clean.
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
→ `Finished`, no diagnostics. `scripts/pii-scan-generic.sh` → `pii-scan: clean (HEAD)`; every
PII-shaped token in the files this task touched (`222-33-4444`, `123-45-6789`, `987-65-4321`,
`000-00-0001`, `11-1111111`, `22-2222222`, `33-3333333`, `44-4444444`) is already on the script's
token-exact allow list, and no new identifier was minted.

## Follow-ups worth filing (not built here)

- **FR — the granularity note belongs in `LIMITATIONS.md`** (owning phase: T12). It is printed by
  `income answer` today and has no docs home.
- **FR — `hand_marks`' Digital Assets entry is now unreachable in production.** `screen_inputs`
  refuses a `None`, so no filed packet can carry an unanswered box. It was kept as a fail-closed
  backstop with a rewritten sentence; a later pass may decide the dead branch is worth deleting.
- **FR — the Step 0 panel is computed per `income answer` invocation and per TUI open**, and it
  re-projects the ledger. That is cheap today; if a very large vault makes it noticeable, the TUI
  already caches it and the CLI could too.
