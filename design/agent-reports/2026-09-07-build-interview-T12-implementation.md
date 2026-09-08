# T12 build report — the render pass (R12): the panel in the TUI, the commit modal's lists, the manifest block, the docs

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatched at HEAD
`5743ecd6`. No subagents. Nothing committed, nothing pushed. No `git checkout --` / `restore` /
`stash`: every plant was reverted from a `cp` backup, with `find crates -name '*.rs' -exec touch {} +`
after **both** the plant and the restore (FR-90).

---

## Closing gate

```
$ make check
make check exit: 0
     Summary [  18.971s] 3529 tests run: 3529 passed, 12 skipped
```

**3510 → 3529 passed** (+19), 12 skipped unchanged. Every added test is named below with the defect
it was watched red on.

```
$ cargo fmt --all                                   # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
                                                    # clean (also run inside `make check`)
$ make docs                                         # OK — NO diff to any committed man page,
                                                    #      byte-stable on a second run
```

### The five instruments — old → new

| instrument | at `5743ecd6` | now | cause |
|---|---|---|---|
| `line-coverage` | 375 / 18 forms / 31 exceptions (ratchet 31) / 0 unverifiable / 17 not-line-bound | **identical** | T12 adds no form line |
| `census-join` | 274 across 13 maps | **identical** | T12 adds no map cell |
| `stop-list` | 8 input-form sources + 4 state-bearing sources, 91 prompts | **8 + 4 + 1 renderer source, 91 prompts** | T12 added a **fourth** R15 check, `progress_widgets`, over `draw_edit.rs` — see FR-98 below |
| `prompt-check` | 88 assertions | **identical** | no prompt gained a citation; the one prompt whose TEXT changed (FR-83) carries none |
| `box-census` | 268 / 19 editions / 9 returns | **identical** | T12 transcribes no box |

No other pinned number moved.

---

## 1. The answer-panel pane in the TUI

**`p` toggles it, from the entry screen and from every section** (§4.1: *"a pane visible from every
section"*). It renders `interview_state()` — with the year's package when there is one — through
**`btctax_cli::panel_lines`, the same function `income answer` prints**.

- `crates/btctax-cli/src/cmd/answer.rs` — `write_panel` was split into `panel_lines(st, when) ->
  Vec<String>` plus `refusing_lines` / `forgoing_lines`. `write_panel` now writes those lines and
  nothing else; its output is byte-identical (the leading blank line stayed with the CLI, where it is
  framing, and is deliberately not part of `panel_lines`, which a bordered pane draws inside).
- `crates/btctax-cli/src/lib.rs` — the three renderers are re-exported at the crate root. **Required**:
  `btctax-tui-edit`'s KAT-G1 source gate forbids the `cmd::` token in non-test code. All three are
  pure `&InterviewState -> Vec<String>`; the gate's intent (no session-lifecycle fn in the
  held-session editor) is honored, not evaded, and the file says so beside the existing
  `guard_allocation_vs_tranche` / `promote_export_gate` precedents.
- `crates/btctax-tui-edit/src/edit/form.rs` — `TaxInputsFormState::panel_lines()` and
  `commit_panel_lines()`; two new view bits, `panel_open` and `panel_scroll`.
- `crates/btctax-tui-edit/src/draw_edit.rs` — `draw_tax_inputs_panel`, drawn last so it sits above
  everything, including the NI-2 entry screen. Legend on every section gains `[p] answer panel`.

**No progress bar, no "N of M"** (R15). The pane lists items.

**Nothing it renders is unreachable.** The pane pre-wraps and draws a contiguous window, and its
footer names the range: `showing 39–75 of 141 lines · [↑/↓ · PgUp/PgDn] scroll · [p/Esc] close`.
`draw_tax_inputs_form` and `draw_tax_inputs_panel` now take `&mut TaxInputsFormState` — **the clamp
lives in the renderer**, because how many rows a line wraps to is a fact about the pane's width,
which the key handler does not know. Clamping in the handler (against the count of *logical* lines)
was the first draft and is defect (2) below.

Snapshot, 120×40, fresh TY2024 Single (the fixture the kill uses):

