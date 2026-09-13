# TY2026 port work list — computed, not estimated
<!-- tags: 2025 2026-DRAFT -->

Generated 2026-09-05 by `xtask form-delta <2025-final> <2026-draft>`; **regenerated 2026-09-06**
after the x-aware label join changed the reader behind the "lines that moved" column (fold review
L3: four cells were reader artifacts — f1040s2 27→23, f1040s3 3→2, f1040sa 5→2, **f6251 1→0**, so
Form 6251 does NOT move a line for TY2026); **regenerated 2026-09-13 (FR-210)** after the sub-letter
fix, which moved six rows and is the section *"REGENERATED 2026-09-13 (FR-210)"* below.
**Regenerate when a final lands, OR when the label reader changes** — the handoff is a diff, not a
rebuild from memory. `xtask`'s
`the_committed_work_list_matches_form_delta_at_head` reds when any cell of a row here disagrees with the
tool — **all eleven counts and both verdict words** — when an axis cell prints a number the axis never
witnessed (or `UNWITNESSED` for an axis that did), when an excused row below actually HAS a pair, or when a
stem with a map in any bundled year has no row in either table. Its plants are the printer's own rows with
**exactly one cell replaced**, named by the column word, so a plant cannot red for a second reason it did
not intend.

**Regenerate with `cargo run -p xtask -- port-status 2025 2026-DRAFT`** (FR-50): it prints both tables from the
emitting surface; `port_status_prints_the_committed_work_list` holds this document to it.

