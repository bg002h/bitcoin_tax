# REPORT — `form-delta`'s three missing axes, built. FR-190 / FR-191 / FR-192.

**Agent:** opus, own worktree `.claude/worktrees/agent-adde5e59929bb2397`,
`CARGO_TARGET_DIR=<worktree>/target-axes`. **Not committed, not pushed.** Diffstat at the time of
writing:

```
 crates/xtask/src/form_delta.rs   | 1793 ++++++++++++++++++++++++++++++++++++--
 crates/xtask/src/label_reader.rs |  318 +++++++
 design/TY2026_WORK_LIST.md       |   59 +-
 3 files changed, 2065 insertions(+), 105 deletions(-)
```

**Gate (`make gate` — forced rebuild, because the plant/restore mutation runs race mtime):**
`3670 tests run: 3670 passed, 12 skipped`; `make lint` clean; `cargo fmt --all --check` clean.
`form_delta`'s own test count went **12 -> 32**.

---

## 0. Verdict against the brief, item by item

| brief item | result |
|---|---|
| Axis A compares across a container rename | **DONE** — `f1040s3--2021 -> --2022` went `0 common / 40 added / 41 removed` to `40 paired / 0 added / 1 removed` |
| Axis A: plant a root-subform rename on an otherwise identical pair | **DONE** — 3 stems, all boxes pair, all reported as respelled |
| Axis B: line-SET axis joined to `label-census` | **DONE** — Schedule 8812 TY2021->TY2022 now says `34 retired` and **names all 34** |
| Axis C reds on Schedule A 13/14/15/16/**17**/18 | **5 of 6.** 13, 14, 15, 16 and 18 are reported as collisions. **Line 17 is reported as a NAMED GAP, not a collision** — see F1. It is not silently passed. |
| Axis C reds on Schedule 3 lines 7 and 8 (2020->2021) | **DONE** — and 9 of 13 in total, exactly FR-192's figure |
| Axis C reds on Schedule 1-A 37->43 | **DONE** |
| Axis C must NOT red on a pure renumber where the caption travels | **DONE** — `f8995--2024 -> --2025` (9 boxes respelled, printed form unchanged): 18 of 18 COMPARED, none describing something else, 0 gaps. Three more clean pairs in 3.2. |
| FR-187: catch an eligibility change on a line that ALSO moved; report both facts | **DONE** — both facts printed, and a dedicated test reds on the "it's only a renumber, skip it" shortcut |
| "Materially changed" defined in the source; whitespace and dot leaders irrelevant | **DONE** — section 4 |

---

## 1. Axis A (FR-190) — container-insensitive pairing

**What was wrong, reproduced first:** the label axis keyed on the full AcroForm FQN.
`f1040s3--2021`'s root subform is `form1[0]`, `--2022`'s is `topmostSubform[0]`, nothing else differs.
Full-FQN intersection **0**.

**The fix is a suffix ladder.** For `k` from the deepest FQN down to 1, pair the still-unpaired boxes
whose last `k` dot-segments are equal **and unique on both sides among the still-unpaired**.

Three properties, each held by a test:

* **Monotone by construction.** `k` at maximum depth *is* the full FQN, so every exact match is taken
  first and the ladder can never pair fewer boxes than the old key.
  Measured over the 58 consecutive pairs the 87 committed fixtures form:
  **full-FQN 3373 boxes, suffix ladder 4177, improved on 37 of 58 pairs.**
* **Never merges two boxes that share a leaf.** Uniqueness is required on BOTH sides.
  `fw2--2024` has **272 boxes and only 92 distinct leaves**; a leaf-only key would collapse 272 to 92
  and then compare the wrong boxes against each other — trading FR-190's blindness for a wrong
  answer, which is worse. The ladder pairs all 272 at the top and never reaches the leaf.
  Injectivity is asserted on every pair in the corpus.
