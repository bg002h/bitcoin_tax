# REPORT — wave 3, parcel K. FR-218: the 8949 emitted a blank Part I page per copy.

**Scope:** parcel K of `design/agent-reports/BRIEF-wave3.md` (`8e85a0680`), FR-218.
**Verdict:** FIXED. The brief's premise, cause, authority and prescribed fix are all CONFIRMED — nothing
refuted.
**Gate:** `cargo nextest run --workspace` **3734/3734 pass, 12 skipped**; `cargo clippy --workspace
--all-targets --all-features -- -D warnings` **exit 0, 0 warnings**; `cargo fmt --all --check` **exit 0**.
**Not committed, not pushed.** Diffstat: 9 files, +1003/-159.

---

## 1. The owner's symptom, reproduced and then extinguished — measured, not argued

The brief's confirmation was *"pages 1 and 2 of the 4-page 8949 have identical text layers (same md5);
pages 3 and 4 differ."* I reproduced exactly that in a harness, on the pre-fix emitter, with a
long-term-only filer of 15 legs on the 14-row TY2024 grid (ceil(15/14) = 2 Part II pages):

| | pages | page 1 | page 2 | page 3 | page 4 |
|---|---|---|---|---|---|
| **pre-fix** | 4 | `bf5ac520c970` | `bf5ac520c970` | `05595da0e63d` | `ca4f39f402b7` |
| **post-fix** | 2 | `05595da0e63d` | `ca4f39f402b7` | — | — |

(`pdftotext -layout -f N -l N | md5sum`, first 12 hex.) Pages 1 and 2 pre-fix have byte-identical text
layers — the owner's sentence, verbatim, out of a test fixture. **And each of those two blank pages
carried the filer's SSN** (`grep -c '123-45-6789'` = 1 on every page, pre- and post-fix), which is the
`:78` half of FR-218: a name-and-SSN page that asserts nothing, filed twice.

The owner's mixed shape (1 Part I page + 2 Part II pages) went **4 to 3** pages, all three text layers
now distinct.

## 2. What was wrong, and what the fix does

`fill8949_full.rs:122` (pre-fix) `let n_copies = st_pages.len().max(lt_pages.len()).max(1);` — confirmed
as stated. The copy count is still `max(|ST|, |LT|)`, because a copy is one physical template; what
changed is that **a copy now files only the pages whose part has rows**, so the document carries
`|ST pages| + |LT pages|` pages rather than `2 x max(|ST|, |LT|)`.

Authority, now quoted in the source at four sites, `design/forms/extract/i8949--2024.txt:422-424`: *"You
don't need to complete and file an entire copy of Form 8949 (Parts I and II) if you can check a single
box to describe all your transactions. In that case, complete and file **either Part I or II** and check
the box that describes the transactions."*

Three moving parts, all derived rather than typed:

- **`fill8949.rs:210` `pages_to_file`** — the kept page set, derived from `map.parts[].{term,page}`,
  never a literal `{0,1}`. Fails closed on a part whose `term` is neither `"short"` nor `"long"`, and on
  a template page no part claims (nothing would decide whether to file it).
- **`overflow.rs:265` `retain_pages`** — reduces a filled copy to those pages: rewrites the page-tree
  `/Kids`, removes the AcroForm field subtrees whose widgets live on dropped pages, then
  `prune_objects()`. It runs **after** the writes (the write index is built from the unreduced template)
  and **before** the read-back verify, so `verify_8949` and `no_unmapped_filled` run on the document that
  is actually emitted. Keeping every page is an early-return no-op, which is why the common both-parts
  case stays byte-identical (section 5).
- **`overflow.rs:231` `independent_part_plan` + `:52` `merge_pages`** — the merge now takes an explicit
  `PagePick { copy, page }` plan instead of the `PageOrder::ByPosition` rectangle transpose. Part
  grouping (FR-113) is preserved, and is now ordered by each part's **template page index** read off the
  map rather than by a hardcoded "short first". `fill8949_full.rs:159 part_page_plan` builds it, and
  `lib.rs:184` reuses the same builder.