```
┌ Answer panel ────────────────────────────────────────────────────────────────────────────┐
│  ── The answer panel (tax year 2024) ──                                                  │
│  BLOCKING (23) — commit waits on these:                                                  │
│    • Can someone claim YOU as a dependent on their return? [this question has not been…] │
│    …                                                                                     │
│  showing 1–37 of 141 lines · [↑/↓ · PgUp/PgDn] scroll · [p/Esc] close                     │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

## 2. The commit modal prints the forgoing **and** refusing lists (J-12, J-17, J-32)

`commit_summary_with_step0` gained a `panel: &[String]` argument, filled from
`TaxInputsFormState::commit_panel_lines()` — the same `interview_state()` walk the pane renders,
rendered by the same two functions. A `Declined` benefit is listed *(declined)* with its size.

**A commit with a non-empty `refusing` is refused by the existing gate — asserted, not added**
(`a_return_the_panel_lists_as_refusing_is_refused_by_the_commit_screen_too`): the same
`k1 = Yes` fixture the panel lists as refusing is what `screen_inputs` refuses.

### ★ A defect T12 introduced, caught by regenerating the J-6 golden and reading it

Once the forgoing list rode along, the commit modal — sized `lines.len() + 2` and clamped by
`centered_rect` — **ran off the bottom of a 40-row terminal, taking `[Enter] commit / [Esc] cancel`
with it**, on the one screen that writes the vault. Fixed three ways, all now held by
`the_commit_modal_keeps_its_legend_on_screen_and_reaches_its_last_line`:

1. the header and the legend are drawn from **outside** the scrolled window, so they cannot be
   scrolled away;
2. the body scrolls (Up/Down/PgUp/PgDn while the modal is open; every other key still swallowed) and
   the legend names the range;
3. **`.wrap(Wrap { trim: false })` was removed from this modal.** Everything is pre-wrapped, so
   `Wrap` was a *second, uncounted* layout: a line exactly as wide as the box was pushed onto a
   second row and the last paragraph line fell off. This is the T6 lesson one surface on, and the
   measurement is recorded in the code beside the fix.

## 3. `report`'s rendering (§4.4)

`crates/btctax-cli/src/render.rs::render_interview_block(ri, params)`, appended to the §6 dual report
**after the existing chains** — and to the NOT-COMPUTABLE branch too, which is where it earns its
keep: a return that will not compute is exactly when a filer wants the panel, the census and the
home-sale decision in one place rather than one refusal at a time.

Every section **renders something already decided**:

| section | the one function that decides it |
|---|---|
| the document census (type → declared / rows) | `DocumentRow::ALL` × `documents.get` × `declared_rows` |
| the answer panel | `interview_state{,_with_params}` → `crate::panel_lines` |
| the home-sale decision and its four answers | `return_refuse::home_sale_decision` — **the same call `screen_inputs` makes** |
| row (7) per dependent | `dependent_gates::credit_column` — the entry point the grid emitter uses |
| the `LEAF_SOURCE` provenance of every collected figure | `provenance::collected_figures` |
| the undated document rows | `provenance::undated_document_rows` — the list the manifest prints |

Two new core primitives:

- **`provenance::collected_figures(ri) -> Vec<CollectedFigure>`** — `leaf_walk::money_leaves` (the
  type-driven detector) joined to `LEAF_SOURCE`, keeping `Document` and `FilerRecords` leaves with a
  non-zero amount. **A zero is not listed**, and the doc comment says why: a `Usd` of `0` and an
  untouched box are the same bytes, so listing zeroes would assert a provenance for boxes that carry
  none.
- **`provenance::mask_payer_tin`** and the private `document_identity`, whose match over
  `DocumentKind` is exhaustive so a new document family is a compile error until its issuer field is
  named. **There is no branch of `document_identity` that returns an unmasked TIN** — masking is a
  property of the function that produces the identity, not a discipline each renderer remembers.

`leaf_walk`'s module doc said *"nothing production reads it"*. That sentence is now false and was
**amended rather than left standing**, with the measurement that justifies the read: 30.5 ms on
`every_money_leaf_household` (170 non-zero leaves), against a `report` run that projects the whole
ledger and computes a return.

As printed (from the committed `docs/examples/examples.md`, a real binary run on a synthetic vault):

```
═══ The interview — tax year 2024 ═══

  Documents received (the census)
    Form W-2                                     YES        1 row(s) transcribed
    Form 1099-INT                                no         0 row(s) transcribed
    …
    a rental or royalty statement (Schedule E)   no         no section holds rows of this type
  ── The answer panel (tax year 2024) ──
    FORGOING (5) — lawful to skip; each one costs YOU, not the Treasury:
      • Are YOU legally blind? (§63(f) additional deduction) — up to $1550
      …
    (35 answered, 46 not applicable to this return)

  Sale of your main home
    sold your main home during the year: no
    DECISION: no sale to report

  Where each figure came from (6 figure(s) with an amount; a box left at zero carries no testimony…)
    w2s[0].box1_wages                              $400000         Form W-2 #1 Front Range Health
