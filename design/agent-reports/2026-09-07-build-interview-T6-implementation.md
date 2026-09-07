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
| 1 | a 2024-06-15 Coinbase disposal | `No` | **REFUSE** `DigitalAssetAnswerContradictsLedger { date: "2024-06-15", venue: "exchange:coinbase:default", kind: "a disposition" }`; the message contains all three |
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

**3,358 tests** across the ten crates listed above — a SUB-TOTAL, not the workspace total: the
workspace is twelve crates and measured **3,363 run / 12 skipped** at this commit. 0 failures.
`cargo fmt --all --check` → clean.
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

---

## Fold (seam review C-1, I-1, M-1 … M-4, N-1 … N-4)

**Folded by** the single implementer in the shared main tree `/scratch/code/bitcoin_tax` on `main`,
from HEAD `2363a44c`. Brief: `design/agent-reports/BRIEF-fold-interview-T6-review.md`; review:
`2026-09-07-build-interview-T6-review.md`; ledger: `…-review-VERIFICATION.md`. Nothing committed or
pushed, no subagents, no `git checkout`/`restore`/`stash` — every plant was reverted from a `cp`
backup. `CONTINUITY.md`, `FOLLOWUPS.md`, `design/ROADMAP_STATUS.md` and `design/SPEC_interview.md`
carry uncommitted controller edits and were left untouched.

**All nine findings folded.** M-2 is FR-77's; what landed here is its fixture and an honest label.

---

### C-1 — the slice prints the ANSWER, never the ledger

