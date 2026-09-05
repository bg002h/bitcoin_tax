# label_reader dropped Form 6251 line 1a — mechanism, fix, and what else it was hiding

- **Date:** 2026-09-04
- **Branch:** `fix/label-reader-drops-1a` (from `feat/schedule-1a-ty2025`)
- **File fixed:** `crates/xtask/src/label_reader.rs`
- **Gate:** `make check` exit 0, **2768/2768 tests pass**, 12 skipped

---

## 1. The mechanism — it was never about `1a`, and never about sub-letters

The reader's sub-letter state machine was fine. The defect was one level up, in **which column the
reader decided was the label column**.

An IRS form prints each line number **at least twice**: once in the left margin, and again in the
right-hand gutter beside the amount box. `find_label_column` picked one of them by *the longest
non-decreasing run of numeric tokens*, and two independent facts conspired to make it pick the
gutter on Form 6251.

**(a) The margin was under-counted.** Clusters are keyed on the token's RIGHT edge (`x2`) — correct,
because the column is right-aligned. But a letter suffix hangs past that edge. Measured on
`f6251--2025`:

```
  122.78 x=  45.67 x2=  55.28 '1a'      <- bucket round(55.28/2) = 28
  170.78 x=  45.67 x2=  50.40 '2'       <- bucket round(50.40/2) = 25
  170.78 x=  50.40 x2=  55.28 'a'
```

`1a` prints as ONE token whose right edge is 4.9pt past every plain number's. Buckets 25 and 26 are
merged; 28 is not. **So `1a` was not a member of its own margin column**, and the margin scored 39
where it should have scored 40. The extraction predicate in `witness_text` was always correct here —
it matches a parent that *spans* the edge — so the selector and the extractor disagreed about what
belonged to the column, and `1a` fell in the gap.

**(b) The gutter was over-counted, structurally.** The margin abbreviates sub-rows to a bare `b`,
`c`, `d`; the gutter spells out `2a`…`2t` in full. That is 20 gutter tokens against 2 margin ones
for the same 20 lines. Measured cluster sizes on `f6251--2025`:

| cluster | x2 | numeric tokens | longest run | what it is |
|---|---|---|---|---|
| bucket 25 | 50.40 | 39 | 39 | the **left margin** — `2`, `3`, … `40` (and, but for (a), `1a`) |
| bucket 248 | 498.16 | 58 | 58 | the **outer money gutter** — `1b`, `2a`…`2t`, `3`…`40` |
| bucket 249 | 499.41 | 48 | 48 | outer gutter, second alignment |
| bucket 202 | 404.40 | **1** | 1 | the **inner money gutter** — `1a`, alone |

58 > 39, so the gutter won.

**And the gutter is structurally incomplete.** A heading has no box and therefore appears in no
gutter at all; and a form with two money columns puts the inner column's labels ~95pt further left.
Form 6251 line 1a is exactly that case — its result feeds line 1b, so its box is inset and its
gutter label sits alone at x2≈404, below the 3-token threshold for being a column at all.

Net effect: a census of 59 labels beginning at `1b`, reporting `0 without a box`, complaining about
nothing. The line it dropped is **the seam to Schedule 1-A** — *"Subtract Schedule 1-A (Form 1040),
line 37, from Form 1040, 1040-SR, or 1040-NR, line 14"* (`design/forms/extract/f6251--2025.txt:18`).

## 2. The class

Not "a parent line with a sub-letter followed by a bare-sub-letter continuation" — that path works.
The class is:

> **On any form where the right-hand gutter out-scores the margin on token count, the census silently
> loses every line the gutter cannot represent: headings, and any line whose amount box sits in an
> inner money column.**

It is a whole-form failure, not a per-line one, and its size is set by how many sub-lettered lines
the form has (which inflates the gutter) rather than by anything about the dropped line.

## 3. The fix — score a column by the LABELS it yields, not the tokens it holds

`find_label_column` → `candidate_columns` (returns **every** plausible column, left-to-right),
`column_tokens` (one column's membership, using the same spanning predicate the extractor always
used), and `resolve` (the sub-letter state machine, returning labels **and** the orphan complaint so
one loop can both score a candidate and extract the winner). `witness_text` then picks the candidate
yielding the most **distinct labels**, ties to the leftmost.