* **The full FQN is kept for reporting.** A paired box whose spelling changed is listed in
  `Pairing::renamed` and printed, because the bundled map stores full FQNs and **each rename is a map
  edit**. `f8995--2024 -> --2025` reports 9 such respellings that the old key could not name at all,
  because the boxes had fallen out of `common` entirely.

**Accounting invariant** (new, tested on the whole corpus):
`old.len() == paired + removed + ambiguous(old)` and `new.len() == paired + added + ambiguous(new)`.
An unpaired box whose leaf exists on the other side is reported as **ambiguous**, not as removed —
"two copies of this form share this leaf" is a different fact from "this box was retired".

Output, `xtask form-delta f1040s3--2021 f1040s3--2022`:

```
form-delta f1040s3--2021 -> f1040s3--2022
  fields: 40 paired, 0 added, 1 removed
    - form1[0].Page2[0].Line13z_ReadOrder[0].f2_13[0]
  * 40 paired field(s) were RESPELLED — the box is the same box, so it is compared, but the bundled
    map stores the FULL FQN and every one of these is a map edit:
    form1[0].Page1[0].Line6z_ReadOrder[0].f1_20[0]
        -> topmostSubform[0].Page1[0].Line6z_ReadOrder[0].f1_20[0]
    ... (39 more)
  line->label: 38 of 40 common field(s) COMPARED; none of them changed the printed line it sits beside
  * 2 of 40 common field(s) yielded NO comparable label. ...
  line set: 37 printed line number(s), none retired, none introduced
  *** 4 of 37 COMPARED surviving line number(s) now print a MATERIALLY DIFFERENT caption ...
EXIT=0
```

Before this change that same command printed `0 common, 40 added, 41 removed`, the banner
`LINE->LABEL DRIFT UNWITNESSED — 0 of 0 common field(s)`, and **exit 1** — the loudest output in the
corpus for its calmest transition.

---

## 2. Axis B (FR-191) — the line SET

`label-census` could always answer *"which line numbers does this revision print"*; nothing joined it
to `form-delta`. It is now the same reader: axis B's label set is `witness_text`'s, the function
`label-census` prints from, reached through the caption reader so there is one reader with two
consumers rather than two readers that can drift.

Output, `xtask form-delta f1040s8--2021 f1040s8--2022`:

```
  fields: 20 paired, 21 added, 51 removed
  ** 20 of 20 COMPARED field(s) (20 common) KEPT THEIR NAME but now sit beside a different printed line
  ** the PRINTED LINE SET changed: 34 retired, 3 introduced, 29 survived. A retired line number that
     code still reads reads a BLANK, and a blank and a zero are the same thing on the page:
    retired:    4a 4b 4c 14a 14b 14c 14d 14e 14f 14g 14h 14i 15a 15b 15c 15d 15e 15f 15g 15h 28a 28b
                29 30 31 32 33 34 35 36 37 38 39 40
    introduced: 4 14 15
  *** 9 of 29 COMPARED surviving line number(s) now print a MATERIALLY DIFFERENT caption ...
```

FR-191 measured "34 killed printed lines" against an output that said *"54 removed"* field names,
eight shown, none labelled. The count now matches and every retired number is named.

---

## 3. Axis C (FR-192) — the line MEANING. This is the one.

For every line number **both** revisions print, the printed caption is read from the text layer and
compared. A materially changed caption under an unchanged number is the collision.

### 3.1 The three known positives

**Schedule A 2025 -> 2026-DRAFT (FR-185, FR-187).** 8 collisions of 21 compared:

```
  *** 8 of 21 COMPARED surviving line number(s) now print a MATERIALLY DIFFERENT caption ...
    line 8d:
        WAS: reserved for future use
        NOW: mortgage insurance premiums see instructions
        -> line 8d's old text prints nowhere on the new revision — this meaning was RETIRED, not moved
    line 8e:
        WAS: add lines 8a through 8c
        NOW: add lines 8a through 8d
    line 13:
        WAS: carryover from prior year
        NOW: enter the amount from line 6 of the charitable contribution limitation worksheet
        -> line 13's old text now prints at line 14 (verbatim) — a pure renumber, and every
           cross-reference to 13 must become 14
    line 14:
        WAS: add lines 11 through 13
        NOW: carryover from prior year
    line 15:
        WAS: casualty and theft loss es from a federally declared disaster other than net qualified ...
        NOW: add lines 13 and 14
        -> line 15's old text now prints at line 16 AND WAS REWORDED (similarity 0.90) — the number
           moved and the RULE changed; a move table recording "15 -> 16, same quantity" is right about
           the number and wrong about the rule
    line 16:
        WAS: other from list in instructions list type and amount
        NOW: casualty and theft loss es from a federally or state declared disaster other than net ...
    line 18:
        WAS: if you elect to itemize deductions even though they are less than your standard deduction
             check this box
        NOW: is the amount on form 1040 or 1040 sr line 11b minus the amounts on lines 13a and 13b of
             that form more than $384 350 no your deductions are not limited add the...
        -> line 18's old text now prints at line 19 (verbatim)
  * 2 of 23 surviving line number(s) yielded NO caption comparison. The verdict above says nothing
    about these:
    2 — the NEW revision prints this label more than once: 17 5
```

Line 18 is FR-185's worst shape — a **checkbox becomes a six-figure dollar amount** — and the axis
reports it plus the travel of the checkbox to line 19. Line 13 is FR-185's adverse one, in the
understating direction.

**Schedule 3 2020 -> 2021 (FR-192).** 9 of 13 COMPARED — exactly FR-192's "nine of thirteen":

```
    line 7:
        WAS: add lines 1 through 6 enter here and on form 1040 1040 sr or 1040 nr line 20 part ii other
             payments and refundable credits
        NOW: total other nonrefundable credits add lines 6a through 6z
        -> line 7's old text now prints at line 8 AND WAS REWORDED (similarity 0.62)
    line 8:
        WAS: net premium tax credit attach form 8962
        NOW: add lines 1 through 5 and 7 enter here and on form 1040 1040 sr or 1040 nr line 20
        -> line 8's old text now prints at line 9 (verbatim) — every cross-reference to 8 must become 9
```

Line 7 is the double-count: the Part I total became an "other credits" subtotal, so carrying the old
cross-reference forward adds lines 1-5 twice. The whole 8->9->10->11->12->13 travel chain is printed.

**Schedule 1-A 37 -> 43.**

```
    line 37:
        WAS: enhanced deduction for seniors add lines 36a and 36b part vi total additional deductions
        NOW: enter the amount from line 3
        -> line 37's old text now prints at line 43 AND WAS REWORDED (similarity 0.75)
```

### 3.2 The negative case — the axis is not a noise generator

`f8995--2024 -> f8995--2025`: 9 boxes respelled, 9 added, 9 retired, printed form unchanged.

```
  fields: 24 paired, 9 added, 9 removed
  * 9 paired field(s) were RESPELLED ...
  line->label: 24 of 24 common field(s) COMPARED; none of them changed the printed line it sits beside
  line set: 18 printed line number(s), none retired, none introduced
  line meaning: 18 of 18 surviving line number(s) COMPARED by printed caption; none of them describes
                something else
EXIT=0
```

Three more clean, all with every line compared and zero gaps:
`f1040s3--2024 -> --2025` (35/35), `f1040sd--2020 -> --2021` (24/24), `f8959--2024 -> --2025` (24/24).
`f1040sd--2025 -> --2026-DRAFT` is the only form in the whole TY2026 work list whose shape cell still
reads `unchanged`.

### 3.3 FR-187: a moved line is still compared at its own number

