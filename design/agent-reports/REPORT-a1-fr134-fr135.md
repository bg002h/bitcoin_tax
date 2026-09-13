# REPORT — A1: FR-134 (the doc-comment gate is not year-generic) and FR-135 (the quoting year is a literal)

**Agent:** A1, opus, isolated worktree `376d1141`, `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-a1`
(clippy in `…/target-a1-clippy`). **Nothing committed, nothing pushed, nothing stashed.** Two files changed,
both owned; the tree is otherwise identical to `376d1141`.
**Date:** 2026-09-12.

## 0. The headline, in five numbers

| question | answer |
|---|---|
| **Is the caption gate now derived from the maps on disk?** | **Yes.** `bundled_maps()` globs `crates/btctax-forms/forms/*/*.map.toml`, reads each map's OWN `irs_stem` and `year`, and opens `design/forms/extract/<irs_stem>--<year>.txt`. No year is named anywhere in the checker. A new year is covered because it has a map, not because someone edited a list. |
| **What it measures on the committed tree** | **429 captions across all 38 maps** — every one printed on its own year's extract, **2 pinned** as known paraphrases (below). The run's own words: `429 map caption(s) across 38 bundled map(s) checked against their OWN year's extract (floor 429), all printed there but 2 pinned as known paraphrases`. |
| **The kill** | The rehearsal's exact plant — TY2024's SALT figures in the **TY2025** Schedule A map — **reds 4 tests** with this change and **0 without it.** Measured both ways, same worktree, same command. |
| **FR-135's live surface** | **12** bundled `(form, year)` pairs are quoted from another year's booklet, enumerated by the run, ratcheted at 12. Two bundled 2025 pairs are correctly NOT in the list. |
| **Cost** | `line_coverage_check.rs` +994, `line_coverage.rs` +13 (doc only). 7 new tests (xtask 186 → 193). `cargo clippy -p xtask --all-targets -- -D warnings` clean; `cargo fmt --all --check` clean. |

**And the result that was not in the brief:** building the gate found **two real transcription defects** and
**two caption conventions no checker had ever read**. Both in section 4.

---

## 1. What changed, where, and why that mechanism

Everything is in the two files A1 owns. **No product behaviour changed** — `line_coverage_check.rs` is a
dev-only xtask module (`publish = false`), and the `line_coverage.rs` edit is a doc comment.

### `crates/xtask/src/line_coverage_check.rs`

