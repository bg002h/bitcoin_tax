# REPORT — wave 2, parcel G: FR-210, the sub-letter witness

**Scope:** `crates/xtask/src/label_reader.rs`, `crates/xtask/src/form_delta.rs` (one test),
`design/TY2026_WORK_LIST.md` (the tool's own output + its prose), `design/forms/LABEL_READER.md`
(layout fact #4). **Not committed, not pushed.** No subagents. Every command foreground.

## Verdict

FR-210 is **fixed in the witness, and the refusal survives untouched.** Axis C still refuses every
label it cannot tell apart — `f8949`'s `1` and `2` were ambiguous before and are ambiguous now, and
a planted sub-letter the reader still cannot claim is still refused (B1 below). Nothing in the axis
was changed at all.

| gate | result |
|---|---|
| `cargo nextest run --workspace --no-fail-fast` | **3692 passed, 0 failed, 12 skipped** (baseline at HEAD: 3690 passed; +2 new tests) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **exit 0**, zero warnings |
| `cargo fmt --all --check` | **exit 0** |

★ `make check` runs nextest and clippy **concurrently** and was OOM-killed twice in this worktree
(`ld terminated with signal 9`, 24 GB already in use by the other parcels). It was run as the same
two commands sequentially with `CARGO_BUILD_JOBS=4`, after a forced `touch` of every `.rs` file —
i.e. `make gate`'s forced rebuild, split in two. Not a stale-rlib error (FR-176): SIGKILL on `ld`,
and it went away on fewer build jobs.

## The premises, checked

| brief said | verdict |
|---|---|
| `witness_text` misses margin sub-letters `a/e/g/h/k` on `f1040sa--2026-DRAFT` | **TRUE, and it misses more.** The dropped set is `5a 5e 8a 8b 8c 17g 17h 17k` — seven distinct letters `a b c e g h k`, not five. `8b` and `8c` were missing too. |
| it prints `17` three times and axis C refuses line 17 as ambiguous | **TRUE**, and it prints `5` twice for the same reason, which the brief did not name. Both were refused. |
| **9 of the 13** unread line numbers are this ambiguity | **TRUE, exactly.** Machine-checked: the 13 gaps are `f1040s1a` 3 + `f1040s2` 3 + `f1040s3` 1 + `f1040sa` 2 + `f1040sc` 1 + `f8949` 2 + `f8995` 1; the 9 ambiguity ones are s2's `1/13/17`, s3's `13`, sa's `17/5`, sc's `16`, and both of `f8949`'s. ★ But only **7 of the 9** are the sub-letter defect. `f8949`'s two are a *genuine* duplication — it prints `1` and `2` once per page on two pages — and no sub-letter rule can ever disambiguate them. They still read as gaps, and must. |
| Schedule A's `8` collisions is a FLOOR, not a count | **TRUE. It is 10.** |
| it re-bases three other checkers | **TRUE, and they are exactly three**, all on `design/TY2026_WORK_LIST.md`: `port_status_prints_the_committed_work_list`, `the_committed_work_list_matches_form_delta_at_head`, `the_work_list_checker_reds_on_every_planted_row`. Nothing else in the 3690 moved. |
| **REFUTED, in part:** *"the most consequential collision on that form, TY2025's line-18 checkbox versus TY2026's itemized total, currently prints as a GAP"* | **The pairing is wrong, and line 18 was never a gap.** Before the fix, line 18 already printed as a *hit*: TY2025's elect-to-itemize **checkbox** against TY2026's new itemized-deduction **limitation question**, with the checkbox's old text located travelling verbatim to **line 19**. The collision that was hidden is at line **17** — TY2025's *"Add the amounts in the far-right column for lines 4 through 16… enter on Form 1040 line 12e"* (the itemized **total**) against TY2026's *"Other itemized deductions"* heading. The brief's *conclusion* (the worst Schedule A collision was invisible) is right; its *identification* of which line was not. |

## Root cause

`candidate_columns` returns, per accepted x-cluster, the **maximum** `x2` in its bucket pair. A label
the extractor emitted as one suffixed token ends further right than a bare numeral, so that maximum
overshoots the column's actual right alignment. On `f1040sa--2026-DRAFT` every margin numeral ends at
**x2 = 108.00**, while `17i/17j` end at 110.32/110.50, `17f` at 111.00 and `17a`–`17e` at
113.17/113.50. The chosen candidate was **111.00**.

`column_tokens` then admitted a bare sub-letter on `w.x >= right - 2.0 && w.x <= right + 15.0`. Every
bare sub-letter on that form starts at **x = 108.00** — exactly the numerals' right edge — and
`111.00 - 2.0 = 109.00` is **1pt to the right of all of them**. Eight were dropped.

The band's other end was just as wrong in the other direction: `+15.0` reaches 15pt into prose. On
`f1040s1--2026-DRAFT` it swallowed the `a` of *"sold at **a** loss"* (offset +15.07), which has no
numeric parent above it, so `resolve` raised its orphan complaint and **`witness_text` returned a hard
error** — which is why Schedule 1's whole work-list row read `UNWITNESSED` in every page column.

## The fix

Three edits in `label_reader.rs`, all in the witness. The axis is untouched.

1. **`numeral_right_edge(parents)`** — the right edge the column's **unsuffixed** numerals agree on,
   as the **MODE**, not the maximum. Right-alignment means agreement, so the value they agree on is
   the column's edge and a stray prose numeral cannot outvote a column. (The maximum was tried: on
   `f1099b` a prose numeral inside the parent band ends at 415.12 where the real column ends at
   404.89, and the maximum re-based three fixtures for no reason. The mode changes nothing there.)
   `None` when a column holds no unsuffixed numeral; the caller then falls back to the candidate.
2. **`SUB_LETTER_SLACK = 2.0`, symmetric**, replacing `[-2.0, +15.0]`. Measured, not chosen: of the
   **633** bare letters that fall inside ±2.0 across all 87 fixtures, **624** sit at offset 0.00 (623)
   or +0.09 (Schedule 1-A's `a`) — because a bare sub-letter is the *tail of its parent's own printed
   word*, split by the extractor at a zero-width gap. The nearest bare letter that is not a label is
   prose at **+0.74**; everything from **+3.55** up is prose without exception. 2.0 is also the
   tolerance the parent band already allows for column wobble, so it is not a new constant.
3. **The row merge is order-insensitive.** `f1040sc--2026-DRAFT` emits line 16c's bare `c` at
   y=559.41 and its parent `16` at y=559.91 — one printed row, **letter first** — so the merge never
   fired, Schedule C printed `16` twice, and axis C refused line 16. It now merges in either order,
   keeping the sub-letter's coordinates (the larger `x2`, which is what `locator_words` matches back
   to keep the token out of captions).

Also: the sort is now fed parents-then-letters, so a parent and its same-row letter always arrive in
merge order rather than depending on the extractor's emission order.

## Blast radius — measured over all 87 committed geometry fixtures

Method: dump `label-census` for every fixture at HEAD, then at the fix, and diff the two dumps.
(HEAD's reader was restored into the file from object storage, built, dumped, then restored from a
plain file copy — no branch or checkout operation anywhere in this worktree.) **12 fixtures changed;
not one lost a printed label.**

| fixture | change |
|---|---|
| `f1040s1--2026-DRAFT` | **hard error → 64 labels** |
| `f1040sa--2026-DRAFT` | 36 → 41 rows; gained `5a 5e 8a 8b 8c 17g 17h 17k`; ambiguities `5`, `17` gone |
| `f1040s2--2026-DRAFT` | 49 → 54; gained `1b 1c 1e 1f 13d 13g 13h 13i 13o 17b 17c 19a`, lost `19` (a misread `19a` — TY2026 Schedule 2 prints no bare 19); ambiguities `1 13 17` gone |
| `f1040s3--2026-DRAFT` | gained `13b 13e`; ambiguity `13` gone |
| `f1040sc--2026-DRAFT` | 44 → 43; ambiguity `16` gone (the order-insensitive merge) |
| `f6251--2026-DRAFT` | `2` → `2a` (the draft prints no bare line 2), retiring a **spurious 1-retired / 1-introduced pair** on the f6251 row |
| `f1040s1--2022` | 64 → 63; the duplicate phantom `8a` from *"**a** nongovernmental…"* is gone |
| `fw2--2024`, `fw2--2025` | lost phantoms `14a` and `62a` (from *"If **a** year follows code D"*, and *"received **a** distribution"* under *"age **62**"*) |
| `fw2--2026` | lost a duplicate phantom `3a` |
| `f4868--2024`, `f4868--2025` | its phantom `9a` now comes from a different prose `a` (see the stated boundary) |
| `f1040v--2024/2025` | unchanged: still no numbered column, still a refusal |

**Document justification for the regeneration** (a golden cannot validate its own regeneration):
every newly-read label — 64 + 12 + 2 + 8 + 1 = 87 across those five drafts — was looked up in its own
`design/forms/extract/<stem>.txt` and **found printed there, with its extract line number**, before
the work list was touched. Spot-checked by hand where the automated match could have been
coincidental (`f1040s1`'s `4`, `24`, `26` → extract lines 57, 127, 148). The two "lost" labels were
checked the other way: the TY2026 Schedule 2 extract contains **no** bare line 19 (only 19a/19b/19c),
and the TY2026 Form 6251 extract contains no bare line 2 (it opens at `2a`, as TY2025's does).

## What the port table now says

`cargo run -p xtask -- port-status 2025 2026-DRAFT`, totalled by script, never by hand:

| column | before | after |
|---|---|---|
| lines that moved | 123 | 113 |
| **boxes UNREAD** | **137** | **67** |
| lines retired | 34 | 22 |
| **lines introduced** | 65 | **74** |
| **line numbers whose MEANING changed** | 118 | **126** |
| **line numbers UNREAD** | **13** | **6** |

The 6 that remain: `f1040s1a`'s `4a/4b/4c` and `f8995`'s `1i` (empty captions on the new side, not
this defect), and **`f8949`'s `1`/`2` — the genuine duplication that must stay refused.**

★★ **The eight new collisions are not bookkeeping.** Three of them matter:

- **`f1040s1` line 14** — *"moving expenses for members of the armed forces"* becomes *"…armed forces
  **and the intelligence community**"*. An eligibility widening on a deduction, on a form whose row
  previously read UNWITNESSED in every page column — so **no axis could see it at all.**
- **`f1040sa` line 5e** — the SALT cap moves **$40,000 → $40,400** (**$20,000 → $20,200** MFS) and its
  phase-out start **$500,000 → $505,000** (**$250,000 → $252,500**). Read off both extracts
  (`f1040sa--2025.txt:29-30`, `f1040sa--2026-DRAFT.txt:74-77`). A figure change inside a caption is
  precisely what axis C exists for, and it was inside the ambiguity.
- **`f1040sa` line 17** — the itemized **total** becomes *"Other itemized deductions"*: the real hidden
  collision (see the refutation above).
- (`f1040s1` lines 7 and 10 are a year token and a footer artefact — real text changes, low value.)

## B1 — the kill the brief asked for

`label_reader::tests::a_sub_letter_the_reader_cannot_claim_is_still_refused_by_the_caption_axis`
builds one synthetic revision pair twice, differing **in a single coordinate**: the bare sub-letter
`d` sits either at the numerals' agreed edge, or **8pt out**, which is prose distance. The two
revisions differ only in that line's caption.

| | letter AT the edge | letter 8pt out (the plant) |
|---|---|---|
| `witness_text` | `1 2 2c 2d 3 4 5a` | `1 2 2c 2 3 4 5a` — the parent printed twice |
| `caption_join` | nothing ambiguous | `2` **ambiguous** |
| `caption_axis` | **1 collision, at `2d`**, compared 7 | **0 collisions, a named GAP at `2` (`AmbiguousOld`)**, compared 5 |

The right-hand column is the axis declining to say anything about a line whose meaning **really did
change** — *"a skipped field is not a passed one"* — and it is asserted as `collisions.is_empty()`
with the message *"picking one of two candidates is worse than the gap it closes"*. Had the fix been
made in the axis, that is the assertion that reds.

The fixture carries `5a` as a single suffixed token ending 2.5pt right of the numerals, in the
neighbouring 2pt bucket, so `candidate_columns` reports 52.90 for a column aligned at 50.40 — FR-210's
mechanism in miniature. **Seen RED, measured:** planting `let edge = right;` returns `1 2 3 4 5a` —
both claimable sub-letters gone — and the test fails on its first assertion.

Two further kills:

- `a_sub_letter_emitted_above_its_parent_still_merges_into_one_label` — on the **real**
  `f1040sc--2026-DRAFT` fixture, asserts the interest block reads exactly `16 16a 16b 16c` and that
  `16` is no longer ambiguous. **Seen RED:** neutralising the letter-first merge arm to `if false`
  returns `["16","16a","16b","16c","16"]`.
- The same test asserts, on the committed `f8949--2025` fixture, that `caption_join`'s ambiguous set is
  **exactly `["1","2"]`** — the in-tree proof that the refusal survived a reader fix.

## The re-based checkers — and the one that needed more than a paste

All three green, named:

```
PASS form_delta::tests::the_committed_work_list_matches_form_delta_at_head
PASS form_delta::tests::port_status_prints_the_committed_work_list
PASS form_delta::tests::the_work_list_checker_reds_on_every_planted_row
PASS schedule_1a_membership::tests::every_printed_label_is_a_field_or_a_recorded_heading
PASS schedule_1a_membership::tests::a_dropped_line_an_invented_line_and_an_unrecorded_heading_are_all_rejected
```

The first two were a paste of the printer's own output into `design/TY2026_WORK_LIST.md` (six rows
moved). **The third was a real finding.** It had hardcoded `f1040s1` as its exemplar of *"a pair the
reader cannot witness"* and planted `lines that moved → 0` against it. Fixing the reader turned that
plant into a **no-op** — and it surfaced only because `plant` asserts that it changes the cell it
plants (`assertion left != right failed: … already reads "0"`). That guard is the only reason this did
not become a silently dead plant, which is the exact shape `design/HARNESS.md` class β is about.

Per this repo's own highest-yield rule, the exemplar is now **derived, not typed**: the test searches
the printer's own output for a row whose `lines that moved` cell begins `**UNWITNESSED**`, over a
stated tag pair (`f1040v--2024 → --2025`; a payment voucher prints no numbered column at all, so
`witness_text` refuses it outright — the only shape that makes `label_compared == 0` on a non-empty
pair). **Its absence is a hard failure that says what to do about it**, never a skip. Seen RED:
pointing the tags at `("2025","2026-DRAFT")` — the pair that no longer has one — panics with *"no stem
on the emitting surface has an UNWITNESSABLE label axis at … any more, so the two plants below cannot
fire … do not delete the plants."*

## Documents touched, and why

- **`design/TY2026_WORK_LIST.md`** — the numeric table is this tool's output and a test pins it, so it
  had to be regenerated. Its **prose** cited four numbers the fix retracted (`65` introduced, `137` and
  `13` unread, and *"Schedule 1's 1 is the only change of any kind its row can report"*) plus the
  line-18 pairing. History is not rewritten: each superseded claim is marked in place with the new
  figure, and a new dated section records the mechanism, the before/after table, the collisions and the
  document justification. That is FR-212's lesson applied to the document FR-212 is about.
- **`design/forms/LABEL_READER.md`** — this file numbers the reader's discovered layout facts 1, 2, 3.
  FR-210 is a fourth of the identical shape (fact 2 biting from the other side, through fact 3's bucket
  maximum), so it is recorded as fact **4**, with its measurements and its stated boundary.

Neither file is claimed by parcels E, F or H.

## Residue, for the controller to file (`FOLLOWUPS.md` untouched — three agents are live in it)

1. **FR-210 can be closed.** FR-211's gap-count column already exists and is now informative
   (`f8949` reads `UNWITNESSED | 2`; every other unread count dropped).
2. **Three prose bare letters still fall inside ±2.0** — `f1099b` p5 (+0.74), `fw2--2026` p7 (+1.71),
   `f4868` p3 (+1.90) — and `f4868`'s phantom `9a` is the one label they produce. All three forms are
   outside the emitting surface. Tightening the upper bound to +0.5 removes them; it was deliberately
   **not** done, because the measured gap below them (0.09 → 0.74) is a tenth the width of the one
   above (0.09 → 3.55), and dropping a real sub-letter is the defect this parcel was about. Stated in
   the source rather than hidden. **Minor.**
3. **Form 8995 line 1's roman row markers are read asymmetrically.** `i` is a single character and is
   claimed (so `1i` reads, and it is one of the 6 remaining unread line numbers, as an *empty
   caption*); `ii`, `iii`, `iv`, `v` are multi-character and can never be claimed by any sub-letter
   rule. That is FR-27's row model, not this band. **Minor.**
4. **`f8949` is still `UNWITNESSED | 2`, and no reader can fix it.** Its label `1` genuinely appears on
   two pages. Telling those apart needs a per-page `CaptionSet` (or a page-qualified label) — a change
   to axis C's *key*, not to the witness — and it is the one change that would let the caption axis say
   anything at all about the form btctax emits the most rows on. **Important, but out of this parcel's
   scope by design:** the brief forbade making the axis guess, and this is the non-guessing way.
5. `locator_words` matches a merged token back to a word by `(x2, y)`, so only the sub-letter half of a
   merged label is registered as a locator; the parent numeral is kept out of captions only because its
   `x2` fails the `> primary_right` filter. Harmless on all 87 fixtures, but two facts are holding one
   invariant. **Nit.**