`caption_axis` compares **every** surviving number, then separately reports where a changed line's old
text turned up — `Verbatim { to }`, `Reworded { to, similarity }`, or `None` (the meaning was retired,
not moved). Travel is a report, never a licence to skip.

**Watched red:** planting the obvious shortcut — `if o != n && travel_of(l, o, new).is_none()`, i.e.
"skip a line whose text is found elsewhere, it is only a renumber" — reds
`axis_c_catches_an_eligibility_change_on_a_line_that_also_moved`,
`axis_c_reds_on_the_schedule_a_cascade`, `axis_c_reds_on_schedule_3_nine_of_thirteen`,
`axis_c_reds_on_schedule_1a_37_to_43`, `travel_reports_...`, and both work-list tests (8 failures).

---

## 4. "Materially changed", defined and defended

In `label_reader::normalise_caption_text`:

> Two captions differ materially **iff their token sequences differ**, where a token is a maximal run
> of `[0-9a-z$%]` after lowercasing and **every other character is a separator**.

So a dot-leader run, a line break, a reflowed space, a curly vs straight apostrophe, an em dash vs a
hyphen and a parenthesis are all invisible. Every word, every cross-referenced line number and every
printed dollar figure is load-bearing. **Stated boundary:** a change that is only punctuation is by
this definition not material, and `$384,350` becomes the two tokens `$384` and `350` — consistent on
both sides, so it compares, but it would not distinguish `384,350` from `384350`.

Held by `materially_changed_ignores_layout_and_never_ignores_a_word_or_a_number`, whose "must differ"
cases include the real Form 6251 line-33 defect (`from line 22` vs `from line 12`), FR-187's widened
eligibility clause, an inflation-adjusted threshold, and a **reordering** (`Subtract line 12 from line
22` vs `Subtract line 22 from line 12`) — which is why the comparison is on the token *sequence* and
not the token set.

### Four reader rules, each forced by a measured false positive or negative

1. **The body-height ceiling is derived PER PAGE** (modal word height x 1.35), not typed.
   The first cut was an absolute 12.0pt, measured across all 87 fixtures as sitting in a real gap
   (largest body word 11.95pt, smallest heading 12.43pt). **`f1040s3--2020` sets its body text at
   12.83pt over 341 of its 463 words**, so every caption on Schedule 3 for TY2020, 2021 and 2022 came
   out EMPTY and axis C reported all three pairs as **clean**. That is this repo's dominant failure
   shape, produced by one edit of mine, and it is now pinned by
   `the_body_height_ceiling_is_derived_per_page_not_typed`.
2. **The `DRAFT` / `DO NOT FILE` watermark must never reach a caption.** A draft prints it down both
   margins at 16.0-53.63pt where its body is 9.33-10.49pt, and those spans overlap real lines.
   January's whole job is draft-to-final and the final has no watermark, so one watermark token reds
   every line it touches. `caption_join_tuned(g, factor)` exists purely so the ceiling can be
   **watched red**: at 6.0x the token `draft` contaminates **19 captions** across the drafts; at the
   shipped ceiling **84 of 87 fixtures are clean and none carries it**.
3. **Locators are dropped, in three widening rules.** An IRS form prints each line's number twice —
   the margin and the gutter beside the box — and the gutter one lands inside the row being read. It
   is identical on both sides so it cannot itself flag a change; what it does is **move**, because the
   box it sits beside moves. On `f8995--2024 -> --2025` the gutter `3` sits **12.30pt** left of its box
   in TY2024 and **11.30pt** in TY2025 (`label_join`'s calibrated window is 12.0, and its own doc
   records that 1.0pt gap), and that alone reported lines 3 and 7 as reworded. So rule 3 absorbs the
   **stragglers of a confirmed gutter column** — a label-shaped word sharing a 2pt right-edge bucket
   with three or more confirmed ones. Removing rule 3 reds
   `axis_c_is_silent_on_a_rename_that_changed_no_printed_line`.