| `file:line` | what | why this mechanism |
|---|---|---|
| `:1845` | **rule (4d)**, inside `check()` | `check()` is what `run()` calls and what `tests::the_committed_coverage_table_is_consistent_with_the_form_text` drives, so the gate is inside `make check` (= `cargo nextest run --workspace`) by construction rather than by a new wiring anyone could forget. It sits beside `(4b)`/`(4c)`, which already read the repo rather than the table. |
| `:1371` `bundled_maps` | the glob walk | Reads `year` and `irs_stem` **as TOML**, not by a line scan, because those two fields decide *which extract is the authority*. Refuses a map whose declared `year` disagrees with its folder — the extract path is built from the declared year, so a disagreement silently checks the wrong booklet. Refuses an empty walk (section 2.4). |
| `:1205` `map_captions` | the caption parser, one rule over four shapes | A caption is *a comment chunk whose text before the opening quote is nothing but markers and at most one line label*. That covers the four shapes in section 4.1, where reading only the first would have reported success over 17 unopened maps. |
| `:1129` `is_caption_label` | at most 2 digits | **Found by measurement, not foresight.** With any digit run allowed, `2024` reads as a line label, and Schedule A's TY2025 header — which quotes the TY2024 sentence verbatim *to document the SALT change* — parses as a TY2025 caption and reds. No form these maps cover prints a line above 40. |
| `:1148` `comment_start` | first `#` outside a quoted string | `f8275.map.toml` has six FQNs containing `.#subform[0].`; a naive scan slices a field name in half and invents a comment. |
| `:1109` `caption_normalize` | whitespace + brace glyphs + **quote glyphs** + **leader dots** | Quote glyphs collapse because a caption is delimited by a double quote and therefore *cannot* carry the form's own curly-quoted "Yes," (Schedule B line 8); the map has to write 'Yes,'. A lone `.` is a dot leader, never language — sentence punctuation attaches to its word. ★ Deliberately **not** folded into `normalize()`, which is held byte-identical with `f6251_map.rs::norm` on purpose and lives in a file A1 does not own. |
| `:1254` `destub` / `:1265` `strip_left_cell` | drop each line's left marginal cell | `pdftotext -layout` interleaves Schedule A's stub column into a wrapped sentence: line 2 comes out as *"Enter amount from Form 1040 or 1040-SR, / **Expenses** line 11b"*. This is a mechanism, not an exemption, and it is load-bearing: **without it the scan reports 8 problems instead of 2, six of them false** (section 8). |
| `:1292` `rejoin_column_header` | zip a 2-row header by column | Form 8995-A's Part I captions are nowhere contiguous in the text layer. Load-bearing: **without it, 6 problems instead of 2** (4 false). The same mechanism `f8995a_map.rs` already uses — carried here so it is not per-form. |
| `:1337` `caption_present` | ellipsis fragments **in order** | An ellipsis is the author's own declared gap (`f8959` line 24 writes "Total ... Add lines 22 and 23"). A caption with no ellipsis gets no such licence, and the test pins that no normalisation turns `line 12` into `line 22`. |
| `:1078` `CAPTION_PARAPHRASES` | the 2 real defects, pinned to their exact wrong text | Section 4.2. Self-retiring: `stale_paraphrase_claims` (`:1473`) reds the day a row stops matching. |
| `:1065` `MIN_MAP_CAPTIONS = 429` | the floor | Every caption rule is per-caption, so a caption that is **deleted** is invisible to all of them — swapping the quote glyph is the cheapest way to silence this gate. The floor is what notices, and `tests::stripping_a_maps_captions_takes_the_scan_below_its_floor` is the mutation it notices. |
| `:1504` `unquoted_bundled_years` + `:1560` `MAX_UNQUOTED_BUNDLED_YEARS = 12` | **FR-135** | Section 5. |

### `crates/btctax-core/src/tax/line_coverage.rs`

`:314` — thirteen doc-comment lines on `Coverage::quoting`, where the 26 literals live and where the next
author will be standing. It records the 26-vs-2 measurement and says that forgetting to re-point the literal
is now caught by a ratchet in the checker, naming it. Nothing executable changed in this crate.

---

## 2. The kill — red, then green, pasted

The plant is the rehearsal's, on a real map: **`crates/btctax-forms/forms/2025/f1040sa.map.toml` line 121,
line 5e's caption reverted from TY2025's `$40,000 ($20,000 …)` to TY2024's `$10,000 ($5,000 …)`** — the
section 164(b) SALT change this map's own header documents as item 2. Applied with `sed`, restored from a
byte-copy backup; the working tree afterwards listed only the two owned files as modified.

```
-line5e = "form1[0].Page1[0].f1_11[0]"    # ★★★ "Enter the smaller of line 5d or $40,000 ($20,000 if married filing
+line5e = "form1[0].Page1[0].f1_11[0]"    # ★★★ "Enter the smaller of line 5d or $10,000 ($5,000 if married filing
```

### 2.1 The plant with the checker AT HEAD — nothing looks

`line_coverage_check.rs` restored from a byte copy of `376d1141`'s version, plant in place:

```
cargo nextest run -p xtask --no-fail-fast
     Summary [  34.470s] 186 tests run: 179 passed, 7 failed, 1 skipped
        FAIL (100/186) form_delta::tests::the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals
        FAIL (112/186) form_delta::tests::every_common_field_is_either_compared_or_named_as_unwitnessed
        FAIL (113/186) form_delta::tests::no_archived_pair_reports_a_clean_verdict_from_zero_comparisons
        FAIL (119/186) form_delta::tests::the_work_list_checker_reds_on_every_planted_row
        FAIL (167/186) form_delta::tests::port_status_prints_the_committed_work_list
        FAIL (172/186) form_delta::tests::the_committed_work_list_matches_form_delta_at_head
        FAIL (186/186) harness_check::tests::the_write_hook_denies_new_archives_and_asks_once_per_new_directory
```

**The identical seven fail with the plant REMOVED and the same HEAD checker** (`179 passed, 7 failed,
1 skipped`, same names). So the plant reds exactly nothing — F2 reproduced inside this worktree. All seven are
environmental and their mechanism is in the message, not in any code I touched:

* 6 x `form_delta` — *"the TY2026 draft is archived and its geometry extracted: no PDF found for
  f6251--2026-DRAFT"* (`form_delta.rs:1075`). `design/forms/*.pdf` is excluded from version control and absent
  from an isolated worktree; the rehearsal recorded the same thing.
* 1 x `harness_check` — *"cargo build -p xtask succeeded but left no binary where on-write.sh looks — if the
  target dir moved, the HOOK's lookup needs updating too"* (`harness_check.rs:665`). That is **FR-147/F13**,
  the mandated `CARGO_TARGET_DIR` override, verbatim.

### 2.2 The plant with this change — RED

```
cargo run -q -p xtask -- line-coverage        # exits 1
xtask line-coverage: line-coverage FAILED (1 problem(s)):
  - crates/btctax-forms/forms/2025/f1040sa.map.toml: the caption on line5e is NOT printed on f1040sa--2025.txt:
      "Enter the smaller of line 5d or $10,000 ($5,000 if married filing separately). If Form 1040 or 1040-SR, line 11b is more than $500,000 ($250,000 if married filing separately), or if you completed Form 2555, Form 4563, or excluded income from Puerto Rico, see instructions"
    This is the Form 6251 line-33 class — a sentence carried forward from a document nobody re-read. Either the caption is wrong, or it was transcribed from another year's form.
```

…and it is in the **suite**, not only the CLI:

```
cargo nextest run -p xtask --no-fail-fast
     Summary [  28.196s] 193 tests run: 182 passed, 11 failed, 1 skipped
        FAIL (133/193) line_coverage_check::tests::a_caption_quoting_another_years_figures_reds_and_the_committed_map_passes
        FAIL (162/193) line_coverage_check::tests::each_rule_rejects_a_table_that_violates_it
        FAIL (168/193) line_coverage_check::tests::the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds
        FAIL (178/193) line_coverage_check::tests::the_committed_coverage_table_is_consistent_with_the_form_text
        … plus the same 7 environmental
```

11 = the 7 environmental + **4 caused by the plant**. The three besides the dedicated kill red because they
assert `check(…)` is clean on a control table, and rule (4d) is repo-wide — the same way `(4b)`'s type scan
already behaves. **The plant restored, the run returns to the 7-failure baseline** (`186 passed, 7 failed,
1 skipped`), and the whole workspace is:

```
cargo nextest run --workspace --no-fail-fast
     Summary [  30.422s] 3621 tests run: 3614 passed, 7 failed, 12 skipped     (the same 7)

cargo run -q -p xtask -- line-coverage
line-coverage OK: 377 money lines across 18 form(s) [...], 31 exception(s) (ratchet 31), 0 unverifiable
(ratchet 0), 17 not line-bound (ratchet 17); 429 map caption(s) across 38 bundled map(s) checked against
their OWN year's extract (floor 429), all printed there but 2 pinned as known paraphrases; 12 bundled
year(s) quoted from another (ratchet 12)
```

### 2.3 "Which test reds when this checker is removed?" — three mutations, measured

B1's reviewable question, answered by name rather than by promise. Each mutation applied to the finished
change, the filtered binary run, then restored from a byte-copy backup.

