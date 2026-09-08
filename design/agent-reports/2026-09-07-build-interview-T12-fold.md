# T12 fold — the seam review (1C / 2I / 2M / 3N) folded into `main`

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatched at
`477d995b`. No subagents. **Nothing committed, nothing pushed.** No `git checkout --` / `restore` /
`stash`: every plant was reverted from a `cp` backup, with
`find crates -name '*.rs' -exec touch {} +` after **both** the plant and the restore (FR-90).

---

## Closing gate

```
$ cargo fmt --all                                            # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
                                                             # clean
$ make check
     Summary [  20.945s] 3537 tests run: 3537 passed, 12 skipped
make check exit: 0
$ make docs
make docs exit: 0        # NO diff to any committed man page (`git status --short` shows no docs/ entry)
```

**3529 → 3537 passed (+8), 12 skipped unchanged.** Every added test is named below with the defect it
was watched red on. M-1's kill is an extension of an existing test rather than a ninth, which is why
the count is +8 and not +9.

### The five instruments — HEAD → now

| instrument | at `477d995b` | now | cause |
|---|---|---|---|
| `line-coverage` | 375 / 18 forms / 31 exceptions (ratchet 31) / 0 unverifiable / 17 not-line-bound | **identical** | the fold adds no form line |
| `census-join` | 274 across 13 maps | **identical** | no map cell |
| `stop-list` | 8 + 4 + **1** renderer source, 91 prompts | 8 + 4 + **6** renderer source(s), 91 prompts | **N-3** — `renderer_sources()` widened from `draw_edit.rs` to every file that renders the panel |
| `prompt-check` | 88 assertions | **identical** | no prompt gained a citation |
| `box-census` | 268 / 19 editions / 9 returns | **identical** | no box transcribed |

Verbatim:

```
line-coverage OK: 375 money lines across 18 form(s) [...], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0), 17 not line-bound (ratchet 17)
census join: 274 unmodeled entries across 13 maps, every one placed by a direction block asserted against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a sentence the form prints) and covered by an existing variant
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources, 6 renderer source(s) and 91 registry prompts scanned; no forbidden shape
xtask prompt-check: OK — 88 assertions, all verbatim
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one decided (268 entries)
```

`stop-list` is the only pinned number that moved, and N-3 is its cause.

---

## C-1 (Critical) — the panel pane can be scrolled to its end

