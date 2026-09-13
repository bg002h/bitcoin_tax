# REPORT — wave 1 B: FR-209 + FR-211 (TY2026 work list)

**Agent:** opus, worktree `agent-a563c69e0020ddecb`, brief `design/agent-reports/BRIEF-wave1-B-worklist.md`
(`323cbeba4`). **Owned and touched:** `design/TY2026_WORK_LIST.md`, `crates/xtask/src/form_delta.rs`.
Nothing else. `form_delta.rs`'s axis logic and `label_reader.rs` were read, never edited — the axes are
consumed, not authored. **NOT committed, NOT pushed.**

**Gate:** `make gate` (forced rebuild — this run follows a plant/restore cycle, so `check` alone could
have raced mtime freshness) **3674/3674 passed, 12 skipped, clippy 0 errors / 0 warnings, exit 0**.
`cargo fmt --all --check` exit 0. Both foreground.

---

## 0. ★★ The brief's premise was one commit stale — and its numbers were all correct

**Disproved, as instructed to report.** The brief says *"TY2026_WORK_LIST.md:41 says
| f6251 | 62 | 0 | 0 | 0 | unchanged |"* and that five forms *are recorded* `unchanged`.
**At `323cbeba4` the table did not say that.** The axes commit `2d2c3ff7c` had already regenerated the
whole numeric table in the same commit that added the axes, so the committed f6251 row read
`| f6251 | 62 | 0 | 0 | 0 | 1 | 8 | port |` and the five rows read `port` with their
8 / 12 / 2 / 2 / 6 meanings printed. The stale citation lives on in `FOLLOWUPS.md:8556`, which is where
the brief inherited it.

**Every measurement in the brief is otherwise confirmed, twice, by two independent paths** (per-pair
`form-delta` over all 15 stems, and `port-status`): f6251 **8**, f8959 **12**, f8960 **2**, f1040sb
**2**, f8995a **6** — 30 moved meanings — and `f1040sd` is the only genuinely unchanged form. So
FR-209's *numbers* were folded a commit before the brief was written; its **structural** half — make the
table incapable of conflating the two claims again — was not, and that is what this work does.

One refinement to FR-211's wording: **Schedule A is not a "0 collisions" case.** It prints **8**
collisions and **2** gaps. The zero-versus-unmeasured defect is real, but its sharpest instance is
`f8949` (0 compared, 2 gaps, cell printed UNWITNESSED with no gap count anywhere) and the
`lines that moved` column generally — see section 2.

---

## 1. FR-209 — two claims, two columns, two vocabularies with no word in common

The `shape` column (REBUILT / unchanged / port) is gone. Two cells replace it, derived by two functions
that the printer **and** the checker both call, so each word exists in exactly one place:

| column | question it answers | vocabulary |
|---|---|---|
| `field map` | do the map's box names carry forward? | transfers / edits / REBUILT |
| `the FORM` | is this the same printed form? | unchanged / CHANGED / UNWITNESSED |

`FieldMapVerdict::cell` and `FormVerdict::cell` have **disjoint** ranges, which is what makes a conflated
cell a test failure rather than a judgement call. **Form 6251 is now `transfers | CHANGED`** — its field
map genuinely does transfer (62 paired, 0 added, 0 removed, **0 respelled**), so the 2026-09-11 owner
ruling that greenlit its transcription on exactly that ground stands and is now *machine-asserted* in the
test; and 8 surviving line numbers changed meaning, so the "nothing to do" reading is refused by a
different cell. `f1040sd` is the only row that reads `unchanged`, and `f8949` — where nothing was
compared — reads `UNWITNESSED` rather than borrowing that word.

### Three quantities the table was not printing, found while re-deriving the rows

1. **`respelled` — 112 map edits, invisible.** A paired box whose FQN spelling changed is the same box on
   every other axis and a **map edit** all the same, because the bundled map stores the full FQN.
   Schedule C's row priced **6** edits (5 added, 1 removed) while **45 of its 104 paired boxes** were
   respellings. Also Schedule 1-A 44, Schedule 2 15, Schedule 3 2, Schedule SE 2, Form 8995 2,
   Schedule A 1, Schedule 1 1. **This is a pricing defect in the document that prices the port**, and it
   is the FR-209 family exactly: the field-map columns were showing a third of the field-map axis.
