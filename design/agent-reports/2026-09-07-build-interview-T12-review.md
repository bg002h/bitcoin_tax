# Seam review — interview build T12 (the render pass, R12)

Independent adversarial build review, own git worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-ad8661c68cefc15d3` at `833ce3f1`, branch `main`.
Read-only for the record: every plant was made here, reverted from a `cp` backup (or `rm` for a
probe file), with `find crates -name '*.rs' -exec touch {} +` after **both** the plant and the
restore (FR-90). Nothing committed, nothing pushed, no subagents. `git status --short` is empty at
the time of writing.

---

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

cargo nextest run --locked -p btctax-cli      -E 'test(the_report_block_prints_the_census_and_masks_every_payer_tin)'
cargo nextest run --locked -p btctax-tui-edit -E 'test(zz_probe_panel_counts)'            --no-capture   # planted probe
cargo nextest run --locked -p btctax-tui-edit -E 'test(zz_probe_handler_clamp)'           --no-capture   # planted probe
cargo nextest run --locked -p btctax-tui-edit -E 'test(zz_probe_tail_unreachable)'        --no-capture   # planted probe
cargo nextest run --locked -p btctax-tui-edit -E 'test(zz_probe_modal_sizes)'             --no-capture   # planted probe
cargo nextest run --locked -p btctax-tui-edit -E 'test(zz_probe_panel_sizes)'             --no-capture   # planted probe
cargo nextest run --locked -p btctax-core     -E 'test(zz_probe_undated)'                 --no-capture   # planted probe
cargo nextest run --locked -p btctax-cli      -E 'test(zz_probe_panel_vs_screen)'         --no-capture   # planted probe
cargo nextest run --locked -p btctax-tui-edit --no-fail-fast                                             # plants A / B / C
cargo nextest run --locked -p btctax-core     -E 'test(the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523)' --no-capture
cargo nextest run --locked -p btctax-core     -E 'test(only_the_two_credit_arms_check_a_box)'            --no-capture
cargo nextest run --locked -p xtask           -E 'test(every_slot_caption_is_the_forms_own_words)'       --no-capture
cargo nextest run --locked -p btctax-cli -p btctax-tui-edit --no-fail-fast                               # the `(declined)` plant
cargo run   -q -p xtask -- stop-list                                                                     # baseline + a planted `Gauge`
cargo nextest run --locked -p btctax-core -p btctax-tui-edit -p xtask --no-fail-fast                     # restored-tree confirmation
```