**What changed.** The two-line clamp in `handle_tax_inputs_key`'s `form.panel_open` block is
**deleted**. The arms now only *move* the cursor; `draw_tax_inputs_panel` clamps against its own
wrapped-row count and writes the clamped value back — the pattern the commit modal already used
(`main.rs`'s `Down|Up|PageDown|PageUp` arm for `m.scroll`). A comment in its place says why the
absence is the fix, with the measurement.

**Where.** `crates/btctax-tui-edit/src/main.rs` (the `panel_open` block of
`handle_tax_inputs_key`).

**The kill, and why it is the part that matters.** The existing kill
(`draw_edit.rs::the_answer_panel_pane_draws_every_line_it_renders_and_names_the_blocking_items`)
sets `form.panel_scroll = usize::MAX / 2` and draws, so it exercises the **renderer's** clamp and
never the handler's — the fourth shadow of the arc. The new kill,
`the_filer_can_scroll_the_answer_panel_to_its_last_line_through_the_key_handler`
(`crates/btctax-tui-edit/src/main.rs`), enters where the filer enters and **never assigns
`panel_scroll` anywhere**:

- `handle_key(&mut app, press(KeyCode::Char('p')))` — the filer opens the pane;
- then the real event loop, interleaved: `terminal.draw(|f| draw_edit::draw(f, &mut app))`, then
  `handle_key(&mut app, press(KeyCode::PageDown))`, forty times — both the calls `main`'s own loop
  makes;
- then: the last line of `panel_lines()` must be on the drawn frame, and the footer's window must
  **end at the total**.

Two guards keep it from asserting nothing: the fixture is a *fresh* TY2024 Single (nothing answered,
so the panel is 141 wrapped rows against a 37-row window), and the range footer is located by
`showing ` + ` lines` on its own line. ★ The first draft matched a bare `" of "` and `"lines"`
anywhere on screen and was satisfied by a prompt's own prose (*"gift of $250 or more"*, *"Schedule A
lines 11 and 12"*) — found by running it, not by reading it.

**Observed red** (clamp planted back, `touch`ed, restored from a `cp` backup afterwards):

```
thread 'tests::the_filer_can_scroll_the_answer_panel_to_its_last_line_through_the_key_handler'
panicked at crates/btctax-tui-edit/src/main.rs:10524:9:
  wanted: (0 answered, 51 not applicable to this return)
  screen: … │ showing 39–75 of 141 lines · [↑/↓ · PgUp/PgDn] scroll · [p/Esc] close │ …
```

`showing 39–75 of 141 lines` is the review's measured ceiling, reproduced exactly. Restored: PASS.

---

## I-1 (Important) — the panel's refusing list is derived from the screen the commit gate runs

**What changed — derived, not extended.**

1. `interview_state_with` (`crates/btctax-core/src/tax/interview_state.rs`) gains a **section 6**:
   after the registry walks it runs `return_refuse::screen_param_free(ri)` — the commit screen's own
   body at its **value tier** — and pushes any refusal the registries did not already model
   (deduplicated on `RefuseReason` equality, so the registry's version, which carries the panel
   *item*, wins). `refusing` is therefore total over every `RefuseReason` that tier raises, not the
   five it could construct.
2. `Refusing.item` becomes `Option<PanelItem>`. A screen rule can read several fields at once (the
   home sale reads four), and inventing a key would point the filer's cursor at one of them as
   though it were the cause. The change is compiler-enforced: every existing constructor had to say
   `Some(item)`.
3. **The value tier, not the unanswered tier** — deliberately, and in the panel's own vocabulary.
   `Refusing` is documented as *"an answer you have already given that stops the return"*; an
   unanswered class-(A) declaration is already listed as **blocking**, with its own heading, prompt
   and cursor target. `unanswered_refuses: true` would print every blocking item a second time under
   a heading that told the filer their answer was wrong when they have not given one.
4. `panel_lines`' zero-open sentence now carries its boundary on the next line, and
   `crates/btctax-cli/LIMITATIONS.md`'s REFUSING bullet says the same thing in the filer's words.

**Where.** `crates/btctax-core/src/tax/interview_state.rs` (module doc, `Refusing`, section 6),
`crates/btctax-cli/src/cmd/answer.rs` (`panel_lines`), `crates/btctax-cli/LIMITATIONS.md`.

### ★ What the new check covers, and what it does NOT

**Covers.** Every `RefuseReason` the commit screen raises at its **param-free value tier** — measured
as **54** distinct reasons on the plant below, `HomeSaleNotComputed` among them. The guarantee is
exact and testable: *the panel's refusing list is empty only if that tier raises nothing.*

**Does not cover.** The **five package-gated rules** — `ExcessSsEmployerUnknown`,
`ExcessElectiveDeferral`, `ForeignTaxOverCeiling`, `HsaExcessContributionsNeedForm5329`,
`HsaExcessEmployerContributions`. Each compares an amount against a figure in the year's
**`TaxTable`**, and `interview_state_with` holds `Option<&FullReturnParams>` and never a `TaxTable`;
`ScreenTier::package` is a single `Option<(&TaxTable, &FullReturnParams)>`, so there is no half-tier
to run. Threading a `&TaxTable` through the walk would ripple into four production call sites and
every test that calls `interview_state_with_params`; splitting `ScreenTier::package` in two would
change the guard strings that `the_screen_tiers_are_read_off_the_source` reads verbatim out of the
commit gate's body. Both are surgery on the riskiest function in the repo, for four more reasons out
of 126. **So the gap is stated rather than closed**, on the surface and in the doc, and the surface
no longer asserts a completeness it does not have. It is also **not silent**: it is pinned by an
assertion (below), so the qualifier cannot be edited away while the claim stays.

Also outside this walk, and outside `screen_inputs` too: the compute/ledger-dependent refuse rows,
which are screened later and which the panel has never claimed anything about.

**The kills.**

1. `the_answer_panel_lists_every_refusal_the_value_tier_of_the_screen_raises`
   (`crates/btctax-core/src/tax/return_refuse.rs`, the `param_free_tier` test module) — **derived
   from the screen's own source.** It walks `param_free_fixtures()`, whose membership is pinned
   against a census read out of `screen_inputs_tiered`'s body by
   `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths`. So a rule added to
   this tier tomorrow arrives with a fixture already attached and is covered on the day. Nothing is
   enumerated by hand. For each fixture: the panel must list a refusal, and it must be *that* one
   (its exit sentence, or its reason).

   Planted `screen_param_free` away (back to the five hand-built reasons):

   ```
   panicked at crates/btctax-core/src/tax/return_refuse.rs:9026:9:
   the commit screen refuses these returns and the answer panel says nothing is refusing, so the
   filer meets the refusal at the write instead of while authoring: ["NegativeAmount",
   "Schedule1aTipsFromTradeOrBusiness", "QualifiedTipsCautionNotMet", … 51 more …,
   "DirectDepositNumberMalformed", "HomeSaleNotComputed"]
   ```

   54 reasons, the review's example included.

2. `the_panel_does_not_claim_nothing_refuses_when_the_commit_screen_refuses`
   (`crates/btctax-cli/src/cmd/answer.rs`) — the review's own J-32 fixture, asserted on the
   **rendered lines**. TY2024 Single, `answer_all_live_declarations`, then every live skippable
   answered through the registry to a fixpoint (so `open_items() == 0` and the sentence's branch is
   actually reached — asserted, so the test cannot go vacuous), then `sold_main_home = Some(true)`,
   both tests `Some(true)`, `can_exclude_all_gain = Some(false)`, `S1099 = Some(false)`. The refusal
   is read off `screen_inputs` rather than typed. Same plant:

   ```
   panicked at crates/btctax-cli/src/cmd/answer.rs:2181:9:
   the commit gate refuses this return (HomeSaleNotComputed("you said you cannot exclude all of your
   gain")) and the panel told the filer nothing refuses: [
       "── The answer panel (tax year 2024) ──",
       "  nothing is open: every live question is answered, nothing is forgone, and no answer refuses.",
       "  (39 answered, 47 not applicable to this return)",
   ]
   ```

   Byte-identical to the review's `PROBE` output, `(39 answered, 47 not applicable to this return)`
   included. The same test then takes the same return **without** the sale, asserts the zero-open
   sentence IS printed, and asserts the boundary line rides beside it — so the qualifier and the
   claim cannot be separated by a later edit.

---

## I-2 (Important) — the undated-row walk is the enum

**What changed — the list is deleted, not extended.**

1. `DocumentKind::ALL` (9 families) is added.
2. `document_identity`'s match is replaced by **one** exhaustive match, `document_row_facts(ri, kind,
   i) -> DocumentRowFacts`, carrying designation, **row count**, issuer, TIN **and the transcription
   date**. A new family is a compile error there until all five are named.
3. `TranscribedOn { NoColumn, Row(Option<Date>) }`. `NoColumn` is a **declaration** — the W-2's row
   struct has no such field — replacing the prose exception the old function carried. Two blanks look
   identical on the page and are not the same thing.
4. `undated_document_rows` walks `DocumentKind::ALL` and reads each family's dates out of that match.
   `document_identity` is now a thin formatter over the same facts. Output wording is unchanged.

**Where.** `crates/btctax-core/src/tax/provenance.rs`.

### ★ What the new check covers, and what it does NOT

**Covers, two directions.**

- **A ninth `DocumentKind`** → a **compile error** in `document_row_facts` and in the test fixture
  builder `push_undated_row`. It cannot be a silent gap.
- **An existing family that GAINS a `transcribed_on` column** — the half the compiler cannot see, and
  the W-2 is one `pub transcribed_on: Option<Date>` away from being it. `the_transcription_date_
  columns_in_the_source_are_all_walked` counts `pub transcribed_on: Option<Date>,` in
  `return_inputs.rs` (via `include_str!`, the discipline `return_refuse.rs` already uses for its tier
  census) and requires the walk to declare the same number, with a `>= 8` vacuity guard so a broken
  parse is loud rather than permissive.
- **Every family with a date column, end to end**: `every_document_family_that_carries_a_
  transcription_date_is_named` pushes one undated row of **every** `DocumentKind` and asserts
  `undated_document_rows` returns exactly the expected list, in `ALL` order — with the expectation
  built from `document_row_facts`'s own declarations, never typed. (`>= 8` guard again.)

**Does not cover.**

- `DocumentKind::ALL` **listing** every variant is held by a count assertion
  (`every_document_kind_is_listed_once`, `ALL.len() == 9` plus a duplicate check) — the same shape
  `DocumentRow::ALL` uses. A variant added and given a `document_row_facts` arm but left out of `ALL`
  would compile; the count then reds, but the count is a number someone can update. This is the
  repo's existing pattern and I did not invent a stronger one.
- The check is at the **function**, not at each surface. `undated_document_rows` is the single source
  both consumers call (`render.rs`'s §4.4 block, `admin.rs`'s manifest), and both have tests that
  they call it — so totality at the function is totality at both surfaces. What remains true is the
  review's FR-88 observation: the **surface** tests still populate `int_1099` and only `int_1099`. I
  did not add an undated Form 1098 / 1099-SA / 5498-SA row to the packet fixture because each needs a
  coherent surrounding return (a 1098 row is live only under an itemize election and carries four
  refuse-guards of its own; a 1099-SA needs `hsa_activity` and Form 8889), and getting the packet to
  export would have been a fixture project rather than a fold. Stated, not hidden.

**Observed reds** (both plants reverted from `cp` backups, `touch`ed):

*Plant A — a family's date column stops being walked* (`Form1098 => TranscribedOn::NoColumn`):

```
panicked at crates/btctax-core/src/tax/provenance.rs:1234:9:
assertion `left == right` failed: `return_inputs.rs` declares 8 `transcribed_on` columns and
`undated_document_rows` walks 7 families that have one. A column that nothing walks is an undated
row nobody is told about, on two surfaces that call themselves complete.
  left: 7
 right: 8