```

## 4. The manifest block (J-15)

`cmd/admin.rs::forgoing_block(ri, params)`, written immediately after `hand_marks_block`. The two are
different categories and the order encodes it: the hand-marks are blanks btctax **could not** fill
and the filer must; these are blanks the filer is **entitled** to leave and chose to.

**Why it is not cosmetic:** the `COMPLETE BY HAND` block frames itself as the closed list of
deliberate blanks in the packet, and it was not one. A forgone credit is invisible on the printed
page in a way an unsigned signature line is not.

As printed on the fixture packet (captured from the kill's own export, then reverted):

```
# ── FORGONE — benefits you are lawfully entitled to skip, and did ──
#
# None of these makes the return wrong. Each one costs YOU, not the Treasury, and each is
# here because a blank you chose looks exactly like a blank nobody asked about. An item
# marked (declined) is one you were asked about and passed over — that is provenance,
# not absence, so it stays on the list.
#
# FORGOING (6) — lawful to skip; each one costs YOU, not the Treasury:
#  • Are YOU legally blind? (§63(f) additional deduction) (declined) — up to $1950
#  • YOUR date of birth — up to $1950
#  • Did ANY property you donated this year have strings attached? Answer YES if any of these
#    is true of ANY donation: (a) there is a restriction, temporary or permanent, on the
#    …
```

★ The first capture printed `#  • • Are YOU legally blind?` under a bulleted heading —
`wrap_bulleted` adds its own marker and the shared panel lines already carry one. Fixed: the item's
`• ` is stripped before wrapping and the heading is written as a plain comment line. **The lists are
shared; the chrome is each surface's own.**

## 5. Docs

- **`make docs`** — clean, and **no committed man page changed**: T12 altered no clap doc comment,
  and every `income` subcommand (`answer`, `import`, `open-next-year`, `project`, `clear`, `show`,
  `scrub`) already had its page. Byte-stable on a second run.
- **`crates/btctax-cli/LIMITATIONS.md`** — a new section, *"The interview — what it asks you, and
  exactly where it stops"*: the five panel states in the filer's words (BLOCKING / REFUSING /
  FORGOING with the *(declined)* rule / WAITING / NOT COMPUTED), no progress bar, §2.2's eleven
  excluded families each with its exit, the home-sale branch, the 1099-DA keystroke, **FR-73's
  venue/account granularity note**, no self-custody import, and what the interview never does.
- **The J-6 walkthrough goldens** were regenerated (`make regen-walkthrough`) — every moved line
  explained in §"Pinned numbers and goldens" below.
- **`docs/examples/examples.md`** regenerated: +44 lines, all of them the new interview block on the
  `report --tax-year 2024` example.

---

## The four follow-ups this task owned

### FR-73 — the venue/account granularity note has no docs home → **CLOSED**

Carried into `LIMITATIONS.md`, and held by **two derived kills** rather than by prose:

- `limitations_carries_the_venue_account_granularity_note` takes the two load-bearing tokens
  (`exchange:<venue>:default`, `§1012(c)(1)`) **out of `step0::VENUE_GRANULARITY_NOTE` at test time**,
  so a change to either surface reds.
- `limitations_names_every_excluded_document_family_the_census_refuses` enumerates its expectation
  from `DocumentRow::ALL` filtered by `exit_sentence().is_some()` — never a hand-list.

★ That second kill **red on its first run** and found three designations my hand-written table had
paraphrased: *"Form SSA-1099 / RRB-1099"* for *"Form SSA-1099 or RRB-1099"*, *"Form 1099-NEC /
1099-MISC / 1099-K"* for *"…, 1099-MISC or 1099-K"*, and *"Schedule K-1 (any)"* for *"(any flavour)"*.
The doc now uses the census's own words.

### FR-83 — two wordings of one gate → **CLOSED**

