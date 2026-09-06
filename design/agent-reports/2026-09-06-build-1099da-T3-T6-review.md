# Build review — Form 1099-DA, T3 → T7 ACROSS THE SEAMS (`18d1332b..ee28b889`)

**Reviewer.** Independent adversarial build reviewer, own git worktree, read-only w.r.t. the repo
(no source edited, nothing committed; this file is the deliverable).
**Date.** 2026-09-06.
**Commit reviewed.** `ee28b889` ("1099da(T7) + docs: the owner action under Notice 2026-20 §4.02(2)…"),
HEAD of `main`.
**Tree state.** The worktree opened at `2bd04d45` (154 commits behind `ee28b889`, clean). I detached
it onto `ee28b889` — `git rev-parse HEAD` → `ee28b88999def6653064e742a08f0b2f349dff3f` — and every
line number below is that commit. No other mutation.
**Scope.** `2feb53d0` (T4), `ab0c98f8` (T5), `17753789` (T6-a), `bb6d140d` (T6-b), `820b6d9b` (the r2
fold), `ee28b889` (T7 + docs), and the two continuity commits. T0–T3 were **not** re-reviewed for
correctness-per-commit; only where a T4–T7 hand-off touches them.

**What I ran.**

| command | summary line |
|---|---|
| `cargo run -q -p xtask -- dump-fields crates/btctax-forms/forms/2025/schedule_d.pdf` | Row1b `f1_7..f1_10` at x = 288/360/432/504, y 480–492; Row2 `f1_11..f1_14` at the same x, y 456–468; Row8b `f1_27..f1_30`; Row9 `f1_31..f1_34` |
| `… forms/2024/schedule_d.pdf` | Row1b `f1_07..f1_10`, Row2 `f1_11..f1_14`, Row8b `f1_27..f1_30`, Row9 `f1_31..f1_34`, same four x positions as the known-good Row3/Row10 |
| `cargo nextest run --locked -p btctax-core -p btctax-forms -E 'test(broker) or test(routing_table) or test(line_1b) or test(cohort)'` | **27 tests run: 27 passed, 0 failed, 1525 skipped** (0.169s) |
| `grep -rn "route_8949_boxes" crates/btctax-tui/` | **no hits** — the finding behind I-1 |
| `grep -rn "screen_broker_reporting" --include=*.rs crates/ \| grep -v tests/` | two hits: the definition, and `return_1040.rs:2619` inside `screen_absolute` |
| `grep -rn "seed_broker_rows" crates/` | one production call site, `tui-edit/src/main.rs:928` |

I did **not** re-run the whole workspace suite (already machine-verified green at this commit,
3091 passed).

---

## Seam 1 — T3 page-sets ↔ T4 Schedule D per-box lines