4. **Reading order groups words into visual LINES first.** Ordering on a rounded `y` let a one-point
   shift swap two words and report a caption as reworded when only the layout moved
   (`f1040sse--2024 -> --2025` line 15, `f1040sa--2024 -> --2025` line 12). Both pairs are now pinned
   cell for cell by `a_reflowed_caption_is_not_a_reworded_line`.

**Axis C's accounting invariant:** compared + gaps == surviving line numbers, always, with five named
`CaptionGap` reasons (ambiguous old/new, empty old/new/both). **Zero comparisons is its own verdict and
exits non-zero**, the same discipline the label axis already had. Held on hand-built inputs *and* on
every pair in the corpus (47 witnessed pairs, 11 blind, and no blind pair reports a collision).

---

## 5. Findings — things a reviewer should know

**F1 (Important for the TY2026 port, NOT a defect in this work). Schedule A line 17 is a named GAP,
not a collision, and the cause is a pre-existing `witness_text` limitation.**
`xtask label-census f1040sa--2026-DRAFT` prints **36 labels, including `5` twice and `17` three times**:

```
    p1 5     / p1 5b / p1 5c / p1 5d / p1 5        <- 5a and 5e never resolved
  H p2 17    / p2 17a ... p2 17f / p2 17 / p2 17i / p2 17j / p2 17 / p2 17z
```

