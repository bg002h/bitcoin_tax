# Enumerating a form's labels from its text layer — findings, and why this is not a regex

> ## ⑤ STATUS 2026-07-30 — witnesses BUILT (increment 1 of 2)
>
> Built on the ④ consult's `hybrid` verdict (`reviews/label-reader-strategy-fable-r1.md`):
> `crates/xtask/src/form_geometry.rs` (committed raw observation) and
> `crates/xtask/src/label_reader.rs` (both witnesses). `xtask extract-geometry <stem>` regenerates
> the fixture from a local PDF; `xtask label-census <stem>` runs the witnesses.
>
> **On Schedule 1-A the text witness yields 50 labels, up from the regex's 45**, and recovers every
> class the regex dropped — lines 1/3, the bare sub-letters `2b`–`2e`, and `22a`/`22b`.
>
> ★★ **Two layout facts were discovered by building it, and both would have silently dropped lines:**
>
> 1. **The margin column is RIGHT-ALIGNED.** Single digits start at x≈45.4, double digits at x≈40.4 —
>    two left-edge clusters for one column. Clustering on `xMin` yielded labels 10..38 and silently
>    dropped 1..9 while still returning a long, plausible list. Their right edges coincide at x2≈50.4.
> 2. **A letter suffix hangs PAST that right edge** (`1` ends at 50.4, `2a` at ~55), so matching on
>    `x2 ≈ right` dropped `2a`, and the bare sub-letters then inherited `1` as parent and came out as
>    `1b`..`1e`. Parents are matched by *spanning* the column edge instead.
>
> ★ Both were caught only because the ONE form with a known answer was the test. Neither would have
> been visible on a form whose truth we had not established by hand — which is the argument for the
> anchor form, made concrete.
>
> ### ★★★ 50 vs 48 — RESOLVED by opening the PDF. Both counts were right.
>
> | | |
> |---|---|
> | **50** | printed line labels — what the text witness enumerates |
> | **48** | of those, the ones that TAKE AN ENTRY — **the hand-established figure, reproduced exactly** |
> | **2** | headings with no box of their own: **4** (heads 4a–4c) and **22** (heads the VIN table; its columns are (i)/(ii)/(iii) and line 23 adds rows 22a/22b) |
>
> `14`+`a` and `36`+`a` share a y-row and merge mechanically to `14a`/`36a` (measured, `|dy| < 3.0`).
>
> ★★ **Nothing was tuned to make the numbers agree.** The witnesses were corrected against the FORM,
> and 48 fell out. Had the reader been nudged to emit 48 *labels*, the two headings would have
> vanished from the census entirely — which is exactly the "we forgot this line" defect it exists to
> catch.
>
> ### ★★ Three box-assignment models, two refuted by the rendered page
>
> | model | refuted by |
> |---|---|
> | `y ± 12pt` fixed tolerance | **4a reported box-less.** Its label sits at the top of a three-line paragraph, its box ~36pt below at the foot — beyond any fixed window. |
> | span label→next, testing the box's **top** | **22 reported as HAVING a box.** Boxes are vertically centred on their row, so 22a's VIN box top (159.0) sits a hair above label `a` (161) and bleeds upward. |
> | nearest label to the box centre | **11, 19, 28 reported box-less.** On a two-line paragraph the box is nearer the NEXT label, which steals it. |
>
> **What works: the last label at or above the box's CENTRE.** The centre kills the bleed (a centre
> is unambiguously inside its own row); "at or above" kills the theft (a box can never be claimed by
> a label printed below it).
>
> ★ Every one of these produced a *plausible* wrong answer, and each fix would have masked the
> others. None was visible from the text layer, the field names, or the extract — only from the page.
>
> ### ★★★ OWNER CORRECTION 2026-07-30 — a LINE is not a CELL, and the grid forms need cells
>
> Lines 22a/22b are not single entries. The form prints **three columns** and the AcroForm carries
> **six fields** for the two rows:
>
> | cell | field | note |
> |---|---|---|
> | 22a(i) | `Line22a.VIN-1_Comb` | **`maxlen=17`** — ONE field, drawn as 17 per-character cells |
> | 22a(ii), 22a(iii) | `Line22a.f2_02`, `f2_03` | single boxes |
> | 22b(i) | `Line22b.VIN-2_Comb` | `maxlen=17` |
> | 22b(ii), 22b(iii) | `Line22b.f2_05`, `f2_06` | single boxes |
>
> ★ The VIN is **one entry** in the data model, rendered as a comb of character cells. Both readings
> are true at different layers, and `maxlen` is load-bearing: this crate already learned that a comb
> takes BARE characters, because a `/MaxLen 9` SSN comb silently truncates a hyphenated value.
>
> ★★ **The FORM addresses cells by column.** Line 23: *"Add lines 22a and 22b, **column (iii)**"*.
> The Caution: *"Column (iii) is the total QPVLI paid in 2025 less the amounts reported in **column
> (ii)**."* Column is part of the address, stated by the authority itself.
>
> **Both counts survive, at different granularities:**
>
> | | |
> |---|---|
> | **48** | entry-taking **LINES** — what the census currently reports, and the hand-established figure |
> | **52** | entry **CELLS** — 48 lines, with 22a and 22b each holding 3 |
>
> ★★★ **Why this is not a line-22 quirk.** A census at LINE granularity reports `22a` as covered when
> column (iii) was never mapped — "we forgot this line", one level down, invisible. And **`f8949` is
> a grid with 190 fields**, so at line granularity the biggest form in the set would get the weakest
> check. **Increment 2's ledger must therefore key on CELL ADDRESS (row + column), not on the line
> label.** Recorded here because it changes the ledger's shape before it is built, which is the
> cheapest moment to learn it.
>
> **NOT yet built (increment 2):** the committed per-form **ledger**, and the typed disagreement
> rules (BOX-WITH-NO-LABEL = hard fail, LABEL-WITH-NO-BOX = must carry a kind, SEQUENCE GAP = hard
> fail). Until those land, this is two witnesses and a viewer, not yet a gate.