**Checked.** `printed.rs:90-150` (`form_8949_printed`, `by_box`), `:1089-1210` (`schedule_d_lines`),
`schedule_d_full.rs:128-243` (`box_row` + the three column vectors), both `schedule_d.map.toml`
bindings against the PDFs' own widget rectangles, the 2025/2024 form text extracts, `forms.rs:384-431`
(`form_8949`'s part/box pairing), `lib.rs:114-152` and `fill8949_full.rs:98-127` (the two paginators),
`line_coverage.rs:2059-2200` (the exhaustive destructure and the quotes).

**Verdict — sound.** No finding.

- **Every routed box's total lands on exactly one Schedule D line.** `Form8949Box` has eight
  variants (`forms.rs:43-62`). Both the constructor (`forms.rs:400-410`) and the router
  (`forms.rs:118-152`) derive the box from `row.part`, so `st_by_box ⊆ {C,I,G,H}` and
  `lt_by_box ⊆ {F,L,J,K}` by construction. `schedule_d_lines`'s six `of(…)` calls — `{G}`, `{H}`,
  `{I,C}` / `{J}`, `{K}`, `{L,F}` — partition each of those sets exactly. A gain cannot land on 1b
  *and* 3, and no box's total can vanish.
- **The map bindings are right, verified geometrically, not from the commit message.** The 2025
  extract (`f1040sd--2025.txt:31-37, 57-62`) reads 1b = "Box A or Box G", 2 = "B or H", 3 = "C or I",
  8b = "D or J", 9 = "E or K", 10 = "F or L" — exactly the code. `dump-fields` puts Row1b's four
  widgets at the same four x positions (288/360/432/504 = columns d/e/g/h) as Row3's known-good
  `f1_15..f1_18`, one row higher on the page, on both revisions.
- **(g) stays blank on the new rows (R4).** `schedule_d_full.rs` builds `p1_amounts`/`p1_d`/`p1_e`
  from `gain_h`/`proceeds_d`/`cost_e` only; `adj_g` is mapped but never pushed.
- **Cross-footing.** `line7 = 1a_h + 1b_h + 2_h + 3_h − 6` and `line15 = 8a_h + 8b_h + 9_h + 10_h +
  13 − 14`, matching "Combine lines 1a through 6 / 8a through 14 in column (h)" for the lines btctax
  models (4/5/11/12 are unmodelled — pre-existing, outside this range).
- **The kill is real.** `kat_broker_reporting.rs::the_g_total_lands_on_line_1b_not_3` asserts
  1b = (1000,400,600) *and* 3 = (0,0,0), plus both cross-foot identities, plus
  `f.st_by_box[G].proceeds_d == d.line1b_d`. Re-binding 1b to line 3 reds it.
- **Prior round's I-1 is properly folded.** `fill_8949_full_with_map` now groups by `r.box_`
  (`fill8949_full.rs:98-115`) and routes through `place_part`, whose `part.box_cell(letter)` refuses
  an unmapped letter (`fill8949.rs:100-110`) — the full-return filler can no longer launder a G row
  under the I checkbox.

Recorded as **M-3** below (not a defect in the code, a hole in its coverage): no test writes a
non-zero value into 1b/2/8b/9 and reads it back off a PDF.

---

## Seam 2 — T5 (f)/(g) blank + the advisory ↔ T2's routing

**Checked.** `admin.rs:470-514` (`broker_reporting_advisory`), `:917-918` and `:1283-1286` (both
`IrsPdfReport` construction sites), `:1446-1468` (`slice_broker_refusal`), `main.rs:906-913` (the one
emission), `year_readiness.rs:215-251` (`regime_for` / `regime_or_refuse` / `regime_or_pre_regime`),
`fill8949.rs:39-54, 133-140` (the only (f)/(g) writers), `render.rs:1205-1226` (`routed_8949_rows`).

**Verdict — the regime plumbing is clean; one wording defect (M-1).**

- **No path computes the advisory from a constant.** `broker_reporting_advisory` takes
  `InformationReturnRegime` by value; both arms fill `IrsPdfReport.regime` from
  `year_readiness::regime_or_refuse(tax_year)` (`admin.rs:668`, `:1050`), which reads
  `YEAR.toml` and refuses a year with no record. `DIGITAL_ASSET_8949_FIRST_YEAR` no longer appears
  in the advisory or in `tabs/forms.rs`.
- **The row count is right on a live year.** Both arms pass a `box_needs_review` count
  (`possibly_broker_reported`, `forms.rs:472`), and `box_needs_review` is
  `matches!(leg.wallet, Exchange{..})` — the same predicate as `broker_key`'s `Some` case
  (`forms.rs:80-84`). So the count is exactly the number of keyed (routed) rows. Routing does not
  change `box_needs_review`, and the full-return arm reads it off the same `Printed8949` the packet
  printed.
- **(f)/(g) genuinely cannot carry an invented code.** `adjustment_code`/`adjustment_amount` are set
  once, to `String::new()`/`Usd::ZERO` (`forms.rs:414-415`); nothing in the router or either filler
  writes them. The kill is real *at the core level*:
  `every_cell_of_the_routing_table` runs `form_8949` → `route_8949_boxes` and asserts both fields on
  every routed row (`kat_broker_reporting.rs:364-378`), so planting `row.adjustment_code = "B"` in
  the router reds it. (`broker_boxes.rs::a_broker_reported_row_prints_nothing_in_columns_f_and_g`
  builds rows by hand, so it kills the *filler*, not the router — the two together cover R4.)
- **The slice is closed correctly.** `slice_broker_refusal` is a pure predicate over
  `broker_question_is_live`, fires before `mkdir_out`, and the r2 fold's `import_note` suffix is in
  place. The TUI's own CSV export (`btctax-tui/src/export.rs:176-181`) joins the regime and refuses
  before its exclusive `mkdir`. Both of prior-round I-3's CSV call sites are covered.

---

## Seam 3 — T6-a surfaces

**Checked.** `render.rs:1861-1944` (`render_broker_answers`) and its tests at `:4921-5010`;
`cmd/tax.rs:524-534` (the call) and `main.rs:157-168` (the emission); `year_readiness.rs:115-144`
(`sentence`); `tui/src/tabs/forms.rs:62-80` (`broker_box_note`) and `:96-131` (the table);
`forms.rs:88-99` (`broker_key_census`) versus `return_refuse.rs:896-902` (the screen's own census).

**Verdict — the key SET agrees everywhere; two disagreements about what is shown (I-1, I-2, M-2).**

- The report's census is literally `broker_key_census(form_8949(&state, year))`, and
  `screen_broker_reporting` rebuilds the identical map inline from the same `form_8949` rows. The two
  cannot disagree about *which* keys exist, and `report` cannot say "G / J" for a key the packet
  would refuse: `BasisMatches` is the only answer that prints "G / J" and it is exactly the answer
  the router maps to G/J.
- The **box column on the TUI Forms tab is unrouted** — I-1, the sharpest finding in this review.
- Neither `report` nor the TUI **enumerates the rows** under a key, only counts them — I-2, against
  an explicit spec MUST.
- The **converse** case does exist, in the mild direction: a stored answer on a non-live year whose
  key *has* rows refuses as `BrokerAnswerUnread` while the report block prints nothing — M-2, with an
  existing assertion pinning the silence.

---

## Seam 4 — T6-b the input-form block

**Checked.** `spec/sections.rs:1546-1671` (`BROKER_CHOICES`, `broker_get/set/parse`, `BROKER_FIELDS`,
`BROKER_REPORTING`) and its kill at `:1697-1756`; `seam.rs:35, 186-190`; `apply.rs:130-149`
(`row_depth`) and `:155-161` (`guard_arity`); `attribute.rs:64-75, 296-327`; `spec/coverage.rs:129-137,
164, 232, 718-723`; `tui-edit/src/edit/form.rs:220, 325-362, 477-505`; `main.rs:805-931`;
`draw_edit.rs:2404-2450, 2596-2606`; `edit/tax_inputs.rs:545-587, 745-825`;
`classifier.rs:796-812`; `return_refuse.rs:888-985`.

**Verdict — the index question is clean; the seeding wiring has a hole (I-3), plus M-4.**

- **The three "i-th key" sites cannot disagree.** `broker_get`/`broker_set` use
  `.values()`/`.values_mut().nth(i)`, `remove` uses `.keys().nth(i)`, and
  `broker_row_provider` (`spec/mod.rs:44-49`) uses `.keys().nth(i)` — all four over the *same*
  `BTreeMap`, whose `keys`/`values` iterators are guaranteed to be in the same key order. A `remove`
  shifts all four identically, and `apply_edit` → `clamp_focus` (`edit/tax_inputs.rs:552-587`)
  re-derives `live_sections` and re-clamps `section_idx`/`field_focus`, including the case where the
  section disappears entirely. `the_block_reads_writes_clears_and_refuses_hand_rows` exercises
  get/set on row 1 while asserting row 0 is untouched, then removes row 0 and asserts the survivor.
- **An all-`None` entry is inert everywhere, so the seed does not change any outcome.** Verified at
  all four consumers: `classify_broker_reporting` pushes only `Some` slots; `screen_broker_reporting`
  `continue`s on `None`; `route_8949_boxes` treats `answer(...) == None` as `Unanswered` identically
  to an absent provider; `render_broker_answers` inserts a key only `if c.covered.is_some()`.
  `CohortAnswers` carries `skip_serializing_if = "Option::is_none"` on both slots, so the entry
  round-trips through the vault's JSON as `{"coinbase":{}}` and comes back all-`None`. The one place
  it is *not* inert is `section_is_live` (`form.rs:492`), which is its intended purpose.
- **`Unanswered` round-trips through the clear path.** `broker_parse` maps the token to `Ok(None)`
  and `broker_set` assigns `None`; the kill asserts `answer("coinbase", Covered) == None` after the
  write, and `broker_choice(None)` renders it back as `"Unanswered"`.
- **Liveness is *not* consistent across the three.** The seam's field-level `live` is `|_| true` on
  both slots; the TUI's section gate is `!ri.broker_reporting.0.is_empty()`; and the section is
  populated only by a seed that is skipped on the one open where the working copy is `None` — I-3.
  A refusal anchoring `BrokerCovered` while the map is empty resolves to `None` in
  `resolve_field_anchor` (`edit/tax_inputs.rs:762-771`) and moves nothing — M-4.

---

## Seam 5 — the answers' whole path, end to end

`income import` TOML → `ReturnInputs.broker_reporting` → `screen_broker_reporting` →
`route_8949_boxes` → `fill_form_8949` page-sets → `schedule_d_lines` → `report` → TUI.

**Verdict — the computation path is coherent; the READ surfaces are where it splits.**

Three states where two surfaces disagree about the same key:

1. **Live year, keyed row, answer `BasisMatches`.** The packet prints box **G**; the TUI Forms tab
   prints box **I** for that same row, directly above a note saying "boxes G/H/J/K follow your Form
   1099-DA answers". → **I-1**.
2. **Live year, key unanswered.** The export refuses and names the TUI input form as the exit; the
   TUI input form opened for the first time on that year does not contain the block. → **I-3**.
3. **Non-live year (TY2024/TY2025/TY2017), key WITH rows, answer stored.**
   `screen_broker_reporting` refuses `BrokerAnswerUnread`; `render_broker_answers` returns `None`.
   → **M-2**.

And one state where *no* surface can answer the question being asked: on a live year the filer must
answer per (provider, cohort) and nothing enumerates the rows under a key with their column (e) —
`report`'s disposal listing (`render.rs:462-478`) prints sat/proceeds/basis/gain/term but **not the
wallet**, the 8949 CSV *refuses* until every key is already answered
(`render.rs:1212-1225`), and the TUI Forms table has no wallet column. → **I-2**.

---

## Findings

### I-1 (Important) — the TUI Forms tab prints an UNROUTED Form 8949 box on a live year, under a note that says the boxes followed the filer's answers

**Where.** `crates/btctax-tui/src/tabs/forms.rs:96` (`let rows_8949 = form_8949(&snap.state, year);`)
and `:121` (`Cell::from(form8949_box_tag(r.box_))`), rendered by both TUIs
(`btctax-tui/src/draw.rs`, and `btctax-tui-edit/src/draw_edit.rs:226`). The note beneath it is
`tabs/forms.rs:67-80`, added by T6-a.

**What is wrong.** `form_8949` returns rows carrying the *pre-route* box — `I` short-term, `L`
long-term from TY2025 (`forms.rs:400-410`). `route_8949_boxes` is never called anywhere in
`btctax-tui` (`grep -rn "route_8949_boxes" crates/btctax-tui/` → no hits). So on a TY2026 vault with
one coinbase disposition and `[broker_reporting.coinbase] covered = "basis_matches"` stored, the
Forms tab's Box column reads **I** for a row the packet files under **G** — while the footnote two
lines below reads *"NOTE: boxes G/H/J/K follow your Form 1099-DA answers (`report` lists the keys) —
compare column (e) of every G/J row with box 1g…"*. The filer is told to find their G/J rows on a
table that shows none. This is the "what the tool claims it did" shape: a screen asserting a box the
return will not carry.

T6-a **extended `form8949_box_tag` with the G/H/J/K arms** (`:39-43`) — arms that no code path in
either TUI can reach, because nothing routes. That is the green-and-blind signature: a widened
instrument that was never watched discriminating.

**Evidence.**
- `git show 17753789 -- crates/btctax-tui/src/tabs/forms.rs` — the whole diff is the `use` line, the
  `broker_box_note` fn, the four new `form8949_box_tag` arms and the footnote call. `render`'s
  `form_8949(...)` line is untouched.
- The **previous round explicitly handed this forward**:
  `design/agent-reports/2026-09-06-build-1099da-T0-C-review.md`, I-3's Minimal change —
  *"The same question should be asked once of `btctax-tui/src/tabs/forms.rs:76`, which renders box
  tags from unrouted rows (**T6 owns that surface**, so it is not counted here)."* T6-a edited that
  exact file and did not ask it. The analogous fix for the CSV is in the tree
  (`render.rs:761-767, 1205-1226`) and for the TUI's CSV export
  (`btctax-tui/src/export.rs:176-181`); only the on-screen table was left.
- No test asserts the Box column of that table; `the_note_follows_the_regime`
  (`tabs/forms.rs:237-254`) tests the footnote string only. Nothing would red.

**Minimal change.** Either (a) route the rows in `render` — it has `snap`, and the editor's snapshot
already reaches `regime_for(year)` and the stored `ReturnInputs` at `main.rs:819-826`; or (b) apply
`slice_broker_refusal`'s predicate and, on a live year, replace the Box column's letter with `—`
plus a one-line "answer the 1099-DA keys to see the box" caption. Whichever is chosen, land it with
a kill that reds when routing is removed.

### I-2 (Important) — no surface enumerates a key's ROWS, so the filer cannot perform the comparison the answer they are asked for asserts

**Where.** `crates/btctax-cli/src/render.rs:1905-1942` (the `report` block prints
`provider · cohort · rows · answer · box` — a **count**, not the rows);
`crates/btctax-tui-edit/src/draw_edit.rs:2596-2606` (`broker_row_preview` prints
`"— coinbase   covered: 3 row(s) · noncovered: 1 row(s)"` — again a count).

**What is wrong.** Spec R1 makes this a MUST, in the paragraph headed *"What the declaration is
about, and what it cannot see"*: *"The answer quantifies over a row set the engine derived, so the
tool MUST show it: `report` and the TUI prompt **enumerate each key's rows (date sold, amount,
proceeds, btctax's column (e))** before the answer is taken (T6)."* T6 delivered the count and not
the enumeration.

This is load-bearing, not cosmetic. `BasisMatches` is defined (`forms.rs:246-250`, and the field's
own help text at `sections.rs:1625-1631`) as *"Every one is listed with box 2 checked, **and box 1g
equals btctax's column (e) on each**"*. The filer is asked to swear to a per-row equality against a
column the tool never shows them. Under this repo's own rule that an entry is testimony, that is the
worst shape available: the tool solicits a declaration it has made unverifiable.

And there is no fallback surface on a live year:
- `report`'s disposal listing prints sat/proceeds/basis/gain/term but **no wallet**
  (`render.rs:462-478`), so a leg cannot be attributed to a provider, let alone a cohort.
- `export-snapshot`'s `form8949.csv` does carry `wallet`, `proceeds` and `cost_basis` — but on a
  live year it **refuses until every key is already answered** (`render.rs:1212-1225`). Circular.
- The TUI Forms table has no wallet column at all (`tabs/forms.rs:106-131`) — and, per I-1, a wrong
  box.

**Evidence.** Read the two renderers above; run
`grep -n "wallet" crates/btctax-cli/src/render.rs` over `render_disposal_leg` (no hit) and
`crates/btctax-tui/src/tabs/forms.rs:106-113` (the header row is Part/Box/Description/Acquired/Sold/
Proceeds/Basis/Gain). `broker_answers_tests::the_block_lists_keys_rows_answers_and_boxes` asserts
`"    3  basis_matches"` — i.e. the count is what the test was written against.

**Minimal change.** `render_broker_answers` already receives the census; give it the rows instead
(`&[Form8949Row]`) and print, under each key, one line per row: `date_sold`, the description
(BTC amount), `proceeds` (d) and `cost_basis` (e). The TUI's `broker_row_preview` can print the same
lines from `form.broker_census` widened to hold the rows rather than the counts. Kill: a fixture with
two rows under one key renders both dates and both column-(e) figures.

### I-3 (Important) — the Form 1099-DA block never appears on a FIRST authoring session, which is the exit both refusals name

**Where.** `crates/btctax-tui-edit/src/main.rs:926-929` — the seed is guarded by
`if let Some(ri) = form.working.as_mut()`; `:849` — the `Loaded::Fresh` arm sets `working: None`;
`crates/btctax-tui-edit/src/edit/form.rs:492` — `SectionId::BrokerReporting => !ri.broker_reporting.0.is_empty()`;
`crates/btctax-input-form/src/spec/sections.rs:1657` — `add: |_, _| Err(SetError::Immutable)`.

**What is wrong.** `seed_broker_rows` has exactly one production call site, inside
`open_tax_inputs_form`, and it is skipped when the working copy is `None`. A year with no committed
inputs and no draft loads as `Loaded::Fresh` ⇒ `working: None` ⇒ **no seed**. The filer then chooses
a filing status, which materializes a `ReturnInputs` whose `broker_reporting` map is empty (NI-2,
`edit/tax_inputs.rs:881-896`); `section_is_live` therefore hides the block for the rest of that
session, and `add` refuses, so there is no in-session way to create the row.

The journey this breaks is the primary one, and `default_year()` is 2026 after T0, so it is the
year both TUIs open on:

1. TY2026 vault, exchange dispositions, no return inputs.
2. `btctax export-irs-pdf --tax-year 2026` → the slice arm → `slice_broker_refusal`
   (`admin.rs:1462-1467`): *"…`income import` (the `[broker_reporting.<provider>]` table) or **the
   TUI input form**, then export the FULL return…"*.
3. The filer opens the TUI input form for 2026. `Loaded::Fresh`. They pick a filing status. **The
   "Form 1099-DA answers" section is not in the left pane.** The exit named by the refusal is empty.
4. It appears only after the draft is flushed and the form is *re-opened* (`Loaded::Draft` ⇒
   `working: Some` ⇒ seed runs).

Note the slice arm runs *precisely because* no `ReturnInputs` is stored — so the refusal that names
the TUI form and the open that skips the seed are guaranteed to co-occur.

**Evidence.**
- `grep -rn "seed_broker_rows" crates/` → one production call site (`main.rs:928`), one test module.
- `main.rs:849` (`Fresh` ⇒ `working: None`) versus `:872`/`:894` (`Committed`/`Draft` ⇒
  `working: Some`).
- `rows_are_seeded_from_the_ledger_on_a_basis_year_only` (`edit/form.rs:383-416`) tests
  `seed_broker_rows` as a function against a `ReturnInputs` it constructs itself. Nothing tests the
  wiring — no test opens the form on a live year and asserts `live_sections` contains
  `SectionId::BrokerReporting`. Deleting `main.rs:926-929` outright reds nothing.

**Minimal change.** Re-seed after materialization: call `seed_broker_rows` from `apply_edit`'s
success path (or from `clamp_focus`) whenever `form.working` has just become `Some`, using the
already-cached `form.broker_census` and a regime cached on the form state. Kill: open on a basis year
with a non-empty census and `Loaded::Fresh`, choose a filing status, assert
`live_sections(working)` contains `BrokerReporting` and that row 0's provider is the ledger's.

### M-1 (Minor) — the live-year [I5] advisory tells the filer their rows were filed under G/H/J/K even when their answer put them in I/L

**Where.** `crates/btctax-cli/src/cmd/admin.rs:501-505`.

**What is wrong.** The advisory reads only the *row count*, never the answers, so on a live year it
prints, unconditionally: *"N disposition(s) occurred on a venue that issues Form 1099-DA for TY2026;
each was filed under the Form 8949 box your answer chose (G/H short-term, J/K long-term), with
columns (f) and (g) left blank."* A filer who answered `not_reported` for every key has every row on
box **I/L**, and no 1099-DA lists any of them — yet the sentence asserts the venue issued one and
enumerates only the broker-reported boxes. The follow-on instruction ("compare column (e) of every
G/J row with box 1g, and column (d) of every listed row with box 1f") then degenerates to comparing
against a form that does not exist.

**Evidence.** `broker_reported_rows` is `possibly_broker_reported` = the `box_needs_review` count
(`forms.rs:472`), which is `matches!(leg.wallet, Exchange{..})` — independent of the answer. The
advisory's signature takes no answers. `broker_advisory_on_a_live_year_names_box_1g_and_box_1f`
(`admin.rs:1382-1400`) passes only `(2026, PROCEEDS_AND_BASIS, 2)`, so the discrepancy is invisible
to it.

**Minimal change.** Word the parenthetical for what actually happened — either drop it ("under the
box your answers chose") or pass the routed rows' box set and name it. Both export arms have the
routed rows in hand at the call site.

### M-2 (Minor) — `report`'s 1099-DA block is silent on the one unread-answer state its own doc comment names, and an existing assertion pins the silence

**Where.** `crates/btctax-cli/src/render.rs:1874-1890` (`live` / `stored_unread` / the early `None`);
the contradicting refusal is `crates/btctax-core/src/tax/return_refuse.rs:909-939`.

**What is wrong.** `render_broker_answers`'s doc says the block prints when the question is live *"or
an answer is stored that no row reads (which refuses as unread, and the filer should see why)"*. But
`stored_unread` is computed as *"a stored key that is not in the census"* — i.e. only the **no rows**
half. `screen_broker_reporting`'s condition is `if !live || !has_rows`, so it *also* refuses every
stored answer on a year whose regime lacks `basis`, **even when the key has rows**. In that state
`live == false` and `stored_unread == false`, and the block returns `None`.

Concretely: TY2024 vault with coinbase dispositions and `[broker_reporting.coinbase] covered =
"basis_matches"` stored → `report` prints no 1099-DA block, while `screen_absolute` refuses
`BrokerAnswerUnread` ("…the tool would never read this answer, and testimony it would discard is not
kept. Remove the answer."). On TY2025 nothing surfaces it at all, because `full_return_for(2025)` is
`None` so `screen_absolute` never runs on that year.

**Evidence.** `render.rs:4988` asserts exactly this:
`assert!(render_broker_answers(2025, Some(R::PROCEEDS_ONLY), &c, Some(&a)).is_none());` where `c`
contains `("coinbase", Covered, 3)` and `a` contains `("coinbase", Covered, BasisMatches)` — the
state `screen_broker_reporting` refuses. The guarantee has no kill; the test asserts its negation.

**Minimal change.** `let stored_unread = keys.iter().any(|k| !census.contains_key(k)) || (!live && answers.is_some_and(|a| !a.0.is_empty()));`
and fix the pinned assertion to expect the block, with the `— REFUSES as unread` reason for the
not-live case.

### M-3 (Minor) — the four new Schedule D map rows have no read-back kill; a swapped binding would be caught by nothing

**Where.** `crates/btctax-forms/forms/{2024,2025}/schedule_d.map.toml` `line1b`/`line2`/`line8b`/
`line9`; `crates/btctax-forms/src/schedule_d_full.rs:128-243`.

**What is wrong.** The only machine checks on these bindings are existence checks:
`map_pdf_conformance.rs::every_committed_map_field_exists_in_its_own_pdf` (the FQN is in the PDF) and
`kats.rs:47-52` (the FQNs are in the fill's field set). Neither distinguishes `Row1b.f1_7` from
`Row2.f1_11`, nor `proceeds_d` from `cost_e` within a row. No test writes a non-zero 1b/2/8b/9 and
reads the value back off the produced PDF: `full_return_forms.rs:1243-1256` sets all six of them to
`Usd::ZERO` (so `box_row` returns `None` and no write happens at all), the oracle-sweep corpus is
TY2024 where a G/J row cannot exist, and `the_g_total_lands_on_line_1b_not_3` stops at
`ScheduleDLines`. The `box_row` **refusal** path (`need(...)` on an unbound row with a non-zero
total) is likewise never executed.

I verified the bindings by hand against the widget rectangles (`xtask dump-fields`, both revisions —
see the header table), so nothing is wrong today; what is missing is the instrument that keeps it
right. This is exactly a class the repo's own B1 rule exists for.

**Minimal change.** One test per revision: build `ScheduleDLines` with four distinct non-zero
per-box totals, fill Schedule D, and read back that `Row1b.f1_7`/`f1_8`/`f1_10` carry d/e/h of the
1b figures and `Row2.*` the 2 figures (the geometry verifier already resolves rects). Plus a planted
`line1b: None` on a map with a non-zero total, asserting the `UnmappedField` refusal.

### M-4 (Minor) — the four broker refusal anchors cannot fire on any live path, and when they do they land on row 0

**Where.** `crates/btctax-input-form/src/attribute.rs:64-75`;
`crates/btctax-tui-edit/src/edit/tax_inputs.rs:778-827` (`focus_refusal`, `form.addr = RowAddr::default()`);
`crates/btctax-cli/src/input_form_store.rs:317` (the commit gate).

**What is wrong.** Two things, both about an instrument never seen discriminating.

1. The TUI's commit runs `screen_inputs`, and `screen_broker_reporting` is called only from
   `screen_absolute` (`return_1040.rs:2619`) — verified by
   `grep -rn "screen_broker_reporting" --include=*.rs crates/ | grep -v tests/` (two hits). So no
   `CommitOutcome::Refused` can carry a broker `RefuseReason`, and `focus_refusal` has never been
   watched moving on one. `attribute.rs:296-327` tests `attribute()` in isolation, which cannot see
   this.
2. `focus_refusal` resets `form.addr = RowAddr::default()`. `BrokerReporting` is the first repeating
   section whose *row identity is the whole content of the refusal* (the provider), so a refusal
   about `("gemini", Covered)` will focus row 0 — `coinbase`. The refusal detail names the provider,
   so it is recoverable, but the jump is wrong.

**Minimal change.** For (1), leave the anchors and record that they are forward-looking until a live
path produces the refusal, or extend the commit gate. For (2), widen `Anchor::Field` to carry an
optional row (or add `Anchor::Row(SectionId, usize)`) and resolve the provider's index via
`broker_row_provider`.

### N-1 (Nit) — a `Debug` render of `BrokerRouteError` reaches a user-facing message

**Where.** `crates/btctax-cli/src/render.rs:1221` — `…do not settle every row ({e:?})…`. Every other
broker refusal in the tree composes prose. Format the three variants by hand.

### N-2 (Nit) — `line_coverage` quotes the 2024 Schedule D text for lines that the code fills per the 2025 revision

**Where.** `crates/btctax-core/src/tax/line_coverage.rs:2095` (`Coverage::quoting("2024")`) and
`:2107-2200`. Lines 1b/2/8b/9 are quoted as *"…with Box A / B / D / E checked"* — correct against
`f1040sd--2024.txt:33-36, 58-61`, which I checked — while `schedule_d_lines` routes G/H/J/K to them
per the 2025 revision's *"Box A or Box G"*. Pre-existing convention (line 3 is quoted "Box C checked"
while the code puts I there), so a Nit, not a defect; worth a `quoting_year("2025")` variant when the
2026 package lands.

---

**Counts: C=0 I=3 M=4 N=2**

---

## What I did not examine

- **T0–T3 for correctness-per-commit** (`18d1332b` and earlier). Out of scope by the brief; I touched
  `route_8949_boxes`, `cohort_of`, `fill_form_8949` and the T3 page-set rule only where a T4–T7
  hand-off depends on them.
- **The full workspace suite.** Already machine-verified green at this commit (3091 passed); I ran a
  27-test broker-scoped slice instead.
- **The `spec/coverage.rs` 98/98 pin and the scrub-axis leaf policing** beyond reading the broker
  fixture, sentinel, `addr_for` and pin entries — the brief lists them as machine-verified.
- **Any mutation testing.** I planted no defects (read-only worktree); every "would this red?"
  verdict above is from reading what the test constructs and what the production path builds. Where I
  could not settle it that way I said so (M-3, M-4).
- **PDF pixel/appearance rendering** and the golden packets — I verified the Schedule D bindings
  against widget *rectangles*, not against a rasterized page.
- **The T7 owner action itself** (`ROADMAP_STATUS.md` §0a, Notice 2026-20 §4.02(2)) — not code, and
  the brief does not scope the legal adjudication.
- **`FOLLOWUPS.md` / `CONTINUITY.md` bookkeeping accuracy** in `ee28b889`.
- **Non-broker regressions** in the range (the T4 Schedule D fields touch `full_return_forms.rs`
  fixtures; I confirmed the destructure is exhaustive but did not re-audit the full-return packet).