**The subtree/page partition was measured, not assumed** before `retain_pages` was written: on both
bundled templates the AcroForm root has exactly two kids (`Page1[0]`, `Page2[0]`) and each holds exactly
one page's widgets — 122/122 on TY2024, 101/101 on TY2025 (`qpdf --json`, leaf-to-`/Annots` join). A
subtree straddling a kept and a dropped page would **refuse**; that branch is a fail-closed guard for a
future revision, not a live case, and the doc comment says so.

**The retired rectangle check was replaced by a strictly stronger one.** `PageOrder::ByPosition` required
every copy to have the same page count; FR-218 makes ragged copies the normal case (a long-term-only
filer's copies carry one page each), so `overflow.rs:138 plan_covers_every_page_once` now requires the
plan to be a **bijection onto every page of every copy** — it can neither drop a filled page nor emit one
twice. `flat_page_tree` is kept, and is now applied on the `ByCopy` path too (Form 8283's merge), whose
page tree I measured flat first.

## 3. A SECOND EMITTER HAD THE IDENTICAL DEFECT — and it is outside the brief's file list

`crates/btctax-forms/src/lib.rs:134 fill_form_8949` (the crypto-slice path) carried the same
`max()`-of-both-parts pagination, line for line, and emitted the same blank part pages. FR-218 and the
brief name only `fill8949_full.rs`.

I fixed it, and **`lib.rs` is not in parcel K's OWNS list.** The reasons, for the controller to accept or
revert independently:

1. The fix necessarily lands in the shared single-copy filler (`fill_8949_parts_inner`), which both paths
   call, because the page reduction has to happen before the read-back verify. Once copies can be ragged,
   `lib.rs`'s `PageOrder::ByPosition` call **refuses** (`pages_per_copy` rejects a ragged set) — so
   `lib.rs` could not have been left alone and still compile and pass.
2. Nobody else is editing `crates/btctax-forms/` in this wave (parcel J owns `btctax-core/src/tax/`,
   `btctax-cli/src/cmd/tax.rs`, `xtask/src/toml_schema.rs`), so there is no collision risk.
3. Leaving a sibling emitter with the same defect is the two-code-paths shape `CLAUDE.md`'s "derive the
   list, or make the compiler hold it" exists to stop. The slice path's blank pages are not identity-
   stamped (no header is written there), but they are still filed pages asserting nothing.

The `lib.rs` change is 4 lines of code (the plan call) plus doc comments.

## 4. B1 — the kill is STRUCTURAL, and every guard was watched RED

Nothing this project owns could see this defect: the golden is byte-stable, every filled field reads back,
both oracles agree, the roll-up closes, and the blank page's cells are *correctly* blank. A test that
looks at **values** cannot reach it. What distinguishes the defect is that the **page exists** — a fact
about the page tree and the widget annotations.

So `tests/overflow.rs` grew `part_of_each_page`: it classifies every page of the emitted document by whose
mapped widgets sit on that page's `/Annots`, with the cell list **derived from `PartMap`** (rows + line-2
totals + box checkboxes), and it **asserts** that each page classifies to exactly one part rather than
reporting an unclassifiable page as agreement (HARNESS class beta). A blank Part I page classifies as
`"short"` exactly as a populated one does, which is what makes the instrument able to see FR-218.

Four new tests; three planted mutations; every one observed red.

| guard | mutation | observed |
|---|---|---|
| `a_part_with_no_rows_files_no_page_at_all` (6 filer shapes x both emitters) | pre-fix emitter restored from `HEAD` | RED — `left: ["short","long"] right: ["long"]` |
| `the_full_return_8949_names_only_the_pages_it_files` | pre-fix emitter | RED — `left: ["short","short","long","long"] right: ["long","long"]` |
| `every_filed_pages_line2_totals_sum_to_the_parts_schedule_d_total` | per-page line-2 total omits that page's first row (`printed_part_data`) | RED — `short (d) proceeds ... left: 12000 right: 13600` |
| `overflow::tests::a_page_plan_that_is_not_a_bijection_refuses` (duplicate branch) | delete the `if *slot { return Err }` branch | RED — *"a page named twice must refuse"* |
| `overflow::tests::a_page_plan_that_is_not_a_bijection_refuses` (missed branch) | delete the `if !missed.is_empty()` branch | RED — *"a plan that forgets a filled page must refuse"* |
| `overflow::tests::the_independent_part_plan_emits_one_page_per_part_page` | — (a table of six page-count shapes, incl. the owner's) | green; it is the plan's own KAT |

**B1 paid immediately, exactly as `CLAUDE.md` says it does.** My first duplicate-pick case was
`[(0,0),(0,0),(0,1),(1,1)]` — and with the duplicate branch deleted it **still passed**, because that plan
also *misses* `(1,0)` and the `missed` branch caught it. The duplicate branch had no independent kill. The
case is now `[(0,0),(0,0),(0,1),(1,0),(1,1)]` — five picks over four pages, every slot covered, one repeat
— which only the duplicate branch can reject. Without writing the kill I would have shipped a guard whose
reviewable claim ("which test reds when this is removed?") had no answer.

The roll-up assertion the brief demanded is in place and **can fail**: the sum of the per-page line-2
totals for columns (d), (e) and (h) of **both** parts must equal `printed.{st,lt}_totals`, over a fixture
whose every part spans two pages. The owner's measured numbers (352655+468635 = 821290; 86361+102520 =
188881; 266294+366115 = 632409) are recorded in its doc comment as the worked example; the *property* is
what is asserted, since the owner's ledger is not a fixture. It passed on the pre-fix emitter too — and
correctly so: it is a **preservation** check, not an FR-218 kill, which is exactly why it needed its own
mutation to earn B1.

## 5. What the change does NOT touch — measured, not asserted

Whole-file md5 of eight emitted PDFs, pre-fix vs post-fix:

| fixture | slice path | full-return path |
|---|---|---|
| mixed, 1 Part I page + 1 Part II page | **IDENTICAL** | **IDENTICAL** |
| long-term only, 1 page | changed (2 to 1 page) | changed |
| long-term only, 2 pages | changed (4 to 2) | changed |
| owner's shape, 1 ST + 2 LT pages | changed (4 to 3) | changed |

The both-parts single-copy case — the common one, and the shape every byte-stable 8949 assertion in the
suite exercises — is byte-for-byte what it was. `retain_pages` early-returns when nothing is dropped.

`qpdf --check` on all eight emitted PDFs: exit 0, *"No syntax or stream encoding errors found"* on every
one. `pdfinfo`/`pdftotext` page counts agree with `lopdf`'s. So the page surgery does not produce a
malformed file — worth measuring, because `retain_pages` rewrites `/Kids`, edits the AcroForm field tree
and prunes objects.

## 6. Five committed tests were asserting the defect. Each regeneration is justified from the instruction

The brief asked me to check this rather than assume it. Five did. I record each one's pre-fix red so the
diff shows what changed and why — a golden cannot validate its own regeneration, so **none of these
expectations was taken from my new output**: each is re-derived from ceil(rows/grid) per part and the i8949
sentence, and each edit carries that sentence in its comment.

| test | asserted | now | pre-fix red |
|---|---|---|---|
| `tests/overflow.rs::eleven_rows_per_page` | `6` "3 copies x 2 pages" | `st+lt` = 5, derived from `cap` | `left: 6 right: 5` |
| `tests/sp3.rs::ty2024_8949_14_rows` | `2` "14 rows = 1 copy" | `1` (14 ST rows, no LT rows) | `left: 1 right: 2` |
| `tests/broker_boxes.rs::a_mixed_short_term_set_prints_one_page_set_per_box` | `4` "2 copies x 2 pages"; comment ended *"copy 2 page 2 blank"* | `3` (Part I boxed G, Part I boxed I, Part II boxed J) | `left: 3 right: 4` |
| `tests/full_return_forms.rs::the_full_return_8949_paginates_and_the_fifteenth_leg_lands_on_copy_two` | `4` pages **and** *"both copies' page 1 is named"* | `2` pages; **no** Part I name or SSN cell exists | `left: 2 right: 4` |
| `crates/btctax-cli/tests/export_irs_pdf.rs::a_full_return_with_more_8949_legs_than_a_page_holds_now_files_on_multiple_copies` | `4` "16 legs => 2 copies x 2 pages" | `2` — `dca_events_2024` buys in 2022 and sells in 2024, so all 16 legs are long-term | `left: 2 right: 4` |

**The fourth is the sharpest evidence that FR-218 was a shipped expectation rather than an oversight.**
That test did not merely count pages: it asserted *"both copies' page 1 is named"* — it positively required
the filer's name and SSN on both blank Part I pages of a filer with no short-term transactions. The defect
had a passing test defending it. It is now inverted: `values_ending("Page1[0].f1_1[0]")` must be **empty**.

`export_irs_pdf.rs` is also outside parcel K's OWNS list (it is `btctax-cli/tests/`, and parcel J owns only
`btctax-cli/src/cmd/tax.rs`). It is an 8949 page-count assertion and it would otherwise be red.