```

…and `every_document_family_that_carries_a_transcription_date_is_named` red beside it.

*Plant B — the W-2 GAINS a column, and the author answers every compiler complaint.* Adding
`pub transcribed_on: Option<Date>,` to `W2` produced three `E0027`s (`classifier.rs:732`,
`return_refuse.rs:1086`, `scrub.rs:1021`) and one `E0063` (`scrub_axis.rs:222`). I fixed all four —
the realistic failure, where the mechanical errors are answered and the walk is forgotten. **The
compiler was then fully satisfied and the check still red:**

```
panicked at crates/btctax-core/src/tax/provenance.rs:1234:9:
assertion `left == right` failed: `return_inputs.rs` declares 9 `transcribed_on` columns and
`undated_document_rows` walks 8 families that have one. …
  left: 8
 right: 9
```

All five files restored from backups; `git diff` over them is empty.

---

## M-1 (Minor) — the manifest's FORGONE block carries the NOT COMPUTED instruction

**What changed.** `not_computed_lines(st)` is extracted in `crates/btctax-cli/src/cmd/answer.rs`,
re-exported at the crate root beside its three siblings, and used by **both** `panel_lines` and
`admin.rs::forgoing_block` — heading included, with the same heading-vs-bullet split
`forgoing_lines` gets. Render, never recompute: the manifest was appending `n.line()` alone, so on
paper 1040 line 19 read as a benefit the filer *chose* to skip and the sentence telling them *the
credit boxes on your return are printed, the amount is yours to enter* was gone.

**Where.** `crates/btctax-cli/src/cmd/answer.rs`, `crates/btctax-cli/src/lib.rs`,
`crates/btctax-cli/src/cmd/admin.rs`.

**The kill.** The existing manifest test
(`crates/btctax-cli/tests/export_irs_pdf.rs::the_manifest_lists_the_forgone_benefits_marking_the_
declined_ones_and_names_the_undated_rows`) gains a credit-box dependent (one `Dependent` +
`answer_all_live_declarations`, which the row makes necessary — `FilerTinIssuedByDueDate` goes live),
an assertion that `not_computed` is non-empty *"or the M-1 assertions are vacuous"*, and a loop
requiring every line of `not_computed_lines(&st)` to appear in the manifest. Comparison is on the
comment-stripped, whitespace-flattened manifest — the block wraps and prefixes each row with `#`, and
a `#` from a continuation row otherwise lands in the middle of the sentence.

