# Fold — the interview T3 seam review (I1–I5, M1/M3/M5, N1; D16 recorded)

Single implementer, shared main tree, branch `main`, dispatched at `a2b1bb7f`. No subagents, nothing
committed, nothing pushed. Every plant below was made on the real tree and reverted from a `cp`
backup; `git checkout --`/`restore`/`stash` were never used. Brief:
`design/agent-reports/BRIEF-fold-interview-T3-review.md`.

**Gate at the end of the work:** all eleven test-bearing crates green (3,250 run / 3,250 passed / 12
skipped — see §Suite lines); `cargo fmt --all --check` clean;
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
clean; `census-join`, `stop-list` and `line-coverage` all return their committed lines.

---

## 1. I1(a) — a block that heads a LINE RANGE may not be `NoDollar` (structural)

`census_join.rs::verdict` rule **2b**. `NoDollar` means *this cell carries no dollar*; a range of the
form's own numbered lines is precisely the claim that it does, so the one edit that switches the
`Understates` rule and the REACH check off for a whole block of numbered lines at once is refused on
its face.

Verified against the committed tree before landing it (the ledger's own check reproduced): all six
`NoDollar` blocks are rangeless, every ranged block reads `Understates` or `Overstates`, and the sole
rangeless non-`NoDollar` block is Schedule 1's unnumbered 1099-K line, which is placed by `part`.

**Kill 1 — the reviewer's plant 1, on the real map** (`2024/f1040.map.toml`, the `Income` block,
`direction = "Understates"` → `"NoDollar"`), run while the `direction` key still existed:

```
$ cargo run -q -p xtask -- census-join
xtask census-join: the census join found 1 finding(s):

2024/f1040: [[direction]] "Income" is graded `NoDollar` and yet heads the form's numbered lines 1a–9
— a block that carries a LINE RANGE carries dollars by construction, so `NoDollar` there would
switch the `Understates` rule and the REACH check off for every line it heads.
```

Reverted → the committed success line returns. The rule now runs on the *derived* direction (item 2),
and the same plant expressed as a reading is in-suite case (16).

## 2. I1(b) — the direction VALUE leaves the maps

**What landed.**

- `DirectionBlock` **loses its `direction` field** (`btctax-forms/src/map.rs`). The struct carries
  `deny_unknown_fields`, so a map that states a direction of its own is now a **parse error**.
- `Direction` **leaves `btctax-forms` entirely** — the enum, its `Deserialize` derive and the
  `lib.rs` re-export are gone. It is redeclared in `xtask::census_join`, the checker. A map has no
  vocabulary for a direction, cannot name it, and cannot parse it.
- `direction = …` deleted from **all 28 `[[direction]]` blocks across the 11 tables**
  (machine-counted: `grep -c '^direction' crates/btctax-forms/forms/**` → 28 before, 0 after). The
  eleven per-map preamble comments that described the key were rewritten to state the new rule.
- `DIRECTION_OF_CAPTION: &[(&'static str, Direction, &'static str)]` — **22 readings**, one per
  distinct committed caption, each citing the form's own sentence (below).
- `verdict` now takes `DirectionTable { blocks, readings }` (one parameter, so the rule and its
  vocabulary travel together and the arity does not trip clippy). Rule **2a** grades each block by
  **exactly one** reading; a caption with no reading grades nothing and its entries fall through to
  the existing *"no `[[direction]]` block places it"* finding rather than being defaulted.
- `run()` walks both remaining directions: `reading_table_findings` (a reading no committed map
  carries; a caption read twice) and `evidence_findings` (an evidence sentence no in-scope extract
  prints).

**★ The evidence is itself asserted, not merely quoted.** Every one of the 22 sentences is checked to
be printed in one of the archived extracts — whitespace-normalised over the whole file, because the
forms wrap their own total sentences across two printed lines. Without that, the justification for a
sign would still be typed text; with it, the *reading* is what is committed and the sentence is a
reading of the form. (This is one assertion beyond what the brief specified. It is the same rule the
fold exists to enforce, applied one level up, and it costs nothing: all 22 passed first try.)