The figureless fallback was phrased as a **question**, and the form seam draws it as a `Field` label
beside an answerable tri-state — so on a params-less year a filer met an answerable question on one
surface and *"cannot be asked yet"* on the other. It is now the waiting wording itself:

> *"Step 4's gross income test (§152(d)(1)(B), Form 1040 instructions) — WAITING ON THE TAX YEAR'S
> PARAMETER PACKAGE. This question quotes the year's own dollar limit, and that figure arrives with
> its tax package; until then the question cannot be stated, so it is not yet yours to answer."*

Two kills, both watched red:

- `a_params_quoting_gates_fallback_is_a_waiting_label_and_its_rendered_prompt_is_the_question` —
  **derived over every `prompt_from_params` gate**, both halves (the fallback must not be a question
  and must say WAITING; the rendered prompt must be a question), so a second params-quoting gate
  added later is covered on the day.
- `the_dependents_pane_and_the_panel_word_the_waiting_gate_identically` — takes the sentence out of
  the **panel's** `waiting` row and looks for it on the **drawn** dependents pane.

### FR-86 — six T8 kills never watched red → **CLOSED, all six red**

Each defect was planted in **production** code, never in the test.

| kill | plant | observed red |
|---|---|---|
| `only_the_two_credit_arms_check_a_box` | `credit_column` maps `NoCreditBox → CreditForOtherDependents` | ``assertion `left == right` failed: NoCreditBox / left: CreditForOtherDependents / right: Neither`` |
| `single_and_mfj_ask_no_hoh_or_qss_question` | `HohQualifyingPerson.live` → `\|_ri\| true` | *"HohQualifyingPerson must not be live on Single — a filing-status test asked of a filer who did not claim that status is a question with no answer"* |
| `a_hoh_test_answered_no_refuses_with_the_exit` | the refusal loses *"CHOOSE ANOTHER FILING STATUS"* | *"a refusal with no exit is a brick: you are filing as HEAD OF HOUSEHOLD and answered NO to one of the instructions' own conditions…"* |
| `the_qss_window_is_derived_from_the_tax_year` | `qss_window_prompt`'s `a = y - 2, b = y - 1` → literal `2023, 2024` | *"TY2026: Qualifying surviving spouse, condition 1: \"Your spouse died in 2023 or 2024 and you didn't remarry before the end of 2026.\""* |
| `the_ty2025_grid_checks_the_more_than_four_box_with_its_statement` | TY2025 emitter's `check(…, !overflow.is_empty())` → `check(…, false)` | ``assertion `left == right` failed / left: None / right: Some("1")`` |
| `every_slot_caption_is_the_forms_own_words` | `SLOT_CAPTIONS`' `"And in the U.S."` → `"And in the United States"` | *"LivedWithYouInUs's caption is not in design/forms/extract/f1040--2025.txt: \"And in the United States\""* |

All six discriminate. **No checker needed changing.**

### FR-87 — the unreachable `s_1099 == Some(true)` arm → **CLOSED: DELETED**

**Decision: delete, and let the census own the outcome with a kill on it.** Reaching the arm would
mean reordering the document census behind a value rule — changing which refusal a filer meets in
order to make a redundant arm testable. That is the wrong trade: the census's §2.2 sentence names
Form 8949 code H and the Pub. 523 worksheet, **the same exit**, one rule earlier.

What landed instead: the whole branch selector moved into a named
`return_refuse::home_sale_decision(ri) -> HomeSaleDecision { NoSale, NotReported, Reported(&str) }`,
which `screen_inputs_tiered` calls **and §4.4's report block reads** — one decider, two readers,
which is also what stopped the report from re-deriving a decision beside the rule that owns it
(the T9/T11 fold shape). The `Some(true)` arm is gone from it; the `s_1099.is_none()` arm **stays**
and is reachable at the param-free (import) tier, where the census's unanswered rule does not run.

The outcome is now held by a kill that was **watched red** — planting the census's own §2.2 rule
away (`if false { … }` on `row.exit_sentence()`):

```
the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523
  (true,true,true,s_1099=Some(true)) refused unexpectedly: None
```

— the exact case that would otherwise file a home sale with a Form 1099-S and no Form 8949. Both
*"a documented fail-closed backstop, not a tested guard"* labels are gone from the tree.

---

## Every kill, with its planted defect and the red

Nineteen tests were added. Each was observed red on a plant in production code, then green after a
`cp` restore + `touch`.