**Observed red** (fix planted back to `for n in &st.not_computed { push(&mut s, &n.line()); }`):

```
panicked at crates/btctax-cli/tests/export_irs_pdf.rs:3124:9:
the manifest must carry the NOT COMPUTED block the panel renders — heading and instruction included.
  wanted: NOT COMPUTED (1) — btctax does not file the schedule these are figured on; the credit
          boxes on your return are printed, the amount is yours to enter:
```

---

## M-2 (Minor) — the pane's `[p/Esc] close` legend survives a narrow terminal

**What changed.** The pane's footer is pre-wrapped through `wrap_to` and the window is sized against
its **actual row count** — `footer_of(total, total, true)`, the widest variant, so the window can only
be conservative. A `debug_assert!` pins that the assembled pane fits its own box. Exactly what
`draw_tax_inputs_modal::legend_of` does, one pane over. At 120×40 `view_h` is 37 as before, so no
golden moved.

**Where.** `crates/btctax-tui-edit/src/draw_edit.rs` (`draw_tax_inputs_panel`).

**The kill.** `the_answer_panel_pane_keeps_its_close_legend_at_every_terminal_width` renders the pane
at the review's seven sizes (120×40, 100×30, 80×24, 80×20, 72×16, 60×12, 40×10) and requires
`[p/Esc] close` on screen at each — it is the only printed way out of a full-screen read-only
overlay.

