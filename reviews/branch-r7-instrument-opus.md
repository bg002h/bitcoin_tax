# r7 — INSTRUMENT LENS (Opus)

**Date:** 2026-08-02 · **Range:** `4fe5ce4..58515da` · **Brief:** [`BRIEF-r7.md`](./BRIEF-r7.md)
**Scope:** `crates/xtask/src/line_coverage_check.rs` (rules 2b/2c, the `MAX_UNLOCATABLE` ratchet, the
twelve planted-defect kills) + the two corrected rows in `crates/btctax-core/src/tax/line_coverage.rs`.
Lens A's file (`form1040_full.rs`) was not reviewed.

**Result: 0 Critical / 4 Important / 2 Minor / 1 Nit.** Every claim below is verified by executing the
shipped code — either through `check()` on a synthetic table, or by compiling `label_precedes` verbatim
standalone. Every mutation was `cp`-restored; **the tree is clean** (`git status --short` empty,
`git diff -- crates/xtask crates/btctax-core` empty, `crates/xtask/src/line_coverage_check.rs`
md5 `91a36940a71b1c8ae7e15864099e8aab` = HEAD).

★ **Note on HEAD.** Two commits (`01446f8`, `5ac2504`) landed from the concurrent ink lens while I
worked. Neither touches my two files (`git diff 58515da..HEAD -- <both>` is empty), so every result
below holds at the current HEAD. `cargo test -p xtask` = 55 passed / 1 ignored;
`cargo run -p xtask -- line-coverage` = 189 rows, 11 exceptions, 0 unverifiable, 13 not line-bound.

---

★★★ **THE DELIVERABLE THE BRIEF ASKED FOR: I built the misattribution, and then two more.** Rule (2b)
matches `"{label} {quote}"` as a **raw substring with no left boundary**, so a row naming line *N* binds
to the text of any line whose printed label **ends with** *N*. `check()` returns `Ok` on
**f8995 line 5 carrying line 15's verbatim sentence**, on **f1040 line 1z carrying line 11's AGI
sentence**, and on **f1040 line 6b carrying line 16's "Tax (see instructions)"**. §G-27a is marked
**CLOSED** on a guarantee it delivers in one direction only — and the direction it does not deliver is
the one where the wrong line has the *bigger* number, which is the Form 6251 line-33 defect's own
direction (line 12 for line 22).

★★ **The kill test cannot see it, and it is not an accident of the plant.** Both committed (2b) plants
name a victim whose label is *not* a suffix of the claim (line 8's text under line 4; line 2b's under
5b). The suffix direction was never planted, so B1's own question — *"which test reds when this rule is
removed?"* — is answered for half of the rule's domain.

★★ **Five of the checker's fifteen mechanisms still have no kill.** I neutralised each rule and ratchet
one at a time (15 mutations, matrix below). Ten red. **`(3b)` the "-0-" half of rule (3), and all three
ratchets — `MAX_EXCEPTIONS`, `MAX_UNVERIFIABLE`, and `MAX_UNLOCATABLE`, the one this very range
introduces — plus the `(4b)` completeness scan can each be deleted with the suite green.**

★★ **A blank quote is a fully-passing row.** `text.contains("")` is `true` and
`label_precedes(text, "5", "")` matches `"5 "` somewhere on every form, so
`c.line(v, "f8995", "5", "field", Collected, "")` **passes rules (2) and (2b)** and is counted by no
ratchet. Verified against `check()`. The minimum edit that satisfies the compiler's "name this money
field" demand now has a second door, and it is quieter than `field: _`.

★ **Six of the thirteen `MAX_UNLOCATABLE` rows are form lines, filed under a class name that says they
are not.** The comment says *"12 Form 8949 column cells whose quote is a column HEADER rather than any
line's text … neither is a form line."* Six of the twelve quote **Form 8949 line 2's own row text**,
printed with the label `2` immediately before it at `f8949--2024.txt:45` and `:86`. They are unbindable
only because `label_forms` cannot parse the `I-`/`II-` prefix that `fmt_part` writes.

**Verbatim below.**

---

VERDICT: 0 Critical / 4 Important

---

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:195-196 (label_precedes)
CLAIM: Rule (2b)'s needle is matched as a raw substring with NO left word boundary, so a row naming
       line N binds to the text of ANY line whose printed label ENDS with N. The rule delivers its
       guarantee in one direction only, and §G-27a is marked CLOSED on the strength of it.