| # | test | planted defect | observed red |
|---|---|---|---|
| 1 | `the_answer_panel_pane_draws_every_line_it_renders_and_names_the_blocking_items` | `panel_lines` drops one blocking row (`.iter().skip(1)`) | *"a blocking item was not rendered: Can someone claim YOU as a dependent on their return?"* |
| 2 | (same) | the pane's scroll clamps by LOGICAL lines instead of wrapped rows | *"the LAST panel line must be reachable by scrolling. wanted: (0 answered, 51 not applicable to this return)"* |
| 3 | `the_pane_marks_a_declined_benefit_declined_and_never_lists_it_as_blocking` | `forgoing_lines`' `let mark = "";` | ``assertion `left == right` failed: the declined benefit is listed once, marked / left: 0 / right: 1`` |
| 4 | `on_a_params_less_year_the_pane_sizes_nothing_and_lists_the_waiting_gate` | the pane sizes a params-less year from TY2024's package | *"a params-quoting gate WAITS rather than blocks: … • Are YOU legally blind? … — up to $1950"* |
| 5 | `the_commit_modal_prints_the_forgoing_and_refusing_lists_with_the_exit` | `commit_summary_with_step0` drops the panel lists | *"the refusing heading is DRAWN, not merely built: ┌ Sections …"* |
| 6 | `the_commit_modal_keeps_its_legend_on_screen_and_reaches_its_last_line` | the pre-fix modal (sized `lines.len()+2`, `Wrap` on) | *"the modal that WRITES THE VAULT must always show how to confirm and how to cancel"* |
| 7 | `a_return_the_panel_lists_as_refusing_is_refused_by_the_commit_screen_too` | the census's §2.2 rule (shared with FR-87's plant) | *"(true,true,true,s_1099=Some(true)) refused unexpectedly: None"* |
| 8 | `the_dependents_pane_and_the_panel_word_the_waiting_gate_identically` | the fallback reverts to an answerable question | *"…and it must read as a wait rather than as something to answer: Dependents #1 …"* |
| 9 | `p_opens_the_answer_panel_from_the_entry_and_from_a_section_and_esc_closes_the_pane_only` | `p` never wired (`Char('P')`) | *"`p` opens the pane at the entry screen too — §4.1 says it is visible from every section"* |
| 10 | (same) | the open pane stops swallowing `Esc` | *"Esc inside a read-only overlay must not drop the filer out of the editor"* |
| 11 | `the_report_block_prints_the_census_and_masks_every_payer_tin` | `document_identity` returns the raw TIN | *"…with its TIN masked: ═══ The interview — tax year 2024 ═══ …"* |
| 12 | (same) | the census loop drops one row (`.skip(1)`) | *"the census gives every row of `DocumentRow::ALL` its OWN line; Form W-2 is missing"* |
| 13 | (same) | the block drops the undated document rows | *"an undated row is NAMED: …"* |
| 14 | (same) | the block drops the answer panel | *"the panel is in the block: …"* |
| 15 | `the_report_block_states_the_home_sale_decision_and_the_answers_behind_it` | the decision hardcoded to `NoSale` | assertion on *"the sale is NOT reported"* |
| 16 | `the_report_block_prints_row_7_for_every_dependent_from_the_flowchart` | row (7) always `CreditColumn::Neither` | *"row (7) is printed per dependent: …"* |
| 17 | `report_tax_year_prints_the_interview_block_after_the_existing_chains` | the block built but never pushed onto `dual_report` | *"the §4.4 block reaches the command's output: ═══ Absolute filed return (Form 1040) …"* |
| 18 | `the_manifest_lists_the_forgone_benefits_marking_the_declined_ones_and_names_the_undated_rows` | `forgoing_block` never appended | *"the manifest carries the forgone block: # btctax full-return packet — staple in this order"* |
| 19 | (same) | `forgoing_lines`' `let mark = "";` | *"…and MARKS the benefit the filer was asked about and passed over, ON ITS OWN ROW, in the words it was asked in"* |
| 20 | `every_collected_money_leaf_is_listed_with_its_source` | `collected_figures` drops the `FilerRecords` half | ``assertion `left == right` failed: the listed set IS the derived set`` |
| 21 | `a_documents_payer_tin_is_masked_in_every_collected_figure` | (shared with 11) | as above |
| 22 | `a_params_quoting_gates_fallback_is_a_waiting_label_…` | (shared with 8) | as above |
| 23 | `limitations_names_every_excluded_document_family_the_census_refuses` | *(none needed — it was red on its FIRST run against the committed doc)* | *"LIMITATIONS.md must name the excluded family Ssa1099 in the filer's own words (\"Form SSA-1099 or RRB-1099\")"* |
| 24 | `limitations_carries_the_venue_account_granularity_note` | derived from the shipped constant | asserts the constant still carries each token before asserting the doc does |
| 25 | `the_progress_widget_check_reds_on_a_gauge_and_not_on_its_near_misses` | three plants (`Gauge`, `LineGauge`, a formatted `%`) and three near misses (a comment, *"7.5% of AGI"*, *"{a} of {b} lines"*) | in-test, all three red |
| 26 | the six FR-86 kills | see the FR-86 table | see the FR-86 table |