**The two verdict columns are mechanical, and they answer two different questions in two vocabularies with
no word in common** (`xtask`'s `field_map_verdict` / `form_verdict`; FR-209):

* **`field map`** — do the bundled map's box names carry forward? **`transfers`** when nothing was added,
  removed or respelled; **REBUILT** when added + removed exceed the boxes that paired at all; **`edits`**
  otherwise. A claim about the **AcroForm**, and about nothing else.
* **`the FORM`** — is this the same printed form? **CHANGED** when any axis found something (a printed line
  moved, a line number was retired or introduced, a surviving line number's caption now means something
  else); **`unchanged`** when every axis looked and none of them found anything; **UNWITNESSED** when
  nothing was found and at least one axis could not look.

**Neither verdict is to be read without the two UNREAD columns beside it** (FR-211). `unchanged` says *no
axis found a change*, never *every box was read*: `f1040sd` is `unchanged` beside `4` unread boxes — four
boxes that sit beside no printed line on either revision, so no line-move comparison exists for them to
fail — while `f8949` reads `UNWITNESSED | 2`, meaning nothing compared and two line numbers unread. A `0`
in a collision cell and a `0` from an axis that compared nothing are the same three pixels and are not the
same fact.

> ## ★★★ REGENERATED 2026-09-13 — three axes were added and the numbers moved on every row
> Before this regeneration the table had ONE axis that reads the printed page ("lines that moved"),
> and it was keyed on the full AcroForm FQN. Three things changed (FR-190, FR-191, FR-192):
>
> * **`common`/`added`/`removed` now pair boxes container-insensitively.** A container rename used to
>   empty the label axis outright — `f1040s3--2021` → `--2022` renames only the root subform and the
>   tool printed `0 common / 40 added / 41 removed`. Across the 58 consecutive pairs the committed
>   fixtures can form, the old key paired 3,373 boxes and the new one pairs 4,177.
> * **`lines retired`** is new: which printed line NUMBERS the revision stopped printing. Code that
>   still reads a retired line reads a **blank**, and a blank and a zero are the same thing on paper.
> * **`line numbers whose MEANING changed`** is new and is the expensive column. A line number that
>   survives while its printed caption changes is invisible to a name diff AND to a label diff, and
>   taxpayer-adverse in whichever direction the substituted quantity runs.
>
> ★★ **What the new column immediately says about TY2026.** `f1040sd` is the only form in the table
> that is genuinely unchanged. `f6251`, `f8959`, `f8960`, `f1040sb` and `f8995a` all read `0` in every
> older column and carry 2–12 changed line meanings; **Form 6251 was listed as `unchanged` and moves
> the meaning of 8 line numbers.** `f1040sa`'s 8 are FR-185's cascade, including the casualty line
> moving 15 → 16 *while widening who qualifies* (FR-187).
> ★ **Superseded in two details by FR-210's fix (2026-09-13):** Schedule A's count is **10**, not 8,
> and the checkbox does not become the total — TY2025's line-18 checkbox travels **verbatim to line
> 19** while TY2026's line 18 becomes the itemized-deduction **limitation** question. It is line
> **17** that changes from the itemized total to *"Other itemized deductions"*, and that collision
> was one of the two this table could not see. See the FR-210 section below.

> ## ★★★ REGENERATED AGAIN 2026-09-13 (FR-209, FR-211) — one column carried two claims, and the map axis was printing a third of its work
> **Not one of the older cells moved**: every count this table already had recomputes identically at HEAD.
> What changed is that two questions stopped sharing one word, three quantities the tool always had stopped
> being invisible, and the gaps stopped being indistinguishable from zeroes.
>
> * **`shape` is gone; `field map` and `the FORM` replace it.** **Form 6251 is the row that needed them
>   apart: `transfers | CHANGED`.** Its field map genuinely does carry forward — 62 paired, 0 added, 0
>   removed, **0 respelled** — so the 2026-09-11 owner ruling that greenlit its transcription on exactly
>   that ground was right, and the transcription work is not wasted. And **8** of its surviving line numbers
>   now print a materially different caption, including line 18's old text reappearing at line 39
>   *reworded* (similarity 0.62), so the *"unchanged, nothing to do"* reading attached to that row was void.
>   One column could not hold both facts; it held the wrong one, and `f1040sb`, `f8959`, `f8960` and
>   `f8995a` read the same way.
> * **`respelled` is new, and it re-prices the port: 112 map edits the table never showed.** A paired box
>   whose FQN spelling changed is the same box on every other axis and a **map edit** all the same, because
>   the bundled map stores the full FQN. Schedule C's row priced **6** edits (5 added, 1 removed) while
>   **45 of its 104 paired boxes** were respellings; Schedule 1-A's 44, Schedule 2's 15, Schedule 3's 2,
>   Schedule SE's 2, Form 8995's 2 and Schedule A's 1 were equally invisible — and Schedule 1's **1** was
>   the only change of any kind its row could report at all, every page axis on it being UNWITNESSED.
>   ★ **That last clause is superseded by FR-210 (2026-09-13):** Schedule 1's label set now reads, and
>   its row carries **3** changed line meanings.
>   `unpairable` is the pairing axis's own gap: **1** box, on Schedule A, neither added nor removed nor
>   compared, because its leaf name repeats on one side.
> * **`lines introduced` is new too — 65 printed line numbers that did not exist on the prior revision**
>   (Schedule 1-A 28, Schedule 2 16, Schedule A 10, Form 8995 6, Form 8995-A 3, Schedule C 1, Form 6251 1).
>   The table showed what each revision *retired* and never what it *added*, so a form could introduce a
>   line and read as all zeroes. ★ **FR-210 re-based this: the total is 74** (see below).
> * **`boxes UNREAD` and `line numbers UNREAD` are FR-211** — a zero and an unmeasured, told apart. They
>   totalled **137** unread boxes and **13** unread line numbers, and no older cell mentioned either:
>   `f1040sc` printed `33` moved out of 104 paired boxes with **22** of them unread, `f8949` printed `0`
>   moved with **16**, and `f1040sd`'s `unchanged` rests on 51 of its 55. ★ **FR-210 took those two
>   totals to 67 and 6** (see below); `f1040sc`, `f8949` and `f1040sd` are unmoved.
> * **★ What FR-210 costs this table, stated rather than absorbed.** **9** of those 13 unread line numbers
>   are the ambiguity FR-210 describes — `f1040s2`'s 1/13/17, `f1040s3`'s 13, **`f1040sa`'s 17** and 5,
>   `f1040sc`'s 16, and both of `f8949`'s — where the reader prints one label more than once and the axis
>   refuses to guess rather than guessing. The other 4 are captions it found empty on the new side
>   (`f1040s1a` 4a/4b/4c, `f8995` 1i). FR-210 is deliberately **not fixed here** — it re-bases three other
>   checkers — so its consequence is recorded instead: **the most consequential Schedule A collision,
>   TY2025's line-18 checkbox against TY2026's itemized total, is inside that 9 and prints as a gap rather
>   than as a hit.** Schedule A's `8` is therefore a floor, not a count.
>   ★★ **FR-210 was fixed 2026-09-13 and the bullet above is the record of what it cost, not the
>   current state.** 7 of those 9 now read; `f8949`'s 2 do not and must not — see the next section.

> ## ★★★ REGENERATED 2026-09-13 (FR-210) — the witness was wrong, and the refusal was right
> `witness_text` anchored its sub-letter band on the candidate column's **widest** token rather than on
> the right edge its **numerals** agree on. A suffixed label (`17f` ends at x2=111.00, `17b` at 113.50)
> ends 3–5.5pt right of a bare one (`17` ends at 108.00), so the band's left bound landed right of
> where every bare sub-letter starts and the reader dropped them. It printed `5` twice and `17` three
> times on Schedule A, and **axis C refused both as ambiguous, which was correct** — *"a skipped field
> is not a passed one"*. The fix is to the WITNESS; the axis still refuses what it cannot tell apart,
> and `f8949`'s `1` and `2` (printed once per page, on two pages) are the proof: they were ambiguous
> before, they are ambiguous now, and they are the two unread line numbers that remain.
>
> Measured over the 87 committed geometry fixtures, 12 changed and none lost a printed label:
> `f1040s1--2026-DRAFT` went from a **hard error** (a prose `a` admitted as an orphaned sub-letter) to
> 64 labels; `f1040sa` gained `5a 5e 8a 8b 8c 17g 17h 17k`; `f1040s2` gained 12 and `f1040s3` 2;
> `f1040sc`'s `16c` now merges (its parent is emitted 0.5pt *below* the letter, so the row merge is
> order-insensitive); `f6251--2026-DRAFT`'s `2` is its real `2a`, retiring a spurious 1-retired /
> 1-introduced pair; and `f1040s1--2022`, `fw2--2024/2025/2026` and `f4868` shed prose-derived labels
> (`14a`, `62a`, a duplicate `8a`). Every newly-read label — 64 + 12 + 2 + 8 + 1 across those five
> drafts — was looked up in its own `design/forms/extract/<stem>.txt` and found printed there, so the
> regeneration is justified by the DOCUMENT and not by the new output.
>
> | column | before | after |
> |---|---|---|
> | lines that moved | 123 | 113 |
> | boxes UNREAD | **137** | **67** |
> | lines retired | 34 | 22 |
> | lines introduced | 65 | **74** |
> | line numbers whose MEANING changed | 118 | **126** |
> | line numbers UNREAD | **13** | **6** |
>
> ★★ **The eight new collisions are not bookkeeping.** Three of them are on `f1040s1`, whose row said
> UNWITNESSED in every page column, and one is line **14** — *"moving expenses for members of the armed
> forces"* becomes *"…armed forces **and the intelligence community**"*, an eligibility widening no axis
> could previously see. Two are on Schedule A: line **17**, TY2025's itemized **total** against TY2026's
> *"Other itemized deductions"* heading, and line **5e**, where the SALT cap moves **$40,000 → $40,400**
> (**$20,000 → $20,200** MFS) and its phase-out start **$500,000 → $505,000** (**$250,000 → $252,500**).
> A figure change inside a caption is exactly what this axis exists for.