**`btctax-forms`.** `Form1040Inputs.da_yes: bool` → **two fields**:
`digital_asset_answer: Option<bool>` (the filer's testimony) and `reportable_activity: bool` (the
ledger's produce/skip question). They were one `bool`, which is exactly how *"the ledger says nothing
happened"* and *"the filer said No"* became the same fact. `fill_form_1040_capgains` now writes the
`da_yes` cell on `Some(true)`, the **`da_no` cell on `Some(false)`**, and **neither on `None`**; the
produce/skip branch reads `reportable_activity`. The map-independent adjacency guard resolves BOTH
members itself now — with `None` neither field is touched by the write pass, so the old
`expect("checked above")` would have panicked instead of failing loud. `map.rs`'s `da_present` doc
comment (*"the fill answers it 'Yes'"*) corrected.

**`btctax-cli`.** On the slice path (`export_irs_pdf_from_session_with_regime`):

- `let da_answer = working.as_ref().and_then(|ri| ri.digital_asset_activity);` — the working return
  the arm already resolved at `admin.rs:783`, read once and used by every gate below it.
- **The cross-check runs before any byte**, calling
  `btctax_core::tax::return_1040::screen_digital_asset_answer` — the SAME function
  `screen_compute_dependent` runs first (extracted for this, so the slice and the full return cannot
  disagree about whether a filer's own `No` is contradicted). A contradicted `No` refuses in the
  slice's own sentence, naming the reason and the detail; `wrote_nothing` is asserted.
- **The mirror**: `advisories: Vec::new()` is gone. The arm raises
  `Advisory::DigitalAssetYesNotOnLedger` when
  `return_1040::digital_asset_yes_is_off_ledger(ri, state, year)` — the predicate `advisories()`
  itself now calls, so the asymmetry stopped being arm-dependent.
- **The hand mark**: the slice's `hand_marks` carries one entry when the page was written and the
  answer is `None`. The sentence is a shared `const DIGITAL_ASSET_HAND_MARK` (the full-return
  packet's `hand_marks` uses the same string), so the two surfaces cannot describe one blank cell
  two ways. `main.rs` gained the reader it needed: the pointer at `manifest.txt` stays on the
  full-return path, and on the slice — which writes no manifest — the sentences are printed
  themselves. Without that the list would have been a figure with no reader.

**Deviation from the brief, recorded.** The brief and spec R6 scope the cross-check to **arm (2)**;
the fold gates it on the **answer** instead (`if let Some(ri) = working.as_ref()`), so it also
covers arm (3). Reason: since this fold the box is printed from `da_answer` on *every* arm this
function reaches, and a params-bundled year whose only return is a DRAFT falls to arm (3) with the
answer present and readable — gating on the arm would leave the identical defect standing one branch
over. It is a widening of a refusal, never of an exemption, and it fails closed.

**The full-return path is unchanged**: `assemble_absolute` → `packet.rs` → `form1040_full.rs` were
not touched, and `the_1040_digital_asset_box_prints_the_filers_answer_including_no` stays green.

#### PDF read-back — the slice's box, off the emitted `form_1040_capgains.pdf`

Read through `Form1040Map::ty2025()` (the map the fill wrote from), TY2025 templates with the LIVE
regime injected — the review's own method. Measured:

| stored answer | ledger | outcome | `da_yes` | `da_no` | `hand_marks` |
|---|---|---|---|---|---|
| `Some(true)` | a 2025 `cb` round-trip | prints | **`Some("1")`** | `None` | 0 |
| `None` | a 2025 `cb` round-trip | prints | **`None`** | **`None`** | **1** (names the box) |
| `Some(false)` | a 2025 `cb` round-trip | **REFUSES**, `wrote_nothing` | — | — | — |
| `Some(true)` | nothing in 2025 (events shifted to 2024) | prints, **`Advisory::DigitalAssetYesNotOnLedger { year: 2025 }`** | — | — | — |
| `Some(false)` / `Some(true)` / `None` | (emitter direct, `reportable_activity: true`) | `Some("1")`/`None` · `None`/`Some("2")` · `None`/`None` | | | |

Before the fold every one of the first three printed **`da_yes = Some("1")`**.

★ **`Some(false)` cannot print on this path, and that is sound, not a gap.** The page is produced
only when the ledger has reportable activity, and every disjunct of `reportable_activity` is a
disjunct of `digital_asset_activity` — so a printable `No` is always a contradicted `No`, and it
refuses. The `Some(false)` → `da_no` join is therefore measured at the emitter
(`btctax-forms::sp2`), where the premise can be held.

**Two committed fixtures moved, and the move IS the defect made visible.**
`export_irs_pdf::ty2024_real_ledger_fills_box_c_f_and_line7_and_da` and
`sp2_packet_writes_schedule_se_and_1040_capgains` both export a vault holding a ledger and **no
`ReturnInputs` at all** — nobody has answered — and both asserted `Digital-Asset question = YES`.
They now assert **neither box** (both members read, because *"the Yes box is off"* and *"neither box
is on"* are different facts) plus the hand mark that names the blank.

---

### I-1 — the venue list obeys the year's Form 1099-DA regime

`step0_panel(state, events, ri, year, regime: Option<InformationReturnRegime>)`. The `venues` list is
built only when `broker_question_is_live(&rows, regime)` — `screen_broker_reporting`'s own gate,
**called**, not re-implemented. An unknown regime (no bundled record) is treated as not live: a panel
that guessed would name venues on a year whose question may not exist.

On a non-live year with exchange rows the panel is not silent either — it emits ONE row, **with no
exit**, stating the fact that decides it, following `screen_broker_reporting`'s wording
(*"TY2024's Form 1099-DA regime reports nothing (no Form 1099-DA is issued for this year) — no Form
1099-DA question is asked for this year and an answer would be refused as unread, so your disposals
on coinbase are boxed by mechanism, not by an answer"*).

**Callers.** `cmd/answer.rs` uses `year_readiness::regime_for` — **not** `regime_or_refuse` as the
brief said: a status panel may not refuse, and `None` is the honest unknown it states. (The brief
named `admin.rs` as a caller; `admin.rs` does not call `step0_panel` — the two call sites are
`cmd/answer.rs:379` and `tui-edit/src/main.rs`, where `regime` was already joined and only needed
moving above the `step0_of` closure.)

**The commit modal's false gate is gone by construction.** `commit_summary_with_step0` lists exactly
the venue rows WITH an exit, so on a non-live year the heading *"VENUES WITH NO FORM 1099-DA ANSWER
(commit is not blocked by this; the EXPORT is)"* — which asserted a gate that does not exist there —
never prints. Pinned from both sides: the panel side in `btctax-cli`, the modal side in
`btctax-tui-edit::draw_edit`.

**The TY2024 walkthrough goldens are the proof.** `docs/examples-tui-walkthrough/j6/` is a **TY2024**
journey, and all three files carried the defect: the entry screen said *"river … a venue you disposed
on is not accounted for"* and the commit modal said *"the EXPORT is"*. Regenerated by
`emit_btctax_tui_edit_walkthrough_goldens`; both are gone, and the diffs were read before being
accepted.

---

### M-1 — the standing-order row obeys Notice 2026-20's relief period

`RELIEF_PERIOD_FIRST_YEAR = 2025` / `RELIEF_PERIOD_LAST_YEAR = 2026` and a three-way
`ReliefPeriod::{Before, Within, After}`, cited to §3.03 at
`legal/text/irs-guidance/Notice_2026-20.txt:299-301` (the sentence ends on `:301`, not `:300`),
§4.01 at `:305-309` and §5 at `:388-394`.

- **Within** — the row is byte-identical to before; the citation and the exit stand.
- **Before** (pre-2025) — the row keeps its substance and states *"Notice 2026-20's relief period
  BEGINS 2025-01-01 (§3.03), so §4.02(2)'s standing-order relief does not reach this sale, and
  btctax refuses an election effective before that date as back-dated"*, and its handoff is the
  `Pre2025MethodNote` exit taken **from `blocker_handoff(BlockerKind::Pre2025MethodNote)`** — the
  exhaustive match, not a second literal. The old row named `--set-forward-method`, which
  `method_election_is_forward` refuses outright on such a year.
- **After** (post-2026) — *"the relief period ENDED 2026-12-31 (§3.03), and §5 says taxpayers may
  not rely on §4.02's relief for sales made after it"*; the forward-election exit stands.

**Whole-surface sweep — the heading was a second copy of the same claim.** `write_step0`'s section
title AND the TUI commit modal's *"NO STANDING ORDER (Notice 2026-20 §4.02(2))"* both asserted the
authority. They now read `Step0Panel::standing_orders_authority()` — one derivation, each surface
keeping its own grammar. The TY2024 golden shows the modal correcting itself.

---

### M-2 — the same-day fixture, honestly labelled (the rest is FR-77)

`the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_the_day_before` →
`…_on_or_before_the_sale`, with a new **part (d)**: the election MADE at `2026-03-10 18:00 UTC` and
effective `2026-03-10`, six hours AFTER the 12:00 sale. Measured: **SILENT**, and the assertion says
why — `resolve_election` compares `effective_from <= disposed_at` on a day-granular `TaxDate`, so
the last day of §4.02(2)'s window fails OPEN. `resolve_election` was **not** touched: it decides the
filed basis through `disposal_compliance`, and that is FR-77's, on the method-election track. The
doc comment now says *"on or before"* and names FR-77 and the §4.02(1)-vs-(2) distinction.

---

### M-3 — the granularity negative test reads the RENDERED prompts

`no_registry_question_asks_about_venue_or_account_granularity` now chains `RENDERED_PROMPTS`,
rendered against a probe carrying the fixture's year, alongside the two static registries — and
labels each prompt by its registry identity so a hit names the surface that said it. Two
anti-vacuity assertions: the walk scanned > 50 prompts, and **every** rendered prompt is in the
scanned set (dropping the chain leaves a smaller set that still clears the floor — the exact way the
checker went blind before).

---

### M-4 — Step 0 surfaces a contradicted `No`; the commit gate is unchanged, by decision

`Step0Panel` gains `contradictions: Vec<Step0Row>`, built by calling
`screen_digital_asset_answer` — the rule the export refuses on, so the panel cannot say "clean"
while the export refuses. Printed first after the blockers under **`YOUR ANSWERS vs THE LEDGER`**,
and it leads the TUI's named rows. The row names the event and says what will happen
(*"commit is not blocked by this, but `report` and `export-irs-pdf` REFUSE until one of the two is
corrected"*) — more precise than "commit will be refused", because commit is exactly what is NOT
refused.

The tier boundary is recorded **in the code at the commit site** (`input_form_store::commit`): that
site holds no `LedgerState`, and projecting the vault there would make committing depend on a ledger
R9 promises may still be unresolved. *"The gate did not move; the silence did."*

---

### N-1 … N-4

- **N-1** — `disposal_compliance`'s ~25-line doc block (the `SelfTransfer` scope boundary, the NFR4
  determinism note, *"Read-only"*) moved back above `disposal_compliance`; `voided_set` keeps its own
  four-line comment. No `#[must_use]` was added (it would red every caller that drops the result).
- **N-2** — folded into this report above: the five-row table's row 1 now reads
  **`a 2024-06-15 Coinbase disposal`** (matching the fixture and the quoted payload), and *"3,358
  tests"* is presented as the ten-crate **sub-total** with the workspace figure named beside it.
- **N-3** — measured against the extract and corrected everywhere the strings appear
  (`questions.rs`, `return_refuse.rs`, and the KAT's module doc — the same citations, so the same
  defect):
  - the mandatory-answer sentence `i1040gi--2025.txt:1398-1400` → **`:1399-1401`** (it begins on
    `:1399` — *"send you Form 1099-DA. You must answer…"* — and ends on `:1401`).
  - the carve-outs `:1385-1394` → **`:1382-1391` + `:1395-1396`** (the *"The following actions or
    transactions…"* sentence is `:1382-1384`; `:1392-1394` are a blank line, the page number `16`,
    and another blank; the third bullet continues at `:1395-1396`).
- **N-4** — the KAT now **tests what it said**. It kept the same `LedgerState` and screened an EMPTY
  one under a comment claiming the year had been moved. It now moves the year off both events
  (disposal → `2023-12-31`, income → `2025-01-01`) with a premise assertion that the events are still
  there, asserts no refusal and `printed_box == Some(false)`, and keeps the empty-ledger case
  separately as the different measurement it is.

---

## Kills, each seen RED once, RUNNING the instrument (verbatim)

Plants applied to the working tree and reverted from `cp` backups. (File:line as captured, before the
final `cargo fmt`.)

**F1 — the emitter's box goes back to "always Yes"** (`digital_asset_answer` → `Some(true)`):
```
thread 'form_1040_digital_asset_box_is_the_answer_yes_no_or_neither' panicked at
crates/btctax-forms/tests/sp2.rs:414:9:
assertion `left == right` failed: answer Some(false): the Yes box
  left: Some("1")
 right: None
```

**F2 — the SLICE's box goes back to the ledger predicate** (`digital_asset_answer: Some(reportable_activity)`):
```
thread 'the_slice_prints_the_digital_asset_answer_and_never_the_ledger' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1490:9:
assertion `left == right` failed: answer None: the Yes box on the emitted PDF
  left: Some("1")
 right: None
```

**F3 — delete the slice's DA cross-check** (the arm-(2)/(3) gate never runs):
```
thread 'a_contradicted_no_refuses_the_slice_and_writes_nothing' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1544:6:
a `No` the ledger contradicts must refuse: IrsPdfReport { … form_1040_path:
Some("/tmp/.tmpILCNSc/slice/form_1040_capgains.pdf") … advisories: [], … hand_marks: [] }
```

**F4 — the slice's `advisories` goes back to `Vec::new()`:**
```
thread 'an_off_ledger_yes_prints_the_slice_with_the_advisory' panicked at
crates/btctax-cli/tests/slice_from_answers.rs:1594:5:
…and it is WARNED: []
```

**F5 — drop the Step 0 regime gate** (`let live = true`):
```
thread 'the_venue_row_fires_only_on_a_year_whose_form_1099da_question_is_live' panicked at
crates/btctax-cli/tests/step0_panel.rs:475:13:
TY2024 asks no Form 1099-DA question, so no venue can be 'not accounted for' and no row may hand the
filer to a block that would refuse them: [Step0Row { what: "coinbase has 1 noncovered row(s) on this
year's Form 8949 and NO Form 1099-DA answer on file — a venue you disposed on is not accounted for",
handoff: "btctax income answer --year 2024  (the Form 1099-DA block)" }]
```

**F6 — the relief period is always `Within`:**
```
thread 'the_standing_order_row_cites_the_notice_only_inside_its_relief_period' panicked at
crates/btctax-cli/tests/step0_panel.rs:544:9:
assertion `left == right` failed: TY2024: §4.02(2) is asserted as the authority only inside its
relief period: …
  left: true
 right: false
```

**F7 — the Step 0 contradiction row is never emitted:**
```
thread 'the_step0_panel_names_a_digital_asset_answer_the_ledger_contradicts' panicked at
crates/btctax-cli/tests/step0_panel.rs:630:5:
assertion `left == right` failed: []
  left: 0
 right: 1
```

**F8 — a banned phrase planted inside a RENDERED prompt** (invisible to the pre-fold static scan —
`digital_asset_prompt` only, the static fallback untouched):
```
thread 'no_registry_question_asks_about_venue_or_account_granularity' panicked at
crates/btctax-cli/tests/step0_panel.rs:1035:9:
R9: account granularity is documented, not asked — this prompt asks it: RENDERED_PROMPTS
DigitalAssetActivity: which account? at any time during 2026, did you: (a) receive …
```

**F9 — drop the `RENDERED_PROMPTS` chain from that test** (the anti-vacuity half):
```
thread 'no_registry_question_asks_about_venue_or_account_granularity' panicked at
crates/btctax-cli/tests/step0_panel.rs:1020:5:
assertion `left == right` failed: every RENDERED prompt must be IN the scanned set — the words a
filer is SHOWN are the ones this check exists to read
  left: 0
 right: 4
```

**F10 — the DA event finder loses its year filter** (`.year() == year` → `.year() >= 1900`), the N-4
guarantee:
```
thread 'the_refusal_names_the_earliest_qualifying_event_and_fires_exactly_when_one_exists' panicked at
crates/btctax-core/tests/kat_digital_asset_question.rs:407:5:
assertion `left == right` failed: a 2023-12-31 disposal and a 2025-01-01 receipt are not 2024 events
— the cross-check reads the event's own date, and a `No` for 2024 is truthful about 2024
  left: Some(DigitalAssetAnswerContradictsLedger { date: "2023-12-31", venue:
"exchange:coinbase:default", kind: "a disposition" })
 right: None
```

---

## Pinned numbers moved

| pin | old → new | cause |
|---|---|---|
| workspace test total | **3,363 / 12 skipped → 3,371 / 12 skipped** | eight tests added by this fold (1 `btctax-forms`, 6 `btctax-cli`, 1 `btctax-tui-edit`) |
| `btctax-core` | 1281 → **1281** | N-4 rewrote a test in place; no count change |
| `btctax-cli` | 791 → **797** | +6 (`the_slice_prints_the_digital_asset_answer_and_never_the_ledger`, `a_contradicted_no_refuses_the_slice_and_writes_nothing`, `an_off_ledger_yes_prints_the_slice_with_the_advisory`, `the_venue_row_fires_only_on_a_year_whose_form_1099da_question_is_live`, `the_standing_order_row_cites_the_notice_only_inside_its_relief_period`, `the_step0_panel_names_a_digital_asset_answer_the_ledger_contradicts`) |
| `btctax-forms` | 355 → **356** | +1 (`form_1040_digital_asset_box_is_the_answer_yes_no_or_neither`) |
| `btctax-tui-edit` | 391 → **392** | +1 (`the_commit_modal_names_no_unanswered_venue_when_the_row_carries_no_exit`) |

No production constant, threshold or census count moved. `xtask stop-list` still reports
**8 / 4 / 64** (unchanged: no registry prompt was added or removed).

---

## Goldens regenerated (by their own committed generators, diffs read before acceptance)

- `docs/examples-tui-walkthrough/j6/{01-tax-inputs-sections,02-tax-inputs-w2,03-tax-inputs-commit}.txt`
  — `emit_btctax_tui_edit_walkthrough_goldens`. Content change: the false TY2024 venue row and the
  modal's *"the EXPORT is"* claim are gone; the standing-order row and the modal heading follow the
  relief period; the summary line gains the contradictions count. `regen == committed` re-asserted.
- `docs/examples/examples.md` — **unchanged**, and checked rather than assumed: no journey prints a
  Step 0 panel (`grep -c "Step 0" → 0`), and the hand-mark line at `:799` is the full-return path's,
  which this fold did not touch. `examples_golden_matches_committed` green without regeneration.

---

## Validation

`cargo nextest run --locked --no-fail-fast -p <crate>`, one crate at a time (never `cargo test`,
never `--release`, never a whole-workspace run):

```
btctax-core             Summary [ 0.532s] 1281 tests run: 1281 passed, 0 skipped
btctax-cli              Summary [ 6.891s]  797 tests run:  797 passed, 1 skipped
btctax-input-form       Summary [ 0.042s]   70 tests run:   70 passed, 0 skipped
btctax-forms            Summary [ 5.159s]  356 tests run:  356 passed, 4 skipped
btctax-tui-edit         Summary [ 2.092s]  392 tests run:  392 passed, 2 skipped
btctax-tui              Summary [ 1.588s]  160 tests run:  160 passed, 2 skipped
xtask                   Summary [ 6.289s]  157 tests run:  157 passed, 1 skipped
btctax-oracle-harness   Summary [ 3.142s]    5 tests run:    5 passed, 1 skipped
btctax-adapters         Summary [ 0.024s]  103 tests run:  103 passed, 0 skipped
btctax-store            Summary [ 0.994s]   45 tests run:   45 passed, 0 skipped
btctax                  Summary [ 0.000s]    0 tests run:    0 passed, 0 skipped
btctax-update-prices    Summary [ 0.003s]    5 tests run:    5 passed, 1 skipped
```

**3,371 tests run, 0 failures, 12 skipped — the whole workspace, twelve crates.** The seven
environment failures the review recorded (6 × `form_delta` + `harness_check`) do **not** appear in
this tree: the PDFs are present here, so those tests ran and passed.

`cargo fmt --all --check` → clean. `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace
--all-targets --all-features -- -D warnings` → `Finished`, no diagnostics.
`cargo run -p xtask -- stop-list` → *"8 btctax-input-form sources, 4 state-bearing sources and 64
registry prompts scanned; no forbidden shape"*. `scripts/pii-scan-generic.sh` → `pii-scan: clean
(HEAD)`. No new identifier was minted; the fixtures reuse `222-33-4444`, already on the script's
token-exact allow list.

---

## Follow-ups worth filing (not built here)

- **FR — the DA prompt's quoted carve-outs are not machine-checked.** N-3 corrected two citations by
  hand against `pdftotext` output; `xtask prompt-check`'s clause table covers only the Form 8615
  clauses, so nothing would have red on the wrong span and nothing will red if the extract is
  re-generated with different line breaks. The instrument exists — it needs three more rows.
  (Owning phase: T12, the docs/consistency surface.)
- **FR — `Step0Panel` has six lists and `write_step0`/`tui_lines` name each one by hand.** Adding a
  seventh is not a compile error in either renderer, which is the shape `blocker_handoff`'s
  exhaustive match exists to avoid one level down. A `#[non_exhaustive]`-style enumeration, or one
  `sections()` accessor both renderers walk, would make an omission fail to compile.