### `DIRECTION_OF_CAPTION`, as committed

| # | caption (verbatim) | direction | evidence — the form's own sentence |
|---|---|---|---|
| 1 | `For the year Jan. 1` | `NoDollar` | *"or other tax year beginning"* |
| 2 | `Foreign country name` | `NoDollar` | *"Foreign country name Foreign province/state/county Foreign postal code"* |
| 3 | `Filing Status` | `NoDollar` | *"If treating a nonresident alien or dual-status alien spouse as a U.S. resident for the entire tax year, check the box and enter their name"* |
| 4 | `Income` | `Understates` | *"Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income"* |
| 5 | `Tax and` | `Understates` | *"Add lines 22 and 23. This is your total tax"* |
| 6 | `Payments` | `Overstates` | *"Add lines 25d, 26, and 32. These are your total payments"* |
| 7 | `Refund` | `Overstates` | *"If line 33 is more than line 24, subtract line 24 from line 33. This is the amount you overpaid"* |
| 8 | `Amount` | `Understates` | *"Subtract line 33 from line 24. This is the amount you owe."* |
| 9 | `Third Party` | `NoDollar` | *"Do you want to allow another person to discuss this return with the IRS?"* |
| 10 | `Sign` | `NoDollar` | *"Under penalties of perjury, I declare that I have examined this return"* |
| 11 | `For 2024, enter the amount reported to you on Form(s) 1099-K` | `Overstates` | *"enter the amount reported to you on Form(s) 1099-K that was included in error or for personal items sold at a loss"* |
| 12 | `Part I       Additional Income` | `Understates` | *"Combine lines 1 through 7 and 9. This is your additional income. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 8"* |
| 13 | `Part II     Adjustments to Income` | `Overstates` | *"Add lines 11 through 23 and 25. These are your adjustments to income. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 10"* |
| 14 | `Part I       Tax` | `Understates` | *"Add lines 1z and 2. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 17"* |
| 15 | `Part II      Other Taxes` | `Understates` | *"Add lines 4, 7 through 16, 18, and 19. These are your total other taxes. Enter here and on Form 1040"* |
| 16 | `Part I       Nonrefundable Credits` | `Overstates` | *"Add lines 1 through 4, 5a, 5b, and 7. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 20"* |
| 17 | `Part II      Other Payments and Refundable Credits` | `Overstates` | *"Add lines 9 through 12 and 14. Enter here and on Form 1040, 1040-SR, or 1040-NR, line 31"* |
| 18 | `Itemized Deductions` | `Overstates` | *"Add the amounts in the far right column for lines 4 through 16"* |
| 19 | `Interest and Ordinary Dividends` | `Understates` | *"Subtract line 3 from line 2. Enter the result here and on Form 1040 or 1040-SR, line 2b"* |
| 20 | `Part I        Short-Term Capital Gains and Losses` | `Understates` | *"Net short-term capital gain or (loss). Combine lines 1a through 6 in column (h)"* |
| 21 | `Part II       Long-Term Capital Gains and Losses` | `Understates` | *"Net long-term capital gain or (loss). Combine lines 8a through 14 in column (h)"* |
| 22 | `Part III      Summary` | `Understates` | *"Combine lines 7 and 15 and enter the result"* |

Two readings differ from the brief's illustration because the **forms differ from the report**:
Schedule 1 line 26 reads *"Add lines 11 through 23 and 25"* (not *"…11 through 24 and 25"*) and
Schedule 3 line 8 reads *"Add lines 1 through 4, 5a, 5b, and 7"* (not *"…1 through 5 and 7"*). Both
were taken from `pdftotext -layout` output and both are asserted against it.

### Kills for item 2

**Kill 2 — a `direction` key planted back into a map** (`2024/f1040`, the `Income` block). Plants 2
and 3 of the review are unrepresentable: there is no key to edit.