2. **`lines introduced` — 65 printed line numbers that did not exist on the prior revision** (Schedule
   1-A 28, Schedule 2 16, Schedule A 10, Form 8995 6, Form 8995-A 3, Schedule C 1, Form 6251 1). The
   table showed what a revision *retired* and never what it *added*, so a form could introduce a line and
   read as all zeroes. It is also a conjunct of the `the FORM` verdict, so omitting it would have left
   that verdict unexplainable from its own row.
3. **`unpairable` — 1** (Schedule A): a box neither added nor removed nor compared, because its leaf name
   repeats on one side. The pairing axis's own gap.

---

## 2. FR-211 — the gap count beside the collision count

Two new cells, `boxes UNREAD` and `line numbers UNREAD`, produced by the **same closure** that decides
every other axis cell, each witnessed by what makes it *knowable* rather than by its neighbour:

* `compared + unwitnessed == common` is an invariant of the label axis, so the box gap is **always a
  number** — including on `f1040s1`, whose `lines that moved` is UNWITNESSED and whose **73** unread boxes
  are the entire reason it is.
* the caption gaps are enumerated from the **surviving line set**, so `f8949` now prints
  `UNWITNESSED | 2` — nothing compared, two line numbers unread — which is precisely the pair of facts one
  cell could not hold.

**Measured census (column totals, computed by script, not counted by hand):** **137** unread boxes and
**13** unread line numbers. Worst rows: `f1040sc` printed `33` moved out of 104 paired boxes with **22**
of them unread; `f8949` printed `0` moved with **16**; and **`f1040sd`'s `unchanged` rests on 51 of its 55
paired boxes** — the other four sit beside no printed line on *either* revision, so no line-move
comparison exists for them to fail. That reading is now printed beside the verdict, and the document
states in terms that neither verdict may be read without those two columns.

★ The brief scoped FR-211 to the caption cell. The identical defect was measurably present in the
`lines that moved` cell (the 137 above), and it bites the one row carrying the `unchanged` verdict, so
both axes got the treatment. The line-**set** axis (retired / introduced) has **no** per-line gap count
available — a label the reader never read is invisible to a set difference — and that boundary is stated
in the source rather than papered over.

## 2a. What FR-210 costs this table (not fixed, as instructed)

**9 of the 13 unread line numbers are FR-210's ambiguity**: `f1040s2` 1/13/17, `f1040s3` 13,
**`f1040sa` 17** and 5, `f1040sc` 16, and **both** of `f8949`'s. The other 4 are captions the reader found
empty on the new side (`f1040s1a` 4a/4b/4c, `f8995` 1i).

**The cost, plainly:** the most consequential Schedule A collision — TY2025's line-18 checkbox against
TY2026's itemized total — is inside that 9 and **prints as a gap, not as a hit**, so Schedule A's `8` is a
floor rather than a count. And `f8949`'s caption axis is blind *entirely* because of it: both its line
numbers are ambiguous on the old side, which is why its `the FORM` verdict is UNWITNESSED and not
`unchanged`. FR-210 stays OPEN; nothing here was re-based.

---

## 3. B1 — the new checks were watched going red, five ways

`the_work_list_checker_reds_on_every_planted_row` now carries **27 one-cell plants + 4 controls**, and the
plants are no longer typed out: **each is the printer's own row for a stem with exactly ONE cell
replaced**, the column named by the word the table prints and its index read off the printer's own header
line. A plant therefore cannot disagree with the tool in a second cell and red for a reason it did not
intend (the failure documented at the top of that test), and widening the table from 6 cells to 13 did not
mean re-typing 13 numbers 27 times. The four controls (f6251, f8949, f1040sd, f1040s1 clean) are what make
that safe.

Each mutation below was applied, measured, and restored by cp-backup:

| mutation | result |
|---|---|
| **the FR-209 defect itself** — the doc's f6251 `the FORM` cell set to `unchanged` | **2 tests RED**: the committed-work-list test with *"f6251: column the FORM says unchanged, form-delta at HEAD says CHANGED"*, and the printer-vs-document test with the full row diff |
| verdict comparison neutralised to `if false` | **RED** — *"unchanged printed for a form that moved the meaning of 8 line numbers: []"* |
| both FR-211 gap-cell comparisons deleted | **RED** — *"the BOXES-UNREAD cell off by one: []"* |
| respelled/unpairable dropped from the counts tuple | **RED** — *"the RESPELLED cell off by one: []"* |
| the `introduced` axis call deleted | **RED** — *"the INTRODUCED cell off by one: []"* |