Restored-tree confirmation: **1941 tests run: 1934 passed, 7 failed, 3 skipped** — the seven failures
are exactly the environment set the brief names (six `form_delta::*` and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`). Nothing else red.

---

## Summary

T12's central claim holds: the pane, the commit modal, `report`'s §4.4 block and the manifest all
render through `btctax_cli::panel_lines` / `forgoing_lines` / `refusing_lines`, and three separate
divergence plants (a locally-filtered state, a modal that drops the refusing list, a pane that
ignores the year's package) each red a named kill. The four follow-ups are genuinely closed — I
reproduced FR-87's kill and two of FR-86's six verbatim, and FR-83's derived kill is real. The
commit-modal layout fix is robust: it keeps `[Enter] commit / [Esc] cancel` and reaches its last
line at all seven terminal sizes I measured, down to 40×10.

Three defects were found, all measured with plants:

1. **Critical** — the answer-panel pane's scroll is clamped a *second* time in the key handler, by
   the count of **logical** lines. On the fixture T12's own kill uses, that caps the cursor at 38
   while the renderer needs 104: **66 of 141 rows are unreachable by any keystroke**, on a pane
   whose doc comment guarantees the opposite and whose footer invites the filer to scroll. Kill #2
   cannot see it because it writes `panel_scroll` directly. The builder pasted the stuck footer —
   `showing 39–75 of 141 lines` — into §1 of the report as the pane's normal output.
2. **Important** — the panel prints `nothing is open: … and no answer refuses.` on a return
   `screen_inputs` refuses with `HomeSaleNotComputed`. `interview_state.refusing` can produce 5 of
   the 126 `RefuseReason` variants; the LIMITATIONS.md section T12 ships promises the filer will
   meet a refusal "while you are still authoring rather than at commit."
3. **Important** — `undated_document_rows` walks 5 of the 8 row structs that carry
   `transcribed_on`. An undated **Form 1098**, **Form 1099-SA** or **Form 5498-SA** row is silently
   absent from both the §4.4 block and the packet manifest. Both tests that cover it pick
   `int_1099` by hand.

---

## Findings

### C-1 (Critical) — the answer-panel pane cannot be scrolled to its end; the key handler re-clamps by LOGICAL lines

**Where.** `crates/btctax-tui-edit/src/main.rs:1313-1314` (inside the `form.panel_open` block of
`handle_tax_inputs_key`), against `crates/btctax-tui-edit/src/draw_edit.rs:2182`
(`draw_tax_inputs_panel`'s clamp).

**What is wrong.** `draw_tax_inputs_panel` correctly clamps against **wrapped rows**
(`form.panel_scroll = form.panel_scroll.min(total.saturating_sub(view_h))`) — that is deviation 1's
whole justification, and its comment says so:

> *"Clamping there would bound the cursor by the number of LOGICAL lines, and a panel whose prompts
> each wrap to twenty rows would then stop scrolling a fifth of the way down with the rest
> unreachable: the FR-63 defect, one surface on."*

The key handler nonetheless still does exactly that, on every keypress:

```rust
// Clamp against the CURRENT panel so a scroll can never run past the last line.
let last = form.panel_lines().len().saturating_sub(1);
form.panel_scroll = form.panel_scroll.min(last);
```

`panel_lines()` returns **logical** lines. The renderer's write-back is therefore overwritten by the
tighter bound before the next frame, and the pane stops dead partway down. This is the defect kill
#2 was written for, surviving one layer up, because the kill sets `form.panel_scroll` directly
(`draw_edit.rs:6867`, `form.panel_scroll = usize::MAX / 2;`) and never presses a key. The
`p`-toggle test (`main.rs:10353`) presses keys but never scrolls.

The pane's own doc comment states the guarantee that is broken:
*"★★ **A line that does not fit is REACHABLE, never clipped** (the FR-63 rule)."*

**Evidence.**

*Plant 1 — measure the two denominators* (temporary `#[test]` appended to `draw_edit.rs`'s test
module, fresh TY2024 Single, the fixture kill #1/#2 use, inner width 112 = the 120×40 pane):

```
PROBE logical=39 wrapped=141 view_h=37 renderer_max=104 handler_max=38
```

*Plant 2 — drive the real key handler* (temporary `#[test]` in `main.rs`'s test module; `p`, then
500 × `PageDown` through `handle_key`):

```
PROBE after 500 PageDown: panel_scroll=38 logical_lines=39
```

*Plant 3 — render at the ceiling the handler enforces* (temporary `#[test]` in `draw_edit.rs`,
`panel_scroll = panel_lines().len() - 1`, `TestBackend::new(120, 40)`):

```
PROBE footer at handler ceiling: │ │  showing 39–75 of 141 lines · [↑/↓ · PgUp/PgDn] scroll · [p/Esc] close   │ │
PROBE last line reachable? false
PROBE wanted: (0 answered, 51 not applicable to this return)
```

Rows 76–141 — 47% of the panel, and the tail is where `FORGOING`, `NOT COMPUTED` and the
`(n answered, m not applicable)` summary live — are unreachable, while the footer keeps telling the
filer to press ↑/↓/PgUp/PgDn.

★ This exact footer string is in the T12 build report, §1, as the pane's ordinary output:
`showing 39–75 of 141 lines · [↑/↓ · PgUp/PgDn] scroll · [p/Esc] close`. It is the stuck ceiling.

**Minimal change.** Delete the two-line clamp in the handler (`main.rs:1313-1314`) and let the
renderer's write-back own it — which is precisely the pattern the commit modal already uses
(`main.rs:1259-1268` only *moves* `m.scroll`; `draw_tax_inputs_modal` clamps and writes back). I
applied that deletion in the worktree and re-ran the crate: **400 passed, 2 skipped**, no
regression (the only failure was my own deliberately-panicking probe). Then add the missing half to
kill #2 — reach the tail *through `handle_key`*, not by assigning `panel_scroll`, so the guarantee
is held where a filer actually exercises it.

---

### I-1 (Important) — the panel asserts "no answer refuses" on a return the commit gate refuses

**Where.** `crates/btctax-cli/src/cmd/answer.rs:323-330` (`panel_lines`' zero-open sentence), the
`refusing` set built in `crates/btctax-core/src/tax/interview_state.rs:284 / 459 / 476`, and the
promise T12 ships in `crates/btctax-cli/LIMITATIONS.md`:

> **REFUSING** — an answer you have *already given* that stops the return, each with its exit, so
> you meet it **while you are still authoring rather than at commit**;

**What is wrong.** `InterviewState::refusing` is assembled from registry-question refusals,
dependent-row refusals and the document-census row invariant only. It can produce **5** of the
**126** `RefuseReason` variants (`DependentGateRefused`, `DependentRefusedByQuestion`,
`DocumentCensusContradicted`, `DocumentDeclaredNotTranscribed`, `DocumentTypeUnsupported`). The
commit gate is `screen_inputs`, which raises all of them — including `HomeSaleNotComputed`, the
refusal T9 added and §4.4 asks `report` to print.

So a filer whose *answers* stop the return sees no `REFUSING` section, and if nothing else is open
the panel prints an affirmatively false sentence. The commit modal is the surface this matters most
on: it shows forgoing + refusing only, so on this return it shows **nothing at all**, and the write
is refused after Enter — the opposite of J-32's *"meet it before the write"*.

`report`'s §4.4 block partially self-corrects (it prints the home-sale decision three lines lower);
the TUI pane, the commit modal and `income answer` do not.

**Evidence.** Temporary integration test `crates/btctax-cli/tests/zz_probe_panel_vs_screen.rs` —
TY2024 Single, `answer_all_live_declarations`, every live skippable answered, then the home sale:
`sold_main_home = Some(true)`, both tests `Some(true)`, `can_exclude_all_gain = Some(false)`,
`S1099 = Some(false)`:

```
PROBE blocking=0 refusing=0 forgoing=0 waiting=0 not_computed=0 open=0
PROBE screen_inputs = Some(HomeSaleNotComputed("you said you cannot exclude all of your gain"))
PANEL| ── The answer panel (tax year 2024) ──
PANEL|   nothing is open: every live question is answered, nothing is forgone, and no answer refuses.
PANEL|   (39 answered, 47 not applicable to this return)
```

The TUI commit gate is the same call: `edit/persist.rs::form_commit` →
`btctax_cli::input_form_store::commit`, which at `input_form_store.rs:666` runs
`screen_inputs(ri, table, params)` and returns `CommitOutcome::Refused`.

**Minimal change.** Either (a) widen the panel — run `screen_inputs_tiered` beside the registry walk
and push any refusal it raises that the registry did not already model into `st.refusing`, which is
what makes the LIMITATIONS.md sentence true and is one call; or (b) if that is out of a render
pass's scope, narrow both claims in the same commit: the zero-open sentence becomes *"no answer in
the interview refuses; the commit screen runs further checks"*, and LIMITATIONS.md's REFUSING bullet
says which refusals it covers. What must not stand is the unqualified sentence beside a gate that
disagrees with it. Whichever is chosen, the kill is the fixture above: a return with
`open_items() == 0` and `screen_inputs(..) == Some(_)` must not print *"no answer refuses"*.

---

### I-2 (Important) — three of the eight document families that carry `transcribed_on` are missing from the undated-rows list

**Where.** `crates/btctax-core/src/tax/provenance.rs:162-197` (`undated_document_rows`), consumed by
`crates/btctax-cli/src/render.rs:3245-3255` (the §4.4 block T12 added) and
`crates/btctax-cli/src/cmd/admin.rs:2044-2046` (`undated_rows_block` in the packet manifest).

**What is wrong.** Eight row structs carry `transcribed_on`
(`return_inputs.rs:120 / 165 / 212 / 282 / 314 / 502 / 555 / 664` → `Form1099Int`, `Form1099Div`,
`Form1099G`, `Form1098E`, **`Form1098`**, **`Form1099Sa`**, **`Form5498Sa`**, `Form1099B`). The
walk covers five: `int_1099`, `div_1099`, `g_1099`, `b_1099`, `form_1098e`. **Form 1098** (T9 added
it as a document family and gave it a `transcribed_on`), **Form 1099-SA** and **Form 5498-SA** are
never visited.

The function is hand-enumerated and carries a prose exception for the W-2 ("the W-2 row carries no
`transcribed_on` of its own") — an excuse list rather than a mechanism, and it has gone stale
exactly the way `CLAUDE.md` predicts. T12's report claims the block "nam[es] any document row that
carries no `transcribed_on`", and the manifest's block says *"These document rows carry no
transcription date"* — both present themselves as closed lists and are not.

**Evidence.** Temporary integration test `crates/btctax-core/tests/zz_probe_undated.rs` — one
undated row of each of `int_1099`, `form_1098`, `sa_1099`, `sa_5498`:

```
PROBE undated rows = [
    "Form 1099-INT #1 (Bank A) — transcribed without a date",
]
```

Three undated rows are invisible on both surfaces. Neither test can see it: the T12 kill
(`export_irs_pdf.rs:2983`, line 3011) and the pre-existing T5 kill (line 3113) both populate
`int_1099` and only `int_1099` — the FR-88 shape, a fixture deciding what the guard can see.

**Minimal change.** Add the three missing loops (`form_1098` → `lender`, `sa_1099` → `payer`,
`sa_5498` → `trustee`), and make the coverage structural rather than remembered: derive the
expectation in the kill from the families that have a `transcribed_on` — the same move
`limitations_names_every_excluded_document_family_the_census_refuses` makes off `DocumentRow::ALL`
— so the next family added reds on the day. A cheap version: a KAT that populates one undated row
of every `DocumentKind` arm of `document_identity` (which is already exhaustive) and asserts each
one is named.

---

### M-1 (Minor) — the manifest's FORGONE block drops the NOT COMPUTED instruction

**Where.** `crates/btctax-cli/src/cmd/admin.rs:625-627`.

**What is wrong.** `forgoing_block` appends `st.not_computed` items under the heading
*"FORGONE — benefits you are lawfully entitled to skip, and did"*, using `n.line()` alone. The
panel keeps them under their own heading, which carries the actionable half:
*"the credit boxes on your return are printed, the amount is yours to enter."* On paper — the
artifact the filer follows while assembling the envelope — line 19 is presented as something they
chose to skip, and the sentence telling them to enter an amount by hand is gone. The `COMPLETE BY
HAND` block immediately above is where that instruction belongs.

**Evidence.** Read directly: `forgoing_block` writes the `FORGONE` header, then
`for line in crate::forgoing_lines(&st)`, then `for n in &st.not_computed { push(&mut s, &n.line()); }`
— no `NOT COMPUTED` heading is emitted, and `panel_lines` (`answer.rs:355-364`) is the only place
that sentence exists.

**Minimal change.** Emit the `NOT COMPUTED (n) — …` heading line before the loop, the same way the
`FORGOING (n)` heading rides along from `forgoing_lines`.

---

### M-2 (Minor) — below 80 columns the pane's own `[p/Esc] close` legend is truncated; the modal wraps its legend and the pane does not

**Where.** `crates/btctax-tui-edit/src/draw_edit.rs:2196-2210` (the pane's footer is a single
unwrapped `Line`; the `Paragraph` has no `Wrap`, so it is cut at the box edge).

**Evidence.** Temporary `#[test]` rendering the pane at eight sizes with `panel_scroll` past the
end:

```
PROBE panel 120x40: close_legend=true  tail_reachable=true  clamped_scroll=104
PROBE panel 100x30: close_legend=true  tail_reachable=true  clamped_scroll=140
PROBE panel  80x24: close_legend=true  tail_reachable=true  clamped_scroll=196
PROBE panel  80x20: close_legend=true  tail_reachable=true  clamped_scroll=200
PROBE panel  72x16: close_legend=false tail_reachable=true  clamped_scroll=228
PROBE panel  60x12: close_legend=false tail_reachable=true  clamped_scroll=289
PROBE panel  40x10: close_legend=false tail_reachable=false clamped_scroll=491
PROBE panel  40x6:  close_legend=false tail_reachable=false clamped_scroll=495
```

Minor because 80×24 is this repo's documented floor (`draw_edit.rs:628`, `:1986`) and it holds
there. It is recorded because the commit modal solved the identical problem the right way in the
same commit — `legend_of` runs its text through `wrap_to` — and the pane did not.

**Minimal change.** Run the pane footer through `wrap_to(&more, inner_w)` and size `view_h` against
its row count, exactly as `draw_tax_inputs_modal` does.

---

### N-1 (Nit) — a duplicated comment block in `draw_tax_inputs_modal`

`crates/btctax-tui-edit/src/draw_edit.rs:2268-2278` and `:2279-2295` are two near-identical copies
of the *"T12 — A PAYLOAD TALLER THAN THE TERMINAL SCROLLS"* block; the first is an earlier draft
(it says the header and legend "are now drawn from OUTSIDE the window" and omits the pre-wrap
paragraph). Delete the first.

### N-2 (Nit) — `r15_stop_list.rs` still says "three" in two doc comments

`crates/xtask/src/r15_stop_list.rs:459` (*"The three checks"*) and `:468` (*"each of the three
greps"*) — there are four since T12; `:370` was updated and these were not.

### N-3 (Nit) — `progress_widgets` scans one file, and the CLI panel renderer is outside its field of view

`renderer_sources()` (`r15_stop_list.rs`) lists only
`crates/btctax-tui-edit/src/draw_edit.rs`. The same panel is rendered by
`crates/btctax-cli/src/cmd/answer.rs` and printed by `render.rs` / `admin.rs`; a hand-rolled
`format!("{pct}% done")` there is caught by nothing. `panel_lines`' own kill asserts no `%` in the
rendered lines, so the exposure is small — but the check's mechanism (a formatted percentage)
applies verbatim to those files and adding them is a one-line change to the source list. FR-98
already owns the *field-name* half of this residue; this is the renderer half.

---

## Verdict on the five areas

**1. Is anything recomputed rather than rendered?** — **Clean.** The pane
(`edit/form.rs:393`), the commit modal (`edit/form.rs:414`), `report`
(`render.rs:3143-3148`) and the manifest (`admin.rs:598-602`) each build an `InterviewState` from
`interview_state{,_with_params}` and hand it to `btctax_cli::panel_lines` / `forgoing_lines` /
`refusing_lines`. There is no second renderer, no liveness predicate and no registry walk in any
render path — I grepped the whole `+` side of the diff for `save_draft` / `return_inputs::set` /
`record_answer` / `.set(` / `fs::write` and every hit is inside a `#[cfg(test)]` fixture. Three
divergence plants each red a named kill:

| plant | red |
|---|---|
| A — `TaxInputsFormState::panel_lines` does `st.forgoing.clear()` (a local filter) | `the_pane_marks_a_declined_benefit_declined_and_never_lists_it_as_blocking` |
| B — the pane ignores the year's package (`interview_state`, never `_with_params`) | `the_pane_marks_a_declined_benefit_declined_and_never_lists_it_as_blocking` |
| C — `commit_panel_lines` drops the refusing half | `the_commit_modal_prints_the_forgoing_and_refusing_lists_with_the_exit` |

`write_panel`'s refactor is byte-identical (the diff moves each `writeln!` to an `out.push` of the
same format string; the leading blank line stayed in the CLI). The `(declined)` mark is held on two
surfaces at once — planting `let mark = "";` reds
`the_manifest_lists_the_forgone_benefits_marking_the_declined_ones_and_names_the_undated_rows`
**and** `the_pane_marks_a_declined_benefit_declined_and_never_lists_it_as_blocking`. And
`collected_figures` cannot silently skip a money leaf: its set is `leaf_walk::money_leaves` joined
to `LEAF_SOURCE`, and `every_money_leaf_has_exactly_one_source_and_every_source_prefix_is_live`
asserts `audit(LEAF_SOURCE, &money) == Audit::default()` in both directions. All nine
`Source::Document` prefixes are `Vec` fields, so `row_index_of` always resolves and no document
figure can be mis-attributed to "your own records".

**2. Do the lists that present themselves as complete actually complete?** — **No, on two of
three.** The forgoing list (with `(declined)`) is complete and held. The undated-document-row list
is **not** (I-2). The refusing list is **not**, and the panel makes an affirmative claim about it
(I-1).

**3. The four follow-ups.** — **FR-73 CLOSED, verified**: both kills are genuinely derived —
`limitations_names_every_excluded_document_family_the_census_refuses` enumerates from
`DocumentRow::ALL` filtered by `exit_sentence().is_some()` with a `checked >= 8` vacuity guard, and
`limitations_carries_the_venue_account_granularity_note` asserts the token is in
`step0::VENUE_GRANULARITY_NOTE` *before* asserting it is in the doc. **FR-83 CLOSED, verified**:
the fallback is the waiting wording, and the kill loops over every `needs_params()` gate asserting
both halves. **FR-86 CLOSED** — I re-planted 2 of the 6 and both reproduce the report's quoted red
verbatim:

```
# credit_column: NoCreditBox -> CreditForOtherDependents
assertion `left == right` failed: NoCreditBox
  left: CreditForOtherDependents
 right: Neither
# SLOT_CAPTIONS: "And in the U.S." -> "And in the United States"
LivedWithYouInUs's caption is not in design/forms/extract/f1040--2025.txt: "And in the United States"
```

No checker was changed to make them red. **FR-87 CLOSED, verified**: `home_sale_decision` is one
decider with two readers (`return_refuse.rs:3517` in `screen_inputs_tiered`, `render.rs:3192` in the
report block) — confirmed by reading both call sites, not by the report. Planting the census's §2.2
rule away (`if let Some(exit) = row.exit_sentence()` → `if let Some(exit) = None::<&str>`) reds the
cross table with the report's exact line:

```
(true,true,true,s_1099=Some(true)) refused unexpectedly: None
```

**4. The layout defect T12 introduced and fixed.** — **Holds, at every size I tried.** The modal
keeps its legend at the top *and* at maximum scroll, and reaches its last payload line, at
120×40, 100×30, 80×24, 80×20, 72×16, 60×12 and 40×10:

```
PROBE 120x40: legend_top=true legend_end=true tail_reachable=true
PROBE 100x30: legend_top=true legend_end=true tail_reachable=true
PROBE  80x24: legend_top=true legend_end=true tail_reachable=true
PROBE  80x20: legend_top=true legend_end=true tail_reachable=true
PROBE  72x16: legend_top=true legend_end=true tail_reachable=true
PROBE  60x12: legend_top=true legend_end=true tail_reachable=true
PROBE  40x10: legend_top=true legend_end=true tail_reachable=true
```

The `debug_assert!(lines.len() + 2 <= area.height.max(10))` is live in the test profile and does not
fire. The same class **does** appear in the new pane, twice: C-1 (vertical, via the handler) and
M-2 (horizontal, below 80 columns).

**5. The deviations.** — Five of six are sound; one is the source of C-1.

- **(1) `&mut TaxInputsFormState` in the three draw fns.** Correct and necessary: the wrapped-row
  count is a fact about the width. But it makes the renderer the *sole* owner of the clamp, and the
  handler's leftover clamp (C-1) is what that deviation was supposed to remove. Accept the
  deviation; delete the leftover.
- **(2) The crate-root re-export.** Reasonable. All three are pure `&InterviewState -> Vec<String>`
  and the alternative — a second renderer in the TUI — is what R12 forbids. It does mean KAT-G1's
  grep can no longer see the dependency; nothing to change today, but the gate now has a channel a
  future non-pure function could travel.
- **(3) `home_sale_decision` extracted.** Correct, and the best decision in the task: it is what
  makes §4.4's home-sale line a *render*. Verified above; the 24-combination cross test is intact
  and reds on the census plant.
- **(4) The modal scrolls, `Wrap` removed.** Correct and verified at seven sizes.
- **(5) A fourth R15 check instead of a wider field walk.** Correct — refusing the
  `remaining_sat` excuse list is the rule, not a shortcut, and FR-98 records the residue with its
  mechanism. Verified red: planting `let _plant = ratatui::widgets::Gauge::default();` into
  `draw_tax_inputs_panel` gives
  `xtask stop-list: R15: a PROGRESS WIDGET is drawn … crates/btctax-tui-edit/src/draw_edit.rs:2166: Gauge`,
  and `main.rs` exits 1 on findings. See N-3 for the one file it does not reach.
- **(6) `leaf_walk`'s doc sentence amended.** Correct — the sentence was false the moment
  `collected_figures` shipped, and the measured cost is recorded.

Drift: the brief's own line numbers were stale, as warned. I re-measured every anchor I used; the
build report's drift table matched what I found (`admin.rs` `COMPLETE BY HAND` at `:576`,
`crates/btctax-cli/LIMITATIONS.md` not `docs/LIMITATIONS.md`). Noted, not a finding.

---

## Seams checked clean

- **Derived, not recomputed** (seam 1) — one derivation, four surfaces; three divergence plants
  each red a named kill; no writes in any render path.
- **The seven states rendered** (seam 2) — blocking rows are per-item and derived from the walk;
  `Declined` renders *(declined)* with its size and never as blocking, and `Given` removes it;
  a params-less year sizes nothing and lists the gate as waiting; `waiting` names its package;
  `not_computed` has its own heading in the panel. (The `refusing` **set** is I-1; its *rendering*
  is faithful.)
- **The modal and the gate** (seam 3) — the modal prints forgoing and refusing from the same two
  functions; `a_return_the_panel_lists_as_refusing_is_refused_by_the_commit_screen_too` is a real
  assertion over the actual `screen_inputs` call and reds on the census plant; the modal's lists are
  year-derived, so it cannot mention a gate the year does not have.
- **`report` and the manifest** (seam 4) — the census prints every `DocumentRow::ALL` row with its
  declared state; the panel, the home-sale decision, row (7) per dependent and the `LEAF_SOURCE`
  provenance are all reads of the deciding function; `document_identity`'s match is exhaustive over
  `DocumentKind` and there is no branch returning an unmasked TIN (`mask_payer_tin("")` → `""`,
  `mask_payer_tin("12-3456789")` → `**-***6789`); the block rides the NOT-COMPUTABLE branch too.
  (The undated-rows list is I-2.)
- **Docs** (seam 5) — `LIMITATIONS.md`'s new section carries the five states in the filer's words,
  the *(declined)* rule, the eleven excluded families in the census's own designations, the
  home-sale branch, the 1099-DA keystroke, FR-73's granularity note, no self-custody import, and
  "no progress bar and no 'N of M'"; the walkthrough goldens moved only where a screen changed
  (j6/01 and j6/02: one glyph row and one style run each, the legend gaining `[p] answer panel`;
  j6/03: the taller commit modal, which still carries `[Enter] commit [Esc] cancel` and its range
  footer at row 37–38).
- **Nothing writes** (seam 6) — verified by grep over the diff's `+` side; every hit is a test
  fixture.

---

Counts: C=1 I=2 M=2 N=3