```
$ cargo run -q -p xtask -- census-join
xtask census-join: 2024/f1040: TOML parse error at line 222, column 1
    |
222 | direction    = "NoDollar"
    | ^^^^^^^^^
unknown field `direction`, expected one of `caption`, `extract_line`, `first_line`, `last_line`
```

**Kill 3 — one reading deleted** (the `Income` row of `DIRECTION_OF_CAPTION`). 17 findings; the
orphaned caption is named first, and its 16 entries then red as unplaceable rather than being graded
by anything:

```
2024/f1040: [[direction]] "Income" has no reading in DIRECTION_OF_CAPTION — a block states no
direction of its own, so a caption with no reading grades nothing. Add the reading with the form's
own total sentence as evidence; it is never defaulted.

2024/f1040 topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].c1_22[0] (line "6c"): no [[direction]]
block places it — a heading may have been deleted or a revision re-parted. It is never defaulted to
Overstates.
```

…and the in-suite twin reds too:
`FAIL xtask::bin/xtask census_join::tests::every_committed_caption_has_exactly_one_reading_and_every_reading_is_used`.

**Kill 4 — a reading for a caption no map carries** (`"Part IV      Information on Your Vehicle"`):

```
DIRECTION_OF_CAPTION reads "Part IV      Information on Your Vehicle" as `Overstates`, but no
committed [[direction]] block carries that caption — a reading nothing uses is a judgement nobody can
watch being applied. Delete it, or add the block it was written for.
```

In-suite twin, same run:

```
thread 'census_join::tests::every_committed_caption_has_exactly_one_reading_and_every_reading_is_used'
panicked at crates/xtask/src/census_join.rs:1295:9:
["DIRECTION_OF_CAPTION reads \"Part IV      Information on Your Vehicle\" as `Overstates`, but no
committed [[direction]] block carries that caption — …"]
```

**In-suite plants added** (all inside `the_join_reds_on_every_planted_defect` and a new
`the_reading_table_reds_on_a_dead_reading_a_duplicate_and_an_unprintable_evidence`, each asserted on
its own finding text so removing the rule reds the test): (14) a caption with no reading, *and* the
assertion that its entries do **not** get graded by default; (15) two readings of one caption; (16)
the reviewer's `NoDollar`-over-a-range plant, expressed as a reading — *"is graded `NoDollar` and yet
heads the form's numbered lines 1–10"*; a dead reading; a duplicate reading; an evidence sentence no
extract prints (*"which no in-scope extract prints"*).

### K10–K13 and the D11 kills, re-run on the folded tree

| kill | plant | result |
|---|---|---|
| **K10** | `covered_by = "Advisory::EicOmitted"` on `2024/f1040s1` line 2a (Part I) | **1 finding** — *"an ADVISORY covers an `Understates` line ("Part I       Additional Income"). A blank there is a FALSE STATEMENT, not a forgone benefit…"* |
| **K11** | delete `Jury duty pay; ` from the attestation prompt | **1 finding** — *"OtherOutOfScopeIncome's prompt never says \"Jury duty pay\". A variant that exists is not a cover…"* |
| **K12** | delete the Part II heading from `2024/f1040s1` | **27 findings** (was 26) — all 26 Part II entries *"no [[direction]] block places it"*, **plus** the now-orphaned reading. Louder than before, and zero silently regraded |
| **K13** | caption drift, `…Adjustments to Income` → `…Incomes` | **29 findings** — the verbatim-extract red, the drifted caption having no reading, the real caption's reading orphaned, and 26 unplaceable entries |
| **K-D11a** | delete the `[[subtracts]]` block from `2024/f1040sb` | **1 finding** — the Schedule B line 3 advisory cover reds under `Understates` |
| **K-D11b** | `"Subtract line 3 from line 2"` → `"…from line 1"` | **2 findings** — the fake sentence *and* the cover it was propping up, together |

All reverted; the join returns its success line each time.

## 3. I2 / I3 — the rows invariant is two predicates