**Observed red** (footer reverted to one unwrapped line):

```
panicked at crates/btctax-tui-edit/src/draw_edit.rs:6939:13:
at 72×16 the only printed way OUT of a full-screen overlay was cut off:
┌ ┌ Answer panel ──────────────────────────────────────────────────┐─┐
```

**Not covered:** at 40×10 and 40×6 the review also measured `tail_reachable=false`. Both are far below
this repo's documented 80×24 floor and outside M-2, which was the legend; the legend now holds at
40×10 and is asserted there.

---

## N-1, N-2, N-3

- **N-1** — the earlier of the two near-identical *"T12 — A PAYLOAD TALLER THAN THE TERMINAL
  SCROLLS"* comment blocks in `draw_tax_inputs_modal` is deleted
  (`crates/btctax-tui-edit/src/draw_edit.rs`). The surviving copy is the one with the pre-wrap
  paragraph.
- **N-2** — `crates/xtask/src/r15_stop_list.rs`: *"The three checks"* → *"The four checks"* and
  *"each of the three greps"* → *"each of the four greps"*. The module header's *"Three checks, three
  sentences"* is corrected too, with the fourth row added to its table and a note that *"no progress
  bar; no persisted 'what remains'"* is two claims needing a grep each.
- **N-3 — fixed, and made derived rather than listed.** `renderer_sources()` went from
  `draw_edit.rs` alone to all **six** files that render, print or write the panel (`draw_edit.rs`,
  `edit/form.rs`, `main.rs`, `cmd/answer.rs`, `render.rs`, `cmd/admin.rs`). Extending a list would
  have re-armed the same defect, so the new
  `every_file_that_renders_the_panel_is_in_the_progress_widget_checks_field_of_view` **derives** the
  expected set by scanning both crates' `src` for callers of `panel_lines` / `forgoing_lines` /
  `refusing_lines` / `not_computed_lines` and requiring each to be in the check's field of view, with
  a `>= 6` vacuity guard. Observed red on the narrowed list:

  ```
  panicked at crates/xtask/src/r15_stop_list.rs:517:9:
  these files render the answer panel and the progress-widget check cannot see them, so a
  hand-rolled bar there is caught by nothing: ["crates/btctax-cli/src/cmd/admin.rs",
  "crates/btctax-cli/src/cmd/answer.rs", "crates/btctax-cli/src/render.rs",
  "crates/btctax-tui-edit/src/edit/form.rs", "crates/btctax-tui-edit/src/main.rs"]
  ```

  `FOLLOWUPS.md` FR-98 is amended to record the widening; the residue it actually owns (a *stored*
  progress field on a TUI struct) is unchanged and still open.

