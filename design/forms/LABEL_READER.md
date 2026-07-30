# Enumerating a form's labels from its text layer — findings, and why this is not a regex

**Status: characterised, NOT built.** Deliberately. A reader that is wrong on the one form whose answer we
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