## 7. Report, don't edit — for the controller

1. **`FOLLOWUPS.md` FR-218 is not marked closed.** FOLLOWUPS is not mine and parcel J may be in it.
2. **FR-113's closed entry, `FOLLOWUPS.md:6936`**, cites `overflow.rs:48 merge_copies_ordered` and
   `PageOrder::ByPosition` — both now gone. It is a historical record of what was true on 2026-09-12 and I
   did **not** rewrite it. If the controller wants the trail intact, a superseding line ("FR-218 replaced
   the rectangle transpose with an explicit page plan") belongs on FR-218's entry, not as an edit to
   FR-113's.
3. Same for `design/agent-reports/REPORT-build-batch-b-filer-facing.md:63,66,100`, which describes
   `PageOrder`, `merge_copies_ordered` and `pages_per_copy`. A persisted report is a record; leave it.
4. **FR-133 (two paginators) is cheaper to close than it was.** Both paths already shared
   `fill_8949_parts_inner`; they now also share `part_page_plan` and `merge_pages`, so what is still
   duplicated is only the `pages()` box-grouping chunker (`lib.rs:146` and `fill8949_full.rs:113`) — one
   function, differing only in `&Form8949Row` vs `Printed8949Row`.
5. **A latent understatement hole I closed in passing, because the new code had to consult it.** The old
   filler did `if let Some(p) = map.part("short")` — a map declaring no part for a side **silently dropped
   every row routed to it**. It now refuses (`fill8949.rs:187 unplaceable_part`). No bundled map is missing
   a part, so nothing changes today; it is a fail-closed guard on a path that would otherwise lose rows off
   a filed return.
6. **Not FR-218, but adjacent and unresolved:** a filer with **no** disposals at all still gets a blank
   2-page Form 8949 if `fill_form_8949` is called with an empty row set (the full-return path cannot reach
   this — `form_8949_printed` returns `None`). I deliberately preserved that shipped behaviour: a zero-page
   PDF is worse, FR-218 is about a blank page filed *beside* a populated one, and the right answer is
   probably that no 8949 should be emitted at all — a caller-level decision, not this function's.
   `pages_to_file`'s comment says so in the source.

## 8. Files changed

```
crates/btctax-forms/src/overflow.rs            | 553 ++--   PagePick, merge_pages,
                                                            plan_covers_every_page_once,
                                                            independent_part_plan, retain_pages
crates/btctax-forms/src/fill8949.rs            | 118 ++--   pages_to_file, identity per FILED page,
                                                            unplaceable_part, the retain call
crates/btctax-forms/src/fill8949_full.rs       |  50 +--    part_page_plan, merge_pages
crates/btctax-forms/src/lib.rs                 |  24 +--    OUT OF LIST: the slice path, same defect
crates/btctax-forms/tests/overflow.rs          | 329 ++--   3 new tests + part_of_each_page
crates/btctax-forms/tests/full_return_forms.rs |  45 +--    defect-asserting expectations inverted
crates/btctax-forms/tests/broker_boxes.rs      |  22 +--    4 to 3 pages
crates/btctax-forms/tests/sp3.rs               |   9 +--    2 to 1 page
crates/btctax-cli/tests/export_irs_pdf.rs      |  12 +--    OUT OF LIST: 4 to 2 pages
```

Working tree only — **nothing committed, nothing pushed.** An untracked `scratch-k/` holds the run logs and
mutation captures; it is not part of the change, delete it or keep it.