`document_census.rs`: `transcribed_rows` is replaced by **`declared_rows(ri, kind) -> Option<usize>`**
(a fact about the return — `Some(len)` for all five `Vec`-bearing kinds) and
**`requires_transcription(kind) -> bool`** (a demand on the filer). `DocumentDeclaredNotTranscribed`
now fires only where `requires_transcription`; the contradiction rule and `apply`'s `SetField(No)`
guard keep running on `declared_rows`, i.e. on all five.

Both sites are gated: `return_refuse::screen_document_census` **and**
`interview_state::census_row_invariant`, because a panel that kept the old rule would tell a truthful
1099-B filer their return is refusing while `screen_inputs` files it.

### The two predicates over the seven **supported** kinds (the other eleven are §2.2 families: `declared_rows` `None`, `requires_transcription` `false`, and `Some(true)` refuses with the family's exit before either is read)

| kind | `declared_rows` | `requires_transcription` | why |
|---|---|---|---|
| `W2` | `Some(w2s.len())` | **`true`** | Form 1040 line 1a is reachable only through `w2s` |
| `Int1099` | `Some(int_1099.len())` | **`true`** | line 2a/2b and Schedule B line 1, boxes 1–9 fully modelled |
| `Div1099` | `Some(div_1099.len())` | **`true`** | lines 3a/3b and Schedule B line 5, boxes 1a–13 fully modelled |
| `B1099` | `Some(b_1099.len())` | **`false`** | the form's own printed option — *"if you choose to report all these transactions on Form 8949, leave this line blank and go to line 1b"*. The `[[b_1099]]` row is the Schedule D line 1a/8a **summary option** only, and the crypto filer's 1099-B is the ledger, which prints per transaction on Form 8949 |
| `G1099` | `Some(g_1099.len())` | **`false` until T5** | `Form1099G` has no box 2; a prior-year state refund reaches Schedule 1 line 1 through `sch1.state_refund_taxable`. T5 adds `box2_state_refund` and its screen — named at the site |
| `Form1098` | `None` | `false` | scalar-shadowed, not live (T9) |
| `Form1098e` | `None` | `false` | scalar-shadowed, not live (T5) |

A committed invariant binds the two: **a row may only demand rows if it has somewhere to count them**
(`!requires_transcription(r) || declared_rows(r).is_some()`, over all eighteen).

`entry_route(B1099)` no longer names a route that double-counts — it names the form's own blank first,
and says entering a summary row beside ledger dispositions *"would count the same gains twice"*.
`entry_route(G1099)` names `sch1.state_refund_taxable` as where box 2 reaches the return today and T5
as the screen. Both are latent (neither row demands transcription), so both are **read by a test**
rather than left as prose nobody checks.

### Kills for item 3

**Probes P1/P2 committed** as `a_truthful_1099b_on_form_8949_and_a_box_2_only_1099g_both_file`; the
truth table as `the_rows_invariant_is_two_predicates_and_a_demand_needs_somewhere_to_count`; the route
text as `the_1099b_and_1099g_routes_name_the_blank_and_the_scalar_not_a_double_count`.

**Kill 5 — `requires_transcription(B1099) => true`** (one word). Three tests red:

```
the_rows_invariant_is_two_predicates_and_a_demand_needs_somewhere_to_count
  assertion `left == right` failed: ★ 1099-B is out because the FORM offers the blank …
    left: [W2, Int1099, Div1099, B1099]
   right: [W2, Int1099, Div1099]

a_truthful_1099b_on_form_8949_and_a_box_2_only_1099g_both_file
  assertion `left == right` failed: a filer who received a Form 1099-B and reports every transaction
  on Form 8949 answers YES truthfully and transcribes NOTHING …
    left: Some(DocumentDeclaredNotTranscribed { kind: B1099 })
   right: None

the_four_1099_rows_declare_like_the_w2_row_and_only_three_demand_rows
  B1099's refusal must say which task replaces that route with a screen: …
```

**Kill 6 — the panel's gate removed** (`census_row_invariant` loses `requires_transcription(row)`):