---

## Deviations

1. **I-1 was fixed with BOTH halves the brief offered, not one.** The brief said *"either derive the
   list from the same screen the commit gate runs (preferred) **or** stop making the universal
   claim."* Deriving alone leaves five package-gated rules the walk cannot run, so the unqualified
   sentence would still have been false — narrowly, but on the same surface and for the same reason.
   Deriving is the fix; the qualifier is what makes the remaining claim true. Doing only one would
   have left a surface asserting completeness it does not have, which is what the finding was.

2. **I-1 runs the screen at the VALUE tier (`unanswered_refuses: false`), not the commit gate's own
   tier.** The commit gate runs `unanswered_refuses: true`. Copying that would push every unanswered
   class-(A) declaration into `refusing`, where the panel already lists it — correctly, and with a
   cursor target — as `blocking`. The reviewer's own wording (*"an answer you have already given"*)
   and the `Refusing` doc both say value tier. Recorded because it is a narrower read of *"the same
   screen the commit gate runs"* than the sentence alone implies.

3. **`Refusing.item` became `Option<PanelItem>`** — a struct change the brief did not ask for. A
   screen-raised refusal owns no single registry answer, and the alternative was to invent a key,
   which would point the filer's cursor at one of the four fields the home-sale rule reads as though
   it were the cause. The change is compiler-enforced and nothing in production read the field.

4. **I-2's totality has two mechanisms, not one**, because one is not enough: the exhaustive match
   cannot see a family that gains a `transcribed_on` column, and the source count cannot see a family
   added with no column. Both were watched red separately (plants A and B).

5. **M-1 was fixed by extracting `not_computed_lines`** rather than by emitting a heading line in
   `admin.rs`. The brief's phrasing (*"emit the `NOT COMPUTED (n) — …` heading line before the
   loop"*) would have put the panel's sentence in a second place; T12's whole rule is render, never
   recompute.

6. **N-3 was fixed rather than documented, and made derived.** The brief allowed *"fix or state the
   limit"*. The one-line source list would have gone stale the same way `undated_document_rows` did,
   which is the finding two entries above it.

7. **`FOLLOWUPS.md` FR-98 was edited** — not in the brief. N-3 changed the fact FR-98 cites
   (`progress_widgets` over `draw_edit.rs`), and leaving a follow-up citing the pre-fold scope is the
   stale-citation shape this repo keeps paying for. The residue FR-98 owns is untouched.