★ Drafts are EVIDENCE ONLY. Nothing here is transcribed; these are counts of what CHANGED.

> ## ★★ CORRECTED 2026-09-05 — the first version of this table was WRONG
> Its `lines that moved` column was an artifact of a **draft cover-sheet page offset**. Every
> IRS draft carries a "DRAFT—NOT FOR FILING" cover as page 1, so the form starts on page 2 —
> while box pages come from the AcroForm FQN (`Page1[0]`), which still says 1. The join matched
> page-1 boxes against the cover sheet and produced **plausible** wrong labels.
>
> The first table claimed Form 1040 moved **31** line bindings and Form 6251 **28**, and a commit
> message called that "the trap, on the two forms that matter most". **The true figures are 0
> and 1.** The calibration tests did not catch it because they compare two FINALS, neither of
> which has a cover sheet. Found by an independent recon lens; now guarded by
> `no_committed_geometry_fixture_contains_a_draft_cover_sheet`.

| form | common | added | removed | respelled | unpairable | lines that moved | boxes UNREAD | lines retired | lines introduced | line numbers whose MEANING changed | line numbers UNREAD | field map | the FORM |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `f1040s1` | 73 | 0 | 0 | 1 | 0 | 0 | 3 | 0 | 0 | 3 | 0 | edits | **CHANGED** |
| `f1040s1a` | 54 | 131 | 0 | 44 | 0 | 44 | 2 | 7 | 28 | 33 | 3 | **REBUILT** | **CHANGED** |
| `f1040s2` | 59 | 9 | 4 | 15 | 0 | 25 | 2 | 15 | 22 | 21 | 0 | edits | **CHANGED** |
| `f1040s3` | 37 | 1 | 0 | 2 | 0 | 3 | 2 | 0 | 1 | 4 | 0 | edits | **CHANGED** |
| `f1040sa` | 15 | 31 | 18 | 1 | 1 | 0 | 0 | 0 | 13 | 10 | 0 | **REBUILT** | **CHANGED** |
| `f1040sb` | 72 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 2 | 0 | transfers | **CHANGED** |
| `f1040sc` | 104 | 5 | 1 | 45 | 0 | 33 | 22 | 0 | 1 | 16 | 0 | edits | **CHANGED** |
| `f1040sd` | 55 | 0 | 0 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 0 | transfers | unchanged |
| `f1040sse` | 27 | 0 | 0 | 2 | 0 | 0 | 3 | 0 | 0 | 5 | 0 | edits | **CHANGED** |
| `f6251` | 62 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 8 | 0 | transfers | **CHANGED** |
| `f8949` | 202 | 0 | 0 | 0 | 0 | 0 | 16 | 0 | 0 | **UNWITNESSED** | 2 | transfers | **UNWITNESSED** |
| `f8959` | 26 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 12 | 0 | transfers | **CHANGED** |
| `f8960` | 38 | 0 | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 2 | 0 | transfers | **CHANGED** |
| `f8995` | 24 | 12 | 9 | 2 | 0 | 8 | 0 | 0 | 6 | 4 | 1 | edits | **CHANGED** |
| `f8995a` | 111 | 3 | 0 | 0 | 0 | 0 | 2 | 0 | 3 | 6 | 0 | edits | **CHANGED** |