**Status of the original characterisation below: superseded in part.** Deliberately. A reader that is wrong on the one form whose answer we
know would manufacture exactly the false confidence this project keeps getting burned by — and the whole
purpose of the label census is to distinguish *"this line encodes no decision"* from *"we forgot this
line."* A reader that quietly finds 45 of 48 cannot do that.

## Measured behaviour of the obvious approach

A leading-number regex over `design/forms/extract/`:

| form | leading-number | category-column | verdict |
|---|---|---|---|
| f6251, f8995, f1040sd, f1040sc, f8960, f1040s1a | 40, 16, 24, 23, 21, 35 | 0 | plausible |
| **f1040sa**, **f1040** | **0**, **0** | 11, 17 | ★ a *second layout* — the line number sits in a second column beside a category label ("Medical / and / Dental Expenses") |
| **f8949** | **2** | 0 | ★ a *grid*, not a numbered list — neither pattern applies |

★★ **`0 labels` must be a hard failure, never a quiet pass.** Two of sixteen forms return zero under the
obvious pattern; a census that accepted that would report a form as fully conformant by having nothing to
check. Same trap as a `BTreeSet` built from `1..=38`, and as a direction check keyed to a hand-list of
three parts — this is its third costume.

## The three sub-problems, from the one form with a known answer

Schedule 1-A's label set is **48** (established by hand: I 7, II 12, III 10, IV 10, V 8, VI 1). The reader
finds **45**, and each shortfall is a distinct problem, not a tuning issue:

1. **Whitespace assumptions.** Lines 1 and 3 are `  1       Enter the amount from…` — *seven* spaces after
   the number, where the pattern allowed six. Trivially fixable, and precisely the sort of accident that
   makes a "working" reader silently drop lines on the next form.
2. **★ Sub-letters have no parent in the text.** `2b`-`2e`, `4a`-`4c`, `14b`-`14c`, `36b` appear as a bare
   `b` / `c` on their own line. Resolving them requires carrying the last numeric parent — so the reader
   is a small state machine, not a filter.