| mutation | tests red |
|---|---|
| `caption_present` made unconditionally true | **5** — `each_layout_normalisation_is_load_bearing`, `a_caption_quoting_another_years_figures_reds_and_the_committed_map_passes`, `each_rule_rejects_a_table_that_violates_it`, `the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds`, `the_committed_coverage_table_is_consistent_with_the_form_text` |
| `map_captions` made to return nothing | **7** — the five above minus the layout one, plus `every_committed_caption_shape_is_read_and_prose_is_not`, `stripping_a_maps_captions_takes_the_scan_below_its_floor`, `a_paraphrase_row_that_no_longer_matches_anything_is_reported_stale` |
| `MAX_UNQUOTED_BUNDLED_YEARS` 12 to 11 | the CLI reds, naming all 12 pairs (pasted in section 5) |

★ A second-order property worth recording: **neutralising the checker makes `CAPTION_PARAPHRASES` go stale,
and the staleness rule reports it.** Mutation 1's failure text is *"CAPTION_PARAPHRASES still claims
…f1040sa.map.toml line line5e paraphrases the form (…), but the scan did not find that caption failing."*
A vacuous checker cannot keep the exemption list quiet, which is the opposite of the usual relationship
between the two.

### 2.4 The vacuity guards, each observed

`tests::the_map_walk_refuses_to_walk_nothing_and_refuses_a_misfiled_year` (`:2572`) builds real trees in a
`tempfile::tempdir()` and observes three refusals plus one acceptance:

1. no `forms/` directory at all, which cannot be read;
2. `forms/2031/` with no maps — *"no *.map.toml found under … — the caption gate would pass by walking nothing"*;
3. a map declaring `year = 2030` inside `forms/2031/` — *"declares year = 2030 but is bundled under forms/2031/"*;
4. the same tree with the year corrected is **accepted** — so (3) is about the disagreement, not the fixture.

An **unreadable extract is an error, not a skip** (`:1856`): *"bundled for {year} but {path} cannot be read —
its captions are unverifiable, and a caption nobody can check is exactly what a port carries forward."*
And an **empty extract** reds every caption rather than finding nothing: direction (4) of the headline kill.

---

## 3. Is it derived, or does it still declare a year?

**Derived.** No tax year is written anywhere in the new code. The chain is
`forms/<year>/<stem>.map.toml` to that map's own `irs_stem` + `year` to
`design/forms/extract/<irs_stem>--<year>.txt`, and the year-folder set comes from `read_dir`.
What forces a new year to be covered:

1. **Existence.** A new map is walked the moment it lands. There is no registry to add it to and no
   `include_str!` to point at it.
2. **The map's own extract, or a loud failure.** If `design/forms/extract/<irs_stem>--<year>.txt` is missing,
   that is an error naming the map — `xtask line-coverage` does not fall back to another year, which is the
   substantive difference from the pinned gate.
3. **A directory/declaration disagreement refuses.** The path is built from the *declared* year, so a map
   filed in the wrong folder cannot quietly authenticate against the wrong booklet.
4. **The floor.** 429 only goes up as maps arrive; deleting captions to quiet the gate reds.
5. **FR-135's ratchet.** Bundling a form for a new year whose collector still says the old one raises the
   count from 12 and reds.

**What still declares a year: `Coverage::quoting("2024")`, 26 times, in `line_coverage.rs`.** That is FR-135's
real subject and it is **not** removed — see section 5.

---

## 4. Three things the gate found on its first run

### 4.1 A third and fourth caption convention, with no checker between them

The two committed per-form gates (`f8995a_map.rs`, `f6251_map.rs`) parse a bare-number caption above the key.
Measured over all 38 maps with that shape alone: **134 captions, in 4 maps.** With all four shapes:
**429 captions, in 21 maps** (17 maps deliberately carry none — `f1040.map.toml` annotates lines as
`# sum of W-2 box 1`, which makes no quotation claim and so has none to check). The shapes in the tree:

| shape | example | where |
|---|---|---|
| bare number, above the key | `# 27 "Total qualified business income component…"` | `f6251` x2, `f8995a`, `f1040s2/2025` |
| trailing the key, unlabelled | `line1 = "…"   # "Medical and dental expenses (see instructions)"` | the majority — Schedules A/B/C/1/1-A/2/3/D, `f8889`, `f8959/2024`, `f8960`, `f8995` |
| **L-prefixed, above the key** | `# L5 — "Enter the following amount for your filing status: …"` | `f8959/2025`, `f4868` x2 |
| **L-prefixed, trailing the key** | `line1 = "…"   # L1 — "Foreign tax credit. Attach Form 1116 if required"` | `f1040s3/2025`, `f1040sb/2025`, `f1040/2024` |

Both bolded shapes were invisible to my first parser, and `f8959` is the sharp case: **the 2024 map writes 17
trailing captions and the 2025 map writes the same 17 in the L-prefixed shape.** A checker reading one shape
would report the 2024 map and say nothing at all about the 2025 one. The two L-prefixed shapes are
**41 of the 429**. `tests::every_committed_caption_shape_is_read_and_prose_is_not` (`:2490`) pins all four on a
synthetic map, pins that a year-prefixed quote and a "Form text:" prose quote are **not** captions, and
asserts against the real tree that the bare-number shape is a minority — as an inequality, so it cannot rot
into a stale number.

### 4.2 Two real transcription defects — **in files A1 does not own**

Both are one-phrase comment edits in a `.map.toml`. The partition assigns those maps to nobody, so per the
brief I stopped rather than editing across the boundary, and pinned them at `line_coverage_check.rs:1078` so
the gate is enforceable today. **Both are Minor** (an abbreviation and a dropped parenthetical; neither changes
a figure, a line number or a cross-reference), and both are the "Transcribe IRS forms" rule being bent:

| map | line | the map says | the form prints |
|---|---|---|---|
| `crates/btctax-forms/forms/2024/f1040sa.map.toml` | `line5e` | "…or $10,000 ($5,000 **MFS**)" | "…or $10,000 ($5,000 **if married filing separately**" (`f1040sa--2024.txt:28`) |
| `crates/btctax-forms/forms/2024/f8959.map.toml` | `line8` | "Self-employment income from **Schedule SE**, Part I, line 6" | "Self-employment income from **Schedule SE (Form 1040)**, Part I, line 6." (`f8959--2024.txt:31`) |

★ The TY2025 successors of both are already correct — `f8959/2025` L8 writes "Schedule SE (Form 1040)" — so
these are two stale TY2024 captions, not a house style. The fix is to replace the caption text with the form's
own and delete the two `CAPTION_PARAPHRASES` rows in the same diff; `stale_paraphrase_claims` reds if only one
half is done. **Recommendation to the coordinator: do it at integration, in the fold commit, with the
`xtask line-coverage` output in the message.** Nothing blocks on it.

### 4.3 `f8959/2025` L5 and L9 are a three-row caption, and the dot leaders are why a naive gate would red

*"Enter the following amount for your filing status: Married filing jointly $250,000 …"* is three printed rows
joined by dot leaders in the text layer. Handling it is what forced the leader-dot rule; nothing is wrong with
the map. Recorded because it is the shape a future form with a bracketed filing-status block will hit again.

---

## 5. FR-135 — what is closed and what is not

`unquoted_bundled_years` (`:1504`) derives every bundled `(irs_stem, year)` from the maps, and reports those
the coverage table covers **only at some other year**. Run with the ratchet one lower, to show it live:

```
xtask line-coverage: line-coverage FAILED (1 problem(s)):
  - 12 bundled (form, year) pair(s) are covered by this table at a DIFFERENT year only, so their quotes are
    verified against another booklet (ratchet 11): f1040--2025 (quoted: 2024), f1040s2--2025 (quoted: 2024),
    f1040s3--2025 (quoted: 2024), f1040sa--2025 (quoted: 2024), f1040sb--2025 (quoted: 2024),
    f1040sc--2025 (quoted: 2024), f8889--2025 (quoted: 2024), f8949--2025 (quoted: 2024),
    f8959--2025 (quoted: 2024), f8960--2025 (quoted: 2024), f8995--2025 (quoted: 2024),
    f1040sse--2025 (quoted: 2024). Coverage::quoting("…") is a literal in each collector; bundling a year
    does not move it.
```

★★ **The two absences are the rule discriminating, not a gap.** `f6251--2025` and `f1040sd--2025` are bundled
and are **not** listed, because those two collectors do name 2025 via `Coverage::quoting_year` —
`cover_form6251line1` for the OBBBA line-1 region, `cover_scheduledlines` for the twelve per-box rows. A rule
that reported all fourteen would be counting maps rather than measuring quotation.
`tests::a_bundled_year_quoted_from_another_booklet_is_reported_and_a_quoted_one_is_not` (`:2622`) pins all
three cases — covered-at-this-year (silent), covered-only-elsewhere (reported), covered-nowhere (silent) — and
then removes the 2025 rows to watch `f6251--2025` enter the report.

**Closed:** the silence. Bundling `f6251/2026` now raises the count and reds, where before it moved nothing
and 377 money lines went on being verified against TY2024 extracts.

**NOT closed:** the 12 themselves. Re-pointing them needs per-year coverage rows for twelve forms, because the
`cover_*` collectors take year-agnostic structs and cannot derive their own revision — the refactor FR-135
names as its full fix, and which the rehearsal placed in the *port machine / step-24 widening* phase rather
than NOW. **So FR-135 should be read as "enforced going forward, live surface pinned at 12", and the
coordinator should decide whether that closes the entry or downgrades it to a scheduled item.** I did not
silently claim the whole thing.

★ The boundary is also stated in the source rather than implied: the rule is scoped to forms the table already
covers, because `f1040v`, `f4868` and `f8283` are bundled and legitimately carry no money line this census
describes — reporting them would red on three correct cases and teach the next author to widen an exemption.

---

## 6. Refuted premises

| premise | verdict |
|---|---|
| **FR-134's mechanism sentence:** *"line_coverage_check.rs:1067 builds the extract path from the coverage entry's own declared year … and porting a form means copying its block, which still says 2024"* | **Two findings conflated, and that citation belongs to the other one.** `:1067` (now `:1590`) is rule (2)'s per-row `<form>--<year>` path over the **coverage table**, whose year comes from `Coverage::quoting` — that is **FR-135**. The gate FR-134 is about is `btctax-forms/tests/f8995a_map.rs:9-10`, pinned by two `include_str!`s. Both are real; the fix for each is different (a glob walk vs. a derived per-year assertion), which is why this report delivers two mechanisms and not one. |
| **the report's proposed fix:** *"one glob-walking test that, for every .map.toml on disk, re-runs the caption extraction against design/forms/extract/<irs_stem>--<year>.txt"* | **Adopted, and it is right** — but *"re-runs the caption extraction"* understates it by 3x. Re-running `f8995a_map.rs`'s extraction over the glob reads **134 of 429** captions and reports success over 17 maps it never opens (section 4.1). The estimate *"about 40 lines"* is also low: the committed rule is about 190 lines plus four layout mechanisms, two of them load-bearing by measurement. |
| *"My throwaway Python version found 44/44 and red on 2/44 … so the checker is about 40 lines and its B1 kill is free"* | **The kill is free; the gate was not.** Over one form and one year there is nothing to discover. Over 38 maps the same idea surfaced two real paraphrases, two unparsed caption conventions and three layout mechanisms — which is CLAUDE.md's own point about deriving the list rather than typing one beside it. |
| **FR-135's counts** | **Confirmed exactly:** 26 blocks say 2024 and 2 say 2025. Also 1 `quoting_year("2024")` and 5 `quoting_year("2025")`, which is why the *bundled-year* count is 12 and not 14. |
| **FR-135:** *"377 money lines keep being checked against TY2024 extracts"* | **Confirmed as the table size** (`377 money lines across 18 form(s)`); the number of bundled pairs that actually carries it is **12**. |
| *the rehearsal's 3,614 / 3,614* | **Reproduced in shape.** This worktree runs 3,621 with my 7 new tests (3,614 before), and the planted defect reds **0** of them at HEAD's checker. |