Source verified byte-identical to the pre-mutation copy afterwards (`diff` clean), and the gate was then
run as `make gate`, which touches every `.rs` before building.

**Which test reds when this checker is removed?** All three of
`the_committed_work_list_matches_form_delta_at_head` (all 11 counts plus both verdict words, per row),
`port_status_prints_the_committed_work_list` (printer against document, cell for cell) and
`the_work_list_checker_reds_on_every_planted_row` (the 27 plants).

**Nothing in the table was hand-transcribed.** The numeric table was spliced into the document from
`xtask port-status` stdout by script; the only cell edited afterwards is `f1040s1`'s moved cell, whose
longer FR-58 prose annotation was preserved (prose and the printer's short form parse to the same
UNWITNESSED, and the test holds that). Every count quoted in the document's prose was computed by script
from the printer's output, not added up by eye.

---

## 4. Findings for other owners — three live overclaims outside my ownership

★ None of these were touched. Each is the FR-209 shape in a file I do not own.

1. **`design/ROADMAP_STATUS.md:63` understates Form 6251 by six captions, and hides what looks like a
   statutory change.** It says *"Only two text cells differ — line 1a's Schedule 1-A cross-reference
   (37 to 43) and line 4's MFS threshold ($900,350 to $640,200)"*. The tool prints **8**: 1a, 4, **5**,
   **7**, **18**, **19**, **25**, **39**. The omitted one that matters most is **line 5**, the
   exemption / phase-out table: *"single or head of household $626,350 / $88,100"* becomes
   **"$500,000 / $90,100"**. A phase-out threshold moving $626,350 down to $500,000 is not an indexing
   bump in the direction indexing goes; it has the shape of the OBBBA section 55(d) phase-out reversion,
   and it is **taxpayer-adverse**. Also omitted: lines 18 and 39 swapping places *while reworded* (the
   26%/28% breakpoint $239,100 to $244,500), line 19 ($96,700 to $98,900) and line 25 ($533,400 to
   $545,500). **This is the row that priced the f6251 transcription.**
2. **`design/SPEC_1099da_broker_reporting.md:358`** says *"both forms are 'unchanged' in shape on the 2026
   drafts per TY2026_WORK_LIST.md, so the port is two rows"*, of Form 8949 and Schedule D. Schedule D is
   `transfers | unchanged` and that holds. **Form 8949 is now `transfers | UNWITNESSED`**: its caption axis
   compared 0 of 2 surviving line numbers, and 16 of its 202 paired boxes are unread. The claim rests on an
   axis that never looked.
3. **Form 6251's own text reports a renumber on a form we have no draft of.** Line 7's caption changed from
   *"...directly on form 1040 or 1040-SR line **7**"* to *"line **7a**"*. The work list carries `f1040` as
   **NO DRAFT**, so this is the only evidence in the tree that the TY2026 1040 splits line 7 — worth a
   follow-up, because the 1040 is the form every other map cross-references.

Plus decayed line-anchored citations (FR-152's shape): `crates/btctax-core/src/tax/return_inputs.rs:356`
cites `TY2026_WORK_LIST.md:36`, `design/SPEC_interview.md:607` cites `:36`, and `ROADMAP_STATUS.md:63`
cites `:41`. All three already pointed at the wrong lines before today; this document grew by 110 lines, so
they are further adrift. The claims those files make about f6251's map (`map.rs:536`,
`f6251_revision.rs:9` — *"62 fields, 0 renamed, 0 moved"*) are **verified true** at HEAD.

---

## 5. What is on disk

* `crates/xtask/src/form_delta.rs` — `FieldMapVerdict` / `FormVerdict` and their derivations (shared by the
  printer and the checker, never re-implemented); the printer's 14-column row through one witness closure;
  `NumericRow` 6 cells to 13 (`Copy` dropped, so the compiler named every reader); `check_work_list_against`
  extended to all 11 counts and both verdict words; 27 derived plants and 4 controls.
* `design/TY2026_WORK_LIST.md` — the numeric table regenerated from the printer; the `shape` prose replaced
  by the two-verdict prose and the "neither verdict without its UNREAD columns" rule; a
  `REGENERATED AGAIN 2026-09-13 (FR-209, FR-211)` block carrying the measured census and FR-210's cost; the
  f8995a footnote's five-column row literal marked as historical.
* Diffstat: 2 files changed, 585 insertions, 101 deletions. Working tree left dirty, uncommitted.