8. **Not done: an undated Form 1098 / 1099-SA / 5498-SA row on the packet-manifest fixture.**
   Reasoned above under I-2's "does not cover". The core-level walk is total; the surface fixture is
   not, and that is stated rather than papered over.

---

## Files touched

```
 FOLLOWUPS.md                                  |   9 +-
 crates/btctax-cli/LIMITATIONS.md              |   8 +-
 crates/btctax-cli/src/cmd/admin.rs            |  16 +-
 crates/btctax-cli/src/cmd/answer.rs           | 162 ++++++++++++-
 crates/btctax-cli/src/lib.rs                  |   2 +-
 crates/btctax-cli/tests/export_irs_pdf.rs     |  48 ++++
 crates/btctax-core/src/tax/interview_state.rs |  73 +++++-
 crates/btctax-core/src/tax/provenance.rs      | 325 ++++++++++++++++++++++----
 crates/btctax-core/src/tax/return_refuse.rs   |  49 ++++
 crates/btctax-tui-edit/src/draw_edit.rs       | 123 +++++++---
 crates/btctax-tui-edit/src/main.rs            | 136 ++++++++++-
 crates/xtask/src/r15_stop_list.rs             |  84 ++++++-
 12 files changed, 929 insertions(+), 107 deletions(-)
```

## The eight new tests, with the defect each was watched red on

| test | file | planted defect |
|---|---|---|
| `the_filer_can_scroll_the_answer_panel_to_its_last_line_through_the_key_handler` | `btctax-tui-edit/src/main.rs` | the handler's logical-line clamp restored |
| `the_answer_panel_lists_every_refusal_the_value_tier_of_the_screen_raises` | `btctax-core/src/tax/return_refuse.rs` | `screen_param_free` call removed from the walk |
| `the_panel_does_not_claim_nothing_refuses_when_the_commit_screen_refuses` | `btctax-cli/src/cmd/answer.rs` | (same plant) |
| `every_document_family_that_carries_a_transcription_date_is_named` | `btctax-core/src/tax/provenance.rs` | `Form1098 => TranscribedOn::NoColumn` |
| `the_transcription_date_columns_in_the_source_are_all_walked` | `btctax-core/src/tax/provenance.rs` | (same plant) **and** a `transcribed_on` added to `W2` with all four compiler errors answered |
| `every_document_kind_is_listed_once` | `btctax-core/src/tax/provenance.rs` | count/duplicate guard for `DocumentKind::ALL` |
| `the_answer_panel_pane_keeps_its_close_legend_at_every_terminal_width` | `btctax-tui-edit/src/draw_edit.rs` | the pane footer reverted to one unwrapped line |
| `every_file_that_renders_the_panel_is_in_the_progress_widget_checks_field_of_view` | `xtask/src/r15_stop_list.rs` | `renderer_sources()` narrowed to `draw_edit.rs` |

Plus one existing test extended with M-1's kill:
`the_manifest_lists_the_forgone_benefits_marking_the_declined_ones_and_names_the_undated_rows`.

---

## Standing lessons, as they landed

- **Enter at the layer the filer enters at.** C-1's kill presses `p`, then `PageDown`, and reads the
  drawn frame — `draw_edit::draw` + `handle_key`, the event loop's own two calls. It contains no
  assignment to `panel_scroll`. Its first draft *did* reach past the layer in a smaller way (a
  substring guard a prompt satisfied), and running it is what found that.
- **A surface that presents a complete list must have one.** I-1 and I-2 are now derived; where a
  gap remains (I-1's package tier) the surface says so and an assertion holds the two together.
- **Derive; never type a list beside a set that grows.** Three hand lists deleted this fold —
  `undated_document_rows`' five families, `refusing`'s five reasons, `renderer_sources()`' one file —
  and none was replaced with a longer list.
- **Render, never recompute.** M-1's fix is a fourth shared renderer, not a second heading.
