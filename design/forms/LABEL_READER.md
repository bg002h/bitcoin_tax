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

## Cost, honestly

Sixteen forms × two years, each needing its label list read off the form once. That is real work and it is
the work — it is the same act as transcribing the form, done once and then held by a test forever.