## Not listed — stated PER CELL, never omitted (Fable plan review I8)

The table above enumerates from available fixture **pairs**. That is the wrong denominator: a form
the packet emits with no pair silently has no row — the same "clean verdict from zero comparisons"
shape the cover-sheet correction was about, one level up. `xtask port-status` prints these rows from the emitting surface (FR-50, done); the PROSE in this table is
hand-maintained — only the numeric table above is pasted verbatim, and the test holds each row's two claims
(prior side / new side) to the printer's:

| form | emitted? | prior side (`--2025` final) | TY2026 side | cell |
|---|---|---|---|---|
| `f1040` | **yes** — the return itself | `f1040--2025` | **NO DRAFT** — the file archived as `f1040--2026-DRAFT.pdf` was the TY2025 form (found 2026-09-05); its fixture is gone and the "199 common, unchanged" row it produced was a reader artifact (removed 2026-09-06 by the work-list test) | **NO DRAFT** |
| `f4868` | yes — the extension application; NOT a packet member (spec 4868/1040-V R2 makes it `btctax extension`, mailed on its own) | `f4868--2025` | **NO DRAFT** — the IRS posts no TY2026 Form 4868 draft; the revision arrives with the January 2027 package | **NO DRAFT** |
| `f1040v` | yes — the payment voucher, written BESIDE the packet and enclosed loose (spec 4868/1040-V R4) | `f1040v--2025` | **NO DRAFT** — the IRS posts no TY2026 Form 1040-V draft; the revision arrives with the January 2027 package | **NO DRAFT** |
| `f8283` | yes — `packet.rs:223` | `f8283--2025` (Rev. 12-2025, periodic) | **NO DRAFT** — the draft URL served a 2025 document; archiver refused | **NO DRAFT** |
| `f8889` | yes — `packet.rs`, Form 8889 (Health Savings Accounts), attachment sequence 52 (T16 / FR-76) | `f8889--2025` | **NO DRAFT** — the IRS posts no TY2026 Form 8889 draft; the revision arrives with the January 2027 package. ★ Its §223(b)(2) figures ARE already published (Rev. Proc. 2025-19: $4,400 / $8,750), because §223(g) requires them by June 1 of the preceding year — so the CONSTANTS are known a year before the paper is | **NO DRAFT** |
| `f8275` | yes — `packet.rs:218` | **NO PRIOR SIDE** for the `--2025` tag — `f8275--2024` (Rev. 10-2024, periodic) is aliased by hash for 2025 | **NO DRAFT** — the draft URL served a 2024 document; archiver refused | **NO DRAFT** — periodic; a new revision would be a hash change, not a year |

★ **2026-09-06 (residue sweep 1, item 6): `f8995a` moved OUT of this table and INTO the numeric one.**
Its cell used to read *"**NO PRIOR SIDE** — bundled for 2024 only; no `f8995a--2025` authority or
extract archived"*; the TY2025 revision is now archived as evidence (note, `-layout` extract,
geometry — no map, no filler, no bundled template), so the pair computes and the row is in the numeric
table above — it then printed `| f8995a | 111 | 3 | 0 | 0 | port |`, and the columns have doubled twice
since. That was the only row that moved: the numeric table went 14 rows → 15 and the excused table 6 → 5.

★ **2026-09-07 (T16 / FR-76): `f8889` ENTERED this table.** Form 8889 is bundled and emitted for
TY2024 and TY2025 from the day it was transcribed, and the IRS has posted no TY2026 draft of it — so
it is a row here rather than in the numeric table, with the prior side present and the new side
absent. The excused table went 5 rows → 6.

Re-run the archiver when the two periodic forms post a 2026 revision; regenerate this list from the
emitting surface, not from `design/forms/geometry/` pairs.