That metric is the quantity the reader exists to produce, and it is immune to both distortions: a
sub-letter counts once whether spelled out or abbreviated, and a column that omits a line scores
lower for omitting it. A margin can therefore never lose to a gutter that repeats a subset of it.

Two deliberate non-changes, both measured:

- **No monotonicity admission gate.** A `run >= 3` filter was tried; `f8949--2024` and `f8949--2025`
  print only lines `1` and `2` per page, so their real column's longest run is 2 and the filter
  refused both forms outright — a 4-label census became a hard error. Admitting every ≥3-token
  cluster changed nothing else across all 32 forms. Prose is rejected downstream by yielding almost
  no labels, rather than by a threshold.
- **No union over columns.** Measured, and it is wrong: unioning every candidate injects prose
  numerals as phantom labels — `45` and `50` on `f1040s1a--2025`, a **38-line** form whose truth is
  48 entry lines; `5a`, `10a`, `12e` on `f6251--2025`. The anchor would break. See §6.

## 4. B1 — the test that reds when the fix is removed

Two, both written **before** the fix and observed red:

1. `label_reader::tests::a_line_whose_gutter_label_sits_in_the_inner_money_column_is_not_dropped` —
   synthetic, and it plants the *class*, not the form. Real measured coordinates from `f6251--2025`
   with the prose removed: a margin at x2≈50.4 whose `1a` hangs to 55.28, an outer gutter at
   x2≈498.16 with more tokens, and `1a`'s gutter label alone at x2≈404.40.
2. `label_reader::tests::form_6251_part_i_begins_at_1a_the_schedule_1a_seam` — the live defect, on
   the real committed fixture, asserting Part I's printed sequence `1a, 1b, 2a…2t, 3, 4` and 60
   labels total.

**Mutation applied:** the pre-fix implementation (`git show HEAD:crates/xtask/src/label_reader.rs`,
everything above `#[cfg(test)]`) spliced back under the post-fix tests — i.e. *fix removed, tests
kept*.

```
     Summary [   0.015s] 7 tests run: 5 passed, 2 failed, 63 skipped
        FAIL a_line_whose_gutter_label_sits_in_the_inner_money_column_is_not_dropped
        FAIL form_6251_part_i_begins_at_1a_the_schedule_1a_seam

  assertion `left == right` failed: the reader locked onto the right-hand gutter and dropped every
  line whose amount box sits in the INNER money column. `1a` is the whole set of them here.
    left: ["1b", "2a", "2b", "2c", "2d", "2e", "3", "4", "5", "6", "7", "8"]
   right: ["1a", "1b", "2a", "2b", "2c", "2d", "2e", "3", "4", "5", "6", "7", "8"]

  assertion `left == right` failed: Part I opens at line 1a — the Schedule 1-A seam.
  Got: ["1b", "2a", "2b", "2c", "2d", "2e"]
    left: Some("1b")
   right: Some("1a")
```

The five pre-existing `label_reader` tests **passed under the mutation** — which is the finding
about them, and the reason the drop shipped: none of them could see it. With the fix restored, all
seven pass.

## 5. Which other archived forms were affected

Geometry was generated for **all 32 archived form PDFs** (17 × TY2024, 15 × TY2025) and
`label-census` run over each, before and after. Seven forms changed count, one changed from a hard
error to an answer; 24 were already reading their margin column and are byte-identical.

| form | labels before | labels after | Δ | recovered (`H` = a heading, which no gutter can ever carry) |
|---|---|---|---|---|
| `f1040s1--2024` | 35 | 64 | **+29** | `1, 2a, 2b, 3–7, 8ᴴ, 9–18, 19a–19c, 20–23, 24ᴴ, 25, 26` |
| `f1040s3--2024` | 19 | 35 | **+16** | `1–4, 5a, 5b, 6ᴴ, 7–12, 13ᴴ, 14, 15` |
| `f1040s3--2025` | 19 | 35 | **+16** | `1–4, 5a, 5b, 6ᴴ, 7–12, 13ᴴ, 14, 15` |
| `f1040--2024` | 39 | 54 | **+15** | `1i, 2a, 3a, 4a, 5a, 6a, 6c, 25ᴴ, 25a–25c, 27–31, 35b, 35d, 36, 38` — **and lost `2b, 3b, 4b, 5b, 6b`, see §6** |
| `f1040sse--2024` | 19 | 24 | **+5** | `5a, 8a, 8b, 8c, 13` |
| `f1040sse--2025` | 19 | 24 | **+5** | `5a, 8a, 8b, 8c, 13` |
| `f6251--2025` | 59 | **60** | **+1** | **`1a`** |
| `f1040--2025` | *hard error* | 66 | — | the reader used to abort: `bare sub-letter 'a' at page 1 y=139.873 has no numeric parent` |