### ★★ Two of my own new kills were blind, and the plants found it

Both are FR-88's shape landing on T12's own work, and both are recorded in the code beside the fix:

1. **`the_report_block_prints_the_census_and_masks_every_payer_tin` searched the whole block.**
   Dropping `DocumentRow::W2` from the census loop **passed**, because the *provenance* section a few
   lines down prints *"Form W-2 #1 ACME"*. Scoped to the census section — and then it *still* passed,
   because `contains("Form W-2")` is satisfied by *"Form W-2G"* two rows further down. Now asserted
   per row: some census line must start with the designation followed by whitespace. Two plants to
   find two defects in one assertion.
2. **`the_manifest_lists_…` asserted `manifest.contains("(declined)")`.** The block's own header
   *explains* what *(declined)* means, so deleting the mark from every row still passed. Now: one
   **bullet line** must carry the mark **and** the words the benefit was asked in.

---

## Deviations from the brief, with reasoning

1. **`draw_tax_inputs_form` / `draw_tax_inputs_panel` / `draw_tax_inputs_modal` take `&mut
   TaxInputsFormState`.** Not in the brief. The scroll clamp has to live where the pane width is
   known; clamping in the key handler bounds the cursor by the count of *logical* lines and leaves
   four fifths of a long panel unreachable (measured — kill 2). The repo already uses `&mut` flow
   state in `draw_select_lots_list` and siblings.
2. **The three panel renderers are re-exported at the `btctax-cli` crate root.** Required by KAT-G1;
   the alternative was a second renderer in the TUI, which is the exact thing R12 forbids.
3. **`return_refuse::home_sale_decision` was extracted.** Not in the brief, but §4.4 requires `report`
   to print *"the home-sale decision"*, and re-deriving it beside the rule that decides it is the
   defect T9's and T11's folds were both for. The refusal's reason, detail and tier behaviour are
   unchanged — the home-sale cross test passes untouched over all 24 combinations.
4. **The commit modal now scrolls, and its `Wrap` was removed.** A fix for a defect T12 introduced;
   see §2.
5. **R15's stop list gained a fourth check rather than a wider field walk.** The brief said *"extend
   if prompts/fields grew"*. Fields did grow — `panel_open` / `panel_scroll` — but pointing
   `progress_shaped_fields` at `btctax-tui-edit` reds on `LotPickFormRow::remaining_sat`, the sats
   left in a ledger lot. An excuse list keyed by that name is what `CLAUDE.md` forbids, so the
   renderer gets a check that asks the renderer's own question (`Gauge` / `LineGauge` / a formatted
   percentage). Filed as **FR-98** so the residual — a *stored* progress field on a TUI struct — has
   an owner and a stated mechanism for closing it.
6. **`leaf_walk`'s "nothing production reads it" sentence was amended, not left standing.** Production
   now reads it; the doc says so and carries the measured cost.

---

## Citation drift in the brief's settled-facts block (measured at `5743ecd6`)

The brief warned it was ~60 commits stale. It was.