```
the_declared_document_and_rows_invariant_is_a_panel_item
  a Form 1099-B whose transactions are all on Form 8949 is a CORRECT return with zero summary rows —
  the panel must not refuse what the screen files: [Refusing { item: Question(DocB1099), …,
  reason: DocumentDeclaredNotTranscribed { kind: B1099 }, … }]
```

The refusals the split did **not** relax are asserted in the same test: a `No` beside a `[[b_1099]]`
row still contradicts, and a declared W-2 with zero rows still refuses.

`the_four_1099_rows_take_the_same_three_rules_as_the_w2_row` is renamed
`…declare_like_the_w2_row_and_only_three_demand_rows` and its rule (2) now branches on the predicate.

## 4. I4 — Schedule C line 6 is covered

**Attestation limb**, in the form's words, appended to limb (a):

> *"And it covers OTHER INCOME ON A SCHEDULE C, including a federal or state gasoline or fuel tax
> credit or refund (Form 4136) — Schedule C line 6, which btctax never asks about and leaves blank,
> so a blank there would UNDERSTATE your business income."*

`covered_by = "QuestionId::OtherOutOfScopeIncome", names = "fuel tax"` added to **both** years'
`line = "6"` entry, each with a comment saying why this one line is different from every other
Schedule C blank and naming the test that holds it.

**Narrow KAT** `census_join::tests::schedule_c_line_6_is_covered_and_the_attestation_reaches_it`
applies the join's own two `QuestionId` rules directly to those two entries — the keyword must be in
the entry's own `reason` (it is: *"…gasoline or **fuel tax** credit or refund…"*) and in the covering
prompt (REACH) — plus that the prompt names `Form 4136`. Without it the cover could rot silently,
because no walk reaches the map.

**Scope, stated:** Schedule C is outside the join's 13 maps, so this adds **nothing** to the 298 and
nothing to the cover split. Widening the join's scope is **FR-66**, not this fold.

## 5. I5 — the advisory's false clause

`Advisory::UnmodeledReturnOptionsOmitted` now carries **two clauses**. The reassurance is scoped —
*"None of **THOSE SEVEN** changes your tax; … mark the form by hand before signing"* — and the
§6013(g)/(h) election gets its own:

> *"ONE FURTHER CELL IS LEFT BLANK AND IT IS NOT ADMINISTRATIVE: the §6013(g)/(h) election to treat a
> NONRESIDENT-ALIEN SPOUSE as a U.S. resident… That election DOES change your tax — it is what makes
> a joint return available when one spouse is a nonresident alien, and it subjects that spouse's
> WORLDWIDE INCOME to U.S. tax for the entire year… So do NOT check that box by hand — checking it
> without adding that spouse's worldwide income would file an understated return under penalties of
> perjury. If it applies to you, this is a preparer's return."*

**Kill 7 — restore the old blanket sentence** (`THOSE SEVEN` → `these`):

```
the_return_options_advisory_scopes_its_reassurance_and_warns_off_the_nra_election
  the blanket reassurance covered the §6013(g)/(h) election, which DOES change the tax: RETURN
  OPTIONS NOT OFFERED — …
```

The same test also holds that all six remaining administrative cells are still named (so the split
cannot silently drop one), that the election's clause contradicts rather than inherits the
reassurance, that it says *why*, that the hand-check instruction is withdrawn, and that it names an
exit. `docs/examples/examples.md` regenerated (two occurrences; the whole diff is this advisory).

An NRA-spouse **gate** is FR-67 (T8); the false clause did not wait for it.

## 6. M1 / M3 / N1 / M5

**M1 — the three stale passages are gone.** The module header's *"the rows are NOT all countable"*
paragraph is replaced by the two-predicate statement; `exit_sentence`'s *"the four rows whose SCREEN
is task T5 carry a sentence…"* paragraph is deleted and its `None` set restated correctly;
`transcribed_rows`' doc is gone with the function. One further stale line the review did not list was
found while sweeping and fixed: the header table's *"the section is live; **zero transcribed rows
refuses**"*, which is now conditional on `requires_transcription`.