3. **★★ Some numeric lines are HEADINGS, not labels.** Lines **4**, **14** and **22** carry no amount box —
   each is a heading for its lettered sub-rows. The reader emits `22` and misses `22a`/`22b` (the VIN grid
   rows are a bare `a`/`b` with *nothing after them*, so a pattern requiring following text drops them).
   Distinguishing a heading from a label means knowing whether the line has an amount box, which the text
   layer does not directly say.

## The design that follows

**The reader proposes; a human-established list is the authority.** Per form:

- an **expected label list** (not merely a count), established by reading the form once — the way 48 was;
- the reader's output asserted **equal** to it, so any extraction drift or layout change reds;
- **zero labels is always a failure**, whatever the layout;
- forms whose layout is not yet analysed sit in an explicit ratchet list that may only shrink — never
  silently absorbed by a permissive pattern.

★ Note the asymmetry that makes this sound: the expected list is a *pinned observation of the form*, which
is legitimate (it reds when the form changes). Pinning the *reader's own output* would be circular — it
would assert only that the reader still does what it did.

## ★★★ MEASURED 2026-07-30 — the AcroForm lead, and what it actually gives

`CONTINUITY.md` §5a called `xtask dump-fields` *"the lead most likely to change the answer"* and
speculated that a fillable form's field list **is** an enumeration of its boxes. That was worth
**measuring rather than asking a reviewer to guess about**, so it was measured.

**The naive hope is FALSE.** Field names are overwhelmingly sequential, not semantic:

```
form1[0].Page1[0].f1_01[0]   form1[0].Page1[0].f1_02[0]   …   f1_31[0]
```

and semantic naming is **wildly inconsistent across forms**:

| form | fields | lines named semantically |
|---|---|---|
| f1040s1a | 54 | `Table_Line22`, `Line22a`, `Line22b`, `VIN-1_Comb` |
| f1040sa | 30 | 4 (`Line2`, `Line8`, `Line8b`, `Line18`) |
| f1040 | 126 | **1** (`Line28`) |
| f8949 | 190 | `Table_Line1_Part1`, `Table_Line1_Part2` |
| **f6251** | 62 | **ZERO** |

So **AcroForm names cannot enumerate labels.** A strategy resting on them would work on Schedule 1-A
and collapse on Form 6251.

**But the field GEOMETRY is universal, and it answers the exact question the text layer cannot.**
§"three sub-problems" #3 says distinguishing a heading from a label *"means knowing whether the line
has an amount box, which the text layer does not directly say."* The AcroForm says it precisely — an
amount box is a field, with coordinates. And `pdftotext -bbox` gives every word's coordinates too:

```
<word xMin="45.396" yMin="120.649" …>1</word>      ← the printed line number, left margin
p1  504.0, 396.0- 576.0, 408.0  text  …f1_12[0]    ← an amount box, right column
```

Two coordinate systems (bbox is top-down, AcroForm bottom-up, page height ~792), which is a
**mechanical flip, not a heuristic**. Join on the flipped y and you get, per row: *the line number
printed on it* and *whether it carries an amount box*. That is sub-problem #3 solved by construction
rather than by tuning — and sub-problem #2 (parentless sub-letters) largely too, since a bare `b` on
its own row still has a y and still sits under its parent's y.

★ It also names the hardest case outright: lines **22a/22b**, which the regex *"misses entirely"*, are
`Table_Line22[0].Line22a[0]` in the AcroForm.

★★ **What this does NOT settle**, and why the consult is still worth having: the AcroForm enumerates
**boxes**, and the census question is about **lines**. A heading line (4, 14, 22) has no box and must
still appear in the label set; a single line can own several boxes (22a's VIN + two amounts). So the
join is evidence, not an oracle — and 54 fields ≠ 48 labels on the one form whose answer we know.

## Cost, honestly

Sixteen forms × two years, each needing its label list read off the form once. That is real work and it is
the work — it is the same act as transcribing the form, done once and then held by a test forever.