---

## 7. What I could not do, and why

1. **The two paraphrase fixes (4.2)** are in `crates/btctax-forms/forms/2024/f1040sa.map.toml` and
   `crates/btctax-forms/forms/2024/f8959.map.toml`. **Not in A1's exclusive set, and assigned to no agent in
   the partition table.** Pinned and reported instead of edited. Both are comment-only and neither changes a
   figure.
2. **Retiring `f8995a_map.rs` / `f6251_map.rs`'s pinned caption tests.** Rule (4d) now covers their 44 and 41
   captions as part of 429, so they are redundant for verbatim-ness — but `f8995a_map.rs` also asserts
   `checked == 44`, a **per-map** count guard that a single total floor does not replicate. Not A1's files, and
   the redundancy is harmless. **Recommendation: leave them.** If anyone does retire them, the per-map count
   needs a home first.
3. **A `forms/2026/` on-disk plant for FR-135.** Creating a map with no PDF touches the `btctax-forms/build.rs`
   pairing refusal (A2's file) and would have needed a new committed map, so the ratchet was watched live via
   the 12 to 11 mutation plus the pure three-case kill instead.
4. **Actually re-pointing the 12 quoting literals** — section 5.
5. **A full `make check` / `make gate`.** Run as `cargo nextest run --workspace --no-fail-fast` in the worktree;
   the 7 failures are environmental with their mechanisms quoted in 2.1. `make gate`'s forced-rebuild variant
   was not used (the rehearsal recorded it OOM-killing the linker at 3 GB free, and five agents are concurrent).

---

## 8. The literal gate numbers

```
cargo fmt --all -- --check                                  clean
cargo clippy -p xtask --all-targets -- -D warnings          clean
cargo nextest run -p xtask  (line_coverage_check filter)    14 run, 14 passed, 180 skipped
cargo nextest run -p xtask --no-fail-fast                   193 run, 186 passed, 7 failed, 1 skipped
                                                            (7 = 6 form_delta absent-PDF + 1 harness_check
                                                             CARGO_TARGET_DIR; identical at 376d1141)
cargo nextest run --workspace --no-fail-fast                3621 run, 3614 passed, 7 failed, 12 skipped
xtask line-coverage                                         exit 0
  377 money lines / 18 forms / 31 exceptions (ratchet 31) / 0 unverifiable (ratchet 0)
  17 not line-bound (ratchet 17)
  429 map captions / 38 bundled maps / floor 429 / 2 pinned paraphrases
  12 bundled years quoted from another (ratchet 12)
diff                                                        2 files, +1005 -2
  crates/xtask/src/line_coverage_check.rs                   +994  (7 new tests)
  crates/btctax-core/src/tax/line_coverage.rs               +13   (doc comment only)
```

Ablations — each measured by removing one mechanism and re-running the scan over all 38 maps:

| mechanism removed | result | false failures |
|---|---|---|
| none (committed) | 429 captions, 2 problems | 0 — both are 4.2's real paraphrases |
| `destub` | 8 problems | 6 |
| `rejoin_column_header` | 6 problems | 4 |
| leader dots | `f8959/2025` L5 + L9 fail | 2 |
| quote glyphs | `f1040sb/2025` line 8 fails | 1 |
| the two L-prefixed shapes | 429 to 388 captions read; 41 unread, incl. all 17 on `f8959/2025` | — |
| the two-digit label cap | Schedule A/2025's header quote of the TY2024 sentence reads as a TY2025 caption | 1 |
