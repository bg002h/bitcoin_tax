# Fold — build review `2026-09-06-build-1099da-T3-T6-review.md` (0C / 3I / 4M / 2N)

**Implementer.** Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`,
base `e13fa24b`. No subagents. Nothing committed, nothing pushed, no `git checkout --`/`restore`.
**Date.** 2026-09-06.
**Result.** All 9 findings folded. Whole workspace **3105 tests run: 3105 passed, 12 skipped**
(3091 at the reviewed commit → **+14** tests). Clippy `--workspace --all-targets --all-features
-D warnings` clean. `cargo fmt --all` applied.

Every kill below was **watched red** by planting the defect and reverting it, except where the note
says otherwise. Plants were made on scratch copies (`cp` backup → mutate → run → `cp` restore); no
`git checkout` was used.

---

## I-1 — the TUI Forms tab printed an UNROUTED Form 8949 box on a live year

**What changed.**

| file | what |
|---|---|
| `crates/btctax-tui/src/app.rs:129-138` | `Snapshot.broker_answers: BTreeMap<i32, BrokerReporting>` — the stored answers, per year, projected at unlock. |
| `crates/btctax-cli/src/session.rs:583-604` | new typed accessor `Session::broker_reporting_answers()`. Per-year `return_inputs::years` + `get`; a year whose blob will not deserialize is SKIPPED, matching `resolve_all_screened`'s per-year fail-closed rule. |
| `crates/btctax-tui/src/unlock.rs:206-210, 226` | `build_snapshot` fills it **through the accessor**, never `session.conn()` ([R0-I1]). |
| `crates/btctax-tui/src/tabs/forms.rs:82-151` | new PURE fn `routed_box_tags(rows, regime, answers) -> (Vec<(Form8949Row, &'static str)>, Option<String>)`. |
| `crates/btctax-tui/src/tabs/forms.rs:166-176, 195-197, 264-269` | `render` calls it; the Box cell prints the returned tag; the caption goes into the bottom pane under the existing regime note. |
| 12 test `Snapshot` literals in `btctax-tui`, 11 in `btctax-tui-edit` | `broker_answers: Default::default()` (the `E0063` blast radius — the compiler enumerated every one). |

**Behaviour.** Not live (no year record, or a regime without `basis`, or no keyed row) → the rows
exactly as `form_8949` built them, no caption. Live and every key settled → the ROUTED letters.
Live and a key is unanswered / `mixed` / `basis_differs` → `—` on every **keyed** row (self-custody
rows keep their mechanical I/L) plus a one-line caption naming the exit.

**Deviation from the minimal change, and why.** The review offered (a) route, or (b) `—` + caption.
I did **both**, because they are not alternatives: (a) is the correct display when the answers
settle, and (b) is the only honest display when they do not. Two further deviations, both forced:

- The pure fn returns each row paired with its **Box cell text**, not just `rows`, because
  `Form8949Box` has no "undecided" variant and `—` is not a box — inventing one would put an
  unrepresentable state into the domain type.
- The routing attempt runs on a **clone**. `route_8949_boxes` mutates in place and returns `Err` at
  the *first* unsettled key, so on a two-key ledger the rows before the failure are already
  rewritten. Displaying that half-routed vector would be a claim about a return that will not be
  filed. `an_unsettled_key_shows_an_em_dash_and_a_caption` pins exactly this with an `aaa` key that
  routes and a `zzz` key that does not.

**Kills** (`crates/btctax-tui/src/tabs/forms.rs`, `mod broker_route_tests`):

- `a_live_year_shows_the_routed_box_not_the_pre_route_default` — the fixture is pinned as `["I","L"]`
  pre-route, so `G` can only come from routing. Also covers `proceeds_only → H` and
  `not_reported → I` (the same letter, now by answer rather than by default).
- `an_unsettled_key_shows_an_em_dash_and_a_caption` — unanswered, `mixed`, `basis_differs`, and no
  stored answers at all.
- `a_not_live_year_prints_the_rows_as_built` — `None` / `NONE` / `PROCEEDS_ONLY` regimes and a
  live year with no keyed row. **This is the test that holds the two pre-live walkthrough goldens.**

**Seen red:** planted an early `return (tagged(rows), None);` (i.e. "never routes") — the first two
tests failed, the first reporting `left: ["I","I","L"]` against `right: ["—","—","L"]`. Reverted.

---

## I-2 — no surface enumerated a key's ROWS (spec R1 MUST)

### (a) `report`

`crates/btctax-cli/src/render.rs:1867-1997` — `render_broker_answers` now takes
`rows: &[Form8949Row]` **instead of** the census and derives the census itself with
`broker_key_census`. Under each key's summary line it prints one line per row:
`date_sold · column (a) description · (d) proceeds · (e) cost basis`. The block header gained one
sentence naming what the indented lines are and restating that `basis_matches` asserts box 1g equals
column (e) on **each** of them. Call site updated at `crates/btctax-cli/src/cmd/tax.rs:524-535`
(it already computed `rows`).

**Deviation.** The review said "instead of (or in addition to) the census". I took **instead of**:
carrying both would reintroduce the seam the finding is about — a count and the rows it counts,
derived separately, able to disagree. The existing summary line and its assertions are unchanged;
only the test fixture moved from a hand-built census to hand-built rows.

Rendered output (from the test, `--no-capture`):

```
  provider      cohort       rows  answer          box
  coinbase      covered         3  basis_matches   G / J
      2026-01-01  0.00000001 BTC  (d)        1000.00  (e)         100.00
      2026-01-02  0.00000002 BTC  (d)        2000.00  (e)         200.00
      2026-01-03  0.00000003 BTC  (d)        3000.00  (e)         300.00
  coinbase      noncovered      1  (unanswered)    — refuses until answered
      2026-01-04  0.00000004 BTC  (d)        4000.00  (e)         400.00
```

**Kill:** `render::broker_answers_tests::each_keys_rows_are_enumerated_with_their_dates_and_column_e`
— two rows under one key; every row's date, amount, (d) and (e) must appear on one line, and the
fixture asserts the two rows are genuinely distinct so a render that printed the first twice cannot
pass. **Seen red:** planted `.take(1)` on the enumeration loop → FAIL, output showed one line where
two were required. Reverted.

### (b) the TUI input form

| file | what |
|---|---|
| `crates/btctax-tui-edit/src/edit/form.rs:338-376` | new `BrokerProviderRows { covered: Vec<String>, noncovered: Vec<String> }` with `counts()` and `lines(cohort)`. |
| `…/edit/form.rs:378-408` | `broker_census_by_provider` now takes `&[Form8949Row]` (not a census) and renders each cohort's lines. |
| `…/edit/form.rs:410-441` | new pure `broker_row_detail_lines(census, provider) -> Vec<String>` — a heading per cohort with its count, then the rows; an empty cohort says an answer there would be unread; an absent provider says its rows are gone. |
| `…/edit/form.rs:220` | `TaxInputsFormState.broker_census` retyped to `BTreeMap<String, BrokerProviderRows>`. |
| `crates/btctax-tui-edit/src/main.rs:816-826` | the open reads the rows once from the snapshot. |
| `crates/btctax-tui-edit/src/draw_edit.rs:2453-2467` | the row PANE prints the detail lines under the `"Form 1099-DA answers #n — coinbase …"` header, before the two fields. The row LIST keeps the count line (`broker_row_preview`, now via `counts()`). |

The rows are rendered in `edit/form.rs`, not in `draw_edit.rs`, because the renderer may not name a
`ReturnInputs` field or a ledger row (§9A/§13); the provider still comes through the
`broker_row_provider` seam. `tax_inputs_render_never_reads_a_bare_return_inputs_field` still passes.

**Kill:** `edit::form::broker_seed_tests::a_keys_rows_are_enumerated_with_their_dates_and_column_e`.
**Seen red:** planted `.take(1)` inside `broker_row_detail_lines` → FAIL. Reverted.

---

## I-3 — the block never appeared on a FIRST authoring session

| file | what |
|---|---|
| `crates/btctax-tui-edit/src/edit/form.rs:221-236` | `TaxInputsFormState.broker_regime: Option<InformationReturnRegime>`, cached at open. |
| `crates/btctax-tui-edit/src/main.rs` (all four open arms) + `edit/form.rs::fresh` | set from `year_readiness::regime_for(year)` / `None`. |
| `crates/btctax-tui-edit/src/edit/tax_inputs.rs:545-575` | `apply_edit` captures `was_unmaterialized = form.working.is_none()` before the apply and re-seeds on the `false→true` transition, before `clamp_focus`. |

**Gated on the TRANSITION, not on `working.is_some()`** — seeding on every successful apply would
resurrect a provider row the filer removed with `[d]`, and removal is the block's only mutation
besides answering. That trade-off is itself tested.

**Kills** (`edit::tax_inputs::tests`):

- `the_broker_block_appears_after_materializing_on_a_first_session` — `fresh(2026)`, non-empty
  census, `PROCEEDS_AND_BASIS`, `tax_inputs_apply_edit(&mut form, "Single")`, then `live_sections`
  contains `BrokerReporting` and rows 0/1 are the ledger's providers in order.
- `the_broker_block_is_not_seeded_on_a_year_that_does_not_ask` — `None` / `NONE` / `PROCEEDS_ONLY`.
- `a_removed_broker_row_is_not_resurrected_by_the_next_edit`.

**Seen red, both directions:** planting `if false && was_unmaterialized` red the first and third
tests; planting `if true || was_unmaterialized` red the third with
`left: Some("coinbase") right: Some("gemini")`. Both reverted.

---

## M-1 — the live-year [I5] advisory named only G/H/J/K

`crates/btctax-cli/src/cmd/admin.rs:500-509` — the parenthetical now reads *"(I/L where you answered
that nothing was reported, H/K where only proceeds were, G/J where basis was)"*, and the doc comment
records **why**: the line reads only the row COUNT and never the answers, so a filer who answered
`not_reported` everywhere has every row on I/L and no 1099-DA lists any of them.

Test `broker_advisory_on_a_live_year_names_box_1g_and_box_1f` (`admin.rs:1404-1420`): the
`!msg.contains("Box I/L")` needle is replaced by (a) a positive loop requiring all three pairs
`I/L`, `H/K`, `G/J`, and (b) `!msg.contains("This export files EVERY")` — the not-live blanket-filing
sentence, which is the guarantee the old needle actually held. The not-live loop below now also
asserts that sentence **is** present, so the new needle cannot go stale silently.

**Seen red:** restored the old wording → FAIL on `the live advisory must name the box pair "I/L"`.
Reverted.

---

## M-2 — `report` was silent on the unread-answer state its own doc comment names

`crates/btctax-cli/src/render.rs:1896-1905` — the review's one-liner, with the reason inline:

```rust
let stored_unread = keys.iter().any(|k| !census.contains_key(k))
    || (!live && answers.is_some_and(|a| !a.0.is_empty()));
```

Plus a new match arm at `render.rs:1948-1955`: on a **not-live** year a key WITH rows and an answer
now prints *"— TY does not ask the 1099-DA question: REFUSES as unread (delete the answer)"* rather
than `G / J`, which would name a box the return will not carry.

The pinned `assert!(render_broker_answers(2025, PROCEEDS_ONLY, &c, Some(&a)).is_none())` is gone;
`a_stored_answer_on_a_not_live_year_with_rows_is_shown_as_unread` asserts the block IS shown, for
TY2025 (`PROCEEDS_ONLY`) and TY2024 (`NONE`), and that a not-live year with **nothing** stored is
still silent (the block is not a nag).

**Seen red:** reverted the second clause → FAIL on the `expect`. Reverted back.

---

## M-3 — no read-back kill on the four Schedule D per-box map rows

Three tests in `crates/btctax-forms/tests/full_return_forms.rs:3856-4123`, beside the §G-31 1a/8a
KATs they are modelled on:

1. **`schedule_d_per_box_rows_write_the_right_figure_to_the_right_cell`** — `ScheduleDLines` with
   four DISTINCT non-zero (d,e,h) triples on 1b/2/8b/9, filled through `fill_schedule_d_full(…,
   2024)`, all 12 cells read back by FQN, plus 4 assertions that column (g) stays absent.
   ★ The FQNs are **typed out, not read from the map** — exactly as the 1a/8a KAT does. A test that
   read the map back would be satisfied by any map, including a swapped one.
2. **`schedule_d_per_box_row_bindings_are_geometrically_distinct_on_both_revisions`** — for TY2024
   **and TY2025**, resolves each per-box cell's widget rectangle off the bundled PDF and asserts
   (a) each row's four cells sit at the same four x positions as the known-good line 3 / line 10
   rows and run (d) < (e) < (g) < (h), and (b) the rows descend the page in printed order (1b above
   2 above 3; 8b above 9 above 10).
3. **`schedule_d_refuses_a_non_zero_per_box_total_on_an_unbound_row`** — `ScheduleDMap::ty2024()`
   with `line1b = None` and a non-zero 1b total must refuse naming `line1b`; and with the 1b total
   zeroed the *same* unbound map fills cleanly (a pure-crypto year must still be able to file).

**Deviation, with the measurement.** The brief said "fill through the 2024 **and the 2025** fillers".
**`fill_schedule_d_full` cannot run on TY2025** — measured, not assumed:

```
year 2024: Ok(159735)
year 2025: Err("geometric read-back FAILED (mis-mapped cell): the TY2025 Schedule D map has no
                `line6` — the full-return fill needs it. Full-return v1 is TY2024-only.")
```

The 2025 map is the crypto-slice map (no `line1a`/`line8a`/`line6`/`line13`/`line14`), and
`need(&map.line6, …)` is unconditional, so no per-box cell is ever written; the crypto-slice filler
`fill_schedule_d_totals` writes only 3/7/10/15/16 and never touches these rows. Filling 2025 would
have required either extending the 2025 map (a T4 behaviour change outside this fold) or borrowing
2024's FQNs against the 2025 PDF (fabricated geometry). So the 2025 revision gets the same
**discrimination** geometrically — the exact fact the reviewer established by hand with `xtask
dump-fields`, made permanent — and TY2024 gets it twice, once by value and once by geometry.

**Seen red, three plants:**

- 2024 map `line1b` `proceeds_d`↔`cost_e` swapped → the value KAT failed at the filler's own column
  verifier (`x-center 395.6 not in column 0 cluster`) and the geometric test failed too.
- 2024 map `Row1b`↔`Row2` swapped → both failed (`ordinal-y descent broken`). ★ Note the shipped
  verifier catches this *only because a value is now written*: before these tests, `box_row`
  returned `None` on every fixture and no placement was produced at all.
- 2025 map `Row1b`↔`Row2` swapped → the geometric test failed (the 2024 value KAT is blind to the
  2025 map, which is why test 2 exists).
- `box_row`'s `need(...)` replaced with a silent `Ok(cells.clone())` → the refusal test failed on
  its `expect_err`.

All four plants reverted; `git status` shows both map TOMLs unmodified.

---

## M-4 — the broker refusal anchors: forward-looking, and landing on row 0

**(1) Recorded.** `crates/btctax-input-form/src/attribute.rs:64-75` — a comment above the four
broker arms stating that they are forward-looking: the TUI commit gate runs `screen_inputs`, and
`screen_broker_reporting` is reached only from `screen_absolute`, which runs only on a year that
computes — so no `CommitOutcome::Refused` can carry a broker `RefuseReason` today. It names the
commit gate as the thing to re-check when a live full-return year lands, so the next reader does not
re-derive it.

**(2) Done — it was small.** `crates/btctax-tui-edit/src/edit/tax_inputs.rs`:

- new `broker_refusal_row(ri, reason) -> Option<usize>` (`:778-798`), resolving the refusal's
  `provider` to a row index through the `broker_row_provider` seam (this module never names a
  `ReturnInputs` field);
- `focus_refusal` (`:837-847, 862-865`) computes it under the same immutable borrow as the anchor,
  **only when the jump lands on `SectionId::BrokerReporting`**, and sets `form.addr =
  RowAddr(vec![i])` instead of `RowAddr::default()`. No other section's behaviour changes.

`Anchor` was **not** widened — a cross-crate enum change was not needed, since `focus_refusal`
already receives the `RefuseReason` and the variants carry `provider`.

**Kill:** `a_broker_refusal_focuses_the_named_providers_row` — two providers, refusal about `gemini`
(row 1); asserts `form.addr == RowAddr(vec![1])`, `refused_section == BrokerReporting`, and the
focused field is `BrokerCovered`; repeats for all four broker variants; and asserts an **unknown**
provider falls back to `RowAddr::default()` rather than a wrong row.
**Seen red:** reverted to `form.addr = RowAddr::default()` → FAIL, `left: RowAddr([]) right:
RowAddr([1])`. Reverted.

---

## N-1 — a `Debug` render of `BrokerRouteError` reached a user-facing message

`crates/btctax-core/src/forms.rs:77-104` — `impl Display for BrokerRouteError`, naming the
`(provider, cohort)` key the way the filer types it back into `[broker_reporting.<provider>]` and
saying what is wrong in prose. `crates/btctax-cli/src/render.rs:1223-1228` now formats `{e}`.

**Kill:** `kat_broker_reporting::a_route_error_renders_as_prose_naming_the_key` — every variant must
name its provider and slot, and must NOT contain `Unanswered` / `Mixed {` / `BasisDiffers` /
`cohort:` / `provider:` (the `Debug` shape that was leaking).
**Seen red:** replaced the impl with `write!(f, "{self:?}")` → FAIL, message
`Unanswered { provider: "coinbase", cohort: Covered }`. Reverted.

---

## N-2 — `line_coverage` quoted the 2024 Schedule D text for lines the code fills per 2025

`crates/btctax-core/src/tax/line_coverage.rs:2105-2119, 2226-2228` — `Coverage::quoting_year` does
support a per-row year, so the twelve rows for lines 1b/2/8b/9 are now quoted from
`f1040sd--2025.txt` ("with Box A **or Box G** checked", etc.), with `c.quoting_year("2025")` before
them and `c.quoting_year("2024")` restored after. A block comment states that the routing the code
implements is the 2025 revision's pairing, and that lines 3 and 10 stay on 2024 by the pre-existing
convention (out of scope, unchanged).

**Kill:** the checker itself, now genuinely pointed at the 2025 booklet.

```
$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 341 money lines across 17 form(s) […f1040sd:31…], 24 exception(s) (ratchet 24),
0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)
```

**Seen red:** restored the 2024 sentence on the now-2025 rows →

```
line-coverage FAILED (3 problem(s)):
  - f1040sd:1b(d) (line1b_d) quotes text NOT FOUND in f1040sd--2025.txt: …
```

Reverted. **No ratchet moved**: exceptions 24/24, unverifiable 0/0, not-line-bound 12/12, and no
census register or `max_unwitnessed` was touched anywhere in this fold.

---

## Validation

Commands run, with their summary lines.

| command | summary |
|---|---|
| `cargo nextest run --locked -p btctax-tui` | `159 tests run: 159 passed, 2 skipped` |
| `cargo nextest run --locked -p btctax-tui-edit` | `381 tests run: 381 passed, 2 skipped` |
| `cargo nextest run --locked -p btctax-cli --lib` | `213 tests run: 213 passed, 0 skipped` |
| `cargo nextest run --locked -p btctax-cli` | `662 tests run: 662 passed, 1 skipped` |
| `cargo nextest run --locked -p btctax-forms` | `341 tests run: 341 passed, 4 skipped` |
| `cargo nextest run --locked -p btctax-core` | `1211 tests run: 1211 passed, 0 skipped` |
| `cargo nextest run --locked -p btctax-core -E 'test(line_coverage) or test(broker) or test(routing_table) or test(cohort)'` | `27 tests run: 27 passed, 1184 skipped` |
| `cargo nextest run --locked -p btctax-input-form` | `65 tests run: 65 passed, 0 skipped` |
| `cargo nextest run --workspace --no-fail-fast` | `3105 tests run: 3105 passed, 12 skipped` |
| `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo run -q -p xtask -- line-coverage` | `OK: 341 money lines … 24 exception(s) (ratchet 24)` |
| `cargo fmt --all` | applied |

**Byte-identical, as required, and machine-confirmed rather than asserted:**
`git diff -- docs/` is empty. `docs/examples-tui-walkthrough/j2/03-forms.txt` (TY2025) and
`j6/04-forms.txt` (TY2024) are unchanged and their goldens pass
(`btctax_tui_walkthrough_goldens_match_committed`,
`btctax_tui_edit_walkthrough_goldens_match_committed`); `docs/examples/examples.md` is unchanged and
`examples_golden_matches_committed` passes. Both are structural, not luck: those years are not live,
and `a_not_live_year_prints_the_rows_as_built` is the test that keeps them that way.

## Files touched (22)

`btctax-core`: `src/forms.rs`, `src/tax/line_coverage.rs`, `tests/kat_broker_reporting.rs`.
`btctax-cli`: `src/cmd/admin.rs`, `src/cmd/tax.rs`, `src/render.rs`, `src/session.rs`.
`btctax-forms`: `tests/full_return_forms.rs`.
`btctax-input-form`: `src/attribute.rs`.
`btctax-tui`: `src/app.rs`, `src/unlock.rs`, `src/tabs/forms.rs`, `src/tabs/tests.rs`, `src/lib.rs`,
`src/export.rs`, `src/whatif_panel.rs`.
`btctax-tui-edit`: `src/main.rs`, `src/draw_edit.rs`, `src/edit/form.rs`, `src/edit/tax_inputs.rs`.

Neither the spec nor the review report was touched.

**Not mine, present in the working tree before this fold:** `M .gitignore` (adds `target-review/`),
`M CONTINUITY.md`, `?? target-review/`, `?? design/agent-reports/BRIEF-build-4868-T1-T5.md`.

## What I could not do

- **The TY2025 Schedule D per-box fill-and-read-back** (M-3). `fill_schedule_d_full` refuses on
  TY2025 for want of `line6`, unrelated to the 1099-DA rows; measured and quoted above. Substituted
  a geometric binding test covering both revisions, which discriminates both confusions M-3 names.
  Whoever extends the TY2025 map to a full return should add the value read-back then; it is three
  lines of fixture on the existing test.
- **No live path exercises the broker refusal anchors** (M-4 (1)). Recorded in code rather than
  fixed: making one would mean giving TY2026 a `FullReturnParams`, which is a different project.