**M3 — recorded at the site.** A 14-line note above the dead sums in both years' `f1040s2.map.toml`
states plainly that the join **cannot express** "covered by the conjunction of these lines' covers",
and names exactly what would rot: if an addend's own cover (line 6's, any of 1a–1y's, any of
17a–17z's) is later weakened, the sum above it stays green on a refusal that never touches it. So
changing any addend's cover on that form means re-reading the sum. Same shape as D9c's dead-block
note.

**N1 — `census_tristate!`'s `clear` takes the liveness guard** `get` and `set` already had. Kill:
`clearing_a_non_live_census_row_is_refused_exactly_as_setting_it_is`, which drives both a live and a
non-live row through `SetField` **and** `ClearField`. Removing the guard:

```
apply::tests::clearing_a_non_live_census_row_is_refused_exactly_as_setting_it_is
  assertion `left == right` failed: ★ …and `clear` must refuse on the SAME predicate — un-answering a
  row nobody is being asked is still a write to it
    left: Ok(())
   right: Err(SetError(NoSuchRow))
```

**M5 — `live_questions` is document-first.** A **stable partition** inside `live_questions`: the live
census rows in registry order, then every other live declaration in registry order, then the
skippables. The `FORM_QUESTIONS` array is untouched and `decl_tristate!`/`census_tristate!`'s index
coupling is unchanged. A Single TY2024 filer now answers `DocW2` first instead of after seven gate
declarations and a 4,673-character attestation.

The test asserts the property as well as the vector: every live census row precedes every other live
declaration, **and** registry order still decides everything inside each group (so it is a partition,
never a sort).

### Lines that moved for M5, each with its cause

| where | move | cause |
|---|---|---|
| `answer.rs::a_single_filer_is_asked_the_always_live_declarations_and_no_spouse_question` | the sixteen `QuestionId::Doc*` entries moved from the end of the expected vector to its head | `live_questions` partitions census rows first; the eight gate declarations follow in unchanged registry order |
| same test | +19 lines asserting the partition property | so a future registry entry inserted mid-array cannot slip a gate in front of the shoebox |
| `tax_report.rs` keystroke script | `b"n\nn\nn\nn\nn\nn\nn\ny\nn…"` → `b"y\nn\nn…"` — the `y` moves from the 8th answer to the 1st | `DocW2` now leads. Same 23 mandatory answers and 7 bare Enters: the partition adds nothing. **Watched red first**: with the order moved and the `y` left in place, the run died on *"you answered that you received NO Form W-2, and this return carries 1 transcribed row(s)"* — `DocumentCensusContradicted`, the census working |

The no-brick test (`answering_every_blocking_item_empties_the_panel_and_refusing_is_empty_before_the_screen_passes`)
needed **no** move: it answers to a fixpoint through each item's own setter and is order-independent.
The `income answer` walkthrough goldens (`docs/examples-tui-walkthrough/j6/*`) likewise did not move —
they are byte-compared and stayed green.

## 7. D16 recorded

Appended to the build report's deviation table (`2026-09-07-build-interview-T3-implementation.md`),
not rewritten: `form_1098` / `form_1098e` unconditionally non-live until **T9** / **T5** replace the
scalars, ruled by the controller in `BRIEF-build-interview-T3.md`, with M2's residue noted
(`schedule_a.mortgage_interest_1098` is a bare `Usd`, so `$0` there is still indistinguishable from
"never asked").

---

## Pinned numbers moved