The margin sub-letters `a`, `e`, `g`, `h` and `k` are not picked up, so those rows collapse onto their
bare parent. Axis C therefore refuses to compare `17` and `5` (their captions are ambiguous) and
**names them**, which is the honest behaviour but still means FR-185's line-17 collision (the TOTAL
becoming "Other itemized deductions", enumerated 17a-17z) is not asserted by a test. This is the same
family as FR-58 (`f1040s1--2026-DRAFT`'s label set is a hard refusal for the same reason), and fixing
it means changing `column_tokens`' sub-letter window, which moves the label set of every fixture.
**Recommend a follow-up owned by the TY2026 port, before Schedule A is wired.** I deliberately did not
touch it — it is `label-census`'s own gate, and a change there re-bases three other checkers.

**F2 (Minor, found in my own work, already fixed and pinned). Adding two columns silently demoted all
eight existing work-list plants to the excused arm, and the test stayed GREEN.** The shape cell landed
where an axis cell was expected, so `numeric` parsed as `None`, and each plant then redded on
*"excused as having no pair, but form-delta computes one"* rather than on the cell it was planting. A
plant redding for the wrong reason is an instrument nobody is watching. Fixed structurally: the row
parser now returns a `NumericRow` **struct**, so a new axis is a build error at every read site, and
every plant is asserted to parse as a numeric row before it is checked.

**F3 (Minor, mine, fixed). An asymmetric locator filter is worse than none.** Schedule A's line-15
caption says *"enter the amount from line 18 of that form"*, and the prose `18` wraps to x2=407.46 on
TY2024 and x2=404.09 on TY2025 — inside the gutter column on the second revision only. The token was
eaten from one caption and kept in the other, and the line reported itself reworded with an identical
sentence. Fixed by requiring a rule-3 locator to have a box on its own row, which a prose number never
does. The reproduction is in the source comment.

**F4 (Minor, for the ledger). The work-list caption cell counts collisions among COMPARED lines and
does not carry the gap count.** `f1040sa` prints `8` in the new column while 2 of its 23 surviving
numbers were not compared at all. `form-delta` prints the gaps; `port-status` does not. I chose not to
add a ninth column. **Recommend a follow-up**, because a number in a table is read as complete.

**F5 (informational — what the new columns immediately say about TY2026).** Regenerating
`design/TY2026_WORK_LIST.md` moved every row:

| form | was | now (retired / meaning changed) |
|---|---|---|
| `f6251` | `unchanged` | **1 retired, 8 line numbers whose MEANING changed** |
| `f8959` | `unchanged` | 0 retired, **12 changed** |
| `f8960` | `unchanged` | 0 retired, 2 changed |
| `f1040sb` | `unchanged` | 0 retired, 2 changed |
| `f8995a` | `port`, 0 moved | 0 retired, 6 changed |
| `f1040s1a` | 1 moved | 7 retired, **33 changed** |
| `f1040s2` | 23 moved | **20 retired, 18 changed** |
| `f1040sc` | 8 moved | 0 retired, **16 changed** |
| `f1040sa` | 2 moved | 5 retired, 8 changed (FR-185) |
| `f1040sd` | `unchanged` | 0 / 0 — **the only genuinely unchanged form in the table** |

**Form 6251 was listed as `unchanged` for TY2026 and moves the meaning of 8 line numbers.** That is
the single most consequential line in this report: the AMT form is the one the packet is most exposed
on, and the old work list said there was nothing to do on it.

---

## 6. Scope discipline

**Touched:** `crates/xtask/src/form_delta.rs` (all three axes, the printer, `port_status`, 20 new
tests), `crates/xtask/src/label_reader.rs` (the caption reader, the locator rules, the normaliser,
`label_column_edges`, `caption_join_tuned`), `design/TY2026_WORK_LIST.md` (regenerated from the
printer, as its own prose instructs).

**NOT touched, as instructed:** nothing under `crates/btctax-core/`, no `forms/**` map, nothing in
`scripts/`, no year bundled, no document re-archived. `form_geometry.rs` needed no change — the
caption reader reads the committed fixture through the existing `load` / `box_top_down_y`.

**Not done:** F1's `witness_text` sub-letter fix (out of scope, and it re-bases three other checkers),
F4's ninth work-list column, and the `FOLLOWUPS.md` ledger entries for F1 and F4 — the controller owns
the ledger.

**Mutation runs performed** (each restored from a backup copy, and `make gate` re-run with a forced
rebuild afterwards): ladder reduced to exact-FQN only (7 failures), axis C skipping renumbers (8),
locator rule 3 removed (3), reading order on a rounded `y` (2), the work-list retired-cell check
neutralised (1), the work-list caption-cell check neutralised (1). No mutation left the suite green.

---

## 7. The 20 new tests

Axis A: `axis_a_compares_across_a_root_container_rename`,
`axis_a_still_compares_when_a_planted_rename_is_the_only_difference`,
`the_ladder_never_pairs_fewer_boxes_than_the_full_fqn_key`,
`every_box_is_paired_added_removed_or_named_ambiguous`,
`the_ladder_never_merges_two_boxes_that_share_a_leaf`.

Axis B: `axis_b_names_every_retired_line_number`,
`axis_b_reads_the_same_line_set_label_census_prints`.

Axis C: `axis_c_reds_on_the_schedule_a_cascade`,
`axis_c_catches_an_eligibility_change_on_a_line_that_also_moved`,
`axis_c_reds_on_schedule_3_nine_of_thirteen`, `axis_c_reds_on_schedule_1a_37_to_43`,
`axis_c_is_silent_on_a_rename_that_changed_no_printed_line`,
`a_reflowed_caption_is_not_a_reworded_line`, `the_draft_watermark_never_reaches_a_caption`,
`the_body_height_ceiling_is_derived_per_page_not_typed`,
`every_surviving_line_is_either_compared_or_named_as_a_caption_gap`,
`materially_changed_ignores_layout_and_never_ignores_a_word_or_a_number`,
`travel_reports_verbatim_reworded_and_gone_without_suppressing_the_collision`,
`the_rewording_threshold_separates_the_two_measured_shapes`,
`no_archived_pair_reports_a_clean_caption_verdict_from_zero_comparisons`.

Every corpus-walking test enumerates its pairs **from the filesystem** (`consecutive_pairs()`), never
from a typed list.