| cited | actual at `5743ecd6` | drift |
|---|---|---|
| `admin.rs:494-500` (`COMPLETE BY HAND`) | **`:576`** | +76…82 |
| **`docs/LIMITATIONS.md`** | **`crates/btctax-cli/LIMITATIONS.md`** — `docs/LIMITATIONS.md` does not exist | wrong path (the brief hedged *"check the path"*) |
| `answer.rs:412` (the no-brick property, from R12) | `:412` is inside `resolve_answer_target`; the property is documented at **`:68`** and its per-row extension at **`:1349`** | ~344 |
| `return_1040.rs:2468-2475` (`digital_asset_activity`, R9) | **`:2863`** | +395 |
| `advisories.rs:172` (`RefundByPaperCheck`, R14) | variant at **`:242`**, emitted at **`:1376`** | +70 / +1204 |
| `classifier.rs:28,913` (R14's kill) | `:28` correct; the test is at **`:1512`** | +599 |
| `cli.rs:552,555` (§4.2's `income answer` / `set-pii` help) | the `set-pii` mention is at **`:664`**, `Answer {` at **`:667`** | +112 |
| `spec/mod.rs:19-38` (`form_spec`) | `pub fn form_spec()` at `:23` — inside the cited range | none |
| `coverage.rs:88` (`maximal_fixture`) | `:88` | none |
| `FIELD_PROVENANCE.md:108-110`, `:123-125` | both land correctly, at `design/forms/FIELD_PROVENANCE.md` (the spec cites the bare filename) | line numbers exact; path unqualified |

Nothing I relied on was wrong in substance — every anchor resolved to the thing it named once the
line number was re-measured.

---

## Pinned numbers and goldens moved, with cause

| artifact | change | cause |
|---|---|---|
| `make check` | 3510 → **3529** passed; 12 skipped unchanged | 19 new tests |
| `xtask stop-list` summary line | now names *"1 renderer source(s)"* | the fourth R15 check |
| `docs/examples-tui-walkthrough/j6/01-tax-inputs-sections.txt` | one glyph row + one style run | the legend gained `[p] answer panel` (row 37; the DarkGray run 0..92 → 0..111) |
| `docs/examples-tui-walkthrough/j6/02-tax-inputs-w2.txt` | identical single change | same legend |
| `docs/examples-tui-walkthrough/j6/03-tax-inputs-commit.txt` | the commit modal is taller and starts at row 0 | it now carries the FORGOING list (J-12) and the scroll footer: `[Enter] commit [Esc] cancel — writes nothing · showing 1–33 of 36 lines, [↑/↓ · PgUp/PgDn] scroll` |
| `docs/examples/examples.md` | +44 lines | the §4.4 interview block on the `report --tax-year 2024` example |
| `docs/man/*.1` | **no change** | no clap doc comment was touched; `make docs` byte-stable on a second run |

---

## Follow-ups filed

- **FR-97 (new, not blocking)** — ★ **the TUI writes a dependent-gate answer with NO `AnswerRecord`**,
  pre-existing since T7. `btctax-input-form`'s `apply.rs::answer_key_for` returns `None` for all
  twenty `DepGate*` fields, so `Edit::SetField` on any dependent gate sets the leaf and records
  nothing — while `income answer` records them. Consequences: R10.3's re-ask rule is disabled for a
  TUI-answered gate (`interview_state` finds no record and counts the row *answered* under words it
  may never have shown), and the answer carries no date and no prompt hash. Found while closing
  FR-83. **Not fixed here**: it is a writer defect and T12 adds no writer; the fix needs the row's
  `ssn` to build the key, i.e. a seam change. A kill exists to copy — T1's identical-records test,
  widened to the dependent gates.
- **FR-98 (new, ownerless residue)** — the R15 field-name walk still does not reach the TUI's state
  modules, and the reason (the `remaining_sat` false positive) is recorded so the next pass types the
  mechanism rather than naming the exception.

Both are in `FOLLOWUPS.md`; FR-73, FR-83, FR-86 and FR-87 are marked **CLOSED 2026-09-07 (T12
build)** in place, each with its evidence.

---

## Suite lines per crate (final run)

```
btctax-core        1371 tests run: 1371 passed,  0 skipped
btctax-cli          823 tests run:  823 passed,  1 skipped
btctax-tui-edit     400 tests run:  400 passed,  2 skipped
btctax-tui          160 tests run:  160 passed,  2 skipped
btctax-forms        371 tests run:  371 passed,  4 skipped
xtask               170 tests run:  170 passed,  1 skipped
── workspace ──    3529 tests run: 3529 passed, 12 skipped     (make check, exit 0)
```

Nothing is unfinished. The tree compiles, is fmt-clean, clippy-clean at `-D warnings`, `make docs` is
clean and stable, and nothing is committed.