★ The `ᴴ` rows are the second, quieter half of the same defect: a heading has no amount box, so it
can appear in **no** gutter — five of them (`f1040s1` 8 and 24, `f1040s3` 6 and 13, `f1040` 25) had
been invisible on every form that read its gutter. `f1040s1--2024` line 8 heads `8a`–`8z`, which the
census was reporting as entry lines under a parent it had never seen.

**Net +87 labels.** Two forms were losing more than half their lines (`f1040s1--2024` 35 of 64;
`f1040s3` 19 of 35), and Form 1040 itself was reading its gutter on both years.

Unchanged and verified: `f1040s1a--2025` stays at **50 labels, 48 entry lines, 2 without a box** —
the pinned anchor, before and after. `f6251--2024` is unchanged at 59, because TY2024's Part I had a
single line `1` and no inner-column entry.

## 6. Residual — honestly stated, and filed as `FOLLOWUPS.md` FR-27

`f1040--2024` gained 19 labels and **lost five**: `2b`, `3b`, `4b`, `5b`, `6b`. These are not a
regression of the fix's mechanism; they are what a **single-column** reader cannot reach.

Form 1040 puts two entries on one row, in two money columns:

```
2a  Tax-exempt interest . . . 2a |____|    b  Taxable interest . . . 2b |____|
```

The margin prints only `2a`. The `b` is mid-row, at no column at all — so the margin column is
complete for headings and inner-column lines, and the gutter is complete for outer-column lines, and
neither is complete for Form 1040. Before the fix the reader had the gutter's five and was missing
nineteen; now it has the nineteen and is missing five. Strictly better, still incomplete, and — this
is the part that matters — **still silent about it.**

I did not extend the fix to close it, for a measured reason: both obvious closures are wrong.

- **Union over candidate columns** injects prose numerals as labels. On the anchor `f1040s1a--2025`
  it adds `45` and `50` to a 38-line form; on `f6251--2025` it adds `5a`, `10a`, `12e`. The anchor
  test would red, and correctly.
- **Admitting a bare mid-row letter** cannot work on text alone: the English article *"a"* is a bare
  lowercase word on nearly every row of every form.

What is actually owed is a **row model** — a row may carry more than one labelled entry, and the
signal that distinguishes a second entry's sub-letter from prose is *a bare letter with an amount box
to its right*, i.e. it needs witness W2 in the loop. That is a design change to W1, not a bug fix,
and it is scoped and filed as FR-27 with its owning phase.

## 7. The geometry fixture — committed, and why

`design/forms/geometry/f6251--2025.json` **is committed**, alongside the three that were already
tracked. The form PDFs are gitignored (`.gitignore:63`), so CI has none; a fixture is the only way a
test can read a form, which is exactly why `f1040--2024.json`, `f1040s1a--2025.json` and
`f1040sa--2024.json` are tracked. `form_6251_part_i_begins_at_1a_the_schedule_1a_seam` reads it, so
it is committed by the same rule.

The other 28 fixtures generated for §5's sweep are **not** committed: no test reads them, and
committing untested payload is how a fixture set drifts out of correspondence with the forms. The
sweep is reproducible with `xtask extract-geometry <stem>` from a local PDF.

★ The fixture's pinned `pdf_sha256:6995bfd29c6fe1b8…` matches the header of the already-committed
text layer `design/forms/extract/f6251--2025.txt`, so the observation and the transcription are of
the same PDF.

## 8. Files changed

| file | change |
|---|---|
| `crates/xtask/src/label_reader.rs` | `find_label_column` → `candidate_columns` + `column_tokens` + `resolve`; `witness_text` selects by label yield; module doctrine paragraph corrected; two new tests |
| `design/forms/geometry/f6251--2025.json` | new committed fixture (188K, 2553 words, 62 boxes, 2 pages) |
| `design/forms/LABEL_READER.md` | third layout fact recorded in the ⑤ STATUS block, with the measurements and the two refuted fixes |
| `FOLLOWUPS.md` | FR-27, the same-row second entry, with its owning phase |