| where | old → new | cause |
|---|---|---|
| `OtherOutOfScopeIncome` prompt length | **4,425 → 4,673** chars (+248) | I4's Schedule C line 6 limb. Measured by parsing the literal out of `questions.rs` and applying Rust's `\`-newline rule, at HEAD and at the fold |
| `DirectionBlock` fields | 5 → **4** | `direction` deleted (I1(b)) |
| `direction = …` keys in the maps | 28 → **0** | ditto; machine-counted before and after |
| `DIRECTION_OF_CAPTION` | (new) **22 readings**, over 28 committed blocks / 22 distinct captions | both counts asserted in the KAT |
| `btctax-forms` public API | `{CensusDecision, Direction, DirectionBlock, SubtractSentence}` → `{CensusDecision, DirectionBlock, SubtractSentence}` | `Direction` moved into the checker |
| `census-join` success line | now also names the reading count | the line is asserted nowhere else in the repo (checked) |
| workspace test count | 3,242 → **3,250** (+8) | 3 new xtask tests, 4 new `btctax-core`, 1 new `btctax-input-form` |

## Pinned numbers that did **not** move (verified)

- `census-join`: **298 unmodeled entries across 13 maps** — unchanged.
- Cover split over the 13 in-scope maps: **Advisory 129 / QuestionId 122 / RefuseReason 47** —
  unchanged. Schedule C's two new `QuestionId::OtherOutOfScopeIncome` covers are **outside** that
  count (Schedule C is not one of the 13 maps), exactly as the brief anticipated.
- `line-coverage`: `341 money lines across 17 form(s) … 24 exception(s) (ratchet 24), 0 unverifiable
  (ratchet 0), 12 not line-bound (ratchet 12)` — unchanged.
- `stop-list`: `8 btctax-input-form sources, 4 state-bearing sources and 54 registry prompts scanned;
  no forbidden shape` — unchanged.
- `RefuseReason` / `Advisory` / `SetError` variant counts, `EXEMPT_LEAVES`, the coverage field and
  covered-leaf counts, the tui-edit section clamp, the `answer_log` record count: all unchanged (no
  variant, field or registry entry was added or removed by this fold).

## Constraints honoured

- `record_answer` remains the only writer of `answer_log` — untouched.
- No new prompt beyond the attestation limb and the advisory clause, both in the form's / the
  statute's words.
- `FORM_QUESTIONS` is unchanged: no entry added, removed or reordered, and the macro index literals
  are untouched.

---

## Suite lines per crate (final, after `cargo clean -p btctax-core -p btctax-adapters`)

```
btctax-core            Summary [ 0.551s] 1251 tests run: 1251 passed, 0 skipped
btctax-input-form      Summary [ 0.026s]   68 tests run:   68 passed, 0 skipped
btctax-forms           Summary [ 5.707s]  354 tests run:  354 passed, 4 skipped
btctax-cli             Summary [ 6.491s]  723 tests run:  723 passed, 1 skipped
xtask                  Summary [ 6.315s]  154 tests run:  154 passed, 1 skipped
btctax-tui-edit        Summary [ 2.028s]  382 tests run:  382 passed, 2 skipped
btctax-tui             Summary [ 1.589s]  160 tests run:  160 passed, 2 skipped
btctax-store           Summary [ 0.977s]   45 tests run:   45 passed, 0 skipped
btctax-adapters        Summary [ 0.028s]  103 tests run:  103 passed, 0 skipped
btctax-oracle-harness  Summary [ 3.177s]    5 tests run:    5 passed, 1 skipped
btctax-update-prices   Summary [ 0.003s]    5 tests run:    5 passed, 1 skipped
                                          ————
                                          3250 tests run: 3250 passed, 12 skipped
```

`crates/btctax` has no tests. `cargo fmt --all --check` clean ·
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
clean.

**★ One environment note, not a finding.** A `cargo nextest` batch was killed by a 2-minute tool
timeout mid-link and left `target/` with a stale `btctax-core` rlib; `btctax-adapters` then failed to
link with `ld.lld: error: undefined hidden symbol: anon.…llvm.…` referencing `compute.rs:0`.
`cargo clean -p btctax-core -p btctax-adapters` cleared it and all 103 adapter tests pass. Every
figure above was re-measured after that clean.

---

## Not done here, by design

- **FR-66** (widen the join's scope to Schedule C's 88 entries per year, Schedule 1-A, SE, 8949 …),
  **FR-67** (an NRA-spouse election gate, T8) and **FR-68** (the D11 flip's PAREN convention, T5) are
  the controller's follow-ups and were not touched.
- **M2** is recorded as D16 with no behaviour change, as briefed.
- **M4** and **D7** are unchanged and still open.