FAILURE: Three misattributions, each a REAL sentence from a REAL committed extract, each returning
       Ok from the shipped check():

       (a) f8995 line 5, carrying line 15's verbatim sentence:
             c.line(ZERO, "f8995", "5", "line5", Production::Scaled,
                    "Qualified business income deduction. Enter the smaller of line 10 or line 14. \
                     Also enter this amount on the applicable line of your return (see instructions)")
           -> Ok("line-coverage OK: 1 money lines ... 0 not line-bound (ratchet 13)")
           Line 5 is the QBI COMPONENT ("Multiply line 4 by 20% (0.20)"); line 15 is the DEDUCTION
           itself. The doc comment on the field that carries line 5 would now describe line 15.

       (b) f1040 line 1z, carrying line 11's verbatim sentence:
             c.line(ZERO, "f1040", "1z", "line1z", Production::Combine,
                    "Subtract line 10 from line 9. This is your adjusted gross income")
           -> Ok. Line 1z is total wages ("Add lines 1a through 1h"); line 11 is AGI.
           This one goes through the STEM form: forms("1z") = ["1z", "z", "1"], and "1 Subtract line
           10 from line 9…" is a substring of "11 Subtract line 10 from line 9…".

       (c) f1040 line 6b, carrying line 16's verbatim sentence:
             c.line(ZERO, "f1040", "6b", "line6b", Production::Collected, "Tax (see instructions)")
           -> Ok. Line 6b is the taxable part of social security; line 16 is the TAX.

       All three were run as a table through `check()` in-tree and printed Ok; the same three, and the
       asymmetry, reproduce when label_precedes is compiled verbatim standalone (rustc, no workspace).

EVIDENCE: The whole mechanism is two lines —
             let needle = format!("{f} {quote}");
             text.match_indices(&needle).any(|(i, _)| { ... })
       `match_indices` is a plain substring scan. The extract, normalized, reads
             "… 14 Income limitation. Multiply line 13 by 20% (0.20) … 14 15 Qualified business income
              deduction. Enter the smaller of line 10 or line 14. …"
       so the needle "5 Qualified business income deduction. Enter the smaller of line 10 or line 14…"
       matches INSIDE the label "15", at the '5'. For a non-lowercase form the closure then returns
       true immediately (line 200) — the 700-byte stem run-up guards only the bare-letter form.

       THE ASYMMETRY IS THE FINGERPRINT, and it is why the plants miss it:
             line 15's sentence claimed as line  5 -> Some(true)    <- accepted
             line  5's sentence claimed as line 15 -> Some(false)   <- rejected
       Both committed (2b) plants sit on the rejected side: line 8's text under line 4 ("8" is not a
       suffix of "4"), and line 2b's text under 5b (blocked by the stem run-up, not by the needle).
       The suffix direction was never planted, so it was never observed.

       This is CLAUDE.md's standing root cause in the direction it actually occurred: Form 6251 line 33
       transcribed as "Subtract line 32 from line 12" where the form says line 22 — the wrong line
       carrying the BIGGER number, exactly the case (2b) accepts.

       SCOPE OF THE CLASS, measured. Re-implementing label_precedes faithfully in Python (validated:
       it reproduces the shipped binding decision on all 189 committed rows and on all three vectors
       above) and cross-producting every harvested line sentence against every label on the same form
       yields accepted misattributions on EIGHT of the covered forms — 40 on Schedule D, 8 on Form
       8959, 8 on Schedule C, 7 on Form 8960, 4 on Schedule 1, 3 on Schedule SE, 1 on Schedule 3, plus
       the f8995 and f1040 families above.

       THE COMMITTED TABLE IS CLEAN TODAY — this is a latent false accept, not a realized one. Every
       one of the 175 line-bound rows still binds when a left boundary is required (0 diffs).

       THE FIX IS ONE LINE, and it is non-breaking, verified in-tree:
             text.match_indices(&needle).any(|(i, _)| {
                 if i > 0 && text.as_bytes()[i - 1].is_ascii_alphanumeric() { return false; }
                 ...
       With it: `cargo run -p xtask -- line-coverage` prints the IDENTICAL summary (189 rows, 11
       exceptions, 0 unverifiable, 13 not line-bound), `cargo test -p xtask line_coverage` green, and
       vectors (a), (b) and (c) all flip to rejected. (★ Mind the precedence — my first attempt wrote
       `A && B || C`, which parses as `(A && B) || C` and silently left the whole class open; it
       reported "STILL ACCEPTED" and that is how I caught my own patch.)

       ★ ONE VECTOR SURVIVES THE BOUNDARY FIX and needs its own decision: the STEM form. A row named
       "25d" quoting line 25's caption "Federal income tax withheld from:" is accepted, before and
       after. Argued from the form: Form 1040 prints "25 Federal income tax withheld from:" as a
       CAPTION with no amount box, and 25a/25b/25c/25d each have their own box; line 25d's own text is
       "Add lines 25a through 25c". So the caption is not 25d's text and accepting it is wrong. The
       stem form is load-bearing for EXACTLY ONE committed row — f1040 25a, whose quote is
       "Federal income tax withheld from: a Form(s) W-2", i.e. the caption PLUS the sub-line's own
       bare letter (measured: 152 rows bind on the exact label, 22 on the bare suffix, 1 on the stem,
       13 unlocatable). So the stem form can be tightened to "accept only when the quote also contains
       the row's own bare suffix as a token" without touching the table.
```

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:325 and :332 (rules (2) and (2b) on an empty quote)
CLAIM: An EMPTY instruction passes rules (2) and (2b) on any real line label, is counted by no
       ratchet, and requires no reason — so a money field can be added to the table with a
       fully-passing, entirely unverified row.
FAILURE: Run through the shipped check():
             c.line(Usd::ZERO, "f8995", "5",      "smuggled", Production::Collected, "")  -> Ok
             c.line(Usd::ZERO, "f8995", "5",      "smuggled", Production::Collected, " ") -> Ok
             c.line(Usd::ZERO, "f8995", "14",     "smuggled", Production::Collected, "")  -> Ok
             c.line(Usd::ZERO, "f8995", "1z",     "smuggled", Production::Collected, "")  -> Ok
             c.line(Usd::ZERO, "f8995", "(none)", "smuggled", Production::Collected, "")  -> Ok
             c.line(Usd::ZERO, "f8995", "4",      "smuggled", Production::Combine,   "")  -> Ok
       Every one prints "0 exception(s) … 0 unverifiable … 0 not line-bound". No counter moves.
EVIDENCE: `if !text.contains(&want)` — Rust's `str::contains("")` is TRUE for every haystack, so rule
       (2) is vacuous on an empty quote. Rule (2b) is then reached and
       `label_precedes(text, "5", "")` builds the needle `"5 "`, which occurs on every form; the form
       "5" is not all-lowercase, so line 200 returns true immediately. Confirmed by compiling
       label_forms/label_precedes verbatim with rustc against f8995--2024.txt:
             label=5    quote=""    rule(2) contains=true  rule(2b)=Some(true)
             label=14   quote=""    rule(2) contains=true  rule(2b)=Some(true)
       Rules (3) and (4) are the only others that read `instruction`; (3) fires only on Clamped, (4)
       only on Combine-with-"-0-", and an empty quote can never contain "-0-". So a `Collected` or
       `Carry` row with a blank quote faces NO rule at all.

       THIS IS THE OTHER HALF OF THE COMMIT'S OWN THESIS. 58515da is titled "(2c) must not be a way OUT
       of the checker" and its comment says a `(none)` row escapes "Rules (2)/(2b) — and ONLY those".
       That is true of the RULES and false of the COUNTING: the module's own standard, written eight
       lines below the `(none)` branch, is that a row (2b) could not reach must be "COUNTED and pinned,
       not left as a silent shrug … an instrument that cannot say which cases it did not cover is not
       a check." A `(none)` row is not counted in `unlocatable`, and — unless it is an `Exception` —
       not in `MAX_EXCEPTIONS` either. The committed `addl` row is safe only because it happens to be
       an Exception carrying a written reason.

       WHY IT MATTERS BEYOND THE HYPOTHETICAL: r6's I-4 established that the compiler's guarantee is
       "a new money field must be NAMED, not CLASSIFIED", and that the cheapest evasion is `field: _`
       plus a zero in the builder — visible in a diff as a `_`. A blank sixth argument is the same
       evasion with nothing to grep for, and unlike `_` it leaves a row in the table that LOOKS
       classified: it has a form, a line number and a production. Two blanks that are not the same
       thing, indistinguishable on the page — CLAUDE.md's "blank is the normal case", one level up.

       FIX: reject an empty/whitespace-only `instruction` on a non-`(none)` row outright (it is the
       exact mirror of (2c)), and count `(none)` rows in `unlocatable` — they are, by construction,
       rows (2b) did not reach.
```

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:363 (3b), :497 :504 :525 (the three ratchets),
       :461-472 ((4b) completeness)
CLAIM: Five of the checker's fifteen mechanisms have no planted-defect kill. Each can be deleted with
       the whole suite green — including `MAX_UNLOCATABLE`, the ratchet this range introduces and
       documents as "★★★ THE RATCHET ON RULE (2b)'s REACH".
FAILURE: I neutralised one mechanism at a time (each edit makes the guard unreachable, e.g.
       `if false && !q.contains("-0-")`), ran `cargo test -p xtask -- line_coverage`, restored, and
       recorded whether ANY test reds:

         (1a) Exception-with-no-reason ........................ RED
         (1b) reason-on-a-production .......................... RED
         (2)  verbatim quote ................................... RED
         (2b) line binding ..................................... RED
         (2c) "(none)" carries no quote ........................ RED
         (3a) clamp polarity ................................... RED
         (3b) a Clamped row needs a "-0-" clause ............... *** GREEN — nothing caught it ***
         (4)  Combine carrying a clamp clause .................. RED
         (5)  duplicate coverage ............................... RED
         (x)  unknown form stem is an error .................... RED
         (x)  an empty table is not vacuous .................... RED
         (6)  MAX_EXCEPTIONS ratchet ........................... *** GREEN — nothing caught it ***
         (6)  MAX_UNLOCATABLE ratchet .......................... *** GREEN — nothing caught it ***
         (6)  MAX_UNVERIFIABLE ratchet ......................... *** GREEN — nothing caught it ***
         (4b) completeness scan ................................ *** GREEN — nothing caught it ***

EVIDENCE: B1: "no checker exists until it has been observed RED on a planted defect", and its
       reviewable question is one sentence — "which test reds when this checker is removed?" For these
       five the answer is still: none. r6's I-1 fix made that answer correct for the checker AS A
       WHOLE (gutting `check()` now reds, confirmed); FOLLOWUPS §G-27 records exactly that. But B1 is
       per-instrument, and the fold stopped at the twelve rules the plants happened to cover.

       (3b) IS NOT DECORATIVE. r6's own ALSO-CHECKED says the "-0-" half of rule (3) "is what correctly
       forces f1040:34 into the Exception bucket rather than blessing a clamp the form does not
       state" — f1040 line 34's clause matches the FLOOR_IDIOM "is more than line" and is kept out of
       Clamped ONLY by (3b). It is load-bearing for a live disposition and nothing holds it. The
       escape-hatch plant asserts on "polarity is TRANSCRIBED", which is (3a).

       THE THREE RATCHETS ARE UNOBSERVABLE BY CONSTRUCTION while the table sits AT them (11/11, 0/0,
       13/13) — which is precisely the condition under which a ratchet needs a synthetic table to be
       watched, and `check(&Coverage)` now makes that a three-line test: push one more Exception than
       the ratchet and assert the error. Same for MAX_UNLOCATABLE (push a row labelled "QDCGT
       Worksheet, 3") and MAX_UNVERIFIABLE.

       (4b) is the completeness scan whose SCOPE r6 filed as G-27b/G-27c. Those stay open and out of
       scope here; this is the different point that it also has no kill, so a future edit to
       `money_bearing_types` or `mentions_ident` that made it match nothing would be silent.
```

```
SEVERITY: Important
WHERE: crates/xtask/src/line_coverage_check.rs:521-524 (the MAX_UNLOCATABLE residue comment) and
       FOLLOWUPS.md §G-27a's CLOSED entry
CLAIM: The ratchet's account of its own residue is wrong for 6 of the 13 rows, and the count it
       reports is short by one — so the ratchet carries unearned slack and misnames what it holds.
FAILURE: The comment reads:
         "The residue, by class: 12 Form 8949 column cells whose quote is a column HEADER rather than
          any line's text, and the Qualified Dividends & Capital Gain Tax Worksheet line 3 … Both are
          stated, neither is a form line."
       Enumerating the rows `label_forms` cannot express gives 13, and they split two ways:

         SIX are genuinely column headers   f8949 I-1(d) "Proceeds", I-1(e) "Cost or other basis",
                                            I-1(h) "Gain or (loss)", and the Part II twins.
         SIX are FORM LINE 2's OWN TEXT     f8949 I-2(d), I-2(e), I-2(h) and the Part II twins, all
                                            three quoting "Totals. Add the amounts in columns (d),
                                            (e), (g), and (h) (subtract negative amounts). Enter each
                                            total here and include on your"
         ONE is the QDCGT worksheet line.

EVIDENCE: `design/forms/extract/f8949--2024.txt:45` (and `:86` for Part II) reads

           2 Totals. Add the amounts in columns (d), (e), (g), and (h) (subtract
                negative amounts). Enter each total here and include on your
                Schedule D, line 1b (if Box A above is checked), line 2 (if Box B
                above is checked), or line 3 (if Box C above is checked) . . .

       — the label "2" printed immediately before the sentence, which is the exact shape rule (2b)
       exists to recognise. Form 8949 line 2 is a form line, it is the TOTALS line, and it carries
       figures btctax writes. Those six rows are unbindable not because their quote is a header but
       because `fmt_part` (line_coverage.rs:1992) writes the label as "I-2(d)" and `label_forms`
       (line 170-173) strips only at '(' — a leading "I-"/"II-" leaves the stem empty and returns
       None. Stripping a leading part prefix would bind all six against line 2, and MAX_UNLOCATABLE
       would drop 13 -> 7.

       And the count itself: the summary prints "13 not line-bound" while FOURTEEN rows are not bound
       — the `(none)` row is not line-bound either and (2c) now routes it past the counter (finding
       above). §G-27a's closure sentence carries both errors at once: "**189 of 189 rows bind**; 13 do
       not", which is self-contradictory on its face; the true figures are 152 exact-label + 22
       bare-suffix + 1 stem = 175 bound, 13 counted-unbound, 1 uncounted-unbound.

       WHY THIS IS MORE THAN A COMMENT FIX: a ratchet's only content is what it is understood to hold.
       Six units of slack sit in MAX_UNLOCATABLE under a class name ("column headers, not form lines")
       that tells the next author those rows are unbindable in principle. They are not, and the
       instrument's own doctrine — "an instrument that cannot say which cases it did not cover is not
       a check" — is the standard it fails here.
```

---

**Minor / Nit (recorded, not gating)**

- **M-1 — the 700-byte run-up window is sound in both directions, with the numbers.** I could not
  exploit it: brute-forcing every lettered line on all twelve covered forms against claims `1..99` ×
  that line's letter, with the left boundary of finding 1 repaired, yields **zero** bare-suffix
  misattributions. In the other direction it is over-provisioned, not tight: across the 22 committed
  rows that bind through the bare-suffix form the **largest** run-up distance to the row's own stem
  token is **241 bytes** (`f1040sa:8a`), the next is 31, and the remaining 20 are 4–5 bytes — the
  amount-column echo of the previous sub-line ("`… 2a b Taxable interest`"). So 700 has ~3× headroom
  and no committed transcription is near the edge. The magic number is defensible; it would be worth
  one sentence in the comment saying the observed maximum is 241, so a future reader can see the
  margin rather than re-deriving it.
- **M-2 — the two corrected rows are right, and (2b) is what forced one of them.** `f1040sse:13`
  (`SeTaxResult.deductible_half`) now quotes the full sentence, verbatim against
  `f1040sse--2024.txt:49-50`, and — the part worth noting — **identical to its `ScheduleSeLines::line13`
  twin** (line_coverage.rs:766 vs :2096). The old fragment `"Multiply line 12 by 50% (0.50)."` began
  mid-sentence and could not have been preceded by the label "13", so (2b) could not have passed it:
  this is the rule discriminating on a real row, which is the best evidence in the range that it does
  something. The `addl` row's empty instruction is correct under (2c) and it remains an `Exception`
  with a written reason, so it is still counted in `MAX_EXCEPTIONS`. ★ r6's I-6 (the two Schedule SE
  shapes contradicting each other on lines 10, 4c and 2) is filed as §G-27d and is **unchanged** by
  this range — line 13 was the one pair the fold brought into agreement. Not re-filed.
- **N-1 — `MAX_UNLOCATABLE` is a function-local `const` (line 524)** buried mid-`check()`, while
  `MAX_EXCEPTIONS` and `MAX_UNVERIFIABLE` are module-level items carrying the ratchet-history doc
  comments that make raising them a visible decision. Raising a `const` inside a function body is a
  quieter diff than raising the two it is meant to sit beside.

---

**ALSO CHECKED, SOUND:**

- **(2c) is an `else`, not a `continue`, and nothing below it silently assumes a non-empty
  `instruction`.** Rule (3) on `normalize("").to_lowercase()` yields `says_floor == false` and
  `says_ceil == false`, so a `(none)` row declaring a clamp fails **both** halves — fail-closed, and
  the committed escape-hatch plant is a real kill (mutating (3a) reds it). Rule (4) cannot fire on an
  empty quote, but it is a rejection rule, so that is absence of a trigger rather than a wrong pass.
  Rule (5) keys on `e.line`, so two `(none)` rows with different fields correctly do not collide. The
  concern the brief raised is clean; my finding above is the *counting*, not the rules.
- **The twelve committed plants are real kills and each reds for the rule it names.** The matrix in
  finding 3 shows ten distinct mechanisms whose removal reds the test. I also checked the converse —
  that no plant reds for a reason other than its own — by inspection of each: the (2b) line-8-under-4
  plant declares `Clamped(FloorAtZero)` with a matching floor clause so rule (3) is satisfied and only
  (2b) can fire; the 5b/"Taxable interest" plant is `Collected` with no clamp language; the (2c) plant
  is an `Exception` carrying a reason so rule (1) is satisfied; the (3) plant's quote *is* line 16's
  own text so (2)/(2b) pass; the (4) plant's quote *is* line 4's own text. The control row is asserted
  `is_ok()` first, which is the right guard and the reason the plants differ in exactly one way.
- **`label_forms`'s vacuity path cannot currently be used to dodge (2b) deliberately** — a label with a
  parenthesis, a trailing space, a three-digit stem or a two-character suffix returns `None` and
  increments `unlocatable`, and since the table sits **at** the ratchet (13/13) any such row fails the
  build. That door is shut. It is shut by a numeric coincidence rather than by a rule, and the two
  doors that *are* open (`(none)`, and a blank quote) are the findings above.
- **The committed table has no realized misattribution.** All 189 rows were replayed through a
  boundary-anchored `label_precedes`: **zero** rows change verdict, i.e. not one of the 175 line-bound
  rows is bound only through the leak. Finding 1 is a latent false accept.
- **`MAX_UNVERIFIABLE = 0` and the emitted/typo split are correct as written**, and the `(none)` branch
  sits *after* the extract lookup, so a `(none)` row on a form with no text layer still lands in
  `unverifiable` or errors — it does not skip that half.
- **Rule (2)'s `normalize` still does its job on the new quotes**: the corrected `deductible_half`
  sentence spans two physical lines in the extract (`f1040sse--2024.txt:49-50`) and matches only
  because of the whitespace collapse.
- **The 189/14-form/11-exception summary is stable** across the range and unchanged by the two commits
  that landed during this review.

**WHAT WOULD MAKE THIS REVIEW WRONG:**

1. If the coverage table is understood as **documentation that no code will ever consume**, findings 1
   and 2 are doc-quality issues rather than defects — a wrong doc comment on a money field misleads a
   reader but marks no return. I read them as Important because §G-11 P0b/T3a plan to make `LineEntry`
   honour the declared production and blank-ness rule, at which point a row bound to the wrong line is
   a line *built* from another line's rule; and because this module's entire thesis is that the
   quote↔line binding is the thing review cannot be trusted to check. If P0b is abandoned, downgrade.
2. If `MAX_UNLOCATABLE`'s residue comment is read as a rough gloss rather than a classification,
   finding 4 is a Nit. I read it as Important because it is the ratchet's *only* statement of what it
   holds, because §G-27a's CLOSED entry repeats it verbatim as part of the closure argument, and
   because six units of unearned slack in a ratchet is exactly the currency the next author spends.
3. If the three ratchets are considered unkillable-by-nature (they cannot red while the table sits at
   them), part of finding 3 is unfair. I do not think so: `check(&Coverage)` — the r6 fold's own
   contribution — makes a synthetic over-ratchet table a three-line test, which is precisely why the
   fold's other twelve kills became expressible.
4. I did not attempt a false POSITIVE on (2b) — a *correct* transcription the boundary fix would newly
   reject. I bounded it instead: 0 of 189 committed rows change verdict, and the bare-suffix run-up
   has 3× headroom. A future form whose text layer prints a label glued to the preceding token (no
   space) would break under the fix and does not exist in the current corpus.
5. I reviewed `form1040_full.rs` not at all, per the brief, and the concurrent ink lens committed twice
   into the shared tree during this review. If either commit had touched my two files my results would
   be stale; `git diff 58515da..HEAD` over both files is empty, so they are not.
